use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Unauthorized,
    Forbidden(String),
    Internal(String),
    Conflict(String),
    TooManyRequests(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ApiError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ApiError::Unauthorized => write!(f, "Unauthorized"),
            ApiError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            ApiError::Internal(msg) => write!(f, "Internal error: {}", msg),
            ApiError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            ApiError::TooManyRequests(msg) => write!(f, "Too many requests: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::TooManyRequests(msg) => (StatusCode::TOO_MANY_REQUESTS, msg),
        };

        let body = Json(json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

// Convert from sqlx errors
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ApiError::NotFound("Resource not found".to_string()),
            sqlx::Error::Database(db_err) => {
                // Check for unique constraint violations (Feature 016: Per-type email uniqueness)
                let message = db_err.message();
                if message.contains("UNIQUE") || message.contains("unique") {
                    // Check for specific per-type email constraint violations (migration 058)
                    if message.contains("idx_users_email_unique_agent") || message.contains("agent")
                    {
                        ApiError::Conflict("An agent with this email already exists".to_string())
                    } else if message.contains("idx_users_email_unique_contact")
                        || message.contains("contact")
                    {
                        ApiError::Conflict("A contact with this email already exists".to_string())
                    } else {
                        // Generic unique constraint violation
                        ApiError::Conflict("Email already exists".to_string())
                    }
                } else {
                    ApiError::Internal(format!("Database error: {}", message))
                }
            }
            _ => ApiError::Internal("Internal server error".to_string()),
        }
    }
}

// Convert from argon2 errors
impl From<argon2::password_hash::Error> for ApiError {
    fn from(_: argon2::password_hash::Error) -> Self {
        ApiError::Internal("Password hashing error".to_string())
    }
}

// Convert from domain errors
impl From<crate::domain::errors::DomainError> for ApiError {
    fn from(err: crate::domain::errors::DomainError) -> Self {
        match err {
            crate::domain::errors::DomainError::NotFound(msg) => ApiError::NotFound(msg),
            crate::domain::errors::DomainError::ValidationError(msg) => ApiError::BadRequest(msg),
            crate::domain::errors::DomainError::Conflict(msg) => ApiError::Conflict(msg),
            crate::domain::errors::DomainError::Forbidden(msg) => ApiError::Forbidden(msg),
            crate::domain::errors::DomainError::Internal(msg) => ApiError::Internal(msg),
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
