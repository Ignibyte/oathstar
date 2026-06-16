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
