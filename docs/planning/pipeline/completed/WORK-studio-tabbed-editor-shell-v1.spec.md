---
pipeline_id: dc74f391-8ad1-4257-be9d-222f50effc8a
title: WORK-studio-tabbed-editor-shell-v1
ticket: e87f802b-cd8e-40c8-b382-6c9be8d26388
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-studio-tabbed-editor-shell-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-studio-tabbed-editor-shell-v1

> Pipeline spec (always-loaded contract). Per-phase detail lives in the paired `.notes.md`.

## Work Spec
- **Title:** The **tabbed editor shell** — the editor controls move to a **right rail**, and the rail
  gains a **Tiles | Regions | Rooms | Map** tab bar (Tiles active; the rest stubbed). Pivot item ③
  slice a — the structural foundation for the editor-UX redesign the owner sketched.
- **Today:** `render.rs editor_page` is `editor-main` → `div.editor-left { canvas-panel ; controls
  (#map-name/#save/#validate/#activate/#result) }` + `palette-panel (#active-tile + #palette)`.
- **Scope (in) — LAYOUT ONLY, no behavior change:**
  1. Restructure into **LEFT** `section.panel.canvas-panel` (the stage) + a **RIGHT**
     `aside.editor-rail` with: (a) a **doc-actions** block = the controls **moved verbatim** (same
     ids `#map-name/#save/#validate/#activate/#result`), (b) a **tab bar** `div[role=tablist]` with
     four `button[role=tab]` Tiles|Regions|Rooms|Map (Tiles `aria-selected`), (c) four
     `section[role=tabpanel]` — **Tiles** holds the palette (`#active-tile` + `#palette`, moved
     verbatim, **visible**), **Regions/Rooms/Map** are `Coming soon` stub panels (`hidden`).
  2. **`tabPanelStates(tabIds, activeId)`** — NEW pure fn in `static/editor-canvas.js`: per-tab
     `{id, selected, hidden}` where the active tab is `activeId` when in `tabIds`, else `tabIds[0]`
     (unknown/blank → first). Node-tested seam (mirrors `filterRows`/`formatActivateResult`).
  3. **EDITOR_GLUE** — a `tablist` click handler reads the clicked tab id, computes
     `tabPanelStates`, applies `aria-selected`/class + `hidden` to each tab/panel (defensive: no-op
     when there's no tablist).
  4. **studio.css** — `.editor-rail` / `.tab-bar` + `[role=tab][aria-selected]` / `.tab-panel[hidden]`
     / `.doc-actions`, on the `:root` tokens; the `.editor-main` grid becomes stage-left + rail-right.
- **Scope (out):** filling the **Regions/Rooms/Map** tabs (later slices ③b+); the **room-metadata**
  inspector; **map expansion**; the **quick-edit modal**; **curating** the palette.
- **Systems:** `oathstar-studio` — `render.rs` (`editor_page` markup + EDITOR_GLUE), `studio.css`,
  `static/editor-canvas.js` (`tabPanelStates`) + tests. **No game/engine change.**

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The editor page shall render a right rail with a `role="tablist"` bar of four tabs (Tiles, Regions, Rooms, Map), Tiles `aria-selected="true"`. | cargo test (render assert) |
| REQ-002 | The document controls (`#map-name`, `#save`, `#validate`, `#activate`, `#result`) shall be present in the rail (ids unchanged). | cargo test |
| REQ-003 | The Tiles tabpanel shall contain `#palette` + `#active-tile` and be visible; the Regions/Rooms/Map panels shall be `hidden` "Coming soon" stubs. | cargo test |
| REQ-004 | `tabPanelStates(tabIds, activeId)` shall select+show the matching tab and hide the rest, falling back to the first tab when `activeId` is unknown/blank. | node --test |
| REQ-005 | The existing editor wiring (the `#save`/`#validate`/`#activate` control tests + the canvas/palette `#map`/`#palette` ids) shall keep passing. | cargo test (existing) |
| REQ-006 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **Layout only — every id is preserved** (`#map-name/#save/#validate/#activate/#result/#map/
  #palette/#active-tile`); the canvas/palette init and the `#save`/`#validate`/`#activate` glue are
  untouched, so behavior is identical and the wiring tests still match.
- **The palette stays visible** in the Tiles panel (not `hidden`) — its `<canvas>` init measures
  layout, which breaks if it starts display-none. Tiles is the default tab for this reason too.
- **Regions/Rooms/Map are stubs** this slice (`<p class="soon">Coming soon.</p>`, hidden).
- **`tabPanelStates` is the pure seam** (node + mutation tested); the tablist click handler is thin
  glue. `:root` tokens for all new CSS (consistent with #50/#58).
- **Branch off `main`** (`b5ca668`); **autonomous through commit + push + FF-merge**; `stash@{0}`
  parked.

## Linked Artifacts
- Design docs: `docs/map-system.md` (the editor section — stage + tabbed rail). Design re-reads.
- Plan: memory `studio-editable-world-pivot` (item ③ slice a). Builds on #45/#48 (editor + palette),
  #55/#60 (Save/Activate controls), #58 (the data-*/glue + pure-helper pattern).
- Ticket doc: `docs/planning/tickets/open/TICKET-61-studio-tabbed-editor-shell.md`
- Forge ticket: `e87f802b-cd8e-40c8-b382-6c9be8d26388` (#61).

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
