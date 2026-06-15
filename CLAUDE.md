# CLAUDE.md — Oathstar

Guidance for Claude Code working in this repo.

## What this is

**Oathstar** is a single-player, MUD-inspired text RPG: a Rust engine
(`crates/oathstar-*`), a working JS prototype/frontend (`src/`, `tests/`), and
a Tauri shell (`src-tauri/`). Design docs live in `docs/`; locked design
choices in `docs/decisions.md`. The engine is mid-migration from the JS
prototype to the Rust workspace. The secure admin/management UI is a **separate
loopback Rust sidecar** (`oathstar-studio`) on a shared `oathstar-auth` crate —
not routes on the public game server (Decision 058).

## How we work — the pipeline (binding: CONSTITUTION.md)

Non-trivial work flows through a phase-gated pipeline. Enforcement hooks in
`.claude/hooks/` make the gates real (they no-op outside a pipeline session).

```
/work                  pre-flight: forge up? tooling? bulletins? recall context
  → /pipeline:plan       forge ticket/doc + active spec/notes
  → /pipeline:design     design + regression test plan
  → /pipeline:implement  code (recall from forge first — §18.3)
  → /pipeline:inspect    adversarial critic review of the diff, then fix (§18.1)
  → /pipeline:validate   write + RUN tests; bin/gate.sh green
  → /pipeline:complete   docs + capture knowledge to forge + archive
  → /commit              full bin/gate.sh, then commit/PR
```

Pre-ticket intake lives in `docs/planning/intake/`. Forge-backed backlog lives
in `docs/planning/tickets/{open,closed}/`; every forge ticket must have a local
ticket doc there. Pipeline docs live in
`docs/planning/pipeline/{active,completed,_templates}/` as a `<title>.spec.md`
plus `.notes.md` pair for the single active implementation item. Acceptance
criteria use EARS (`shall`, one observable behavior, verification method).

If the user waives the pipeline ("just do it"), do the work directly — but
still recall from the forge first and capture lessons after.

## The gate — `bin/gate.sh` (binding: CONSTITUTION §0)

The single source of truth for shippable — a Rust port of aic's PHP quality
stack. Strict, no baselines, source-fix only. **17 gates**:

```
1 rustfmt   2 clippy(strict)  3 cargo test  4 node --test  5 cargo-audit
6 cargo-deny  7 cargo-machete  8 gitleaks(history+worktree)  9 shellcheck
10 no-suppressions  11 source-bans(SAST)  12 lints-allowlist  13 doc-todos
14 tauri-shell(fmt; +clippy FULL)
[FULL] 15 rust coverage  16 js coverage  17 mutation (cargo-mutants)
```

- `bin/gate.sh` — FULL (all 17, incl. coverage + mutation). Required before `/commit`.
- `bin/gate.sh --fast` — the 14 static gates, for a quick loop (prints `GATE GREEN
  [fast]`). Only a FULL green writes the commit-gate receipt, so `--fast` can't
  satisfy `/commit`.
- Strict clippy (pedantic+nursery+restriction) is in `[workspace.lints]`; gate:12
  pins its allow-list to `bin/.clippy-allowlist` (can't grow into a baseline).
- Floors are baked-in minimums env can only raise: `RUST_COV_MIN=94`,
  `JS_COV_MIN=75`, `MUT_MSI_MIN=100` (mutation at 100% MSI — aic parity; the lone
  excluded fn is `main`, see `.cargo/mutants.toml`).
- On a FULL green the gate writes `.git/oathstar-gate-receipt`; the commit hook
  blocks `git commit` of code unless that fingerprint matches the worktree.

Tools: `cargo install cargo-mutants cargo-audit cargo-deny cargo-machete cargo-llvm-cov`,
`brew install gitleaks shellcheck`. Run the gate before `/commit`; fix every red at source.

## The forge sidecar (knowledge)

Oathstar is paired with **oathstar-forge** (`../oathstar-forge`), a knowledge +
codegraph + doc-search MCP service registered as the `forge` server in
`.mcp.json` (tools appear as `mcp__forge__*`). Start it with
`../oathstar-forge/scripts/start-all.sh`.

- **Recall** before planning/coding: `knowledge-search`, `knowledge-context`
  (lessons/failures/prevention rules), `docs-search` (design docs),
  `code-find`/`code-callers` (the codegraph over Rust + JS).
- **Capture** at phase close: `aar-submit` (lessons), `failure-record`,
  `prevention-rule-record` / `architecture-decision-record`.
- Tickets/sprints/bulletins are there too (`ticket-*`, `bulletin-list`).

`.mcp.json` is owner-managed — don't overwrite it (gitignored: holds the bearer).

## Code conventions (binding: CONSTITUTION §14)

Rust: `cargo fmt` law; clippy-clean at `-D warnings`; typed errors over
`unwrap()/expect()` on input paths; doc-comment public items. JS: deterministic
engine (injectable RNG), state/view separated, functional style. No secrets in
source. Reuse before adding (`code-find` first).
