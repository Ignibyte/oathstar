// Componentized output catalog: a parsed client event -> a render descriptor.
//
// The output feed is not a textarea (docs/ui-design.md). Each event becomes a
// structured descriptor carrying a visual variant (reusing the styles.css
// `log-entry` classes), a meta label, the text, and `data-*` attributes for
// later interactivity. The DOM layer turns one descriptor into one article.

const CHANNEL_VARIANT = {
  room: "room",
  oath: "success",
  combat: "danger",
  narrative: "dialogue",
  region: "system",
  loot: "success",
  skill: "system",
  inventory: "system",
  equipment: "system",
  system: "system",
  dm: "dialogue",
  debug: "system",
};

const CHANNEL_LABEL = {
  room: "Room",
  oath: "Oath",
  combat: "Combat",
  narrative: "Narrative",
  region: "Region",
  loot: "Loot",
  skill: "Skill",
  inventory: "Pack",
  equipment: "Gear",
  system: "System",
  dm: "Storyteller",
  debug: "Debug",
};

// OutputComponent (on log_message) overrides the channel mapping so a room
// header always reads as a room block, an oath card as an oath block, etc.
const COMPONENT_VARIANT = {
  room_header: "room",
  narrative_message: "dialogue",
  system_message: "system",
  combat_message: "danger",
  oath_card: "success",
  region_standing_card: "system",
  entity_chip: "system",
  item_card: "success",
  map_patch: "system",
};

const COMPONENT_LABEL = {
  room_header: "Room",
  narrative_message: "Narrative",
  system_message: "System",
  combat_message: "Combat",
  oath_card: "Oath",
  region_standing_card: "Region",
  entity_chip: "Nearby",
  item_card: "Item",
  map_patch: "Map",
};

function textFor(event) {
  switch (event.type) {
    case "log_message":
      return event.text ?? "";
    case "room_entered":
      return event.title ? `You enter ${event.title}.` : "You enter a new room.";
    case "oath_sworn":
      return event.title ? `Oath sworn: ${event.title}.` : "You swear an oath.";
    case "oath_fulfilled":
      return "Your oath is fulfilled.";
    default:
      return event.text ?? "";
  }
}

/**
 * Build a componentized output descriptor from a parsed client event.
 * Returns `null` for events that should not appear in the feed (e.g. ticks).
 *
 * @param {object|null} event a parsed event from {@link module:wire.parseEvent}
 * @returns {object|null}
 */
export function toComponent(event) {
  if (!event || typeof event.type !== "string" || event.type === "tick") {
    return null;
  }

  const channel = event.channel ?? "system";
  let variant = CHANNEL_VARIANT[channel] ?? "system";
  let label = CHANNEL_LABEL[channel] ?? "System";

  if (event.type === "log_message" && event.component && COMPONENT_VARIANT[event.component]) {
    variant = COMPONENT_VARIANT[event.component];
    label = COMPONENT_LABEL[event.component];
  }

  return {
    component: event.type,
    channel,
    variant,
    label,
    text: textFor(event),
    dataset: {
      eventId: event.id ?? "",
      channel,
      type: event.type,
    },
  };
}
