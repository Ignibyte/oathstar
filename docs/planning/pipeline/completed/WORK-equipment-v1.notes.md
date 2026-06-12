# WORK-equipment-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- Request: S2 — equipment v1: item kind `equipment` with authored slot
  (weapon/armor) + stat mods; `equip`/`unequip` verbs with typed refusals;
  combat math reads equipped attack/defense; Gear panel wires to real state;
  SaveData round-trip; starter gear in the existing world. Lean — last engine
  system before the tilemap steps (A/B/C + W1).
- Intake source: INTAKE-blank-colors-vertical-slice-city-forest-cave (S2 row).
- Classification / tier: work pipeline — one shippable engine system + thin
  protocol/client wiring, same tier as #33/#34.
- Forge recall (lessons/failures surfaced):
  - `docs/inventory-and-items.md` reserves wear slots, Weapon/Armor item
    types, Wearable/Cursed/No-drop flags; open question "how many slots in
    the first slice" → the client placeholder already renders six.
  - Decision 052 (commerce): equipment must carry `value` and trade through
    the existing verbs — composition, not new commerce code.
  - PR-claude-reveal-rules-on-every-projection-001 (high, bit twice):
    hidden gear must be non-equippable and every new projection filtered.
  - PR-claude-mutants-package-scoped-test-placement-001: killing tests live
    in the mutated crate (protocol fields → protocol tests).
  - Serde-additive pattern (#28/#30/#34): `#[serde(default)]` keeps old
    saves/payloads valid; SAVE_FORMAT_VERSION stays 2.
  - gate:10: inline `#[allow]` needs a same-line trailing justification.
- Ticket: forge #35 `3fa9b4ab-c0c3-4b0e-80ec-965562ac5a4c` (open); local doc
  TICKET-35-equipment-v1-slots-equip-unequip-gear-aware-combat.md.
- EARS requirements reviewed: REQ-001..006 in the spec — verbs, refusal arms,
  combat math boundaries, snapshot/panel wiring, save round-trip + legacy
  payload, authored starter gear played end-to-end.
- AAR: `ff28d674-885c-4c84-9878-edf3db71f821` (opened at plan; failures and
  the close-out aar-submit capture into it).

## Phase 2 — Design

- Approach / architecture:
  - **Model (core):** `EquipSlot` enum (`Weapon | Armor`, serde lowercase —
    invalid authored slots fail at TOML parse, validation for free) and
    `EquipmentProfile { slot, attack: u32, defense: u32 }` (mods default 0;
    u32 forbids negative authoring; converted to i32 with saturating
    `try_from` per the operator-sweep rule so crafted `u32::MAX` never
    panics). `Item.equipment: Option<EquipmentProfile>` serde-additive —
    mirrors the `CombatProfile` table pattern (`equipment = { slot =
    "weapon", attack = 2 }`).
  - **State:** `PlayerState.equipped_weapon` / `.equipped_armor`
    (`Option<String>` item ids, `#[serde(default)]`, omit-when-none). Pack
    stays on `GameState`; equip moves the id pack→slot, unequip slot→pack
    (with a contains-guard so a crafted save duplicating an id can't dup
    items). Equipped gear therefore naturally refuses `drop`/`sell` (they
    search the pack) — must unequip first.
  - **Verbs (command.rs):** `Equip { target }` (aliases `equip`/`wield`/
    `wear` — slot comes from the item, not the verb) and `Unequip { target }`
    (`unequip`/`remove`, pending a collision check on `remove`). Required-
    target parse via a `parse_gear_verb` helper in the `parse_trade_verb`
    pattern (line-ceiling-safe).
  - **Resolution (lib.rs):** `equip_at` — mid-combat refusal (trade parity) →
    `matching_pack_items` reuse (not-carried / ambiguous arms) → no
    `equipment` profile → "not something you can equip" → success swaps:
    prior occupant returns to pack, line per slot ("You wield the X." /
    "You wear the X.", "…, stowing the Y." when swapping). `unequip_at` —
    slot keyword (`weapon`/`armor`) resolves first (empty-slot arm), else
    name-prefix over equipped items (no-match / ambiguous arms).
  - **Combat hooks (3 sites):** basic strike 2567 and power strike 1315 add
    `player_attack_bonus()` (equipped weapon's attack); incoming hit 2619
    becomes `enemy_attack.saturating_sub(player_defense()).max(0)` — the
    narrated line shows DEALT damage (disclosed enemy stats stay raw).
    Bonuses computed before `&mut combat` borrows. Power strike gains the
    weapon bonus too (one weapon swings every strike).
  - **Wire (protocol):** `EquippedItemSnapshot { slot, id, name }` camelCase;
    `PlayerSnapshot.equipment: Vec<…>` omit-when-empty + default — old
    payloads/clients unaffected. In-crate tests (package-scoped mutants).
  - **Client (snapshot.js):** `toGear(snapshot)` maps `player.equipment`
    into the six fixed panel slots via `SLOT_LABELS = { weapon: "Main hand",
    armor: "Body" }`; unknown future slots are ignored gracefully; the
    client-app render loop (`menu.gear.{filled,total,slots}`) is unchanged.
  - **Content:** no loader code change (serde derives carry it); beginner
    world authors `rust_edge_blade` (weapon, attack 2, value 6) and
    `waxed_coat` (armor, defense 1, value 4) into Mara's stock, and the
    stray's `stray_fang` gains `equipment = { slot = "weapon", attack = 1 }`
    so the first drop is equippable.
  - **Save:** SaveData embeds GameState/PlayerState wholesale — the two new
    Options ride the existing format; `SAVE_FORMAT_VERSION` stays 2.

- File manifest:
  | # | File | Change |
  |---|---|---|
  | 1 | crates/oathstar-core/src/lib.rs | EquipSlot, EquipmentProfile, Item.equipment, PlayerState equipped slots, equip_at/unequip_at + typed refusals, 3 combat hooks, bonus helpers, snapshot projection, help line |
  | 2 | crates/oathstar-core/src/command.rs | Equip/Unequip variants, parse_gear_verb, parser tests |
  | 3 | crates/oathstar-protocol/src/lib.rs | EquippedItemSnapshot, PlayerSnapshot.equipment, in-crate serde tests |
  | 4 | crates/oathstar-server/src/main.rs | served gear loop test (buy→equip→boss with boosted strikes→buy coat→wear→save/load) |
  | 5 | crates/oathstar-content/src/lib.rs | loader tests: authored equipment loads by value; invalid slot rejected at parse |
  | 6 | modules/beginner/world.toml | blade + coat in Mara's stock; fang equipment line; why-comments |
  | 7 | src/client/snapshot.js | toGear(snapshot) wired to player.equipment; SLOT_LABELS; toMenuModel passes snapshot |
  | 8 | tests/client.test.js | toGear cases (empty / equipped / unknown slot) |
  | 9 | docs/inventory-and-items.md + docs/decisions.md | Phase 5: implemented-direction note + Decision 053 |

- ### Regression Test Plan
  | # | Test | Proves Requirement |
  |---|---|---|
  | T1 | equip moves a carried equipment item to its slot, removes it from pack, emits the slot-flavored line | REQ-001 |
  | T2 | equip onto an occupied slot swaps: prior occupant back in pack, line names both | REQ-001 |
  | T3 | refusal arms, state unchanged each: missing target / not carried / not equipment / ambiguous prefix / mid-combat (equip+unequip) / unequip empty slot / unequip no match | REQ-002 |
  | T4 | strike damage boundary with weapon: bare 4, fang(+1) 5, blade(+2) 6; power strike 6/7/8 | REQ-003 |
  | T5 | incoming hit reduced by armor: stray 3 vs coat(1) → 2; defense ≥ attack → 0 (boundary ±1); narrated line shows dealt | REQ-003 |
  | T6 | protocol: PlayerSnapshot.equipment serializes camelCase, omits when empty, defaults absent (in protocol crate) | REQ-004 |
  | T7 | JS toGear: empty snapshot → 0/6 all "empty"; weapon+armor → Main hand/Body filled with names, count 2/6; unknown slot ignored | REQ-004 |
  | T8 | save round-trip preserves equipped slots; legacy payload (keys stripped) loads with slots empty; crafted dup (id in pack+slot) unequips without duplicating | REQ-005 |
  | T9 | crafted extremes: equipment mods u32::MAX never panic in combat math | REQ-003/005 |
  | T10 | content: beginner blade/coat/fang equipment loads by value; invalid slot string rejects the module at parse | REQ-006 |
  | T11 | served loop: fund via #34 route → buy blade → equip → boss falls to strike-6 lines → buy+wear coat → save/load preserves both slots | REQ-006/005/001 |
  | T12 | parser: equip/wield/wear + unequip/remove strict shapes incl. missing-target refusals | REQ-001/002 |
  - Uncoverable: none identified — every arm is reachable through commands
    or crafted saves.

- Risks / decisions:
  - Slots live on PlayerState while pack lives on GameState — the cross-
    struct move is contained inside equip_at/unequip_at.
  - Swap-on-equip (no occupied-slot refusal) — locked by REQ-001.
  - Mid-combat gear changes refused — parity with trade_combat_refusal.
  - Narration shows mitigated (dealt) damage with no absorption prose.
  - `remove` alias: verify no collision at implement; drop the alias if so.
  - Mara the candle-keeper stocking a blade is a flavor stretch the slice
    accepts; W1 re-homes vendor stock.
  - MSI 100 exposure: every new string arm and boundary gets an exact
    in-crate killing test (T1–T12).

## Phase 3 — Implement
- Built:
  - protocol: `EquippedItemSnapshot` + `PlayerSnapshot.equipment`
    (omit-when-empty, default) + `bare_snapshot` fixture field.
  - core lib.rs: `EquipSlot` (serde lowercase) + `EquipmentProfile` (u32
    mods, default 0) + `Item.equipment`; `PlayerState.equipped_weapon/
    equipped_armor`; `item_name`, `equipped_slot_mut` (const), `slot_mod`
    (saturating u32→i32), `player_attack_bonus`/`player_defense`,
    `gear_combat_refusal`, `equip_at` (swap via `map_or_else`),
    `unequip_at` (slot keyword → name match), `finish_unequip`,
    `stow_into_pack` (crafted-dup guard), `equipment_snapshot`; combat
    hooks in `resolve_combat_round` (strike + bonus; incoming `dealt` =
    attack − defense floored 0, narrated as dealt) and
    `resolve_power_strike` (+ bonus); help line + dispatcher arms.
  - command.rs: `Equip { target }` / `Unequip { target }`;
    `parse_gear_verb` (equip/wield/wear, unequip/remove — no collisions);
    extracted `parse_targeted_verb` (talk/take/drop) to hold `parse` under
    the line ceiling.
  - world.toml: `rust_edge_blade` (weapon +2, value 6) and `waxed_coat`
    (armor +1, value 4) added to Mara's stock; `stray_fang` gains
    `equipment = { slot = "weapon", attack = 1 }`.
  - snapshot.js: `SLOT_LABELS` (weapon→Main hand, armor→Body);
    `toGear(snapshot)` fills the six panel slots from `player.equipment`
    (unknown slots ignored; first entry per label wins); `toMenuModel`
    passes the snapshot.
  - Test-fixture compile fixes: `equipment: None` in awareness.rs
    `make_item` and lib.rs `item()` (the #34 E0063 class, caught
    immediately with `--all-targets`).
- Deviations from design (+ reason):
  - `parse` needed the extraction anyway (102/100 lines after adding the
    gear hook) — `parse_targeted_verb` instead of growing per-verb blocks;
    behavior identical, planned as a risk.
  - Clippy nursery asked for `map_or_else` in `equip_at` and `const fn` on
    `equipped_slot_mut` — applied, no semantic change.
  - Existing string-asserting tests (help line) intentionally not updated
    here — Phase 4 owns test changes.

## Inspect (Phase 3.5)
- Lenses run: correctness, data/state integrity, projection/reveal-rule,
  simplification/reuse — four parallel critics over the full diff, each
  instructed to verify concretely.
- Findings:
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | high | `stow_into_pack` dedup guard destroys legitimate duplicate copies (lib.rs, both unequip + swap call sites) — both callers move a live reference OUT of a slot, so the push conserves count and the guard can ONLY lose items; reachable in legal play in any module placing one id twice | REAL (two critics converged; loss sequence traced) | unconditional push; doc rewritten to record why the guard was wrong |
  | 2 | med | Equipped gear vanished from every text projection: `look <gear>` denied a wielded item, `inventory` omitted it, and drop/sell/equip refused with the false "not carrying" line | REAL (player-facing denial of possessed state) | `find_equipped` helper; `look_at` gains an equipped fallback; `list_pack` appends an `Equipped:` clause; drop/sell/equip empty-match arms gain honest "is equipped — unequip it first" / "already equipped" refusals |
  | 3 | med | client-app.js gear glue hardcoded `equipment-value empty` — filled slots would render dimmed | REAL (REQ-004 wiring incomplete) | class toggles on `slot.filled` |
  | 4 | med | slot→field mapping repeated 4× while `equipped_slot_mut`'s doc claimed "single mapping" | REAL | read-only `equipped_slot` accessor added; slot_mod / unequip filter / equipment_snapshot routed through it; docs name the pair |
  | 5 | med | three required-target parser helpers in two idioms (targeted / trade / gear), identical control flow | REAL | unified into one `parse_targeted_verb` table (talk/take/drop/buy/sell/equip/unequip); trade+gear helpers deleted; behavior identical (tests assert `parse` output) |
  | 6 | low | `slot_mod` ignored `profile.slot` — a crafted save could harvest the wrong stat from cross-slot gear | REAL (hardening) | `.filter(profile.slot == slot)` — a slot only pays its own profile's stat |
  | 7 | low | toGear re-implemented `targetName` and did a Map+two-pass for a ≤2-element list; `SLOT_LABELS[slot]` walked the prototype chain (the #32 class, verified harmless here) | REAL | single-pass `find` with `targetName`; label comparison form is prototype-immune |
  | 8 | low | `list_pack` kept an inline copy of the new `item_name`; `matches.first().cloned()` cloned where a move is free | REAL | routed through `item_name`; `into_iter().next()` |
  | 9 | info | `from_save` doc-comment claimed room/enemy/oath are "exactly" the invariants — equipment ids are tolerance-class, undocumented | REAL (doc) | comment extended: pack/equipped ids deliberately unvalidated, every consumer tolerates dangles |
  | 10 | low | `unequip` slot keywords shadow items aliased "weapon"/"armor" | REJECTED — documented priority order, no such aliases authored, slot-first is the sane resolution |
  | 11 | low | refusal strings duplicated across drop/sell/equip | REJECTED — n=2/3 string duplication is the codebase's chosen idiom (#34 precedent: sell consciously copied drop's line) |
  | 12 | info | forward-compat: a pre-#35 binary loading a new save drops equipped items (additive posture, version stays 2) | ACCEPTED POSTURE — inside the locked serde-additive decision; recorded for the future format-version discussion |
- Post-fix verification: clippy strict green, `cargo fmt` applied, full
  workspace suite 389/389 green, both JS files parse.

## Phase 4 — Validate
- Tests added (21 new):
  - core (16): equip moves gear into slot + snapshot slot/id/name exact;
    snapshot weapon-first order; swap returns prior occupant; mid-combat
    refusal both verbs; missing/plain/ambiguous equip arms (state pinned);
    already-equipped honest arm; empty-slot arms both keywords; unequip
    unknown + ambiguous (shared-alias); unequip by slot and by name; drop
    + sell equipped honest arms; strike sweep 4/5/6 + power strike 8;
    armor sweep dealt 2/1/0 with exact lines; crafted extremes (u32::MAX
    attack/defense, cross-slot pays zero); save round-trip + legacy
    payload (keys stripped) + crafted-duplicate conservation; text
    projections (look equipped fallback, inventory Equipped clause, both
    restored after unequip); bought gear equips (commerce composition).
  - command (1): gear verbs strict shapes — equip/wield/wear,
    unequip/remove, multiword case-kept targets, bare-verb unknown,
    near-miss.
  - protocol (1): equipment omits when empty, defaults when absent,
    slot/id/name serialize exact.
  - content (2): beginner blade/coat/fang equipment loads by value +
    Mara stocks the gear; unknown slot string rejects the module at parse.
  - server (1): played gear loop over the seam — oath, fund 8, buy blade
    (2 left), wield (snapshot slot), boss falls to strike-6 lines in two
    blows, +25 coins → 27, buy+wear coat, both slots in snapshot weapon
    first, save/load restores the geared /state byte-for-byte.
  - JS (1): toGear panel fill — pre-#35 payload all-empty, Main
    hand/Body filled with names, unknown/duplicate/nameless/prototype
    entries handled.
- `cargo test --workspace`: ok — 410 passed, 0 failed (25 content +
  296 core + 16 command + 23 protocol + 30 server + 20 storage/datastar).
- `node --test tests/*.test.js`: ok — 61 pass, 0 fail.
- `bin/gate.sh`: **GATE GREEN [full]** — 17 passed, 0 failed.
  `mutation: 399 caught / 0 missed → MSI 100.0% (floor 100%)`; rust
  coverage 98.59% lines (floor 94), JS coverage 87.62% (floor 75). First
  FULL run was RED on gate:2 only — two strict-clippy findings in the
  NEW test code (the served test at 124/100 lines; the `gear()` fixture's
  unnecessary `Option` wrap). Fixed at source: a shared `played()` driver
  + `equipped_pairs()` helper collapsed the served loop well under the
  ceiling, and `gear()` returns the bare profile (callers wrap). Re-run
  GREEN.
- Pre-existing exclusions: none — the whole board was green before and
  after.

## Phase 5 — Complete
- Docs updated: `docs/inventory-and-items.md` (implemented-direction
  paragraph for #35), `docs/decisions.md` (Decision 053 — slot-authored,
  pack-exclusive, gear-aware-at-three-sites, with revisit triggers).
- Forge capture: AAR `ff28d674` submitted (completed, effectiveness 5;
  4 novel findings, 7 surfacing verdicts); failures
  `BF-equip-stow-dedup-guard-loses-items-001` (high) and
  `BF-equipped-gear-invisible-to-text-projections-001` (medium) recorded
  at inspect; prevention rule
  `PR-claude-state-moves-audit-source-container-projections-001` (high);
  architecture decision
  `AD-claude-equipment-slot-authored-pack-exclusive-001`.
- Ticket closed: forge #35 `3fa9b4ab` → done; local doc moved to
  `docs/planning/tickets/closed/`.
- Archived: spec+notes pair moved to
  `docs/planning/pipeline/completed/`.
