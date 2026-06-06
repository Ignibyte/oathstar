# WORK-raise-coverage-baseline — Notes

Per-phase working notes for the paired `.spec.md`.

## Phase 1 — Plan
- **Request:** raise the coverage baseline before feature expansion.
- **Intake source:** none.
- **Classification / tier:** work pipeline, single shippable slice (`chore`).
- **Forge recall:** `PR-claude-loader-testability-001` (cc73c333 — separate
  parse-from-source so invariant branches are testable; directly applicable),
  failure `BF-world-init-untestable-loader-branch-001`, ADs from ticket #2.
  Bulletins: none.
- **Ticket:** forge #8 `e8a10f21-3e97-4c2a-87a7-6ea77f321502`; claimed by
  `6ac70d82-7254-4723-9cd8-4fdaf84e826e`; local doc
  `docs/planning/tickets/open/TICKET-8-raise-coverage-baseline.md`.
- **AAR:** `3cf6eaa5-db2b-4a64-a7cf-701d2853ab25`.
- **Pre-pipeline cleanup done:** fixed the two stale ticket-#2 archive
  cross-links (closed ticket doc `pipeline_spec`/"Completed pipeline" → `completed/`;
  completed spec "Ticket doc" → `closed/`).

### Baseline coverage (from `cargo llvm-cov --workspace --show-missing-lines`)
| File | Line cov | Missed lines |
|---|---|---|
| oathstar-content/src/lib.rs | 100.00% (regions 97.70%) | — (2 partial regions on parse-error path) |
| oathstar-core/src/lib.rs | 97.49% | 234–239 (empty input), 312–314 (no-exit), 320–322 (unfinished world-data), 328–330 (impassable) |
| oathstar-server/src/main.rs | 58.82% | 24–56 main; 62–67 health; 69–72 state_snapshot; 74–88 command; 90–118 events_json; 120–146 events_html |
| oathstar-storage/src/lib.rs | 94.55% | 3 error-context closures: l.37 create_dir fail, l.40 write fail, l.48 parse fail |
| **TOTAL Rust** | **88.66%** | — |
| JS src/world.js | 100.00% | — |
| JS src/engine.js | 67.61% | large — replacement-bound subsystems |
| **JS all files** | **75.19%** | — |
| Mutation MSI | 100.0% (29/0) | — |

> Baseline note: 88.66% is the **post-ticket-#2 FULL-gate** Rust line coverage
> (the user's stated reference). Measured strictly against `HEAD` (9cf41b8 — whose
> focused commit did not include the `oathstar-storage` crate) the number is
> 85.23%; either reference point rises after this ticket.

### Reachable vs documented-exception (scoping)
- **Reachable → real tests:** core empty-input (234–239), no-exit move (312–314),
  impassable move (328–330); content malformed-TOML parse path (regions);
  storage l.37/l.40/l.48 error closures; server `health`/`state_snapshot`/`command`.
- **Documented exceptions (REQ-004):** core 320–322 (unreachable — `try_new`
  validates exit targets; defense-in-depth kept by ticket #2); server `main`
  (binary composition root, mutation-excluded) + SSE loops `events_json`/
  `events_html` (per-event transform tested via `render_event_html`; stream/
  keep-alive wiring is glue).
- **JS:** `src/engine.js` gap = prototype-replacement debt (being ported to Rust
  core); `src/world.js` already 100%. Do not over-invest (REQ-002 documents).
- **MSI stays 100%:** current MSI is 100% with all the above uncovered, so the
  uncovered code yields no viable mutants; adding tests only raises coverage.

## Phase 2 — Design

### Approach / architecture
- **Tests-only change.** No production Rust/JS is modified (plus the already-done
  ticket-#2 archive-link fixes + these notes). Because no production code
  changes, **no new mutants are introduced → mutation MSI stays 100% by
  construction** (REQ-005), and coverage rises purely from new tests.
- **Server handlers tested directly** (construct `AppState`, call the `async fn`
  in a `#[tokio::test]`) — NO extracted shims/abstractions (REQ: no abstractions
  for coverage; the handlers are already the right unit).
- **Documented exceptions** (REQ-004) recorded here; exact line numbers
  re-confirmed at Validate from the post-change `cargo llvm-cov` run.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/lib.rs` | +3 tests (empty input; no-exit move; impassable-target move) |
| 2 | `crates/oathstar-content/src/lib.rs` | +2 tests (malformed module TOML; malformed rooms TOML) |
| 3 | `crates/oathstar-storage/src/lib.rs` | +3 tests (create-dir failure; write-collision failure; corrupt-JSON parse failure) |
| 4 | `crates/oathstar-server/src/main.rs` | +3 tests (`health`; `state_snapshot`; `command` + broadcast) — tests only |
| 5 | (done) ticket-#2 archive cross-link fixes | closed ticket doc + completed spec |

### Regression Test Plan (≥1 per AC; meaningful asserts)
| # | Test (location) | Proves / covers |
|---|---|---|
| T1 | core `empty_command_input_waits` | REQ-001/003; core 234–239; assert `accepted=false` + "world waits" message |
| T2 | core `move_with_no_exit_is_refused` | core 312–314; assert "cannot go that way" + room unchanged |
| T3 | core `move_into_impassable_room_is_blocked` | core 328–330; world `a`(start)→east→`c`(impassable); assert "Something blocks that way" + room unchanged |
| T4 | content `load_rejects_malformed_module_toml` | content parse-error region; err context `invalid module TOML` |
| T5 | content `load_rejects_malformed_rooms_toml` | content parse-error region; err context `invalid rooms TOML` |
| T6 | storage `write_fails_when_dir_cannot_be_created` | storage l.37; root under a regular file; err `failed to create save directory` |
| T7 | storage `write_fails_when_target_path_is_a_dir` | storage l.40; `name.json` pre-created as a dir; err `failed to write` |
| T8 | storage `read_fails_on_corrupt_json` | storage l.48; garbage file; err `failed to parse` |
| T9 | server `health_reports_ok` | REQ-001; server 62–67; `ok=true`, `service="oathstar-server"` |
| T10 | server `state_snapshot_returns_engine_state` | server 69–72; `current_room_id == hollowmere_square` |
| T11 | server `command_processes_and_broadcasts` | server 74–88; response accepted + a pre-subscribed receiver gets the broadcast event |

### Documented exceptions (REQ-004 — finalized at Validate)
- **core `move_direction` "unfinished world-data" (~320–322):** unreachable for a
  constructed `Engine` — `try_new` validates that every exit targets an existing
  room; kept as defense-in-depth (ticket #2 locked decision).
- **server `main` (~24–56):** binary composition root (socket bind, router, serve);
  not unit-testable; the `replace main ->` mutant is already excluded in `.cargo/mutants.toml`.
- **server `events_json`/`events_html` (~90–146):** SSE stream loop + keep-alive +
  axum `Event` wiring; the per-event transform (`render_event_html`) is tested; the
  stream/loop is composition glue with no viable mutant.
- **JS `src/engine.js` (~67.6%):** prototype-replacement debt — being ported to the
  Rust core; not over-invested per scope. `src/world.js` is already 100%.

### Risks / decisions
- No production code change ⇒ MSI 100% preserved; pure coverage gain. (If Inspect/
  Validate finds a place where a *tiny* meaningful refactor would help, it must
  keep MSI 100% and not add an abstraction-for-coverage.)
- Server handler tests build `AppState` like the existing `spawn_tick_loop` test
  (engine from `load_beginner_world`, a `broadcast::channel`). `command` broadcast
  is asserted via `try_recv` on a receiver subscribed before the call (no timing).
- T3 builds a dedicated world (start passable → impassable neighbor); `validate`
  accepts non-start impassable rooms, so `try_new` succeeds and the runtime
  "blocked" branch is reachable.

## Phase 3 — Implement
- **Built — 11 tests, no production code changed (tests-only ticket):**
  - `core/src/lib.rs` +3: `empty_command_input_waits`, `move_with_no_exit_is_refused`,
    `move_into_impassable_room_is_blocked` (reuse existing `test_world`/`cmd`/`room_with`/`world_with` helpers).
  - `content/src/lib.rs` +2: `load_rejects_malformed_module_toml`, `load_rejects_malformed_rooms_toml`.
  - `storage/src/lib.rs` +3: `write_fails_when_dir_cannot_be_created`,
    `write_fails_when_target_path_is_a_dir`, `read_fails_on_corrupt_json`.
  - `server/src/main.rs` +3: `health_reports_ok`, `state_snapshot_returns_engine_state`,
    `command_processes_and_broadcasts` (+ a `test_app_state` helper); handlers tested
    directly — no extracted shims.
- **In-phase checks (green):** `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✓;
  `cargo test --workspace` ✓ — 40 Rust tests (core 20, server 9, storage 6, content 5), 0 failed.
- **Deviation from the usual phase split (intentional, transparent):** for a
  coverage ticket the tests ARE the deliverable, so they were written here (not
  deferred to Validate) so that Inspect can adversarially review them and Validate
  runs them + drives the FULL gate / confirms the coverage delta.
- **Production code authored this ticket: none** (only `#[cfg(test)]` tests).
  NB for the commit: the diff vs `HEAD` also carries the **pre-existing uncommitted
  `oathstar-storage` crate** (the `FileSaveStore` impl + its original tests),
  which ticket #2's focused commit never included — including a pre-existing
  `#[must_use]` on `root()`. That code adds exactly one mutant
  (`replace FileSaveStore::root -> &Path`), killed by `root_returns_configured_path`,
  so MSI stays 100%.

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel critics): test-meaningfulness; coverage/exception-validity + MSI.
- **Verdicts:** test-meaningfulness = MEANINGFUL; coverage/exception = CLAIMS-HOLD.
- **Findings:**
  | # | Severity | Finding | Verdict | Action |
  |---|---|---|---|---|
  | 1 | (positive) | All 11 new tests assert unique discriminating behavior; none vacuous/shallow; `command_processes_and_broadcasts` is race-free (send happens-before return; receiver pre-subscribed); `move_into_impassable_room_is_blocked` builds a `try_new`-valid world. | CONFIRMED | none |
  | 2 | (positive) | `llvm-cov`: Rust TOTAL **94.34%** (core 99.45%, content 100%, storage 100%, server 77.73%); JS 75.19% (world.js 100%, engine.js 67.61%). MSI stays 100%. Floors + `.clippy-allowlist` + `[workspace.lints]` unchanged. | CONFIRMED | none |
  | 3 | (positive) | core 320–322 "unfinished world-data" branch independently traced as genuinely unreachable for any `try_new`-built Engine (private `world`, sole constructor, no post-construction mutation). | CONFIRMED | documented exception stands |
  | 4 | low / docs | "no production code changed" was imprecise — the commit carries the pre-existing uncommitted `oathstar-storage` impl (incl. a pre-existing `#[must_use] root()`). | REAL (notes accuracy) | **FIXED** — Phase 3 note clarified; mutation-neutral. |
  | 5 | low / docs | 88.66% baseline is the post-ticket-#2 gate number, not HEAD-without-storage (85.23%). | REAL (notes accuracy) | **FIXED** — baseline footnote added. |
  | 6 | low / pre-existing | `storage::read_missing_is_err` asserts bare `is_err()`; storage `scratch_dir` uses fixed temp names (safe within one run; could collide across concurrent crate test runs). | REJECTED for fix | pre-existing, out of scope; the new corrupt-json test already distinguishes read-vs-parse; not touching unrelated code. |
- **No code defects found.** The two REAL findings were notes-accuracy and are fixed; no production edits made in Inspect.

## Phase 4 — Validate
- **Tests added (written in Phase 3):** 11 — core 3, content 2, storage 3, server 3.
- **`cargo test --workspace`:** ok — 40 Rust tests (core 20, server 9, storage 6, content 5), 0 failed.
- **`node --test tests/*.test.js`:** ok — 4 passed, 0 failed.
- **`bin/gate.sh` (FULL):** **GREEN [full] — 16/16 passed.** Receipt `d33c1295`.

### Before / after coverage
| Scope | Before (post-#2 gate) | After | Δ |
|---|---|---|---|
| Rust line (gate:14 TOTAL) | 88.66% | **94.34%** | +5.68 (REQ-001 ✓) |
| oathstar-core | 97.49% | **99.45%** | 3 lines documented-unreachable |
| oathstar-content | 100.00% | **100.00%** | regions 97.70% → 100.00% |
| oathstar-storage | 94.55% | **100.00%** | — |
| oathstar-server | 58.82% | **77.73%** | handlers covered; main+SSE documented |
| JS line (gate:15) | 75.19% | 75.19% | unchanged — documented as prototype-replacement debt (REQ-002) |
| Mutation MSI (gate:16) | 100.0% | **100.0%** (29/0) | unchanged (REQ-005 ✓) |
- Floors unchanged: `RUST_COV_FLOOR=60 JS_COV_FLOOR=70 MUT_MSI_FLOOR=100`; `bin/.clippy-allowlist` + `[workspace.lints]` untouched (REQ-005 ✓).

### Documented intentional exceptions (REQ-004 — final line numbers)
- **`crates/oathstar-core/src/lib.rs:320–322`** — `move_direction` "That exit points
  into unfinished world-data" branch. Unreachable for any `Engine` (sole constructor
  `try_new` runs `validate()`, which rejects dangling exits; `world` is private and
  never mutated post-construction — independently traced in Inspect). Kept as
  defense-in-depth per ticket #2's locked decision.
- **`crates/oathstar-server/src/main.rs` `main` (~25–56)** — binary composition root
  (socket bind, router, serve). Not unit-testable; the `replace main ->` mutant is
  excluded in `.cargo/mutants.toml`.
- **`crates/oathstar-server/src/main.rs` `events_json`/`events_html` (~90–146)** — SSE
  stream loop + keep-alive + axum `Event` wiring. The per-event transform
  (`render_event_html`) is tested; the streaming loop is composition glue with no
  viable mutant (confirmed: MSI 100%).
- **`src/engine.js` (67.61% line)** — JS prototype being ported to the Rust core;
  uncovered blocks are secondary commands/error arms/save edges, not the playable
  arc (which `tests/game.test.js` exercises). Prototype-replacement debt, not padded
  (REQ-002). `src/world.js` is 100%.
- **Pre-existing exclusions / out of scope:** none introduced; the `storage` crate's
  pre-existing uncommitted impl rides in this commit (see Phase 3 note).

## Phase 5 — Complete
- **Docs updated:** coverage before/after + intentional exceptions recorded in these
  notes (Phase 4, REQ-004); stale ticket-#2 archive cross-links fixed. No
  architecture/decision change → `docs/decisions.md` untouched.
- **Forge capture:** `aar-submit` 3cf6eaa5 (completed, effectiveness 5, 1 novel
  finding); `prevention-rule-record` `PR-claude-coverage-exceptions-001` (6b6d9158).
  No failures recorded (Inspect found no code defects).
- **Ticket closed:** forge #8 → `done`; completion comment dfda8aeb.
- **Archived:** pipeline pair → `docs/planning/pipeline/completed/`; local ticket doc
  → `docs/planning/tickets/closed/`.
