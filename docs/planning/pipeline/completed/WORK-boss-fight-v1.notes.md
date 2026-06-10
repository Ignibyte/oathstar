# WORK-boss-fight-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #29 — Boss Fight v1: `confront` starts a real
  pulse-loop encounter with the Bell-Eater (replacing the scripted instant
  win at `confront()`); victory drops the clapper via the #26 funnel;
  recovering the oath-flagged clapper fulfills the oath (#27 announcements
  + Mara's fulfilled dialogue ride unchanged). AUTO-APPROVE; STOP BEFORE
  `/commit`; validate = workspace tests + node tests + `npm run build` +
  `./bin/gate.sh --fast` (FULL gate/commit owner-gated).
- **Intake source:** none — the carried observation from the #22–#28
  session: the entire combat stack exists, but the module climax bypasses
  it; Decision 007 wants boss battles authored and memorable; the JS
  prototype's loop was "defeat drops key item"; the Bell-Eater has carried
  future-combat stats + the clapper since #16/#21 as forward hooks.
- **Classification / tier:** Work pipeline, one shippable slice — a combat
  entry path, a fulfillment trigger move, content numbers, tests. The
  systems being stitched (#22–#28) all exist; expected new engine code is
  small and the protocol/UI layers are expected untouched.
- **Base verified:** branch `codex/oathstar-ticket-29-boss-fight` off
  `main` @ `db26fe0` (#28 merged + pushed). No active pipeline; forge up;
  no bulletins; Codex strays untouched.
- **Forge recall (lessons/failures surfaced):**
  - `confront()` (lib.rs:1783–1863): boss-by-role lookup → scripted
    fulfillment ("You overcome {boss_name}…") + #27 announcement delivery
    + two distinguished refusals (unsworn / already fulfilled). The #16
    design locked "the oath gates the boss" and noted confront would need
    a target arg only if multi-boss rooms ever exist.
  - The #16 risk note anticipated exactly this ticket: "Boss = boss role
    on a placed entity… if it feels artificial later, the gate is a single
    condition to relax."
  - Decision 007 (combat core): boss battles authored/memorable; not every
    actor killable; combat region-bound. Mechanics doc "Conflict": the
    prototype's loop — attack, retaliate, defeat DROPS KEY ITEM, player
    cannot permanently die — is precisely this ticket's shape.
  - Current-code anchors: `start_combat`/`attack` path is Role::Hostile-
    gated + `room.combat_enabled`-gated + proximity-gated (hostile contract
    REQUIRES a combat profile — RoleContractUnmet validates it);
    `end_combat` Victory = remove + drop inventory + award xp; Defeat =
    max-HP reset to start + penalty `max(1, xp/10)`; Fled = state cleared,
    enemy intact. `CombatState` is enemy-generic (id/name/hp/attack) — a
    boss fits with zero struct changes.
  - Content anchors: `bell_eater` roles `["boss", "combatant"]` (NOT
    hostile), `combat = { health = 12 }` (attack defaults 0, xp defaults 0,
    disclose_stats defaults false); inventory `["bell_clapper"]`; clapper
    item has `flags = ["oath"]`; `bell_eater_roost` does NOT set
    `combat_enabled` (only ashen_road does); Mara's `dialogue.oath.fulfilled`
    line selects on OathStatus::Fulfilled (#19).
  - `take` path (#18/#26): resolver-gated, moves item room→pack — the
    fulfillment hook lands there; the stray's fang (`kind = "trophy"`, no
    flags) must NOT trigger anything (both-arms).
  - Standing rules that bind: **both-arms refusal staging**
    (PR-claude-fixture-distinguishable-transitions-001) — every refusal/
    no-op arm needs its distinguishable sibling; **package-scope mutation**
    (PR-claude-package-scope-mutation-001) — engine killers in core, slice
    killers in server; **operator-sweep** (PR-claude-operator-sweep-
    untrusted-arithmetic-001) — only if `GameState` grows (none expected);
    **expect-invariants** (PR-claude-expect-invariants-over-unreachable-
    arms-001) — confront's oath expect precedent stands.
  - ADs: combat-pulse-rides-tick (the encounter loop), reward-loop-through-
    end_combat (the funnel victory must reuse), save-load boundary (#28 —
    mid-boss saves must keep round-tripping).
- **Ticket:** forge `89436a8c-4038-4c05-8519-ff28059e3626` (#29), local doc
  `docs/planning/tickets/open/TICKET-29-boss-fight-v1-confront-the-bell-eater.md`.
- **AAR opened:** `5ec2ffca-7911-43ba-a1a0-93cb3ab6b7e3` (inspect's
  `failure-record` and complete's `aar-submit` capture into it).
- **EARS requirements reviewed:** REQ-001..008 (verbatim in the spec).
  001 confront-starts-combat; 002 victory funnel without fulfillment;
  003 fulfillment-on-recovery (+announcements +Mara); 004 refusal arms;
  005 defeat consequences + retry-intact; 006 the served played route;
  007 mid-boss save preservation; 008 the gate.

### Open design questions (for Phase 2 — Planner does NOT decide these)
1. **Combat-entry shape.** Reuse `start_combat` with a boss-resolved
   entity (extract a shared helper?) vs a confront-specific entry. The
   hostile path is proximity/room/role-gated; confront's gate is
   oath+boss-in-room. What does `attack bell-eater` do — refused (boss is
   not hostile-roled; "confront" is the boss verb) or also enters? Decide
   + document the role semantics (Decision 007's "not every actor
   killable" cuts both ways).
2. **Does the boss gain `hostile`?** If yes: the hostile contract
   (profile required) is already satisfied, `attack` works, threat
   affordances appear in Nearby — but then confront's distinct gate
   matters less. If no: confront is the only entry and Nearby shows no
   attack affordance. Design decides which fiction the module wants.
3. **`combat_enabled` on the roost.** The room combat gate exists for
   hostile attacks — does confront's entry respect the same gate
   (uniformity) or bypass it (the boss room is implicitly a combat
   space)? Either way the roost likely gains `combat_enabled = true`
   for honesty.
4. **Fulfillment trigger shape.** The clapper already has
   `flags = ["oath"]`. Options: (a) taking ANY oath-flagged item while
   sworn fulfills (generic, but what if a module has several?); (b) an
   authored objective link on the oath (e.g. `objective_item_id`) —
   additive serde, content names the clapper explicitly; (c) keyed off
   the designated oath + flag. Design picks the smallest honest shape +
   the validation contract (dangling objective id must be a
   WorldValidationError).
5. **Re-confront semantics.** After victory-but-before-recovery (boss
   gone, oath sworn): "nothing here to confront" (boss absent) falls out
   naturally — is that the right line? After fulfillment: the existing
   "already broken; your oath is kept" line references the boss by name —
   but the boss entity is REMOVED after victory; does that arm still
   reach? Audit which refusal arms survive the boss's removal and what
   each says.
6. **Flee-from-boss.** Existing Fled semantics (encounter cleared, enemy
   intact) — fine as-is? The pulse keeps hitting while fleeing is queued
   (#24) — boss attack 4 makes flee a real cost. Confirm no special case.
7. **Mid-fight oath state.** Saving mid-boss-fight persists CombatState
   (#28) — the loaded session must keep the sworn oath + boss encounter
   coherent (from_save's enemy gate already covers the boss id). Confirm
   no new coherence gate needed.
8. **Boss numbers.** health 12 / attack ? / xp ? vs player 20 hp, strike
   4, power strike 6, guard negates one return. Candidate: attack 4
   (three returns = 12 damage — dangerous but survivable without guard;
   guard/power-strike visibly matter), xp 25 (5× the stray — milestone
   reward). Design locks the numbers + the disclose_stats choice
   (unknown threat vs visible boss card).
9. **Dialogue/dom content.** Mara's `sworn` line says "the Bell-Eater
   roosts up the old tower" — still right. Does the roost description
   need a post-victory variant (static room text says "The Bell-Eater
   crouches beside the stolen clapper" — a lie after removal)? Room
   descriptions are static (#27 verified begin() renders only static
   fields) — scope says no new description machinery; decide whether to
   reword the static text to stay honest in both states or accept the
   blemish (documented).
10. **OathFulfilled event ordering on take.** take currently emits its
    pickup line; fulfillment adds OathFulfilled + announcements — order
    (pickup → fulfilled → announcements) and channels must be pinned for
    the feed narrative.

## Phase 2 — Design

### Approach / architecture (the 10 open questions, resolved)

1. **Combat entry (Q1/Q2): confront-specific entry; the boss does NOT
   gain `hostile`; `attack bell-eater` stays refused.** The roles carry
   distinct semantics (Decision 007's "not every actor can be killed"):
   `hostile` = ambient attackable via `attack`; `boss` = the authored,
   OATH-GATED encounter via `confront`. Making the boss hostile would
   open an un-gated second entry and break the #16 interlock — the
   distinction is load-bearing, not flavor. `attack bell-eater` falls
   through `find_hostile`'s existing non-hostile refusal (its exact line
   gets a test pin). Mechanically, the `CombatState` construction +
   `CombatStarted` emission + opening round in `start_combat`
   (lib.rs:1949–1974) is extracted into a shared private helper
   `engage_enemy(id, name, profile)`; `start_combat` (hostile path) and
   `confront` (boss path) both call it — one combat model, two gates.
2. **Boss role contract (Q1 cont.): `boss` now REQUIRES a combat
   profile** — a new arm in `validate_entity_contracts` (the exact
   `RoleContractUnmet { entity_id, role: "boss", missing }` shape the
   hostile contract uses at lib.rs:704). `engage_enemy` then reads the
   boss profile through a try_new-validated expect (the house invariant
   pattern). A profile-less boss is a loud load error, never a runtime
   surprise. (Fixture cost: `oath_world`'s warden gains a tiny profile —
   health 4 / attack 0 — so confront's opening strike kills it in one
   round, keeping old test flows short.)
3. **Room gate (Q3): confront BYPASSES `combat_enabled`; the roost does
   NOT gain it.** `combat_enabled` gates ambient hostility (`attack`
   start); the boss encounter's gate IS the oath + boss presence. Once
   combat is active, every mid-fight path (attack-resolves-round, queued
   verbs, pulses) already ignores the room flag, so no invariant breaks.
   The roost stays honest: no hostiles there, so `combat_enabled` would
   be inert anyway. Documented in combat-system.md at Phase 5.
4. **Mid-combat confront (Q5 addendum, found by the audit): `confront`
   while a fight is active resolves the next round** — exactly what
   `attack` does mid-combat (lib.rs:1871). Without this guard a sworn
   re-confront would CONSTRUCT A NEW CombatState and reset the boss's
   hp; with it, confront is idempotent re-entry into the same fight.
5. **Fulfillment trigger (Q4): an authored objective link.**
   `OathDefinition.objective_item_id: Option<String>` (`#[serde(default)]`
   — additive, flows through the content loader's direct serde reuse
   with zero loader changes, the #27 precedent). Taking that item while
   the oath is Sworn fulfills. Validation: a new
   `WorldValidationError::OathObjectiveMissing { oath_id, item_id }`
   when the named item is not in `items` (+ Display + both-arms tests).
   `None` stays valid — an oath without an objective is #16-style
   (fulfillable by nothing; documented). The clapper's `flags = ["oath"]`
   remains display metadata only — flags do NOT trigger (a module with
   two flagged items must not double-fulfill).
6. **The fulfillment block MOVES from `confront` to a helper.**
   `fulfill_oath_on_recovery(item_id) -> Vec<GameEvent>` (private):
   oath is Sworn AND the sworn def's `objective_item_id == Some(item_id)`
   → flip to Fulfilled, emit the fulfillment line + `OathFulfilled` +
   the #27 announcement delivery loop (moved verbatim, expect-invariant
   intact). Called from `take_at`'s success arm; `confront` loses its
   fulfillment branch entirely. `take_at`'s success arm restructures to
   return multiple events (pickup first).
7. **Event ordering (Q10), pinned:** `ItemCard` pickup line ("You take
   the Bell Clapper.") → Oath-channel `OathCard` fulfillment line ("The
   bell's voice is in your hands. Your oath is fulfilled.") →
   `OathFulfilled { oath_id }` → delivered announcements in authored
   order. (Old confront-success Combat-channel line retires with the
   branch.)
8. **Refusal-arm audit (Q5):** post-#29 reachability —
   *unsworn + boss present* ✓ (climb without swearing; line unchanged);
   *nothing to confront* ✓ (pre-tower rooms AND the post-victory roost —
   the line is honest once the boss is removed);
   *already-fulfilled + boss present* ("already broken; your oath is
   kept") — UNREACHABLE in beginner play (fulfillment requires victory,
   which removes the boss) but REACHABLE and load-bearing in authored
   content (objective item placed loose: swear → take → fulfilled → walk
   to a living boss). Kept, tested via a synthetic world. Confront's
   sworn arm becomes the combat entry.
9. **Defeat/flee (Q6): zero new mechanics.** Boss defeat runs the #26
   funnel (max-HP reset to start, `max(1, xp/10)` penalty); the boss
   placement, inventory, and the SWORN oath all survive — the retry
   loop is climb-again-and-confront. Flee keeps existing semantics
   (encounter cleared, boss intact at full hp on re-confront — that hp
   reset on REFRESH entry is correct: a new encounter, not the guarded
   re-entry of (4)).
10. **Save (Q7): no new work.** `from_save`'s enemy gate covers the boss
    id (the registry entry survives victory per #26), mid-fight
    CombatState round-trips per #28. One preservation test re-pins it.
11. **Numbers (Q8), locked:** `combat = { health = 12, attack = 4,
    xp = 25 }`, `disclose_stats` stays false (the boss reads "unknown" —
    intended menace). Fight math vs player 20hp/strike 4: three rounds,
    two returns → player ends 12/20 bare-handed; guard/power-strike
    visibly shorten/soften; a player arriving hurt (≤8 hp) can genuinely
    lose. xp 25 = 5× the stray (milestone reward).
12. **Roost description (Q9): reworded state-neutral** — describes the
    place, not the occupant (the Nearby panel shows the boss while it
    lives): "The roofless chamber smells of copper and wet feathers.
    Broken bell-metal is heaped in a nest against the far wall."
13. **No protocol/datastar/server/JS changes.** The battle modal renders
    any `CombatState`; `GameState` does not grow (no new operator-sweep
    surface); announcements/save machinery untouched.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/lib.rs` | Extract `engage_enemy(id, name, &CombatProfile)` from `start_combat`; rewrite `confront` (mid-combat → resolve round; sworn + boss → `engage_enemy`; unsworn/fulfilled refusal arms kept verbatim; fulfillment branch removed); `take_at` success arm → multi-event + `fulfill_oath_on_recovery` hook; new `fulfill_oath_on_recovery` helper (status flip + line + `OathFulfilled` + announcement loop moved from confront); `OathDefinition.objective_item_id: Option<String>` (serde default, doc-commented); `WorldValidationError::OathObjectiveMissing { oath_id, item_id }` + Display arm + `validate()` check; boss arm in `validate_entity_contracts` (combat profile required); test-fixture compile fixes only (warden profile; struct literal updates). |
| 2 | `modules/beginner/world.toml` | `bell_eater` combat → `{ health = 12, attack = 4, xp = 25 }`; `hollow_bell` gains `objective_item_id = "bell_clapper"`; comments updated (ticket #29). |
| 3 | `modules/beginner/rooms.toml` | Roost description reworded state-neutral. |
| 4 | docs (Phase 5): `docs/decisions.md` (Decision 047), `docs/combat-system.md` (boss entry vs ambient gate), `docs/mechanics-and-systems.md` (Conflict — prototype loop realized), `docs/spatial-awareness.md` only if the confront note needs a touch. |

**No changes:** `oathstar-protocol`, `oathstar-datastar`,
`oathstar-server` (non-test), `oathstar-storage`, `oathstar-content`
(loader — serde flows the new field), `command.rs`, JS, CSS.

### Regression Test Plan
≥1 row per EARS REQ; refusals/no-ops both-arms
(PR-claude-fixture-distinguishable-transitions-001); package-scoped kills.
Existing-test churn is deliberate: #16's confront tests (T5–T8 shapes),
#27's confront-fulfilled announcement tests, and the server slice reshape
to the new two-step (fight → recover) flow; `oath_world`'s warden gains
`health 4 / attack 0` + a loose objective where needed.

| # | Test | Proves |
|---|---|---|
| B1 | sworn + boss present: confront accepted → `CombatStarted` with the boss id/name, `snapshot.combat` carries the authored stats BY VALUE (hp 12→8 after the opening round, attack 4), and the oath is STILL Sworn | REQ-001 |
| B2 | confront mid-fight resolves the next round and does NOT reset `enemy_hp` (the re-entry guard's killer); both arms with a fresh confront | REQ-001 |
| B3 | pulse the boss fight to victory → `CombatEnded { Victory }`, boss placement removed, clapper in the room contents, xp 25 awarded by value, oath STILL Sworn (the no-fulfill-on-victory pin) | REQ-002 |
| B4 | take clapper while Sworn → events pinned IN ORDER: ItemCard pickup → OathCard fulfillment line → `OathFulfilled { oath_id }` → delivered announcement(s); snapshot oath Fulfilled | REQ-003 |
| B5 | no-fulfill arms ×3: take a NON-objective item while Sworn (no oath events); take the objective with NO oath sworn (plain pickup); take when already Fulfilled (no double-fulfill) — each both-staged against B4 | REQ-003 |
| B6 | refusals: unsworn confront (exact line, no CombatState); fulfilled + boss present via synthetic loose-objective world → "already broken" line, no combat; post-victory re-confront → "nothing here to confront" | REQ-004 |
| B7 | defeat by a synthetic boss (attack 99): reset to start, hp restored, penalty applied, boss placement + inventory + Sworn oath all intact; re-confront starts a FRESH full-hp fight (the retry loop) | REQ-005 |
| B8 | mid-boss-fight save → `from_save` → restored twin resumes the same pulse sequence (the #28 SL3 pattern against the boss) | REQ-007 |
| B9 | validation both-arms ×2: boss without a combat profile → `RoleContractUnmet` (role "boss", exact missing text) / with → ok; `objective_item_id` dangling → `OathObjectiveMissing` by value + Display / valid + None → ok | REQ-008 (contract) |
| B10 | `attack bell-eater` at the roost is refused through the existing non-hostile path (exact line) — the boss is not ambushable around the oath gate | REQ-001/004 |
| B11 | content: beginner world validates; bell_eater profile `{12, 4, 25}` by value; hollow_bell objective = bell_clapper | REQ-008 |
| SV-B | server slice (paused-time): talk mara → swear → n,n,n,u,u → confront → pulses to victory → take clapper → oath Fulfilled in /state, world-alarm announcement EXACT text delivered at the roost + hollowmere notice ABSENT (the #27 both-arms demo rides the new flow) → talk mara returns the fulfilled line | REQ-006 |
| — | preservation: stray suites (#22–#26), announcements unit tests, save/load suites, `npm run build` | REQ-008 |

**JS rows: none** (zero JS changes). **Genuinely uncoverable:** none new —
every refusal/validation arm is reachable with synthetic worlds.

### Risks / decisions
- **R1 — test churn is the deliberate cost** of making the climax real:
  confront-fulfills tests become confront-fight-recover flows. The warden
  tiny-profile (one-round kill) keeps old flows two commands long.
- **R2 — the boss contract is intentionally breaking** for any future
  profile-less boss: loud at `try_new`, the hostile-contract precedent.
- **R3 — an oath with `objective_item_id: None` is valid but
  unfulfillable** — accepted; future objective kinds (kill-the-boss,
  speak-to) arrive as an enum extension, not v1.
- **R4 — flee-then-reconfront resets boss hp** (fresh encounter) —
  correct per the #22 Fled semantics ("enemy intact"); documented so it
  is not re-reported as a bug.
- **R5 — the fulfillment line's channel/component** (Oath/OathCard) is a
  new pairing; implement verifies the datastar feed renders it (existing
  OathCard arms exist from #16 — confirm, no new arms expected).
- **R6 — mutation pins:** `engage_enemy` fn-replace (B1 by-value stats);
  the mid-combat `is_some` guard (B2); the no-fulfill-on-victory pin
  (B3's oath-still-Sworn); the `objective_item_id ==` comparison (B4/B5
  arms); both validation `contains_key`/`is_none` arms (B9); the
  Sworn-status check in the fulfill helper (B5's fulfilled arm).

## Phase 3 — Implement
- Built (to the manifest; workspace `check --all-targets` + strict clippy
  clean):
  - **core:** `engage_enemy(enemy_id, enemy_name, health, attack)` extracted
    from `start_combat` (CombatState build + `CombatStarted` + opening
    round — one combat model, two gates); `confront` rewritten — mid-combat
    arm resolves the next round (the re-entry guard), Sworn + boss →
    `engage_enemy` through the boss-contract profile expect, the
    nothing/unsworn/fulfilled refusal lines kept verbatim, the fulfillment
    branch removed; `take_at` restructured to multi-event with the
    `fulfill_oath_on_recovery(&item_id)` hook appended on success;
    `fulfill_oath_on_recovery` helper (Sworn check → objective match →
    status flip → OathCard line + `OathFulfilled` + the #27 announcement
    loop moved verbatim, expect-invariant intact);
    `OathDefinition.objective_item_id: Option<String>` (serde default,
    doc-commented); `WorldValidationError::OathObjectiveMissing` + Display
    + the validate() check after the announcement-scope loop; the boss arm
    in `validate_entity_contracts` (combat profile required — mirrors the
    hostile contract's shape and wording).
  - **fixtures (compile/contract only):** three `OathDefinition` test
    literals gain `objective_item_id: None`; both warden fixtures
    (`oath_world` + the dialogue world) gain `health 4 / attack 0`
    profiles so the boss contract holds and confront-flows kill in the
    opening strike.
  - **content:** `bell_eater` → `combat = { health = 12, attack = 4,
    xp = 25 }` (stats stay undisclosed); `hollow_bell` →
    `objective_item_id = "bell_clapper"`; roost description reworded
    state-neutral; comments tell the #29 story. Content test T9 re-pinned
    to the new authored values (a stale by-value pin, not a new test).
- Deviations from design (+ reason):
  - **The fulfillment line is generic:** "Your oath is fulfilled."
    (Oath/OathCard) — the design's bell-flavored line would put module
    fiction in engine code (the standing engine/content split). The bell
    flavor arrives one event later via the authored world announcement.
  - `engage_enemy` takes `(health, attack)` rather than `&CombatProfile` —
    the hostile path already carries copied stats in `ResolvedHostile`;
    primitives keep both call sites borrow-clean.
- **Known churn handed to validate (5 red, all anticipated by design R1):**
  core `confront_fulfills_active_oath_and_emits_oath_fulfilled`,
  `confront_when_oath_already_fulfilled_is_refused`,
  `dialogue_reflects_fulfilled_state`,
  `fulfillment_delivers_only_in_scope_announcements`; server
  `beginner_slice_runs_through_command_path` — all assert the OLD
  confront-instantly-fulfills behavior and reshape to the
  fight-then-recover flow per test-plan rows B1–B6/SV-B.

## Inspect (Phase 3.5)
- Lenses run (3 critics, parallel): oath/combat state machine; save/load +
  crafted-save interplay; content/docs/simplification. All three verified
  findings concretely (scratch tests written, run, deleted; worktree
  confirmed restored).
- Findings:
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | MAJOR | The fulfillment beat rendered DOUBLED: the generic OathCard log "Your oath is fulfilled." is byte-identical to what both renderers already print for the typed `OathFulfilled` (datastar + components.js hardcode the same string) — two pixel-identical feed lines at the ticket's climax. | REAL | Dropped the OathCard log; the typed event IS the human line. Event order is now pickup → `OathFulfilled` → announcements (B4 pin adjusted). |
  | 2 | LOW (played-route) | "The The Bell-Eater drops Bell Clapper." — `drop_enemy_inventory`'s hardcoded "The {enemy_name}" doubles an authored article; boss victory put the line on REQ-006's route (reproduced in a scratch run). | REAL | Article-aware subject (skip the prefix when the name starts with "The "/"the "); the #26 stray pin "The Ashen Stray drops Cracked Fang." is byte-identical still. |
  | 3 | MED | A pre-#29 v1 save loads fine but its embedded world lacks `objective_item_id` → a sworn oath becomes silently unfulfillable (confront no longer fulfills; the recovery hook never matches `None`). | REAL | `SAVE_FORMAT_VERSION` bumped to 2 with a doc-comment explaining why — Decision 046's posture is loud refusal, not silent stuckness. Three #28 pins updated (mismatch test now uses `SAVE_FORMAT_VERSION + 1`; Display pin; server SV3c writes version 99). |
  | 4 | MAJOR (docs) | `confront`'s doc-comment still described the old fulfills-on-success contract ("emits … OathFulfilled and marks the oath fulfilled"). | REAL | Rewritten: combat entry + mid-fight press + recovery fulfillment + refusals. |
  | 5 | MIN | "{boss} is already broken; your oath is kept." — post-#29 this arm is reachable ONLY with a LIVING boss (victory removes the placement), making "already broken" false fiction. | REAL | Reworded premise-neutral: "Your oath is already kept; there is no cause to confront {name}." + a reachability comment. |
  | 6 | MIN | `engage_enemy(String, String, u32, u32)` — adjacent swappable u32s; design had planned a profile param; `ResolvedHostile` is exactly the needed shape. | REAL | Signature → `engage_enemy(enemy: ResolvedHostile)`; confront builds one; the struct doc now names both entries. Kills the arg-transposition mutant class structurally. |
  | 7 | MIN | Client intent hint promised the old behavior: "Fulfill your sworn oath." on the confront button (intent.js). | REAL | → "Face your oath's foe." (behavior-honest, generic; no test pinned the hint text). |
  | 8 | MIN (docs) | Stale doc-comments: `Role::Boss` (no contract mention while Hostile documents its identical one), `Entity::combat` ("no combat system reads it yet"), `Command::Confront` ("resolve the boss"), `take_at` (no hook mention — helper carries its own doc, one-line accepted as-is). | REAL | All rewritten in place (take_at covered by the helper's doc + in-body comment). |
- Rejected / accepted-as-is (with verification):
  - **"4 failing tests" reported as HIGH by one critic** — rejected as a
    finding: it is the DECLARED churn set (notes Phase 3), excluded by the
    inspect charter; validate rewrites them. Census re-confirmed: exactly
    4 core + 1 server red, everything else green.
  - **Confront state matrix** — all arms exercised via scratch tests:
    mid-combat arm ≡ attack's (resolves a round even mid-stray-fight —
    intended global-encounter semantics); kill-round-inside-confront ends
    cleanly; refusal arms exact + state-preserving; Sworn+boss opens
    12→8 / 20→16.
  - **Both new expects unreachable from input** — boss profile: validator
    and confront share the same role parse; `validate_entity_contracts`
    runs in try_new AND from_save; `entity.combat` never cleared in-session
    (drop takes inventory only; removal takes placements only). Sworn oath
    id: swear records only the validated designated oath; from_save gates
    it; `world.oaths` is never mutated by gameplay (grep-verified).
  - **Operator sweep over the diff** — clean: `i32::try_from(..).unwrap_or`
    total; `next_pulse_at` saturating; no new indexing/arithmetic on loaded
    values.
  - **Defeat/victory/flee integrity** — defeat leaves boss + inventory +
    Sworn oath intact, retry opens a FRESH full-hp fight from the authored
    profile; victory drops the clapper into the roost, registry entry
    survives (post-victory saves load), oath stays Sworn through the
    funnel; flee-then-reconfront resets boss hp (design R4, documented).
  - **Mid-combat take of a loose objective** (synthetic worlds only) —
    fulfills cleanly mid-fight, no corruption; in the authored world the
    clapper is unreachable until victory (inventories are not perceivable;
    combat clears before the drop lands). Accepted v1 semantics.
  - **Crafted save with the objective already in pack** — inert until a
    take happens (take-only trigger); drop+retake fulfills exactly once;
    no event farming (Fulfilled gates re-fire). Accepted; noted as a
    future-content footgun alongside "swearing while already holding the
    objective never fulfills" (unreachable in beginner; ledgered, not
    fixed).
  - **`enemy_name` desync vs registry in a crafted save** — cosmetic only
    (no lookups keyed on the name). Accepted.
  - **attack/confront mid-combat duplication (~5 lines)** — accepted:
    distinct intent comments beat an indirection.
  - **Numbers vs comments** — all verified (3 rounds, two 4-damage
    returns → 12/20; xp 25 = 5× stray; warden one-strike math).
  - **Datastar/OathCard rendering** — moot after fix 1 (the log line is
    gone); both renderers pre-mapped OathCard anyway (no panic path).
  - **Roost reword** — "crouches beside" pinned nowhere; Nearby carries
    the living boss; clapper/boss descriptions hold the fiction.
- **Stale-docs inventory handed to Phase 5** (the complete phase owns
  docs): `docs/combat-system.md` ("boss/oath confront flow is unchanged" —
  now false; needs a #29 block), `docs/entity-model.md` (boss row lacks
  the profile requirement), `docs/vertical-slice.md` ("boss is a scripted
  placeholder" status claims), `docs/decisions.md` (Decision 045's
  "emitted by confront" → new Decision 047 records the move + closes
  Decision 031's "full combat replaces the boss placeholder" revisit
  trigger).
- Mutation-surface notes handed to validate (beyond the design's R6):
  mid-combat-confront guard (pin hp NOT reset, e.g. 4 not 8); engage_enemy
  via-confront pulse anchor (+2 tick); the version gate at its NEW value
  (found `SAVE_FORMAT_VERSION+1` by value); the article branch in
  `drop_enemy_inventory` (boss line "The Bell-Eater drops Bell Clapper."
  AND stray line both pinned — the two arms); the `!=` objective
  comparison; the boss-contract arm vs a `Role::Boss → Role::Hostile`
  mutant (a boss-only entity fixture).

## Phase 4 — Validate
- **Churn rewrites (the 5 declared reds, all reshaped to the new
  semantics):** core `confront_fulfills_active_oath_and_emits_oath_fulfilled`
  → `confront_with_sworn_oath_starts_the_boss_encounter` (B1 — CombatStarted,
  authored 12/4 stats by value via `engine.state.combat`, opening round
  resolved, NO OathFulfilled, oath stays Sworn);
  `confront_when_oath_already_fulfilled_is_refused` (B6 — loose-objective
  world, living boss + kept oath, the NEW reworded line exact, no combat);
  `dialogue_reflects_fulfilled_state` (fulfills via confront-kill →
  take relic; dialogue_world gained the relic objective);
  `fulfillment_delivers_only_in_scope_announcements` (N9/B4 merged —
  pickup → OathFulfilled → in-scope announcement IN ORDER, no OathCard
  twin, out-of-scope silent) + `announcement_delivery_is_deterministic`
  re-anchored on the take; server slice split (clippy too_many_lines at
  source) into `play_to_boss_victory` helper +
  `beginner_slice_fights_the_boss_to_victory` +
  `beginner_recovery_fulfills_and_rings_the_bell`.
- **New fixtures:** `boss_world(health, attack)`,
  `boss_objective_world(health, attack, xp)` (warden carries `sigil`),
  `loose_objective_world()` (sigil + pebble loose in town).
- **New tests (core 11, content 1, server net +1):** B2 mid-fight confront
  resolves-not-resets (hp 4 not 8, round 2, no second CombatStarted); B2b
  confront-entry pulse anchor (+2 ticks → CombatPulse round 2); B3 victory
  funnel ("The Warden drops sigil." article arm, victory text + xp 25 by
  value, placement removed, objective on the floor, oath STILL Sworn); B5a
  non-objective take inert; B5b unsworn objective take plain; B5c
  refulfillment inert after drop+retake; B6b post-victory confront finds
  nothing; B7 defeat consequences + boss/inventory/oath intact + FRESH
  retry (95 = 99−4, not a stale resume); B8 mid-boss-fight save
  round-trip (byte-identical + twin pulse parity); B9a boss contract both
  arms by value; B9b objective validation both arms + Display; B10
  `attack the warden` refused through the non-hostile path in a
  combat-enabled room; content `beginner_oath_names_the_clapper_objective`.
  Server slice act 1 pins the pulse-driven victory ("You have defeated The
  Bell-Eater. Victory! You gain 25 XP.", "The Bell-Eater drops Bell
  Clapper.", xp 25 / hp 12 / oath Sworn); act 2 pins recovery → fulfilled
  + both announcement arms + the walk back + Mara's "kept your word" line.
- `cargo test --workspace`: **ok — 335 passed, 0 failed** (core 234,
  server 24, content 23, storage 22, protocol 20, datastar 15 — minus
  doc-test stubs), after two test-design fixes of my own (the warden has
  no "warden" alias → `attack the warden`; a 99-attack boss also kills the
  retry's opening round → two-round-fatal 99/10 fixture).
- `node --test tests/*.test.js`: **all passing** (~113ms).
- `npm run build`: built in 87ms.
- `./bin/gate.sh --fast`: **GATE GREEN [fast] — 14/14 PASS** (first run
  caught clippy `too_many_lines` on the monolithic slice rewrite — fixed
  at source by the helper + two-act split, no suppressions). Gates 15–17
  owner-gated with the FULL gate before `/commit`.
- Pre-existing exclusions: none.

## Phase 5 — Complete
- Docs updated:
- Forge capture (aar/failures/rules/decisions):
- Ticket closed:
- Archived:
