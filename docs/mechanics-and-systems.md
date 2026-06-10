# Oathstar Mechanics And Systems

## Current Mechanical Direction

Oathstar should be a turn-based command game with RPG structure, MUD-style world navigation, visible state panels, and a central oath system that ties narrative choices to mechanical consequences.

The current prototype proves the basic shell:

- Command input
- Room descriptions
- Directional movement
- Inventory
- NPC interaction
- Item use
- Combat
- Quest/oath tracking
- Save/load
- Visible map/status panels

The next design phase should decide which of these stay simple and which become signature systems.

## Player Input

### Command Parser

The parser should support natural-feeling verb phrases without pretending to understand everything.

This is locked in [Decision 002](./decisions.md#decision-002-use-a-forgiving-symbolic-parser).

> **Status — v1 implemented** (ticket #5): `oathstar-core::command::parse(input) -> Command`,
> a pure, deterministic typed parser. v1 covers movement aliases (`n`…`down`,
> `go <dir>` — exactly one direction), `look`/`examine`/`l`/`x` with an optional
> target (target text preserved), `help`/`h`, and a typed unknown-command path
> that mutates no state. The richer Decision-002 grammar (`verb target on|to
> target`, `ask … about …`) and the inventory/social/oath/conflict verbs below
> are not yet wired — future tickets.

Core command categories:

- Movement: north, south, east, west, up, down, go north
- Inspection: look, examine lantern, inspect gate
- Inventory: take, drop, inventory, use
- Social: talk, ask, give, offer, swear
- Oath actions: vow, bind, break, remember, forgive
- Conflict: attack, defend, spare, ward
- Utility: help, save, load, rest, map, clear

Parser principles:

- Common aliases should work.
- Basic `look <mob>`, `look <item>`, and `look <room feature>` phrasing should work.
- Failed commands should suggest useful alternatives.
- The game should prefer clear affordances over guess-the-verb puzzles.
- Important puzzles should support more than one reasonable wording.
- The UI can provide command chips, but typing should remain first-class.

Supported command shapes:

- `verb`
- `verb target`
- `verb target on target`
- `verb target to target`
- `ask target about topic`
- direction shortcuts

Open design question:

Should oath-related verbs be explicit commands like `swear mercy` and `break vow`, or should oaths behave more like inventory/status objects used in context?

## Rooms And World Model

Room structure is locked in [Decision 003](./decisions.md#decision-003-rooms-are-described-entity-containers-with-optional-attributes).

> **Status — v1 implemented** (ticket #6): the world *data model* lives in
> `oathstar-core` — `RegionDefinition`/`SubregionDefinition` registries on
> `WorldDefinition`; `RoomDefinition` (title/description/passability/exits/x,y,z/
> glyph + `entities`/`items` id-lists); `Entity` (Actor/Fixture + role tags +
> `inventory`); `Item` (leaf data). Placement is by reference (containers hold
> ids — rooms never inline item state). `WorldDefinition::validate` rejects any
> dangling region/subregion/room/entity/item reference with a typed error;
> `oathstar-content` loads it all from module TOML. Behaviors, role contracts,
> item flags/slots/elements, and player inventory are future tickets.

### Rooms

Rooms are the primary units of space and interaction.

Each room should support:

- Id
- Region
- Optional subregion
- Title
- Description
- Exits
- Entities
- Optional attributes/tags when mechanically useful

Optional room features:

- First-visit text
- State-based description changes
- Local scripts/hooks for special behavior

### Regions And Subregions

Every room belongs to a region, and may belong to a subregion. This lets room-level play trickle up into broader systems.

Region/subregion rules can eventually support:

- Local oath laws
- Region-specific hazards
- Encounter behavior
- Ambient text or music
- Faction control
- Travel restrictions
- UI grouping and map labels

Room entities can include:

- Items
- NPCs
- Mobs/enemies
- Hazards
- Interactive fixtures/features

## Entity Model

The entity model is locked in [Decision 004](./decisions.md#decision-004-entities-use-shared-data-with-role-contracts-and-code-behind-behaviors). The working detail lives in [Entity Model](./entity-model.md).

> **Status — v1 implemented** (ticket #6): `oathstar-core::Entity` is the shared
> shape (kind `Actor`/`Fixture`, free-form `roles` tags, `inventory` of owned item
> ids) — an "enemy" is an Actor with a `combatant` role, not a separate type.
> **Typed role contracts are validated at the world construction boundary
> (ticket #21)** — see [Entity Model → Role Contracts](./entity-model.md#role-contracts-v1-ticket-21).
> The code-behind behavior dispatch described below remains future work.

Core direction:

- Rooms contain entities.
- Entities share a common base shape.
- NPCs and enemies are both actors.
- Actors gain capabilities through roles.
- Roles have contracts that require specific metadata.
- Special behavior is attached through code-behind behavior ids and registered hooks.

This lets a shopkeeper, hostile guard, oath witness, and ordinary NPC all share the same actor foundation while still behaving differently.

### Exits

Exits should usually be stable and readable. Hidden exits are allowed, but the game should teach the player what revealed them.

Possible exit types:

- Open
- Locked
- Oath-locked
- Hazardous
- One-way
- State-dependent

### Map

The visible map should help orientation without solving every mystery.

Map direction is locked in [Decision 025](./decisions.md#decision-025-the-map-uses-a-square-grid-with-cardinal-directions-plus-updown). More detail lives in [Map And Minimap System](./map-system.md).

Recommended behavior:

- Show visited rooms.
- Highlight current room.
- Show known exits when discovered.
- Keep unknown rooms blank.
- Allow district labels.
- Use a square grid.
- Support north, south, east, west, up, and down.
- Avoid diagonal directions in core navigation.
- Track passable/non-passable cells for collision.
- Receive renderer-agnostic JSON map data from the server.

## Character State

### Baseline Stats

The current prototype uses:

- Health
- Focus

These are good starting stats because they are readable and not overbuilt.

Possible future stats:

- Health: physical endurance
- Focus: mental/spiritual effort used for rituals, wards, or difficult commands
- Oathweight: burden from active promises
- Standing: reputation with city remnants or factions

Recommendation:

Keep Health and Focus for the first vertical slice. Add Oathweight only if oath commitments need a clear systemic cost.

## Oath System

This should become the game's defining mechanic.

Working decision:

Oaths are quests++.

This is locked in [Decision 001](./decisions.md#decision-001-oaths-are-quests).

A normal quest asks the player to complete an objective. An oath asks the player to complete an objective while bound by a promise. The important extra ingredients are the witness, the restriction, and the consequences for keeping or breaking the promise.

### What Is An Oath?

An oath is a promise with world-state consequences. It may be spoken by the player, imposed by an NPC, inherited from an object, or discovered as part of the city.

The first oath lifecycle is locked in [Decision 005](./decisions.md#decision-005-oaths-use-a-simple-four-state-lifecycle).

An oath should have:

- Name
- Text
- Witness
- Condition to keep it
- Condition to break it
- Reward or unlock
- Cost or burden
- Consequence if broken

First-version oath states:

- `available`
- `active`
- `fulfilled`
- `broken`

### Possible Oath Types

- Memory: preserve, recover, or reveal truth
- Mercy: spare, forgive, release, heal
- Flame: act, fight, endure, sacrifice
- Silence: withhold information or refuse a command
- Debt: owe or collect a favor
- Witness: bind an event into law

### How Oaths Could Play

Option A: Oaths As Quest Contracts

The player accepts clear oath objectives. Breaking or completing them changes rewards, endings, and NPC behavior.

Pros:

- Easy to understand
- Strong quest structure
- Simple to implement

Cons:

- Risks feeling like ordinary quest tracking with fancy names

Option B: Oaths As Active Loadout

The player can carry only a few active oaths. Active oaths grant verbs or passive effects but impose restrictions.

Pros:

- Mechanically distinctive
- Creates interesting choices

Cons:

- More balancing work
- Needs excellent UI clarity

Option C: Oaths As World Logic

The city has oath laws. The player learns and manipulates them like puzzle rules.

Pros:

- Deeply thematic
- Strong puzzle potential

Cons:

- Harder to teach
- Risks becoming opaque

Recommended first version:

Blend A and C. Treat oaths as explicit objectives, but make each one alter room logic or available command outcomes. Add loadout-style limits only after the base system feels good.

### Implementation Horizon

Now:

- Oaths are authored quests with explicit accept/complete/break conditions.
- Each oath has a clear witness.
- The UI shows active, kept, and broken oaths.
- The parser supports simple oath commands such as `swear oath`, `swear mercy`, and `break oath`.
- Keeping or breaking an oath changes at least one world state.

Later:

- Oaths can conflict with each other.
- Some oaths impose restrictions such as do not kill, do not lie, do not enter a place, or do not drop an item.
- NPCs react differently to kept and broken oaths.
- Oaths can grant limited verbs, protections, or routes.

Later Later:

- Oaths become a deeper language of play.
- The player can compose or modify certain vows.
- District laws, enemy bindings, and endings are shaped by the player's history of promises.

## Inventory And Items

Items should be few, memorable, and stateful.

Inventory and item direction is locked in [Decision 013](./decisions.md#decision-013-inventory-and-items-are-rot-inspired-slot-based-and-behavior-driven). More detail lives in [Inventory And Items](./inventory-and-items.md).

Item categories:

- Tools: lantern, bell, key, prism
- Tokens: letters, seals, warrants, names
- Relics: vow shards, oathstones, saint bones
- Consumables: rare, likely not a main focus
- Burdens: items that cannot be dropped without consequence

Item principles:

- Every carried item should have a reason to exist.
- Important items should support examination and contextual use.
- Some items can transform after use.
- Items can act as proof, payment, witness, or ritual component.
- Equipment is slot-based and ROT-inspired.
- Items can have rich attributes, flags, elemental aspects, and code-behind behavior.

Avoid:

- Large junk inventories
- Generic loot
- Equipment treadmills

## NPCs And Dialogue

NPCs should feel like remnants of a civic/magical system, not quest vending machines.

NPC memory is locked in [Decision 006](./decisions.md#decision-006-npcs-use-basic-memory-with-advanced-memory-for-special-characters).

Interaction modes:

- talk NPC
- ask NPC about topic
- give item
- offer oath
- accuse NPC
- forgive/spare/release NPC

NPC state should track:

- First meeting
- Known topics
- Given items
- Trust or suspicion
- Oaths made to or witnessed by the NPC
- Whether their local problem is resolved

For the first vertical slice, dialogue can remain command-based instead of full dialogue trees.

Memory direction:

- All actors can use basic memory.
- Special characters can opt into advanced memory through roles or behavior ids.
- Prefer authored flags and custom behavior over broad relationship meters.

## Conflict

Combat is locked as a core system in [Decision 007](./decisions.md#decision-007-combat-is-core-region-bound-and-supports-alternate-resolutions). More detail lives in [Combat System](./combat-system.md).

Combat should be robust, fast, and useful for grinding in the right regions. It should not mean every actor in the world can be killed.

Current prototype:

- Attack enemy
- Enemy retaliates
- Health changes
- Defeat drops key item
- Player cannot permanently die

Possible improvements:

- Defend to reduce incoming damage
- Use focus for warding
- Spare or bind defeated enemies
- Enemies tied to broken oaths
- Combat alternatives through items or vows

Recommendation:

Build normal battles to be quick and repeatable in combat regions. Build boss battles to be authored, memorable, and sometimes resolvable through persuasion, binding, sparing, items, or oath logic instead of only killing.

## Failure And Recovery

Failure should create texture, not dead ends.

Defeat behavior is locked in [Decision 008](./decisions.md#decision-008-defeat-resets-the-player-with-a-penalty).

Possible failure modes:

- Health reaches zero and player wakes at a safe room
- Focus depletion blocks rituals until rest
- Broken oath changes NPC trust or ending path
- Enemy becomes stronger or moves
- A route closes but another opens
- Region standing decreases

Avoid:

- Unannounced unwinnable states
- Random instant death
- Long repetition after failure

## Region Standing

Region standing is locked in [Decision 009](./decisions.md#decision-009-regions-track-coarse-standing). More detail lives in [Region Standing](./region-standing.md).

Core direction:

- Regions can like, dislike, or become hostile toward the player.
- Oaths tied to a region can raise or lower standing.
- Standing should be coarse and readable, not a detailed relationship meter.
- Special NPCs can still have advanced personal memory.

Region standing can influence shops, guards, encounters, rumors, oath witnesses, rest safety, routes, and endings.

## Progression

Progression is locked in [Decision 010](./decisions.md#decision-010-progression-uses-levels-plus-use-based-percentage-skills). More detail lives in [Progression System](./progression-system.md).

Emergent class and alignment direction is locked in [Decision 011](./decisions.md#decision-011-classes-emerge-from-player-behavior) and [Decision 012](./decisions.md#decision-012-alignment-changes-based-on-actions). More detail lives in [Class And Alignment](./class-and-alignment.md).

Progression should mostly come from:

- New verbs
- New oaths
- New information
- Changed relationships
- Access to new districts
- Objects with broader implications

Possible character progression:

- Maximum focus increases after major oath completion
- Health improves through rare milestones
- New command verbs unlock as rituals
- Reputation changes available bargains
- Character levels grant broad growth
- Percentage-based skills improve through use
- Skill points unlock new skills, tiers, caps, or training access
- Classes emerge from skills, oaths, standing, traits, and transformations
- Alignment changes based on actions

Recommendation:

Use levels for broad milestone growth, but use percentage-based skills for mastery. Skill points should mostly unlock skills rather than directly buying raw percentages.

## Save And Persistence

The game should be local-first.

Shipped (ticket #28, [Decision 046](./decisions.md#decision-046-saves-are-the-complete-versioned-session-and-loading-is-an-untrusted-input-boundary)):

- A save is the COMPLETE session — the mutated world + game state + event
  counter under a format version — written as JSON to a named slot through
  the hardened `oathstar-storage` layer (slot validation, symlink defense,
  atomic temp+rename writes). Default slot `quicksave`; save root
  `OATHSTAR_SAVE_DIR` (default `saves/`, gitignored).
- Loading is an untrusted-input boundary: version mismatch, world
  re-validation, and state/world coherence are all typed refusals that
  leave the running session untouched. Mid-combat saves persist and resume
  the exact pulse cadence.
- Server `POST /save` / `POST /load` swap the engine atomically under the
  engine lock; the client Save/Load buttons drive them.

Future direction:

- Tauri app-data save root (the configurable root is the hook)
- Migration tooling when version 2 exists (v1 rejects loudly)
- Autosave plus manual save slots / slot-picker UI

## Content Authoring

The current prototype stores world data in JavaScript objects. That is fine for the seed, but content will become easier to manage if we define a clean data shape.

Content authoring is locked in [Decision 022](./decisions.md#decision-022-content-is-toml-first-with-rust-validation-and-future-editor-tooling).

Possible future authoring formats:

- JavaScript modules for maximum flexibility
- JSON for stricter data validation
- YAML/TOML for writer-friendly content
- Hybrid: data files plus scripted room hooks

Recommendation:

Use TOML-first content definitions with Rust validation and Rust behavior hooks/code-behind. JSON remains useful for saves/API/debug payloads. Build a friendly editor later that reads and writes TOML.

## Vertical Slice Mechanics Target

A strong first vertical slice should prove:

- Parser can handle common commands gracefully.
- Rooms can change based on state.
- Oaths are more than quest labels.
- At least one problem has multiple solutions.
- At least one oath can be kept or broken.
- NPCs remember at least one meaningful player action.
- Combat can be avoided, transformed, or resolved with a non-attack action.
- Save/load preserves all meaningful state.

## Key Design Decisions To Make Next

1. What exactly does the player swear, and how explicit is that action?
2. Are oaths tracked like quests, resources, laws, or loadout slots?
3. How much combat should the final game contain?
4. Should enemies be killable, bindable, redeemable, or all three?
5. Do we want multiple endings in the first vertical slice?
6. Should commands be typed only, or should the UI offer clickable command composition?
7. What is the minimum parser quality needed before adding lots of content?
8. Should the story be chapter-based or one continuous city?
