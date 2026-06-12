---
title: INTAKE-blank-colors-vertical-slice-city-forest-cave
status: candidate
created: 2026-06-12
ticket:
pipeline_spec:
---

# INTAKE-blank-colors-vertical-slice-city-forest-cave

## Problem / Opportunity

Owner direction (2026-06-12): build a **city → forest → cave** world in
pure solid colors and use it to exercise the ENTIRE game loop — visit a
shop, use skills, battle through the forest, get equipment, fell the cave
boss, complete an oath. The hero renders as a specific color and enemies
as colors too, so the whole game is legible with zero art. Concept render
produced in the flat-color spike (`/tmp/oathstar-flat-spike/concept.png`,
2026-06-12): stone city + gold shop door + teal hero, green forest with
brown road + ember enemies, violet-grey cave + red boss + yellow chest.

This is the integration target the blank-colors program builds toward
(see [[INTAKE-tileset-region-authoring-per-tile-metadata]] for the
tileset/importer side).

## Gap Analysis (engine reality, 2026-06-12)

| Loop step | Status | Notes |
|---|---|---|
| Walk city/forest/cave, map renders | EXISTS | rooms/movement/map + #32 tiles; world CONTENT is new |
| Hero as a color | EXISTS | current-room cell (spawn_marker tile + brass ring) |
| Enemies/items as colors | PLANNED | step 0 overlay markers (has_hostiles/has_items per room → ember/gold dots) |
| Skills in battle | EXISTS | guard / power strike / flee + focus economy (#25/#31), rest recovery |
| Battle / XP / levels / drops / defeat | EXISTS | #22–#26, #29–#31 — fully tested incl. served fights |
| Oath offer → swear → fulfill | EXISTS | #19/#29 — needs a NEW oath authored in the new world |
| **Shop (buy/sell)** | **MISSING** | no currency, no vendor mechanics, no buy/sell/list verbs |
| **Equipment** | **MISSING** | client gear panel is a UI placeholder; no slots/equip verbs/stat effects in engine |
| Save/load the whole run | EXISTS | #28 — new fields (coins, equipment) must join SaveData |

## Proposed Outcome

A played-end-to-end vertical slice in flat colors: start in the city,
take an oath, buy a starter weapon at the shop (commerce v1), walk the
road into the forest, fight strays with skills under the focus economy,
loot drops, sell/level up, equip better gear (equipment v1), descend into
the cave, fell the boss, recover the oath object, fulfill the oath — all
observable on the colored map (hero teal, enemies ember, loot gold).

## Candidate Tickets (sequencing decision below)

- **S0 — map entity markers** (small): `has_hostiles`/`has_items` per
  discovered room in the map snapshot + colored dot overlays client-side.
  The "enemies as colors" ask. Independent of everything else.
  **→ PROMOTED 2026-06-12:** ticket #33
  (`9fe59e9c-776a-4147-9fdf-0b43aa6fe181`), pipeline
  `WORK-map-entity-markers-v1`. This intake stays `candidate` — it
  tracks the whole slice; remaining steps promote individually.
- **S1 — commerce v1** (engine system): `coins` on PlayerState (+ save);
  authored `vendor` role with stock + prices; `list`/`buy <item>`/`sell
  <item>` verbs through the #16/#25 parser pattern; typed refusals; coin
  rewards from victories OR sellable drops (decide at design). Lands in
  Mara's Candle Shop — the shop room already exists.
- **S2 — equipment v1** (engine system): item kind `equipment` with
  authored slot (weapon/armor) + stat mods; `equip`/`unequip` verbs;
  combat math reads equipped attack/defense; gear panel wires to real
  state; SaveData round-trip. The stray's fang or a shop blade becomes
  the first weapon.
- **A / B / C** — from the tileset intake: flatten the sheet; the .tmx
  importer; per-room tile names over the wire (subregion colors live).
- **W1 — the world + oath content** (integration payoff): paint
  city/forest/cave (one .tmx per sub-region via B, or TOML if B is
  deferred); author the enemy roster, vendor stock, equipment drops, and
  the slice's oath chain; balance the route; the served end-to-end test
  plays the WHOLE loop over the seam.

## Sequencing Options

- **Systems-first (recommended):** S0 → S1 → S2 in the EXISTING
  Hollowmere world (the shop room, hostiles, drops, and the served test
  harness are already there) → A/B/C → W1. Every new system lands on
  proven ground; W1 becomes pure content + balance; visible progress
  each step.
- **World-first:** A → B → W1 (TOML-or-tmx) → S1/S2 inside it. Gets the
  three-biome geography sooner, but commerce/equipment then debut inside
  brand-new content — two unknowns stacked.

## Scope Notes

- In (candidate): the tickets above; "skills" for this slice = the
  existing battle verbs + focus (no new skill system).
- Out (candidate): classes, skill trees, crafting, multiple vendors,
  economy sinks beyond buy/sell, enemy movement/AI on the map, real art
  (ship-time sheet swap per the blank-colors program).

## Promotion Checklist

- [ ] Forge ticket created (per ticket as promoted).
- [ ] Pipeline spec/notes pair created.
- [ ] `ticket:` frontmatter updated.
- [ ] `pipeline_spec:` frontmatter updated.
- [ ] `status:` changed to `promoted`.
