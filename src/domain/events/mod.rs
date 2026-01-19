use crate::domain::entities::conversation::ConversationStatus;

/// System events that can trigger automation rules
#[derive(Debug, Clone)]
pub enum SystemEvent {
    ConversationCreated {
        conversation_id: String,
        inbox_id: String,
        contact_id: String,
        status: ConversationStatus,
        timestamp: String, // ISO 8601
    },
    ConversationStatusChanged {
        conversation_id: String,
        old_status: ConversationStatus,
        new_status: ConversationStatus,
        agent_id: Option<String>,
        timestamp: String, // ISO 8601
    },
    MessageReceived {
        message_id: String,
        conversation_id: String,
        contact_id: String,
        timestamp: String, // ISO 8601
    },
    MessageSent {
        message_id: String,
        conversation_id: String,
        agent_id: String,
        timestamp: String, // ISO 8601
    },
    MessageFailed {
        message_id: String,
        conversation_id: String,
        retry_count: i32,
        timestamp: String, // ISO 8601
    },
    ConversationAssigned {
        conversation_id: String,
        assigned_user_id: Option<String>,
        assigned_team_id: Option<String>,
        assigned_by: String,
        timestamp: String, // ISO 8601
    },
    ConversationUnassigned {
        conversation_id: String,
        previous_assigned_user_id: Option<String>,
        previous_assigned_team_id: Option<String>,
        unassigned_by: String,
        timestamp: String, // ISO 8601
    },
    ConversationTagsChanged {
        conversation_id: String,
        previous_tags: Vec<String>,
        new_tags: Vec<String>,
        changed_by: String,
        timestamp: String, // ISO 8601
    },
    ConversationPriorityChanged {
        conversation_id: String,
        previous_priority: Option<String>,
        new_priority: Option<String>,
        updated_by: String,
        timestamp: String, // ISO 8601
    },
    AgentAvailabilityChanged {
        agent_id: String,
        old_status: String,
        new_status: String,
        timestamp: String, // ISO 8601
        reason: String, // "manual", "inactivity_timeout", "max_idle_threshold", "login", "logout"
    },
    AgentLoggedIn {
        agent_id: String,
        user_id: String,
        timestamp: String, // ISO 8601
    },
    AgentLoggedOut {
        agent_id: String,
        user_id: String,
        timestamp: String, // ISO 8601
    },
    SlaBreached {
        event_id: String,
        applied_sla_id: String,
        conversation_id: String,
        event_type: String,  // "first_response", "resolution", "next_response"
        deadline_at: String, // ISO 8601
        breached_at: String, // ISO 8601
        timestamp: String,   // ISO 8601
    },
}
