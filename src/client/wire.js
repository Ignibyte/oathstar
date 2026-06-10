// Wire-format adapter for the `/events` SSE stream.
//
// Per docs/decisions.md Decision 031, a GameEvent serializes with a camelCase
// envelope (`eventId`, `tick`, `channel`) and its kind flattened under a
// snake_case `type` tag whose payload fields stay snake_case (`room_id`,
// `oath_id`). This module normalizes one raw event into a camelCase client
// event so the rest of the client never has to know the wire casing.

/**
 * Normalize a raw `/events` GameEvent object into a client event.
 * Returns `null` for input that is not an object carrying a string `type`.
 *
 * @param {unknown} raw parsed JSON from one SSE `game_event` payload
 * @returns {object|null}
 */
export function parseEvent(raw) {
  if (!raw || typeof raw !== "object" || typeof raw.type !== "string") {
    return null;
  }

  const base = {
    id: raw.eventId ?? null,
    tick: raw.tick ?? 0,
    channel: raw.channel ?? "system",
    type: raw.type,
  };

  switch (raw.type) {
    case "log_message":
      return {
        ...base,
        component: raw.component ?? "narrative_message",
        text: raw.text ?? "",
      };
    case "room_entered":
      return { ...base, roomId: raw.room_id ?? null, title: raw.title ?? "" };
    case "oath_sworn":
      return { ...base, oathId: raw.oath_id ?? null, title: raw.title ?? "" };
    case "oath_fulfilled":
      return { ...base, oathId: raw.oath_id ?? null };
    case "level_up":
      // Ticket #30: keep the payload so textFor renders the exact line —
      // the default arm would silently strip it (the live-fallback trap).
      return { ...base, level: raw.level ?? null, maxHp: raw.max_hp ?? null };
    case "tick":
      return { ...base, value: raw.value ?? 0 };
    default:
      return base;
  }
}
