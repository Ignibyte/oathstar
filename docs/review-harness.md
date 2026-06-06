# Review Harness

This document defines the review gate for implementation work.

Claude or another coding agent may write code. Codex reviews the work against the architecture docs, runs the verification harness, and calls out regressions before changes are considered ready.

## Review Role

Codex review should check:

- Does the change follow [Design Decisions](./decisions.md)?
- Does it preserve the Rust server as the game authority?
- Does it keep frontend/client code separate from core game rules?
- Does it keep content TOML-first where content definitions are involved?
- Does it keep domain events renderer-agnostic?
- Does it avoid hardcoding Beginner-module assumptions into the core engine?
- Does it add tests or validation for risky behavior?

## Required Gate

Before blessing implementation work, run:

```bash
bin/gate.sh
```

This is the canonical full gate and currently runs:

- `cargo fmt --all -- --check`
- strict `cargo clippy`
- `cargo test --workspace`
- `npm test`
- `cargo audit`
- `cargo deny check`
- `cargo machete`
- `gitleaks`
- `shellcheck`
- no-suppressions/source-ban/lint-allowlist/doc gates
- Rust coverage (≥94% lines)
- JS coverage (≥75% lines)
- mutation testing (100% MSI)

For fast iteration, run:

```bash
bin/gate.sh --fast
```

`npm run verify` and `npm run verify:quick` are wrappers around those same
commands for convenience.

## API Smoke Checks

When server behavior changes, also run a live smoke check:

```bash
npm run server:dev
```

Then in another terminal:

```bash
curl -s http://127.0.0.1:7878/health
curl -s http://127.0.0.1:7878/state
curl -s -X POST http://127.0.0.1:7878/command \
  -H 'content-type: application/json' \
  -d '{"input":"north"}'
curl --max-time 3 -sN http://127.0.0.1:7878/events
```

Stop the server after smoke checks.

## Review Output

Codex should review like a code reviewer:

1. Findings first, ordered by severity.
2. File and line references when possible.
3. Test results and any commands that failed.
4. Short summary of what changed only after findings.

If no issues are found, say that clearly and mention any remaining risk or untested area.

## Non-Negotiables

- Do not import ROT/ROM/Diku code without license review.
- Do not let modules mutate arbitrary state outside core validation.
- Do not put raw functions into save data.
- Do not make LLM output authoritative for deterministic state.
- Do not make Tauri the backend authority.
- Do not bypass the typed domain event model for player-facing output.
