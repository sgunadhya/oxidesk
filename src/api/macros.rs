use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::middleware::{error::ApiError, ApiResult, AppState, AuthenticatedUser},
    models::*,
    services::MacroService,
};

// ===== Request DTOs =====

#[derive(Debug, Deserialize)]
pub struct ApplyMacroRequest {
    pub conversation_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMacroRequest {
    pub name: String,
    pub message_content: String,
    #[serde(default)]
    pub actions: Vec<MacroActionRequest>,
    #[serde(default = "default_access_control")]
    pub access_control: String,
}

fn default_access_control() -> String {
    "all".to_string()
}

#[derive(Debug, Deserialize)]
pub struct MacroActionRequest {
    pub action_type: String,
    pub action_value: String,
    #[serde(default)]
    pub action_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMacroRequest {
    pub message_content: Option<String>,
    pub actions: Option<Vec<MacroActionRequest>>,
    pub access_control: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GrantAccessRequest {
    pub entity_type: String,
    pub entity_id: String,
}

// ===== Response DTOs =====

#[derive(Debug, Serialize)]
pub struct ApplyMacroResponse {
    pub message_content: String,
    pub actions_to_queue: Vec<MacroActionResponse>,
    pub variables_replaced: i32,
}

#[derive(Debug, Serialize)]
pub struct MacroResponse {
    pub id: String,
    pub name: String,
    pub message_content: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub usage_count: i32,
    pub access_control: String,
    pub actions: Vec<MacroActionResponse>,
}

#[derive(Debug, Serialize)]
pub struct MacroActionResponse {
    pub action_type: String,
    pub action_value: String,
    pub action_order: i32,
}

#[derive(Debug, Serialize)]
pub struct MacroAccessResponse {
    pub id: String,
    pub macro_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub granted_at: String,
    pub granted_by: String,
}

#[derive(Debug, Serialize)]
pub struct MacroApplicationLogResponse {
    pub id: String,
    pub macro_id: String,
    pub agent_id: String,
    pub conversation_id: String,
    pub applied_at: String,
    pub actions_queued: Vec<String>,
    pub variables_replaced: i32,
}

#[derive(Debug, Serialize)]
pub struct MacroListResponse {
    pub macros: Vec<MacroResponse>,
    pub total: usize,
}

// ===== API Handlers =====

/// Apply macro to conversation
/// POST /api/macros/:id/apply
pub async fn apply_macro(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(macro_id): Path<String>,
    Json(req): Json<ApplyMacroRequest>,
) -> ApiResult<impl IntoResponse> {
    // Apply macro
    let result =
        MacroService::apply_macro(&state.db, &macro_id, &req.conversation_id, &user.user.id)
            .await?;

    // Convert to response
    let response = ApplyMacroResponse {
        message_content: result.message_content,
        actions_to_queue: result
            .actions_to_queue
            .into_iter()
            .map(|a| MacroActionResponse {
                action_type: a.action_type,
                action_value: a.action_value,
                action_order: a.action_order,
            })
            .collect(),
        variables_replaced: result.variables_replaced,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Create macro
/// POST /api/macros
pub async fn create_macro(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<CreateMacroRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check admin permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "Administrator permission required".to_string(),
        ));
    }

    // Convert actions
    let actions: Vec<(String, String, i32)> = req
        .actions
        .into_iter()
        .map(|a| (a.action_type, a.action_value, a.action_order))
        .collect();

    // Create macro
    let macro_obj = MacroService::create_macro(
        &state.db,
        req.name,
        req.message_content,
        actions,
        &user.user.id,
        req.access_control,
    )
    .await?;

    // Convert to response
    let response = macro_to_response(macro_obj);

    Ok((StatusCode::CREATED, Json(response)))
}

/// List accessible macros
/// GET /api/macros
pub async fn list_macros(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> ApiResult<impl IntoResponse> {
    // Get accessible macros
    let macros = MacroService::list_accessible_macros(&state.db, &user.user.id).await?;

    // Convert to response
    let total = macros.len();
    let macros_response: Vec<MacroResponse> = macros.into_iter().map(macro_to_response).collect();

    let response = MacroListResponse {
        macros: macros_response,
        total,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Get macro by ID
/// GET /api/macros/:id
pub async fn get_macro(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(macro_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Get macro
    let macro_obj = MacroService::get_macro(&state.db, &macro_id).await?;

    // Check access
    if !MacroService::check_macro_access(&state.db, &macro_obj, &user.user.id).await? {
        return Err(ApiError::Forbidden(
            "You do not have access to this macro".to_string(),
        ));
    }

    // Convert to response
    let response = macro_to_response(macro_obj);

    Ok((StatusCode::OK, Json(response)))
}

/// Update macro
/// PUT /api/macros/:id
pub async fn update_macro(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(macro_id): Path<String>,
    Json(req): Json<UpdateMacroRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check admin permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "Administrator permission required".to_string(),
        ));
    }

    // Convert actions if provided
    let actions = req.actions.map(|acts| {
        acts.into_iter()
            .map(|a| (a.action_type, a.action_value, a.action_order))
            .collect()
    });

    // Update macro
    let macro_obj = MacroService::update_macro(
        &state.db,
        &macro_id,
        req.message_content,
        actions,
        req.access_control,
    )
    .await?;

    // Convert to response
    let response = macro_to_response(macro_obj);

    Ok((StatusCode::OK, Json(response)))
}

/// Delete macro
/// DELETE /api/macros/:id
pub async fn delete_macro(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(macro_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Check admin permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "Administrator permission required".to_string(),
        ));
    }

