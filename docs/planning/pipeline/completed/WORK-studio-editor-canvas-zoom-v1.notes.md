# WORK-studio-editor-canvas-zoom-v1 — Notes

## Phase 1 — Plan
- **Request:** an adjustable canvas **zoom** (render tile size slider) for the editor — view-only;
  size the map to fit / avoid the horizontal scrollbar. From the owner's "preset where we add the tile
  size" → "Adjustable render size (zoom)". Part of the studio-editable-world program.
- **Classification / tier:** work pipeline, one small studio-only slice. `editor-canvas.js`
  (`editorClampTilePixels`) + `render.rs` EDITOR_GLUE (the slider + `resizeCanvas` + `TILE`→`tileSize`)
  + tests + `map-system.md`. No Rust domain change, no endpoint, no doc-model change.
- **Recon (main `8ec0f8b`, render.rs EDITOR_GLUE):**
  - `const TILE = 40` (417) → 6 usages: the sizing block (423-427, top-level at load), drawGrid
    grid-lines (442-447), `redraw`→`editorDrawPlan` (454 `tilePixels: TILE`), `cellAt` (534).
  - `redraw();` is called at load (511, after the 423-427 sizing). So `resizeCanvas()` (the factored
    423-427) is called at load where the block was; the load `redraw()` stays.
  - `editorCanvasSize(doc,{tilePixels})` / `editorDrawPlan(doc,{z,tilePixels})` already parameterize on
    `tilePixels` — the render is zoom-ready; only the glue's `TILE` const needs to become mutable.
  - Controls at 887-890 (`#map-name`/`#map-title`/`#validate`) — `#zoom` slots in after `#map-title`.
  - One render test asserts `editorDrawPlan(doc, { z: Z, tilePixels: TILE, focusSubregion })` (1380) →
    update `TILE`→`tileSize`.
- **Approach (design refines):** pure `editorClampTilePixels`; `let tileSize = 40`; `resizeCanvas()`;
  a `#zoom` range slider + `#zoom-px` readout; the `input` handler clamps → `tileSize` → resize+redraw.
