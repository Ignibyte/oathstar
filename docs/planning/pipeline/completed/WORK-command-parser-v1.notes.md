# WORK-command-parser-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #5 — implement the first Rust command-parser layer for
  MUD-style input (movement aliases, look/examine + target, normalize, typed
  unknown). Auto-approve: drive the full pipeline autonomously through commit.
- **Intake source:** none — forge ticket #5 + local ticket doc pre-existed.
- **Classification / tier:** work pipeline, `feature` — a single shippable slice
  (one parser module + typed enum + `handle_command` refactor). Real Rust code +
  tests; the coverage/mutation floors are the hard part.
- **Forge recall:** Decision 002 (Locked) "forgiving symbolic parser"; design
  docs `mechanics-and-systems.md` (Player Input), `event-lifecycle.md` (Command
  Lifecycle). Gate-area prevention rules from ticket #4 still apply. (Deeper
  recall via `knowledge-context` at Design entry now the AAR is open.)
- **Current-state findings:**
  - `Engine::handle_command` (`crates/oathstar-core/src/lib.rs:229`) does inline
    string-matching: trims, **lowercases the whole input**, matches `help` /
    `look|l` / direction aliases / `go <dir>` prefix / else unknown. No typed
    `Command`, no target support, and lowercasing the whole string would destroy
    target case — so REQ-002/003 are genuinely new behavior.
  - Unknown + empty already mutate no state and emit a `LogMessage` system event
    (REQ-004 partly satisfied; v1 routes it through the typed path).
  - `direction_alias()` (`:432`) maps short→long; becomes a typed `Direction`.
  - Protocol (`oathstar-protocol/src/lib.rs`): `CommandRequest{input, actor_id}`
    → `CommandResponse{accepted, events, snapshot}`; `GameEventKind` =
    `LogMessage{component,text}` / `Tick` / `RoomEntered`. No `UnknownCommand`
    variant today.
  - JS reference (`src/engine.js:681` `parseCommand`): tokenize→normalize (trim,
    collapse `\s+`, **lowercase all**, strip non-word), then direction / `go` /
    `{verb,target}` with articles filtered. The Rust port must NOT lowercase the
    target (REQ-003) — a deliberate improvement over the JS reference.
- **Ticket:** forge #5 `6651420b-9cd1-418c-9f66-98ff825c6650` (pre-existing,
  documented at `docs/planning/tickets/open/TICKET-5-...md`).
- **EARS reviewed:** REQ-001..004 carried from the ticket doc; added REQ-005
  (gate green incl. coverage+mutation floors) to make "shippable" observable.
- **Open questions for Design:**
  - Exact `Command` enum shape (proposed: `Empty`, `Help`, `Look{target:
    Option<String>}`, `Move(Direction)`, `Unknown{input}`) — keep it minimal so
    100% MSI stays achievable; every variant must be reachable + tested.
  - REQ-004 "typed unknown-command event": reuse `LogMessage` (no protocol churn)
    vs add `GameEventKind::UnknownCommand{input}`. Default = reuse; justify if changed.
  - Target normalization rule: collapse whitespace + case-fold the verb, preserve
    target case. Decide whether to also article-strip the target (JS does).
- **AAR id:** `fc8076c7-715a-4570-b221-253267156503` (inspect→failure-record, complete→aar-submit capture into it)

## Phase 2 — Design

### Approach / architecture
A new pure parser module + two typed enums in `oathstar-core`, with
`Engine::handle_command` refactored to `parse → match → emit`.

**Types (`command.rs`, all `pub`, doc-commented):**
```rust
pub enum Direction { North, South, East, West, Up, Down }   // Copy, Eq
impl Direction {
    fn from_token(tok: &str) -> Option<Self>   // "n"|"north"→North, … else None
    const fn as_str(self) -> &'static str      // North→"north" (exits-map key)
}
pub enum Command {                              // Clone, Eq
    Empty,
    Help,
    Look { target: Option<String> },            // look|l|examine|x [target]
    Move(Direction),
    Unknown { input: String },                  // collapsed echo of the raw input
}
pub fn parse(input: &str) -> Command;
```

**`parse` algorithm (pure, deterministic — no RNG/IO):**
1. `let mut tokens = input.split_whitespace();` (this trims + collapses runs of
   whitespace — REQ-003 — in one step).
2. `let Some(first) = tokens.next() else { return Command::Empty };` — empty/all-
   whitespace input → `Empty`. (No `unwrap`; the else arm is reachable + covered.)
3. `let verb = first.to_lowercase();` (case-fold the VERB only — REQ-003);
   `let rest: Vec<&str> = tokens.collect();` (target tokens, **original case**).
4. `Direction::from_token(&verb)` → `Move(dir)` (REQ-001 bare aliases).
5. `verb == "go"`: if `rest.first()` lowercased is a direction → `Move(dir)`, else
   `Unknown` (typed Direction means `go banana`/`go` can't be a Move).
6. `verb == "help" | "h"` → `Help`.
7. `verb ∈ {look,l,examine,x}` → `Look { target: rest.is_empty() ? None :
   Some(rest.join(" ")) }` (REQ-002; target case preserved, whitespace collapsed).
8. else → `Unknown { input: collapse(input) }` where
   `collapse = input.split_whitespace().join(" ")` (case-preserving echo).

**`handle_command` refactor:** `match parse(&request.input)` →
- `Empty` → "The world waits…", `accepted=false` (unchanged).
- `Help` → directions hint, `accepted=true`.
- `Look{None}` → `describe_current_room()` (unchanged), `accepted=true`.
- `Look{Some(t)}` → placeholder narrative echoing `t` (no entity resolution in
  core yet — out of scope), `accepted=true`. The PARSER satisfies REQ-002; the
  handler keeps the target visible end-to-end.
- `Move(dir)` → `move_direction(dir)` (now takes `Direction`, uses `as_str()`).
- `Unknown{input}` → "I do not know how to '{input}' yet.", `accepted=false`,
  no mutation (REQ-004).
`move_direction` signature `&str → Direction`; the free `direction_alias` fn is
deleted (replaced by `Direction::from_token`). No `oathstar-protocol` change.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/command.rs` | **NEW** — `Direction` + `Command` enums, `from_token`/`as_str`, `collapse`, `pub fn parse`; `#[cfg(test)]` parser tests covering every branch. Doc-comment public items. |
| 2 | `crates/oathstar-core/src/lib.rs` | `pub mod command;` + `use command::{parse, Command, Direction};`; refactor `handle_command` to parse→match; `move_direction(Direction)`; delete `direction_alias`; add handler tests asserting `accepted` per path + `look <target>` echo. Keep all existing tests green. |
| 3 | notes (this file) | Phase 3/4 records. |
- **No protocol change**, no JS change (the JS prototype keeps its own parser).

### Regression Test Plan
"Pure parser + thin handler", tested to **94% line + 100% MSI**. Parser tests live
in `command.rs`; engine-wiring/`accepted` tests in `lib.rs`. ≥1 row per AC; the
rest exist to kill specific mutants (noted).

| # | Test | Proves / kills |
|---|---|---|
| P1 | table: every direction token (`north,n,south,s,east,e,west,w,up,u,down,d`) + `N`,`NORTH` → `Move(dir)` | REQ-001; kills `from_token` arms + verb `to_lowercase` |
| P2 | `go east`→Move(E); `go N`→Move(N); `go`→Unknown; `go banana`→Unknown | REQ-001; kills `go` branch, `rest.first`, go-Unknown fallback |
| P3 | `look warden`→Look{Some("warden")}; `examine the gate`→Look{Some("the gate")}; `look`/`l`/`examine`/`x`→variants; bare `look`→Look{None} | REQ-002; kills look arms + target Some/None |
| P4 | `"  LOOK   Warden "`→Look{Some("Warden")}; `"ExAmInE  Black  Lantern"`→Look{Some("Black Lantern")} | REQ-003; kills verb `to_lowercase` + `rest.join` (collapse, case-preserve) |
| P5 | `""`/`"   "`→Empty; `xyzzy`→Unknown{"xyzzy"}; `"  foo  bar "`→Unknown{"foo bar"} | REQ-004(parser)+empty; kills `else{Empty}` + unknown fallback + `collapse` |
| P6 | `help`/`h`/`HELP`→Help | kills help arm + lowercase |
| P7 | `Direction::as_str` each variant → canonical str; `from_token("zzz")`→None | kills `as_str` arms + from_token default |
| H1 | engine `east` → room `b`, `accepted==true` | REQ-001 e2e + `accepted` true-mutant |
| H2 | engine `look warden` → `accepted==true` + an event text contains `warden` | REQ-002 e2e (target preserved through engine) |
| H3 | engine `xyzzy` → `accepted==false`, room unchanged, event "I do not know how to 'xyzzy'" | REQ-004 e2e + `accepted` false-mutant + no-mutation |
| H4 | engine `help` → `accepted==true` + lists directions | handler help arm + accepted |
| H5 | engine `look` → `accepted==true` + RoomHeader "A [test]" | handler look-None arm (regression) |
| H6 | engine `"   "` → `accepted==false` + "world waits" | empty arm (regression) |
| H7 | engine `go east` → room `b`, `accepted==true` | go e2e (regression) |
| H8 | existing: move-no-exit refused; move-into-impassable blocked | movement regressions stay green |
- **Genuinely-uncoverable:** only the pre-existing `current_room().expect(...)`
  invariant (ticket #2, already excluded from the line floor). No new uncoverable
  paths — `parse` has no `unwrap`/unreachable arm.

### Risks / decisions
- **R1 (typed Direction):** `go banana`/`go` → `Unknown` (can't be a Move). Minor
  refinement over the current stringly "cannot go that way"; no existing test
  covers it; cleaner + typed.
- **R2 (unknown→accepted=false):** was `true` in current code; now `false` to
  align with `Empty` and better serve REQ-004. **Verify** `oathstar-server`'s
  `command_processes_and_broadcasts` test sends a *known* command (so its
  `accepted==true` assert is unaffected) — check at Implement.
- **R3 (REQ-004 event repr):** reuse the existing typed `LogMessage`/System event
  (no `oathstar-protocol` churn, no new mutation surface). "Typed event" satisfied;
  a dedicated `UnknownCommand` variant is a possible future refinement.
- **R4 (no article-stripping in v1):** the JS reference strips `the/a/an`; v1
  preserves literal target text (REQ-003). Article/target resolution is deferred
  to the future inspect-gameplay ticket.
- **R5 (look<target> handler is a placeholder):** no entity system in core yet, so
  `look <target>` echoes the target rather than resolving it. The typed
  `Command::Look{target}` fully satisfies REQ-002 at the parser boundary.
- **R6 (100% MSI tactics):** assert `accepted` on every `handle_command` arm and
  exhaust parser branches (tables) — every mutant has a killing test above.
- **§14:** `parse` is pure/deterministic, no `unwrap`/`expect` on the input path;
  public items doc-commented; `Direction`/`Command` derive `PartialEq, Eq` for
  `assert_eq!` tests.

## Phase 3 — Implement
- **Built (2 files, production code only — full test suite is Phase 4):**
  - `crates/oathstar-core/src/command.rs` (NEW) — `Direction` enum
    (`from_token` private, `as_str` pub const), `Command` enum (`Empty`/`Help`/
    `Look{target}`/`Move`/`Unknown{input}`), pure `pub fn parse`, private
    `collapse`. All public items doc-commented. Matches the design exactly.
  - `crates/oathstar-core/src/lib.rs` — `pub mod command;` +
    `use command::{parse, Command, Direction};`; `handle_command` refactored to
    `match parse(&request.input)` over the 6 arms (Empty/Help/Look-None/Look-Some/
    Move/Unknown); `Unknown` and `Empty` return `accepted=false` with no mutation;
    `move_direction(Direction)` via `as_str()`; the free `direction_alias` fn
    deleted. Existing tests untouched.
- **In-phase checks (green):**
  - `cargo check -p oathstar-core` — clean.
  - `cargo clippy -p oathstar-core --all-targets --all-features -- -D warnings`
    — **clean** under the strict workspace lints (pedantic+nursery+restriction);
    the `if rest.is_empty() {None} else {Some(..)}` pattern passed without
    complaint.
  - `cargo test -p oathstar-core` — **20 passed; 0 failed** (all pre-existing
    tests still green; the refactor preserved behavior).
  - **R2 verified:** `oathstar-server`'s `command_processes_and_broadcasts` sends
    `input: "look"` (a known command) → its `accepted==true` assert is unaffected
    by the new `Unknown → accepted=false` behavior.
- **Deviations from design:** none. Test suite (parser branch tables + handler
  `accepted` assertions, P1–P7 / H1–H8) is deferred to Phase 4 per the pipeline
  (implement = code; validate = tests).

## Inspect (Phase 3.5)
- **Lenses run:** 3 parallel `general-purpose` critics, each verifying concretely
  (ran cargo test/clippy/mutants --list, probed `parse`): (1) parser correctness +
  behavior-preservation vs the old inline matcher, (2) mutation / test-plan
  completeness for 100% MSI, (3) simplification / idiom / §14. Verdicts:
  PARSER-CORRECT · TEST-PLAN-COMPLETE (0 must-add) · CLEAN.
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | LOW | `go <dir> <trailing>` (e.g. `go east now`) moved east — only `rest.first()` was checked — while `go banana`/`go to north`→Unknown; an undocumented asymmetry that silently ignores trailing tokens (`command.rs` go branch). | **REAL** | **Fixed** — `go` now requires exactly one direction token (`if let [token] = rest.as_slice()`); `go`/`go banana`/`go east now` all → `Unknown`. clippy clean, 20 tests still green. |
  | 2 | LOW (advisory) | Plan's MSI rationale leaned on `accepted` bool-flip + per-arm mutants that cargo-mutants does NOT generate here (no `Default` impl on `Direction`/`Command`/`CommandResponse`/`GameEvent` → those replacements are compiler-unviable). | NOTED | Plan still reaches 100% MSI (real killers = `from_token`/`as_str` arms, `parse` `==`/`||`, `collapse`, `move_direction` `vec![]`/`!passable`). Keep `accepted` asserts as behavior guards; **carry to Validate.** |
  | 3 | LOW (advisory) | P7 must assert a concrete `as_str` literal per variant (`assert_eq!(Direction::North.as_str(),"north")`), not only a `from_token`/`as_str` round-trip. | NOTED | **Carry to Validate.** |
  | 4 | NIT | `go` re-lowercases its direction token; `look go`/`examine north` capture direction words as targets. | REJECTED | Both correct per Decision 002 grammar (target is literal text); harmless. |
  | 5 | LOW | `collapse` re-runs `split_whitespace`; `rest: Vec` eagerly allocated. | REJECTED | Critic 3 concurred: leave for clarity — turn-based game, one parse per command; inlining hurts readability and the named helper preserves case correctly. |
- **Verified-clean (critics ran these):** `parse` never panics (empty / whitespace
  / tabs / newlines / no-target / multi-space target / 200 KB / non-ASCII / emoji);
  deterministic (no RNG/IO/global state); §14 — no `unwrap`/`expect` on the input
  path (`let-else`), all `pub` items doc-commented; `cargo clippy … -D warnings`
  green under strict workspace lints; 20/20 core tests pass; `as_str()` returns the
  exact lowercase keys used in room `exits`; every behavior change vs the old
  matcher is intended (REQ-002 look-target, R1 `go<bad>`→Unknown, R2
  unknown→accepted=false) — no accidental regressions.
- **Carry to Validate:** add `go east now`→`Unknown` (new strict-go behavior) to the
  P2 row; ensure P7 asserts concrete `as_str` literals; keep the per-arm `accepted`
  assertions as behavior guards.
- **Capture:** `failure-record` BF-parser-go-trailing-tokens-001 (low) — the
  go-branch trailing-token edge case caught + fixed in inspect.

## Phase 4 — Validate
- **Tests added (16 new, all from the Phase 2 plan + inspect carry-forwards):**
  - `command.rs` `#[cfg(test)]` (11 fns, P1–P7): empty/whitespace→Empty; every
    direction word+letter alias→Move (12-token table); case-insensitive verbs;
    `go <one dir>`→Move; **`go`/`go banana`/`go east now`→Unknown** (strict-go
    carry-forward); look/examine/x with target (text preserved); bare look
    aliases→None; normalize (verb case-folded, target case+spacing preserved);
    help/h/HELP→Help; unknown collapsed echo; **`as_str` concrete literal per
    direction** (carry-forward).
  - `lib.rs` (5 fns, H1–H5): `accepted` asserted on move/look-target/unknown/
    help/look arms; `look <target>` echoes the preserved target end-to-end; H6
    (empty=!accepted), H7 (`go east`), H8 (move-no-exit / impassable) already
    existed.
- **`cargo test --workspace`:** all pass — **oathstar-core 36 passed; 0 failed**
  (was 20; +16), other crates 5/9/… all green.
- **`node --test tests/*.test.js`:** **4 pass; 0 fail**.
- **`bin/gate.sh` (FULL): `GATE GREEN [full]` — 17 passed, 0 failed.**
  - gate:17 mutation **53 caught / 0 missed → MSI 100.0%** (8 new viable parser
    mutants, all killed — the design's branch tables + the `as_str`/`accepted`
    asserts did their job).
  - gate:15 rust coverage **96.06% line ≥ 94** (rose from 94.09%; the parser is
    ~fully covered).
  - gate:14 tauri shell PASS; gate:1-13/16 PASS; clippy strict clean.
- **Pre-existing exclusions:** `fn main` (mutation exclusion, `.cargo/mutants.toml`);
  `oathstar-server/src/main.rs` lower per-file coverage — both pre-existing, not in
  scope; the workspace line floor still passes.
- **All AC verified:** REQ-001 ✓ REQ-002 ✓ REQ-003 ✓ REQ-004 ✓ REQ-005 ✓.

## Phase 5 — Complete
- **Docs updated:** `docs/mechanics-and-systems.md` Command Parser section — added a
  "Status — v1 implemented" note (`oathstar-core::command`, the v1 subset).
  `docs/technical-architecture.md:144` already lists oathstar-core's `parser`
  responsibility (no change). `decisions.md` Decision 002 left as the locked vision.
- **Forge capture:**
  - `aar-submit` **fc8076c7** — completed, effectiveness 5, 4 novel findings
    (distillation / confidence-drift / pattern-emergence jobs enqueued).
  - `failure-record` **BF-parser-go-trailing-tokens-001** (d294d121) — recorded at Inspect.
  - `architecture-decision-record` **AD-claude-command-parser-001** (3b9dc30c) —
    typed `Command` / pure-`parse` boundary in oathstar-core; go-strict; reuse LogMessage.
  - `prevention-rule-record` **PR-claude-parser-grammar-arity-001** (6d74c8c9) —
    strict verb arity (prevents BF-parser-go-trailing-tokens-001).
  - `prevention-rule-record` **PR-claude-mutation-arm-coverage-001** (464d9c69) —
    per-arm/binop tests + no-`Default` insight for 100% MSI.
- **Ticket closed:** forge #5 (6651420b) → `done`; local doc moved open/→closed/.
- **Archived:** pipeline doc pair moved active/→completed/.

## Pre-merge review fix — bare-direction arity
- **Finding (review):** `parse` returned `Command::Move` as soon as the FIRST token
  was a direction, ignoring trailing tokens — so `north now` / `n guard` parsed to
  `Move(...)` and could mutate state on malformed input. Inconsistent with
  `go <dir>` (already strict via the inspect fix). Same class as
  BF-parser-go-trailing-tokens-001, but for bare directions — a gap the inspect
  pass missed (it only tightened the `go` form).
- **Fix:** the bare-direction branch now requires `rest.is_empty()`; with trailing
  tokens it returns `Command::Unknown { input: collapse(input) }` — the same strict
  arity as `go <dir>` (`command.rs` bare-direction block).
- **Tests added (38 core total):** parser `bare_direction_with_trailing_tokens_is_unknown`
  (`north now`/`n guard` → `Unknown{input}`); engine `malformed_bare_direction_does_not_move`
  (`east now` → `accepted == false`, room unchanged even though `east` has a real
  exit — proves no state mutation).
- **Gate:** FULL `bin/gate.sh` GREEN 17/17 — Rust coverage ≥94% line, JS ≥75%,
  mutation MSI 100%. Scope limited to the parser arity fix (no other behavior changed).
