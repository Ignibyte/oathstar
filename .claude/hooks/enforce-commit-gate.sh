#!/usr/bin/env bash
# =============================================================================
# enforce-commit-gate.sh — no committing code without a green gate (PreToolUse: Bash)
# =============================================================================
# CONSTITUTION §0/§15: bin/gate.sh is the truth gate. This hook makes it binding
# — a `git commit` that includes Rust/JS source is BLOCKED unless bin/gate.sh
# left a RECEIPT (.git/oathstar-gate-receipt) proving a FULL green ran on the
# EXACT current worktree.
#
# The receipt is a CONTENT FINGERPRINT (gate_state_hash), not a transcript
# string. That closes the holes the old string-match had:
#   - can't be forged by printing/echoing "GATE GREEN [full]" or by reading a
#     file that contains the literal (no receipt is written by either);
#   - any post-green edit by ANY tool (Write, Edit, or a Bash `cat >`/`sed -i`)
#     changes the fingerprint, so a stale green is rejected;
#   - a FAST run writes no receipt, so `--fast` can never satisfy /commit;
#   - lowering a floor can't help — the gate clamps floors to the §0 minimums
#     before it will print green and write the receipt.
#
# Always-on (not gated on a pipeline session). Exit 0 = allow, 2 = block.
# Bash 3.2 + BSD-grep safe.
# =============================================================================
set -euo pipefail
INPUT=$(cat)
command -v jq >/dev/null 2>&1 || exit 0
source "$(dirname "$0")/lib-hook-helpers.sh"

CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty')
[ -n "$CMD" ] || exit 0

# Detect a real `git ... commit` in any common spelling: `git commit`,
# `git -C . commit`, `git -c k=v commit`, `env X=1 git commit`, `(git commit)`,
# `eval "git commit"`, `sh -c "git commit"`. (A literal `git` token, then a
# `commit` token, allowing intervening flags/args but not a command separator.)
echo "$CMD" | grep -qE '(^|[^[:alnum:]_.-])git[[:space:]]+([^;&|]*[[:space:]])?commit([[:space:]]|$)' || exit 0
# Allow a STANDALONE dry-run/help (writes no commit) — but NOT when chained with
# another command (which could smuggle a real commit past the skip).
if echo "$CMD" | grep -qE -- '(--dry-run|--help|(^|[[:space:]])-h([[:space:]]|$))'; then
    echo "$CMD" | grep -qE '[;&|]' || exit 0
fi

cd "$PROJECT_ROOT" 2>/dev/null || exit 0
# Only gate when Rust/JS SOURCE is in the change set (staged or unstaged). The
# optional trailing quote matches git's C-quoted paths (names with spaces/tabs).
CODE_CHANGED=$(git status --porcelain 2>/dev/null | grep -E '\.(rs|js|mjs)"?$' || true)
[ -n "$CODE_CHANGED" ] || exit 0

# The gate's receipt must exist AND still match the current worktree fingerprint.
GITDIR=$(git rev-parse --git-dir 2>/dev/null || true)
RECEIPT="${GITDIR:-.git}/oathstar-gate-receipt"
if [ -f "$RECEIPT" ] && [ "$(cat "$RECEIPT" 2>/dev/null)" = "$(gate_state_hash)" ]; then
    exit 0
fi

{ echo ""
  echo "COMMIT BLOCKED — no green FULL gate for the current worktree."
  echo "CONSTITUTION §0: run  bin/gate.sh  (it must print GATE GREEN [full]) AFTER your"
  echo "last code change, then commit. Fix every red at the source — no baselines, no"
  echo "suppressions, no lowering a floor."
  if [ -f "$RECEIPT" ]; then
    echo "(a receipt exists but its fingerprint no longer matches — code changed since the"
    echo " gate ran; re-run bin/gate.sh.)"
  fi
} >&2
exit 2
