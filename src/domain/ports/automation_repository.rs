use crate::api::middleware::error::ApiResult;
use crate::models::{AutomationRule, RuleEvaluationLog};

/// Repository for automation rule operations
#[async_trait::async_trait]
pub trait AutomationRepository: Send + Sync {
    /// Get enabled automation rules that subscribe to a specific event type
    async fn get_enabled_rules_for_event(&self, event_type: &str) -> ApiResult<Vec<AutomationRule>>;

    /// Create a rule evaluation log entry
    async fn create_rule_evaluation_log(&self, log: &RuleEvaluationLog) -> ApiResult<()>;

    /// Create a new automation rule
    async fn create_automation_rule(&self, rule: &AutomationRule) -> ApiResult<()>;

    /// Get automation rule by ID
    async fn get_automation_rule_by_id(&self, id: &str) -> ApiResult<Option<AutomationRule>>;

    /// Get all automation rules, optionally filtering by enabled status
    async fn get_automation_rules(&self, enabled_only: bool) -> ApiResult<Vec<AutomationRule>>;

    /// Update an existing automation rule
    async fn update_automation_rule(&self, rule: &AutomationRule) -> ApiResult<()>;

    /// Delete an automation rule
    async fn delete_automation_rule(&self, id: &str) -> ApiResult<()>;

    /// Enable an automation rule
    async fn enable_automation_rule(&self, id: &str) -> ApiResult<()>;

    /// Disable an automation rule
    async fn disable_automation_rule(&self, id: &str) -> ApiResult<()>;

    /// Get rule evaluation logs with optional filters
    async fn get_rule_evaluation_logs(
        &self,
        rule_id: Option<&str>,
        conversation_id: Option<&str>,
        event_type: Option<&str>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> ApiResult<Vec<RuleEvaluationLog>>;
}
