---
title: TICKET-40-w1-city-forest-cave-world-oath-chain
status: open
ticket: 4a2356ea-a9b9-46b8-8977-f4b1ac2f667c
ticket_number: 40
type: feature
created: 2026-06-12
intake: docs/planning/intake/INTAKE-blank-colors-vertical-slice-city-forest-cave.md
pipeline_spec:
---

# TICKET-40-w1-city-forest-cave-world-oath-chain

## Summary

Author a city → forest → cave world in flat colors and play the whole game loop
through it: oath, shop, skills, battle, gear, boss, fulfillment — all legible on
the colored biome map. The vertical slice's finale.

## Why

Everything since #33 was systems on proven ground (markers, commerce,
equipment, flat tiles, biome rendering). W1 is the payoff: the one world that
exercises the ENTIRE loop end to end in three legible biomes, proving the slice
the blank-colors program was built toward.

## EARS Requirements (candidate — finalize at /pipeline:plan)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The world shall present three biomes (city, forest, cave) as distinct subregions, each rendering in its authored floor color on the map. | served test + visual smoke |
| REQ-002 | A player shall be able to play the full loop — swear an oath, buy a weapon, fight through the forest, loot/sell/level, equip gear, fell the cave boss, recover the objective, fulfill the oath — over the seam. | served end-to-end test |
| REQ-003 | The route shall be balanced so the loop is winnable as designed (coins fund the starter gear; the boss is winnable with the slice's gear/skills). | served test asserting the played economy + fight |

## Scope

- In: the three-biome world (rooms + subregions), enemy roster, vendor stock,
  equipment drops, the oath chain; route balance; a served full-loop test.
- Out: classes, skill trees, crafting, multiple vendors, enemy AI/movement,
  real art (ship-time swap).

## Notes

- Forge ticket: 4a2356ea-a9b9-46b8-8977-f4b1ac2f667c (#40)
- Related docs: INTAKE-blank-colors-vertical-slice-city-forest-cave (the whole
  slice + the concept render); docs/decisions.md (049–054)
- Authoring: TOML (existing modules/<world>/world.toml + rooms.toml) — no
  importer required; or via #39 if it lands first.
- Depends on: #38 (biome colors) so the three biomes read on the map.
- Promoted from intake: INTAKE-blank-colors-vertical-slice-city-forest-cave (W1)
- Active pipeline: none yet — promote via `/work` when ready
- Sequence: the finale — after #38 (and optionally #39).
