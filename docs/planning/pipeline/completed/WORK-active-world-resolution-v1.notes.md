# WORK-active-world-resolution-v1 — Notes

## Phase 1 — Plan
- **Request:** item ① — the game picks up a saved authored map on reload (active-world
  convention), no manual `OATHSTAR_WORLD`. Memory `studio-authoring-next-phase` item ①.
- **Classification / tier:** work pipeline, one slice, **server-side Rust** (`oathstar-server`
  `main.rs`) + `.gitignore` + tests. New logic (two resolver/loader units) → mutation surface.
- **Recon (working tree):**
  - **Format aligns (#53):** `load_startup_world(Some(path))` (`oathstar-content/src/lib.rs:291`)
    → `load_authored_world` → `MapDocument::materialize()` reads the studio's saved
    `MapDocument` JSON directly (test fixture `AUTHORED_DOC`, lib.rs:318). No conversion.
  - Server resolution today (`oathstar-server/main.rs:98-99`):
    `authored_world_path(OATHSTAR_WORLD)` → `load_startup_world(...)?`. `authored_world_path`
    (`:371`) is the blank-==-unset helper. Server uses `OATHSTAR_SAVE_DIR` (saves) but **not**
    `OATHSTAR_MAPS_DIR` yet — this slice adds it.
  - `maps/` is **not** gitignored (only `saves/`, `.gitignore:17`).
- **Approach (for design):** `resolve_world_path(OATHSTAR_WORLD, maps_dir) -> WorldSource`
  (Explicit→ActiveSlot `maps_dir/world.json`→Beginner) + `load_world(source)` (explicit invalid
  = loud `Err`; active-slot invalid = warn + beginner; valid = materialize). `main` composes
  them (excluded from mutants); both helpers unit-/mutation-tested with temp dirs + fixtures.
- **EARS:** REQ-001..003 resolver precedence; REQ-004 `load_world` (valid/invalid-active/
  invalid-explicit/beginner); REQ-005 gitignore `maps/`; REQ-006 gate.
- **Ticket:** forge **#56** `991e92fb-8406-42bf-a686-6e3676c05e54` (NOT #55).
  Local doc `docs/planning/tickets/open/TICKET-56-active-world-resolution.md`.
- **aar_id:** `234cf75d-6ee9-4022-b84d-b890055cb9ed`
- **Delivery:** goal-driven autonomous — plan→complete then commit + push + FF-merge to `main`.
  Branch off `main` `7768403`. Stash parked. Slot name `world`; "Set as active" UI is a follow-on.

## Phase 2 — Design

### Code reconnaissance
- `main.rs` imports `std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration}`
  — needs `std::path::{Path, PathBuf}` added. `WorldDefinition` referenced as
  `oathstar_content::WorldDefinition` (call site already uses `oathstar_content::load_startup_world`).
- Today (`main.rs:98-99`): `authored_world_path(OATHSTAR_WORLD)` → `load_startup_world(...)?`.
  `authored_world_path` (`:371`) + its test are **superseded** by `resolve_world_path` — remove them.
- Test discriminators (in `oathstar-content` tests, replicated into the server test):
  beginner `start_room_id == "hollowmere_square"`; a valid authored doc (`AUTHORED_DOC`, spawn
  (0,0,0) → room `alpha`) → `"alpha"`; `UNMATERIALIZABLE_DOC` (`tile_size:7`) parses but fails
  `materialize()`.

### Approach / architecture (server-side Rust)
1. **`WorldSource` enum** `{ Explicit(PathBuf), ActiveSlot(PathBuf), Beginner }` (in `main.rs`).
2. **`resolve_world_path(oathstar_world: Option<String>, maps_dir: &Path) -> WorldSource`** —
   `oathstar_world.filter(|p| !p.is_empty())` → `Explicit` (wins); else
   `maps_dir.join(ACTIVE_WORLD_FILE)` where `ACTIVE_WORLD_FILE = "world.json"` — `.is_file()`
   → `ActiveSlot`; else `Beginner`. Pure logic + one `is_file` fs check (temp-dir testable).
3. **`load_world(source) -> anyhow::Result<oathstar_content::WorldDefinition>`** reusing
   `load_startup_world`: `Explicit(p)` → `load_startup_world(Some(&p))` (**loud `Err`** on
   invalid, unchanged posture); `ActiveSlot(p)` → `match load_startup_world(Some(&p))` `Ok→Ok`,
   `Err(e)→ { eprintln!("…active world {p:?} failed to load ({e}); using the baked beginner
   world"); load_startup_world(None) }` (**best-effort** — a draft can't brick startup);
   `Beginner` → `load_startup_world(None)`.
4. **`main` composes** (excluded from mutants): `let maps_dir = env("OATHSTAR_MAPS_DIR")
   .unwrap_or_else(|_| "maps".into()); let world = load_world(resolve_world_path(env("OATHSTAR_WORLD").ok(),
   Path::new(&maps_dir)))?;`
5. **`.gitignore`** gains `maps/`.

### Locked decisions (this phase)
- `WorldSource` **enum** (not `Option`+flag) — the three startup outcomes are distinct and the
  match in `load_world` is exhaustive + mutation-legible.
- **`load_world` lives in `oathstar-server`** (server startup policy: the env vars + the slot
  convention). It only *reuses* `oathstar_content::load_startup_world` (the load primitive).
- **`eprintln!`** for the active-slot warning (matches the owner-password warning idiom,
  `main.rs:42`). Risk noted below.
- Active slot file = **`world.json`** (const `ACTIVE_WORLD_FILE`).

### File manifest
| # | File | Change |
|---|---|---|
| 1 | `crates/oathstar-server/src/main.rs` | Add `std::path::{Path,PathBuf}`; `ACTIVE_WORLD_FILE` const; `WorldSource` enum; `resolve_world_path`; `load_world`; `main` reads `OATHSTAR_MAPS_DIR` (default `maps`) + composes; **remove** `authored_world_path` + its test (superseded). |
| 2 | `.gitignore` | Add `maps/`. |
| 3 | `crates/oathstar-server/src/main.rs` (`#[cfg(test)]`) | resolver precedence + `load_world` tests (temp dirs + `AUTHORED`/`UNMATERIALIZABLE` fixtures). |
| 4 | `docs/map-system.md` | (Phase 5) note the active-world resolution chain. |

### Regression Test Plan
| # | Test | Proves |
|---|---|---|
| T1 | `resolve_world_path(Some("explicit.json"), tmp)` → `Explicit("explicit.json")` even when `tmp/world.json` exists (explicit wins) | REQ-001 |
| T2 | `resolve_world_path(Some(""), tmp+world.json)` → `ActiveSlot`; `resolve_world_path(None, tmp+world.json)` → `ActiveSlot(tmp/world.json)` | REQ-001/002 |
| T3 | `resolve_world_path(None, tmp-empty)` → `Beginner`; `resolve_world_path(None, tmp where world.json is a DIR)` → `Beginner` (`is_file` false) | REQ-003 |
| T4 | `load_world(ActiveSlot(valid AUTHORED))` → start `"alpha"`; `load_world(ActiveSlot(UNMATERIALIZABLE))` → `"hollowmere_square"` (fallback); `load_world(Explicit(valid))` → `"alpha"`; `load_world(Explicit(UNMATERIALIZABLE))` → `Err`; `load_world(Beginner)` → `"hollowmere_square"` | REQ-004 |
| T5 | `git check-ignore maps/` succeeds | REQ-005 (validate shell check) |
| G1 | `bin/gate.sh` FULL green, MSI 100% | REQ-006 |
- Each test uses a **unique temp dir** (`std::env::temp_dir().join(format!("oathstar-server-aw-{n}"))`,
  cleaned first) — **never the real `maps/`**; deterministic.
- **Mutation:** `resolve_world_path` (precedence branches, `is_file`, blank-filter, the join name)
  + `load_world` (match arms, the `Err`→fallback) are all killed by T1–T4. `main` is excluded.
- **Genuinely uncoverable / risk:** the `eprintln!` in `load_world`'s `ActiveSlot` `Err` arm is a
  side effect T4 doesn't assert; if cargo-mutants deletes it the mutant could survive (a bare
  macro-statement is usually not a mutants target, so likely fine). If it surfaces at validate,
  fix by making the fallback observable (return the resolved source / capture) — see risks.

### Risks / decisions
1. **`eprintln!` mutation** (above) — accept + watch the gate; restructure only if a survivor appears.
2. **Shared `maps/` dir** — studio + game both default to `maps/` relative to cwd, so dev runs
   share it; documented. (Different cwd/deploy → set `OATHSTAR_MAPS_DIR` on both.)
3. **Draft safety** — `ActiveSlot` best-effort means saving a broken draft as `world` logs +
   plays beginner rather than bricking; explicit `OATHSTAR_WORLD` stays loud (opt-in).
4. **No determinism risk** — tests never touch real `maps/`; `maps/` git-ignored.

## Phase 3 — Implement
- **Built to the manifest** (tests Phase 4):
  - `oathstar-server/src/main.rs` — `use std::path::{Path, PathBuf}`; `const
    ACTIVE_WORLD_FILE = "world.json"`; `enum WorldSource { Explicit(PathBuf),
    ActiveSlot(PathBuf), Beginner }` (derives Debug/PartialEq/Eq for tests);
    `resolve_world_path(oathstar_world, maps_dir)` (blank-filter → Explicit wins; else
    `maps_dir.join(ACTIVE_WORLD_FILE).is_file()` → ActiveSlot; else Beginner);
    `load_world(source)` (Explicit → loud; ActiveSlot → `Ok`/`Err`→warn+beginner; Beginner →
    beginner). `main` reads `OATHSTAR_MAPS_DIR` (default `maps`) and composes
    `load_world(resolve_world_path(env("OATHSTAR_WORLD"), Path::new(&maps_dir)))?`. **Removed**
    `authored_world_path` + its test (superseded).
  - `.gitignore` — `maps/` (with a comment; `git check-ignore maps/` ✓).
- **Verified:** `cargo fmt`; `clippy -p oathstar-server --all-targets` **clean**; server tests
  compile.
- **Deviation from design:** the active-slot warning uses `path.display()` (not `{path:?}`) —
  clippy `unnecessary_debug_formatting` flags `{:?}` on a `Path`; `.display()` is the idiom.
- **For Phase 4:** resolver precedence tests (T1–T3, temp dirs incl. dir-not-file) + `load_world`
  (T4: valid authored→`alpha`, garbage active→`hollowmere_square`, invalid explicit→`Err`,
  beginner) + the `git check-ignore maps/` check (T5). Watch the active-slot `eprintln!` for a
  mutation survivor (noted risk).

## Inspect (Phase 3.5)
- **Lenses run** (2 parallel **read-only `Explore`** critics — `PR-claude-inspect-critic-read-only-001`):
  **correctness + mutation**, **security + integrity**.
- **Findings:**
  | # | Sev | Finding | Verdict |
  |---|---|---|---|
  | 1 | med | the active-slot `eprintln!` is an unobserved side effect → a possible mutation survivor | **REJECTED — verified false** |
- **Finding 1 rejected with evidence:** `cargo mutants --list -f …/main.rs` generates only three
  mutants for the new code — `resolve_world_path` body→`Default::default()` (**unviable**:
  `WorldSource` has no `Default`), the **`!`-deletion** in the blank filter (`398:53`, killed by
  T1), and `load_world` body→`Ok(Default::default())` (`414:5`, **viable** since
  `WorldDefinition: Default` (`lib.rs:67`) → killed by T4 `Beginner`→`hollowmere_square` ≠ default).
  **No mutant targets the `eprintln!`** — cargo-mutants does not mutate the bare side-effect
  statement. So no fix; the warning stays.
- **Cleared (critics' concrete checks):**
  - **Precedence:** explicit `OATHSTAR_WORLD` wins via the early `return` even when `maps/world.json`
    exists; blank `""` filters out → falls through to the active-slot check.
  - **`is_file` safety:** a `world.json` that's a dir / dangling symlink / missing → `false` → `Beginner`,
    no panic; no attempt to load a non-file.
  - **Error scope:** only the `ActiveSlot` arm catches `Err`→beginner; an `Explicit` error propagates
    (loud). No `unwrap`/`expect`/indexing in either new fn.
  - **Untrusted load:** `load_startup_world`→`load_authored_world` returns typed `AuthoredWorldError::{Read,Parse,Materialize}`; the `Err(_)` catch covers **all** modes → garbage `world.json` can't crash the server.
  - **Path:** fixed `ACTIVE_WORLD_FILE = "world.json"` const (no user path component / traversal); symlink-escape acceptable under Decision 058 (single-user loopback).
  - **`.gitignore maps/`:** nothing tracked under `maps/` (so nothing newly hidden); no SAST/secrets.
  - **Determinism:** resolution runs only in `main`; tests use temp dirs / beginner — a local `maps/world.json` can't perturb the gate. `authored_world_path` removal has zero dangling refs; server tests (34) pass.
- **Re-verified:** worktree = the 2 expected files (no clobber); clippy clean.
- **Capture:** no `failure-record` (no bug); no new rule. (The "verify a flagged mutation-survivor
  with `cargo mutants --list` before restructuring" practice is already implied by the existing
  inspect discipline.)

## Phase 4 — Validate
- **Tests added (+6, `oathstar-server` `#[cfg(test)]`):** `AUTHORED`/`UNMATERIALIZABLE` fixtures
  + a `fresh_dir(tag)` unique-temp-dir helper (never the real `maps/`):
  - T1 `resolve_world_path_explicit_wins_even_with_an_active_slot` (REQ-001).
  - T2 `resolve_world_path_blank_or_unset_uses_the_active_slot` (REQ-001/002).
  - T3 `resolve_world_path_no_file_active_slot_is_beginner` (no slot + slot-is-a-dir → Beginner, REQ-003).
  - T4a `load_world_loads_authored_for_a_valid_active_slot_or_explicit` → `alpha`.
  - T4b `load_world_active_slot_falls_back_but_explicit_errs_on_invalid` (active→`hollowmere_square`,
    explicit→`Err`) — kills the `load_world`→`Ok(Default::default())` mutant.
  - T4c `load_world_beginner_loads_the_baked_world` → `hollowmere_square`.
- **`cargo test --workspace`:** GREEN — `oathstar-server` **40** (+6), all crates pass.
- **`git check-ignore maps/`:** `maps/` **IGNORED** ✓ (REQ-005).
- **`node --test tests/*.test.js`:** GREEN — 84 pass (regression; no JS change this slice).
- **`bin/gate.sh` (FULL):** **GATE GREEN — 17/17, mutation 590 caught / 0 missed → MSI 100.0%**
  (the new `!`-deletion + `load_world` Default-replace mutants killed; the `eprintln!` is not a
  mutant, as `--list` showed at inspect). Rust + JS coverage held. Commit-gate receipt written.
- **Pre-existing exclusions:** none.

## Phase 5 — Complete
- **Docs:** `docs/map-system.md` — new "Loaded by the game (#53, #56)" paragraph (the
  `OATHSTAR_WORLD` → active slot `maps/world.json` → beginner chain; loud-explicit /
  best-effort-active; `maps/` gitignored; the loop is closed) + the stale "loading is next"
  note updated.
- **Forge:** `aar-submit` (AAR `234cf75d`, completed, score 5; reused recon-before-build +
  read-only-critic rules); **`PR-claude-verify-mutation-survivor-with-list-001`** (verify a
  flagged mutation survivor with `cargo mutants --list` before restructuring — saved an
  unnecessary API change here); no `failure-record` (inspect clean, the one finding was a
  verified false positive).
- **Ticket:** forge **#56 CLOSED (done)**.
- **Archived:** `…/completed/WORK-active-world-resolution-v1.{spec,notes}.md`.
