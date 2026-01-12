use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use crate::{
    api::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser},
    models::*,
    services::*,
};

pub async fn create_contact(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(request): Json<CreateContactRequest>,
) -> ApiResult<(StatusCode, Json<ContactResponse>)> {
    let response = crate::services::contact_service::create_contact(&state.db, &auth_user, request).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_contact(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<Json<ContactResponse>> {
    let response = crate::services::contact_service::get_contact(&state.db, &id).await?;
    Ok(Json(response))
}

#[derive(Deserialize)]
pub struct ContactPaginationParams {
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

pub async fn list_contacts(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Query(params): Query<ContactPaginationParams>,
) -> ApiResult<Json<ContactListResponse>> {
    let response = crate::services::contact_service::list_contacts(&state.db, params.page, params.per_page).await?;
    Ok(Json(response))
}

pub async fn update_contact(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdateContactRequest>,
) -> ApiResult<Json<ContactResponse>> {
    let response = crate::services::contact_service::update_contact(&state.db, &auth_user, &id, request).await?;
    Ok(Json(response))
}

pub async fn delete_contact(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    crate::services::contact_service::delete(&state.db, &auth_user, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
