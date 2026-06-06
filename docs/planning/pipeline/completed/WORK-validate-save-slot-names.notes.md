# WORK-validate-save-slot-names — Notes

## Phase 1 — Plan
- **Request:** ticket #3 — validate save slot names.
- **Intake source:** none.
- **Classification / tier:** work pipeline, single shippable slice (`chore`).
- **Forge recall:** prevention rule `9769be11` (typed errors over panics on
  untrusted/malformed input — the same boundary discipline), `6b6d9158`
  (coverage-exception discipline), AD `2530308c` (core validation boundary at
  construction). Bulletins: none.
- **Code grounding (`crates/oathstar-storage/src/lib.rs`):**
  - `FileSaveStore::path_for(&self, name: &str) -> PathBuf` = `self.root.join(format!("{name}.json"))`
    — joins an arbitrary name, so `../x` or `/abs` would escape the save root.
  - `write_json`/`read_json` call `path_for`. The fix: a typed slot-name
    validation boundary enforced here, rejecting separators + `..` before the join.
  - Storage is at 100% line coverage (ticket #8) and MSI 100% — new code + tests
    must hold the ratcheted floors (ticket #9: Rust ≥94, JS ≥75, MSI 100).
- **Ticket:** forge #3 `e58bad86-b2e6-4e97-b36e-10c6bf63491d`; claimed by
  `bfd73c91-9f58-451e-b9a5-1e958b0f4eb2`; local doc linked.
- **AAR:** `d8694ec2-28b5-422a-afc6-f2bbdb721804`.

## Phase 2 — Design

### Approach / architecture
- One new crate-local boundary in `oathstar-storage`, no new deps:
  - `SaveSlotError` enum (`#[derive(Debug, Clone, PartialEq, Eq)]` + `Display` +
    `std::error::Error`): `Empty`, `ContainsSeparator { name }`,
    `Traversal { name }`, `DisallowedCharacter { name, character }`.
  - `pub fn validate_save_slot_name(name: &str) -> Result<(), SaveSlotError>` —
    THE rule, public + reusable (REQ-004). Order: empty → separator (`/`,`\`) →
    traversal (`contains("..")`) → allowlist loop (ASCII alphanumeric, `-`, `_`).
    The explicit separator/traversal checks come before the allowlist so REQ-002
    and REQ-003 get their own distinguishable variants.
- **Enforcement:** `FileSaveStore::path_for` becomes
  `-> Result<PathBuf, SaveSlotError>` and calls `validate_save_slot_name(name)?`
  before joining. `write_json` is reordered to compute `path_for(name)?` **first**
  (validate before `create_dir_all`), so an invalid name fails with **no
  filesystem side effects**. `read_json` gains `?` on `path_for`. Trait stays
  `&str` (no cascade); `SaveSlotError` converts to `anyhow::Error` via `?`.
- Allowlist disallows `.` entirely, so `..` is doubly-blocked; the explicit
  `Traversal` check just yields the clearer variant.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-storage/src/lib.rs` | Add `SaveSlotError` (+`Display`+`Error`); add `pub validate_save_slot_name`; `path_for` → `Result` (validates); `write_json` validate-first then `create_dir_all`; `read_json` `?`; add tests T1–T8. |

### Regression Test Plan (≥1 per AC; assert EXACT variant)
| # | Test (location) | Proves / covers |
|---|---|---|
| T1 | `validate_accepts_allowed_characters` — `validate_save_slot_name("slot_1-A")` is `Ok` (covers alphanumeric + `_` + `-`) | REQ-001 |
| T2 | `validate_rejects_path_separator` — `"a/b"` → `ContainsSeparator`; `"a\\b"` → `ContainsSeparator` | REQ-002 |
| T3 | `validate_rejects_parent_traversal` — `".."` and `"a..b"` → `Traversal` | REQ-003 |
| T4 | `validate_rejects_empty` — `""` → `Empty` | REQ-005 |
| T5 | `validate_rejects_disallowed_character` — `"a b"` → `DisallowedCharacter{character:' '}` | REQ-005 |
| T6 | `save_slot_error_messages_render` — `to_string()` of all 4 variants names the offender (kills `Display` mutants) | REQ-002/003/005 detail |
| T7 | `write_json_rejects_unsafe_name` — `write_json("../escape", ..)` → `Err`; assert the save dir/file was NOT created (validate-first) | REQ-002/003 + enforcement |
| T8 | `read_json_rejects_unsafe_name` — `read_json("../escape")` → `Err` | enforcement |
| — | existing #8 tests (`write_then_read_round_trips`, `read_missing_is_err`, the 3 error-path tests) keep passing — `"hero"`/`"does-not-exist"` are valid names | REQ-001 regression |
- REQ-004 verification: source review (single public `validate_save_slot_name` reused by `path_for`; no ad-hoc checks) + T7/T8 prove enforcement.
- Genuinely-uncoverable: none.

### Risks / decisions
- **Boundary shape:** chose a public `validate_save_slot_name` fn + typed
  `SaveSlotError` over a `SaveSlotId` newtype — it's the minimal "small typed
  boundary" (REQ-004) with no `SaveStore` trait/`&str`-API cascade and no churn
  to the #8 tests. A `SaveSlotId` newtype can wrap this rule later when the
  save/load HTTP endpoints land (out of scope here). Flagged for Inspect.
- **No new mutants escape:** every `validate` branch + `Display` arm is tested
  with the EXACT variant; `path_for`/`write`/`read` mutation is covered by the
  round-trip + reject tests. MSI stays 100%; coverage rises (all new lines tested).
- **Ordering:** validate-before-`create_dir_all` is load-bearing (no side effects
  on rejection) and tested by T7.

## Phase 3 — Implement
- **Built (production code; `crates/oathstar-storage/src/lib.rs` only):**
  - `SaveSlotError` enum (+ `Display` + `std::error::Error`, `#[derive(Debug, Clone, PartialEq, Eq)]`).
  - `pub fn validate_save_slot_name(&str) -> Result<(), SaveSlotError>` — empty →
    separator → traversal → allowlist (ASCII alphanumeric / `-` / `_`).
  - `FileSaveStore::path_for` → `Result<PathBuf, SaveSlotError>` (validates first).
  - `write_json` reordered to `path_for(name)?` BEFORE `create_dir_all` (no fs
    side effects on a rejected name); `read_json` gains `?` on `path_for`.
- **In-phase checks (green):** `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✓;
  `cargo test --workspace` ✓ — 40 tests, 0 failed (existing #8 storage tests unaffected; `"hero"`/`"does-not-exist"` are valid).
- **Deviations from design:** none.
- **Deferred to Phase 4:** tests T1–T8 (the new validation branches + `Display`
  are intentionally still uncovered until Validate writes them).

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel critics): security/path-traversal-bypass; mutation/coverage-readiness.
- **Verdicts:** TRAVERSAL-SAFE; MSI 100% + coverage ≥94% ACHIEVABLE.
- **Findings:**
  | # | Severity | Finding | Verdict |
  |---|---|---|---|
  | 1 | (verified) | Deny-by-default allowlist `[A-Za-z0-9_-]` is sound — exhaustive Unicode proof + 46-case adversarial PoC + real-FS check: **0 accepted names escape root** (absolute/drive/UNC/`..`/`.`/null/URL-encoded/unicode-homoglyph all rejected). | confirmed (REQ-001/002/003/005) |
  | 2 | (verified) | Enforcement complete: `path_for` is the only path builder and validates first; `write_json` + `read_json` both `path_for(name)?`; validation runs BEFORE `create_dir_all` (no fs side effects on rejection). No other `join`/`root` path construction. | confirmed (REQ-004) |
  | 3 | (verified) | No panic/`unwrap`/`expect` on the input path; no `unsafe`/`Command`/secrets; errors are typed and propagate via `?`. | confirmed |
  | 4 | low / residual / OUT OF SCOPE | symlink/TOCTOU: lexical name validation cannot stop a pre-existing symlink inside the save dir; defense-in-depth would add `canonicalize` + `starts_with(root)`. Requires a separate write primitive in the save dir; not a name-based bypass. | documented → future-hardening follow-up |
  | 5 | info / residual / OUT OF SCOPE | Windows reserved device names (CON/NUL/COM1…) and unbounded length pass the allowlist; on the current target (darwin) they're ordinary filenames and still cannot escape root. Ticket scope is separators + traversal. | documented (future-only) |
  | 6 | (mutation) | `cargo mutants --list` → 12 mutants; #12 (`read_json→Ok(Default)`) is **unviable** (Hero isn't `Default`), excluded from MSI. #9/#10/#11 already killed by the existing #8 I/O tests. The rest need the Validate checklist. | carry to Validate |
- **Validate test checklist (from the mutation critic):** assert EXACT variant/field
  throughout (not `is_err()`); a valid-name test must exercise `-` AND `_` AND an
  alphanumeric (kills the `=='-'`/`=='_'`/`||→&&` mutants); all 4 `Display` arms
  asserted (line coverage); + 2 integration bad-name tests (`write_json`/`read_json`
  reject → Err). Existing #8 I/O tests cover the happy path / `root`.
- **No code defects; no fixes made in Inspect.** Residuals #4/#5 are out of this
  ticket's scope and recorded for follow-up.

## Phase 4 — Validate
- **Tests added (8, in `oathstar-storage`):** `validate_accepts_allowed_characters`
  (REQ-001, exercises `-`/`_`/alnum), `validate_rejects_empty` (REQ-005),
  `validate_rejects_path_separators` (REQ-002, both `/` and `\`),
  `validate_rejects_parent_traversal` (REQ-003), `validate_rejects_disallowed_character`
  (REQ-005, exact `character`), `save_slot_error_messages_render` (all 4 `Display` arms),
  `write_json_rejects_disallowed_name` + `read_json_rejects_disallowed_name` (REQ-004
  enforcement; write asserts NO dir is created → validate-before-side-effects).
- **`cargo test --workspace`:** ok — 48 Rust tests (storage 14, core 20, server 9, content 5), 0 failed.
- **`node --test tests/*.test.js`:** ok — 4 passed.
- **`bin/gate.sh` (FULL):**
  - Run 1: **RED** — gate:11 source-bans flagged the literal token `unsafe ` in test
    comments/strings ("an unsafe slot name"). The SAST grep is a naive text match for
    the `unsafe` keyword and caught the English word. Everything else green.
  - Fix at source (CONSTITUTION §0, no suppression): rephrased "unsafe" → "disallowed"
    in the new test comments/strings/fn-names.
  - Run 2: **GREEN [full] — 16/16.** Receipt `c84a3429`.
- **Before → after coverage:** Rust line 94.34% → **94.97%** (`oathstar-storage` 100%);
  JS 75.19% (unchanged); mutation **29 → 37 caught / 0 missed, MSI 100%** (the 8 new
  tests killed the new validation mutants). No gate floor lowered (still 94/75/100).
- **Documented residuals (Inspect, out of scope):** symlink/TOCTOU (needs
  `canonicalize`+`starts_with` — follow-up) and Windows reserved device names / length
  limits (future-only; cannot escape root on darwin). Recorded for a future ticket.
- **Pre-existing failures:** none.

## Phase 5 — Complete
- **Docs updated:** none beyond these pipeline notes — this is an instance of the
  existing validation-boundary pattern (AD `2530308c`), no new architecture/decision.
  Out-of-scope residuals (symlink/TOCTOU, Windows reserved names/length) recorded
  in the Inspect/Validate notes for a future hardening ticket.
- **Forge capture:** `aar-submit` d8694ec2 (completed, effectiveness 5, 2 novel
  findings); `failure-record` `BF-storage-sast-keyword-in-test-prose-001` (f7e245c3);
  `prevention-rule-record` `PR-claude-sast-keyword-prose-001` (71d2f65d).
- **Ticket closed:** forge #3 → `done`; completion comment 55842649.
- **Archived:** pipeline pair → `docs/planning/pipeline/completed/`; local ticket doc
  → `docs/planning/tickets/closed/` (cross-links pre-fixed).
