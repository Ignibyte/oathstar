---
title: TICKET-43-oathstar-map-document-model-v1
status: open
ticket: 934b2799-2b92-45b1-aaf6-3a9164cafd29
ticket_number: 43
type: feature
created: 2026-06-13
intake: docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md
pipeline_spec: docs/planning/pipeline/active/WORK-map-document-model-v1.spec.md
---

# TICKET-43-oathstar-map-document-model-v1

## Summary

Define the Oathstar-native map document model that `/admin/editor` will edit
and the server will validate/materialize into playable world data.

## Why

The editor should be a first-party Tiled-style tool, not only a wrapper around
external `.tmx` files. Before building canvas painting, we need a canonical
document that represents tiles, rooms, exits, metadata, and placements in terms
the Rust server can validate. Tiled/TMX interop can then target this model
instead of becoming the only authoring path.

## EARS Requirements (candidate — finalize at /pipeline:plan)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The map document model shall represent map identity, size, floor/z-plane, region/subregion metadata, terrain tiles, collision/passability, room cells, exits/stairs, spawn points, and entity/item/fixture references. | Rust test |
| REQ-002 | When a valid document is loaded, the validator shall materialize deterministic room/world data compatible with the existing engine model. | Rust test |
| REQ-003 | When required metadata is missing, a referenced entity/item does not exist, an exit points out of bounds, a room sits on blocked terrain, or no spawn exists, validation shall refuse with a typed error naming the offending cell/ref. | Rust test |
| REQ-004 | The model shall support implicit ordinary rooms and explicit special rooms with stable ids/title/long description overrides. | Rust test |
| REQ-005 | The model shall be serializable as a draft artifact suitable for authenticated save/validate/publish APIs. | serialization test |

## Scope

- In: Rust data model, validation/materialization seams, typed errors, tests,
  docs, compatibility notes for TMX/TMJ import/export.
- Out: editor UI, auth routes, live world mutation, collaborative editing,
  Tiled importer implementation, full world migration.

## Notes

- Forge ticket: 934b2799-2b92-45b1-aaf6-3a9164cafd29 (#43).
- Pipeline: docs/planning/pipeline/active/WORK-map-document-model-v1.spec.md
- Related tickets: #38, #39, #42.
- Related decision: Decision 056.
- This ticket should decide where the model lives (`oathstar-content`,
  `oathstar-protocol`, or a new authoring module) during design.
