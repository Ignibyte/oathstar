---
pipeline_id: b4d31855-789a-427c-9265-a8bb1eed9256
title: WORK-map-real-tiles-default-v1
ticket: 5bafbae1-d347-464d-8a70-2525e3e6c000
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-map-real-tiles-default-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-map-real-tiles-default-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Real tiles on by default in the game map (S3 — map visuals, **slice 1**).
  The owner-confirmed next step of the region-authoring program ("drop the solid/flat
  tiles — paint with real tiles"), after S1 persistence (#53) and S2 authoring (#51).
- **The finding that shapes this slice:** the by-kind **real-tile render path already
  exists end to end** — `canvas-map.js` `toDrawPlan(model, tileset)` computes a per-cell
  `sprite` rect (`kindTileRects`), `client-app.js` `drawMapCanvas` calls `ctx.drawImage`
  for sprites with a flat-color `fillRect` **fallback**, the committed sheet
  `public/tilesets/arctic.{png,json}` carries the four kind tiles
  (`shadow_void`/`stone_floor`/`wall_face`/`spawn_marker` == the four `cellKind`s), and
  it is served at `/tilesets/arctic.json` (Vite `public/` → dev, `dist/`, and Tauri all
  ship it). **The only reason the game shows flat colors is that the client's tileset URL
  defaults to `null`** unless the build-time `VITE_OATHSTAR_TILESET` env var is baked
  (`client-app.js` `resolveAuthorTilesetUrl`). So slice 1 simply **flips real tiles on by
  default**.
- **Scope:**
  - **In:** a pure, node-tested helper (in `src/client/tileset.js`) that resolves the map
    tileset URL to the committed default `/tilesets/arctic.json` when no
    `VITE_OATHSTAR_TILESET` override is set (override still wins; blank == unset); wire
    `client-app.js` `resolveAuthorTilesetUrl` to it. The flat-color fallback stays for
    robustness (a failed fetch keeps today's behavior).
  - **Out (explicit, → follow-on slices):**
    - **S3.2** — per-subregion/per-room floor **variety** over the wire (a serde-additive
      floor-tile-name on `MapRoomSnapshot`; client maps name→sprite — the resurrected #38
      idea with real tiles). Needs a protocol field; not here.
    - **S3.3** — full authored `MapDocument` **layer paint** over the wire (the editor's
      exact per-cell paint). Bigger; not here.
    - A **game-server (`oathstar-server`) tileset route** — not needed; the SPA host serves
      `public/`. No protocol/engine/server change in this slice.
    - Changing the sheet art, the studio, or the `cellKind`→name contract.
- **Systems:** game client only — `src/client/tileset.js` (pure helper) + `src/client-app.js`
  (one-line wiring) + tests (`node --test`). **No Rust change.**

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When no `VITE_OATHSTAR_TILESET` override is set (unset or blank), the client shall resolve the map tileset URL to the committed default `/tilesets/arctic.json`. | node --test (the pure resolver returns the default for `undefined`/`""`/whitespace) |
| REQ-002 | When a `VITE_OATHSTAR_TILESET` override is set, the client shall use that override URL instead of the default. | node --test (the resolver returns the trimmed override) |
| REQ-003 | When a tileset is provided to the draw plan, the map shall render each cell as its kind's sprite, and when none is provided it shall fall back to the flat-color fill. | node --test (`toDrawPlan` with a tileset → ops carry kind sprite rects; without → `sprite:null`/flat — existing path, regression-guarded) |
| REQ-004 | The full gate shall stay green. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **Slice 1 = delivery default only.** The by-kind render path + flat fallback already
  exist; this slice adds **no render code** — it points the client at the committed sheet
  by default.
- **JS-only — no protocol/engine/server change.** The origin-relative `/tilesets/arctic.json`
  is already served by the SPA host in every target (Vite dev, static `dist/`, Tauri bundle).
- **Default sheet = the committed `arctic` sheet** (its four named tiles are exactly the four
  `cellKind`s; `arctic.json` validates under the existing tileset contract).
- **Pure resolver in `src/client/tileset.js`** (node-testable, like `kindTileRects`);
  `client-app.js` `resolveAuthorTilesetUrl` becomes a thin call into it (the browser-entry
  wiring is the reviewed seam, not unit-tested — mirrors the glue seams).
- **No regression risk:** worst case (the sheet 404s) is the existing flat-color fallback.
- **Branch off `main`** (`f56cc48`). The stashed online-first WIP (`stash@{0}`) must not be swept.
- **Design (Phase 2) decides:** the exact helper name/signature and whether the default URL is
  a shared `const` in `tileset.js`; confirms the origin-relative path resolves in Tauri.

## Linked Artifacts
- Design docs: `docs/tileset-contract.md` (the sheet contract), `docs/map-system.md`
  (the map render), `docs/decisions.md` (#32 name-resolution). Design re-reads via Explore.
- Intake: `docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md` (S3),
  `docs/planning/intake/INTAKE-tileset-region-authoring-per-tile-metadata.md` (#32 "swap art
  under the same names is free").
- Ticket doc: `docs/planning/tickets/open/TICKET-54-map-real-tiles.md`
- Forge ticket: `5bafbae1-d347-464d-8a70-2525e3e6c000` (#54). Supersedes #38/#40 (shelved).
- Builds on the merged region program (S1 #53, S2 #51); the ticket #32 sprite-tile path.

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
