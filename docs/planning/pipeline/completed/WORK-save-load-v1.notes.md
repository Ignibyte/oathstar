# WORK-save-load-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #28 — Save & Load v1, the carried-forward persistence
  gap (#22 R5; Decisions 044/045 revisit triggers): persist the complete
  session through the existing hardened storage layer, with an
  input-hardened engine load surface, atomic server endpoints, and the
  stubbed client buttons wired. AUTO-APPROVE; STOP BEFORE `/commit`;
  validate = workspace tests + node tests + `npm run build` +
  `./bin/gate.sh --fast` (FULL gate/commit owner-gated).
- **Intake source:** none — the gap is documented across the #22/#26/#27
  pipeline notes (this is the first non-intake, non-pre-authored ticket:
  the planner minted forge `27ec2cb2-b62d-4b26-aa1b-74c6e176c5dc` (#28) and
  created the local ticket doc from the template).
- **Classification / tier:** Work pipeline, **one shippable slice** — a
  payload type, an engine surface, two endpoints, two button handlers. The
  storage layer already exists (zero new persistence code expected).
- **Base verified:** branch `codex/oathstar-ticket-28-save-load` off `main`
  @ `9807418` (#27 — everything merged). Worktree clean except the
  Codex-owned strays (untouched) and this pipeline's docs. No active
  pipeline; forge up.
- **Forge recall (lessons/failures surfaced):**
  - AAR opened: `53b58ef7-25f4-411c-a5e5-91eef88106e3`; Plan-phase
    `knowledge-context` logged (13 surfacings).
  - Standing rules that bind here:
    **PR-claude-fixture-distinguishable-transitions-001** — every refusal
    path (missing/malformed/version/validation/slot) stages both arms, and
    the load-restores test must MUTATE before loading (a no-op load passes
    vacuously otherwise — exactly this rule's class);
    **PR-claude-package-scope-mutation-001** — engine save/load killers in
    core, endpoint killers in the server crate;
    **PR-claude-expect-invariants-over-unreachable-arms-001** — but note
    the INVERSE applies on the load path: file content is INPUT, so typed
    errors (not expects) are mandatory there (§14).
  - ADs: pulse-rides-tick `f20f3ff4` (the engine-swap-under-lock seam),
    inventory `c5c90992`, reward-loop `2aadcccb` (what the world-mutation
    payload must capture).
- **Current-code anchor map (from the #22–#27 sessions + pre-flight):**
  - `crates/oathstar-storage/src/lib.rs`: **complete and hardened** —
    `validate_save_slot_name` (traversal/reserved/length), `SaveStore`
    trait (`write_json<T: Serialize>` / `read_json<T: DeserializeOwned>`),
    `FileSaveStore { root }` with symlink defense + dir creation + pretty
    JSON + context-rich `anyhow` errors. Zero callers. The crate speaks
    `anyhow::Result` — the engine surface will want typed errors at ITS
    boundary (design decides the split).
  - `crates/oathstar-core/src/lib.rs`: `Engine { world, state,
    next_event_id }` — all private; fully `Serialize`/`Deserialize`
    (`WorldDefinition`, `GameState` incl. `CombatState`'s plain fields —
    a v1 save always writes complete structs, so the no-serde-default
    concern from #24 is moot for same-version round-trips);
    `Engine::try_new(world)` — the validation boundary loading must
    re-cross; `snapshot()` — the byte-identity witness.
  - `crates/oathstar-server/src/main.rs`: `AppState { engine:
    Arc<Mutex<Engine>>, events, opening }`; `main` builds the engine +
    captures `opening` from `begin()`; routes incl. POST `/command`. New:
    POST `/save` + `/load`. The swap: `*engine_guard = loaded` — pulses
    and commands serialize through the same lock (#24 seam).
  - JS: `src/client-app.js` — `el.saveButton`/`el.loadButton` EXIST with a
    stub handler ("Save/load isn't wired into this shell yet"); `el.newButton`
    re-renders the session. Wiring = two fetch handlers + `refreshState()`.
  - Docs: `ui-design.md` mentions "save/load UI" as out-of-scope-then;
    Decisions 040/044/045 carry the revisit triggers this closes.
- **EARS requirements reviewed:** REQ-001..008 (verbatim in the spec).
  001 save completeness/non-mutation; 002 byte-identical restore; 003 the
  four refusal arms; 004 slot validation; 005 atomic swap; 006 client
  wiring; 007 the played loop; 008 preservation.

### Open design questions (for Phase 2 — Planner does NOT decide these)
1. **Payload + ownership.** `SaveData { version: u32, world, state,
   next_event_id }` — where does the type live? Core (engine constructs and
   consumes it; storage stays generic) seems right; design confirms, and
   names the format-version const (`SAVE_FORMAT_VERSION: u32 = 1`).
2. **Engine surface shape.** `Engine::save_data() -> SaveData` (clones) +
   `Engine::from_save(SaveData) -> Result<Engine, LoadError>` — or restore
   IN PLACE (`&mut self`)? From-save constructing a NEW engine reuses
   `try_new`'s validation verbatim and makes the server swap trivial
   (`*guard = new_engine`); in-place restore avoids... nothing much. The
   typed error: a new `LoadError` enum (version mismatch / invalid world
   (wrapping `WorldValidationError`) / engine-level issues) vs reusing
   `WorldValidationError` + a wrapper at the server. Design decides; §14
   typed-error rule binds.
3. **Where serde/file errors live.** Storage returns `anyhow`; the server
   endpoint maps storage failures (missing/malformed file) + engine
   `LoadError` into an HTTP response shape. What do the endpoints return —
   a small JSON `{ ok, error? }` (new response type — protocol crate or
   server-local serde struct?) and which status codes. Keep it
   server-local if possible (no protocol-crate churn for an endpoint
   envelope).
4. **State-integrity check on load** (beyond world validation):
   `current_room_id` must exist in the loaded world, `combat.enemy_id`
   should resolve, pack ids resolve, etc. — `try_new` validates the WORLD
   but state-vs-world coherence is new territory. v1 line: which checks
   are load-blocking vs tolerated? (Recommend: a focused
   `validate_state_against_world` for the invariants the engine `expect`s
   on — `current_room_id` is the panic-relevant one (the `current_room()`
   invariant!); pack/inventory orphans are tolerated by total fallbacks
   already; combat enemy lookups… `enemy_xp_reward` is total but
   `drop_enemy_inventory` EXPECTS the registry entry — audit which loaded
   states could panic and gate exactly those.) THIS IS THE CRITICAL §14
   QUESTION — a crafted save must not be able to panic the engine later.
5. **Mid-combat saves.** CombatState round-trips completely (pulse anchor
   is tick-relative and `tick` is saved — cadence resumes coherently).
   Recommend: persist (simplest honest); design confirms + documents that
   the pulse loop simply continues post-load.
6. **The opening seed + SSE after load.** `AppState.opening` is captured at
   startup; after a load it replays the ORIGINAL world's opening to NEW
   subscribers (stale but harmless?) and `next_event_id` continuity
   prevents id regressions for EXISTING subscribers. Options: leave the
   seed (documented quirk), regenerate from the loaded engine
   (`begin()` mutates — no), or emit a load-time `RoomEntered` +
   description burst (the defeat-arrival pattern) so feeds narrate the
   resumption and the stale seed matters less. Design picks.
7. **Load/save feedback in the feed.** A `LogMessage`/system line ("Game
   saved." / "Game loaded.") emitted by the server? By the engine? Or
   client-side `appendLine` only (the buttons' current stub pattern)?
   Server-authoritative feedback via the broadcast keeps the Datastar feed
   coherent; design picks the exact lines.
8. **Save root configuration.** `OATHSTAR_SAVE_DIR` env default
   (`./saves`? platform dirs later) — mirrors `OATHSTAR_ADDR` precedent.
   Gitignore the default dir.
9. **The default slot name.** `"quicksave"`? `"slot1"`? One const shared
   server-side; the client sends none (v1 endpoints take an optional slot
   defaulting to it — or no body at all).
10. **`From<SaveSlotError>` / error-source threading** so REQ-004 refusals
    surface the slot-validation message verbatim through the endpoint.

## Phase 2 — Design

### Approach / architecture (the 10 open questions, resolved)

The keystone is the Q4 audit. **Three engine expect-invariants are
STATE-driven** — a crafted save could violate them and panic the engine
later — and exactly those are gated at load with typed errors:

| invariant | violating state | panics at |
|---|---|---|
| `current_room()` | `current_room_id` ∉ `world.rooms` | the first snapshot/command |
| `drop_enemy_inventory`'s registry expect | `combat.enemy_id` ∉ `world.entities` (combat `Some`) | the eventual victory |
| `confront`'s oath lookup expect | `oath.oath_id` ∉ `world.oaths` (oath `Some`) | fulfillment |

Everything else loaded state can carry is **tolerated and safe by
construction** (documented): pack/inventory orphan ids (total snapshot
fallbacks; `perceive` filter-maps unknown placements; the player `drop`
verb has no expect), `discovered_rooms` strays (membership checks only),
`offered_oath_id` mismatches (comparison only), and `CombatState` numeric
weirdness (hp/xp already saturate per #26; a 0 `pulse_rate` just pulses
every tick; a past `next_pulse_at` is due immediately) — odd, never
unsound.

**Audit addendum — overflow sites (the #26 xp class, found by the full
expect/arithmetic sweep):** three unsaturated `u64` additions operate on
loaded values and would panic a debug build on a crafted save
(`tick: u64::MAX` / `next_event_id: u64::MAX`):
`state.tick += 1` (lib.rs:989), `next_pulse_at = tick + pulse_rate`
(lib.rs:1019 and 1837), `next_event_id += 1` (lib.rs:2365). Fix is the
established precedent — `saturating_add` at the site (also hardens
in-game accumulation), NOT load-gating numeric ranges.

1. **Payload + version (Q1).** In CORE:
   `SaveData { version: u32, world: WorldDefinition, state: GameState,
   next_event_id: u64 }` (serde) + `pub const SAVE_FORMAT_VERSION: u32 = 1`.
   The engine constructs and consumes it; storage stays generic.
2. **Engine surface (Q2): construct-new.**
   `Engine::save_data(&self) -> SaveData` (clones; never mutates) and
   `Engine::from_save(SaveData) -> Result<Engine, LoadError>` — version
   check → `world.validate()` (the same try_new-grade boundary) → the three
   state-coherence gates → construct. A new engine makes the server swap a
   one-line `*guard = engine` and reuses validation verbatim; in-place
   restore buys nothing.
   `LoadError { VersionMismatch { found, supported },
   InvalidWorld(WorldValidationError), StateIncoherent { what: String } }`
   (+ Display/Error impls; `what` names the offender, e.g.
   "current room 'ghost' is not in the world").
3. **Errors at the seams (Q3/Q10).** Storage keeps `anyhow` (file
   missing/malformed parse surface as its context-rich errors); the server
   endpoint maps ANY failure (slot validation, file, serde, `LoadError`)
   into a server-local `SaveLoadResponse { ok, error: Option<String> }` at
   HTTP 200 — the in-band-refusal convention `/command`'s `accepted: false`
   established. No protocol-crate changes.
4. **The audit is the design (Q4).** Gated: the three rows above.
   Tolerated: the list above, each with its safety reason. Inspect re-runs
   this audit adversarially.
5. **Mid-combat saves persist (Q5).** `CombatState` round-trips completely;
   `next_pulse_at` is an absolute tick and `tick` is saved, so cadence
   resumes exactly — the restored engine's next pulse equals the unsaved
   twin's (tested).
6. **The opening seed is a non-issue (Q6), documented.** Saves are
   same-module: a reconnecting subscriber's seed replays `begin()`'s
   opening for the same world definition — byte-identical text to what a
   fresh session shows. `next_event_id` persistence keeps post-load event
   ids from colliding with the loaded session's own history (loading an
   OLDER save still rewinds ids relative to the live feed — verified
   harmless: both SSE consumers key on event type, not id, and the
   datastar opening-skip only compares against the boot opening's ids).
7. **Feedback is client-side (Q7), the existing button idiom.** Save: the
   handler `appendLine`s "Game saved." (or the error). Load: clear the
   feed (`el.log.replaceChildren()`, the New-button pattern — the loaded
   session's feed history is not persisted, so a stale log would lie),
   `appendLine` "Game loaded.", `refreshState()`. No server broadcast, no
   engine feed lines — the server returns `{ ok }`.
8. **Save root (Q8):** `OATHSTAR_SAVE_DIR` env, default `saves`
   (the `OATHSTAR_ADDR` precedent); `.gitignore` gains `saves/`.
9. **Slot (Q9):** `const DEFAULT_SAVE_SLOT: &str = "quicksave"` server-side;
   the endpoints accept an OPTIONAL JSON body
   `{ slot?: string }` defaulting to it — so REQ-004's invalid-slot refusal
   is exercisable end-to-end (the storage layer's `path_for` validation
   surfaces verbatim through the response error). The v1 client sends no
   body.
10. **Endpoints + swap (REQ-005).** POST `/save`: lock → `save_data()` →
    drop → `write_json` (file IO outside the lock — the engine is never
    held across disk writes). POST `/load`: `read_json::<SaveData>` →
    `Engine::from_save` → only on success lock → `*guard = engine` → drop;
    any failure leaves the running session untouched (the new engine is
    fully built BEFORE the lock is taken — the swap itself is a move).
    The tick loop holds the same `Arc`, so pulses drive the restored
    session immediately.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/lib.rs` | `SAVE_FORMAT_VERSION`; `SaveData` (serde, doc-commented); `LoadError` (+ Display + `std::error::Error`); `Engine::save_data` (pub); `Engine::from_save` (pub — version gate → `world.validate()` → the three coherence gates → construct); saturate the three loaded-`u64` addition sites (tick increment, both `next_pulse_at` computations, event-id allocation). |
| 2 | `crates/oathstar-server/Cargo.toml` | `oathstar-storage = { path = … }` — the layer's first consumer. |
| 3 | `crates/oathstar-server/src/main.rs` | `AppState.saves: FileSaveStore` (root from `OATHSTAR_SAVE_DIR`, default `saves`); `DEFAULT_SAVE_SLOT`; server-local `SaveLoadRequest { slot: Option<String> }` / `SaveLoadResponse { ok, error }`; POST `/save` + `/load` handlers (IO outside the lock; swap in-lock); `test_app_state` gains a per-test temp-dir store. |
| 4 | `src/client-app.js` | Replace the `unavailable` stub: `saveGame()` (POST `/save` → feedback line) + `loadGame()` (POST `/load` → clear log, "Game loaded.", `refreshState()`); offline catch like `runCommand`. |
| 5 | `.gitignore` | `saves/`. |
| 6 | docs (Phase 5): `docs/decisions.md` (Decision 046), `docs/ui-design.md` (buttons live), `docs/mechanics-and-systems.md` (persistence note if a section fits). |

**No changes**: `oathstar-protocol`, `oathstar-datastar`,
`oathstar-storage` (consumed as-is), `command.rs`, content TOML, CSS/HTML.

### Regression Test Plan
≥1 per EARS REQ; refusal paths staged both-arms; byte-identity via
`serde_json::to_value(snapshot)` equality.

| # | Test | Proves |
|---|---|---|
| P1 | `save_data` carries the mutated session: play the spoils fixture (victory → xp + fang taken), then assert the payload by value — no stray placement, fang in `state.pack`, `player.xp`, `version == 1`, `next_event_id` — AND the running engine's snapshot is identical before/after the call | REQ-001 |
| P2 | byte-identical restore: mutate → `save_data` → `from_save` → restored snapshot Value == at-save snapshot Value; then both the restored engine and an unsaved twin tick forward and produce identical pulse bursts (mid-combat variant: save mid-fight with a queued action + armed guard charge → cadence and Phase-2 resolution identical post-load) | REQ-002 |
| P3 | refusals, each both-arms: `version: 2` → `VersionMismatch { found: 2, supported: 1 }` by value; corrupt world (dangling exit) → `InvalidWorld(DanglingExit…)`; bogus `current_room_id` / `combat.enemy_id` / `oath.oath_id` → `StateIncoherent` naming each offender; the matching VALID payloads load fine | REQ-003 |
| P4 | tolerance line: a payload with an orphan pack id + stray `discovered_rooms` entry loads OK and the snapshot falls back safely (documents the audit's tolerated class) | REQ-003 |
| P5 | overflow hardening: a loaded session at `tick: u64::MAX` / `next_event_id: u64::MAX` ticks and emits its first event without panicking (values saturate) — the #26 crafted-save class | REQ-003 |
| SV1 | server round-trip: POST `/save` → `ok:true` + the slot file exists under the test temp root; walk away (mutate); POST `/load` → `ok:true`; `/state` Value == the at-save `/state` Value | REQ-005/006 |
| SV2 | invalid slot end-to-end: POST `/save` `{ "slot": "../evil" }` → `ok:false` with the slot-validation message; nothing written outside/at all | REQ-004 |
| SV3 | missing-save load: POST `/load` (no file) → `ok:false`; `/state` unchanged (before == after) | REQ-003/005 |
| SV4 | the played loop: north ×2 → pulse-fight to victory (xp 5) → `take fang` → `/save` → drop the fang + walk south → `/load` → `/state` shows xp 5, the fang back in the pack, the player back at `ashen_road` | REQ-007 |
| SV5 | paused-time swap under fire: spawn the real tick loop, start a fight, `/save` mid-combat, keep fighting, `/load` → the restored fight's next pulses are exactly the saved session's; no panic, no partial state | REQ-005 |
| — | preservation: the untouched suites + `npm run build` | REQ-008 |

**JS rows: none** — the handlers are thin fetch glue (the Attack/Flee
carve-out); feedback strings live in the handlers (smoke-verified).
**Genuinely uncoverable:** none new.

### Risks / decisions
- **R1 — the coherence line is the contract:** three gated checks, the rest
  tolerated with named safety reasons. Any future engine `expect` whose
  input is state-reachable must add a gate here (noted for Decision 046 —
  this is the standing rule the ticket creates).
- **R2 — `ok:false` at HTTP 200** mirrors `/command`'s `accepted:false`
  in-band convention; status-code purists revisit with a public API ticket.
- **R3 — file IO outside the engine lock** (save serializes after drop;
  load builds before lock): pulses never stall on disk.
- **R4 — the stale-feed-on-load problem is solved client-side** (clear +
  "Game loaded.") because feed history is deliberately not persisted.
- **R5 — server tests use per-test temp save roots** injected through
  `test_app_state` (no shared dirs, no cleanup races).
- **R6 — mutation pins:** the version `==` gate (P3's found/supported by
  value), each coherence `contains_key` (P3 both-arms ×3), `from_save`/
  `save_data` fn-replaces (P1/P2's byte-identity), handler fn-replaces
  (SV1/SV3), `DEFAULT_SAVE_SLOT`/env fallback (SV1's file path assert).
- **R7 — save files are pretty JSON** (FileSaveStore's choice) — humans can
  read them; tampering is the THREAT MODEL the load gates exist for, not a
  thing to prevent.
- **R8 — the saturation fix is engine-wide, not load-only:** the four
  hardened addition sites also remove the (astronomically distant)
  in-game overflow; behavior at the saturation point is "time stands
  still / ids stop advancing", which is harmless and tested by P5. The
  audit's standing rule (R1) now reads: state-reachable expects AND
  state-reachable unchecked arithmetic both get a sweep on every ticket
  that grows `GameState`.

## Phase 3 — Implement
- Built (to the manifest; workspace `check --all-targets` + strict clippy +
  `node --check` + `npm run build` all green):
  - **core:** `SAVE_FORMAT_VERSION = 1`; `SaveData` (serde, doc-commented);
    `LoadError { VersionMismatch, InvalidWorld, StateIncoherent }` with
    Display (delegating to `WorldValidationError`'s) + `std::error::Error`;
    `Engine::save_data` (clone-out, `#[must_use]`); `Engine::from_save`
    (version gate → `world.validate()` → the three coherence gates, each
    naming its offender in `what`); the four `u64` sites saturated (tick
    increment — with the crafted-save comment, both `next_pulse_at`
    computations, event-id allocation in the `const fn event`).
  - **server:** `oathstar-storage` + `serde` deps; `DEFAULT_SAVE_SLOT =
    "quicksave"`; `AppState.saves: FileSaveStore` rooted at
    `OATHSTAR_SAVE_DIR` (default `saves`); `SaveLoadRequest { slot:
    Option }` / `SaveLoadResponse { ok, error }` with `success()`/`refusal()`
    constructors; POST `/save` (clone in-lock, write after drop) + `/load`
    (read + `from_save` before the lock, `*engine = loaded` in-lock);
    test-side `scratch_save_dir(tag)` (pid + tag namespacing) +
    `test_app_state_with_saves(tag)` with `test_app_state()` delegating.
  - **client:** stub replaced — shared `persistenceRequest(path, verb)`
    glue (in-band refusal + transport-failure lines), `saveGame()` ("Game
    saved."), `loadGame()` (clear log → "Game loaded." → `refreshState()`,
    the New-button pattern).
  - **.gitignore:** `saves/`.
- Deviations from design (+ reason):
  - The endpoints take a REQUIRED `Json<SaveLoadRequest>` body (the client
    sends `{}`) instead of an optional body: axum's strict extractor 4xxes
    a missing/malformed body rather than silently defaulting a malformed
    one to `quicksave` — stricter than designed, never looser.
  - anyhow refusals use the alternate format (`{error:#}`) so the context
    chain ("failed to parse …: expected value …") survives into the
    response `error` string.

## Inspect (Phase 3.5)
- Lenses run (3 critics, parallel): load-path security/coherence-audit
  completeness; server concurrency/semantics; client-UX/simplification.
- Findings:
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | HIGH | `combat.round + 1` / `round += 1` overflow (core lib.rs pulse + manual-attack sites): a crafted save with `round: u32::MAX` passes `from_save` (only `enemy_id` gated) and panics the next pulse — killing the spawned tick task (world clock stops; HTTP keeps answering). Proven by critic's scratch `#[should_panic]` test. The Phase-2 overflow sweep covered the payload's `u64`s but missed the nested `u32`. | REAL | Both sites → `saturating_add(1)` — the same pattern as the four designed sites. |
  | 2 | MED | `FileSaveStore::write_json` truncate-in-place (`fs::write`): crash/disk-full/double-click-save can destroy the previous good slot or interleave two writes into torn JSON. Pre-existing storage code, but #28 puts live traffic on it. | REAL | Temp-sibling + `fs::rename` (atomic within the dir) with symlink check on the temp path too and best-effort temp cleanup on rename failure. All 20 storage tests still green. |
  | 3 | MED | First-ever Load click renders `failed to read /…/quicksave.json: No such file or directory (os error 2)` into the feed — a crash-dump where the prototype said "No saved oath exists yet." | REAL | `/load` maps a NotFound anywhere in the anyhow chain to `no save exists in slot '<slot>' yet`; other errors keep the context chain. |
  | 4 | MED | `CombatState` doc ("mid-encounter state is never persisted") now FALSE — #28 serializes it on every mid-combat save, at the exact boundary `from_save` documents as untrusted. | REAL | Doc rewritten: v1 saves always write the complete struct; plain fields stay correct because a missing one should refuse the parse. |
  | 5 | LOW | `SaveData` doc + notes overclaimed "ids monotone across a load" — loading an OLDER save rewinds live-feed ids. Verified harmless (consumers key on type; datastar opening-skip compares only boot-opening ids). | REAL (doc) | Doc + notes restated as "no collision with the loaded session's own history". |
  | 6 | LOW | `SaveLoadRequest` doc said "optional body" but `Json<…>` requires one (no-body → axum 4xx, not the `{ok,error}` envelope); unused `Default` derive. | REAL (doc) | Doc reworded (body required, `slot` field optional — matches the recorded Phase-3 deviation); `Default` dropped. |
  | 7 | LOW | `loadGame()` fire-and-forgot `refreshState()`; a silent refresh failure strands a stale HUD/battle-modal next to "Game loaded." | REAL | `await refreshState()` (newButton's identical fragility noted as pre-existing, untouched). |
  | 8 | LOW | Long unbroken refusal tokens (paths) force horizontal feed scroll (`.log-entry p` had no wrap). | REAL | `overflow-wrap: anywhere` + comment in `styles.css`. |
- Rejected / accepted-as-is (with verification):
  - **Coherence audit otherwise COMPLETE** — both core critics independently
    swept every `GameState` field to every read site and cleared them:
    pack/discovered/offered ids total; hp/xp/enemy-stat arithmetic saturating;
    `pulse_rate: 0` = pulse-per-tick (no loop); world-side expects covered by
    `validate()`; serde surface total (no `#[serde(default)]` bypass; missing
    fields refuse the parse). Slot validation precedes ALL fs contact; symlink
    defense on read+write+temp; load failures provably pre-lock.
  - **Save snapshot consistency** — clone under the single engine mutex is
    point-in-time; write after drop can't tear it. Blocking `std::fs` in the
    async handlers: sub-ms payloads, engine lock never held across IO,
    single player — accepted (note: `spawn_blocking` if payloads grow).
  - **Opening-seed staleness (Q6)** — rationale VERIFIED: `begin()` renders
    only static room fields (title/description/exits; never placements), so
    the boot opening is byte-identical for any same-module save; the
    pre-existing reload artifact is unchanged. Accepted as designed.
  - **Load broadcasts nothing** — designed (Q7, client-side feedback);
    second-tab staleness + the in-flight-combat-line race noted for a v2
    load-announcement (the #27 machinery exists). Accepted.
  - **Reentrancy** — double-click converges client-side (append-only feed,
    idempotent requests); the fs hazard was finding 2, fixed server-side
    where an untrusted client must be defended anyway. No in-flight guard
    (house pattern guards nowhere; consistency over one-off armor).
  - **Refusal XSS** — `appendLine` is `textContent`-only; datastar escapes
    server-rendered HTML. Cleared.
  - **Battle-modal interplay** — `toBattle.active` mirrors `snapshot.combat`;
    renderBattle closes/opens on load-out/into combat; cadence resumes from
    persisted `next_pulse_at`/`tick`. Cleared (modulo finding 7's fix).
  - **`focusCommandInput` try/finally idiom** — equivalent today
    (`persistenceRequest` is throw-proof); left as-is to avoid churn.
  - **event-id saturation rail** (`u64::MAX` duplicate ids) — cosmetic at an
    unreachable rail; not clamped.
- Mutation-surface notes handed to validate: the version `==` gate, the three
  coherence `contains_key`s (both arms each), `save_data`/`from_save`
  fn-replaces, the two new `saturating_add(1)` round sites (P5 must pin round
  advance at the rail… or via exact round assertions in round-trip tests),
  the NotFound-vs-other refusal branch (SV3 asserts the friendly string;
  needs a sibling non-NotFound arm or the branch survives), the temp+rename
  branch (target-is-dir test asserts rename-failure context + temp cleanup),
  `DEFAULT_SAVE_SLOT`/env fallback (SV1 file-path assert).

## Phase 4 — Validate
- **Tests added (15 new; every EARS REQ covered):**
  - Core (`oathstar-core`, 11): `played_spoils_engine` +
    `snapshot_value` helpers; SL1
    `save_data_captures_the_mutated_session_without_mutating_it`
    (REQ-001 — payload by value incl. removed placement/cleared
    inventory/pack/xp/event counter; before==after snapshot); SL2
    `from_save_restores_a_byte_identical_snapshot` (REQ-002); SL3
    `mid_combat_save_resumes_the_exact_cadence_and_queued_action`
    (REQ-002 mid-combat — queued PowerStrike + banked guard charge in the
    payload; restored future == unsaved twin's, event-for-event over two
    pulses); SL4 ×5 refusal both-arms
    (`…version_mismatch_by_value`, `…revalidates_the_world` DanglingExit
    by value, `…unknown_current_room` / `…unknown_combat_enemy` /
    `…unknown_sworn_oath` each with its matching valid arm — REQ-003);
    SL4b `load_error_messages_render` (the three Display contracts);
    SL5 `from_save_tolerates_orphan_pack_and_discovered_ids` (the
    audit's tolerance line — id fallback pinned); SL6
    `loaded_integer_ceilings_saturate_instead_of_panicking` (tick +
    next_event_id at `u64::MAX`, round at `u32::MAX` due NOW — pulses
    twice, pins `CombatPulse { round: u32::MAX }`; inspect finding 1's
    regression test).
  - Server (`oathstar-server`, 6): `slot_request`/`state_value` helpers;
    SV1 `save_then_load_round_trips_state` (REQ-005/006 — slot file under
    the temp root, diverge, byte-identical /state restore); SV2
    `invalid_slot_is_refused_without_touching_the_filesystem` (REQ-004 —
    `../evil` separator message verbatim, no dir created); SV3a
    `missing_save_load_is_refused_and_state_unchanged` (the friendly
    NotFound line, exact); SV3b
    `corrupt_save_load_is_refused_with_the_parse_context` (the
    non-NotFound arm — branch-flip killer); SV3c
    `version_mismatched_save_load_is_refused_via_from_save` (on-disk
    version bump → exact LoadError display forwarded); SV4
    `played_loop_xp_and_fang_survive_save_and_load` (REQ-007 — earn →
    save → drop fang + walk → load → xp 5/fang/ashen_road); SV5
    `mid_combat_save_and_load_resume_the_saved_fight` (REQ-005 — save
    mid-fight, live fight finishes, load under the RUNNING paused-time
    tick loop, at-save /state byte-identical, restored fight pulses to
    the same victory: xp 5, hp 14).
  - Storage (2 assertions into existing tests): round-trip asserts no
    `.tmp` remains after success; target-is-dir asserts the failed
    rename cleans up its temp sibling (inspect finding 2's pins).
  - JS rows: none (thin-glue carve-out per design; `npm run build` green).
- **Suites run (actual):** `cargo test --workspace` — 322 passed
  (core 222 / server 23 / storage 22 / protocol+datastar+content the
  rest), 0 failed; `node --test tests/*.test.js` — all passing
  (duration ~91ms); `npm run build` — built in 72ms.
- **Gate:** `./bin/gate.sh --fast` → **GATE GREEN [fast]**, 14/14 PASS
  (rustfmt, clippy strict, cargo test, node --test, audit, deny, machete,
  gitleaks, shellcheck, no-suppressions, source-bans, lints-allowlist,
  doc-todos, tauri shell). Gates 15–17 (coverage+mutation) are
  owner-gated with the FULL gate before `/commit`, per instruction.
- **Pre-existing failures:** none.
- Tests added:
- `cargo test --workspace`: <result>
- `node --test tests/*.test.js`: <result>
- `npm run build`: <result>
- `bin/gate.sh --fast`: <result — FULL gate is owner-gated this run>
- Pre-existing exclusions:

## Phase 5 — Complete
- Docs updated:
- Forge capture (aar/failures/rules/decisions):
- Ticket closed:
- Archived:
