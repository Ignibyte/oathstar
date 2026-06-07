---
pipeline_id: ac0d30da-32f2-4872-8845-8468af76ed8d
title: WORK-beginner-vertical-slice
ticket: c1937d4e-2367-4884-a6e5-bcc7023f6a57
type: work
intake:
notes: WORK-beginner-vertical-slice.notes.md
status: Phase 5 — Complete PASS
---

# WORK-beginner-vertical-slice

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Beginner module vertical slice ("The Bell At Hollowmere") in the
  Rust authority path — the playable loop **town → swear oath → travel the
  authored route → resolve the tower boss**, proven end-to-end through the
  existing `/command`→`/events` server path. The geography already exists
  (ticket #6); this slice adds the **behavioral layer**: an oath lifecycle
  (placeholder), boss-endpoint resolution (placeholder), and the typed events
  that make the loop observable.
- **Scope:**
  - **In:**
    - **Oath lifecycle (placeholder)** in `oathstar-core`: oath **state** on the
      game state (e.g. not-sworn → sworn/active → fulfilled/broken), a
      `swear`/accept command in the parser, and a typed **oath event** emitted on
      the `Oath` channel (the `Oath` channel + `OathCard` component already exist
      in `oathstar-protocol`) (REQ-002).
    - **Authored route progression** (REQ-003): with the oath active, the player
      can traverse the existing authored route from `hollowmere_square` to the
      tower/boss room via the existing movement commands.
    - **Boss endpoint resolution (placeholder)** (REQ-004): a resolve/confront
      command at `bell_eater_roost` produces the authored outcome as **typed
      events** and updates oath state to fulfilled. Deterministic outcome — no
      RNG combat.
    - **Start-room room event** (REQ-001): a new beginner game places the player
      in `hollowmere_square` and produces a typed room-description event for it.
    - **Server smoke** (REQ-005): an integration test driving the whole slice
      (start → swear → route → boss) through the `oathstar-server`
      `/command`→`/events` handlers.
    - **TOML-first content** (REQ-007): the beginner oath (and any boss/route
      content the slice needs) is authored in `modules/beginner/*.toml`, loaded +
      validated by `load_beginner_world`, with dangling references rejected by a
      typed error naming the offender (reuse the `WorldValidationError` pattern).
    - Tests for every new state transition, command arm, and event variant
      (≥94% line cov, 100% mutation MSI over the new code — REQ-006).
  - **Out:** full combat system (HP/damage/pulse-timed rounds), advanced class
    transformations, shops/trainers economy, NPC memory/dialogue, skill %
    progression, region-standing mechanics, save/load of oath state, multiplayer,
    LLM/DM scripting, and **any Tauri/JS front-end** (the UI that renders this
    slice is a follow-up ticket). The boss is a **scripted placeholder
    resolution**, not a combat encounter. `relightOathstar` (JS prototype) is the
    behavioral reference to port, not a spec for full fidelity.
- **Systems:** parser + engine (`oathstar-core`: oath state, `swear`/resolve
  command handling, deterministic boss resolution), protocol
  (`oathstar-protocol`: oath/boss event surface), content (`modules/beginner/`
  TOML + `oathstar-content` loader/validation), server smoke
  (`oathstar-server`).

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a new beginner game starts, the engine shall place the player in the town start room (`hollowmere_square`) and produce a typed room-description event for that room. | Rust test (start room id + typed room event) |
| REQ-002 | When the player swears the beginner oath, the engine shall record the oath as active and emit a typed oath event on the `Oath` channel. | Rust test (oath state = active + typed oath event) |
| REQ-003 | While the beginner oath is active, the engine shall allow the player to traverse the authored route from `hollowmere_square` to the tower boss room (`bell_eater_roost`) via movement commands. | Rust test (drive the route to the boss room) |
| REQ-004 | When the player resolves the boss at the endpoint (`bell_eater_roost`), the engine shall resolve the authored outcome through typed events and set the oath state to fulfilled. | Rust test (resolve → typed outcome event(s) + oath fulfilled) |
| REQ-005 | The server shall expose the full beginner slice (start → swear → route → boss resolution) through the same `/command` and `/events` path used by normal play. | Rust integration/smoke test over the server handlers |
| REQ-006 | The full gate shall pass green, including ≥94% line coverage and 100% mutation MSI over the new oath/boss/event code. | `bin/gate.sh` → GREEN [full] |
| REQ-007 | The beginner oath/route/boss content shall be authored in `modules/beginner/*.toml` and loaded + validated by `load_beginner_world`; a dangling content reference shall be rejected with a typed error naming the offender. | Rust test (content loads; dangling ref rejected) |

## Locked-In Decisions
- **TOML-first content** (user-confirmed; `docs/vertical-slice.md` guardrail).
  The slice extends `modules/beginner/*.toml`; content loads + validates through
  `load_beginner_world` / `WorldDefinition::validate`, reusing the typed-error
  pattern that names the offender (ticket #2/#6 style) — no parallel content path,
  no `unwrap`/`expect` on file input (CONSTITUTION §14).
- **Backend-only.** No Tauri/JS UI in this ticket; the slice is proven by Rust
  tests + a server smoke test through the existing `/command`→`/events` path. The
  rendering client is a follow-up ticket.
- **Oath + boss are placeholders, but real.** Minimal-but-observable state +
  typed events, not full oath/combat systems (full combat is explicitly out).
  Each new event variant / state field / command arm is mutation surface that
  needs a killing test — keep every shape the smallest that satisfies its EARS
  (`MUT_MSI_MIN=100`).
- **Deterministic engine.** Boss resolution is a scripted, deterministic outcome
  (no RNG); if any future randomness is introduced it must use an injected RNG
  (engine determinism convention).
- **Reuse the existing typed-event path.** Extend `GameEvent` / `GameEventKind` /
  `EventChannel::Oath` / `OutputComponent::OathCard` minimally rather than adding
  a parallel event mechanism. The exact representation of oath/boss events (new
  `GameEventKind` variants vs. `LogMessage` on the `Oath`/`Combat` channel with a
  card component) is a **Phase 2 design decision**.
- **Geography is done (ticket #6).** This slice adds behavior, not map geometry;
  the boss is currently only narrative text in `bell_eater_roost` — whether to
  model it as a placed `Entity` is a Phase 2 decision.
- **One cohesive pipeline.** Engine + protocol + content + server smoke all share
  the slice; splitting would re-touch the same files.

## Linked Artifacts
- Design docs: `docs/vertical-slice.md` (the module shape/flow/guardrails),
  `docs/event-lifecycle.md` (command lifecycle), `docs/protocol-and-output.md`
  (typed domain events + channels/components), `docs/technical-architecture.md`
  (server-authoritative), `docs/game-overview.md`, `docs/decisions.md`
  (002 forgiving symbolic parser)
- Reference impl (to port): `src/engine.js` — `relightOathstar` and the oath loop
- Existing code: `crates/oathstar-core/src/lib.rs` (`Engine`/`GameState`/
  `handle_command`), `crates/oathstar-core/src/command.rs` (parser),
  `crates/oathstar-protocol/src/lib.rs` (`GameEventKind`/`EventChannel`/
  `OutputComponent`), `crates/oathstar-content/src/lib.rs` (loader),
  `crates/oathstar-server/src/main.rs` (`/command`/`/events`),
  `modules/beginner/{module,rooms,world}.toml`
- Intake doc: none (ticket pre-existed)
- Ticket doc: `docs/planning/tickets/closed/TICKET-7-build-beginner-module-vertical-slice-in-rust.md`
- Forge ticket: `c1937d4e-2367-4884-a6e5-bcc7023f6a57` (#7)
- AAR: `1a76f475-491d-4977-892f-1821c1187c61`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
