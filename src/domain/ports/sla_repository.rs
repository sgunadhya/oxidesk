use crate::infrastructure::http::middleware::error::ApiResult;
use crate::domain::entities::{AppliedSla, AppliedSlaStatus, SlaEvent, SlaEventStatus, SlaEventType, SlaPolicy};

/// Repository for SLA operations
#[async_trait::async_trait]
pub trait SlaRepository: Send + Sync {
    // SLA Policy operations
    async fn create_sla_policy(&self, policy: &SlaPolicy) -> ApiResult<()>;
    async fn get_sla_policy(&self, policy_id: &str) -> ApiResult<Option<SlaPolicy>>;
    async fn get_sla_policy_by_name(&self, name: &str) -> ApiResult<Option<SlaPolicy>>;
    async fn list_sla_policies(&self, limit: i64, offset: i64) -> ApiResult<(Vec<SlaPolicy>, i64)>;
    async fn update_sla_policy(
        &self,
        policy_id: &str,
        name: Option<&str>,
        description: Option<Option<&str>>,
        first_response_time: Option<&str>,
        resolution_time: Option<&str>,
        next_response_time: Option<&str>,
    ) -> ApiResult<()>;
    async fn delete_sla_policy(&self, policy_id: &str) -> ApiResult<()>;

    // Applied SLA operations
    async fn create_applied_sla(&self, applied_sla: &AppliedSla) -> ApiResult<()>;
    async fn get_applied_sla(&self, applied_sla_id: &str) -> ApiResult<Option<AppliedSla>>;
    async fn get_applied_sla_by_id(&self, applied_sla_id: &str) -> ApiResult<Option<AppliedSla>>;
    async fn get_applied_sla_by_conversation(
        &self,
        conversation_id: &str,
    ) -> ApiResult<Option<AppliedSla>>;
    async fn list_applied_slas(
        &self,
        status_filter: Option<AppliedSlaStatus>,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<AppliedSla>, i64)>;
    async fn update_applied_sla_status(
        &self,
        applied_sla_id: &str,
        status: AppliedSlaStatus,
    ) -> ApiResult<()>;

    // SLA Event operations
    async fn create_sla_event(&self, event: &SlaEvent) -> ApiResult<()>;
    async fn get_sla_event(&self, event_id: &str) -> ApiResult<Option<SlaEvent>>;
    async fn get_sla_events_by_applied_sla(&self, applied_sla_id: &str)
        -> ApiResult<Vec<SlaEvent>>;
    async fn get_pending_sla_event(
        &self,
        applied_sla_id: &str,
        event_type: SlaEventType,
    ) -> ApiResult<Option<SlaEvent>>;
    async fn get_pending_events_past_deadline(&self) -> ApiResult<Vec<SlaEvent>>;
    async fn mark_sla_event_met(&self, event_id: &str, met_at: &str) -> ApiResult<()>;
    async fn mark_sla_event_breached(&self, event_id: &str, breached_at: &str) -> ApiResult<()>;

    // Holiday operations
    async fn is_holiday(&self, date: &str) -> ApiResult<bool>;
}
