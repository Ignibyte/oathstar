---
pipeline_id: ef2e17de-18cd-41a5-9e46-8202ebd2959a
title: WORK-region-subregion-description-authoring-v1
ticket: 341c0863-3fdc-49cd-a438-18a4f5d827f2
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-region-subregion-description-authoring-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-region-subregion-description-authoring-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Region & sub-region **description** authoring (ticket #51 continuation,
  region-program **model-enrichment / S4**) — add an editable `description` to regions
  and sub-regions, authored through **S2's CRUD surface**, persisted via the
  materialize-gated edit seam, and carried through `materialize()` into the world. S2
  shipped id/name/parent CRUD; the ticket's remaining attribute work is "at minimum
  name **and description**", and this slice adds description.
- **Scope:**
  - **In:**
    1. **Model field** — add a `description` to `oathstar_core::RegionDefinition` and
       `SubregionDefinition`, **serde-additive** (`#[serde(default)]`) so existing
       TOML/JSON without it stay valid (default empty). The three struct-literal
       construction sites gain the field; no new validation rule (free text).
    2. **Authoring** — the S2 create forms accept an optional description; an Editor
       can **edit** an existing region's / sub-region's description through the
       regions surface; every mutation still re-validates through `materialize()` and
       persists via the S1 store (no half-write).
    3. **Pass-through** — `materialize()` carries each description unchanged into the
       `WorldDefinition` (it rides `build_world`'s `regions`/`subregions` clone).
    4. **Display** — the per-map region editor shows each description, HTML-escaped.
  - **Out (explicit, → later slices):**
    - The **region-standing consequence system** (Decision 009) and any engine
      behavior from standing — this is authoring-only metadata with no engine consumer
      this slice.
    - A **`standing_default` attribute** — *Design decides (Phase 2)*; planner **leans
      OUT** to keep the model change to one `description` field (see decisions).
    - The **sub-region → tile-editor deep link** (#51c), and editing the **baked
      beginner world** (authoring targets authored `MapDocument`s, per S2's AD).
- **Systems:** engine model (`oathstar-core` region/subregion defs + serde) | content
  (`map_document` materialize pass-through; `map_edit` edit seam) | studio (`render`
  forms + `regions` handlers extend to description).

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an Editor creates a region or sub-region with a description, the studio shall store it on the definition and persist the document. | cargo test (create-with-description round-trips through save/load) |
| REQ-002 | When an Editor edits the description of an existing region or sub-region, the studio shall update only the description — id, name, and parent unchanged — and persist. | cargo test (edit description; other fields intact; round-trips) |
| REQ-003 | A region or sub-region authored without a description shall load and materialize as an empty description, and existing description-less JSON/TOML shall stay valid. | cargo test (serde default; a description-less doc + the beginner TOML load + materialize ok) |
| REQ-004 | When a document is materialized, the engine-bound `WorldDefinition` shall carry each region's and sub-region's description unchanged. | cargo test (materialize; `world.regions[id].description` == authored) |
| REQ-005 | The region editor shall display each region's and sub-region's description, HTML-escaped. | cargo test (render contains the escaped description; a markup-bearing description is escaped) |
| REQ-006 | Every region/sub-region description edit endpoint shall remain Editor-gated — anonymous and Player callers refused. | cargo test (per-route gating, mirroring S2) |
| REQ-007 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **#51 continuation, not a new ticket.** Builds directly on S2 — reuses the `map_edit`
  edit seam, the studio `regions` surface, and the **materialize gate**
  (`AD-claude-authored-map-region-crud-001`). Description never affects
  materialization (free metadata), so the validation boundary is unchanged.
- **`description` is serde-additive metadata on the core defs** (`#[serde(default)]
  description: String`). Backward-compatible: existing TOML/JSON default to empty; the
  three struct-literal sites (`oathstar-core/lib.rs`, `oathstar-content/map_document.rs`,
  `map_edit.rs`) gain the field. It rides `build_world`'s `regions.clone()` /
  `subregions.clone()` into the world automatically — **no `map_document` materialize
  change** beyond the field existing.
- **Authoring-only this slice — no standing engine.** The region-standing consequence
  system (Decision 009) stays OUT.
- **`standing_default` attribute — Design decides (Phase 2).** Whether to also add a
  coarse `standing_default` (per `region-standing.md` / Decision 009 — *defaults only*,
  not the consequence system) is deferred to design; the planner **leans OUT** to keep
  this slice to one `description` field and shippable. If design includes it, it adds a
  matching EARS AC.
- **Branch stacks on S2** — branch off `ticket-51-region-subregion-authoring` (PR #2),
  not `main`/`ticket-53`. The stashed online-first WIP (`stash@{0}`) must not be swept in.
- **Design decides (Phase 2):** the exact edit surface (extend S2's op-dispatched
  `rename` into an `edit` covering name+description, vs. a description field on the
  existing forms / a new op), and whether the field is `String` (default empty) or
  `Option<String>`.

## Linked Artifacts
- Design docs: `docs/region-standing.md` (standing boundary — Decision 009),
  `docs/map-system.md` (world/region model + the regions authoring surface),
  `docs/decisions.md` (009 region standing, 058 studio sidecar). Design re-reads via Explore.
- Intake doc: `docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md`
  (program — region-model enrichment).
- Ticket doc: `docs/planning/tickets/open/TICKET-51-region-subregion-dashboard.md`
- Forge ticket: `341c0863-3fdc-49cd-a438-18a4f5d827f2` (#51, continuation)
- Builds on: S2 `WORK-region-subregion-authoring-v1` (completed, PR #2),
  `AD-claude-authored-map-region-crud-001`; S1 `#53`.

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
