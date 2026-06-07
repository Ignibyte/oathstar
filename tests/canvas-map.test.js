import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import {
  MAP_PALETTE,
  canvasSize,
  cellKind,
  toDrawPlan,
  mapAriaLabel,
} from "../src/client/canvas-map.js";
import { toMapModel } from "../src/client/map.js";

function cell(overrides = {}) {
  return {
    x: 0,
    y: 0,
    present: true,
    id: "r",
    title: "Room",
    glyph: "+",
    discovered: true,
    current: false,
    passable: true,
    ...overrides,
  };
}

// A two-floor snapshot stacked at (0,0): current room on z=0 with a blocked
// (non-passable) neighbour; a room on z=1 that must NOT render.
function stackedSnapshot() {
  return {
    region: "test",
    currentRoomId: "a",
    rooms: [
      { id: "a", title: "Square", x: 0, y: 0, z: 0, glyph: "+", passable: true, discovered: true, current: true, exits: {} },
      { id: "b", title: "Wall", x: 1, y: 0, z: 0, glyph: "#", passable: false, discovered: true, current: false, exits: {} },
      { id: "c", title: "Tower", x: 0, y: 0, z: 1, glyph: "^", passable: true, discovered: true, current: false, exits: {} },
    ],
  };
}

test("canvasSize scales the backing store by devicePixelRatio (REQ-005)", () => {
  const model = { columns: 3, rows: 2, tilePixels: 32 };
  const at1 = canvasSize(model, 1);
  assert.equal(at1.cssWidth, 96);
  assert.equal(at1.cssHeight, 64);
  assert.equal(at1.backingWidth, 96);
  assert.equal(at1.backingHeight, 64);

  const at2 = canvasSize(model, 2);
  assert.equal(at2.cssWidth, 96, "CSS size is dpr-independent");
  assert.equal(at2.backingWidth, 192);
  assert.equal(at2.backingHeight, 128);

  const frac = canvasSize({ columns: 1, rows: 1, tilePixels: 31 }, 1.5);
  assert.equal(frac.backingWidth, Math.round(31 * 1.5)); // 47 (rounded)

  for (const bad of [0, -2, Number.NaN, Infinity, undefined]) {
    assert.equal(canvasSize(model, bad).dpr, 1, `dpr ${bad} clamps to 1`);
  }
});

test("cellKind classifies empty/discovered/current/blocked (REQ-004)", () => {
  assert.equal(cellKind(cell({ present: false })), "empty");
  assert.equal(cellKind(cell({ current: true })), "current");
  assert.equal(cellKind(cell({ discovered: true, passable: false })), "blocked");
  assert.equal(cellKind(cell({ discovered: true, passable: true })), "discovered");
  // passable must be STRICTLY false to block; undefined/null stay "discovered"
  assert.equal(cellKind(cell({ discovered: true, passable: undefined })), "discovered");
  assert.equal(cellKind(cell({ discovered: true, passable: null })), "discovered");
  // present but undiscovered stays fog (matches the prior DOM renderer)
  assert.equal(cellKind(cell({ discovered: false })), "empty");
  // undiscovered non-passable is still fog (walls aren't revealed before discovery)
  assert.equal(cellKind(cell({ discovered: false, passable: false })), "empty");
  // current wins over blocked
  assert.equal(cellKind(cell({ current: true, passable: false })), "current");
});

test("toDrawPlan positions cells and carries kind/label/colors (REQ-001/004)", () => {
  const model = {
    mode: "glyph",
    tilePixels: 32,
    columns: 2,
    rows: 1,
    minX: 3,
    minY: 5,
    cells: [
      cell({ x: 3, y: 5, current: true, title: "Here" }),
      cell({ x: 4, y: 5, discovered: true, passable: false, glyph: "#", title: "Wall" }),
    ],
  };
  const plan = toDrawPlan(model);
  assert.equal(plan.width, 64);
  assert.equal(plan.height, 32);
  assert.equal(plan.ops.length, 2);

  const [here, wall] = plan.ops;
  assert.deepEqual([here.x, here.y, here.size], [0, 0, 32], "minX/minY offset applied");
  assert.equal(here.kind, "current");
  assert.equal(here.fill, MAP_PALETTE.current.fill);
  assert.equal(here.glyph, "+", "the room glyph is the on-tile mark");
  assert.equal(here.here, true);
  assert.equal(plan.glyphFontPx, 13, "32px tile -> 13px glyph font");

  assert.deepEqual([wall.x, wall.y], [32, 0], "second column at one tile over");
  assert.equal(wall.kind, "blocked");
  assert.equal(wall.stroke, MAP_PALETTE.blocked.stroke);
});

test("toDrawPlan draws the glyph on-tile, floors the glyph font, blanks empty cells", () => {
  const model = {
    mode: "glyph",
    tilePixels: 16,
    columns: 2,
    rows: 1,
    minX: 0,
    minY: 0,
    cells: [
      cell({ x: 0, y: 0, discovered: true, glyph: "@", title: "Square" }),
      cell({ x: 1, y: 0, present: false }),
    ],
  };
  const plan = toDrawPlan(model);
  assert.equal(plan.tile, 16);
  assert.equal(plan.glyphFontPx, 9, "16px tile floors the glyph font at 9px (REQ-005)");
  assert.equal(plan.ops[0].glyph, "@", "the room glyph is drawn on-tile (title goes to aria)");
  assert.equal(plan.ops[0].size, 16);
  assert.equal(plan.ops[1].kind, "empty");
  assert.equal(plan.ops[1].textColor, null, "empty cells draw no glyph (null text color)");
});

test("toMapModel + toDrawPlan render only the current z-plane (REQ-003)", () => {
  const model = toMapModel(stackedSnapshot());
  assert.equal(model.z, 0);
  assert.deepEqual(model.planes, [0, 1]);
  const plan = toDrawPlan(model);
  // plane z=0 has 2 rooms (a,b) in a 2x1 box; the z=1 tower is excluded.
  assert.equal(plan.ops.length, 2);
  assert.ok(plan.ops.some((op) => op.kind === "current"));
  assert.ok(plan.ops.some((op) => op.kind === "blocked"));
});

test("toMapModel cell carries passable; the server snapshot is untouched (REQ-004)", () => {
  const snap = stackedSnapshot();
  const model = toMapModel(snap);
  const wall = model.cells.find((c) => c.id === "b");
  assert.equal(wall.passable, false, "passable flows into the cell model");
  const square = model.cells.find((c) => c.id === "a");
  assert.equal(square.passable, true);
  // the derived model must not mutate the input rooms
  assert.ok(!("present" in snap.rooms[0]));
});

test("mapAriaLabel summarizes region, floor, count, and current room (REQ-006)", () => {
  const label = mapAriaLabel(toMapModel(stackedSnapshot()));
  assert.match(label, /^Map: test/);
  assert.match(label, /floor 0/);
  assert.match(label, /2 rooms discovered/);
  assert.match(label, /here: Square/);
});

test("mapAriaLabel handles singular and empty maps (REQ-006)", () => {
  const one = mapAriaLabel({
    region: "Keep",
    planes: [0],
    z: 0,
    cells: [{ present: true, discovered: true, current: true, title: "Lone", glyph: "+" }],
  });
  assert.match(one, /1 room discovered/);
  assert.match(one, /here: Lone/);

  const empty = mapAriaLabel(toMapModel({ region: "x", currentRoomId: null, rooms: [] }));
  assert.match(empty, /0 rooms discovered/);
  assert.doesNotThrow(() => mapAriaLabel(toMapModel({ rooms: [] })));
});

test("no game-engine dependency is declared (REQ-007)", () => {
  const root = join(dirname(fileURLToPath(import.meta.url)), "..");
  const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  const deps = { ...(pkg.dependencies || {}), ...(pkg.devDependencies || {}) };
  for (const banned of ["phaser", "pixi.js", "pixi", "kiwi.js", "melonjs"]) {
    assert.ok(!(banned in deps), `must not depend on ${banned}`);
  }
});
