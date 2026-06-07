# UI Design

Oathstar's first playable UI should be map-forward, text-rich, and built for
Tauri plus Datastar.

The current prototype is a design reference, not the final client architecture.
The production client should keep the same player-facing shape while rendering
from the Rust server through REST, SSE, and Datastar/HTML fragments.

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
  components underneath the map.
- Character menu: a top-right tab panel aligned with the map/stage height.
- Intent panel: a bottom-right command panel aligned with the output feed
  height.
- Command entry: a persistent text command input for MUD-style play.

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

- Nearby: actors, items, exits, and quick actions such as `talk`, `look`, and
  `examine`.
- Oaths: current oath state, witness, region impact, progress, and break/keep
  implications.
- Gear: equipment slots such as main hand, off hand, body, left earring, right
  earring, and trinket.
- Pack: inventory stacks, quick use/equip/drop actions, and item inspection.

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

## Map Direction

The first map can be an HTML/CSS or Datastar-rendered grid, but the data model
must remain renderer-agnostic.

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

## Tauri And Datastar Target

The preferred implementation target is:

- Rust core/server remains the game authority.
- Tauri is the desktop shell and local server manager.
- Datastar/HTML fragments render live UI panels.
- REST handles commands and snapshots.
- SSE streams event and UI updates.
- JSON remains available for tests, alternate clients, and debug tooling.

The Tauri client should feel like a native game shell, but it should not own the
rules. The UI sends commands, receives state/events, and renders the result.

## Guardrails

- Keep the player in the game immediately; avoid a marketing or landing page.
- Preserve MUD readability even as the map becomes more important.
- Keep typed output components structured and interactive.
- Make panels modular enough for future systems and swappable worlds.
- Keep the map renderer configurable so canvas/sprite work can arrive later.
- Keep the client compatible with Datastar and SSE instead of drifting into a
  large SPA framework by accident.
