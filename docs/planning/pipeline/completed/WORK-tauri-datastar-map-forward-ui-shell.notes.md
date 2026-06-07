# WORK-tauri-datastar-map-forward-ui-shell — Notes

Per-phase working notes for the paired `.spec.md`. The spec is the contract;
this is the record of what happened.

## Phase 1 — Plan
- **Request:** Start ticket #12 — build the first playable Tauri + Datastar/SSE
  map-forward UI shell for the beginner vertical slice; smallest
  production-direction shell satisfying the ticket; run targeted tests +
  `./bin/gate.sh --fast`. Auto-approve through the pipeline.
- **Intake source:** `docs/planning/intake/INTAKE-beginner-slice-ui-startup.md`
  (promoted from ticket #7's UI/startup residual).
- **Classification / tier:** Work pipeline (one shippable slice). Touches the
  server SSE boundary (small Rust change) + a new JS client. Not large enough to
  split; map render config and Datastar-library adoption are explicitly deferred.
- **Forge recall (lessons/failures surfaced):**
  - AAR opened: `30b3f1ce-d02b-4629-b055-3ae2fe741bc7`. `knowledge-context` will
    be pulled per phase.
  - Decision 031 (locked): domain events emit only from `handle_command` +
    `begin`; `try_new` emits nothing → REQ-001 must replay `begin()`. Wire split:
    `/events` snake_case payload, `/state` camelCase.
  - Decision 028 / 016 / 015: hybrid typed events, REST + SSE first, Rust server
    is authority; Datastar is the preferred (prototyped) direction, not a hard
    requirement to vendor the library now.
  - `node --experimental-test-coverage` only measures test-imported modules
    (floor 75%); `app.js`-style DOM glue is invisible to it. → pure/glue split.
- **Ticket:** forge #12 `a063dadb-6a68-4380-80b7-21c55966aead` (already minted by
  owner; framework "Tauri + Datastar", language "Rust + JavaScript").
- **EARS requirements reviewed:** REQ-001..009 carried verbatim from the ticket;
  all are observable + verification-tagged. REQ-002/003/006 verified via the pure
  render core under `node --test`; REQ-004/005 via documented smoke; REQ-007/008
  via review; REQ-001 via server integration test; REQ-009 via `gate.sh --fast`.

### Grounding (code facts established before Phase 2)
- Server `crates/oathstar-server/src/main.rs`: Axum; routes `/`, `/health`,
  `/state` (GET → `GameSnapshot`), `/command` (POST → `CommandResponse`),
  `/events` + `/events/json` (SSE JSON `game_event`), `/events/html` (SSE HTML
  `game_event_html`). New subscribers receive nothing until the next broadcast —
  the opening scene is never replayed (REQ-001 gap). `spawn_tick_loop` broadcasts
  a `Tick` every 1s.
- Core `crates/oathstar-core/src/lib.rs`: `begin(&mut self) -> Vec<GameEvent>`
  emits `RoomEntered{hollowmere_square}` + `describe_current_room()` (RoomHeader +
  narrative); does **not** move the player. `snapshot(&self) -> GameSnapshot`.
- Protocol `crates/oathstar-protocol/src/lib.rs`: `GameEvent{eventId,tick,channel,
  ...kind}`; kinds `log_message{component,text}`, `tick{value}`, `room_entered{
  room_id,title}`, `oath_sworn{oath_id,title}`, `oath_fulfilled{oath_id}`;
  channels narrative/room/combat/.../oath/system/debug; `OutputComponent`
  narrative_message/room_header/system_message/.../map_patch. Snapshots camelCase.
- Frontend: `index.html` + `styles.css` already implement the map-forward layout
  (HUD meters, stage/map grid, feed + command form, tabbed Nearby/Oaths/Gear/Pack
  menu, Intent search + quick-commands). `src/app.js` drives it against the
  in-browser `src/engine.js` (prototype), NOT the server. `tests/game.test.js`
  exercises `engine.js` via `node --test` (pure).
