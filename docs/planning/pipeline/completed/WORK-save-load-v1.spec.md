---
pipeline_id: 8bb74a5b-15d2-4b46-a357-b19169e0f2b0
title: WORK-save-load-v1
ticket: 27ec2cb2-b62d-4b26-aa1b-74c6e176c5dc
type: work
intake:
notes: WORK-save-load-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-save-load-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Save & Load v1 — session persistence end to end: a versioned
  save payload carrying the complete session (mutated world + game state +
  event counter) through the existing hardened `FileSaveStore`, an
  input-hardened engine load surface, atomic server save/load endpoints, and
  the client's stubbed Save/Load buttons made real.
- **Scope:**
  - **In:** the save payload (format version + mutated `WorldDefinition` +
    `GameState` + `next_event_id`); an engine save/load surface — load
    treats the file as UNTRUSTED INPUT (typed errors, world re-validation,
    loud version rejection, no panics); the existing
    `validate_save_slot_name` + `FileSaveStore` as the only storage path
    (zero new storage code expected); server save/load endpoints with the
    engine swap performed atomically inside the `Arc<Mutex>` (the tick loop
    follows for free); client Save/Load button wiring through the
    established thin-glue pattern (v1: a single default slot, no UI
    redesign); deterministic round-trip + played-loop tests; docs.
  - **Out:** autosave, multiple profiles / slot-picker UI, cloud sync,
    encryption, migration tooling beyond a version field + loud rejection,
    Tauri-specific save paths beyond a configurable save root.
- **Systems:** storage(reuse), engine, server, ui(light), protocol(none
  expected).

## Acceptance Criteria (EARS)
Verbatim from `TICKET-28` (forge `27ec2cb2-b62d-4b26-aa1b-74c6e176c5dc`).

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

## Locked-In Decisions
Settled before design; not re-litigated mid-pipeline. The open *design*
choices they leave are enumerated in the notes for Phase 2 to settle.

- **The save is the COMPLETE session.** The #26 lesson is binding:
  `GameState` alone misses world mutations (removed placements, dropped
  items, cleared inventories), so the payload carries the mutated
  `WorldDefinition` + `GameState` + `next_event_id` (SSE id monotonicity
  across a load), under a format-version field — mismatches reject loudly;
  no migration tooling in v1.
- **Storage is the existing layer, untouched or near-untouched.** Every
  byte goes through `validate_save_slot_name` + `FileSaveStore` (#3/#10:
  traversal/reserved-name/symlink hardening, generic serde JSON). No
  parallel persistence path.
- **Load is an input path (§14, binding).** A save file is untrusted: the
  engine load surface returns typed errors for missing/malformed/
  version-mismatched files and re-validates the world through the same
  `try_new`-grade boundary; a failed load leaves the running session
  unchanged. No `unwrap`/`expect` reachable from file content.
- **The swap is atomic under the engine lock.** Endpoints lock the shared
  `Arc<Mutex<Engine>>` and replace the engine in place — the tick loop and
  concurrent commands serialize through the same lock, so no partial
  session is ever observable (the #24 concurrency seam, reused).
- **Client stays thin.** The existing Save/Load buttons call the endpoints
  and refresh state — no slot-picker, no new panels; the v1 slot is a
  single default name. Label/feedback decisions live where the house
  pattern puts them.
- **Deterministic round-trips are the proof.** Same session → save → load →
  byte-identical snapshots; the played loop (earn xp + fang → save → lose
  them → load → restored) is the acceptance demo. Both-arms staging applies
  to every refusal path (the standing fixture-distinguishability rule).
- **Process bounds (owner-set).** Validate runs `cargo test --workspace`,
  `node --test`, `npm run build`, `./bin/gate.sh --fast`; the FULL gate and
  `/commit` are owner-gated — STOP after Phase 5. Untracked
  `assets/tilesets/` + `bin/generate_oathstar_tileset.py` are untouchable.

## Linked Artifacts
- Design docs: `docs/decisions.md` (040 R5, 044, 045 revisit triggers),
  `docs/ui-design.md`, `docs/mechanics-and-systems.md`
- Intake doc: none — carried-forward engineering item (#22/#26/#27 notes)
- Ticket doc: `docs/planning/tickets/open/TICKET-28-save-and-load-v1-session-persistence.md`
- Forge ticket: `27ec2cb2-b62d-4b26-aa1b-74c6e176c5dc` (#28)
- AAR: `53b58ef7-25f4-411c-a5e5-91eef88106e3`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS (see notes — the coherence audit + overflow addendum are the contract) |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS (8 real findings fixed — see notes ledger; failure FAIL-claude-load-overflow-sweep-by-field-not-operator-001 + rule PR-claude-operator-sweep-untrusted-arithmetic-001 recorded) |
| 4 — Validate | PASS (15 new tests; 322 workspace + node + build; GATE GREEN [fast] 14/14) |
| 5 — Complete | PASS (Decision 046 + docs; AAR closed; AD-claude-save-load-untrusted-boundary-001; ticket #28 closed; archived) |
