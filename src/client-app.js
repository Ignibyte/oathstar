// Oathstar production client (browser glue).
//
// Server-authoritative: the Rust runtime owns the game. This shell POSTs to
// `/command`, consumes the `/events` SSE stream, and renders `/state` snapshots
// into the map-forward layout. It is framework-free (no React/Vue/Svelte) and
// stays Datastar/SSE-compatible (docs/ui-design.md guardrails, REQ-008).
//
// All render/logic lives in the pure, tested modules under `./client/`; this
// file is the DOM + transport seam (EventSource / fetch / document) and is
// verified by smoke, mirroring the prototype `app.js` split.

import { parseEvent } from "./client/wire.js";
import { toComponent } from "./client/components.js";
import { toHud, toMenuModel } from "./client/snapshot.js";
import { toRoomDisplay, toExitPad } from "./client/room.js";
import { toMapModel, DEFAULT_MAP_CONFIG } from "./client/map.js";
import { suggestCommands } from "./client/intent.js";

// Resolve the server base URL. Default to same-origin (vite dev proxy, or the
// server serving the page); a Tauri build can bake VITE_OATHSTAR_API to point
// the packaged client at the loopback server.
function resolveApiBase() {
  try {
    if (import.meta && import.meta.env && import.meta.env.VITE_OATHSTAR_API) {
      return import.meta.env.VITE_OATHSTAR_API;
    }
  } catch (_err) {
    // import.meta.env is absent outside a bundler — fall through to same-origin.
  }
  return "";
}

const API_BASE = resolveApiBase();

// Client-owned map render config (REQ-007). Kept out of server state so a later
// canvas/sprite renderer can change tile size / mode without a protocol change.
const mapRenderConfig = { ...DEFAULT_MAP_CONFIG };

const el = {
  log: document.querySelector("#log"),
  form: document.querySelector("#command-form"),
  input: document.querySelector("#command-input"),
  commandContext: document.querySelector("#command-context"),
  roomKicker: document.querySelector("#room-kicker"),
  roomName: document.querySelector("#room-name"),
  roomDescription: document.querySelector("#room-description"),
  exitLine: document.querySelector("#exit-line"),
  exitPad: document.querySelector("#exit-pad"),
  viewRoomButton: document.querySelector("#view-room-button"),
  roomModal: document.querySelector("#room-modal"),
  roomModalTitle: document.querySelector("#room-modal-title"),
  roomModalDescription: document.querySelector("#room-modal-description"),
  roomModalExits: document.querySelector("#room-modal-exits"),
  roomModalMedia: document.querySelector("#room-modal-media"),
  map: document.querySelector("#map"),
  mapLabel: document.querySelector("#map-label"),
  hpValue: document.querySelector("#hp-value"),
  hpBar: document.querySelector("#hp-bar"),
  focusValue: document.querySelector("#focus-value"),
  focusBar: document.querySelector("#focus-bar"),
  turnCount: document.querySelector("#turn-count"),
  nearby: document.querySelector("#nearby"),
  nearbyCount: document.querySelector("#nearby-count"),
  quests: document.querySelector("#quests"),
  questCount: document.querySelector("#quest-count"),
  equipment: document.querySelector("#equipment"),
  equipmentCount: document.querySelector("#equipment-count"),
  inventory: document.querySelector("#inventory"),
  inventoryCount: document.querySelector("#inventory-count"),
  quickCommands: document.querySelector("#quick-commands"),
  quickCount: document.querySelector("#quick-count"),
  commandSearch: document.querySelector("#command-search"),
  saveButton: document.querySelector("#save-button"),
  loadButton: document.querySelector("#load-button"),
  newButton: document.querySelector("#new-button"),
  menuTabs: [...document.querySelectorAll(".menu-tab")],
  tabPanels: [...document.querySelectorAll(".tab-panel")],
};

let latestSnapshot = null;
const history = [];
let historyIndex = 0;
// Event ids already rendered into the feed. EventSource auto-reconnects and the
// server re-seeds the opening scene on every subscription, so dedup by id keeps
// the feed from duplicating the opening (or any event) across reconnects.
const seenEventIds = new Set();

boot();

function boot() {
  bindEvents();
  setActiveMenuTab("nearby");
  refreshState();
  connectEvents();
}

function bindEvents() {
  el.form.addEventListener("submit", (event) => {
    event.preventDefault();
    runCommand(el.input.value);
  });

  el.input.addEventListener("keydown", (event) => {
    if (event.key === "ArrowUp") {
      event.preventDefault();
      cycleHistory(-1);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      cycleHistory(1);
    }
  });

  el.commandSearch.addEventListener("input", () => {
    if (latestSnapshot) {
      renderIntent(latestSnapshot);
    }
  });

  el.newButton.addEventListener("click", () => {
    el.log.replaceChildren();
    appendLine("system", "System", "Showing the current session.");
    refreshState();
    el.input.focus();
  });
  const unavailable = () =>
    appendLine("system", "System", "Save/load isn't wired into this shell yet.");
  el.saveButton.addEventListener("click", unavailable);
  el.loadButton.addEventListener("click", unavailable);

  for (const tab of el.menuTabs) {
    tab.addEventListener("click", () => setActiveMenuTab(tab.dataset.tab));
    tab.addEventListener("keydown", (event) => navigateMenuTabs(event, tab));
  }

  el.viewRoomButton.addEventListener("click", () => {
    if (latestSnapshot) {
      openRoomModal(toRoomDisplay(latestSnapshot));
    }
  });
  // Clicking the dialog backdrop (the dialog element itself) closes it.
  el.roomModal.addEventListener("click", (event) => {
    if (event.target === el.roomModal) {
      el.roomModal.close();
    }
  });
}

// ---- transport ------------------------------------------------------------

async function refreshState() {
  try {
    const response = await fetch(`${API_BASE}/state`);
    if (!response.ok) {
      return;
    }
    latestSnapshot = await response.json();
    renderAll(latestSnapshot);
  } catch (_err) {
    el.commandContext.textContent = "Offline";
  }
}

function connectEvents() {
  const source = new EventSource(`${API_BASE}/events`);

  source.addEventListener("open", () => {
    el.commandContext.textContent = "Connected";
  });

  source.addEventListener("game_event", (event) => {
    let raw;
    try {
      raw = JSON.parse(event.data);
    } catch (_err) {
      return;
    }
    const parsed = parseEvent(raw);
    const descriptor = toComponent(parsed);
    if (descriptor) {
      appendComponent(descriptor);
    }
    // Refresh panels/map/HUD on state-affecting events (the feed already updated).
    if (
      parsed &&
      (parsed.type === "room_entered" ||
        parsed.type === "oath_sworn" ||
        parsed.type === "oath_fulfilled")
    ) {
      refreshState();
    }
  });

  source.addEventListener("error", () => {
    el.commandContext.textContent = "Reconnecting…";
  });
}

async function runCommand(rawInput) {
  const input = rawInput.trim();
  if (!input) {
    return;
  }
  history.push(input);
  historyIndex = history.length;
  appendLine("command", "Command", `> ${input}`);
  el.input.value = "";

  try {
    const response = await fetch(`${API_BASE}/command`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ input }),
    });
    if (!response.ok) {
      appendLine("danger", "System", `The command failed (${response.status}).`);
      return;
    }
    // The command's events arrive on the SSE stream (single source for the
    // feed); the response snapshot drives the HUD / map / panels.
    const data = await response.json();
    if (data && data.snapshot) {
      latestSnapshot = data.snapshot;
      renderAll(latestSnapshot);
    }
  } catch (_err) {
    appendLine("danger", "System", "Could not reach the server.");
  } finally {
    el.input.focus();
  }
}

// ---- rendering ------------------------------------------------------------

function renderAll(snapshot) {
  if (!snapshot) {
    return;
  }
  renderHud(snapshot);
  renderRoom(snapshot);
  renderMap(snapshot);
  renderMenu(snapshot);
  renderIntent(snapshot);
}

function renderHud(snapshot) {
  const hud = toHud(snapshot);
  el.hpValue.textContent = `${hud.hp}/${hud.maxHp}`;
  el.focusValue.textContent = `${hud.focus}/${hud.maxFocus}`;
  el.hpBar.style.width = `${hud.hpPct}%`;
  el.focusBar.style.width = `${hud.focusPct}%`;
  el.turnCount.textContent = `Turn ${hud.tick}`;
}

function renderRoom(snapshot) {
  const display = toRoomDisplay(snapshot);
  const hud = toHud(snapshot);
  el.roomName.textContent = hud.roomName;
  el.roomKicker.textContent = hud.roomKicker;
  el.roomDescription.textContent = display.main;
  el.exitLine.textContent = display.exits.length
    ? `Exits: ${display.exits.map((exit) => exit.direction).join(", ")}`
    : "Exits: none";
  renderExitPad(toExitPad(snapshot));
}

