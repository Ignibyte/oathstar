---
title: TICKET-34-commerce-v1-coins-vendor-buy-sell
status: done
ticket: 96e117f5-9d77-4ca7-81cb-10e6032d12d1
ticket_number: 34
type: feature
created: 2026-06-12
closed: 2026-06-12
intake: docs/planning/intake/INTAKE-blank-colors-vertical-slice-city-forest-cave.md
pipeline_spec: docs/planning/pipeline/completed/WORK-commerce-v1.spec.md
---

# TICKET-34-commerce-v1-coins-vendor-buy-sell

## Summary

Make trade real: coins on the player, authored per-item values, vendor
stock on the existing `shopkeeper` role, and shop verbs (browse stock
with prices, `buy <item>`, `sell <item>`) with typed refusals — debuting
in Mara's Candle Shop. S1 of the blank-colors vertical slice.

## Why

The vertical slice's loop (shop → skills → battle → equipment → oath)
has no shop. The substrate has waited for it: `Role::Shopkeeper` shipped
in #21 with "shop metadata is a future ticket" in its doc comment; Mara
already carries the tag, an inventory, and the shop room;
`docs/inventory-and-items.md` reserves per-item `Value`, a `Currency`
type, and `No-trade` flags; #20's inventory v1 named shops a target
consumer.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player uses the shop-browse verb near a shopkeeper, the engine shall list the vendor's stock with name and price per line. | Rust test |
| REQ-002 | When the player buys an affordable stocked item, the engine shall move it from vendor stock to the pack and deduct the price exactly once, observable in the snapshot. | Rust test |
| REQ-003 | If the player buys with insufficient coins, names an unstocked item, or no shopkeeper is reachable, the engine shall refuse with a typed line and change no state. | Rust test (each arm) |
| REQ-004 | When the player sells a carried sellable item, the engine shall move it to vendor stock and credit the sell price exactly once; unsellable items (no-trade/oath flags or zero value) shall be refused typed. | Rust test (both arms) |
| REQ-005 | The designed coin source shall provide coins in the played beginner world (victory reward and/or sell), proven over the server seam. | server test |
| REQ-006 | The client shall display the player's coins via the snapshot with no protocol breakage. | node --test + smoke |
| REQ-007 | Save/load shall round-trip coins and vendor stock mutations; crafted extreme coin values shall not panic any new arithmetic. | Rust test + operator sweep |
| REQ-008 | Existing engine/client behavior and the gate shall continue to pass. | gate |

## Scope

- In: coins on `PlayerState` (+ snapshot + save); authored `Item.value`;
  vendor stock semantics on the `shopkeeper` role (design decides:
  inventory-as-stock vs separate stock list, and the role contract);
  browse/buy/sell verbs via the established parser patterns; the
  refusal/settlement rules (typed, no state change on refusal); the coin
  faucet (design decides: victory rewards like `CombatProfile.xp` and/or
  sell values); Mara's authored stock/prices in `world.toml`; thin coin
  display in the client; crafted-value sweep; deterministic tests + the
  served shop loop; docs.
- Out: equipment/wearing (S2); stacking/quantities/weight; multiple
  vendors; haggling/markup curves; crafting; item use; restocking;
  economy sinks beyond buy/sell.

## Notes

- Forge ticket: `96e117f5-9d77-4ca7-81cb-10e6032d12d1` (#34)
- Related docs: `docs/inventory-and-items.md` (Item Model: Value,
  Currency, No-trade), `docs/mechanics-and-systems.md`,
  `docs/decisions.md` (004 additive content, 041 server-computes)
- Promoted from intake: `INTAKE-blank-colors-vertical-slice…` (step S1)
- Active pipeline: `WORK-commerce-v1`
- Anchors verified at plan: `Role::Shopkeeper` exists (lib.rs:172,
  tag "shopkeeper", no contract yet); Mara is
  `["talkable", "oath_giver", "shopkeeper"]` with `inventory =
  ["candle"]` (world.toml:44-51); `Item` carries
  id/name/description/aliases/hidden/kind/flags — `value` is new
  (serde-defaulted); `PlayerState` gains `coins` (xp:u64 precedent;
  serde default keeps v2 saves loading — design decides default vs
  version bump); `SAVE_FORMAT_VERSION = 2` (lib.rs:998); the talk/take
  proximity-resolution pattern is the vendor-reach seam; parser: `l` is
  taken (look) but `list`/`shop`/`browse`/`buy`/`sell` are free.
