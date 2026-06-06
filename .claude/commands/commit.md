You are the **Delivery Gate** — `/commit`. The sole truth gate: the FULL `bin/gate.sh` runs here, then you commit (and open a PR if asked). Run after `/pipeline:complete`.

Read [CONSTITUTION.md](../../CONSTITUTION.md) §0 (no baselines, source-fix only) and §15 (transcript is truth). The `enforce-commit-gate.sh` hook blocks this commit unless `bin/gate.sh` printed `GATE GREEN` after your last code change — so step 1 is mandatory, not advisory.

## Step 0 — TaskCreate
Create: "run full gate", "stage", "commit", "(PR if asked)". Resolve all before Stop.

## Steps
1. **Run the full gate and paste the real output:**
   ```bash
   bin/gate.sh
   ```
   It must end `GATE GREEN`. If any step is red, STOP and fix at the source — do not stage, do not weaken a gate, do not lower a coverage floor. Re-run until green.
2. **Confirm the pipeline is complete** — `docs/planning/pipeline/active/` is empty (the doc was archived at Phase 5) and the forge ticket is closed.
3. **Stage + review** — `git add -A` then show `git diff --cached --stat`; sanity-check nothing unintended (no `.env`, no `.mcp.json`, no secrets) is staged.
4. **Commit** — a clear message: a subject line, a body explaining *why*, the ticket id, and end with:
   ```
   Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
   ```
   If on the default branch, branch first.
5. **PR (only if the user asked)** — `gh pr create` with a body ending:
   ```
   🤖 Generated with [Claude Code](https://claude.com/claude-code)
   ```

## Closeout
- Resolve all tasks. Report: gate result, commit SHA, branch/PR.
- Commit/push only what the user authorized.

$ARGUMENTS
