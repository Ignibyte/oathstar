//! The studio map editor's server endpoint (ticket #44).
//!
//! `POST /editor/maps/validate` accepts a posted [`MapDocument`], validates and
//! materializes it against the server's content catalog behind the Editor gate,
//! and answers with a typed JSON summary (success) or a cell/ref-naming error.

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use oathstar_auth::{principal_from_cookie, require_role, AuthRole, SessionStore};
use oathstar_content::{MapDocument, MapValidationError};
use oathstar_storage::{validate_save_slot_name, SaveStore};
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

/// A successful map save — the id under which the document was persisted.
#[derive(Serialize)]
struct Saved {
    ok: bool,
    id: String,
}

/// The ids of all persisted map documents.
#[derive(Serialize)]
struct MapList {
    ok: bool,
    maps: Vec<String>,
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

/// The refusal [`Response`] for a request that is not an authorized Editor:
/// `Some(401)` when there is no session, `Some(403)` for a non-editor, or `None`
/// when the cookie resolves to a principal that grants the Editor role.
fn editor_refusal(jar: &CookieJar, sessions: &SessionStore) -> Option<Response> {
    let Some(principal) = principal_from_cookie(jar, sessions) else {
        return Some(refuse(StatusCode::UNAUTHORIZED, "authentication required"));
    };
    if require_role(&principal, AuthRole::Editor).is_err() {
        return Some(refuse(StatusCode::FORBIDDEN, "editor role required"));
    }
    None
}

/// `POST /editor/maps/validate` — validate + materialize a posted map document.
///
/// Gated to the Editor role (an Owner session grants it). Answers `200 {ok:true,
/// …summary}` for a valid document, `200 {ok:false, message, error}` for one that
/// fails validation, and `401`/`403`/`400` (typed JSON) for a missing session, a
/// non-editor, or a malformed body.
pub async fn validate(State(studio): State<StudioState>, jar: CookieJar, body: Bytes) -> Response {
    if let Some(refusal) = editor_refusal(&jar, &studio.sessions) {
        return refusal;
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

/// `POST /editor/maps` — persist a posted map document under its own id.
///
/// Gated to the Editor role. The document's `id` is the storage slot and must
/// pass [`validate_save_slot_name`] (`400` otherwise). Drafts are allowed — the
/// document is NOT required to materialize (use `/editor/maps/validate` for that
/// feedback). Answers `200 {ok:true, id}` on success.
pub async fn save_map(State(studio): State<StudioState>, jar: CookieJar, body: Bytes) -> Response {
    if let Some(refusal) = editor_refusal(&jar, &studio.sessions) {
        return refusal;
    }
    let Ok(document) = serde_json::from_slice::<MapDocument>(&body) else {
        return refuse(
            StatusCode::BAD_REQUEST,
            "request body is not a valid map document",
        );
    };
    if validate_save_slot_name(&document.id).is_err() {
        return refuse(
            StatusCode::BAD_REQUEST,
            "map id is not a valid storage name",
        );
    }
    if studio.maps.write_json(&document.id, &document).is_err() {
        return refuse(StatusCode::INTERNAL_SERVER_ERROR, "failed to save map");
    }
    Json(Saved {
        ok: true,
        id: document.id,
    })
    .into_response()
}

/// `GET /editor/maps` — list the ids of all persisted map documents.
///
/// Gated to the Editor role. Answers `200 {ok:true, maps:[…]}`.
pub async fn list_maps(State(studio): State<StudioState>, jar: CookieJar) -> Response {
    if let Some(refusal) = editor_refusal(&jar, &studio.sessions) {
        return refusal;
    }
    studio.maps.list().map_or_else(
        |_| refuse(StatusCode::INTERNAL_SERVER_ERROR, "failed to list maps"),
        |maps| Json(MapList { ok: true, maps }).into_response(),
    )
}

/// `GET /editor/maps/{id}` — return a persisted map document for reopening.
///
/// Gated to the Editor role. The `id` must pass [`validate_save_slot_name`]
/// (`400` otherwise). Answers `200` with the document JSON, or `404` when no map
/// is stored under that id.
pub async fn load_map(
    State(studio): State<StudioState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Response {
    if let Some(refusal) = editor_refusal(&jar, &studio.sessions) {
        return refusal;
    }
    if validate_save_slot_name(&id).is_err() {
        return refuse(
            StatusCode::BAD_REQUEST,
            "map id is not a valid storage name",
        );
    }
    studio.maps.read_json::<MapDocument>(&id).map_or_else(
        |_| refuse(StatusCode::NOT_FOUND, "map not found"),
        |document| Json(document).into_response(),
    )
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

/// `GET /tilesets/arctic.png` — the arctic tile sheet, served as `image/png` from the
/// runtime assets dir (`OATHSTAR_ASSETS_DIR`, #59) so the owner can swap it without a
/// rebuild. The editor canvas fetches it to draw the palette and the painted sprites.
pub async fn arctic_sheet(State(studio): State<StudioState>) -> Response {
    crate::assets::serve_png(&studio.assets_dir, "tilesets/arctic.png").await
}

#[cfg(test)]
mod tests {
    use super::{arctic_sheet, editor_page, list_maps, load_map, save_map, validate, STARTER_DOC};
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
        use std::sync::atomic::{AtomicU32, Ordering};
        // A unique, freshly-emptied maps dir per call so save/list/load tests
        // never collide or see stale entries from a prior run.
        static MAPS_DIR_SEQ: AtomicU32 = AtomicU32::new(0);
        let seq = MAPS_DIR_SEQ.fetch_add(1, Ordering::Relaxed);
        let maps_dir = std::env::temp_dir().join(format!("oathstar-studio-test-maps-{seq}"));
        let _ = std::fs::remove_dir_all(&maps_dir);
        StudioState {
            sessions: SessionStore::new(),
            owner_secret: Some("pw".to_owned()),
            catalog: Arc::new(ContentCatalog::default()),
            maps: oathstar_storage::FileSaveStore::new(maps_dir),
            assets_dir: std::path::PathBuf::from("public"),
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

    /// Collect a handler `Response` into `(status, json body)`.
    async fn decode(response: axum::response::Response) -> (StatusCode, serde_json::Value) {
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1 << 20)
            .await
            .expect("the body collects");
        (status, serde_json::from_slice(&bytes).expect("a JSON body"))
    }

    async fn call_save(
        state: StudioState,
        cookie: Option<String>,
        body: &str,
    ) -> (StatusCode, serde_json::Value) {
        let jar = jar(cookie.as_deref()).await;
        decode(save_map(State(state), jar, Bytes::from(body.to_owned())).await).await
    }

    async fn call_list(
        state: StudioState,
        cookie: Option<String>,
    ) -> (StatusCode, serde_json::Value) {
        let jar = jar(cookie.as_deref()).await;
        decode(list_maps(State(state), jar).await).await
    }

    async fn call_load(
        state: StudioState,
        cookie: Option<String>,
        id: &str,
    ) -> (StatusCode, serde_json::Value) {
        let jar = jar(cookie.as_deref()).await;
        decode(load_map(State(state), jar, axum::extract::Path(id.to_owned())).await).await
    }

    /// A studio whose maps store root is a regular FILE, so every store op fails —
    /// exercises the 500 paths.
    fn studio_with_maps_as_file(tag: &str) -> StudioState {
        let file = std::env::temp_dir().join(format!("oathstar-studio-test-maps-file-{tag}"));
        std::fs::remove_dir_all(&file).ok();
        std::fs::write(&file, b"not a directory").expect("seed a file");
        let mut state = studio();
        state.maps = oathstar_storage::FileSaveStore::new(file);
        state
    }

    #[tokio::test]
    async fn save_list_load_round_trips_for_an_editor() {
        let state = studio();
        let session = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        let cookie = cookie_header(&session);

        let (status, body) = call_save(state.clone(), Some(cookie.clone()), VALID_DOC).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], true);
        assert_eq!(body["id"], "m");

        let (status, body) = call_list(state.clone(), Some(cookie.clone())).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], true);
        assert_eq!(body["maps"], serde_json::json!(["m"]));

        let (status, body) = call_load(state, Some(cookie), "m").await;
        assert_eq!(status, StatusCode::OK);
        let loaded: MapDocument = serde_json::from_value(body).expect("load returns a document");
        let expected: MapDocument = serde_json::from_str(VALID_DOC).expect("VALID_DOC parses");
        assert_eq!(loaded, expected, "load returns the saved document");
    }

    #[tokio::test]
    async fn map_endpoints_refuse_an_anonymous_caller() {
        assert_eq!(
            call_save(studio(), None, VALID_DOC).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(call_list(studio(), None).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(
            call_load(studio(), None, "m").await.0,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn map_endpoints_refuse_a_non_editor() {
        let with_player = || {
            let state = studio();
            let session = state
                .sessions
                .create_session(principal(vec![AuthRole::Player]));
            let cookie = cookie_header(&session);
            (state, cookie)
        };
        let (state, cookie) = with_player();
        assert_eq!(
            call_save(state, Some(cookie), VALID_DOC).await.0,
            StatusCode::FORBIDDEN
        );
        let (state, cookie) = with_player();
        assert_eq!(
            call_list(state, Some(cookie)).await.0,
            StatusCode::FORBIDDEN
        );
        let (state, cookie) = with_player();
        assert_eq!(
            call_load(state, Some(cookie), "m").await.0,
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn save_rejects_a_malformed_body() {
        let state = studio();
        let session = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        let (status, body) = call_save(state, Some(cookie_header(&session)), "{not json").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["message"], "request body is not a valid map document");
    }

    #[tokio::test]
    async fn save_rejects_a_path_unsafe_id() {
        let bad = VALID_DOC.replacen(r#""id":"m""#, r#""id":"../escape""#, 1);
        let state = studio();
        let session = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        let (status, body) = call_save(state, Some(cookie_header(&session)), &bad).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["message"], "map id is not a valid storage name");
    }

    #[tokio::test]
    async fn load_a_missing_map_is_not_found() {
        let state = studio();
        let session = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        let (status, body) = call_load(state, Some(cookie_header(&session)), "absent").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["message"], "map not found");
    }

    #[tokio::test]
    async fn load_rejects_a_path_unsafe_id() {
        let state = studio();
        let session = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        let (status, body) = call_load(state, Some(cookie_header(&session)), "../escape").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["message"], "map id is not a valid storage name");
    }

    #[tokio::test]
    async fn save_reports_a_storage_failure() {
        let state = studio_with_maps_as_file("save");
        let session = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        let (status, body) = call_save(state, Some(cookie_header(&session)), VALID_DOC).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["message"], "failed to save map");
    }

    #[tokio::test]
    async fn list_reports_a_storage_failure() {
        let state = studio_with_maps_as_file("list");
        let session = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        let (status, body) = call_list(state, Some(cookie_header(&session))).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["message"], "failed to list maps");
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
        assert!(html.contains(r#"href="/""#)); // the studio nav's brand home link (#49)
        assert!(html.contains("Sketch Map")); // the embedded starter doc title
    }

    #[tokio::test]
    async fn serves_the_arctic_sheet() {
        // #59: the sheet is read from the runtime assets dir and served as image/png.
        // Unique per call (atomic seq, like `studio()`) so the fixture never collides.
        use std::sync::atomic::{AtomicU32, Ordering};
        static ARCTIC_DIR_SEQ: AtomicU32 = AtomicU32::new(0);
        let seq = ARCTIC_DIR_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("oathstar-studio-arctic-asset-{seq}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("tilesets")).expect("temp tilesets dir");
        std::fs::write(dir.join("tilesets/arctic.png"), b"PNGARCTIC").expect("sheet fixture");
        let mut state = studio();
        state.assets_dir = dir;
        let res = arctic_sheet(State(state)).await;
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
        assert_eq!(
            &body[..],
            b"PNGARCTIC",
            "the on-disk sheet bytes are served"
        );
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
