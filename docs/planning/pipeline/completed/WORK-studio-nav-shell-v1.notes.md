# WORK-studio-nav-shell-v1 — Notes

## Phase 1 — Plan
- **Request:** a persistent navigation shell for `oathstar-studio` so it grows from
  a single map editor into a multi-section admin (Maps · Regions · Items · Enemies ·
  Game Settings). Reparent the dashboard + `/editor` under the nav; unbuilt sections
  are Editor-gated "Coming soon" stubs. First slice (build order 1 of 4) of the
  pre-tilemap program.
- **Intake source:** `INTAKE-studio-admin-and-world-model-program.md` (the 4-ticket
  program: #49 nav → #50 UI kit → #51 regions → #52 world model).
- **Classification / tier:** work pipeline, one slice. Pure studio-UI change
  (server-rendered HTML + routes); no engine, no new data, no persistence. The gate
  rides on Rust render/handler tests (mutation 100% MSI) — the nav `format!` markers
  + the per-route gate tests carry it.
- **Forge recall (pre-flight green; no bulletins; no active pipeline):**
  - `PR-claude-gated-page-role-mutant-001` — every Editor-gated route needs BOTH a
    Player-session redirect test AND an Editor/Owner 200 test, or the `require_role`
    branch mutant survives. The four new section routes each need that pair.
  - Render `format!` marker-pinning (#45/#48): assert the nav labels/hrefs in the
    page tests so a blank-body / dropped-link mutant dies.
  - `PR-claude-serve-every-asset-the-glue-fetches-001` — only relevant if the nav
    introduces a fetched asset (it does not; plain server-rendered links).
  - Architecture (read this session): studio pages are `format!` HTML + embedded
    `STUDIO_CSS` (`render.rs`); Editor gate = `principal_from_cookie(&jar,
    &studio.sessions)` then `require_role(&principal, AuthRole::Editor)` (see
    `handlers::dashboard`, `editor::editor_page`); routes in `main.rs`.
- **Architecture sketch (for Phase 2 to firm up):** a shared `nav(active)` helper in
  `render.rs` returning the five links with an active marker; thread it into
  `dashboard_page` + `editor_page`; a generic `section_stub_page(section)` renderer;
  Editor-gated stub handlers + routes for `/regions`, `/items`, `/enemies`,
  `/settings` (consider a small shared gate helper mirroring `dashboard`).
- **Scope OUT:** the actual item/enemy/settings editors; the region dashboard (#51);
  the UI re-skin (#50); persistence; responsive/collapsible nav polish.
- **EARS reviewed:** REQ-001..006 — nav present + labels/hrefs, Maps→/editor,
  section stubs 200 (not 404), per-route Editor gate (player+none redirect), active
  marker, gate green.
- **Ticket:** #49 `7a030671-ba5a-4a08-b257-aadb69528115` (feature, exists — not
  re-minted).
- **AAR id:** cf74f189-cadf-409a-996f-a33e6cf4a721

## Phase 2 — Design

### Approach / architecture
All server-rendered Rust in `oathstar-studio` (no JS, no new data, no
persistence). State/view split honored: the gate (controller) stays in handlers;
the nav markup (view) lives in `render.rs`.

- **`NavSection` enum** (`render.rs`, `pub`, `#[derive(Clone, Copy, PartialEq)]`):
  `Maps | Regions | Items | Enemies | Settings`, with `label(self) -> &'static
  str` and `href(self) -> &'static str` (Maps→`/editor`, Regions→`/regions`,
  Items→`/items`, Enemies→`/enemies`, Settings→`/settings`) and a
  `const SECTIONS: [NavSection; 5]`.
- **`nav(active: Option<NavSection>) -> String`** (`render.rs`, private): emits
  `<nav class="studio-nav"><a class="brand" href="/">Oathstar Studio</a>` then one
  `<a href="{href}"{aria}>{label}</a>` per `SECTIONS`, where `{aria}` is
  ` aria-current="page"` iff `Some(section) == active`. The brand is the home
  link (preserves the editor page's existing `href="/"` assertion).
- **Shared header**: each authenticated page renders
  `<header class="studio-header">{nav(active)}<form method="post"
  action="/logout"><button>Sign out</button></form></header>`. The brand replaces
  the per-page `<h1>` and the editor's old `crumbs`.
- **`section_stub_page(section: NavSection) -> Html<String>`** (`render.rs`): a
  full page (`body class="dashboard"` to reuse styling) with the shared header
  `nav(Some(section))` and a `<section class="panel"><h2>{label}</h2><p
  class="soon">Coming soon.</p></section>`. One renderer backs all four stubs.
- **`dashboard_page`**: header `nav(None)` (home; no active section) + the existing
  "who"/panels body (kept → existing dashboard tests still pass; nav markers added
  to the test). The "Map editor" panel is now redundant with the nav's Maps link
  but is KEPT this slice (minimal churn).
- **`editor_page`**: header `nav(Some(Maps))` replacing the old crumbs; the
  `editor-main` body unchanged.
- **`sections.rs`** (NEW module): a local gate helper
  `editor_gate(jar: &CookieJar, sessions: &SessionStore) -> Result<Principal,
  Response>` (`principal_from_cookie` → `require_role(Editor)`; `Err` →
  `Redirect::to("/login").into_response()`), plus four thin Editor-gated handlers
  `regions` / `items` / `enemies` / `settings`, each:
  `match editor_gate(&jar, &studio.sessions) { Ok(_) =>
  render::section_stub_page(NavSection::X).into_response(), Err(r) => r }`.
- **`main.rs`**: `mod sections;` + four routes
  (`/regions`,`/items`,`/enemies`,`/settings` → the handlers).
- **`studio.css`**: `.studio-nav` (flex row), `.brand`, the
  `a[aria-current="page"]` active style, `.studio-header` layout — reusing the
  existing CSS tokens.

**Mutation-tightness (gate 17 is Rust here):** `label`/`href` match arms pinned by
nav-marker render tests (each label+href asserted); the `Some(s)==active` marker
pinned by an active-on-exactly-one test; `editor_gate`'s `require_role` branch
pinned by per-section Player-redirect + Editor-200 tests
(`PR-claude-gated-page-role-mutant-001`); each handler's `NavSection::X` pinned by
a section-title render assertion; `section_stub_page`'s `format!` pinned by the
"Coming soon" + label markers.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-studio/src/render.rs` | + `NavSection` enum (`label`/`href`) + `SECTIONS` + `nav(active)` + `section_stub_page`; thread the shared header (`nav`) into `dashboard_page` (active None) + `editor_page` (active Maps); extend the existing dashboard/editor render tests with nav markers (additive). |
| 2 | `crates/oathstar-studio/src/sections.rs` (NEW) | `editor_gate` helper + `regions`/`items`/`enemies`/`settings` handlers + their gate/render tests. |
| 3 | `crates/oathstar-studio/src/main.rs` | `mod sections;` + 4 routes. |
| 4 | `crates/oathstar-studio/static/studio.css` | `.studio-nav` / `.brand` / `.studio-header` / active-marker styles. |

`editor.rs` is **unchanged** (its handler calls `render::editor_page` whose output
changes, but the existing `editor_page_renders_for_an_editor` assertions — incl.
`<a href="/">` via the brand — still hold; nav markers are asserted in `render.rs`'s
editor test).

### Regression Test Plan
| # | Test | Proves Req |
|---|---|---|
| RT-1 | `render`: `nav(None)` contains all 5 labels + hrefs (Maps→/editor, Regions→/regions, Items→/items, Enemies→/enemies, Settings→/settings) | REQ-001, REQ-002 |
| RT-2 | `render`: `dashboard_page` body contains the nav (5 labels + hrefs) and no `aria-current` (home, nothing active) | REQ-001, REQ-002, REQ-005 |
| RT-3 | `render`: `editor_page` contains the nav with `aria-current="page"` on **Maps** and not the others | REQ-001, REQ-002, REQ-005 |
| RT-4 | `render`: `section_stub_page(Regions)` contains "Coming soon" + "Regions" + the nav with Regions active | REQ-003, REQ-005 |
| RT-5 | `render`: active marker lands on exactly the passed section — `nav(Some(Items))` marks Items, not Maps/Regions/Enemies/Settings | REQ-005 |
| RT-6 | `sections`: `regions` with an Editor session → 200 + body names "Regions"/"Coming soon"; with an Owner session → 200 | REQ-003 |
| RT-7 | `sections`: `regions` with a **Player** session → redirect `/login`; with **no** session → redirect `/login` | REQ-004 (role mutant) |
| RT-8 | `sections`: `items` / `enemies` / `settings` each → Editor 200 (names the section) AND Player → redirect `/login` (a small per-handler assertion helper) | REQ-003, REQ-004 |
| RT-9 | `bin/gate.sh` FULL green — mutation 100% MSI (all the `format!`/match/gate mutants pinned above) | REQ-006 |

**Genuinely uncoverable:** none — every path is server-rendered Rust, fully
exercised by `cargo test`.

### Risks / decisions
- **Dashboard "Map editor" panel kept** despite redundancy with the nav's Maps
  link — minimal churn this slice; #50/#51 can tidy. (reversible)
- **Editor crumbs replaced by the shared nav**; the brand `<a href="/">` keeps the
  existing `editor_page` `href="/"` assertion valid.
- **`editor_gate` is `sections.rs`-local**, not yet shared with the `dashboard`/
  `editor_page` handlers — DRY-ing those is deferred to avoid touching working,
  tested code. Mild duplication accepted. (reversible)
- **`NavSection` lives in `render.rs`** (view layer); `sections.rs` imports it.
- No new assets/fetches → `PR-claude-serve-every-asset-the-glue-fetches-001` N/A.

## Phase 3 — Implement
- **Built:**
  - `render.rs`: `pub enum NavSection` (`Maps|Regions|Items|Enemies|Settings`) with
    `const fn label`/`const fn href`; `const SECTIONS`; a `studio_header(active:
    Option<NavSection>)` helper (the whole `<header>` — brand home link + the
    section nav with `aria-current` on the active one + the sign-out form);
    `section_stub_page(section)`. Threaded `studio_header` into `dashboard_page`
    (`None`) and `editor_page` (`Some(Maps)`), replacing the editor's old crumbs.
    Extended the render tests (dashboard has the 5 nav links + no active link;
    editor has Maps active; new `section_stub_page` test: Coming soon + label +
    Regions active + Maps inactive).
  - `sections.rs` (NEW): `editor_gate(jar, sessions) -> Option<Principal>`
    (`principal_from_cookie` → `require_role(Editor)`); `gated_section` (redirect
    to `/login` when `None`); 4 handlers `regions`/`items`/`enemies`/`settings`;
    a `gate_tests!` macro emitting the 3-way gate test per route (Editor 200 +
    names section + "Coming soon"; Player → redirect; no-session → redirect) +
    an Owner-200 test.
  - `main.rs`: `mod sections;` + 4 routes. `studio.css`: `.studio-header` /
    `.studio-nav` / `.brand` / `a[aria-current="page"]`.
  - `editor.rs`: one test assertion updated (`<a href="/">` → `href="/"`) — the
    brand home link replaced the Dashboard crumb.
- **Deviations from design (+ reason):**
  1. **Folded `nav()` into `studio_header(active)`** returning the whole `<header>`
     (nav + sign-out) — DRYer across the three pages than a bare `nav()` plus a
     per-page header. Behaviour identical.
  2. **`editor_gate` returns `Option<Principal>`**, not `Result<Principal,
     Response>` — `Result<_, Response>` tripped `clippy::result_large_err` (a
     `Response` Err is large). The redirect moved into `gated_section` (one place);
     same behaviour. `gated_section` uses `map_or_else` (clippy
     `option_if_let_else`).
  3. **`editor.rs` got a one-line test change** (the design said "unchanged") — the
     `href="/"` brand assertion replaced the removed crumb.
  4. **Tests written here** (co-located, per the codebase pattern + the design RT
     plan) rather than deferred — Validate RUNS them + the full gate.
- **Checks:** `cargo clippy -p oathstar-studio --tests` strict-clean;
  `cargo test -p oathstar-studio` **34 passed**; `cargo fmt` applied. String build
  uses the `core::fmt::Write` + `let _ = write!` idiom (matches `session.rs`;
  avoids `format_collect`/`format_push_string`).

## Inspect (Phase 3.5)
- **Lenses (2 critics):** A = gate correctness + security/XSS + routing/reparent +
  panics; B = mutation-readiness (empirical) + simplification/reuse + scope.
- **Findings:**
  | # | Severity | Finding | Verdict | Action |
  |---|---|---|---|---|
  | — | info | **Gate correctness** — `editor_gate` (`principal_from_cookie? → require_role(Editor).ok()?`) is byte-equivalent to the inline gates in `handlers::dashboard` + `editor::editor_page`; Owner reaches stubs because `Principal::grants` is `roles.contains(Owner) \|\| contains(required)`; no anon/Player path to a 200 stub (the `gate_tests!` Player+anon legs assert 303→/login per route). | CLEAN | none |
  | — | info | **Security/XSS** — every new interpolation (`label()`/`href()`, the brand, `section_stub_page`) is a compile-time `&'static str`; `section_stub_page` takes the `NavSection` enum (not a string) and is reachable only from the 4 fixed handlers; redirect target is the constant `/login`. `principal.name` is the pre-existing owner-only field. No request data reaches the new markup. | CLEAN | none |
  | — | info | **Routing/reparent + panics** — the 4 routes go through `gated_section`; the `editor.rs` `href="/"` assertion correctly replaced `<a href="/">` (the brand markup changed, so the old literal had to go); dashboard still links `/editor`; `let _ = write!(String)` is genuinely infallible (matches `session.rs`); no unwrap/expect/index on input paths. | CLEAN | none |
  | — | **empirical** | **Mutation-readiness** — Critic B ran `cargo mutants -p oathstar-studio --file render.rs --file sections.rs`: **42 mutants → 17 caught / 25 unviable / 0 MISSED = 100% MSI**. The two CSS traps are sidestepped (`align-items`⊅capital-`Items`; the tests assert full anchor tags, never a bare `contains("aria-current")`). Items/Enemies labels pinned via the `gate_tests!` `body.contains($label)`. | CLEAN | none |
  | 1 | low | **`href()`↔route-table are two sources of truth** — a future section's `href()` added without the matching `main.rs` `.route(...)` would render a dead (404) nav link; handler-level tests never boot the assembled `Router`, and `main()` is mutation-excluded by project policy (`.cargo/mutants.toml`). | REAL but **forward-looking** | **DEFER** — all 5 hrefs have routes today; consistent with the project excluding `main()` from mutation. Candidate: an `app()` boot + per-`SECTIONS`-href non-404 integration test, naturally added when #51 grows the route set. |
  | 2 | low | **Gate duplication** — `editor_gate` repeats the `principal_from_cookie? + require_role(Editor)` shape also in `handlers::dashboard`, `editor::editor_page`, and `editor::validate` (whose JSON-`refuse` arm differs). | REAL but **predates #49** | **DEFER** — `editor_gate` already picks the reusable seam (`Option<Principal>`); consolidating the two pre-existing redirect handlers is a separate tidy, and `validate`'s arm wouldn't fit one helper. |
- **No `failure-record`:** no bug — both critics confirm correctness, and the diff is
  empirically mutation-tight (0 survivors). The two LOWs are deferred improvements,
  not defects.

## Phase 4 — Validate
- **Tests (written in Phase 3, co-located; confirmed covering RT-1..9):**
  - `render.rs`: `dashboard_page_has_shell_markers` (the 5 nav labels+hrefs, no
    active link — RT-1/2/5), `editor_page_has_canvas_doc_and_controls` (nav present,
    Maps active — RT-3), `section_stub_page_shows_coming_soon_and_active_nav`
    (Coming soon + label + Regions active + Maps inactive — RT-4/5).
  - `sections.rs`: the `gate_tests!` macro → 3-way gate per route (Editor 200 +
    names section + Coming soon; Player → /login; anon → /login — RT-6/7/8 +
    PR-claude-gated-page-role-mutant-001) + `an_owner_session_reaches_a_section`.
  - `editor.rs`: the `href="/"` brand assertion (the reparent).
- **`cargo test --workspace`:** all crates green, **0 failed** (oathstar-studio 34;
  oathstar-core 296; others green).
- **`node --test tests/*.test.js`:** **77 pass, 0 fail** (JS untouched this slice).
- **`bin/gate.sh` (FULL):** `GATE GREEN [full]` — **17/17**.
  - gate:15 rust coverage **98.73%** line (floor 94); gate:16 js coverage **89.22%**
    (floor 75); gate:17 mutation **507 caught / 0 missed → MSI 100.0%** (was 493;
    the +14 new render.rs/sections.rs mutants all killed — matches Critic B's
    pre-verification).
- **Pre-existing exclusions:** none. The online-first WIP + the already-committed
  backlog docs stay out of scope — selective staging at `/commit`.

## Phase 5 — Complete
- **Docs updated:** `docs/map-system.md` — added an "Inside a nav shell (ticket #49)"
  note to the studio section (the editor is the Maps section of a persistent nav;
  the other four are Editor-gated stubs; the shell #51/#50 build on). No
  `decisions.md` change (no decision made/changed).
- **Forge capture:**
  - `failure-record` **BF-claude-embedded-css-marker-substring-001** (test, low):
    the first nav test asserted "no active section" via `!contains("aria-current")`,
    which failed because the page embeds `studio.css` and that carries the selector
    `a[aria-current="page"]`. Fixed by asserting the Maps link's inactive element
    form instead.
  - `prevention-rule-record` **PR-claude-assert-element-form-not-substring-001**
    (low): for server-rendered pages that embed their own CSS/JS, assert the
    specific rendered element form, not a bare whole-page substring scan (the
    embedded asset can contain the token).
  - `aar-submit` cf74f189 — outcome completed, effectiveness 5; materialized the
    BF + PR (2 novel findings). Win: the inspect critic empirically ran
    cargo-mutants (0 survivors) pre-verifying the gate; the role-mutant rule drove
    the `gate_tests!` macro. Bit: 3 clippy-strict first-compile fixes
    (`result_large_err`→`Option`, `format_collect`→`write!`,
    `option_if_let_else`→`map_or_else`) + the embedded-CSS test trap.
- **Ticket closed:** #49 `7a030671-…` → done.
- **Archived:** spec+notes → `docs/planning/pipeline/completed/`; ticket doc →
  `docs/planning/tickets/closed/` (status closed).
