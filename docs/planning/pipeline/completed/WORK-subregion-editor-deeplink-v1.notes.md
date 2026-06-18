# WORK-subregion-editor-deeplink-v1 — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** the final #51 slice (#51c) — link each sub-region row in the
  `/regions/{map_id}` editor into the tile editor for its map, with the editor
  highlighting that sub-region's rooms. Continuation of forge **#51** (not new).
- **Intake source:** `INTAKE-region-model-rethink-and-owner-authoring.md` (program).
  Also `INTAKE-tileset-region-authoring-per-tile-metadata.md` — its "one `.tmx` per
  sub-region" sketch is the **multi-map architecture**, explicitly OUT of this slice.
- **Classification / tier:** work pipeline, one slice. Studio **render** (the link +
  the glue param) + the **pure `editor-canvas.js` draw model** (`editorDrawPlan`
  focus) + tests (Rust render + `node --test`). **No core/model change.**
- **Forge recall (pre-flight green; no bulletins):**
  - The editor `?map=<id>` reopen **already works** — the glue (`render.rs:327`) reads
    `URLSearchParams.get("map")` and fetches `/editor/maps/<id>`. So `/editor?map=<map_id>`
    reopens the sub-region's map; the glue can also read `subregion` the same way.
  - `editorDrawPlan(doc, {z, tilePixels})` (`editor-canvas.js:75`) is the pure, node-tested
    draw model; it iterates `doc.rooms`, which carry `.subregion` — so a `focusSubregion`
    option can mark matching rooms as focused. Called from the glue at `render.rs:377`.
  - Sub-region rows currently have **no** editor link (S2 replaced slice-1's "Open in
    editor" with the edit/delete forms); #51c adds a scoped link back.
  - In today's model a sub-region belongs to **one** `MapDocument` (the `/regions/{map_id}`
    map) → its map is that `map_id`; no `SubregionDefinition` field needed. (The hypothesised
    serde-additive `map` field only matters under the multi-map architecture, which is OUT.)
- **Ticket:** forge **#51** `341c0863-3fdc-49cd-a438-18a4f5d827f2` (LINKED, not minted);
  local doc `docs/planning/tickets/open/TICKET-51-region-subregion-dashboard.md` (its scope:
  "a link from each sub-region to the tile-map editor scoped to that sub-region").
- **EARS reviewed:** REQ-001 (the link), REQ-002/003 (`editorDrawPlan` focus / no-focus),
  REQ-004 (glue reads `?subregion=` → focus), REQ-005 (gate).
- **Design crux (Phase 2):** the focus representation in the draw plan (a `focused` flag on
  the room op vs a separate highlight op) + the visual (outline/tint); whether region rows
  also get a (focus-less) editor link. The glue stays the smoke-verified seam; all testable
  focus logic in the pure model.
- **aar_id:** `d8eabcc3-ab72-4c76-8bf6-be9c843efb76`
- **Branch / WIP:** branch off `main` (S1+S2+S3 merged there, tip `adcdf18`); online-first
  WIP stays stashed (`stash@{0}`) — do NOT sweep.

## Phase 2 — Design

### Code reconnaissance (working tree)
- `editorDrawPlan(doc, { z = 0, tilePixels })` (`editor-canvas.js:75`) emits one op per
  cell `{ x, y, size, kind, fill, stroke, textColor, glyph, sprites }`, with
  `room = roomAt(doc, x, y, z)`. Rooms carry `.subregion`. Existing tests use
  field-by-field asserts + a self-determinism `deepEqual` (no full-op literal) →
  **an always-present `focused` boolean won't break them**.
- The glue `redraw()` (`render.rs:375`) loops `editorDrawPlan(doc, { z: Z, tilePixels:
  TILE }).ops` and fills/blits/glyphs each — the place to add a focused-cell stroke.
