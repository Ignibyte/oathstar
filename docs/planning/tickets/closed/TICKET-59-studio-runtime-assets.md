# TICKET-59 — Studio serves content assets (tileset + UI sprites) from a runtime dir

- **Forge ID:** `16d0daf7-bbca-42f8-a270-7da9dde7bc5c` (#59)
- **Type:** feature · **Status:** open (pipeline `WORK-studio-runtime-assets-v1`)
- **Program:** pivot item ① of the studio-editable-world program (memory `studio-editable-world-pivot`)

## Why
The studio compiles its content into the binary (`include_bytes!` for the arctic tileset + the
UI sprites), so editing them needs a rebuild. The owner wants to edit content (tilesets first)
without touching the Rust system.

## What
Serve those PNGs from a **runtime directory**: `resolve_assets_dir` (`OATHSTAR_ASSETS_DIR`,
default `public`) + `StudioState.assets_dir` + a shared `serve_png` (read from disk per request →
`200 image/png` / logged `404`); rewire `arctic_sheet`/`panel_frame`/`button` and drop the 3
`include_bytes` consts. Code (CSS/JS) stays embedded. Fixed paths → no traversal.

## Acceptance
See `docs/planning/pipeline/active/WORK-studio-runtime-assets-v1.spec.md` (EARS REQ-001..005).

## Decision
Records `AD-claude-studio-runtime-content-assets-001` — reverses Decision 058's "no runtime asset
dir" for CONTENT only (the studio now needs `public/` at runtime; editability over self-contained).

## Out of scope
Default-world-as-editable-map (pivot ②); the STARTER_DOC; the editor UX overhaul (pivot ③);
curating the palette; the game server.
