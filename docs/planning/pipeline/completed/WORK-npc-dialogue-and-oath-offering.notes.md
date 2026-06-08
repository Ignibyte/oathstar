# WORK-npc-dialogue-and-oath-offering — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #19 — move the beginner Hollow Bell oath from a contextless
  global `swear` to an NPC-offered flow: `talk mara` (the issuer, at interaction
  distance) returns authored dialogue and offers the oath; `swear` then binds the
  *offered* oath and emits the existing `OathSworn` shape; add issuer/source
  metadata to the oath model; Mara's dialogue reflects already-sworn/fulfilled;
  Bell-Eater progression stays playable.
- **Intake source:** none (not promoted from an intake doc).
- **Classification / tier:** work pipeline — a single shippable slice (one NPC =
  Mara, one oath = Hollow Bell, command-based dialogue, an authored offer gate,
  issuer metadata). Builds directly on #17 (awareness) + #18 (talk/take). No
  split needed; scope-outs hold.
- **Forge recall (AAR `bcfdb840-0d77-409e-a6e4-6ea1a821bb17`):** `bulletin-list`
  empty. `knowledge-context` (Plan) surfaced + logged 13 nodes. Design/Implement
  must pull the bodies of these via `knowledge-context` at phase entry and apply
  them:
  - architecture_decisions: `e6210497-…`, `1700e917-…`, `2530308c-…` (map to
    decisions.md 028/030/031 — typed events, construction-boundary validation,
    stable wire split).
  - distilled_lessons: `051347df-…`, `8075dc26-…`, `c851de70-…`.
  - prevention_rules: `a8213de4-…`, `1f466495-…`, `1641f517-…`, `464d9c69-…`,
    `7ba20e34-…`.
  - recent_failures: `eb5805b6-…`, `6e58ea62-…`.
  - **Load-bearing lesson (from the #7/#18 design record):** under the 100% MSI
    mutation gate, an *unconstructed* enum variant or *unreachable* defensive
    branch is an uncoverable mutant — `OathBroken` was deliberately omitted for
    exactly this reason. Prefer `expect`-invariants guaranteed by `validate()`
    (the `current_room()` precedent). New "offered" state + issuer metadata must
    be reachable and test-killable.
- **Ticket:** forge #19 `8a66fea8-56eb-4015-b445-2608b8c4ddbf` (already existed;
  not re-created). Local doc
  `docs/planning/tickets/open/TICKET-19-npc-dialogue-and-oath-offering-v1.md`
  (`pipeline_spec` field linked to this pipeline).
- **EARS requirements reviewed:** REQ-001..007 carried verbatim into the spec.

### Current-code anchors (ground truth; index lags uncommitted #18 — read the tree)
| Area | Location |
|---|---|
| `talk_at` handler (hardcoded strings today) | `crates/oathstar-core/src/lib.rs:666-702` |
| `swear()` (module-global, offer-less) | `crates/oathstar-core/src/lib.rs:802-852` |
| `confront()` (Sworn→Fulfilled, REQ-007) | `crates/oathstar-core/src/lib.rs:861-920` |
| `OathDefinition {id,title,description}` (REQ-006 add metadata here) | `crates/oathstar-core/src/lib.rs:136-140` |
| `GameState.oath: Option<OathProgress>` (None=available) | `crates/oathstar-core/src/lib.rs:386-400` |
| `Entity {…, roles, …}` (no dialogue fields yet) | `crates/oathstar-core/src/lib.rs:94-111` |
| Awareness resolver (`resolve_target`/`perceive`, radii) | `crates/oathstar-core/src/awareness.rs:314-342` |
| Parser (`Command::Talk`/`Swear` already exist) | `crates/oathstar-core/src/command.rs:54-84,140-173` |
| Content loader structs (`ModuleToml.oath_id`, `WorldToml.oaths`) + `index_by_id` | `crates/oathstar-content/src/lib.rs:14-22,51-88` |
| `OathSworn{oath_id,title}` / `OathFulfilled{oath_id}` | `crates/oathstar-protocol/src/lib.rs:207-215` |
| `OathSnapshot` / `GameSnapshot.oath` | `crates/oathstar-protocol/src/lib.rs:32-34,158-164` |
| Mara entity (roles `conversable`,`shopkeeper`) | `modules/beginner/world.toml:44-51` |
| Hollow Bell oath `[[oaths]]` | `modules/beginner/world.toml:87-90` |
| Bell-Eater boss | `modules/beginner/world.toml:76-83` |
| Mara placement `candle_shop (1,0,0)` | `modules/beginner/rooms.toml:20-33` |
| Client intent vocab + contextual `swear`/`confront` | `src/client/intent.js:6-30` |
| Client oaths/nearby snapshot mapping | `src/client/snapshot.js:41-63,72-136` |

### Open questions for Phase 2 (design must settle — not pre-decided here)
1. **"Offered" representation:** new `GameState` field (e.g.
   `offered_oath_id: Option<String>`), an NPC-memory flag, or a new
   pre-swear oath value? Must be reachable + mutation-killable.
2. **Dialogue authoring shape:** a `dialogue` field on `Entity`, a `[[dialogue]]`
   table, or oath-keyed lines (`offer` / `already_sworn` / `fulfilled`)? Must
   serve REQ-001 (authored), REQ-002 (introduce problem + expose oath), and
   REQ-005 (state-reflective) with the fewest new types.
3. **Offer trigger:** does `talk mara` set the offered state as a side effect
   (talk mutates state — today `talk_at` only emits narrative), and does it emit a
   new typed event or just narrative + a snapshot flag?
4. **Issuer/source field set on `OathDefinition`:** `issuer_id` (entity ref),
   `source`/`region`/`faction`? How much is "enough" for REQ-006 without
   overbuilding? Validate a dangling `issuer_id` at the construction boundary
   (Decision 030) only if a test can construct the failure.
5. **`swear` gate semantics (REQ-003):** require only that the oath was *offered*,
   or also that the player is near the issuer at swear time? What refusal/guidance
   text points to the oath-giver?
6. **Client guidance:** does the snapshot need an additive "oath offered" signal
   so `intent.js` stops advertising `swear` until offered, or is the engine's
   refusal text sufficient for v1?

## Phase 2 — Design

### Approach / architecture
A thin **authored-dialogue + oath-offer gate** layer over the existing engine,
reusing every established pattern (TOML-deserialized core content types, typed
`WorldValidationError` at the construction boundary, the awareness resolver,
`expect`-invariants over unreachable arms, additive `#[serde(default)]` fields).
No new transport, **no protocol change, no JS change** — dialogue is delivered as
the existing `NarrativeMessage` event; the oath wire shapes are untouched.

**Resolved open questions (from Phase 1 notes):**
1. **"Offered" representation → `GameState.offered_oath_id: Option<String>`**
   (`#[serde(default)]`, like `oath`/`pack`). Set when the player talks to the
   oath's issuer; read by `swear`. Smallest honest flag (Decision 006 "authored
   flag over relationship meter"); both set/unset states are test-reachable. Not
   an `OathStatus` variant (that enum is the *post-swear* lifecycle; pre-swear is
   `oath == None`).
2. **Dialogue authoring → `Entity.dialogue: Option<EntityDialogue>`** where
   `EntityDialogue { greeting: String, oath: Option<OathDialogue> }` and
   `OathDialogue { offer: String, sworn: String, fulfilled: String }`. Required
   inner strings (no per-field `Option`/`unwrap_or` → no uncoverable fallback
   branch); the whole `oath` block is optional so a plain conversable NPC carries
   only a `greeting`. Matches REQ-001 ("a talkable NPC *has* dialogue metadata").
   New core content types, `#[derive(Debug, Clone, PartialEq, Eq, Serialize,
   Deserialize)]` (Entity already derives Eq).
3. **Offer trigger → `talk_at` side-effect.** Talking to the designated oath's
   issuer while `oath == None` records `offered_oath_id` and returns the `offer`
   line. Idempotent (re-talking re-offers). No new event type — the dialogue is a
   `NarrativeMessage`, same as today's talk reply.
4. **Issuer/source on the model → `OathDefinition` gains
   `issuer_id: Option<String>` + `source: Option<String>`** (both
   `#[serde(default)]`). `issuer_id` = the oath-giver entity (drives the offer
   gate); `source` = region/faction origin string for future UI/effects (REQ-006).
5. **`swear` gate semantics → offer-gated only when the oath has an `issuer_id`.**
   An issuer-less oath stays globally swearable (backward-compatible — the
   existing `oath_world()` tests pass unchanged). The beginner oath gains
   `issuer_id="mara"`, so it takes the gated path: `swear` before
   `offered_oath_id == Some(oath_id)` is refused with a message naming the issuer
   (REQ-003). The gate sits between the existing "no designated oath" check and
   the record step; the player need not be *near* the issuer at swear time (being
   offered once is enough), which keeps the manual path simple.
6. **Client signal → none (engine guidance only).** The base `COMMAND_VOCAB`
   always lists `swear`, so gating only `contextualCommands` wouldn't hide it;
   truly gating the UI is out of proportion for v1. The engine's REQ-003
   refusal+guidance makes the path obvious, and Mara already exposes a Talk action
   in the Nearby panel (#18). Protocol + JS unchanged. (Future polish: an additive
   `oathOffered` snapshot flag could gate the suggestion proactively.)

**Construction-boundary validation (Decision 030):** `WorldDefinition::validate()`
gains an `OathIssuerMissing { oath_id, issuer_id }` check — each oath whose
`issuer_id` is `Some` must name a known entity. Reachable from crafted TOML, so
the swear/talk issuer lookups can use `.expect("…validated invariant")` (the
`current_room()` / designated-oath precedent) rather than an unreachable defensive
arm.

**Command flow (engine):**
- `talk_at` → extracts a private `npc_dialogue_line(&mut self, entity_id) -> String`:
  no `dialogue` → today's generic line (`conversable` ? "turns to face you" :
  "nothing to say") — preserves #18; `dialogue` present + this NPC issues the
  designated oath + `dialogue.oath` is `Some` → match oath state
  (`Fulfilled`→`fulfilled`, `Sworn`→`sworn`, `None`→record `offered_oath_id` then
  `offer`); otherwise → `greeting`. **Clone the line string before the
  `offered_oath_id` write** to keep the `world` (immutable) / `state` (mutable)
  borrows disjoint.
- `swear` → after the existing `expect`-validated oath lookup, clone
  `issuer_id`/`title`/`description` out (ending the `oath` borrow), then if
  `issuer_id` is `Some` and `offered_oath_id != Some(oath_id)`, look up the issuer
  name (`expect`-invariant) and return a refusal naming the issuer; else proceed
  unchanged (record `Sworn`, emit `OathSworn{oath_id,title}` — **unchanged shape**).
- `confront` → unchanged (REQ-007).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/lib.rs` | Add `EntityDialogue{greeting, oath:Option<OathDialogue>}` + `OathDialogue{offer,sworn,fulfilled}`; add `#[serde(default)] dialogue: Option<EntityDialogue>` to `Entity`; add `#[serde(default)] issuer_id: Option<String>` + `source: Option<String>` to `OathDefinition`; add `WorldValidationError::OathIssuerMissing{oath_id,issuer_id}` + its `Display` arm; in `validate()` (after the `oath_id` check ~L371) loop oaths and reject an unknown `issuer_id`; add `#[serde(default)] offered_oath_id: Option<String>` to `GameState` + init `None` in `try_new`'s state; add `npc_dialogue_line` + rewire `talk_at` (REQ-001/002/005); add the offer gate to `swear` (REQ-003/004); update the `oath_world()` test literal (`issuer_id:None,source:None`); add the new tests below. |
| 2 | `crates/oathstar-core/src/awareness.rs` | Update the 2 `Entity{…}` test literals with `dialogue: None` (field addition ripple; test-only). |
| 3 | `crates/oathstar-content/src/lib.rs` | No struct changes (it deserializes core `Entity`/`OathDefinition` directly). Add content tests: beginner world loads Mara's dialogue + oath `issuer_id`/`source`; loader rejects an oath with a missing issuer; an oath without issuer/source still loads. |
| 4 | `crates/oathstar-server/src/main.rs` | Update `beginner_slice_runs_through_command_path`: insert `talk mara` before `swear` (the real oath is now offer-gated) and assert the offer narrative; rest of the route/confront unchanged. (Optional: a bare-`swear`-refused test.) |
| 5 | `modules/beginner/world.toml` | Add `issuer_id = "mara"` + `source = "hollowmere"` to `[[oaths]] hollow_bell`; add `[entities.dialogue]` (`greeting`) + `[entities.dialogue.oath]` (`offer`/`sworn`/`fulfilled`) sub-tables to Mara, placed **immediately after Mara's `[[entities]]` block and before the next `[[items]]`** (TOML array-of-tables subtable rule). |

