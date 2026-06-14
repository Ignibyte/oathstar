---
pipeline_id: dcbf7a22-f02f-47e0-8ced-50e05984f982
title: WORK-auth-session-and-role-boundary-v1
ticket: f6a3d20a-1888-422f-a1a5-0fcb69b47633
type: work
intake: docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md
notes: WORK-auth-session-and-role-boundary-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-auth-session-and-role-boundary-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Auth session + role boundary v1 — the first server-side
  authentication/session/role seam so future online play and `/admin/editor`
  (Oathstar Studio) APIs can be refused server-side (Decision 056).
- **Scope:**
  - **In:** `Principal`/`Session` DTOs (id/name/roles) + an **auth `Role`** enum
    (`player`/`editor`/`admin`/`dm`/`owner`); a custom axum `FromRequestParts`
    extractor + a require-role helper for protected handlers; a typed
    `AuthError` (`IntoResponse`) returning **real HTTP 401 (unauthorized)** vs
    **403 (forbidden)**; a **deterministic local-dev owner/editor** strategy via
    a new env var (prod default = off); **one minimal protected JSON probe
    route** (e.g. `GET /admin/session`) as the seam's first consumer + test
    surface; regression tests (401/403/200 + player-endpoint non-regression);
    docs. Server lives in `crates/oathstar-server`; auth types likely in
    `crates/oathstar-protocol`.
  - **Out:** production password storage/hashing, OAuth, registration/email,
    donations, database/persistence migration, multiplayer player identity in
    engine state, the **admin shell UI (#42)**, WebSockets, the map editor (#43).
- **Systems:** server (`oathstar-server`), protocol (`oathstar-protocol`). **No**
  engine/core/storage/client changes.

## Acceptance Criteria (EARS)
| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a request to a protected handler carries no valid session, the server shall refuse it with a typed **HTTP 401 (unauthorized)** and not execute the handler's body. | server test (`#[tokio::test]`) |
| REQ-002 | When a request to a protected handler carries a valid session that lacks the required role, the server shall refuse it with a typed **HTTP 403 (forbidden)**. | server test |
| REQ-003 | When a request to a protected handler carries a valid session with the required role, the handler shall receive an authenticated `Principal` carrying `id`, `name`, and `roles`. | server test (probe echoes the principal) |
| REQ-004 | When the local-dev owner env toggle is set, the server shall deterministically grant an owner/editor session for development; when unset (the production default), protected handlers shall refuse. | server test (both arms) + docs |
| REQ-005 | The existing player endpoints (`/state`, `/command`, `/events`, `/events/json`, `/events/datastar`) shall continue to respond without authentication in local development. | server test (existing tests still green; auth is per-route, not global) |
| REQ-006 | The server shall expose at least one minimal protected JSON route (the auth probe) requiring an admin/editor-tier role, which echoes the authenticated principal — the boundary's first consumer. | server test (401 / 403 / 200 matrix) |

## Locked-In Decisions
- **Typed boundary → real HTTP status.** The auth seam returns 401/403 via an
  `AuthError: IntoResponse`, **deliberately breaking** the server's existing
  in-band `200 {ok:false,error}` refusal pattern (`SaveLoadResponse`). An
  unauthorized request must **never reach game logic**. [REQ-001/002]
- **Per-route opt-in, not global middleware.** A custom `FromRequestParts`
  extractor (`Principal`) + a require-role helper; handlers opt in by taking it.
  Player routes stay open (REQ-005) — no blanket layer over the whole router.
- **Auth `Role` is distinct from the entity `Role`.** `oathstar-core` already
  has an entity-capability `Role` (Decision 039: Talkable/Shopkeeper/…). The
  auth role is a separate type/namespace — no collision.
- **Deterministic dev-owner via env, no prod weakening.** Follow the existing
  `std::env::var(X).unwrap_or_else(default)` pattern; an unset toggle (prod
  default) grants nothing → protected routes refuse. No production
  password/OAuth/DB (out of scope). [REQ-004]
- **Session stays minimal in v1.** Just enough principal/session to exercise the
  boundary; real accounts/sessions/persistence are deferred (Decision 056
  "revisit when the first hosted deployment is designed").
- **Auth logic in a testable module**, not the mutation-excluded `fn main`;
  `main` only wires the protected route(s). Tests follow the existing
  direct-handler `#[tokio::test]` + `State(test_app_state())` pattern.
- **Probe route is an API, not the admin shell.** The `GET /admin/session`-style
  JSON probe is the seam's test surface; the admin UI is #42.

## Design-Deferred (Phase 2 decides)
- Session carrier: `Authorization: Bearer <token>` vs cookie vs dev-env-derived
  principal; how the dev-owner env maps to a principal/token.
- Whether a tiny in-memory token→principal registry is worth it in v1, or
  env-derived only.
- Exact role(s) the probe requires ({editor, admin, owner}?) and require-role
  ergonomics (helper fn vs typed extractor wrapper like `RequireRole<…>`).
- Final home for `Principal`/auth-`Role` (`oathstar-protocol` — recon's lean —
  vs a server-local `auth` module if they stay server-only for now).

## Linked Artifacts
- Design docs: `docs/decisions.md` (056 online-first/auth-gated studio; 016
  REST+SSE; 039 entity `Role` — disambiguation), `docs/technical-architecture.md`
  (API & transport).
- Intake: `docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md`
  (program-level — #41 is slice **A**; the intake is NOT flipped to `promoted`
  because it also spawns #42/#43+).
- Ticket doc: `docs/planning/tickets/open/TICKET-41-auth-session-and-role-boundary-v1.md`
- Forge ticket: `f6a3d20a-1888-422f-a1a5-0fcb69b47633` (#41)

## Operating Constraints (this run)
- **AUTO-APPROVE; STOP BEFORE `/commit`.** Drive plan → complete, then halt.
- **Dirty-worktree guardrails:** the worktree holds unrelated uncommitted work
  (UI edits, ElvGames tileset import, modified architecture docs, tickets
  #42/#43). **Do not revert/delete/stage/commit unrelated files.** #41 touches
  only `crates/oathstar-server`, `crates/oathstar-protocol`, its own ticket/
  pipeline docs, and directly-relevant tests.
- **Tests:** `cargo test --workspace` + `./bin/gate.sh --fast`. No npm (server
  only). Any `--fast` JS reds from the pre-existing dirty worktree
  (client-app.js, elvgames test) are documented as unrelated, not fixed.

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
