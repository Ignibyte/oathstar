# WORK-studio-runtime-assets-v1 — Notes

## Phase 1 — Plan
- **Request:** pivot item ① — serve the studio's content PNGs from a runtime dir (not
  `include_bytes!`), so the owner edits the tileset/sprites without a rebuild. First slice of
  the studio-editable-world program (memory `studio-editable-world-pivot`).
- **Classification / tier:** work pipeline, one slice, **`oathstar-studio` only** — `main.rs`
  (`StudioState` + `resolve_assets_dir` + wiring) + `editor.rs`/`ui.rs` (handlers + shared
  `serve_png`) + tests + an AD. No game/engine change. New Rust logic → mutation surface.
- **Recon (working tree, main `c187818`):**
  - 3 stateless handlers: `editor::arctic_sheet` (`Bytes::from_static(ARCTIC_PNG)`, `editor.rs:258`),
    `ui::panel_frame`/`ui::button` (`PANEL_FRAME_PNG`/`BUTTON_PNG`, `ui.rs:16/20`). The editor
    comment literally cites *"without a runtime asset dir (Decision 058)"* — this reverses that.
  - Routes `main.rs:71-73`. `StudioState` (`:30-36`) = sessions/owner_secret/catalog/maps.
  - `resolve_maps_dir` (`main.rs:93`, returns `String`, default `maps`) + its blank-as-unset test
    (`:103`) are the precedent for `resolve_assets_dir`.
  - The maps store + the game client tileset (Vite `public/`) are already runtime; only the
    studio's PNGs are embedded.
- **Approach (design refines):** `resolve_assets_dir(Option<String>) -> PathBuf` (default
  `public`); `StudioState.assets_dir`; shared `serve_png(&Path, &str) -> Response`
  (`tokio::fs::read` → 200 image/png / logged 404); handlers → `State(studio)` + the fixed rel
  paths; drop the 3 `include_bytes`. `serve_png`/`resolve_assets_dir` location = design's call.
- **EARS:** REQ-001 resolver · REQ-002 serve_png 200 · REQ-003 serve_png 404 · REQ-004 the 3
  routes serve from disk · REQ-005 gate.
- **Mutation surface:** `resolve_assets_dir` (blank-filter/default) + `serve_png` (Ok→200/Err→404)
  — killed by REQ-001/002/003. Existing handler tests (ui.rs/editor.rs) get updated (signature +
  temp-dir fixtures) at Phase 4.
- **Ticket:** forge **#59** `16d0daf7-bbca-42f8-a270-7da9dde7bc5c`. Local doc
  `docs/planning/tickets/open/TICKET-59-studio-runtime-assets.md`.
- **aar_id:** `c23c0cc2-ea36-4544-bfc2-14001744af9d`
- **Delivery:** auto through validate; **PAUSE before push/PR/merge** (the /goal is complete).
  Branch off `main` `c187818`. Stash parked. AD `AD-claude-studio-runtime-content-assets-001` at complete.

## Phase 2 — Design

### Code reconnaissance
- **5 `StudioState` literals** gain `assets_dir`: `main.rs:50` (real) + the test helpers
  `handlers.rs:72`, `sections.rs:63`, `regions.rs:237`, `editor.rs:305` (the `regions.rs`
  `seeded_studio`/`seeded_studio_with_sub` and `editor.rs` `studio_with_maps_as_file` build on
  those, no extra literal). Adding a struct field is a compile error until all 5 are updated —
  caught immediately.
- The 3 handler tests are no-arg today: `ui.rs serves_the_panel_frame` (`:47`) / `serves_the_button`
  (`:65`), `editor.rs serves_the_arctic_sheet` (`:674`) — they call `panel_frame().await` etc. and
  assert content-type + non-empty body. They become `State(studio)`-based over a temp assets dir.
- `handlers.rs` test `studio()` already uses `ContentCatalog::default()` + a temp-dir placeholder
  `FileSaveStore` — so a **minimal** `StudioState` (for the ui.rs handler test) is cheap.

### Approach / architecture (oathstar-studio only)
- **NEW `crates/oathstar-studio/src/assets.rs`** (`mod assets;` in `main.rs`):
  - `pub(crate) fn resolve_assets_dir(raw: Option<String>) -> PathBuf` —
    `raw.filter(|d| !d.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("public"))`.
  - `pub(crate) async fn serve_png(assets_dir: &Path, rel: &str) -> Response` — `match
    tokio::fs::read(assets_dir.join(rel)).await { Ok(bytes) => ([(header::CONTENT_TYPE,
    "image/png")], bytes).into_response(), Err(error) => { eprintln!("oathstar-studio: asset {rel}
    unreadable under {} ({error})", assets_dir.display()); (StatusCode::NOT_FOUND, "asset not
    found").into_response() } }`. The `Vec<u8>` body is a valid axum body — `Bytes` no longer needed.
- **`StudioState.assets_dir: PathBuf`** (PathBuf is `Clone`); `main` sets it via
  `resolve_assets_dir(std::env::var("OATHSTAR_ASSETS_DIR").ok())`.
