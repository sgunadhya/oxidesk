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
    // Check permission (admin only for manual contact creation)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'contacts:create' permission".to_string(),
        ));
    }

    // Validate email
    let email = validate_and_normalize_email(&request.email)?;

    // Check if email already exists for contacts (per-type uniqueness)
    if let Some(_) = state
        .db
        .get_user_by_email_and_type(&email, &UserType::Contact)
        .await?
    {
        return Err(ApiError::Conflict("Contact email already exists".to_string()));
    }

    // Create user
    let user = User::new(email, UserType::Contact);
    state.db.create_user(&user).await?;

    // Create contact
    let contact = Contact::new(user.id.clone(), request.first_name.clone());
    state.db.create_contact(&contact).await?;

    // Create contact channel if inbox_id is provided
    if !request.inbox_id.is_empty() {
        let channel = ContactChannel::new(
            contact.id.clone(),
            request.inbox_id.clone(),
            user.email.clone(),
        );
        state.db.create_contact_channel(&channel).await?;
    }

    // Get channels for response
    let channels = state.db.get_contact_channels(&contact.id).await?;

    let response = ContactResponse {
        id: user.id.clone(),
        email: user.email.clone(),
        user_type: user.user_type.clone(),
        first_name: contact.first_name.clone(),
        channels,
        created_at: user.created_at.clone(),
        updated_at: user.updated_at.clone(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_contact(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<Json<ContactResponse>> {
    // Get user
    let user = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Contact not found".to_string()))?;

    // Verify it's a contact
    if !matches!(user.user_type, UserType::Contact) {
        return Err(ApiError::NotFound("Contact not found".to_string()));
    }

    // Get contact
    let contact = state
        .db
        .get_contact_by_user_id(&user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Contact not found".to_string()))?;

    // Get channels
    let channels = state.db.get_contact_channels(&contact.id).await?;

    let response = ContactResponse {
        id: user.id.clone(),
        email: user.email.clone(),
        user_type: user.user_type.clone(),
        first_name: contact.first_name.clone(),
        channels,
        created_at: user.created_at.clone(),
        updated_at: user.updated_at.clone(),
    };

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
    // Validate pagination parameters
    let page = if params.page < 1 { 1 } else { params.page };
    let per_page = if params.per_page < 1 {
        20
    } else if params.per_page > 100 {
        100
    } else {
        params.per_page
    };

    let offset = (page - 1) * per_page;

    // Get contacts with pagination
    let contacts_data = state.db.list_contacts(per_page, offset).await?;

    // Get total count for pagination metadata
    let total_count = state.db.count_contacts().await?;
    let total_pages = (total_count + per_page - 1) / per_page;

    // Build contact responses with channels
    let mut contact_responses = Vec::new();
    for (user, contact) in contacts_data {
        let channels = state.db.get_contact_channels(&contact.id).await?;

        contact_responses.push(ContactResponse {
            id: user.id.clone(),
            email: user.email.clone(),
            user_type: user.user_type.clone(),
            first_name: contact.first_name.clone(),
            channels,
            created_at: user.created_at.clone(),
            updated_at: user.updated_at.clone(),
        });
    }

    Ok(Json(ContactListResponse {
        contacts: contact_responses,
        pagination: PaginationMetadata {
            page,
            per_page,
            total_count,
            total_pages,
        },
    }))
}

pub async fn update_contact(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdateContactRequest>,
) -> ApiResult<Json<ContactResponse>> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'contacts:update' permission".to_string(),
        ));
    }

    // Check if user exists and is a contact
    let user = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Contact not found".to_string()))?;

    if !matches!(user.user_type, UserType::Contact) {
        return Err(ApiError::NotFound("Contact not found".to_string()));
    }

    // Get contact
    let contact = state
        .db
        .get_contact_by_user_id(&user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Contact not found".to_string()))?;

    // Update contact first_name
    state.db.update_contact(&contact.id, &request.first_name).await?;

    // Get channels for response
    let channels = state.db.get_contact_channels(&contact.id).await?;

    let response = ContactResponse {
        id: user.id.clone(),
        email: user.email.clone(),
        user_type: user.user_type.clone(),
        first_name: request.first_name.clone(),
        channels,
        created_at: user.created_at.clone(),
        updated_at: user.updated_at.clone(),
    };

    Ok(Json(response))
}

pub async fn delete_contact(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'contacts:delete' permission".to_string(),
        ));
    }

    // Check if user exists and is a contact
    let user = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Contact not found".to_string()))?;

    if !matches!(user.user_type, UserType::Contact) {
        return Err(ApiError::NotFound("Contact not found".to_string()));
    }

    // Delete contact (cascade will delete contact_channels)
    state.db.delete_contact(&user.id).await?;

    Ok(StatusCode::NO_CONTENT)
}
