//! Static UI-kit assets for the studio theme (ticket #50).
//!
//! The mini-medieval fantasy UI sprites the studio stylesheet references — embedded
//! so the loopback studio serves them without a runtime asset dir (the same pattern
//! as the embedded arctic tile sheet, [`crate::editor::arctic_sheet`]). `studio.css`
//! uses them as `border-image`s for panels and buttons.

use axum::{
    body::Bytes,
    http::header,
    response::{IntoResponse, Response},
};

/// The wooden nine-slice panel frame (cropped from the kit's Frames sheet) — the
/// `border-image` for studio panels and the nav header.
const PANEL_FRAME_PNG: &[u8] = include_bytes!("../../../public/ui/panel-frame.png");

/// The gold button background (cropped from the kit's Inputs sheet) — the
/// `border-image` for studio buttons.
const BUTTON_PNG: &[u8] = include_bytes!("../../../public/ui/button.png");

/// `GET /ui/panel-frame.png` — the embedded panel frame, served as `image/png`.
pub async fn panel_frame() -> Response {
    (
        [(header::CONTENT_TYPE, "image/png")],
        Bytes::from_static(PANEL_FRAME_PNG),
    )
        .into_response()
}

/// `GET /ui/button.png` — the embedded button sprite, served as `image/png`.
pub async fn button() -> Response {
    (
        [(header::CONTENT_TYPE, "image/png")],
        Bytes::from_static(BUTTON_PNG),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::{button, panel_frame};
    use axum::body::to_bytes;
    use axum::http::{header, StatusCode};

    #[tokio::test]
    async fn serves_the_panel_frame() {
        // ticket #50: the studio serves the embedded frame so the CSS `border-image`
        // resolves (the studio has no runtime asset dir).
        let res = panel_frame().await;
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
        assert!(!body.is_empty(), "the frame is served");
    }

    #[tokio::test]
    async fn serves_the_button() {
        let res = button().await;
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
        assert!(!body.is_empty(), "the button is served");
    }
}
