import { directionAliases, directionLabels, questDefinitions, world } from "./world.js";

const SAVE_VERSION = 1;

const articles = new Set(["a", "an", "the"]);

export function createGame(options = {}) {
  const rng = options.rng ?? Math.random;
  const state = options.state ? hydrateState(options.state) : createInitialState();

  function intro() {
    const room = getCurrentRoom(state);
    state.visited[room.id] = true;
    return [
      {
        type: "system",
        text: "The prompt wakes beneath your hand."
      },
      { type: "room", text: room.firstVisit ?? room.description },
      ...describeRoom(state)
    ];
  }

  function submit(rawInput) {
    const input = normalizeInput(rawInput);
    if (!input) {
      return result([{ type: "system", text: "The ruin waits for a command." }]);
    }

    state.turn += 1;

    const parsed = parseCommand(input);
    const messages = runCommand(parsed, input);
    return result(messages);
  }

  function runCommand(parsed, input) {
    if (parsed.kind === "direction") {
      return move(parsed.direction);
    }

    switch (parsed.verb) {
      case "help":
        return help();
      case "look":
      case "l":
        return describeRoom(state);
      case "examine":
      case "x":
      case "inspect":
        return examine(parsed.target);
      case "go":
      case "move":
      case "walk":
        return move(directionAliases[parsed.target] ?? parsed.target);
      case "take":
      case "get":
      case "grab":
        return takeItem(parsed.target);
      case "drop":
        return dropItem(parsed.target);
      case "inventory":
      case "inv":
      case "i":
        return inventory();
      case "talk":
      case "speak":
        return talk(parsed.target);
      case "give":
        return give(parsed.target);
      case "use":
      case "light":
      case "repair":
      case "relight":
        return useItem(parsed.target || input);
      case "ring":
        return useItem(parsed.target || "bell");
      case "attack":
      case "fight":
      case "strike":
        return attack(parsed.target);
      case "rest":
      case "sleep":
        return rest();
      case "stats":
      case "status":
        return stats();
      case "map":
        return mapText();
      default:
        return [
          {
            type: "system",
            text: `I do not know "${parsed.verb}" yet. Try help, look, a direction, take, talk, use, attack, or rest.`
          }
        ];
    }
  }

  function result(messages) {
    return {
      messages,
      view: getView(),
      state: serializeState(state)
    };
  }

  return {
    intro,
    submit,
    getView,
    serialize: () => serializeState(state)
  };

  function getView() {
    return buildView(state);
  }

  function move(direction) {
    const room = getCurrentRoom(state);
    if (!directionAliases[direction] && !Object.values(directionAliases).includes(direction)) {
      return [{ type: "system", text: "Choose a direction: north, south, east, or west." }];
    }

    const canonical = directionAliases[direction] ?? direction;
    const destinationId = room.exits[canonical];
    if (!destinationId) {
      return [{ type: "system", text: "There is no path that way." }];
    }

    const livingEnemy = getLivingEnemies(state, room).find((enemy) => enemy.id === "oathlessShade");
    if (livingEnemy && canonical === "north") {
      return [
        {
          type: "danger",
          text: "The oathless shade rises in front of the stair. It will not let you pass while its borrowed flame burns."
        }
      ];
    }

    state.roomId = destinationId;
    const destination = getCurrentRoom(state);
    const firstVisit = !state.visited[destination.id];
    state.visited[destination.id] = true;

    return [
      {
        type: "room",
        text: firstVisit ? destination.firstVisit ?? destination.description : `You enter ${destination.name}.`
      },
      ...describeRoom(state)
    ];
  }

  function describeRoom(currentState) {
    const room = getCurrentRoom(currentState);
    const messages = [{ type: "room", text: room.description }];
    const roomItems = getRoomItems(currentState, room);
    const npcs = getRoomNpcs(room);
    const enemies = getLivingEnemies(currentState, room);
    const exits = Object.keys(room.exits).map((exit) => directionLabels[exit] ?? exit);

    if (room.id === "deepWell" && currentState.flags.wellLit && !currentState.inventory.includes("brassPrism")) {
      messages.push({
        type: "success",
        text: "Lantern-light stains the water black. The brass prism is visible below."
      });
    }

    if (room.id === "glassReliquary" && currentState.flags.reliquaryQuieted) {
      messages.push({
        type: "success",
        text: "The mirror is quiet now. The shelves no longer whisper your name back in the wrong voice."
      });
    }

    if (room.id === "observatory" && currentState.flags.oathstarLit) {
      messages.push({
        type: "win",
        text: "The Oathstar burns above the armillary, small and stubborn and real."
      });
    }

    if (roomItems.length) {
      messages.push({
        type: "system",
        text: `You notice ${joinNames(roomItems.map((item) => item.name))}.`
      });
    }

    if (npcs.length) {
      messages.push({
        type: "system",
        text: `${joinNames(npcs.map((npc) => npc.name))} ${npcs.length === 1 ? "is" : "are"} here.`
      });
    }

    if (enemies.length) {
      messages.push({
        type: "danger",
        text: `${joinNames(enemies.map((enemy) => enemy.name))} ${enemies.length === 1 ? "blocks" : "block"} your way.`
      });
    }

    messages.push({ type: "system", text: `Exits: ${exits.join(", ")}.` });
    return messages;
  }

  function examine(target) {
    if (!target) {
      return [{ type: "system", text: "Examine what?" }];
    }

    const item = findVisibleItem(state, target) ?? findInventoryItem(state, target);
    if (item) {
      return [{ type: "system", text: item.description }];
    }

    const npc = findNpcInRoom(state, target);
    if (npc) {
      return [{ type: "system", text: npc.description }];
    }

    const enemy = findEnemyInRoom(state, target);
    if (enemy) {
      return [{ type: "danger", text: `${enemy.description} Health: ${state.enemyHp[enemy.id]}/${enemy.maxHp}.` }];
    }

    const room = getCurrentRoom(state);
    if (matchesTarget(target, [room.name, room.shortName, room.zone])) {
      return describeRoom(state);
    }

    if (room.id === "observatory" && matchesTarget(target, ["oathstar", "star", "armillary"])) {
      return [
        {
          type: "system",
          text: state.flags.oathstarLit
            ? "The Oathstar turns slowly, throwing green and gold light over the broken city."
            : "The Oathstar is hollow. Three sockets wait: memory, mercy, and flame."
        }
      ];
    }

    return [{ type: "system", text: "You study it as best you can, but the ruin keeps that secret for now." }];
  }

  function takeItem(target) {
    if (!target) {
      return [{ type: "system", text: "Take what?" }];
    }

    const room = getCurrentRoom(state);
    const item = findRoomItem(state, room, target);
    if (!item) {
      return [{ type: "system", text: "You do not see that here." }];
    }

    if (!item.takeable) {
      return [{ type: "system", text: "That is not something you can carry." }];
    }

    removeFromArray(state.roomItems[room.id], item.id);
    addInventory(state, item.id);

    return [{ type: "success", text: `Taken: ${item.name}.` }];
  }

  function dropItem(target) {
    if (!target) {
      return [{ type: "system", text: "Drop what?" }];
    }

    const item = findInventoryItem(state, target);
    if (!item) {
      return [{ type: "system", text: "You are not carrying that." }];
    }

    removeFromArray(state.inventory, item.id);
    state.roomItems[state.roomId].push(item.id);
    return [{ type: "system", text: `Dropped: ${item.name}.` }];
  }

  function inventory() {
    if (!state.inventory.length) {
      return [{ type: "system", text: "You are carrying nothing but an unreasonable amount of nerve." }];
    }

    const items = state.inventory.map((id) => world.items[id].name);
    return [{ type: "system", text: `You carry ${joinNames(items)}.` }];
  }

  function talk(target) {
    const room = getCurrentRoom(state);
    const npc = target ? findNpcInRoom(state, target) : getRoomNpcs(room)[0];

    if (!npc) {
      return [{ type: "system", text: "No one here answers to that." }];
    }

    if (npc.id === "maskedWarden") {
      const messages = [
        {
          type: "dialogue",
          text:
            '"Three vows went dark," says the warden. "Memory sleeps below, mercy is trapped in glass, and flame is guarded by what broke its promise."'
        }
      ];

      if (!state.flags.talkedToWarden) {
        state.flags.talkedToWarden = true;
        addInventory(state, "mercyBell");
        messages.push({
          type: "success",
          text: "The warden presses a mercy bell into your palm."
        });
      } else {
        messages.push({
          type: "dialogue",
          text: '"Bring memory, mercy, and flame to the Oathstar. The city will remember how to keep dawn."'
        });
      }

      return messages;
    }

    if (npc.id === "glassArchivist") {
      if (state.inventory.includes("sealedLetter") && !state.flags.archivistLetterRead) {
        return give("sealed letter");
      }

      return [
        {
          type: "dialogue",
          text:
            'The archivist ripples inside the mirror. "I preserve endings. Bring me something with an address, and I may recall a beginning."'
        }
      ];
    }

    return [{ type: "system", text: "They have nothing more to say." }];
  }

  function give(target) {
    const room = getCurrentRoom(state);
    if (room.id !== "glassReliquary") {
      return [{ type: "system", text: "No one here seems ready to receive that." }];
    }

    const item = findInventoryItem(state, target);
    if (!item) {
      return [{ type: "system", text: "You are not carrying that." }];
    }

    if (item.id !== "sealedLetter") {
      return [{ type: "system", text: "The archivist lets that reflection pass through their hands." }];
    }

    removeFromArray(state.inventory, "sealedLetter");
    state.flags.archivistLetterRead = true;
    return [
      {
        type: "success",
        text:
          'The archivist reads the letter without opening it. "The well forgot the sun. Show it a black light."'
      }
    ];
  }

  function useItem(target) {
    if (!target) {
      return [{ type: "system", text: "Use what?" }];
    }

    const room = getCurrentRoom(state);
    const item = findInventoryItem(state, target);
    const targetWords = tokenize(target);

    if (room.id === "observatory" && targetWords.some((word) => ["oathstar", "star", "armillary"].includes(word))) {
      return relightOathstar();
    }

    if (item?.id === "blackLantern") {
      if (room.id !== "deepWell") {
        return [{ type: "system", text: "The black lantern drinks the room's shadows and gives back no answer." }];
      }

      if (!state.flags.wellLit) {
        state.flags.wellLit = true;
        addRoomItem(state, room.id, "brassPrism");
        return [
          {
            type: "success",
            text: "You open the lantern. Black light spills into the well, and a brass prism glints beneath the water."
          }
        ];
      }

      return [{ type: "system", text: "The prism is already visible in the well's black-lit water." }];
    }

    if (item?.id === "mercyBell") {
      if (room.id !== "glassReliquary") {
        return [{ type: "system", text: "The mercy bell gives one soft note, then quiets." }];
      }

      if (!state.flags.reliquaryQuieted) {
        state.flags.reliquaryQuieted = true;
        addRoomItem(state, room.id, "mercyShard");
        return [
          {
            type: "success",
            text:
              "You ring the mercy bell. The mirror cracks cleanly, and a green shard falls into the dust."
          }
        ];
      }

      return [{ type: "system", text: "The reliquary is already quiet." }];
    }

    if (room.id === "observatory" && item && ["brassPrism", "mercyShard", "emberShard"].includes(item.id)) {
      return relightOathstar();
    }

    if (!item) {
      return [{ type: "system", text: "You are not carrying that." }];
    }

    return [{ type: "system", text: `You try the ${item.name}, but the ruin does not answer here.` }];
  }

  function relightOathstar() {
    if (state.roomId !== "observatory") {
      return [{ type: "system", text: "The Oathstar is not here." }];
    }

    if (state.flags.oathstarLit) {
      return [{ type: "win", text: "The Oathstar is already awake." }];
    }

    const required = ["brassPrism", "mercyShard", "emberShard"];
    const missing = required.filter((itemId) => !state.inventory.includes(itemId));
    if (missing.length) {
      return [
        {
          type: "system",
          text: `The sockets reject your empty hands. Missing: ${joinNames(missing.map((id) => world.items[id].name))}.`
        }
      ];
    }

    for (const itemId of required) {
      removeFromArray(state.inventory, itemId);
    }

    state.flags.oathstarLit = true;
    return [
      {
        type: "win",
        text:
          "You set memory, mercy, and flame into the armillary. The Oathstar catches, no larger than a fist and bright enough to change the weather."
      },
      {
        type: "win",
        text:
          "Far below, the gate exhales. The city has not been saved forever. It has been saved for morning, and morning is enough."
      }
    ];
  }

  function attack(target) {
    const enemy = target ? findEnemyInRoom(state, target) : getLivingEnemies(state, getCurrentRoom(state))[0];
    if (!enemy) {
      return [{ type: "system", text: "There is nothing here you need to fight." }];
    }

    const playerDamage = 4 + Math.floor(rng() * 4);
    state.enemyHp[enemy.id] = Math.max(0, state.enemyHp[enemy.id] - playerDamage);
    const messages = [
      {
        type: "danger",
        text: `You strike the ${enemy.name} for ${playerDamage}.`
      }
    ];

    if (state.enemyHp[enemy.id] <= 0) {
      state.defeated.push(enemy.id);
      addRoomItem(state, state.roomId, "emberShard");
      messages.push({
        type: "success",
        text: "The shade folds inward with a sound like torn paper. An ember shard drops to the iron floor."
      });
      return messages;
    }

    const enemyDamage = 2 + Math.floor(rng() * enemy.attack);
    state.hp = Math.max(0, state.hp - enemyDamage);
    messages.push({
      type: "danger",
      text: `The ${enemy.name} answers for ${enemyDamage}.`
    });

    if (state.hp <= 0) {
      state.hp = 1;
      state.roomId = "starfallGate";
      messages.push({
        type: "danger",
        text:
          "You fall through a cold dark and wake at Starfall Gate with one heartbeat left. The ruin is not finished with you."
      });
    }

    return messages;
  }

  function rest() {
    const room = getCurrentRoom(state);
    if (!room.safe) {
      return [{ type: "system", text: "This place will not let you rest." }];
    }

    const oldHp = state.hp;
    const oldFocus = state.focus;
    state.hp = Math.min(state.maxHp, state.hp + 6);
    state.focus = Math.min(state.maxFocus, state.focus + 2);

    return [
      {
        type: "success",
        text: `You gather yourself. Health ${oldHp} -> ${state.hp}. Focus ${oldFocus} -> ${state.focus}.`
      }
    ];
  }

  function stats() {
    return [
      {
        type: "system",
        text: `Health ${state.hp}/${state.maxHp}. Focus ${state.focus}/${state.maxFocus}. Turn ${state.turn}.`
      }
    ];
  }

  function mapText() {
    const room = getCurrentRoom(state);
    return [
      {
        type: "system",
        text: `You are at ${room.name}. Known rooms: ${Object.keys(state.visited)
          .filter((id) => state.visited[id])
          .map((id) => world.rooms[id].name)
          .join(", ")}.`
      }
    ];
  }

  function help() {
    return [
      {
        type: "system",
        text:
          "Useful commands: look, examine target, north/south/east/west, take item, inventory, talk person, give item, use item, attack enemy, rest, stats, map."
      }
    ];
  }
}

export function createInitialState() {
  const roomItems = {};
  for (const [roomId, room] of Object.entries(world.rooms)) {
    roomItems[roomId] = [...(room.items ?? [])];
  }

  const enemyHp = {};
  for (const [enemyId, enemy] of Object.entries(world.enemies)) {
    enemyHp[enemyId] = enemy.maxHp;
  }

  return {
    version: SAVE_VERSION,
    roomId: world.startRoom,
    turn: 0,
    hp: 18,
    maxHp: 18,
    focus: 5,
    maxFocus: 5,
    inventory: [],
    roomItems,
    flags: {},
    visited: {},
    enemyHp,
    defeated: []
  };
}

export function buildView(state) {
  const room = getCurrentRoom(state);
  const visibleRooms = Object.values(world.rooms).map((mapRoom) => ({
    id: mapRoom.id,
    name: mapRoom.name,
    shortName: mapRoom.shortName,
    zone: mapRoom.zone,
    x: mapRoom.x,
    y: mapRoom.y,
    visited: Boolean(state.visited[mapRoom.id]),
    current: mapRoom.id === room.id
  }));

  const items = getRoomItems(state, room);
  const npcs = getRoomNpcs(room);
  const enemies = getLivingEnemies(state, room);
  const nearby = [
    ...items.map((item) => ({ id: item.id, name: item.name, kind: "item" })),
    ...npcs.map((npc) => ({ id: npc.id, name: npc.name, kind: "npc" })),
    ...enemies.map((enemy) => ({
      id: enemy.id,
      name: `${enemy.name} (${state.enemyHp[enemy.id]}/${enemy.maxHp})`,
      kind: "enemy"
    }))
  ];

  const quests = questDefinitions.map((quest) => ({
    id: quest.id,
    label: quest.label,
    complete: quest.complete(state)
  }));

  return {
    room: {
      id: room.id,
      name: room.name,
      zone: room.zone,
      description: dynamicRoomDescription(state, room),
      exits: Object.entries(room.exits).map(([direction, destinationId]) => ({
        direction,
        label: directionLabels[direction] ?? direction,
        destinationId,
        destinationName: world.rooms[destinationId].name
      }))
    },
    stats: {
      hp: state.hp,
      maxHp: state.maxHp,
      focus: state.focus,
      maxFocus: state.maxFocus,
      turn: state.turn
    },
    inventory: state.inventory.map((id) => world.items[id]),
    nearby,
    quests,
    map: visibleRooms,
    flags: { ...state.flags },
    won: Boolean(state.flags.oathstarLit)
  };
}

export function serializeState(state) {
  return JSON.parse(JSON.stringify({ ...state, version: SAVE_VERSION }));
}

export function hydrateState(savedState) {
  const fresh = createInitialState();
  const merged = {
    ...fresh,
    ...savedState,
    flags: { ...fresh.flags, ...(savedState.flags ?? {}) },
    visited: { ...fresh.visited, ...(savedState.visited ?? {}) },
    enemyHp: { ...fresh.enemyHp, ...(savedState.enemyHp ?? {}) },
    roomItems: { ...fresh.roomItems, ...(savedState.roomItems ?? {}) },
    inventory: Array.isArray(savedState.inventory) ? [...savedState.inventory] : fresh.inventory,
    defeated: Array.isArray(savedState.defeated) ? [...savedState.defeated] : fresh.defeated
  };

  if (!world.rooms[merged.roomId]) {
    merged.roomId = world.startRoom;
  }

  return merged;
}

function parseCommand(input) {
  const words = tokenize(input);
  const first = words[0] ?? "";

  if (directionAliases[first]) {
    return { kind: "direction", direction: directionAliases[first] };
  }

  if (first === "go" && directionAliases[words[1]]) {
    return { kind: "direction", direction: directionAliases[words[1]] };
  }

  const verb = first;
  const target = words.slice(1).filter((word) => !articles.has(word)).join(" ");
  return { kind: "verb", verb, target };
}

function normalizeInput(rawInput) {
  return String(rawInput ?? "")
    .trim()
    .replace(/\s+/g, " ")
    .toLowerCase();
}

function tokenize(input) {
  return normalizeInput(input)
    .replace(/[^\w\s-]/g, "")
    .split(" ")
    .filter(Boolean);
}

function getCurrentRoom(state) {
  return world.rooms[state.roomId] ?? world.rooms[world.startRoom];
}

function getRoomItems(state, room) {
  return (state.roomItems[room.id] ?? []).map((id) => world.items[id]).filter(Boolean);
}

function getRoomNpcs(room) {
  return (room.npcs ?? []).map((id) => world.npcs[id]).filter(Boolean);
}

function getLivingEnemies(state, room) {
  return (room.enemies ?? [])
    .filter((id) => !state.defeated.includes(id))
    .map((id) => world.enemies[id])
    .filter(Boolean);
}

function findRoomItem(state, room, target) {
  return getRoomItems(state, room).find((item) => matchesTarget(target, [item.name, ...item.aliases]));
}

function findVisibleItem(state, target) {
  return findRoomItem(state, getCurrentRoom(state), target);
}

function findInventoryItem(state, target) {
  return state.inventory
    .map((id) => world.items[id])
    .find((item) => matchesTarget(target, [item.name, ...item.aliases]));
}

function findNpcInRoom(state, target) {
  return getRoomNpcs(getCurrentRoom(state)).find((npc) => matchesTarget(target, [npc.name, ...npc.aliases]));
}

function findEnemyInRoom(state, target) {
  return getLivingEnemies(state, getCurrentRoom(state)).find((enemy) =>
    matchesTarget(target, [enemy.name, ...enemy.aliases])
  );
}

function matchesTarget(target, names) {
  const normalizedTarget = normalizeInput(target);
  if (!normalizedTarget) {
    return false;
  }

  return names.some((name) => {
    const normalizedName = normalizeInput(name);
    return normalizedName === normalizedTarget || normalizedName.includes(normalizedTarget);
  });
}

function addInventory(state, itemId) {
  if (!state.inventory.includes(itemId)) {
    state.inventory.push(itemId);
  }
}

function addRoomItem(state, roomId, itemId) {
  if (!state.roomItems[roomId].includes(itemId) && !state.inventory.includes(itemId)) {
    state.roomItems[roomId].push(itemId);
  }
}

function removeFromArray(array, value) {
  const index = array.indexOf(value);
  if (index >= 0) {
    array.splice(index, 1);
  }
}

function joinNames(names) {
  if (names.length <= 1) {
    return names[0] ?? "";
  }

  if (names.length === 2) {
    return `${names[0]} and ${names[1]}`;
  }

  return `${names.slice(0, -1).join(", ")}, and ${names.at(-1)}`;
}

function dynamicRoomDescription(state, room) {
  if (room.id === "deepWell" && state.flags.wellLit) {
    return "Black lantern-light pools in the well, revealing shapes beneath the water that daylight politely ignored.";
  }

  if (room.id === "ironSanctum" && state.defeated.includes("oathlessShade")) {
    return "The sanctum is quiet except for iron settling in the walls. The northern stair is open.";
  }

  if (room.id === "observatory" && state.flags.oathstarLit) {
    return "The armillary turns with a clean brass sound. The awakened Oathstar paints the dust in green and gold.";
  }

  return room.description;
}
