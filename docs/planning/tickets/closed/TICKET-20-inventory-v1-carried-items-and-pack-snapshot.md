---
title: TICKET-20-inventory-v1-carried-items-and-pack-snapshot
status: closed
ticket: ec4a28af-73db-4614-b4f8-04870e420b3e
ticket_number: 20
type: feature
created: 2026-06-07
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-inventory-v1-carried-items-and-pack.spec.md
---

# TICKET-20-inventory-v1-carried-items-and-pack-snapshot

## Summary

Build the first durable inventory foundation so `take` has a real home without
jumping straight to the full ROT-style equipment system.

## Why

The game needs a robust inventory eventually, but the immediate need is smaller:
carried items, pack display, inspect, and drop. This avoids testing debt and gives
future equipment, shops, crafting, loot, and quest items a stable base.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player carries items, the engine shall store carried item ids in player/game state rather than only emitting events. | Rust test |
| REQ-002 | When a snapshot is produced, it shall expose a JSON-friendly pack/inventory list with item id, name, kind/type placeholder, and basic flags. | Rust/protocol test |
| REQ-003 | When the player runs `inventory`/`pack`/`i`, the engine shall list carried items or an honest empty state. | Rust test |
| REQ-004 | When the player looks at a carried item, the engine shall resolve it from inventory as well as nearby world contents. | Rust test |
| REQ-005 | When the player drops an item, the engine shall remove it from carried state and place it in the current room/cell, making it visible through spatial awareness. | Rust test |
| REQ-006 | Inventory operations shall reject unknown, missing, hidden, or duplicate invalid item references without corrupting state. | Rust test |
| REQ-007 | The UI Pack tab shall render server snapshot data without inventing placeholder items. | JS test / browser smoke |

## Scope

- In: carried items, inspect/list/drop, snapshot/protocol shape, Pack tab wiring,
  tests, and docs.
- Out: equipment slots, weight, stacking/quantities, shops, crafting, item use,
  persistence, rarity, and elemental stats.

## Notes

- Forge ticket: `ec4a28af-73db-4614-b4f8-04870e420b3e` (#20)
- This may absorb the minimal pack shape from #18 if #18 keeps inventory as small
  as possible.
- Do not implement the full ROT equipment model here.
