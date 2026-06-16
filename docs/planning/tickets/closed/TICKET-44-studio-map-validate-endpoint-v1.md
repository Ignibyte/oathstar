---
title: TICKET-44-studio-map-validate-endpoint-v1
status: open
ticket: e7c7a0d4-db81-4b91-9cf8-4546ebdcd6c6
ticket_number: 44
type: feature
created: 2026-06-15
intake: docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md
pipeline_spec: docs/planning/pipeline/active/WORK-studio-map-validate-endpoint-v1.spec.md
---

# TICKET-44-studio-map-validate-endpoint-v1

## Summary

An owner/editor-gated JSON endpoint in the `oathstar-studio` sidecar that accepts a
posted `MapDocument`, runs `oathstar-content`'s validate + materialize against a
server-built `ContentCatalog`, and returns a typed JSON result — a compact summary on
success, a cell/ref-naming error on failure. Wires the studio (#42) to the map
document model (#43); the backend the `/admin/editor` canvas UI will call.

## Why

#43 defined the map document model but nothing serves it. Before the canvas editor
(intake section D) can save/validate drafts, the studio needs a server endpoint that
validates and materializes a posted document behind the auth boundary.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a caller without an Editor-granting session posts to the endpoint, the studio shall refuse without validating (no 2xx, no summary). | Rust handler test |
| REQ-002 | When an Editor posts a valid MapDocument, the endpoint shall respond 2xx with a JSON summary (`ok=true`, room count, region count, `start_room_id`). | Rust handler test |
| REQ-003 | When an Editor posts a MapDocument that fails validation, the endpoint shall respond with a typed JSON error (`ok=false`) that names the offending cell/reference. | Rust handler test |
| REQ-004 | When the request body is not a well-formed MapDocument, the endpoint shall refuse with a typed JSON error and never panic. | Rust handler test |
| REQ-005 | The endpoint response shall be renderer-agnostic JSON with no internal/engine leakage, and identical input shall yield identical output. | Rust test (serde shape + determinism) |

## Scope

- In: `oathstar-studio` → `oathstar-content` dependency; one Editor-gated POST route;
  MapDocument JSON request; `{ok, summary | error}` JSON response; additive serde
  `Serialize` for `MapValidationError` (or a response DTO) in oathstar-content; a
  server-built `ContentCatalog` from `load_beginner_world()`; in-crate tests + docs.
- Out: the `/admin/editor` canvas UI (intake D); draft persistence/publish (intake F);
  player-facing changes; a real user/content DB; client-supplied catalogs.

## Notes

- Forge ticket: `e7c7a0d4-db81-4b91-9cf8-4546ebdcd6c6` (#44)
- Related docs: `docs/map-system.md` (Map Document Model); forge
  `AD-claude-map-document-model-001`, `AD-claude-studio-sidecar-001`; intake
  `INTAKE-online-first-multiplayer-and-auth-gated-studio` (sections D/F)
- Promoted from intake: derived (the intake birthed #41–#44; left unedited per the
  owner's preserve guardrail)
- Active pipeline: `WORK-studio-map-validate-endpoint-v1`
