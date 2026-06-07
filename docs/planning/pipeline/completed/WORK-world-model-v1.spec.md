---
pipeline_id: fee30f6d-cf08-421a-895b-83ec9c946b6b
title: WORK-world-model-v1
ticket: f4fe738e-ae33-42c8-8dcd-185fa724afab
type: work
intake:
notes: WORK-world-model-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-world-model-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** First-class world **data model v1** in `oathstar-core` — a
  region/subregion registry rooms reference, a shared typed `Entity` model
  (actors + interactables, role metadata), a typed `Item` model (placement or
  ownership, not inlined into rooms), and reference validation that rejects
  dangling refs. Data shapes + validation + content loading only — no behaviors.
- **Scope:**
  - **In:**
    - `oathstar-core`: a **region/subregion registry** the `WorldDefinition`
      holds; rooms reference a region (and optional subregion) by id (REQ-001).
      Keep the existing `RoomDefinition` metadata — title/description/passable/
      exits/x,y,z/glyph (REQ-002).
    - A **typed `Entity`** model: one representation for NPCs, enemies, and
      special interactables, distinguished by a kind/type + **role metadata**
      (declared roles + their data), with id/name/description/aliases/attributes
      and optional room placement (REQ-003).
    - A **typed `Item`** model in its own registry: placement (in a room) **or**
      ownership (by an entity/the player) modeled by reference, so rooms never
      inline full item state (REQ-004).
    - Extend `WorldDefinition::validate` + `WorldValidationError` to reject a
      world whose rooms/entities/items reference a **missing region, subregion,
      room, entity, or item**, each a typed variant naming the offender (REQ-005).
    - `oathstar-content`: load regions/entities/items from TOML (extend the
      beginner module as needed); duplicate-id + reference checks.
    - Tests for every model + validation branch (≥94% line cov, 100% MSI).
  - **Out:** full combat, shops, NPC memory, **code-behind behavior dispatch /
    hooks**, role-contract metadata validation beyond "the referenced id
    exists", inventory **equipment slots**, item flags/weight/rarity/effects/
    elemental aspects, and any gameplay command that *acts on* entities/items
    (the parser exists; wiring actions to entities is a future ticket).
    `oathstar-protocol` snapshot surfacing of entities/items is **deferred**
    unless Design shows a test needs it (the model + validation are testable in
    `oathstar-core` without snapshot changes).
- **Systems:** engine domain model (`oathstar-core`) + content loader
  (`oathstar-content`) + beginner module TOML (`modules/beginner/`).

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The world model shall represent regions and subregions that rooms reference by id. | Rust test (room→region/subregion resolves) |
| REQ-002 | The room model shall expose title, description, passability, exits, and map-position (x,y,z,glyph) metadata. | Rust test (fields present + carried through load) |
| REQ-003 | The entity model shall represent NPCs, enemies, and special interactables in one typed shape distinguished by role metadata. | Rust test (one type, different roles) |
| REQ-004 | The item model shall support a room placement or an ownership reference without rooms inlining full item state. | Rust test (item placed/owned by reference) |
| REQ-005 | If content references a missing region, subregion, room, entity, or item, then `validate` shall reject the world with a typed error naming the offender. | Rust test per missing-ref variant |
| REQ-006 | The full gate shall pass green, including ≥94% line coverage and 100% mutation MSI over the new model + validation code. | `bin/gate.sh` → GREEN [full] |

## Locked-In Decisions
- **Decisions 003 / 004 / 025 govern** (rooms = described entity-containers;
  entities = shared data + role contracts + code-behind behaviors, NPCs/enemies
  are actors; square-grid map + passability). v1 implements the **data shapes**;
  the code-behind behavior layer and full role contracts are later tickets.
- **v1 = data model + validation only, minimal shapes.** Each new type is the
  smallest shape that satisfies its EARS + reference validation. Every added
  field/variant is mutation surface that needs a killing test — keep it lean.
- **Items are not inlined into rooms** — items live in their own registry and are
  attached by reference (room placement or owner id). The exact representation
  (separate `Item` struct vs an `Entity` of kind Item) is a **Phase 2 design
  decision**, informed by `entity-model.md` (Item is listed as an entity type)
  vs the separate REQ-003/REQ-004 ACs.
- **Reuse and extend** the existing `WorldValidationError` / `validate()` pattern
  (typed errors that name the offender, rejected at the construction boundary —
  the ticket #2 / #6 style), not a parallel validation path.
- **One cohesive pipeline, not split** — the models share `validate()`, the
  content loader, and `WorldDefinition`; splitting would re-touch the same files.

## Linked Artifacts
- Design docs: `docs/entity-model.md`, `docs/inventory-and-items.md`, `docs/map-system.md`, `docs/module-system.md`, `docs/mechanics-and-systems.md` (Rooms And World Model / Entity Model), `docs/decisions.md` (003/004/025)
- Existing code: `crates/oathstar-core/src/lib.rs` (`WorldDefinition`/`RoomDefinition`/`WorldValidationError`/`validate`), `crates/oathstar-content/src/lib.rs` (loader), `modules/beginner/{module,rooms}.toml`
- Intake doc: none (ticket pre-existed)
- Ticket doc: `docs/planning/tickets/open/TICKET-6-model-rooms-regions-entities-and-items-v1.md`
- Forge ticket: `f4fe738e-ae33-42c8-8dcd-185fa724afab` (#6)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
