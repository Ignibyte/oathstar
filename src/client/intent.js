// Intent panel: the command helper. Provides a base command vocabulary plus
// context-aware suggestions and a search filter, while the free-text command
// form stays the primary input (REQ-006). Pure: derives from a snapshot, never
// touches the DOM.

export const COMMAND_VOCAB = Object.freeze([
  { id: "look", label: "look", command: "look", hint: "Describe the current room." },
  { id: "swear", label: "swear", command: "swear", hint: "Swear the module's oath." },
  { id: "confront", label: "confront", command: "confront", hint: "Confront the oath's endpoint." },
  { id: "help", label: "help", command: "help", hint: "List available commands." },
  { id: "map", label: "map", command: "map", hint: "Show the known map." },
  { id: "inventory", label: "inventory", command: "inventory", hint: "Show your pack." },
  { id: "stats", label: "stats", command: "stats", hint: "Show your character." },
]);

const DIRECTION_HINTS = {
  north: "Travel north.",
  south: "Travel south.",
  east: "Travel east.",
  west: "Travel west.",
  up: "Travel up.",
  down: "Travel down.",
};

function movementCommands(snapshot) {
  const exits = snapshot?.room?.exits ?? {};
  return Object.keys(exits).map((direction) => ({
    id: `move-${direction}`,
    label: direction,
    command: direction,
    hint: DIRECTION_HINTS[direction] ?? `Travel ${direction}.`,
  }));
}

function contextualCommands(snapshot) {
  const commands = [];
  const oath = snapshot?.oath;
  if (!oath) {
    commands.push({ id: "ctx-swear", label: "swear", command: "swear", hint: "An oath is available." });
  } else if (oath.status === "sworn") {
    commands.push({ id: "ctx-confront", label: "confront", command: "confront", hint: "Fulfill your sworn oath." });
  }
  return commands.concat(movementCommands(snapshot));
}

/**
 * Ordered, de-duplicated suggestions for the current snapshot, optionally
 * filtered by a search query (substring over label + command). Free-text entry
 * is unaffected — this only powers the helper buttons.
 *
 * @param {object} snapshot the `/state` snapshot
 * @param {string} [query] search filter
 * @returns {Array<{id:string,label:string,command:string,hint:string}>}
 */
export function suggestCommands(snapshot, query = "") {
  const seen = new Set();
  const ordered = [];
  for (const command of [...contextualCommands(snapshot), ...COMMAND_VOCAB]) {
    if (seen.has(command.command)) {
      continue;
    }
    seen.add(command.command);
    ordered.push(command);
  }

  const needle = String(query ?? "").trim().toLowerCase();
  if (!needle) {
    return ordered;
  }
  return ordered.filter(
    (command) =>
      command.label.toLowerCase().includes(needle) ||
      command.command.toLowerCase().includes(needle),
  );
}
