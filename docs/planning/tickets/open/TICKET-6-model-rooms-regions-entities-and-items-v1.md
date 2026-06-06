---
title: TICKET-6-model-rooms-regions-entities-and-items-v1
status: open
ticket: f4fe738e-ae33-42c8-8dcd-185fa724afab
ticket_number: 6
type: feature
created: 2026-06-06
intake:
pipeline_spec:
---

# TICKET-6-model-rooms-regions-entities-and-items-v1

## Summary

Expand the Rust domain model toward the planned world architecture: regions,
subregions, passable rooms, entities, and items.

## Why

The core needs a stable content shape before larger systems arrive. Rooms should
trickle up into subregions and regions, passability should support collision and
future top-down rendering, and entities/items should be represented without
hardcoding beginner-module assumptions.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The world model shall represent regions and subregions that rooms can reference. | Rust test |
| REQ-002 | The room model shall expose title, description, passability, exits, and map position metadata. | Rust test |
| REQ-003 | The entity model shall allow NPCs, enemies, and special interactables to share one typed representation with role metadata. | Rust test |
| REQ-004 | The item model shall support room placement or ownership without requiring rooms to inline full item state. | Rust test |
| REQ-005 | If content references a missing region, subregion, room, entity, or item, then validation shall reject the world with a typed error. | Rust test |

## Scope

- In: protocol/domain structs, content validation, beginner TOML/schema updates as needed, tests.
- Out: Full combat, shops, advanced memory, behavior scripting, inventory equipment slots.

## Notes

- Forge ticket: `f4fe738e-ae33-42c8-8dcd-185fa724afab`
- Related docs: `docs/map-system.md`, `docs/entity-model.md`, `docs/inventory-and-items.md`, `docs/module-system.md`
- Promoted from intake:
- Active pipeline:
