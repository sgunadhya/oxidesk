use oxidesk::models::conversation::ConversationStatus;
use oxidesk::services::state_machine::validate_transition;

#[test]
fn test_all_valid_transitions_pass() {
    // Valid transitions based on spec:
    // Open -> Snoozed
    assert!(validate_transition(ConversationStatus::Open, ConversationStatus::Snoozed).is_ok());

    // Open -> Resolved
    assert!(validate_transition(ConversationStatus::Open, ConversationStatus::Resolved).is_ok());

    // Snoozed -> Open
    assert!(validate_transition(ConversationStatus::Snoozed, ConversationStatus::Open).is_ok());

    // Resolved -> Open
    assert!(validate_transition(ConversationStatus::Resolved, ConversationStatus::Open).is_ok());

    // Same state (should be allowed)
    assert!(validate_transition(ConversationStatus::Open, ConversationStatus::Open).is_ok());
    assert!(validate_transition(ConversationStatus::Snoozed, ConversationStatus::Snoozed).is_ok());
    assert!(validate_transition(ConversationStatus::Resolved, ConversationStatus::Resolved).is_ok());
    assert!(validate_transition(ConversationStatus::Closed, ConversationStatus::Closed).is_ok());
}

#[test]
fn test_all_invalid_transitions_fail() {
    // Invalid transitions (not in allowed list):
    // Open -> Closed
    assert!(validate_transition(ConversationStatus::Open, ConversationStatus::Closed).is_err());

    // Snoozed -> Resolved
    assert!(validate_transition(ConversationStatus::Snoozed, ConversationStatus::Resolved).is_err());

    // Snoozed -> Closed
    assert!(validate_transition(ConversationStatus::Snoozed, ConversationStatus::Closed).is_err());

    // Resolved -> Snoozed
    assert!(validate_transition(ConversationStatus::Resolved, ConversationStatus::Snoozed).is_err());

    // Resolved -> Closed
    assert!(validate_transition(ConversationStatus::Resolved, ConversationStatus::Closed).is_err());

    // Closed -> anything (Closed is terminal)
    assert!(validate_transition(ConversationStatus::Closed, ConversationStatus::Open).is_err());
    assert!(validate_transition(ConversationStatus::Closed, ConversationStatus::Snoozed).is_err());
    assert!(validate_transition(ConversationStatus::Closed, ConversationStatus::Resolved).is_err());
}
