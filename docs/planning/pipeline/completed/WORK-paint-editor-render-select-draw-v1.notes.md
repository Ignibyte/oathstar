# WORK-paint-editor-render-select-draw-v1 — Notes

## Phase 1 — Plan
- **Request:** the paintable editor — make the studio `/editor` visibly render
  the arctic tileset and let the author select a tile and paint it onto the
  tilemap. Paint-system S2 (render) + S3 (palette/paint) merged into one visible,
  interactive slice on the #47 model. One autonomous run; the owner reviews
  visually tomorrow.
- **Intake source:** `INTAKE-paint-system-tile-editor.md` — this is its S2+S3
  (S1 = #47, shipped). Intake updated to note S2+S3 promoted to #48.
- **Classification / tier:** work pipeline, one slice. The gate rides on the
  PURE node-testable logic in `editor-canvas.js` (index→source-rect, point→cell,
  paint mutation, palette→index, sprite draw-plan) at 100% MSI; the DOM/canvas/
  mouse/image-load glue is the smoke-/review-verified seam (the existing #45
  split). If too big for one green slice, prioritise VISIBLE arctic rendering +
  basic paint over polish.
- **Forge recall:** pre-flight green (forge up, no bulletins, no active
  pipeline). Surfaced `AD-claude-paint-layer-model-001` (#47, the model this
  builds on) + the #46 tileset contract. Editor seam (`render.rs` `EDITOR_GLUE`
  + `static/editor-canvas.js`) read this session. Heeding
  `PR-claude-non-square-capacity-fixture-001` (non-square test tilesets).
- **Ticket:** #48 `42024e47-caba-44b7-865b-c09f4a8a941d` (feature).
- **EARS reviewed:** REQ-001..008 — the 5 pure units + the arctic descriptor +
  the page/seam markers + the gate. The seam is the declared genuinely-
  uncoverable path.
- **AAR id:** 6da99592-773b-4c59-9fae-9c9b79bca909

## Phase 2 — Design

### Approach / architecture
- **arctic descriptor** — `public/tilesets/arctic.json`:
  `{ name:"arctic", tileSize:8, columns:30, rows:203, image:"arctic.png",
  tiles:[ the 4 #46 load-bearing names at row 0 ] }`. With the 4 names it is a
  valid #46 tileset (the game client's `validateTileset` passes — reusable);
  the editor reads `columns`/`tileSize`/`image` and paints by INDEX.
- **5 pure fns in `editor-canvas.js`** (node-tested). NOTE: mutation (gate 17) is
  cargo-mutants = **Rust-only**, so these carry the JS **coverage** gate
  (editor-canvas.js stays ~100%), not MSI; still written with clear boundaries.
  - `tileIndexToSourceRect(index, columns, tileSize)` -> `{sx:(index%columns)*ts,
    sy:Math.floor(index/columns)*ts, size:ts}`.
  - `canvasPointToCell(px, py, tilePixels, width, height)` -> `{x,y}` (floor
    division) or `null` when `px/py<0` or `x>=width`/`y>=height`.
  - `paletteIndexAtPoint(px, py, columns, tileSize, scale, tileCount)` ->
    `index` or `null` (outside columns, or `index>=tileCount`).
  - `paintCell(doc, layerId, cell, tileRef)` -> a NEW doc with the named layer's
    cell at `(x,y,z)` removed-then-appended (dedup by coordinate; immutable;
    no-op if the layer is absent — caller ensures it exists).
  - `editorDrawPlan` extended **additively**: each op gains `sprites: []`;
    extract `tilesetsById(doc)` + `layerSpritesByCell(doc, z, tilesets)` so each
    painted layer cell contributes `{sx,sy,sSize,tileset}` (layer order = bottom
    to top) to its op's `sprites`. Existing fill/stroke/glyph/determinism
    UNCHANGED (the `doc()` test helper omits tilesets/layers -> `sprites:[]`).
- **The seam** (`render.rs` `editor_page` HTML + `EDITOR_GLUE`; `static/studio.css`;
  `editor.rs` `STARTER_DOC`):
  - `STARTER_DOC` gains the arctic tileset + an empty `"ground"` layer (so the
    page opens paintable). Map `tile_size` stays 16 (the source-grid unit); the
    arctic tileset's 8px is independent (#47) — an 8px sprite upscales into the
    TILE=24 editor cell.
  - `editor_page` adds a **palette `<canvas id="palette">`** panel + an
    active-tile indicator beside `#map`.
  - `EDITOR_GLUE`: load `arctic.png` via `Image()`; draw the palette (the scaled
    sheet); palette click -> `paletteIndexAtPoint` -> active index; `#map`
    `mousedown`/`mousemove`(while down) -> `canvasPointToCell` -> `paintCell` ->
    repaint; `drawMap` runs `editorDrawPlan` and per op: **sprite-or-fill** (if
    `op.sprites.length` blit each via `drawImage(sx,sy,sSize -> x,y,size)`, else
    `fillRect(op.fill)`), then stroke (grid), then glyph; `imageSmoothingEnabled
    =false`.
