#!/usr/bin/env bash
# enforce-pipeline-completion.sh — pipeline doc sanity (Stop hook).
# When a pipeline phase has run this session, the active pipeline doc must have
# a real status line that names the phase just worked — i.e. status was actually
# advanced, not left on a template placeholder. CONSTITUTION §3.
# Exit 0 = allow, 2 = block.
set -euo pipefail
INPUT=$(cat)
command -v jq &>/dev/null || exit 0
source "$(dirname "$0")/lib-hook-helpers.sh"
TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty')
[ -n "$TRANSCRIPT_PATH" ] && [ -f "$TRANSCRIPT_PATH" ] || exit 0
is_pipeline_session "$TRANSCRIPT_PATH" || exit 0

PHASE=$(detect_active_command "$TRANSCRIPT_PATH")
[ -n "$PHASE" ] || exit 0

DOC=$(get_active_pipeline_doc)
[ -n "$DOC" ] || exit 0   # plan may not have created it yet; don't block

STATUS=$(sed -n 's/^status:[[:space:]]*//p' "$DOC" 2>/dev/null | head -1 || true)
# Placeholder markers as WHOLE WORDS (so a legit status mentioning "TODO" in
# prose isn't false-blocked), plus an angle-bracket placeholder, plus the
# template's un-advanced default "IN PROGRESS" (a doc never moved past Plan).
if echo "$STATUS" | grep -qiE '(^|[^[:alnum:]])(TEMPLATE|PLACEHOLDER|TODO)([^[:alnum:]]|$)|<[^>]+>|IN PROGRESS'; then
    { echo ""; echo "STOP BLOCKED — pipeline doc status is still a placeholder / not advanced."
      echo "Advance it to reflect the phase just worked, e.g.:"
      echo "  status: Phase 3 — Implement PASS; ready for Phase 3.5 — Inspect"
      echo "Doc: $DOC   (current: ${STATUS:-<none>})"; } >&2
    exit 2
fi
exit 0
