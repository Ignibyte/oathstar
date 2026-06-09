---
title: TICKET-21-interaction-metadata-and-entity-contracts-v1
status: closed
ticket: ef9c9854-e3ed-4f86-a9e6-2bd9439456b4
ticket_number: 21
type: feature
created: 2026-06-07
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-entity-contracts-v1.spec.md
---

# TICKET-21-interaction-metadata-and-entity-contracts-v1

## Summary

Formalize role and interaction metadata so entities do not devolve into loose
strings and special cases.

## Why

Entities are intended to share one model while gaining behavior through roles,
contracts, and later code-behind hooks. As soon as talk, oath-giving, shops, and
combat start to coexist, the engine needs validation rules that say what metadata
each role must provide.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an entity declares a role such as talkable/oath_giver/shopkeeper/combatant/fixture, validation shall require the minimum metadata needed by that role where applicable. | Rust test |
| REQ-002 | When an entity lacks required role metadata, world validation shall fail with a typed error naming the entity, role, and missing field. | Rust test |
| REQ-003 | When command handlers check capabilities, they shall use typed helpers/contracts rather than ad hoc string checks wherever this ticket touches behavior. | code review / Rust test |
| REQ-004 | When Mara is declared as an oath-giver/talkable actor, her metadata shall satisfy the contract and load from TOML. | content test |
| REQ-005 | When the Bell-Eater is declared combatant/boss, its metadata shall remain valid and future-combat-ready without changing current boss progression. | content test |
| REQ-006 | Docs shall define the initial contract vocabulary and explain how code-behind/script hooks will attach later. | docs review |

## Scope

- In: typed contract helpers, validation for currently used roles, TOML schema
  updates, docs, and tests.
- Out: full scripting, shop inventory/economy, advanced NPC memory, combat AI,
  and dynamic mod loading.

## Notes

- Forge ticket: `ef9c9854-e3ed-4f86-a9e6-2bd9439456b4` (#21)
- This becomes more valuable after #18/#19 reveal which role metadata is actually
  needed by command handlers.
- Avoid turning roles into a large class hierarchy too early.
