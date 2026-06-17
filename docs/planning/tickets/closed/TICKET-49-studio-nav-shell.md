---
title: TICKET-49-studio-nav-shell
status: closed
ticket: 7a030671-ba5a-4a08-b257-aadb69528115
ticket_number: 49
type: feature
created: 2026-06-17
closed: 2026-06-17
intake: docs/planning/intake/INTAKE-studio-admin-and-world-model-program.md
pipeline_spec: docs/planning/pipeline/completed/WORK-studio-nav-shell-v1.spec.md
---

# TICKET-49-studio-nav-shell

## Summary

Add a persistent navigation shell to `oathstar-studio` (Maps · Regions · Items ·
Enemies · Game Settings) so it grows from a single map editor into a multi-section
content tool. The existing dashboard and the map editor are reparented under the nav;
unbuilt sections ship as "Coming soon" stubs.

## Why

The studio is currently just the map editor. The region dashboard (#51), the future
item/enemy/settings editors, and the UI re-skin (#50) all need a shared shell to live
in. This is the **enabling first step** (build order 1 of 4) of the pre-tilemap
program — small, and it unblocks everything after it.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an Editor loads any studio page, the studio shall render a persistent navigation listing Maps, Regions, Items, Enemies, and Game Settings. | studio render test + smoke |
| REQ-002 | When an Editor selects Maps, the studio shall route to the existing `/editor`. | route test |
| REQ-003 | When an Editor selects a not-yet-built section, the studio shall render a stub ("Coming soon") page rather than a 404. | route test |
| REQ-004 | The navigation and its section routes shall be Editor-gated (a missing/non-editor session redirects to `/login`). | auth test |

## Scope

- In: a shared server-rendered nav component (`render.rs`); section landing routes
  (stubs ok); reparent the dashboard + `/editor` under the nav; Editor-gated like the
  rest of the studio.
- Out: the actual item/enemy/settings editors (later tickets); the region dashboard
  (#51); the UI re-skin (#50, applied after this).

## Notes

- Forge ticket: #49 `7a030671-ba5a-4a08-b257-aadb69528115`
- Build order: **1 of 4** (nav → UI → regions → world model). Depends on: nothing.
- Build it so #50 can re-skin it and #51 can plug into it.
- Related docs: `INTAKE-studio-admin-and-world-model-program.md`; current studio
  routes in `crates/oathstar-studio/src/main.rs`.
- Promoted from intake: yes. Active pipeline: not yet.
