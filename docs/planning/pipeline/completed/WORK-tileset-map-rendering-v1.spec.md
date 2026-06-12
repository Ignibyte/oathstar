---
pipeline_id: 2b069ccb-9bf2-4328-aabf-4d9bfba6c2d8
title: WORK-tileset-map-rendering-v1
ticket: 4e0b8ebd-508c-469b-9d70-13b27b584087
type: work
intake:
notes: WORK-tileset-map-rendering-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-tileset-map-rendering-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Tileset Map Rendering v1 — map cells draw as sprite tiles from
  the committed starter sheet, with flat-color fallback (the #16 canvas
  renderer's reserved "sprite tiles" step).
- **Scope:**
  - **In:** a pure, node-tested tileset module under `src/client/` (parse +
    validate the tileset JSON; resolve cell kind → tile id → source pixel
    rect); extend the #16 draw-plan with tile-source references; `drawImage`
    blitting ONLY in the existing `drawMapCanvas` seam; flat-color fallback
    while the sheet image is unloaded or metadata invalid (no errors, no
    blank map); crisp 16px→`tilePixels` pixel-art scaling (smoothing
    settings pinned at design); serve assets through vite `public/` (exact
    path at design; `public/vendor/` is the precedent); **commit the tileset
    assets + generator** (`assets/tilesets/*`,
    `bin/generate_oathstar_tileset.py`); glyph + aria-label preserved;
    tests + docs.
  - **Out:** server/Rust changes (renderer-agnostic JSON — Decisions
    025/035); entity/item/fog/DM overlays; animation; pan/zoom/camera;
    authored per-room tile assignment from world data (v1 maps by cell
    KIND); Tiled `.tmx` maps; re-authoring art/generator.
- **Systems:** ui (map renderer), frontend build/assets. **No** engine,
  server, protocol, or storage changes.

## Acceptance Criteria (EARS)
Verbatim from `TICKET-32` (forge `4e0b8ebd-508c-469b-9d70-13b27b584087`).

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The pure module shall resolve each map cell kind to a deterministic tile reference (id + source pixel rect) from the committed tileset JSON, unit-testable without DOM or canvas. | node --test |
| REQ-002 | When the tileset image and metadata are loaded, the canvas seam shall draw each present cell's base as the resolved sprite tile from the sheet instead of the flat palette fill. | JS draw-plan test + browser smoke |
| REQ-003 | While the tileset image is not loaded or its metadata is invalid, the renderer shall fall back to the existing flat-color draw with no thrown errors and no blank map. | node --test (plan/fallback) + smoke |
| REQ-004 | The tileset metadata loader shall validate the JSON (tile size, columns, image, required tile names) and signal a typed invalid result on malformed data instead of throwing. | node --test |
| REQ-005 | The renderer shall scale 16px source tiles to the configured tilePixels with pixel-art-crisp settings at any devicePixelRatio. | JS sizing test + smoke |
| REQ-006 | The glyph mark and canvas aria-label behavior shall be preserved under tile rendering. | existing + extended node --test |
| REQ-007 | The committed tileset assets shall be served by the dev server and included in the production build. | build check |
| REQ-008 | Existing map/client/engine behavior and the gate shall continue to pass with no server/Rust changes. | gate |

## Locked-In Decisions
Settled before design; not re-litigated mid-pipeline. Open *design* choices
are enumerated in the notes for Phase 2.

- **Client-only.** The server map payload stays renderer-agnostic JSON
  (Decisions 025/035) — zero Rust/protocol/storage changes.
- **The #16 pure/seam split is load-bearing.** node/jsdom has no canvas-2D
  and no `Image` — ALL tile-resolution logic (metadata parse/validate, kind
  → tile ref, draw-plan) lives in pure DOM-free modules under `node --test`;
  only `drawMapCanvas` may touch `drawImage`/image elements (browser smoke).
- **Runtime consumes the JSON twin.** The `.tsx` (Tiled XML) ships committed
  for editor interop but is NOT parsed by the client — the generator emits
  the engine-friendly `.json` for exactly this purpose (its README says so).
- **Fallback, never blank.** Tile rendering is an enhancement over the #16
  flat-color draw: image-not-yet-loaded and invalid-metadata both fall back
  to the existing palette rendering with no thrown errors.
- **v1 maps by cell kind.** The four #16 kinds (empty/discovered/current/
  blocked) map to tiles; authored per-room tile choice from world data is a
  future ticket.
- **The strays become real.** `assets/tilesets/*` + the generator are
  committed as part of this work (the #31-era "untouchable" bound is lifted
  by the owner's direct request).

## Linked Artifacts
- Design docs: `docs/map-system.md` (Rendering Direction), `docs/ui-design.md`
  (Map Direction), `docs/decisions.md` (025, 035), `assets/tilesets/README.md`
- Intake doc: none — direct owner request
- Ticket doc: `docs/planning/tickets/open/TICKET-32-tileset-v1-map-cells-draw-sprite-tiles.md`
- Forge ticket: `4e0b8ebd-508c-469b-9d70-13b27b584087` (#32)
- AAR: (recorded in notes at Phase 1 closeout)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
