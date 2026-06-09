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
  const items = contents.map(toNearbyItem);
  return { count: items.length, items };
}

function targetName(entry) {
  return entry.name ?? entry.id ?? "Something";
}

function canInteract(entry) {
  return entry.interactable ?? (!entry.proximity || entry.proximity === "exact" || entry.proximity === "interactable");
}

function distanceLabel(entry) {
  if (entry.proximity === "exact") {
    return "here";
  }
  if (typeof entry.distance === "number") {
    return `${entry.distance} away`;
  }
  return null;
}

function toNearbyItem(entry) {
  const name = targetName(entry);
  const kind = entry.kind ?? "thing";
  const interactable = canInteract(entry);
  const look = entry.command ?? `look ${name}`;
  const actions = [{ label: "Look", command: look, hint: `Look at ${name}` }];

  if (kind === "actor" || kind === "npc") {
    actions.push({
      label: "Talk",
      command: `talk ${name}`,
      disabled: !interactable,
      hint: interactable ? `Talk to ${name}` : `${name} is too far away to talk to`,
    });
  } else if (kind === "item") {
    actions.push({
      label: "Take",
      command: `take ${name}`,
      disabled: !interactable,
      hint: interactable ? `Take ${name}` : `${name} is too far away to take`,
    });
  }

  return {
    name,
    kind,
    distance: entry.distance ?? null,
    proximity: entry.proximity ?? null,
    interactable,
    detail: distanceLabel(entry),
    command: look,
    actions,
  };
}

/** Gear panel: the six equipment slots, all empty in v1. */
export function toGear() {
  return {
    filled: 0,
    total: EQUIPMENT_SLOTS.length,
    slots: EQUIPMENT_SLOTS.map((label) => ({ label, value: "empty", filled: false })),
  };
}

/**
 * Pack panel: the player's carried items (ticket #18/#20). Reads the additive
 * `snapshot.pack` (id, name, kind, and optional flags per item) — server data
 * only, never invented. An absent or empty `pack` is an honest empty state, so an
 * older snapshot without the key still renders.
 */
export function toPack(snapshot) {
  const pack = Array.isArray(snapshot?.pack) ? snapshot.pack : [];
  const items = pack.map((entry) => ({
    name: entry.name ?? entry.id ?? "Something",
    id: entry.id ?? null,
    kind: entry.kind ?? null,
    flags: Array.isArray(entry.flags) ? entry.flags : [],
  }));
  return { count: items.length, items };
}

/** Aggregate the four character-menu panels. */
export function toMenuModel(snapshot) {
  return {
    nearby: toNearby(snapshot),
    oaths: toOaths(snapshot),
    gear: toGear(),
    pack: toPack(snapshot),
  };
}

/**
 * Battle modal view model (ticket #22): the active combat encounter as the modal
 * renders it, or an inactive shell when `snapshot.combat` is absent. Pure — the
 * glue opens the modal when `active` flips true (REQ-008) and closes it when it
 * flips false (REQ-010). Participants are split by `side` so the right pane can
 * group allies vs enemies (multi-party-ready, REQ-009); each carries an `hpPct`
 * for its meter. Reads only server-authored `snapshot.combat`, never invented.
 */
export function toBattle(snapshot) {
  const combat = snapshot?.combat;
  if (!combat) {
    return { active: false, round: 0, log: [], participants: [], allies: [], enemies: [] };
  }
  const participants = (Array.isArray(combat.participants) ? combat.participants : []).map(
    toCombatant,
  );
  return {
    active: true,
    round: combat.round ?? 0,
    log: Array.isArray(combat.log) ? combat.log : [],
    participants,
    allies: participants.filter((entry) => entry.side !== "enemy"),
    enemies: participants.filter((entry) => entry.side === "enemy"),
  };
}

function toCombatant(entry) {
  const hp = entry.hp ?? 0;
  const maxHp = entry.maxHp ?? 0;
  return {
    id: entry.id ?? null,
    name: entry.name ?? entry.id ?? "Combatant",
    hp,
    maxHp,
    side: entry.side ?? "enemy",
    hpPct: pct(hp, maxHp),
    defeated: hp <= 0,
  };
}
