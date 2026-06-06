---
pipeline_id: 47060ed7-7793-4d14-b526-b972c8132839
title: WORK-validate-save-slot-names
ticket: e58bad86-b2e6-4e97-b36e-10c6bf63491d
ticket_number: 3
aar_id: d8694ec2-28b5-422a-afc6-f2bbdb721804
type: work
intake:
notes: WORK-validate-save-slot-names.notes.md
status: Phase 5 — Complete PASS
---

# WORK-validate-save-slot-names

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Validate save slot names — a typed save-slot-id boundary in `oathstar-storage`.
- **Scope:**
  - **In:** a typed slot-name validation boundary (typed `SaveSlotError`) in
    `oathstar-storage`; enforcement at `FileSaveStore` so a malformed name is
    rejected before a path is built; Rust tests for every invariant.
  - **Out:** save/load HTTP endpoints, save-browser UI, save schema versioning
    (the boundary is built so those can adopt it later).
- **Systems:** storage (`oathstar-storage` — `FileSaveStore`).

## Acceptance Criteria (EARS)
| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a save slot id contains only allowed characters, the storage system shall resolve it under the configured save root. | Rust test |
| REQ-002 | If a save slot id contains a path separator (`/` or `\`), then the storage system shall reject it with a typed error before building a path. | Rust test |
| REQ-003 | If a save slot id attempts parent-directory traversal (`..`), then the storage system shall reject it with a typed error before building a path. | Rust test |
| REQ-004 | The storage API shall expose the slot-name validation as a small typed boundary (a typed error) reused by its own call sites, not ad-hoc string checks scattered at future call sites. | Rust test + source review |
| REQ-005 | If a save slot id is empty or contains a disallowed character, then the storage system shall reject it with a typed error. | Rust test |

## Locked-In Decisions
- **Validation lives in `oathstar-storage`** as a typed boundary returning a
  typed `SaveSlotError` (mirrors ticket #2's "validate at the edge, typed error"
  pattern; prevention rule `9769be11`, AD `2530308c`).
- **Allowlist policy:** a slot id is valid iff non-empty and every character is
  ASCII alphanumeric or `-`/`_`. Path separators and `..` traversal get their
  own specific error variants (so REQ-002/003 are distinguishable + testable);
  any other out-of-allowlist character is a disallowed-character error.
- **Enforced at `FileSaveStore`** so the existing `&str` API cannot build a path
  from an unvalidated name; the validator is public so future save/load call
  sites reuse it instead of re-checking (REQ-004).
- **No gate floor lowered.** New code is meaningfully tested (assert the exact
  rejection variant) to hold the ratcheted floors (Rust ≥94%, MSI 100%).
- **Deferred to Phase 2 — Design (NOT locked):** exact boundary shape — a
  `SaveSlotId` newtype vs a `validate_save_slot_name(&str) -> Result<(), SaveSlotError>`
  function — and whether `path_for` becomes fallible vs a newtype-typed API.

## Linked Artifacts
- Design docs: `docs/technical-architecture.md`
- Intake doc: none
- Ticket doc: `docs/planning/tickets/closed/TICKET-3-validate-save-slot-names.md`
- Forge ticket: `e58bad86-b2e6-4e97-b36e-10c6bf63491d` (#3)
- AAR: `d8694ec2-28b5-422a-afc6-f2bbdb721804`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
