import assert from "node:assert/strict";
import test from "node:test";

import { toBattle } from "../src/client/snapshot.js";
import { toComponent } from "../src/client/components.js";
import { parseEvent } from "../src/client/wire.js";

// A `/state` snapshot carrying an active combat encounter (camelCase, as the
// server emits — CombatantSnapshot is #[serde(rename_all = "camelCase")]).
function combatSnapshot(combatOverrides = {}) {
  return {
    combat: {
      round: 2,
      participants: [
        { id: "player", name: "Oathbearer", hp: 14, maxHp: 20, side: "player" },
        { id: "stray", name: "Ashen Stray", hp: 1, maxHp: 9, side: "enemy" },
      ],
      log: [
        "You strike Ashen Stray for 4 (1/9).",
        "Ashen Stray hits you for 3 (14/20).",
      ],
      ...combatOverrides,
    },
  };
}

// J1 (REQ-008/010): toBattle is inactive with no combat (so the modal stays
// closed / closes), and active when a combat snapshot is present (so it opens).
test("toBattle is inactive without a combat snapshot", () => {
  const battle = toBattle({});
  assert.equal(battle.active, false);
  assert.deepEqual(battle.participants, []);
  assert.deepEqual(battle.log, []);
  assert.equal(battle.round, 0);
  // Absent / null snapshots are handled defensively.
  assert.equal(toBattle(null).active, false);
  assert.equal(toBattle(undefined).active, false);
  assert.equal(toBattle({ combat: null }).active, false);
});

test("toBattle is active with a combat snapshot", () => {
  const battle = toBattle(combatSnapshot());
  assert.equal(battle.active, true);
  assert.equal(battle.round, 2);
  assert.equal(battle.log.length, 2);
  assert.equal(battle.log[0], "You strike Ashen Stray for 4 (1/9).");
});

// J2 (REQ-009): participants split by side into allies vs enemies; each carries
// name/hp/maxHp/hpPct (and `defeated`) so the right pane can render both sides.
test("toBattle splits participants by side with hp meters", () => {
  const battle = toBattle(combatSnapshot());
  assert.equal(battle.participants.length, 2);
  assert.equal(battle.allies.length, 1);
  assert.equal(battle.enemies.length, 1);

  const [player] = battle.allies;
  assert.equal(player.name, "Oathbearer");
  assert.equal(player.side, "player");
  assert.equal(player.hp, 14);
  assert.equal(player.maxHp, 20);
  assert.equal(player.hpPct, 70); // 14/20
  assert.equal(player.defeated, false);

  const [enemy] = battle.enemies;
  assert.equal(enemy.name, "Ashen Stray");
  assert.equal(enemy.side, "enemy");
  assert.ok(enemy.hpPct > 11 && enemy.hpPct < 12, "1/9 ≈ 11.1%");
  assert.equal(enemy.defeated, false);
});

// J2 edge: zero maxHp yields hpPct 0 (no NaN/Infinity); hp <= 0 is `defeated`.
test("toBattle handles zero maxHp and marks defeated combatants", () => {
  const battle = toBattle(
    combatSnapshot({
      participants: [
        { id: "ghost", name: "Ghost", hp: 0, maxHp: 0, side: "enemy" },
        { id: "fallen", name: "Fallen", hp: 0, maxHp: 10, side: "enemy" },
      ],
    }),
  );
  const [ghost, fallen] = battle.enemies;
  assert.equal(ghost.hpPct, 0, "0/0 is 0%, not NaN");
  assert.equal(ghost.defeated, true);
  assert.equal(fallen.defeated, true);
});

// J3 (REQ-006): combat events render through the existing component catalog as
// the `danger` variant — both the play-by-play LogMessage and the typed marker.
test("toComponent renders combat events as the danger variant", () => {
  const strike = toComponent({
    type: "log_message",
    channel: "combat",
    component: "combat_message",
    text: "You strike Ashen Stray for 4 (1/9).",
    id: "3",
  });
  assert.equal(strike.variant, "danger");
  assert.equal(strike.label, "Combat");
  assert.equal(strike.text, "You strike Ashen Stray for 4 (1/9).");

  const started = toComponent({
    type: "combat_started",
    channel: "combat",
    text: "Ashen Stray turns on you.",
    id: "11",
  });
  assert.equal(started.variant, "danger");
  assert.equal(started.text, "Ashen Stray turns on you.");
});

// J4 (REQ-010): when combat ends the modal closes (toBattle inactive) and the
// compact summary survives in the feed (the combat_ended event renders its text).
test("combat end leaves an inactive battle and a feed summary", () => {
  assert.equal(toBattle({ combat: null }).active, false);

  const ended = toComponent({
    type: "combat_ended",
    channel: "combat",
    text: "You have defeated Ashen Stray. Victory!",
    id: "12",
  });
  assert.equal(ended.variant, "danger");
  assert.equal(ended.label, "Combat");
  assert.equal(ended.text, "You have defeated Ashen Stray. Victory!");
});

// ---- ticket #24: the real-time pulse loop, client side ----

// J1 (REQ-004): the queued between-pulse action and its label are view-model
// decisions — null-safe when absent, mapped when queued, and never invented
// for an unknown action.
test("toBattle exposes the queued action and its label", () => {
  assert.equal(toBattle({}).queuedAction, null);
  assert.equal(toBattle({}).queuedActionLabel, null);

  const idle = toBattle(combatSnapshot());
  assert.equal(idle.queuedAction, null, "no queuedAction key → null");
  assert.equal(idle.queuedActionLabel, null);

  const fleeing = toBattle(combatSnapshot({ queuedAction: "flee" }));
  assert.equal(fleeing.queuedAction, "flee");
  assert.equal(fleeing.queuedActionLabel, "Looking for an opening to flee…");

  const unknown = toBattle(combatSnapshot({ queuedAction: "ward" }));
  assert.equal(unknown.queuedAction, "ward", "the raw action passes through");
  assert.equal(unknown.queuedActionLabel, null, "no label is invented for unknown actions");
});

// J3 (REQ-003): the wire adapter passes the pulse marker through with its type
// intact (the default case), so the client's /state-refresh predicate matches
// it and the battle modal updates live without a command.
test("parseEvent passes combat_pulse through with type intact", () => {
  const parsed = parseEvent({
    type: "combat_pulse",
    eventId: 12,
    tick: 4,
    channel: "combat",
    round: 2,
  });
  assert.equal(parsed.type, "combat_pulse");
  assert.equal(parsed.channel, "combat");
  assert.equal(parsed.id, 12);
});

// VJ1/VJ2 (ticket #25, REQ-006): the direct battle verbs map to their exact
// queued-status labels; raw values pass through untouched.
test("toBattle labels the direct battle verbs", () => {
  const guarding = toBattle(combatSnapshot({ queuedAction: "guard" }));
  assert.equal(guarding.queuedAction, "guard");
  assert.equal(guarding.queuedActionLabel, "Guarding against the next blow…");

  const winding = toBattle(combatSnapshot({ queuedAction: "power_strike" }));
  assert.equal(winding.queuedAction, "power_strike");
  assert.equal(winding.queuedActionLabel, "Winding up a power strike…");
});
