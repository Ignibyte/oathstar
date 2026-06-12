#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import random
from pathlib import Path
from xml.sax.saxutils import escape

from PIL import Image, ImageDraw


TILE = 16
COLS = 8
ROWS = 8
NAME = "oathstar-starter-16x16"

ROOT = Path(__file__).resolve().parents[1]
# The vite-served home (ticket #32): regeneration writes where the app reads.
OUT_DIR = ROOT / "public" / "tilesets" / NAME

PALETTE = {
    "transparent": (0, 0, 0, 0),
    "grass": (52, 123, 65, 255),
    "grass_dark": (34, 88, 49, 255),
    "grass_light": (85, 157, 78, 255),
    "grass_yellow": (150, 170, 88, 255),
    "flower": (222, 208, 101, 255),
    "flower_blue": (105, 161, 205, 255),
    "dirt": (129, 91, 48, 255),
    "dirt_dark": (88, 62, 38, 255),
    "dirt_light": (171, 120, 63, 255),
    "stone": (105, 111, 104, 255),
    "stone_dark": (67, 74, 74, 255),
    "stone_light": (148, 151, 135, 255),
    "moss": (70, 128, 70, 255),
    "water": (38, 92, 145, 255),
    "water_dark": (25, 61, 107, 255),
    "water_light": (87, 159, 188, 255),
    "shore": (88, 76, 48, 255),
    "void": (16, 20, 25, 255),
    "wood": (117, 74, 38, 255),
    "wood_dark": (71, 45, 28, 255),
    "wood_light": (169, 106, 55, 255),
    "gold": (216, 169, 65, 255),
    "ember": (236, 104, 43, 255),
    "rune": (95, 204, 177, 255),
}

N, E, S, W = 1, 2, 4, 8
DIRECTIONS = ((N, "n"), (E, "e"), (S, "s"), (W, "w"))


def rng_for(name: str) -> random.Random:
    digest = hashlib.sha256(name.encode("utf-8")).digest()
    return random.Random(int.from_bytes(digest[:8], "big"))


def new_tile(color_name: str | None = None) -> Image.Image:
    fill = PALETTE[color_name] if color_name else PALETTE["transparent"]
    return Image.new("RGBA", (TILE, TILE), fill)


def draw_noise(
    tile: Image.Image,
    name: str,
    colors: list[str],
    count: int,
    bounds: tuple[int, int, int, int] = (1, 1, 14, 14),
) -> None:
    draw = ImageDraw.Draw(tile)
    rng = rng_for(name)
    x0, y0, x1, y1 = bounds
    for _ in range(count):
        x = rng.randint(x0, x1)
        y = rng.randint(y0, y1)
        draw.point((x, y), fill=PALETTE[rng.choice(colors)])


def draw_grass(name: str, flowers: bool = False) -> Image.Image:
    tile = new_tile("grass")
    draw_noise(tile, f"{name}:grass", ["grass_dark", "grass_light"], 24)
    draw = ImageDraw.Draw(tile)
    rng = rng_for(f"{name}:blades")
    for _ in range(7):
        x = rng.randint(2, 13)
        y = rng.randint(3, 13)
        draw.line((x, y, x, min(14, y + 1)), fill=PALETTE["grass_dark"])
    if flowers:
        for x, y, color in ((4, 5, "flower"), (11, 9, "flower_blue"), (7, 12, "flower")):
            draw.point((x, y), fill=PALETTE[color])
            draw.point((x + 1, y), fill=PALETTE[color])
            draw.point((x, y + 1), fill=PALETTE["grass_light"])
    return tile


def draw_dirt(name: str, rocks: bool = False) -> Image.Image:
    tile = new_tile("dirt")
    draw_noise(tile, f"{name}:dirt", ["dirt_dark", "dirt_light"], 32)
    draw = ImageDraw.Draw(tile)
    if rocks:
        for x, y in ((3, 4), (10, 6), (6, 12), (13, 11)):
            draw.point((x, y), fill=PALETTE["stone_light"])
            draw.point((x + 1, y), fill=PALETTE["stone_dark"])
    return tile


def draw_stone_floor(name: str, cracked: bool = False) -> Image.Image:
    tile = new_tile("stone")
    draw = ImageDraw.Draw(tile)
    for y in (5, 11):
        draw.line((0, y, 15, y), fill=PALETTE["stone_dark"])
    for x, y0, y1 in ((4, 0, 5), (11, 0, 5), (7, 6, 11), (13, 6, 11), (3, 12, 15), (10, 12, 15)):
        draw.line((x, y0, x, y1), fill=PALETTE["stone_dark"])
    draw_noise(tile, f"{name}:stone", ["stone_dark", "stone_light", "moss"], 22, (1, 1, 14, 14))
    if cracked:
        draw.line((4, 3, 7, 6), fill=PALETTE["stone_dark"])
        draw.line((7, 6, 6, 9), fill=PALETTE["stone_dark"])
        draw.line((11, 12, 13, 14), fill=PALETTE["stone_dark"])
    return tile


def draw_deep_water(name: str) -> Image.Image:
    tile = new_tile("water")
    draw = ImageDraw.Draw(tile)
    draw.rectangle((0, 0, 15, 15), outline=PALETTE["water_dark"])
    for y in (4, 10):
        draw.line((2, y, 6, y), fill=PALETTE["water_light"])
        draw.line((10, y + 1, 13, y + 1), fill=PALETTE["water_dark"])
    draw_noise(tile, f"{name}:water", ["water_dark", "water_light"], 16)
    return tile


def shape_for_mask(mask: int, width: int) -> set[tuple[int, int]]:
    low = (TILE - width) // 2
    high = low + width - 1
    shape: set[tuple[int, int]] = set()
    for y in range(low, high + 1):
        for x in range(low, high + 1):
            shape.add((x, y))
    if mask & N:
        for y in range(0, low):
            for x in range(low, high + 1):
                shape.add((x, y))
    if mask & S:
        for y in range(high + 1, TILE):
            for x in range(low, high + 1):
                shape.add((x, y))
    if mask & W:
        for y in range(low, high + 1):
            for x in range(0, low):
                shape.add((x, y))
    if mask & E:
        for y in range(low, high + 1):
            for x in range(high + 1, TILE):
                shape.add((x, y))
    return shape


def edge_pixel(x: int, y: int, shape: set[tuple[int, int]]) -> bool:
    for nx, ny in ((x, y - 1), (x + 1, y), (x, y + 1), (x - 1, y)):
        if 0 <= nx < TILE and 0 <= ny < TILE and (nx, ny) not in shape:
            return True
    return False


def draw_mask_tile(
    name: str,
    mask: int,
    kind: str,
    base_tile: Image.Image | None = None,
) -> Image.Image:
    tile = base_tile.copy() if base_tile else draw_grass(f"{name}:base")
    draw = ImageDraw.Draw(tile)
    if kind == "path":
        shape = shape_for_mask(mask, 6)
        fill_name, edge_name = "dirt", "dirt_dark"
        texture = ["dirt_light", "dirt_dark"]
    else:
        shape = shape_for_mask(mask, 8)
        fill_name, edge_name = "water", "shore"
        texture = ["water_light", "water_dark"]
    for x, y in sorted(shape):
        draw.point((x, y), fill=PALETTE[edge_name if edge_pixel(x, y, shape) else fill_name])
    rng = rng_for(f"{name}:texture")
    inner = [
        (x, y)
        for x, y in shape
        if not edge_pixel(x, y, shape) and 0 < x < TILE - 1 and 0 < y < TILE - 1
    ]
    for _ in range(18):
        x, y = rng.choice(inner)
        draw.point((x, y), fill=PALETTE[rng.choice(texture)])
    return tile


def draw_void() -> Image.Image:
    tile = new_tile("void")
    draw_noise(tile, "void", ["stone_dark", "water_dark"], 18)
    return tile


def draw_wall(name: str) -> Image.Image:
    tile = new_tile("stone_dark")
    draw = ImageDraw.Draw(tile)
    draw.rectangle((1, 1, 14, 14), fill=PALETTE["stone"])
    draw.rectangle((1, 1, 14, 4), fill=PALETTE["stone_light"])
    draw.line((1, 5, 14, 5), fill=PALETTE["stone_dark"])
    draw_noise(tile, f"{name}:wall", ["stone_dark", "stone_light", "moss"], 18, (2, 2, 13, 13))
    return tile


def wall_variant(name: str, variant: str) -> Image.Image:
    tile = draw_wall(name)
    draw = ImageDraw.Draw(tile)
    if variant == "face":
        draw.rectangle((1, 5, 14, 14), fill=PALETTE["stone_dark"])
        draw.rectangle((2, 6, 13, 13), fill=PALETTE["stone"])
    elif variant == "left":
        draw.rectangle((1, 1, 5, 14), fill=PALETTE["stone_dark"])
    elif variant == "right":
        draw.rectangle((10, 1, 14, 14), fill=PALETTE["stone_dark"])
    elif variant == "corner_nw":
        draw.rectangle((1, 1, 5, 14), fill=PALETTE["stone_dark"])
        draw.rectangle((1, 1, 14, 5), fill=PALETTE["stone_light"])
    elif variant == "corner_ne":
        draw.rectangle((10, 1, 14, 14), fill=PALETTE["stone_dark"])
        draw.rectangle((1, 1, 14, 5), fill=PALETTE["stone_light"])
    elif variant == "corner_sw":
        draw.rectangle((1, 1, 5, 14), fill=PALETTE["stone_dark"])
        draw.rectangle((1, 10, 14, 14), fill=PALETTE["stone_dark"])
    elif variant == "corner_se":
        draw.rectangle((10, 1, 14, 14), fill=PALETTE["stone_dark"])
        draw.rectangle((1, 10, 14, 14), fill=PALETTE["stone_dark"])
    return tile


def transparent_prop(name: str) -> tuple[Image.Image, ImageDraw.ImageDraw]:
    tile = new_tile()
    return tile, ImageDraw.Draw(tile)


def draw_prop(name: str) -> Image.Image:
    tile, draw = transparent_prop(name)
    if name == "rock":
        draw.rectangle((4, 7, 11, 12), fill=PALETTE["stone_dark"])
        draw.rectangle((5, 5, 10, 10), fill=PALETTE["stone"])
        draw.point((6, 6), fill=PALETTE["stone_light"])
    elif name == "boulder":
        draw.rectangle((3, 5, 12, 13), fill=PALETTE["stone_dark"])
        draw.rectangle((4, 4, 11, 11), fill=PALETTE["stone"])
        draw.line((5, 5, 8, 5), fill=PALETTE["stone_light"])
    elif name == "bush":
        for rect in ((4, 8, 11, 12), (3, 9, 12, 13), (5, 6, 9, 10)):
            draw.rectangle(rect, fill=PALETTE["grass_dark"])
        draw.point((6, 8), fill=PALETTE["grass_light"])
        draw.point((10, 10), fill=PALETTE["grass_light"])
    elif name == "tree_canopy":
        for rect in ((4, 3, 11, 11), (2, 6, 13, 12), (5, 1, 10, 7)):
            draw.rectangle(rect, fill=PALETTE["grass_dark"])
        draw.rectangle((5, 4, 10, 10), fill=PALETTE["grass"])
        draw.point((8, 5), fill=PALETTE["grass_light"])
    elif name == "tree_trunk":
        draw.rectangle((6, 5, 9, 14), fill=PALETTE["wood_dark"])
        draw.rectangle((7, 5, 8, 14), fill=PALETTE["wood"])
        draw.point((7, 8), fill=PALETTE["wood_light"])
    elif name == "mushrooms":
        for x, cap in ((5, "flower"), (10, "flower_blue")):
            draw.rectangle((x, 9, x + 1, 13), fill=PALETTE["stone_light"])
            draw.rectangle((x - 1, 7, x + 2, 9), fill=PALETTE[cap])
    elif name == "chest_closed":
        draw.rectangle((3, 7, 12, 12), fill=PALETTE["wood_dark"])
        draw.rectangle((4, 6, 11, 10), fill=PALETTE["wood"])
        draw.rectangle((7, 8, 8, 10), fill=PALETTE["gold"])
    elif name == "sign":
        draw.rectangle((7, 8, 8, 14), fill=PALETTE["wood_dark"])
        draw.rectangle((3, 4, 12, 8), fill=PALETTE["wood"])
        draw.line((4, 6, 10, 6), fill=PALETTE["wood_dark"])
    elif name == "door_closed":
        draw.rectangle((4, 3, 11, 14), fill=PALETTE["wood_dark"])
        draw.rectangle((5, 4, 10, 14), fill=PALETTE["wood"])
        draw.point((9, 9), fill=PALETTE["gold"])
    elif name == "stairs_down":
        draw.rectangle((3, 4, 12, 13), fill=PALETTE["stone_dark"])
        for y, x0 in ((5, 4), (7, 5), (9, 6), (11, 7)):
            draw.line((x0, y, 12, y), fill=PALETTE["stone_light"])
    elif name == "torch":
        draw.rectangle((7, 7, 8, 14), fill=PALETTE["wood_dark"])
        draw.rectangle((6, 4, 9, 7), fill=PALETTE["ember"])
        draw.point((7, 3), fill=PALETTE["flower"])
    elif name == "pillar":
        draw.rectangle((5, 3, 10, 13), fill=PALETTE["stone"])
        draw.rectangle((4, 2, 11, 4), fill=PALETTE["stone_light"])
        draw.rectangle((4, 12, 11, 14), fill=PALETTE["stone_dark"])
    elif name == "rubble":
        for rect in ((4, 9, 6, 11), (8, 7, 11, 10), (10, 11, 13, 13), (3, 12, 5, 13)):
            draw.rectangle(rect, fill=PALETTE["stone"])
            draw.point((rect[0], rect[1]), fill=PALETTE["stone_light"])
    elif name == "altar":
        draw.rectangle((3, 8, 12, 13), fill=PALETTE["stone_dark"])
        draw.rectangle((4, 6, 11, 10), fill=PALETTE["stone"])
        draw.point((7, 7), fill=PALETTE["rune"])
        draw.point((8, 7), fill=PALETTE["rune"])
    elif name == "spawn_marker":
        draw.rectangle((5, 5, 10, 10), outline=PALETTE["rune"])
        draw.point((7, 7), fill=PALETTE["rune"])
        draw.point((8, 8), fill=PALETTE["rune"])
    elif name == "exit_marker":
        draw.rectangle((4, 3, 11, 13), fill=PALETTE["void"])
        draw.rectangle((5, 4, 10, 12), outline=PALETTE["rune"])
        draw.line((7, 8, 10, 8), fill=PALETTE["rune"])
        draw.point((9, 7), fill=PALETTE["rune"])
        draw.point((9, 9), fill=PALETTE["rune"])
    return tile


def tile_record(index: int, name: str, tags: list[str], collision: bool = False) -> dict[str, object]:
    return {
        "id": index,
        "name": name,
        "x": (index % COLS) * TILE,
        "y": (index // COLS) * TILE,
        "tags": tags,
        "collision": collision,
    }


def paste_tile(sheet: Image.Image, index: int, tile: Image.Image) -> None:
    sheet.alpha_composite(tile, ((index % COLS) * TILE, (index // COLS) * TILE))


def build_tiles() -> tuple[Image.Image, list[dict[str, object]]]:
    sheet = Image.new("RGBA", (COLS * TILE, ROWS * TILE), PALETTE["transparent"])
    records: list[dict[str, object]] = []

    base_tiles = [
        ("grass", draw_grass("grass"), ["terrain", "grass"], False),
        ("grass_flowers", draw_grass("grass_flowers", True), ["terrain", "grass", "detail"], False),
        ("dirt", draw_dirt("dirt"), ["terrain", "dirt"], False),
        ("dirt_rocks", draw_dirt("dirt_rocks", True), ["terrain", "dirt", "detail"], False),
        ("stone_floor", draw_stone_floor("stone_floor"), ["terrain", "floor", "stone"], False),
        ("cracked_stone_floor", draw_stone_floor("cracked_stone_floor", True), ["terrain", "floor", "stone"], False),
        ("deep_water", draw_deep_water("deep_water"), ["terrain", "water"], True),
        ("shadow_void", draw_void(), ["terrain", "void"], True),
    ]

    for i, (name, tile, tags, collision) in enumerate(base_tiles):
        paste_tile(sheet, i, tile)
        records.append(tile_record(i, name, tags, collision))

    for mask in range(16):
        index = 8 + mask
        suffix = "".join(label for bit, label in DIRECTIONS if mask & bit) or "isolated"
        name = f"path_{suffix}"
        paste_tile(sheet, index, draw_mask_tile(name, mask, "path"))
        records.append(tile_record(index, name, ["terrain", "path", f"mask:{mask}"], False))

    for mask in range(16):
        index = 24 + mask
        suffix = "".join(label for bit, label in DIRECTIONS if mask & bit) or "isolated"
        name = f"water_{suffix}"
        paste_tile(sheet, index, draw_mask_tile(name, mask, "water"))
        records.append(tile_record(index, name, ["terrain", "water", "shore", f"mask:{mask}"], True))

    wall_names = [
        ("wall_top", "top"),
        ("wall_face", "face"),
        ("wall_left", "left"),
        ("wall_right", "right"),
        ("wall_corner_nw", "corner_nw"),
        ("wall_corner_ne", "corner_ne"),
        ("wall_corner_sw", "corner_sw"),
        ("wall_corner_se", "corner_se"),
    ]
    for offset, (name, variant) in enumerate(wall_names):
        index = 40 + offset
        paste_tile(sheet, index, wall_variant(name, variant))
        records.append(tile_record(index, name, ["wall", "stone"], True))

    prop_names = [
        "rock",
        "boulder",
        "bush",
        "tree_canopy",
        "tree_trunk",
        "mushrooms",
        "chest_closed",
        "sign",
        "door_closed",
        "stairs_down",
        "torch",
        "pillar",
        "rubble",
        "altar",
        "spawn_marker",
        "exit_marker",
    ]
    solid_props = {"rock", "boulder", "tree_canopy", "tree_trunk", "chest_closed", "sign", "door_closed", "pillar", "altar"}
    for offset, name in enumerate(prop_names):
        index = 48 + offset
        paste_tile(sheet, index, draw_prop(name))
        tags = ["object"] if index < 56 else ["feature"]
        records.append(tile_record(index, name, tags, name in solid_props))

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
    preview.save(path)


def write_tiled_tsx(path: Path, records: list[dict[str, object]]) -> None:
    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        f'<tileset version="1.10" tiledversion="1.11.2" name="{NAME}" tilewidth="{TILE}" tileheight="{TILE}" tilecount="{COLS * ROWS}" columns="{COLS}">',
        f' <image source="{NAME}.png" width="{COLS * TILE}" height="{ROWS * TILE}"/>',
    ]
    for record in records:
        props = {
            "name": str(record["name"]),
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

    sheet.save(png_path)
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
