# WORK-studio-editor-canvas-room-select-v1 — Notes

## Phase 1 — Plan
- **Request:** clicking a room on the `#map` canvas opens #63's Rooms-tab inspector — the canvas half
  of "both list + canvas-click" (pivot ③ slice c2, memory `studio-editable-world-pivot`).
- **Classification / tier:** work pipeline, one small slice. `oathstar-studio` only (`editor-canvas.js`
  + `render.rs` EDITOR_GLUE + tests + `map-system.md`). No Rust domain change, no new endpoint.
- **Recon (main `feaff0a`):**
  - Paint loop (render.rs EDITOR_GLUE ~528-549): `canvas.mousedown` records the start cell,
    `canvas.mouseup` fills `paintRect(start→end)`. **Unconditional** (always paints).
  - `cellFromEvent(e)` (~533) = `canvasPointToCell(e.clientX-rect.left, e.clientY-rect.top, TILE,
    doc.width, doc.height)`. `canvasPointToCell(px,py,tilePixels,width,height)` (editor-canvas.js:357)
    → `{x,y}` or `null` (negative or out-of-bounds → null). `TILE=40`, `Z=0` (render.rs 417/419) —
    single-floor.
  - Active tab: `document.getElementById("panel-rooms").hidden` (false ⇒ Rooms active), set by the #61
    tab bar (`tabPanelStates` → `panel.hidden`).
  - #63's `selectRoom(id)` opens/populates the inspector. Reuse it.
- **Approach (design refines):** pure `editorRoomAt(doc,x,y,z)`; `roomsTabActive()`; gate the
  `mousedown`/`mouseup` paint with an early-return when Rooms is active; a `canvas` `click` handler that
  (Rooms-active) `cellFromEvent` → `editorRoomAt` → `selectRoom` on a hit.
- **Decisions:** the canvas tool is **tab-gated** (Rooms→select, Tiles→paint) — no new toggle, no touch
  to `paintRect`. Single-floor `Z=0`. Empty-cell click = no-op. Reuse `selectRoom` (no inspector dup).
- **EARS:** REQ-001 `editorRoomAt` matches x∧y∧z / null · REQ-002 Rooms-active canvas click selects the
  room (no-op on empty) · REQ-003 Tiles-tab paint still works (gated, not removed) · REQ-004 gate.
- **Mutation surface:** `editorRoomAt`'s `x===x && y===y && z===z` — each conjunct killed by a
  wrong-x / wrong-y / wrong-z node case. The canvas wiring is server-string glue (no viable Rust
  mutant; covered by the render assert).
- **Ticket:** forge **#64** `56d2dada-a7b0-46d1-8740-a61e6c481260`. Local doc
  `docs/planning/tickets/open/TICKET-64-studio-editor-canvas-room-select.md`.
- **aar_id:** `5dadc04c-a3b8-44a2-82ea-1c0b906ffd44`
- **Delivery:** AUTONOMOUS through commit+push+FF-merge. Branch off `main` `feaff0a`. Stash parked.
  (If the paint-vs-select gating turns out to need an explicit tool toggle the author should choose,
  surface it rather than guessing — but tab-gating is the sensible default.)

## Phase 2 — Design

### Code reconnaissance (the two open questions, resolved)
- **No import to extend** — the editor page is `<script type="module">{editor_js}{EDITOR_GLUE}</script>`
  (`render.rs:914`, `editor_js = include_str!("../static/editor-canvas.js")`). The whole module is
  **inlined**, so its `export function`s are already in the glue's scope — adding `editorRoomAt` as an
  export makes it callable from the glue with **no import statement**.
- **`selectRoom` is hoisted** — `function selectRoom(id)` (`render.rs:762`, a declaration), so the click
  handler can call it regardless of placement; it runs at click-time (after module init), so the
  inspector `const`s it closes over are defined. No ordering constraint.
- The pixel→cell helper is **`cellAt(e)`** (`:532`) = `canvasPointToCell(clientX-rect.left,
  clientY-rect.top, TILE, doc.width, doc.height)`. Paint handlers (`:537`/`:541`) gate on `active` (a
  selected tile); `mouseup` deliberately clears `dragStart` **first** (the "always clear the drag"
  invariant).

### Approach / architecture (studio-only; no Rust domain change)
- **`editor-canvas.js`** — NEW `export function editorRoomAt(doc, x, y, z)` → `((doc&&doc.rooms)||[])
  .find(r => r.x===x && r.y===y && r.z===z) ?? null` (pure, null-safe; `?? null` so a miss is `null`).
