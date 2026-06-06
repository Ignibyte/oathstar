# WORK-add-tauri-shell-quality-gate — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #4 — bring `src-tauri` into the quality story with an
  explicit check/build gate so shell-side Rust can't drift outside verification
  (auto-approve: drive the full pipeline autonomously).
- **Intake source:** none — forge ticket #4 + local ticket doc pre-existed.
- **Classification / tier:** work pipeline (single shippable slice; tooling +
  docs, no application behavior change).
- **Forge recall (lessons/failures surfaced):**
  - `WORK-ratchet-coverage-floors` (completed) is the house precedent for
    `bin/gate.sh` edits: minimal change, keep `shellcheck`/gate:9 clean,
    cross-check governing docs, "the gate IS the test".
  - Knowledge-search surfaced gate-area prevention rules + failures + arch
    decisions (will be re-surfaced with content via `knowledge-context` at
    Design entry, now that the AAR is open).
- **Current-state findings (the actual gap):**
  - `src-tauri` is a **standalone crate** (`name = "oathstar"`, lib
    `oathstar_lib`, own `Cargo.lock`), **not** a member of the root workspace
    (`crates/oathstar-*` only).
  - Compile gates therefore **skip it**: gate:1 `cargo fmt --all --check`,
    gate:2 `cargo clippy --workspace`, gate:3 `cargo test --workspace`,
    gate:14 `cargo llvm-cov --workspace` all scope to workspace members.
  - Static text-gates **already cover** `src-tauri/src`: gate:8 gitleaks,
    gate:10 no-suppressions, gate:11 source-bans (all list `src-tauri/src`),
    and gate:7 `cargo machete` walks the tree (hence the justified ignore in
    `src-tauri/Cargo.toml`). → The gap is specifically the **compile/lint/fmt**
    gates, not the text scans.
  - `CONSTITUTION.md:72-73` documents the current "src-tauri is outside the
    compile gates" stance — the doc REQ-004 targets.
  - Shell source today: `src-tauri/src/lib.rs` (one `#[tauri::command] app_name`
    + `run()`), `src-tauri/src/main.rs` (thin `main`). Tiny — gate cost is the
    first-build of `tauri` deps, not the project's own code.
- **Ticket:** forge #4 `2885b6ed-5deb-403b-afcb-67f80b35eb1d` (pre-existing,
  already documented at `docs/planning/tickets/open/TICKET-4-...md`).
- **EARS requirements reviewed:** REQ-001..004 carried from the ticket doc and
  sharpened with concrete verification; added REQ-005 (FULL gate green +
  shellcheck clean) to make "shippable" observable.
- **AAR id:** `a6c52a17-dd44-45e8-a23c-2ec92716b07b` (inspect→failure-record, complete→aar-submit capture into it)

## Phase 2 — Design

### Probe finding (load-bearing — the real root cause)
`cargo fmt --manifest-path src-tauri/Cargo.toml --check` fails today, NOT on
formatting but on workspace resolution:
> error: current package believes it's in a workspace when it's not … add the
> package to `workspace.exclude`, or add an empty `[workspace]` table to the
> package's manifest.
`src-tauri` lives inside the root-`[workspace]` directory but isn't a member, so
cargo refuses to operate on it at all. **That is why it's "outside the gates"
today — not policy, mechanics.** `src-tauri` already owns its own `Cargo.lock`
(the signature of a standalone workspace root), so the honest fix is to declare
it one with an empty `[workspace]` table. (Chosen over root `workspace.exclude`:
self-contained, matches the existing separate lock, canonical for a split tauri
shell.)

### Approach / architecture
- One new gate in `bin/gate.sh` — **gate:14 "tauri shell"** — placed at the end
  of the always-on section (so its cheap part runs in `--fast`), mode-aware:
  - **both modes:** `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
    (no compile, instant → REQ-002's "cheapest useful").
  - **FULL only:** `cargo clippy --manifest-path src-tauri/Cargo.toml
    --all-targets -- -D warnings` (compiles the tauri dep tree + lints →
    REQ-001's "compile-check and lint"). FULL-only because compiling tauri is
    the expensive part; keeps the quick loop painless.
- **Lint policy decision:** default clippy + `-D warnings` (the §14 baseline),
  NOT the workspace `pedantic/nursery/restriction` set. The shell is its own
  workspace and does not inherit root `[workspace.lints]`; duplicating that
  policy into the crate would be a second copy to keep in sync. Reversible
  (add `[lints]`/`[workspace.lints]` to `src-tauri` later for parity).
- **Numbering:** inserting gate:14 renumbers the FULL-only gates to **15 rust
  coverage / 16 js coverage / 17 mutation** (total **17 gates**; `--fast` now
  runs **14 static gates**). All live governing docs renumbered in the same
  change (REQ-004) — this IS the doc work the ticket asks for.
- **Already-covered, no change:** gate:7 machete already walks `src-tauri`
  (hence its justified ignore — REQ-003 is "don't churn it"); gate:8/10/11 text
  scans already list `src-tauri/src`. The only true gap was compile/fmt/lint.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `src-tauri/Cargo.toml` | Add empty `[workspace]` table (+ comment) so cargo can target the crate standalone. **Enabler — without it gate:14 errors.** Leave the `cargo-machete` ignore untouched (REQ-003). |
| 2 | `bin/gate.sh` | Add `tauri_shell_g()` + `run_gate "gate:14 tauri shell (fmt; +clippy full)"` after gate:13, before the FULL-only block. Renumber the 3 FULL-only `run_gate` labels 14/15/16→15/16/17. Update the `--fast` SKIP line `gate:14-16`→`gate:15-17`. Add a tauri line to the header tool-map comment. |
| 3 | `CONSTITUTION.md` | Gate enum (L24-42): insert `gate:14 tauri shell`, renumber coverage/mutation→15/16/17. Prose refs: L67 `Mutation (gate:16)`→17, L70 `Coverage gate:14`→15. Rewrite the "src-tauri … outside the compile gates" note (L72-74) to describe gate:14. Update "13 static gates" (≈L20)→14. |
| 4 | `docs/review-harness.md` | Add a bullet to the "currently runs" list (L27-42): `cargo fmt --check` + `cargo clippy` on the `src-tauri` shell crate (gate:14; fmt-only in `--fast`). |
| 5 | `CLAUDE.md` | Gate-list block: insert `14 tauri-shell`, renumber coverage/mutation→15/16/17; `**16 gates**`→`**17 gates**`; `--fast … 13 static gates`→`14 static gates`. |
| 6 | notes (this file) | Phase 3/4 records. |

### Regression Test Plan
"The gate IS the test" (tooling/config change, no app behavior — locked decision,
per the `WORK-ratchet-coverage-floors` precedent). §7 satisfied: the FULL gate
RUNS the existing suites (gate:3 `cargo test --workspace`, gate:4 `node --test`)
at Validate, plus negative smokes below. One check per AC.

| # | Test / Check | Proves Requirement |
|---|---|---|
| V1 | FULL `bin/gate.sh`: `gate:14 tauri shell` appears and PASS (clippy compiled `src-tauri`). | REQ-001 |
| V2 | Negative smoke: inject a clippy/compile error in `src-tauri/src/lib.rs` → FULL gate:14 FAILS → revert. | REQ-001 (truly compiles+lints) |
| V3 | FAST `bin/gate.sh --fast`: `gate:14 tauri shell` appears + PASS (fmt only), prints `GATE GREEN [fast]`; no tauri build. | REQ-002 |
| V4 | Negative smoke: misformat `src-tauri/src/lib.rs` → FAST gate:14 FAILS (fmt) → revert. | REQ-002 (fast validation is real) |
| V5 | gate:7 `cargo machete` stays PASS with `[package.metadata.cargo-machete] ignored=["serde","serde_json"]` intact. | REQ-003 |
| V6 | `grep` cross-check: CONSTITUTION (enum + note), review-harness, CLAUDE.md describe gate:14 + read 17 gates / 14 static; no stale "outside the compile gates". | REQ-004 |
| V7 | FULL `bin/gate.sh` → `GATE GREEN [full]`; `shellcheck -S info -e SC1091 bin/gate.sh` clean (gate:9). | REQ-005 |
- Genuinely-uncoverable: none. No new unit tests (config/tooling; the gate's exit codes + negative smokes are the verification).

### Risks / decisions
- **R1 (enabler):** `[workspace]` in `src-tauri/Cargo.toml` is mandatory or cargo can't target the crate. Chosen over root `exclude` (self-contained; matches existing own-lock).
- **R2 (lint policy):** default lints + `-D warnings`, not workspace pedantic/nursery — avoids a duplicated, drift-prone lint copy. Reversible.
- **R3 (fast cost):** clippy is FULL-only (tauri compile is heavy); fast = fmt-only.
- **R4 (build feasibility):** FULL gate:14 compiles tauri deps — slow first run, needs deps fetchable; repo has built tauri before (`src-tauri/gen/`, committed lock) → low risk. Confirm at Validate.
- **R5 (renumber churn):** mitigated by `grep`-ing all `gate:14/15/16` refs and updating every live doc in one change.
- **R6 (shellcheck):** new bash fn mirrors existing gate-fn style; gate:9 must stay clean.
- **Out of scope (honest gap):** `src-tauri` deps are still not audit/deny-gated and the shell is not in the coverage/mutation floors — future ratchet, consistent with the spec's "Out".

## Phase 3 — Implement
- **Built (5 files; config + bash + docs — no Rust/JS source, no tests):**
  - `src-tauri/Cargo.toml` — added empty `[workspace]` table (+ explanatory
    comment) so the standalone crate can be targeted by `cargo … --manifest-path`.
    The `[package.metadata.cargo-machete] ignored` block left untouched (REQ-003).
  - `bin/gate.sh` — new `tauri_shell_g()` + `run_gate "gate:14 tauri shell
    (fmt; +clippy full)"` after gate:13: `cargo fmt --manifest-path
    src-tauri/Cargo.toml --check` in both modes, `cargo clippy … --all-targets
    -- -D warnings` only when `MODE=full`. Renumbered the FULL-only gates
    14/15/16 → 15/16/17 (labels + `# NN.` comments + the `--fast` SKIP line
    `gate:15-17`). Added a `tauri-shell` line to the header tool-map.
  - `CONSTITUTION.md` — gate enum gains `gate:14 tauri shell`, coverage/mutation
    → 15/16/17; prose refs fixed (Mutation gate:17, Coverage gate:15); the
    "src-tauri … outside the compile gates" note rewritten to describe gate:14;
    counts → 17 gates / 14 static.
  - `docs/review-harness.md` — added the `src-tauri` fmt+clippy bullet (gate:14;
    fmt-only under `--fast`) to the "currently runs" list.
  - `CLAUDE.md` — gate-list block gains `14 tauri-shell`, renumber → 15/16/17;
    `16 gates`→`17 gates`; `all 16`→`all 17`; `13 static`→`14 static`.
