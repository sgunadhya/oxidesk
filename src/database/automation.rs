use crate::api::middleware::error::ApiResult;
use crate::database::automation_rules::AutomationRulesRepository;
use crate::database::Database;
use crate::domain::ports::automation_repository::AutomationRepository;
use crate::models::{AutomationRule, RuleEvaluationLog};

/// Implement AutomationRepository trait for Database by delegating to AutomationRulesRepository
#[async_trait::async_trait]
impl AutomationRepository for Database {
    async fn get_enabled_rules_for_event(&self, event_type: &str) -> ApiResult<Vec<AutomationRule>> {
        <Self as AutomationRulesRepository>::get_enabled_rules_for_event(self, event_type).await
    }

    async fn create_rule_evaluation_log(&self, log: &RuleEvaluationLog) -> ApiResult<()> {
        <Self as AutomationRulesRepository>::create_rule_evaluation_log(self, log).await
    }

    async fn create_automation_rule(&self, rule: &AutomationRule) -> ApiResult<()> {
        <Self as AutomationRulesRepository>::create_automation_rule(self, rule).await
    }

    async fn get_automation_rule_by_id(&self, id: &str) -> ApiResult<Option<AutomationRule>> {
        <Self as AutomationRulesRepository>::get_automation_rule_by_id(self, id).await
    }

    async fn get_automation_rules(&self, enabled_only: bool) -> ApiResult<Vec<AutomationRule>> {
        <Self as AutomationRulesRepository>::get_automation_rules(self, enabled_only).await
    }

    async fn update_automation_rule(&self, rule: &AutomationRule) -> ApiResult<()> {
        <Self as AutomationRulesRepository>::update_automation_rule(self, rule).await
    }

    async fn delete_automation_rule(&self, id: &str) -> ApiResult<()> {
        <Self as AutomationRulesRepository>::delete_automation_rule(self, id).await
    }

    async fn enable_automation_rule(&self, id: &str) -> ApiResult<()> {
        <Self as AutomationRulesRepository>::enable_automation_rule(self, id).await
    }

    async fn disable_automation_rule(&self, id: &str) -> ApiResult<()> {
        <Self as AutomationRulesRepository>::disable_automation_rule(self, id).await
    }

    async fn get_rule_evaluation_logs(
        &self,
        rule_id: Option<&str>,
        conversation_id: Option<&str>,
        event_type: Option<&str>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> ApiResult<Vec<RuleEvaluationLog>> {
        <Self as AutomationRulesRepository>::get_rule_evaluation_logs(
            self,
            rule_id,
            conversation_id,
            event_type,
            limit,
            offset,
        )
        .await
    }
}
