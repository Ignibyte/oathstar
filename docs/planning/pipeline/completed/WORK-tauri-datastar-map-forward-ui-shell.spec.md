---
pipeline_id: 1d2e291b-a3c4-4c35-9d00-da046daf0bd0
title: WORK-tauri-datastar-map-forward-ui-shell
ticket: a063dadb-6a68-4380-80b7-21c55966aead
type: work
intake: docs/planning/intake/INTAKE-beginner-slice-ui-startup.md
notes: WORK-tauri-datastar-map-forward-ui-shell.notes.md
status: Phase 5 — Complete PASS
---

# WORK-tauri-datastar-map-forward-ui-shell

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Build the first playable Tauri + Datastar/SSE map-forward UI shell
  for the beginner vertical slice, server-authoritative against the Rust runtime.
- **Scope:**
  - **In:** seed `Engine::begin()`'s opening scene onto every new `/events` SSE
    subscription (server change, REQ-001); a framework-free, server-authoritative
    hypermedia client that POSTs `/command`, consumes `/events` SSE, and renders
    `/state` snapshots; typed event → componentized output (not a textarea);
    map-forward layout (HUD, map/stage, output feed, top-right tabbed character
    menu, bottom-right Intent panel); Intent search/filter + quick commands with
    free-text entry preserved; an explicit client-side map render config
    (tile size + render mode) leaving the server `MapSnapshot` shape unchanged;
    Tauri-ready client path (loopback base URL, configurable).
  - **Out:** full inventory/equipment mechanics; save/load UI; production combat
    UI; production canvas/sprite renderer; full accessibility pass; multiplayer
    or DM panels; adopting the Datastar **library** + server-side Datastar SSE
    rendering (deferred — see Locked-In Decisions); replacing the Rust
    server/core boundaries; auto-spawning the server from the Tauri Rust shell.
- **Systems:** ui, engine (server boundary / SSE), map, oath.

## Acceptance Criteria (EARS)
Each acceptance criterion uses EARS syntax, describes one observable behavior,
and includes a verification method. Carried verbatim from forge ticket #12.

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a player starts a new local session, the server shall deliver the start room's opening scene without requiring the player to submit `look` first. | server integration test |
| REQ-002 | When the player submits text through the client command prompt, the Tauri/Datastar client shall POST the command to `/command` and render the resulting typed events. | UI test (pure render core) / API smoke |
| REQ-003 | While the client is connected, it shall consume `/events` SSE and render typed events into componentized output rather than a textarea. | UI test (pure render core) / manual smoke |
| REQ-004 | When the main play screen renders, it shall follow the map-forward layout documented in `docs/ui-design.md`, including top health/focus HUD, prominent map/stage, typed output underneath, top-right menu, and bottom-right Intent panel. | screenshot smoke / review |
| REQ-005 | When contextual actions are available, the client shall expose a tabbed character menu with at least `Nearby`, `Oaths`, `Gear`, and `Pack` panels. | UI test / manual smoke |
| REQ-006 | When command suggestions are available, the client shall expose an Intent panel with search/filter and quick command buttons while preserving free text command entry. | UI test (pure render core) / manual smoke |
| REQ-007 | When map state is rendered, the map layer shall keep tile size and render mode configurable enough for a later canvas or sprite renderer without changing the server's map state shape. | code review / UI test |
| REQ-008 | When implementing the shell, the frontend shall remain Tauri + Datastar/HTML-fragment oriented and shall not introduce a React, Vue, Svelte, or similar SPA framework without a separate architecture decision. | code review |
| REQ-009 | When the ticket is complete, the repository fast gate shall pass and any new UI behavior shall have targeted coverage or documented smoke verification. | `./bin/gate.sh --fast` |

## Locked-In Decisions
- **Opening scene = `begin()` replayed on subscribe (Decision 031).** `Engine::try_new`
  emits nothing; `Engine::begin()` is the on-start emitter. The server captures
  `begin()` once at startup and seeds those events at the head of every new
  `/events` (and `/events/html`) subscription. begin() does not move the player,
  so `/state` stays consistent. No new "begin" endpoint is required.
- **Framework-free, server-authoritative hypermedia client; Datastar-library
  adoption deferred.** The client uses plain ES modules with `fetch`
  (`/command`, `/state`) + `EventSource` (`/events`). This satisfies REQ-008
  ("Datastar/HTML-fragment oriented", no React/Vue/Svelte) and the
  `docs/ui-design.md` guardrail ("compatible with Datastar and SSE instead of
  drifting into a large SPA framework"). Vendoring the Datastar runtime + emitting
  Datastar-format SSE from the server is a deliberate, separately-ticketed follow-up
  (recorded as an architecture decision in Phase 5), kept out to honour "smallest
  production-direction shell."
- **Pure/glue split for testability + the 75% JS floor.** All render/logic is
  DOM-free, importable ES modules under `src/client/` (tested by `tests/*.test.js`,
  counted by `node --experimental-test-coverage`); the DOM + transport shell
  (`EventSource`, `fetch`, `document`) is a thin browser-only entry that no test
  imports (smoke-verified). Mirrors the proven `engine.js` (tested) / `app.js`
  (glue) split.
- **Wire-format split honoured (Decision 031).** `/events` payload fields are
  snake_case (`room_id`, `oath_id`, `type`); `/state` snapshot fields are
  camelCase (`currentRoomId`, `maxHp`). The client parses each accordingly.
- **Map render config lives in the client, not world state (REQ-007).** An
  explicit `{ tilePixels, mode }` config drives rendering; the server
  `MapSnapshot` shape is unchanged.
- **Reuse the existing map-forward shell.** `index.html` + `styles.css` already
  implement the layout, tabs, and Intent panel; this work repoints the page from
  the in-browser prototype engine to the server-backed client. The prototype
  files (`src/engine.js`, `src/world.js`, `src/app.js`) are retained as the
  visual/interaction reference per `docs/ui-design.md`.

## Linked Artifacts
- Design docs: `docs/ui-design.md`, `docs/technical-architecture.md`,
  `docs/protocol-and-output.md`, `docs/map-system.md`, `docs/decisions.md`
  (Decisions 015/016/028/031)
- Intake doc: `docs/planning/intake/INTAKE-beginner-slice-ui-startup.md`
- Ticket doc: `docs/planning/tickets/open/TICKET-12-build-tauri-datastar-map-forward-ui-shell.md`
- Forge ticket: a063dadb-6a68-4380-80b7-21c55966aead (#12)
- AAR: 30b3f1ce-d02b-4629-b055-3ae2fe741bc7

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
