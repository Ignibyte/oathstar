#!/usr/bin/env bash
# enforce-mcp-config.sh — forge sidecar must be configured (PreToolUse).
# Blocks app work in a pipeline session when .mcp.json lacks the `forge` server.
# CONSTITUTION §19. Exit 0 = allow, 2 = block.
set -euo pipefail
INPUT=$(cat)
command -v jq &>/dev/null || exit 0
source "$(dirname "$0")/lib-hook-helpers.sh"

TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty')
[ -n "$TRANSCRIPT_PATH" ] && [ -f "$TRANSCRIPT_PATH" ] || exit 0
# Only enforce once we're clearly in pipeline/forge work.
is_pipeline_session "$TRANSCRIPT_PATH" || exit 0

if [ ! -f "${PROJECT_ROOT}/.mcp.json" ] || ! jq -e '.mcpServers.forge // empty' "${PROJECT_ROOT}/.mcp.json" >/dev/null 2>&1; then
    { echo ""; echo "BLOCKED — the forge sidecar is not configured in .mcp.json."
      echo "CONSTITUTION §19: the pipeline recalls/captures knowledge via the forge MCP server."
      echo "Add it (see .mcp.json.example) and ensure the sidecar is up:"
      echo "  ../oathstar-forge/scripts/start-all.sh"; } >&2
    exit 2
fi
exit 0
