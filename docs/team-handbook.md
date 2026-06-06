# Oathstar Team Handbook

This document is the shared operating brief for humans, Codex, Claude Code, and
future agents working on Oathstar.

## Project North Star

Oathstar is a MUD-inspired roleplaying game and, over time, a modular
roleplaying runtime.

The first playable target is a strong single-player authored experience: a
Beginner module with a local town, a simple oath, exploration, combat/grinding,
a tower objective, and a boss endpoint.

The long-term target is broader: a choose-your-own-adventure and roleplaying
engine where worlds, rule systems, directors, and UI surfaces can be swapped or
composed for solo play, friend-group sessions, DM-led campaigns, and eventually
LLM-assisted storytelling.

## Product Principles

- Single-player first, but do not close the door on multiplayer, DM control, or
  swappable clients.
- Text and language are core to the fantasy, even when the UI becomes richer.
- Oaths are quests++: promises with mechanical, social, and narrative weight.
- The game should support both a focused RPG run and deep opt-in simulation.
- Complexity should be discoverable, not dumped on a new player all at once.
- The Beginner module must be finishable and satisfying on its own.

## Architecture Principles

- Rust server/runtime is the authority for rules, state, saves, and events.
- Tauri is a player shell and server manager, not the backend authority.
- Clients send commands and render results; they do not own game rules.
- Domain events are typed first, then renderable as JSON, HTML fragments, or
  plain text.
- REST plus SSE is the first transport; WebSockets are later if justified.
- Content starts TOML/file-first with Rust validation and future editor tooling.
- The core validates and commits state changes. Modules submit requests; they do
  not mutate arbitrary state directly.
- Build one official ruleset first, while keeping combat, stats, progression,
  inventory, director behavior, and worlds from becoming permanent kernel
  assumptions.

## Team Roles

- User: product owner and creative director. Locks game direction, taste, scope,
  and priorities.
- Codex: architect, reviewer, documentation steward, and implementation manager.
  Codex decides technical shape, decomposition, algorithms, ticket ordering, and
  review criteria.
- Claude Code/coder agent: implementation worker. The coder follows the active
  ticket/spec, writes code and tests, runs gates, and reports completion.
- Forge sidecar: project memory, tickets, AARs, prevention rules, docs search,
  and code search.

Codex should review agent output before the next implementation step. The review
checks the ticket, EARS requirements, architecture docs, tests, and gate output.

## Work Flow

1. Idea capture: rough ideas go to `docs/planning/intake/`.
2. Backlog: ready work gets a Forge ticket and a matching local ticket document
   under `docs/planning/tickets/open/`.
3. Active work: one ticket is promoted into `docs/planning/pipeline/active/`
   with a `.spec.md` and `.notes.md` pair.
4. Plan: acceptance criteria are written in EARS.
5. Design: the implementation approach and regression test plan are recorded.
6. Implement: the coder agent writes the scoped change.
7. Inspect: Codex or the required inspect phase reviews the diff
   adversarially.
8. Validate: tests and `bin/gate.sh` run green.
9. Complete: docs are updated, knowledge is captured to Forge, the ticket is
   closed, and pipeline docs move to completed.

Only one pipeline is active at a time.

## Current Build Priorities

Recommended order:

1. `#2` Harden core world initialization.
2. `#3` Validate save slot names.
3. `#5` Design command parser v1.
4. `#6` Model rooms, regions, entities, and items v1.
5. `#7` Build beginner module vertical slice in Rust.
6. `#4` Add Tauri shell quality gate before meaningful Tauri behavior lands.

Strategic intake:

- `INTAKE-fully-swappable-rulesets`: long-term rule-system modularity.

## Definition Of Ready

A ticket is ready for implementation when it has:

- A Forge ticket id.
- A local ticket document.
- EARS requirements with verification methods.
- Scope that can be completed in one focused pipeline.
- Links to relevant design docs.
- Known out-of-scope items.

## Definition Of Done

Implementation is done when:

- The EARS requirements are satisfied.
- Tests were added or updated for the behavior.
- `bin/gate.sh` passes before commit.
- Docs changed by the behavior are updated.
- Codex review finds no blocking issues.
- Forge knowledge capture and ticket closure happen during pipeline completion.

## Hard Guardrails

- Do not copy ROT/ROM/Diku source code. Use it only as design reference after
  license review.
- Do not make LLM output authoritative for deterministic state.
- Do not let modules bypass core validation.
- Do not hardcode Beginner-module assumptions into the core engine.
- Do not make Tauri the backend authority.
- Do not add broad abstractions before a real ticket needs them.
- Do not lower gate floors or create baselines to make a change pass.

## Documentation Map

- `docs/game-overview.md`: game promise and player fantasy.
- `docs/decisions.md`: locked decisions and technical direction.
- `docs/technical-architecture.md`: crate/runtime/server/client shape.
- `docs/module-system.md`: module presets, hooks, rule-system modularity.
- `docs/event-lifecycle.md`: ticks, scheduled events, time sequences, DM/LLM
  boundaries.
- `docs/protocol-and-output.md`: typed events, JSON/HTML/plain text output.
- `docs/vertical-slice.md`: Beginner module proof of concept.
- `docs/planning/README.md`: intake, ticket, and pipeline workflow.
