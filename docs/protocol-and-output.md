# Protocol And Output

Oathstar should use a Datastar-first protocol for first-party UI, with JSON kept
for renderer data, diagnostics, tests, and alternate adapters.

The Rust core emits typed domain events. A Datastar presentation layer renders
those events into HTML fragments and SSE patches for first-party web surfaces.
The same domain events can also be exposed as structured JSON or plain text for
specialized clients and tooling.

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

**Implemented (ticket #22).** Combat v1 emits the per-strike play-by-play as
`LogMessage { component: CombatMessage }` plus typed `CombatStarted` /
`CombatEnded { outcome }` lifecycle markers, all on the `Combat` channel (the
finer `CombatHit` / `CombatMiss` granularity above is deferred). The
`oathstar-datastar` layer renders each into a `danger`-variant feed `<article>`;
the JSON snapshot carries the live encounter as `GameSnapshot.combat`
(`CombatSnapshot` — a side-tagged participant list) for the client's battle modal.

**Implemented (ticket #24).** The real-time pulse loop adds a typed
`CombatPulse { round }` marker on the `Combat` channel, emitted once per due
pulse ahead of its exchange events. It carries no feed text and the Datastar
layer renders nothing for it (like `Tick`) — the JSON client uses it as the
refresh trigger that keeps the battle modal live without a command (state still
travels via `/state`, the Decision 034 carve-out). `CombatOutcome` gains
`fled`, and `CombatSnapshot` gains the additive camelCase `queuedAction`
(omitted when nothing is queued) surfacing the player's queued between-pulse
action — `"flee"` (#24), `"guard"` and `"power_strike"` (#25); the field is an
open string, so future verbs ride it without protocol changes.

**Implemented (ticket #27).** Scoped announcements add a typed
`Announcement { severity, text }` event on the `Region` channel, with
`AnnouncementSeverity` (`notice` / `warning` / `alarm`) driving the feed
presentation. Delivery is decided at emission — the engine matches the
announcement's scope (`world` / `region` / `subregion` / `room` / `radius`,
the radius reusing the spatial-awareness plane + Chebyshev model) against the
player's current location and emits the event only when it is received, so
nothing scope-filtered ever serializes and a client never decides receipt.
Announcement content is authored (v1 carrier:
`OathDefinition.fulfillment_announcements`, validated at construction); the
mechanism is the engine's. See `decisions.md` Decision 045.

## Representations

JSON representation, reserved for maps/canvas data, diagnostics, tests, and
adapter clients:

```json
{
  "type": "combat.hit",
  "actorId": "player",
  "targetId": "shade_01",
  "damage": 7,
  "message": "You strike the shade for 7."
}
```

HTML/Datastar representation, the first-party UI default:

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

Implemented endpoints (ticket #15):

- `GET /events/datastar` — first-party Datastar SSE: `datastar-patch-elements`
  patches that append server-rendered HTML fragments to the feed (`#log`)
- `GET /events/json` (and `GET /events`) — JSON `game_event` SSE for diagnostics/tests/adapters
- `POST /command` — server-authoritative
- `GET /state` — JSON `GameSnapshot`; map renderer data rides here as `map`
  (there is no separate `/map` route yet — Decision 035 keeps map payloads JSON)

Aspirational: `GET /components/:component` and dedicated JSON renderer-data endpoints.

The exact paths can change. The important point is that Datastar/HTML rendering
is the first-party UI path, while JSON stays available from the same domain model
where structured renderer or adapter data is the better contract.

## Design Guardrails

- Domain events stay renderer-agnostic.
- Datastar/HTML fragments are the first-party UI adapter, not the core state model.
- JSON remains available for maps/canvas/sprites, alternate clients, debugging, DM
  tools, and tests.
- Components should be interactive where useful, but not visually overwhelming.
- The player should still be able to read the game like a MUD.

## Implementation Status

As of ticket #30 the implemented `GameEventKind` set is `LogMessage` (carrying
an `OutputComponent`), `Tick`, `RoomEntered`, `OathSworn`, `OathFulfilled`,
`CombatStarted`, `CombatEnded`, `CombatPulse` (ticket #24), `Announcement`
(ticket #27), and `LevelUp` (ticket #30); the rest of the event catalog above
is still aspirational. Channels and `OutputComponent`s are defined in
`oathstar-protocol`.

**Wire-format note (clients):** a `GameEvent` serializes camelCase (`eventId`,
`tick`, `channel`) with its kind `flatten`-ed in under a snake_case `type` tag
(`oath_sworn`, `room_entered`); the kind's payload fields stay **snake_case**
(`oath_id`, `room_id`) because serde `rename_all` does not cross `flatten`.
View/snapshot structs (`GameSnapshot`, `OathSnapshot`) are **camelCase**
(`oathId`). So `/events` payloads use snake_case keys and `/state` snapshots use
camelCase keys — see `docs/decisions.md` Decision 031.

**Transport (ticket #15).** The first-party event feed is delivered as Datastar
`datastar-patch-elements` SSE from `GET /events/datastar`, rendered by the
`oathstar-datastar` crate (every server-provided string HTML-escaped). The JSON
`/events`, `/events/json`, and `/state` endpoints are preserved for renderer data,
tests, and adapters (Decisions 033/034). The vendored Datastar runtime lives at
`public/vendor/datastar/datastar.js`.
