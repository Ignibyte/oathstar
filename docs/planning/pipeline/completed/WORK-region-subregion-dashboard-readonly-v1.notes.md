# WORK-region-subregion-dashboard-readonly-v1 — Notes

## Phase 1 — Plan
- **Request:** the region & sub-region dashboard (ticket #51, build order 3 of 4).
  **Slice 1 = read-only**: fill the #49 nav's "Regions" slot with a real page
  listing regions → sub-regions + room counts, each sub-region linking to the
  editor. Editing/persistence/per-map identity are deferred slices.
- **Intake source:** `INTAKE-studio-admin-and-world-model-program.md` (#49 ✓ →
  **#50 foundation ✓** → #51 regions → #52 world model).
- **Classification / tier:** work pipeline, one slice. Studio UI (server-rendered)
  reading the existing world model — no engine change, no persistence. The gate
  rides on the Rust handler + the `regions_page` `format!` markers (mutation) + the
  per-route gate tests.
- **Explore-confirmed data model:**
  - `oathstar-core/src/lib.rs:75-87` — `RegionDefinition {id, name}`,
    `SubregionDefinition {id, name, region}`; minimal (no richer attributes yet;
    `region-standing.md` hints future ones).
  - `WorldDefinition` (lib.rs:22-43) carries `regions: BTreeMap<…,RegionDefinition>`
    + `subregions: BTreeMap<…,SubregionDefinition>`. `ContentCatalog` does NOT.
  - `oathstar_content::load_beginner_world() -> Result<WorldDefinition>` has REAL
    data: **3 regions** (hollowmere / ashen_road / old_bell_tower) + **5 subregions**
    (town / wall / wilds / tower / boss). `RoomDefinition` carries `region` +
    `Option<subregion>` → room counts.
- **Forge recall (pre-flight green; no bulletins):** `PR-claude-gated-page-role-mutant-001`
  (Player + anon redirect per gated route), `PR-claude-assert-element-form-not-substring-001`
  (render markers as element/name forms — `STUDIO_CSS` embeds theme tokens). The
  #49 section pattern (`sections.rs` gated handlers + `render.rs` page +
  `gate_tests!`) is the template.
- **Architecture sketch (Phase 2 firms up):** `StudioState` gains
  `world: Arc<WorldDefinition>` (loaded once in `main()` via
  `load_beginner_world()?`, like the catalog — keeps the handler unwrap-free);
  `sections::regions` becomes a real Editor-gated handler →
  `render::regions_page(&studio.world)`; the page lists regions → sub-regions
  (`subregion.region == region.id`) + room counts (group `world.rooms`), each
  sub-region → `/editor`. The `studio()` test helper needs a `world` too.
- **Scope OUT:** editing + persistence (#51b), per-sub-region map identity +
  create/delete (#51c), richer attributes.
- **EARS reviewed:** REQ-001..006 — list regions+subregions, room counts, editor
  links, gated, active nav, gate green.
- **Ticket:** #51 `341c0863-3fdc-49cd-a438-18a4f5d827f2` (feature, exists — not
  re-minted).
- **AAR id:** 6ebd92fe-a8df-45d1-a7b6-5a6802becbad

## Phase 2 — Design

### Approach / architecture
All server-rendered Rust in `oathstar-studio`, reading the existing world model
read-only. State/view split kept: data load in `main()`/state, render in `render.rs`.

- **Import path resolved:** the studio depends only on `oathstar-content`, which
  **re-exports `WorldDefinition`** and exposes `load_beginner_world() ->
  anyhow::Result<WorldDefinition>`. So `oathstar_content::WorldDefinition` +
  `oathstar_content::load_beginner_world` — no new dep.
- **`StudioState` (main.rs)** gains `world: Arc<WorldDefinition>`. `main()` loads it
  beside the catalog: `world: Arc::new(oathstar_content::load_beginner_world()?)`
  (`?` on the `anyhow::Result`, same as `beginner_catalog()?` — the handler path
  never unwraps). Add `WorldDefinition` to the `oathstar_content` import.
- **`sections::regions`** (replaces the stub): Editor-gated, renders the real page —
  ```rust
  editor_gate(&jar, &studio.sessions).map_or_else(
      || Redirect::to("/login").into_response(),
      |_principal| render::regions_page(&studio.world).into_response())
  ```
  `items`/`enemies`/`settings` keep using `gated_section` (the stub path).
- **`render.rs`:** `use oathstar_content::WorldDefinition;` + a `pub fn
  regions_page(world: &WorldDefinition) -> Html<String>` and a private
  `room_counts(world) -> (BTreeMap<&str, usize>, BTreeMap<&str, usize>)` (one pass
  over `world.rooms`, grouping by `room.region` and by `room.subregion`).
  `regions_page`: `body class="dashboard"`, `studio_header(Some(NavSection::Regions))`,
  then **one `.panel` per region** (BTreeMap order → deterministic) with
  `<h2>{region.name}</h2>`, its room count, and a `<ul>` of its sub-regions
  (`world.subregions.values().filter(|s| s.region == region.id)`), each `<li>` =
  `{sub.name}` + its room count + `<a href="/editor">Open in editor</a>`. Built with
  `core::fmt::Write` + `write!` into a `String` (NOT `map`+`format!`+`collect` —
  avoids `clippy::format_collect`, mirrors `studio_header`).
- **XSS:** region/sub-region names are server-controlled world content (the committed
  beginner TOML, not request input) — same posture as `principal.name`; no escaping
  needed this slice (a real user-authored world store would revisit it — out of
  scope, read-only).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-studio/src/main.rs` | `StudioState` += `world: Arc<WorldDefinition>`; `main()` loads `load_beginner_world()?`; import `WorldDefinition`. |
| 2 | `crates/oathstar-studio/src/sections.rs` | `regions` handler → real `render::regions_page(&studio.world)`; the `studio()` **test helper** += `world` (via `load_beginner_world().expect(...)`); adapt the `regions` gate test (now asserts a region name, not "Coming soon"). |
| 3 | `crates/oathstar-studio/src/render.rs` | + `regions_page` + `room_counts`; import `WorldDefinition`; + render tests (use `load_beginner_world()`). |

### Regression Test Plan
| # | Test | Proves Req |
|---|---|---|
| RT-1 | `render`: `regions_page(&beginner_world)` contains the 3 region names (Hollowmere / The Ashen Road / The Old Bell Tower) AND the 5 sub-region names, with each sub-region after its parent region in the string (nested) | REQ-001 |
| RT-2 | `render`: the page shows a room count per region + sub-region — the test counts `world.rooms` by region/subregion **independently** and asserts the page contains that exact count for a known region (pins the `room_counts` `+=` logic) | REQ-002 |
| RT-3 | `render`: each sub-region `<li>` carries `<a href="/editor">` | REQ-003 |
| RT-4 | `render`: the page nav marks Regions active — `<a href="/regions" aria-current="page">Regions</a>` (element form) | REQ-005 |
| RT-5 | `sections`: `regions` with an Editor session → 200 + body names a real region ("Hollowmere"); an Owner session → 200 | REQ-001 (real page, not the stub) |
| RT-6 | `sections`: `regions` with a Player session → redirect `/login`; no session → redirect `/login` (PR-claude-gated-page-role-mutant-001) | REQ-004 |
| RT-7 | `bin/gate.sh` FULL green — mutation 100% MSI (the `regions_page` markers + `room_counts` logic pinned by RT-1/2; the gate by RT-5/6) | REQ-006 |

**Genuinely uncoverable:** none — server-rendered Rust, fully exercised.

### Risks / decisions
- **World loaded once at startup** (not per request) — `Arc<WorldDefinition>` in
  state; cheap clone, no repeated TOML parse, no handler unwrap. (reversible)
- **`regions` diverges from `gated_section`** (it renders a real page, not a stub) —
  the other three sections stay on `gated_section`. Minor asymmetry, intended.
- **Tests use the real beginner world** (`load_beginner_world()`) so they assert
  real region names + counts — couples the test to the committed beginner content
  (acceptable; a content change that renames a region SHOULD flag the test). `expect`
  in the test helper is test-only (allowed).
- **`room_counts` returns borrowed `&str` keys** (tied to `world`); if clippy/
  lifetimes object, fall back to owned `String` keys. (reversible)
- **Read-only** — no create/edit/delete/persistence; the sub-region link is the
  general `/editor` (per-map identity deferred).

## Phase 3 — Implement
- **Built:**
  - `main.rs`: `StudioState` gained `world: Arc<WorldDefinition>`; `main()` loads
    `oathstar_content::load_beginner_world()?` beside the catalog; import updated.
  - `render.rs`: `room_counts(world)` (one pass over `world.rooms` → per-region +
    per-sub-region tallies, borrowed `&str` keys) + `pub fn regions_page(world)`
    (a `.panel` per region with `<h2>name</h2>`, room count, a `<ul>` of its
    sub-regions filtered by `s.region == region.id`, each linking `/editor`; built
    with `write!`, header active = Regions). + the render test (real beginner world:
    the 3 regions + 5 sub-regions, the editor links, Regions active, and
    Hollowmere's room count via an independent count).
  - `sections.rs`: `regions` now renders the real `render::regions_page(&studio.world)`
    (others stay stubs); the `studio()` test helper gained a `world`; a dedicated
    `regions_serves_the_dashboard_and_is_gated` test (Editor 200 + "Hollowmere" +
    not "Coming soon"; Player + anon → `/login`) replaced the stub gate-test;
    module doc updated.
- **Deviations from design (+ reason):**
  1. **`oathstar-content` re-export added** — `WorldDefinition` was only a *private*
     `use` in oathstar-content, yet `load_beginner_world()`/`materialize()` already
     **return** it, so callers couldn't name the type (E0603). Closed the latent API
     gap with `pub use oathstar_core::WorldDefinition;` (1 line; the studio reaches
     it via its existing oathstar-content dep — no new dep). The #51 diff now
     includes `crates/oathstar-content/src/lib.rs`.
  2. **Two more test-helper updates** — `StudioState` gained `world`, so the
     `handlers.rs` and `editor.rs` test `studio()` helpers also needed the field
     (both load the beginner world; `expect` is test-only). The design named only
     the sections.rs helper.
- **Checks:** `cargo clippy -p oathstar-studio --tests` + `-p oathstar-content`
  strict-clean; `cargo test -p oathstar-studio` **38 passed**; `cargo fmt` applied.
  No `unwrap`/`expect` on production paths (the world load uses `?` in `main()`).

## Inspect (Phase 3.5)
- **Lenses (2 critics):** A = correctness + XSS + the oathstar-content API change +
  state; B = mutation-readiness (empirical `cargo mutants`) + scope.
- **Findings:**
  | # | Severity | Finding | Verdict | Fix |
  |---|---|---|---|---|
  | — | info | **Correctness CLEAN** — `room_counts` cross-checked against the beginner module (8 rooms → regions 4/1/3, sub-regions town 3 / wall 1 / wilds 1 / tower 2 / boss 1); `regions_page` deterministic (BTreeMap); the gate is byte-equivalent to the other sections (Owner grants Editor); no `unwrap` on a production path (world loaded with `?` in `main()`); the orphan-region / orphan-sub-region branches are defensive (won't panic) + unexercised by beginner data — acceptable read-only v1. | CLEAN | none |
  | — | info | **XSS CLEAN** — every interpolated value (region/sub-region names, counts) comes from the loaded world (committed beginner TOML), not request input; the `principal` is discarded (not even rendered). Same server-controlled posture as `dashboard_page`. | CLEAN | none |
  | — | info | **API re-export CLEAN** — `pub use oathstar_core::WorldDefinition;` is minimal + correct (the crate's `load_beginner_world`/`materialize` already return it), no duplicate/shadow, and cleaner than adding an `oathstar-core` dep to the studio (keeps oathstar-content the single content seam). All 4 `StudioState` construction sites carry the new `world`. | CLEAN | none |
  | 1 | **high** | **Mutation: 2 survivors** (Critic B ran `cargo mutants`: 79 mutants → 2 **missed**). `render.rs:139` `+= → *=` on the **sub-region** count (`0 *= 1` stays 0 — every sub-region silently zeroes) and `render.rs:159` `== → !=` on the `s.region == region.id` **nesting filter** (sub-regions still print *somewhere*, so `contains()` misses the mis-nesting). The test pinned only the region count + sub-region name-presence. | **REAL** | **FIXED** — strengthened the render test to assert a sub-region's **exact count fragment scoped to its parent region's panel slice** (Hollowmere Town's count *inside* the Hollowmere `<h2>…</h2>…</section>` slice, + Town Wall in / Roadside Wilds out). `cargo mutants --file render.rs` re-run: **42 caught / 30 unviable / 0 missed**. |
  | — | info | **Scope CLEAN** — only #51 files (+ the content re-export + the 2 test-helper updates) + pipeline docs; online-first WIP untouched; `main()` excluded from mutation by project policy (no `main.rs` mutant). | CLEAN | none |
- **Captured:** `failure-record` **BF-claude-unpinned-nested-count-and-nesting-001**
  + `prevention-rule-record` **PR-claude-pin-nested-counts-and-placement-001**
  (the hierarchy-rendering test gap).

## Phase 4 — Validate
- **Tests (written Phase 3 + strengthened at inspect; cover RT-1..7):** `render`
  `regions_page_lists_regions_subregions_counts_and_links` (the 3 regions + 5
  sub-regions, the `/editor` links, Regions active, Hollowmere's region count, and
  — added at inspect — Hollowmere Town's sub-region count **scoped to the Hollowmere
  panel** + nesting); `sections` `regions_serves_the_dashboard_and_is_gated`
  (Editor 200 + "Hollowmere" + not "Coming soon"; Player + anon → `/login`).
- **`cargo test --workspace`:** all crates green, **0 failed** (527 passed,
  incl. the unchanged `oathstar-content` after the re-export).
- **`node --test tests/*.test.js`:** **77 pass, 0 fail** (JS untouched).
- **`bin/gate.sh` (FULL):** `GATE GREEN [full]` — **17/17**.
  - gate:15 rust coverage **98.72%** line (floor 94); gate:16 js **89.22%** (floor
    75); gate:17 mutation **540 caught / 0 missed → MSI 100.0%** (+31 over #50's 509
    — the region/room-count + handler mutants, all killed after the inspect fix).
- **Genuinely uncoverable:** none.
- **Pre-existing exclusions:** none. Online-first WIP stays out — selective staging.

## Phase 5 — Complete
- **Docs updated:** `docs/map-system.md` — a "Regions dashboard (ticket #51, slice 1)"
  note under the studio section (read-only list of regions + sub-regions + room
  counts, each sub-region → editor; editing/persistence deferred). No `decisions.md`
  change.
- **Forge capture:**
  - `failure-record` **BF-claude-unpinned-nested-count-and-nesting-001** +
    `prevention-rule-record` **PR-claude-pin-nested-counts-and-placement-001**
    (recorded at inspect — the hierarchy-rendering mutation gap).
  - `aar-submit` 6ebd92fe — outcome completed, effectiveness 4; materialized the
    BF + PR (2 novel findings). Win: the Explore mapped the exact data flow so the
    dashboard rendered real data first try; the inspect critic empirically ran
    cargo-mutants and caught 2 survivors before the gate. Bit: a render test that
    pins only the top-level count + child name-presence leaves nested-count +
    nesting mutants alive; oathstar-content needed a `WorldDefinition` re-export.
- **Ticket NOT closed (multi-slice):** `ticket-comment` on #51 — slice 1 shipped;
  **#51 stays open** for #51b (editing + persistence) + #51c (map identity +
  create/delete).
- **Archived:** spec+notes → `docs/planning/pipeline/completed/`. `TICKET-51` stays
  in `tickets/open/`.
