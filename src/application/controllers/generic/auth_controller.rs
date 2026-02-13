/// Framework-Agnostic Authentication Controller
///
/// This controller is generic over HttpRuntime, allowing it to work with
/// both Axum (traditional servers) and worker-rs (Cloudflare Workers).
///
/// # Example Usage with Axum
///
/// ```rust,ignore
/// use oxidesk::application::controllers::generic::GenericAuthController;
/// use oxidesk::infrastructure::http::adapters::axum_adapter::AxumRuntime;
///
/// let controller = GenericAuthController::<AxumRuntime>::new(auth_provider);
///
/// // Use in Axum router
/// let app = axum::Router::new()
///     .route("/api/auth/login", post(|req| controller.login(req)));
/// ```
///
/// # Example Usage with Workers
///
/// ```rust,ignore
/// use oxidesk::application::controllers::generic::GenericAuthController;
/// use oxidesk::infrastructure::http::adapters::worker_adapter::WorkerRuntime;
///
/// let controller = GenericAuthController::<WorkerRuntime>::new(auth_provider);
///
/// // Use in worker-rs router
/// Router::new()
///     .post_async("/api/auth/login", |req, _| async {
///         controller.login(req).await
///     })
/// ```

use crate::domain::ports::auth_provider::{AuthProvider, Credentials};
use crate::domain::ports::http_runtime::*;
use std::marker::PhantomData;
use std::sync::Arc;

/// Framework-agnostic authentication controller
pub struct GenericAuthController<R: HttpRuntime> {
    auth_provider: Arc<dyn AuthProvider>,
    _phantom: PhantomData<R>,
}

impl<R: HttpRuntime> GenericAuthController<R> {
    /// Create a new generic auth controller
    pub fn new(auth_provider: Arc<dyn AuthProvider>) -> Self {
        Self {
            auth_provider,
            _phantom: PhantomData,
        }
    }

