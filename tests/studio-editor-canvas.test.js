// node --test suite for the studio editor's pure draw model (ticket #45). The
// module is DOM-free, so these tests load it directly and assert the draw plan,
// classification, Hi-DPI sizing, aria-label, and validate-result formatting. The
// canvas/fetch/DOM glue is a separate browser-only seam (render.rs EDITOR_GLUE)
// and is not exercised here.

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  EDITOR_PALETTE,
  editorCellKind,
  editorDrawPlan,
  editorCanvasSize,
  editorAriaLabel,
  formatValidateResult,
  formatSaveResult,
  formatActivateResult,
  tileIndexToSourceRect,
  canvasPointToCell,
  paletteIndexAtPoint,
  paintCell,
  cellsInRect,
  paintRect,
} from "../crates/oathstar-studio/static/editor-canvas.js";

/** A small MapDocument with sane defaults; override fields per test. */
function doc(overrides = {}) {
  return {
    id: "d",
    title: "Doc",
    tile_size: 16,
    width: 3,
    height: 2,
    floors: 1,
    terrain_palette: {
      floor: { tile: "f", passable: true },
      wall: { tile: "w", passable: false },
    },
    terrain: [],
    regions: {},
    subregions: {},
    rooms: [],
    spawn: null,
    ...overrides,
  };
}

test("editorCellKind: five kinds with spawn>room>floor>wall>empty precedence + wall fallback", () => {
  const d = doc({
    terrain: [
      { x: 0, y: 0, z: 0, terrain: "floor" },
      { x: 1, y: 0, z: 0, terrain: "wall" },
      { x: 2, y: 0, z: 0, terrain: "ghost" }, // names a palette key that does not exist
    ],
    rooms: [{ x: 0, y: 1, z: 0, id: "r", region: "reg" }],
    spawn: { x: 0, y: 1, z: 0 }, // sits on the room cell
  });
  assert.equal(editorCellKind(d, 0, 0, 0), "floor");
  assert.equal(editorCellKind(d, 1, 0, 0), "wall");
  assert.equal(editorCellKind(d, 2, 0, 0), "wall"); // missing-palette → wall fallback
  assert.equal(editorCellKind(d, 0, 1, 0), "spawn"); // spawn wins over the room beneath
  assert.equal(editorCellKind(d, 2, 1, 0), "empty");
});

test("editorCellKind: a room without a spawn is 'room'; a spawn on another z does not match", () => {
  const d = doc({
    rooms: [{ x: 1, y: 1, z: 0, id: "r", region: "reg" }],
    spawn: { x: 1, y: 1, z: 1 }, // different z-plane
  });
  assert.equal(editorCellKind(d, 1, 1, 0), "room");
});

test("editorDrawPlan: deterministic, row-major, width*height ops; glyph only on rooms", () => {
  const d = doc({
    terrain: [{ x: 0, y: 0, z: 0, terrain: "floor" }],
    rooms: [{ x: 2, y: 1, z: 0, id: "r", region: "reg", glyph: "@" }],
  });
  const plan = editorDrawPlan(d, { z: 0, tilePixels: 10 });
  assert.equal(plan.ops.length, 6); // width(3) * height(2)
  assert.equal(plan.width, 30);
  assert.equal(plan.height, 20);
  assert.equal(plan.tile, 10);
  assert.deepEqual(
    plan.ops.map((o) => [o.x, o.y]),
    [
      [0, 0],
      [10, 0],
      [20, 0],
      [0, 10],
      [10, 10],
      [20, 10],
    ],
  );
  const floor = plan.ops[0];
  assert.equal(floor.kind, "floor");
  assert.equal(floor.fill, EDITOR_PALETTE.floor.fill);
  assert.equal(floor.stroke, EDITOR_PALETTE.floor.stroke);
  assert.equal(floor.textColor, EDITOR_PALETTE.floor.text);
  assert.equal(floor.glyph, null); // a non-room cell carries no glyph
  const room = plan.ops[5]; // (2, 1)
  assert.equal(room.kind, "room");
  assert.equal(room.glyph, "@");
  // determinism: identical input → identical plan
  assert.deepEqual(editorDrawPlan(d, { z: 0, tilePixels: 10 }), plan);
});

