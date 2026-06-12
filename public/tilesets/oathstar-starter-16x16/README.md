# Oathstar Starter 16x16 Tileset

This is a small proof-of-concept tileset for painting the early canvas map with
simple readable colors before the final art direction exists.

## Files

- `oathstar-starter-16x16.png` - source sprite sheet, 128x128 pixels.
- `oathstar-starter-16x16-preview.png` - scaled preview with grid lines.
- `oathstar-starter-16x16.json` - engine-friendly metadata.
- `oathstar-starter-16x16.tsx` - Tiled-compatible tileset metadata.
- `bin/generate_oathstar_tileset.py` - deterministic generator.

## Format

- Tile size: 16x16.
- Sheet size: 8 columns by 8 rows.
- Tile IDs: row-major, starting at 0.
- Metadata includes `name`, pixel `x`/`y`, `tags`, and `collision`.

## Tile Ranges

| IDs | Purpose |
|---:|---|
| 0-7 | Base terrain: grass, dirt, stone, water, void |
| 8-23 | Dirt path masks using N/E/S/W bit flags |
| 24-39 | Water and shore masks using N/E/S/W bit flags |
| 40-47 | Stone wall shapes and corners |
| 48-63 | Props and features: rocks, trees, signs, doors, stairs, torches, markers |

## Usage Notes

- Use green grass tiles for forest and town outskirts.
- Use `tree_canopy`, `tree_trunk`, `bush`, `rock`, and `boulder` as blocking
  proof-of-concept objects.
- Use `collision: true` as the first pass for map passability until room/tile
  metadata grows a richer collision model.
- Regenerate the assets with:

```bash
python3 bin/generate_oathstar_tileset.py
```
