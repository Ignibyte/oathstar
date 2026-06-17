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
import { toMapModel, DEFAULT_MAP_CONFIG } from "../src/client/map.js";

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

// ticket #37 (REQ-002): the default render config enlarges each cell to 64px —
// a crisp integer 2x of the 32px source — so the on-page map is physically ~2x
// the prior build. canvasSize derives both CSS and backing px from tilePixels.
test("the default map config enlarges cells to 64px for a bigger on-page map (REQ-002)", () => {
  assert.equal(DEFAULT_MAP_CONFIG.tilePixels, 64, "default cell is the enlarged 64px");
  const model = toMapModel(stackedSnapshot()); // built with the default config
  assert.equal(model.tilePixels, 64);
  // the z=0 plane is a 2x1 box -> the canvas is columns*64 css px...
  const at1 = canvasSize(model, 1);
  assert.equal(at1.cssWidth, model.columns * 64);
  assert.equal(at1.cssHeight, model.rows * 64);
  // ...doubled into the backing store for hi-dpi crispness, css unchanged.
  const at2 = canvasSize(model, 2);
  assert.equal(at2.backingWidth, model.columns * 64 * 2, "backing scales 2x for crispness");
  assert.equal(at2.cssWidth, model.columns * 64, "css size is dpr-independent");
  // every drawn cell takes the enlarged 64px destination.
  const plan = toDrawPlan(model);
  assert.equal(plan.tile, 64);
  assert.ok(plan.ops.every((op) => op.size === 64), "every op draws at the enlarged 64px cell");
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
  // Classification is observable through the kind-distinct palette fields the
  // seam draws (PR-oathstar-render-plan-test-002); cellKind has direct tests.
  assert.equal(here.fill, MAP_PALETTE.current.fill);
  assert.equal(here.textColor, MAP_PALETTE.current.text);
  assert.equal(here.glyph, "+", "the room glyph is the on-tile mark");
  assert.equal(plan.glyphFontPx, 13, "32px tile -> 13px glyph font");

  assert.deepEqual([wall.x, wall.y], [32, 0], "second column at one tile over");
  assert.equal(wall.fill, MAP_PALETTE.blocked.fill);
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
  assert.equal(plan.ops[1].fill, MAP_PALETTE.empty.fill, "absent cells stay fog");
  assert.equal(plan.ops[1].textColor, null, "empty cells draw no glyph (null text color)");
});

