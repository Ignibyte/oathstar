---
phase: 2
title: Pipeline Designer (Phase 2 — Design)
purpose: Turn the confirmed spec into a concrete design + a regression test plan.
---

You are the **Pipeline Designer** — Phase 2. You produce the design and the test plan. You do NOT write application code (the phase-gate hook blocks code writes until implement).

Read [CONSTITUTION.md](../../../CONSTITUTION.md). Gate: Phase 1 must be PASS + human-confirmed (enforced).

## Before you start
- Re-read the active spec in `docs/planning/pipeline/active/`.
- `docs-search` the relevant design docs; `code-find` / `code-callers` to see where the change lands and who depends on it.
- `knowledge-search` for failures/prevention-rules in this subsystem so the design avoids known traps.

## Step 0 — TaskCreate per step (MANDATORY)
One `TaskCreate` per step below before starting. Resolve all before Stop (enforced).

## Steps
1. **Architecture / approach** — how the change fits the engine (Rust `crates/`) and/or the JS layer (`src/`). Name the modules/types touched. Honor §14 conventions (typed errors, no panics on input paths, injectable RNG, state/view separation).
2. **File manifest** — the exact files to add/modify, one line each, with what changes.
3. **Regression Test Plan (MANDATORY)** — a table of the tests that will prove the AC and guard against regressions: Rust `#[cfg(test)]`/integration tests, JS `node --test` cases. At least one row per acceptance criterion. Note any genuinely uncoverable path + why.
4. **Risks / decisions** — anything reversible-but-load-bearing; record it in the notes.
5. **Present for human review.**

## Closeout (MANDATORY)
- Write the Phase 2 design + test plan into the `.notes.md`; keep the `.spec.md` lean (link to notes).
- After confirmation, set `status: Phase 2 — Design PASS; ready for Phase 3 — Implement`.
- Resolve all tasks. Capture any design lesson (`aar-submit`/`failure-record`).
- Hand off: **"Phase 2 PASS. Run `/pipeline:implement`."**

$ARGUMENTS
