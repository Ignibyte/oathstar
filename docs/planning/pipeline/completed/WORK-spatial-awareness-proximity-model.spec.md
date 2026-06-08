---
pipeline_id: 4a69706a-f5a9-4a42-81e7-ccf4782acf3f
title: WORK-spatial-awareness-proximity-model
ticket: 35ec2315-2823-462b-8a41-fbf3d03b3f4e
type: work
intake:
notes: WORK-spatial-awareness-proximity-model.notes.md
status: Phase 1 — Plan PASS; Phase 2 — Design PASS; Phase 3 — Implement PASS; Phase 3.5 — Inspect PASS; Phase 4 — Validate PASS; Phase 5 — Complete PASS
---

# WORK-spatial-awareness-proximity-model

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Spatial Awareness + Proximity Query Model (foundation) — discover
  entities/items by distance within a subregion/z-plane, not only the exact cell.
- **Scope:**
  - **In:**
    1. A reusable **proximity/awareness domain model** in `oathstar-core` (new
       module): a cell `Position` derived from a room's `(region, subregion, x,
       y, z)`; **action-specific radius config** (sight radius + interaction
       radius, shaped to extend to hearing/detection later); a region/subregion/
       z-plane-gated **distance query** over positioned entities/items; and a
       **structured awareness result** that classifies each perceived thing as
       `exact` / `visible-not-interactable` / `interactable`, with a
       `blocked`/reveal (line-of-sight) **placeholder** flag.
    2. **Additive JSON snapshot exposure** — extend the `oathstar-protocol`
       snapshot DTOs and the `oathstar-core` snapshot builder so the awareness/
       nearby data ships as JSON-friendly state, populating the existing client
       **Nearby panel** contract.
    3. A **proximity-aware target resolver** wired into the stubbed
       `look <target>` handler (exact cell first, then nearby within radius,
       alias/name-aware), preserving current exact-room behavior.
    4. **Tests** (Rust unit/`#[cfg(test)]`; a JS test if the client-consumed
       snapshot shape changes) covering radius math, region/subregion/z-plane
       boundaries, exact-vs-nearby resolution, and the blocked/reveal placeholder.
    5. **Docs** — add a spatial-awareness design doc explaining the model and how
       it underpins future rooms-as-areas, NPC/item proximity, combat aggro,
       stealth/noise, and map overlays.
  - **Out:** full combat aggro, stealth, sound propagation, pathfinding, final
    line-of-sight blockers, dialogue trees, shops, modals, multiplayer, DM
    controls *(ticket scope-out)*; **per-entity intra-room cell coordinates**
    (v1 is room/cell-granularity — entities inherit their room's position);
    **`talk`/`take` command parsing** (no `Command` enum variants exist yet — the
    resolver is built reusable so they adopt it later); any client
    renderer/canvas rewrite; tileset asset changes.
- **Systems:** engine (`oathstar-core`), protocol (`oathstar-protocol` snapshot
  DTOs), server (`oathstar-server` `/state` smoke), ui (client Nearby panel —
  `src/client/snapshot.js`, render-only), docs. **Server-authoritative; Rust
  emits structured JSON only — no canvas drawing instructions** (Decisions 034/035).

## Acceptance Criteria (EARS)
| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the engine evaluates surroundings from an origin room, it shall return positioned entities/items within a configurable **sight radius**, computed over room/cell `(x,y,z)` coordinates and restricted to the origin's region + subregion + z-plane — not only the origin cell. | Rust unit tests |
| REQ-002 | When a candidate lies outside the sight radius, on a different subregion or z-plane, or is flagged by the blocked/reveal (line-of-sight) placeholder, the awareness query shall exclude it from the perceivable results. | Rust unit tests |
| REQ-003 | When a target is within sight radius but outside the **interaction radius**, the awareness result shall classify it as visible-but-not-interactable, distinct from interactable. | Rust unit tests |
| REQ-004 | When the player issues `look <target>`, the engine shall resolve the target through the proximity resolver (exact cell first, then nearby within radius, alias/name-aware) and report what is perceived, while the existing exact same-room behavior and all other commands keep working. | Rust unit tests / slice smoke |
| REQ-005 | When the server builds a `/state` snapshot, it shall expose the proximity/awareness data as **additive** JSON fields on the `oathstar-protocol` snapshot DTOs (existing `map`/`room` payload unchanged) and shall emit no canvas drawing instructions. | code review / serde + API smoke |
| REQ-006 | The proximity model's tests shall cover radius math, region/subregion/z-plane boundaries, exact-vs-nearby resolution, and the blocked/reveal placeholder. | Rust unit tests |
| REQ-007 | When this ships, the beginner slice's room title/description, navigation (move + room discovery), event feed, and canvas map shall continue to function unchanged. | gate / existing tests / browser smoke |
| REQ-008 | When the snapshot exposes nearby contents, the existing client Nearby panel shall render them through its current `toNearby` contract, with no regression to the other panels. | JS test / browser smoke |
| REQ-009 | The design docs shall be updated (new/extended spatial-awareness doc) to explain the proximity/awareness model and how it underpins future sight/interaction/hearing/detection/aggro/stealth radiuses and map overlays. | doc check |
| REQ-010 | When the ticket is complete, `cargo test --workspace`, `node --test` (if JS changed), `npm run build` (if client-consumed state shape changed), and `./bin/gate.sh --fast` shall pass. | command output |

## Locked-In Decisions
- **Room/cell-granularity proximity (v1).** Entities/items derive their position
  from their containing room's `(region, subregion, x, y, z)`; rooms ARE the grid
  cells (Decision 025; `docs/map-system.md`). No per-entity intra-room cell
  coordinates this ticket. *Grounded:* `Entity`/`Item`
  (`crates/oathstar-core/src/lib.rs:91-115`) carry no coordinates; placement is
  room-membership ID lists (`RoomDefinition.entities/items`, `lib.rs:52-55`),
  while rooms already carry `region/subregion/x/y/z` (`lib.rs:41-47`).
- **Awareness is gated by region + subregion + z-plane, then by radius.** Cross-z
  or cross-subregion things are never "nearby." Distance metric resolved in
  Design → **Chebyshev** (square radius; integer, deterministic), isolated in one
  fn for a future swap. Default radii sight=3, interaction=1 (see notes).
- **Reusable model in `oathstar-core` (new module).** Action-specific radiuses
  (sight, interaction — extensible to hearing/detection/aura) plus a structured
  awareness result (`exact` / `visible-not-interactable` / `interactable` +
  `blocked`/reveal placeholder). Not hardcoded into the Nearby panel or the
  snapshot builder (ticket: "prefer a foundation future systems can reuse").
- **Server-authoritative, JSON-friendly, additive snapshot — no canvas
  instructions.** New data is added to `oathstar-protocol` DTOs and produced by
  explicit engine snapshot calls (Decision 031 — stable wire split); the existing
  `map`/`room` JSON stays backward-compatible; Rust emits structured JSON only
  (Decisions 034/035, REQ-005). Core invariant validation stays at the
  construction boundary (Decision 030).
- **Light up the existing Nearby panel via its existing client contract.**
  Populate the snapshot to match the client's `toNearby` shape
  (`room.contents`/`actors`/`items`, `src/client/snapshot.js:72-87`) so the panel
  becomes data-driven with no client-logic rewrite; all other panels and the
  beginner slice are preserved (REQ-007/008).
- **`look <target>` is the concrete command proof; talk/take are future
  consumers.** Wire the stub `Look { target }` handler
  (`crates/oathstar-core/src/lib.rs:509-564`) to the new resolver. `talk`/`take`
  command *parsing* is out of scope (no `Command` enum variants today,
  `command.rs:54-76`); the resolver is reusable so they adopt it later.
- **Deterministic, inline-tested.** Radius/boundary/resolution/placeholder
  behavior covered by inline `#[cfg(test)]` Rust tests using the existing
  `model_world`/`entity`/`item` helpers; no RNG (the engine is deterministic). §7.

## Linked Artifacts
- Design docs: `docs/map-system.md`, `docs/ui-design.md`, `docs/entity-model.md`,
  `docs/decisions.md` (003 rooms-as-containers · 025 square grid · 030 core-boundary
  validation · 031 wire split · 034 JSON-for-renderer-data · 035 canvas); **new**
  `docs/spatial-awareness.md` (to add — REQ-009).
- Intake doc: none.
- Ticket doc: `docs/planning/tickets/open/TICKET-17-spatial-awareness-proximity-query-model.md`
- Forge ticket: `35ec2315-2823-462b-8a41-fbf3d03b3f4e` (#17)
- Forge AAR: `73d74d36-0726-4bbd-8e63-6d21e99519cd`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS (design + 22-row test plan in notes) |
| 3 — Implement | PASS (compiles; clippy strict clean; 122 existing tests green; tests authored in Phase 4) |
| 3.5 — Inspect | PASS (4 critics; 0 production defects; 1 doc carve-out; MSI test-hardening list for Phase 4) |
| 4 — Validate | PASS (127 rust + 27 js green; gate --fast GREEN; new code 100% MSI verified) |
| 5 — Complete | PASS (docs + Decision 036 + forge AAR/ADR captured; ticket closed; archived) |
