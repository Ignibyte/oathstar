---
title: TICKET-16-canvas-grid-map-renderer
status: done
ticket: f69a718c-7ae8-418c-895e-9e1baae72f98
ticket_number: 16
type: feature
created: 2026-06-07
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-canvas-grid-map-renderer.spec.md
---

# TICKET-16-canvas-grid-map-renderer

## Summary

Build Oathstar's first-party `<canvas>` map renderer (v1), replacing the current
DOM tile surface (`#map`) while keeping the rest of the UI Datastar/browser-first.
The server map payload stays renderer-agnostic JSON from `GET /state`; no canvas
drawing commands are added to Rust (Decisions 025 + 035).

## Why

Decision 035 locks the first map renderer as a small first-party 32×32 square-grid
canvas consuming server JSON; Decision 025 says "consider canvas early so the grid
can later support sprites and overlays." The DOM/CSS grid (#12/#13) was the
placeholder; this is the "revisit when the minimap renderer is implemented"
trigger. Canvas gives a stable grid, less DOM churn, and a clean path to
sprites/overlays — without a game-engine dependency.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player client renders the map, it shall draw to an HTML5 `<canvas>` (replacing the DOM `#map` grid), consuming `snapshot.map` via the existing `toMapModel`. | browser smoke / JS test |
| REQ-002 | When a client render config sets `tilePixels`, the canvas shall size tiles to it, defaulting to 32×32. | JS test / smoke |
| REQ-003 | When rooms stack across floors, the canvas shall render only the current z-plane (the cells `toMapModel` selects). | JS test |
| REQ-004 | When drawing the grid, the canvas shall visually distinguish unknown/empty cells, discovered rooms, and the current room, and shall show glyph/title hints (and passable/blocked styling where the model exposes passability). | JS test / smoke |
| REQ-005 | When the map renders on a Hi-DPI display, the canvas shall scale its backing store by `devicePixelRatio` so output stays crisp. | JS test / smoke |
| REQ-006 | When the canvas map is in place, the map label, room panel, exit pad, intent panel, and Datastar event feed shall continue to function. | browser smoke / JS tests |
| REQ-007 | The renderer shall not add a game-engine dependency (no Phaser/Pixi/etc.); only the canvas 2D API. | package.json review |
| REQ-008 | The server map payload shall remain JSON from `GET /state`; no canvas drawing commands shall be added to Rust. | code review / API smoke |
| REQ-009 | When complete, focused JS tests shall cover the canvas model/math (pure functions), with a browser smoke for the rendered canvas. | node --test |
| REQ-010 | When the ticket is complete, `npm test`, `npm run build`, and `./bin/gate.sh --fast` shall pass. | command output |

## Scope

- In: pure tested canvas-model/math module under `src/client/`; thin canvas-2D
  draw seam in the glue; replace `#map` div with `<canvas>`; reuse
  `toMapModel`/`DEFAULT_MAP_CONFIG`/`mapRenderConfig`; extend the cell model with
  `passable` only if needed for blocked styling; keep the map label + panels;
  canvas accessibility summary.
- Out: sprites/atlases/animation; server-side map changes; full Tauri lifecycle;
  fog-of-war/entity/DM overlays; pan/zoom/camera.

## Notes

- Forge ticket: `f69a718c-7ae8-418c-895e-9e1baae72f98` (#16)
- Decisions: 025, 035 (and 031) in `docs/decisions.md`
- Related docs: `docs/map-system.md`, `docs/ui-design.md`
- Reuse: `src/client/map.js`, `client-app.js` `renderMap`, `#map` in `index.html`
- Pipeline (completed): `docs/planning/pipeline/completed/WORK-canvas-grid-map-renderer.spec.md`
