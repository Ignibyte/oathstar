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
import { toHud, toMenuModel, toBattle } from "./client/snapshot.js";
import { toRoomDisplay, toExitPad } from "./client/room.js";
import { toMapModel, DEFAULT_MAP_CONFIG } from "./client/map.js";
import { canvasSize, toDrawPlan, mapAriaLabel } from "./client/canvas-map.js";
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
  battleModal: document.querySelector("#battle-modal"),
  battleModalTitle: document.querySelector("#battle-modal-title"),
  battleRound: document.querySelector("#battle-round"),
  battleLog: document.querySelector("#battle-log"),
  battleParticipants: document.querySelector("#battle-participants"),
  battleStatus: document.querySelector("#battle-status"),
  battleAttackButton: document.querySelector("#battle-attack-button"),
  battlePowerStrikeButton: document.querySelector("#battle-power-strike-button"),
  battleGuardButton: document.querySelector("#battle-guard-button"),
  battleFleeButton: document.querySelector("#battle-flee-button"),
  entityModal: document.querySelector("#entity-modal"),
  entityModalTitle: document.querySelector("#entity-modal-title"),
  entityModalKind: document.querySelector("#entity-modal-kind"),
  entityModalStatus: document.querySelector("#entity-modal-status"),
  entityModalStats: document.querySelector("#entity-modal-stats"),
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

boot();

function boot() {
  bindEvents();
  bindLogAutoscroll();
  setActiveMenuTab("nearby");
  refreshState();
  connectEvents();
  focusCommandInput();
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
    focusCommandInput();
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

  // The battle modal's Attack button issues the next combat round (ticket #22).
  el.battleAttackButton.addEventListener("click", () => runCommand("attack"));
  // The Flee button queues the between-pulse flee action (ticket #24); the next
  // combat pulse's skill window resolves it into a fled outcome.
  el.battleFleeButton.addEventListener("click", () => runCommand("flee"));
  // Direct battle verbs (ticket #25): the buttons send the exact verbs the
  // parser accepts; the next pulse's skill window resolves them.
  el.battlePowerStrikeButton.addEventListener("click", () => runCommand("power strike"));
  el.battleGuardButton.addEventListener("click", () => runCommand("guard"));

  // Clicking the entity-detail dialog backdrop closes it (ticket #23). The detail
  // dialog is a pure overlay — opening or closing it changes no game state.
  el.entityModal.addEventListener("click", (event) => {
    if (event.target === el.entityModal) {
      el.entityModal.close();
    }
  });
}

