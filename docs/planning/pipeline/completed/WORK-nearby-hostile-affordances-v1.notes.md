# WORK-nearby-hostile-affordances-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #23 — Nearby Hostile Affordances + Entity Inspection. Branch `codex/oathstar-ticket-23-nearby-hostile-affordances`, stacked on `b588b85` (combat v1, full gate was GREEN 17/17). Server-authoritative additive `NearbySnapshot` affordances + a generic entity detail dialog. Auto-approve through complete; **stop before commit**; validate runs `gate --fast` (full gate/commit need Codex). Don't touch untracked `assets/tilesets/` or `bin/generate_oathstar_tileset.py`. #24 owns the two-phase combat loop — out of scope.
- **Intake source:** none (ticket pre-existed; forge id minted this run).
- **Classification / tier:** Work pipeline, **one slice** (protocol additive + a `room_snapshot` computation + a minimal disclosure fixture + JS view-models + an entity-detail dialog). Smaller than #22 — no new command, no new engine state machine; it's additive snapshot fields + pure view-models + one new `<dialog>`. Not split.
- **Base verified (pre-flight):** branch `codex/oathstar-ticket-23-nearby-hostile-affordances`, HEAD `b588b85` "Add combat encounter v1"; working tree clean except the untracked tileset guardrails; no active pipeline; forge up; no bulletins.
- **Forge:** ticket `e8eaca33-1701-4009-93c6-e63007f700d7` (#23) minted → in-progress/plan; local doc frontmatter updated (ticket id + pipeline_spec). AAR `ca38d7cb-9b35-4674-b2df-11da22c455a4` opened; Plan knowledge logged (13 surfacings — combat-v1 ADR `2e4d969e`, entity-contracts `9d063c49`, spatial-awareness ADR `939d18d0`, the additive/assert-by-value lessons, PR-claude-validator-length + PR-claude-expect-invariants).
- **Current-code anchor map (Explore digest — `file:line` for Design):**
  - `crates/oathstar-protocol/src/lib.rs`: `NearbySnapshot` @ ~117–132 (`id/name/kind/distance/proximity/interactable`, camelCase) — add `hostile`/`attackable`/`attack_command` + a disclosure object here. `CombatSnapshot`/`CombatantSnapshot` @ ~198–223 (skip-if pattern). `OathStatus`/`CombatOutcome` @ ~161–179 (closed snake_case enums — **no Unknown variant anywhere**; use `Option` for unknown). `GameSnapshot.oath/combat` use `#[serde(default, skip_serializing_if = "Option::is_none")]`.
  - `crates/oathstar-core/src/lib.rs`: `room_snapshot` @ ~1660–1689 — the `awareness::perceive(...).map(|thing| NearbySnapshot {…})` closure is **the compute site**; `self` (&Engine) → `self.world.entities.get(&thing.id)` reaches the `Entity` for `has_role(Role::Hostile)` + `combat`, and `room.combat_enabled` is in scope. `thing.proximity.is_interactable()` gives reach. `Entity` @ ~101–128 (`roles`/`combat: Option<CombatProfile>`), `Role::Hostile` @ ~150–165, `CombatProfile { health, #[serde(default)] attack }` @ ~132–142, `RoomDefinition.combat_enabled` @ ~41–65.
  - `crates/oathstar-core/src/awareness.rs`: `Awareness { id, room_id, name, description, kind, distance, proximity }` @ ~189–208 (carries `id` + `description`, dropped by `room_snapshot` today). `perceive` @ ~320–329. The id survives to the closure → re-lookup is clean; no need to thread roles/combat through `perceive`.
  - JS `src/client/snapshot.js`: `toNearby` @ ~72–83, `toNearbyItem` @ ~103–136 (builds `{name,kind,interactable,detail,command,actions[]}`; adds Talk for actors / Take for items) — add `hostile`/`attackable` flags + an Attack action here; add a `toEntityDetail`-style view-model.
  - JS `src/client-app.js`: `actionCard` @ ~571–602 (card + action buttons → `runCommand`), `renderMenu` @ ~411–455 (Nearby panel), `renderBattle` @ ~480–509 + `el.battleModal` + Attack-button wiring (reuse for Nearby Attack → battle modal), `openRoomModal` @ ~328–356 + `#room-modal` open/close/backdrop seam (mirror for the entity-detail dialog).
  - `index.html`: `#room-modal` @ ~254–269, `#battle-modal` @ ~275–297 (`<dialog>` patterns to mirror); `.entity-card`/`.entity-actions` Nearby card markup. `styles.css`: `.entity-card` @ ~563–596 (add hostile/attackable style hooks), modal styles.
  - Content `modules/beginner/world.toml`: `ashen_stray` (roles `["combatant","hostile"]`, `combat = { health = 9, attack = 3 }`) in `ashen_road` (`combat_enabled`, rooms.toml) — the visible/attackable demo. `mara` non-hostile actor. The Bell-Eater (`["boss","combatant"]`, not hostile) — the not-attackable demo. **No disclosure/visibility field on `CombatProfile` today.**
- **EARS requirements reviewed:** REQ-001..007 (verbatim in the spec). Engine/protocol REQ-001/002/003/005 (Rust/JS tests), REQ-004/006 (JS/browser smoke), REQ-007 preservation (gate). Every REQ gets ≥1 test in the Phase-2 plan.

### Open design questions (for Phase 2 — Planner does NOT decide these)
1. **Disclosure shape + source.** Where the "disclosed stats" come from: a `disclose_stats: bool` (or richer) on `CombatProfile` (authored), vs computed-in-snapshot, vs a per-entity disclosure. The `NearbySnapshot` disclosure field shape: a nested `Option<NearbyCombatSnapshot { health, max_health, attack }>` vs flat optionals. Field names per repo style. Absent = unknown.
2. **Minimal visible+hidden fixture.** To prove BOTH renderings: ashen_stray discloses (visible); a hidden-stats hostile shows unknown. Whether to author a shipped "shrouded" hostile, flag an existing one's stats hidden, or keep the hidden case test-only — smallest option that proves it without a bestiary.
3. **Detail view-model fields.** What the generic dialog shows (name/kind/`description`?/hostile/attackable/disclosed-or-unknown stats). NOTE: `NearbySnapshot` has no `description` today (Awareness does, room_snapshot drops it) — decide whether to add `description` to `NearbySnapshot` for the detail view or keep it minimal.
3b. The detail dialog reads the snapshot entry the client already holds (no `look`/fetch, no command — REQ-004). Confirm the click→open seam (card click vs an explicit "Inspect" affordance) without breaking the existing action-button clicks.
4. **Not-attackable wording / reason.** `attackable: false` can mean too-far OR non-combat-area; the client knows `proximity` + `interactable` but not the area reason. Decide whether to expose a reason (e.g. a `not_attackable_reason`) or render a generic quiet "can't fight here / too far" from `interactable` + `hostile`. REQ-002 needs the state flagged clearly, not necessarily the reason.
5. **`attack <name>` target text.** Which name the command uses (display `name` vs an id/alias) so it resolves through the #22 `awareness::resolve_target` (the battle path matches by name/alias) — keep consistent with how Talk/Take build their commands (they use the display name).

## Phase 2 — Design

### Approach / architecture (the 5 open questions, resolved)
Server-authoritative: the engine computes the affordances in `room_snapshot`'s
existing `awareness::perceive(...).map(|thing| …)` closure (it has `&self` + `room`,
so `self.world.entities.get(&thing.id)` reaches the `Entity` for `has_role` + `combat`,
and `room.combat_enabled` is in scope). The client only reads the new fields.

1. **Disclosure shape + source (Q1) + the two new fields.** Two additive optional
   objects on `NearbySnapshot`, both `#[serde(default, skip_serializing_if = "Option::is_none")]`
   (no `is_false` bool-skip helper → dodges clippy `trivially_copy_pass_by_ref` under
   the no-suppressions gate; mirrors how `CombatSnapshot` groups data):
   - `threat: Option<NearbyThreatSnapshot>` — **present iff the entity is hostile**
     (`has_role(Role::Hostile)`); so "hostile" is conveyed by presence (no `hostile`
     bool). `NearbyThreatSnapshot { attackable: bool, attack_command: Option<String> (skip-none) }`.
   - `stats: Option<NearbyStatsSnapshot>` — **present iff the entity has a `CombatProfile`**
     (any combatant, hostile or not). `NearbyStatsSnapshot { health: Option<u32>, max_health: Option<u32>, attack: Option<u32> }` (all camelCase, all skip-none). Inner `Some` = **disclosed**, `None` = **unknown** (`Option` = unknown; no enum-unknown precedent in the codebase). Disclosure source: a new authored `#[serde(default)] CombatProfile.disclose_stats: bool` — `true` → inner `Some(profile.health/attack)`; `false` → inner `None`.
   - This is a deviation from the request's *suggested* flat `hostile`/`attackable` bools, taken under "use repo style"; see risk **R3**.
2. **Minimal visible + hidden fixture (Q2) — zero new mobs.** `ashen_stray` gets
   `disclose_stats = true` (the shipped **visible** demo). The **Bell-Eater** already
   has `combat = { health = 12 }` and no disclose flag (default `false`) and is a
   combatant → `stats: Some { health: None, … }` → the shipped **hidden/unknown**
   demo, inspectable in its roost. No new content beyond one flag on `ashen_stray`.
3. **Detail view-model fields (Q3) — no `description` added.** The generic detail
   dialog shows `name`, `kind`, the hostile/attackable status, and the combat stats
   (disclosed values or "unknown"). `NearbySnapshot` does **not** gain `description`
   (it would bloat every entry; REQ-005 is about combat stats; richer inspection /
   `look`-detail is a future ticket). The detail view-model `toEntityDetail(entry)`
   reads only the `NearbySnapshot` entry the client already holds — **no fetch, no
   command, no mutation** (REQ-004).
4. **Not-attackable wording / reason (Q4) — derived client-side, no server reason.**
   `attackable = hostile (⟹ Actor by the #21 contract) & proximity.is_interactable() & room.combat_enabled`.
   The client derives the status from the fields already present (`threat` +
   `interactable` + `threat.attackable`): hostile & attackable → enabled Attack;
   hostile & !attackable & !interactable → "too far to attack"; hostile & !attackable
   & interactable → "can't fight here" (combat disabled). No `reason` field needed.
5. **Attack target text (Q5) — server-built, display name.** `attack_command =
   Some(format!("attack {}", thing.name))` only when `attackable` — the server builds
   it (the client doesn't construct the verb), using the display `name` that the #22
   `awareness::resolve_target` matches by name/alias (e.g. `attack Ashen Stray`). The
   client runs `entry.threat.attackCommand` verbatim; `renderBattle` (unchanged) opens
   the #22 battle modal off the response snapshot (REQ-006).

**Compute (room_snapshot closure):** `let entity = self.world.entities.get(&thing.id);`
→ `threat = entity.filter(|e| e.has_role(Role::Hostile)).map(|_| { let attackable = thing.proximity.is_interactable() && room.combat_enabled; NearbyThreatSnapshot { attack_command: attackable.then(|| format!("attack {}", thing.name)), attackable } })`;
`stats = entity.and_then(|e| e.combat.as_ref()).map(|p| { let d = p.disclose_stats; NearbyStatsSnapshot { health: d.then_some(p.health), max_health: d.then_some(p.health), attack: d.then_some(p.attack) } })`.
The "actor" half of attackable is implicit (`has_role(Hostile)` ⟹ Actor by contract), so no redundant always-true `kind == Actor` check (avoids an equivalent mutant — risk R4).

**Client (pure view-model / thin glue, Decision 032):** `toNearbyItem` gains
`hostile`/`attackable` flags, a `combatStatus` label, and an Attack action (only when
attackable, command = `entry.threat.attackCommand`). The card's name/kind row becomes
a clickable **inspect** control opening `#entity-modal` via a new `openEntityDetail`
(mirrors `openRoomModal`'s `showModal`/`close`/backdrop seam); action buttons stay
command-senders (separate buttons, no propagation conflict). New pure `toEntityDetail`
builds the dialog model. Combat events/battle modal/feed unchanged.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-protocol/src/lib.rs` | `NearbySnapshot`: add `threat: Option<NearbyThreatSnapshot>` + `stats: Option<NearbyStatsSnapshot>` (skip-none). New `NearbyThreatSnapshot { attackable: bool, attack_command: Option<String> (skip-none) }` + `NearbyStatsSnapshot { health/max_health/attack: Option<u32> (skip-none) }` (camelCase). Update the `nearby(id)` test helper + any `NearbySnapshot {…}` literal with `threat: None, stats: None`. |
| 2 | `crates/oathstar-core/src/lib.rs` | `CombatProfile`: add `#[serde(default)] disclose_stats: bool`. `room_snapshot` closure: compute `threat` + `stats` (entity lookup by `thing.id`; reuse `has_role(Role::Hostile)`, `room.combat_enabled`, `thing.proximity.is_interactable()`). Update `CombatProfile {…}` literals in the #22 combat tests (`combat_world`, titan) with `disclose_stats`. |
| 3 | `modules/beginner/world.toml` | `ashen_stray`: `combat = { health = 9, attack = 3, disclose_stats = true }` (visible demo). Bell-Eater unchanged (hidden demo). |
| 4 | `src/client/snapshot.js` | `toNearbyItem`: add `hostile`/`attackable`/`combatStatus` + an Attack action (when attackable, `command = entry.threat.attackCommand`). New pure `toEntityDetail(entry)` (name/kind/hostile/attackable/statusLabel/stats{isCombatant,disclosed,health,maxHealth,attack}). |
| 5 | `src/client-app.js` | `actionCard`: clickable inspect control on the name/kind row → `openEntityDetail(item)`; hostile/attackable card class hooks; Attack button when attackable. New `openEntityDetail` + `el.entityModal*` refs + backdrop-close. |
| 6 | `index.html` | `<dialog id="entity-modal">` (mirror `#room-modal`): title, kind line, hostile/attackable status, a stats section (health/maxHealth/attack or "unknown"). |
| 7 | `styles.css` | `#entity-modal` styles + `.entity-card.hostile` / `.attackable` flags + a quiet `.combat-status` chip. |
| 8 | `tests/nearby-affordances.test.js` (new) | JS view-model tests (J1–J6). |
| 9 | docs (→ Complete) | ui-design / entity-model (#23 note → implemented) / spatial-awareness (NearbySnapshot threat/stats) / decisions.md (Decision 041). |

### Regression Test Plan
≥1 per EARS REQ. Rust = `#[cfg(test)]` in the owning crate; JS = `node --test`.

| # | Test | Proves |
|---|---|---|
| N1 | room_snapshot: hostile in a combat_enabled room, interactable → `threat = Some{ attackable: true, attack_command: Some("attack <name>") }` (by value) | REQ-001 |
| N2 | room_snapshot: hostile visible-but-not-interactable (distance-2 pattern) → `threat = Some{ attackable: false, attack_command: None }` | REQ-002 |
| N2b | room_snapshot: hostile interactable but room NOT combat_enabled → `threat = Some{ attackable: false, attack_command: None }` | REQ-002 |
| N3 | room_snapshot: non-hostile actor (talkable) → `threat = None` | REQ-003 |
| N4 | room_snapshot: hostile with `disclose_stats = true` → `stats = Some{ health: Some(9), max_health: Some(9), attack: Some(3) }` | REQ-005 |
| N5 | room_snapshot: hostile with `disclose_stats = false` → `stats = Some{ health: None, max_health: None, attack: None }` (unknown) | REQ-005 |
| N6 | room_snapshot: non-combatant (no CombatProfile) → `stats = None` | REQ-005 |
| N7 | integration: take the server's `attack_command` for a nearby hostile and run it → combat starts (`snapshot.combat` Some) — proves the command is canonical + resolvable | REQ-001/006 |
| N8 | serde additive (core): a non-hostile non-combatant NearbySnapshot omits `threat`+`stats` from JSON; engine snapshot still builds | REQ-007 |
| P1 | protocol: `NearbyThreatSnapshot`/`NearbyStatsSnapshot` camelCase (`maxHealth`, `attackCommand`) round-trip; `threat`/`stats` omitted when None; inner stat None omitted | REQ-001/005 |
| P2 | protocol: old `NearbySnapshot` JSON without `threat`/`stats` deserializes (both None) | REQ-007 |
| CT1 | content: beginner `ashen_stray.combat.disclose_stats == true` (visible demo) | REQ-005 |
| CT2 | content: beginner `bell_eater.combat.disclose_stats == false` (hidden demo) + world still validates | REQ-005/007 |
| J1 | `toNearbyItem` hostile+attackable entry → `hostile:true`, `attackable:true`, an Attack action whose `command === entry.threat.attackCommand` | REQ-001/006 |
| J2 | `toNearbyItem` hostile+not-attackable (too-far AND non-combat variants) → `hostile:true`, `attackable:false`, NO enabled Attack action, a `combatStatus` flag | REQ-002 |
| J3 | `toNearbyItem` non-hostile → `hostile:false`, no Attack action; Talk/Take/Look unchanged | REQ-003 |
| J4 | `toEntityDetail`: disclosed hostile → `stats {disclosed:true, health:9, maxHealth:9, attack:3}`; hidden hostile → `{disclosed:false, health:null,…}` (unknown); non-combatant → `stats` isCombatant:false/null | REQ-005 |
| J5 | `toEntityDetail` is pure — returns data only, sends no command / mutates nothing (no fetch/runCommand) | REQ-004 |
| J6 | existing `tests/*.test.js` (toNearby/toComponent/toBattle/look/talk/take) stay green | REQ-007 |

**Browser-smoke (REQ-004/006, not node-coverable — jsdom lacks `<dialog>`):** clicking a Nearby card opens `#entity-modal` with the disclosed/unknown stats and sends no command; clicking Attack sends `attack <name>` and the #22 battle modal opens. The pure view-models (`toNearbyItem`/`toEntityDetail`) are node-tested above; the `showModal`/`close` + click-send glue is the thin smoke-verified seam (same status as `#room-modal`/`#battle-modal`).

### Risks / decisions
- **R1 — `CombatProfile.disclose_stats` addition** breaks `CombatProfile {…}` struct literals in the #22 core tests (`combat_world`, titan) + the content Bell-Eater assertion. Compile-caught; update with `disclose_stats: false` (true where the test wants disclosure).
- **R2 — two new `NearbySnapshot` fields** break the protocol `nearby(id)` test helper + any literal. Compile-caught; add `threat: None, stats: None`.
- **R3 — design deviation (load-bearing).** Used `threat: Option<…>` (presence = hostile) + `stats: Option<…>` instead of the request's *suggested* flat `hostile`/`attackable` bools. Rationale: serde-clean (no `is_false` skip helper → avoids clippy `trivially_copy_pass_by_ref`, which the no-suppressions gate forbids `#[allow]`-ing), groups affordances like `CombatSnapshot`, byte-identical for non-hostiles, and a free shipped hidden-stats demo (Bell-Eater) with zero new mobs. Same semantic content the request asked for. Flagged for Codex review.
- **R4 — dropped the redundant `kind == Actor` check** in attackable (hostile ⟹ Actor by the #21 contract) to avoid an always-true equivalent mutant; documented.
- **R5 — card click vs button click:** the inspect-open lives on a dedicated name/kind control, action buttons stay separate command-senders — no event-propagation conflict. Browser-smoke.
- **R6 — `attack_command` uses the display `name`** (matches how #22 `resolve_target` matches + how Talk/Take build commands); N7 integration-tests that the emitted command actually starts combat.
- **R7 — validate runs `--fast`** (no coverage/mutation gate this ticket, per the owner); still designed for 100% MSI + assert-by-value so Codex's pre-commit full gate stays green.
- **Decision (→ decisions.md Decision 041 at Complete):** Nearby combat affordances are server-authored on `NearbySnapshot` as `threat` (presence = hostile; `attackable` = hostile & interactable & combat_enabled; server-built `attack_command`) + `stats` (combat disclosure; `Option` = unknown, gated by `CombatProfile.disclose_stats`); the generic entity-detail dialog renders the snapshot entry with no command/mutation; the client never infers.

## Phase 3 — Implement
- **Built (manifest rows 1–7; tests row 8 → Validate, docs row 9 → Complete):**
  - **protocol** (`oathstar-protocol/src/lib.rs`): `NearbySnapshot` gains `threat: Option<NearbyThreatSnapshot>` + `stats: Option<NearbyStatsSnapshot>` (both skip-none); `NearbyThreatSnapshot { attackable: bool, attack_command: Option<String> (skip-none) }`; `NearbyStatsSnapshot { health/max_health/attack: Option<u32> (skip-none) }` (camelCase). Updated the `nearby(id)` test helper with `threat: None, stats: None`.
  - **core** (`oathstar-core/src/lib.rs`): `CombatProfile.disclose_stats: bool` (`#[serde(default)]`); `room_snapshot` closure now looks up the entity by `thing.id` and computes `threat` (Some iff `has_role(Role::Hostile)`; `attackable = thing.proximity.is_interactable() && room.combat_enabled`; `attack_command = attackable.then(|| format!("attack {}", thing.name))`) + `stats` (Some iff `entity.combat`; inner `disclose_stats.then_some(...)`). No redundant `kind == Actor` check (hostile ⟹ Actor by contract). Import extended with the two new types.
  - **content** (`modules/beginner/world.toml`): `ashen_stray.combat` gains `disclose_stats = true` (visible demo); Bell-Eater unchanged (hidden demo).
  - **JS** (`src/client/snapshot.js`): `toNearbyItem` adds `hostile`/`attackable`/`combatStatus` + an Attack action (only when attackable, command = `entry.threat.attackCommand`, `variant: "danger"`) + a precomputed `entityDetail`; new `combatStatusLabel` helper; new exported pure `toEntityDetail(entry)` (name/kind/hostile/attackable/statusLabel/stats{isCombatant,disclosed,health,maxHealth,attack}).
  - **JS glue** (`src/client-app.js`): `actionCard` — hostile/attackable card classes, the name/kind row is now an inspect `<button>` → `openEntityDetail(item.entityDetail)`, a quiet `combat-status` chip, and `action-<variant>` class on action buttons; new `openEntityDetail(detail)` (renders into `#entity-modal`, `showModal` guarded); `el.entityModal*` refs + backdrop-close binding.
  - **HTML/CSS**: `index.html` `<dialog id="entity-modal">` (mirrors `#room-modal`, `<form method="dialog">`): title, kind, status, stats container. `styles.css`: `.entity-main` button reset + hover; `.entity-card.hostile`/`.attackable`, `.combat-status.is-attackable`/`.is-quiet`, `.entity-actions .action-danger`, `#entity-modal` + `.entity-stat` styles.
- **Compile/check (this phase):** `cargo fmt` clean; **`cargo clippy --workspace --all-targets` GREEN** (after splitting two new struct doc-comments to satisfy nursery `too_long_first_doc_paragraph`); `npm run build` OK. Regression sanity: `cargo test --workspace` green (core 155 / protocol 20 / datastar 20 / content 13 / storage+server) + `node --test` 38/38 — no regression.
- **Deviations from design (+ reason):**
  1. Two new struct doc-comments split into `summary` + body paragraphs to satisfy `clippy::too_long_first_doc_paragraph` (nursery). No behavior/shape change.
  2. `toNearbyItem` precomputes `entityDetail: toEntityDetail(entry)` so the inspect click opens the dialog from the card model without the glue re-deriving from a raw entry — consistent with the design (the detail view-model reads the snapshot entry; this just computes it once, in the pure layer). Function-hoisting makes the earlier `toNearbyItem` able to call the later-declared `toEntityDetail`.
  3. Predicted **R1/R2 fan-out** materialized exactly: `CombatProfile.disclose_stats` forced the field on 5 core `CombatProfile {…}` test literals (set `false`) + 2 content assertions (Bell-Eater `false`, ashen_stray `true`); the two `NearbySnapshot` fields forced `threat: None, stats: None` on the protocol `nearby()` helper. All `#[serde(default)]`/skip, so TOML/JSON round-trips unchanged; literals are `#[cfg(test)]`. Compile-caught.

## Inspect (Phase 3.5)
- **Lenses run** (3 parallel `general-purpose` critics over `git diff b588b85`): (1) correctness / server-authority / attackable-edges / MSI (each ran `cargo mutants` + `cargo test`); (2) additive serde / state-integrity / REQ-007 preservation (ran the suite + a throwaway serde round-trip); (3) JS state/view separation / detail-dialog no-command-no-mutation / id-class cross-ref / reuse (ran `node --test` + a Node harness). Verdict: the implementation is **correct, server-authoritative, and REQ-007-preserving** — no CRITICAL/HIGH *code* bug. **No inspect code changes.**
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Disposition |
  |---|---|---|---|---|
  | I1 | HIGH | Unkilled `&& → ||` mutant at `attackable = is_interactable() && combat_enabled` (core lib.rs ~1681) → fails FULL `MUT_MSI_MIN=100`. Critic 1 confirmed via `cargo mutants` it's the **only** viable mutant in the new closure (the `room_snapshot -> Default` mutant is unviable — `RoomSnapshot` has no `Default`). | **REAL, → VALIDATE** (not a code fix) | Carry-forward: write **N2** (hostile, too-far → `Some{attackable:false, attack_command:None}`) **and N2b** (hostile, non-combat room → same) by value — each kills the mutant; REQ-002 needs both. `--fast` won't run mutation, but Codex's full gate will. |
  | I2 | HIGH | Zero by-value test coverage for the new `threat`/`stats` compute (Rust) + `toNearbyItem`/`toEntityDetail` (JS). | **REAL, → VALIDATE** | The Phase-2 plan already specifies N1–N8/P1–P2/CT1–CT2 + J1–J6; Validate writes them. N7 (run the server's `attack_command` → combat starts) is the key server-authority round-trip. |
  | I3 | LOW | A hidden-stats combatant serializes `stats: {}` (present-but-empty), not omitted (core lib.rs ~1691). | **REJECTED (intentional)** | This is the design: `stats` present ⟺ has a `CombatProfile`, so the client distinguishes "combatant, hidden → unknown" (REQ-005) from "not a combatant → no stats section". Byte-identity still holds for the common non-combatant case; only the rare combatant gains a tiny meaningful key (additive — old clients ignore it). |
  | I4 | LOW | "Attackable" status/styling shows even if a payload had `attackable:true` without `attackCommand`, while the Attack action is gated on both (snapshot.js ~135). | **REJECTED (unreachable)** | The server contract `attack_command = attackable.then(...)` guarantees the command is `Some` whenever `attackable` — the mismatch can't occur with the real server. The `&& threat.attackCommand` guard is correct defensive code (prevents a `runCommand(undefined)` Attack button). |
  | I5 | LOW | `health` and `max_health` both filled from `profile.health` (core lib.rs ~1694) — reads like a copy-paste bug. | **REJECTED (intentional)** | Per the design: a not-yet-engaged nearby combatant is at full health, so nearby disclosed `health == max_health == authored` (live HP belongs to the #22 battle modal). Validate's N4 asserts this deliberately. |
- **Verified hard rules (all PASS, concretely):** server-authority — the engine computes everything; `toNearbyItem`/`toEntityDetail` read only `entry.threat`/`entry.stats`/`entry.threat.attackCommand` (no name/CSS/role inference; the Attack command is the server's verbatim). REQ-004 — `openEntityDetail`/`toEntityDetail` send no command + mutate no state; the inspect `<button>` and the action buttons are DOM siblings (no bubbling overlap). REQ-005 — three distinct disclosure states (disclosed numbers / hidden→unknown / non-combatant→no section), never an invented value. REQ-007 — `disclose_stats` is read only in `room_snapshot`, never in attack/resolve/confront; the Bell-Eater is not hostile (→ not attackable, confront intact); the full #22/oath/boss suite passes. Additive serde — both new fields skip-none + camelCase; old payloads deserialize to `None`. Borrow/panic — no `unwrap`/`expect`/`as` on input paths; `thing.name` is borrowed by `format!` before being moved into the struct. R4 equivalent-mutant avoided (no redundant `kind==Actor`). ids/classes cross-referenced verbatim across index.html ↔ client-app.js ↔ styles.css.
- **Validate carry-forward (the test backbone):** **N2 + N2b are mandatory** (kill the `&&` mutant). Plus N1 (attackable→`Some{true, Some("attack <name>")}`), N3 (non-hostile→`threat None`), N4 (disclosed→`Some{Some(9),Some(9),Some(3)}`), N5 (hidden→`Some{None,None,None}`), N6 (non-combatant→`stats None`), N7 (server `attack_command` round-trips into combat — the authority test), N8/P1/P2 (additive serde + camelCase + old-payload-deserializes), CT1/CT2 (ashen_stray discloses / Bell-Eater hidden + world validates), J1–J6 (toNearbyItem flags/Attack-action + toEntityDetail visible/hidden/non-combatant + purity). Assert by value, never `is_some`.
- **Verification of state:** `cargo test --workspace` + `node --test` 38/38 already green (no code changed this phase); fmt/clippy `--all-targets` green from Implement. No `failure-record` (no behavioral bug shipped — the implementation was correct; all findings are test-coverage or intentional design).

## Phase 4 — Validate
- **Tests added (15 new):**
  - `oathstar-core/src/lib.rs` (7, + a `nearby_world`/`nearby_engine`/`thing_named` fixture): **N1+N4** reachable hostile → `threat Some{attackable:true, attack_command:Some("attack Stray")}` + disclosed `stats Some{Some(9),Some(9),Some(3)}`; **N2** out-of-reach hostile → `threat Some{attackable:false, attack_command:None}` (kills the `&&`→`||` mutant, interactable side); **N2b** hostile in a non-combat-enabled area (after moving there) → same (kills the mutant, combat_enabled side); **N3** non-hostile → `threat None`; **N5** undisclosed combatant → `stats Some{None,None,None}` (unknown); **N6** non-combatant → `stats None`; **N7** the server's `attack_command` run through `handle_command` starts combat (server-authority round-trip).
  - `oathstar-protocol/src/lib.rs` (3): **P1** threat/stats camelCase (`attackCommand`/`maxHealth`) round-trip by value; the optional inner fields omit when `None` (`stats:{}` for a hidden combatant); **P2** a non-hostile non-combatant omits `threat`+`stats`, and a legacy JSON without them deserializes to `None`/`None`.
  - `tests/nearby-affordances.test.js` (new, 5): **J1** attackable hostile → flags + Attack action with `command === entry.threat.attackCommand`; **J2** not-attackable hostile (too-far + non-combat) → flagged, no Attack action, the quiet `combatStatus`; **J3** non-hostile → not flagged/armed, Look/Talk preserved; **J4** `toEntityDetail` disclosed/hidden(unknown)/non-combatant stat shapes; **J5** `toEntityDetail` purity (no command, no mutation of the input).
  - **CT1/CT2** are covered by the (Implement-updated) #22 content assertions: `beginner_ashen_stray_is_a_hostile_combatant` asserts `disclose_stats: true`; `beginner_bell_eater_is_combatant_boss_with_combat` asserts `false` + `world.validate()==Ok`.
- `cargo test --workspace`: **GREEN** — `oathstar-core` **162** (+7 N-series), `oathstar-protocol` +3 (P1/P2), `oathstar-content` 13, `oathstar-datastar` 20, storage/server suites; 0 failed.
- `node --test tests/*.test.js`: **43 pass / 0 fail** (was 38; +5 J-series).
- `bin/gate.sh --fast`: **GATE GREEN [fast] — 14/14** (gates 1–14: rustfmt, clippy strict, cargo test, node --test, audit/deny/machete, gitleaks, shellcheck, no-suppressions, source-bans, lints-allowlist, doc-todos, tauri-shell). Gates 15–17 (coverage + mutation) are SKIP under `--fast` per the owner — reserved for Codex's pre-commit FULL gate. The `&&`→`||` mutant the inspect critic flagged is killed by N2 + N2b (assert-by-value), so the FULL `MUT_MSI_MIN=100` should hold.
- Pre-existing exclusions: none — no pre-existing failures.

## Phase 5 — Complete
- **Docs updated:** `decisions.md` Decision 041 (Nearby affordances server-authored on NearbySnapshot — threat/stats objects, attackable gate, Option=unknown disclosure, generic detail dialog, the threat/stats-vs-flat-bools rationale); `entity-model.md` #23 note flipped future→implemented; `ui-design.md` Implementation Status gains the #23 Nearby-combat-aware + entity-detail-dialog bullet; `spatial-awareness.md` Server-authoritative section gains the threat/stats bullet.
- **Forge capture:** AAR `ca38d7cb` closed (`completed`, effectiveness 5, 20 verdicts, 2 novel findings; distillation/drift/pattern jobs enqueued). `architecture-decision-record` **AD-claude-nearby-affordances-001** (`60cc1bba`). `prevention-rule-record` **PR-claude-serde-skip-bool-needs-is-false-helper-001** (`c768ca60`) — the is_false/clippy trap that drove the grouped-object shape. No `failure-record`: the 3-critic inspect found no behavioral bug (all findings were Validate test-coverage or intentional design).
- **Ticket closed:** forge `e8eaca33` → `done` (closing comment posted); local doc moved `tickets/open/ → tickets/closed/`, frontmatter `status: closed` + `pipeline_spec` repointed to `completed/`.
- **Archived:** `WORK-nearby-hostile-affordances-v1.{spec,notes}.md` moved `pipeline/active/ → pipeline/completed/`; spec `status: Phase 5 — Complete PASS`.
- **Stopped before `/commit`** for Codex manager review (branch `codex/oathstar-ticket-23-nearby-hostile-affordances`; nothing committed; the FULL gate was not run — the owner reserves it + commit for Codex).
