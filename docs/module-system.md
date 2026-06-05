# Module System

Oathstar should support a modular system that lets the game grow into deep, strategy-scale complexity without overwhelming new players.

## Core Idea

The base game should be playable as a focused RPG/MUD experience:

- Explore
- Fight
- Level
- Complete oaths
- Finish the main game

Advanced systems should be opt-in modules.

Examples:

- Outpost building
- Trade
- Crafting
- Wars
- Politics
- Faction management
- Economy
- Settlement simulation
- DM tools
- LLM director tools
- Advanced survival
- Advanced region events

This lets a player start simple and later replay with more systems enabled.

## Complexity Philosophy

The game should be capable of Stellaris/Hearts of Iron levels of mastery, but not require that mastery on a first run.

Principles:

- Complexity should be discoverable.
- Advanced systems should be opt-in when possible.
- Modules should build on the event lifecycle.
- The base experience should stay coherent without every module enabled.
- The player should be able to choose a lightweight run or a dense simulation run.

## Module Types

Possible module categories:

- World modules
- Region modules
- Quest/oath modules
- Entity/item modules
- Combat modules
- Economy modules
- Crafting modules
- Outpost modules
- War/politics modules
- Class/transformation modules
- DM/director modules
- UI modules
- AI/LLM modules

## Module Manifest

Each module should eventually declare:

- Id
- Name
- Version
- Description
- Author
- Dependencies
- Conflicts
- Required game version
- Content registrations
- Behavior hooks
- Save migrations
- UI panels
- Settings

Example shape:

```json
{
  "id": "oathstar.outposts",
  "name": "Outposts",
  "version": "0.1.0",
  "description": "Adds outpost building and settlement logistics.",
  "author": "Oathstar",
  "category": "system",
  "requiredEngine": ">=0.1.0",
  "enabledByDefault": false,
  "dependencies": ["oathstar.core"],
  "conflicts": [],
  "content": {
    "regions": [],
    "rooms": [],
    "entities": [],
    "items": [],
    "skills": [],
    "oaths": [],
    "classes": [],
    "transformations": []
  },
  "hooks": ["onWorldLoad", "onTick", "onSave", "onLoad"],
  "settings": {}
}
```

## Module Hooks

Modules should attach to the event lifecycle.

Possible hooks:

- `onServerStart`
- `onWorldLoad`
- `onPlayerCreate`
- `beforeCommand`
- `afterCommand`
- `beforeAction`
- `afterAction`
- `onTick`
- `onScheduledEvent`
- `onSequenceStart`
- `onSequenceResolve`
- `onSequenceEnd`
- `onCombatStart`
- `onCombatRound`
- `onCombatEnd`
- `onOathSworn`
- `onOathFulfilled`
- `onOathBroken`
- `onRegionStandingChanged`
- `onSave`
- `onLoad`

The actual first hook list should stay small.

## State Mutation Rule

Modules should not freely mutate arbitrary game state.

Instead, modules should:

- Register content
- Observe events
- Submit change requests
- Call engine APIs
- Return command/action outcomes

The Rust core validates and commits state changes.

This keeps modules powerful without letting them corrupt saves or bypass core rules.

## First Implementation Boundary

First version:

- Official modules only
- Local file/module loading
- Manifest required
- Small hook list
- No hot-loading mid-save unless explicitly safe
- Core-validated state changes only

Later:

- Community modules
- Dependency resolution
- Conflict detection
- Save migration rules
- Sandboxing
- Module presets
- Module browser/manager

## Nice To Have: Oathscript

Oathscript is a possible future lightweight scripting language for Oathstar modules.

Goal:

Allow module authors to hook into lifecycle events without writing or compiling Rust.

Oathscript should be:

- Simple
- Deterministic
- Sandboxed
- Interpreted by Rust
- Limited to approved engine APIs
- Suitable for content behavior, not arbitrary system access

Possible uses:

- Simple item behavior
- NPC conditional responses
- Oath keep/break reactions
- Region event triggers
- Scheduled event scripts
- DM-authored quick events
- Tutorial/module logic

Example concept:

```text
on oath.fulfilled "town_mercy" {
  region.standing "beginner_town" +1
  say "The town bell rings once for mercy kept."
  unlock exit "town_square" "north"
}
```

Oathscript should not replace Rust behavior modules for complex systems. It should be a safe scripting layer for common lifecycle hooks.

Design guardrails:

- No file system access by default
- No network access by default
- No arbitrary code execution
- Engine validates all state changes
- Scripts are versioned with modules
- Script errors should fail safely and report clearly

## New Game Module Selection

New games can offer module presets.

Possible presets:

- Story/Combat: core RPG experience
- Adventurer: core plus crafting/trade
- Strategist: outposts, politics, wars, economy
- Dungeon Master: enables DM controls
- Full Simulation: everything enabled
- Custom: choose modules manually

This lets the same game serve very different play styles.

## Dungeon Master And Module Control

The dungeon master can eventually control modules on the fly.

Possible powers:

- Enable or disable certain event chains
- Trigger module events
- Build or grant skills
- Spawn political crises
- Adjust war state
- Create custom oaths
- Modify faction/region standing
- Advance or pause sequences

The Rust core should validate these changes.

## Design Guardrails

- The core game must be fun without advanced modules.
- Modules should not silently break saves.
- Module choices should be visible at game start.
- Advanced systems should introduce themselves through play.
- Module hooks should not become a chaotic free-for-all.
- Open-source modules need clear contracts and versioning.

## Open Questions

- Should modules be enabled only at new game start, or can some be toggled mid-save?
- Should official modules be bundled separately from community modules?
- Should modules be data-only at first?
- How strict should module sandboxing be?
- How do module conflicts get reported to the player?
