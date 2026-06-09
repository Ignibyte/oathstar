---
pipeline_id: f7c8d65b-cb0e-41a8-af47-dfdf1422ece9
title: WORK-nearby-hostile-affordances-v1
ticket: e8eaca33-1701-4009-93c6-e63007f700d7
type: work
intake:
notes: WORK-nearby-hostile-affordances-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-nearby-hostile-affordances-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Nearby Hostile Affordances + Entity Inspection — additive,
  server-authored `NearbySnapshot` combat affordances (hostile / attackable /
  attack command + a disclosure object) computed in the engine, surfaced as Nearby
  card flags + an Attack action (reusing the #22 battle modal) and a generic entity
  detail dialog. Built on combat v1 (#22).
- **Scope:**
  - **In:** additive `NearbySnapshot` fields — `hostile` (bool, default false,
    skip-if-false), `attackable` (bool, default false, skip-if-false),
    `attack_command` (Option, skip-none), and a small disclosure object for
    server-disclosed combat stats; `room_snapshot` computes them from
    `has_role(Role::Hostile)` + interactable proximity + `RoomDefinition.combat_enabled`;
    client Nearby cards flag hostile/attackable + a quiet not-attackable state + an
    Attack action (only when attackable, sends `attack <name>`, opens the #22 battle
    modal); a generic entity **detail dialog** (no command, no mutation) showing
    disclosed stats with an explicit unknown state for hidden stats; the smallest
    authored/test fixture to prove both visible and hidden disclosure; Rust +
    protocol + content + JS tests; docs.
  - **Out:** full bestiary, stealth/perception skills, random stat rolls, equipment
    comparison, loot tables, aggro AI, pathfinding, making every combatant
    attackable, **the two-phase combat loop (ticket #24)**, and any client-side
    inference of hostility/stats.
- **Systems:** combat, engine (snapshot), protocol, content, ui.

## Acceptance Criteria (EARS)
Verbatim from `TICKET-23` (forge `e8eaca33-1701-4009-93c6-e63007f700d7`).

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a Nearby actor is hostile and attackable from the player's current position, the snapshot/view model shall mark it as attackable and expose an `attack <target>` action. | Rust/JS test |
| REQ-002 | When a Nearby actor is hostile but not attackable because it is too far away or combat is disabled in the current area, the UI shall flag that state clearly and shall not offer an enabled attack action. | Rust/JS test |
| REQ-003 | When a Nearby actor is not hostile, the UI shall not show it as an enemy and shall not offer attack unless the server marks it attackable. | Rust/JS test |
| REQ-004 | When the player clicks or activates a Nearby entity card, the client shall open a focused entity detail view without sending a command or mutating game state. | JS/browser smoke |
| REQ-005 | When an entity detail view opens for a hostile actor, it shall show only server-disclosed combat stats such as health, danger, role, or visibility state, and shall render hidden stats as unknown rather than inventing values. | Rust/JS test |
| REQ-006 | When an attack action is clicked from Nearby, it shall send the same canonical command (`attack <target>`) as typing in the command prompt, and the battle modal from ticket #22 shall open on success. | JS/browser smoke |
| REQ-007 | Existing `look`, `talk`, `take`, movement, combat modal, and Datastar event-feed behavior shall continue to pass. | gate |

## Locked-In Decisions
Settled before design; the open *design* choices they leave (the exact disclosure
shape, the minimal hidden-stats fixture, the detail view-model fields, and the
not-attackable wording) are in the notes for Phase 2.

- **Server-authoritative (the ticket's hard rule).** `hostile`, `attackable`,
  `attack_command`, and disclosed stats are computed in the engine (`room_snapshot`,
  reusing `awareness::perceive` + `self.world.entities` lookup by `thing.id`) and
  exposed on `NearbySnapshot`. The client (`toNearby` / the detail view-model) ONLY
  reads server fields — it never infers hostility/attackability/stats from names,
  CSS classes, or local role strings.
- **Additive protocol only.** New `NearbySnapshot` fields use
  `#[serde(default, skip_serializing_if = …)]` (bools skip-if-false, `attack_command`
  skip-none, disclosure skip-when-absent), so a non-hostile / combatless entry is
  byte-identical to today and an old payload without the keys still deserializes
  (the #17/#18/#22 additive pattern; Decisions 028/031). No breaking change to
  `RoomSnapshot`/`GameSnapshot` shape.
- **Attackability rule (reuses combat v1 gates, Decision 040).** `hostile` =
  `has_role(Role::Hostile)`. `attackable` = the thing is an Actor **and** hostile
  **and** within interactable proximity (`proximity.is_interactable()`) **and** the
  current room is `combat_enabled`. `attack_command` = `Some("attack <name>")` iff
  `attackable`. A non-hostile actor is never attackable and gets no attack action; a
  hostile that is too far or in a non-combat area is `hostile: true, attackable: false`.
- **Disclosure uses `Option` = unknown (no enum-unknown precedent).** The detail view
  exposes only server-disclosed combat stats; an absent disclosure (or absent stat)
  renders as an explicit "unknown", never an invented value. A nearby (pre-combat)
  hostile's disclosed stats are its authored maxima (`CombatProfile.health` /
  `attack`), not live HP — live HP belongs to the #22 battle modal during a fight.
- **Generic entity detail dialog.** A NEW `<dialog>` mirroring the existing
  `#room-modal` seam (`showModal`/`close`/backdrop-click), reusable for any entity
  kind (NPC/item/fixture/boss later). Clicking a Nearby card opens it from data
  already in the snapshot — it sends NO command and mutates NO state (REQ-004). Its
  model is a pure view-model over the `NearbySnapshot` entry.
- **Attack reuses combat v1, not new modal logic.** The Attack button (rendered only
  when `attackable`) runs `runCommand("attack <name>")` (canonical, identical to
  typing); the existing `renderBattle` opens the #22 battle modal off the response
  snapshot (REQ-006). No new combat state/commands — #24 owns the loop.
- **Preserve existing behavior (REQ-007).** `look` / `talk` / `take`, movement, the
  Datastar feed, the combat modal, and the boss/oath `confront` path are unchanged;
  the Attack action + detail dialog are purely additive to Nearby.
- **Quality.** Engine branch logic (hostile/attackable) is deterministic ⇒ 100% MSI
  reachable; assert by value (not `is_some`); prefer expect-invariants over
  unreachable arms (PR-claude-expect-invariants-over-unreachable-arms-001). Pure
  view-model / thin glue-DOM split (Decision 032); the `<dialog>` open/close is
  browser-smoke (jsdom lacks it). **Validate runs `bin/gate.sh --fast`** (the owner
  reserves the full gate + commit for Codex).
- **One slice** (protocol + core + minimal content/fixture + JS) — not split.

## Linked Artifacts
- Design docs: `docs/ui-design.md` (Nearby combat affordances), `docs/entity-model.md`
  (#23 note + combatant stat-disclosure contract), `docs/spatial-awareness.md`
  (NearbySnapshot), `docs/combat-system.md` (context), `docs/decisions.md`
- Intake doc: none
- Ticket doc: `docs/planning/tickets/open/TICKET-23-nearby-hostile-affordances-and-entity-inspection.md`
- Forge ticket: `e8eaca33-1701-4009-93c6-e63007f700d7` (#23)
- AAR: `ca38d7cb-9b35-4674-b2df-11da22c455a4`

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
