# Map And Minimap System

Oathstar's map/minimap should start text-first but be designed as a square grid that can later support richer visual rendering.

## Core Direction

The world map should use a square grid.

Supported directions:

- North
- South
- East
- West
- Up
- Down

Avoid diagonal directions such as northwest, northeast, southwest, and southeast for the core game. Diagonals can make navigation more overwhelming and less MUD-readable.

## Room Metadata

Rooms define their map position and exits through metadata.

Room metadata can include:

- Region
- Subregion
- X coordinate
- Y coordinate
- Z coordinate or level
- Exits
- Passable/collision flag
- Visibility/discovery state
- Map glyph
- Optional sprite/tile reference later

Example:

```toml
id = "town_square"
title = "Town Square"
region = "beginner_town"
subregion = "center"
x = 0
y = 0
z = 0
glyph = "+"
passable = true

[exits]
north = "north_gate"
east = "market_lane"
up = "bell_tower"
```

## Rendering Direction

First version:

- Text/ASCII grid
- Current room marker
- Discovered room markers
- Region/subregion labels
- Basic up/down indication

Later:

- Canvas renderer
- Tile sprites
- Room icons
- Region overlays
- Fog of war
- Enemy/event markers
- DM/debug overlays
- Denser ASCII/tile layouts with walls and walkable spaces

## Passability And Collision

Rooms or map cells should declare whether they are passable.

Passable means the player can occupy or move into that grid cell.

Non-passable cells can represent:

- Walls
- Cliffs
- Locked barriers
- Dense forest
- Water
- Rubble
- Shop walls
- Decorative map structure

This lets the map evolve beyond classic MUD room nodes into denser ASCII or tile-based spaces.

Example:

```text
#######
#..S..#
#..@..#
#######
```

In this example:

- `#` cells are non-passable walls.
- `.` cells are passable floor.
- `S` is a shopkeeper/entity.
- `@` is the player.

A shop might eventually contain one or two passable interior cells surrounded by non-passable wall cells. This keeps the current room/grid model compatible with a future top-down game representation.

## Backend Payload

The backend should expose map state as JSON.

The frontend decides how to render it.

Example payload shape:

```json
{
  "region": "beginner_town",
  "subregion": "center",
  "currentRoom": "town_square",
  "rooms": [
    {
      "id": "town_square",
      "title": "Town Square",
      "x": 0,
      "y": 0,
      "z": 0,
      "glyph": "+",
      "discovered": true,
      "current": true,
      "exits": {
        "north": "north_gate",
        "east": "market_lane",
        "up": "bell_tower"
      }
    }
  ]
}
```

## Canvas Consideration

It may be beneficial to implement the minimap as canvas early, even if the first renderer draws text-like tiles.

Benefits:

- Easy transition to sprites later
- Stable grid dimensions
- Better performance for overlays
- Less DOM churn
- Useful for DM/debug views

Guardrail:

The map data should remain renderer-agnostic. The server should send structured JSON, not frontend-specific canvas instructions.

## Design Guardrails

- Keep navigation readable.
- Avoid diagonal bloat.
- Support up/down for vertical spaces.
- Let room metadata define exits.
- Support passable/non-passable map cells.
- Keep the backend renderer-agnostic.
- Allow text, canvas, and sprite renderers to use the same map payload.
