# WORK-studio-editor-rooms-tab-v1 — Notes

## Phase 1 — Plan
- **Request:** the editor Rooms tab — a per-room metadata inspector (edit title/description/region/
  subregion, list-select) + a map-title field. Pivot ③ slice c1 (memory `studio-editable-world-pivot`).
- **Classification / tier:** work pipeline, one slice. `oathstar-content` (a new `update_room` +
  `UnknownRoom`) + `oathstar-studio` (room_op, route, Rooms-tab markup + glue, editorRoomRows,
  #map-title) + tests.
- **Recon (main `115f8c5`):**
  - `RoomCell` (`map_document.rs:91`): `x/y/z/id/title:Option/description:Option/region/subregion:
    Option/glyph:Option/combat_enabled/exits:BTreeMap/entities`.
  - **No room edit-CRUD** on `MapDocument` (only region/subregion CRUD in `map_edit.rs`). So
    `update_room` is NEW.
  - `RegionEditError` (`map_edit.rs:24`) = the map-edit error enum (`BlankField`/`UnknownRegion`/
    `UnknownSubregion`/`WouldBreakWorld`/…, `impl Display`). `finish(self, catalog)` (`:135`)
    re-validates the edited clone via `validate()` → `WouldBreakWorld`. `update_region` (`:182`) is
    the mirror (`non_blank` → clone → mutate → `finish`).
  - `region_op` (#62, `editor.rs`) + `editorRegionRows` + the Regions-tab glue are the studio mirrors;
    the `#map-name`/Save glue (`doc.id`) is where `#map-title` (`doc.title`) slots in.
- **FORGE FLAGGED (recall):** the #62 failure `BF-region-rename-wipes-description-001` + its rule
  `PR-claude-roundtrip-edit-echo-unchanged-fields-001` → `update_room` is designed as a **PARTIAL
  update** (set the 4 fields, preserve the rest), which structurally avoids that class.
- **Approach (design refines):** `update_room` (partial) + `UnknownRoom`; `room_op` dispatch; the
  Rooms-tab room list → click → inspector (title/description + region/subregion selects) → `room_op
  "edit"` → swap the doc; `editorRoomRows` pure; the `#map-title` field.
- **EARS:** REQ-001 update_room edits-4 + preserves-rest · REQ-002 unknown room · REQ-003 blank/
  undeclared region · REQ-004 room_op 200/400/auth · REQ-005 editorRoomRows · REQ-006 Rooms tab UI +
  #map-title · REQ-007 gate.
- **Mutation surface:** `update_room` (the find + the field sets + `finish`) + `editorRoomRows` (the
  map + title-fallback) — killed by the rust + node tests. `room_op` is dispatch-only (no viable
  Rust mutant, like `region_op`) — coverage is the bar.
- **Ticket:** forge **#63** `09c70ac8-3cd7-4a0c-ab49-1ef0d34e1fb9`. Local doc
  `docs/planning/tickets/open/TICKET-63-studio-editor-rooms-tab.md`.
- **aar_id:** `735787dd-b4c5-4774-9581-c09fc254400b`
- **Delivery:** AUTONOMOUS through commit+push+FF-merge (room-metadata run). Branch off `main`
  `115f8c5`. Stash parked.

## Phase 2 — Design

### Code reconnaissance
- `MapDocument.rooms: Vec<RoomCell>` (`map_document.rs:273`, pub); `RoomCell` fields all pub → an
  `iter_mut().find()` + per-field set works. `update_region` (`map_edit.rs:182-198`) is the exact
  mirror: `non_blank` → `let mut edited = self.clone()` → `get_mut`/find → set fields → `finish(catalog)`.
- Save glue (`render.rs`): `nameInput.value = doc.id` (`:577`); `doc.id = nameInput.value.trim()`
  (`:579`); `JSON.stringify(doc)` (`:586`). So `#map-title` reads/writes `doc.title` alongside —
  setting `doc.title` before `:586` ships it (the Save posts the full doc).

### Approach / architecture
- **`map_edit.rs`** — add `UnknownRoom { id: String }` to `RegionEditError` + its `Display`
  ("no room '{id}' exists"). NEW `update_room(&self, id, title: Option<String>, description:
  Option<String>, region: &str, subregion: Option<String>, catalog) -> Result<Self, RegionEditError>`:
  `let region = non_blank(region, "region")?;` → `let mut edited = self.clone();` →
  `let Some(room) = edited.rooms.iter_mut().find(|r| r.id == id) else { return
  Err(UnknownRoom{id}) };` → set **only** `room.title = title; room.description = description;
  room.region = region.to_owned(); room.subregion = subregion;` (**preserve** x/y/z/glyph/
  combat_enabled/exits/entities) → `edited.finish(catalog)` (re-validate — an undeclared region/
  subregion → `WouldBreakWorld`). The inspector pre-trims (empty → `null`), so the method sets the
  `Option`s as given.
- **`editor.rs`** — `#[derive(Deserialize)] struct RoomOp { document, op, #[serde(default)] id,
  title: Option<String>, description: Option<String>, region: String, subregion: Option<String> }`
  (the `Option` fields `#[serde(default)]`). `room_op` mirrors `region_op`: gate → parse →
  `match op { "edit" => update_room(…), _ => refuse(400) }` → `Ok(doc)=Json` / `Err=refuse(400,&msg)`.
- **`main.rs`** — `POST /editor/maps/room-op`.
- **`render.rs`** — `#panel-rooms`: `<h2>Rooms</h2><ul id="room-list" class="room-list">` + a
  `<form id="room-inspector" hidden>` (`#room-title`, `#room-desc`, region `<select id="room-region">`,
  subregion `<select id="room-subregion">`, `<pre id="room-result">`, `<button id="room-save">`).
  Controls: `<label>Title <input id="map-title"></label>` next to `#map-name`. EDITOR_GLUE rooms
  controller: `renderRooms()` builds the list from `editorRoomRows(doc)` (textContent; each row a
  button with `dataset.roomId`); a row click → `selectRoom(id)` (find the room, populate the inputs,
  rebuild the region/subregion `<select>`s from `Object.values(doc.regions|subregions)` with
  `textContent` `<option>`s + the current value selected, the subregion select has a `(none)` option,
  un-hide the inspector); `#room-save` → `roomOp("edit", {id, title: t.trim()||null, description:
  d.trim()||null, region: regionSel.value, subregion: subSel.value||null})` → on 200 `doc = await
  res.json(); renderRooms(); redraw();` else show `body.message`. The `#save` handler also sets
  `doc.title = mapTitle.value.trim()` before its POST; `mapTitle.value = doc.title` on load.
- **`editor-canvas.js`** — `export function editorRoomRows(doc)` → `((doc&&doc.rooms)||[]).map(r =>
  ({ id: r.id, title: r.title ?? r.id, region: r.region, subregion: r.subregion ?? null }))`. Pure.
- **`studio.css`** — `.room-list` (like `.region-list`), `.room-inspector` (a grid); extend the
  `#result`/`[data-ok]` selectors to `#room-result` (as the #62 fix did for `#region-result`).

### Locked decisions (this phase)
- **`update_room` is a PARTIAL update** — only the 4 metadata fields; x/y/z/glyph/combat/exits/entities
  are untouched (the structural fix for the #62 data-loss class). `finish(catalog)` is the validity
  boundary (undeclared region/subregion → `WouldBreakWorld`); no extra pre-checks needed.
- `room_op` dispatch-only (no viable Rust mutant — coverage is the bar). List/inspector/`<option>`s
  via `textContent` (XSS-safe). The subregion select includes a `(none)` option (it's Optional).
- **Edit only** — create/delete rooms (③c3), canvas-click (③c2), exits/glyph editing (later) are out.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-content/src/map_edit.rs` | `UnknownRoom` variant + Display; NEW `update_room` (partial); unit tests. |
| 2 | `crates/oathstar-studio/src/editor.rs` | `RoomOp` struct + `room_op` handler; tests. |
| 3 | `crates/oathstar-studio/src/main.rs` | route `POST /editor/maps/room-op`. |
| 4 | `crates/oathstar-studio/src/render.rs` | `#panel-rooms` list+inspector; `#map-title` control; EDITOR_GLUE rooms controller + the `doc.title` save wiring; render test. |
| 5 | `crates/oathstar-studio/static/editor-canvas.js` | NEW pure `editorRoomRows`. |
| 6 | `crates/oathstar-studio/static/studio.css` | `.room-list`/`.room-inspector`; `#room-result` styling. |
| 7 | `tests/studio-editor-canvas.test.js` | `editorRoomRows` cases. |
| 8 | `docs/map-system.md` | (Phase 5) the Rooms tab edits room metadata. |

### Regression Test Plan
| # | Test | Proves |
|---|---|---|
| T1 | `update_room` (a doc with a room carrying a `glyph` + an `exit`) editing title/description → the 4 fields change **and** `glyph`/`exits`/`x`/`y`/`z`/`combat_enabled` are unchanged (assert the whole `RoomCell`) | REQ-001 (map_edit unit) |
| T2 | `update_room` with an unknown id → `UnknownRoom`, document unchanged | REQ-002 |
| T3 | blank region → `BlankField`; an **undeclared** region → `WouldBreakWorld` (Err) | REQ-003 |
| T4 | `room_op` `"edit"` → `200` (returned doc's room updated); unknown op → `400`; no session → `401`; Player → `403` | REQ-004 (editor) |
| T5 | `editorRoomRows` — `title` falls back to `id` when null; `region`/`subregion` carried; `{}`/null → `[]` | REQ-005 (node) |
| T6 | the editor `#panel-rooms` has `id="room-list"` + the inspector inputs (`room-title`/`room-region`) + `editorRoomRows(`; the controls have `id="map-title"` | REQ-006 (render) |
| G1 | `bin/gate.sh` FULL green, MSI 100% | REQ-007 |
- **Mutation:** `update_room`'s `find(|r| r.id == id)` `==` (→ `!=` killed by T1/T2) + the `non_blank`
  path (T3); `editorRoomRows` `?? r.id` (T5). The field assignments aren't mutants targets; `room_op`
  is dispatch-only (no viable mutant) — coverage from T4. **Uncoverable:** none new.

### Risks / decisions
1. **The preserve test (T1) needs a fixture room with a glyph + an exit** — VALID_DOC's rooms have
   neither. Construct a small valid doc (2 rooms, an exit between them, a glyph on one) in the
   map_edit unit test; edit title/description only (not region) so `finish` stays valid, and assert
   the non-edited fields survive.
2. **`null` title clears to the materialize default** — the inspector turns an empty title input into
   `null`; `update_room` stores `None`; `finish` re-validates. That's intended (a room with no title
   override). Not data loss (the user chose to clear it).
3. **`region` change validity** — editing a room to an undeclared region is refused by `finish`
   (`WouldBreakWorld`); the inspector's region `<select>` only offers declared regions, so the UI
   path can't hit it (the test does, directly).

## Phase 3 — Implement
- **Built (manifest as designed):**
  - `map_edit.rs` — `UnknownRoom { id }` variant + Display; NEW `update_room` (a **partial** update:
    `non_blank(region)` → clone → `rooms.iter_mut().find(id)` or `UnknownRoom` → set only
    `title`/`description`/`region`/`subregion` → `finish(catalog)`; coords/glyph/combat/exits/entities
    untouched).
  - `editor.rs` — `RoomOp` struct + `room_op` handler (`"edit"` → `update_room`, `_` → 400, `Ok` →
    `Json(doc)` / `Err` → `refuse(400, &msg)`), mirroring `region_op`.
  - `main.rs` — `POST /editor/maps/room-op` route.
  - `render.rs` — `#map-title` control + the save wiring (`mapTitle.value = doc.title` on load;
    `doc.title = mapTitle.value.trim()` before the Save POST); `#panel-rooms` = `#room-result` +
    `#room-list` + a hidden `#room-inspector` form; EDITOR_GLUE rooms controller (`optionList` /
    `selectRoom` populating the inputs + region/subregion `<select>`s via `textContent` `<option>`s /
    `renderRooms` (the list, textContent) / `roomOp` (POST → swap `doc` → `renderRooms()` + `redraw()`)
    + the `#room-save` wiring).
  - `editor-canvas.js` — pure `editorRoomRows(doc)` (title-fallback-to-id, null-safe).
  - `studio.css` — `.room-list`/`.room-list button`/`.room-inspector`; `#room-result` folded into the
    `#result`/`#region-result` box + data-ok selector groups.
- **Deviations:** none of substance. `update_room` is a partial update (the #62-class fix); `doc` is
  reassigned on a room op (matches the regions path); strings rendered via `textContent`.
- **Checks:** `cargo check`/`clippy -p oathstar-content -p oathstar-studio --all-targets` clean;
  `cargo fmt` clean; `node --check editor-canvas.js` OK; `cargo test -p oathstar-content
  -p oathstar-studio` → **content 113 + studio 98 passed** (unchanged — existing suites green). New
  tests + gate at Phase 4.

## Inspect (Phase 3.5)
- **Lenses:** 2 read-only `Explore` critics: correctness/security (focused on the partial-update
  preservation + XSS) + simplification/reuse. **Both CLEAN.**
- **Critic 1 (correctness/security) — no findings.** Verified the **critical guardrail**: `update_room`
  sets **only** `room.title`/`description`/`region`/`subregion` and leaves `x`/`y`/`z`/`glyph`/
  `combat_enabled`/`exits`/`entities` (and `items`/`fixtures`) **untouched** — so a room's glyph/exits/
  coords survive an edit (the #62 `BF-region-rename-wipes-description` data-loss class is structurally
  avoided). Also: `non_blank(region)` → clone → `find(|r| r.id == id)` (correct `==`) else `UnknownRoom`
  → `finish(catalog)` re-validates (undeclared region/subregion → `WouldBreakWorld`); no panic;
  `room_op` mirrors `region_op` (gate/parse/dispatch/refuse); **XSS-safe** — the list + `<option>`s are
  built with `textContent` (never `innerHTML`), endpoint returns JSON; `doc = await res.json()`
  reassigns the live `let doc` (Save/redraw see the new rooms); the Save sets `doc.title` **before**
  `JSON.stringify(doc)`; `editorRoomRows` null-safe + title-fallback. **211 tests pass.**
- **Critic 2 (reuse) — clean.** `update_room`↔`update_region`, `room_op`↔`region_op`,
  `editorRoomRows`↔`editorRegionRows` all mirror; `UnknownRoom` added cleanly to `RegionEditError`;
  `RoomOp` minimal (`#[serde(default)]` Options, all used); CSS consistent (`#room-result` folded into
  the shared selector group, `:root` tokens); no dead code; the `#map-title` is wired both ways.
- **One [low] — REJECTED (non-issue).** The subregion load passes `room.subregion ?? ""`; the critic
  noted it's "slightly indirect" but **works correctly** (a null subregion → `""` → no `entry.id`
  matches → `if (!current)` selects the `(none)` option; Save maps `value || null` back) — the critic
  itself marked it "not a bug / fix optional". No change.
- **No code fix; no `failure-record`** (no defect — the design's partial-update structurally applied
  the #62 prevention rule `PR-claude-roundtrip-edit-echo-unchanged-fields-001`). The Phase-4 tests
  (update_room/room_op rust + render + node editorRoomRows) are scope, not findings.

## Phase 4 — Validate
- **Tests added (T1–T6, all green):**
  - `map_edit.rs` — **T1** `update_room_edits_metadata_and_preserves_the_rest` (a `doc_with_a_decorated_room`
    fixture: `start` carries `glyph: Some('A')` + an `east→hall` exit; after editing title/description
    the glyph/exit/coords/region survive — the #62-class guard); **T2** `update_room_refuses_unknown`
    (`UnknownRoom`); **T3** `update_room_refuses_a_blank_or_undeclared_region` (`BlankField` +
    `WouldBreakWorld`).
  - `editor.rs` — **T4** `room_op_edits_a_room` (200; the returned doc's `alpha` has the new
    title/description) + `room_op_refuses_a_bad_op_and_non_editors` (unknown op 400, anon 401, Player 403).
  - `render.rs` — **T6** `editor_page_rooms_tab_has_the_room_ui` (`#room-list`/`#room-inspector`/
    `#room-region`/`#room-save` + `editorRoomRows(`/`renderRooms(`/`fetch("/editor/maps/room-op"` +
    `#map-title`).
  - `tests/studio-editor-canvas.test.js` — **T5** `editorRoomRows` (title-fallback-to-id; region/
    subregion carried; `{}`/null/undefined → `[]`).
- `cargo test --workspace`: **PASS** — oathstar-content **116** (+3), oathstar-studio **101** (+3).
- `node --test tests/*.test.js`: **PASS** — **92** tests, 0 fail.
- `bin/gate.sh`: **GATE GREEN [full]** — all 17 gates; **mutation 600 caught / 0 missed → MSI 100.0%**
  (the new `update_room` `==`/`non_blank` + `editorRoomRows` `?? id` mutants killed; `room_op` is
  dispatch-only, covered by T4). No pre-existing exclusions.

## Phase 5 — Complete
- Docs / forge / ticket / archived:
