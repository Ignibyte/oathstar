# Protocol And Output

Oathstar should use a hybrid protocol.

The Rust core emits typed domain events. The server can expose those events as structured JSON, rendered Datastar/HTML fragments, or plain text for simple clients.

## Core Direction

Domain events are the source of truth.

Examples:

- `LogMessage`
- `RoomEntered`
- `CombatHit`
- `CombatMiss`
- `SkillImproved`
- `ItemEquipped`
- `OathSworn`
- `OathFulfilled`
- `OathBroken`
- `RegionStandingChanged`
- `MapUpdated`
- `InventoryUpdated`
- `SequenceStarted`

Those events can be rendered differently for different clients.

## Representations

JSON representation:

```json
{
  "type": "combat.hit",
  "actorId": "player",
  "targetId": "shade_01",
  "damage": 7,
  "message": "You strike the shade for 7."
}
```

HTML/Datastar representation:

```html
<article class="message message-combat" data-event-id="evt_123">
  <strong>You strike</strong>
  <span>the shade for 7.</span>
</article>
```

Plain text representation:

```text
You strike the shade for 7.
```

## Componentized Output

The game output should not be a single textarea.

Output should be rendered as a catalog of typed components.

Possible components:

- Narrative message
- Combat message
- Loot message
- Skill improvement message
- Oath card
- Region standing card
- Room header
- Entity chip
- Item card
- Combat action prompt
- Boss phase banner
- DM narration block
- Map patch
- Inventory patch
- Equipment patch
- Quest/oath update

This lets output become interactive.

Examples:

- Click an entity chip to `look <entity>`.
- Click an item card to inspect/equip/use.
- Click an oath card to view promise, witness, keep/break rules.
- Click a region standing card to see why the region likes or dislikes you.
- Click a combat action prompt to send `spare`, `flee`, or `bind`.

## Event Channels

Events can be grouped by channel.

Possible channels:

- `narrative`
- `room`
- `combat`
- `loot`
- `skill`
- `oath`
- `region`
- `inventory`
- `equipment`
- `system`
- `dm`
- `debug`

Channels help clients render, filter, style, and persist output.

## API Shape

Potential endpoints:

- `GET /events/json`
- `GET /events/html`
- `POST /command`
- `GET /state`
- `GET /components/:component`

The exact paths can change. The important point is that JSON and HTML rendering are both supported from the same domain events.

## Design Guardrails

- Domain events stay renderer-agnostic.
- Datastar/HTML fragments are render adapters, not the core state model.
- JSON remains available for alternate clients, debugging, DM tools, and tests.
- Components should be interactive where useful, but not visually overwhelming.
- The player should still be able to read the game like a MUD.

## Implementation Status (v1)

As of ticket #7 the implemented `GameEventKind` set is `LogMessage` (carrying an
`OutputComponent`), `Tick`, `RoomEntered`, `OathSworn`, and `OathFulfilled`; the
rest of the event catalog above is still aspirational. Channels and
`OutputComponent`s are defined in `oathstar-protocol`.

**Wire-format note (clients):** a `GameEvent` serializes camelCase (`eventId`,
`tick`, `channel`) with its kind `flatten`-ed in under a snake_case `type` tag
(`oath_sworn`, `room_entered`); the kind's payload fields stay **snake_case**
(`oath_id`, `room_id`) because serde `rename_all` does not cross `flatten`.
View/snapshot structs (`GameSnapshot`, `OathSnapshot`) are **camelCase**
(`oathId`). So `/events` payloads use snake_case keys and `/state` snapshots use
camelCase keys — see `docs/decisions.md` Decision 031.
