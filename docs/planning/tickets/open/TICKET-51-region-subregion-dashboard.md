---
title: TICKET-51-region-subregion-dashboard
status: open
ticket: 341c0863-3fdc-49cd-a438-18a4f5d827f2
ticket_number: 51
type: feature
created: 2026-06-17
intake: docs/planning/intake/INTAKE-studio-admin-and-world-model-program.md
pipeline_spec: docs/planning/pipeline/active/WORK-region-subregion-authoring-v1.spec.md
---

# TICKET-51-region-subregion-dashboard

## Summary

A studio dashboard (in the #49 nav shell) that lists regions and their sub-regions,
edits their attributes, and gives **each sub-region a link into the tile-map editor**
scoped to that sub-region.

## Why

Sub-regions already exist in the model (`SubregionDefinition { id, name, region }`;
rooms carry an optional `subregion`) but have **no editing surface**. The owner wants
to step back and edit region/sub-region information before returning to tile-map
polish, and to jump straight from a sub-region into its map. Build order 3 of 4.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an Editor opens the Regions section, the studio shall list every region with its sub-regions in a region→sub-region hierarchy. | render test |
| REQ-002 | When an Editor edits a region's or sub-region's attributes and saves, the studio shall persist the change. | test (needs storage) |
| REQ-003 | When an Editor creates or deletes a region or sub-region, the studio shall reflect it in the list. | test |
| REQ-004 | Each sub-region shall present a link to the tile-map editor scoped to that sub-region. | render test |

## Scope

- In: region→sub-region list/hierarchy; create/edit/delete regions + sub-regions;
  edit attributes (at minimum name + description); a sub-region→tile-editor link.
- Out: warp / cross-region transition authoring (arrives with #52); the full
  region-standing consequence system.

## Notes

- Forge ticket: #51 `341c0863-3fdc-49cd-a438-18a4f5d827f2`
- Build order: **3 of 4**. Depends on: #49 (nav) + #50 (theme) + the existing region
  model.
- Open questions (design): which attributes beyond name/description (region-standing
  defaults from `docs/region-standing.md`?); **how a sub-region maps to a tile-map
  document** — linking a sub-region to *its own* map implies map persistence/identity
  (paint S4 territory); **where edited region data is stored** (the studio has no
  persistence layer yet).
- Coordinates with #52: warp authoring is added to this dashboard when the world-model
  ticket lands.
- Promoted from intake: yes. Active pipeline: not yet.
