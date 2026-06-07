---
pipeline_id: f2120f7b-38f9-48ce-ba94-b820226a55e7
title: WORK-canvas-grid-map-renderer
ticket: f69a718c-7ae8-418c-895e-9e1baae72f98
type: work
intake:
notes: WORK-canvas-grid-map-renderer.notes.md
status: Phase 1 — Plan PASS; Phase 2 — Design PASS; Phase 3 — Implement PASS; Phase 3.5 — Inspect PASS; Phase 4 — Validate PASS; Phase 5 — Complete PASS
---

# WORK-canvas-grid-map-renderer

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Canvas/Grid Map Renderer v1 — replace the DOM `#map` with a first-party canvas.
- **Scope:**
  - **In:** a pure, tested canvas-model/math module under `src/client/` (tile→pixel
    math, canvas + backing-store sizing with `devicePixelRatio`, cell
    classification / draw-plan); a thin canvas-2D draw seam in the browser glue
    (`client-app.js`); replace the `#map` div with a `<canvas>`; reuse
    `toMapModel`/`DEFAULT_MAP_CONFIG` and `mapRenderConfig`; extend the cell model
    with `passable` **only if** needed for blocked styling (Design decides); keep
    `#map-label` + room panel + exit pad + intent panel + Datastar feed; a canvas
    accessibility summary (the canvas is one element, so per-cell DOM a11y is lost).
  - **Out:** sprites/atlases/animation; any server/Rust map change; full Tauri
    lifecycle; fog-of-war/entity/DM overlays; pan/zoom/camera.
- **Systems:** ui (player-client map surface), frontend build. **No** Rust/server
  change — the `/state` map payload stays renderer-agnostic JSON (Decisions 025/035).

## Acceptance Criteria (EARS)
| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player client renders the map, it shall draw to an HTML5 `<canvas>` (replacing the DOM `#map` grid), consuming `snapshot.map` via `toMapModel`. | browser smoke / JS test |
| REQ-002 | When a client render config sets `tilePixels`, the canvas shall size tiles to it, defaulting to 32×32. | JS test / smoke |
| REQ-003 | When rooms stack across floors, the canvas shall render only the current z-plane (the cells `toMapModel` selects). | JS test |
| REQ-004 | When drawing the grid, the canvas shall visually distinguish unknown/empty cells, discovered rooms, and the current room, and shall show glyph/title hints (and passable/blocked styling where the model exposes passability). | JS test / smoke |
| REQ-005 | When the map renders on a Hi-DPI display, the canvas shall scale its backing store by `devicePixelRatio` so output stays crisp. | JS test (sizing math) / smoke |
| REQ-006 | When the canvas map is in place, the map label, room panel, exit pad, intent panel, and Datastar event feed shall continue to function. | browser smoke / existing JS tests |
| REQ-007 | The renderer shall not add a game-engine dependency (no Phaser/Pixi/etc.); only the canvas 2D API. | package.json review |
| REQ-008 | The server map payload shall remain JSON from `GET /state`; no canvas drawing commands shall be added to Rust. | code review / API smoke |
| REQ-009 | When complete, focused JS tests shall cover the canvas model/math (pure functions), with a browser smoke for the rendered canvas. | node --test |
| REQ-010 | When the ticket is complete, `npm test`, `npm run build`, and `./bin/gate.sh --fast` shall pass. | command output |

## Locked-In Decisions
- **Square grid; server map is renderer-agnostic JSON; no game-engine dependency**
  (Decisions 025 + 035). The renderer is first-party canvas-2D; default tile 32×32,
  configurable; renders the current z-plane only.
- **No server/Rust change** — `GET /state` keeps returning `Json<GameSnapshot>`
  with `map`. Canvas drawing stays entirely client-side.
- **Reuse the existing model** — `toMapModel`/`DEFAULT_MAP_CONFIG`/`mapRenderConfig`
  are the source of truth; the canvas consumes the model, it does not re-fetch or
  re-shape server data. Extending the cell model with `passable` is the only
  permitted model change, and only if blocked styling needs it (Design decides).
- **Pure/glue split (mirror #15)** — tile/pixel math, DPR sizing, and cell
  classification live in a DOM-free tested module (`node --test`, counts toward the
  75% JS floor); the `canvas.getContext('2d')` draw calls live in the browser-only
  glue, smoke-verified.
- **Replace, don't dual-render** — the `#map` DOM grid is replaced by the canvas
  (not kept alongside). Other panels are untouched.

## Linked Artifacts
- Design docs: `docs/map-system.md`, `docs/ui-design.md`, `docs/decisions.md` (025/035/031).
- Intake doc: none.
- Ticket doc: `docs/planning/tickets/open/TICKET-16-canvas-grid-map-renderer.md`
- Forge ticket: `f69a718c-7ae8-418c-895e-9e1baae72f98` (#16)
- Forge AAR: `625818a1-34f2-4263-9c2d-492cb4184209`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS (design + test plan in notes) |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS (3 critics; 1 consistency + LOWs fixed) |
| 4 — Validate | PASS (gate --fast GREEN; node 26, cargo 122) |
| 5 — Complete | PASS |
