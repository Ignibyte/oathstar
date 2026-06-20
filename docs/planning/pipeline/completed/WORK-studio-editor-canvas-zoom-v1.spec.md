---
pipeline_id: 6be0ded7-124c-41c7-83d3-fd53f6624c10
title: WORK-studio-editor-canvas-zoom-v1
ticket: 47713464-4ffc-4267-b8bc-86c6c50778a0
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-studio-editor-canvas-zoom-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-studio-editor-canvas-zoom-v1

> Pipeline spec (always-loaded contract). Per-phase detail lives in the paired `.notes.md`.

## Work Spec
- **Title:** An **adjustable canvas zoom** for the editor — a slider that sets the **render tile size**
  (how big each cell draws), so a big map can be sized to fit and the horizontal scrollbar avoided.
  **View-only** (never written to the document).
- **Origin:** the owner's "a preset where we add the tile size" → clarified as *adjustable render size
  (zoom)*. Distinct from `doc.tile_size` (the SOURCE tileset sampling size), and a proper follow-on to
  the earlier `.canvas-panel { min-width: 0 }` overflow fix.
- **Today:** the render tile size is a fixed `const TILE = 40` in the EDITOR_GLUE; the canvas is sized
  once at load. No way to zoom.
- **Scope (in):**
  1. **`editorClampTilePixels(value, min, max, fallback)`** (NEW, `editor-canvas.js`) — `floor(Number)`,
     clamp to `[min,max]` when finite, else `fallback`. Pure, exported.
  2. **Glue (render.rs EDITOR_GLUE):** `const TILE = 40` → `let tileSize = 40`; replace all 6 `TILE`
     usages with `tileSize`; factor the canvas-sizing block into **`resizeCanvas()`** (called at load +
     by the zoom handler); a **zoom slider** in the controls (`#zoom` range 8–80 step 4 default 40 +
     `#zoom-px` readout, `textContent`); the `input` handler clamps via `editorClampTilePixels`, sets
     `tileSize`, then `resizeCanvas()` + `redraw()`.
  3. Update the one render test that asserts `tilePixels: TILE`.
- **Scope (out):** auto-fit-to-screen (later); ③c3 room create/delete + exits; ③b2 sub-regions; ③d map
  expansion (resizes the **document**, distinct from render zoom); palette curation.
- **Systems:** `oathstar-studio` only — `editor-canvas.js` (`editorClampTilePixels`) + `render.rs`
  EDITOR_GLUE (the slider + `resizeCanvas` + `TILE`→`tileSize`) + tests + `docs/map-system.md`.

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | `editorClampTilePixels(value, min, max, fallback)` shall floor a numeric value and clamp it to `[min, max]`, and return `fallback` for a non-finite value. | node --test |
| REQ-002 | The editor controls shall render a zoom slider (`#zoom` + `#zoom-px`) whose `input` re-sizes and redraws the canvas at the chosen tile size (`resizeCanvas()` + `redraw()` over `tileSize`). | cargo test (render assert) |
| REQ-003 | Adjusting the zoom shall not alter the saved document — Save/Validate/Activate and `doc.tile_size` are untouched (the zoom is view state). | cargo test (existing save/validate tests stay green; glue never writes `tileSize` to `doc`) |
| REQ-004 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **Zoom is VIEW state** — `tileSize` is a module variable, never written to `doc`; Save/Validate/Activate
  and `doc.tile_size` are untouched. (`doc.tile_size` = source tileset sampling; `tileSize` = render scale.)
- **`cellAt` uses `tileSize`** — so paint (#48) and room-click selection (#64) hit-test correctly at any
  zoom (pixel→cell uses the live render size).
- **Range 8–80, step 4, default 40** (40 matches the current fixed render). The `#zoom-px` readout is
  `textContent`.
- **`resizeCanvas()`** factors the load-time sizing block so the same code runs at load and on zoom.
- **Branch off `main`** (`8ec0f8b`); **autonomous through commit + push + FF-merge**; stash parked.

## Linked Artifacts
- Design docs: `docs/map-system.md` (the editor section). Design re-reads.
- Plan: memory `studio-editable-world-pivot`. Builds on `editorCanvasSize`/`editorDrawPlan` (already
  `tilePixels`-parameterized), #48 (`cellAt`/paint), #64 (`cellAt` room-click).
- Ticket doc: `docs/planning/tickets/open/TICKET-65-studio-editor-canvas-zoom.md`
- Forge ticket: `47713464-4ffc-4267-b8bc-86c6c50778a0` (#65).

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
