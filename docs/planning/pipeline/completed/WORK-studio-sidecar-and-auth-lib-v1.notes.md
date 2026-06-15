# WORK-studio-sidecar-and-auth-lib-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

- **pipeline_id:** e4fddf43-71b2-49fd-a5de-5f220dfe73dd
- **aar_id:** bb4a5e84-db9e-4305-b9de-2f1645a89218

## Phase 1 — Plan
- **Request:** stand up Oathstar Studio as a separate Rust **sidecar** with owner
  login, on a shared `oathstar-auth` lib — the secure management surface. This
  IS ticket #42 (admin studio shell), **reframed** from Decision 056's "in-app
  admin surface" to the owner's decided **separate Rust sidecar** architecture.
- **Intake source:** `INTAKE-online-first-multiplayer-and-auth-gated-studio.md`
  (program-level; this is slice **B "Admin Shell"**, reframed). Left `candidate`.
- **Classification / tier:** work pipeline; one shippable slice (studio shell +
  auth-lib extraction + owner login). **Flagged as meaty** — split option
  offered (extract `oathstar-auth` first, then studio+login).
- **Forge recall:**
  - `docs-search`: **TICKET-42 already exists** (`pending-forge`), framed "same
    web app" — superseded by the sidecar decision. #41's WORK spec (the auth
    foundation lifting into `oathstar-auth`). Decision 056 (to amend).
  - `knowledge-search`: top hit = `AD-claude-auth-seam-001` (#41's seam — the
    foundation). Failure `BF-auth-empty-env-owner-bypass-001` (set-but-empty env
    bypass) — re-watch for the new owner-secret + session code.
  - No bulletins.
- **Verified at plan:**
  - Workspace = 6 crates (`Cargo.toml [workspace].members`); this adds 2
    (`oathstar-auth` lib + `oathstar-studio` bin). New crates inherit
    `version/edition/authors/license.workspace` + **must add `[lints] workspace =
    true`** for strict clippy.
  - **No cookie/session crate in the tree** (no `axum-extra`/`tower-sessions`/
    `tower-cookies`/`cookie`/`time`) → browser login needs a **new dep** (or
    hand-rolled `Set-Cookie`/`Cookie` via `HeaderMap`); strict deny/machete
    applies. Workspace deps today: anyhow, async-stream, axum 0.8, futures-core,
    serde, serde_json, tokio, toml.
  - #41's extractor is `impl FromRequestParts<AppState> for AuthPrincipal`
    (`oathstar-server/src/auth.rs:136`, reads `state.auth_sessions`) — **must
    become generic over state** (`FromRef`) to be shared by two binaries.
  - Branch `ticket-41-…`, HEAD `7551b02` (#41 committed) — the foundation.
- **Ticket:** forge `2ba66eaf-a7cb-499c-bf71-cf888f37bc08` (#42); local doc
  `docs/planning/tickets/open/TICKET-42-admin-studio-shell-v1.md` (reframed).
- **EARS reviewed:** REQ-001 redirect-to-login / REQ-002 login→cookie→dashboard /
  REQ-003 invalid→re-render-no-session / REQ-004 logout / REQ-005 loopback bind /
  REQ-006 shared auth lib + generic extractor / REQ-007 server #41 non-regression
  / REQ-008 embedded UI / REQ-009 owner-secret both arms. Finalized in spec.
- **Owner signed off** ("lets continue"): crate structure approved; **(A) one
  slice**; **`axum-extra`** for cookies; **single owner-secret** credential (v1).
  AAR `bb4a5e84` opened; Phase 1 PASS. Proceeding to design (driving the
  remaining phases; will stop before `/commit` to confirm scope, per the messy
  worktree).

## Phase 2 — Design
- **Approach / architecture** — 2 new crates + a rewire, all Rust (monolithic-tech).
  - **`oathstar-auth` (lib)** — the shared auth foundation. Lifts #41's auth and
    adds session/cookie/login primitives. `Principal`/`AuthRole` STAY in
    `oathstar-protocol` (wire types); `oathstar-auth` depends on protocol.
    - **`SessionStore` becomes mutable** — `{ sessions: Arc<RwLock<HashMap<String,
      Principal>>> }`, `#[derive(Clone)]` (cheap, Arc inside). `from_owner_token`
      (seed, #41) + `resolve` (read lock) unchanged for the game server's bearer
      path; NEW `create_session(principal) -> id` (CSPRNG id via **getrandom**,
      32 bytes → 64-hex; write lock) and `remove_session(id)` for login/logout.
      The lock is held only for the map op (no `await` while held).
    - **Generic extractor** — `impl<S> FromRequestParts<S> for AuthPrincipal where
      SessionStore: FromRef<S>` (bearer path, 401/403) so BOTH binaries' app
      states share it. `AuthError`/`require_role`/`authenticate` move here.
    - **Cookie + login primitives** — `session_cookie(id) -> Cookie` (httpOnly,
      **SameSite=Strict**, Path=/, name `oathstar_studio_session`),
      `clearing_cookie()`, `principal_from_cookie(&CookieJar, &SessionStore)`
      (via **axum-extra**); `verify_secret(provided, configured) -> bool`
      (**constant-time**, hand-rolled XOR — no dep); owner-secret-from-env with a
      blank guard (reuses `PR-claude-blank-env-config-is-not-absent-001`).
  - **`oathstar-studio` (bin)** — loopback `127.0.0.1:7879`. `fn main` = thin
    composition root (mutation-excluded by the existing regex). `StudioState
    { sessions: SessionStore, owner_secret: Option<String> }` + `FromRef`.
    Handlers/render/config in testable modules: **login** (GET form / POST verify
    → `create_session(owner)` → Set-Cookie → 303 `/`), **logout** (POST →
    `remove_session` + clear cookie → `/login`), **dashboard** (GET →
    `principal_from_cookie` → role-gate → render, else 303 `/login`). HTML is
    server-rendered in Rust; it echoes **no user input** (fixed error strings,
    password never reflected) → **no injection surface**, no escape dep. CSS via
    `include_str!`.
  - **`oathstar-server` (rewire)** — depend on `oathstar-auth`; **delete
    `src/auth.rs`** (moved); `AppState.auth_sessions: SessionStore` (was
    `Arc<SessionStore>`) + `FromRef`; `admin_session` uses the generic extractor.
    #41 unit tests move to `oathstar-auth`; integration tests adapt. Behavior
    unchanged (REQ-007).
  - **New deps:** `axum-extra` (cookie feature) + `getrandom` — both new
    (not transitive). Added to `[workspace.dependencies]`; **must clear
    cargo-deny (license/source) + machete (used)**.
- **File manifest:**
  | # | File | Change |
  |---|---|---|
  | 1 | `Cargo.toml` (workspace) | add members `oathstar-auth`, `oathstar-studio`; add `axum-extra` (cookie) + `getrandom` to `[workspace.dependencies]` |
  | 2 | `crates/oathstar-auth/Cargo.toml` | NEW lib — deps protocol, axum, axum-extra, getrandom, serde_json; `[lints] workspace = true` |
  | 3 | `crates/oathstar-auth/src/lib.rs` | NEW — module root + pub re-exports |
  | 4 | `crates/oathstar-auth/src/session.rs` | NEW — `SessionStore` (mutable; from_owner_token/resolve/create_session/remove_session) + tests |
  | 5 | `crates/oathstar-auth/src/error.rs` | NEW — `AuthError` + `IntoResponse` 401/403 (from #41) + tests |
  | 6 | `crates/oathstar-auth/src/extract.rs` | NEW — `authenticate`, `require_role`, generic `AuthPrincipal`, `principal_from_cookie` + tests |
  | 7 | `crates/oathstar-auth/src/cookie.rs` | NEW — `session_cookie`/`clearing_cookie`, constant-time `verify_secret`, owner-secret-from-env (blank guard) + tests |
  | 8 | `crates/oathstar-studio/Cargo.toml` | NEW bin — deps oathstar-auth, protocol, axum, axum-extra, tokio; `[lints] workspace = true` |
  | 9 | `crates/oathstar-studio/src/main.rs` | NEW — composition root: `StudioState`, router, loopback bind, serve |
  | 10 | `crates/oathstar-studio/src/handlers.rs` | NEW — login GET/POST, logout POST, dashboard GET + tests |
  | 11 | `crates/oathstar-studio/src/render.rs` | NEW — `login_page(error)`, `dashboard_page(principal)` → `Html` + tests |
  | 12 | `crates/oathstar-studio/src/config.rs` | NEW — `StudioConfig` (bind addr, owner secret) from env + tests |
  | 13 | `crates/oathstar-studio/static/studio.css` | NEW — login/dashboard CSS (`include_str!`) |
  | 14 | `crates/oathstar-server/Cargo.toml` | MOD — add `oathstar-auth` path dep |
  | 15 | `crates/oathstar-server/src/main.rs` | MOD — `use oathstar_auth::…`; `auth_sessions: SessionStore` + `FromRef`; admin_session via generic extractor; integration tests adapt |
  | 16 | `crates/oathstar-server/src/auth.rs` | DELETE — moved to `oathstar-auth` (unit tests move too) |
- ### Regression Test Plan
  | # | Test | Proves |
  |---|---|---|
  | T1 | auth `SessionStore` — create_session→resolve→remove_session; ids non-empty + distinct | REQ-002/004 |
  | T2 | auth cookie — `session_cookie` is httpOnly + SameSite=Strict + named; `clearing_cookie` clears | REQ-002/004 |
  | T3 | auth `verify_secret` — constant-time; true only for the exact secret | REQ-002/009 |
  | T4 | auth owner-secret-from-env — set→Some; unset/blank→None (both arms) | REQ-009 |
  | T5 | auth `principal_from_cookie` — valid id→Some; absent/unknown→None | REQ-001/002 |
  | T6 | auth (moved #41) — authenticate bearer (missing/non-bearer/unknown/known/blank), require_role, AuthError 401/403, grants | REQ-006/007 |
  | T7 | studio POST /login valid → session + Set-Cookie + 303 `/` | REQ-002 |
  | T8 | studio POST /login invalid → re-render, **no** Set-Cookie, no session | REQ-003 |
  | T9 | studio POST /login when secret unset/blank → refused | REQ-009 |
  | T10 | studio POST /logout w/ session → remove + clearing cookie + 303 `/login` | REQ-004 |
  | T11 | studio GET `/` no/invalid session → 303 `/login` (not render) | REQ-001 |
  | T12 | studio GET `/` valid session+role → 200 dashboard (shell markers) | REQ-002 |
  | T13 | studio config — default bind `127.0.0.1` (loopback) | REQ-005 |
  | T14 | studio render — login HTML has the form + embedded CSS | REQ-008 |
  | T15 | server (rewire) — #41 `/admin/session` 401/403/200 via generic extractor; player endpoints unaffected; all #41 tests green | REQ-007 |
  - **Uncoverable/excluded:** both `fn main` (composition roots, mutation-excluded);
    the real TCP bind/serve; the browser visual smoke (manual — no jsdom for an
    HTTP login flow). Sessions are in-memory (lost on restart) — by design (v1).
- **Risks / decisions:**
  - **R-1 new deps** `axum-extra` + `getrandom` must pass **cargo-deny** (MIT/Apache
    — standard; add to allowlist if deny is strict-source) + **machete** (both
    used). Verify with `cargo deny` at implement.
  - **R-2 mutable `SessionStore`** (RwLock) — #41's read-only usage unaffected;
    #41 tests adapt to the field change. Lock held only for the map op.
  - **R-3 CSRF** — **SameSite=Strict** means a cross-site POST won't carry the
    session cookie, so cross-site login/logout POSTs are unauthenticated no-ops;
    a per-form CSRF token is deferred (belt-and-suspenders). Inspect verifies.
  - **R-4 cookie `Secure` flag** — loopback http v1 → `Secure=false` (else the
    browser drops it over http); a TLS deploy sets `Secure=true` (config). Noted.
  - **R-5 constant-time secret compare** (hand-rolled XOR-accumulate, no dep) —
    no timing leak; reuse #41's blank-env guard for the secret.
  - **R-6 mutation 100%** on the new auth/studio logic (FULL gate at commit) —
    tests target every branch; both mains excluded.
  - **R-7 extraction touches #41's working auth** (types move) — #41 tests are the
    safety net (REQ-007).
  - **D-1** server-side session store + getrandom id + httpOnly cookie (NOT
    signed/private cookies) — supports clean logout; v1.
  - **D-2** studio renders no echoed user input → no injection surface (no escape
    dep / no datastar coupling).

## Phase 3 — Implement
- **Built — `oathstar-auth` (NEW lib):**
  - `session.rs` — `SessionStore` is now **mutable** (`Arc<RwLock<HashMap<String,
    Principal>>>`, `Clone + Default`): `new`, `from_owner_token` (blank-env guard),
    `from_env` (`OATHSTAR_DEV_OWNER`), `resolve`, `create_session` (mints a 256-bit
    OS-CSPRNG hex id via `getrandom::fill`), `remove_session`. `owner_principal()`
    is the one owner identity both the dev-bearer seed and a studio login resolve to.
  - `cookie.rs` — `SESSION_COOKIE`, `session_cookie`/`clearing_cookie` (HttpOnly +
    `SameSite=Strict`, Path=/, Secure-off-for-loopback), `verify_secret`
    (constant-time XOR fold incl. length), `owner_secret_from_env`
    (`OATHSTAR_OWNER_PASSWORD`, blank≠set).
  - `extract.rs` — `authenticate` (Bearer), `require_role`, `principal_from_cookie`
    (browser path), and the **generic** `AuthPrincipal: FromRequestParts<S> where
    SessionStore: FromRef<S>` — one extractor, two binaries.
  - `error.rs` — `AuthError` → 401/403 JSON (lifted from #41).
- **Built — `oathstar-studio` (NEW bin, loopback `127.0.0.1:7879`):** `config.rs`
  (`StudioConfig::from_env`, `default_studio_addr` = `127.0.0.1:7879`), `render.rs`
  (embedded `login_page`/`dashboard_page` via `include_str!` CSS — no echoed input),
  `handlers.rs` (`login_form`/`login_submit`/`logout`/`dashboard`), `main.rs`
  (`StudioState` + `FromRef<StudioState> for SessionStore`, router, warns when the
  owner secret is unset). `static/studio.css` embedded.
- **Rewired — `oathstar-server`:** deleted `src/auth.rs` (moved to the lib); now
  `use oathstar_auth::{…}`; `AppState.auth_sessions: SessionStore` (was
  `Arc<SessionStore>`) + `impl FromRef<AppState> for SessionStore`; `admin_session`
  calls `oathstar_auth::require_role`. **All 418 existing tests pass unchanged**
  (REQ-007).
- **Workspace:** added both crates to `members`; added `axum-extra` (cookie) +
  `getrandom` to `[workspace.dependencies]`.
- **Checks (impl-time):** `cargo clippy --workspace --all-targets -D warnings`
  clean; `cargo test --workspace` 418 green; **`cargo deny check` ok** (new deps
  axum-extra/cookie/getrandom/time clear advisories/bans/licenses/sources);
  `cargo machete` no unused deps; `cargo fmt --check` clean.
- **Deviations from design (+ reason):**
  - Cookie posture is **cookie-for-browser + bearer-for-API** (both kept), not
    cookie-only — the generic extractor preserves #41's Bearer path for the game
    server while the studio uses `principal_from_cookie`. (Design left this open.)
  - `from_env` (the `OATHSTAR_DEV_OWNER` seed) **moved into the lib** alongside
    `from_owner_token`, so the server's call site is unchanged — keeps the env-read
    next to the seeding logic it drives.
  - `Principal`/`AuthRole` **stay in `oathstar-protocol`** (re-exported from
    `oathstar-auth`) — they are wire types; no move needed.
- **Tests deferred to Phase 4** (per pipeline): `oathstar-auth` + `oathstar-studio`
  ship with 0 tests at implement; the moved #41 unit tests + new studio/auth tests
  land at Validate.

## Inspect (Phase 3.5)
- **Lenses run** (4 parallel `general-purpose` critics over the full diff):
  security/auth-boundary, correctness/flows, refactor-fidelity-vs-#41,
  simplification/reuse/deps. Each fed the two #41 failure classes
  (`BF-auth-empty-env-owner-bypass-001`, `PR-claude-blank-env-config-is-not-absent-001`)
  to re-check.
- **Security — CLEAN.** Verified: 256-bit OS-CSPRNG session ids (no weak fallback);
  `verify_secret` constant-time (length folded into the accumulator, no early-return
  oracle); cookie HttpOnly + SameSite=Strict (adequate CSRF defense for the
  cookie-authenticated POST `/logout`; login needs the secret so login-CSRF can't
  escalate); login mints a fresh id and overwrites any pre-set cookie (no fixation);
  logout invalidates server-side **and** clears the cookie; both #41 blank-env classes
  intact on the studio-login **and** bearer paths; the generic extractor weakens
  nothing #41 had; the secret is never logged or reflected; no unauthenticated route
  reaches the dashboard or any state change.
- **Correctness — CLEAN.** Traced REQ-001…005 end-to-end; owner `grants(Editor)` so
  the dashboard is reachable; `Redirect::to` = 303 (POST→GET); `Form` is the last
  extractor; the only `expect` is the OS-CSPRNG (non-input path).
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | low | Seeded owner principal id/name unified `dev-owner`/`Local Dev Owner` → `owner`/`Owner` (`oathstar-auth/src/session.rs:86`) | **Real but intended** — `roles` (the only auth-driving field) unchanged; grep confirms no code/test asserts the old literal (every `dev-owner` hit is the *concept* in #41's archived docs). Design-sanctioned unification (notes §Phase-3). | No code fix. The wire-visible `/admin/session` `id` changed → call it out in the Decision 056/057 amendment (Phase 5). |
  | 2 | low | `default_studio_addr` needlessly `pub` (`oathstar-studio/src/config.rs:32`) | **Real** — only caller is the internal `studio_addr`; dead public surface. | **Fixed** — privatized (dropped `pub` + `#[must_use]`); re-clippy green. Phase-4 test reaches it via a same-module `#[cfg(test)]`. |
  | 3 | low | `authenticate` re-exported with no external caller (`oathstar-auth/src/extract.rs:14`, `lib.rs`) | **Rejected** — intentional shared primitive, symmetric with `principal_from_cookie` (which the studio uses); the building block for future manual-header / WebSocket auth in the player-auth slice. | Keep public; documented. |
  | 4 | low | `dashboard`/`logout` clone the whole `StudioState` (incl. `owner_secret`) per request though only `login_submit` reads it (`oathstar-studio/src/handlers.rs:41,50`) | **Rejected** — marginal on a loopback admin tool's human-click paths; the `SessionStore` clone is already a cheap `Arc`; one `State` type keeps the handler model simple. | Keep as-is. |
- **Net:** no critical/high/medium. One trivial cleanliness fix applied; one intended
  identity change to surface in the docs; two rejected-with-rationale. No
  `failure-record` warranted (no real bug found); no new prevention rule (no mistake
  class). Lenses confirm the auth boundary is sound.

## Phase 4 — Validate
- **Tests added (T1–T15 mapped):**
  - `oathstar-auth` (**20 tests**): session — create/resolve/remove roundtrip,
    unique 64-hex ids, idempotent remove, `from_owner_token` blank guard,
    `from_env` (set/blank/unset), `owner_principal` (T1/T4); cookie — hardened
    `session_cookie`, `clearing_cookie`, `verify_secret` (exact/length/None),
    `owner_secret_from_env` blank guard (T2/T3/T4); extract — the moved #41
    bearer suite (missing/non-bearer/unknown/known/blank/trim/non-utf8),
    `require_role`, and a **direct** `principal_from_cookie` test (T5/T6); error —
    `AuthError` 401/403 (T6).
  - `oathstar-studio` (**14 tests**): config — loopback default + `from_env`
    addr/secret wiring (T13); render — login form + embedded CSS + error banner,
    dashboard shell markers (T8/T14); handlers — login valid/invalid/unset,
    logout (with + without session), dashboard no/unknown/owner session, the
    `login_form` GET, and an **editor-admitted / player-redirected** pair that
    pins the role gate (T7–T12).
  - `oathstar-server`: #41's integration suite came along unchanged and passes —
    `/admin/session` 401/403/200 via the now-generic extractor (T15, REQ-007).
- **Source tweaks during validate (for mutation/cleanliness):** dropped the
  unused `FromRef<StudioState>` impl (dead + a guaranteed survivor); replaced
  `String::with_capacity(len*2)` in `new_session_id` with `String::new()` (the
  capacity arithmetic was an unkillable mutant); added `tokio` as an
  `oathstar-auth` dev-dependency for the async cookie test.
- **`cargo test --workspace`:** all green — 20 `oathstar-auth` + 14
  `oathstar-studio` + 34 `oathstar-server` + the rest of the workspace; plus
  `node --test tests/*.test.js` **67/67** (JS untouched by #42).
- **`bin/gate.sh` (FULL):** `GATE GREEN [full]` — **17/17**. mutation
  **429 caught / 0 missed → MSI 100.0%**; Rust line coverage **98.40% (≥94)**;
  JS coverage **87.65% (≥75)**.
- **The gate earned its keep:** the first FULL run was RED on exactly two things —
  one mutation survivor (`principal_from_cookie -> None`, catchable only by a
  direct in-package test because cargo-mutants scopes its run to the mutated
  crate) and a rustfmt nit on the hand-written tests. Both fixed at source; the
  re-run is green. No pre-existing failures; no baselines or suppressions.

## Phase 5 — Complete
- **Docs updated:** `docs/decisions.md` — **Decision 058** appended additively
  (separate Rust studio sidecar on a shared `oathstar-auth`; amends 056; records
  the `dev-owner`→`owner` principal-id unification). `CLAUDE.md` "What this is" now
  notes the studio is a separate loopback sidecar. The user's 056/057 left verbatim.
- **Forge capture:** `BF-studio-cross-crate-mutation-gap-001` (failure — the
  `principal_from_cookie` cross-crate mutation gap), `PR-claude-cross-crate-mutation-coverage-001`
  (rule — shared-lib fns need in-crate tests; cargo-mutants scopes per crate),
  `AD-claude-studio-sidecar-001` (the sidecar + shared-auth decision). AAR
  `bb4a5e84…` closed (completed, effectiveness 4, 3 novel findings).
- **Ticket closed:** forge `#42` (`2ba66eaf…`) → done.
- **Archived:** doc pair moved to `docs/planning/pipeline/completed/`.
- **STOP BEFORE COMMIT** (per the run constraint): the FULL gate is green and the
  receipt is written, but `/commit` is the owner's to run.
