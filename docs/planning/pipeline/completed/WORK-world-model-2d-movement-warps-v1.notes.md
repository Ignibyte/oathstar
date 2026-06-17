# WORK-world-model-2d-movement-warps-v1 — Notes

## Phase 1 — Plan
- **Request:** the world-model redesign (ticket #52, build order 4 of 4, LAST). Drop
  up/down, make movement/rooms 2D, add region warps (cross-boundary cardinal exits),
  migrate the beginner world, amend locked Decision 025. ENGINE + content change.
- **Owner forks resolved (AskUserQuestion, this session):**
  1. **Scope:** "2D + warps **together**" — slice 1 includes the warp mechanism +
     the Bell Tower migration (not 2D-only-first).
  2. **Warp model:** "a **special exit**" — a cardinal exit whose target is in
     another region/sub-region (not a distinct entrance/portal object).
- **Already decided (program Q&A):** movement/rooms 2D; tile layers keep `z` for
  visuals only.
- **Key discovery (this phase):** the beginner world USES up/down — 6 exits:
  `hollowmere_square↔bell_frame` (both hollowmere/town), the tower climb
  `tower_foot↔tower_landing↔bell_eater_roost` (old_bell_tower; roost is sub-region
  `boss`). The cross-region step `ashen_road --N--> tower_foot` is **already** a
  cardinal exit crossing a region boundary → it IS the warp. So the migration:
  within-region up/down → cardinal (N/S); the cross-region/sub-region cardinal
  exits become the warps (engine detects the boundary + emits a transition).
- **Forge recall (pre-flight green; no bulletins):** the directions/movement
  architecture-decisions surfaced; Decision 025 (locked) is the one amended.
  Heeding the gated/marker/mutation rules where JS/engine tests touch them.
- **Architecture (Explore-confirmed earlier + this session; Design re-Explores the
  full blast radius):** `Direction` enum `command.rs:16-22` (N/S/E/W/Up/Down) +
  aliases/labels; `move_direction` `oathstar-core/src/lib.rs ~2501`;
  `WorldDefinition`/`RoomDefinition` carry region + `Option<subregion>` + `exits:
  BTreeMap<String,String>` + x/y/z; `MapDocument`/`materialize` copy z/floors;
  `src/world.js` direction maps; beginner world `modules/beginner/*.toml`.
- **Scope OUT:** studio warp-authoring UI (deferred); region-standing; 3D rendering.
- **EARS reviewed:** REQ-001..007 — drop up/down, warp transition, 2D + layer-z,
  warp validation, beginner migration, Decision 025 amendment, gate green.
- **Size note:** the biggest slice of the program — engine + content + JS + many
  tests + a locked-decision amendment. Design does a thorough Explore + designs the
  warp transition, the z/floors field treatment, and the exact migration.
- **Ticket:** #52 `32db0cc9-0176-45a4-a599-6a6a37ff8c18` (feature, exists).
- **AAR id:** 8111ccd2-1c3e-4953-bad9-0388c08a6a83

## Phase 2 — Design

### Blast radius (Explore — file:line)
- **Direction** (`oathstar-core/src/command.rs`): `Up`/`Down` in the enum (16-23),
  `from_token` (34-35), `as_str` (48-49) + 4 tests (339-342, 354, 362, 490-491).
  The **only** exhaustive `match` on `Direction` is `as_str` — no other workspace
  match breaks. ✓
- **move_direction** (`oathstar-core/src/lib.rs:2501`): resolves the exit, checks
  passability, sets `current_room_id`, emits `RoomEntered` + `describe_current_room`.
  **Never reads `z`.** `GameEventKind` enum is in `oathstar-protocol/src/lib.rs:398`.
- **z/floors:** `RoomDefinition.z` (read only in `room_snapshot:3354` + the wire +
  `awareness::Position.same_plane`); `RoomCell.z`/`TerrainCell.z` (materialize/
  validation); **`LayerCell.z` is separate → KEEP** (visual); `MapDocument.floors`
  (layer/terrain/room z-bounds); `awareness same_plane` gates by region+subregion+z;
  wire `RoomSnapshot.z`/`MapRoomSnapshot.z`; JS `map.js` filters rooms by z-plane.
- **Exit validation** (`map_document.rs:731`): dangling-exit check
  (`room_ids.contains(target)`) — **no cross-region check today; cross-region exits
  already work.**
- **JS:** `world.js` aliases/labels (up/u, down/d); `room.js` `CANONICAL_DIRECTIONS`
  (6 incl up/down); `map.js` z-plane filtering; `canvas-map.js` floor label;
  `tests/client.test.js` (up exits, the 6-direction pad, MOVEMENT_COMMANDS).
- **Beginner** (`modules/beginner/rooms.toml`): 6 up/down exits; the stacked rooms
  `bell_frame`(0,0,z1), `tower_landing`(0,-3,z1), `bell_eater_roost`(0,-3,z2).
- **Decision 025** (`docs/decisions.md:1173-1209`, Locked) — **NOT in the online-first
  WIP** (that diff starts ~1675). Safe to amend.

### Approach — MINIMAL-BLAST (keep z vestigial) [needs owner OK]
- **Remove Up/Down** from `Direction` (enum/`from_token`/`as_str`) + `world.js` +
  `room.js` `CANONICAL_DIRECTIONS` + the direction tests. Movement is cardinal-only.
- **move_direction:** when the next room crosses a boundary
  (`next.region != cur.region || next.subregion != cur.subregion`), push a transition
  **`LogMessage`** ("You enter {region/sub-region name}.") before `RoomEntered`
  (name from `world.regions`/`world.subregions`). **Uses the existing `LogMessage`
  variant — no protocol/client change.** (A structured `RegionTransition` variant is
  a deferred enhancement.)
- **KEEP the `z` fields (all 0).** `move_direction` never used `z`, so removing
  up/down + flattening every room to one z-plane achieves 2D movement **without** the
  serde/wire/awareness/JS-client/fixture churn of field removal. `same_plane` is
  unchanged (all rooms z=0 → the z check is always true; no behavior change). Tile
  `LayerCell.z` keeps z. **Full z-field removal is a deferred cosmetic cleanup.**
- **Beginner migration** (`rooms.toml`) — flatten the 3 stacked rooms to z=0 with
  distinct 2D coords + cardinal exits:
  - `bell_frame` (0,0,z1) → **(-1,0,z0)**; `hollowmere_square` `west`↔`bell_frame`
    `east` (square's south/west were free).
  - `tower_landing` (0,-3,z1) → **(0,-4,z0)**; `tower_foot` `north`↔`tower_landing`
    `south`.
  - `bell_eater_roost` (0,-3,z2) → **(0,-5,z0)**; `tower_landing` `north`↔
    `bell_eater_roost` `south`.
  - The existing `ashen_road`(0,-2) `--north-->` `tower_foot`(0,-3) is the **region
    warp** (ashen_road→old_bell_tower); `tower_landing` `--north-->`
    `bell_eater_roost` is the **sub-region warp** (tower→boss). Both emit transitions.
- **Amend Decision 025** — a targeted edit (status note + body: cardinal-only,
  up/down retired in favour of cardinal + region warps, z is tile-layer-only) +
  an `architecture-decision-record` AD-claude-2d-movement-warps at complete.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/command.rs` | remove `Up`/`Down` (enum/`from_token`/`as_str`); update the 4 direction tests. |
| 2 | `crates/oathstar-core/src/lib.rs` | `move_direction`: emit the cross-boundary transition `LogMessage`; + tests (a region-crossing move narrates the transition; a within-region move does not). |
| 3 | `src/world.js` | drop up/u + down/d from `directionAliases` + `directionLabels`. |
| 4 | `src/client/room.js` | drop up/down from `CANONICAL_DIRECTIONS` (now 4). |
| 5 | `modules/beginner/rooms.toml` | flatten the 3 stacked rooms (z=0 + 2D coords) + rewrite the 6 up/down exits to cardinal. |
| 6 | `docs/decisions.md` | amend Decision 025 (outside the online-first WIP). |
| 7 | `tests/client.test.js` | 4-direction pad (no up/down), the migrated cardinal exits, MOVEMENT_COMMANDS. |

`map_document.rs` needs **no source change** (z fields stay; a cross-region exit is
already valid) — only a warp test. `awareness.rs`/the wire/`map.js`/`canvas-map.js`
are **untouched** (z=0 everywhere is a no-op for them).

### Regression Test Plan
| # | Test | Proves Req |
|---|---|---|
| RT-1 | `command`: `"up"`/`"u"`/`"down"`/`"d"` no longer parse to a Move; the enum/`as_str` cover only N/S/E/W | REQ-001 |
| RT-2 | `node`: `directionAliases`/`directionLabels` have no up/down; the exit pad exposes 4 directions | REQ-001 |
| RT-3 | `engine`: a move across a region boundary (`ashen_road→tower_foot`) yields a transition LogMessage naming "The Old Bell Tower"; a within-region move (e.g. square→candle_shop) yields none | REQ-002 |
| RT-4 | `content`: the beginner world has all rooms at z=0 and no up/down exit key (2D) | REQ-003 |
| RT-5 | `content`: a cross-region exit validates when its target exists; a dangling one is refused (existing check) | REQ-004 |
| RT-6 | `content`: the beginner world materializes + validates; a tower-climb traversal (square→…→roost) works via cardinal/warp with no orphaned exit | REQ-005 |
| RT-7 | doc check: Decision 025 amended (cardinal-only + warps; up/down retired) | REQ-006 |
| RT-8 | `bin/gate.sh` FULL green incl mutation 100% MSI | REQ-007 |

**Genuinely uncoverable:** none — engine/content/JS all testable.

### Risks / decisions
- **KEEP-z vs REMOVE-z (the load-bearing call):** **OWNER CONFIRMED — keep**
  (vestigial z=0). Full removal touches core+content+protocol+awareness+JS-client+~15
  tests for zero behavior gain (z was never used for movement). Field-removal is a
  deferred cosmetic cleanup.
- **Transition as `LogMessage` vs a new `RegionTransition` event** — recommending the
  `LogMessage` (no protocol/client churn); structured variant deferred.
- **Decision 025 edit is safe** (outside the online-first WIP hunks) — but
  `decisions.md` is otherwise dirty with WIP, so selective staging must take ONLY the
  Decision-025 hunk. (Confirm at commit.)
- The flattened tower coords are a judgment call (a vertical 2D corridor going north).

## Phase 3 — Implement
- **Built (the manifest):**
  - `command.rs`: removed `Up`/`Down` from the `Direction` enum, `from_token`
    (up/u, down/d), `as_str`; updated the 4 direction tests to cardinal.
  - `lib.rs` `move_direction`: a cross-region OR cross-sub-region move emits a
    `LogMessage` ("You enter {name}.", `OutputComponent::NarrativeMessage`,
    `EventChannel::Room`) **before** `RoomEntered`; the name is resolved from
    `world.regions`/`world.subregions` into an owned `Option<String>` first, so the
    later `self.log` borrow is clean.
  - `src/world.js` + `src/client/room.js`: dropped up/down from
    `directionAliases`/`directionLabels`/`CANONICAL_DIRECTIONS` (now 4 cardinals).
  - `modules/beginner/rooms.toml`: flattened the 3 stacked rooms to z=0 with 2D
    coords (`bell_frame` (-1,0), `tower_landing` (0,-4), `bell_eater_roost` (0,-5))
    and rewrote the 6 up/down exits to cardinal — the tower climbs **north**,
    `bell_frame` sits **west** of the square. `ashen_road --N--> tower_foot` is the
    region warp; `tower_landing --N--> bell_eater_roost` the sub-region warp.
  - `docs/decisions.md`: amended Decision 025 (status note + struck up/down from the
    direction list + an Amendment paragraph) — **only** the 025 block.
  - `tests/client.test.js`: 4-direction exit pad, cardinal sample exits,
    `MOVEMENT_COMMANDS`.
- **Deviations / extra work (required, not in the manifest):**
  1. **`crates/oathstar-server/src/main.rs` playthrough tests** — 7 navigation
     sequences retargeted (the tower climb up→north, descent down→south). PLUS one
     **re-subscribe** in `played_progression_reaches_level_three`: the new warp
     narration events (emitted during the climb) overflowed the burst-draining
     `broadcast` receiver by exactly one (`Lagged(1)`) — re-subscribing `rx` before
     the boss fight starts its drain fresh. **Not a real-gameplay bug:** SSE clients
     drain the stream continuously and never accumulate; the test's fight→navigate→
     fight burst pattern is the artifact.
  2. **Kept all `z` fields** (owner-confirmed). `awareness.rs` `same_plane`, the wire
     `RoomSnapshot.z`, `map.js` z-plane filtering, `canvas-map.js` floor label are
     **untouched** — with every room at z=0 they are no-ops (proven by the green
     workspace + node suites).
- **Checks:** `cargo fmt`; `cargo clippy` clean (core/content/server, `--tests`);
  `cargo test --workspace` **527 passed, 0 failed**; `node --test` **77 pass**;
  `node --check` world.js/room.js OK. The transition test (RT-3) is written at
  Validate per the design.

## Inspect (Phase 3.5)

4 adversarial critics over the diff (transition+mutation / world-graph / re-subscribe+keep-z+scope / JS-parity). Each verified concretely (read code, ran `cargo mutants`, parsed the TOML, ran node). Ledger:

**Critic A — transition correctness + mutation-readiness (the gate-critical lens):**
- **[HIGH — REAL — FIXED] 2 surviving mutants** at `lib.rs:2530:46` and `:2537:39` (`replace != with ==` on `next_room.region != room.region` and `next_room.subregion != room.subregion`). Root cause: every pre-existing movement test walks within one region/subregion (`room_with` defaults `region:"test", subregion:None`), so the live transition was always `None` and nothing asserted on warp narration (`grep "You enter"` → only the production string). **Fix: wrote RT-3** — a 4-room `warp_world()` fixture (regions r1/r2, sub-regions s1/s2 under r1; display names ≠ ids to also pin the registry name lookup): `warp_across_region_narrates_region_name` (east → "You enter Far Region."), `warp_across_subregion_narrates_subregion_name` (south → "You enter The Deep."), `move_within_region_and_subregion_does_not_narrate` (north → no "You enter"). The negative test is what kills both `==` mutants. **Re-ran `cargo mutants -p oathstar-core --file lib.rs --re move_direction`: `5 mutants tested: 4 caught, 1 unviable` — 0 missed.** (The 1 unviable is `vec![Default::default()]` — GameEvent has no Default.)
- [LOW — REAL — accepted, no fix] `Some→None` subregion (entering an unlabelled sub-area in the same region): `!=` true but `.map` on `None` → `None` → no narration. Sound (no panic; narrating a bare id would leak an internal token). RT-3c documents the no-narration intent.
- [LOW — rejected] registry-miss fallback + borrow soundness: confirmed clean (`map_or_else` → raw-id fallback, no unwrap; `room`/`next_room` owned clones so `self.world` borrows release before `&mut self`). No action.
- command.rs (4-variant `from_token`/`as_str`): `cargo mutants --file command.rs` = `40 tested: 35 caught, 5 unviable, 0 missed` — the alias tuples test already pins it. No action.

**Critic B — world-graph integrity (`rooms.toml`):** ALL 7 checks PASS (Python TOML parse + grep). No up/down keys; 8 rooms with unique `(x,y,z)`; every exit resolves; the migrated tower/bell edges are all bidirectional; tower reachable from hollowmere_square via a 5-hop north climb (`square→north_gate→ashen_road→tower_foot→tower_landing→bell_eater_roost`); both warps cross boundaries (region `ashen_road`→`tower_foot`; sub-region `tower_landing[tower]`→`bell_eater_roost[boss]` within `old_bell_tower` — region branch correctly skipped first so the sub-region narration fires); every cardinal direction agrees with its coordinate delta (north = −y). No action.

**Critic C — re-subscribe / keep-z / scope:**
- [VERDICT: re-subscribe LEGIT, not masking] real SSE handlers (`events_json` main.rs:241, `events_datastar` :291) drain continuously (`loop recv().await → yield`), so they can't lag; the test channel is cap 16 vs prod 256 and the test parks `rx` then burst-drains at the boss fight. Sibling test `beginner_slice_plays_the_focus_economy` uses the identical drop/re-subscribe idiom. No action.
- [VERDICT: keep-z confirmed no-op] `awareness.rs:58 same_plane` z-term is invariantly true at z=0; movement follows exit ids (no z arithmetic); JS `map.js` plane filter collapses to `[0]`. Prior non-zero z values had no behavioral dependents and rooms were re-laid so no x,y collision. No action.
- [VERDICT: scope CLEAN] no online-first file modified by #52; `decisions.md` diff confined to the Decision 025 block. **Carried to /commit:** stage `decisions.md` by explicit hunk/path only (the file co-resides uncommitted online-first WIP 035/050/054–059) — never `git add` the whole file.
- **[MED — REAL — FIXED] stale player-facing direction strings** (left by the up/down removal, none test-pinned so they shipped green): oathstar-core Help text `lib.rs:1472` ("…west, up, down, swear…") and Mara's sworn dialogue `world.toml:64` ("north, then up"). Fixed both.

**Critic D — JS direction parity:** tables clean & consistent (world.js + room.js agree on the 4 cardinals + n/s/e/w aliases; no opposite-map or compass orphan; node 18/18; exit-pad `deepEqual` positively fails if up/down reappear). **[HIGH — REAL — FIXED]** `src/engine.js:122` move() error still read "north, south, east, west, up, or down." — a live contradiction (typing `up` now hits exactly that message). Fixed. Plus doc-comment residue `room.js:91-93` ("six canonical directions… `up`") → fixed to "four cardinal… `west`", and `map_document.rs:117` (`"north"`…`"down"`) → `"…west"`. [MED — coverage] no JS test pins the parser/alias vocabulary → **deferred to Validate**: add an assertion that `directionAliases.up`/`.down` are undefined (also guards the engine.js regression).

**Fixes applied at inspect:** RT-3 (3 engine tests + `warp_world`/`entered` helpers, kills the 2 mutants — proven) · 5 stale strings (`lib.rs:1472` help, `map_document.rs:117` doc, `engine.js:122` error, `room.js:91-93` doc, `world.toml:64` dialogue). **Verify:** `cargo test --workspace` = **530 passed / 0 failed** (527 + 3 RT-3); `node --test` = **77 pass**; clippy (core+content) clean; mutants(move_direction) 0 missed.

**Forge:** BF-claude-transition-narration-untested-001, BF-claude-stale-direction-vocab-strings-001; PR-claude-boundary-event-needs-negative-assertion-001, PR-claude-pin-removed-vocab-strings-001 (aar_id 8111ccd2).

**Carried to Phase 4 — Validate:** the RT-1..8 plan (RT-3 already done here) + the JS alias-absence assertion (Critic D MED); then full `bin/gate.sh`.

## Phase 4 — Validate

**Tests written/confirmed (one+ per AC):**
- **RT-1 (REQ-001 Rust):** `command::tests::retired_up_down_tokens_do_not_parse_to_move` — added; `up`/`u`/`down`/`d` parse to `Unknown`, not `Move`. (The positive `every_direction_alias_parses_to_move` pins the 4 cardinals.)
- **RT-2 (REQ-001 JS):** added to `tests/game.test.js` — `direction vocabulary is cardinal-only: no up/down/u/d` (asserts `directionAliases`/`directionLabels` from world.js have no up/down/u/d and resolve exactly the 4 cardinals) + `'up' is not a movement direction and does not move the player` (behavioral). Closes the Critic-D coverage gap and guards the engine.js stale-prompt regression.
- **RT-3 (REQ-002):** `lib.rs` `warp_across_region_narrates_region_name` / `warp_across_subregion_narrates_subregion_name` / `move_within_region_and_subregion_does_not_narrate` (written at Inspect; kills the 2 transition mutants — proven 0-missed).
- **RT-4 (REQ-003):** `oathstar-content` `beginner_world_is_two_dimensional` — added; every beginner room z=0 + no up/down exit key. (LayerCell.z retention for tile visuals confirmed at `map_document.rs` `layers_do_not_affect_materialization` — untouched #47/#48.)
- **RT-5 (REQ-004):** `map_document::tests::refuses_dangling_exit` (pre-existing) + `validate_rejects_dangling_exit` — a cross-boundary exit to a missing room is refused.
- **RT-6 (REQ-005):** `oathstar-content` `beginner_tower_climbs_north_via_cardinal_warps` — added; a successful `load_beginner_world()` is a materialize+validate, the climb `hollowmere_square→…→bell_eater_roost` is a straight north run, and the two warps cross a region (ashen_road→tower_foot) + a sub-region (tower_landing→bell_eater_roost) boundary. Traversal also covered end-to-end by the server playthrough `played_progression_reaches_level_three`.
- **RT-7 (REQ-006):** Decision 025 amended in `docs/decisions.md` (Status line `Amended (ticket #52…)` + the Amendment paragraph). Doc check.

**Run results (real output):**
- `cargo test --workspace` = **533 passed / 0 failed** (527 baseline + RT-1 + RT-4 + RT-6 in Rust; RT-3's 3 were counted at Inspect). Per-crate: auth 20, content 78, core 300, datastar 16, protocol 27, server 34, storage 20, studio 38.
- `node --test tests/*.test.js` = **79 pass / 0 fail** (77 baseline + RT-2's 2).
- `bin/gate.sh` FULL: gates 1–14 GREEN (fmt, clippy `-D warnings`, cargo test, node, audit, deny, machete, gitleaks, shellcheck, no-suppressions, source-bans, lints-allowlist, doc-todos, tauri-shell). gate:15 rust coverage **98.47% lines** (≥94). gate:16 js coverage **89.44%** (≥75). gate:17 mutation **540 caught / 0 missed → MSI 100.0%** (floor 100). **`GATE GREEN [full]` — 17/17 passed, 0 failed** (FULL green wrote the commit-gate receipt).

**Pre-existing exclusions:** none in scope. (`oathstar-studio/src/main.rs` shows 0% line coverage — `main()` is the gate's excluded entrypoint, pre-existing, not #52.)

## Phase 5 — Complete
