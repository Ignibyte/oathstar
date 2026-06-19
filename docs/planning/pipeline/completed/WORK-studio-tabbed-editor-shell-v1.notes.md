# WORK-studio-tabbed-editor-shell-v1 — Notes

## Phase 1 — Plan
- **Request:** the tabbed editor shell — controls to a right rail + a Tiles|Regions|Rooms|Map tab
  bar (Tiles active, rest stubbed). Pivot ③ slice a (memory `studio-editable-world-pivot`).
- **Classification / tier:** work pipeline, one slice, **layout-only**, `oathstar-studio` —
  `render.rs` (`editor_page` markup + EDITOR_GLUE), `studio.css`, `editor-canvas.js`
  (`tabPanelStates`) + tests. No game/engine change.
- **Recon (main `b5ca668`):** `render.rs editor_page:635-667` = `editor-main` →
  `div.editor-left { section.panel.canvas-panel (#map) ; section.panel.controls (#map-name/#save/
  #validate/#activate/#result) }` + `section.panel.palette-panel (#active-tile + #palette)`.
  EDITOR_GLUE + `editor-canvas.js` init the `#map`/`#palette` canvases and wire `#save`/`#validate`/
  `#activate` by id; existing tests `editor_page_has_canvas_doc_and_controls` (`:729`, asserts
  `#palette` etc.) + `editor_page_wires_the_{save,validate,activate}` assert ids + fetch.
- **Approach (design refines):** LEFT `canvas-panel` + RIGHT `aside.editor-rail` { doc-actions
  (controls moved, ids kept) ; `div[role=tablist]` (4 tabs, Tiles selected) ; 4 `[role=tabpanel]`
  (Tiles = palette visible ; Regions/Rooms/Map = hidden stubs) }. Pure `tabPanelStates(tabIds,
  activeId)` (first-tab fallback) + a tablist click handler + `:root`-token CSS.
- **EARS:** REQ-001 tablist+4 tabs · REQ-002 controls in rail (ids) · REQ-003 Tiles=palette
  visible / stubs hidden · REQ-004 tabPanelStates · REQ-005 existing wiring still green · REQ-006 gate.
- **Mutation surface:** `tabPanelStates` (`=== `selected, `!==` hidden, `includes`-fallback) — killed
  by REQ-004. The render markup is assertion-covered; the glue handler has no viable mutant (DOM
  side-effects, no operators) — the logic lives in `tabPanelStates`.
- **Risk:** the palette `<canvas>` must NOT start `hidden` (its init measures layout) → Tiles is the
  default visible tab. The existing `editor_page_has_canvas_doc_and_controls` test may need updating
  for the moved markup (it asserts `contains`, so most holds).
- **Ticket:** forge **#61** `e87f802b-cd8e-40c8-b382-6c9be8d26388`. Local doc
  `docs/planning/tickets/open/TICKET-61-studio-tabbed-editor-shell.md`.
- **aar_id:** `8701da5d-28b0-464e-952a-de14b0f4be2e`
- **Delivery:** AUTONOMOUS through commit+push+FF-merge. Branch off `main` `b5ca668`. Stash parked.

## Phase 2 — Design

### Code reconnaissance
- `studio.css`: `.editor-main` (`:72`, grid `1fr 1fr`), `.editor-left` (`:79`, wraps canvas+controls),
  `.map-scroll`/`.palette-scroll`/`.controls`/`#result`/`canvas#map`/`canvas#palette` rules; the
  `@media (max-width:760px)` collapses to one column; `.soon` (`:67`) already exists. There is **no**
  `.palette-panel` rule (only the `.panel` base + `.palette-scroll`), so dropping that class is free.
- The existing render tests assert **ids + glue that all survive** the relocation —
  `editor_page_has_canvas_doc_and_controls` (`:729`: `#map`,`#map-doc`,`#validate`,`#result`,
  `class="editor"`, the `editor*`/`paint*`/`fetch` glue, `#palette`, `/tilesets/`, the nav), the
  marquee test (`mousedown`/`mouseup`/`paintRect(`), the subregion-focus test (`?subregion`,
  `focusSubregion`). **All pass unchanged** (every asserted id/string is preserved). No edits to them.

### Approach / architecture (oathstar-studio, layout-only)
- **`render.rs editor_page`** — replace the `editor-main` body with: LEFT
  `section.panel.canvas-panel` (h2 + paint hint + `#map`), RIGHT `aside.editor-rail` holding (a) the
  **relocated `section.panel.controls`** (`#map-name`/`#save`/`#validate`/`#activate`/`#result` —
  **verbatim, same class**, so the `.controls` rule + the wiring tests still apply), (b)
  `div.tab-bar[role="tablist"]` with four `button[role="tab"]` (`id=tab-{tiles,regions,rooms,map}`,
  `data-tab`, `aria-controls=panel-{id}`; `tab-tiles aria-selected="true"`), (c) four
  `section.tab-panel[role="tabpanel"]` (`id=panel-{id}`, `data-tab`) — **Tiles** = the palette
  (`#active-tile` + `#palette`, verbatim, **visible**), Regions/Rooms/Map = `<p class="soon">Coming
  soon.</p>` (`hidden`). Update the `editor_page` doc comment.
- **`static/editor-canvas.js`** — `export function tabPanelStates(tabIds, activeId)`:
  `const active = tabIds.includes(activeId) ? activeId : tabIds[0];` →
  `tabIds.map(id => ({ id, selected: id === active, hidden: id !== active }))`. JSDoc sibling of
  `formatActivateResult`. Pure.
- **EDITOR_GLUE** — a `[role="tablist"]` click handler (defensive: `if (!tablist) return`): on a
  `[role="tab"]` click, read `data-tab`, compute `tabPanelStates(ids, clicked)`, and for each state
  set the tab's `aria-selected` + the panel (`document.getElementById("panel-"+id)`) `.hidden`.
  References `tabPanelStates` directly (same concatenated module).
- **`studio.css`** — rework `.editor-main` to **stage-left + rail-right**
  (`grid-template-columns: minmax(0, 1fr) minmax(19rem, 23rem)`); **remove** the now-unused
  `.editor-left`; add `.editor-rail` (grid column, gap, align-content start), `.tab-bar`
  (flex row, `border-bottom: 1px solid var(--line)`), `button[role="tab"]` (inactive: `--muted`,
  transparent) + `[role="tab"][aria-selected="true"]` (active: `--ink` + a `--brass` underline/bg),
  `.tab-panel` + `.tab-panel[hidden] { display: none; }`. All on the `:root` tokens. The `@media`
  one-column collapse still applies.

### Locked decisions (this phase)
- The actions block **reuses `class="panel controls"`** (relocated) — same styling, no new/dropped
  CSS rule, and the `#validate`/`#result` assertions keep matching. (Lighter than a `.doc-actions`
  rename.)
- Palette **stays visible** in Tiles (its `<canvas>` init measures layout) → Tiles is the default tab.
- `tabPanelStates` is the **only** new logic (node + mutation tested); the glue handler is DOM
  side-effect only (no viable mutant); the markup is render-assertion covered.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-studio/src/render.rs` | `editor_page` markup → stage + `editor-rail` (controls relocated, tab bar, 4 tabpanels); doc comment; the tablist click handler in `EDITOR_GLUE`; NEW `editor_page_has_a_tabbed_rail` test. |
| 2 | `crates/oathstar-studio/static/editor-canvas.js` | NEW pure `tabPanelStates`. |
| 3 | `crates/oathstar-studio/static/studio.css` | rework `.editor-main`; drop `.editor-left`; add `.editor-rail`/`.tab-bar`/`[role=tab]`/`.tab-panel`. |
| 4 | `tests/studio-editor-canvas.test.js` | `tabPanelStates` cases. |
| 5 | `docs/map-system.md` | (Phase 5) editor = stage + tabbed rail note. |

### Regression Test Plan
| # | Test | Proves |
|---|---|---|
| T1 | NEW `editor_page_has_a_tabbed_rail` — `role="tablist"`; the 4 `data-tab` ids; `aria-selected="true"` on `tab-tiles`; `#palette` appears after `id="panel-tiles"` (inside the Tiles panel); the 3 `hidden` `panel-{regions,rooms,map}` stubs with "Coming soon" | REQ-001/003 (rust) |
| T2 | (existing `editor_page_has_canvas_doc_and_controls` + the wiring tests) — `#map-name`/`#save`/`#validate`/`#activate`/`#result`/`#map`/`#palette` all still present | REQ-002/005 (rust, unchanged) |
| T3 | `tabPanelStates(['tiles','regions','rooms','map'], x)` — `x∈{'tiles','',' ','nope'}` → tiles selected + 3 hidden; `x='regions'` → regions selected; length 4; `[0].id==='tiles'` | REQ-004 (node) |
| G1 | `bin/gate.sh` FULL green, MSI 100% | REQ-006 |
- **Mutation:** `tabPanelStates` — `id === active` (selected), `id !== active` (hidden), and the
  `includes(activeId) ? : tabIds[0]` fallback — killed by T3. **No genuinely-uncoverable code** (the
  glue handler is DOM-only, not a mutants target; markup is assertion-covered).

### Risks / decisions
1. **Palette init** — must not start `hidden`; Tiles is the default visible tab. (If a later slice
   makes Tiles non-default, the palette canvas needs a resize-on-show.)
2. **`aria-selected="true"` is a literal** in the initial render (Tiles); the glue updates it on
   click. A render assert pins it.
3. **Mobile** — `.editor-main` collapses to one column (`@media`), stage over rail; still usable.
4. **No id/behavior change** — the canvas/palette init, paint, Save/Validate/Activate, subregion
   focus all key off ids that are preserved; this is purely a DOM reparent + CSS.

## Phase 3 — Implement
- **Built (manifest as designed):**
  - `render.rs editor_page` — `editor-main` is now LEFT `section.panel.canvas-panel` (#map) +
    RIGHT `aside.editor-rail` { the relocated `section.panel.controls` (ids verbatim) ;
    `div.tab-bar[role=tablist]` (4 `button.tab[role=tab]`, `tab-tiles aria-selected="true"`) ;
    4 `section.panel.tab-panel[role=tabpanel]` — `#panel-tiles` (the `#active-tile`+`#palette`
    palette, visible) ; `#panel-{regions,rooms,map}` `hidden` "Coming soon" stubs }. Doc comment
    updated.
  - `EDITOR_GLUE` — a `[role=tablist]` click handler (no-op without a tablist) applying
    `tabPanelStates` to each tab's `aria-selected` + each panel's `.hidden`.
  - `editor-canvas.js` — pure `tabPanelStates(tabIds, activeId)` (first-tab fallback) + JSDoc.
  - `studio.css` — `.editor-main` → `minmax(0,1fr) minmax(19rem,23rem)` (stage+rail); dropped
    `.editor-left`; added `.editor-rail`, `.tab-bar`, `.tab` + `.tab[aria-selected="true"]`
    (brass underline), `.tab-panel[hidden]`. `:root` tokens; the mobile `@media` collapse kept.
- **Deviations:** the actions block **reuses `class="panel controls"`** (relocated) rather than a new
  `.doc-actions` class — identical styling, zero CSS churn, and the `#validate`/`#result` assertions
  keep matching (as designed).
- **Checks:** `cargo check`/`clippy -p oathstar-studio --all-targets` clean; `cargo fmt` clean;
  `node --check editor-canvas.js` OK; `cargo test -p oathstar-studio` → **91 passed** —
  `editor_page_has_canvas_doc_and_controls` + all wiring (`save`/`validate`/`activate`) + marquee +
  subregion-focus tests **pass unchanged** (every id/glue string preserved). New tests + gate at
  Phase 4.

## Inspect (Phase 3.5)
- **Lenses:** 2 read-only `Explore` critics (no worktree mutation): correctness + simplification/
  consistency.
- **Critic 1 (correctness) — CLEAN.** Verified: `tabPanelStates` marks exactly one tab selected
  (`selected === !hidden`) with the first-tab fallback for unknown/blank/undefined `activeId`; the
  glue handler no-ops without a tablist / non-tab click and its `"panel-"+id` ids match the rendered
  `panel-{tiles,regions,rooms,map}`; `tabPanelStates` is in scope (concatenated module); all 8 ids
  (`#map`/`#palette`/`#active-tile`/`#map-name`/`#save`/`#validate`/`#activate`/`#result`) present
  once; the palette is in the **default-visible** Tiles panel so its init layout-measure works; the
  3 stubs are `hidden` + Tiles `aria-selected="true"`. **91 tests pass.**
- **Critic 2 findings — all REJECTED (false positive / hypothetical), no defect:**
  - **[high] "controls Save/Validate/Activate lack `type="button"`"** → **FALSE POSITIVE.** Verified
    `render.rs:677-679` — all three are `<button … type="button">` (moved verbatim from #55/#60). No
    change.
  - **[med] "`.tab` `--brass`/`--ink` in isolation"** → REJECTED. `studio.css:2-7` defines
    `--bg/--panel/--line/--ink/--muted/--brass` in `:root`; the new `.tab` rules use them exactly as
    the rest of the stylesheet does. Not a defect.
  - **[med] aria-controls/aria-labelledby DOM-order risk** → REJECTED. The pairs are correct
    (`tab-tiles`↔`panel-tiles`, …) and `data-tab` is the source of truth the glue uses; a future
    refactor reordering markup is not a current defect.
  - **[low] CSS cleanup / class reuse / defensive handler / tabPanelStates idiom** → all confirmed
    CLEAN (`.editor-left`/`.palette-panel` fully removed from markup **and** CSS; controls reuse
    `.panel controls`; the helper mirrors `filterRows`/`formatActivateResult`).
- **No code fix; no `failure-record`** (the only "actionable" item was a critic mis-read). The NEW
  render + node tests are Phase 4 scope (T1/T3).

## Phase 4 — Validate
- **Tests added** (T1 + T3):
  - `render.rs` — NEW `editor_page_has_a_tabbed_rail`: asserts `<aside class="editor-rail"`,
    `role="tablist"`, the 4 `data-tab` ids, the `tab-tiles … aria-selected="true"` (Tiles default),
    `#palette` index **after** `#panel-tiles` (palette inside the Tiles panel), and the 3 `hidden`
    `panel-{regions,rooms,map}` "Coming soon" stubs. The existing `editor_page_has_canvas_doc_and_
    controls` + wiring/marquee/subregion tests are **unchanged** (they pass).
  - `tests/studio-editor-canvas.test.js` — NEW `tabPanelStates` case: every
    `activeId ∈ {'tiles','','  ','nope',undefined}` → first tab selected + 3 hidden;
    `'regions'` → regions selected; length 4, exactly one selected, `selected === !hidden` ∀.
- **`cargo test --workspace`:** green — `oathstar-studio` **92 passed** (+1: tabbed rail); all other
  crates green.
- **`node --test tests/*.test.js`:** **90 passed / 0 fail** (+1: `tabPanelStates`).
- **`bin/gate.sh` FULL:** **GATE GREEN [full]** — 17/17. rustfmt, clippy strict, both suites, rust
  cov ≥94, js cov ≥75, **mutation 594 caught / 0 missed → MSI 100.0%**. Receipt written.
- **Pre-existing exclusions:** none.

## Phase 5 — Complete
- Docs / forge / ticket / archived:
