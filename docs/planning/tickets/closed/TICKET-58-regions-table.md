# TICKET-58 — Regions dashboard: a searchable, sortable table

- **Forge ID:** `76bcc9e4-3205-4118-babd-15d114171bda` (#58)
- **Type:** feature · **Status:** open (pipeline `WORK-regions-table-v1`)
- **Program:** item ③ (LAST) of the owner's 2026-06-19 authoring-loop plan (memory `studio-authoring-next-phase`)

## Why
The `/regions` dashboard lists maps as bare `.panel` cards with unstyled links. The owner wants
a proper **table with search + sort**.

## What
Render the maps as a semantic `<table>` (Title, Id, Regions, Sub-regions, action) with a labeled
search input and sortable `<th>` (aria-sort). Search/sort are **client-side**: a new pure
node-testable `static/regions-table.js` (`filterRows` + `sortRows`) + a thin inline `REGIONS_GLUE`
seam that filters/reorders the server-rendered rows — mirroring `editor-canvas.js` + `EDITOR_GLUE`.
Table CSS in `studio.css` reusing the #50 theme tokens; the empty-state prompt stays.

## Acceptance
See `docs/planning/pipeline/active/WORK-regions-table-v1.spec.md` (EARS REQ-001..005).

## Out of scope
The `/regions/{id}` region editor's lists (follow-on); pagination; column show/hide; persisting
sort/filter; server-side search.
