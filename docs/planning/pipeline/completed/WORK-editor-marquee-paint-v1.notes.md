# WORK-editor-marquee-paint-v1 — Notes

## Phase 1 — Plan
- **Request:** item ④ — marquee (rectangle) multi-tile paint in the studio tile editor.
  Memory `studio-authoring-next-phase` item ④.
- **Classification / tier:** work pipeline, one slice, **studio client only** —
  `editor-canvas.js` (pure `cellsInRect` + `paintRect`) + `render.rs` `EDITOR_GLUE` (drag
  tracking) + tests. No backend/protocol/Rust-logic change.
- **Recon (working tree):**
  - `paintCell(doc, layerId, {x,y,z}, {tileset,index})` (`editor-canvas.js:321`) — immutable
    single-cell paint (filters the old cell, pushes the new); `canvasPointToCell(px,py,tile,w,h)`
    (`:271`) → `{x,y}`|null. `paintRect` folds `paintCell` over `cellsInRect(a,b)`.
  - Glue `paintAt` (`render.rs:474`): `mousedown`=paintAt (483) + `mousemove`-while-down (484,
    freehand) → one cell each. This slice replaces that with marquee drag.
- **Interaction decision (surfaced):** REPLACE freehand with marquee (recommended) — a click =
  a 1×1 rect, so single-paint is subsumed; no mode toggle. Freehand-as-a-mode = follow-on.
- **EARS:** REQ-001 `cellsInRect` (normalized, row-major) · REQ-002 `paintRect` (fill +
  immutable + 1×1) · REQ-003 glue `mousedown`/`mouseup`→`paintRect` (single click = one cell) ·
  REQ-004 gate.
- **No new Rust mutation surface:** `EDITOR_GLUE` is a `&str` const + `editor_page` a render fn;
  the testable logic is the pure JS (`cellsInRect`/`paintRect`, node-covered).
- **Ticket:** forge **#57** `29bd6245-960c-4b60-9771-3044dcbf32f6` (NOT #55/#56).
  Local doc `docs/planning/tickets/open/TICKET-57-editor-marquee-paint.md`.
- **aar_id:** `09619627-ab0f-4cfa-b662-656775739e0d`
- **Delivery:** goal-driven autonomous — plan→complete then commit + push + FF-merge to `main`.
  Branch off `main` `cc9ffc8`. Stash parked.

## Phase 2 — Design

### Code reconnaissance
- The glue paint section (`render.rs:471-484`): `paintAt(e)` (guarded by `active`) →
  `canvasPointToCell` → `paintCell(doc,"ground",{x,y,z:Z},active)` + `redraw()`; wired to
  `mousedown` (483) and `mousemove`-while-down (484, freehand). This block is what marquee
  replaces.
- `paintCell` is **defined in the embedded `editor-canvas.js` source** (and `paintRect` will
  call it there), so the existing `editor_page` test's `html.contains("paintCell(")` stays
  true after the glue stops calling `paintCell` directly.

### Approach / architecture (studio client only)
1. **`editor-canvas.js` (pure):**
   - `cellsInRect(a, b)` → the inclusive rectangle's `{x,y}` cells, **normalized**
     (`Math.min`/`Math.max` on each axis) in **row-major** order (`y` outer, `x` inner).
   - `paintRect(doc, layerId, a, b, z, tileRef)` → `let next = doc; for (cell of cellsInRect(a,b))
     next = paintCell(next, layerId, {x,y,z}, tileRef); return next;` — a pure immutable fold.
2. **`render.rs` `EDITOR_GLUE`** — replace `paintAt` + the `mousedown`/`mousemove` wiring with
   marquee: a `cellAt(e)` helper + a `let dragStart = null;`; `mousedown` (guarded by `active`)
   sets `dragStart = cellAt(e)`; `mouseup` (guarded by `active` + `dragStart`) computes `end =
   cellAt(e)`, and on a valid `end` does `doc = paintRect(doc, "ground", dragStart, end, Z,
   active); redraw();` then clears `dragStart`. A **single click** → `start == end` → a 1×1 fill
   (single-cell paint preserved).

