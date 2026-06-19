# TICKET-62 — Editor Regions tab: inline region create/rename/delete via a round-trip over the Rust CRUD

- **Forge ID:** `bedd26d3-a0e8-4998-837c-7001b5264dac` (#62)
- **Type:** feature · **Status:** open (pipeline `WORK-studio-editor-regions-tab-v1`)
- **Program:** pivot item ③ slice b of the studio-editable-world program (memory `studio-editable-world-pivot`)

## Why
The #61 editor rail's Regions tab is a "Coming soon" stub. Fill it with inline region editing for the
map being edited — without leaving the editor or duplicating the region CRUD that already lives on
`MapDocument`.

## What
A round-trip endpoint `POST /editor/maps/region-op` (Editor-gated) that dispatches `create`/`edit`/
`delete` to the existing `MapDocument` region CRUD against the catalog and returns the updated
document (or the typed `RegionEditError` as a `400`); the Regions tab renders the region list
client-side from the in-memory doc via a pure `editorRegionRows`, with an add-region form + per-row
rename/delete that swap in the returned doc. The in-memory doc stays authoritative (Save persists it).
Author id/name are escaped via `textContent` (client-rendered list).

## Acceptance
See `docs/planning/pipeline/active/WORK-studio-editor-regions-tab-v1.spec.md` (EARS REQ-001..008).

## Out of scope
Sub-region editing (slice ③b2); the Rooms tab / room-metadata (③c); map expansion (③d); curating the
palette.
