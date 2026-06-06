#!/usr/bin/env bash
# =============================================================================
# enforce-phase-gate.sh — Pipeline phase ordering (PreToolUse: Write|Edit)
# =============================================================================
# Blocks Write/Edit to application code unless the active pipeline doc shows the
# previous phase as PASS (+ human-confirmed for plan/design). CONSTITUTION §3.
#
# Activates only when: a pipeline phase command is active AND a pipeline doc
# exists AND the target is application code. Exit 0 = allow, 2 = block.
# =============================================================================
set -euo pipefail
INPUT=$(cat)
command -v jq &>/dev/null || exit 0
source "$(dirname "$0")/lib-hook-helpers.sh"

FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
[ -n "$FILE_PATH" ] || exit 0
NORMALIZED=$(normalize_path "$FILE_PATH")

# Only application code is gated. Docs, .claude/, planning, config are exempt.
case "$NORMALIZED" in
    docs/*|.claude/*|bin/*|*.md|*.json|*.toml|*.yaml|*.yml|*.lock|*.env*|*.html|*.css|*.svg) exit 0 ;;
esac
case "$NORMALIZED" in
    crates/*.rs|src/*.js|src/*.mjs|src-tauri/src/*.rs|tests/*) : ;;  # gated
    *) exit 0 ;;
esac

TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty')
[ -n "$TRANSCRIPT_PATH" ] && [ -f "$TRANSCRIPT_PATH" ] || exit 0
is_pipeline_session "$TRANSCRIPT_PATH" || exit 0

COMMAND_NAME=$(detect_active_command "$TRANSCRIPT_PATH")
[ -n "$COMMAND_NAME" ] || exit 0

PIPELINE_DIR="docs/planning/pipeline/active"

# plan: no prerequisite phase, but never allow two active pipelines.
if [ "$COMMAND_NAME" = "plan" ]; then
    n=0
    for d in "$PIPELINE_DIR"/*.spec.md; do [ -f "$d" ] && n=$((n+1)); done
    if [ "$n" -gt 1 ]; then
        { echo ""; echo "PHASE GATE BLOCKED — $n active pipelines. CONSTITUTION §3: never two active at once."
          echo "Archive one: mv docs/planning/pipeline/active/<file>.{spec,notes}.md docs/planning/pipeline/completed/"; } >&2
        exit 2
    fi
    exit 0
fi

PIPELINE_DOC=$(get_active_pipeline_doc)
if [ -z "$PIPELINE_DOC" ]; then
    { echo ""; echo "PHASE GATE BLOCKED — no active pipeline doc. Run /pipeline:plan first."; } >&2
    exit 2
fi

# Real UUID required (not a template placeholder).
# BSD-grep compatible (macOS): POSIX ERE, no -P/\s.
if ! grep -qE '^pipeline_id:[[:space:]]*[0-9a-f-]{36}' "$PIPELINE_DOC" 2>/dev/null; then
    { echo ""; echo "PHASE GATE BLOCKED — pipeline doc has no real pipeline_id UUID."
      echo "Set frontmatter: pipeline_id: \$(python3 -c 'import uuid;print(uuid.uuid4())')"; echo "Doc: $PIPELINE_DOC"; } >&2
    exit 2
fi

# Phase N is PASS when status frontmatter says "Phase N <…> PASS".
# BSD-grep compatible: sed extracts the value (no \K); [[:space:]] not \s; the
# dash is dropped from the pattern (it's a multibyte em-dash) — "Phase N " then
# any chars then PASS is enough, and "Phase 3 " can't match "Phase 3.5".
phase_pass() {
    local status_line escaped
    status_line=$(sed -n 's/^status:[[:space:]]*//p' "$1" 2>/dev/null | head -1 || true)
    # Escape regex metachars in the phase number (e.g. "3.5" — the dot must be
    # literal, else it matches "3X5"). bash 3.2: ${var//./\\.} replaces dots.
    escaped=${2//./\\.}
    echo "$status_line" | grep -qE "Phase[[:space:]]+${escaped}[[:space:]].*PASS"
}

GATE_FAILED=false; MSG=""
case "$COMMAND_NAME" in
    design)    phase_pass "$PIPELINE_DOC" 1   || { GATE_FAILED=true; MSG="Phase 1 (Plan) must be PASS + human-confirmed before design."; } ;;
    implement) phase_pass "$PIPELINE_DOC" 2   || { GATE_FAILED=true; MSG="Phase 2 (Design) must be PASS + human-confirmed before implementation."; } ;;
    inspect)   phase_pass "$PIPELINE_DOC" 3   || { GATE_FAILED=true; MSG="Phase 3 (Implement) must be PASS before inspection."; } ;;
    validate)  phase_pass "$PIPELINE_DOC" 3.5 || { GATE_FAILED=true; MSG="Phase 3.5 (Inspect) must be PASS before validation. CONSTITUTION §18.1 — the inspect checkpoint is mandatory."; } ;;
    complete)  phase_pass "$PIPELINE_DOC" 4   || { GATE_FAILED=true; MSG="Phase 4 (Validate) must be PASS before completion."; } ;;
esac

if [ "$GATE_FAILED" = true ]; then
    { echo ""; echo "PHASE GATE BLOCKED — $MSG"; echo ""
      echo "CONSTITUTION §3: every phase has an entry gate; the previous phase must be PASS."
      echo "Doc: $PIPELINE_DOC   Active: /pipeline:$COMMAND_NAME"; } >&2
    exit 2
fi
exit 0
