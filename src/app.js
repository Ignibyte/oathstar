import { createGame } from "./engine.js";

const saveKey = "oathstar-save-v1";
const equipmentSlots = ["Main hand", "Off hand", "Body", "Left earring", "Right earring", "Trinket"];
const mapTileConfig = {
  spritePixels: 32
};

const elements = {
  log: document.querySelector("#log"),
  form: document.querySelector("#command-form"),
  input: document.querySelector("#command-input"),
  roomKicker: document.querySelector("#room-kicker"),
  roomName: document.querySelector("#room-name"),
  roomDescription: document.querySelector("#room-description"),
  exits: document.querySelector("#exits"),
  map: document.querySelector("#map"),
  mapLabel: document.querySelector("#map-label"),
  inventory: document.querySelector("#inventory"),
  inventoryCount: document.querySelector("#inventory-count"),
  equipment: document.querySelector("#equipment"),
  equipmentCount: document.querySelector("#equipment-count"),
  nearby: document.querySelector("#nearby"),
  nearbyCount: document.querySelector("#nearby-count"),
  quests: document.querySelector("#quests"),
  questCount: document.querySelector("#quest-count"),
  quickCommands: document.querySelector("#quick-commands"),
  quickCount: document.querySelector("#quick-count"),
  commandSearch: document.querySelector("#command-search"),
  commandContext: document.querySelector("#command-context"),
  hpValue: document.querySelector("#hp-value"),
  hpBar: document.querySelector("#hp-bar"),
  focusValue: document.querySelector("#focus-value"),
  focusBar: document.querySelector("#focus-bar"),
  turnCount: document.querySelector("#turn-count"),
  saveButton: document.querySelector("#save-button"),
  loadButton: document.querySelector("#load-button"),
  newButton: document.querySelector("#new-button"),
  menuTabs: [...document.querySelectorAll(".menu-tab")],
  tabPanels: [...document.querySelectorAll(".tab-panel")]
};

let game = createGame();
let commandHistory = [];
let historyIndex = 0;

boot();

function boot() {
  appendMessages(game.intro());
  render(game.getView());
  bindEvents();
}

function bindEvents() {
  elements.form.addEventListener("submit", (event) => {
    event.preventDefault();
    runCommand(elements.input.value);
  });

  elements.input.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      elements.form.requestSubmit();
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      cycleHistory(-1);
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      cycleHistory(1);
    }
  });

  elements.saveButton.addEventListener("click", () => saveGame());
  elements.loadButton.addEventListener("click", () => loadGame());
  elements.newButton.addEventListener("click", () => newGame());
  elements.commandSearch.addEventListener("input", () => renderQuickCommands(game.getView()));

  for (const tab of elements.menuTabs) {
    tab.addEventListener("click", () => setActiveMenuTab(tab.dataset.tab));
    tab.addEventListener("keydown", (event) => navigateMenuTabs(event, tab));
  }

  setActiveMenuTab("nearby");
}

function runCommand(rawCommand) {
  const command = rawCommand.trim();
  if (!command) {
    return;
  }

  commandHistory.push(command);
  historyIndex = commandHistory.length;
  appendMessage({ type: "command", text: `> ${command}` });
  elements.input.value = "";

  if (command.toLowerCase() === "clear") {
    elements.log.replaceChildren();
    appendMessage({ type: "system", text: "The old ink fades." });
    return;
  }

  if (command.toLowerCase() === "save") {
    saveGame();
    return;
  }

  if (command.toLowerCase() === "load") {
    loadGame();
    return;
  }

  if (["new", "restart", "reset"].includes(command.toLowerCase())) {
    newGame();
    return;
  }

  const outcome = game.submit(command);
  appendMessages(outcome.messages);
  render(outcome.view);
}

function saveGame() {
  localStorage.setItem(saveKey, JSON.stringify(game.serialize()));
  appendMessage({ type: "success", text: "Saved." });
  elements.input.focus();
}

function loadGame() {
  const saved = localStorage.getItem(saveKey);
  if (!saved) {
    appendMessage({ type: "system", text: "No saved oath exists yet." });
    elements.input.focus();
    return;
  }

  try {
    game = createGame({ state: JSON.parse(saved) });
    appendMessage({ type: "success", text: "Loaded." });
    appendMessages([{ type: "room", text: `You return to ${game.getView().room.name}.` }]);
    render(game.getView());
  } catch {
    appendMessage({ type: "danger", text: "That save is unreadable." });
  }

  elements.input.focus();
}

function newGame() {
  localStorage.removeItem(saveKey);
  game = createGame();
  elements.log.replaceChildren();
  appendMessages(game.intro());
  render(game.getView());
  elements.input.focus();
}

function appendMessages(messages) {
  for (const message of messages) {
    appendMessage(message);
  }
}

function appendMessage(message) {
  const entry = document.createElement("article");
  entry.className = `log-entry ${message.type ?? "system"}`;

  const meta = document.createElement("span");
  meta.className = "log-meta";
  meta.textContent = logLabel(message.type);

  const text = document.createElement("p");
  text.textContent = message.text;

  entry.append(meta, text);
  elements.log.append(entry);
  elements.log.scrollTop = elements.log.scrollHeight;
}

function logLabel(type = "system") {
  const labels = {
    command: "Command",
    room: "Room",
    success: "Oath",
    danger: "Threat",
    dialogue: "Dialogue",
    system: "System",
    win: "Oathstar"
  };
  return labels[type] ?? "System";
}

function render(view) {
  elements.roomKicker.textContent = view.room.zone;
  elements.roomName.textContent = view.room.name;
  elements.roomDescription.textContent = view.room.description;
  elements.turnCount.textContent = `Turn ${view.stats.turn}`;
  elements.hpValue.textContent = `${view.stats.hp}/${view.stats.maxHp}`;
  elements.focusValue.textContent = `${view.stats.focus}/${view.stats.maxFocus}`;
  elements.hpBar.style.width = `${Math.max(0, (view.stats.hp / view.stats.maxHp) * 100)}%`;
  elements.focusBar.style.width = `${Math.max(0, (view.stats.focus / view.stats.maxFocus) * 100)}%`;

  renderExits(view);
  renderMap(view);
  renderInventory(view);
  renderEquipment();
  renderNearby(view);
  renderQuests(view);
  renderQuickCommands(view);
}

function renderExits(view) {
  elements.exits.replaceChildren();
  for (const exit of view.room.exits) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = exit.label;
    button.title = exit.destinationName;
    button.addEventListener("click", () => runCommand(exit.direction));
    elements.exits.append(button);
  }
}

function renderMap(view) {
  elements.map.replaceChildren();
  elements.mapLabel.textContent = view.room.zone;
  elements.map.dataset.spriteTileSize = String(mapTileConfig.spritePixels);

  const xs = view.map.map((room) => room.x);
  const ys = view.map.map((room) => room.y);
  const minX = Math.min(...xs);
  const maxX = Math.max(...xs);
  const minY = Math.min(...ys);
  const maxY = Math.max(...ys);
  const roomsByPosition = new Map(view.map.map((room) => [`${room.x},${room.y}`, room]));
  elements.map.style.gridTemplateColumns = `repeat(${maxX - minX + 1}, minmax(0, 1fr))`;

  for (let y = minY; y <= maxY; y += 1) {
    for (let x = minX; x <= maxX; x += 1) {
      const room = roomsByPosition.get(`${x},${y}`);
      const cell = document.createElement("div");
      cell.className = "map-cell";
      cell.setAttribute("aria-label", room && (room.visited || room.current) ? room.name : "Uncharted");

      if (room?.visited || room?.current) {
        const name = document.createElement("span");
        name.className = "map-name";
        name.textContent = room.shortName;
        const zone = document.createElement("span");
        zone.className = "map-zone";
        zone.textContent = room.zone;
        cell.append(name, zone);
        cell.classList.add("visited");
      }

      if (room?.current) {
        cell.classList.add("current");
      }

      elements.map.append(cell);
    }
  }
}

function renderInventory(view) {
  elements.inventory.replaceChildren();
  elements.inventoryCount.textContent = `${view.inventory.length} ${view.inventory.length === 1 ? "item" : "items"}`;

  if (!view.inventory.length) {
    elements.inventory.append(emptyChip("empty"));
    return;
  }

  for (const item of view.inventory) {
    const chip = commandChip(item.name, `examine ${item.name}`);
    elements.inventory.append(chip);
  }
}

function renderEquipment() {
  elements.equipment.replaceChildren();
  elements.equipmentCount.textContent = `0/${equipmentSlots.length}`;

  for (const slot of equipmentSlots) {
    const row = document.createElement("div");
    row.className = "equipment-slot";

    const label = document.createElement("span");
    label.className = "equipment-label";
    label.textContent = slot;

    const value = document.createElement("span");
    value.className = "equipment-value empty";
    value.textContent = "empty";

    row.append(label, value);
    elements.equipment.append(row);
  }
}

function renderNearby(view) {
  elements.nearby.replaceChildren();
  elements.nearbyCount.textContent = `${view.nearby.length} ${view.nearby.length === 1 ? "thing" : "things"}`;

  if (!view.nearby.length) {
    elements.nearby.append(emptyChip("quiet"));
    return;
  }

  for (const nearby of view.nearby) {
    elements.nearby.append(nearbyCard(nearby));
  }
}

function renderQuests(view) {
  elements.quests.replaceChildren();
  const completeCount = view.quests.filter((quest) => quest.complete).length;
  elements.questCount.textContent = `${completeCount}/${view.quests.length}`;

  for (const quest of view.quests) {
    const item = document.createElement("li");
    item.textContent = quest.label;
    item.className = quest.complete ? "complete" : "";
    elements.quests.append(item);
  }
}

