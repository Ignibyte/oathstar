# UI Design

Oathstar's first playable UI should be map-forward, text-rich, browser-first, and
built around Datastar plus SSE HTML fragments.

The current prototype is a design reference, not the final client architecture.
The production client should keep the same player-facing shape while rendering
from the Rust server through REST, SSE, and Datastar/HTML fragments. Tauri can
package the web surfaces later, but the player client should remain a pure client
that connects to a compatible server.

## Design Reference

Current mockup artifacts:

- `artifacts/oathstar-desktop.png`
- `artifacts/oathstar-desktop-viewport.png`
- `artifacts/oathstar-mobile.png`

Prototype files that capture the latest layout direction:

- `index.html`
- `styles.css`
- `src/app.js`
- `src/engine.js`

When implementation begins, treat these as visual and interaction references.
Do not let the prototype become the architecture source of truth.

## Layout Contract

The first playable screen should open directly into the game.

Primary regions:

- Top HUD: health and focus bars, current room/status, and lightweight turn or
  action context.
- Map/stage: the most prominent region, showing the current room/grid state.
- Output feed: typed narrative, combat, oath, dialogue, loot, and system
  components underneath the map, with its own scrollbar.
- Character menu: a top-right tab panel aligned with the map/stage height.
- Intent panel: a bottom-right command panel aligned with the output feed
  height.
- Command entry: a persistent text command input for MUD-style play.

The application shell should use the full available browser/Tauri viewport
without forcing ordinary page scrolling. Desktop layouts should fill the
available width and height, leaving panel interiors responsible for overflow.
The output feed should stay bounded: new events appear at the bottom, older
events scroll upward inside the feed, and the command area remains reachable.

The map and output feed should feel like co-equal pillars at first, with the map
slightly leading the composition. The design should leave room for the map to
become the primary play surface later if the game evolves toward a denser
top-down representation.

## Menu Tabs

The character menu should start with these tabs:

- Nearby
- Oaths
- Gear
- Pack

The tabs should be componentized so future modules can add or replace panels
without rewriting the whole client. For example, a crafting module might add a
`Craft` panel, while a DM mode might add a `Director` panel.

Expected early content:

- Nearby: actors, items, fixtures, and quick actions such as `talk`, `look`, and
  `examine`.
- Oaths: current oath state, witness, region impact, progress, and break/keep
  implications.
- Gear: equipment slots such as main hand, off hand, body, left earring, right
  earring, and trinket.
- Pack: inventory stacks, quick use/equip/drop actions, and item inspection.

Exits are navigation, not Nearby content. Nearby should not render exits as
cards once the Exit Pad exists. If the current snapshot does not yet expose room
actors/items/fixtures, Nearby should show an honest empty state rather than
inventing content.

## Room And Exits

The room surface should balance immersion and scanability.

Rooms should support:

- Title.
- Short or summarized description.
- Long description.
- First-visit or state-specific description later.
- Optional media hint later, such as a static image, animation, or ambient scene.

The normal play surface should show the room title and enough description to
orient the player without crowding the map, feed, and command controls. Long room
text should be truncated or summarized at a polished breakpoint, with a focused
action such as `View room` or `Expand` that opens the full room view.

The full room view should be a focused modal/dialog. It can show the title, the
full long description, exits, known contents, region/subregion context, and a
future visual asset. The visual slot should be optional: a room can stay purely
textual, show a static generated/curated image, or eventually show an animated
ambient view. A warrior's grave, shrine, boss chamber, or strange outpost should
be able to feel larger than one paragraph on the main screen.

Exit controls should move out of Nearby and stay near the command hand, not in
the room description area. The preferred control is a compact directional pad
anchored in the lower play surface near the event feed/command input, above or
beside the Enter control:

```text
    U
    N
W       E
    S
    D
```

Only available exits are active. Missing exits are disabled or visually quiet.
Clicking a direction sends the same canonical text command (`north`, `up`,
`down`) that the command prompt would send. The room text can still include a
plain `Exits: east, north, up` line for MUD readability.

Movement controls should not also appear as Intent command suggestions once the
Exit Pad exists. Keeping movement in one dedicated control prevents noisy
redundancy while preserving free-text commands for players who prefer typing.

## Intent Panel

The intent panel is the player's command helper.

It should sit on the bottom-right and match the output feed height. It should
include:

- Command search/filter.
- Suggested commands for the current context.
- Quick command buttons.
- Recent or pinned intents later.

The command helper should never replace free text commands. It should teach the
available command vocabulary while keeping the classic MUD-style prompt alive.
Movement commands are excluded from Intent when the Exit Pad is visible.

## Componentized Output

The output feed should not be a textarea.

The Rust core emits typed events. The client renders those events into a catalog
of interactive components:

- Narrative blocks
- Dialogue blocks
- Room headers
- Combat messages
- Oath cards
- Skill and progression notices
- Loot and item cards
- Region standing changes
- Map updates
- System/debug messages

Interactive components can send commands back to the server, such as
`look <mob>`, `talk <npc>`, `equip <item>`, or `swear <oath>`.

Longer event sequences should eventually be grouped and collapsible. Combat,
shop transactions, important dialogue, rituals, crafting, and oath scenes can
stream detailed events while active, then collapse to a summary when complete.
For example, a fight can show red combat events during the exchange and collapse
to a result card such as victory, wounds, loot, and skill changes.

The feed should not grow the application vertically. It is a bounded event
surface with an internal scrollbar, newest entries at the bottom, and enough
space reserved for the command controls. Button clicks inside the feed, Exit Pad,
Intent panel, and modal surfaces should not cause viewport jumps or unexpected
page scrolling.

Interaction surfaces should follow this policy:

- Inline feed/card: small flavor dialogue, examine/look output, minor pickups,
  simple confirmations, and low-stakes NPC comments.
- Focused modal/view: combat, shops, major quest or oath scenes, irreversible
  choices, important dialogue trees, and any interaction that benefits from a
  larger scene.
- Both: a modal interaction still emits feed events, and closing it should leave
  behind a concise summary card.

## Map Direction

The first map can be an HTML/CSS or Datastar-rendered grid, but the data model
must remain renderer-agnostic. When canvas begins, the target is an actual
square-tile grid, defaulting to 32x32 cells, rather than freeform room rectangles.

The backend should send structured map JSON and typed map events. It should not
send canvas-specific drawing instructions.

The renderer should keep these future paths open:

- Text or ASCII grid rendering.
- Configurable tile sizes such as 8px, 16px, 32px, or larger accessibility
  sizes.
- Canvas rendering.
- Sprite tiles.
- Collision/passability visualization.
- Entity and item markers.
- Region, subregion, fog-of-war, DM, and debug overlays.

The prototype currently carries a tile-size hook with a 32px sprite tile
placeholder. Production code should preserve that intent as an explicit map
renderer configuration rather than hardcoding layout assumptions into world
state.

