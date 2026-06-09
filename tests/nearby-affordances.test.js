import assert from "node:assert/strict";
import test from "node:test";

import { toNearby, toEntityDetail } from "../src/client/snapshot.js";

// A `/state` snapshot whose `room.contents` carries the server-authored hostile
// affordances (camelCase, exactly as the engine's `room_snapshot` emits).
function nearbySnapshot(contents) {
  return { room: { contents } };
}

const HOSTILE_ATTACKABLE = {
  id: "stray",
  name: "Ashen Stray",
  kind: "actor",
  distance: 0,
  proximity: "exact",
  interactable: true,
  threat: { attackable: true, attackCommand: "attack Ashen Stray" },
  stats: { health: 9, maxHealth: 9, attack: 3 },
};
const HOSTILE_TOO_FAR = {
  id: "wolf",
  name: "Wolf",
  kind: "actor",
  distance: 2,
  proximity: "visible",
  interactable: false,
  threat: { attackable: false }, // attackCommand omitted (not attackable)
  stats: {}, // a combatant, but stats hidden → "unknown"
};
const HOSTILE_NONCOMBAT = {
  id: "brute",
  name: "Brute",
  kind: "actor",
  distance: 0,
  proximity: "exact",
  interactable: true,
  threat: { attackable: false },
  stats: {},
};
const NON_HOSTILE = {
  id: "mara",
  name: "Mara",
  kind: "actor",
  distance: 0,
  proximity: "exact",
  interactable: true,
  // no threat, no stats — a non-hostile non-combatant
};

function itemFor(entry) {
  return toNearby(nearbySnapshot([entry])).items[0];
}

// J1 (REQ-001/006): a hostile + attackable entry is flagged and gets an Attack
// action whose command is the SERVER's attackCommand (never client-built).
test("toNearby flags an attackable hostile and adds the server's Attack action", () => {
  const item = itemFor(HOSTILE_ATTACKABLE);
  assert.equal(item.hostile, true);
  assert.equal(item.attackable, true);
  assert.equal(item.combatStatus, "Attackable");
  const attack = item.actions.find((a) => a.label === "Attack");
  assert.ok(attack, "an Attack action is present");
  assert.equal(attack.command, HOSTILE_ATTACKABLE.threat.attackCommand);
  assert.equal(attack.command, "attack Ashen Stray", "uses the server command verbatim");
  assert.equal(attack.variant, "danger");
});

// J2 (REQ-002): a hostile that is not attackable (too far OR non-combat area) is
// flagged hostile but gets NO enabled Attack action, plus a quiet status label.
test("toNearby flags a non-attackable hostile without an Attack action", () => {
  for (const [entry, label] of [
    [HOSTILE_TOO_FAR, "Too far to attack"],
    [HOSTILE_NONCOMBAT, "Can't fight here"],
  ]) {
    const item = itemFor(entry);
    assert.equal(item.hostile, true);
    assert.equal(item.attackable, false);
    assert.equal(item.combatStatus, label);
    assert.equal(
      item.actions.find((a) => a.label === "Attack"),
      undefined,
      "no Attack action when not attackable",
    );
  }
});

// J3 (REQ-003): a non-hostile actor is not an enemy and gets no Attack action;
// Look/Talk are unchanged.
test("toNearby does not flag or arm a non-hostile actor", () => {
  const item = itemFor(NON_HOSTILE);
  assert.equal(item.hostile, false);
  assert.equal(item.attackable, false);
  assert.equal(item.combatStatus, null);
  assert.equal(item.actions.find((a) => a.label === "Attack"), undefined);
  assert.ok(item.actions.find((a) => a.label === "Look"), "Look preserved");
  assert.ok(item.actions.find((a) => a.label === "Talk"), "Talk preserved");
});

// J4 (REQ-005): toEntityDetail discloses numbers when the server discloses, renders
// hidden stats as unknown (null), and shows no stats section for a non-combatant.
test("toEntityDetail discloses, hides as unknown, or omits stats", () => {
  const disclosed = toEntityDetail(HOSTILE_ATTACKABLE);
  assert.equal(disclosed.hostile, true);
  assert.equal(disclosed.attackable, true);
  assert.deepEqual(disclosed.stats, {
    isCombatant: true,
    disclosed: true,
    health: 9,
    maxHealth: 9,
    attack: 3,
  });

  const hidden = toEntityDetail(HOSTILE_TOO_FAR);
  assert.equal(hidden.stats.isCombatant, true);
  assert.equal(hidden.stats.disclosed, false);
  assert.equal(hidden.stats.health, null);
  assert.equal(hidden.stats.maxHealth, null);
  assert.equal(hidden.stats.attack, null);

  const plain = toEntityDetail(NON_HOSTILE);
  assert.equal(plain.hostile, false);
  assert.equal(plain.stats.isCombatant, false);
});

// J5 (REQ-004): toEntityDetail is pure — it returns plain data and neither sends a
// command nor mutates the input entry (so opening the dialog has no side effect).
test("toEntityDetail is pure and mutates nothing", () => {
  const entry = structuredClone(HOSTILE_ATTACKABLE);
  const before = JSON.stringify(entry);
  const detail = toEntityDetail(entry);
  assert.equal(JSON.stringify(entry), before, "the input entry is not mutated");
  assert.equal(typeof detail, "object");
  assert.ok(
    !("command" in detail) && !("actions" in detail),
    "the detail model carries no command/action surface",
  );
});
