---
title: TICKET-19-npc-dialogue-and-oath-offering-v1
status: closed
ticket: 8a66fea8-56eb-4015-b445-2608b8c4ddbf
ticket_number: 19
type: feature
created: 2026-06-07
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-npc-dialogue-and-oath-offering.spec.md
---

# TICKET-19-npc-dialogue-and-oath-offering-v1

## Summary

Move the beginner oath from a global anytime action toward an NPC-offered oath
flow, starting with Mara.

## Why

The Hollow Bell oath currently works mechanically, but it can be sworn without
context. The beginner slice will feel more like an authored RPG if the player
discovers the problem by talking to Mara, then swears the oath after it has been
offered.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a talkable NPC has dialogue metadata, `talk <npc>` shall return authored dialogue from world/module data rather than a hardcoded generic response. | Rust test |
| REQ-002 | When the player talks to Mara from an interactable distance, the response shall introduce the Hollow Bell problem and expose that an oath can be sworn. | Rust/server test |
| REQ-003 | When the player attempts to swear before discovering or being offered the oath, the engine shall refuse or guide them toward the oath-giver instead of allowing a contextless global oath. | Rust test |
| REQ-004 | When the oath has been offered, `swear`/`vow` shall bind the offered oath and emit the existing oath event shape. | Rust test |
| REQ-005 | When the oath is already sworn or fulfilled, Mara's dialogue shall reflect that state at a minimal authored level. | Rust test |
| REQ-006 | The oath model shall record enough issuer/source metadata to support future oath-giver UI and region/faction effects. | Rust test / docs |
| REQ-007 | Existing beginner oath progression to the Bell-Eater shall remain playable. | gate / server smoke |

## Scope

- In: minimal dialogue metadata, Mara's beginner dialogue, oath-offered state,
  source/issuer metadata, tests, and docs.
- Out: branching dialogue UI, modal conversations, persuasion, reputation
  changes, LLM dialogue, and multiple simultaneous oath offers.

## Notes

- Forge ticket: `8a66fea8-56eb-4015-b445-2608b8c4ddbf` (#19)
- This ticket should follow #18 so `talk mara` already exists.
- Keep the oath event shape compatible unless there is a strong reason to add
  optional metadata.
