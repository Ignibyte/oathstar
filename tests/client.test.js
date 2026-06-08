import assert from "node:assert/strict";
import test from "node:test";

import { parseEvent } from "../src/client/wire.js";
import { toComponent } from "../src/client/components.js";
import { toHud, toMenuModel, toOaths, toNearby, toPack } from "../src/client/snapshot.js";
import { toRoomDisplay, toExitPad, DEFAULT_ROOM_DISPLAY_LIMIT } from "../src/client/room.js";
import { toMapModel, DEFAULT_MAP_CONFIG } from "../src/client/map.js";
import { suggestCommands, COMMAND_VOCAB } from "../src/client/intent.js";

// A representative `/state` snapshot (camelCase per Decision 031).
function sampleSnapshot(overrides = {}) {
  return {
    worldId: "beginner",
    worldTitle: "Oathstar Beginner",
    tick: 3,
    currentRoomId: "hollowmere_square",
    player: { id: "player", name: "You", level: 1, xp: 0, hp: 12, maxHp: 18, focus: 4, maxFocus: 5 },
    room: {
      id: "hollowmere_square",
      title: "Hollowmere Square",
      region: "hollowmere",
      subregion: "center",
      description: "A quiet square.",
      exits: { north: "north_gate", up: "bell_tower" },
      x: 0,
      y: 0,
      z: 0,
      glyph: "+",
      passable: true,
    },
    map: {
      region: "hollowmere",
      subregion: "center",
      currentRoomId: "hollowmere_square",
      rooms: [
        { id: "hollowmere_square", title: "Hollowmere Square", x: 0, y: 0, z: 0, glyph: "+", passable: true, discovered: true, current: true, exits: { north: "north_gate" } },
        { id: "north_gate", title: "North Gate", x: 0, y: -1, z: 0, glyph: "#", passable: true, discovered: false, current: false, exits: { south: "hollowmere_square" } },
      ],
    },
    ...overrides,
  };
}

// T5 (REQ-003): wire-format normalization honors the snake/camel split.
test("wire.parseEvent normalizes snake_case event payloads (Decision 031)", () => {
  const room = parseEvent({ eventId: 7, tick: 2, channel: "room", type: "room_entered", room_id: "hollowmere_square", title: "Hollowmere Square" });
  assert.equal(room.id, 7);
  assert.equal(room.type, "room_entered");
  assert.equal(room.roomId, "hollowmere_square");
  assert.equal(room.title, "Hollowmere Square");

  const sworn = parseEvent({ eventId: 8, channel: "oath", type: "oath_sworn", oath_id: "still_the_bell", title: "Still the Bell" });
  assert.equal(sworn.oathId, "still_the_bell");

  const log = parseEvent({ eventId: 9, channel: "narrative", type: "log_message", component: "narrative_message", text: "Wind moves." });
  assert.equal(log.component, "narrative_message");
  assert.equal(log.text, "Wind moves.");

  const tick = parseEvent({ eventId: 10, tick: 5, channel: "debug", type: "tick", value: 5 });
  assert.equal(tick.value, 5);

  assert.equal(parseEvent(null), null);
  assert.equal(parseEvent({ noType: true }), null);
});

// T6 (REQ-003): typed events become componentized descriptors; ticks are skipped.
test("components.toComponent maps events to render descriptors; tick skipped", () => {
  const room = toComponent(parseEvent({ eventId: 1, channel: "room", type: "room_entered", room_id: "r", title: "Hollowmere Square" }));
  assert.equal(room.variant, "room");
  assert.equal(room.label, "Room");
  assert.match(room.text, /Hollowmere Square/);
  assert.equal(room.dataset.type, "room_entered");

  // OutputComponent overrides the channel mapping.
  const header = toComponent(parseEvent({ eventId: 2, channel: "narrative", type: "log_message", component: "room_header", text: "Hollowmere Square" }));
  assert.equal(header.variant, "room");
  assert.equal(header.label, "Room");

  const oath = toComponent(parseEvent({ eventId: 3, channel: "oath", type: "oath_sworn", oath_id: "o", title: "Bell" }));
  assert.equal(oath.variant, "success");

  assert.equal(toComponent(parseEvent({ eventId: 4, channel: "debug", type: "tick", value: 1 })), null);
  assert.equal(toComponent(null), null);
});

// T7 (REQ-002): snapshot view models drive HUD + the tabbed menu.
test("snapshot view models feed HUD and the tabbed menu", () => {
  const snap = sampleSnapshot();

  const hud = toHud(snap);
  assert.equal(hud.hp, 12);
  assert.equal(hud.maxHp, 18);
  assert.ok(Math.abs(hud.hpPct - (12 / 18) * 100) < 1e-9);
  assert.equal(hud.roomName, "Hollowmere Square");
  assert.equal(hud.roomKicker, "center");

  const menu = toMenuModel(snap);
  assert.equal(menu.gear.total, 6);
  assert.equal(menu.pack.count, 0);
  assert.equal(menu.nearby.count, 0); // Nearby is contents-only; the sample has none

  assert.equal(toOaths(snap).items[0].status, "available");
  const sworn = toOaths(sampleSnapshot({ oath: { oathId: "o", title: "Bell", status: "sworn" } }));
  assert.equal(sworn.items[0].complete, false);
  const fulfilled = toOaths(sampleSnapshot({ oath: { oathId: "o", title: "Bell", status: "fulfilled" } }));
  assert.equal(fulfilled.complete, 1);
  assert.equal(fulfilled.items[0].complete, true);
});

// TR1 (REQ-005, REQ-008): the room display model separates main / full and
// summarizes long descriptions at a polished breakpoint; future fields are read.
test("room.toRoomDisplay separates main/full and summarizes long text", () => {
  const short = toRoomDisplay(sampleSnapshot());
  assert.equal(short.title, "Hollowmere Square");
  assert.equal(short.truncated, false);
  assert.equal(short.main, short.full);
  assert.equal(short.mediaHint, null);
  assert.deepEqual(short.exits.map((exit) => exit.direction), ["north", "up"]);
  assert.equal(short.exits[0].command, "north");

  // long text WITH a sentence boundary before the limit → clean first sentence.
  const sentences =
    "A vast warrior's grave opens before you. " +
    "Banners rot along the walls and a cold wind carries the names of the dead.";
  const bySentence = toRoomDisplay(
    sampleSnapshot({ room: { ...sampleSnapshot().room, description: sentences } }),
    { limit: 60 },
  );
  assert.equal(bySentence.truncated, true);
  assert.equal(bySentence.full, sentences);
  assert.equal(bySentence.main, "A vast warrior's grave opens before you.");

  // long text with NO sentence boundary before the limit → word-cut + ellipsis.
  const clause = "a long winding corridor stretches onward past many silent shuttered alcoves";
  const byWord = toRoomDisplay(
    sampleSnapshot({ room: { ...sampleSnapshot().room, description: clause } }),
    { limit: 40 },
  );
  assert.equal(byWord.truncated, true);
  assert.ok(byWord.main.length <= 41);
  assert.ok(byWord.main.endsWith("…"));
  assert.ok(!byWord.main.includes(" …"));

  // a single token longer than the limit → hard cut + ellipsis.
  const byChar = toRoomDisplay(
    sampleSnapshot({ room: { ...sampleSnapshot().room, description: "x".repeat(80) } }),
    { limit: 20 },
  );
  assert.equal(byChar.truncated, true);
  assert.ok(byChar.main.endsWith("…"));

  // future fields: shortDescription drives main; fullDescription keeps the long text.
  const future = toRoomDisplay(
    sampleSnapshot({
      room: {
        ...sampleSnapshot().room,
        shortDescription: "Brief.",
        fullDescription: "Brief. And then much, much more detail follows here.",
        mediaHint: { kind: "image", alt: "A grave" },
      },
    }),
    { limit: 999 },
  );
  assert.equal(future.main, "Brief.");
  assert.equal(future.full, "Brief. And then much, much more detail follows here.");
  assert.equal(future.truncated, true);
  assert.deepEqual(future.mediaHint, { kind: "image", alt: "A grave" });

  assert.ok(DEFAULT_ROOM_DISPLAY_LIMIT > 0);
});

