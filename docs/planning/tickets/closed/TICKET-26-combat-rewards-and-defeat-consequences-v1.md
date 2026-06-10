---
title: TICKET-26-combat-rewards-and-defeat-consequences-v1
status: closed
ticket: aa8c7f72-991a-4c30-9bb7-80b41caa2172
ticket_number: 26
type: feature
created: 2026-06-09
closed: 2026-06-09
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-combat-rewards-and-defeat-consequences-v1.spec.md
---

# TICKET-26-combat-rewards-and-defeat-consequences-v1

## Summary

Turn combat into a complete grind loop by adding deterministic victory rewards,
simple item drops, and the first real defeat consequence. Combat already works;
this ticket makes winning and losing matter.

## Why

Tickets #22, #24, and #25 built the encounter, pulse, and direct-verb combat
foundation. The next missing game-feel layer is the reward/consequence loop:
players should fight, gain something, see drops, recover from defeat, and have
the result summarized clearly in the feed/modal.

This should stay deliberately smaller than the full progression system. Skill
usage, classes, currency, reputation, equipment scaling, respawn tables, and
loot rarity are later tickets.

## Locked v1 Decisions

- Victory may award authored XP from the defeated hostile's `CombatProfile`.
  Missing XP metadata defaults to `0` so existing combat fixtures remain valid.
- Victory drops the defeated hostile's authored `inventory` into the defeated
  room as ground items, then clears that inventory to avoid duplicate drops in
  the current session.
- Defeated hostile room placement remains removed for the current session.
- Defeat resets the player to the world start room, restores HP to max, clears
  combat, leaves the enemy in the world, and applies a deterministic XP penalty:
  if current XP is greater than zero, lose `max(1, floor(current_xp / 10))`; XP
  never goes below zero.
- Reward and penalty output rides the existing combat/feed surfaces. Do not add
  a new modal or protocol event kind unless design proves the existing
  `CombatEnded` text plus existing snapshots are insufficient.

## EARS Requirements

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

## Scope

- In: additive reward metadata, victory XP, defeated-hostile inventory drops,
  defeat reset/XP penalty, beginner-module demo content, tests, docs.
- Out: skill XP/percentages, levels, class unlocks, currency, shops, equipment,
  random loot, respawn rules, corpse entities, item stacking, reputation,
  alternate resolution rewards, boss-specific reward scripting, and UI redesign.

## Notes

- Forge ticket: pending. Mint this in Forge before implementation if the Forge
  connector is available, then update this frontmatter.
- Keep the engine deterministic and server-authoritative. No RNG.
- Keep the protocol additive. Prefer existing `player.xp`, `room.contents`,
  `pack`, and `CombatEnded`/`CombatMessage` surfaces.
- Preserve existing #24/#25 timing rules: Phase 1 can still preempt queued
  actions, and Phase 2 victory should award rewards exactly once.
- Use exact-string and by-value tests for rewards/penalties; these will matter
  for mutation coverage.
