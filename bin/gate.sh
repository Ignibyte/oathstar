#!/usr/bin/env bash
# =============================================================================
# bin/gate.sh — Oathstar canonical quality gate (1:1 strict, air-tight)
# =============================================================================
#
# The single source of truth for "is this change shippable?". Invoked by
# /commit (the delivery gate) and the enforce-commit-gate.sh hook. Strict by
# charter (CONSTITUTION §0): no baselines, no suppressions, source-fix only.
#
# Runs ALL gates, reports each, exits non-zero if any failed (full picture per
# run). Every gate's verdict is the tool's EXIT CODE — never a grep of output.
# Bash 3.2 compatible (macOS /bin/bash) — no mapfile/declare -A.
#
# The Rust port of aic's PHP quality stack:
#   fmt          rustfmt            ← Pint
#   clippy       clippy (strict)    ← PHPStan L10 + PHPMD + PHPCS
#   test         cargo test         ← PHPUnit (strict)
#   js-test      node --test        ← (JS suite)
#   audit        cargo-audit        ← composer audit + roave/security-advisories
#   deny         cargo-deny         ← dependency hygiene + license + sources
#   machete      cargo-machete      ← composer-unused
#   secrets      gitleaks           ← gitleaks (history + working tree)
#   sh-lint      shellcheck         ← shellcheck
#   no-suppr     grep meta-gate     ← NoBaselinesGate (gate:21)
#   source-bans  grep meta-gate     ← semgrep anti-pattern rules (gate:15)
#   lints-base   diff meta-gate     ← NoBaselinesGate, config side (gate:21)
#   doc-todos    grep meta-gate     ← doc-todos (gate:24)
#   tauri-shell  fmt + clippy       ← src-tauri (standalone) compile gate
#   studio-css   tailwind no-drift  ← compiled studio.css == fresh DaisyUI build (gate:18)
#   [FULL] rust-cov  cargo-llvm-cov ← coverage floor (gate:20)
#   [FULL] js-cov    node coverage  ← (JS coverage)
#   [FULL] mutation  cargo-mutants  ← Infection MSI (gate:12)
#
# Modes:
#   bin/gate.sh           FULL — every gate (the /commit + merge sweep).
#   bin/gate.sh --fast    FAST — skips coverage + mutation (quick local loop).
#   GATE_FAST=1 also selects FAST. /commit always runs FULL.
#
# Floors (env may RAISE to ratchet; the §0 minimums below are a HARD floor env
# can NEVER lower — `MUT_MSI_MIN=0 bin/gate.sh` is clamped back up to 100):
#   RUST_COV_MIN=94  JS_COV_MIN=75  MUT_MSI_MIN=100
# =============================================================================

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)" || exit 2

# Shared helpers — gate_state_hash() for the commit-gate receipt. Sourcing is
# side-effect-free beyond setting PROJECT_ROOT (which equals our CWD here).
# shellcheck source=.claude/hooks/lib-hook-helpers.sh
. ./.claude/hooks/lib-hook-helpers.sh 2>/dev/null \
  || { echo "FATAL: cannot load .claude/hooks/lib-hook-helpers.sh" >&2; exit 2; }

# Floors. The §0 minimums are baked in here, not in overridable env: env may
# raise a floor (ratchet up) but a value below the minimum is clamped back up,
# so a green can never be bought by lowering the bar.
# §0 minimums — a single source of truth, baked in. env may RAISE a floor
# (ratchet up) but a value below the minimum is clamped back up, so a green can
# never be bought by lowering the bar (`MUT_MSI_MIN=0 bin/gate.sh` runs at 100).
RUST_COV_FLOOR=94; JS_COV_FLOOR=75; MUT_MSI_FLOOR=100
RUST_COV_MIN="${RUST_COV_MIN:-$RUST_COV_FLOOR}"
JS_COV_MIN="${JS_COV_MIN:-$JS_COV_FLOOR}"
MUT_MSI_MIN="${MUT_MSI_MIN:-$MUT_MSI_FLOOR}"
if awk -v c="$RUST_COV_MIN" -v f="$RUST_COV_FLOOR" 'BEGIN{exit !(c+0 < f+0)}'; then echo "note: RUST_COV_MIN below the §0 minimum $RUST_COV_FLOOR — clamped." >&2; RUST_COV_MIN=$RUST_COV_FLOOR; fi
if awk -v c="$JS_COV_MIN"   -v f="$JS_COV_FLOOR"   'BEGIN{exit !(c+0 < f+0)}'; then echo "note: JS_COV_MIN below the §0 minimum $JS_COV_FLOOR — clamped." >&2; JS_COV_MIN=$JS_COV_FLOOR; fi
if awk -v c="$MUT_MSI_MIN"  -v f="$MUT_MSI_FLOOR"  'BEGIN{exit !(c+0 < f+0)}'; then echo "note: MUT_MSI_MIN below the §0 minimum $MUT_MSI_FLOOR — clamped." >&2; MUT_MSI_MIN=$MUT_MSI_FLOOR; fi

MODE="full"
case "${1:-}" in --fast|fast) MODE="fast" ;; esac
[ "${GATE_FAST:-0}" = "1" ] && MODE="fast"

PASS=0; FAIL=0
RESULTS=()

run_gate() {
  local label="$1"; shift
  printf '\n\033[1m▶ %s\033[0m\n    %s\n' "$label" "$*"
  if "$@"; then RESULTS+=("PASS  $label"); PASS=$((PASS + 1))
  else RESULTS+=("FAIL  $label"); FAIL=$((FAIL + 1)); fi
}

need() { command -v "$1" >/dev/null 2>&1 || { echo "MISSING TOOL: $1 — $2" >&2; return 1; }; }

# ── 1. rustfmt ───────────────────────────────────────────────────────────────
run_gate "gate:1  rustfmt" cargo fmt --all --check

# ── 2. clippy (strict: pedantic+nursery+restriction via [workspace.lints]) ───
run_gate "gate:2  clippy (strict)" cargo clippy --workspace --all-targets --all-features -- -D warnings

# ── 3. rust tests ────────────────────────────────────────────────────────────
run_gate "gate:3  cargo test" cargo test --workspace

# ── 4. js tests (node exit code is the verdict; require ≥1 test file) ─────────
js_tests() {
  shopt -s nullglob; set -- tests/*.test.js; shopt -u nullglob
  [ "$#" -gt 0 ] || { echo "no JS test files matched tests/*.test.js"; return 1; }
  node --test "$@"
}
run_gate "gate:4  node --test" js_tests

# ── 5. security advisories (RUSTSEC) ─────────────────────────────────────────
audit_g() { need cargo-audit "cargo install cargo-audit" || return 1; cargo audit; }
run_gate "gate:5  cargo-audit" audit_g

# ── 6. supply chain (advisories/licenses/bans/sources) ───────────────────────
deny_g() { need cargo-deny "cargo install cargo-deny" || return 1; cargo deny check; }
run_gate "gate:6  cargo-deny" deny_g

# ── 7. unused dependencies ───────────────────────────────────────────────────
machete_g() { need cargo-machete "cargo install cargo-machete" || return 1; cargo machete; }
run_gate "gate:7  cargo-machete" machete_g

# ── 8. secrets — committed history AND the working tree ──────────────────────
# `detect` scans git history (catches anything committed); `dir` scans the
# working tree per source dir (catches a secret in the SAME change that adds it
# — the pre-commit window `detect` alone misses). .gitleaks.toml extends the
# default rules with Anthropic-key + forge-bearer shapes. Scanning named source
# dirs (not `.`) keeps target/ out of the walk — fast and no build-artifact FPs.
secrets_g() {
  need gitleaks "brew install gitleaks" || return 1
  local cfg=".gitleaks.toml" d
  gitleaks detect --no-banner -s . -c "$cfg" || return 1
  for d in crates src src-tauri/src bin tests; do
    [ -d "$d" ] || continue
    gitleaks dir "$d" --no-banner -c "$cfg" || return 1
  done
  return 0
}
run_gate "gate:8  gitleaks (secrets)" secrets_g

# ── 9. shell scripts (the hooks + bin) ───────────────────────────────────────
# -S info catches word-splitting (SC2086) etc.; SC1091 is the unavoidable
# "can't follow the dynamically-sourced lib" false positive — excluded, not the
# severity floor raised past it.
shellcheck_g() { need shellcheck "brew install shellcheck" || return 1; shellcheck -S info -e SC1091 .claude/hooks/*.sh bin/*.sh; }
run_gate "gate:9  shellcheck" shellcheck_g

# ── 10. no inline suppressions (CONSTITUTION §0/§15; aic NoBaselinesGate) ─────
# ANY inline allow/expect (clippy OR rustc lint) in game source must carry a
# real `// <text>` justification. Both `allow(` and `expect(` are covered (the
# modern `#[expect]` is an equally-powerful suppression). Blanket group
# suppressions are banned outright — with or without a comment.
no_suppr_g() {
  local unjust blanket
  unjust=$(grep -rnE '#!?\[(allow|expect)\(' crates/*/src src-tauri/src 2>/dev/null \
           | grep -vE '//[[:space:]]*[^[:space:]]' || true)
  blanket=$(grep -rnE '#!?\[(allow|expect)\((clippy::(all|correctness|suspicious|complexity|perf|style|pedantic|nursery|restriction)|warnings|unused)\b' crates/*/src src-tauri/src 2>/dev/null || true)
  if [ -n "$unjust$blanket" ]; then
    echo "unjustified / blanket suppressions in game source (CONSTITUTION §0/§15):"
    [ -n "$unjust" ]  && { echo "— missing a // justification (allow/expect):"; echo "$unjust"; }
    [ -n "$blanket" ] && { echo "— blanket group suppression (banned outright):"; echo "$blanket"; }
    return 1
  fi
  return 0
}
run_gate "gate:10 no-suppressions" no_suppr_g

# ── 11. source bans (SAST; = aic semgrep anti-pattern rules) ─────────────────
# Primitives that have no place in game logic: process spawning, hard exits,
# transmute, and unsafe without a // SAFETY: justification. Maps semgrep's
# no-exec / no-unsafe rules to the Rust source.
source_bans_g() {
  local prims unsafes
  prims=$(grep -rnE 'Command::new|std::process::(Command|exit|abort)|mem::transmute' crates/*/src src-tauri/src 2>/dev/null || true)
  unsafes=$(grep -rnE '(^|[^_[:alnum:]])unsafe[[:space:]]' crates/*/src src-tauri/src 2>/dev/null | grep -v 'SAFETY:' || true)
  if [ -n "$prims$unsafes" ]; then
    echo "banned source primitives (CONSTITUTION §14):"
    [ -n "$prims" ]   && { echo "— process/transmute primitives:"; echo "$prims"; }
    [ -n "$unsafes" ] && { echo "— unsafe without a // SAFETY: justification:"; echo "$unsafes"; }
    return 1
  fi
  return 0
}
run_gate "gate:11 source-bans (SAST)" source_bans_g

# ── 12. clippy allow-list baseline (NoBaselinesGate, config side) ────────────
# The [workspace.lints.clippy] allow-list is a "justified ratchet backlog". This
# gate pins it to bin/.clippy-allowlist so it cannot silently grow into a
# de-facto baseline: any add/remove must update the baseline file in the same
# commit (visible + reviewable).
allowlist_g() {
  local base="bin/.clippy-allowlist" cur
  [ -f "$base" ] || { echo "missing allow-list baseline $base"; return 1; }
  cur=$(sed -n '/^\[workspace\.lints\.clippy\]/,/^\[/p' Cargo.toml \
        | grep '"allow"' \
        | sed -E 's/[[:space:]]*=.*//' \
        | grep -vE '^\[|^[[:space:]]*$' \
        | LC_ALL=C sort -u)
  if ! diff <(printf '%s\n' "$cur") <(grep -vE '^#|^[[:space:]]*$' "$base" | LC_ALL=C sort -u) >/dev/null 2>&1; then
    echo "clippy allow-list differs from $base (NoBaselinesGate). diff (< live / > baseline):"
    diff <(printf '%s\n' "$cur") <(grep -vE '^#|^[[:space:]]*$' "$base" | LC_ALL=C sort -u) || true
    echo "If intentional, update $base in the SAME commit (CONSTITUTION §0)."
    return 1
  fi
  return 0
}
run_gate "gate:12 lints-allowlist" allowlist_g

# ── 13. doc TODOs ────────────────────────────────────────────────────────────
# No TODO/FIXME/XXX in committed design docs (planning/ is working scratch).
doc_todos_g() {
  local hits
  hits=$(grep -rlE 'TODO|FIXME|XXX' --include='*.md' docs/ 2>/dev/null | grep -v 'docs/planning/' || true)
  [ -z "$hits" ] || { echo "TODO/FIXME/XXX markers in committed docs:"; echo "$hits"; return 1; }
  return 0
}
run_gate "gate:13 doc-todos" doc_todos_g

# ── 14. tauri shell — the standalone src-tauri crate (NOT a workspace member) ─
# src-tauri is its own workspace root (own Cargo.lock), so gates 1-3's
# --workspace/--all never reach it. fmt-check runs in BOTH modes (cheap, no
# compile). clippy compiles the tauri dep tree, so it is FULL-only to keep
# --fast painless. Default lints + -D warnings (the §14 baseline); the shell
# crate does not inherit the workspace pedantic/nursery lints by design (#4).
tauri_shell_g() {
  local m="src-tauri/Cargo.toml"
  [ -f "$m" ] || { echo "missing $m"; return 1; }
  cargo fmt --manifest-path "$m" --check || return 1
  [ "$MODE" = "full" ] || return 0
  cargo clippy --manifest-path "$m" --all-targets -- -D warnings
}
run_gate "gate:14 tauri shell (fmt; +clippy full)" tauri_shell_g

# ── 18. studio CSS no-drift (Tailwind v4 + DaisyUI compiled output, ticket #66) ─
# `crates/oathstar-studio/static/studio.css` is GENERATED from `studio.tw.css` by
# the Tailwind CLI (DaisyUI plugin) scanning the studio's render.rs class literals,
# then committed + `include_str!`'d. A forgotten rebuild would ship stale styling,
# so a fresh build must byte-match the committed CSS. Built to a temp file and
# compared — never mutates the worktree. Deterministic because tailwindcss /
# @tailwindcss/cli / daisyui are pinned exact in package.json. Runs in BOTH modes
# (cheap); the bin comes from `npm install`, guarded like the cargo tools.
studio_css_g() {
  local bin="node_modules/.bin/tailwindcss"
  local src="crates/oathstar-studio/static/studio.tw.css"
  local out="crates/oathstar-studio/static/studio.css"
  [ -x "$bin" ] || { echo "MISSING: $bin — run 'npm install' (pins tailwindcss + daisyui)"; return 1; }
  local tmp; tmp=$(mktemp) || { echo "mktemp failed"; return 1; }
  if ! "$bin" -i "$src" -o "$tmp" --minify >/dev/null 2>&1; then
    echo "tailwind build failed for $src"; rm -f "$tmp"; return 1
  fi
  if ! cmp -s "$tmp" "$out"; then
    echo "studio.css is STALE — run 'npm run studio:css' and commit the result (gate:18)."
    rm -f "$tmp"; return 1
  fi
  rm -f "$tmp"
}
run_gate "gate:18 studio-css (no-drift)" studio_css_g

# ── FULL-only: coverage + mutation ───────────────────────────────────────────
# FAST is a legitimate green-able quick loop, but it prints "GATE GREEN [fast]";
# the enforce-commit-gate.sh hook requires a worktree-bound receipt that is only
# written on a FULL green, so /commit can never be satisfied by a FAST run.
if [ "$MODE" = "fast" ]; then
  RESULTS+=("SKIP  gate:15-17 coverage+mutation (--fast) — run the FULL gate before /commit")
else
  # 15. rust line coverage floor
  rust_cov() {
    need cargo-llvm-cov "cargo install cargo-llvm-cov" || return 1
    cargo llvm-cov --workspace --fail-under-lines "$RUST_COV_MIN"
  }
  run_gate "gate:15 rust coverage (>= ${RUST_COV_MIN}% lines)" rust_cov

  # 16. js line coverage floor
  js_cov() {
    local out pct
    shopt -s nullglob; set -- tests/*.test.js; shopt -u nullglob
    [ "$#" -gt 0 ] || { echo "no JS test files"; return 1; }
    out=$(node --test --experimental-test-coverage "$@" 2>&1) || return 1
    pct=$(echo "$out" | grep 'all files' | grep -oE '[0-9]+\.[0-9]+|[0-9]+' | head -1)
    [ -n "$pct" ] || { echo "could not parse JS coverage"; return 1; }
    awk -v p="$pct" -v min="$JS_COV_MIN" 'BEGIN{exit !(p+0 >= min+0)}' \
      || { echo "JS line coverage ${pct}% < floor ${JS_COV_MIN}%"; return 1; }
    echo "JS line coverage ${pct}% >= ${JS_COV_MIN}%"
  }
  run_gate "gate:16 js coverage (>= ${JS_COV_MIN}% lines)" js_cov

  # 17. mutation testing (MSI floor) — the Infection equivalent
  # Hardened against the stale-outcomes false green: the previous run's output
  # is removed first, the run's exit code is checked (a crash/build-fail is a
  # gate failure, not a vacuous green off a leftover file), and the result must
  # be a fresh file with viable mutants.
  mutation_g() {
    need cargo-mutants "cargo install cargo-mutants" || return 1
    need jq "brew install jq" || return 1
    rm -rf mutants.out
    local rc
    cargo mutants --workspace >/dev/null 2>&1; rc=$?
    # cargo-mutants exit: 0 = no survivors, 2 = survivors found — BOTH are valid
    # COMPLETED runs we threshold below. Anything else (1 usage error, 3 no
    # mutants, 4 baseline build/test failed, 101 crash, signal) is NOT a valid
    # measurement — fail closed rather than read a partial/garbage outcomes.json.
    case "$rc" in
      0|2) : ;;
      *) echo "cargo mutants did not complete a valid run (exit $rc)"; return 1 ;;
    esac
    [ -f mutants.out/outcomes.json ] || { echo "no mutants.out/outcomes.json produced"; return 1; }
    local caught missed total msi
    caught=$(jq '[.outcomes[]|select(.summary=="CaughtMutant")]|length' mutants.out/outcomes.json)
    missed=$(jq '[.outcomes[]|select(.summary=="MissedMutant" or .summary=="Timeout")]|length' mutants.out/outcomes.json)
    total=$((caught + missed))
    [ "$total" -gt 0 ] || { echo "no viable mutants produced — mutation did not exercise any code"; return 1; }
    msi=$(awk -v c="$caught" -v t="$total" 'BEGIN{printf "%.1f", 100*c/t}')
    echo "mutation: ${caught} caught / ${missed} missed → MSI ${msi}% (floor ${MUT_MSI_MIN}%)"
    awk -v m="$msi" -v min="$MUT_MSI_MIN" 'BEGIN{exit !(m+0 >= min+0)}' \
      || { echo "MSI ${msi}% < floor ${MUT_MSI_MIN}% — kill more mutants (write tests)"; return 1; }
  }
  run_gate "gate:17 mutation (MSI >= ${MUT_MSI_MIN}%)" mutation_g
fi

# ── Summary ──────────────────────────────────────────────────────────────────
printf '\n\033[1m══ gate summary (%s) ══\033[0m\n' "$MODE"
for r in "${RESULTS[@]}"; do
  case "$r" in
    PASS*) printf '  \033[32m%s\033[0m\n' "$r" ;;
    FAIL*) printf '  \033[31m%s\033[0m\n' "$r" ;;
    *)     printf '  %s\n' "$r" ;;
  esac
done
printf '  %d passed, %d failed\n' "$PASS" "$FAIL"

[ "$FAIL" -eq 0 ] || { echo "GATE RED — fix at source (CONSTITUTION §0: no baselines, no suppressions)."; exit 1; }
echo "GATE GREEN [$MODE]"

# Receipt — bind this FULL green to the exact worktree it ran on. The
# enforce-commit-gate.sh hook reads it back and blocks `git commit` unless the
# fingerprint still matches, so the verdict cannot be forged in prose and a
# stale green (code edited after) cannot be reused.
if [ "$MODE" = "full" ]; then
  GITDIR=$(git rev-parse --git-dir 2>/dev/null || true)
  if [ -n "$GITDIR" ]; then gate_state_hash > "$GITDIR/oathstar-gate-receipt" 2>/dev/null || true; fi
fi
