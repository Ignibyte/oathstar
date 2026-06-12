# WORK-map-entity-markers-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #33 — S0 of the blank-colors vertical slice: map
  snapshot exposes per-room hostile/item presence; the client draws
  ember/gold dot overlays. Owner picked S0 first ("indifferent on order"
  → lead's recommendation: smallest, instant visual payoff). Process: no
  explicit end-to-end waiver stated; the session's established pattern
  (#31/#32) is run-through-commit+ff-merge with the owner reading along —
  proceed likewise, pause only on gate failure/scope conflict.
- **Intake source:**
  `INTAKE-blank-colors-vertical-slice-city-forest-cave.md` (S0 row) —
  stays `candidate` (it tracks the whole slice; S0's promotion is
  recorded in its ticket row, not by flipping the doc).
- **Classification / tier:** Work pipeline, one slice — two snapshot
  flags, model/plan/seam threading, tests.
- **Base verified:** `main` @ `1baeda6` (#32 merged+pushed); no active
  pipeline; forge up; no bulletins.
- **Anchors verified at plan:**
  - `Engine::map_snapshot` (lib.rs ~2667): iterates `world.rooms` into
    `MapRoomSnapshot { id, title, x, y, z, glyph, passable, discovered,
    current, exits }` (protocol lib.rs:193). The flags compute here.
  - Presence sources are LIVE: victory's `remove_entity_everywhere`
    drops the placement; `take`/`drop`/enemy-inventory-drops mutate
    `room.items` — flags need no extra bookkeeping (REQ-002 is free).
  - `Entity::has_role(Role::Hostile)` (lib.rs:218) — the same test the
    #23 `threat` affordance uses (room_snapshot ~2620); the hostile role
    CONTRACT guarantees a combat profile.
  - Client thread: `toMapModel` cell (map.js — gains the flags),
    `toDrawPlan` ops (canvas-map.js — markers enter only as drawn
    fields), `drawMapCanvas` seam (client-app.js — draws dots after
    tile/stroke, before-or-after glyph: design pins z-order).
  - Server test harness: `play_to_boss_victory` + the stray fight give
    REQ-005 its lifecycle route (kill stray on ashen_road → fang drops
    → has_items true; take fang → false; has_hostiles true → false).
  - Standing rules in play: PR-oathstar-render-plan-test-002 (ops carry
    only what the seam draws — applied in #16 AND #32);
    PR-claude-fixture-distinguishable-transitions-001 (stage flag
    transitions so before ≠ after); package-scope mutation (MSI 100) —
    boolean presence fields generate `&&`/`||`/negation mutants needing
    both-arm tests; PR-claude-renderer-tests-through-the-normalizer-001
    (if any new wire field reaches the JS via parseEvent — map data
    flows via /state snapshot + snapshot-bearing responses, NOT
    parseEvent, so likely N/A; design confirms).
- **Ticket:** forge `9fe59e9c-776a-4147-9fdf-0b43aa6fe181` (#33), local
  doc `docs/planning/tickets/open/TICKET-33-map-entity-markers-hostiles-items-as-colors.md`.
- **AAR opened:** `33891aa4-d9d3-42be-a49e-b9c932730f90`.
- **EARS requirements reviewed:** REQ-001..006 (verbatim in the spec).

### Open design questions (for Phase 2 — Planner does NOT decide these)
1. **Visibility/fog rule (the intake's knowledge question).** Options:
   (a) live presence on DISCOVERED rooms only (undiscovered stay fog —
   simplest, still "live state through walls" for visited rooms);
   (b) live presence on the CURRENT room only (most MUD-conservative,
   but the map adds nothing the Nearby panel doesn't already say);
   (c) remembered-last-seen (what you saw when last there — needs new
   per-room memory state in GameState + saves: heaviest). Design picks,
   documents the knowledge-model rationale, and notes which option the
   vertical slice's "enemies as colors" vision wants (likely (a)).
2. **Protocol field shape.** `has_hostiles`/`has_items` plain bools
   (always present — two bytes per room) vs `#[serde(default,
   skip_serializing_if)]` omission for false (byte-stable for
   marker-less rooms, the #18 omit-when-empty precedent). Design picks +
   pins the serde attributes and JS-side defaults.
3. **Dot geometry + z-order.** Corner-anchored small circles (ember
   top-right, gold bottom-right?) vs edge badges; radius vs tile size;
   draw order relative to glyph text (dots under glyph? over?); both
   dots present simultaneously — layout. Colors from the concept: ember
   (236,104,43), gold (230,200,90). Aria: does the label mention counts
   ("2 rooms with enemies")? Design pins exact geometry + aria wording.
4. **Current-room suppression?** The player IS in the room — the Nearby
   panel already shows the stray; does the cell still need the ember dot
   (visual consistency) or is it noise next to the hero ring? Design
   decides (lean: keep the dot — consistency and the at-a-glance read).
5. **Marker fields' plan shape.** Per-op `markers: {hostile: bool,
   item: bool}`? Two nullable colors? Pre-resolved dot ops in a separate
   plan array the seam iterates? Must satisfy "ops carry only what the
   seam draws."
6. **mapAriaLabel extension.** Summarize marker counts or stay silent?
   (Accessibility parity for the color-only signal — lean: add "N
   hostile, M item rooms" to the summary.)

## Phase 2 — Design

- **Recall (12 surfacings):** AD-claude-nearby-affordances-001 (#23 — the
  direct precedent: server-computed affordances exposed additively, the
  exact pattern these flags follow); AD-claude-tileset-name-keyed-
  fallback-render-001 (#32 — the draw-plan/seam the dots ride);
  PR-oathstar-render-plan-test-002 (ops carry only what the seam draws —
  third application); PR-claude-fixture-distinguishable-transitions-001
  (stage flag transitions before ≠ after). Confirmed: protocol structs
  are `#[serde(rename_all = "camelCase")]` (wire: `hasHostiles`/
  `hasItems`); map data reaches the client via the /state + response
  snapshots consumed directly by `toMapModel` — parseEvent is NOT in the
  path, so PR-claude-renderer-tests-through-the-normalizer-001 is N/A.

### Approach / architecture (settles the 6 Phase-1 questions)

1. **Fog rule (Q1): live presence on DISCOVERED rooms, enforced
   server-side.** `flag = discovered && presence` computed in
   `map_snapshot` — undiscovered rooms emit false, so the payload never
   leaks fogged state to devtools (Decision 041: the server decides
   knowability). Discovered rooms show LIVE state (a visited room's
   marker updates even from across the map) — the slice's at-a-glance
   legibility wants exactly this; remembered-last-seen needs per-room
   memory in GameState + saves and is deferred until stealth/scouting
   matters (revisit trigger recorded).
2. **Field shape (Q2): omit-when-false bools.**
   `#[serde(default, skip_serializing_if = "is_false")]` on both fields
   (+ a private `fn is_false(b: &bool) -> bool` in the protocol crate —
   the `Option::is_none`/omit-when-empty house pattern for bools).
   Marker-less rooms stay byte-identical to today's payload; the JS
   default (`Boolean(room?.hasHostiles)`) makes absence false.
3. **Engine computation:** in the existing `map_snapshot` per-room
   closure — bind `discovered` first, then
   `has_hostiles = discovered && room.entities.iter().any(|id|
   self.world.entities.get(id).is_some_and(|e|
   e.has_role(Role::Hostile)))` and
   `has_items = discovered && !room.items.is_empty()`. Total lookups, no
   panic path (§14); placements are live state so REQ-002 needs no extra
   bookkeeping.
4. **Dot geometry (Q3):** pure, in `canvas-map.js` —
   `MAP_MARKER_COLORS = Object.freeze({ hostile: "#ec682b", item:
   "#e6c85a" })` (the concept's ember/gold); radius
   `max(2, round(tile * 0.14))` (4px at 32); hostile dot top-right at
   `(x + size - inset, y + inset)`, item dot bottom-right at
   `(x + size - inset, y + size - inset)`, `inset = radius + 2`. Drawn
   LAST (over tile, stroke, and glyph) with a 1px `#0f1216` outline for
   contrast on any tile.
5. **Plan shape (Q5):** each op gains `markers: []` of pre-resolved
   `{ cx, cy, r, fill }` (0–2 entries) — the seam purely executes
   `arc()` per entry; every field is drawn
   (PR-oathstar-render-plan-test-002 satisfied); empty array no-ops.
6. **Current room (Q4): no suppression** — the dot stays beside the
   hero ring (consistency; "I'm in a room WITH a hostile" is the battle
   read).
7. **Aria (Q6):** `mapAriaLabel` appends `, hostiles in N room(s)` and
   `, loot in M room(s)` segments only when the count is nonzero
   (singular/plural like the existing rooms text) — accessibility
   parity for a color-only signal.
8. **Seam:** after the glyph block, per op:
   `for marker of op.markers: beginPath, arc(cx, cy, r), fill =
   marker.fill, fill(), stroke with #0f1216, lineWidth 1`.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-protocol/src/lib.rs` | `MapRoomSnapshot` + `has_hostiles`/`has_items` (serde default + omit-when-false; private `is_false` helper) |
| 2 | `crates/oathstar-core/src/lib.rs` | `map_snapshot` computes both flags (discovered-gated, live placements); `#[cfg(test)]` T1–T6 |
| 3 | `crates/oathstar-server/src/main.rs` | T11 served marker-lifecycle test |
| 4 | `src/client/map.js` | `toMapModel` cell gains `hasHostiles`/`hasItems` (Boolean defaults) |
| 5 | `src/client/canvas-map.js` | `MAP_MARKER_COLORS`, marker geometry, `op.markers`, aria count segments |
| 6 | `src/client-app.js` | seam: marker arc loop after glyph |
| 7 | `tests/canvas-map.test.js` | T7–T10 |
| 8 | `docs/map-system.md`, `docs/spatial-awareness.md` | (Phase 5) markers implemented notes |

### Regression Test Plan
Mutation-aware: both arms of every `&&` independently
(PR-oathstar-msi-test-assertions-001); flag transitions staged before ≠
after. Core tests use the `combat_world` fixture (full control: per-test
item/entity insertion); the served test uses the real beginner world.

| # | Test | Proves Requirement |
|---|---|---|
| T1 | core: discovered start room with an item placement → `has_items` true; same room `has_hostiles` true via the placed stray; both flags FALSE on the undiscovered "ridge" room even after inserting an item + hostile there (the fog arm — kills `&&`→`∥`) | REQ-001 |
| T2 | core: discovered room with NO items/hostiles ("clearing" after walking there) → both false (the presence arm) | REQ-001 |
| T3 | core: discovered room whose only entity is NON-hostile (elder in "field" after removing the hostiles per-test) → `has_hostiles` false (kills `has_role`/`is_some_and`→true) | REQ-001 |
| T4 | core: kill the stray via manual attacks → `has_hostiles` flips true→false; give the fixture stray an inventory item first → drop flips `has_items` false→true; `take` flips it back (staged transitions, before ≠ after) | REQ-002 |
| T5 | core: player `drop`s a carried item in a discovered itemless room → `has_items` false→true | REQ-002 |
| T13 | core (ADDED at inspect): a discovered room whose ONLY hostile is `hidden` and whose ONLY item is `hidden` → both flags false (the map never discloses what perceive conceals — the reveal-rule mirror) | REQ-001 |
| T6 | **protocol-crate** serde shape (AMENDED at inspect — cargo-mutants is package-scoped, so the `is_false` mutant killers must live in `oathstar-protocol`'s own `mod tests`, beside the omit-when-empty precedent): construct `MapRoomSnapshot`, false flags → keys ABSENT; true → `hasHostiles`/`hasItems` present | REQ-001/006 |
| T7 | js: `toMapModel` cell carries both flags; absent wire fields default false; snapshot input not mutated | REQ-003 |
| T8 | js: `toDrawPlan` op `markers` exact — both flags on a 32px cell → two entries with exact `{cx, cy, r, fill}` (ember top-right, gold bottom-right, r=4, inset=6); one flag → one entry; none → `[]`; AMENDED at inspect: + a small-tile case (10px → r=2, clamped inset=3, exact centers) pinning the radius floor and the collision clamp | REQ-003 |
| T9 | js: a flagged plan differs from the unflagged plan ONLY in `markers`; tile/sprite/glyph/textColor identical (REQ-004 preservation) | REQ-004 |
| T10 | js: `mapAriaLabel` — zero counts append nothing (byte-identical to today); 1 hostile room → ", hostiles in 1 room"; 2 loot rooms → ", loot in 2 rooms" (exact strings) | REQ-003/004 |
| T11 | server: /state lifecycle — square `hasItems` true at start (wax stub); walk to ashen_road → `hasHostiles` true; three manual attacks fell the stray → `hasHostiles` false + `hasItems` true (fang dropped); `take fang` → false; undiscovered tower rooms absent/false throughout | REQ-005 |
| T12 | browser smoke: dots visible over tiles; defeat clears ember dot live (fallback: shell asserts on /state as in #32 if the extension is down) | REQ-003 |
| — | gate: full suite (REQ-006) | REQ-006 |

Genuinely uncoverable by unit test: the `arc()` calls in the seam
(node/jsdom has no canvas-2D — the standing #16/#32 constraint); T12
smoke carries it; all geometry is pure and exactly asserted in T8.

### Risks / decisions
- **D1 fog rule = discovered-gated live state, server-enforced** (no
  payload leak; revisit at stealth/scouting).
- **D2 omit-when-false serde** — marker-less payloads byte-identical;
  `is_false` helper is two trivial mutants, killed by T6.
- **D3 markers as pre-resolved draw entries** — geometry pure + tested,
  seam dumb (the thrice-applied render-plan rule).
- **D4 dots over glyph** — markers are the at-a-glance signal; corner
  placement avoids glyph collision at 16px+ tiles.
- **R1 — combat_world fixture rooms lack item placements today**; tests
  insert items/entities per-test (the fixture returns a mutable
  WorldDefinition pre-`try_new`) — no fixture rewrite.
- **R2 — aria string growth** could break existing exact-match aria
  tests — T10 keeps zero-count byte-identical, so only new-flag cases
  change.

## Phase 3 — Implement
- Built (to the manifest):
  - `oathstar-protocol`: `MapRoomSnapshot.has_hostiles`/`has_items`
    (`serde(default, skip_serializing_if = "is_false")`, doc comments
    carrying the fog/leak rationale); private `const fn is_false`.
  - `oathstar-core`: `map_snapshot` per-room closure binds `discovered`
    first, computes both flags discovered-gated from live placements
    (`has_role(Role::Hostile)` via total `get`/`is_some_and` — no panic
    path).
  - `src/client/map.js`: cell gains `hasHostiles`/`hasItems`
    (`Boolean(room?.…)` — wire omission coerces false).
  - `src/client/canvas-map.js`: `MAP_MARKER_COLORS` (ember/gold/outline),
    private `markerRadius` (`max(2, round(tile*0.14))`) + `cellMarkers`
    (hostile top-right, item bottom-right, inset r+2); ops gain
    `markers: [{cx, cy, r, fill}]`; `mapAriaLabel` appends nonzero
    `, hostiles in N room(s)` / `, loot in M room(s)` segments.
  - `src/client-app.js`: seam draws markers LAST per op (arc + fill +
    1px `MAP_MARKER_COLORS.outline` stroke); import extended.
- Deviations from design (+ reason): two clippy strict-tier hits fixed
  at source during implement — `struct_excessive_bools` on the wire DTO
  (justified inline `#[allow]`: independent JSON facets, not a state
  machine; restructuring would break the payload contract) and
  `missing_const_for_fn` (made `is_false` const). Both within §0's
  justified-inline-allow rule; no allowlist change.
- `cargo check`/`fmt`/`clippy --workspace --all-targets` clean;
  `node --check` clean ×3; existing canvas-map/client suites green (28).

## Inspect (Phase 3.5)
- Lenses run: 2 critics — (1) correctness + data-leak (fog gate on the
  wire, crafted saves, geometry probes, seam state, back-compat both
  directions), (2) plan-integrity + conventions + mutation-readiness
  (gate:10 grep run verbatim, `cargo mutants --list` ground truth,
  render-plan rule, reuse).
- Findings:
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | critical | gate:10 would go RED: the `struct_excessive_bools` allow's justification was a prose block ABOVE the attribute; the suppression meta-gate greps per-line for a same-line trailing comment (protocol lib.rs:196) | REAL (critic ran the gate's exact grep) | Justification moved onto the attribute line; grep now returns empty. Forge: BF-claude-gate10-justification-must-be-same-line-001 |
  | 2 | high | The planned core-located T6 can never kill the 3 new protocol-crate `is_false` mutants — cargo-mutants is package-scoped (gate:17 `--workspace` enumerates workspace-wide but runs each mutant against the mutated crate's OWN tests); protocol had zero mutable fns before this diff | REAL (`cargo mutants --list` + docs) | Test plan amended: T6 lives in `oathstar-protocol`'s `mod tests` (omit-when-empty precedent). Forge: PR-claude-mutants-package-scoped-test-placement-001 |
  | 3 | medium | Markers leak `hidden` content: flags counted any hostile placement / any item, not mirroring the #17 reveal rule (awareness::perceive skips hidden) — a hidden ambush or secret item would dot the map while look shows nothing (core map_snapshot) | REAL (traced; latent — no hidden content shipped) | Both flags now filter `!hidden` via the registries (items switched to perceive's any-over-registry form); T13 added to the plan. Forge: BF-claude-map-markers-leak-hidden-content-001 |
  | 4 | low | Corner dots collide at tiles ≤12px (coincide exactly at 8px — item occludes hostile); unreachable today (config min 32, tests min 16) | REAL (node probe) | Inset clamped: `min(r+2, max(r, tile/2 − r))` — 32/16px geometry unchanged, tiny tiles stay tangent; T8 amended with a 10px case |
  | 5 | low | `markerRadius`'s `max(2, …)` floor arm unpinned by any planned test | REAL | Covered by the same T8 amendment (10px → r=2) |
- Verified clean (evidence-backed): fog gate holds on the wire (fogged
  rooms emit NEITHER key — probe on the real world); crafted saves
  (orphan discovered ids; current-room-undiscovered) load with correct
  flags and no panic; boss (non-hostile role) sets no flag; wire compat
  both directions incl. byte-identical pre-#33 payload round-trip; zero
  save-format impact; markers absent on fog/empty cells; legacy aria
  byte-identical; seam lineWidth/beginPath state clean; single
  construction site for MapRoomSnapshot; render-plan rule satisfied (all
  marker fields drawn; `outline` correctly a seam-side constant, not an
  op field); no helper duplication; suites green post-fix (core 260,
  protocol 20, JS 28 in the touched suites).

## Phase 4 — Validate
- Tests added (12 new — every plan row incl. the inspect amendments):
  - `oathstar-core` (6): `marker_flags_gate_on_discovery` (T1 — fog arms
    incl. hostile+item in the never-visited ridge),
    `marker_flags_stay_false_without_presence` (T2),
    `non_hostile_actors_do_not_flag` (T3),
    `marker_flags_track_defeat_loot_and_take` (T4 — staged
    true→false→true→false lifecycle via two manual strikes),
    `dropping_an_item_flags_the_room` (T5),
    `hidden_content_never_flags_the_map` (T13 — the reveal-rule mirror);
    + a `map_room_of` helper.
  - `oathstar-protocol` (1, IN-CRATE per the inspect amendment):
    `map_room_marker_flags_omit_when_false` (T6 — absent keys when false,
    camelCase when true, old-payload deserialize). **Targeted mutation
    verified: `cargo mutants -p oathstar-protocol` → 3/3 caught, 0
    missed** (the `is_false` mutants the inspect critic flagged).
  - `tests/canvas-map.test.js` (4): model flags + omit-false defaults +
    input immutability (T7); exact marker geometry at 32px AND the 10px
    floor/clamp case (T8); markers-only plan diff (T9); aria exact
    strings with byte-identical zero-count case (T10).
  - `oathstar-server` (1): `map_marker_flags_track_the_served_fight`
    (T11 — wax-stub flag at start, fog→discovery reveal, 3-strike
    victory clearing ember + raising gold, take clearing it, tower dark
    throughout). Green on first run.
- `cargo test --workspace`: ALL GREEN — core 266, protocol 21, server 28,
  all other crates green, 0 failed.
- `node --test tests/*.test.js`: **59 pass, 0 fail** (was 55).
- Live smoke (T12): dev server restarted on the new binary —
  `/state` shows `hasItems: true` on the square (wax stub), NO marker
  keys on fogged rooms (fog never leaks); the old-binary payload read
  earlier doubles as old-wire compat evidence. Browser-visual dot check
  available to the owner in the open tab (extension still unavailable —
  the #32 fallback posture).
- `bin/gate.sh --fast`: **GATE GREEN [fast]** — 14/14. FULL gate at
  `/commit`.
- Pre-existing exclusions: none.

## Phase 5 — Complete
- Docs updated: `docs/map-system.md` ("Implemented (ticket #33 —
  entity/item presence markers)" block); `docs/spatial-awareness.md`
  (the reserved-overlay line now records its shipping under the same
  principles); `docs/decisions.md` **Decision 051** (server-computed,
  discovery-gated at the wire, reveal-rule-faithful, presence-not-
  identity; revisit triggers incl. stealth + enemy movement).
- Forge capture: `aar-submit` `33891aa4…` → completed, effectiveness 5,
  4 surfacings used, 3 rules materialized (render-plan-test ×3rd
  application, fixture-transitions, the new package-scoped-mutants
  rule). Failures + rule captured AT INSPECT:
  BF-claude-map-markers-leak-hidden-content-001,
  BF-claude-gate10-justification-must-be-same-line-001,
  PR-claude-mutants-package-scoped-test-placement-001.
  `architecture-decision-record`: AD-claude-map-presence-markers-001.
- Ticket closed: forge `9fe59e9c…` (#33) → **done**; local doc →
  `docs/planning/tickets/closed/` (status/closed/pipeline_spec updated).
- Archived: spec+notes pair → `docs/planning/pipeline/completed/`.
