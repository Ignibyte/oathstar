---
pipeline_id: eb36fb74-a5ae-4b37-92e2-84b1db7c3cbe
title: WORK-refine-play-surface-navigation-and-room-focus
ticket: 3ca08c0a-16a7-467e-bc03-3d683ac3fde7
type: work
intake:
notes: WORK-refine-play-surface-navigation-and-room-focus.notes.md
status: Phase 5 — Complete PASS
---

# WORK-refine-play-surface-navigation-and-room-focus

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Refine the ticket #12 play surface — separate navigation from Nearby,
  add a directional Exit Pad, and give long room descriptions a focused full-room
  view. Client-only; the Rust server/snapshot is unchanged.
- **Scope:**
  - **In:** stop rendering exits in the Nearby tab; Nearby shows only visible room
    contents (actors/items/fixtures) with an honest empty state when none are in the
    snapshot; a compact N/S/E/W/U/D Exit Pad whose enabled buttons send the same
    canonical movement commands as the text prompt (unavailable directions quiet);
    client-side room description truncation with a "View room" action; a focused
    full-room modal (native `<dialog>`) showing title, full description, exits, and a
    reserved optional media area, whose close preserves prompt/feed/map/state; a
    client room display model (title, main description, full description, media hint)
    ready for future server fields; updated JS tests + browser smoke notes.
  - **Out:** adding real NPC/item/fixture fields to the Rust snapshot (none exist —
    Nearby stays empty honestly); generated/animated room images; production
    combat/shop/quest modals; collapsible event groups; Tauri sidecar/server
    lifecycle.
- **Systems:** ui.

## Acceptance Criteria (EARS)
Carried verbatim from forge ticket #13.

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

## Locked-In Decisions
- **New pure module `src/client/room.js`** holds `toRoomDisplay(snapshot, opts)`
  (title, mainDescription, fullDescription, `truncated` flag, mediaHint, region,
  subregion, exits) and `toExitPad(snapshot)` (the six canonical directions, each
  `{ dir, label, command, available, destinationId }`). DOM-free + tested — extends
  the pure-core/glue split of AD-claude-ui-shell-001.
- **No new server fields (REQ-008 stays client-side).** The single server
  `RoomSnapshot.description` is the source: `mainDescription` is it truncated at a
  word boundary near a polished limit; `fullDescription` is the whole text;
  `mediaHint` is `room.mediaHint ?? null` (a reserved, currently-absent field). When
  the server later splits short/long/media, the model reads the new fields without a
  UI rewrite.
- **Nearby = contents only (REQ-001/002).** `snapshot.toNearby` no longer derives
  from exits; it reads room contents (`actors`/`items`/`fixtures`/`contents` if
  present) and returns an honest empty state today (the snapshot exposes none).
- **Exit Pad sends canonical commands (REQ-003/004).** Enabled direction buttons
  call the existing `runCommand(dir)` path — identical to typing the direction.
  Unavailable directions render disabled + visually quiet. A readable
  `Exits: e, n, u` line stays for MUD readability.
- **Full-room view = native `<dialog>` (REQ-006/007).** Framework-free, accessible,
  Esc/backdrop-closable; it is an overlay, so closing changes no game state (state
  lives in `latestSnapshot` + the feed/map DOM). No SPA framework introduced.
- **Client-only change:** `crates/oathstar-*` and the protocol are untouched; gate's
  Rust side is unaffected. `npm test` (now `tests/*.test.js`) + `npm run build`
  (vite) + `./bin/gate.sh --fast` are the bar.

## Linked Artifacts
- Design docs: `docs/ui-design.md` ("Room And Exits", "Menu Tabs"),
  `docs/map-system.md`, `docs/protocol-and-output.md`, `docs/decisions.md` (Decision 032)
- Builds on: ticket #12 (`docs/planning/pipeline/completed/WORK-tauri-datastar-map-forward-ui-shell.spec.md`, AD-claude-ui-shell-001) — **uncommitted** working tree
- Ticket doc: `docs/planning/tickets/open/TICKET-13-refine-play-surface-navigation-and-room-focus.md`
- Forge ticket: 3ca08c0a-16a7-467e-bc03-3d683ac3fde7 (#13)
- AAR: 6718862d-8d60-44fe-b268-27ead04c1166

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
