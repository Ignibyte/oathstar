---
title: TICKET-39-tmx-map-importer-build-time
status: open
ticket: b46517e6-9306-4a2e-a09e-18c04b013151
ticket_number: 39
type: feature
created: 2026-06-12
intake: docs/planning/intake/INTAKE-tileset-region-authoring-per-tile-metadata.md
pipeline_spec:
---

# TICKET-39-tmx-map-importer-build-time

## Summary

Step B: a build-time Rust importer that ingests a sub-region `.tmx` map and
materializes the engine's room graph — rooms, exits, and validation derived
from the Tiled layers, with the description cascade and reachability checks.

## Why

W1 (#40) is a multi-biome world; hand-authoring it room-by-room in TOML gets
tedious fast. The importer lets a map be drawn in Tiled and compiled into
rooms. It's authoring infrastructure — invisible on its own — and the
flat-color spike already proved the shape end to end, so it's de-risked. Per
the recommended sequence it can wait until hand-authoring genuinely hurts.

## EARS Requirements (candidate — finalize at /pipeline:plan)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the importer ingests a sub-region map, it shall derive rooms (id, title, description, coords, exits, region/subregion/floor) from the object layer and tile placements deterministically. | Rust test |
| REQ-002 | If a room object sits on a collision tile, a room is unreachable from spawn (absent an explicit override), or required map properties are missing, the importer shall refuse with a typed error naming the room/cell. | Rust test (each arm) |
| REQ-003 | A room's description shall come from the room object, falling back to its sub-region's description (the cascade). | Rust test |
| REQ-004 | The importer shall reproduce the beginner world's room graph from an equivalent `.tmx` (round-trip parity). | Rust test |

## Scope

- In: build-time importer (Rust, content-crate orbit); object-layer → rooms;
  adjacency-derived exits; stair-link + reachability validation with typed
  refusals; description cascade; beginner-world round-trip parity.
- Out: runtime `.tmx` parsing (locked out — build-time only); entity defs
  (stay TOML); runtime state (stays overlay markers); the world content itself
  (#40).

## Notes

- Forge ticket: b46517e6-9306-4a2e-a09e-18c04b013151 (#39)
- Related docs: INTAKE-tileset-region-authoring-per-tile-metadata (full model,
  one-source-per-concern, build-time-importer decision, spike artifacts)
- Architecture: build-time importer, NOT engine parsing (locked in the intake).
- Promoted from intake: INTAKE-tileset-region-authoring-per-tile-metadata (step B)
- Active pipeline: none yet — promote via `/work` when ready
- Sequence: deferred — authoring infra; bring in when TOML hand-authoring of
  W1 gets painful, or before #40 if authoring in Tiled is preferred.
