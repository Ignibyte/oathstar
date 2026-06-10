---
title: TICKET-27-scoped-announcements-and-notification-delivery-v1
status: closed
ticket: 5e945705-2a54-4add-a7d2-cce9b60cefc4
ticket_number: 27
type: feature
created: 2026-06-10
closed: 2026-06-10
intake: docs/planning/intake/INTAKE-announcements-notifications-and-area-scopes.md
pipeline_spec: docs/planning/pipeline/completed/WORK-scoped-announcements-v1.spec.md
---

# TICKET-27-scoped-announcements-and-notification-delivery-v1

## Summary

Give the world a voice: a server-authoritative scoped announcement layer
delivered through the existing event lifecycle. The engine can emit an
announcement with a scope (world / region / subregion / room / radius), decide
deterministically whether the player's current location receives it, and the
feed renders what was delivered.

## Why

Tickets #22–#26 made combat a complete loop, but the world itself never speaks
— nothing tells the player about events in the scope they occupy. The intake
(`INTAKE-announcements-notifications-and-area-scopes`) captured the full
direction (boards, trays, region events, DM/LLM sources); this ticket is its
own "Candidate Future Tickets" item 2 — the notification API and scoped
delivery — shipped WITHOUT item 1's Area hierarchy, since every needed scope
(world/region/subregion/room) plus radius already exists in the world model
and the spatial-awareness distance system. Single-player now; the same
delivery decision generalizes to multiplayer fan-out later.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an announcement is emitted with world scope, the engine shall deliver it to the player regardless of current location, as a typed event on the existing stream. | Rust test |
| REQ-002 | When an announcement is emitted with a region, subregion, or room scope, the engine shall deliver it iff the player's current location lies within that scope, and shall emit nothing to the player otherwise. | Rust test (both arms) |
| REQ-003 | When an announcement is emitted with a radius scope around an origin, the engine shall deliver it iff the player's current cell is within the radius under the spatial-awareness distance model. | Rust test (both arms) |
| REQ-004 | The announcement event shall be additive on the existing protocol (snake_case typed kind, wire conventions preserved) and shall render in the event feed through the existing component path. | Rust/JS test |
| REQ-005 | The delivery decision shall be server-authoritative and deterministic — the client shall render delivered announcements without deciding receipt. | Rust test + review |
| REQ-006 | The beginner module shall author at least one real announcement emission site whose delivered outcome is reachable in play. | content/server test + smoke |
| REQ-007 | Existing combat (pulses, direct verbs, rewards), oath/boss confront, movement/look/talk/take, the Datastar feed, and the client build shall continue to pass. | gate |

## Scope

- In: a typed additive announcement event (the intake's candidate shape
  trimmed to v1 — severity/text plus whatever scope/source data design proves
  the client needs); an engine emit API with the deterministic scope-matching
  delivery decision; radius delivery reusing the spatial-awareness distance
  model as a separate delivery layer; feed rendering via the existing
  component path; one authored beginner emission site (the confront/oath
  bell-alarm is the lead candidate); exact tests; docs.
- Out: the Area scope hierarchy (intake item 1), bulletin boards, the
  notification tray UI, region event scheduler/hooks, persistence/read-state/
  expiry, multiplayer fan-out, DM/LLM sources, player speech verbs
  (say/yell), item/fixture acoustics.

## Notes

- Forge ticket: `5e945705-2a54-4add-a7d2-cce9b60cefc4` (#27)
- Related docs: `docs/event-lifecycle.md`, `docs/spatial-awareness.md`,
  `docs/protocol-and-output.md`, `docs/ui-design.md`, `docs/decisions.md`
- Promoted from intake:
  `docs/planning/intake/INTAKE-announcements-notifications-and-area-scopes.md`
- Active pipeline: `WORK-scoped-announcements-v1`
- Awareness answers "what can I perceive nearby?"; this layer answers "who
  should be told?" — keep them separate (intake note). Clients render
  notifications; they never decide receipt. Deterministic, no RNG.
- Demo caution: the Bell-Eater roost sits in the `old_bell_tower` region, so
  the bell-alarm demo's scope choice (world vs region vs radius) needs care,
  and the not-delivered case must be stageable
  (PR-claude-fixture-distinguishable-transitions-001).
