use core::fmt::Write as _;
use std::collections::BTreeMap;

use axum::response::Html;
use oathstar_auth::Principal;
use oathstar_content::MapDocument;

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

/// HTML-escape author-supplied text before it is interpolated into a page.
///
/// Region/sub-region ids and names are now request input (the authoring forms),
/// so every interpolation into markup or an attribute value goes through this.
/// (Deliberately a small local copy rather than reusing `oathstar_datastar`'s
/// identical escaper — the management sidecar must not depend on the player-client
/// SSE crate just to share a few lines.)
fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Percent-encode `input` for a URL query value: RFC 3986 unreserved bytes
/// (`A-Za-z0-9` and `-._~`) pass through; every other byte (incl. non-ASCII, as its
/// UTF-8 bytes) becomes `%XX`. Sub-region ids are free author text, so the deep-link
/// query value goes through this before it reaches the `href`.
fn url_query_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for &byte in input.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// A persisted authored map, summarized for the regions dashboard list.
pub struct MapSummary {
    /// The storage slot / map id (the `/regions/{id}` target).
    pub id: String,
    /// The map's human title.
    pub title: String,
    /// How many regions it declares.
    pub region_count: usize,
    /// How many sub-regions it declares.
    pub subregion_count: usize,
}

/// Room tallies per region id and per sub-region id, in one pass over a document's
/// authored room cells. The borrowed `&str` keys are valid while `doc` lives.
fn doc_room_counts(doc: &MapDocument) -> (BTreeMap<&str, usize>, BTreeMap<&str, usize>) {
    let mut regions: BTreeMap<&str, usize> = BTreeMap::new();
    let mut subregions: BTreeMap<&str, usize> = BTreeMap::new();
    for room in &doc.rooms {
        *regions.entry(room.region.as_str()).or_default() += 1;
        if let Some(sub) = &room.subregion {
            *subregions.entry(sub.as_str()).or_default() += 1;
        }
    }
    (regions, subregions)
}

/// Render the regions dashboard: the persisted authored maps, each linking to its
/// per-map region editor (ticket #51, slice 2). Authoring targets an authored
/// document, not the baked world — an empty store shows a "create one" prompt.
pub fn regions_list_page(maps: &[MapSummary]) -> Html<String> {
    let mut body = String::new();
    if maps.is_empty() {
        body.push_str(
            r#"<section class="panel"><p class="soon">No authored maps yet. Create one in the <a href="/editor">Maps editor</a>, then manage its regions here.</p></section>"#,
        );
    } else {
        for map in maps {
            let id = escape_html(&map.id);
            let title = escape_html(&map.title);
            let _ = write!(
                body,
                r#"<section class="panel"><h2>{title}</h2><p class="who">{regions} regions · {subs} sub-regions</p><p><a class="cta" href="/regions/{id}">Edit regions</a></p></section>"#,
                regions = map.region_count,
                subs = map.subregion_count,
            );
        }
    }
    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Oathstar Studio — Regions</title><style>{STUDIO_CSS}</style></head>
<body class="dashboard">
  {header}
  <main>{body}</main>
</body>
</html>"#,
        header = studio_header(Some(NavSection::Regions)),
    ))
}