function renderQuickCommands(view) {
  elements.quickCommands.replaceChildren();
  const baseCommands = ["look", "inventory", "rest", "stats", "map", "help"];
  const contextual = [];

  if (view.nearby.some((thing) => thing.kind === "npc")) {
    contextual.push("talk");
  }

  if (view.nearby.some((thing) => thing.kind === "enemy")) {
    contextual.push("attack");
  }

  if (view.inventory.some((item) => item.id === "blackLantern") && view.room.id === "deepWell") {
    contextual.push("use lantern");
  }

  if (view.inventory.some((item) => item.id === "mercyBell") && view.room.id === "glassReliquary") {
    contextual.push("ring bell");
  }

  if (view.room.id === "observatory") {
    contextual.push("use oathstar");
  }

  const search = elements.commandSearch.value.trim().toLowerCase();
  const commands = [...contextual, ...baseCommands].filter((command) =>
    command.toLowerCase().includes(search)
  );
  elements.commandContext.textContent = view.won ? "Lit" : "Ready";
  elements.quickCount.textContent = `${commands.length} ${commands.length === 1 ? "command" : "commands"}`;

  if (!commands.length) {
    elements.quickCommands.append(emptyChip("no commands"));
    return;
  }

  for (const command of commands) {
    elements.quickCommands.append(commandButton(command));
  }
}

function commandChip(label, command) {
  const chip = document.createElement("button");
  chip.type = "button";
  chip.className = "chip";
  chip.textContent = label;
  chip.title = command;
  chip.addEventListener("click", () => runCommand(command));
  return chip;
}

function nearbyCard(nearby) {
  const card = document.createElement("article");
  card.className = `entity-card ${nearby.kind}`;

  const main = document.createElement("div");
  main.className = "entity-main";

  const name = document.createElement("strong");
  name.className = "entity-name";
  name.textContent = nearby.name;

  const kind = document.createElement("span");
  kind.className = "entity-kind";
  kind.textContent = nearby.kind;

  main.append(name, kind);

  const actions = document.createElement("div");
  actions.className = "entity-actions";

  for (const action of nearbyActions(nearby)) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = action.label;
    button.title = action.command;
    button.addEventListener("click", () => runCommand(action.command));
    actions.append(button);
  }

  card.append(main, actions);
  return card;
}

function nearbyActions(nearby) {
  const baseName = nearby.name.split(" (")[0];
  if (nearby.kind === "enemy") {
    return [
      { label: "Attack", command: `attack ${baseName}` },
      { label: "Examine", command: `examine ${baseName}` }
    ];
  }

  if (nearby.kind === "npc") {
    return [
      { label: "Talk", command: `talk ${nearby.name}` },
      { label: "Examine", command: `examine ${nearby.name}` }
    ];
  }

  return [
    { label: "Take", command: `take ${nearby.name}` },
    { label: "Examine", command: `examine ${nearby.name}` }
  ];
}

function emptyChip(label) {
  const chip = document.createElement("span");
  chip.className = "chip empty";
  chip.textContent = label;
  return chip;
}

function commandButton(command) {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = command;
  button.addEventListener("click", () => {
    if (["talk", "attack"].includes(command)) {
      elements.input.value = `${command} `;
      elements.input.focus();
      return;
    }

    runCommand(command);
  });
  return button;
}

function setActiveMenuTab(tabId) {
  for (const tab of elements.menuTabs) {
    const active = tab.dataset.tab === tabId;
    tab.classList.toggle("active", active);
    tab.setAttribute("aria-selected", String(active));
    tab.tabIndex = active ? 0 : -1;
  }

  for (const panel of elements.tabPanels) {
    const active = panel.dataset.panel === tabId;
    panel.classList.toggle("active", active);
    panel.hidden = !active;
  }
}

function navigateMenuTabs(event, currentTab) {
  if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) {
    return;
  }

  event.preventDefault();
  const currentIndex = elements.menuTabs.indexOf(currentTab);
  const lastIndex = elements.menuTabs.length - 1;
  const nextIndex =
    event.key === "Home"
      ? 0
      : event.key === "End"
        ? lastIndex
        : event.key === "ArrowLeft"
          ? Math.max(0, currentIndex - 1)
          : Math.min(lastIndex, currentIndex + 1);
  const nextTab = elements.menuTabs[nextIndex];
  setActiveMenuTab(nextTab.dataset.tab);
  nextTab.focus();
}

function cycleHistory(delta) {
  if (!commandHistory.length) {
    return;
  }

  historyIndex = Math.min(commandHistory.length, Math.max(0, historyIndex + delta));
  elements.input.value = commandHistory[historyIndex] ?? "";
  requestAnimationFrame(() => {
    elements.input.selectionStart = elements.input.value.length;
    elements.input.selectionEnd = elements.input.value.length;
  });
}
