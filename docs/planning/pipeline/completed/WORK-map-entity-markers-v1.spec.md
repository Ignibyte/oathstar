---
pipeline_id: f0880b4c-3bbd-42a4-990d-93b2d265900e
title: WORK-map-entity-markers-v1
ticket: 9fe59e9c-776a-4147-9fdf-0b43aa6fe181
type: work
intake: docs/planning/intake/INTAKE-blank-colors-vertical-slice-city-forest-cave.md
notes: WORK-map-entity-markers-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-map-entity-markers-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Map Entity Markers v1 — hostiles and ground items render as
  colored dots on the map (S0 of the blank-colors vertical slice).
- **Scope:**
  - **In:** two additive per-room presence fields on `MapRoomSnapshot`,
    computed server-side in `Engine::map_snapshot` from LIVE placements
    (`Role::Hostile` entities; `room.items`) — Decision 041's
    server-computes-affordances principle; the visibility/fog rule
    (design decides; intake flagged the knowledge-model question);
    `toMapModel` cell flags; draw-plan marker fields that the seam
    ACTUALLY draws (PR-oathstar-render-plan-test-002); canvas-seam dots
    (ember hostile, gold item) over tiles, preserving glyph/ring/aria;
    deterministic engine + model + plan tests and the served marker
    lifecycle test; docs.
  - **Out:** enemy movement/AI; per-entity map identity; item counts;
    NPC/vendor markers; fog-of-war redesign; battle-modal changes;
    tileset/.tmx work.
- **Systems:** engine (snapshot), protocol (additive), ui (map model +
  plan + seam). No parser, storage, or content changes.

## Acceptance Criteria (EARS)
Verbatim from `TICKET-33` (forge `9fe59e9c-776a-4147-9fdf-0b43aa6fe181`).

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The map snapshot shall expose per-room hostile-presence and item-presence flags computed server-side from live placements. | Rust test |
| REQ-002 | When a room's hostile is defeated or an item is taken/dropped, the next snapshot's flags shall reflect the change. | Rust test |
| REQ-003 | The client map shall draw distinct colored markers for hostile-presence and item-presence on cells per the designed visibility rule. | node --test (plan) + smoke |
| REQ-004 | Marker rendering shall preserve existing tile, glyph, and aria behavior. | node --test |
| REQ-005 | The served fight shall show the marker lifecycle over the seam (present → defeat → cleared; drop → present → take → cleared). | server test |
| REQ-006 | Existing engine/client behavior and the gate shall continue to pass. | gate |

## Locked-In Decisions
Settled before design; not re-litigated mid-pipeline. Open *design*
choices are enumerated in the notes for Phase 2.

- **The server computes presence; the client never infers** (Decision
  041's principle): `has_hostiles` from `Role::Hostile` placements,
  `has_items` from `room.items` — both live state, so defeat/take/drop
  update them for free.
- **Additive protocol only.** Two fields on `MapRoomSnapshot`; existing
  payload fields byte-stable (the #18 additive-snapshot precedent).
- **Markers are presence, not identity** — a dot says "something
  hostile/lootable here", never who or how many (that's the Nearby
  panel's job in-room).
- **Ops carry only what the seam draws** (PR-oathstar-render-plan-test-002
  — twice-applied lesson); marker fields enter the plan only as drawn.
- **Hero stays as-is** — the current-room marker (ring + spawn tile) is
  already the "hero color"; this ticket adds enemies/loot only.

## Linked Artifacts
- Design docs: `docs/spatial-awareness.md`, `docs/map-system.md`,
  `docs/decisions.md` (041, 050)
- Intake doc: `docs/planning/intake/INTAKE-blank-colors-vertical-slice-city-forest-cave.md` (S0)
- Ticket doc: `docs/planning/tickets/open/TICKET-33-map-entity-markers-hostiles-items-as-colors.md`
- Forge ticket: `9fe59e9c-776a-4147-9fdf-0b43aa6fe181` (#33)
- AAR: (recorded in notes at Phase 1 closeout)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