function bindLogAutoscroll() {
  if (!el.log || typeof MutationObserver !== "function") {
    return;
  }

  let scheduled = false;
  const scheduleScroll = () => {
    if (scheduled) {
      return;
    }
    scheduled = true;
    requestAnimationFrame(() => {
      scheduled = false;
      el.log.scrollTop = el.log.scrollHeight;
    });
  };

  const observer = new MutationObserver((mutations) => {
    if (
      mutations.some(
        (mutation) => mutation.type === "childList" && mutation.addedNodes.length > 0,
      )
    ) {
      scheduleScroll();
    }
  });

  observer.observe(el.log, { childList: true });
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
    // The event feed itself is now rendered server-side and patched into #log by
    // Datastar (/events/datastar, ticket #15). This JSON stream is kept only to
    // keep the HUD/map/panels in sync: refresh /state on state-affecting events.
    let parsed;
    try {
      parsed = parseEvent(JSON.parse(event.data));
    } catch (_err) {
      return;
    }
    if (
      parsed &&
      (parsed.type === "room_entered" ||
        parsed.type === "oath_sworn" ||
        parsed.type === "oath_fulfilled" ||
        parsed.type === "combat_started" ||
        parsed.type === "combat_ended" ||
        // A combat pulse resolved server-side without a command (ticket #24) —
        // refetch so the battle modal and HUD update live (REQ-003).
        parsed.type === "combat_pulse")
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
    focusCommandInput();
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
  renderBattle(snapshot);
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
  el.mapLabel.textContent =
    model.planes && model.planes.length > 1
      ? `${model.region || "Map"} · floor ${model.z}`
      : model.region || "Map";
  drawMapCanvas(el.map, model);
}

// Browser-only canvas seam: execute the pure draw-plan from canvas-map.js. Kept
// deliberately thin (no geometry/classification logic) because node/jsdom has no
// 2D context — all of that is unit-tested in canvas-map.js. Hi-DPI: the backing
// store is scaled by devicePixelRatio and the context scaled to match, so 1px
// strokes and the grid stay crisp on retina displays (ticket #16, REQ-005).
function drawMapCanvas(canvas, model) {
  if (!canvas || typeof canvas.getContext !== "function") {
    return;
  }
  const dpr = window.devicePixelRatio || 1;
  const size = canvasSize(model, dpr);
  canvas.width = size.backingWidth;
  canvas.height = size.backingHeight;
  canvas.style.width = `${size.cssWidth}px`;
  canvas.style.height = `${size.cssHeight}px`;
  canvas.setAttribute("aria-label", mapAriaLabel(model));

  const ctx = canvas.getContext("2d");
  if (!ctx) {
    return;
  }
  ctx.setTransform(size.dpr, 0, 0, size.dpr, 0, 0);
  ctx.clearRect(0, 0, size.cssWidth, size.cssHeight);
  if (!model.columns) {
    return;
  }

  const plan = toDrawPlan(model);
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  for (const op of plan.ops) {
    ctx.fillStyle = op.fill;
    ctx.fillRect(op.x, op.y, op.size, op.size);
    ctx.strokeStyle = op.stroke;
    // +0.5 keeps the 1px border on a device pixel boundary (crisp, not blurry).
    ctx.strokeRect(op.x + 0.5, op.y + 0.5, op.size - 1, op.size - 1);
    if (op.textColor && op.glyph) {
      ctx.fillStyle = op.textColor;
      ctx.font = `${plan.glyphFontPx}px ui-sans-serif, system-ui, sans-serif`;
      ctx.fillText(op.glyph, op.x + op.size / 2, op.y + op.size / 2, Math.max(1, op.size - 4));
    }
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
  for (const item of menu.pack.items) {
    el.inventory.append(packChip(item.name));
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

// The battle modal (ticket #22). Opens on a combat snapshot (REQ-008), renders
// the play-by-play (left) and participant state (right, REQ-009), and closes when
// combat resolves (REQ-010 — the compact summary remains in the server-rendered
// feed). The modal is a pure overlay over latestSnapshot; the Attack button (in
// bindEvents) drives the next round. showModal/close are guarded for jsdom.
function renderBattle(snapshot) {
  const battle = toBattle(snapshot);

  if (!battle.active) {
    if (el.battleModal.open && typeof el.battleModal.close === "function") {
      el.battleModal.close();
    }
    return;
  }

  const enemyName = battle.enemies.length === 1 ? battle.enemies[0].name : null;
  el.battleModalTitle.textContent = enemyName ? `Fighting ${enemyName}` : "Battle";
  el.battleRound.textContent = battle.round ? `Round ${battle.round}` : "";

  el.battleLog.replaceChildren();
  for (const line of battle.log) {
    const entry = document.createElement("li");
    entry.textContent = line;
    el.battleLog.append(entry);
  }

  el.battleParticipants.replaceChildren();
  for (const participant of battle.participants) {
    el.battleParticipants.append(combatantCard(participant));
  }

  // The queued between-pulse action (ticket #24): a quiet status line so a
  // queued flee is visible while it waits for the next pulse's skill window.
  // The label decision lives in the toBattle view-model; this is glue only.
  el.battleStatus.textContent = battle.queuedActionLabel ?? "";

  if (!el.battleModal.open && typeof el.battleModal.showModal === "function") {
    el.battleModal.showModal();
  }
}

function combatantCard(participant) {
  const card = document.createElement("article");
  card.className = `combatant-card ${participant.side}${participant.defeated ? " defeated" : ""}`;

  const head = document.createElement("div");
  head.className = "combatant-head";
  const name = document.createElement("strong");
  name.className = "combatant-name";
  name.textContent = participant.name;
  const hp = document.createElement("span");
  hp.className = "combatant-hp";
  hp.textContent = `${participant.hp}/${participant.maxHp}`;
  head.append(name, hp);

  const track = document.createElement("div");
  track.className = "combatant-hp-track";
  const fill = document.createElement("div");
  fill.className = "combatant-hp-fill";
  fill.style.width = `${participant.hpPct}%`;
  track.append(fill);

  card.append(head, track);
  return card;
}

// ---- feed -----------------------------------------------------------------

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

function packChip(label) {
  const chip = document.createElement("span");
  chip.className = "chip";
  chip.textContent = label;
  return chip;
}

function actionCard(item) {
  const card = document.createElement("article");
  // Ticket #23: server-authored hostile/attackable flags drive the styling hooks
  // (the client never infers them); the name row opens the detail dialog.
  card.className = `entity-card ${item.kind}${item.hostile ? " hostile" : ""}${
    item.attackable ? " attackable" : ""
  }`;

  // The name/kind row is a button that opens the entity detail dialog (ticket
  // #23) — inspection only, no command, no state change. The action buttons below
  // stay separate command-senders, so a button click never opens the dialog.
  const main = document.createElement("button");
  main.type = "button";
  main.className = "entity-main entity-inspect";
  main.title = `Inspect ${item.name}`;
  const name = document.createElement("strong");
  name.className = "entity-name";
  name.textContent = item.name;
  const kind = document.createElement("span");
  kind.className = "entity-kind";
  kind.textContent = item.detail ? `${item.kind} / ${item.detail}` : item.kind;
  main.append(name, kind);

  // A quiet hostile status chip (REQ-002): flags hostile/attackable state without
  // offering an enabled attack when combat is not allowed from here.
  if (item.combatStatus) {
    const status = document.createElement("span");
    status.className = `combat-status ${item.attackable ? "is-attackable" : "is-quiet"}`;
    status.textContent = item.combatStatus;
    main.append(status);
  }
  main.addEventListener("click", () => openEntityDetail(item.entityDetail));

  const actions = document.createElement("div");
  actions.className = "entity-actions";
  for (const action of item.actions ?? [{ label: "Look", command: item.command }]) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = action.label;
    button.title = action.hint ?? action.command;
    button.disabled = Boolean(action.disabled);
    if (action.variant) {
      button.classList.add(`action-${action.variant}`);
    }
    button.addEventListener("click", () => {
      if (!action.disabled) {
        runCommand(action.command);
      }
    });
    actions.append(button);
  }

  card.append(main, actions);
  return card;
}

// The generic entity detail dialog (ticket #23). Renders a server-authored detail
// view model (name / kind / hostile status / disclosed-or-unknown combat stats)
// into #entity-modal. A pure overlay over the snapshot the client already holds:
// opening it sends no command and mutates no game state (REQ-004). Disclosed stats
// show their values; hidden stats render as "unknown" (REQ-005). showModal is
// guarded for jsdom (which lacks <dialog>).
function openEntityDetail(detail) {
  if (!detail) {
    return;
  }
  el.entityModalTitle.textContent = detail.name;
  el.entityModalKind.textContent = detail.kind;
  el.entityModalStatus.textContent = detail.statusLabel ?? "";
  el.entityModalStatus.hidden = !detail.statusLabel;

  el.entityModalStats.replaceChildren();
  if (detail.stats.isCombatant) {
    const health = detail.stats.disclosed
      ? `${detail.stats.health}/${detail.stats.maxHealth}`
      : "unknown";
    const attack = detail.stats.disclosed ? String(detail.stats.attack) : "unknown";
    for (const [label, value] of [
      ["Health", health],
      ["Attack", attack],
    ]) {
      const row = document.createElement("div");
      row.className = "entity-stat";
      const key = document.createElement("span");
      key.className = "entity-stat-label";
      key.textContent = label;
      const val = document.createElement("span");
      val.className = "entity-stat-value";
      val.textContent = value;
      row.append(key, val);
      el.entityModalStats.append(row);
    }
    el.entityModalStats.hidden = false;
  } else {
    el.entityModalStats.hidden = true;
  }

  if (typeof el.entityModal.showModal === "function") {
    el.entityModal.showModal();
  }
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

function focusCommandInput() {
  try {
    el.input.focus({ preventScroll: true });
  } catch (_err) {
    el.input.focus();
  }
}
