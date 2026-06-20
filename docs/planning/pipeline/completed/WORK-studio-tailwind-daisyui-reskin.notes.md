# WORK-studio-tailwind-daisyui-reskin — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Redo the studio / Host Manager UI with Tailwind v4 + DaisyUI — the
  current UI is clunky and needs a cleaner component system. (Owner answered the
  three `/work` forks: surface = Studio only; tooling = Tailwind engine + DaisyUI
  on studio; bite = full redesign of the surface.)
- **Intake source:** none (direct owner request via `/work`).
- **Classification / tier:** `feature`, forge ticket **#66**. A larger feature
  delivered as **one shippable slice** (the full studio re-skin). The full surface
  is only ~6 page-render fns + 251 lines of CSS; converting it wholesale avoids a
  broken half-skinned interim (a partial slice would replace `studio.css` out from
  under un-converted pages). The genuinely-new infra is the **node Tailwind build +
  gate no-drift wiring**, which is bounded. **Split offered to the owner at the
  review gate** as a de-risking fallback (Slice 1 = toolchain + shell; Slice 2 =
  editor + regions) if they'd rather stage it.
- **Forge recall (lessons/failures surfaced):**
  - **Decision 034 (Locked)** — first-party UI = Datastar/SSE HTML; *styling
    direction* says use project-owned CSS for the player client and **do not adopt
    DaisyUI for the player client now**, but **DaisyUI can be reconsidered for the
    Host Manager**. → This ticket is studio-only and decision-sanctioned.
  - **Decision 060 (Locked)** — the studio serves **content** PNGs from a runtime
    dir but keeps **code (CSS/JS) embedded** via `include_str!`. → The compiled
    Tailwind CSS is code → stays embedded/inlined; no new served-asset route.
  - **`PR-claude-serve-every-asset-the-glue-fetches-001`** — serve every
    asset/endpoint URL the glue references in the same slice. → If the Tailwind
    output references any asset (e.g. a webfont), serve it in the same slice;
    prefer system fonts to avoid this.
  - **`PR-claude-assert-element-form-not-substring-001`** — assert element *form*,
    not a bare substring (folded into REQ-002/003).
  - WORK-studio-fantasy-theme-foundation-v1 (#50 slice 1, completed) is the theme
    this re-skin supersedes (`studio.css:185–202` `border-image` block + the
    `pages_reference_the_ui_kit_assets` test).
- **Explore findings (studio frontend surface):**
  - HTML is emitted from `format!`/`write!` literals in **`render.rs`** (login,
    `studio_header`, `section_stub_page`, `regions_list_page`, `region_editor_page`,
    `dashboard_page`, `editor_page`); other modules delegate to it. Class names are
    inline `class="..."`.
  - `static/studio.css` (251 lines) is `include_str!`'d at `render.rs:8` and inlined
    as `<style>{STUDIO_CSS}</style>` on every page — **not** a linked file.
  - The two static JS modules are **pure functions** (no runtime DOM/class output).
  - **No Datastar** in the studio — interactivity is `fetch` + inline glue
    (`EDITOR_GLUE`, `REGIONS_GLUE`) reading `data-*` app-state hooks.
  - ~30 HTML-substring test assertions in `render.rs:957–1416` key off the old
    class names and will need rewriting.
- **Ticket:** 5dd351ba-7422-498a-8a50-1ef8df9c3019 (#66); local doc
  `docs/planning/tickets/open/TICKET-66-studio-tailwind-daisyui-reskin.md`.
- **EARS requirements reviewed:** REQ-001..006 in the spec (build no-drift,
  embedded DaisyUI CSS, component classes present, interactivity hooks preserved,
  #50 sprites de-referenced, gate FULL green). Appearance is owner-judged.
- **AAR:** aar_id `2b9e4549-1a47-4bdd-a8d3-585564dbc0f0` (opened at Phase 1 closeout; `inspect`/`complete` capture into it).

## Phase 2 — Design

### Approach / architecture
**Idiomatic-DaisyUI (Approach A):** the studio's HTML `class="..."` literals (all in
`render.rs`, 7 page fns) are rewritten to **Tailwind v4 utilities + DaisyUI component
classes**; a node Tailwind build **content-scans the studio `.rs` sources**, tree-shakes,
and compiles the stylesheet. The CSS stays `include_str!`-embedded and inlined per page
(Decision 060 — code/CSS embedded; no new route). Rejected Approach B (keep semantic
classes, restyle via `@apply`) because REQ-003 wants the component classes *in the rendered
HTML*, and B keeps the generic feel DaisyUI is meant to remove.

All HTML lives in **`render.rs` only** — `editor.rs:403 editor_page` is the async auth
handler that delegates to `render::editor_page(doc_json)`. So the class rewrite is one file.
The two static JS modules emit no DOM/classes (pure fns) → no scanning needed beyond `.rs`.

**Entry CSS — `crates/oathstar-studio/static/studio.tw.css` (NEW), Tailwind v4 CSS-first:**
```css
@import "tailwindcss";
@plugin "daisyui";
@plugin "daisyui/theme" {            /* custom theme so it does NOT feel generic (Decision 034) */
  name: "oathstar"; default: true; color-scheme: dark;
  --color-base-100: #0e1215; --color-base-200: #151a1f; --color-base-300: #28323b;
  --color-base-content: #e6ded0; --color-primary: #e5c56f; --color-primary-content: #101318;
  --color-error: #d98a8a; --radius-box: 0.5rem; --radius-field: 0.375rem;
}
@source "../src";                    /* scan render.rs class literals (relative to this file) */
@utility pixelated { image-rendering: pixelated; }   /* Tailwind has no built-in */
@layer components {                  /* preserve glue contracts WITHOUT touching the JS */
  #result, #region-result, #room-result { white-space: pre-wrap; }
  [data-ok="true"]  { /* success border/text */ }
  [data-ok="false"] { /* danger  border/text */ }
  .tab[aria-selected="true"] { /* active tab — glue toggles aria-selected, not a class */ }
}
```
Exact DaisyUI v5 theme/plugin syntax is finalized at implement against the installed version.

**Build wiring (deterministic — load-bearing for the gate):** the output **overwrites
`static/studio.css`** so `render.rs:8 include_str!("../static/studio.css")` is unchanged.
Versions are **pinned exact** (no `^`) and `package-lock.json` committed so every machine
produces byte-identical CSS. `--minify` (smaller inlined payload; markers chosen minify-stable).

**Component mapping (old semantic class → new):**
| Surface (render.rs) | Old | New (DaisyUI + Tailwind v4) |
|---|---|---|
| login body / card / input / error | `body.login` `.card` `input` `.error` | `min-h-screen grid place-items-center bg-base-100 p-4` · `card bg-base-200 shadow-xl` + `card-body` · `input input-bordered w-full` · `alert alert-error` |
| dashboard / panel / cta / who / soon | `body.dashboard` `.panel` `.cta` `.who` `.soon` | container utilities · `card bg-base-200 shadow` + `card-body` · `btn btn-primary btn-sm` (or `link link-primary`) · `text-base-content/60` · `italic opacity-60` |
| nav shell | `.studio-header` `.studio-nav` `.brand` `a[aria-current]` | `navbar bg-base-200` + `navbar-start/end` · `menu menu-horizontal` · brand `btn btn-ghost text-xl text-primary` · active via existing `aria-current` (tiny custom rule) |
| editor layout / tabs / panels | `.editor-main` `.tab-bar` `.tab` `.tab-panel` | `grid lg:grid-cols-[minmax(0,1fr)_minmax(19rem,23rem)] gap-4 items-start` · `tabs tabs-bordered` · **`tab` kept** (now DaisyUI) · panel `card ... ` keeping `hidden`/`data-tab`/`aria-*` |
| editor rail / controls / canvases | `.editor-rail` `.controls` `canvas#map/#palette` `.map-scroll` `.palette-scroll` | grid/flex utilities · keep `id=map/palette` + `pixelated` utility + `cursor-crosshair` |
| result blocks | `#result[data-ok]` … | keep ids + `data-ok`; styled by the `@layer` rule above (glue unchanged) |
| regions table / search | `.regions-table` `.table-search` | `table table-zebra` · `input input-bordered input-sm` |
| region editor forms | `.panel` `.edit/.delete` `textarea` `select` | `card` · `btn`/`btn-error btn-outline` · `textarea textarea-bordered` · `select select-bordered` |

**Preserved verbatim (REQ-004):** every `id`, `name`, `role`, `aria-*`, `hidden`, and
`data-*` (`data-tab`, `data-ok`, regions-row `data-title/id/regions/subs`, sort `data-key`)
attribute, and the inlined `EDITOR_GLUE` / `REGIONS_GLUE` scripts. Only `class="..."` values
change. (Many existing test assertions key off these functional attributes and survive as-is.)

**Gate integration — new `gate:18 studio-css (no-drift)` in `bin/gate.sh`** (after gate:14,
runs in BOTH fast+full — cheap): builds the entry CSS to a **temp** file via the local
`node_modules/.bin/tailwindcss` and `cmp`s it against the committed `static/studio.css`;
non-mutating; guarded with a clear "run `npm install` / `npm run studio:css`" message if the
bin is missing or the file is stale. Header comment updated 17→18 gates.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `package.json` | + devDeps `tailwindcss`, `@tailwindcss/cli`, `daisyui` (pinned **exact**); + scripts `studio:css` (build → `static/studio.css`) and `studio:css:check` (build-to-temp + `cmp`). |
| 2 | `package-lock.json` | regenerated by `npm install`; committed for reproducible `npm ci`. |
| 3 | `crates/oathstar-studio/static/studio.tw.css` **(NEW)** | Tailwind v4 entry: `@import`/`@plugin daisyui`/custom `oathstar` theme/`@source ../src`/`@utility pixelated`/`@layer` glue-contract rules. |
| 4 | `crates/oathstar-studio/static/studio.css` | **REGENERATED** — compiled Tailwind+DaisyUI output replacing the 251-line hand-written file. `include_str!` path unchanged. |
| 5 | `crates/oathstar-studio/src/render.rs` (7 page fns) | rewrite `class="..."` literals per the mapping; wrap tab-bar in `tabs`; nav→`navbar`/`menu`; `.panel`→`card`; `button`→`btn`; `.regions-table`→`table`; `.error`→`alert`. Preserve all id/name/role/aria/hidden/data-* + the glue. |
| 6 | `crates/oathstar-studio/src/render.rs` (`mod tests`) | update the class-keyed assertions to the new component classes; replace `pages_reference_the_ui_kit_assets` with a DaisyUI-marker-present + `/ui/*.png`-absent pair (REQ-002/005). |
| 7 | `bin/gate.sh` | add `gate:18 studio-css (no-drift)`; update header 17→18. |
| 8 | `docs/decisions.md` | **(Phase 5)** add Decision 061: studio adopts Tailwind v4 + DaisyUI (Host-Manager, sanctioned by Decision 034), superseding #50's studio fantasy theme; `/ui/*.png` routes dormant (Decision 060 intact). |

### Regression Test Plan
| # | Test | Proves Req |
|---|---|---|
| RT-1 | `bin/gate.sh` **gate:18**: `tailwindcss` temp build `cmp`s byte-equal to committed `static/studio.css`; a hand-edited stale CSS fails the gate. | REQ-001 |
| RT-2 | `render` test: `STUDIO_CSS` carries the custom-theme marker (brass `--color-primary` token / a DaisyUI component selector), asserted by **form not bare substring** (`PR-claude-assert-element-form-not-substring-001`). | REQ-002 |
| RT-3 | `render` tests per page fn: `dashboard_page` nav → `class="navbar …"` + `menu`; `.panel`→`card`; CTAs→`btn`; `regions_list_page` → `table`; `editor_page` tab bar → `tabs` with `tab` children. | REQ-003 |
| RT-4 | `render` tests: `editor_page` still emits `data-tab` / `aria-selected` / `id="result|region-result|room-result"`; regions rows keep `data-title/id/regions/subs` + sort `data-key`; `EDITOR_GLUE`/`REGIONS_GLUE` still inlined. Existing `node --test` JS-module suites stay green. | REQ-004 |
| RT-5 | `render` test: `STUDIO_CSS` does **not** contain `/ui/panel-frame.png` or `/ui/button.png`. | REQ-005 |
| RT-6 | `bin/gate.sh` FULL green incl. gate:18; mutation **100% MSI**. | REQ-006 |

**Genuinely uncoverable:** the rendered *appearance* ("clean / un-clunky") — owner-judged on
the running loopback studio (cannot self-screenshot). The gate proves the wiring.

**Mutation note:** the re-skin adds **no new Rust functions** — only string-literal edits
inside existing `render.rs` fns, test rewrites, and a bash gate step. Mutation surface is
essentially unchanged; the existing render fns stay pinned by RT-3/RT-4, and the generated CSS
+ bash gate are non-Rust (outside mutation scope). 100% MSI holds by keeping render assertions
strong.

### Risks / decisions
1. **Deterministic build is load-bearing** for gate:18 → pin exact versions, commit
   `package-lock.json`, build-to-temp + `cmp` (non-mutating). If any nondeterministic byte
   appears, the gate flaps — verify reproducibility twice at implement. Reversible fallback:
   "commit + manual rebuild, no gate" (weaker guarantee).
2. **DaisyUI generic-feel** (the exact worry in Decision 034) → mitigated by the custom
   `oathstar` theme (brass `#e5c56f` on dark `#0e1215`), not a stock theme. Owner tunes tokens
   in one place (`studio.tw.css`) after viewing. Reversible.
3. **Glue contracts** (`[data-ok]`, `aria-selected` tab-active) kept via a tiny `@layer` in the
   entry CSS — **no JS change** → REQ-004 safest. Alternative (glue toggles classes) rejected.
4. **Inlined CSS size** — tree-shaken + minified, localhost tool → non-issue; stays inlined
   (Decision 060), no new route.
5. **#50 dormant routes** — `/ui/*.png` + `ui.rs` + their serves-it tests stay (test the
   handler, not the CSS); only the CSS reference is dropped. Decision 060 intact. Removal = opt
   follow-up.
6. **Gate 17→18** — new run_gate; receipt/allowlist gates unaffected.
7. **Tailwind scanning Rust literals** — studio classes are static literals (no
   `format!("btn-{}")`), so the extractor sees them; `@source "../src"` is explicit. A missed
   class shows as an unstyled element (owner view catches it); build is deterministic.

## Phase 3 — Implement
- **Built (manifest as designed):**
  - `package.json` — devDeps **pinned exact**: `tailwindcss` 4.3.1, `@tailwindcss/cli`
    4.3.1, `daisyui` 5.5.23; scripts `studio:css` (build → `static/studio.css`,
    `--minify`) + `studio:css:check` (temp build + `cmp`). `package-lock.json` updated.
  - `crates/oathstar-studio/static/studio.tw.css` **(NEW)** — `@import "tailwindcss"`;
    `@plugin "daisyui" { themes: false }`; custom `@plugin "daisyui/theme"` **oathstar**
    (dark, brass `#e5c56f` primary, the old `:root` tokens); `@source "../src"`;
    `@utility pixelated`; `@layer components` for the glue-driven bits (result `[data-ok]`
    states, `.tab[aria-selected]`, `[role=tabpanel][hidden]`, glue-created `.region-row`/
    `.room-row`, `.regions-table` sort arrows).
  - `crates/oathstar-studio/static/studio.css` — **REGENERATED** (compiled output, 93 KB
    minified). Markers confirmed: navbar/card/btn/table/tabs/input/select/textarea/range/
    alert all present; brass `e5c56f` present; `/ui/*.png` references **0** (REQ-005);
    **deterministic** (two builds byte-identical). `include_str!` path unchanged.
  - `crates/oathstar-studio/src/render.rs` — class literals across **all 7 page fns**
    (login, `studio_header`, `section_stub_page`, `regions_list_page`,
    `region_editor_page`, `dashboard_page`, `editor_page`) rewritten to DaisyUI + Tailwind
    (navbar/menu nav, card panels, btn/btn-primary/btn-error, table table-zebra, tabs/tab,
    alert-error, input/select/textarea/range, utility layouts). **Every** `id` / `name` /
    `role` / `aria-*` / `hidden` / `data-*` attribute and both glue consts preserved
    verbatim; `regions-table` class **kept** (REGIONS_GLUE selector hook).
  - `bin/gate.sh` — `gate:18 studio-css (no-drift)`: temp Tailwind build + `cmp` vs the
    committed CSS (non-mutating; runs both modes; guards a missing bin). Header updated.
- **Checks:** `cargo fmt --all --check` clean; `cargo check -p oathstar-studio` ✓; `cargo
  clippy -p oathstar-studio --all-targets -- -D warnings` ✓; `bash -n bin/gate.sh` + `shellcheck`
  ✓; gate:18 build verified byte-identical to committed CSS.
- **Deviations from design (+ reason):**
  - **`card-body` wrapper added on the login card only** — the design said class-swap-only,
    but login's isolated card reads best with DaisyUI's `card-body` padding; zero ids/glue
    there, so safe. Every other panel uses `card … p-4` (no wrapper / no DOM restructure) as
    designed, preserving all ids + glue.
  - **`[role=tabpanel][hidden] { display:none }` added to `@layer`** — DaisyUI/utility display
    rules would otherwise override the `hidden` attribute the tab glue toggles; tab panels use
    `bg-base-200 rounded-box` (not `.card`) plus this rule, so the #61 tab switch still hides
    panels with **no glue change** (REQ-004).
  - **DaisyUI v5 emits its full component CSS** (components are not content-tree-shaken; only
    Tailwind *utilities* are). → 93 KB minified, inlined per page; fine for a loopback tool,
    and deterministic for gate:18.
  - **Test-assertion rewrites deferred to Phase 4** (per the implement contract): ~15 `render.rs`
    assertions key off old class names and currently fail at runtime; `cargo check`/`clippy`
    stay green. Validate rewrites them to assert the DaisyUI component classes (RT-2/3/4/5)
    and replaces `pages_reference_the_ui_kit_assets`.

## Inspect (Phase 3.5)
- **Lenses run** (3 parallel general-purpose critics + my own confirm): (A) glue/selector
  preservation, (B) Rust/format! correctness + test categorization, (C) gate:18 /
  build-determinism / Decision-060+034 alignment.
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | — | Inline-glue selectors vs new markup (`render.rs` REGIONS_GLUE/EDITOR_GLUE) | **CLEAN** — Critic A matched all 9 REGIONS_GLUE + 50+ EDITOR_GLUE DOM accesses to surviving ids/attrs; `regions-table` kept (+ DaisyUI `table`), tabs/`data-tab`/`role`/`data-ok`/`hidden` + the `%20` deep-link intact; `[role=tabpanel][hidden]` wins (panels use `bg-base-200`, not `.card`). | none |
  | 2 | — | `format!` integrity + no logic regression (`render.rs`) | **CLEAN** — Critic B: no stray `{}`/dropped args; **class-string-only** (`href`/`action`/`name`/`value`/`escape_html` all unchanged); `cargo check` + strict `clippy` green. | none |
  | 3 | (expected) | gate:3 red — 14 unit tests fail (12 `render.rs` + `editor.rs:1126` + `regions.rs:399`) | **REAL but BY-DESIGN** — all 14 are stale OLD-class / removed-`/ui` asset assertions; Critic B verified **0 behavioral breaks** (escaping, hrefs, `%20` intact). Test rewrites are Phase 4 (validate) per the implement contract + RT-2/3/4/5. | DEFERRED to validate (not an inspect defect) |
  | 4 | LOW | Dormant `/ui/*.png` routes + `ui.rs` handlers now CSS-unreferenced (`main.rs:82-83`, `ui.rs`) | **REJECTED as a fix** — the planner's explicit decision (keep dormant; Decision 060 intact; removal = optional follow-up). `ui.rs` serves-it tests still pass. | none (documented follow-up) |
  | 5 | — | gate:18 + Decision 060/034 + version pinning (`bin/gate.sh`, `package.json`, `main.rs`) | **CLEAN** — Critic C verified non-mutating / fail-closed / both-modes / deterministic / no-drift; CSS stays `include_str!`-embedded, no new route; exact pins + lockfile; player client untouched. | none |
- **Capture:** no `failure-record` — no real bug. The red tests are the scheduled validate work, not a defect; the markup-assertion-brittleness lesson is already covered by `PR-claude-assert-element-form-not-substring-001`.
- **Hand-off to validate — the 14 assertions to rewrite** (keep every FUNCTIONAL assertion — ids/`data-*`/hrefs/`%20`/escaped payloads — re-point only the class strings):
  - `render.rs`: `login_page_renders_the_error_banner` · `dashboard_page_has_shell_markers` · `dashboard_links_the_editor` · `editor_page_has_canvas_doc_and_controls` · `editor_page_has_a_tabbed_rail` · `section_stub_page_shows_coming_soon_and_active_nav` · `regions_list_page_lists_maps_with_counts_and_links` · `regions_list_page_shows_an_empty_state` · `region_editor_page_renders_panels_forms_counts_and_options` · `region_editor_page_renders_the_description_in_an_escaped_textarea` (⚠ keep asserting the escaped `&lt;/textarea&gt;…` payload — only add the new class) · `region_editor_page_links_each_subregion_into_the_editor` (keep the `%20`).
  - Cross-file: `editor.rs:1126 editor_page_renders_for_an_editor` (old `class="editor"`) · `regions.rs:399 region_editor_renders_for_an_editor` (old `<h2>Region</h2>`).
  - Replace `pages_reference_the_ui_kit_assets` → **REQ-005** (no `/ui/panel-frame.png`/`/ui/button.png` in `STUDIO_CSS`) **+ REQ-002** (DaisyUI `oathstar` theme marker present — brass `e5c56f` / a component selector, element-form not bare substring).
  - Add **RT-3** coverage (component classes `navbar`/`card`/`btn`/`table`/`tabs` present in rendered HTML) and **RT-1** (gate:18 drift). Then run `bin/gate.sh` FULL.

## Phase 4 — Validate
- **Tests rewritten/added** (kept every functional assertion — ids/`data-*`/hrefs/`%20`/escaped
  payloads — re-pointed only class strings to DaisyUI; element-form not brittle full-line):
  - 12 `render.rs` assertions re-pointed; 2 cross-file (`editor.rs` `class="editor"`→`editor-main`,
    `regions.rs` `<h2>Region</h2>`→`>Region</h2>`).
  - **New `studio_css_embeds_the_daisyui_theme_and_drops_the_fantasy_sprites`** (REQ-002 brass
    `e5c56f` + `.btn` component present; REQ-005 no `/ui/panel-frame.png`/`/ui/button.png`).
  - **New `studio_pages_use_daisyui_components`** (REQ-003: `navbar`/`card`/`btn`/`table`(+`regions-table`
    hook)/`tabs`/`input`).
  - The `region_editor_page_renders_panels_forms_counts_and_options` giant exact-string asserts
    rewritten to robust token forms (action/op/id/name/escaped-content + DaisyUI component classes).
  - **Production consistency fix found in validate:** the region-editor error banner still emitted
    `class="error"` (unstyled post-reskin) — made it `class="alert alert-error"` to match login; test
    updated. (No CSS rebuild needed — `alert alert-error` was already in the scan; gate:18 stayed green.)
- **`cargo test --workspace`:** GREEN — auth 20 · content 116 · core 300 · datastar 16 · protocol 27 ·
  server 40 · storage 23 · **studio 104** · 0 failed.
- **`node --test tests/*.test.js`:** **94 pass, 0 fail** (JS modules untouched).
- **`bin/gate.sh` (FULL):** **GATE GREEN [full] — 18/18.**
  - gate:18 studio-css (no-drift) ✓ · gate:2 strict clippy ✓ (fixed a `needless_raw_string_hashes`
    in a new test) · gate:15 rust coverage **98.74%** (floor 94) · gate:16 js **90.43%** (floor 75) ·
    gate:17 mutation **600 caught / 0 missed → MSI 100.0%** (+8 over the prior 592 — the new render.rs
    mutants all killed by the re-pointed assertions). Commit-gate receipt written.
- **REQ → proving test:**
  | REQ | Proven by |
  |---|---|
  | 001 CSS = fresh build, no drift | **gate:18** `studio-css (no-drift)` (temp build + `cmp`) |
  | 002 page embeds DaisyUI theme | `studio_css_embeds_the_daisyui_theme_and_drops_the_fantasy_sprites` (brass `e5c56f` + `.btn`) |
  | 003 component classes in HTML | `studio_pages_use_daisyui_components` + per-page asserts (navbar/card/btn/table/tabs) |
  | 004 interactivity hooks preserved | `editor_page_has_a_tabbed_rail` (`data-tab`/`role`/`hidden`), `regions_list_page_lists_maps…` (row `data-*`), `regions_list_page_wires_search_sort…`, `editor_page_wires_{save,activate,subregion_focus}`; + node JS-module suite |
  | 005 no #50 `/ui` sprites | `studio_css_embeds_…_drops_the_fantasy_sprites` (`!/ui/panel-frame.png`, `!/ui/button.png`) |
  | 006 gate FULL green @ 100% MSI | `bin/gate.sh` FULL — 18/18, MSI 100% |
- **Genuinely uncoverable:** the rendered *appearance* (clean/un-clunky) — owner-judged on the running
  loopback studio. Suggested smoke at Complete: rebuild + restart the studio, sign in (pw `oathstar`),
  eyeball the nav/cards/buttons/editor; tune theme tokens in `studio.tw.css` + `npm run studio:css`.
- **Pre-existing exclusions:** none. The dormant `/ui/*.png` routes + their `ui.rs` serves-it tests
  stay green (planner decision; Decision 060 intact).

## Phase 5 — Complete
- Docs updated:
- Forge capture (aar/failures/rules/decisions):
- Ticket closed:
- Archived:
