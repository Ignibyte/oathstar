---
pipeline_id: c76c3dc4-5d49-4458-af50-c142f5bdbedd
title: WORK-combat-rewards-and-defeat-consequences-v1
ticket: aa8c7f72-991a-4c30-9bb7-80b41caa2172
type: work
intake:
notes: WORK-combat-rewards-and-defeat-consequences-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-combat-rewards-and-defeat-consequences-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Combat Rewards + Defeat Consequences v1 — the grind loop closes:
  victory awards the defeated hostile's authored XP (default 0) and drops its
  authored inventory into the room as takeable ground items (cleared, no
  session duplicates); defeat resets the player to the world start room at
  max HP, clears combat, leaves the enemy in place, and applies the
  deterministic XP penalty `max(1, floor(xp/10))` floored at zero — all
  riding the existing combat/feed surfaces, with a beginner demo hostile.
- **Scope:**
  - **In:** additive reward metadata (`CombatProfile` XP; a new additive
    entity `inventory` of item ids); victory XP awarded exactly once through
    `end_combat`; defeated-hostile drops into the existing room-item/`take`
    flow; the defeat reset + penalty; beginner-module demo content (a
    reachable hostile granting XP and dropping a simple item); exact-value
    Rust tests + any JS/content tests REQ-007/008 need; docs.
  - **Out:** skill XP/percentages, levels, class unlocks, currency, shops,
    equipment, random loot, respawn rules, corpse entities, item stacking,
    reputation, alternate-resolution rewards, boss-specific reward scripting,
    UI redesign. No RNG anywhere.
- **Systems:** combat, engine, content, inventory(reuse), protocol(no new
  kinds expected), ui(light).

## Acceptance Criteria (EARS)
Verbatim from `TICKET-26` (forge `aa8c7f72-991a-4c30-9bb7-80b41caa2172`).

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a hostile with an authored XP reward is defeated by any victory path, the engine shall add that XP to the player and expose the updated XP in the command/tick snapshot. | Rust test |
| REQ-002 | When a hostile without authored XP reward is defeated, the engine shall preserve existing victory behavior and award zero XP without panicking or inventing a reward. | Rust test |
| REQ-003 | When a defeated hostile owns inventory items, the engine shall move those item references into the defeated room as ground items, clear the defeated entity inventory for the session, and make the drops visible/takeable through the existing room contents and `take` flow. | Rust test |
| REQ-004 | When a hostile has no inventory, victory shall preserve existing removal behavior and shall not add phantom drops. | Rust test |
| REQ-005 | When the player is defeated, the engine shall restore HP to max, move the player to the world start room, clear combat, leave the enemy in place, and apply the deterministic XP penalty without underflow. | Rust test |
| REQ-006 | When the player has zero XP and is defeated, the engine shall apply no XP penalty and shall still reset location/HP/combat state correctly. | Rust test |
| REQ-007 | When combat ends through victory, defeat, or flee, the output shall summarize the result clearly enough for the event feed/battle history without adding a new client-only inference path. | Rust/JS test or review |
| REQ-008 | The beginner module shall demonstrate the loop with at least one reachable hostile that grants XP and drops a simple item. | content test + playable smoke |
| REQ-009 | Existing combat pulse behavior, direct battle verbs, nearby hostile affordances, inventory/take/drop, oath/boss confront flow, Datastar feed, and client build shall continue to pass. | gate |

## Locked-In Decisions
Settled before design (the ticket carries locked v1 decisions); not
re-litigated mid-pipeline. Open design choices are in the notes for Phase 2.

- **Victory XP is authored on `CombatProfile`, defaulting to 0** — additive
  `#[serde(default)]`, so every existing fixture and the Bell-Eater stay
  valid; a missing reward means zero, never an invented value. Award happens
  in `end_combat`'s Victory arm — the single funnel both victory paths
  (Phase-1 round and the #25 Phase-2 power strike) already pass through, so
  "exactly once" is structural. `PlayerSnapshot.xp` is already on the wire.
- **Drops come from a NEW additive entity `inventory`** (authored item ids):
  on victory they move into the defeated room's item placements — visible
  and takeable through the existing #17/#18 contents/`take` flow — and the
  entity inventory clears (no duplicate drops this session). No corpse
  entities; the hostile's room placement stays removed (#22 semantics). The
  validation contract (inventory ids must resolve in `world.items`,
  #21-style) is a design question to settle, not skip.
- **Defeat semantics change is AUTHORIZED**: reset to `world.start_room_id`,
  HP restored to max, combat cleared, enemy left in place, XP penalty
  `max(1, floor(current_xp / 10))` applied only when `current_xp > 0`, XP
  never below zero (u64, saturating). The #22/#24 in-place-revive tests
  (`defeat_at_zero_hp_revives_player_and_clears_combat`,
  `pulse_defeat_at_exact_zero_revives_and_stops_pulsing`) are deliberately
  UPDATED as part of this change — documented behavior change, not
  regression. Defeat can only occur in the encounter room (movement
  disengages as fled — `BF-combat-pulse-follows-player-001`), so the reset
  has no mid-walk edge.
- **Output rides existing surfaces.** Reward/penalty/drop narration goes
  through `CombatEnded` text and/or `CombatMessage` lines on the Combat
  channel; NO new modal and NO new protocol event kind unless design proves
  the existing surfaces insufficient (the ticket's explicit bar). The
  defeat-reset must leave the client coherent (the existing
  `combat_ended`-triggered `/state` refresh re-renders room/map/HUD).
- **Deterministic, no RNG; #24/#25 timing preserved.** Phase 1 can still
  preempt queued actions; pulse cadence untouched. Exact-string and by-value
  tests for XP, drops, defeat reset, and no-duplicate rewards (the mutation
  discipline; `PR-claude-enumerate-variant-string-arms-001` applies to any
  new per-variant strings).
- **Process bounds (owner-set).** Validate runs `cargo test --workspace`,
  `node --test`, `npm run build`, and `./bin/gate.sh --fast`; the FULL gate
  and `/commit` are owner-gated. STOP after Phase 5 (complete + archive).
  Untracked `assets/tilesets/` and `bin/generate_oathstar_tileset.py` are
  Codex-owned — untouched.

## Linked Artifacts
- Design docs: `docs/combat-system.md`, `docs/decisions.md` (040–043),
  `docs/mechanics-and-systems.md`, `docs/entity-model.md`,
  `docs/protocol-and-output.md`, `docs/ui-design.md`
- Intake doc: none
- Ticket doc: `docs/planning/tickets/closed/TICKET-26-combat-rewards-and-defeat-consequences-v1.md`
- Forge ticket: `aa8c7f72-991a-4c30-9bb7-80b41caa2172` (#26)
- AAR: `e152bf2c-a059-40a8-bcf6-9db136737236`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
