# Oathstar Constitution

The binding rules for building Oathstar. The pipeline skills and the
`.claude/hooks/` enforcement layer treat this document as law. When a hook
blocks you, it cites a section here. Sections are stable anchors — hooks grep
for `§N`.

Oathstar is a single-player, MUD-inspired text RPG: a Rust engine
(`crates/`), a JS prototype/frontend (`src/`, `tests/`), and a Tauri shell
(`src-tauri/`). It is paired with the **oathstar-forge** knowledge sidecar
(MCP server `forge` in `.mcp.json`) — the pipeline records what it learns
there and recalls it on the next run.

---

## §0 — Quality Gates (binding)

The canonical gate is **`bin/gate.sh`** — a Rust port of the aic PHP quality
stack. The FULL gate (16 gates) must pass green before `/commit`; `--fast` runs
the 13 static gates for a quick local loop but prints `GATE GREEN [fast]` — only
a FULL green writes the receipt the commit hook requires (§15), so `--fast`
can't satisfy `/commit`.

```
gate:1  rustfmt        cargo fmt --all --check                  ← Pint
gate:2  clippy         cargo clippy --workspace --all-targets   ← PHPStan L10 + PHPMD + PHPCS
        (strict)         --all-features -- -D warnings
gate:3  cargo test     cargo test --workspace                   ← PHPUnit (strict)
gate:4  js test        node --test tests/*.test.js
gate:5  audit          cargo audit                              ← composer audit + roave
gate:6  supply chain   cargo deny check                         ← dep hygiene / license / sources
gate:7  unused deps    cargo machete                            ← composer-unused
gate:8  secrets        gitleaks (history + working tree)        ← gitleaks
gate:9  shell lint     shellcheck -S info (hooks + bin)         ← shellcheck
gate:10 no-suppress    grep meta-gate (allow/expect + justify)  ← NoBaselinesGate
gate:11 source-bans    grep meta-gate (SAST primitives)         ← semgrep anti-patterns
gate:12 lints-base     diff meta-gate (allow-list baseline)     ← NoBaselinesGate (config)
gate:13 doc-todos      grep meta-gate                           ← doc-todos
gate:14 rust coverage  cargo llvm-cov --fail-under-lines $RUST_COV_MIN   [FULL]
gate:15 js coverage    node --experimental-test-coverage (floor $JS_COV_MIN) [FULL]
gate:16 mutation       cargo mutants  (MSI ≥ $MUT_MSI_MIN%)     ← Infection MSI [FULL]
```

Strict static analysis lives in `[workspace.lints]` (Cargo.toml): clippy
`all` + `pedantic` + `nursery` are **deny**, plus restriction lints
(`dbg_macro`, `todo`, `unimplemented`, `mem_forget`). The allow-list there is
the **justified ratchet backlog** — debatable lints for a young game engine,
fixed and removed over time. gate:12 pins it to `bin/.clippy-allowlist`, so it
can't quietly grow into a de-facto baseline: any change to it must update the
baseline file in the same (visible, reviewed) commit.

**No baselines. No suppressions. Source-fix only.** Any inline `#[allow(…)]` /
`#[expect(…)]` in game source must carry a real `//` justification (gate:10,
clippy *and* rustc lints); blanket group allows (`clippy::all`/`pedantic`/
`nursery`/`correctness`/…/`warnings`/`unused`) are banned outright. Banned source
primitives — process spawning, `mem::transmute`, `unsafe` without `// SAFETY:` —
fail gate:11. A `.skip`/`xfail` is a violation unless the spec's test plan
records why and a follow-up exists.

**Floors ratchet up, never down — and never below the §0 minimum.** The minimums
(`RUST_COV_MIN=94`, `JS_COV_MIN=75`, `MUT_MSI_MIN=100`) are baked into the gate;
env may *raise* a floor but a lower value is clamped back up, so `MUT_MSI_MIN=0
bin/gate.sh` still enforces it. Mutation is at **100% MSI — parity with aic's
Infection floor**; coverage actuals sit above the floors (~94% rust lines, ~75%
js lines). Lowering a floor to pass is a charter violation — write the test.

**Known scope (honest — not yet a complete 1:1 of aic).** Mutation (gate:16) is
at MSI 100% over the whole workspace; the **sole excluded function is `fn main`**
(the composition root — binds a socket and serves forever, no unit-testable
contract; recorded in `.cargo/mutants.toml`). Coverage gate:14 is a *workspace
line* floor, not aic's per-file 100% line+branch+function (branch coverage needs
nightly llvm-cov and isn't gated yet). `src-tauri` (the Tauri shell) is outside
the compile gates — it's built by `tauri` with its own toolchain — but its source
is covered by the text gates (8/10/11). Not yet ported: architecture layering
(deptrac), refactor-drift (Rector), taint analysis (Psalm). These are the ratchet
roadmap, recorded so the gap is explicit.

The gate runs ALL steps and reports each; a single red step fails the gate.
Every verdict is the tool's exit code, never a grep of its output. On a FULL
green it writes a worktree-bound receipt (`.git/oathstar-gate-receipt`) that
`enforce-commit-gate.sh` validates at commit (§15).

---

## §3 — Phase Gates (binding)

Work flows through an ordered pipeline. **Every phase has an entry gate: the
previous phase must be `PASS` (and, for plan/design, human-confirmed) before
the next begins.** The `enforce-phase-gate.sh` PreToolUse hook blocks
`Write`/`Edit` to application code until the gate is satisfied.

```
/work        pre-flight (env, forge, bulletins, context) → hands off to plan
  → /pipeline:plan       (Phase 1)  forge ticket/doc + active spec/notes
  → /pipeline:design     (Phase 2)  design + regression test plan
  → /pipeline:implement  (Phase 3)  code
  → /pipeline:inspect    (Phase 3.5) adversarial review checkpoint (§18)
  → /pipeline:validate   (Phase 4)  write + RUN tests; gate green
  → /pipeline:complete   (Phase 5)  docs + AAR to forge + archive
  → /commit              delivery gate: full bin/gate.sh + commit/PR
```

- **NEVER have two pipeline documents active** in `docs/planning/pipeline/active/`.
- A pipeline doc is a `<title>.spec.md` + `<title>.notes.md` pair. The `.spec.md`
  carries `pipeline_id:` (a real UUID) and `status:` frontmatter. Phase advance =
  setting `status: Phase N — <Title> PASS; ready for Phase M — <Title>`.
- Work that is not ready for a forge ticket lives in `docs/planning/intake/`.
  Intake docs are candidates, not active pipeline docs.
- Every forge ticket created for Oathstar work must have a local ticket document
  in `docs/planning/tickets/open/` or `docs/planning/tickets/closed/`; the
  `ticket:` frontmatter is the canonical link.
- When a forge ticket becomes active implementation work, it also gets the
  pipeline `.spec.md` + `.notes.md` pair in `active/`; completed pipeline docs
  move to `completed/`.
- Pipeline acceptance criteria use EARS: `shall`, one observable behavior per
  requirement, and a verification method for each requirement.
- Application code = `crates/**/*.rs`, `src/**/*.js`, `src-tauri/src/**/*.rs`,
  `tests/**`. Docs, `.claude/`, config, and `docs/planning/` are not gated.

---

## §7 — Testing Standards (binding)

**Full testing is expected.** Every pipeline produces meaningful tests for the
code it writes — Rust `#[cfg(test)]` / integration tests, JS `node --test`.
Tests are not optional and not skippable because a change "looks simple."

- **NEVER mark a phase PASS if tests did not actually RUN.** Writing a test
  file is not testing. The `enforce-tests-ran.sh` Stop hook checks the
  transcript for real `cargo test` / `node --test` invocations at
  `/pipeline:validate`.
- **Pre-existing failures are not your problem — but document them.** Note them
  in the pipeline notes as "pre-existing" and move on; don't fix unrelated
  breakage unless asked.
- **Genuinely uncoverable paths** (need external services/hardware) get a
  documented skip with the reason recorded in the pipeline notes.

---

## §14 — Code Conventions (binding)

**Rust** (`crates/`, `src-tauri/`):
- Matches the surrounding code's idiom; `cargo fmt` is law (gate:1).
- No `unwrap()`/`expect()` on fallible paths that a malformed input can reach —
  return a typed error. `expect()` is acceptable only for genuine invariants
  with a message that states the invariant.
- Public items carry doc comments. Prefer `&str`/slices over needless clones.
- clippy clean at `-D warnings`, `--all-targets` (gate:2). An `#[allow]` needs a
  trailing-comment justification.

**JavaScript** (`src/`, `tests/`):
- Pure, deterministic engine logic (`engine.js`) — the RNG is injectable; no
  hidden globals. The view/render layer is separate from game-state mutation.
- One concern per module; match the existing functional style.

**Both:** no secrets in source; reuse existing helpers before adding new ones;
comments explain *why*, not *what*.

---

## §15 — Anti-Circumvention (binding)

**The transcript is the source of truth. If it didn't happen in the
transcript, it didn't happen.** Claiming "tests pass" without a visible test
run is a violation. Claiming a gate is green without running `bin/gate.sh` is a
violation. Hooks evaluate evidence (tool calls, Bash commands, file state), not
prose.

Do not weaken a gate, delete a test, lower a coverage floor, or add a blanket
`#[allow]` to get past a blocked Stop. Fix the cause.

**What the enforcement is — and isn't.** The hooks are a *discipline scaffold*,
not a security boundary. They reliably catch **omissions** — writing code before
a phase is PASS, stopping a phase with unresolved tasks, reaching `/pipeline:validate`
without running tests, completing without capturing knowledge, committing code
without a green gate. They do **not** try to defeat deliberate fabrication: the
`status:` line and the AAR/test/capture calls are self-reported, and a determined
agent could forge them. The one hard, evidence-based gate is **`bin/gate.sh` at
commit** — `enforce-commit-gate.sh` blocks `git commit` of code unless a FULL
gate run left a *receipt* (`.git/oathstar-gate-receipt`, a content fingerprint of
the gated source) that still matches the worktree being committed. The receipt is
written only by a real FULL green, so the verdict cannot be forged by printing or
quoting `GATE GREEN` in prose, by `echo`-ing it, or by reading a file that
contains it; and any edit after the green — by Write, Edit, or a Bash heredoc —
changes the fingerprint and re-blocks. The gate trusts only tool exit codes and
clamps floors to the §0 minimums, so a green can't be bought by lowering a bar.
So: the pipeline is how we keep ourselves honest; the commit gate is what's
actually load-bearing. Don't forge status to skip work — the only thing it buys
is shipping past the one gate that re-runs everything anyway.

---

## §18 — Inspect & Explore (binding)

**§18.1 — Mandatory inspect checkpoint.** After implementation (Phase 3, and
Phase 3.5 if styling), `/pipeline:inspect` runs an adversarial review before
validation. It spawns independent critics (correctness, security/secrets,
data-integrity, simplification) against the diff, then the lead reviews the
findings and fixes the real ones. This is the same adversarial-critic loop the
project uses by hand — `/pipeline:inspect` makes it a phase. The phase-gate
requires Phase 3.5 PASS before `/pipeline:validate`; populating the inspect
ledger in the notes is required by the command (no hook checks the ledger body
— it's a convention, kept honest by review).

**§18.2 — Delegate broad file-discovery to the Explore subagent.** Any lookup
with more than ~3 candidate paths, or where the location isn't known a priori,
goes through `Agent(subagent_type=Explore)`. Inline grep walks don't substitute.

**§18.3 — Forge first.** Before planning and before implementing, recall prior
knowledge: `knowledge-context` / `knowledge-search` (lessons, failures,
prevention rules) and `docs-search` (the design docs). At phase close, record
what you learned: `aar-submit` (lessons), `failure-record` (what bit you). The
sidecar only gets smarter if you feed it.

---

## §19 — Forge Process (push-only)

Oathstar owns its pipeline locally. The forge sidecar is for *this* project's
knowledge — capture to it, recall from it. Knowledge flows in (capture) and out
(recall); the pipeline definition itself is not pulled from elsewhere.
`.mcp.json` is owner-managed — the `forge` server points at the local
oathstar-forge sidecar (`scripts/start-all.sh` in `../oathstar-forge`).

---

## Amending this Constitution

These rules change deliberately, not mid-pipeline to dodge a gate. To amend:
state the section, the change, and the reason in a commit that touches only
this file (and any hook that enforces the changed rule). Raising a coverage
floor or tightening a convention needs no ceremony; loosening one needs a
recorded reason.
