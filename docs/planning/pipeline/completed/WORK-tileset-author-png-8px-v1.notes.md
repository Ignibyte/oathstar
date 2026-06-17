# WORK-tileset-author-png-8px-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Owner pivot — squash to an 8px-native tile grid because the real
  art is 8x8 PNG sheets (not the ElvGames 16x16 we first tried). Retire the
  generated-tileset/blank-colors era, make the map model accept the sheet's
  declared size, and swap the client onto an author PNG-sheet loader. Owner
  auto-approved running the full pipeline autonomously.
- **Intake source:** `INTAKE-tileset-region-authoring-per-tile-metadata.md` —
  its Tiled-importer (#39) + blank-colors (Decision 054) program is what this
  pivot supersedes. The authoring surface moves Tiled -> first-party studio
  editor (already begun in #43–45). Intake to be marked partially superseded at
  Complete.
- **Classification / tier:** work pipeline, one coherent shippable slice. The
  asset contract flips from generated-flat to author-PNG-at-declared-size, and
  the model must accept that size for the change to be end-to-end — so
  model + client + asset-retirement ship together (splitting would leave an
  awkward half-state). Real art + the painting backend are explicitly deferred.
- **Forge recall (lessons/failures surfaced):**
  - Pre-flight green: forge up, no bulletins, gate + toolchain present, no
    active pipeline.
  - Failure node `524538ee` ranked on the "delete committed / supersede
    decision" theme — heeded via the locked decision that deletion is paired
    with reference + test cleanup so the gate stays green. Full text to be
    pulled via `knowledge-context` at Design (needs the AAR id).
  - The map/tileset architecture-decisions (035/050/054 family) ranked top on
    the model query — the ones this slice supersedes.
- **Ticket:** #46 `9117db55-c8d1-4e90-bb83-3fc7738da864` (feature).
- **EARS requirements reviewed:** REQ-001..007 in the spec. Testable arms:
  model accept/reject (Rust), client sheet-load + fallback (node), gate green,
  asset-absence. Doc/record arms: decisions superseded, #39 closed.
- **AAR id:** e01e1caf-e9b0-4ccf-a710-32416f2131d7

## Phase 2 — Design

### Approach / architecture
- **Rust (`oathstar-content`) — the only behavioural change.** Replace the
  singular `SUPPORTED_TILE_SIZE: u32 = 16` with a set
  `SUPPORTED_TILE_SIZES: [u32; 3] = [8, 16, 32]`. `check()` rejects with
  `!SUPPORTED_TILE_SIZES.contains(&self.tile_size)`; the typed
  `UnsupportedTileSize { found }` variant keeps its shape; `Display` renders
  `expected one of {SUPPORTED_TILE_SIZES:?}` (derives from the const, so a
  mutated const fails the message test too). Decision 025 (1 cell = 1 room,
  cardinal movement) and render scale are untouched — `tile_size` is the
  source/art unit only. No `oathstar-studio` change: its validate endpoint is
  model-driven (`materialize` → `check`).
- **JS client — retire + relabel, no renderer logic change.**
  `validateTileset()` already accepts any positive-integer `tileSize` (8px
  validates today) and `toDrawPlan(model, null)` already is the flat-color
  fallback (tested). So the work is: retire the committed generated sheet,
  repoint the browser seam at a single author-sheet URL (default `null` →
  fallback), and relabel the stale "generator / Tiled .tsx twin" comments in
  `tileset.js`. The name-keyed contract (Decision 050 mechanism) survives — the
  author sheet supplies the `name -> cell` map instead of a generated twin.
- **Contract + fixture.** A documented author tile-sheet contract
  (`docs/tileset-contract.md`) + a JSON-only 8px fixture under `tests/fixtures/`
  that the JS tests load in place of the deleted starter. JSON-only is honest:
  the one test that decoded a PNG was the Decision-054 blank-colors pin, which
  is being removed.
- **Records.** `docs/decisions.md` gains Decision 059 (author PNG sheets are
  the contract; flat color is the fallback skin; supersedes 035/050/054; 025
  unchanged) and annotates 035/050/054 as superseded; the forge AD +
  `ticket-close #39` land at Complete.
- **Deletion is paired with a ref sweep** (heeds forge failure `524538ee`):
  client-app, tests, docs, and build config get swept so nothing dangles and
  the gate stays green.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-content/src/map_document.rs` | `SUPPORTED_TILE_SIZE=16` → `SUPPORTED_TILE_SIZES:[u32;3]=[8,16,32]`; `check()` → `!…contains(&self.tile_size)`; `Display` → `expected one of {SUPPORTED_TILE_SIZES:?}`; field+variant doc comments. Tests: `supported_tile_size_is_sixteen`→assert the set; `tile_size_sixteen_validates`→8/16/32 all validate; `wrong_tile_size_is_refused` 32→7; `validate_rejects_an_invalid_document` 8→7; `error_messages_render_each_variant` found 32→7 + new string. |
| 2 | `src/client/tileset.js` | Doc-comment relabel only ("generated JSON / .tsx twin" → "author tile-sheet descriptor"; KIND_TILE_NAMES "generator's contract" → "author sheet's contract"). No logic change. |
| 3 | `src/client-app.js` | Replace `TILESET_DIR`/`TILESET_JSON_URL` (committed starter path, L45-46) with one `AUTHOR_TILESET_URL` defaulting to `null`; guard the load so null/absent → `tileset=null` → flat fallback, never throw. |
| 4 | `src/client/canvas-map.js` | Comment touch-up only (fallback already implemented + tested); no logic change. |
| 5 | `docs/tileset-contract.md` (NEW) | The author tile-sheet contract: descriptor shape, required kind names, fallback behaviour, sample-fixture reference. |
| 6 | `tests/fixtures/tilesets/sample-8px.json` (NEW) | Valid 8px author-sheet descriptor (4 kind tiles) — the committed fixture the JS tests load. |
| 7 | `tests/tileset.test.js` | Drop committed-file paths + the PNG decoder; repoint T1/T3/T4 to the 8px fixture; keep T2 (inline); DELETE T5 (blank-colors PNG pin — Decision 054 superseded) + T6 (.tsx twin — generator gone). |
| 8 | `tests/canvas-map.test.js` | `committedTileset()` → load the 8px fixture; T6 `sSize` 32→8 (dest size stays `tilePixels`); rest unchanged. |
| 9 | `tests/elvgames-tileset.test.js` | DELETE (untracked ElvGames experiment). |
| 10 | `bin/generate_oathstar_tileset.py` | DELETE (committed generator). |
| 11 | `bin/import_elvgames_tilesets.py` | DELETE (untracked). |
| 12 | `public/tilesets/oathstar-starter-32x32/` | DELETE (committed png/json/tsx/preview/README). |
| 13 | `public/tilesets/oathstar-elvgames-fantasy-16x16/` | DELETE (untracked). |
| 14 | `docs/map-system.md` | Drop committed-sheet/generator language (L107-109); point at the contract; note 8/16/32 incl. 8px-native. |
| 15 | `docs/ui-design.md` | Drop committed starter-sheet/generator refs (L229-238); point at the contract. |
| 16 | `docs/decisions.md` | Add Decision 059 (supersede 035/050/054; 025 unchanged); annotate 035/050/054 superseded. |
| 17 | (sweep) | Grep `vite.config*`, `index.html`, service worker, `src-tauri/tauri.conf.json`, `.gitignore`, `crates/oathstar-studio` for refs to the deleted tilesets/generator; fix any. |

### Regression Test Plan
| # | Test | Proves Requirement |
|---|---|---|
| RT-1 | Rust: `valid_doc` with `tile_size ∈ {8,16,32}` each `validate()==Ok` | REQ-001 |
| RT-2 | Rust: `SUPPORTED_TILE_SIZES == [8,16,32]` (pin the set) | REQ-001 |
| RT-3 | Rust: `tile_size` 7 and 0 → `UnsupportedTileSize{found}` | REQ-002 |
| RT-4 | Rust: Display of `UnsupportedTileSize{7}` == "tile size 7 is unsupported (expected one of [8, 16, 32])" (mutation pin) | REQ-002 |
| RT-5 | Rust: `validate_rejects_an_invalid_document` with `tile_size=7` → `is_err` | REQ-002 |
| RT-6 | JS `tileset.test` T1': 8px fixture validates; `tileSize==8`; names resolve | REQ-004 |
| RT-7 | JS `tileset.test` T2 (kept): malformed shapes → typed refusal, never throws | REQ-004 |
| RT-8 | JS `tileset.test` T3'/T4': `tileRect`/`kindTileRects` resolve on the fixture; unknown→null | REQ-004 |
| RT-9 | JS `canvas-map` T5 (kept): `toDrawPlan(model,null)` → `sprite:null` + flat fill retained | REQ-005 |
| RT-10 | JS `canvas-map` T6': `toDrawPlan(model,fixture)` → `sprite.sSize==8`, dest `size==tilePixels` (decoupling) | REQ-004 |
| RT-11 | Path-absence: no `public/tilesets/*` generated dirs, no `bin/*_tileset*.py`; gate green | REQ-003 |
| RT-12 | `bin/gate.sh` FULL green (cov 94/75, mutation 100% MSI) after the ref sweep | REQ-006 |
| RT-13 | Doc/record: decisions.md Decision 059 supersedes 035/050/054; forge AD recorded; #39 closed | REQ-007 |

Genuinely uncoverable: the browser seam (`client-app.js` `drawMapCanvas`/`Image` load + the `AUTHOR_TILESET_URL==null` guard) stays smoke-/review-verified, not `node --test`-covered (DOM). REQ-003 asset-absence is a review + path check + the gate.

### Risks / decisions
- **Allowlist `{8,16,32}` over a range/power-of-two:** matches the owner's
  stated sizes, keeps the typed error legible, trivially mutation-testable.
  Reversible (widen later).
- **JSON-only fixture (no PNG):** the only PNG-decoding test was the 054
  blank-colors pin being removed; node tests need only the descriptor. Keeps
  "real art deferred" honest.
- **`AUTHOR_TILESET_URL` defaults to `null`:** the app renders the flat-color
  fallback until the owner drops a sheet — visually ~unchanged (the old
  committed sheet was itself flat colors) and the intended missing-art skin.
- **Delete (not repoint) the 054 blank-colors PNG pin + the `.tsx` twin pin:**
  they test the generated-flat contract being superseded; end-to-end
  validate/resolve coverage moves to the fixture.
- **Render scale + Decision 025 untouched:** 8px is the source/art unit only; an
  8px sprite upscales (nearest-neighbour) into the 64px cell exactly as the
  32px sprite did.
- **Heed failure `524538ee`:** every deletion is paired with the ref sweep
  (row 17) so nothing dangles and `bin/gate.sh` stays green.

## Phase 3 — Implement
- **Built:**
  - `map_document.rs`: `SUPPORTED_TILE_SIZES=[8,16,32]`; `check()` uses
    `!…contains`; `Display` uses `{SUPPORTED_TILE_SIZES:?}`; field+variant docs;
    test fixups (set-pin renamed `supported_tile_sizes_are_8_16_32`,
    `wrong_tile_size_is_refused` 32→7, `validate_rejects_an_invalid_document`
    8→7, `error_messages_render_each_variant` 32→7 + new string, import renamed).
  - `src/client/tileset.js`: comments relabelled (author-sheet contract). No
    logic change. `src/client-app.js`: `resolveAuthorTilesetUrl()`
    (`VITE_OATHSTAR_TILESET`) → `AUTHOR_TILESET_URL`; null-guard early-return in
    `loadTileset`; image base derived from the descriptor URL. `canvas-map.js`
    untouched (fallback already present).
  - NEW `docs/tileset-contract.md`; NEW `tests/fixtures/tilesets/sample-8px.json`
    (8px, 4 kind tiles). `tests/tileset.test.js` rewritten fixture-based (dropped
    committed paths + the PNG decoder; **deleted** T5 blank-colors pin + T6 .tsx
    twin). `tests/canvas-map.test.js` repointed (`committedTileset`→`authorTileset`
    →fixture; `sSize` 32→8; stale title tidied).
  - DELETED: `bin/generate_oathstar_tileset.py`, `bin/import_elvgames_tilesets.py`,
    `public/tilesets/oathstar-starter-32x32/`,
    `public/tilesets/oathstar-elvgames-fantasy-16x16/`,
    `tests/elvgames-tileset.test.js`.
  - `docs/decisions.md`: Decision 059 added; 035/050/054/055 annotated.
    `docs/map-system.md` + `docs/ui-design.md` de-generatored.
  - **Checks:** `cargo check --workspace --tests` green; `cargo fmt --check`
    clean; sweep found **no live refs** to deleted assets; Rust tests
    content 65 + studio 27 pass; full JS suite 70 pass; all changed JS
    `node --check`-parses.
- **Deviations from design (+ reason):**
  - `crates/oathstar-content/src/lib.rs` re-export renamed
    `SUPPORTED_TILE_SIZE`→`SUPPORTED_TILE_SIZES` — not in the manifest; the
    compiler caught the re-export. Mechanical.
  - `crates/oathstar-studio/src/editor.rs` `BAD_TILE_DOC` + its two assertions
    32→7 — manifest row 1 named only `map_document.rs`; the **sweep** (row 17)
    found a studio test that used 32 as the unsupported case (32 is now valid).
    This is exactly failure `524538ee` in action.
  - Decision 059 also **amends 055** (design said 035/050/054) — 055's
    "native 32px source" claim needed superseding too; its 64px render-cell half
    stands.
  - `AUTHOR_TILESET_URL` resolved via `resolveAuthorTilesetUrl()`
    (`VITE_OATHSTAR_TILESET`) rather than a literal `null` const — mirrors
    `resolveApiBase()`, avoids a constant-condition smell, and lets the owner
    point at art via env with no code edit.

## Inspect (Phase 3.5)
- **Lenses run (4 parallel critics):** Rust correctness + mutation-tightness;
  JS seam correctness + coverage; deletion-safety + commit-scope; decisions +
  contract-doc accuracy.
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | **critical** | New `SUPPORTED_TILE_SIZES` doc tripped `clippy::too_long_first_doc_paragraph` (nursery, `-D warnings`) — gate:2 RED, masked by `cargo test` passing (`map_document.rs:19`) | REAL (critic ran `cargo clippy --tests`) | Split into a 1-sentence first paragraph + blank `///`; re-ran clippy on both crates — clean |
  | 2 | medium | Stale "16×16-pixel source grid" doc-comments contradict the multi-size model (`map_document.rs:3,7,31`; `map-system.md:260,267`) | REAL | Reworded to source-tile units / `SUPPORTED_TILE_SIZES` (8/16/32); final grep clean |
  | 3 | low | `tileset-contract.md` over-promised image "relative to the descriptor URL" (code does sibling-filename concat) | REAL | Narrowed to "sibling of the descriptor (bare filename)" |
  | 4 | low | `tileset-contract.md` blurred which validator enforces 8/16/32 (client `validateTileset` accepts any +int; the bound is the server map model) | REAL | Added a two-validator note in the Validation section |
  | 5 | **high (scope)** | Pre-existing online-first redesign WIP in the tree (`game-overview`/`team-handbook`/`technical-architecture.md`, `index.html`, `styles.css`, an intake doc, `modules/elvgames-demo/`) + intertwined `exitLine`-removal hunks in `client-app.js` — NOT ticket #46 | REAL (scope, not a code bug) | **Carried to `/commit`:** stage only #46 files; `client-app.js` needs hunk-level staging (keep `exitLine` hunks out); `decisions.md` whole-file OK. Surfaced to owner. |
  | 6 | info | `modules/elvgames-demo/grasslands.tmx:10` references the deleted ElvGames `.tsx` | REAL but OUT OF SCOPE | The referrer is untracked/out-of-scope; exclude from #46. Flagged: the elvgames-demo module is now dead with the tileset retired |
  | 7 | — | Claimed mutation gap (only 16 exercises the accept path) | **REJECTED** | Critic ran `cargo mutants --list`: all 4 mutants already killed (100% MSI); cargo-mutants does not mutate array literals. Phase 4's 8/16/32-accept + reject-0 are behavioral coverage, not MSI-required |
  | 8 | low | `resolveAuthorTilesetUrl` duplicates `resolveApiBase` | **REJECTED (won't-fix)** | Two consumers, different defaults; critic agrees — leave until a 3rd appears |
- **Deletion safety: GREEN** — all build/config surfaces (vite/index.html/tauri/.gitignore/package.json; no service worker exists) verified clean; no live ref to a deleted asset in any tracked/in-scope file.
- **Re-verified after fixes:** `cargo clippy -p oathstar-content -p oathstar-studio --tests` clean; `cargo test -p oathstar-content` 65 pass; stale-claim grep clean.

## Phase 4 — Validate
- **Tests added (Rust, RT-1/RT-3):** `every_supported_tile_size_validates`
  (8/16/32 each -> `Ok`) and `unsupported_tile_sizes_are_refused` (0/7/24/64 ->
  typed `UnsupportedTileSize{found}`), replacing the narrower
  `tile_size_sixteen_validates` + `wrong_tile_size_is_refused`. JS tests were
  already repointed to the 8px fixture at implement; no new JS tests needed.
- **`cargo test --workspace`:** 505 tests across 8 crates — 0 failed (content
  65 incl. the new size tests).
- **`node --test tests/*.test.js`:** 70 pass, 0 fail.
- **`bin/gate.sh` (FULL):** `GATE GREEN [full]` — 17/17.
  - gate:15 rust coverage **98.72%** lines (floor 94) — `map_document.rs` 100%,
    `lib.rs` 99.73%, `editor.rs` 99.61%.
  - gate:16 js coverage **88.52%** (floor 75).
  - gate:17 mutation **473 caught / 0 missed → MSI 100.0%** (floor 100).
  - Receipt written (`.git/oathstar-gate-receipt`); will be re-validated at
    `/commit` after the Phase 5 doc/archive edits change the tree.
- **Pre-existing exclusions:** none caused a gate failure (the static gates
  passed over the whole worktree, pre-existing WIP included). The separate
  online-first redesign WIP + `modules/elvgames-demo/` (Inspect finding #5)
  stay OUT OF SCOPE — a `/commit` staging concern, not a validate failure.

## Phase 5 — Complete
- Docs updated:
- Forge capture (aar/failures/rules/decisions):
- Ticket closed:
- Archived:
