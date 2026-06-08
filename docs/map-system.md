# Map And Minimap System

Oathstar's map/minimap should start text-first but be designed as a square grid
that can later support richer visual rendering. The long-term direction is an
actual tile grid, not freeform room rectangles.

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

Canvas/grid target:

- Use square cells.
- Default tile size is 32x32 pixels.
- Keep tile size configurable so accessibility, zoom, and sprite-pack choices can
  use 8px, 16px, 32px, or larger tiles.
- Draw from renderer-agnostic JSON map data.
- Keep the first renderer small and first-party rather than adopting a full HTML5
  game engine.

Implemented (ticket #16 — canvas v1):

- The minimap is a first-party `<canvas id="map">` rendered by `client-app.js`'s
  thin `drawMapCanvas` seam, driven by a pure, unit-tested model in
  `src/client/canvas-map.js` (`canvasSize` for Hi-DPI backing-store sizing,
  `cellKind`, `toDrawPlan`, `mapAriaLabel`). No game-engine dependency.
- It consumes the renderer-agnostic JSON via `src/client/map.js` `toMapModel`
  (whose cell now carries `passable`); the server `/state` map payload is
  unchanged. Default 32×32 tiles, configurable through the client `mapRenderConfig`.
- Draws the current z-plane only; distinguishes unknown/empty, discovered,
  current, and blocked (discovered non-passable) cells; the room glyph is drawn
  on-tile and the room title is surfaced via the canvas `aria-label`.
- Hi-DPI: the backing store is `round(cssPx · devicePixelRatio)` with the context
  scaled to match, so the grid stays crisp on retina displays.

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

## Spatial Awareness

Once the grid carries entities and items, the player should perceive and reach
things by *distance* — noticing the `S` two cells west of `@` above — not only by
exact co-location. That "blast radius" model (sight radius, interaction radius,
structured awareness results) is specified in
[`docs/spatial-awareness.md`](spatial-awareness.md). It stays server-authoritative
and renderer-agnostic: the engine exposes nearby things as JSON on
`RoomSnapshot.contents`, never as canvas drawing instructions, and the canvas
entity/event overlays remain a future addition.

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

The v1 canvas renderer should be built in-house around the Oathstar grid model.
External libraries or engines can be reconsidered when the renderer needs sprite
batching, camera transforms, animation timelines, particles, or a map editor. For
now, a full game engine such as Kiwi.js is more coupling than value for a
server-authoritative MUD/grid surface.

## Design Guardrails

- Keep navigation readable.
- Avoid diagonal bloat.
- Support up/down for vertical spaces.
- Let room metadata define exits.
- Support passable/non-passable map cells.
- Keep the backend renderer-agnostic.
- Allow text, canvas, and sprite renderers to use the same map payload.
- Default to a 32x32 tile grid when canvas rendering begins.
