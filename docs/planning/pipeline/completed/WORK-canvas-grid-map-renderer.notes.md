# WORK-canvas-grid-map-renderer — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #16 — first-party `<canvas>` map renderer v1 replacing the
  DOM `#map` grid; rest of UI stays Datastar/browser-first; server map stays JSON
  from `/state` (no canvas commands in Rust). Default 32×32 configurable; current
  z-plane only; unknown/discovered/current cells + glyph/title + passable/blocked;
  Hi-DPI crisp; no Phaser/Pixi; pure JS tests + browser smoke.
- **Intake source:** none (ticket minted directly).
- **Classification / tier:** single **work pipeline**, one shippable slice.
  Systems: ui (player-client map surface) + frontend build. No Rust/server change.
- **AAR:** `625818a1-34f2-4263-9c2d-492cb4184209` (opened Phase 1; capture at inspect/complete).
- **Forge recall (from /work):**
  - **Decision 025 (Locked):** square grid, cardinal + up/down, passable/non-passable,
    server exposes map as JSON, frontend chooses text/ASCII/canvas/sprites; "consider
    canvas early." **Decision 035 (Locked):** first canvas renderer = first-party
    32×32 square grid consuming server JSON, no engine dep, map stays JSON.
  - **`docs/map-system.md`** pre-endorses canvas (Canvas Consideration: stable grid,
    less DOM churn, sprite path; Backend Payload JSON shape; Passability `#`/`.`/`@`).
    **`docs/ui-design.md` Map Direction**: renderer-agnostic data, tile-size hook.
  - Surfaced node ids (bodies via `knowledge-context` at Plan/Implement):
    failures `deaa10b6`, `fa11e61e`, `d240d92c`, `360ee9a3`; prevention rules
    `be0f3145`, `31d4b76d`, `1f466495` (my escaping rule). Themes: recent UI/JS
    pitfalls + the renderer-agnostic-JSON guardrail.
- **Code landscape (for Design):**
  - `src/client/map.js` — `toMapModel(mapSnapshot, config)` returns
    `{ mode, tilePixels, columns, rows, minX, minY, z, planes, cells, region, currentRoomId }`;
    each cell `{ x, y, present, id, title, glyph, discovered, current }`. Already
    filters to the **current z-plane** and computes the bounding-box grid. `DEFAULT_MAP_CONFIG = { tilePixels: 32, mode: "glyph" }`. **`passable` is NOT in the
    cell model** (it exists on `MapRoomSnapshot` in `oathstar-protocol` but
    `toMapModel` drops it) — REQ-004 blocked styling needs a small model extension.
  - `client-app.js` — `renderMap(snapshot)` (≈L345) builds the DOM grid from
    `toMapModel` into `#map` (CSS `grid-template-columns`, per-cell `<div>` with
    `aria-label`); `mapRenderConfig = { ...DEFAULT_MAP_CONFIG }` (≈L36); `el.map`,
    `el.mapLabel`. This is what #16 replaces with canvas drawing.
  - `index.html` — `<div id="map" class="map-grid"></div>` (≈L72) → becomes
    `<canvas id="map">`. Other surfaces (`#map-label`, room panel, exit pad,
    intent, `#log` Datastar feed) are untouched.
  - Build: vite (`npm run dev`/`build`); tests `node --test tests/*.test.js`
    (JS coverage floor 75%, so the canvas math must live in a tested pure module).
- **Branch note:** currently on `ticket-15-datastar-ui-transport` (#15 not yet
  merged to main); #16 modifies the same UI files (`index.html`, `client-app.js`,
  `map.js`) so it correctly builds on #15's tree. Finalize branch strategy at /commit.
- **Ticket:** forge #16 `f69a718c-7ae8-418c-895e-9e1baae72f98`; local doc
  `docs/planning/tickets/open/TICKET-16-canvas-grid-map-renderer.md`.
- **EARS reviewed:** REQ-001..010 in the spec, each one observable behavior + a
  verification method.

## Phase 2 — Design

### Decisions (with justification)
- **Extend `toMapModel`'s cell with `passable` (REQ-004).** `MapRoomSnapshot` already
  carries `passable: bool`; `toMapModel` does the per-cell room lookup but currently
  drops it. Add `passable: room?.passable` to the derived cell so `false` remains a
  known blocked room while missing/legacy payloads stay unknown instead of being
  coerced into blocked. The data already flows in the snapshot and the model already
  looks the room up; the field is additive and the server snapshot is still not mutated
  (the existing immutability test holds).
