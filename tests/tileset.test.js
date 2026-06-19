import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import {
  KIND_TILE_NAMES,
  validateTileset,
  tileRect,
  kindTileRects,
  resolveTilesetUrl,
  DEFAULT_TILESET_URL,
} from "../src/client/tileset.js";

// The author tile-sheet contract (docs/tileset-contract.md): a descriptor plus
// named-tile rects. Tests load a committed sample sheet — 8px-native, since the
// real art is author-provided and deferred — so a malformed contract fails here
// loudly instead of silently blanking the map at runtime.
const here = dirname(fileURLToPath(import.meta.url));
const FIXTURE_PATH = join(here, "fixtures", "tilesets", "sample-8px.json");

function sampleTilesetJson() {
  return JSON.parse(readFileSync(FIXTURE_PATH, "utf8"));
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

// T1 (REQ-004): the committed 8px sample sheet validates and every name resolves
// end to end — including the 8px source unit (the model is size-agnostic).
test("the sample author tileset validates with every name resolvable", () => {
  const raw = sampleTilesetJson();
  const result = validateTileset(raw);
  assert.equal(result.ok, true, `sample sheet must validate: ${result.reason ?? ""}`);
  const { tileset } = result;
  assert.equal(tileset.tileSize, 8, "the sample sheet is 8px-native");
  assert.equal(tileset.columns, 2);
  assert.equal(tileset.rows, 2);
  assert.equal(tileset.image, "sample-author-8px.png");
  for (const tile of raw.tiles) {
    const rect = tileRect(tileset, tile.name);
    assert.deepEqual(
      rect,
      { sx: tile.x, sy: tile.y, sSize: 8 },
      `every name resolves to its authored rect (${tile.name})`,
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

// T3 (REQ-004): name -> rect resolution from the sample sheet, and null for the
// unknown name (the only absent-name path left after validation).
test("tileRect resolves sample names to authored rects and unknown names to null", () => {
  const { tileset } = validateTileset(sampleTilesetJson());
  assert.deepEqual(tileRect(tileset, "stone_floor"), { sx: 8, sy: 0, sSize: 8 });
  assert.deepEqual(tileRect(tileset, "spawn_marker"), { sx: 8, sy: 8, sSize: 8 });
  assert.equal(tileRect(tileset, "no_such_tile"), null);
  assert.equal(tileRect(tileset, "hasOwnProperty"), null, "prototype names are not tiles");
});

// T4 (REQ-004): the kind table covers exactly the four cell kinds, every name
// exists on the sample sheet, and kindTileRects resolves all four.
test("KIND_TILE_NAMES maps all four cell kinds onto sample tiles", () => {
  assert.deepEqual(
    Object.keys(KIND_TILE_NAMES).sort(),
    ["blocked", "current", "discovered", "empty"],
  );
  const { tileset } = validateTileset(sampleTilesetJson());
  const rects = kindTileRects(tileset);
  for (const [kind, name] of Object.entries(KIND_TILE_NAMES)) {
    assert.ok(tileset.byName[name], `'${name}' (${kind}) exists on the sheet`);
    assert.deepEqual(rects[kind], tileRect(tileset, name), `${kind} resolves through the table`);
  }
});

// ---- S3.1 (ticket #54): the map tileset URL defaults to the committed sheet ----

test("resolveTilesetUrl: an unset / blank / non-string override resolves to the default sheet (REQ-001)", () => {
  for (const override of [undefined, null, "", "   ", "\t\n", 123, {}, []]) {
    assert.equal(
      resolveTilesetUrl(override),
      DEFAULT_TILESET_URL,
      `${JSON.stringify(override)} -> default`,
    );
  }
  assert.equal(DEFAULT_TILESET_URL, "/tilesets/arctic.json");
});

test("resolveTilesetUrl: a non-blank override wins and is trimmed (REQ-002)", () => {
  assert.equal(resolveTilesetUrl("/custom/sheet.json"), "/custom/sheet.json");
  assert.equal(resolveTilesetUrl("  /trim.json  "), "/trim.json");
  assert.equal(resolveTilesetUrl("https://cdn.example/x.json"), "https://cdn.example/x.json");
});
