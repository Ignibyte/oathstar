#!/usr/bin/env bash
# enforce-docs-before-code.sh — recall before you write (PreToolUse: Write|Edit).
# Before the first application-code write in a pipeline session, the session
# must have recalled prior knowledge from the forge (knowledge-context /
# knowledge-search / docs-search). CONSTITUTION §18.3. Exit 0 = allow, 2 = block.
set -euo pipefail
INPUT=$(cat)
command -v jq &>/dev/null || exit 0
source "$(dirname "$0")/lib-hook-helpers.sh"

FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
[ -n "$FILE_PATH" ] || exit 0
NORMALIZED=$(normalize_path "$FILE_PATH")
case "$NORMALIZED" in
    crates/*.rs|src/*.js|src/*.mjs|src-tauri/src/*.rs) : ;;   # only gate real code
    *) exit 0 ;;
esac

TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty')
[ -n "$TRANSCRIPT_PATH" ] && [ -f "$TRANSCRIPT_PATH" ] || exit 0
is_pipeline_session "$TRANSCRIPT_PATH" || exit 0

# Only enforce during implement (code phase).
[ "$(detect_active_command "$TRANSCRIPT_PATH")" = "implement" ] || exit 0

MCP=$(extract_mcp_tool_names "$TRANSCRIPT_PATH")
if ! echo "$MCP" | grep -qE '^(knowledge-context|knowledge-search|knowledge-explain|knowledge-by-pattern|knowledge-related|docs-search|code-find|code-callers|code-callees)$'; then
    { echo ""; echo "BLOCKED — recall before you write (CONSTITUTION §18.3)."
      echo "Call the forge first so you build on what's known:"
      echo "  knowledge-context / knowledge-search  (prior lessons, failures, prevention rules)"
      echo "  docs-search                           (the design docs)"
      echo "  code-find / code-callers              (where it lives, who calls it)"
      echo "Then write $NORMALIZED."; } >&2
    exit 2
fi
exit 0
