# WORK-region-subregion-authoring-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** S2 of the region-authoring program — Editor-gated create/edit/delete
  of regions and sub-regions in an authored `MapDocument`, persisted via S1. This is
  the continuation of forge ticket **#51** (slice 1, the read-only dashboard,
  shipped as `WORK-region-subregion-dashboard-readonly-v1`).
- **Sources (two intakes converge):** `INTAKE-region-model-rethink-and-owner-authoring.md`
  (program, slice S2) and `INTAKE-studio-admin-and-world-model-program.md` (#51 origin;
  #51 scope = create/edit/delete regions+subregions, attributes, editor link).
- **Classification / tier:** work pipeline, one slice. Studio CRUD handlers + forms
  (server-rendered), content (`MapDocument` + `materialize` validation), storage
  (reuse S1's `FileSaveStore`). No engine change.
- **Forge recall (pre-flight green; no bulletins):**
  - #51 (`341c0863-…`) OPEN — slice 1 read-only done; deferred #51b (edit +
    persistence) + #51c (create/delete) = this slice. **S1 unblocked #51b's
    persistence dependency** ("the studio has no storage layer" → S1 added it).
  - Model: `RegionDefinition {id,name}` / `SubregionDefinition {id,name,region}`
    (no description yet — S4); `MapDocument` carries `regions`/`subregions` BTreeMaps
    (`map_document.rs`); `materialize()` already errors on dangling region/subregion
    refs — reuse as the validation boundary.
  - Read-only dashboard to extend: `render::regions_page` (`render.rs:149`); studio
    `sections`/`render` gated-handler pattern; S1's `editor_refusal` gate +
    `/editor/maps` persistence.
  - Prevention rules to honor: `PR-claude-gated-page-role-mutant-001` (Player/anon
    refused per gated route), `PR-claude-assert-element-form-not-substring-001`
    (assert element/name forms), `PR-claude-extract-main-logic-for-mutation-coverage-001`
    (logic out of `main`).
- **Ticket:** forge #51 `341c0863-3fdc-49cd-a438-18a4f5d827f2` (LINKED, not minted);
  local doc `docs/planning/tickets/open/TICKET-51-region-subregion-dashboard.md`.
- **EARS reviewed:** REQ-001..007 (region create/dup, subregion create/parent,
  rename, delete-if-unreferenced, per-route gating, materialize-before-persist, gate).
- **Design crux (Phase 2):** CRUD targets an authored `MapDocument` (locked) — but
  the surface is open: extend the `/regions` dashboard with forms over a *selected*
  authored map, vs. editor-side controls; and which authored document the dashboard
  operates on (S1 persists many by id). Resolve via Explore at design; if it turns
  product-level, surface to the owner.
- **aar_id:** `1dca37d9-93ad-4f23-bbd3-49bb066cc5cc`
- **Branch / WIP:** branch off `ticket-53` (has S1) at implement; online-first WIP
  stays stashed (`stash@{0}`) — do NOT sweep.

## Phase 2 — Design

### Code reconnaissance (working tree, not the lagging codegraph)
- **Content model** (`oathstar-content/src/map_document.rs`): `MapDocument` carries
  `regions: BTreeMap<String, RegionDefinition>` + `subregions: BTreeMap<String,
  SubregionDefinition>` (it reuses the **core** types directly — `RegionDefinition
  {id,name}`, `SubregionDefinition{id,name,region}`). `validate()` **delegates to**
  `materialize()` (so `validate()==Ok ⟺ materialize()==Ok`); `materialize()` =
  `check()` → `build_world()` → `world.validate()`.
- **Materialize net reach** (`oathstar-core/src/lib.rs` `WorldDefinition::validate`):
  catches `RoomRegionMissing` (`:611`), `RoomSubregionMissing` (`:617`),
  **`SubregionRegionMissing`** (`:643` — a subregion whose parent region is gone),
  and `RoomSubregionRegionMismatch` (`:660`). ⇒ deletes and bad parents are caught
  by `materialize()`. **Gap:** a *duplicate region/subregion id on create* is NOT
  caught — a `BTreeMap` insert silently overwrites. So create-dup needs a targeted
  pre-check; everything else has a backstop.
- **Studio gate + persistence**: HTML pages gate via `sections::editor_gate(jar,
  sessions) -> Option<Principal>` (redirect `/login` on None); the JSON map API gates
  via `editor::editor_refusal` (401/403). S1 store = `studio.maps: FileSaveStore`
  (`write_json`/`read_json`/`list`), slot names guarded by
  `oathstar_storage::validate_save_slot_name` (reused for the `{map_id}` path param,
  as `editor::load_map` does). `studio.catalog: Arc<ContentCatalog>` = `beginner_catalog()`.
- **Today `/regions`** renders the **baked** `studio.world` read-only
  (`sections::regions` → `render::regions_page(&studio.world)`). `studio.world` has
  **no other consumer** in the studio crate (grep at implement to confirm).

### Approach / architecture
Two seams, each in its lowest sensible crate so the mutation gate (100% MSI) is
satisfied **in the type's home crate** (lesson `BF-studio-cross-crate-mutation-gap-001`):

1. **`oathstar-content` — pure edit seam (`map_edit.rs`, NEW).** Six `impl
   MapDocument` methods, each takes `&self` + `&ContentCatalog`, returns a *new,
   edited* `MapDocument` (the original is untouched on refusal — REQ-004) or a typed
   `RegionEditError`:
   - `create_region(id, name, catalog)` — refuse `DuplicateRegionId` (the net can't
     catch it); else insert `RegionDefinition{id,name}`.
   - `rename_region(id, name, catalog)` — refuse `UnknownRegion`; else set `name`.
   - `delete_region(id, catalog)` — refuse `UnknownRegion`; refuse
     `RegionReferencedByRoom{room_id}` (any `rooms[].region==id`); refuse
     `RegionReferencedBySubregion{subregion_id}` (any `subregions[].region==id`);
     else remove.
   - `create_subregion(id, name, region, catalog)` — refuse `DuplicateSubregionId`;
     refuse `UnknownParentRegion{region}` (`!regions.contains_key(region)`); else
     insert `SubregionDefinition{id,name,region}`.
   - `rename_subregion(id, name, catalog)` — refuse `UnknownSubregion`; else set name.
   - `delete_subregion(id, catalog)` — refuse `UnknownSubregion`; refuse
     `SubregionReferencedByRoom{room_id}` (any `rooms[].subregion==Some(id)`); else remove.
   - Blank `id`/`name`/`region` (trimmed-empty) → `BlankField{field}` (this is the
     first request-input surface; forms can submit empties).
   - **Materialize net (REQ-006):** a private `finish(edited, catalog)` runs
     `edited.validate(catalog).map_err(RegionEditError::WouldBreakWorld)?` before
     returning. So nothing non-materializable is ever returned ⇒ never persisted.
   - `RegionEditError` — hand-rolled `enum` + `Display` + `Error` (with `source()` for
     `WouldBreakWorld`), `derive(Serialize)`, mirroring `MapValidationError`'s style.
   - Re-export `RegionEditError` from `lib.rs` (the methods ride on `MapDocument`).

2. **`oathstar-studio` — thin HTML surface (`regions.rs`, NEW; `render.rs` + `main.rs`
   + `sections.rs` MODIFY).** Server-rendered forms over the **persisted authored
   maps** (S1 store), map-scoped, Editor-gated — the existing `sections`/`render`
   pattern. Handlers parse a `Form`, load the doc, call a content mutator, and on
   `Ok` **persist via `studio.maps.write_json` + 303-redirect** (PRG) back to the
   editor; on `Err` **re-render with an escaped error banner** (login_submit pattern),
   doc unchanged. Op-dispatched to keep the route/gate surface small:
   - `GET /regions` — list persisted maps (id, title, region/subregion counts),
     each linking to `/regions/{id}`; empty-state when the store is empty.
     **Repoints off `studio.world`.**
   - `GET /regions/{id}` — the per-map region editor: each region panel (rename +
     delete forms) with its nested subregions (rename + delete), a *create region*
     form, and a *create subregion* form (parent `<select>` of region ids). `400`
     on a bad slot name, `404` when absent.
   - `POST /regions/{id}/region` — form `{op: create|rename|delete, id, name?}` →
     dispatch to the matching content mutator; unknown `op` → `400`.
   - `POST /regions/{id}/subregion` — form `{op, id, name?, region?}` likewise.
   - All four reuse `sections::editor_gate` (made `pub(crate)`); anon + Player →
     redirect `/login` (REQ-005, `PR-claude-gated-page-role-mutant-001`).
   - **Escaping:** region/subregion `id` + `name` are now **author input**, rendered
     back into HTML (text + `value=` attrs) → new `render::escape_html` (`& < > " '`).
     The slice-1 "names are server-controlled, no escaping" note no longer holds.
   - **Remove the now-unused `world: Arc<WorldDefinition>` from `StudioState`**
     (slice-1 scaffolding; the baked seed is purged in S5) — updates `main.rs` + the
     three test `studio()` builders. (Confirm no other use at implement; else retain.)

§14: typed errors (no `unwrap`/`panic` on the form/IO path — store failures → 500
banner like S1); state/view split (pure mutators vs. render); deterministic
(`BTreeMap` order). Decision 058: loopback + Editor gate, no public-server route.

### Locked design decisions (load-bearing — flag at review)
- **D1 — `/regions` repoints from the baked `studio.world` to the persisted authored
  maps (S1 store), map-scoped.** Per the locked "authoring targets an authored
  `MapDocument`, not the baked world." This *evolves* slice-1's read-only baked-world
  dashboard (its renderer + two tests are rewritten, not regressed) and retires the
  unused `world` field. Surface = **dashboard forms** (server-rendered), not
  editor-canvas controls — matches the existing pattern and avoids a JS/canvas change.
- **D2 — every CRUD result must `materialize()` before persisting (REQ-006).** Region
  authoring is world-structure management ⇒ it operates on a **playable** map.
  Targeted pre-checks give precise refusals; `materialize()` is the backstop. A
  not-yet-playable draft (no spawn/rooms) renders read-only but refuses edits with the
  materialize reason ("…declares no spawn point") — safe, and nudges the owner to the
  Maps editor. Bootstrapping regions on an empty map is out of scope (rooms already
  require a region, and the editor's starter doc ships one).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-content/src/map_edit.rs` | **NEW** — `RegionEditError` (enum + `Display`/`Error`/`Serialize`); 6 `impl MapDocument` edit methods + private `finish()` materialize-net; `#[cfg(test)]` unit tests (the mutation surface). |
| 2 | `crates/oathstar-content/src/lib.rs` | MODIFY — `mod map_edit;` + `pub use map_edit::RegionEditError;`. |
| 3 | `crates/oathstar-studio/src/regions.rs` | **NEW** — `regions` (list), `region_editor` (GET `/regions/{id}`), `edit_region` + `edit_subregion` (POST, op-dispatch) handlers + form structs; integration tests (gating, success-redirect, refusal-rerender, dispatch, 404/400, escaping). |
| 4 | `crates/oathstar-studio/src/render.rs` | MODIFY — add `regions_list_page(&[MapSummary])`, `region_editor_page(&MapDocument, Option<&str>)`, `escape_html`; retire the world-based `regions_page` + rewrite its unit test. |
| 5 | `crates/oathstar-studio/src/sections.rs` | MODIFY — remove the `regions` handler (moved to `regions.rs`) + its test; make `editor_gate` `pub(crate)`. Items/Enemies/Settings stubs untouched. |
| 6 | `crates/oathstar-studio/src/main.rs` | MODIFY — wire `mod regions;` + routes `GET /regions/{id}`, `POST /regions/{id}/region`, `POST /regions/{id}/subregion`; point `GET /regions` at `regions::regions`; drop the `world` field from `StudioState` + its init. |
| 7 | `crates/oathstar-studio/src/{handlers,sections,editor}.rs` | MODIFY (tests only) — drop `world:` from the test `studio()` builders after the field is removed. |
| 8 | `docs/map-system.md` | MODIFY (Phase 5) — document the region/sub-region authoring surface (additive). `docs/decisions.md` left untouched; AD captured in the forge. |

### Regression Test Plan
At least one row per AC. Content tests are in-crate (`map_edit.rs` `#[cfg(test)]`) —
the mutation-killing surface; studio tests are handler-level (`regions.rs`).

| # | Test | Crate / kind | Proves |
|---|---|---|---|
| C1 | `create_region` adds it; a **duplicate id** → `DuplicateRegionId`, original doc unchanged | content unit | REQ-001 |
| C2 | `create_subregion` with a valid parent → added; **unknown parent** → `UnknownParentRegion`; duplicate id → `DuplicateSubregionId` | content unit | REQ-002 |
| C3 | `rename_region` / `rename_subregion` update the name; unknown id → `UnknownRegion`/`UnknownSubregion` | content unit | REQ-003 |
| C4 | `delete_region`: unreferenced → removed; **referenced by a room** → `RegionReferencedByRoom` (unchanged); **referenced by a child subregion** → `RegionReferencedBySubregion` | content unit | REQ-004 |
| C5 | `delete_subregion`: unreferenced → removed; **referenced by a room** → `SubregionReferencedByRoom` | content unit | REQ-004 |
| C6 | editing a **non-materializable** doc (spawn removed) → `WouldBreakWorld`, nothing returned; blank id/name → `BlankField` | content unit | REQ-006 |
| S1 | `GET /regions` lists two saved maps (ids + titles + a `/regions/{id}` link each) and shows the empty-state with none | studio | REQ-005 + read |
| S2 | `GET /regions/{id}` renders region/subregion panels + create forms (assert full element forms); `404` absent, `400` bad slot | studio | REQ-005 + read |
| S3 | `POST …/region` create **success → 303** to `/regions/{id}` and the store round-trips the new region (reload shows it); rename round-trips | studio | REQ-001/003 |
| S4 | `POST …/region` create **duplicate** → 200 re-render with the error banner, store unchanged; `POST …/subregion` unknown parent → banner; delete-referenced → banner | studio | REQ-001/002/004 |
| S5 | unknown `op` → `400` (kills the dispatch fallback mutant) | studio | dispatch |
| S6 | **per route** (`GET {id}`, `POST region`, `POST subregion`): Editor 200/303, **Player → /login, anon → /login** | studio | REQ-005 (`PR-…-gated-page-role-mutant-001`) |
| S7 | a region named `<script>` renders **escaped** (`&lt;script&gt;`) in the editor page | studio | XSS / input-as-content |
| G1 | `bin/gate.sh` FULL green, mutation 100% MSI | gate | REQ-007 |

Render assertions test **element/name forms**, not bare substrings
(`PR-claude-assert-element-form-not-substring-001`); edit logic lives in the content
crate, not `main` (`PR-claude-extract-main-logic-for-mutation-coverage-001`).
No genuinely uncoverable paths (pure mutators; handler IO-500 paths exercised via a
file-as-dir store, the S1 technique).

### Risks / decisions
1. **D1/D2 above** are the load-bearing, reversible calls — surfaced at review.
2. **Removing `StudioState.world`** ripples to 3 test builders; clean since the field
   is studio-private with a single (replaced) consumer. Verify at implement.
3. **Op-dispatch vs. REST subpaths** — chose `op`-dispatched POSTs (2 routes vs. 6) to
   shrink the gate/test surface; the unknown-`op` fallback is pinned (S5).
4. **Escaping regression** — slice-1 didn't escape (server-controlled names); S2 makes
   names author input, so `escape_html` + S7 are mandatory (also keeps gate:11 SAST clean).
5. **Catalog for the net** — handlers materialize against `studio.catalog`
   (`beginner_catalog`), the same catalog the maps were authored/saved against.

## Phase 3 — Implement
- **Built to the manifest** (production code; tests are Phase 4):
  - `crates/oathstar-content/src/map_edit.rs` (NEW) — `RegionEditError` (10 variants;
    `Display` + `Error` with `source()` for `WouldBreakWorld`; `derive(Serialize)`),
    a private `non_blank` helper, and six `impl MapDocument` methods
    (`create/rename/delete_region`, `create/rename/delete_subregion`) — each clones,
    runs targeted referential checks, then a private `finish()` re-validates via
    `MapDocument::validate` (the materialize net) and returns the edited clone.
  - `crates/oathstar-content/src/lib.rs` — `mod map_edit;` + `pub use RegionEditError`.
  - `crates/oathstar-studio/src/render.rs` — `escape_html` (private), `MapSummary`,
    `regions_list_page`, `region_editor_page` (+ `doc_room_counts`); retired the
    world-based `regions_page`/`room_counts` and their unit test. Swapped the
    `WorldDefinition` import for `MapDocument`.
  - `crates/oathstar-studio/src/regions.rs` (NEW) — `regions` (list), `region_editor`
    (GET), `edit_region` + `edit_subregion` (POST, op-dispatched) handlers; `RegionForm`
    /`SubregionForm`; free `map_summaries`/`load_doc`/`apply` helpers. PRG-redirect on
    success, banner re-render on refusal/save-failure; gated via `sections::editor_gate`.
  - `crates/oathstar-studio/src/sections.rs` — removed the `regions` handler (moved),
    `editor_gate` now `pub`, owner-session test repointed to `items`; dropped the old
    dashboard test + the `world`/`load_beginner_world` test scaffolding.
  - `crates/oathstar-studio/src/main.rs` — `mod regions;`, 3 new routes + repointed
    `GET /regions`; dropped the `world` field/init + the `WorldDefinition` import.
  - `crates/oathstar-studio/src/{handlers,editor}.rs` — dropped `world:` from the test
    `studio()` builders.
- **Verified:** `cargo fmt`; `cargo check -p oathstar-content -p oathstar-studio` clean;
  `cargo clippy … --all-targets` clean under the strict workspace lints. (Tests run at
  Phase 4.)
- **Deviations from design (+ reason):**
  1. **`load_doc` returns `Result<MapDocument, (StatusCode, &'static str)>`** (not
     `Result<_, Response>`) — `clippy::result_large_err` (an axum `Response` is a large
     `Err`). Callers convert with `.into_response()`. No behavior change.
  2. **`editor_gate` is `pub`, not `pub(crate)`** — `clippy::redundant_pub_crate` in a
     private module; matches the crate's cross-module `pub fn` convention (the render fns).
  3. `studio.world` had exactly one consumer (confirmed by grep), so the removal landed
     as designed — no retained-field fallback needed.
- **For Phase 4 (test seams):** `apply` is a plain free fn — cover Ok+write-ok (303 +
  store round-trip), Ok+write-fail (file-as-dir store → "failed to save" banner), and
  Err (refusal banner) directly, without driving HTTP load. Content edits (C1–C6) are
  in-crate `#[cfg(test)]`; handler gating per route (S6) needs a real Player session
  (not no-cookie alone) — `PR-claude-gated-page-role-mutant-001`.

## Inspect (Phase 3.5)
- **Lenses run** (4 parallel `general-purpose` critics, each verifying concretely +
  `cargo check`/`clippy`): **correctness**, **security/XSS**, **data/state integrity**,
  **simplification/reuse**.
- **Findings & verdicts:**
  1. *(correctness, suspected bug → REJECTED as a bug, FIXED as hardening)* — the editor
     page derived its form `action`/redirect target from `doc.id`, while persistence +
     the 303 redirect key off the path **slot** `map_id`. Critic proved no live bug (the
     `slot == doc.id` invariant holds: `save_map` keys storage on `doc.id` and the
     mutators never touch it). **Verdict: real latent inconsistency** (render keyed off a
     mutable field; persist off the slot) — would bite when the studio loads
     externally-authored maps where slot ≠ id. **Fixed:** `region_editor_page` now takes
     the slot `map_id` and builds all form actions from it, so render/persist/redirect
     are uniformly slot-keyed (`render.rs`, `regions.rs` call sites).
  2. *(simplification, LOW → ACCEPTED, documented)* — `render::escape_html` is
     byte-identical to `oathstar_datastar::escape_html`. **Verdict: keep the local copy** —
     reuse would couple the loopback management sidecar to the player-client SSE crate
     (`oathstar-datastar` pulls in `oathstar-protocol` game-event types), a wrong
     dependency direction for 7 lines. **Fixed:** added a comment recording the
     deliberate non-coupling so it isn't "DRY"-ed into a bad edge later.
  - *Rejected / cleared (with the critic's check):* XSS escaping is complete incl. the
    error banner (it escapes the composed `RegionEditError` Display string, covering
    nested author ids); path-traversal guarded by `validate_save_slot_name` on **both**
    read and write (+ symlink guard, tmp-then-rename write); all four routes gate before
    any data access; the axum `Form`-before-gate 422 for a malformed anon POST leaks no
    data; no silent `BTreeMap` overwrite (dup-id check precedes insert); leave-unchanged-
    on-refusal holds (mutators clone, `apply` writes only on `Ok`); determinism (all
    `BTreeMap`); `world`-field removal left no dead code/imports (clippy `--all-targets`
    clean). `map_summaries` silently skipping a malformed stored map is intentional/robust.
- **Re-verified:** `cargo fmt`; `cargo clippy -p oathstar-content -p oathstar-studio
  --all-targets` clean under the strict workspace lints.
- **Capture:** no `failure-record` (no real bug shipped — finding 1 was a latent
  inconsistency hardened, not a manifested defect). Recorded a prevention rule:
  *derive a page's edit-action / redirect target from the authoritative storage key the
  page was loaded under, not from a mutable id field inside the loaded document.*

## Phase 4 — Validate
- **Tests added:**
  - **content `map_edit.rs`** (24 in-crate tests) — C1 create+dup, C2 subregion
    parent/dup, C3 rename region/subregion (+unknown), C4 delete_region (unreferenced
    / room-ref / subregion-ref / unknown), C5 delete_subregion (unreferenced / room-ref
    / unknown), C6 materialize-net (non-materializable → `WouldBreakWorld`), blank-field
    refusals, `non_blank` trimming, a per-variant `Display` test, and `source()`.
  - **studio `render.rs`** (7 tests) — `escape_html` all metacharacters; list page
    (counts/links + empty state); editor page (panels/forms/counts/options, full element
    forms per `PR-…-assert-element-form-not-substring-001`); author-content escaping;
    escaped error banner; and the slot-not-`doc.id` action invariant (the inspect fix).
  - **studio `regions.rs`** (23 handler tests) — S1 list (+empty, +`map_summaries`
    skips malformed), S2 editor GET (+404/400), S3 create/rename round-trips (region +
    sub-region), S4 refusal banners (duplicate / unknown-parent / delete-referenced) +
    `apply` save-failure, S5 unknown-op → 400, S6 per-route gating (Editor vs Player vs
    anon, every route — `PR-…-gated-page-role-mutant-001`).
- **`cargo test --workspace`:** GREEN — auth 20, content 106, core 300, datastar 16,
  protocol 27, server 35, storage 23, studio 74; 0 failed.
- **`node --test tests/*.test.js`:** GREEN — 79 pass, 0 fail (JS untouched).
- **`bin/gate.sh` (FULL):** **GATE GREEN — 17/17.** Mutation **589 caught / 0 missed →
  MSI 100.0%**; Rust line coverage **98.42%** (≥94; `map_edit` 100%, `regions` 99.63%,
  `render` 100%); JS **89.44%** (≥75). Commit-gate receipt written.
- **Mutation survivors found + fixed (the gate did its job):**
  1. Two survivors — `edit_subregion`'s `"rename"` and `"delete"` match arms — because
     the handler suite drove `edit_subregion` only with `create`/unknown-op (deleting
     those arms falls through to `_ => 400` unnoticed). **Fixed:** added
     `rename_subregion_persists` + `delete_subregion_persists` (each asserts a 303, which
     a deleted arm would turn into a 400). Verified killed via a scoped
     `cargo mutants -f regions.rs` (12 caught / 0 missed) before the full re-run.
     *Lesson for capture (Phase 5):* an op-dispatched handler needs a success test per
     arm, not just create + unknown-op.
  2. SAST false-positive — gate:11 flagged the word "unsafe" inside the doc comment
     "path-unsafe id" (the `-` is a `\bunsafe\b` boundary; `path_unsafe` with `_` is
     not). **Fixed:** reworded to "an invalid storage id".
- **Pre-existing failures:** none.

## Phase 5 — Complete
- Docs updated:
- Forge capture:
- Ticket:
- Archived:
