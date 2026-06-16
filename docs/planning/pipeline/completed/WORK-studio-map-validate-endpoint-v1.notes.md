# WORK-studio-map-validate-endpoint-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** an owner/editor-gated JSON endpoint in `oathstar-studio` that accepts a
  posted `MapDocument`, runs validate + materialize against a server-built
  `ContentCatalog`, and returns a typed summary/error. Owner delegated the pick
  ("dealer's choice"); auto-approved through commit.
- **Intake source:** `INTAKE-online-first-multiplayer-and-auth-gated-studio` (sections D
  "/admin/editor canvas shell" + F "validate and publish" — this is the backend slice
  that precedes the canvas UI). Intake left **unedited** (preserve guardrail; #41–#43
  likewise).
- **Classification / tier:** work pipeline, type=feature, one shippable slice (one
  Editor-gated route + an additive serde derive + in-crate tests).
- **Forge recall (anchors):**
  - Map model: `oathstar_content::{MapDocument, ContentCatalog, MapValidationError,
    validate, materialize}` (#43, `AD-claude-map-document-model-001`).
    `MapValidationError` is **not** serde-serializable yet → additive `Serialize` (or a
    response DTO) needed.
  - Studio gate to mirror: `handlers::dashboard` (oathstar-studio/src/handlers.rs:50-58 —
    `principal_from_cookie` → `require_role(Editor)` → else redirect/401); `StudioState`
    holds the `SessionStore`; handler-direct tests (CookieJar via `from_request_parts`,
    body via `axum::body::to_bytes`). An Owner session grants Editor (#42 tests).
  - Catalog source: `oathstar_content::load_beginner_world()` → `WorldDefinition` whose
    `entities`/`items` BTreeMaps seed the catalog (fixtures empty); design confirms.
  - Prevention rules in force: tests in-crate (`BF-studio-cross-crate-mutation-gap-001`);
    no unreachable defensive branches (`PR-claude-unreachable-defensive-branch-mutants-001`).
- **Ticket:** forge `e7c7a0d4-db81-4b91-9cf8-4546ebdcd6c6` (#44); local
  `docs/planning/tickets/open/TICKET-44-studio-map-validate-endpoint-v1.md`.
- **EARS requirements reviewed:** REQ-001 (gate), REQ-002 (valid → summary), REQ-003
  (invalid → typed error JSON naming the cell/ref), REQ-004 (malformed body → typed
  refusal, no panic), REQ-005 (renderer-agnostic JSON, no leakage, deterministic).
- **AAR id:** 07a75623-32bc-489c-9d85-56e3dde2c3d0

## Phase 2 — Design

**Studio surface (read):** `StudioState { sessions: SessionStore, owner_secret: Option<String> }`
(Clone) in main.rs; router inline in `main()` (mutation-excluded): `/`, `/login`,
`/logout` + `.with_state(state)`. `handlers::dashboard` gates via
`principal_from_cookie(&jar,&studio.sessions)` → `require_role(&p, AuthRole::Editor)`
→ else redirect. Handler-direct tests build `CookieJar` via `from_request_parts`, read
bodies via `axum::body::to_bytes`. Studio deps lack `oathstar-content`/`serde_json`.

### Approach / architecture
A new Editor-gated JSON endpoint. The gate is **reused** (`oathstar_auth::{principal_from_cookie,
require_role, AuthRole}`) — not reimplemented. For an API (vs the browser dashboard) the
refusal is a JSON status, not a redirect.

- **Body as `axum::body::Bytes`** (last extractor) parsed with `serde_json::from_slice::<MapDocument>`
  — makes malformed-body handling **handler-direct testable** (pass arbitrary bytes), no
  `JsonRejection`/tower needed.
- **Catalog once at startup:** `StudioState` gains `catalog: Arc<ContentCatalog>`, built in
  `main()` from `oathstar_content::beginner_catalog()?` (the fallible load lives in the
  mutation-excluded `main`; cheap Clone via `Arc`). The handler reads `&studio.catalog` —
  **no per-request load, no unreachable error branch.**
- **Handler `editor::validate(State<StudioState>, CookieJar, Bytes) -> Response`:**
  1. gate: `principal_from_cookie` → else `401 {ok:false,message}`; `require_role(Editor)`
     err → `403 {ok:false,message}` (an Owner session grants Editor — #42).
  2. parse: `serde_json::from_slice(&bytes)` err → `400 {ok:false,message}` (no panic).
  3. `doc.materialize(&studio.catalog)`: `Ok(world)` → `200 {ok:true, room_count,
     region_count, start_room_id}`; `Err(e)` → `200 {ok:false, message: e.to_string(),
     error: <serialized e>}`.
- **Response DTOs (studio):** `Success { ok:true, room_count, region_count, start_room_id }`
  and `Failure { ok:false, message, #[serde(skip_serializing_if="Option::is_none")] error:
  Option<MapValidationError> }` (auth/malformed → `error:None`; validation → `error:Some`).
- **Additive serde in oathstar-content:** derive `Serialize` on `MapValidationError` + `RefKind`
  (`Cell` already has it); the `WorldInvalid(WorldValidationError)` field uses
  `#[serde(serialize_with = …)]` to emit the inner error's Display string (WorldValidationError
  is not Serialize, and we will NOT modify oathstar-core). `message` (Display) always names the
  offending cell/ref (proven in #43) — REQ-003's reliable carrier; the structured `error` is the
  bonus.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | crates/oathstar-content/src/map_document.rs | MODIFY — derive `Serialize` on `RefKind` + `MapValidationError`; `serialize_with` on the `WorldInvalid` field (inner → Display string); + serde tests |
| 2 | crates/oathstar-content/src/lib.rs | MODIFY — `pub fn beginner_catalog() -> anyhow::Result<ContentCatalog>` (entities/items from `load_beginner_world()`, fixtures empty); + test |
| 3 | crates/oathstar-studio/Cargo.toml | MODIFY — add `oathstar-content` dep + `serde_json` dep; ensure axum `json` feature (workspace; add `features=["json"]` only if needed) |
| 4 | crates/oathstar-studio/src/editor.rs | NEW — `validate` handler + `Success`/`Failure` DTOs + response helpers; in-crate handler-direct tests |
| 5 | crates/oathstar-studio/src/main.rs | MODIFY — `mod editor;`; `StudioState += catalog: Arc<ContentCatalog>` built via `beginner_catalog()?`; route `POST /editor/maps/validate` |
| 6 | docs/map-system.md | MODIFY (Phase 5) — note the studio validate endpoint under "Map Document Model" |

### Regression Test Plan
| # | Test | Proves |
|---|---|---|
| T1 | `validate_refuses_without_an_editor_session` — anon → 401 `{ok:false}`; Player session → 403; neither materializes | REQ-001 |
| T2 | `validate_admits_an_owner_session` — Owner session (grants Editor) → 200 | REQ-001 |
| T3 | `validate_summarizes_a_valid_document` — Editor + valid MapDocument bytes → 200 `{ok:true, room_count, region_count, start_room_id}` (assert values) | REQ-002 |
| T4 | `validate_reports_an_invalid_document_naming_the_cell` — Editor + invalid doc (room on blocked terrain / undeclared region) → 200 `{ok:false}`, `message` names the cell/ref, `error` present | REQ-003 |
| T5 | `validate_rejects_a_malformed_body` — Editor + garbage bytes → 400 `{ok:false}`, no panic | REQ-004 |
| T6 | `success_and_failure_carry_the_ok_flag_without_leakage` — `ok` true/false asserted; response has only the documented fields | REQ-005 |
| T7 (oathstar-content) | `map_validation_error_serializes_naming_the_cell` — e.g. `RoomRegionMissing` `serde_json` → carries `cell` + ids | REQ-003/005 |
| T8 (oathstar-content) | `world_invalid_serializes_as_its_message` — `WorldInvalid` → the Display string | REQ-005 |
| T9 (oathstar-content) | `beginner_catalog_has_expected_content` — `beginner_catalog()` Ok + contains known beginner entity/item ids | REQ-002 |

No JS tests (no client change). Genuinely-uncoverable: `beginner_catalog()`'s error
propagation (`load_beginner_world()?`) — the embedded module always parses, so the Err
path is unreachable; `main()`'s `beginner_catalog()?` is mutation-excluded. T9 pins the Ok
path; the `?` is not an explicit hand-written branch.

### Risks / decisions (resolved)
1. **Route + status:** `POST /editor/maps/validate`; **200 with an `{ok}` flag for both
   validation outcomes** (the HTTP call succeeded; document validity is application-level),
   reserving 401/403 for auth, 400 for malformed body, 500 for a (practically-impossible)
   catalog-load failure — none of which require `unwrap`/panic.
2. **Error JSON:** derive `Serialize` on `MapValidationError`+`RefKind`; `WorldInvalid` inner
   via `serialize_with` → Display string. **No oathstar-core change.** `message` (Display) is
   the reliable cell/ref namer; structured `error` is the bonus.
3. **Catalog at startup in `StudioState` (`Arc<ContentCatalog>`)**, built in `main()` — memoized
   + keeps the fallible load out of the handler (no unreachable branch). Per-request load and a
   handler-side 500 arm were rejected for that reason.
4. **Body as `Bytes` + `serde_json::from_slice`** — handler-direct malformed testing; avoids
   `JsonRejection`/tower (consistent with #42's handler-direct tests).
5. **New `editor.rs` module** (cohesion); test helpers mirror handlers.rs (test-only
   duplication is harmless to coverage/mutation).
6. **Owner grants Editor** — the `OATHSTAR_OWNER_PASSWORD` login can use the endpoint (T2).
7. **No game-server / protocol / engine / client change**; oathstar-content change is the
   additive serde derive + `beginner_catalog()` only (validate/materialize behavior unchanged).

## Phase 3 — Implement
- **Built** (production only; tests are Phase 4):
  - `oathstar-content/src/map_document.rs` — `Serialize` on `RefKind` + `MapValidationError`;
    the `WorldInvalid` field serializes via a free `serialize_world_error` fn (Display string —
    no oathstar-core change). validate/materialize behavior unchanged.
  - `oathstar-content/src/lib.rs` — `pub fn beginner_catalog() -> anyhow::Result<ContentCatalog>`
    (entities/items moved from `load_beginner_world()`; fixtures empty); imported `BTreeSet`.
  - `oathstar-studio/Cargo.toml` — `+ oathstar-content`, `+ serde_json` (axum `json` is a
    default feature of axum 0.8 — confirmed via oathstar-server, no feature change).
  - `oathstar-studio/src/editor.rs` (NEW) — `validate(State, CookieJar, Bytes) -> Response`:
    gate (401/403) → `serde_json::from_slice` parse (400) → `materialize(&studio.catalog)` →
    `200 Success` | `200 Failure`; `Success`/`Failure` serde DTOs + a `refuse` helper. Every
    branch is reachable (gate-none / gate-not-editor / parse-err / materialize-ok /
    materialize-err).
  - `oathstar-studio/src/main.rs` — `mod editor;`; `StudioState += catalog: Arc<ContentCatalog>`
    built once in the mutation-excluded `main()` via `beginner_catalog()?`; route
    `POST /editor/maps/validate`.
- **Deviations from design (+ reason):**
  1. Also updated `oathstar-studio/src/handlers.rs`'s `#[cfg(test)] studio()` helper to set the
     new `catalog` field (an empty `ContentCatalog::default()`) — a **compile-necessary**,
     test-only change caused by the `StudioState` field addition. Still in-scope (oathstar-studio
     test code); the `enforce-tests-ran`/Phase-4 work is unaffected.
  2. No axum feature change needed (json is default in axum 0.8).
- `cargo clippy -p oathstar-content -p oathstar-studio --all-targets` **clean** (strict lints);
  `cargo fmt` applied. No engine/server/protocol/client change; owner's unrelated worktree untouched.

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel general-purpose critics + skeptical review): security + correctness;
  serde + API + no-leakage + mutation-readiness. Both ran `cargo check`/`clippy` and read the auth
  seam, the serde surface, and axum/serde_json internals.
- **Findings:**

  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | low | Pre-gate body buffering — `body: Bytes` (editor.rs) is buffered before the 401/403 gate runs | **ACCEPTED (no fix)** — bounded by axum 0.8's default **2 MB** body limit (verified in force; no `DefaultBodyLimit::disable`) AND the studio binds loopback-only (127.0.0.1:7879). Negligible for an owner-only sidecar. Future hardening: a pre-body auth extractor or `DefaultBodyLimit::max(N)`. |
  | 2 | nit | `Failure.message` (Display) duplicates the structured `error` on a validation failure (editor.rs) | **REJECTED** — intentional (design Risk #2): `message` is the reliable cell/ref namer, `error` the structured bonus. |
- **Verified FINE:** gate order correct — no non-editor path reaches `serde_json::from_slice`/`materialize`
  (both gate arms `return` early); Owner grants Editor (`Principal::grants` short-circuits on Owner), so the
  owner login uses the endpoint; **zero panics on the request path** (serde_json is panic-free incl. a
  128-deep recursion cap → 400, no stack overflow; `materialize` is total; no unwrap/expect/index in
  editor.rs); response semantics correct (valid→200 ok:true summary; invalid→200 ok:false+error); catalog
  is the startup `Arc<ContentCatalog>` (deref coercion correct); **every `MapValidationError` variant
  serializes** (`serialize_world_error` flattens the non-Serialize `WorldInvalid` inner to its Display
  string — no panic/recursion); **no leakage** (Success projects only ok + 3 fields; the full
  `WorldDefinition` is not returned); `beginner_catalog()` correct (entities/items from the world, fixtures
  empty); **no unreachable/defensive branches** (all 5 `validate()` arms reachable — the #43 PR rule held).
- **Carried to Phase 4 (mutation kill-list):**
  - **editor.rs (5 tests, DISTINCT counts — e.g. 2 rooms / 1 region / start "alpha"):** 401 (no cookie →
    `UNAUTHORIZED` + "authentication required" + `ok:false`); 403 (Player session → `FORBIDDEN` + "editor
    role required"); 400 (garbage bytes → `BAD_REQUEST` + "request body is not a valid map document");
    200-ok (valid → `OK` + `ok:true` + room_count=2 + region_count=1 + start_room_id="alpha" + NO `error`
    key); 200-err (e.g. `tile_size=32` → `OK` + `ok:false` + `message` names the offender + `error`
    present). Owner-admitted positive too.
  - **oathstar-content (3 tests):** `serialize_world_error` via `serde_json::to_value(WorldInvalid(..))` →
    exact `{"WorldInvalid":"materialized world is invalid: …"}` string; `beginner_catalog()` → known ids
    (e.g. mara/bell_eater) present + `fixtures.is_empty()`; a representative variant serde-shape pin
    (`UnknownReference` → exact JSON incl. `"kind":"Entity"` capitalized) to freeze the API contract.
- **Post-review:** no source change needed; clippy + fmt clean; no failure-record (no real bug found).

## Phase 4 — Validate
- **Tests added (9, all in-crate):**
  - `oathstar-studio/src/editor.rs` (6, handler-direct with JSON-literal bodies): `refuses_an_anonymous_caller`
    (401 + "authentication required"); `refuses_a_non_editor` (Player → 403 + "editor role required");
    `summarizes_a_valid_document_for_an_editor` (200 + ok:true + room_count=2 + region_count=1 +
    start_room_id="alpha" + no `error` key); `an_owner_session_is_admitted` (Owner grants Editor → 200);
    `rejects_a_malformed_body` (garbage → 400 + "request body is not a valid map document");
    `reports_an_invalid_document_with_a_named_error` (tile_size=32 → 200 + ok:false + message "tile size
    32 is unsupported" + `error.UnsupportedTileSize.found == 32`).
  - `oathstar-content` (3): `world_invalid_serializes_as_the_inner_message` (corrected expected —
    `{"WorldInvalid":"start room 's' does not exist"}`, the **inner** Display, not the wrapper's prefixed
    one); `unknown_reference_serializes_with_named_fields` (pins `RefKind` `"Entity"` + external tag +
    field names); `beginner_catalog_mirrors_the_world_content` (entities/items == the world's, fixtures empty).
- `cargo test --workspace`: **ok** — 0 failed across all crates (296 core + 65 content + 20 studio + 34 + 27 + …).
- `node --test tests/*.test.js`: **ok** — 67 pass / 0 fail.
- `bin/gate.sh` (FULL): **GATE GREEN [full]** — 17/17 PASS; mutation **MSI 100.0% (471 caught / 0 missed)**;
  rust cov ≥94; js cov ≥75; `REAL_GATE_EXIT=0`. `cargo fmt` applied pre-gate (gate:1 clean on the first run).
- Pre-existing exclusions: none.

## Phase 5 — Complete
- **Docs updated:** `docs/map-system.md` — added the "Served by the studio (ticket #44)" note to
  the Map Document Model section (`POST /editor/maps/validate`). `decisions.md` intentionally NOT
  touched (owner's uncommitted 056/057/058 — preserve guardrail); the decision is recorded here +
  in the forge AAR.
- **Forge capture:** AAR `07a75623` submitted (outcome completed, effectiveness 5). The
  `architecture-decision-record` tool **could not be invoked** — its multi-field call repeatedly
  rejected every parameter after the first ("missing field" on title/decision regardless of
  content or ordering; a recurring tool-call parse glitch, also hit on #43). The decision is
  captured here instead:
  > **AD-claude-studio-json-endpoint-001 — Studio Editor-gated map-document validate endpoint.**
  > `oathstar-studio` serves `POST /editor/maps/validate`: Editor-gated (reusing the dashboard
  > `principal_from_cookie` + `require_role` seam; an Owner session grants Editor), it parses a
  > `MapDocument` from the request body bytes via `serde_json`, materializes it against a
  > `ContentCatalog` built ONCE at startup into `StudioState` (an `Arc`, from `load_beginner_world`),
  > and returns HTTP 200 with an `ok` flag for both validation outcomes (a summary of
  > room/region counts + `start_room_id` on success; the typed `MapValidationError` as JSON naming
  > the offending cell/ref on failure), reserving 401/403/400 for auth and malformed body.
  > `MapValidationError` gained an additive serde `Serialize` in oathstar-content (the non-Serialize
  > `WorldInvalid` inner flattened to its Display via `serialize_with`) — no oathstar-core change.
  > **Rationale:** one reused auth seam; the catalog built in the mutation-excluded `main()` avoids
  > an unreachable handler error branch (the unreachable-defensive-branch rule); body-bytes +
  > `serde_json` keeps malformed handling panic-free and handler-direct testable without tower;
  > 200-with-`ok`-flag separates the validation outcome from transport/auth status; `serialize_with`
  > keeps the change additive within oathstar-content.

  No failures/prevention-rules to record — inspect found no real defects.
- **Ticket closed:** forge #44 (`e7c7a0d4`) → done.
- **Archived:** pipeline pair → `completed/`; TICKET-44 → `closed/`.
