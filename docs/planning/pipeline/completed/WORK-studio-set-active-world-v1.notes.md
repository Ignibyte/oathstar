# WORK-studio-set-active-world-v1 — Notes

## Phase 1 — Plan
- **Request:** "Set as active world" — promote an authored map to the game's active slot, with a
  validity guard. Pivot item ② slice 1 (memory `studio-editable-world-pivot`).
- **Classification / tier:** work pipeline, one slice, **`oathstar-studio` only** + a pure JS fn.
  No game/engine change (#56 already loads the slot at startup).
- **Recon (main `1157fee`):**
  - Server: `ACTIVE_WORLD_FILE = "world.json"` (`oathstar-server/src/main.rs:381`), `resolve_world_path`
    (`:397`) → `ActiveSlot(maps_dir/world.json)` loaded at startup. The studio writes `maps/` via
    `FileSaveStore`, so `write_json("world", doc)` → `maps/world.json` (the same slot). Shared
    `maps/` (both default `OATHSTAR_MAPS_DIR`).
  - `editor.rs`: `validate` (`:88`) materializes via `document.materialize(&studio.catalog)` (`:100`);
    `save_map` writes via `studio.maps.write_json`; `refuse` (`:57`) + `editor_refusal` (`:72`) gate;
    `validate_save_slot_name` imported.
  - `SaveStore::write_json<T>(name, value) -> Result` (`oathstar-storage`): validates the slot name +
    rejects symlink targets — so `"world"` is safe.
  - `EDITOR_GLUE` (`render.rs:~575`): the `#save` button POSTs `/editor/maps`, `formatSaveResult`,
    `result.dataset.ok`; Validate POSTs `/editor/maps/validate`, `formatValidateResult`.
    `formatSaveResult`/`formatValidateResult` are **pure exports** in `editor-canvas.js` (`:203/:230`,
    node-tested) → `formatActivateResult` joins them.
- **Approach (design refines):** `set_active` = `editor_refusal` → parse (400) →
  `materialize` (`Err`→400 no-write / `Ok`→`write_json("world")`→200 `{active:"world"}`); route
  `POST /editor/maps/activate`; a "Set as active world" button + glue calling `formatActivateResult`.
- **EARS:** REQ-001 valid→write+200 · REQ-002 non-materializing→400 no-write · REQ-003 malformed→400
  · REQ-004 auth 401/403 · REQ-005 formatActivateResult · REQ-006 gate.
- **Mutation surface:** `set_active` branches (gate / parse-Err / materialize-Err / write-Err / Ok)
  + `formatActivateResult` — killed by REQ-001..005.
- **Ticket:** forge **#60** `057bcd35-bdae-4996-a557-755ee6434844`. Local doc
  `docs/planning/tickets/open/TICKET-60-studio-set-active-world.md`.
- **aar_id:** `747a265b-d2e4-4b68-b336-633ae2118abf`
- **Delivery:** AUTONOMOUS through commit+push+FF-merge (user re-granted 2026-06-19). Branch off
  `main` `1157fee`. Stash parked.

## Phase 2 — Design

### Code reconnaissance
- `editor.rs validate` (`:88`): `editor_refusal` gate → parse-or-`refuse(400)` →
  `match document.materialize(&studio.catalog) { Ok(world) => Json(Success{…}), Err(error) =>
  (StatusCode::OK, Json(Failure{ok:false,message,error:Some(error)})) }`. **`validate` returns the
  Err as `200`** (it answers "is this valid"); **`set_active` returns Err as `400`** (it refuses to
  promote) — the one deliberate divergence. `Failure` struct (`:32`) is reused as-is.
- Editor controls (`render.rs:629-632`): `<input id="map-name">`, `<button id="save">`,
  `<button id="validate">`, `<pre id="result">`. The Save glue (`#save`) POSTs `/editor/maps`,
  `formatSaveResult`, sets `result.textContent`/`result.dataset.ok`, then rewrites `?map=`.
- `formatSaveResult`/`formatValidateResult` (`editor-canvas.js:230/203`) return
  `{ok, headline, detail}` — `formatActivateResult` mirrors that shape.
- Editor page tests (`render.rs:713/742`) already assert `id="validate"`, `id="result"`,
  `<button id="save"`, `<input id="map-name"` — extend with `id="activate"`.

### Approach / architecture (oathstar-studio + 1 pure JS fn)
- **`editor.rs set_active(State(studio), jar, body: Bytes) -> Response`** — mirrors `validate`'s
  opening verbatim: `editor_refusal` gate; `serde_json::from_slice::<MapDocument>` or
  `refuse(BAD_REQUEST, "request body is not a valid map document")`; then
  `match document.materialize(&studio.catalog) { Ok(_world) => { if
  studio.maps.write_json("world", &document).is_err() { return refuse(INTERNAL_SERVER_ERROR,
  "failed to activate world"); } Json(Activated{ ok: true, active: "world" }).into_response() }
  Err(error) => (StatusCode::BAD_REQUEST, Json(Failure{ ok: false, message: error.to_string(),
  error: Some(error) })).into_response() }`. NEW `#[derive(Serialize)] struct Activated { ok: bool,
  active: &'static str }`.
- **`main.rs`** — `.route("/editor/maps/activate", post(editor::set_active))` beside the other
  `/editor/maps` routes (`post` already imported).
- **`render.rs`** — add `<button id="activate" type="button">Set as active world</button>` after
  `#validate`; update the hint to mention it. In `EDITOR_GLUE`, an `#activate` click handler that
  mirrors `#save` but POSTs the current doc to `/editor/maps/activate`, calls
  `formatActivateResult(json)`, sets `result.textContent`/`result.dataset.ok` — **no `?map=`
  rewrite** (activation isn't a save).
- **`editor-canvas.js`** — `export function formatActivateResult(resp)`: `resp.ok === true` →
  `{ ok:true, headline:"Active world set", detail:"Restart the game server to play it." }`; else
  `{ ok:false, headline:"Not activated", detail:(resp && resp.message) || "the world does not
  materialize" }`. Pure (no DOM).

### Locked decisions (this phase)
- **Err → 400** (refuse to promote), unlike `validate`'s 200. The fixed slot `"world"` is a literal
  (no caller input). `Activated.active` is `&'static str` (always `"world"`).
- **`Failure` reused** (not redefined); `materialize`/`refuse`/`editor_refusal` composed, no dup.
- **No new viable mutants:** `set_active` is all match / if-let / method-calls (no binary/comparison/
  `!` operators), and `Response` isn't `Default`, so cargo-mutants finds no viable target (same as
  `save_map`/`validate`). The bar is **coverage** — every arm must be exercised. `formatActivateResult`
  is JS (coverage only, not mutated).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-studio/src/editor.rs` | NEW `set_active` handler + `Activated` struct; tests (200+write / 400-no-write / 401 / 403). |
| 2 | `crates/oathstar-studio/src/main.rs` | route `POST /editor/maps/activate` → `set_active`. |
| 3 | `crates/oathstar-studio/src/render.rs` | `#activate` button + hint; `EDITOR_GLUE` activate handler; extend the editor-page test to assert `id="activate"`. |
| 4 | `crates/oathstar-studio/static/editor-canvas.js` | NEW pure `formatActivateResult`. |
| 5 | `tests/editor-canvas.test.js` | `formatActivateResult` cases (ok / refused). |
| 6 | `docs/map-system.md` | (Phase 5) active-world "Set as active" note. |

### Regression Test Plan
| # | Test | Proves |
|---|---|---|
| T1 | `set_active`(owner session, `STARTER_DOC` — ref-free, materializes) → `200`, body `active:"world"`, **and** `studio.maps.read_json::<MapDocument>("world")` round-trips | REQ-001 (rust, temp maps) |
| T2 | `set_active`(owner, a parse-OK but **non-materializing** doc — a room citing an undeclared region) → `400`, **and** `read_json("world")` is `Err` (no file written) | REQ-002 |
| T3 | `set_active`(owner, body `b"not a document"`) → `400` | REQ-003 |
| T4 | `set_active`(no cookie) → `401`; `set_active`(Player session) → `403`; neither writes `world` | REQ-004 |
| T5 | `formatActivateResult({ok:true})` → `{ok:true, headline:"Active world set"}`; `formatActivateResult({ok:false,message:"x"})` → `{ok:false, headline:"Not activated", detail:"x"}` | REQ-005 (node) |
| T6 | editor page HTML contains `id="activate"` | REQ-001 wiring (rust, extend existing page test) |
| G1 | `bin/gate.sh` FULL green, MSI 100% | REQ-006 |
- Each rust test uses the existing `studio()` temp-maps helper (atomic-seq dir) + `owner_principal`/
  `jar`. **Genuinely uncoverable:** the `write_json(...).is_err()` → 500 arm (write fails only on a
  real fs error) — same as `save_map`'s 500 arm; not a mutants target (no operator; `Response` ≠
  `Default`), so it neither breaks MSI nor coverage floors (it's an early-return statement, not a
  branch line counted against the 94% — matching the existing handler).

