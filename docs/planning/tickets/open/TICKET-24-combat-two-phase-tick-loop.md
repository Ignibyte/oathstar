---
title: TICKET-24-combat-two-phase-tick-loop
status: open
ticket: pending-forge
ticket_number: 24
type: feature
created: 2026-06-09
intake:
pipeline_spec:
---

# TICKET-24-combat-two-phase-tick-loop

## Summary

Evolve combat from ticket #22's one-command-one-round loop into a repeating
two-phase combat cycle: an initial exchange phase followed by an optional skill
phase, repeating until someone flees or dies.

## Why

The v1 battle loop proves combat can start, advance, and end. The next combat
mechanical foundation should create room for player skill choices without making
every battle feel slow. A two-phase cycle keeps quick battles readable while
leaving space for future skills, class abilities, tactics, flee, and scripted
boss behavior.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When combat is active and a combat cycle begins, the engine shall resolve Phase 1 as the baseline exchange of initial hits/actions. | Rust test |
| REQ-002 | When Phase 1 completes and the fight has not ended, the engine shall enter a Phase 2 skill window. | Rust test |
| REQ-003 | When the player submits a valid skill during the Phase 2 window, the engine shall resolve that skill before advancing to the next combat cycle. | Rust test |
| REQ-004 | When no skill is submitted during the Phase 2 window, the engine shall skip the skill phase cleanly and continue to the next combat cycle. | Rust test |
| REQ-005 | When either side reaches zero HP during either phase, combat shall end immediately with the correct outcome instead of advancing phases. | Rust test |
| REQ-006 | When the player flees during combat, combat shall end with a fled outcome or clear flee state according to the authored rule. | Rust test |
| REQ-007 | The battle modal shall show the current phase/cycle state without blocking the command prompt or breaking ticket #22's summary-on-end behavior. | JS/browser smoke |
| REQ-008 | Existing attack start, hostile targeting, Nearby affordances, Datastar combat feed, and boss/oath `confront` behavior shall continue to pass. | gate |

## Scope

- In: explicit combat phase/cycle state; skill-window command handling; skip
  behavior when no skill is selected; flee endpoint; modal phase display; tests;
  docs.
- Out: full skill tree, equipment modifiers, class transformations, RNG, enemy AI
  tactics beyond the authored v1 return action, loot/rewards, and multiplayer
  turn arbitration.

## Notes

- Forge ticket: pending. Mint this in Forge before implementation if the Forge
  connector is available.
- Keep the first implementation deterministic. Skills can be placeholder verbs
  if needed, but the phase/cycle state should be real.
- This ticket should follow ticket #23 unless we decide combat cadence is more
  urgent than UI discoverability.
- The intended loop is:
  1. Phase 1: initial player/hostile exchange resolves.
  2. Phase 2: player skill window resolves if a skill was entered; otherwise it
     is skipped.
  3. Repeat until victory, defeat, flee, or a later alternate resolution.
