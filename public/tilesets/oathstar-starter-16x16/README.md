# Oathstar Starter 16x16 Tileset (blank-colors era)

Every tile is ONE uniform color block (ticket #36): the slice plays in
legible flat color with zero art debt. The client resolves tiles BY NAME
(Decision 050), so real art returns at ship time as a pure regeneration of
this sheet — no code, map, or save changes.

## Files

- `oathstar-starter-16x16.png` - source sprite sheet, 64x48 pixels.
- `oathstar-starter-16x16-preview.png` - scaled preview with grid lines.
- `oathstar-starter-16x16.json` - engine-friendly metadata (the contract).
- `oathstar-starter-16x16.tsx` - Tiled-compatible tileset metadata.
- `bin/generate_oathstar_tileset.py` - deterministic generator (re-runs
  reproduce the committed bytes exactly on the same PIL toolchain; the
  test pin compares pixels, not bytes).

## Format

- Tile size: 16x16.
- Sheet size: 4 columns by 3 rows; 11 named tiles, last slot a spare.
- Tile IDs: row-major, starting at 0.
- Metadata includes `name`, pixel `x`/`y`, `color` (the authored palette
  hex — the committed-asset test cross-checks the PNG pixels against it),
  `tags`, and `collision`.

## Tiles

| ID | Name | Color | Role |
|---:|---|---|---|
| 0 | `shadow_void` | #101419 | unexplored (load-bearing) |
| 1 | `stone_floor` | #696f68 | discovered / city floor (load-bearing) |
| 2 | `wall_face` | #434a4a | blocked (load-bearing) |
| 3 | `spawn_marker` | #5fccb1 | current room / hero teal (load-bearing) |
| 4 | `grass` | #347b41 | forest floor |
| 5 | `dirt` | #815b30 | road |
| 6 | `cave_floor` | #463e4e | cave floor |
| 7 | `deep_water` | #265c91 | water |
| 8 | `stairs_up` | #a96a37 | floor link up |
| 9 | `stairs_down` | #472d1c | floor link down |
| 10 | `exit_marker` | #d8a941 | door / exit marker |

The first four names are load-bearing — the client's `KIND_TILE_NAMES`
resolves them and must keep working untouched. Hero/enemies/items are
runtime overlays (Decision 051), never tiles.

## Usage Notes

- Step C paints sub-regions with the floor variants (city stone, forest
  grass, road, cave) so biomes appear with no client changes.
- Use `collision: true` as the first pass for map passability until
  room/tile metadata grows a richer collision model.
- Regenerate the assets with:

```bash
python3 bin/generate_oathstar_tileset.py
```
