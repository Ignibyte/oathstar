---
title: TICKET-24-combat-encounter-v2-real-time-two-phase-pulse-loop
status: closed
ticket: 59ab51c9-4286-419a-9465-52bd7eb2af52
ticket_number: 24
type: feature
created: 2026-06-09
closed: 2026-06-09
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-combat-encounter-v2-realtime-pulse-loop.spec.md
---

# TICKET-24-combat-encounter-v2-real-time-two-phase-pulse-loop

## Summary

Make combat real-time: layer the server-authoritative pulse loop (Decision 023)
on top of the deterministic command-driven combat v1 (#22), so an active encounter
advances on its own clock and streams each pulse to the client.

## Why

Combat v1 (#22) shipped a deliberately command-driven loop (Decision 040) — no
clock, no RNG — to establish a fully testable foundation (100% mutation, every
branch pinned) before adding real-time concurrency. `docs/combat-system.md` always
specced combat as **server pulses** and explicitly reserved that loop for "ticket
#24". This is the largest architectural shift so far: the server moves from pure
request/response to **driving active encounters on a timer and pushing** each pulse
to the client over the existing SSE/Datastar `/events` stream. Much of the plumbing
is already in place — the SSE feed (#15), the battle modal + `GameState.combat`
(#22), and the Nearby affordances (#23) — so the bulk of the new work is the
server-side clock, the two-phase cycle, and the concurrency model.

## EARS Requirements

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

## Scope

- In: a server-side pulse/tick loop (e.g. a `tokio` interval) driving active
  encounters; the two-phase cycle (baseline exchange + skill window with a clean
  skip when none is queued); SSE push of pulse events + snapshot; a minimal
  between-pulse command set (at least `flee`); an injectable clock for
  deterministic tests; concurrency-safe shared engine state; tests; docs.
- Out: the full skills/classes system (only the skill-*window* mechanism + skip;
  authored skills come later), equipment/loot/AI-tactics/aggro, per-region pulse
  tuning beyond a per-actor default, boss scripted timings/phases, alternate
  resolutions beyond `flee` (persuade/spare/bind — Decision 007, later), and
  multiplayer turns.

## Notes

- Forge ticket: `59ab51c9-4286-419a-9465-52bd7eb2af52` (#24)
- Depends on combat v1 (#22, Decision 040) + Nearby affordances (#23, Decision 041).
  Realises the deferred pulse half of `combat-system.md` "Combat Timing" (Decision
  023): world tick ~1s, default combat pulse ~2s (per-actor variation possible).
- Already in place: the SSE/Datastar push path (#15), persistent `GameState.combat`
  (#22), and the battle modal that renders `snapshot.combat`.
- The **injectable clock** (REQ-006) is the key to keeping real-time combat testable
  — mirror the #22 discipline of avoiding wall-clock/RNG in the engine so pulses are
  reproducible. The **concurrency model** (REQ-007) is the main new design problem:
  a background ticker mutating engine state while `/command` requests arrive — likely
  a shared `Arc<Mutex<Engine>>` with a single owner of the tick, or an actor/channel
  model. Design must settle where the tick lives and how the SSE stream is fed.
- Keep the first version modest per the doc: baseline two-phase pulse + `flee`;
  defer skills content, variable cadence, and boss phases.
