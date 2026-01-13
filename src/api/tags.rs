use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::{
    api::middleware::{ApiResult, AppState, AuthenticatedUser},
    models::*,
    services::TagService,
};

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

/// POST /api/tags - Create a new tag
pub async fn create_tag(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Json(req): Json<CreateTagRequest>,
) -> ApiResult<Json<TagResponse>> {
    let tag_service = TagService::new(state.db.clone());

    // Get user permissions
    let permissions = tag_service.get_user_permissions(&user.user.id).await?;

    // Create tag
    let tag = tag_service.create_tag(req, &permissions).await?;

    Ok(Json(TagResponse::from(tag)))
}

/// GET /api/tags - List all tags
pub async fn list_tags(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Query(params): Query<PaginationQuery>,
) -> ApiResult<Json<TagListResponse>> {
    let tag_service = TagService::new(state.db.clone());

    // Get user permissions
    let permissions = tag_service.get_user_permissions(&user.user.id).await?;

    // List tags
    let offset = (params.page - 1) * params.per_page;
    let (tags, total) = tag_service
        .list_tags(params.per_page, offset, &permissions)
        .await?;

    let total_pages = (total + params.per_page - 1) / params.per_page;

    let tag_responses: Vec<TagResponse> = tags.into_iter().map(TagResponse::from).collect();

    Ok(Json(TagListResponse {
        tags: tag_responses,
        pagination: PaginationMetadata {
            page: params.page,
            per_page: params.per_page,
            total_count: total,
            total_pages,
        },
    }))
}

/// GET /api/tags/:id - Get tag by ID
pub async fn get_tag(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Path(tag_id): Path<String>,
) -> ApiResult<Json<TagResponse>> {
    let tag_service = TagService::new(state.db.clone());

    // Get user permissions
    let permissions = tag_service.get_user_permissions(&user.user.id).await?;

    // Get tag
    let tag = tag_service.get_tag(&tag_id, &permissions).await?;

    Ok(Json(TagResponse::from(tag)))
}

/// PATCH /api/tags/:id - Update tag
pub async fn update_tag(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Path(tag_id): Path<String>,
    Json(req): Json<UpdateTagRequest>,
) -> ApiResult<Json<TagResponse>> {
    let tag_service = TagService::new(state.db.clone());

    // Get user permissions
    let permissions = tag_service.get_user_permissions(&user.user.id).await?;

    // Update tag
    let tag = tag_service.update_tag(&tag_id, req, &permissions).await?;

    Ok(Json(TagResponse::from(tag)))
}

/// DELETE /api/tags/:id - Delete tag
pub async fn delete_tag(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Path(tag_id): Path<String>,
) -> ApiResult<StatusCode> {
    let tag_service = TagService::new(state.db.clone());

    // Get user permissions
    let permissions = tag_service.get_user_permissions(&user.user.id).await?;

    // Delete tag
    tag_service.delete_tag(&tag_id, &permissions).await?;

    Ok(StatusCode::NO_CONTENT)
}
