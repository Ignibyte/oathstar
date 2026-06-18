//! Region & sub-region authoring — the studio's Editor-gated CRUD surface over the
//! persisted authored maps (ticket #51, slice 2).
//!
//! `GET /regions` lists the persisted maps; `GET /regions/{id}` is the per-map
//! editor; `POST /regions/{id}/region` and `…/subregion` apply op-dispatched
//! create / rename / delete operations via the content edit seam (the region methods
//! on [`MapDocument`]). Each accepted edit re-validates through `materialize()` and is
//! persisted to the S1 store; a refusal re-renders the editor with the reason and
//! leaves the stored document unchanged. Every route is gated like the dashboard —
//! an anonymous or non-Editor caller is redirected to `/login`.

use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use oathstar_content::{MapDocument, RegionEditError};
use oathstar_storage::{validate_save_slot_name, SaveStore};
use serde::Deserialize;

use crate::{
    render::{self, MapSummary},
    sections::editor_gate,
    StudioState,
};

/// A region create / rename / delete form. `op` selects the operation; `name` is
/// unused by `delete`.
#[derive(Deserialize)]
pub struct RegionForm {
    op: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
}

/// A sub-region create / rename / delete form. `region` (the parent) is used by
/// `create` only.
#[derive(Deserialize)]
pub struct SubregionForm {
    op: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    region: String,
}

/// Summarize every persisted authored map for the dashboard list. Unreadable or
/// malformed entries are skipped rather than failing the whole list.
fn map_summaries(studio: &StudioState) -> Vec<MapSummary> {
    studio
        .maps
        .list()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|id| {
            studio
                .maps
                .read_json::<MapDocument>(&id)
                .ok()
                .map(|doc| MapSummary {
                    id,
                    title: doc.title,
                    region_count: doc.regions.len(),
                    subregion_count: doc.subregions.len(),
                })
        })
        .collect()
}

/// Load the authored map at `map_id`, or the refusal to return as `(status,
/// message)`: `400` for an invalid storage id, `404` when no map is stored there.
fn load_doc(studio: &StudioState, map_id: &str) -> Result<MapDocument, (StatusCode, &'static str)> {
    if validate_save_slot_name(map_id).is_err() {
        return Err((
            StatusCode::BAD_REQUEST,
            "map id is not a valid storage name",
        ));
    }
    studio
        .maps
        .read_json::<MapDocument>(map_id)
        .map_err(|_| (StatusCode::NOT_FOUND, "map not found"))
}

/// Persist an accepted edit and redirect back to the editor (POST/redirect/GET), or
/// re-render the editor with the refusal / save-failure banner. `original`
/// (unchanged) backs the re-render, so a refused edit never alters the stored map.
fn apply(
    studio: &StudioState,
    map_id: &str,
    original: &MapDocument,
    result: Result<MapDocument, RegionEditError>,
) -> Response {
    match result {
        Ok(edited) => {
            if studio.maps.write_json(map_id, &edited).is_err() {
                return render::region_editor_page(
                    map_id,
                    original,
                    Some("failed to save the map"),
                )
                .into_response();
            }
            Redirect::to(&format!("/regions/{map_id}")).into_response()
        }
        Err(error) => {
            render::region_editor_page(map_id, original, Some(&error.to_string())).into_response()
        }
    }
}

/// `GET /regions` — list the persisted authored maps, each linking to its per-map
/// region editor. Editor-gated.
pub async fn regions(State(studio): State<StudioState>, jar: CookieJar) -> Response {
    if editor_gate(&jar, &studio.sessions).is_none() {
        return Redirect::to("/login").into_response();
    }
    render::regions_list_page(&map_summaries(&studio)).into_response()
}

/// `GET /regions/{id}` — the per-map region & sub-region editor. Editor-gated; `400`
/// on a bad id, `404` when absent.
pub async fn region_editor(
    State(studio): State<StudioState>,
    jar: CookieJar,
    Path(map_id): Path<String>,
) -> Response {
    if editor_gate(&jar, &studio.sessions).is_none() {
        return Redirect::to("/login").into_response();
    }
    match load_doc(&studio, &map_id) {
        Ok(doc) => render::region_editor_page(&map_id, &doc, None).into_response(),
        Err(refusal) => refusal.into_response(),
    }
}

/// `POST /regions/{id}/region` — create / rename / delete a region (op-dispatched).
/// Editor-gated; an unknown `op` is `400`.
pub async fn edit_region(
    State(studio): State<StudioState>,
    jar: CookieJar,
    Path(map_id): Path<String>,
    Form(form): Form<RegionForm>,
) -> Response {
    if editor_gate(&jar, &studio.sessions).is_none() {
        return Redirect::to("/login").into_response();
    }
    let doc = match load_doc(&studio, &map_id) {
        Ok(doc) => doc,
        Err(refusal) => return refusal.into_response(),
    };
    let result = match form.op.as_str() {
        "create" => doc.create_region(&form.id, &form.name, &studio.catalog),
        "rename" => doc.rename_region(&form.id, &form.name, &studio.catalog),
        "delete" => doc.delete_region(&form.id, &studio.catalog),
        _ => return (StatusCode::BAD_REQUEST, "unknown region operation").into_response(),
    };
    apply(&studio, &map_id, &doc, result)
}

/// `POST /regions/{id}/subregion` — create / rename / delete a sub-region
/// (op-dispatched). Editor-gated; an unknown `op` is `400`.
pub async fn edit_subregion(
    State(studio): State<StudioState>,
    jar: CookieJar,
    Path(map_id): Path<String>,
    Form(form): Form<SubregionForm>,
) -> Response {
    if editor_gate(&jar, &studio.sessions).is_none() {
        return Redirect::to("/login").into_response();
    }
    let doc = match load_doc(&studio, &map_id) {
        Ok(doc) => doc,
        Err(refusal) => return refusal.into_response(),
    };
    let result = match form.op.as_str() {
        "create" => doc.create_subregion(&form.id, &form.name, &form.region, &studio.catalog),
        "rename" => doc.rename_subregion(&form.id, &form.name, &studio.catalog),
        "delete" => doc.delete_subregion(&form.id, &studio.catalog),
        _ => return (StatusCode::BAD_REQUEST, "unknown sub-region operation").into_response(),
    };
    apply(&studio, &map_id, &doc, result)
}

