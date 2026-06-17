use core::fmt::Write as _;

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

/// The studio's top-level sections — the persistent navigation shell (ticket #49).
///
/// `Maps` is the live map editor; the rest are stub sections until their tickets
/// land (#51 regions, then items / enemies / settings).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NavSection {
    Maps,
    Regions,
    Items,
    Enemies,
    Settings,
}

impl NavSection {
    /// The label shown in the navigation.
    const fn label(self) -> &'static str {
        match self {
            Self::Maps => "Maps",
            Self::Regions => "Regions",
            Self::Items => "Items",
            Self::Enemies => "Enemies",
            Self::Settings => "Game Settings",
        }
    }

    /// The route this section links to.
    const fn href(self) -> &'static str {
        match self {
            Self::Maps => "/editor",
            Self::Regions => "/regions",
            Self::Items => "/items",
            Self::Enemies => "/enemies",
            Self::Settings => "/settings",
        }
    }
}

/// Every section, in navigation order.
const SECTIONS: [NavSection; 5] = [
    NavSection::Maps,
    NavSection::Regions,
    NavSection::Items,
    NavSection::Enemies,
    NavSection::Settings,
];

/// The persistent studio header shared by every authenticated page: the brand
/// (home link), the section navigation with `active` marked `aria-current`, and
/// the sign-out control — so the studio reads as one multi-section admin.
fn studio_header(active: Option<NavSection>) -> String {
    let mut links = String::new();
    for section in SECTIONS {
        let current = if Some(section) == active {
            r#" aria-current="page""#
        } else {
            ""
        };
        let _ = write!(
            links,
            r#"<a href="{href}"{current}>{label}</a>"#,
            href = section.href(),
            label = section.label(),
        );
    }
    format!(
        r#"<header class="studio-header">
    <nav class="studio-nav"><a class="brand" href="/">Oathstar Studio</a>{links}</nav>
    <form method="post" action="/logout"><button type="submit">Sign out</button></form>
  </header>"#
    )
}

/// Render an Editor-gated section stub — the shared header with `section` active
/// and a "Coming soon" panel. One renderer backs every not-yet-built section
/// (Regions / Items / Enemies / Game Settings, ticket #49).
pub fn section_stub_page(section: NavSection) -> Html<String> {
    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Oathstar Studio — {label}</title><style>{STUDIO_CSS}</style></head>
<body class="dashboard">
  {header}
  <main>
    <section class="panel"><h2>{label}</h2><p class="soon">Coming soon.</p></section>
  </main>
</body>
</html>"#,
        label = section.label(),
        header = studio_header(Some(section)),
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
  {header}
  <main>
    <p class="who">Signed in as <strong>{name}</strong>.</p>
    <section class="panel"><h2>World management</h2><p class="soon">Coming soon.</p></section>
    <section class="panel"><h2>Map editor</h2><p><a class="cta" href="/editor">Open the map editor</a></p></section>
  </main>
</body>
</html>"#,
        name = principal.name,
        header = studio_header(None),
    ))
}

/// The browser-only seam for the editor page — the canvas/`fetch`/DOM glue that
/// runs after the pure `editor-canvas.js` module (both live in the same inline
/// `<script type="module">`). It calls the module's exported pure functions and
/// is verified by browser smoke, not `node --test` (it is never imported).
const EDITOR_GLUE: &str = r#"
let doc = JSON.parse(document.getElementById("map-doc").textContent);
const canvas = document.getElementById("map");
const palette = document.getElementById("palette");
const TILE = 40;
const PALETTE_SCALE = 2;
const Z = 0;
let active = null;
const sheets = {};

const size = editorCanvasSize(doc, { tilePixels: TILE, devicePixelRatio: window.devicePixelRatio });
canvas.width = size.backingWidth;
canvas.height = size.backingHeight;
canvas.style.width = size.cssWidth + "px";
canvas.style.height = size.cssHeight + "px";
canvas.setAttribute("aria-label", editorAriaLabel(doc));
const ctx = canvas.getContext("2d");
ctx.scale(size.dpr, size.dpr);
ctx.imageSmoothingEnabled = false;
ctx.font = "18px ui-monospace, monospace";
ctx.textBaseline = "middle";
ctx.textAlign = "center";

// A uniform tile grid over every cell so each tile is delineated (#48 fix 4).
function drawGrid() {
  ctx.strokeStyle = "rgba(160, 170, 180, 0.45)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (let gx = 0; gx <= doc.width; gx += 1) {
    ctx.moveTo(gx * TILE + 0.5, 0);
    ctx.lineTo(gx * TILE + 0.5, doc.height * TILE);
  }
  for (let gy = 0; gy <= doc.height; gy += 1) {
    ctx.moveTo(0, gy * TILE + 0.5);
    ctx.lineTo(doc.width * TILE, gy * TILE + 0.5);
  }
  ctx.stroke();
}

function redraw() {
  ctx.clearRect(0, 0, size.cssWidth, size.cssHeight);
  for (const op of editorDrawPlan(doc, { z: Z, tilePixels: TILE }).ops) {
    if (op.sprites.length) {
      for (const s of op.sprites) {
        const img = sheets[s.tileset];
        if (img) {
          ctx.drawImage(img, s.sx, s.sy, s.sSize, s.sSize, op.x, op.y, op.size, op.size);
        }
      }
    } else {
      ctx.fillStyle = op.fill;
      ctx.fillRect(op.x, op.y, op.size, op.size);
    }
    if (op.glyph) {
      ctx.fillStyle = op.textColor;
      ctx.fillText(op.glyph, op.x + op.size / 2, op.y + op.size / 2);
    }
  }
  drawGrid();
}

// Draw the active tileset sheet into the palette, outlining the selected tile.
function drawPalette() {
  const ts = doc.tilesets && doc.tilesets[0];
  const img = ts && sheets[ts.id];
  if (!palette || !ts || !img) { return; }
  const pctx = palette.getContext("2d");
  pctx.imageSmoothingEnabled = false;
  pctx.clearRect(0, 0, palette.width, palette.height);
  pctx.drawImage(img, 0, 0, palette.width, palette.height);
  if (active && active.tileset === ts.id) {
    const pcell = ts.tile_size * PALETTE_SCALE;
    const col = active.index % ts.columns;
    const row = Math.floor(active.index / ts.columns);
    pctx.strokeStyle = "rgb(229, 197, 111)";
    pctx.lineWidth = 2;
    pctx.strokeRect(col * pcell + 1, row * pcell + 1, pcell - 2, pcell - 2);
  }
}

for (const ts of (doc.tilesets || [])) {
  const img = new Image();
  img.onload = () => {
    sheets[ts.id] = img;
    redraw();
    if (palette && doc.tilesets[0] && ts.id === doc.tilesets[0].id) {
      palette.width = ts.columns * ts.tile_size * PALETTE_SCALE;
      palette.height = ts.rows * ts.tile_size * PALETTE_SCALE;
      drawPalette();
    }
  };
  img.src = "/tilesets/" + ts.image;
}
redraw();

if (palette) {
  palette.addEventListener("click", (e) => {
    const ts = doc.tilesets[0];
    if (!ts) { return; }
    const rect = palette.getBoundingClientRect();
    const index = paletteIndexAtPoint(e.clientX - rect.left, e.clientY - rect.top, ts.columns, ts.tile_size, PALETTE_SCALE, ts.columns * ts.rows);
    if (index !== null) {
      active = { tileset: ts.id, index: index };
      const ind = document.getElementById("active-tile");
      if (ind) { ind.textContent = "Active tile: #" + index; }
      drawPalette();
    }
  });
}

// Paint the active tile onto the "ground" layer at the clicked cell. The cell
// MUST carry the active z-plane (Z): editorDrawPlan filters layer cells by z, so
// a cell with z === undefined is silently dropped and never drawn (#48 fix 2).
function paintAt(e) {
  if (!active) { return; }
  const rect = canvas.getBoundingClientRect();
  const cell = canvasPointToCell(e.clientX - rect.left, e.clientY - rect.top, TILE, doc.width, doc.height);
  if (cell) {
    doc = paintCell(doc, "ground", { x: cell.x, y: cell.y, z: Z }, active);
    redraw();
  }
}
canvas.addEventListener("mousedown", paintAt);
canvas.addEventListener("mousemove", (e) => { if (e.buttons === 1) { paintAt(e); } });

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
  {header}
  <main class="editor-main">
    <div class="editor-left">
      <section class="panel canvas-panel">
        <h2>Map editor</h2>
        <p class="hint">Pick a tile, then click or drag on the map to paint. Validate checks it.</p>
        <div class="map-scroll"><canvas id="map" width="0" height="0"></canvas></div>
      </section>
      <section class="panel controls">
        <button id="validate" type="button">Validate</button>
        <pre id="result" aria-live="polite"></pre>
      </section>
    </div>
    <section class="panel palette-panel">
      <h2>Tiles</h2>
      <p class="hint" id="active-tile">No tile selected</p>
      <div class="palette-scroll"><canvas id="palette" width="0" height="0"></canvas></div>
    </section>
  </main>
  <script type="application/json" id="map-doc">{doc_json}</script>
  <script type="module">{editor_js}{EDITOR_GLUE}</script>
</body>
</html>"#,
        editor_js = include_str!("../static/editor-canvas.js"),
        header = studio_header(Some(NavSection::Maps)),
    ))
}

#[cfg(test)]
mod tests {
    use super::{dashboard_page, editor_page, login_page, section_stub_page, NavSection};
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
                                          // ticket #49: the persistent nav with all five sections; the dashboard is
                                          // "home", so nothing is marked active.
        assert!(html.contains(r#"class="studio-nav""#));
        assert!(html.contains(r#"href="/editor""#)); // Maps
        assert!(html.contains(r#"href="/regions""#));
        assert!(html.contains(r#"href="/items""#));
        assert!(html.contains(r#"href="/enemies""#));
        assert!(html.contains(r#"href="/settings""#));
        assert!(html.contains(">Game Settings<"));
        // home marks no active section — the nav links carry no `aria-current`
        // (the string also appears in the embedded CSS selector, so assert the
        // Maps link is in its inactive form rather than scanning the whole page).
        assert!(html.contains(r#"<a href="/editor">Maps</a>"#));
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
        // ticket #48: the palette + paint wiring (pins the format! body against
        // a blank-the-page / drop-a-call mutant).
        assert!(html.contains(r#"id="palette""#));
        assert!(html.contains("/tilesets/"));
        assert!(html.contains("paletteIndexAtPoint("));
        assert!(html.contains("canvasPointToCell("));
        assert!(html.contains("paintCell("));
        assert!(html.contains("tileIndexToSourceRect("));
        // ticket #49: the editor is the Maps section — the nav is present with
        // Maps marked active and the other sections linked.
        assert!(html.contains(r#"class="studio-nav""#));
        assert!(html.contains(r#"<a href="/editor" aria-current="page">Maps</a>"#));
        assert!(html.contains(r#"href="/regions""#));
        assert!(html.contains(r#"href="/""#)); // the brand home link
    }

    #[test]
    fn section_stub_page_shows_coming_soon_and_active_nav() {
        // ticket #49 / REQ-003 + REQ-005: a stub names its section, says "Coming
        // soon", marks that section active, and leaves the others inactive.
        let html = section_stub_page(NavSection::Regions).0;
        assert!(html.contains("<h2>Regions</h2>"));
        assert!(html.contains("Coming soon."));
        assert!(html.contains(r#"<a href="/regions" aria-current="page">Regions</a>"#));
        assert!(html.contains(r#"<a href="/editor">Maps</a>"#)); // Maps not active
        assert!(html.contains(r#"class="studio-nav""#));
    }
}
