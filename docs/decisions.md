# Oathstar Design Decisions

This log records design choices that are considered locked until we deliberately reopen them.

## Decision 001: Oaths Are Quests++

Status: Locked

Date: 2026-06-05

Oaths are the central progression and consequence system of Oathstar.

A normal quest asks the player to complete an objective. An oath asks the player to complete an objective while bound by a promise, before a witness, with consequences for keeping or breaking that promise.

Each oath should eventually include:

- Objective
- Promise
- Witness
- Keep condition
- Break condition
- Reward or unlock
- Consequence if broken
- World state changes

Near-term implementation:

- Oaths are authored rather than freely composed.
- The player can accept/swear specific oaths.
- The game tracks active, kept, and broken oaths.
- At least one oath in the vertical slice can be kept or broken.
- Keeping or breaking the oath changes NPC behavior, room state, or available routes.

Rationale:

This keeps the system legible while making quests feel more binding, thematic, and consequential than ordinary task lists.

Revisit when:

- Authored oaths feel too rigid.
- The parser can support richer language safely.
- We have enough content to justify oath conflicts or oath composition.

## Decision 002: Use A Forgiving Symbolic Parser

Status: Locked

Date: 2026-06-05

Oathstar uses a simple command parser built around readable symbolic phrases rather than full natural language.

The core input model is:

- `verb`
- `verb target`
- `verb target on target`
- `verb target to target`
- `ask target about topic`
- directional shortcuts such as `north`, `s`, `up`

Examples:

- `look`
- `look warden`
- `look shade`
- `take lantern`
- `use lantern on well`
- `give letter to archivist`
- `ask warden about oath`
- `talk archivist`
- `swear mercy`
- `break oath`
- `spare shade`

Parser behavior:

- Support common aliases.
- Support basic mob/NPC/object targeting, such as `look <mob>`.
- Keep commands deterministic and testable.
- Provide helpful failure messages when possible.
- Do not rely on AI or full sentence interpretation for core gameplay.

Rationale:

This keeps the game readable, fair, and authorable while still feeling friendlier than brittle old parser games.

Revisit when:

- The symbolic parser feels too limiting.
- There is enough content to justify disambiguation, autocomplete, or optional AI parser assistance.

## Decision 003: Rooms Are Described Entity Containers With Optional Attributes

Status: Locked

Date: 2026-06-05

Rooms are the primary unit of world space. Each room should have a title, description text, exits, and entities. Rooms may also have attributes/tags when those attributes are mechanically useful.

Core room data:

- Id
- Region
- Optional subregion
- Title
- Description
- Exits
- Entities

Optional room data:

- First-visit description
- Attributes/tags such as `dark`, `safe`, `locked`, `watched`, `sacred`, `flooded`, or `oathbound`
- State-based description variants
- Local scripts/hooks for special behavior

Regions and subregions:

- Every room belongs to a region.
- A room may also belong to a subregion.
- Region and subregion membership should be available to mechanics.
- Region-level rules can support things like local oath laws, ambient hazards, encounter tables, music/mood, faction control, or travel restrictions.

Entities include:

- Items
- NPCs
- Mobs/enemies
- Hazards
- Interactive fixtures/features

Rationale:

This keeps rooms simple to author while leaving room for mechanical depth when a room or region needs it. Attributes should exist because systems read them, not because every room needs a large schema.

Revisit when:

- Content authoring becomes repetitive.
- We need stronger validation for room data.
- Room attributes become numerous enough to require formal taxonomy.
- Region rules become important enough to need their own dedicated decision.

## Decision 004: Entities Use Shared Data With Role Contracts And Code-Behind Behaviors

Status: Locked

Date: 2026-06-05

Rooms contain entities. An entity is anything the player can inspect, target, use, talk to, fight, buy from, open, bind, swear to, or otherwise interact with.

All entities share a common base shape, then gain behavior through type, roles, attributes, and code-behind behavior hooks.

Base entity data:

- Id
- Type
- Name
- Description
- Aliases
- Attributes/tags
- Optional placement
- Optional behavior/script references

Actors:

- NPCs and enemies use the same actor model.
- Enemies are actors with combat/hostility behavior, not a separate top-level model.
- Shopkeepers, oath witnesses, guards, trainers, ghosts, and hostile mobs are all actors with different roles.

Roles and contracts:

- Roles declare capabilities such as `shopkeeper`, `combatant`, `conversable`, or `oathWitness`.
- If an entity declares a role, it must provide the metadata required by that role.
- Example: a shopkeeper must define stock/trade metadata and shop behavior.
- Example: a combatant must define health, attack profile, hostility state, and defeat behavior.

Code-behind:

- Entities may reference specific behavior scripts/handlers.
- This is similar to a Unity-style script concept.
- Entity data should reference behavior ids rather than store raw functions directly.
- Behavior code lives in registered modules and exposes known hooks.

Example hooks:

- `onLook`
- `onEnterRoom`
- `onTake`
- `onUse`
- `onTalk`
- `onAsk`
- `onGive`
- `onAttack`
- `onDefeat`
- `onSwearOath`
- `onKeepOath`
- `onBreakOath`
- `onTurn`

Rationale:

This keeps content data serializable and testable while allowing special entities to have custom behavior. It also prevents the engine from hardcoding every special case.

Revisit when:

- The behavior hook list becomes too large.
- We need schema validation or TypeScript contracts.
- A more formal ECS/component model becomes worthwhile.
- Save/load requires stricter separation between static entity definitions and mutable entity state.

## Decision 005: Oaths Use A Simple Four-State Lifecycle

Status: Locked

Date: 2026-06-05

Oaths use a simple first-version lifecycle:

- `available`
- `active`
- `fulfilled`
- `broken`

State meanings:

- `available`: The oath exists in the world, but the player has not sworn it.
- `active`: The player has sworn the oath, and the game is watching its keep/break conditions.
- `fulfilled`: The player completed the oath objective and its consequences have been applied.
- `broken`: The player violated the oath promise.

For now, `fulfilled` means the oath was successfully kept through completion. We will avoid extra states like `kept`, `declined`, `forsaken`, or `expired` until the design needs them.

UI direction:

- Show active oaths clearly.
- Show fulfilled and broken oaths as history.
- Make broken oaths visible rather than silently hiding them.
- Keep available oaths discoverable through NPCs, rooms, or interactions rather than cluttering the main UI.

Rationale:

This gives the game enough state to make promises consequential without overbuilding the system before the first vertical slice.

Revisit when:

- Oaths need time limits.
- The player needs to abandon an oath without breaking it.
- A promise can be kept while the broader objective remains unfinished.
- Multiple endings require finer oath history.

## Decision 006: NPCs Use Basic Memory With Advanced Memory For Special Characters

Status: Locked

Date: 2026-06-05

All NPC/actor entities can use basic memory. Important characters can opt into advanced memory through roles and code-behind behavior.

Basic NPC memory:

- Has met player
- Has been talked to
- Items received
- Topics discussed
- Oaths witnessed
- Oaths seen fulfilled
- Oaths seen broken
- Local problem state
- Hostile/friendly/neutral disposition when needed

Advanced NPC memory:

- Character-specific state
- Deeper relationship or trust state
- Custom remembered events
- Conditional topic unlocks
- Faction or region reactions
- Long-term consequences tied to oaths, mercy, violence, lies, or gifts

Implementation direction:

- Basic memory should be generic and available to all actors.
- Advanced memory should be opt-in, attached through actor roles or behavior ids.
- Avoid global relationship meters until the design needs them.
- Prefer authored memory flags over opaque scoring.

Rationale:

This gives normal NPCs enough continuity to feel responsive while allowing major characters to carry richer state without forcing every NPC into a heavy relationship system.

Revisit when:

- Many special NPCs need the same advanced behavior.
- Factions become important enough to need their own memory layer.
- We add AI-assisted dialogue or lore recall.

## Decision 007: Combat Is Core, Region-Bound, And Supports Alternate Resolutions

Status: Locked

Date: 2026-06-05

Combat is a robust core system of Oathstar. The game should support fast-paced regular battles, meaningful grinding, and compelling boss encounters.

Combat boundaries:

- Not every actor can be killed.
- Combat is controlled by actor roles, room attributes, region rules, story state, and oath constraints.
- Combat is primarily expected in outside areas, hostile regions, wild zones, ruins, encounter routes, and boss spaces.
- Protected NPCs such as shopkeepers, oath witnesses, court figures, and important quest characters should not be killable by default.

Battle direction:

- Normal battles should be quick and frequent in combat regions.
- Grinding is expected and should be mechanically useful.
- Boss battles should be more authored and memorable.
- Combat should stay readable through the command interface.

Alternate resolution:

- Some goals can be resolved without killing.
- Possible alternatives include persuade, spare, bind, bargain, use item, exploit a region rule, or fulfill/break a related oath.
- Major conflicts should often support multiple paths, but ordinary grinding battles can stay straightforward.

Rationale:

This gives the game RPG weight and repeatable play while preserving the oath-driven choice space. Combat matters, but the world does not become a universal murder sandbox.

Revisit when:

- Grinding rewards are defined.
- Region rules for encounter spawning are designed.
- Combat starts overpowering oath and exploration systems.
- Boss encounters need a dedicated phase/resolution model.

## Decision 008: Defeat Resets The Player With A Penalty

Status: Locked

Date: 2026-06-05

Losing a battle should usually reset the player to a recovery point with a penalty rather than cause a hard game over.

First-version direction:

- Defeat moves the player to a safe recovery room or region checkpoint.
- Defeat applies a meaningful but recoverable penalty.
- The exact penalty can vary by region, oath state, or encounter type.
- Boss defeats may have special authored consequences.

Possible penalties:

- Lose currency
- Lose materials
- Lose region progress
- Lose temporary buffs
- Reduce region standing
- Increase local danger
- Strain or break a relevant oath
- Enemy or boss state changes

Rationale:

This supports grinding and experimentation without making failure meaningless. The player should feel the cost of defeat, but the game should keep moving.

Revisit when:

- The progression economy is defined.
- Region checkpoints are designed.
- Oath-breaking penalties become more concrete.
- Boss battle structure is designed.

## Decision 009: Regions Track Coarse Standing

Status: Locked

Date: 2026-06-05

Regions track broad standing toward the player. Completing or breaking oaths tied to a region can move standing up or down.

Standing should be coarse and readable rather than a high-maintenance relationship meter.

Suggested first-version states:

- `unknown`
- `neutral`
- `liked`
- `disliked`
- `hostile`

UI can present this even more simply as liked/not liked unless the region is actively hostile.

Standing can be affected by:

- Completing regional oaths
- Breaking regional oaths
- Helping important regional actors
- Harming protected actors
- Defeating regional threats
- Boss outcomes
- Major story choices

Standing can affect:

- Shop access or prices
- Guard behavior
- Encounter pressure
- Available rumors
- Oath witnesses
- Rest safety
- Region routes or endings

Rationale:

This creates a Starfield-like sense that places remember the player's behavior, but keeps balancing simple. The game should not require every NPC to maintain a detailed relationship score.

Revisit when:

- Factions become distinct from regions.
- Region standing needs numeric thresholds.
- Standing becomes important enough to need dedicated UI.

## Decision 010: Progression Uses Levels Plus Use-Based Percentage Skills

Status: Locked

Date: 2026-06-05

Oathstar uses a hybrid progression model:

- Character levels for broad growth
- Percentage-based skills that improve through use
- Skill points for unlocking skills, tiers, caps, or training access
- Oaths, region standing, and gear for world-facing progression

Skills:

- Skills are represented as percentages.
- Skills improve by being used.
- This is inspired by classic ROM/ROT-style MUD skill progression.
- Examples include `first attack`, `second attack`, `third attack`, `parry`, `persuade`, and `bind oath`.

Skill points:

- Skill points should not primarily buy raw percentage increases.
- Skill points should unlock new skills, higher-tier skills, combat techniques, skill caps, or initial training.
- Once unlocked, a skill improves through use.

Levels:

- Leveling should feel good and support grinding.
- Levels can grant health, focus, skill points, trainer access, and broad survivability.
- Levels should not be the only meaningful progression axis.

Rationale:

This preserves the satisfying feel of leveling while keeping mastery grounded in practice. It also fits Oathstar's MUD inspiration and supports combat grinding without flattening progression into simple stat purchases.

Revisit when:

- We define exact skill improvement formulas.
- We decide whether trainers are required past percentage thresholds.
- We define combat skill tiers and social/oath skill categories.

## Decision 011: Classes Emerge From Player Behavior

Status: Locked

Date: 2026-06-05

Oathstar does not use fixed starting classes. The player starts from a neutral identity and class identity emerges automatically from learned skills, skill percentages, oaths, region standing, traits, transformations, and major choices.

Direction:

- No rigid starting class lock.
- Class titles are recognized by the game based on player development.
- Class identity can shift as the player changes.
- Class mastery may later unlock powerful skills, mythic/godlike abilities, pivots, titles, or transformations.
- Major transformations can alter or permanently close/open class paths.

Example class recognition:

- Sword skill + healing + good/faith oaths + sacred standing can become Holy Knight.
- Sword skill + forbidden rituals + broken mercy oaths + hostile sacred standing can become Dark Knight.

Rationale:

This keeps progression flexible and reactive while allowing the game to name and reward the kind of person the player is becoming.

Revisit when:

- We define class recipes.
- We design class mastery.
- Transformations need exact mechanical rules.
- UI needs to present current and possible class identities.

## Decision 012: Alignment Changes Based On Actions

Status: Locked

Date: 2026-06-05

Oathstar uses an action-driven alignment system inspired by D&D-style moral/order axes.

Suggested axes:

- Good / Neutral / Evil
- Lawful / Neutral / Chaotic

Alignment changes based on what the player does, not a fixed character creation choice.

Alignment can be affected by:

- Keeping or breaking oaths
- Mercy or cruelty
- Honesty or betrayal
- Following or defying region laws
- Helping or harming protected actors
- Transformation choices
- Boss resolutions

Alignment can influence:

- Emergent class recognition
- NPC reactions
- Region standing
- Oath availability
- Transformation eligibility
- Boss resolutions
- Endings

Rationale:

Alignment gives the game a broad moral/chaotic signal that can support emergent classes and transformations without replacing specific oath and region consequences.

Revisit when:

- We decide whether alignment is visible or mostly inferred.
- We define numeric thresholds under the hood.
- We decide how easy alignment recovery should be.

## Decision 013: Inventory And Items Are ROT-Inspired, Slot-Based, And Behavior-Driven

Status: Locked

Date: 2026-06-05

Oathstar's inventory and item systems should be robust and heavily inspired by ROT-style MUD inventory/equipment.

Inventory direction:

- Carry items
- Wear/remove equipment
- Use items
- Give/drop/trade items
- Inspect detailed item stats
- Support item flags, attributes, elements, and code-behind behavior

Equipment direction:

- Equipment is slot-based.
- Slots should support expressive MUD-style locations such as head, neck, body, hands, wrists, fingers, main hand, off hand, held, light, and side-specific jewelry such as left/right earring.
- More slots can be added as the game needs them.

Items:

- Items are entities.
- Items use the shared entity/code-behind model.
- Items can reference behavior ids for special hooks.
- Items can have full attributes, flags, rarity/tier, value, weight, effects, and elemental aspects.

Elemental aspects:

- Items, actors, skills, rooms, and regions may have elemental aspects.
- Elements can support rock-paper-scissors combat and puzzle logic.
- Start with a small readable element set before adding setting-specific aspects.

ROT codebase stance:

- ROT is a major design reference.
- Do not import ROT source directly until license implications are reviewed.
- Prefer implementing an original Oathstar model inspired by ROT behavior.

Rationale:

This supports combat grinding, class emergence, equipment identity, item-driven puzzle solving, and MUD authenticity.

Revisit when:

- Exact equipment slots are finalized.
- Inventory limits are designed.
- Elemental relationships are formalized.
- ROT source/license review is complete.

## Decision 014: Saves Are Versioned Local State Snapshots

Status: Locked

Date: 2026-06-05

Oathstar saves are local, versioned snapshots of mutable game state.

Static definitions stay in content/modules/code:

- Room definitions
- Region definitions
- Entity definitions
- Item templates
- Skill definitions
- Class recipes
- Oath definitions
- Behavior ids and handlers

Save files store mutable state:

- Current location
- Player stats
- Player level and XP
- Player skills and percentages
- Skill unlocks
- Inventory item instances
- Equipped item instances
- Item instance state
- Room state
- NPC/actor memory
- Oath states
- Region standing
- Alignment values
- Class identity and transformations
- Combat progress
- Quest/oath history

Implementation direction:

- Use JSON while iterating.
- Include a save schema/version field.
- Store behavior ids, not raw functions.
- Keep static definitions separate from mutable state.
- Move from prototype localStorage to Tauri app-data save files.

Rationale:

The game will have many stateful systems. Separating static content from mutable save state keeps save files smaller, migrations saner, and behavior code testable.

Revisit when:

- Module loading becomes real.
- Save migrations are needed.
- Multiple save slots or autosave are designed.
- Modded content needs compatibility rules.

## Technical Direction: Open-Source Modular Architecture

Status: Directional note, not locked implementation

Oathstar may become an open-source, module-friendly game/engine.

Preferred direction:

- Rust-first deterministic core
- Tauri shell
- Full event lifecycle around command resolution and state changes
- Module registry for regions, entities, items, skills, oaths, classes, transformations, behaviors, and UI panels
- Clear contracts/interfaces for roles and module contributions
- Optional LLM integration later

More detail lives in [Technical Architecture](./technical-architecture.md).

## Technical Direction: Datastar/SSE Frontend

Status: Directional note, not locked implementation

Datastar is a preferred candidate for keeping the frontend simple and backend-driven.

Potential shape:

- Rust core processes commands and state changes.
- A Rust HTTP/SSE layer streams rendered updates.
- Datastar patches DOM panels such as log, map, combat, inventory, equipment, oaths, and region standing.
- Tauri hosts the UI and local runtime.

This should be prototyped before being locked.

## Technical Direction: Core Engine With Swappable Worlds And Future DM/Multiplayer Modes

Status: Long-horizon directional note, not locked implementation

Oathstar may eventually separate into a core engine plus swappable world modules.

The first official world would be the Oathstar campaign, but the engine could later support alternate worlds, custom campaigns, DM-assisted sessions, LLM-assisted storytelling, or multiplayer/MUD-like modes.

Near-term guardrail:

- Build the single-player authored game first.
- Keep the Rust core deterministic and separate from presentation.
- Keep world/content definitions separate from core engine rules.
- Do not design the first vertical slice around multiplayer requirements.

Long-horizon possibilities:

- Human DM console
- AI-assisted DM tools
- Multiplayer party sessions
- Server-authoritative Rust world runtime
- Community worlds/modules
- Full world swapping

More detail lives in [Technical Architecture](./technical-architecture.md#long-horizon-direction-core-engine-swappable-worlds-and-dm-layer).

## Decision 015: The Backend Is A Standalone Rust Server Runtime

Status: Locked

Date: 2026-06-05

The backend is not Tauri. The backend is a standalone Rust server/runtime that owns game state, rules, saves, modules, and event resolution.

Tauri is the default local player app and can manage the server for normal desktop play.

Normal local flow:

1. Player opens the Tauri app.
2. Tauri starts or connects to a local Rust game server.
3. The frontend sends commands to the server.
4. The server resolves commands and streams events.
5. The frontend renders results.

This keeps the frontend swappable.

Possible clients:

- Tauri app
- Browser client
- CLI client
- DM console
- Debug/admin tools
- Future multiplayer client

Rationale:

This gives Oathstar a clean open-source architecture, supports future swappable frontends, and keeps the Rust runtime as the authority instead of embedding core game rules in the desktop shell.

Revisit when:

- Local server management creates too much packaging complexity.
- Tauri APIs need deeper integration than expected.
- Multiplayer or DM mode changes runtime requirements.

## Decision 016: Use REST Plus SSE First, With WebSockets Later If Needed

Status: Locked

Date: 2026-06-05

The Rust server should expose an API.

Initial transport direction:

- REST-style HTTP endpoints for commands, state snapshots, saves, settings, and world selection.
- SSE for server-to-client event streaming.

Possible first endpoints:

- `POST /command`
- `GET /state`
- `GET /events`
- `POST /save`
- `POST /load`
- `GET /worlds`
- `POST /worlds/select`

WebSockets are allowed later if the game needs bidirectional realtime communication for multiplayer, DM mode, chat, or simultaneous clients.

Rationale:

REST + SSE is simple, debuggable, Datastar-friendly, and enough for local single-player command/event flow. WebSockets are powerful, but should be added when their complexity is justified.

Revisit when:

- Multiplayer becomes active work.
- DM mode requires live bidirectional control.
- Chat or simultaneous clients become core.
- SSE becomes limiting.

## Decision 017: Persistence Starts File-Based Behind A Storage Interface

Status: Locked

Date: 2026-06-05

Oathstar starts with file-based persistence for local single-player.

File-based storage should handle:

- Saves
- Settings/config
- World/module manifests
- Logs
- Debug snapshots

Storage should sit behind an interface so a database can be added later.

A database may become useful for:

- Multiplayer server state
- User accounts
- Large event histories
- Cloud sync
- Mod indexes
- Analytics/debug timelines

Rationale:

File-based persistence fits the local-first, open-source, moddable shape of the project. Keeping storage abstract prevents this early choice from blocking future multiplayer/server work.

Revisit when:

- We design multiplayer persistence.
- Save counts or event history become large.
- Module indexes become complex.
- Cloud sync becomes a target.

## Decision 018: The Server Uses Events, Ticks, Scheduled Events, And Time Sequences

Status: Locked

Date: 2026-06-05

Oathstar's Rust server should be event-driven and support a shared world clock.

Timing model:

- Base tick target is 1 second.
- Tick interval should be configurable.
- Manual tick mode should exist for debugging and future DM control.
- Systems subscribe to ticks instead of all work running every tick.

The server should support:

- Command events
- Tick events
- Scheduled events
- Time sequences
- DM-triggered events
- Future LLM-proposed events

Time sequences:

- Allow the world to pause or slow.
- Resolve large strategic/story decisions.
- Apply validated event batches.
- Resume normal ticking afterward.

Rationale:

Ticks make the world feel alive and support multiplayer/MUD-like play. Time sequences allow slower systems such as human DM actions or LLM planning to participate without blocking the normal simulation.

Revisit when:

- Combat round timing is designed.
- Multiplayer concurrency is active work.
- DM tools are implemented.
- LLM director behavior is prototyped.

## Decision 019: LLMs And DM Directors Propose Actions Outside The Tick-Critical Path

Status: Locked

Date: 2026-06-05

LLMs should not directly perform tick-critical actions.

LLMs are too slow and unpredictable to control the normal tick loop.

Preferred LLM/DM use:

- Propose scheduled events
- Generate narration drafts
- Assist parser interpretation
- Suggest NPC strategic decisions
- Help human DMs
- Resolve larger actions during time sequences

Director types:

- Automatic scripted director
- Human director
- LLM-assisted director
- Fully LLM-scripted director, if safe enough later

The Rust core remains authoritative. DM and LLM layers propose or request changes; the core validates and commits.

Rationale:

This preserves deterministic game integrity while leaving room for rich DM and AI-assisted experiences.

Revisit when:

- LLM integration becomes active work.
- DM console permissions are designed.
- Multiplayer moderation and trust boundaries are defined.

## Decision 020: Advanced Complexity Is Opt-In Through Modules

Status: Locked

Date: 2026-06-05

Oathstar should be capable of very deep systemic play, but the base game should remain approachable.

The core game should support a basic experience:

- Explore
- Fight
- Level
- Complete oaths
- Finish the main game

Advanced systems should be opt-in modules.

Possible advanced modules:

- Outpost building
- Trade
- Crafting
- Wars
- Politics
- Faction management
- Economy
- Settlement simulation
- Dungeon master tools
- LLM director tools
- Advanced region events

New games can offer module presets such as:

- Story/Combat
- Adventurer
- Strategist
- Dungeon Master
- Full Simulation
- Custom

Rationale:

This allows Oathstar to grow toward Stellaris/Hearts of Iron levels of mastery without scaring away a first-time player. Complexity becomes a replay and mastery layer rather than an onboarding wall.

Revisit when:

- The first module manifest is designed.
- We decide which modules can be toggled mid-save.
- Official advanced systems are scoped.
- DM tools become active work.

## Decision 021: Modules Use Manifests, Registered Hooks, And Core-Validated State Changes

Status: Locked

Date: 2026-06-05

Modules are registered packages with manifests, content registrations, hooks, settings, and optional migrations.

Modules can contribute:

- Worlds
- Regions
- Rooms
- Entities
- Items
- Skills
- Oaths
- Classes
- Transformations
- Combat rules
- Economy systems
- UI panels
- Event hooks
- Save migrations
- Settings

Manifest data should include:

- Id
- Name
- Version
- Author
- Description
- Category
- Required engine version
- Dependencies
- Conflicts
- Enabled-by-default flag
- Registered content
- Registered hooks
- Settings

State mutation rule:

- Modules should not freely mutate arbitrary state.
- Modules register content, observe events, call engine APIs, and submit change requests.
- The Rust core validates and commits state changes.

First implementation boundary:

- Official modules only
- Local file/module loading
- Manifest required
- Small hook list
- No hot-loading mid-save unless explicitly safe

Rationale:

This keeps the module system expandable without turning the engine into uncontrolled spaghetti.

Revisit when:

- Community modules become active work.
- Module sandboxing is required.
- Save migrations are needed.
- Module hot-loading becomes desirable.

## Decision 022: Content Is TOML-First With Rust Validation And Future Editor Tooling

Status: Locked

Date: 2026-06-05

Oathstar content definitions should be TOML-first.

TOML should be used for:

- Module manifests
- Regions
- Rooms
- Entities
- Items
- Skills
- Oaths
- Class recipes
- Transformations
- Settings/config where appropriate

Rust should be used for:

- Core engine logic
- Behavior hooks/code-behind
- Validation
- Migrations
- Runtime systems

JSON should be used for:

- Save files while iterating
- API payloads where convenient
- Debug snapshots where useful

Future editor tooling:

- TOML is the durable source format.
- A friendly editor can later read/write TOML files.
- Possible editors include room editor, entity editor, item editor, oath editor, region map editor, module manifest editor, and DM tools.

Validation requirements:

- Missing required fields should fail early.
- Broken references should fail early.
- Role contracts should be validated.
- Module dependencies/conflicts should be validated.
- Save/content version compatibility should be checked.

Rationale:

TOML is human-readable, Rust-friendly, diffable, and good for open-source/modded content. Future editor tooling can improve authoring without replacing the underlying format.

Revisit when:

- TOML becomes too awkward for large content.
- A visual editor becomes active work.
- Community modules require stricter schemas.

## Decision 023: Combat Uses Server-Authoritative Pulses

Status: Locked

Date: 2026-06-05

Combat runs on server-authoritative pulses layered over the normal world tick.

Initial timing:

- World tick: 1 second
- Default combat pulse: 2 seconds

On combat pulses:

- Engaged actors can auto-attack.
- Combat skills can trigger.
- Defensive checks can resolve.
- Timed effects can tick.
- Combat events stream to clients.

Manual commands can be submitted between pulses.

Examples:

- `flee`
- `use potion`
- `cast ward`
- `switch target`
- `spare shade`
- `persuade guard`
- `bind oath`

Combat pulse timing may vary by actor, skill, effect, region, or boss phase.

Boss fights, DM events, or major scripted moments can pause combat into a time sequence, resolve special actions, and then resume pulses.

Rationale:

A 2-second default pulse keeps grinding battles fast and readable without spamming text every tick. Server-authoritative timing supports future multiplayer and DM control.

Revisit when:

- We prototype combat feel.
- Skill trigger rates are tuned.
- Multiplayer timing is designed.
- Boss phase scripting begins.

## Decision 024: The First Proof Of Concept Is A Beginner Module

Status: Locked

Date: 2026-06-05

The first real proof of concept should be a small, complete Beginner module.

Core shape:

- Start in a local town.
- Receive or discover a simple town oath.
- Explore nearby areas.
- Fight and grind in a basic outside/combat region.
- Progress toward a tower or similar dungeon objective.
- Defeat or otherwise resolve a boss.
- Use the ending to hook into the broader story or next module.

The Beginner module should prove:

- Server/client architecture
- Command parser
- Rooms/regions/entities
- Oath lifecycle
- Combat pulses
- Basic grinding
- Inventory/equipment
- Skill progression
- Region standing
- Save/load
- SSE event streaming

The larger Oathstar story can then hook onto this via future modules that expand the world, skills, oaths, regions, classes, transformations, and advanced systems.

Rationale:

This provides a finishable proof of concept without drowning the first build in every long-term idea.

Revisit when:

- The Rust server architecture is ready for implementation.
- The first module content outline is written.
- We need to decide the exact town, oath, tower, and boss.

## Decision 025: The Map Uses A Square Grid With Cardinal Directions Plus Up/Down

Status: Locked

Date: 2026-06-05

Oathstar's map/minimap uses a square grid.

Supported core directions:

- North
- South
- East
- West
- Up
- Down

Diagonal directions are not part of the core navigation model.

Rooms define exits, map coordinates, and passability through metadata. The server exposes map state as structured JSON. The frontend decides whether to render that data as text, ASCII, canvas, sprites, or debug overlays.

Rendering direction:

- Start text/ASCII-style.
- Consider canvas early so the grid can later support sprites and overlays.
- Support passable/non-passable cells for collision.
- Keep the backend renderer-agnostic.

Rationale:

Square cardinal navigation is easier to understand and matches MUD-style movement. Up/down supports vertical spaces without overwhelming the player with diagonal exits. Passability keeps the model compatible with richer ASCII maps and eventual top-down tile/sprite rendering.

Revisit when:

- The minimap renderer is implemented.
- Sprite/tile rendering becomes active work.
- World modules need non-grid spaces.

## Decision 026: The Implementation Uses A Cargo Workspace

Status: Locked

Date: 2026-06-05

Oathstar should be implemented as a Rust Cargo workspace.

Likely crate/app shape:

- `oathstar-core`: deterministic game engine, parser, state, event lifecycle
- `oathstar-protocol`: shared API/event/save DTOs
- `oathstar-server`: standalone Rust runtime, REST/SSE API, module loading
- `oathstar-storage`: file-first persistence and future database abstraction
- `oathstar-content`: built-in content/world/module loading
- `oathstar-ai`: optional local AI integration later
- `apps/tauri`: desktop shell that manages/connects to the local server
- `apps/web`: Datastar/web frontend
- `modules/beginner`: first proof-of-concept module

Architecture docs remain the source of truth. Coding agents and CLI tools can execute the implementation, but should follow the locked decisions and system docs.

Rationale:

The project is engine-shaped. A Cargo workspace gives clean boundaries between core rules, transport, storage, content, Tauri, web UI, and optional AI.

Revisit when:

- The workspace layout is scaffolded.
- We decide exact HTTP framework and client rendering stack.
- Module loading needs its own crate split.

## Decision 027: The Rust Server Uses Axum And Tokio

Status: Locked

Date: 2026-06-05

The standalone Rust server/runtime should use Axum on Tokio.

Axum/Tokio should support:

- REST-style HTTP routes
- SSE event streams
- WebSockets later if needed
- Shared server state
- Middleware through Tower
- Async world loop and tick tasks
- Local Tauri-managed server runtime

This aligns with prior decisions:

- REST + SSE first
- WebSockets later if justified
- Server-authoritative game state
- Event/tick-driven runtime
- Swappable frontends

Rationale:

Axum and Tokio are mature, idiomatic Rust choices for async HTTP services. They fit local-first APIs now and leave room for multiplayer/server expansion later.

Revisit when:

- Axum conflicts with Tauri packaging or local runtime needs.
- Datastar integration suggests a different approach.
- Multiplayer networking requirements change substantially.

## Decision 028: Protocol Is Hybrid With Typed Domain Events And Componentized Output

Status: Locked

Date: 2026-06-05

The Rust core emits typed domain events. Domain events are the source of truth.

The server can expose or render those events as:

- Structured JSON
- Datastar-compatible HTML/SSE fragments
- Plain text for simple clients

The player UI should not be a single textarea. Output should be rendered as a catalog of typed interactive components.

Possible output components:

- Narrative message
- Combat message
- Loot message
- Skill improvement message
- Oath card
- Region standing card
- Room header
- Entity chip
- Item card
- Combat action prompt
- Boss phase banner
- DM narration block
- Map patch
- Inventory patch
- Equipment patch

Rationale:

This keeps the engine client-agnostic while allowing the Datastar frontend to feel rich and interactive. JSON supports alternate clients, debugging, DM tools, tests, and future multiplayer. HTML fragments keep the main UI simple and backend-driven.

Revisit when:

- Datastar rendering is prototyped.
- Event schema becomes too broad.
- Alternate clients require stricter protocol guarantees.

## Decision 029: Long-Term Rule Systems Are Swappable

Status: Accepted

Oathstar should strive toward a modular roleplaying runtime where major rule
systems can eventually be swapped or composed.

Potentially swappable rule systems include:

- Battle system
- Player stats and attributes
- Progression and skill advancement
- Inventory and equipment model
- Class/transformation rules
- Oath and quest logic
- Region standing and faction rules
- Economy/trade/crafting rules
- World/director behavior
- DM and LLM director behavior
- UI/rendering components

The first implementation should still ship one official built-in ruleset. The
important early constraint is architectural: do not permanently weld combat,
stats, progression, inventory, worlds, or director behavior into the core
kernel.

The Rust core should own:

- Module loading
- Compatibility validation
- Event ordering
- Save/load and migrations
- Authoritative state commits
- Typed contracts for rule-system modules

Rule modules should implement contracts and submit core-validated change
requests. They should not freely mutate arbitrary state.

Rationale:

This keeps the Beginner module achievable while preserving the long-term dream:
a choose-your-own-adventure and roleplaying experience for solo play, friends,
DM-led campaigns, custom worlds, and eventually LLM-assisted directors.

Implications:

- Early systems should be built with clear boundaries.
- The first combat/stat/progression implementation can be concrete, but it
  should expose the shape of a future contract.
- Module presets need compatibility validation before arbitrary combinations are
  allowed.
- Saves must eventually record active modules and rule-system versions.

Do not overbuild this now. Let tickets extract the contracts as real systems are
implemented.

## Nice To Have: Oathscript

Status: Future idea, not locked implementation

Oathscript is a possible lightweight scripting language interpreted by Rust.

Purpose:

- Let module authors hook into lifecycle events.
- Avoid requiring every module author to write/compile Rust.
- Keep scripts controlled, deterministic, sandboxed, and core-validated.

Possible uses:

- Simple item behavior
- NPC conditional responses
- Oath reactions
- Region events
- Scheduled events
- DM-authored quick events

Guardrails:

- No arbitrary system access
- No network or filesystem access by default
- Engine validates state changes
- Script errors fail safely
- Rust behavior modules remain available for complex systems

## Next Mechanics To Decide

Recommended order:

1. Beginner module content outline
2. Combat/stat formulas
3. Initial component catalog

## Technical Direction Notes

### Local AI Runtime

Status: Directional note, not locked implementation

Oathstar may eventually include an optional local AI runtime for NPC variation, parser assistance, lore recall, or authored-content tooling.

Preferred approaches:

1. Tauri sidecar that bundles an Ollama or Ollama-like local model runner.
2. Full Rust-side integration using a local inference library if it becomes practical.
3. Fallback support for user-installed Ollama during development or advanced-user use.

Current stance:

- Do not make local AI required for the first vertical slice.
- Keep core game mechanics deterministic and authored.
- Treat AI as an enhancement layer, not the source of truth for quests, oaths, or world state.
- If bundled later, use only models with licenses compatible with free distribution and optional donations.
- Keep model size, hardware requirements, startup time, and offline behavior central to the decision.

Rationale:

Bundling a local AI runner is technically possible in Tauri through sidecars, and a Rust-native path may be attractive later. The larger risks are model size, model licensing, reliability, and making sure the parser game remains designed rather than improvised.

## Decision 030: World Invariants Are Validated At The Core Construction Boundary

Status: Locked

Date: 2026-06-06

Concretizes Decision 021 (core-validated state changes) and Decision 022 (TOML
content with Rust validation) for engine startup.

`oathstar-core` owns world-invariant validation. `WorldDefinition::validate()`
checks the invariants, and `Engine::try_new(world) -> Result<_, WorldValidationError>`
is the only constructor (the infallible `Engine::new` is removed), so a malformed
world cannot construct an `Engine` that later panics. `oathstar-content` delegates
to the core validator rather than keeping its own copy.

Validated invariants:

- Each room is stored under a map key equal to its own `id`.
- The start room exists.
- The start room is passable.
- Every room exit targets an existing room.

Rationale:

- Malformed or untrusted module data must surface as a typed error at the
  construction boundary, not a deferred panic — important as worlds become
  swappable (Decision 029, module system).
- Single source of truth for invariants; no duplicate validators.
- The room key==id check is load-bearing: movement sets the current room from a
  room's own `id` field, so without it the engine's current-room lookup could
  fail.

Revisit when:

- World data gains new structural requirements (reachability, required entities),
  or community modules require input-size / sandboxing limits at the loader
  boundary.

## Decision 031: Domain Events Are Emitted Through Explicit Engine Calls, With A Stable Wire Split

Status: Locked

Date: 2026-06-06

Concretizes Decision 028 (typed domain events) and Decision 005 (oath lifecycle)
as implemented in the beginner vertical slice (ticket #7).

Event emission:

- The engine emits domain events only from explicit calls: `Engine::handle_command`
  (per player command) and `Engine::begin` (the opening scene for a new game).
  `Engine::try_new` constructs state but emits nothing.
- Therefore any "when a new game starts, show X" behavior must call `begin()`; the
  first player command is not the trigger.

Wire format (serde):

- A `GameEvent` is camelCase (`eventId`, `tick`, `channel`) with its
  `GameEventKind` `#[serde(flatten)]`-ed in under a snake_case `type` tag (e.g.
  `oath_sworn`). Event-kind payload fields stay snake_case (`oath_id`, `room_id`)
  because `rename_all` does not cross `flatten`.
- View/snapshot structs (`GameSnapshot`, `OathSnapshot`, …) are camelCase
  (`oathId`).
- Net: clients read snake_case keys on the `/events` stream and camelCase keys on
  `/state` snapshots. The split is intentional and consistent with the existing
  `RoomEntered.room_id` vs `RoomSnapshot.id`.

Oath/boss v1 (placeholder):

- Oath state is `OathStatus { Sworn, Fulfilled }` (Decision 005's
  `active`/`fulfilled`); "available" = not-yet-sworn (`None`); `Broken` is deferred
  until a break path exists (an unconstructed variant would be an uncoverable
  mutant under the 100% MSI gate).
- The boss endpoint is the placed entity carrying the `"boss"` role (Decision 004
  roles); `confront` is gated on an active oath.

Rationale:

- Explicit emission keeps the engine request/response and testable; an
  opening-scene emitter (`begin`) makes "on start" behavior real without
  auto-emitting from a constructor.
- Documenting the snake/camel split keeps client authors from guessing field
  casing per payload.
- Minimal oath/boss shapes keep mutation surface small (100% MSI) while proving
  the loop.

Revisit when:

- Full combat replaces the boss placeholder (Decision 007 / 023).
- A break path requires `OathStatus::Broken`.
- A second oath or boss per room needs explicit targeting.

## Decision 032: The First Playable Client Is A Framework-Free, Server-Authoritative Hypermedia Shell

Status: Locked

Date: 2026-06-07

Implements ticket #12 (the beginner-slice Tauri + Datastar map-forward UI shell)
and concretizes Decisions 015/016/028 (the Rust server is the authority; REST +
SSE; hybrid typed events).

The first client:

- Is plain ES modules — no React, Vue, Svelte, or other SPA framework (REQ-008).
- Is server-authoritative: it POSTs `/command`, consumes the `/events` SSE stream,
  and renders `/state` snapshots. The Rust runtime owns all game state.
- Splits pure render/logic (DOM-free, tested under `node --test`, in `src/client/`)
  from a thin browser-only DOM/transport seam (`src/client-app.js`, smoke-verified).
  This mirrors the prototype `engine.js` (tested) / `app.js` (glue) split and keeps
  the JS coverage floor honest.
- Renders typed events into componentized output (a structured article per event),
  never a textarea, and never sets `innerHTML` from server data (XSS-safe by
  construction — all `textContent`).
- Owns its map render config (`{ tilePixels, mode }`) client-side; the server
  `MapSnapshot` shape is unchanged (REQ-007). The minimap renders one z-plane (the
  current room's floor) so rooms stacked at the same `(x, y)` never collide.

The opening scene is delivered by replaying `Engine::begin()` onto every new
`/events` subscription (Decision 031: `try_new` emits nothing). begin() is captured
once at startup; the seed reuses the live serialization path.

Deferred (separate tickets/decisions), NOT chosen here:

- Vendoring the Datastar runtime + emitting Datastar-format SSE (`datastar-merge-*`)
  from the server. The shell stays Datastar/SSE-compatible (the `ui-design.md`
  guardrail) so adopting the library later is additive, not a rewrite.
- Tauri server lifecycle (auto-spawn/sidecar) + cross-origin CORS. Dev uses a vite
  proxy for same-origin; the client base URL is overridable (`VITE_OATHSTAR_API`)
  for a Tauri loopback build (the client is Tauri-ready).

Rationale:

- "Smallest production-direction shell": prove the server-authoritative
  command/event/render loop end-to-end without committing to a frontend framework
  or a server-side Datastar rewrite.
- The pure/glue split makes the render logic unit-testable under the strict gate
  while the browser seam is smoke-verified.

Revisit when:

- A panel needs reactive server-driven DOM patching that hand-rolled glue makes
  unwieldy → adopt the Datastar library + server-side Datastar SSE.
- Tauri packaging needs the desktop shell to manage the local server lifecycle.

## Decision 033: Player Client And Host Manager Are Separate Web Surfaces, Packaged By Tauri Later

Status: Locked

Date: 2026-06-07

Oathstar will keep the Rust game server, the player client, and the host/server
manager as separate responsibilities.

The runtime topology is:

- `oathstar-server`: the headless Rust authority for world state, rules, saves,
  ticks, modules, REST/SSE transports, and future DM/LLM integration points.
- Player Client: a pure game UI that connects to a compatible server URL, sends
  commands, receives UI/state/event streams, and renders play. It does not own
  server lifecycle or host administration.
- Host Manager / Director Console: a separate web surface for local server
  lifecycle, worlds/modules/saves, settings, connected players, DM controls,
  tick control, and future automated director/LLM tools.

Development is browser-first for both web surfaces. Tauri is a packaging and
native-lifecycle layer, not the source of gameplay authority. A desktop build may
bundle a launcher, player client, host manager, and server sidecar for
convenience, but the player client must remain able to connect to any compatible
server.

Rationale:

- Browser iteration is faster than repeatedly validating desktop packaging while
  the UI is still moving.
- The split keeps local single-player, LAN/friends, dedicated-server, and DM modes
  compatible instead of baking server management into the player UI.
- Tauri-specific Rust should be used for native lifecycle, local files, tray/window
  behavior, and sidecar management when packaging needs it. It should not move
  rules or game state out of the server.

Revisit when:

- Tauri packaging requires shared launcher UX that intentionally combines host
  and player entry points.
- A mobile or terminal client requires a new adapter contract.

## Decision 034: First-Party UI Uses Datastar SSE HTML By Default, With JSON Reserved For Renderer Data And Adapters

Status: Locked

Date: 2026-06-07

Oathstar's first-party web UI will use Datastar plus SSE-delivered HTML fragments
as the default UI component model. JSON remains available, but it is not the
primary first-party UI rendering contract.

Default transport responsibilities:

- Datastar/SSE HTML: panels, event feed components, menus, oaths, inventory,
  equipment, dialogue, shops, combat surfaces, host-manager controls, and other
  ordinary UI fragments.
- JSON: canvas/map/sprite renderer data, diagnostics, tests, machine-readable API
  surfaces, and optional alternate-client adapters.

The Rust backend should expose a Datastar presentation layer, preferably as a
module/crate boundary, that maps core game state/events into Datastar-compatible
HTML/SSE fragments. The core engine must not depend on Datastar directly.

Styling direction:

- Use project-owned CSS/component vocabulary for the player client first.
- Do not adopt DaisyUI for the player client now; it risks making the game shell
  feel generic and dashboard-like.
- DaisyUI or another component kit can be reconsidered later for the Host Manager
  if admin velocity matters more than bespoke presentation.

Rationale:

- We are intentionally optimizing for the Oathstar product, not for React/Vue
  client neutrality.
- Keeping Datastar in a presentation adapter preserves a clean core while letting
  first-party UI move quickly with server-rendered components.
- JSON is still kept where structured renderer data is truly useful, especially
  maps/canvas/sprites.

Revisit when:

- A second real frontend needs a non-Datastar adapter.
- Datastar cannot express an interaction surface cleanly enough without excessive
  custom glue.

## Decision 035: The Map Renderer Starts As A First-Party 32x32 Square-Grid Canvas, Not A Full Game Engine Dependency

Status: Locked

Date: 2026-06-07

The visual map/minimap should evolve from node-like room cards into an actual
square tile grid. The first canvas renderer should be a small first-party renderer
that consumes server JSON and draws a 32x32 tile grid by default.

Renderer direction:

- Use square grid cells, not freeform rectangles.
- Default tile size: 32x32 pixels.
- Keep tile size configurable for accessibility and future sprite packs.
- Preserve the current no-diagonal navigation model: north, south, east, west,
  up, and down.
- Represent passable and non-passable cells so rooms can become dense interiors
  such as shops, towers, walls, and walkable floor tiles.
- Keep server map payloads renderer-agnostic JSON; the server sends map state, not
  canvas drawing commands.

External game-engine dependencies such as Kiwi.js are deferred. The v1 renderer is
small enough that a dedicated engine would add more coupling than value. Kiwi.js
appears to be an older HTML5 engine, so it is not the right anchor for the first
Oathstar grid renderer.

Rationale:

- The map is initially a tactical/navigation surface inside a Datastar UI, not a
  full action game engine.
- A first-party renderer lets us match the MUD/grid rules exactly and avoid
  fighting an engine's scene/entity lifecycle before we need one.
- Moving to Pixi.js, Phaser, MelonJS, or a custom WebGL path can remain a later
  performance/art decision if sprite count, animation, lighting, or effects demand
  it.

Revisit when:

- The canvas renderer needs animation batching, sprite atlases, particles, camera
  transforms, or high entity counts.
- Map tooling/editor needs become large enough to justify adopting a rendering
  library.

## Decision 036: Spatial Awareness Uses Room/Cell-Granularity Proximity With Action-Specific Radiuses

Date: 2026-06-07

Entities, items, and fixtures become discoverable and interactable by *distance* —
the "blast radius" model — rather than requiring exact room co-location. The v1
foundation (ticket #17) is room/cell-granularity: a positioned thing inherits the
`(region, subregion, x, y, z)` of the room that places it, because rooms ARE the
grid cells (Decision 025) and entities/items carry no coordinates of their own.

The model (engine, `crates/oathstar-core/src/awareness.rs`):

- Proximity is gated by the same region, subregion, and z-plane, then measured by
  Chebyshev (king-move) distance, `max(|dx|, |dy|)` — a square radius matching the
  square grid; integer-only and deterministic.
- Action-specific radiuses (sight, interaction; defaults 3 and 1 cells), extensible
  to hearing/detection without changing call sites.
- A structured awareness result classifies each perceived thing as `exact`,
  `interactable`, or `visible` (seen but out of reach), plus a `hidden` reveal-rule
  placeholder that excludes a thing from all perception.
- The result is server-authoritative JSON exposed additively on
  `RoomSnapshot.contents` (`NearbySnapshot`) — never canvas drawing instructions
  (Decisions 034/035). The existing client Nearby panel renders it unchanged.
- `look <target>` resolves through the proximity resolver (exact cell first,
  alias/case-insensitive); `talk`/`take` reuse the same resolver when they land.

Rationale:

- Entities/items have no coordinates yet; inheriting the room cell is the smallest
  honest foundation and is exactly what "notice an NPC two cells west" means on the
  current grid. Per-entity intra-room coordinates can extend `Position` later.
- A square (Chebyshev) radius matches the square tile grid and avoids floating
  point; the metric is isolated in one function for a future swap.
- Additive JSON keeps the server renderer-agnostic and the change backward-
  compatible (empty `contents` omitted; old payloads still deserialize).

Revisit when:

- Rooms become multi-cell areas (per-entity intra-room coordinates).
- Combat aggro, stealth/noise, or detection need computed reveal rules, a hearing
  radius, or line-of-sight/passability occlusion (the `hidden` flag is the seam).
- Map overlays need entity/event markers driven by distance/proximity.

See `docs/spatial-awareness.md` for the full model.

## Decision 037: Oaths Are Offered By An Issuer NPC Before They Can Be Sworn

Status: Locked

Date: 2026-06-08

Concretizes Decision 005 (oath lifecycle) and Decision 006 (NPCs keep oaths
discoverable through conversation) as implemented in ticket #19.

An oath may name an **issuer**. `OathDefinition` gains optional `issuer_id` (the
oath-giver entity) and `source` (a region/faction origin string, recorded for
future oath-giver UI and region/faction effects). When an oath names an issuer it
becomes **offer-gated**: the player must be *offered* it — by talking to the
issuer at interaction range — before `swear` will bind it. The offer is a single
authored flag (`GameState.offered_oath_id`), not a relationship meter
(Decision 006). An oath with no issuer stays globally swearable (the pre-#19
behavior), so the gate is opt-in and backward-compatible.

Dialogue is authored and TOML-first (Decision 022): a conversable NPC may carry
an `EntityDialogue { greeting, oath: { offer, sworn, fulfilled } }`. `talk <npc>`
returns the line selected by the player's oath state when the NPC issues the
designated oath, otherwise the `greeting`; an NPC with no dialogue keeps the
generic reply. Dialogue is delivered as the existing `NarrativeMessage` event, so
the oath event/snapshot wire shapes (`OathSworn`/`OathFulfilled`) are unchanged
(Decisions 028/031). A dangling `issuer_id` is rejected at the construction
boundary (Decision 030, `WorldValidationError::OathIssuerMissing`).

Rationale:

- Moves the beginner oath from a contextless global action to a discovered,
  authored interaction without overbuilding NPC memory or dialogue trees.
- Opt-in via `issuer_id` keeps existing and other-module oaths working unchanged.
- Issuer/source metadata gives future oath-giver UI and region/faction systems a
  real hook while costing one optional field today.

Revisit when:

- Multiple simultaneous oath offers, branching/modal dialogue, or persuasion are
  needed (all scoped out of v1).
- NPC memory (Decision 006) graduates from a single offered-flag to per-actor
  state, or `source` begins to drive real region/faction standing (Decision 009).

## Decision 038: Inventory v1 Is A Carried-Id Pack With Drop/List/Inspect And A Kind/Flags Placeholder

Status: Locked

Date: 2026-06-08

Concretizes Decision 013 (ROT-inspired inventory) as the minimal first foundation
(ticket #20) — without jumping to the full equipment system.

The player's carried inventory is `GameState.pack: Vec<String>` — ordered carried
item ids (#18); names/kind/flags are resolved from the `world.items` registry at
snapshot time. Supported operations:

- `take <item>` (#18) — carry a reachable world item.
- `drop <item>` — the inverse of take: remove from the pack and place the item in
  the current room/cell, where the #17 awareness resolver surfaces it again.
- `inventory` / `pack` / `i` — list carried items, or an honest empty state.
- `look <item>` — resolves a carried item from the pack as well as nearby world
  contents (a carried item has no cell, so it resolves from inventory).

Items (`Item`) gain optional `kind` (a coarse type placeholder, e.g. `"light"` /
`"quest"`, defaulting to `"item"`) and `flags` (a small authored tag list, e.g.
`["oath"]`). These are **authored content data, never invented by engine or
client**; the pack snapshot (`PackItemSnapshot`) exposes id/name/kind/flags
additively, and the client Pack tab renders that server data only. Inventory
operations are total and non-corrupting — an unknown/uncarried/orphan target is
refused with no state change.

Deferred to later tickets (NOT v1, per Decision 013's long horizon): equipment
slots, wear/remove, weight, stacking/quantities, containers, currency, item use,
shops, crafting, rarity/tier, elemental aspects, and persistence.

Rationale:

- Gives `take` a durable home and a stable base for future equipment/shops/loot/
  quest items without the testing debt of a premature full ROT model.
- Reuses the #17 awareness resolver for drop/look (no new geometry) and stays
  additive on the wire (Decisions 028/031/034).
- `kind`/`flags` are the smallest forward-looking hook for future item taxonomy
  and behavior, authored in TOML (Decision 022).

Revisit when:

- Equipment slots / wear-remove are designed (Decision 013).
- Stacking, weight, containers, or currency become needed.
- Item `kind` graduates from a placeholder string to a real taxonomy/enum.

## Decision 039: Entity Roles Are A Typed Vocabulary With Validated Contracts

Status: Locked

Date: 2026-06-08

Implements Decision 004's role-contract layer (ticket #21) as the first typed slice
— the validation that `mechanics-and-systems.md` previously flagged "not yet
implemented."

Entity capabilities stay authored as free-form `roles: Vec<String>` tags
(Decision 022 — no content migration), but are parsed into a typed `Role`
vocabulary: `Talkable`, `OathGiver`, `Shopkeeper`, `Combatant`, `Boss`
(`"conversable"` is a synonym for `talkable`; `fixture` is the
`EntityKind::Fixture` classification, not a role). Command handlers check
capability through `Entity::has_role(Role)` rather than ad-hoc string matches.

Role contracts are validated at the world construction boundary
(`WorldDefinition::validate`, Decision 030): a failed contract is a typed
`WorldValidationError::RoleContractUnmet { entity_id, role, missing }`, so broken
content fails fast. v1 contracts are the minimum each role needs *where applicable*
today:

- Every interaction role requires `EntityKind::Actor` (a Fixture carries none).
- `oath_giver` must additionally be named as some oath's `issuer_id`.
- `talkable` / `shopkeeper` / `combatant` / `boss` need no further metadata in v1;
  an optional `combat = { health }` is the future-combat hook, not required.

Unknown role tags are ignored (forward-compatible), so a new role can be authored
before it is typed. Roles are NOT serialized (the wire shape is unchanged); there is
no class hierarchy.

Rationale:

- Replaces brittle `roles.iter().any(|r| r == "…")` checks with one typed source of
  truth, and makes broken content (a shopkeeper on a fixture, an oath_giver wired to
  no oath) fail at construction instead of as a confusing runtime no-op.
- Keeps content additive and the engine free of a premature component/ECS hierarchy.

Revisit when:

- A role needs real required metadata (shop stock, full combat profile) — extend its
  contract in `validate`.
- Code-behind behavior dispatch lands (Decision 004's hook list).
- Strict unknown-tag rejection or per-kind role taxonomies become worthwhile.

## Decision 040: Combat v1 Is A Deterministic, Command-Driven Encounter With A Battle Modal

Status: Locked

Date: 2026-06-09

Implements the first combat loop (ticket #22) on the typed-role foundation
(Decision 039), deliberately small per `combat-system.md`: deterministic,
event-driven, and testable, without starting the class/skill/equipment systems.

Combat is **command-driven**, NOT the real-time server pulse of Decision 023
(that pulse loop is deferred). `attack` / `strike` / `fight` (optional target)
both starts an encounter and advances it; the engine resolves one full round per
command — the player strikes for a fixed `PLAYER_STRIKE_DAMAGE`, and a surviving
enemy returns its authored `CombatProfile.attack`. There is no RNG, so mutation
and regression tests stay strong.

Two additive, authored gates control combat (Decisions 007 / 004):

- `Role::Hostile` (tag `"hostile"`) marks an attackable entity; its contract
  requires a `CombatProfile` (a hostile must be fightable). An entity without the
  tag is not attackable — the Bell-Eater (`boss` + `combatant`, no `hostile`)
  stays `confront`-only (REQ-007).
- `RoomDefinition.combat_enabled: bool` (default false) opts a room into combat;
  `attack` is refused cleanly elsewhere. Region/subregion-level gating is future.

State and wire are additive (Decisions 028 / 031): `GameState.combat:
Option<CombatState>` holds the encounter and is cleared the moment it resolves;
`GameSnapshot.combat: Option<CombatSnapshot>` (skip-if-none) exposes a
side-tagged participant list (player + enemy in v1, multi-party-extensible); the
typed `CombatStarted` / `CombatEnded { outcome }` events bracket the per-strike
`CombatMessage` play-by-play on the existing `Combat` channel. Old saves and
combatless snapshots stay byte-identical.

Outcomes (death penalties out of scope): victory removes the defeated enemy from
its room (no corpse, no loot); defeat clears combat and revives the player at
full HP in place (Decision 008's penalty/relocation deferred). Enemy HP does not
persist across encounters.

UI: combat opens a focused battle modal (a `<dialog>` mirroring the room modal)
with the battle log on the left and participant state on the right; the client
opens it when `snapshot.combat` is present and closes it when the fight resolves,
leaving a compact summary in the server-rendered feed (Decision 034). The pure
`toBattle` view-model stays DOM-free (Decision 032).

Rationale:

- Gives the game its first repeatable danger loop while staying deterministic,
  additive, and fully covered (100% mutation MSI), and leaving the oath/boss flow
  untouched.
- Command-driven keeps v1 testable and avoids committing to the pulse engine
  before it is needed.

Revisit when:

- Real-time pulses (Decision 023), skills/equipment, AI tactics, loot, or death
  penalties (Decision 008) land — each extends `CombatState` / `CombatProfile`
  and the resolution without changing this foundation.
- Multi-party battles need the participant list populated beyond player + enemy.
