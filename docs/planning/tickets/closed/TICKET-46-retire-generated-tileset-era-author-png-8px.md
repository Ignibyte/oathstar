---
title: TICKET-46-retire-generated-tileset-era-author-png-8px
status: closed
ticket: 9117db55-c8d1-4e90-bb83-3fc7738da864
ticket_number: 46
type: feature
created: 2026-06-16
intake: docs/planning/intake/INTAKE-tileset-region-authoring-per-tile-metadata.md
pipeline_spec: docs/planning/pipeline/completed/WORK-tileset-author-png-8px-v1.spec.md
---

# TICKET-46-retire-generated-tileset-era-author-png-8px

## Summary

Retire the generated-tileset / blank-colors era and move to author-provided
PNG tile sheets at the sheet's declared size, unlocking 8px-native maps. Clears
the runway for the tile-painting editor backend.

## Why

The real art is 8x8 PNG sheets, not the ElvGames 16x16 set we first imported.
The owner is building a first-party studio editor (the in-house "Tiled"), so the
Tiled-import path and the generated flat-color tilesets are dead weight. The map
model currently hard-pins `tile_size = 16`; the design docs already promise
configurable 8/16/32 tiles — this finishes that and inverts Decision 054 so real
art (not flat color) is the contract.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a map document declares a `tile_size` of 8, 16, or 32, the content model shall validate and materialize it. | Rust test |
| REQ-002 | When a map document declares a `tile_size` outside the supported set, the content model shall refuse it with a typed `UnsupportedTileSize` error. | Rust test |
| REQ-003 | After this work, the repository shall contain no committed tileset assets and no tileset-generator scripts. | Review + path-absence; gate green |
| REQ-004 | When the client is given a tile-sheet descriptor conforming to the documented contract, it shall draw map cells from that sheet. | node --test (fixture sheet) |
| REQ-005 | While no conforming sheet is available, or a referenced tile name is absent, the client shall render the flat-color fallback. | node --test |
| REQ-006 | The full gate shall pass with the generated-tileset assets and tests removed and no dangling references. | `bin/gate.sh` FULL |
| REQ-007 | The record shall mark Decisions 035/050/054 superseded and ticket #39 closed won't-fix. | doc check + forge AD + ticket-close |

## Scope

- In: retire generated tilesets + generators + their tests; tile-size-agnostic
  model (8/16/32); documented author PNG-sheet contract + fixture; client loader
  swap keeping the flat-color fallback; record the decision supersede + close
  #39.
- Out: real 8px art ingestion (fixture only); the tile-painting editor backend
  (next pipeline); ticket #38 rework (noted); movement / 1-cell = 1-room
  (Decision 025) / render scale changes.

## Notes

- Forge ticket: 9117db55-c8d1-4e90-bb83-3fc7738da864 (#46)
- Related docs: docs/decisions.md (035/050/054 superseded; 025 unchanged),
  docs/map-system.md, docs/ui-design.md, docs/technical-architecture.md
- Promoted from intake: INTAKE-tileset-region-authoring-per-tile-metadata.md
  (Tiled + blank-colors portions superseded)
- Active pipeline: WORK-tileset-author-png-8px-v1
