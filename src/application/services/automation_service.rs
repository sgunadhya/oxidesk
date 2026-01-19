use crate::domain::ports::automation_repository::AutomationRepository;
use crate::domain::entities::{
    ActionResult, AutomationRule, ConditionResult, Conversation, RuleEvaluationLog,
};
use crate::domain::services::action_executor::ActionExecutor;
use crate::domain::services::condition_evaluator::ConditionEvaluator;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct AutomationConfig {
    pub cascade_max_depth: u32,
    pub condition_timeout_secs: u64,
    pub action_timeout_secs: u64,
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self {
            cascade_max_depth: 3,
            condition_timeout_secs: 5,
            action_timeout_secs: 10,
        }
    }
}

#[derive(Clone)]
pub struct AutomationService {
    automation_repo: Arc<dyn AutomationRepository>,
    condition_evaluator: ConditionEvaluator,
    action_executor: ActionExecutor,
    config: AutomationConfig,
}

impl AutomationService {
    pub fn new(
        automation_repo: Arc<dyn AutomationRepository>,
        action_executor: ActionExecutor,
        config: AutomationConfig,
    ) -> Self {
        Self {
            automation_repo,
            condition_evaluator: ConditionEvaluator::new(),
            action_executor,
            config,
        }
    }

    /// Handle a conversation-related event
    pub async fn handle_conversation_event(
        &self,
        event_type: &str,
        conversation: &Conversation,
        executed_by: &str,
    ) -> Result<(), String> {
        self.handle_conversation_event_with_depth(event_type, conversation, executed_by, 0)
            .await
    }

    /// Handle a conversation-related event with cascade depth tracking
    pub async fn handle_conversation_event_with_depth(
        &self,
        event_type: &str,
        conversation: &Conversation,
        executed_by: &str,
        cascade_depth: u32,
    ) -> Result<(), String> {
        // Check cascade depth limit
        if cascade_depth > self.config.cascade_max_depth {
            tracing::warn!(
                "Cascade depth {} exceeds limit {} for conversation {}, skipping automation",
                cascade_depth,
                self.config.cascade_max_depth,
                conversation.id
            );
            return Ok(());
        }

        tracing::info!(
            "Processing automation rules for event '{}' on conversation {} (depth={})",
            event_type,
            conversation.id,
            cascade_depth
        );

        // Get enabled rules that subscribe to this event type
        let rules = self
            .automation_repo
            .get_enabled_rules_for_event(event_type)
            .await
            .map_err(|e| format!("Failed to fetch rules: {}", e))?;

        if rules.is_empty() {
            tracing::debug!("No enabled rules found for event '{}'", event_type);
            return Ok(());
        }

        tracing::info!(
            "Found {} enabled rule(s) for event '{}'",
            rules.len(),
            event_type
        );

        // Sort rules by priority (lower number = higher priority, executes last to win conflicts)
        let mut sorted_rules = rules;
        sorted_rules.sort_by_key(|r| std::cmp::Reverse(r.priority));

        // Evaluate and execute each rule
        for rule in sorted_rules {
            if let Err(e) = self
                .evaluate_and_execute_rule(
                    &rule,
                    event_type,
                    conversation,
                    executed_by,
                    cascade_depth,
                )
                .await
            {
                // Log error but continue with other rules
                tracing::error!("Error evaluating rule '{}' ({}): {}", rule.name, rule.id, e);
            }
        }

        Ok(())
    }

    /// Evaluate a single rule and execute its action if condition matches
    async fn evaluate_and_execute_rule(
        &self,
        rule: &AutomationRule,
        event_type: &str,
        conversation: &Conversation,
        executed_by: &str,
        cascade_depth: u32,
    ) -> Result<(), String> {
        let start_time = Instant::now();

        tracing::debug!(
            "Evaluating rule '{}' ({}) for conversation {}",
            rule.name,
            rule.id,
            conversation.id
        );

        // Evaluate condition
        let (condition_result, condition_matched, condition_error) = match self
            .condition_evaluator
            .evaluate(&rule.condition, conversation)
            .await
        {
            Ok(true) => (ConditionResult::True, true, None),
            Ok(false) => (ConditionResult::False, false, None),
            Err(e) => {
                tracing::error!("Condition evaluation error for rule '{}': {}", rule.name, e);
                (ConditionResult::Error, false, Some(e.to_string()))
            }
        };

        // Execute action if condition matched
        let (action_executed, action_result, action_error) = if condition_matched {
            tracing::info!(
                "Condition matched for rule '{}', executing action {:?}",
                rule.name,
                rule.action.action_type
            );

            match self
                .action_executor
                .execute(&rule.action, &conversation.id, executed_by)
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        "Action {:?} executed successfully for rule '{}'",
                        rule.action.action_type,
                        rule.name
                    );
                    (true, ActionResult::Success, None)
                }
                Err(e) => {
                    tracing::error!("Action execution error for rule '{}': {}", rule.name, e);
                    (false, ActionResult::Error, Some(e.to_string()))
                }
            }
        } else {
            tracing::debug!(
                "Condition not matched for rule '{}', skipping action",
                rule.name
            );
            (false, ActionResult::Skipped, None)
        };

        let evaluation_time_ms = start_time.elapsed().as_millis() as i64;

        // Create evaluation log
        let error_message = condition_error.or(action_error);
        let log = RuleEvaluationLog {
            id: uuid::Uuid::new_v4().to_string(),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            event_type: event_type.to_string(),
            conversation_id: Some(conversation.id.clone()),
            matched: true, // Rule matched event subscription
            condition_result: Some(condition_result),
            action_executed,
            action_result: Some(action_result),
            error_message,
            evaluation_time_ms,
            evaluated_at: chrono::Utc::now().to_rfc3339(),
            cascade_depth,
        };

        self.automation_repo
            .create_rule_evaluation_log(&log)
            .await
            .map_err(|e| format!("Failed to create evaluation log: {}", e))?;

        tracing::info!(
            "Rule '{}' evaluation complete: matched={}, condition={:?}, action_executed={}, time={}ms",
            rule.name,
            log.matched,
            log.condition_result,
            action_executed,
            evaluation_time_ms
        );

        Ok(())
    }

    // Proxy methods for AutomationRepository

    pub async fn create_automation_rule(&self, rule: &AutomationRule) -> Result<(), String> {
        self.automation_repo
            .create_automation_rule(rule)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_automation_rule_by_id(
        &self,
        id: &str,
    ) -> Result<Option<AutomationRule>, String> {
        self.automation_repo
            .get_automation_rule_by_id(id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_automation_rules(
        &self,
        enabled_only: bool,
    ) -> Result<Vec<AutomationRule>, String> {
        self.automation_repo
            .get_automation_rules(enabled_only)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn update_automation_rule(&self, rule: &AutomationRule) -> Result<(), String> {
        self.automation_repo
            .update_automation_rule(rule)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn delete_automation_rule(&self, id: &str) -> Result<(), String> {
        self.automation_repo
            .delete_automation_rule(id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn enable_automation_rule(&self, id: &str) -> Result<(), String> {
        self.automation_repo
            .enable_automation_rule(id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn disable_automation_rule(&self, id: &str) -> Result<(), String> {
        self.automation_repo
            .disable_automation_rule(id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_rule_evaluation_logs(
        &self,
        rule_id: Option<&str>,
        conversation_id: Option<&str>,
        event_type: Option<&str>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<RuleEvaluationLog>, String> {
        self.automation_repo
            .get_rule_evaluation_logs(rule_id, conversation_id, event_type, limit, offset)
            .await
            .map_err(|e| e.to_string())
    }
}
