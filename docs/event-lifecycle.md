# Event Lifecycle And Time

Oathstar should use an event-driven Rust server with a clear lifecycle for commands, ticks, scheduled events, modules, and future DM/LLM systems.

## Core Direction

The server is authoritative.

Everything meaningful should flow through events:

- Player commands
- NPC actions
- Combat actions
- Item behavior
- Oath changes
- Region standing changes
- Scheduled world events
- DM interventions
- Future LLM-generated proposals

## Base Tick

The game should support a regular world tick.

Initial target:

- 1 second base tick
- Configurable per server/world if needed
- Manual tick mode for debugging and DM control

One second is fast enough to keep the world feeling alive without forcing every system to operate at twitch-game speed.

## Tick Responsibilities

Ticks can drive:

- Regeneration
- Timed effects
- Combat round timers
- NPC patrols
- Region hazards
- Scheduled events
- Spawn/respawn logic
- Oath timers, if added later
- Autosave checks

Not every system needs to run every tick. Systems can subscribe to the ticks they care about.

## Command Lifecycle

Suggested command flow:

1. Receive command text or API action.
2. Parse command into intent.
3. Validate intent.
4. Run `beforeCommand` hooks.
5. Resolve action.
6. Run relevant entity, item, room, region, oath, and combat hooks.
7. Commit state changes.
8. Emit events.
9. Render or stream UI updates.
10. Save/autosave if needed.

## Event Phases

Possible phases:

- `beforeParse`
- `afterParse`
- `beforeCommand`
- `beforeAction`
- `afterAction`
- `beforeStateCommit`
- `afterStateCommit`
- `beforeRender`
- `afterRender`
- `onTick`
- `onScheduledEvent`
- `onSequenceStart`
- `onSequenceResolve`
- `onSequenceEnd`

The first implementation should use fewer phases and add more only when needed.

## Scheduled Events

The world should support scheduled events.

Examples:

- At tick 600, a patrol reaches the gate.
- After 3 in-game hours, the well floods.
- On the next dawn, a broken oath is judged.
- On the Nth hour, a town attack begins.
- After enough regional unrest, a boss moves.

Scheduled events can be authored, module-driven, DM-triggered, or eventually AI-proposed.

## Time Sequences

Some events are too large or slow to resolve inside the normal tick loop.

Oathstar should support time sequences: special moments where the world can pause or slow, resolve larger decisions, apply a batch of events, and then resume ticking.

Examples:

- A town siege begins.
- A dungeon master triggers a regional crisis.
- A scripted chapter turn resolves.
- An LLM director proposes a set of world changes.
- A multiplayer party reaches a major decision point.

Sequence flow:

1. Pause or suspend normal ticks.
2. Gather needed decisions.
3. Human DM, scripted director, or LLM director proposes actions.
4. Rust core validates proposed actions.
5. Approved events are committed.
6. Players receive narration/UI updates.
7. Normal ticking resumes.

## LLM Boundary

LLMs should not be responsible for per-tick actions.

LLMs are too slow and unpredictable for tick-critical simulation.

Better use:

- Propose scheduled events
- Generate narration drafts
- Assist a DM
- Suggest NPC strategic moves
- Help parse unusual player intent into safe commands
- Prepare plans during sequence pauses

The Rust core remains the authority. LLMs propose; the core validates and applies.

## Dungeon Master Modes

A future dungeon master can be:

- Automatic scripted director
- Human director
- LLM-assisted director
- Fully LLM-scripted director, if safe enough later

Possible DM controls:

- Pause/resume ticks
- Advance ticks manually
- Trigger scheduled events
- Spawn encounters
- Move NPCs
- Award items, XP, standing, or oaths
- Send narration
- Approve unusual actions
- Override event timing

## Multiplayer Shape

In multiplayer, the server tick gives the world a shared clock.

Players can:

- Explore individually
- Grind in different regions
- Work on personal quests
- Join party objectives
- Be called back by scheduled crises

Example:

Five players spread out to grind and explore. On the Nth in-game hour, the director triggers a town-destruction event. The server streams the alert, region state changes, and new objectives. Players can return, ignore it, or suffer consequences.

## Design Guardrails

- Keep the first event lifecycle small.
- Do not block normal ticks on LLM calls.
- Use sequences for slow strategic decisions.
- Validate every DM/LLM action through the Rust core.
- Make tick interval configurable.
- Keep single-player working without DM, LLM, or multiplayer.
