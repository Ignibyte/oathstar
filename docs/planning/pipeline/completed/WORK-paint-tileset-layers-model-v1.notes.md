# WORK-paint-tileset-layers-model-v1 — Notes

Per-phase working notes for the paired `.spec.md`.

## Phase 1 — Plan
- **Request:** Paint-system SLICE 1 — the document-model foundation (tileset
  registry + multiple tile layers + validation) in `MapDocument`. Additive,
  Rust-only, no UI. First slice of the multi-slice paint-system program; owner
  wants to chain slices this session ("see how far we get").
- **Intake source:** `INTAKE-paint-system-tile-editor.md` (new this session —
  the S1-S5 program vision). S1 promoted to ticket #47.
- **Classification / tier:** work pipeline, one shippable slice. Pure additive
  model extension + validation; gate-green on Rust tests alone (no UI/wire).
- **Forge recall:** pre-flight green (forge up, no bulletins, no active
  pipeline). Surfaced the #46 decision family (050/059 — `SUPPORTED_TILE_SIZES`,
  which the tileset registry reuses). Code already mapped this session:
  `MapDocument` (terrain/rooms/spawn/regions/subregions + `in_bounds` + the
  `MapValidationError` typed-error style + `terrain_at` BTreeMap pattern), the
  studio editor (`editor-canvas.js` read-only flat-color render), `render.rs`.
- **Ticket:** #47 `c7c90d82-a960-4de5-919a-3d01823de95b` (feature).
- **EARS requirements reviewed:** REQ-001..010 in the spec — happy path,
  each validation failure arm, the serde-additive backward-compat proof
  (REQ-008), the materialize-equivalence proof (REQ-009, authoring-visual), and
  the gate (REQ-010, mutation-tight).
- **AAR id:** 274d0a2e-005b-4dfe-b959-ecd24e48d773

## Phase 2 — Design

### Approach / architecture
All in `crates/oathstar-content/src/map_document.rs`, additive. New types:
- `Tileset { id, image: String, tile_size: u32, columns: u32, rows: u32, tiles:
  BTreeMap<u32, TileMeta> }` — `tiles` is sparse per-tile metadata keyed by tile
  index (`#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`).
- `TileMeta { name: Option<String>, passable: Option<bool>, tags: Vec<String> }`
  — all optional/empty-skipped.
- `LayerCell { x, y, z, tileset: String, index: u32 }` — a painted cell,
  **flattened** like `TerrainCell` (a map can't be JSON-keyed by a struct `Cell`;
  `serde_json` requires string keys). `fn cell(&self) -> Cell` helper.
- `Layer { id, name: String, kind: LayerKind, visible: bool, cells:
  Vec<LayerCell> }`. `LayerKind` = a `#[derive(Default)]` enum, single
  `#[default] Tile` variant (forward-looking; S2 adds Ground/Decoration/…).
  `visible` defaults true (`#[serde(default = "default_true")]`).
- **New `MapDocument` fields:** `tilesets: Vec<Tileset>`, `layers: Vec<Layer>` —
  both `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. **Vec, not
  BTreeMap**, so duplicate-id is reachable + testable (mirrors `rooms`).
- **7 new `MapValidationError` variants** (+ Display arms, name the offender):
  `DuplicateTilesetId{id}`, `UnsupportedTilesetGeometry{id}`,
  `TileMetaIndexOutOfRange{tileset,index}`, `DuplicateLayerId{id}`,
  `LayerCellOutOfBounds{layer,cell}`, `UnknownTilesetRef{layer,cell,tileset}`,
  `TileIndexOutOfRange{layer,cell,index}`. (A duplicate layer cell reuses the
  existing `DuplicateCell{cell}`.)
- **`check()` hook** — after the `tile_size` check, before the terrain pass, add:
  (1) **tileset pass**: dedup ids (`BTreeSet`); `SUPPORTED_TILE_SIZES.contains`
  + `columns>0 && rows>0`; per-tile-meta index `< capacity`; build a
  `tileset_capacity: BTreeMap<&str, u64>` lookup. (2) **layer pass**: dedup layer
  ids; per cell — `in_bounds` (reuse), dedup cell (`DuplicateCell`), resolve
  tileset via the lookup (`UnknownTilesetRef`), `index < capacity`
  (`TileIndexOutOfRange`). `capacity = u64::from(columns) * u64::from(rows)`
  (widen to avoid overflow; implement-time: confirm clippy `arithmetic_side_effects`
  is allowlisted, else `checked_mul`). check() still returns the start room id.
- **`materialize`/`build_world` UNCHANGED** — `build_world` reads only
  `self.rooms` + catalog, so layers are ignored ⇒ REQ-009 holds with no code.
- Conventions (§14): typed errors, no panics on input paths, `BTreeSet`/`BTreeMap`
  for determinism, Eq-safe (no `f32` — **opacity omitted this slice**; the
  `MapDocument` derives `Eq`).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-content/src/map_document.rs` | Add `Tileset`/`TileMeta`/`LayerCell`/`Layer`/`LayerKind` + `default_true`; add `tilesets`/`layers` fields (serde-default, skip-empty); add the 7 `MapValidationError` variants + Display arms; add the tileset + layer passes to `check()`; add the slice's tests. **Only file this slice.** |

