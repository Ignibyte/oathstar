---
pipeline_id: 428b6b69-b203-4c3b-9efe-214781c97fd0
title: WORK-editor-marquee-paint-v1
ticket: 29bd6245-960c-4b60-9771-3044dcbf32f6
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-editor-marquee-paint-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-editor-marquee-paint-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`.

## Work Spec
- **Title:** Marquee (rectangle) multi-tile paint in the studio tile editor — item ④
  ("grab multiple tiles at once").
- **Today:** the glue's `paintAt` (`render.rs:474`) paints one cell on `mousedown` (483) and
  `mousemove`-while-down (484, freehand), via `canvasPointToCell` + the immutable `paintCell`.
- **Scope:**
  - **In:**
    1. **Pure model** (`editor-canvas.js`): `cellsInRect(a, b)` — the inclusive, **normalized**
       (drag any direction) rectangle of `{x,y}` cells in deterministic row-major order; and
       `paintRect(doc, layerId, a, b, z, tileRef)` — folds the existing immutable `paintCell`
       over those cells, returning a new doc. Node-tested.
    2. **Glue** (`EDITOR_GLUE`): **replace the freehand drag with a marquee** — `mousedown`
       records the start cell, `mouseup` fills the rectangle `start→end` via `paintRect` on the
       active `"ground"` layer + `Z`. A **single click is a 1×1 rect** → single-cell paint
       preserved. (A live rectangle **preview** during drag is **optional** — design decides;
       it stays in the glue/smoke-verified seam.)
  - **Out (explicit):** a brush / freehand-as-a-mode **toggle**; non-rectangular selection;
    copy/paste/move of a selection; undo/redo; multi-layer painting. Item ③ (regions table) is
    separate. No backend/protocol/Rust-logic change.
- **Systems:** studio client only — `static/editor-canvas.js` (pure `cellsInRect` + `paintRect`)
  + `render.rs` `EDITOR_GLUE` (drag tracking) + tests (`node --test` + Rust render).

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | `cellsInRect(a, b)` shall return every cell of the inclusive rectangle spanned by `a` and `b` — normalized so any drag direction yields the same set — in deterministic row-major (`y` outer, `x` inner) order. | node --test (incl. `a > b`, a 1×1, a row, a column) |
| REQ-002 | `paintRect(doc, layerId, a, b, z, tileRef)` shall place `tileRef` in every cell of the rectangle on `layerId`/`z`, return a new document (input unchanged), and for `a == b` shall paint exactly that one cell. | node --test (all rect cells present; immutability; 1×1) |
| REQ-003 | The editor glue shall, on `mousedown`→`mouseup`, fill the dragged rectangle with the active tile via `paintRect`, and a single click (start == end) shall paint exactly one cell. | cargo test (the `/editor` glue wires `mousedown`/`mouseup` + `paintRect(` for the active `"ground"`/`Z`; the freehand `mousemove` paint is gone) |
| REQ-004 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **Replace freehand with marquee** (recommended): a single click = a 1×1 rectangle, so
  single-cell paint is subsumed; no mode toggle. Keeping freehand as a selectable mode is a
  possible follow-on (needs a toggle UI).
- **Rectangle math + multi-paint are pure** (`cellsInRect` + `paintRect`, node-tested,
  reusing the immutable `paintCell`); the **drag state + optional preview live in the glue**
  (smoke-/review-verified — a Rust `&str` const, not a cargo-mutants surface, so no new Rust
  mutants this slice).
- **Paint targets the active `"ground"` layer + `Z`** as today; determinism preserved
  (deterministic cell order; `paintRect` is a pure fold).
- **Live drag preview is OPTIONAL** — design decides whether to include a rectangle outline
  during drag; it is glue-only and doesn't affect the pure model / ACs.
- Render assertions test element/call forms (`PR-claude-assert-element-form-not-substring-001`).
- **Branch off `main`** (`cc9ffc8`); stash (`stash@{0}`) stays parked.
- **Design (Phase 2) decides:** `cellsInRect`/`paintRect` exact signatures, the glue drag-state
  shape (a `dragStart` cell var), and whether to ship the live preview now or defer.

## Linked Artifacts
- Design docs: `docs/map-system.md` (the editor + paint loop, ticket #48). Design re-reads.
- Intake / plan: `docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md`;
  memory `studio-authoring-next-phase` (item ④). Builds on #48 (the paint loop, `paintCell`).
- Ticket doc: `docs/planning/tickets/open/TICKET-57-editor-marquee-paint.md`
- Forge ticket: `29bd6245-960c-4b60-9771-3044dcbf32f6` (#57).

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
