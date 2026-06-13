# WORK-tileset-flatten-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- Request: step A of the blank-colors program — flatten the committed
  tileset to solid colors with a lean name set; keep the four
  load-bearing names so the client changes nothing; add flat extras for
  the slice's biomes; pin the blank-colors contract in tests.
- Intake source: INTAKE-tileset-region-authoring-per-tile-metadata
  (resequenced steps, row A) + INTAKE-blank-colors-vertical-slice
  (program context). The intake's lean-name-set wording supersedes the
  initial "keep all 64 names" framing.
- Classification / tier: work pipeline, small — generator rework +
  regenerated committed assets + test updates; no engine, no client.
- Forge recall:
  - Decision 050 (#32): name-keyed tile resolution over committed assets
    under public/ — the contract that makes flatten a pure asset change.
  - Decision 051 (#33): entities are runtime overlays — no hero/enemy
    tiles in the sheet.
  - #32 inspect failures to not repeat in new tests:
    `Object.create(null)` for name maps (`__proto__`), `Number.isInteger`
    for geometry.
  - Python env quirk (spike): the generator needs the PIL-bearing
    interpreter (system 3.9 has PIL, 3.12 has tomllib but no PIL).
  - Spike artifacts (ephemeral, /tmp — gone on reboot/fresh clone) were
    the palette source; the authored truth now lives in the generator's
    FLAT_TILES table, so nothing load-bearing references /tmp.
- Ticket: forge #36 `edd292c5-a346-49aa-86a8-f60191f2a081` (open); local
  doc TICKET-36-flatten-tileset-solid-colors-lean-names.md.
- EARS requirements reviewed: REQ-001..005 — load-bearing names intact,
  per-tile uniformity pin, lean-set cross-checks, deterministic
  regeneration, renderer suites unchanged.
- AAR: `c5f23435-7b90-4a77-8651-46f265687754` (opened at plan).

## Phase 2 — Design

- Approach / architecture:
  - **One table is the whole generator.** `FLAT_TILES` — an ordered list
    of `(name, palette_color)` — replaces all texture painters,
    connectivity-variant generation, and the seeded-noise rng (hashlib +
    random go away entirely). Each tile paints as one `rectangle(fill)`.
  - **The lean set (11 names, 4×3 sheet, 64×48 px):**
    | name | color (rgba) | role |
    |---|---|---|
    | shadow_void | (16,20,25) | unexplored — load-bearing |
    | stone_floor | (105,111,104) | discovered / city floor — load-bearing |
    | wall_face | (67,74,74) | blocked — load-bearing |
    | spawn_marker | (95,204,177) | current room / hero teal — load-bearing |
    | grass | (52,123,65) | forest floor (step C paints) |
    | dirt | (129,91,48) | road (spike road brown) |
    | cave_floor | (70,62,78) | cave floor (spike violet-grey — new) |
    | deep_water | (38,92,145) | water |
    | stairs_up | (169,106,55) | floor link up |
    | stairs_down | (71,45,28) | floor link down |
    | exit_marker | (216,169,65) | door/exit marker (spike door gold) |
    Slot 12 stays transparent and unnamed (spare). Colors come from the
    existing PALETTE + the spike concept (`/tmp/oathstar-flat-spike/
    concept.py`); hero/enemy/item colors stay runtime overlays
    (Decision 051) — no entity tiles.
  - **The JSON/tsx carry the palette as contract:** each tile entry gains
    `color: "#rrggbb"` (authored truth, future server-side use in C).
    `validateTileset` ignores unknown tile keys (verify at implement) —
    client untouched.
  - **Uniformity pin = decode the committed PNG in the test.** A
    ~50-line zero-dep PNG reader in `tests/tileset.test.js` using
    `node:zlib` (parse IHDR/IDAT, inflate, unfilter all five row filter
    types — PIL emits 8-bit RGBA non-interlaced). The pin asserts, for
    every NAMED tile: all 256 pixels identical AND equal to the tile's
    declared JSON `color`; plus the four load-bearing tiles match the
    authored palette hex exactly.
  - **Determinism:** no rng anywhere; `Image.save(..., compress_level=9)`
    pinned explicitly; JSON/tsx emitted in FLAT_TILES order. Validate
    re-runs the generator and `git diff --exit-code` the asset dir.
    Generator runs under the PIL-bearing system python3 (the spike's
    interpreter-split lesson).

- File manifest:
  | # | File | Change |
  |---|---|---|
  | 1 | bin/generate_oathstar_tileset.py | FLAT_TILES table; drop texture painters/variants/rng; 4×3 sheet; per-tile `color` in JSON+tsx; flat preview; README wording; compress_level pinned |
  | 2 | public/tilesets/oathstar-starter-16x16/* (png/json/tsx/preview/README) | regenerated committed assets |
  | 3 | tests/tileset.test.js | update T1 pins (4 cols, 3 rows, 11 tiles), T3 rects (new positions); add PNG-decode uniformity pin + lean-set json↔tsx cross-check |

- ### Regression Test Plan
  | # | Test | Proves Requirement |
  |---|---|---|
  | TT1 | committed JSON validates; 4 load-bearing names resolve; geometry 16px/4col/3row/11 tiles (T1 updated) | REQ-001 |
  | TT2 | PNG decode: every named tile uniform; pixels == declared JSON `color`; four kind tiles match authored palette hex exactly (NEW) | REQ-002 |
  | TT3 | lean-set names exactly the designed 11 (sorted compare); tsx contains the same names once each (NEW) | REQ-003 |
  | TT4 | re-run generator → `git diff --exit-code public/tilesets/` (validate-phase shell step, recorded in notes) | REQ-004 |
  | TT5 | canvas-map + client suites pass unchanged (no edits) | REQ-005 |
  | TT6 | existing T2 (validator refusal table) and T4 (KIND_TILE_NAMES) pass untouched | REQ-001 |
  - Uncoverable: none — the PNG decoder handles all five PIL filter
    types; preview.png has no pin beyond regeneration (human artifact).

- Risks / decisions:
  - PIL version drift could change PNG compression bytes across machines
    — accepted (single-dev gate today); compress_level pinned to reduce
    variance; the uniformity pin reads pixels, not bytes, so it survives
    drift.
  - The in-test PNG reader must handle filters 0–4 — small fixed-size
    input (64×48), exhaustively exercised by the committed sheet itself.
  - Dropping 53 names is safe: grep shows only tests reference dropped
    names (`grass` rect pin — updated); client uses the 4 via
    KIND_TILE_NAMES only.
  - `color` key tolerance in validateTileset verified at implement (it
    reads name/x/y and ignores extras per #32 design).

## Phase 3 — Implement
- Built:
  - `bin/generate_oathstar_tileset.py` rewritten: the `FLAT_TILES` table
    (11 names × rgba/tags/collision) IS the generator — all texture
    painters, mask variants, rng/hashlib deleted (484 → ~170 lines);
    4×3 sheet; per-tile `color` hex in JSON and tsx properties;
    `compress_level=9` pinned on both PNG saves.
  - Committed assets regenerated: png (225 bytes — flat color compresses
    to nothing), preview, json, tsx; README rewritten for the
    blank-colors era with the full name/color table.
  - Verified in-phase: re-run reproduces identical bytes (shasum);
    `validateTileset` accepts the new JSON with zero client changes and
    all four `KIND_TILE_NAMES` rects resolve; preview visually confirmed
    (void/stone/wall/teal · grass/road/cave/water · stairs/stairs/gold).
- Deviations from design (+ reason):
  - Kept `tags`/`collision` fields from the old record shape (the future
    importer reads collision; dropping them would be scope creep in
    reverse). Design's table carried them implicitly.
  - Tile order puts the four load-bearing names first (ids 0–3) — the
    design table's order, now also the sheet order.

## Inspect (Phase 3.5)
- Lenses run: correctness (PNG↔JSON↔tsx pixel/field verification,
  determinism, dropped-name sweep) and consumer-impact (every reader of
  the assets: client modules, tests, tauri, crates, vite, docs) — two
  parallel critics, both verifying concretely (decoded the PNG, ran the
  suites, re-ran the generator).
- Findings:
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | — | Five stale committed-asset pins in tests/tileset.test.js (50/51/53 count+grid, 119/120 grass+stone_floor rects); 2 tests red, 3 asserts hidden behind short-circuits | REAL, planned — Phase 4 owns test updates; full pin list recorded for validate | updated at validate |
  | 2 | low | README + generator docstring claim the PNG-pixel cross-check test in present tense; no test reads the PNG yet | REAL, resolves at validate — the pin is REQ-002's planned deliverable this same ticket; README qualifier added for the byte-vs-pixel distinction | validate lands TT2 |
  | 3 | low | Generator lacked capacity + opacity guards: a 12th+ entry would clip-paint silently; a translucent color is inexpressible in the `color` hex contract | REAL | `assert len(FLAT_TILES) <= COLS*ROWS` + per-entry alpha==255 assert; regenerated — bytes unchanged |
  | 4 | low | /tmp spike paths cited as live sources in the notes | REAL (doc) | marked ephemeral; FLAT_TILES named as the authored truth |
  | 5 | nit | Wrong-by-one dates (TICKET-36 created, intake PROMOTED: 2026-06-13 → 12) | REAL | fixed |
  | 6 | info | Byte-exact regeneration is PIL-toolchain-contingent | ACCEPTED — gate never regenerates; validate's pin compares pixels, not bytes; README now says so |
  | 7 | info | spawn_marker teal vs the renderer's permanent fallback gold (`MAP_PALETTE.current`); exit_marker gold near the item-dot gold | ACCEPTED — fallback palette is intentionally separate (Decision 050); markers carry outlines; flagged for a designer glance at W1 |
  | 8 | info | Decision 050's prose still describes the 128×128/64-tile sheet | REAL (doc) | amended at Phase 5 with the #36 note |
  - Verified clean by the critics: PNG decoded — all 11 tiles uniform,
    alpha 255, pixels == JSON `color`, spare slot transparent; json↔tsx
    field-identical; sparse tsx tilecount=12 is valid Tiled semantics
    with no programmatic tsx consumer; zero references to any of the 55
    dropped names outside tests; client/canvas/tauri/crates/vite read
    everything dynamically; canvas-map+client suites 34/34 green against
    the new asset; generator idempotent and byte-stable across re-runs.

## Phase 4 — Validate
- Tests added (2 new + 5 pins updated in tests/tileset.test.js):
  - T5 "every committed tile is one uniform color matching its declared
    contract" — a zero-dep PNG reader (node:zlib; IHDR/IDAT, all five
    row filters) decodes the committed sheet; every named tile's 256
    pixels identical, opaque, equal to its JSON `color`; the four
    load-bearing tiles match the authored palette hex verbatim; the
    spare 12th slot is transparent. Pixel-based, so it survives PNG
    encoder byte drift.
  - T6 "the lean name set is exact and consistent across json and tsx" —
    the 11 names in exact order, duplicate-free; tsx names AND colors
    mirror the JSON in order; tsx grid says tilecount 12 / columns 4.
  - Updated pins: columns 8→4, rows 8→3, count 64→11, grass rect
    (0,0)→(0,16), stone_floor rect (64,0)→(16,0) — the five stale
    asserts inspect enumerated (incl. the three hidden behind
    short-circuits).
- TT4 determinism: re-ran the generator; `shasum -c` over all five
  committed files — every byte reproduced (the `git diff --exit-code`
  form applies post-commit; checksums are the pre-commit equivalent).
- `cargo test --workspace`: ok — 410 passed, 0 failed (untouched by this
  ticket; run as gate evidence).
- `node --test tests/*.test.js`: ok — 63 pass, 0 fail (61 + 2 new).
- `bin/gate.sh`: **GATE GREEN [full]** — 17 passed, 0 failed; mutation
  399 caught / 0 missed → MSI 100.0% (no Rust changed; suite identical).
- Pre-existing exclusions: none.

## Phase 5 — Complete
- Docs updated: `docs/decisions.md` — Decision 054 (blank-colors era:
  flat tiles are the contract, art is a skin) + Decision 050's geometry
  prose amended with the #36 note; README rewritten at implement.
- Forge capture: AAR `c5f23435` submitted (completed, effectiveness 5);
  AD `AD-claude-blank-colors-flat-tileset-contract-001`. No
  failure-record — inspect found hardening/doc items, no bug (assets
  were pixel-perfect on first generation).
- Ticket closed: forge #36 `edd292c5` → done; local doc moved to
  `docs/planning/tickets/closed/`.
- Archived: spec+notes pair moved to
  `docs/planning/pipeline/completed/`.
