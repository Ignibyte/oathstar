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
    // Ticket #30: the progression pair the header line renders ("Lv N · M xp").
    level: player.level ?? 0,
    xp: player.xp ?? 0,
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

  // Ticket #23: hostile-combat affordances are server-authored — `threat` is
  // present iff the server marks this thing hostile, and `attackable` is the
  // server's verdict. The client never infers any of this from name/kind/CSS.
  const threat = entry.threat ?? null;
  const hostile = Boolean(threat);
  const attackable = Boolean(threat?.attackable);
  // Add Attack only when the server says it is attackable, using the server's
  // canonical command (REQ-001/006); a hostile that is not attackable gets a
  // quiet status instead of an enabled action (REQ-002).
  if (attackable && threat.attackCommand) {
    actions.push({
      label: "Attack",
      command: threat.attackCommand,
      hint: `Attack ${name}`,
      variant: "danger",
    });
  }

  return {
    name,
    kind,
    distance: entry.distance ?? null,
    proximity: entry.proximity ?? null,
    interactable,
    hostile,
    attackable,
    combatStatus: combatStatusLabel(hostile, attackable, interactable),
    entityDetail: toEntityDetail(entry),
    detail: distanceLabel(entry),
    command: look,
    actions,
  };
}

/**
 * The quiet hostile status shown on a Nearby card / detail view (ticket #23).
 * `null` for non-hostiles (so the UI shows no enemy state, REQ-003); otherwise a
 * short label flagging whether the hostile can be attacked from here (REQ-002).
 */
function combatStatusLabel(hostile, attackable, interactable) {
  if (!hostile) {
    return null;
  }
  if (attackable) {
    return "Attackable";
  }
  return interactable ? "Can't fight here" : "Too far to attack";
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
/**
 * The status line shown for a queued between-pulse action (ticket #24), keyed
 * by the server's `queuedAction` value. Label decisions live here in the
 * view-model (the `combatStatusLabel` pattern), not in the DOM glue.
 */
const QUEUED_ACTION_LABELS = {
  flee: "Looking for an opening to flee…",
  guard: "Guarding against the next blow…",
  power_strike: "Winding up a power strike…",
};

export function toBattle(snapshot) {
  const combat = snapshot?.combat;
  if (!combat) {
    return {
      active: false,
      round: 0,
      log: [],
      participants: [],
      allies: [],
      enemies: [],
      queuedAction: null,
      queuedActionLabel: null,
    };
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
    // The server-queued between-pulse action (ticket #24), e.g. "flee"; null
    // when nothing is queued. Read-only server data, never invented here.
    queuedAction: combat.queuedAction ?? null,
    queuedActionLabel: QUEUED_ACTION_LABELS[combat.queuedAction] ?? null,
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

/**
 * Entity detail view model (ticket #23): the focused inspection of one Nearby
 * entry, for the generic entity detail dialog. Pure — it reads only the
 * server-authored snapshot entry the client already holds, so opening the detail
 * view sends no command and mutates no game state (REQ-004). Disclosed combat
 * stats are numbers; hidden stats are `null` so the view renders them as
 * "unknown" — never an invented value (REQ-005).
 */
export function toEntityDetail(entry) {
  const safe = entry ?? {};
  const threat = safe.threat ?? null;
  const hostile = Boolean(threat);
  const attackable = Boolean(threat?.attackable);
  const stats = safe.stats ?? null;
  return {
    name: targetName(safe),
    kind: safe.kind ?? "thing",
    hostile,
    attackable,
    statusLabel: combatStatusLabel(hostile, attackable, canInteract(safe)),
    stats: stats
      ? {
          isCombatant: true,
          // All-or-nothing disclosure in v1: a disclosed profile carries its
          // numbers; a hidden one carries none (rendered as "unknown").
          disclosed: stats.health != null,
          health: stats.health ?? null,
          maxHealth: stats.maxHealth ?? null,
          attack: stats.attack ?? null,
        }
      : { isCombatant: false, disclosed: false, health: null, maxHealth: null, attack: null },
  };
}