No changes to `oathstar-protocol` (oath event/snapshot shapes preserved) or `src/` (JS client). `oathstar-content` needs no struct edits because new core fields are `#[serde(default)]`.

### Regression Test Plan
| # | Test (location) | Proves |
|---|---|---|
| T1 | `talk_dialogue_npc_returns_authored_greeting` (core) — talk a dialogue NPC that is not the oath issuer → its authored `greeting`, not the hardcoded line | REQ-001 |
| T2 | `talk_npc_without_dialogue_keeps_generic_line` (core) — conversable NPC, `dialogue:None` → "turns to face you" (and a non-conversable → "nothing to say") | REQ-001 (preserve #18) |
| T3 | `talk_issuer_offers_oath_and_enables_swear` (core) — talk issuer with `oath==None` → `offer` line (introduces the problem / exposes swearing) **and** `offered_oath_id` set so a following `swear` is accepted | REQ-002 |
| T4 | `swear_before_offer_refused_and_names_issuer` (core) — fresh engine, `swear` → refused, message names the issuer, `oath` stays `None` | REQ-003 |
| T5 | `swear_without_issuer_stays_globally_swearable` = existing `swear_sets_oath_active_and_emits_oath_sworn` on `oath_world()` (issuer-less) still accepted with no talk | REQ-003 (backward-compat path) |
| T6 | `swear_after_offer_binds_and_emits_unchanged_oath_sworn` (core) — talk→swear → accepted, `OathSworn{oath_id,title}` shape unchanged, snapshot `oath.status==Sworn` | REQ-004 |
| T7 | `dialogue_reflects_sworn_state` (core) — swear→talk issuer → `sworn` line | REQ-005 |
| T8 | `dialogue_reflects_fulfilled_state` (core) — swear→confront→talk issuer → `fulfilled` line | REQ-005 |
| T9 | `validate_rejects_oath_with_unknown_issuer` (core) — world with oath `issuer_id` ∉ entities → `OathIssuerMissing` | REQ-006 (model + Decision 030) |
| T10 | `beginner_world_loads_mara_dialogue_and_oath_issuer` (content) — `load_beginner_world()`: `entities["mara"].dialogue` Some w/ non-empty `offer`; `oaths["hollow_bell"].issuer_id==Some("mara")`, `source==Some("hollowmere")` | REQ-001 + REQ-006 (authored-from-data) |
| T11 | `load_rejects_oath_with_missing_issuer` (content) — TOML oath `issuer_id="ghost"` → load error "missing issuer 'ghost'" | REQ-006 (loader boundary) |
| T12 | `load_accepts_oath_without_issuer` (content) — oath with no issuer/source loads (serde default `None`) | REQ-006 (default path / backward-compat) |
| T13 | `beginner_slice_runs_through_command_path` (server, **updated**) — look → **talk mara (assert offer narrative)** → swear (OathSworn) → route → confront (OathFulfilled) | REQ-002 + REQ-007 (real-world end-to-end) |
| T14 | `bin/gate.sh --fast` (cargo test + node --test, fmt, clippy) green; existing oath/talk/look/move/snapshot suites pass | REQ-007 |

**Genuinely uncoverable / by-design:** the new `.expect("oath issuer is a
try_new-validated invariant")` in `swear`/`talk` panic branches are unreachable
for a constructed `Engine` (guaranteed by the new `OathIssuerMissing` validation),
mirroring the existing `current_room()` and designated-oath `expect`s that already
pass the 100% MSI gate. Mara's authored `greeting` is unused by the *beginner*
module (she always hits the oath path) but is exercised by core test T1, so its
branch is killed.

### Risks / decisions (reversible-but-load-bearing)
- **Existing server smoke breaks without the talk step.** Adding `issuer_id` to
  the beginner oath makes the bare `swear` in `beginner_slice_runs_through_command_path`
  refuse; T13 *must* insert `talk mara` first. (Caught in design — not a
  regression if updated together.)
- **Offer gate is opt-in via `issuer_id`.** Keeps every issuer-less oath (and the
  synthetic `oath_world` tests) on the original global-swear path; only oaths that
  declare an issuer are gated. Reversible: dropping `issuer_id` reverts an oath to
  global swear.
- **`source` is stored, not yet read by engine logic.** Satisfies REQ-006's
  "record … to support future" without overbuilding; covered by T10's assertion
  (kills its deserialization mutant). No region/faction effect is implemented now
  (scoped out).
- **No protocol/JS change.** Tightest scope; the UX guidance is engine-side. If
  playtest shows players miss the talk-first step, a follow-up can add an additive
  `oathOffered` snapshot flag + intent gating.
- **Field-addition ripple.** Adding `dialogue` to `Entity` forces `dialogue:None`
  on ~5 test literals (core + awareness); adding issuer/source forces 2 fields on
  the one `oath_world()` literal. Mechanical; enumerated in the manifest.

## Phase 3 — Implement
- **Built (production code + content; per the manifest):**
  - `crates/oathstar-core/src/lib.rs`: added `EntityDialogue{greeting, oath:Option<OathDialogue>}` + `OathDialogue{offer,sworn,fulfilled}`; `Entity.dialogue: Option<EntityDialogue>` (`serde(default)`); `OathDefinition.issuer_id` + `source` (`serde(default)`); `WorldValidationError::OathIssuerMissing{oath_id,issuer_id}` + its `Display` arm; `validate()` loop rejecting an oath whose `issuer_id` ∉ entities (Decision 030); `GameState.offered_oath_id: Option<String>` (`serde(default)`) + init `None` in `try_new`; new `npc_dialogue_line(&mut self, entity_id)` helper (authored line by oath state; clone-before-mutate so `world`/`state` borrows stay disjoint; consumes the offer by setting `offered_oath_id`); rewired `talk_at`'s reachable-actor arm to call it; added the offer gate to `swear` (refuse + name the issuer when the oath is issuer-offered and not yet offered; issuer-less oaths stay globally swearable). `OathSworn`/`OathFulfilled` shapes unchanged.
  - `crates/oathstar-core/src/awareness.rs` + `lib.rs`: `dialogue: None` on the two `Entity{…}` builder helpers; `issuer_id:None, source:None` on the `oath_world()` `OathDefinition` literal (compile-fix only).
  - `modules/beginner/world.toml`: `issuer_id="mara"` + `source="hollowmere"` on `[[oaths]] hollow_bell`; `[entities.dialogue]` (`greeting`) + `[entities.dialogue.oath]` (`offer`/`sworn`/`fulfilled`) on Mara, with the offer line introducing the Bell-Eater problem and signposting the swear.
  - **No `oathstar-protocol` change, no `src/` (JS) change** — dialogue rides the existing `NarrativeMessage`; the oath wire shapes are preserved.
- **Verified:** `cargo fmt --all` clean; `cargo check --workspace --all-targets` green (the borrow-safe `npc_dialogue_line`/`swear` typecheck); existing `oathstar-content` tests (12) pass — so the authored TOML parses, deserializes, and passes the new issuer validation on the real beginner world.
- **Deviations from design (+ reason):**
  1. **Test work deferred to Phase 4 (per phase rules).** Manifest files #3 (content tests T10–T12) and #4 (server-smoke `talk mara` step, T13) and the new core tests T1–T9 are NOT written here — `/pipeline:implement` forbids writing/expanding tests beyond what compiles. Only the mechanical literal compile-fixes were done now.
  2. **⚠ Known-expected RED until Phase 4:** the existing server smoke `beginner_slice_runs_through_command_path` (`oathstar-server/src/main.rs:338`) swears the *real* beginner oath **without talking first**; the new offer gate now (correctly) refuses that bare `swear`, so this test will FAIL until Phase 4 inserts `talk mara` before `swear` (planned as T13). This is the intended consequence of the offer gate, not a regression — flagged here so Inspect/Validate expect it.
  3. **Awareness literal count:** design said "2 `Entity{…}` literals" in `awareness.rs`; there is only **1** builder literal (`make_entity`) — the second `Entity {` grep hit was the `-> Entity {` return type. One literal fixed; no behavioral impact.

## Inspect (Phase 3.5)
- **Lenses run** (4 parallel critics over the #19 surface only; #18 base excluded):
  Correctness · 100%-mutation-MSI readiness · serde/state-integrity & wire stability ·
  simplification/strict-clippy.
- **Findings:**

  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | HIGH | `validate()` is 110/100 lines → `clippy::too_many_lines` → **gate:2 RED** (`crates/oathstar-core/src/lib.rs:314`); the #19 issuer loop tipped it over. (`cargo check` missed it — only `cargo clippy` runs the workspace lints.) | **REAL** — verified by running `cargo clippy --workspace --all-targets`. | **Fixed:** extracted the oath/issuer checks into a private `validate_oaths()` helper (source-fix, not `#[allow]`); `validate()` is now an orchestrator. Re-ran clippy → `Finished`, 0 warnings; content 12/12 + core 109/109 still pass. |
  | 2 | HIGH (flagged) | `OathDefinition.source` deserialization mutant could survive if its test only checks `.is_some()` (`lib.rs:153`). | **Not a Phase-3 code defect → Phase-4 test requirement.** `source` is data with a derived `Deserialize`; correctness depends on the test asserting it by value. | **Ledger note for Validate:** T10 MUST assert `oath.source == Some("hollowmere")` and `issuer_id == Some("mara")` by exact value (already specified in the Phase-2 plan). No code change. |
  | 3 | HIGH (flagged) | issuer-name `.expect("…validated invariant")` panic is never executed (`lib.rs` swear gate). | **Rejected as a defect — by-design.** Mirrors the existing `current_room()` and designated-oath `expect`s that already pass 100% MSI; `validate_oaths` guarantees the issuer exists, and removing the `expect` is compile-prevented (type mismatch). | None. |
  | 4 | — | **Correctness:** REQs 001–007 satisfied; offer gate ordered correctly; offered set only for the *designated* oath's issuer; sworn/fulfilled re-talk shows the right line without re-offering; no player-move/state leak; the new `expect`s are construction-validated-unreachable. | **Clean.** | None. |
  | 5 | — | **Serde/state/wire:** `GameState` is never persisted (fresh `Engine` at startup); all new fields `serde(default)` (old TOML/saves load); `oathstar-protocol` byte-identical (`OathSworn`/`OathFulfilled`/`OathSnapshot`/`GameSnapshot` unchanged — nothing #19 leaked); deterministic (`BTreeMap`); no collision path. | **Clean.** | None. |
  | 6 | LOW | **Simplification:** the `.clone()`s in `npc_dialogue_line`/`swear` are borrow-necessary (clone-before-mutate to keep `world`/`state` disjoint); `.map(str::to_owned)`, the `.filter` chain, the nested `if let`/`match`, and the new structs/`Display` arm are idiomatic and clippy-clean. No `option_if_let_else`/`collapsible_if`/`needless_borrow` fired. | **Clean** (no change). Future-only observation (out of #19 scope): a "test/find entity by role" idiom recurs 3× — not worth a helper now. | None. |

- **Carry-forward to Phase 4 (load-bearing tests, from the mutation critic):** T1 must use a *non-issuer* dialogue NPC (kills the always-true `issued_oath_id` mutant); T2 must hit *both* the conversable and non-conversable no-dialogue arms; T7 and T8 must be *separate* tests (Sworn vs Fulfilled arms); T9 must construct a world with a bad `issuer_id` and assert `OathIssuerMissing`; T10 must assert `source`/`issuer_id` by exact value; T3 should assert `offered_oath_id` is set; T4 (refused) + T6 (accepted) together kill the `!=`-comparison mutant.
- **Net:** 1 real defect found and fixed (gate:2 RED → green); 0 other code changes. `cargo clippy --workspace --all-targets` clean; `cargo fmt` clean; content + core suites green.

## Phase 4 — Validate
- **Tests added (+15), one+ per AC, covering the inspect critic's mutation-killing cases:**
  - `oathstar-core` (+10, via a new `dialogue_world()`/`dialogue_engine()` fixture):
    T1 `talk_dialogue_npc_that_is_not_issuer_returns_greeting` (REQ-001, guards the
    issuer filter), T2a `talk_conversable_npc_without_dialogue_uses_generic_line` +
    T2b `talk_non_conversable_npc_without_dialogue_has_nothing_to_say` (REQ-001),
    T3 `talk_issuer_offers_oath_and_records_offer` (REQ-002, asserts `offered_oath_id`),
    T4 `swear_before_offer_is_refused_and_guides_to_issuer` (REQ-003),
    T5 `swear_oath_without_issuer_needs_no_offer` (REQ-003 backward-compat),
    T6 `swear_after_offer_binds_and_emits_oath_sworn` (REQ-004, unchanged shape),
    T7 `dialogue_reflects_sworn_state` + T8 `dialogue_reflects_fulfilled_state` (REQ-005),
    T9 `validate_rejects_oath_with_unknown_issuer` (REQ-006).
  - `oathstar-content` (+3): T10 `beginner_world_loads_mara_dialogue_and_oath_issuer`
    (REQ-001/006, asserts `issuer_id`/`source` by **exact value** per the MSI critic),
    T11 `load_rejects_oath_with_missing_issuer` (REQ-006), T12
    `load_accepts_oath_without_issuer` (REQ-006 default path).
  - `oathstar-server` (+2): updated `beginner_slice_runs_through_command_path` to
    `talk mara` → assert offer narrative → swear → route → confront (REQ-002/007);
    added `beginner_swear_before_talking_to_mara_is_refused` (REQ-003, real world).
- **`cargo test --workspace`:** ✅ green — core 119, content 15, server 13, protocol 8,
  datastar 11, storage 20; 0 failed.
- **`node --test tests/*.test.js`:** ✅ green — 31 pass, 0 fail (JS unchanged).
- **`bin/gate.sh --fast`:** ✅ `GATE GREEN [fast]` — 14/14 (rustfmt, clippy strict,
  cargo test, node --test, cargo-audit, cargo-deny, cargo-machete, gitleaks,
  shellcheck, no-suppressions, source-bans, lints-allowlist, doc-todos, tauri-shell).
- **Coverage sanity (the FULL gate's gate:15 floor `RUST_COV_MIN=94`):** changed crates
  at 99.62% regions / **100% functions** / 99.65% lines — well above the floor.
- **FULL gate (gates 15–17: coverage + cargo-mutants 100% MSI):** deferred to `/commit`
  per the ticket (`--fast`) + CLAUDE.md (only a FULL green writes the commit receipt).
  Tests were authored to the inspect critic's load-bearing requirements (non-issuer
  filter, both generic arms, separate Sworn/Fulfilled, `OathIssuerMissing`, exact-value
  `source` assert, `offered_oath_id` assert, the `!=`-gate via T4+T6), and the new
  `.expect` invariants mirror the gate-passing `current_room()` precedent — so 100% MSI
  is expected; the machine-check runs at `/commit`.
- **Pre-existing exclusions:** none — no pre-existing failures; nothing skipped.

## Phase 5 — Complete
- Docs updated:
- Forge capture (aar/failures/rules/decisions):
- Ticket closed:
- Archived:
