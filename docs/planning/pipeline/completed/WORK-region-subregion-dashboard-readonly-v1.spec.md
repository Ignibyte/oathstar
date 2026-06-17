---
pipeline_id: 19d400ef-3dac-47a1-89c5-dd2908735f89
title: WORK-region-subregion-dashboard-readonly-v1
ticket: 341c0863-3fdc-49cd-a438-18a4f5d827f2
type: work
intake: docs/planning/intake/INTAKE-studio-admin-and-world-model-program.md
notes: WORK-region-subregion-dashboard-readonly-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-region-subregion-dashboard-readonly-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** The region & sub-region dashboard (ticket #51, **slice 1 of N**) — a
  **read-only** Regions section that replaces the #49 stub: lists every region with
  its sub-regions (hierarchy) + room counts, each sub-region linking to the editor.
  Built in the #49 nav shell + the #50 fantasy theme.
- **Scope:**
  - **In:**
    1. **`StudioState` gains `world: Arc<WorldDefinition>`** — loaded once at startup
       (`load_beginner_world()?` in `main()`, alongside the catalog; the `?` keeps
       the handler path unwrap-free).
    2. **A real `sections::regions` handler** (replaces the stub) — Editor-gated,
       renders `render::regions_page(&studio.world)`. `items`/`enemies`/`settings`
       stay stubs.
    3. **`render::regions_page(world)`** — the shared header (`Regions` active) +
       theme, listing each region (deterministic `BTreeMap` order) with its
       sub-regions (`subregion.region == region.id`) nested, a **room count** per
       region + per sub-region (from `world.rooms`), and each sub-region linking to
       `/editor`.
  - **Out (explicit, → later #51 slices):**
    - **#51b** — editing region/sub-region attributes + **persistence** (the studio
      has no storage layer; wiring `oathstar-storage` is its own slice).
    - **#51c** — per-sub-region **map identity** (the editor serves one embedded
      `STARTER_DOC`, so links go to the general `/editor` for now); create/delete.
    - Richer region attributes (description, standing — `region-standing.md`).
- **Systems:** ui (studio regions dashboard) | content (reads `WorldDefinition`)

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an Editor opens `/regions`, the studio shall render a page listing every region in the loaded world, each with its sub-regions nested beneath its parent region. | render test: the 3 region names (Hollowmere / The Ashen Road / The Old Bell Tower) + the 5 sub-region names appear, sub-regions grouped under their region |
| REQ-002 | The dashboard shall show a room count for each region and each sub-region, derived from the world's rooms. | render test asserts a count rendered against a known region/sub-region |
| REQ-003 | Each sub-region shall present a link to the tile-map editor (`/editor`). | render test asserts a sub-region row carries `href="/editor"` |
| REQ-004 | When a request without a valid Editor session reaches `/regions`, the studio shall redirect to `/login` and not serve the page. | handler tests: a Player session AND no session both redirect (PR-claude-gated-page-role-mutant-001) |
| REQ-005 | The navigation shall mark **Regions** active on the dashboard page. | render test asserts `aria-current="page"` on the Regions link (element form) |
| REQ-006 | The full gate shall stay green with mutation 100% MSI (the handler + `regions_page` `format!` markers pinned by the render + gate tests). | `bin/gate.sh` FULL |

## Locked-In Decisions
- **The world is loaded once at startup** into `StudioState.world: Arc<WorldDefinition>`
  (via `load_beginner_world()?` in `main()`, like the catalog) — the dashboard's
  data source. No per-request load; no `unwrap`/`expect` on the handler path.
- **Read-only this slice.** Editing + persistence + per-sub-region map identity are
  named deferred slices (above).
- **Data from `WorldDefinition`** — `regions`/`subregions` are `BTreeMap`s (sorted,
  deterministic render); room counts group `world.rooms` by `region` + `subregion`.
- **Replaces the `sections::regions` stub**; the other three sections stay stubs.
- The `studio()` **test helper** must gain a `world` (load the beginner world or a
  small fixture) now that `StudioState` carries one — design decides.
- **Heed** `PR-claude-gated-page-role-mutant-001` (Player + anon redirect tests) and
  `PR-claude-assert-element-form-not-substring-001` (assert specific element/name
  forms in the render markers, since `STUDIO_CSS` is embedded).

## Linked Artifacts
- Design docs: `docs/region-standing.md` (future region attributes), `docs/map-system.md`
  (the world/region model). Note at complete if warranted.
- Intake doc: `docs/planning/intake/INTAKE-studio-admin-and-world-model-program.md`
- Ticket doc: `docs/planning/tickets/open/TICKET-51-region-subregion-dashboard.md`
- Forge ticket: `341c0863-3fdc-49cd-a438-18a4f5d827f2` (#51)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
