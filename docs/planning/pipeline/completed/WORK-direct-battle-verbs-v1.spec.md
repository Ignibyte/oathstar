---
pipeline_id: 78bc0e84-cd5b-4513-bcee-4cd433f04f8c
title: WORK-direct-battle-verbs-v1
ticket: 3d9e96bb-e19a-4153-911f-70f38403e859
type: work
intake:
notes: WORK-direct-battle-verbs-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-direct-battle-verbs-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Direct Battle Verbs v1 — the first non-terminal `CombatAction`
  variants: direct battle verbs (`guard`, `power strike`) that queue into
  #24's Phase-2 skill window between pulses and resolve with small,
  deterministic effects; parser support with clean outside-combat refusal;
  documented deterministic second-queue semantics; battle-modal verb buttons
  + queued status — all preserving the #22/#23/#24 combat surface.
- **Scope:**
  - **In:** two (max three) deterministic direct battle verbs — `guard`
    (reduce/prevent the next enemy return strike, resolved through Phase 2)
    and `power strike` (small deterministic extra hit in Phase 2); `focus
    strike` only if it stays tiny against existing focus state (no economy);
    parser support for the verbs (outside combat: refuse cleanly, zero
    mutation); queueing into the #24 Phase-2 window with a documented,
    deterministic second-queue rule (replace-with-event or refuse-with-event);
    combat-channel output for queue + resolution sufficient for the feed and
    modal; battle-modal buttons for the verbs + queued-status display; Rust +
    JS tests for every EARS REQ; docs.
  - **Out:** the full skill tree, class unlocks, XP/skill progression,
    equipment scaling, cooldowns, mana/focus economy beyond a tiny
    deterministic placeholder, enemy skill AI, multiplayer turn arbitration,
    and any change to the #24 pulse model beyond narrow queue-semantics
    adjustments.
- **Systems:** combat, engine, parser, protocol, ui.

## Acceptance Criteria (EARS)
Verbatim from `TICKET-25` (forge `3d9e96bb-e19a-4153-911f-70f38403e859`).

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When combat is active and the player enters a supported battle verb during the between-pulse window, the engine shall queue that action for Phase 2. | Rust test |
| REQ-002 | When Phase 2 resolves with a queued battle verb, the engine shall apply the deterministic effect, emit combat-channel output, and clear the queued action. | Rust test |
| REQ-003 | When no battle verb is queued before Phase 2, the engine shall keep the ticket #24 skip behavior unchanged. | Rust test |
| REQ-004 | When the player enters a battle verb outside combat, the engine shall refuse cleanly without mutating state. | Rust test |
| REQ-005 | When a player tries to queue a second battle verb before the next pulse, the engine shall handle it deterministically without duplicating or corrupting the queued action. | Rust test |
| REQ-006 | The battle modal shall expose buttons for available direct battle verbs and show the queued action status while waiting for the next pulse. | JS/browser smoke |
| REQ-007 | Existing `attack`, `flee`, Nearby Attack, entity detail, combat pulse, Datastar feed, and boss/oath `confront` behavior shall continue to pass. | gate |

## Locked-In Decisions
Settled before design; not re-litigated mid-pipeline. The open *design*
choices they leave are enumerated in the notes for Phase 2 to settle.

- **OWNER PRODUCT DECISION (binding): direct verbs.** During battle the
  player types the action (`guard`, `power strike`) — `skill <name>` is NOT
  the player-facing battle path and must not be implemented as one. Prefer
  one or two verbs done correctly over a list; `focus strike` ships only if
  it stays tiny with existing focus state.
- **Build on #24's Phase-2 window; do not change the pulse model (Decision
  042 / AD-claude-combat-pulse-rides-tick-001).** New verbs are additive
  `CombatAction` variants resolved by `resolve_queued_action`; Phase 1 stays
  the untouched baseline exchange; cadence/anchoring untouched. Narrow queue-
  semantics adjustments only where REQ-005 requires them.
- **Deterministic small effects only.** Fixed consts, no RNG, no cooldowns,
  no economy — the #22/#24 discipline that keeps 100% MSI honest. A killing
  Phase-2 effect ends combat with the existing outcome semantics; a fight
  ended by Phase 1 drops the queued verb (the #24 preemption rule).
- **The take-vs-peek consume must be pinned (AD f20f3ff4 consequence).** The
  first non-terminal action makes #24's noted equivalence observable: a
  resolved verb fires ONCE and the queue clears — a peek-not-take regression
  would re-fire it every pulse. A test pins single-fire.
- **Second-queue semantics are deterministic, documented, and tested
  (REQ-005).** Design picks replace-with-event or refuse-with-event (one
  uniform rule, including its interaction with a queued `flee`) and records
  why; either way the player gets a clear combat-channel line.
- **Additive protocol only (Decisions 028/031).** `CombatSnapshot.queuedAction`
  already carries the queued-action string — new verbs ride it as new values;
  any new event kinds are additive; Datastar feed behavior stays consistent
  with existing combat events (queue/resolve lines via the established
  `CombatMessage` path unless design shows a typed marker is needed).
- **Modal buttons send direct verbs (Decision 032 split).** Buttons issue
  `runCommand("<verb>")` exactly as typed; queued status renders through the
  existing `toBattle` view-model (`queuedAction`/`queuedActionLabel`) — label
  decisions live in the view-model, not the DOM glue.
- **Preserve the combat surface (REQ-007).** `attack`, `flee`, Nearby Attack
  affordances, entity detail, `CombatPulse` refresh, the Datastar feed, and
  boss/oath `confront` keep passing untouched.
- **Process bounds (owner-set).** Validate runs `./bin/gate.sh --fast`; the
  FULL gate and `/commit` are owner-gated — ask Codex before either. The
  pipeline stops after Phase 5 (complete + archive), NO commit. Untracked
  `assets/tilesets/` and `bin/generate_oathstar_tileset.py` stay untouched.

## Linked Artifacts
- Design docs: `docs/combat-system.md` (Combat Timing, v2-implemented),
  `docs/decisions.md` (040, 041, 042), `docs/ui-design.md` (battle modal),
  `docs/protocol-and-output.md`
- Intake doc: none
- Ticket doc: `docs/planning/tickets/closed/TICKET-25-direct-battle-verbs-v1.md`
- Forge ticket: `3d9e96bb-e19a-4153-911f-70f38403e859` (#25)
- AAR: `41f34dce-18d0-4eee-a1b3-7dbe30d39864`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
