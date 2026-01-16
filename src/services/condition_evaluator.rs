use crate::models::{ComparisonOperator, Conversation, ConversationStatus, RuleCondition};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum ConditionError {
    InvalidAttribute(String),
    TypeMismatch(String),
    Timeout,
    EvaluationFailed(String),
}

impl std::fmt::Display for ConditionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConditionError::InvalidAttribute(attr) => write!(f, "Invalid attribute: {}", attr),
            ConditionError::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            ConditionError::Timeout => write!(f, "Condition evaluation timeout"),
            ConditionError::EvaluationFailed(msg) => write!(f, "Evaluation failed: {}", msg),
        }
    }
}

impl std::error::Error for ConditionError {}

#[derive(Clone)]
pub struct ConditionEvaluator {
    timeout: Duration,
}

impl ConditionEvaluator {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(5),
        }
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Evaluate a condition against a conversation
    pub async fn evaluate(
        &self,
        condition: &RuleCondition,
        conversation: &Conversation,
    ) -> Result<bool, ConditionError> {
        // Wrap evaluation with timeout
        tokio::time::timeout(
            self.timeout,
            self.evaluate_internal(condition, conversation),
        )
        .await
        .map_err(|_| ConditionError::Timeout)?
    }

    fn evaluate_internal<'a>(
        &'a self,
        condition: &'a RuleCondition,
        conversation: &'a Conversation,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<bool, ConditionError>> + Send + 'a>,
    > {
        Box::pin(async move {
            match condition {
                RuleCondition::Simple {
                    attribute,
                    comparison,
                    value,
                } => {
                    self.evaluate_simple(attribute, comparison, value, conversation)
                        .await
                }
                RuleCondition::And { conditions } => {
                    self.evaluate_and(conditions, conversation).await
                }
                RuleCondition::Or { conditions } => {
                    self.evaluate_or(conditions, conversation).await
                }
                RuleCondition::Not { condition } => {
                    let result = self.evaluate_internal(condition, conversation).await?;
                    Ok(!result)
                }
            }
        })
    }

    async fn evaluate_simple(
        &self,
        attribute: &str,
        comparison: &ComparisonOperator,
        expected_value: &Value,
        conversation: &Conversation,
    ) -> Result<bool, ConditionError> {
        let attr_value = self.get_attribute_value(conversation, attribute)?;

        match comparison {
            ComparisonOperator::Contains => self.evaluate_contains(&attr_value, expected_value),
            ComparisonOperator::Equals => self.evaluate_equals(&attr_value, expected_value),
            ComparisonOperator::NotEquals => {
                Ok(!self.evaluate_equals(&attr_value, expected_value)?)
            }
            ComparisonOperator::GreaterThan => {
                self.evaluate_greater_than(&attr_value, expected_value)
            }
            ComparisonOperator::LessThan => self.evaluate_less_than(&attr_value, expected_value),
            ComparisonOperator::In => self.evaluate_in(&attr_value, expected_value),
            ComparisonOperator::NotIn => Ok(!self.evaluate_in(&attr_value, expected_value)?),
        }
    }

    fn get_attribute_value(
        &self,
        conversation: &Conversation,
        attribute: &str,
    ) -> Result<Value, ConditionError> {
        match attribute {
            "tags" => Ok(match &conversation.tags {
                Some(tags) => Value::Array(tags.iter().map(|t| Value::String(t.clone())).collect()),
                None => Value::Array(vec![]),
            }),
            "priority" => Ok(match &conversation.priority {
                Some(p) => Value::String(p.to_string()),
                None => Value::Null,
            }),
            "status" => Ok(Value::String(match conversation.status {
                ConversationStatus::Open => "open".to_string(),
                ConversationStatus::Snoozed => "snoozed".to_string(),
                ConversationStatus::Resolved => "resolved".to_string(),
                ConversationStatus::Closed => "closed".to_string(),
            })),
            "assigned_user_id" => Ok(match &conversation.assigned_user_id {
                Some(id) => Value::String(id.clone()),
                None => Value::Null,
            }),
            "assigned_team_id" => Ok(match &conversation.assigned_team_id {
                Some(id) => Value::String(id.clone()),
                None => Value::Null,
            }),
            _ => Err(ConditionError::InvalidAttribute(attribute.to_string())),
        }
    }

    fn evaluate_contains(
        &self,
        attr_value: &Value,
        expected: &Value,
    ) -> Result<bool, ConditionError> {
        match attr_value {
            Value::Array(arr) => {
                // Check if array contains the expected value
                Ok(arr.contains(expected))
            }
            Value::String(s) => {
                // Check if string contains expected string
                if let Value::String(expected_str) = expected {
                    Ok(s.contains(expected_str))
                } else {
                    Err(ConditionError::TypeMismatch(
                        "Expected string value for contains comparison".to_string(),
                    ))
                }
            }
            _ => Err(ConditionError::TypeMismatch(
                "Contains operator requires array or string".to_string(),
            )),
        }
    }

    fn evaluate_equals(
        &self,
        attr_value: &Value,
        expected: &Value,
    ) -> Result<bool, ConditionError> {
        Ok(attr_value == expected)
    }

    fn evaluate_greater_than(
        &self,
        attr_value: &Value,
        expected: &Value,
    ) -> Result<bool, ConditionError> {
        match (attr_value, expected) {
            (Value::Number(a), Value::Number(b)) => {
                if let (Some(a_f), Some(b_f)) = (a.as_f64(), b.as_f64()) {
                    Ok(a_f > b_f)
                } else {
                    Err(ConditionError::TypeMismatch(
                        "Could not convert numbers to f64".to_string(),
                    ))
                }
            }
            _ => Err(ConditionError::TypeMismatch(
                "Greater than comparison requires numbers".to_string(),
            )),
        }
    }

    fn evaluate_less_than(
        &self,
        attr_value: &Value,
        expected: &Value,
    ) -> Result<bool, ConditionError> {
        match (attr_value, expected) {
            (Value::Number(a), Value::Number(b)) => {
                if let (Some(a_f), Some(b_f)) = (a.as_f64(), b.as_f64()) {
                    Ok(a_f < b_f)
                } else {
                    Err(ConditionError::TypeMismatch(
                        "Could not convert numbers to f64".to_string(),
                    ))
                }
            }
            _ => Err(ConditionError::TypeMismatch(
                "Less than comparison requires numbers".to_string(),
            )),
        }
    }

    fn evaluate_in(&self, attr_value: &Value, expected: &Value) -> Result<bool, ConditionError> {
        if let Value::Array(expected_arr) = expected {
            Ok(expected_arr.contains(attr_value))
        } else {
            Err(ConditionError::TypeMismatch(
                "In operator requires array of values".to_string(),
            ))
        }
    }

    async fn evaluate_and(
        &self,
        conditions: &[RuleCondition],
        conversation: &Conversation,
    ) -> Result<bool, ConditionError> {
        for condition in conditions {
            let result = self.evaluate_internal(condition, conversation).await?;
            if !result {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn evaluate_or(
        &self,
        conditions: &[RuleCondition],
        conversation: &Conversation,
    ) -> Result<bool, ConditionError> {
        for condition in conditions {
            let result = self.evaluate_internal(condition, conversation).await?;
            if result {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl Default for ConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_conversation() -> Conversation {
        Conversation {
            id: "conv-123".to_string(),
            reference_number: 1001,
            status: ConversationStatus::Open,
            inbox_id: "inbox-001".to_string(),
            contact_id: "contact-001".to_string(),
            subject: Some("Test".to_string()),
            resolved_at: None,
            closed_at: None, // Feature 019
            snoozed_until: None,
            assigned_user_id: None,
            assigned_team_id: None,
            assigned_at: None,
            assigned_by: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
            tags: Some(vec!["Bug".to_string()]),
            priority: Some(crate::models::Priority::High),
        }
    }

    #[tokio::test]
    async fn test_evaluate_contains_operator() {
        let evaluator = ConditionEvaluator::new();
        let conversation = create_test_conversation();

        let condition = RuleCondition::Simple {
            attribute: "tags".to_string(),
            comparison: ComparisonOperator::Contains,
            value: json!("Bug"),
        };

        let result = evaluator.evaluate(&condition, &conversation).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[tokio::test]
    async fn test_evaluate_equals_operator() {
        let evaluator = ConditionEvaluator::new();
        let conversation = create_test_conversation();

        let condition = RuleCondition::Simple {
            attribute: "priority".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("High"),
        };

        let result = evaluator.evaluate(&condition, &conversation).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[tokio::test]
    async fn test_invalid_attribute_error() {
        let evaluator = ConditionEvaluator::new();
        let conversation = create_test_conversation();

        let condition = RuleCondition::Simple {
            attribute: "invalid_field".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("value"),
        };

        let result = evaluator.evaluate(&condition, &conversation).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConditionError::InvalidAttribute(_)
        ));
    }
}
