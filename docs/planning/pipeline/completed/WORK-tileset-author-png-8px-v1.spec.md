---
pipeline_id: 98fe19ea-0126-4f42-b928-cbd0d07aa120
title: WORK-tileset-author-png-8px-v1
ticket: 9117db55-c8d1-4e90-bb83-3fc7738da864
type: work
intake: docs/planning/intake/INTAKE-tileset-region-authoring-per-tile-metadata.md
notes: WORK-tileset-author-png-8px-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-tileset-author-png-8px-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Retire the generated-tileset era — author-PNG-sheet tiles at the
  sheet's declared size (8px-native). Ticket #46. The runway for the
  tile-painting editor backend.
- **Scope:**
  - **In:**
    1. **Retire the generated-tileset era.** Delete the untracked ElvGames
       experiment (`bin/import_elvgames_tilesets.py`,
       `public/tilesets/oathstar-elvgames-fantasy-16x16/`,
       `tests/elvgames-tileset.test.js`) AND the committed starter generator +
       sheet (`bin/generate_oathstar_tileset.py`,
       `public/tilesets/oathstar-starter-32x32/`). No generated tilesets or
       generator scripts remain.
    2. **Tile-size-agnostic model.** `SUPPORTED_TILE_SIZE` in
       `crates/oathstar-content/src/map_document.rs` becomes a supported set
       (8, 16, 32); the existing `tile_size = 8` rejection test flips to
       acceptance; an unsupported size still refuses with a typed error.
    3. **Author-PNG-sheet client loader.** Document a tile-sheet contract (grid
       PNG + declared tile size + name->cell map) + a tiny committed fixture
       sheet; swap `src/client/tileset.js` / `canvas-map.js` / `client-app.js`
       from the committed generated tileset to that loader; KEEP the flat-color
       fallback as the missing-art skin.
    4. **Record the pivot.** Supersede Decisions 035/050/054; close ticket #39
       won't-fix; note ticket #38 rework.
  - **Out (explicit):** real 8px art ingestion (fixture only this slice; wire
    real art later); the tile-painting editor backend (next pipeline); ticket
    #38 rework (noted, not done); any change to movement, 1-cell = 1-room
    (Decision 025), or render scale.
- **Systems:** assets (committed tilesets) | content (map document model, Rust)
  | ui (client renderer, JS) | docs/decisions

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a map document declares a `tile_size` of 8, 16, or 32, the content model shall validate and materialize it (previously only 16 was accepted). | Rust test — 8 and 32 pass; the former `tile_size = 8` rejection case now asserts acceptance |
| REQ-002 | When a map document declares a `tile_size` outside the supported set, the content model shall refuse it with a typed `UnsupportedTileSize` error naming the offending size. | Rust test — an unsupported size (e.g. 7) returns the typed error |
| REQ-003 | After this work, the repository shall contain no committed tileset assets and no tileset-generator scripts. | Review + path-absence check at validate; gate green |
| REQ-004 | When the client is given a tile-sheet descriptor conforming to the documented contract (PNG + declared tile size + name->cell map), it shall draw map cells from that sheet. | node --test against a committed fixture sheet |
| REQ-005 | While no conforming sheet is available, or a referenced tile name is absent from it, the client shall render the flat-color fallback for the affected cells. | node --test — fallback path |
| REQ-006 | The full gate (`bin/gate.sh`) shall pass with the generated-tileset assets and their tests removed and no dangling references remaining. | `bin/gate.sh` FULL green at validate |
| REQ-007 | The project record shall mark Decisions 035, 050, and 054 superseded by the author-PNG-sheet contract, and ticket #39 closed won't-fix. | doc check (`docs/decisions.md`) + forge `architecture-decision-record` + `ticket-close` at Complete |

## Locked-In Decisions
- **Author PNG sheets are the contract; flat color is the skin** — inverts
  Decision 054; supersedes 035 (32px renderer) + 050 (committed generated
  tileset).
- **Flat-color fallback render path is KEPT** as the missing-art skin —
  Decision 050's fallback survives as the graceful degrade.
- **Real art ingestion is DEFERRED** — build against the documented sheet
  contract + a tiny committed fixture sheet; no real 8px art is required to
  ship this slice.
- **Decision 025 (1 cell = 1 room, cardinal movement) and render scale are
  UNCHANGED** — `tile_size` is the source/art unit only, independent of how
  big a cell is drawn (editor/game render scale stays its own knob).
- **No Tiled** — the first-party studio editor (#43–45 + the upcoming painting
  backend) is the authoring surface; #39 closed won't-fix.
- **Name-keyed tile resolution stays the mechanism** — the author sheet supplies
  the name->cell map instead of a generated JSON twin.
- **Deletion is paired with reference + test cleanup** so the gate stays green
  (heeds forge failure `524538ee`: removing committed/decision-backed artifacts
  must leave no dangling references).

## Linked Artifacts
- Design docs: `docs/decisions.md` (supersede 035/050/054; 025 unchanged),
  `docs/map-system.md` + `docs/ui-design.md` (already state configurable
  8/16/32 tiles), `docs/technical-architecture.md` (asset/render path)
- Intake doc: `docs/planning/intake/INTAKE-tileset-region-authoring-per-tile-metadata.md`
  (its Tiled + blank-colors portions are superseded by this pivot)
- Ticket doc: `docs/planning/tickets/open/TICKET-46-retire-generated-tileset-era-author-png-8px.md`
- Forge ticket: 9117db55-c8d1-4e90-bb83-3fc7738da864 (#46)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
