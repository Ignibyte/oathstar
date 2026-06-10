---
title: TICKET-29-boss-fight-v1-confront-the-bell-eater
status: closed
ticket: 89436a8c-4038-4c05-8519-ff28059e3626
ticket_number: 29
type: feature
created: 2026-06-10
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-boss-fight-v1.spec.md
---

# TICKET-29-boss-fight-v1-confront-the-bell-eater

## Summary

Make the beginner module's climax real: `confront` (sworn, boss present)
starts a pulse-loop combat encounter with the Bell-Eater instead of
instantly fulfilling the oath; victory drops the bell clapper through the
existing reward machinery; recovering the clapper is what fulfills the oath
and rings the bell.

## Why

`confront` still resolves the boss as a scripted instant win — the one
place the game cheats past the combat stack built across #22–#28 (pulse
loop, direct verbs, drops/xp/defeat consequences, fulfillment
announcements, save/load). Decision 007 wants boss battles authored and
memorable; the JS prototype's loop was "defeat drops key item"; the
Bell-Eater has carried future-combat stats and the clapper in its
inventory since #16/#21 as forward hooks. This ticket cashes them in and
completes the played loop: swear → practice on the stray → climb → boss →
clapper → the bell rings.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player confronts with the oath sworn and the boss present, the engine shall start a real combat encounter with the boss (CombatState active, pulses resolving) instead of resolving instantly. | Rust test |
| REQ-002 | When the boss falls in combat, the engine shall remove it, drop its authored inventory into the room, and award its authored XP through the existing end_combat funnel — without fulfilling the oath yet. | Rust test |
| REQ-003 | When the player takes the oath-flagged clapper while the oath is sworn, the oath shall be fulfilled — emitting OathFulfilled, delivering the authored fulfillment announcements per scope, and Mara's fulfilled dialogue shall be reachable. | Rust test |
| REQ-004 | If the player confronts without a sworn oath or after fulfillment, the engine shall refuse with distinguished messages and start no combat. | Rust test (both arms) |
| REQ-005 | If the player is defeated by the boss, the existing defeat consequences shall apply (reset to start with HP restored, deterministic XP penalty) and the boss, its inventory, and the sworn oath shall remain intact for a retry. | Rust test |
| REQ-006 | The played route shall run end to end over the server seam: talk Mara → swear → climb → confront → pulse fight → victory → clapper drops → take clapper → fulfilled + world announcement → Mara's fulfilled line. | server test |
| REQ-007 | Mid-boss-fight saves shall round-trip through the #28 surface without new save work. | Rust test |
| REQ-008 | Existing combat/pulse/direct-verb/reward/announcement/save behavior and the client build shall continue to pass. | gate |

## Scope

- In: confront-starts-combat (reusing CombatState/pulses/verbs wholesale);
  boss victory drop + authored xp via the existing #26 funnel; oath
  fulfillment moved from confront-success to clapper recovery (the #27
  announcements + Mara's fulfilled dialogue ride it unchanged); defeat =
  existing #26 consequences with boss/oath state intact for retry; the #16
  oath-gates-the-boss interlock preserved; content (bell_eater attack + xp,
  roost combat gating); deterministic tests + the served played route; docs.
- Out: alternate resolutions (persuade/spare/bind — Decision 007 future),
  boss phases/special moves, new verbs, levels, focus costs, multiplayer,
  UI redesign (the battle modal already renders combat).

## Notes

- Forge ticket: `89436a8c-4038-4c05-8519-ff28059e3626` (#29)
- Related docs: `docs/decisions.md` (007, 040, 042–046), `docs/combat-system.md`,
  `docs/mechanics-and-systems.md` (Conflict), `docs/event-lifecycle.md`
- Promoted from intake: none — carried observation from the #22–#28 session
  (confront bypasses the combat stack; the prototype's boss loop was
  "defeat drops key item").
- Active pipeline: `WORK-boss-fight-v1`
- Design decides (documented): boss combat-entry path (hostile role vs a
  confront-specific entry; attack-verb semantics for bosses), the
  fulfillment trigger shape (the existing `flags = ["oath"]` on
  bell_clapper vs an authored objective link), re-confront-after-victory
  and after-fulfillment messages, flee-from-boss semantics, boss stat
  values (winnable-but-dangerous vs hp 20 / strike 4).
