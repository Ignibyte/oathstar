---
pipeline_id: b88da913-3c32-46ed-9a7b-e028ee61cf8b
title: WORK-paint-editor-render-select-draw-v1
ticket: 42024e47-caba-44b7-865b-c09f4a8a941d
type: work
intake: docs/planning/intake/INTAKE-paint-system-tile-editor.md
notes: WORK-paint-editor-render-select-draw-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-paint-editor-render-select-draw-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`.

## Work Spec
- **Title:** The paintable editor (ticket #48) — render the arctic tileset,
  select a tile, and paint it onto the tilemap. Paint-system S2+S3, on the #47
  model. The owner opens `/editor` and visibly paints.
- **Scope:**
  - **In:**
    1. **arctic descriptor** — a committed tileset descriptor for
       `public/tilesets/arctic.png` (240x1624 = 30x203 @ 8px): `tileSize 8,
       columns 30, rows 203, image arctic.png` (per `docs/tileset-contract.md`).
    2. **Pure logic** (in `crates/oathstar-studio/static/editor-canvas.js`,
       node-tested — these carry the gate): `tileIndexToSourceRect`,
       `canvasPointToCell`, `paletteIndexAtPoint`, a `paintCell` layer-mutation
       helper (insert/replace, dedup), and `editorDrawPlan` extended to emit a
       sprite op per painted layer cell beneath the room/spawn overlay.
    3. **The seam** (`crates/oathstar-studio/src/render.rs` `editor_page` +
       `EDITOR_GLUE`): a tileset **palette panel** drawing the sheet (scrollable
       strip, not 6090 DOM nodes), tile **selection**, sheet-image load, **click/
       drag paint** wiring (calls the pure fns, mutates the in-memory doc,
       repaints), sprite **blit** with `imageSmoothingEnabled=false`.
  - **Out (explicit):** save/load persistence (S4); per-tile/layer/room metadata
    panels (S5); undo/redo; multi-layer management UI (paint targets one active
    layer, auto-created); runtime materialization of layers (#38).
- **Systems:** ui (studio editor) | content (arctic descriptor; model is #47)

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | `tileIndexToSourceRect(index, columns, tileSize)` shall return the sheet source rect `{ sx: (index % columns) * tileSize, sy: (index / columns | 0) * tileSize, size: tileSize }`. | node --test (non-square `columns` to kill the `*` mutant — PR-claude-non-square-capacity-fixture-001) |
| REQ-002 | `canvasPointToCell(px, py, tilePixels, width, height)` shall return the grid cell `{ x: ⌊px/tilePixels⌋, y: ⌊py/tilePixels⌋ }`, or `null` when the point lies outside the `width × height` grid. | node --test |
| REQ-003 | When the paint helper places the active `(tileset, index)` on the active layer at a cell, it shall add or replace exactly that cell (never a duplicate coordinate). | node --test |
| REQ-004 | `paletteIndexAtPoint(...)` shall resolve a point over the palette to its tile index, and `null` outside the drawn tiles. | node --test |
| REQ-005 | `editorDrawPlan` shall emit, beneath the room/spawn overlay, one sprite op per painted layer cell carrying its sheet source rect + destination position, while the existing room/spawn/glyph ops still render unchanged (additive). | node --test |
| REQ-006 | The committed arctic descriptor shall declare `tileSize 8 / columns 30 / rows 203 / image`, and a `MapDocument` whose layer references tileset `arctic` shall validate (#47 model). | node --test (descriptor) + Rust test (doc validates) |
| REQ-007 | The `/editor` page shall serve the tileset palette and the paint wiring (glue that loads the sheet, selects on palette click, paints on canvas click, and repaints). | studio render test (page carries palette/canvas/glue markers) + browser smoke |
| REQ-008 | The full gate shall stay green with mutation 100% MSI on the new pure logic. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **arctic gets a COMMITTED descriptor** (per `docs/tileset-contract.md`) — simpler
  + reusable by the game client; not deriving the grid from image dimensions.
- **The palette draws the sheet IMAGE** (a scrollable strip) — never 6090 DOM
  nodes.
- **Pure logic lives in `editor-canvas.js`** (node-tested, carries the gate +
  100% MSI); **DOM / canvas / mouse / image-load live in `EDITOR_GLUE`** (the
  smoke-/review-verified seam) — mirrors the existing #45 split.
- **Paint targets one ACTIVE layer**, auto-created if the document has none; full
  layer management is deferred.
- **Heed `PR-claude-non-square-capacity-fixture-001`** — non-square test
  tilesets so `*`-operator mutants die.
- The DOM/canvas/mouse/image-load seam is the **genuinely-uncoverable path** (no
  node tests; consistent with the existing editor glue).

## Linked Artifacts
- Design docs: `docs/tileset-contract.md`, `docs/ui-design.md` (editor),
  `docs/map-system.md` (the #47 model). Forge: `AD-claude-paint-layer-model-001`.
- Intake doc: `docs/planning/intake/INTAKE-paint-system-tile-editor.md` (S2+S3)
- Ticket doc: `docs/planning/tickets/open/TICKET-48-paint-editor-render-select-draw.md`
- Forge ticket: 42024e47-caba-44b7-865b-c09f4a8a941d (#48)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
