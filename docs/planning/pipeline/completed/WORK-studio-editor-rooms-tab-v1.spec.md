---
pipeline_id: f9f1f403-f91f-4f56-940c-94a9bf910561
title: WORK-studio-editor-rooms-tab-v1
ticket: 09c70ac8-3cd7-4a0c-ab49-1ef0d34e1fb9
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-studio-editor-rooms-tab-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-studio-editor-rooms-tab-v1

> Pipeline spec (always-loaded contract). Per-phase detail lives in the paired `.notes.md`.

## Work Spec
- **Title:** Fill the editor rail's **Rooms tab** with a per-room **metadata inspector** (edit
  `title` / `description` / `region` / `subregion` for existing rooms, list-select) + add a
  **map-title** field. Pivot ③ slice c1.
- **Today:** the #61 Rooms tab is a `Coming soon` stub. The editor edits an in-memory `MapDocument`
  (Save → `POST /editor/maps`). Unlike regions, **`MapDocument` has no room edit-CRUD** — rooms only
  exist in the JSON. And only the storage slot (`doc.id`, via `#map-name`) is editable, not the
  display title (`doc.title`).
- **Scope (in) — EDIT existing rooms only:**
  1. **`MapDocument::update_room(id, title, description, region, subregion, catalog)`** → a **PARTIAL
     update**: find the room by `id` (new `UnknownRoom` variant on `RegionEditError` + `Display`); set
     **only** `title`/`description`/`region`/`subregion`; **preserve** `x`/`y`/`z`/`glyph`/
     `combat_enabled`/`exits`/`entities`; `non_blank` the region; `clone → edit → finish(catalog)`.
     Mirrors `update_region`. (Partial-update structurally avoids the **#62 data-loss class**.)
  2. **`editor.rs room_op`** (`POST /editor/maps/room-op`, mirrors `region_op`): `editor_refusal`
     gate; parse `RoomOp { document, op, id, title, description, region, subregion }`; `"edit"` →
     `update_room`; `_` → 400; `Ok` → 200 the updated doc, `Err` → 400 `{message}`.
  3. **Route** `main.rs`.
  4. **Rooms tab** (replace the stub) + EDITOR_GLUE: pure **`editorRoomRows(doc)`** → `[{id, title,
     region, subregion}]` (title falls back to id); a room **list** (textContent); click a row → an
     **inspector** (title + description inputs + region `<select>` from `doc.regions` + subregion
     `<select>` from `doc.subregions`, populated from the room); inspector Save POSTs `room_op "edit"`,
     swaps the returned doc, re-renders + `redraw()`.
  5. **Map-title:** `<input id="map-title">` next to `#map-name` → `doc.title` (the Save posts the
     full doc).
- **Scope (out):** create/delete rooms (③c3); canvas-click select (③c2); editing exits/glyph/combat/
  entities; sub-region editing (③b2); palette curation.
- **Systems:** `oathstar-content` (`update_room` + `UnknownRoom`), `oathstar-studio` (`room_op`,
  route, Rooms-tab markup + glue, `editorRoomRows`, `#map-title`) + tests.

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | `update_room` shall set `title`/`description`/`region`/`subregion` on the named room and **preserve** its `x`/`y`/`z`/`glyph`/`combat_enabled`/`exits`/`entities`. | cargo test (round-trip a room with a glyph + exit) |
| REQ-002 | `update_room` for an unknown room id shall return `UnknownRoom` and change nothing. | cargo test |
| REQ-003 | `update_room` with a blank or undeclared region shall return an error (no partial mutation persists). | cargo test |
| REQ-004 | `room_op` `"edit"` shall return `200` with the updated document; an unknown op or a non-editor/anon caller shall return `400`/`403`/`401`. | cargo test |
| REQ-005 | `editorRoomRows(doc)` shall return one row per room (`title` falling back to `id`, with `region`/`subregion`); an empty doc → `[]`. | node --test |
| REQ-006 | The editor Rooms tabpanel shall render the room list + inspector (not a "Coming soon" stub), and the rail shall include `#map-title`. | cargo test (render assert) |
| REQ-007 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **`update_room` is a PARTIAL update** (sets only the 4 metadata fields, preserves the rest) — the
  structural fix for the **#62 full-overwrite data-loss class** (`BF-region-rename-wipes-description-001`
  / `PR-claude-roundtrip-edit-echo-unchanged-fields-001`). The inspector sends only the 4 fields.
- **`UnknownRoom` added to `RegionEditError`** (the map-edit error enum); `finish(catalog)` catches an
  undeclared region/subregion via the materialize boundary (designer may add explicit
  `UnknownRegion`/`UnknownSubregion` pre-checks for a clearer 400, reusing the existing variants).
- **`room_op` is dispatch-only** (no domain logic), mirroring `region_op`. The list + inspector are
  **client-rendered with `textContent`** (XSS-safe). The map-title sets `doc.title` (Save persists).
- **EDIT only** this slice — creating/deleting rooms + canvas-click + exits/glyph are later slices.
- **Branch off `main`** (`115f8c5`); **autonomous through commit + push + FF-merge**; stash parked.

## Linked Artifacts
- Design docs: `docs/map-system.md` (the editor/rooms section). Design re-reads.
- Plan: memory `studio-editable-world-pivot` (item ③ slice c1). Builds on #62 (region_op/editorRegionRows
  pattern), `map_edit.rs` (update_region/finish/RegionEditError).
- Ticket doc: `docs/planning/tickets/open/TICKET-63-studio-editor-rooms-tab.md`
- Forge ticket: `09c70ac8-3cd7-4a0c-ab49-1ef0d34e1fb9` (#63).

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
