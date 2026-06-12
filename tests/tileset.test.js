import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import { KIND_TILE_NAMES, validateTileset, tileRect, kindTileRects } from "../src/client/tileset.js";

// The committed sheet metadata IS the contract (ticket #32): tests read the
// real served file, so regenerating with different names/geometry fails here
// loudly instead of silently blanking the map at runtime.
const here = dirname(fileURLToPath(import.meta.url));
const REAL_JSON_PATH = join(
  here,
  "..",
  "public",
  "tilesets",
  "oathstar-starter-16x16",
  "oathstar-starter-16x16.json",
);

function realTilesetJson() {
  return JSON.parse(readFileSync(REAL_JSON_PATH, "utf8"));
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
  assert.equal(tileset.columns, 8);
  assert.equal(tileset.rows, 8);
  assert.equal(tileset.image, "oathstar-starter-16x16.png");
  assert.equal(tileset.tiles.length, 64, "the starter sheet is 8x8");
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
  assert.deepEqual(tileRect(tileset, "grass"), { sx: 0, sy: 0, sSize: 16 });
  assert.deepEqual(tileRect(tileset, "stone_floor"), { sx: 64, sy: 0, sSize: 16 });
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
