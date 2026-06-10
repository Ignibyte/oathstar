---
pipeline_id: 6bc790a0-d006-4b16-86ed-3bff8ea327e6
title: WORK-boss-fight-v1
ticket: 89436a8c-4038-4c05-8519-ff28059e3626
type: work
intake:
notes: WORK-boss-fight-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-boss-fight-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Boss Fight v1 — `confront` starts a real pulse-loop encounter
  with the Bell-Eater (replacing the scripted instant win); victory drops
  the clapper through the existing reward funnel; recovering the
  oath-flagged clapper is what fulfills the oath and rings the bell.
- **Scope:**
  - **In:** the confront→combat entry (sworn + boss present → a real
    `CombatState` encounter, reusing the #24 pulse loop and #25 direct
    verbs wholesale — no new combat mechanics); boss victory through the
    existing #26 `end_combat` funnel (removal, authored inventory drop,
    authored xp) WITHOUT fulfilling the oath; fulfillment moved to clapper
    recovery (taking the oath-flagged item while sworn → `OathFulfilled` +
    the #27 authored announcements + Mara's fulfilled dialogue, all riding
    unchanged machinery); defeat = the existing #26 consequences with the
    boss, its inventory, and the sworn oath intact for a retry; the #16
    oath-gates-the-boss interlock preserved (unsworn/post-fulfillment
    confront refused, both arms); content (bell_eater authored attack + xp,
    roost combat gating as the room contract requires); deterministic
    engine tests + the full served played route; docs.
  - **Out:** alternate resolutions (persuade/spare/bind — Decision 007's
    future), boss phases/special moves, new verbs/commands, levels, focus
    costs, multiplayer, UI redesign (the battle modal already renders
    combat), new save/announcement machinery (preservation only).
- **Systems:** combat, oath, engine, content, inventory(reuse),
  protocol(none expected), ui(none expected).

## Acceptance Criteria (EARS)
Verbatim from `TICKET-29` (forge `89436a8c-4038-4c05-8519-ff28059e3626`).

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the player confronts with the oath sworn and the boss present, the engine shall start a real combat encounter with the boss (CombatState active, pulses resolving) instead of resolving instantly. | Rust test |
| REQ-002 | When the boss falls in combat, the engine shall remove it, drop its authored inventory into the room, and award its authored XP through the existing end_combat funnel — without fulfilling the oath yet. | Rust test |
| REQ-003 | When the player takes the oath-flagged clapper while the oath is sworn, the oath shall be fulfilled — emitting OathFulfilled, delivering the authored fulfillment announcements per scope, and Mara's fulfilled dialogue shall be reachable. | Rust test |
| REQ-004 | If the player confronts without a sworn oath or after fulfillment, the engine shall refuse with distinguished messages and start no combat. | Rust test (both arms) |
| REQ-005 | If the player is defeated by the boss, the existing defeat consequences shall apply (reset to start with HP restored, deterministic XP penalty) and the boss, its inventory, and the sworn oath shall remain intact for a retry. | Rust test |
| REQ-006 | The played route shall run end to end over the server seam: talk Mara → swear → climb → confront → pulse fight → victory → clapper drops → take clapper → fulfilled + world announcement → Mara's fulfilled line. | server test |
| REQ-007 | Mid-boss-fight saves shall round-trip through the #28 surface without new save work. | Rust test |
| REQ-008 | Existing combat/pulse/direct-verb/reward/announcement/save behavior and the client build shall continue to pass. | gate |

## Locked-In Decisions
Settled before design; not re-litigated mid-pipeline. The open *design*
choices they leave are enumerated in the notes for Phase 2 to settle.

- **Confront becomes the combat entry, not a parallel combat.** The fight
  IS the existing encounter system — `CombatState`, the tick-riding pulse
  loop, attack/flee/guard/power-strike — entered through `confront`'s
  existing gate (sworn oath + boss in the room). No second combat model,
  no boss-specific mechanics.
- **Victory does not fulfill; recovery fulfills.** The boss falling runs
  the existing `end_combat` Victory funnel (removal, drop, xp) and the
  oath stays Sworn; TAKING the oath-flagged clapper while sworn is the
  fulfillment trigger — `OathFulfilled`, the #27 authored announcements,
  and Mara's fulfilled dialogue all fire from there. The oath text
  ("recover the bell's stolen clapper") becomes mechanically honest.
- **The interlock survives.** Unsworn confront and post-fulfillment
  confront remain refusals (both arms staged); the oath still gates the
  boss (#16's load-bearing decision).
- **Defeat is the existing #26 path.** Reset to start with HP restored +
  the deterministic xp penalty; the boss, its inventory, and the sworn
  oath survive for the retry. No corpse runs, no boss-specific defeat.
- **Content stays authored; the engine stays generic.** Boss danger is
  TOML (`attack`, `xp` on bell_eater's combat profile; roost combat
  gating); no boss fiction in engine code. Numbers land at design
  (winnable-but-dangerous vs hp 20 / strike 4 / guard / power strike 6).
- **Preservation is a requirement, not a hope.** Mid-boss-fight saves
  round-trip through #28 unchanged; the stray's #22–#26 behavior, the
  announcements' both-arms demo, and the client build keep passing.
- **Process bounds (owner-set).** Validate runs `cargo test --workspace`,
  `node --test tests/*.test.js`, `npm run build`, `./bin/gate.sh --fast`;
  the FULL gate and `/commit` are owner-gated — STOP after Phase 5.
  Untracked `assets/tilesets/` + `bin/generate_oathstar_tileset.py` are
  untouchable.

## Linked Artifacts
- Design docs: `docs/decisions.md` (007, 040, 042–046), `docs/combat-system.md`,
  `docs/mechanics-and-systems.md` (Conflict), `docs/event-lifecycle.md`,
  `docs/spatial-awareness.md` (hidden/confront note)
- Intake doc: none — carried observation from the #22–#28 session
- Ticket doc: `docs/planning/tickets/open/TICKET-29-boss-fight-v1-confront-the-bell-eater.md`
- Forge ticket: `89436a8c-4038-4c05-8519-ff28059e3626` (#29)
- AAR: `5ec2ffca-7911-43ba-a1a0-93cb3ab6b7e3`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS (see notes — entry shape, objective fulfillment, refusal audit, locked numbers) |
| 3 — Implement | PASS (5 anticipated-churn tests red, handed to validate — see notes) |
| 3.5 — Inspect | PASS (8 real findings fixed incl. doubled fulfillment line + save version bump to 2; FAIL-claude-generic-log-twin-duplicates-typed-render-001 recorded; see notes ledger) |
| 4 — Validate | PASS (13 new tests + 5 churn rewrites; 335 workspace + node + build; GATE GREEN [fast] 14/14) |
| 5 — Complete | PASS (Decision 047 + 5 docs updated; AAR closed; AD-claude-boss-encounter-oath-gated-recovery-fulfillment-001; ticket #29 closed; archived) |
