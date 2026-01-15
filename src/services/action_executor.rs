use crate::database::agents::AgentRepository;
use crate::api::middleware::ApiError;
use crate::database::Database;
use crate::models::{ActionType, ConversationStatus, RuleAction};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum ActionError {
    InvalidParameters(String),
    ExecutionFailed(String),
    Timeout,
    ConversationNotFound,
    UserNotFound,
    TeamNotFound,
    TagNotFound,
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionError::InvalidParameters(msg) => write!(f, "Invalid parameters: {}", msg),
            ActionError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            ActionError::Timeout => write!(f, "Action execution timeout"),
            ActionError::ConversationNotFound => write!(f, "Conversation not found"),
            ActionError::UserNotFound => write!(f, "User not found"),
            ActionError::TeamNotFound => write!(f, "Team not found"),
            ActionError::TagNotFound => write!(f, "Tag not found"),
        }
    }
}

impl std::error::Error for ActionError {}

impl From<ApiError> for ActionError {
    fn from(err: ApiError) -> Self {
        ActionError::ExecutionFailed(err.to_string())
    }
}

pub struct ActionExecutor {
    db: Arc<Database>,
    timeout: Duration,
}

impl ActionExecutor {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            timeout: Duration::from_secs(10),
        }
    }

    pub fn with_timeout(db: Arc<Database>, timeout: Duration) -> Self {
        Self { db, timeout }
    }

    /// Execute an action on a conversation
    pub async fn execute(
        &self,
        action: &RuleAction,
        conversation_id: &str,
        executed_by: &str,
    ) -> Result<(), ActionError> {
        // Wrap execution with timeout
        tokio::time::timeout(
            self.timeout,
            self.execute_internal(action, conversation_id, executed_by),
        )
        .await
        .map_err(|_| ActionError::Timeout)?
    }

    async fn execute_internal(
        &self,
        action: &RuleAction,
        conversation_id: &str,
        executed_by: &str,
    ) -> Result<(), ActionError> {
        // Verify conversation exists
        let _conversation = self
            .db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or(ActionError::ConversationNotFound)?;

        tracing::info!(
            "Executing action {:?} on conversation {} by {}",
            action.action_type,
            conversation_id,
            executed_by
        );

        match action.action_type {
            ActionType::SetPriority => {
                self.execute_set_priority(conversation_id, &action.parameters)
                    .await
            }
            ActionType::AssignToUser => {
                self.execute_assign_to_user(conversation_id, executed_by, &action.parameters)
                    .await
            }
            ActionType::AssignToTeam => {
                self.execute_assign_to_team(conversation_id, executed_by, &action.parameters)
                    .await
            }
            ActionType::AddTag => {
                self.execute_add_tag(conversation_id, executed_by, &action.parameters)
                    .await
            }
            ActionType::RemoveTag => {
                self.execute_remove_tag(conversation_id, &action.parameters)
                    .await
            }
            ActionType::ChangeStatus => {
                self.execute_change_status(conversation_id, &action.parameters)
                    .await
            }
        }
    }

    async fn execute_set_priority(
        &self,
        conversation_id: &str,
        parameters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), ActionError> {
        let priority = parameters
            .get("priority")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ActionError::InvalidParameters("Missing 'priority' parameter".to_string())
            })?;

        // Validate priority value (Feature 020: Only Low, Medium, High)
        if !["Low", "Medium", "High"].contains(&priority) {
            return Err(ActionError::InvalidParameters(format!(
                "Invalid priority value: {}. Must be one of: Low, Medium, High",
                priority
            )));
        }

        let priority_enum = crate::models::Priority::from(priority.to_string());
        self.db
            .set_conversation_priority(conversation_id, &priority_enum)
            .await?;

        tracing::info!(
            "Set priority to '{}' for conversation {}",
            priority,
            conversation_id
        );

        Ok(())
    }

    async fn execute_assign_to_user(
        &self,
        conversation_id: &str,
        assigned_by: &str,
        parameters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), ActionError> {
        let user_id = parameters
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ActionError::InvalidParameters("Missing 'user_id' parameter".to_string())
            })?;

        // Verify user exists and is an agent
        let user = self
            .db
            .get_user_by_id(user_id)
            .await?
            .ok_or(ActionError::UserNotFound)?;

        // Verify it's an agent
        self.db
            .get_agent_by_user_id(&user.id)
            .await?
            .ok_or(ActionError::UserNotFound)?;

        // Assign conversation to user
        self.db
            .assign_conversation_to_user(
                conversation_id,
                Some(user_id.to_string()),
                Some(assigned_by.to_string()),
            )
            .await?;

        tracing::info!(
            "Assigned conversation {} to user {} by {}",
            conversation_id,
            user_id,
            assigned_by
        );

        Ok(())
    }

    async fn execute_assign_to_team(
        &self,
        conversation_id: &str,
        assigned_by: &str,
        parameters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), ActionError> {
        let team_id = parameters
            .get("team_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ActionError::InvalidParameters("Missing 'team_id' parameter".to_string())
            })?;

        // Verify team exists
        self.db
            .get_team_by_id(team_id)
            .await?
            .ok_or(ActionError::TeamNotFound)?;

        // Assign conversation to team
        self.db
            .assign_conversation_to_team(
                conversation_id,
                Some(team_id.to_string()),
                Some(assigned_by.to_string()),
            )
            .await?;

        tracing::info!(
            "Assigned conversation {} to team {} by {}",
            conversation_id,
            team_id,
            assigned_by
        );

        Ok(())
    }

    async fn execute_add_tag(
        &self,
        conversation_id: &str,
        added_by: &str,
        parameters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), ActionError> {
        let tag_name = parameters
            .get("tag")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ActionError::InvalidParameters("Missing 'tag' parameter".to_string()))?;

        // Get or create tag
        let tag = match self.db.get_tag_by_name(tag_name).await? {
            Some(tag) => tag,
            None => {
                return Err(ActionError::TagNotFound);
            }
        };

        // Add tag to conversation (idempotent - won't fail if already exists)
        self.db
            .add_conversation_tag(conversation_id, &tag.id, added_by)
            .await
            .map_err(|e| ActionError::ExecutionFailed(e.to_string()))?;

        tracing::info!(
            "Added tag '{}' to conversation {} by {}",
            tag_name,
            conversation_id,
            added_by
        );

        Ok(())
    }

    async fn execute_remove_tag(
        &self,
        conversation_id: &str,
        parameters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), ActionError> {
        let tag_name = parameters
            .get("tag")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ActionError::InvalidParameters("Missing 'tag' parameter".to_string()))?;

        // Get tag by name
        let tag = match self.db.get_tag_by_name(tag_name).await? {
            Some(tag) => tag,
            None => {
                // If tag doesn't exist, consider it already removed (idempotent)
                tracing::debug!(
                    "Tag '{}' not found, considering it already removed from conversation {}",
                    tag_name,
                    conversation_id
                );
                return Ok(());
            }
        };

        // Remove tag from conversation (idempotent - won't fail if not present)
        self.db
            .remove_conversation_tag(conversation_id, &tag.id)
            .await
            .map_err(|e| ActionError::ExecutionFailed(e.to_string()))?;

        tracing::info!(
            "Removed tag '{}' from conversation {}",
            tag_name,
            conversation_id
        );

        Ok(())
    }

    async fn execute_change_status(
        &self,
        conversation_id: &str,
        parameters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), ActionError> {
        let status_str = parameters
            .get("status")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ActionError::InvalidParameters("Missing 'status' parameter".to_string())
            })?;

        // Parse status
        let status = match status_str {
            "open" => ConversationStatus::Open,
            "snoozed" => ConversationStatus::Snoozed,
            "resolved" => ConversationStatus::Resolved,
            "closed" => ConversationStatus::Closed,
            _ => {
                return Err(ActionError::InvalidParameters(format!(
                    "Invalid status value: {}. Must be one of: open, snoozed, resolved, closed",
                    status_str
                )));
            }
        };

        // Update conversation status
        self.db
            .update_conversation_status(conversation_id, status)
            .await?;

        tracing::info!(
            "Changed status to '{}' for conversation {}",
            status_str,
            conversation_id
        );

        Ok(())
    }
}

impl Default for ActionExecutor {
    fn default() -> Self {
        panic!(
            "ActionExecutor cannot be created with default(). Use new() with a Database instance."
        );
    }
}
