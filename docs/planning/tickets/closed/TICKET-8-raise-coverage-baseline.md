---
title: TICKET-8-raise-coverage-baseline
status: done
ticket: e8a10f21-3e97-4c2a-87a7-6ea77f321502
ticket_number: 8
type: chore
created: 2026-06-06
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-raise-coverage-baseline.spec.md
---

# TICKET-8-raise-coverage-baseline

## Summary

Harden the test baseline before gameplay feature expansion: raise
`oathstar-core`, `oathstar-content`, and `oathstar-storage` toward 100% line
coverage with meaningful tests, cover the testable `oathstar-server` handlers,
and document intentional exceptions (unreachable defensive branches, binary /
composition glue) and JS prototype-replacement debt. No gate floor is lowered;
mutation MSI stays at 100%.

## Why

Ticket #2 shipped green but with uneven coverage (Rust 88.66%, JS 75.19%).
Locking in meaningful tests now prevents testing debt from accumulating as
gameplay systems (#3–#7) land on top of the engine.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the full gate runs, Rust line coverage shall exceed the 88.66% baseline. | gate:14 output |
| REQ-002 | When the full gate runs, JS line coverage shall exceed the 75.19% baseline, or the remaining JS gap shall be documented in the pipeline notes as prototype-replacement debt. | gate:15 output + notes |
| REQ-003 | The `oathstar-core`, `oathstar-content`, and `oathstar-storage` crates shall reach near-100% meaningful line coverage or carry documented unreachable/composition-only exceptions. | `cargo llvm-cov` per-file + notes |
| REQ-004 | If any line remains uncovered by design, the pipeline notes shall identify the file/line area and the rationale. | notes review |
| REQ-005 | The full gate shall pass green with mutation MSI at 100% and no gate floor lowered. | `bin/gate.sh` green; gate:16 = 100%; floors + `.clippy-allowlist` unchanged |

## Scope

- In: meaningful Rust tests for core/content/storage; direct tests for the
  testable server handlers; documented intentional exceptions; the two stale
  ticket-#2 archive cross-link fixes.
- Out: gameplay tickets #3–#7; rewriting the JS prototype engine; artificial
  abstractions added solely to touch lines; lowering any coverage/MSI floor.

## Notes

- Forge ticket: `e8a10f21-3e97-4c2a-87a7-6ea77f321502` (#8)
- Baseline (post-ticket-#2): Rust 88.66%, JS 75.19%, mutation MSI 100%.
- Related: completed `WORK-harden-core-world-init`; `PR-claude-loader-testability-001`.
- Promoted from intake:
- Completed pipeline: `docs/planning/pipeline/completed/WORK-raise-coverage-baseline.spec.md`
