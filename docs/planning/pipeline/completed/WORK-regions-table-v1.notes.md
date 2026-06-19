# WORK-regions-table-v1 — Notes

## Phase 1 — Plan
- **Request:** item ③ (last) — the `/regions` dashboard as a styled, searchable, sortable
  table. Memory `studio-authoring-next-phase` item ③.
- **Classification / tier:** work pipeline, one slice, **studio client only** — `render.rs`
  (`regions_list_page` + `REGIONS_GLUE`) + new `static/regions-table.js` (pure) + `studio.css`
  + tests. No backend/protocol/Rust-logic change.
- **Recon (working tree):**
  - `regions_list_page` (`render.rs:198`) — each `MapSummary` (`id`/`title`/`region_count`/
    `subregion_count`) → a `.panel` card (`<h2>title</h2>` · "N regions · M sub-regions" · an
    "Edit regions" `.cta` link); empty store → "create one" prompt.
  - `STUDIO_CSS` has `.panel`/`.cta`/`.soon`/theme tokens (`--brass`,`--muted`) — **no `<table>`
    styles**. Regions pages have **no inline JS** (unlike `editor_page`'s `EDITOR_GLUE`).
  - Embed pattern (`render.rs:585`): `<script type="module">{editor_js}{EDITOR_GLUE}</script>`,
    `editor_js = include_str!("../static/editor-canvas.js")` — `regions-table.js` mirrors this.
- **Decision (surfaced):** CLIENT-side search/sort (recommended — snappy for a few maps, keeps
  the pure-helper/seam split) vs server `?q=`/`?sort=` re-render (out).
- **EARS:** REQ-001 semantic table + empty-state · REQ-002 `filterRows` · REQ-003 `sortRows`
  (string/numeric, stable) · REQ-004 search input + sortable `th` + glue wiring · REQ-005 gate.
- **No new Rust mutation surface:** `regions_list_page` render fn + `REGIONS_GLUE` const; the
  testable logic is the pure JS (`filterRows`/`sortRows`, node-covered).
- **Ticket:** forge **#58** `76bcc9e4-3205-4118-babd-15d114171bda` (NOT #55/#56/#57).
  Local doc `docs/planning/tickets/open/TICKET-58-regions-table.md`.
- **aar_id:** `d2f9051c-5ae8-405d-900e-5d53274400d3`
- **Delivery:** goal-driven autonomous — plan→complete then commit + push + FF-merge to `main`.
  Branch off `main` `6fd1ad6`. Stash parked. **This is the last item — the goal completes here.**

## Phase 2 — Design

### Code reconnaissance
- `studio.css :root` tokens: `--bg --panel --line --ink --muted --brass --danger` — the table
  CSS reuses these (#50 theme). Embed pattern (`render.rs:585`): `<script type="module">{js}{GLUE}
  </script>` via `include_str!`.
- Existing tests: `regions_list_page_lists_maps_with_counts_and_links` (`:753`) asserts the
  **panel** markup (`<h2>`, "N regions · M sub-regions") → **must be updated to the table** (T3);
  `regions_list_page_shows_an_empty_state` (`:781`) asserts "No authored maps yet" + no `.cta`
  → **unchanged** (the table only renders for a non-empty store).

### Approach / architecture (studio client only)
1. **NEW `static/regions-table.js` (pure):**
   - `filterRows(rows, query)` — `q = String(query||"").trim().toLowerCase()`; `q===""` →
     `rows.slice()` (all); else `rows.filter(r => \`${r.title} ${r.id}\`.toLowerCase().includes(q))`.
   - `sortRows(rows, key, dir)` — `const NUMERIC = new Set(["regions","subs"]); const sign = dir
     === "desc" ? -1 : 1; return rows.slice().sort((a,b) => sign * (NUMERIC.has(key) ?
     Number(a[key])-Number(b[key]) : String(a[key]).localeCompare(String(b[key]))));` — a copy;
     **stable** via `Array.prototype.sort` (ES2019+) so equal keys keep input order in both dirs.
   - Rows are plain records `{title,id,regions,subs[,el]}` — the helpers read only the data
     fields (`el` ignored); `regions`/`subs` may be strings (from `data-*`) — `Number()` coerces.
2. **`render.rs` `regions_list_page`** → a `.panel` with a labeled search `<input id="regions-search"
   type="search">` + a semantic `<table class="regions-table">`: `<thead>` with sortable headers
   `<th aria-sort="none"><button type="button" data-key="title">Title</button></th>` for
   Title/Id/Regions/Sub-regions + a plain `Actions` `<th>`; `<tbody>` one `<tr data-title=".."
   data-id=".." data-regions=".." data-subs="..">` per map with `escape_html`'d cells + the
   `<a class="cta" href="/regions/{id}">Edit regions</a>` action. Empty store → the unchanged
   "create one" prompt. Embed `<script type="module">{regions_js}{REGIONS_GLUE}</script>`.
3. **`REGIONS_GLUE` const** — builds records from the `tbody` `<tr>` `dataset` (+ `el`); on
   search `input` → `filterRows` (hide non-matching via `el.hidden`); on header `button` click →
   toggle `sortDir`, `sortRows` the visible rows, re-`appendChild` into the `tbody`, and set the
   header's `aria-sort` (`ascending`/`descending`, others `none`).
4. **`studio.css`** — `.regions-table`, `thead/th button`, row striping/hover, `.table-search`
   using the tokens.

### Locked decisions (this phase)
- **Client-side** search/sort; the table is server-rendered; the glue filters (hide) + sorts
  (reorder). Pure `filterRows`/`sortRows` (node-tested); `REGIONS_GLUE` is the smoke seam.
- **`sortRows` relies on stable `Array.sort`** (ES2019+) + returns a copy; numeric keys coerced
  via `Number()`; string keys via `localeCompare`.
- **Sortable `<th>` = a `<button data-key>` inside `th[aria-sort]`** (keyboard-accessible); the
  glue owns the dir toggle + `aria-sort`.
- `data-*` + cells `escape_html`'d (author `title`/`id`); `dataset` decodes back for the helpers.
- Empty state unchanged. No new Rust mutation surface (render fn + glue const; T3/T4 kill any
  viable render mutant; the JS logic is node-covered).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-studio/static/regions-table.js` | **NEW** pure `filterRows` + `sortRows`. |
| 2 | `crates/oathstar-studio/src/render.rs` | `regions_list_page` → search input + semantic sortable `<table>` (rows carry `data-*`) + embed module/glue; new `REGIONS_GLUE` const. |
| 3 | `crates/oathstar-studio/static/studio.css` | `.regions-table` + sortable-header + `.table-search` styles (theme tokens). |
| 4 | `tests/regions-table.test.js` | **NEW** node: `filterRows` + `sortRows`. |
| 5 | `crates/oathstar-studio/src/render.rs` (`#[cfg(test)]`) | **update** `regions_list_page_lists_…` → table assertions; add a glue/search/`aria-sort` test. |
| 6 | `docs/map-system.md` | (Phase 5) note the regions table. |

### Regression Test Plan
| # | Test | Proves |
|---|---|---|
| T1 | `filterRows` — matches on title; on id; case-insensitive; `""`/whitespace → all; no match → `[]`; returns a copy | REQ-002 (node) |
| T2 | `sortRows` — title asc/desc (lexicographic); regions asc/desc (numeric, not lexicographic e.g. `10` after `9`); **stable** on equal keys (input order kept) in both dirs; returns a copy | REQ-003 (node) |
| T3 | `regions_list_page(maps)` contains `<table`, a `<tr data-title="First" data-id="m1" data-regions="2" data-subs="1">` with the `<td>` cells + `<a class="cta" href="/regions/m1">Edit regions</a>`; the empty store still shows "No authored maps yet" + no `.cta` | REQ-001 (cargo; replaces the panel test) |
| T4 | the page has `<input id="regions-search"`, `aria-sort=` on the headers, and references `filterRows(` + `sortRows(` (the embedded module+glue) | REQ-004 (cargo) |
| G1 | `bin/gate.sh` FULL green, MSI 100% | REQ-005 |
- Coverage: `filterRows`/`sortRows` fully exercised by T1/T2 (new test-imported module). No Rust
  delta → no new mutants. **Uncoverable:** `REGIONS_GLUE` runtime (browser-only) — pinned by
  T4's render-string assertions + smoke.

### Risks / decisions
1. **Test update** — the panel-markup dashboard test becomes the table test (T3); the empty-state
   test is unchanged.
2. **Escaping** — `data-*` + cells `escape_html`'d; a title with `"`/`<` stays attribute-safe and
   `dataset` decodes it for `filterRows`.
3. **numeric sort** — `regions`/`subs` from `data-*` are strings; `Number()` coercion avoids a
   lexicographic `"10" < "9"` bug (T2 guards it).
4. **Glue seam** — all decision logic is the pure `filterRows`/`sortRows` (node); the DOM wiring
   is render-string-asserted (T4) + smoke.

## Phase 3 — Implement
- **Built to the manifest** (new tests Phase 4):
  - **NEW `static/regions-table.js`** — pure `filterRows(rows, query)` (trim/lowercase; `""`→
    `slice()`; else substring over `` `${title} ${id}` ``) + `sortRows(rows, key, dir)` (copy;
    `NUMERIC_KEYS={regions,subs}` via `Number()`, else `localeCompare`; `sign` for dir; stable).
  - `render.rs` `regions_list_page` — non-empty store now renders a `.panel` with a labeled
    search `<input id="regions-search" type="search">` + `<table class="regions-table">`
    (sortable `<th aria-sort="none"><button data-key=…>` headers + an Actions `<th>`; `<tbody>`
    one `<tr data-title data-id data-regions data-subs>` per map, `escape_html`'d cells + the
    `.cta` Edit link); embeds `<script type="module">{regions_js}{REGIONS_GLUE}</script>` via
    `include_str!`. New `REGIONS_GLUE` const (records from `dataset`; search→`filterRows` hide;
    header click→toggle dir + `sortRows` reorder + `aria-sort`). Empty store prompt unchanged.
  - `static/studio.css` — `.regions-table`/`.table-search`/sortable-`th` (`::after` ▲/▼) using
    the `:root` tokens.
- **No backend/protocol/Rust-logic change.**
- **Verified:** `cargo fmt`; `clippy -p oathstar-studio --all-targets` clean; `regions-table.js`
  behaves (`filter "AL"`→`[a]` case-insensitive; blank→all; `sort regions asc`→`["9","10"]`
  **numeric** not lexicographic; `sort title`→`["alpha","Beta"]` locale). studio render **20** +
  node **20** pass.
- **Deviation:** updated the existing `regions_list_page_lists_…` test to the **table** markup
  now (the panel markup is gone) so the suite stays green through inspect — the *new* node
  (`filterRows`/`sortRows`) + glue assertions are Phase 4.
- **For Phase 4:** `tests/regions-table.test.js` (T1 `filterRows`, T2 `sortRows`) + a `regions_list_page`
  glue/search/`aria-sort` test (T4).

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel **read-only `Explore`** critics — `PR-claude-inspect-critic-read-only-001`):
  **correctness + edges**, **security (XSS) + simplification**.
- **Findings: none.** "No findings; lenses covered: filterRows/sortRows edges, the glue
  filter/sort/aria-sort + empty-state guard, XSS across data-attr/cell/href, the updated test,
  SAST/secrets, simplification."
- **Cleared (critics' concrete checks, run via node + cargo):**
  - `filterRows` — case-insensitive title|id substring (space join → no spurious cross-boundary
    match); blank/whitespace → all (a copy); no-match → `[]`; input unmutated.
  - `sortRows` — `regions`/`subs` sort **numerically** (`9` before `10`, not lexicographic) via
    `Number()`; `localeCompare` for `title`/`id`; `desc` reverses; **stable** on ties (input
    order kept in both dirs); returns a copy.
  - Glue — `if (search && tbody)` guards the empty-state page (no-op); records from `dataset`;
    `filterRows` hides non-matching (`el.hidden`); `sortRows` reorders the `tbody` via
    `appendChild`; `aria-sort` toggles correctly. Only safe DOM APIs (`dataset`/`appendChild`/
    `setAttribute`/`.hidden`) — **no `innerHTML`/`eval`**.
  - **XSS-safe:** `escape_html` covers `& < > " '`; the escaped `title`/`id` are safe in the
    `data-*` attribute, the `<td>` cell, AND the `href` (`a" onmouseover=…` → `&quot;…`,
    `</td><script>` → `&lt;…`); `dataset` decodes back so `filterRows` sees the real text.
  - Empty state keeps no `.cta` (the glue/module source has no `class="cta"`); no SAST/secrets;
    CSS reuses the `:root` tokens; `filterRows`/`sortRows` minimal + new (no reinvention).
  - The updated `regions_list_page_lists_…` test passes (exact `<tr data-*>` match); 25 regions
    tests green.
- **Re-verified:** worktree = 2 tracked + 1 new `regions-table.js` (no clobber); clippy clean.
- **Phase 4 add (critic note):** an explicit escaped-`title` render assertion (XSS guard) folded
  into the dashboard test.
- **Capture:** no `failure-record` (no bug); no new rule.

## Phase 4 — Validate
- **Tests added (+3, +1 updated):**
  - T1 node (NEW `tests/regions-table.test.js`) — `filterRows`: title/id match, case-insensitive,
    blank→all (a copy, `notEqual` the input ref), no-match→`[]`.
  - T2 node — `sortRows`: regions **numeric** (`2,9,10` not lexicographic) asc/desc; title
    `localeCompare` asc/desc; **stable** on ties (input order kept both dirs); input unmutated.
  - T4 rust `regions_list_page_wires_search_sort_and_escapes_titles` — the page has
    `<input id="regions-search"`, `aria-sort="none"`, `filterRows(`/`sortRows(`; **XSS guard**:
    a title `a"<b` renders `data-title="a&quot;&lt;b"` + `<td>a&quot;&lt;b</td>`, never raw.
  - (Phase 3) updated `regions_list_page_lists_…` to the table markup.
- **`node --test tests/*.test.js`:** GREEN — **88 pass** (+2).
- **`cargo test --workspace`:** GREEN — studio **83** (+1), all crates pass.
- **`bin/gate.sh` (FULL):** **GATE GREEN — 17/17, mutation 590 caught / 0 missed → MSI 100.0%**
  (no new Rust mutants — JS-only logic + render fn/glue const). JS coverage 89.99%; rust
  coverage held. Commit-gate receipt written.
- **Pre-existing exclusions:** none.

## Phase 5 — Complete
- **Docs:** `docs/map-system.md` — the "Regions dashboard" note now describes `GET /regions` as a
  **searchable, sortable table** (client-side `filterRows`/`sortRows` + `REGIONS_GLUE` seam, #58).
- **Forge:** `aar-submit` (AAR `d2f9051c`, completed, score 5; reused the read-only-critic +
  assert-element-form rules); no `failure-record` (inspect clean); no new rule.
- **Ticket:** forge **#58 CLOSED (done)**.
- **Archived:** `…/completed/WORK-regions-table-v1.{spec,notes}.md`.
- **GOAL COMPLETE:** all four authoring-loop items (#55 Save · #56 save→game · #57 marquee ·
  #58 regions table) are shipped + merged to `main`.
