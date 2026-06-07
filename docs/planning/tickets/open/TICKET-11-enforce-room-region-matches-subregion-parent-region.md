---
title: TICKET-11-enforce-room-region-matches-subregion-parent-region
status: open
ticket: 5fd49448-bd4e-42a1-b900-e293b079e493
ticket_number: 11
type: chore
created: 2026-06-06
intake:
pipeline_spec:
---

# TICKET-11-enforce-room-region-matches-subregion-parent-region

## Summary

Enforce that a room's region matches its subregion's parent region in
`WorldDefinition::validate`.

## Why

The v1 world model (ticket #6) validates that a room's subregion *exists*
(`RoomSubregionMissing`) and that each subregion points to a real parent region
(`SubregionRegionMissing`), but it does **not** check that a room's own region
matches its subregion's parent region. A room can declare `region = "A"` while
sitting in a subregion whose parent region is `"B"` and still validate.

This is acceptable for ticket #6 — REQ-005 covers only *missing* references and
consistency was explicitly scoped out (see `AD-claude-world-model-v1-001`) — but
it should be ratcheted **before** any region-specific mechanics (local oath laws,
region hazards, encounter behavior, travel restrictions, map labels) start
depending on room↔subregion↔region coherence, or bad content will silently slip
through.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a room references a subregion whose parent region differs from the room's own region, `WorldDefinition::validate` shall reject the world with a typed error naming the room, its region, and the subregion's parent region. | Rust test |
| REQ-002 | When a room's region and its subregion's parent region agree, validation shall accept the world (no false rejection). | Rust test |

## Scope

- In: a new `WorldValidationError` variant + the consistency check in
  `oathstar-core::WorldDefinition::validate`; Rust tests (reject + accept); keep
  the full gate green (coverage ≥94%, mutation MSI 100%).
- Out: the other deferred v1 ratchets — registry key-integrity (`key != value.id`
  for the new registries), code-behind behaviors, role contracts, item
  flags/slots/elements, player-owned items.

## Notes

- Forge ticket: `5fd49448-bd4e-42a1-b900-e293b079e493`
- Deferred from: ticket #6 (Phase 3.5 inspect LOW finding); see
  `AD-claude-world-model-v1-001`.
- Reference: `crates/oathstar-core/src/lib.rs` `validate()` — the room loop
  (room→subregion check) and the subregion loop (subregion→region check) are
  where the new cross-region check belongs.
- Promoted from intake:
- Active pipeline:
