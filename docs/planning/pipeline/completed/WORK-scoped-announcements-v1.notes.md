# WORK-scoped-announcements-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #27 — Scoped Announcements & Notification Delivery v1,
  promoted from the announcements intake (its Candidate Future Tickets
  item 2; item 1 Area hierarchy deferred). A server-authoritative scoped
  announcement layer through the existing event lifecycle: typed additive
  event, engine emit API with deterministic scope matching
  (world/region/subregion/room/radius — radius via spatial awareness), feed
  rendering via the existing component path, one authored beginner emission
  site, exact tests, docs. AUTO-APPROVE; STOP BEFORE `/commit`; validate =
  workspace tests + node tests + `npm run build` + `./bin/gate.sh --fast`
  (FULL gate/commit owner-gated).
- **Intake source:**
  `docs/planning/intake/INTAKE-announcements-notifications-and-area-scopes.md`
  — promoted this phase (frontmatter `status: promoted` + `ticket` +
  `pipeline_spec` filled). Forge ticket minted:
  `5e945705-2a54-4add-a7d2-cce9b60cefc4` (#27); fresh local ticket doc
  created from the template (this ticket had no pre-existing doc).
- **Classification / tier:** Work pipeline, **one shippable slice** — the
  notification API only. The intake's other slices (Area hierarchy, boards,
  scheduler, tray) are explicit later tickets. Touches: a new event kind
  (protocol + datastar arms), an engine scope/emit module, one authored
  trigger, light-or-zero JS.
- **Base verified:** branch `codex/oathstar-ticket-27-scoped-announcements`
  off `main` @ `a707cff` (#26 — everything merged). Worktree clean except
  the Codex-owned strays (untouched) and this pipeline's docs. No active
  pipeline; forge up.
- **Forge recall (lessons/failures surfaced):**
  - AAR opened: `63e0fd3e-173b-4f20-b778-aab48eef9540`; Plan-phase
    `knowledge-context` logged (13 surfacings). Fresh subsystem — no prior
    announcement lessons; the intake doc itself is indexed and is the
    design source (scope ladder, audibility levels, candidate shape,
    perceive-vs-receive split).
  - Directly-applicable standing rules (all surfaced):
    **PR-claude-fixture-distinguishable-transitions-001** — every scope
    test stages BOTH the delivered and the not-delivered location;
    **PR-claude-enumerate-variant-string-arms-001** — severity/scope
    variant arms each get exact tests;
    **PR-claude-package-scope-mutation-001** — the delivery-decision
    killers live in `oathstar-core`;
    **PR-claude-expect-invariants-over-unreachable-arms-001**.
  - ADs `60cc1bba`/`939d18d0` (spatial/event lineage) +
    `2aadcccb` (the reward-loop single-funnel pattern — the same
    one-emission-point shape applies to the demo trigger).
- **Current-code anchor map (from the #22–#26 sessions):**
  - `crates/oathstar-protocol/src/lib.rs`: `GameEventKind` (snake_case
    `type` tags — gains the announcement variant; datastar + wire tests
    compile-catch the arms); `EventChannel` already includes `Region`,
    `System`, `Dm`, `Narrative` (design picks the channel — a NEW channel
    variant would force `channel_str`/JS channel-label arms, also
    compile-caught); `OutputComponent` (if a feed component label is
    wanted).
  - `crates/oathstar-core/src/lib.rs`: `Engine::event`/`log` constructors;
    `GameState.current_room_id` + `current_room()` (the location the
    decision matches against); `RoomDefinition { region, subregion, x, y,
    z }` (scope fields); `confront()` (~line 1570 — the lead demo trigger;
    fulfills the oath, emits on Combat/Oath channels);
    `world.rooms` lookups for the radius origin.
  - `crates/oathstar-core/src/awareness.rs`: the Chebyshev distance +
    same-z/subregion semantics (`perceive`/`RadiusConfig`) — the radius
    DELIVERY check reuses the distance math but NOT the perceive filter
    (hidden entities etc. are irrelevant to receipt).
  - `crates/oathstar-datastar/src/lib.rs`: `describe`/`kind_type`/
    `feed_patch` exhaustive matches (announcement gains arms — the
    severity→variant mapping mirrors `component_variant_label`).
  - JS: `src/client/wire.js` `parseEvent` (default passthrough covers a new
    type for the refresh predicate — announcements likely need NO refresh
    since they change no snapshot state; design confirms);
    `src/client/components.js` `toComponent` (a JS label/variant mapping if
    the JSON-feed path needs it — likely only the datastar path matters);
    no modal/HUD work.
  - Content: `modules/beginner/world.toml` / oaths — the bell-alarm trigger
    authoring lives wherever design hooks it (likely the confront path +
    optional authored fields on the oath or module).
- **EARS requirements reviewed:** REQ-001..007 (verbatim in the spec,
  derived from the intake + the promotion scope). 001/002/003 the scope
  matrix (both arms each); 004 wire + feed; 005 server authority; 006 the
  authored demo; 007 preservation. Every REQ gets ≥1 exact test in the
  Phase-2 plan.

### Open design questions (for Phase 2 — Planner does NOT decide these)
1. **Event shape.** What rides the wire: minimal
   `Announcement { severity, text }` vs the trimmed intake shape
   (`+ title? + source? + scope-echo?`). The client needs severity (render
   variant) and text; `source` gives attribution ("The bell of Hollowmere:
   …" could just live in the text); echoing the scope is only useful for a
   future tray — recommend trimming hard and documenting what was cut.
2. **Severity set.** Trim the intake's open list to render-meaningful
   levels (e.g. `notice / warning / alarm`?) — each becomes a feed variant
   arm with an exact test (string-arm rule). How severities map to the
   existing feed variants (`system`/`danger`/etc.) vs new CSS.
3. **Channel.** Reuse `EventChannel::Region` (semantic fit for scoped world
   news) vs `System`/`Dm` vs a NEW `Announcement` channel (additive but
   forces channel arms in datastar + the JS channel labels — all
   compile-caught/enumerable). Recommendation to weigh: reuse `Region`.
4. **Scope type + decision function.** Engine-side
   `AnnouncementScope { World, Region(id), Subregion(id), Room(id),
   Radius { room_id, radius } }`; `fn announcement_received(&self, scope) ->
   bool` (pure, testable) + `fn announce(&mut self, scope, severity, text)
   -> Option<GameEvent>`/push-into-events. Where it lives (lib.rs vs a new
   `announce.rs` module — core has precedent for `awareness.rs`).
5. **Radius semantics.** Same z-plane + same subregion like awareness, or
   pure Chebyshev across the world? (The intake's explosion example implies
   spatial reach; awareness constrains to subregion/z — recommend matching
   awareness semantics for consistency, documented.) Radius origin: a room
   id (resolve to x/y/z) vs raw coordinates — room id is authorable.
6. **The demo trigger + scope.** Lead candidate: `confront()` fulfillment
   emits the bell announcement. Scope choice problem: the roost is in
   `old_bell_tower`, the player stands there at fulfillment — a
   `hollowmere`-region scope would NOT be delivered to them (great test,
   bad demo); `world` scope delivers but demonstrates nothing scoped;
   radius-from-roost delivers within N cells. Options design weighs:
   (a) world-scoped bell ("all Hollowmere hears" as flavor text — delivered
   anywhere, scoping proven by constructed tests only); (b) TWO authored
   announcements at confront (a world alarm + a region notice — one
   delivered, one provably not, both visible in tests; only the delivered
   one reaches the feed in play); (c) move the demo trigger (e.g. first
   entry to `ashen_road` emits a subregion warning — delivered when
   entering, not delivered when elsewhere). Pick what makes the played demo
   AND the both-arms tests honest.
7. **Does the client need ANY change?** Datastar renders the feed line
   server-side (new arms there); the JSON client's refresh predicate
   doesn't need announcements (no snapshot state changes); `toComponent`
   only matters for... nothing rendered client-side from the JSON stream.
   Possibly ZERO JS changes — confirm, and decide whether the JS
   `components.js` map should still learn the type for future-proofing
   (recommend: no speculative code).
8. **Emit API mutability.** `announce` needs `&mut self` for event ids
   (`self.event(...)`); callers are engine-internal triggers now, the
   server/DM later — keep it private or `pub`? (Recommend private until a
   real external caller exists; the server has no announce route in v1.)
9. **Severity↔variant mapping + exact strings.** The Datastar arm's
   variant/label per severity and the demo announcement's exact authored
   text — pinned at design so validate's exact-string tests are written
   against decided strings.

## Phase 2 — Design

### Approach / architecture (the 9 open questions, resolved)

The engine/content split is the keystone (Q6 sharpened during design):
**announcements are an engine MECHANISM with authored CONTENT** — the bell
texts live on the beginner oath, never hardcoded in `confront()`. The same
shape as `CombatProfile.xp`: the engine knows how to deliver; modules say
what and where.

1. **Event shape (Q1): trimmed hard.**
   `GameEventKind::Announcement { severity: AnnouncementSeverity, text: String }`
   — snake_case tag `announcement`. Cut from the intake's candidate shape
   (documented): `title` (one feed line — the text IS the message), `source`
   (attribution reads naturally in authored text), scope echo (no v1
   consumer — a future tray re-adds it), `origin/radius/audibility/ticks/
   persistent/read_state/actions` (later slices). The envelope's existing
   `event_id`/`tick` cover identity/time.
2. **Severity set (Q2): `Notice / Warning / Alarm`** (snake_case wire
   values) — the intake's audibility list collapsed to render-meaningful
   levels. Feed mapping (Q9, against the existing variant vocabulary —
   room/dialogue/system/danger/success — NO new CSS):
   Notice → `("system", "Notice")`, Warning → `("danger", "Warning")`,
   Alarm → `("danger", "Alarm")`. Two severities share the danger variant
   with distinct labels — accepted for v1; a dedicated announcement style
   belongs to the future tray ticket.
3. **Channel (Q3): reuse `EventChannel::Region`.** Scoped world news is
   exactly what Region exists for; no new channel arms anywhere. (The
   datastar arm carries its own severity mapping, so the channel label is
   not load-bearing.)
4. **Scope type + decision (Q4).** Core-side serde types (TOML-authorable):
   `AnnouncementScope { World, Region(String), Subregion(String),
   Room(String), Radius { room_id: String, radius: u32 } }` (externally
   tagged — TOML: `scope = "world"` / `scope = { region = "hollowmere" }` /
   `scope = { radius = { room_id = "bell_eater_roost", radius = 2 } }`) and
   `AnnouncementSeverity` (core mirror of the protocol enum? NO — core
   reuses the protocol enum directly, like `CombatOutcome`). Decision fn:
   `fn announcement_received(&self, scope: &AnnouncementScope) -> bool` —
   pure over the CURRENT room: World → true; Region(id) → `room.region ==
   id`; Subregion(id) → `room.subregion.as_deref() == Some(id)`; Room(id) →
   `room.id == id`; Radius → resolve the origin room and reuse
   **awareness's `cell_distance`** (`Option<u32>` — None across
   z-planes/subregions → not received), `map_or(false, |d| d <= radius)`,
   with an unknown origin room also `false` via the total chain (reachable
   both-arms, no defensive arm). Emit:
   `fn announce(&mut self, scope, severity, text) -> Option<GameEvent>` —
   `received.then(|| self.event(Region, Announcement { … }))`. Both private
   (Q8 — no external caller until a DM/server route exists). Both live in
   `lib.rs` (~40 lines; `awareness.rs` stays perception-only — the intake's
   perceive-vs-receive split).
5. **Radius semantics (Q5): awareness-consistent.** Same subregion + same
   z-plane + Chebyshev ≤ radius, by REUSING `cell_distance` rather than
   re-deriving — one spatial model, two layers. Origin is a room id
   (authorable), resolved against `world.rooms`.
6. **Authored announcements + the demo (Q6).** `OathDefinition` gains
   `#[serde(default)] fulfillment_announcements: Vec<AuthoredAnnouncement>`
   with `AuthoredAnnouncement { scope, severity, text }`. `confront()`
   emits them right after `OathFulfilled` (each through `announce` — only
   received ones become events). **Validation contract (#21-style):**
   `validate()` gains a check that every authored scope id resolves
   (region/subregion/room registries; radius room id) → new
   `WorldValidationError::AnnouncementScopeMissing { oath_id, scope_kind,
   id }` — fail-fast authored content. Beginner demo — the `hollow_bell`
   oath authors TWO:
   - `scope = "world"`, severity `alarm`, text
     **"The bell of Hollowmere rings again. Its voice rolls out over every
     road and roof."** → DELIVERED at the roost (the played demo line).
   - `scope = { region = "hollowmere" }`, severity `notice`, text
     **"Hollowmere's streets fill with voices as the bell's song
     returns."** → NOT delivered at the roost (`old_bell_tower` region) —
     the in-play not-delivered case, asserted absent in the confront tests;
     its delivered arm is proven on a constructed world (both-arms rule).
7. **Client (Q7): ZERO JS changes.** Datastar renders the feed line
   server-side (new arms); the JSON stream carries the event through
   `parseEvent`'s default case; announcements change no snapshot state so
   the refresh predicate is untouched; no speculative `components.js`
   mapping. REQ-004's JS leg is satisfied by the existing
   passthrough (no new JS code ⇒ no new JS test; noted).
8. **Emit API private (Q8)** until a real external caller lands (DM route,
   scheduler).
9. **Strings pinned (Q9):** the two demo texts above + the three
   severity→(variant, label) pairs — exact tests against these.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-protocol/src/lib.rs` | `GameEventKind::Announcement { severity, text }` (additive, doc-commented); `pub enum AnnouncementSeverity { Notice, Warning, Alarm }` (snake_case serde). |
| 2 | `crates/oathstar-core/src/lib.rs` | `AnnouncementScope` (serde, TOML-authorable) + `AuthoredAnnouncement { scope, severity, text }`; `OathDefinition.fulfillment_announcements` (`#[serde(default)]`); `announcement_received` + `announce` (private); `confront()` emits the oath's authored announcements after `OathFulfilled`; `validate()` scope-id contract + `WorldValidationError::AnnouncementScopeMissing` (+ Display arm). |
| 3 | `crates/oathstar-core/src/awareness.rs` | None expected — `cell_distance` is already callable; only visibility widening if needed (pub(crate)). |
| 4 | `crates/oathstar-datastar/src/lib.rs` | `describe` arm: Announcement → severity-mapped `(variant, label)` + text body; `kind_type` → `"announcement"`. |
| 5 | `modules/beginner/world.toml` | `hollow_bell` oath authors the two fulfillment announcements (world alarm + hollowmere notice). |
| 6 | docs (Phase 5): `docs/event-lifecycle.md` or `docs/protocol-and-output.md` (announcement event + scopes), `docs/spatial-awareness.md` (the perceive-vs-receive note), `docs/decisions.md` (Decision 045), `docs/mechanics-and-systems.md` if apt. |

**No changes**: `oathstar-server`, `command.rs`, all JS, `styles.css`,
`index.html`, `rooms.toml`.

### Regression Test Plan
≥1 per EARS REQ; both arms for every scope (the fixture-distinguishability
rule); exact strings/values. Core tests build small constructed worlds
(two regions/subregions/z-planes as needed — `room_with`/`world_with`
fixtures already parameterize this).

| # | Test | Proves |
|---|---|---|
| N1 | world scope: received in the start room AND in a different region's room (no false arm exists — the `World => true` mutant dies to the delivery asserts) | REQ-001 |
| N2 | region scope both arms: received in a room of the named region; NOT received from a room in another region (two-region fixture; the emitted-vs-absent asserted by value) | REQ-002 |
| N3 | subregion scope both arms: received in the subregion; not received in a sibling subregion AND in a room with `subregion: None` | REQ-002 |
| N4 | room scope both arms: received in the exact room; not received one room away | REQ-002 |
| N5 | radius both arms + model consistency: received at distance == radius (boundary, kills `<=`→`<`); not received at radius+1; not received across z-planes; not received across subregions; not received for an unknown origin room id (the total-chain false arm) | REQ-003 |
| N6 | wire: `{"type":"announcement","severity":"alarm","text":…}` snake_case + all three severities round-trip | REQ-004 |
| N7 | datastar: three exact severity arms — Notice→system/"Notice", Warning→danger/"Warning", Alarm→danger/"Alarm" — body text escaped; `kind_type == "announcement"` | REQ-004 |
| N8 | determinism: identical world+commands twice → byte-identical announcement events; server authority is structural (the event exists only when received — review note) | REQ-005 |
| N9 | confront on a constructed world whose oath authors a Room-scoped announcement for the boss room → delivered with exact text; a second authored announcement scoped to an elsewhere-room → absent from the same response | REQ-002/005/006 |
| N10 | beginner content: `hollow_bell` authors exactly the two announcements (scope/severity/text by value); the world validates | REQ-006 |
| N11 | beginner slice (server test extension): `confront` response/stream contains the world-alarm announcement with its exact text and does NOT contain the hollowmere notice (the in-play both-arms demo) | REQ-006 |
| N12 | validation contract: an authored announcement naming an unknown region/room → `AnnouncementScopeMissing` by value; valid scopes pass | REQ-006 |
| N13 | (preservation) the confront/oath flow tests, full combat suites, JS 46, build — all stay green | REQ-007 |

**No JS rows** (zero JS code — REQ-004's client leg is the existing default
passthrough; the datastar Rust tests pin the rendered feed).
**Genuinely uncoverable:** none new (browser smoke for the played bell line).

### Risks / decisions
- **R1 — authored-on-oath (load-bearing):** announcement CONTENT lives on
  `OathDefinition`, mechanism in the engine — no module fiction in engine
  code. Future trigger sites (room entry, schedulers, DM) reuse
  `announce()` with their own authored carriers.
- **R2 — channel reuse (`Region`)** keeps the wire surface flat; if a tray
  ticket later wants a dedicated channel, the event kind (not the channel)
  is the stable contract.
- **R3 — radius = awareness semantics** (same subregion/z): the intake's
  cross-room explosion example is SATISFIED within a subregion; cross-
  subregion sound is explicitly out (documented) until the Area ticket
  revisits acoustics.
- **R4 — scope validation contract:** fail-fast at `try_new` like #21/
  `EntityItemMissing`; the runtime decision fn still total-chains to
  `false` for robustness (both reachable: validation tests construct
  pre-validate worlds via direct struct building).
- **R5 — emit-iff-received** means nothing scope-filtered ever serializes —
  the strongest possible REQ-005 (no client could decide receipt even
  maliciously). Multiplayer later changes the CALLER (per-session
  decision), not the decision fn.
- **R6 — severity variant sharing** (Warning+Alarm both `danger`): labels
  differ; dedicated styling deferred with the tray. Exact-arm tests pin all
  three (string-arm rule).
- **R7 — mutation pins:** the five scope match arms (each both-arms by
  value), the radius `<=` boundary (N5's == radius case), `cell_distance`
  reuse (cross-plane None arm), severity arms (N7 exact strings), the
  `then()` emit guard (N2's absent-assert kills `received`→`true`).
  Killers all live in core/datastar (package-scope rule).

## Phase 3 — Implement
- **Built (manifest rows 1–5; tests → Validate, docs → Complete):**
  - **protocol**: `GameEventKind::Announcement { severity, text }` (additive,
    doc-commented with the emit-iff-received contract) +
    `AnnouncementSeverity { Notice, Warning, Alarm }` (snake_case serde).
  - **core**: `AnnouncementScope` (serde, TOML-authorable, doc'd TOML forms)
    + `AuthoredAnnouncement { scope, severity, text }`;
    `OathDefinition.fulfillment_announcements` (`#[serde(default)]`);
    `announcement_received` (pure match over the current room; Radius via
    `awareness::Position::from_room` + `cell_distance` with
    `is_some_and(d <= radius)` — unknown origin and cross-plane both land
    `false` through the total chain); `announce` (received → `event(Region,
    Announcement)`; emit-iff-received); `confront()` fulfillment block now
    clones the oath's authored announcements (`map_or_else(Vec::new, …)`)
    and extends the response with each delivered one;
    `validate_oaths` gains the scope-id contract →
    `WorldValidationError::AnnouncementScopeMissing { oath_id, scope_kind,
    id }` (+ Display arm). Three `OathDefinition` test literals compile-fixed
    with `fulfillment_announcements: Vec::new()`.
  - **datastar**: `describe` Announcement arm (severity→variant/label:
    Notice→system/"Notice", Warning→danger/"Warning", Alarm→danger/"Alarm";
    body = text) + `kind_type` → `"announcement"`; import widened.
  - **content** (`world.toml`): `hollow_bell` authors the two pinned
    fulfillment announcements (world alarm + hollowmere-region notice, with
    the why-comment explaining the in-play both-arms demo).
  - **Untouched as designed:** `oathstar-server`, `command.rs`, all JS,
    `styles.css`, `index.html`, `rooms.toml`, `awareness.rs`
    (`Position`/`cell_distance` were already `pub` — no visibility change
    needed).
- **Compile/check (this phase):** `cargo check -p oathstar-core
  --all-targets` surfaced exactly the three predicted literal sites (fixed);
  `cargo fmt --all`; `cargo clippy --workspace --all-targets --all-features
  -- -D warnings` **GREEN first run**; full regression: all 11 Rust suites
  green (incl. every confront/oath test — the new announcement events ride
  the responses additively and the existing `.any()` asserts tolerate
  them), `node --test` 46/46, `npm run build` OK.
- **Deviations from design (+ reason):** none — implemented exactly to the
  manifest and pinned strings. One note: `OathFulfilled`'s `oath_id` is now
  cloned (the id is needed again for the announcements lookup) — shape-only.

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel `general-purpose` critics over `git diff HEAD` @
  base `a707cff`): (1) correctness / TOML-serde reality / confront interplay —
  **CLEAN** across 23 scratch tests: all three authored scope TOML forms
  parse (plus loud failures for malformed ones), every scope edge verified
  (subregion-None rooms, region↔subregion id crosstalk, radius 0/boundary/
  cross-plane/unknown-origin, empty-string determinism), confront order
  exact ([combat line, OathFulfilled, delivered announcements]), re-confront
  and no-oath paths emit nothing, the announcement rides the SSE broadcast
  with the exact wire shape, and `announce` is provably the sole
  `Announcement` constructor (nothing scope-filtered can serialize).
  (2) mutation hygiene — full mutant enumeration with a kill-list; the #25
  exhaustive-match finding reconfirmed (zero arm-delete mutants for the
  scope/severity/validate matches → the exact-test rows are the sole pins).
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | I1 | HIGH | The confront announcements lookup used a defensive `map_or_else(Vec::new, …)` on a try_new-validated invariant — the `Vec::new` arm is production-unreachable, mutation-invisible, and contradicts PR-claude-expect-invariants-over-unreachable-arms-001 (and `swear()`'s own `.expect` on the SAME lookup) (lib.rs ~1702) | **REAL, discipline** | `.expect("sworn oath is a try_new-validated invariant")` + invariant comment |
  | I2 | LOW | Radius docs said "same plane (subregion + z)" but `same_plane` also requires REGION equality — behavior stricter than documented (lib.rs scope enum + decision fn docs) | **REAL, doc rot** | Both comments now say region + subregion + z |
  | I3 | INFO→FIXED | `AnnouncementSeverity` was core public API (via `AuthoredAnnouncement.severity`) without a re-export — core dependents (the content N10 test!) couldn't name it | **REAL** | `pub use oathstar_protocol::AnnouncementSeverity;` (the re-export is now the sole import) |
  | I4 | MEDIUM | The datastar announcement arm is 100%-MSI-invisible (no mutants generated; the fn-level mutants die to OTHER kinds' tests) — N7's three exact severity tuples are its only pin | **REAL, test-plan emphasis** | N7 binding at validate, in oathstar-datastar |
  | I5 | MEDIUM | N12 needs the Display exact string (house T6 pattern) including a Radius case pinning the `Room\|Radius` or-pattern's `"room"` label; the Display arm has no other in-crate coverage path | **REAL, test-plan gap** | N12 extended at validate |
  | I6 | LOW | N12's valid-pass arm under-enumerates — no valid Subregion/Radius scope ever validates in any fixture (both-arms rule) | **REAL, test-plan gap** | N12 valid-pass enumerates all four scope kinds |
  | R1 | INFO | The datastar escape-guard test's kinds array lacks Announcement (escaping verified in place by scratch) | **carry to validate** | extend the array |
  | R2 | INFO | `announce → Some(Default)` / confront vec-Default mutants are UNVIABLE (no `GameEvent: Default`) — excluded from MSI | accepted | none |
  | R3 | INFO | `<=` at the radius boundary only generates the `>` mutant; the `d == radius` case stays design-mandated (N5) | accepted | keep N5's boundary |
  | R4 | INFO | Reuse verdicts clean: the Radius chain cleanly reuses awareness; the inline severity match is the right home (Announcement carries no OutputComponent); no dead derives | accepted | none |
- **Mutation kill-list (binding for Phase 4, from critic 2):** N1 is the
  ONLY pin for the World arm (fn-level `→ false` mutant); the three `==`
  comparisons die to both-arms fixtures (N2/N3/N4); the radius `<= → >` to
  N5's boundary pair; `announce → None` to N9/N11's exact delivered event;
  the validate `delete !` trio to N12's missing+valid pairs (now all four
  kinds); protocol has ZERO mutants (N6 is REQ-only); datastar's arm is
  pinned solely by N7. NEW rows from inspect: Display exact string (+Radius
  →"room"), valid-Subregion/Radius passes, the escape-guard array, and the
  optional region/subregion id-crosstalk hardening.
- **Verification of fixes:** one import collision from the re-export fixed
  (the `pub use` is now the sole import); `cargo fmt --check` clean; clippy
  strict GREEN; all 11 suites green; JS 46/46. No `failure-record` — no
  behavioral bug (I1 is the known discipline class, already covered by the
  existing prevention rule; the rest are doc/test-plan items).

## Phase 4 — Validate
- **Tests added (11 new: 8 core + 1 protocol + 1 datastar (+ escape-guard
  extension) + 1 content + the extended server slice):**
  - `oathstar-core` (8, + the `scoped_engine`/`at`/`announcing_oath_engine`
    fixtures): `world_scope_is_received_everywhere` (N1 — the sole pin for
    the World arm), `region_scope_matches_only_the_named_region` (N2 + the
    subregion-id crosstalk arm),
    `subregion_scope_matches_only_the_named_subregion` (N3 + None-room +
    region-id crosstalk), `room_scope_matches_only_the_exact_room` (N4),
    `radius_scope_follows_the_awareness_plane_model` (N5 — inclusive
    boundary, beyond, cross-floor, cross-subregion, cross-region with None
    subregions, unknown origin, radius 0),
    `fulfillment_delivers_only_in_scope_announcements` (N9 — exact
    severity/text on the Region channel, positioned after OathFulfilled;
    the out-of-scope announcement absent entirely),
    `announcement_delivery_is_deterministic` (N8),
    `announcement_scopes_are_validated_at_construction` (N12 extended —
    four missing-id arms with EXACT Display strings incl. the
    Radius→"room" label, plus the valid-pass arm enumerating all five
    authored scope forms).
  - `oathstar-protocol` (1):
    `announcement_serializes_with_snake_case_tag_and_severity` (N6 — tag,
    channel, all three severity wire values, round-trip).
  - `oathstar-datastar` (1 + 1 extension): `announcements_render_by_severity`
    (N7 — the sole pin for the arm: three exact (variant, label) tuples,
    `data-component="announcement"`, escaped body); the
    `no_feed_kind_leaks_raw_markup` kinds array now includes Announcement
    (inspect R1).
  - `oathstar-content` (1): `beginner_oath_authors_the_bell_announcements`
    (N10 — both authored announcements by value; world validates).
  - `oathstar-server`: the beginner-slice test extended with N11 — the
    world-scoped bell alarm delivered at the roost with its exact text, the
    hollowmere-region notice provably absent (the in-play both-arms demo).
  - N13 preservation = the untouched existing suites.
- `cargo test --workspace`: **GREEN** — core **211**, content **22**,
  datastar **15**, protocol **20**, server 16 (slice extended), storage 20;
  0 failed. All new tests passed on the first run.
- `node --test tests/*.test.js`: **46 pass / 0 fail** (zero JS code changes
  — REQ-004's client leg is the existing wire passthrough).
- `npm run build`: OK.
- `bin/gate.sh --fast`: **GATE GREEN [fast] — 14/14 PASS** (gates 15–17
  SKIPPED per the `--fast` owner instruction; the kill-list killers are all
  in their owning crates for the owner-gated FULL run).
- Pre-existing exclusions: none encountered.

## Phase 5 — Complete
- **Docs updated:** `decisions.md` Decision 045 (announcements are
  engine-delivered, content-authored, emitted only when received — the scope
  set, emit-iff-received, the authored-carrier pattern, severity contract,
  the both-arms demo, revisit triggers); `protocol-and-output.md`
  "Implemented (ticket #27)" note (event shape, severity wire values, the
  delivery model); `spatial-awareness.md` "Perceive vs receive" note (the
  reuse boundary: `cell_distance` yes, perception filters no).
- **Forge capture:** AAR `63e0fd3e` closed (`completed`, effectiveness 5,
  25 verdicts, 1 novel finding; jobs enqueued). No failure-records (inspect
  found one discipline item — the defensive `map_or_else` — already covered
  by the existing expect-invariants rule, plus doc/test-plan items).
  `architecture-decision-record` **AD-claude-scoped-announcements-001**
  (`2995ec8f`).
- **Ticket closed:** forge `5e945705` → `done` (closing comment `eb94e152`);
  local doc moved `tickets/open/ → tickets/closed/`, frontmatter updated;
  the intake stays `promoted` with items 1, 3–5 as future candidates.
- **Archived:** `WORK-scoped-announcements-v1.{spec,notes}.md` moved
  `pipeline/active/ → pipeline/completed/`; spec
  `status: Phase 5 — Complete PASS`.
- **STOPPED BEFORE `/commit`** per owner instruction: the FULL gate (15–17)
  and the commit are owner-gated. Branch
  `codex/oathstar-ticket-27-scoped-announcements` carries the uncommitted
  #27 implementation on top of `a707cff` (main).
