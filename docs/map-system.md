# Map And Minimap System

Oathstar's map/minimap should start text-first but be designed as a square grid
that can later support richer visual rendering. The long-term direction is an
actual tile grid, not freeform room rectangles.

## Core Direction

The world map should use a square grid.

Supported directions (cardinal only):

- North
- South
- East
- West

Avoid diagonal directions such as northwest, northeast, southwest, and southeast for the core game. Diagonals can make navigation more overwhelming and less MUD-readable.

**Movement is 2D (ticket #52, amends Decision 025).** Up/down were retired:
vertical traversal is expressed as cardinal movement plus **warps**. A warp is a
cardinal exit whose target room lies in a different region or sub-region — the
engine moves the player there and narrates the crossing (`"You enter <name>."`).
The `z` coordinate is retained only for tile-**layer** visual stacking (#47/#48),
never for movement or spatial awareness.

## Room Metadata

Rooms define their map position and exits through metadata.

Room metadata can include:

- Region
- Subregion
- X coordinate
- Y coordinate
- Z coordinate (tile-layer visuals only; not movement — ticket #52)
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
west = "old_road"   # a cardinal exit into another region/sub-region is a "warp"
```

## Rendering Direction

First version:

- Text/ASCII grid
- Current room marker
- Discovered room markers
- Region/subregion labels
- Region/sub-region transition cues (warps)

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

Implemented (sprite tiles, Decisions 050 + 059):

- Map cells draw as sprite tiles blitted from an **author-provided tile sheet**
  (the contract: `docs/tileset-contract.md` — a grid PNG + a JSON descriptor
  with `tileSize`/`columns`/`rows`/`image`/`tiles[{name,x,y}]`). The model
  accepts 8/16/32px sheets (`SUPPORTED_TILE_SIZES`); 8px-native is the
  direction. There is no generated sheet or generator script — real art is
  author-provided (Decision 059, retiring the blank-colors generator).
- A pure module `src/client/tileset.js` validates the descriptor (typed
  `{ok}|{ok:false,reason}` result, never throws, integer-only geometry,
  prototype-safe name lookup) and resolves cell kinds to sheet rects by
  **name** (`shadow_void`/`stone_floor`/`wall_face`/`spawn_marker`).
- `toDrawPlan(model, tileset)` ops carry a `sprite` source rect alongside the
  flat-color palette fields: one plan serves both modes, and the seam falls
  back to flat fills with no author sheet (`VITE_OATHSTAR_TILESET` unset) or on
  any metadata/image failure — warn once, never a blank map.
- Pixel-art crispness: `imageSmoothingEnabled = false` per draw + CSS
  `image-rendering: pixelated`; 16px source tiles scale to the configured
  `tilePixels`. The per-kind stroke ring and on-tile glyph draw over the tile.
- Authored per-room/region tile assignment and per-tile description metadata
  are the reserved next step (intake:
  `INTAKE-tileset-region-authoring-per-tile-metadata`).

Implemented (ticket #33 — entity/item presence markers, Decision 051):

- `MapRoomSnapshot` carries two additive, server-computed flags —
  `hasHostiles` (a live, non-hidden `Role::Hostile` placement) and
  `hasItems` (live, non-hidden ground items) — gated on `discovered`, so
  fogged rooms emit neither key and the payload never leaks concealed
  state. Both follow the omit-when-false pattern (marker-less payloads
  stay byte-identical).
- The client draws presence dots over the cell (ember `#ec682b` hostile,
  top-right; gold `#e6c85a` items, bottom-right; dark outline, drawn over
  tile/stroke/glyph). Geometry is pure (`toDrawPlan` op `markers`), the
  seam just executes arcs; `mapAriaLabel` voices nonzero counts
  ("hostiles in N rooms, loot in M rooms").
- The `hidden` reveal rule (#17) applies to the map exactly as to
  `look`/nearby: hidden things never flag a room.

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
        "west": "old_road"
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
- Movement is 2D cardinal-only; cross-region cardinal exits ("warps") replace vertical traversal.
- Let room metadata define exits.
- Support passable/non-passable map cells.
- Keep the backend renderer-agnostic.
- Allow text, canvas, and sprite renderers to use the same map payload.
- Default to a 32x32 tile grid when canvas rendering begins.

## Map Document Model (authoring)

Ticket #43 adds a first-party **map document model** (`oathstar-content`'s
`MapDocument`) — the canonical artifact the Studio `/admin/editor` edits, distinct
from the runtime [`MapSnapshot`](#backend-payload) wire DTO. It is renderer-agnostic
and serde-serializable (JSON drafts), authored on a **source-tile grid** whose unit
is `tile_size` ∈ `SUPPORTED_TILE_SIZES` (8/16/32px; 8px-native — Decision 059). (The
on-screen render scale — 64px cells — is a separate client concern; the name-keyed
tileset keeps source resolution independent of map/world data.)

Shape:

- `tile_size` (one of `SUPPORTED_TILE_SIZES` — 8/16/32, 8px-native), and
  `width` / `height` / `floors` grid bounds.
- a name-keyed `terrain_palette` (`name → { tile, passable }`) and a sparse `terrain`
  layer (a `Vec` of `{ x, y, z, terrain }` cells).
- `rooms`: a `Vec` of room cells, each a stable `id` with optional
  `title` / `description` / `glyph` overrides (an *ordinary* room takes engine
  defaults; a *special* room overrides them), a required `region` (+ optional
  `subregion`), `combat_enabled`, `exits` (cardinal direction → room id; an exit
  whose target is in a different region/sub-region is a **warp**), and entity /
  item / fixture reference ids.
- `spawn`: the start cell (must land on a room cell).
- `tilesets` + `layers` (additive — slice 1 of the paint editor): a registry of
  tile sheets (`{ id, image, tile_size, columns, rows }`, sliced into tiles with
  optional sparse per-tile metadata) and named tile layers (each a `Vec` of
  `{ x, y, z, tileset, index }` cells — a tile is referenced by tileset id +
  integer index). **Authoring-visual**: validated but not materialized into the
  runtime world yet (see `INTAKE-paint-system-tile-editor`).

Two seams turn a draft into playable world data:

- `MapDocument::validate(&ContentCatalog)` refuses, with a typed
  `MapValidationError` that **names the offending cell/reference**, on: an unsupported
  tile size, an out-of-bounds or duplicate cell, unknown terrain, a room without
  passable terrain, a duplicate room id, an undeclared region/subregion, a dangling
  exit, an unknown entity/item/fixture reference, a missing / non-room spawn, or
(slice 1) a duplicate tileset/layer id, unsupported tileset geometry, or a layer
tile reference / index out of range.
- `MapDocument::materialize(&ContentCatalog)` deterministically builds an
  `oathstar_core::WorldDefinition` (one room per room cell; entities/items pulled from
  the catalog), then runs the engine's own `WorldDefinition::validate` as a final net
  so the output is always `Engine::try_new`-constructable.

Fixtures are modeled and validated but not yet materialized (the engine has no fixture
concept). The TMX/TMJ importer (#39), per-room biome colors over the wire (#38), and
the `/admin/editor` canvas UI are separate tickets; both Studio and any importer
materialize the **same** validated world data.

**Served by the studio (ticket #44).** The `oathstar-studio` sidecar exposes an
Editor-gated `POST /editor/maps/validate`: it takes a `MapDocument` JSON body and runs
validate + materialize against a server-built `ContentCatalog` (from
`load_beginner_world()`), answering `200 {ok:true, room_count, region_count,
start_room_id}` for a valid document or `200 {ok:false, message, error}` (the typed
`MapValidationError` as JSON, naming the offending cell/ref) — with `401`/`403`/`400` for
a missing session, a non-editor, or a malformed body. This is the backend the
`/admin/editor` canvas UI will call.

**Rendered by the studio (ticket #45).** The same sidecar now serves an
Editor-gated `GET /editor` page that draws a server-embedded starter
`MapDocument` on a first-party `<canvas>` (current z-plane: empty / terrain
floor / wall / room / spawn) and a **Validate** control that POSTs the document
to `/editor/maps/validate` and shows the typed result. It follows the same
pure-model + thin-seam split as the game canvas: a DOM-free studio-owned model
(`static/editor-canvas.js`, `node --test`-covered) plus a thin canvas/`fetch`
glue kept as a server-side string — **mirroring** the #16 renderer (Decision
050), not reusing it (the authoring document shape differs from the runtime
snapshot).

**Paintable (ticket #48).** The `/editor` page now carries a **tileset palette**
and a click/drag **paint loop**. The starter document registers the `arctic`
tileset (8px, 30×203, embedded sheet) and an empty `ground` layer; the page
serves the sheet itself at `GET /tilesets/arctic.png` (embedded via
`include_bytes!` — the same no-runtime-asset-dir pattern as the studio's
embedded CSS/JS since #45). The palette draws the
sheet (a scrollable strip, not thousands of DOM nodes); clicking it selects an
active `(tileset, index)`; clicking/dragging the map paints that tile onto the
active layer's cell and repaints. The pure model gained `tileIndexToSourceRect`,
`canvasPointToCell`, `paletteIndexAtPoint`, and an immutable `paintCell`, and
`editorDrawPlan` now emits a `sprites` op per painted layer cell (blitted under
the room/spawn overlay, `imageSmoothingEnabled=false`); the DOM/canvas/mouse/
image-load glue stays the smoke-/review-verified seam. v1 paints one active
layer with one tileset; the room inspector (E), per-tile/layer metadata (S5),
undo/redo, multi-layer UI, save/load (S4), and runtime materialization (#38) are
later slices.

**Inside a nav shell (ticket #49).** The map editor is no longer a standalone
page — every authenticated studio page now carries a persistent navigation
(**Maps · Regions · Items · Enemies · Game Settings**). `/editor` is the **Maps**
section; the other four are Editor-gated "Coming soon" stub routes (HTTP 200, not
404). This is the shell the region & sub-region dashboard (#51) and the future
item/enemy/settings editors slot into, and that the fantasy UI kit (#50) re-skins
— the first slice of the pre-tilemap studio-admin program
(`INTAKE-studio-admin-and-world-model-program`).

**Themed (ticket #50, slice 1).** The studio now wears the mini-medieval fantasy
UI kit: a nine-slice wooden **panel frame** (`border-image` on every `.panel` +
the nav header) and a themed gold **button**, served from `/ui/panel-frame.png`
and `/ui/button.png` (embedded via `include_bytes!`, the same pattern as the
arctic sheet). The crops are committed under `public/ui/`. The game-client re-skin
and the rest of the kit (icons, portraits, bars, banners) are later #50 slices.

**Regions dashboard + authoring (ticket #51).** The nav's **Regions** section manages
the regions and sub-regions of the **persisted authored maps** (the S1 store), not the
baked world. `GET /regions` lists the saved maps (each with region / sub-region counts);
`GET /regions/{id}` is a per-map editor that lists each region — with its nested
sub-regions and room counts — beside Editor-gated **create / rename / delete** forms for
both (`POST /regions/{id}/region` and `…/subregion`, op-dispatched). Every mutation runs
through the content edit seam (the region methods on `MapDocument`): targeted referential
checks give a precise refusal (duplicate id, unknown parent region, a still-referenced
delete), then the whole document must still `materialize()` before it is persisted — a
break is refused and the stored document left untouched. Author-supplied ids/names are
HTML-escaped, and form actions key off the storage slot the page was loaded under (not
the document's own `id`). Slice 1 shipped the read-only baked-world view; slice 2 (this)
replaced it with authored-document CRUD over `id` / `name` / parent — retiring the
`StudioState.world` field. Richer region attributes (descriptions, standing defaults) and
per-sub-region map identity remain later slices; the baked seed is purged after replacement.
