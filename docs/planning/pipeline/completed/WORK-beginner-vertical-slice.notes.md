# WORK-beginner-vertical-slice — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #7 — build the beginner module vertical slice in the Rust
  authority path (town → oath → tower route → boss endpoint → typed events).
  User-locked constraints: TOML-first content; backend-only (Tauri UI deferred);
  verified by Rust tests + server API smoke on the existing `/command`→`/events`
  path.
- **Intake source:** none (forge ticket #7 pre-existed).
- **Classification / tier:** work pipeline, one shippable slice. Touches parser,
  engine, protocol, content, and a server smoke test — but they share the slice
  and the same files, so it is **one cohesive pipeline, not split**.
- **Forge recall (lessons/failures surfaced):**
  - AAR opened: `1a76f475-491d-4977-892f-1821c1187c61`. `knowledge-context`
    (Plan) wrote 10 surfacings (3 architecture-decisions, 5 prevention-rules, 2
    recent failures) clustered on JS→Rust porting + typed-boundary discipline.
  - The strongest signal is the **existing code**, which already embodies those
    rules: typed errors at the construction boundary (`WorldValidationError`, no
    `unwrap`/`expect` on input); a pure deterministic parser kept separate from
    the effectful engine; and — for the gate — **every new enum variant / state
    field is mutation surface** that needs a killing test (`MUT_MSI_MIN=100`).
- **Discovery — what already exists (so the slice is behavior, not geography):**
  - **Content is already TOML-first.** `load_beginner_world` `include_str!`s
    `modules/beginner/{module,rooms,world}.toml` and validates them. (Earlier
    pre-flight wrongly said content was code-defined — corrected.)
  - **The full route geography exists (ticket #6):** 8 rooms forming
    `hollowmere_square` (start, town) → `north_gate` → `ashen_road` →
    `tower_foot` → `tower_landing` → `bell_eater_roost` (boss), plus
    `candle_shop` and `bell_frame`. Narrative is pre-seeded: a silent bell whose
    clapper the Bell-Eater boss has stolen to its roost.
  - **Engine today:** `Engine::try_new/snapshot/tick/handle_command`. Commands:
    `Empty | Help | Look{target} | Move(Direction) | Unknown`. Events:
    `GameEventKind = LogMessage | Tick | RoomEntered` (only 3). `EventChannel`
    already has `Oath`; `OutputComponent` already has `OathCard`. **No** oath
    state, **no** boss resolution, **no** swear command yet — that is the #7 gap.
  - **Server path exists:** `POST /command` → `handle_command` → broadcast;
    `GET /events`/`/events/json`/`/events/html` (SSE). REQ-005 mostly needs a
    smoke test, not new transport.
- **Ticket:** forge #7 `c1937d4e-2367-4884-a6e5-bcc7023f6a57`; local doc
  `docs/planning/tickets/open/TICKET-7-build-beginner-module-vertical-slice-in-rust.md`.
- **EARS requirements reviewed:** REQ-001..005 carried from the ticket and
  sharpened with verification methods; added REQ-006 (full gate green: ≥94% cov,
  100% MSI) and REQ-007 (TOML-first content loads + validates, dangling ref
  rejected). See spec.

### Open questions handed to Phase 2 — Design
- **Event representation:** new `GameEventKind` variants (`OathSworn` /
  `OathFulfilled` / boss-resolved) vs. `LogMessage` on the `Oath`/`Combat`
  channel with the `OathCard`/`CombatMessage` component. Minimize mutation
  surface either way.
- **Oath state shape:** enum (`NotSworn`/`Active`/`Fulfilled`/`Broken`) on
  `GameState`, plus how the oath is identified (id from TOML).
- **Boss modeling:** keep the Bell-Eater as room narrative + a scripted resolve
  command, or model it as a placed `Entity` in `bell_eater_roost`.
- **Route gating:** is "while oath active" a real precondition (e.g. boss resolve
  refused until sworn) or just the asserted slice flow? Keep placeholder-minimal.
- **Oath content schema:** the TOML shape for the oath (id/title/promise…) and
  where it lives (`world.toml` vs a new `oaths.toml`), parsed by the loader.

## Phase 2 — Design

### Approach / architecture
Add a thin **oath lifecycle + boss-endpoint resolution** layer over the existing
engine, reusing the established patterns (typed domain events, typed validation
errors at the construction boundary, pure parser, deterministic engine, state→
view snapshot mapping). The route geography and TOML loader already exist, so the
work is: two new commands, two new typed events, one oath-state field, one oath
content registry + designation, and a server smoke test. No new transport, no UI.

**Resolved open questions:**
- **Event representation → new typed `GameEventKind` variants** (`OathSworn`,
  `OathFulfilled`), not stringly-typed `LogMessage`. This is the documented
  "domain events are the source of truth" direction (`protocol-and-output.md`
  lists `OathSworn`/`OathFulfilled`), and gives REQ-002/004 concrete, matchable
  events. Each action also emits a human-readable `LogMessage` (existing
  machinery) so the loop still reads like a MUD. `Oath` channel + `OathCard`
  component already exist. **`OathBroken` is NOT added** — no break path exists in
  this slice, and an unconstructed variant is an uncoverable mutant (hurts MSI).
- **Oath state shape →** `GameState.oath: Option<OathProgress>` (`None` = not
  sworn, avoids a `NotSworn` variant). `OathProgress { oath_id, title, status }`
  with `status: OathStatus { Sworn, Fulfilled }`. Both statuses are reachable
  (swear→Sworn, confront→Fulfilled), so both are killable. `OathStatus` lives in
  `oathstar-protocol` and is reused by core state, the snapshot, and is `Copy`.
- **Boss modeling → a placed `Entity` identified by the `"boss"` role**, not a new
  world field. Reuses the existing `RoomDefinition.entities` placement +
  `Entity.roles` model (whose validation already exists), so no new
  `WorldValidationError` for the boss. `confront` scans the current room for an
  entity whose def carries the `"boss"` role.
- **Route gating → movement stays ungated; the OATH gates the boss.** Geography
  is open (REQ-003 = drive the route with the oath active). `confront` is the
  load-bearing interlock: boss present **and** oath active (`Sworn`) → resolve +
  fulfill; boss present + no oath → refused; boss present + already `Fulfilled` →
  refused; no boss here → refused. This makes the oath real (not cosmetic) and
  gives four observable, mutation-killable branches.
- **Oath TOML schema →** a new `[[oaths]]` array in `world.toml`
  (`id`/`title`/`description`) indexed into `WorldDefinition.oaths`, plus a
  module-level `oath_id` (in `module.toml`, like `start_room_id`) naming the
  beginner oath. `swear` is module-global (swears the designated oath); NPC-
  offered oaths are a future refinement. `validate()` rejects a designated
  `oath_id` absent from the registry (new `WorldValidationError::OathMissing`) —
  this is REQ-007's dangling-ref check.

**Command flow (engine):**
- `swear`/`vow` → `swear()`: if already sworn → refused ("already sworn"); if the
  module designates no oath → refused ("no oath to swear"); else record
  `OathProgress{Sworn}`, emit a Narrative `LogMessage` + `OathSworn{oath_id,title}`
  on the `Oath` channel, accepted. The oath def lookup uses an
  `expect`-invariant (validate guarantees `oath_id ∈ oaths`), mirroring the
  existing `current_room()` precedent — preferred over an unreachable defensive
  branch that would be an uncoverable mutant.
- `confront`/`challenge` → `confront()`: four branches above; success emits a
  Combat `LogMessage` (authored outcome referencing `boss.name`) +
  `OathFulfilled{oath_id}`, sets status `Fulfilled`. Deterministic (no RNG).
- `Help` text extended to list `swear` and `confront`.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-protocol/src/lib.rs` | Add `GameEventKind::OathSworn{oath_id,title}` + `OathFulfilled{oath_id}`; add `OathStatus{Sworn,Fulfilled}` (Copy, serde); add `OathSnapshot{oath_id,title,status}`; add `oath: Option<OathSnapshot>` to `GameSnapshot` (serde default + skip_if none) |
| 2 | `crates/oathstar-core/src/lib.rs` | Add `OathDefinition{id,title,description}`; add `oaths: BTreeMap<String,OathDefinition>` + `oath_id: Option<String>` to `WorldDefinition` (serde default); add `WorldValidationError::OathMissing{oath_id}` + Display; `validate()` checks designated oath exists; add `OathProgress{oath_id,title,status}` + `oath: Option<OathProgress>` on `GameState`; `try_new` inits `oath:None`; `handle_command` Swear/Confront arms; `swear()`/`confront()` helpers; `snapshot()` maps oath; Help text +swear/confront; update test helpers + new tests |
| 3 | `crates/oathstar-core/src/command.rs` | Add `Command::Swear` + `Command::Confront`; parse `swear`/`vow` and `confront`/`challenge` (bare, strict arity → else `Unknown`); parser tests |
| 4 | `crates/oathstar-content/src/lib.rs` | `ModuleToml.oath_id: Option<String>` (default); `WorldToml.oaths: Vec<OathDefinition>` (default); `load_world_from_toml` builds `oaths` via `index_by_id` + sets `oath_id`; content tests |
| 5 | `crates/oathstar-server/src/main.rs` | `render_event_html` arms for `OathSworn`/`OathFulfilled`; full-slice smoke test over `command()`; render-oath-events test |
| 6 | `modules/beginner/module.toml` | Add `oath_id = "hollow_bell"` |
| 7 | `modules/beginner/world.toml` | Add `[[oaths]]` hollow_bell; `[[entities]]` bell_eater (roles=["boss","combatant"], inventory=["bell_clapper"]); `[[items]]` bell_clapper |
| 8 | `modules/beginner/rooms.toml` | Place `entities = ["bell_eater"]` in `bell_eater_roost`; `entities = ["mara"]` in `candle_shop` (flavor; array key before the `[rooms.exits]` sub-table) |

### Regression Test Plan
| # | Test (crate) | Proves |
|---|---|---|
| T1 | core `swear_sets_oath_active_and_emits_oath_sworn` (synthetic oath world) — accepted; snapshot.oath = Some{Sworn,id,title}; events contain `OathSworn` on `Oath` channel | REQ-002 |
| T2 | core `swear_twice_is_refused` — 2nd swear not accepted ("already sworn") | REQ-002 (branch) |
| T3 | core `swear_with_no_designated_oath_is_refused` — world w/o `oath_id` → refused | REQ-002 (branch) |
| T4 | command `swear_and_vow_parse_to_swear`; `swear_with_trailing_is_unknown` | REQ-002 (parser) |
| T5 | core `confront_fulfills_oath_and_emits_oath_fulfilled` (sworn + boss in room) — accepted; Combat `LogMessage`; `OathFulfilled`; snapshot.oath=Fulfilled | REQ-004 |
| T6 | core `confront_without_boss_is_refused` — "nothing to confront", not accepted | REQ-004 (branch) |
| T7 | core `confront_without_oath_is_refused` — boss present, oath None → refused, oath stays None | REQ-004 (branch) |
| T8 | core `confront_when_already_fulfilled_is_refused` — confront twice → 2nd refused | REQ-004 (branch) |
| T9 | command `confront_and_challenge_parse_to_confront`; `confront_with_trailing_is_unknown` | REQ-004 (parser) |
| T10 | core `validate_rejects_missing_designated_oath` (`OathMissing`) + `validate_accepts_world_with_valid_oath` + Display names offender | REQ-007 (core validate) |
| T11 | content `beginner_world_has_oath_and_boss` — oaths has `hollow_bell`; `oath_id==Some`; `bell_eater` entity has `boss` role + owns `bell_clapper`; `bell_eater_roost` places `bell_eater` | REQ-007 / content |
| T12 | content `load_rejects_missing_oath_reference` (module `oath_id` not in registry → OathMissing) + `load_rejects_duplicate_oath_id` (index_by_id "oath") | REQ-007 (dangling/dup) |
| T13 | core `help_lists_swear_and_confront` | REQ-002/004 (help) |
| T14 | server `render_event_html_renders_oath_events` (OathSworn + OathFulfilled → oath articles, ids/title escaped) | REQ-005 (render arms) |
| T15 | server `beginner_slice_runs_through_command_path` — drive look→swear→n,n,n,u,u→confront via `command()`; assert each accepted, `OathSworn` then `OathFulfilled` in responses, final snapshot.current_room_id=="bell_eater_roost" + oath Fulfilled | REQ-001, REQ-003, REQ-004, REQ-005 |
| T16 | `bin/gate.sh` GREEN [full] — ≥94% line cov + 100% MSI over new code | REQ-006 |
| — | Regression: existing core movement/look/parser, content load, server broadcast/escape tests stay green | guards existing behavior |

**Uncoverable path:** the oath-def lookup in `swear()` uses an `expect`-invariant
(validate guarantees `oath_id ∈ oaths`) — same gate-passing precedent as
`current_room()`. No genuinely uncoverable branches are introduced; the no-oath
and no-boss branches are all reachable with crafted synthetic worlds.

### Risks / decisions (reversible-but-load-bearing)
- **Oath gates the boss** (confront refused unless oath active). Makes the systems
  interlock and gives testable branches; if it feels artificial later, the gate is
  a single condition to relax.
- **`swear` is module-global** (no giver/location). Simplest placeholder; NPC-
  offered oaths (e.g. Mara) are a future ticket.
- **Boss = `"boss"` role on a placed entity** (not a new world field). Reuses the
  entity model; if multiple bosses per room ever matter, `confront` would need a
  target arg.
- **Adding fields to `WorldDefinition`/`GameSnapshot`/`GameState`** forces literal
  updates at the known sites (loader + 3 core test helpers + `snapshot`/`try_new`)
  — accounted for in the manifest; `#[serde(default)]` keeps wire formats
  backward-compatible.
- **`OathStatus` placed in `oathstar-protocol`** (reused by core state + snapshot
  + events) to avoid a duplicate enum across the crate boundary.

## Phase 3 — Implement
- **Built (to the 8-file manifest):**
  - protocol: `GameEventKind::OathSworn`/`OathFulfilled`; `OathStatus{Sworn,Fulfilled}`
    (Copy); `OathSnapshot`; `GameSnapshot.oath`.
  - core: `OathDefinition`; `WorldDefinition.oaths`+`oath_id`;
    `WorldValidationError::OathMissing` + Display + `validate()` check;
    `OathProgress` + `GameState.oath`; `try_new` inits `oath:None`;
    `handle_command` Swear/Confront arms; `swear()`/`confront()`; `snapshot()`
    oath mapping; Help text; 3 test helpers updated for the new fields.
  - command.rs: `Command::Swear`/`Confront`; parse `swear`|`vow`,
    `confront`|`challenge` (bare, strict arity → trailing = `Unknown`).
  - content: `ModuleToml.oath_id`, `WorldToml.oaths`; loader indexes oaths via
    `index_by_id` + sets `oath_id`.
  - server: `render_event_html` arms for OathSworn/OathFulfilled.
  - modules/beginner: module.toml `oath_id="hollow_bell"`; world.toml `[[oaths]]`
    hollow_bell + `bell_eater` entity (roles `boss`/`combatant`, owns
    `bell_clapper`) + `bell_clapper` item; rooms.toml placed `bell_eater` in the
    roost + `mara` in the candle shop.
- **Compile/check:** `cargo check --workspace --tests` clean; `cargo clippy
  --workspace --all-targets` clean (strict pedantic+nursery+restriction);
  existing suite green (content 8, core 49, server 9, storage 20 = 86). Real
  beginner world loads + validates.
- **Deviations from design (+ reason):**
  - `confront()` uses a borrow-safe two-phase match (flip status under a scoped
    `&mut`, copy the id out, then emit) — needs **no** `expect`, cleaner than the
    design's expect hint. `swear()` keeps the single documented expect-invariant
    for the oath-def lookup (validate guarantees presence; mirrors
    `current_room()`).
  - Fixed one clippy nursery lint (`too_long_first_doc_paragraph`) on the
    `OathDefinition` doc during implementation.
  - **No new regression tests written** (phase contract) — T1–T15 are authored +
    run in Phase 4 (Validate); the new swear/confront/validate/render branches are
    intentionally untested until then.

## Inspect (Phase 3.5)
- **Lenses run** (4 parallel independent critics over `git diff`): correctness/AC,
  borrow-safety/determinism/idiom, mutation-surface/100% MSI readiness, and
  serde wire-format/back-compat/content-integrity.
- **Findings:**

  | # | Sev | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | MED | REQ-001: no typed room event on game start — only via `look` (core: `try_new`/`handle_command`) | **REAL** | Added `Engine::begin()` opening-scene API (`RoomEntered{start}` + describe), reusing the movement room path. Phase-4 REQ-001 test asserts it. |
  | 2 | LOW | Module lore ("Hollowmere/stolen clapper") hardcoded in core `confront()` outcome (core lib ~690) | **REAL** | Outcome text made generic: `"You overcome {boss_name}, and your oath is fulfilled."` (uses only content `boss_name`; module flavor stays in room/oath content). |
  | 3 | HIGH* | Mutation gap: `role == "boss"` and `.find().any()` mutants not killed by an empty-room test — need a **non-boss-entity-in-room** confront test (*test-plan gap, not a code defect) | **REAL** | Recorded as a Phase-4 requirement (new test T17). |
  | 4 | LOW | `swear()` `.expect()` reachability (core lib ~622) | **REJECTED** | Provably unreachable: `validate()` guarantees `oath_id ∈ oaths`, `try_new` validates, `self.world` never mutated. All 4 critics confirmed; mirrors `current_room()`. |
  | 5 | MED | `oath_id` is camelCase in `OathSnapshot` but snake in the flattened `OathSworn`/`OathFulfilled` events (protocol) | **REJECTED (code)** | Mirrors the shipped `RoomEntered.room_id` vs `RoomSnapshot.id` convention; serde-forced (rename_all doesn't cross `flatten`). Changing it would break the existing event contract. Documented as a known wire split for the UI ticket. |
  | 6 | LOW | confront/swear `.clone()`s | **REJECTED** | Load-bearing (the `String` is moved into the event) / house style; clippy clean. |
  | 7 | LOW | Back-compat of added fields incl. `GameState.oath` save shape | **REJECTED** | All new fields `#[serde(default)]`; critic verified old saves load (`oath`→`None`); `GameSnapshot.oath` also `skip_serializing_if`. Sound. |

  Fixes verified: `cargo check --workspace --tests` + `cargo clippy --workspace
  --all-targets` clean; existing 86 tests still green.

- **Phase-4 mutation/coverage requirements (carry-forward from critics — needed
  for 100% MSI + ≥94% line):**
  - **Assert message TEXT, not just `!accepted`**, on every refusal so the
    string-swap mutants die (each refusal shares `accepted=false`): swear
    "already sworn" / "no oath"; confront "nothing…to confront" / "sworn no oath"
    / "already broken".
  - **swear success (T1):** assert `snapshot.oath = {oath_id, title, status:Sworn}`
    (all three fields, distinctive values); `OathSworn{oath_id,title}` on
    **`EventChannel::Oath`**; AND a **Narrative** `LogMessage` whose text contains
    the oath title **and** description. Use a synthetic oath with distinctive
    strings.
  - **confront success (T5):** assert a **Combat**/`CombatMessage` `LogMessage`
    whose text contains the boss **name**; `OathFulfilled{oath_id==sworn id}` on
    `EventChannel::Oath`; AND `snapshot.oath.status == Fulfilled`.
  - **NEW T17 — non-boss entity present:** confront in a room holding an entity
    *without* the `"boss"` role → refused "nothing to confront" (kills
    `role=="boss"`→`!=` and `.find(..).any(..)`→`true`). An empty room alone does
    not kill these.
  - **parser (T4/T9):** assert BOTH aliases (`swear`|`vow`, `confront`|`challenge`)
    and the **exact** `Unknown{input:"…"}` payload for the trailing-token cases.
  - **validate (T10):** both `OathMissing` (variant + Display contains "designated
    oath" + the offending id) **and** an accepts-valid-oath `Ok` case.
  - **content (T11/T12):** assert values (`oath_id==Some("hollow_bell")`, `oaths`
    key, boss roles/inventory, room placements); duplicate-oath-id error contains
    "duplicate" + "oath"; missing-designated-oath error contains "designated oath".
  - **render (T14):** both arms — assert substrings (`message-oath`,
    `data-oath-id="…"`, `Sworn: <title>` / `Oath fulfilled`) and HTML-escaping
    (feed an id/title containing `<`/`&`).
  - **help (T13):** assert the text contains **"swear"** and **"confront"** (the
    old "look, north" substring is unchanged, so it won't catch a reverted help).
  - **NEW T18 (line coverage):** a content fixture that **omits** `oath_id` and
    `[[oaths]]` → loads with `oath_id None` / empty `oaths` (exercises the
    `#[serde(default)]` None/empty paths).
  - **begin() (T-REQ-001):** assert `begin()` on the beginner world returns
    `RoomEntered{room_id:"hollowmere_square"}` + a `RoomHeader` containing
    "Hollowmere Square".
  - The smoke test (T15) is for line coverage of the wired path; it does **not**
    substitute for the targeted MSI assertions above.

## Phase 4 — Validate
- **Tests added (+23 Rust):**
  - core (`oathstar-core/src/lib.rs`, synthetic `oath_world`): swear success +
    refusals (already-sworn, no-designated-oath); confront success + 4 refusal
    branches incl. **T17 non-boss entity present**; `validate` OathMissing +
    accepts-valid + Display; `begin()` start-room event; help lists swear/confront.
  - parser (`command.rs`): swear|vow + confront|challenge aliases; trailing →
    exact `Unknown` payload.
  - content (`oathstar-content`): real beginner world has oath+boss+placement
    (value asserts); dangling oath → "designated oath"; duplicate oath id →
    "oath"; no-oath module fixture (serde-default None/empty).
  - server (`oathstar-server`): render OathSworn/OathFulfilled + HTML escaping;
    `begin()` on the real world (REQ-001); **full-slice smoke** look→swear→
    n,n,n,u,u→confront over `command()`.
- `cargo test --workspace`: **109 passed, 0 failed** (core 65, content 12,
  server 12, storage 20; protocol has no unit tests).
- `node --test tests/*.test.js`: **4 passed, 0 failed**.
- `bin/gate.sh` (FULL): **GREEN — 17/17.** Mutation **71 caught / 0 missed →
  MSI 100.0%** (floor 100); Rust coverage **97.60% lines** (floor 94); JS
  coverage **75.19%** (floor 75). Required one `cargo fmt` (formatting only);
  gate receipt written to `.git/oathstar-gate-receipt`.
- **AC coverage:** REQ-001 (begin start-room event + smoke look), REQ-002
  (swear), REQ-003 (route smoke), REQ-004 (confront branches), REQ-005 (server
  smoke + render), REQ-006 (gate green), REQ-007 (content load/validate) — all
  verified.
- Pre-existing exclusions: none (no pre-existing failures encountered).

## Phase 5 — Complete
- Docs updated:
- Forge capture (aar/failures/rules/decisions):
- Ticket closed:
- Archived:
