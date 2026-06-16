//! The studio map editor's server endpoint (ticket #44).
//!
//! `POST /editor/maps/validate` accepts a posted [`MapDocument`], validates and
//! materializes it against the server's content catalog behind the Editor gate,
//! and answers with a typed JSON summary (success) or a cell/ref-naming error.

use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use oathstar_auth::{principal_from_cookie, require_role, AuthRole};
use oathstar_content::{MapDocument, MapValidationError};
use serde::Serialize;

use crate::StudioState;

/// A successful validate/materialize — a compact summary of the world.
#[derive(Serialize)]
struct Success {
    ok: bool,
    room_count: usize,
    region_count: usize,
    start_room_id: String,
}

/// A refusal — auth, a malformed body, or a document that failed validation.
#[derive(Serialize)]
struct Failure {
    ok: bool,
    message: String,
    /// The typed validation error — present only when a well-formed document
    /// failed validation, absent for auth/parse refusals.
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<MapValidationError>,
}

/// A refusal response with the given status and message (no typed error).
fn refuse(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(Failure {
            ok: false,
            message: message.to_owned(),
            error: None,
        }),
    )
        .into_response()
}

/// `POST /editor/maps/validate` — validate + materialize a posted map document.
///
/// Gated to the Editor role (an Owner session grants it). Answers `200 {ok:true,
/// …summary}` for a valid document, `200 {ok:false, message, error}` for one that
/// fails validation, and `401`/`403`/`400` (typed JSON) for a missing session, a
/// non-editor, or a malformed body.
pub async fn validate(State(studio): State<StudioState>, jar: CookieJar, body: Bytes) -> Response {
    let Some(principal) = principal_from_cookie(&jar, &studio.sessions) else {
        return refuse(StatusCode::UNAUTHORIZED, "authentication required");
    };
    if require_role(&principal, AuthRole::Editor).is_err() {
        return refuse(StatusCode::FORBIDDEN, "editor role required");
    }

    let Ok(document) = serde_json::from_slice::<MapDocument>(&body) else {
        return refuse(
            StatusCode::BAD_REQUEST,
            "request body is not a valid map document",
        );
    };

    match document.materialize(&studio.catalog) {
        Ok(world) => Json(Success {
            ok: true,
            room_count: world.rooms.len(),
            region_count: world.regions.len(),
            start_room_id: world.start_room_id,
        })
        .into_response(),
        Err(error) => (
            StatusCode::OK,
            Json(Failure {
                ok: false,
                message: error.to_string(),
                error: Some(error),
            }),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::validate;
    use crate::StudioState;
    use axum::body::{to_bytes, Body, Bytes};
    use axum::extract::{FromRequestParts, State};
    use axum::http::{header, Request, StatusCode};
    use axum_extra::extract::cookie::CookieJar;
    use oathstar_auth::{owner_principal, AuthRole, Principal, SessionStore, SESSION_COOKIE};
    use oathstar_content::ContentCatalog;
    use std::sync::Arc;

    /// A valid, reference-free document: two rooms in one region, spawned on `alpha`.
    const VALID_DOC: &str = r#"{
        "id":"m","title":"M","tile_size":16,"width":4,"height":4,"floors":1,
        "terrain_palette":{"floor":{"tile":"f","passable":true}},
        "terrain":[{"x":0,"y":0,"z":0,"terrain":"floor"},{"x":1,"y":0,"z":0,"terrain":"floor"}],
        "regions":{"reg":{"id":"reg","name":"Reg"}},
        "rooms":[{"x":0,"y":0,"z":0,"id":"alpha","region":"reg"},{"x":1,"y":0,"z":0,"id":"beta","region":"reg"}],
        "spawn":{"x":0,"y":0,"z":0}
    }"#;

    /// A well-formed document that fails validation (unsupported tile size).
    const BAD_TILE_DOC: &str = r#"{
        "id":"m","title":"M","tile_size":32,"width":4,"height":4,"floors":1,
        "terrain_palette":{},"terrain":[],"regions":{},"rooms":[],"spawn":null
    }"#;

    fn studio() -> StudioState {
        StudioState {
            sessions: SessionStore::new(),
            owner_secret: Some("pw".to_owned()),
            catalog: Arc::new(ContentCatalog::default()),
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

    async fn call(
        state: StudioState,
        cookie: Option<String>,
        body: &str,
    ) -> (StatusCode, serde_json::Value) {
        let jar = jar(cookie.as_deref()).await;
        let response = validate(State(state), jar, Bytes::from(body.to_owned())).await;
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1 << 20)
            .await
            .expect("the body collects");
        let value = serde_json::from_slice(&bytes).expect("a JSON body");
        (status, value)
    }

    #[tokio::test]
    async fn refuses_an_anonymous_caller() {
        let (status, body) = call(studio(), None, VALID_DOC).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["ok"], false);
        assert_eq!(body["message"], "authentication required");
    }

    #[tokio::test]
    async fn refuses_a_non_editor() {
        let state = studio();
        let id = state
            .sessions
            .create_session(principal(vec![AuthRole::Player]));
        let (status, body) = call(state, Some(cookie_header(&id)), VALID_DOC).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["ok"], false);
        assert_eq!(body["message"], "editor role required");
    }

    #[tokio::test]
    async fn summarizes_a_valid_document_for_an_editor() {
        let state = studio();
        let id = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        let (status, body) = call(state, Some(cookie_header(&id)), VALID_DOC).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], true);
        assert_eq!(body["room_count"], 2);
        assert_eq!(body["region_count"], 1);
        assert_eq!(body["start_room_id"], "alpha");
        assert!(
            body.get("error").is_none(),
            "success carries no error field"
        );
    }

    #[tokio::test]
    async fn an_owner_session_is_admitted() {
        let state = studio();
        let id = state.sessions.create_session(owner_principal());
        let (status, body) = call(state, Some(cookie_header(&id)), VALID_DOC).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], true);
    }

    #[tokio::test]
    async fn rejects_a_malformed_body() {
        let state = studio();
        let id = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        let (status, body) = call(state, Some(cookie_header(&id)), "{not json").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["ok"], false);
        assert_eq!(body["message"], "request body is not a valid map document");
    }

    #[tokio::test]
    async fn reports_an_invalid_document_with_a_named_error() {
        let state = studio();
        let id = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        let (status, body) = call(state, Some(cookie_header(&id)), BAD_TILE_DOC).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], false);
        assert!(
            body["message"]
                .as_str()
                .expect("message is a string")
                .contains("tile size 32 is unsupported"),
            "message must name the offender: {}",
            body["message"]
        );
        assert_eq!(body["error"]["UnsupportedTileSize"]["found"], 32);
    }
}
