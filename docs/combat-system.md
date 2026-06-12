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
> `combat_enabled`). See `decisions.md` Decision 040.

> **v2 implemented** (ticket #24): the **server-authoritative pulse loop**
> (Decision 023) now drives an active encounter in real time, layered on the v1
> foundation — see `decisions.md` Decision 042. The combat pulse rides the 1s
> world tick (`Engine::tick()`; default cadence 2 ticks ≈ 2s, per-encounter
> `pulse_rate` representable): each due pulse emits a typed
> `CombatPulse { round }` marker, resolves **Phase 1** (the baseline exchange —
> the v1 round: player auto-strike + the enemy's authored return), then
> **Phase 2** (the skill window): a queued between-pulse action resolves, or
> the phase skips cleanly. `flee` is the v2 queued action — it resolves at the
> next pulse boundary into `CombatEnded { fled }` (the enemy survives in
> place; the player keeps current HP), and **leaving the encounter room
> disengages as fled** (no remote pulsing). Manual `attack` between pulses
> still resolves a round immediately without moving the cadence. The engine
> stays wall-clock-free — the tick stream is its only clock, so pulses are
> deterministic and test-driven; real time exists only in the server's tokio
> interval (missed ticks are skipped, never burst). The full combat-state
> machine, authored skills content, per-actor/boss pulse tuning, and alternate
> resolutions below remain the long-horizon direction.

> **Direct battle verbs v1 implemented** (ticket #25): the Phase-2 window's
> first real actions — see `decisions.md` Decision 043. During battle the
> player types **direct verbs**, never `skill <name>`: `guard` (a one-shot
> charge armed at the window that turns the next enemy return — pulse or
> manual round — aside entirely) and `power strike` (a fixed heavier blow of
> 6 in the window; a kill there ends the fight in victory). Verbs queue
> between pulses through the same mechanism as `flee`: re-queueing the same
> action is a no-op with its own line, and queueing a different action
> **replaces** the queued one ("You change tack. …") — one deterministic rule
> across all actions. Outside combat every battle verb refuses cleanly. The
> battle modal exposes Power Strike / Guard / Flee buttons beside Attack and
> shows the queued action while it waits for the boundary. Skills content
> and cooldowns remain out of scope below.

> **Focus economy v1 implemented** (ticket #31): the #25 skills now cost
> focus — see `decisions.md` Decision 049. Engine-const prices (`power
> strike` 2, `guard` 1; `attack`/`flee` stay free), **spent at the queue** —
> the moment of commitment — with a typed System-channel refusal when the
> pool can't cover it ("You lack the focus for a power strike." / "…to
> guard."), which changes no state. Change-tack settles deterministically:
> the replaced action's cost refunds before the new one charges (no
> double-spend, no free-cancel gain), and a fight that ends with an action
> still queued refunds the unfired cost on every outcome. **Victory and
> flee keep the spent pool — defeat restores it with HP** ("battered but
> whole"). Recovery is the `rest` verb: out of combat it refills focus to
> max in one breath; mid-encounter it refuses ("There is no rest in the
> midst of battle."). All arithmetic saturates, so crafted saves at i32
> extremes load and play without panicking; a 5-point pool affords exactly
> two power strikes and a guard. Warding/rituals, focus items, max-focus
> growth, and authored skill economies stay future work.

> **Rewards + defeat consequences v1 implemented** (ticket #26): winning and
> losing now matter — see `decisions.md` Decision 044. **Victory** awards the
> defeated hostile's authored `CombatProfile.xp` (default 0 — an unrewarded
> win is byte-identical to before) and drops its authored `inventory` into
> the room as takeable ground items ("The Ashen Stray drops Cracked Fang."),
> clearing it for the session — no corpses, no duplicate drops, and a fled
> enemy keeps its spoils for the eventual win. **Defeat** resets the player
> to the world start room at full HP, clears combat, leaves the enemy in
> place, and applies the deterministic XP penalty `max(1, floor(xp/10))`
> (never below zero; nothing at zero XP) — the summary names the wake room
> and the loss, and the feed narrates the arrival with the normal room
> entry. Everything rides the existing combat/feed surfaces (no new event
> kinds); all of it deterministic, exactly once, through the single
> `end_combat` funnel. Levels, currency, loot tables, respawn, and richer
> death penalties remain the long-horizon direction below.

> **The boss encounter is real** (ticket #29) — see `decisions.md`
> Decision 047. `confront` (sworn oath + boss present) now STARTS the same
> pulse-loop encounter the ambient hostiles use, through the shared
> `engage_enemy` entry — one combat model behind two gates. The gates carry
> distinct semantics: `combat_enabled` + the `hostile` role gate ambient
> `attack`; the **sworn oath** gates the boss (`confront` bypasses the room
> flag, the boss is deliberately NOT `hostile`, and `attack <boss>` stays
> refused — no ungated path around the oath). The `boss` role contract now
> requires a combat profile (like `hostile`); mid-fight `confront` presses
> the attack like `attack` does. **Victory does not fulfill** — the boss
> falls through the #26 funnel (removal, drop, authored xp 25) and the oath
> stays sworn until the player RECOVERS the oath's authored objective
> (`OathDefinition.objective_item_id`, validated): taking it while sworn
> emits `OathFulfilled` + the #27 announcements. Defeat is the #26 path
> (reset, penalty) with the boss, its inventory, and the sworn oath intact
> — climb back up and try again. Boss phases, special moves, and alternate
> resolutions (persuade/spare/bind) remain the direction below.

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

Implemented cadence (ticket #24 — the two-phase cycle below is live; authored
skills content, variable per-actor cadence, and `paused_sequence` are still to
come):

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
