# TICKET-65 — Adjustable canvas zoom (render tile size slider, view-only)

- **Forge ID:** `47713464-4ffc-4267-b8bc-86c6c50778a0` (#65)
- **Type:** feature · **Status:** open (pipeline `WORK-studio-editor-canvas-zoom-v1`)
- **Program:** the studio-editable-world program (memory `studio-editable-world-pivot`)

## Why
The owner asked for "a preset where we add the tile size" and clarified it as **adjustable render
zoom**: a control that sets how big each cell draws, so a big map can be sized to fit and the
horizontal scrollbar avoided. A proper follow-on to the earlier `.canvas-panel { min-width: 0 }` fix.

## What
A pure `editorClampTilePixels(value, min, max, fallback)` + a glue change making the render tile size
adjustable (`const TILE = 40` → `let tileSize`, a `resizeCanvas()` factored from the load-time sizing,
a `#zoom` range slider + `#zoom-px` readout). **View-only** — never written to the document
(`doc.tile_size` is the source sampling size, separate from the render scale).

## Acceptance
See `docs/planning/pipeline/active/WORK-studio-editor-canvas-zoom-v1.spec.md` (EARS REQ-001..004).

## Out of scope
Auto-fit-to-screen; room create/delete + exits (③c3); sub-region editing (③b2); map expansion (③d,
which resizes the document); palette curation.
