# Author tile-sheet contract

The client renders the map from an **author-provided tile sheet**: a grid PNG
plus a JSON descriptor that names each tile and gives its cell on the sheet.
Real art is author-provided; until a sheet is supplied the map renders the
**flat-color fallback** (the "missing-art skin"). This is the contract that
replaced the generated flat-color tileset (Decision 059, superseding 050/054).

## The descriptor (JSON)

```json
{
  "name": "my-sheet",
  "tileSize": 8,
  "columns": 4,
  "rows": 3,
  "image": "my-sheet.png",
  "tiles": [
    { "name": "shadow_void", "x": 0, "y": 0 },
    { "name": "stone_floor", "x": 8, "y": 0 }
  ]
}
```

- `tileSize` — the source tile edge in pixels. The map document model accepts
  **8, 16, or 32** (`SUPPORTED_TILE_SIZES`); 8px-native is the default
  direction. This is the *art* unit only — how large a cell is drawn is a
  separate client knob (render scale, `DEFAULT_MAP_CONFIG.tilePixels`), so an
  8px sprite upscales nearest-neighbour into the cell.
- `columns` / `rows` — the sheet grid. The image must be `columns * tileSize`
  wide by `rows * tileSize` tall.
- `image` — the sheet PNG filename, taken as a **sibling of the descriptor**
  (its directory): a bare filename, not a sub-path or absolute URL.
- `tiles[]` — each `{ name, x, y }`: a stable name and its top-left pixel on the
  sheet. `x`/`y` must be integers and the tile must lie inside the sheet.

## Required names

Cells resolve art **by name** (`src/client/tileset.js` `KIND_TILE_NAMES`), so
swapping in a new sheet under the same names needs zero code or map changes. A
sheet MUST carry the four load-bearing names: `shadow_void`, `stone_floor`,
`wall_face`, `spawn_marker`. Extra names are free, and become the per-cell
painting vocabulary as the editor grows.

## Pointing the client at a sheet

The map tileset URL resolves through `resolveTilesetUrl` (in `src/client/tileset.js`,
the single source — no other code names a tileset path): a baked
`VITE_OATHSTAR_TILESET` (trimmed, non-blank) wins, otherwise it defaults to the
committed sheet at `DEFAULT_TILESET_URL` (`/tilesets/arctic.json`). So the game map
**renders real tiles by default** (S3.1, ticket #54) — the SPA host serves `public/`
(Vite dev, `dist/`, and the Tauri bundle). A failed fetch or invalid descriptor still
falls back to flat colors (below), so "real by default" never costs robustness.

## Validation & fallback

`validateTileset` never throws: a malformed descriptor is a typed refusal and
the map keeps the fallback; a referenced tile name absent from the sheet draws
the fallback for that cell. See `tests/fixtures/tilesets/sample-8px.json` for a
minimal valid 8px sheet.

Note the two validators: the client `validateTileset` accepts any
positive-integer `tileSize`, while the **8/16/32 bound is the server-side map
document model** (`SUPPORTED_TILE_SIZES`) — a document declaring another size is
refused at validate/materialize.