- **`render.rs` EDITOR_GLUE:**
  - `function roomsTabActive() { const panel = document.getElementById("panel-rooms"); return !!panel
    && !panel.hidden; }` (added near `cellAt`).
  - **Gate paint:** `mousedown` — `if (roomsTabActive()) { return; }` at the top (before `dragStart` is
    set). `mouseup` — `if (roomsTabActive()) { return; }` placed **after** `dragStart = null;` (so the
    clear-invariant holds even on a tab-switch-mid-drag). Paint is now Tiles-only.
  - **Click→select:** `canvas.addEventListener("click", (e) => { if (!roomsTabActive()) { return; }
    const cell = cellAt(e); if (!cell) { return; } const room = editorRoomAt(doc, cell.x, cell.y, Z);
    if (room) { selectRoom(room.id); } });` placed right after the `mouseup` handler.

### Locked decisions (this phase)
- **Tab-gated canvas tool** (Rooms→select, Tiles→paint) — the active tab picks the tool; no new toggle,
  `paintRect`/marquee math untouched. **`Z=0`** (single-floor). **Empty-cell click is a no-op**
  (keeps the current selection). **Reuse `selectRoom`** (no inspector duplication).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-studio/static/editor-canvas.js` | NEW pure `editorRoomAt(doc,x,y,z)`. |
| 2 | `crates/oathstar-studio/src/render.rs` | EDITOR_GLUE: `roomsTabActive()` + 2 paint guards + the `click`→`selectRoom` handler; render-assert test. |
| 3 | `tests/studio-editor-canvas.test.js` | `editorRoomAt` cases. |
| 4 | `docs/map-system.md` | (Phase 5) a room is reachable by clicking it on the canvas, not just the list. |

### Regression Test Plan
| # | Test | Proves |
|---|---|---|
| T1 | `editorRoomAt` — rooms `[{x:1,y:2,z:0,id:"a"},{x:3,y:0,z:0,id:"b"}]`: `(1,2,0)→a`, `(3,0,0)→b`, `(9,9,0)→null`, **`(1,2,1)→null` (wrong z)**, **`(1,0,0)→null` (y differs)**, **`(0,2,0)→null` (x differs)**; `{}`→`null`; `null`→`null` | REQ-001 (node) |
| T2 | render `editor_page_canvas_selects_rooms_on_the_rooms_tab` — the glue contains `editorRoomAt(`, `roomsTabActive(`, `addEventListener("click"`, `selectRoom(room.id)`, and `if (roomsTabActive()) { return; }` (the paint guard) | REQ-002 + REQ-003 (render assert) |
| T3 | the existing #48 paint (`mousedown`/`mouseup`/`paintRect`), #61 tab, #63 inspector tests stay green | REQ-003 regression |
| G1 | `bin/gate.sh` FULL green, MSI 100% | REQ-004 |
- **Mutation:** `editorRoomAt`'s `r.x===x && r.y===y && r.z===z` — the wrong-x / wrong-y / wrong-z node
  cases (T1) kill each `===` and the `&&`. The canvas wiring is server-string glue → no viable Rust
  mutant; covered by the T2 render assert. **Uncoverable:** none new.

### Risks / decisions
1. **`mouseup` guard ordering** — placed after `dragStart = null;` (not at the very top) so a
   tab-switch-mid-drag can't strand a stale `dragStart`. T3 (existing mouseup paint test) guards the
   non-Rooms path.
2. **`click` fires after `mousedown`+`mouseup`** — on the Tiles tab the click handler early-returns
   (`!roomsTabActive()`), so it never interferes with paint; on the Rooms tab the paint handlers
   early-return, so only the click's select runs. No double-handling.
3. **`active` (a selected tile) is still required to paint** — unchanged; the Rooms gate is additional.

## Phase 3 — Implement
- **Built (manifest as designed):**
  - `editor-canvas.js` — NEW pure `editorRoomAt(doc, x, y, z)` (`rooms.find(r => r.x===x && r.y===y &&
    r.z===z) ?? null`, null-safe) after `editorRoomRows`.
  - `render.rs` EDITOR_GLUE — `roomsTabActive()` (after `cellAt`); `if (roomsTabActive()) { return; }`
    at the top of `mousedown` and **after** `dragStart = null;` in `mouseup` (drag-clear invariant
    preserved); a new `canvas` `click` handler that (Rooms-active) `cellAt` → `editorRoomAt` →
    `selectRoom(room.id)` on a hit.
- **Deviations:** none. Reused `selectRoom` (no inspector duplication); `cellAt`/`canvasPointToCell`/
  `TILE`/`Z` reused; no import needed (editor-canvas.js is inlined into the module).
- **Checks:** `node --check editor-canvas.js` OK; `clippy -p oathstar-studio --all-targets` clean;
  `cargo fmt` clean; `cargo test -p oathstar-studio` → **101 passed** (the #48 marquee-paint, #61 tab,
  and #63 inspector tests all stay green — the paint gate didn't regress painting). New tests + gate at
  Phase 4.

## Inspect (Phase 3.5)
- **Lenses:** 2 read-only `Explore` critics — correctness/edge-cases + paint-regression-safety.
  **Both CLEAN.**
- **Critic 1 (correctness/edge-cases) — no findings.** `editorRoomAt`: null/`{}`/no-rooms → `null` (the
  `doc && doc.rooms` guard); a miss returns `null` not `undefined` (`?? null`); matches on **x AND y AND
  z** (not a subset). The click handler: non-Rooms tabs no-op (`!roomsTabActive()`); a null cell
  (out-of-bounds) is guarded (`if (!cell) return`); an empty cell is a no-op (keeps selection); a hit →
  `selectRoom(room.id)`. All ids in scope; `selectRoom` is a hoisted declaration (runs at click-time).
  node 24 + cargo 101 pass.
- **Critic 2 (paint-regression) — clean, no regression.** `mousedown` guard is the FIRST statement (no
  `dragStart` while Rooms active); `mouseup` guard is placed **after** `dragStart = null;` (the
  "always-clear-the-drag" invariant holds even on a tab-switch-mid-drag); Tiles-tab paint is
  byte-for-byte unchanged (`paintRect`/`cellAt`/`TILE`/`Z`/`active` untouched); **no double-handling** —
  exactly one of paint/select fires per gesture (Rooms: mousedown+mouseup early-return, click selects;
  Tiles: click early-returns, paint runs); out-of-bounds drag end still guarded by the unchanged
  `if (end)`; `roomsTabActive()` safe-fails to `false` if `#panel-rooms` is missing. #48/#57 paint +
  #61 tab tests pass.
- **No findings; no fix; no `failure-record`** (no defect — a clean, well-gated slice). Critic 2's
  "watch items" (tab-switch-mid-drag, empty-cell click, out-of-bounds click) are Phase-4 test angles —
  the empty-cell/out-of-bounds logic is covered by the `editorRoomAt`-miss-→-null node cases; the
  glue-level browser interaction is covered by the render-assert (the glue isn't `node`-imported).

## Phase 4 — Validate
- **Tests added (T1–T2, green):**
  - `tests/studio-editor-canvas.test.js` — **T1** `editorRoomAt: the room at a cell, matching x AND y
    AND z; null otherwise (#64)` — `(1,2,0)→a`, `(3,0,0)→b`, `(9,9,0)→null`, **`(1,2,1)→null` (wrong
    z)**, **`(1,0,0)→null` (y differs)**, **`(0,2,0)→null` (x differs)**, `{}`/`null` → `null`. (The
    three coordinate-mismatch cases pin each `===` conjunct.)
  - `render.rs` — **T2** `editor_page_canvas_selects_rooms_on_the_rooms_tab` — the glue contains
    `editorRoomAt(`, `roomsTabActive(`, `addEventListener("click"`, `selectRoom(room.id)`, and
    `if (roomsTabActive()) { return; }` (the paint guard).
- `cargo test --workspace`: **PASS** — oathstar-studio **102** (+1, the render assert).
- `node --test tests/*.test.js`: **PASS** — **93** tests, 0 fail (+1 `editorRoomAt`).
- `bin/gate.sh`: **GATE GREEN [full]** — all 17 gates; **mutation 600 caught / 0 missed → MSI 100.0%**
  (`editorRoomAt` is JS — its `x/y/z ===` are killed by the T1 node cases, not cargo-mutants; the canvas
  wiring is server-string glue with no viable Rust mutant, covered by T2). No pre-existing exclusions.

## Phase 5 — Complete
- Docs / forge / ticket / archived:
