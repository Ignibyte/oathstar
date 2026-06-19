# TICKET-57 — Studio tile editor: marquee (rectangle) multi-tile paint

- **Forge ID:** `29bd6245-960c-4b60-9771-3044dcbf32f6` (#57)
- **Type:** feature · **Status:** open (pipeline `WORK-editor-marquee-paint-v1`)
- **Program:** item ④ of the owner's 2026-06-19 authoring-loop plan (memory `studio-authoring-next-phase`)

## Why
The editor paints one cell at a time (plus a basic freehand drag). The owner wants to "grab
multiple tiles at once" — drag a rectangle and fill it.

## What
Replace the freehand drag with a **marquee**: `mousedown` records the start cell, `mouseup`
fills the `start→end` rectangle (normalized) with the active tile. A single click is a 1×1
rect, so single-cell paint is preserved. New **pure** helpers in `editor-canvas.js` —
`cellsInRect(a,b)` + `paintRect(doc, layerId, a, b, z, tileRef)` (folding the immutable
`paintCell`) — are node-tested; the drag tracking + optional live preview stay in the glue seam.

## Acceptance
See `docs/planning/pipeline/active/WORK-editor-marquee-paint-v1.spec.md` (EARS REQ-001..004).

## Out of scope
Freehand-as-a-mode toggle; non-rectangular selection; copy/paste/move; undo/redo; multi-layer;
item ③ regions table.
