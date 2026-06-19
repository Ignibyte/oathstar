# WORK-map-real-tiles-default-v1 — Notes

Per-phase working notes for the paired `.spec.md`.

## Phase 1 — Plan
- **Request:** S3 (map visuals) slice 1 — make the game map render real tiles instead of
  flat `MAP_PALETTE` colors. Owner-confirmed program step ("paint with real tiles"), after
  S1 #53 + S2 #51.
- **Intake:** `INTAKE-region-model-rethink-and-owner-authoring` (S3); supersedes the
  blank-colors direction (#38/#40 shelved — new ticket **#54**, not a reuse).
- **Classification / tier:** work pipeline, one slice, **client-only JS** (`src/client/tileset.js`
  + `src/client-app.js`) + node tests. No Rust.
- **Recon (Explore, working tree) — the slice-shaping finding:** the by-kind real-tile
  render path **already exists**:
  - `protocol::MapSnapshot`/`MapRoomSnapshot` carry **no** kind/tile-name — the client
    classifies via `cellKind(cell)` → `empty|discovered|current|blocked` (`canvas-map.js`).
  - `toDrawPlan(model, tileset=null)` already sets `sprite: kindRects[kind]` when a tileset
    is passed (`kindTileRects`), null otherwise; `client-app.js` `drawMapCanvas` already
    `ctx.drawImage`s the sprite with a `fillRect` **flat fallback**.
  - `src/client/tileset.js` is pure + node-tested (`KIND_TILE_NAMES`, `validateTileset`,
    `tileRect`, `kindTileRects`); `KIND_TILE_NAMES` maps the 4 kinds → 4 named tiles.
  - `public/tilesets/arctic.{png,json}` ships those 4 tiles and is served at
    `/tilesets/arctic.json` (Vite `public/` — dev, `dist/`, Tauri). `oathstar-server` is
    **API-only** (no static/tileset route, no SPA serving — Vite proxies `/command|state|events`).
  - **The gap:** `client-app.js` `resolveAuthorTilesetUrl()` returns `null` unless the
    build-time `VITE_OATHSTAR_TILESET` is baked → no sheet → flat colors by default.
- **Lean/rich crux → resolved:** the lean render path is built; slice 1 is just the
  **default-on** flip (no protocol/server change). The rich paths (S3.2 per-room floor name
  over the wire; S3.3 authored layer paint) are clean follow-ons that add data to the wire —
  explicitly deferred. No owner decision needed to proceed with slice 1.
- **EARS:** REQ-001 default when unset/blank · REQ-002 override wins · REQ-003 sprite/flat
  render (regression-guard of the existing path) · REQ-004 gate.
- **Ticket:** forge **#54** `5bafbae1-d347-464d-8a70-2525e3e6c000` (minted; supersedes #38/#40).
  Local doc `docs/planning/tickets/open/TICKET-54-map-real-tiles.md`.
- **aar_id:** `fb58e79d-f692-4925-9a7f-4661a1ad158f`
- **Branch:** off `main` (`f56cc48`). Stash (`stash@{0}`) stays parked.

## Phase 2 — Design

### Code reconnaissance (working tree)
- `src/client/tileset.js` — pure, DOM-free; exports `KIND_TILE_NAMES` (line 11),
  `validateTileset`, `tileRect`, `kindTileRects`. The natural home for the resolver +
  default const. Dedicated test file **`tests/tileset.test.js`** exists.
- `src/client-app.js` — `resolveAuthorTilesetUrl()` (lines 51-60) reads
  `import.meta.env.VITE_OATHSTAR_TILESET` (truthy → return), else `null`; `AUTHOR_TILESET_URL`
  (62) feeds `loadTileset()` (67) which fetches → validates → sets `tilesetData`/`tilesetImage`,
  and on any failure **warns + returns** (keeps flat colors). The browser seam (`import.meta`,
  `fetch`, `Image`) is not node-importable.
- REQ-003 (sprite/flat) is **already covered** by the ticket-#32 tests in
  `tests/canvas-map.test.js` (`toDrawPlan without a tileset keeps the flat-color contract`
  + the with-tileset sprite-rect test).

### Approach / architecture
Pure default-resolution in `tileset.js`; a one-line wiring change in the browser entry. **No
render-path change, no Rust.**
1. **`tileset.js`** — add `export const DEFAULT_TILESET_URL = "/tilesets/arctic.json";` and
   `export function resolveTilesetUrl(override)`: returns the **trimmed** `override` when it is
   a non-empty string, else `DEFAULT_TILESET_URL`. Pure, total (never throws, never empty).
2. **`client-app.js`** — `resolveAuthorTilesetUrl()` reads the env override into a local (in
   the existing `try`, tolerating absent `import.meta.env`) and returns
   `resolveTilesetUrl(override)`; update the now-stale "or null for no sheet yet" comment.
   Import `resolveTilesetUrl` (+ `DEFAULT_TILESET_URL` is internal to the helper).

### Locked decisions (this phase)
- **`resolveTilesetUrl(override)`** signature (override-in, URL-out) — pure + node-testable,
  so the default/override/trim logic is fully covered without the browser.
- **`DEFAULT_TILESET_URL = "/tilesets/arctic.json"`** as a shared `const` in `tileset.js`
  (single source). Origin-relative → resolves under the SPA host in **Vite dev** (`public/`
  served at `/`), **`dist/`** (Vite copies `public/`), and **Tauri** (bundles `public/`).
- **Blank == unset** (trim → `""` → default); a configured non-blank override wins.
- **No regression risk:** if the default sheet ever 404s, `loadTileset` warns + the flat-color
  fallback fires (today's behavior).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `src/client/tileset.js` | Add `DEFAULT_TILESET_URL` const + pure `resolveTilesetUrl(override)`. |
| 2 | `src/client-app.js` | `resolveAuthorTilesetUrl()` → `resolveTilesetUrl(envOverride)` (default-on); refresh the stale comment; import the helper. |
| 3 | `tests/tileset.test.js` | node tests for `resolveTilesetUrl` (default + override + trim). |
| 4 | `docs/tileset-contract.md` | (Phase 5) note the default sheet / default-on behavior. |

### Regression Test Plan
| # | Test | Kind | Proves |
|---|---|---|---|
| T1 | `resolveTilesetUrl(undefined)`, `("")`, `("   ")`, `(null)`, `(123)` all === `DEFAULT_TILESET_URL` | node (`tileset.test.js`) | REQ-001 |
| T2 | `resolveTilesetUrl("/custom/sheet.json")` === itself; `("  /trim.json  ")` === `"/trim.json"` (trimmed, override wins) | node (`tileset.test.js`) | REQ-002 |
| T3 | (existing) `toDrawPlan(model, tileset)` ops carry kind sprite rects; `toDrawPlan(model)`/`(model,null)` → `sprite:null` + flat palette | node (`canvas-map.test.js`, #32) | REQ-003 (regression-guard) |
| G1 | `bin/gate.sh` FULL green (node tests + JS coverage ≥75%) | gate | REQ-004 |
- **Coverage:** `resolveTilesetUrl` both branches exercised by T1/T2 (it's in the
  test-imported `tileset.js`, so it counts toward JS coverage). No Rust delta → no new mutants.
- **Genuinely uncoverable:** the `client-app.js` `import.meta.env` read (browser-only, not
  node-importable) — the reviewed wiring seam; all override *values* are covered via the pure
  helper. (Same class as the studio glue seams.)

### Risks / decisions
1. **Default-on fetch** — every load now requests `/tilesets/arctic.json`; absent → graceful
   flat fallback. Net: real tiles appear wherever the asset ships (everywhere it does today).
2. **Tauri origin** — origin-relative path assumed to resolve against the bundled `public/`;
   can't unit-test the Tauri shell, so it's a reasoned call de-risked by the fallback.
3. **Hardcoded default path** — acceptable (the committed sheet); a per-world authored sheet
   is the S3.2/S3.3 follow-on, not this slice.

## Phase 3 — Implement
- **Built to the manifest** (new tests are Phase 4):
  - `src/client/tileset.js` — `export const DEFAULT_TILESET_URL = "/tilesets/arctic.json"`
    + pure `resolveTilesetUrl(override)` (`typeof override === "string" ? override.trim()`
    non-empty → trimmed, else the default). Total, never throws/empty.
  - `src/client-app.js` — `import { validateTileset, resolveTilesetUrl }`; `resolveAuthorTilesetUrl()`
    reads `import.meta.env.VITE_OATHSTAR_TILESET` into a local (try-guarded for absent
    `import.meta.env`) and returns `resolveTilesetUrl(override)` — **default-on**; the stale
    "or null for no sheet yet" comment refreshed.
- **No render-path change, no Rust.** The sprite/flat fallback (`toDrawPlan` + `drawMapCanvas`)
  is untouched.
- **Verified:** `tileset.js` parses + exports; `resolveTilesetUrl(undefined)` →
  `/tilesets/arctic.json`, `("  /x.json  ")` → `/x.json`; `node --test tests/tileset.test.js
  tests/canvas-map.test.js` → 21 pass, 0 fail (no regression).
- **Deviations from design:** none.
- **For Phase 4:** `tests/tileset.test.js` — T1 (default for `undefined`/`""`/whitespace/non-string)
  + T2 (override wins + trim). REQ-003 already covered by the #32 sprite/flat tests.

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel **read-only `Explore`** critics, no worktree mutation —
  `PR-claude-inspect-critic-read-only-001`): **correctness + regression**, **simplification
  + seam**.
- **Findings:**
  | # | Sev | Finding | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | low | `loadTileset`'s `if (!AUTHOR_TILESET_URL) return;` is now effectively unreachable (default-on ⇒ the URL is always set) and its inline comment "no author sheet yet" is stale (`client-app.js:70`). | **Real (comment), guard kept** | Refreshed the comment — the guard stays as defense-in-depth (never `fetch` a falsy URL); the actual flat-color fallback is the fetch/validate/image failure paths. Not removed (a falsy-URL fetch guard is sensible if the resolver's contract ever changes). |
- **Cleared (critics' concrete checks):**
  - `resolveTilesetUrl` traced over `undefined`/`null`/`""`/whitespace/non-string → default;
    `"  /x.json  "` → `"/x.json"` (trim, override wins); never returns empty.
  - **`AUTHOR_TILESET_URL` null-consumer sweep:** the *only* consumer is `loadTileset`'s
    guard; the 404 / bad-json / image-`onerror` paths all warn + return with `tilesetData`
    null → `toDrawPlan(model, null)` → `sprite:null` → flat-color `fillRect`. **No regression,
    no crash/blank** when the sheet is missing.
  - Single source: no other hardcoded `/tilesets/` path; `tileset.js` (pure) is the right home;
    `public/tilesets/arctic.json` exists and `public/` is Vite's served root (dev/`dist/`/Tauri);
    no secrets/SAST tokens; resolver matches the file's idiom.
- **Re-verified:** worktree = the 2 expected files (no critic clobber); `node --test
  tests/tileset.test.js tests/canvas-map.test.js` → 21 pass.
- **Capture:** no `failure-record` (no bug — a stale comment); no new rule.

## Phase 4 — Validate
- **Tests added (+2, `tests/tileset.test.js`):**
  - T1 (REQ-001) — `resolveTilesetUrl` returns `DEFAULT_TILESET_URL` for `undefined`, `null`,
    `""`, whitespace, `123`, `{}`, `[]`; asserts the default is `/tilesets/arctic.json`.
  - T2 (REQ-002) — a non-blank override wins and is trimmed (`"  /trim.json  "` → `"/trim.json"`).
  - REQ-003 (sprite/flat) stays covered by the ticket-#32 tests in `canvas-map.test.js`.
- **`node --test tests/*.test.js`:** GREEN — **83 pass**, 0 fail (+2).
- **`cargo test --workspace`:** GREEN (no Rust change — regression only).
- **`bin/gate.sh` (FULL):** **GATE GREEN — 17/17.** Mutation **592 caught / 0 missed → MSI
  100.0%** (no Rust delta → no new mutants); **JS coverage ≥75%** passed (the resolver's both
  branches covered by T1/T2); rust coverage held. Commit-gate receipt written.
- **Pre-existing exclusions:** none.

## Phase 5 — Complete
- **Docs updated:** `docs/tileset-contract.md` "Pointing the client at a sheet" — now
  describes `resolveTilesetUrl` (override wins, else `DEFAULT_TILESET_URL`
  `/tilesets/arctic.json`) and **real tiles by default**, served via the SPA host's
  `public/`, with the flat fallback for a failed fetch. (`map-system.md`'s render-path
  description stays accurate — no change.)
- **Forge capture:** `aar-submit` (AAR `fb58e79d`, completed, score 5);
  `PR-claude-recon-before-build-slice-001` (recon the existing seam before scoping a
  "build X" slice — it may collapse to a config flip, as it did here); no `failure-record`
  (no bug). Process slip (not recorded — harness usage, no shipped defect): tried to `Edit`
  a test file only read via `sed`/Bash + appended via `cat >>` → the Edit needed a Read-tool
  read first; re-read then edited the import; net-correct.
- **Ticket status:** forge **#54 stays OPEN** — slice 1 of a multi-slice ticket;
  `ticket-comment` recorded slice-1 done + the S3.2/S3.3 plan.
- **Archived:** `…/completed/WORK-map-real-tiles-default-v1.{spec,notes}.md`.
