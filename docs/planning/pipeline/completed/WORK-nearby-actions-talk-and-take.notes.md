# WORK-nearby-actions-talk-and-take — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Implement ticket #18 — `talk <target>` / `take <target>` consuming the
  #17 spatial-awareness model. Reuse `resolve_target`/`perceive`; minimal carried
  inventory + additive pack snapshot; server-authoritative. No commit (Codex reviews).
- **Intake source:** none (ticket minted directly).
- **Classification / tier:** single-slice **work pipeline** (feature). Well-bounded by
  the ticket's scope-out list; one shippable slice, one doc.
- **Forge recall (lessons/failures surfaced):** no bulletins. `docs-search` surfaced the
  #17 "Locked-In Decisions" (room/cell placement = id lists; rooms own
  `entities`/`items` Vecs; talk/take deferred from #17 with the resolver built to be
  reused). `knowledge-search` ranked prevention-rules + an architecture decision in
  the spatial/snapshot domain — Phase 2's `knowledge-context` (needs the AAR id below)
  will pull their bodies into the budgeted bundle.
- **Ticket:** forge #18 `c78c03e0-067f-4876-ba6f-17b760b5d2ff`; local doc
  `docs/planning/tickets/open/TICKET-18-nearby-actions-v1-talk-and-take-commands.md`.
- **AAR id:** `275a6f78-c60c-4813-9413-363ded3bd6c0` (Phase 2+ recall via `knowledge-context`;
  `inspect`→`failure-record` and `complete`→`aar-submit` capture into it).
- **EARS requirements reviewed:** REQ-001..008 carried verbatim from the ticket into the
  spec; all are EARS-form (one observable behavior + verification). No rewrites needed.

### Discovery map (seams the implementer will touch)
| Concern | Location | Note |
|---|---|---|
| Command enum + `parse` | `crates/oathstar-core/src/command.rs:54-160` | Add `Talk`/`Take` variants; follow `Look { target }`. `Unknown` echo via `collapse`. |
| Command dispatch | `crates/oathstar-core/src/lib.rs:522-573` (`handle_command`) | Add match arms; `swear`/`confront` (`:684-802`) are the `(accepted, events)` precedent. |
| Resolve/describe template | `lib.rs:618-636` (`look_at`) | `talk_at`/`take_at` mirror this; gate on `is_interactable()`. |
| Engine state | `lib.rs:392-413` (`GameState`/`PlayerState`) | Add minimal carried-item state (ids). |
| World mutation | `lib.rs:40-58` (`RoomDefinition.items`), engine owns `world` | `take` removes id from room `items`. |
| Snapshot builders | `lib.rs:462-491` (`snapshot`), `:817-846` (`room_snapshot`) | Add pack mapping; `room.contents` auto-drops taken item via `perceive`. |
| Protocol DTOs | `crates/oathstar-protocol/src/lib.rs:21-97` | Add additive pack DTO; `EventChannel::Inventory` + `OutputComponent::ItemCard` already exist. |
| Client Pack render | `src/client/snapshot.js:99-101` (`toPack`), `src/client-app.js:436-447` | Panel already wired to `menu.pack`; `toPack` currently ignores the snapshot. |
| JS tests | `tests/client.test.js` (`toPack`/`toNearby`) | Existing `menu.pack.count === 0` stays valid (empty pack). |
| Content (manual play) | `modules/beginner/rooms.toml` + `world.toml` | `mara` IS placed in `candle_shop` (talk target exists). NO room has `items =` (every item is entity-owned), so **take has no manual target** today. |

### Open design questions for Phase 2 (NOT yet locked)
- **A. How `take` finds the item's room.** Recommend extending `Awareness` with the
  source `room_id` (keeps all spatial knowledge in the #17 module, no engine-side
  re-scan) vs. an engine scan of `world.rooms` for the id within interaction range.
- **B. Pack state shape/location.** Recommend `GameState.pack: Vec<String>` (ids) →
  `GameSnapshot.pack: Vec<PackItemSnapshot { id, name }>` (additive, skip-if-empty),
  matching the client's `snapshot.pack` access, vs. nesting under `PlayerSnapshot`.
- **C. Talk gating + response text.** Any interactable `Actor` vs. require the
  `conversable` role; reject fixtures/items ("can't hold a conversation with that").
  Exact narrative line(s).
- **D. Take rejection messages** per branch: too-far / not-an-item / unknown / hidden
  (hidden already returns `None` from `resolve_target`, so it reads as "nothing like that").
- **E. Event typing.** Typed `GameEventKind::ItemTaken { item_id }` on the `Inventory`
  channel (Decision 031 parity with `OathSworn`) vs. a `LogMessage`. Check whether the
  JS event feed needs a case for a new typed kind (REQ-008 must stay green). Talk →
  `Narrative` `LogMessage` like `look`.
- **F. `accepted` semantics.** Follow `swear`/`confront`: talk to a reachable actor →
  `accepted = true` (no move); too-far/unknown/non-actor → `false`. take success →
  `true` (state changed); any reject → `false`.
- **G. Parser details.** Aliases `talk`/`speak`, `take`/`get`, and the two-token verb
  `pick up`. Bare `talk`/`take` with no target → `Unknown` (strict arity, consistent
  with bare-verb handling) vs. a typed command with empty target the engine rejects.
- **H. Content for manual verification (optional / possible follow-up).** To exercise
  `take` by hand, the beginner module needs one ground-placed item (add an `items =`
  entry to a room, e.g. `hollowmere_square`). Rust tests are synthetic so this does
  not block the gate; decide in Design whether to add it here or file a content ticket.
- **I. Help text.** Optionally extend the `help` string (`lib.rs:534-540`) to list
  `talk`/`take`; keep the existing `"look, north"` / `swear`+`confront` substrings so
  `help_*` tests stay green.

## Phase 2 — Design

### Resolved design questions (A–I)
- **A — `take` locates the item's room via `Awareness.room_id` (NOT an engine re-scan).**
  Add `room_id` to `Awareness` (and `Candidate.room_id: &'w str`), populated from the
  room `perceived_candidates` already iterates. Keeps all spatial knowledge in the #17
  module (DRY with "no duplicate distance math"); additive + core-internal (never on the
  wire). Lets `take` remove the item from the *exact* placing room even when it's an
  adjacent interactable cell.
- **B — pack state: `GameState.pack: Vec<String>` (ids, pickup order) → additive
  `GameSnapshot.pack: Vec<PackItemSnapshot { id, name }>`** with
  `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. Names resolved at snapshot
  time from `world.items` (the registry entry survives — `take` removes only the room
  *placement*). Matches the client's `snapshot.pack` access + the `oath`/`contents`
  additive precedent.
- **C — talk gates on kind then proximity; honors the `conversable` role for flavor.**
  Order: unknown → "no one like that"; kind≠Actor → "can't talk to that"; visible-but-
  unreachable Actor → "too far"; interactable Actor → respond (conversable ⇒ greeting,
  else "nothing to say"). Any interactable Actor ⇒ `accepted=true`, no move.
- **D — take rejection messages** per branch: unknown/hidden → "nothing like that to
  take"; not-an-item → "can't carry that"; too-far → "too far away to reach". All
  `accepted=false`, state preserved.
- **E — take emits a `LogMessage` on the `Inventory` channel + `OutputComponent::ItemCard`
  (NO new typed event).** Verified `oathstar-datastar` has two *exhaustive* matches on
  `GameEventKind` (`describe` :118, `kind_type` :151) — a new `ItemTaken` variant would
  force cross-crate arms + tests for zero REQ benefit and added REQ-008 risk. `Inventory`
  channel (→ "Pack") and `ItemCard` (→ "Item", :206) already render. The additive `pack`
  snapshot is the authoritative state (Decision 031). Typed `ItemTaken` is a documented
  future option. Talk → `Narrative`/`NarrativeMessage` (mirrors `look`).
- **F — `accepted` follows `swear`/`confront`:** talk reachable Actor ⇒ true (no move);
  take success ⇒ true; every reject ⇒ false. `talk_at`/`take_at` return `(bool, Vec<GameEvent>)`.
- **G — parser:** aliases `talk`|`speak` and `take`|`get`, plus the two-token verb
  `pick up` (`verb=="pick"` && first rest token == "up", target = remainder). `Talk`/`Take`
  carry a **required** `target: String`; bare `talk`/`take`/`get`/`speak`/`pick`/`pick up`
  (no target) → `Unknown` (strict arity, consistent with the existing grammar). One-word
  `pickup` is NOT take. Verb case-folded; target case preserved (like `Look`).
- **H — YES, add ONE ground item to the start room** (`hollowmere_square`) so `take` is
  exercisable in the real module (otherwise every beginner item is entity-owned and `take`
  is invisible in manual play / `/verify`). Minimal + reversible: one `[[items]]` entry +
  one `items = [...]` line. `talk` already has a target (`mara` is placed in `candle_shop`).
- **I — extend the `help` string** to list `talk`/`take`, preserving the existing
  `"look, north"` and `swear`/`confront` substrings so `help_*` tests stay green.

### Approach / architecture
`talk`/`take` are sibling perception-actions to `look`: parse → typed `Command` → an engine
handler that calls `awareness::resolve_target` (interaction-gated) → typed events; `take`
additionally mutates engine-owned world placement + player pack. State/view stay separated
(Decision 028/031): engine owns `GameState.pack` (ids); the view maps it to the additive
`GameSnapshot.pack`. No RNG, no panics on input paths (defensive `get`/`get_mut`, no
`unwrap`/`expect` on resolved data). No changes to `oathstar-datastar`/`-server`/`-storage`.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/command.rs` | Add `Command::Talk { target: String }` + `Take { target: String }`; parse `talk`/`speak`, `take`/`get`, two-token `pick up`; bare/`pick`-only → `Unknown`. + unit tests. |
| 2 | `crates/oathstar-core/src/awareness.rs` | Add `room_id` to `Awareness` + `Candidate`; populate in `perceived_candidates`/`into_awareness`. + test asserting `room_id`. |
| 3 | `crates/oathstar-core/src/lib.rs` | `GameState.pack: Vec<String>` (`#[serde(default)]`); init in `try_new`; `handle_command` `Talk`/`Take` arms; `talk_at`/`take_at`; `snapshot()` builds `pack` from `world.items`; extend `help`. + engine tests. |
| 4 | `crates/oathstar-protocol/src/lib.rs` | Add `PackItemSnapshot { id, name }` + `GameSnapshot.pack` (`default, skip_serializing_if = "Vec::is_empty"`). + serde tests. |
| 5 | `src/client/snapshot.js` | `toPack(snapshot)` reads `snapshot.pack`; `toMenuModel` passes the snapshot through. |
| 6 | `src/client-app.js` | Pack render block: append a name chip per `menu.pack.items` (mirror the Nearby loop) so a non-empty pack renders honestly. |
| 7 | `tests/client.test.js` | Add `toPack`/`toMenuModel` cases (reads `snapshot.pack`; empty/absent → count 0; names surfaced). |
| 8 | `modules/beginner/world.toml` | Add one ground `[[items]]` (e.g. `wax_stub`). |
| 9 | `modules/beginner/rooms.toml` | Place it: `items = ["wax_stub"]` on `hollowmere_square`. |
| 10 | `docs/spatial-awareness.md` | Commands section: mark `talk`/`take` implemented (were pre-described as future). Minimal edit. |

No change: `oathstar-datastar` (E2), `oathstar-server`, `oathstar-storage`, `docs/decisions.md`
(reuses Decisions 002/028/031/036 — no new decision).

### Regression Test Plan
At least one row per acceptance criterion. Rust tests are inline `#[cfg(test)]` reusing
existing helpers (`proximity_engine`, `model_world`, `cmd`, `narrative_text`).

| # | Test (file) | Proves |
|---|---|---|
| T-P1 | `command.rs`: `talk`/`speak` + target → `Talk{target}`; multi-word target; verb case-folded, target case kept | REQ-001 |
| T-P2 | `command.rs`: `take`/`get`/`pick up` + target → `Take{target}`, text preserved (incl. `pick up black candle`) | REQ-002 |
| T-P3 | `command.rs`: bare `talk`/`take`/`get`/`speak`/`pick`/`pick up` (no target) → `Unknown` (strict arity); `pickup` one-word → `Unknown` | REQ-001/002 boundary |
| T-T1 | `lib.rs`: talk to interactable **conversable** actor → Narrative response naming it, `accepted`, `current_room_id` unchanged | REQ-003 |
| T-T2 | `lib.rs`: talk to interactable **non-conversable** actor → "nothing to say" response, no move (kills role-branch mutant) | REQ-003 |
| T-T3 | `lib.rs`: talk to **visible-but-too-far** actor → "too far away to talk", `!accepted`, no move | REQ-004 |
| T-T4 | `lib.rs`: talk to an interactable **non-actor** (item) → "can't talk to that", `!accepted` | REQ-003 kind-gate |
| T-T5 | `lib.rs`: talk **unknown** target → "no one like that", `!accepted`, no move | REQ-004 sibling |
| T-K1 | `lib.rs`: take interactable world item → `accepted`, Inventory/`ItemCard` "You take…", id in `state.pack`, removed from room `items`, **absent from `snapshot.room.contents`** | REQ-005 |
| T-K2 | `lib.rs`: take a **too-far** (visible) item → "too far to reach", `!accepted`, item still in room + contents (state preserved) | REQ-006 too-far |
| T-K3 | `lib.rs`: take a **non-item** interactable (actor) → "can't carry", `!accepted`, state preserved | REQ-006 not-an-item |
| T-K4 | `lib.rs`: take **unknown/hidden** target → "nothing like that", `!accepted`, state preserved | REQ-006 unknown/hidden |
| T-K5 | `lib.rs`: take item placed in an **adjacent interactable cell** (d=1) removes it from the correct room (proves `Awareness.room_id`) | REQ-005 / decision A |
| T-A1 | `awareness.rs`: `perceive`/`resolve_target` set `room_id` to the placing room | REQ-005 support |
| T-S1 | `oathstar-protocol`: `PackItemSnapshot` serializes `{id,name}`; empty `pack` omitted; legacy snapshot w/o `pack` deserializes; populated round-trips | REQ-007 |
| T-S2 | `lib.rs`: snapshot after `take` exposes `pack=[{id,name}]` resolved from `world.items` | REQ-007 |
| T-J1 | `tests/client.test.js`: `toPack(snapshot)` → `{count, items[].name}` from `snapshot.pack`; empty/absent → 0; `toMenuModel` carries it | REQ-007 (JS) |
| T-R1 | `bin/gate.sh` (cargo test + node --test): existing look/move/oath/canvas/event-feed + datastar + server + content suites stay green | REQ-008 |

**Coverage note (honest):** the `client-app.js` DOM append loop (#6) has no unit test — the
repo unit-tests view-models (`snapshot.js`), not the DOM, so the honest-render piece of
REQ-007 is proven by T-J1 (`toPack`) + inspect review + `/verify` smoke, not a DOM test.
No other genuinely-uncoverable paths.

### Risks / decisions
- **R1 (A):** `Awareness` gains a public field — additive, contained to `awareness.rs`, off
  the wire. Covered by T-A1.
- **R2 (E):** `take` is a `LogMessage`, not a typed domain event — reversible; a typed
  `ItemTaken` can be added later if domain consumers need it.
- **R3 (H):** the new start-room item changes `hollowmere_square` snapshot contents — verify
  no server/JS test pins that room's contents as empty (initial scan: server tests assert
  `RoomEntered`/`LogMessage`, JS uses synthetic snapshots). Re-check at Validate.
- **R4:** `take` mutates the engine-owned `world` placement; with no persistence (scope-out)
  a reload resets taken items — acceptable for v1.
- **R5:** pack stores ids; names resolved from `world.items` (registry entry retained — only
  the room placement is removed). Documented assumption.
- **R6 (parser):** guard `pick`-alone and `pick up`-with-no-target → `Unknown`; one-word
  `pickup` is not `take`. Covered by T-P3.

## Phase 3 — Implement
- **Built (all 10 manifest items):**
  1. `command.rs` — `Command::Talk { target }` + `Take { target }`; parse `talk`/`speak`,
     `take`/`get`, two-token `pick up`; bare verb / `pick`-only / one-word `pickup` → `Unknown`.
  2. `awareness.rs` — `room_id` on `Awareness` + `Candidate`, populated from the iterated room.
  3. `lib.rs` — `GameState.pack: Vec<String>` (`#[serde(default)]`) + `try_new` init;
     `handle_command` `Talk`/`Take` arms (return via `response(accepted, …)`); `talk_at`/
     `take_at` (kind→proximity gating, `conversable` flavor, take mutates room placement +
     pushes pack); `pack_snapshot()` helper resolving names from `world.items`; help text.
  4. `oathstar-protocol` — `PackItemSnapshot { id, name }` + `GameSnapshot.pack`
     (`default, skip_serializing_if = "Vec::is_empty"`).
  5. `src/client/snapshot.js` — `toPack(snapshot)` reads `snapshot.pack`; `toMenuModel` passes it.
  6. `src/client-app.js` — pack render loop + `packChip` helper.
  7/8. `modules/beginner/world.toml` + `rooms.toml` — `wax_stub` item placed in `hollowmere_square`.
  9. `docs/spatial-awareness.md` — Commands section documents talk/take; removed from future-scope.
- **Compile/regression checks (Phase 3 sanity; full suite is Phase 4):** `cargo fmt` clean;
  `cargo check --workspace` clean; **147 existing Rust tests pass, 27 existing JS tests pass
  (zero regressions, REQ-008 holding)**; beginner module loads + validates with the new item;
  both JS files `node --check` clean.
- **Deviations from design (+ reason):**
  - **Help text appended, not inserted.** Design said "extend help, preserving substrings." To
    keep the existing `help_command_lists_directions` test (`contains("look, north")`) green
    *without editing a test in implement*, talk/take are appended after `confront` rather than
    placed after `look`. Validate may add a positive talk/take assertion and (optionally)
    de-brittle the `"look, north"` check.
  - **`packChip` helper added** instead of reusing `actionCard` (which builds a `look`-action card
    inappropriate for carried items) or `emptyChip` (muted styling). Minimal `chip`-class span,
    mirrors `emptyChip`.
  - **Tests NOT written here** (per implement-phase rule). The full Regression Test Plan (T-P*,
    T-T*, T-K*, T-A1, T-S*, T-J1) is Phase 4 — validate writes + runs them. Production code is in
    place and exercised only by existing tests so far.

## Inspect (Phase 3.5)
- **Lenses run** (3 independent general-purpose critics over `git diff HEAD`):
  (1) correctness — talk/take gating + `accepted` semantics + REQ-001..008 mapping + panics;
  (2) data/state integrity — take mutation via `room_id`, perceive-drop, pack back-compat,
  determinism, borrow soundness (critic verified live with a probe; worktree left clean);
  (3) parser edge cases + simplification/reuse + mutation-survivability.
- **Inspector's own verification:** `cargo clippy --workspace --all-targets` **clean** (strict
  pedantic+nursery+restriction); 88 core + 27 JS + all workspace tests green; the take-drops-from-
  contents flow was reproduced live by critic #2.
- **Verdict: no CRITICAL/HIGH/MEDIUM findings; no code fixes required.** Diff is internally
  consistent with the `swear`/`confront` precedent, panic-free on resolved data, and clippy-clean.

