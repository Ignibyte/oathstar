# WORK-scale-map-32px-enlarged-canvas — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

- **pipeline_id:** 649f5994-f284-4910-99fd-bd6fbc2ad2d7
- **aar_id:** bd639a22-c009-4a91-b316-8ae9fc24d4b1

## Phase 1 — Plan
- **Request:** `/work 37 auto approve` — ticket #37 "Scale the map: 32px tiles
  and an enlarged canvas." Owner direction 2026-06-12: the map is too small to
  read; make cells 32px and the canvas physically bigger/crisper. Run autonomous
  through `/commit` ("auto approve").
- **Intake source:** none — direct owner direction.
- **Classification / tier:** work pipeline (single shippable client+asset slice).
  Systems: ui (map renderer) + frontend assets. No engine/wire.
- **Forge recall (lessons/failures surfaced):**
  - `docs-search`: Decision 050 (sprite tiles render from a committed,
    name-keyed tileset + permanent flat-color fallback); Decision 054 (blank-
    colors era — uniformity pin decodes the PNG and asserts every named tile is
    N identical opaque pixels == declared color; deterministic, checksum-guarded
    regen). UI-design Map Direction + map-system Rendering Direction.
  - `knowledge-search`: top hits were the tileset-contract failures + the
    blank-colors / name-keyed architecture decisions (same lineage as #32/#36).
    No bulletins active.
  - Prevention rule of note (from #38 lineage, lower relevance here):
    render-plan ops should carry only fields that are actually drawn — don't add
    speculative fields to the draw plan.
- **Anchors verified at plan** (Explore recon — read-only):
  - `src/client/map.js:10` — `DEFAULT_MAP_CONFIG = { tilePixels: 32, mode: "glyph" }`
    → **dest cell already 32px**; today 16px source upscales 2× to 32px dest.
  - `src/client/canvas-map.js` — `canvasSize(model,dpr)` (cssWidth = columns ×
    tilePixels; backingWidth = cssWidth × dpr), `toDrawPlan(model,tileset)`
    (op `size = tilePixels`, `glyphFontPx = max(9, round(tile×0.4))`, sprite rect
    carries `sSize = tileset.tileSize`), `cellKind`, `mapAriaLabel`, `MAP_PALETTE`.
    Pure — no hardcoded 16; reads geometry from the model. **No change needed**
    beyond what dest-size choice implies.
  - `src/client/tileset.js` — `KIND_TILE_NAMES` (4 load-bearing), `validateTileset`
    reads `tileSize/columns/rows` from JSON, `tileRect` returns
    `{sx,sy,sSize: tileset.tileSize}`. Pure; no hardcoded 16.
  - `src/client-app.js` — `drawMapCanvas` seam (~L496): sets `canvas.width/height`
    = backing, `canvas.style.width/height` = css, `ctx.setTransform(dpr,…)`,
    `imageSmoothingEnabled=false`, `drawImage(img, sx,sy,sSize,sSize, x,y,size,size)`.
    `TILESET_DIR = "/tilesets/oathstar-starter-16x16"` (L45) — **path baked with
    "16x16"**. `mapRenderConfig = { ...DEFAULT_MAP_CONFIG }` (L38).
  - `index.html:73` `<canvas id="map" class="map-canvas">`; `styles.css:254`
    `.map-canvas { image-rendering: pixelated }` — **no width/height in CSS**
    (the seam sets pixel + CSS size in JS).
  - Asset: `public/tilesets/oathstar-starter-16x16/` — JSON (`tileSize:16,
    columns:4, rows:3, 11 tiles`), PNG 64×48 (225 bytes), `.tsx`, preview 256×192.
    **Dir + filenames bake in "16x16".**
  - Generator: `bin/generate_oathstar_tileset.py` — `TILE=16, COLS=4, ROWS=3,
    NAME="oathstar-starter-16x16"`; `FLAT_TILES` table (11 named solid colors);
    sheet = `(COLS*TILE, ROWS*TILE)`; deterministic.
  - Contract tests: `tests/tileset.test.js` (tileSize==16, tileRect sSize/sx/sy,
    PNG w/h == cols×tileSize / rows×tileSize, per-tile uniformity loop over
    `raw.tileSize`, `.tsx` `tilecount/columns` match) and
    `tests/canvas-map.test.js` (committed-tileset draw: `current.sprite.sSize==16`,
    `current.size==32`; plus function-level unit assertions on local models).
  - **Engine/wire confirmed pixel-free:** `crates/oathstar-protocol/src/lib.rs`
    `MapRoomSnapshot { id,title,x,y,z,glyph,passable,discovered,current,exits,
    has_hostiles,has_items }` — no tile/pixel field. No Rust change required.
- **Ticket:** forge `5a348bfb-4c21-4696-bea3-4597c32da5da` (#37, already minted);
  local doc `docs/planning/tickets/open/TICKET-37-scale-map-32px-tiles-enlarged-canvas.md`.
- **EARS requirements reviewed:** REQ-001 native-32px source, REQ-002 enlarged
  on-page (design picks lever), REQ-003 contract + uniformity re-scaled to 32,
  REQ-004 wire/engine unchanged. Finalized in spec.
- **Open design questions for Phase 2** (see spec "Design-Deferred"): enlargement
  mechanism + final cell px; rename `16x16`→`32x32` dir (lean yes); which
  byte-size/`.tsx` pins move.

## Phase 2 — Design
- **Approach / architecture** — client + asset only (engine/wire confirmed
  pixel-free). Two faithful levers, both honoring the existing pure-model design:
  1. **Source sheet → native 32px.** Generator `TILE=32` → 128×96 sheet — exactly
     the ticket's "regen at 32×32." Dir renamed `oathstar-starter-16x16` →
     `oathstar-starter-32x32` (name must reflect geometry). Stays name-keyed
     (Decision 050) + flat-uniform (Decision 054); the pixel uniformity pin
     auto-scales (it loops `raw.tileSize`, so 16×16=256 → 32×32=1024 px/tile with
     no test-logic change — only the hardcoded geometry literals move). REQ-001/003.
  2. **On-page enlargement → `DEFAULT_MAP_CONFIG.tilePixels` 32 → 64.** The
     existing, node-testable knob; `mapRenderConfig` stays the live runtime hook
     (ui-design "configurable tile sizes"). `canvasSize`/`toDrawPlan` already
     derive both css and backing from `tilePixels`, so cssWidth **and**
     backingWidth double → satisfies BOTH "CSS size up" and "device-pixel size
     up." The seam blits 32px source → 64px dest at **integer 2× nearest-neighbor**
     (`imageSmoothingEnabled=false`, already set) — crisp. REQ-002.
  - **Reconciliation:** dest was already 32px (16px source upscaled 2×). Beginner
    world is x∈{0,1}, y∈{−3..0} → the z-plane box is ~2×4 cells → at 32px the map
    is only ~64×128 css px (validates "too small to read"); at 64px ~128×256 px —
    legible. `.map-frame { overflow:auto }` scrolls larger worlds, so 64 is safe
    at any extent. `mapRenderConfig` makes 64 a one-line tunable later.
  - **`canvas-map.js` & `tileset.js` need NO change** — pure + parameterized
    (read `tileSize` from JSON, `tilePixels` from model). The change is the
    generator constant, the regenerated asset, two one-line config/path edits,
    and the contract-test geometry literals.
- **File manifest:**
  | # | File | Change |
  |---|---|---|
  | 1 | `bin/generate_oathstar_tileset.py` | `TILE` 16→32; `NAME` `oathstar-starter-16x16`→`oathstar-starter-32x32` (COLS/ROWS stay 4/3). |
  | 2 | `public/tilesets/oathstar-starter-32x32/{png,json,tsx,-preview.png}` | Regen via generator → 128×96 sheet, 32px tiles. `git rm -r` old `oathstar-starter-16x16/`. |
  | 3 | `public/tilesets/oathstar-starter-32x32/README.md` | Move + update (64×48→128×96, 16px→32px, filenames). |
  | 4 | `src/client/map.js` | `DEFAULT_MAP_CONFIG.tilePixels` 32→64 (+ comment: 2× on-page enlargement, `mapRenderConfig` is the live knob). |
  | 5 | `src/client-app.js` | `TILESET_DIR` + `TILESET_JSON_URL`: `oathstar-starter-16x16`→`-32x32`. |
  | 6 | `tests/tileset.test.js` | Rename paths (L14–17) + image name (L123); `tileSize` 16→32 (L120); `sSize` 16→32 (L129); grass→`{sx:0,sy:32,sSize:32}` (L190); stone_floor→`{sx:32,sy:0,sSize:32}` (L191). `minimalTiles` STAYS 16 (validator generality). |
  | 7 | `tests/canvas-map.test.js` | Rename committed path (L198); `sSize` 16→32 (L256); ADD REQ-002 enlargement test (DEFAULT tilePixels===64 + canvasSize doubling). |
  | 8 | `docs/map-system.md`, `docs/ui-design.md` | Path refs → `-32x32`; note 32px source / 64px dest. (Phase 5 doc pass.) |
  | 9 | `docs/decisions.md` | Phase 5: append a short decision (054 lineage) recording #37's 32px-source / 64px-dest scale. |
- ### Regression Test Plan
  | # | Test | Proves Requirement |
  |---|---|---|
  | T1 | tileset.test.js "committed tileset validates" — `tileSize===32`, image name `-32x32.png`, every `tileRect` `sSize===32`, 11 tiles | REQ-001, REQ-003 |
  | T2 | tileset.test.js "tileRect resolves committed names" — grass `{0,32,32}`, stone_floor `{32,0,32}` | REQ-001 |
  | T3 | tileset.test.js "every committed tile is one uniform color" — sheet 128×96, each tile 32×32 px uniform == declared color, spare slot transparent | REQ-003 |
  | T4 | tileset.test.js "lean name set exact" — same 11 names, tsx `tilecount="12" columns="4"` | REQ-003 |
  | T5 | canvas-map.test.js "committed tileset resolves a sprite rect per kind" — `current.sprite.sSize===32` | REQ-001 |
  | T6 | canvas-map.test.js **NEW** "default config enlarges cells to 64px" — `DEFAULT_MAP_CONFIG.tilePixels===64`; `canvasSize` cssWidth/backingWidth == columns×64 (×dpr) | REQ-002 |
  | T7 | canvas-map.test.js existing fallback/marker/aria unit tests (explicit 16/32/10px models) pass unchanged — regression guard that the pure model is geometry-agnostic | REQ-001/002 no-regression |
  | T8 | `cargo test --workspace` (oathstar-protocol) — `MapRoomSnapshot`/`MapSnapshot` carry no pixel field; unchanged | REQ-004 |
  | T9 | Deterministic regen — re-run generator, `git diff` clean (pinned `compress_level`) | REQ-003 |
  | T10 | Visual smoke — `bin/run` (or vite), map renders ~2× bigger, crisp, four kinds colored | REQ-001/002 (uncoverable in node: jsdom has no canvas-2D/Image — the #16 pure/seam split is why) |
- **Risks / decisions:**
  - **D-1 — enlarge via `tilePixels`, not CSS.** Raises backing resolution
    (true "device-pixel up"), node-testable through `canvasSize`, zero new
    plumbing. CSS-scaling would stretch a 32px backing, need new seam code, and
    isn't node-testable (jsdom no layout).
  - **D-2 — rename the asset dir** `16x16`→`32x32`. Ripples to 6 code/test refs
    (enumerated via grep: generator, client-app ×2, tileset.test ×5,
    canvas-map.test ×1) + 2 design docs + the asset README. A missed ref →
    tileset 404 → flat-color fallback (no crash, but no sprites); the
    committed-asset tests load the real path, so a miss fails loudly.
  - **R-1 — 2× integer upscale of FUTURE real 32px art.** Today (solid colors):
    pixel-identical. Real art: crisp nearest-neighbor 2×. If 1:1 native is ever
    wanted, regen at `TILE=64` (name-keyed — no code/map/wire change). Recorded
    for the art-direction revisit (Decision 054).
  - **R-2 — REQ-001 wording tightened from the spec's draft** ("source == dest")
    to "32px native source, crisp integer 2× to the 64px dest." The spec draft
    pre-assumed dest stayed ~32; the ticket's literal "TILE=32 regen" + "canvas
    size up" resolves to dest 64. Spec REQ-001/002 updated to match this design.
  - **R-3 — 64px is the chosen enlargement** (2× of today's 32). `mapRenderConfig`
    is the live knob if the owner wants a different size after the visual smoke.

## Phase 3 — Implement
- **Built** (production + assets only; tests = Phase 4, docs = Phase 5 per the
  implement-skill boundary):
  - `bin/generate_oathstar_tileset.py`: `TILE` 16→32, `NAME` →
    `oathstar-starter-32x32` (COLS/ROWS unchanged 4/3).
  - Regenerated `public/tilesets/oathstar-starter-32x32/` — 128×96 sheet, 32px
    tiles (verified: PIL `size (128, 96) RGBA`; json `tileSize 32`, coords
    doubled — stone_floor x=32, grass y=32, 11 tiles). **Deterministic:** re-run
    is byte-identical (png `e07c1e4e…`, json `0783d49e…`, preview `80f2fc82…`).
  - Removed old `public/tilesets/oathstar-starter-16x16/`; wrote the
    carried-forward `README.md` (128×96, 32px, new filenames).
  - `src/client/map.js`: `DEFAULT_MAP_CONFIG.tilePixels` 32→64 (+ comment: crisp
    integer 2× of the 32px source; `mapRenderConfig` live knob).
  - `src/client-app.js`: `TILESET_DIR` + `TILESET_JSON_URL` → `…-32x32`.
  - **`canvas-map.js` & `tileset.js` untouched** (pure + parameterized, as
    designed — they read `tileSize`/`tilePixels` from data).
  - Verified: `node --check` both edited JS files parse; no Rust change (no
    `cargo check` needed); grep confirms zero stray `16x16` refs in shipping code.
- **Deviations from design (+ reason):** none. Test geometry literals (manifest
  6–7) and design-doc updates (8–9) intentionally deferred — the implement skill
  forbids expanding tests here; `/pipeline:validate` updates + RUNS them, and
  `/pipeline:complete` does the doc/decision pass.

## Inspect (Phase 3.5)
- **Lenses run** (3 independent critics, parallel, over the Phase 3 diff):
  1. Rename-completeness / integration (Explore, read-only, whole-repo).
  2. Render-math correctness at `tilePixels=64` (general-purpose, ran `node`).
  3. Asset / contract integrity (general-purpose, independently decoded the PNG
     with PIL + ran the generator twice for determinism).
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | clean | Production rename complete; no dangling refs in vite/index.html/tauri/Rust/service-worker/gitignore; regenerated `.tsx` `<image source>` = `oathstar-starter-32x32.png` @ 128×96 | **Verified clean** (Critic 1) | none |
  | 2 | clean/info | Render math at 64px: glyphFontPx `max(9,round(64·0.4))=26`; marker `r=9`, `inset=11`, dots at (53,11)/(53,53) on-tile (far edge 62 ≤ 64); seam blits `drawImage(…,sSize 32,…,op.size 64)` = crisp 2× integer (`imageSmoothingEnabled=false`); nothing hardcodes 32; Hi-DPI backing bounded (beginner 2×4@dpr2 = 256×512); `.map-frame{overflow:auto}` scrolls | **Verified clean** (Critic 2) | none |
  | 3 | clean | Asset contract: PNG 128×96 RGBA; all 11 tiles uniform == declared `color`; spare (96,64) alpha 0; JSON coords `=(id%4)·32,(id//4)·32`; `.tsx` mirrors json (`tilecount="12" columns="4"`); palette byte-identical to #36's `FLAT_TILES`; deterministic (byte-identical across re-runs) | **Verified clean** (Critic 3) | none |
  | 4 | critical → **reclassified expected** | Test suite red: 8 failures — ENOENT on deleted `-16x16` path + stale `16` literals (`canvas-map.test.js:198,256`; `tileset.test.js:14–17,120,123,129,190,191`) | **NOT an implementation defect** — this is the Phase 4 (validate) test work the implement-skill defers; the design test plan already lists it. Critic 3 independently **confirmed the correct expected values** (grass `{0,32,32}`, stone_floor `{32,0,32}`, tileSize 32, `sSize` 32). Asset + test updates land in one commit; validate greens the gate before `/commit`. | Deferred to Phase 4 (no inspect fix) |
  | 5 | low | `src/app.js:6` `spritePixels: 32` | **Rejected** — legacy JS prototype, untested, not on the `client-app.js`→`client/*` render path, out of #37 scope | none |
  | 6 | info | New `oathstar-starter-32x32/` untracked; old 5 files `D` | Housekeeping — `/commit` does `git add -A`; the regen replaces the asset atomically in the commit | none at inspect |
- **Outcome:** no implementation defect found; the diff is correct and the render
  math + asset contract are verified by independent decode. No `failure-record`
  (nothing bit the delivered code). The reusable lesson — *regenerating a
  contracted asset turns the committed-asset tests red until updated in the same
  change; sequence them together* — is captured in the Phase 5 AAR (the existing
  pixel-based uniformity pin + committed-asset tests are the prevention already
  in place, and they did surface it via the critics).

## Phase 4 — Validate
- **Tests added / updated:**
  - `tests/tileset.test.js` (T1–T4): asset dir/filenames + `image` → `-32x32`;
    `tileSize` 16→32; committed-rect `sSize` 16→32; `tileRect("grass")` →
    `{0,32,32}`, `tileRect("stone_floor")` → `{32,0,32}`. `minimalTiles()` left
    synthetic-16 (validator generality). The uniformity/structure checks
    auto-adapted (they read `raw.tileSize`).
  - `tests/canvas-map.test.js` (T5–T6): committed path → `-32x32`; `current.sprite.sSize`
    16→32; **NEW** REQ-002 test pinning `DEFAULT_MAP_CONFIG.tilePixels===64`,
    `canvasSize` cssWidth/backingWidth == columns×64(×dpr), css dpr-independent,
    and `plan.tile===64` / every op `size===64`.
  - `tests/client.test.js`: two **default-derived** assertions 32→64 (L329 config
    passthrough of `DEFAULT_MAP_CONFIG`; L343 invalid-value fallback to default).
    **Caught by RUNNING the suite**, not by the inspect critics — they read only
    the two map test files; a third file also asserted the old default. §7/§15
    in action: writing tests isn't enough, running them is what found it.
- `cargo test --workspace`: **ok** — 296 + 30 + 25 + 23 + 20 + 16 passed, 0
  failed (+ doc-tests ok). No engine/protocol change → **REQ-004** confirmed.
- `node --test tests/*.test.js`: **64 passed, 0 failed.**
- `bin/gate.sh`: **GATE GREEN [full] — 17/17.** rustfmt, clippy(strict),
  cargo test, node --test, audit, deny, machete, gitleaks, shellcheck,
  no-suppressions, source-bans, lints-allowlist, doc-todos, tauri-shell;
  **gate:15 rust coverage 98.77%** (≥94), **gate:16 js coverage 87.65%** (≥75),
  **gate:17 mutation MSI 100.0%** (399 caught / 0 missed). Receipt written.
- **Determinism (T9):** generator re-run is byte-identical (pinned
  `compress_level`; sha verified in Implement + by inspect Critic 3). REQ-003 ✓.
- **Visual smoke (T10):** the testable contract is node-proven (REQ-002 canvas
  dimensions double; REQ-001 source `sSize===32`). The actual on-canvas pixels
  can't run in node (jsdom has no canvas-2D/Image — the load-bearing #16
  pure/seam split), and a full appearance check needs the live stack (vite :5173
  + oathstar-server :7878) with a session that has discovered rooms.
  Rename-completeness + `public/` asset-serving were exhaustively verified by
  inspect Critic 1; the asset exists at the new path and decodes correctly.
  **Owner-observable on next play:** the map renders ~2× larger and crisp.
  Not spun up here (disproportionate full-stack launch for a screenshot).
- **Pre-existing exclusions:** none — no pre-existing failures encountered; the
  only red was the planned test-geometry update (this phase's work).
- **Validate-phase catch (for the AAR):** changing a shared default
  (`DEFAULT_MAP_CONFIG.tilePixels`) requires grepping **all** test files for
  default-derived assertions, not just the obviously-related modules — captured
  for Phase 5.

## Phase 5 — Complete
- **Docs updated:** `docs/decisions.md` (new **Decision 055** — native 32px
  tiles, 64px cells); `docs/ui-design.md` (default 32→64, path → `-32x32`, an
  "Implemented (ticket #37)" note); `docs/map-system.md` (path + geometry →
  `-32x32`, 11 32px tiles on a 4×3 sheet — also corrected a stale `#36`
  "64 16px tiles" count). `CLAUDE.md` unchanged (no convention shift).
- **Forge capture (aar/failures/rules/decisions):**
  - `aar-submit` AAR `bd639a22` → completed, effectiveness 4, 7 verdicts written,
    3 novel findings; surfaced_used = arch-decision `62987d6a`, failure
    `35c0e883` (tileset-contract), lesson `00006cff`.
  - `failure-record` **BF-map-shared-default-tests-001** (`524538ee`) — the
    default-derived assertion in `client.test.js` the critics didn't read.
  - `prevention-rule-record` **PR-claude-shared-default-tests-001** (`9b230354`)
    — grep ALL test files on a shared-default change.
  - `architecture-decision-record` **AD-claude-map-tile-scale-001** (`df14f58f`)
    — native 32px source / 64px on-page cell; client+asset only.
- **Ticket closed:** forge #37 → `done`; local doc moved
  `tickets/open/` → `tickets/closed/`.
- **Archived:** spec + notes moved `pipeline/active/` → `pipeline/completed/`.
