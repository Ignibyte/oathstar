---
pipeline_id: 832cd212-5822-401c-af23-b0c5c3ccbcb9
title: WORK-studio-editor-regions-tab-v1
ticket: bedd26d3-a0e8-4998-837c-7001b5264dac
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-studio-editor-regions-tab-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-studio-editor-regions-tab-v1

> Pipeline spec (always-loaded contract). Per-phase detail lives in the paired `.notes.md`.

## Work Spec
- **Title:** Fill the editor rail's **Regions tab** with inline **region** create / rename / delete,
  routed through the in-memory doc via a round-trip endpoint over the **existing Rust CRUD**. Pivot
  ③ slice b. (**Regions only**; sub-regions are **slice ③b2**.)
- **Today:** the #61 Regions tab is a `Coming soon` stub. The editor edits an **in-memory**
  `MapDocument` (`#map-doc`, Save → `POST /editor/maps`); the region CRUD already lives on
  `MapDocument` (`map_edit.rs`): `create_region`/`update_region`/`delete_region(id,name,desc,catalog)`
  → `Result<Self, RegionEditError>` (a new validated doc; `delete_region` refuses while a room/
  sub-region references it; `RegionEditError: Display`).
- **Scope (in):**
  1. **`editor.rs region_op`** (`POST /editor/maps/region-op`): `editor_refusal` gate; parse
     `{ document: MapDocument, op: "create"|"edit"|"delete", id, name, description }`; **dispatch**
     `op` → the matching `MapDocument` method against `studio.catalog`; `Ok(doc)` → `200` the updated
     document JSON; `Err(e)` → `400 {ok:false, message: e.to_string()}` (reuse `refuse`/`Failure`).
     **No CRUD reimplementation.**
  2. **Route** `POST /editor/maps/region-op` → `region_op` (`main.rs`).
  3. **Regions tab** (replace the stub) + **EDITOR_GLUE**: an **add-region** form + a **list
     container**; the glue builds the rows **client-side** from the in-memory doc via
     `editorRegionRows`, wires add / per-row rename / delete to POST `doc+op` to the endpoint,
     **replaces the in-memory `doc`** with the returned document, and re-renders the list + the
     canvas; a `400` shows its `message`.
  4. **`editorRegionRows(doc)`** — NEW pure fn in `static/editor-canvas.js` →
     `[{ id, name, roomCount, subregionCount }]` from `doc.regions` + `doc.subregions` + `doc.rooms`.
- **Scope (out):** **sub-region** editing (③b2); the **Rooms** tab / room-metadata (③c); **map
  expansion** (③d); curating the palette; modal-styling polish.
- **Systems:** `oathstar-studio` (`editor.rs` handler, `main.rs` route, `render.rs` Regions-tab markup
  + glue, `editor-canvas.js` `editorRegionRows`) + tests. The CRUD is **reused** from
  `oathstar-content` — no engine change.

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | `region_op` `"create"` with a fresh id/name shall return `200` with the document JSON containing the new region. | cargo test |
| REQ-002 | `region_op` `"edit"` shall return `200` with the region's name/description updated. | cargo test |
| REQ-003 | `region_op` `"delete"` of an unreferenced region shall return `200` with it removed; deleting a region a room references shall return `400` (the typed message) with the document unchanged. | cargo test |
| REQ-004 | `region_op` with an unknown `op`, or a blank/duplicate/unknown id, shall return `400` and change nothing. | cargo test |
| REQ-005 | Without an Editor session, `region_op` shall return `401`/`403` and change nothing. | cargo test |
| REQ-006 | `editorRegionRows(doc)` shall return one row per region with `id`/`name`/`roomCount`/`subregionCount`; an empty doc → `[]`. | node --test |
| REQ-007 | The editor Regions tabpanel shall render the region UI (an add-region form + a list container wired to `editorRegionRows`), not the "Coming soon" stub. | cargo test (render assert) |
| REQ-008 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **Regions only this slice; sub-regions = ③b2** (the same endpoint pattern + a nested UI). Keeps the
  diff focused/reviewable.
- **Thin dispatch over the existing CRUD** — `region_op` adds no domain logic; it calls
  `create_region`/`update_region`/`delete_region` and surfaces `RegionEditError::to_string()` as the
  `400`. The in-memory doc stays **authoritative** (the endpoint returns the new doc; the client
  swaps it in; Save persists it — no server-side clobber).
- **The list is client-rendered** from the in-memory doc (it changes on every op), so the XSS guard is
  **`textContent` / safe DOM construction in the glue — never `innerHTML`** for author id/name
  (the pure `editorRegionRows` carries them raw; the glue escapes by using `textContent`). (Unlike the
  server-rendered #58 table, which used `escape_html`.)
- **`editorRegionRows` is the pure node+mutation seam** (mirrors `filterRows`/`tabPanelStates`).
- **Branch off `main`** (`7fc327c`); **autonomous through commit + push + FF-merge**; `stash@{0}` parked.

## Linked Artifacts
- Design docs: `docs/map-system.md` (the editor/regions section). Design re-reads.
- Plan: memory `studio-editable-world-pivot` (item ③ slice b). Builds on #61 (the tab shell),
  #51/#58 (the server-side regions editor + table), #44/#55 (validate/save), `map_edit.rs` (the CRUD).
- Ticket doc: `docs/planning/tickets/open/TICKET-62-studio-editor-regions-tab.md`
- Forge ticket: `bedd26d3-a0e8-4998-837c-7001b5264dac` (#62).

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
