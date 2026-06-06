---
title: TICKET-7-build-beginner-module-vertical-slice-in-rust
status: open
ticket: c1937d4e-2367-4884-a6e5-bcc7023f6a57
ticket_number: 7
type: feature
created: 2026-06-06
intake:
pipeline_spec:
---

# TICKET-7-build-beginner-module-vertical-slice-in-rust

## Summary

Port the beginner proof-of-concept loop into the Rust authority path.

## Why

The Beginner module is our proof that the architecture works before the game
gets huge. It should demonstrate the local town, a simple oath, exploration to a
tower, a boss endpoint, and typed events that clients can render.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a new game starts, the Rust engine shall place the player in the beginner town start room with a typed room description event. | Rust test |
| REQ-002 | When the player accepts the beginner oath, the Rust engine shall record oath state and emit a typed oath event. | Rust test |
| REQ-003 | While the beginner oath is active, the Rust engine shall allow progression along the authored route toward the tower. | Rust test |
| REQ-004 | When the player reaches the beginner boss endpoint, the Rust engine shall resolve the authored encounter outcome through typed events. | Rust test |
| REQ-005 | The server API shall expose the vertical slice through the same command and event path used by normal play. | Rust integration or API smoke test |

## Scope

- In: Rust beginner content path, oath state placeholder, route progression, boss endpoint placeholder, server smoke coverage.
- Out: Full combat system, advanced class transformations, multiplayer, LLM/DM scripting.

## Notes

- Forge ticket: `c1937d4e-2367-4884-a6e5-bcc7023f6a57`
- Related docs: `docs/vertical-slice.md`, `docs/game-overview.md`, `docs/event-lifecycle.md`, `docs/protocol-and-output.md`
- Promoted from intake:
- Active pipeline:
