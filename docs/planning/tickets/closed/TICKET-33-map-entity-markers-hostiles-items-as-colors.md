---
title: TICKET-33-map-entity-markers-hostiles-items-as-colors
status: done
ticket: 9fe59e9c-776a-4147-9fdf-0b43aa6fe181
ticket_number: 33
type: feature
created: 2026-06-12
closed: 2026-06-12
intake: docs/planning/intake/INTAKE-blank-colors-vertical-slice-city-forest-cave.md
pipeline_spec: docs/planning/pipeline/completed/WORK-map-entity-markers-v1.spec.md
---

# TICKET-33-map-entity-markers-hostiles-items-as-colors

## Summary

The map snapshot exposes server-computed per-room presence flags for
hostiles and ground items, and the client map draws colored dot overlays
on those cells — ember for enemies, gold for items — so dangers and loot
read as pure colors beside the teal hero marker. S0 of the blank-colors
vertical slice.

## Why

The vertical slice (city → forest → cave) wants the whole game legible
in solid colors: "the hero can be a specific color and enemies as well."
The hero marker exists (current-room cell); enemies and loot have no map
presence. `docs/spatial-awareness.md` explicitly reserved this overlay,
and Decision 041's principle governs it: the server computes affordances
(`Role::Hostile` placements, room item placements — both live state);
the client never infers hostility.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The map snapshot shall expose per-room hostile-presence and item-presence flags computed server-side from live placements. | Rust test |
| REQ-002 | When a room's hostile is defeated or an item is taken/dropped, the next snapshot's flags shall reflect the change. | Rust test |
| REQ-003 | The client map shall draw distinct colored markers for hostile-presence and item-presence on cells per the designed visibility rule. | node --test (plan) + smoke |
| REQ-004 | Marker rendering shall preserve existing tile, glyph, and aria behavior. | node --test |
| REQ-005 | The served fight shall show the marker lifecycle over the seam (present → defeat → cleared; drop → present → take → cleared). | server test |
| REQ-006 | Existing engine/client behavior and the gate shall continue to pass. | gate |

## Scope

- In: two additive `MapRoomSnapshot` fields computed in
  `Engine::map_snapshot`; the visibility/fog rule (design decides:
  live-state-through-fog vs discovered/visited-only); `toMapModel` cell
  flags; draw-plan marker fields per PR-oathstar-render-plan-test-002;
  canvas-seam dot drawing over tiles; deterministic engine/model/plan
  tests + the served lifecycle test; docs.
- Out: enemy movement/AI; per-entity identity on the map (presence, not
  who); item counts; NPC/vendor markers; fog-of-war redesign;
  battle-modal changes; tileset/.tmx work (other tickets).

## Notes

- Forge ticket: `9fe59e9c-776a-4147-9fdf-0b43aa6fe181` (#33)
- Related docs: `docs/spatial-awareness.md` (the reserved overlay +
  Decision 041 principle), `docs/map-system.md` (Backend Payload,
  "Enemy/event markers" under Later), `docs/decisions.md` (041, 050)
- Promoted from intake:
  `INTAKE-blank-colors-vertical-slice-city-forest-cave.md` (step S0)
- Active pipeline: `WORK-map-entity-markers-v1`
- Anchors verified at plan: `Engine::map_snapshot` (lib.rs ~2667)
  iterates `world.rooms` → `MapRoomSnapshot{…discovered, current, exits}`
  (protocol lib.rs:193); room placements are LIVE state (victory removes
  the entity placement, take/drop mutate `room.items`), so flags computed
  there are automatically current; `Entity::has_role(Role::Hostile)` is
  the existing hostility test (the #23 threat affordance uses the same);
  client cells derive in `toMapModel` (map.js), ops in `toDrawPlan`
  (canvas-map.js), dots would draw in the `drawMapCanvas` seam.
