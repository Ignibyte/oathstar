---
pipeline_id: d2aa7028-e303-4f57-9ef2-ab5b530184ac
title: WORK-studio-set-active-world-v1
ticket: 057bcd35-bdae-4996-a557-755ee6434844
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-studio-set-active-world-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-studio-set-active-world-v1

> Pipeline spec (always-loaded contract). Per-phase detail lives in the paired `.notes.md`.

## Work Spec
- **Title:** "**Set as active world**" — a studio action that promotes an authored map to the
  game's active world slot, with a validity guard. Pivot item ② slice 1 of the
  studio-editable-world program.
- **Today:** an authored map only becomes the game's world if the owner saves it under the **magic
  id `world`** (the game server's active-world resolver, #56, loads `maps_dir/world.json` =
  `ACTIVE_WORLD_FILE` at startup; `crates/oathstar-server/src/main.rs:381/397`). There's **no
  guard** — saving a broken doc as `world` would make the game silently fall back to baked beginner.
- **Scope (in):**
  1. **`editor.rs set_active(State(studio), jar, body: Bytes) -> Response`** (sibling of `validate`/
     `save_map`): `editor_refusal` gate (401/403); parse the `MapDocument` (400 on malformed via
     `refuse`); **`document.materialize(&studio.catalog)`** — on `Err` `refuse` **400
     `{ok:false,message,error}`** (validate's shape) and **write nothing**; on `Ok` write to the
     **fixed** active slot `studio.maps.write_json("world", &document)` (500 on write failure) →
     **200 `{ok:true, active:"world"}`**.
  2. **Route** `POST /editor/maps/activate` → `set_active` (`main.rs`).
  3. **EDITOR_GLUE** (`render.rs`): a **"Set as active world"** button (sibling of Save/Validate)
     that POSTs the current doc to `/editor/maps/activate` and renders `formatActivateResult`.
  4. **`formatActivateResult(resp)`** — NEW pure fn in `static/editor-canvas.js` (sibling of
     `formatSaveResult`), node-tested.
- **Scope (out):** converting the baked beginner world into an editable map (a later slice); world
  **hot-reload** (the server loads the slot at **startup** — a restart is still needed); the editor
  **tabbed UX** (pivot ③); an active-world **indicator** beyond the button feedback.
- **Systems:** `oathstar-studio` (`editor.rs` handler, `main.rs` route, `render.rs` glue/button,
  `static/editor-canvas.js` pure fn) + tests. **No game/engine change** (#56 already loads the slot).

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an Editor posts a **materializing** `MapDocument`, `set_active` shall write it to the active slot `world` and return `200 {ok:true, active:"world"}`. | cargo test (temp maps dir; `maps/world.json` exists + round-trips) |
| REQ-002 | When the posted document **fails to materialize**, `set_active` shall return `400 {ok:false,message,error}` and write **no** slot file. | cargo test |
| REQ-003 | When the body is **not a valid map document**, `set_active` shall return `400`. | cargo test |
| REQ-004 | Without an Editor session, `set_active` shall return `401` (no session) / `403` (non-editor) and write nothing. | cargo test |
| REQ-005 | `formatActivateResult` shall produce an "activated" result for `ok:true` and a "refused/invalid" result for `ok:false`. | node --test |
| REQ-006 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **The slot name `world` is a fixed literal** — the caller's document `id` is irrelevant to the
  slot; activation means "make *this* the game world." No request input reaches the slot name, so
  there is no traversal/arbitrary-slot-overwrite surface (`write_json` also validates the name +
  rejects symlinks).
- **Activation requires materialize.** A document that doesn't materialize cannot be promoted (it
  would otherwise become a `world.json` the game rejects → silent fallback to beginner). The 400
  carries the same typed error `validate` returns.
- **Reuse, no duplication** — `document.materialize`, `refuse`, `editor_refusal` are the existing
  seams; `set_active` composes them. `formatActivateResult` mirrors `formatSaveResult`
  (pure → node-tested; the handler/route/button are the thin browser seam).
- **Startup-load, not hot-reload** — the activated world takes effect on the next game-server start;
  documented in the button feedback/docs. (Rebuild+restart the game server on the #56 binary to
  exercise the loop end-to-end — the running one is the pre-#56 build.)
- **Branch off `main`** (`1157fee`); **autonomous through commit + push + FF-merge** (user
  re-granted 2026-06-19); `stash@{0}` stays parked.

## Linked Artifacts
- Design docs: `docs/map-system.md` (active-world note). Design re-reads.
- Plan: memory `studio-editable-world-pivot` (item ② slice 1). Builds on #55 (Save), #56
  (active-world resolver), #44 (validate/materialize).
- Ticket doc: `docs/planning/tickets/open/TICKET-60-studio-set-active-world.md`
- Forge ticket: `057bcd35-bdae-4996-a557-755ee6434844` (#60).

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
