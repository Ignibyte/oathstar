# WORK-auth-session-and-role-boundary-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

- **pipeline_id:** dcbf7a22-f02f-47e0-8ced-50e05984f982
- **aar_id:** c2cf3454-c130-4235-9838-d2d0cbb1adb6

## Phase 1 — Plan
- **Request:** `/work ticket #41 Auth Session + Role Boundary v1` — auto-approve,
  **STOP BEFORE COMMIT**. Build the server-side auth/session/role seam per
  TICKET-41 + Decision 056 + the online-first intake. Dirty-worktree guardrails
  (preserve unrelated UI/ElvGames/docs/tickets; no revert/delete/stage/commit).
- **Intake source:** `INTAKE-online-first-multiplayer-and-auth-gated-studio.md`
  (program-level; #41 = slice A "Online/Auth Foundation"). Left as `candidate`
  (it spawns #42/#43+; not a 1:1 promotion).
- **Classification / tier:** work pipeline, one shippable slice (the auth seam).
  Systems: `oathstar-server` + `oathstar-protocol`. No engine/core/storage/client.
- **Forge recall:**
  - `docs-search`: Decision 056 (online-first; auth is an API-layer concern that
    gates APIs not just frontend links; roles player/editor/admin/dm/owner;
    `/admin/editor` = Studio later; REST+SSE stays). Decision 016 (REST+SSE
    first, WebSockets later). **Decision 039 (entity `Role` vocabulary)** — the
    name to disambiguate the auth role from. technical-architecture API section.
  - `knowledge-search`: general server typed-refusal lessons + 1 prevention rule
    + 1 failure (logged to the AAR at plan via knowledge-context below).
  - No bulletins.
- **Anchors verified at plan** (Explore recon — read-only,
  `crates/oathstar-server/src/main.rs`):
  - Router `main.rs:97-107` — `Router::new().route(...).with_state(app_state)`,
    **no middleware**. Routes: `GET /`, `/health`, `/state`, `POST /command`,
    `/save`, `/load`, `GET /events`, `/events/json`, `/events/datastar`.
  - `AppState` `main.rs:28-40` — `{ engine: Arc<Mutex<Engine>>, events:
    broadcast::Sender, opening, saves }`. **No identity.**
  - Handlers return `impl IntoResponse`, all **200 + JSON**. Refusal pattern is
    **in-band** `SaveLoadResponse { ok:false, error }` (`main.rs:55-76`), NOT
    HTTP status — the auth seam must deliberately break this for 401/403.
  - DTOs in `crates/oathstar-protocol/src/lib.rs` — `#[serde(rename_all =
    "camelCase")]`, additive `#[serde(default, skip_serializing_if)]`. Idiomatic
    home for `Principal`/`Session`/auth-`Role`.
  - Config: `std::env::var(X).unwrap_or_else(default)` (`OATHSTAR_ADDR`,
    `OATHSTAR_SAVE_DIR`). Dev-owner toggle = a new env var, prod default off.
  - Tests: `#[tokio::test]` calling handlers directly with
    `State(test_app_state())` (`main.rs:382-412`); ~40 existing. No router/TCP
    spin-up. **`fn main` is the mutation-excluded composition root** — auth logic
    goes in a testable module (e.g. `src/auth.rs`); `main` only wires routes.
  - Identity today: **none** (greenfield); only `last-event-id` header read in
    `events_datastar` (SSE replay, not auth).
- **Ticket:** forge `f6a3d20a-1888-422f-a1a5-0fcb69b47633` (#41); local doc
  `docs/planning/tickets/open/TICKET-41-auth-session-and-role-boundary-v1.md`
  (frontmatter `ticket:` updated).
- **EARS reviewed:** REQ-001 401 / REQ-002 403 / REQ-003 principal{id,name,roles}
  / REQ-004 deterministic dev-owner (no prod weakening) / REQ-005 player
  endpoints unaffected / REQ-006 minimal protected probe route. Finalized in spec.
- **Open for Phase 2** (spec "Design-Deferred"): session carrier (Bearer/cookie/
  dev-env), require-role ergonomics, probe's required role set, final home for
  the auth types (protocol vs server `auth` module).

## Phase 2 — Design
- **Approach / architecture** — a greenfield, **per-route opt-in** seam (NOT a
  global middleware that would gate player routes). Pure auth logic in a testable
  `auth` module + thin glue (extractor / probe handler / `main` wiring), matching
  the project's pure-model+thin-seam idiom and the mutation-excluded-`main`
  convention. axum **0.8.9** → `FromRequestParts` via RPITIT, **no new deps**.
  - **Carrier:** `Authorization: Bearer <token>`. A read-only `SessionStore`
    (`HashMap<String, Principal>`, held as `Arc<SessionStore>` in `AppState`)
    resolves token → `Principal`. Real accounts/sessions/persistence deferred
    (Decision 056 "revisit when first hosted deployment is designed").
  - **Dev-owner:** `SessionStore::from_env()` reads `OATHSTAR_DEV_OWNER`; set →
    seeds one deterministic owner principal under that token; **unset (prod
    default) → empty store → every protected request is 401**. No prod weakening.
  - **Role model:** `AuthRole { player, editor, admin, dm, owner }` +
    `Principal { id, name, roles }` live in **`oathstar-protocol`** (the probe
    echoes a `Principal` as JSON). The auth `AuthRole` is **distinct** from
    `oathstar-core`'s entity `Role` (Decision 039). `Principal::grants(required)`
    encodes **owner-implies-all** (Decision 056: owner = full authority); admin ≠
    editor (distinct powers).
  - **Typed boundary:** server-side `AuthError { Unauthorized, Forbidden }` impls
    `IntoResponse` → real **401 / 403** (deliberately breaks the in-band
    `200 {ok:false}` pattern). `authenticate(&HeaderMap, &SessionStore) ->
    Result<Principal, AuthError>` produces 401s; `require_role(&Principal,
    AuthRole) -> Result<(), AuthError>` produces 403.
  - **Probe:** `GET /admin/session` takes the `AuthPrincipal` extractor (→401 on
    no/invalid session), calls `require_role(.., Editor)` (→403; owner satisfies
    via implies-all), and echoes the `Principal` as JSON. The seam's first
    consumer + test surface — an API, not the admin shell UI (#42).
  - `Session` is represented server-side (the token→Principal binding in
    `SessionStore`); a wire `Session` DTO is deferred (the probe needs only
    `Principal`). Honors the ticket's "DTOs **or helpers**".
- **File manifest:**
  | # | File | Change |
  |---|---|---|
  | 1 | `crates/oathstar-protocol/src/lib.rs` | Add `AuthRole` enum (`#[serde(rename_all="snake_case")]` → player/editor/admin/dm/owner; Copy/Eq/Hash) + `Principal{id,name,roles}` (`camelCase`) + `Principal::grants(AuthRole)->bool` (owner-implies-all, `#[must_use]`) + `#[cfg(test)]` for `grants`. Doc all public items. |
  | 2 | `crates/oathstar-server/src/auth.rs` (NEW) | `SessionStore` (`from_owner_token(Option<String>)` pure ctor, `from_env()` thin wrapper, `resolve(&str)->Option<Principal>`); `AuthError`(+`IntoResponse` 401/403); `authenticate(&HeaderMap,&SessionStore)->Result<Principal,AuthError>`; `require_role(&Principal,AuthRole)->Result<(),AuthError>`; `AuthPrincipal(Principal)` extractor `impl FromRequestParts<AppState>`; `#[cfg(test)]` units. No `unwrap`/`expect` on the header path. |
  | 3 | `crates/oathstar-server/src/main.rs` | `mod auth;`; `AppState` += `auth_sessions: Arc<SessionStore>`; `main` seeds `Arc::new(SessionStore::from_env())` + adds `.route("/admin/session", get(admin_session))`; add `async fn admin_session(principal: AuthPrincipal) -> Result<Json<Principal>, AuthError>`; update **all 3** `AppState` construction sites (main + `test_app_state_with_saves` + the inline one in `spawn_tick_loop_broadcasts_ticks`); add auth integration tests to `mod tests`. |
  | 4 | docs (Phase 5) | Append **Decision 057** (v1 auth seam) to `decisions.md` (additive, EOF — won't disturb the user's uncommitted 056); document `OATHSTAR_DEV_OWNER` in `auth.rs` rustdoc. No edits to the user's other uncommitted docs. |
- ### Regression Test Plan
  | # | Test | Proves Requirement |
  |---|---|---|
  | T1 | protocol `Principal::grants` — player⊉editor; editor⊇editor; **owner⊇anything** (implies-all); admin⊉editor | REQ-002/003 |
  | T2 | auth `authenticate` — no `Authorization` header → `Err(Unauthorized)` | REQ-001 |
  | T3 | auth `authenticate` — malformed / non-`Bearer` header → `Err(Unauthorized)` | REQ-001 |
  | T4 | auth `authenticate` — unknown token → `Err(Unauthorized)`; known token → `Ok(Principal{id,name,roles})` | REQ-001/003 |
  | T5 | auth `require_role` — player → `Err(Forbidden)`; editor & owner → `Ok` | REQ-002 |
  | T6 | auth `AuthError::into_response()` — Unauthorized=**401**, Forbidden=**403** | REQ-001/002 |
  | T7 | auth `AuthPrincipal` extractor — built `Parts` w/o header → 401 rejection; with valid token+state → `Ok(principal)` | REQ-001/003 |
  | T8 | server `admin_session(AuthPrincipal(player))` → 403; `(editor)`/`(owner)` → 200 + JSON echoes id/name/roles | REQ-003/006 |
  | T9 | auth `SessionStore::from_owner_token(Some(tok))` resolves an owner principal; `(None)` → empty (token → 401) — the deterministic dev-owner, both arms | REQ-004 |
  | T10 | server probe matrix end-to-end (direct handler): no session → 401, player → 403, owner → 200 | REQ-006 |
  | T11 | server `state_snapshot(State(test_app_state()))` still 200 with **no** auth; the ~40 existing handler tests stay green (auth is per-route) | REQ-005 |
  - **Uncoverable / excluded:** `SessionStore::from_env()` env read (thin glue like
    `OATHSTAR_ADDR`/`OATHSTAR_SAVE_DIR`; the deterministic behavior is covered by
    the pure `from_owner_token`, T9). `fn main` route wiring is the
    mutation-excluded composition root.
- **Risks / decisions:**
  - **D-1 carrier:** Bearer token + in-memory `SessionStore`. Minimal/reversible
    (can become a cookie/real session later); accounts/persistence are out of v1.
  - **D-2 owner-implies-all** in `Principal::grants` (Decision 056). Documented +
    tested; admin ≠ editor (distinct powers).
  - **D-3 type home:** `AuthRole`/`Principal` in `oathstar-protocol` (the probe's
    JSON echo is a wire shape), named to avoid collision with core's entity `Role`.
  - **D-4 per-route opt-in** extractor, not a global layer → player routes
    untouched (REQ-005).
  - **D-5 dev-owner** = `OATHSTAR_DEV_OWNER`; prod unset → 401. Pure
    `from_owner_token` tested; `from_env` thin.
  - **R-1:** `AppState` gains a field → 3 construction sites updated (compile-checked).
  - **R-2:** this run validates with `--fast` (no mutation/coverage gate); tests are
    written to also satisfy the eventual FULL gate at the owner's later commit.
  - **R-3 guardrail:** Phase 5 doc capture appends Decision 057 additively at the
    EOF of the user's already-dirty `decisions.md` (won't touch their 056); no
    edits to their other uncommitted docs.
  - **Clippy(strict):** doc all public items, `#[must_use]` on `grants`, typed
    errors (no `unwrap`/`expect` on the header path), `AuthRole` derives Copy/Eq/Hash.

## Phase 3 — Implement
- **Built** (production only; tests = Phase 4, docs = Phase 5):
  - `crates/oathstar-protocol/src/lib.rs`: `AuthRole` enum (`snake_case` serde —
    player/editor/admin/dm/owner; `Copy`/`Eq`) + `Principal{id,name,roles}`
    (`camelCase`) + `Principal::grants` (owner-implies-all, `#[must_use]`).
  - `crates/oathstar-server/src/auth.rs` (NEW): `SessionStore`
    (`from_owner_token` pure ctor / `from_env` thin wrapper / `resolve`;
    `#[derive(Default)]` = empty); `AuthError{Unauthorized,Forbidden}` +
    `IntoResponse` → real **401/403** with a small `error` body;
    `authenticate(&HeaderMap,&SessionStore)` (Bearer header → `Principal` or
    `Unauthorized`); `require_role(&Principal,AuthRole)` (→ `Forbidden`);
    `AuthPrincipal` extractor `impl FromRequestParts<AppState>` (axum 0.8
    RPITIT — **no `async_trait`, no new deps**). No `unwrap`/`expect` on the
    header path.
  - `crates/oathstar-server/src/main.rs`: `mod auth;` + imports;
    `AppState.auth_sessions: Arc<SessionStore>`; `main` seeds
    `SessionStore::from_env()` + adds `.route("/admin/session", get(admin_session))`;
    `admin_session` handler (`require_role(Editor)?` then echo `Principal`);
    updated **all 3** `AppState` construction sites (main + the two `#[cfg(test)]`
    sites use `SessionStore::default()` — compile-needed field additions, not new
    test logic).
  - **Verified:** `cargo clippy -p oathstar-protocol -p oathstar-server
    --all-targets -- -D warnings` **CLEAN**; `cargo fmt --all --check` clean.
- **Deviations from design (+ reason):** none. The regression tests (T1–T11) are
  deferred to `/pipeline:validate` (implement skill forbids expanding tests
  here); the two test-construction-site edits are required for the existing test
  module to compile after `AppState` gained a field. Decision 057 doc capture is
  deferred to `/pipeline:complete`.

## Inspect (Phase 3.5)
- **Lenses run** (3 parallel critics over the auth diff):
  1. Security / auth-bypass (general-purpose; **empirically harness-verified** the
     env→store→resolve chain).
  2. Correctness / EARS + axum (general-purpose; ran compile + test-compile).
  3. Simplification / Rust-idiom (general-purpose; ran clippy `-D warnings`).
- **Findings:**
  | # | Severity | Finding (file) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | **critical → FIXED** | Empty `OATHSTAR_DEV_OWNER` (`=`) → `env::var().ok()` = `Some("")` → seeds owner under the `""` token → unauthenticated `Authorization: Bearer ` trims to `""` and resolves to **owner** (`auth.rs` `from_owner_token`/`from_env` + `authenticate`) | **REAL** — harness-proven; contradicts REQ-004 + the module's own "never weakens production" claim (set-but-empty ≠ unset) | **FIXED both arms:** `from_owner_token` now `.filter(`blank`)` so a blank token never seeds; `authenticate` rejects an empty post-trim token before `resolve`. Clippy re-clean. Phase-4 regression tests added to the plan. |
  | 2 | low | `.trim()` accepts whitespace-padded tokens | **Rejected** — only ever aids a legitimate token holder; the empty case is closed by #1's guard | none |
  | 3 | low | `Bearer` match is case-sensitive/space-strict (`bearer`/`BEARER` → 401) | **Rejected** — fails *closed* (no bypass); first-party client sends canonical `Bearer `; RFC-7235 case-insensitivity deferred to the real-auth ticket | none |
  | 4 | low | distinct 401 reasons = token-existence enumeration oracle | **Rejected** for v1 — one opaque dev token, negligible; messages aid dev debugging; noted for the real-auth ticket | none |
  | 5 | info | 401 omits `WWW-Authenticate: Bearer` | **Rejected** — no AC requires it; spec nicety for later | none |
  | 6 | low | "session" naming w/ no `Session` type; single-caller helpers | **Rejected** — `SessionStore` is a fine conceptual name; the `authenticate`(authn) vs `require_role`(authz) split is intentional seam design for #42 | none |
  | 7 | med | auth surface has **zero tests** | **Deferred to Phase 4** — the test plan (T1–T11) writes them; **adding** an empty-`OATHSTAR_DEV_OWNER` + `Bearer ` → 401 regression (T12). `--fast` doesn't gate cov/mutation this run, but tests are written per §7. | Phase 4 |
- **Positives confirmed:** all 6 REQs met (correctness critic; `cargo test --no-run` exit 0); minimal + idiomatic, clippy `-D warnings` clean (simplification critic); the extractor truly short-circuits before the body, refusals are real **401/403 not in-band-200**, player routes stay open (no global layer), and roles cannot be forged from client input (security critic).
- **Capture:** `failure-record` **BF-auth-empty-env-owner-bypass-001** + `prevention-rule` **PR-claude-blank-env-config-is-not-absent-001** (a real bug a critic caught — the value of the adversarial phase).

## Phase 4 — Validate
- **Tests added:**
  - `oathstar-protocol/src/lib.rs` `mod auth_tests` (T1 + serde): `grants`
    owner-implies-all + non-owner-only-own-roles (admin ≠ editor); `AuthRole`
    `snake_case` wire tokens; `Principal` `camelCase` round-trip.
  - `oathstar-server/src/auth.rs` `mod tests` (T2–T7, T9, **T12**): `authenticate`
    no-header / non-`Bearer` / unknown-token / known-token; **blank-bearer-token
    rejected** (the inspect bypass fix); `from_owner_token` seeds-owner +
    **ignores blank/None** (the env-empty fix, both arms); `require_role`
    grants-enforcement; `AuthError` → real **401/403** status.
  - `oathstar-server/src/main.rs` `mod tests` (T8, T10, T7, T11): `admin_session`
    forbids-a-player (**403**) / echoes-an-authorized-principal (editor + owner →
    **200**); `AuthPrincipal` extractor no-header → **401** / seeded-token →
    principal; player endpoints answer with **no auth** (REQ-005).
- `cargo test --workspace`: **ok** — 25 + 296 + 16 + 27 + 42 + 20 passed, **0
  failed** (server +12 auth tests, protocol +4). All 6 REQs + the inspect
  empty-token regression green; no regressions.
- `bin/gate.sh --fast`: **GATE GREEN [fast] — 14/14** (rustfmt, clippy-strict,
  cargo test, node --test, cargo-audit, cargo-deny, cargo-machete, gitleaks,
  shellcheck, no-suppressions, source-bans, lints-allowlist, doc-todos, tauri).
- **Pre-existing / unrelated exclusions: NONE.** gate:4 (`node --test`) ran the
  whole JS suite — including the unrelated dirty-worktree work (the UI edits to
  `client-app.js` and the user's `tests/elvgames-tileset.test.js`) — and it all
  passed; no unrelated reds surfaced. Coverage + mutation (gates 15–17) are
  FULL-only and deliberately deferred to the owner's eventual `/commit` (this run
  stops before commit, per instruction).
- **Beyond the design's T1–T11:** added an empty-`OATHSTAR_DEV_OWNER` /
  blank-`Bearer ` → unauthorized regression (T12) to lock the inspect-found
  auth-bypass fix (`BF-auth-empty-env-owner-bypass-001`).

## Phase 5 — Complete
- **Docs updated:** `docs/decisions.md` — **Decision 057** (the v1 auth seam),
  appended additively at EOF (the user's uncommitted Decision 056 untouched).
  `OATHSTAR_DEV_OWNER` is documented in `auth.rs` rustdoc. **No edits** to the
  user's other uncommitted docs (game-overview / team-handbook /
  technical-architecture) — guardrail respected.
- **Forge capture:**
  - `aar-submit` AAR `c2cf3454` → completed, effectiveness 4, 19 verdicts, 3
    novel findings; surfaced_used = arch-decision `9d063c49`.
  - `failure-record` **BF-auth-empty-env-owner-bypass-001** (`9e8faa0b`, at inspect).
  - `prevention-rule` **PR-claude-blank-env-config-is-not-absent-001** (`6dc89335`,
    at inspect).
  - `architecture-decision` **AD-claude-auth-seam-001** (`8bb590c5`).
- **Ticket closed:** forge #41 → `done`; local doc moved `open/` → `closed/`.
- **Archived:** spec + notes moved `pipeline/active/` → `pipeline/completed/`.
- **Delivered** (owner then said "continue and commit"): re-opened Phase 4 to add
  the three FULL-gate tests `--fast` had skipped — `from_env` env-read, the
  `.trim()` branch, and the non-UTF8 header arm — then ran the **FULL
  `bin/gate.sh` → GATE GREEN 17/17** (rust cov + js cov pass; **mutation MSI
  100.0%**, 408 caught / 0 missed).
- **Scoped commit** of only #41's files (the two Rust crates, `auth.rs`, this
  pipeline pair, the closed ticket). The unrelated dirty-worktree work — UI,
  ElvGames, architecture docs, #42/#43 — was **not** staged or committed, per the
  guardrail. **Decision 057 was left UNCOMMITTED** in `decisions.md` (it shares
  the file with the user's uncommitted Decision 056); it is captured in the forge
  (`AD-claude-auth-seam-001`) and will land when the owner commits their
  online-first doc batch.
