# WORK-world-model-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #6 — model rooms, regions, entities, items v1 (the Rust
  domain model). Auto-approved: drive the full pipeline autonomously through commit.
- **Intake source:** none — forge ticket #6 + local ticket doc pre-existed.
- **Classification / tier:** work pipeline, `feature` — larger than #4/#5.
  **Decided: ONE cohesive pipeline** (not split). The five REQs share
  `WorldDefinition::validate`, the content loader, and `WorldDefinition`;
  splitting would re-touch the same files. Kept tractable by scoping each new
  model to its **minimal** shape (behaviors/contracts/flags deferred).
- **Forge recall:** Decisions 003 (rooms = entity containers), 004 (entities =
  shared data + role contracts + code-behind behaviors; NPCs/enemies are
  actors), 025 (square-grid map + passability). Design docs read: `entity-model.md`
  (base shape: id/type/name/description/aliases/attributes/placement/behaviors;
  actor roles; **code-behind = future**), `inventory-and-items.md` (rich item
  vision — slots/weight/rarity/flags/elements/behaviors all **future**; note line
  86 "items are entities and should use the shared entity model"). Prevention
  rules from #5 (PR-claude-mutation-arm-coverage-001) directly apply to the
  per-variant validation tests.
- **Current-state findings:**
  - `oathstar-core::RoomDefinition` already carries region/subregion (as plain
    `String`/`Option<String>`), passable, exits, x/y/z, glyph → **REQ-002 largely
    satisfied**; REQ-001 adds a region/subregion *registry* + ref validation.
  - `WorldValidationError` + `validate()` exist (StartRoomMissing/Impassable/
    DanglingExit/RoomKeyMismatch) → **REQ-005 extends** them with missing
    region/subregion/entity/item variants.
  - **No `Entity`/`Item`/`Region` types exist** anywhere yet (grep confirmed) →
    REQ-001/003/004 are net-new typed models.
  - `oathstar-content` loads `modules/beginner/{module,rooms}.toml` into
    `WorldDefinition` via `RoomToml`; v1 adds regions/entities/items TOML.
- **Open questions for Design:**
  - Is `Item` a separate struct or an `Entity` of kind `Item`? (entity-model
    lists Item as a type; REQ-003/004 are separate ACs.) Pick the leaner option.
  - Where do entities/items live — on `WorldDefinition` (registries keyed by id)
    and referenced from rooms, vs nested in rooms? (REQ-004 forbids inlining full
    item state into rooms.)
  - Minimal role representation for REQ-003 (e.g. `roles: Vec<Role>` /
    `BTreeMap<Role, metadata>`); validate only that referenced ids exist (not
    full role contracts).
  - Do `oathstar-protocol` snapshots need entities/items in v1? (Default: no —
    deferred; the model + validation are testable in core.)
  - Mutation cost: every new field/variant + each `validate` branch needs a
    distinct killing test — keep shapes minimal and use table-driven tests.
- **Ticket:** forge #6 `f4fe738e-ae33-42c8-8dcd-185fa724afab` (pre-existing,
  documented at `docs/planning/tickets/open/TICKET-6-...md`).
- **EARS reviewed:** REQ-001..005 carried from the ticket doc (verification
  sharpened); added REQ-006 (gate green incl. coverage + mutation floors).
- **AAR id:** `0cc650bc-287e-490b-8352-664fed5a4680` (inspect→failure-record, complete→aar-submit capture into it)

## Phase 2 — Design

### Resolved open questions
- **Item = separate pure-data struct** (not an `Entity` of kind Item). Placement
  is expressed by **containers referencing contents by id** ("model B'"), which is
  the only model that gives every REQ-005 ref-kind a real "missing X" path.
- **Registries live on `WorldDefinition`** (`BTreeMap<id, T>`); rooms/entities
  hold **id references**, never inlined structs (REQ-004).
- **Role rep = `roles: Vec<String>`** (declared tags). No role-contract validation
  in v1 (out of scope) — so roles add no validation/mutation surface.
- **Protocol snapshots: no change** (deferred) — the model + validation are fully
  testable in `oathstar-core`.
