# WORK-combat-rewards-and-defeat-consequences-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #26 — Combat Rewards + Defeat Consequences v1. Close
  the grind loop on the #22/#24/#25 combat foundation: deterministic victory
  XP from `CombatProfile`, defeated-hostile authored inventory drops into
  the room, defeat reset to start at max HP with the `max(1, floor(xp/10))`
  penalty, beginner demo content, exact tests, docs. AUTO-APPROVE; STOP
  BEFORE `/commit`; validate = workspace tests + node tests + `npm run
  build` + `./bin/gate.sh --fast` (FULL gate/commit owner-gated).
- **Intake source:** none (ticket doc pre-existed with locked v1 decisions
  AND EARS; forge ticket minted this phase:
  `aa8c7f72-991a-4c30-9bb7-80b41caa2172`, #26 — frontmatter updated from
  `pending-forge`; the doc's `pipeline_spec` already pointed at this spec
  name).
- **Classification / tier:** Work pipeline, **one shippable slice** — three
  additive engine behaviors (award, drop, reset) hanging off the existing
  `end_combat` funnel + one new authored field family + demo content. No
  protocol kinds expected, no UI redesign.
- **Base verified:** new branch `codex/oathstar-ticket-26-combat-rewards`
  off `main` @ `0d3489f` (#25 — all dependencies merged). Worktree clean
  except the Codex-owned strays (untouched) and this pipeline's docs. No
  active pipeline; forge up.
- **Forge recall (lessons/failures surfaced):**
  - AAR opened: `e152bf2c-a059-40a8-bcf6-9db136737236`; Plan-phase
    `knowledge-context` logged (13 surfacings).
  - ADs: combat v1 (`2e4d969e`), pulse-rides-tick (`f20f3ff4`), entity
    contracts (`9d063c49` — the model for the new inventory validation
    contract).
  - Prevention rules:
    **PR-claude-driver-change-simplification-audit-001** — re-audit
    documented simplifications when semantics change: the defeat-reset
    interacts with v1's "enemy HP resets between encounters" and the #24
    disengage rule (both audited: defeat only happens in the encounter
    room; the left-in-place enemy re-fights from authored HP — consistent).
    **PR-claude-enumerate-variant-string-arms-001** — any new per-variant
    strings (e.g. per-outcome summary additions) need every-variant
    exact-string tests.
    **PR-claude-expect-invariants-over-unreachable-arms-001** — keep new
    arms reachable.
  - Failures `ab1fba07` (pulse-follows-player — why defeat-in-room is an
    invariant we can lean on) and `46d06e06` (burst fast-forward) as
    regression context.
- **Current-code anchor map (from the #24/#25 implementations, same session):**
  - `crates/oathstar-core/src/lib.rs`: `CombatProfile { health, attack,
    disclose_stats }` (gains `xp`, `#[serde(default)]`); `Entity` (gains
    additive `inventory: Vec<String>`, `#[serde(default)]`);
    `PlayerState.xp: u64` (init 0 — currently write-only);
    `end_combat(outcome)` — the single resolution funnel: Victory arm
    (remove enemy → NOW award XP + drop inventory), Defeat arm (NOW reset to
    start + penalty instead of revive-in-place), Fled arm (unchanged);
    `CombatState.enemy_id` survives into `end_combat` via the taken state
    (the drop/XP lookups hang off it); `take_at`/room item placements (#18)
    — the drop target; `validate_entity_contracts` (#21) — where the
    inventory-ids-resolve contract lands; `world.start_room_id` +
    `discovered_rooms` — the reset target; `snapshot()` already carries
    `player.xp`.
  - `crates/oathstar-core/src/lib.rs` tests pinning current defeat
    semantics (DELIBERATE updates ahead):
    `defeat_at_zero_hp_revives_player_and_clears_combat` (#22),
    `pulse_defeat_at_exact_zero_revives_and_stops_pulsing` (#24 — asserts
    revive at max), plus any text asserts on the Defeat `CombatEnded` line
    ("…You wake later, battered but whole.").
  - `crates/oathstar-protocol/src/lib.rs`: `PlayerSnapshot.xp` (wire-ready);
    `CombatEnded { outcome, text }` — the summary surface; NO changes
    expected.
  - Content: `modules/beginner/world.toml` (`ashen_stray` — gains
    `combat.xp` + an `inventory` item; a new simple item entry),
    `rooms.toml` (unchanged unless the demo item needs authoring elsewhere).
  - JS: likely zero code changes (XP already renders in the HUD via
    `toHud`; drops appear through existing Nearby/contents; summary rides
    `CombatEnded` text). REQ-007 may be satisfiable by Rust text asserts +
    existing JS rendering tests — design confirms.
- **EARS requirements reviewed:** REQ-001..009 (verbatim in the spec).
  001/002 XP award paths; 003/004 drops; 005/006 defeat reset + penalty
  boundaries; 007 summary output; 008 beginner content; 009 preservation.
  Every REQ gets ≥1 exact-value test in the Phase-2 plan.

### Open design questions (for Phase 2 — Planner does NOT decide these)
1. **`CombatProfile.xp` shape.** `u64` (matches `PlayerState.xp`) with
   `#[serde(default)]` → 0. Name: `xp` vs `xp_reward` (authoring clarity vs
   brevity) — design picks and pins the TOML key.
2. **`Entity.inventory` shape + contract.** `Vec<String>` of item ids,
   `#[serde(default)]`. Validation: a #21-style construction contract —
   every inventory id must resolve in `world.items` (else
   `WorldValidationError`)? Or lenient skip-at-drop? (Recommend the
   contract: fail-fast authored content, consistent with #21.) Also: does
   ANY entity get an inventory or only combatants/hostiles (recommend: any
   entity — it's just authored data; only victory reads it).
3. **Drop mechanics.** Where do ids land — the defeated room's item
   placement list (the same structure `take_at` reads / `drop_at` writes)?
   Order (append in authored order — deterministic)? What if the room
   already holds the same item id (duplicates allowed by the placement
   model? check `drop_at` semantics)? Drop narration: one `CombatMessage`
   line ("The Ashen Stray drops a cracked fang.") per item or one combined
   line — and does it fold into the `CombatEnded` text instead (the ticket
   prefers existing surfaces; design picks the exact text shape).
4. **XP award narration.** Fold into the Victory `CombatEnded` text ("You
   have defeated X. Victory! You gain 5 XP.") vs a separate `CombatMessage`
   line before it. Folding changes the existing victory text — the existing
   exact-string victory asserts get updated deliberately (flag like the
   defeat tests). Zero-XP victories must keep the ORIGINAL text exactly
   (REQ-002 "preserve existing behavior")? Or also restructure? Design
   decides + documents which strings change.
5. **Defeat reset semantics details.** Emit `RoomEntered` +
   room-description events for the start room (the move_direction pattern)
   so the feed narrates the wake-up, or rely on the `CombatEnded` text +
   client `/state` refresh alone? (The modal closes via refresh either way;
   the feed coherence question is the design call.) Defeat text rewrite:
   "…bested you. You wake at {start room}…" + penalty mention ("You lose
   N XP."). The penalty math lives where — `end_combat`'s Defeat arm
   (recommend) with `u64` saturating ops.
6. **Penalty math pinning.** `max(1, xp / 10)` (integer division IS floor)
   when `xp > 0`; `xp.saturating_sub(penalty)`. Exact fixtures: xp=0 → no
   penalty, no line (or "no XP lost" line?); xp=1 → lose 1 → 0; xp=5 → lose
   1 → 4; xp=100 → lose 10 → 90. Whether the penalty line appears at xp=0
   (recommend: no line — nothing happened).
7. **Demo content.** `ashen_stray` gains `xp` (e.g. 5) + drops one new
   authored item (e.g. `stray_fang` — name "Cracked Fang", kind "trophy"?).
   The item must be takeable (REQ-003 via the existing flow) — `take fang`
   then shows in pack. Reachability already proven (ashen_road). Does the
   Bell-Eater get XP too (confront path doesn't go through end_combat —
   out: confront is oath resolution, not combat victory; leave it).
8. **REQ-007 verification shape.** Rust exact-string asserts on the three
   outcome texts + the existing JS `combat_ended` feed-summary test
   (already passing) — is any NEW JS test needed? (Likely no new JS code →
   no new JS test beyond maybe a HUD-xp view-model check if `toHud` maps
   xp... check whether toHud exposes xp today.)
9. **Save/`GameState` shape.** Drops mutate `world` room placements +
   entity inventories in memory (the #22 "in-memory only" precedent, noted
   R5 there) — no persistence concerns now, but document the carried-forward
   note. `PlayerState.xp` changes are plain state. No serde-default needed
   on new CombatState fields (none added).

## Phase 2 — Design

### Approach / architecture (the 9 open questions, resolved)

Smaller than planned — two discoveries collapse the schema work:
**`Entity.inventory: Vec<String>` already exists** (`#[serde(default)]`,
"ids of items the entity owns" — authored since the world model) and **its
validation contract already exists** (`WorldValidationError::EntityItemMissing`
— every entity-owned item id must resolve in `world.items`, checked at
`try_new`). The only new field is `CombatProfile.xp`. Everything else is
`end_combat` arm behavior + content + tests.

1. **`CombatProfile.xp: u64`** (Q1), `#[serde(default)]` → 0 — matches
   `PlayerState.xp`'s type; TOML key `xp`
   (`combat = { health = 9, attack = 3, xp = 5 }`). Existing fixtures and
   the Bell-Eater (xp absent → 0) stay valid untouched.
2. **Inventory + contract (Q2): nothing to build.** Reuse the existing field
   and the existing `EntityItemMissing` validation. Any entity may own
   items; only victory reads them.
3. **Drop mechanics (Q3).** In `end_combat`'s Victory arm:
   `mem::take(entity.inventory)` (via `world.entities.get_mut` — the
   registry survives placement removal, the #22 precedent `take_at` relies
   on) → append each id to the CURRENT room's `items` placements in
   authored order (victory always happens in the encounter room — the #24
   disengage rule guarantees it) → one `CombatMessage` line per item:
   `"The {enemy} drops {item name}."` (name from the item registry). The
   take()-clear IS the no-duplicate guarantee. Drops flow into the existing
   #17/#18 contents/`take` path with zero new plumbing. No battle-log push
   (the state is being consumed; the modal closes).
4. **XP award + narration (Q4).** Victory arm:
   `player.xp += enemy_xp_reward(enemy_id)` —
   `entities.get(id).and_then(|e| e.combat.as_ref()).map_or(0, |c| c.xp)`
   (a total chain; no defensive arms). Text:
   - xp > 0: `"You have defeated {enemy}. Victory! You gain {xp} XP."`
   - xp = 0: **byte-identical** to the existing
     `"You have defeated {enemy}. Victory!"` — REQ-002's "preserve existing
     behavior" taken literally, which also keeps every existing victory
     exact-string assert green (the constructed test strays have no xp).
   Award sits in `end_combat` — the single funnel both victory paths
   (Phase-1 round, #25 Phase-2 power strike) already pass through, so
   exactly-once is structural.
5. **Defeat reset (Q5) — the authorized semantics change.** Defeat arm:
   `apply_defeat_penalty()` (returns xp lost) → `hp = max_hp` →
   `current_room_id = world.start_room_id` (already discovered) → events:
   `CombatEnded` with
   - penalty > 0: `"{enemy} has bested you. You wake at {start room title},
     battered but whole. You lose {n} XP."`
   - penalty = 0: same without the penalty clause (nothing happened —
     no line about it),
   then **`RoomEntered{start}` + `describe_current_room()`** — the
   `move_direction` arrival pattern reused verbatim (existing event kinds,
   zero new surface) so the feed narrates the wake-up and the
   `room_entered` refresh keeps the client coherent. DELIBERATE test
   updates: `defeat_at_zero_hp_revives_player_and_clears_combat` (#22) and
   `pulse_defeat_at_exact_zero_revives_and_stops_pulsing` (#24) — rewritten
   to the new semantics (location = start room asserted), documented here
   as the authorized behavior change. The old text
   `"You wake later, battered but whole."` disappears.
6. **Penalty math (Q6).** Only when `xp > 0`:
   `penalty = (xp / 10).max(1)` (u64 integer division is floor);
   `xp = xp.saturating_sub(penalty)`. Pinned fixtures: 0 → no penalty (and
   no clause), 1 → lose 1 → 0, 5 → lose 1 → 4, 100 → lose 10 → 90.
7. **Demo content (Q7).** `ashen_stray`:
   `combat = { health = 9, attack = 3, xp = 5 }` +
   `inventory = ["stray_fang"]`; new `[[items]]` `stray_fang` — name
   "Cracked Fang", aliases `["fang", "cracked fang"]`, kind `"trophy"`,
   short description. Reachability already proven (ashen_road). The
   Bell-Eater stays reward-less (confront is oath resolution, not combat
   victory — out of scope).
8. **REQ-007 shape (Q8).** Rust exact-string asserts on all three outcome
   texts (+ the penalty/zero-penalty variants and drop lines). **No JS code
   changes**: the feed renders `CombatEnded` text via the existing path
   (existing JS test covers), drops surface through existing
   contents/Nearby, and the HUD doesn't display xp today (UI redesign is
   out of scope — noted as a future-ticket idea; `PlayerSnapshot.xp`
   carries it for any consumer). `npm test`/`npm run build` still run at
   validate per the owner instruction.
9. **State-shape notes (Q9).** No `CombatState`/protocol/datastar/server
   changes at all. Drops and placement removal mutate the in-memory world
   (the #22 R5 carried-forward note — no save wiring yet). `player.xp` is
   plain state, already on the wire.

**Decomposition (line-ceiling + reuse):** `end_combat` stays the owner;
three small helpers — `enemy_xp_reward(&self, enemy_id) -> u64` (the total
chain), `drop_enemy_inventory(&mut self, enemy_id, enemy_name, events)`
(take + append + lines), `apply_defeat_penalty(&mut self) -> u64`. The
Victory/Defeat arm bodies grow modestly; extract further only if clippy's
ceiling bites.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/lib.rs` | `CombatProfile.xp: u64` (`#[serde(default)]`, doc-commented); `enemy_xp_reward` + `drop_enemy_inventory` + `apply_defeat_penalty` helpers; `end_combat` Victory arm (drops → award → conditional XP text) + Defeat arm (penalty → restore → relocate → conditional text + `RoomEntered` + description); the two pinned defeat tests rewritten to the new semantics (deliberate). |
| 2 | `modules/beginner/world.toml` | `ashen_stray` gains `xp = 5` + `inventory = ["stray_fang"]`; new `[[items]]` `stray_fang` (Cracked Fang, trophy). |
| 3 | `crates/oathstar-content/src/lib.rs` | Content tests (X13) asserting the demo reward/drop authoring by value (existing Bell-Eater assert gains `xp: 0`). |
| 4 | `tests/` (JS) | No changes expected (REQ-007 rides existing surfaces); suites still run. |
| 5 | docs (Phase 5): `docs/combat-system.md` (rewards/defeat note), `docs/decisions.md` (Decision 044), `docs/entity-model.md` (CombatProfile.xp row; inventory-drop note), `docs/mechanics-and-systems.md` (grind-loop note if a section fits). |

**No changes**: `oathstar-protocol`, `oathstar-datastar`, `oathstar-server`,
`command.rs`, JS client code, `rooms.toml`.

### Regression Test Plan
≥1 per EARS REQ; exact values/strings throughout. Rust in the owning crate
(`combat_engine`/`combat_world` fixtures — the fixture builder gains
xp/inventory knobs or per-test entity mutation).

| # | Test | Proves |
|---|---|---|
| X1 | victory over a 7-xp hostile via the PULSE path → `player.xp == 7` in the snapshot; `CombatEnded` text exactly `"You have defeated Stray. Victory! You gain 7 XP."` | REQ-001 |
| X2 | victory via the #25 POWER-STRIKE window over a 7-xp hostile → xp exactly 7 (awarded once, not doubled); same text | REQ-001 |
| X3 | xp-less victory → text byte-identical to the old `"You have defeated Stray. Victory!"`, `player.xp == 0` (existing victory tests double-cover) | REQ-002 |
| X4 | victory over a hostile owning `[fang]` → drop line exactly `"The Stray drops Cracked Fang."`, fang visible in `room.contents`, `take fang` succeeds → pack contains it, entity inventory empty (internal) | REQ-003 |
| X5 | no-inventory victory → zero drop lines, no phantom items in contents, removal semantics intact | REQ-004 |
| X6 | no-duplicates: fled first encounter (inventory intact, nothing dropped) → re-engage → victory → exactly ONE fang placement + one drop line | REQ-003 |
| X7 | defeat at xp 100 → at start room (`current_room_id` by value), hp == max, combat `None`, enemy placement still present, `xp == 90`; text exactly `"…You wake at Hollowmere…You lose 10 XP."`-shaped (full string pinned); `RoomEntered{start}` + description events follow `CombatEnded` | REQ-005 |
| X8 | penalty floor: defeat at xp 5 → lose exactly 1 → 4; at xp 1 → 0 | REQ-005 |
| X9 | defeat at xp 0 → xp stays 0, text has NO penalty clause (full exact string), reset (room/hp/combat) still correct | REQ-006 |
| X10 | defeat via the pulse path (rewrite of the #24 pinned test) → same reset semantics under `tick()` | REQ-005 |
| X11 | after defeat: walk back to the field and `attack` again → a fresh encounter starts (enemy survived in place) | REQ-005/009 |
| X12 | fled outcome text byte-unchanged; the three outcome texts each pinned exactly (victory+xp / defeat+penalty / fled) | REQ-007 |
| X13 | content: beginner `ashen_stray` has `xp == 5` + `inventory == ["stray_fang"]`; `stray_fang` item exists with name "Cracked Fang"; the world validates; Bell-Eater xp defaults 0 | REQ-008 |
| X14 | beginner smoke: walk to ashen_road, win, `player.xp == 5`, take the fang → in pack (the playable loop) | REQ-008 |
| X15 | constructed world: an entity inventory id missing from the registry → `EntityItemMissing` (the existing contract — regression pin for the drop path's assumption) | REQ-003 |
| X16 | serde: TOML `combat = { health = 9 }` → xp 0; `combat = { …, xp = 5 }` → 5 (covered through the content + constructed-world tests, no new dev-deps) | REQ-002 |

**Deliberate updates (not regressions):**
`defeat_at_zero_hp_revives_player_and_clears_combat` → asserts the start-room
reset; `pulse_defeat_at_exact_zero_revives_and_stops_pulsing` → same under
ticks (X10 is its rewrite). No other pinned string touches the defeat text.
**Genuinely uncoverable:** none new (browser smoke only for the played loop,
as ever).

### Risks / decisions
- **R1 — zero-xp text byte-preservation is load-bearing:** it keeps every
  existing victory assert green and REQ-002 honest; the conditional text is
  the only branch — both arms pinned (X1/X3). Same pattern on the defeat
  penalty clause (X7/X9). `PR-claude-enumerate-variant-string-arms-001`
  satisfied: every text variant has an exact test.
- **R2 — defeat-semantics change blast radius:** exactly two pinned tests
  rewritten (enumerated above); the old "You wake later" string exists only
  in `end_combat` + those tests (verified by grep at implement).
- **R3 — registry-survives-removal invariant:** xp/drop lookups run off
  `enemy_id` after the encounter — `remove_entity_everywhere` touches room
  placement lists only, never `world.entities` (the #22 `take_at` precedent).
  The total-chain lookup (`map_or(0, …)`) keeps this off the panic path
  even for a hypothetical missing entity (§14 — no expect on the lookup).
- **R4 — drop ordering + room duplicates:** authored order appended;
  duplicate ids in a room are legal placements (the Vec model — `take`
  takes the first). Documented, not coded around.
- **R5 — carried-forward in-memory mutation** (#22 R5): drops/removals/xp
  live in memory; save wiring remains a future ticket.
- **R6 — mutation pins:** `+=` on xp (X1's exact 7), `/ 10` and `.max(1)`
  (X7's 100→90 kills `/`→`*` and `+`; X8's 5→4 kills `max`→`min`),
  `saturating_sub` boundary (X8's 1→0), helper fn-replace mutants (exact
  strings + contents asserts), the `xp > 0` guards on both text clauses
  (X3/X9's full-string asserts).

## Phase 3 — Implement
- **Built (manifest rows 1–2; tests row → Validate, docs → Complete):**
  - **core** (`oathstar-core/src/lib.rs`): `CombatProfile.xp: u64`
    (`#[serde(default)]`, doc-commented — REQ-002's never-invented default);
    `enemy_xp_reward` (the total `get → and_then → map_or(0)` chain — no
    defensive arms); `drop_enemy_inventory` (`mem::take` the inventory →
    per-item `"The {enemy} drops {name}."` `CombatMessage` → append ids to
    the current room's placements; expect-invariants on the registry/room
    lookups per house style); `apply_defeat_penalty` (`xp == 0 → 0`, else
    `(xp/10).max(1)` saturating; returns lost); `end_combat` Victory arm
    (remove → drop → award → conditional XP text, zero-xp branch
    byte-identical to pre-#26) + Defeat arm (restore HP → penalty →
    relocate to `start_room_id` → conditional penalty clause) + the Defeat
    wake-up arrival (`RoomEntered` + `describe_current_room`, the movement
    pattern) after the `CombatEnded` push; fn doc updated.
  - **literal back-fill:** all 11 `CombatProfile` literal constructions
    (9 core fixtures + 2 content asserts) gained `xp: 0` via a guarded perl
    pass; the content-crate stray assert then set to `xp: 5` to match the
    new authoring.
  - **content** (`modules/beginner/world.toml`): `ashen_stray` gains
    `inventory = ["stray_fang"]` + `combat = { …, xp = 5 }`; new `[[items]]`
    `stray_fang` ("Cracked Fang", trophy, aliases fang/cracked fang).
    (Noted: the Bell-Eater already authored `inventory = ["bell_clapper"]` —
    the existing-field discovery confirmed in content; it remains
    confront-only so no combat-drop path touches it.)
  - **Untouched as designed:** protocol, datastar, command.rs, JS client,
    rooms.toml.
- **Deliberate pinned-test updates (the authorized #26 semantics change):**
  1. `defeat_at_zero_hp_revives_player_and_clears_combat` →
     `defeat_at_zero_hp_resets_to_start_at_full_hp` (exact zero-XP defeat
     text — no penalty clause; `RoomEntered{start}`; room/hp/xp by value).
  2. `pulse_defeat_at_exact_zero_revives_and_stops_pulsing` →
     `pulse_defeat_at_exact_zero_resets_and_stops_pulsing` (xp 100 seeded →
     exact "You lose 10 XP." text; 90 after; arrival events; stops pulsing).
     The old burst-length `== 5` assert was replaced by content asserts
     (the arrival events made the count fixture-dependent).
  3. Server S1 `pulses_stream_combat_to_subscribers_until_victory`: the
     beginner stray now drops + rewards, so the exact sequence is 10 events —
     the new `kinds[8]` drop-line and `kinds[9]` XP-bearing victory text are
     pinned, plus `/state` `xp == 5` (a content-driven strengthening of the
     #24 integration pin).
- **Compile/check (this phase):** `cargo check --workspace --all-targets`
  clean; `cargo fmt --all`; `cargo clippy --workspace --all-targets
  --all-features -- -D warnings` **GREEN first run**; full regression:
  all 11 Rust suites green (core 194, server 16 incl. the updated S1),
  `node --test` 46/46, `npm run build` OK.
- **Deviations from design (+ reason):** one addition — the S1 server-test
  update above wasn't in the manifest (the design missed that authoring the
  demo reward changes the beginner-world integration pin); handled as a
  deliberate update in the same spirit as the two enumerated defeat tests.
  Otherwise implemented exactly to the manifest and pinned strings.

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel `general-purpose` critics over `git diff HEAD` @
  base `0d3489f`): (1) correctness/state-coherence — defeat-reset coherence
  (relocation preserves pack/oath/discovered; exact burst tail order;
  start==encounter no-op clean; queued actions die with the taken state),
  victory/drop edges (fled-then-victory once; cross-room loot lands at the
  player; Bell-Eater clapper combat-unreachable; no double-defeat path),
  penalty table (0/1/9/10/11/19/20/1M/u64::MAX), REQ-008 played end-to-end
  on the real beginner world. (2) mutation hygiene — 17 new mutants
  enumerated with kills, expect-invariant audit (all three true invariants,
  registry-removal grep), string-arm scorecard, the xp:0 back-fill audit
  (the "11th" = the pre-existing PlayerState init — all 9 fixture sites
  correct).
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | I1 | HIGH | **Relocation is vacuously tested**: `combat_world`'s start room IS the encounter room, so `current_room_id = start_room_id` (lib.rs ~1920) is a no-op in every written core defeat test, and cargo-mutants generates NO mutant for the assignment — deleting the line would pass MSI 100% | **REAL, test-design** (no code change) | BINDING Phase-4 row: stage a defeat with encounter ≠ start (hostile placed off-start in a fixture variant AND/OR the beginner-world defeat leg in X14) asserting wake-room ≠ encounter-room by value |
  | I2 | MEDIUM | **Server pins cannot kill core mutants**: cargo-mutants runs only the mutated package's tests, so S1's exact-text/xp pins are out of mutation scope — X1 (xp award), X3 (zero-xp victory text), X4 (drop line) MUST be core engine-level tests; today 8/17 new mutants survive pending them | **REAL, test-plan mechanics** (expected pre-validate; the mechanic itself is the lesson) | Phase-4 kill-list updated; X3 written as a real core exact-string test, not a "double-covered" wave |
  | I3 | LOW | Victory XP `+=` overflow-panics in debug at u64 extremes (scratch-proven `#[should_panic]`); asymmetric with the defeat path's saturating discipline (lib.rs ~1908) | **REAL** (theoretical reach; real panic) | `saturating_add` + why-comment |
  | I4 | LOW | Per-item name `.clone()` in the drop loop is avoidable (format while the borrow is live) (lib.rs ~1989) | **REAL** (micro) | format-then-push; clone removed |
  | I5 | LOW | Defeat arrival cloned the full `RoomDefinition` for two strings (lib.rs ~1944) | **REAL** (micro) | clone `(id, title)` only |
  | I6 | LOW | Same-id duplicate placements: #26's drop is the first mechanism that can materialize a duplicate item id in a room from VALID content, and `take_at`'s `retain` then strips ALL copies (one silently destroyed). Pre-existing retain-all; unreachable in shipped content (the fang exists only in the stray's inventory) | **ACKNOWLEDGED, pre-existing interaction** | Noted for a future ticket: positional removal in `take_at` or a cross-placement uniqueness validation; out of #26 scope |
  | R1 | INFO | T8's "arrival follows the summary" message is membership-checked, not order-checked (order verified correct by scratch) | **ACCEPTED** (validate may pin positions if cheap) | optional |
  | R2 | INFO | Drop line hardcodes "The " (a "The The Bell-Eater" would read badly; no such hostile exists or is reachable) | **ACKNOWLEDGED** | content-time concern |
  | R3 | INFO | Adjacent-room kills drop loot at the player's feet (interaction radius 1) — coherent with the engagement model | **ACCEPTED** | none |
  | R4 | INFO | `.max(1)`/`saturating_sub`/`is_empty`-return/`matches!` guard generate NO mutants — the by-value rows (T8's 90, X8's 4, X5's absence, event-count pins) are their sole behavioral pins | **carry-forward** | keep those rows exact |
- **Mutation kill-list (binding for Phase 4, from critic 2):** X1 = pulse
  victory over an xp-7 fixture — `snapshot.player.xp == 7` BY VALUE (sole
  killer for `+=`→`*=`) + exact gain text (kills the `>` trio + fn-replace);
  X3 = CORE zero-xp victory full exact string (sole killer for `>`→`>=`) +
  `xp == 0`; X4 = core drop test — exact line + contents + take→pack (kills
  `drop_enemy_inventory → ()`); X5/X6 absence/no-dup; penalty mutants
  already killed by the rewritten T8 (90/exact text) + the no-clause defeat
  string; NEW binding row per I1 (relocation distinguishability).
- **Verification of fixes:** `cargo fmt --all --check` clean; clippy strict
  GREEN; all 11 Rust suites green; JS 46/46. No `failure-record` — no
  shipped behavioral bug (I3 was a theoretical-reach panic caught pre-ship;
  I1/I2 are test-design lessons → prevention-rule candidates at Complete).

## Phase 4 — Validate
- **Tests added (11 new: 9 core + 1 content + the extended server S1):**
  - `oathstar-core` (9, + the `spoils_engine`/`relocated_stray_engine`
    fixtures): `pulse_victory_awards_authored_xp` (X1 — xp 7 BY VALUE, the
    sole `+=`→`*=` killer, + exact gain text),
    `window_victory_awards_xp_exactly_once` (X2 — the #25 P2 path through
    the same funnel), `zero_xp_victory_keeps_the_original_summary` (X3 —
    the in-core exact old string, killing the `>`→`>=` text-guard mutant),
    `victory_drops_inventory_into_the_room` (X4 — exact drop line, contents,
    `take`→pack, cleared inventory),
    `victory_without_inventory_drops_nothing` (X5),
    `drops_happen_exactly_once_after_a_flee` (X6 — one line, one placement,
    across a full pulse-driven refight),
    `defeat_away_from_start_relocates_to_the_start_room` (the inspect-I1
    BINDING row — encounter in the clearing, wake at field, exact text with
    "You lose 4 XP.", enemy left in place — the relocation assignment is now
    genuinely distinguishable), `defeat_penalty_floors_at_one_and_zero`
    (X8 — 5→4, 1→0), `player_can_reengage_after_a_defeat_reset` (X11).
  - `oathstar-content` (1): `beginner_stray_authors_the_reward_loop` (X13 —
    inventory, Cracked Fang/trophy/alias, world validates; the xp=5 profile
    assert was updated at implement).
  - `oathstar-server`: S1 extended with the X14 played-loop leg
    (`take fang` → accepted → "Cracked Fang" in the pack).
  - X7/X9/X10 live in the implement-phase rewrites of the two defeat pins
    (exact penalty/no-clause strings, snapshot values, arrival events);
    X15 (`EntityItemMissing`) pre-existed; X16 serde defaults covered by the
    content asserts (Bell-Eater no-xp-key → 0). X12's fled text is pinned by
    the untouched #24/#25 exact-string tests.
- `cargo test --workspace`: **GREEN** — core **203**, content **21**,
  datastar 14, protocol 19, server 16 (S1 extended), storage 20; 0 failed.
  All 11 new/extended tests passed on the first run.
- `node --test tests/*.test.js`: **46 pass / 0 fail** (no JS changes — REQ-007
  rides existing surfaces; suites run per the owner instruction).
- `npm run build`: OK.
- `bin/gate.sh --fast`: **GATE GREEN [fast] — 14/14 PASS** (gates 15–17
  coverage+mutation SKIPPED per the `--fast` owner instruction; the
  inspect-derived kill-list is written so the FULL gate's mutation run has
  its killers in core — to be confirmed when the owner authorizes the FULL
  gate before `/commit`).
- Pre-existing exclusions: none encountered.

## Phase 5 — Complete
- **Docs updated:** `decisions.md` Decision 044 (rewards/consequences through
  the end-combat funnel — award/drop/reset semantics, byte-preservation,
  penalty formula, revisit triggers incl. the duplicate-placement edge);
  `combat-system.md` "Rewards + defeat consequences v1 implemented"
  blockquote; `entity-model.md` (combatant contract row gains
  `disclose_stats, xp`; new #26 paragraph — inventory mechanically live).
- **Forge capture:** AAR `e152bf2c` closed (`completed`, effectiveness 5,
  25 verdicts, 3 novel findings; jobs enqueued). No failure-records (no
  shipped behavioral bug). `prevention-rule-record` ×2:
  **PR-claude-fixture-distinguishable-transitions-001** (`b64edaab`) — stage
  fixtures where state transitions are observable (the vacuous relocation
  test); **PR-claude-package-scope-mutation-001** (`03b18e26`) —
  cargo-mutants is package-scoped: killers must live in the owning crate.
  `architecture-decision-record` **AD-claude-combat-reward-loop-001**
  (`2aadcccb`).
- **Ticket closed:** forge `aa8c7f72` → `done` (closing comment `10643231`);
  local doc moved `tickets/open/ → tickets/closed/`, frontmatter updated.
- **Archived:** `WORK-combat-rewards-and-defeat-consequences-v1.{spec,notes}.md`
  moved `pipeline/active/ → pipeline/completed/`; spec
  `status: Phase 5 — Complete PASS`.
- **STOPPED BEFORE `/commit`** per owner instruction: the FULL gate (15–17)
  and the commit are owner-gated. Branch
  `codex/oathstar-ticket-26-combat-rewards` carries the uncommitted #26
  implementation on top of `0d3489f` (main).