// TR2 (REQ-003, REQ-004): the Exit Pad has the six canonical directions, each
// available iff the room has that exit, sending the canonical command.
test("room.toExitPad exposes six canonical directions with availability", () => {
  const pad = toExitPad(sampleSnapshot()); // exits: north, up
  assert.deepEqual(
    pad.directions.map((entry) => entry.dir),
    ["up", "north", "west", "east", "south", "down"],
  );
  const byDir = Object.fromEntries(pad.directions.map((entry) => [entry.dir, entry]));
  assert.equal(byDir.north.available, true);
  assert.equal(byDir.up.available, true);
  assert.equal(byDir.south.available, false);
  assert.equal(byDir.west.available, false);
  assert.equal(byDir.north.command, "north");
  assert.equal(pad.availableCount, 2);

  const none = toExitPad({ room: { exits: {} } });
  assert.equal(none.availableCount, 0);
  assert.ok(none.directions.every((entry) => !entry.available));
});

// TN (REQ-001, REQ-002): Nearby lists room contents only — never exits — and is
// an honest empty state when the snapshot has no contents.
test("snapshot.toNearby shows contents only, never exits", () => {
  const empty = toNearby(sampleSnapshot()); // room has exits but no contents
  assert.equal(empty.count, 0);
  assert.deepEqual(empty.items, []);

  const withContents = toNearby({
    room: {
      exits: { north: "x" },
      contents: [
        { name: "Mara", kind: "npc" },
        { name: "Ledger", kind: "fixture" },
      ],
    },
  });
  assert.equal(withContents.count, 2);
  assert.equal(withContents.items[0].name, "Mara");
  assert.equal(withContents.items[0].kind, "npc");
  assert.ok(withContents.items[0].command.includes("Mara"));
  assert.deepEqual(
    withContents.items[0].actions.map((action) => action.label),
    ["Look", "Talk"],
  );
  assert.ok(!withContents.items.some((item) => item.kind === "exit"));

  // forward-compat: assembled from actors/items/fixtures when there is no
  // `contents` array.
  const assembled = toNearby({
    room: {
      exits: { north: "x" },
      actors: [{ name: "Warden", kind: "npc" }],
      items: [{ name: "Candle", kind: "item" }],
      fixtures: [{ name: "Ledger", kind: "fixture" }],
    },
  });
  assert.equal(assembled.count, 3);
  assert.deepEqual(
    assembled.items.map((item) => item.kind),
    ["npc", "item", "fixture"],
  );
});

// TN2 (REQ-008): toNearby renders the server's NearbySnapshot wire shape
// (id/name/kind/distance/proximity/interactable — ticket #17/#18). Awareness
// fields drive labels and action availability, so the panel lights up
// data-driven from the real `/state` payload with no client-specific guessing.
test("snapshot.toNearby renders the server NearbySnapshot shape", () => {
  const near = toNearby({
    room: {
      exits: { north: "x" },
      contents: [
        {
          id: "mara",
          name: "Mara Candlekeep",
          kind: "actor",
          distance: 1,
          proximity: "interactable",
          interactable: true,
        },
        { id: "coin", name: "Coin", kind: "item", distance: 2, proximity: "visible", interactable: false },
      ],
    },
  });
  assert.equal(near.count, 2);
  assert.equal(near.items[0].name, "Mara Candlekeep");
  assert.equal(near.items[0].kind, "actor");
  assert.equal(near.items[0].detail, "1 away");
  assert.equal(near.items[0].command, "look Mara Candlekeep");
  assert.equal(near.items[0].actions[1].command, "talk Mara Candlekeep");
  assert.equal(near.items[1].kind, "item");
  assert.equal(near.items[1].actions[1].label, "Take");
  assert.equal(near.items[1].actions[1].disabled, true);
});

// T8 (REQ-007): the map model carries a configurable tile size + render mode and
// never mutates the server's map snapshot shape.
test("map.toMapModel builds a grid, flags current, and passes config through", () => {
  const snap = sampleSnapshot();
  const model = toMapModel(snap.map, DEFAULT_MAP_CONFIG);
  assert.equal(model.mode, "glyph");
  assert.equal(model.tilePixels, 32);
  assert.equal(model.columns, 1);
  assert.equal(model.rows, 2);
  const current = model.cells.find((cell) => cell.current);
  assert.equal(current.id, "hollowmere_square");
  assert.equal(current.discovered, true);
  assert.equal(current.passable, true);

  const ascii = toMapModel(snap.map, { mode: "ascii", tilePixels: 16 });
  assert.equal(ascii.mode, "ascii");
  assert.equal(ascii.tilePixels, 16);

  const fallback = toMapModel(snap.map, { mode: "nope", tilePixels: -5 });
  assert.equal(fallback.mode, "glyph");
  assert.equal(fallback.tilePixels, 32);

  // server map shape is untouched
  assert.equal(snap.map.rooms.length, 2);
  assert.ok(!("present" in snap.map.rooms[0]));

  const legacy = toMapModel({
    region: "legacy",
    currentRoomId: "old_room",
    rooms: [
      { id: "old_room", title: "Old Room", x: 0, y: 0, z: 0, glyph: ".", discovered: true, current: true, exits: {} },
    ],
  });
  assert.equal(
    legacy.cells[0].passable,
    undefined,
    "missing passable stays unknown rather than becoming blocked",
  );

  const empty = toMapModel({ region: "x", currentRoomId: null, rooms: [] });
  assert.equal(empty.columns, 0);
  assert.deepEqual(empty.cells, []);
});

// T8b (REQ-007 regression): the beginner tower stacks rooms at the same (x,y) on
// different z. The minimap must render the CURRENT room's floor only — never
// collapsing floors into one cell or dropping the "here" marker.
test("map.toMapModel renders only the current room's z-plane (stacked rooms)", () => {
  const stacked = {
    region: "old_bell_tower",
    currentRoomId: "bell_eater_roost",
    rooms: [
      { id: "hollowmere_square", title: "Hollowmere Square", x: 0, y: 0, z: 0, glyph: "+", passable: true, discovered: true, current: false, exits: {} },
      { id: "bell_frame", title: "Silent Bell Frame", x: 0, y: 0, z: 1, glyph: "^", passable: true, discovered: true, current: false, exits: {} },
      { id: "tower_foot", title: "Old Bell Tower Foot", x: 0, y: -3, z: 0, glyph: "|", passable: true, discovered: true, current: false, exits: {} },
      { id: "tower_landing", title: "Tower Landing", x: 0, y: -3, z: 1, glyph: "|", passable: true, discovered: true, current: false, exits: {} },
      { id: "bell_eater_roost", title: "Bell-Eater Roost", x: 0, y: -3, z: 2, glyph: "@", passable: true, discovered: true, current: true, exits: {} },
    ],
  };

  const top = toMapModel(stacked, DEFAULT_MAP_CONFIG);
  assert.equal(top.z, 2);
  assert.deepEqual(top.planes, [0, 1, 2]);
  // Only the boss floor (z=2) renders, and it IS the current cell — never dropped.
  assert.deepEqual(top.cells.filter((cell) => cell.present).map((cell) => cell.id), ["bell_eater_roost"]);
  assert.ok(top.cells.some((cell) => cell.current), "the current marker is never dropped");

  // Standing on the ground floor shows z=0 rooms, not the stacked z=1 bell_frame.
  const ground = toMapModel(
    {
      ...stacked,
      currentRoomId: "hollowmere_square",
      rooms: stacked.rooms.map((room) => ({ ...room, current: room.id === "hollowmere_square" })),
    },
    DEFAULT_MAP_CONFIG,
  );
  assert.equal(ground.z, 0);
  const groundIds = ground.cells.filter((cell) => cell.present).map((cell) => cell.id).sort();
  assert.deepEqual(groundIds, ["hollowmere_square", "tower_foot"]);
  assert.ok(!groundIds.includes("bell_frame"));
});

// T9 (REQ-003/REQ-004): the Intent helper EXCLUDES movement commands (those live
// on the Exit Pad now); it still offers contextual + non-movement vocabulary.
// Typed movement is unaffected — it goes through the command input, not Intent.
const MOVEMENT_COMMANDS = ["north", "south", "east", "west", "up", "down"];

test("intent.suggestCommands excludes movement, keeps contextual + vocab", () => {
  const snap = sampleSnapshot(); // room exits {north, up}
  const all = suggestCommands(snap, "");

  assert.ok(
    !all.some((command) => MOVEMENT_COMMANDS.includes(command.command)),
    "no movement command appears in Intent",
  );
  assert.ok(all.some((command) => command.command === "swear"));
  assert.ok(all.some((command) => command.command === "look"));
  assert.ok(all.some((command) => command.command === "talk mara"));
  assert.ok(all.some((command) => command.command === "take wax stub"));

  const sworn = suggestCommands(
    sampleSnapshot({ oath: { oathId: "o", title: "Bell", status: "sworn" } }),
    "",
  );
  assert.ok(sworn.some((command) => command.command === "confront"));
  assert.ok(!sworn.some((command) => MOVEMENT_COMMANDS.includes(command.command)));

  const filtered = suggestCommands(snap, "map");
  assert.ok(filtered.length >= 1);
  assert.ok(filtered.every((command) => command.command.includes("map") || command.label.includes("map")));

  // searching for a direction surfaces nothing — movement is not in the vocabulary
  assert.equal(suggestCommands(snap, "north").length, 0);

  assert.ok(COMMAND_VOCAB.some((command) => command.command === "swear"));
  assert.ok(COMMAND_VOCAB.some((command) => command.command === "talk mara"));
  assert.ok(COMMAND_VOCAB.some((command) => command.command === "take wax stub"));
  assert.ok(!COMMAND_VOCAB.some((command) => MOVEMENT_COMMANDS.includes(command.command)));
});

// ---- ticket #18: Pack panel reads the additive snapshot.pack ----

// TP1 (REQ-007): toPack reads carried items (id + name) from snapshot.pack, in order.
test("snapshot.toPack reads carried items from snapshot.pack", () => {
  const pack = toPack({
    pack: [
      { id: "wax_stub", name: "Wax Stub" },
      { id: "candle", name: "Black Candle" },
    ],
  });
  assert.equal(pack.count, 2);
  assert.equal(pack.items[0].name, "Wax Stub");
  assert.equal(pack.items[1].name, "Black Candle");
});

// TP2 (REQ-007/008): an absent or empty pack is an honest empty state, so an old
// snapshot without the key still renders.
test("snapshot.toPack is an honest empty state when pack is absent or empty", () => {
  assert.equal(toPack(undefined).count, 0);
  assert.equal(toPack({}).count, 0);
  assert.equal(toPack({ pack: [] }).count, 0);
  assert.deepEqual(toPack({}).items, []);
});

// TP3: the display name falls back name → id → "Something".
test("snapshot.toPack falls back name to id to placeholder", () => {
  assert.equal(toPack({ pack: [{ id: "wax_stub" }] }).items[0].name, "wax_stub");
  assert.equal(toPack({ pack: [{}] }).items[0].name, "Something");
});

// TP4 (REQ-002/007): toPack passes through server kind + flags and invents nothing.
test("snapshot.toPack passes through server kind and flags, inventing nothing", () => {
  const out = toPack({
    pack: [{ id: "lamp", name: "Brass Lamp", kind: "light", flags: ["lit"] }],
  });
  assert.equal(out.items[0].kind, "light");
  assert.deepEqual(out.items[0].flags, ["lit"]);
  // Missing kind/flags are reported as null/[] — never invented.
  const bare = toPack({ pack: [{ id: "x", name: "X" }] });
  assert.equal(bare.items[0].kind, null);
  assert.deepEqual(bare.items[0].flags, []);
  // An empty pack invents no items.
  assert.deepEqual(toPack({ pack: [] }).items, []);
});

// TP4 (REQ-007): the aggregate menu model carries the pack panel from the snapshot.
test("snapshot.toMenuModel includes the pack panel from the snapshot", () => {
  const menu = toMenuModel({ pack: [{ id: "wax_stub", name: "Wax Stub" }] });
  assert.equal(menu.pack.count, 1);
  assert.equal(menu.pack.items[0].name, "Wax Stub");
});
