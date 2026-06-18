//! The studio's section routes (ticket #49). `regions` is the read-only region &
//! sub-region dashboard (ticket #51); `items` / `enemies` / `settings` are still
//! Editor-gated "Coming soon" stubs (HTTP 200, never a 404) until their tickets
//! land. All are gated like the dashboard.

use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use oathstar_auth::{principal_from_cookie, require_role, AuthRole, Principal, SessionStore};

use crate::{
    render::{self, NavSection},
    StudioState,
};

/// The shared Editor gate: the session principal when it grants the Editor role,
/// else `None`. Mirrors the dashboard/editor gate — a missing session and a
/// non-editor both yield `None` (the caller redirects to `/login`).
fn editor_gate(jar: &CookieJar, sessions: &SessionStore) -> Option<Principal> {
    let principal = principal_from_cookie(jar, sessions)?;
    require_role(&principal, AuthRole::Editor).ok()?;
    Some(principal)
}

/// Render `section`'s stub behind the Editor gate, or redirect to `/login`.
fn gated_section(jar: &CookieJar, studio: &StudioState, section: NavSection) -> Response {
    editor_gate(jar, &studio.sessions).map_or_else(
        || Redirect::to("/login").into_response(),
        |_principal| render::section_stub_page(section).into_response(),
    )
}

/// `GET /regions` — the read-only region & sub-region dashboard (ticket #51): the
/// regions and sub-regions of the loaded world, behind the Editor gate.
pub async fn regions(State(studio): State<StudioState>, jar: CookieJar) -> Response {
    editor_gate(&jar, &studio.sessions).map_or_else(
        || Redirect::to("/login").into_response(),
        |_principal| render::regions_page(&studio.world).into_response(),
    )
}

/// `GET /items` — the item editor (stub).
pub async fn items(State(studio): State<StudioState>, jar: CookieJar) -> Response {
    gated_section(&jar, &studio, NavSection::Items)
}

/// `GET /enemies` — the enemy editor (stub).
pub async fn enemies(State(studio): State<StudioState>, jar: CookieJar) -> Response {
    gated_section(&jar, &studio, NavSection::Enemies)
}

/// `GET /settings` — game settings (stub).
pub async fn settings(State(studio): State<StudioState>, jar: CookieJar) -> Response {
    gated_section(&jar, &studio, NavSection::Settings)
}

#[cfg(test)]
mod tests {
    use super::{enemies, items, regions, settings};
    use crate::StudioState;
    use axum::body::{to_bytes, Body};
    use axum::extract::{FromRequestParts, State};
    use axum::http::{header, Request, StatusCode};
    use axum_extra::extract::cookie::CookieJar;
    use oathstar_auth::{owner_principal, AuthRole, Principal, SessionStore, SESSION_COOKIE};
    use oathstar_content::{load_beginner_world, ContentCatalog};
    use std::sync::Arc;

    fn studio() -> StudioState {
        StudioState {
            sessions: SessionStore::new(),
            owner_secret: Some("pw".to_owned()),
            catalog: Arc::new(ContentCatalog::default()),
            world: Arc::new(load_beginner_world().expect("the beginner world loads")),
            // These tests never touch the maps store; a never-written placeholder.
            maps: oathstar_storage::FileSaveStore::new(
                std::env::temp_dir().join("oathstar-studio-test-unused"),
            ),
        }
    }

    fn principal(roles: Vec<AuthRole>) -> Principal {
        Principal {
            id: "u".to_owned(),
            name: "U".to_owned(),
            roles,
        }
    }

    async fn jar(cookie: Option<&str>) -> CookieJar {
        let mut builder = Request::builder();
        if let Some(value) = cookie {
            builder = builder.header(header::COOKIE, value);
        }
        let request = builder.body(Body::empty()).expect("a request builds");
        let (mut parts, _) = request.into_parts();
        CookieJar::from_request_parts(&mut parts, &())
            .await
            .expect("the cookie jar extractor is infallible")
    }

    fn cookie_header(id: &str) -> String {
        format!("{SESSION_COOKIE}={id}")
    }

    async fn body_of(response: axum::response::Response) -> String {
        let bytes = to_bytes(response.into_body(), 1 << 20)
            .await
            .expect("the body collects");
        String::from_utf8(bytes.to_vec()).expect("a utf-8 body")
    }

    fn location(response: &axum::response::Response) -> String {
        response
            .headers()
            .get(header::LOCATION)
            .expect("a Location header")
            .to_str()
            .expect("an ascii Location")
            .to_owned()
    }

    // Each section route, the three-way gate (REQ-003 + REQ-004): an Editor sees
    // the stub (200, names the section + "Coming soon"); a Player AND an anonymous
    // caller are both redirected to /login. The Player+Editor pair per route kills
    // the `require_role` mutant (PR-claude-gated-page-role-mutant-001).
    macro_rules! gate_tests {
        ($name:ident, $handler:path, $label:literal) => {
            #[tokio::test]
            async fn $name() {
                let state = studio();
                let id = state
                    .sessions
                    .create_session(principal(vec![AuthRole::Editor]));
                let res = $handler(State(state), jar(Some(&cookie_header(&id))).await).await;
                assert_eq!(res.status(), StatusCode::OK);
                let body = body_of(res).await;
                assert!(body.contains($label), "the stub names its section");
                assert!(body.contains("Coming soon."));

                let state = studio();
                let pid = state
                    .sessions
                    .create_session(principal(vec![AuthRole::Player]));
                let res = $handler(State(state), jar(Some(&cookie_header(&pid))).await).await;
                assert_eq!(res.status(), StatusCode::SEE_OTHER);
                assert_eq!(location(&res), "/login");

                let state = studio();
                let res = $handler(State(state), jar(None).await).await;
                assert_eq!(res.status(), StatusCode::SEE_OTHER);
                assert_eq!(location(&res), "/login");
            }
        };
    }

    gate_tests!(items_is_gated_stub, items, "Items");
    gate_tests!(enemies_is_gated_stub, enemies, "Enemies");
    gate_tests!(settings_is_gated_stub, settings, "Game Settings");

    #[tokio::test]
    async fn regions_serves_the_dashboard_and_is_gated() {
        // ticket #51: `regions` renders the real read-only dashboard, not a stub —
        // an Editor sees a real region; a Player AND an anonymous caller are both
        // redirected to /login (PR-claude-gated-page-role-mutant-001).
        let state = studio();
        let id = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        let res = regions(State(state), jar(Some(&cookie_header(&id))).await).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body = body_of(res).await;
        assert!(body.contains("Hollowmere"), "lists a real region");
        assert!(!body.contains("Coming soon."), "not the stub");

        let state = studio();
        let pid = state
            .sessions
            .create_session(principal(vec![AuthRole::Player]));
        let res = regions(State(state), jar(Some(&cookie_header(&pid))).await).await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&res), "/login");

        let state = studio();
        let res = regions(State(state), jar(None).await).await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&res), "/login");
    }

    #[tokio::test]
    async fn an_owner_session_reaches_a_section() {
        // An Owner session grants Editor and reaches a section stub.
        let state = studio();
        let id = state.sessions.create_session(owner_principal());
        let res = regions(State(state), jar(Some(&cookie_header(&id))).await).await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}
