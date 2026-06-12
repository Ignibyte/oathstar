---
title: TICKET-35-equipment-v1-slots-equip-unequip-gear-aware-combat
status: closed
ticket: 3fa9b4ab-c0c3-4b0e-80ec-965562ac5a4c
ticket_number: 35
type: feature
created: 2026-06-12
intake: docs/planning/intake/INTAKE-blank-colors-vertical-slice-city-forest-cave.md
pipeline_spec: docs/planning/pipeline/completed/WORK-equipment-v1.spec.md
---

# TICKET-35-equipment-v1-slots-equip-unequip-gear-aware-combat

## Summary

Items can be authored as equipment with a slot (weapon/armor) and stat mods
(attack/defense). `equip`/`unequip` verbs with typed refusals; combat reads
equipped attack and defense; the client Gear panel wires to real state;
SaveData round-trips; starter gear debuts in the existing world.

## Why

S2 of the blank-colors vertical slice — the last missing engine system
before the tilemap steps (A/B/C importer + W1 city/forest/cave). It also
gives commerce (#34) stock that matters: a blade worth buying.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player enters `equip <item>` naming a carried, visible equipment item, the engine shall move it into its authored slot (replacing and returning any prior occupant to the pack) and emit an inventory-channel confirmation. | cargo test |
| REQ-002 | When the player enters `equip`/`unequip` in a state that cannot satisfy it (no such carried item, item not equipment, ambiguous prefix, empty slot, mid-combat, missing target), the engine shall refuse with a typed, arm-specific message and change no state. | cargo test (one per arm) |
| REQ-003 | While a weapon with an attack mod is equipped, combat strikes by the player shall deal base strike + mod damage; while armor with a defense mod is equipped, incoming hits shall be reduced by the mod with damage floored at ≥ 0. | cargo test (boundary ±1) |
| REQ-004 | When `/state` is served, the player snapshot shall list each equipped item with its slot, and the client Gear panel shall render equipped names in their slots with remaining slots `empty` (count `filled/total` accurate). | cargo test + node --test |
| REQ-005 | When a session with equipped gear is saved and loaded, the loaded state shall reproduce the equipped slots exactly, and a pre-equipment save payload shall load with all slots empty. | cargo test (round-trip + legacy payload) |
| REQ-006 | When equipment items are authored in world content with slot/mods, the content loader shall validate and expose them, and the starter gear shall be purchasable at Mara's and equippable in a served play loop. | cargo test (served end-to-end) |

## Scope

- In: equipment item fields (slot + attack/defense mods); equip/unequip verbs
  + typed refusals; gear-aware combat math; PlayerSnapshot gear + Gear panel
  wiring; save round-trip; authored starter gear (blade at Mara's + an armor
  piece per design); composition with commerce and reveal rules.
- Out: durability, cursed/bound, proficiencies, mechanics on the other four
  panel slots, mods beyond attack/defense, pack-UI equip buttons, enemy gear,
  set bonuses.

## Notes

- Forge ticket: 3fa9b4ab-c0c3-4b0e-80ec-965562ac5a4c (#35)
- Related docs: docs/inventory-and-items.md, docs/combat.md, docs/decisions.md
- Promoted from intake: INTAKE-blank-colors-vertical-slice-city-forest-cave (S2)
- Pipeline (completed): docs/planning/pipeline/completed/WORK-equipment-v1.spec.md
