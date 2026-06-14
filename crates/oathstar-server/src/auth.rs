//! Server-side authentication, session, and role boundary (ticket #41,
//! Decision 056).
//!
//! v1 is intentionally minimal: a read-only [`SessionStore`] maps an opaque
//! bearer token to a [`Principal`], the [`AuthPrincipal`] extractor pulls and
//! validates the caller, and [`AuthError`] renders refusals as real 401/403
//! responses — never the server's in-band `200 {ok:false}` convention, which
//! must not apply to an unauthorized request. Real accounts, password storage,
//! and persistence are out of scope.

use std::collections::HashMap;

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use oathstar_protocol::{AuthRole, Principal};
use serde_json::json;

use crate::AppState;

/// A read-only registry mapping an opaque bearer token to its [`Principal`].
///
/// v1 holds only what the deterministic local-dev owner seeds (see
/// [`SessionStore::from_env`]); production starts empty, so every protected
/// request is refused until a real session source exists.
#[derive(Debug, Default, Clone)]
pub struct SessionStore {
    tokens: HashMap<String, Principal>,
}

impl SessionStore {
    /// Build a store seeded only by the optional local-dev owner token. `None`
    /// (the production default) yields an empty store; `Some(token)` seeds one
    /// deterministic owner principal reachable through that bearer token.
    #[must_use]
    pub fn from_owner_token(owner_token: Option<String>) -> Self {
        let mut tokens = HashMap::new();
        // A set-but-empty/blank token (e.g. `OATHSTAR_DEV_OWNER=`) is treated as
        // absent and must NEVER seed a principal: otherwise a token-less
        // `Authorization: Bearer ` request would resolve to the empty-string key
        // and gain owner authority — an auth bypass (inspect, ticket #41).
        if let Some(token) = owner_token.filter(|candidate| !candidate.trim().is_empty()) {
            tokens.insert(
                token,
                Principal {
                    id: "dev-owner".to_owned(),
                    name: "Local Dev Owner".to_owned(),
                    roles: vec![AuthRole::Owner],
                },
            );
        }
        Self { tokens }
    }

    /// Build the store from the environment: `OATHSTAR_DEV_OWNER`, when set,
    /// supplies the bearer token that grants a deterministic local-dev owner;
    /// unset (the production default) leaves the store empty. This is the only
    /// development bypass, and it never weakens production — production simply
    /// does not set the variable.
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_owner_token(std::env::var("OATHSTAR_DEV_OWNER").ok())
    }

    /// Resolve a bearer token to its principal, if the token is known.
    #[must_use]
    pub fn resolve(&self, token: &str) -> Option<Principal> {
        self.tokens.get(token).cloned()
    }
}

/// A refusal at the auth boundary. Renders as a real status code (401 or 403)
/// with a small `error` body — never the in-band `200 {ok:false}` convention,
/// so an unauthorized request can never be mistaken for an accepted one.
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

/// Resolve the caller's [`Principal`] from the request headers, or refuse with
/// [`AuthError::Unauthorized`]. Expects an `Authorization: Bearer <token>`
/// header carrying a token known to `sessions`.
pub fn authenticate(headers: &HeaderMap, sessions: &SessionStore) -> Result<Principal, AuthError> {
    let header = headers
        .get(AUTHORIZATION)
        .ok_or(AuthError::Unauthorized("missing authorization header"))?;
    let value = header
        .to_str()
        .map_err(|_| AuthError::Unauthorized("malformed authorization header"))?;
    let token = value
        .strip_prefix("Bearer ")
        .ok_or(AuthError::Unauthorized("expected a bearer token"))?
        .trim();
    // Defense in depth: a blank token never authenticates, even if some future
    // seed path admitted a blank key (see `SessionStore::from_owner_token`).
    if token.is_empty() {
        return Err(AuthError::Unauthorized("expected a bearer token"));
    }
    sessions
        .resolve(token)
        .ok_or(AuthError::Unauthorized("invalid session token"))
}

/// Require that `principal` is granted `required`, or refuse with
/// [`AuthError::Forbidden`].
pub fn require_role(principal: &Principal, required: AuthRole) -> Result<(), AuthError> {
    if principal.grants(required) {
        Ok(())
    } else {
        Err(AuthError::Forbidden("missing required role"))
    }
}

/// An axum extractor yielding the authenticated [`Principal`] for a protected
/// handler, refusing unauthenticated callers with 401 before the handler body
/// runs. Role checks are an explicit per-handler step via [`require_role`].
#[derive(Debug, Clone)]
pub struct AuthPrincipal(pub Principal);

impl FromRequestParts<AppState> for AuthPrincipal {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let principal = authenticate(&parts.headers, &state.auth_sessions)?;
        Ok(Self(principal))
    }
}

#[cfg(test)]
mod tests {
    use super::{authenticate, require_role, AuthError, SessionStore};
    use axum::http::{header::AUTHORIZATION, HeaderMap, HeaderValue, StatusCode};
    use axum::response::IntoResponse;
    use oathstar_protocol::{AuthRole, Principal};
    use std::collections::HashMap;