- **Decisions:** zoom is VIEW state (never written to `doc`; `doc.tile_size` ≠ render scale); `cellAt`
  uses `tileSize` (paint #48 + room-click #64 hit-test at any zoom); range 8–80 step 4 default 40;
  readout via `textContent`.
- **EARS:** REQ-001 `editorClampTilePixels` floor+clamp/fallback · REQ-002 the zoom slider re-sizes +
  redraws · REQ-003 view-only (doc untouched) · REQ-004 gate.
- **Mutation surface:** `editorClampTilePixels`'s `Math.min`/`Math.max` + the `Number.isFinite` branch —
  killed by the node below-min/above-max/in-range/fallback cases. The glue is server-string (no viable
  Rust mutant; render-assert coverage).
- **Ticket:** forge **#65** `47713464-4ffc-4267-b8bc-86c6c50778a0`. Local doc
  `docs/planning/tickets/open/TICKET-65-studio-editor-canvas-zoom.md`.
- **aar_id:** `1552d4aa-0100-435b-9472-ecf8b6f260e2`
- **Delivery:** AUTONOMOUS through commit+push+FF-merge. Branch off `main` `8ec0f8b`. Stash parked.

## Phase 2 — Design

### Code reconnaissance (the load-order catch)
- All `TILE` occurrences: 417 (const), 423 (sizing), 442/443/446/447 (drawGrid), 454 (editorDrawPlan),
  534 (cellAt) + the **one** test assert at 1380. No other test references `TILE`.
- **CATCH:** the sizing block computes `const size = editorCanvasSize(…)` (423) and **`redraw()` reads
  `size`** — line 453 `ctx.clearRect(0, 0, size.cssWidth, size.cssHeight)`. So `size` must stay
  reachable from `redraw()` after we factor the block out → make it a **module-level `let size;` that
  `resizeCanvas()` reassigns** (not a local inside `resizeCanvas`). Load order: `let tileSize` (417) →
  `let size;` + `resizeCanvas()` def + `resizeCanvas();` call (replacing 423-427) → `redraw()` def
  (452, reads `size`) → load `redraw();` (511). `canvas.setAttribute("aria-label", …)` (428) stays.

### Approach / architecture (studio-only; no Rust/doc-model change)
- **`editor-canvas.js`** — NEW `export function editorClampTilePixels(value, min, max, fallback)` →
  `const n = Math.floor(Number(value)); return Number.isFinite(n) ? Math.min(max, Math.max(min, n)) :
  fallback;` (pure). **Nuance (intended + tested):** `Number("")===0` is finite, so `""` clamps to
  `min`; only a genuinely non-numeric value (`"x"`, `undefined`, `NaN`) → `fallback`. A range slider
  only ever sends a valid number string, so `fallback` is purely defensive.
- **`render.rs` EDITOR_GLUE:**
  - `const TILE = 40` → `let tileSize = 40`; replace all 6 `TILE` usages (sizing, drawGrid ×4,
    editorDrawPlan, cellAt) with `tileSize`.
  - Replace 423-427 with `let size;` + `function resizeCanvas() { size = editorCanvasSize(doc,
    { tilePixels: tileSize, devicePixelRatio: window.devicePixelRatio }); canvas.width =
    size.backingWidth; canvas.height = size.backingHeight; canvas.style.width = size.cssWidth + "px";
    canvas.style.height = size.cssHeight + "px"; }` + `resizeCanvas();`. `redraw()` unchanged (reads the
    module `size`).
  - Controls (after `#map-title`, 888): `<label>Zoom <input id="zoom" type="range" min="8" max="80"
    step="4" value="40"><span id="zoom-px">40px</span></label>`.
  - Handler (placed after the load `redraw();`, where `tileSize`/`resizeCanvas`/`redraw` are in scope):
    `const zoom = document.getElementById("zoom"); const zoomPx = document.getElementById("zoom-px");
    zoom.addEventListener("input", () => { tileSize = editorClampTilePixels(zoom.value, 8, 80, 40);
    zoomPx.textContent = tileSize + "px"; resizeCanvas(); redraw(); });`
  - Update the 1380 assert `tilePixels: TILE` → `tilePixels: tileSize`.

### Locked decisions (this phase)
- **View-only** — `tileSize` is module state, never written to `doc`; Save/Validate/Activate +
  `doc.tile_size` untouched (source sampling ≠ render scale).
- **`cellAt` uses `tileSize`** — paint (#48) + room-click (#64) hit-test at any zoom.
- **`size` is module-level** so `resizeCanvas()` re-points it and `redraw()` clears at the new size.
- Range **8–80 step 4 default 40**; `#zoom-px` readout via `textContent`. `""`→min, non-numeric→fallback.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-studio/static/editor-canvas.js` | NEW pure `editorClampTilePixels`. |
| 2 | `crates/oathstar-studio/src/render.rs` | `TILE`→`tileSize`; `let size;` + `resizeCanvas()`; zoom control + handler; 1380 test update. |
| 3 | `tests/studio-editor-canvas.test.js` | `editorClampTilePixels` cases. |
| 4 | `docs/map-system.md` | (Phase 5) the editor has a zoom slider — adjustable render tile size, view-only. |

### Regression Test Plan
| # | Test | Proves |
|---|---|---|
| T1 | `editorClampTilePixels` — `40→40`, **`4→8` (below min)**, **`200→80` (above max)**, `23.9→23` (floors), **`"x"→40` (fallback, NaN)**, `undefined→40` (fallback), `""→8` (`Number("")=0`→min, documented intentional) | REQ-001 (node) |
| T2 | render `editor_page_has_a_zoom_control` — html contains `id="zoom"`, `id="zoom-px"`, `editorClampTilePixels(`, `resizeCanvas(`, `let tileSize = 40`, `tilePixels: tileSize` | REQ-002 (render assert) |
| T3 | existing #48 paint / #61 tab / #63 inspector / #64 canvas-select render+canvas tests stay green (the 1380 assert updated `TILE`→`tileSize`); no test asserts `tileSize` is written to `doc` | REQ-003 regression |
| G1 | `bin/gate.sh` FULL green, MSI 100% | REQ-004 |
- **Mutation:** `editorClampTilePixels`'s `Math.min`/`Math.max` (the below-min/above-max T1 cases) + the
  `Number.isFinite` branch (the `"x"`/`undefined` fallback cases). The glue is server-string (no viable
  Rust mutant; render-assert coverage). **Uncoverable:** none new.

### Risks / decisions
1. **`size` module-level (the catch above)** — if left as a `const` local to `resizeCanvas`, `redraw()`
   would throw `size is not defined`. Made a module `let`. (The existing canvas tests would catch a
   regression here only at runtime, not in `node` — so the render-assert + manual smoke is the guard;
   the rename is mechanical.)
2. **`""` vs non-numeric fallback** — documented in T1 names so the `Number("")===0` behavior is
   intentional, not a latent bug.
3. **Handler scope/order** — placed after the load `redraw();` so `tileSize`/`resizeCanvas`/`redraw` +
   the fetched els are all defined.

## Phase 3 — Implement
- **Built (manifest as designed):**
  - `editor-canvas.js` — NEW pure `editorClampTilePixels(value, min, max, fallback)` after
    `editorRoomAt`.
  - `render.rs` EDITOR_GLUE — `const TILE = 40` → `let tileSize = 40`; all 8 `TILE` usages → `tileSize`
    (verified `grep \bTILE\b` → none left); `resizeCanvas()` + `let size`; the `#zoom` slider + `#zoom-px`
    readout after `#map-title`; the `zoom` `input` handler (clamp → `tileSize` → `resizeCanvas()` +
    `redraw()`) after the load `redraw();`; the one render-test assertion `tilePixels: TILE` →
    `tilePixels: tileSize`.
- **Deviation (necessary correctness fix beyond the Phase-2 sketch):** the design said "factor the
  sizing block (423-427) into `resizeCanvas()`", but lines **429-434** (`const ctx = …; ctx.scale(dpr);
  imageSmoothingEnabled; font; textBaseline; textAlign`) apply the dpr transform + text style **once**
  after the load sizing — and **setting `canvas.width` resets the 2D context**, so a *zoom* re-size would
  drop them and draw unscaled/with the default font. Fix: move `const ctx = …` **above** `resizeCanvas`,
  and fold `ctx.scale(size.dpr,…)` + the four `ctx` settings **into** `resizeCanvas()` so they re-apply
  on every (re)size. (`size` is a module `let`, so `redraw()`'s `clearRect(…, size.cssWidth, …)` and
  `ctx.scale(size.dpr)` both see the current size.)
- **Checks:** `node --check editor-canvas.js` OK; `clippy -p oathstar-studio --all-targets` clean;
  `cargo fmt` clean; `cargo test -p oathstar-studio` → **102 passed** (the `TILE`→`tileSize` rename kept
  the #48 paint / #61 tab / #63 inspector / #64 canvas-select tests green; the 1380 assert updated).
  New tests + gate at Phase 4.

## Inspect (Phase 3.5)
- **Lenses:** 2 read-only `Explore` critics — correctness/ctx-reapply + rename-regression/view-only.
  **Both CLEAN.**
- **Critic 1 (correctness) — no findings.** The **ctx-reset re-apply** is sound: `const ctx` is defined
  **before** `resizeCanvas()` is called (no TDZ); `resizeCanvas()` sets `canvas.width/height` (which
  resets the 2D context) **then** re-applies `ctx.scale(size.dpr)` + smoothing/font/baseline/align — so
  both load and zoom produce a scaled, styled context; the scale is **not cumulative** (each
  `canvas.width` assignment resets the transform to identity before `ctx.scale(dpr)`); `size` is a
  module `let` so `redraw()`'s `clearRect`/`ctx.scale` see the current size. `editorClampTilePixels`
  verified across in-range / below-min / above-max / float-floor / `"x"`·`undefined`·NaN→fallback /
  `""`→min. Handler scope correct. node + cargo 102 pass.
- **Critic 2 (rename/view-only) — clean.** `grep \bTILE\b` → zero; all usages (sizing, drawGrid,
  editorDrawPlan, cellAt) → `tileSize`, no over-replacement; **paint (#48/#57) + room-click (#64)
  hit-test correctly at any zoom** (`cellAt` uses `tileSize`; `paintRect` operates on logical cells,
  zoom-independent); **view-only** — the handler/`resizeCanvas` never assign `doc.*` (grep clean on
  `doc.tile_size`/`doc.tileSize`); Save posts the unchanged `doc`; `#zoom-px` via `textContent`. 102
  pass.
- **One [low] — REJECTED.** Critic 2 suggested an `if (zoom)` null-guard on the handler. Rejected: the
  `#zoom` control is **always** rendered in the editor controls — exactly like `#save`/`#validate`/
  `#map-name`, none of which guard. The codebase convention guards only *optionally*-present elements
  (`#palette` when no tileset, `#roomSave` in a tab). A guard only on `#zoom` would be inconsistent. The
  critic itself marked it "not required / not critical."
- **No code fix; no `failure-record`** — the one real subtlety (setting `canvas.width` resets the 2D
  context, so the dpr transform + text style must re-apply on resize) was caught and handled **in the
  implement phase** (a documented deviation), not shipped as a defect. Worth a light prevention note at
  complete (a reusable canvas gotcha). The Phase-4 tests are scope, not findings.

## Phase 4 — Validate
- **Tests added (T1–T2, green):**
  - `tests/studio-editor-canvas.test.js` — **T1** `editorClampTilePixels: floor + clamp to [min,max];
    non-numeric → fallback (#65)` — `40→40`, **`4→8` (below min)**, **`200→80` (above max)**, `23.9→23`
    (floors), **`"x"→40` / `undefined→40` (fallback)**, `""→8` (`Number("")=0`→min, documented).
  - `render.rs` — **T2** `editor_page_has_a_zoom_control` — the glue contains `id="zoom"`, `id="zoom-px"`,
    `editorClampTilePixels(`, `resizeCanvas(`, `let tileSize = 40`, `tilePixels: tileSize`.
- `cargo test --workspace`: **PASS** — oathstar-studio **103** (+1).
- `node --test tests/*.test.js`: **PASS** — **94** tests, 0 fail (+1 `editorClampTilePixels`).
- `bin/gate.sh`: **GATE GREEN [full]** — all 17 gates; **mutation 600 caught / 0 missed → MSI 100.0%**
  (`editorClampTilePixels`'s `Math.min`/`Math.max` + `Number.isFinite` branch are JS — killed by the T1
  below-min/above-max/fallback cases; the glue is server-string with no viable Rust mutant, covered by
  T2; the `TILE`→`tileSize` rename kept #48/#61/#63/#64 green). No pre-existing exclusions.

## Phase 5 — Complete
- Docs / forge / ticket / archived:
