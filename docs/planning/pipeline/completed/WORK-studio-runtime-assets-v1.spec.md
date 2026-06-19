---
pipeline_id: ce41d8b5-9045-45b8-88bd-e4fc54bc8d61
title: WORK-studio-runtime-assets-v1
ticket: 16d0daf7-bbca-42f8-a270-7da9dde7bc5c
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-studio-runtime-assets-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-studio-runtime-assets-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`.

## Work Spec
- **Title:** The studio serves its content PNGs from a **runtime directory** instead of
  `include_bytes!` — pivot item ① ("get content out of the binary so the owner edits without a
  rebuild"). First slice of the studio-editable-world program.
- **Today:** the studio compiles its content in — `editor.rs ARCTIC_PNG` (`/tilesets/arctic.png`
  via `arctic_sheet`), `ui.rs PANEL_FRAME_PNG`/`BUTTON_PNG` (`/ui/panel-frame.png`,
  `/ui/button.png`). All three handlers are **stateless** `Bytes::from_static(<const>)`. Editing
  `public/tilesets/arctic.png` (or a UI sprite) needs a rebuild. (`resolve_maps_dir`,
  `main.rs:93`, is the dir-resolver precedent; the maps store is already runtime.)
- **Scope:**
  - **In:**
    1. **`resolve_assets_dir(raw: Option<String>) -> PathBuf`** — env `OATHSTAR_ASSETS_DIR`,
       blank == unset, **default `public`** (mirrors `resolve_maps_dir`).
    2. **`StudioState.assets_dir: PathBuf`** — set in `main` from the env.
    3. **`serve_png(assets_dir: &Path, rel: &str) -> Response`** (shared) — `tokio::fs::read(
       assets_dir.join(rel))` → **`200 image/png`** + bytes on `Ok`; **logged `404`** on `Err`
       (no panic, no brick).
    4. **Rewire** `arctic_sheet` / `panel_frame` / `button` to `State(studio)` →
       `serve_png(&studio.assets_dir, <fixed rel>)` (`tilesets/arctic.png`, `ui/panel-frame.png`,
       `ui/button.png` — **fixed**, no caller input → no traversal). **Remove** the 3
       `include_bytes!` consts.
  - **Out (explicit):** the **default-world-as-editable-map** (pivot ②); the `STARTER_DOC`;
    the **editor UX overhaul** (pivot ③); curating the 6090-tile palette; the **game server**
    (it doesn't serve the tileset — the client gets it from Vite/`public`). **Code stays
    embedded** (`studio.css`, `editor-canvas.js`, `regions-table.js`).
- **Systems:** `oathstar-studio` only — `main.rs` (`StudioState` + `resolve_assets_dir` + wiring),
  `editor.rs`/`ui.rs` (the 3 handlers + the shared `serve_png`), + tests. **No game/engine change.**

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | `resolve_assets_dir` shall return `public` for an unset or blank `OATHSTAR_ASSETS_DIR`, and the given value otherwise. | cargo test |
| REQ-002 | `serve_png(dir, rel)` shall answer `200` with `content-type: image/png` and the file's bytes when `dir/rel` exists. | cargo test (temp dir + fixture png) |
| REQ-003 | `serve_png` shall answer `404` (logged, no panic) when `dir/rel` is missing. | cargo test (temp dir, no file) |
| REQ-004 | `GET /tilesets/arctic.png`, `/ui/panel-frame.png`, `/ui/button.png` shall each serve the corresponding file read from `<assets_dir>/<fixed path>` at request time (not an embedded constant). | cargo test (the handlers over a temp `StudioState.assets_dir` with fixtures) |
| REQ-005 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **Content → runtime, code → embedded.** The three **PNGs** move to a runtime dir; `studio.css`
  + the JS modules stay `include_str!`'d (they're program logic).
- **`resolve_assets_dir` mirrors `resolve_maps_dir`** (env `OATHSTAR_ASSETS_DIR`, blank == unset,
  default `public`); `StudioState.assets_dir: PathBuf`.
- **Shared `serve_png`** — `tokio::fs::read` per request → `200 image/png` / logged `404`. **Fixed
  relative paths** (no caller input) keep it traversal-free; a missing file is a 404, never a panic.
- **Reverses Decision 058's "self-contained, no runtime asset dir"** for CONTENT only → record an
  **`AD-claude-studio-runtime-content-assets-001`** (ASCII prose, no generics/arrays —
  `forge-mcp-ascii-only-fields`). The studio now needs `public/` present at runtime (the trade
  the owner accepts: editability over a fully self-contained binary).
- Tests use **temp assets dirs + fixture pngs**, never the real `public/` (determinism).
- **Branch off `main`** (`c187818`); **pause before push/PR/merge** (the `/goal` is complete);
  the online-first stash (`stash@{0}`) stays parked.
- **Design (Phase 2) decides:** where `resolve_assets_dir` + `serve_png` live (a new `assets`
  module vs `main.rs`/existing modules), and `PathBuf` vs `String` for `assets_dir`.

## Linked Artifacts
- Design docs: `docs/decisions.md` (Decision 058 — the reversal), `docs/map-system.md` (the
  tileset serving). Design re-reads.
- Plan: memory `studio-editable-world-pivot` (pivot item ①). Builds on #50 (UI sprites), #45/#48
  (the editor + tileset serving).
- Ticket doc: `docs/planning/tickets/open/TICKET-59-studio-runtime-assets.md`
- Forge ticket: `16d0daf7-bbca-42f8-a270-7da9dde7bc5c` (#59).

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
