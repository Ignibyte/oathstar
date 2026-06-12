# WORK-focus-economy-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #31 — Focus Economy v1: the #25 battle skills cost
  focus, with typed refusals, recovery, and settled edge semantics.
  OWNER AUTHORIZED END-TO-END (auto-approve all phases AND `/commit`;
  fast gate at validate, FULL gate at `/commit`; commit + ff-merge +
  push; stop only on gate failure/scope conflict).
- **Intake source:** none — the #30 closeout follow-up ("power strike for
  free makes focus a dead stat").
- **Classification / tier:** Work pipeline, one slice — two cost consts,
  a spend/refusal hook in the existing queue machinery, one recovery
  path, tests.
- **Base verified:** branch `codex/oathstar-ticket-31-focus-economy` off
  `main` @ `aa31f1f` (#30 merged + pushed). No active pipeline; forge up;
  no bulletins; Codex strays untouched.
- **Anchors verified at plan:**
  - `state.player.focus`'s ONLY product-code reference is the snapshot
    copy (lib.rs:1149) — confirmed dead stat; `max_focus` likewise
    (seeded 5/5 at `try_new`).
  - The #25 queue seam: `queue_combat_action(action)` with
    `CombatAction::{queue_refusal, queue_confirmation, queue_already}`
    and the change-tack replace ("You change tack. …") — the refusal
    pattern the cost check joins.
  - Phase-2 resolution: `resolve_queued_action` takes the queued action;
    `resolve_guard` arms `guard_charge`; `resolve_power_strike` deals 6.
    A fight can end (Phase 1 kill) with an action still queued — the
    queued action is dropped with the cleared CombatState (#24) — the
    cost-settlement question design must answer.
  - `rest` is in ui-design's utility verb list but does NOT parse today
    (command.rs has no arm) — a new verb follows the #16/#25 parser
    pattern if design picks it.
  - HUD: focus bar renders `focus/maxFocus` already (toHud); the battle
    modal's participants carry hp/max_hp only (CombatantSnapshot) — any
    battle-modal focus display would need protocol surface (design
    keeps it OUT unless trivial; the HUD bar is visible beside the
    modal anyway).
  - Defeat currently restores `hp = max_hp`; focus untouched by any
    outcome — design decides the combat-end semantics.
  - Standing rules: both-arms boundary staging
    (PR-claude-fixture-distinguishable-transitions-001); operator sweep
    (PR-claude-operator-sweep-untrusted-arithmetic-001 — focus is
    save-reachable i32, any crafted value flows through new spend/regen
    math); package-scope mutation; the log-twin renderer check
    (FAIL-claude-generic-log-twin-…) if any new typed event appears
    (none expected — refusals are LogMessages); renderer-tests-through-
    the-normalizer (PR-claude-renderer-tests-through-the-normalizer-001)
    if any new event kind appears.
- **Ticket:** forge `bb63c3d7-b213-4e27-9417-ac64a40e92c6` (#31), local
  doc `docs/planning/tickets/open/TICKET-31-focus-economy-v1-battle-skills-cost-focus.md`.
- **AAR opened:** `1621584e-09a8-494c-b2a8-3c8ea231ee84`.
- **EARS requirements reviewed:** REQ-001..008 (verbatim in the spec):
  cost-once, boundary refusal, replace settlement, recovery both-arms,
  combat-end semantics, the served economy fight, crafted-save sweep,
  preservation.

### Open design questions (for Phase 2 — Planner does NOT decide these)
1. **Cost numbers + placement.** Candidate `POWER_STRIKE_FOCUS_COST: i32
   = 2`, `GUARD_FOCUS_COST: i32 = 1` as engine consts beside
   `POWER_STRIKE_DAMAGE`. With max 5: two power strikes + one guard =
   exactly 5 — the served-fight demo. Design locks.
2. **Spend point: on-queue vs on-resolve.** On-queue is simpler and
   matches the refusal point (you commit the focus when you commit the
   intent) but needs the replace settlement (refund the old cost, charge
   the new — net no exploit) and the fight-ends-queued rule (lost? or
   refunded?). On-resolve avoids refunds but lets a player queue with
   insufficient focus unless the check ALSO runs at queue → two checks.
   Design picks one, documents the exploit analysis.
3. **Flee's queue cost.** Flee remains free (locked) — but under
   spend-on-queue, replacing a PAID power strike with FREE flee must
   refund. Settle in the same rule.
4. **Recovery shape.** `rest` verb (parser + engine handler: out of
   combat restores focus to max — and hp too? mechanics doc says rest
   recovers; keep focus-only for the tight slice?) vs out-of-combat tick
   regen (+1 per N ticks — touches `tick()`, more moving parts) vs both.
   Mechanics doc leans rest. Design picks the smallest honest shape +
   exact lines.
5. **Combat-end focus semantics.** Victory: keep remaining focus (spent
   resources stay spent — the economy's bite)? Defeat: restore focus
   with hp (the #26 "battered but whole" reset — full reset is kinder
   and consistent)? Flee: keep as-is? Design decides + documents all
   three.
6. **Refusal line texts.** Per-action ("You lack the focus for a power
   strike." / "…to guard.") via the CombatAction enum (the
   queue_refusal precedent) — exact strings pinned.
7. **Crafted-save sweep.** Focus is `i32` — a crafted negative focus
   must not panic spend (saturating_sub) or block forever (refusal at
   negative is fine — rest recovers); crafted focus > max_focus
   tolerated or clamped at rest only? Design documents.
8. **Battle-modal visibility.** OUT unless trivial — the HUD bar sits
   beside the modal. Design confirms OUT (no protocol change).

## Phase 2 — Design

- **Process note:** forge MCP tools were not registered this session (sidecar
  was down at session start); started `start-all.sh` and recalled over the
  HTTP MCP seam directly (`/tmp/forge-call.sh` — initialize handshake +
  `tools/call`; 12 surfacings written via `knowledge-context`). Recall pulled:
  PR-oathstar-msi-test-assertions-001, PR-claude-enumerate-variant-string-arms-001,
  PR-claude-fixture-distinguishable-transitions-001,
  PR-claude-operator-sweep-untrusted-arithmetic-001, the #24 pulse/#28
  tick-task-panic lessons, AD-claude-direct-battle-verbs-001,
  AD-claude-save-load-untrusted-boundary-001.

### Approach / architecture (settles the 8 Phase-1 questions)

1. **Costs + placement (Q1).** Engine consts beside `POWER_STRIKE_DAMAGE`
   (lib.rs ~836): `POWER_STRIKE_FOCUS_COST: i32 = 2`,
   `GUARD_FOCUS_COST: i32 = 1`. Exposed per-action via a new
   `CombatAction::focus_cost(self) -> i32` const fn: Flee → 0, Guard → 1,
   PowerStrike → 2. Max 5 affords PS+PS+G exactly.
2. **Spend point: ON-QUEUE (Q2).** Check+spend are atomic at the one seam,
   `queue_combat_action` (lib.rs:1996). Flow inside the existing match:
   - *already* arm (same action re-queued): untouched — no charge, no refund
     (cost was paid at first queue; "deduct exactly once").
   - *fresh/replace* arms: `refund` = replaced action's cost (0 for fresh);
     `effective = focus.saturating_add(refund)`; **refusal gate
     `cost > 0 && effective < cost`** → typed refusal, accepted=false, no
     state change (old queue + its paid cost stay). Else
     `focus = effective.saturating_sub(cost)` and queue as today.
   - Spend-on-resolve was rejected: it splits check (queue) from spend
     (resolve) across sites that can drift when a future focus spender lands
     between them — the #24 "latent simplification + new execution context"
     failure shape.
3. **Change-tack settlement (Q3).** Refund-then-charge as above: no
   double-spend (refund exactly the paid cost), no free-cancel gain (PS→flee
   lands back at the pre-queue value, never above), replace-to-costlier at
   the boundary refusable (guard queued at focus 0 → PS refused, guard
   stays). Flee stays free **at any focus** — the `cost > 0` left-conjunct
   means a crafted negative focus can never soft-lock flee (locked rule
   "free verbs stay free").
4. **Fight-ends-with-action-queued: REFUND (Q2/Q3).** First thing in
   `end_combat` (lib.rs:2257): if the taken `CombatState.queued_action` is
   `Some(unfired)`, `focus.saturating_add(unfired.focus_cost())`. A fired
   action was already `take()`n by `resolve_queued_action`, so no double
   refund is possible. Order: refund precedes the outcome arms (defeat's
   restore below overwrites it — uniform rule, documented).
5. **Recovery: `rest` verb only, no tick regen (Q4).** Out of combat,
   `focus < max_focus`: set `focus = max_focus`, accepted=true. In combat:
   typed refusal. Already-full (`focus >= max_focus`): typed no-op,
   accepted=false — `>=` means crafted focus > max is never clamped down by
   rest. Focus-only (no HP) — defeat already restores HP; mechanics doc's
   "focus depletion blocks rituals until rest" is focus-shaped. Tick regen
   rejected: adds arithmetic to the tick task (the #28 silent-death surface)
   and softens the economy (waiting = free focus).
   Parser: `Command::Rest` via the existing `parse_bare_verb` arm
   (command.rs:254) — strict arity, no `parse()` growth (the #20
   too_many_lines trap avoided). Help line at lib.rs:1299 gains `rest`.
6. **Combat-end focus semantics (Q5).** Victory: keep remaining focus
   (spent stays spent — the economy's bite). Defeat: `focus = max_focus`
   beside the existing `hp = max_hp` (lib.rs:2281) — "battered but whole"
   full reset (#26 consistency). Flee: keep as-is. All three deterministic.
7. **Texts (Q6), pinned exactly.**
   - `CombatAction::focus_refusal(self)`: Flee → "You lack the focus to
     flee." (product-unreachable while flee is free — unit-covered, kept for
     table uniformity); Guard → "You lack the focus to guard."; PowerStrike
     → "You lack the focus for a power strike."
   - Insufficient-focus refusal channel: `EventChannel::System` /
     `OutputComponent::SystemMessage`, **no `combat.log` push** — REQ-002's
     "change no state" holds literally (the log is save-serialized state);
     matches the existing no-battle refusal family.
   - `rest`: in-combat "There is no rest in the midst of battle."
     (System/SystemMessage, accepted=false); already-full "You are already
     fully focused." (System/SystemMessage, accepted=false); success
     "You rest. Focus returns to you ({focus}/{max_focus})."
     (Narrative/NarrativeMessage, accepted=true).
   - Queue confirmation/already/no-battle lines stay **byte-identical**
     (REQ-008; HUD shows the spend).
8. **Crafted-save sweep (Q7).** New arithmetic sites, each cleared:
   spend `saturating_sub` (domain: only runs when `effective >= cost`, but
   saturating anyway, codebase idiom); refunds `saturating_add` ×2
   (change-tack, end_combat — crafted `focus = i32::MAX` + refund must not
   panic; saturates, no max-clamp, crafted >max tolerated — rest refuses as
   already-full, nothing panics, normal play never exceeds max);
   affordability compare on the saturated `effective` (crafted MAX loses the
   refund distinction — no panic, documented distortion). `rest` is
   assignment + comparison, no arithmetic. At implement, sweep the **diff**
   by operator (`+`, `-`, `+=`, `-=`, indexing) per
   PR-claude-operator-sweep-untrusted-arithmetic-001.
9. **Battle-modal visibility: OUT (Q8).** `CombatantSnapshot` stays hp-only;
   the HUD focus bar (PlayerSnapshot.focus — already rendered by
   src/client-app.js:361 from /state and command snapshots) is the visible
   surface. **Zero protocol, JS, storage, content, or Tauri changes.**
   PlayerState already serializes focus; saves round-trip it today.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/lib.rs` | Consts `POWER_STRIKE_FOCUS_COST`/`GUARD_FOCUS_COST`; `CombatAction::focus_cost` + `focus_refusal`; settlement+refusal in `queue_combat_action`; unfired-queue refund + defeat focus-restore in `end_combat`; `rest()` handler + `Command::Rest` arm in `handle_command`; help line; `#[cfg(test)]` tests T1–T25, T28–T29 |
| 2 | `crates/oathstar-core/src/command.rs` | `Command::Rest` variant; `rest` arm in `parse_bare_verb` (+doc); parser tests T19 |
| 3 | `crates/oathstar-server/src/main.rs` | Served economy tests T26–T27 in `mod tests` (reuse `play_to_boss_victory` route pieces, `spawn_tick_loop`, drain helpers, `/load`) |

No changes: `oathstar-protocol`, `oathstar-storage`, `oathstar-content`,
`modules/`, `src/` (JS), `tests/` (JS), `src-tauri/`. Docs land at Phase 5.

### Regression Test Plan
Standing rules applied: exact values at boundary AND boundary±1; every new
enum arm × behavior cell exact-string asserted (incl. product-unreachable
Flee refusal — unit-covered); fixtures stage before≠after (focus never
already at the asserted value); `&&` conjuncts pinned independently.
Fixture: `combat_world(stray_health, stray_attack)` (lib.rs:4936).

| # | Test (crate: name sketch) | Proves Requirement |
|---|---|---|
| T1 | core: queue PS at focus 5 → accepted, snapshot focus 3, queued `power_strike`, confirmation line byte-identical | REQ-001 |
| T2 | core: queue guard fresh → focus 4, line byte-identical | REQ-001 |
| T3 | core: re-queue same PS (already) → "already" line, focus stays 3 — deduct exactly once | REQ-001 |
| T4 | core: queue flee → focus unchanged 5 (free) | REQ-001 |
| T5 | core: queued PS fires at pulse → focus still 3 after resolution (no second charge) | REQ-001 |
| T6 | core: PS at focus==2 (boundary) → accepted, focus 0 | REQ-002 |
| T7 | core: PS at focus==1 (boundary−1) → refused: exact line, accepted=false, focus 1, queue None, combat.log length unchanged | REQ-002 |
| T8 | core: guard at 1 → accepted (0); guard at 0 → refused exact line | REQ-002 |
| T9 | core unit: `focus_cost` (0/1/2 == consts) and `focus_refusal` all three exact strings | REQ-001/002 |
| T10 | core: flee at focus 0 → accepted (left conjunct false ⇒ free verbs free) | REQ-002/003 |
| T11 | core: guard queued (4) → change-tack PS → focus 3 (refund 1, charge 2), change-tack line exact | REQ-003 |
| T12 | core: PS queued (3) → change-tack guard → focus 4 (net spend 1, no double-spend) | REQ-003 |
| T13 | core: PS queued (3) → change-tack flee → focus 5 exactly (full refund, no free-cancel GAIN) | REQ-003 |
| T14 | core: focus 0, guard queued → change-tack PS refused (effective 1 < 2): exact line, guard still queued, focus 0 | REQ-003 |
| T15 | core: focus 1, guard queued → change-tack PS accepted (effective 2): focus 0, PS queued (boundary pair with T14) | REQ-003 |
| T16 | core: rest out of combat at focus 1 → accepted, focus 5, exact line "(5/5)", Narrative channel | REQ-004 |
| T17 | core: rest mid-combat (focus staged ≠ max) → refused exact line, focus unchanged | REQ-004 |
| T18 | core: rest at focus==max → already-full exact line, accepted=false (pins `>=`) | REQ-004 |
| T19 | core parser: `rest`→Rest, `REST` folds, `rest now`/`rests` → Unknown | REQ-004 |
| T20 | core: help line exact (now lists rest) — update existing help tests | REQ-004/008 |
| T21 | core: PS fired (3) → drive to victory → focus still 3 (victory keeps spent) | REQ-005 |
| T22 | core: defeat with focus spent ≠ max and an action queued → focus == max AND hp == max (restore; executes refund-then-restore order) | REQ-005 |
| T23 | core: guard fired (4) then flee → Fled with focus 4 (flee keeps; guard stays spent) | REQ-005 |
| T24 | core: enemy hp ≤ baseline strike, PS queued (3) → phase-1 kill → Victory with focus 5 (unfired refund) | REQ-003/005 |
| T28 | core: from_save crafted sweep — focus ∈ {MIN, −1, 0, MAX} (× crafted >max, max_focus 0): queues refuse/accept per rule, **flee accepted at −1/MIN**, victory-with-queue at MAX saturates (no panic), rest restores at MIN/−1, already-full at 7>5 (no clamp), 0/0 already-full — no panics anywhere | REQ-007 |
| T29 | core: spend to 3 → save_data → from_save → snapshot focus 3 (spent value round-trips; 5/5 was the only value ever pinned) | REQ-007 |
| T26 | server: played economy fight — route + stray (PS over seam: 5→3), boss confront, guard (3→2), change-tack PS (→1), pulse victory keeps 1, `rest` over seam → 5/5 on /state | REQ-006 |
| T27 | server: /load crafted focus 1 (mid-route save mutated or constructed) → confront boss → queue PS refused over seam (typed line, accepted=false, focus 1, no queued action in combat snapshot) → queue guard accepted (1→0) → pulse victory: the pool visibly limits which skill is usable | REQ-006 |
| — | gate: full suite (cargo test, node --test, clippy strict, coverage, MSI 100) — all existing combat/levels/oath/announcement/save tests byte-stable | REQ-008 |

Genuinely uncoverable: none. `CombatAction::focus_refusal(Flee)` is
product-unreachable by design (flee is free); covered by the T9 unit test
per PR-claude-enumerate-variant-string-arms-001.

### Risks / decisions
- **D1 spend-on-queue** (vs on-resolve): atomic check+spend at one seam;
  exploit analysis above (replace refund-then-charge; end refund). Reversible
  by moving the charge to `resolve_queued_action` later; tests T5/T24 pin
  today's semantics.
- **D2 refund-on-any-end** for an unfired queue; victory keeps / defeat
  restores-to-max / flee keeps. Defeat restore makes refund unobservable
  there (T22 still executes the path).
- **D3 rest**: focus-only, full restore, out-of-combat only, no regen. The
  `>=` already-full guard doubles as the crafted->max no-clamp rule.
- **D4 refusal channel**: System + no log push ⇒ REQ-002 "change no state"
  is literal (combat.log is persisted state).
- **D5 `cost > 0 &&` gate**: keeps flee free at crafted negative focus —
  no mid-combat soft-lock; T28's flee-at-−1 pins the conjunct against
  `>`→`>=` mutants.
- **D6 saturating refunds, no max-clamp**: crafted focus > max tolerated,
  never panics; rest never clamps down (refuses as already-full). Crafted
  MAX loses refund distinction by saturation — documented, panic-free.
- **D7 battle-modal focus OUT** — zero protocol/client surface (HUD bar is
  the surface). Re-openable as a thin `CombatantSnapshot` field later.
- **R1**: T27's crafted-focus delivery (mutate a /save payload vs construct
  `SaveData` in-test) — settle at implement; either stays over the HTTP seam.
- **R2**: `queue_combat_action` borrow shape (player.focus mutated while
  `state.combat` borrowed) — disjoint fields; mirror the existing
  line-then-log structure; watch clippy too_many_lines → extract a
  settlement helper if it trips (the #20 pattern).

## Phase 3 — Implement
- **PAUSED 2026-06-11 before first code write:** forge MCP tools never
  registered this session (sidecar was down at session start; started during
  Phase 2 and recalled over HTTP — see Phase 2 process note). The §18.3
  `enforce-docs-before-code.sh` hook requires a real `mcp__forge__*`
  transcript entry and correctly blocks code writes. Waiting on owner to
  `/mcp`-reconnect the forge server; no application code touched yet.
  Implement tasks deleted to satisfy the stop-hook; recreate on resume.
- **RESUMED** after owner `/mcp`-reconnected forge; real `knowledge-search`
  recall made (same nodes as the HTTP recall); hook satisfied.
- Built (to the manifest, no scope added):
  - `command.rs`: `Command::Rest` variant + `"rest"` arm in `parse_bare_verb`
    (strict arity, case-folded like its siblings); doc comments updated.
  - `lib.rs`: `POWER_STRIKE_FOCUS_COST = 2` / `GUARD_FOCUS_COST = 1` beside
    `POWER_STRIKE_DAMAGE`; `CombatAction::focus_cost` + `focus_refusal`
    const-fn tables (Flee arm documented product-unreachable);
    `queue_combat_action` settlement — `replaced` binding folds the old
    Some(_)/None arms, refund-then-charge with `cost > 0 && effective < cost`
    refusal gate (System channel, no `combat.log` push, no state change);
    `end_combat` refunds an unfired queued action's cost first (every
    outcome), Defeat arm restores `focus = max_focus` beside hp; `rest()`
    handler (in-combat refusal / `>=` already-full no-op / restore-to-max,
    Narrative on success, System on both refusals); `Command::Rest` arm in
    `handle_command`; help line gains `rest`.
- Deviations from design (+ reason): none functional. Cosmetic: the two old
  `Some(_)`/`None` queue arms merged into one `replaced` binding (the
  settlement is identical for both; avoids duplicating it); `cargo fmt`
  reflowed the end_combat refund chain.
- `cargo check -p oathstar-core` clean; `cargo fmt --all` applied;
  `cargo clippy --workspace --all-targets --all-features` clean (no
  too_many_lines trip — queue_combat_action stays well under the ceiling).
  Existing help-string tests now fail BY DESIGN until Phase 4 updates them
  (the help line changed); test updates are validate-phase work.

## Inspect (Phase 3.5)
- Lenses run: 3 independent critics (general-purpose agents) over the diff +
  full-file context — (1) correctness vs the design contract, (2) data/state
  integrity with the by-operator crafted-save sweep (focus ∈ {MIN,−1,0,1,2,5,
  MAX} × max_focus ∈ {MIN,0,5,MAX} traced through every new
  arithmetic/comparison site), (3) simplification/reuse + mutation-readiness
  (ground truth via `cargo mutants --list`).
- Findings:
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | — | — | No findings across all three lenses | clean | n/a |
- Lead self-verification of load-bearing critic claims: walk-out-of-room
  combat end funnels through `end_combat` (lib.rs ~1865) so the unfired-queue
  refund covers it ✓; help tests assert via `contains` so the help-line
  change breaks nothing ✓ (corrects the Phase 3 note "help tests fail by
  design" — they don't; all 239 existing core tests pass per the correctness
  critic's run).
- Verified clean, with evidence: single charge site; already-arm charges
  nothing; refusal early-returns before every mutation (combat.log/queued/
  focus untouched — only the carved-out `next_event_id` increments); no
  double-refund on any of the seven traced fight-end paths; normal-play
  invariant 0 ≤ focus ≤ max_focus proven; max_focus never written; no
  clock/RNG; clippy pedantic+nursery clean with no reliance on allowlisted
  lints; no existing helper duplicated.
- **Constraints handed to Phase 4 (from the mutation analysis):** (a) a
  crafted NEGATIVE-focus flee-accepted test is REQUIRED to kill the
  `cost > 0` → `>=` mutant (plan T28 covers); (b) an exact-affordability
  boundary test (focus == cost accepted) is REQUIRED to kill `<` → `<=`
  (plan T6 covers); (c) change-tack tests must assert the FOCUS ACCOUNTING,
  not just the line (the `==`→`!=` guard mutant nets zero focus — only the
  exact already-line kills it; plan T3/T11); (d) the end_combat refund,
  defeat restore, and rest assignment generate ZERO mutants (plain
  statements) — REQ-005/REQ-004 tests + the coverage floor are their only
  enforcement (plan T16/T21–T24); (e) `parse("rest now") == Unknown` has no
  direct mutant — pin the strict-arity contract anyway (plan T19).

## Phase 4 — Validate
- Tests added (23 new — every plan row T1–T29 landed; names map via F-T
  comments):
  - `oathstar-core` lib.rs (20): `focus_cost_and_refusal_tables_are_exact`,
    `queueing_a_skill_spends_its_cost_in_the_snapshot`,
    `requeue_and_change_tack_keep_the_ledger_exact`,
    `a_fired_power_strike_charges_nothing_more`,
    `an_exactly_affordable_skill_queues`,
    `an_unaffordable_skill_refuses_and_mutates_nothing`,
    `flee_queues_at_zero_focus`, `change_tack_settles_refund_then_charge`,
    `change_tack_respects_the_refund_adjusted_boundary`,
    `rest_restores_focus_out_of_combat`, `rest_refuses_mid_combat`,
    `rest_refuses_when_already_full`, `help_lists_rest`,
    `victory_keeps_the_spent_pool`,
    `a_phase_one_kill_refunds_the_unfired_queue`,
    `defeat_restores_focus_with_hp`, `fleeing_keeps_the_spent_pool`,
    `crafted_negative_focus_never_blocks_flee` (the required `>`→`>=`
    mutant killer), `crafted_extremes_survive_the_new_arithmetic`,
    `spent_focus_round_trips_through_save`; plus a `system_line` test helper.
  - `oathstar-core` command.rs (1): `rest_parses_as_bare_strict_arity_verb`.
  - `oathstar-server` (2): `beginner_slice_plays_the_focus_economy` (T26 —
    the played stray+boss economy over the seam, change-tack settlement,
    victory keeps, rest restores), `crafted_low_focus_limits_the_served_boss_fight`
    (T27 — crafted slot through the real `/load`, typed refusal over the
    seam, guard fits, spent point stays spent).
- One test-infra fix mid-phase: the first T26 run hit `Lagged(1)` — the
  16-slot test broadcast channel lags a receiver parked across the
  inter-fight walk. Fixed in the tests by subscribing fresh immediately
  before each drain (pulses only fire while the drain awaits under paused
  time, so nothing is missed). Not an engine defect.
- `cargo test --workspace`: ALL GREEN — oathstar-core 260 passed (was 239),
  oathstar-server 27 passed (was 25), all other crates green, 0 failed.
- `node --test tests/*.test.js`: 48 pass, 0 fail.
- `bin/gate.sh --fast`: **GATE GREEN [fast]** — 14/14 static gates PASS
  (rustfmt, clippy strict, cargo test, node --test, audit, deny, machete,
  gitleaks, shellcheck, no-suppressions, source-bans, lints-allowlist,
  doc-todos, tauri shell). Coverage+mutation deferred to the FULL gate at
  `/commit` per the owner-set process bounds (spec, Locked-In Decisions).
- Pre-existing exclusions: none — no pre-existing failures encountered.

## Phase 5 — Complete
- Docs updated:
  - `docs/combat-system.md` — "Focus economy v1 implemented" block (#31)
    after the #25 block; the #25 block's "any focus economy" out-of-scope
    tail trimmed to "skills content and cooldowns".
  - `docs/mechanics-and-systems.md` — Conflict "Shipped" gains the economy
    line; Failure And Recovery's "focus depletion blocks rituals until
    rest" annotated **shipped for skills**.
  - `docs/decisions.md` — **Decision 049: Focus Is A Real Economy — Spend
    On Queue, Refund On Replace, Rest To Recover** (status Locked, with
    revisit triggers incl. the second-spender re-analysis).
  - `docs/ui-design.md` — no edit needed: the utility verb list lives in
    mechanics-and-systems.md (the ticket's pointer was stale).
- Forge capture (aar/failures/rules/decisions):
  - `aar-submit` `1621584e-09a8-494c-b2a8-3c8ea231ee84` → completed,
    effectiveness 5, 12 surfacings marked used, 4 prevention rules
    materialized; server enqueued distillation/confidence-drift/pattern-
    emergence (4 novel findings).
  - `failure-record` ×2: `BF-forge-mcp-session-start-dependency-001`
    (infra — the mid-pipeline MCP stall), `BF-test-broadcast-receiver-
    parked-lag-001` (test — the 16-slot channel Lagged(1)).
  - `prevention-rule-record`: `PR-claude-forge-up-before-session-001`
    (forge up + reconnected BEFORE entering the pipeline; HTTP seam is a
    content fallback, never hook satisfaction).
  - `architecture-decision-record`:
    `AD-claude-focus-spend-on-queue-economy-001` (mirrors Decision 049 with
    the mutation-catalog consequence noted).
- Ticket closed: forge `bb63c3d7…` → **done**; local doc moved to
  `docs/planning/tickets/closed/` with `status: done`, `closed: 2026-06-11`,
  spec pointer updated to `completed/`.
- Archived: spec+notes pair → `docs/planning/pipeline/completed/`.
