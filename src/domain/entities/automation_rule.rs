use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Automation rule configuration defining when and how to automate conversation management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub rule_type: RuleType,
    pub event_subscription: Vec<String>,
    pub condition: RuleCondition,
    pub action: RuleAction,
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Rule category (type of automation)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleType {
    ConversationUpdate,
    MessageReceived,
    AssignmentChanged,
}

impl std::fmt::Display for RuleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleType::ConversationUpdate => write!(f, "conversation_update"),
            RuleType::MessageReceived => write!(f, "message_received"),
            RuleType::AssignmentChanged => write!(f, "assignment_changed"),
        }
    }
}

impl std::str::FromStr for RuleType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "conversation_update" => Ok(RuleType::ConversationUpdate),
            "message_received" => Ok(RuleType::MessageReceived),
            "assignment_changed" => Ok(RuleType::AssignmentChanged),
            _ => Err(format!("Invalid rule type: {}", s)),
        }
    }
}

/// Rule condition expression that evaluates to true or false
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operator", rename_all = "snake_case")]
pub enum RuleCondition {
    Simple {
        attribute: String,
        comparison: ComparisonOperator,
        value: serde_json::Value,
    },
    And {
        conditions: Vec<RuleCondition>,
    },
    Or {
        conditions: Vec<RuleCondition>,
    },
    Not {
        condition: Box<RuleCondition>,
    },
}

/// Comparison operators for simple conditions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    Contains,
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    In,
    NotIn,
}

/// Rule action to execute when condition is met
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAction {
    pub action_type: ActionType,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Action types for automation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    SetPriority,
    AssignToUser,
    AssignToTeam,
    AddTag,
    RemoveTag,
    ChangeStatus,
}

// Validation methods

impl AutomationRule {
    pub fn new(
        name: String,
        rule_type: RuleType,
        event_subscription: Vec<String>,
        condition: RuleCondition,
        action: RuleAction,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description: None,
            enabled: true,
            rule_type,
            event_subscription,
            condition,
            action,
            priority: 100,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Validate rule configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate name
        if self.name.is_empty() || self.name.len() > 200 {
            return Err("Rule name must be 1-200 characters".to_string());
        }

        // Validate event subscription
        if self.event_subscription.is_empty() {
            return Err("Rule must subscribe to at least one event".to_string());
        }

        // Validate priority
        if self.priority < 1 || self.priority > 1000 {
            return Err("Priority must be between 1 and 1000".to_string());
        }

        // Validate condition
        self.condition.validate()?;

        // Validate action
        self.action.validate()?;

        Ok(())
    }
}

impl RuleCondition {
    /// Validate condition syntax
    pub fn validate(&self) -> Result<(), String> {
        match self {
            RuleCondition::Simple {
                attribute,
                comparison: _,
                value: _,
            } => {
                // Validate attribute is a known conversation field
                let valid_attributes = [
                    "tags",
                    "priority",
                    "status",
                    "assigned_user_id",
                    "assigned_team_id",
                ];
                if !valid_attributes.contains(&attribute.as_str()) {
                    return Err(format!("Invalid attribute: {}", attribute));
                }
                Ok(())
            }
            RuleCondition::And { conditions } | RuleCondition::Or { conditions } => {
                if conditions.len() < 2 {
                    return Err("AND/OR conditions must have at least 2 sub-conditions".to_string());
                }
                for condition in conditions {
                    condition.validate()?;
                }
                Ok(())
            }
            RuleCondition::Not { condition } => condition.validate(),
        }
    }
}

impl RuleAction {
    /// Validate action parameters
    pub fn validate(&self) -> Result<(), String> {
        match self.action_type {
            ActionType::SetPriority => {
                if !self.parameters.contains_key("priority") {
                    return Err("SetPriority action requires 'priority' parameter".to_string());
                }
                Ok(())
            }
            ActionType::AssignToUser => {
                if !self.parameters.contains_key("user_id") {
                    return Err("AssignToUser action requires 'user_id' parameter".to_string());
                }
                Ok(())
            }
            ActionType::AssignToTeam => {
                if !self.parameters.contains_key("team_id") {
                    return Err("AssignToTeam action requires 'team_id' parameter".to_string());
                }
                Ok(())
            }
            ActionType::AddTag | ActionType::RemoveTag => {
                if !self.parameters.contains_key("tag") {
                    return Err(format!(
                        "{:?} action requires 'tag' parameter",
                        self.action_type
                    ));
                }
                Ok(())
            }
            ActionType::ChangeStatus => {
                if !self.parameters.contains_key("status") {
                    return Err("ChangeStatus action requires 'status' parameter".to_string());
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rule_type_serialization() {
        let rule_type = RuleType::ConversationUpdate;
        let json = serde_json::to_string(&rule_type).unwrap();
        assert_eq!(json, "\"conversation_update\"");

        let deserialized: RuleType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, RuleType::ConversationUpdate);
    }

    #[test]
    fn test_simple_condition_deserialization() {
        let json = json!({
            "operator": "simple",
            "attribute": "tags",
            "comparison": "contains",
            "value": "Bug"
        });

        let condition: RuleCondition = serde_json::from_value(json).unwrap();
        match condition {
            RuleCondition::Simple {
                attribute,
                comparison,
                value,
            } => {
                assert_eq!(attribute, "tags");
                assert_eq!(comparison, ComparisonOperator::Contains);
                assert_eq!(value, json!("Bug"));
            }
            _ => panic!("Expected Simple condition"),
        }
    }

    #[test]
    fn test_and_condition_deserialization() {
        let json = json!({
            "operator": "and",
            "conditions": [
                {
                    "operator": "simple",
                    "attribute": "tags",
                    "comparison": "contains",
                    "value": "Bug"
                },
                {
                    "operator": "simple",
                    "attribute": "priority",
                    "comparison": "equals",
                    "value": "High"
                }
            ]
        });

        let condition: RuleCondition = serde_json::from_value(json).unwrap();
        match condition {
            RuleCondition::And { conditions } => {
                assert_eq!(conditions.len(), 2);
            }
            _ => panic!("Expected And condition"),
        }
    }

    #[test]
    fn test_rule_action_validation() {
        let action = RuleAction {
            action_type: ActionType::SetPriority,
            parameters: HashMap::from([("priority".to_string(), json!("High"))]),
        };
        assert!(action.validate().is_ok());

        let invalid_action = RuleAction {
            action_type: ActionType::SetPriority,
            parameters: HashMap::new(),
        };
        assert!(invalid_action.validate().is_err());
    }

    #[test]
    fn test_rule_condition_validation() {
        let valid_condition = RuleCondition::Simple {
            attribute: "tags".to_string(),
            comparison: ComparisonOperator::Contains,
            value: json!("Bug"),
        };
        assert!(valid_condition.validate().is_ok());

        let invalid_condition = RuleCondition::Simple {
            attribute: "invalid_field".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("value"),
        };
        assert!(invalid_condition.validate().is_err());
    }

    #[test]
    fn test_automation_rule_validation() {
        let rule = AutomationRule::new(
            "Test Rule".to_string(),
            RuleType::ConversationUpdate,
            vec!["conversation.tags_changed".to_string()],
            RuleCondition::Simple {
                attribute: "tags".to_string(),
                comparison: ComparisonOperator::Contains,
                value: json!("Bug"),
            },
            RuleAction {
                action_type: ActionType::SetPriority,
                parameters: HashMap::from([("priority".to_string(), json!("High"))]),
            },
        );
        assert!(rule.validate().is_ok());
    }
}