- Tauri `src-tauri/tauri.conf.json`: `devUrl http://localhost:5173`,
  `frontendDist ../dist`; loads the vite `index.html` entry. Shell compiles under
  gate:14.
- Gate JS: gate:4 `node --test tests/*.test.js`; gate:16 (FULL only)
  `node --test --experimental-test-coverage tests/*.test.js`, floor 75%. `--fast`
  runs gate:4 (not gate:16). Server change is covered by gate:3 `cargo test`.

## Phase 2 — Design

### Approach / architecture
**Server (REQ-001) — seed `begin()` onto every subscription.**
- `AppState` gains `opening: Arc<Vec<GameEvent>>`. `main()` captures it once:
  `let mut engine = Engine::try_new(world)?; let opening = Arc::new(engine.begin());`
  before moving the engine into the `Mutex` and before `spawn_tick_loop`. `begin()`
  does not move the player, so `/state` stays at `hollowmere_square` (Decision 031).
- Extract a standalone `fn event_to_json(event: &GameEvent) -> Option<String>`
  (= `serde_json::to_string(event).ok()`), used by both the seed and the live loop.
  It is mutation-tested (normal fn); the seed loop itself lives **inside** the
  `stream!` macro, which cargo-mutants does not descend into → no MSI hole, no new
  deps. `events_json` and `events_html` iterate `app.opening` first (json via
  `event_to_json`, html via the existing tested `render_event_html`), each yielded
  with its `event_id`, then enter the existing live `recv` loop.
- `test_app_state()` mirrors `main()` (captures `opening` via `begin()`), keeping
  existing handler tests valid.

**Client — framework-free, server-authoritative, pure-core/glue split.**
- *Pure DOM-free core* under `src/client/` (imported by `tests/client.test.js`,
  counted by `node --experimental-test-coverage`):
  - `wire.js` `parseEvent(raw)` — normalize a `/events` `GameEvent` (camelCase
    outer `eventId/tick/channel`, snake_case kind `type/room_id/oath_id/component/
    text`, Decision 031) → camelCase client event. (REQ-003)
  - `components.js` `toComponent(ev)` — typed event → componentized output
    descriptor `{className,label,text,dataset}` (room header / narrative / oath /
    system / combat); `tick` → `null` (skipped from feed). Structured per-event
    articles = "not a textarea". (REQ-003)
  - `snapshot.js` — `toHud/toRoomBrief/toOaths/toNearby/toGear/toPack/toMenuModel`
    from the camelCase `/state` `GameSnapshot`. Gear/Pack are honest empty-states
    (no inventory in snapshot — out of scope). (REQ-002 render, REQ-005)
  - `map.js` — `DEFAULT_MAP_CONFIG={tilePixels:32,mode:'glyph'}`;
    `toMapModel(mapSnap, config)` → grid bounds + cells with current/discovered,
    passing `mode`+`tilePixels` through untouched; server `MapSnapshot` unchanged.
    (REQ-007)
  - `intent.js` — `COMMAND_VOCAB` + `suggestCommands(snapshot, query)` (substring
    filter, contextual: swear when no oath, confront when sworn, movement from
    exits) + `quickCommandsFor`. Free text preserved (separate command form).
    (REQ-006)
- *Browser-only glue* `src/client-app.js` (no test imports → smoke-verified):
  resolves `API_BASE` (`import.meta.env.VITE_OATHSTAR_API` || relative), GETs
  `/state` for first paint, opens `EventSource('/events')` (renders the seeded
  opening scene — REQ-001 visible without `look`), renders `game_event` SSE into
  the feed via the pure core, POSTs `/command` and re-renders HUD/map/panels from
  the response `snapshot` (feed comes only from SSE → no double-render), wires the
  existing tabbed menu + Intent search/quick-commands. Targets the existing
  `index.html` ids. (REQ-002/003/004/005/006)
- `index.html` repoints its module script to `./src/client-app.js`; the prototype
  (`app.js`/`engine.js`/`world.js`) stays as the documented reference, engine still
  tested.
