---
title: TICKET-12-build-tauri-datastar-map-forward-ui-shell
status: open
ticket:
ticket_number: 12
type: feature
created: 2026-06-07
intake: docs/planning/intake/INTAKE-beginner-slice-ui-startup.md
pipeline_spec:
---

# TICKET-12-build-tauri-datastar-map-forward-ui-shell

## Summary

Build the first playable Tauri + Datastar UI shell for the beginner vertical
slice, using the map-forward design direction as the implementation reference.

## Why

Ticket #7 proved the beginner slice in the Rust authority path, but players
still need a real client that starts the game, streams typed events, and renders
the first room without requiring an initial command. This ticket turns the UI
mockup into a production-direction shell while preserving the server-authoritative
Tauri + Datastar architecture.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a player starts a new local session, the server shall deliver the start room's opening scene without requiring the player to submit `look` first. | server integration test |
| REQ-002 | When the player submits text through the client command prompt, the Tauri/Datastar client shall POST the command to `/command` and render the resulting typed events. | UI test / API smoke |
| REQ-003 | While the client is connected, it shall consume `/events` SSE and render typed events into componentized output rather than a textarea. | UI test / manual smoke |
| REQ-004 | When the main play screen renders, it shall follow the map-forward layout documented in `docs/ui-design.md`, including top health/focus HUD, prominent map/stage, typed output underneath, top-right menu, and bottom-right Intent panel. | screenshot smoke / review |
| REQ-005 | When contextual actions are available, the client shall expose a tabbed character menu with at least `Nearby`, `Oaths`, `Gear`, and `Pack` panels. | UI test / manual smoke |
| REQ-006 | When command suggestions are available, the client shall expose an Intent panel with search/filter and quick command buttons while preserving free text command entry. | UI test / manual smoke |
| REQ-007 | When map state is rendered, the map layer shall keep tile size and render mode configurable enough for a later canvas or sprite renderer without changing the server's map state shape. | code review / UI test |
| REQ-008 | When implementing the shell, the frontend shall remain Tauri + Datastar/HTML-fragment oriented and shall not introduce a React, Vue, Svelte, or similar SPA framework without a separate architecture decision. | code review |
| REQ-009 | When the ticket is complete, the repository fast gate shall pass and any new UI behavior shall have targeted coverage or documented smoke verification. | `./bin/gate.sh --fast` |

## Scope

- In: Tauri shell hookup or Tauri-ready client path; Datastar/SSE rendering for
  `/events`; command submission through `/command`; `/state` snapshot rendering
  where needed; opening-scene delivery; typed output components; map-forward
  layout; tabbed character menu; Intent command search/helper; configurable map
  tile-size/render-mode hook.
- Out: full inventory/equipment mechanics; save/load UI; production combat UI;
  production canvas/sprite renderer; full accessibility pass; multiplayer or DM
  panels; replacing the Rust server/core boundaries.

## Notes

- Forge ticket:
- Related docs: `docs/ui-design.md`, `docs/technical-architecture.md`,
  `docs/protocol-and-output.md`, `docs/map-system.md`, `docs/decisions.md`
- Design artifacts: `artifacts/oathstar-desktop.png`,
  `artifacts/oathstar-desktop-viewport.png`, `artifacts/oathstar-mobile.png`
- Prototype reference files: `index.html`, `styles.css`, `src/app.js`,
  `src/engine.js`
- Promoted from intake: `docs/planning/intake/INTAKE-beginner-slice-ui-startup.md`
- Active pipeline:
