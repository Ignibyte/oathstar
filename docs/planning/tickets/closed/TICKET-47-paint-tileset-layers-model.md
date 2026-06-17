---
title: TICKET-47-paint-tileset-layers-model
status: closed
ticket: c7c90d82-a960-4de5-919a-3d01823de95b
ticket_number: 47
type: feature
created: 2026-06-17
intake: docs/planning/intake/INTAKE-paint-system-tile-editor.md
pipeline_spec: docs/planning/pipeline/completed/WORK-paint-tileset-layers-model-v1.spec.md
---

# TICKET-47-paint-tileset-layers-model

## Summary

Paint-system slice 1: extend the `MapDocument` model with a tileset registry and
multiple named tile layers (+ validation), additively. The data foundation the
tile-painting editor will paint onto.

## Why

The studio `/editor` only renders maps read-only today; to paint (the owner's
goal, with an 8px `arctic.png` sheet), the document first needs to model
tilesets and stackable tile layers. Slice 1 of the paint-system program
(INTAKE-paint-system-tile-editor); the visible painting comes in S2/S3.

## EARS Requirements

See the pipeline spec (REQ-001..010): happy-path validate/materialize, each
typed-refusal arm (unknown tileset ref, out-of-range index, duplicate ids,
out-of-bounds cell, bad tileset geometry, per-tile metadata index), the
serde-additive backward-compat proof, the materialize-equivalence proof
(layers authoring-visual, not materialized), and a green gate at 100% MSI.

## Scope

- In: tileset registry + tile layers + validation in
  `crates/oathstar-content/src/map_document.rs` (+ tests). Additive,
  serde-additive.
- Out: editor UI / render (S2), palette + paint (S3), persistence (S4), metadata
  panels (S5); runtime materialization of layers (deferred to #38).

## Notes

- Forge ticket: c7c90d82-a960-4de5-919a-3d01823de95b (#47)
- Program intake: docs/planning/intake/INTAKE-paint-system-tile-editor.md
- Active pipeline: WORK-paint-tileset-layers-model-v1