- `vite.config.js` adds a dev `server.proxy` for `/command`,`/state`,`/events` →
  `127.0.0.1:7878` so the browser smoke is same-origin (SSE-friendly); no server
  CORS dependency added (smallest). Tauri uses the build-time `VITE_OATHSTAR_API`
  override (Tauri-ready; auto-spawn/CORS deferred per ticket Out).

### File manifest
  | # | File | Change |
  |---|---|---|
  | 1 | `crates/oathstar-server/src/main.rs` | Add `opening` to `AppState`; capture `begin()` in `main()` + `test_app_state()`; extract `event_to_json`; seed opening in `events_json`/`events_html`; add REQ-001 tests |
  | 2 | `src/client/wire.js` | ADD: parse `/events` event (Decision 031 casing) → normalized client event |
  | 3 | `src/client/components.js` | ADD: typed event → componentized output descriptor catalog |
  | 4 | `src/client/snapshot.js` | ADD: `/state` snapshot → HUD/room/oaths/nearby/gear/pack view models |
  | 5 | `src/client/map.js` | ADD: map snapshot + render config → grid model (configurable tile/mode) |
  | 6 | `src/client/intent.js` | ADD: command vocabulary + search/suggest + quick commands |
  | 7 | `src/client-app.js` | ADD: browser entry — EventSource/fetch glue wiring pure core into existing DOM |
  | 8 | `index.html` | MODIFY: repoint module script `./src/app.js` → `./src/client-app.js` |
  | 9 | `vite.config.js` | MODIFY: add dev `server.proxy` for `/command`,`/state`,`/events` |
  | 10 | `tests/client.test.js` | ADD: `node --test` cases for wire/components/snapshot/map/intent |

### Regression Test Plan
  | # | Test | Proves Requirement |
  |---|---|---|
  | T1 | Rust `opening_scene_is_captured` — `test_app_state().opening` has `RoomEntered{hollowmere_square}` + RoomHeader "Hollowmere Square" | REQ-001 (capture) |
  | T2 | Rust `event_to_json_emits_wire_shape` — sample `room_entered` → string with `"type":"room_entered"` + snake_case `room_id` | REQ-001 wire (Decision 031) |
  | T3 | Rust `opening_scene_seeds_serialize` — every `opening` event → `Some(_)`; joined payloads contain `hollowmere_square` + `Hollowmere Square` (bytes seeded on subscribe) | REQ-001 (seed delivery) |
  | T4 | Rust `events_html_opening_renders_room` — opening via `render_event_html` contains `message-room` + "Hollowmere Square" | REQ-001 (html seed) |
  | T5 | JS `wire.parseEvent` — room_entered/oath_sworn/log_message/tick snake_case → normalized camelCase; unknown tolerated | REQ-003 |
  | T6 | JS `components.toComponent` — room/narrative/oath/system → className+label; tick → null | REQ-003 |
  | T7 | JS `snapshot.*` — HUD pcts, room exits, oath panel (available/sworn/fulfilled), gear/pack empty-states | REQ-002, REQ-005 |
  | T8 | JS `map.toMapModel` — bounds, current/discovered cells, mode+tilePixels passthrough (glyph→ascii/16), snapshot not mutated | REQ-007 |
  | T9 | JS `intent.suggestCommands` — substring filter; contextual swear/confront/movement; vocab additive (free text unconstrained) | REQ-006 |
  | S1 | Smoke: `npm run server:dev` + `npm run dev`, browser → opening scene renders without `look`; submit `swear`/`north` updates feed+HUD+map; tabs + Intent work | REQ-001/002/003/004/005/006 (manual) |
  | R1 | Review: no react/vue/svelte added (package.json deps unchanged); map config client-side, server shape untouched | REQ-007, REQ-008 |
  | G1 | `./bin/gate.sh --fast` GREEN | REQ-009 |

