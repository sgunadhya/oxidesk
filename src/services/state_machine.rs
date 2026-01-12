use crate::models::conversation::ConversationStatus;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransitionError {
    #[error("Invalid transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: ConversationStatus,
        to: ConversationStatus,
    },
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Context for a state transition, including metadata about who triggered it
#[derive(Debug, Clone)]
pub struct TransitionContext {
    pub conversation_id: String,
    pub from_status: ConversationStatus,
    pub to_status: ConversationStatus,
    pub agent_id: Option<String>,
    pub snooze_duration: Option<String>,
}

/// Result of a state transition execution
#[derive(Debug)]
pub struct TransitionResult {
    pub success: bool,
    pub new_status: ConversationStatus,
    pub message: String,
}

/// Validates if a state transition is allowed
pub fn validate_transition(
    from: ConversationStatus,
    to: ConversationStatus,
) -> Result<(), TransitionError> {
    use ConversationStatus::*;

    match (from, to) {
        // Same state is always valid (no-op)
        (a, b) if a == b => Ok(()),

        // Valid transitions
        (Open, Snoozed) => Ok(()),
        (Open, Resolved) => Ok(()),
        (Snoozed, Open) => Ok(()),
        (Resolved, Open) => Ok(()),

        // All other transitions are invalid
        _ => Err(TransitionError::InvalidTransition { from, to }),
    }
}

/// Execute a state transition with side effects (event publishing)
pub fn execute_transition(
    context: TransitionContext,
    event_bus: Option<&crate::events::EventBus>,
) -> Result<TransitionResult, TransitionError> {
    // Validate the transition
    validate_transition(context.from_status, context.to_status)?;

    tracing::info!(
        "Executing state transition for conversation {} from {:?} to {:?}",
        context.conversation_id,
        context.from_status,
        context.to_status
    );

    // Publish event to event bus if available
    if let Some(event_bus) = event_bus {
        let event = crate::events::SystemEvent::ConversationStatusChanged {
            conversation_id: context.conversation_id.clone(),
            old_status: context.from_status,
            new_status: context.to_status,
            agent_id: context.agent_id.clone(),
            timestamp: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
        };

        event_bus.publish(event);

        tracing::debug!(
            "Published status change event for conversation {}",
            context.conversation_id
        );
    }

    Ok(TransitionResult {
        success: true,
        new_status: context.to_status,
        message: format!(
            "Transition from {:?} to {:?} completed successfully",
            context.from_status, context.to_status
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_state_valid() {
        assert!(validate_transition(ConversationStatus::Open, ConversationStatus::Open).is_ok());
    }

    #[test]
    fn test_open_to_snoozed_valid() {
        assert!(validate_transition(ConversationStatus::Open, ConversationStatus::Snoozed).is_ok());
    }

    #[test]
    fn test_open_to_resolved_valid() {
        assert!(validate_transition(ConversationStatus::Open, ConversationStatus::Resolved).is_ok());
    }

    #[test]
    fn test_snoozed_to_open_valid() {
        assert!(validate_transition(ConversationStatus::Snoozed, ConversationStatus::Open).is_ok());
    }

    #[test]
    fn test_resolved_to_open_valid() {
        assert!(validate_transition(ConversationStatus::Resolved, ConversationStatus::Open).is_ok());
    }

    #[test]
    fn test_snoozed_to_resolved_invalid() {
        let result = validate_transition(ConversationStatus::Snoozed, ConversationStatus::Resolved);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TransitionError::InvalidTransition { .. }
        ));
    }

    #[test]
    fn test_resolved_to_snoozed_invalid() {
        let result = validate_transition(ConversationStatus::Resolved, ConversationStatus::Snoozed);
        assert!(result.is_err());
    }
}
