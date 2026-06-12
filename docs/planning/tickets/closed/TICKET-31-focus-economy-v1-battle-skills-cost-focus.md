---
title: TICKET-31-focus-economy-v1-battle-skills-cost-focus
status: done
ticket: bb63c3d7-b213-4e27-9417-ac64a40e92c6
ticket_number: 31
type: feature
created: 2026-06-10
closed: 2026-06-11
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-focus-economy-v1.spec.md
---

# TICKET-31-focus-economy-v1-battle-skills-cost-focus

## Summary

Make focus a real resource: the #25 battle skills (power strike, guard)
gain authored focus costs with typed insufficient-focus refusals, a
deterministic recovery path (`rest` and/or regen — design decides), and
settled combat-end semantics — so the HUD's focus bar finally measures
something and skill-spam stops dominating.

## Why

Focus has rendered 5/5 in the HUD since #7 but no mechanic reads or
writes it (the only product reference is the snapshot copy). Power strike
(6 dmg) being free strictly dominates attack (4); guard is costless
insurance. The mechanics doc has always pointed here: "use focus for
warding", "focus depletion blocks rituals until rest" — and `rest` is
already in the UI's utility verb list, unimplemented.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player queues power strike or guard with enough focus, the engine shall deduct the skill's authored cost exactly once, observable in the snapshot. | Rust test |
| REQ-002 | If the player queues a skill without enough focus, the engine shall refuse the queue with a typed line and change no state. | Rust test (both arms at the boundary) |
| REQ-003 | When a queued skill is replaced via change-tack, the engine shall settle costs deterministically with no double-spend and no free-cancel gain. | Rust test |
| REQ-004 | The engine shall provide a deterministic focus-recovery path usable out of combat and refused during combat. | Rust test (both arms) |
| REQ-005 | Combat-end outcomes shall apply the documented focus semantics deterministically (victory/defeat/flee). | Rust test |
| REQ-006 | The played boss fight shall run under the economy over the server seam, with the focus pool visibly limiting skill use. | server test |
| REQ-007 | Save/load shall round-trip focus through the new arithmetic without panics for any crafted value. | Rust test + operator sweep |
| REQ-008 | Existing combat/levels/oath/announcement/save behavior and the client build shall continue to pass. | gate |

## Scope

- In: const skill costs (numbers + spend point at design); typed
  insufficient-focus queue refusals (both arms); the change-tack
  cost-settlement rule; the recovery path (`rest` verb and/or regen);
  combat-end focus semantics; thin-or-zero client work; save preservation
  + crafted-value sweep; deterministic tests + the served economy fight;
  docs.
- Out: rituals/warding spells, focus items, max-focus growth, cooldowns,
  new skills beyond rest, UI redesign, multiplayer.

## Notes

- Forge ticket: `bb63c3d7-b213-4e27-9417-ac64a40e92c6` (#31)
- Related docs: `docs/combat-system.md` (#25 verbs block),
  `docs/mechanics-and-systems.md` (Conflict / Failure And Recovery),
  `docs/decisions.md` (043, 048), `docs/ui-design.md` (utility verbs)
- Promoted from intake: none — the #30 closeout follow-up.
- Active pipeline: `WORK-focus-economy-v1`
- Design decides (documented): cost numbers + placement; spend-on-queue
  vs spend-on-resolve + the replace/cancel settlement + the
  fight-ends-with-action-queued case; the recovery shape (`rest` verb
  vs tick regen vs both) and combat-end focus semantics (does defeat
  restore focus like hp?); battle-modal focus visibility (thin).
- Verified at plan: focus's only product-code reference is the snapshot
  copy (lib.rs:1149); `rest` parses as nothing today; the #25 queue
  machinery (queue_refusal/confirmation/already + change-tack) is the
  refusal seam; the HUD bar renders focus already.
