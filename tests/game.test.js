import assert from "node:assert/strict";
import test from "node:test";
import { createGame } from "../src/engine.js";

test("starts at Starfall Gate and can take the lantern", () => {
  const game = createGame({ rng: () => 0.5 });
  game.intro();

  const outcome = game.submit("take lantern");
  assert.equal(outcome.view.room.id, "starfallGate");
  assert.deepEqual(
    outcome.view.inventory.map((item) => item.id),
    ["blackLantern"]
  );
  assert.match(outcome.messages.at(-1).text, /Taken/);
});

test("black lantern reveals the brass prism in the well", () => {
  const game = createGame({ rng: () => 0.5 });
  game.intro();

  game.submit("take lantern");
  game.submit("east");
  game.submit("north");
  const reveal = game.submit("use lantern");

  assert.match(reveal.messages[0].text, /brass prism/i);
  const take = game.submit("take prism");
  assert.deepEqual(
    take.view.inventory.map((item) => item.id),
    ["blackLantern", "brassPrism"]
  );
});

test("shade blocks the observatory until defeated", () => {
  const game = createGame({ rng: () => 0.99 });
  game.intro();

  game.submit("north");
  game.submit("north");
  const blocked = game.submit("north");
  assert.equal(blocked.view.room.id, "ironSanctum");
  assert.match(blocked.messages[0].text, /will not let you pass/i);

  game.submit("attack shade");
  game.submit("attack shade");
  const finalHit = game.submit("attack shade");
  assert.match(finalHit.messages.at(-1).text, /ember shard/i);

  const moved = game.submit("north");
  assert.equal(moved.view.room.id, "observatory");
});

test("can complete the first playable arc", () => {
  const game = createGame({ rng: () => 0.99 });
  game.intro();

  game.submit("take lantern");
  game.submit("north");
  game.submit("talk warden");
  game.submit("south");
  game.submit("east");
  game.submit("north");
  game.submit("use lantern");
  game.submit("take prism");
  game.submit("south");
  game.submit("west");
  game.submit("west");
  game.submit("ring bell");
  game.submit("take mercy");
  game.submit("east");
  game.submit("north");
  game.submit("north");
  game.submit("attack shade");
  game.submit("attack shade");
  game.submit("attack shade");
  game.submit("take ember");
  game.submit("north");
  const win = game.submit("use oathstar");

  assert.equal(win.view.won, true);
  assert.match(win.messages[0].text, /memory, mercy, and flame/i);
  assert.equal(
    win.view.quests.every((quest) => quest.complete),
    true
  );
});
