---
pipeline_id: REPLACE-WITH-UUID
title: WORK-<short-title>
ticket: REPLACE-WITH-FORGE-TICKET-ID
type: work
intake:
notes: WORK-<short-title>.notes.md
status: Phase 1 — Plan IN PROGRESS
---

# WORK-<short-title>

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** <one line>
- **Scope:** <what's in; what's explicitly out>
- **Systems:** <oath | combat | map | inventory | parser | engine | ui | …>

## Acceptance Criteria (EARS)
Each acceptance criterion must use EARS syntax, describe one observable
behavior, and include a verification method.

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When <trigger>, the <system> shall <observable response>. | <test, smoke check, doc check, or review check> |
| REQ-002 | The <system> shall <observable response>. | <test, smoke check, doc check, or review check> |

## Locked-In Decisions
- <decision that should not be re-litigated mid-pipeline>

## Linked Artifacts
- Design docs: <docs/*.md touched>
- Intake doc: <docs/planning/intake/*.md or none>
- Ticket doc: <docs/planning/tickets/open/TICKET-*.md>
- Forge ticket: <id>

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | — |
| 2 — Design | — |
| 3 — Implement | — |
| 3.5 — Inspect | — |
| 4 — Validate | — |
| 5 — Complete | — |
