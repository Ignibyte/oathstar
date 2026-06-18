---
pipeline_id: 592f5ece-4cfd-4578-98f5-eba26d6867c6
title: WORK-region-subregion-authoring-v1
ticket: 341c0863-3fdc-49cd-a438-18a4f5d827f2
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-region-subregion-authoring-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-region-subregion-authoring-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Region & sub-region authoring (ticket #51 continuation, region-program
  **S2**) — Editor-gated **create / edit / delete** of regions and sub-regions in an
  **authored `MapDocument`**, persisted through S1's save/load loop (#53) and kept
  valid through the existing `materialize()` boundary. Slice 1 of #51 shipped the
  read-only dashboard; this adds the editing surface, now unblocked because S1 gave
  the studio a persistence layer.
- **Scope:**
  - **In:**
    1. **Region CRUD** — create (id + name), rename, delete a region in an authored
       `MapDocument` (`map_document.rs` `regions: BTreeMap`).
    2. **Sub-region CRUD** — create (id + name + parent region), rename, delete a
       sub-region (`subregions: BTreeMap`); create requires an existing parent.
    3. **Persist via S1** — every mutation re-validates through `materialize()` and
       saves the document via the S1 `FileSaveStore` maps store; an op that would
       break materialization is refused with a typed error (no half-write).
    4. **Editor-gated surface** — server-rendered forms/handlers (Decision 058
       loopback + Editor role), consistent with the existing `sections`/`render`
       pattern and S1's `editor_refusal` gate.
  - **Out (explicit, → later slices):**
    - **Description / richer region attributes** (standing defaults per
      `region-standing.md`) — that is **S4** (model enrichment); this slice edits
      `id`/`name`/parent only.
    - Per-sub-region **map identity** + the tile-editor deep link (#51c remainder),
      room authoring, entities/items/oaths authoring, the content **purge** (S5).
    - Editing the **baked beginner world** — authoring operates on an authored
      document, not the read-only seed (the seed is replaced/purged later).
- **Systems:** studio (region/subregion CRUD handlers + forms + dashboard) | content
  (`MapDocument` regions/subregions + `materialize` validation) | storage (reuse S1's
  `FileSaveStore`)

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an Editor creates a region with an id and name in an authored map, the studio shall add it to the `MapDocument` and persist the document, refusing a duplicate region id. | cargo test (create adds + persists; duplicate id → typed refusal) |
| REQ-002 | When an Editor creates a sub-region with an id, name, and parent region, the studio shall add it only if the parent region exists (else refuse), and persist. | cargo test (valid parent → added; unknown parent → refused) |
| REQ-003 | When an Editor renames a region or sub-region, the studio shall update the `MapDocument` and persist. | cargo test (rename round-trips through save/load) |
| REQ-004 | When an Editor deletes a region or sub-region that is still referenced (a room's region/subregion, or a region with child sub-regions), the studio shall refuse and leave the document unchanged; an unreferenced one shall be removed and persisted. | cargo test (referenced → refused, doc intact; unreferenced → removed) |
| REQ-005 | Every region/sub-region CRUD endpoint shall be reachable only by an Editor — anonymous and Player callers are refused (401/403 or redirect). | cargo test (per-route gating, mirroring S1) |
| REQ-006 | Before persisting any CRUD result, the studio shall confirm the `MapDocument` still `materialize()`s; a mutation that would break it is rejected with a typed error and not written. | cargo test (a break-inducing op → typed error, no write) |
| REQ-007 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **Authoring targets an authored `MapDocument`, not the baked world** — CRUD edits
  the editable document's `regions`/`subregions` and persists via S1; the read-only
  beginner seed is replaced/purged in a later slice (owner's purge-after-replace).
- **`materialize()` is the validation boundary** — reuse its `RoomRegionMissing` /
  `RoomSubregionMissing` / duplicate checks; never persist a non-materializable doc.
- **Attributes = id + name + parent only this slice** — description/standing
  defaults are S4; keeps S2 focused and avoids a model change.
- **Editor-gated + loopback** (Decision 058); **logic stays out of `main`** in
  testable helpers (PR-claude-extract-main-logic-for-mutation-coverage-001); render
  assertions test element/name forms (PR-claude-assert-element-form-not-substring-001);
  every gated route refuses Player + anon (PR-claude-gated-page-role-mutant-001).
- **Reuse S1** (`FileSaveStore`, `editor_refusal`, the `/editor/maps` persistence) —
  no new storage; compose, don't duplicate.
- **Branch off `ticket-53`** (has S1). The stashed online-first WIP (`stash@{0}`)
  must not be swept in.
- **Design decides (Phase 2):** the exact surface (extend the `/regions` dashboard
  with forms over a selected authored map vs. editor-side controls), which authored
  document the dashboard targets, and the precise endpoint/route shapes.

## Linked Artifacts
- Design docs: `docs/region-standing.md` (future attributes — S4), `docs/map-system.md`
  (world/region model), `docs/decisions.md` (058 studio sidecar). Design re-reads via Explore.
- Intake docs: `docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md`
  (program, slice S2) + `docs/planning/intake/INTAKE-studio-admin-and-world-model-program.md` (#51 origin).
- Ticket doc: `docs/planning/tickets/open/TICKET-51-region-subregion-dashboard.md`
- Forge ticket: `341c0863-3fdc-49cd-a438-18a4f5d827f2` (#51, continuation)
- Builds on: S1 `WORK-persist-authored-worlds-v1` (#53, committed `c1f96cd`),
  `AD-claude-authored-world-persistence-001`.

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
