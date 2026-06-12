# WORK-tileset-map-rendering-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #32 — pull the generated starter tileset into the #16
  canvas map renderer: cells draw as sprite tiles from the sheet instead of
  flat fills. Direct owner request ("what id like to build is an ability to
  pull in tsx into the map"); no end-to-end waiver this time — plan and
  design gates get human confirmation.
- **Intake source:** none — direct request; the assets exist as untracked
  strays from a Codex side-session (Jun 7–9).
- **Classification / tier:** Work pipeline, one slice — a pure tileset
  module, a draw-plan extension, seam blitting, asset placement, tests.
- **Base verified:** `main` @ `ff41c3b` (#31 merged + pushed); no active
  pipeline; forge up (reconnected earlier this session); no bulletins.
- **Anchors verified at plan:**
  - `src/client/canvas-map.js` — pure module: `MAP_PALETTE`, `canvasSize`
    (Hi-DPI backing math), `cellKind` (empty/discovered/current/blocked),
    `toDrawPlan` (ops: x/y/size/kind/fill/stroke/textColor/glyph/here),
    `mapAriaLabel`. No DOM/canvas access — the #16 split.
  - `src/client-app.js` — `mapRenderConfig = { ...DEFAULT_MAP_CONFIG }`
    (line 37, 32px tiles), `renderMap` → `toMapModel` → `drawMapCanvas`
    (seam at ~449: sizing, transform, fillRect/strokeRect/fillText).
  - Tileset assets (untracked): `assets/tilesets/oathstar-starter-16x16.png`
    (128×128, 8×8 grid of 16px tiles), `.tsx` (Tiled XML, per-tile
    name/tags/collision properties), `.json` twin `{name, tileSize: 16,
    columns: 8, rows: 8, image, tiles[{id, name, x, y, tags, collision}]}`,
    preview PNG, README (tile ID ranges: 0–7 base terrain, walls, water,
    props, glyph tiles…), generator `bin/generate_oathstar_tileset.py`
    (deterministic, 18 KB Python).
  - vite serves `public/` verbatim into `dist/` — `public/vendor/datastar/`
    is the committed-asset precedent; `assets/` is NOT served today.
  - node/jsdom has no canvas-2D context and no `Image` — #16's validated
    constraint; image loading + `drawImage` must stay in the seam, resolution
    logic in pure modules (REQ-001's testability hangs on this).
  - JS coverage floor 75% (gate:16): new pure modules add covered lines;
    seam glue stays uncounted (the #16 pattern — coverage rose then).
- **Forge recall (at /work pre-flight):** #16's complete design notes
  surfaced (pure/seam split rationale, palette, test plan — the direct
  architectural substrate); docs/map-system.md + ui-design.md both reserve
  "sprite tiles" as the next step; no contrary prevention rules. Recent
  rules surfaced but peripheral: forge-up-before-session (heeded this
  session), broadcast-receiver lag (server tests — N/A here).
- **Ticket:** forge `4e0b8ebd-508c-469b-9d70-13b27b584087` (#32), local doc
  `docs/planning/tickets/open/TICKET-32-tileset-v1-map-cells-draw-sprite-tiles.md`.
- **AAR opened:** `f0f3ad07-b3fc-4983-b693-d98873ebb776`.
- **EARS requirements reviewed:** REQ-001..008 (verbatim in the spec):
  pure kind→tile resolution, sheet blitting over flat fills, never-blank
  fallback, validated metadata, crisp scaling, glyph/a11y preservation,
  served+built assets, gate preservation.

- **Owner note at design entry (2026-06-11):** the longer direction is
  per-tile `description` metadata in the .tsx/.json and authoring regions/
  sub-regions as tilesets — captured as
  `docs/planning/intake/INTAKE-tileset-region-authoring-per-tile-metadata.md`
  (NOT this ticket; #32 stays the render-with-current-rooms slice — "lets
  see if we can get it to render with our rooms we have now"). Design
  implication for #32: resolve tiles **by name** (Q2's named-tile table)
  and keep the metadata module's validated shape open to additive fields,
  so the description/region extension lands without reshaping.

### Open design questions (for Phase 2 — Planner does NOT decide these)
1. **Asset placement.** Move `assets/tilesets/` under
   `public/tilesets/oathstar-starter-16x16/` (vendor precedent: served
   verbatim, one canonical home) vs keep `assets/` canonical + copy the
   served pair (PNG+JSON) into `public/` (two copies to keep in sync) vs
   vite `publicDir`/alias config. Where do the .tsx/preview/README/generator
   live? Design picks ONE canonical layout + records it.
2. **Kind → tile mapping.** Which of the 64 tiles for empty/discovered/
   current/blocked? Candidates per the README ranges (base terrain 0–7,
   walls, props, glyph tiles). Current-room: a distinct tile, a tinted
   overlay on the base tile, or keep the brass fill? Blocked: wall tile?
   Pinned as a named-tile table (resolve by `name`, not raw id — the
   generator's names are the stable contract).
3. **Metadata validation depth (REQ-004).** Minimal structural checks
   (tileSize/columns/image/tiles array, required names present, rects in
   sheet bounds?) and the typed invalid result shape.
4. **Image lifecycle.** Preload at startup vs lazy on first renderMap;
   redraw trigger on `load` event; failure (404/decode) → permanent
   fallback for the session? Where does the loaded-image handle live
   (module-scope cache in the seam)?
5. **Scaling crispness (REQ-005).** `ctx.imageSmoothingEnabled = false` per
   draw + CSS `image-rendering: pixelated` on `.map-canvas`? Both? At
   16→32 integer scale either is clean; non-integer (accessibility sizes)
   needs the pin.
6. **Draw-plan shape.** Extend each op with a `tileRect` (sx/sy/sSize) +
   keep fill/stroke for fallback? Or a parallel `tiles` array? One plan
   serving both render modes (REQ-003's fallback wants the palette fields
   retained).
7. **Glyph under tiles (REQ-006).** Glyph stays text-drawn on top of the
   tile (contrast color per kind?) vs the sheet's glyph tiles. README has
   glyph tiles — but title-mode labels still need text. Design pins.
8. **Generator hygiene.** `bin/generate_oathstar_tileset.py` is committed
   as-is (deterministic, documented) — does the gate need anything for a
   Python file in `bin/` (shellcheck is shell-only; gitleaks/source-bans
   scan text fine)? Verify gate:9's file glob at design; no new gate work
   in scope.

## Phase 2 — Design

- **Recall (12 surfacings):** AD-oathstar-canvas-map-renderer-002 (the #16
  substrate: pure draw-plan + thin seam, this design extends it);
  **PR-oathstar-render-plan-test-002** (plan fields must be asserted as the
  seam consumes them — no dead plan fields; shapes T6/T7);
  PR-claude-minimap-zplane-001 (z-plane handling — untouched, regression via
  existing tests); PR-oathstar-html-escape-001 +
  PR-claude-renderer-tests-through-the-normalizer-001 (peripheral: no server
  HTML, no new event kinds).

### Approach / architecture (settles the 8 Phase-1 questions)

1. **Asset placement (Q1): ONE canonical home —**
   `public/tilesets/oathstar-starter-16x16/` (PNG + JSON + .tsx + preview +
   README move there; vite serves `public/` verbatim → REQ-007 is free; the
   `public/vendor/datastar` precedent). The generator's `OUT_DIR` constant
   (`bin/generate_oathstar_tileset.py:19`) retargets to the same directory —
   ONE line, so regeneration writes where the app reads (this is plumbing,
   not the out-of-scope "re-authoring"); everything else in the generator
   untouched. `assets/` directory disappears (nothing else lives there).
   Gate exposure of the Python file verified at design: shellcheck globs
   `*.sh` only; source-bans greps `crates/*/src` + `src-tauri/src`;
   gitleaks/doc-todos find nothing in it — no gate work needed.
2. **Kind → tile mapping (Q2): a frozen NAMED-tile table** in the new pure
   module — `KIND_TILE_NAMES = { empty: "shadow_void", discovered:
   "stone_floor", blocked: "wall_face", current: "spawn_marker" }` (names
   are the generator's stable contract; the intake's description/region
   extension lands additively). One blit per cell; the existing per-kind
   stroke ring and glyph text stay ON TOP of the tile (REQ-006), so
   "current" keeps its brass ring + marker tile.
3. **Validation depth (Q3):** `validateTileset(raw)` checks: object;
   `tileSize`/`columns`/`rows` finite > 0; `image` non-empty string;
   `tiles` an array; every tile has non-empty `name` + finite `x`,`y`;
   every rect within the `columns*tileSize × rows*tileSize` sheet; all four
   `KIND_TILE_NAMES` present. Returns a **typed result, never throws**
   (REQ-004): `{ ok: true, tileset }` (tileset = normalized input + a
   `byName` lookup object) or `{ ok: false, reason }` (short string naming
   the failed check).
4. **Image lifecycle (Q4): seam-owned, module-scope state** in
   `client-app.js`: `loadTileset()` runs once at startup — `fetch` the JSON
   → `validateTileset` (pure) → on ok, `new Image()` with `src` = the PNG
   URL; `load` → cache the image + redraw from the seam's cached last map
   model; any failure (fetch, invalid metadata, image `error`) → permanent
   flat-color fallback for the session + ONE `console.warn` (no retry loop,
   no errors thrown — REQ-003). The seam caches `lastMapModel` at each
   `renderMap` so the load-event redraw needs no snapshot replumbing.
5. **Crisp scaling (Q5): both pins** — `ctx.imageSmoothingEnabled = false`
   set each draw (after `setTransform`), and `image-rendering: pixelated`
   on `.map-canvas` in styles.css. 16→32 is integer ×2; dpr is already
   handled by the #16 backing-store scale.
6. **Draw-plan shape (Q6): one plan serves both render modes** (REQ-003).
   `toDrawPlan(model, tileset = null)` — each op KEEPS
   fill/stroke/textColor/glyph/here and GAINS
   `tile: { sx, sy, sSize } | null` (resolved kind → name → rect via the
   tileset module; null when no/invalid tileset). Seam draw order per op:
   `op.tile && image-ready` ? `drawImage(img, sx, sy, sSize, sSize, x, y,
   size, size)` : `fillRect` — then the existing strokeRect + glyph text
   unconditionally. Per PR-oathstar-render-plan-test-002 the tests assert
   exactly the fields this seam reads (`tile` + retained fallback fields).
7. **Glyph/text (Q7):** unchanged — glyph drawn over the tile with the
   kind's existing palette `textColor`; aria-label logic untouched
   (REQ-006). The sheet's glyph-ish tiles (spawn/exit markers) are kind
   tiles, not text replacements.
8. **Generator hygiene (Q8):** covered under Q1 — OUT_DIR retarget only;
   gate scan exposure verified none.

New pure module **`src/client/tileset.js`** (DOM-free, node-tested):
`KIND_TILE_NAMES`, `validateTileset(raw)`, `tileRect(tileset, name) →
{sx, sy, sSize} | null`, `kindTileRects(tileset) → {empty, discovered,
current, blocked}`. `canvas-map.js` imports `kindTileRects` for the plan;
`client-app.js` imports `validateTileset` for the loader. URLs
(`/tilesets/oathstar-starter-16x16/oathstar-starter-16x16.{json,png}`) are
seam constants (the pure modules never fetch).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `public/tilesets/oathstar-starter-16x16/*` | tileset assets MOVED here from `assets/tilesets/` (png, json, tsx, preview png, README — committed) |
| 2 | `bin/generate_oathstar_tileset.py` | `OUT_DIR` constant → `public/tilesets/oathstar-starter-16x16` (one line; committed) |
| 3 | `src/client/tileset.js` | **new** pure module: `KIND_TILE_NAMES`, `validateTileset`, `tileRect`, `kindTileRects` |
| 4 | `src/client/canvas-map.js` | `toDrawPlan(model, tileset = null)`: ops gain `tile` rect (kind-resolved), all existing fields retained |
| 5 | `src/client-app.js` | seam: tileset loader (fetch+validate+Image, module-scope cache, redraw-on-load via cached `lastMapModel`, warn-once fallback); draw branch `tile→drawImage` else `fillRect`; `imageSmoothingEnabled = false` |
| 6 | `styles.css` | `.map-canvas { image-rendering: pixelated; }` |
| 7 | `tests/tileset.test.js` | **new** node --test (T1–T4) |
| 8 | `tests/canvas-map.test.js` | extend: plan-with-tileset / plan-without (T5–T8) |
| 9 | `docs/map-system.md`, `docs/ui-design.md`, `decisions.md` | (Phase 5) sprite-tiles implemented note + decision |

No changes: `crates/**`, `src-tauri/**`, `modules/**`, `index.html` (canvas
exists), `oathstar-protocol`/server JSON (REQ-008).

### Regression Test Plan
All JS `node --test`; the real committed JSON is read from
`public/tilesets/…/oathstar-starter-16x16.json` in tests (keeps the asset
honest — drift fails the suite). Per PR-oathstar-render-plan-test-002, plan
assertions target the fields the seam consumes (`tile`, `fill`, `stroke`,
`textColor`, `glyph`).

| # | Test | Proves Requirement |
|---|---|---|
| T1 | `validateTileset(realCommittedJson)` → ok; tileset carries tileSize 16, columns/rows 8, byName resolves all 64 names | REQ-001/004/007 |
| T2 | `validateTileset` rejects each malformed shape (non-object, tileSize 0/-1/NaN/missing, empty image, tiles non-array, tile missing name, non-finite x/y, out-of-sheet rect, a missing required kind name) → `{ok:false, reason}` naming the check; **never throws** | REQ-004 |
| T3 | `tileRect` by name from the real JSON → exact `{sx,sy,sSize}` (grass→0,0,16; stone_floor→64,0,16; wall_face per its authored x/y); unknown name → null | REQ-001 |
| T4 | `KIND_TILE_NAMES` covers exactly the four cell kinds and each name exists in the real committed JSON | REQ-001/002 |
| T5 | `toDrawPlan(model)` (no tileset) — byte-identical ops to today plus `tile: null`: fill/stroke/textColor/glyph retained per kind | REQ-003 |
| T6 | `toDrawPlan(model, tileset)` — each op's `tile` rect equals `tileRect(tileset, KIND_TILE_NAMES[kind])` for its kind across all four kinds; fallback fields STILL present and palette-correct | REQ-002/003 |
| T7 | plan with tileset: glyph + textColor + here unchanged vs no-tileset plan; `glyphFontPx` unchanged; `mapAriaLabel` unchanged | REQ-006 |
| T8 | existing `canvas-map.test.js` + `client.test.js` suites stay green (z-plane, DPR sizing, cellKind, immutability) | REQ-006/008 |
| T9 | `npm run build` → `dist/tilesets/oathstar-starter-16x16/`(png+json) exists | REQ-007 |
| T10 | browser smoke: tiles visibly replace flat fills; kill the JSON URL → flat-color map, no console errors; throttled load shows flat→tiles swap; dpr zoom stays crisp | REQ-002/003/005 |
| T11 | `bin/gate.sh` full suite green; no `crates/**` diff | REQ-008 |

Genuinely uncoverable by unit test (documented §7): the `drawImage` call,
`Image` loading, and `imageSmoothingEnabled` live in the seam — node/jsdom
has no canvas-2D context or Image (the #16 validated constraint). Covered
by T10 smoke at validate; everything decision-bearing is pure and tested.

### Risks / decisions
- **D1 (Q1) one canonical asset home under `public/`** + generator OUT_DIR
  retarget (one line — keeps regenerate→serve coherent). Reversible by
  moving the directory back.
- **D2 (Q2) named-tile table** — resolution by `name` (stable contract,
  additive-metadata-proof per the intake), never by raw id.
- **D3 (Q4/Q3) loader = fetch→validate→Image with warn-once permanent
  fallback**; typed validate result, no throws on any input (the §14
  no-panics-on-input-paths convention, JS edition).
- **D4 (Q6) single plan, both modes** — `tile` + retained palette fields;
  the seam picks per-op. PR-oathstar-render-plan-test-002 satisfied: T6
  asserts the exact fields the seam reads, and the fallback fields stay
  live (read whenever image isn't ready), not dead plan data.
- **R1 — marker-tile legibility** ("current" = spawn_marker + brass ring +
  glyph may be busy at 32px): smoke-judged at validate; the named table
  makes a swap a one-string change.
- **R2 — coverage floor:** new pure module + extended plan are tested
  (covered lines rise); seam glue grows but stays uncounted, as in #16.
  Confirm at validate (gate:16).
- **R3 — test-reads-real-asset coupling:** tests read the committed JSON
  from `public/`; regenerating with different names/geometry fails T1/T3/T4
  loudly — intended (the asset is now a contract).

## Phase 3 — Implement
- Built (to the manifest):
  - Assets moved: `assets/tilesets/*` → `public/tilesets/oathstar-starter-16x16/`
    (png, json, tsx, preview, README); `assets/` itself stays (it holds the
    tracked `oathstar-sigil.svg` — only the tilesets subdir moved).
  - `bin/generate_oathstar_tileset.py`: `OUT_DIR` → `public/tilesets/<NAME>`
    (one constant + why-comment). README needed no path fix (it references
    only the generator path, which didn't move).
  - `src/client/tileset.js` (new, pure): `KIND_TILE_NAMES` frozen table,
    `validateTileset` (typed `{ok,tileset}|{ok:false,reason}`, never throws,
    builds `byName`, checks bounds + required names), `tileRect`,
    `kindTileRects`.
  - `src/client/canvas-map.js`: `toDrawPlan(model, tileset = null)` — ops
    gain `tile` (kind-resolved source rect | null); all palette fields kept.
  - `src/client-app.js`: `validateTileset` import; `TILESET_DIR`/
    `TILESET_JSON_URL` consts (page-origin static paths, deliberately NOT
    `API_BASE` — vite/`public/` serves them, not the game server);
    module-scope `tilesetData`/`tilesetImage`/`lastMapModel`;
    `loadTileset()` (fetch→validate→Image; publishes data+image together in
    `onload` then repaints `lastMapModel`; warn-once fallback on HTTP
    error/fetch throw/invalid metadata/image error); called from `boot()`;
    `renderMap` caches `lastMapModel`; `drawMapCanvas` sets
    `imageSmoothingEnabled = false` and per-op draws `drawImage(sheet,
    sx,sy,sSize,sSize → x,y,size,size)` when `op.tile && tilesetImage`,
    else the flat `fillRect`; stroke + glyph unchanged on top.
  - `styles.css`: `.map-canvas` gains `image-rendering: pixelated`.
- Deviations from design (+ reason): none functional. One naming note:
  plan-level `tile` (tile edge px, pre-existing) vs new op-level `tile`
  (source rect) coexist; kept the design's field name — the seam reads
  `op.tile`, `plan.tile` is plan-level only.
- `node --check` clean on all three modules; existing
  `canvas-map.test.js` + `client.test.js` suites green (25 pass) untouched.

## Inspect (Phase 3.5)
- Lenses run: 2 critics over the diff + new files — (1) correctness +
  lifecycle (validator fuzz, async races, drawImage/context-state order,
  proxy routing, committed-asset contract), (2) plan-integrity per
  PR-oathstar-render-plan-test-002 + reuse/conventions/coverage.
- Findings:
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | medium | `byName` plain `{}`: a tile named `__proto__` re-prototypes the lookup; required-names check passes on INHERITED props — structurally empty tileset validates ok, tileRect returns sx/sy undefined → drawImage silently draws nothing (tileset.js:50,64) | REAL (both critics, executed probes) | `byName = Object.create(null)` + why-comment; probe now rejects ("required tile 'shadow_void' is missing"). Forge: BF-claude-proto-key-poisons-plain-object-lookup-001 + PR-claude-null-proto-untrusted-key-lookup-001 |
  | 2 | medium | `Number.isFinite` admits fractional geometry (x:3.5 ok) — subpixel drawImage source rects smear across atlas boundaries, defeating the crispness this diff installs (tileset.js:33-58) | REAL (probe) | `Number.isInteger` for tileSize/columns/rows/x/y + reason strings; the generator emits integers only, validator is the lone guard. Forge: BF-claude-finite-check-admits-subpixel-rects-001 |
  | 3 | high | Dead op fields `kind` + `here` — nothing in src/ reads them (pre-#32 dead too); exist only for test assertions — the exact PR-oathstar-render-plan-test-002 ban (canvas-map.js:105,110) | REAL (grep-verified, incl. `git show HEAD` seam) | Removed both from ops; re-pointed the #16 test assertions at the palette fields the seam draws (fill/textColor/stroke identity per kind); classification stays covered by direct `cellKind` tests; JSDoc notes the ops-carry-only-what-the-seam-draws rule |
  | 4 | low | `plan.tile` (number) vs `op.tile` (rect) — same name, two meanings one level apart (canvas-map.js:111,117) | REAL (lead had flagged it in Phase 3 notes) | Renamed the op field to `sprite` (plan, seam, JSDoc) |
  | 5 | low | gate:16 won't force the tileset tests: tileset.js sits at 45% lines / 0% functions while the all-files floor passes — validate's T1–T8 land on discipline, not gate pressure | NOTED (informational) | No source change; validate writes T1–T8 and confirms tileset.js function coverage moves off 0% |
- Verified clean (with evidence): validateTileset never throws (19-shape
  fuzz); committed JSON validates with all 64 rects in bounds + the four
  kind names; toDrawPlan preserves every pre-#32 op field byte-identically
  (diffed against `git show HEAD` module) and mutates nothing;
  `await response.json()` inside the try (invalid-JSON rejection caught);
  image-onload-before-first-render guarded; canvas.width state reset
  precedes getContext/setTransform/smoothing-off (order correct);
  `/tilesets` never matches the vite proxy keys; pure modules DOM-free;
  no existing helper duplicated; suites green after fixes (25 pass).

## Phase 4 — Validate
- Tests added (7 new JS tests; every plan row landed):
  - `tests/tileset.test.js` (new, T1–T4): committed-asset validation with
    ALL 64 names resolving to authored rects; 26-case malformed-shape table
    (typed reasons, never throws — including the inspect-found `__proto__`
    poisoning and fractional-position probes); name→rect resolution + null
    for unknown/prototype names; KIND_TILE_NAMES × committed-sheet
    cross-check through `kindTileRects`.
  - `tests/canvas-map.test.js` (extended, T5–T7): plan without tileset
    keeps the flat-color contract (`sprite: null`, palette fields
    retained); plan with the real tileset resolves a sprite rect per kind
    (16px source, configured dest) with fallback fields intact; tiled vs
    flat plans byte-identical apart from `sprite` + aria unchanged
    (REQ-006). Plus the inspect re-point: assertions now target the
    palette fields the seam draws, not the removed `kind`/`here`.
- `node --test tests/*.test.js`: **55 pass, 0 fail** (was 48).
- `cargo test --workspace`: ALL GREEN — 260 core, 27 server, all other
  crates; 0 failed (no Rust touched, unchanged).
- `npm run build`: OK — `dist/tilesets/oathstar-starter-16x16/` ships the
  png + json (+tsx/preview/README) verbatim (REQ-007).
- Coverage spot-check (inspect ledger #5 resolved): `tileset.js`
  100/100/100 (was 45% lines / 0% fn), `canvas-map.js` 100 lines,
  all-files 87.12% ≥ the 75 floor.
- `bin/gate.sh --fast`: **GATE GREEN [fast]** — 14/14. FULL gate at
  `/commit` per process bounds.
- Browser smoke (T10): Chrome-extension automation unavailable this
  session (extension not connected; two attempts) — substituted a
  shell-level smoke: vite serves the JSON (200, correct body) and the
  sheet PNG (200, image/png, real 128×128 RGBA); index + client-app.js +
  tileset.js all 200 over the dev server; `/state` proxies fine. Fallback
  arms verified at the logic level (inspect critic + unit tests), with one
  dev-specific discovery: a missing JSON in dev returns 200+HTML (vite SPA
  fallback) so the failure is `response.json()` throwing — inside the try,
  caught (prod 404 takes the `!response.ok` arm; both covered). **The
  visual eyeball (tiles on screen) is the one open item** — dev servers
  left running and Chrome opened at `http://127.0.0.1:5173/` for the
  owner; the README preview PNG shows the expected art. Documented per §7
  (canvas-2D/Image unreachable under node; in-session browser automation
  unavailable).
- Pre-existing exclusions: none.

## Phase 5 — Complete
- Docs updated: `docs/map-system.md` + `docs/ui-design.md` ("Implemented
  (ticket #32 — sprite tiles)" blocks, intake pointer for the region/
  description direction); `docs/decisions.md` **Decision 050** (committed
  name-keyed tileset, permanent fallback, strict-where-corruption-hurts
  validation, crispness pinned twice; revisit triggers incl. region
  authoring + multi-sheet).
- Forge capture: `aar-submit` `f0f3ad07…` → completed, effectiveness 4,
  4 surfacings confirmed used, 2 rules materialized
  (PR-oathstar-render-plan-test-002 — which caught real dead fields again —
  and the new PR-claude-null-proto-untrusted-key-lookup-001). Failures +
  the prevention rule were recorded AT INSPECT (BF-claude-proto-key-…,
  BF-claude-finite-check-…). `architecture-decision-record`:
  AD-claude-tileset-name-keyed-fallback-render-001.
- Ticket closed: forge `4e0b8ebd…` (#32) → **done**; local doc →
  `docs/planning/tickets/closed/` (status/closed/pipeline_spec updated).
- Archived: spec+notes pair → `docs/planning/pipeline/completed/`.
- Open item handed to `/commit` + owner: the visual eyeball (dev servers
  left running; Chrome open at `http://127.0.0.1:5173/`).
