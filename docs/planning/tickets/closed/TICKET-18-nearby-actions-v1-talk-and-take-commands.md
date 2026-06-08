---
title: TICKET-18-nearby-actions-v1-talk-and-take-commands
status: closed
ticket: c78c03e0-067f-4876-ba6f-17b760b5d2ff
ticket_number: 18
type: feature
created: 2026-06-07
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-nearby-actions-talk-and-take.spec.md
---

# TICKET-18-nearby-actions-v1-talk-and-take-commands

## Summary

Add the first gameplay actions that consume ticket #17's spatial-awareness model:
`talk <target>` and `take <target>`.

## Why

Spatial awareness lets the engine know what the player can see and reach. This
ticket turns that foundation into play: the player should be able to speak to an
interactable actor and pick up an interactable item without standing on the exact
same room/cell.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the parser receives `talk`/`speak` with a target, it shall produce a typed talk command preserving the target text. | Rust test |
| REQ-002 | When the parser receives `take`/`get`/`pick up` with a target, it shall produce a typed take command preserving the target text. | Rust test |
| REQ-003 | When a talk target resolves to an interactable actor through spatial awareness, the engine shall emit a response for that actor without moving the player. | Rust test |
| REQ-004 | When a talk target is visible but outside interaction radius, the engine shall report that the target is too far away to talk to. | Rust test |
| REQ-005 | When a take target resolves to an interactable item placed in the world, the engine shall move that item into player-carried state and remove it from room/nearby contents. | Rust test |
| REQ-006 | When a take target is visible but outside interaction radius, hidden, unknown, or not an item, the engine shall reject the action with a clear event and preserve state. | Rust test |
| REQ-007 | When a snapshot is produced after taking an item, the item shall appear in player inventory/pack state through a minimal additive protocol shape. | Rust/JS test |
| REQ-008 | Existing look, movement, oath, canvas map, and event feed behavior shall continue to pass. | gate |

## Scope

- In: parser commands; engine handlers using `resolve_target`/`perceive`; minimal
  carried-item/player inventory state; tests; small client update only if
  required to surface pack data.
- Out: full ROT equipment, quantities/stacks, shops, dialogue trees, item use or
  equip, combat loot, and persistence.

## Notes

- Forge ticket: `c78c03e0-067f-4876-ba6f-17b760b5d2ff` (#18)
- This should be the next implementation ticket.
- Keep the command handlers server-authoritative and reuse the spatial-awareness
  resolver rather than duplicating distance logic.
- Completed pipeline: `docs/planning/pipeline/completed/WORK-nearby-actions-talk-and-take.spec.md`
  (pipeline_id `9a6904ef-25b1-49cf-97c1-94728ddd21ca`).
