# WORK-direct-battle-verbs-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #25 — Direct Battle Verbs v1. Give #24's Phase-2 skill
  window its first real battle verbs as direct commands (`guard`,
  `power strike`; `focus strike` only if tiny): queue between pulses, resolve
  deterministically in Phase 2, refuse cleanly outside combat, handle a
  second queue deterministically, expose modal buttons + queued status.
  AUTO-APPROVE through phases unless a gate fails or scope conflicts; STOP
  BEFORE `/commit`; validate runs `./bin/gate.sh --fast` (FULL gate/commit
  are owner-gated — ask Codex first).
- **Intake source:** none (ticket doc pre-existed with EARS; forge ticket
  minted this phase: `3d9e96bb-e19a-4153-911f-70f38403e859`, #25 — frontmatter
  updated from `pending-forge`).
- **Classification / tier:** Work pipeline, **one small shippable slice** —
  additive `CombatAction` variants + parser + queue semantics + modal
  buttons. Much smaller than #24 (no architecture shift: the pulse model and
  concurrency seam are untouched).
- **Base verified:** branch `codex/oathstar-ticket-25-direct-battle-verbs`,
  HEAD `27fb7b4` (#24 commit — this ticket's dependency), stacked on
  #22/#23/#24 (all unmerged to main). Worktree clean except the protected
  untracked strays (`assets/tilesets/`, `bin/generate_oathstar_tileset.py` —
  untouched) and this pipeline's docs. No active pipeline before this; forge
  up; gate tooling present.
- **Forge recall (lessons/failures surfaced):**
  - AAR opened: `41f34dce-18d0-4eee-a1b3-7dbe30d39864`. Plan-phase
    `knowledge-context` logged (13 surfacings: 3 ADs, 3 distilled lessons,
    5 prevention rules, 2 recent failures — the #24 knowledge is live).
  - **AD-claude-combat-pulse-rides-tick-001** (`f20f3ff4`, #24) anticipated
    this ticket verbatim: "authored skills later become CombatAction
    variants; when a non-terminal action lands, the Phase-2 take()-vs-peek
    consume becomes observable and needs a pinning test." That pin is a
    locked requirement here.
  - **PR-claude-driver-change-simplification-audit-001** (`a5d75a6d`) — the
    #24 lesson; #25 doesn't change the driver, but guard's cross-pulse effect
    must be audited against movement/flee/disengage interactions (a guard
    charge surviving a fled encounter would be the same class of bug).
  - **PR-claude-offzero-anchor-fixtures-001** (`4fa00183`) — applies if any
    new schedule arithmetic appears (none expected; cadence untouched).
  - **PR-claude-expect-invariants-over-unreachable-arms-001** (`b5597507`) +
    PR-claude-validator-length-001 — the standing structure/coverage
    disciplines (parse + resolve_queued_action will grow arms; extract
    helpers before clippy's 100-line ceiling bites).
  - Failures `ab1fba07`/`46d06e06` (the #24 inspect catches) — regression
    context for REQ-007.
- **Current-code anchor map (from the #24 implementation, same session):**
  - `crates/oathstar-core/src/lib.rs`: `CombatAction` enum @ ~728 (sole
    variant `Flee`; `as_str` @ ~741) — gains the new variants;
    `CombatState.queued_action` (+ `pulse_rate`/`next_pulse_at`) @ ~700-721;
    `resolve_queued_action` @ ~883 (the Phase-2 window — `take() ==
    Some(Flee)` single-arm today, becomes a match over actions);
    `resolve_combat_round` @ ~1697 (Phase 1 — enemy return strike lives here;
    guard's reduction hooks around it); `flee()` @ ~1535 (the queue-command
    pattern to mirror); `end_combat` (Victory/Defeat/Fled arms);
    `combat_snapshot()` queued_action mapping @ ~1915;
    `PLAYER_STRIKE_DAMAGE = 4` + `DEFAULT_COMBAT_PULSE_TICKS = 2` consts;
    `handle_command` dispatch.
  - `crates/oathstar-core/src/command.rs`: `Command` enum + `parse_bare_verb`
    @ ~233 (`flee` precedent — `guard` slots here); **`power strike` is
    two-token** — needs its own parse path (cf. the `pick up` two-token
    precedent @ ~202).
  - `crates/oathstar-protocol/src/lib.rs`: `CombatSnapshot.queued_action`
    (string values — new verbs ride it); `GameEventKind` (no new kinds
    expected; design confirms).
  - `crates/oathstar-datastar/src/lib.rs`: untouched if no new event kinds.
  - JS: `src/client/snapshot.js` `QUEUED_ACTION_LABELS` + `toBattle`
    (`queuedAction`/`queuedActionLabel`); `src/client-app.js` battle-modal
    footer bindings (`battleAttackButton`/`battleFleeButton` precedent);
    `index.html` battle-modal footer; `styles.css` `.battle-flee-button`
    (the quiet-sibling pattern); `tests/combat-client.test.js`.
- **EARS requirements reviewed:** REQ-001..007 (verbatim in the spec, from
  the ticket). REQ-001/002/003/004/005 engine+parser (Rust); REQ-006 modal
  (JS + browser smoke); REQ-007 preservation (gate — `--fast` this run by
  owner instruction; FULL gate is owner-gated).

### Open design questions (for Phase 2 — Planner does NOT decide these)
1. **Verb set.** `guard` + `power strike` (both tiny, orthogonal:
   defense/offense) vs one verb. `focus strike`: include only if it is a
   trivial deterministic variant against existing `PlayerState.focus`
   WITHOUT an economy (likely: skip; recommend at design).
2. **Parser shapes.** `guard` = bare strict-arity verb (the `flee` slot in
   `parse_bare_verb`). `power strike` = two tokens — parse like `pick up`
   (`power` + `strike`), strict arity; decide aliases (`powerstrike`?
   none?). Command enum: one variant per verb vs a carried
   `Command::BattleVerb(action)` — match the existing one-variant-per-verb
   idiom unless it bloats dispatch.
3. **Guard semantics (the real design call).** Phase ordering is P1
   (exchange incl. enemy return) → P2 (queued action), so a guard queued
   now resolves in THIS pulse's P2 and must protect FORWARD: it sets a
   deterministic one-shot charge on `CombatState` consumed by the NEXT
   enemy return strike. Decide: full prevent vs fixed reduction const;
   charge stacking (no — one-shot, non-stacking); what consumes it (the
   next P1 return only, or any return incl. a manual-attack round's return
   — recommend: the next enemy return from ANY source, single charge);
   interaction with end-of-combat (charge dies with the encounter state —
   the `CombatState`-resident design gets this for free, audit per
   PR-claude-driver-change-simplification-audit-001).
4. **Power-strike semantics.** Fixed `POWER_STRIKE_DAMAGE` const (> the
   baseline 4 — e.g. 6) applied to the enemy in P2; can kill → Victory from
   P2 (end_combat from the window — new path, needs the exact-zero test);
   no enemy counter inside P2 (the window is the player's).
5. **Second-queue rule (REQ-005).** Replace-with-event vs refuse-with-event
   — ONE uniform rule across all queued actions including `flee` (today
   flee re-queue refuses idempotently with its own line; a different verb
   over a queued flee must follow the chosen rule). Recommendation to
   weigh at design: replace (changing your mind between pulses is the
   better game feel; emit "You shift your stance…" + update
   `queuedAction`), with same-action re-queue keeping the #24 idempotent
   line. Document why; test both directions (verb→flee, flee→verb,
   verb→verb).
6. **Wire values + labels.** `CombatAction::as_str` values for the new
   variants (`"guard"`, `"power_strike"` — snake_case like event tags?
   `queuedAction` today is `"flee"`; pick and pin); `QUEUED_ACTION_LABELS`
   entries ("Guarding for the next blow…", "Winding up a power strike…").
7. **Events.** Queue confirmation + P2 resolution lines as
   `LogMessage{CombatMessage}` (the flee pattern — zero datastar changes)?
   Or any typed marker needed for the modal? (Likely none: `combat_pulse`
   already triggers the refresh; queue confirmations arrive via the command
   response snapshot.) Design confirms datastar untouched.
8. **Modal buttons.** Guard + Power Strike buttons in the battle footer
   (send the exact verbs); enabled whenever combat is active (no
   availability logic in v1)? Button styling: the `.battle-flee-button`
   quiet-sibling pattern vs the brass Attack. Queued status: existing
   `#battle-status` line shows the label of whatever is queued (already
   generic via `queuedActionLabel`).
9. **Mutation/coverage pins.** Single-fire (take-not-peek): guard resolves
   once, next pulse's P2 skips. Guard-charge consumption boundary (exactly
   one return blocked/reduced, by value). Second-queue rule arms. The new
   `as_str` values. `resolve_queued_action` match arms (delete-arm
   mutants). Outside-combat refusal exactness.

## Phase 2 — Design

### Approach / architecture (the 9 open questions, resolved)

Two verbs, both additive `CombatAction` variants resolved by the existing
Phase-2 window; the pulse model, protocol crate, datastar crate, and server
are **untouched**. The #24 `flee()` queue logic generalizes into one
`queue_combat_action` path all three verbs share, with per-action strings —
flee's existing strings byte-preserved so every #24 test stays green.

1. **Verb set (Q1): `guard` + `power strike`. No `focus strike`.** Two tiny,
   orthogonal verbs exercise both effect shapes the window needs — a
   state-charge (defense) and an immediate effect (offense). Focus strike
   would either duplicate power strike or pull the focus stat into an
   economy; owner guidance says 1–2 done right.
2. **Parser (Q2).** `Command::Guard` + `Command::PowerStrike` (one variant
   per verb — house idiom). `guard` joins `parse_bare_verb` (strict arity;
   `guard now` → Unknown). `power strike` parses via the two-token `pick up`
   precedent: verb `power` + sole rest token `strike` (case-insensitive,
   whitespace-collapsed) → `PowerStrike`; bare `power`, `power slam`,
   `power strike now`, and `powerstrike` are all Unknown. No aliases in v1.
3. **Guard semantics (Q3): one-shot forward charge, full prevent.** Phase
   order is P1 (exchange) → P2 (window), so a guard queued now resolves in
   THIS pulse's P2 and protects FORWARD. `CombatState.guard_charge: bool`
   (plain field, same never-persisted justification as the #24 trio): P2
   resolution sets it (+ "You raise your guard." line); the NEXT enemy
   return strike **from any source** — pulse P1 or a manual-attack round —
   consumes it in `resolve_combat_round`: damage skipped entirely, line
   "{enemy} strikes, but your guard turns the blow aside.", player HP
   untouched (so a blocked hit can never defeat). Full prevent over a
   reduction const: zero arithmetic, one boolean, trivially mutation-pinned;
   balance tuning is out of scope and a later `GUARD_BLOCK` reduction is a
   two-line change. Non-stacking by construction (the charge is consumed by
   the first return after it exists; setting it in P2 is effectively
   unconditional — no reachable double-set arm). Charge lives on
   `CombatState` → dies automatically with EVERY encounter end
   (victory/defeat/fled/move-out): the
   PR-claude-driver-change-simplification-audit-001 sweep comes free.
4. **Power strike (Q4): `POWER_STRIKE_DAMAGE: i32 = 6`** (baseline strike
   is 4). P2 resolution: enemy HP −6 (saturating, clamped 0), line
   "Your power strike slams into {enemy} for 6 ({hp}/{max}).", pushed to the
   battle log; **exactly 0 → `end_combat(Victory)` from the window** — a new
   (tested) P2-victory path; the enemy never acts inside the window so a P2
   defeat is impossible. The round counter does not advance (rounds count
   P1 exchanges).
5. **Second-queue rule (Q5): uniform REPLACE-with-event; same-action
   re-queue stays idempotent.** Queue X over empty → confirmation. Queue X
   over X → action-specific "already" line, no mutation (flee keeps its
   exact #24 string "You are already watching for an opening to flee." —
   the existing test is untouched). Queue Y over X (any pair, including
   flee↔verb) → `queued_action = Y` + line "You change tack. " + Y's
   confirmation. Rationale: changing your mind between pulses is the better
   game feel; refusal punishes exploring the new verbs; one rule for every
   action keeps the matrix learnable and testable. All three line families
   also push to the battle log (the flee pattern).
6. **Wire values + labels (Q6).** `CombatAction::as_str`: `"guard"`,
   `"power_strike"` (snake_case for multiword, matching event-tag
   convention). `QUEUED_ACTION_LABELS` gains
   guard → "Guarding against the next blow…",
   power_strike → "Winding up a power strike…". **No protocol changes** —
   `queuedAction` is already a string field.
7. **Events (Q7): none new.** Queue/already/change lines are
   `LogMessage{CombatMessage}` at command time (they reach the client in the
   command response + the Datastar feed via the existing path); resolution
   and block lines ride the pulse burst, whose `combat_pulse` marker already
   triggers the modal refresh. **Datastar crate untouched.**
8. **Modal (Q8).** Footer gains Power Strike + Guard buttons between Attack
   (brass primary) and Flee — quiet-sibling styling via a shared group
   selector (`.battle-flee-button, .battle-verb-button { … }`; new buttons
   use `battle-verb-button`). Buttons send the exact direct verbs
   (`runCommand("power strike")`, `runCommand("guard")`). All verbs are
   available whenever combat is active (no availability logic in v1 — no
   cooldowns in scope). Queued status: the existing `#battle-status` line
   already renders `queuedActionLabel` generically.
9. **Per-action strings (pinned here; tests assert exact):**
   | action | refusal (no combat) | confirmation | already |
   |---|---|---|---|
   | flee | There is nothing to flee from. | You watch for an opening to flee. | You are already watching for an opening to flee. |
   | guard | There is nothing to guard against. | You ready your guard for the next blow. | You are already set to guard. |
   | power strike | There is nothing to strike at. | You wind up a power strike. | You are already winding up a power strike. |
   Replace prefix: "You change tack. " + confirmation. Resolve lines:
   guard "You raise your guard."; power strike per Q4; block line per Q3.

**Engine decomposition (clippy `too_many_lines` + reuse):** `flee()` is
replaced by `queue_combat_action(&mut self, action: CombatAction) -> (bool,
Vec<GameEvent>)` (refusal / already / replace / queue paths) + three small
`const fn`-style string helpers on `CombatAction` (or a lib.rs match helper)
for refusal/confirmation/already. `resolve_queued_action` becomes a
`match action { Flee → end_combat(Fled), Guard → set charge + line,
PowerStrike → strike + line + exact-zero victory }`. The guard-block branch
sits in `resolve_combat_round` exactly where the enemy return applies.
`handle_command` gains Guard/PowerStrike arms (the Flee arm re-targets to
`queue_combat_action`); Help text adds `guard, power strike`.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/lib.rs` | `CombatAction::{Guard, PowerStrike}` + `as_str` values; `POWER_STRIKE_DAMAGE: i32 = 6`; `CombatState.guard_charge: bool` (+ `start_combat` init `false`); `queue_combat_action` (replaces `flee()`; per-action refusal/confirmation/already helpers; replace-with-"change tack"); `resolve_queued_action` match with Guard/PowerStrike arms (P2 victory path); guard-block branch in `resolve_combat_round`'s enemy return; `handle_command` `Guard`/`PowerStrike` arms + Help text. |
| 2 | `crates/oathstar-core/src/command.rs` | `Command::{Guard, PowerStrike}` (doc-commented); `parse_bare_verb` gains `"guard"`; two-token `power strike` parse (the `pick up` pattern). |
| 3 | `src/client/snapshot.js` | `QUEUED_ACTION_LABELS` gains `guard` + `power_strike` entries. |
| 4 | `src/client-app.js` | `el.battlePowerStrikeButton`/`el.battleGuardButton` refs + click bindings → `runCommand("power strike")`/`runCommand("guard")`. |
| 5 | `index.html` | Battle-modal footer: Power Strike + Guard buttons (class `battle-verb-button`) between Attack and Flee. |
| 6 | `styles.css` | Group the quiet-button rule: `.battle-flee-button, .battle-verb-button { … }` (both blocks). |
| 7 | `tests/combat-client.test.js` | VJ rows below (labels). |
| 8 | docs (Phase 5): `docs/combat-system.md` (direct-verbs note), `docs/decisions.md` (Decision 043), `docs/ui-design.md` (modal buttons bullet), `docs/protocol-and-output.md` (queuedAction values, one line). |

**No changes**: `oathstar-protocol`, `oathstar-datastar`, `oathstar-server`,
content TOML.

### Regression Test Plan
≥1 per EARS REQ. Rust in the owning crate (reusing `combat_engine`/
`active_combat`/`enemy_of`/`player_of`); JS `node --test`. Exact strings and
values throughout (#22/#24 playbook).

| # | Test | Proves |
|---|---|---|
| V1 | in combat, `guard` queues: accepted, exact confirmation line (Combat/CombatMessage), `queuedAction == "guard"`, battle-log tail, `next_pulse_at` unchanged | REQ-001 |
| V2 | in combat, `power strike` queues: exact line, `queuedAction == "power_strike"` | REQ-001 |
| V3 | guard resolves forward: pulse 1 P1 return hits normally (HP by value) then P2 emits "You raise your guard." + queue cleared; pulse 2 P1 emits the block line, player HP unchanged, enemy still struck; charge consumed | REQ-002 |
| V4 | take-not-peek pin (AD f20f3ff4): across V3's fight exactly ONE resolve line and ONE block line; pulse 2's P2 skips silently; pulse 3 P1 return hits normally | REQ-002/003 |
| V5 | power strike resolves: pulse P1 exchange by value, then P2 slam line with exact damage/HP; queue cleared; enemy HP = post-P1 − 6 | REQ-002 |
| V6 | P2 victory at exactly 0: power strike lands the kill from the window → `CombatEnded{Victory}`, enemy removed, combat `None`, pulses stop | REQ-002/007 |
| V7 | no queue → pulse is the #24 skip exactly (marker + P1 only, no verb lines) | REQ-003 |
| V8 | outside combat: `guard` and `power strike` refuse with exact lines, accepted=false, zero mutation (no combat state, player HP untouched) | REQ-004 |
| V9 | same-action re-queue: `guard` ×2 → exact already-line, still queued once | REQ-005 |
| V10 | replace: `guard` then `power strike` → "You change tack. You wind up a power strike.", `queuedAction == "power_strike"`; the pulse resolves ONLY power strike (no charge set) | REQ-005 |
| V11 | flee interactions: `flee`→`guard` replaces (no Fled at pulse); `guard`→`flee` replaces (Fled at pulse); `flee`→`flee` keeps the exact #24 already-line (existing test untouched) | REQ-005/007 |
| V12 | preemption: queued guard + P1 kills the enemy that pulse → Victory, no resolve line, no charge | REQ-002/007 |
| V13 | manual `attack` with a queued verb: round resolves, queue survives (manual rounds don't consume the window) | REQ-001/007 |
| V14 | the charge guards manual rounds too: charge set → manual `attack` → that round's return blocked + charge consumed → next pulse return hits normally | REQ-002 |
| V15 | parser: `guard`/`GUARD` → Guard, `guard now` → Unknown; `power strike`/`POWER STRIKE`/`power   strike` → PowerStrike; `power`, `power slam`, `power strike now`, `powerstrike` → Unknown | REQ-001/004 |
| V16 | fled-state hygiene: charge set → flee out → re-attack → fresh state has `guard_charge == false`, `queuedAction` absent (no leak across encounters) | REQ-007 |
| VJ1 | `QUEUED_ACTION_LABELS`: toBattle maps `"guard"` and `"power_strike"` to their exact labels; unknown action still labels `null` | REQ-006 |
| VJ2 | toBattle queuedAction passthrough for the new values (raw kept) | REQ-006 |

**Genuinely browser-only (smoke):** the two new buttons' DOM click →
`runCommand` glue (same thin-seam carve-out as Attack/Flee; the strings they
send are pinned by V15's parser coverage).

### Risks / decisions
- **R1 — replace-rule vs #24 (load-bearing):** same-action re-queue keeps
  the #24 idempotent behavior byte-for-byte (flee's strings preserved in the
  refactor), so no existing test changes; cross-action replace is new,
  uniform, and pinned by V10/V11. `queue_combat_action` must reproduce
  flee's three strings exactly.
- **R2 — guard full-prevent balance:** dominant against weak enemies,
  worthless tuning-wise — accepted; v1 is mechanism, not balance. A later
  `GUARD_BLOCK` reduction is two lines + retests.
- **R3 — new P2-victory path** (`end_combat` from the window): pinned at
  exact zero (V6); P2 defeat is impossible (the enemy never acts in the
  window) — documented, not coded around.
- **R4 — charge cleanup:** `guard_charge` on `CombatState` dies with every
  end (incl. fled/move-out) automatically; V16 pins the re-engage hygiene
  (the driver-change-audit lesson applied).
- **R5 — `guard_charge` plain field:** same never-persisted justification as
  the #24 pulse trio; doc-commented alongside them.
- **R6 — two-token verb UX:** `powerstrike`/bare `power` are Unknown by
  design (precise grammar; aliases deferred deliberately).
- **R7 — coverage/mutation:** new match arms (delete-arm mutants) killed by
  V3/V5/V11; `POWER_STRIKE_DAMAGE` literal mutants by V5's exact values;
  `guard_charge` boolean mutants by V3/V4/V14's by-value HP asserts; no new
  schedule arithmetic (off-zero-anchor rule not triggered, cadence asserts
  reused in V1).

## Phase 3 — Implement
- **Built (manifest rows 1–6; tests row 7 → Validate, docs row 8 → Complete):**
  - **core** (`oathstar-core/src/lib.rs`): `CombatAction::{Guard, PowerStrike}`
    + `as_str` (`"guard"`/`"power_strike"`) + private `queue_refusal`/
    `queue_confirmation`/`queue_already` string helpers (the design's pinned
    table, flee strings byte-identical); `POWER_STRIKE_DAMAGE: i32 = 6`;
    `CombatState.guard_charge: bool` (plain, doc-commented with its
    never-persisted siblings) + `start_combat` init `false`;
    `flee()` → `queue_combat_action(action)` (refusal / same-action already /
    different-action "You change tack. {confirmation}" replace / queue —
    all lines also pushed to the battle log); `resolve_queued_action` is now
    a `match` (None skip / Flee→Fled / Guard→`resolve_guard` /
    PowerStrike→`resolve_power_strike`); `resolve_guard` arms the charge +
    "You raise your guard."; `resolve_power_strike` deals 6 (saturating,
    clamped), exact-zero → `end_combat(Victory)` from the window;
    the guard-block branch sits at the top of `resolve_combat_round`'s
    enemy-return section (consume charge → "…turns the blow aside." → skip
    damage → return); `handle_command` `Guard`/`PowerStrike` arms (Flee arm
    re-targeted to `queue_combat_action`); Help text gains
    `guard, power strike`.
  - **parser** (`command.rs`): `Command::{Guard, PowerStrike}` (doc-commented);
    `parse_bare_verb` gains `"guard"`; two-token `power strike` parse block
    (the `pick up` pattern — `power` + case-insensitive `strike`, strict
    arity, fused/partial forms Unknown), placed before the `pick` block.
  - **JS/UI**: `snapshot.js` `QUEUED_ACTION_LABELS` gains
    `guard`/`power_strike` labels; `client-app.js` el refs + click bindings
    (`runCommand("power strike")`, `runCommand("guard")`); `index.html`
    footer buttons (Power Strike, Guard) between Attack and Flee with a
    why-comment; `styles.css` quiet-button rule grouped
    (`.battle-flee-button, .battle-verb-button` + hover).
  - **Untouched as designed:** `oathstar-protocol`, `oathstar-datastar`,
    `oathstar-server`, content TOML.
- **Compile/check (this phase):** `cargo check --workspace --all-targets`
  clean on first run; `cargo fmt --all`; `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` **GREEN first run**;
  `node --check` both JS files; `npm run build` OK. Regression sanity
  (existing tests only): all 11 Rust suites green incl. every #24
  flee/pulse test (the `queue_combat_action` refactor is behavior-preserving
  — flee strings byte-identical); `node --test` 45/45.
- **Deviations from design (+ reason):** none — implemented exactly to the
  manifest and the pinned string table.

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel `general-purpose` critics over `git diff HEAD` @
  base `27fb7b4`, scaled to the small diff): (1) correctness / guard-charge +
  queue lifecycle — **CLEAN**: 10 scratch tests traced the guard
  arm/block/consume cycle (a blocked lethal return cannot kill; the early
  return skips damage AND the defeat check), preemption both ways (P1
  victory AND P1 defeat drop the queued verb silently, no panic), P2
  exact-zero victory + no double-kill (the let-else guard), the replace rule
  with flee↔verb in both directions, parser block ordering (bare `strike` is
  still `Attack`; the `power` block can't shadow anything; the test-world
  entity named "guard" is target-position only), byte-identical flee
  strings, and full-suite green (265 Rust + 45 JS). (2) mutation hygiene +
  reuse — 21 new mutants enumerated, **no equivalent mutants**; kill-list
  recorded into the Phase-4 plan below.
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | I1 | MEDIUM | `queue_already`'s PowerStrike arm is executed by NO planned test row (V9 was guard-only, V11 flee-only) — a line-coverage gap with no arm-deletion mutant to force it (lib.rs ~790) | **REAL, test-plan gap** | Validate: extend V9 to a power-strike×2 row asserting the exact already-line |
  | I2 | LOW | Help-text addition (`guard, power strike`) unasserted — house precedent pins new verbs in the help test (lib.rs ~1021) | **REAL, test-plan gap** | Validate: help `contains("guard")` + `contains("power strike")` row |
  | I3 | LOW | Window-overkill clamp unobservable: no planned row enters Phase 2 with enemy HP in 1..6, so a "(−2/8)" display regression would pass (no mutants exist for `saturating_sub`/`.max(0)`/the literal) (lib.rs ~972) | **REAL, test-plan gap** | Validate: overkill row asserting the slam line shows "(0/{max})" + Victory |
  | I4 | LOW | `parse_bare_verb` doc comment omitted `guard` (command.rs ~248) | **REAL, doc rot** | Fixed in place |
  | I5 | INFO | Protocol `queued_action` doc said `"flee" in v2` though it now carries the #25 values (protocol lib.rs ~257) | **REAL, doc rot** | Fixed in place (values enumerated) |
  | R1 | INFO | Armed guard charge is invisible to the client (queuedAction clears when the charge arms — the modal shows nothing while guarded; only the log narrates) | **ACKNOWLEDGED, not in ACs** | Noted for a future ticket (e.g. expose an `armed` state) |
  | R2 | INFO | Guard-every-window is a deterministic invincibility loop vs any 1v1 | **ACKNOWLEDGED** (design R2 — balance out of scope; mechanism v1) | none |
  | R3 | INFO | `resolve_power_strike` duplicates ~8 lines of the player-strike shape; five `combat.as_mut().expect` sites accumulate | **REJECTED as a change** (the decomposition is the design's explicit choice; site-specific expect messages document local invariants — house style) | revisit if a third strike variant lands |
- **Mutation/coverage carry-forward (into Phase 4):** critic 2's kill-list
  recorded — load-bearing notes: V9's **exact-string assert is the only
  killer** for the match-guard `==`→`!=`/`true`/`false` mutants (queue-state
  asserts alone cannot distinguish); the guard-path lines
  (`guard_charge` flips, the early return, `.max(0)`, the literal 6,
  resolve_queued_action's arm dispatch) generate **no mutants at all** — the
  by-value HP/log/occurrence-count asserts in V3/V4/V5/V14 are their sole
  pins, so none of those rows are redundant; V4 should count block-line
  occurrences across the whole event stream; V6's exact-zero is reachable
  with `combat_engine(14, _)` (4+4+6).
- **Verification of fixes:** `cargo fmt --all --check` clean;
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  GREEN; all 11 Rust suites green. No `failure-record` warranted — no
  behavioral bug shipped (two doc-rot nits fixed in place; the substantive
  findings are test-plan amendments, captured above for Validate).

## Phase 4 — Validate
- **Tests added (19 new: 16 core + 2 parser + 1 JS):**
  - `oathstar-core/src/lib.rs` (16, + two small test helpers `combat_line`/
    `count_lines`): `guard_queues_between_pulses_without_moving_the_schedule`
    (V1), `power_strike_queues_with_its_wire_value` (V2),
    `battle_verbs_refuse_cleanly_outside_combat` (V8 — both verbs, exact
    lines, zero mutation), `same_action_requeue_is_a_noop_with_exact_lines`
    (V9 + inspect I1 — BOTH actions' exact already-lines: the strings are the
    sole killers for the match-guard mutants),
    `different_action_replaces_with_change_tack` (V10 — replaced guard never
    arms), `replace_rule_covers_flee_in_both_directions` (V11),
    `manual_attack_leaves_the_queued_verb_alone` (V13),
    `guard_protects_forward_through_the_next_return` (V3 — full
    arm-then-block cycle by value),
    `guard_fires_exactly_once_take_not_peek` (V4 — occurrence counts across
    a 6-tick stream + HP arithmetic 17→14→14→11),
    `guard_charge_blocks_a_manual_round_return` (V14),
    `power_strike_resolves_in_the_window_by_value` (V5 — exact slam line
    "(26/40)"), `power_strike_victory_at_exact_zero_from_the_window` (V6 —
    `combat_engine(14,1)`, "(0/14)", Victory from Phase 2, pulses stop),
    `power_strike_overkill_clamps_at_zero` (inspect I3 — 4−6 → "(0/12)"),
    `pulse_victory_preempts_a_queued_verb` (V12),
    `fled_encounter_leaves_no_verb_residue` (V16 — incl. a mid-flee blocked
    return), `help_lists_the_battle_verbs` (inspect I2).
  - `oathstar-core/src/command.rs` (2): `guard_parses_as_bare_strict_arity_verb`,
    `power_strike_parses_as_exactly_two_tokens` (V15 — all negative forms +
    the bare-`strike`-is-still-Attack pin).
  - `tests/combat-client.test.js` (1): `toBattle labels the direct battle
    verbs` (VJ1/VJ2 — exact labels + raw passthrough).
  - V7 (skip unchanged) remains pinned by #24's
    `pulse_skill_window_skips_cleanly_when_nothing_queued` (still green).
- `cargo test --workspace`: **GREEN** — core **194** (was 176), protocol 19,
  datastar 14, server 16, content 20, storage 20; 0 failed. All 18 new Rust
  tests passed on the first run.
- `node --test tests/*.test.js`: **46 pass / 0 fail** (was 45).
- `bin/gate.sh --fast`: **GATE GREEN [fast] — 14/14 PASS** (gates 15–17
  coverage+mutation SKIPPED per the `--fast` owner instruction; the FULL
  gate is owner-gated and must run before any `/commit`). Two reds fixed at
  source on the way: gate:1 rustfmt (new test blocks) and gate:2 clippy
  pedantic in test helpers (`doc_markdown` backticks;
  `map().unwrap_or(false)` → `is_some_and`). No suppressions.
- Pre-existing exclusions: none encountered.

## Phase 5 — Complete
- **Docs updated:** `decisions.md` Decision 043 (battle actions are direct
  verbs — the binding owner decision, the verb semantics, the uniform replace
  rule, the no-`skill <name>` constraint, revisit triggers);
  `combat-system.md` "Direct battle verbs v1 implemented" blockquote;
  `ui-design.md` #25 bullet (verb buttons + generic queued labels);
  `protocol-and-output.md` queuedAction values line (open-string note).
- **Forge capture:** AAR `41f34dce` closed (`completed`, effectiveness 5,
  25 verdicts, 2 novel findings; distillation/drift/pattern jobs enqueued).
  No failure-records (inspect found no behavioral bug — doc rot + test-plan
  gaps only). `prevention-rule-record`
  **PR-claude-enumerate-variant-string-arms-001** (`be951a7b`) — exhaustive
  per-variant string matches generate no arm-deletion mutants, so every
  variant×behavior cell needs an exact-string test. 
  `architecture-decision-record` **AD-claude-direct-battle-verbs-001**
  (`dbc0dca4`).
- **Ticket closed:** forge `3d9e96bb` → `done` (closing comment `a67aa17e`);
  local doc moved `tickets/open/ → tickets/closed/`, frontmatter
  `status: closed` + `pipeline_spec` repointed to `completed/`.
- **Archived:** `WORK-direct-battle-verbs-v1.{spec,notes}.md` moved
  `pipeline/active/ → pipeline/completed/`; spec
  `status: Phase 5 — Complete PASS`.
- **STOPPED BEFORE `/commit`** per owner instruction: the FULL gate (15–17
  coverage+mutation) and the commit are owner-gated — ask Codex. Branch
  `codex/oathstar-ticket-25-direct-battle-verbs`, worktree carries the
  uncommitted #25 implementation on top of `27fb7b4`.