Implemented (ticket #16): the DOM tile grid was replaced by a first-party
`<canvas id="map">` renderer — a pure model in `src/client/canvas-map.js`
(`canvasSize`/`cellKind`/`toDrawPlan`/`mapAriaLabel`) drawn by a thin
`drawMapCanvas` seam in `client-app.js`. 32×32 default (configurable via
`mapRenderConfig`), current z-plane only, Hi-DPI via `devicePixelRatio`, no
game-engine dependency; the server map stays renderer-agnostic JSON.

## Web, Tauri, And Datastar Target

The preferred implementation target is:

- Rust core/server remains the game authority.
- Player Client and Host Manager are separate browser-first web surfaces.
- Tauri is packaging and native lifecycle later, not gameplay authority.
- Datastar/HTML fragments render live first-party UI panels.
- REST handles commands and snapshots.
- SSE streams event and UI updates.
- JSON remains available for maps/canvas/sprites, tests, alternate clients, and
  debug tooling.

The Tauri client should feel like a native game shell, but it should not own the
rules. The UI sends commands, receives state/events, and renders the result. A
Tauri launcher may manage a local server sidecar, but the Player Client itself
should not become the server manager.

## Guardrails

- Keep the player in the game immediately; avoid a marketing or landing page.
- Preserve MUD readability even as the map becomes more important.
- Keep typed output components structured and interactive.
- Make panels modular enough for future systems and swappable worlds.
- Keep the map renderer configurable so canvas/sprite work can arrive later.
- Keep the client compatible with Datastar and SSE instead of drifting into a
  large SPA framework by accident.

## Implementation Status

As of ticket #12 the production client is implemented as a framework-free,
server-authoritative hypermedia shell (see `docs/decisions.md` Decision 032):

- `index.html` + `styles.css` carry the map-forward layout; `src/client-app.js`
  is the browser entry that drives `/command` (POST), `/events` (SSE), and
  `/state` (snapshot) against the local `oathstar-server`.
- Render/logic lives in DOM-free, tested modules under `src/client/` (`wire`,
  `components`, `snapshot`, `map`, `intent`); the prototype `src/app.js` /
  `src/engine.js` remain as the in-browser visual/interaction reference.
- The server delivers the opening scene on connect by replaying
  `Engine::begin()` onto each new `/events` subscription — no `look` required.
- The minimap renders the current room's z-plane (the beginner tower stacks
  rooms at the same `(x, y)` on different floors).
- Ticket #13 refined the play surface: Nearby lists room contents only (honest
  empty state, no exits); a directional Exit Pad (`src/client/room.js`
  `toExitPad`) sends the canonical movement commands; long room descriptions
  truncate on the main surface (`toRoomDisplay`) with a focused full-room
  `<dialog>` (title, full description, exits, reserved media area).
- Ticket #14 moved the Exit Pad into the command bar (beside the input), removed
  movement commands from Intent (the Exit Pad is the single movement control;
  typed movement still works), and contained the shell to the viewport (`100dvh`,
  no page scroll on desktop) with a bounded, internally-scrolling event feed.
- Decision 033 split the browser-first Player Client from the Host Manager, with
  Tauri as later packaging/lifecycle.
- Decision 034 made Datastar/SSE HTML the first-party UI transport default; JSON
  stays for map/canvas renderer data, diagnostics, tests, and adapters.
- Decision 035 made the future canvas map a first-party 32x32 square-grid renderer
  instead of a full game-engine dependency.
- Run it locally: `npm run server:dev` (Rust server) + `npm run dev` (vite,
  proxied to the server).
- Ticket #15 landed the Datastar transport: the vendored runtime
  (`public/vendor/datastar/datastar.js`, pinned v1.0.2) loads via the build, and the
  event feed (`#log`) is now server-rendered — `data-init` opens the
  `GET /events/datastar` SSE stream and the `oathstar-datastar` crate emits
  `datastar-patch-elements` patches (every server string HTML-escaped). The JSON
  `/state` + `/events` + `/events/json` endpoints are unchanged; the rest of the
  client stays hand-rolled for now. Tauri server lifecycle remains a follow-up ticket.
- Ticket #16 replaced the DOM tile grid with a first-party `<canvas id="map">`
  renderer: a pure, tested model in `src/client/canvas-map.js`
  (`canvasSize`/`cellKind`/`toDrawPlan`/`mapAriaLabel`) drawn by a thin
  `drawMapCanvas` seam in `client-app.js`. 32×32 default (configurable via
  `mapRenderConfig`), current z-plane only, Hi-DPI via `devicePixelRatio`, the room
  glyph drawn on-tile + room title via `aria-label`, no game-engine dependency; the
  server map stays renderer-agnostic JSON.