- **Rewire** `editor::arctic_sheet` / `ui::panel_frame` / `ui::button` → `async fn(State(studio):
  State<StudioState>) -> Response` calling `assets::serve_png(&studio.assets_dir, "<fixed rel>")`
  (`tilesets/arctic.png`, `ui/panel-frame.png`, `ui/button.png`). **Remove** the 3 `include_bytes!`
  consts + the now-unused `Bytes` imports.

### Locked decisions (this phase)
- **`assets.rs` module** (cohesive + the core logic is `StudioState`-free, so `serve_png`/
  `resolve_assets_dir` are directly unit-/mutation-tested). `assets_dir` is `PathBuf`.
- The 3 handlers are **thin wrappers** (`serve_png(&studio.assets_dir, rel)`) — the testable logic
  is `serve_png`; the handlers have no viable mutant (`Response` isn't `Default`; the rel literal
  isn't mutated). Handler tests are regression (the right rel path is served via `State`).
- **Missing `public/` at runtime** → the assets 404 (the palette/sprites just don't load) — degraded,
  not bricked. The AD records this trade.

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-studio/src/assets.rs` | **NEW** `resolve_assets_dir` + `serve_png` (+ their `#[cfg(test)]` tests). |
| 2 | `crates/oathstar-studio/src/main.rs` | `mod assets;`; `StudioState.assets_dir: PathBuf`; `main` sets it via `resolve_assets_dir(env OATHSTAR_ASSETS_DIR)`. |
| 3 | `crates/oathstar-studio/src/editor.rs` | `arctic_sheet` → `State(studio)` + `serve_png`; remove `ARCTIC_PNG`/`Bytes`; update `serves_the_arctic_sheet` (temp assets dir) + the `studio()` helper's `assets_dir`. |
| 4 | `crates/oathstar-studio/src/ui.rs` | `panel_frame`/`button` → `State(studio)` + `serve_png`; remove the 2 consts/`Bytes`; update the 2 handler tests (a minimal `StudioState` over a temp assets dir). |
| 5 | `crates/oathstar-studio/src/{handlers,sections,regions}.rs` | add `assets_dir: …` to each test `StudioState` literal (any value — these tests don't hit the asset routes). |
| 6 | `docs/decisions.md` + `docs/map-system.md` | (Phase 5) note the runtime-content-assets reversal of Decision 058. |

### Regression Test Plan
| # | Test | Proves |
|---|---|---|
| T1 | `resolve_assets_dir(None)` → `public`; `(Some(""))` → `public`; `(Some("custom"))` → `custom` | REQ-001 (assets.rs) |
| T2 | `serve_png(tmp, "x.png")` with a fixture written → `200`, `content-type: image/png`, body == the fixture bytes | REQ-002 (assets.rs, temp dir) |
| T3 | `serve_png(tmp, "missing.png")` → `404` (no panic) | REQ-003 (assets.rs) |
| T4 | `arctic_sheet`/`panel_frame`/`button` via `State(studio)` whose `assets_dir` is a temp dir with the fixtures → `200` + `image/png`; a `studio` with an empty assets dir → `404` | REQ-004 (editor.rs/ui.rs) |
| G1 | `bin/gate.sh` FULL green, MSI 100% | REQ-005 |
- Each test uses a **unique temp dir** (`std::env::temp_dir().join(format!("oathstar-studio-assets-{n}"))`,
  cleaned first), **never the real `public/`**. **Mutation:** `resolve_assets_dir` (blank-filter +
  default) + `serve_png` (`Ok`→200 / `Err`→404 arm) killed by T1–T3. No genuinely-uncoverable code
  (the `eprintln!` is a non-asserted side effect, not a mutants target — verified-pattern from #56).

### Risks / decisions
1. **Struct field fan-out** — 5 `StudioState` literals; a missed one is an immediate compile error.
2. **Runtime `public/` dependency** — the trade the owner accepts (editability > self-contained);
   missing → graceful 404, documented in the AD.
3. **No new dep** — uses `tokio::fs` (already in the workspace); avoids `tower-http ServeDir` + its
   audit/deny surface. Fixed rel paths → no traversal (no caller input reaches `join`).

## Phase 3 — Implement
- **Built (manifest as designed):** `assets.rs` (`resolve_assets_dir` + `serve_png`); `main.rs`
  (`mod assets;`, `StudioState.assets_dir`, env wiring); `editor.rs` `arctic_sheet(State)` →
  `serve_png(…,"tilesets/arctic.png")` (removed `ARCTIC_PNG`); `ui.rs` `panel_frame`/`button(State)`
  → `serve_png` (removed the 2 consts; doc updated); `assets_dir` added to the 4 test `StudioState`
  literals; the 3 handler tests rebuilt to `State` over temp fixtures asserting the **on-disk bytes**
  (`PNGARCTIC`/`PNGFRAME`/`PNGBUTTON`) — never the real `public/`.
- **Deviations (minor):** (1) **tokio `fs` feature** — workspace `tokio` lacks it; added
  `features = ["fs"]` scoped to `oathstar-studio/Cargo.toml` (no new crate; machete clean).
  (2) **`pub`, not `pub(crate)`** on the two `assets.rs` fns — clippy `redundant_pub_crate` (nursery)
  rejects `pub(crate)` inside a private `mod` (crate convention is plain `pub`, like
  `editor::arctic_sheet`). (3) Removed the now-unused top-level `header` import in `editor.rs`;
  `ui.rs` dropped `Bytes`/`header`/`IntoResponse`.
- **Checks:** `cargo check`/`clippy -p oathstar-studio --all-targets` clean; `cargo fmt` clean;
  `cargo machete` clean; `cargo test -p oathstar-studio` → **83 passed** (incl. the 3 asset
  handlers). Full suites + gate at Phase 4.

## Inspect (Phase 3.5)
- **Lenses:** 2 read-only `Explore` critics (no worktree mutation — PR-claude-inspect-critic-read-only-001):
  correctness/security, and simplification/test-integrity.
- **Critic 1 (correctness/security) — CLEAN.** Verified: (a) **no traversal surface** — `serve_png`'s
  `rel` is always a fixed caller literal; the 3 routes (`main.rs:78-80`) take only `State`, no
  `Path`/`Query`, so request input never reaches `assets_dir.join`; (b) `serve_png` is panic-free —
  `match` on `tokio::fs::read`, `Err`→logged 404 (no unwrap/expect on the fs path); (c)
  content-type is the immutable `"image/png"`; (d) `resolve_assets_dir` correct (None/blank→`public`,
  set→value); (e) startup-safe — `assets_dir` is a path, read lazily per request, so a missing
  `public/` 404s rather than crashing; (f) `StudioState` derives `Clone`, `PathBuf: Clone`.
- **Findings (Critic 2):**
  - **[low] arctic test used a static temp-dir name** (`editor.rs` `serves_the_arctic_sheet`) while
    `studio()` uses an atomic seq. **Verdict: safe today** (cargo-mutants runs **serial** — no `jobs`
    in `.cargo/mutants.toml` — so no cross-process collision, and it's a dedicated dir cleaned
    first), but **FIXED anyway** for uniformity/future-proofing: added an `AtomicU32` seq
    (`oathstar-studio-arctic-asset-{seq}`) matching the `studio()` norm. Re-verified: clippy clean +
    the test passes.
  - **[med] `resolve_assets_dir` returns `PathBuf` vs `resolve_maps_dir`'s `String`** — **REJECTED
    (by design):** `serve_png` needs `&Path`/`.join`; `FileSaveStore` takes `impl Into<PathBuf>`. The
    critic itself marked it CLEAN. No change.
  - **[high] `assets.rs` has no `#[cfg(test)]`** — **NOT a Phase 3 defect; the known Phase 4 gap.**
    The 3 handler tests cover only `serve_png`'s 200 path. **Phase 4 MUST add** T1 (`resolve_assets_dir`
    default/blank/override) + T3 (`serve_png` missing→404) so mutation/coverage are satisfied —
    already in the Regression Test Plan.
- **Also confirmed clean:** no dead refs to the removed `ARCTIC_PNG`/`PANEL_FRAME_PNG`/`BUTTON_PNG`
  (`Bytes` still legitimately used by `editor::save_map`/`validate`); all 5 `StudioState` literals
  carry `assets_dir`; the `tokio fs` feature is minimal/scoped (no new crate); no fixture touches the
  real `public/`. No `failure-record` (the [low] was a robustness tidy, not a defect).

## Phase 4 — Validate
- **Tests added** — a new `#[cfg(test)] mod tests` in `assets.rs` (closes the inspect-flagged gap):
  - **T1** `resolve_assets_dir_treats_blank_as_unset` (REQ-001) — `None`/blank→`public`, set→value.
  - **T2** `serve_png_serves_an_existing_file_as_image_png` (REQ-002) — fixture in a unique temp dir
    → `200` + `image/png` + exact bytes.
  - **T3** `serve_png_404s_a_missing_file` (REQ-003) — missing file → `404` (no panic).
  - (T4 — the 3 handler tests — already landed in Phase 3.) `fresh_dir()` uses an atomic seq, cleans
    first, never touches the real `public/`.
- **`cargo test --workspace`:** green — `oathstar-studio` **86 passed** (incl. the 3 new
  `assets::tests`); all other crates green (auth 20, content 113, core 300, datastar 16, protocol 27,
  server 40, storage 23).
- **`node --test tests/*.test.js`:** **88 passed / 0 fail** (JS untouched).
- **`bin/gate.sh` FULL:** **GATE GREEN [full]** — 17/17. rustfmt (one fmt pass on the new test block),
  clippy strict, both suites, rust cov ≥94, js cov 89.99%, **mutation 593 caught / 0 missed → MSI
  100.0%**. Receipt written (`.git/oathstar-gate-receipt`).
- **Pre-existing exclusions:** none.

## Phase 5 — Complete
- Docs / forge / ticket / archived:
