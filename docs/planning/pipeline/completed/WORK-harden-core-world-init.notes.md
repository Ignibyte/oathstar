# WORK-harden-core-world-init — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** forge ticket #2 — harden core world initialization.
- **Intake source:** none (only intake on disk is unrelated: swappable rulesets).
- **Classification / tier:** work pipeline, single shippable slice (`chore`).
- **Forge recall (lessons/failures surfaced):**
  - `knowledge-context` (phase=Plan, aar=fa4ac433…) surfaced 2 nodes, both
    recency-top (almost certainly from ticket #1's capture):
    - architecture_decision `7bbe40c7-1dce-4c7c-9f0b-16e11b2ab5c5` (semantic ~0.65)
    - prevention_rule `9769be11-a6e7-4396-a637-695559c4550d` (semantic ~0.60)
  - `surfacings_written: 2` — both logged into this AAR's surfacing log.
  - **Tooling limitation (surfaced, not bypassed):** the forge MCP read surface
    (`knowledge-search/-explain/-context/-related`) returns node **refs +
    rankings only**, not bodies; `knowledge-related` depth-1 returned no
    neighbors. The two nodes' bodies could not be dereferenced via MCP. Their
    domain (typed errors over `expect`, core-owned validation boundary) is
    already encoded as binding ACs in this ticket (REQ-001..004), so the
    guidance is captured even without verbatim text.
  - `bulletin-list`: none active.
  - `docs-search`: confirmed the "Core engine + swappable worlds" direction and
    the module-system rule "core-validated state changes only" — validation
    belongs in core.
- **Code grounding (working tree):**
  - `oathstar-core::Engine::new` (`crates/oathstar-core/src/lib.rs:61`) trusts
    its `WorldDefinition` blindly.
  - `oathstar-core::Engine::current_room` (`…:244`) does
    `.expect("current room must exist…")` — the latent panic the ticket targets.
  - `oathstar-content::validate_world` (`crates/oathstar-content/src/lib.rs:78`)
    is a private validator (missing-start + dangling-exit only; no
    impassable-start check) — to be superseded by the core boundary.
- **Ticket:** forge #2 `99619421-df57-4aec-976f-a4139eafd469`; claimed by
  session owner `4f9b50eb-33b3-404c-aa01-9ba129a6d361`; local doc
  `docs/planning/tickets/open/TICKET-2-harden-core-world-initialization.md`.
- **EARS requirements reviewed:** REQ-001..007 (ticket's REQ-001..004 carried
  forward + REQ-005 key/id consistency, REQ-006 valid-world guard, REQ-007
  content-delegates-to-core).
- **AAR:** `fa4ac433-a861-4278-afec-343044afbe6c`.

## Phase 2 — Design

### Approach / architecture
- **Typed error in core (no new deps).** Add `WorldValidationError` to
  `oathstar-core` — a hand-rolled enum impl `Display` + `std::error::Error`,
  `#[derive(Debug, Clone, PartialEq, Eq)]`. `thiserror` is NOT a workspace dep
  and core depends only on `oathstar-protocol` + `serde`; hand-rolling keeps
  gate:6 (deny) / gate:7 (machete) / gate:5 (audit) untouched. Variants:
  - `StartRoomMissing { start_room_id }`  (REQ-001)
  - `StartRoomImpassable { start_room_id }`  (REQ-002)
  - `DanglingExit { room_id, direction, target_room_id }`  (REQ-003)
  - `RoomKeyMismatch { key, room_id }`  (REQ-005)
- **Pure validator.** `WorldDefinition::validate(&self) -> Result<(), WorldValidationError>`
  checks, in order: key/id consistency (REQ-005) → start exists (REQ-001) →
  start passable (REQ-002) → every exit resolves (REQ-003). Reusable by content
  (REQ-007).
- **Validated constructor.** `Engine::try_new(world) -> Result<Self, WorldValidationError>`
  calls `world.validate()?` then constructs exactly as the old `new` did. The
  infallible `Engine::new` is **removed** so an invalid `Engine` is
  unconstructable (the only entry point validates).
- **Content delegates.** `load_beginner_world` calls `world.validate()?` (typed
  error → `anyhow` via `?`) and the private `validate_world` is deleted (single
  source of truth, REQ-007). The duplicate-room-id `bail!` stays in content — it
  is a TOML-list parse concern, not a `WorldDefinition` invariant (the `BTreeMap`
  would silently dedupe).
- **`current_room()` keeps its `.expect()` — deliberate, justified.** After
  `try_new`, `current_room_id` is an invariant-checked key for the Engine's whole
  life (start is validated present; `move_direction` only ever sets it to a room
  it already fetched). So the path is **no longer reachable from malformed module
  data** → REQ-004 (as worded) holds. Removing the `.expect()` would require
  either making the public `snapshot()` API fallible (out of scope — protocol
  cascade) or adding a stored-start-room / `Option` fallback; both create a
  branch that is **unreachable for any validated Engine**, which gate:16
  (mutation, MSI 100%) would flag as a surviving mutant → RED. The `.expect()`
  is MSI-neutral (cargo-mutants emits no mutant for a `&T`-returning fn). This is
  flagged for the Inspect lens to confirm unreachability.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/lib.rs` | Add `WorldValidationError` (enum + `Display` + `Error`); add `WorldDefinition::validate`; add `Engine::try_new`; **remove** `Engine::new`; update 8 in-crate test call sites `new(test_world())` → `try_new(test_world()).expect("valid test world")`; add malformed-world unit tests (T1–T8). `current_room()` unchanged (its `.expect()` is now a `try_new`-guaranteed invariant; comment added). |
| 2 | `crates/oathstar-content/src/lib.rs` | `load_beginner_world` calls `world.validate()?`; **delete** private `validate_world`; remove the 3 `validate_world` unit tests (logic moved to core); keep `beginner_world_loads`; keep duplicate-id `bail!`. |
| 3 | `crates/oathstar-server/src/main.rs` | `:27` `Engine::new(world)` → `Engine::try_new(world)?` (fn returns `anyhow::Result`); `:274` test `Engine::new(world)` → `Engine::try_new(world).expect("valid beginner world")`. |

### Regression Test Plan (≥1 per AC; mutation-aware — assert exact variant + message)
| # | Test (location) | Proves |
|---|---|---|
| T1 | `core: try_new_rejects_missing_start_room` — assert `Err == StartRoomMissing{..}` | REQ-001 |
| T2 | `core: try_new_rejects_impassable_start_room` — assert `Err == StartRoomImpassable{..}` | REQ-002 |
| T3 | `core: try_new_rejects_dangling_exit` — assert `Err == DanglingExit{room,direction,target}` | REQ-003 |
| T4 | `core: malformed_worlds_return_err_not_panic` — each bad world yields `Err` (no panic) | REQ-004 |
| T5 | `core: try_new_rejects_room_key_id_mismatch` — assert `Err == RoomKeyMismatch{..}` | REQ-005 |
| T6 | `core: try_new_accepts_valid_world` + `snapshot()` works on it | REQ-006 |
| T7 | `content: load_beginner_world_validates_through_core` (loads ok) | REQ-007 |
| T8 | `core: validation_error_messages_name_the_offender` — `to_string()` of each variant contains the offending id/direction (kills `Display` mutants) | REQ-001/003 detail + MSI |
| — | existing `movement_discovers_rooms` already proves `current_room` stays valid after movement (guards the retained `.expect()`) | invariant guard |

Uncoverable path: the `current_room()` `.expect()` arm is unreachable for a
validated Engine by design — not separately testable, and intentionally so
(see approach). No other genuinely-uncoverable paths.

### Risks / decisions
- **Removing `Engine::new`** is a public-API change; blast radius is the 3 files
  above (enumerated via working-tree `rg`, not just the codegraph). In scope per
  the ticket ("call-site updates").
- **Gate harness is untracked** (added after the initial commit) → the workspace
  may carry **pre-existing** gate failures (esp. gate:16 MSI 100% on existing
  content/server branches). **Design-close baseline: `bin/gate.sh --fast` =
  GREEN, 13/13 static gates** on the pristine tree. Coverage+mutation (14–16)
  deferred to Validate (FULL run); surviving-mutant file:line will self-attribute
  mine vs pre-existing. Any pre-existing red is surfaced + fixed (user directive:
  no skips) or escalated, not silently bypassed.
- **Mutation-tightness** is designed in: every `validate` branch has a dedicated
  malformed-world test asserting the **exact** error variant + message, so
  condition-flip and variant-swap mutants are killed.

## Phase 3 — Implement
- **Built (exactly the 3-file manifest):**
  - `core/src/lib.rs`: `WorldValidationError` (enum + `Display` + `std::error::Error`,
    `#[derive(Debug, Clone, PartialEq, Eq)]`); `WorldDefinition::validate`
    (key/id → start-exists → start-passable → exits-resolve); `Engine::try_new`
    (`world.validate()?` then construct); **removed** `Engine::new`;
    `current_room()` `.expect()` retained with an invariant comment (REQ-004
    justification); 8 in-crate test call sites → `try_new(test_world()).expect(...)`.
  - `content/src/lib.rs`: `load_beginner_world` now builds the `WorldDefinition`
    then calls `world.validate()?` (typed error → `anyhow` via `?`); **deleted**
    the private `validate_world` + its 3 unit tests + the now-unused `room()`
    helper; kept `beginner_world_loads` and the duplicate-id `bail!`.
  - `server/src/main.rs`: `:27` `Engine::new(world)` → `Engine::try_new(world)?`;
    `:274` test → `Engine::try_new(world).expect("valid beginner world")`.
- **In-phase checks (green):** `cargo check -p oathstar-core --all-targets` ✓;
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✓ (no
  warnings); `cargo test --workspace` ✓ (all pre-existing tests pass under the
  new constructor).
- **Deviations from design:** none.
- **Deferred (correctly) to Phase 4:** the new invariant tests T1–T8 (the
  implement skill forbids test expansion here).
- **Watch item for Validate:** content's duplicate-id `bail!` is unreachable from
  the embedded const module (no way to inject a duplicate) → a likely
  **pre-existing** coverage/mutation gap, independent of this ticket. Decide at
  Validate: refactor `load_beginner_world` to delegate parsing to a testable
  helper, or document as pre-existing. Do NOT silently skip.

## Inspect (Phase 3.5)
- **Lenses run** (4 independent parallel critics): correctness, mutation/coverage-readiness,
  simplification/reuse/API, security/data-integrity. Each verified by reading code +
  running `cargo`/`grep`.
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | (key) | `current_room()` `.expect()` reachability (core:346–357) | **CONFIRMED SAFE** — traced both writers of `current_room_id` (try_new=start; move_direction=`next_room.id`). REQ-005 key==id is *necessary AND sufficient*: `move_direction` assigns the room's own id, so without key==id it could set a non-key. Unreachable for any constructed Engine; also backstopped by move_direction's runtime defense. | None (documented invariant) |
  | 2 | (regression) | content validate vs deleted `validate_world` | **CONFIRMED** equivalent (Display strings byte-identical to old `bail!`) and strictly stricter (adds impassable-start + key/id). No regression. | None |
  | 3 | (correctness) | `validate()` logic/order/edge-cases/determinism (core:86–121) | **SOUND** — empty map→StartRoomMissing; self-loop exit→Ok (benign); non-start impassable→Ok (intentional, move_direction blocks at runtime); deterministic (BTreeMap order). No false +/−. | None |
  | 4 | (API) | `try_new` replaces `new`; `validate` pub; hand-rolled error | **CLEAN** — no pre-validated construction path exists (every world comes from untrusted data); no dead imports; clippy `-D warnings` clean. | None |
  | 5 | (security) | bypass / secrets / unsafe / overflow / info-leak | **SAFE** — Engine fields private (no struct-literal bypass); no unsafe/Command/process; u64 overflow unreachable; Display echoes only world ids, surfaced to operator console not HTTP. | None (future: input-size limit at loader when community worlds land — out of scope) |
  | 6 | **medium / ACTIONABLE** | New core code needs targeted tests for gate:16 MSI 100% | **REAL** — must assert EXACT error variant (not just `is_err()`), all 4 `Display` messages, and `try_new` numeric-literal initializers. | **→ Validate** (9-test checklist below) |
  | 7 | **medium / ACTIONABLE / PRE-EXISTING** | content duplicate-id `bail!` unreachable from embedded const → 2 surviving mutants (content:46–48) | **REAL** — blocks `--workspace` MSI 100% regardless of this ticket. | **→ Validate**: extract `load_world_from_toml(module_src, rooms_src)` + duplicate-id test. NOT excluded in mutants.toml (CONSTITUTION §0). |
- **No code defects to fix in the diff.** Items 6–7 are test-phase work, scheduled for Validate and tracked (not skipped).
- **Validate test checklist (from the mutation critic):**
  1. `validate_accepts_valid_world` → `assert_eq!(w.validate(), Ok(()))` (kills the 3 `condition→true` mutants; world must have ≥1 real exit)
  2. `validate_rejects_key_id_mismatch` → assert exact `RoomKeyMismatch{key,room_id}`
  3. `validate_rejects_missing_start_room` → assert exact `StartRoomMissing{start_room_id}`
  4. `validate_rejects_impassable_start_room` → assert exact `StartRoomImpassable{start_room_id}`
  5. `validate_rejects_dangling_exit` → assert exact `DanglingExit{room_id,direction,target_room_id}`
  6. `validation_error_messages_render` → assert `to_string()` of ALL 4 variants (kills `Display::fmt→Ok(())` + per-arm swaps)
  7. `try_new_seeds_initial_state` → assert tick=0, level=1, xp=0, hp=20, max_hp=20, focus=5, max_focus=5, current="a" (kills literal mutants)
  8. `try_new_rejects_invalid_world` → assert `Err(...)` propagates through try_new
  9. `try_new_marks_start_discovered` → assert start room `discovered==true` (coverage)
  - content: `load_rejects_duplicate_room_id` (after the helper extraction) → assert err contains `duplicate room id 'dup'`
  - (no test needed for `current_room`/`.expect()` — cargo-mutants emits no viable mutant for a `&T`-returning fn).

## Phase 4 — Validate
- **Tests added (12):**
  - core (9): `validate_accepts_valid_world` (REQ-006), `validate_rejects_key_id_mismatch`
    (REQ-005), `validate_rejects_missing_start_room` (REQ-001),
    `validate_rejects_impassable_start_room` (REQ-002), `validate_rejects_dangling_exit`
    (REQ-003), `validation_error_messages_name_the_offender` (Display, all 4 variants),
    `try_new_seeds_initial_state`, `try_new_rejects_invalid_world` (REQ-001/004),
    `try_new_marks_start_discovered`. Each rejection test asserts the EXACT error
    variant (kills variant-swap mutants).
  - content (2 new + 1 kept): `load_rejects_duplicate_room_id`,
    `load_propagates_core_validation_error` (REQ-007), `beginner_world_loads`.
  - Refactor enabling them: extracted `load_world_from_toml(&str,&str)` so the
    loader's invariant branches are reachable (closes the pre-existing MSI gap the
    inspect phase flagged — NOT excluded in mutants.toml, per §0).
- **`cargo test --workspace`:** ok — 29 tests, 0 failed (core 17, server 6, content 3, storage 3).
- **`node --test tests/*.test.js`:** ok — 4 pass, 0 fail.
- **`bin/gate.sh` (FULL):**
  - Run 1: **RED** — only gate:1 rustfmt failed (my new code needed wrapping). 15/16
    passed incl. gate:16 mutation. *The gate fired all 16 and correctly blocked.*
  - Fix at source: `cargo fmt --all` (no suppression, no baseline).
  - Run 2: **GREEN [full] — 16/16 passed.** Receipt `ccbe5421…` written.
  - Metrics: Rust coverage **88.66%** lines (floor 60), JS **75.19%** (floor 70),
    mutation **MSI 100.0%** — 29 caught / 0 missed (floor 100).
- **Pre-existing exclusions:** none. The pre-existing content duplicate-id mutation
  gap was FIXED (refactor + test), not excluded. The only `mutants.toml` exclusion
  is the pre-existing `main` entry (not introduced here).

## Phase 5 — Complete
- **Docs updated:** `docs/decisions.md` → Decision 030 (world invariants validated
  at the core construction boundary; concretizes Decisions 021 + 022). No
  CLAUDE.md/convention changes needed.
- **Forge capture:**
  - `aar-submit` fa4ac433 → outcome `completed`, effectiveness 5; surfaced nodes
    7bbe40c7 + 9769be11 marked used; 3 novel findings; distillation +
    pattern-emergence jobs enqueued.
  - `failure-record` `BF-world-init-untestable-loader-branch-001` (6e58ea62) — the
    pre-existing untestable content duplicate-id branch (inspect-caught), now fixed.
  - `architecture-decision-record` `AD-claude-world-validation-boundary-001` (2530308c).
  - `prevention-rule-record` `PR-claude-loader-testability-001` (cc73c333, recorded in Inspect).
- **Ticket closed:** forge #2 → `done`; completion comment c6185927 added.
- **Archived:** pipeline doc pair → `docs/planning/pipeline/completed/`; local ticket
  doc → `docs/planning/tickets/closed/`.
