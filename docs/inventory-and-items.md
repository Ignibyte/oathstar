# Inventory And Items

Inventory and equipment should be robust and heavily inspired by ROT-style MUD systems.

> **v1 implemented** (ticket #6): `oathstar-core::Item` is leaf data (id, name,
> description, aliases). Placement is by reference — a room's `items` (ground) or
> an entity's `inventory` (ownership). Equipment slots, weight, rarity, flags,
> effects, elemental aspects, and code-behind item behavior below are future work.

The player should manage worn equipment, carried items, item attributes, magical/elemental properties, and special item behavior.

## ROT Reference Direction

ROT should be treated as a major reference for inventory/equipment feel.

However, directly vendoring or relying on ROT source code requires a license audit first. ROT is derived from ROM/Merc/Diku lineage, and that family of codebases can carry non-commercial and attribution requirements.

Working stance:

- Use ROT as design inspiration.
- Study ROT mechanics and data shapes.
- Do not import ROT code into Oathstar until license implications are understood.
- Prefer implementing our own clean Rust/TypeScript model inspired by ROT behavior.

## Inventory Direction

Inventory should support:

- Carrying items
- Wearing equipment
- Removing equipment
- Dropping items
- Giving items
- Trading items
- Using items
- Inspecting detailed item stats
- Item flags and attributes
- Code-behind item behavior

Inventory should eventually account for:

- Carry count
- Weight
- Item size
- Containers
- Currency
- Stackable items
- Bound/quest/oath items
- Cursed or non-removable items

## Equipment Slots

Equipment should be slot-based and expressive.

Possible slots:

- Head
- Face
- Neck
- Body
- About body
- Arms
- Wrists
- Hands
- Fingers
- Waist
- Legs
- Feet
- Main hand
- Off hand
- Held
- Light
- Left ear
- Right ear
- Back
- Cloak
- Trinket
- Floating or companion relic

Slots can later support side-specific variants:

- Left finger
- Right finger
- Left wrist
- Right wrist
- Left earring
- Right earring

## Item Model

Items are entities and should use the shared entity/code-behind model.

Base item data:

- Id
- Name
- Description
- Aliases
- Attributes/tags
- Item type
- Wear slots
- Weight
- Value
- Level or tier
- Rarity
- Flags
- Effects
- Elemental aspects
- Behavior ids

Item types might include:

- Weapon
- Armor
- Jewelry
- Relic
- Consumable
- Container
- Key
- Currency
- Material
- Oath item
- Quest item
- Tool

## Item Attributes And Flags

Items should support rich flags.

Examples:

- Takeable
- Wearable
- Usable
- Stackable
- Container
- Cursed
- Bound
- Oathbound
- Fragile
- Heavy
- Unique
- Region-bound
- No-drop
- No-trade
- Hidden
- Glowing
- Humming

## Code-Behind Item Behavior

Items can reference behavior scripts through behavior ids.

Possible hooks:

- `onLook`
- `onTake`
- `onDrop`
- `onWear`
- `onRemove`
- `onUse`
- `onUseOn`
- `onGive`
- `onEquipTick`
- `onCombatRound`
- `onBreakOath`
- `onFulfillOath`

Examples:

- A cursed ring refuses removal.
- A lantern reveals hidden fixtures in dark rooms.
- A relic reacts when an oath is broken.
- A sword unlocks a combat technique after enough use.
- An earring whispers when entering a hostile region.

## Elemental Aspects

Items, actors, skills, rooms, and regions can have elemental aspects.

These can support rock-paper-scissors style combat and puzzle interactions.

Possible first aspects:

- Flame
- Frost
- Storm
- Stone
- Glass
- Iron
- Ash
- Mercy
- Memory
- Void

We should probably keep the first combat elemental set small.

Example relationship:

- Flame pressures Frost
- Frost pressures Storm
- Storm pressures Stone
- Stone pressures Flame

Oathstar-specific aspects like Mercy, Memory, Iron, Glass, or Ash can layer on top as special cases once the base system works.

## Design Guardrails

- Robust does not mean cluttered.
- Items should be meaningful, not endless junk.
- Equipment should support build identity and class emergence.
- Code-behind should be behavior ids, not raw functions embedded directly in save data.
- Elemental interactions should be readable in combat text.
- ROT should inspire the shape, but Oathstar should own its implementation.

## Open Questions

- Should inventory limits be weight-based, slot-based, count-based, or a mix?
- Should equipment have durability?
- How many equipment slots should be visible in the first vertical slice?
- Should unidentified items exist?
- Should elements be purely combat, or also puzzle/world logic?
- Should item rarity be traditional, diegetic, or both?