- **State/view**: pure draw/paint logic in `editor-canvas.js`; all DOM/canvas/
  mouse/`Image` in `EDITOR_GLUE` (the smoke seam) — the #45 split.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `public/tilesets/arctic.json` (NEW) | the arctic descriptor (tileSize 8, 30x203, image, 4 kind names). |
| 2 | `crates/oathstar-studio/static/editor-canvas.js` | + `tileIndexToSourceRect`, `canvasPointToCell`, `paletteIndexAtPoint`, `paintCell`, `tilesetsById`, `layerSpritesByCell`; `editorDrawPlan` emits per-op `sprites[]` (additive). |
| 3 | `crates/oathstar-studio/src/render.rs` | `editor_page`: palette canvas + active-tile UI; `EDITOR_GLUE`: arctic load, palette draw+click->select, map paint handlers, sprite blit (smoothing off). Render test asserts palette/glue markers. |
| 4 | `crates/oathstar-studio/static/studio.css` | palette panel layout (scrollable, sized). |
| 5 | `crates/oathstar-studio/src/editor.rs` | `STARTER_DOC` gains the arctic tileset + an empty `"ground"` layer. |
| 6 | `tests/studio-editor-canvas.test.js` | node tests: the 5 pure fns (non-square tileset, exact boundaries) + `editorDrawPlan` additive. |
| 7 | `tests/arctic-descriptor.test.js` (NEW) | arctic.json geometry + `validateTileset` (imports `src/client/tileset.js`). |

### Regression Test Plan
| # | Test | Proves Req |
|---|---|---|
| RT-1 | `tileIndexToSourceRect`: columns 30, ts 8 — index 0→{0,0,8}, 30→{0,8,8}, 31→{8,8,8}, 5→{40,0,8} | REQ-001 |
| RT-2 | `canvasPointToCell`: in-grid floor; `px/py<0`→null; `x>=width`→null (exact boundary at `px==width*tile`); `y>=height`→null | REQ-002 |
| RT-3 | `paintCell`: insert on "ground"; repaint same cell replaces (no dup); paint a missing layer → doc unchanged | REQ-003 |
| RT-4 | `paletteIndexAtPoint`: in-grid index; `col>=columns`→null; `index>=tileCount`→null; `px<0`→null | REQ-004 |
| RT-5 | `editorDrawPlan` additive: doc + arctic tileset + "ground" layer with one cell → that op `sprites=[{sx,sy,sSize,tileset}]`, others `[]`; room/spawn ops + determinism unchanged | REQ-005 |
| RT-6 | `arctic.json`: tileSize 8 / columns 30 / rows 203 / image; `validateTileset` ok. Rust `starter_doc_is_valid` now covers a doc with the arctic tileset + layer validating | REQ-006 |
| RT-7 | Rust render test: `editor_page` carries `id="palette"`, the arctic ref, and the glue calls (`tileIndexToSourceRect`/`canvasPointToCell`/`paintCell`/`paletteIndexAtPoint`) | REQ-007 |
| RT-8 | `bin/gate.sh` FULL green — mutation 100% MSI (Rust render markers pinned), JS coverage ≥75% | REQ-008 |

Genuinely uncoverable: the DOM/canvas/mouse/`Image`-load seam (`EDITOR_GLUE`) — smoke/review only; the pure fns it calls ARE node-tested.

### Risks / decisions
- **JS is not cargo-mutants-mutated** (Rust-only) — the pure fns carry the JS
  *coverage* gate; the Rust `render.rs` `format!` markers carry mutation (pin
  every new marker in the render test, per the #46 format-mutant pattern).
- **Sprite-or-fill layering** — a painted cell draws its sprite (over the palette
  fill); an empty cell draws the fill (room structure still legible); stroke +
  glyph always on top.
- **One active tileset (arctic) + one active layer ("ground")** this slice;
  multi-tileset/layer management UI deferred.
- **`paintCell` is immutable** (returns a new doc) — state/view separation; the
  seam reassigns + repaints.
- `STARTER_DOC` `tile_size` stays 16; the arctic tileset's 8px is independent.
- `editor_page` HTML grows — keep it mutation-tight via the render-test markers.

## Phase 3 — Implement
- **Built:**
  - NEW `public/tilesets/arctic.json` — the arctic descriptor (tileSize 8,
    30x203, image, the 4 #46 kind names so `validateTileset` passes).
  - `editor-canvas.js`: 5 pure exports (`tileIndexToSourceRect`,
    `canvasPointToCell`, `paletteIndexAtPoint`, `paintCell`) + 2 private helpers
    (`tilesetsById`, `layerSpritesByCell`); `editorDrawPlan` now adds a `sprites`
    array to every op (additive — defaults to `[]` when the doc has no
    tilesets/layers; the `doc()` helper case).
  - `editor.rs` `STARTER_DOC` gains the arctic tileset + an empty `ground` layer.
  - `render.rs` `editor_page`: a `<canvas id="palette">` panel + active-tile
    indicator; `EDITOR_GLUE` rewritten into the paint loop (mutable `doc`,
    `redraw()`, arctic `Image()` load, palette draw + `paletteIndexAtPoint`
    select, map `mousedown`/`mousemove` -> `canvasPointToCell` -> `paintCell` ->
    repaint, sprite-or-fill blit with `imageSmoothingEnabled=false`). The render
    test gained 6 markers (`id="palette"`, `/tilesets/`, the 4 glue/fn calls) —
    pins the `format!` against mutants.
  - `studio.css`: scrollable palette panel + crosshair cursors.
  - **Checks:** `cargo clippy -p oathstar-studio --tests` strict-green;
    `cargo test -p oathstar-studio` 27 pass (STARTER_DOC validates with the new
    tileset/layer; render markers present); `editor-canvas.js` parses; existing
    editor JS tests still pass (additive).
- **Deviations from design (+ reason):** none material. `EDITOR_GLUE` holds `doc`
  as a reassigned `let` (per design — `paintCell` is immutable). The pure-fn node
  tests are deferred to Validate (the fns compile/parse; the Rust render + starter
  tests already cover the seam markers + STARTER_DOC validity).

## Inspect (Phase 3.5)
- **Lenses run (2 critics):** pure-fn correctness + additive editorDrawPlan;
  the seam + render-marker mutation pin + arctic.json + scope.
- **Findings:**
  | # | Severity | Finding | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | low (hygiene) | `raw_assets/` (~1.5 MB of third-party art zips + extracted trees) is untracked and NOT gitignored — a `git add -A` hazard | REAL | Added `raw_assets/` to `.gitignore` (selective staging already excludes it; this is belt-and-suspenders). Flagged to owner. |
  | 2 | high → Validate | The 5 new pure fns + the `editorDrawPlan` sprite branch (~80 lines) have NO node tests yet → would drop JS coverage below 75 at the gate | REAL but **phase-normal** | Validate writes them (Critic A's exact per-fn list, carried below). Both critics verified the CODE is correct. |
  | — | — | **Pure-fn correctness** — all 5 fns (row-major rect; `canvasPointToCell` exact-edge→null + last-pixel→last cell; `paletteIndexAtPoint` partial-last-row→null; `paintCell` immutable via map/filter/spread + dedup + missing-layer no-op; `editorDrawPlan` snake_case `tile_size`, bottom-first layer order, z-filter, unknown-tileset skip) | **CLEAN** (Critic A verified with throwaway node scripts) | none |
  | — | — | **Seam** (reassigned-`let doc`→redraw; click `clientX-rect.left`÷TILE — no CSS/backing mismatch; 8→24 blit; all throw-guards; fill fallback) + **render-marker mutation pin** (empty `editor_page` body fails the 6 asserts) + STARTER_DOC valid + arctic.json passes `validateTileset` | **CLEAN** (Critic B verified) | none |
  | — | — | **Scope** — only #48 files + arctic.png + the pipeline docs; the online-first WIP is untouched | **CLEAN** | none |
  | 3 | info | `columns≥1` precondition; missing-layer no-op re-allocates `layers` | **REJECTED** (non-issues, correct as designed) | none |
- **Carried to Validate (write these node tests; the code passes them already):**
  `tileIndexToSourceRect` (index 0/29/30/31 on a 30-wide sheet); `canvasPointToCell`
  (neg→null, in-cell floor, exact right/bottom edge→null, last-pixel→last cell);
  `paletteIndexAtPoint` (col overflow→null, partial last row past last tile→null,
  neg→null); `paintCell` (no-mutate deep-snapshot, replace-at-(x,y,z), add-new,
  missing-layer no-op, layers/cells-absent); `editorDrawPlan` sprite branch
  (sprites:[] when doc omits tilesets/layers; bottom-first snake_case rects;
  z-exclude; unknown-tileset skip). **Non-square inline tilesets** for clarity
  (JS is not cargo-mutants-mutated, so the non-square rule is Rust-only here).
- No failure-record: no bug — the diff is correct; the coverage gap is the normal
  Phase-3→4 boundary.

## Phase 4 — Validate
- **Tests added (7 node, in `tests/studio-editor-canvas.test.js`):**
  `tileIndexToSourceRect` (0/29/30/31 on a 30-wide sheet), `canvasPointToCell`
  (neg/in-cell/exact-edge→null/last-pixel), `paletteIndexAtPoint`
  (in-grid/col-overflow/past-tileCount/neg), `paintCell` (insert/replace/append/
  missing-layer no-op + **immutability snapshot** + cells-absent), and the
  `editorDrawPlan` sprite branch (empty-default / z-filter / unknown-tileset skip
  / bottom-first layer order). Non-square inline tilesets for clarity.
- **Validate-revealed gap → fixed (the deliverable's missing piece):** the
  `EDITOR_GLUE` loads the sheet from `/tilesets/arctic.png`, but the studio router
  served **no** such route — so the palette + every painted sprite would 404 and
  render blank, defeating the whole "see arctic tiles" goal. Closed it the
  Decision-058 way (embed, no runtime asset dir): `editor.rs` now
  `include_bytes!`-embeds `arctic.png` into `const ARCTIC_PNG` and serves it via
  `pub async fn arctic_sheet() -> Response` (`Content-Type: image/png`);
  `main.rs` adds `.route("/tilesets/arctic.png", get(editor::arctic_sheet))`. New
  Rust test `serves_the_arctic_sheet` asserts `200 OK` + `image/png` + non-empty
  body (it pins the handler's mutants). This is a small, test-pinned serving
  handler — not re-inspected (disproportionate for an embed-and-return-bytes fn);
  the byte payload is the owner's committed asset.
- **`cargo test --workspace`:** all crates green, 0 failed (studio **28** pass,
  incl. `serves_the_arctic_sheet`).
- **`node --test tests/*.test.js`:** 77 pass (70 + 7 new), 0 fail.
- **`bin/gate.sh` (FULL):** `GATE GREEN [full]` — 17/17 (re-run after the
  arctic-sheet route).
  - gate:15 rust coverage **98.74%** line (floor 94); gate:16 js coverage
    **89.22%** (floor 75; up from 88.52% — the new editor tests); gate:17 mutation
    **493 caught / 0 missed → MSI 100.0%** (492 + the one new `arctic_sheet`
    mutant, killed by `serves_the_arctic_sheet`; the Rust render-marker pins held;
    the route line lives in `main()`, which cargo-mutants excludes).
- **Pre-existing exclusions:** none. The online-first WIP + `raw_assets/` (now
  gitignored) stay out of scope — selective staging at `/commit`.

## Phase 5 — Complete
- **Docs updated:** `docs/map-system.md` — added a "Paintable (ticket #48)"
  paragraph under the studio-editor section (palette + paint loop, the embedded
  `GET /tilesets/arctic.png` sheet route, the new pure fns + sprite-augmented
  `editorDrawPlan`, what v1 paints vs. what's deferred). Cited the established
  `#45` embed pattern rather than the uncommitted Decision 058 (it lives only in
  the online-first WIP working tree, not HEAD — avoided a dangling reference).
  `docs/ui-design.md` unchanged (it is the player-client doc; the editor palette
  is a studio surface, documented in map-system.md). No `decisions.md` change —
  the embed is an application of the already-committed `#45` pattern.
- **Forge capture:**
  - `failure-record` **BF-claude-glue-fetches-unserved-asset-001** (validation,
    high): the `EDITOR_GLUE` fetched `/tilesets/arctic.png` but the router served
    no such route — palette + sprites would 404 and render blank while the whole
    gate stayed green (pure-fn + HTML-marker tests never exercise the network
    path). Caught at Validate by tracing the glue's fetch URLs against the route
    table; fixed via the `include_bytes!` embed + `arctic_sheet` handler + route +
    `serves_the_arctic_sheet` test.
  - `prevention-rule-record` **PR-claude-serve-every-asset-the-glue-fetches-001**
    (high): serve every asset/endpoint URL the glue references in the SAME slice,
    with a GET test (status + content-type + non-empty body); trace glue URLs
    against the router at inspect/validate.
  - `aar-submit` 6da99592 — outcome completed, effectiveness 4, materialized the
    BF + PR above (2 novel findings; 12 verdicts written).
- **Ticket closed:** #48 `42024e47-…` → done.
- **Archived:** spec+notes → `docs/planning/pipeline/completed/`; local ticket
  doc → `docs/planning/tickets/closed/` (status closed).
