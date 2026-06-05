# First Vertical Slice: Beginner Module

The first proof of concept should be a small, complete beginner module.

It should prove the core systems without trying to be the full game.

## Working Shape

The module starts in a local town.

The player receives or discovers a simple oath tied to the town.

The player explores nearby areas, fights/grinds, learns basic commands, gains initial progression, and eventually reaches a tower or similar dungeon-like objective.

The module ends with a boss encounter.

## Goals

The Beginner module should prove:

- Rust server/runtime architecture
- Command parser
- Rooms, regions, and subregions
- Entities and code-behind behavior
- Basic NPC memory
- Oath lifecycle
- Combat pulse timing
- Basic grinding
- Inventory/equipment
- Skill percentage progression
- Region standing
- Save/load
- SSE event streaming
- Tauri-managed local play

## Scope

Suggested scope:

- 1 town region
- 1 nearby combat/outside region
- 1 tower/dungeon region
- 8-15 rooms
- 2-4 NPCs
- 3-6 regular enemy types
- 1 boss
- 1 simple oath that can be fulfilled or broken
- 1-2 shops or trainers if time allows
- Basic equipment and loot
- Basic level/skill progression

## Example Flow

1. Player starts in a local town.
2. A town figure offers a simple oath.
3. Player explores the town and learns commands.
4. Player goes outside to fight and gather resources.
5. Player improves basic combat skills through use.
6. Player reaches a tower.
7. Player resolves the tower boss through combat or an alternate oath-aware route.
8. Town standing changes based on the outcome.
9. The next module hook becomes available.

## Expansion Hook

The Beginner module should end by opening the door to the broader Oathstar game.

Possible expansion hooks:

- A road opens beyond the town.
- A regional map unlocks.
- A higher oath is witnessed.
- A messenger arrives.
- The tower reveals the first true Oathstar clue.
- A new module becomes selectable or enabled.

## Design Guardrails

- Keep the module complete and finishable.
- Do not include every advanced system.
- Keep the oath simple but real.
- Include enough combat to feel the progression loop.
- Use TOML-first content if the Rust rewrite has begun.
- Treat this as the engine proving ground.
