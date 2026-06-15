//! Oathstar Studio — the authenticated management sidecar (ticket #42).
//!
//! A separate Rust binary from the game server, bound to loopback so the
//! "manage everything" surface is never exposed alongside the public game. It
//! shares the auth/session model with the game server via `oathstar-auth`. This
//! v1 is the shell + owner login; the map editor is a later ticket (#43).

use axum::{
    routing::{get, post},
    Router,
};
use oathstar_auth::SessionStore;

mod config;
mod handlers;
mod render;

use config::StudioConfig;

/// Shared studio state: the session registry + the configured owner secret.
#[derive(Clone)]
struct StudioState {
    sessions: SessionStore,
    owner_secret: Option<String>,
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
    };

    let app = Router::new()
        .route("/", get(handlers::dashboard))
        .route(
            "/login",
            get(handlers::login_form).post(handlers::login_submit),
        )
        .route("/logout", post(handlers::logout))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.addr).await?;
    println!("oathstar-studio listening on http://{}", config.addr);
    axum::serve(listener, app).await?;

    Ok(())
}
