---
pipeline_id: 91877f69-e467-4c68-a91d-ce092a591da3
title: WORK-raise-coverage-baseline
ticket: e8a10f21-3e97-4c2a-87a7-6ea77f321502
ticket_number: 8
aar_id: 3cf6eaa5-db2b-4a64-a7cf-701d2853ab25
type: work
intake:
notes: WORK-raise-coverage-baseline.notes.md
status: Phase 5 — Complete PASS
---

# WORK-raise-coverage-baseline

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Raise the test-coverage baseline before gameplay feature expansion.
- **Scope:**
  - **In:** meaningful Rust tests raising `oathstar-core`, `oathstar-content`,
    `oathstar-storage` toward 100% line coverage; direct tests for the testable
    `oathstar-server` handlers (`health`, `state_snapshot`, `command`);
    documentation of every intentionally-uncovered line (file/line + rationale);
    the two stale ticket-#2 archive cross-link fixes.
  - **Out:** gameplay tickets #3–#7; rewriting the JS prototype engine
    (`src/engine.js` is being ported to Rust); any abstraction added solely to
    touch lines; lowering any coverage/MSI floor.
- **Systems:** engine (`oathstar-core`), content loader (`oathstar-content`),
  storage (`oathstar-storage`), server (`oathstar-server`), JS prototype (`src/`).

## Acceptance Criteria (EARS)
| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the full gate runs, Rust line coverage shall exceed the 88.66% baseline. | gate:14 output |
| REQ-002 | When the full gate runs, JS line coverage shall exceed the 75.19% baseline, or the remaining JS gap shall be documented in the notes as prototype-replacement debt. | gate:15 output + notes |
| REQ-003 | The `oathstar-core`, `oathstar-content`, and `oathstar-storage` crates shall reach near-100% meaningful line coverage or carry documented unreachable/composition-only exceptions. | `cargo llvm-cov` per-file + notes |
| REQ-004 | If any line remains uncovered by design, the pipeline notes shall identify the file/line area and the rationale. | notes review |
| REQ-005 | The full gate shall pass green with mutation MSI at 100% and no gate floor lowered. | `bin/gate.sh` green; gate:16 = 100%; floors + `bin/.clippy-allowlist` unchanged |

## Locked-In Decisions
- **Meaningful tests only.** No shallow line-touching; assert real behavior
  (error variants, messages, endpoint contracts).
- **No artificial abstractions for coverage** (CONSTITUTION; user directive).
  Server handlers are tested directly, not wrapped in extracted shims.
- **No floor lowered.** `RUST_COV_MIN`/`JS_COV_MIN`/`MUT_MSI_MIN` baked floors
  and `bin/.clippy-allowlist` stay as-is.
- **JS prototype is replacement-bound.** `src/world.js` stays 100%; the
  `src/engine.js` gap (subsystems being ported to Rust) is documented as
  prototype-replacement debt, not padded with throwaway tests.
- **Documented intentional exceptions** (carried from the coverage report;
  finalized in Design/Validate): core `move_direction` "unfinished world-data"
  branch (unreachable — `try_new` validates exit targets), `oathstar-server`
  `main` (binary composition root, already mutation-excluded) and the SSE stream
  loops (`events_json`/`events_html` — per-event transform is tested; the
  stream/keep-alive wiring is glue).

## Linked Artifacts
- Design docs: `docs/technical-architecture.md`, `docs/review-harness.md`
- Intake doc: none
- Ticket doc: `docs/planning/tickets/closed/TICKET-8-raise-coverage-baseline.md`
- Forge ticket: `e8a10f21-3e97-4c2a-87a7-6ea77f321502` (#8)
- AAR: `3cf6eaa5-db2b-4a64-a7cf-701d2853ab25`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
