---
pipeline_id: 4a124b99-ec4e-48af-8635-f65a80e6ca4a
title: WORK-regions-table-v1
ticket: 76bcc9e4-3205-4118-babd-15d114171bda
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-regions-table-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-regions-table-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`.

## Work Spec
- **Title:** The `/regions` dashboard as a styled, **searchable + sortable table** — item ③
  (the last of the owner's authoring-loop plan).
- **Today:** `regions_list_page` (`render.rs:198`) renders each `MapSummary`
  (`id`/`title`/`region_count`/`subregion_count`) as a `.panel` card with an "Edit regions"
  `.cta` link; `STUDIO_CSS` has no table styles; the regions pages carry no inline JS.
- **Scope:**
  - **In (the `/regions` dashboard):**
    1. **`render.rs` `regions_list_page`** — render the maps as a **semantic `<table>`**:
       a header row (Title, Id, Regions, Sub-regions, action) with sortable `<th>`
       (`aria-sort`, a `data-key`), and one `<tr>` per map carrying its data on `data-*`
       attributes + the "Edit regions" link in the action cell. A **labeled search input**
       above the table. The empty store keeps the "create one" prompt unchanged.
    2. **NEW `static/regions-table.js`** (pure, node-tested): `filterRows(rows, query)`
       (case-insensitive substring over title+id; empty query → all) + `sortRows(rows, key,
       dir)` (string compare for `title`/`id`, numeric for `regions`/`subs`; `asc`/`desc`;
       **stable**).
    3. **Inline `REGIONS_GLUE` seam** — reads the rendered `<tr>` records (from `data-*`),
       applies `filterRows`/`sortRows` on search-input + header-click, and shows/reorders the
       `tbody`. `include_str!` the module + glue const into the page (the `editor-canvas.js` +
       `EDITOR_GLUE` pattern).
    4. **`studio.css`** — table styles using the #50 theme tokens (`--brass`, `--muted`, …);
       sortable-header affordance.
  - **Out (explicit):** the `/regions/{id}` **region editor's** region/sub-region lists (a
    follow-on); pagination; column show/hide; persisting the sort/filter; **server-side**
    search; any backend/protocol change.
- **Systems:** studio client only — `render.rs` (`regions_list_page` + `REGIONS_GLUE`) +
  `static/regions-table.js` (pure) + `static/studio.css` + tests (`node --test` + Rust render).

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | `regions_list_page` shall render the authored maps as a semantic `<table>` — a header row and one `<tr>` per map carrying its `title`/`id`/counts (on `data-*`) plus the per-map "Edit regions" link; an empty store shall keep the "create one" prompt. | cargo test (the `<table>`, a data row, the empty-state) |
| REQ-002 | `filterRows(rows, query)` shall return the rows whose `title` or `id` contains `query` (case-insensitive); an empty/blank query shall return all rows. | node --test |
| REQ-003 | `sortRows(rows, key, dir)` shall return the rows sorted by `key` — lexicographic for `title`/`id`, numeric for `regions`/`subs` — ascending or descending, preserving input order among equal keys (stable). | node --test |
| REQ-004 | The dashboard shall include a labeled search input and sortable column headers (`th` with `aria-sort`), and load the inline seam that wires them to `filterRows`/`sortRows`. | cargo test (the search input + `th`+`aria-sort` + the page references `filterRows(`/`sortRows(`) |
| REQ-005 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **`/regions` dashboard only** this slice; the region editor's lists are a follow-on.
- **Client-side search/sort** (recommended): snappy for a few maps; keeps the pure-helper +
  thin-seam split. The table is **server-rendered**; the glue filters (show/hide) + sorts
  (reorder the `tbody`) the existing rows. A server-side `?q=`/`?sort=` re-render is the
  out-of-scope alternative.
- **Pure `filterRows`/`sortRows` in a new `static/regions-table.js`** (node-tested, like
  `editor-canvas.js`); the glue (`REGIONS_GLUE` const) is the smoke-/review-verified seam — no
  new Rust mutation surface (the render fn + glue const; viable render mutants die on REQ-001/004).
- **Accessible + themed:** semantic `<table>`, `th[aria-sort]`, a labeled search `<input>`;
  reuse the #50 theme tokens; the empty-state prompt unchanged.
- Render assertions test element/call forms (`PR-claude-assert-element-form-not-substring-001`);
  author data (`title`/`id`) stays `escape_html`'d in the cells + `data-*` attributes.
- **Branch off `main`** (`6fd1ad6`); stash (`stash@{0}`) stays parked.
- **Design (Phase 2) decides:** the exact columns/`data-*` shape, how the glue reads rows
  (record `{el, title, id, regions, subs}`), the sort-direction toggle UX, and the CSS specifics.

## Linked Artifacts
- Design docs: `docs/map-system.md` (the regions surface). Design re-reads.
- Intake / plan: `docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md`;
  memory `studio-authoring-next-phase` (item ③, last). Builds on #51 (the dashboard), #50 (theme).
- Ticket doc: `docs/planning/tickets/open/TICKET-58-regions-table.md`
- Forge ticket: `76bcc9e4-3205-4118-babd-15d114171bda` (#58).

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
