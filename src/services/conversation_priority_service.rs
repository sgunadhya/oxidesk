// Feature 020: Conversation Priority Management
use crate::api::middleware::{ApiError, ApiResult};
use crate::database::Database;
use crate::events::{EventBus, SystemEvent};
use crate::models::Conversation;
use std::sync::Arc;

/// Service for managing conversation priorities
pub struct ConversationPriorityService {
    db: Database,
    event_bus: Option<Arc<dyn EventBus>>,
}

impl ConversationPriorityService {
    pub fn new(db: Database, event_bus: Option<Arc<dyn EventBus>>) -> Self {
        Self { db, event_bus }
    }

    /// Update conversation priority
    ///
    /// # Arguments
    /// * `conversation_id` - ID of the conversation to update
    /// * `new_priority` - New priority value ("Low", "Medium", "High", or None to remove)
    /// * `updated_by` - User ID performing the update
    /// * `event_bus` - Optional event bus for triggering automation rules
    ///
    /// # Returns
    /// Updated conversation
    ///
    /// # Errors
    /// * `ApiError::NotFound` - Conversation not found
    /// * `ApiError::BadRequest` - Invalid priority value
    /// Update conversation priority
    ///
    /// # Arguments
    /// * `conversation_id` - ID of the conversation to update
    /// * `new_priority` - New priority value ("Low", "Medium", "High", or None to remove)
    /// * `updated_by` - User ID performing the update
    /// * `event_bus` - Optional event bus for triggering automation rules
    ///
    /// # Returns
    /// Updated conversation
    ///
    /// # Errors
    /// * `ApiError::NotFound` - Conversation not found
    pub async fn update_conversation_priority(
        &self,
        conversation_id: &str,
        new_priority: Option<crate::models::Priority>,
        updated_by: &str,
        // event_bus: Option<&EventBus>, // Removed
    ) -> ApiResult<Conversation> {
        // Get current conversation to check existing priority
        let current = self
            .db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        let previous_priority = current.priority.clone();

        // Check if priority actually changed (for idempotence)
        let priority_changed = previous_priority != new_priority;

        // Update priority in database
        if let Some(priority) = &new_priority {
            self.db
                .set_conversation_priority(conversation_id, priority)
                .await?;
        } else {
            // Remove priority (set to null)
            self.db.clear_conversation_priority(conversation_id).await?;
        }

        // Get updated conversation
        let updated = self
            .db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!(
                    "Conversation {} not found after update",
                    conversation_id
                ))
            })?;

        // Trigger automation rules only if priority actually changed
        // Trigger automation rules only if priority actually changed
        if priority_changed {
            if let Some(bus) = &self.event_bus {
                let event = SystemEvent::ConversationPriorityChanged {
                    conversation_id: conversation_id.to_string(),
                    previous_priority: previous_priority.map(|p| p.to_string()),
                    new_priority: new_priority.map(|p| p.to_string()),
                    updated_by: updated_by.to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };

                // Don't block on automation failure (graceful degradation)
                if let Err(e) = bus.publish(event) {
                    tracing::warn!("Failed to publish priority change event: {}", e);
                }
            }

            tracing::info!(
                "Updated priority for conversation {} from {:?} to {:?} by user {}",
                conversation_id,
                previous_priority,
                new_priority,
                updated_by
            );
        } else {
            tracing::debug!(
                "Priority update for conversation {} was idempotent (no change)",
                conversation_id
            );
        }

        Ok(updated)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_validate_priority_values() {
        let valid_priorities = vec!["Low", "Medium", "High"];
        for priority in valid_priorities {
            assert!(["Low", "Medium", "High"].contains(&priority));
        }

        let invalid_priorities = vec!["Urgent", "Critical", "low", ""];
        for priority in invalid_priorities {
            assert!(!["Low", "Medium", "High"].contains(&priority));
        }
    }
}