#[cfg(test)]
mod tests {
    use super::{
        apply, edit_region, edit_subregion, map_summaries, region_editor, regions, RegionForm,
        SubregionForm,
    };
    use crate::StudioState;
    use axum::body::{to_bytes, Body};
    use axum::extract::{Form, FromRequestParts, Path, State};
    use axum::http::{header, Request, StatusCode};
    use axum_extra::extract::cookie::CookieJar;
    use oathstar_auth::{AuthRole, Principal, SessionStore, SESSION_COOKIE};
    use oathstar_content::{ContentCatalog, MapDocument};
    use oathstar_storage::SaveStore;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    /// A ref-free, materializable map: two rooms in region `reg`, spawned on `alpha`.
    const SEED_DOC: &str = r#"{
        "id":"m","title":"M","tile_size":16,"width":4,"height":4,"floors":1,
        "terrain_palette":{"floor":{"tile":"f","passable":true}},
        "terrain":[{"x":0,"y":0,"z":0,"terrain":"floor"},{"x":1,"y":0,"z":0,"terrain":"floor"}],
        "regions":{"reg":{"id":"reg","name":"Region"}},
        "rooms":[{"x":0,"y":0,"z":0,"id":"alpha","region":"reg"},{"x":1,"y":0,"z":0,"id":"beta","region":"reg"}],
        "spawn":{"x":0,"y":0,"z":0}
    }"#;

    /// A fresh, emptied temp dir per call so the maps stores never collide.
    fn fresh_dir(tag: &str) -> std::path::PathBuf {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("oathstar-studio-regions-{tag}-{seq}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn studio() -> StudioState {
        StudioState {
            sessions: SessionStore::new(),
            owner_secret: Some("pw".to_owned()),
            catalog: Arc::new(ContentCatalog::default()),
            maps: oathstar_storage::FileSaveStore::new(fresh_dir("store")),
        }
    }

    /// A studio with `SEED_DOC` persisted under slot `m`.
    fn seeded_studio() -> StudioState {
        let state = studio();
        let doc: MapDocument = serde_json::from_str(SEED_DOC).expect("seed parses");
        state.maps.write_json("m", &doc).expect("seed writes");
        state
    }

    /// `SEED_DOC` plus an unreferenced sub-region `vale` (parent `reg`) — so it can
    /// be renamed and deleted through the handler.
    const SEED_WITH_SUB: &str = r#"{
        "id":"m","title":"M","tile_size":16,"width":4,"height":4,"floors":1,
        "terrain_palette":{"floor":{"tile":"f","passable":true}},
        "terrain":[{"x":0,"y":0,"z":0,"terrain":"floor"},{"x":1,"y":0,"z":0,"terrain":"floor"}],
        "regions":{"reg":{"id":"reg","name":"Region"}},
        "subregions":{"vale":{"id":"vale","name":"Vale","region":"reg"}},
        "rooms":[{"x":0,"y":0,"z":0,"id":"alpha","region":"reg"},{"x":1,"y":0,"z":0,"id":"beta","region":"reg"}],
        "spawn":{"x":0,"y":0,"z":0}
    }"#;

    fn seeded_studio_with_sub() -> StudioState {
        let state = studio();
        let doc: MapDocument = serde_json::from_str(SEED_WITH_SUB).expect("seed parses");
        state.maps.write_json("m", &doc).expect("seed writes");
        state
    }

    fn principal(roles: Vec<AuthRole>) -> Principal {
        Principal {
            id: "u".to_owned(),
            name: "U".to_owned(),
            roles,
        }
    }

    fn cookie_header(id: &str) -> String {
        format!("{SESSION_COOKIE}={id}")
    }

    fn editor_cookie(state: &StudioState) -> String {
        let id = state
            .sessions
            .create_session(principal(vec![AuthRole::Editor]));
        cookie_header(&id)
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

    async fn decode(response: axum::response::Response) -> (StatusCode, String) {
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1 << 20)
            .await
            .expect("the body collects");
        (
            status,
            String::from_utf8(bytes.to_vec()).expect("a utf-8 body"),
        )
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

    // ---- S1: the dashboard list (REQ-005) ----

    #[tokio::test]
    async fn regions_lists_saved_maps_for_an_editor() {
        let state = studio();
        let alpha: MapDocument =
            serde_json::from_str(&SEED_DOC.replace(r#""title":"M""#, r#""title":"Alpha World""#))
                .expect("parses");
        let bravo: MapDocument =
            serde_json::from_str(&SEED_DOC.replace(r#""title":"M""#, r#""title":"Bravo World""#))
                .expect("parses");
        state.maps.write_json("alpha", &alpha).expect("writes");
        state.maps.write_json("bravo", &bravo).expect("writes");
        let cookie = editor_cookie(&state);
        let (status, body) = decode(regions(State(state), jar(Some(&cookie)).await).await).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("Alpha World"));
        assert!(body.contains("Bravo World"));
        assert!(body.contains(r#"href="/regions/alpha""#));
        assert!(body.contains(r#"href="/regions/bravo""#));
    }

    #[tokio::test]
    async fn regions_shows_an_empty_state_with_no_maps() {
        let state = studio();
        let cookie = editor_cookie(&state);
        let (status, body) = decode(regions(State(state), jar(Some(&cookie)).await).await).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("No authored maps yet"));
    }

    #[tokio::test]
    async fn regions_list_is_gated() {
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
    async fn map_summaries_skips_unreadable_entries() {
        let state = studio();
        let good: MapDocument = serde_json::from_str(SEED_DOC).expect("parses");
        state.maps.write_json("good", &good).expect("writes");
        // A valid slot name whose content is not a MapDocument: listed, but skipped.
        std::fs::write(
            state.maps.root().join("broken.json"),
            b"{\"not\":\"a map\"}",
        )
        .expect("write broken");
        let summaries = map_summaries(&state);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, "good");
    }

    // ---- S2: the per-map editor GET (REQ-005) ----

    #[tokio::test]
    async fn region_editor_renders_for_an_editor() {
        let state = seeded_studio();
        let cookie = editor_cookie(&state);
        let (status, body) = decode(
            region_editor(State(state), jar(Some(&cookie)).await, Path("m".to_owned())).await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("<h2>Region</h2>"));
        assert!(body.contains(r#"action="/regions/m/region""#));
    }

    #[tokio::test]
    async fn region_editor_is_404_for_a_missing_map() {
        let state = studio();
        let cookie = editor_cookie(&state);
        let res = region_editor(
            State(state),
            jar(Some(&cookie)).await,
            Path("absent".to_owned()),
        )
        .await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn region_editor_is_400_for_a_bad_slot() {
        let state = studio();
        let cookie = editor_cookie(&state);
        let res = region_editor(
            State(state),
            jar(Some(&cookie)).await,
            Path("../escape".to_owned()),
        )
        .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn region_editor_is_gated() {
        let state = seeded_studio();
        let pid = state
            .sessions
            .create_session(principal(vec![AuthRole::Player]));
        let res = region_editor(
            State(state),
            jar(Some(&cookie_header(&pid))).await,
            Path("m".to_owned()),
        )
        .await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&res), "/login");

        let state = seeded_studio();
        let res = region_editor(State(state), jar(None).await, Path("m".to_owned())).await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&res), "/login");
    }

    // ---- S3: create / rename round-trips through the store (REQ-001/003) ----

    #[tokio::test]
    async fn create_region_persists_and_redirects() {
        let state = seeded_studio();
        let cookie = editor_cookie(&state);
        let form = RegionForm {
            op: "create".to_owned(),
            id: "forest".to_owned(),
            name: "Forest".to_owned(),
        };
        let res = edit_region(
            State(state.clone()),
            jar(Some(&cookie)).await,
            Path("m".to_owned()),
            Form(form),
        )
        .await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&res), "/regions/m");
        let saved: MapDocument = state.maps.read_json("m").expect("reloads");
        assert!(saved.regions.contains_key("forest"));
        assert_eq!(saved.regions.get("forest").expect("forest").name, "Forest");
    }

    #[tokio::test]
    async fn rename_region_persists() {
        let state = seeded_studio();
        let cookie = editor_cookie(&state);
        let form = RegionForm {
            op: "rename".to_owned(),
            id: "reg".to_owned(),
            name: "Renamed".to_owned(),
        };
        let res = edit_region(
            State(state.clone()),
            jar(Some(&cookie)).await,
            Path("m".to_owned()),
            Form(form),
        )
        .await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        let saved: MapDocument = state.maps.read_json("m").expect("reloads");
        assert_eq!(saved.regions.get("reg").expect("reg").name, "Renamed");
    }

    #[tokio::test]
    async fn create_subregion_persists() {
        let state = seeded_studio();
        let cookie = editor_cookie(&state);
        let form = SubregionForm {
            op: "create".to_owned(),
            id: "vale".to_owned(),
            name: "Vale".to_owned(),
            region: "reg".to_owned(),
        };
        let res = edit_subregion(
            State(state.clone()),
            jar(Some(&cookie)).await,
            Path("m".to_owned()),
            Form(form),
        )
        .await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        let saved: MapDocument = state.maps.read_json("m").expect("reloads");
        assert_eq!(saved.subregions.get("vale").expect("vale").region, "reg");
    }

    #[tokio::test]
    async fn rename_subregion_persists() {
        let state = seeded_studio_with_sub();
        let cookie = editor_cookie(&state);
        let form = SubregionForm {
            op: "rename".to_owned(),
            id: "vale".to_owned(),
            name: "Glen".to_owned(),
            region: String::new(),
        };
        let res = edit_subregion(
            State(state.clone()),
            jar(Some(&cookie)).await,
            Path("m".to_owned()),
            Form(form),
        )
        .await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        let saved: MapDocument = state.maps.read_json("m").expect("reloads");
        assert_eq!(saved.subregions.get("vale").expect("vale").name, "Glen");
    }

    #[tokio::test]
    async fn delete_subregion_persists() {
        let state = seeded_studio_with_sub();
        let cookie = editor_cookie(&state);
        let form = SubregionForm {
            op: "delete".to_owned(),
            id: "vale".to_owned(),
            name: String::new(),
            region: String::new(),
        };
        let res = edit_subregion(
            State(state.clone()),
            jar(Some(&cookie)).await,
            Path("m".to_owned()),
            Form(form),
        )
        .await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        let saved: MapDocument = state.maps.read_json("m").expect("reloads");
        assert!(!saved.subregions.contains_key("vale"));
    }

    // ---- S4: refusals re-render a banner and leave the store unchanged ----

    #[tokio::test]
    async fn create_duplicate_region_shows_banner_and_leaves_the_store() {
        let state = seeded_studio();
        let cookie = editor_cookie(&state);
        let form = RegionForm {
            op: "create".to_owned(),
            id: "reg".to_owned(),
            name: "Dup".to_owned(),
        };
        let (status, body) = decode(
            edit_region(
                State(state.clone()),
                jar(Some(&cookie)).await,
                Path("m".to_owned()),
                Form(form),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("already exists"), "shows the refusal: {body}");
        let saved: MapDocument = state.maps.read_json("m").expect("reloads");
        assert_eq!(
            saved.regions.get("reg").expect("reg").name,
            "Region",
            "the stored document is unchanged"
        );
        assert_eq!(saved.regions.len(), 1);
    }

    #[tokio::test]
    async fn create_subregion_with_unknown_parent_shows_banner() {
        let state = seeded_studio();
        let cookie = editor_cookie(&state);
        let form = SubregionForm {
            op: "create".to_owned(),
            id: "vale".to_owned(),
            name: "Vale".to_owned(),
            region: "ghost".to_owned(),
        };
        let (status, body) = decode(
            edit_subregion(
                State(state.clone()),
                jar(Some(&cookie)).await,
                Path("m".to_owned()),
                Form(form),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        // The single quotes around the id are HTML-escaped (&#39;) in the banner, so
        // match the quote-free phrase plus the offending id.
        assert!(
            body.contains("references unknown parent region") && body.contains("ghost"),
            "shows the refusal: {body}"
        );
        let saved: MapDocument = state.maps.read_json("m").expect("reloads");
        assert!(saved.subregions.is_empty());
    }

    #[tokio::test]
    async fn delete_referenced_region_shows_banner() {
        let state = seeded_studio();
        let cookie = editor_cookie(&state);
        let form = RegionForm {
            op: "delete".to_owned(),
            id: "reg".to_owned(),
            name: String::new(),
        };
        let (status, body) = decode(
            edit_region(
                State(state.clone()),
                jar(Some(&cookie)).await,
                Path("m".to_owned()),
                Form(form),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            body.contains("still used by room"),
            "shows the refusal: {body}"
        );
        let saved: MapDocument = state.maps.read_json("m").expect("reloads");
        assert!(saved.regions.contains_key("reg"), "unchanged");
    }

    #[tokio::test]
    async fn apply_reports_a_save_failure() {
        // A maps store rooted at a FILE makes every write fail; `apply` surfaces the
        // banner rather than redirecting, leaving the unchanged original rendered.
        let mut state = studio();
        let file = fresh_dir("apply-fail");
        std::fs::write(&file, b"not a directory").expect("seed a file");
        state.maps = oathstar_storage::FileSaveStore::new(file);
        let original: MapDocument = serde_json::from_str(SEED_DOC).expect("parses");
        let edited = original
            .create_region("forest", "Forest", &state.catalog)
            .expect("the edit itself is valid");
        let (status, body) = decode(apply(&state, "m", &original, Ok(edited))).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("failed to save the map"), "{body}");
    }

    // ---- S5: op dispatch (unknown op → 400) ----

    #[tokio::test]
    async fn unknown_region_op_is_bad_request() {
        let state = seeded_studio();
        let cookie = editor_cookie(&state);
        let form = RegionForm {
            op: "frobnicate".to_owned(),
            id: "x".to_owned(),
            name: "Y".to_owned(),
        };
        let res = edit_region(
            State(state),
            jar(Some(&cookie)).await,
            Path("m".to_owned()),
            Form(form),
        )
        .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn unknown_subregion_op_is_bad_request() {
        let state = seeded_studio();
        let cookie = editor_cookie(&state);
        let form = SubregionForm {
            op: "frobnicate".to_owned(),
            id: "x".to_owned(),
            name: "Y".to_owned(),
            region: "reg".to_owned(),
        };
        let res = edit_subregion(
            State(state),
            jar(Some(&cookie)).await,
            Path("m".to_owned()),
            Form(form),
        )
        .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    // ---- S6: per-route gating (PR-claude-gated-page-role-mutant-001) ----

    #[tokio::test]
    async fn edit_region_is_gated() {
        let form = || RegionForm {
            op: "create".to_owned(),
            id: "x".to_owned(),
            name: "Y".to_owned(),
        };
        let state = seeded_studio();
        let pid = state
            .sessions
            .create_session(principal(vec![AuthRole::Player]));
        let res = edit_region(
            State(state),
            jar(Some(&cookie_header(&pid))).await,
            Path("m".to_owned()),
            Form(form()),
        )
        .await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&res), "/login");

        let state = seeded_studio();
        let res = edit_region(
            State(state),
            jar(None).await,
            Path("m".to_owned()),
            Form(form()),
        )
        .await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&res), "/login");
    }

    #[tokio::test]
    async fn edit_subregion_is_gated() {
        let form = || SubregionForm {
            op: "create".to_owned(),
            id: "x".to_owned(),
            name: "Y".to_owned(),
            region: "reg".to_owned(),
        };
        let state = seeded_studio();
        let pid = state
            .sessions
            .create_session(principal(vec![AuthRole::Player]));
        let res = edit_subregion(
            State(state),
            jar(Some(&cookie_header(&pid))).await,
            Path("m".to_owned()),
            Form(form()),
        )
        .await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&res), "/login");

        let state = seeded_studio();
        let res = edit_subregion(
            State(state),
            jar(None).await,
            Path("m".to_owned()),
            Form(form()),
        )
        .await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&res), "/login");
    }
}
