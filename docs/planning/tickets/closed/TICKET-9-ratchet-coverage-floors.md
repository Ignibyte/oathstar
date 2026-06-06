---
title: TICKET-9-ratchet-coverage-floors
status: done
ticket: e97fbc02-b958-4156-8a93-f78f434e801b
ticket_number: 9
type: chore
created: 2026-06-06
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-ratchet-coverage-floors.spec.md
---

# TICKET-9-ratchet-coverage-floors

## Summary

Lock in ticket #8's coverage gains by ratcheting the baked-in gate floors up so
coverage cannot silently fall back to the old weak values. `RUST_COV_FLOOR`
60→94, `JS_COV_FLOOR` 70→75; `MUT_MSI_FLOOR` stays 100. Update every doc that
states the floors to match.

## Why

Ticket #8 raised Rust line coverage to 94.34% (JS 75.19%, MSI 100%), but the
gate still permits a regression all the way down to 60% / 70%. Ratcheting the
floors preserves the new baseline before gameplay systems land. Raising floors is
the sanctioned ratchet direction under CONSTITUTION §0 ("env may RAISE … never
lower below the §0 minimum").

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the full gate runs, the Rust coverage floor shall be at least 94%. | `bin/gate.sh` floors + gate:14 label |
| REQ-002 | When the full gate runs, the JS coverage floor shall be at least 75%. | `bin/gate.sh` floors + gate:15 label |
| REQ-003 | If an environment variable sets a coverage floor below the baked-in minimum, the gate shall clamp it back up. | clamp smoke check (`RUST_COV_MIN=10 bin/gate.sh --fast` prints "clamped") |
| REQ-004 | The mutation MSI floor shall remain 100%. | `bin/gate.sh` floors |
| REQ-005 | The governing docs (gate.sh header, CONSTITUTION.md, CLAUDE.md, docs/review-harness.md) shall state the same floor values as the gate script. | grep cross-check |
| REQ-006 | The full gate shall pass green at the new floors. | `bin/gate.sh` GREEN [full] |

## Scope

- In: raise the three floor constants in `bin/gate.sh`; fix the stale gate.sh
  header comment; update CONSTITUTION.md / CLAUDE.md / docs/review-harness.md to
  the new values; planning docs.
- Out: changing the clamp logic (already correct); adding tests (unless a change
  forces a failure); touching gameplay tickets #3–#7; lowering/faking coverage.

## Notes

- Forge ticket: `e97fbc02-b958-4156-8a93-f78f434e801b` (#9)
- Observed coverage (clears the new floors): Rust 94.34%, JS 75.19%, MSI 100%.
- Builds on ticket #8 (`WORK-raise-coverage-baseline`).
- Completed pipeline: `docs/planning/pipeline/completed/WORK-ratchet-coverage-floors.spec.md`