### Locked decisions (this phase)
- **Replace freehand with marquee** (a click = 1×1; no mode toggle).
- **Defer the live drag preview** — keep the slice to fill-on-release; a rectangle-outline
  preview is a glue-only follow-on (doesn't touch the pure model / ACs).
- **mouseup off-grid** (`end == null`) → no paint + clear `dragStart` (a drag released past the
  edge is a no-op; edge-clamping is a possible follow-on).
- `cellsInRect`/`paintRect` are **pure + node-tested**; the `dragStart` state lives in the glue
  (`&str` const seam — not a cargo-mutants surface, so **no new Rust mutants**).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-studio/static/editor-canvas.js` | Add pure `cellsInRect(a,b)` + `paintRect(doc, layerId, a, b, z, tileRef)` (folds `paintCell`). |
| 2 | `crates/oathstar-studio/src/render.rs` | `EDITOR_GLUE`: replace `paintAt` + the mousedown/mousemove freehand wiring with the `cellAt`/`dragStart` + `mousedown`/`mouseup`→`paintRect` marquee. |
| 3 | `tests/studio-editor-canvas.test.js` | node: `cellsInRect` (normalize / 1×1 / row / column / row-major order) + `paintRect` (all cells painted on layer/z; input unchanged; 1×1 == single). |
| 4 | `crates/oathstar-studio/src/render.rs` (`#[cfg(test)]`) | `editor_page` glue wires `addEventListener("mousedown"` + `addEventListener("mouseup"` + `paintRect(`. |
| 5 | `docs/map-system.md` | (Phase 5) update the paint-loop note (freehand → marquee). |

### Regression Test Plan
| # | Test | Proves |
|---|---|---|
| T1 | `cellsInRect({x:0,y:0},{x:2,y:1})` → 6 cells in row-major `[(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)]`; reversed corners → the **same** set (normalized); `({x:1,y:1},{x:1,y:1})` → `[{x:1,y:1}]`; a 1×N row and an N×1 column | REQ-001 (node) |
| T2 | `paintRect` over a 2×2 rect → the `"ground"` layer holds all four `{x,y,z,tileset,index}` cells; the **input doc is unchanged** (`deepEqual` a clone); a 1×1 rect paints exactly the single cell (matches `paintCell`) | REQ-002 (node) |
| T3 | the `/editor` page glue contains `addEventListener("mousedown"`, `addEventListener("mouseup"`, and `paintRect(` (marquee wired; the freehand per-move paint is gone) | REQ-003 (cargo) |
| G1 | `bin/gate.sh` FULL green, MSI 100% | REQ-004 |
- Coverage: `cellsInRect`/`paintRect` fully exercised by T1/T2 (test-imported). No Rust delta →
  no new mutants. **Uncoverable:** the `EDITOR_GLUE` runtime drag (browser-only; not node-imported)
  — the reviewed seam, pinned by T3's render-string assertions + smoke.

### Risks / decisions
1. **Behavior change** — replacing freehand means a *move* no longer paints mid-drag; paint
   happens on *release*. Intended (the owner asked for marquee). Single-cell paint preserved.
2. **off-grid release** → no-op (noted); edge-clamp is a follow-on.
3. **Glue seam** — all decision logic is the pure `cellsInRect`/`paintRect` (node); the drag
   wiring is render-string-asserted (T3) + smoke, as with the rest of the editor glue.

## Phase 3 — Implement
- **Built to the manifest** (tests Phase 4):
  - `editor-canvas.js` — pure `cellsInRect(a,b)` (Math.min/max normalize; row-major y-outer
    x-inner inclusive `{x,y}` cells) + `paintRect(doc, layerId, a, b, z, tileRef)` (folds the
    immutable `paintCell` over `cellsInRect`, returns a new doc).
  - `render.rs` `EDITOR_GLUE` — replaced `paintAt` + the `mousedown`/`mousemove` freehand wiring
    with the marquee: a `cellAt(e)` helper, `let dragStart = null`, `mousedown` (guard `active`)
    → `dragStart = cellAt(e)`, `mouseup` (guard `active && dragStart`) → `end = cellAt(e)`, on a
    valid `end` `doc = paintRect(doc,"ground",dragStart,end,Z,active); redraw()`, then clear
    `dragStart`. Single click → `start==end` → 1×1.
- **No backend/protocol/Rust-logic change.**
- **Verified:** `cargo fmt`; `clippy -p oathstar-studio --all-targets` clean;
  `cellsInRect({0,0},{2,1})` → row-major 6 cells, reversed corners → the **same** set;
  `paintRect` paints both cells (immutable — input untouched); studio `editor_page` tests +
  node `studio-editor-canvas` (18) pass — no regression (the existing `paintCell(` assertion
  still holds; the module defines `paintCell` + `paintRect` folds it).
- **Deviations from design:** none (live preview deferred as planned).
- **For Phase 4:** node `cellsInRect` (normalize/1×1/row/column/order) + `paintRect`
  (fill/immutable/1×1); rust `editor_page` glue (`mousedown`/`mouseup`/`paintRect(`).

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel **read-only `Explore`** critics — `PR-claude-inspect-critic-read-only-001`):
  **correctness + edges**, **simplification + seam**.
