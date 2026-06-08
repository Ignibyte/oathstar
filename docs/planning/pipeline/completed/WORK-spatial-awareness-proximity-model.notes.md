# WORK-spatial-awareness-proximity-model — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Implement the foundation for Oathstar's "blast radius" spatial
  awareness — entities/items/fixtures/dialogue targets discoverable (and
  eventually interactable) by distance within the current subregion/z-plane
  instead of exact tile/room co-location. Server-authoritative, JSON-friendly,
  no canvas drawing instructions. (Ticket #17.)
- **Intake source:** none (ticket minted directly; doc
  `docs/planning/tickets/open/TICKET-17-spatial-awareness-proximity-query-model.md`).
- **Classification / tier:** work pipeline (one shippable foundation slice).
  Engine + protocol-DTO + a render-only client touch. No new crate; no breaking
  wire change (additive only).
- **Forge recall (lessons/failures surfaced):**
  - AAR `73d74d36-0726-4bbd-8e63-6d21e99519cd` opened; `knowledge-context` (Plan)
    logged 13 surfacings. Surfaced: architecture decisions `109184b7`,
    `1700e917`, `63ceb8db`; prevention rules (top `be0f3145`); distilled lessons
    `b7eea654`, `051347df`, `c851de70`; recent failures `deaa10b6`, `360ee9a3`.
    (Design phase will `knowledge-context` again for the build.)
  - `docs-search` (readable): Decisions **025** (square grid + cardinal/up-down),
    **034** (JSON reserved for renderer/structured data), **035** (first-party
    canvas, no game-engine dep), **031** (stable wire split), **030** (core-boundary
    validation), **003** (rooms as entity containers); `map-system.md` room
    metadata (`region/subregion/x/y/z`, passability); ticket #6 world model;
    ticket #16 canvas (client-only, `/state` map unchanged).
  - **Codebase grounding (Explore subagent):**
    - Crates: `oathstar-core` (domain + engine), `oathstar-protocol` (wire DTOs),
      `oathstar-server` (Axum `/state`,`/command`,`/events`), `oathstar-content`
      (TOML loader), `oathstar-datastar` (SSE), `oathstar-storage` (minimal).
    - Rooms carry `region/subregion/x/y/z/glyph/passable` + `entities`/`items` ID
      refs (`core/src/lib.rs:37-56`). **Entities/items carry NO coordinates**
      (`lib.rs:91-115`) — placed by room membership only. ⇒ v1 proximity must be
      **room/cell-granularity** (things inherit their room's position).
    - `GameSnapshot`/`RoomSnapshot`/`MapSnapshot` in `protocol/src/lib.rs:21-90`;
      `RoomSnapshot` exposes **no contents** today. Snapshot built by
      `Engine::snapshot()` (`core/src/lib.rs:450-478`) via `room_snapshot`/
      `map_snapshot` — clean domain→DTO split, all serde-ready.
    - `look <target>` is **parsed but a stub** (`core/src/lib.rs:509-564`); the
      `Command` enum has `Look { target }` but **no `talk`/`take`**
      (`command.rs:54-76`). ⇒ natural resolver attach point.
    - Client **Nearby panel exists + wired**: `toNearby` scans
      `room.contents/actors/items/fixtures` (`src/client/snapshot.js:72-87`) —
      honest empty state today; lights up data-driven when the snapshot carries
      contents (zero client-logic change).
    - Tests: inline `#[cfg(test)]` with `model_world`/`entity`/`item`/`room_with`
      helpers; engine deterministic (no RNG).
- **Ticket:** forge `35ec2315-2823-462b-8a41-fbf3d03b3f4e` (#17) — linked (not
  minted). Local doc updated with `pipeline_spec` pointer.
- **EARS requirements reviewed:** REQ-001..010 in the spec; REQ-001..007 trace to
  the ticket's table; REQ-008 (Nearby panel), REQ-009 (docs), REQ-010 (gate) added.

### Open questions for Design (Phase 2)
- Distance **metric**: Chebyshev (8-dir grid, matches "two cells west" + cardinal
  + diagonals) vs Euclidean-rounded. (Gating + cell-granularity already locked.)
- Where awareness data attaches in the DTO: extend `RoomSnapshot` with
  `contents` vs a new `awareness`/`nearby` block on `GameSnapshot` — must match
  the client's `toNearby` shape to satisfy REQ-008 cheaply.
- Default radius values (sight vs interaction) and whether they're constants now
  vs per-entity/per-action config (keep the *type* extensible either way).
- Docs placement: new `docs/spatial-awareness.md` vs a section in `map-system.md`.

## Phase 2 — Design

### Resolved open questions
1. **Distance metric → Chebyshev (king-move): `max(|dx|,|dy|)`.** A square grid +
   square "blast radius" matches a tactical/roguelike sight window and the canvas
   square cells (Decision 025). Integer-only ⇒ deterministic, no float. `z` is NOT
   folded into horizontal distance — a different z-plane is *excluded entirely*
   (gated), same for region/subregion. Metric is isolated in one fn
   (`Position::cell_distance`) so Manhattan/Euclidean can swap later.
2. **DTO attach point → `RoomSnapshot.contents: Vec<NearbySnapshot>`.** This is
   exactly the client's primary path (`snapshot.room.contents`, `snapshot.js:74`),
   so the Nearby panel lights up with **zero client-logic change** (REQ-008). Each
   entry also carries `distance`/`proximity`/`interactable` — additive structured
   awareness the client currently ignores but future overlays use (REQ-005).
3. **Default radii → `sight = 3`, `interaction = 1` cells** (named consts in a
   `RadiusConfig` with `Default`). Gives clean, testable bands: d0 `Exact`,
   d1 `Interactable`, d2–3 `Visible`, d≥4 excluded — and "notice an NPC two cells
   west" = visible-not-interactable, exactly REQ-003.
4. **Docs → new `docs/spatial-awareness.md`** (awareness is an engine concern,
   distinct from `map-system.md` rendering) + a one-line cross-link added to
   `map-system.md` (REQ-009).
5. **Reveal/blocked placeholder → `hidden: bool` (`#[serde(default)]`) on `Entity`
   + `Item`.** The query excludes `hidden` things (REQ-002 "blocked by reveal
   rules → not in results"); serde-default keeps all TOML backward-compatible. The
   honest placeholder seam: future stealth/perception computes it; v1 reads a
   static flag. Covers REQ-006's "passability/line-of-sight placeholder" test.

### Approach / architecture
**Room/cell-granularity, server-authoritative, additive — no canvas output.**
Positioned things derive their cell from their containing room (rooms ARE the
grid cells; entities/items have no own coords). A new **pure-ish awareness module
in `oathstar-core`** owns the geometry + query; the snapshot builder maps results
into an additive protocol DTO that the existing client Nearby panel already reads.

- **`crates/oathstar-core/src/awareness.rs`** (new `pub mod awareness;`):
  - `Position { region, subregion: Option<String>, x, y, z }` + `from_room(&RoomDefinition)`,
    `same_plane(&self,&Position)->bool` (region == ∧ subregion == ∧ z ==),
    `cell_distance(&self,&Position)->Option<u32>` (Chebyshev via `i32::unsigned_abs`,
    `None` if not coplanar). No `as` casts (clippy-restriction clean).
  - `RadiusConfig { sight: u32, interaction: u32 }` + `DEFAULT_SIGHT_RADIUS=3`,
    `DEFAULT_INTERACTION_RADIUS=1`, `impl Default`.
  - `Proximity { Exact, Interactable, Visible }` (`Copy`, serde snake_case for
    JSON-friendliness) + `classify(distance,&RadiusConfig)->Option<Proximity>`
    (0→Exact, ≤interaction→Interactable, ≤sight→Visible, else None),
    `is_interactable(self)->bool` (Exact|Interactable), `as_str(self)->&'static str`.
  - `AwarenessKind { Actor, Fixture, Item }` + `as_str`.
  - `Awareness { id, name, kind: AwarenessKind, distance: u32, proximity: Proximity }`
    — the structured result (one perceived thing).
  - Private `perceived_candidates<'w>(&'w WorldDefinition,&RoomDefinition,&RadiusConfig)
    -> Vec<Candidate<'w>>` — walk every room on the same plane within `sight`,
    each room's `entities`/`items` (skip `hidden`), compute distance + classify,
    borrow the source `Entity`/`Item` (for name/aliases/description). Stable sort
    by `(distance, kind, id)` for determinism.
  - `pub fn perceive(...) -> Vec<Awareness>` = candidates mapped to `Awareness`.
  - `pub fn resolve_target(...,query:&str) -> Option<Awareness>` = candidates
    filtered by case-insensitive match on name **or** any alias, min by
    `(distance,id)` (exact cell wins). Pure over `&WorldDefinition`; no panics.
- **`crates/oathstar-protocol/src/lib.rs`** — add `NearbySnapshot { id, name, kind:
  String, distance: u32, proximity: String, interactable: bool }` (camelCase) and
  `RoomSnapshot.contents: Vec<NearbySnapshot>` with `#[serde(default)]` (additive;
  old payloads without it still deserialize). Protocol stays free of core (no
  cycle): `kind`/`proximity` are strings produced from core's `as_str`.
- **`crates/oathstar-core/src/lib.rs`**:
  - `room_snapshot(&self,room)` → populate `contents` via
    `awareness::perceive(&self.world, room, &RadiusConfig::default())`, mapping
    `Awareness`→`NearbySnapshot` (`proximity.as_str()`, `kind.as_str()`,
    `interactable = proximity.is_interactable()`). `command` is intentionally NOT
    server-sent — the client derives `look <name>` (keeps UI verbs out of the
    server, §14 / Decision 034).
  - Replace the Look stub (531-539) with `events.extend(self.look_at(&target));`
    and add private `fn look_at(&mut self,target:&str)->Vec<GameEvent>`: resolve via
    `awareness::resolve_target` (borrow world → owned `Awareness`, drop borrow,
    re-fetch description by id+kind, then `self.log`). Branches: interactable/exact
    → "You study {name}. {description}"; visible → "You can make out {name} nearby,
    but it is too far off to examine closely." (REQ-003); `None` → "You see nothing
    like '{target}' nearby." Bare `Look{None}` path untouched (REQ-007).
  - Add `hidden: bool` to `Entity` (after `inventory`) + `Item` (after `aliases`),
    each `#[serde(default)]`. Update the `entity()`/`item()` test helpers + any
    other struct-literal sites (grep in implement: server tests, content tests).
- **JS:** no logic change — `toNearby` already reads `room.contents` and derives
  `look <name>`. Add a `node --test` case proving it renders populated contents
  (REQ-008); keep the existing empty-state case.
- **Docs:** new `docs/spatial-awareness.md`; cross-link from `docs/map-system.md`.

§14 compliance: typed/`Option` returns (no `unwrap`/`expect` on data paths),
`#[must_use]` on pure fns, doc-comments on all public items, integer math (no
float), exhaustive matches, deterministic (no RNG).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/awareness.rs` | **new** — `Position`, `RadiusConfig`(+consts), `Proximity`, `AwarenessKind`, `Awareness`, `perceive`, `resolve_target`, `perceived_candidates`; full `#[cfg(test)]` geometry+query suite |
| 2 | `crates/oathstar-core/src/lib.rs` | `pub mod awareness;`; `hidden: bool` on `Entity`+`Item`; `room_snapshot` populates `contents` via `perceive`; Look-target stub → `look_at`; update `entity()`/`item()` test helpers |
| 3 | `crates/oathstar-protocol/src/lib.rs` | add `NearbySnapshot`; `RoomSnapshot.contents: Vec<NearbySnapshot>` (`#[serde(default)]`) |
| 4 | `crates/oathstar-server/src/main.rs` | (test only) assert beginner-slice snapshot `room.contents` is populated + slice still green |
| 5 | `tests/snapshot.test.js` (or existing client snapshot test) | add `toNearby` populated-contents case (REQ-008) |
| 6 | `docs/spatial-awareness.md` | **new** — proximity model + how it underpins sight/interaction/hearing/aggro/stealth/overlays |
| 7 | `docs/map-system.md` | one-line cross-link to the new doc |
| 8 | `docs/planning/tickets/closed/…` / spec+notes | status/AAR housekeeping at complete |

### Regression Test Plan
| # | Test (file) | Proves |
|---|---|---|
| T1 | `awareness::chebyshev_distance_is_king_move` | REQ-001/006 — `cell_distance` = max(|dx|,|dy|); diagonal (2,2)=2, (3,0)=3 |
| T2 | `awareness::distance_none_across_region_subregion_z` | REQ-001/002/006 — different region/subregion/z ⇒ `None`; same plane ⇒ `Some`; `None==None` subregion is coplanar |
| T3 | `awareness::proximity_classify_bands` | REQ-001/003/006 — d0→Exact, d1→Interactable, d2/d3→Visible, d4→None (default radii); boundary mutants die |
| T4 | `awareness::proximity_is_interactable_and_as_str` | REQ-003 — Exact/Interactable interactable, Visible not; `as_str` literals |
| T5 | `awareness::perceive_within_sight_sorted_excludes_offplane` | REQ-001/006 — returns d0..3 nearest-first, excludes d4 + other plane |
| T6 | `awareness::perceive_excludes_hidden_reveal_placeholder` | REQ-002/006 — `hidden:true` thing in sight is excluded |
| T7 | `awareness::perceive_classifies_visible_vs_interactable` | REQ-003 — d2 Visible(not interactable), d1 Interactable, d0 Exact |
| T8 | `awareness::resolve_exact_beats_nearby` | REQ-004 — same name at d0 & d2 ⇒ returns d0 (Exact) |
| T9 | `awareness::resolve_matches_alias_case_insensitive` | REQ-004 — query matches alias regardless of case |
| T10 | `awareness::resolve_none_out_of_sight_or_unmatched` | REQ-002/004 — name only at d5 ⇒ None; unknown ⇒ None |
| T11 | `core::look_interactable_describes` | REQ-004 — `look <name>` (d≤1) narrates the thing's description |
| T12 | `core::look_visible_is_too_far` | REQ-003/004 — `look` a d2 thing ⇒ "too far … examine closely" |
| T13 | `core::look_unknown_reports_nothing_nearby` | REQ-004 — absent target ⇒ "nothing like '…' nearby" |
| T14 | `core::bare_look_still_describes_room` | REQ-007 — `Look{None}` unchanged (header/desc/exits) |
| T15 | `core::snapshot_room_contents_lists_nearby` | REQ-001/005 — `snapshot().room.contents` carries placed thing w/ distance+proximity+interactable |
| T16 | `core::snapshot_room_contents_empty_when_alone` | REQ-002 — nothing in sight ⇒ empty contents |
| T17 | `core::movement_discovers_rooms` (existing, re-run) | REQ-007 — navigation/discovery unaffected |
| T18 | `protocol::nearby_snapshot_camelcase_and_default` | REQ-005 — serializes camelCase `contents[*].{name,kind,distance,proximity,interactable}`; deserializes w/o `contents` ⇒ empty (additive) |
| T19 | `server::beginner_slice_snapshot_has_contents` | REQ-004/005/007 — slice smoke: `/state` room.contents populated, slice still green |
| T20 | `js: toNearby renders populated room.contents` | REQ-008 — count>0, maps name/kind/`look <name>` |
| T21 | `js: toNearby empty when no contents` (existing) | REQ-002 — honest empty state preserved |
| T22 | `./bin/gate.sh --fast` + `cargo test --workspace` + `node --test` + `npm run build` | REQ-010 — gate green |
| — | docs check: `docs/spatial-awareness.md` present + cross-link | REQ-009 (doc review, not auto-tested) |

No genuinely uncoverable paths (deterministic, no external deps). `current_room()`'s
`expect` is a pre-existing construction invariant, untouched.

### Risks / decisions (reversible-but-load-bearing)
- **`hidden` on `Entity`/`Item` is a domain schema change** — mitigated by
  `#[serde(default)]` (TOML/JSON back-compat) and a default-false test; must fix
  all struct-literal sites or the workspace won't compile (grep in implement).
- **`RoomSnapshot.contents` = "perceivable from here"**, broader than literal
  room contents (includes nearby cells). Documented; field path kept for the client.
- **Chebyshev** chosen (square radius); isolated in one fn for a future swap.
- **Perception is decoupled from map-discovery memory** in v1 (live radius query,
  not gated by `discovered_rooms`); coupling sight→discovery is future work.
- **`perceive` walks all rooms per snapshot — O(rooms).** Fine at current scale;
  a spatial index is a future optimization if worlds grow large.
- **100% mutation MSI (gate:17):** boundary asserts at d==interaction, d==sight,
  d==sight+1 and `as_str`/`is_interactable` literal asserts are required to kill
  off-by-one and match-arm mutants (mirrors the `command.rs` test discipline).

## Phase 3 — Implement
- **Built** (production code per the manifest):
  - `crates/oathstar-core/src/awareness.rs` (new) — `Position` (`from_room`,
    `same_plane`, `cell_distance` = Chebyshev via `i32::abs_diff`), `RadiusConfig`
    (+`DEFAULT_SIGHT_RADIUS=3`/`DEFAULT_INTERACTION_RADIUS=1`), `Proximity`
    (`classify`/`is_interactable`/`as_str`, all `const`), `AwarenessKind`,
    `Awareness`, private `Candidate<'w>`, `perceive`, `resolve_target`.
  - `crates/oathstar-protocol/src/lib.rs` — `NearbySnapshot` +
    `RoomSnapshot.contents: Vec<NearbySnapshot>` (`#[serde(default,
    skip_serializing_if = "Vec::is_empty")]`, camelCase).
  - `crates/oathstar-core/src/lib.rs` — `pub mod awareness;`; `hidden: bool`
    (`#[serde(default)]`) on `Entity` + `Item`; `room_snapshot` fills `contents`
    via `awareness::perceive`; Look-target stub → `look_at` resolver; `entity()`/
    `item()` test helpers updated with `hidden: false`.
  - `docs/spatial-awareness.md` (new) + cross-link from `docs/map-system.md`.
- **Deviations from design (+ reason):**
  1. **Tests deferred to Phase 4.** The Phase-2 manifest bundled the awareness
     `#[cfg(test)]` suite + the JS `toNearby` test into implement; phase
     discipline (the implement skill reserves tests for `/pipeline:validate`)
     moves them to Phase 4. All new production code is reachable from
     `room_snapshot`/`look_at`, so it compiles + lints clean with no dead-code.
  2. **`Awareness` gained a `description` field** (not in the Phase-2 shape).
     Carrying the description lets `look_at` report the thing without a second
     world lookup, and — critically — avoids an *always-`Some`* re-fetch branch
     that would be an unkillable mutant under the 100% MSI floor. The snapshot
     view does not map `description` (wire stays lean).
  3. **`resolve_target` has no trim/empty-guard.** An empty query already yields
     `None` (no name matches `""`), so the planned guard would be unreachable
     code (another MSI hazard). Dropped it; documented the empty-query behavior.
  4. **`docs/spatial-awareness.md` written now** (implement) rather than at
     complete — docs aren't gated and the model was fresh; Phase 5 will verify +
     capture forge knowledge.
- **Compile/check (this phase, not the Phase-4 test run):**
  - `cargo check --workspace --all-targets` — clean.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean
    after fixing 2 nursery `too_long_first_doc_paragraph` hits in protocol docs.
  - `cargo test --workspace` (existing suite, sanity) — **122 passed, 0 failed**;
    the additive `hidden`/`contents` change and the `look` rewrite are
    non-breaking (the old look stub had no test).
- **Phase-4 carry-forward:** write the 22-row test plan (awareness geometry/query
  suite, core look/snapshot tests, protocol serde additive test, server slice
  `contents` assertion, JS `toNearby` populated case) and RUN the full gate.

## Inspect (Phase 3.5)
- **Lenses run** (4 independent critics, parallel): correctness, data-integrity/
  safety, mutation-survivability (100% MSI), simplification/reuse. Each read the
  actual files + AC and verified concretely (the data-integrity critic ran serde
  round-trip probes against the real types; correctness/data critics ran
  `cargo test -p oathstar-core` = 68 pass, `-p oathstar-content` = 12 pass, strict
  clippy = clean).
- **Net result: ZERO production-code defects.** The implementation is correct,
  panic-free, serde-backward-compatible, deterministic, and mutation-friendly.
  One doc clarification applied; the rest is Phase-4 test-plan hardening.

| # | Severity | Finding (file:line) | Verdict | Fix |
|---|---|---|---|---|
| F1 | MED (info) | `confront` resolves the boss by role directly, bypassing the `hidden` reveal filter (`lib.rs:744`) — a hidden roled entity would be unperceivable yet confrontable | **REAL but not a bug** — `hidden` is a v1 always-false perception placeholder; gating scripted same-room actions is out of scope (combat). Worth documenting. | Documented the carve-out in `docs/spatial-awareness.md` (reveal-placeholder section). No code change. |
| F2 | HIGH (Phase 4) | Mutation gaps in the *planned* tests: several mutants survive as the 22-row plan is worded (no production defect) | **REAL** — the gate fails at validate without precise asserts | Hardened-assertion list added below; Phase 4 must satisfy it. |
| F3 | MED | `into_awareness` clones `description` on the `perceive`→snapshot path that never reads it (`awareness.rs:218`) | **REJECTED** — splitting the lowering puts `description` on an unobserved path → unkillable mutant under 100% MSI (deviation #2 keeps one killable lowering); allocation is negligible (single-digit Strings per command, small worlds). | None (kept; rationale recorded). |
| F4 | LOW | `Position::from_room` clones region/subregion per room per `perceive` (`awareness.rs:41`) | **REJECTED** — negligible at world scale; `Position` is intended public API; a borrow-helper adds surface + mutation load for no measured gain. | None (kept). |
| F5 | LOW | entity-loop/item-loop duplication; shared `room.entities→world.entities.get` idiom with `confront` (`awareness.rs:260`) | **REJECTED** — shallow, local, reads clearly; abstracting two call sites is premature and would couple combat to awareness. | None. |
| F6 | — | Correctness (boundaries, plane gating, Chebyshev neg/overflow via `abs_diff`, exact-vs-nearby, borrow order, origin inclusion, bare-look/movement untouched, additive payload) | **CLEAN** — all verified, incl. empirical probes | None. |
| F7 | — | Safety: no `unwrap`/`expect`/`as`/index/overflow added; no unsafe/secrets/IO; serde additive proven (old JSON w/o `contents` → empty; empty omitted; `hidden`-less TOML loads) | **CLEAN** | None. |

### Phase-4 carry-forward — MSI-hardened assertions (from the mutation critic; required to hit 100% MSI)
The production code is mutation-friendly; these pin the **tests** so no mutant survives. Fold into the regression suite:
1. **`cell_distance` `max`→first/second-arg:** `(2,2)` and `(3,0)` are insufficient. Assert **`(3,1)→3` AND `(1,3)→3`** (each axis dominates once) so neither `abs_diff` operand can be dropped.
2. **Const literals (`SIGHT=3`,`INTERACTION=1`) + `Default for RadiusConfig`:** at least one band/perceive test MUST go through **`RadiusConfig::default()`** with things at **d1 (Interactable), d2 (Visible), d3 (Visible), d4 (excluded)** — this single fixture kills the two literal mutants AND the `Default`→`{0,0}` body mutant.
3. **`same_plane` conjuncts:** isolate each — a pair identical except **subregion** (incl. `Some` vs `None`), and a pair identical except **z** — each asserting `cell_distance == None`. (Not one combined "all differ" case.)
4. **Item side:** perceive an **item** asserted `kind=="item"` (else item-loop + `AwarenessKind::Item` mutants live); and a **hidden item** excluded (separate from the hidden-entity case).
5. **`sort_by_key` deletion:** build the fixture so BTree room-id order ≠ distance order, then assert returned distances are non-decreasing (a naive id-ascending fixture leaves the mutant alive). Same for `resolve_exact_beats_nearby`: arrange so the d2 duplicate sorts *first* in world order, assert the match is `proximity==Exact, distance==0`.
6. **`as_str` arms:** assert **all three** kind literals (`actor`/`fixture`/`item`) and **all three** proximity literals (`exact`/`interactable`/`visible`) appear in some asserted awareness/snapshot; include a **Fixture** entity (kills `from_entity_kind`).
7. **`look_at` branches:** assert a fixed phrase from **each** of the 3 arms (`"You study"`, `"too far off to examine closely"`, `"nothing like '<target>'"`), the **description text** in the interactable arm (deviation #2 obligation — description must be *observed* or it's unkillable), the interpolated target in the none arm, and `component == NarrativeMessage`.
8. **Snapshot `contents` values:** a `contents` test must include a **visible** entry (`d2`, `interactable==false`, `distance==2`) **alongside** an exact entry (`d0`, `interactable==true`, `distance==0`) so the `is_interactable()`/`distance` mappings aren't constant-equivalent.

### Verification
No production code changed (doc-only fix), so the Phase-3 green state holds: `cargo check --workspace --all-targets` clean, strict clippy clean, 122 existing tests pass — re-confirmed below at inspect close.

## Phase 4 — Validate
- **Tests added (25 Rust + 1 JS), all from the plan + inspect-hardened asserts:**
  - `awareness.rs` `#[cfg(test)]` (13): `cell_distance` Chebyshev incl. asymmetric
    (3,1)/(1,3) + i32::MIN/MAX overflow-safety; cross-plane None (subregion
    Some/Some, Some/None; z; region) + same-plane Some; `classify` bands via
    `RadiusConfig::default()` d0–d4; `default_radii_are_three_and_one`;
    `is_interactable`+`as_str` all variants; `AwarenessKind::as_str`; perceive
    nearest-first w/ BTree-id≠distance order, excludes d4 + hidden entity + hidden
    item + off-plane z + off-plane subregion; perceive actor/fixture/item kinds +
    bands + description; resolve name-only, alias-only (case-insensitive),
    none/hidden/out-of-sight/empty, and exact-beats-nearby.
  - `lib.rs` (7): look interactable entity (+description observed) / item /
    visible-too-far / unknown / bare-look-unchanged; snapshot contents exact+visible
    pair; snapshot empty when alone.
  - `protocol/lib.rs` (4): NearbySnapshot field serialization; empty-contents
    omitted; deserialize-without-contents → empty; populated round-trip.
  - `server/main.rs` (1): start-room snapshot exposes Mara (candle_shop, d1,
    interactable) — the beginner slice's Nearby panel lights up.
  - `tests/client.test.js` (1): `toNearby` renders the real `NearbySnapshot` wire
    shape (REQ-008 also already covered by the existing populated-contents case).
- `cargo test --workspace`: **GREEN** — 127 passed, 0 failed (content 12, core 88,
  datastar 11, protocol 4, server 12).
- `node --test tests/*.test.js`: **GREEN** — 27 passed, 0 failed.
- `npm run build`: **GREEN** — 11 modules, built in ~90ms.
- `./bin/gate.sh --fast`: **GATE GREEN [fast]** — 14/14 static gates pass (fmt was
  RED once on unformatted test code → `cargo fmt --all` → GREEN; formatting-only).
- **Mutation (100% MSI) pre-verified on the new code** (full-workspace mutation runs
  at `/commit`): scoped `cargo mutants` — `awareness.rs` 34 mutants → 27 caught,
  7 unviable, **0 survived**; `lib.rs` `look_at`/`room_snapshot` 5 → 3 caught,
  2 unviable, **0 survived**. The inspect-hardened asserts killed every mutant.
- **Pre-existing exclusions:** none. No pre-existing failures; no skips. Added
  `serde_json` as a `[dev-dependencies]` to `oathstar-protocol` for the serde test
  (passes machete/deny — it is used in the tests).
- **Carry-forward to `/commit`:** FULL `bin/gate.sh` (gates 15–17: rust+js coverage
  floors and full-workspace mutation) writes the commit receipt. New code is
  heavily covered and mutation-clean, so the floors should hold.

## Phase 5 — Complete
- Docs updated:
- Forge capture (aar/failures/rules/decisions):
- Ticket closed:
- Archived:
