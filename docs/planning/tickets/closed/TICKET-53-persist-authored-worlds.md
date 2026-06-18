---
title: TICKET-53-persist-authored-worlds
status: done
ticket: 9d39d561-de36-494a-93b5-cb2b7ce81698
ticket_number: 53
type: feature
created: 2026-06-17
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
pipeline_spec: docs/planning/pipeline/active/WORK-persist-authored-worlds-v1.spec.md
---

# TICKET-53-persist-authored-worlds

## Summary

Region-authoring program slice **S1**: the studio **saves** an editable
`MapDocument` and can **reopen** it; the game **loads** an authored world at
startup (opt-in via config) through the existing `materialize()` path. Closes the
author → save → restart → play loop. No purge (that is S5).

## Why

The studio can paint and validate a map but **discards** it, and the game only
loads the compile-time-baked beginner TOML — so authored content never persists
or plays. S1 is the keystone of the region-authoring program: the CRUD UI (S2),
real-tile visuals (S3), and rebuilding Hollowmere in the tool (S5) are all
worthless until authored content survives and loads.

## EARS Requirements

Authoritative list lives in the pipeline spec; summary:

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the owner saves a map, the studio shall persist the `MapDocument` JSON in the owned dir and return its id. | cargo test |
| REQ-003 | The save/list/load endpoints shall be Editor-only (anon/Player denied). | cargo test |
| REQ-004 | Persistence shall reject path-unsafe ids and write only within the owned dir. | cargo test (traversal/symlink/reserved) |
| REQ-005 | With an authored-world path configured the server shall load+materialize it; else load beginner. | cargo test |
| REQ-006 | A malformed/invalid authored doc shall fail with a typed error, never panic. | cargo test |
| REQ-007 | Beginner stays the default and existing tests are unaffected. | existing suites green |

## Scope

- In: studio save/list/load endpoints for `MapDocument` (Editor-gated, loopback,
  path-safe writes reusing `FileSaveStore`); game runtime load of an authored
  world via `materialize()` behind config/env (untrusted-input posture); tests.
- Out: region/sub-region CRUD UI (S2); map visuals / retire flat-colors (S3);
  region-model enrichment (S4); content purge + replacement (S5); authoring of
  entities/items/oaths.

## Notes

- Forge ticket: 9d39d561-de36-494a-93b5-cb2b7ce81698 (#53)
- Related docs: docs/module-system.md, docs/technical-architecture.md,
  docs/decisions.md (058); precedent WORK-save-load-v1, TICKET-2.
- Promoted from intake: INTAKE-region-model-rethink-and-owner-authoring (program,
  slice S1).
- Active pipeline: docs/planning/pipeline/active/WORK-persist-authored-worlds-v1.spec.md
  (pipeline_id c47a48d1-7e6a-483c-bf33-16610ad29411).
- Locked: persist the EDITABLE MapDocument (not materialized TOML); beginner stays
  default; no purge this slice.
