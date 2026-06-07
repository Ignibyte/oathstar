---
title: TICKET-15-datastar-first-ui-transport-foundation
status: done
ticket: 25b0bddd-a96c-470b-8565-6f4c59e86130
ticket_number: 15
type: feature
created: 2026-06-07
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-datastar-first-ui-transport-foundation.spec.md
---

# TICKET-15-datastar-first-ui-transport-foundation

## Summary

Make the Datastar-first UI direction real by loading the Datastar runtime,
introducing a Rust presentation boundary for Datastar HTML/SSE fragments, and
converting one narrow player-client surface to the new transport while preserving
JSON for map/canvas renderer data.

## Why

The current player UI is Datastar-compatible but still hand-rolled with native
`EventSource`, `fetch`, and DOM mutation. Decision 034 locks Datastar/SSE HTML as
the first-party UI default. We need a small, testable implementation slice that
proves the Rust server can own Datastar presentation without moving game rules
into the frontend and without breaking the browser-first Player Client.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player client loads, the actual Datastar runtime shall be loaded or vendored through the project build in a reproducible way. | build / browser smoke |
| REQ-002 | When the server renders first-party UI fragments, the Datastar-specific rendering shall live behind a Rust presentation boundary, not inside `oathstar-core`. | code review / Rust tests |
| REQ-003 | When one selected UI surface is updated, it shall be delivered as Datastar-compatible HTML/SSE rather than hand-written DOM mutation for that surface. | browser smoke / integration test |
| REQ-004 | When map/minimap renderer data is requested, JSON map payloads shall remain available and shall not be replaced by HTML fragments. | test / API smoke |
| REQ-005 | When first-party UI uses Datastar HTML, server-provided text shall be escaped or otherwise rendered safely and shall not introduce HTML injection. | Rust tests |
| REQ-006 | When the player submits commands, the command path shall remain server-authoritative and shall not move rules/state into client JavaScript. | code review / smoke |
| REQ-007 | When the implementation is complete, the Player Client shall still run in the browser-first dev flow with `npm run server:dev` and `npm run dev`. | browser smoke |
| REQ-008 | When documenting the surface split, the Player Client, Host Manager, and Rust Game Server responsibilities shall reflect Decision 033. | docs review |
| REQ-009 | When the ticket is complete, `npm test`, `npm run build`, and `./bin/gate.sh --fast` shall pass. | command output |

## Scope

- In: load/vendor Datastar; add a Rust Datastar presentation boundary
  (`oathstar-datastar` crate or equivalent module if smaller); choose one narrow
  player-client surface to convert first, preferably event feed or a small status
  panel; preserve JSON map/state endpoints needed by the canvas/grid future; add
  escaping tests; update architecture/docs with the actual route/module names.
- Out: full Host Manager UI; Tauri sidecar lifecycle; canvas map renderer; full
  conversion of every panel; DaisyUI; React/Vue/Svelte; gameplay mechanics.

## Notes

- Forge ticket: `25b0bddd-a96c-470b-8565-6f4c59e86130` (#15)
- Decisions: 033, 034, 035 in `docs/decisions.md`
- Related docs: `docs/technical-architecture.md`, `docs/protocol-and-output.md`,
  `docs/ui-design.md`, `docs/map-system.md`
- The current frontend is allowed to remain partially hand-rolled during the
  slice. The goal is to prove the adapter and runtime with one concrete surface,
  not to rewrite everything in one pass.