- **Findings (all LOW) + verdicts:**
  | # | Sev | Finding (file:line) | Verdict |
  |---|---|---|---|
  | F1 | LOW | talk/take prose ("…here to talk") diverges from look's ("…nearby") — `lib.rs` talk_at/take_at | **Rejected (not a bug).** Intentional; no REQ pins prose. Validate must not assume look's strings. |
  | F2 | LOW | nearest-name-match: a too-far actor beats a farther out-of-sight namesake → "too far" (REQ-004) | **Rejected (correct).** Desired behavior. Coverage note for validate. |
  | F3 | LOW | demo item `wax_stub` sits in the player's own cell (Exact, d0), so module data doesn't exercise the *adjacent-cell* `room_id` path | **Deferred to validate**, not content. T-K5 (synthetic adjacent-cell take) is the right place to lock `room_id`; don't contort content for coverage. |
  | F4 | LOW | `is_some_and` `None`-arm in `talk_at` is unreachable (resolved Actor always exists in `world.entities`) — latent mutant | **Keep as-is.** Defensive lookup is correct §14; do NOT thread `roles` through `Awareness`. The *outer* result is killable by the conversable-vs-not tests (T-T1/T-T2). |
  | F5 | LOW | `pack_snapshot` `map_or_else` id-fallback is near-unreachable (registry survives take) — latent mutant | **Keep as-is; validate kills it** with a unit test pushing a bogus id into `state.pack` and asserting name==id (or excludes in `.cargo/mutants.toml` if needed). |
  | F6 | LOW | `take_at` 4-tuple vs `talk_at` 2-tuple asymmetry; `packChip`/`emptyChip` differ only by class | **Rejected (style).** Asymmetry is justified by real per-arm channel divergence (Inventory/ItemCard vs Narrative); `packChip` matches the file's verbose-helper idiom. Not worth churn. |

- **Carried forward to Phase 4 — mutation-survivability MUST-PIN checklist** (MUT_MSI=100 at /commit):
  - Parser: both `talk`/`speak` and `take`/`get` aliases + an UPPERCASE verb; bare verb → `Unknown` (exact collapsed echo); `pick up <t>` accepted AND all four negatives (`pick`, `pick up`, `pick xyz`, one-word `pickup`) → `Unknown`; target case+whitespace preserved (`talk  Mara  X` → `"Mara X"`).
  - `talk_at`: `kind != Actor` guard (fixture → "cannot hold a conversation"); `!is_interactable()` boundary (d2 actor → "too far"); kind-before-reach ordering (assert text); `role == "conversable"` true/false lines (mara vs a plain reachable actor); `accepted` true only on reachable actor; `current_room_id` unchanged on accept (REQ-003).
  - `take_at`: `kind != Item` guard (actor → "cannot carry"); `!is_interactable()` boundary (d2 item → "too far"); `retain(|id| id != &found.id)` (use a room with ≥2 items — taken id gone, others remain); `get_mut(&found.room_id)` proven by taking from an **adjacent** cell; `state.pack.push` + pickup-order; item drops from `snapshot.room.contents` (REQ-005); success channel/component = `Inventory`/`ItemCard`; `accepted`/state-unchanged on every refusal (REQ-006).
  - `pack_snapshot`: carried id resolves to registry **name** (take `wax_stub` → "Wax Stub"); fallback-to-id via a bogus-id unit test.
  - Protocol: empty `pack` omitted; packless payload deserializes; populated round-trip (mirror the `contents` tests).
  - JS `toPack`: `{pack:[…]}` → count/names; `{}`/`undefined` → count 0; `name ?? id ?? "Something"` fallback (entry with only id; entry with neither). Render: empty chip present iff count 0, item chips present iff count > 0.
  - Help: add a positive assertion that help lists `talk` and `take` (existing test only pins "look, north" + swear/confront).

## Phase 4 — Validate
- **Tests added (+29):**
  - `command.rs` (+5): talk/speak + take/get aliases + uppercase verb; bare verbs → Unknown;
    `pick up`/`pick`/`pickup` arity (4 negatives); target case + whitespace preservation incl. `pick up UP`.
  - `awareness.rs` (+1): `room_id` = placing room for an origin-cell entity, an adjacent-cell
    entity, and an adjacent-cell item.
  - `lib.rs` (+15): `interaction_engine` fixture + `log_text` helper; talk conversable/non-conversable/
    too-far/non-actor/unknown (+ no-move); take success (exact + **adjacent-cell room_id**, ≥2-item
    retain, drops from contents, Inventory/ItemCard event, accepted, no-move); too-far/non-item/
    unknown/hidden refusals preserve state; pickup-order; no double-take; pack name-resolution +
    **bogus-id fallback**; help-lists-talk-take.
  - `oathstar-protocol` (+4): PackItemSnapshot fields; empty pack omitted; legacy (packless) deserialize; populated round-trip.
  - `tests/client.test.js` (+4): `toPack` populated/absent/empty/fallback; `toMenuModel` carries pack.
  - Every inspect mutation-checklist must-pin point has a dedicated assertion (kind/`is_interactable`
    boundaries, `retain` predicate with a 2-item room, conversable true/false, `room_id` adjacent-cell,
    per-arm `accepted`, Inventory/ItemCard channel, id-fallback).
- **`cargo test --workspace`: PASS** — content 12, core **109** (was 88, +21), datastar 11, protocol
  **8** (was 4, +4), server 12, storage 20. 0 failed.
- **`node --test tests/*.test.js`: PASS** — **31** (was 27, +4). 0 failed.
- **`npm run build`: PASS** (vite build clean; protocol `pack` shape consumed by client).
- **`bin/gate.sh --fast`: GATE GREEN [fast] — 14/14** (rustfmt, clippy strict, cargo test, node
  --test, cargo-audit, cargo-deny, cargo-machete, gitleaks, shellcheck, no-suppressions, source-bans,
  lints-allowlist, doc-todos, tauri shell). One transient red (gate:1 rustfmt on freshly-added test
  code) fixed at source via `cargo fmt`; re-run green.
- **Deferred to `/commit` (FULL gate):** gates 15–17 (Rust coverage ≥94, JS coverage ≥75, mutation
  MSI 100). `--fast` was the validation scope per the ticket; the suite was authored against the
  inspect mutation checklist so the FULL gate should pass at `/commit`.
- **Pre-existing exclusions:** none — no pre-existing failures; no unrelated breakage touched.

## Phase 5 — Complete
- **Docs updated:** `docs/spatial-awareness.md` Commands section (talk/take documented as implemented;
  removed from future-scope) — done in Phase 3. Confirmed no other command-listing doc drift;
  `docs/decisions.md` needs no new decision (reuses 002 parser / 028 events / 031 wire / 036 awareness).
  `CLAUDE.md` conventions unchanged. `MEMORY.md` unchanged (no durable cross-session fact beyond what
  the forge + repo already record).
- **Forge capture:**
  - `aar-submit` → AAR `275a6f78-…` closed `completed`, effectiveness 5, 12 verdicts written, 2 novel
    findings, distillation/confidence-drift/pattern-emergence jobs enqueued.
  - `architecture-decision-record` → **AD-claude-nearby-actions-001** (talk/take reuse the awareness
    resolver; `Awareness.room_id`; additive ids-only `pack`; LogMessage over a typed take event).
  - `prevention-rule-record` → **PR-claude-wire-enum-variant-cost-001** (grep exhaustive matches before
    adding a wire/event enum variant; reuse an existing variant for minimal additive actions).
  - No `failure-record` — inspect surfaced no real defects.
- **Ticket:** forge #18 left OPEN with a completion `ticket-comment` (impl complete, pending Codex
  review + `/commit`); forge `ticket-close` deliberately deferred to post-commit. Local ticket doc
  `TICKET-18-*.md` marked `status: closed` and moved `open/` → `closed/`.
- **Archived:** this `.spec.md` + `.notes.md` moved `pipeline/active/` → `pipeline/completed/`.
- **Recommended follow-ups:** ticket #20 (Inventory V1) builds on `GameState.pack`/`GameSnapshot.pack`;
  a typed `GameEventKind::ItemTaken` if domain consumers need to dispatch on takes; content can now
  place takeable items in adjacent (d1) cells. FULL gate (Rust cov ≥94, JS cov ≥75, mutation MSI 100)
  runs at `/commit`.
