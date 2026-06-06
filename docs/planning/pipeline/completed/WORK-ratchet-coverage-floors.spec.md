---
pipeline_id: 81ba5b7e-fc2c-4eb3-8a75-87f52ff2595b
title: WORK-ratchet-coverage-floors
ticket: e97fbc02-b958-4156-8a93-f78f434e801b
ticket_number: 9
aar_id: f226f854-6974-4e4e-997e-02b0c3667ebf
type: work
intake:
notes: WORK-ratchet-coverage-floors.notes.md
status: Phase 5 — Complete PASS
---

# WORK-ratchet-coverage-floors

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Ratchet the baked-in coverage floors to lock in ticket #8's gains.
- **Scope:**
  - **In:** raise the three floor constants in `bin/gate.sh`
    (`RUST_COV_FLOOR` 60→94, `JS_COV_FLOOR` 70→75, `MUT_MSI_FLOOR` stays 100);
    fix the stale gate.sh header comment; update `CONSTITUTION.md`, `CLAUDE.md`,
    and `docs/review-harness.md` to state the same values; planning docs.
  - **Out:** changing the clamp logic (already correct + dynamic); adding tests
    (unless a change forces a real failure); gameplay tickets #3–#7;
    lowering/faking coverage.
- **Systems:** quality-gate harness (`bin/gate.sh`) + governing docs.

## Acceptance Criteria (EARS)
| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the full gate runs, the Rust coverage floor shall be at least 94%. | `bin/gate.sh` floor const + gate:14 label |
| REQ-002 | When the full gate runs, the JS coverage floor shall be at least 75%. | `bin/gate.sh` floor const + gate:15 label |
| REQ-003 | If an environment variable sets a coverage floor below the baked-in minimum, the gate shall clamp it back up to the baked floor. | clamp smoke check: `RUST_COV_MIN=10 bin/gate.sh --fast` prints the "clamped" note |
| REQ-004 | The mutation MSI floor shall remain 100%. | `bin/gate.sh` floor const |
| REQ-005 | The governing docs (gate.sh header, `CONSTITUTION.md`, `CLAUDE.md`, `docs/review-harness.md`) shall state the same floor values as the gate script. | grep cross-check, all read 94/75/100 |
| REQ-006 | The full gate shall pass green at the new floors. | `bin/gate.sh` → GREEN [full] |

## Locked-In Decisions
- **Floor values:** `RUST_COV_FLOOR=94`, `JS_COV_FLOOR=75`, `MUT_MSI_FLOOR=100`
  (user policy — set "a little below" the observed 94.34% / 75.19% so the gate
  has a hair of margin while preventing silent regression).
- **Clamp logic unchanged.** It is already dynamic (`MIN := env ?: FLOOR`, then
  clamp `MIN` up to `FLOOR`), so AC3 holds with only the constant change.
- **No coverage faked or lowered; no new tests** unless a gate/doc change causes
  an actual failure.
- **Ratcheting up aligns with CONSTITUTION §0** ("Floors ratchet up, never down").

## Linked Artifacts
- Files: `bin/gate.sh`, `CONSTITUTION.md`, `CLAUDE.md`, `docs/review-harness.md`
- Intake doc: none
- Ticket doc: `docs/planning/tickets/closed/TICKET-9-ratchet-coverage-floors.md`
- Forge ticket: `e97fbc02-b958-4156-8a93-f78f434e801b` (#9)
- AAR: `f226f854-6974-4e4e-997e-02b0c3667ebf`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
