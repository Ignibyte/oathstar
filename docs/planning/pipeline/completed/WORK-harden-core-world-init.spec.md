---
pipeline_id: 70715e07-66b4-482a-a593-98f830cf7edf
title: WORK-harden-core-world-init
ticket: 99619421-df57-4aec-976f-a4139eafd469
ticket_number: 2
aar_id: fa4ac433-a861-4278-afec-343044afbe6c
type: work
intake:
notes: WORK-harden-core-world-init.notes.md
status: Phase 5 — Complete PASS
---

# WORK-harden-core-world-init

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Harden core world initialization — move world-invariant validation into `oathstar-core`.
- **Scope:**
  - **In:** A validated construction boundary in `oathstar-core` that rejects a
    malformed `WorldDefinition` with a typed error instead of constructing an
    `Engine` that panics later; removal of the `expect` on the world-invariant
    path (`current_room`); `oathstar-content::load_beginner_world` delegates to
    the core validator (single source of truth); call-site updates for the new
    fallible constructor; Rust tests for every invariant.
  - **Out:** Full content-loader redesign, dynamic/community module loading,
    save/persistence changes, runtime command-path behavior changes (the
    graceful `move_direction` defenses stay as-is).
- **Systems:** engine (`oathstar-core`), content loader (`oathstar-content`),
  server bootstrap (`oathstar-server` call site).

## Acceptance Criteria (EARS)
Each criterion is one observable behavior with a verification method.

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the core engine is initialized with a world whose `start_room_id` is absent from its rooms, it shall reject construction with a typed error and not panic. | Rust test |
| REQ-002 | When the core engine is initialized with a world whose start room exists but is not passable, it shall reject construction with a typed error. | Rust test |
| REQ-003 | If any room exit targets a room id that is absent from the world, then the core engine shall reject construction with a typed error identifying the offending room and direction. | Rust test |
| REQ-004 | The core engine shall not use `expect`/`unwrap`/`panic!` on world-invariant paths reachable from malformed module data; such invalid worlds shall surface as typed errors. | Rust test + source review (gate:10 no-suppressions / grep) |
| REQ-005 | If a room is stored under a key that differs from that room's own `id`, then the core engine shall reject construction with a typed error (room-graph key/id consistency). | Rust test |
| REQ-006 | The core engine shall construct successfully when all world invariants hold (no false rejection of valid worlds). | Rust test |
| REQ-007 | When `oathstar-content::load_beginner_world` builds the shipped world, it shall validate that world through the `oathstar-core` boundary (single source of truth, no duplicate validator). | Rust test + source review |

## Locked-In Decisions
- **Validation ownership is `oathstar-core`.** The invariant boundary lives at
  `Engine` construction in core; content does not own a parallel validator.
- **An invalid `Engine` must be unconstructable.** Malformed input yields a
  typed `Result::Err`; no panic/`expect` on input-derived world paths
  (CLAUDE.md §14 / ticket REQ-004).
- **Content delegates.** `load_beginner_world` calls the core validator rather
  than keeping its own copy of the missing-start-room / dangling-exit checks.
- **Runtime defenses remain.** The graceful messages in `move_direction`
  (unfinished world-data, blocked way) stay as defense-in-depth.
- **Deferred to Phase 2 — Design (NOT locked here):** exact constructor shape
  (`try_new` vs `new -> Result` vs a `WorldDefinition::validate`), the error
  type mechanism (`thiserror` enum vs hand-rolled `std::error::Error`), and
  whether the infallible `new` is retained for already-validated worlds.

## Linked Artifacts
- Design docs: `docs/module-system.md`, `docs/technical-architecture.md`, `docs/map-system.md`
- Intake doc: none (the only intake item is unrelated — swappable rulesets)
- Ticket doc: `docs/planning/tickets/open/TICKET-2-harden-core-world-initialization.md`
- Forge ticket: `99619421-df57-4aec-976f-a4139eafd469` (#2)
- AAR: `fa4ac433-a861-4278-afec-343044afbe6c`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