### Risks / decisions
1. **Activation ≠ save.** `set_active` writes only the `world` slot; the doc's own id is untouched.
   If the owner wants both, they Save (its id) and then Set-active (→ `world`). Documented in copy.
2. **Startup-load.** The activated world loads on the next game-server start (no hot-reload) — the
   button copy says "Restart the game server". The running game server is the pre-#56 build, so I'll
   rebuild+restart it at validate/demo to exercise the loop.
3. **Non-materializing fixture (T2).** Construct a minimal doc that parses but fails `materialize`
   (a room whose `region` isn't declared — mirrors `oathstar-content`'s `refuses_undeclared_region`).

## Phase 3 — Implement
- **Built (manifest as designed):**
  - `editor.rs` — `Activated { ok, active: &'static str }` struct + `set_active` handler
    (`editor_refusal` → parse-or-`refuse(400)` → `materialize`: `Ok` → `write_json("world")` or
    `refuse(500)` → `200 {ok:true, active:"world"}`; `Err` → `400 {ok:false,message,error}`,
    **no write**).
  - `main.rs` — `POST /editor/maps/activate` → `set_active`, placed **before** `/editor/maps/{id}`
    so the static route wins over the `{id}` param (axum matchit prioritises static, but ordering
    keeps it obvious).
  - `render.rs` — `<button id="activate">Set as active world</button>` after `#validate`; hint
    updated; `EDITOR_GLUE` activate handler mirrors `#save` (POST `/editor/maps/activate`,
    `formatActivateResult`, no `?map=` rewrite). The glue references `formatActivateResult` directly
    (it's concatenated into the same module as `editor-canvas.js` — no import line).
  - `editor-canvas.js` — pure `formatActivateResult` (`{ok,headline,detail}`, sibling of
    `formatSaveResult`).
- **Deviations:** (1) `cargo fmt` wrapped `set_active`'s signature onto multiple lines (cosmetic).
  (2) The node test file is **`tests/studio-editor-canvas.test.js`** (not `editor-canvas.test.js` as
  the design named) — Phase 4 adds the `formatActivateResult` cases there.
- **Checks:** `cargo check`/`clippy -p oathstar-studio --all-targets` clean; `cargo fmt` clean;
  `cargo test -p oathstar-studio` → **86 passed**; `node --test tests/*.test.js` → **88 passed**;
  `node --check editor-canvas.js` OK. New tests + gate at Phase 4.

## Inspect (Phase 3.5)
- **Lenses:** 2 read-only `Explore` critics (no worktree mutation): correctness/security +
  simplification/reuse.
- **Both critics — implementation CLEAN.** Verified: (a) **materialize guard** — `write_json` is
  reached **only** on the `Ok` arm; a non-materializing doc returns `400 {ok:false,message,error}`
  and writes nothing; (b) **fixed slot** — `write_json("world", …)` is a hardcoded literal, no
  request input reaches the slot name (and `write_json` validates the name + rejects symlinks); (c)
  **auth gate** early-returns 401/403 before any parse/materialize/write; (d) **no panics** (guarded
  `let Ok(...) else`, `.is_err()`); (e) **route precedence** — `POST /editor/maps/activate` reaches
  `set_active` (matchit prioritises the static segment over `{id}`, and `{id}` is GET-only — no
  collision); (f) **reuse** — composes `editor_refusal`/`refuse`/`materialize`/`Failure`, `Activated`
  is a minimal new struct; `formatActivateResult`/glue mirror the Save siblings (pure, no `?map=`
  rewrite). The `400` (vs `validate`'s `200`) on a non-materializing doc is **intentional** (refuse to
  promote) — per spec.
- **Findings — all "missing tests", i.e. the known Phase 4 scope (NOT Phase 3 defects):** both
  critics flagged that `set_active` has no rust tests, the editor page has no `id="activate"`
  assertion, and `formatActivateResult` has no node test. **Correct — these are exactly the planned
  Regression Test Plan rows T1–T6**, which `/pipeline:validate` writes. Reconfirmed Phase 4 must add:
  rust `set_active` (200+write / 400-no-write / 401 / 403), an editor-page wiring assertion
  (`id="activate"` + `formatActivateResult(`), and node `formatActivateResult` cases in
  `tests/studio-editor-canvas.test.js`. The 500-write-fail arm is not a mutants target (no operator;
  `Response` ≠ `Default`, like `save_map`).
- **Nits rejected:** (1) "the glue doesn't check `res.ok` before parsing" — fine + consistent with
  `#save`: the `400` body is still valid `Failure` JSON, so `formatActivateResult` renders the
  refusal (`ok:false`). (2) "mention restart in the hint" — the button **feedback** already says
  "Restart the game server to play it."; the hint stays concise. **No code fix; no `failure-record`
  (no defect).**

## Phase 4 — Validate
- **Tests added** (T1–T6, all the inspect-confirmed rows):
  - `editor.rs` — `call_activate` helper + **T1** `set_active_promotes_a_materializing_document`
    (`VALID_DOC` → 200 `active:"world"` + the `world` slot round-trips), **T2**
    `set_active_refuses_a_non_materializing_document` (`BAD_TILE_DOC` parses but fails materialize →
    400 + `read_json::<MapDocument>("world")` is `Err` — no write), **T3**
    `set_active_refuses_a_malformed_body` (→ 400), **T4** `set_active_refuses_non_editor_callers`
    (anon → 401, Player → 403, neither writes). Added `set_active` + `SaveStore` to the test imports.
  - `render.rs` — **T6** `editor_page_wires_the_activate_control` (`<button id="activate"`,
    `getElementById("activate")`, `fetch("/editor/maps/activate"`, `formatActivateResult(`).
  - `tests/studio-editor-canvas.test.js` — **T5** `formatActivateResult` (ok → "Active world set" +
    restart detail; refusal → "Not activated" + surfaces `message`; missing message + `null`/
    `undefined` fall back; non-strict `ok` is a failure).
- **`cargo test --workspace`:** green — `oathstar-studio` **91 passed** (+5: T1–T4 + T6); all other
  crates green.
- **`node --test tests/*.test.js`:** **89 passed / 0 fail** (+1: `formatActivateResult`).
- **`bin/gate.sh` FULL:** **GATE GREEN [full]** — 17/17. rustfmt, clippy strict, both suites, rust
  cov ≥94, js cov 90.08%, **mutation 594 caught / 0 missed → MSI 100.0%**. Receipt written.
- **Pre-existing exclusions:** none.

## Phase 5 — Complete
- Docs / forge / ticket / archived:
