---
pipeline_id: da92661a-bc04-438f-931b-0bc66d314258
title: WORK-paint-tileset-layers-model-v1
ticket: c7c90d82-a960-4de5-919a-3d01823de95b
type: work
intake: docs/planning/intake/INTAKE-paint-system-tile-editor.md
notes: WORK-paint-tileset-layers-model-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-paint-tileset-layers-model-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Paint system slice 1 — tileset registry + multiple tile layers in
  the `MapDocument` model (ticket #47). The data foundation the editor paints
  onto; no UI.
- **Scope:**
  - **In (Rust, `crates/oathstar-content/src/map_document.rs` + tests):**
    1. **Tileset registry** — a registry of sheets, each `{ id, image,
       tile_size, columns, rows }` (sliced into `columns*rows` tiles) plus
       OPTIONAL sparse per-tile metadata (`tile_index -> { name?, passable?,
       tags? }`).
    2. **Tile layers** — multiple named layers, each `{ id, name, kind, visible,
       opacity?, cells: sparse (Cell -> TileRef) }` where `TileRef =
       { tileset, index }`.
    3. **Validation** — new typed `MapValidationError` arms (name the offender),
       covering: duplicate tileset/layer ids; tileset `tile_size` not in
       `SUPPORTED_TILE_SIZES` or non-positive `columns`/`rows`; per-tile
       metadata index out of range; layer cell out of grid bounds (reuse
       `in_bounds`); tile ref to an unknown tileset or an out-of-range index.
  - **Out (explicit):** any UI/editor/render change (that is S2+); persistence
    (S4); **materializing layers into the runtime `WorldDefinition`** — layers
    are authoring-visual this slice (validated, NOT materialized; the runtime
    world is byte-identical with or without them — deferred to the #38
    per-room-sprites rework); touching `terrain`/`rooms`/`spawn`/`regions`/
    `subregions` (additive only).
- **Systems:** content (map document model, Rust)

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a map document declares a tileset registry and tile layers whose every cell references a valid (tileset, index), the model shall validate and materialize it. | Rust test (happy path) |
| REQ-002 | When a layer cell references a tileset id absent from the registry, the model shall refuse with a typed error naming the layer, cell, and tileset id. | Rust test |
| REQ-003 | When a layer cell's `index` is greater than or equal to its tileset's `columns*rows`, the model shall refuse with a typed error naming the layer, cell, and index. | Rust test |
| REQ-004 | When two tilesets, or two layers, share an id, the model shall refuse with a typed error naming the duplicate id. | Rust test (both arms) |
| REQ-005 | When a layer cell lies outside the declared grid bounds, the model shall refuse with a typed error naming the cell. | Rust test |
| REQ-006 | When a tileset declares a `tile_size` not in `SUPPORTED_TILE_SIZES`, or a non-positive `columns`/`rows`, the model shall refuse with a typed error naming the tileset. | Rust test (each arm) |
| REQ-007 | When per-tile metadata names an index greater than or equal to `columns*rows`, the model shall refuse with a typed error naming the tileset and index. | Rust test |
| REQ-008 | An existing document carrying no tileset/layer fields shall deserialize and validate unchanged (serde-additive backward compatibility). | Rust test (deserialize a pre-slice JSON; round-trip) |
| REQ-009 | While a document carries tile layers, `materialize` shall produce a `WorldDefinition` identical to the same document without them (layers are authoring-visual, not materialized). | Rust test (materialize-equivalence) |
| REQ-010 | The full gate shall stay green with the new validation branches mutation-tight. | `bin/gate.sh` FULL (cov + 100% MSI) |

## Locked-In Decisions
- **Tile reference is `(tileset_id, integer tile_index)`, not by-name.** A
  30x203 = 6090-tile sheet cannot be all-named; names/passable/tags are OPTIONAL
  sparse per-tile metadata for the tiles that matter.
- **Additive + serde-additive.** New fields carry `#[serde(default)]` (and skip
  serializing when empty) so existing `MapDocument` JSON deserializes
  byte-compatibly; `terrain`/`rooms`/`spawn`/`regions`/`subregions` are
  untouched.
- **Layers are authoring-visual this slice** — validated but NOT materialized
  into the runtime world (deferred to #38). `materialize` ignores them.
- **Sparse, deterministic cell storage** — keyed by `Cell` in a `BTreeMap` (as
  `terrain_at` already does) so iteration/serialization is deterministic.
- **Typed errors mirror `MapValidationError`** — one variant per failure class,
  naming the offending id/cell/index.

## Linked Artifacts
- Design docs: `docs/decisions.md` (059 — `SUPPORTED_TILE_SIZES`),
  `docs/map-system.md` (map document model). A new Decision for the layer model
  is a candidate at Complete if the shape proves load-bearing.
- Intake doc: `docs/planning/intake/INTAKE-paint-system-tile-editor.md` (the S1-S5 program)
- Ticket doc: `docs/planning/tickets/open/TICKET-47-paint-tileset-layers-model.md`
- Forge ticket: c7c90d82-a960-4de5-919a-3d01823de95b (#47)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
