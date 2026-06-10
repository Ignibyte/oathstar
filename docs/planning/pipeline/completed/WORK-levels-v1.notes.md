# WORK-levels-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #30 — Levels v1: XP milestones become levels, with a
  visible benefit, a typed `LevelUp` event, and a thin HUD level+XP
  display. OWNER AUTHORIZED END-TO-END: auto-approve all phases AND
  `/commit` (fast gate at validate; FULL gate at `/commit`; commit +
  ff-merge + push). Stop only on gate failure or scope conflict.
- **Intake source:** none — the #29 closeout follow-up ("25 boss XP lands
  in a stat that does nothing") + the carried #26 backlog item
  ("HUD xp display").
- **Classification / tier:** Work pipeline, one shippable slice — a pure
  curve fn, two recompute call sites, one protocol variant + renderer
  arms, one view-model field + one DOM line, tests.
- **Base verified:** branch `codex/oathstar-ticket-30-levels-v1` off
  `main` @ `ee4165a` (#29 merged + pushed). No active pipeline; forge up;
  no bulletins; Codex strays untouched.
- **Forge recall + anchors verified at plan:**
  - `PlayerState.level: u32` exists, set to 1 at `try_new`, serialized in
    saves, copied into `PlayerSnapshot.level` (protocol line ~77; core
    snapshot mapping ~1122). One existing test pins level 1.
  - Both xp-change sites are in core: the victory award
    (`saturating_add` in `end_combat`) and `apply_defeat_penalty`
    (`saturating_sub`).
  - **The client renders NEITHER level nor xp** — `toHud` carries
    hp/focus/percentages/room/tick only; no menu panel shows xp. The
    "HUD already renders level" assumption from the request was FALSE;
    one thin `toHud` + DOM-line addition is in scope.
  - Standing rules that bind:
    **FAIL-claude-generic-log-twin-duplicates-typed-render-001 /** the
    renderer-arm check — before pairing a human line with `LevelUp`,
    read the datastar `describe()` + JS components arms; if the typed
    render is prose, the typed event IS the line;
    **PR-claude-operator-sweep-untrusted-arithmetic-001** — level/max-HP
    arithmetic gets the operator sweep (xp is save-reachable, so the
    curve fn and benefit math run on untrusted values);
    **PR-claude-fixture-distinguishable-transitions-001** — threshold
    boundary tests stage both arms (xp at threshold−1 vs threshold);
    **PR-claude-package-scope-mutation-001** — curve/benefit killers in
    core, slice killers in server, view-model killers in node tests.
  - ADs in play: reward-loop-through-end_combat (the award site),
    save-load boundary (046 — stale-pair posture), boss-encounter
    (047 — the progression demo's xp sources: stray 5, boss 25).
- **Ticket:** forge `379aa89b-be69-4aa5-93f8-b7754f1d1b63` (#30), local
  doc `docs/planning/tickets/open/TICKET-30-levels-v1-xp-milestones-become-levels.md`.
- **AAR opened:** `8c113101-9986-4014-93c6-e5d56dd84116`.
- **EARS requirements reviewed:** REQ-001..009 (verbatim in the spec):
  curve+event, benefit, penalty interaction, multi-threshold, no-twin
  rendering, HUD, save coherence, the served progression, preservation.

### Open design questions (for Phase 2 — Planner does NOT decide these)
1. **Curve placement + shape.** Engine consts (the
   `PLAYER_STRIKE_DAMAGE` precedent — simplest, mutation-pinnable) vs
   module-authored thresholds (content-tunable, needs validation). And
   the shape: an explicit threshold table vs a formula. Demo constraint:
   stray grind (5 xp each) reaches level 2; the boss (+25) lands level 3
   (candidate: thresholds 10 / 30 — two strays → 2; two strays + boss =
   35 ≥ 30 → 3; or boss-first 25 + one stray = 30 → 3 exactly).
2. **Recompute shape.** A pure `level_for_xp(xp) -> u32` + a
   `sync_level(&mut events)` applied after each xp mutation — or inline
   at both sites? Where does the function live so both sites share it?
3. **Ratchet vs de-level on xp loss.** The defeat penalty can drop xp
   below a threshold. Candidate: levels RATCHET (never decrease) — the
   stored level is max(stored, curve) — documented; or strict curve
   (de-level on defeat). Pick one, document, test both arms.
4. **The benefit.** Max-HP growth per level (+N each level? authored?)
   and heal semantics on level-up (heal to full vs +N current vs no
   heal). Keep it visible in the snapshot and deterministic. Interaction
   with defeat (which restores to max) noted.
5. **LevelUp payload.** `LevelUp { level }` vs `{ level, max_hp }` vs
   `{ from, to }`. Channel (System? a new Progress channel? — existing
   channels only, no protocol channel additions unless trivial).
6. **Renderer arms.** datastar `describe()` + `kind_type` arm for
   `level_up`; JS components arm. The #29 rule: decide whether the typed
   render IS the human line (then no log) — likely yes ("You reach
   level 2." as the typed render).
7. **Stale-save posture.** A v2 save can carry level 1 with xp 35 (saved
   pre-#30 semantics? No — v2 saves are #28/#29-era, level always 1).
   Loading: recompute-on-load (silently levels the player up on load —
   surprising but honest?) vs tolerate (stored level stands until the
   next xp change syncs it) vs version bump. With ratchet semantics,
   "sync on next xp change" converges naturally; recompute-on-load is
   cleaner. Decide + document; the curve makes no stored pair UNSOUND
   (no panics either way), so 046's loud-refusal posture may not
   require a bump — design confirms with the operator sweep.
8. **Multi-threshold single award.** One award crossing two thresholds:
   one LevelUp event per level vs one event landing at the final level.
   Decide + pin.
9. **HUD shape.** `toHud` gains `level` + `xp`; one DOM line (e.g. "Lv 2
   · 35 xp" near the turn counter or hp block — match index.html's
   existing structure). Which element + format, at design after reading
   index.html.
10. **Does the battle modal / character menu need the level?** OUT
    (HUD only) unless a one-liner falls out free — design confirms OUT.

## Phase 2 — Design

### Approach / architecture (the 10 open questions, resolved)

1. **Curve (Q1): an engine-const threshold table.**
   `const LEVEL_XP_THRESHOLDS: [u64; 4] = [10, 30, 60, 100]` — level 1 at
   0 xp, +1 per threshold crossed; max level 5 when the table is
   exhausted (v1's cap, documented). The `PLAYER_STRIKE_DAMAGE`
   precedent: module-authored curves need validation machinery and are a
   future ticket. Pure fn `level_for_xp(xp: u64) -> u32` (count of
   thresholds ≤ xp, +1; `u32::try_from(..).unwrap_or(u32::MAX)` total).
   Demo math: two strays (10) → level 2; +boss (35 ≥ 30) → level 3 — or
   boss-first 25 → 2, +one stray 30 → 3 exactly. Both orders satisfy
   REQ-008.
2. **Recompute (Q2): one shared `sync_level(&mut self, events)`** called
   after BOTH xp mutations (the `end_combat` victory award and
   `apply_defeat_penalty`). It loops `while level < level_for_xp(xp)`:
   `level.saturating_add(1)`, `max_hp.saturating_add(LEVEL_UP_MAX_HP_GROWTH)`,
   **heal to full** (`hp = max_hp`), push one `LevelUp` event — so a
   single award crossing two thresholds emits one event PER level with
   each benefit applied exactly once (Q8 answered by the loop shape).
3. **Ratchet (Q3): levels never decrease.** `sync_level` only raises;
   the defeat penalty lowers xp but the milestone is kept (no max-HP
   clawback, no de-level whiplash). Re-crossing an already-held
   threshold is a natural no-op (`level >= target` → loop doesn't run) —
   no double benefit, no duplicate event. Both arms staged in tests.
4. **Benefit (Q4):** `const LEVEL_UP_MAX_HP_GROWTH: i32 = 5` and heal to
   full on each level gained — the classic milestone moment, visible in
   the snapshot, and victory-ordered (award → sync) so the renewal lands
   after the fight's damage. No interaction conflict with defeat (defeat
   restores hp first; the penalty then lowers xp; ratchet means sync is
   a no-op there).
5. **Event (Q5):** `GameEventKind::LevelUp { level: u32, max_hp: i32 }`
   (additive serde, camelCase fields like its siblings; wire tag
   `level_up`). Channel `EventChannel::Skill` — progression's home;
   datastar maps Skill to ("system", "Skill"), so the feed line reads
   `Skill — You reach level 2.`.
6. **Renderers (Q6), the #29 no-twin rule applied:** the typed render IS
   the human line — core emits NO LogMessage twin. datastar `describe()`
   arm → `(channel variant, label, format!("You reach level {level}."))`;
   `kind_type` arm → `"level_up"`; the escape-guard all-kinds test array
   gains LevelUp. JS `components.js`: `textFor` case `"level_up"` →
   same string; label map entry. `client-app.js`: the SSE refresh list
   gains `"level_up"` (level-ups from pulse victories arrive with no
   command response; combat_ended already refetches, but the explicit
   entry is one honest line of armor).
7. **Stale-save posture (Q7): lazy convergence, no version bump.** The
   payload FORMAT is unchanged; a v2 save with level 1 / xp 35 loads
   fine and converges at the next xp change (sync then fires the earned
   LevelUps — true milestones, just announced late). Recompute-on-load
   would mutate silently with no observable events (load broadcasts
   nothing — #28 Q7). Crafted pairs are SOUND either way (operator
   sweep: level/max_hp arithmetic saturates; a crafted level 99 ratchets
   — weird, never unsound; level 0 syncs up to the curve at the next
   change). No new from_save gates.
8. **HUD (Q9):** `index.html` `header-status` gains
   `<span id="level-value">Lv 1 · 0 xp</span>` beside the turn counter;
   `toHud` gains `level` + `xp` (defaulted like its siblings);
   `renderAll`'s HUD block sets
   `el.levelValue.textContent = `Lv ${hud.level} · ${hud.xp} xp``.
   Pure view-model + one DOM line — no redesign. Battle modal/menus:
   OUT confirmed (Q10).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-protocol/src/lib.rs` | `GameEventKind::LevelUp { level, max_hp }` (additive; doc-commented). |
| 2 | `crates/oathstar-core/src/lib.rs` | `LEVEL_XP_THRESHOLDS` + `LEVEL_UP_MAX_HP_GROWTH` consts (doc-commented); `level_for_xp` pure fn; `Engine::sync_level`; call after the victory award in `end_combat` and after `apply_defeat_penalty`'s xp write. |
| 3 | `crates/oathstar-datastar/src/lib.rs` | `describe()` LevelUp arm ("You reach level {level}."); `kind_type` → `"level_up"`; escape-guard kinds array entry. |
| 4 | `src/client/components.js` | `textFor` case + label map entry for `level_up`. |
| 5 | `src/client/snapshot.js` | `toHud` gains `level` / `xp`. |
| 6 | `src/client-app.js` | `el.levelValue` lookup; HUD render line; `"level_up"` in the SSE refresh list. |
| 7 | `index.html` | the `level-value` span in `header-status`. |
| 8 | docs (Phase 5): `docs/decisions.md` (Decision 048), `docs/mechanics-and-systems.md` (Growth/levels shipped), `docs/protocol-and-output.md` (event list) if it enumerates kinds. |

**No changes:** `oathstar-server` (non-test), `oathstar-storage`,
`oathstar-content`, `command.rs`, module TOML, CSS.

### Regression Test Plan
≥1 per EARS REQ; thresholds both-arms; package-scoped kills.

| # | Test | Proves |
|---|---|---|
| L1 | `level_for_xp` boundary table: 0→1, 9→1, 10→2, 29→2, 30→3, 59→3, 60→4, 99→4, 100→5, u64::MAX→5 (each threshold's both arms — the table-value killers) | REQ-001 |
| L2 | victory award crosses a threshold: spoils fight to xp 10 → `LevelUp { level: 2, max_hp: 25 }` on the Skill channel in the VICTORY response (after the CombatEnded event), snapshot level 2 / max_hp 25 / hp 25 (healed to full) | REQ-001/002 |
| L3 | a single award crossing two thresholds (authored xp 35 from level 1): exactly TWO LevelUp events in order (level 2 then 3), max_hp 30, healed once per level — and a follow-up award below the next threshold emits nothing (no re-fire) | REQ-004 |
| L4 | ratchet: reach level 2 at 10 xp → defeat (penalty → 9 xp) → level STAYS 2, max_hp stays 25, NO LevelUp event; the next win to 14 xp emits nothing (already held); both arms vs L2 | REQ-003 |
| L5 | save round-trip: level up, save, load → byte-identical snapshot (the existing #28 harness covers; one explicit pin on level/max_hp in the payload); stale-pair convergence: a crafted v2 payload with level 1 / xp 35 loads, and the NEXT award fires the two earned LevelUps (lazy convergence pinned) | REQ-007 |
| L6 | datastar: `feed_patch` on a LevelUp renders the exact line `You reach level 2.` with the Skill label + system variant + `data-component="level_up"`; the escape-guard array compiles with the new kind; NO LogMessage twin in the engine's level-up burst (asserted in L2's event list) | REQ-005 |
| L7 | node `toHud` exposes `level`/`xp` (defaults 0/absent-safe like siblings); `textFor("level_up")` returns the same exact line as datastar (the cross-renderer consistency pin) | REQ-006 |
| SV-L | server slice extension (the existing two-act boss tests): act 1's victory now ALSO pins level 3 / max_hp 30 in /state after the boss falls (talk→swear→stray? — the slice route has no stray kill; instead extend act 1's post-victory asserts: xp 25 → level 2 / max_hp 25; then a stray grind variant OR adjust: the slice fights ONLY the boss → 25 xp → level 2. The played stray-then-boss progression (5+5+25 = 35 → level 3) gets its own paused-time server test reusing walk_to_ashen_road + drain) | REQ-008 |
| — | preservation: all existing suites + `npm run build` | REQ-009 |

**Note on SV-L:** the boss-only slice lands level 2 (25 xp); the
three-kill progression test (stray ×2 → level 2 mid-grind, boss → level
3) is the REQ-008 demo. Existing tests that pin `player.hp 12` after the
boss fight (act 1) will CHANGE: victory at 25 xp levels to 2 and heals to
25/25 — the act-1 hp pin updates from 12 to 25, and S1's stray test pins
update (stray victory at 5 xp: no level-up, hp pins unchanged ✓ — only
boss-victory pins move). Audit at validate.
**Genuinely uncoverable:** none.

### Risks / decisions
- **R1 — heal-to-full changes existing boss-test pins** (hp 12 → 25/25
  after victory): deliberate, audited at validate; the stray fight (5 xp,
  no threshold) keeps every #22–#26 pin byte-identical.
- **R2 — the cap is the table.** Level 5 max in v1; xp keeps
  accumulating (saturating); a longer table is a one-line content
  decision later.
- **R3 — Skill channel reuse** (first emitter) — if a dedicated Progress
  channel is ever wanted, it's a protocol addition then, not now.
- **R4 — lazy convergence on stale saves** announces earned milestones
  at the next xp change rather than on load — documented in Decision
  048; revisit if load ever broadcasts.
- **R5 — mutation pins:** the threshold table values (L1's both-arms
  ladder), the while-guard `<` (L3's exactly-two + L4's no-op arms — a
  `!=` mutant either no-ops or hangs, both caught), `saturating_add`
  sites (L1's MAX row, L3's by-value max_hp), the heal assignment (L2's
  hp == max_hp), the Skill-channel + exact-line renders (L6/L7), the
  refresh-list entry (review-pinned; JS coverage floor).

## Phase 3 — Implement
- Built (to the manifest; workspace check + strict clippy clean, node
  parses, `npm run build` green):
  - **protocol:** `GameEventKind::LevelUp { level, max_hp }` (additive,
    doc-commented with the no-twin note).
  - **core:** `LEVEL_XP_THRESHOLDS = [10, 30, 60, 100]` +
    `LEVEL_UP_MAX_HP_GROWTH = 5` consts; `level_for_xp` (pure, total —
    `try_from(..).unwrap_or(MAX).saturating_add(1)`); `Engine::sync_level`
    (the ratcheting while-loop: level/max_hp saturating, heal to full, one
    Skill-channel `LevelUp` per level); ONE call site in `end_combat`
    right after the `CombatEnded` push (covers victory award AND defeat
    penalty — the penalty arm is a documented ratchet no-op; the defeat
    feed order stays CombatEnded → arrival narration).
  - **datastar:** describe arm ("You reach level {level}." via
    `channel_variant_label`) + `kind_type` `"level_up"` — both surfaced
    by the exhaustive-match compile errors, as the #27 rule intends.
  - **client:** `components.js` `textFor` case (byte-identical line,
    sibling-style fallback); `snapshot.js` `toHud` gains `level`/`xp`;
    `client-app.js` `el.levelValue` + the `Lv N · M xp` render line +
    `"level_up"` in the SSE refresh list; `index.html` `level-value`
    span beside the turn counter.
- Deviations from design (+ reason):
  - **No components.js label-map entry** — the design assumed a
    type→label map; labels actually flow from the CHANNEL map, where
    `skill → "Skill"` already matches datastar's. Zero-change correct.
  - **One sync call site, not two** — `end_combat` is the single funnel
    both xp mutations flow through; calling sync once after the
    CombatEnded push covers both (and Fled trivially no-ops). Simpler
    than instrumenting `apply_defeat_penalty` separately.
- **Known churn handed to validate (3 red, all the designed R1 class):**
  core `defeat_away_from_start_relocates_to_the_start_room` +
  `pulse_defeat_at_exact_zero_resets_and_stops_pulsing` — their fixtures
  set xp directly (e.g. 40 at level 1), so the defeat's sync performs the
  DESIGNED lazy convergence and levels mid-defeat, moving hp/max pins;
  server `beginner_slice_fights_the_boss_to_victory` — the boss's 25 xp
  now lands level 2 (max 25, healed 25; the old hp-12 pin moves). Node
  46/46 green.

## Inspect (Phase 3.5)
- Lenses run (2 critics, parallel): progression correctness + crafted-save
  sweep; renderer/client/docs consistency. Both verified concretely
  (scratch tests 6/6 then deleted; worktree confirmed restored).
- Findings:
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | MAJOR | `wire.js parseEvent` drops the LevelUp payload — its default arm strips `level`, so `textFor`'s "dead defense" fallback was the LIVE branch through the canonical parse→render composition; byte-identity with datastar was unachievable (the same drop already silently afflicts combat/announcement texts, masked by hand-built test events). | REAL | `level_up` case added to the switch (`level`/`maxHp` preserved, camelCase per the roomId convention). FAIL-claude-normalizer-strips-payload-live-fallback-001 recorded; validate's L7 pins THROUGH `parseEvent(rawWireJson)`, never a hand-built event. |
  | 2 | INFO (docs) | The Fled doc ("player keeps their current HP") and `sync_level`'s doc ("surfaces at its next XP change") both miss that lazy convergence fires at ANY combat end — a stale save that flees levels + heals. | REAL (doc) | Both doc-comments amended; behavior itself is sound, desirable convergence. |
  | 3 | MIN (validate must-do) | The datastar LevelUp arms have zero pins and the escape-guard all-kinds array (datastar tests ~:484) lacks the LevelUp row the design promised — MSI 100 would fail at `/commit`. | REAL (deferred by phase contract) | Handed to validate: L6 pins (exact line, "level_up" kind_type, Skill label) + the array row. |
  | 4 | INFO (Phase 5) | Doc staleness inventory: `protocol-and-output.md` kind list (stale pre-#30, now missing LevelUp too), `ui-design.md` HUD line (now shows level/XP), `vertical-slice.md` "skill progression deferred" (narrow to percentage-skills), Decision 048 MUST land (the `sync_level` doc cites it). | REAL (deferred) | Appended to the Phase 5 doc list. |
- Rejected / accepted-as-is (with verification):
  - **level_for_xp boundaries** — all ten pairs scratch-proven exact
    (0→1 … u64::MAX→5); the conversion chain total; the `unwrap_or` arm
    defensively dead (count ≤ 4).
  - **sync_level terminates under ALL crafted state** — proven by bound
    (target ≤ 5; the loop body only runs at level ≤ 4, strictly
    increasing) AND empirically at the worst pairs (level u32::MAX,
    level 0, max_hp near i32::MAX — saturates, no panic, no loop). A
    crafted level-0 save levels to 1 with one event — sound.
  - **Defeat-arm convergence order** — `[CombatEnded, LevelUp(s),
    RoomEntered, description]` is renderer-coherent; "battered but whole"
    then waking stronger is a stale-save-only cosmetic quirk (normal play
    can never sync at defeat — the victory award already synced; proven).
  - **Operator sweep** — clean: comparisons/saturating/assignment only;
    the negative-crafted-max_hp exposure is pre-existing (defeat arm),
    not widened.
  - **Victory order + modal consistency** — `[CombatEnded(Victory),
    LevelUp]` same response, hp 25/25, combat None. Pin holds.
  - **xp-writer census** — exactly two product writes (award, penalty),
    both upstream of the single sync site; a FUTURE xp source must call
    `sync_level` (noted in Decision 048's revisit list).
  - **Channel/label parity** — "skill" → ("system","Skill") byte-equal in
    both renderers; server needs no awareness (bare serde serialize).
  - **toHud defaults / static seed** — sibling-consistent; the "Lv 1 ·
    0 xp" seed equals a fresh engine's truth (the existing HP seed is
    already a cosmetic placeholder precedent).
  - **`try_new` level-1 pin (~:3322)** — still valid (it pins seeding,
    not immutability).
  - **Refresh burst (+1 fetch per level)** — pre-existing unthrottled
    pattern, idempotent, loopback; accepted (future coalesce note).
  - **`.header-status` CSS nowrap** — extreme-squeeze wrap possible;
    manifest pins "no CSS changes"; accepted as a nit with the
    aria-label stretch noted for a future UI pass.
- Mutation-surface notes handed to validate: the four boundary pairs
  both-arms (kills `>=` flips + table values); the `while <` → `<=`
  mutant (exact final-level AND event-count pins); the `<` → `!=` mutant
  (DIVERGES only when stored level > curve — the defeat-after-victory
  ratchet test makes it hang and die by timeout; without it the mutant
  likely survives); the heal assignment (enter below max, assert == new
  max); exact 25/30 max_hp values; the datastar line + kind_type pins;
  L7 through the parse composition.

## Phase 4 — Validate
- **Churn rewrites (the 3 declared reds):**
  `pulse_defeat_at_exact_zero_resets_and_stops_pulsing` +
  `defeat_away_from_start_relocates_to_the_start_room` — kept their
  direct-xp fixtures and pinned the NEW converged values as the
  lazy-convergence demo in played form (level 4/35-35 and level 3/30-30,
  LevelUp counts pinned); server
  `beginner_slice_fights_the_boss_to_victory` — hp pin 12 → 25/25 plus
  NEW pins: `LevelUp { 2, 25 }` received on the Skill channel right after
  the drain (the `play_to_boss_victory` helper now returns the live
  subscription), /state level 2 / max 25.
- **New tests:** core L1 `level_for_xp_boundary_ladder` (all ten exact
  pairs incl. `u64::MAX`); L2 `victory_award_levels_up_with_benefit`
  (event order CombatEnded → LevelUp, by-value 2/25, healed from 19/20,
  exactly one event); L3 `multi_threshold_award_levels_once_per_level`
  (`vec![(2,25),(3,30)]` ascending); L4 `defeat_penalty_never_delevels`
  (the ratchet + the `<`→`!=` mutant timeout-killer — stored level above
  the curve); L5 `levels_round_trip_and_stale_saves_converge_lazily`
  (byte-identical save + the stale level-1/xp-35 pair converging with
  `vec![2, 3]` at its next combat end, NOT on load). datastar L6
  `level_up_renders_the_exact_line` (the line, `level_up` kind tag,
  system variant, Skill label) + the LevelUp row in the escape-guard
  all-kinds array. node: `toHud exposes level and xp` (with null-snapshot
  defaults) + `level_up renders ... through the parse composition`
  (parseEvent preserves level/maxHp — the inspect catch — and
  toComponent renders byte-identical "You reach level 2." with
  Skill/system; the stripped-field fallback also pinned). server SV-L
  `played_progression_reaches_level_three` (stray 5 xp → level 1 pinned
  mid-grind; boss +25 = 30 → the double burst `[(2,25),(3,30)]` and
  /state 3 / 30 / 30-30).
- `cargo test --workspace`: **ok — 343 passed, 0 failed** (core 239,
  server 25, content 23, storage 22, protocol 20, datastar 16).
- `node --test tests/*.test.js`: **48 pass, 0 fail**.
- `npm run build`: built in 74ms.
- `./bin/gate.sh --fast`: **GATE GREEN [fast] — 14/14** (first run caught
  one `doc_markdown` on a new helper doc-comment; backticks added at
  source). FULL gate runs at `/commit` (owner-authorized this ticket).
- Pre-existing exclusions: none.

## Phase 5 — Complete
- Docs updated:
- Forge capture (aar/failures/rules/decisions):
- Ticket closed:
- Archived:
