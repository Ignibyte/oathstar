//! Runtime content assets for the studio (ticket #59). The tileset + UI-sprite PNGs
//! are served from a runtime directory (`OATHSTAR_ASSETS_DIR`, default `public`) so
//! the owner can edit them without a rebuild — a deliberate reversal of the
//! embed-everything posture for CONTENT (CODE — CSS/JS — stays embedded).

use std::path::{Path, PathBuf};

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

/// The runtime assets root from a raw `OATHSTAR_ASSETS_DIR` value: the value when
/// set and non-blank (a set-but-empty var is treated as unset), otherwise `public`.
/// Split out so the blank-≠-unset rule is unit-testable (mirrors `resolve_maps_dir`).
pub fn resolve_assets_dir(raw: Option<String>) -> PathBuf {
    raw.filter(|dir| !dir.is_empty())
        .map_or_else(|| PathBuf::from("public"), PathBuf::from)
}

/// Serve the PNG read from `assets_dir/rel` at request time as `image/png`. A missing
/// or unreadable file is a logged `404` — never a panic — so a stripped deploy
/// degrades (the palette/sprite just doesn't load) instead of bricking. `rel` is a
/// fixed caller-chosen literal (never request input), so there is no traversal surface.
pub async fn serve_png(assets_dir: &Path, rel: &str) -> Response {
    match tokio::fs::read(assets_dir.join(rel)).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, "image/png")], bytes).into_response(),
        Err(error) => {
            eprintln!(
                "oathstar-studio: asset {rel} unreadable under {} ({error})",
                assets_dir.display()
            );
            (StatusCode::NOT_FOUND, "asset not found").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_assets_dir, serve_png};
    use axum::body::to_bytes;
    use axum::http::{header, StatusCode};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A fresh, empty temp dir unique to each call (never the real `public/`).
    fn fresh_dir() -> PathBuf {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("oathstar-studio-assets-test-{seq}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp assets dir");
        dir
    }

    #[test]
    fn resolve_assets_dir_treats_blank_as_unset() {
        // REQ-001: None and a blank value both default to `public`; a set value wins.
        assert_eq!(resolve_assets_dir(None), PathBuf::from("public"));
        assert_eq!(
            resolve_assets_dir(Some(String::new())),
            PathBuf::from("public")
        );
        assert_eq!(
            resolve_assets_dir(Some("custom".to_owned())),
            PathBuf::from("custom")
        );
    }

    #[tokio::test]
    async fn serve_png_serves_an_existing_file_as_image_png() {
        // REQ-002: an on-disk file is served 200 image/png with its exact bytes.
        let dir = fresh_dir();
        std::fs::write(dir.join("x.png"), b"PNGBYTES").expect("fixture");
        let res = serve_png(&dir, "x.png").await;
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
            b"PNGBYTES",
            "the on-disk bytes are served verbatim"
        );
    }

    #[tokio::test]
    async fn serve_png_404s_a_missing_file() {
        // REQ-003: a missing file is a 404 (logged, no panic) — not a 500/crash.
        let dir = fresh_dir();
        let res = serve_png(&dir, "missing.png").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}