// The directional Exit Pad. Enabled buttons send the same canonical movement
// command as the text prompt; unavailable directions are disabled + quiet.
function renderExitPad(pad) {
  el.exitPad.replaceChildren();
  for (const entry of pad.directions) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = `exit-pad-button dir-${entry.dir}`;
    button.textContent = entry.label;
    if (entry.available) {
      button.title = `Travel ${entry.dir}`;
      button.setAttribute("aria-label", entry.dir);
      button.addEventListener("click", () => runCommand(entry.command));
    } else {
      button.title = `No ${entry.dir} exit`;
      button.setAttribute("aria-label", `${entry.dir} (unavailable)`);
      button.disabled = true;
      button.classList.add("is-quiet");
    }
    el.exitPad.append(button);
  }
}

// Focused full-room view. The modal is a pure overlay over latestSnapshot + the
// feed/map DOM, so opening or closing it changes no game state (REQ-006/007).
function openRoomModal(display) {
  el.roomModalTitle.textContent = display.title;
  el.roomModalDescription.textContent = display.full;

  el.roomModalExits.replaceChildren();
  if (display.exits.length) {
    for (const exit of display.exits) {
      const chip = document.createElement("span");
      chip.className = "chip";
      chip.textContent = exit.label;
      el.roomModalExits.append(chip);
    }
  } else {
    el.roomModalExits.append(emptyChip("no exits"));
  }

  // Reserved media area — hidden until a room carries a media hint (REQ-008).
  if (display.mediaHint) {
    el.roomModalMedia.hidden = false;
    el.roomModalMedia.textContent = display.mediaHint.alt ?? "Scene";
  } else {
    el.roomModalMedia.hidden = true;
    el.roomModalMedia.textContent = "";
  }

  if (typeof el.roomModal.showModal === "function") {
    el.roomModal.showModal();
  }
}

function renderMap(snapshot) {
  const model = toMapModel(snapshot.map, mapRenderConfig);
  el.map.replaceChildren();
  el.map.dataset.renderMode = model.mode;
  el.map.dataset.tileSize = String(model.tilePixels);
  el.map.style.setProperty("--tile-size", `${model.tilePixels}px`);
  el.mapLabel.textContent =
    model.planes && model.planes.length > 1
      ? `${model.region || "Map"} · floor ${model.z}`
      : model.region || "Map";

  if (!model.columns) {
    return;
  }
  el.map.style.gridTemplateColumns = `repeat(${model.columns}, minmax(0, 1fr))`;
  for (const cell of model.cells) {
    const node = document.createElement("div");
    node.className = "map-cell";
    if (cell.present && (cell.discovered || cell.current)) {
      node.classList.add("visited");
      const name = document.createElement("span");
      name.className = "map-name";
      name.textContent = model.mode === "ascii" ? cell.glyph : cell.title || cell.glyph;
      const zone = document.createElement("span");
      zone.className = "map-zone";
      zone.textContent = cell.current ? "here" : "";
      node.append(name, zone);
      node.setAttribute("aria-label", cell.title || "Room");
    } else {
      node.setAttribute("aria-label", "Uncharted");
    }
    if (cell.current) {
      node.classList.add("current");
    }
    el.map.append(node);
  }
}

function renderMenu(snapshot) {
  const menu = toMenuModel(snapshot);

  el.nearby.replaceChildren();
  el.nearbyCount.textContent = `${menu.nearby.count} ${menu.nearby.count === 1 ? "thing" : "things"}`;
  if (!menu.nearby.count) {
    el.nearby.append(emptyChip("no one here"));
  }
  for (const item of menu.nearby.items) {
    el.nearby.append(actionCard(item));
  }

  el.quests.replaceChildren();
  el.questCount.textContent = `${menu.oaths.complete}/${menu.oaths.count}`;
  for (const oath of menu.oaths.items) {
    const li = document.createElement("li");
    li.textContent = oath.complete ? `${oath.title} (fulfilled)` : oath.title;
    li.className = oath.complete ? "complete" : "";
    el.quests.append(li);
  }

  el.equipment.replaceChildren();
  el.equipmentCount.textContent = `${menu.gear.filled}/${menu.gear.total}`;
  for (const slot of menu.gear.slots) {
    const row = document.createElement("div");
    row.className = "equipment-slot";
    const label = document.createElement("span");
    label.className = "equipment-label";
    label.textContent = slot.label;
    const value = document.createElement("span");
    value.className = "equipment-value empty";
    value.textContent = slot.value;
    row.append(label, value);
    el.equipment.append(row);
  }

  el.inventory.replaceChildren();
  el.inventoryCount.textContent = `${menu.pack.count} ${menu.pack.count === 1 ? "item" : "items"}`;
  if (!menu.pack.count) {
    el.inventory.append(emptyChip("empty"));
  }
}

function renderIntent(snapshot) {
  el.quickCommands.replaceChildren();
  const commands = suggestCommands(snapshot, el.commandSearch.value);
  el.quickCount.textContent = `${commands.length} ${commands.length === 1 ? "command" : "commands"}`;
  if (!commands.length) {
    el.quickCommands.append(emptyChip("no commands"));
    return;
  }
  for (const command of commands) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = command.label;
    button.title = command.hint;
    button.addEventListener("click", () => runCommand(command.command));
    el.quickCommands.append(button);
  }
}

// ---- feed -----------------------------------------------------------------

function appendComponent(descriptor) {
  const eventId = descriptor.dataset.eventId;
  if (eventId !== "" && eventId !== null && eventId !== undefined) {
    if (seenEventIds.has(eventId)) {
      return;
    }
    seenEventIds.add(eventId);
  }
  const entry = document.createElement("article");
  entry.className = `log-entry ${descriptor.variant}`;
  entry.dataset.eventId = String(descriptor.dataset.eventId);
  entry.dataset.channel = descriptor.dataset.channel;
  entry.dataset.component = descriptor.dataset.type;
  appendEntry(entry, descriptor.label, descriptor.text);
}

function appendLine(variant, label, text) {
  const entry = document.createElement("article");
  entry.className = `log-entry ${variant}`;
  appendEntry(entry, label, text);
}

function appendEntry(entry, label, text) {
  const meta = document.createElement("span");
  meta.className = "log-meta";
  meta.textContent = label;
  const body = document.createElement("p");
  body.textContent = text;
  entry.append(meta, body);
  el.log.append(entry);
  el.log.scrollTop = el.log.scrollHeight;
}

// ---- small DOM helpers ----------------------------------------------------

function emptyChip(label) {
  const chip = document.createElement("span");
  chip.className = "chip empty";
  chip.textContent = label;
  return chip;
}

function actionCard(item) {
  const card = document.createElement("article");
  card.className = `entity-card ${item.kind}`;
  const main = document.createElement("div");
  main.className = "entity-main";
  const name = document.createElement("strong");
  name.className = "entity-name";
  name.textContent = item.name;
  const kind = document.createElement("span");
  kind.className = "entity-kind";
  kind.textContent = item.kind;
  main.append(name, kind);

  const actions = document.createElement("div");
  actions.className = "entity-actions";
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = "Go";
  button.title = item.command;
  button.addEventListener("click", () => runCommand(item.command));
  actions.append(button);

  card.append(main, actions);
  return card;
}

function setActiveMenuTab(tabId) {
  for (const tab of el.menuTabs) {
    const active = tab.dataset.tab === tabId;
    tab.classList.toggle("active", active);
    tab.setAttribute("aria-selected", String(active));
    tab.tabIndex = active ? 0 : -1;
  }
  for (const panel of el.tabPanels) {
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
  const currentIndex = el.menuTabs.indexOf(currentTab);
  const lastIndex = el.menuTabs.length - 1;
  let nextIndex = currentIndex;
  if (event.key === "Home") {
    nextIndex = 0;
  } else if (event.key === "End") {
    nextIndex = lastIndex;
  } else if (event.key === "ArrowLeft") {
    nextIndex = Math.max(0, currentIndex - 1);
  } else {
    nextIndex = Math.min(lastIndex, currentIndex + 1);
  }
  const nextTab = el.menuTabs[nextIndex];
  setActiveMenuTab(nextTab.dataset.tab);
  nextTab.focus();
}

function cycleHistory(delta) {
  if (!history.length) {
    return;
  }
  historyIndex = Math.min(history.length, Math.max(0, historyIndex + delta));
  el.input.value = history[historyIndex] ?? "";
  requestAnimationFrame(() => {
    el.input.selectionStart = el.input.value.length;
    el.input.selectionEnd = el.input.value.length;
  });
}
