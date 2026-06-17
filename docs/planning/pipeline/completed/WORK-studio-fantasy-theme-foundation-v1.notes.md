# WORK-studio-fantasy-theme-foundation-v1 — Notes

## Phase 1 — Plan
- **Request:** adopt the mini-medieval fantasy UI kit (ticket #50, build order 2 of
  4). Sliced: **this pipeline = slice 1**, a reusable fantasy theme foundation on
  the STUDIO (the #49 nav header + `.panel`s + buttons). Game client + the rest of
  the kit are deferred #50 slices.
- **Intake source:** `INTAKE-studio-admin-and-world-model-program.md` (the 4-ticket
  program: #49 nav ✓ → **#50 UI kit** → #51 regions → #52 world model).
- **Classification / tier:** work pipeline, one slice. Mostly assets + CSS (the
  genuinely-uncoverable surface); the gate rides on the new asset route handler(s)
  + render markers (Rust mutation), mirroring #48 (arctic.png) and #49 (nav).
- **Art inspected (this phase):** `Frames.png` (896×472) — a grid of nine-slice
  panel borders in many palettes (brown/gold, silver, green, red, blue) + corners +
  segment bars; the brown/gold frames read most "fantasy". `Inputs.png` (240×368)
  — bar/pill button backgrounds (gold/brown, gray, cream), scroll-end + arrow
  buttons. So the highest-leverage foundation is **one nine-slice panel frame + one
  button**. Both need a single frame cropped out of the sheet (border-image needs a
  clean image). Tools present: `magick` + PIL 11.3.0 + `sips`.
- **Forge recall (pre-flight green; no bulletins):**
  - `PR-claude-serve-every-asset-the-glue-fetches-001` — the embedded UI asset(s)
    MUST get a served route + a serves-it test (the #48 arctic gap).
  - `PR-claude-assert-element-form-not-substring-001` — render-marker tests assert
    specific element/url forms; `STUDIO_CSS` is embedded so the page contains theme
    tokens regardless.
  - Architecture: studio = `format!` HTML + embedded `STUDIO_CSS` (`render.rs`);
    the arctic asset pattern (`editor.rs` `ARCTIC_PNG`/`arctic_sheet`/route/test)
    is the template to mirror.
- **Approach sketch (Phase 2 firms up):** crop 1 frame + 1 button → `public/ui/`;
  embed + serve (`/ui/*.png`) with serves-it tests; `border-image` nine-slice on
  `.panel`/`.studio-header` + a themed button in `studio.css`. Exact crop rects +
  which frame/button + slice insets = design decisions.
- **Scope OUT:** game client re-skin (#50b); Portraits/Icons/Emotes/Banners/Bars
  + per-component theming (#50c); exact visual tuning (owner-adjustable post-view).
- **EARS reviewed:** REQ-001..005 — assets committed+served, panels/header framed,
  buttons themed, pages reference the asset, gate green.
- **Known limitation:** the loopback studio can't be self-screenshot (the extension
  Chrome can't reach it), so visual quality is **owner-judged** after viewing.
- **Ticket:** #50 `7c6d2165-76eb-4954-b99f-e2e4ba89d3b5` (feature, exists — not
  re-minted).
- **AAR id:** e9002fde-5bbf-4a5f-aa0a-41e573659a71

## Phase 2 — Design

### Art picked (measured from the sheets — deterministic crops)
- **Panel frame** → `public/ui/panel-frame.png`:
  `magick raw_assets/mini-medieval/user-interface/Frames.png -crop 30x32+5+5 +repage public/ui/panel-frame.png`
  — a wooden gold-outer / brown-inner frame, darker shadowed bottom (lit-from-top
  3D), transparent center. **border-image-slice ≈ 7** (the corner ornament).
- **Gold button** → `public/ui/button.png`:
  `magick raw_assets/mini-medieval/user-interface/Inputs.png -crop 26x12+7+7 +repage public/ui/button.png`
  — gold fill, brown edges, dark shadowed bottom; matches the brand brass
  (`--brass #e5c56f`). **border-image-slice ≈ 4** (edges); fill colour ~`#c8843a`.
- Both verified clean by scaling 800–1600% and viewing. `raw_assets/` stays
  gitignored; only the two crops are committed.

### Approach / architecture (mirror the #48 arctic asset pattern)
- **`crates/oathstar-studio/src/ui.rs` (NEW):**
  `const PANEL_FRAME_PNG: &[u8] = include_bytes!("../../../public/ui/panel-frame.png");`
  + `const BUTTON_PNG`; `pub async fn panel_frame() -> Response` / `pub async fn
  button() -> Response`, each `([(header::CONTENT_TYPE, "image/png")],
  Bytes::from_static(...)).into_response()` (exactly like `editor::arctic_sheet`);
  serves-it tests (200 + `image/png` + non-empty) per asset.
- **`main.rs`:** `mod ui;` + `.route("/ui/panel-frame.png", get(ui::panel_frame))`
  + `.route("/ui/button.png", get(ui::button))`.
- **`static/studio.css` theme layer** (over the existing `:root` tokens; keep the
  dark base):
  - `.panel, .studio-header { border: 14px solid transparent; border-image:
    url("/ui/panel-frame.png") 7 repeat; border-radius: 0; image-rendering:
    pixelated; }` — the wooden frame around every box; the panel keeps its dark
    `background`, shown inside the frame.
  - `button { border: 5px solid transparent; border-image: url("/ui/button.png") 4
    repeat; background-color: #c8843a; color: #2a1a0a; image-rendering: pixelated;
    font-weight: 700; }` — the beveled gold button (centre from the bg-colour, edges
    from the sprite). The login `.card` button + Sign out + Validate all inherit.
  - A `--frame` accent token; otherwise additive.
- **State/view:** assets are static bytes (no logic); the gate's Rust mutation
  surface is the two `ui.rs` handlers (pinned by the serves-it tests, like
  `arctic_sheet`). The **CSS is the genuinely-uncoverable surface** (visual only).
- **Layout safety:** border-image replaces the 1px panel borders; padding stays
  inside the 14px frame. The #49 nav + #48 editor split (`.editor-main` grid,
  `.map-scroll`, `.palette-scroll`) sit inside framed `.panel`s — unaffected by the
  border swap (decide at implement if any panel needs padding nudge).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `public/ui/panel-frame.png` (NEW) | the cropped wooden frame (30×32). |
| 2 | `public/ui/button.png` (NEW) | the cropped gold button (26×12). |
| 3 | `crates/oathstar-studio/src/ui.rs` (NEW) | `PANEL_FRAME_PNG`/`BUTTON_PNG` consts + `panel_frame`/`button` handlers + serves-it tests. |
| 4 | `crates/oathstar-studio/src/main.rs` | `mod ui;` + the 2 `/ui/*.png` routes. |
| 5 | `crates/oathstar-studio/static/studio.css` | the theme layer (panel/header frame + button). |
| 6 | `crates/oathstar-studio/src/render.rs` | a test asserting the embedded `STUDIO_CSS` references the two `/ui/...` assets (the page-wires-the-theme marker). |

### Regression Test Plan
| # | Test | Proves Req |
|---|---|---|
| RT-1 | `ui::panel_frame()` → `200` + `Content-Type: image/png` + non-empty body | REQ-001 |
| RT-2 | `ui::button()` → `200` + `image/png` + non-empty body | REQ-001 |
| RT-3 | `render`: `STUDIO_CSS` contains the panel rule `border-image: url("/ui/panel-frame.png")` (specific form, not a bare substring — PR-claude-assert-element-form-not-substring-001) | REQ-002 |
| RT-4 | `render`: `STUDIO_CSS` contains the button rule `border-image: url("/ui/button.png")` | REQ-003 |
| RT-5 | `render`: an authenticated page (`dashboard_page`) embeds the CSS referencing both `/ui/panel-frame.png` and `/ui/button.png` | REQ-004 |
| RT-6 | `bin/gate.sh` FULL green — mutation 100% MSI (the `ui.rs` handlers pinned by RT-1/2; main routes excluded) | REQ-005 |

**Genuinely uncoverable:** the rendered *appearance* of the border-image theme (does
it look good) — no automated test; **owner-judged after viewing** (the loopback
studio can't be self-screenshot).

### Risks / decisions
- **border-image vs stretched background** — chose nine-slice (`border-image`) so the
  frame corners/edges stay crisp at any panel size; pixel-art kept sharp via
  `image-rendering: pixelated`. (reversible — the button could fall back to a
  stretched `background` if the slice reads off.)
- **Exact slice values (7 / 4) are first estimates** from the measured art; implement
  verifies the crops are clean and may nudge ±2px. Owner tunes the final look.
- **Frame on `.studio-header` too** (per the spec) — a framed top banner; if it reads
  awkward against the nav, easy to drop to a themed bottom-border in a follow-up.
- **Two new committed binary assets** (~tiny PNGs) under `public/ui/` — selective
  staging at commit; `raw_assets/` stays out.

## Phase 3 — Implement
- **Built:**
  - `public/ui/panel-frame.png` (30×32, 395 B) + `public/ui/button.png` (26×12,
    370 B) — cropped from the kit sheets with the exact `magick` commands from the
    design; both valid PNGs.
  - `crates/oathstar-studio/src/ui.rs` (NEW): `PANEL_FRAME_PNG`/`BUTTON_PNG`
    `include_bytes!` consts + `panel_frame()`/`button()` handlers (`image/png`,
    `Bytes::from_static`) mirroring `editor::arctic_sheet`; `serves_the_panel_frame`
    + `serves_the_button` tests (200 + `image/png` + non-empty).
  - `main.rs`: `mod ui;` + `/ui/panel-frame.png` + `/ui/button.png` routes.
  - `static/studio.css`: the theme layer — `.panel, .studio-header { border: 14px
    solid transparent; border-image: url("/ui/panel-frame.png") 7 repeat; … }` +
    a themed gold `button` (`border-image: url("/ui/button.png") 4 repeat;
    background-color: #c8843a; …`), `image-rendering: pixelated`. Appended after the
    #49 nav block; the dark `:root` base + layouts kept.
  - `render.rs`: `pages_reference_the_ui_kit_assets` — the dashboard page (embedding
    `STUDIO_CSS`) contains `url("/ui/panel-frame.png")` + `url("/ui/button.png")`.
- **Deviations from design:** none material. The button's centre fill comes from
  `background-color: #c8843a` (the gold), the bevel edges from the `border-image`
  (design-intended).
- **Checks:** `cargo clippy -p oathstar-studio --tests` strict-clean;
  `cargo test -p oathstar-studio` **37 passed** (+3 over #49's 34); `cargo fmt`
  applied. Visual appearance is **owner-judged** (can't self-screenshot the loopback
  studio).

## Inspect (Phase 3.5)
- **Lenses (2 critics):** A = correctness/reuse + CSS-layout safety + asset
  integrity/serve-every-asset; B = mutation-readiness (empirical) + scope.
- **Findings:**
  | # | Severity | Finding | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | **med** | **`.studio-header` frame was asymmetric** — the pre-existing `.dashboard header` + `.editor header { border-bottom: 1px solid var(--line) }` (specificity 0,0,1,1) out-specify the new `.panel, .studio-header { border: 14px … }` (0,0,1,0), so the header's **bottom** edge stayed 1px while the other three sides got the 14px frame — the wooden frame didn't close. | **REAL** | **FIXED** — removed the two now-redundant `header` element-rules (fully superseded by the `.studio-header` class, which the element always carries) + the dead `.crumbs a`; cleaned `.studio-header`'s now-overridden `border-bottom` and gave it `padding: 0.35rem 0.7rem` so content doesn't touch the frame. 37 tests still green; frame closes uniformly. Captured BF + PR. |
  | — | info | `.panel`'s 14px border narrows content, but `* { box-sizing: border-box }` absorbs it inside each `minmax(0,1fr)` grid track — no #48 editor-split overflow; the 480px palette stays behind its existing `overflow:auto`. Cosmetic only. | CLEAN | none |
  | — | info | `button` cascade correct — the appended rule wins for the props it sets (`border`, `border-image`, colors) and **keeps** the earlier `padding`/`cursor`; `background-color` clears any stale `background` image. Button border is symmetric (no header-style collision). | CLEAN | none |
  | — | **empirical** | **Mutation:** Critic B ran `cargo mutants -p oathstar-studio --file ui.rs` → **2 mutants, 2 caught, 0 missed**. `panel_frame`/`button` are byte-for-byte mirrors of the proven `arctic_sheet`; the serves-it tests kill the `Default::default()` mutant on two axes (content-type + non-empty). The render `pages_reference_the_ui_kit_assets` pins the `url("/ui/…")` element-forms. CSS is the only uncovered surface (expected, visual). | CLEAN | none |
  | — | info | **Serve-every-asset** (`PR-claude-serve-every-asset-the-glue-fetches-001`): the CSS references exactly the two `/ui/*.png` urls; **both** have a `main.rs` route + a serves-it test. No referenced-but-unserved asset. | CLEAN | none |
  | — | info | **Scope:** only #50 files (+ pipeline docs); the online-first WIP is untouched; the game client + rest-of-kit are correctly deferred. | CLEAN | none |
  | 2 | low | Three near-identical PNG handlers now (`arctic_sheet`/`panel_frame`/`button`); a shared `fn png(bytes)` helper would DRY ~10 lines but spans two modules. | **DEFER** | Not worth it at 3 trivial copies; extract when a 4th embedded-PNG handler lands. |
- **Captured:** `failure-record` **BF-claude-css-specificity-border-override-001**
  + `prevention-rule-record` **PR-claude-css-class-rule-vs-specific-element-rule-001**
  (the cascade gotcha the critic caught).

## Phase 4 — Validate
- **Tests (written Phase 3, confirmed covering RT-1..6):** `ui::serves_the_panel_frame`
  + `ui::serves_the_button` (200 + image/png + non-empty — RT-1/2),
  `render::pages_reference_the_ui_kit_assets` (the dashboard embeds
  `url("/ui/panel-frame.png")` + `url("/ui/button.png")` — RT-3/4/5).
- **`cargo test --workspace`:** all crates green, **0 failed** (oathstar-studio 37).
- **`node --test tests/*.test.js`:** **77 pass, 0 fail** (JS untouched).
- **`bin/gate.sh` (FULL):** `GATE GREEN [full]` — **17/17**.
  - gate:15 rust coverage **98.72%** line (floor 94); gate:16 js **89.22%**
    (floor 75); gate:17 mutation **509 caught / 0 missed → MSI 100.0%** (+2 over
    #49's 507 — the two `ui.rs` handlers, killed by the serves-it tests).
- **Genuinely uncoverable:** the CSS visual appearance (the `border-image` frame +
  button) — owner-judged after viewing. The two PNG crops are binary assets.
- **Pre-existing exclusions:** none. Online-first WIP + `raw_assets/` stay out —
  selective staging at `/commit`.

## Phase 5 — Complete
- **Docs updated:** `docs/map-system.md` — added a "Themed (ticket #50, slice 1)"
  note to the studio section (the kit frame + button, served `/ui/*.png`; game
  client + rest-of-kit deferred). No `decisions.md` change.
- **Forge capture:**
  - `failure-record` **BF-claude-css-specificity-border-override-001** +
    `prevention-rule-record` **PR-claude-css-class-rule-vs-specific-element-rule-001**
    (recorded at inspect — the asymmetric-frame cascade bug).
  - `aar-submit` e9002fde — outcome completed, effectiveness 4; materialized the
    BF + PR (2 novel findings). Win: the inspect CSS-layout critic caught the
    specificity bug no test could (CSS is uncoverable); the mutation critic
    empirically pre-ran cargo-mutants (0 missed); the #48 arctic embed pattern made
    the asset serving trivial + mutation-tight.
- **Ticket NOT closed (multi-slice):** `ticket-comment` on #50 — slice 1 shipped;
  **#50 stays open** for slice 2 (#50b game client) + slice 3 (#50c rest-of-kit).
- **Archived:** spec+notes → `docs/planning/pipeline/completed/`. `TICKET-50` stays
  in `tickets/open/`.
