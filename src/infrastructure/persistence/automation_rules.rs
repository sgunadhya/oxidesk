use async_trait::async_trait;
use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};
use crate::infrastructure::persistence::Database;
use crate::domain::entities::{
    ActionResult, AutomationRule, ConditionResult, RuleAction, RuleCondition, RuleEvaluationLog,
    RuleType,
};
use sqlx::Row;

#[async_trait]
impl AutomationRulesRepository for Database {
    /// Create automation rule
    async fn create_automation_rule(&self, rule: &AutomationRule) -> ApiResult<()> {
        let event_subscription_json =
            serde_json::to_string(&rule.event_subscription).map_err(|e| {
                ApiError::Internal(format!("Failed to serialize event_subscription: {}", e))
            })?;
        let condition_json = serde_json::to_string(&rule.condition)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize condition: {}", e)))?;
        let action_json = serde_json::to_string(&rule.action)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize action: {}", e)))?;

        sqlx::query(
            "INSERT INTO automation_rules (id, name, description, enabled, rule_type, event_subscription, condition, action, priority, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
            .bind(&rule.id)
            .bind(&rule.name)
            .bind(&rule.description)
            .bind(rule.enabled)
            .bind(rule.rule_type.to_string())
            .bind(&event_subscription_json)
            .bind(&condition_json)
            .bind(&action_json)
            .bind(rule.priority)
            .bind(&rule.created_at)
            .bind(&rule.updated_at)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    /// Get automation rule by ID
    async fn get_automation_rule_by_id(&self, id: &str) -> ApiResult<Option<AutomationRule>> {
        let row = sqlx::query(
            "SELECT id, name, description, CAST(enabled AS INTEGER) as enabled, rule_type, event_subscription, condition, action, priority, created_at, updated_at
             FROM automation_rules
             WHERE id = ?",
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database error fetching automation rule: {:?}", e);
                ApiError::Internal(format!("Database error: {}", e))
            })?;

        if let Some(row) = row {
            let rule_type_str: String = row.try_get("rule_type")?;
            let rule_type = rule_type_str.parse::<RuleType>().map_err(|e| {
                ApiError::Internal(format!(
                    "Failed to parse rule_type '{}': {}",
                    rule_type_str, e
                ))
            })?;

            let event_subscription_str: String = row.try_get("event_subscription")?;
            let event_subscription: Vec<String> = serde_json::from_str(&event_subscription_str)
                .map_err(|e| {
                    ApiError::Internal(format!("Failed to deserialize event_subscription: {}", e))
                })?;

            let condition_str: String = row.try_get("condition")?;
            let condition: RuleCondition = serde_json::from_str(&condition_str).map_err(|e| {
                ApiError::Internal(format!("Failed to deserialize condition: {}", e))
            })?;

            let action_str: String = row.try_get("action")?;
            let action: RuleAction = serde_json::from_str(&action_str)
                .map_err(|e| ApiError::Internal(format!("Failed to deserialize action: {}", e)))?;

            Ok(Some(AutomationRule {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row
                    .try_get::<Option<String>, _>("description")
                    .ok()
                    .flatten(),
                enabled: {
                    let enabled_int: i32 = row.try_get("enabled")?;
                    enabled_int != 0
                },
                rule_type,
                event_subscription,
                condition,
                action,
                priority: row.try_get("priority")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }
    /// Get automation rule by name
    async fn get_automation_rule_by_name(
        &self,
        name: &str,
    ) -> ApiResult<Option<AutomationRule>> {
        let row = sqlx::query(
            "SELECT id, name, description, CAST(enabled AS INTEGER) as enabled, rule_type, event_subscription, condition, action, priority, created_at, updated_at
             FROM automation_rules
             WHERE name = ?",
        )
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let rule_type_str: String = row.try_get("rule_type")?;
            let rule_type = rule_type_str.parse::<RuleType>().map_err(|e| {
                ApiError::Internal(format!(
                    "Failed to parse rule_type '{}': {}",
                    rule_type_str, e
                ))
            })?;

            let event_subscription_str: String = row.try_get("event_subscription")?;
            let event_subscription: Vec<String> = serde_json::from_str(&event_subscription_str)
                .map_err(|e| {
                    ApiError::Internal(format!("Failed to deserialize event_subscription: {}", e))
                })?;

            let condition_str: String = row.try_get("condition")?;
            let condition: RuleCondition = serde_json::from_str(&condition_str).map_err(|e| {
                ApiError::Internal(format!("Failed to deserialize condition: {}", e))
            })?;

            let action_str: String = row.try_get("action")?;
            let action: RuleAction = serde_json::from_str(&action_str)
                .map_err(|e| ApiError::Internal(format!("Failed to deserialize action: {}", e)))?;

            Ok(Some(AutomationRule {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row
                    .try_get::<Option<String>, _>("description")
                    .ok()
                    .flatten(),
                enabled: {
                    let enabled_int: i32 = row.try_get("enabled")?;
                    enabled_int != 0
                },
                rule_type,
                event_subscription,
                condition,
                action,
                priority: row.try_get("priority")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }
    /// Get all automation rules (with optional enabled filter)
    async fn get_automation_rules(&self, enabled_only: bool) -> ApiResult<Vec<AutomationRule>> {
        let query = if enabled_only {
            "SELECT id, name, description, CAST(enabled AS INTEGER) as enabled, rule_type, event_subscription, condition, action, priority, created_at, updated_at
             FROM automation_rules
             WHERE enabled = TRUE
             ORDER BY priority ASC, created_at ASC"
        } else {
            "SELECT id, name, description, CAST(enabled AS INTEGER) as enabled, rule_type, event_subscription, condition, action, priority, created_at, updated_at
             FROM automation_rules
             ORDER BY priority ASC, created_at ASC"
        };

        let rows = sqlx::query(query).fetch_all(&self.pool).await?;

        let mut rules = Vec::new();
        for row in rows {
            let rule_type_str: String = row.try_get("rule_type")?;
            let rule_type = rule_type_str.parse::<RuleType>().map_err(|e| {
                ApiError::Internal(format!(
                    "Failed to parse rule_type '{}': {}",
                    rule_type_str, e
                ))
            })?;

            let event_subscription_str: String = row.try_get("event_subscription")?;
            let event_subscription: Vec<String> = serde_json::from_str(&event_subscription_str)
                .map_err(|e| {
                    ApiError::Internal(format!("Failed to deserialize event_subscription: {}", e))
                })?;

            let condition_str: String = row.try_get("condition")?;
            let condition: RuleCondition = serde_json::from_str(&condition_str).map_err(|e| {
                ApiError::Internal(format!("Failed to deserialize condition: {}", e))
            })?;

            let action_str: String = row.try_get("action")?;
            let action: RuleAction = serde_json::from_str(&action_str)
                .map_err(|e| ApiError::Internal(format!("Failed to deserialize action: {}", e)))?;

            rules.push(AutomationRule {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row
                    .try_get::<Option<String>, _>("description")
                    .ok()
                    .flatten(),
                enabled: {
                    let enabled_int: i32 = row.try_get("enabled")?;
                    enabled_int != 0
                },
                rule_type,
                event_subscription,
                condition,
                action,
                priority: row.try_get("priority")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(rules)
    }
    /// Get enabled rules that subscribe to a specific event
    async fn get_enabled_rules_for_event(
        &self,
        event_type: &str,
    ) -> ApiResult<Vec<AutomationRule>> {
        // Get all enabled rules
        let all_rules = self.get_automation_rules(true).await?;

        // Filter by event subscription
        let matching_rules: Vec<AutomationRule> = all_rules
            .into_iter()
            .filter(|rule| rule.event_subscription.contains(&event_type.to_string()))
            .collect();

        Ok(matching_rules)
    }
    /// Update automation rule
    async fn update_automation_rule(&self, rule: &AutomationRule) -> ApiResult<()> {
        let event_subscription_json =
            serde_json::to_string(&rule.event_subscription).map_err(|e| {
                ApiError::Internal(format!("Failed to serialize event_subscription: {}", e))
            })?;
        let condition_json = serde_json::to_string(&rule.condition)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize condition: {}", e)))?;
        let action_json = serde_json::to_string(&rule.action)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize action: {}", e)))?;

        sqlx::query(
            "UPDATE automation_rules
             SET name = ?, description = ?, enabled = ?, rule_type = ?, event_subscription = ?, condition = ?, action = ?, priority = ?, updated_at = ?
             WHERE id = ?",
        )
            .bind(&rule.name)
            .bind(&rule.description)
            .bind(rule.enabled)
            .bind(rule.rule_type.to_string())
            .bind(&event_subscription_json)
            .bind(&condition_json)
            .bind(&action_json)
            .bind(rule.priority)
            .bind(&rule.updated_at)
            .bind(&rule.id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    /// Delete automation rule
    async fn delete_automation_rule(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM automation_rules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    /// Enable automation rule
    async fn enable_automation_rule(&self, id: &str) -> ApiResult<()> {
        let updated_at = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE automation_rules SET enabled = TRUE, updated_at = ? WHERE id = ?")
            .bind(&updated_at)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    /// Disable automation rule
    async fn disable_automation_rule(&self, id: &str) -> ApiResult<()> {
        let updated_at = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE automation_rules SET enabled = FALSE, updated_at = ? WHERE id = ?")
            .bind(&updated_at)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    /// Create rule evaluation log
    async fn create_rule_evaluation_log(&self, log: &RuleEvaluationLog) -> ApiResult<()> {
        let condition_result_str = log.condition_result.as_ref().map(|r| r.to_string());
        let action_result_str = log.action_result.as_ref().map(|r| r.to_string());

        sqlx::query(
            "INSERT INTO rule_evaluation_logs (id, rule_id, rule_name, event_type, conversation_id, matched, condition_result, action_executed, action_result, error_message, evaluation_time_ms, evaluated_at, cascade_depth)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
            .bind(&log.id)
            .bind(&log.rule_id)
            .bind(&log.rule_name)
            .bind(&log.event_type)
            .bind(&log.conversation_id)
            .bind(log.matched)
            .bind(&condition_result_str)
            .bind(log.action_executed)
            .bind(&action_result_str)
            .bind(&log.error_message)
            .bind(log.evaluation_time_ms)
            .bind(&log.evaluated_at)
            .bind(log.cascade_depth as i32)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    /// Get rule evaluation logs with optional filters
    async fn get_rule_evaluation_logs(
        &self,
        rule_id: Option<&str>,
        conversation_id: Option<&str>,
        event_type: Option<&str>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> ApiResult<Vec<RuleEvaluationLog>> {
        let mut query = String::from(
            "SELECT id, rule_id, rule_name, event_type, conversation_id, CAST(matched AS INTEGER) as matched, condition_result, CAST(action_executed AS INTEGER) as action_executed, action_result, error_message, evaluation_time_ms, evaluated_at, cascade_depth
             FROM rule_evaluation_logs
             WHERE 1=1"
        );

        let mut params: Vec<String> = Vec::new();

        if rule_id.is_some() {
            query.push_str(" AND rule_id = ?");
            params.push(rule_id.unwrap().to_string());
        }

        if conversation_id.is_some() {
            query.push_str(" AND conversation_id = ?");
            params.push(conversation_id.unwrap().to_string());
        }

        if event_type.is_some() {
            query.push_str(" AND event_type = ?");
            params.push(event_type.unwrap().to_string());
        }

        query.push_str(" ORDER BY evaluated_at DESC");

        if limit.is_some() {
            query.push_str(" LIMIT ?");
            params.push(limit.unwrap().to_string());
        }

        if offset.is_some() {
            query.push_str(" OFFSET ?");
            params.push(offset.unwrap().to_string());
        }

        let mut sql_query = sqlx::query(&query);
        for param in &params {
            sql_query = sql_query.bind(param);
        }

        let rows = sql_query.fetch_all(&self.pool).await?;

        let mut logs = Vec::new();
        for row in rows {
            let condition_result_str: Option<String> = row.try_get("condition_result")?;
            let condition_result =
                condition_result_str.and_then(|s| s.parse::<ConditionResult>().ok());

            let action_result_str: Option<String> = row.try_get("action_result")?;
            let action_result = action_result_str.and_then(|s| s.parse::<ActionResult>().ok());

            let cascade_depth: i32 = row.try_get("cascade_depth")?;

            // SQLite stores BOOLEAN as INTEGER
            let matched: i32 = row.try_get("matched")?;
            let matched = matched != 0;

            let action_executed: i32 = row.try_get("action_executed")?;
            let action_executed = action_executed != 0;

            logs.push(RuleEvaluationLog {
                id: row.try_get("id")?,
                rule_id: row.try_get("rule_id")?,
                rule_name: row.try_get("rule_name")?,
                event_type: row.try_get("event_type")?,
                conversation_id: row
                    .try_get::<Option<String>, _>("conversation_id")
                    .ok()
                    .flatten(),
                matched,
                condition_result,
                action_executed,
                action_result,
                error_message: row
                    .try_get::<Option<String>, _>("error_message")
                    .ok()
                    .flatten(),
                evaluation_time_ms: row.try_get("evaluation_time_ms")?,
                evaluated_at: row.try_get("evaluated_at")?,
                cascade_depth: cascade_depth as u32,
            });
        }

        Ok(logs)
    }
    /// Get evaluation logs for a specific rule
    async fn get_evaluation_logs_by_rule(
        &self,
        rule_id: &str,
    ) -> ApiResult<Vec<RuleEvaluationLog>> {
        self.get_rule_evaluation_logs(Some(rule_id), None, None, None, None)
            .await
    }
    /// Get evaluation logs for a specific conversation
    async fn get_evaluation_logs_by_conversation(
        &self,
        conversation_id: &str,
    ) -> ApiResult<Vec<RuleEvaluationLog>> {
        self.get_rule_evaluation_logs(None, Some(conversation_id), None, None, None)
            .await
    }
    /// Get evaluation logs for a specific rule
    async fn get_rule_evaluation_logs_by_rule(
        &self,
        rule_id: &str,
        limit: i32,
        offset: i32,
    ) -> ApiResult<Vec<RuleEvaluationLog>> {
        self.get_rule_evaluation_logs(Some(rule_id), None, None, Some(limit), Some(offset))
            .await
    }
}

#[async_trait]
pub trait AutomationRulesRepository : Send + Sync {
    /// Create automation rule
    async fn create_automation_rule(&self, rule: &AutomationRule) -> ApiResult<()>;
    /// Get automation rule by ID
    async fn get_automation_rule_by_id(&self, id: &str) -> ApiResult<Option<AutomationRule>>;
    /// Get automation rule by name
    async fn get_automation_rule_by_name(
        &self,
        name: &str,
    ) -> ApiResult<Option<AutomationRule>>;
    /// Get all automation rules (with optional enabled filter)
    async fn get_automation_rules(&self, enabled_only: bool) -> ApiResult<Vec<AutomationRule>>;
    /// Get enabled rules that subscribe to a specific event
    async fn get_enabled_rules_for_event(
        &self,
        event_type: &str,
    ) -> ApiResult<Vec<AutomationRule>>;
    /// Update automation rule
    async fn update_automation_rule(&self, rule: &AutomationRule) -> ApiResult<()>;
    /// Delete automation rule
    async fn delete_automation_rule(&self, id: &str) -> ApiResult<()>;
    /// Enable automation rule
    async fn enable_automation_rule(&self, id: &str) -> ApiResult<()>;
    /// Disable automation rule
    async fn disable_automation_rule(&self, id: &str) -> ApiResult<()>;
    /// Create rule evaluation log
    async fn create_rule_evaluation_log(&self, log: &RuleEvaluationLog) -> ApiResult<()>;
    /// Get rule evaluation logs with optional filters
    async fn get_rule_evaluation_logs(
        &self,
        rule_id: Option<&str>,
        conversation_id: Option<&str>,
        event_type: Option<&str>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> ApiResult<Vec<RuleEvaluationLog>>;
    /// Get evaluation logs for a specific rule
    async fn get_evaluation_logs_by_rule(
        &self,
        rule_id: &str,
    ) -> ApiResult<Vec<RuleEvaluationLog>>;
    /// Get evaluation logs for a specific conversation
    async fn get_evaluation_logs_by_conversation(
        &self,
        conversation_id: &str,
    ) -> ApiResult<Vec<RuleEvaluationLog>>;
    /// Get evaluation logs for a specific rule
    async fn get_rule_evaluation_logs_by_rule(
        &self,
        rule_id: &str,
        limit: i32,
        offset: i32,
    ) -> ApiResult<Vec<RuleEvaluationLog>>;
}
