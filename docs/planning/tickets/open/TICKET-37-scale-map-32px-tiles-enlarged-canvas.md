---
title: TICKET-37-scale-map-32px-tiles-enlarged-canvas
status: open
ticket: 5a348bfb-4c21-4696-bea3-4597c32da5da
ticket_number: 37
type: feature
created: 2026-06-12
intake:
pipeline_spec:
---

# TICKET-37-scale-map-32px-tiles-enlarged-canvas

## Summary

Render map cells at 32px (up from 16) and scale the canvas element up so the
tilemap reads physically bigger and crisper on the page.

## Why

Owner direction (2026-06-12): the map is too small to read comfortably. Bigger
cells + a bigger canvas make the world legible — and it pairs naturally with
the biome-color step (#38), which is when the bigger map starts carrying real
information.

## EARS Requirements (candidate — finalize at /pipeline:plan)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The map canvas shall render each room cell at 32px and present at an enlarged on-page size, with the four cell kinds still resolving by name. | node --test (canvas-map draw plan) + visual smoke |
| REQ-002 | When the tileset is regenerated, the committed-asset contract tests shall pin the new tile geometry and the blank-colors uniformity contract shall still hold (every tile one color). | node --test (tileset contract + uniformity pin) |
| REQ-003 | The wire/engine shall be unchanged (room coordinates, not pixels); no map-snapshot field is added. | review + protocol tests unchanged |

## Scope

- In: 32px cell rendering; enlarged canvas (CSS / device-pixel scaling);
  tileset regen at 32×32 OR 2× upscale of the 16px source (design decides —
  lean to 32px source so real art isn't upscaled later); contract-test updates.
- Out: per-room biome colors (#38), any wire/engine change, real art.

## Notes

- Forge ticket: 5a348bfb-4c21-4696-bea3-4597c32da5da (#37)
- Related docs: docs/decisions.md (050 name-keyed tiles, 054 blank-colors)
- Promoted from intake: none — direct owner direction
- Active pipeline: none yet — promote via `/work` when ready
- Sequence: rendering polish; do alongside or before #38 (both touch
  src/client/canvas-map.js).
