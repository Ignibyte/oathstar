use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts, HeaderMap},
};
use axum_extra::extract::cookie::CookieJar;
use oathstar_protocol::{AuthRole, Principal};

use crate::{cookie::SESSION_COOKIE, error::AuthError, session::SessionStore};

/// Resolve a caller from an `Authorization: Bearer <token>` header.
///
/// Resolves against `sessions`, or refuses with [`AuthError::Unauthorized`]. The
/// game server's API path (#41); the studio uses [`principal_from_cookie`].
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

/// Resolve a caller from the studio session cookie, if one is present.
///
/// Resolves the cookie id against `sessions`. Browser surfaces use this — and
/// redirect to the login page on `None` — rather than the 401-returning
/// [`AuthPrincipal`].
#[must_use]
pub fn principal_from_cookie(jar: &CookieJar, sessions: &SessionStore) -> Option<Principal> {
    let id = jar.get(SESSION_COOKIE)?.value();
    sessions.resolve(id)
}

/// An axum extractor yielding the authenticated [`Principal`] (bearer path).
///
/// For a protected API handler — refuses unauthenticated callers with 401 before
/// the handler body runs. Generic over any app state that can produce a
/// [`SessionStore`] via [`FromRef`], so both binaries share one extractor.
#[derive(Debug, Clone)]
pub struct AuthPrincipal(pub Principal);

impl<S> FromRequestParts<S> for AuthPrincipal
where
    S: Send + Sync,
    SessionStore: FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let sessions = SessionStore::from_ref(state);
        let principal = authenticate(&parts.headers, &sessions)?;
        Ok(Self(principal))
    }
}

#[cfg(test)]
mod tests {
    use super::{authenticate, require_role};
    use crate::{error::AuthError, session::SessionStore};
    use axum::http::{header::AUTHORIZATION, HeaderMap, HeaderValue};
    use oathstar_protocol::{AuthRole, Principal};

    fn principal(roles: Vec<AuthRole>) -> Principal {
        Principal {
            id: "u".to_owned(),
            name: "User".to_owned(),
            roles,
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
        // T6
        assert!(matches!(
            authenticate(&HeaderMap::new(), &SessionStore::default()),
            Err(AuthError::Unauthorized(_))
        ));
    }

    #[test]
    fn authenticate_rejects_non_bearer_schemes() {
        // T6
        let store = SessionStore::seeded("tok", principal(vec![AuthRole::Owner]));
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
        let store = SessionStore::seeded("known", principal(vec![AuthRole::Owner]));
        assert!(matches!(
            authenticate(&auth_header("Bearer nope"), &store),
            Err(AuthError::Unauthorized(_))
        ));
    }

    #[test]
    fn authenticate_resolves_a_known_token() {
        let store = SessionStore::seeded("s3cr3t", principal(vec![AuthRole::Editor]));
        let resolved =
            authenticate(&auth_header("Bearer s3cr3t"), &store).expect("known token resolves");
        assert_eq!(resolved.id, "u");
        assert_eq!(resolved.roles, vec![AuthRole::Editor]);
    }

    #[test]
    fn authenticate_rejects_a_blank_bearer_token() {
        // The #41 bypass guard: even a store carrying an empty key must not
        // resolve a blank `Bearer ` — the is_empty() guard fires before resolve.
        let store = SessionStore::seeded("", principal(vec![AuthRole::Owner]));
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
    fn authenticate_trims_surrounding_whitespace() {
        let store = SessionStore::seeded("tok", principal(vec![AuthRole::Editor]));
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
    fn require_role_enforces_grants() {
        // T6
        assert!(require_role(&principal(vec![AuthRole::Editor]), AuthRole::Editor).is_ok());
        assert!(require_role(&principal(vec![AuthRole::Owner]), AuthRole::Editor).is_ok());
        assert!(matches!(
            require_role(&principal(vec![AuthRole::Player]), AuthRole::Editor),
            Err(AuthError::Forbidden(_))
        ));
    }

    #[tokio::test]
    async fn principal_from_cookie_resolves_only_a_live_session() {
        // T5: a live session id → Some; absent or unknown → None. Tested directly
        // (not only cross-crate via the studio) so cargo-mutants — which scopes
        // its test run to this package — kills the `-> None` mutant.
        use crate::cookie::SESSION_COOKIE;
        use crate::session::owner_principal;
        use axum::extract::FromRequestParts;
        use axum::http::{header::COOKIE, Request};
        use axum_extra::extract::cookie::CookieJar;

        async fn jar(cookie: Option<&str>) -> CookieJar {
            let mut builder = Request::builder();
            if let Some(value) = cookie {
                builder = builder.header(COOKIE, value);
            }
            let request = builder.body(()).expect("a request builds");
            let (mut parts, ()) = request.into_parts();
            CookieJar::from_request_parts(&mut parts, &())
                .await
                .expect("the cookie jar extractor is infallible")
        }

        let store = SessionStore::new();
        let id = store.create_session(owner_principal());

        let live = jar(Some(&format!("{SESSION_COOKIE}={id}"))).await;
        assert!(super::principal_from_cookie(&live, &store)
            .expect("a live session resolves")
            .grants(AuthRole::Owner));
        assert!(super::principal_from_cookie(&jar(None).await, &store).is_none());
        let unknown = jar(Some(&format!("{SESSION_COOKIE}=nope"))).await;
        assert!(super::principal_from_cookie(&unknown, &store).is_none());
    }
}