- **Player-owned items deferred** — v1 ownership is by an *entity* (`entity.inventory`);
  player inventory is runtime/gameplay (out of scope).

### Architecture / approach (all in `oathstar-core`, pure data + validation)
New types (pure data → ~0 mutation surface; the surface is in `validate()`):
```rust
pub struct RegionDefinition    { pub id: String, pub name: String }
pub struct SubregionDefinition { pub id: String, pub name: String, pub region: String }
pub enum   EntityKind          { Actor, Fixture }                       // serde snake_case
pub struct Entity { pub id, name, description: String, pub aliases: Vec<String>,
                    pub kind: EntityKind, pub roles: Vec<String>, pub inventory: Vec<String> }
pub struct Item   { pub id, name, description: String, pub aliases: Vec<String> }
```
`WorldDefinition` gains (all `#[serde(default)]`): `regions`, `subregions`,
`entities`, `items` (`BTreeMap<String, _>`). `RoomDefinition` gains (both
`#[serde(default)]`): `entities: Vec<String>` (entity ids placed here),
`items: Vec<String>` (item ids on the ground). Existing room metadata
(title/description/passable/exits/x,y,z,glyph) is **unchanged** (REQ-002).

**Reference graph → covers all 5 REQ-005 ref-kinds:** room→region, room→subregion,
subregion→region, room.exits→room (existing `DanglingExit`), room→entity,
room→item, entity→item. Entity *placement* = membership in `room.entities`; item
*room-placement* = `room.items`; item *ownership* = `entity.inventory` (REQ-004).

`Engine` logic is **unaffected** (it doesn't read the new fields); `Engine::try_new`
already calls `world.validate()`, so the new checks gate construction too.

### `validate()` additions (after the existing checks) + `WorldValidationError` variants
6 new typed variants, each naming the offender, each an existence check:
| Variant | Check |
|---|---|
| `SubregionRegionMissing{subregion_id, region}` | every `subregion.region` ∈ `regions` |
| `RoomRegionMissing{room_id, region}` | every `room.region` ∈ `regions` |
| `RoomSubregionMissing{room_id, subregion}` | `room.subregion` (if `Some`) ∈ `subregions` |
| `RoomEntityMissing{room_id, entity_id}` | every id in `room.entities` ∈ `entities` |
| `RoomItemMissing{room_id, item_id}` | every id in `room.items` ∈ `items` |
| `EntityItemMissing{entity_id, item_id}` | every id in `entity.inventory` ∈ `items` |
Each gets a `Display` arm (naming the offender, matching the existing style). **No
new key-mismatch variants** (existence-only — registries are built keyed-by-id by
the loader; key-integrity for the new registries is a future ratchet).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/lib.rs` | Add the 5 model types; add 4 maps to `WorldDefinition` + 2 vecs to `RoomDefinition` (all `#[serde(default)]`); add 6 `WorldValidationError` variants + `Display`; extend `validate()` with the 6 checks. Update test helpers (`test_world`/`world_with`/`room_with`) to build **valid** worlds (auto-register the regions their rooms use; empty subregions/entities/items). New `#[cfg(test)]` tests at Validate. |
| 2 | `crates/oathstar-content/src/lib.rs` | `RoomToml` += `entities`/`items` (`#[serde(default)]`); new `WorldToml` with `[[regions]]/[[subregions]]/[[entities]]/[[items]]` (serde default sections); `load_world_from_toml(module, rooms, world)` assembles the registries; keep duplicate-id checks; `validate()` runs as today. Tests at Validate. |
| 3 | `modules/beginner/world.toml` | **NEW** — 3 regions (hollowmere, ashen_road, old_bell_tower), 5 subregions (town/wall→hollowmere, wilds→ashen_road, tower/boss→old_bell_tower), 1 sample entity (Mara: Actor, roles conversable/shopkeeper, inventory=["candle"]), 1 sample item (candle). Makes the beginner world exercise every registry + the entity→item ref. `rooms.toml` unchanged (rooms already carry region/subregion strings). |
| 4 | notes (this file) | Phase 3/4 records. |
- **No `oathstar-protocol` change**; no JS change.

### Regression Test Plan (≥94% line + 100% MSI; the surface is `validate()` + loader)
| # | Test | Proves / kills |
|---|---|---|
| T1 | valid world with regions+subregions+entities+items → `validate()==Ok`; `world.regions.get(room.region)` resolves | REQ-001; kills the "always-Err" mutants on every new check |
| T2 | a loaded/constructed room carries title/description/passable/exits/x,y,z,glyph | REQ-002 |
| T3 | one `Entity` type expresses an NPC (kind Actor, roles `["conversable"]`), an enemy (Actor, `["combatant"]`), and an interactable (kind Fixture) | REQ-003; kills `EntityKind` usage |
| T4 | an item in `room.items` (placement) and an item in `entity.inventory` (ownership); rooms/entities hold only ids, `world.items` holds the data | REQ-004 |
| T5a–f | one test per missing-ref variant — world valid except a single broken ref → `Err(thatVariant)` (RoomRegion, RoomSubregion, SubregionRegion, RoomEntity, RoomItem, EntityItem) | REQ-005; kills each check's removal + `!contains_key` flip |
| T6 | each new `WorldValidationError` Display names the offending id (extend `validation_error_messages_name_the_offender`) | REQ-005 Display arms |
| T7 | `load_beginner_world()` → `Ok`; assert it has the 3 regions / 5 subregions / Mara / candle | loader assembly + beginner content valid |
| T8 | crafted-TOML loader test: a `world.toml` entity whose `inventory` names a missing item → loader returns the `EntityItemMissing` error (validation runs in the loader) | loader→validate wiring + REQ-005 e2e |
| T9 | existing 20 core + 4 JS tests still pass (helpers updated to valid worlds) | no regression |
- **Genuinely-uncoverable:** only the pre-existing `current_room().expect(...)`
  invariant. New data structs have no methods → no new uncoverable paths.

### Risks / decisions
- **R1 (load-bearing):** adding region validation means the **beginner world AND
  every existing core test helper** must register the regions their rooms use
  (`"test"` for helpers; the 3 real regions for beginner) or `validate()` rejects
  them. Mitigation: `world.toml` (beginner) + helper updates (auto-register). De-risked
  by reading `modules/beginner/rooms.toml` (3 regions / 5 subregions enumerated above).
- **R2 (model B' chosen for REQ-005 completeness):** containers reference contents
  so "missing entity" and "missing item" refs have real, testable paths. Trade-off:
  no single-location invariant (an id could appear in two containers) — not validated
  in v1 (REQ-005 is about *missing* refs, not duplicates); future ratchet.
- **R3 (minimal shapes):** roles are free string tags (no contract validation);
  `EntityKind` = {Actor, Fixture}; items are leaf data — behaviors, item
  flags/slots/weight/elements, role contracts, player inventory all deferred.
- **R4 (mutation):** the surface is the 6 `validate()` checks + `Display` + loader;
  each has a dedicated killing test (T5a–f valid+broken pairs, T6 Display, T7/T8
  loader). Data structs add ~0 mutants (no methods). Per PR-claude-mutation-arm-coverage-001.
- **R5 (serde compat):** all new fields `#[serde(default)]` so existing serialized
  worlds + the unchanged `rooms.toml` still deserialize.
- **Size:** the largest pipeline so far (6 validation variants + 5 types + loader +
  content). All mechanical data/validation; kept tractable by minimal shapes +
  table-driven tests.

## Phase 3 — Implement
- **Built (3 files; production code — full test suite is Phase 4):**
  - `crates/oathstar-core/src/lib.rs` — added `RegionDefinition`,
    `SubregionDefinition`, `EntityKind` (Actor/Fixture, serde snake_case),
    `Entity` (id/name/description/aliases/kind/roles/inventory), `Item`
    (leaf data); 4 `#[serde(default)]` registries on `WorldDefinition`; 2
    `#[serde(default)]` vecs (`entities`/`items`) on `RoomDefinition`; 6
    `WorldValidationError` variants (RoomRegion/RoomSubregion/SubregionRegion/
    RoomEntity/RoomItem/EntityItem-Missing) + `Display` arms; 6 existence checks
    merged into `validate()` (room loop + subregion loop + entity loop). Updated
    `test_world`/`room_with`/`world_with` to build valid worlds (register region
    `"test"`; `world_with` auto-registers its rooms' regions).
  - `crates/oathstar-content/src/lib.rs` — `RoomToml` += `entities`/`items`
    (default); new `WorldToml` deserialized **directly into the core types**
    (`Vec<RegionDefinition>`/etc.); `index_by_id` helper (dedupe→`BTreeMap`);
    `load_world_from_toml(module, rooms, world)` assembles all registries then
    `validate()`s; `load_beginner_world` includes `world.toml`. Updated the 4
    existing loader test call sites for the new arg.
  - `modules/beginner/world.toml` (NEW) — 3 regions, 5 subregions (correct
    parents), Mara (Actor; roles conversable/shopkeeper; `inventory=["candle"]`),
    candle item.
- **In-phase checks (green):**
  - `cargo check --workspace` clean; `cargo clippy --workspace --all-targets
    --all-features -- -D warnings` **clean** under strict lints.
  - `cargo test --workspace` — all pass: **oathstar-core 38**, **oathstar-content
    5** (incl. `beginner_world_loads`, which now exercises load→assemble→validate
    end-to-end with the new ref checks), oathstar-server 9, others 0.
  - **Load-bearing R1 confirmed:** the beginner world validates under the new
    region/subregion/entity/item checks (regions/subregions defined in world.toml;
    mara→candle ownership ref resolves).
- **Deviations from design:** none. `rooms.toml` left unchanged (Mara lives in the
  entity registry, unplaced — room placement of entities is a future ticket); the
  room→entity/room→item validation paths are covered by crafted unit tests at
  Validate rather than by beginner content.

## Inspect (Phase 3.5)
- **Lenses run:** 3 parallel `general-purpose` critics, each verifying concretely
  (compiled throwaway probes against the rlib, ran `cargo mutants --list`, grepped
  for hardcoded ids): (1) validation correctness/completeness/ordering, (2)
  mutation / test-plan completeness, (3) loader/serde/no-hardcoded-beginner/idiom.
  Verdicts: **VALIDATION-COMPLETE · TEST-PLAN-COMPLETE (0 must-add) · CLEAN.**
- **Findings:**
  | # | Severity | Finding | Verdict |
  |---|---|---|---|
  | 1 | LOW | No key-integrity (`key != value.id`) check for the 4 new registries (rooms have `RoomKeyMismatch`; regions/subregions/entities/items don't). | **REJECTED** — out of REQ-005 scope (that's *missing references*, not key integrity); the loader always builds keyed-by-id; documented future ratchet in the design. |
  | 2 | LOW | A room in region A referencing a subregion whose parent region is B is accepted (no cross-region consistency check). | **REJECTED** — REQ-005 rejects *missing* refs only; spec "Out" excludes consistency beyond "the id exists". Future ticket. |
  | 3 | (carry-forward) | The 6 new `validate()` checks + Display arms have no tests yet. | **EXPECTED** — Phase 4 deliverable (planned T5a–f / T6); not an implement defect. |
- **Verified-clean (critics ran these):** every REQ-005 ref-kind (region, subregion,
  room via `DanglingExit`, entity, item from BOTH `room.items` and
  `entity.inventory`) has a reachable, correct, panic-free check returning the
  right variant with accurate Display (probed each, incl. `subregion=Some("ghost")`
  → `RoomSubregionMissing` and a missing parent region → `SubregionRegionMissing`);
  existing checks still run first; serde-default compat holds (legacy worlds +
  unchanged `rooms.toml` + empty `world_src` all deserialize); `Entity.kind` is
  required → a clear `missing field 'kind'` error, not a panic; **zero hardcoded
  beginner ids** in core/content (grep: only in the `beginner_world_loads` test);
  `index_by_id` correct; beginner `world.toml` regions/subregions/parents match
  `rooms.toml` exactly; clippy strict clean; 38 core + 5 content tests pass.
- **Carry to Validate (from the critics):**
  - Each `T5a–f` world must be **valid except the single broken ref** so it reaches
    its target check (RoomRegion runs first in the room loop) — else the wrong
    mutant is exercised.
  - `T6` should assert **all 6** new Display arms — needed for the ≥94% *line*
    floor (one arm suffices for MSI: there's a single whole-`fmt`-body mutant).
  - A duplicate-id test for a new registry is **optional for MSI** (cargo-mutants
    generates no mutant on the `contains_key`/`bail!` dedup branch); include only
    as a correctness regression if desired.
- **Capture:** no `failure-record` — no code defect found; the only finding is a
  Phase-4 test deliverable already in the plan. Two LOWs are documented
  out-of-scope deferrals.

## Phase 4 — Validate
- **Tests added (14 new, from the Phase 2 plan + inspect carry-forwards):**
  - `oathstar-core` (11): `model_world` helper + per-type builders;
    `model_world_is_valid_and_refs_resolve` (T1), `room_exposes_metadata` (T2),
    `one_entity_type_carries_role_metadata` (T3, Actor+Fixture+roles),
    `items_are_referenced_by_room_and_owner` (T4), the 6 `rejects_missing_*`
    tests (T5a–f — each a **valid-except-one-broken-ref** world so it reaches its
    target check), `new_validation_errors_name_the_offender` (T6, **all 6** Display
    arms for the line floor).
  - `oathstar-content` (3): `beginner_world_has_regions_entities_items` (T7),
    `load_rejects_missing_item_reference` (T8, dangling ref e2e through the loader),
    `load_rejects_duplicate_region_id` (T8b, covers `index_by_id`'s dedup).
- **`cargo test --workspace`:** all pass — **oathstar-core 49** (+11),
  **oathstar-content 8** (+3), oathstar-server 9, others 0.
- **`node --test tests/*.test.js`:** **4 pass; 0 fail**.
- **`bin/gate.sh` (FULL): `GATE GREEN [full]` — 17 passed, 0 failed.**
  - gate:17 mutation **60 caught / 0 missed → MSI 100.0%** (the 6 new `validate()`
    `!contains_key` checks + Display body, all killed by T5a–f / T6).
  - gate:15 rust coverage **96.92% line ≥ 94**.
  - gate:14 tauri shell PASS; clippy strict + all static gates PASS.
- **One fix during Validate:** the first FULL run was RED on **gate:1 rustfmt**
  only (my edits needed rustfmt's line-wrapping on a few struct variants / `write!`
  / a test call). Applied `cargo fmt --all` (formatting-only, no logic change),
  re-ran → GREEN. (Lesson: run `cargo fmt --all --check` during Implement, not
  just clippy.)
- **Pre-existing exclusions:** `fn main` (mutation, `.cargo/mutants.toml`);
  `oathstar-server/src/main.rs` per-file coverage — both pre-existing, not in scope.
- **All AC verified:** REQ-001 ✓ REQ-002 ✓ REQ-003 ✓ REQ-004 ✓ REQ-005 ✓ REQ-006 ✓.

## Phase 5 — Complete
- **Docs updated:** `docs/mechanics-and-systems.md` (Rooms And World Model + Entity
  Model sections) got "v1 implemented" notes; `docs/entity-model.md` +
  `docs/inventory-and-items.md` each got a top status note pointing to the
  implemented `oathstar-core` subset. `decisions.md` 003/004/025 left as the locked
  vision (no change).
- **Forge capture:**
  - `aar-submit` **0cc650bc** — completed, effectiveness 5, 2 novel findings
    (distillation / confidence-drift / pattern-emergence jobs enqueued).
  - `architecture-decision-record` **AD-claude-world-model-v1-001** (109184b7) —
    containers-reference-contents ("model B'") + minimal shapes + reference
    validation; documented deferrals (key-integrity, cross-region, behaviors,
    contracts, item flags/slots, player inventory).
  - `prevention-rule-record` **PR-claude-fmt-in-implement-001** (bec88c5e) — run
    `cargo fmt --all --check` at Implement, not just clippy (a fmt-only RED bounced
    the first Validate FULL gate).
  - No `failure-record` — inspect found no defect; the fmt-only RED is captured as
    the prevention rule above.
- **Ticket closed:** forge #6 (f4fe738e) → `done`; local doc moved open/→closed/.
- **Archived:** pipeline doc pair moved active/→completed/.
