# Combat System

This document captures the current combat direction for Oathstar.

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
