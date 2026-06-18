# WORK-region-subregion-description-authoring-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** the next #51 slice after S2's id/name/parent CRUD — add an editable
  **description** to regions and sub-regions, authored through S2's surface and
  carried through `materialize()`. Continuation of forge **#51** (not a new ticket).
- **Intake source:** `INTAKE-region-model-rethink-and-owner-authoring.md` — "region-model
  enrichment" is in scope; "the region-standing consequence system itself" is OUT
  (only its defaults may be authored). Origin: `INTAKE-studio-admin-and-world-model-program.md`.
- **Classification / tier:** work pipeline, one slice. Touches the **core model**
  (`RegionDefinition`/`SubregionDefinition` + serde), **content** (`map_document`
  materialize pass-through, `map_edit` edit seam), **studio** (`render`/`regions`).
- **Forge recall (pre-flight green; no bulletins):**
  - Model is tiny — `RegionDefinition {id,name}` / `SubregionDefinition {id,name,region}`
    (`oathstar-core/lib.rs:75,83`); the `RegionDefinition` doc even says "the registry
    lets region-level systems … attach later." Struct-literal construction in only
    **3 files** (core lib.rs, content map_document.rs, map_edit.rs) — small ripple;
    `#[serde(default)]` keeps the beginner TOML (serde-loaded) valid.
  - `materialize()`/`build_world` copies `self.regions.clone()` / `subregions.clone()`
    into the world, so a new field **passes through automatically** — no materialize
    logic change. Description is free metadata → no validation rule, no effect on the
    materialize gate (`AD-claude-authored-map-region-crud-001`).
  - `region-standing.md` + Decision 009: standing is a **coarse** social layer
    (unknown/neutral/liked/disliked/hostile); only its *defaults* are authorable in this
    program, and the consequence system is OUT. ⇒ any `standing_default` is at most a
    coarse enum with no engine consumer yet — a bigger commitment than `description`.
  - Reuse S2: `map_edit` methods (extend create + add an edit/set-description path),
    `render::region_editor_page` (show + edit description, escaped), `regions` handlers
    (op-dispatched, Editor-gated). Honor the S2 prevention rules
    (`PR-…-gated-page-role-mutant-001`, `…-assert-element-form-…`, `…-op-dispatch-arm-test-001`,
    `…-extract-main-logic-…`).
- **Ticket:** forge **#51** `341c0863-3fdc-49cd-a438-18a4f5d827f2` (LINKED, not minted);
  local doc `docs/planning/tickets/open/TICKET-51-region-subregion-dashboard.md` (its
  scope: "edit attributes (at minimum name + description)").
- **EARS requirements reviewed:** REQ-001..007 — create-with-description, edit-only-
  description, serde-default backward-compat, materialize pass-through, escaped display,
  per-route gating, gate green.
- **Design crux (Phase 2):** (a) the edit surface — extend S2's op-dispatched `rename`
  into an `edit` (name+description) vs. a new op / a description field on the forms;
  (b) `String` (default empty) vs `Option<String>`; (c) **standing-default in or out**
  (planner leans OUT — keep to one field). Resolve via Explore; surface to owner if it
  turns product-level.
- **aar_id:** `d584f02e-1b31-405e-a9c5-40ae72066eab`
- **Branch / WIP:** stacks on S2 — branch off `ticket-51-region-subregion-authoring`
  (PR #2) at implement; online-first WIP stays stashed (`stash@{0}`) — do NOT sweep.

## Phase 2 — Design

### Code reconnaissance (working tree)
- **Pass-through is free:** `build_world` does `regions: self.regions.clone()` /
  `subregions: self.subregions.clone()` (`map_document.rs:850-851`) — a new field on the
  core defs rides into the `WorldDefinition` automatically (REQ-004), no materialize change.
- **No validation on region name/desc** (`WorldDefinition::validate` touches neither) — description is free metadata, no new check, the materialize gate is unchanged.
- **Beginner TOML** is serde-loaded `[[regions]] id/name` (no struct literal in the loader) → `#[serde(default)]` keeps it valid, zero loader change (REQ-003).
- **Struct-literal fan-out** (each must gain the field): `RegionDefinition {` ×8 and `SubregionDefinition {` ×8 — core `lib.rs` (≈10, incl. `region()`/`subregion()` test helpers), `map_document.rs` (5, tests), `map_edit.rs` (4 — the 2 `create_*` production sites I edit + 2 test helpers). Predominantly test fixtures; mechanical.
- **No other crate constructs the defs** (server/studio deserialize) — ripple is core + content only.

### Approach / architecture
Add `description` to the **core** region defs (MapDocument reuses them directly — there is no map-layer region type), author it through S2's surface, let it ride `clone()` into the world.

1. **`oathstar-core`** — `RegionDefinition` and `SubregionDefinition` gain
   `#[serde(default, skip_serializing_if = "String::is_empty")] pub description: String`.
   Empty = none; skip-if-empty keeps existing JSON byte-clean and matches the
   materialized `RoomDefinition.description` (plain `String`, default-on-absence). No
   `validate` change.
2. **`oathstar-content::map_edit`** — extend the edit seam:
   - `create_region(id, name, description, catalog)` / `create_subregion(id, name,
     region, description, catalog)` — gain a `description: &str` (trimmed, **empty
     allowed** — not `non_blank`) and set it on the inserted def.
   - **`rename_region`/`rename_subregion` → `update_region`/`update_subregion(id, name,
     description, catalog)`** — set **name + description** (id/parent untouched); reuse
     `non_blank` for name only. No new `RegionEditError` variant (description has no
     failure mode beyond the existing blank-id/name + unknown checks).
   - `finish()` materialize-net unchanged.
3. **`oathstar-studio`** — thin plumbing:
   - `regions.rs`: `RegionForm`/`SubregionForm` gain `#[serde(default)] description`; the
     op **`"rename"` → `"edit"`** dispatches to `update_*`; `"create"` passes description.
   - `render.rs`: each region/sub-region row's form becomes an **edit form** — a name
     `<input>` + a description `<textarea>` (pre-filled with the escaped current
     description) + Save; the create forms gain an (empty) description `<textarea>`. The
     **pre-filled textarea is the escaped display** (REQ-005) — no separate element.
§14: typed errors unchanged; escaping via the existing `escape_html`; deterministic.

### Locked decisions (this phase)
- **Combined `edit` op (name + description), replacing S2's `rename`** — one Save per
  row (better authoring UX) over an additive description-only `describe` op. Evolves
  S2's rename surface (op string + `rename_*`→`update_*`); bounded churn on the stacked
  branch. REQ-002 ("update only the description, name unchanged") is satisfied because
  the edit form pre-fills name, so a description-only change re-submits the same name.
- **`String` (default empty, skip-if-empty), not `Option<String>`** — matches the
  materialized room description; empty string = no description.
- **`standing_default` deferred (lean-out confirmed)** — keep this slice to one field;
  revisit when the standing system exists or the owner asks. The full standing
  consequence system stays OUT (Decision 009).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/lib.rs` | Add `description: String` (`#[serde(default, skip_serializing_if = "String::is_empty")]`) to `RegionDefinition` + `SubregionDefinition`; add the field to the ≈10 struct-literal sites in tests (incl. the `region()`/`subregion()` helpers). No `validate` change. |
| 2 | `crates/oathstar-content/src/map_edit.rs` | `create_region`/`create_subregion` gain a trimmed `description` (empty allowed) and set it; `rename_*` → `update_*(id, name, description)` set name+description; update the 2 test helpers + the rename→update tests. No new error variant. |
| 3 | `crates/oathstar-content/src/map_document.rs` | Add `description` to the 5 struct-literal sites in tests; `build_world` unchanged (clone carries it → REQ-004). |
| 4 | `crates/oathstar-studio/src/regions.rs` | `RegionForm`/`SubregionForm` gain `#[serde(default)] description`; op `"rename"`→`"edit"` → `update_*`; `"create"` passes description; update the handler tests. |
| 5 | `crates/oathstar-studio/src/render.rs` | Per-row edit form: name `<input>` + description `<textarea>` (escaped, pre-filled) + Save; create forms gain a description `<textarea>`. Update render tests. |
| 6 | `docs/map-system.md` | (Phase 5) note description on the regions authoring surface. |

### Regression Test Plan
| # | Test | Crate / kind | Proves |
|---|---|---|---|
| C1 | `create_region` / `create_subregion` with a description → stored on the def (and on the persisted/round-tripped doc) | content unit | REQ-001 |
| C2 | `update_region`/`update_subregion(id, current-name, new-desc)` → description updated, **name/id/parent unchanged**; `(new-name, new-desc)` → both; blank name → `BlankField`; unknown id → `UnknownRegion`/`UnknownSubregion` | content unit | REQ-002 |
| C3 | core serde: a def **without** `description` deserializes to `""`; **with** it round-trips; an empty description is **skip-serialized** (no key) | core unit | REQ-003 |
| C4 | a `MapDocument` with a described region + sub-region → `materialize()` → `world.regions[id].description` / `subregions[id].description` == authored | content unit | REQ-004 |
| C5 | backward-compat: `load_beginner_world()` still loads (TOML w/o description → empty) and the existing core/content/studio suites stay green | existing suites | REQ-003 |
| S1 | POST `op=create` with a description → 303 + reload shows it persisted | studio | REQ-001 |
| S2 | POST `op=edit` updates the description → 303 + reload; name unchanged | studio | REQ-002 |
| S3 | `region_editor_page` renders each description in its edit `<textarea>`, **HTML-escaped** (a `<script>` description → `&lt;script&gt;`) | studio render | REQ-005 |
| S4 | per-route gating — create / **edit** / delete arms each Editor-success + Player→/login + anon→/login (the new `edit` arm gets a success test — `PR-…-op-dispatch-arm-test-001`) | studio | REQ-006 |
| G1 | `bin/gate.sh` FULL green, mutation 100% MSI | gate | REQ-007 |
No genuinely uncoverable paths. Assertions use full element forms (`…-assert-element-form-…`); edit logic stays in the content crate (`…-extract-main-logic-…`).

### Risks / decisions
1. **Edit-op shape (D1, load-bearing)** — combined `edit` replacing `rename` (above). Alternative: additive `describe` op keeping `rename` intact (lower S2 churn, literal REQ-002 match) — rejected for the cleaner single-form UX; flag at review.
2. **Mechanical fan-out** — ~17 struct-literal sites; trivial but broad. `cargo check` drives them out; serde-default means no production TOML/JSON path changes.
3. **No new error variant / no materialize change** — description rides the existing clone; keeps the slice small and the gate boundary intact.
4. **`standing_default` deferred** — design-scoped OUT (one field this slice).

## Phase 3 — Implement
- **Built to the manifest** (production code; new tests are Phase 4):
  - `oathstar-core/src/lib.rs` — `RegionDefinition` + `SubregionDefinition` gained
    `#[serde(default, skip_serializing_if = "String::is_empty")] pub description: String`
    (the attr resolves — verified by compile). Fixed the 10 struct-literal sites in
    core tests (incl. the `region()`/`subregion()` helpers).
  - `oathstar-content/src/map_edit.rs` — `create_region`/`create_subregion` gained a
    trimmed `description: &str` (empty allowed) set on the inserted def;
    `rename_region`/`rename_subregion` → **`update_region`/`update_subregion(id, name,
    description)`** (set name + description; id/parent unchanged). No new error variant.
    Updated the 2 test helpers + the existing create/rename test calls.
  - `oathstar-content/src/map_document.rs` — 5 test-literal sites gained the field;
    `build_world` unchanged (clone carries description into the world → REQ-004 free).
  - `oathstar-studio/src/regions.rs` — `RegionForm`/`SubregionForm` gained
    `#[serde(default)] description`; op `"rename"`→`"edit"` dispatching to `update_*`;
    `"create"` passes description. Updated the 13 test form-literals (+ op→edit, + the
    direct `create_region` call).
  - `oathstar-studio/src/render.rs` — each region/sub-region row is now an **edit form**
    (name `<input>` + a description `<textarea>` pre-filled with the escaped current
    description, Save); the create forms gained a description `<textarea>`. The
    pre-filled textarea is the escaped display (REQ-005).
- **Compiler-as-checklist:** added the field to the core structs, then `cargo check
  --all-targets` enumerated every missing-field site (10 core + 5 map_document + 2 helpers +
  ~12 form literals) — fixed each mechanically. **Verified:** `cargo fmt`;
  `cargo check --all-targets` clean (core + content + studio); `cargo clippy … --all-targets`
  clean under the strict lints.
- **Deviations from design:** none of substance. `skip_serializing_if = "String::is_empty"`
  compiles fine (the resolution concern from design was unfounded). Description is trimmed
  with empty allowed, as designed.
- **For Phase 4 (known follow-ups):** the S2 render test
  `region_editor_page_renders_panels_forms_counts_and_options` still asserts the *old*
  `rename`-form HTML, so it will fail until updated to the `edit` form — that update plus
  the new description tests (C1–C4 description assertions, S1 create-with-description
  round-trip, S2 edit-description, S3 escaped-textarea display, S4 the new `edit`-arm
  gating — `PR-…-op-dispatch-arm-test-001`) land in validate.

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel `general-purpose` critics, scaled to the small/mechanical
  diff, each verifying concretely incl. cargo check + empirical serde tests):
  **correctness + data-integrity**, **security/escaping + simplification**.
