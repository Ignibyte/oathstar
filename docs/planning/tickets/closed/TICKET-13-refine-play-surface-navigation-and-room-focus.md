---
title: TICKET-13-refine-play-surface-navigation-and-room-focus
status: done
ticket: 3ca08c0a-16a7-467e-bc03-3d683ac3fde7
ticket_number: 13
type: feature
created: 2026-06-07
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-refine-play-surface-navigation-and-room-focus.spec.md
---

# TICKET-13-refine-play-surface-navigation-and-room-focus

## Summary

Refine the first playable surface after ticket #12 by separating navigation from
room contents, adding a directional Exit Pad, and giving long room descriptions a
focused full-room view.

## Why

The ticket #12 shell is working, but the Nearby tab currently uses exits as a
placeholder. That makes Nearby redundant with the room exit controls and weakens
the mental model: Nearby should mean actors, items, fixtures, and other things
in the room. At the same time, rooms need room to become more immersive without
overwhelming the main map/feed layout. Long descriptions should be readable in
place, but expandable into a focused view that can eventually support images or
ambient animations.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the main play screen renders a room, the Nearby tab shall list only visible room contents such as NPCs, enemies, items, or fixtures, and shall not render exits as Nearby cards. | UI test / browser smoke |
| REQ-002 | When the current room has no visible contents in the snapshot, the Nearby tab shall render a quiet empty state rather than inventing placeholder exits or entities. | UI test |
| REQ-003 | When the current room has exits, the client shall render a compact directional Exit Pad with north, south, east, west, up, and down positions. | UI test / browser smoke |
| REQ-004 | When an exit exists, its Exit Pad control shall be enabled and shall send the same canonical movement command as the text prompt; unavailable directions shall be disabled or visually quiet. | UI test / browser smoke |
| REQ-005 | When the room description exceeds the main-surface display limit, the client shall show a polished truncated or summarized description with a clear action to open the full room view. | UI test / browser smoke |
| REQ-006 | When the player opens the full room view, the client shall show a focused modal/dialog containing the room title, full description, exits, and a reserved optional media area without changing game state. | UI test / browser smoke |
| REQ-007 | When the full room view is closed, the command prompt, event feed, map, and current room state shall remain intact. | UI test / browser smoke |
| REQ-008 | When implementing the room display model, the client shall keep room title, main description, full description, and optional future media data separated enough that later server fields can be adopted without rewriting the UI. | code review |
| REQ-009 | When the ticket is complete, `npm test`, `npm run build`, and `./bin/gate.sh --fast` shall pass. | command output |

## Scope

- In: remove exits from Nearby; render an honest empty Nearby state when room
  contents are absent; add a directional Exit Pad; wire Exit Pad buttons through
  the existing command path; add client-side room description truncation; add a
  full-room modal/dialog with title, full description, exits, and optional media
  placeholder; update focused JS tests and browser smoke notes.
- Out: adding real NPC/item/fixture fields to the Rust snapshot if they are not
  already available; generated room images; animated room scenes; production
  combat, shop, or quest modals; collapsible event groups; Tauri sidecar/server
  lifecycle.

## Notes

- Forge ticket: `3ca08c0a-16a7-467e-bc03-3d683ac3fde7` (#13)
- Related docs: `docs/ui-design.md`, `docs/map-system.md`,
  `docs/protocol-and-output.md`
- Builds on: ticket #12 / `docs/planning/tickets/open/TICKET-12-build-tauri-datastar-map-forward-ui-shell.md`
- Follow-up candidates:
  - Room contents snapshot: expose visible actors/items/fixtures from the Rust
    server so Nearby becomes fully data-driven.
  - Collapsible event groups: group combat, shops, dialogue, crafting, and oath
    sequences with summary cards.
  - Focused interaction modals: first-class combat, shop, and major dialogue
    modal flows.
