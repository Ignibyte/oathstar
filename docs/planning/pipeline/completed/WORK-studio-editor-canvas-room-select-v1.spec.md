---
pipeline_id: 1f7ec9f9-8b35-44ce-922b-a68a4c96f415
title: WORK-studio-editor-canvas-room-select-v1
ticket: 56d2dada-a7b0-46d1-8740-a61e6c481260
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-studio-editor-canvas-room-select-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-studio-editor-canvas-room-select-v1

> Pipeline spec (always-loaded contract). Per-phase detail lives in the paired `.notes.md`.

## Work Spec
- **Title:** Click a room on the `#map` canvas to open its inspector — the canvas half of the
  "both list + canvas-click" selection (pivot ③ slice c2). Reuses #63's `selectRoom`/inspector.
- **Today:** #63 gave the Rooms tab a list → inspector. The canvas only **paints** (an unconditional
  `mousedown`/`mouseup` marquee, #48). There's no way to pick a room by clicking the map.
- **Scope (in):**
  1. **`editorRoomAt(doc, x, y, z)`** (NEW, `editor-canvas.js`) → the room at that cell, or `null`
     (pure, exported, null-safe).
  2. **Glue (render.rs EDITOR_GLUE):** a `roomsTabActive()` helper (`#panel-rooms` not hidden);
     **gate the paint** so the `mousedown`/`mouseup` handlers early-return when Rooms is active (paint
     stays Tiles-only); a `canvas` **click** handler that — when Rooms is active — maps the click to a
     cell (`cellFromEvent` → `canvasPointToCell`), looks up `editorRoomAt`, and on a hit calls
     `selectRoom(room.id)` (opens that room's inspector). Empty cell → no-op.
- **Scope (out):** creating/deleting rooms (③c3); editing exits/glyph (③c3+); sub-region editing
  (③b2); palette curation; map expansion (③d).
- **Systems:** `oathstar-studio` only — `editor-canvas.js` (`editorRoomAt`) + `render.rs` EDITOR_GLUE
  (the gate + click handler) + tests + `docs/map-system.md`.

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | `editorRoomAt(doc, x, y, z)` shall return the room whose `x` **and** `y` **and** `z` all equal the arguments, and `null` when none matches (or the doc has no rooms). | node --test |
| REQ-002 | When the Rooms tab is active, a click on the canvas shall select the room at the clicked cell (opening its inspector via `selectRoom`); a click on a cell with no room shall be a no-op. | cargo test (glue wiring assert) |
| REQ-003 | Painting shall remain functional on the Tiles tab — the `mousedown`/`mouseup` paint handlers are gated (early-return when Rooms is active), not removed. | cargo test (existing #48 paint tests stay green + the guard is present) |
| REQ-004 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **The canvas tool is TAB-GATED** — Rooms tab → click selects a room; Tiles tab → click paints. The
  active tab picks the canvas tool. No new mode/tool toggle, no change to `paintRect`/the marquee math.
  (If the author later wants an explicit tool toggle, that's a separate slice — flagged, not assumed.)
- **Single-floor** — `Z = 0` (the editor const); `editorRoomAt` takes `z` for forward-compat but the
  click passes `Z`.
- **Reuse, don't rebuild** — clicking calls #63's `selectRoom(id)`; the inspector is not duplicated.
- **Empty-cell click is a no-op** (keeps the current selection) — not an error.
- **Branch off `main`** (`feaff0a`); **autonomous through commit + push + FF-merge**; stash parked.

## Linked Artifacts
- Design docs: `docs/map-system.md` (the editor section). Design re-reads.
- Plan: memory `studio-editable-world-pivot` (item ③ slice c2). Builds on #63 (`selectRoom`/inspector),
  #48 (the paint loop + `canvasPointToCell`), #61 (the tab bar / `#panel-rooms.hidden`).
- Ticket doc: `docs/planning/tickets/open/TICKET-64-studio-editor-canvas-room-select.md`
- Forge ticket: `56d2dada-a7b0-46d1-8740-a61e6c481260` (#64).

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