test("editorDrawPlan: a room without a glyph override defaults to '.'", () => {
  const d = doc({ rooms: [{ x: 0, y: 0, z: 0, id: "r", region: "reg" }] });
  const plan = editorDrawPlan(d, { z: 0, tilePixels: 8 });
  assert.equal(plan.ops[0].glyph, ".");
});

test("editorCanvasSize: backing = round(css*dpr); non-finite/non-positive dpr clamps to 1", () => {
  const d = doc({ width: 4, height: 3 });
  assert.deepEqual(editorCanvasSize(d, { tilePixels: 10, devicePixelRatio: 2 }), {
    cssWidth: 40,
    cssHeight: 30,
    backingWidth: 80,
    backingHeight: 60,
    dpr: 2,
  });
  assert.deepEqual(editorCanvasSize(d, { tilePixels: 10, devicePixelRatio: 1.5 }), {
    cssWidth: 40,
    cssHeight: 30,
    backingWidth: 60,
    backingHeight: 45,
    dpr: 1.5,
  });
  for (const bad of [0, -1, NaN, Infinity]) {
    const size = editorCanvasSize(d, { tilePixels: 10, devicePixelRatio: bad });
    assert.equal(size.dpr, 1);
    assert.equal(size.backingWidth, 40);
    assert.equal(size.backingHeight, 30);
  }
  // dpr defaults to 1 when omitted.
  assert.equal(editorCanvasSize(d, { tilePixels: 10 }).dpr, 1);
});

test("editorAriaLabel: title + plural counts + spawn coordinate", () => {
  const d = doc({
    title: "Sketch",
    terrain: [
      { x: 0, y: 0, z: 0, terrain: "floor" },
      { x: 1, y: 0, z: 0, terrain: "floor" },
    ],
    rooms: [
      { x: 0, y: 0, z: 0, id: "a", region: "r" },
      { x: 1, y: 0, z: 0, id: "b", region: "r" },
    ],
    spawn: { x: 0, y: 0, z: 0 },
  });
  assert.equal(
    editorAriaLabel(d),
    "Map editor: Sketch — 2 rooms, 2 terrain cells, spawn at (0, 0, 0)",
  );
});

test("editorAriaLabel: singular counts and the no-spawn branch", () => {
  const d = doc({
    title: "One",
    terrain: [{ x: 0, y: 0, z: 0, terrain: "floor" }],
    rooms: [{ x: 0, y: 0, z: 0, id: "a", region: "r" }],
    spawn: null,
  });
  assert.equal(editorAriaLabel(d), "Map editor: One — 1 room, 1 terrain cell, no spawn");
});

test("formatValidateResult: ok:true summary (singular + plural) and the message/fallback path", () => {
  assert.deepEqual(
    formatValidateResult({ ok: true, room_count: 1, region_count: 1, start_room_id: "a" }),
    { ok: true, headline: "Valid map", detail: "1 room, 1 region, start: a" },
  );
  assert.deepEqual(
    formatValidateResult({ ok: true, room_count: 2, region_count: 3, start_room_id: "entry" }),
    { ok: true, headline: "Valid map", detail: "2 rooms, 3 regions, start: entry" },
  );
  const failed = formatValidateResult({ ok: false, message: "cell (1, 2, 0) is out of bounds" });
  assert.equal(failed.ok, false);
  assert.equal(failed.headline, "Invalid map");
  assert.equal(failed.detail, "cell (1, 2, 0) is out of bounds");
  // missing message, and null/undefined bodies, fall back.
  assert.equal(formatValidateResult({ ok: false }).detail, "validation failed");
  assert.equal(formatValidateResult(null).detail, "validation failed");
  assert.equal(formatValidateResult(undefined).ok, false);
  // a non-strict ok (e.g. the string "true") is treated as a failure.
  assert.equal(formatValidateResult({ ok: "true" }).ok, false);
});

test("formatSaveResult: ok names the saved slot; a refusal / null renders the message (#55)", () => {
  assert.deepEqual(formatSaveResult({ ok: true, id: "vale" }), {
    ok: true,
    headline: "Saved",
    detail: "as vale",
  });
  const refused = formatSaveResult({ ok: false, message: "map id is not a valid storage name" });
  assert.equal(refused.ok, false);
  assert.equal(refused.headline, "Not saved");
  assert.equal(refused.detail, "map id is not a valid storage name");
  // missing message, and null/undefined bodies, fall back.
  assert.equal(formatSaveResult({ ok: false }).detail, "save failed");
  assert.equal(formatSaveResult(null).detail, "save failed");
  assert.equal(formatSaveResult(undefined).ok, false);
  // a non-strict ok (e.g. the string "true") is treated as a failure.
  assert.equal(formatSaveResult({ ok: "true" }).ok, false);
});

test("formatActivateResult: ok confirms the active world; a refusal / null renders the message (#60)", () => {
  assert.deepEqual(formatActivateResult({ ok: true }), {
    ok: true,
    headline: "Active world set",
    detail: "Restart the game server to play it.",
  });
  const refused = formatActivateResult({ ok: false, message: "tile size 7 is not supported" });
  assert.equal(refused.ok, false);
  assert.equal(refused.headline, "Not activated");
  assert.equal(refused.detail, "tile size 7 is not supported");
  // missing message, and null/undefined bodies, fall back.
  assert.equal(formatActivateResult({ ok: false }).detail, "the world does not materialize");
  assert.equal(formatActivateResult(null).detail, "the world does not materialize");
  assert.equal(formatActivateResult(undefined).ok, false);
  // a non-strict ok (e.g. the string "true") is treated as a failure.
  assert.equal(formatActivateResult({ ok: "true" }).ok, false);
});

// ---- ticket #48: paint helpers ----

test("tileIndexToSourceRect: row-major source rect on a 30-wide 8px sheet", () => {
  assert.deepEqual(tileIndexToSourceRect(0, 30, 8), { sx: 0, sy: 0, size: 8 });
  assert.deepEqual(tileIndexToSourceRect(29, 30, 8), { sx: 232, sy: 0, size: 8 });
  assert.deepEqual(tileIndexToSourceRect(30, 30, 8), { sx: 0, sy: 8, size: 8 }); // wraps to row 1
  assert.deepEqual(tileIndexToSourceRect(31, 30, 8), { sx: 8, sy: 8, size: 8 });
});

test("canvasPointToCell: floor inside the grid; null outside and at the far edges", () => {
  assert.equal(canvasPointToCell(-1, 5, 10, 3, 2), null);
  assert.equal(canvasPointToCell(5, -1, 10, 3, 2), null);
  assert.deepEqual(canvasPointToCell(5, 5, 10, 3, 2), { x: 0, y: 0 });
  assert.deepEqual(canvasPointToCell(25, 15, 10, 3, 2), { x: 2, y: 1 });
  assert.equal(canvasPointToCell(30, 5, 10, 3, 2), null, "px == width*tile is outside");
  assert.equal(canvasPointToCell(5, 20, 10, 3, 2), null, "py == height*tile is outside");
  assert.deepEqual(canvasPointToCell(29, 19, 10, 3, 2), { x: 2, y: 1 }, "last pixel -> last cell");
});

test("paletteIndexAtPoint: row-major index; null past the columns or the tile count", () => {
  // columns 4, tileSize 8, scale 2 -> 16px palette cells; 10 tiles (partial last row).
  assert.equal(paletteIndexAtPoint(0, 0, 4, 8, 2, 10), 0);
  assert.equal(paletteIndexAtPoint(20, 0, 4, 8, 2, 10), 1);
  assert.equal(paletteIndexAtPoint(0, 20, 4, 8, 2, 10), 4);
  assert.equal(paletteIndexAtPoint(40, 20, 4, 8, 2, 10), 6);
  assert.equal(paletteIndexAtPoint(64, 0, 4, 8, 2, 10), null, "col 4 >= columns");
  assert.equal(paletteIndexAtPoint(40, 32, 4, 8, 2, 10), null, "index 10 >= tileCount");
  assert.equal(paletteIndexAtPoint(-1, 0, 4, 8, 2, 10), null);
});

test("paintCell: inserts, replaces in place, and never mutates the input", () => {
  const d = {
    width: 4,
    height: 4,
    floors: 1,
    layers: [{ id: "ground", name: "Ground", kind: "tile", visible: true, cells: [] }],
  };
  const snapshot = JSON.stringify(d);

  const painted = paintCell(d, "ground", { x: 1, y: 2, z: 0 }, { tileset: "arctic", index: 5 });
  assert.equal(JSON.stringify(d), snapshot, "input doc is not mutated");
  assert.deepEqual(painted.layers[0].cells, [{ x: 1, y: 2, z: 0, tileset: "arctic", index: 5 }]);

  const repainted = paintCell(painted, "ground", { x: 1, y: 2, z: 0 }, { tileset: "arctic", index: 9 });
  assert.deepEqual(
    repainted.layers[0].cells,
    [{ x: 1, y: 2, z: 0, tileset: "arctic", index: 9 }],
    "same coordinate is replaced, not duplicated",
  );

  const two = paintCell(repainted, "ground", { x: 2, y: 2, z: 0 }, { tileset: "arctic", index: 1 });
  assert.equal(two.layers[0].cells.length, 2, "a new coordinate appends");

  const noop = paintCell(d, "missing", { x: 0, y: 0, z: 0 }, { tileset: "arctic", index: 0 });
  assert.deepEqual(noop.layers[0].cells, [], "a missing layer is a no-op");
});

test("cellsInRect: inclusive, normalized, row-major (#57)", () => {
  const rowMajor = [
    [0, 0],
    [1, 0],
    [2, 0],
    [0, 1],
    [1, 1],
    [2, 1],
  ];
  assert.deepEqual(cellsInRect({ x: 0, y: 0 }, { x: 2, y: 1 }).map((c) => [c.x, c.y]), rowMajor);
  // reversed corners -> the same set (normalized via min/max)
  assert.deepEqual(cellsInRect({ x: 2, y: 1 }, { x: 0, y: 0 }).map((c) => [c.x, c.y]), rowMajor);
  // a 1x1 (a == b) -> exactly one cell
  assert.deepEqual(cellsInRect({ x: 3, y: 4 }, { x: 3, y: 4 }), [{ x: 3, y: 4 }]);
  // a row and a column
  assert.deepEqual(cellsInRect({ x: 0, y: 5 }, { x: 2, y: 5 }).map((c) => c.x), [0, 1, 2]);
  assert.deepEqual(cellsInRect({ x: 5, y: 0 }, { x: 5, y: 2 }).map((c) => c.y), [0, 1, 2]);
});

test("paintRect: fills the rect on the layer/z immutably; 1x1 == one paintCell (#57)", () => {
  const d = {
    layers: [{ id: "ground", name: "Ground", kind: "tile", visible: true, cells: [] }],
  };
  const snapshot = JSON.stringify(d);
  const painted = paintRect(d, "ground", { x: 0, y: 0 }, { x: 1, y: 1 }, 0, {
    tileset: "arctic",
    index: 7,
  });
  assert.equal(JSON.stringify(d), snapshot, "input doc is not mutated");
  assert.deepEqual(
    painted.layers[0].cells.map((c) => [c.x, c.y, c.z, c.tileset, c.index]),
    [
      [0, 0, 0, "arctic", 7],
      [1, 0, 0, "arctic", 7],
      [0, 1, 0, "arctic", 7],
      [1, 1, 0, "arctic", 7],
    ],
    "all four cells of the 2x2 rect carry the tile + z",
  );
  // a 1x1 rectangle equals a single paintCell
  assert.deepEqual(
    paintRect(d, "ground", { x: 2, y: 3 }, { x: 2, y: 3 }, 0, { tileset: "arctic", index: 4 }),
    paintCell(d, "ground", { x: 2, y: 3, z: 0 }, { tileset: "arctic", index: 4 }),
  );
});

test("paintCell: a layer without a cells array gains the painted cell", () => {
  const d = { layers: [{ id: "ground" }] };
  const painted = paintCell(d, "ground", { x: 0, y: 0, z: 0 }, { tileset: "arctic", index: 3 });
  assert.deepEqual(painted.layers[0].cells, [{ x: 0, y: 0, z: 0, tileset: "arctic", index: 3 }]);
});

