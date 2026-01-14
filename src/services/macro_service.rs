use crate::{
    api::middleware::error::{ApiError, ApiResult},
    database::Database,
    models::*,
};
use regex::Regex;
use std::sync::Arc;
use time::OffsetDateTime;

/// Context for variable substitution
#[derive(Debug, Clone)]
pub struct VariableContext {
    pub contact_name: Option<String>,
    pub agent_name: Option<String>,
    pub conversation_id: String,
    pub team_name: Option<String>,
    pub contact_email: Option<String>,
    pub conversation_status: String,
    pub conversation_priority: Option<String>,
}

/// Result of applying a macro
#[derive(Debug, Clone)]
pub struct MacroApplicationResult {
    pub message_content: String,
    pub actions_to_queue: Vec<MacroAction>,
    pub variables_replaced: i32,
}

/// Service for macro management and application
pub struct MacroService;

impl MacroService {
    /// Replace variables in template with context data
    pub fn replace_variables(template: &str, context: &VariableContext) -> (String, i32) {
        let mut result = template.to_string();
        let mut replaced_count = 0;

        // Define variable replacements
        let replacements = vec![
            ("{{contact_name}}", context.contact_name.as_deref().unwrap_or("")),
            ("{{agent_name}}", context.agent_name.as_deref().unwrap_or("")),
            ("{{conversation_id}}", &context.conversation_id),
            ("{{team_name}}", context.team_name.as_deref().unwrap_or("")),
            ("{{contact_email}}", context.contact_email.as_deref().unwrap_or("")),
            ("{{conversation_status}}", &context.conversation_status),
            ("{{conversation_priority}}", context.conversation_priority.as_deref().unwrap_or("")),
        ];

        // Replace each variable
        for (variable, value) in replacements {
            if result.contains(variable) {
                let count = result.matches(variable).count();
                replaced_count += count as i32;
                result = result.replace(variable, value);
            }
        }

        // Handle whitespace variations (e.g., {{ contact_name }})
        let re = Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap();
        for cap in re.captures_iter(&result.clone()) {
            let full_match = &cap[0];
            let var_name = &cap[1];

            // Find matching variable (case-sensitive)
            let replacement = match var_name {
                "contact_name" => context.contact_name.as_deref().unwrap_or(""),
                "agent_name" => context.agent_name.as_deref().unwrap_or(""),
                "conversation_id" => &context.conversation_id,
                "team_name" => context.team_name.as_deref().unwrap_or(""),
                "contact_email" => context.contact_email.as_deref().unwrap_or(""),
                "conversation_status" => &context.conversation_status,
                "conversation_priority" => context.conversation_priority.as_deref().unwrap_or(""),
                _ => continue, // Unknown variable, leave unchanged
            };

            if result.contains(full_match) {
                replaced_count += 1;
                result = result.replace(full_match, replacement);
            }
        }

        (result, replaced_count)
    }

    /// Validate action type and value
    pub fn validate_action(action: &MacroAction) -> ApiResult<()> {
        action.validate().map_err(|e| ApiError::BadRequest(e))
    }

    /// Check if user has access to macro
    pub async fn check_macro_access(
        db: &Database,
        macro_obj: &Macro,
        user_id: &str,
    ) -> ApiResult<bool> {
        // If access_control is "all", everyone has access
        if macro_obj.access_control == "all" {
            return Ok(true);
        }

        // Check direct user access
        if db.user_has_macro_access(&macro_obj.id, user_id).await? {
            return Ok(true);
        }

        // Check team access
        // Get user's teams
        let teams = db.get_user_teams(user_id).await?;
        for team in teams {
            if db.team_has_macro_access(&macro_obj.id, &team.id).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Load conversation context for variable substitution
    pub async fn load_conversation_context(
        db: &Database,
        conversation_id: &str,
        agent_id: &str,
    ) -> ApiResult<VariableContext> {
        // Get conversation
        let conversation = db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Conversation not found".to_string()))?;

        // Get contact
        let contact = db.get_user_by_id(&conversation.contact_id).await?;
        let contact_email = contact.as_ref().map(|c| c.email.clone());

        // For contact_name, we'll use email as fallback since User model doesn't have name field
        let contact_name = contact.map(|c| c.email.clone());

        // Get agent
        let agent = db
            .get_user_by_id(agent_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

        // Get team if assigned
        let team_name = if let Some(team_id) = &conversation.assigned_team_id {
            db.get_team_by_id(team_id).await?.map(|t| t.name)
        } else {
            None
        };

        Ok(VariableContext {
            contact_name,
            agent_name: Some(agent.email.clone()), // Using email as name
            conversation_id: conversation.id.clone(),
            team_name,
            contact_email,
            conversation_status: conversation.status.to_string(),
            conversation_priority: conversation.priority.map(|p| p.to_string()),
        })
    }

    /// Apply macro to conversation
    pub async fn apply_macro(
        db: &Database,
        macro_id: &str,
        conversation_id: &str,
        agent_id: &str,
    ) -> ApiResult<MacroApplicationResult> {
        // Get macro
        let mut macro_obj = db
            .get_macro_by_id(macro_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Macro not found".to_string()))?;

        // Check access
        if !Self::check_macro_access(db, &macro_obj, agent_id).await? {
            return Err(ApiError::Forbidden(
                "You do not have access to this macro".to_string(),
            ));
        }

        // Load actions
        let actions = db.get_macro_actions(macro_id).await?;
        macro_obj.actions = Some(actions.clone());

        // Load conversation context
        let context = Self::load_conversation_context(db, conversation_id, agent_id).await?;

        // Replace variables
        let (message_content, variables_replaced) =
            Self::replace_variables(&macro_obj.message_content, &context);

        // Increment usage count
        db.increment_macro_usage(macro_id).await?;

        // Create application log
        let now = OffsetDateTime::now_utc();
        let log = MacroApplicationLog {
            id: uuid::Uuid::new_v4().to_string(),
            macro_id: macro_id.to_string(),
            agent_id: agent_id.to_string(),
            conversation_id: conversation_id.to_string(),
            applied_at: now.format(&time::format_description::well_known::Rfc3339).unwrap(),
            actions_queued: serde_json::to_string(
                &actions
                    .iter()
                    .map(|a| a.action_type.clone())
                    .collect::<Vec<String>>(),
            )
            .unwrap(),
            variables_replaced,
        };
        db.create_macro_application_log(&log).await?;

        Ok(MacroApplicationResult {
            message_content,
            actions_to_queue: actions,
            variables_replaced,
        })
    }

    /// Create a new macro
    pub async fn create_macro(
        db: &Database,
        name: String,
        message_content: String,
        actions: Vec<(String, String, i32)>, // (action_type, action_value, action_order)
        created_by: &str,
        access_control: String,
    ) -> ApiResult<Macro> {
        // Check for duplicate name
        if let Some(_) = db.get_macro_by_name(&name).await? {
            return Err(ApiError::Conflict(format!(
                "Macro with name '{}' already exists",
                name
            )));
        }

        // Create macro
        let now = OffsetDateTime::now_utc();
        let macro_id = uuid::Uuid::new_v4().to_string();
        let macro_obj = Macro {
            id: macro_id.clone(),
            name,
            message_content,
            created_by: created_by.to_string(),
            created_at: now.format(&time::format_description::well_known::Rfc3339).unwrap(),
            updated_at: now.format(&time::format_description::well_known::Rfc3339).unwrap(),
            usage_count: 0,
            access_control,
            actions: None,
        };

        // Validate
        macro_obj.validate().map_err(|e| ApiError::BadRequest(e))?;

        // Save macro
        db.create_macro(&macro_obj).await?;

        // Save actions
        let mut macro_actions = Vec::new();
        for (action_type, action_value, action_order) in actions {
            let action = MacroAction {
                id: uuid::Uuid::new_v4().to_string(),
                macro_id: macro_id.clone(),
                action_type,
                action_value,
                action_order,
            };

            // Validate action
            action.validate().map_err(|e| ApiError::BadRequest(e))?;

            db.create_macro_action(&action).await?;
            macro_actions.push(action);
        }

        // Return macro with actions
        let mut result = macro_obj;
        result.actions = Some(macro_actions);
        Ok(result)
    }

    /// Update an existing macro
    pub async fn update_macro(
        db: &Database,
        macro_id: &str,
        message_content: Option<String>,
        actions: Option<Vec<(String, String, i32)>>,
        access_control: Option<String>,
    ) -> ApiResult<Macro> {
        // Get existing macro
        let mut macro_obj = db
            .get_macro_by_id(macro_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Macro not found".to_string()))?;

        // Update fields
        if let Some(content) = message_content {
            macro_obj.message_content = content;
        }
        if let Some(access) = access_control {
            macro_obj.access_control = access;
        }

        let now = OffsetDateTime::now_utc();
        macro_obj.updated_at = now.format(&time::format_description::well_known::Rfc3339).unwrap();

        // Validate
        macro_obj.validate().map_err(|e| ApiError::BadRequest(e))?;

        // Save macro
        db.update_macro(&macro_obj).await?;

        // Update actions if provided
        if let Some(new_actions) = actions {
            // Delete existing actions
            db.delete_macro_actions(macro_id).await?;

            // Create new actions
            let mut macro_actions = Vec::new();
            for (action_type, action_value, action_order) in new_actions {
                let action = MacroAction {
                    id: uuid::Uuid::new_v4().to_string(),
                    macro_id: macro_id.to_string(),
                    action_type,
                    action_value,
                    action_order,
                };

                // Validate action
                action.validate().map_err(|e| ApiError::BadRequest(e))?;

                db.create_macro_action(&action).await?;
                macro_actions.push(action);
            }
            macro_obj.actions = Some(macro_actions);
        }

        Ok(macro_obj)
    }

    /// Delete a macro
    pub async fn delete_macro(db: &Database, macro_id: &str) -> ApiResult<()> {
        // Check if macro exists
        let _ = db
            .get_macro_by_id(macro_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Macro not found".to_string()))?;

        // Delete macro (cascade will handle actions and access)
        db.delete_macro(macro_id).await?;

        Ok(())
    }

    /// Get macro by ID with actions
    pub async fn get_macro(db: &Database, macro_id: &str) -> ApiResult<Macro> {
        let mut macro_obj = db
            .get_macro_by_id(macro_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Macro not found".to_string()))?;

        // Load actions
        let actions = db.get_macro_actions(macro_id).await?;
        macro_obj.actions = Some(actions);

        Ok(macro_obj)
    }

    /// List accessible macros for user
    pub async fn list_accessible_macros(
        db: &Database,
        user_id: &str,
    ) -> ApiResult<Vec<Macro>> {
        // Get all macros
        let all_macros = db.list_macros().await?;

        // Filter by access
        let mut accessible_macros = Vec::new();
        for macro_obj in all_macros {
            if Self::check_macro_access(db, &macro_obj, user_id).await? {
                accessible_macros.push(macro_obj);
            }
        }

        Ok(accessible_macros)
    }

    /// Grant access to a macro
    pub async fn grant_access(
        db: &Database,
        macro_id: &str,
        entity_type: &str,
        entity_id: &str,
        granted_by: &str,
    ) -> ApiResult<()> {
        // Check if macro exists
        let _ = db
            .get_macro_by_id(macro_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Macro not found".to_string()))?;

        // Validate entity type
        if entity_type != "user" && entity_type != "team" {
            return Err(ApiError::BadRequest(
                "Entity type must be 'user' or 'team'".to_string(),
            ));
        }

        // Validate entity exists
        if entity_type == "user" {
            let _ = db
                .get_user_by_id(entity_id)
                .await?
                .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;
        } else {
            let _ = db
                .get_team_by_id(entity_id)
                .await?
                .ok_or_else(|| ApiError::NotFound("Team not found".to_string()))?;
        }

        // Create access entry
        let now = OffsetDateTime::now_utc();
        let access = MacroAccess {
            id: uuid::Uuid::new_v4().to_string(),
            macro_id: macro_id.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            granted_at: now.format(&time::format_description::well_known::Rfc3339).unwrap(),
            granted_by: granted_by.to_string(),
        };

        db.create_macro_access(&access).await?;

        Ok(())
    }

    /// Revoke access to a macro
    pub async fn revoke_access(
        db: &Database,
        macro_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> ApiResult<()> {
        // Check if macro exists
        let _ = db
            .get_macro_by_id(macro_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Macro not found".to_string()))?;

        // Delete access entry
        db.delete_macro_access(macro_id, entity_type, entity_id)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_variables() {
        let context = VariableContext {
            contact_name: Some("John Doe".to_string()),
            agent_name: Some("Alice Smith".to_string()),
            conversation_id: "conv-123".to_string(),
            team_name: Some("Support Team".to_string()),
            contact_email: Some("john@example.com".to_string()),
            conversation_status: "open".to_string(),
            conversation_priority: Some("high".to_string()),
        };

        // Test basic replacement
        let template = "Hello {{contact_name}}, I'm {{agent_name}}";
        let (result, count) = MacroService::replace_variables(template, &context);
        assert_eq!(result, "Hello John Doe, I'm Alice Smith");
        assert_eq!(count, 2);

        // Test all variables
        let template = "Contact: {{contact_name}} ({{contact_email}})\nAgent: {{agent_name}}\nConversation: {{conversation_id}}\nTeam: {{team_name}}\nStatus: {{conversation_status}}\nPriority: {{conversation_priority}}";
        let (result, count) = MacroService::replace_variables(template, &context);
        assert!(result.contains("John Doe"));
        assert!(result.contains("john@example.com"));
        assert!(result.contains("Alice Smith"));
        assert!(result.contains("conv-123"));
        assert!(result.contains("Support Team"));
        assert!(result.contains("open"));
        assert!(result.contains("high"));
        assert_eq!(count, 7);

        // Test undefined variable
        let context_missing = VariableContext {
            contact_name: None,
            agent_name: Some("Alice".to_string()),
            conversation_id: "conv-123".to_string(),
            team_name: None,
            contact_email: None,
            conversation_status: "open".to_string(),
            conversation_priority: None,
        };
        let template = "Hello {{contact_name}}, Team: {{team_name}}";
        let (result, count) = MacroService::replace_variables(template, &context_missing);
        assert_eq!(result, "Hello , Team: "); // Empty strings
        assert_eq!(count, 2);

        // Test same variable multiple times
        let template = "{{agent_name}} here. Contact {{agent_name}} for help.";
        let (result, count) = MacroService::replace_variables(template, &context);
        assert_eq!(result, "Alice Smith here. Contact Alice Smith for help.");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_replace_variables_with_whitespace() {
        let context = VariableContext {
            contact_name: Some("John".to_string()),
            agent_name: Some("Alice".to_string()),
            conversation_id: "conv-123".to_string(),
            team_name: None,
            contact_email: None,
            conversation_status: "open".to_string(),
            conversation_priority: None,
        };

        // Test whitespace variations
        let template = "Hello {{ contact_name }} and {{  agent_name  }}";
        let (result, count) = MacroService::replace_variables(template, &context);
        assert!(result.contains("John"));
        assert!(result.contains("Alice"));
        assert!(count >= 2);
    }

    #[test]
    fn test_malformed_variables() {
        let context = VariableContext {
            contact_name: Some("John".to_string()),
            agent_name: Some("Alice".to_string()),
            conversation_id: "conv-123".to_string(),
            team_name: None,
            contact_email: None,
            conversation_status: "open".to_string(),
            conversation_priority: None,
        };

        // Test malformed variables (should be left unchanged)
        let template = "Hello {contact_name} and {{{agent_name}}}";
        let (result, _) = MacroService::replace_variables(template, &context);
        // Malformed variables should remain unchanged
        assert!(result.contains("{contact_name}") || result.contains("{{{agent_name}}}") || result.contains("Alice"));
    }
}