### Regression Test Plan
| # | Test | Proves Req |
|---|---|---|
| RT-1 | `valid_doc` + a tileset + a layer with an in-range cell → `validate() == Ok` | REQ-001 |
| RT-2 | layer cell references a tileset id not in the registry → `UnknownTilesetRef` | REQ-002 |
| RT-3 | layer cell `index >= columns*rows` → `TileIndexOutOfRange` | REQ-003 |
| RT-4 | two tilesets share an id → `DuplicateTilesetId`; two layers share an id → `DuplicateLayerId` | REQ-004 (both arms) |
| RT-5 | layer cell out of grid bounds → `LayerCellOutOfBounds`; duplicate layer cell → `DuplicateCell` | REQ-005 |
| RT-6 | tileset `tile_size=7` → `UnsupportedTilesetGeometry`; `columns=0` → same; `rows=0` → same | REQ-006 (3 sub-conditions, each killed) |
| RT-7 | per-tile metadata index `>= columns*rows` → `TileMetaIndexOutOfRange` | REQ-007 |
| RT-8 | deserialize a JSON document with **no** `tilesets`/`layers` → `Ok` + `validate Ok`; re-serialize omits them (round-trip) | REQ-008 |
| RT-9 | `valid_doc.materialize()` byte-equals `(valid_doc + a layer).materialize()` | REQ-009 |
| RT-10 | extend `error_messages_render_each_variant` with the 7 new variants (Display arms pinned) | REQ-002..007 (mutation-tight Display) |
| RT-11 | `bin/gate.sh` FULL green incl. mutation 100% MSI on the new `check()` conditionals | REQ-010 |

Genuinely uncoverable: none — all new code is data + conditionals fully driven by RT-1..10. (cargo-mutants targets the new `check()` conditionals + Display arms; RT-2..7,10 kill them; confirmed last run that it does not mutate data/literals.)

### Risks / decisions
- **`tilesets`/`layers` are `Vec` (not `BTreeMap`)** so duplicate-id refusals are
  reachable + mutation-testable — mirrors `rooms`.
- **Opacity omitted** this slice — `f32` would break the `MapDocument` `Eq`
  derive; add later as an integer percent if render needs it (S2).
- **`LayerKind` is a single-variant enum** (`Tile`) — deliberate extension point;
  an unknown kind string is a serde deserialize error, not a validation arm.
- **Layer cells serialize as flat `Vec<LayerCell>`** (not a `Cell`-keyed map) —
  JSON can't key a map by a struct; mirrors `TerrainCell`.
- **Duplicate layer cell reuses `DuplicateCell`** — names the cell, consistent
  with terrain/rooms.
- **`capacity` is `u64`** (widen `columns*rows`) to avoid overflow; heed
  `PR-claude-clippy-clean-before-green-001` — run `cargo clippy --tests` before
  declaring green (arithmetic + new code).
- **`materialize` unchanged** — layers are authoring-visual (REQ-009); runtime
  materialization is the later #38 rework.

## Phase 3 — Implement
- **Built (all in `crates/oathstar-content/src/map_document.rs`, additive):**
  - New types: `Tileset` (id/image/tile_size/columns/rows + sparse
    `tiles: BTreeMap<u32, TileMeta>`), `TileMeta` (name?/passable?/tags),
    `LayerKind` enum (`#[default] Tile`, snake_case), `LayerCell`
    (x/y/z/tileset/index + `const fn cell()`), `Layer`
    (id/name/kind/visible/cells), `const fn default_true()`. Every pub item
    doc-commented; first doc paragraph kept short (heeds the clippy rule).
  - `MapDocument` gained `tilesets: Vec<Tileset>` + `layers: Vec<Layer>`, both
    `#[serde(default, skip_serializing_if = "Vec::is_empty")]` (byte-compat).
  - 7 new `MapValidationError` variants + Display arms (DuplicateTilesetId,
    UnsupportedTilesetGeometry, TileMetaIndexOutOfRange, DuplicateLayerId,
    LayerCellOutOfBounds, UnknownTilesetRef, TileIndexOutOfRange). Dup layer
    cell reuses `DuplicateCell`.
  - `check()` calls two new private helpers `check_tilesets()` (returns the
    id→capacity map) + `check_layers(&map)`, inserted after the `tile_size`
    check, before the terrain pass. `materialize`/`build_world` untouched.
  - `valid_doc()` test helper gained the two empty fields (struct-literal needs
    them).
  - **Checks:** `cargo clippy -p oathstar-content --tests` strict-green;
    `cargo fmt --check` clean; existing 65 content tests pass.
- **Deviations from design (+ reason):**
  - The two passes are **extracted into `check_tilesets`/`check_layers` helpers**
    rather than inlined in `check()` — inlining tripped clippy
    `too_many_lines` (139/100). Cleaner + the design anticipated an
    implement-time clippy check. Logic identical to the design.
  - **`arithmetic_side_effects` did NOT trip** on the `u64 * u64` capacity — the
    lint is allowlisted, so plain `*` is used (no `checked_mul` needed).
  - **RT tests deferred to Validate** (per the phase boundary — the new code
    compiles clean and existing tests stay green without them; the full
    RT-1..10 suite is written + RUN at Phase 4).

## Inspect (Phase 3.5)
- **Lenses run (2 critics):** correctness + mutation-readiness + materialize-
  equivalence; serde-additive backward-compat + edge cases + simplification.
- **Findings:**
  | # | Severity | Finding | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | low | `LayerCellOutOfBounds{layer,cell}` is redundant — terrain/room bounds reuse the shared `CellOutOfBounds{cell}`, and the layer pass *already* reuses `DuplicateCell`; the new variant was the lone divergence | REAL | Dropped the variant + its Display arm; layer bounds now reuses `CellOutOfBounds`; updated `CellOutOfBounds`/`DuplicateCell` docs to name layers. 6 new variants now (was 7). Re-verified clippy + 65 tests green. |
  | — | — | **Code correctness** (check_tilesets/check_layers comparisons, `>=` boundaries, `\|\|` chain, pass ordering) | **CLEAN** (Critic A verified empirically) | none |
  | — | — | **REQ-009 materialize-equivalence** — `build_world` reads only `self.rooms`/regions/subregions + catalog; `self.tilesets`/`layers` referenced only in the two check helpers | **CLEAN** (grep-confirmed) | none |
  | — | — | **REQ-008 serde-additive** round-trip (no-keys doc deserializes + re-serializes without the keys; Layer defaults kind=Tile/visible=true; id/name stay required) | **CLEAN** (Critic B verified with a throwaway round-trip test, reverted) | none |
  | — | — | u32²<u64::MAX overflow safety; `Eq` derive (no f32, opacity omitted); no panics on input paths | **CLEAN** | none |
- **Carried to Validate (mutation-readiness — test-design requirements, NOT code
  bugs):** the RT suite at Phase 4 MUST be worded to kill these mutants:
  1. **RT-3/RT-7 (high):** assert refusal at `index == columns*rows` *exactly*
     (kills `>=`→`>`); also assert `index == capacity-1` validates.
  2. **RT-6:** each geometry case perturbs exactly ONE predicate (valid on the
     other two) so `\|\|`→`&&` mutants die: (size 16, rows 4, cols 0),
     (size 16, cols 4, rows 0), (size 7, cols 4, rows 4).
  3. **RT-4:** the dup-tileset case must use VALID geometry so control reaches
     the `.insert().is_some()` line.
  4. **RT-5:** the dup-cell case must paint an IN-BOUNDS coord twice (reach
     `DuplicateCell`); the OOB case now asserts `CellOutOfBounds` (variant changed).
  5. **RT-10:** assert EXACT Display equality (incl. `[8, 16, 32]`) for the 6 new
     variants, not substrings.
  6. **default_true (high):** add a test deserializing a `Layer` JSON omitting
     `visible` and asserting `visible == true` (kills `default_true`→`false`).
- No failure-record: the single fix was a consistency nit, not a bug; the
  mutation items are forward-looking test requirements.

## Phase 4 — Validate
- **Tests added (11, in `map_document.rs`):** `tileset`/`layer`/`layer_cell`
  helpers + RT-1..11 honoring the inspect mutation requirements (exact
  `index == capacity` boundary, one-predicate-perturbation geometry cases,
  valid-geometry dup-tileset, in-bounds dup-cell, exact Display equality, a
  `visible`-defaults-true serde test).
- **`cargo test --workspace`:** all green — content **76** (was 65; +11 RT),
  every other crate 0 failed.
- **`node --test tests/*.test.js`:** 70 pass, 0 fail.
- **`bin/gate.sh` (FULL):** `GATE GREEN [full]` — 17/17.
  - **First run caught a surviving mutant** the inspect lens hadn't predicted:
    `check_tilesets:590 replace * with +` — `capacity = columns * rows` was
    indistinguishable from `columns + rows` because the test helper tileset was
    **2x2** (`2*2 == 2+2 == 4`). **Fix:** helper → **2x3** (product 6 != sum 5),
    RT-3/RT-7 boundaries shifted to index 5 (valid) / 6 (refused), with a comment
    to keep it non-square. Targeted `cargo mutants --re check_tilesets`: 13/13
    caught. Full re-run: **mutation 492 caught / 0 missed → MSI 100.0%**.
  - gate:15 rust coverage **98.75%** (floor 94); gate:16 js **88.52%** (floor 75).
- **Pre-existing exclusions:** none — the pre-existing online-first WIP did not
  affect the gate; it stays out of scope (selective staging at `/commit`).

## Phase 5 — Complete
- Docs updated:
- Forge capture (aar/failures/rules/decisions):
- Ticket closed:
- Archived:
