# WORK-commerce-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Ticket #34 — S1 of the blank-colors vertical slice:
  commerce v1 (coins, item values, vendor stock, browse/buy/sell at
  Mara's shop). Owner: "lets proceed to the next things" — the intake's
  recommended order continues; session pattern (run through
  commit+ff-merge+push, pause on gate failure/scope conflict) applies.
- **Intake source:** `INTAKE-blank-colors-vertical-slice…` (S1 row).
- **Classification / tier:** Work pipeline, one slice — but the LARGEST
  of the slice so far (new player state + content field + role contract
  + 3 verbs + faucet). If design balloons, the faucet (victory coin
  rewards) can split out; the slice stands on sell-the-fang alone.
- **Base verified:** `main` @ `a0ba659` (#33 merged+pushed); no active
  pipeline; forge up; no bulletins.
- **Anchors verified at plan:**
  - `Role::Shopkeeper` EXISTS (lib.rs:166-181, tag `"shopkeeper"`) — doc
    comment: "shop metadata is a future ticket". No contract enforced
    today (unlike hostile/boss → combat profile).
  - Mara (world.toml:44-51): `roles = ["talkable", "oath_giver",
    "shopkeeper"]`, `inventory = ["candle"]`, aliased "shopkeeper" —
    authored substrate ready; the candle is proto-stock.
  - `Item` (lib.rs:232-252): id/name/description/aliases/hidden/kind/
    flags. `value` is NEW (serde-default). `flags` already carries
    free-text tags (e.g. `["oath"]` on the clapper? — verify which items
    carry which flags at design).
  - `PlayerState` (lib.rs:789-798): level/xp/hp/max_hp/focus/max_focus —
    `coins` joins (`xp: u64` is the unsigned precedent).
  - `SAVE_FORMAT_VERSION = 2` (lib.rs:998); `from_save` rejects other
    versions. A serde-defaulted `coins` keeps v2 loadable with 0 coins —
    design pins default-vs-bump.
  - Verb seams: `parse_bare_verb` (browse verb), `Talk`-style
    required-target arms (`buy <item>` / `sell <item>`); `l` is look's
    alias but `list`/`shop`/`browse`/`buy`/`sell` are unclaimed; the
    talk/take proximity resolution is the vendor-reach pattern
    (interactable shopkeeper in awareness radius).
  - `CombatProfile.xp` (authored, serde-default 0, saturating award in
    `end_combat`) is the exact template for a `coins` reward field if
    design picks the victory faucet.
  - Standing rules in play: PR-oathstar-msi-test-assertions-001 (exact
    boundary values for affordability, like #31's focus);
    PR-claude-enumerate-variant-string-arms-001 (refusal-line tables —
    every arm exact-string asserted);
    PR-claude-operator-sweep-untrusted-arithmetic-001 (crafted coins at
    u64 extremes through every new +/-);
    PR-claude-mutants-package-scoped-test-placement-001 (any protocol
    helper gets in-crate tests);
    PR-claude-fixture-distinguishable-transitions-001 (stock/coin
    transitions staged before ≠ after).
- **Ticket:** forge `96e117f5-9d77-4ca7-81cb-10e6032d12d1` (#34), local
  doc `docs/planning/tickets/open/TICKET-34-commerce-v1-coins-vendor-buy-sell.md`.
- **AAR opened:** `3ddb7374-a5ba-4618-9706-f7e9e45f1cd9`.
- **EARS requirements reviewed:** REQ-001..008 (verbatim in the spec).

### Open design questions (for Phase 2 — Planner does NOT decide these)
1. **Coin representation + width.** `coins: u64` on PlayerState
   (xp precedent, saturating) vs currency-as-item (docs reserve a
   Currency item type — heavier, stacking-shaped). Lean u64 field;
   currency-items stay future.
2. **Save compatibility.** serde(default) on coins (v2 stays loadable,
   0-coin migration) vs SAVE_FORMAT_VERSION bump to 3 (loud refusal of
   old saves). The #30 lazy-convergence precedent leans
   serde(default) + no bump. Vendor stock mutations ride the existing
   world-in-save (#28 carries the MUTATED world — verify stock moves
   persist for free).
3. **Stock semantics.** Vendor `inventory` IS the stock (finite; buys
   drain it, sells add to it — a real two-way economy with zero new
   content surface) vs a separate authored `stock` list (infinite?).
   Lean inventory-as-stock. Role contract: does `shopkeeper` REQUIRE
   anything (à la hostile→combat)? Lean: no hard contract in v1 (an
   empty-handed vendor lists an empty stock honestly).
4. **Price model.** Buy price = `item.value`; sell price = value/2
   (floor 1 for value ≥ 1)? Zero/absent value ⇒ unsellable + unbuyable
   (priceless)? Pin exact integer math + the no-trade rule
   (`flags` containing `"oath"` or `"no_trade"` refuse sell — check the
   clapper/fang/candle/wax_stub flags and author values for each).
5. **Verb shapes + lines.** Browse: `shop` vs `list` vs `browse` (+
   aliases?) — bare strict-arity; works only with a reachable
   shopkeeper. `buy <item>` / `sell <item>` required-target. Refusal
   table (no vendor / unstocked / can't afford / not carried /
   unsellable) — exact strings pinned at design
   (enumerate-variant rule applies).
6. **The faucet.** Sell-only (stray fang value N funds the first buy —
   smallest) vs + authored victory coin reward (`CombatProfile.coins`,
   the xp template — makes grinding pay). Slice needs: fight stray →
   coins → buy something meaningful at Mara's. What does Mara stock
   that matters pre-equipment? (candle + wax_stub are flavor; a
   `healing draught`-ish consumable is OUT (item use is out of scope) —
   maybe the candle as the demo purchase is honest enough for v1, with
   equipment stock landing in S2.)
7. **Vendor reach rule.** Same-cell/interactable shopkeeper via the
   awareness resolver (talk's rule) — and WHICH vendor when several
   (nearest? first?) — v1 has one; pin the deterministic pick anyway.
8. **Client surface.** `PlayerSnapshot.coins` + where it renders: HUD
   chip beside XP? Menu/equipment tab? Lean HUD text (toHud) — smallest
   honest display. Does the Nearby panel's vendor entry gain a "shop"
   affordance chip (the #23 threat pattern) — or out (verbs suffice)?
9. **Mid-combat trading.** Refuse shop verbs in combat (the rest
   precedent) or allow? Lean refuse (combat is committed).
10. **Mara's dialogue.** Optional greeting tweak mentioning trade —
    content polish, decide in/out.

## Phase 2 — Design

- **Recall (12 surfacings):** AD focus-economy (#31 — the
  spend/refusal/settlement template commerce mirrors); AD map-markers
  (#33 — reveal-rule-on-every-projection: a hidden shopkeeper must not
  trade); PR msi-test-assertions (exact affordability boundaries);
  PR enumerate-variant-string-arms (the refusal table — every arm
  exact-pinned); PR operator-sweep (crafted u64 coins through every new
  +/−); PR mutants-package-scoped (protocol additions tested in-crate).
  Seams verified: `talk_at`'s resolve→actor→interactable ladder;
  `toHud`; clapper already `flags = ["oath"]`.

### Approach / architecture (settles the 10 Phase-1 questions)

1. **Coins (Q1): `coins: u64` on `PlayerState`**, serde-default 0 (the
   `xp` precedent). Currency-as-item stays future.
2. **Saves (Q2): NO version bump.** `coins`/`value`/`CombatProfile.
   coins` all serde-default — a v2 save without them loads as a coinless
   player over priceless items (sound state, #30's
   tolerate-and-converge posture). Vendor stock mutations persist FREE:
   SaveData carries the mutated world incl. entity inventories (the #26
   `mem::take` victory drops prove inventories live in the saved world).
3. **Stock (Q3): vendor `inventory` IS the stock.** Finite and two-way —
   buying drains it, selling adds to it. No new content surface, no hard
   role contract in v1 (an empty-handed vendor lists honestly).
4. **Prices (Q4):** authored `Item.value: u64` (serde-default 0).
   Buy price = `value`; `value == 0` ⇒ not for sale ("won't part
   with"). Sell price = `max(1, value / 2)` for `value ≥ 1`;
   `value == 0` ⇒ unsellable ("has no use for"). `flags` containing
   `"oath"` ⇒ never sellable (the clapper, already authored so).
   Integer math only.
5. **The faucet (Q6): BOTH — and the numbers make the loop tight.**
   `CombatProfile.coins: u64` (serde-default 0, the `xp` field's exact
   template, awarded saturating in `end_combat` beside xp). Authored:
   stray `coins = 4`, bell_eater `coins = 25`; values: candle 8,
   wax_stub 2 (sell 1), fang 6 (sell 3), clapper 0 + oath flag
   (double-locked). The played loop: stray fight (+4) + sell fang (+3)
   + sell wax stub (+1) = 8 → exactly one candle. Sell-only was
   rejected: the fang is a ONE-TIME drop (strays don't respawn), so
   sell-only caps the lifetime economy below the cheapest item.
   Victory line composition (4 arms, all exact-pinned): both → "You
   gain {xp} XP and {coins} coins."; xp-only → byte-identical to #26;
   coins-only → "You gain {coins} coins."; neither → bare victory.
6. **Verbs (Q5):** `shop` (alias `browse`) — bare strict-arity via
   `parse_bare_verb`; `buy <item>` / `sell <item>` — required-target
   (the `talk`/`take` arm pattern). Help line gains all three.
7. **Vendor reach (Q7): same-room only**, first placed entity with
   `has_role(Shopkeeper)` AND `!hidden` (the #33 reveal-rule lesson) in
   `room.entities` order — deterministic with multiple vendors, honest
   for v1 (you trade standing in the shop). `find_vendor(&self) ->
   Option<&Entity>`.
8. **Refusal table (exact strings, every arm unit-pinned per the
   enumerate rule; System channel, no state change):**
   - no vendor (all 3 verbs): "There is no shopkeeper here to trade with."
   - in combat (all 3): "There is no trading in the midst of battle."
   - shop, empty stock: "{vendor} has nothing to sell."
   - buy, unstocked: "{vendor} does not have '{target}'."
   - buy, priceless: "{vendor} won't part with {name}."
   - buy, can't afford: "You cannot afford {name} ({price} coins; you have {coins})."
   - sell, not carried: "You are not carrying '{target}'."
   - sell, oath-bound: "{name} is bound to your oath — you cannot sell it."
   - sell, worthless: "{vendor} has no use for {name}."
   Success (Narrative): buy "You buy {name} for {price} coins. ({coins}
   remain.)"; sell "You sell {name} for {price} coins. ({coins} now.)";
   shop listing follows `list_pack`'s existing event shape (header
   "{vendor} offers:", per-item "{name} — {price} coins", footer "You
   have {coins} coins."; priceless stock listed as "not for sale").
9. **Settlement (exactly-once):** buy = guard (vendor→stock item→price
   >0→coins ≥ price) then ONE mutation block: remove first matching id
   from vendor inventory, push to pack, `coins = coins.saturating_sub
   (price)` (guard makes underflow impossible; saturating is
   defense-in-depth). Sell mirror: remove from pack, push to vendor
   inventory, `saturating_add(sell_price)`. Stock/pack item matching
   mirrors `find_in_pack`'s name/alias rule (verify exact matcher at
   implement).
10. **Mid-combat (Q9): refused** (the `rest` precedent). **Client
    (Q8):** `PlayerSnapshot.coins` (serde-default for old payloads);
    `toHud` gains `coins`; the header line appends " · {coins}c"
    (where "Lv N · M xp" renders). Vendor affordance chips: OUT.
    **Dialogue (Q10): OUT.**

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-protocol/src/lib.rs` | `PlayerSnapshot.coins: u64` (serde default; in-crate wire test) |
| 2 | `crates/oathstar-core/src/lib.rs` | `PlayerState.coins`, `Item.value`, `CombatProfile.coins` (all serde-default); `find_vendor`; `shop()`/`buy_at()`/`sell_at()`; `handle_command` arms; help line; `enemy_coin_reward` + award in `end_combat` + 4-arm victory line; snapshot copies coins; tests |
| 3 | `crates/oathstar-core/src/command.rs` | `Command::Shop`/`Buy{target}`/`Sell{target}`; `shop`/`browse` bare arms; `buy`/`sell` required-target arms; parser tests |
| 4 | `modules/beginner/world.toml` | values: candle 8, wax_stub 2, fang 6, clapper 0; rewards: stray `coins = 4`, bell_eater `coins = 25` |
| 5 | `src/client/snapshot.js` | `toHud` gains `coins` |
| 6 | `src/client-app.js` | header line appends the coin count |
| 7 | `tests/client.test.js` | toHud coins (+ absence default) |
| 8 | `crates/oathstar-server/src/main.rs` | served shop-loop test |
| 9 | `docs/*` | (Phase 5) commerce implemented notes + decision |

### Regression Test Plan
| # | Test | Proves Requirement |
|---|---|---|
| C-T1 | core: `shop` near Mara lists stock with exact lines (candle — 8 coins; footer coins count); empty-stock vendor arm ("nothing to sell") | REQ-001 |
| C-T2 | core: buy at coins == price (boundary: accepted, coins → 0, candle vendor→pack exactly once) and coins == price−1 (refused exact line, NOTHING moved) | REQ-002/003 |
| C-T3 | core: every buy refusal arm exact + zero state change — unstocked, priceless (value 0), no vendor in room, mid-combat | REQ-003 |
| C-T4 | core: sell the fang (+3, pack→vendor stock, exactly once); sell-price floor pins: value 1 → 1, value 2 → 1, value 6 → 3 | REQ-004 |
| C-T5 | core: every sell refusal arm exact — not carried, oath-bound (clapper), worthless (value 0), no vendor, mid-combat | REQ-004 |
| C-T6 | core: victory coin award — stray +4 beside xp; ALL FOUR line-composition arms exact (both/xp-only/coins-only/neither — the #26 xp-only line stays byte-identical) | REQ-005 |
| C-T7 | core: buy+sell mutations round-trip a save byte-identically (coins, pack, vendor stock); an old v2 payload WITHOUT coins/value keys loads (defaults 0) | REQ-007 |
| C-T8 | core: crafted sweep — coins u64::MAX + victory award saturates; sell credit at MAX saturates; buy guard at extreme prices; value u64::MAX sell price; no panics | REQ-007 |
| C-T9 | protocol (in-crate): `coins` serializes camelCase, old payload without it deserializes to 0 | REQ-006 |
| C-T10 | parser: `shop`/`browse` bare strict-arity; `buy`/`sell` required-target; near-misses Unknown | REQ-001-004 |
| C-T11 | js: `toHud` coins + absence default 0 | REQ-006 |
| C-T12 | server: the played loop over the seam — walk to the stray, 3 manual strikes (victory: +4 coins, fang drops), take fang, walk to Mara, sell fang (+3) + sell wax stub (+1) = 8, buy candle (→ 0 coins, candle in pack, Mara's stock holds the fang+stub), /state coins exact at every step | REQ-005/006 |
| — | gate full suite (intentional content change: rewarded victories now also pay coins — the served boss-line assert and any exact victory-line tests update accordingly, documented) | REQ-008 |

Genuinely uncoverable: none new (no canvas/Image involvement; the
header-line render is textContent, smoke-checkable but also asserted via
toHud).

### Risks / decisions
- **D1 both faucets** — sell-only mathematically caps below the
  cheapest item (one-time fang); victory coins use the xp template.
- **D2 inventory-as-stock** — finite two-way economy, zero new content
  surface; revisit at restocking/multiple-vendor tickets.
- **D3 no save bump** — all new fields serde-default; old saves =
  coinless players, sound.
- **D4 oath-flag no-trade** — the clapper is double-locked (flag +
  value 0); generalizes to any future oath item.
- **D5 same-room vendor reach** — simplest honest rule; the awareness
  radius stays talk/look's domain.
- **R1 — victory-line test updates**: boss/stray exact-line asserts in
  core+server change because rewarded wins now also pay coins —
  intentional, enumerated at validate (the unrewarded-win line stays
  byte-identical).
- **R2 — `browse` alias collision check** at implement (grep parser for
  the token; believed free).
- **R3 — stock matching rule** mirrors `find_in_pack` — verify its
  name/alias case rule at implement and reuse, don't fork.

## Phase 3 — Implement
- Built (to the manifest):
  - State: `PlayerState.coins`, `Item.value`, `CombatProfile.coins`,
    `PlayerSnapshot.coins` — all serde-defaulted; try_new seeds 0;
    snapshot copies coins.
  - Engine: `sell_price` const fn (`max(1, value/2)`); `find_vendor`
    (same-room, first placed, `!hidden` + `Shopkeeper`); shared
    `trade_combat_refusal`/`trade_vendor_refusal`; `shop()` (joined
    Inventory-channel listing, "not for sale" for priceless stock,
    empty-stock refusal); `buy_at` (resolve over stock via the shared
    name/alias matcher → priceless → affordability → exactly-once
    settle); `sell_at` (pack resolve → oath-flag → worthless →
    settle); `enemy_coin_reward` + saturating award in `end_combat` with
    the 4-arm victory line (xp-only arm byte-identical to #26).
  - Parser: `Command::Shop`/`Buy`/`Sell`; `shop`/`browse` bare arms;
    `parse_trade_verb` helper (required targets).
  - Content: candle value 8, wax_stub 2, fang 6 (clapper stays
    priceless + oath-flagged); stray `coins = 4`, bell_eater
    `coins = 25` — the loop lands at exactly 8 = one candle.
  - Client: `toHud.coins`; header renders "Lv N · M xp · K coins".
- Deviations from design (+ reason):
  - **`handle_command` + `parse` both tripped clippy too_many_lines**
    (the #19/#20 recurring class, predicted at design): fixed at source
    by `parse_trade_verb` (the established split-helper pattern) and a
    new `Engine::acted(events, action)` helper that folds every
    `(accepted, events)` verb arm to one line — handle_command shrank
    well under the ceiling and the recurring class is structurally
    closed for future verbs.
  - **Victory-line asserts updated at implement, not validate** (two
    server tests): rewarded victories now also pay coins — leaving the
    suite red into inspect would be worse than updating the expected
    strings for the intentional content change (documented R1).
  - **A careless regex while patching test fixtures mangled the
    CombatProfile struct's doc comment** — caught immediately by cargo
    check, repaired by hand; lesson noted for inspect (script edits to
    source need anchored patterns, not `[^}]*?` over multi-line Rust).
- `cargo check`/`fmt`/clippy (0 issues) clean; full workspace suite
  green (266 core / 21 protocol / 28 server / 16 content / 23
  datastar / 20 storage); JS 59/59.

## Inspect (Phase 3.5)
- Lenses run: 3 critics — (1) correctness + economy exploits (arbitrage,
  dup-id settlement, self-dealing, murder-hobo, content math),
  (2) state-integrity + crafted saves (operator sweep executed at u64
  extremes via a scratch harness, save round-trip, regex-mangle audit),
  (3) plan-integrity + conventions + mutation (`cargo mutants --list`
  ground truth, gate:10 grep, reuse).
- Findings:
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | 1 | major | Hidden vendor STOCK bypassed the reveal rule — listed with a price and BUYABLE while `look` concealed the same item (executed crafted-save probe); `find_vendor` filtered the hidden vendor but neither stock chain filtered hidden items (shop ~1915, buy_at ~1964) | REAL — the #33 class recurring through a new projection | `.filter(\|item\| !item.hidden)` in both chains (hidden stock reads as unstocked). Forge: BF-claude-hidden-stock-bypasses-reveal-rule-001 + PR-claude-reveal-rules-on-every-projection-001 (high — second occurrence in consecutive tickets) |
  | 2 | minor | Buying a sworn oath's objective did not fulfill the oath — `take_at` was the only acquisition path calling `fulfill_oath_on_recovery` (executed probe: bought clapper left oath Sworn); module-facing latent (clapper is value-0 + never stocked in beginner) | REAL (both critics independently) | `buy_at` now extends events with `fulfill_oath_on_recovery(&item_id)` after the purchase line (take_at's ordering) — acquisition is acquisition |
  | 3 | low | `sell` silently sold the first ambiguous pack match while `drop` (freely reversible) refuses ambiguity — the LOSSY action had the weaker rule | REAL | `sell_at` resolves via `matching_pack_items` and refuses `len > 1` with drop's exact ambiguity line |
  | 4 | high (plan) | Mutation gap: `find_vendor`'s `&&`→`∥` mutant had NO planned killer (no hidden-vendor test row; bare no-vendor rooms kill nothing); `shop`'s `>`→`>=` on the not-for-sale arm likewise (C-T1 listed only the candle) | REAL (`cargo mutants --list`) | Test plan amended: hidden-SHOPKEEPER refusal row, visible-bystander no-vendor staging, the "not for sale" listing arm, `shop` mid-combat/no-vendor rows, hidden-STOCK rows (finding 1), buy-fulfills-oath + sell-ambiguity rows (findings 2–3), C-T6's coins-only arm noted crafted-profile-only |
  | 5 | info | No role contract forbids `hostile`+`shopkeeper`: sell-then-slay refunds the goods and keeps the coins (authored-content foot-gun; beginner double-protected — Mara not hostile, shop room not combat_enabled) | NOTED — classic MUD murder-hobo semantics, acceptable v1; a contract validator is future work if unwanted | Ledger note only |
  | 6 | info | Spec wording "near/reachable a shopkeeper" vs the pinned same-room rule — Mara is awareness-TALKABLE from the square where `shop` refuses ("here" is honest) | NOTED — consistent-by-design (D5) | Phase 5 tightens spec wording to "in the same room"; validate stages the no-vendor test from the square beside talkable Mara |
  | 7 | low | world.toml reward comments + the unlogged header-format deviation (" · {coins}c" pinned vs "· K coins" rendered) | REAL (cosmetic) | Comments appended at validate alongside test work; deviation now logged here: the header renders `· {coins} coins` (clearer than the pinned `{coins}c`) |
- Verified clean (evidence-backed): no buy-sell arbitrage (`sell ≤ buy` ∀v≥1;
  v=1 break-even shuttle is profit-free); exactly-once settlement under
  duplicate ids both directions; self-dealing consistently lossy; buy/take
  surfaces disjoint; no steal path (awareness never surfaces inventories);
  combat guard first in all three verbs; `acted` refactor behavior-identical
  (374/374 green); victory 4-arm match keeps #26/#22 lines byte-identical;
  both vendor `expect`s invariant-true; parser collision-free; content math
  exact (4+3+1=8=candle); operator sweep clean at u64 extremes (coins=MAX
  sell/buy/award all saturate, no panic); old-save compat executed
  (stripped-keys v2 payload loads and plays); save round-trip carries stock
  mutations; no pre-recorded JSON fixtures to break; determinism holds;
  the Phase-3 regex incident audited — no other mangle beyond the repaired
  doc comment; gate:10 grep zero hits; no dead code; reuse honored
  (name_or_alias_matches, find_in_pack→matching_pack_items, list_pack shape).

## Phase 4 — Validate
- Tests added (17 new — every C-T row + every inspect amendment):
  - `oathstar-core` (13): `commerce_world`/`commerce_engine` fixture
    (vendor "keeper" stocking a priced lamp + priceless relic, ground
    trinket); `shop_lists_stock_with_prices` (priced + not-for-sale arms
    exact, coins footer, empty-stock refusal);
    `hidden_stock_is_neither_listed_nor_buyable` (inspect #1);
    `a_hidden_shopkeeper_does_not_trade` (inspect #4 — the `&&→∥`
    killer, visible bystander present);
    `shop_refuses_mid_combat_and_without_a_vendor`;
    `buying_settles_exactly_once_at_the_boundary` (coins == price and
    price−1, both sides of the counter asserted);
    `buy_refusals_change_nothing` (unstocked/priceless/mid-combat/
    no-vendor); `selling_credits_the_floored_half_price` (sell_price
    pins 1→1, 2→1, 6→3 + exactly-once both sides);
    `sell_refusals_change_nothing` (not-carried/oath-bound/worthless/
    AMBIGUOUS — inspect #3); `buying_the_oath_objective_fulfills_it`
    (inspect #2, the ransom-the-relic shape on the boss-objective
    fixture); `victory_lines_compose_per_authored_rewards` (ALL FOUR
    arms exact incl. the crafted coins-only profile);
    `commerce_state_round_trips_saves` (byte-identical restore incl.
    stock mutation + stripped-keys old payload loads coinless);
    `crafted_coin_extremes_never_panic` (award/credit/buy at u64::MAX).
  - `oathstar-core` command.rs (1): `trade_verbs_parse_with_strict_shapes`.
  - `oathstar-protocol` (1, in-crate): `player_coins_serialize_and_default`.
  - `tests/client.test.js` (1): `toHud` coins + coinless default.
  - `oathstar-server` (1): `played_shop_loop_funds_the_candle` — the
    full authored loop over the seam: stub take → stray fight (+4) →
    fang take → walk back → sell fang (+3) → sell stub (+1) → buy
    candle → EXACTLY 0 coins; Mara's stock then lists the player's
    spoils and no candle (the two-way economy observed).
- One fixture retarget mid-phase: the oath-purchase test first stocked
  "bell_clapper" on the crafted vendor, but the boss fixture's authored
  objective is "sigil" (EntityItemMissing at validation — the loud
  refusal working as designed); retargeted at the fixture's own
  objective.
- `cargo test --workspace`: ALL GREEN — core 279, protocol 22, server
  29, content 16, datastar 23, storage 20; 0 failed.
- `node --test tests/*.test.js`: **60 pass, 0 fail**.
- `bin/gate.sh --fast`: first run RED on gate:1 (unformatted appended
  test code — fmt'd, suites re-confirmed green), then **GATE GREEN
  [fast]** 14/14. FULL gate at `/commit`.
- Pre-existing exclusions: none.

## Phase 5 — Complete
- Docs updated: `docs/inventory-and-items.md` (commerce-implemented
  block at Inventory Direction); `docs/mechanics-and-systems.md`
  (Conflict shipped gains the coin faucet + shop line);
  `docs/decisions.md` **Decision 052**; spec REQ-001/003 wording
  tightened to same-room (inspect #6).
- Forge capture: `aar-submit` `3ddb7374…` → completed, effectiveness 5,
  5 surfacings used, 4 rules materialized (enumerate-arms,
  operator-sweep, package-scoped-mutants, reveal-rules-on-every-
  projection). Failures + the high-severity recurrence rule captured AT
  INSPECT (BF-claude-hidden-stock-bypasses-reveal-rule-001,
  PR-claude-reveal-rules-on-every-projection-001).
  `architecture-decision-record`:
  AD-claude-commerce-value-priced-vendor-stock-001.
- Ticket closed: forge `96e117f5…` (#34) → **done**; local doc →
  closed/ (status/closed/pipeline_spec updated).
- Archived: spec+notes pair → `docs/planning/pipeline/completed/`.