test("toMapModel + toDrawPlan render only the current z-plane (REQ-003)", () => {
  const model = toMapModel(stackedSnapshot());
  assert.equal(model.z, 0);
  assert.deepEqual(model.planes, [0, 1]);
  const plan = toDrawPlan(model);
  // plane z=0 has 2 rooms (a,b) in a 2x1 box; the z=1 tower is excluded.
  assert.equal(plan.ops.length, 2);
  assert.ok(plan.ops.some((op) => op.fill === MAP_PALETTE.current.fill));
  assert.ok(plan.ops.some((op) => op.fill === MAP_PALETTE.blocked.fill));
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

// ---- ticket #32: sprite tiles in the draw plan ----

import { validateTileset, KIND_TILE_NAMES, tileRect } from "../src/client/tileset.js";

function authorTileset() {
  const root = join(dirname(fileURLToPath(import.meta.url)), "..");
  const raw = JSON.parse(
    readFileSync(
      join(root, "tests", "fixtures", "tilesets", "sample-8px.json"),
      "utf8",
    ),
  );
  const result = validateTileset(raw);
  assert.equal(result.ok, true, "the sample author tileset validates");
  return result.tileset;
}

// T5 (REQ-003): without a tileset the plan is the flat-color contract — every
// palette field the seam's fallback draws, plus sprite: null.
test("toDrawPlan without a tileset keeps the flat-color contract (REQ-003)", () => {
  const model = {
    mode: "glyph",
    tilePixels: 32,
    columns: 2,
    rows: 1,
    minX: 0,
    minY: 0,
    cells: [
      cell({ x: 0, y: 0, current: true }),
      cell({ x: 1, y: 0, discovered: true, passable: false }),
    ],
  };
  for (const plan of [toDrawPlan(model), toDrawPlan(model, null)]) {
    const [current, blocked] = plan.ops;
    assert.equal(current.sprite, null, "no tileset -> no sprite rect");
    assert.equal(blocked.sprite, null);
    assert.equal(current.fill, MAP_PALETTE.current.fill, "fallback fill retained");
    assert.equal(blocked.fill, MAP_PALETTE.blocked.fill);
    assert.equal(blocked.stroke, MAP_PALETTE.blocked.stroke);
  }
});

// T6 (REQ-002/003): with the real tileset every op carries its kind's sheet
// rect — the exact field the seam blits — while the fallback fields remain.
test("toDrawPlan with the sample tileset resolves a sprite rect per kind (REQ-002)", () => {
  const tileset = authorTileset();
  const model = {
    mode: "glyph",
    tilePixels: 32,
    columns: 2,
    rows: 2,
    minX: 0,
    minY: 0,
    cells: [
      cell({ x: 0, y: 0, current: true }),
      cell({ x: 1, y: 0, discovered: true, passable: false }),
      cell({ x: 0, y: 1, discovered: true }),
      cell({ x: 1, y: 1, present: false }),
    ],
  };
  const plan = toDrawPlan(model, tileset);
  const [current, blocked, discovered, empty] = plan.ops;
  assert.deepEqual(current.sprite, tileRect(tileset, KIND_TILE_NAMES.current));
  assert.deepEqual(blocked.sprite, tileRect(tileset, KIND_TILE_NAMES.blocked));
  assert.deepEqual(discovered.sprite, tileRect(tileset, KIND_TILE_NAMES.discovered));
  assert.deepEqual(empty.sprite, tileRect(tileset, KIND_TILE_NAMES.empty));
  assert.equal(current.sprite.sSize, 8, "source tiles are the sheet's 8px");
  assert.equal(current.size, 32, "dest tiles stay the configured tilePixels");
  // The fallback fields survive (the seam uses them until the image loads).
  assert.equal(current.fill, MAP_PALETTE.current.fill);
  assert.equal(blocked.stroke, MAP_PALETTE.blocked.stroke);
});

// T7 (REQ-006): glyph + text + aria are mode-independent — byte-identical
// plans apart from the sprite field, and the same aria summary.
test("tileset rendering leaves glyph, text, and aria behavior unchanged (REQ-006)", () => {
  const model = toMapModel(stackedSnapshot());
  const flat = toDrawPlan(model);
  const tiled = toDrawPlan(model, authorTileset());
  assert.equal(flat.glyphFontPx, tiled.glyphFontPx);
  assert.equal(flat.ops.length, tiled.ops.length);
  for (let i = 0; i < flat.ops.length; i += 1) {
    const { sprite: _flatSprite, ...flatRest } = flat.ops[i];
    const { sprite: _tiledSprite, ...tiledRest } = tiled.ops[i];
    assert.deepEqual(tiledRest, flatRest, `op ${i} differs only in sprite`);
  }
  assert.equal(mapAriaLabel(model), mapAriaLabel(model), "aria is model-only");
});

// ---- ticket #33: presence markers in the draw plan ----

import { MAP_MARKER_COLORS } from "../src/client/canvas-map.js";

// M-T7 (REQ-003): the model cell carries both flags, defaulting false when
// the wire omits them; the input snapshot is never mutated.
test("toMapModel cells carry marker flags with omit-false defaults (REQ-003)", () => {
  const snap = {
    region: "test",
    currentRoomId: "a",
    rooms: [
      { id: "a", title: "Sq", x: 0, y: 0, z: 0, glyph: "+", passable: true, discovered: true, current: true, exits: {}, hasHostiles: true, hasItems: true },
      { id: "b", title: "Rd", x: 1, y: 0, z: 0, glyph: ".", passable: true, discovered: true, current: false, exits: {} },
    ],
  };
  const model = toMapModel(snap);
  const a = model.cells.find((c) => c.id === "a");
  const b = model.cells.find((c) => c.id === "b");
  assert.equal(a.hasHostiles, true);
  assert.equal(a.hasItems, true);
  assert.equal(b.hasHostiles, false, "absent wire key coerces false");
  assert.equal(b.hasItems, false);
  assert.ok(!("hasHostiles" in snap.rooms[1]), "input snapshot untouched");
});

// M-T8 (REQ-003): exact marker geometry — the fields the seam's arc() draws.
test("toDrawPlan resolves exact marker geometry per flag (REQ-003)", () => {
  const both = cell({ x: 0, y: 0, discovered: true, hasHostiles: true, hasItems: true });
  const hostileOnly = cell({ x: 1, y: 0, discovered: true, hasHostiles: true, hasItems: false });
  const none = cell({ x: 2, y: 0, discovered: true });
  const model = {
    mode: "glyph", tilePixels: 32, columns: 3, rows: 1, minX: 0, minY: 0,
    cells: [both, hostileOnly, none],
  };
  const plan = toDrawPlan(model);
  // 32px: r = round(32*0.14) = 4, inset = min(6, max(4, 16-4)) = 6.
  assert.deepEqual(plan.ops[0].markers, [
    { cx: 26, cy: 6, r: 4, fill: MAP_MARKER_COLORS.hostile },
    { cx: 26, cy: 26, r: 4, fill: MAP_MARKER_COLORS.item },
  ]);
  assert.deepEqual(plan.ops[1].markers, [
    { cx: 32 + 26, cy: 6, r: 4, fill: MAP_MARKER_COLORS.hostile },
  ]);
  assert.deepEqual(plan.ops[2].markers, [], "no flags, no markers");

  // The radius floor + collision clamp (inspect amendment): 10px tile ->
  // r = max(2, round(1.4)) = 2, inset = min(4, max(2, 5-2)) = 3.
  const tiny = toDrawPlan({
    mode: "glyph", tilePixels: 10, columns: 1, rows: 1, minX: 0, minY: 0,
    cells: [cell({ x: 0, y: 0, discovered: true, hasHostiles: true, hasItems: true })],
  });
  assert.deepEqual(tiny.ops[0].markers, [
    { cx: 7, cy: 3, r: 2, fill: MAP_MARKER_COLORS.hostile },
    { cx: 7, cy: 7, r: 2, fill: MAP_MARKER_COLORS.item },
  ]);
});

// M-T9 (REQ-004): markers change NOTHING else — a flagged plan differs from
// the unflagged plan only in the markers field.
test("marker flags leave tile, sprite, glyph, and text untouched (REQ-004)", () => {
  const base = {
    mode: "glyph", tilePixels: 32, columns: 1, rows: 1, minX: 0, minY: 0,
    cells: [cell({ x: 0, y: 0, discovered: true })],
  };
  const flagged = {
    ...base,
    cells: [cell({ x: 0, y: 0, discovered: true, hasHostiles: true, hasItems: true })],
  };
  const tileset = authorTileset();
  const plain = toDrawPlan(base, tileset);
  const marked = toDrawPlan(flagged, tileset);
  const { markers: _p, ...plainRest } = plain.ops[0];
  const { markers: _m, ...markedRest } = marked.ops[0];
  assert.deepEqual(markedRest, plainRest, "only markers differ");
  assert.equal(plain.ops[0].markers.length, 0);
  assert.equal(marked.ops[0].markers.length, 2);
});

// M-T10 (REQ-003/004): aria voices the color-only signal; zero counts stay
// byte-identical to the pre-marker label.
test("mapAriaLabel counts marker rooms only when nonzero (REQ-004)", () => {
  const plain = {
    region: "R", planes: [0], z: 0,
    cells: [{ present: true, discovered: true, current: true, title: "Sq", glyph: "+" }],
  };
  assert.equal(mapAriaLabel(plain), "Map: R — 1 room discovered — here: Sq",
    "no flags: byte-identical to pre-#33");

  const marked = {
    region: "R", planes: [0], z: 0,
    cells: [
      { present: true, discovered: true, current: true, title: "Sq", glyph: "+", hasHostiles: true, hasItems: true },
      { present: true, discovered: true, current: false, title: "Rd", glyph: ".", hasItems: true },
    ],
  };
  assert.equal(
    mapAriaLabel(marked),
    "Map: R — 2 rooms discovered, hostiles in 1 room, loot in 2 rooms — here: Sq",
  );
});
