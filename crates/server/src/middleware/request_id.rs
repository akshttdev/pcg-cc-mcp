use axum::{
    extract::Request,
    http::{header::HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

/// HTTP header name for request ID
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Middleware that generates and propagates request IDs
///
/// This middleware:
/// 1. Checks if the incoming request has an x-request-id header
/// 2. If not present, generates a new UUID
/// 3. Adds the request ID to the response headers
/// 4. Logs the request ID with tracing for correlation
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    // Check for existing request ID in headers
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Add request ID to tracing span for correlation
    tracing::Span::current().record("request_id", &request_id.as_str());

    // Insert request ID into request extensions so handlers can access it
    request.extensions_mut().insert(RequestId(request_id.clone()));

    // Process the request
    let mut response = next.run(request).await;

    // Add request ID to response headers
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(
            HeaderName::from_static(REQUEST_ID_HEADER),
            header_value,
        );
    }

    response
}

/// Request ID extractor that can be used in handlers
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl RequestId {
    /// Get the request ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_generates_request_id_when_missing() {
        let app = axum::Router::new()
            .route(
                "/test",
                axum::routing::get(|| async { "ok" }),
            )
            .layer(axum::middleware::from_fn(request_id_middleware));

        let request = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should have a request ID in response
        assert!(response.headers().contains_key(REQUEST_ID_HEADER));
        let request_id = response.headers().get(REQUEST_ID_HEADER).unwrap();
        // Should be a valid UUID format
        assert!(Uuid::parse_str(request_id.to_str().unwrap()).is_ok());
    }

    #[tokio::test]
    async fn test_preserves_existing_request_id() {
        let app = axum::Router::new()
            .route(
                "/test",
                axum::routing::get(|| async { "ok" }),
            )
            .layer(axum::middleware::from_fn(request_id_middleware));

        let existing_id = "custom-request-id-123";
        let request = Request::builder()
            .uri("/test")
            .header(REQUEST_ID_HEADER, existing_id)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should preserve the existing request ID
        let response_id = response.headers().get(REQUEST_ID_HEADER).unwrap();
        assert_eq!(response_id.to_str().unwrap(), existing_id);
    }

    #[test]
    fn test_request_id_as_str() {
        let request_id = RequestId("test-id".to_string());
        assert_eq!(request_id.as_str(), "test-id");
    }
}
