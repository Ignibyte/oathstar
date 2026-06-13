---
title: TICKET-38-map-biomes-per-room-tile-colors-over-the-wire
status: open
ticket: 51636b55-d4e7-4f22-a826-0a2b5fc04b76
ticket_number: 38
type: feature
created: 2026-06-12
intake: docs/planning/intake/INTAKE-tileset-region-authoring-per-tile-metadata.md
pipeline_spec:
---

# TICKET-38-map-biomes-per-room-tile-colors-over-the-wire

## Summary

Step C of the blank-colors program: the engine projects a floor tile name per
discovered room (from its subregion), and the client renders each cell in that
authored color — so the flat tileset's biome colors finally appear on the map.

## Why

#36 put the biome colors in the sheet, but every discovered room still draws as
the generic `stone_floor` grey. C is the wire+render step that makes the colors
mean something — and it's immediately visible on the existing Hollowmere world,
no new content required. It's the prerequisite that makes the city/forest/cave
biomes legible when W1 (#40) lands.

## EARS Requirements (candidate — finalize at /pipeline:plan)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When `/state` is served, each discovered map room shall carry an authored floor tile name (derived from its subregion), serde-additive so old payloads/clients are unaffected. | cargo test (protocol + engine projection) |
| REQ-002 | While rendering the map, the client shall draw each discovered cell with its room's floor tile, falling back to the generic discovered tile when none is authored. | node --test (canvas-map draw plan) |
| REQ-003 | The reveal rules shall apply to the new projection: only discovered rooms carry a floor; hidden/undiscovered rooms are unchanged. | cargo test (discovered-gating) |

## Scope

- In: subregion→floor-tile mapping; serde-additive `MapRoomSnapshot` field;
  client draw-plan wiring through the existing name→rect path; reveal-gating.
- Out: per-tile descriptions (later metadata step), the importer (#39), new
  world content (#40), the 32px scale (#37).

## Notes

- Forge ticket: 51636b55-d4e7-4f22-a826-0a2b5fc04b76 (#38)
- Related docs: docs/decisions.md (050/051/054); INTAKE-tileset-region-authoring
  (step C); INTAKE-blank-colors-vertical-slice
- Watch: PR-claude-reveal-rules-on-every-projection-001 (high — bit at #33/#34);
  PR-oathstar-render-plan-test-002 (draw-plan ops carry only drawn fields).
- Promoted from intake: INTAKE-tileset-region-authoring-per-tile-metadata (step C)
- Active pipeline: none yet — promote via `/work` when ready
- Sequence: recommended NEXT (small, visible). Depends on #36 ✅; composes
  with #37.
