---
pipeline_id: 7d550388-222e-40ef-aeb7-23223ad301e4
title: WORK-studio-map-validate-endpoint-v1
ticket: e7c7a0d4-db81-4b91-9cf8-4546ebdcd6c6
type: work
intake: docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md
notes: WORK-studio-map-validate-endpoint-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-studio-map-validate-endpoint-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Studio map-document validate/materialize endpoint v1 — an Editor-gated
  JSON route in `oathstar-studio` that validates + materializes a posted `MapDocument`
  and returns a typed summary/error.
- **Scope:**
  - **In:** `oathstar-studio` depends on `oathstar-content`; one Editor-gated POST route
    (request = `MapDocument` JSON; response = `{ok, summary | error}` JSON); a compact
    success summary (room/region count + `start_room_id`); the typed validation error
    rendered as JSON that **names the offending cell/ref** (additive serde `Serialize` on
    `MapValidationError` or a response DTO, in oathstar-content); a server-built
    `ContentCatalog` from `oathstar_content::load_beginner_world()`; in-crate tests + docs.
  - **Out:** the `/admin/editor` canvas UI (intake D); draft persistence/publish (intake F);
    player-facing changes; a real user/content DB; client-supplied catalogs; any change to
    the game server, protocol, engine, or the #43 model's *behavior* (serde derive only).
- **Systems:** studio (sidecar handler/router) + auth (role gate) + content (map model +
  serde). No game-server / protocol / engine / client change.

## Acceptance Criteria (EARS)
| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a caller without an Editor-granting session posts to the endpoint, the studio shall refuse without validating (no 2xx, no summary). | Rust handler test |
| REQ-002 | When an Editor posts a valid MapDocument, the endpoint shall respond 2xx with a JSON summary (`ok=true`, room count, region count, `start_room_id`). | Rust handler test |
| REQ-003 | When an Editor posts a MapDocument that fails validation, the endpoint shall respond with a typed JSON error (`ok=false`) that names the offending cell/reference. | Rust handler test |
| REQ-004 | When the request body is not a well-formed MapDocument, the endpoint shall refuse with a typed JSON error and never panic. | Rust handler test |
| REQ-005 | The endpoint response shall be renderer-agnostic JSON with no internal/engine leakage, and identical input shall yield identical output. | Rust test (serde shape + determinism) |

## Locked-In Decisions
- **Editor-gated** via `oathstar-auth` `principal_from_cookie` + `require_role(Editor)`,
  mirroring `handlers::dashboard` (an Owner session grants Editor).
- **Server-built `ContentCatalog`** from `load_beginner_world()` (entities/items;
  fixtures empty) — not client-supplied.
- **Typed JSON both ways:** success → `{ok:true, summary}`; failure → `{ok:false, error}`
  where `error` is the serde-serialized `MapValidationError` naming the cell/ref. The
  additive serde `Serialize` lands in oathstar-content (no change to validate/materialize
  behavior).
- **Stateless** — validate/materialize only; no draft persistence (intake F).
- **Tests in-crate** (oathstar-studio handler-direct; oathstar-content for the error
  serde) per `BF-studio-cross-crate-mutation-gap-001`; no unreachable defensive branches
  (`PR-claude-unreachable-defensive-branch-mutants-001`).

## Design Decisions (resolved Phase 2 — see notes)
- **Route:** `POST /editor/maps/validate`; 200 with an `{ok}` flag for validation outcomes;
  401/403 for auth, 400 for malformed body (500 reserved for a practically-impossible catalog load).
- **Error JSON:** derive `Serialize` on `MapValidationError` + `RefKind`; the `WorldInvalid`
  inner serializes as its Display string via `serialize_with` (no oathstar-core change). The
  `message` (Display) reliably names the cell/ref; structured `error` is the bonus.
- **Summary:** `{ok:true, room_count, region_count, start_room_id}`.
- **Body** as `Bytes` + `serde_json::from_slice` (handler-direct malformed testing).
- **Catalog** built once in `main()` into `StudioState.catalog: Arc<ContentCatalog>` (memoized;
  keeps the fallible load out of the handler → no unreachable branch). Full design in notes.

## Linked Artifacts
- Design docs: `docs/map-system.md` (Map Document Model); forge
  `AD-claude-map-document-model-001`, `AD-claude-studio-sidecar-001`.
- Intake doc: `docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md` (sections D/F)
- Ticket doc: `docs/planning/tickets/open/TICKET-44-studio-map-validate-endpoint-v1.md`
- Forge ticket: `e7c7a0d4-db81-4b91-9cf8-4546ebdcd6c6` (#44)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
| 3 — Implement | — |
| 3.5 — Inspect | — |
| 4 — Validate | — |
| 5 — Complete | — |