- **Findings: none.** No code defects. "No findings; lenses covered: materialize
  pass-through, create/update semantics, trim, serde backward-compat + determinism,
  op rename→edit gating, the mechanical literal sites, textarea/attribute escaping,
  SAST tokens, simplification."
- **Cleared (with the critic's check):**
  - **Materialize pass-through (REQ-004)** — `build_world` clones `regions`/`subregions`
    so `description` rides into the world with zero code change; a grep confirms
    `validate`/`check` never inspect the field (genuinely inert at the boundary).
  - **update_* keeps id/parent (REQ-002)** — `update_region`/`update_subregion` set only
    `name` + `description`; id is the map key, parent untouched. The combined-`edit`
    op satisfies "update only the description" because the form pre-fills name.
  - **serde backward-compat + determinism (REQ-003)** — critic empirically round-tripped:
    a description-less doc deserializes to `""` and re-serializes byte-clean (skip-if-empty);
    populated survives; no struct-eq vs JSON-eq divergence; the determinism path stays
    deterministic. The in-tree `SEED_DOC` (no description) is the backward-compat proof.
  - **op rename→edit** — `editor_gate` runs *before* the op `match` in both handlers, so
    the rename can't bypass gating; `"rename"` now falls to the unknown-op 400.
  - **~27 mechanical sites** — all sensible empties (`String::new()` in tests); grep found
    no wrong-value (`description = name`) site.
  - **Textarea escaping** — a description of `</textarea><script>…` renders as
    `&lt;/textarea&gt;…` (escape_html escapes `<`), inert — cannot break out of the
    textarea. `escape_html` complete (`& < > " '`); aria-label/`value` attrs use escaped
    values. No new SAST token.
- **Process anomaly (verified, no net change):** the correctness critic ran `git checkout`
  on the uncommitted `lib.rs` while cleaning up a throwaway test, then reconstructed it.
  I independently verified the worktree is byte-correct — **10** `description: String::new()`
  sites + **2** struct fields (+18), `cargo check --all-targets` + `clippy` clean across
  core/content/studio. The critic's prose "8 sites" was a miscount; the file is intact.
- **Capture:** no `failure-record` (no bug). Recorded a prevention rule:
  *inspection critics must be read-only — never run `git checkout`/`reset`/`stash` or
  otherwise mutate the worktree under review* (this run's clobber-and-reconstruct happened
  to net-zero, but it risks silently reverting uncommitted work).

## Phase 4 — Validate
- **Tests added** (+12):
  - **content `map_edit.rs`** (7) — C1 create stores a *trimmed* description (region +
    sub-region); C2 `update_region`/`update_subregion` set the description while keeping
    id/name/parent (and the both-change case); C4 `materialize()` carries region +
    sub-region descriptions into the world; C3 `RegionDefinition` serde is additive
    (absent → `""`, empty skip-serialized byte-clean, populated round-trips).
  - **studio `regions.rs`** (2) — S1 `op=create` with a description persists it; S2
    `op=edit` updates the description and keeps the name (REQ-002 through the handler).
  - **studio `render.rs`** (1 new + 1 updated) — S3 the description renders in the row's
    `<textarea>` HTML-escaped (a `</textarea><script>` value is inert); and the S2 panel
    test was updated from the old `rename` form to the new `edit` form (name input +
    description textarea), with descriptions added to its `RICH_DOC` fixture.
- **`cargo test --workspace`:** GREEN — auth 20, content **113**, core 300, datastar 16,
  protocol 27, server 35, storage 23, studio **77**; 0 failed.
- **`node --test tests/*.test.js`:** GREEN — 79 pass, 0 fail (JS untouched).
- **`bin/gate.sh` (FULL):** **GATE GREEN — 17/17.** Mutation **589 caught / 0 missed →
  MSI 100.0%** (no survivors — the field / `.trim()` / assignment / textarea are
  non-mutable constructs, and the one real arm `op=edit` is covered by the repurposed S2
  tests). Rust + JS coverage floors held. Commit-gate receipt written.
- **Pre-existing exclusions:** none. (The S2 render panel test that asserted the old
  `rename` form was updated to the `edit` form, as flagged in Phase 3.)

## Phase 5 — Complete
- Docs updated:
- Forge capture (aar/failures/rules/decisions):
- Ticket closed:
- Archived:
