---
pipeline_id: 7670faf1-3622-4552-a30b-b4ab7af7ac64
title: WORK-levels-v1
ticket: 379aa89b-be69-4aa5-93f8-b7754f1d1b63
type: work
intake:
notes: WORK-levels-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-levels-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Levels v1 — a deterministic XP→level curve with a visible
  max-HP benefit, a typed `LevelUp` event, and a thin HUD level+XP display
  (the grind loop's payoff).
- **Scope:**
  - **In:** the deterministic curve over `PlayerState.xp` (placement —
    engine consts vs module-authored — decided at design; no RNG); level
    recomputation at BOTH xp-change sites (`end_combat` victory award,
    `apply_defeat_penalty`), with the xp-loss level-retention rule decided
    + documented; a visible level-up benefit (max-HP growth + heal
    semantics at design); a typed `LevelUp` protocol event (additive
    serde, snake_case tag) + datastar arm + JSON rendering with NO log
    twin (renderer-arm check first); a thin client HUD addition for level
    + xp through `toHud` (verified absent today — folds in the carried
    #26 HUD-xp item); save/load coherence posture for a stored level that
    disagrees with the curve (per Decisions 046/047); thresholds tuned so
    stray grinding reaches level 2 and the boss lands level 3;
    operator-sweep on all new arithmetic; deterministic tests + the
    served progression; docs.
  - **Out:** classes, skills/skill points, percentage mastery, focus
    economy, level-gated content, equipment, HUD redesign.
- **Systems:** engine, protocol, datastar, ui(thin), combat(touchpoints),
  storage(none expected beyond behavior round-trip).

## Acceptance Criteria (EARS)
Verbatim from `TICKET-30` (forge `379aa89b-be69-4aa5-93f8-b7754f1d1b63`).

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

## Locked-In Decisions
Settled before design; not re-litigated mid-pipeline. The open *design*
choices they leave are enumerated in the notes for Phase 2 to settle.

- **Deterministic, no RNG.** The curve is a pure function of xp; identical
  sessions level identically. Mutation testing pins exact thresholds.
- **Levels are milestone growth (mechanics-and-systems.md), one slice.**
  v1 is the curve + one visible benefit + the event + the HUD field —
  no classes, skills, or gating.
- **The event is typed and additive.** `LevelUp` joins `GameEventKind`
  under the snake_case wire convention (Decisions 028/031); renderers get
  arms; NO duplicate human log line (the #29 renderer-collision rule is
  binding — check the arms before pairing).
- **The client stays thin.** Level + xp enter through `toHud` and one DOM
  line in the existing HUD — no new panels, no redesign. (Verified at
  plan: the client renders neither today.)
- **Saves keep their posture.** Level/xp already serialize; whatever the
  design decides about stale stored-level/xp pairs must be deterministic,
  documented, and consistent with Decision 046/047's loud-refusal-over-
  silent-weirdness stance.
- **Process bounds (owner-set, END-TO-END).** Auto-approve through ALL
  phases AND `/commit`: fast gate at validate; FULL gate at `/commit`;
  then commit + ff-merge to main + push origin main. Stop only on a gate
  failure or scope conflict. Untracked `assets/tilesets/` +
  `bin/generate_oathstar_tileset.py` are untouchable.

## Linked Artifacts
- Design docs: `docs/mechanics-and-systems.md` (Growth and Progression /
  Conflict), `docs/decisions.md` (008, 044, 046, 047),
  `docs/protocol-and-output.md`, `docs/ui-design.md`
- Intake doc: none — #29 closeout follow-up + the carried #26 HUD-xp item
- Ticket doc: `docs/planning/tickets/open/TICKET-30-levels-v1-xp-milestones-become-levels.md`
- Forge ticket: `379aa89b-be69-4aa5-93f8-b7754f1d1b63` (#30)
- AAR: (recorded in notes at Phase 1 closeout)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS (engine-const curve 10/30/60/100, ratchet, +5 max-HP heal-to-full, Skill-channel LevelUp, lazy save convergence — see notes) |
| 3 — Implement | PASS (3 designed-churn tests red, handed to validate — see notes) |
| 3.5 — Inspect | PASS (wire.js payload-drop fixed — FAIL-claude-normalizer-strips-payload-live-fallback-001; doc fixes; see ledger) |
| 4 — Validate | PASS (9 new tests + 3 churn rewrites; 343 workspace + 48 node + build; GATE GREEN [fast] 14/14) |
| 5 — Complete | PASS (Decision 048 + 4 docs; AAR closed; PR-claude-renderer-tests-through-the-normalizer-001; ticket #30 closed; archived) |
