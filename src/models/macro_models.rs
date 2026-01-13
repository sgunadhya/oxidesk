use serde::{Deserialize, Serialize};

/// Reusable message template with associated actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub message_content: String,
    pub created_by: String,
    pub created_at: String,  // ISO 8601
    pub updated_at: String,  // ISO 8601
    pub usage_count: i32,
    pub access_control: String,  // "all" or "restricted"

    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<MacroAction>>,
}

impl Macro {
    /// Validate macro fields
    pub fn validate(&self) -> Result<(), String> {
        // Name validation
        if self.name.is_empty() || self.name.len() > 255 {
            return Err("Macro name must be between 1 and 255 characters".to_string());
        }

        // Message content validation
        if self.message_content.is_empty() || self.message_content.len() > 10000 {
            return Err("Message content must be between 1 and 10,000 characters".to_string());
        }

        // Access control validation
        if self.access_control != "all" && self.access_control != "restricted" {
            return Err("Access control must be 'all' or 'restricted'".to_string());
        }

        // Usage count validation
        if self.usage_count < 0 {
            return Err("Usage count cannot be negative".to_string());
        }

        Ok(())
    }
}

/// Action to execute when macro is applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroAction {
    pub id: String,
    pub macro_id: String,
    pub action_type: String,  // set_status, assign_to_user, assign_to_team, add_tag, set_priority
    pub action_value: String,
    pub action_order: i32,
}

impl MacroAction {
    /// Validate action type and value
    pub fn validate(&self) -> Result<(), String> {
        // Validate action type
        let valid_types = vec![
            "set_status",
            "assign_to_user",
            "assign_to_team",
            "add_tag",
            "set_priority",
        ];

        if !valid_types.contains(&self.action_type.as_str()) {
            return Err(format!(
                "Invalid action type '{}'. Must be one of: {}",
                self.action_type,
                valid_types.join(", ")
            ));
        }

        // Validate action value is not empty
        if self.action_value.is_empty() {
            return Err("Action value cannot be empty".to_string());
        }

        // Validate action order
        if self.action_order < 0 {
            return Err("Action order cannot be negative".to_string());
        }

        Ok(())
    }
}

/// Access control entry for restricted macros
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroAccess {
    pub id: String,
    pub macro_id: String,
    pub entity_type: String,  // "user" or "team"
    pub entity_id: String,
    pub granted_at: String,  // ISO 8601
    pub granted_by: String,
}

impl MacroAccess {
    /// Validate access entry
    pub fn validate(&self) -> Result<(), String> {
        // Validate entity type
        if self.entity_type != "user" && self.entity_type != "team" {
            return Err("Entity type must be 'user' or 'team'".to_string());
        }

        // Validate entity_id is not empty
        if self.entity_id.is_empty() {
            return Err("Entity ID cannot be empty".to_string());
        }

        Ok(())
    }
}

/// Audit log for macro application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroApplicationLog {
    pub id: String,
    pub macro_id: String,
    pub agent_id: String,
    pub conversation_id: String,
    pub applied_at: String,  // ISO 8601
    pub actions_queued: String,  // JSON array
    pub variables_replaced: i32,
}

impl MacroApplicationLog {
    /// Validate log entry
    pub fn validate(&self) -> Result<(), String> {
        // Validate variables_replaced is non-negative
        if self.variables_replaced < 0 {
            return Err("Variables replaced count cannot be negative".to_string());
        }

        // Validate actions_queued is valid JSON array
        if let Err(e) = serde_json::from_str::<Vec<String>>(&self.actions_queued) {
            return Err(format!("Invalid actions_queued JSON: {}", e));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_validation() {
        let mut macro_obj = Macro {
            id: "test-id".to_string(),
            name: "Test Macro".to_string(),
            message_content: "Hello {{contact_name}}".to_string(),
            created_by: "user-123".to_string(),
            created_at: "2026-01-13T10:00:00Z".to_string(),
            updated_at: "2026-01-13T10:00:00Z".to_string(),
            usage_count: 0,
            access_control: "all".to_string(),
            actions: None,
        };

        // Valid macro
        assert!(macro_obj.validate().is_ok());

        // Invalid name (empty)
        macro_obj.name = "".to_string();
        assert!(macro_obj.validate().is_err());

        // Invalid name (too long)
        macro_obj.name = "a".repeat(256);
        assert!(macro_obj.validate().is_err());

        // Valid name
        macro_obj.name = "Test Macro".to_string();
        assert!(macro_obj.validate().is_ok());

        // Invalid message content (empty)
        macro_obj.message_content = "".to_string();
        assert!(macro_obj.validate().is_err());

        // Invalid message content (too long)
        macro_obj.message_content = "a".repeat(10001);
        assert!(macro_obj.validate().is_err());

        // Valid message content
        macro_obj.message_content = "Hello {{contact_name}}".to_string();
        assert!(macro_obj.validate().is_ok());

        // Invalid access control
        macro_obj.access_control = "invalid".to_string();
        assert!(macro_obj.validate().is_err());

        // Valid access control
        macro_obj.access_control = "restricted".to_string();
        assert!(macro_obj.validate().is_ok());
    }

    #[test]
    fn test_macro_action_validation() {
        let mut action = MacroAction {
            id: "action-id".to_string(),
            macro_id: "macro-id".to_string(),
            action_type: "set_status".to_string(),
            action_value: "resolved".to_string(),
            action_order: 0,
        };

        // Valid action
        assert!(action.validate().is_ok());

        // Invalid action type
        action.action_type = "invalid_action".to_string();
        assert!(action.validate().is_err());

        // Valid action types
        for action_type in &[
            "set_status",
            "assign_to_user",
            "assign_to_team",
            "add_tag",
            "set_priority",
        ] {
            action.action_type = action_type.to_string();
            assert!(action.validate().is_ok());
        }

        // Invalid action value (empty)
        action.action_type = "set_status".to_string();
        action.action_value = "".to_string();
        assert!(action.validate().is_err());

        // Valid action value
        action.action_value = "resolved".to_string();
        assert!(action.validate().is_ok());
    }

    #[test]
    fn test_macro_access_validation() {
        let mut access = MacroAccess {
            id: "access-id".to_string(),
            macro_id: "macro-id".to_string(),
            entity_type: "user".to_string(),
            entity_id: "user-123".to_string(),
            granted_at: "2026-01-13T10:00:00Z".to_string(),
            granted_by: "admin-123".to_string(),
        };

        // Valid access
        assert!(access.validate().is_ok());

        // Invalid entity type
        access.entity_type = "invalid".to_string();
        assert!(access.validate().is_err());

        // Valid entity types
        access.entity_type = "team".to_string();
        assert!(access.validate().is_ok());

        // Invalid entity_id (empty)
        access.entity_type = "user".to_string();
        access.entity_id = "".to_string();
        assert!(access.validate().is_err());
    }

    #[test]
    fn test_macro_application_log_validation() {
        let mut log = MacroApplicationLog {
            id: "log-id".to_string(),
            macro_id: "macro-id".to_string(),
            agent_id: "agent-123".to_string(),
            conversation_id: "conv-123".to_string(),
            applied_at: "2026-01-13T10:00:00Z".to_string(),
            actions_queued: r#"["set_status", "add_tag"]"#.to_string(),
            variables_replaced: 2,
        };

        // Valid log
        assert!(log.validate().is_ok());

        // Invalid variables_replaced (negative)
        log.variables_replaced = -1;
        assert!(log.validate().is_err());

        // Valid variables_replaced
        log.variables_replaced = 2;
        assert!(log.validate().is_ok());

        // Invalid actions_queued (not JSON)
        log.actions_queued = "not json".to_string();
        assert!(log.validate().is_err());

        // Valid actions_queued (empty array)
        log.actions_queued = "[]".to_string();
        assert!(log.validate().is_ok());
    }

    #[test]
    fn test_macro_serialization() {
        let macro_obj = Macro {
            id: "test-id".to_string(),
            name: "Test Macro".to_string(),
            message_content: "Hello {{contact_name}}".to_string(),
            created_by: "user-123".to_string(),
            created_at: "2026-01-13T10:00:00Z".to_string(),
            updated_at: "2026-01-13T10:00:00Z".to_string(),
            usage_count: 5,
            access_control: "all".to_string(),
            actions: Some(vec![MacroAction {
                id: "action-id".to_string(),
                macro_id: "test-id".to_string(),
                action_type: "set_status".to_string(),
                action_value: "resolved".to_string(),
                action_order: 0,
            }]),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&macro_obj).unwrap();
        assert!(json.contains("Test Macro"));
        assert!(json.contains("contact_name"));

        // Deserialize from JSON
        let deserialized: Macro = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Test Macro");
        assert_eq!(deserialized.usage_count, 5);
        assert!(deserialized.actions.is_some());
        assert_eq!(deserialized.actions.unwrap().len(), 1);
    }
}
