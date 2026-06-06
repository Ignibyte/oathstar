---
pipeline_id: 3154b525-57fc-4081-a3f0-ef409bd691da
title: WORK-add-tauri-shell-quality-gate
ticket: 2885b6ed-5deb-403b-afcb-67f80b35eb1d
type: work
intake:
notes: WORK-add-tauri-shell-quality-gate.notes.md
status: Phase 5 — Complete PASS
---

# WORK-add-tauri-shell-quality-gate

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Bring `src-tauri` (the Tauri shell crate) under `bin/gate.sh` so
  shell-side Rust is compile-checked/linted instead of drifting unverified.
- **Scope:**
  - **In:** add gate step(s) to `bin/gate.sh` that explicitly target the
    `src-tauri` crate (it is outside the root workspace, so `--workspace`/`--all`
    skip it); wire the appropriate treatment into FULL and FAST modes; update the
    governing docs (`CONSTITUTION.md` lines ~72-73, `docs/review-harness.md`, and
    `CLAUDE.md`'s 16-gate list if a gate count changes) to describe the Tauri gate
    scope accurately; keep `cargo machete`'s justified unused-dep exception intact.
  - **Out:** folding `src-tauri` into the root workspace; new Tauri features or
    `#[tauri::command]`s; frontend redesign; installer/packaging; adding `src-tauri`
    to the Rust coverage (gate:14) or mutation (gate:16) floors.
- **Systems:** engine/tooling — quality-gate harness (`bin/gate.sh`) + the Tauri
  shell crate (`src-tauri/`) + governing docs.

## Acceptance Criteria (EARS)
Each acceptance criterion uses EARS syntax, describes one observable behavior,
and includes a verification method.

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the FULL gate runs, `bin/gate.sh` shall compile-check and lint the `src-tauri` crate via an explicit command (not via `--workspace`/`--all`), and report a labeled PASS/FAIL gate for it. | FULL `bin/gate.sh`: a Tauri gate label appears and PASSes; verify it fails if `src-tauri/src` is made non-compiling/unformatted (negative smoke). |
| REQ-002 | When the FAST gate runs, `bin/gate.sh --fast` shall give the `src-tauri` crate the cheapest useful validation that does not make the quick loop painful. | FAST `bin/gate.sh --fast`: the Tauri gate appears in the summary and prints `GATE GREEN [fast]`; its cost is bounded (documented in design). |
| REQ-003 | If `src-tauri` dependencies are intentionally unused during early shell setup, then the gate shall keep enforcing the documented, justified exception rather than churning the deps. | gate:7 `cargo machete` stays green with `src-tauri/Cargo.toml`'s `[package.metadata.cargo-machete] ignored = ["serde","serde_json"]` intact; no removal of those deps. |
| REQ-004 | The governing docs shall describe the Tauri gate scope accurately (no longer stating the shell is simply "outside the compile gates"). | Doc check: `CONSTITUTION.md` (~L72-73) + `docs/review-harness.md` gate list updated; grep cross-check that they match the gate script. |
| REQ-005 | The FULL gate shall pass green at the new Tauri gate. | `bin/gate.sh` → `GATE GREEN [full]`; `shellcheck` (gate:9) stays clean on the edited script. |

## Locked-In Decisions
- **`src-tauri` stays a standalone crate** (own `Cargo.lock`, own `tauri`
  toolchain). This ticket gates it *in place* via an explicit command; it does
  **not** fold the shell into the root workspace (larger, riskier change — out of
  scope, keeps this to one shippable slice).
- **The gate is the test.** This is a tooling change; verification is the gate's
  own exit codes plus a negative smoke check, not new unit tests (no application
  behavior changes). House style per `WORK-ratchet-coverage-floors`.
- **Preserve the justified `cargo-machete` ignore** for `serde`/`serde_json` in
  `src-tauri/Cargo.toml` — do not churn the deps to satisfy a gate.
- **Coverage/mutation floors stay workspace-only** — the shell is a thin
  IPC wrapper; gating its *compile + lint* (not its coverage) is the slice.

## Linked Artifacts
- Design docs: `CONSTITUTION.md` (§ compile-gates note, ~L72-73), `docs/review-harness.md`, `CLAUDE.md` (gate list)
- Intake doc: none (ticket pre-existed)
- Ticket doc: `docs/planning/tickets/open/TICKET-4-add-tauri-shell-quality-gate.md`
- Forge ticket: `2885b6ed-5deb-403b-afcb-67f80b35eb1d` (#4)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
