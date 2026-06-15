---
title: TICKET-42-admin-studio-shell-v1
status: open
ticket: 2ba66eaf-a7cb-499c-bf71-cf888f37bc08
ticket_number: 42
type: feature
created: 2026-06-13
intake: docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md
pipeline_spec: docs/planning/pipeline/active/WORK-studio-sidecar-and-auth-lib-v1.spec.md
---

# TICKET-42-admin-studio-shell-v1

## Summary

Create the authenticated `/admin` and `/admin/editor` shell surfaces for
Oathstar Studio without building the full map editor yet.

## Why

**Reframed (2026-06-14):** the studio is now a **separate Rust sidecar**
(`oathstar-studio`, loopback-bound) rather than living in the game's web app —
the owner's architecture decision (amends Decision 056). A narrow first slice
(this ticket) stands up the sidecar shell + a shared `oathstar-auth` lib + owner
login, proving routing, auth-gating, session/cookie, and separation from the
player game before any editor or map-painting logic lands (#43).

## EARS Requirements (candidate — finalize at /pipeline:plan)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an unauthenticated user requests `/admin` or `/admin/editor`, the server shall refuse or redirect according to the chosen auth pattern. | server/browser test |
| REQ-002 | When an authenticated user with the required role requests `/admin`, the server shall render an admin shell with server/world status placeholders. | server/browser test |
| REQ-003 | When an authenticated user with the required role requests `/admin/editor`, the server shall render an editor shell with reserved regions for canvas, palette, inspector, validation, and actions. | server/browser test |
| REQ-004 | The player client route shall remain command-first and shall not load editor-only UI or behavior. | build/browser test |
| REQ-005 | The shell shall use the existing first-party frontend direction: static assets plus Datastar/SSE where useful, JSON only for structured editor data. | review + build |

## Scope

- In: protected routes, shell markup/styles, minimal status placeholders,
  navigation between player/admin/editor, docs.
- Out: map document model, painting tools, draft save APIs, validation/publish,
  production auth UI, collaborative editing, DM controls.

## Notes

- Forge ticket: 2ba66eaf-a7cb-499c-bf71-cf888f37bc08 (#42).
- Pipeline: WORK-studio-sidecar-and-auth-lib-v1 (reframed to the sidecar).
- Depends on #41 (committed 7551b02) — its auth lifts into the shared `oathstar-auth` crate.
- The goal is a real protected surface, not the editor itself.
