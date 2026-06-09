---
title: TICKET-23-nearby-hostile-affordances-and-entity-inspection
status: open
ticket: pending-forge
ticket_number: 23
type: feature
created: 2026-06-09
intake:
pipeline_spec:
---

# TICKET-23-nearby-hostile-affordances-and-entity-inspection

## Summary

Make Nearby combat-aware: hostile actors should show whether they can be attacked,
provide a quick attack action when valid, and support opening an entity detail
view with authored/disclosed stats.

## Why

Ticket #22 added a working battle loop, but the player currently has to know to
type `attack <target>`. Nearby already teaches `look`, `talk`, and `take`; it
should also teach combat affordances when the server says an actor is hostile and
reachable. This keeps combat discoverable without putting movement or noisy
commands back into Intent.

The detail view also establishes a reusable pattern for inspecting mobs, NPCs,
items, fixtures, and later shops or bosses. Not every stat should be revealed:
some enemies may expose health and danger plainly, while others may hide,
obscure, or conditionally reveal stats based on skills, oaths, items, region
standing, or future perception mechanics.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a Nearby actor is hostile and attackable from the player's current position, the snapshot/view model shall mark it as attackable and expose an `attack <target>` action. | Rust/JS test |
| REQ-002 | When a Nearby actor is hostile but not attackable because it is too far away or combat is disabled in the current area, the UI shall flag that state clearly and shall not offer an enabled attack action. | Rust/JS test |
| REQ-003 | When a Nearby actor is not hostile, the UI shall not show it as an enemy and shall not offer attack unless the server marks it attackable. | Rust/JS test |
| REQ-004 | When the player clicks or activates a Nearby entity card, the client shall open a focused entity detail view without sending a command or mutating game state. | JS/browser smoke |
| REQ-005 | When an entity detail view opens for a hostile actor, it shall show only server-disclosed combat stats such as health, danger, role, or visibility state, and shall render hidden stats as unknown rather than inventing values. | Rust/JS test |
| REQ-006 | When an attack action is clicked from Nearby, it shall send the same canonical command (`attack <target>`) as typing in the command prompt, and the battle modal from ticket #22 shall open on success. | JS/browser smoke |
| REQ-007 | Existing `look`, `talk`, `take`, movement, combat modal, and Datastar event-feed behavior shall continue to pass. | gate |

## Scope

- In: additive proximity/nearby metadata for attackability and disclosed stats;
  UI flags for hostile/attackable/not-attackable; quick Attack action; entity
  detail modal or focused view; tests; docs.
- Out: full bestiary, stealth/perception skills, random stat rolls, equipment
  comparison, loot tables, aggro AI, pathfinding, and making every combatant
  attackable.

## Notes

- Forge ticket: pending. Mint this in Forge before implementation if the Forge
  connector is available to the coding agent.
- Keep the server authoritative. The client should not infer hostility from
  names, CSS classes, or local role strings unless those are explicit server
  fields in the snapshot.
- Prefer additive protocol fields on `NearbySnapshot` or a small nested
  disclosure object so older clients remain compatible.
- "Attackable" is not identical to "hostile": an enemy can be visible but too
  far away, visible in a non-combat area, hidden behind a condition, or reserved
  for another verb such as `confront`.
- The entity detail view should become the future home for richer NPC/item/fixture
  inspection, so build the first version generically rather than hardcoding only
  Ashen Stray.
