# WORK-map-document-model-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #43 — define the Oathstar-native map document model the
  studio `/admin/editor` edits and the server validates/materializes into world
  data. Owner steer: set it up to use 16×16-px tiles.
- **Intake source:** docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md
  (section C — Editor Document Model). Intake left **unedited** — it is in the
  owner's preserve set and #41/#42 likewise left it; promotion is recorded here +
  in the ticket doc + forge.
- **Classification / tier:** work pipeline, type=feature, one shippable slice — a
  Rust data model + `validate`/`materialize` seam + typed errors + serde + tests/docs.
- **Forge recall (lessons/failures surfaced):**
  - The existing map model is a room-graph **snapshot DTO** — `MapSnapshot` /
    `MapRoomSnapshot` (oathstar-protocol src/lib.rs:251–284), built by
    `Engine::map_snapshot` (oathstar-core). Rooms carry
    id/title/x/y/z/glyph/passable/discovered/current/exits + additive
    `hasHostiles`/`hasItems` (omit-when-false). #43's authoring document is a
    **separate** artifact that materializes *into* world data, not this wire DTO.
  - Tile-size history: tileset source was 16px (#36) → 32px (#37), rendered at a
    configurable cell size (default 32; "8/16/32/larger" — map-system.md:82-84).
    The name-keyed tileset decouples source resolution from map/world data
    (map-system.md #32 notes; intake "Tile Art Direction"). The **16×16 steer
    fixes the authoring unit, not the render scale.**
  - Renderer-agnostic JSON is the standing guardrail (Decisions 025/035;
    map-system.md "Backend Payload"/"Design Guardrails"); additive-field precedent
    from #33/#38.
  - `oathstar-content` crate **exists** — a natural home candidate (design decides).
- **Ticket:** forge `934b2799-2b92-45b1-aaf6-3a9164cafd29` (#43); local
  docs/planning/tickets/open/TICKET-43-oathstar-map-document-model-v1.md.
- **EARS requirements reviewed:** REQ-001..006 finalized in the spec — the ticket's
  candidate REQ-001..005 plus an explicit 16×16 source-unit (REQ-002) and a
  lossless serde round-trip (REQ-006).
- **AAR id:** 5ff9acc4-fd65-4e9d-877b-0f15899a8387

## Phase 2 — Design

**Materialization target (Explore sweep under aar 5ff9acc4):** the engine's
in-memory world is `oathstar_core::WorldDefinition` (core/src/lib.rs:22-43) —
`{ id, title, start_room_id, rooms: BTreeMap<String,RoomDefinition>, regions,
subregions, entities: BTreeMap<String,Entity>, items: BTreeMap<String,Item>,
oaths, oath_id }`. `RoomDefinition` (lib.rs:46-70):
id/title/region/subregion?/description/exits(`BTreeMap<dir,room_id>`)/x/y/z:i32/
glyph:char/passable/entities:`Vec<id>`/items:`Vec<id>`/combat_enabled.
`Engine::try_new(world)` runs `WorldDefinition::validate()` →
`WorldValidationError` (hand-rolled enum, `impl Display + Error`, **no thiserror**)
then builds. Directions: `oathstar_core::command::Direction`
{North,South,East,West,Up,Down}. Existing authoring path: `oathstar-content`
TOML → WorldDefinition (`load_world_from_toml`). Saves are JSON via
oathstar-storage. **No Rust tile/grid/terrain type exists — #43 introduces it.**

### Approach / architecture
A NEW authoring artifact in **`oathstar-content`** (already the content→
WorldDefinition crate; depends on oathstar-core), beside the TOML loader as a
second authoring→world path (intake "Canonical Map Authoring"). All types derive
`serde`, are renderer-agnostic, additive (`skip_serializing_if` on optionals):

- `MapDocument { id, title, tile_size:u32(=16), width:u32, height:u32, floors:u32,
  terrain_palette: BTreeMap<String,TerrainDef>, terrain: BTreeMap<Cell,String>,
  regions: BTreeMap<String,MapRegion>, subregions: BTreeMap<String,MapSubregion>,
  rooms: BTreeMap<Cell,RoomCell>, spawn: Option<Cell> }`
- `Cell { x:u32, y:u32, z:u32 }` — tile-grid coord, `Ord` ⇒ deterministic
  iteration. The 16px unit is `tile_size`; **geometry is in tile units, never
  pixels** (REQ-002).
- `TerrainDef { tile:String (tileset tile name), passable:bool }` — name-keyed,
  carries collision/passability.
- `RoomCell { id:String (stable), title:Option<String>, description:Option<String>,
  region:String, subregion:Option<String>, glyph:Option<char>, combat_enabled:bool,
  exits: BTreeMap<String,String> (dir→room_id; up/down = stairs), entities:Vec<String>,
  items:Vec<String>, fixtures:Vec<String> }`
- `ContentCatalog { entities: BTreeMap<String,oathstar_core::Entity>,
  items: BTreeMap<String,oathstar_core::Item>, fixtures: BTreeSet<String> }` — the
  registry refs validate against (entities/items also supply materialized defs;
  fixtures are id-only in v1).
- `RefKind { Entity, Item, Fixture }`.
- `MapValidationError` — hand-rolled `enum` + `impl Display + Error` (matches
  WorldValidationError; **not thiserror**). Variants (1:1 with REQ-004 classes +
  structural siblings): `UnsupportedTileSize{found}`, `MissingRegion{cell,room_id}`,
  `UnknownTerrain{cell,name}`, `CellOutOfBounds{cell}`,
  `RoomOnBlockedTerrain{cell,room_id,terrain}`, `DuplicateRoomId{cell,room_id}`,
  `DanglingExit{room_id,direction,target_room_id}`,
  `UnknownReference{room_id,cell,kind:RefKind,id}`, `NoSpawnPoint`,
  `SpawnNotARoom{cell}`.

Seams mirror `Engine::try_new` (validate-then-build; typed errors; no panics on
input paths — §14):
- `MapDocument::validate(&self, catalog:&ContentCatalog) -> Result<(),MapValidationError>`
  — tile_size==16; every terrain/room cell within w×h×floors; room terrain passable;
  unique room ids; exit targets resolve to a room id; entity/item/fixture refs ∈
  catalog; each room has a region; a spawn exists and lands on a room cell.
- `MapDocument::materialize(&self, catalog) -> Result<WorldDefinition,MapValidationError>`
  — `validate()` first, then deterministically build: one `RoomDefinition` per room
  cell (x/y/z = Cell; passable from terrain; title/description **defaulted when None
  = "ordinary", overridden = "special"**; glyph default `'.'`; exits/entities/items
  copied; region/subregion carried), `start_room_id` = spawn's room, regions/subregions
  copied, `world.entities/items` copied from the catalog for referenced ids, `oaths={}`,
  `oath_id=None`. `BTreeMap` ordering ⇒ byte-identical output for identical input.
  Result feeds `Engine::try_new` unchanged (REQ-003).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | crates/oathstar-content/src/map_document.rs | NEW — MapDocument/Cell/TerrainDef/RoomCell/MapRegion/MapSubregion/ContentCatalog/RefKind/MapValidationError; `validate()` + `materialize()`→WorldDefinition; `#[cfg(test)]` unit tests T1–T8 |
| 2 | crates/oathstar-content/src/lib.rs | MODIFY — `mod map_document;` + `pub use map_document::{MapDocument, Cell, TerrainDef, RoomCell, MapRegion, MapSubregion, ContentCatalog, RefKind, MapValidationError};` |
| 3 | crates/oathstar-content/Cargo.toml | MODIFY if needed — ensure `serde` (derive) present; add `serde_json` dev-dep for the round-trip test |
| 4 | docs/map-system.md | MODIFY (Phase 5) — additive "Map Document Model (authoring)" section: 16px authoring unit, validate/materialize→WorldDefinition, renderer-agnostic |

### Regression Test Plan
| # | Test (in `oathstar-content`, in-crate) | Proves |
|---|---|---|
| T1 | `map_document_represents_all_facets` — build a doc with identity, w/h/floors, region/subregion, terrain+passability, room cells, exits, spawn, entity/item/fixture refs; assert each facet present | REQ-001 |
| T2 | `tile_size_is_sixteen_in_tile_units` — `tile_size==16`; cells are tile coords; no pixel/render field exists; `validate` rejects `tile_size!=16` → `UnsupportedTileSize` | REQ-002 |
| T3 | `materialize_yields_engine_constructable_world` — valid doc+catalog → WorldDefinition; `Engine::try_new(world)` Ok; rooms carry expected id/x/y/z/exits/passable | REQ-003 |
| T4 | `materialize_is_deterministic` — same (doc,catalog) materialized twice → serde_json byte-identical | REQ-003 |
| T5a | `validate_refuses_missing_region` → `MissingRegion` names the cell | REQ-004 |
| T5b | `validate_refuses_unknown_entity_item_fixture_ref` → `UnknownReference{kind,id,cell}` (one per RefKind) | REQ-004 |
| T5c | `validate_refuses_dangling_exit_and_oob_cell` → `DanglingExit` / `CellOutOfBounds` | REQ-004 |
| T5d | `validate_refuses_room_on_blocked_terrain` → `RoomOnBlockedTerrain{cell,terrain}` | REQ-004 |
| T5e | `validate_refuses_when_no_spawn` (and spawn-not-a-room) → `NoSpawnPoint` / `SpawnNotARoom` | REQ-004 |
| T6 | `ordinary_defaults_and_special_overrides` — ordinary room cell → default title/desc; special → overrides; both stable ids | REQ-005 |
| T7 | `map_document_round_trips_through_json` — `serde_json` to_string→from_str == original; no renderer-specific fields | REQ-006 |
| T8 | `unknown_terrain_and_duplicate_room_id_refused` → `UnknownTerrain`, `DuplicateRoomId` (structural siblings) | REQ-004 |

No genuinely uncoverable paths (pure Rust). Fixtures are validated (T5b) but not
materialized (no engine target) — materialize asserted to ignore them without error.

### Risks / decisions (resolved)
1. **Home crate = `oathstar-content`** — already the content→WorldDefinition crate;
   depends on oathstar-core; in-crate tests satisfy cargo-mutants (per the #42 lesson
   BF-studio-cross-crate-mutation-gap-001).
2. **Serialized form = JSON** (serde) — matches storage + intake's "JSON for
   canvas/map/editor data"; TOML stays possible via serde but isn't required for v1.
3. **Sparse grid** — `BTreeMap<Cell,_>` for terrain + rooms inside declared
   `width/height/floors`; rejects dense arrays (mostly-empty, huge); `Ord` ⇒ determinism.
4. **Materialize target = `WorldDefinition`** — rooms 1:1 with room cells;
   entities/items pulled from the provided `ContentCatalog`; `oaths` empty in v1.
5. **Exits explicit per room cell** (dir→room_id), uniform N/S/E/W/Up/Down (stairs =
   up/down exits) — maps 1:1 to `RoomDefinition.exits`.
6. **Fixtures modeled + validated but NOT materialized** (the engine has no fixture
   concept; an engine change is out of scope) — forward-looking per REQ-001; revisit
   when the engine grows fixtures.
7. **Ordinary vs special rooms = override presence** (`title`/`description` Option);
   ids always explicit/stable.
8. **`decisions.md` left untouched** — it holds the owner's uncommitted 056/057/058
   (preserve guardrail) and appending re-creates the #42 commit-entanglement; #43's
   architecture decision is captured in the forge (`architecture-decision-record`) +
   `docs/map-system.md` at Phase 5 instead.
9. **No change to `MapSnapshot`/`MapRoomSnapshot`, the engine, the server, or the
   client** — the document model and its seams are self-contained in oathstar-content.

## Phase 3 — Implement
- **Built** (production code only; tests are Phase 4):
  - `crates/oathstar-content/src/map_document.rs` (NEW, ~470 lines) — `MapDocument`,
    `Cell`, `TerrainDef`, `TerrainCell`, `RoomCell`, `ContentCatalog`, `RefKind`,
    `MapValidationError` (13 variants, hand-rolled `Display` + `Error` with
    `source()` for the wrapped engine error), `const SUPPORTED_TILE_SIZE = 16`.
    Seams: `validate(&ContentCatalog)`, `materialize(&ContentCatalog) ->
    WorldDefinition`, private `check()` (returns resolved start-room id) +
    `build_world()` + free `check_refs()`.
  - `crates/oathstar-content/src/lib.rs` — `mod map_document;` + `pub use` of the
    public surface.
  - `crates/oathstar-content/Cargo.toml` — added `serde_json` dev-dep (Phase-4
    round-trip test). `serde` (with derive) already present.
  - `cargo clippy -p oathstar-content --all-targets` **clean** under the strict
    workspace lints (after making the two `cell()` helpers `const fn`).
- **Deviations from design (+ reason):**
  1. **Terrain/rooms are `Vec<TerrainCell>` / `Vec<RoomCell>`, not
     `BTreeMap<Cell, _>`** — `serde_json` cannot serialize struct-keyed maps (would
     have broken the REQ-006 JSON round-trip); a Vec of cell-entries is JSON-native
     and Tiled-like. Internal `BTreeMap`s (`terrain_at`, `room_id_at`) are rebuilt
     in `check()` for lookups + deterministic iteration.
  2. **Reused `oathstar_core::RegionDefinition`/`SubregionDefinition` directly**
     (dropped the parallel `MapRegion`/`MapSubregion`) — identical shape, already
     serde, matches the existing `WorldToml` pattern; subregion parent-consistency
     is then enforced for free by the engine via the `WorldInvalid` net.
  3. **`materialize` runs `WorldDefinition::validate()` as a final net** →
     `MapValidationError::WorldInvalid(WorldValidationError)`. Guarantees the output
     is engine-constructable and catches subregion-parent / entity-inventory /
     role-contract invariants without re-implementing them. **`validate()` delegates
     to `materialize()`**, so `validate()==Ok ⟺ materialize()==Ok` (no drift).
  4. **All materialized rooms are `passable: true` by construction** — a
     non-passable cell is never a room (`RoomOnBlockedTerrain` rejects it), so no
     per-room passability lookup is needed (removes an unreachable default branch /
     mutation survivor).
  5. **`world.entities`/`items` = referenced closure** (placed entities + placed
     items + placed entities' inventory items), not the whole catalog — keeps the
     world minimal and stops unrelated catalog content (e.g. an unplaced entity with
     an unmet role contract) from failing materialization.
  6. **`check()` returns the resolved start-room id** — the single spawn lookup
     doubles as the `SpawnNotARoom` check, leaving no unreachable branch.
- **No engine/server/protocol/client change.** Unrelated owner worktree untouched.

## Inspect (Phase 3.5)
- **Lenses run** (3 parallel general-purpose critics + skeptical review): correctness;
  determinism + serde + API/§14; mutation-readiness (the #42
  BF-studio-cross-crate-mutation-gap lesson). Each verified by reading + `cargo
  check`/`clippy`.
- **Findings:**

  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | medium | Mutation survivors — unreachable `None` arms: `build_world` `filter_map(catalog.get)` for entities/items + the inventory `if let Some(get)`, and the `is_some_and` passability lookup (always-present key) | **REAL** — would miss `cargo-mutants` MSI=100% (gate RED at validate, the #42 class) | Refactored: `terrain_at` stores resolved `(name, passable)` via `get` (not `contains_key`); `entities`/`items` built by **iterating the catalog and filtering** the referenced set; inventory folded from the already-built `entities`. Every branch now reachable. |
  | 2 | medium | Subregion membership not doc-diagnosed — an undeclared `RoomCell.subregion` only surfaced via the opaque `WorldInvalid` net, without naming the cell (asymmetric with `RoomRegionMissing`) | **REAL** (REQ-004 "name the offender" spirit) | Added `RoomSubregionMissing { cell, room_id, subregion }` + check + Display, mirroring the region check. Subregion *parent-mismatch* stays with the engine net — a clean `WorldInvalid` test for Phase 4. |
  | 3 | low | OOB spawn reported as `SpawnNotARoom`, not `CellOutOfBounds` (check :spawn) | **REJECTED** — still names the offending cell (REQ-004 holds); "not a room" is accurate for an out-of-grid spawn; not worth an added branch. |
  | 4 | nit | Exit direction keys not validated against N/S/E/W/Up/Down → a typo'd key is a silently dead exit | **REJECTED (v1)** — matches the engine's opaque-exit behavior (it doesn't validate keys either); `Direction::from_token` is private; out of REQ-004's named classes. Editor (#43-D) guards via direction pickers. |
  | 5 | low | `check()` error *identity* is input-Vec-order dependent for a multi-fault doc | **NOT A DEFECT** — the success path (materialized world) is fully order-independent (REQ-003 holds). Phase 4 isolates single-fault docs (also a mutation-plan requirement). |

- **Verified FINE (no change):** `in_bounds` off-by-one correct on all three axes; zero
  `unwrap`/`expect`/`as`/index/overflow on input paths; the all-rooms-passable
  invariant is sound; REQ-001..006 hold; determinism (no `HashMap`/`HashSet`
  anywhere); serde round-trip safe (`Cell` is never a map key); `validate()==Ok ⟺
  materialize()==Ok`; all `RoomDefinition`/`WorldDefinition` fields set correctly;
  re-export surface complete.
- **Carried to Phase 4 (mutation kill-list):** `in_bounds` boundary matrix (0 / max-1 /
  max / -1 per axis, one axis perturbed at a time); one refusal **and** one `Display`
  assertion per `MapValidationError` variant (now 14, incl. `RoomSubregionMissing`) +
  the 3 `RefKind` Display arms; `passable: true`; `combat_enabled` with **both**
  values; `start_room_id` (distinct value); a `validate()`-on-bad-doc → `Err`; a
  `WorldInvalid` case via subregion parent-mismatch; the entity→inventory item
  pull-in; exact default constants (`Unnamed Room` / `An unremarkable area.` / `'.'` /
  16); catalog with an *unreferenced* entity (kills the new filter false-branch).
- **Post-fix:** `cargo clippy -p oathstar-content --all-targets` **clean**.

## Phase 4 — Validate
- **Tests added (28, in-crate `#[cfg(test)] mod tests` in `map_document.rs`):**
  - REQ-001 `represents_all_facets`. REQ-002 `supported_tile_size_is_sixteen` /
    `tile_size_sixteen_validates` / `wrong_tile_size_is_refused`.
  - REQ-003 `materialize_yields_engine_constructable_world` (`Engine::try_new` Ok) /
    `materialized_room_keeps_authored_coordinates` /
    `materialize_is_deterministic_regardless_of_input_order` (Vec-reversed → serde-equal).
  - REQ-004 one isolated refusal per variant: cell-OOB (terrain + room call sites) +
    `in_bounds_boundary_matrix`; unknown-terrain; duplicate terrain-cell + room-cell;
    missing-terrain; room-on-blocked; duplicate-room-id; region-missing;
    subregion-missing; dangling-exit (+ `forward_referenced_exit_validates`); unknown
    entity/item/fixture refs; no-spawn; spawn-not-a-room; and
    `refuses_world_invalid_on_subregion_parent_mismatch` (the engine `WorldInvalid` net).
  - REQ-005 ordinary-defaults vs special-overrides. REQ-006 JSON round-trip (+ asserts
    no renderer/pixel field).
  - Mutation pins: `passable:true`; `combat_enabled` both values; `start_room_id`;
    `validate()`-on-bad-doc; inventory item travels into `world.items`; unreferenced
    catalog entry excluded; full `Display` for all 14 variants + 3 `RefKind` labels +
    `Cell`; `source()` only for `WorldInvalid`.
- `cargo test --workspace`: **ok** — 296 (core) + 62 (content, incl. 28 new) + 34 +
  27 + 20 + 20 + 16 + 14; **0 failed** across all crates.
- `node --test tests/*.test.js`: **ok** — 67 pass / 0 fail.
- `bin/gate.sh` (FULL): **GATE GREEN [full]** — 17/17 PASS; mutation **MSI 100.0%
  (468 caught / 0 missed)**; rust coverage ≥94%; js coverage ≥75%; `REAL_GATE_EXIT=0`.
  (First run was RED on `gate:1` only — hand-written test code needed `cargo fmt`; the
  gate runs all 17 and collects failures rather than failing fast. Re-ran green.)
- Pre-existing exclusions: none — fully green.

## Phase 5 — Complete
- Docs updated:
- Forge capture (aar/failures/rules/decisions):
- Ticket closed:
- Archived:
