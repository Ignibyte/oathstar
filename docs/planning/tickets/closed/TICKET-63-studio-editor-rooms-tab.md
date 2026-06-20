# TICKET-63 — Editor Rooms tab: a room-metadata inspector + a map-title field

- **Forge ID:** `09c70ac8-3cd7-4a0c-ab49-1ef0d34e1fb9` (#63)
- **Type:** feature · **Status:** open (pipeline `WORK-studio-editor-rooms-tab-v1`)
- **Program:** pivot item ③ slice c1 of the studio-editable-world program (memory `studio-editable-world-pivot`)

## Why
The #61 Rooms tab is a "Coming soon" stub. Fill it with a per-room metadata inspector so the owner can
edit a room's title/description/region/subregion without leaving the editor. Also make the map's
display title (`doc.title`) editable — today only the storage slot (`doc.id`) is.

## What
A new **partial** `MapDocument::update_room` (sets the 4 metadata fields, **preserves** coords/glyph/
exits/entities — structurally avoiding the #62 full-overwrite data-loss class) + an `UnknownRoom`
error; a `room_op` endpoint (`POST /editor/maps/room-op`) mirroring `region_op`; the Rooms-tab room
list → inspector glue (`editorRoomRows`, `textContent`); and a `#map-title` field → `doc.title`.

## Acceptance
See `docs/planning/pipeline/active/WORK-studio-editor-rooms-tab-v1.spec.md` (EARS REQ-001..007).

## Out of scope
Creating/deleting rooms (slice ③c3); canvas-click selection (slice ③c2); editing exits/glyph/combat/
entities; sub-region editing (③b2); palette curation.
