// View models derived from a `/state` GameSnapshot (camelCase per Decision 031).
//
// Pure: maps server state into the shapes the HUD, room brief, and the tabbed
// character menu (Nearby / Oaths / Gear / Pack) render. State stays separate
// from the DOM; the glue layer reads these models and updates elements.

const EQUIPMENT_SLOTS = [
  "Main hand",
  "Off hand",
  "Body",
  "Left earring",
  "Right earring",
  "Trinket",
];

function pct(value, max) {
  if (!max || max <= 0) {
    return 0;
  }
  return Math.max(0, Math.min(100, (value / max) * 100));
}

/** Top HUD: health/focus meters, current room kicker, tick. */
export function toHud(snapshot) {
  const player = snapshot?.player ?? {};
  const room = snapshot?.room ?? {};
  return {
    hp: player.hp ?? 0,
    maxHp: player.maxHp ?? 0,
    focus: player.focus ?? 0,
    maxFocus: player.maxFocus ?? 0,
    hpPct: pct(player.hp ?? 0, player.maxHp ?? 0),
    focusPct: pct(player.focus ?? 0, player.maxFocus ?? 0),
    roomName: room.title ?? snapshot?.worldTitle ?? "",
    roomKicker: room.subregion ?? room.region ?? "",
    tick: snapshot?.tick ?? 0,
  };
}

/** Oaths panel: the single beginner oath, or an available-state placeholder. */
export function toOaths(snapshot) {
  const oath = snapshot?.oath;
  if (!oath) {
    return {
      count: 0,
      complete: 0,
      items: [{ id: null, title: "No oath sworn yet.", status: "available", complete: false }],
    };
  }
  const complete = oath.status === "fulfilled";
  return {
    count: 1,
    complete: complete ? 1 : 0,
    items: [
      {
        id: oath.oathId ?? null,
        title: oath.title ?? "Oath",
        status: oath.status ?? "sworn",
        complete,
      },
    ],
  };
}

/**
 * Nearby panel: visible room contents (actors / items / fixtures). Exits are
 * navigation, not Nearby content, so they are intentionally absent (REQ-001).
 * The current snapshot exposes no room contents, so this is an honest empty
 * state today (REQ-002); when the server adds contents fields it becomes
 * data-driven with no UI change.
 */
export function toNearby(snapshot) {
  const room = snapshot?.room ?? {};
  const contents = Array.isArray(room.contents)
    ? room.contents
    : [
        ...(Array.isArray(room.actors) ? room.actors : []),
        ...(Array.isArray(room.items) ? room.items : []),
        ...(Array.isArray(room.fixtures) ? room.fixtures : []),
      ];
  const items = contents.map((entry) => ({
    name: entry.name ?? entry.id ?? "Something",
    kind: entry.kind ?? "thing",
    command: entry.command ?? (entry.name ? `look ${entry.name}` : "look"),
  }));
  return { count: items.length, items };
}

/** Gear panel: the six equipment slots, all empty in v1. */
export function toGear() {
  return {
    filled: 0,
    total: EQUIPMENT_SLOTS.length,
    slots: EQUIPMENT_SLOTS.map((label) => ({ label, value: "empty", filled: false })),
  };
}

/** Pack panel: inventory, empty in v1. */
export function toPack() {
  return { count: 0, items: [] };
}

/** Aggregate the four character-menu panels. */
export function toMenuModel(snapshot) {
  return {
    nearby: toNearby(snapshot),
    oaths: toOaths(snapshot),
    gear: toGear(),
    pack: toPack(),
  };
}
