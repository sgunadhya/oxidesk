use crate::{
    api::middleware::{ApiError, AuthenticatedUser},
    database::Database,
    models::*,
};
use std::fmt;

#[derive(Debug)]
pub enum ContactError {
    NotFound,
    Forbidden,
    Database(ApiError),
}

impl fmt::Display for ContactError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "Contact not found"),
            Self::Forbidden => write!(f, "Permission denied"),
            Self::Database(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl From<ApiError> for ContactError {
    fn from(e: ApiError) -> Self {
        Self::Database(e)
    }
}

/// Delete a contact with business logic validation
pub async fn delete(
    db: &Database,
    auth_user: &AuthenticatedUser,
    contact_id: &str,
) -> Result<(), ContactError> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ContactError::Forbidden);
    }

    // Check if user exists and is a contact
    let user = db.get_user_by_id(contact_id).await?
        .ok_or(ContactError::NotFound)?;

    if !matches!(user.user_type, UserType::Contact) {
        return Err(ContactError::NotFound);
    }

    // Delete user (cascade will delete contact and channels)
    db.delete_contact(contact_id).await?;

    Ok(())
}
