---
title: TICKET-4-add-tauri-shell-quality-gate
status: done
ticket: 2885b6ed-5deb-403b-afcb-67f80b35eb1d
ticket_number: 4
type: chore
created: 2026-06-06
closed: 2026-06-06
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-add-tauri-shell-quality-gate.spec.md
---

# TICKET-4-add-tauri-shell-quality-gate

## Summary

Bring `src-tauri` into the quality story with an explicit check or build gate.

## Why

The Constitution currently records that Tauri is outside the Rust workspace
compile gates. That is honest for now, but shell-side Rust should become gated
before the desktop client grows meaningful behavior.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the full gate runs, the Tauri shell shall be checked or built by an explicit command. | `bin/gate.sh` |
| REQ-002 | When the fast gate runs, the Tauri shell shall receive the cheapest useful validation that does not make iteration painful. | `bin/gate.sh --fast` |
| REQ-003 | If Tauri dependencies are intentionally unused during early shell setup, then the gate shall document and enforce the allowed exception. | Gate run plus review |
| REQ-004 | The review harness docs shall describe the Tauri gate scope accurately. | Doc check |

## Scope

- In: gate script updates, docs updates, any needed Tauri check command.
- Out: New Tauri features, frontend redesign, installer packaging.

## Notes

- Forge ticket: `2885b6ed-5deb-403b-afcb-67f80b35eb1d`
- Related docs: `CONSTITUTION.md`, `docs/review-harness.md`, `docs/technical-architecture.md`
- Promoted from intake:
- Completed pipeline: docs/planning/pipeline/completed/WORK-add-tauri-shell-quality-gate.spec.md
