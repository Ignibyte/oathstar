---
pipeline_id: e4fddf43-71b2-49fd-a5de-5f220dfe73dd
title: WORK-studio-sidecar-and-auth-lib-v1
ticket: 2ba66eaf-a7cb-499c-bf71-cf888f37bc08
type: work
intake: docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md
notes: WORK-studio-sidecar-and-auth-lib-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-studio-sidecar-and-auth-lib-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Oathstar Studio as a separate Rust **sidecar** with owner login,
  on a shared **`oathstar-auth`** library — the authenticated management surface
  (ticket #42, reframed from "in-app admin surface" to a separate sidecar).
- **Scope:**
  - **In:** (1) a NEW **`oathstar-auth`** lib crate — lift #41's
    `Principal`/roles/`SessionStore`/extractor/`AuthError` out of
    `oathstar-server`, make the extractor **generic over app state**
    (`FromRef`), and add **login** primitives (credential verify, session
    issuance, **httpOnly + SameSite session cookie**). (2) a NEW
    **`oathstar-studio`** binary on **`127.0.0.1:7879`** (loopback) —
    `GET`/`POST /login`, `POST /logout`, a role-gated dashboard shell; **owner
    login via a single configured secret (env var)** for v1. (3) **rewire
    `oathstar-server`** onto `oathstar-auth`, #41 behavior + tests unchanged.
    (4) studio static UI (login HTML/CSS) **embedded** in the binary.
  - **Out (later tickets):** the **map editor** (#43), **player auth** on the
    game server, a real **user store / password DB**, OAuth, registration,
    email, content draft/validate/publish, DM tools.
- **Systems:** NEW `oathstar-auth` (lib) + `oathstar-studio` (bin); MODIFY
  `oathstar-server` (rewire). **No** engine/core/content/storage/game-client
  changes.

## Acceptance Criteria (EARS)
| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an unauthenticated request hits the studio dashboard, the studio shall redirect to `/login` and not render the admin shell. | studio test |
| REQ-002 | When valid owner credentials are POSTed to `/login`, the studio shall create a session, set an httpOnly + SameSite session cookie, and grant the dashboard. | studio test (login → cookie → dashboard 200) |
| REQ-003 | When invalid credentials are POSTed to `/login`, the studio shall re-render the login page with an error and set **no** session — a typed refusal, never a panic/500. | studio test |
| REQ-004 | When `/logout` is POSTed with a valid session, the studio shall invalidate the session and clear the cookie. | studio test |
| REQ-005 | The studio shall bind to a loopback interface (`127.0.0.1`) by default and never default to a public interface. | review + config test |
| REQ-006 | The shared `oathstar-auth` crate shall own the auth primitives and be consumed by **both** `oathstar-server` and `oathstar-studio`; the extractor shall be generic over app state (`FromRef`). | cargo test + dependency check |
| REQ-007 | After `oathstar-server` is rewired onto `oathstar-auth`, the existing #41 behavior (player endpoints, `/admin/session` probe, 401/403) shall be unchanged and all #41 tests shall pass. | cargo test --workspace |
| REQ-008 | The studio login page + CSS shall be served from the binary (embedded assets), requiring no external asset directory at runtime. | build + studio test |
| REQ-009 | The v1 owner credential shall be a single configured secret (env var); unset or blank shall mean no owner can log in (reusing #41's set-but-empty ≠ unset guard). | studio test (both arms) |

## Locked-In Decisions
- **Separate Rust sidecar** (`oathstar-studio` bin, loopback-bound,
  server-rendered HTML) — **amends Decision 056** ("in-app admin surface" →
  "separate Rust studio sidecar"). Monolithic-Rust, no second backend stack.
- **Shared `oathstar-auth` lib.** #41's auth lifts here; the extractor becomes
  **generic over state** (`FromRef<S> for Arc<SessionStore>`); both binaries
  depend on it → one auth/session model. Game server (#41) behavior unchanged.
- **Session carrier = httpOnly + SameSite session cookie** (random session id →
  `Principal` in the `SessionStore`). The extractor reads the **cookie**
  (browser) — extending #41's Bearer-header path (Design picks cookie-only vs
  cookie+bearer, and Strict vs Lax).
- **v1 credential = a single configured owner secret (env).** No user
  DB/passwords-at-rest yet; **extends to a real user store when player accounts
  land** (Decision 056). Reuse the blank-env-≠-unset guard.
- **Studio shell + login only** in this slice — NO map editor (#43), NO player
  auth on the game server (later), NO content publish.

## Design-Deferred (Phase 2 decides)
- **Cookie/session mechanism + NEW DEP:** `axum-extra` (`CookieJar`) vs
  `tower-sessions` vs **hand-rolled** `Set-Cookie`/`Cookie` via `HeaderMap` (no
  new dep). None are in the tree today → any crate needs **cargo-deny + machete**
  clearance; the project prizes a lean tree, so weigh minimal-hand-rolled vs
  robust-axum-extra.
- SameSite Strict vs Lax; cookie name; session-id CSPRNG source; session
  TTL/expiry; in-memory (v1, lost on restart) vs persisted.
- **CSRF** posture for `POST /login`/`/logout` (SameSite + same-origin form;
  possibly a CSRF token).
- The generic-extractor refactor (`FromRef`), and whether `Principal`/`AuthRole`
  stay in `oathstar-protocol` (wire types) or move to `oathstar-auth`.
- Static-asset embedding (`rust-embed` vs `include_str!`/`include_dir`); HTML
  rendering (reuse `oathstar-datastar`'s escaping for safety).
- Exact owner-secret env var name + how it maps to the owner `Principal`.

## Proposed Crate Structure (for sign-off)
```
crates/
  oathstar-auth/    NEW lib  — Principal/roles, SessionStore, authenticate,
                              require_role, AuthError, GENERIC extractor
                              (cookie+bearer), login (verify/issue/cookie)
  oathstar-studio/  NEW bin  — axum :7879 (loopback); /login /logout / (dashboard);
                              static/ embedded; owner-secret env
  oathstar-server/  MODIFY   — depends on oathstar-auth; deletes src/auth.rs
                              (moved); #41 behavior + tests unchanged
```

## Scope Note / Sequencing (for sign-off)
This is a **meaty slice** (2 new crates + a rewire + new login/session/cookie).
Default plan = one slice (delivers the working login). **Alternative if you'd
prefer smaller, lower-risk steps:** split into **A)** extract `oathstar-auth`
(pure behavior-preserving refactor, #41 tests as the net), then **B)** the
`oathstar-studio` sidecar + login. Owner picks at the plan gate.

## Operating Constraints (this run)
- **NOT auto-approve.** Present the plan + crate structure; **wait for owner
  confirmation** before Phase 2/implementation.
- **Dirty-worktree guardrails:** preserve all unrelated uncommitted work (UI
  tweaks, ElvGames import, `decisions.md` 056/057, tickets, intake). No
  revert/delete/stage/commit of unrelated files. Scope to `oathstar-auth` +
  `oathstar-studio` + the `oathstar-server` rewire + directly-relevant docs/tests.
- **Tests:** `cargo test --workspace` + `bin/gate.sh` (FULL at commit).

## Linked Artifacts
- Design docs: `docs/decisions.md` (056 online-first/auth-gated studio — to be
  amended; 057 the #41 auth seam; 016 REST+SSE), `docs/technical-architecture.md`.
- Intake: `docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md`
  (program-level — this is slice **B "Admin Shell"**, reframed).
- Ticket doc: `docs/planning/tickets/open/TICKET-42-admin-studio-shell-v1.md`
- Forge ticket: `2ba66eaf-a7cb-499c-bf71-cf888f37bc08` (#42)
- Builds on: #41 (`f6a3d20a`, committed `7551b02`; `AD-claude-auth-seam-001`).

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
