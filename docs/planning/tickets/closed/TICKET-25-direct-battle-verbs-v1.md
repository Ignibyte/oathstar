---
title: TICKET-25-direct-battle-verbs-v1
status: closed
ticket: 3d9e96bb-e19a-4153-911f-70f38403e859
ticket_number: 25
type: feature
created: 2026-06-09
closed: 2026-06-09
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-direct-battle-verbs-v1.spec.md
---

# TICKET-25-direct-battle-verbs-v1

## Summary

Add the first direct battle verbs that queue into ticket #24's Phase 2 skill
window. During battle, players use direct verbs such as `guard`, `power strike`,
or `focus strike`; they do not type `skill <name>`.

## Why

Ticket #24 created the two-phase combat loop: Phase 1 resolves the baseline
exchange, then Phase 2 resolves a queued action or skips cleanly. That window
needs real battle verbs so combat starts feeling like a game system instead of an
auto-attack loop with flee.

Direct verbs preserve the MUD feel. They also keep the command parser readable:
players type the action they intend, and the engine decides whether that action
is valid in the current combat phase.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When combat is active and the player enters a supported battle verb during the between-pulse window, the engine shall queue that action for Phase 2. | Rust test |
| REQ-002 | When Phase 2 resolves with a queued battle verb, the engine shall apply the deterministic effect, emit combat-channel output, and clear the queued action. | Rust test |
| REQ-003 | When no battle verb is queued before Phase 2, the engine shall keep the ticket #24 skip behavior unchanged. | Rust test |
| REQ-004 | When the player enters a battle verb outside combat, the engine shall refuse cleanly without mutating state. | Rust test |
| REQ-005 | When a player tries to queue a second battle verb before the next pulse, the engine shall handle it deterministically without duplicating or corrupting the queued action. | Rust test |
| REQ-006 | The battle modal shall expose buttons for available direct battle verbs and show the queued action status while waiting for the next pulse. | JS/browser smoke |
| REQ-007 | Existing `attack`, `flee`, Nearby Attack, entity detail, combat pulse, Datastar feed, and boss/oath `confront` behavior shall continue to pass. | gate |

## Scope

- In: two or three deterministic direct battle verbs; parser support; queueing
  into Phase 2; battle modal buttons; tests; docs.
- Out: full skill tree, class unlocks, XP/skill progression, equipment scaling,
  cooldowns, mana/focus economy beyond any tiny deterministic placeholder, enemy
  skill AI, and multiplayer turn arbitration.

## Notes

- Forge ticket: pending. Mint this in Forge before implementation if the Forge
  connector is available.
- Use direct verbs during battle. Do **not** implement `skill <name>` as the
  player-facing battle command path.
- Keep v1 deterministic and small. Suggested verbs:
  - `guard`: reduce or prevent the next enemy return strike during Phase 2.
  - `power strike`: deal a small deterministic extra hit during Phase 2.
  - `focus strike`: optional only if it can stay simple with existing focus state.
- Prefer one or two verbs done cleanly over a larger list with weak semantics.
- Do not change the #24 pulse model unless the queue semantics require a narrow
  adjustment.