- The glue already reads `new URLSearchParams(window.location.search).get("map")`
  (`render.rs:327`) → it can read `"subregion"` the same way. `map_id` is slot-safe
  (URL-safe); a sub-region id is free author text → percent-encode it for the query.

### Approach / architecture
Pure-model focus + a thin render link; **no core/model change** (a sub-region's map is
the `map_id` being edited).
1. **`editor-canvas.js`** — `editorDrawPlan(doc, { z = 0, tilePixels, focusSubregion =
   null })`. Each op gains `focused: room != null && focusSubregion != null &&
   room.subregion === focusSubregion`. Pure + node-tested; default `null` ⇒ all
   `focused:false` (no behavior change for existing callers/tests).
2. **`render.rs` — the link.** Each sub-region row in `region_editor_page` gets an
   "Open in editor" `<a>` to `/editor?map=<map_id>&subregion=<sub_id>`. `map_id` is
   slot-safe; the sub-region id is **percent-encoded** for the query via a small
   `url_query_encode` (encode any char outside `A-Za-z0-9_.-~`), then the whole href is
   `escape_html`'d for the attribute. (map_id stays escape-only — slot-safe.)
3. **`render.rs` — the glue.** Read `const focusSubregion = new URLSearchParams(
   window.location.search).get("subregion");` and pass it: `editorDrawPlan(doc, { z: Z,
   tilePixels: TILE, focusSubregion })`; in `redraw()`, after fill/glyph, stroke focused
   cells (`if (op.focused) { ctx.strokeStyle = <accent>; ctx.lineWidth = 3;
   ctx.strokeRect(op.x+1.5, op.y+1.5, op.size-3, op.size-3); }`). The glue stays the
   smoke-/review-verified seam (a Rust `const &str` — not cargo-mutants-mutated).

### Locked decisions (this phase)
- **`focused` boolean on the room op** (not a separate highlight op) — 1:1 with cells,
  node-testable, and additive (existing op-shape asserts are field-by-field).
- **Percent-encode the sub-region id** in the query (`url_query_encode`); `map_id` is
  slot-safe. Avoids a broken link for an id with a space/`&`.
- **Sub-regions only** get the link (the ticket's scope); a region-level editor link is a
  trivial future add, OUT here.
- **No model/core change**, **no new route** (reuse `?map=`); the `/regions` editor +
  `/editor` are already Editor-gated, so no new gated surface.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-studio/static/editor-canvas.js` | `editorDrawPlan` gains a `focusSubregion` option + a `focused` boolean per op (true for room cells whose `subregion` matches). |
| 2 | `crates/oathstar-studio/src/render.rs` | `region_editor_page`: sub-region rows gain an "Open in editor" `<a href="/editor?map=<map>&subregion=<enc(sub)>">`; new `url_query_encode` helper; `EDITOR_GLUE` reads `?subregion=` → passes `focusSubregion` to `editorDrawPlan` and strokes focused cells in `redraw()`. |
| 3 | `tests/studio-editor-canvas.test.js` | node: focus marks matching room ops `focused:true` (others false); no/unknown focus → none focused. |
| 4 | `crates/oathstar-studio/src/render.rs` (`#[cfg(test)]`) | the sub-region link element form (+ a space-bearing id → `%20`); `url_query_encode` unit; the editor page glue wires `?subregion=` → `editorDrawPlan` focus. |
| 5 | `docs/map-system.md` | (Phase 5) note the sub-region → editor deep link + focus highlight. |

### Regression Test Plan
| # | Test | Crate / kind | Proves |
|---|---|---|---|
| C1 | `region_editor_page` renders, per sub-region, `<a href="/editor?map=m&subregion=vale">Open in editor</a>`; a sub-region id `"a b"` encodes to `subregion=a%20b` | studio render unit | REQ-001 |
| C2 | `editorDrawPlan(doc, {…, focusSubregion:"vale"})` → room ops whose room is in `vale` have `focused:true`; rooms in `other` / with no subregion have `focused:false` | node --test | REQ-002 |
| C3 | `editorDrawPlan` with `focusSubregion` absent → every op `focused:false`; with an unknown id → none focused (and the non-focus plan otherwise matches today's shape) | node --test | REQ-003 |
| C4 | the `/editor` page glue contains `URLSearchParams(...).get("subregion")` and `editorDrawPlan(doc, { z: Z, tilePixels: TILE, focusSubregion })` + a focused-cell stroke | studio render unit | REQ-004 |
| C5 | `url_query_encode`: unreserved (`A-Za-z0-9_.-~`) pass through; space→`%20`, `&`→`%26`, non-ASCII→ UTF-8 %-bytes | studio render unit | REQ-001 (helper) |
| G1 | `bin/gate.sh` FULL green, mutation 100% MSI | gate | REQ-005 |
Render asserts test element forms (`PR-…-assert-element-form-…`). The JS focus logic is
node-covered (not cargo-mutants' Rust surface); the Rust mutation surface is the link
`write!` + `url_query_encode`, both covered by C1/C5. No genuinely uncoverable paths.

### Risks / decisions
1. **URL-encoding (D-encode)** — the sub-region id is author-free-text; `url_query_encode`
   makes the link correct for spaces/specials. Small helper, fully unit-tested (C5).
2. **`focused` flag additivity** — verified existing `editorDrawPlan` tests don't do a
   full-op `deepEqual`, so the new field is safe; determinism `deepEqual` stays equal.
3. **Glue is the non-mutated seam** — the focus *rendering* (stroke) lives in the Rust
   `const` glue (string, not mutated); all testable focus *logic* is in the pure JS model.
4. **Scope** — sub-regions only; no model change; multi-map architecture stays OUT.

## Phase 3 — Implement
- **Built to the manifest** (production code; new tests are Phase 4):
  - `static/editor-canvas.js` — `editorDrawPlan(doc, { z, tilePixels, focusSubregion = null })`;
    each op gains `focused: room !== null && focusSubregion != null && room.subregion ===
    focusSubregion` (always a clean boolean; default `null` ⇒ all false). JSDoc updated.
  - `src/render.rs` — new `url_query_encode` (RFC-3986 unreserved pass-through, else `%XX`
    over UTF-8 bytes, via the existing `core::fmt::Write`); each sub-region row now renders
    `<a class="cta" href="/editor?map={map_id}&subregion={senc}">Open in editor</a>`
    (`senc = url_query_encode(&sub.id)`); `EDITOR_GLUE` reads `?subregion=` into
    `focusSubregion`, passes it to `editorDrawPlan`, and strokes focused cells (gold inset
    `strokeRect`) in `redraw()`.
- **No core/model change** (a sub-region's map is the `map_id` it's edited under).
- **Verified:** `cargo fmt`; `cargo clippy -p oathstar-studio --all-targets` clean under the
  strict lints; `node --test tests/studio-editor-canvas.test.js` parses + the 15 existing
  tests pass (the `focused` field is additive — no full-op `deepEqual`, determinism holds).
- **Deviations from design:** none. (The S2 panel render test and the editor-page glue test
  still pass: the sub-region link is additive to the `<li>`, and they assert `editorDrawPlan(`
  / the `<li>` prefix, both unchanged.)
- **For Phase 4:** C1 link + `%20` encode, C2/C3 node focus/no-focus, C4 glue wiring (read +
  pass + stroke), C5 `url_query_encode` unit. The Rust mutation surface is the link `write!`
  + `url_query_encode` (both covered); the JS focus logic is node-covered (not the Rust
  cargo-mutants surface); the glue stroke is the non-mutated `const` seam.

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel **read-only `Explore`** critics — enforcing
  `PR-claude-inspect-critic-read-only-001`, with an explicit no-git-mutation instruction;
  each verified concretely incl. `cargo`/`node`): **correctness + integrity**,
  **security/href + simplification**.
- **Findings: none.** "No findings; lenses covered: url_query_encode correctness +
  href-injection safety, the `focused` boolean edge cases, the link href, the glue wiring
  (`?map=` reopen + `?subregion=` focus + stroke), SAST/secrets, simplification, no
  existing-test break."
- **Cleared (with the critics' checks):**
  - **`url_query_encode`** — unreserved `A-Za-z0-9-._~` pass through; every other byte →
    `%XX` uppercase; non-ASCII as UTF-8 bytes (`é`→`%C3%A9`); empty→empty. **Href-safe**: a
    `"><script>` id → `%3E%3Cscript%3E` — no attribute-breaking char escapes, so the
    unescaped `senc` is safe in the double-quoted `href` (and `map_id` is the slot-validated,
    URL-safe value).
  - **`focused`** — true only for a room whose `subregion` matches a non-null focus; non-room
    cells, a room without `subregion`, a null/empty/unknown focus all → `false` (always a
    clean boolean, never `undefined`).
  - **Glue** — `URLSearchParams.get("subregion")` decodes the `%`-encoding back to the raw
    id, so it matches `room.subregion`; the `?map=` reopen is unaffected by the extra
    `&subregion=`; the focused-cell stroke only fires on `op.focused`.
  - **No SAST token/secret**; `url_query_encode` reinvents nothing (no prior encoder);
    minimal (pre-sized, single pass, no clone). Non-defect note: `URLSearchParams` is read
    twice (map + subregion) — acceptable, left as-is.
  - render 27 + node 15 tests still pass; the S2 panel test + editor-page glue test are
    unbroken (link additive to the `<li>`; `editorDrawPlan(` unchanged).
- **Re-verified independently:** worktree still the same 2-file diff (no critic clobber —
  the read-only-critic rule held), `clippy --all-targets` clean, node 15 + render 15 green.
- **Capture:** no `failure-record` (no bug); no new rule (the existing read-only-critic rule
  was applied and worked).

## Phase 4 — Validate
- **Tests added (+5):**
  - **`render.rs`** (3) — C1 `region_editor_page` renders each sub-region's `<a class="cta"
    href="/editor?map=m&subregion=a%20b">Open in editor</a>` (a space-bearing id → `%20`);
    C5 `url_query_encode` unit (unreserved pass-through, `space/&/=` → `%XX`, `é`→`%C3%A9`,
    empty); C4 the `/editor` page glue wires `?subregion=` → `editorDrawPlan(…
    focusSubregion)` + the `if (op.focused)` stroke.
  - **`tests/studio-editor-canvas.test.js`** (2) — C2 `editorDrawPlan(…, focusSubregion)`
    flags only the focused sub-region's room cells (`focused:true`); a room in another
    sub-region, a room with no subregion, and a non-room cell stay `false`. C3 no/unknown
    focus → every cell `focused:false`.
- **`cargo test --workspace`:** GREEN — auth 20, content 113, core 300, datastar 16,
  protocol 27, server 35, storage 23, studio **80**; 0 failed.
- **`node --test tests/*.test.js`:** GREEN — **81 pass**, 0 fail.
- **`bin/gate.sh` (FULL):** **GATE GREEN — 17/17.** Mutation **592 caught / 0 missed →
  MSI 100.0%** (the 3 new `url_query_encode` mutants caught by C5; the JS focus logic is
  node-covered, not the Rust cargo-mutants surface; the glue is a non-mutated `const`).
  Rust + JS coverage floors held. Commit-gate receipt written.
- **Pre-existing exclusions:** none. The S2 panel render test + the editor-page glue test
  stayed green (the sub-region link is additive to the `<li>`; `editorDrawPlan(` unchanged).

## Phase 5 — Complete
- Docs updated:
- Forge capture (aar/failures/rules/decisions):
- Ticket closed:
- Archived:
