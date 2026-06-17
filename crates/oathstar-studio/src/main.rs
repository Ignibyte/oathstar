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

mod config;
mod editor;
mod handlers;
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = StudioConfig::from_env();
    if config.owner_secret.is_none() {
        eprintln!(
            "oathstar-studio: OATHSTAR_OWNER_PASSWORD is unset/blank — nobody can sign in until it is set."
        );
    }

    let state = StudioState {
        sessions: SessionStore::new(),
        owner_secret: config.owner_secret,
        catalog: Arc::new(oathstar_content::beginner_catalog()?),
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
        .route("/tilesets/arctic.png", get(editor::arctic_sheet))
        .route("/ui/panel-frame.png", get(ui::panel_frame))
        .route("/ui/button.png", get(ui::button))
        .route("/regions", get(sections::regions))
        .route("/items", get(sections::items))
        .route("/enemies", get(sections::enemies))
        .route("/settings", get(sections::settings))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.addr).await?;
    println!("oathstar-studio listening on http://{}", config.addr);
    axum::serve(listener, app).await?;

    Ok(())
}
