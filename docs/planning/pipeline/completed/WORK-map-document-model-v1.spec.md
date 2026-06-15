---
pipeline_id: 514032f4-c923-4ec6-b1bc-8764943e28e4
title: WORK-map-document-model-v1
ticket: 934b2799-2b92-45b1-aaf6-3a9164cafd29
type: work
intake: docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md
notes: WORK-map-document-model-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-map-document-model-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Oathstar map document model v1 — a renderer-agnostic authoring
  document on a 16×16-px source grid, with `validate` + `materialize` seams that
  turn a draft into engine-compatible world data.
- **Scope:**
  - **In:** a Rust map-document data model (map identity, grid size, floor/z-plane,
    region/subregion metadata, terrain tiles, collision/passability, room cells,
    exits/stairs, spawn points, entity/item/fixture references); a `validate` seam
    returning typed errors that name the offending cell/ref; a `materialize` seam
    producing deterministic engine-compatible room/world data; serde draft
    serialization (renderer-agnostic, serde-additive); tests + docs; TMX/TMJ
    interop **notes** only.
  - **Out:** the `/admin/editor` UI; auth routes / save-validate-publish APIs; the
    `.tmx` importer implementation (#39); per-room biome colors over the wire (#38);
    new world content (#40); live world mutation; collaborative editing; full world
    migration; ANY player-facing rendering change (the runtime `MapSnapshot` DTO and
    #37's 32px-source/64px-display render scale are untouched).
- **Systems:** content / map / engine data model (Rust). No UI, server-route, or
  protocol-wire change.

## Acceptance Criteria (EARS)
| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The map document model shall represent map identity, grid size, floor/z-plane, region/subregion metadata, terrain tiles, collision/passability, room cells, exits/stairs, spawn points, and entity/item/fixture references. | Rust test (construct + field/round-trip assertions) |
| REQ-002 | The map document model shall express tile geometry in 16×16-pixel source units (the authoring grid unit), independent of any client render scale. | Rust test (tile unit is 16; materialized geometry carries 16px source; no render-scale field) |
| REQ-003 | When a valid document is materialized, the seam shall produce deterministic room/world data compatible with the existing engine room model (ids, x/y/z coordinates, exits, passability). | Rust test (materialize → engine-compatible structures; same input → byte-identical output) |
| REQ-004 | When required metadata is missing, a referenced entity/item/fixture is unknown, an exit targets a non-existent or out-of-bounds room, a room cell sits on non-passable terrain, or no spawn point exists, validation shall refuse with a typed error naming the offending cell/reference. | Rust test (one case per failure class → distinct typed-error variant) |
| REQ-005 | The map document model shall support implicit ordinary rooms and explicit special rooms with stable ids and title/long-description overrides. | Rust test |
| REQ-006 | The map document shall round-trip losslessly through its serialized draft form (serde) with no renderer-specific fields. | Rust serde round-trip test |

## Locked-In Decisions
- **16×16-px tiles are the authoring/source grid unit** (owner steer; the
  painted-16px target feel — intake "Tile Art Direction"). Decoupled from the
  client render scale; #37's 32px-source/64px-display path is unchanged and out of
  scope. The name-keyed tileset keeps source resolution independent of map/world
  data.
- **Renderer-agnostic + serde-additive** (Decisions 025/035; map-system.md
  "Backend Payload" / "Design Guardrails"). The document is a server-side draft
  artifact suitable for later authenticated save/validate/publish APIs.
- **Square grid, six directions** (N/S/E/W/Up/Down), no diagonals (map-system.md).
- **The document model is NEW and distinct from the runtime `MapSnapshot` DTO**
  (`oathstar-protocol` src/lib.rs:251–284). It *materializes into* engine/world
  data; it is not the wire snapshot. No change to `MapSnapshot`/`MapRoomSnapshot`.
- **Deterministic materialization** — same document in → identical world data out.

## Design Decisions (resolved Phase 2 — see notes)
- **Home crate:** `oathstar-content` (already content→`WorldDefinition`; in-crate tests).
- **Serialized form:** JSON via serde (renderer-agnostic, additive).
- **Grid representation:** sparse `BTreeMap<Cell, _>` within declared `width/height/floors`.
- **Materialize target:** `oathstar_core::WorldDefinition` — rooms 1:1 with room cells;
  entities/items from a provided `ContentCatalog`; `oaths` empty in v1.
- **Seams:** `MapDocument::validate(&self, &ContentCatalog)` + `materialize(..) ->
  WorldDefinition` (validate-then-build); `MapValidationError` hand-rolled
  (`Display`+`Error`, not thiserror). Full design + file manifest + test plan in notes.

## Linked Artifacts
- Design docs: docs/map-system.md; docs/decisions.md (025, 035, 050, 051, 056);
  docs/spatial-awareness.md; docs/ui-design.md (Map Direction).
- Intake doc: docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md
- Ticket doc: docs/planning/tickets/open/TICKET-43-oathstar-map-document-model-v1.md
- Forge ticket: 934b2799-2b92-45b1-aaf6-3a9164cafd29 (#43)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
| 3 — Implement | — |
| 3.5 — Inspect | — |
| 4 — Validate | — |
| 5 — Complete | — |
