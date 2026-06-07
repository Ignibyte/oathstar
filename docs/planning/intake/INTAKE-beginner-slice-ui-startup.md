---
title: INTAKE-beginner-slice-ui-startup
status: candidate
created: 2026-06-06
ticket: a063dadb-6a68-4380-80b7-21c55966aead
pipeline_spec:
---

# INTAKE-beginner-slice-ui-startup

## Problem / Opportunity

Ticket #7 delivered the beginner vertical slice in the Rust authority path (oath
lifecycle, boss resolution, typed events) but it is **backend-only** — nothing
renders it yet. There is also a concrete **startup gap**: `Engine::begin()`
produces the opening room scene and is unit-tested, but the **server never calls
it**. It neither auto-broadcasts the opening scene on a new `/events`
subscription nor exposes a `begin`/opening endpoint, so a client that connects
today sees nothing until it sends its first command (e.g. `look`). This must be
wired before or with the UI so the player gets an opening scene on connect.

## Proposed Outcome

A player can start a beginner game, immediately see the opening room, and play
the full slice (swear → route → confront) through a rendered client:

- The server delivers the opening scene on session start — a `begin` endpoint
  and/or the `Engine::begin()` events seeded onto a new SSE subscription.
- A Tauri + Datastar front-end drives `POST /command` and renders the `/events`
  SSE stream into componentized output (room header, narrative, oath card,
  dialogue, combat, map), using `/state` for snapshots.
- The first client follows the map-forward layout captured in
  `docs/ui-design.md`: top health/focus HUD, prominent map/stage, typed output
  underneath, top-right tabbed character menu, and bottom-right Intent command
  helper.

## Candidate EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a client starts a new session, the server shall deliver the start room's opening scene (the `Engine::begin()` events) without requiring a prior command. | server integration test |
| REQ-002 | When the player submits a command in the UI, the client shall POST it to `/command` and render the returned/streamed typed events. | UI test / manual smoke |
| REQ-003 | While connected, the client shall render the `/events` SSE stream into the typed output components. | UI test / manual smoke |
| REQ-004 | When the client renders the main play screen, it shall follow the map-forward layout documented in `docs/ui-design.md`. | UI test / screenshot smoke |
| REQ-005 | When the player needs contextual actions, the client shall provide a tabbed character menu (`Nearby`, `Oaths`, `Gear`, `Pack`) and an Intent command search/helper panel. | UI test / manual smoke |
| REQ-006 | When map state is rendered, the client shall keep tile size and render mode configurable so the first HTML/Datastar grid can later become a canvas or sprite renderer without changing server state shape. | code review / UI test |

## Scope Notes

- In: server opening-scene delivery (begin endpoint or initial-events-on-subscribe);
  a Tauri + Datastar client that drives `/command` and renders `/events` + `/state`;
  honor the snake-in-events / camel-in-snapshots wire split
  (`docs/decisions.md` Decision 031); map-forward UI shell and typed event
  components.
- Out: full combat, skills, region standing, save/load UI, production sprite or
  canvas renderer — separate tickets.

## Promotion Checklist

- [x] Forge ticket created.
- [ ] Pipeline spec/notes pair created.
- [ ] `ticket:` frontmatter updated.
- [ ] `pipeline_spec:` frontmatter updated.
- [ ] `status:` changed to `promoted`.
