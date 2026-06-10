---
title: TICKET-28-save-and-load-v1-session-persistence
status: closed
ticket: 27ec2cb2-b62d-4b26-aa1b-74c6e176c5dc
ticket_number: 28
type: feature
created: 2026-06-10
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-save-load-v1.spec.md
---

# TICKET-28-save-and-load-v1-session-persistence

## Summary

Wire session persistence end to end: a versioned save payload carrying the
complete session (mutated world + game state + event counter) saved to and
loaded from a named slot through the existing hardened `FileSaveStore`, with
server endpoints, an input-hardened engine load surface, and the client's
stubbed Save/Load buttons made real.

## Why

The grind loop now produces state worth keeping — XP, drops, defeated
enemies, oath progress — and all of it evaporates on restart (the in-memory
gap flagged at #22 R5 and carried through Decisions 044/045's revisit
triggers). The storage layer was built and hardened for this in #3/#10
(slot-name validation, symlink defense) and has zero callers. Loading is the
codebase's first real file-input boundary: a save file is untrusted input
and must fail typed and loud, never panic.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player saves to a valid slot, the engine shall persist the complete session (mutated world + game state + event counter, with a format version) through the existing `FileSaveStore`, without mutating the running session. | Rust test |
| REQ-002 | When the player loads a valid save, the engine shall restore the session exactly — the post-load snapshot shall equal the at-save snapshot byte-for-byte. | Rust test |
| REQ-003 | If a save file is missing, malformed, version-mismatched, or fails world validation, then loading shall be refused with a typed error and the running session shall remain unchanged. | Rust test |
| REQ-004 | When a save or load names an invalid slot, the request shall be refused through the existing slot-name validation without touching the filesystem. | Rust test |
| REQ-005 | The server shall expose save/load endpoints whose engine swap happens atomically under the engine lock, so pulses and commands never observe a partial session. | server test |
| REQ-006 | When the client's Save/Load buttons are used, the session shall persist/restore through those endpoints and the rendered state shall refresh. | server test + JS review/smoke |
| REQ-007 | The played loop shall round-trip: earned XP and a taken drop survive save → defeat → load. | integration test |
| REQ-008 | Existing combat/pulse/direct-verb/reward/announcement/oath behavior, the Datastar feed, and the client build shall continue to pass. | gate |

## Scope

- In: the versioned save payload (world + state + event counter); engine
  save/load surface with typed errors and world re-validation on load; the
  existing `FileSaveStore`/slot-validation as the only storage path; server
  save/load endpoints with the atomic in-lock engine swap; client Save/Load
  button wiring (single default slot); deterministic round-trip + played-loop
  tests; docs.
- Out: autosave, multiple profiles / slot-picker UI, cloud sync, encryption,
  migration tooling beyond a version field + loud rejection, Tauri-specific
  save paths beyond a configurable save root.

## Notes

- Forge ticket: `27ec2cb2-b62d-4b26-aa1b-74c6e176c5dc` (#28)
- Related docs: `docs/decisions.md` (040 R5, 044, 045), `docs/ui-design.md`
  (the stubbed buttons), `docs/mechanics-and-systems.md`
- Promoted from intake: none — carried-forward engineering item documented in
  the #22/#26/#27 pipeline notes.
- Active pipeline: `WORK-save-load-v1`
- Design decides (documented): mid-combat save semantics (CombatState
  serializes completely — persist vs refuse), the stale opening-scene seed
  after load, load-time feed narration, and the save-root configuration.