/// Render the per-map region & sub-region editor (ticket #51, slice 2): each region
/// is a panel with rename + delete forms and a nested list of its sub-regions (each
/// with edit + delete), plus create-region and create-sub-region forms. Every
/// author-supplied id/name is escaped (see [`escape_html`]); `error` shows a refusal
/// banner. `map_id` is the storage slot the page was loaded under; the forms POST to
/// `/regions/{map_id}/region` and `/regions/{map_id}/subregion`.
pub fn region_editor_page(map_id: &str, doc: &MapDocument, error: Option<&str>) -> Html<String> {
    let (region_rooms, subregion_rooms) = doc_room_counts(doc);
    // Form actions key off the storage slot the page was loaded under — the
    // authoritative id that persistence and the redirect use — not the document's
    // own `id` field, so an edit always targets the map being viewed.
    let map_id = escape_html(map_id);
    let title = escape_html(&doc.title);
    let banner = error.map_or_else(String::new, |message| {
        format!(
            r#"<p class="error" role="alert">{}</p>"#,
            escape_html(message)
        )
    });

    let mut body = String::new();
    for region in doc.regions.values() {
        let rid = escape_html(&region.id);
        let rname = escape_html(&region.name);
        let rdesc = escape_html(&region.description);
        let rooms = region_rooms.get(region.id.as_str()).copied().unwrap_or(0);
        let _ = write!(
            body,
            r#"<section class="panel"><h2>{rname}</h2><p class="who">{rooms} rooms</p>
<form method="post" action="/regions/{map_id}/region" class="edit"><input type="hidden" name="op" value="edit"><input type="hidden" name="id" value="{rid}"><label>Name <input name="name" value="{rname}" aria-label="Name for region {rname}"></label> <label>Description <textarea name="description" aria-label="Description for region {rname}">{rdesc}</textarea></label> <button type="submit">Save</button></form>
<form method="post" action="/regions/{map_id}/region" class="delete"><input type="hidden" name="op" value="delete"><input type="hidden" name="id" value="{rid}"><button type="submit">Delete</button></form>
<ul>"#,
        );
        for sub in doc.subregions.values().filter(|s| s.region == region.id) {
            let sid = escape_html(&sub.id);
            let sname = escape_html(&sub.name);
            let sdesc = escape_html(&sub.description);
            let senc = url_query_encode(&sub.id);
            let srooms = subregion_rooms.get(sub.id.as_str()).copied().unwrap_or(0);
            let _ = write!(
                body,
                r#"<li><span>{sname} — {srooms} rooms</span> <a class="cta" href="/editor?map={map_id}&subregion={senc}">Open in editor</a>
<form method="post" action="/regions/{map_id}/subregion" class="edit"><input type="hidden" name="op" value="edit"><input type="hidden" name="id" value="{sid}"><label>Name <input name="name" value="{sname}" aria-label="Name for sub-region {sname}"></label> <label>Description <textarea name="description" aria-label="Description for sub-region {sname}">{sdesc}</textarea></label> <button type="submit">Save</button></form>
<form method="post" action="/regions/{map_id}/subregion" class="delete"><input type="hidden" name="op" value="delete"><input type="hidden" name="id" value="{sid}"><button type="submit">Delete</button></form></li>"#,
            );
        }
        let _ = write!(body, "</ul></section>");
    }

    let mut options = String::new();
    for region in doc.regions.values() {
        let _ = write!(
            options,
            r#"<option value="{id}">{name}</option>"#,
            id = escape_html(&region.id),
            name = escape_html(&region.name),
        );
    }

    let _ = write!(
        body,
        r#"<section class="panel"><h2>Add a region</h2>
<form method="post" action="/regions/{map_id}/region"><input type="hidden" name="op" value="create"><label>Id <input name="id" required></label> <label>Name <input name="name" required></label> <label>Description <textarea name="description"></textarea></label> <button type="submit">Create region</button></form></section>
<section class="panel"><h2>Add a sub-region</h2>
<form method="post" action="/regions/{map_id}/subregion"><input type="hidden" name="op" value="create"><label>Id <input name="id" required></label> <label>Name <input name="name" required></label> <label>Parent region <select name="region" required>{options}</select></label> <label>Description <textarea name="description"></textarea></label> <button type="submit">Create sub-region</button></form></section>"#,
    );

    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Oathstar Studio — Regions: {title}</title><style>{STUDIO_CSS}</style></head>
<body class="dashboard">
  {header}
  <main><h1>{title}</h1>{banner}{body}</main>
</body>
</html>"#,
        header = studio_header(Some(NavSection::Regions)),
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
// Reopen a saved map when the page is opened as `/editor?map=<id>`: fetch the
// persisted document and use it in place of the embedded starter. Any failure
// (missing map, network error) silently keeps the starter doc.
const savedMapId = new URLSearchParams(window.location.search).get("map");
// The sub-region to highlight, from `/editor?subregion=<id>` (#51c). Passed to the
// pure draw model, which flags that sub-region's room cells as focused.
const focusSubregion = new URLSearchParams(window.location.search).get("subregion");
if (savedMapId) {
  try {
    const response = await fetch("/editor/maps/" + encodeURIComponent(savedMapId));
    if (response.ok) {
      doc = await response.json();
    }
  } catch (_error) {
    /* keep the embedded starter document */
  }
}
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
  for (const op of editorDrawPlan(doc, { z: Z, tilePixels: TILE, focusSubregion }).ops) {
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
    if (op.focused) {
      ctx.strokeStyle = "rgb(229, 197, 111)";
      ctx.lineWidth = 3;
      ctx.strokeRect(op.x + 1.5, op.y + 1.5, op.size - 3, op.size - 3);
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

// Save the document to its storage slot (the name input == doc.id). The save
// endpoint validates the slot name and persists a draft; on success the URL gains
// `?map=<id>` so a reload reopens it. (#55 — sibling of the Validate handler.)
const nameInput = document.getElementById("map-name");
nameInput.value = doc.id;
document.getElementById("save").addEventListener("click", async () => {
  doc.id = nameInput.value.trim();
  result.textContent = "Saving…";
  delete result.dataset.ok;
  try {
    const res = await fetch("/editor/maps", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(doc),
    });
    const json = await res.json();
    const out = formatSaveResult(json);
    result.textContent = out.headline + " — " + out.detail;
    result.dataset.ok = String(out.ok);
    if (out.ok) {
      const url = new URL(window.location.href);
      url.searchParams.set("map", json.id);
      window.history.replaceState(null, "", url);
    }
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
        <p class="hint">Pick a tile, then click or drag on the map to paint. Save persists it; Validate checks it.</p>
        <div class="map-scroll"><canvas id="map" width="0" height="0"></canvas></div>
      </section>
      <section class="panel controls">
        <label>Map name <input id="map-name" name="map-name" aria-label="Map name (storage slot)"></label>
        <button id="save" type="button">Save</button>
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
    fn editor_page_wires_the_save_control() {
        // #55 / REQ-001/002/004: the Save control + its glue — a name input, a Save
        // button, the POST to /editor/maps, the formatSaveResult render, and the
        // ?map= reopen update on success.
        let html = editor_page(r#"{"id":"x","title":"T"}"#).0;
        assert!(html.contains(r#"<button id="save""#));
        assert!(html.contains(r#"<input id="map-name""#));
        assert!(html.contains(r#"getElementById("save")"#));
        assert!(html.contains(r#"fetch("/editor/maps", {"#));
        assert!(html.contains("formatSaveResult("));
        assert!(html.contains(r#"searchParams.set("map""#));
        assert!(html.contains("history.replaceState("));
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

    #[test]
    fn pages_reference_the_ui_kit_assets() {
        // ticket #50: the embedded STUDIO_CSS wires the fantasy frame + button
        // sprites, so every authenticated page references the served /ui assets.
        // Assert the specific url() forms (the CSS embeds them) — not a bare token.
        let html = dashboard_page(&owner_principal()).0;
        assert!(html.contains(r#"url("/ui/panel-frame.png")"#));
        assert!(html.contains(r#"url("/ui/button.png")"#));
    }

    // ---- ticket #51 slice 2: regions authoring render ----

    /// A map with one region (`reg`, two rooms) and one sub-region (`vale`, one room).
    const RICH_DOC: &str = r#"{
        "id":"m","title":"Mappy","tile_size":16,"width":4,"height":4,"floors":1,
        "regions":{"reg":{"id":"reg","name":"Region","description":"A test region."}},
        "subregions":{"vale":{"id":"vale","name":"Vale","region":"reg","description":"A test sub."}},
        "rooms":[
            {"x":0,"y":0,"z":0,"id":"alpha","region":"reg"},
            {"x":1,"y":0,"z":0,"id":"beta","region":"reg","subregion":"vale"}
        ]
    }"#;

    #[test]
    fn escape_html_escapes_all_metacharacters() {
        assert_eq!(
            super::escape_html("a&b<c>d\"e'f"),
            "a&amp;b&lt;c&gt;d&quot;e&#39;f"
        );
        assert_eq!(super::escape_html("plain text"), "plain text");
    }

    #[test]
    fn regions_list_page_lists_maps_with_counts_and_links() {
        use super::{regions_list_page, MapSummary};
        let maps = vec![
            MapSummary {
                id: "m1".to_owned(),
                title: "First".to_owned(),
                region_count: 2,
                subregion_count: 1,
            },
            MapSummary {
                id: "m2".to_owned(),
                title: "Second".to_owned(),
                region_count: 0,
                subregion_count: 0,
            },
        ];
        let html = regions_list_page(&maps).0;
        assert!(html.contains("<h2>First</h2>"));
        assert!(html.contains("<h2>Second</h2>"));
        assert!(html.contains(r#"<a class="cta" href="/regions/m1">Edit regions</a>"#));
        assert!(html.contains(r#"<a class="cta" href="/regions/m2">Edit regions</a>"#));
        assert!(html.contains("2 regions · 1 sub-regions"));
        assert!(html.contains("0 regions · 0 sub-regions"));
        assert!(html.contains(r#"<a href="/regions" aria-current="page">Regions</a>"#));
        assert!(!html.contains("No authored maps yet"));
    }

    #[test]
    fn regions_list_page_shows_an_empty_state() {
        let html = super::regions_list_page(&[]).0;
        assert!(html.contains("No authored maps yet"));
        assert!(html.contains(r#"<a href="/editor">Maps editor</a>"#));
        assert!(
            !html.contains(r#"class="cta""#),
            "no map cards in the empty state"
        );
    }

    #[test]
    fn region_editor_page_renders_panels_forms_counts_and_options() {
        let doc: oathstar_content::MapDocument =
            serde_json::from_str(RICH_DOC).expect("fixture parses");
        let html = super::region_editor_page("m", &doc, None).0;
        // Region panel header + its room count (alpha + beta).
        assert!(
            html.contains(r#"<section class="panel"><h2>Region</h2><p class="who">2 rooms</p>"#)
        );
        // Edit (name + description) + delete region forms POST to the slot with the id.
        assert!(html.contains(
            r#"<form method="post" action="/regions/m/region" class="edit"><input type="hidden" name="op" value="edit"><input type="hidden" name="id" value="reg"><label>Name <input name="name" value="Region" aria-label="Name for region Region"></label> <label>Description <textarea name="description" aria-label="Description for region Region">A test region.</textarea></label> <button type="submit">Save</button></form>"#
        ));
        assert!(html.contains(
            r#"<form method="post" action="/regions/m/region" class="delete"><input type="hidden" name="op" value="delete"><input type="hidden" name="id" value="reg"><button type="submit">Delete</button></form>"#
        ));
        // Sub-region nested with its own count + edit form.
        assert!(html.contains("<li><span>Vale — 1 rooms</span>"));
        assert!(html.contains(
            r#"<form method="post" action="/regions/m/subregion" class="edit"><input type="hidden" name="op" value="edit"><input type="hidden" name="id" value="vale"><label>Name <input name="name" value="Vale" aria-label="Name for sub-region Vale"></label> <label>Description <textarea name="description" aria-label="Description for sub-region Vale">A test sub.</textarea></label> <button type="submit">Save</button></form>"#
        ));
        // Create forms + the parent-region option.
        assert!(html.contains(
            r#"<form method="post" action="/regions/m/region"><input type="hidden" name="op" value="create">"#
        ));
        assert!(html.contains(
            r#"<form method="post" action="/regions/m/subregion"><input type="hidden" name="op" value="create">"#
        ));
        assert!(html.contains(r#"<option value="reg">Region</option>"#));
        assert!(html.contains(r#"<a href="/regions" aria-current="page">Regions</a>"#));
    }

    #[test]
    fn region_editor_page_renders_the_description_in_an_escaped_textarea() {
        // REQ-005: the description is shown (and edited) in the row's textarea, escaped —
        // a closing-tag injection cannot break out of the textarea.
        let doc: oathstar_content::MapDocument = serde_json::from_str(
            r#"{"id":"m","title":"T","tile_size":16,"width":1,"height":1,"floors":1,
            "regions":{"r":{"id":"r","name":"R","description":"</textarea><script>x</script>"}}}"#,
        )
        .expect("fixture parses");
        let html = super::region_editor_page("m", &doc, None).0;
        assert!(html.contains(
            r#"<textarea name="description" aria-label="Description for region R">&lt;/textarea&gt;&lt;script&gt;x&lt;/script&gt;</textarea>"#
        ));
        assert!(
            !html.contains("</textarea><script>x"),
            "the raw tag must not appear"
        );
    }

    #[test]
    fn region_editor_page_escapes_author_content() {
        let doc: oathstar_content::MapDocument = serde_json::from_str(
            r#"{"id":"m","title":"T","tile_size":16,"width":1,"height":1,"floors":1,
            "regions":{"x":{"id":"x","name":"<script>alert(1)</script>"}}}"#,
        )
        .expect("fixture parses");
        let html = super::region_editor_page("m", &doc, None).0;
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>alert(1)</script>"));
    }

    #[test]
    fn region_editor_page_shows_an_escaped_error_banner() {
        let doc: oathstar_content::MapDocument = serde_json::from_str(
            r#"{"id":"m","title":"T","tile_size":16,"width":1,"height":1,"floors":1}"#,
        )
        .expect("fixture parses");
        let html = super::region_editor_page("m", &doc, Some("bad <x> & y")).0;
        assert!(html.contains(r#"<p class="error" role="alert">bad &lt;x&gt; &amp; y</p>"#));
    }

    #[test]
    fn region_editor_page_uses_the_slot_not_the_document_id_for_actions() {
        // The page is loaded under slot `external` while the doc's own id is
        // `internal`; every form action must target the slot.
        let doc: oathstar_content::MapDocument = serde_json::from_str(
            r#"{"id":"internal","title":"T","tile_size":16,"width":1,"height":1,"floors":1,
            "regions":{"r":{"id":"r","name":"R"}}}"#,
        )
        .expect("fixture parses");
        let html = super::region_editor_page("external", &doc, None).0;
        assert!(html.contains(r#"action="/regions/external/region""#));
        assert!(
            !html.contains("/regions/internal/"),
            "actions must not use doc.id"
        );
    }

    // ---- #51c: sub-region → tile-editor deep link + focus ----

    #[test]
    fn region_editor_page_links_each_subregion_into_the_editor() {
        // REQ-001: each sub-region row links to the tile editor for its map, the
        // sub-region id percent-encoded in the query (a space → %20).
        let doc: oathstar_content::MapDocument = serde_json::from_str(
            r#"{"id":"m","title":"T","tile_size":16,"width":1,"height":1,"floors":1,
            "regions":{"reg":{"id":"reg","name":"Region"}},
            "subregions":{"a b":{"id":"a b","name":"Vale","region":"reg"}}}"#,
        )
        .expect("fixture parses");
        let html = super::region_editor_page("m", &doc, None).0;
        assert!(html
            .contains(r#"<a class="cta" href="/editor?map=m&subregion=a%20b">Open in editor</a>"#));
    }

    #[test]
    fn url_query_encode_keeps_unreserved_and_encodes_the_rest() {
        // REQ-001 helper: RFC-3986 unreserved pass through; everything else → %XX
        // (uppercase); non-ASCII as its UTF-8 bytes.
        assert_eq!(super::url_query_encode("Az09-._~"), "Az09-._~");
        assert_eq!(super::url_query_encode("a b&c=d"), "a%20b%26c%3Dd");
        assert_eq!(super::url_query_encode("é"), "%C3%A9");
        assert_eq!(super::url_query_encode(""), "");
    }

    #[test]
    fn editor_page_wires_the_subregion_focus() {
        // REQ-004: the glue reads `?subregion=`, passes `focusSubregion` to the draw
        // model, and outlines focused cells.
        let html = editor_page(r#"{"id":"x","title":"T"}"#).0;
        assert!(html.contains(r#"new URLSearchParams(window.location.search).get("subregion")"#));
        assert!(html.contains("editorDrawPlan(doc, { z: Z, tilePixels: TILE, focusSubregion })"));
        assert!(html.contains("if (op.focused) {"));
    }
}
