---
pipeline_id: 795bb650-4161-484d-b480-9fa3e46dd7c8
title: WORK-scoped-announcements-v1
ticket: 5e945705-2a54-4add-a7d2-cce9b60cefc4
type: work
intake: docs/planning/intake/INTAKE-announcements-notifications-and-area-scopes.md
notes: WORK-scoped-announcements-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-scoped-announcements-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Scoped Announcements & Notification Delivery v1 — the world gets
  a voice: a typed, additive announcement event delivered through the
  existing event lifecycle, with a deterministic server-authoritative
  scope-matching decision (world / region / subregion / room / radius — the
  radius via the spatial-awareness distance model), an engine emit API, feed
  rendering through the existing component path, and an authored beginner
  emission site.
- **Scope:**
  - **In:** the typed announcement event (the intake's candidate shape
    trimmed to v1: severity + text, plus whatever scope/source data design
    proves the client needs); `AnnouncementScope`-style scope matching
    against the player's current location with the radius case reusing the
    spatial-awareness Chebyshev model (a SEPARATE delivery layer — awareness
    stays "what can I perceive?", this is "who should be told?"); the
    single-player delivery decision (the engine emits the event only when
    the player's location matches; clients render, never decide receipt);
    Datastar/feed rendering via the existing component path; ≥1 authored
    beginner emission site whose delivered outcome is reachable in play;
    exact Rust (+ JS where the render path needs it) tests; docs.
  - **Out:** the Area scope hierarchy (intake item 1 — a later ticket),
    bulletin boards, the notification tray UI, region event
    scheduler/hooks, persistence/read-state/expiry, multiplayer fan-out,
    DM/LLM sources, player speech verbs (say/yell), item/fixture acoustics.
- **Systems:** engine, protocol, datastar, content, ui(light).

## Acceptance Criteria (EARS)
Verbatim from `TICKET-27` (forge `5e945705-2a54-4add-a7d2-cce9b60cefc4`).

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an announcement is emitted with world scope, the engine shall deliver it to the player regardless of current location, as a typed event on the existing stream. | Rust test |
| REQ-002 | When an announcement is emitted with a region, subregion, or room scope, the engine shall deliver it iff the player's current location lies within that scope, and shall emit nothing to the player otherwise. | Rust test (both arms) |
| REQ-003 | When an announcement is emitted with a radius scope around an origin, the engine shall deliver it iff the player's current cell is within the radius under the spatial-awareness distance model. | Rust test (both arms) |
| REQ-004 | The announcement event shall be additive on the existing protocol (snake_case typed kind, wire conventions preserved) and shall render in the event feed through the existing component path. | Rust/JS test |
| REQ-005 | The delivery decision shall be server-authoritative and deterministic — the client shall render delivered announcements without deciding receipt. | Rust test + review |
| REQ-006 | The beginner module shall author at least one real announcement emission site whose delivered outcome is reachable in play. | content/server test + smoke |
| REQ-007 | Existing combat (pulses, direct verbs, rewards), oath/boss confront, movement/look/talk/take, the Datastar feed, and the client build shall continue to pass. | gate |

## Locked-In Decisions
Settled before design; not re-litigated mid-pipeline. The open *design*
choices they leave are enumerated in the notes for Phase 2 to settle.

- **Ride the existing event lifecycle — no parallel transport.** One typed,
  additive `GameEventKind` variant on the existing channels/stream; the
  Datastar feed and the JSON client consume it exactly like every other
  event (Decisions 028/031/034 wire discipline). No new endpoint, no tray.
- **The scope set is fixed: world / region / subregion / room / radius.**
  NO Area scope (intake item 1 stays deferred — the intake's own direction).
  All four named scopes already exist in the world model; radius reuses the
  spatial-awareness Chebyshev distance model but lives in a separate
  delivery layer (the intake's perceive-vs-receive split is binding).
- **Delivery is the engine's decision, made at emission.** Single-player
  reading: the engine emits the announcement event only when the player's
  current location matches the scope — nothing scope-filtered ever reaches
  a client to discard (clients render, never decide receipt — REQ-005).
  The decision function must be written so multiplayer fan-out later swaps
  "the player's location" for "each session's location" without
  restructuring.
- **Deterministic, no RNG.** Authored or trigger-driven emissions only;
  same input world + commands ⇒ same announcements.
- **Demo emission site is authored, real, and both-arms testable.** Design
  picks the trigger (the confront/oath bell-alarm is the lead candidate —
  noting the roost lives in `old_bell_tower`, so the scope choice must make
  the delivered case reachable in play) and the not-delivered case must be
  stageable per PR-claude-fixture-distinguishable-transitions-001.
- **Standing quality disciplines.** Additive protocol; every new
  string/variant arm enumerated with exact-string tests
  (PR-claude-enumerate-variant-string-arms-001); mutation killers live in
  the owning crate (PR-claude-package-scope-mutation-001); no unreachable
  defensive arms; assert by value.
- **Process bounds (owner-set).** Validate runs `cargo test --workspace`,
  `node --test`, `npm run build`, `./bin/gate.sh --fast`; the FULL gate and
  `/commit` are owner-gated — STOP after Phase 5 (complete + archive).
  Untracked `assets/tilesets/` + `bin/generate_oathstar_tileset.py` are
  Codex-owned, untouchable.

## Linked Artifacts
- Design docs: `docs/event-lifecycle.md`, `docs/spatial-awareness.md`,
  `docs/protocol-and-output.md`, `docs/ui-design.md`, `docs/decisions.md`
- Intake doc: `docs/planning/intake/INTAKE-announcements-notifications-and-area-scopes.md`
- Ticket doc: `docs/planning/tickets/closed/TICKET-27-scoped-announcements-and-notification-delivery-v1.md`
- Forge ticket: `5e945705-2a54-4add-a7d2-cce9b60cefc4` (#27)
- AAR: `63e0fd3e-173b-4f20-b778-aab48eef9540`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
