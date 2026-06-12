---
pipeline_id: 5414334d-a015-4639-8222-0de76d3bfc96
title: WORK-focus-economy-v1
ticket: bb63c3d7-b213-4e27-9417-ac64a40e92c6
type: work
intake:
notes: WORK-focus-economy-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-focus-economy-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Focus Economy v1 — the #25 battle skills cost focus, with
  typed insufficient-focus refusals, a deterministic recovery path, and
  settled combat-end semantics (the HUD's focus bar finally measures
  something).
- **Scope:**
  - **In:** const focus costs for power strike and guard (numbers + the
    spend point at design, PLAYER_STRIKE_DAMAGE precedent); typed
    insufficient-focus QUEUE refusals through the existing queue_refusal
    pattern (both arms at the exact boundary); the change-tack replace
    settlement (no double-spend, no free-cancel gain; the
    fight-ends-with-action-queued case decided); recovery (design
    decides: a `rest` verb out of combat + refused mid-combat, and/or
    tick regen) + combat-end focus semantics (victory/defeat/flee);
    level-up stays HP-only (#30); zero-or-thin client work (the HUD bar
    exists; battle-modal visibility verified at design); save
    preservation + the crafted-value operator sweep over new arithmetic;
    deterministic tests + the served economy fight; docs.
  - **Out:** rituals/warding spells, focus items, max-focus growth,
    cooldowns, new skills beyond rest, UI redesign, multiplayer.
- **Systems:** combat, engine, parser (if `rest` lands), ui(verify-thin),
  protocol(none expected), storage(none).

## Acceptance Criteria (EARS)
Verbatim from `TICKET-31` (forge `bb63c3d7-b213-4e27-9417-ac64a40e92c6`).

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

## Locked-In Decisions
Settled before design; not re-litigated mid-pipeline. The open *design*
choices they leave are enumerated in the notes for Phase 2 to settle.

- **Deterministic, no RNG; costs are constants** (engine consts like
  `POWER_STRIKE_DAMAGE` — module-authored skill economies are future
  work alongside skills content).
- **Refusal at the QUEUE, through the existing machinery.** The #25
  `queue_combat_action` flow (refusal/confirmation/already/change-tack)
  is the seam — no new command surface for the costs themselves.
- **No exploit seams.** Whatever spend point design picks, replacing a
  queued action and a fight ending with an action queued must settle
  costs with no double-spend and no free gain — both documented and
  pinned.
- **Free verbs stay free.** `attack` and `flee` cost nothing in v1
  (flee's cost is the pulse you endure — #24's design); only the two
  skills price in.
- **Level-up benefit stays HP-only** (#30/Decision 048); max focus does
  not grow here.
- **Process bounds (owner-set, END-TO-END).** Auto-approve through ALL
  phases AND `/commit`: fast gate at validate; FULL gate at `/commit`;
  then commit + ff-merge to main + push origin main. Stop only on a gate
  failure or scope conflict. Codex strays untouchable.

## Linked Artifacts
- Design docs: `docs/combat-system.md`, `docs/mechanics-and-systems.md`
  (Conflict / Failure And Recovery), `docs/decisions.md` (043, 048),
  `docs/ui-design.md`
- Intake doc: none — the #30 closeout follow-up
- Ticket doc: `docs/planning/tickets/open/TICKET-31-focus-economy-v1-battle-skills-cost-focus.md`
- Forge ticket: `bb63c3d7-b213-4e27-9417-ac64a40e92c6` (#31)
- AAR: (recorded in notes at Phase 1 closeout)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
