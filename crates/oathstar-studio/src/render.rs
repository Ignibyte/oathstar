use axum::response::Html;
use oathstar_auth::Principal;

const STUDIO_CSS: &str = include_str!("../static/studio.css");

/// Render the sign-in page.
///
/// `error` is a server-controlled constant (e.g. "Invalid credentials."); no
/// caller input is ever reflected into the page, so there is no injection surface.
pub fn login_page(error: Option<&str>) -> Html<String> {
    let banner = error.map_or_else(String::new, |message| {
        format!(r#"<p class="error" role="alert">{message}</p>"#)
    });
    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Oathstar Studio — Sign in</title><style>{STUDIO_CSS}</style></head>
<body class="login">
  <main class="card">
    <h1>Oathstar Studio</h1>
    {banner}
    <form method="post" action="/login">
      <label for="secret">Owner password</label>
      <input id="secret" name="secret" type="password" autocomplete="current-password" autofocus>
      <button type="submit">Sign in</button>
    </form>
  </main>
</body>
</html>"#
    ))
}

/// Render the authenticated dashboard shell for `principal`.
///
/// `principal` is server-constructed (the owner) in v1; when a real user store
/// lands, escape its fields before they reach the page.
pub fn dashboard_page(principal: &Principal) -> Html<String> {
    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Oathstar Studio</title><style>{STUDIO_CSS}</style></head>
<body class="dashboard">
  <header>
    <h1>Oathstar Studio</h1>
    <form method="post" action="/logout"><button type="submit">Sign out</button></form>
  </header>
  <main>
    <p class="who">Signed in as <strong>{name}</strong>.</p>
    <section class="panel"><h2>World management</h2><p class="soon">Coming soon.</p></section>
    <section class="panel"><h2>Map editor</h2><p><a class="cta" href="/editor">Open the map editor</a></p></section>
  </main>
</body>
</html>"#,
        name = principal.name
    ))
}

/// The browser-only seam for the editor page — the canvas/`fetch`/DOM glue that
/// runs after the pure `editor-canvas.js` module (both live in the same inline
/// `<script type="module">`). It calls the module's exported pure functions and
/// is verified by browser smoke, not `node --test` (it is never imported).
const EDITOR_GLUE: &str = r#"
const doc = JSON.parse(document.getElementById("map-doc").textContent);
const canvas = document.getElementById("map");
const TILE = 24;
const size = editorCanvasSize(doc, { tilePixels: TILE, devicePixelRatio: window.devicePixelRatio });
canvas.width = size.backingWidth;
canvas.height = size.backingHeight;
canvas.style.width = size.cssWidth + "px";
canvas.style.height = size.cssHeight + "px";
canvas.setAttribute("aria-label", editorAriaLabel(doc));
const ctx = canvas.getContext("2d");
ctx.scale(size.dpr, size.dpr);
ctx.font = "14px ui-monospace, monospace";
ctx.textBaseline = "middle";
ctx.textAlign = "center";
for (const op of editorDrawPlan(doc, { z: 0, tilePixels: TILE }).ops) {
  ctx.fillStyle = op.fill;
  ctx.fillRect(op.x, op.y, op.size, op.size);
  ctx.strokeStyle = op.stroke;
  ctx.strokeRect(op.x + 0.5, op.y + 0.5, op.size - 1, op.size - 1);
  if (op.glyph) {
    ctx.fillStyle = op.textColor;
    ctx.fillText(op.glyph, op.x + op.size / 2, op.y + op.size / 2);
  }
}
const result = document.getElementById("result");
document.getElementById("validate").addEventListener("click", async () => {
  result.textContent = "Validating…";
  delete result.dataset.ok;
  try {
    const res = await fetch("/editor/maps/validate", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(doc),
    });
    const out = formatValidateResult(await res.json());
    result.textContent = out.headline + " — " + out.detail;
    result.dataset.ok = String(out.ok);
  } catch (err) {
    result.textContent = "Request failed — " + err;
    result.dataset.ok = "false";
  }
});
"#;

