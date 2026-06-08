---
title: TICKET-17-spatial-awareness-proximity-query-model
status: closed
ticket: 35ec2315-2823-462b-8a41-fbf3d03b3f4e
ticket_number: 17
type: feature
created: 2026-06-07
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-spatial-awareness-proximity-model.spec.md
---

# TICKET-17-spatial-awareness-proximity-query-model

## Summary

Introduce Oathstar's Spatial Awareness / Proximity Radius foundation so entities,
items, fixtures, and dialogue targets can be discovered and interacted with by
distance within the current subregion and z-plane instead of requiring exact
tile or room co-location.

This is the "blast radius" model: a player should be able to notice an NPC two
cells west, see nearby items or people within a configured radius, and later
interact with targets according to action-specific rules.

## Why

Classic MUD rooms are mostly atomic: you are either in the same room as a thing
or you are not. Oathstar's map is evolving toward a tactical square grid where a
larger place can contain multiple walkable cells, walls, fixtures, NPCs, items,
and encounter spaces. Spatial awareness makes that map meaningful without
forcing the player to step onto the exact same tile as every target.

This foundation should support future sight radius, interaction radius,
hearing/noise radius, detection radius, aura/social radius, stealth, combat
aggro, and richer map overlays while keeping the server authoritative and the
client renderer agnostic.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the engine evaluates player surroundings, it shall query entities/items/fixtures within configurable proximity radiuses in the current subregion and z-plane, not only the exact current cell. | Rust tests |
| REQ-002 | When an entity/item is outside sight radius or blocked by reveal rules, it shall not appear in nearby/perception results. | Rust tests |
| REQ-003 | When a target is within sight radius but outside interaction radius, the state model shall be able to present it as visible but not directly talkable/takeable/interactable. | Rust/JS tests |
| REQ-004 | When a target is within interaction radius, command targeting shall be able to resolve commands such as `look <target>`, `talk <target>`, and `take <item>` without exact co-location, while keeping current exact-room behavior working. | Rust tests / smoke |
| REQ-005 | When map/state snapshots are produced, the server shall expose structured proximity/awareness data as JSON-friendly state; Rust shall not emit canvas drawing instructions. | API smoke / code review |
| REQ-006 | When implemented, tests shall cover radius math, z-plane/subregion boundaries, passability/line-of-sight placeholders, and exact vs nearby target resolution. | Rust tests |
| REQ-007 | When this ticket ships, the beginner slice's existing room title, description, navigation, event feed, and canvas map behavior shall keep working. | gate / browser smoke |

## Scope

- In: engine/domain model for spatial awareness results and action-specific
  radiuses; query logic over positioned entities/items/fixtures using
  region/subregion/z-aware coordinates; minimal command-target resolver support
  where the existing parser/engine shape allows it; tests and docs explaining
  how this relates to the canvas map and future rooms-as-areas.
- Out: full combat aggro, stealth, sound propagation, pathfinding, final
  line-of-sight blockers, dialogue trees, shops, modals, multiplayer, and DM
  controls.

## Notes

- Forge ticket: `35ec2315-2823-462b-8a41-fbf3d03b3f4e` (#17)
- Start after Ticket #16 is committed or isolated onto its own branch/worktree.
- Keep the backend server-authoritative and client/rendering agnostic.
- JSON remains the map/canvas data shape; Datastar HTML remains the component
  transport for ordinary UI.
- Do not touch generated tileset assets as part of this ticket.
- Prefer a foundation that future systems can reuse rather than hard-coding this
  only into the current Nearby panel.
