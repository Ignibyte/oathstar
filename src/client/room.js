// Room display + navigation models.
//
// Pure: derives what the play surface shows from a `/state` GameSnapshot's room.
// The server sends a single `description` today; this module keeps the room's
// title, main (surface) description, full description, and an optional media hint
// separated so later server fields (shortDescription / fullDescription / media)
// drop in without reworking the UI (REQ-008). Exits become a directional Exit Pad
// model — navigation, kept distinct from Nearby contents.

export const DEFAULT_ROOM_DISPLAY_LIMIT = 160;

// Pad layout order: up / north on top, west·east flanking a centre gap, then
// south / down below — the directional pad in docs/ui-design.md "Room And Exits".
const CANONICAL_DIRECTIONS = [
  { dir: "up", label: "U" },
  { dir: "north", label: "N" },
  { dir: "west", label: "W" },
  { dir: "east", label: "E" },
  { dir: "south", label: "S" },
  { dir: "down", label: "D" },
];

function capitalize(word) {
  return word ? word.charAt(0).toUpperCase() + word.slice(1) : word;
}

function exitList(room) {
  return Object.entries(room.exits ?? {}).map(([direction, destinationId]) => ({
    direction,
    destinationId,
    label: capitalize(direction),
    command: direction,
  }));
}

// Summarize long text at a polished breakpoint. A full first sentence that ends
// within the limit is a clean summary (no ellipsis); otherwise cut at the last
// word boundary before the limit and mark it with a trailing ellipsis. Only ever
// called when the text already exceeds the limit (the caller guards length).
function summarize(text, limit) {
  const sentenceEnd = text.slice(0, limit + 1).search(/[.!?]\s/);
  if (sentenceEnd > 0) {
    return text.slice(0, sentenceEnd + 1);
  }
  const slice = text.slice(0, limit);
  const lastSpace = slice.lastIndexOf(" ");
  const cut = lastSpace > 0 ? slice.slice(0, lastSpace) : slice;
  return `${cut.trimEnd()}…`;
}

/**
 * Build the room display model for the current snapshot. The single server
 * `description` feeds both `main` (truncated for the surface) and `full` (the
 * modal); `shortDescription` / `fullDescription` / `mediaHint` are read when the
 * server later provides them.
 *
 * @param {object} snapshot the `/state` snapshot
 * @param {{limit?: number}} [opts] main-surface description budget (chars)
 * @returns {object} { id, title, main, full, truncated, mediaHint, exits }
 */
export function toRoomDisplay(snapshot, opts = {}) {
  const room = snapshot?.room ?? {};
  const limit =
    Number.isFinite(opts?.limit) && opts.limit > 0 ? opts.limit : DEFAULT_ROOM_DISPLAY_LIMIT;

  const full = room.fullDescription ?? room.description ?? room.shortDescription ?? "";
  let main;
  let truncated;
  if (typeof room.shortDescription === "string" && room.shortDescription.length > 0) {
    main = room.shortDescription;
    truncated = main !== full;
  } else if (full.length > limit) {
    main = summarize(full, limit);
    truncated = true;
  } else {
    main = full;
    truncated = false;
  }

  return {
    id: room.id ?? "",
    title: room.title ?? "",
    main,
    full,
    truncated,
    mediaHint: room.mediaHint ?? null,
    exits: exitList(room),
  };
}

/**
 * Build the directional Exit Pad model: the six canonical directions in pad
 * order, each marked available iff the room has that exit. Enabled controls send
 * the same canonical command (`north`, `up`, …) the text prompt would.
 *
 * @param {object} snapshot the `/state` snapshot
 * @returns {{directions: Array, availableCount: number}}
 */
export function toExitPad(snapshot) {
  const exits = snapshot?.room?.exits ?? {};
  const directions = CANONICAL_DIRECTIONS.map(({ dir, label }) => {
    const available = Object.prototype.hasOwnProperty.call(exits, dir);
    return {
      dir,
      label,
      command: dir,
      available,
      destinationId: available ? exits[dir] : null,
    };
  });
  return {
    directions,
    availableCount: directions.filter((entry) => entry.available).length,
  };
}
