use serde::{Deserialize, Serialize};

/// Audit record of rule evaluation for observability and compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvaluationLog {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub event_type: String,
    pub conversation_id: Option<String>,
    pub matched: bool,
    pub condition_result: Option<ConditionResult>,
    pub action_executed: bool,
    pub action_result: Option<ActionResult>,
    pub error_message: Option<String>,
    pub evaluation_time_ms: i64,
    pub evaluated_at: String,
    pub cascade_depth: u32,
}

/// Result of condition evaluation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConditionResult {
    True,
    False,
    Error,
}

impl std::fmt::Display for ConditionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConditionResult::True => write!(f, "true"),
            ConditionResult::False => write!(f, "false"),
            ConditionResult::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for ConditionResult {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "true" => Ok(ConditionResult::True),
            "false" => Ok(ConditionResult::False),
            "error" => Ok(ConditionResult::Error),
            _ => Err(format!("Invalid condition result: {}", s)),
        }
    }
}

/// Result of action execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActionResult {
    Success,
    Failure,
    Error,
    Skipped,
}

impl std::fmt::Display for ActionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionResult::Success => write!(f, "success"),
            ActionResult::Failure => write!(f, "failure"),
            ActionResult::Error => write!(f, "error"),
            ActionResult::Skipped => write!(f, "skipped"),
        }
    }
}

impl std::str::FromStr for ActionResult {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "success" => Ok(ActionResult::Success),
            "failure" => Ok(ActionResult::Failure),
            "error" => Ok(ActionResult::Error),
            "skipped" => Ok(ActionResult::Skipped),
            _ => Err(format!("Invalid action result: {}", s)),
        }
    }
}

impl RuleEvaluationLog {
    /// Create a new evaluation log
    pub fn new(
        rule_id: String,
        rule_name: String,
        event_type: String,
        conversation_id: Option<String>,
        cascade_depth: u32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            rule_id,
            rule_name,
            event_type,
            conversation_id,
            matched: false,
            condition_result: None,
            action_executed: false,
            action_result: None,
            error_message: None,
            evaluation_time_ms: 0,
            evaluated_at: chrono::Utc::now().to_rfc3339(),
            cascade_depth,
        }
    }

    /// Set condition evaluation result
    pub fn set_condition_result(&mut self, result: ConditionResult) {
        self.condition_result = Some(result);
    }

    /// Set action execution result
    pub fn set_action_result(&mut self, result: ActionResult) {
        self.action_executed = !matches!(result, ActionResult::Skipped);
        self.action_result = Some(result);
    }

    /// Set error message
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    /// Set evaluation time
    pub fn set_evaluation_time(&mut self, time_ms: i64) {
        self.evaluation_time_ms = time_ms;
    }

    /// Mark rule as matched
    pub fn set_matched(&mut self, matched: bool) {
        self.matched = matched;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_result_serialization() {
        let result = ConditionResult::True;
        let json = serde_json::to_string(&result).unwrap();
        assert_eq!(json, "\"true\"");

        let deserialized: ConditionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ConditionResult::True);
    }

    #[test]
    fn test_action_result_serialization() {
        let result = ActionResult::Success;
        let json = serde_json::to_string(&result).unwrap();
        assert_eq!(json, "\"success\"");

        let deserialized: ActionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ActionResult::Success);
    }

    #[test]
    fn test_evaluation_log_creation() {
        let log = RuleEvaluationLog::new(
            "rule-123".to_string(),
            "Test Rule".to_string(),
            "conversation.tags_changed".to_string(),
            Some("conv-456".to_string()),
            0,
        );

        assert_eq!(log.rule_id, "rule-123");
        assert_eq!(log.rule_name, "Test Rule");
        assert_eq!(log.event_type, "conversation.tags_changed");
        assert_eq!(log.conversation_id, Some("conv-456".to_string()));
        assert_eq!(log.cascade_depth, 0);
        assert!(!log.matched);
        assert!(!log.action_executed);
    }

    #[test]
    fn test_evaluation_log_mutations() {
        let mut log = RuleEvaluationLog::new(
            "rule-123".to_string(),
            "Test Rule".to_string(),
            "conversation.tags_changed".to_string(),
            None,
            0,
        );

        log.set_matched(true);
        assert!(log.matched);

        log.set_condition_result(ConditionResult::True);
        assert_eq!(log.condition_result, Some(ConditionResult::True));

        log.set_action_result(ActionResult::Success);
        assert_eq!(log.action_result, Some(ActionResult::Success));
        assert!(log.action_executed);

        log.set_error("Test error".to_string());
        assert_eq!(log.error_message, Some("Test error".to_string()));

        log.set_evaluation_time(123);
        assert_eq!(log.evaluation_time_ms, 123);
    }
}
