---
title: TICKET-2-harden-core-world-initialization
status: done
ticket: 99619421-df57-4aec-976f-a4139eafd469
ticket_number: 2
type: chore
created: 2026-06-06
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-harden-core-world-init.spec.md
---

# TICKET-2-harden-core-world-initialization

## Summary

Move world invariant validation into `oathstar-core` so malformed module data
cannot construct an `Engine` that later panics.

## Why

The current beginner loader validates content, but the core engine public API
still trusts any `WorldDefinition`. The module system will eventually load
swappable worlds, so core should own the invariant boundary.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a world is missing its start room, the core engine shall reject initialization with a typed error. | Rust test |
| REQ-002 | When a world start room is not passable, the core engine shall reject initialization with a typed error. | Rust test |
| REQ-003 | If a room exit points to a missing room, then the core engine shall reject initialization with a typed error. | Rust test |
| REQ-004 | The core engine shall not use `expect` for world invariants reachable from malformed module data. | Rust test plus review |

## Scope

- In: `Engine::try_new` or equivalent validated constructor, tests for malformed worlds, call-site updates.
- Out: Full content loader redesign, dynamic module loading, persistence changes.

## Notes

- Forge ticket: `99619421-df57-4aec-976f-a4139eafd469`
- Related docs: `docs/module-system.md`, `docs/map-system.md`, `docs/technical-architecture.md`
- Promoted from intake:
- Completed pipeline: `docs/planning/pipeline/completed/WORK-harden-core-world-init.spec.md` (pipeline_id `70715e07-66b4-482a-a593-98f830cf7edf`, AAR `fa4ac433-a861-4278-afec-343044afbe6c`)
