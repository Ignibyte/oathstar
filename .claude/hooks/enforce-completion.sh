#!/usr/bin/env bash
# enforce-completion.sh — capture knowledge before completing (Stop hook).
# At /pipeline:complete the session must have recorded at least one piece of
# knowledge to the forge (aar-submit / failure-record / prevention-rule-record
# / architecture-decision-record). The sidecar only learns if you feed it.
# CONSTITUTION §18.3. Exit 0 = allow, 2 = block.
set -euo pipefail
INPUT=$(cat)
command -v jq &>/dev/null || exit 0
source "$(dirname "$0")/lib-hook-helpers.sh"
TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty')
[ -n "$TRANSCRIPT_PATH" ] && [ -f "$TRANSCRIPT_PATH" ] || exit 0
is_pipeline_session "$TRANSCRIPT_PATH" || exit 0
[ "$(detect_active_command "$TRANSCRIPT_PATH")" = "complete" ] || exit 0

MCP=$(extract_mcp_tool_names "$TRANSCRIPT_PATH")
if ! echo "$MCP" | grep -qE '^(aar-submit|failure-record|prevention-rule-record|architecture-decision-record)$'; then
    { echo ""; echo "STOP BLOCKED — /pipeline:complete recorded no knowledge to the forge."
      echo "CONSTITUTION §18.3: capture what this pipeline taught you before closing it —"
      echo "  aar-submit                    (lessons from this run)"
      echo "  failure-record                (anything that bit you)"
      echo "  prevention-rule-record / architecture-decision-record  (a durable rule / decision)"; } >&2
    exit 2
fi
exit 0
