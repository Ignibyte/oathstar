# Technical Architecture

This document captures the emerging technical direction for Oathstar.

## Current Direction

Oathstar may become an open-source, module-friendly Tauri game with a Rust-first core and a simple hypermedia frontend.

The preferred shape is:

- Standalone Rust game server/runtime
- Tauri desktop shell that manages the local server for normal play
- Rust game core
- Event lifecycle around commands and world changes
- Module/behavior registry for expansion
- Datastar-style frontend if it proves practical
- SSE streams for UI events and DOM/state updates
- Optional local AI hooks later

The first playable target should still be a strong single-player authored experience. The architecture should avoid closing the door on broader modes such as multiplayer, dungeon-master control, LLM-assisted sessions, or swappable worlds.

## Open Source Direction

The project may be open sourced as a fun, expandable game/engine.

Design implications:

- Favor clear contracts.
- Favor deterministic core systems.
- Keep content data inspectable.
- Keep behavior modules registered and testable.
- Avoid source license contamination from reference codebases.
- Make extension points explicit.

## Event Lifecycle

The game should eventually have a full event lifecycle.

The event/tick direction is defined in [Event Lifecycle And Time](./event-lifecycle.md).

Example command lifecycle:

1. Player submits command.
2. Parser turns text into a command intent.
3. Engine validates intent.
4. Region, room, entity, item, oath, and actor hooks can observe or modify the intent.
5. Engine resolves the command.
6. State changes are applied.
7. Events are emitted.
8. UI receives rendered patches or structured state updates.
9. Save/autosave can persist the new state.

Possible event phases:

- `beforeParse`
- `afterParse`
- `beforeCommand`
- `beforeAction`
- `afterAction`
- `beforeStateCommit`
- `afterStateCommit`
- `beforeRender`
- `afterRender`
- `onTurn`

The exact lifecycle should stay small until real systems need more phases.

The server should support a base world tick, scheduled events, and sequence pauses for larger decisions. LLM and DM systems should use scheduled events or time sequences rather than per-tick authority.

## Module System

Modules can eventually provide:

- Regions
- Rooms
- Entities
- Items
- Battle systems
- Stat models
- Progression models
- Inventory/equipment models
- Skills
- Oaths
- Classes
- Transformations
- Behavior handlers
- UI panels
- Renderers
- Event hooks

The module direction is described in [Module System](./module-system.md).

Modules should declare:

- Id
- Version
- Dependencies
- Content registrations
- Behavior registrations
- Optional migrations

Modules should also support optional complexity. A new game can enable only the core experience or opt into advanced modules such as crafting, trade, outposts, politics, wars, economy, DM tools, or LLM directors.

Modules should use core-validated state changes. They can register content and hooks, but the Rust core should validate and commit mutations.

Future module authors may get a lightweight interpreted scripting layer called Oathscript for simple lifecycle hooks without compiling Rust. Oathscript should remain sandboxed and core-validated.

Long term, Oathstar should support swappable rule-system modules. The core
runtime should avoid baking in one permanent combat, stat, progression, or
inventory model. Instead, the runtime should expose typed contracts and module
presets that choose compatible implementations for a given world or campaign.
The first implementation can ship one official ruleset, but it should keep the
kernel small enough that alternate rulesets remain possible.

## Datastar And SSE Frontend Direction

Datastar is a candidate frontend technology because it keeps the UI simple and backend-driven.

Possible shape:

- Rust core owns game state.
- A small Rust HTTP/SSE layer exposes game endpoints.
- Datastar components submit commands through `data-*` attributes.
- Rust returns HTML patches or SSE event streams.
- The browser UI morphs updated panels, logs, maps, inventory, and combat state.

This fits:

- Command log streaming
- Fast battle updates
- Inventory/equipment panel patches
- Region standing updates
- Oath state changes
- Debug/event inspector panels

In Tauri, this likely means running a local Rust HTTP server bound to loopback, or another Tauri-compatible bridge that can support SSE semantics.

## Rust Core Boundary

The core engine and runtime should not depend directly on Datastar or Tauri.

Recommended layering:

- `oathstar-core`: deterministic game engine, parser, state, event lifecycle
- `oathstar-protocol`: shared API/event/save DTOs used by server and clients
- `oathstar-server`: standalone Rust runtime, API, event streams, saves, module loading
- `oathstar-storage`: file-first persistence and future database abstraction
- `oathstar-content`: built-in content/modules/worlds
- `oathstar-tauri`: desktop shell that launches/manages/connects to the local server
- `oathstar-web`: Datastar/HTML render layer
- `oathstar-ai`: optional local AI integration later

This keeps the game open-source friendly and easier to test.

## Cargo Workspace Direction

Oathstar should use a Cargo workspace.

Likely shape when we pull the implementation trigger:

```text
crates/
  oathstar-core/
  oathstar-protocol/
  oathstar-server/
  oathstar-storage/
  oathstar-content/
  oathstar-ai/

apps/
  tauri/
  web/

modules/
  beginner/
  oathstar-core-world/
```

The current prototype can remain available while the workspace is introduced. The migration should move engine authority into Rust first, then replace the prototype frontend with a client against the local server.

## Development Workflow

Architecture docs should stay the source of truth.

Coding agents and CLI tools can help execute the implementation, but they should work from:

- `docs/decisions.md`
- system-specific docs such as combat, map, modules, progression, and events
- crate-level README files once the workspace exists

