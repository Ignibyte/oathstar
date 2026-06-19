---
pipeline_id: 732d5cf7-ba83-41f2-8155-46b4e3a34cca
title: WORK-editor-save-control-v1
ticket: 5cf4995b-0ae7-4ec0-941f-4b9c384c4e6f
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-editor-save-control-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-editor-save-control-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`.

## Work Spec
- **Title:** A Save control for the studio tile editor (`/editor`) — item ② / keystone of
  the owner's 2026-06-19 authoring-loop plan (the studio save→play loop).
- **The gap:** the editor can Validate but not Save. The controls wire only
  `<button id="validate">` → `POST /editor/maps/validate` (`render.rs:531`, glue `487-503`).
  The **save backend already exists + is tested** — `save_map` (`POST /editor/maps`,
  `editor.rs:126`): parses the `MapDocument`, enforces `validate_save_slot_name(doc.id)`
  (→400), `write_json` (→500), returns `200 {ok,id}`; drafts allowed. It is just **unwired**.
- **Scope:**
  - **In (UI only):**
    1. `editor_page` gains a **map-name/slot input** + a **Save button** beside Validate.
    2. `EDITOR_GLUE`: prefill the name input from `doc.id` on load; on **Save** set
       `doc.id` to the input value, `POST` the document to `/editor/maps`, render the result
       in `#result` via a new pure `formatSaveResult`, and on success update the URL to
       `?map=<id>` (the existing reopen path).
    3. A pure **`formatSaveResult(resp)`** in `editor-canvas.js` (sibling of
       `formatValidateResult`): `{ok:true,id}` → a "saved as <id>" success, any other shape
       → the server `message`.
  - **Out (explicit):** the save **backend** (exists); **item ① save→game world loading**
    (the saved map becomes the playable world; gitignore `maps/`) — next ticket; client-side
    re-implementation of `validate_save_slot_name` (the backend enforces; the UI surfaces the
    refusal); a full map-manager / new-map / autosave UI; **item ④** marquee paint; **item ③**
    regions table. No backend/protocol/engine change.
- **Systems:** studio client only — `render.rs` (`editor_page` HTML + `EDITOR_GLUE`) +
  `static/editor-canvas.js` (pure `formatSaveResult`) + tests (Rust render + `node --test`).

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The editor page shall render a Save button and a map-name input in the controls, alongside Validate. | cargo test (`editor_page` contains the `#save` button + the name `<input>` element forms) |
| REQ-002 | When Save is clicked, the editor glue shall set the document id from the name input, POST the document to `/editor/maps`, and render the outcome in `#result` via `formatSaveResult`. | cargo test (the glue wires `#save` → `fetch("/editor/maps"` (no `/validate`) + `formatSaveResult(`) |
| REQ-003 | `formatSaveResult` shall map `{ok:true,id}` to a success naming the saved id, and any other shape (refusal/parse/auth) to the server `message` (not a success). | node --test |
| REQ-004 | On a successful save the editor glue shall update the URL to `?map=<id>` so a reload reopens the saved map. | cargo test (the glue contains the `?map=`/`history.replaceState` update keyed off the saved id) |
| REQ-005 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **UI-only** — reuse the existing `save_map` endpoint; no backend/protocol change. (No new
  Rust *logic*: `editor_page` is a render fn, `EDITOR_GLUE` a `&str` const — neither is a
  cargo-mutants surface; the testable logic is the pure `formatSaveResult`, node-covered.)
- **The save slot = `doc.id`** — the name input sets it; the backend's `validate_save_slot_name`
  is the single validator and its refusal is surfaced verbatim (the UI does not re-validate).
- **Mirror the Validate flow** — same `#result` element + `formatXResult` shape; Save sits
  beside Validate.
- **`?map=<id>` on success** — reuse the existing reopen path so save → reload round-trips.
- **Render assertions test element/name forms** (`PR-claude-assert-element-form-not-substring-001`);
  the glue is the reviewed browser seam (smoke-verified, not a mutant).
- **Branch off `main`** (`661fcc3`); stash (`stash@{0}`) stays parked.
- **Design (Phase 2) decides:** the exact control markup/labels, the name-prefill mechanism
  (glue-on-load vs server-render), and `formatSaveResult`'s exact headline/detail strings.

## Linked Artifacts
- Design docs: `docs/map-system.md` (the editor), `docs/tileset-contract.md` (n/a). Design re-reads.
- Intake / plan: `docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md`;
  memory `studio-authoring-next-phase` (the 4-item plan; this is item ②).
- Ticket doc: `docs/planning/tickets/open/TICKET-55-editor-save-control.md`
- Forge ticket: `5cf4995b-0ae7-4ec0-941f-4b9c384c4e6f` (#55). Builds on `editor.rs` save_map (#44),
  the `?map=` reopen (#53/S1), and the region program (S1 #53, S2 #51, S3.1 #54 — all on `main`).

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
