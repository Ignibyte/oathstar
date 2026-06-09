# WORK-entity-contracts-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #21 — formalize entity roles into typed contracts: a typed
  `Role` vocabulary for the six current roles (talkable, oath_giver, shopkeeper,
  combatant, boss, fixture); construction-boundary validation that a declared role
  has its minimum metadata where applicable, with a typed error naming
  entity/role/field; convert the two ad-hoc role-string checks (talk_at, confront)
  to typed helpers; keep Mara (talkable+oath_giver) and the Bell-Eater
  (combatant+boss) valid without changing boss progression; docs for the contract
  vocabulary + future code-behind hooks. Implements Decision 004.
- **Intake source:** none.
- **Classification / tier:** work pipeline — one shippable slice (typed vocabulary
  + contract validation + 2 handler conversions + content + docs + tests). Scope
  bounded by "validate currently-used roles" + "avoid a large class hierarchy too
  early"; scripting/shop-economy/combat-AI/NPC-memory/mod-loading deferred. No split.
- **Forge recall (AAR `bcac7916-003c-4286-8c1f-eee6ee61c77b`):** `bulletin-list`
  empty. `knowledge-context` (Plan) surfaced + logged 13 nodes — heavily my own
  captures: ADRs `25864fab` (oath-offer), `c5c90992` (inventory-v1), `938775f7`
  (pack); prevention rules `8b57bdfd` (`PR-claude-validator-length-001`), `69bc1241`;
  failures `b6c76371`/`56433190` (the recurring `too_many_lines` on validate/parse).
  **Load-bearing:** `validate()` grows again here (role contracts) — extract a
  `validate_entity_contracts` helper from the start and run clippy in implement.
