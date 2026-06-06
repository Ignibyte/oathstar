#!/usr/bin/env bash
# enforce-phase-tasks.sh — TaskCreate-first convention (Stop hook).
# Each pipeline phase must (a) call TaskCreate at least once after entering the
# phase, and (b) resolve every task it created (completed|deleted) before Stop.
# Blocks Stop otherwise. Exit 0 = allow, 2 = block.
set -euo pipefail
INPUT=$(cat)
command -v jq &>/dev/null || exit 0
source "$(dirname "$0")/lib-hook-helpers.sh"
TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty')
[ -n "$TRANSCRIPT_PATH" ] && [ -f "$TRANSCRIPT_PATH" ] || exit 0
is_pipeline_session "$TRANSCRIPT_PATH" || exit 0

# Only enforce inside an actual pipeline phase (not /commit, /work, chat).
PHASE=$(detect_active_command "$TRANSCRIPT_PATH")
[ -n "$PHASE" ] || exit 0

IDX=$(index_of_latest_phase_advance "$TRANSCRIPT_PATH")
[ -n "$IDX" ] || exit 0   # no phase advance recorded; nothing to scope

CREATED=$(count_tool_uses_after_index "$TRANSCRIPT_PATH" "$IDX" "TaskCreate")
RESOLVED=$(count_terminal_task_updates_after_index "$TRANSCRIPT_PATH" "$IDX")

if [ "$CREATED" = "0" ]; then
    { echo ""; echo "STOP BLOCKED — /pipeline:$PHASE created no tasks."
      echo "Convention: the FIRST action of a phase is one TaskCreate per checklist item."
      echo "Create the phase's tasks, work them, resolve them, then Stop."; } >&2
    exit 2
fi
if [ "$RESOLVED" -lt "$CREATED" ]; then
    { echo ""; echo "STOP BLOCKED — /pipeline:$PHASE has unresolved tasks ($RESOLVED/$CREATED terminal)."
      echo "Before leaving a phase, TaskUpdate every task to completed (done) or deleted (n/a)."
      echo "Call TaskList to see what's still open."; } >&2
    exit 2
fi
exit 0
