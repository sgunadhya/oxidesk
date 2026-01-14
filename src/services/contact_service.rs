use crate::api::middleware::{ApiError, ApiResult, AuthenticatedUser};
use crate::database::Database;
use crate::models::*;
use crate::services::validate_and_normalize_email;
use time;

/// Create a new contact
pub async fn create_contact(
    db: &Database,
    auth_user: &AuthenticatedUser,
    request: CreateContactRequest,
) -> ApiResult<ContactResponse> {
    // Check permission (admin only for manual contact creation)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'contacts:create' permission".to_string(),
        ));
    }

    // Validate email
    let email = validate_and_normalize_email(&request.email)?;

    // Check if email already exists for contacts (per-type uniqueness)
    if let Some(_) = db
        .get_user_by_email_and_type(&email, &UserType::Contact)
        .await?
    {
        return Err(ApiError::Conflict(
            "Contact email already exists".to_string(),
        ));
    }

    // Create user
    let user = User::new(email, UserType::Contact);
    db.create_user(&user).await?;

    // Create contact
    let contact = Contact::new(user.id.clone(), request.first_name.clone());
    db.create_contact(&contact).await?;

    // Create contact channel if inbox_id is provided
    if !request.inbox_id.is_empty() {
        let channel = ContactChannel::new(
            contact.id.clone(),
            request.inbox_id.clone(),
            user.email.clone(),
        );
        db.create_contact_channel(&channel).await?;
    }

    // Get channels for response
    let channels = db.get_contact_channels(&contact.id).await?;

    Ok(ContactResponse {
        id: user.id.clone(),
        email: user.email.clone(),
        user_type: user.user_type.clone(),
        first_name: contact.first_name.clone(),
        channels,
        created_at: user.created_at.clone(),
        updated_at: user.updated_at.clone(),
    })
}

/// Get a contact by ID
pub async fn get_contact(db: &Database, id: &str) -> ApiResult<ContactResponse> {
    // Get user
    let user = db
        .get_user_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Contact not found".to_string()))?;

    // Verify it's a contact
    if !matches!(user.user_type, UserType::Contact) {
        return Err(ApiError::NotFound("Contact not found".to_string()));
    }

    // Get contact
    let contact = db
        .get_contact_by_user_id(&user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Contact not found".to_string()))?;

    // Get channels
    let channels = db.get_contact_channels(&contact.id).await?;

    Ok(ContactResponse {
        id: user.id.clone(),
        email: user.email.clone(),
        user_type: user.user_type.clone(),
        first_name: contact.first_name.clone(),
        channels,
        created_at: user.created_at.clone(),
        updated_at: user.updated_at.clone(),
    })
}

/// Delete a contact
pub async fn delete(db: &Database, auth_user: &AuthenticatedUser, id: &str) -> ApiResult<()> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'contacts:delete' permission".to_string(),
        ));
    }

    // Check if user exists and is a contact
    let user = db
        .get_user_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Contact not found".to_string()))?;

    if !matches!(user.user_type, UserType::Contact) {
        return Err(ApiError::NotFound("Contact not found".to_string()));
    }

    // Delete contact (cascade will delete contact_channels)
    db.delete_contact(&user.id).await?;

    Ok(())
}

/// List contacts with pagination
pub async fn list_contacts(
    db: &Database,
    page: i64,
    per_page: i64,
) -> ApiResult<ContactListResponse> {
    // Validate pagination parameters
    let page = if page < 1 { 1 } else { page };
    let per_page = if per_page < 1 {
        20
    } else if per_page > 100 {
        100
    } else {
        per_page
    };

    let offset = (page - 1) * per_page;

    // Get contacts with pagination
    let contacts_data = db.list_contacts(per_page, offset).await?;

    // Get total count for pagination metadata
    let total_count = db.count_contacts().await?;
    let total_pages = (total_count + per_page - 1) / per_page;

    // Build contact responses with channels
    let mut contact_responses = Vec::new();
    for (user, contact) in contacts_data {
        let channels = db.get_contact_channels(&contact.id).await?;

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

    Ok(ContactListResponse {
        contacts: contact_responses,
        pagination: PaginationMetadata {
            page,
            per_page,
            total_count,
            total_pages,
        },
    })
}

/// Update a contact
pub async fn update_contact(
    db: &Database,
    auth_user: &AuthenticatedUser,
    id: &str,
    request: UpdateContactRequest,
) -> ApiResult<ContactResponse> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'contacts:update' permission".to_string(),
        ));
    }

    // Check if user exists and is a contact
    let user = db
        .get_user_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Contact not found".to_string()))?;

    if !matches!(user.user_type, UserType::Contact) {
        return Err(ApiError::NotFound("Contact not found".to_string()));
    }

    // Get contact
    let contact = db
        .get_contact_by_user_id(&user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Contact not found".to_string()))?;

    // Update contact first_name
    db.update_contact(&contact.id, &request.first_name).await?;

    // Get channels for response
    let channels = db.get_contact_channels(&contact.id).await?;

    Ok(ContactResponse {
        id: user.id.clone(),
        email: user.email.clone(),
        user_type: user.user_type.clone(),
        first_name: request.first_name.clone(),
        channels,
        created_at: user.created_at.clone(),
        updated_at: user.updated_at.clone(),
    })
}

/// Update contact details (email and name)
pub async fn update_contact_details(
    db: &Database,
    _auth_user: &AuthenticatedUser,
    id: &str,
    full_name: &str,
    email: &str,
) -> ApiResult<()> {
    // Validate contact exists
    let contact = db
        .get_contact_by_user_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Contact not found".to_string()))?;

    // Update User Email
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();

    db.update_user_email(id, email, &now).await?;

    // Update Contact Name
    db.update_contact_name(&contact.id, full_name).await?;

    Ok(())
}

/// Resolve internal Contact ID from User ID
/// Used when frontend provides User ID (which acts as public Contact ID) but backend needs internal Contact ID for FKs
pub async fn resolve_contact_id_from_user_id(db: &Database, user_id: &str) -> ApiResult<String> {
    let id = user_id.trim();
    tracing::info!("Resolving contact for user_id: '{}'", id);

    let contact = db.get_contact_by_user_id(id).await?.ok_or_else(|| {
        tracing::warn!("Contact resolution failed for user_id: '{}'", id);
        ApiError::NotFound("Contact not found".to_string())
    })?;

    Ok(contact.id)
}
