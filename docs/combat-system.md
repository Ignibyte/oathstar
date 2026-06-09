# Combat System

This document captures the current combat direction for Oathstar.

> **v1 implemented** (ticket #22): a deliberately small, deterministic,
> command-driven battle loop. `attack` / `strike` / `fight` engages a hostile
> (`Role::Hostile`; its contract requires a `CombatProfile`) in a
> `combat_enabled` room; the engine resolves one round per command — fixed player
> damage plus the enemy's authored return strike, no RNG — ending in victory
> (the enemy is removed) or defeat (the player revives at full HP; death
> penalties deferred) and clearing combat state. Typed `CombatStarted` /
> `CombatEnded { outcome }` events bracket the `CombatMessage` play-by-play on the
> `Combat` channel, and the client opens a battle modal (left log / right
> participants) that closes to a compact feed summary. The boss/oath `confront`
> flow is unchanged (the Bell-Eater is not `hostile`, and its roost is not
> `combat_enabled`). v1 is **command-driven, NOT the server pulse** described
> below (Decision 023, deferred) — see `decisions.md` Decision 040. The pulse
> timing, the full combat-state machine, combat roles, and alternate resolutions
> below remain the long-horizon direction.

## Core Direction

Combat is a core system, not a rare edge case.

The game should support fast, repeatable battles for grinding and progression, while still reserving major boss encounters for more authored, memorable, multi-solution conflicts.

## Combat Boundaries

Not every actor can be killed.

Combat should be controlled by:

- Region rules
- Room attributes
- Actor roles
- Actor state
- Oath constraints
- Story gating

This means the player should not be able to murder every shopkeeper, witness, quest-giver, or civic remnant by default.

Combat is expected primarily in:

- Outside regions
- Wild zones
- Ruined districts
- Specific hostile rooms
- Encounter routes
- Boss arenas

Non-combat or protected regions can exist, such as:

- Markets
- Courts
- Sanctuaries
- Oath halls
- Settlements
- Important NPC rooms

## Battle Feel

Normal battles should be:

- Quick
- Frequent in the right areas
- Low-friction
- Useful for grinding
- Readable in text
- Easy to recover from

## Combat Timing

Combat runs on server-authoritative pulses layered over the base world tick.

Target follow-up cadence (ticket #24):

- Combat is a repeating two-phase cycle until someone flees or dies.
- Phase 1 resolves the initial exchange: baseline player/hostile hits and
  immediate authored reactions.
- Phase 2 is the skill window: if the player has queued or entered a skill, it
  resolves; if not, the skill phase is skipped cleanly.
- After Phase 2, the next cycle begins unless combat has ended through victory,
  defeat, flee, or a future alternate resolution.

Initial timing:

- World tick: 1 second
- Default combat pulse: 2 seconds

On each combat pulse:

- Engaged actors can auto-attack.
- Combat skills can trigger.
- Defensive checks can resolve.
- Timed effects can tick.
- Combat events stream to clients.

Manual commands can be submitted between pulses, such as:

- `flee`
- `use potion`
- `cast ward`
- `switch target`
- `spare shade`
- `persuade guard`
- `bind oath`

The combat pulse can vary by actor, skill, effect, or region:

- Fast enemies may act every 1 second.
- Slow heavy enemies may act every 3 seconds.
- Bosses may use scripted timings.

Boss fights, DM interventions, or major events can pause normal combat into a time sequence, resolve special actions, then resume pulses.

Possible combat states:

- `not_in_combat`
- `engaged`
- `round_pending`
- `resolving_round`
- `victory`
- `defeat`
- `fled`
- `paused_sequence`

Boss battles should be:

- More authored
- Mechanically distinct
- Connected to oaths, regions, or story
- Capable of multiple resolution paths when appropriate

## Resolution Paths

Even though combat is core, not every goal should require killing.

Possible resolution paths:

- Defeat through battle
- Persuade
- Bind with an oath
- Spare after weakening
- Use a specific item
- Exploit a region rule
- Fulfill or break a related oath
- Bargain with an enemy

This lets combat remain important without flattening every conflict into hit points.

## Combat Roles

Combat should use actor roles rather than a separate enemy model.

An actor can be:

- Non-combatant
- Combatant
- Hostile combatant
- Boss
- Protected
- Sparable
- Persuadable
- Bindable

Combat metadata might include:

- Health
- Attack profile
- Defense/resistances
- Rewards
- Behavior pattern
- Boss phase data
- Allowed resolution paths
- Defeat outcome
- Kill/spare/bind consequences

## Grinding

Grinding is expected to be part of the game.

Grinding should reward:

- Currency
- Materials
- Reputation
- Focus/health recovery resources
- Oath-related resources
- Regional progress
- Practice with combat verbs

Open design questions:

- Should enemies respawn by region, by rest, by travel, or by explicit patrol systems?
- Should grinding reward XP, skill points, currency, materials, or some other progression currency?
- How do we prevent grinding from undermining oath consequences?
- Should combat difficulty scale by region, player progression, or authored encounter tier?