- **Findings:**
  | # | Sev | Finding | Verdict |
  |---|---|---|---|
  | 1 | low | mouseup's `dragStart = null` cleanup is skipped on the early-return path → stale `dragStart` (`render.rs` glue) | **Fixed (hygiene)** |
  | 2 | low | the `editor_page` smoke test asserts `paintCell(` but not `paintRect(` | **Deferred to Phase 4** (T3 asserts the glue's `paintRect(`/`mousedown`/`mouseup`) |
- **Finding 1 fix:** restructured `mouseup` to **clear `dragStart` first** (`const start =
  dragStart; dragStart = null;` then guard), so an early return can never leave a stale start.
  Effectively unreachable today (`active` never clears once a tile is picked, so a `dragStart`
  set under `active` can't survive into a `!active` mouseup), but it's robust against a future
  "deselect tile" feature — cheap to make correct.
- **Cleared (critics' concrete checks, run via `node`):**
  - `cellsInRect` — reversed corners → the **same** set (normalized); inclusive row-major order;
    `a==b` → one cell; 1×N row / N×1 column / negative coords all correct.
  - `paintRect` — **immutable** (input doc untouched; the fold rebinds `next`); all rect cells
    painted; last-write-wins on overlap; 1×1 == a single `paintCell`; replaces in-place on a
    layer with existing cells.
  - Glue drag edges — mousedown off-grid → `dragStart` null → no-op; mouseup off-grid → no paint
    + cleared; stray mouseup → no-op; single click → 1×1; `redraw()` present on success.
  - No dead code (`paintAt`/`mousemove` fully removed, no dangling refs); pure/seam split clean
    (rect math + fill pure; `dragStart` in the glue); `paintRect` reuses `paintCell` (no
    re-implementation); JSDoc consistent; no secrets/SAST.
  - Regression: `paintCell` still defined/exported (the existing `editor_page` `paintCell(`
    assertion holds); node 18 + editor_page tests green.
- **Re-verified after fix:** worktree = 2 expected files (no clobber); clippy clean; node 18 +
  editor_page tests pass.
- **Capture:** no `failure-record` (the fix was hygiene, not a reachable bug); no new rule.

## Phase 4 — Validate
- **Tests added (+3):**
  - T1 node `cellsInRect` (`studio-editor-canvas.test.js`) — inclusive row-major; reversed
    corners → same set; 1×1 → one cell; a row + a column.
  - T2 node `paintRect` — a 2×2 rect fills all four `{x,y,z,tileset,index}` cells; the input
    doc is unmutated (JSON snapshot); a 1×1 equals a single `paintCell`.
  - T3 rust `editor_page_wires_the_marquee_paint` — the glue has `addEventListener("mousedown"`
    + `addEventListener("mouseup"` + `paintRect(`, and the freehand `e.buttons === 1` paint is gone.
- **`node --test tests/*.test.js`:** GREEN — **86 pass** (+2).
- **`cargo test --workspace`:** GREEN — studio **82** (+1), all crates pass.
- **`bin/gate.sh` (FULL):** **GATE GREEN — 17/17, mutation 590 caught / 0 missed → MSI 100.0%**
  (no new Rust mutants — JS-only logic + a `const` glue; the marquee logic is node-covered).
  JS coverage 89.81%; rust coverage held. Commit-gate receipt written.
- **Pre-existing exclusions:** none.

## Phase 5 — Complete
- **Docs:** `docs/map-system.md` "Paintable (#48)" — now describes **marquee** paint (drag a
  rectangle to fill it; single click = 1×1; `cellsInRect`/`paintRect`), replacing the freehand
  drag; the "later slices" list updated (marquee landed with #57).
- **Forge:** `aar-submit` (AAR `09619627`, completed, score 5; reused the read-only-critic rule);
  no `failure-record` (the inspect finding was a hygiene fix, not a reachable bug); no new rule.
- **Ticket:** forge **#57 CLOSED (done)**.
- **Archived:** `…/completed/WORK-editor-marquee-paint-v1.{spec,notes}.md`.
