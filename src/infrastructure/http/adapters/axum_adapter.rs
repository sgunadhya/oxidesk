/// Axum HTTP Runtime Adapter
///
/// This adapter implements the HttpRuntime traits for Axum, allowing
/// framework-agnostic controllers to run on traditional Axum servers.

use crate::domain::ports::http_runtime::*;
use async_trait::async_trait;
use axum::{
    body::Body,
    extract::Request as AxumRequest,
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri},
    response::{IntoResponse, Response as AxumResponse},
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::str::FromStr;

/// Axum request wrapper
///
/// Wraps Axum's Request type to implement the HttpRequest trait.
pub struct AxumRequestWrapper {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
    path_params: HashMap<String, String>,
    extensions: axum::http::Extensions,
}

impl AxumRequestWrapper {
    /// Create from Axum request
    pub async fn from_axum(mut req: AxumRequest) -> Result<Self, HttpError> {
        use axum::body::to_bytes;

        let method = req.method().clone();
        let uri = req.uri().clone();
        let headers = req.headers().clone();
        let extensions = std::mem::take(req.extensions_mut());

        // Read body bytes
        let body_bytes = to_bytes(req.into_body(), usize::MAX)
            .await
            .map_err(|e| HttpError::BodyRead(e.to_string()))?;

        Ok(Self {
            method,
            uri,
            headers,
            body: Some(body_bytes.to_vec()),
            path_params: HashMap::new(),
            extensions,
        })
    }

    /// Set path parameters (extracted by router)
    pub fn with_path_params(mut self, params: HashMap<String, String>) -> Self {
        self.path_params = params;
        self
    }
}

#[async_trait]
impl HttpRequest for AxumRequestWrapper {
    fn method(&self) -> &str {
        self.method.as_str()
    }

    fn path(&self) -> &str {
        self.uri.path()
    }

    fn query_params(&self) -> HashMap<String, String> {
        self.uri
            .query()
            .map(|q| {
                url::form_urlencoded::parse(q.as_bytes())
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }

    fn headers(&self) -> HashMap<String, String> {
        self.headers
            .iter()
            .filter_map(|(k, v)| {
                v.to_str()
                    .ok()
                    .map(|v| (k.as_str().to_string(), v.to_string()))
            })
            .collect()
    }

    async fn json<T: DeserializeOwned>(&mut self) -> Result<T, HttpError> {
        let body = self
            .body
            .take()
            .ok_or_else(|| HttpError::BodyRead("Body already consumed".to_string()))?;

        serde_json::from_slice(&body).map_err(|e| HttpError::JsonParse(e.to_string()))
    }

    async fn bytes(&mut self) -> Result<Vec<u8>, HttpError> {
        self.body
            .take()
            .ok_or_else(|| HttpError::BodyRead("Body already consumed".to_string()))
    }

    fn path_param(&self, name: &str) -> Option<String> {
        self.path_params.get(name).cloned()
    }

    fn extension<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.extensions.get::<T>()
    }
}

/// Axum response wrapper
///
/// Wraps Axum's Response to implement the HttpResponse trait.
pub struct AxumResponseWrapper {
    status: StatusCode,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpResponse for AxumResponseWrapper {
    fn ok(body: Vec<u8>) -> Self {
        Self {
            status: StatusCode::OK,
            headers: vec![],
            body,
        }
    }

    fn json<T: Serialize>(value: &T) -> Result<Self, HttpError> {
        let body = serde_json::to_vec(value).map_err(|e| HttpError::ResponseSerialize(e.to_string()))?;

        Ok(Self {
            status: StatusCode::OK,
            headers: vec![(
                "content-type".to_string(),
                "application/json".to_string(),
            )],
            body,
        })
    }

    fn error(status: u16, message: &str) -> Self {
        let body = serde_json::json!({
            "error": message
        })
        .to_string()
        .into_bytes();

        Self {
            status: StatusCode::from_u16(status)
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            headers: vec![(
                "content-type".to_string(),
                "application/json".to_string(),
            )],
            body,
        }
    }

    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    fn status(mut self, code: u16) -> Self {
        self.status = StatusCode::from_u16(code)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        self
    }
}

impl IntoResponse for AxumResponseWrapper {
    fn into_response(self) -> AxumResponse {
        let mut response = AxumResponse::new(Body::from(self.body));
        *response.status_mut() = self.status;

        for (name, value) in self.headers {
            if let Ok(header_name) = HeaderName::from_str(&name) {
                if let Ok(header_value) = HeaderValue::from_str(&value) {
                    response.headers_mut().insert(header_name, header_value);
                }
            }
        }

        response
    }
}

/// Axum HTTP runtime
///
/// Implements HttpRuntime for Axum servers.
pub struct AxumRuntime;

#[async_trait]
impl HttpRuntime for AxumRuntime {
    type Request = AxumRequestWrapper;
    type Response = AxumResponseWrapper;

    async fn serve(addr: &str) -> Result<(), HttpError> {
        // Note: This is a simplified version. In real usage, you'd pass a router
        // The actual serve logic should be handled by the application bootstrap
        Err(HttpError::NotImplemented(
            "Use AxumRuntime in application bootstrap with proper router setup".to_string(),
        ))
    }

    async fn handle_request(request: Self::Request) -> Self::Response {
        // Axum doesn't use single-request handling - it uses a router with serve
        // This is only here for trait completeness
        Self::Response::error(501, "Axum uses serve() not handle_request()")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_axum_response_json() {
        #[derive(Serialize)]
        struct TestData {
            message: String,
        }

        let data = TestData {
            message: "Hello".to_string(),
        };

        let response = AxumResponseWrapper::json(&data).unwrap();
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(
            response.headers,
            vec![("content-type".to_string(), "application/json".to_string())]
        );
    }

    #[test]
    fn test_axum_response_error() {
        let response = AxumResponseWrapper::error(404, "Not Found");
        assert_eq!(response.status, StatusCode::NOT_FOUND);
        assert!(String::from_utf8_lossy(&response.body).contains("Not Found"));
    }

    #[test]
    fn test_axum_response_builder() {
        let response = AxumResponseWrapper::ok(b"test".to_vec())
            .status(201)
            .header("x-custom", "value");

        assert_eq!(response.status, StatusCode::CREATED);
        assert!(response
            .headers
            .contains(&("x-custom".to_string(), "value".to_string())));
    }
}
