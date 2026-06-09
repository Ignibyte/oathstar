# WORK-combat-encounter-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #22 — Combat Encounter v1 (fast authored battle loop). Branch `codex/oathstar-ticket-22-combat-v1`; base = committed #21 at `55633d1` (full gate GREEN 17/17). Deterministic, server-authoritative combat: combat state, `attack`/`strike`/`fight` path, deterministic HP/damage, hostile return action, win/loss resolution, clean refusal outside combat / combat-enabled areas, typed combat-channel events, preserved oath/boss flow, AND a client battle modal (left log / right participants) closing to a compact feed summary. Rust + JS tests per EARS REQ incl. modal. Run `cargo test --workspace`, `node --test tests/*.test.js`, `npm run build`, `./bin/gate.sh --fast`. Autonomous through design→implement→inspect→validate→complete; **stop before /commit** for Codex manager review. Scope only #22; do not touch untracked `assets/tilesets/` or `bin/generate_oathstar_tileset.py`.
- **Intake source:** none (ticket pre-existed; forge `4167bcb6-c807-4c2c-8ed6-311b7b3ae20b`).
- **Classification / tier:** Work pipeline, **one large shippable slice** spanning engine + parser + protocol + content + UI. Honored the owner's deliberate bundling (the modal is a v1 UX contract requirement coupled to the combat-state shape) — not split into sub-pipelines. Largest slice of the #18–#22 arc. DESIGN must sequence internally: engine/protocol core → snapshot → content encounter → UI modal. Size is the headline risk (see below).
- **Base verified:** branch `codex/oathstar-ticket-22-combat-v1`, HEAD `55633d1` ("Add entity role contracts", #21), `55633d1` confirmed ancestor; working tree clean except the untracked tileset guardrails (`assets/tilesets/`, `bin/generate_oathstar_tileset.py`); no active pipeline before this; forge up.
- **Forge recall (lessons/failures surfaced):**
  - ADRs: entity-contracts (`9d063c49` — the typed `Role`/contract foundation this builds on), inventory (`c5c90992`), pack snapshot (`938775f7`), `1700e917`.
  - Prevention rules: PR-claude-validator-length-001 (extract per-concern helpers; run `clippy --all-targets` in IMPLEMENT) — directly applicable to the new combat resolution + the growing `handle_command`/`parse`. PR for stale llvm-cov instrumentation (`cargo llvm-cov clean --workspace` before trusting a low number). Plus distilled lessons on 100% MSI / assert-by-value.
  - `docs-search`: `docs/ui-design.md` **pre-specifies the modal contract** — combat is a "Focused modal/view"; "a modal interaction still emits feed events, and closing it should leave behind a concise summary card" (REQ-008/009/010 are the documented direction). Datastar/SSE feed (`#log`) is server-rendered (`oathstar-datastar`, ticket #15); JSON stays for state/canvas (Decision 034). `docs/protocol-and-output.md` lists `CombatHit`/`CombatMiss` + "Combat messages / Combat action prompt / Boss phase banner" as the documented (not-yet-built) catalog.
  - AAR opened: `5e3cf138-c359-466b-90c0-301dcd5e2241`. Plan-phase knowledge-context logged (13 surfacings).
- **Current-code anchor map (Explore digest — `file:line` for Design):**
  - `crates/oathstar-core/src/lib.rs`:
    - `confront()` @ ~1270–1336 — finds boss via `has_role(Role::Boss)`, fulfills the active oath, emits `EventChannel::Combat` + `OutputComponent::CombatMessage` + `OathFulfilled`. **PRESERVE** (does not read `combat.health`).
    - `CombatProfile { health: u32 }` @ ~126–130; `Entity.combat: Option<CombatProfile>` (`#[serde(default)]`) @ ~116–120 — authored, currently unread.
    - `Role` enum + `from_tag`/`as_str` @ ~137–178; `Entity::has_role`/`roles_typed` @ ~180–192 (`Combatant`, `Boss`).
    - `PlayerState { hp: i32, max_hp: i32, focus, max_focus }` @ ~625–635 — init `(20,20)`; **unread by combat today**.
    - `GameState { tick, current_room_id, discovered_rooms, player, oath, pack, offered_oath_id }` @ ~602–623 — **no combat field** → additive `combat: Option<…>` lands here.
    - `handle_command` dispatch @ ~747–816 + `response(accepted, events)` helper @ ~818–824 — arms return `(bool, Vec<GameEvent>)`. New combat verb arm(s) follow this shape.
    - `RoomDefinition` @ ~40–58 (no combat flag), `RegionDefinition`/`SubregionDefinition` @ ~62–75 (id/name only) — **no combat-enabled attribute exists yet**; v1 must introduce one (room or subregion — Design decides).
  - `crates/oathstar-core/src/command.rs`: `Command` enum @ ~54–91 + `parse` @ ~93–209 + `parse_bare_verb` @ ~216–230 (the bare-verb + required-target patterns to extend for `attack`/`strike`/`fight`).
  - `crates/oathstar-protocol/src/lib.rs`: `GameSnapshot` @ ~21–42 (additive `combat:` sub-state lands here), `GameEventKind` @ ~199–222 (`#[serde(tag="type", rename_all="snake_case")]` — add combat variants), `EventChannel` @ ~182–197 (`Combat` exists), `OutputComponent` @ ~224–236 (`CombatMessage` exists).
  - JS: `src/client-app.js` — `el` cache incl. **existing `#room-modal` `<dialog>` infra** + `openRoomModal()`/`showModal()` (the pattern to mirror), `renderAll()` orchestrator (`renderHud`/`renderRoom`/`renderMap`/`renderMenu`/`renderIntent`). `src/client/snapshot.js` — `toHud`/`toOaths`/`toNearby`/`toMenuModel` (add a `toBattle`-style view-model). `src/client/components.js` — `toComponent` maps events→descriptors; `combat` channel + `combat_message` already style as `"danger"`, so new combat events auto-style in the feed.
  - `modules/beginner/world.toml`: regions `hollowmere`/`ashen_road`/`old_bell_tower`; subregions `town`/`wall`/`wilds`(ashen_road)/`tower`/`boss`. Bell-Eater @ ~90–102 `roles=["boss","combatant"]` + `combat={health=12}`. The **`ashen_road`/`wilds` subregion is the natural home** for the new hostile road encounter.
- **Ticket:** forge `4167bcb6-c807-4c2c-8ed6-311b7b3ae20b` (#22); local doc `docs/planning/tickets/open/TICKET-22-…md`.
- **EARS requirements reviewed:** REQ-001..010 (verbatim in the spec). Engine REQ-001..005 (Rust tests), REQ-006 cross-cutting (Rust+JS), REQ-007 preservation (gate/smoke), REQ-008..010 modal/feed (JS/browser smoke). Every REQ gets ≥1 test in the Phase-2 plan, including the modal.

### Open design questions (for Phase 2 — Planner does NOT decide these)
1. **`CombatState` shape.** Fields for the active encounter on `GameState` (enemy id/name + current/max HP; player HP lives on `PlayerState`; a round/turn counter; whether to keep a short in-state log or rely on emitted events). Model the enemy side as a list now (multi-party-extensible per REQ-009) vs a single opponent for v1 simplicity.
2. **Damage model.** Fully fixed authored damage vs deriving from profile. Whether `CombatProfile` gains an additive `attack`/damage field (and a player attack value source). Must stay deterministic (no RNG, or a seeded injectable RNG kept fixed in tests).
3. **`attack`/`strike`/`fight` arity & semantics.** Bare verb (strike the active/nearby hostile) vs required target (`attack <name>`); whether the same verb both *starts* combat (targeting a hostile) and *advances* a round (during combat), and how `confront` relates (kept separate, per the lock).
4. **How "hostile" is expressed.** A `hostile` role/tag, a `CombatProfile`/entity flag, "any combatant in a combat-enabled area is attackable", or only explicitly-authored hostiles — and whether the Bell-Eater (boss+combatant) is attackable via this path or remains confront-only.
5. **How "combat-enabled area" is expressed.** New optional attribute on `RoomDefinition` vs `SubregionDefinition`/`RegionDefinition` vs derived-from-a-hostile-present. Where it is authored (the wilds encounter room). Must round-trip serde + validate at the construction boundary (Decision 030).
6. **Loss outcome.** Death penalties are out of scope — define the minimal v1 loss (e.g. clear combat + a "you fall" outcome, or reset HP and return to a safe room per Decision 008's spirit) without building a penalty economy.
7. **Combat `GameEventKind` variants + fields.** e.g. `CombatStarted`/`CombatStrike`(or Hit/Miss)/`CombatEnded{outcome}` — names, fields, and which carry the per-side HP deltas the modal needs.
8. **`CombatSnapshot` shape + modal trigger.** The snapshot sub-object (participants list + state + active flag) driving the modal; open-on-active / close-on-absent logic; the compact summary component on `CombatEnded` (REQ-010). Confirm the `confront`/Bell-Eater smoke still passes unchanged (REQ-007).

## Phase 2 — Design

### Approach / architecture (the 8 open questions, resolved)
Command-driven, server-authoritative, deterministic. The `attack` command both
*starts* and *advances* combat; the engine resolves a full round per command
(player strike → enemy return) with fixed damage and no RNG. Combat state lives
on `GameState`, the modal is driven by `snapshot.combat`, and the feed carries
combat narration through the existing channel/component path. The boss/oath
`confront` flow is untouched.

1. **CombatState shape (engine) + CombatSnapshot (protocol).**
   - Engine `CombatState { enemy_id: String, enemy_name: String, enemy_hp: i32, enemy_max_hp: i32, enemy_attack: i32, round: u32, log: Vec<String> }` on `GameState.combat: Option<CombatState>` (`#[serde(default)]`). Player HP reuses `PlayerState.hp/max_hp`. `Some` = an active encounter; resolution sets it back to `None` (REQ-004 "clear"). Enemy stats are snapshotted from the entity's `CombatProfile` at start (so resolution is self-contained and survives removing the entity on victory). `i32` throughout; the one `u32`→`i32` conversion (from `CombatProfile`) uses `i32::try_from(..).unwrap_or(i32::MAX)` (no `as`, clippy-safe). HP clamps at 0 with `(hp - dmg).max(0)`.
   - Protocol `CombatSnapshot { round: u32, participants: Vec<CombatantSnapshot>, log: Vec<String> (skip-if-empty) }` + `CombatantSnapshot { id, name, hp: i32, max_hp: i32, side: String }` (camelCase). `side` is a `String` (`"player"`/`"enemy"`), matching the existing `NearbySnapshot.kind`/`proximity` convention — and a **list** so multi-party fits later (REQ-009). v1 participants = `[player(side=player), enemy(side=enemy)]`.
2. **Damage model.** Fully deterministic, fixed, authored — **no RNG** (keeps mutation/regression strong). Player strike = a documented `const PLAYER_STRIKE_DAMAGE: i32 = 4`. Enemy return = the entity's authored attack, via an **additive** `CombatProfile.attack: u32` (`#[serde(default)]` ⇒ existing `combat = { health = 12 }` Bell-Eater stays valid with attack 0).
3. **`attack`/`strike`/`fight` arity + semantics.** New `Command::Attack { target: Option<String> }` (verbs `attack`/`strike`/`fight`; optional target). If `state.combat` is `Some` → resolve the next round vs the active enemy (target text ignored in v1). If `None` → try to *start*: gate on the room, resolve a hostile, build `CombatState`, emit `CombatStarted`, resolve the first round. `confront` stays a separate verb (boss/oath); unchanged.
4. **How "hostile" is expressed.** A new typed **`Role::Hostile`** (tag `"hostile"`), parsed like the other #21 roles. `attack` only engages an entity with `has_role(Role::Hostile)`. New #21-style contract in `validate_entity_contracts`: a `hostile` must be an `Actor` (already enforced) **and** carry a `combat` profile (else `RoleContractUnmet{missing:"a combat profile (health) so the hostile can be fought"}`) — fail-fast on broken content. The Bell-Eater is `["boss","combatant"]` (NOT hostile) ⇒ `attack` can never engage it (REQ-007); only `confront` resolves it.
5. **How "combat-enabled area" is expressed.** Additive `RoomDefinition.combat_enabled: bool` (`#[serde(default)]` ⇒ every existing room stays non-combat, preserving all current behavior). Authored `true` only on the wilds `ashen_road` room. `attack` (when not already in combat) refuses cleanly in a non-combat-enabled room (REQ-005). Region/subregion-level gating is noted as future; v1 is room-level. Engine-only (not surfaced in `RoomSnapshot`).
6. **Loss outcome (death penalties out of scope).** On player HP 0 → emit `CombatEnded{defeat}` + a feed summary, **clear** combat, and **revive the player to `max_hp` in place** (no penalty, no relocation). Documented as the v1 minimum; Decision 008's penalty/relocation is deferred. On victory → emit `CombatEnded{victory}` + summary, clear combat, and **remove the defeated enemy** from the room's `entities` (same `world.rooms.get_mut(room_id).retain(..)` mutation `take_at` uses) so there's no corpse to re-fight; enemy HP does not persist across encounters (loot/persistence out of scope).
7. **Combat `GameEventKind` variants.** Per-strike + start/end narration is emitted as `LogMessage { component: CombatMessage }` on `EventChannel::Combat` — renders in the feed with **zero** new render code (same path as `confront`'s combat line). Plus two **typed lifecycle markers** for REQ-006's "future collapsible combat logs": `CombatStarted { enemy_id, enemy_name, text }` and `CombatEnded { outcome: CombatOutcome, text }` (new `CombatOutcome { Victory, Defeat }`, snake_case). Both carry `text`, so `oathstar-datastar` renders them via the Combat-channel `(danger,"Combat")` mapping and the client `toComponent` default (`event.text`). REQ-010's compact summary = the `CombatEnded` line in the feed.
8. **CombatSnapshot + modal trigger.** `GameSnapshot.combat: Option<CombatSnapshot>` (additive, skip-if-none) drives the client. `renderAll` gains `renderBattle(snapshot)`: `snapshot.combat` present → ensure `#battle-modal` open (`showModal()` if not already) + render left log + right participants; absent → `close()`. The modal is a true `<dialog>` mirroring `#room-modal`, with an **in-modal "Attack" button** (`runCommand("attack")`) as the combat action while it's open (the command bar starts combat; the modal continues it). The compact end-summary lives in the feed (server-rendered), so closing the modal leaves the summary behind (REQ-010).

**Engine helper decomposition (clippy `too_many_lines`=100 + 100% MSI):** `attack` (thin dispatch) → `start_combat` (room gate + hostile resolution + build state + first round) → `resolve_combat_round(&mut events)` (player strike → victory? → enemy return → defeat? → else round+=1) → `end_combat(outcome)` (emit `CombatEnded` + summary, remove-enemy/revive, clear state). Hostile lookup reuses `awareness::resolve_target` (named) / a room scan for `has_role(Hostile)` + reach (bare). Borrow discipline copies ids/values out before `self.log` (the `confront`/`take_at` pattern).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-core/src/lib.rs` | `CombatProfile`: add `#[serde(default)] attack: u32`. |
| 2 | `crates/oathstar-core/src/lib.rs` | `Role`: add `Hostile`; `from_tag` `"hostile"`; `as_str` `"hostile"`. |
| 3 | `crates/oathstar-core/src/lib.rs` | `validate_entity_contracts`: hostile ⇒ `combat` profile required (else `RoleContractUnmet`). |
| 4 | `crates/oathstar-core/src/lib.rs` | `RoomDefinition`: add `#[serde(default)] combat_enabled: bool`. |
| 5 | `crates/oathstar-core/src/lib.rs` | New `CombatState` struct; `GameState.combat: Option<CombatState>` (`#[serde(default)]`); init `None` in `try_new`. |
| 6 | `crates/oathstar-core/src/lib.rs` | `const PLAYER_STRIKE_DAMAGE: i32 = 4` (doc-commented, tunable). |
| 7 | `crates/oathstar-core/src/lib.rs` | New `attack`/`start_combat`/`resolve_combat_round`/`end_combat` + hostile-resolution helper. |
| 8 | `crates/oathstar-core/src/lib.rs` | `handle_command`: `Command::Attack` arm; add attack/strike/fight to Help text. |
| 9 | `crates/oathstar-core/src/lib.rs` | `snapshot()`: add `combat: self.combat_snapshot()`; new `combat_snapshot()` builds `[player, enemy]` participants. |
| 10 | `crates/oathstar-core/src/command.rs` | `Command::Attack { target: Option<String> }`; parse `attack`/`strike`/`fight` (optional target). Run clippy — extract a target-verb helper if `parse` tips over 100 lines (PR-claude-validator-length-001). |
| 11 | `crates/oathstar-protocol/src/lib.rs` | `GameSnapshot.combat: Option<CombatSnapshot>` (additive); new `CombatSnapshot` + `CombatantSnapshot`; new `GameEventKind::CombatStarted`/`CombatEnded`; new `CombatOutcome`. Update the `bare_snapshot()` test helper (+ any `GameSnapshot {` literal) with `combat: None`. |
| 12 | `crates/oathstar-datastar/src/lib.rs` | `render_feed_fragment` + `kind_type`: arms for `CombatStarted`/`CombatEnded` (`combat_started`/`combat_ended`, Combat `(danger,"Combat")`, body = `text`). |
| 13 | `modules/beginner/world.toml` | New `[[entities]] ashen_stray` — `kind=actor`, `roles=["combatant","hostile"]`, `combat={health=9,attack=3}`, aliases, description. |
| 14 | `modules/beginner/rooms.toml` | `ashen_road` room: `combat_enabled = true` + `entities = ["ashen_stray"]`. |
| 15 | `src/client/snapshot.js` | New pure `toBattle(snapshot)` (active, round, log, participants split into allies/enemies, `hpPct`). |
| 16 | `src/client-app.js` | `el.battleModal` + child refs; `renderBattle(snapshot)` in `renderAll` (open/update/close); in-modal Attack button → `runCommand("attack")`; add `combat_started`/`combat_ended` to the SSE `/state`-refresh list. |
| 17 | `index.html` | `<dialog id="battle-modal">` split layout (`#battle-log` left, `#battle-participants` right) + title + Attack button, mirroring `#room-modal`. |
| 18 | `styles.css` | Battle-modal styles (split layout, participant cards, HP bars) — mirror room-modal. |
| 19 | `tests/combat-client.test.js` (new) | JS view-model + component tests (J1–J4). |
| 20 | `docs/combat-system.md` | "v1 implemented" section (command-driven loop, hostile role + combat_enabled gate, win/loss, modal). |
| 21 | `docs/decisions.md` | Decision 040 (combat v1: deterministic command-driven; `Role::Hostile` + `combat_enabled`; additive events/snapshot; battle modal). |
| 22 | `docs/entity-model.md` | Add `hostile` role + its contract to the role table. |
| 23 | `docs/protocol-and-output.md` / `docs/ui-design.md` | Note combat events + battle modal landed (light). |

### Regression Test Plan
≥1 per EARS REQ. Rust = `#[cfg(test)]` in the owning crate; JS = `node --test`.

| # | Test | Proves |
|---|---|---|
| C1 | `attack <hostile>` in a combat_enabled room → `state.combat` Some (enemy name/hp by value), `CombatStarted` emitted | REQ-001 |
| C2 | bare `attack` in a combat_enabled room with one hostile → starts combat on it | REQ-001 |
| C3 | attack during combat → `enemy_hp -= 4` (exact), a `Combat`/`CombatMessage` LogMessage emitted | REQ-002 |
| C4 | non-lethal strike → enemy returns → `player.hp -= enemy_attack` (exact) + enemy-strike combat message | REQ-003 |
| C5 | strike to enemy_hp 0 → `CombatEnded{victory}`, `state.combat` None, enemy removed from room entities | REQ-004 |
| C6 | constructed tough enemy / low player hp → player hp 0 → `CombatEnded{defeat}`, combat None, player hp restored to max | REQ-004 |
| C7 | `attack` in a non-combat_enabled room → refused (accepted=false), no combat state, no hp change | REQ-005 |
| C8 | `attack` (no live combat) with no hostile present → refused cleanly, no state change | REQ-005 |
| C9 | `attack bell-eater` (boss+combatant, not hostile) and `attack mara` (non-combatant) → refused; nothing mutated | REQ-005/007 |
| C10 | `attack` a hostile that is visible-but-out-of-reach → refused "too far", no state change | REQ-005 |
| C11 | combat events carry `EventChannel::Combat` (+ CombatMessage / CombatStarted / CombatEnded) — by value | REQ-006 |
| C12 | swear → confront Bell-Eater still fulfills the oath (boss/oath flow intact alongside combat) | REQ-007 |
| C13 | parser: `attack`/`strike`/`fight` → `Attack{None}`; `attack stray` → `Attack{Some("stray")}`; verb case-folded, target kept | REQ-002 |
| C14 | a `hostile` entity with no combat profile → `validate()` `RoleContractUnmet` (entity/"hostile"/missing); hostile WITH combat passes | REQ-001/005 |
| C15 | `Role::from_tag("hostile")==Hostile`, `as_str=="hostile"`, `has_role(Hostile)` true/false — by value | REQ-001 |
| C16 | `CombatProfile`: `{health=12}` → attack defaults 0; `{health=9,attack=3}` → attack 3; round-trips | REQ-003 |
| C17 | `RoomDefinition`: no `combat_enabled` key → false; `combat_enabled=true` → true; round-trips | REQ-005 |
| C18 | active combat → `snapshot.combat` Some, participants `[player(side player), enemy(side enemy)]` + round + log, by value | REQ-008/009 |
| C19 | no combat → `snapshot.combat` None, omitted from JSON; old JSON without `combat` deserializes to None | REQ-008 |
| C20 | `CombatStarted`/`CombatEnded` serialize with snake_case `type` + fields + outcome; round-trip | REQ-006 |
| C21 | `render_feed_fragment(CombatStarted/CombatEnded)` → article class `danger`, channel `combat`, text body; `kind_type` → `combat_started`/`combat_ended` | REQ-006 |
| C22 | beginner world `Engine::try_new` succeeds WITH ashen_stray + combat_enabled room (Bell-Eater/Mara/oath intact) | REQ-007 |
| C23 | content: ashen_stray is hostile+combatant w/ health 9/attack 3; ashen_road is combat_enabled + places it — by value | REQ-001/007 |
| J1 | `toBattle(combat)` → `{active:true,…}`; `toBattle(no combat)` → `{active:false}` (modal open/close basis) | REQ-008/010 |
| J2 | `toBattle` splits participants into allies/enemies by side; each has name/hp/maxHp/hpPct; log in order; round present | REQ-009 |
| J3 | `toComponent` on combat `log_message`(combat_message) → variant `danger`/label `Combat`; on `combat_started`/`combat_ended` (text) → `danger` + text | REQ-006 |
| J4 | `toComponent(combat_ended)` returns the summary text + `danger` (feed retains the summary) | REQ-010 |

**Genuinely browser-only (not node-coverable), verified by smoke:** the actual `<dialog>.showModal()/close()` open/close DOM — jsdom doesn't implement `<dialog>` modality, and `client-app.js`/`renderBattle` is the thin glue seam already verified by smoke (same as the existing `#room-modal`). The *decisions* it acts on (`toBattle.active`, participant split, summary rendering) are node-tested above (J1–J4); REQ-008/009/010's verification method in the spec is "JS/browser smoke".

### Risks / decisions
- **R1 — `GameSnapshot` field addition** breaks every literal construction. Compile-caught; update protocol `bare_snapshot()` and grep `GameSnapshot {` for other literals.
- **R2 — new `GameEventKind` variants** force arms in `oathstar-datastar`'s two exhaustive matches (`render_feed_fragment`, `kind_type`). Compile-caught; in the manifest.
- **R3 — `parse()` clippy `too_many_lines`** (the #19/#20 recurring trap) after the attack arm. Run `cargo clippy --workspace --all-targets` in IMPLEMENT; extract a target-verb helper if needed.
- **R4 — modal modality:** a true `showModal()` makes the background inert, so the **in-modal Attack button** is the action while open (command bar starts combat). Escape/backdrop dismiss → reopens on the next attack. Documented; browser-smoke.
- **R5 — determinism / 100% MSI:** fixed `PLAYER_STRIKE_DAMAGE` + authored enemy attack, no RNG. Loss path tested via a constructed tough enemy (the shipped ashen_stray is winnable in 3 rounds, player ends ~14/20).
- **R6 — v1 simplifications (documented, not bugs):** no flee/disengage; movement doesn't end combat; enemy HP doesn't persist across encounters; victory removes the enemy (no corpse/loot — out of scope); defeat revives at full HP, no penalty (Decision 008 deferred).
- **R7 — REQ-007 preservation:** Bell-Eater is combatant-but-not-hostile and its roost isn't combat_enabled (double-protected); `confront` untouched. Regression-tested (C9/C12/C22).
- **Load-bearing decisions** (→ decisions.md Decision 040 at complete): combat state on `GameState` cleared on resolution + modal driven by `snapshot.combat` presence; `Role::Hostile` (+combat-profile contract) and `RoomDefinition.combat_enabled` as the gates; deterministic fixed damage; battle modal = true `<dialog>` w/ in-modal Attack, left log / right participant-list.

## Phase 3 — Implement
- **Built (manifest rows 1–18; tests row 19 → Validate, docs rows 20–23 → Complete per the brief):**
  - **protocol** (`oathstar-protocol/src/lib.rs`): `GameSnapshot.combat: Option<CombatSnapshot>` (additive, skip-if-none); `CombatSnapshot { round, participants, log }` + `CombatantSnapshot { id, name, hp, max_hp, side }` (camelCase, side a `String` per the `NearbySnapshot` convention); `GameEventKind::CombatStarted { enemy_id, enemy_name, text }` + `CombatEnded { outcome, text }`; `CombatOutcome { Victory, Defeat }` (snake_case); `bare_snapshot()` test helper updated with `combat: None`.
  - **core types** (`oathstar-core/src/lib.rs`): `CombatProfile.attack: u32` (`#[serde(default)]`); `Role::Hostile` (+ `from_tag`/`as_str`); hostile contract in `validate_entity_contracts` (hostile ⇒ `combat` profile, else `RoleContractUnmet`); `RoomDefinition.combat_enabled: bool` (`#[serde(default)]`); `CombatState` struct; `GameState.combat` + `try_new` init `None`; `const PLAYER_STRIKE_DAMAGE: i32 = 4`.
  - **core logic** (`oathstar-core/src/lib.rs`): `attack` (dispatch) → `start_combat` (combat_enabled gate → `find_hostile` → build `CombatState` → `CombatStarted` → first round) → `resolve_combat_round` (player strike via `saturating_sub(..).max(0)` → victory? → enemy return → defeat?) → `end_combat` (emit `CombatEnded` + summary; victory removes the enemy via `remove_entity_everywhere`, defeat revives to `max_hp`; clears state). Plus `find_hostile`/`resolve_named_hostile`/`hostile_stats` (reusing `awareness::resolve_target`), `combat_snapshot()`, the `handle_command` `Attack` arm, and the Help text. `ResolvedHostile` private struct.
  - **parser** (`oathstar-core/src/command.rs`): `Command::Attack { target: Option<String> }` + `parse_combat_verb` helper (attack/strike/fight, optional target).
  - **datastar** (`oathstar-datastar/src/lib.rs`): `describe()` + `kind_type()` arms for `CombatStarted`/`CombatEnded` (combined or-pattern; Combat-channel `danger`, body = `text`).
  - **content**: `world.toml` `ashen_stray` (combatant+hostile, `combat = { health = 9, attack = 3 }`); `rooms.toml` `ashen_road` → `combat_enabled = true` + `entities = ["ashen_stray"]`.
  - **JS**: `snapshot.js` `toBattle` (pure, side-split participants + `hpPct`); `client-app.js` `el.battleModal` refs + `renderBattle`/`combatantCard` + Attack-button binding + `renderAll` hook + combat events added to the SSE `/state`-refresh list; `index.html` `#battle-modal` split layout + Attack button; `styles.css` battle-modal styles (verdigris player / wine enemy / brass action).
- **Compile/check (this phase):** `cargo fmt` clean; **`cargo clippy --workspace --all-targets` GREEN** (ran `--all-targets` per PR-claude-validator-length-001 — `parse()` stayed under `too_many_lines=100`, the `parse_combat_verb` extraction did its job); `npm run build` OK (11 modules). Regression sanity (not new tests): `cargo test --workspace` green + `node --test tests/*.test.js` 32/32 — no regression.
- **Deviations from design (+ reason):**
  1. `find_hostile` implemented as a `let-else` rather than a `match`, to satisfy `clippy::option_if_let_else` (nursery). Same behavior; cleaner.
  2. The compile-caught fixture fan-out predicted in design risks **R1/R2 materialized exactly**: adding `RoomDefinition.combat_enabled` forced the field on 4 test-fixture literals (core `test_world` rooms a/b, `room_with`, `awareness.rs` `make_room`); `CombatProfile.attack` forced it on the content Bell-Eater assertion (set to `attack: 0`); `GameSnapshot.combat` forced `bare_snapshot()`. All additive `#[serde(default)]`, so TOML/JSON round-trips unchanged; the literals are `#[cfg(test)]` and were updated to compile (allowed in Implement).
  3. No new `WorldValidationError` variant — the hostile contract reuses `RoleContractUnmet` (as designed).
  4. The end-of-combat summary is carried solely by `CombatEnded.text` (rendered by the datastar arm) — no separate `LogMessage` — as designed (REQ-010 satisfied via the typed event's text).

## Inspect (Phase 3.5)
- **Lenses run** (4 parallel `general-purpose` critics over `git diff 55633d1`): (1) correctness / combat state machine; (2) determinism + 100% MSI (ran the *actual* `cargo mutants --list` to enumerate real mutants); (3) serde / state-integrity + REQ-007 preservation; (4) JS state/view separation + simplification/reuse. Verdict: correctness, serde/REQ-007, and JS came back **CLEAN** (no CRITICAL/HIGH); determinism critic found 4 real coverage-hygiene items (unreachable defensive arms), all fixed.
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | I1 | MEDIUM | `resolve_combat_round` `let Some(combat) = … else { return }` is unreachable (both call sites guarantee `Some`) → uncovered `return` line pressuring `RUST_COV_MIN=94` (lib.rs ~1550) | **REAL** (verified both call sites) | Converted to `.expect("resolve_combat_round is only called with an active encounter")` (house-style invariant, cf. `current_room`) |
  | I2 | MEDIUM | enemy-line push `if let Some(combat) = …` — combat is guaranteed `Some` when the enemy survives; the Option-handling is misleading (lib.rs ~1582) | **REAL** (consistency) | Converted to `.expect("combat remains active until a combatant falls").log.push(…)` |
  | I3 | MEDIUM | `end_combat` `let Some(combat) = …take() else { return }` unreachable (only called mid-encounter) (lib.rs ~1601) | **REAL** | Converted to `.expect("end_combat is only called mid-encounter …")` |
  | I4 | MEDIUM | `hostile_stats` `entity.combat.as_ref()?` (+ `entities.get()?`) unreachable for a validated world → uncovered; doc comment wrongly claimed the `combat` check filters non-hostiles (lib.rs ~1530/1534) | **REAL** | Both `?` → `.expect()` invariants (entity-lookup + combat); the reachable non-hostile filter stays `has_role → None`; doc comment corrected |
  | I5 | INFO→VALIDATE | `i32::try_from(health/attack).unwrap_or(i32::MAX)` — `--list` shows no mutant, but the `unwrap_or` fallback is an uncovered tail (lib.rs ~1462) | **REAL, deferred** | No code change; Validate adds a `CombatProfile { health: u32::MAX, attack: u32::MAX }` test asserting `enemy_max_hp == i32::MAX` to cover the tail |
  | R1 | — | "`.max(0)` is a redundant/equivalent-mutant survivor" | **REJECTED** | `cargo mutants --list` generates no mutant there, and `.max(0)` is load-bearing (`i32::saturating_sub` saturates at `i32::MIN`, so it *can* go negative) |
  | R2 | — | "`<=0`→`<0`/`<1` mutation survivor" | **REJECTED** | mutants only generates `<= → >`; killed by exact-zero-HP victory/defeat tests (Validate) |
  | R3 | — | "datastar `CombatStarted`/`CombatEnded` arm-swap survivor" | **REJECTED** | mutants does whole-function mutants only (no arm-swap); the new arm folds into existing `describe`/`kind_type` mutants already killed |
  | R4 | LOW | bare `attack` engages only a same-cell hostile; named `attack <x>` can reach an adjacent interactable cell | **REJECTED** (intended; identical to the `talk`/`take` proximity model; doc-comment-accurate) | none |
  | R5 | INFO | enemy removal mutates `self.world` in memory only — not persisted (no save-game wiring yet) | **ACKNOWLEDGED** (design property, not a regression) | none — note for a future save/load ticket |
- **REQ-007 preservation — verified by the serde critic (ran the suite):** Bell-Eater is `["boss","combatant"]` (not `hostile`) so `attack bell-eater` refuses (`"… is not something you can attack."`); `bell_eater_roost` is not `combat_enabled` so the gate refuses first; `confront` is byte-unchanged; the beginner world still validates with the new hostile; all confront/oath/boss tests pass. Additive serde round-trips confirmed (old saves/snapshots load; combatless snapshot byte-identical; `combat_enabled` engine-only, absent from `RoomSnapshot`).
- **Validate carry-forward (from the determinism critic's enumerated kill-assertions — the Phase-4 test backbone):** assert event **kinds/fields and snapshot values by value** (esp. `combat_snapshot`'s `Some(Default)` mutant — assert `round`/HP/`side`, never `is_some`); author HP fixtures landing **exactly on 0** to kill the `<= → >` boundary on both sides; a **two-entity** room (bystander survives `remove_entity_everywhere`'s `!= → ==`) and a **two-hostile** room (first-authored engaged — locks `find_hostile` order/determinism); the four `resolve_named_hostile` refusal strings (unknown / fixture / too-far / non-hostile); `round += 1` value asserts (1 then 2); the `u32::MAX` profile test (I5).
- **Verification of fixes:** `cargo fmt` clean; `cargo clippy --workspace --all-targets` GREEN (`expect` is house-allowed, cf. `current_room`); `cargo test -p oathstar-core` 137/137. No `failure-record` warranted — no behavioral bug shipped; the fixes are coverage-hygiene preventives of the already-known "prefer expect-invariants over unreachable arms" discipline (captured at Complete in the AAR).

## Phase 4 — Validate
- **Tests added (31 new):**
  - `oathstar-core/src/lib.rs` (18): named/bare start; deterministic strike damage + Combat/CombatMessage event; enemy return damage; victory at exactly-0 enemy HP (+ enemy removed + bystander survives, killing `remove_entity_everywhere`'s `!=`→`==`); defeat at exactly-0 player HP (+ revive to max_hp); refusals (non-combat-enabled room, no hostile present, non-hostile actor, fixture, unknown name, too-far); in-combat target-ignored + round 1→2; combat-snapshot by-value (round/hp/maxHp/side/log — kills the `Some(Default)` mutant); no-combat snapshot; u32::MAX profile saturates to i32::MAX; `Role::Hostile` tag round-trip; hostile-contract `RoleContractUnmet`; REQ-007 boss-not-attackable + confront still fulfills.
  - `oathstar-core/src/command.rs` (1): `attack`/`strike`/`fight` parse with optional target (case-folded verb, preserved target, `attacker` ≠ attack).
  - `oathstar-protocol/src/lib.rs` (4): `CombatSnapshot` camelCase round-trip; empty-log omitted; `GameSnapshot.combat` omitted/optional; combat events snake_case `type` tag + `CombatOutcome`.
  - `oathstar-datastar/src/lib.rs` (1): `combat_started`/`combat_ended` render on the Combat channel (danger variant) with escaped text + type tag.
  - `oathstar-content/src/lib.rs` (2): Ashen Stray is a hostile combatant (health 9 / attack 3) by value; Ashen Road combat-enabled + places the stray, boss roost not combat-enabled.
  - `tests/combat-client.test.js` (new, 6): `toBattle` inactive/active; participant split by side + `hpPct`; zero-maxHp (no NaN) + `defeated`; `toComponent` combat events → danger variant + text; combat-end inactive + feed summary.
  - **Deviation:** the planned core C16/C17 (serde-default unit tests) were dropped — core has no toml/json dev-dependency, and the defaults are proven by real-TOML deserialization in the content tests (Bell-Eater `attack: 0`; roost `combat_enabled` false). Adding a dev-dep would be out-of-scope churn.
- `cargo test --workspace`: **GREEN** — oathstar-core 155, command/parse incl.; oathstar-protocol 20; oathstar-datastar 20; oathstar-content 13; oathstar-storage + oathstar-server suites pass; 0 failed.
- `node --test tests/*.test.js`: **38 pass / 0 fail** (was 32; +6 combat-client).
- `bin/gate.sh` (FULL): **GATE GREEN [full] — 17/17 PASS.** Coverage gate:15 Rust **98.00%** lines ≥ 94% (combat code: core lib.rs 99.37%, command.rs 100%, protocol 100%, content 99.23%); gate:16 JS **85.34%** ≥ 75%; gate:17 mutation **230 caught / 0 missed → MSI 100.0%** (no survivors on the first run). The FULL green wrote `.git/oathstar-gate-receipt`.
- Pre-existing exclusions: none — no pre-existing failures encountered.

## Phase 5 — Complete
- **Docs updated:** `decisions.md` Decision 040 (combat v1 — command-driven, Role::Hostile + combat_enabled gates, additive events/snapshot, battle modal, victory-removes/defeat-revives); `combat-system.md` "v1 implemented" blockquote; `entity-model.md` role table gains the `hostile` row (+ combatant `combat` now consumed by #22); `protocol-and-output.md` "Implemented (ticket #22)" note (CombatStarted/CombatEnded + CombatMessage on the Combat channel; CombatSnapshot); `ui-design.md` Implementation Status gains the #22 battle-modal bullet.
- **Forge capture:** AAR `5e3cf138` closed (`completed`, effectiveness 5, 20 verdicts, 2 novel findings; distillation/drift/pattern jobs enqueued). `architecture-decision-record` **AD-claude-combat-v1-001** (`2e4d969e`). `prevention-rule-record` **PR-claude-expect-invariants-over-unreachable-arms-001** (`b5597507`) — the inspect-caught coverage smell. No `failure-record`: no behavioral bug shipped (inspect findings were coverage-hygiene preventives; the validate compile hiccups — no toml/json dev-dep, an E0716 temporary — were standard dev iteration).
- **Ticket closed:** forge `4167bcb6` → `done` (closing comment posted); local doc moved `tickets/open/ → tickets/closed/`, frontmatter `status: closed` + `pipeline_spec` repointed to `completed/`.
- **Archived:** `WORK-combat-encounter-v1.{spec,notes}.md` moved `pipeline/active/ → pipeline/completed/`; spec `status: Phase 5 — Complete PASS`.
- **Stopped before `/commit`** for Codex manager review (branch `codex/oathstar-ticket-22-combat-v1`; nothing committed).
