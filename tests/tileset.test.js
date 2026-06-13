import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { inflateSync } from "node:zlib";

import { KIND_TILE_NAMES, validateTileset, tileRect, kindTileRects } from "../src/client/tileset.js";

// The committed sheet metadata IS the contract (ticket #32): tests read the
// real served file, so regenerating with different names/geometry fails here
// loudly instead of silently blanking the map at runtime.
const here = dirname(fileURLToPath(import.meta.url));
const ASSET_DIR = join(here, "..", "public", "tilesets", "oathstar-starter-16x16");
const REAL_JSON_PATH = join(ASSET_DIR, "oathstar-starter-16x16.json");
const REAL_PNG_PATH = join(ASSET_DIR, "oathstar-starter-16x16.png");
const REAL_TSX_PATH = join(ASSET_DIR, "oathstar-starter-16x16.tsx");

function realTilesetJson() {
  return JSON.parse(readFileSync(REAL_JSON_PATH, "utf8"));
}

// --- a minimal zero-dep PNG reader (ticket #36) ---------------------------
// Just enough to decode the committed sheet (8-bit RGBA, non-interlaced, the
// shape PIL writes): parse IHDR + IDAT, inflate, unfilter the five row
// filters. The blank-colors pin reads PIXELS, so it survives PNG-encoder
// byte drift across PIL versions.

function paeth(a, b, c) {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  if (pa <= pb && pa <= pc) {
    return a;
  }
  return pb <= pc ? b : c;
}

function decodePng(buffer) {
  const signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  signature.forEach((byte, i) => assert.equal(buffer[i], byte, "PNG signature"));
  let offset = 8;
  let width = 0;
  let height = 0;
  const idat = [];
  while (offset < buffer.length) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.toString("ascii", offset + 4, offset + 8);
    const data = buffer.subarray(offset + 8, offset + 8 + length);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      assert.equal(data[8], 8, "bit depth 8");
      assert.equal(data[9], 6, "color type RGBA");
      assert.equal(data[12], 0, "non-interlaced");
    } else if (type === "IDAT") {
      idat.push(data);
    }
    offset += 12 + length;
  }
  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * 4;
  const pixels = Buffer.alloc(height * stride);
  for (let y = 0; y < height; y += 1) {
    const filter = raw[y * (stride + 1)];
    const row = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    const out = pixels.subarray(y * stride, (y + 1) * stride);
    const prev = y > 0 ? pixels.subarray((y - 1) * stride, y * stride) : null;
    for (let x = 0; x < stride; x += 1) {
      const left = x >= 4 ? out[x - 4] : 0;
      const up = prev ? prev[x] : 0;
      const upLeft = prev && x >= 4 ? prev[x - 4] : 0;
      let value = row[x];
      if (filter === 1) {
        value += left;
      } else if (filter === 2) {
        value += up;
      } else if (filter === 3) {
        value += Math.floor((left + up) / 2);
      } else if (filter === 4) {
        value += paeth(left, up, upLeft);
      } else {
        assert.equal(filter, 0, `row ${y} uses a known PNG filter`);
      }
      out[x] = value & 0xff;
    }
  }
  return { width, height, pixels };
}

function pixelAt(image, x, y) {
  const i = (y * image.width + x) * 4;
  return [image.pixels[i], image.pixels[i + 1], image.pixels[i + 2], image.pixels[i + 3]];
}

// A minimal valid tileset carrying exactly the four required kind tiles.
function minimalTiles(overrides = {}) {
  return {
    tileSize: 16,
    columns: 2,
    rows: 2,
    image: "sheet.png",
    tiles: [
      { name: "shadow_void", x: 0, y: 0 },
      { name: "stone_floor", x: 16, y: 0 },
      { name: "wall_face", x: 0, y: 16 },
      { name: "spawn_marker", x: 16, y: 16 },
    ],
    ...overrides,
  };
}

// T1 (REQ-001/004/007): the committed asset validates and resolves end to end.
test("the committed tileset JSON validates with every name resolvable", () => {
  const raw = realTilesetJson();
  const result = validateTileset(raw);
  assert.equal(result.ok, true, `committed asset must validate: ${result.reason ?? ""}`);
  const { tileset } = result;
  assert.equal(tileset.tileSize, 16);
  assert.equal(tileset.columns, 4);
  assert.equal(tileset.rows, 3);
  assert.equal(tileset.image, "oathstar-starter-16x16.png");
  assert.equal(tileset.tiles.length, 11, "the flat sheet carries 11 named tiles (ticket #36)");
  for (const tile of raw.tiles) {
    const rect = tileRect(tileset, tile.name);
    assert.deepEqual(
      rect,
      { sx: tile.x, sy: tile.y, sSize: 16 },
      `every committed name resolves to its authored rect (${tile.name})`,
    );
  }
});

// T2 (REQ-004): every malformed shape is a typed refusal naming the check —
// never a throw. Includes the two inspect-found holes (__proto__, fractional).
test("validateTileset refuses malformed metadata with typed reasons, never throws", () => {
  const cases = [
    [null, "not an object"],
    [undefined, "not an object"],
    [[], "not an object"],
    ["tileset", "not an object"],
    [minimalTiles({ tileSize: 0 }), "tileSize"],
    [minimalTiles({ tileSize: -16 }), "tileSize"],
    [minimalTiles({ tileSize: Number.NaN }), "tileSize"],
    [minimalTiles({ tileSize: 16.5 }), "tileSize"],
    [minimalTiles({ tileSize: undefined }), "tileSize"],
    [minimalTiles({ columns: 0 }), "columns"],
    [minimalTiles({ columns: 2.5 }), "columns"],
    [minimalTiles({ rows: Number.POSITIVE_INFINITY }), "rows"],
    [minimalTiles({ image: "" }), "image"],
    [minimalTiles({ image: 7 }), "image"],
    [minimalTiles({ tiles: null }), "tiles"],
    [minimalTiles({ tiles: {} }), "tiles"],
    [minimalTiles({ tiles: [null] }), "tile must be an object"],
    [minimalTiles({ tiles: [{ name: "", x: 0, y: 0 }] }), "name"],
    [minimalTiles({ tiles: [{ name: 7, x: 0, y: 0 }] }), "name"],
    [minimalTiles({ tiles: [{ name: "shadow_void", x: "0", y: 0 }] }), "non-integer"],
    [minimalTiles({ tiles: [{ name: "shadow_void", x: 3.5, y: 0 }] }), "non-integer"],
    [minimalTiles({ tiles: [{ name: "shadow_void", x: 0, y: Number.NaN }] }), "non-integer"],
    [minimalTiles({ tiles: [{ name: "shadow_void", x: 32, y: 0 }] }), "outside the sheet"],
    [minimalTiles({ tiles: [{ name: "shadow_void", x: 0, y: -16 }] }), "outside the sheet"],
    // One required kind tile absent.
    [minimalTiles({ tiles: minimalTiles().tiles.slice(0, 3) }), "required tile 'spawn_marker'"],
    // The inspect-found prototype poisoning: a "__proto__" tile must not
    // satisfy the required-names check via the prototype chain.
    [
      minimalTiles({
        tiles: [
          { name: "__proto__", x: 0, y: 0, shadow_void: 1, stone_floor: 1, wall_face: 1, spawn_marker: 1 },
        ],
      }),
      "required tile",
    ],
  ];
  for (const [raw, reasonFragment] of cases) {
    const result = validateTileset(raw);
    assert.equal(result.ok, false, `must refuse: ${JSON.stringify(raw)?.slice(0, 80)}`);
    assert.ok(
      result.reason.includes(reasonFragment),
      `reason '${result.reason}' names the check ('${reasonFragment}')`,
    );
  }
});

// T3 (REQ-001): name → rect resolution from the real sheet, and null for the
// unknown name (the only absent-name path left after validation).
test("tileRect resolves committed names to authored rects and unknown names to null", () => {
  const { tileset } = validateTileset(realTilesetJson());
  assert.deepEqual(tileRect(tileset, "grass"), { sx: 0, sy: 16, sSize: 16 });
  assert.deepEqual(tileRect(tileset, "stone_floor"), { sx: 16, sy: 0, sSize: 16 });
  assert.equal(tileRect(tileset, "no_such_tile"), null);
  assert.equal(tileRect(tileset, "hasOwnProperty"), null, "prototype names are not tiles");
});

