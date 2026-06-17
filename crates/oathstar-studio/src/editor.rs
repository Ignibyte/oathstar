//! The studio map editor's server endpoint (ticket #44).
//!
//! `POST /editor/maps/validate` accepts a posted [`MapDocument`], validates and
//! materializes it against the server's content catalog behind the Editor gate,
//! and answers with a typed JSON summary (success) or a cell/ref-naming error.

use axum::{
    body::Bytes,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use oathstar_auth::{principal_from_cookie, require_role, AuthRole};
use oathstar_content::{MapDocument, MapValidationError};
use serde::Serialize;

use crate::{render, StudioState};

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

/// A ref-free, valid starter [`MapDocument`] (JSON) the editor opens with — a
/// 16×12 paint canvas with a small walled sketch in the top-left corner (a floor
/// corridor: rooms `atrium` and `hall`, `spawn` on `atrium`); the rest of the
/// grid is empty and paintable. Ref-free, so it validates against any catalog;
/// embedded verbatim into the page and sent by the Validate control.
const STARTER_DOC: &str = r#"{
  "id": "sketch-map",
  "title": "Sketch Map",
  "tile_size": 16,
  "width": 16,
  "height": 12,
  "floors": 1,
  "terrain_palette": {
    "floor": { "tile": "stone_floor", "passable": true },
    "wall": { "tile": "wall_face", "passable": false }
  },
  "terrain": [
    { "x": 0, "y": 0, "z": 0, "terrain": "wall" },
    { "x": 1, "y": 0, "z": 0, "terrain": "wall" },
    { "x": 2, "y": 0, "z": 0, "terrain": "wall" },
    { "x": 3, "y": 0, "z": 0, "terrain": "wall" },
    { "x": 4, "y": 0, "z": 0, "terrain": "wall" },
    { "x": 0, "y": 1, "z": 0, "terrain": "wall" },
    { "x": 1, "y": 1, "z": 0, "terrain": "floor" },
    { "x": 2, "y": 1, "z": 0, "terrain": "floor" },
    { "x": 3, "y": 1, "z": 0, "terrain": "floor" },
    { "x": 4, "y": 1, "z": 0, "terrain": "wall" },
    { "x": 0, "y": 2, "z": 0, "terrain": "wall" },
    { "x": 1, "y": 2, "z": 0, "terrain": "wall" },
    { "x": 2, "y": 2, "z": 0, "terrain": "wall" },
    { "x": 3, "y": 2, "z": 0, "terrain": "wall" },
    { "x": 4, "y": 2, "z": 0, "terrain": "wall" }
  ],
  "regions": {
    "sketch": { "id": "sketch", "name": "Sketch" }
  },
  "rooms": [
    { "x": 1, "y": 1, "z": 0, "id": "atrium", "region": "sketch", "exits": { "east": "hall" } },
    { "x": 3, "y": 1, "z": 0, "id": "hall", "region": "sketch", "exits": { "west": "atrium" } }
  ],
  "spawn": { "x": 1, "y": 1, "z": 0 },
  "tilesets": [
    { "id": "arctic", "image": "arctic.png", "tile_size": 8, "columns": 30, "rows": 203 }
  ],
  "layers": [
    { "id": "ground", "name": "Ground", "kind": "tile", "visible": true, "cells": [] }
  ]
}"#;

/// `GET /editor` — the studio map editor canvas page (ticket #45).
///
/// Editor-gated like [`crate::handlers::dashboard`] (an Owner session grants
/// Editor): redirects to `/login` without a valid editor session, otherwise
/// renders the canvas shell around [`STARTER_DOC`]. The page validates by
/// posting the document to [`validate`]; this handler itself takes no input.
pub async fn editor_page(State(studio): State<StudioState>, jar: CookieJar) -> Response {
    let Some(principal) = principal_from_cookie(&jar, &studio.sessions) else {
        return Redirect::to("/login").into_response();
    };
    if require_role(&principal, AuthRole::Editor).is_err() {
        return Redirect::to("/login").into_response();
    }
    render::editor_page(STARTER_DOC).into_response()
}

/// The committed arctic tile sheet, embedded so the loopback studio serves it
/// without a runtime asset dir (Decision 058). The editor canvas fetches it from
/// `/tilesets/arctic.png` to draw the palette and the painted sprites.
const ARCTIC_PNG: &[u8] = include_bytes!("../../../public/tilesets/arctic.png");

/// `GET /tilesets/arctic.png` — the embedded arctic sheet, served as `image/png`.
pub async fn arctic_sheet() -> Response {
    (
        [(header::CONTENT_TYPE, "image/png")],
        Bytes::from_static(ARCTIC_PNG),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::{arctic_sheet, editor_page, validate, STARTER_DOC};
    use crate::StudioState;
    use axum::body::{to_bytes, Body, Bytes};
    use axum::extract::{FromRequestParts, State};
    use axum::http::{header, Request, StatusCode};
    use axum_extra::extract::cookie::CookieJar;
    use oathstar_auth::{owner_principal, AuthRole, Principal, SessionStore, SESSION_COOKIE};
    use oathstar_content::{ContentCatalog, MapDocument};
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
        "id":"m","title":"M","tile_size":7,"width":4,"height":4,"floors":1,
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

    /// Call `editor_page` (the GET page handler) with an optional session cookie.
    async fn page(state: StudioState, cookie: Option<String>) -> axum::response::Response {
        let jar = jar(cookie.as_deref()).await;
        editor_page(State(state), jar).await
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

    async fn body_string(response: axum::response::Response) -> String {
        let bytes = to_bytes(response.into_body(), 1 << 20)
            .await
            .expect("the body collects");
        String::from_utf8(bytes.to_vec()).expect("a utf-8 body")
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
                .contains("tile size 7 is unsupported"),
            "message must name the offender: {}",
            body["message"]
        );
        assert_eq!(body["error"]["UnsupportedTileSize"]["found"], 7);
    }

    #[tokio::test]
    async fn editor_page_redirects_anonymous() {
        // REQ-001 / T1: no session → redirect to /login, page not served.
        let response = page(studio(), None).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response), "/login");
    }

    #[tokio::test]
    async fn editor_page_redirects_a_player() {
        // REQ-001 / T2: a real Player session is refused. Paired with the editor/owner
        // 200s below, this kills the role-gate mutant (PR-claude-gated-page-role-mutant-001) —
        // a no-cookie-only test would leave the require_role branch alive.
        let state = studio();
        let id = state
            .sessions
            .create_session(principal(vec![AuthRole::Player]));
        let response = page(state, Some(cookie_header(&id))).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response), "/login");
    }

    #[tokio::test]
    async fn editor_page_renders_for_an_editor() {
        // REQ-002 / T3: an Editor sees the canvas shell with the embedded starter doc.
        let state = studio();
        let id = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        let response = page(state, Some(cookie_header(&id))).await;
        assert_eq!(response.status(), StatusCode::OK);
        let html = body_string(response).await;
        assert!(html.contains(r#"class="editor""#));
        assert!(html.contains(r#"<canvas id="map""#));
        assert!(html.contains(r#"id="map-doc""#));
        assert!(html.contains(r#"id="validate""#));
        assert!(html.contains(r#"id="result""#));
        assert!(html.contains(r#"<a href="/">"#));
        assert!(html.contains("Sketch Map")); // the embedded starter doc title
    }

    #[tokio::test]
    async fn serves_the_arctic_sheet() {
        // ticket #48: the studio serves the embedded sheet so the editor palette
        // and painted sprites actually render (the studio has no runtime asset dir).
        let res = arctic_sheet().await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers()
                .get(header::CONTENT_TYPE)
                .expect("content-type present"),
            "image/png"
        );
        let body = to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        assert!(!body.is_empty(), "the sheet is served");
    }

    #[tokio::test]
    async fn editor_page_admits_an_owner() {
        // REQ-002 / T4: an Owner session grants Editor and reaches the page.
        let state = studio();
        let id = state.sessions.create_session(owner_principal());
        let response = page(state, Some(cookie_header(&id))).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn starter_doc_is_valid() {
        // REQ-006/REQ-008 / T12: the shipped starter doc deserializes, materializes,
        // and validates ok:true through the real endpoint (2 rooms, 1 region, start atrium).
        let doc: MapDocument = serde_json::from_str(STARTER_DOC).expect("STARTER_DOC parses");
        let world = doc
            .materialize(&ContentCatalog::default())
            .expect("STARTER_DOC materializes");
        assert_eq!(world.rooms.len(), 2);
        assert_eq!(world.regions.len(), 1);
        assert_eq!(world.start_room_id, "atrium");

        let state = studio();
        let id = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        let (status, body) = call(state, Some(cookie_header(&id)), STARTER_DOC).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], true);
        assert_eq!(body["room_count"], 2);
        assert_eq!(body["region_count"], 1);
        assert_eq!(body["start_room_id"], "atrium");
    }
}