    fn principal(roles: Vec<AuthRole>) -> Principal {
        Principal {
            id: "u".to_owned(),
            name: "User".to_owned(),
            roles,
        }
    }

    fn store_with(token: &str, roles: Vec<AuthRole>) -> SessionStore {
        SessionStore {
            tokens: HashMap::from([(token.to_owned(), principal(roles))]),
        }
    }

    fn auth_header(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(value).expect("valid header value"),
        );
        headers
    }

    #[test]
    fn authenticate_without_header_is_unauthorized() {
        let result = authenticate(&HeaderMap::new(), &SessionStore::default());
        assert!(matches!(result, Err(AuthError::Unauthorized(_))));
    }

    #[test]
    fn authenticate_rejects_non_bearer_schemes() {
        let store = store_with("tok", vec![AuthRole::Owner]);
        for bad in ["Basic tok", "tok", "Bearer", "bearer tok", "BEARER tok"] {
            assert!(
                matches!(
                    authenticate(&auth_header(bad), &store),
                    Err(AuthError::Unauthorized(_))
                ),
                "{bad:?} must be unauthorized",
            );
        }
    }

    #[test]
    fn authenticate_rejects_unknown_token() {
        let store = store_with("known", vec![AuthRole::Owner]);
        assert!(matches!(
            authenticate(&auth_header("Bearer nope"), &store),
            Err(AuthError::Unauthorized(_))
        ));
    }

    #[test]
    fn authenticate_resolves_a_known_token() {
        let store = store_with("s3cr3t", vec![AuthRole::Editor]);
        let resolved =
            authenticate(&auth_header("Bearer s3cr3t"), &store).expect("known token resolves");
        assert_eq!(resolved.id, "u");
        assert_eq!(resolved.roles, vec![AuthRole::Editor]);
    }

    // inspect (ticket #41): a blank bearer token never authenticates, even if a
    // store somehow carried an empty key.
    #[test]
    fn authenticate_rejects_a_blank_bearer_token() {
        let store = store_with("", vec![AuthRole::Owner]);
        for blank in ["Bearer ", "Bearer    "] {
            assert!(
                matches!(
                    authenticate(&auth_header(blank), &store),
                    Err(AuthError::Unauthorized(_))
                ),
                "{blank:?} must be unauthorized",
            );
        }
    }

    #[test]
    fn from_owner_token_seeds_owner_and_ignores_blank() {
        let seeded = SessionStore::from_owner_token(Some("devtok".to_owned()));
        assert!(seeded
            .resolve("devtok")
            .expect("owner seeded")
            .grants(AuthRole::Owner));
        assert!(seeded.resolve("other").is_none());

        // None and blank/whitespace are treated as absent — the inspect bypass fix.
        assert!(SessionStore::from_owner_token(None).resolve("").is_none());
        assert!(SessionStore::from_owner_token(Some(String::new()))
            .resolve("")
            .is_none());
        assert!(SessionStore::from_owner_token(Some("   ".to_owned()))
            .resolve("")
            .is_none());
    }

    #[test]
    fn require_role_enforces_grants() {
        assert!(require_role(&principal(vec![AuthRole::Editor]), AuthRole::Editor).is_ok());
        assert!(require_role(&principal(vec![AuthRole::Owner]), AuthRole::Editor).is_ok());
        assert!(matches!(
            require_role(&principal(vec![AuthRole::Player]), AuthRole::Editor),
            Err(AuthError::Forbidden(_))
        ));
    }

    #[test]
    fn auth_error_renders_real_status_codes() {
        assert_eq!(
            AuthError::Unauthorized("x").into_response().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AuthError::Forbidden("x").into_response().status(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn authenticate_trims_surrounding_whitespace_on_a_token() {
        let store = store_with("tok", vec![AuthRole::Editor]);
        // an extra space after `Bearer ` and a trailing space both trim to "tok"
        for padded in ["Bearer  tok", "Bearer tok "] {
            assert!(
                authenticate(&auth_header(padded), &store).is_ok(),
                "{padded:?} must resolve via trim",
            );
        }
    }

    #[test]
    fn authenticate_rejects_a_non_utf8_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_bytes(&[0xff, 0xfe]).expect("opaque header bytes"),
        );
        assert!(matches!(
            authenticate(&headers, &SessionStore::default()),
            Err(AuthError::Unauthorized(_))
        ));
    }

    #[test]
    fn from_env_seeds_the_owner_from_the_dev_var() {
        // No other test reads OATHSTAR_DEV_OWNER, so this set/remove is isolated.
        std::env::set_var("OATHSTAR_DEV_OWNER", "env-owner-tok");
        let store = SessionStore::from_env();
        std::env::remove_var("OATHSTAR_DEV_OWNER");
        assert!(store
            .resolve("env-owner-tok")
            .expect("from_env seeds the dev owner")
            .grants(AuthRole::Owner));
    }
}
