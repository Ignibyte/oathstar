---
pipeline_id: c47a48d1-7e6a-483c-bf33-16610ad29411
title: WORK-persist-authored-worlds-v1
ticket: 9d39d561-de36-494a-93b5-cb2b7ce81698
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-persist-authored-worlds-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-persist-authored-worlds-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Persist authored worlds and load them in the game (region-authoring
  program, slice **S1** — the keystone authoring loop). The studio **saves** an
  authored `MapDocument` (editable source of truth, JSON) and can **reopen** it;
  the game **loads** an authored world at startup (opt-in via config) through the
  existing `MapDocument::materialize()` path. Closes the round-trip:
  author → save → restart pointed at it → play. **No purge** this slice (S5).
- **Scope:**
  - **In:**
    1. **Studio persistence** — save / list / load endpoints for `MapDocument`,
       writing JSON into an owner-owned content dir; the editor can reopen a saved
       doc. **Editor-gated + loopback** (Decision 058).
    2. **Path-safe storage** — reuse `oathstar-storage`'s `FileSaveStore`
       path/symlink/reserved-name safety + atomic-write posture; writes stay inside
       the owned dir.
    3. **Game runtime load** — the server, when an authored-world path is configured
       (env/config), loads that `MapDocument` → `materialize()` → `WorldDefinition`
       and serves it; with no config it loads the baked beginner world unchanged.
       The loaded file is **untrusted input** (typed errors, re-validate, no panics,
       loud rejection — the `WORK-save-load-v1` posture).
    4. Tests for all of the above.
  - **Out (explicit):** region/sub-region CRUD UI (S2); map visuals / retiring
    flat-colors (S3); region-model attribute enrichment (S4); content purge +
    replacement (S5); authoring of entities/items/oaths (stay TOML for now).
- **Systems:** studio (persistence endpoints + editor reopen) | content
  (`MapDocument` persistence; reuse `materialize()`) | engine/server (runtime
  authored-world load) | storage (reuse `FileSaveStore` safety)

## Acceptance Criteria (EARS)
Each criterion uses EARS syntax, one observable behavior, with a verification method.

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the owner saves an authored map, the studio shall persist the `MapDocument` as JSON in the owned content dir and return its identifier. | cargo test (save round-trips a document to disk; returns id) |
| REQ-002 | When the owner lists or opens saved maps, the studio shall return the persisted `MapDocument` so it reopens in the editor unchanged. | cargo test (list returns saved ids; load returns the byte-faithful document) |
| REQ-003 | The save/list/load endpoints shall be reachable only by an Editor and reject anonymous/Player callers. | cargo test (Editor allowed; anon/Player redirected/denied — mirrors existing studio gate tests) |
| REQ-004 | The persistence layer shall reject path-unsafe identifiers (traversal, absolute, symlink escape, reserved names) and write only within the owned dir. | cargo test (traversal/symlink/reserved refused — reuse `FileSaveStore` safety) |
| REQ-005 | When the server starts with an authored-world path configured, the engine shall load that `MapDocument`, materialize + re-validate it, and serve it; with no config it shall load the baked beginner world. | cargo test (configured → authored world served; default → beginner) |
| REQ-006 | When loading an authored world from a malformed or invalid document, the engine shall fail with a typed error (loud rejection) and never panic, leaving startup behavior defined. | cargo test (malformed JSON / invalid doc → typed error, no panic) |
| REQ-007 | The baked beginner world and its existing tests shall be unaffected (beginner stays the default; nothing purged). | existing cargo/node test suites stay green |
| REQ-008 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **Persist the EDITABLE `MapDocument` (JSON), not materialized TOML** — so it
  round-trips back into the editor (owner-confirmed 2026-06-17).
- **Game loads at startup via the existing `MapDocument::materialize()`** — no
  duplicate world-building; opt-in by config/env; **beginner stays default; no
  purge** (purge is S5, last).
- **Reuse `oathstar-storage` `FileSaveStore`** path/symlink/reserved-name safety +
  atomic write — do not reinvent storage (the save-slot hardening precedent).
- **Load = untrusted input** (`WORK-save-load-v1` / TICKET-2 posture): typed
  errors, re-validate through `materialize()` → `WorldDefinition::validate()`, no
  panics, loud version/format rejection.
- **Studio save surface is Editor-gated + loopback** (Decision 058); writes only
  within an owned content dir.
- **Only the `MapDocument` persists this slice** (terrain/rooms/regions/subregions/
  tilesets/layers). Entities/items/oaths authoring is OUT (stays TOML).
- **Branch + WIP hygiene** — a dedicated `ticket-53-persist-authored-worlds`
  branch is created before implement; the online-first WIP stays stashed
  (`stash@{0}`) and must not be swept in.

## Linked Artifacts
- Design docs: `docs/module-system.md` (local file loading, core-validated, no
  hot-load), `docs/technical-architecture.md` (core engine + swappable worlds),
  `docs/decisions.md` (058 studio sidecar/loopback/Editor-gate, 057 auth seam).
- Intake doc: `docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md`
  (program intake — this is slice **S1**).
- Ticket doc: `docs/planning/tickets/open/TICKET-53-persist-authored-worlds.md`
- Forge ticket: `9d39d561-de36-494a-93b5-cb2b7ce81698` (#53)
- Precedent: `WORK-save-load-v1` (reuse `FileSaveStore`), TICKET-2 (core validates
  any `WorldDefinition`).

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