test("editorDrawPlan: ops gain sprites[]; painted layer cells become sprite ops (z + tileset filtered)", () => {
  const base = doc({ width: 3, height: 2 });
  for (const op of editorDrawPlan(base, { z: 0, tilePixels: 10 }).ops) {
    assert.deepEqual(op.sprites, [], "no tilesets/layers -> empty sprites");
  }

  const withPaint = doc({
    width: 3,
    height: 2,
    tilesets: [{ id: "arctic", image: "arctic.png", tile_size: 8, columns: 30, rows: 203 }],
    layers: [
      {
        id: "ground",
        name: "Ground",
        kind: "tile",
        visible: true,
        cells: [
          { x: 1, y: 0, z: 0, tileset: "arctic", index: 31 },
          { x: 2, y: 1, z: 1, tileset: "arctic", index: 0 }, // other z -> excluded
          { x: 0, y: 0, z: 0, tileset: "ghost", index: 0 }, // unknown tileset -> skipped
        ],
      },
    ],
  });
  const plan = editorDrawPlan(withPaint, { z: 0, tilePixels: 10 });
  const at = (x, y) => plan.ops.find((o) => o.x === x * 10 && o.y === y * 10);
  assert.deepEqual(at(1, 0).sprites, [{ sx: 8, sy: 8, sSize: 8, tileset: "arctic" }]);
  assert.deepEqual(at(0, 0).sprites, [], "unknown tileset skipped");
  assert.deepEqual(at(2, 1).sprites, [], "other-z cell excluded");
});

test("editorDrawPlan: overlapping layers stack bottom-first in the sprite list", () => {
  const d = doc({
    width: 2,
    height: 1,
    tilesets: [{ id: "arctic", image: "arctic.png", tile_size: 8, columns: 30, rows: 203 }],
    layers: [
      { id: "a", name: "A", kind: "tile", visible: true, cells: [{ x: 0, y: 0, z: 0, tileset: "arctic", index: 0 }] },
      { id: "b", name: "B", kind: "tile", visible: true, cells: [{ x: 0, y: 0, z: 0, tileset: "arctic", index: 1 }] },
    ],
  });
  const plan = editorDrawPlan(d, { z: 0, tilePixels: 10 });
  assert.deepEqual(plan.ops[0].sprites, [
    { sx: 0, sy: 0, sSize: 8, tileset: "arctic" },
    { sx: 8, sy: 0, sSize: 8, tileset: "arctic" },
  ]);
});

test("editorDrawPlan: focusSubregion flags only the focused sub-region's room cells (#51c)", () => {
  const d = doc({
    rooms: [
      { x: 0, y: 0, z: 0, id: "a", region: "reg", subregion: "vale" },
      { x: 1, y: 0, z: 0, id: "b", region: "reg", subregion: "other" },
      { x: 2, y: 0, z: 0, id: "c", region: "reg" },
    ],
  });
  const ops = editorDrawPlan(d, { z: 0, tilePixels: 10, focusSubregion: "vale" }).ops;
  assert.equal(ops[0].focused, true); // (0,0) room in the focused sub-region
  assert.equal(ops[1].focused, false); // (1,0) room in another sub-region
  assert.equal(ops[2].focused, false); // (2,0) room with no subregion
  assert.equal(ops[3].focused, false); // (0,1) a non-room cell
});

test("editorDrawPlan: no / unknown focusSubregion leaves every cell unfocused (#51c)", () => {
  const d = doc({
    rooms: [{ x: 0, y: 0, z: 0, id: "a", region: "reg", subregion: "vale" }],
  });
  for (const op of editorDrawPlan(d, { z: 0, tilePixels: 10 }).ops) {
    assert.equal(op.focused, false); // default: no focus
  }
  for (const op of editorDrawPlan(d, { z: 0, tilePixels: 10, focusSubregion: "ghost" }).ops) {
    assert.equal(op.focused, false); // an unknown sub-region matches nothing
  }
});
