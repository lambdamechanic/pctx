use crate::model::{ErrorCode, ErrorData};
use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use uuid::Uuid;

pub static CODE_MODE_SESSION_HEADER: &str = "x-code-mode-session";

/// Extractor for the x-code-mode-session header
///
/// This extractor will parse the `x-code-mode-session` header value as a UUID.
/// If the header is missing or invalid, it will return a 400 Bad Request error.
pub struct CodeModeSession(pub Uuid);

impl<S> FromRequestParts<S> for CodeModeSession
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorData>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get the header value
        let header_key = "x-code-mode-session";
        let header_value = parts.headers.get(header_key).ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorData {
                    code: ErrorCode::InvalidSession,
                    message: format!("Missing {header_key} header"),
                    details: None,
                }),
            )
        })?;

        // Convert header value to string
        let session_str = header_value.to_str().map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorData {
                    code: ErrorCode::InvalidSession,
                    message: format!("Invalid {header_key} header value"),
                    details: None,
                }),
            )
        })?;

        // Parse as UUID
        let session_id = Uuid::parse_str(session_str).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorData {
                    code: ErrorCode::InvalidSession,
                    message: format!("Invalid {header_key} header value"),
                    details: None,
                }),
            )
        })?;

        Ok(CodeModeSession(session_id))
    }
}