- **Pure/glue split (mirror #15).** All geometry, Hi-DPI sizing, cell classification,
  draw-plan, and the a11y summary live in a NEW DOM-free module `src/client/canvas-map.js`
  (tested by `node --test`, counts toward the 75% floor). The only canvas-touching code
  is a thin `drawMapCanvas` seam in `client-app.js` (issues `ctx` calls; smoke-verified,
  uncounted glue). node/jsdom has no canvas-2D context, so logic MUST stay in the pure module.
- **Replace, don't dual-render.** `<div id="map" class="map-grid">` → `<canvas id="map">`;
  the `.map-cell/.map-name/.map-zone` CSS is superseded (removed) and replaced by a
  `.map-canvas` rule; `.map-frame`, `#map-label`, legend, and all other panels stay.
- **Theme-matched palette mirrored from `styles.css`** as a constant in `canvas-map.js`
  (empty `#10151a`/`#28303a`; discovered `#1b2425`/`#344149`/text `#ddd4c4`; current
  `#e5c56f`/`#f1d98f`/text `#101318`; blocked `#181c20`/`#3a2e2e`/text `#8a8076`). Keeps
  the seam thin (no `getComputedStyle`); documented to track the CSS.
- **No Rust/server change (REQ-008).** `GET /state` keeps returning `Json<GameSnapshot>`;
  the canvas consumes `snapshot.map` via `toMapModel`. No engine-graphics dependency.

### Architecture / approach
1. **`src/client/canvas-map.js`** (new, pure, tested):
   - `MAP_PALETTE` — frozen kind→{fill,stroke,text} mirroring `styles.css`.
   - `canvasSize(model, dpr=1) -> {cssWidth, cssHeight, backingWidth, backingHeight, dpr}`
     — `cssW=columns*tilePixels`, `cssH=rows*tilePixels`, `backing=round(css*dpr)`; clamps
     a non-finite/≤0 dpr to 1.
   - `cellKind(cell) -> "empty"|"discovered"|"current"|"blocked"` — `!present→empty`;
     `current→current`; `discovered && passable===false → blocked`; `discovered→discovered`;
     else `empty` (present-but-undiscovered stays fog, matching the DOM).
   - `toDrawPlan(model) -> {width, height, tile, ops[]}` — one op per cell:
     `{x:(cell.x-minX)*tile, y:(cell.y-minY)*tile, size:tile, kind, fill, stroke, textColor,
     glyph, label, here}`; `label` = "" for empty, else `mode==="ascii" ? glyph : (title||glyph)`.
     Ops are in CSS px (the seam scales the ctx by dpr).
   - `mapAriaLabel(model) -> string` — e.g. `"Map: <region>, floor <z> — <n> rooms, here: <currentTitle>"`
     (canvas a11y summary, replacing the lost per-cell DOM labels).
2. **`src/client/map.js`** — add `passable` to each derived cell (one line).
3. **`client-app.js`** — `renderMap`: keep the `#map-label` text logic; build
   `model = toMapModel(...)` → `drawMapCanvas(el.map, model)`. New `drawMapCanvas(canvas, model)`
   seam: `dpr = window.devicePixelRatio||1`; `size = canvasSize(model, dpr)`; set
   `canvas.width/height` (backing), `canvas.style.width/height` (css px), reset transform
   + `ctx.scale(dpr,dpr)`, `clearRect`; for each op: `fillRect` (fill), `strokeRect` at +0.5
   for crisp 1px lines (stroke), and `fillText` (centered, when `textColor && label`);
   set `canvas.setAttribute("aria-label", mapAriaLabel(model))` and `role="img"`. Handle the
   empty model (columns 0) by sizing the canvas to 0/clearing.
4. **`index.html`** — `<canvas id="map" class="map-canvas" role="img">`.
5. **`styles.css`** — replace `.map-grid`/`.map-cell`/`.map-name`/`.map-zone` (incl. the
   responsive `@media` variants) with a `.map-canvas` rule (block, centered in `.map-frame`,
   `max-width:100%`, `image-rendering:auto`). Legend + frame stay.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `src/client/canvas-map.js` | **new** pure module: `MAP_PALETTE`, `canvasSize`, `cellKind`, `toDrawPlan`, `mapAriaLabel` |
| 2 | `src/client/map.js` | add `passable: room?.passable` to the derived cell (REQ-004) |
| 3 | `src/client-app.js` | import canvas-map; rewrite `renderMap` to draw via `drawMapCanvas`; add the thin `drawMapCanvas(canvas, model)` ctx seam; drop the DOM-grid cell loop |
| 4 | `index.html` | `<div id="map" class="map-grid">` → `<canvas id="map" class="map-canvas" role="img">` |
| 5 | `styles.css` | replace `.map-grid`/`.map-cell*` (+ `@media` variants) with `.map-canvas`; keep `.map-frame`/legend |
| 6 | `tests/canvas-map.test.js` | **new** node --test: `canvasSize` DPR math, `cellKind`, `toDrawPlan`, `mapAriaLabel`, tilePixels, z-plane, passable, no-engine-dep guard |
| 7 | `docs/map-system.md`, `docs/ui-design.md` | (Phase 5) note the canvas renderer is implemented + names |

### Regression Test Plan
JS = `node --test`; smoke = browser/manual at validate (no canvas-2D in node/jsdom, documented).
| # | Test | Type | Proves |
|---|---|---|---|
| T1 | `toDrawPlan(model)` (model from `toMapModel(sample)`) yields one op per cell, ops in CSS-px grid positions | JS | REQ-001 |
| T2 | Browser smoke: `#map` is a `<canvas>`, drawn on load + after move | smoke | REQ-001, REQ-006 |
| T3 | `canvasSize`/`toDrawPlan` honor `tilePixels` (32 default; 16 → op.size 16, width=cols*16) | JS | REQ-002 |
| T4 | stacked-floor snapshot → `toDrawPlan` ops cover only the current z-plane cells | JS | REQ-003 |
| T5 | `cellKind` → empty/discovered/current/blocked (blocked = discovered && `passable===false`); `toDrawPlan` sets distinct fill/stroke + label (ascii=glyph, glyph-mode=title‖glyph) | JS | REQ-004 |
| T6 | `map.js` cell now carries `passable`; server snapshot still untouched (no `passable`/`present` added to `snap.map.rooms`) | JS | REQ-004 |
| T7 | `canvasSize` backing = `round(css*dpr)` for dpr 1/1.5/2; non-finite/≤0 dpr clamps to 1 | JS | REQ-005 |
| T8 | existing `client.test.js` (map/snapshot/room/intent) stays green; `mapAriaLabel` summarizes region/floor/current | JS | REQ-006 |
| T9 | guard test: `package.json` declares no `phaser`/`pixi`/`kiwi`/`melonjs` dependency | JS | REQ-007 |
| T10 | no `crates/**` change in the diff; `/state` server tests unchanged (still `Json<GameSnapshot>`) | review | REQ-008 |
| T11 | `tests/canvas-map.test.js` runs under `node --test` | JS | REQ-009 |
| T12 | `npm test`, `npm run build`, `./bin/gate.sh --fast` green | command | REQ-010 |
Uncoverable-by-unit-test (documented §7): T2 (real canvas draw — node/jsdom has no 2D context) is smoke; T10 is review.

### Risks / decisions
- **R1 — no canvas-2D in node/jsdom.** All logic stays in the pure `canvas-map.js`
  (sizing/classification/draw-plan/a11y are plain data); only `drawMapCanvas`'s `ctx`
  calls are untested glue (smoke). This is the load-bearing constraint behind the split.
- **R2 — JS coverage floor (75%).** The new tested module adds covered lines; the
  enlarged `client-app.js` glue stays uncounted (not imported by tests, like today). Net
  coverage should rise. Confirm at validate.
- **R3 — canvas a11y regression.** The DOM grid had per-cell `aria-label`s; a canvas is
  one node. Mitigate with `role="img"` + `mapAriaLabel(model)` summary.
- **R4 — Hi-DPI crispness.** Backing store `round(css*dpr)` + `ctx.scale(dpr,dpr)`; 1px
  strokes at +0.5 offset; integer tile origins. Re-set transform each draw (avoid compounding).
- **R5 — styles.css cleanup.** Removing `.map-cell*` must also update the responsive
  `@media` blocks that reference `.map-grid`/`.map-cell`; verify no other selector depends.
- **R6 — empty/zero-room map.** `columns===0` → size canvas to 0 and clear (no ops); seam guards.
- **Decision log:** extend `toMapModel` with `passable` (data already present); pure draw-plan
  the tests assert without a canvas; palette mirrored as constants; replace (not dual-render) `#map`.

## Phase 3 — Implement
- **Built (per manifest):**
  - `src/client/canvas-map.js` (new pure module): `MAP_PALETTE` (mirrors styles.css +
    a `blocked` variant), `canvasSize(model,dpr)` (backing = round(css*dpr), clamps bad
    dpr→1), `cellKind(cell)`, `toDrawPlan(model)` (one CSS-px op/cell), `mapAriaLabel(model)`.
    DOM-free.
  - `src/client/map.js`: added `passable: room?.passable` to the derived cell (preserves
    missing/legacy payloads as unknown rather than blocked).
  - `src/client-app.js`: import canvas-map; `renderMap` keeps the `#map-label` logic then
    calls the new thin `drawMapCanvas(el.map, model)` seam (dpr sizing, `ctx.setTransform`
    scale, `clearRect`, per-op `fillRect`/`strokeRect(+0.5)`/`fillText`, `aria-label` via
    `mapAriaLabel`, empty-model guard). Dropped the DOM cell loop.
  - `index.html`: `<div id="map" class="map-grid">` → `<canvas id="map" class="map-canvas" role="img" aria-label="Map">`.
  - `styles.css`: replaced `.map-grid`/`.map-cell*`/`.map-name`/`.map-zone` (+ the responsive
    `@media` variants) with a small `.map-canvas` rule; kept `.map-frame` + legend.
  - `tests/canvas-map.test.js` (new, colocated): 9 tests — DPR sizing, cellKind, draw-plan
    positions/labels/colors, ascii vs glyph label, z-plane (via toMapModel), passable +
    snapshot immutability, mapAriaLabel, no-engine guard.
- **Verified so far:** `node --check` both JS files; `node --test tests/*.test.js` → **25/25**
  (canvas-map 9 + client 8 + game 4 + vendor 4); `npm run build` OK and `dist/index.html`
  ships `<canvas id="map" class="map-canvas">`. No `crates/**`/Rust touched. (Full `gate.sh`
  is Phase 4.)
- **Deviations from design (+ reason):**
  - The seam draws `op.glyph` (single char) on-tile rather than `op.label` (the title): a full
    room title does not fit a 32px tile. The title is surfaced via the canvas `aria-label`
    (`mapAriaLabel`), and `toDrawPlan` still returns `label` (title/glyph per mode) for the
    tests + a future denser renderer. On-tile = "glyph hint", aria = "title hint" (REQ-004).
  - Removed the responsive `@media` `.map-grid/.map-cell/.map-name/.map-zone` rules — tile size
    now comes from `mapRenderConfig` (JS), not CSS breakpoints.
  - Canvas-map tests colocated now (module is pure; mirrors #15). Authoritative run is Phase 4.

## Inspect (Phase 3.5)
- **Lenses run** (3 parallel `general-purpose` critics over the JS/CSS diff):
  correctness (canvas-map math/seam), data-integrity + a11y/regression,
  simplification/CSS-hygiene. Critics verified concretely (throwaway node probes,
  a fake-canvas call-order trace, repo greps, `node --test`). **No CRITICAL/HIGH/MEDIUM
  correctness or security defects; REQ-002/004/006/008 confirmed.** All actionable
  items LOW + one internal-consistency fix.
- **Findings:**
  | # | Sev | Finding | Verdict | Fix / reason |
  |---|---|---|---|---|
  | 1 | consistency | `toDrawPlan` computed `label` (title) but the seam draws `op.glyph`; a test asserted `label` ("glyph mode shows the title" — now false): dead field + misleading green | REAL (fixed) | At the mandated 32px tile (Decision 035) a title can't fit, so glyph-on-tile + title-via-`aria-label` is the right call. Removed the dead `label`; canvas draws the glyph; title surfaced via `mapAriaLabel`; test now asserts the drawn glyph. Title→aria documented. |
  | 2 | LOW | glyph font-size calc lived in the untested seam (the `Math.max(9,…)` clamp branch uncovered) | REAL (fixed) | Lifted to pure `glyphFontPx(tile)` (tested); seam uses `plan.glyphFontPx`. |
  | 3 | LOW | orphaned `--map-tile-size` (all consumers removed) | REAL (fixed) | Removed from `:root`. |
  | 4 | LOW | `MAP_PALETTE` comment referenced now-deleted `.map-cell*` | REAL (fixed) | Retargeted to "former map-cell palette (removed #16); sync with `.legend-*`". |
  | 5 | LOW | coverage gaps: cellKind strict `===false`, aria singular/empty, font clamp | REAL (fixed) | Added tests (passable undefined/null→discovered; aria 1-room/empty; `glyphFontPx` 9 floor). |
  | 6 | LOW | `fillText` maxWidth ≤0 for sub-4px tiles (unreachable today) | REAL (hardened) | `Math.max(1, op.size-4)` in the seam. |
  | 7 | LOW | pure fns throw on a non-`toMapModel` model | REJECTED | Documented contract is `toMapModel` output; not a public API for arbitrary callers. |
  | 8 | note | the `drawMapCanvas` seam is untested (no canvas-2D in node/jsdom) | DEFERRED (by design) | Seam is smoke-only per the pure/glue split (#15 precedent); all testable logic now lives in the pure module. |
  | 9 | LOW | legacy `src/app.js` still has DOM-grid code | REJECTED (out of scope) | Pre-existing orphan prototype, not loaded by `index.html`, untouched by this ticket. |
- **Post-fix verify:** `node --test tests/*.test.js` → **26/26**; `npm run build` OK; no `crates/**` touched.
- **Captured:** prevention rule `PR-oathstar-render-plan-test-002`.

## Phase 4 — Validate
- **Tests:** `tests/canvas-map.test.js` (10 cases, written in implement + hardened in inspect):
  DPR sizing, cellKind (incl. strict `===false`), draw-plan positions/glyph/colors, glyph-font
  floor, ascii/empty cells, z-plane (via toMapModel), passable + snapshot immutability,
  mapAriaLabel (summary + singular/empty), no-engine guard. No Rust tests (no Rust changed).
- **`cargo test --workspace`:** PASS — 122 (content 12, core 68, datastar 11, server 11, storage 20),
  0 failed (unchanged — no Rust touched).
- **`node --test tests/*.test.js` (`npm test`):** PASS — **26 pass / 0 fail** (canvas-map 10 +
  client 8 + game 4 + vendor 4).
- **`npm run build`:** OK — `dist/index.html` ships `<canvas id="map" class="map-canvas" role="img">`;
  the bundle carries the canvas-2D draw code (`getContext`/`setTransform`/`strokeRect`) and has
  no DOM-grid leftovers (`gridTemplateColumns`/`map-cell` absent).
- **`./bin/gate.sh --fast`:** **GATE GREEN [fast]** — 14/14 static gates. (Coverage+mutation are
  FULL-only, run at `/commit`.)
- **AC → proof map:**
  | REQ | Proven by |
  |---|---|
  | 001 canvas via toMapModel | `toDrawPlan` test (ops per cell) + `dist` canvas + `getContext` in bundle; browser smoke |
  | 002 32×32 configurable tiles | `canvasSize`/`toDrawPlan` honor `tilePixels` (16/32) |
  | 003 current z-plane only | toMapModel+`toDrawPlan` stacked-floors test (z=1 excluded) |
  | 004 cell kinds + glyph + passable | `cellKind` (empty/discovered/current/blocked, strict `===false`) + `passable` cell test + draw-plan colors |
  | 005 Hi-DPI crisp | `canvasSize` backing=`round(css*dpr)` (1/1.5/2, clamp) + `glyphFontPx` floor |
  | 006 panels preserved + a11y | `mapAriaLabel` tests; `#map-label`/legend/room/exit/intent/feed untouched (inspect-verified); browser smoke |
  | 007 no engine dep | `package.json` no-engine guard test |
  | 008 server map stays JSON | no `crates/**`/`.rs` in the diff; `/state` unchanged (`Json<GameSnapshot>`) |
  | 009 JS model tests | `tests/canvas-map.test.js` under `node --test` |
  | 010 npm test+build+gate | all green (above) |
- **Manual browser smoke (REQ-001/006 — not gate-coverable; node/jsdom has no canvas-2D):**
  1. `npm run server:dev` (127.0.0.1:7878); 2. `npm run dev` (127.0.0.1:5173); 3. open
  `http://127.0.0.1:5173`; 4. `#map` is a `<canvas>` with `width`/`height` backing-store attrs
  (on a retina display `width === cssWidth * devicePixelRatio` → crisp); 5. the current room is a
  brass tile, discovered rooms are dark tiles, the glyph is drawn centered; 6. move via the Exit Pad
  → the canvas redraws and the canvas `aria-label` updates; 7. the map label shows region/floor and
  the legend, room panel, exit pad, intent panel, and Datastar event feed still work.
- **Pre-existing exclusions:** none (no pre-existing test failures; no Rust touched).

## Phase 5 — Complete
- Docs updated: `docs/map-system.md`, `docs/ui-design.md`, local ticket doc, and this
  completed pipeline notes/spec pair.
- Forge capture: AAR `625818a1-34f2-4263-9c2d-492cb4184209`; captured
  `AD-oathstar-canvas-map-renderer-002` and `PR-oathstar-render-plan-test-002`.
- Ticket closed: forge #16 `f69a718c-7ae8-418c-895e-9e1baae72f98` → `done`; local
  ticket moved to `docs/planning/tickets/closed/TICKET-16-canvas-grid-map-renderer.md`.
- Archived: active pipeline moved to
  `docs/planning/pipeline/completed/WORK-canvas-grid-map-renderer.spec.md` and
  `docs/planning/pipeline/completed/WORK-canvas-grid-map-renderer.notes.md`.
