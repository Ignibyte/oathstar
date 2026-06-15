use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// A refusal at the auth boundary.
///
/// Renders as a real status code (401 or 403) with a small `error` body — never
/// the game server's in-band `200 {ok:false}` convention (#41 / Decision 057), so
/// an unauthorized request can never be mistaken for an accepted one. Browser
/// surfaces redirect instead of using this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// No valid session: the caller is unauthenticated. Renders 401.
    Unauthorized(&'static str),
    /// A valid session that lacks the required role. Renders 403.
    Forbidden(&'static str),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, reason) = match self {
            Self::Unauthorized(reason) => (StatusCode::UNAUTHORIZED, reason),
            Self::Forbidden(reason) => (StatusCode::FORBIDDEN, reason),
        };
        (status, Json(json!({ "error": reason }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::AuthError;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[test]
    fn auth_error_renders_real_status_codes() {
        // T6: a refusal is a real 401/403, never the in-band 200 {ok:false}.
        assert_eq!(
            AuthError::Unauthorized("x").into_response().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AuthError::Forbidden("x").into_response().status(),
            StatusCode::FORBIDDEN
        );
    }
}
