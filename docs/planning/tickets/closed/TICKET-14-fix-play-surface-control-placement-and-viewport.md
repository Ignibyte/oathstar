---
title: TICKET-14-fix-play-surface-control-placement-and-viewport
status: done
ticket: a4ef40f6-cb5d-4a01-a1e6-2bf2cf488533
ticket_number: 14
type: feature
created: 2026-06-07
intake:
pipeline_spec: docs/planning/pipeline/completed/WORK-fix-play-surface-control-placement-and-viewport.spec.md
---

# TICKET-14-fix-play-surface-control-placement-and-viewport

## Summary

Correct the ticket #13 play-surface placement by moving movement controls near
the command/event-feed area, removing duplicate movement commands from Intent,
and constraining the shell to the browser/Tauri viewport with bounded panel
scrolling.

## Why

The Exit Pad is functionally correct, but its current location in the room
description area is too far from the command input and becomes cumbersome during
normal play. Movement should live near the player's command hand while the room
description stays focused on title, description, exits text, and the full-room
view action. The Intent panel should not also list movement commands once the
Exit Pad exists, because that makes the command helper noisy. The current page
can also grow beyond the visible app area; the game should feel like a contained
desktop shell with internal scrolling, not a web page that jumps around.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the room description area renders, it shall not contain the directional Exit Pad. | browser smoke / UI test |
| REQ-002 | When the player is viewing the main play screen, the directional Exit Pad shall be anchored near the event feed and command input, above or beside the Enter control, so movement is reachable from the command area. | browser smoke / screenshot review |
| REQ-003 | When Intent suggestions render while the Exit Pad is visible, the Intent panel shall exclude movement commands such as `north`, `south`, `east`, `west`, `up`, and `down`. | UI test |
| REQ-004 | When the player types a movement command manually, free-text movement shall still work through the command input. | API/browser smoke |
| REQ-005 | When the app renders on desktop-sized viewports, the game shell shall use the available viewport width and height without leaving unnecessary side gutters or forcing page-level scroll. | browser smoke |
| REQ-006 | When the event feed receives more entries than fit, the feed shall keep a fixed/bounded height, scroll internally, and keep the newest entries at the bottom. | browser smoke / UI test where practical |
| REQ-007 | When buttons in the Exit Pad, Intent panel, feed, or room modal are clicked, they shall not cause unexpected viewport jumps or page scrolling; command focus should remain ergonomic. | browser smoke |
| REQ-008 | When the layout is checked at a mobile viewport, the shell shall avoid horizontal overflow and keep map, feed, command input, Exit Pad, Intent, and menu usable. | browser smoke |
| REQ-009 | When the ticket is complete, `npm test`, `npm run build`, and `./bin/gate.sh --fast` shall pass. | command output |

## Scope

- In: move the Exit Pad out of the room/stage description area and into the
  lower play surface near the feed/command input; remove movement commands from
  Intent suggestions; preserve manual typed movement; make the app shell fill
  the available viewport; prevent page-level scroll on desktop; make the event
  feed internally scroll with newest events at the bottom; fix button click
  behavior that causes viewport jumps; update focused tests and browser smoke
  notes.
- Out: changing server movement rules; changing the command parser; adding
  combat/shop/quest modals; implementing generated room media; Tauri sidecar
  lifecycle; converting the map to canvas/sprites.

## Notes

- Forge ticket: `a4ef40f6-cb5d-4a01-a1e6-2bf2cf488533` (#14)
- Related docs: `docs/ui-design.md`, `docs/protocol-and-output.md`
- Builds on: ticket #12 and ticket #13.
- Keep the room description readable and immersive: title/description/exits
  text/full-room view stay in the stage area, but the directional controls move
  near the command/feed area.
