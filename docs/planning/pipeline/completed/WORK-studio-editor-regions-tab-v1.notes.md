# WORK-studio-editor-regions-tab-v1 — Notes

## Phase 1 — Plan
- **Request:** fill the editor Regions tab with inline region create/rename/delete, via a round-trip
  endpoint over the existing Rust CRUD. Pivot ③ slice b (memory `studio-editable-world-pivot`).
  **Regions only**; sub-regions = ③b2.
- **Classification / tier:** work pipeline, one slice, `oathstar-studio` (handler + route + tab markup
  + glue) + a pure JS fn + tests. CRUD **reused** from `oathstar-content` — no engine change.
- **Recon (main `7fc327c`):**
  - `MapDocument` (`map_document.rs:246`): `regions: BTreeMap<String,RegionDefinition>` (`:267`),
    `subregions: BTreeMap<String,SubregionDefinition>` (`:270`); `RegionDefinition{id,name,description}`.
  - `map_edit.rs:152-302`: `create_region`/`update_region`/`delete_region`(id,name,desc,catalog) +
    the sub-region trio → `Result<Self, RegionEditError>` (clone→edit→`finish(catalog)`).
    `delete_region` refuses with `RegionReferencedByRoom`/`RegionReferencedBySubregion`.
    `RegionEditError` (`:24`) is a typed `Display` enum (DuplicateRegionId / UnknownRegion /
    BlankField / WouldBreakWorld / …) → the `400` message.
  - The server-side `/regions` editor (`regions.rs edit_region`) already calls these on a **saved**
    map; the editor edits an **in-memory** doc, so ③b routes edits through it (else Save clobbers).
  - `editor.rs` has `editor_refusal`/`refuse`/`Failure` to reuse; EDITOR_GLUE holds the in-memory `doc`.
- **Approach (design refines):** `region_op` dispatch endpoint (Ok→200 doc / Err→400 message) +
  the Regions tab (client-rendered list via `editorRegionRows`, add form, per-row rename/delete,
  swap-in the returned doc). `editorRegionRows(doc)` pure.
- **EARS:** REQ-001 create · REQ-002 edit · REQ-003 delete (+ refusal) · REQ-004 bad op/id · REQ-005
  auth · REQ-006 editorRegionRows · REQ-007 tab UI not a stub · REQ-008 gate.
- **Mutation surface:** `editorRegionRows` (the per-region tally + map) + the `region_op` `op`
  dispatch (`match op` arms + the unknown-op 400) — killed by the rust + node tests.
- **Decisions:** dispatch-only (no CRUD reimpl); in-memory doc authoritative; **client `textContent`**
  for author id/name (the list is client-built, not server-rendered → not `escape_html`).
- **Ticket:** forge **#62** `bedd26d3-a0e8-4998-837c-7001b5264dac`. Local doc
  `docs/planning/tickets/open/TICKET-62-studio-editor-regions-tab.md`.
- **aar_id:** `bdaa6261-1dc0-492e-b664-77c521f52bb1`
- **Delivery:** AUTONOMOUS through commit+push+FF-merge. Branch off `main` `7fc327c`. Stash parked.

## Phase 2 — Design

### Code reconnaissance
- **`MapDocument` derives `Serialize + Deserialize`** (`map_document.rs:245`) → `Json(doc)` (return the
  updated doc) and the `RegionOp.document` body parse both work.
- **`#panel-regions` stub** (`render.rs:693`): `<section … id="panel-regions" … hidden><p class="soon">
  Coming soon.</p></section>` — only the **inner** `<p>` changes (the `<section>` tag stays, so the
  #61 `editor_page_has_a_tabbed_rail` assertion `id="panel-regions" … hidden` still holds; "Coming
  soon." stays for rooms/map).
- **EDITOR_GLUE**: `let doc = JSON.parse(#map-doc)` (`:397`, reassignable); `redraw()` (`:452`)
  re-renders the canvas from `doc`. The Save/Validate/Activate handlers close over `doc`, so swapping
  `doc = returnedDoc` flows through everywhere (same pattern the marquee paint uses, `doc = paintRect(…)`).

### Approach / architecture (oathstar-studio + reused oathstar-content CRUD)
- **`editor.rs`** — `#[derive(Deserialize)] struct RegionOp { document: MapDocument, op: String,
  #[serde(default)] id: String, #[serde(default)] name: String, #[serde(default)] description:
  String }`. NEW `region_op(State(studio), jar, body: Bytes) -> Response`: `editor_refusal` gate;
  parse-or-`refuse(400,"request body is not a valid region op")`; `match req.op.as_str() { "create"
  => req.document.create_region(&id,&name,&desc,&catalog), "edit" => …update_region…, "delete" =>
  req.document.delete_region(&id,&catalog), _ => return refuse(400,"unknown region op") }`; then
  `Ok(doc) => Json(doc).into_response()`, `Err(error) => refuse(400, &error.to_string())`. **Pure
  dispatch — no CRUD reimplementation.**
- **`main.rs`** — `.route("/editor/maps/region-op", post(editor::region_op))`.
- **`render.rs`** — replace the `#panel-regions` inner content with: an **add-region** form
  (`#region-add-id`/`#region-add-name`/`#region-add-desc` + `#region-add` button), a `#region-result`
  feedback line, and `<ul id="region-list"></ul>`. In `EDITOR_GLUE`, a regions controller:
  `renderRegions()` clears `#region-list` and appends, per `editorRegionRows(doc)` row, an `<li>`
  built with **`textContent`** (NO `innerHTML`) — `name (id) — N rooms · M sub-regions` + a **Rename**
  (`prompt` → `edit`) and **Delete** (`delete`) button (`dataset.regionId`); a shared
  `async regionOp(op, fields)` that POSTs `{ document: doc, op, …fields }` to `/editor/maps/region-op`,
  and on `200` does `doc = await res.json(); renderRegions(); redraw();`, else shows the `message`.
  Wire `#region-add`; call `renderRegions()` once on load.
- **`editor-canvas.js`** — `export function editorRegionRows(doc)` → `Object.values(doc.regions ?? {})`
  mapped to `{ id, name, roomCount, subregionCount }`, tallying `doc.rooms` by `room.region` and
  `Object.values(doc.subregions ?? {})` by `sub.region`. **Preserves `doc.regions` order** (id-sorted
  in real data, since `regions` is a `BTreeMap`); **no explicit sort** (keeps the mutation surface to
  the tallies). Null-safe (`?? {}`/`?? []`). Pure.

### Locked decisions (this phase)
- **No viable Rust mutant in `region_op`** (all `match`/`if let`/method-calls, no operators; `Response`
  ≠ `Default`) — same as the other handlers; **coverage** (every arm exercised by T1–T5) is the bar.
  `editorRegionRows` is JS → coverage (gate:16), not cargo-mutants.
- **Client `textContent`** for author id/name (the list is client-built from the live doc) — XSS-safe,
  no `escape_html` (that's for server-rendered content).
- `editorRegionRows` order = `doc.regions` order (no sort); the node test constructs a known order.
- **Sub-regions out** (③b2) — `editorRegionRows` reports a sub-region *count* only this slice.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-studio/src/editor.rs` | `RegionOp` struct + `region_op` handler; tests (create/edit/delete/refusal/bad-op/auth). |
| 2 | `crates/oathstar-studio/src/main.rs` | route `POST /editor/maps/region-op`. |
| 3 | `crates/oathstar-studio/src/render.rs` | `#panel-regions` regions UI; `EDITOR_GLUE` regions controller; NEW `editor_page_regions_tab_has_the_region_ui` test. |
| 4 | `crates/oathstar-studio/static/editor-canvas.js` | NEW pure `editorRegionRows`. |
| 5 | `tests/studio-editor-canvas.test.js` | `editorRegionRows` cases. |
| 6 | `crates/oathstar-studio/static/studio.css` | minimal `.region-list`/`.region-row` styles (`:root` tokens). |
| 7 | `docs/map-system.md` | (Phase 5) the Regions tab edits regions inline. |

### Regression Test Plan
| # | Test | Proves |
|---|---|---|
| T1 | `region_op` `create` (new id) over `VALID_DOC` → `200`; the returned `document.regions` contains the new region | REQ-001 |
| T2 | `region_op` `edit` of `reg` → `200`; the region's `name` is updated | REQ-002 |
| T3 | `region_op` `delete` of a fresh unreferenced region → `200`, gone; `delete` of `reg` (rooms `alpha`/`beta` use it) → `400`, message names the room, `regions` unchanged | REQ-003 |
| T4 | `region_op` `op:"bogus"` → `400`; a blank/duplicate id on create → `400` | REQ-004 |
| T5 | no cookie → `401`; a Player session → `403` | REQ-005 |
| T6 | `editorRegionRows` — 2 regions + subs + rooms → correct `roomCount`/`subregionCount`; `{}` doc → `[]`; a region with no rooms → `roomCount:0` | REQ-006 (node) |
| T7 | the editor `#panel-regions` contains `id="region-list"` + the add-region form, and the glue calls `editorRegionRows(` / `renderRegions(` (not a "Coming soon" stub for regions) | REQ-007 (render) |
| G1 | `bin/gate.sh` FULL green, MSI 100% | REQ-008 |
- Rust tests reuse `studio()`/`owner_principal`/`jar`/`decode` + a `call_region_op` helper; the body
  is `serde_json::json!({"document": <VALID_DOC parsed>, "op": …, "id": …, "name": …})`.
  **Uncoverable:** none new (the `WouldBreakWorld`/`500`-style arms aren't reachable here; the dispatch
  arms are all hit).

### Risks / decisions
1. **`doc` reassignment** — `region_op`'s returned doc replaces the in-memory `doc`; Save/Validate/
   Activate + `redraw()` all read the `let doc` binding, so they pick up the new regions. Pin with a
   smoke note (glue is review-verified, the logic is in `editorRegionRows` + the Rust CRUD).
2. **`#61` test compat** — the `<section id="panel-regions" … hidden>` tag is unchanged (only its
   inner content), so `editor_page_has_a_tabbed_rail` keeps passing.
3. **Author input** — ids/names flow into the client list via `textContent` (no injection); the Rust
   CRUD validates them (`non_blank`, duplicate/unknown checks) and re-`finish`es against the catalog.

## Phase 3 — Implement
- **Built (manifest as designed):**
  - `editor.rs` — `RegionOp` struct (`#[derive(Deserialize)]`; `serde` import → `{Deserialize,
    Serialize}`) + `region_op` handler: gate → parse → `match op { create/edit/delete → the reused
    `MapDocument` CRUD, _ → 400 }` → `Ok(doc)=Json(doc)` / `Err=refuse(400, &error.to_string())`.
  - `main.rs` — `POST /editor/maps/region-op` route.
  - `render.rs` — `#panel-regions` now an add-region form + `#region-result` + `<ul id="region-list">`
    (the `<section>` tag unchanged → #61 test still passes); EDITOR_GLUE gains the regions controller
    (`regionOp(op,fields)` POSTs `{document: doc, op, …}`, swaps `doc = await res.json()`, then
    `renderRegions()` + `redraw()`; `renderRegions()` builds `<li>`s with **`textContent`** from
    `editorRegionRows(doc)`, Rename via `prompt`, Delete; called once on load).
  - `editor-canvas.js` — pure `editorRegionRows(doc)` (null-safe tallies; preserves `doc.regions`
    order).
  - `studio.css` — `.region-add`/`.region-list`/`.region-row` on the tokens.
- **Deviations:** none of substance. `region_op` is a pure dispatch over the reused CRUD; `doc` is
  reassigned (matches the marquee paint path); the JSON-decoded JS objects use `(doc && doc.x) || …`
  guards.
- **Checks:** `cargo check`/`clippy -p oathstar-studio --all-targets` clean; `cargo fmt` clean;
  `node --check editor-canvas.js` OK; `cargo test -p oathstar-studio` → **92 passed** (unchanged;
  `editor_page_has_a_tabbed_rail` + the server-side `regions::tests` all still green). New tests +
  gate at Phase 4.

## Inspect (Phase 3.5)
- **Lenses:** 2 read-only `Explore` critics: correctness/security + simplification/reuse.
- **Critic 1 (correctness/security) — CLEAN.** Verified the dispatch (each op → the matching reused
  `MapDocument` CRUD with the right args; `_`→400), `Ok`→full updated doc / `Err`→`error.to_string()`
  400, no panic (let-else), the Editor gate; the glue's `doc = await res.json()` reassigns the live
  `let doc` so Save/Validate/Activate + `redraw()` see the new regions (no stale closure);
  `textContent` (no `innerHTML`) → XSS-safe; the endpoint returns JSON, not HTML. 92 tests pass.
- **REAL FINDING — [high] rename wipes the region description (FIXED).** The Rename control posted
  `description:""`; `update_region` overwrites `region.description` with it → renaming silently
  **deleted an authored description** (data loss). **Fixed:** `editorRegionRows` now carries
  `description: reg.description ?? ""`, and the Rename echoes `description: row.description` back
  unchanged so `update_region` preserves it. Recorded **BF-region-rename-wipes-description-001** +
  **PR-claude-roundtrip-edit-echo-unchanged-fields-001** (a client round-trip over a *full-overwrite*
  CRUD must echo back the fields it isn't changing). Re-verified: clippy/JS/92-tests green.
- **Also fixed — [med] `#region-result` was unstyled** (the `#result` box/`data-ok` color rules were
  id-specific). Extended those selectors to `#region-result` so the regions feedback matches the other
  controls. (cheap consistency win.)
- **Rejected / noted:** the `.region-add` bare grid (vertical stack is fine in the narrow rail); the
  "no region_op tests yet" + "sub-region count shown without edit UI" are Phase-4 scope / intentional
  (③b is regions-only; sub-region editing is ③b2). Critic 1's "no viable Rust mutant in `region_op`"
  is confirmed (match/no-operators) — coverage is the bar.

## Phase 4 — Validate
- **Tests added** (T1–T7):
  - `editor.rs` — `call_region_op`/`region_op_body`/`valid_doc_value` helpers + **T1** create
    (200, the returned doc has the new region), **T2** edit (rename reflected), **T3** delete an
    unreferenced region (200, gone) **and** refuse deleting `reg` (rooms use it → 400, message names
    the room, body is a `Failure`), **T4** bad op + duplicate id → 400, **T5** anon→401 / Player→403.
  - `render.rs` — **T7** `editor_page_regions_tab_has_the_region_ui` (`#region-list`/`#region-add`/
    `#region-add-id` + the glue calls `editorRegionRows(`/`renderRegions(`/`fetch("/editor/maps/
    region-op"`).
  - `tests/studio-editor-canvas.test.js` — **T6** `editorRegionRows` (room/sub-region counts; `{}`/
    `null`/`undefined` → `[]`; the row carries `description` — the inspect fix).
- **`cargo test --workspace`:** green — `oathstar-studio` **98 passed** (+6); all other crates green.
- **`node --test tests/*.test.js`:** **91 passed / 0 fail** (+1).
- **`bin/gate.sh` FULL:** **GATE GREEN [full]** — 17/17. rustfmt, clippy strict, both suites,
  rust cov ≥94, js cov ≥75, **mutation 598 caught / 0 missed → MSI 100.0%**. Receipt written.
- **Pre-existing exclusions:** none.

## Phase 5 — Complete
- Docs / forge / ticket / archived:
