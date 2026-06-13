---
title: TICKET-36-flatten-tileset-solid-colors-lean-names
status: closed
ticket: edd292c5-a346-49aa-86a8-f60191f2a081
ticket_number: 36
type: feature
created: 2026-06-12
intake: docs/planning/intake/INTAKE-tileset-region-authoring-per-tile-metadata.md
pipeline_spec: docs/planning/pipeline/completed/WORK-tileset-flatten-v1.spec.md
---

# TICKET-36-flatten-tileset-solid-colors-lean-names

## Summary

Regenerate the committed starter tileset as solid color blocks: every tile
one uniform color from a named palette, a lean name set (the four
load-bearing names survive; the 32 connectivity variants go; flat extras
for the slice's biomes arrive), and a test pin that keeps it that way.

## Why

Step A of the blank-colors program: the slice plays in legible flat color
with zero art debt, and because everything resolves by name, real art
returns at ship time as a pure file swap. Foundation for B (the .tmx
importer) and C (per-room tile names over the wire — biome colors live).

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The regenerated committed tileset shall contain the four load-bearing tile names with unchanged 16×16 tile geometry, and the client tileset module shall validate it with zero code changes. | node --test (existing contract tests, untouched client) |
| REQ-002 | Every tile in the committed sheet shall be one uniform color block (all pixels in the tile identical). | node --test blank-colors pin over the committed PNG |
| REQ-003 | The lean name set shall include the flat extras (forest/grass floor, road, cave rock floor, water, stairs up, stairs down, exit marker), each name unique and consistent across the png/json/tsx triplet. | node --test count + json↔tsx cross-checks |
| REQ-004 | When `bin/generate_oathstar_tileset.py` is re-run, it shall reproduce the committed asset bytes exactly. | re-run + `git diff --exit-code` at validate |
| REQ-005 | While the flattened sheet is loaded, the existing map canvas draw-plan suites shall pass unchanged. | node --test (canvas-map + client suites) |

## Scope

- In: generator palette rework, regenerated committed assets
  (png/json/tsx/preview/README), test updates + uniformity pin.
- Out: client/renderer changes, per-tile descriptions, the importer (B),
  wire changes (C), entity tiles, real art.

## Notes

- Forge ticket: edd292c5-a346-49aa-86a8-f60191f2a081 (#36)
- Related docs: docs/decisions.md (050/051)
- Promoted from intake: INTAKE-tileset-region-authoring-per-tile-metadata (step A)
- Pipeline (completed): docs/planning/pipeline/completed/WORK-tileset-flatten-v1.spec.md
