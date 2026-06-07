---
pipeline_id: 2411a22d-1b44-4e0a-90a1-370a6c6e3ed1
title: WORK-fix-play-surface-control-placement-and-viewport
ticket: a4ef40f6-cb5d-4a01-a1e6-2bf2cf488533
type: work
intake:
notes: WORK-fix-play-surface-control-placement-and-viewport.notes.md
status: Phase 5 — Complete PASS
---

# WORK-fix-play-surface-control-placement-and-viewport

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Fix the ticket #13 play-surface UX — relocate the directional Exit
  Pad near the command/feed area, drop duplicate movement commands from Intent,
  and constrain the shell to the viewport with a bounded internal-scroll feed.
  Client-only; the Rust server/snapshot/parser are unchanged.
- **Scope:**
  - **In:** move the Exit Pad out of the room/stage description area into the
    lower play surface near the event feed + command input (above/beside Enter);
    keep the room-brief focused on title/description/exits-text/`View room`;
    remove movement commands (`north`/`south`/`east`/`west`/`up`/`down`) from
    Intent suggestions; preserve manual typed movement through the command input;
    make the app shell fill the available viewport (no side gutters, no
    page-level scroll on desktop); make the event feed fixed/bounded with internal
    scrolling (newest at the bottom, older scrolls up); fix button clicks that
    cause viewport jumps; preserve ergonomic command focus; mobile: no horizontal
    overflow with all panels usable; update focused tests + browser smoke notes.
  - **Out:** changing server movement rules or the command parser; combat/shop/
    quest modals; generated room media; Tauri sidecar lifecycle; canvas/sprite map.
- **Systems:** ui.

## Acceptance Criteria (EARS)
Carried verbatim from forge ticket #14.

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

## Locked-In Decisions
- **Exit Pad relocates to the command/feed area (REQ-001/002).** The `#exit-pad`
  DOM node moves out of `.room-brief` into the feed panel next to the command form
  (above/beside the Enter button). `room.js` `toExitPad` is unchanged — only its
  mount point moves; `client-app.js` `renderExitPad` targets the relocated node.
  The room-brief keeps `#room-description`, `#exit-line`, and `#view-room-button`.
- **Movement leaves Intent (REQ-003/004).** `intent.js` `suggestCommands` no longer
  emits the six directional commands (drop `movementCommands` + `DIRECTION_HINTS`);
  the Exit Pad is the single movement control. Typed movement is untouched — the
  command input still POSTs raw text the server parses (no parser/server change).
- **Viewport-contained shell (REQ-005/006/008).** CSS only: `html`/`body`/the shell
  fill `100dvh`/`100vh` with no page-level scroll; the shell uses the full width
  (no fixed `max-width` gutters); each panel interior owns its overflow. The event
  feed (`#log`) gets a bounded height + `overflow-y:auto`, newest at the bottom
  (the existing append + `scrollTop = scrollHeight` already does this). Mobile:
  `min-width:0` on grid children + wrapping so there is no horizontal overflow.
- **No viewport jumps (REQ-007).** All interactive controls are `type="button"`
  (no implicit form submit); only the command form submits; the modal closes via
  `method="dialog"`. Command focus returns to the input after actions with
  scroll-preserving focus (`preventScroll`) so mobile/browser previews do not jump
  toward the command bar.
- **Pure/glue split holds (AD-claude-ui-shell-001).** The only pure-module change is
  `intent.js` (movement exclusion) — unit-tested. The DOM move + CSS are glue/style,
  verified by documented browser smoke (desktop + mobile). `room.js`/`snapshot.js`/
  the server are unchanged.

## Linked Artifacts
- Design docs: `docs/ui-design.md` (Layout Contract, Room And Exits, Intent Panel,
  Componentized Output — updated #14 guidance), `docs/protocol-and-output.md`
- Builds on: ticket #12 + #13 (`docs/planning/pipeline/completed/WORK-tauri-datastar-map-forward-ui-shell.spec.md`,
  `WORK-refine-play-surface-navigation-and-room-focus.spec.md`)
- Ticket doc: `docs/planning/tickets/open/TICKET-14-fix-play-surface-control-placement-and-viewport.md`
- Forge ticket: a4ef40f6-cb5d-4a01-a1e6-2bf2cf488533 (#14)
- AAR: 1e527e55-039e-4d82-91df-7b9f4b177903

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
