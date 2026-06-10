---
pipeline_id: b561c30b-6e18-4097-8e74-7288472ace62
title: WORK-combat-encounter-v2-realtime-pulse-loop
ticket: 59ab51c9-4286-419a-9465-52bd7eb2af52
type: work
intake:
notes: WORK-combat-encounter-v2-realtime-pulse-loop.notes.md
status: Phase 5 — Complete PASS
---

# WORK-combat-encounter-v2-realtime-pulse-loop

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Combat Encounter v2 — the server-authoritative real-time two-phase
  pulse loop (Decision 023) layered on deterministic command-driven combat v1
  (#22): an active encounter advances on a recurring combat pulse without a
  player command (Phase 1 baseline exchange, then Phase 2 skill window with a
  clean skip when nothing is queued), each pulse streams its combat events and
  the updated combat snapshot to clients over the existing SSE/Datastar
  channel, and between-pulse manual commands (at minimum `flee`) apply at the
  pulse boundary — with an injectable/deterministic clock and
  concurrency-safe shared engine state.
- **Scope:**
  - **In:** a server-side pulse/tick loop (tokio interval in
    `oathstar-server`, following the existing `spawn_tick_loop` seam) driving
    the active encounter; the two-phase cycle in the engine (baseline exchange
    + skill window with a clean skip when none is queued); SSE push of pulse
    events + updated combat snapshot so the battle modal updates live; a
    minimal between-pulse command set (at least `flee`, ending the encounter
    with a fled outcome); an injectable/deterministic clock (engine never
    reads wall-clock; tests drive pulses explicitly); concurrency-safe shared
    engine state (background pulse task + `/command` requests); Rust + JS
    tests for every EARS REQ; docs.
  - **Out:** the full skills/classes system (only the skill-*window* mechanism
    + clean skip; authored skills come later), equipment/loot/AI-tactics/
    aggro, per-region pulse tuning beyond a per-actor default, boss scripted
    timings/phases (`paused_sequence`), alternate resolutions beyond `flee`
    (persuade/spare/bind — Decision 007, later), multiplayer turns, and
    save-game persistence of mid-encounter state.
- **Systems:** combat, engine, server, protocol, datastar, ui.

## Acceptance Criteria (EARS)
Verbatim from `TICKET-24` (forge `59ab51c9-4286-419a-9465-52bd7eb2af52`).

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a combat encounter is active, the server shall advance it on a recurring combat pulse without a player command, resolving Phase 1 (the baseline exchange — auto-attacks + authored reactions) each pulse. | Rust test (injected clock) |
| REQ-002 | When a pulse resolves Phase 1, the server shall resolve Phase 2 (the skill window): a queued player skill resolves, or the skill phase is skipped cleanly when none is queued. | Rust test |
| REQ-003 | When a pulse resolves, the server shall stream the resulting combat events and the updated combat snapshot to connected clients over the existing SSE channel, so the battle modal updates live without a command. | Rust/integration test + browser smoke |
| REQ-004 | When the player submits a manual combat command between pulses (at minimum `flee`; `use <item>`/`switch target` as design allows), the server shall apply it at the pulse boundary without breaking the cadence, and `flee` shall be able to end the encounter. | Rust test |
| REQ-005 | When either side reaches zero HP or the player flees, the server shall end combat with the correct outcome, stop pulsing that encounter, and clear combat state (preserving #22 win/loss/clear semantics). | Rust test |
| REQ-006 | The pulse clock shall be injectable/deterministic — the engine shall not depend on wall-clock time, and tests shall drive pulses explicitly and reproducibly. | Rust test |
| REQ-007 | The background pulse task and incoming `/command` requests shall share engine state safely (no races or corruption) under concurrent access. | Rust test / review |
| REQ-008 | Existing command-driven combat v1 (attack/strike/fight), the #22 battle modal, the #23 Nearby hostile affordances + entity detail, the oath/boss confront flow, look/talk/take/movement, and the Datastar event feed shall continue to pass. | gate |

## Locked-In Decisions
Settled before design; not re-litigated mid-pipeline. The open *design*
choices they leave are enumerated in the notes for Phase 2 to settle.

- **Layer on v1; do not rewrite it (Decision 040 + REQ-008).** Combat v1's
  state (`GameState.combat`/`CombatState`), command path
  (`attack`/`strike`/`fight`), end semantics (victory removes the enemy,
  defeat revives to `max_hp` in place, state cleared to `None`), the battle
  modal, and the confront/oath/boss flow all keep working. The pulse loop is
  additive on top of `start_combat`/`resolve_combat_round`/`end_combat` —
  extending them is allowed; replacing the v1 model is not.
- **Decision 023 cadence.** World tick 1s (already live —
  `spawn_tick_loop`, `oathstar-server/src/main.rs`); default combat pulse 2s.
  Per-actor/region/boss pulse variation is out of scope beyond keeping the
  default representable.
- **The engine stays wall-clock-free and deterministic (REQ-006, the #22
  discipline).** Real time exists only in `oathstar-server` (tokio interval).
  The engine exposes an explicit, deterministic pulse entry point that tests
  call directly and reproducibly; no RNG, no `Instant::now()`/`SystemTime` in
  `oathstar-core`.
- **Two-phase cycle per `docs/combat-system.md` "Combat Timing".** Each pulse:
  Phase 1 resolves the baseline exchange (auto-attacks + authored reactions),
  Phase 2 resolves a queued player skill or skips cleanly. Only the window
  *mechanism* ships; authored skill content is deferred.
- **Concurrency builds on the existing server seam (REQ-007).** `AppState` is
  already `Arc<Mutex<Engine>>` + `broadcast::Sender<GameEvent>` with a spawned
  interval task (`spawn_tick_loop` precedent). The combat pulse uses the same
  shared-state model; commands and pulses serialize through the engine lock so
  "apply at the pulse boundary" is the lock's atomicity. Where the pulse task
  lives (ride the world tick vs its own interval) is Phase 2's call.
- **Additive protocol only (Decisions 028/031).** New event kinds/snapshot
  fields are additive (`#[serde(default)]` / `skip_serializing_if`), wire
  conventions preserved: snake_case event `type` tags, camelCase snapshot
  fields. The existing `/events` + `/events/datastar` SSE channels carry pulse
  output — no new transport.
- **Between-pulse commands: `flee` is the v2 contract (REQ-004/005).** `flee`
  ends the encounter with a fled outcome and clears combat state; the fled
  outcome is an additive variant alongside #22's win/loss. Richer between-
  pulse commands (`use <item>`, `switch target`) only if design finds them
  cheap; persuade/spare/bind stay deferred (Decision 007).
- **Quality discipline (gate §0; #22/#23 prevention rules).** 100% MSI with
  only `fn main` excluded ⇒ every new server fn outside `main` is testable
  (the `spawn_tick_loop` test is the precedent). Extract per-concern helpers
  to stay under clippy `too_many_lines = 100` and run
  `cargo clippy --workspace --all-targets` during IMPLEMENT
  (PR-claude-validator-length-001). Prefer `.expect("invariant")` over
  unreachable defensive arms (PR-claude-expect-invariants-over-unreachable-
  arms-001). Assert by value, never `is_some`; pin boundary fixtures exactly
  (the #22 mutation-killing playbook). No suppressions, no baselines.
- **One shippable slice.** Engine pulse cycle + server loop + SSE push +
  minimal client handling ship together (the pulse without the push is not
  observable; the push without the pulse has nothing to stream). DESIGN
  sequences internally: engine two-phase pulse → protocol additions → server
  loop + concurrency → client.

## Linked Artifacts
- Design docs: `docs/combat-system.md` (Combat Timing), `docs/decisions.md`
  (023, 040, 041, 028/031/034), `docs/event-lifecycle.md` (base tick),
  `docs/protocol-and-output.md`, `docs/ui-design.md`
- Intake doc: none
- Ticket doc: `docs/planning/tickets/closed/TICKET-24-combat-encounter-v2-real-time-two-phase-pulse-loop.md`
- Forge ticket: `59ab51c9-4286-419a-9465-52bd7eb2af52` (#24)
- AAR: `e79fc9fd-6818-49ea-b1bc-8a4dac6b26f8`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