This keeps implementation work aligned even if multiple tools or agents contribute code.

## Server/Client Boundary

The Rust server/runtime is the game authority.

The server owns:

- Game state
- Rules
- Saves
- Combat
- Oaths
- Skills
- Inventory
- Region standing
- Module loading
- Event lifecycle

Clients send commands and render results.

Possible clients:

- Tauri local player app
- Browser client
- CLI client
- DM console
- Debug/admin tools
- Future multiplayer client

For normal local play:

1. Player opens the Tauri app.
2. Tauri starts or connects to a local `oathstar-server`.
3. The frontend sends commands to the server.
4. The server resolves actions and streams events.
5. The frontend renders the game.

## API And Transport Direction

Oathstar should expose a local API from the Rust server.

The server framework direction is Axum on Tokio.

Protocol/output direction is hybrid: typed domain events can be exposed as JSON, rendered as Datastar/HTML fragments, or reduced to plain text. More detail lives in [Protocol And Output](./protocol-and-output.md).

Preferred first shape:

- REST-style HTTP endpoints for commands, saves, settings, and snapshots
- SSE for server-to-client event streams

Possible first endpoints:

- `POST /command`
- `GET /state`
- `GET /events`
- `POST /save`
- `POST /load`
- `GET /worlds`
- `POST /worlds/select`

SSE should be first-class because it fits game log/event streaming and Datastar-style UI updates.

WebSockets can be added later if we need:

- Bidirectional realtime channels
- Multiplayer rooms
- Live DM control
- Chat
- Low-latency combat streams
- Multiple simultaneous clients

Initial stance:

- REST + SSE first.
- WebSockets later when justified.
- Design the event model so it can be transported over SSE or WebSockets.

## Persistence Direction

Start file-based.

File-based persistence fits:

- Local-first play
- Open-source simplicity
- Easy save inspection
- Easy modding and debugging
- Versioned JSON snapshots

Likely files:

- Save files
- Settings/config
- World/module manifests
- Logs
- Debug snapshots

A database can be introduced later if needed for:

- Multiplayer server state
- Many users/accounts
- Event history
- Analytics/debug timelines
- Large mod indexes
- Cloud sync

Initial stance:

- File-first for local single-player.
- Keep storage behind an interface so SQLite or another DB can be added later.

## Optional LLM Direction

Open sourcing and modularizing the game makes optional LLM integration easier later.

Possible uses:

- Parser assistance
- NPC flavor variation
- Lore lookup
- Content authoring tools
- Modding assistant

Guardrail:

LLM output should not be the source of truth for deterministic game state unless a later design deliberately chooses that risk.

## Long-Horizon Direction: Core Engine, Swappable Worlds, And DM Layer

Oathstar can be designed as a core engine plus world modules.

In the near term, this means the built-in Oathstar world is just the first official world. In a larger future, the same engine could run different worlds, campaigns, or rule packs.

Possible layers:

- Core engine: commands, event lifecycle, actors, rooms, items, combat, saves, skills, oaths, alignment
- World module: regions, rooms, entities, quests/oaths, lore, class recipes, transformations, items
- Runtime mode: single-player, local hosted campaign, multiplayer server, or DM-assisted session
- Presentation layer: Tauri desktop UI, web client, debug tools, DM console
- Optional AI layer: parser assist, NPC variation, lore lookup, DM assistant, worldbuilding tools

## Dungeon Master Mode

A future DM mode could let a human or AI-assisted operator influence the world.

Possible DM powers:

- Spawn encounters
- Trigger events
- Adjust NPC disposition
- Award items, XP, standing, or oaths
- Send narration
- Open or close routes
- Override or approve unusual player actions
- Inspect party state and region state

The important boundary:

The deterministic Rust core should still validate and apply state changes. The DM layer proposes or authorizes changes; it should not bypass the core state model.

## Multiplayer Possibility

Multiplayer would push Oathstar closer to actual MUD territory.

Possible multiplayer shapes:

- Shared world server with multiple players
- Small party/co-op campaign
- DM plus players
- Local network/private server
- Asynchronous region/world state sharing

This would require:

- Server-authoritative Rust core
- Account/session model
- Concurrent command handling
- Shared room state
- Chat/say/tell channels
- Party/group mechanics
- Multiplayer-safe saves
- Clear rules for time, turns, combat, and instancing

This should remain later-later. It is architecturally exciting, but it can easily consume the whole project before the single-player game exists.

## Swappable Worlds

Worlds should eventually be replaceable or selectable.

Examples:

- Oathstar official campaign
- A smaller test/tutorial world
- Community campaign
- High-fantasy MUD-like world
- Sci-fi/tech-infused world
- Horror/vampire transformation world

World modules could provide:

- Regions and rooms
- Oaths/quests
- Entities and items
- Skills and class recipes
- Transformations
- Elemental/aspect rules
- Dialogue and lore
- UI labels or theme hints

This reinforces the need to keep core engine rules separate from Oathstar-specific lore.

## Open Questions

- Should the production Tauri app run an embedded loopback HTTP server?
- Should the frontend be fully Datastar, or Datastar only for selected panels?
- What is the minimum event lifecycle we need for the first Rust rewrite?
- How should third-party modules be loaded and sandboxed?
- Do we want a formal plugin API before the first vertical slice?
- Does DM mode require multiplayer, or can it exist first as a single-player debug/storyteller console?
- Should world modules be data-only at first, or allowed to ship behavior code?
