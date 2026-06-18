//! Oathstar Studio — the authenticated management sidecar (ticket #42).
//!
//! A separate Rust binary from the game server, bound to loopback so the
//! "manage everything" surface is never exposed alongside the public game. It
//! shares the auth/session model with the game server via `oathstar-auth`. This
//! v1 is the shell + owner login; the map editor is a later ticket (#43).

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use oathstar_auth::SessionStore;
use oathstar_content::ContentCatalog;
use oathstar_storage::FileSaveStore;

mod config;
mod editor;
mod handlers;
mod regions;
mod render;
mod sections;
mod ui;

use config::StudioConfig;

/// Shared studio state: the session registry + the configured owner secret.
#[derive(Clone)]
struct StudioState {
    sessions: SessionStore,
    owner_secret: Option<String>,
    catalog: Arc<ContentCatalog>,
    /// Persistent store for authored map documents (root: `OATHSTAR_MAPS_DIR`).
    maps: FileSaveStore,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = StudioConfig::from_env();
    if config.owner_secret.is_none() {
        eprintln!(
            "oathstar-studio: OATHSTAR_OWNER_PASSWORD is unset/blank — nobody can sign in until it is set."
        );
    }

    // Authored map documents persist here (loopback, owner-owned).
    let maps_dir = resolve_maps_dir(std::env::var("OATHSTAR_MAPS_DIR").ok());

    let state = StudioState {
        sessions: SessionStore::new(),
        owner_secret: config.owner_secret,
        catalog: Arc::new(oathstar_content::beginner_catalog()?),
        maps: FileSaveStore::new(maps_dir),
    };

    let app = Router::new()
        .route("/", get(handlers::dashboard))
        .route(
            "/login",
            get(handlers::login_form).post(handlers::login_submit),
        )
        .route("/logout", post(handlers::logout))
        .route("/editor", get(editor::editor_page))
        .route("/editor/maps/validate", post(editor::validate))
        .route(
            "/editor/maps",
            post(editor::save_map).get(editor::list_maps),
        )
        .route("/editor/maps/{id}", get(editor::load_map))
        .route("/tilesets/arctic.png", get(editor::arctic_sheet))
        .route("/ui/panel-frame.png", get(ui::panel_frame))
        .route("/ui/button.png", get(ui::button))
        .route("/regions", get(regions::regions))
        .route("/regions/{id}", get(regions::region_editor))
        .route("/regions/{id}/region", post(regions::edit_region))
        .route("/regions/{id}/subregion", post(regions::edit_subregion))
        .route("/items", get(sections::items))
        .route("/enemies", get(sections::enemies))
        .route("/settings", get(sections::settings))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.addr).await?;
    println!("oathstar-studio listening on http://{}", config.addr);
    axum::serve(listener, app).await?;

    Ok(())
}

/// The maps-store root from a raw `OATHSTAR_MAPS_DIR` value: the value when set and
/// non-blank (a set-but-empty var is treated as unset), otherwise `maps`. Split out
/// of `main` so the blank-≠-unset rule is unit-testable.
fn resolve_maps_dir(raw: Option<String>) -> String {
    raw.filter(|dir| !dir.is_empty())
        .unwrap_or_else(|| "maps".to_owned())
}

#[cfg(test)]
mod tests {
    use super::resolve_maps_dir;

    #[test]
    fn resolve_maps_dir_treats_blank_as_unset() {
        assert_eq!(resolve_maps_dir(None), "maps");
        assert_eq!(resolve_maps_dir(Some(String::new())), "maps");
        assert_eq!(resolve_maps_dir(Some("custom".to_owned())), "custom");
    }
}
