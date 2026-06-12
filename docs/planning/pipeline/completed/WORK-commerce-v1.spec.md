---
pipeline_id: e35f61ea-af25-4b18-861f-5d2a2169f0c6
title: WORK-commerce-v1
ticket: 96e117f5-9d77-4ca7-81cb-10e6032d12d1
type: work
intake: docs/planning/intake/INTAKE-blank-colors-vertical-slice-city-forest-cave.md
notes: WORK-commerce-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-commerce-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Commerce v1 — coins, authored item values, vendor stock on
  the existing `shopkeeper` role, and browse/buy/sell verbs with typed
  refusals, debuting in Mara's Candle Shop (S1 of the vertical slice).
- **Scope:**
  - **In:** `PlayerState.coins` (+ `PlayerSnapshot`, save round-trip);
    authored `Item.value` (serde-defaulted); vendor stock semantics +
    the `shopkeeper` role contract (design decides shape); browse/buy/
    sell verbs via the established parser patterns (typed refusals, no
    state change on refusal — the #25/#31 refusal family); the
    settlement rules (deduct/credit exactly once, saturating
    arithmetic); the coin faucet (design decides: authored victory coin
    rewards à la `CombatProfile.xp`, and/or sell values); Mara's
    authored stock/prices; thin client coin display; crafted-value
    sweep; deterministic tests + the served shop loop; docs.
  - **Out:** equipment/wearing (S2); stacking/quantities/weight;
    multiple vendors; haggling/markup; crafting; item use; restocking;
    sinks beyond buy/sell.
- **Systems:** engine (state, commerce rules), content (world.toml
  values/stock), parser, protocol (additive), ui (thin), storage
  (round-trip only).

## Acceptance Criteria (EARS)
Verbatim from `TICKET-34` (forge `96e117f5-9d77-4ca7-81cb-10e6032d12d1`).

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player uses the shop-browse verb in the same room as a visible shopkeeper, the engine shall list the vendor's stock with name and price per line. | Rust test |
| REQ-002 | When the player buys an affordable stocked item, the engine shall move it from vendor stock to the pack and deduct the price exactly once, observable in the snapshot. | Rust test |
| REQ-003 | If the player buys with insufficient coins, names an unstocked item, or no shopkeeper shares the room, the engine shall refuse with a typed line and change no state. | Rust test (each arm) |
| REQ-004 | When the player sells a carried sellable item, the engine shall move it to vendor stock and credit the sell price exactly once; unsellable items (no-trade/oath flags or zero value) shall be refused typed. | Rust test (both arms) |
| REQ-005 | The designed coin source shall provide coins in the played beginner world (victory reward and/or sell), proven over the server seam. | server test |
| REQ-006 | The client shall display the player's coins via the snapshot with no protocol breakage. | node --test + smoke |
| REQ-007 | Save/load shall round-trip coins and vendor stock mutations; crafted extreme coin values shall not panic any new arithmetic. | Rust test + operator sweep |
| REQ-008 | Existing engine/client behavior and the gate shall continue to pass. | gate |

## Locked-In Decisions
Settled before design; not re-litigated mid-pipeline. Open *design*
choices are enumerated in the notes for Phase 2.

- **The `shopkeeper` role is the seam** — shipped in #21 with "shop
  metadata is a future ticket" in its contract; no new role tag. Mara's
  authoring already carries it.
- **Server-authoritative trade.** All pricing, affordability, and
  settlement compute in the engine; the client renders (Decision 041's
  principle). Typed refusals, no state change on refusal — the #25/#31
  refusal family.
- **Additive content + protocol.** `Item.value` and `PlayerState.coins`
  are serde-defaulted; old worlds/saves/payloads stay loadable
  (the established additive pattern; design pins default-vs-version-bump
  for saves).
- **Deterministic, no RNG.** Prices are authored values; settlement
  arithmetic saturates (the #31 crafted-save posture).
- **Exactly-once settlement.** Buy: item moves stock→pack AND coins
  deduct atomically in one command; sell: pack→stock AND credit
  likewise; no path can double-move or double-charge.
- **Oath integrity.** Items that gate oath progress must not be
  sellable into oblivion (the no-trade/oath-flag refusal — design pins
  the exact rule).

## Linked Artifacts
- Design docs: `docs/inventory-and-items.md` (Item Model / Flags),
  `docs/mechanics-and-systems.md`, `docs/decisions.md` (004, 041)
- Intake doc: `docs/planning/intake/INTAKE-blank-colors-vertical-slice-city-forest-cave.md` (S1)
- Ticket doc: `docs/planning/tickets/open/TICKET-34-commerce-v1-coins-vendor-buy-sell.md`
- Forge ticket: `96e117f5-9d77-4ca7-81cb-10e6032d12d1` (#34)
- AAR: (recorded in notes at Phase 1 closeout)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
