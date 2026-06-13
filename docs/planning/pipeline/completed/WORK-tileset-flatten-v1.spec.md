---
pipeline_id: 83f1d373-0da4-4535-afce-37d4aee33e8f
title: WORK-tileset-flatten-v1
ticket: edd292c5-a346-49aa-86a8-f60191f2a081
type: work
intake: docs/planning/intake/INTAKE-tileset-region-authoring-per-tile-metadata.md
notes: WORK-tileset-flatten-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-tileset-flatten-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Flatten the tileset — solid colors, lean names, blank-colors
  contract (ticket #36, step A of the blank-colors program)
- **Scope:**
  - **In:** rework `bin/generate_oathstar_tileset.py` to paint every tile
    as ONE uniform color from a named palette table; lean name set — keep
    the four load-bearing `KIND_TILE_NAMES`
    (`shadow_void`/`stone_floor`/`wall_face`/`spawn_marker`), drop the 32
    path/water connectivity variants, add the flat extras the slice needs
    (subregion floor variants: forest grass / road / cave rock; water;
    stairs up/down; exit marker); palette aligned with the flat-color
    spike concept and the shipped runtime marker colors; regenerate the
    committed `public/tilesets/oathstar-starter-16x16/` set
    (png/json/tsx/preview/README); update committed-asset tests
    (count/name cross-checks) and add the blank-colors pin (every
    committed tile uniform).
  - **Out:** client/renderer changes (zero — name-keyed contract holds),
    per-tile `description` metadata (the later metadata step), the .tmx
    importer (B), per-room tile names over the wire (C), hero/enemy/item
    tiles (entities stay runtime overlays per Decision 051), real art
    (ship-time swap).
- **Systems:** assets (committed tileset) | generator script | js tests

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The regenerated committed tileset shall contain the four load-bearing tile names with unchanged 16×16 tile geometry, and the client tileset module shall validate it with zero code changes. | node --test (existing committed-asset contract tests stay green, untouched client) |
| REQ-002 | Every tile in the committed sheet shall be one uniform color block (all pixels in the tile identical). | node --test blank-colors pin over the committed PNG (mechanism decided at design) |
| REQ-003 | The lean name set shall include the flat extras (forest/grass floor, road, cave rock floor, water, stairs up, stairs down, exit marker), each name unique and consistent across the png/json/tsx triplet. | node --test count + json↔tsx cross-checks |
| REQ-004 | When `bin/generate_oathstar_tileset.py` is re-run, it shall reproduce the committed asset bytes exactly (deterministic generation). | re-run + `git diff --exit-code` on the asset dir at validate |
| REQ-005 | While the flattened sheet is loaded, the existing map canvas draw-plan suites shall pass unchanged (the four kinds appear as their authored solid colors with no renderer edits). | node --test (canvas-map + client suites) |

## Locked-In Decisions
- `KIND_TILE_NAMES` and the client tileset/canvas modules are untouched —
  the name-keyed contract (Decision 050) is the whole mechanism.
- Lean name set: the 32 path/water connectivity variants are texture-era
  artifacts and are dropped; flat extras are named for step C to paint
  with.
- Entities (hero/enemies/items) stay runtime overlays (Decision 051) —
  no entity tiles in the sheet.
- Palette is authored in ONE table in the generator (single source);
  committed assets remain the artifact (no generation in the gate).
- Art returns at ship time as a pure asset swap; this sheet is the
  contract, not the look.

## Linked Artifacts
- Design docs: docs/decisions.md (050/051), docs/planning/intake/INTAKE-tileset-region-authoring-per-tile-metadata.md (resequenced step A), docs/planning/intake/INTAKE-blank-colors-vertical-slice-city-forest-cave.md
- Intake doc: docs/planning/intake/INTAKE-tileset-region-authoring-per-tile-metadata.md
- Ticket doc: docs/planning/tickets/open/TICKET-36-flatten-tileset-solid-colors-lean-names.md
- Forge ticket: edd292c5-a346-49aa-86a8-f60191f2a081 (#36)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
