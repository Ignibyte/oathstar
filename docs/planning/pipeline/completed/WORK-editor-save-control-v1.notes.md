# WORK-editor-save-control-v1 — Notes

## Phase 1 — Plan
- **Request:** item ② — a Save control for the studio tile editor (`/editor`). Keystone of
  the owner's 2026-06-19 authoring-loop plan (memory `studio-authoring-next-phase`).
- **Classification / tier:** work pipeline, one slice, **studio client only** — `render.rs`
  (`editor_page` HTML + `EDITOR_GLUE`) + `editor-canvas.js` (pure `formatSaveResult`) + tests.
  No backend/protocol change.
- **Recon (working tree):**
  - `save_map` (`editor.rs:126`, `POST /editor/maps`): MapDocument body → `validate_save_slot_name(doc.id)`
    (400) → `maps.write_json(id, doc)` (500) → `200 {ok,id}`; drafts allowed. **Exists + tested.**
  - Editor controls (`render.rs:530-533`): only `<button id="validate">` + `<pre id="result">`.
    Glue (`487-503`): Validate → `fetch("/editor/maps/validate")` → `formatValidateResult` → `#result`.
  - `formatValidateResult` (`editor-canvas.js:203`) returns `{ok, headline, detail}` —
    `formatSaveResult` mirrors it (`{ok:true,id}`→"saved as <id>"; else server `message`).
  - The `?map=` reopen already exists (`render.rs:350` glue fetches `/editor/maps/<id>`); Save
    updates `?map=<id>` to round-trip.
- **No new Rust mutation surface:** `editor_page` (render fn → `Html<String>`, no viable mutant)
  + `EDITOR_GLUE` (`&str` const, not mutated). Testable logic = the pure `formatSaveResult` (node).
- **EARS:** REQ-001 controls render · REQ-002 Save→POST /editor/maps + formatSaveResult ·
  REQ-003 formatSaveResult mapping (node) · REQ-004 ?map= on success · REQ-005 gate.
- **Ticket:** forge **#55** `5cf4995b-0ae7-4ec0-941f-4b9c384c4e6f` (minted; NOT #54).
  Local doc `docs/planning/tickets/open/TICKET-55-editor-save-control.md`.
- **aar_id:** `1f58c2b9-7917-4ae7-b71b-e90bf2e6be5d`
- **Delivery:** goal-driven autonomous — plan→complete then commit + push + FF-merge to `main`
  (no pause). Branch off `main` `661fcc3`. Stash parked.

## Phase 2 — Design

### Code reconnaissance
- Glue: `doc` is finalized after the `?map=` reopen (`render.rs:340-357`); `result =
  getElementById("result")` at `486`; the Validate handler is `487-503` (the last block before
  the closing `"#`). So the Save handler + name-prefill append cleanly after `503`, with `doc`,
  `result`, and `formatSaveResult` (same module scope) all in scope.
- `formatValidateResult` returns `{ok, headline, detail}` (`editor-canvas.js:203`) — the shape
  `formatSaveResult` mirrors. Save response is `{ok:true,id}`; refusal is `{ok:false,message}`.
- The save POST must be assertable distinct from the reopen (`fetch("/editor/maps/" + …`) and
  validate (`fetch("/editor/maps/validate"`): the save is `fetch("/editor/maps", {` (POST opts).

### Approach / architecture
UI-only; mirror the Validate flow. No backend/protocol/Rust-logic change.
1. **`editor-canvas.js`** — pure `formatSaveResult(resp)`: `{ok:true,id}` → `{ok:true, headline:
   "Saved", detail: "as <id>"}`; any other shape → `{ok:false, headline:"Not saved", detail:
   resp.message || "save failed"}`. Node-tested sibling of `formatValidateResult`.
2. **`render.rs` `editor_page` controls** — add `<label>Map name <input id="map-name" …></label>`
   + `<button id="save" type="button">Save</button>` before the Validate button; refresh the hint
   to mention Save.
3. **`render.rs` `EDITOR_GLUE`** (after the Validate handler) — `const nameInput =
   getElementById("map-name"); nameInput.value = doc.id;` then a Save click handler: `doc.id =
   nameInput.value.trim()`, `fetch("/editor/maps", {method:"POST", …, body: JSON.stringify(doc)})`,
   `formatSaveResult(json)` → `#result` (+ `dataset.ok`), and on `out.ok` update the URL via
   `new URL(...)` + `searchParams.set("map", json.id)` + `history.replaceState` (reuse `json.id`,
   the server-confirmed slot).

### Locked decisions (this phase)
- **Glue-on-load prefill** (`nameInput.value = doc.id`) — `editor_page` doesn't parse the doc;
  the glue has it (incl. a `?map=`-reopened doc's id). Simpler than server-render.
- **`formatSaveResult` strings:** headline `Saved`/`Not saved`, detail `as <id>` / server message.
- **Empty/invalid name** isn't blocked client-side — POSTed as-is; the backend's
  `validate_save_slot_name` refusal (400) surfaces via `formatSaveResult`. (Single validator.)
- **`?map=` update keys off `json.id`** (server-confirmed), reusing the reopen path.
- No new Rust mutation surface (`editor_page` render fn + `EDITOR_GLUE` const); logic = the
  node-tested `formatSaveResult`.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-studio/static/editor-canvas.js` | Add pure `formatSaveResult(resp)`. |
| 2 | `crates/oathstar-studio/src/render.rs` | `editor_page` controls gain a `#map-name` input + `#save` button; `EDITOR_GLUE` prefills the name from `doc.id` and wires Save → `POST /editor/maps` → `formatSaveResult` → `#result` + `?map=` update. |
| 3 | `tests/studio-editor-canvas.test.js` | node: `formatSaveResult` ok + refusal + null. |
| 4 | `crates/oathstar-studio/src/render.rs` (`#[cfg(test)]`) | `editor_page` Save button + name input; glue wires `fetch("/editor/maps", {` + `formatSaveResult(` + the `?map=`/`replaceState` update. |
| 5 | `docs/map-system.md` | (Phase 5) note the editor Save control. |

### Regression Test Plan
| # | Test | Kind | Proves |
|---|---|---|---|
| T1 | `formatSaveResult({ok:true,id:"vale"})` → `{ok:true, headline:"Saved", detail:"as vale"}`; `({ok:false,message:"map id is not a valid storage name"})` → `{ok:false, detail: that message}`; `(null)` → `{ok:false, detail:"save failed"}` | node | REQ-003 |
| T2 | `editor_page` HTML contains `<button id="save"` and `<input id="map-name"` (+ Validate still present) | cargo (studio render) | REQ-001 |
| T3 | the editor glue contains `getElementById("save")`, `fetch("/editor/maps", {`, and `formatSaveResult(` | cargo | REQ-002 |
| T4 | the glue contains `searchParams.set("map"` + `history.replaceState` (the post-save reopen update) | cargo | REQ-004 |
| G1 | `bin/gate.sh` FULL green, MSI 100% | gate | REQ-005 |
- Coverage: `formatSaveResult` both branches via T1 (it's in the test-imported module). No Rust
  delta → no new mutants. **Genuinely uncoverable:** the `EDITOR_GLUE` runtime (browser-only;
  not node-imported) — the reviewed seam, pinned by the T2-T4 render-string assertions + smoke.

### Risks / decisions
1. **Glue is the untested seam** — all decision logic is the pure `formatSaveResult` (node);
   the wiring is render-string-asserted, as with Validate.
2. **`doc.id` mutation** on Save sets the working copy's slot — intended (subsequent saves/reopen
   use it). No state/view leak (the doc IS the editor's working state).
3. **No regression:** Validate untouched; the Save POST string is distinct from reopen/validate.

## Phase 3 — Implement
- **Built to the manifest** (tests are Phase 4):
  - `editor-canvas.js` — pure `formatSaveResult(resp)`: `{ok:true,id}` → `{ok:true,
    headline:"Saved", detail:"as <id>"}`; else `{ok:false, headline:"Not saved", detail:
    resp.message||"save failed"}`.
  - `render.rs` `editor_page` controls — `<label>Map name <input id="map-name" …></label>`
    + `<button id="save" type="button">Save</button>` before Validate; hint refreshed
    ("Save persists it; Validate checks it").
  - `render.rs` `EDITOR_GLUE` — after the Validate handler: prefill `nameInput.value = doc.id`;
    `#save` click → `doc.id = nameInput.value.trim()`, `fetch("/editor/maps", {POST, json})`,
    `formatSaveResult(json)` → `#result` + `dataset.ok`, and on `ok` `searchParams.set("map",
    json.id)` + `history.replaceState`.
- **No backend/protocol/Rust-logic change.**
- **Verified:** `cargo fmt`; `clippy -p oathstar-studio --all-targets` clean;
  `formatSaveResult({ok:true,id:"vale"})` → `Saved/as vale`, `({ok:false,message:"bad slot"})`
  → `Not saved/bad slot`; studio editor tests **36 pass**, node `studio-editor-canvas` **17
  pass** — no regression (Save is additive; Validate untouched).
- **Deviations from design:** none.
- **For Phase 4:** T1 node `formatSaveResult` (ok/refusal/null); T2–T4 rust `editor_page`
  (Save button + name input; glue `fetch("/editor/maps", {` + `formatSaveResult(` + the
  `searchParams.set("map"`/`replaceState` update).

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel **read-only `Explore`** critics, no worktree mutation —
  `PR-claude-inspect-critic-read-only-001`): **correctness + seam**, **security + simplification**.
- **Findings: none.** "No findings; lenses covered: formatSaveResult edge cases, glue
  ordering/scope, empty-name refusal, `?map=` server-confirmed id, XSS, SAST/secrets,
  overwrite-by-slot, no Validate/editor_page regression."
- **Cleared (critics' concrete checks):**
  - `formatSaveResult` always returns `{ok,headline,detail}` with non-empty detail; the
    `{ok:true}`-missing-id "as undefined" path is **unreachable** (the Rust `Saved.id: String`
    always carries the validated slot).
  - Glue ordering: `doc` is final (post `?map=` reopen) and `result`/`#map-name`/`#save` all
    exist when the appended save block runs.
  - Empty/whitespace name → `doc.id=""` → backend `validate_save_slot_name` 400 → JSON refusal
    → `formatSaveResult` shows the message; `res.json()` is inside the try/catch. Graceful.
  - `?map=` keys off `json.id` (server-confirmed) via `searchParams.set` (URL-encoded);
    `replaceState` doesn't reload.
  - **XSS-safe:** `result.textContent` (never `innerHTML`); no SAST token/secret; `formatSaveResult`
    mirrors `formatValidateResult` without drift; the small save/validate handler duplication is
    acceptable for the browser seam.
  - **Overwrite-by-slot** (`write_json` overwrites `maps/<id>.json`) is the **intended** save
    semantic for the single-user studio (the owner explicitly wants create-and-overwrite) — not a
    finding; the name input is user-controlled + visible.
  - No regression: Validate handler untouched; the existing `editor_page` test + all 80 studio
    tests pass.
- **Re-verified independently:** worktree = the 2 expected files (no clobber); clippy clean;
  studio 80 + node 17 green.
- **Capture:** no `failure-record` (no bug); no new rule.

## Phase 4 — Validate
- **Tests added (+2):**
  - T1 node (`studio-editor-canvas.test.js`) — `formatSaveResult`: `{ok:true,id:"vale"}` →
    `Saved / as vale`; `{ok:false,message:"map id is not a valid storage name"}` → that message;
    missing message / `null` / `undefined` → `save failed`; non-strict `ok` → failure.
  - T2–T4 rust (`render.rs` `editor_page_wires_the_save_control`) — the page has `<button
    id="save"` + `<input id="map-name"`; the glue wires `getElementById("save")`,
    `fetch("/editor/maps", {`, `formatSaveResult(`, `searchParams.set("map"`, `history.replaceState(`.
- **`node --test tests/*.test.js`:** GREEN — **84 pass**, 0 fail (+1).
- **`cargo test --workspace`:** GREEN — studio **81** (+1), all crates pass.
- **`bin/gate.sh` (FULL):** **GATE GREEN — 17/17, mutation MSI 100% (0 survivors)** (no Rust
  logic delta; the editor_page test pins the render string against a blank/drop-call mutant);
  **JS coverage 89.63% ≥ 75%**; rust coverage held. Commit-gate receipt written.
- **Pre-existing exclusions:** none. (Validate flow + the existing editor_page test unbroken.)

## Phase 5 — Complete
- **Docs:** `docs/map-system.md` — the editor now documents the **Save** control (name/slot
  input + button → `POST /editor/maps`, persist + `?map=<id>` reopen, #55) beside Validate,
  each via a pure `format{Save,Validate}Result`; the "later slices" note updated (save landed
  with #55; loading a saved map into the game is next; marquee paint later).
- **Forge:** `aar-submit` (AAR `1f58c2b9`, completed, score 5; reused
  `PR-claude-recon-before-build-slice-001` — the save backend already existed — and the
  read-only-critic rule); no `failure-record` (inspect clean); no new rule.
- **Ticket:** forge **#55 CLOSED (done)** — this pipeline fully delivers the Save control.
- **Archived:** `…/completed/WORK-editor-save-control-v1.{spec,notes}.md`.
