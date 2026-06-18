---
pipeline_id: 3cad0cb0-5190-4b97-b2fb-51019367e367
title: WORK-subregion-editor-deeplink-v1
ticket: 341c0863-3fdc-49cd-a438-18a4f5d827f2
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-subregion-editor-deeplink-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-subregion-editor-deeplink-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Sub-region → tile-editor deep link with focus (ticket #51 **final slice,
  #51c**) — each sub-region row in the `/regions/{map_id}` editor links straight into the
  tile editor for that map, and the editor **highlights that sub-region's rooms** on the
  canvas. This is the last open piece of #51 (slice 1 read-only dashboard, slice 2 CRUD,
  slice 3 descriptions are shipped); it delivers the ticket's stated goal — "jump straight
  from a sub-region into its map."
- **Scope:**
  - **In:**
    1. **The link** — each sub-region row in `render::region_editor_page` gets an
       Editor-only "Open in editor" link to `/editor?map=<map_id>&subregion=<sub_id>`,
       reusing the existing `?map=` reopen (`editor-canvas` glue already fetches the saved
       map). `map_id` is the map the sub-region belongs to (the one being edited).
    2. **The focus** — the pure `editor-canvas.js` draw model (`editorDrawPlan`) gains a
       `focusSubregion` option: it marks the rooms whose `subregion` matches as focused so
       the canvas can highlight them (a clear visual cue of which rooms are in that
       sub-region). The browser glue reads the `subregion` query param and passes it in.
  - **Out (explicit):**
    - The **multi-map "one map per sub-region" world architecture** (the owner's
      `INTAKE-tileset-region-authoring` sketch) — a separate, much bigger decision; a
      sub-region here is part of the one `MapDocument` being edited.
    - A **new map-identity model field** on `SubregionDefinition` (not needed — see
      decisions); **room sub-region re-assignment** from the editor; **auto-scroll/zoom**
      to the sub-region (highlight only this slice).
    - Any **runtime/player** rendering change (the game canvas + `MapSnapshot` are untouched).
- **Systems:** studio (`render` — the link + the glue param) | editor client model
  (`static/editor-canvas.js` pure `editorDrawPlan` focus) | tests (Rust render + `node --test`).

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the per-map region editor renders a sub-region, it shall include a link to the tile editor for that sub-region's map, carrying the map slot and the sub-region id. | cargo test (the sub-region row contains the `<a href="/editor?map=<map>&subregion=<sub>">` element form) |
| REQ-002 | When `editorDrawPlan` is given a focus sub-region, it shall mark the rooms whose `subregion` matches it as focused and leave non-matching rooms unfocused. | node --test (focused flag/style on matching room ops only) |
| REQ-003 | When `editorDrawPlan` is given no focus sub-region (or one matching no rooms), it shall produce no focused rooms (render unchanged). | node --test (no focus → no room op focused) |
| REQ-004 | The editor page glue shall read the `subregion` query parameter and pass it to `editorDrawPlan` as the focus. | cargo test (the rendered `/editor` page wires the param read → `editorDrawPlan` focus, mirroring the existing `?map=` glue assertions) |
| REQ-005 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **#51 continuation, not a new ticket** — closes #51's last scope item ("a link from each
  sub-region to the tile-map editor scoped to that sub-region").
- **Reuse the current `map_id` — no new model field.** In today's model a sub-region is part
  of the one `MapDocument` being edited (`/regions/{map_id}`), so its map *is* that `map_id`;
  the link needs no `SubregionDefinition` change and the core stays untouched. (Design may
  revisit if it finds a reason, but the focused scope is map_id reuse.)
- **Focus logic lives in the pure `editorDrawPlan` model** (node-tested); the browser glue
  only reads `?subregion=` and passes it. The glue stays the smoke-/review-verified seam (a
  Rust string const — not mutation-mutated); all testable focus logic is in the pure model.
- **"Scoped to that sub-region" = highlight its rooms** (a visual cue); auto-scroll/zoom is OUT.
- **Reuse the existing `?map=` reopen** — no `editor.rs` route/handler change; the link + the
  client param do it. No new gated route (the `/regions` editor and `/editor` are already gated).
- Render assertions test element/name forms (`PR-claude-assert-element-form-not-substring-001`).
- **Branch off `main`** (everything is merged there). The stashed online-first WIP
  (`stash@{0}`) must not be swept in.
- **Design decides (Phase 2):** the exact focus representation in the draw plan (a `focused`
  flag on the room op vs a distinct highlight op), the highlight's visual (outline/tint), and
  whether region rows also get a (focus-less) editor link.

## Linked Artifacts
- Design docs: `docs/map-system.md` (the editor + regions authoring surface),
  `docs/region-standing.md` (n/a here). Design re-reads via Explore.
- Intake docs: `docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md`
  (program); `docs/planning/intake/INTAKE-tileset-region-authoring-per-tile-metadata.md`
  (the one-map-per-sub-region vision — explicitly the OUT/future architecture).
- Ticket doc: `docs/planning/tickets/open/TICKET-51-region-subregion-dashboard.md`
- Forge ticket: `341c0863-3fdc-49cd-a438-18a4f5d827f2` (#51, final slice)
- Builds on: S2 `WORK-region-subregion-authoring-v1`, S3 `WORK-region-subregion-description-authoring-v1`
  (both merged to `main`); `AD-claude-authored-map-region-crud-001`.

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