/// Render the studio map editor canvas page (ticket #45).
///
/// `doc_json` is a server-controlled [`MapDocument`](oathstar_content::MapDocument)
/// JSON string (never caller input — see [`crate::editor::editor_page`]); it is
/// embedded verbatim into a `type="application/json"` data island that the inline
/// module reads. The inline module is the pure draw model
/// (`static/editor-canvas.js`) followed by the browser-only canvas/`fetch` glue
/// ([`EDITOR_GLUE`]). The Validate button sends the document to
/// `/editor/maps/validate` (ticket #44).
pub fn editor_page(doc_json: &str) -> Html<String> {
    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Oathstar Studio — Map editor</title><style>{STUDIO_CSS}</style></head>
<body class="editor">
  <header>
    <h1>Oathstar Studio</h1>
    <nav class="crumbs"><a href="/">Dashboard</a></nav>
    <form method="post" action="/logout"><button type="submit">Sign out</button></form>
  </header>
  <main class="editor-main">
    <section class="panel canvas-panel">
      <h2>Map editor</h2>
      <p class="hint">A starter map. Press Validate to check it against the server.</p>
      <canvas id="map" width="0" height="0"></canvas>
    </section>
    <section class="panel controls">
      <button id="validate" type="button">Validate</button>
      <pre id="result" aria-live="polite"></pre>
    </section>
  </main>
  <script type="application/json" id="map-doc">{doc_json}</script>
  <script type="module">{editor_js}{EDITOR_GLUE}</script>
</body>
</html>"#,
        editor_js = include_str!("../static/editor-canvas.js"),
    ))
}

#[cfg(test)]
mod tests {
    use super::{dashboard_page, editor_page, login_page};
    use oathstar_auth::owner_principal;

    #[test]
    fn login_page_has_form_and_embedded_css() {
        // T14/REQ-008: the login page is self-contained (form + embedded CSS).
        let html = login_page(None).0;
        assert!(html.contains(r#"action="/login""#));
        assert!(html.contains(r#"name="secret""#));
        assert!(html.contains("Oathstar Studio"));
        assert!(html.contains(".card")); // embedded CSS marker
        assert!(!html.contains(r#"role="alert""#)); // no banner without an error
    }

    #[test]
    fn login_page_renders_the_error_banner() {
        // T8/REQ-003: an invalid login re-renders with a visible error.
        let html = login_page(Some("Invalid credentials.")).0;
        assert!(html.contains("Invalid credentials."));
        assert!(html.contains(r#"class="error""#));
    }

    #[test]
    fn dashboard_page_has_shell_markers() {
        // T12/T14: the dashboard shows the owner + sign-out + the shell sections.
        let html = dashboard_page(&owner_principal()).0;
        assert!(html.contains("Sign out"));
        assert!(html.contains("Owner")); // principal.name, server-constructed
        assert!(html.contains("Map editor"));
        assert!(html.contains(".panel")); // embedded CSS marker
    }

    #[test]
    fn dashboard_links_the_editor() {
        // T6/REQ-002: the dashboard "Map editor" panel links to /editor.
        let html = dashboard_page(&owner_principal()).0;
        assert!(html.contains(r#"href="/editor""#));
        assert!(html.contains(r#"class="cta""#));
    }

    #[test]
    fn editor_page_has_canvas_doc_and_controls() {
        // T5/REQ-002/REQ-006: the editor page carries the canvas, the embedded doc
        // data island, the controls, and the glue↔module call contract (so a format!
        // mutant that blanks the body or drops a call dies).
        let html = editor_page(r#"{"id":"x","title":"T"}"#).0;
        assert!(html.contains(r#"<canvas id="map""#));
        assert!(html.contains(r#"id="map-doc""#));
        assert!(html.contains(r#"id="validate""#));
        assert!(html.contains(r#"id="result""#));
        assert!(html.contains(r#"class="editor""#));
        assert!(html.contains(r#""id":"x""#)); // the embedded data island, verbatim
        assert!(html.contains("editorDrawPlan("));
        assert!(html.contains("editorCanvasSize("));
        assert!(html.contains(r#"fetch("/editor/maps/validate""#));
    }
}