Uncoverable-by-unit-test (documented smoke instead): `src/client-app.js` (EventSource/
fetch/`document`) — browser-only, not imported by any test (mirrors the established
`app.js` exclusion); verified by S1. REQ-001 socket-level SSE test intentionally
omitted — axum `Event` is opaque and a real socket test needs dev-deps; the shared
`event_to_json`/`render_event_html` serialization path is asserted directly (T1–T4).

### Risks / decisions
- **Cross-origin dev** (vite :5173 → server :7878): solved with a vite dev proxy
  (same-origin in dev, SSE-friendly via http-proxy streaming); client uses relative
  URLs + `VITE_OATHSTAR_API` override. No server CORS dep (smallest).
- **Tauri prod wiring** (frontendDist + loopback / CORS / sidecar) deferred per
  ticket Out; client is Tauri-ready via the env override.
- **Datastar library not vendored** — deliberate (locked decision); record an
  architecture decision in Phase 5. REQ-008 satisfied (hypermedia + SSE, no SPA fw).
- **Double-render**: feed renders only from SSE; command-response `snapshot` drives
  HUD/map/panels.
- **Mutation (FULL)**: seed loops sit inside `stream!` (not mutated); the new
  standalone `event_to_json` is covered by T2/T3 → MSI stays 100%. `--fast` skips it.
- **Forge surfacings (AAR 30b3f1ce)**: failures 6e58ea62 / 360ee9a3 / f7e245c3,
  lesson b7eea654, AD 6b021064 / 2530308c / cba8fdeb — all in the begin()/wire-split/
  MSI cluster this design pins.

## Phase 3 — Implement
- **Built (all 10 manifest files):**
  - `crates/oathstar-server/src/main.rs` — `AppState.opening: Arc<Vec<GameEvent>>`;
    `main()` captures `engine.begin()` before the `Mutex`/tick loop; standalone
    `event_to_json`; both `events_json`/`events_html` seed the opening scene at the
    head of the `stream!` (before the live `recv` loop); `test_app_state()` +
    the tick-loop test fixture capture `opening`; tests T1–T4 added.
  - `src/client/wire.js` — `parseEvent` (Decision 031 casing → camelCase client event).
  - `src/client/components.js` — `toComponent` catalog (channel + OutputComponent →
    variant/label/text/dataset; `tick` → null).
  - `src/client/snapshot.js` — `toHud/toRoomBrief/toOaths/toNearby/toGear/toPack/toMenuModel`.
  - `src/client/map.js` — `DEFAULT_MAP_CONFIG`, `MAP_RENDER_MODES`, `toMapModel`
    (grid + current/discovered; tile/mode passthrough; input not mutated).
  - `src/client/intent.js` — `COMMAND_VOCAB`, `suggestCommands`, `quickCommandsFor`.
  - `src/client-app.js` — browser glue: `EventSource('/events')`, `fetch /state`
    + `POST /command`, renders pure models into the existing DOM ids; feed only from
    SSE (command echo local); tabs + Intent search/quick-commands; `API_BASE` via
    `VITE_OATHSTAR_API` (Tauri-ready) else same-origin.
  - `index.html` — module script repointed to `./src/client-app.js`.
  - `vite.config.js` — dev `server.proxy` for `/command`,`/state`,`/events` → `:7878`.
  - `tests/client.test.js` — T5–T9.
- **Compile/check as I went:** `cargo clippy -p oathstar-server --all-targets -- -D
  warnings` clean; `cargo test -p oathstar-server` 16/16 pass (incl. T1–T4);
  `node --check` on all JS; `node --test tests/client.test.js` 5/5 pass.
- **Deviations from design (+ reason):**
  - Header Save/Load/New buttons: wired to honest system notes (New clears the feed
    + refetches `/state`; Save/Load post "not wired yet") rather than left inert —
    no server save endpoint exists (out of scope). Trivial glue, not in the manifest.
  - Nearby renders exits as `actionCard`s with a single "Go" action (entities are not
    in the snapshot yet) — matches the design's "exits as navigable items".
  - REQ-001 proven by opening-capture + shared-serializer tests (T1–T4), not a
    socket-level SSE test — as the design called out (axum `Event` is opaque; avoids
    dev-deps; seed loop lives inside `stream!` so MSI is unaffected).

