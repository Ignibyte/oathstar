# WORK-persist-authored-worlds-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** S1 of the region-authoring program — persist authored worlds and
  load them in the game (the keystone authoring loop). Studio saves an editable
  `MapDocument`; the game loads an authored world via `materialize()` at startup.
- **Intake source:** `docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md`
  (program intake covering S1–S5; this is **S1**).
- **Classification / tier:** work pipeline, one slice. Cross-cutting: studio
  (persistence endpoints), content (`MapDocument` persistence + reuse
  `materialize()`), server/engine (runtime authored-world load), storage (reuse
  `FileSaveStore`).
- **Forge recall (lessons/precedent surfaced):**
  - `WORK-save-load-v1` — the hardened `FileSaveStore` + `validate_save_slot_name`,
    atomic writes, and **load = UNTRUSTED input** (typed errors, re-validate, loud
    version rejection, no panics). S1 reuses this posture for `MapDocument` files.
  - `docs/module-system.md` — first boundary: official modules, **local file
    loading, manifest required, core-validated, no hot-loading mid-save**. Matches
    S1 (restart-load, core-validated via `materialize`).
  - TICKET-2 (`harden-core-world-init`) — core owns the invariant boundary;
    `materialize()` → `WorldDefinition::validate()` already enforces it.
  - Decision 058 — studio is loopback + Editor-gated; the save endpoint must stay
    behind that gate and write only inside an owned dir.
  - Path-safety prevention rules surfaced (cd01df21, ef08be37, 12f6e22e, 589d248c)
    — apply `FileSaveStore`'s traversal/symlink/reserved-name defenses.
- **Code anchors (from Explore; verify at design):** `MapDocument`
  `crates/oathstar-content/src/map_document.rs:245`; `materialize()` ~`:772`;
  studio validate endpoint `crates/oathstar-studio/src/editor.rs:54` (doc currently
  discarded); studio world load `crates/oathstar-studio/src/main.rs:~47`; server
  world load `crates/oathstar-server/src/main.rs:~95`; TOML loader
  `crates/oathstar-content/src/lib.rs:126`; `FileSaveStore`
  `crates/oathstar-storage/src/lib.rs:126`.
- **Ticket:** forge `9d39d561-de36-494a-93b5-cb2b7ce81698` (#53); local doc
  `docs/planning/tickets/open/TICKET-53-persist-authored-worlds.md`.
- **EARS reviewed:** REQ-001..008 in the spec (save persists; list/reopen;
  Editor-gated; path-safe; game load via materialize; untrusted-load typed errors;
  beginner unaffected; full gate green).
- **aar_id:** `f55ca2e5-ec6f-499e-8300-34e24745c6f9`
- **Branch / WIP:** will create `ticket-53-persist-authored-worlds` off `c30d549`
  before implement; online-first WIP stays stashed (`stash@{0}`); the region-program
  intake doc is currently untracked (commit with this slice).

## Phase 2 — Design

**Approach / architecture (high reuse — "reuse before adding", §14):**
- **Persistence = the EXISTING `FileSaveStore`.** Its `write_json<T: Serialize>` /
  `read_json<T: DeserializeOwned>` are already generic, path-safe
  (`validate_save_slot_name`: 64-char ASCII, no separators/`..`/reserved names),
  symlink-guarded, and atomic (.tmp→rename) — `crates/oathstar-storage/src/lib.rs:126`.
  `MapDocument` derives `Serialize + Deserialize` (`map_document.rs:246`), so it
  round-trips as-is. The ONLY new storage code is `FileSaveStore::list()`. The
  studio holds a second `FileSaveStore` rooted at a maps dir (separate from game
  saves).
- **Studio (author + persist):** add `maps: FileSaveStore` to `StudioState`
  (`studio/main.rs:28`), rooted at `OATHSTAR_MAPS_DIR` (default `maps`). Add three
  **Editor-gated** handlers mirroring `editor::validate` (`editor.rs:60`):
  `principal_from_cookie` → `require_role(Editor)` first, every time. `POST
  /editor/maps` = parse `MapDocument` (serde) + `write_json(id, &doc)` (drafts
  allowed — NO materialize gate at save; the existing `/editor/maps/validate` gives
  feedback). `GET /editor/maps` = `maps.list()`. `GET /editor/maps/:id` =
  `read_json::<MapDocument>` → JSON. Minimal editor reopen: the editor page accepts
  `?map=<id>`, fetches the load endpoint, loads it into the document (richer
  saved-maps browser UI is S2).
- **Game (load authored world):** all world-loading lives in `oathstar-content`
  (fully unit-testable, server stays thin):
  - `load_authored_world(path, catalog: &ContentCatalog) -> Result<WorldDefinition,
    AuthoredWorldError>` — read file → `serde_json` `MapDocument` → `materialize()`.
    Content is **UNTRUSTED**: typed errors (`Read`/`Parse`/`Materialize`), no panics.
  - `load_startup_world(authored: Option<&Path>) -> Result<WorldDefinition,
    StartupWorldError>` — `None` → `load_beginner_world()` (unchanged); `Some(p)` →
    `load_authored_world(p, &beginner_catalog()?)`.
  - Server (`server/main.rs:95`): read `OATHSTAR_WORLD` (blank-≠-unset guard) →
    `content::load_startup_world(path.as_deref())?` → `Engine::try_new`. **Unset →
    beginner unchanged. Set+valid → authored. Set+invalid → typed error → loud
    startup failure (no panic, NO silent fallback).**
- **Catalog for materialize = `beginner_catalog()`** — authored maps reference the
  existing entity/item catalog (entities/items/oaths authoring is OUT this slice).

**File manifest:**
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-storage/src/lib.rs` | Add `FileSaveStore::list(&self) -> anyhow::Result<Vec<String>>` (read root, collect validated `*.json` slot basenames, sorted; tolerate a missing root → empty). |
| 2 | `crates/oathstar-content/src/lib.rs` (+ maybe a small `authored.rs`) | Add `AuthoredWorldError` + `StartupWorldError` (typed, `thiserror`/manual Error); `load_authored_world(path, &ContentCatalog)`; `load_startup_world(Option<&Path>)`. Re-export from crate root. |
| 3 | `crates/oathstar-studio/src/main.rs` | Add `maps: FileSaveStore` to `StudioState` (root `OATHSTAR_MAPS_DIR`, default `maps`); register `POST /editor/maps`, `GET /editor/maps`, `GET /editor/maps/:id`. |
| 4 | `crates/oathstar-studio/src/editor.rs` | Add `save_map` / `list_maps` / `load_map` handlers, Editor-gated (mirror `validate`); save parses+persists, list returns ids, load returns the doc JSON. |
| 5 | `crates/oathstar-studio/src/render.rs` (embedded editor HTML/JS) | Minimal reopen hook: `?map=<id>` → fetch `GET /editor/maps/:id` → load into the editor document. Thin. |
| 6 | `crates/oathstar-server/src/main.rs` | Read `OATHSTAR_WORLD`; swap `load_beginner_world()?` for `content::load_startup_world(path.as_deref())?` at startup. |
| 7 | tests (inline `#[cfg(test)]` in the above crates) | Per the test plan below. |

### Regression Test Plan
| # | Test (location) | Proves Requirement |
|---|---|---|
| T1 | storage: `list()` returns saved slot names, ignores `.tmp`/non-json, empty when root absent | REQ-002 |
| T2 | storage: a path-unsafe map name (`../`, separators, reserved, >64) is refused by `write_json`/`validate_save_slot_name` | REQ-004 |
| T3 | studio: `POST /editor/maps` then `GET /editor/maps/:id` round-trips a `MapDocument` byte-faithfully; `GET /editor/maps` lists it | REQ-001, REQ-002 |
| T4 | studio: save/list/load reject anonymous (401) and Player (403); Editor → 200 (mirror `validate` gate tests) | REQ-003 |
| T5 | studio: `POST /editor/maps` with a path-unsafe id → 400 refused | REQ-004 |
| T6 | content: `load_authored_world(valid, catalog)` → materialized `WorldDefinition` | REQ-005 |
| T7 | content: `load_startup_world(None)` → beginner; `load_startup_world(Some(valid))` → authored world | REQ-005 |
| T8 | content: `load_authored_world` on missing file → `Read`; malformed JSON → `Parse`; unmaterializable doc → `Materialize` (typed, no panic) | REQ-006 |
| T9 | server: existing `begin_emits_beginner_start_room` / opening-scene tests stay green with no env set (beginner default) | REQ-005, REQ-007 |
| T10 | full existing cargo + `node --test` suites stay green (no regression) | REQ-007 |
| T11 | `bin/gate.sh` FULL green, mutation 100% MSI | REQ-008 |

No genuinely-uncoverable paths expected (all logic is behind testable functions; the env read in `main` is a one-line delegation to the tested `load_startup_world`).

**Risks / decisions:**
- **Reuse `FileSaveStore` (generic) for maps; add only `list()`** — minimal new storage surface; inherits the hardened path-safety the save-slot tickets built.
- **Save allows drafts (no materialize gate at save)** — authoring is iterative; the `/editor/maps/validate` endpoint already gives validity feedback; the game-load is the hard gate. Reversible.
- **Server REFUSES to start on a set-but-invalid `OATHSTAR_WORLD`** (loud typed error, no silent fallback to beginner) — masking would let you think you're playing authored content when you're not. Unset → beginner.
- **`OATHSTAR_WORLD` is a filesystem path** (not an id) for S1 simplicity; an id-based shared store can come in a later slice.
- **`StudioState.world` stays `Arc` (immutable)** — reopen loads into the editor client-side; we do NOT swap the studio's served world this slice (no `RwLock` refactor).
- **Mutation 100% MSI** — `list()`, the three handlers, and `load_authored_world`/`load_startup_world` need exhaustive tests (gate-enforced).
- Surfaced prevention rules (path-safety: `12f6e22e`, `ef08be37`, `8bb5a3eb`, `5454fd44`) are honored by reusing `FileSaveStore`.

## Phase 3 — Implement
- **Built (clippy `-D warnings` clean workspace-wide; 38 existing tests in the
  touched crates still green; new tests are Phase 4):**
  - `oathstar-storage`: `FileSaveStore::list()` — sorted, validated `*.json`
    basenames; missing root → empty; skips `*.json.tmp`/non-json.
  - `oathstar-content`: `AuthoredWorldError` (Read/Parse/Materialize; impls
    `Error` with `source`); `load_authored_world(path, &ContentCatalog)`
    (read → serde_json `MapDocument` → `materialize`; untrusted, no panics);
    `load_startup_world(Option<&Path>)` (None → beginner; Some → authored).
    Promoted `serde_json` from dev- to a real dependency.
  - `oathstar-studio`: `StudioState.maps: FileSaveStore` (root `OATHSTAR_MAPS_DIR`,
    blank-≠-unset, default `maps`); routes `POST/GET /editor/maps` +
    `GET /editor/maps/{id}`; handlers `save_map`/`list_maps`/`load_map`, all
    Editor-gated; extracted a shared `editor_refusal` gate (also adopted by
    `validate`). Minimal editor reopen in `render.rs` (`?map=<id>` → fetch the
    saved doc, falling back to the starter).
  - `oathstar-server`: startup reads `OATHSTAR_WORLD` (blank-≠-unset) and calls
    `load_startup_world(path)`; unset → beginner unchanged.
- **Deviations from design (+ reason):**
  1. The gate helper returns `Option<Response>` (`Some` = refusal), named
     `editor_refusal`, not `Result<(), Response>` — clippy `result_large_err` fires
     on a large `Response` in `Err`; `Option` sidesteps it without boxing.
  2. `load_startup_world` returns `anyhow::Result`, not a separate
     `StartupWorldError` — the beginner path is already `anyhow` and
     `AuthoredWorldError` auto-converts; the untrusted-input path keeps its typed
     `AuthoredWorldError`. (§14 typed-error intent holds where it matters.)
  3. `list_maps`/`load_map` use `map_or_else` (clippy nursery `option_if_let_else`)
     instead of `match`.
  4. Studio test helpers gained the `maps` field (compile fix): `editor.rs` uses a
     unique, freshly-emptied temp dir per call (ready for Phase-4 save/load tests);
     `handlers.rs`/`sections.rs` use a never-written placeholder (those tests do not
     touch maps).
- **Save allows drafts** (no materialize gate on save), per design; validity is
  enforced at game-load and via the existing `/editor/maps/validate`.

## Inspect (Phase 3.5)
- **Lenses run** (3 parallel `general-purpose` critics over the `crates/` diff,
  each verifying against source at runtime): (A) security / auth-boundary /
  path-safety; (B) correctness / status-codes / round-trip / server-selection;
  (C) §14-robustness / reuse / regression / WIP-hygiene.
- **Security — CLEAN.** All four handlers (`validate`/`save_map`/`list_maps`/
  `load_map`) call `editor_refusal` as their first statement (401/403) before any
  side effect; slot names validated on BOTH save and load (handler + the storage
  `path_for`/`ensure_not_symlink` boundary), so traversal/absolute/symlink ids
  cannot escape the maps dir; `list()` re-validates and skips hostile names; the
  authored-world file is untrusted (typed errors, no panic; invalid → loud startup
  failure, no silent fallback); the public game route table is unchanged
  (Decision 058 intact); no secrets/unsafe/shelling.
- **Correctness — CLEAN (runtime-proven).** Byte-faithful save→load round-trip;
  `list()` skips `*.json.tmp` + malformed names, missing root → empty, sorted;
  `OATHSTAR_WORLD` unset→beginner / valid→authored / invalid→loud error;
  `AuthoredWorldError` Read/Parse/Materialize mapping correct; 401/403/400/404/500/200
  all reachable; `?map=<id>` reopen falls back to the starter on any failure and a
  no-query load is unaffected; the `/editor/maps/validate` + `/editor/maps/{id}`
  sibling routes build under axum 0.8.
- **§14 / reuse / regression — CLEAN.** No `unwrap`/`expect`/`panic`/index on input
  paths (test-only `expect` excepted); new public items doc-commented; NO
  suppressions added; `editor_refusal` reproduces the original gate exactly and is
  the sole auth path in all four handlers; `FileSaveStore` reused (no storage
  reinvented); beginner behavior + the `validate` contract byte-identical to HEAD;
  78+20+38+1 existing tests pass with the added `maps` field; the stashed
  online-first WIP was NOT swept in (diff scoped to `crates/` + ticket-53 docs).

| # | Severity | Finding (file:line) | Verdict | Fix |
|---|---|---|---|---|
| 1 | process | New paths ship with NO tests — FULL gate's 100% MSI + 94% coverage will fail until covered (`editor.rs` handlers, `storage::list`, `content::load_*`, `AuthoredWorldError`) | **Real but EXPECTED** — Phase 3 writes code; Phase 4 writes + runs tests. Design test plan already names these; `studio()` fixture provisions a unique maps dir. | No code fix; Phase-4 deliverable. |
| 2 | low | `Saved.ok`/`MapList.ok` always-`true` → mutation survivors (`editor.rs:44-54`) | **Real (mutation target)** — matches the existing `Success { ok: true }` pattern. | No code fix; Phase-4 tests MUST assert the body (`ok`,`id`,`maps`), not just status, to kill the mutant. |
| — | — | Authored worlds pinned to `beginner_catalog()`; drafts bypass save-time validation | **Intended** (entity/item authoring + draft validity are later concerns; game-load is the hard gate). | None. |

- **Net:** no critical/high/medium **code** defects; no source fixes required. No
  `failure-record` (no real bug). Sole actionable output is the Phase-4 test suite,
  with the mutation-survivor targets above called out so it reaches 100% MSI.

## Phase 4 — Validate
- **Tests added (all green):**
  - `oathstar-storage` (+3): `FileSaveStore::list` — missing root → empty;
    non-directory root → Err; sorted ids, skipping `*.json.tmp` / non-json /
    malformed names. (Also refined `list` to `flatten()` + `to_string_lossy` to
    drop two un-coverable defensive branches for the 100% MSI floor.)
  - `oathstar-content` (+6): `load_authored_world` valid → materialized world;
    missing → `Read`; malformed JSON → `Parse`; unmaterializable → `Materialize`
    (each asserts the variant + non-empty `Display` + `source().is_some()`);
    `load_startup_world(None)` → beginner, `Some` → authored.
  - `oathstar-studio` (+9): save→list→load round-trip (asserts the BODY —
    `ok`/`id`/`maps` — killing the always-true `ok` mutants, inspect finding #2);
    all three endpoints refuse anon (401) + Player (403); save bad-json → 400,
    bad-id → 400; load missing → 404, bad-id → 400; save + list storage-failure →
    500 (a maps-as-file fixture).
- **`cargo test --workspace`:** ok — content 84, storage 23, studio 47, core 300,
  protocol 27, server 34, datastar 16, auth 20; 0 failed.
- **`node --test tests/*.test.js`:** 79 pass, 0 fail.
- **`bin/gate.sh --fast`:** GREEN [fast] — all 14 static gates pass.
- **`bin/gate.sh` (FULL), run 1:** RED — 16/17 pass (rust coverage ✓, js coverage
  89.4% ✓); gate:17 mutation 99.6% (553 caught / **2 missed**): `delete !` in the
  `OATHSTAR_WORLD` and `OATHSTAR_MAPS_DIR` blank-≠-unset filters. Those `!`s sat
  inside `main`, and the mutants config excludes only main's *body-replacement*
  (`replace main ->`), not inner mutants — so they were tested and survived.
- **Fix (source, no suppression — §0):** extracted both filters into tested
  helpers — `authored_world_path(Option<String>)` (server) and
  `resolve_maps_dir(Option<String>)` (studio), each with a None/blank/value unit
  test; `main` is now a thin call. Scoped re-run `cargo mutants --file
  server/main.rs --file studio/main.rs`: 42 mutants → 14 caught / 28 unviable /
  **0 missed**.
- **`bin/gate.sh` (FULL), run 2:** GREEN [full] — 17/17. rust coverage ≥94% ✓,
  js coverage 89.44% ✓, mutation 559 caught / 0 missed → MSI 100.0% ✓. Commit-gate
  receipt written.
- **Pre-existing exclusions:** none.

## Phase 5 — Complete
- **Docs updated:** spec + notes finalized (Phase 5 Complete). No `decisions.md`
  edit — the persistence decision is captured as a forge architecture-decision
  instead, to avoid conflicting with the stashed online-first WIP (`decisions.md`
  is parked in `stash@{0}`).
- **Forge capture:** AAR `f55ca2e5-…` submitted (completed, effectiveness 4);
  prevention rule `PR-claude-extract-main-logic-for-mutation-coverage-001` (logic
  in `main` escapes mutation coverage — extract to a tested helper); architecture
  decision `AD-claude-authored-world-persistence-001` (persist the editable
  MapDocument; game loads via materialize behind `OATHSTAR_WORLD`).
- **Ticket closed:** forge #53 → done.
- **Archived:** spec + notes → `docs/planning/pipeline/completed/`; ticket doc →
  `docs/planning/tickets/closed/`.
- **Program:** S1 of region-authoring complete; S2–S5 remain (see the intake).
