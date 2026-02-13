/// HTTP Runtime Abstraction
///
/// This module provides framework-agnostic HTTP traits that allow the same
/// business logic to run on both Axum (traditional servers) and worker-rs
/// (Cloudflare Workers) without code duplication.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────┐
/// │ Controllers (Generic over HttpRuntime)  │
/// └──────────────────┬──────────────────────┘
///                    │ Uses
///                    ↓
/// ┌─────────────────────────────────────────┐
/// │ HTTP Abstraction Traits                 │
/// │  - HttpRequest                          │
/// │  - HttpResponse                         │
/// │  - HttpRuntime                          │
/// └──────────────────┬──────────────────────┘
///                    │ Implemented by
///         ┌──────────┴──────────┐
///         ↓                     ↓
///   AxumAdapter         WorkerAdapter
/// ```
///
/// # Example
///
/// ```rust,ignore
/// // Write controller once
/// pub struct ConversationController<R: HttpRuntime> {
///     service: Arc<ConversationService>,
///     _phantom: PhantomData<R>,
/// }
///
/// impl<R: HttpRuntime> ConversationController<R> {
///     pub async fn list(&self, req: R::Request) -> R::Response {
///         let user_id = req.header("x-user-id").unwrap();
///         let conversations = self.service.list(&user_id).await.unwrap();
///         R::Response::json(&conversations).unwrap()
///     }
/// }
///
/// // Use with Axum
/// let controller = ConversationController::<AxumRuntime>::new(service);
///
/// // OR use with Workers
/// let controller = ConversationController::<WorkerRuntime>::new(service);
/// ```

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

/// HTTP request abstraction
///
/// Abstracts over framework-specific request types (Axum's Request, worker-rs's Request)
/// to provide a unified interface for extracting request data.
#[async_trait]
pub trait HttpRequest: Send + Sync {
    /// Get the HTTP method (GET, POST, PUT, DELETE, etc.)
    fn method(&self) -> &str;

    /// Get the request path (e.g., "/api/conversations")
    fn path(&self) -> &str;

    /// Get all query parameters as a HashMap
    ///
    /// Example: `?status=open&priority=high` becomes
    /// `{"status": "open", "priority": "high"}`
    fn query_params(&self) -> HashMap<String, String>;

    /// Get a single header value by name (case-insensitive)
    ///
    /// Returns None if the header doesn't exist.
    fn header(&self, name: &str) -> Option<String>;

    /// Get all headers as a HashMap
    fn headers(&self) -> HashMap<String, String>;

    /// Parse the request body as JSON
    ///
    /// Consumes the body, so can only be called once per request.
    async fn json<T: DeserializeOwned>(&mut self) -> Result<T, HttpError>;

    /// Get the raw request body as bytes
    ///
    /// Consumes the body, so can only be called once per request.
    async fn bytes(&mut self) -> Result<Vec<u8>, HttpError>;

    /// Get a path parameter by name
    ///
    /// For a route like `/users/:id`, calling `path_param("id")` returns the ID.
    fn path_param(&self, name: &str) -> Option<String>;

    /// Get an extension value stored by middleware
    ///
    /// This allows middleware to pass data to handlers (e.g., authenticated user).
    /// Note: Not supported by all runtimes (worker-rs doesn't have extensions).
    fn extension<T: Send + Sync + 'static>(&self) -> Option<&T>;
}

/// HTTP response builder abstraction
///
/// Abstracts over framework-specific response types to provide a unified
/// interface for building HTTP responses.
pub trait HttpResponse: Send + Sync + Sized {
    /// Create a successful response (200 OK) with raw bytes
    fn ok(body: Vec<u8>) -> Self;

    /// Create a JSON response (200 OK)
    ///
    /// Automatically sets Content-Type: application/json
    fn json<T: Serialize>(value: &T) -> Result<Self, HttpError>;

    /// Create an error response with status code and message
    ///
    /// Returns JSON: `{"error": "message"}`
    fn error(status: u16, message: &str) -> Self;

    /// Add a header to the response (builder pattern)
    fn header(self, name: &str, value: &str) -> Self;

    /// Set the response status code (builder pattern)
    fn status(self, code: u16) -> Self;
}

/// HTTP runtime abstraction
///
/// Abstracts over the HTTP runtime (Axum server vs Cloudflare Workers)
/// to allow the same application logic to run on different platforms.
#[async_trait]
pub trait HttpRuntime: Send + Sync + 'static {
    /// The request type for this runtime
    type Request: HttpRequest;

    /// The response type for this runtime
    type Response: HttpResponse;

    /// Start an HTTP server (for traditional server runtimes like Axum)
    ///
    /// For worker-rs, this will return an error since Workers are invoked
    /// by Cloudflare's infrastructure, not by starting a server.
    async fn serve(addr: &str) -> Result<(), HttpError>;

    /// Handle a single request (for serverless runtimes like Workers)
    ///
    /// For Axum, this is not typically used since `serve` handles routing.
    async fn handle_request(request: Self::Request) -> Self::Response;
}

/// HTTP error type
///
/// Represents errors that can occur during HTTP request/response processing.
#[derive(Debug)]
pub enum HttpError {
    /// Failed to parse JSON body
    JsonParse(String),

    /// Failed to read request body
    BodyRead(String),

    /// Invalid request format
    InvalidRequest(String),

    /// Failed to serialize response
    ResponseSerialize(String),

    /// Runtime-specific error
    Runtime(String),

    /// Not implemented for this runtime
    NotImplemented(String),
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpError::JsonParse(msg) => write!(f, "JSON parse error: {}", msg),
            HttpError::BodyRead(msg) => write!(f, "Body read error: {}", msg),
            HttpError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            HttpError::ResponseSerialize(msg) => write!(f, "Response serialize error: {}", msg),
            HttpError::Runtime(msg) => write!(f, "Runtime error: {}", msg),
            HttpError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
        }
    }
}

impl std::error::Error for HttpError {}

/// Helper trait for converting framework-specific errors to HttpError
pub trait IntoHttpError {
    fn into_http_error(self) -> HttpError;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_error_display() {
        let err = HttpError::JsonParse("invalid JSON".to_string());
        assert_eq!(err.to_string(), "JSON parse error: invalid JSON");

        let err = HttpError::BodyRead("connection closed".to_string());
        assert_eq!(err.to_string(), "Body read error: connection closed");
    }
}
