---
title: TICKET-32-tileset-v1-map-cells-draw-sprite-tiles
status: done
ticket: 4e0b8ebd-508c-469b-9d70-13b27b584087
ticket_number: 32
type: feature
created: 2026-06-11
closed: 2026-06-11
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-tileset-map-rendering-v1.spec.md
---

# TICKET-32-tileset-v1-map-cells-draw-sprite-tiles

## Summary

Pull the generated starter tileset (`assets/tilesets/oathstar-starter-16x16`
— 128×128 sheet PNG of 64 16px tiles + `.tsx` Tiled metadata + engine-friendly
`.json` twin + generator `bin/generate_oathstar_tileset.py`) into the #16
canvas map renderer: map cells draw as sprite tiles blitted from the sheet
instead of flat palette fills, with a flat-color fallback while the image
loads or if metadata is invalid.

## Why

Both `docs/ui-design.md` (Map Direction) and `docs/map-system.md` (Rendering
Direction) reserve "sprite tiles" as the canvas renderer's next step; ticket
#16 kept sprites/atlases explicitly out of scope. The Codex side-session
already generated a deterministic starter sheet — this ticket makes those
stray assets real committed inputs and gives the map its first art pass.

## EARS Requirements

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

## Scope

- In: pure node-tested tileset module (JSON parse/validate; cell kind → tile
  id → sheet pixel rect); draw-plan extension carrying tile-source refs;
  `drawImage` blitting in the existing `drawMapCanvas` seam only; flat-color
  fallback (image not loaded / invalid metadata); 16px→tilePixels crisp
  scaling (smoothing settings pinned at design); serving the assets via the
  vite `public/` path (vendor precedent — exact location at design);
  committing the tileset assets + generator; glyph + aria-label preserved;
  tests + docs.
- Out: server/Rust changes (map payload stays renderer-agnostic JSON —
  Decisions 025/035); entity/item/fog/DM overlays; animation; pan/zoom/
  camera; authored per-room tile assignments from world data (v1 maps by
  cell KIND); Tiled `.tmx` map files; re-authoring the art or generator.

## Notes

- Forge ticket: `4e0b8ebd-508c-469b-9d70-13b27b584087` (#32)
- Related docs: `docs/map-system.md` (Rendering Direction),
  `docs/ui-design.md` (Map Direction), `docs/decisions.md` (025, 035),
  `assets/tilesets/README.md` (tile ID ranges / format)
- Promoted from intake: none — direct owner request
- Active pipeline: `WORK-tileset-map-rendering-v1`
- Anchors verified at plan: `src/client/canvas-map.js` pure module
  (`MAP_PALETTE`/`canvasSize`/`cellKind`/`toDrawPlan`/`mapAriaLabel`);
  `client-app.js` `drawMapCanvas` seam (line ~449) +
  `mapRenderConfig = { ...DEFAULT_MAP_CONFIG }` (32px tiles); tileset JSON
  shape `{name, tileSize: 16, columns: 8, rows: 8, image, tiles[{id, name,
  x, y, tags, collision}]}`; vite serves `public/` verbatim (the
  `public/vendor/datastar` precedent); node/jsdom has no canvas-2D or Image
  (the #16 pure/seam split is load-bearing).
