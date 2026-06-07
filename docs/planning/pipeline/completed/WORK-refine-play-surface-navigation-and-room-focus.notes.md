# WORK-refine-play-surface-navigation-and-room-focus — Notes

Per-phase working notes for the paired `.spec.md`.

## Phase 1 — Plan
- **Request:** Ticket #13 — refine the #12 play surface: Nearby = contents only +
  honest empty state (no exits); N/S/E/W/U/D Exit Pad sending canonical movement
  commands; full-room modal for long descriptions; client room display model
  (title/main/full/mediaHint). Verify: `npm test`, `npm run build`,
  `./bin/gate.sh --fast` + browser smoke notes. Pipeline autonomously; no commit.
- **Intake source:** none (ticket minted directly; follow-up to #12).
- **Classification / tier:** Work pipeline, **client-only** (no Rust/protocol
  change). One shippable slice.
- **Forge recall (AAR 6718862d):**
  - `AD-claude-ui-shell-001` (#12) — framework-free pure-core/glue client; #13
    extends it (new pure `room.js` + glue in `client-app.js`; JS coverage floor 75%).
  - `BF-map-zplane-001` / `PR-claude-minimap-zplane-001` (#12 map bug) — navigation
    surface; keep exits/movement canonical + test with real beginner-world rooms.
  - Note: the docs index still returns the stale "Nearby: actors, items, **exits**"
    chunk; the live `ui-design.md` "Room And Exits" + ticket REQ-001 override it
    (Nearby drops exits once the Exit Pad exists).
- **Ticket:** forge #13 `3ca08c0a-16a7-467e-bc03-3d683ac3fde7` (already minted).
- **EARS reviewed:** REQ-001..009 carried verbatim. REQ-001/003/004/005/006/007 via
  pure-module tests + browser smoke; REQ-002 via test; REQ-008 via review;
  REQ-009 via `npm test` + `npm run build` + `gate --fast`.

### Grounding (working-tree facts — #12 is uncommitted)
- `RoomSnapshot` (crates/oathstar-protocol) has a single `description` + `exits`
  (BTreeMap dir→destId) + region/subregion; **no** actors/items/fixtures/mediaHint.
  So Nearby is honestly empty and main/full description both derive from `description`.
- Current client (#12, uncommitted): `src/client/snapshot.js` `toNearby` currently
  maps exits → nearby items (this is what #13 removes); `toRoomBrief` yields
  description + exits. `src/client-app.js` `renderRoom` renders `#room-description` +
  `#exits` buttons; `renderMenu` fills `#nearby`. `index.html` `.room-brief` holds
  `#room-description` + `#exits.exit-grid`. `styles.css` has `.room-brief`,
  `.room-description`, `.exit-grid`/`.exit-grid button`.
- `package.json` `test` is now `node --test tests/*.test.js` (runs client tests);
  `build` is `vite build`; gate:4 globs `tests/*.test.js`.
- Movement commands are the bare directions (`north`/`up`/...) — the same the
  prompt sends (verified via #12 live smoke: `north` accepted).

## Phase 2 — Design

### Approach / architecture (client-only; pure-core/glue split per AD-claude-ui-shell-001)
**New pure module `src/client/room.js`:**
- `DEFAULT_ROOM_DISPLAY_LIMIT = 160`.
- `toRoomDisplay(snapshot, opts = {})` → `{ id, title, region, subregion, main,
  full, truncated, mediaHint, exits }`:
  - `full = room.fullDescription ?? room.description ?? ""` (future server field, else
    the single `description`).
  - `main` / `truncated`: if `room.shortDescription` present → `main = shortDescription`,
    `truncated = main !== full`; else if `full.length > limit` → `main =
    summarize(full, limit)` (first sentence if one ends before `limit`, else word-cut)
    `+ "…"`, `truncated = true`; else `main = full`, `truncated = false`.
  - `mediaHint = room.mediaHint ?? null` (reserved `{ kind, src, alt }` later).
  - `exits = Object.entries(room.exits ?? {}).map([dir,dest] → { direction, destinationId, label, command:dir })`.
- `toExitPad(snapshot)` → `{ directions, availableCount }` where `directions` is the
  six canonical dirs in pad order `[up, north, west, east, south, down]`, each
  `{ dir, label, command: dir, available: <dir in room.exits>, destinationId }`.
- Keeps title / main / full / mediaHint separated so future server fields drop in
  without a UI rewrite (REQ-008).

**`src/client/snapshot.js`:** `toNearby` no longer derives from exits (REQ-001). It
reads room contents — `room.contents` (array), else assembled from
`room.actors`/`room.items`/`room.fixtures` if present — mapping to
`{ name, kind, command }`. The current snapshot exposes none → honest empty
`{ count: 0, items: [] }` (REQ-002). `toRoomBrief` is **removed** (its job moves to
`room.js`); its only callers (`renderRoom`, old `toNearby`, the T7 test) are updated.

**`src/client-app.js` (glue):**
- `renderRoom`: `display = toRoomDisplay(snapshot)`; set title/kicker; `#room-description`
  = `display.main`; `#exit-line` = `Exits: <dirs>` (or `—`); `#view-room-button`
  **always shown**, click → `openRoomModal(display)`; render the Exit Pad via
  `renderExitPad(toExitPad(snapshot))`.
- `renderExitPad(pad)`: build the six buttons in canonical order; available → enabled,
  `click → runCommand(dir)` (same canonical command as the prompt, REQ-004);
  unavailable → `disabled` + `.is-quiet` (REQ-004).
- `openRoomModal(display)`: fill `#room-modal-title`, `#room-modal-description`
  (`display.full`), `#room-modal-exits` (chips), toggle `#room-modal-media` by
  `display.mediaHint` (reserved/empty now); `dialog.showModal()` (REQ-006).
- Close: native `<dialog>` Esc + a `method="dialog"` close button; closing is an
  overlay dismiss — no game state touched, no re-render (REQ-007).
- `renderMenu` Nearby: unchanged wiring; empty state label clarified to
  "No one else is here." (REQ-002).

**`index.html`:** in `.room-brief` add `#exit-line`, `#view-room-button`, replace
`#exits` with `#exit-pad`; add a `<dialog id="room-modal">` (title, media area
[hidden], full description, exits) with a `method="dialog"` close button.

**`styles.css`:** `.exit-pad` (3×5 `grid-template-areas` for U/N/W·E/S/D),
`.exit-pad button` + `.is-quiet`/`:disabled` (visually quiet), `.view-room-button`,
`.exit-line`, `.room-modal` (+ `::backdrop`, header/close, media slot, exits).

### File manifest
  | # | File | Change |
  |---|---|---|
  | 1 | `src/client/room.js` | ADD: `toRoomDisplay` + `toExitPad` + `DEFAULT_ROOM_DISPLAY_LIMIT` |
  | 2 | `src/client/snapshot.js` | MODIFY: `toNearby` → contents-only (empty today); REMOVE `toRoomBrief` |
  | 3 | `src/client-app.js` | MODIFY: import room.js; `renderRoom` (main + exit-line + always-on View room); `renderExitPad`; `openRoomModal`/close; Nearby empty label |
  | 4 | `index.html` | MODIFY: `.room-brief` (exit-line, view-room button, `#exit-pad`); add `<dialog id="room-modal">` |
  | 5 | `styles.css` | MODIFY: `.exit-pad`, quiet/disabled, `.view-room-button`, `.exit-line`, `.room-modal` |
  | 6 | `tests/client.test.js` | MODIFY: update T7 (drop `toRoomBrief`); ADD room.js + toNearby cases |

### Regression Test Plan
  | # | Test | Proves |
  |---|---|---|
  | TR1 | `room.toRoomDisplay` — long desc → `main` summarized + `…`, `truncated:true`, `full` whole; short desc → `main===full`, `truncated:false`; `shortDescription`/`fullDescription`/`mediaHint` read when present; custom `limit` honored; exits mapped `{direction,command,label}` | REQ-005, REQ-008 |
  | TR2 | `room.toExitPad` — six canonical dirs in order `[up,north,west,east,south,down]`; `available` true only for room exits; `command===dir`; `availableCount`; empty exits → all unavailable | REQ-003, REQ-004 |
  | TN | `snapshot.toNearby` — no contents → `{count:0,items:[]}`; never includes exits even when `room.exits` present (REQ-001); `room.contents` present → mapped (forward-compat) | REQ-001, REQ-002 |
  | T7′ | `snapshot` — `toHud` + `toMenuModel` still feed HUD + tabbed menu (updated to drop `toRoomBrief`) | REQ-002 (menu) |
  | T5,T6,T8,T8b,T9 | unchanged (wire/components/map/map-zplane/intent regress green) | regression |
  | S1 | Browser smoke: Exit Pad — enabled dir moves (feed+map+HUD update), disabled dirs quiet; Nearby shows empty "No one else is here." (no exit cards); View room opens the modal (title/full/exits/media slot), close leaves prompt/feed/map/state intact | REQ-001..007 (manual) |
  | G1 | `npm test` + `npm run build` + `./bin/gate.sh --fast` green | REQ-009 |

Uncoverable-by-unit-test (documented smoke): `src/client-app.js` DOM/dialog glue
(`showModal`/`close`, exit-pad DOM, view-room button) — browser-only, not imported by
a test (mirrors the established `app.js`/#12 `client-app.js` exclusion); verified by S1.
REQ-005 live truncation: beginner descriptions are 77–106 chars (< 160 limit) so they
render in full live; truncation is unit-proven (TR1) and engages when the server
supplies longer text or a separate `fullDescription`.

### Risks / decisions
- **`toRoomBrief` removal**: update every reference (client-app import + `renderRoom`,
  the T7 test). Missed reference → build/test fail; caught by `node --check` +
  `npm run build` + tests.
- **"View room" always shown** (not only when truncated): makes the focused full view
  (REQ-006/007) reachable from any room — the beginner rooms are short, so a
  truncation-gated button would never appear live. REQ-005 stays satisfied (when
  truncated, the truncated text + action are both present); always-on is a superset.
- **Native `<dialog>`**: standard HTML, builds under vite, framework-free (REQ-008);
  close preserves state because the modal is a pure overlay over `latestSnapshot` +
  the feed/map DOM.
- **JS coverage (FULL)**: new pure `room.js` fully unit-tested; removing `toRoomBrief`
  drops an exercised export (net-neutral). Glue stays uncounted. Floor 75% holds.
- **No Rust/protocol change** → Rust gates unaffected; `npm run build` (vite) is the
  new build check this ticket adds to the bar.
- **Forge (AAR 6718862d)**: AD-claude-ui-shell-001 (rank 1) governs the split;
  PR-claude-minimap-zplane-001 / BF-map-zplane-001 → keep movement canonical + test
  the Exit Pad against real beginner exits.

## Phase 3 — Implement
- **Built (6 files):**
  - `src/client/room.js` — `toRoomDisplay` (title/main/full/truncated/mediaHint/exits;
    `summarize` at sentence→word boundary) + `toExitPad` (6 canonical dirs, available
    flags, canonical commands) + `DEFAULT_ROOM_DISPLAY_LIMIT = 160`.
  - `src/client/snapshot.js` — `toNearby` now reads room `contents`/`actors`/`items`/
    `fixtures` (empty today, no exits); removed dead `toRoomBrief` + its `capitalize`.
  - `src/client-app.js` — import `room.js`, drop `toRoomBrief`; `renderRoom` → main
    desc + `Exits:` line + always-on "View room"; `renderExitPad` (enabled→`runCommand`,
    unavailable→`disabled`+`.is-quiet`); `openRoomModal` (`<dialog>.showModal`, title/
    full/exits/reserved media); bind view-room + backdrop-click close; Nearby empty
    label "no one here".
  - `index.html` — `.room-brief` (room-text + `#exit-line` + `#view-room-button` +
    `#exit-pad`); `<dialog id="room-modal">` (title, media slot, full desc, exits).
  - `styles.css` — `.exit-pad` grid-template-areas (U/N/W·E/S/D), `.exit-pad-button`
    + `.is-quiet`/`:disabled`, `.view-room-button`, `.exit-line`, `.room-modal`
    (+ `::backdrop`, header/close, dashed media slot, exits).
  - `tests/client.test.js` — T7 updated (drop `toRoomBrief`, Nearby count 0); added
    TR1 (toRoomDisplay), TR2 (toExitPad), TN (toNearby).
- **Compile/check as I went:** `node --check` ok on all modules; `node --test
  tests/*.test.js` 13/13 pass; `npm run build` ✓ (11 modules, dist built).
- **Deviations from design (+ reason):**
  - Removed `capitalize` from `snapshot.js` too — it was only used by the removed
    `toRoomBrief` (would be dead/clippy-style unused otherwise). `room.js` has its own.
  - Added a backdrop-click-to-close on the dialog (small UX nicety) + guarded
    `showModal` with a `typeof` check (defensive for environments without `<dialog>`);
    neither changes game state on close (REQ-007 holds).
  - "View room" is always visible (per the Phase 2 decision) so the focused view is
    reachable from any room.

## Inspect (Phase 3.5)
- **Lenses run:** 3 parallel critics — correctness; gate-risk/coverage/build;
  simplification/reuse + security(XSS).
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | F1 | MED | `summarize` word-cut/hard-cut branch untested — the long-text test only hit the sentence branch (`room.js:46-48`); a future FULL/mutation run would surface it | REAL | broadened TR1 to force sentence / word-cut / hard-cut paths (+ no-space token) |
  | F2 | LOW | awkward `". …"` ellipsis on the sentence path (period+space+ellipsis reads as a typo) | REAL — polish | sentence path returns a **clean** first sentence (no ellipsis); word-cut attaches `…` with no leading space |
  | F3 | LOW | dead `if (text.length <= limit) return text` guard in `summarize`; `main` could exceed limit by the suffix | REAL | removed the guard (caller guards length); word-cut now ≤ limit+1 |
  | F4 | LOW | empty `full` when only a future `shortDescription` is present | REAL — defensive | `full` fallback now includes `shortDescription` |
  | F5 | LOW | unused `region`/`subregion` on the display model | REAL | removed — REQ-008 names exactly title/main/full/media(+exits) |
  | F6 | LOW | `toNearby` actors/items/fixtures assembly branch unasserted (`snapshot.js`) | REAL | added a TN case passing `{actors,items,fixtures}` |
  | F7 | LOW | `direction` vs `dir` naming split between `exitList` and `toExitPad` | REJECTED | different shapes (exit list vs fixed pad slot); both clear in context — churn not worth it |
  | F8 | LOW | the #13 (and #12) client files are untracked in git | NOTED | expected — `/commit` will `git add` them; no impact on `--fast` |
  | F9 | LOW | dead `#exits` querySelector in the unreferenced prototype `src/app.js` | REJECTED | out of #13 scope; `app.js` is the retained prototype, not loaded by the page |
- **Security lens: CLEAN** — every server-derived string (room title/main/full,
  exit labels, modal title/description, `mediaHint.alt`) renders via `textContent`
  / `setAttribute`; grep for `innerHTML`/`insertAdjacentHTML`/`eval` → zero. All 22
  emitted CSS classes + grid-areas + element ids resolve.
- **Post-fix verification:** `node --test tests/*.test.js` 13/13; JS coverage 83.31%
  (branch ↑ to 64.75% — word-cut path now covered) ≥ 75 floor; `npm run build` ✓;
  `./bin/gate.sh --fast` GREEN (14/14).
- **Forge capture:** failure `BF-room-summarize-untested-001`; prevention rule
  `PR-claude-text-truncation-branches-001`.

## Phase 4 — Validate
- **Tests added (run here):** JS `tests/client.test.js` — TR1 (`toRoomDisplay`,
  3 summarize paths + future fields), TR2 (`toExitPad`), TN (`toNearby`
  empty/contents/assembled), T7 updated (drop `toRoomBrief`, Nearby count 0). No
  Rust tests (client-only change).
- **`cargo test --workspace`:** PASS — 116/116 (12+68+16+20), 0 failed; Rust
  unaffected (no Rust/protocol change).
- **`node --test tests/*.test.js`:** PASS — 13/13 (6 client + 4 prototype + …),
  0 failed.
- **`npm run build` (vite):** PASS — 11 modules, `dist` built (JS 17.43 kB). Structural
  smoke: `dist/index.html` ships `#exit-pad`, `#exit-line`, `#view-room-button`,
  `#room-modal` (+ title/description/exits/media); the bundle includes `room.js`
  (`exit-pad-button`, `availableCount`) + the "no one here" Nearby empty state.
- **`./bin/gate.sh --fast`:** `GATE GREEN [fast]` — 14 passed, 0 failed (REQ-009).
- **Live data backing the smoke:** `/state` start room exits =
  `{east, north, up}` → pad enables E/N/U, quiets S/W/D; description ~100 chars
  (< 160 limit) → renders full on the surface, modal shows the same full text.

### AC → verification traceability
| REQ | Verified by | Result |
|---|---|---|
| REQ-001 | TN (`toNearby` never returns exits) + structural smoke (no exit cards; "no one here") | PASS |
| REQ-002 | TN (empty `{count:0,items:[]}`) + smoke | PASS |
| REQ-003 | TR2 (six canonical dirs) + dist `#exit-pad` + live exits | PASS |
| REQ-004 | TR2 (`available` + `command===dir`) + live (E/N/U enabled, S/W/D quiet) + core parser (bare dirs) | PASS |
| REQ-005 | TR1 (sentence/word-cut/hard-cut summarize) + always-on "View room" ships; beginner desc < limit (unit-proven) | PASS |
| REQ-006 | room-modal DOM ships + `openRoomModal` (title/full/exits/reserved media, no state mutation) + browser smoke | PASS |
| REQ-007 | review (modal is a pure overlay; native close, no re-render) + browser smoke | PASS |
| REQ-008 | TR1 (model = title/main/full/mediaHint, future fields read) + review | PASS |
| REQ-009 | `npm test` 13/13 + `npm run build` ✓ + `./bin/gate.sh --fast` GREEN | PASS |

### Browser smoke notes (manual; glue is documented-smoke per AD-claude-ui-shell-001)
Run `npm run server:dev` + `npm run dev`, open the proxied client:
1. **Exit Pad movement** — Hollowmere Square renders the pad with **E/N/U enabled**
   and **S/W/D quiet/disabled**. Clicking **N** sends `north` (identical to typing
   it) → feed shows the room change, HUD/map update; clicking a quiet direction does
   nothing.
2. **Nearby empty state** — the Nearby tab shows **"no one here"** with **no exit
   cards** (exits live only in the pad + the `Exits: …` line).
3. **Full-room modal** — clicking **"View room"** opens the dialog (title, full
   description, exit chips, hidden media slot); **Esc / close button / backdrop**
   closes it; afterward the **command prompt, feed, map, and current room are
   unchanged**.

- **Pre-existing exclusions:** none — no pre-existing failures; all suites green.
- **FULL gate (15–17 coverage + mutation):** deferred to `/commit` per scope.
  Spot-check: JS line coverage 83.31% (≥ 75); no Rust change so mutation MSI
  untouched. `src/client-app.js` glue stays uncounted (browser-only).

## Phase 5 — Complete
- **Docs updated:** `docs/ui-design.md` Implementation Status — added the #13
  refinements (Nearby contents-only, Exit Pad, full-room modal). No new
  `decisions.md` entry: this is a refinement within Decision 032 / AD-claude-ui-shell-001.
- **Forge capture:** `aar-submit` (aar 6718862d, outcome completed, effectiveness 4,
  12 verdicts, 2 novel → distillation/confidence-drift/pattern-emergence enqueued);
  failure `BF-room-summarize-untested-001` + prevention rule
  `PR-claude-text-truncation-branches-001` (recorded at inspect).
- **Ticket:** forge #13 → `in-review` (NOT done) + summary comment. FULL gate +
  `/commit` deferred per the requester's `--fast` scope.
- **Archived:** pipeline pair moved to `docs/planning/pipeline/completed/`; local
  ticket doc kept in `tickets/open/` at status `in-review` (commit pending).
