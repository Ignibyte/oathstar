---
title: TICKET-41-auth-session-and-role-boundary-v1
status: closed
ticket: f6a3d20a-1888-422f-a1a5-0fcb69b47633
ticket_number: 41
type: feature
created: 2026-06-13
intake: docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md
pipeline_spec: docs/planning/pipeline/active/WORK-auth-session-and-role-boundary-v1.spec.md
---

# TICKET-41-auth-session-and-role-boundary-v1

## Summary

Add the first server-side authentication/session boundary and role model so
future online play and `/admin/editor` APIs have a real permission seam.

## Why

Oathstar is moving online-first. The current server has a clean REST/SSE shape
but no identity, session, or role concept. Before admin/editor tools exist, the
server needs a small explicit boundary that can refuse privileged requests
without relying on hidden frontend links.

## EARS Requirements (candidate — finalize at /pipeline:plan)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a request has no valid session, protected admin/editor handlers shall refuse it with a typed unauthorized response. | server test |
| REQ-002 | When a request has a valid session but lacks the required role, protected admin/editor handlers shall refuse it with a typed forbidden response. | server test |
| REQ-003 | When a request has a valid session with the required role, protected admin/editor handlers shall receive an authenticated principal containing id/name/roles. | server test |
| REQ-004 | The local development server shall have a deterministic way to seed or bypass an owner/editor session without weakening production defaults. | server test + docs |
| REQ-005 | Existing player endpoints (`/state`, `/command`, `/events`, `/events/json`, `/events/datastar`) shall continue to work in local development. | cargo/server test |

## Scope

- In: session/principal DTOs, role enum or role set, middleware/extractor/helper
  for protected handlers, dev owner strategy, docs.
- Out: production password storage, OAuth, registration, email, donations,
  database migration, multiplayer player identity in engine state, admin UI.

## Notes

- Forge ticket: pending.
- Related decision: Decision 056.
- Keep the first version small enough to support protected routes before
  committing to a full account system.
- Do not make frontend visibility the permission boundary.