// T4 (REQ-001/002): the kind table covers exactly the four cell kinds, every
// name exists on the committed sheet, and kindTileRects resolves all four.
test("KIND_TILE_NAMES maps all four cell kinds onto committed tiles", () => {
  assert.deepEqual(
    Object.keys(KIND_TILE_NAMES).sort(),
    ["blocked", "current", "discovered", "empty"],
  );
  const { tileset } = validateTileset(realTilesetJson());
  const rects = kindTileRects(tileset);
  for (const [kind, name] of Object.entries(KIND_TILE_NAMES)) {
    assert.ok(tileset.byName[name], `'${name}' (${kind}) exists on the sheet`);
    assert.deepEqual(rects[kind], tileRect(tileset, name), `${kind} resolves through the table`);
  }
});

// T5 (ticket #36, REQ-002): the blank-colors contract — every named tile in
// the committed PNG is ONE uniform opaque color equal to its declared JSON
// `color`, the four load-bearing tiles match the authored palette exactly,
// and the spare grid slot stays transparent.
test("every committed tile is one uniform color matching its declared contract", () => {
  const raw = realTilesetJson();
  const png = decodePng(readFileSync(REAL_PNG_PATH));
  assert.equal(png.width, raw.columns * raw.tileSize, "sheet width matches the grid");
  assert.equal(png.height, raw.rows * raw.tileSize, "sheet height matches the grid");

  for (const tile of raw.tiles) {
    const [r0, g0, b0, a0] = pixelAt(png, tile.x, tile.y);
    assert.equal(a0, 255, `${tile.name} is opaque`);
    const hex = `#${[r0, g0, b0].map((v) => v.toString(16).padStart(2, "0")).join("")}`;
    assert.equal(hex, tile.color, `${tile.name} pixels match the declared color`);
    for (let dy = 0; dy < raw.tileSize; dy += 1) {
      for (let dx = 0; dx < raw.tileSize; dx += 1) {
        const [r, g, b, a] = pixelAt(png, tile.x + dx, tile.y + dy);
        if (r !== r0 || g !== g0 || b !== b0 || a !== a0) {
          assert.fail(`${tile.name} is not uniform at +${dx},+${dy}`);
        }
      }
    }
  }

  // The four load-bearing tiles carry the authored palette verbatim.
  const authored = {
    shadow_void: "#101419",
    stone_floor: "#696f68",
    wall_face: "#434a4a",
    spawn_marker: "#5fccb1",
  };
  for (const [name, hex] of Object.entries(authored)) {
    const tile = raw.tiles.find((entry) => entry.name === name);
    assert.equal(tile?.color, hex, `${name} keeps its authored color`);
  }

  // The spare 12th slot is transparent — unnamed pixels stay unused.
  const [, , , spareAlpha] = pixelAt(png, 3 * raw.tileSize, 2 * raw.tileSize);
  assert.equal(spareAlpha, 0, "the spare slot stays transparent");
});

// T6 (ticket #36, REQ-003): the lean name set is exact, duplicate-free, and
// mirrored field-for-field by the Tiled .tsx twin.
test("the lean name set is exact and consistent across json and tsx", () => {
  const raw = realTilesetJson();
  const names = raw.tiles.map((tile) => tile.name);
  assert.deepEqual(names, [
    "shadow_void",
    "stone_floor",
    "wall_face",
    "spawn_marker",
    "grass",
    "dirt",
    "cave_floor",
    "deep_water",
    "stairs_up",
    "stairs_down",
    "exit_marker",
  ]);
  assert.equal(new Set(names).size, names.length, "names are unique");

  const tsx = readFileSync(REAL_TSX_PATH, "utf8");
  const tsxNames = [...tsx.matchAll(/<property name="name" value="([^"]+)"\/>/g)].map(
    (match) => match[1],
  );
  assert.deepEqual(tsxNames, names, "tsx names mirror the JSON in order");
  const tsxColors = [...tsx.matchAll(/<property name="color" value="([^"]+)"\/>/g)].map(
    (match) => match[1],
  );
  assert.deepEqual(
    tsxColors,
    raw.tiles.map((tile) => tile.color),
    "tsx colors mirror the JSON in order",
  );
  assert.match(tsx, /tilecount="12" columns="4"/, "the tsx grid matches the 4x3 sheet");
});
