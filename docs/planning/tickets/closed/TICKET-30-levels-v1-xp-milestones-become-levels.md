---
title: TICKET-30-levels-v1-xp-milestones-become-levels
status: closed
ticket: 379aa89b-be69-4aa5-93f8-b7754f1d1b63
ticket_number: 30
type: feature
created: 2026-06-10
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-levels-v1.spec.md
---

# TICKET-30-levels-v1-xp-milestones-become-levels

## Summary

Make levels real and visible: a deterministic level curve over the player's
XP, recomputed wherever XP changes, with a visible max-HP benefit, a typed
`LevelUp` event in the feed, and a thin HUD addition showing level + XP
(both verified absent in the client today).

## Why

XP flows since #26/#29 — stray 5, boss 25, the defeat penalty — but
`PlayerState.level` has been a constant 1 since #7 and the client renders
neither level nor XP. The grind loop earns a number nothing reads.
mechanics-and-systems.md locks "levels for broad milestone growth"; this is
that milestone growth, kept to one slice.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an XP change crosses a level threshold, the engine shall set the level per the deterministic curve and emit a typed LevelUp event naming the new level. | Rust test |
| REQ-002 | When the level rises, the player's maximum HP shall increase deterministically per the documented benefit rule, observable in the snapshot. | Rust test |
| REQ-003 | When XP is reduced by the defeat penalty, the engine shall apply the documented level-retention rule deterministically. | Rust test |
| REQ-004 | When a single XP award crosses multiple thresholds, the engine shall land on the correct level with each level's benefit applied exactly once. | Rust test |
| REQ-005 | The LevelUp event shall render in the Datastar feed and JSON stream without a duplicate human log line. | Rust test |
| REQ-006 | The client HUD shall display the current level and XP through the existing view-model pattern (thin glue). | node test + review |
| REQ-007 | Save/load shall round-trip level and XP coherently, with a deterministic documented posture for a stored level that disagrees with the curve. | Rust test |
| REQ-008 | The played progression shall run over the server seam: grinding the beginner road reaches level 2 and the boss victory lands level 3 with the authored thresholds. | server test |
| REQ-009 | Existing combat/oath/announcement/save behavior and the client build shall continue to pass. | gate |

## Scope

- In: the deterministic curve (placement decided at design); level
  recomputation at both xp-change sites; the max-HP benefit + heal
  semantics (design); the typed `LevelUp` event + datastar/JSON rendering
  with the renderer-arm check (no log twin); the thin HUD level+xp
  addition through `toHud`; save/load coherence posture for stale
  level/xp pairs; threshold tuning (stray grind → 2, boss → 3);
  operator-sweep on new arithmetic; deterministic tests + the served
  progression; docs.
- Out: classes, skills/skill points, percentage mastery, focus economy,
  level-gated content, equipment, HUD redesign.

## Notes

- Forge ticket: `379aa89b-be69-4aa5-93f8-b7754f1d1b63` (#30)
- Related docs: `docs/mechanics-and-systems.md` (Growth/Conflict),
  `docs/decisions.md` (008, 044, 046, 047), `docs/protocol-and-output.md`,
  `docs/ui-design.md`
- Promoted from intake: none — the #29 closeout follow-up (xp lands in a
  stat nothing reads) + the carried #26 "HUD xp display" item.
- Active pipeline: `WORK-levels-v1`
- Design decides (documented): curve placement (engine consts vs
  module-authored) + the exact thresholds; ratchet vs de-level on xp loss;
  max-HP growth amount + whether level-up heals; the stale-save posture
  (recompute vs tolerate vs gate); the LevelUp event payload shape.
- Verified at plan: the client has ZERO level/xp rendering today (`toHud`
  carries hp/focus/room/tick only); `PlayerSnapshot.level` already rides
  the wire; both xp-change sites live in core (`end_combat` award,
  `apply_defeat_penalty`).
