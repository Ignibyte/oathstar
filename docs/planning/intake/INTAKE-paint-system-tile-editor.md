---
title: INTAKE-paint-system-tile-editor
status: promoted
created: 2026-06-17
ticket: c7c90d82-a960-4de5-919a-3d01823de95b
pipeline_spec: docs/planning/pipeline/active/WORK-paint-tileset-layers-model-v1.spec.md
---

# INTAKE-paint-system-tile-editor

## Problem / Opportunity

Owner direction (2026-06-17): build a first-party **tile-painting editor** in
the Oathstar studio — a Tiled-like painter with **metadata, multiple layers**,
and a tileset palette. The studio `/editor` today (ticket #45) only RENDERS the
authoring `MapDocument` read-only (flat-color cells + a Validate button); there
is no painting. The owner dropped a real 8px sheet
(`public/tilesets/arctic.png`, 240x1624 = a 30x203 grid of 8px tiles) and wants
to paint maps with it. Authoring is first-party (no Tiled — Decision 059); the
editor slices a raw sheet itself and the author attaches names/metadata to the
tiles that matter.

## Proposed Outcome

A studio map editor where the author loads a tileset (a sheet sliced at its tile
size into a pickable palette), paints tiles onto **multiple named layers** of
the map grid, edits per-tile / per-layer / per-room metadata, and saves. "Mimic
Tiled a bit" — the core feel (palette, paint tile layers, custom properties),
not full Tiled parity. The model already has Tiled-ish bones — a single terrain
tile-layer, rooms as an object-layer, region/subregion metadata maps — and this
generalizes them.

## Slice roadmap (each ships alone, gate-green)

- **S1 (ticket #47, promoted):** document-model foundation — a tileset registry
  + multiple tile layers + validation; additive + serde-additive.
  Authoring-visual (validated, not yet materialized into the runtime world).
- **S2 + S3 (ticket #48, shipped):** render arctic sprites + stacked layers
  on the editor canvas, a tileset palette panel, tile selection, and click/drag
  paint of the active layer — the visible paint loop.
- **S4:** save/load persistence (studio save endpoint + `oathstar-storage`).
- **S5:** per-tile / per-layer / per-room metadata property panels.
- **Later:** layers materialize into the runtime map (the per-room-sprites
  rework, ticket #38); object/room editing; multi-floor; undo/redo.

## Candidate EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The map document shall carry a tileset registry and multiple named tile layers, validated and serde-additive. | Rust test (S1) |
| REQ-002 | When the author picks a tile and clicks a cell, the editor shall place it on the active layer and render it. | node --test + smoke (S2/S3) |
| REQ-003 | The author shall save and reload a painted map without loss. | server test (S4) |
| REQ-004 | The author shall attach metadata to tiles, layers, and rooms. | test + smoke (S5) |

## Scope Notes

- In: first-party paint editor (palette, multi-layer tile painting, metadata,
  save), built on the existing `MapDocument` + studio `/editor`.
- Out: full Tiled parity (hex/iso, wang/terrain brushes, animations); importing
  Tiled files (Decision 059 — no Tiled); runtime materialization of paint layers
  (deferred to #38).

## Promotion Checklist

- [x] Forge ticket created (S1 = #47).
- [x] Pipeline spec/notes pair created (S1).
- [x] `ticket:` / `pipeline_spec:` updated (S1).
- [x] `status: promoted` (program underway; S1 promoted, S2-S5 carve as they come).
