---
pipeline_id: a326c1fc-c156-446e-a668-d8e610874459
title: WORK-combat-encounter-v1
ticket: 4167bcb6-c807-4c2c-8ed6-311b7b3ae20b
type: work
intake:
notes: WORK-combat-encounter-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-combat-encounter-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Combat Encounter v1 — a deterministic, server-authoritative, command-driven battle loop with a client battle modal, built on the typed `Role`/`CombatProfile` foundation (#21), preserving the confront/oath/boss flow.
- **Scope:**
  - **In:** combat encounter state on `GameState` (additive); an `attack`/`strike`/`fight` command path; deterministic HP/damage (player strike + hostile return action); win/loss resolution that clears combat state; clean refusal when combat is not active or not allowed in the current region/subregion; additive typed combat events on the existing `EventChannel::Combat` + an additive combat sub-state on `GameSnapshot`; a client **battle modal** (left = battle log, right = participant state, multi-party-extensible) opened on combat start, closed/collapsed on end with a compact summary left in the feed; one authored beginner hostile road encounter; Rust + JS tests for every EARS REQ incl. modal behavior; docs.
  - **Out:** full skills/classes, equipment bonuses, AI tactics, loot tables, death penalties, grind economy, multiplayer turns, real-time combat pulses (the Decision 023 pulse loop is deferred — v1 is turn-on-command).
- **Systems:** combat, engine, parser, protocol, content, ui.

## Acceptance Criteria (EARS)
Verbatim from `TICKET-22` (forge `4167bcb6-c807-4c2c-8ed6-311b7b3ae20b`).

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player enters or targets a hostile combatant in a combat-enabled area, the engine shall be able to start a combat encounter state. | Rust test |
| REQ-002 | When the player uses `attack`/`strike`/`fight` during combat, the engine shall resolve deterministic damage, update HP, and emit combat-channel events. | Rust test |
| REQ-003 | When the hostile actor acts, the engine shall resolve deterministic return damage or a simple authored action. | Rust test |
| REQ-004 | When either side reaches zero HP, the engine shall end combat with a win/loss outcome and clear combat state. | Rust test |
| REQ-005 | When combat is not active or not allowed in the current region/subregion, `attack` shall refuse cleanly without damaging entities. | Rust test |
| REQ-006 | The event feed shall receive combat events in the existing typed channel/component system so future collapsible combat logs can be built on top. | Rust/JS test |
| REQ-007 | Existing oath/boss flow shall remain playable; this ticket may add a small road encounter but shall not replace the Bell-Eater boss design wholesale. | gate / server smoke |
| REQ-008 | When combat starts in the client, the UI shall open a battle modal rather than only appending normal feed text. | JS/browser smoke |
| REQ-009 | While combat is active, the battle modal shall use a split layout: the left pane shows the battle text/event log, and the right pane shows battle participants with current state, so the layout can later support multiple allies and enemies. | JS/browser smoke |
| REQ-010 | When combat ends, the battle modal shall close or collapse and the event feed shall retain a compact combat summary that can later become an expandable/collapsible historical combat log. | JS/browser smoke |

## Locked-In Decisions
These are settled before design and must not be re-litigated mid-pipeline. The
open *design* choices they leave (combat-state shape, damage model, verb arity,
how "hostile" and "combat-enabled" are expressed, the loss outcome) are listed
in the notes for Phase 2 to settle.

- **Deterministic, server-authoritative, command-driven v1.** Turn-on-command — NOT the real-time pulse loop (Decision 023 pulses are deferred). No wall-clock / no nondeterminism; keep it deterministic so mutation (100% MSI) and regression tests stay strong (ticket note; Decision 023 determinism).
- **Build on #21; do not re-architect the foundation.** Reuse `Role::Combatant`/`Role::Boss` + `Entity::has_role`, `Entity.combat: Option<CombatProfile>`, and `PlayerState.hp/max_hp` (already present, currently unread by combat). Extending `CombatProfile` additively is allowed; replacing the role/contract model is not.
- **Additive protocol only.** New combat `GameEventKind` variants ride the existing `EventChannel::Combat` (+ `OutputComponent::CombatMessage`); a new optional combat sub-state attaches to `GameSnapshot` (`#[serde(default, skip_serializing_if = …)]`). Preserve the existing snapshot shape, the oath events (`OathSworn`/`OathFulfilled`), and the wire conventions: snake_case `type` tag on events, camelCase snapshot fields (Decisions 028/031).
- **Combat is gated (Decision 007 / `docs/combat-system.md`).** `attack` refuses cleanly when there is no live encounter or the current region/subregion/room is not combat-enabled, and refusal damages nothing (REQ-005). Combat is region/area-bound.
- **Preserve confront/oath/boss (REQ-007 / Decision 007).** The `confront` oath-resolution endpoint and the Bell-Eater boss design stay intact. #22 adds a NEW small hostile road encounter (the Ashen Road / wilds is the natural home) rather than rewiring the boss.
- **Battle modal is part of the v1 UX contract.** Mirror the existing `#room-modal` `<dialog>` pattern (`openRoomModal`/`showModal`). Left pane = battle log; right pane = participants modeled as a **list** so multi-party fits later (REQ-009). On end, the modal closes/collapses and a compact summary remains in the feed (REQ-010). Keep Datastar/SSE + the componentized feed intact (Decision 034 — combat events still stream to `#log` via the existing component path); JSON only for the participant/state snapshot (the canvas/state carve-out). Keep the pure view-model / thin glue-DOM split (Decision 032): a tested `toBattle`-style view-model + a thin client-app render/open/close seam.
- **Quality discipline.** Deterministic ⇒ every combat branch reachable and mutation-killable (assert by value, not `is_some`). Extract per-concern combat/parse/dispatch helpers to stay under clippy `too_many_lines = 100`, and run `cargo clippy --workspace --all-targets` during IMPLEMENT, not just `cargo check` (PR-claude-validator-length-001). No suppressions, no baselines, source-fix only (gate §0).
- **One shippable slice.** Engine + protocol + content + UI ship together per the owner's explicit bundling ("the modal is part of the v1 UX contract"); the combat-state shape and the modal are coupled, so they are not split. DESIGN sequences the work internally (engine/protocol core → snapshot → content encounter → UI modal).

## Linked Artifacts
- Design docs: `docs/combat-system.md`, `docs/ui-design.md`, `docs/protocol-and-output.md`, `docs/entity-model.md`, `docs/mechanics-and-systems.md`, `docs/decisions.md`
- Intake doc: none
- Ticket doc: `docs/planning/tickets/open/TICKET-22-combat-encounter-v1-fast-authored-battle-loop.md`
- Forge ticket: `4167bcb6-c807-4c2c-8ed6-311b7b3ae20b` (#22)
- AAR: `5e3cf138-c359-466b-90c0-301dcd5e2241`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