- **Ticket:** forge #21 `ef9c9854-e3ed-4f86-a9e6-2bd9439456b4` (existed; not
  re-created). Local doc TICKET-21 `pipeline_spec` linked. Branch
  `codex/oathstar-ticket-21-entity-contracts` (forks `codex/oathstar-tickets-18-20`;
  #18–20 committed at `93989a3`, clean base).
- **EARS requirements reviewed:** REQ-001..006 carried verbatim.

### Current-code anchors (post-#20; clean tree)
| Area | Location |
|---|---|
| `Entity { …, kind: EntityKind, roles: Vec<String>, …, dialogue }` | `crates/oathstar-core/src/lib.rs:93-116` |
| `enum EntityKind { Actor, Fixture }` | `crates/oathstar-core/src/lib.rs:82` |
| `talk_at` ad-hoc check `role == "conversable"` (REQ-003 site) | `crates/oathstar-core/src/lib.rs:820` |
| `confront` ad-hoc check `role == "boss"` (REQ-003 site) | `crates/oathstar-core/src/lib.rs:1155` |
| oath-giver capability rides on `OathDefinition.issuer_id` (#19) | `crates/oathstar-core/src/lib.rs` (oath block) |
| `enum WorldValidationError` (+ Display @240) — add the contract variant | `crates/oathstar-core/src/lib.rs:199,240` |
| `validate()` (@323) + `validate_oaths()` helper (@441, the #19 extraction precedent) | `crates/oathstar-core/src/lib.rs:323,441` |
| Mara `roles=["conversable","shopkeeper"]`; Bell-Eater `roles=["boss","combatant"]` | `modules/beginner/world.toml` |
| Authored contract spec (Shopkeeper/Combatant/Conversable/OathWitness requirements + code-behind hooks) | `docs/entity-model.md` |

### Open questions for Phase 2 (design must settle — the crux of this ticket)
1. **`Role` representation:** an enum parsed on demand from `roles: Vec<String>`
   (keep the Vec as source-of-truth), exposing `Entity::has_role(Role)` + named
   helpers. Confirm strings↔enum mapping and whether to accept the existing names
   (`"conversable"`) or the ticket's (`"talkable"`/`"oath_giver"`) or both.
2. **Role ↔ `EntityKind` coherence:** do talkable/oath_giver/shopkeeper/combatant/
   boss require `EntityKind::Actor`? Is `fixture` a `Role` or just the existing
   `EntityKind::Fixture` (likely the latter, with a contract that an actor-role on
   a Fixture — or vice-versa — is invalid)?
3. **Per-role v1 "minimum metadata where applicable":**
   - **talkable** — capability-only (Actor), or require dialogue? (talk works with
     or without `dialogue` — it is optional from #19, so likely capability-only.)
   - **oath_giver** — require the oath-offer metadata (`dialogue.oath` from #19)
     and/or that some oath's `issuer_id` points to it? This is the contract that
     makes the #19 offer flow well-formed.
   - **combatant / boss** — add optional combat metadata now (health/attack →
     "future-combat-ready", REQ-005) validated if present, or keep role-only with a
     placeholder? Does `boss` imply `combatant`?
   - **shopkeeper** — declared-but-unvalidated (shops out of scope) or a minimal
     greeting field? "Where applicable" suggests no required metadata in v1.
   - **fixture** — kind-coherence only.
4. **Typed error shape:** one `RoleContractUnmet { entity_id, role, field }` variant
   vs per-role variants. Must name entity + role + missing field (REQ-002) and be
   reachable + mutation-killable.
5. **Helper surface:** `is_talkable`/`is_boss`/`is_oath_giver`/… vs a generic
   `has_role(Role)` — keep it minimal; only what `talk_at`/`confront` (and the
   contract validator) actually call.
6. **Docs + a possible `Decision 039`** locking the v1 typed-contract approach.

## Phase 2 — Design

### Approach / architecture
A small typed layer over the existing free-form `Entity.roles: Vec<String>` — a
`Role` enum + capability helpers + one construction-boundary contract validator —
implementing Decision 004's "roles have contracts, validated so broken content
fails early." **Roles stay serialized as strings** (no TOML migration); the typed
vocabulary is *parsed* from them. No class hierarchy, no protocol/UI/parser change.

**Resolved open questions:**
1. **`Role` enum (parsed from tags):** `Role { Talkable, OathGiver, Shopkeeper,
   Combatant, Boss }`. `Role::from_tag(&str) -> Option<Self>` maps canonical
   snake_case tags, accepting `"conversable"` as a **synonym for `talkable`** (the
   #18/#19 content + test fixtures use it). Unknown tags → `None`, **ignored** by
   validation (forward-compatible — existing non-contract tags like `"bystander"`
   don't error). `Role::as_str` for error text. All `pub` + `#[must_use]`
   (oathstar-content uses them; pedantic `must_use_candidate`).
2. **Fixture = `EntityKind`, handled by coherence (not a `Role` variant).** All
   five roles are actor capabilities; the contract validator rejects *any* known
   role on a non-`Actor` entity — so "fixture as appropriate" means a Fixture may
   carry no interaction role.
3. **Per-role v1 contract ("minimum where applicable"):**
   - `talkable` / `shopkeeper` / `combatant` / `boss` → **actor-capability only**
     (require `EntityKind::Actor`; no required metadata — shops/combat are out of
     scope, so their metadata isn't applicable yet).
   - `oath_giver` → the one role with a real required-metadata check: must be named
     as **some oath's `issuer_id`** (the #19 wiring that makes the offer flow
     well-formed). Missing → typed error.
   - `combat` (health) is **optional, future-combat-ready data**, NOT required by
     the combatant contract (so test fixtures carrying `"combatant"` don't break);
     authored on the Bell-Eater to satisfy REQ-005 concretely.
4. **One typed error:** `WorldValidationError::RoleContractUnmet { entity_id, role,
   missing }` — names the entity, the role, and the missing requirement (REQ-002).
   Display: *"entity '{entity_id}' declares role '{role}' but is missing {missing}"*.
5. **Helpers:** `Entity::has_role(Role) -> bool` (`pub`) + a private
   `roles_typed()` iterator used by the validator. `talk_at`/`confront` call
   `has_role(Role::Talkable)` / `has_role(Role::Boss)`.
6. **`Decision 039`** (locks the v1 typed-contract approach) is written at /complete.

**Validation flow:** `validate()` gains `self.validate_entity_contracts()?;` after
`self.validate_oaths()?;` (the #19 extraction precedent) — a focused helper, so
`validate()` stays under the clippy `too_many_lines` ceiling
(`PR-claude-validator-length-001`). The helper iterates entities → for each typed
role: (a) reject if `kind != Actor`; (b) if `OathGiver`, reject if no oath's
`issuer_id` names it. Both branches reachable + mutation-killable.

**Behavior preservation (REQ-003/005):** `has_role(Talkable)` accepts
`"conversable"`, so every existing talk path is byte-identical; `has_role(Boss)`
matches `"boss"`, so confront/boss progression is unchanged. **All existing
fixtures pass the new validator** (audited: model_world npc=combatant/Actor,
fix=Fixture/no-roles, owner=conversable/Actor; oath_world warden=boss/Actor,
bystander tag ignored; dialogue_world mara=conversable/Actor and is an issuer but
does *not* declare `oath_giver` so the oath_giver check doesn't fire; interaction_
/proximity_ engines = actor roles or none). **No protocol/JS/parser change.**

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/lib.rs` | Add `Role` enum + `from_tag`/`as_str` (`pub`, `#[must_use]`); `Entity::has_role` + private `roles_typed`; `CombatProfile{health:u32}` + `Entity.combat: Option<CombatProfile>` (`serde(default)`); `WorldValidationError::RoleContractUnmet{entity_id,role,missing}` + Display arm; `validate_entity_contracts()` helper + call it from `validate()`; convert `talk_at` (`role=="conversable"`→`has_role(Role::Talkable)`) and `confront` (`role=="boss"`→`has_role(Role::Boss)`); update the `entity()` test literal (`combat:None`); new tests. |
| 2 | `crates/oathstar-core/src/awareness.rs` | `combat: None` on the `make_entity` test literal. |
| 3 | `crates/oathstar-content/src/lib.rs` | No struct change (deserializes `Entity` directly); add content tests T8/T9. |
| 4 | `modules/beginner/world.toml` | Mara `roles = ["talkable","oath_giver","shopkeeper"]` (was conversable/shopkeeper); Bell-Eater + `combat = { health = 12 }` (roles unchanged). |
| 5 | `docs/entity-model.md` | Flip the v1-implemented note; add a "Role Contracts (v1)" section defining the typed vocabulary + each role's v1 contract; note code-behind hooks attach later. |
| 6 | `docs/mechanics-and-systems.md` | Flip the "role-contract validation … not yet implemented" note to implemented (#21). |

No change to `oathstar-protocol`, `src/` (JS), or the parser.

### Regression Test Plan
| # | Test (location) | Proves |
|---|---|---|
| T1 | `role_from_tag_maps_known_and_synonym_tags` (core) — each known tag + `"conversable"` synonym → its `Role`; unknown → `None` | REQ-003 (kills each `from_tag` arm) |
| T2 | `entity_has_role_reflects_tags` (core) — `["conversable"]` ⇒ `has_role(Talkable)` true, `has_role(Boss)` false | REQ-003 |
| T3 | `validate_rejects_oath_giver_without_issuing_oath` (core) — Actor with `oath_giver` and no oath naming it ⇒ `RoleContractUnmet{role:"oath_giver", missing names issuer}`; an oath_giver *with* an issuing oath validates | REQ-001/002 |
| T4 | `validate_rejects_actor_role_on_fixture` (core) — Fixture with `combatant` ⇒ `RoleContractUnmet{entity_id, role:"combatant", missing names Actor}` | REQ-001/002 (coherence) |
| T5 | `role_contract_error_names_entity_role_and_field` (core) — the error Display contains the entity id, role, and missing text | REQ-002 |
| T6 | `talk_uses_typed_talkable_role` (core) — an NPC tagged `"talkable"` (not `"conversable"`) is talkable ⇒ talk works via the typed helper | REQ-003 |
| T7 | existing `confront_*` + `talk_*` suites stay green (now routed through `has_role`) | REQ-003/005 (preserve) |
| T8 | `beginner_mara_is_talkable_and_oath_giver` (content) — `load_beginner_world`: Mara `has_role(Talkable)` && `has_role(OathGiver)`, world validates | REQ-004 |
| T9 | `beginner_bell_eater_is_combatant_boss_with_combat` (content) — Bell-Eater `has_role(Combatant)` && `has_role(Boss)`, `combat == Some(health 12)`, world validates | REQ-005 |
| T10 | existing server smoke `beginner_slice_runs_through_command_path` (talk→swear→confront) stays green | REQ-005 (progression unchanged) |
| T11 | `cargo test --workspace` + `node --test` + `npm run build` + `bin/gate.sh --fast` green | all REQ regression |

REQ-006 (docs) verified by review (the doc updates, files 5–6). **Genuinely
uncoverable:** none new — every `Role`/`has_role`/validator branch is reachable;
`CombatProfile.health` is asserted by T9.

### Risks / decisions (reversible-but-load-bearing)
- **The new validator runs on every world (all test fixtures + content).** Audited
  above: none break. The biggest risk is an overlooked fixture — `validate` runs in
  `try_new`, so a missed case fails fast at test time (caught in validate).
- **Unknown role tags are ignored, not rejected.** Forward-compatible (future roles
  + non-contract attribute tags), but it means typos in role tags won't be caught
  in v1 — acceptable per "validate currently-used roles"; a future ticket can add
  strict-tag mode.
- **`combat` is optional, not contract-required.** Satisfies "future-combat-ready"
  (REQ-005) and demonstrates the optional-typed-metadata pattern without breaking
  the `"combatant"` test fixtures or inventing a combat system.
- **Mara's `"conversable"` → `"talkable"` rename** is behavior-preserving via the
  `from_tag` synonym; test fixtures keep `"conversable"` and still resolve to
  `Talkable`.
- **`validate_entity_contracts` is extracted up front** (the #19/#20 `too_many_lines`
  lesson); clippy runs in implement.

## Phase 3 — Implement
- **Built (production + content + docs, per the manifest):**
  - `oathstar-core/src/lib.rs`: `Role` enum (`Talkable/OathGiver/Shopkeeper/Combatant/Boss`)
    + `from_tag` (`"conversable"`→`Talkable` synonym; unknown→`None`) + `as_str`
    (both `pub #[must_use]`); `Entity::has_role` (`pub`) + private `roles_typed`;
    `CombatProfile{health:u32}` + `Entity.combat: Option<CombatProfile>` (`serde(default)`);
    `WorldValidationError::RoleContractUnmet{entity_id,role,missing}` + Display arm;
    `validate_entity_contracts()` (actor-coherence + oath_giver-must-issue) called
    from `validate()` after `validate_oaths()`; `talk_at` → `has_role(Role::Talkable)`,
    `confront` → `has_role(Role::Boss)`; `entity()` literal `combat:None`.
  - `oathstar-core/src/awareness.rs`: `make_entity` literal `combat:None`.
  - `modules/beginner/world.toml`: Mara `roles=["talkable","oath_giver","shopkeeper"]`;
    Bell-Eater `combat = { health = 12 }`.
  - `docs/entity-model.md`: flipped the v1 note; added the "Role Contracts (v1)"
    section (typed vocabulary table + per-role contract + how code-behind hooks
    attach later). `docs/mechanics-and-systems.md`: flipped the "role-contract
    validation … not yet implemented" note.
  - **No `oathstar-protocol`, `src/` (JS), or parser change.**
- **Verified:** `cargo fmt` clean; **`cargo clippy --workspace --all-targets` GREEN
  on the first run** (the `validate_entity_contracts` extraction + `#[must_use]`
  preempted `too_many_lines` + `must_use_candidate`); **`oathstar-core` (130) +
  `oathstar-content` (16) tests pass** — every existing fixture clears the new
  contract validator (no breakage), and the beginner world (Mara talkable/oath_giver,
  Bell-Eater combat) loads + validates.
- **Deviations from design (+ reason):**
  1. **Test work deferred to Phase 4** (per phase rules) — only the mechanical
     `combat:None` literal fixes were done. No new tests written yet.
  2. **Zero clippy findings this time** — unlike #19/#20, clippy was clean on the
     first run because the per-concern `validate_entity_contracts` helper and the
     `#[must_use]` annotations were written up front (`PR-claude-validator-length-001`
     applied proactively). The recurring `too_many_lines` class did not recur.
  3. **No fixture breakage** — the design's audit (all existing test worlds pass the
     new validator) is confirmed empirically by the 130 core tests staying green;
     the `conversable` synonym kept every talk path byte-identical.
  4. **`Entity.combat` ripple:** 2 test-only literals (`entity()` in lib.rs,
     `make_entity` in awareness.rs); the content loader needed no change.

## Inspect (Phase 3.5)
- **Lenses run** (4 parallel critics over the #21 diff only — `git diff HEAD` = #21):
  Correctness · 100%-mutation-MSI readiness · serde/state-integrity & wire ·
  simplification/strict-clippy.
- **No code defects.** The diff was clippy-clean before inspect because clippy ran
  in implement (the third consecutive run where `PR-claude-validator-length-001`
  prevented a `too_many_lines` recurrence — `validate_entity_contracts` extracted +
  `#[must_use]` added up front). Findings:

  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | — | **Correctness:** all REQs hold; `has_role(Role::Talkable)` accepts the `"conversable"` synonym (talk byte-identical), `has_role(Role::Boss)` matches `"boss"` (confront/progression unchanged); validator coherence + oath_giver checks correct, deterministic (BTreeMap), no input-path panics; every existing fixture + content entity passes (Mara oath_giver↔issuer, Bell-Eater combatant/boss). | **Clean.** | None. |
  | 2 | — | **Serde/state/wire:** `Entity.combat` is `serde(default)` (old TOML loads); `Role` is not serialized (the `roles: Vec<String>` shape is unchanged); `oathstar-protocol` untouched; `combat` never leaks to a snapshot; Mara's `conversable`→`talkable` rename is backward-compatible via the synonym and no test asserts her exact role list. | **Clean.** | None. |
  | 3 | — | **Clippy:** GREEN (forced-rebuild confirmed, 0 warnings). `from_tag`/`as_str`/`has_role` `#[must_use]`; the two `validate` guards are independent (not collapsible); clones are borrow-necessary. The ad-hoc `== "conversable"`/`== "boss"` handler checks are fully removed (the remaining `"boss"` greps are a test comment + a #7 content test on raw TOML — not #21 handler code). | **Clean.** | None. |
  | 4 | LOW | The `from_tag` closure `|tag| Role::from_tag(tag)` (`lib.rs:190`) looked like a `redundant_closure` candidate. | **Rejected** — the critic proved `.filter_map(Role::from_tag)` fails to compile (`&String`→`&str` deref coercion the fn-pointer can't do); clippy correctly stays silent. | None — leave as-is. |
  | 5 | MED (test) | **MSI gap (Phase-4 test requirement, not a code defect):** `Role::as_str` arms for **Shopkeeper / Boss / Talkable** are only reached via error paths the planned tests (T3 oath_giver, T4 combatant) don't all exercise → those arms' value-mutants would survive. | **Real, but a test gap.** | **Carry-forward:** Phase 4 adds a direct `role_as_str_returns_each_tag` unit test (asserts all 5 by value). |

- **Carry-forward to Phase 4 (the mutation critic's load-bearing specs — write tests
  to these exact shapes):** T1 `from_tag` asserts each arm **by value** incl. *both*
  `"talkable"` and `"conversable"`→Talkable + unknown→None; T2 `has_role` on a
  **multi-role** entity (true *and* false — kills `.any`→`.all`); T3 oath_giver
  matrix (0 / 1-matching / multi-with-one-matching / multi-none-matching); T4
  rejects Fixture+role **and** accepts Actor+role (kills the `!=`→`==` inversion);
  **T-new `role_as_str` asserts all 5 arms by value**; T9 asserts Bell-Eater
  `combat == Some(CombatProfile{health:12})` by value.
- **Out-of-scope note (NOT #21):** the working tree also contains an uncommitted
  edit to `docs/planning/tickets/open/TICKET-22-…md` (adds combat-modal REQ-008/009/010
  + scope) — **Codex's #22 backlog planning, not part of this pipeline.** Left
  untouched (not mine to revert); flagged so it isn't bundled as #21 work.
- **Net:** 0 code defects; 0 fixes; clippy + fmt + core(130)/content(16) green.

## Phase 4 — Validate
- **Tests added (+9, ≥1 per AC + the inspect critic's MSI specs):**
  - `oathstar-core` (+7): T1 `role_from_tag_maps_known_and_synonym_tags` (each tag +
    `conversable` synonym + unknown, by value); `role_as_str_returns_each_canonical_tag`
    (**all 5 arms** — the inspect carry-forward); T2 `entity_has_role_reflects_multiple_tags`
    (multi-role true+false, kills `.any`→`.all`); T6 `talk_resolves_talkable_via_typed_role_tag`
    (the `"talkable"` tag, not `"conversable"`); T3 `validate_oath_giver_contract_matrix`
    (0 / 1 / multi-with-match / multi-none); T4 `validate_rejects_role_on_fixture_and_accepts_on_actor`
    (both branches); T5 `role_contract_error_names_entity_role_and_missing`.
  - `oathstar-content` (+2): T8 `beginner_mara_is_talkable_and_oath_giver`; T9
    `beginner_bell_eater_is_combatant_boss_with_combat` (`combat == Some(CombatProfile{health:12})`
    by value).
- **`cargo test --workspace`:** ✅ green — core **137**, content **18**, protocol 9,
  datastar 11, server 13, storage 20; 0 failed.
- **`node --test tests/*.test.js`:** ✅ green — 32 pass, 0 fail (JS unchanged for #21).
- **`bin/gate.sh --fast`:** ✅ `GATE GREEN [fast]` — 14/14.
- **`npm run build`:** ✅ clean (vite, 72ms) — no UI change, builds fine.
- **Coverage (gate:15 floor 94):** core lib.rs **99.46% regions / 99.48% lines**,
  content **99.15%** — well above the floor.
  - **Tooling note:** the first `cargo llvm-cov -p oathstar-core -p oathstar-content`
    reported a false-low 79% (stale instrumentation after intervening non-instrumented
    `cargo test`/`clippy` builds). `cargo llvm-cov clean --workspace` + a workspace run
    gave the true 99.46% — recorded as a process lesson for /complete.
- **FULL gate (gates 15–17: coverage + cargo-mutants 100% MSI):** deferred to `/commit`
  per the ticket (`--fast`). Tests authored to the inspect critic's specs (by-value
  asserts, multi-role `has_role`, the oath_giver matrix, all 5 `as_str` arms) so 100%
  MSI is expected.
- **Pre-existing exclusions:** none — no pre-existing failures; nothing skipped.
  (Out-of-scope: the uncommitted `TICKET-22` backlog edit is Codex's #22 planning, not
  this pipeline.)

## Phase 5 — Complete
- Docs updated:
- Forge capture (aar/failures/rules/decisions):
- Ticket closed:
- Archived:
