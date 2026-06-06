#!/usr/bin/env bash
# enforce-quality.sh — formatting gate on changed code (Stop hook).
# At validate/complete, if Rust source changed, `cargo fmt --all --check` must
# be clean. The heavier clippy/test/coverage gates live in bin/gate.sh (run at
# /commit) and enforce-tests-ran.sh; this hook is the cheap always-on guard.
# CONSTITUTION §0. Exit 0 = allow, 2 = block.
set -uo pipefail
INPUT=$(cat)
command -v jq &>/dev/null || exit 0
source "$(dirname "$0")/lib-hook-helpers.sh"
TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty')
[ -n "$TRANSCRIPT_PATH" ] && [ -f "$TRANSCRIPT_PATH" ] || exit 0
is_pipeline_session "$TRANSCRIPT_PATH" || exit 0

CMD=$(detect_active_command "$TRANSCRIPT_PATH")
case "$CMD" in validate|complete) : ;; *) exit 0 ;; esac

# Tool absent != gate failed — don't false-block when cargo isn't on the hook PATH.
command -v cargo >/dev/null 2>&1 || exit 0

# Any Rust source touched (tracked changes or untracked new files)?
cd "$PROJECT_ROOT" || exit 0
CHANGED=$(git status --porcelain 2>/dev/null | grep -E '\.rs$' || true)
[ -n "$CHANGED" ] || exit 0

if ! cargo fmt --all --check >/dev/null 2>&1; then
    { echo ""; echo "STOP BLOCKED — Rust formatting is not clean (gate:1)."
      echo "CONSTITUTION §0: no baselines, source-fix only. Run:  cargo fmt --all"
      echo "Then re-run the gate:  bin/gate.sh"; } >&2
    exit 2
fi
exit 0
