# WORK-fix-play-surface-control-placement-and-viewport — Notes

Per-phase working notes for the paired `.spec.md`.

## Phase 1 — Plan
- **Request:** Ticket #14 — fix the #13 play-surface UX: Exit Pad out of room desc
  → near command/feed; drop movement from Intent; viewport-contained shell with a
  bounded internal-scroll feed; no button-click viewport jumps; mobile no overflow.
  Verify: `npm test`, `npm run build`, `./bin/gate.sh --fast` + browser smoke.
- **Intake source:** none (minted directly; follow-up to #13).
- **Classification / tier:** Work pipeline, **client-only** (no Rust/protocol/parser
  change). Mostly CSS + an `index.html` DOM move + a small `intent.js` change.
- **Forge recall (AAR 1e527e55):**
  - `AD-claude-ui-shell-001` — framework-free pure-core/glue client; #14 keeps the
    split (only `intent.js` pure-change is unit-tested; DOM/CSS = documented smoke).
  - No prior viewport/layout failures (first layout-focused ticket). Relevant
    `ui-design.md` guardrails: bounded feed, contained shell, no SPA framework.
  - The updated `docs/ui-design.md` (uncommitted working-tree edits) is the design
    spec: "Exit controls … stay near the command hand, not in the room description
    area"; "Movement commands should not also appear as Intent suggestions once the
    Exit Pad exists"; "bounded event surface with an internal scrollbar … should not
    cause viewport jumps or unexpected page scrolling."
- **Ticket:** forge #14 `a4ef40f6-cb5d-4a01-a1e6-2bf2cf488533` (already minted).
- **EARS reviewed:** REQ-001..009 carried verbatim. REQ-003 via `intent` unit test;
  REQ-001/002/004/005/006/007/008 via browser smoke (+ test where practical);
  REQ-009 via the three verification commands.

### Grounding (committed #12+#13 base)
- `index.html`: `.room-brief` currently holds `#room-description` + `#exit-line` +
  `#view-room-button` + `#exit-pad` (#exit-pad must move out). `.feed-panel` holds
  `#log` + the `#command-form` (the Exit Pad's new neighborhood).
- `src/client-app.js`: `renderExitPad` builds buttons into `el.exitPad`; only the
  mount point changes. `renderIntent` uses `suggestCommands`.
- `src/client/intent.js`: `suggestCommands` = contextual (swear/confront) +
  `movementCommands(snapshot)` + `COMMAND_VOCAB`. Remove `movementCommands` +
  `DIRECTION_HINTS` → movement no longer suggested.
- `styles.css`: `.game-shell` is the layout grid; `.log` is the feed; responsive
  media queries near the file tail. Viewport/overflow rules are the main work.
- Movement parse is server-side (`oathstar-core` command parser: bare `north`/`up`
  etc.) — typed movement is unaffected by removing Intent suggestions.

## Phase 2 — Design

### Approach / architecture (client-only; CSS + index.html DOM move + small intent.js change)
**1. Exit Pad relocation (REQ-001/002).** Move `<nav id="exit-pad">` out of
`.room-brief` into `.feed-panel`, wrapped with `#command-form` in a new
`.command-area` (the feed-panel's bottom `auto` row) — the Exit Pad sits beside
the command input. `.room-brief` becomes single-column (`.room-text` only:
`#room-description`, `#exit-line`, `#view-room-button`). `client-app.js` needs **no
change**: `renderExitPad` targets `#exit-pad` by id + `replaceChildren` (DOM
location-agnostic); `renderRoom` still calls it; pad buttons are `type="button"`
and `runCommand` re-focuses the input in `finally` (REQ-007).

**2. Movement leaves Intent (REQ-003/004).** `src/client/intent.js`: remove
`movementCommands` + `DIRECTION_HINTS`; `contextualCommands` returns only the
swear/confront contextual entries. `COMMAND_VOCAB` already has no directions, so
`suggestCommands` then never emits `north/south/east/west/up/down`. Typed movement
is untouched (the input POSTs raw text the server parses — no parser/server change).

**3. Viewport-contained shell (REQ-005/006/008) — `styles.css`.**
- `body`: `height: 100vh; height: 100dvh; overflow: hidden` (no page scroll; keep
  the gradient bg).
- `.game-shell`: `height: 100vh; height: 100dvh`, full width (drop the
  `min(100rem,…)` cap + `margin:auto` gutters → `width:100%` + padding),
  `overflow:hidden`; grid rows shrinkable — `stage minmax(0,1.15fr)` /
  `feed minmax(0,0.85fr)` (drop the 25rem/18rem mins that forced page growth).
- Feed already scrolls internally (`#log` `min-height:0; overflow-y:auto`; newest
  appended at bottom + `scrollTop=scrollHeight`); the shell-height fix makes the
  feed row bounded so the internal scroll engages (REQ-006).
- `@media (max-width:980px)`: reset `.game-shell { height:auto; overflow:visible }`
  so the single-column stack scrolls normally on mobile (REQ-005 is desktop-scoped;
  REQ-008 only forbids horizontal overflow). `.command-area` stacks; `min-width:0`
  keeps no horizontal overflow. The existing `.room-brief{grid-template-columns:1fr}`
  mobile rule already covers the now-single-column room-brief.

**4. No viewport jumps (REQ-007).** All controls `type="button"` (exit-pad/intent/
view-room); only the command form submits (preventDefault'd); modal closes via
`method="dialog"`. With the shell at fixed height + `overflow:hidden` there is no
page to scroll/jump; focus returns to the command input after actions.

### File manifest
  | # | File | Change |
  |---|---|---|
  | 1 | `index.html` | Move `#exit-pad` from `.room-brief` into a new `.command-area` in `.feed-panel` (with `#command-form`); `.room-brief` keeps room-text only |
  | 2 | `src/client/intent.js` | Remove `movementCommands` + `DIRECTION_HINTS`; `contextualCommands` drops the movement concat |
  | 3 | `styles.css` | Full-viewport shell (`body`/`.game-shell` `100dvh` + `overflow:hidden` + full width, shrinkable rows); `.command-area` + compact `.exit-pad`; `.room-brief` single-column; mobile `.game-shell{height:auto;overflow:visible}` + command-area stack |
  | 4 | `tests/client.test.js` | Update T9 → `suggestCommands` excludes movement, keeps non-movement + contextual |
  | — | `src/client-app.js` | No change expected (verify in implement; `renderExitPad` is DOM-location-agnostic) |

### Regression Test Plan
  | # | Test | Proves |
  |---|---|---|
  | T9′ | `intent.suggestCommands` (room exits {north,up}) → NO movement command (none of north/south/east/west/up/down); still has contextual (swear/confront) + non-movement vocab (look/map/…); query filter still works | REQ-003 |
  | S1 | Smoke (desktop): Exit Pad NOT in room desc, sits near command/feed; Intent has no movement; feed scrolls internally (page does not), newest at bottom; shell fills width (no gutters) + height (no page scroll); clicking Exit Pad/Intent/modal buttons doesn't jump; focus stays on input | REQ-001/002/005/006/007 |
  | S2 | Smoke (mobile ~390px): no horizontal overflow; map/feed/command/Exit Pad/Intent/menu usable | REQ-008 |
  | S3 | Smoke: typed `north` / `east` still moves (server accepts) | REQ-004 |
  | R1 | Structural: built `dist/index.html` has `#exit-pad` inside `.feed-panel`/`.command-area`, not `.room-brief` | REQ-001/002 |
  | G1 | `npm test` + `npm run build` + `./bin/gate.sh --fast` green | REQ-009 |

Uncoverable-by-unit-test (documented smoke): layout/CSS + DOM placement (viewport
fill, bounded feed, no-jump, mobile) are visual/browser behaviors → S1/S2/S3 + the
structural R1 (the established glue/CSS = documented-smoke pattern). Only `intent.js`
is unit-testable (T9′).

### Risks / decisions
- **`100dvh` support:** modern browsers + the Tauri webview support `dvh`; declare
  `height:100vh; height:100dvh` so the `vh` fallback applies if needed.
- **Shrinkable rows (`minmax(0,…)`):** very short viewports shrink the map/feed
  rather than scrolling the page (panels own overflow; feed scrolls) — acceptable.
- **Full-width on ultra-wide:** dropping the cap satisfies REQ-005 "no gutters"; a
  generous cap is a one-line follow-up if panels look over-stretched.
- **Mobile keeps vertical scroll:** REQ-005 (no page scroll) is desktop-scoped; ≤980px
  is auto-height + scrolls; REQ-008 only forbids horizontal overflow.
- **`client-app.js` unchanged:** confirm in implement; any focus/mount tweak is tiny glue.
- **JS coverage:** removing `movementCommands` drops covered lines but also the
  assertions referencing them; `intent.js` stays covered (T9′); floor 75% holds. No
  Rust change → Rust-cov/mutation untouched.
- **Forge (AAR 1e527e55):** AD-claude-ui-shell-001 governs the pure/glue split; no
  prior layout traps surfaced.

## Phase 3 — Implement
- **Built (4 files):**
  - `src/client/intent.js` — removed `DIRECTION_HINTS` + `movementCommands`;
    `contextualCommands` returns only swear/confront → `suggestCommands` never emits
    movement (REQ-003).
  - `index.html` — moved `<nav id="exit-pad">` from `.room-brief` into a new
    `.command-area` inside `.feed-panel`, beside `#command-form` (after the input);
    `.room-brief` now holds only `.room-text` (REQ-001/002).
  - `styles.css` — `body`/`.game-shell` → `100vh`/`100dvh` + `overflow:hidden` +
    full width (dropped the `min(100rem,…)` cap + `margin:auto` gutters) +
    shrinkable rows `minmax(0,1.15fr)`/`minmax(0,0.85fr)` (REQ-005); `.map-frame`
    `min-height:0` + `overflow:auto` (map flexes/scrolls within the bounded stage);
    `.room-brief` single-column; `.exit-pad` compacted for the command area;
    `.command-area` (grid form|pad) carries the command chrome; mobile `@980`
    `.game-shell{height:auto;overflow:visible}` + `.command-area` stacks (REQ-008).
  - `tests/client.test.js` — T9 flipped: `suggestCommands` excludes movement
    (incl. when searched), keeps contextual + non-movement vocab.
- **`src/client-app.js`: no change** (confirmed) — `renderExitPad` targets
  `#exit-pad` by id + `replaceChildren`, so the DOM move needs no glue change; pad
  buttons stay `type="button"` and `runCommand` re-focuses the input (REQ-007).
- **Compile/check:** `node --check` ok; `node --test tests/*.test.js` 13/13;
  `npm run build` ✓; structural grep confirms `#exit-pad` in `.command-area`
  (feed panel), not `.room-brief`.
- **Deviations from design (+ reason):**
  - `.map-frame` base `min-height` 21rem→0 + `overflow` hidden→auto (design said
    only "shrinkable rows"): the beginner z-plane can be ~4 rows (≈21rem), which
    would overflow/clip a bounded stage row — making the map flex/scroll within its
    frame keeps the shell contained without page scroll. Mobile keeps its fixed map
    heights (24/20rem) for the auto-height single-column layout.

## Inspect (Phase 3.5)
- **Lenses run:** 2 parallel critics — correctness + CSS-layout (vs REQ-001..008);
  gate-risk + coverage + build + simplification.
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | F1 | MED | The relocated ~8.4rem cross Exit Pad in `.command-area` steals the feed row's height — `#log` falls to ~3 entries at an 800px (Tauri default) window (`styles.css` command-area/exit-pad/game-shell) | REAL — ergonomics (still scrolls; REQ-005 page-scroll safe) | rebalanced rows 1.15/0.85→**1.05/0.95**; compacted pad (1.5→1.4rem rows, 1.9→1.85rem cols) + command-area padding (0.7→0.45rem) → `#log` recovers ~3rem (~5-6 entries) |
  | F2 | LOW | Orphaned `.exit-grid` CSS rules | REJECTED — pre-existing in the HEAD base (not a #14 change); the selectors are shared with the still-live `.quick-grid`, so not wholesale-removable; out of #14 scope |
  | F3 | LOW | `.room-brief` in the `@640` rule is now a no-op (base is single-column) | REJECTED — harmless; the rule still correctly targets `.state-rail` |
  | F4 | LOW | base `.command-form` padding/border shadowed by `.command-area .command-form` | REJECTED — intentional chrome-lift; correct specificity; base stays the fallback for any non-`.command-area` form |
  | INFO | two-value `height:100vh; height:100dvh` | KEEP — intentional `dvh` progressive-enhancement fallback |
- **All ACs verified PASS by the correctness critic** (REQ-001..008): `#exit-pad`
  out of `.room-brief` and inside `.feed-panel`/`.command-area` (sibling of the
  form, not nested → no submit); Intent excludes all six directions in every path;
  typed movement intact; no page scroll (`body`+shell `overflow:hidden`, internal
  scroll regions); newest-at-bottom preserved; all buttons `type=button`; mobile no
  horizontal overflow.
- **Gate-risk: clean.** `intent.js` 100% line/func coverage; all-files 83.15% ≥ 75;
  no dead `movementCommands`/`DIRECTION_HINTS` refs; determinism clean; build 0 warnings.
- **Post-fix verification:** `node --test` 13/13; `npm run build` ✓;
  `./bin/gate.sh --fast` GREEN (14/14).
- **Forge capture:** failure `BF-exitpad-feed-crush-001`; prevention rule
  `PR-claude-relocate-tall-control-budget-001`.

## Phase 4 — Validate
- **Tests added (run here):** `tests/client.test.js` T9 rewritten —
  `suggestCommands` excludes all six movement commands (incl. when searched),
  keeps contextual (swear/confront) + non-movement vocab. No Rust tests
  (client-only).
- **`cargo test --workspace`:** PASS — 116/116 (12+68+16+20), 0 failed; Rust
  unaffected (no Rust/protocol change).
- **`node --test tests/*.test.js`:** PASS — 13/13, 0 failed.
- **`npm run build` (vite):** PASS — built clean. Structural smoke: `dist/index.html`
  order is `room-brief` → `command-area` (`#command-form` then `#exit-pad`) — the
  Exit Pad is in the command bar, not the room area. (The lone `"Travel "` in the
  bundle is the Exit Pad button `title`, i.e. movement is on the pad, not Intent.)
- **`./bin/gate.sh --fast`:** `GATE GREEN [fast]` — 14 passed, 0 failed (REQ-009).
- **Live smoke (server on 127.0.0.1:7881):** start `hollowmere_square`; typed
  `POST /command {"input":"north"}` → `accepted:true`, `currentRoomId` → `north_gate`
  (REQ-004 — typed movement intact).

### AC → verification traceability
| REQ | Verified by | Result |
|---|---|---|
| REQ-001 | structural (`dist` `#exit-pad` not in `.room-brief`) + browser smoke | PASS |
| REQ-002 | structural (`#exit-pad` in `.command-area` beside the input) + browser smoke | PASS |
| REQ-003 | T9 (`suggestCommands` excludes movement in every path) + bundle grep | PASS |
| REQ-004 | live `POST /command {"input":"north"}` → moved rooms (accepted) | PASS |
| REQ-005 | CSS review (`body`/shell `100dvh` + `overflow:hidden`, full width, shrinkable rows) + browser smoke | PASS |
| REQ-006 | CSS review (`#log` `min-height:0; overflow-y:auto`; newest-at-bottom via `scrollTop=scrollHeight`) + smoke | PASS |
| REQ-007 | review (all controls `type=button`; pad is a `<form>` sibling; `runCommand` re-focuses input) + smoke | PASS |
| REQ-008 | review (`@980` single-column `height:auto`; `min-width:0`; compact pad fits ~360px) + browser smoke | PASS |
| REQ-009 | `npm test` 13/13 + `npm run build` ✓ + `./bin/gate.sh --fast` GREEN | PASS |

### Browser smoke notes (manual; CSS/glue is documented-smoke per AD-claude-ui-shell-001)
Run `npm run server:dev` + `npm run dev`:
1. **Exit Pad placement** — the directional pad is in the **command bar beside the
   input**, NOT in the room description area (REQ-001/002).
2. **Intent** — shows only non-movement helpers (look/swear/help/map/…); no
   north/south/east/west/up/down (REQ-003).
3. **Typed movement** — typing `north`/`east` in the input still moves (REQ-004,
   confirmed via API above).
4. **Bounded feed** — repeated events scroll **inside** `#log` (newest at bottom);
   the **page does not scroll** (REQ-005/006). At an 800px (Tauri default) window
   the feed shows ~5-6 entries after the inspect fix.
5. **Viewport** — desktop shell fills width (no side gutters) + height (no page
   scroll) (REQ-005).
6. **No jumps** — clicking Exit Pad / Intent / modal buttons doesn't jump the
   viewport; focus returns to the command input (REQ-007).
7. **Mobile (~390px)** — single column, no horizontal overflow; map/feed/command/
   Exit Pad/Intent/menu all usable (REQ-008).

- **Pre-existing exclusions:** none (no pre-existing failures). The orphaned
  `.exit-grid` CSS is pre-existing dead code (noted at inspect, out of #14 scope).
- **FULL gate (15–17):** deferred to `/commit`. Spot-check: JS coverage 83.15%
  (≥75; `intent.js` 100%); no Rust change → Rust-cov/mutation untouched.

## Phase 5 — Complete
- **Docs updated:** `docs/ui-design.md` Implementation Status — the #14 bullet
  updated from intent ("should…") to implemented (Exit Pad in the command bar,
  movement out of Intent, viewport-contained shell + bounded feed). No new
  `decisions.md` entry: a refinement within Decision 032 / AD-claude-ui-shell-001.
- **Forge capture:** `aar-submit` (aar 1e527e55, outcome completed, effectiveness 4,
  12 verdicts, 2 novel → distillation/confidence-drift/pattern-emergence enqueued);
  failure `BF-exitpad-feed-crush-001` + prevention rule
  `PR-claude-relocate-tall-control-budget-001` (recorded at inspect).
- **Ticket:** forge #14 → `in-review` (NOT done) + summary comment. FULL gate +
  `/commit` deferred per the requester's scope ("do not commit unless asked").
- **Archived:** pipeline pair moved to `docs/planning/pipeline/completed/`; local
  ticket doc kept in `tickets/open/` at status `in-review` (commit pending).

## Manager Review Addendum
- **Finding:** mobile/browser smoke exposed a real jump risk after the pipeline:
  the HTML `autofocus` attribute could load the mobile layout scrolled down toward
  the command input, even though desktop page scroll was correctly contained.
- **Fix:** removed `autofocus` from `index.html` and added
  `focusCommandInput()` in `src/client-app.js`, using
  `el.input.focus({ preventScroll: true })` with a fallback to plain focus.
  `boot`, New Session, and command completion now preserve command focus without
  turning focus into a viewport scroll.
- **Verification after addendum:** `npm test` 13/13, `npm run build` pass,
  mobile smoke starts at `scrollY: 0`, desktop command/Exit Pad clicks keep
  `scrollY: 0`, typed movement through the Enter button still moves rooms, and
  full `./bin/gate.sh` is green (17/17; Rust coverage 97.51%; JS coverage 83.15%;
  mutation MSI 100%).
