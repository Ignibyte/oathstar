---
title: TICKET-3-validate-save-slot-names
status: open
ticket: e58bad86-b2e6-4e97-b36e-10c6bf63491d
ticket_number: 3
type: chore
created: 2026-06-06
intake:
pipeline_spec:
---

# TICKET-3-validate-save-slot-names

## Summary

Add a safe save-slot identifier boundary before names reach `FileSaveStore`
paths.

## Why

`FileSaveStore` currently joins arbitrary names into file paths. That is fine
while save names are internal, but it becomes risky once save/load endpoints or
UI controls accept player-provided slot names.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a save slot id contains only allowed characters, the storage system shall resolve it under the configured save root. | Rust test |
| REQ-002 | If a save slot id contains path separators, then the storage system shall reject it before building a path. | Rust test |
| REQ-003 | If a save slot id attempts parent-directory traversal, then the storage system shall reject it before building a path. | Rust test |
| REQ-004 | The storage API shall expose the validation rule through a small typed boundary instead of ad hoc string checks at future call sites. | Rust test plus review |

## Scope

- In: save slot validation type/helper, `FileSaveStore` call-site protection, tests.
- Out: Save/load HTTP endpoints, save browser UI, save schema versioning.

## Notes

- Forge ticket: `e58bad86-b2e6-4e97-b36e-10c6bf63491d`
- Related docs: `docs/technical-architecture.md`
- Promoted from intake:
- Active pipeline:
