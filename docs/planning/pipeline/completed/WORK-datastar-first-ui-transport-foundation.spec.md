---
pipeline_id: 1849059e-2762-4e1b-bbd0-c7ab1c291913
title: WORK-datastar-first-ui-transport-foundation
ticket: 25b0bddd-a96c-470b-8565-6f4c59e86130
type: work
intake:
notes: WORK-datastar-first-ui-transport-foundation.notes.md
status: Phase 1 — Plan PASS; Phase 2 — Design PASS; Phase 3 — Implement PASS; Phase 3.5 — Inspect PASS; Phase 4 — Validate PASS; Phase 5 — Complete PASS
---

# WORK-datastar-first-ui-transport-foundation

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Datastar-first UI transport foundation — one narrow vertical slice.
- **Scope:**
  - **In:** vendor/pin the real Datastar runtime through the project build;
    add a dedicated Rust Datastar **presentation boundary** outside
    `oathstar-core` (a crate — `oathstar-datastar` — preferred per Decision 034;
    a module only if Design shows it's clearly smaller/better) and lift the
    existing inline `render_event_html` rendering into it; convert **exactly one**
    narrow read-only player-client surface (event feed is the leading candidate)
    to Datastar-format SSE HTML; keep map/state JSON endpoints intact; add
    server-side HTML-escaping tests; update the architecture/UI/protocol docs with
    the actual route + module names per Decision 033.
  - **Out:** full Host Manager UI; Tauri sidecar/server lifecycle; canvas/map
    renderer (Decision 035 is referenced, not built); DaisyUI; React/Vue/Svelte;
    converting every panel / full UI rewrite; gameplay mechanics. The rest of the
    client may remain hand-rolled during this slice (ticket note).
- **Systems:** ui (player client), server (HTTP/SSE transport + new presentation
  boundary), frontend build/vendoring (npm/vite), docs. **Not** `oathstar-core`
  (it must stay Datastar-agnostic).

## Acceptance Criteria (EARS)
Each acceptance criterion uses EARS syntax, describes one observable behavior,
and includes a verification method. IDs mirror the forge ticket for traceability.

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player client is built and loaded, the Datastar runtime shall be vendored/pinned through the project build (a fixed-version npm/vite dependency) and present at runtime, rather than fetched ad hoc from a CDN at play time. | `npm run build` output + browser smoke (runtime present) |
| REQ-002 | When first-party UI fragments are rendered, the Datastar HTML/SSE rendering shall live in a dedicated Rust presentation boundary outside `oathstar-core`, and `oathstar-core` shall not depend on that boundary. | cargo dependency check + Rust tests + code review |
| REQ-003 | When the one selected surface updates, that surface shall be delivered as Datastar-format SSE HTML (data-\* driven) rather than hand-written DOM mutation. | integration test (SSE event format) + browser smoke |
| REQ-004 | When map/state renderer data is requested at `GET /state` (and `GET /events/json`), the response shall remain JSON (`GameSnapshot` / `MapSnapshot`) and shall not be replaced by HTML fragments. | Rust test asserting JSON shape + API smoke |
| REQ-005 | When server-rendered Datastar HTML embeds server-provided text, that text shall be HTML-escaped so injected markup (`<`, `>`, `&`, `"`, `'`) is rendered inert and cannot inject elements. | Rust unit tests on the presentation boundary |
| REQ-006 | When the player submits a command, resolution shall remain server-authoritative via `POST /command`, and no game rule or authoritative state shall move into client JavaScript. | code review + browser smoke |
| REQ-007 | When developing browser-first, the Player Client shall still run with `npm run server:dev` and `npm run dev`. | browser smoke |
| REQ-008 | When the architecture/UI/protocol docs are updated, they shall name the actual route(s) and module(s) used by this slice and reflect Decision 033's Player Client / Host Manager / Rust Game Server split. | docs review |
| REQ-009 | When the slice is complete, `npm test`, `npm run build`, and `./bin/gate.sh --fast` shall all pass. | command output |

## Locked-In Decisions
- **Datastar/SSE HTML is the first-party UI default; JSON is reserved for
  canvas/map/sprite renderer data, diagnostics, tests, adapters** (Decision 034).
  This slice does not move that line.
- **The Datastar presentation lives behind a Rust boundary outside
  `oathstar-core`; `oathstar-core` must not depend on Datastar** (Decision 034).
  A crate (`oathstar-datastar`) is preferred; a module is acceptable only if
  Design shows it is clearly smaller/better. Design decides crate-vs-module —
  *not* whether the boundary exists.
- **The Rust server stays the authority; the command path stays `POST /command`;
  no rules/authoritative state move into client JS** (Decisions 015 / 032 / 033).
- **Exactly ONE narrow read-only surface is converted this slice** (event feed is
  the leading candidate; final pick in Design). The remainder of the client may
  stay hand-rolled.
- **Map payloads stay renderer-agnostic JSON** in `GET /state` (`GameSnapshot.map`,
  `MapSnapshot`); the canvas renderer itself is out (Decision 035).
- **No DaisyUI for the player client** (Decision 034 styling direction).

## Linked Artifacts
- Design docs: `docs/technical-architecture.md`, `docs/ui-design.md`,
  `docs/protocol-and-output.md`, `docs/map-system.md`,
  `docs/decisions.md` (Decisions 033 / 034 / 035; extends 032).
- Intake doc: none.
- Ticket doc: `docs/planning/tickets/open/TICKET-15-datastar-first-ui-transport-foundation.md`
- Forge ticket: `25b0bddd-a96c-470b-8565-6f4c59e86130` (#15)
- Forge AAR: `b0a6f623-e0b4-43c6-b108-17642507117e`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS (design + test plan in notes) |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS (4 critics; 2 LOW hardened, rest acceptable) |
| 4 — Validate | PASS (gate --fast GREEN; 17 JS + 122 Rust tests) |
| 5 — Complete | PASS |
