# WORK-combat-encounter-v2-realtime-pulse-loop — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #24 — Combat Encounter v2: real-time two-phase pulse
  loop. Layer the Decision 023 server-authoritative pulse loop on top of
  deterministic command-driven combat v1 (#22): a recurring combat pulse
  advances the active encounter (Phase 1 baseline exchange + Phase 2 skill
  window with clean skip), streams pulse events + updated combat snapshot over
  the existing SSE/Datastar channel, applies between-pulse commands (minimum
  `flee`) at the pulse boundary, keeps the engine wall-clock-free (injectable
  clock; tests drive pulses), and shares engine state safely between the
  background pulse task and `/command`. AUTO-APPROVE: user pre-approved all
  phase gates; autonomous through the pipeline.
- **Intake source:** none (ticket pre-existed; forge
  `59ab51c9-4286-419a-9465-52bd7eb2af52`, minted 2026-06-09).
- **Classification / tier:** Work pipeline, **one shippable slice** (engine
  two-phase pulse + protocol additions + server pulse loop + SSE push +
  minimal client handling). The largest *architectural* shift so far — the
  server moves from pure request/response to driving encounters on a clock —
  but bounded: most plumbing exists (#15 SSE feed, #22 combat state + modal,
  #23 Nearby affordances). Deferred to keep the slice small: authored skills,
  variable cadence, boss phases, alternate resolutions.
- **Base verified:** branch `codex/oathstar-ticket-23-nearby-hostile-affordances`,
  HEAD `b5224e1` (#23), 4 commits ahead of `main` — #22/#23 (this ticket's
  dependencies) live here unmerged, so #24 continues on this branch. Worktree
  clean except untracked out-of-scope files (`assets/tilesets/`,
  `bin/generate_oathstar_tileset.py` — untouched) and the new TICKET-24 doc.
  No prior active pipeline; forge up; gate tooling present.
- **Ticket-doc housekeeping:** superseded the stale pre-forge draft
  `docs/planning/tickets/open/TICKET-24-combat-two-phase-tick-loop.md`
  (command-driven variant, "Forge ticket: pending") — removed in favor of the
  forge-backed
  `TICKET-24-combat-encounter-v2-real-time-two-phase-pulse-loop.md`.
- **Forge recall (lessons/failures surfaced):**
  - AAR opened: `e79fc9fd-6818-49ea-b1bc-8a4dac6b26f8`. Plan-phase
    `knowledge-context` logged (13 surfacings: 3 ADs, 3 distilled lessons,
    5 prevention rules, 2 recent failures).
  - ADs: `2e4d969e` AD-claude-combat-v1-001 (the #22 combat architecture this
    layers on), `60cc1bba`, `9d063c49` (entity contracts #21).
  - Prevention rules (from #22's pipeline record):
    **PR-claude-validator-length-001** — extract per-concern helpers; run
    `cargo clippy --workspace --all-targets` during IMPLEMENT (the
    `too_many_lines = 100` trap on `parse`/`handle_command`).
    **PR-claude-expect-invariants-over-unreachable-arms-001** (`b5597507`) —
    prefer `.expect("invariant")` over unreachable defensive `let-else`/`?`
    arms (coverage + MSI hygiene). Plus: run `cargo llvm-cov clean
    --workspace` before trusting a low coverage number.
  - #22 mutation-killing playbook (from WORK-combat-encounter-v1.notes.md):
    assert events/snapshots **by value** (never `is_some`); author HP fixtures
    landing **exactly on 0** to kill `<= → >` boundary mutants; two-entity /
    two-hostile rooms to pin removal and target-order determinism.
  - `docs-search`: `docs/combat-system.md` "Combat Timing" is the verbatim
    spec of this ticket (two-phase cycle, 1s/2s cadence, between-pulse command
    list, combat states incl. `fled`); Decision 023 (locked) is its decision
    record; `docs/event-lifecycle.md` "Base Tick" (1s world tick, manual tick
    mode for debugging — the injectable-clock spirit).
- **Current-code anchor map (for Design):**
  - `crates/oathstar-server/src/main.rs` (540 lines, the whole server):
    `AppState { engine: Arc<Mutex<Engine>>, events: broadcast::Sender<GameEvent>, opening }`
    @ 20–29; `main` wires world → engine → `begin()` → `spawn_tick_loop` →
    axum router @ 31–67; `/command` handler (lock → `handle_command` → drop →
    broadcast events) @ 85–99; `/events` JSON SSE @ 101–140;
    `/events/datastar` feed @ 151–192; **`spawn_tick_loop` @ 194–205 — the
    existing 1s tokio-interval task: lock engine → `engine.tick()` → drop →
    broadcast.** The combat pulse follows this exact seam. `fn main` is the
    sole mutation-excluded fn; `spawn_tick_loop` has a real test (@ 228–245).
  - `crates/oathstar-core/src/lib.rs` (4795): `CombatProfile` @ 133;
    `CombatState` @ 686 (enemy id/name/hp/max/attack, round, log);
    `GameState.combat: Option<CombatState>`; `Engine::tick()` (world tick
    event); `start_combat` @ 1441; `resolve_combat_round` @ 1563 (player
    strike → victory? → enemy return → defeat?); `end_combat` @ 1621 (victory
    removes enemy / defeat revives to max_hp; clears state);
    `combat_snapshot` @ 1784; `PLAYER_STRIKE_DAMAGE = 4`.
  - `crates/oathstar-core/src/command.rs` (666): `Command` enum + `parse`;
    `parse_combat_verb` @ 246 (attack/strike/fight; optional target) — the
    pattern for `flee`/queued-skill verbs.
  - `crates/oathstar-protocol/src/lib.rs` (701): `GameSnapshot.combat:
    Option<CombatSnapshot>`; `CombatSnapshot` @ 246 + `CombatantSnapshot`
    @ 259; `CombatOutcome { Victory, Defeat }` @ 221 (gains `Fled`);
    `GameEventKind` (snake_case `type` tags) incl.
    `CombatStarted`/`CombatEnded`; `EventChannel::Combat`,
    `OutputComponent::CombatMessage`, `Tick` event kind.
  - `crates/oathstar-datastar/src/lib.rs` (546): `feed_patch`/`describe`/
    `kind_type` exhaustive matches — new event kinds force arms here
    (compile-caught).
  - JS: `src/client-app.js` (740) — `#battle-modal` + `renderBattle` +
    `combatantCard` @ 524; **the SSE handler refreshes `/state` on combat
    events** (the v1 list: `combat_started`/`combat_ended`/`log_message`…) —
    the likely live-update path for pulses (design confirms).
    `src/client/snapshot.js` (283) — pure `toBattle` view-model
    (`toCombatant` @ 237). `tests/combat-client.test.js` — the J-test home.
  - Content: `modules/beginner/world.toml` `ashen_stray`
    (`combat={health=9,attack=3}`), `rooms.toml` `ashen_road`
    `combat_enabled = true` — the authored encounter the pulse loop drives.
- **EARS requirements reviewed:** REQ-001..008 (verbatim in the spec, from the
  ticket). REQ-001/002 engine pulse cycle (Rust, injected/explicit pulses);
  REQ-003 SSE push (Rust/integration + browser smoke); REQ-004 between-pulse
  commands + `flee` (Rust); REQ-005 outcome/stop/clear (Rust); REQ-006
  deterministic clock (Rust); REQ-007 concurrency safety (Rust test/review);
  REQ-008 #22/#23/oath/feed preservation (gate). Every REQ gets ≥1 test in
  the Phase-2 plan.

### Open design questions (for Phase 2 — Planner does NOT decide these)
1. **Engine pulse API shape.** A dedicated `combat_pulse() -> Vec<GameEvent>`
   vs growing `Engine::tick()` combat awareness (tick already returns one
   world-tick event) — and where the 2s-per-pulse cadence lives: in the
   server (a second interval / every-Nth-tick) or in the engine (pulse fires
   when `tick % pulse_rate == 0`, keeping cadence deterministic + testable).
   Per-actor default pulse rate representable without building tuning.
2. **Two-phase representation.** Both phases resolved within one pulse call
   (Phase 1 then Phase 2, two event bursts) vs alternating pulses; whether
   `CombatState` gains explicit `phase`/`cycle` fields (combat-system.md lists
   states like `round_pending`/`resolving_round`) or the pulse is atomic and
   only the round counter advances. What "authored reactions" means in v2
   (presumably the v1 enemy return action).
3. **Phase 1 baseline-exchange semantics vs v1 rounds.** v1's
   `resolve_combat_round` = player strike + enemy return on command. With
   auto-pulses: does the player auto-attack each pulse (Decision 023 "engaged
   actors can auto-attack") and `attack` becomes/queues something? Does the
   enemy act every pulse regardless? Keep v1 command semantics working
   (REQ-008) while the pulse drives the baseline — the central design call.
4. **Skill window mechanism with no authored skills.** What is queueable in
   v2 — a placeholder skill verb, `strike` as the de-facto queued skill, or
   nothing (only the clean-skip path ships, with the queue plumbed)? REQ-002
   needs *a* queued-skill resolution path test — minimal honest version.
5. **`flee` semantics.** Parsed verb (`parse_combat_verb` family) — always
   succeeds in v2? Applied immediately under the lock (= at pulse boundary by
   atomicity) vs queued for next pulse. Emits `CombatEnded{Fled}` (additive
   `CombatOutcome::Fled`) + feed summary; out-of-combat `flee` refuses
   cleanly.
6. **Snapshot push on pulse (REQ-003).** The v1 client refreshes `/state` on
   combat events over SSE — is that "streams the updated snapshot" enough, or
   do pulses push snapshot data in-band (e.g. an event carrying the combat
   sub-state, or a datastar patch)? Decide the contract + the integration
   test for it.
7. **Pulse-task lifecycle (REQ-005 "stop pulsing").** Always-running combat
   interval that no-ops when `GameState.combat` is `None` (the
   `spawn_tick_loop` pattern; simplest, single-player) vs spawning/aborting a
   task per encounter. "Stop pulsing" then = the no-op path — must still be
   observable/testable (no combat events emitted when idle).
8. **Concurrency-safety verification (REQ-007).** What the Rust test proves:
   command-vs-pulse interleaving under the mutex (e.g. flee landing between
   pulses; pulse landing mid-command-burst) — plus a review note that the
   lock discipline (`lock → mutate → drop → broadcast`, never holding across
   `.await` sends) is preserved.
9. **Client/UI minimum.** Pulse events render via existing feed components;
   modal updates via the existing refresh path. Anything new needed (phase
   indicator? skill-window affordance? pulse timer)? Default: minimal — no
   new UI surface beyond what REQ-003's live update requires.

## Phase 2 — Design

### Approach / architecture (the 9 open questions, resolved)

The pulse **rides the existing world tick** — no new task, no second clock.
`Engine::tick()` (today: `pub const fn`, returns the single `Tick` event)
becomes `pub fn tick(&mut self) -> Vec<GameEvent>`: it increments the world
tick, emits `Tick`, and — when an encounter is active and due — resolves one
full combat cycle in the same call. The server's existing `spawn_tick_loop`
(1s tokio interval, `lock → tick → drop → broadcast`) just broadcasts the Vec.
Real time exists only in the server; the engine's clock IS the tick stream
(REQ-006: tests call `tick()` directly, reproducibly).

1. **Pulse API (Q1): tick-integrated.** Decision 023 says pulses are "layered
   over the base world tick" — literally: a private
   `combat_pulse_if_due(&mut events)` runs inside `tick()`. Cadence state
   lives on `CombatState`: `pulse_rate: u64` (ticks per pulse, copied from
   `DEFAULT_COMBAT_PULSE_TICKS: u64 = 2` at start — per-actor variation later
   is a one-line copy-from-profile) and `next_pulse_at: u64` (absolute tick).
   Due when `state.tick >= next_pulse_at`; after a surviving pulse,
   re-anchor `next_pulse_at = state.tick + pulse_rate`. Combat started at
   tick T → first auto-pulse at T+2 (~2s), matching Decision 023.
2. **Two-phase cycle (Q2): both phases within one pulse** (the cycle = the
   pulse, per combat-system.md "After Phase 2, the next cycle begins"). No
   persistent `phase` field — a pulse resolves atomically; `round` keeps
   counting exchanges as in v1.
3. **Phase 1 = v1's round (Q3).** The baseline exchange reuses
   `resolve_combat_round` verbatim (player auto-strike `PLAYER_STRIKE_DAMAGE`
   → victory check → enemy return → defeat check) — Decision 023's "engaged
   actors auto-attack" with zero new combat math, all #22 kill-conditions
   intact. Manual `attack` during combat keeps its v1 meaning (resolves a
   round immediately, REQ-008's tests stay green) and does **not** disturb
   `next_pulse_at` (REQ-004 "without breaking the cadence").
4. **Phase 2 = the queued-action window (Q4).** `CombatState.queued_action:
   Option<CombatAction>`; new engine enum `CombatAction { Flee }` (doc: future
   authored skills become variants). After Phase 1, if combat survives:
   `queued_action.take()` — `Some(Flee)` resolves it (ends the encounter),
   `None` skips cleanly (no events, no mutation). This makes the skill-window
   *mechanism* real and testable (REQ-002) without inventing skill content.
5. **`flee` (Q5): a queued action, resolved at the pulse boundary.** New
   `Command::Flee` (bare strict-arity verb in `parse_bare_verb`). In combat:
   queues Flee + a confirmation line (`CombatMessage` + battle log, "You watch
   for an opening to flee."); re-queue is a no-op with an exact already-
   queued line. Out of combat: clean refusal (accepted=false). At the next
   pulse's Phase 2: `end_combat(Fled)` — additive `CombatOutcome::Fled`;
   enemy **survives in place** (no removal), player HP stays as-is (no
   revive), state cleared. REQ-004's "apply at the pulse boundary" is meant
   literally — the queue IS the boundary; the lock only guarantees commands
   never land mid-pulse.
6. **Snapshot push (Q6): typed pulse marker + the v1 refresh path.** New
   additive `GameEventKind::CombatPulse { round: u32 }` (Combat channel),
   emitted at the **start** of every due pulse (round = the cycle it
   resolves). The client adds `combat_pulse` to its `/state`-refresh list
   (`client-app.js` — today `combat_started`/`combat_ended`/`room_entered`/
   oaths only, which is exactly why pulses need a trigger: combat
   `log_message`s do NOT refresh, and with no command there is no
   `CommandResponse.snapshot`). Datastar feed skips it (`feed_patch → None`,
   like `Tick`) — the exchange `CombatMessage`s already narrate, so no 2s
   feed spam (Decision 023 rationale). REQ-003 reading recorded in risks: the
   events stream over SSE; the updated snapshot reaches the client via the
   marker-triggered `/state` fetch (Decision 034's state-vs-feed carve-out).
   Encounter-ending pulses additionally emit `CombatEnded` (already in the
   refresh list).
7. **Pulse lifecycle (Q7): no new task.** The world tick always runs;
   `combat_pulse_if_due` no-ops when `state.combat` is `None`. REQ-005's
   "stop pulsing" = after end/flee, subsequent `tick()`s emit only `Tick`
   (observable + testable).
8. **Concurrency (Q8): the proven seam, verified under virtual time.**
   `AppState` stays `Arc<Mutex<Engine>>` + broadcast; pulses and `/command`
   serialize through the lock; events broadcast only after the lock drops
   (existing discipline, preserved). Server tests use
   `#[tokio::test(start_paused = true)]` so the REAL `spawn_tick_loop` (1s
   interval) auto-advances deterministically — integration tests drive
   attack → pulses → flee/victory through the actual task + handlers and
   assert the full combat-event sequence **by value** (deterministic damage
   makes the whole pulse progression exact). A concurrency test interleaves
   command bursts with the running loop (REQ-007).
9. **Client minimum (Q9).** `combat_pulse` in the refresh list; a **Flee
   button** in the battle-modal footer (mirrors the v1 Attack button →
   `runCommand("flee")`); additive `CombatSnapshot.queued_action:
   Option<String>` (camelCase `queuedAction`, skip-if-none, value `"flee"`)
   so `toBattle` exposes it and the modal shows a quiet "Looking for an
   opening to flee…" status line (mitigates the up-to-2s queued-flee delay).
   No other UI surface.

Storage note: `oathstar-storage` is save-slot-name validation only — no
GameState persistence exists, so the new `CombatState` fields are plain
(non-`#[serde(default)]`) like the rest of that ephemeral mid-encounter
struct; wrong-by-default pulse fields (rate 0) would be worse than a
deserialization error on a payload shape that is never stored.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-protocol/src/lib.rs` | Additive `GameEventKind::CombatPulse { round: u32 }`; `CombatOutcome::Fled`; `CombatSnapshot.queued_action: Option<String>` (`#[serde(default, skip_serializing_if)]`, camelCase `queuedAction`). |
| 2 | `crates/oathstar-core/src/lib.rs` | `CombatState` + `pulse_rate: u64`, `next_pulse_at: u64`, `queued_action: Option<CombatAction>`; new `pub enum CombatAction { Flee }` (+ `as_str`); `DEFAULT_COMBAT_PULSE_TICKS: u64 = 2`; `start_combat` initializes the new fields; `tick()` → `pub fn … -> Vec<GameEvent>` + `combat_pulse_if_due` (marker → Phase 1 `resolve_combat_round` → Phase 2 queued-action window → re-anchor); `flee()` (queue/already-queued/refusal); `end_combat` `Fled` arm (no removal, no revive); `handle_command` `Flee` arm + Help text + `flee`; `combat_snapshot()` `queued_action`. |
| 3 | `crates/oathstar-core/src/command.rs` | `Command::Flee` (doc-commented); `parse_bare_verb` gains `"flee"`. |
| 4 | `crates/oathstar-datastar/src/lib.rs` | `CombatPulse` arms: `describe`/`feed_patch` → `None` (alongside `Tick`); `kind_type` → `"combat_pulse"`. |
| 5 | `crates/oathstar-server/src/main.rs` | `spawn_tick_loop` broadcasts the `Vec<GameEvent>`; new `start_paused` integration tests (pulse flow, flee-between-pulses, command/tick interleaving). |
| 6 | `src/client-app.js` | `combat_pulse` in the `/state`-refresh list; `el` refs + binding for the modal Flee button (`runCommand("flee")`); render the queued-flee status line from `toBattle`. |
| 7 | `src/client/snapshot.js` | `toBattle` gains `queuedAction` (server `queuedAction` or `null`). |
| 8 | `index.html` | Battle-modal footer: Flee button + `#battle-status` line. |
| 9 | `styles.css` | Flee button + status-line styles (mirror the Attack button family). |
| 10 | `tests/combat-client.test.js` | New `toBattle.queuedAction` + status cases (J-rows below). |
| 11 | `docs/combat-system.md`, `docs/decisions.md` (Decision 042), `docs/protocol-and-output.md`, `docs/ui-design.md` | Phase 5: v2 implemented notes (pulse loop, CombatPulse/Fled/queuedAction, Flee button). |

No content changes: the default pulse rate is an engine const; `ashen_stray`
and `ashen_road` already author the encounter.

### Regression Test Plan
≥1 per EARS REQ. Rust = `#[cfg(test)]` in the owning crate (core tests reuse
the `combat_world`/`combat_engine` fixtures); server = `#[tokio::test(start_paused = true)]`;
JS = `node --test`. Assert by value throughout (#22 playbook).

| # | Test | Proves |
|---|---|---|
| T1 | active combat → `tick()` to the due tick resolves a full cycle: `CombatPulse{round}` + player-strike + enemy-return `CombatMessage`s; enemy/player HP reduced by exact amounts; round advanced — all by value | REQ-001 |
| T2 | cadence: combat at tick T → T+1 emits only `Tick` (no combat events); T+2 pulses; T+3 quiet; T+4 pulses (re-anchor exact) — kills the `>=` boundary + re-anchor arithmetic mutants | REQ-001/006 |
| T3 | queued flee resolves in Phase 2: flee → tick to pulse → Phase-1 exchange events then `CombatEnded{Fled}`; combat `None`; enemy still in room; player HP = post-exchange (no revive) | REQ-002/004/005 |
| T4 | skip path: no queued action → pulse emits marker + Phase-1 events only; combat continues; no queued-action mutation | REQ-002 |
| T5 | `flee` between pulses queues without breaking cadence: accepted=true, exact confirmation text, snapshot `queuedAction == "flee"`, `next_pulse_at` unchanged (pulse still fires exactly on schedule) | REQ-004 |
| T6 | `flee` refusal out of combat (accepted=false, exact text, no state change); double `flee` stays queued once with the exact already-queued line | REQ-004 |
| T7 | pulse victory at exactly 0 enemy HP → `CombatEnded{Victory}`, enemy removed, combat `None`, and subsequent `tick()`s emit only `Tick` (stop pulsing) | REQ-005 |
| T8 | pulse defeat at exactly 0 player HP → `CombatEnded{Defeat}`, revive to `max_hp`, combat `None`, no further combat events | REQ-005 |
| T9 | fled preserves #22 semantics: enemy survives and is re-attackable (new encounter restarts at authored HP); no revive on flee | REQ-005/008 |
| T10 | no combat → `tick()` returns exactly the `Tick` event with the incremented value (migrates the existing tick test to the `Vec` shape) | REQ-006 |
| T11 | manual `attack` during combat still resolves a round immediately AND the scheduled pulse still fires at its original tick (cadence undisturbed) | REQ-008/004 |
| T12 | `CombatPulse.round` equals the snapshot round after the pulse; channel `Combat` | REQ-001/003 |
| T13 | `start_combat` initializes `pulse_rate == 2`, `next_pulse_at == tick + 2`, `queued_action == None` by value | REQ-001/006 |
| T14 | parser: `flee` → `Command::Flee`; `FLEE` case-folded; `flee now` → `Unknown` (strict arity) | REQ-004 |
| T15 | `combat_snapshot` `queued_action`: `None` ↔ omitted; queued ↔ `Some("flee")` by value | REQ-004 |
| P1 | `CombatPulse` serializes `{"type":"combat_pulse","round":N}` and round-trips | REQ-003 |
| P2 | `CombatOutcome::Fled` serializes `"fled"`; `CombatEnded{Fled}` round-trips | REQ-005 |
| P3 | `CombatSnapshot.queuedAction` camelCase, omitted when `None`, round-trips when `Some` | REQ-003 |
| D1 | datastar: `CombatPulse` → no feed fragment (like `Tick`); `kind_type == "combat_pulse"` | REQ-003 |
| D2 | datastar: `CombatEnded{Fled}` renders danger/Combat with its text (existing arm, by value) | REQ-005 |
| S1 | server (paused time): walk `north,north`, `attack` via the real handler, then the REAL `spawn_tick_loop` drives pulses — subscriber receives the exact deterministic combat-event sequence (started → r1 → pulse r2 → pulse r3 → victory) and `/state` reflects it | REQ-003/001 |
| S2 | server (paused time): attack → `flee` via handler between pulses → next pulse ends `fled`; `CombatEnded{fled}` broadcast; `/state` shows no combat | REQ-004/003 |
| S3 | server (paused time): command burst (look/attack/flee) interleaved with the running tick loop — no panic, no lock poisoning, coherent final state, broadcast `tick` values monotone non-decreasing | REQ-007 |
| J1 | `toBattle` exposes `queuedAction` (`null` absent; `"flee"` when present) | REQ-004 |
| J2 | queued-flee status line: view-model → exact label; cleared when `queuedAction` null | REQ-004 |
| J3 | refresh-list contract: `parseEvent` passthrough keeps `type === "combat_pulse"` (default case) so the refresh predicate can match it | REQ-003 |

**Genuinely browser-only (smoke, not node):** the live modal repaint on a real
SSE pulse (EventSource + `<dialog>` modality — same v1 carve-out); the Flee
button's DOM click→`runCommand` glue (thin seam, decision logic node-tested).
**REQ-006 negative space:** "no wall-clock in the engine" is verified by
review + the absence of `std::time` in `oathstar-core` (all pulse tests drive
`tick()` synchronously); noted for the inspect lens rather than a grep test.

### Risks / decisions
- **R1 — `tick()` signature change** (`pub const fn → pub fn`, `GameEvent →
  Vec<GameEvent>`): compile-caught at both callers (server loop, core tick
  test). Broadcast order = Vec order (Tick first, then pulse events).
- **R2 — REQ-003 reading (load-bearing):** "stream the updated combat
  snapshot" is satisfied by the typed `CombatPulse` marker streaming over SSE
  + the client's marker-triggered `/state` refetch — the established Decision
  034 carve-out (state stays JSON-pull; feed stays Datastar-push). In-band
  snapshot-in-event was rejected (duplicates state over the event wire; the
  HUD needs the full snapshot anyway). Inspect should pressure-test this.
- **R3 — queued flee (load-bearing UX):** flee lands at the next pulse's
  Phase 2 (up to ~one pulse of delay) — deliberate: it makes the queue
  mechanism real (REQ-002) and honors "apply at the pulse boundary"
  (REQ-004). If Phase 1 of that pulse kills either side first, the flee is
  moot (death/victory wins — "you didn't find the opening in time"). Modal
  status line keeps it legible.
- **R4 — paused-time tests:** `start_paused` auto-advance with an
  always-running interval task is the idiomatic tokio pattern but subtle;
  if an S-test proves unstable, fallback = drive `lock → tick() → broadcast`
  directly for the integration assertions and keep the existing real-time
  `spawn_tick_loop` smoke as the task's liveness proof. Decide at implement;
  do not ship a flaky test.
- **R5 — equivalent-mutant risk in the Phase-2 consume** (`take()` vs peek is
  near-unobservable while `Flee` is the only — always-ending — action):
  T4/T5 assert queued-state by value pre/post pulse to pin what is pinnable;
  residual equivalence accepted and re-examined when a non-ending action
  lands.
- **R6 — carried-forward v1 simplifications (documented, not bugs):**
  movement during combat still doesn't end it — pulses keep resolving while
  the player walks (flee is now the sanctioned exit; aggro/leash out of
  scope); enemy HP still resets between encounters (a fled enemy re-fights
  from authored health); world-tick `Debug` events continue at 1/s
  (pre-existing; datastar feed skips them).
- **R7 — `CombatPulse` emitted before resolution** (round = the cycle being
  resolved): uniform across ending and continuing pulses; `CombatEnded`
  remains the end marker. The marker is feed-invisible by design.

## Phase 3 — Implement
- **Built (manifest rows 1–9; tests row 10 + S-tests → Validate, docs row 11 → Complete):**
  - **protocol** (`oathstar-protocol/src/lib.rs`): `CombatOutcome::Fled`;
    `CombatSnapshot.queued_action: Option<String>` (additive, camelCase
    `queuedAction`, skip-if-none); `GameEventKind::CombatPulse { round: u32 }`
    (doc: the typed cycle marker, no feed text, the JSON client's refresh
    trigger). Two test literals gained `queued_action: None` (compile fix).
  - **core types** (`oathstar-core/src/lib.rs`): `CombatState` + `pulse_rate`,
    `next_pulse_at`, `queued_action` (plain fields — no serde defaults;
    doc-commented why); `CombatAction { Flee }` + `as_str()`;
    `DEFAULT_COMBAT_PULSE_TICKS: u64 = 2`.
  - **core logic**: `tick()` → `pub fn … -> Vec<GameEvent>` (was
    `pub const fn … -> GameEvent`) emitting `Tick` + due-pulse events;
    `combat_pulse_if_due` (let-else guards → `CombatPulse{round}` marker →
    Phase 1 `resolve_combat_round` → Phase 2 `resolve_queued_action` →
    re-anchor `next_pulse_at = tick + pulse_rate` if surviving);
    `resolve_queued_action` (`queued_action.take() == Some(Flee)` →
    `end_combat(Fled)`); `flee()` (queue + confirmation / already-queued /
    out-of-combat refusal; lines also pushed to the battle log);
    `end_combat` `Fled` arm (enemy survives, no revive); `start_combat`
    initializes the pulse fields; `handle_command` `Flee` arm; Help text
    gains `flee`; `combat_snapshot()` maps `queued_action` via `as_str`.
  - **parser** (`command.rs`): `Command::Flee` + `"flee"` in
    `parse_bare_verb` (strict arity).
  - **datastar**: `CombatPulse` joins `Tick` in the no-feed arm (comment:
    Decision 023 no-spam rationale); `kind_type` → `"combat_pulse"`.
  - **server** (`main.rs`): `spawn_tick_loop` broadcasts the tick Vec in
    order after the lock drops (the /command pattern).
  - **JS/UI**: `client-app.js` — `combat_pulse` added to the `/state`-refresh
    list; `el.battleStatus`/`el.battleFleeButton` refs; Flee button →
    `runCommand("flee")`; `renderBattle` sets the queued-flee status line.
    `snapshot.js` — `toBattle` gains `queuedAction` (null-safe, both shells).
    `index.html` — battle-modal footer: `#battle-status` + Flee button.
    `styles.css` — `.battle-status` (quiet italic, `margin-right: auto`) +
    `.battle-flee-button` (outline sibling of Attack); actions row gains
    `gap`/`align-items`.
- **Compile/check (this phase):** `cargo fmt --all` clean;
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  **GREEN** (two nursery findings source-fixed: `too_long_first_doc_paragraph`
  on `CombatAction` → split paragraph; `equatable_if_let` in
  `resolve_queued_action` → `==` comparison); `node --check` both JS files;
  `npm run build` OK. Regression sanity (existing tests only): `cargo test
  --workspace` all green (162 core et al.), `node --test` 43/43 — no
  regression from the `tick()` signature change.
- **Deviations from design (+ reason):**
  1. The two pre-existing core tick tests (`tick_increments_and_reports_value`,
     `event_ids_increment_sequentially`) were updated to the `Vec` return
     shape (compile-caught, as design R1 predicted; the first now also pins
     "an idle tick emits only the Tick event" — REQ-005's stop-pulsing
     negative space).
  2. `resolve_queued_action` uses `take() == Some(Flee)` instead of the
     designed `if let` — clippy `equatable_if_let` (nursery). Same behavior.
  3. None otherwise — the manifest was implemented as written; no content
     changes, no new server functions (the pulse rides `tick()` exactly as
     designed).

## Inspect (Phase 3.5)
- **Lenses run** (4 parallel `general-purpose` critics over `git diff HEAD` @
  base `b5224e1`): (1) correctness / pulse state machine; (2) determinism +
  100% MSI hygiene (ran the actual `cargo mutants --list` and produced a
  25-entry kill-list); (3) concurrency + serde/wire integrity + REQ-008
  preservation + the R2 reading; (4) JS state/view separation +
  simplification/reuse. Correctness came back CLEAN (every cadence/boundary/
  phase-order/panic suspicion disproven by scratch tests); the other lenses
  found 2 real bugs + 1 real consistency issue, all fixed.
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | I1 | MEDIUM→HIGH | Combat pulses follow the player across rooms: walk out mid-fight and the encounter keeps striking remotely every pulse until death — v1's documented "movement doesn't end combat" simplification (design R6) turns harmful under the clock-driven model; reachable via raw `POST /command` (core `move_direction`, lib.rs ~1322) | **REAL** (critic scratch-verified: south out of ashen_road mid-fight → pulse damage in north_gate) | `move_direction`: a successful move with an active encounter ends combat as `Fled` *before* the room events (same event/semantics as queued flee — enemy survives, HP kept, pulses stop). Forge: `BF-combat-pulse-follows-player-001` |
  | I2 | MEDIUM→HIGH | `time::interval` default `MissedTickBehavior::Burst`: laptop sleep / process stall mid-combat fires all missed 1s ticks back-to-back on resume → the encounter fast-forwards to resolution in one instant (+ burst pressure on the 256-slot broadcast channel) (server main.rs:196) | **REAL** (tokio 1.52 documented default; pre-#24 a burst was harmless Ticks) | `interval.set_missed_tick_behavior(MissedTickBehavior::Skip)` with a why-comment. Forge: `BF-tokio-interval-burst-fastforward-001` |
  | I3 | MEDIUM | Queued-flee label decision ("Looking for an opening to flee…") lived in the DOM glue (`client-app.js`) instead of the view-model — violates the Decision-032 split; `combatStatusLabel` (snapshot.js) is the house precedent | **REAL** (consistency) | `toBattle` gains `QUEUED_ACTION_LABELS` + `queuedActionLabel` (node-testable); glue reduces to `textContent = battle.queuedActionLabel ?? ""` (raw `queuedAction` kept for tests/UI logic) |
  | I4 | HIGH | Zero-anchor equivalence trap: a fixture starting combat at tick 0 cannot kill the re-anchor `+`→`*` mutant (`2+2 == 2*2`) — it survives until a third pulse | **REAL, test-design** (no code change) | Phase-4 fixtures anchor off zero: `tick()` once → `attack` (anchor=3; `3+2=5 ≠ 3*2=6`); carried into the kill-list |
  | I5 | MEDIUM | `resolve_queued_action`'s `let-else` None arm is reachable ONLY via a pulse whose Phase 1 ends combat — without that test it's an uncovered line (lib.rs ~886) | **REAL, deferred to Validate** | Kill-list F6: pulse-driven Phase-1 victory with a queued flee → asserts Victory (not Fled) precedence AND covers the arm |
  | I6 | LOW | datastar `kind_type` `combat_pulse` arm is unreachable through the render path (describe → None short-circuits), an uncovered line vs RUST_COV_MIN=94 — same precedent as the existing `Tick` arm | **REAL, deferred to Validate** | Direct in-module `kind_type(&CombatPulse{..}) == "combat_pulse"` unit test (D1 row) |
  | R1 | — | "Queued flee preempted by same-pulse Phase-1 victory/defeat is a bug" | **REJECTED** (intended: phases are ordered; death/victory wins — "you didn't find the opening in time"; doc-commented) | none — F6 pins it |
  | R2 | — | REQ-003's "stream the updated combat snapshot over SSE" not literally implemented (snapshot travels via marker-triggered `/state` refetch) | **REJECTED as a bug; CONFIRMED as the design reading** (critic found no doc contradiction: Decision 023 says "combat events stream to clients" — they do; 031/034 split state vs feed) | Phase-4 S-tests pin the end-to-end behavior; annotate at Complete |
  | R3 | LOW | Cross-task broadcast reordering (SSE lines can interleave out of event_id order between the tick task and /command) | **ACKNOWLEDGED, pre-existing** (same pattern at b5224e1; /state refetches are server-ordered so the modal is unaffected) | none — documented here |
  | R4 | LOW | A ~75s-stalled SSE subscriber can lag past `combat_ended` → stale-open modal | **ACKNOWLEDGED** (self-healing: any modal action re-renders from the response snapshot; EventSource reconnect reseeds → refresh) | none — recovery paths documented |
  | R5 | LOW/INFO | `refreshState` double-fetch on an ending pulse; OR-chain style; `parseEvent` drops `round`; stale (hidden) status span; aria-live | **REJECTED / not-now** (bounded ≤2s staleness, idempotent; style churn; predicate needs only `type`; span invisible when closed + overwritten on next render; a11y noted for a future pass) | none |
- **Mutation/coverage carry-forward (the Phase-4 backbone):** critic 2's
  25-entry kill-list (mutant → killing assertion) recorded verbatim in the
  Phase-4 plan below; headline rules: off-zero anchors (I4), `CombatPulse`
  round asserted **by value**, flee texts and `queuedAction == Some("flee")`
  by value, `t2-no-pulse`/`t3-pulse`/`t5-pulse` cadence pins for the `<`/`+`
  operator mutants, F6 victory-precedence, protocol byte-shape pins
  (`"fled"`, `queuedAction` omission + camelCase), datastar no-feed pin,
  parser `flee`/`FLEE`/`flee now` arity trio. Plus new I1 tests:
  move-during-combat → `Fled` + enemy survives + pulses stop; refused move
  (blocked exit) keeps combat + cadence intact.
- **Verification of fixes:** `cargo fmt` clean; `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` GREEN; `cargo test --workspace`
  all 11 suites green (the disengage-on-move change broke nothing — no
  existing test walks mid-combat); `node --test` 43/43.
- **Forge capture:** `failure-record` ×2 into AAR `e79fc9fd`:
  `BF-combat-pulse-follows-player-001` (ab1fba07) — re-audit documented
  simplifications when the state-transition driver changes;
  `BF-tokio-interval-burst-fastforward-001` (46d06e06) — explicit
  missed-tick policy for state-advancing intervals.

## Phase 4 — Validate
- **Tests added (24 new: 13 core + 1 parser + 3 protocol + 2 datastar + 3 server + 2 JS; plus the two pre-existing core tick tests migrated to the `Vec` shape at implement):**
  - `oathstar-core/src/lib.rs` (13 new `#[test]`s, all off-zero-anchor aware):
    `start_combat_initializes_pulse_fields` (T13 — pulse_rate/next_pulse_at/
    queued_action by value);
    `pulses_fire_on_schedule_and_manual_attack_keeps_cadence` (the cadence
    keystone: quiet t2 → pulse t3 by value → manual round → quiet t4 → pulse
    t5; kills the `<`/`+` boundary+arithmetic mutants incl. the inspect-I4
    zero-anchor trap);
    `flee_queues_between_pulses_without_moving_the_schedule` (T5/T15);
    `flee_refuses_cleanly_outside_combat` (T6);
    `flee_requeue_is_a_noop_with_its_own_line`;
    `queued_flee_resolves_at_the_pulse_boundary` (T3 — exact 5-event burst,
    Fled text by value, no revive, enemy survives, pulses stop);
    `pulse_skill_window_skips_cleanly_when_nothing_queued` (T4);
    `pulse_victory_at_exact_zero_stops_pulsing_and_removes_enemy` (T7 +
    bystander survives);
    `pulse_defeat_at_exact_zero_revives_and_stops_pulsing` (T8);
    `fled_enemy_reengages_at_authored_health` (T9 — authored reset + player
    HP carry);
    `pulse_victory_preempts_a_queued_flee` (F6/I5 — covers the
    resolve_queued_action None arm);
    `moving_out_of_the_encounter_room_disengages_as_fled` (inspect I1 —
    break-away leads the move events, HP kept, enemy survives, pulses stop);
    `refused_move_keeps_the_encounter_and_cadence` (I1b).
  - `oathstar-core/src/command.rs` (1): `flee_parses_as_bare_strict_arity_verb`
    (T14 — flee/FLEE/`flee now`).
  - `oathstar-protocol` (3): `combat_pulse_serializes_with_snake_case_tag_and_round`
    (P1), `fled_outcome_serializes_snake_case_and_round_trips` (P2),
    `combat_snapshot_queued_action_is_additive_camel_case` (P3 — omission +
    camelCase + round-trip).
  - `oathstar-datastar` (2): `combat_pulses_have_no_feed_fragment` (D1 + the
    direct `kind_type` pin for the I6 coverage nibble),
    `fled_combat_ended_renders_on_the_combat_channel` (D2).
  - `oathstar-server` (3, all `#[tokio::test(start_paused = true)]` — the REAL
    1s `spawn_tick_loop` under tokio's paused virtual clock; new dev-dep
    `tokio = { workspace = true, features = ["test-util"] }`):
    `pulses_stream_combat_to_subscribers_until_victory` (S1 — the exact
    9-event deterministic sequence over broadcast + `/state` hp 14),
    `flee_between_pulses_ends_the_encounter_fled` (S2 — exact 8-event fled
    sequence; first run caught my own count miss: the flee confirmation line
    is also a Combat event — fixed the assertion, not the code),
    `commands_and_tick_loop_interleave_safely` (S3/REQ-007 — tokio::join!
    burst + running loop, coherent final state).
  - `tests/combat-client.test.js` (2): queuedAction/queuedActionLabel
    (null-safe / flee-mapped / unknown-action-uninvented), parseEvent
    combat_pulse passthrough (J1/J3).
- `cargo test --workspace`: **GREEN** — core 176, protocol 19, datastar 14,
  server 16, content 20, storage 20; 0 failed. (Paused-time server tests run
  in ~10ms each — no real waiting.)
- `node --test tests/*.test.js`: **45 pass / 0 fail** (was 43).
- `bin/gate.sh` (FULL): **GATE GREEN [full] — 17/17 PASS.** gate:15 rust
  coverage ≥ 94% PASS; gate:16 JS coverage **86.21%** ≥ 75%; gate:17 mutation
  **250 caught / 0 missed → MSI 100.0%** (floor 100%). The FULL green wrote
  `.git/oathstar-gate-receipt`.
  - Two reds fixed at source on the way: (1) gate:1 rustfmt — the new test
    blocks needed `cargo fmt --all` (the first FULL run was killed and
    restarted since files changed mid-run); (2) gate:2 clippy pedantic
    `duration_suboptimal_units` in the S-test drain helper —
    `Duration::from_secs(300)` → `Duration::from_mins(5)`. No suppressions,
    no floor changes.
- Pre-existing exclusions: none encountered.

## Phase 5 — Complete
- **Docs updated:** `decisions.md` Decision 042 (the combat pulse rides the
  world tick — cadence on CombatState, two-phase cycle per pulse, queued-flee
  boundary semantics, move-out disengage, wall-clock-free engine,
  MissedTickBehavior::Skip, Decision-034 refresh carve-out);
  `combat-system.md` v2-implemented blockquote (replacing the stale "Decision
  023, deferred" note) + "Combat Timing" header reworded to implemented-with-
  deferrals; `protocol-and-output.md` "Implemented (ticket #24)" note
  (CombatPulse marker semantics, `fled`, `queuedAction`); `ui-design.md`
  Implementation Status #24 bullet (live modal via combat_pulse refresh, Flee
  button, queuedActionLabel status line).
- **Forge capture:** AAR `e79fc9fd` closed (`completed`, effectiveness 5, 25
  verdicts, 6 novel findings; distillation/drift/pattern jobs enqueued).
  `failure-record` ×2 (filed at inspect): BF-combat-pulse-follows-player-001,
  BF-tokio-interval-burst-fastforward-001. `prevention-rule-record` ×3:
  **PR-claude-driver-change-simplification-audit-001** (re-audit documented
  simplifications when the state-transition driver changes),
  **PR-claude-tokio-interval-missed-tick-policy-001** (explicit
  MissedTickBehavior for state-advancing intervals),
  **PR-claude-offzero-anchor-fixtures-001** (off-zero anchors for `base +
  step` mutation fixtures). `architecture-decision-record`
  **AD-claude-combat-pulse-rides-tick-001** (`f20f3ff4`).
- **Ticket closed:** forge `59ab51c9` → `done` (closing comment
  `06b5ea0e` posted); local doc moved `tickets/open/ → tickets/closed/`,
  frontmatter `status: closed` + `pipeline_spec` repointed to `completed/`.
- **Archived:** `WORK-combat-encounter-v2-realtime-pulse-loop.{spec,notes}.md`
  moved `pipeline/active/ → pipeline/completed/`; spec
  `status: Phase 5 — Complete PASS`.
