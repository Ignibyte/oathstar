#!/usr/bin/env python3
"""Regenerate the committed starter tileset (ticket #32; flattened at #36).

Blank-colors era: every tile is ONE uniform color block from the FLAT_TILES
table below — no texture, no noise, no rng. The map stays legible with zero
art debt, and because the client resolves tiles BY NAME (Decision 050), real
art returns at ship time as a pure regeneration of this sheet. Entities
(hero/enemies/items) are runtime overlays (Decision 051), never tiles.

Run with the PIL-bearing interpreter:  python3 bin/generate_oathstar_tileset.py
Output is committed under public/tilesets/ — the vite-served home.
"""
from __future__ import annotations

import json
from pathlib import Path
from xml.sax.saxutils import escape

from PIL import Image, ImageDraw

TILE = 32
COLS = 4
ROWS = 3
NAME = "oathstar-starter-32x32"

ROOT = Path(__file__).resolve().parents[1]
# The vite-served home (ticket #32): regeneration writes where the app reads.
OUT_DIR = ROOT / "public" / "tilesets" / NAME

# The whole generator is this table (ticket #36): name → (rgba, tags,
# collision). Order is sheet order (row-major on a 4-column sheet). The first
# four names are load-bearing — the client's KIND_TILE_NAMES resolves them —
# and the rest are the flat extras step C paints sub-regions with. Colors
# follow the flat-color spike concept: city stone, forest green, road brown,
# cave violet-grey, door gold, hero teal.
FLAT_TILES: list[tuple[str, tuple[int, int, int, int], list[str], bool]] = [
    ("shadow_void", (16, 20, 25, 255), ["terrain", "void"], True),
    ("stone_floor", (105, 111, 104, 255), ["terrain", "floor", "city"], False),
    ("wall_face", (67, 74, 74, 255), ["wall"], True),
    ("spawn_marker", (95, 204, 177, 255), ["feature", "spawn"], False),
    ("grass", (52, 123, 65, 255), ["terrain", "floor", "forest"], False),
    ("dirt", (129, 91, 48, 255), ["terrain", "floor", "road"], False),
    ("cave_floor", (70, 62, 78, 255), ["terrain", "floor", "cave"], False),
    ("deep_water", (38, 92, 145, 255), ["terrain", "water"], True),
    ("stairs_up", (169, 106, 55, 255), ["feature", "stairs"], False),
    ("stairs_down", (71, 45, 28, 255), ["feature", "stairs"], False),
    ("exit_marker", (216, 169, 65, 255), ["feature", "exit"], False),
]

TRANSPARENT = (0, 0, 0, 0)


def color_hex(rgba: tuple[int, int, int, int]) -> str:
    red, green, blue, _ = rgba
    return f"#{red:02x}{green:02x}{blue:02x}"


def tile_record(
    index: int, name: str, rgba: tuple[int, int, int, int], tags: list[str], collision: bool
) -> dict[str, object]:
    return {
        "id": index,
        "name": name,
        "x": (index % COLS) * TILE,
        "y": (index // COLS) * TILE,
        # The authored palette as contract (ticket #36): the committed-asset
        # test cross-checks every PNG tile's uniform pixels against this.
        "color": color_hex(rgba),
        "tags": tags,
        "collision": collision,
    }


def build_tiles() -> tuple[Image.Image, list[dict[str, object]]]:
    # Guard the table (inspect #36): an overflowing entry would silently
    # clip-paint while its JSON rect went out of bounds, and a translucent
    # color could not be expressed by the JSON `color` hex contract.
    assert len(FLAT_TILES) <= COLS * ROWS, "FLAT_TILES exceeds the sheet"
    assert all(rgba[3] == 255 for _, rgba, _, _ in FLAT_TILES), "tiles must be opaque"
    sheet = Image.new("RGBA", (COLS * TILE, ROWS * TILE), TRANSPARENT)
    draw = ImageDraw.Draw(sheet)
    records: list[dict[str, object]] = []
    for index, (name, rgba, tags, collision) in enumerate(FLAT_TILES):
        x = (index % COLS) * TILE
        y = (index // COLS) * TILE
        draw.rectangle((x, y, x + TILE - 1, y + TILE - 1), fill=rgba)
        records.append(tile_record(index, name, rgba, tags, collision))
    # Any slot past the table stays transparent and unnamed (spare).
    return sheet, records


def write_preview(sheet: Image.Image, path: Path) -> None:
    scale = 4
    scaled = sheet.resize((sheet.width * scale, sheet.height * scale), Image.Resampling.NEAREST)
    preview = Image.new("RGBA", scaled.size, (0, 0, 0, 255))
    draw = ImageDraw.Draw(preview)
    checker = TILE * scale // 2
    for y in range(0, preview.height, checker):
        for x in range(0, preview.width, checker):
            shade = (42, 45, 48, 255) if (x // checker + y // checker) % 2 else (58, 61, 64, 255)
            draw.rectangle((x, y, x + checker - 1, y + checker - 1), fill=shade)
    preview.alpha_composite(scaled)
    for x in range(0, preview.width + 1, TILE * scale):
        draw.line((x, 0, x, preview.height), fill=(255, 255, 255, 96))
    for y in range(0, preview.height + 1, TILE * scale):
        draw.line((0, y, preview.width, y), fill=(255, 255, 255, 96))
    # compress_level pinned so re-runs reproduce the committed bytes.
    preview.save(path, compress_level=9)


def write_tiled_tsx(path: Path, records: list[dict[str, object]]) -> None:
    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        f'<tileset version="1.10" tiledversion="1.11.2" name="{NAME}" tilewidth="{TILE}" tileheight="{TILE}" tilecount="{COLS * ROWS}" columns="{COLS}">',
        f' <image source="{NAME}.png" width="{COLS * TILE}" height="{ROWS * TILE}"/>',
    ]
    for record in records:
        props = {
            "name": str(record["name"]),
            "color": str(record["color"]),
            "tags": ",".join(record["tags"]),  # type: ignore[arg-type]
            "collision": "true" if record["collision"] else "false",
        }
        lines.append(f' <tile id="{record["id"]}">')
        lines.append("  <properties>")
        for key, value in props.items():
            prop_type = ' type="bool"' if key == "collision" else ""
            lines.append(f'   <property name="{escape(key)}"{prop_type} value="{escape(value)}"/>')
        lines.append("  </properties>")
        lines.append(" </tile>")
    lines.append("</tileset>")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    sheet, records = build_tiles()

    png_path = OUT_DIR / f"{NAME}.png"
    preview_path = OUT_DIR / f"{NAME}-preview.png"
    json_path = OUT_DIR / f"{NAME}.json"
    tsx_path = OUT_DIR / f"{NAME}.tsx"

    sheet.save(png_path, compress_level=9)
    write_preview(sheet, preview_path)
    json_path.write_text(
        json.dumps(
            {
                "name": NAME,
                "tileSize": TILE,
                "columns": COLS,
                "rows": ROWS,
                "image": png_path.name,
                "tiles": records,
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    write_tiled_tsx(tsx_path, records)

    print(f"wrote {png_path}")
    print(f"wrote {preview_path}")
    print(f"wrote {json_path}")
    print(f"wrote {tsx_path}")


if __name__ == "__main__":
    main()
