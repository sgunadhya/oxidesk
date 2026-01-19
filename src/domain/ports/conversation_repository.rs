use crate::infrastructure::http::middleware::error::ApiResult;
use crate::domain::entities::{
    AssignmentHistory, Conversation, ConversationStatus, CreateConversation, Priority,
};

#[async_trait::async_trait]
pub trait ConversationRepository: Send + Sync {
    async fn create_conversation(&self, create: &CreateConversation) -> ApiResult<Conversation>;

    async fn get_conversation_by_id(&self, id: &str) -> ApiResult<Option<Conversation>>;

    async fn get_conversation_by_reference_number(
        &self,
        reference_number: i64,
    ) -> ApiResult<Option<Conversation>>;

    async fn update_conversation_status(
        &self,
        conversation_id: &str,
        status: ConversationStatus,
    ) -> ApiResult<()>;

    async fn update_conversation_fields(
        &self,
        id: &str,
        status: ConversationStatus,
        resolved_at: Option<String>,
        closed_at: Option<String>,
        snoozed_until: Option<String>,
    ) -> ApiResult<Conversation>;

    async fn list_conversations(
        &self,
        limit: i64,
        offset: i64,
        status: Option<ConversationStatus>,
        inbox_id: Option<String>,
        contact_id: Option<String>,
    ) -> ApiResult<Vec<Conversation>>;

    async fn count_conversations(
        &self,
        status: Option<ConversationStatus>,
        inbox_id: Option<String>,
        contact_id: Option<String>,
    ) -> ApiResult<i64>;

    async fn set_conversation_priority(
        &self,
        conversation_id: &str,
        priority: &Priority,
    ) -> ApiResult<()>;

    async fn clear_conversation_priority(&self, conversation_id: &str) -> ApiResult<()>;

    // Assignment operations
    async fn assign_conversation_to_user(
        &self,
        conversation_id: &str,
        user_id: Option<String>,
        assigned_by: Option<String>,
    ) -> ApiResult<()>;

    async fn assign_conversation_to_team(
        &self,
        conversation_id: &str,
        team_id: Option<String>,
        assigned_by: Option<String>,
    ) -> ApiResult<()>;

    async fn add_conversation_participant(
        &self,
        conversation_id: &str,
        user_id: &str,
        role: &str,
    ) -> ApiResult<()>;

    async fn get_unassigned_conversations(
        &self,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)>;

    async fn get_user_assigned_conversations(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)>;

    async fn get_team_conversations(
        &self,
        team_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)>;

    async fn unassign_agent_open_conversations(&self, user_id: &str) -> ApiResult<u64>;

    async fn unassign_conversation_user(&self, conversation_id: &str) -> ApiResult<()>;

    async fn record_assignment(&self, history: &AssignmentHistory) -> ApiResult<()>;

    async fn get_assignment_history(
        &self,
        conversation_id: &str,
    ) -> ApiResult<Vec<AssignmentHistory>>;

    async fn find_contact_by_user_id(
        &self,
        user_id: &str,
    ) -> ApiResult<Option<crate::domain::entities::Contact>>;
}