## Inspect (Phase 3.5)
- **Lenses run:** 4 parallel adversarial critics — correctness; security/secrets;
  data/state-integrity + gate-risk; simplification/reuse.
- **Findings:**
  | # | Severity | Finding (file:line) | Verdict | Fix |
  |---|---|---|---|---|
  | F1 | HIGH | rustfmt RED — over-long `assert!` lines fail gate:1 (`main.rs` tests) | REAL — gate blocker (the user's deliverable is `--fast` green) | `cargo fmt --all`; re-verified `fmt --check` clean |
  | F2 | HIGH | `toMapModel` keyed cells by `(x,y)` only → z-stacked tower rooms collapse; the **current** marker is dropped at the boss room (`map.js`) | REAL — verified vs `modules/beginner/rooms.toml` (bell_eater_roost 0,-3,2 over tower_landing 0,-3,1) | render the current room's **z-plane** only; added regression test T8b |
  | F3 | MED | EventSource auto-reconnect re-seeds the opening scene → duplicate feed entries (`main.rs` seed + `client-app.js`) | REAL | client-side `seenEventIds` dedup in `appendComponent` |
  | F4 | MED | post-move reconnect: feed shows start room while panels show current room | REAL — same root as F3 | resolved by F3 dedup (opening ids already seen on reconnect) |
  | F5 | MED | dead export `quickCommandsFor` unused (`intent.js:78`) | REAL | removed (also lifted intent.js coverage) |
  | F6 | LOW | `runCommand` duplicates `renderAll`'s 5-call dispatch (`client-app.js`) | REAL — DRY | replaced with `renderAll(latestSnapshot)` |
  | F7 | LOW | `event_to_json` doc overstates "identical bytes" (`main.rs`) | REAL — doc | reworded to "named test seam" |
  | F8 | LOW | `data-component` vocab differs: JSON feed (`type`) vs `/events/html` (`OutputComponent`) | REJECTED | `/events/html` is not consumed by this client; two intentional vocabularies — noted for a future Datastar path |
  | F9 | LOW | `MAP_RENDER_MODES` includes speculative `"sprite"` | REJECTED | REQ-007 explicitly anticipates a later sprite/canvas renderer; the reserved config seam is the requirement, not bloat |
  | F10 | LOW | over-broad exports (`toNearby/toGear/toPack/MAP_RENDER_MODES`) | REJECTED | used internally + enables targeted tests; harmless |
  | F11 | LOW | new modules carry more JSDoc than the prototype idiom | REJECTED | positive — DOM-free testable units benefit from param docs |
  | F12 | LOW | `app.js` unreferenced after the `index.html` repoint | REJECTED | expected mid-migration state; retained as the documented prototype reference (out of ticket scope to delete) |
  | F13 | LOW | `repeat(NaN,…)` grid if server sends non-numeric coords (`client-app.js`) | REJECTED | not exploitable (CSSOM rejects the value); coords are server-authoritative + validated — noted |
- **Security lens: CLEAN** — all server-derived strings render via `textContent`
  (grep: zero `innerHTML`/`insertAdjacentHTML`/`eval`); client consumes `/events`
  JSON not `/events/html`; gitleaks clean; `JSON.parse` guarded; loopback-only proxy.
- **Post-fix verification:** `cargo fmt --all --check` clean; `cargo clippy
  -p oathstar-server --all-targets -- -D warnings` clean; `cargo test
  -p oathstar-server` 16/16; `node --test tests/*.test.js` 10/10 (incl. T8b);
  JS line coverage 82.14% (≥75 floor); `node --check src/client-app.js` ok.
- **Forge capture:** failure `BF-map-zplane-001` (F2), `BF-gate-rustfmt-001` (F1);
  prevention rule `PR-claude-minimap-zplane-001`.

## Phase 4 — Validate
- **Tests added (written in Phase 3, run here):** Rust T1–T4 in
  `crates/oathstar-server/src/main.rs`; JS T5–T9 + T8b (z-plane regression) in
  `tests/client.test.js`.
- **`cargo test --workspace`:** PASS — all crates green, 0 failed (oathstar-server
  16/16 incl. T1–T4; core/content/protocol/storage all ok; doc-tests ok).
- **`node --test tests/*.test.js`:** PASS — 10/10 (6 client incl. z-plane, 4 prototype).
- **Live API smoke (real server on 127.0.0.1:7879):** `GET /events` on a fresh
  subscription streamed the opening scene (`room_entered` hollowmere_square →
  room header → narrative → exits) with NO prior command (REQ-001 end-to-end);
  `POST /command {"input":"swear"}` → `accepted:true` + typed `oath_sworn` events +
  camelCase snapshot (REQ-002); wire payloads snake_case under camelCase envelope
  (Decision 031, REQ-003). Event ids 1–4 (opening) then live 6+ — opening replays,
  a pre-subscription tick does not; no id collision.
- **`bin/gate.sh --fast`:** `GATE GREEN [fast]` — 14 passed, 0 failed
  (rustfmt, clippy strict, cargo test, node --test, cargo-audit, cargo-deny,
  cargo-machete, gitleaks, shellcheck, no-suppressions, source-bans,
  lints-allowlist, doc-todos, tauri-shell). Satisfies REQ-009.

### AC → verification traceability
| REQ | Verified by | Result |
|---|---|---|
| REQ-001 | Rust T1–T4 + live `/events` smoke (opening scene, no `look`) | PASS |
| REQ-002 | JS T7 (snapshot view models) + live `POST /command` smoke | PASS |
| REQ-003 | JS T5 (wire) + T6 (componentized descriptors, not a textarea) + wire smoke | PASS |
| REQ-004 | map-forward layout reused from `index.html`/`styles.css` (HUD/map/feed/menu/Intent) — review + smoke | PASS |
| REQ-005 | JS T7 (`toMenuModel`: Nearby/Oaths/Gear/Pack) + tabbed menu DOM | PASS |
| REQ-006 | JS T9 (`suggestCommands` filter + contextual) + Intent panel DOM; free text preserved | PASS |
| REQ-007 | JS T8 + T8b (`toMapModel` tile/mode passthrough + z-plane; server `MapSnapshot` shape unchanged) | PASS |
| REQ-008 | review — no React/Vue/Svelte; `package.json` deps unchanged (tauri-cli, esbuild, vite) | PASS |
| REQ-009 | `./bin/gate.sh --fast` GREEN (14/14) | PASS |

- **Pre-existing exclusions:** none — no pre-existing failures; all suites green.
- **FULL gate (15–17 coverage + mutation):** deferred to `/commit` per the user's
  `--fast` scope. Spot-checks during inspect: JS line coverage 82.14% (≥75 floor);
  new Rust `event_to_json` is mutation-covered (T2/T3) and the seed loops live inside
  the `stream!` macro (not mutated) — FULL is expected green when `/commit` runs it.

## Phase 5 — Complete
- **Docs updated:** `docs/decisions.md` Decision 032 (framework-free
  server-authoritative hypermedia client; Datastar library deferred);
  `docs/ui-design.md` Implementation Status section.
- **Forge capture (aar/failures/rules/decisions):** `aar-submit`
  (aar 30b3f1ce, outcome completed, effectiveness 4, 12 verdicts, 4 novel
  findings → distillation/confidence-drift/pattern-emergence enqueued); failures
  `BF-map-zplane-001` (high) + `BF-gate-rustfmt-001` (medium); prevention rule
  `PR-claude-minimap-zplane-001`; architecture decision `AD-claude-ui-shell-001`.
- **Ticket:** forge #12 → `in-review` (NOT done) + summary comment. FULL gate +
  `/commit` deferred per the requester's `--fast` scope; tree is ready to commit.
- **Archived:** pipeline pair moved to `docs/planning/pipeline/completed/`; local
  ticket doc kept in `tickets/open/` at status `in-review` (commit pending).