- **In-phase checks (green):**
  - After the Cargo.toml fix, `cargo fmt --manifest-path src-tauri/Cargo.toml
    --check` → clean, and `cargo clippy … -- -D warnings` compiled the full tauri
    dep tree + the shell → **clean** in 18.9s (build now cached for Validate).
    This proves the workspace-resolution fix works and the shell is lint-clean.
  - `bash -n bin/gate.sh` parses; `shellcheck -S info -e SC1091 bin/gate.sh`
    **clean** (gate:9 stays green).
  - `grep` cross-check: no stale "outside the compile gates" / "16 gates" /
    "13 static"; all four files read 17 gates / 14 static and reference gate:14
    (tauri) + 15/16/17 consistently. The lone `bin/gate.sh:25 (gate:15)` is the
    untouched *aic* semgrep mapping ref, not our numbering.
- **Deviations from design:** none. The design's probe-driven `[workspace]`
  enabler was implemented exactly as planned; clippy confirmed clean with default
  lints + `-D warnings` (no need to relax anything).

## Inspect (Phase 3.5)
- **Lenses run:** 3 parallel `general-purpose` critics, each verifying concretely
  (ran cargo/shellcheck/grep), over: (1) gate-logic correctness + AC mapping,
  (2) side-effects/integration of the `[workspace]` change, (3) doc consistency
  + completeness (REQ-004). Verdicts: GATE-LOGIC-SOUND · NO-SIDE-EFFECTS ·
  DOCS-CONSISTENT.
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | LOW | CONSTITUTION note's "(a thin IPC wrapper)" sat beside "deps (audit/deny)", understating that src-tauri's *separate* ~435-crate dep tree is unaudited (`CONSTITUTION.md:75`). | **REAL** (doc-accuracy under REQ-004) | **Fixed** — split the two gaps: coverage/mutation off because first-party code is thin; the separate `Cargo.lock` dep tree (~400 crates) not yet audit/deny-gated. |
  | 2 | NIT | gate:14 is a new "partial-work-by-mode" pattern (fmt always, clippy FULL) unlike always-on siblings (`bin/gate.sh:211-220`). | REJECTED | Intentional + sound; single label honestly advertises `(fmt; +clippy full)`. Not a defect. |
  | 3 | NIT | Fast summary still prints `gate:14 … (+clippy full)` though clippy didn't run. | REJECTED | The `+clippy full` suffix discloses clippy is FULL-only; label is honest. |
  | 4 | NIT | Header tool-map `(gate:NN)` aic refs unchanged near edited lines (`bin/gate.sh:24-31`). | REJECTED | Pre-existing aic-parity annotations, correct as-is; not our numbering. |
  | 5 | nice-to-have | review-harness gate list has no numeric total; technical-architecture/decisions/team-handbook/package.json/.claude need no change. | REJECTED | Correct/safest form; verified none reference a stale count. |
- **Verified-clean (high-signal checks the critics actually ran):** all 4 exit
  paths of `tauri_shell_g` correct, no false-green vector (`run_gate` reads the
  exit code directly, no pipe); gate:14 runs in BOTH modes; renumber integrity =
  exactly one each of gate:1..17, no gaps/dupes; `shellcheck`/`bash -n` clean;
  `bin/gate.sh --fast` → GREEN with `SKIP gate:15-17`; FULL clippy re-run → exit
  0; root `cargo metadata`/`--workspace` exclude src-tauri (gated once); machete
  still flags src-tauri when its ignore is stripped (REQ-003 enforced); target
  dirs isolated (no coverage pollution).
- **Capture:** no `failure-record` (no code defect; finding #1 was a proactive
  doc-precision fix, not a bug). The supply-chain-gap insight (root audit/deny
  don't cover a standalone crate's separate lockfile) is queued for the Phase 5
  `aar-submit`/lesson.

## Phase 4 — Validate
- **Tests added:** none new (gate-is-test, config/tooling change — locked
  decision). Verification = the gate's own exit codes + negative smokes + the
  existing suites (run explicitly below).
- **Negative smokes (prove gate:14 actually catches drift):**
  - fmt drift appended to `src-tauri/src/lib.rs` → `cargo fmt --manifest-path … --check` **rc=1** (red) → reverted. (V4/REQ-002)
  - clippy lint (dead/unused) appended → `cargo clippy --manifest-path … -- -D warnings` **rc=101** (red) → reverted. (V2/REQ-001)
  - `git status` clean after revert; positive control: both commands rc=0 once restored.
- **`cargo test --workspace`:** ok — **20 passed; 0 failed** (+ 0 doc-tests).
- **`node --test tests/*.test.js`:** ok — **4 pass; 0 fail**.
- **`bin/gate.sh --fast`:** `GATE GREEN [fast]` — 14 passed/0 failed; **gate:14
  tauri shell PASS**; summary `SKIP gate:15-17 coverage+mutation`. (V3/REQ-002)
- **`bin/gate.sh` (FULL):** `GATE GREEN [full]` — **17 passed, 0 failed**:
  - gate:14 tauri shell **PASS** (clippy compiled src-tauri in FULL) — V1/REQ-001
  - gate:7 cargo-machete **PASS** (ignore intact) — V5/REQ-003
  - gate:15 rust cov **94.09%** ≥ 94 · gate:16 js cov **75.19%** ≥ 75 · gate:17 mutation **MSI 100.0%** (45 caught/0 missed)
  - shellcheck (gate:9) **PASS** — V7/REQ-005; FULL green wrote the commit-gate receipt.
- **Doc cross-check (V6/REQ-004):** counts read 17 gates / 14 static across
  CONSTITUTION + CLAUDE; no stale `16 gates`/`13 static`/`outside the compile
  gates`; gate:14 + 15/16/17 consistent.
- **Pre-existing exclusions:** none. `oathstar-server/src/main.rs` shows lower
  per-file coverage (71%), but the *workspace* line floor (94%) passes and `main`
  is the documented mutation exclusion — pre-existing, not in scope.
- **All AC verified:** REQ-001 ✓ REQ-002 ✓ REQ-003 ✓ REQ-004 ✓ REQ-005 ✓.

## Phase 5 — Complete
- **Docs updated:** governing docs were updated in Phase 3 (CONSTITUTION §0 gate
  enum + known-scope note, CLAUDE.md gate list + counts, docs/review-harness.md);
  inspect tightened the supply-chain wording. `docs/decisions.md` reviewed — no
  change needed (game-design log; its Tauri refs are runtime architecture, carry
  no gate count, and don't contradict gating the shell in place).
- **Forge capture:**
  - `failure-record` **BF-tauri-workspace-resolution-001** (32921aab) — the cargo
    "believes it's in a workspace when it's not" blocker.
  - `prevention-rule-record` **PR-claude-cargo-workspace-001** (46b5ca93) — add an
    empty `[workspace]`/`exclude` and gate non-members via `--manifest-path`, never
    `--workspace`/`--all`.
  - `architecture-decision-record` **AD-claude-tauri-gate-001** (cba8fdeb) — gate
    src-tauri in place as a standalone workspace root; residual gaps deferred.
  - `aar-submit` **a6c52a17** — outcome completed, effectiveness 5; 3 novel
    findings (distillation / confidence-drift / pattern-emergence jobs enqueued).
- **Ticket closed:** forge #4 (2885b6ed) → `done`; local doc moved open/→closed/.
- **Archived:** pipeline doc pair moved active/→completed/.
