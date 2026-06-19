# TICKET-55 — Studio tile editor: a Save control (persist the authored map)

- **Forge ID:** `5cf4995b-0ae7-4ec0-941f-4b9c384c4e6f` (#55)
- **Type:** feature · **Status:** open (pipeline `WORK-editor-save-control-v1`)
- **Program:** item ② of the owner's 2026-06-19 authoring-loop plan (memory `studio-authoring-next-phase`)

## Why
The studio tile editor can **Validate** but not **Save** — so authored maps can't be kept.
This is the keystone of the studio save→play loop (item ① loads a saved map into the game).

## What
UI-only. The save backend (`save_map`, `POST /editor/maps`) already exists and is tested; it
is simply unwired. Add a map-name/slot input + a Save button beside Validate, and wire the glue
to set `doc.id`, POST the document, surface success/refusal in `#result` (a pure
`formatSaveResult`), and update `?map=<id>` on success.

## Acceptance
See `docs/planning/pipeline/active/WORK-editor-save-control-v1.spec.md` (EARS REQ-001..005).

## Out of scope (later items of the plan)
Item ① save→game world loading (saved map becomes the playable world; gitignore `maps/`);
item ④ marquee multi-tile paint; item ③ regions-page table/search/sort. Backend/protocol unchanged.
