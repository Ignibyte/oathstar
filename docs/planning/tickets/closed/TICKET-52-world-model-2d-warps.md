---
title: TICKET-52-world-model-2d-warps
status: open
ticket: 32db0cc9-0176-45a4-a599-6a6a37ff8c18
ticket_number: 52
type: feature
created: 2026-06-17
intake: docs/planning/intake/INTAKE-studio-admin-and-world-model-program.md
pipeline_spec:
---

# TICKET-52-world-model-2d-warps

## Summary

Make movement **2D** — drop Up/Down and floors-for-travel — and turn vertical
traversal into a **warp** into another region/sub-region (e.g. walking north into a
cave entrance warps you elsewhere). Tile **layers keep `z` only for visual stacking**.
Amends locked **Decision 025**.

## Why

The owner wants a real-RPG world: no MUD-style up/down, and "going north into a cave"
sends you to another region or sub-region. This is the deepest engine change, so it's
sequenced **last** (build order 4 of 4) — and it reshapes what the region dashboard
(#51) edits, so #51 gets warp authoring once this lands.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The Direction set shall no longer include Up or Down (Rust `command.rs` enum + `src/world.js`). | unit test |
| REQ-002 | When a player moves a cardinal direction into a boundary configured as a warp, the engine shall move them to the target region/sub-region room. | engine test |
| REQ-003 | The map document shall accept a warp only when its target region and room exist. | validation test |
| REQ-004 | Tile layers shall retain `z` for visual stacking while rooms/movement are 2D (no floors-for-travel). | model test |
| REQ-005 | Decision 025 shall be amended to reflect cardinal-only movement + region warps. | doc check |

## Scope

- In: remove Up/Down (`oathstar-core/src/command.rs`, `src/world.js`); update
  `move_direction` (`oathstar-core`); define + validate a warp transition
  (region+room target — net-new); MapDocument changes so rooms are 2D while layers
  keep `z`; amend Decision 025.
- Out: the full region-standing consequences; any real 3D / floor-based movement
  (removed).

## Notes

- Forge ticket: #52 `32db0cc9-0176-45a4-a599-6a6a37ff8c18`
- Build order: **4 of 4**. **Amends locked Decision 025.**
- Decisions (owner): movement/rooms 2D; `z` kept for tile-layer visuals only.
- Open questions (design): is a warp a special exit (direction → region+room) or a
  distinct "entrance" object?; where is a warp authored (tile editor vs region
  dashboard)?; migration of existing room/terrain `z` data and the `floors` field.
- `z`/`floors` is load-bearing today in `map_document.rs`, `oathstar-core` rooms, and
  the studio editor — plan the migration carefully.
- Promoted from intake: yes. Active pipeline: not yet.
