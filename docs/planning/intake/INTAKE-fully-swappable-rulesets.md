---
title: INTAKE-fully-swappable-rulesets
status: candidate
created: 2026-06-06
ticket:
pipeline_spec:
---

# INTAKE-fully-swappable-rulesets

## Problem / Opportunity

Oathstar should strive to become more than a fixed game with optional content
packs. The long-term goal is a modular roleplaying engine where groups can swap
major rule systems and create very different play experiences from the same
runtime.

This should support solo play, friend-group roleplay, dungeon-master sessions,
authored modules, and eventually LLM-assisted directors.

## Proposed Outcome

The engine exposes stable contracts for replaceable systems. A game world can
choose its battle system, stat model, progression model, inventory model,
director model, and UI/rendering components without rewriting the core runtime.

The core remains deterministic and protective: it owns saves, event ordering,
module loading, validation, and state commits. Modules provide behavior through
contracts rather than arbitrary mutation.

## Candidate EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The runtime shall define a minimal core kernel that is independent of any specific battle, stat, inventory, or progression model. | Architecture review |
| REQ-002 | When a new game is created, the runtime shall be able to load a module preset that chooses compatible rule-system modules. | Integration test |
| REQ-003 | If two enabled modules provide conflicting implementations for the same rule-system contract, then module validation shall reject the preset with a typed error. | Rust test |
| REQ-004 | When a rule-system module emits state changes, the core runtime shall validate and commit those changes through the shared event lifecycle. | Rust test |
| REQ-005 | The save format shall record the active modules and rule-system versions needed to reload the game safely. | Rust test |

## Scope Notes

- In:
  - High-level modularity principle
  - Rule-system contracts
  - Preset compatibility
  - Save compatibility
  - DM/LLM-friendly director module concept
- Out:
  - Immediate implementation
  - Public mod marketplace
  - Arbitrary native plugin loading
  - Untrusted third-party sandboxing

## Promotion Checklist

- [ ] Forge ticket created.
- [ ] Pipeline spec/notes pair created.
- [ ] `ticket:` frontmatter updated.
- [ ] `pipeline_spec:` frontmatter updated.
- [ ] `status:` changed to `promoted`.
