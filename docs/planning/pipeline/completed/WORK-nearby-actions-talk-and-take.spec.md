---
pipeline_id: 9a6904ef-25b1-49cf-97c1-94728ddd21ca
title: WORK-nearby-actions-talk-and-take
ticket: c78c03e0-067f-4876-ba6f-17b760b5d2ff
type: work
intake:
notes: WORK-nearby-actions-talk-and-take.notes.md
status: Phase 5 — Complete PASS; ready for /commit (Codex review + FULL gate)
---

# WORK-nearby-actions-talk-and-take

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Nearby Actions V1 — `talk <target>` and `take <target>` commands that
  consume the ticket #17 spatial-awareness model.
- **Scope:**
  - **In:** two new parser commands (`talk`/`speak`, `take`/`get`/`pick up`) preserving
    target text; engine `talk`/`take` handlers that resolve through
    `awareness::resolve_target` (no duplicated distance math); the smallest durable
    player-carried item state; an additive, JSON-friendly pack snapshot field;
    removal of a taken item from world room/nearby contents; focused Rust tests;
    a minimal client `toPack` update so the already-wired Pack panel renders the
    new snapshot honestly (+ a JS test for that rendering).
  - **Out:** full ROT equipment, equipment slots, item use/equip, quantities/stacks,
    shops, dialogue trees, combat loot, persistence, and oath-offering changes
    (ticket #19). No new distance/geometry logic — reuse ticket #17.
- **Systems:** parser · engine · inventory · protocol · ui

## Acceptance Criteria (EARS)
Carried verbatim from ticket #18 (already EARS-form, one observable behavior each).

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the parser receives `talk`/`speak` with a target, it shall produce a typed talk command preserving the target text. | Rust test (`command.rs`) |
| REQ-002 | When the parser receives `take`/`get`/`pick up` with a target, it shall produce a typed take command preserving the target text. | Rust test (`command.rs`) |
| REQ-003 | When a talk target resolves to an interactable actor through spatial awareness, the engine shall emit a response for that actor without moving the player. | Rust test (`lib.rs`) |
| REQ-004 | When a talk target is visible but outside interaction radius, the engine shall report that the target is too far away to talk to. | Rust test (`lib.rs`) |
| REQ-005 | When a take target resolves to an interactable item placed in the world, the engine shall move that item into player-carried state and remove it from room/nearby contents. | Rust test (`lib.rs`) |
| REQ-006 | When a take target is visible but outside interaction radius, hidden, unknown, or not an item, the engine shall reject the action with a clear event and preserve state. | Rust test (`lib.rs`) |
| REQ-007 | When a snapshot is produced after taking an item, the item shall appear in player inventory/pack state through a minimal additive protocol shape. | Rust test (`oathstar-protocol` + `lib.rs`) + JS test (`toPack`) |
| REQ-008 | Existing look, movement, oath, canvas map, and event feed behavior shall continue to pass. | `bin/gate.sh` (cargo test + node --test) |

## Locked-In Decisions
These are fixed by the ticket + prior architecture and must not be re-litigated
mid-pipeline. (Open choices for Phase 2 to settle are listed in the notes, not here.)

- **Reuse the ticket #17 resolver; do not duplicate distance math.** `talk`/`take`
  resolve through `awareness::resolve_target`/`perceive`, gating on
  `Proximity::is_interactable()` (interaction radius) rather than sight
  (Decision 036; `docs/spatial-awareness.md` Commands section already pre-commits
  this). The `look_at` handler (`lib.rs:618-636`) is the template to follow.
- **Server-authoritative, additive, JSON-friendly snapshot.** Carried items surface
  through a NEW protocol field added to the snapshot DTOs, `#[serde(default,
  skip_serializing_if = …)]` so an empty pack is byte-identical to today's wire and
  old payloads still deserialize — the same additive pattern as `oath` and
  `room.contents` (Decisions 030/031/034/035). No canvas/drawing instructions.
- **Parser stays the pure forgiving-symbolic function** (Decision 002). Add typed
  `Command` variants alongside `Look`/`Swear`/`Confront`, verb case-folded, target
  text preserved exactly as `Look { target }` does. Unrecognized stays `Unknown`.
- **Minimal carried state.** Player-carried inventory is item *ids* only (names
  resolved at snapshot time from `world.items`), pickup-ordered. No quantities,
  stacks, equipment slots, weight, or persistence (all scoped out).
- **`take` mutates world placement; perception stays the single source of truth.**
  A taken item is removed from its containing room's `items` list (the engine owns
  `world` mutably) and added to player state; `awareness::perceive` then naturally
  drops it from `room.contents` — no parallel "removed items" set.
- **Deterministic, inline-tested.** Behavior is covered by inline `#[cfg(test)]`
  Rust tests reusing existing helpers (`proximity_engine`, `model_world`, `cmd`,
  `narrative_text`); the engine is deterministic (no RNG). JS tests are added only
  for the `toPack` rendering change (§7 / REQ-007).

## Linked Artifacts
- Design docs: `docs/spatial-awareness.md` (Commands section pre-commits talk/take);
  `docs/decisions.md` (002 parser, 025 grid, 030/031/034/035 wire, 036 awareness).
- Intake doc: none.
- Ticket doc: `docs/planning/tickets/open/TICKET-18-nearby-actions-v1-talk-and-take-commands.md`
- Forge ticket: `c78c03e0-067f-4876-ba6f-17b760b5d2ff` (#18)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS (design + test plan in notes) |
| 3 — Implement | PASS (code in place; fmt+check clean; existing suites green) |
| 3.5 — Inspect | PASS (3 critics; no real defects; clippy strict clean) |
| 4 — Validate | PASS (+29 tests; gate --fast 14/14 green; build clean) |
| 5 — Complete | PASS (docs updated; AAR+AD+PR captured; archived) |
