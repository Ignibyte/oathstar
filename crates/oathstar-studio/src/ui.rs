//! Static UI-kit assets for the studio theme (ticket #50).
//!
//! The mini-medieval fantasy UI sprites the studio stylesheet references, served from
//! the runtime assets dir (`OATHSTAR_ASSETS_DIR`, #59) — the same path as the arctic
//! tile sheet ([`crate::editor::arctic_sheet`]). `studio.css` uses them as
//! `border-image`s for panels and buttons.

use axum::{extract::State, response::Response};

use crate::{assets::serve_png, StudioState};

/// `GET /ui/panel-frame.png` — the wooden nine-slice panel frame (the panel/nav
/// `border-image`), served as `image/png` from the runtime assets dir.
pub async fn panel_frame(State(studio): State<StudioState>) -> Response {
    serve_png(&studio.assets_dir, "ui/panel-frame.png").await
}

/// `GET /ui/button.png` — the gold button background (the button `border-image`),
/// served as `image/png` from the runtime assets dir.
pub async fn button(State(studio): State<StudioState>) -> Response {
    serve_png(&studio.assets_dir, "ui/button.png").await
}

#[cfg(test)]
mod tests {
    use super::{button, panel_frame};
    use crate::StudioState;
    use axum::body::to_bytes;
    use axum::extract::State;
    use axum::http::{header, StatusCode};
    use std::sync::Arc;

    /// A minimal `StudioState` whose `assets_dir` is a fresh temp dir seeded with the
    /// two UI-sprite fixtures (never the real `public/`); the other fields are unused.
    fn studio_with_ui_assets(tag: &str) -> StudioState {
        let dir = std::env::temp_dir().join(format!("oathstar-studio-ui-assets-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("ui")).expect("temp ui dir");
        std::fs::write(dir.join("ui/panel-frame.png"), b"PNGFRAME").expect("frame fixture");
        std::fs::write(dir.join("ui/button.png"), b"PNGBUTTON").expect("button fixture");
        StudioState {
            sessions: oathstar_auth::SessionStore::new(),
            owner_secret: None,
            catalog: Arc::new(oathstar_content::ContentCatalog::default()),
            maps: oathstar_storage::FileSaveStore::new(
                std::env::temp_dir().join("oathstar-studio-ui-test-maps-unused"),
            ),
            assets_dir: dir,
        }
    }

    #[tokio::test]
    async fn serves_the_panel_frame() {
        // #59: the frame is read from the runtime assets dir and served as image/png.
        let res = panel_frame(State(studio_with_ui_assets("frame"))).await;
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
        assert_eq!(&body[..], b"PNGFRAME", "the on-disk frame bytes are served");
    }

    #[tokio::test]
    async fn serves_the_button() {
        let res = button(State(studio_with_ui_assets("button"))).await;
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
            b"PNGBUTTON",
            "the on-disk button bytes are served"
        );
    }
}
