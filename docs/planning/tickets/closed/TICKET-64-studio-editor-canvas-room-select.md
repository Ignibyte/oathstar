# TICKET-64 — Click a room on the map canvas to open its inspector

- **Forge ID:** `56d2dada-a7b0-46d1-8740-a61e6c481260` (#64)
- **Type:** feature · **Status:** open (pipeline `WORK-studio-editor-canvas-room-select-v1`)
- **Program:** pivot item ③ slice c2 of the studio-editable-world program (memory `studio-editable-world-pivot`)

## Why
#63 gave the Rooms tab a list → inspector; the owner chose "both list **and** canvas-click" for
selecting a room. This adds the canvas half: click a room on the `#map` and its inspector opens.

## What
A pure `editorRoomAt(doc, x, y, z)` (the room at a cell, or null) + glue that **tab-gates** the canvas
tool: on the Rooms tab a click selects (reusing #63's `selectRoom`), on the Tiles tab it still paints
(the #48 paint loop is gated with an early-return, not changed).

## Acceptance
See `docs/planning/pipeline/active/WORK-studio-editor-canvas-room-select-v1.spec.md` (EARS REQ-001..004).

## Out of scope
Creating/deleting rooms (③c3); editing exits/glyph (③c3+); sub-region editing (③b2); palette curation;
map expansion (③d).
