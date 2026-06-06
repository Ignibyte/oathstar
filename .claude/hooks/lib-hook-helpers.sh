#!/usr/bin/env bash
# =============================================================================
# lib-hook-helpers.sh — Shared helpers for Oathstar enforcement hooks
# =============================================================================
#
# Source from other hooks: source "$(dirname "$0")/lib-hook-helpers.sh"
#
# Shared transcript-reader + phase-detection helpers. Bash 3.2 compatible
# (macOS /bin/bash) — no mapfile/readarray/declare -A/${x,,}. Transcript
# readers use the canonical `jq -s '[.[] | (.message // .) | ...]'` slurp
# pattern — Claude Code writes
# JSONL where each line is `{ "type": ..., "message": { "role", "content" } }`;
# `(.message // .)` unwraps that and falls back to legacy top-level shape.
# Never use bare `jq '.[]'` (silently errors per-line and the gate goes dormant).
# =============================================================================

# Git root so hooks work regardless of CWD; fall back to pwd.
PROJECT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || pwd)

# Strip the project-root prefix (and any worktree prefix) for pattern matching.
normalize_path() {
    echo "$1" \
        | sed "s|^${PROJECT_ROOT}/||" \
        | sed 's|^\.claude/worktrees/[A-Za-z0-9._-]\+/||'
}

# --- Gate receipt fingerprint ------------------------------------------------
# Content fingerprint of all gated source (CONSTITUTION §3 application code):
# HEAD + the worktree content of every tracked-or-untracked .rs/.js/.mjs file
# under crates/ src/ src-tauri/src/ tests/. bin/gate.sh writes this to
# .git/oathstar-gate-receipt on a FULL green; enforce-commit-gate.sh recomputes
# it at `git commit` and allows the commit only if they match — so a green binds
# to the EXACT worktree it ran on. This is why the verdict can't be forged by
# printing/echoing "GATE GREEN" or by reading a file that contains it (no
# receipt is written), and why any post-green edit by ANY tool (Write, Edit, or
# a Bash heredoc) invalidates it (the content hash changes). Hashing the union
# of tracked+untracked file CONTENT (not `git diff`) makes it invariant to
# `git add` staging, so the normal gate→add→commit flow stays green.
# Bash 3.2 + BSD-safe (no \K/\s/-P; shasum, not sha256sum).
gate_state_hash() {
    local root="${PROJECT_ROOT:-$(pwd)}" f
    {
        git -C "$root" rev-parse HEAD 2>/dev/null || printf 'no-head\n'
        {
            git -C "$root" ls-files -- crates src src-tauri/src tests 2>/dev/null
            git -C "$root" ls-files --others --exclude-standard -- crates src src-tauri/src tests 2>/dev/null
        } | LC_ALL=C sort -u | while IFS= read -r f; do
            case "$f" in
                *.rs|*.js|*.mjs) [ -f "$root/$f" ] && shasum "$root/$f" 2>/dev/null ;;
            esac
        done
    } | shasum 2>/dev/null | awk '{print $1}'
}

# --- Transcript readers ------------------------------------------------------

extract_bash_commands() {
    local t="$1"; [ -f "$t" ] || return 0
    jq -sr '[.[] | (.message // .) | select(.role=="assistant") |
            (.content // [])[] | select(.type=="tool_use") |
            select(.name=="Bash") | .input.command // empty] | .[]' "$t" 2>/dev/null || true
}

extract_user_texts() {
    local t="$1"; [ -f "$t" ] || return 0
    jq -sr '[.[] | (.message // .) | select(.role=="user") | (.content // []) |
            if type=="array" then (.[] | select(.type=="text") | .text // empty)
            elif type=="string" then . else empty end] | .[]' "$t" 2>/dev/null || true
}

# MCP tool names called by the assistant, `mcp__SERVER__` prefix stripped.
extract_mcp_tool_names() {
    local t="$1"; [ -f "$t" ] || return 0
    jq -sr '[.[] | (.message // .) | select(.role=="assistant") |
            (.content // [])[] | select(.type=="tool_use") | .name // empty |
            select(startswith("mcp__")) | sub("^mcp__.*__";"")] | .[]' "$t" 2>/dev/null || true
}

# Every assistant tool_use name (Bash, Edit, Write, TaskCreate, MCP-stripped).
extract_assistant_tool_uses() {
    local t="$1"; [ -f "$t" ] || return 0
    jq -sr '[.[] | (.message // .) | select(.role=="assistant") |
            (.content // [])[] | select(.type=="tool_use") | .name // empty |
            sub("^mcp__.*__";"")] | .[]' "$t" 2>/dev/null || true
}

# The Oathstar pipeline command alphabet (used by the two detectors below).
# Phases: plan design implement inspect validate complete. Plus work/commit.
_PIPE_RE='(pipeline[-:](plan|design|implement|inspect|validate|complete)|commit|work)'

# Most-recent slash command of ANY kind (returns bare name or "").
latest_pipeline_command() {
    local t="$1"; [ -f "$t" ] || { echo ""; return; }
    local m
    m=$(jq -sr --arg re "^\\s*/$_PIPE_RE\\b" '
        [.[] | (.message // .) |
         if .role=="user" then ((.content // []) |
             if type=="array" then (.[] | select(.type=="text") | .text // empty)
             elif type=="string" then . else empty end)
         elif .role=="assistant" then ((.content // [])[]? |
             select(.type=="tool_use" and .name=="Skill") | "/" + (.input.skill // ""))
         else empty end] | map(select(test($re))) | last // empty' "$t" 2>/dev/null || true)
    [ -n "$m" ] && echo "$m" | grep -oE "/$_PIPE_RE" | tail -1 | sed 's#^/##' || true
}

# Bare PIPELINE PHASE name (plan|design|implement|inspect|validate|complete),
# or "" when the latest slash command is NOT a pipeline phase (e.g. /commit).
detect_active_command() {
    local t="$1"; [ -f "$t" ] || { echo ""; return; }
    local m
    m=$(jq -sr --arg re "^\\s*/?$_PIPE_RE\\b" '
        [.[] | (.message // .) |
         if .role=="user" then ((.content // []) |
             if type=="array" then (.[] | select(.type=="text") | .text // empty)
             elif type=="string" then . else empty end)
         elif .role=="assistant" then ((.content // [])[]? |
             select(.type=="tool_use" and .name=="Skill") | "/" + (.input.skill // ""))
         else empty end] | map(select(test($re))) | last // empty' "$t" 2>/dev/null || true)
    [ -n "$m" ] && echo "$m" | grep -oE 'pipeline[-:](plan|design|implement|inspect|validate|complete)' | tail -1 | sed 's/^pipeline[-:]//' || true
}

# 0-based JSONL line index of the last phase-advance (user /pipeline:<phase> or
# assistant Skill pipeline-<phase>). Used to scope per-phase TaskCreate counts.
index_of_latest_phase_advance() {
    local t="$1"; [ -f "$t" ] || { echo ""; return; }
    jq -sr '
        [.[] | (.message // .) | (
            if .role=="user" then ((.content // []) |
                if type=="array" then ([.[]? | select(.type=="text") | .text // empty] | join("\n"))
                elif type=="string" then . else "" end)
            elif .role=="assistant" then
                ([.content // [] | .[]? | select(.type=="tool_use" and .name=="Skill") |
                  "/" + (.input.skill // "")] | join("\n"))
            else "" end)] |
        to_entries | map(select(.value | test("(^|\\n)\\s*/?pipeline[-:](plan|design|implement|inspect|validate|complete)\\b"))) |
        last // empty | if . == "" then "" else (.key | tostring) end' "$t" 2>/dev/null || true
}

# Count assistant tool_use blocks with NAME on lines strictly after INDEX.
count_tool_uses_after_index() {
    local t="$1" from="${2:--1}" tool="$3"; [ -f "$t" ] || { echo 0; return; }
    [ -n "$from" ] || from=-1
    jq -sr --argjson from "$from" --arg tool "$tool" '
        [.[] | (.message // .) | (if .role=="assistant" then
            ([.content // [] | .[]? | select(.type=="tool_use" and .name==$tool)] | length)
         else 0 end)] | to_entries | map(select((.key|tonumber) > $from) | .value) | add // 0' "$t" 2>/dev/null || echo 0
}

# Count TaskUpdate(status=completed|deleted) on lines strictly after INDEX.
count_terminal_task_updates_after_index() {
    local t="$1" from="${2:--1}"; [ -f "$t" ] || { echo 0; return; }
    [ -n "$from" ] || from=-1
    jq -sr --argjson from "$from" '
        [.[] | (.message // .) | (if .role=="assistant" then
            ([.content // [] | .[]? | select(.type=="tool_use" and .name=="TaskUpdate") |
              select(.input.status=="completed" or .input.status=="deleted")] | length)
         else 0 end)] | to_entries | map(select((.key|tonumber) > $from) | .value) | add // 0' "$t" 2>/dev/null || echo 0
}

# First active pipeline .spec.md (or legacy .md), or "". Always returns 0 so
# `VAR=$(get_active_pipeline_doc)` under `set -e` never aborts on "no doc".
get_active_pipeline_doc() {
    local d="${PROJECT_ROOT}/docs/planning/pipeline/active"
    local f
    for f in "$d"/*.spec.md; do [ -f "$f" ] && { echo "$f"; return 0; }; done
    for f in "$d"/*.md; do
        case "$(basename "$f")" in *.notes.md|.gitkeep) continue;; esac
        [ -f "$f" ] && { echo "$f"; return 0; }
    done
    return 0
}

# Pipeline session = pipeline INTENT is present. Armed by ANY of:
#   - an active /pipeline phase in the transcript (detect_active_command), OR
#   - an active pipeline doc on disk, OR
#   - a forge MCP tool call.
# Keying on intent (not just forge usage) closes the "skip the forge → all
# gates stay dormant → ship ungated" hole. Per-hook phase scoping still keeps
# the gates inert outside their phase. Normal chat (no phase, no active doc, no
# forge call) leaves every gate dormant.
is_pipeline_session() {
    local t="$1"; [ -f "$t" ] || return 1
    [ -n "$(detect_active_command "$t")" ] && return 0
    [ -n "$(get_active_pipeline_doc)" ] && return 0
    local n
    n=$(extract_mcp_tool_names "$t" | grep -cE '^(knowledge-search|knowledge-context|knowledge-explain|knowledge-by-pattern|knowledge-related|docs-search|aar-open|aar-submit|failure-record|architecture-decision-record|prevention-rule-record|bulletin-list|bulletin-create|bulletin-dismiss|ticket-create|ticket-get|ticket-list|ticket-update|ticket-close|ticket-claim|ticket-next|ticket-comment|sprint-create|sprint-get|sprint-list|sprint-current|code-find|code-callers|code-callees)$') || n=0
    [ "$n" -gt 0 ]
}

# Was a specific (already-extracted, newline-list) tool called?
tool_was_called() { echo "$1" | grep -q "^${2}$"; }