    // Delete macro
    MacroService::delete_macro(&state.db, &macro_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Grant access to macro
/// POST /api/macros/:id/access
pub async fn grant_macro_access(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(macro_id): Path<String>,
    Json(req): Json<GrantAccessRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check admin permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "Administrator permission required".to_string(),
        ));
    }

    // Grant access
    MacroService::grant_access(
        &state.db,
        &macro_id,
        &req.entity_type,
        &req.entity_id,
        &user.user.id,
    )
    .await?;

    Ok(StatusCode::CREATED)
}

/// List macro access grants
/// GET /api/macros/:id/access
pub async fn list_macro_access(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(macro_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Check admin permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "Administrator permission required".to_string(),
        ));
    }

    // Get access grants
    let accesses = state.db.get_macro_access(&macro_id).await?;

    // Convert to response
    let response: Vec<MacroAccessResponse> = accesses
        .into_iter()
        .map(|a| MacroAccessResponse {
            id: a.id,
            macro_id: a.macro_id,
            entity_type: a.entity_type,
            entity_id: a.entity_id,
            granted_at: a.granted_at,
            granted_by: a.granted_by,
        })
        .collect();

    Ok((StatusCode::OK, Json(response)))
}

/// Revoke macro access
/// DELETE /api/macros/:id/access/:entity_type/:entity_id
pub async fn revoke_macro_access(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((macro_id, entity_type, entity_id)): Path<(String, String, String)>,
) -> ApiResult<impl IntoResponse> {
    // Check admin permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "Administrator permission required".to_string(),
        ));
    }

    // Revoke access
    MacroService::revoke_access(&state.db, &macro_id, &entity_type, &entity_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get macro application logs
/// GET /api/macros/:id/logs
pub async fn get_macro_logs(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(macro_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Check admin permission
    if !user.has_permission("automation:manage").await {
        return Err(ApiError::Forbidden(
            "Administrator permission required".to_string(),
        ));
    }

    // Get logs (with default pagination)
    let logs = state
        .db
        .get_macro_application_logs(&macro_id, 50, 0)
        .await?;

    // Convert to response
    let response: Vec<MacroApplicationLogResponse> = logs
        .into_iter()
        .map(|l| MacroApplicationLogResponse {
            id: l.id,
            macro_id: l.macro_id,
            agent_id: l.agent_id,
            conversation_id: l.conversation_id,
            applied_at: l.applied_at,
            actions_queued: serde_json::from_str(&l.actions_queued).unwrap_or_default(),
            variables_replaced: l.variables_replaced,
        })
        .collect();

    Ok((StatusCode::OK, Json(response)))
}

// ===== Helper Functions =====

fn macro_to_response(macro_obj: Macro) -> MacroResponse {
    let actions = macro_obj.actions.unwrap_or_default();
    MacroResponse {
        id: macro_obj.id,
        name: macro_obj.name,
        message_content: macro_obj.message_content,
        created_by: macro_obj.created_by,
        created_at: macro_obj.created_at,
        updated_at: macro_obj.updated_at,
        usage_count: macro_obj.usage_count,
        access_control: macro_obj.access_control,
        actions: actions
            .into_iter()
            .map(|a| MacroActionResponse {
                action_type: a.action_type,
                action_value: a.action_value,
                action_order: a.action_order,
            })
            .collect(),
    }
}