    /// Login endpoint
    ///
    /// POST /api/auth/login
    /// Body: { "email": "user@example.com", "password": "password123" }
    /// Response: { "token": "...", "expires_at": "...", ... }
    pub async fn login(&self, mut request: R::Request) -> R::Response {
        // Parse JSON body (framework-agnostic)
        #[derive(serde::Deserialize)]
        struct LoginRequest {
            email: String,
            password: String,
            #[serde(default)]
            duration_hours: Option<i64>,
        }

        let body: LoginRequest = match request.json().await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to parse login request: {}", e);
                return R::Response::error(400, "Invalid request body");
            }
        };

        // Authenticate using provider (framework-agnostic)
        let credentials = Credentials::Password {
            email: body.email,
            password: body.password,
        };

        match self
            .auth_provider
            .authenticate(credentials, body.duration_hours, None)
            .await
        {
            Ok(token) => {
                tracing::info!("User logged in successfully");

                // Return JSON response (framework-agnostic)
                match R::Response::json(&token) {
                    Ok(response) => response,
                    Err(e) => {
                        tracing::error!("Failed to serialize response: {}", e);
                        R::Response::error(500, "Internal server error")
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Login failed: {}", e);
                R::Response::error(401, "Invalid credentials")
            }
        }
    }

    /// Verify token endpoint
    ///
    /// GET /api/auth/verify
    /// Headers: Authorization: Bearer <token>
    /// Response: { "user_id": "...", "email": "...", "roles": [...], ... }
    pub async fn verify_token(&self, request: R::Request) -> R::Response {
        // Extract token from Authorization header (framework-agnostic)
        let auth_header = match request.header("authorization") {
            Some(h) => h,
            None => {
                tracing::warn!("Missing Authorization header");
                return R::Response::error(401, "Missing authorization header");
            }
        };

        // Parse Bearer token
        let token = if let Some(token) = auth_header.strip_prefix("Bearer ") {
            token
        } else {
            tracing::warn!("Invalid Authorization header format");
            return R::Response::error(401, "Invalid authorization header format");
        };

        // Verify token using provider (framework-agnostic)
        match self.auth_provider.verify_token(token).await {
            Ok(context) => {
                tracing::debug!("Token verified for user: {}", context.user.id);

                // Build response with user info
                #[derive(serde::Serialize)]
                struct VerifyResponse {
                    user_id: String,
                    email: String,
                    user_type: String,
                    roles: Vec<String>,
                    permissions: Vec<String>,
                }

                let response = VerifyResponse {
                    user_id: context.user.id,
                    email: context.user.email,
                    user_type: format!("{:?}", context.user.user_type),
                    roles: context.roles.iter().map(|r| r.name.clone()).collect(),
                    permissions: context.permissions,
                };

                // Return JSON response (framework-agnostic)
                match R::Response::json(&response) {
                    Ok(response) => response,
                    Err(e) => {
                        tracing::error!("Failed to serialize response: {}", e);
                        R::Response::error(500, "Internal server error")
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Token verification failed: {}", e);
                R::Response::error(401, "Invalid or expired token")
            }
        }
    }

    /// Logout endpoint
    ///
    /// POST /api/auth/logout
    /// Headers: Authorization: Bearer <token>
    /// Response: { "message": "Logged out successfully" }
    pub async fn logout(&self, request: R::Request) -> R::Response {
        // Extract token from Authorization header (framework-agnostic)
        let auth_header = match request.header("authorization") {
            Some(h) => h,
            None => {
                tracing::warn!("Missing Authorization header");
                return R::Response::error(401, "Missing authorization header");
            }
        };

        // Parse Bearer token
        let token = if let Some(token) = auth_header.strip_prefix("Bearer ") {
            token
        } else {
            tracing::warn!("Invalid Authorization header format");
            return R::Response::error(401, "Invalid authorization header format");
        };

        // Revoke token using provider (framework-agnostic)
        match self.auth_provider.revoke_token(token).await {
            Ok(_) => {
                tracing::info!("User logged out successfully");

                #[derive(serde::Serialize)]
                struct LogoutResponse {
                    message: &'static str,
                }

                let response = LogoutResponse {
                    message: "Logged out successfully",
                };

                // Return JSON response (framework-agnostic)
                match R::Response::json(&response) {
                    Ok(response) => response,
                    Err(e) => {
                        tracing::error!("Failed to serialize response: {}", e);
                        R::Response::error(500, "Internal server error")
                    }
                }
            }
            Err(e) => {
                tracing::error!("Logout failed: {}", e);
                R::Response::error(500, "Failed to logout")
            }
        }
    }

    /// Change password endpoint
    ///
    /// POST /api/auth/change-password
    /// Headers: Authorization: Bearer <token>
    /// Body: { "current_password": "...", "new_password": "..." }
    /// Response: { "message": "Password changed successfully" }
    pub async fn change_password(&self, mut request: R::Request) -> R::Response {
        // Extract token from Authorization header
        let auth_header = match request.header("authorization") {
            Some(h) => h,
            None => return R::Response::error(401, "Missing authorization header"),
        };

        let token = if let Some(token) = auth_header.strip_prefix("Bearer ") {
            token
        } else {
            return R::Response::error(401, "Invalid authorization header format");
        };

        // Verify token to get user ID
        let context = match self.auth_provider.verify_token(token).await {
            Ok(ctx) => ctx,
            Err(_) => return R::Response::error(401, "Invalid or expired token"),
        };

        // Parse request body
        #[derive(serde::Deserialize)]
        struct ChangePasswordRequest {
            current_password: String,
            new_password: String,
            #[serde(default)]
            revoke_other_sessions: bool,
        }

        let body: ChangePasswordRequest = match request.json().await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to parse change password request: {}", e);
                return R::Response::error(400, "Invalid request body");
            }
        };

        // Change password using provider
        match self
            .auth_provider
            .change_password(
                &context.user.id,
                &body.current_password,
                &body.new_password,
                body.revoke_other_sessions,
            )
            .await
        {
            Ok(_) => {
                tracing::info!("Password changed for user: {}", context.user.id);

                #[derive(serde::Serialize)]
                struct ChangePasswordResponse {
                    message: &'static str,
                }

                let response = ChangePasswordResponse {
                    message: "Password changed successfully",
                };

                match R::Response::json(&response) {
                    Ok(response) => response,
                    Err(e) => {
                        tracing::error!("Failed to serialize response: {}", e);
                        R::Response::error(500, "Internal server error")
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Password change failed for user {}: {}", context.user.id, e);
                R::Response::error(400, "Password change failed")
            }
        }
    }
}

impl<R: HttpRuntime> Clone for GenericAuthController<R> {
    fn clone(&self) -> Self {
        Self {
            auth_provider: Arc::clone(&self.auth_provider),
            _phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        // This ensures the controller can be shared across threads
        // Required for both Axum and Workers
        #[cfg(feature = "axum-runtime")]
        {
            use crate::infrastructure::http::adapters::axum_adapter::AxumRuntime;
            assert_send_sync::<GenericAuthController<AxumRuntime>>();
        }
    }
}
