---
pipeline_id: b4453dc4-c714-4597-99fe-229faf73ce75
title: WORK-active-world-resolution-v1
ticket: 991e92fb-8406-42bf-a686-6e3676c05e54
type: work
intake: docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md
notes: WORK-active-world-resolution-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-active-world-resolution-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`.

## Work Spec
- **Title:** The game loads the **active authored map** on reload (item ① — the save→play
  loop's "it shows up" half), no manual `OATHSTAR_WORLD`.
- **The finding that shapes this slice:** the **format already aligns** (S1 #53) —
  `load_startup_world(Some(path))` → `load_authored_world` → `MapDocument::materialize()`
  reads the studio's saved `MapDocument` JSON directly. Today you must point the game at it
  (`OATHSTAR_WORLD=<path>`). This slice is the **"active world" convention** only.
- **Scope:**
  - **In (server-side):**
    1. A pure-ish `resolve_world_path(oathstar_world, maps_dir)` returning a `WorldSource`:
       **`Explicit(path)`** when `OATHSTAR_WORLD` is set (blank == unset, wins) → else
       **`ActiveSlot(maps_dir/world.json)`** when that file exists → else **`Beginner`**.
    2. The server reads **`OATHSTAR_MAPS_DIR`** (default `maps`), the **same dir the studio
       saves to** — so saving a map as the active slot (`#55` Save, id `world`) + restarting
       loads it.
    3. A `load_world(source)`: `Explicit` invalid → **loud startup error** (unchanged posture);
       `ActiveSlot` invalid → **logged + fall back to the baked beginner** (a saved *draft*
       must not brick startup — `#55` Save allows non-materializable drafts); `Beginner` →
       baked beginner. A valid `Explicit`/`ActiveSlot` materializes into its authored world.
    4. **gitignore `maps/`** (authored experiments never get committed / never reach the gate).
  - **Out (explicit):** a studio **"Set as active"** UI (follow-on — activate by saving as
    the slot for now); **hot-swap / live reload** (restart-only); changing `OATHSTAR_WORLD`'s
    explicit loud-error; entities/items/oaths authoring (stays TOML); item ③ regions table;
    item ④ marquee paint.
- **Systems:** game server (`oathstar-server` `main.rs` — `resolve_world_path` + `load_world`,
  reusing `oathstar_content::load_startup_world`) + `.gitignore` + tests. **Rust, server-side.**

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When `OATHSTAR_WORLD` is set (non-blank), `resolve_world_path` shall return `Explicit(path)` regardless of any active slot present. | cargo test (temp maps dir with a `world.json` present; explicit set → `Explicit`) |
| REQ-002 | When `OATHSTAR_WORLD` is unset/blank and `maps_dir/world.json` is a file, `resolve_world_path` shall return `ActiveSlot(that path)`. | cargo test (temp dir + file) |
| REQ-003 | When `OATHSTAR_WORLD` is unset/blank and no `maps_dir/world.json` file exists, `resolve_world_path` shall return `Beginner`. | cargo test (temp empty dir; and a dir-not-file case) |
| REQ-004 | `load_world` shall materialize a valid `Explicit`/`ActiveSlot` document into its authored world, load the baked beginner for `Beginner`, return an error for an invalid `Explicit`, and log + fall back to beginner for an invalid `ActiveSlot`. | cargo test (temp fixtures: valid authored → its start room; garbage active → beginner start room; invalid explicit → `Err`) |
| REQ-005 | The maps store directory (`maps/`) shall be git-ignored. | `git check-ignore maps/` (verified at validate) |
| REQ-006 | The full gate shall stay green with mutation at 100% MSI. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **Resolution chain:** explicit `OATHSTAR_WORLD` (wins) → conventional active slot
  `OATHSTAR_MAPS_DIR/world.json` → baked beginner. The active **slot name is `world`**
  (a fixed convention this slice; a studio "Set as active" UI is the follow-on).
- **Best-effort active slot, loud explicit:** an invalid *active slot* logs + falls back to
  beginner (don't brick on a draft); an invalid *explicit* `OATHSTAR_WORLD` stays a loud
  startup error (you asked for it). Two units (`resolve_world_path` + `load_world`) so both
  are unit- + mutation-testable; `main` only composes them (excluded from mutants).
- **The server shares `OATHSTAR_MAPS_DIR`** (default `maps`) with the studio — same filesystem
  dir, so studio-save and game-load meet without a copy step.
- **Determinism / safety:** every test uses **temp dirs or the baked beginner** — never the
  real `maps/`. `maps/` is **git-ignored** so authored content can't be committed or perturb
  the gate. Load stays **untrusted input** (typed errors, re-validate via `materialize`, no
  panics — the #53 posture).
- **Branch off `main`** (`7768403`); stash (`stash@{0}`) stays parked.
- **Design (Phase 2) decides:** the `WorldSource` representation (enum vs `Option`+flag), the
  exact warn message + log channel, and whether `load_world` lives in `oathstar-server` (server
  policy — leaning yes) or `oathstar-content`.

## Linked Artifacts
- Design docs: `docs/map-system.md` (authored worlds), `docs/module-system.md` (load boundary),
  `docs/decisions.md`. Design re-reads.
- Intake / plan: `docs/planning/intake/INTAKE-region-model-rethink-and-owner-authoring.md`;
  memory `studio-authoring-next-phase` (item ①). Builds on S1 #53 (`load_authored_world` +
  `materialize`), #55 (the Save control).
- Ticket doc: `docs/planning/tickets/open/TICKET-56-active-world-resolution.md`
- Forge ticket: `991e92fb-8406-42bf-a686-6e3676c05e54` (#56).

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
