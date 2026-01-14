use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser},
    models::{AutomationRule, RuleAction, RuleCondition, RuleEvaluationLog, RuleType},
};

// Request DTOs
#[derive(Debug, Deserialize)]
pub struct CreateAutomationRuleRequest {
    pub name: String,
    pub description: Option<String>,
    pub rule_type: RuleType,
    pub event_subscription: Vec<String>,
    pub condition: RuleCondition,
    pub action: RuleAction,
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAutomationRuleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub event_subscription: Option<Vec<String>>,
    pub condition: Option<RuleCondition>,
    pub action: Option<RuleAction>,
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct RuleFilters {
    pub enabled: Option<bool>,
    pub rule_type: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct LogFilters {
    pub rule_id: Option<String>,
    pub conversation_id: Option<String>,
    pub event_type: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// Response DTOs
#[derive(Debug, Serialize)]
pub struct AutomationRuleResponse {
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

impl From<AutomationRule> for AutomationRuleResponse {
    fn from(rule: AutomationRule) -> Self {
        Self {
            id: rule.id,
            name: rule.name,
            description: rule.description,
            enabled: rule.enabled,
            rule_type: rule.rule_type,
            event_subscription: rule.event_subscription,
            condition: rule.condition,
            action: rule.action,
            priority: rule.priority,
            created_at: rule.created_at,
            updated_at: rule.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RuleListResponse {
    pub rules: Vec<AutomationRuleResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct EvaluationLogResponse {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub event_type: String,
    pub conversation_id: Option<String>,
    pub matched: bool,
    pub condition_result: Option<String>,
    pub action_executed: bool,
    pub action_result: Option<String>,
    pub error_message: Option<String>,
    pub evaluation_time_ms: i64,
    pub evaluated_at: String,
    pub cascade_depth: u32,
}

impl From<RuleEvaluationLog> for EvaluationLogResponse {
    fn from(log: RuleEvaluationLog) -> Self {
        Self {
            id: log.id,
            rule_id: log.rule_id,
            rule_name: log.rule_name,
            event_type: log.event_type,
            conversation_id: log.conversation_id,
            matched: log.matched,
            condition_result: log.condition_result.map(|r| format!("{:?}", r)),
            action_executed: log.action_executed,
            action_result: log.action_result.map(|r| format!("{:?}", r)),
            error_message: log.error_message,
            evaluation_time_ms: log.evaluation_time_ms,
            evaluated_at: log.evaluated_at,
            cascade_depth: log.cascade_depth,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LogListResponse {
    pub logs: Vec<EvaluationLogResponse>,
    pub total: usize,
}

// API Handlers

/// Create a new automation rule
pub async fn create_automation_rule(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(request): Json<CreateAutomationRuleRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "automation:manage permission required".to_string(),
        ));
    }

    // Validate request
    if request.name.is_empty() || request.name.len() > 200 {
        return Err(ApiError::BadRequest(
            "Rule name must be between 1 and 200 characters".to_string(),
        ));
    }

    if request.event_subscription.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one event subscription is required".to_string(),
        ));
    }

    let priority = request.priority.unwrap_or(100);
    if !(1..=1000).contains(&priority) {
        return Err(ApiError::BadRequest(
            "Priority must be between 1 and 1000".to_string(),
        ));
    }

    // Create rule
    let rule = AutomationRule::new(
        request.name,
        request.rule_type,
        request.event_subscription,
        request.condition,
        request.action,
    );

    // Set priority if provided
    let mut rule = rule;
    rule.priority = priority;
    rule.description = request.description;

    state.db.create_automation_rule(&rule).await?;

    tracing::info!(
        "Automation rule '{}' ({}) created by user {}",
        rule.name,
        rule.id,
        user.user.id
    );

    Ok((
        StatusCode::CREATED,
        Json(AutomationRuleResponse::from(rule)),
    ))
}

/// List automation rules
pub async fn list_automation_rules(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(filters): Query<RuleFilters>,
) -> ApiResult<impl IntoResponse> {
    // Check permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "automation:manage permission required".to_string(),
        ));
    }

    let rules = if let Some(enabled) = filters.enabled {
        state.db.get_automation_rules(enabled).await?
    } else {
        state.db.get_automation_rules(false).await? // Get all rules
    };

    let responses: Vec<AutomationRuleResponse> = rules.into_iter().map(Into::into).collect();
    let total = responses.len();

    Ok(Json(RuleListResponse {
        rules: responses,
        total,
    }))
}

/// Get a single automation rule by ID
pub async fn get_automation_rule(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(rule_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Check permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "automation:manage permission required".to_string(),
        ));
    }

    let rule = state
        .db
        .get_automation_rule_by_id(&rule_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Automation rule not found".to_string()))?;

    Ok(Json(AutomationRuleResponse::from(rule)))
}

/// Update an automation rule
pub async fn update_automation_rule(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(rule_id): Path<String>,
    Json(request): Json<UpdateAutomationRuleRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "automation:manage permission required".to_string(),
        ));
    }

    // Get existing rule
    let mut rule = state
        .db
        .get_automation_rule_by_id(&rule_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Automation rule not found".to_string()))?;

    // Update fields
    if let Some(name) = request.name {
        if name.is_empty() || name.len() > 200 {
            return Err(ApiError::BadRequest(
                "Rule name must be between 1 and 200 characters".to_string(),
            ));
        }
        rule.name = name;
    }

    if let Some(description) = request.description {
        rule.description = Some(description);
    }

    if let Some(event_subscription) = request.event_subscription {
        if event_subscription.is_empty() {
            return Err(ApiError::BadRequest(
                "At least one event subscription is required".to_string(),
            ));
        }
        rule.event_subscription = event_subscription;
    }

    if let Some(condition) = request.condition {
        rule.condition = condition;
    }

    if let Some(action) = request.action {
        rule.action = action;
    }

    if let Some(priority) = request.priority {
        if !(1..=1000).contains(&priority) {
            return Err(ApiError::BadRequest(
                "Priority must be between 1 and 1000".to_string(),
            ));
        }
        rule.priority = priority;
    }

    // Update timestamp
    rule.updated_at = chrono::Utc::now().to_rfc3339();

    state.db.update_automation_rule(&rule).await?;

    tracing::info!(
        "Automation rule '{}' ({}) updated by user {}",
        rule.name,
        rule.id,
        user.user.id
    );

    Ok(Json(AutomationRuleResponse::from(rule)))
}

/// Delete an automation rule
pub async fn delete_automation_rule(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(rule_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Check permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "automation:manage permission required".to_string(),
        ));
    }

    // Verify rule exists
    let rule = state
        .db
        .get_automation_rule_by_id(&rule_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Automation rule not found".to_string()))?;

    state.db.delete_automation_rule(&rule_id).await?;

    tracing::info!(
        "Automation rule '{}' ({}) deleted by user {}",
        rule.name,
        rule.id,
        user.user.id
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Enable an automation rule
pub async fn enable_automation_rule(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(rule_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Check permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "automation:manage permission required".to_string(),
        ));
    }

    // Verify rule exists
    let rule = state
        .db
        .get_automation_rule_by_id(&rule_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Automation rule not found".to_string()))?;

    state.db.enable_automation_rule(&rule_id).await?;

    tracing::info!(
        "Automation rule '{}' ({}) enabled by user {}",
        rule.name,
        rule.id,
        user.user.id
    );

    // Fetch updated rule
    let updated_rule = state.db.get_automation_rule_by_id(&rule_id).await?.unwrap();

    Ok(Json(AutomationRuleResponse::from(updated_rule)))
}

/// Disable an automation rule
pub async fn disable_automation_rule(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(rule_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Check permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "automation:manage permission required".to_string(),
        ));
    }

    // Verify rule exists
    let rule = state
        .db
        .get_automation_rule_by_id(&rule_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Automation rule not found".to_string()))?;

    state.db.disable_automation_rule(&rule_id).await?;

    tracing::info!(
        "Automation rule '{}' ({}) disabled by user {}",
        rule.name,
        rule.id,
        user.user.id
    );

    // Fetch updated rule
    let updated_rule = state.db.get_automation_rule_by_id(&rule_id).await?.unwrap();

    Ok(Json(AutomationRuleResponse::from(updated_rule)))
}

/// List evaluation logs
pub async fn list_evaluation_logs(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(filters): Query<LogFilters>,
) -> ApiResult<impl IntoResponse> {
    // Check permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "automation:manage permission required".to_string(),
        ));
    }

    let logs = state
        .db
        .get_rule_evaluation_logs(
            filters.rule_id.as_deref(),
            filters.conversation_id.as_deref(),
            filters.event_type.as_deref(),
            filters.limit,
            filters.offset,
        )
        .await?;

    let responses: Vec<EvaluationLogResponse> = logs.into_iter().map(Into::into).collect();
    let total = responses.len();

    Ok(Json(LogListResponse {
        logs: responses,
        total,
    }))
}
