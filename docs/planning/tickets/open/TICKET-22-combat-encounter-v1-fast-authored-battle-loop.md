---
title: TICKET-22-combat-encounter-v1-fast-authored-battle-loop
status: open
ticket: 4167bcb6-c807-4c2c-8ed6-311b7b3ae20b
ticket_number: 22
type: feature
created: 2026-06-07
intake:
pipeline_spec:
---

# TICKET-22-combat-encounter-v1-fast-authored-battle-loop

## Summary

Add the first small combat loop for quick authored encounters while preserving
boss/oath progression.

## Why

Combat will be core to Oathstar, but the first battle should be deliberately
small: deterministic, event-driven, testable, and compatible with the existing
beginner slice. This gives the game its first repeatable danger loop without
starting the full class/skills/equipment system yet.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player enters or targets a hostile combatant in a combat-enabled area, the engine shall be able to start a combat encounter state. | Rust test |
| REQ-002 | When the player uses `attack`/`strike`/`fight` during combat, the engine shall resolve deterministic damage, update HP, and emit combat-channel events. | Rust test |
| REQ-003 | When the hostile actor acts, the engine shall resolve deterministic return damage or a simple authored action. | Rust test |
| REQ-004 | When either side reaches zero HP, the engine shall end combat with a win/loss outcome and clear combat state. | Rust test |
| REQ-005 | When combat is not active or not allowed in the current region/subregion, `attack` shall refuse cleanly without damaging entities. | Rust test |
| REQ-006 | The event feed shall receive combat events in the existing typed channel/component system so future collapsible combat logs can be built on top. | Rust/JS test |
| REQ-007 | Existing oath/boss flow shall remain playable; this ticket may add a small road encounter but shall not replace the Bell-Eater boss design wholesale. | gate / server smoke |

## Scope

- In: minimal combat state, attack command, deterministic HP/damage, combat
  events, one authored beginner encounter if useful, tests, and docs.
- Out: full skills/classes, equipment bonuses, AI tactics, loot tables, death
  penalties, modals, grind economy, and multiplayer turns.

## Notes

- Forge ticket: `4167bcb6-c807-4c2c-8ed6-311b7b3ae20b` (#22)
- This should come after nearby actions, oath offering, and inventory basics so
  combat can reuse targeting, item state, and entity contracts.
- Keep the first loop deterministic so mutation and regression tests stay strong.
