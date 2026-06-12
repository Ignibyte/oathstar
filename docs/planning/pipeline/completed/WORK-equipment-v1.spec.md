---
pipeline_id: 27bd789d-d143-43d6-8e60-012d8d00b06e
title: WORK-equipment-v1
ticket: 3fa9b4ab-c0c3-4b0e-80ec-965562ac5a4c
type: work
intake: docs/planning/intake/INTAKE-blank-colors-vertical-slice-city-forest-cave.md
notes: WORK-equipment-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-equipment-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Equipment v1 — slots, equip/unequip, gear-aware combat (ticket #35)
- **Scope:**
  - **In:** items authored as equipment with a slot (`weapon` | `armor`) and
    stat mods (attack / defense); `equip <item>` / `unequip <slot|item>` verbs
    via the existing strict parser pattern with typed refusals for every arm;
    combat math reads equipped attack (outgoing strike) and defense (incoming
    reduction, floored ≥ 0 damage); PlayerSnapshot exposes equipped gear and
    the client Gear panel wires its six placeholder slots to real state;
    SaveData round-trips equipment serde-additively (no format bump); starter
    gear authored into the existing world (a blade in Mara's stock + design
    decides the armor piece); equipment composes with commerce (values,
    buy/sell) and reveal rules (hidden gear never listed/equippable).
  - **Out:** durability, cursed/bound/no-remove mechanics, proficiencies,
    off-hand/jewelry/trinket slots carrying mechanics (they render empty),
    stat mods beyond attack/defense, pack-UI equip buttons (text verbs only),
    enemy equipment, set bonuses.
- **Systems:** engine (oathstar-core) | parser | combat | inventory | protocol | server | content | ui (gear panel)

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player enters `equip <item>` naming a carried, visible equipment item, the engine shall move it into its authored slot (replacing and returning any prior occupant to the pack) and emit an inventory-channel confirmation. | cargo test |
| REQ-002 | When the player enters `equip`/`unequip` in a state that cannot satisfy it (no such carried item, item not equipment, ambiguous prefix, empty slot, mid-combat, missing target), the engine shall refuse with a typed, arm-specific message and change no state. | cargo test (one per arm) |
| REQ-003 | While a weapon with an attack mod is equipped, combat strikes by the player shall deal base strike + mod damage; while armor with a defense mod is equipped, incoming hits shall be reduced by the mod with damage floored at ≥ 0. | cargo test (boundary ±1) |
| REQ-004 | When `/state` is served, the player snapshot shall list each equipped item with its slot, and the client Gear panel shall render equipped names in their slots with remaining slots `empty` (count `filled/total` accurate). | cargo test + node --test |
| REQ-005 | When a session with equipped gear is saved and loaded, the loaded state shall reproduce the equipped slots exactly, and a pre-equipment save payload shall load with all slots empty. | cargo test (round-trip + legacy payload) |
| REQ-006 | When equipment items are authored in world content with slot/mods, the content loader shall validate and expose them, and the starter gear shall be purchasable at Mara's and equippable in a served play loop. | cargo test (served end-to-end) |

## Locked-In Decisions
- Two active slot kinds only (`weapon`, `armor`); the Gear panel's other four
  slots stay visually present but always empty in v1.
- Serde-additive wire + save shape (`#[serde(default)]`; omit-when-empty);
  `SAVE_FORMAT_VERSION` stays 2.
- Equipment composes with commerce: gear items carry `value` and trade
  through the existing shop/buy/sell verbs unchanged.
- Reveal rule applies to every new projection (PR-claude-reveal-rules-on-
  every-projection-001): hidden items are not equippable and equipped gear
  honors existing visibility rules in all listings.
- Combat stays deterministic; no RNG added.
- Lean is the bar: this is the last engine system before the tilemap steps
  (A/B/C + W1) — defer anything not needed by the slice.

## Linked Artifacts
- Design docs: docs/inventory-and-items.md, docs/combat.md, docs/decisions.md
- Intake doc: docs/planning/intake/INTAKE-blank-colors-vertical-slice-city-forest-cave.md (S2)
- Ticket doc: docs/planning/tickets/open/TICKET-35-equipment-v1-slots-equip-unequip-gear-aware-combat.md
- Forge ticket: 3fa9b4ab-c0c3-4b0e-80ec-965562ac5a4c (#35)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
