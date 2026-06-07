---
title: TICKET-5-design-command-parser-v1
status: done
ticket: 6651420b-9cd1-418c-9f66-98ff825c6650
ticket_number: 5
type: feature
created: 2026-06-06
closed: 2026-06-06
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-command-parser-v1.spec.md
---

# TICKET-5-design-command-parser-v1

## Summary

Define and implement the first Rust command parser layer for MUD-style input.

## Why

The game needs forgiving text input early: `look`, `look <mob>`, movement
aliases, target phrasing, and clear unknown-command feedback. This should live
behind typed commands so future UI buttons, Datastar actions, and LLM/DM actions
can share the same engine path.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player enters a movement alias, the parser shall produce the matching typed movement command. | Rust test |
| REQ-002 | When the player enters `look <target>` or `examine <target>`, the parser shall produce a typed inspect command with the target text preserved. | Rust test |
| REQ-003 | When the player enters extra whitespace or mixed case, the parser shall normalize the command without changing meaningful target text. | Rust test |
| REQ-004 | If input cannot be parsed into a known command, then the engine shall return a typed unknown-command event without mutating game state. | Rust test |

## Scope

- In: parser module/API, typed command enum, command handling updates, tests.
- Out: Natural-language intent parsing, LLM interpretation, combat commands beyond a small placeholder if needed.

## Notes

- Forge ticket: `6651420b-9cd1-418c-9f66-98ff825c6650`
- Related docs: `docs/mechanics-and-systems.md`, `docs/protocol-and-output.md`, `docs/technical-architecture.md`
- Promoted from intake:
- Completed pipeline: docs/planning/pipeline/completed/WORK-command-parser-v1.spec.md
