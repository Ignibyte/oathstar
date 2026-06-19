# TICKET-54 — Map visuals: paint the game map with real tiles (retire flat MAP_PALETTE colors)

- **Forge ID:** `5bafbae1-d347-464d-8a70-2525e3e6c000` (#54)
- **Type:** feature
- **Status:** open (slice 1 in pipeline `WORK-map-real-tiles-default-v1`)
- **Program:** S3 of `INTAKE-region-model-rethink-and-owner-authoring` (after S1 #53, S2 #51)
- **Supersedes:** the blank-colors map direction — #38 (per-room colors), #40 (W1) are shelved.

## Why
Owner-confirmed (2026-06-17): "drop the solid/flat tiles — paint with real tiles." The game
map currently renders flat `MAP_PALETTE` colors; the owner wants real tile art.

## What (the program, sliced)
Recon (2026-06-18) found the by-kind real-tile render path **already exists** (the client's
`toDrawPlan`→`sprite`→`drawImage` with a flat fallback; the committed `arctic` sheet served at
`/tilesets/arctic.json`); the only reason the game is flat is that the client's tileset URL
defaults to off.

- **Slice 1 (`WORK-map-real-tiles-default-v1`):** flip real tiles **on by default** — default
  the map tileset to `/tilesets/arctic.json` when no `VITE_OATHSTAR_TILESET` override is set,
  via a pure testable helper; keep the override + the flat fallback. JS-only, no protocol change.
- **Slice 2 (S3.2):** per-subregion/per-room floor **variety** over the wire — a serde-additive
  floor-tile-name on `MapRoomSnapshot`; client maps name→sprite (the #38 idea, with real tiles).
- **Slice 3 (S3.3):** full authored `MapDocument` **layer paint** over the wire (the editor's
  exact per-cell paint).

## Acceptance (slice 1)
See `docs/planning/pipeline/active/WORK-map-real-tiles-default-v1.spec.md` (EARS REQ-001..004):
the client defaults to the committed sheet (real tiles by default), the override wins, and the
sprite/flat render path is regression-guarded; full gate green.

## Out of scope (this ticket's later slices, not slice 1)
A game-server tileset route (not needed — the SPA host serves `public/`); changing the sheet
art; the studio editor (already renders real tiles); S4 model enrichment; S5 content purge.
