---
pipeline_id: 7a4b2f0d-af87-4973-ad8a-a88ebead726c
title: WORK-world-model-2d-movement-warps-v1
ticket: 32db0cc9-0176-45a4-a599-6a6a37ff8c18
type: work
intake: docs/planning/intake/INTAKE-studio-admin-and-world-model-program.md
notes: WORK-world-model-2d-movement-warps-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-world-model-2d-movement-warps-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** The 2D world model + region warps (ticket #52) — remove up/down,
  make movement/rooms 2D (tile layers keep `z` for visuals only), and turn a
  cardinal exit that crosses a region/sub-region boundary into a **warp** (a
  transition into another region). Migrate the beginner world's up/down to cardinal
  exits + warps. **Amends locked Decision 025.** The deepest, last slice of the
  pre-tilemap program — an ENGINE + content change.
- **Scope:**
  - **In:**
    1. **Drop Up/Down** — remove from the `Direction` enum + aliases (`u`/`d`) +
       labels + `as_str` (`oathstar-core/src/command.rs`) and from `src/world.js`
       (`directionAliases`/`directionLabels`). Movement is cardinal-only (N/S/E/W).
    2. **2D movement/rooms** — rooms are addressed by `(x, y, region, subregion)`;
       the `floors` field + the room `z` drop out of movement semantics (exact
       field treatment — remove vs fix-to-0 — decided at design). Awareness/proximity
       (Decision 036) gates by region/sub-region only (z collapses). **Tile LAYER
       cells keep `z`** (visual stacking, #47/#48 — untouched).
    3. **Warp = a cross-boundary cardinal exit** (owner's "special exit" choice) —
       when a cardinal exit's target room is in a **different region or sub-region**,
       the engine moves the player there AND emits a region/sub-region **transition**
       event ("You enter The Old Bell Tower."). Exits stay `direction → room_id`;
       the warp-ness is the boundary crossing. Validation: a warp's target room must
       exist (the cross-region link is deliberate).
    4. **Migrate the beginner world** (`modules/beginner/rooms.toml`) — the 6 up/down
       exits become cardinal: the tower climb (`tower_foot↔tower_landing↔bell_eater_roost`)
       → N/S; `hollowmere_square↔bell_frame` → a free cardinal. The existing
       cross-region cardinal step `ashen_road --N--> tower_foot` becomes the
       region warp; `tower_landing --N--> bell_eater_roost` (tower→boss) a sub-region
       warp. World still materializes + validates; no orphaned exits.
    5. **Amend Decision 025** (cardinal-only + warps replace up/down) — via an
       `architecture-decision-record` + a `decisions.md` amendment (or a new
       decision; `decisions.md` carries uncommitted online-first WIP, so the safe
       mechanism is decided at design — do NOT sweep that WIP).
  - **Out (explicit):** the studio **warp-authoring UI** (editing warps in the
    region dashboard / tile editor — a later #52/#51 slice; warps are TOML-authored
    this slice); region-standing consequences; any 3D/floor rendering.
- **Systems:** engine (oathstar-core movement/directions) | content (map model +
  beginner world) | parser/ui (JS directions)

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The `Direction` set shall not include Up or Down — the Rust enum (+ `u`/`d` aliases + labels) and the JS `directionAliases`/`directionLabels` resolve only N/S/E/W. | Rust unit test (`"up"`/`"u"` no longer parse; enum is 4 variants) + JS `node --test` |
| REQ-002 | When a player moves a cardinal direction whose exit targets a room in a **different region or sub-region**, the engine shall move them to that room and emit a transition event naming the region/sub-region entered. | engine test (cross-region move → moved + transition event) |
| REQ-003 | Movement and rooms shall be 2D — no up/down exits and no floors-for-travel; tile **layer** cells shall retain `z`. | model tests (RoomDefinition/MapDocument 2D; a layer cell keeps z) |
| REQ-004 | The map document shall accept a warp (a cross-boundary cardinal exit) only when its target room exists. | validation test (dangling warp refused; valid warp accepted) |
| REQ-005 | The beginner world shall materialize and validate with the former up/down traversal expressed as cardinal exits + warps, with no orphaned exit. | beginner-world materialize/validate test + a traversal test (climb the tower via cardinal/warp) |
| REQ-006 | Decision 025 shall be amended to record cardinal-only movement + region warps replacing up/down. | doc/AD check |
| REQ-007 | The full gate shall stay green with mutation 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **Owner forks (this session):** (1) **2D + warps together** in this slice; (2) a
  warp is a **special cross-boundary cardinal exit** (not a distinct entrance
  object).
- **Already decided (program):** movement/rooms 2D; tile **layers keep `z`** for
  visuals only.
- **The cross-region cardinal exit IS the warp** — no separate "warp" field on the
  exit; the engine detects the region/sub-region change at move time and emits the
  transition. (Keeps the exit model `direction → room_id` unchanged.)
- **Decision 025 is amended** (owner-sanctioned) — the amendment mechanism avoids
  the online-first WIP entanglement in `decisions.md` (decided at design).
- **Warp authoring stays manual (TOML) this slice** — the studio warp-authoring UI
  is deferred.
- **Heed** the established render/test rules where the JS/engine tests touch them;
  the engine's ~527 workspace tests + the beginner fixtures will move — expect to
  update direction/movement tests + the world fixtures.

## Linked Artifacts
- Design docs: `docs/decisions.md` (Decision 025 — amended), `docs/map-system.md`
  (the world/region model), `docs/region-standing.md`, `docs/game-overview.md`?
  (movement) — Design re-reads via Explore.
- Intake doc: `docs/planning/intake/INTAKE-studio-admin-and-world-model-program.md`
- Ticket doc: `docs/planning/tickets/open/TICKET-52-world-model-2d-warps.md`
- Forge ticket: `32db0cc9-0176-45a4-a599-6a6a37ff8c18` (#52)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
