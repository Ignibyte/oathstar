---
pipeline_id: 3b72977d-54a2-4134-8d40-496446309f04
title: WORK-studio-editor-canvas-shell-v1
ticket: 549049dc-d7b4-4db8-9a6d-231718d5d55b
type: work
intake: docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md
notes: WORK-studio-editor-canvas-shell-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-studio-editor-canvas-shell-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Oathstar Studio editor canvas shell v1 — an Editor-gated `/editor`
  page in `oathstar-studio` that renders a `MapDocument` on an HTML canvas and
  validates it against the #44 endpoint.
- **Scope:**
  - **In:** `GET /editor` (Editor-gated, redirect-to-login, mirroring
    `handlers::dashboard`); a canvas viewport drawing a server-embedded starter
    `MapDocument` on the current z-plane (empty / terrain floor / terrain wall /
    room / spawn); a Validate control that `fetch`-POSTs the document JSON to the
    existing `POST /editor/maps/validate` (#44) and renders the typed
    `{ok, summary | error}` result (naming the offending cell/ref on failure); a
    **new studio-owned pure JS model** (draw-plan / cell-kind / Hi-DPI size /
    aria-label / validate-result format) unit-tested via `node --test`, behind a
    **thin** canvas/fetch/DOM seam; the dashboard "Map editor" placeholder linked
    to `/editor`; docs.
  - **Out:** tile palette + paint/erase/select brushes; client-side draft state +
    a draft-save API (next slice, intake D-paint); the room inspector (intake E);
    publish-to-live-world (intake F); live-state overlays; any change to the game
    player client (`index.html`, `styles.css`, `src/client-app.js`) or the
    existing `src/client/*.js` game-canvas modules; any game server / protocol /
    engine change; a real user/content DB; accepting a client-supplied catalog.
- **Systems:** studio (sidecar handler/render/router) + ui (a new first-party
  canvas model, mirroring the #16 pure-model/glue split). **No** engine / game
  server / protocol / player-client change; reuses the #44 validate API as-is.

## Acceptance Criteria (EARS)
| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a caller without an Editor-granting session requests `GET /editor`, the studio shall redirect to `/login` and not serve the editor page. | Rust handler test |
| REQ-002 | When an Editor or Owner session requests `GET /editor`, the studio shall respond `200` with an HTML page containing a `<canvas>`, the embedded starter `MapDocument`, and a Validate control. | Rust handler test |
| REQ-003 | When given a `MapDocument`, the pure editor model shall produce a deterministic draw plan classifying each current-z cell as empty, terrain floor, terrain wall, room, or spawn. | node --test |
| REQ-004 | When given a tile display size and grid bounds, the pure editor model shall compute a Hi-DPI canvas size whose backing store is `round(cssPx · devicePixelRatio)`. | node --test |
| REQ-005 | When given a `MapDocument`, the pure editor model shall produce an aria-label naming the map title and a content summary. | node --test |
| REQ-006 | When the Validate control is activated, the editor shall POST the document JSON to `/editor/maps/validate` and render the typed result — an `ok:true` summary or an `ok:false` error that names the offending cell/reference. | browser smoke + node --test (pure result-format) |
| REQ-007 | The editor shall not modify the game player client and shall add no game-engine dependency, using only the canvas 2D API and `fetch`. | code / package review |
| REQ-008 | The editor page shall expose only the authoring `MapDocument` and the validate result (no engine/internal leakage), and identical input shall yield an identical draw plan. | node --test + review |
| REQ-009 | When complete, `cargo test --workspace`, `node --test tests/*.test.js`, and `bin/gate.sh` (FULL) shall pass. | command output |

## Locked-In Decisions
- **Editor-gated page** via `oathstar-auth` `principal_from_cookie` +
  `require_role(Editor)`, **redirect-to-login** on failure (page semantics, like
  `handlers::dashboard` — not the JSON 401/403 the API uses). An Owner session
  grants Editor.
- **Reuse the #44 endpoint** — the page validates by `fetch`-POSTing the
  `MapDocument` JSON to `POST /editor/maps/validate`. No second validate path. A
  plain HTML form cannot produce a JSON `MapDocument` body, so a small `fetch`
  glue is required.
- **Pure-model + thin-seam split (mirror #16/Decision 050):** all geometry,
  draw-plan, cell-kind, Hi-DPI sizing, aria-label, and validate-result formatting
  live in a DOM-free studio-owned JS module covered by `node --test` (counts to
  the `JS_COV_MIN=75` floor); only the `canvas.getContext('2d')` draws, `fetch`,
  and DOM wiring live in the thin browser-only seam (smoke).
- **Studio-owned JS, served by the studio**, hosted in the crate and not in the
  game client. It **must not** live in or touch `src/client-app.js`, `index.html`,
  `styles.css`, or the existing `src/client/*.js` game-canvas modules (mirror
  their patterns in new files). Exact serve mechanism finalized in Design.
- **Starter document:** a small server-side sample `MapDocument` (a few floor
  cells + 1–2 rooms + a spawn), **not** the beginner world; kept valid by
  construction so the canvas has something to draw and Validate has a body.
- **v1 = render + validate only.** Paint/palette/draft-save = the next slice
  (D-paint); the room inspector = E; publish = F.
- **Tests in-crate** (oathstar-studio handler-direct) per
  `BF-studio-cross-crate-mutation-gap-001`; JS pure-model via `node --test`; no
  unreachable defensive branches (`PR-claude-unreachable-defensive-branch-mutants-001`);
  `cargo fmt` before the gate (the #43/#44 gate:1 lesson).
- **No `oathstar-content` behavior change** — pure consumer of the #43 model and
  #44 API. (Design confirms `MapDocument: Serialize` exists; add additively only
  if missing, else embed a checked JSON literal.)

## Design Decisions (resolved Phase 2 — see notes)
1. **Route** — `GET /editor` on the sidecar (`editor::editor_page`), gated
   redirect-to-login; the dashboard "Map editor" panel becomes a link to it.
2. **JS hosting** — in-crate `crates/oathstar-studio/static/editor-canvas.js`
   (pure, DOM-free, import-free), served by `render.rs` via `include_str!` inline
   in `<script type="module">`; the DOM/canvas/fetch **glue is a `render.rs` const
   string** (never imported by node, so invisible to the import-driven gate:16
   coverage). Tested by `tests/studio-editor-canvas.test.js`.
3. **Starter doc** — a ref-free `&'static str` `MapDocument` JSON const (6×3×1;
   walled interior + an empty column; rooms `atrium`+`hall`; spawn on `atrium`),
   validated by a Rust test → `ok:true` (2 rooms, 1 region, start `atrium`).
   Ref-free ⇒ valid against any catalog; const ⇒ no runtime serialize branch.
4. **Canvas draw semantics** — current z-plane; `editorCellKind` precedence
   **spawn > room > floor > wall > empty** (floor/wall from palette `passable`);
   flat-color cells; room glyph carried on the op; aria-label = title + counts +
   spawn. **No `oathstar-content` change** (`MapDocument` already `Serialize`).

## Linked Artifacts
- Design docs: `docs/map-system.md` (Map Document Model + canvas sections); forge
  `AD-claude-map-document-model-001`, `AD-claude-studio-sidecar-001`,
  `AD-claude-studio-json-endpoint-001`; Decision 050 (first-party canvas).
- Intake doc: `docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md` (Section D)
- Ticket doc: `docs/planning/tickets/open/TICKET-45-studio-editor-canvas-shell-v1.md`
- Forge ticket: `549049dc-d7b4-4db8-9a6d-231718d5d55b` (#45)
- Forge AAR: `2e225b18-9aeb-49cb-b500-fa70e2f93fd8`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
