---
pipeline_id: a4a944ee-1210-4dc2-b278-30790bf08ada
title: WORK-command-parser-v1
ticket: 6651420b-9cd1-418c-9f66-98ff825c6650
type: work
intake:
notes: WORK-command-parser-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-command-parser-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** First Rust command-parser layer — a pure `parse(input) -> Command`
  over a typed `Command` enum, wired into `Engine::handle_command`, so MUD-style
  text input flows through one typed engine path (reusable by future UI / Datastar
  / DM actions).
- **Scope:**
  - **In:** a parser module/API in `oathstar-core` (pure, deterministic); a typed
    `Command` enum (the engine boundary); refactor `Engine::handle_command` to
    `parse → match typed Command → emit events`, preserving all current behavior
    (help, look, movement aliases, `go <dir>`, unknown, empty); add
    `look <target>` / `examine <target>` producing a typed inspect command with
    the **target text preserved**; whitespace/case normalization that does not
    alter meaningful target text; a typed unknown-command path that mutates no
    state; unit tests covering every parse branch (94% line cov + 100% MSI).
  - **Out (v1):** natural-language / LLM intent parsing; combat commands beyond a
    placeholder; the richer Decision-002 prepositional grammar
    (`verb target on target`, `verb target to target`, `ask target about topic`)
    and the social/inventory/oath gameplay handlers — v1 parses the EARS shapes
    + a typed fallback; those verbs are future tickets. No protocol/event schema
    change unless Design shows REQ-004 needs a dedicated event variant.
- **Systems:** parser + engine (`crates/oathstar-core`); protocol
  (`crates/oathstar-protocol`) only if Design adds an unknown-command event variant.

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player enters a movement alias (`n/s/e/w/u/d`, a full direction, or `go <dir>`), the parser shall produce the matching typed movement command. | Rust unit test per alias |
| REQ-002 | When the player enters `look <target>` or `examine <target>`, the parser shall produce a typed inspect command with the target text preserved. | Rust unit test |
| REQ-003 | When the player enters extra whitespace or mixed case, the parser shall normalize the command (verb match + collapsed whitespace) without changing the meaningful target text. | Rust unit test (`  LOOK   Warden ` → inspect "Warden") |
| REQ-004 | If input cannot be parsed into a known command, then the engine shall return a typed unknown-command event without mutating game state. | Rust unit test (state unchanged + typed event) |
| REQ-005 | The full gate shall pass green, including ≥94% line coverage and 100% mutation MSI over the new parser code. | `bin/gate.sh` → GREEN [full] |

## Locked-In Decisions
- **Decision 002 (Locked) governs the grammar:** forgiving *symbolic* parser, no
  NL/AI. v1 implements the subset the EARS require; the full prepositional grammar
  is a later ticket.
- **The parser is a pure, deterministic function** (`input → Command`), separate
  from the engine's effectful handling. This is what makes it unit-testable to
  100% MSI and lets every front-end share one engine path (the ticket's "why").
- **Target text is preserved; only the verb is case/whitespace-normalized.** This
  is intentionally stricter than the JS reference (`normalizeInput` lowercases the
  whole string) — REQ-002/003 require the target's meaningful text to survive.
- **No behavior regressions:** every currently-passing `handle_command` test
  (help / look / movement / `go` / unknown / empty) must still pass after the
  refactor.
- **Land in `oathstar-core`** as the typed `Command` boundary; touch
  `oathstar-protocol` only if Design proves REQ-004 needs a dedicated
  `UnknownCommand` event (default: reuse the existing typed `LogMessage`/system path).

## Linked Artifacts
- Design docs: `docs/decisions.md#decision-002-use-a-forgiving-symbolic-parser`, `docs/mechanics-and-systems.md` (Player Input), `docs/event-lifecycle.md` (Command Lifecycle), `docs/protocol-and-output.md`, `docs/technical-architecture.md`
- Reference impl (to port): `src/engine.js` — `parseCommand` / `tokenize` / `normalizeInput`
- Intake doc: none (ticket pre-existed)
- Ticket doc: `docs/planning/tickets/open/TICKET-5-design-command-parser-v1.md`
- Forge ticket: `6651420b-9cd1-418c-9f66-98ff825c6650` (#5)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
