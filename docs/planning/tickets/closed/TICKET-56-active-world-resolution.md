# TICKET-56 — Game loads the active authored map on reload (no manual OATHSTAR_WORLD)

- **Forge ID:** `991e92fb-8406-42bf-a686-6e3676c05e54` (#56)
- **Type:** feature · **Status:** open (pipeline `WORK-active-world-resolution-v1`)
- **Program:** item ① of the owner's 2026-06-19 authoring-loop plan (memory `studio-authoring-next-phase`)

## Why
The save→play loop's second half. Saving a map (#55) persists it, and the game *can* load an
authored map — but only if you point it there with `OATHSTAR_WORLD=<path>`. This removes the
manual step: save the map as the active slot, restart, and the game plays it.

## What
Server-side **active-world convention**. `resolve_world_path(OATHSTAR_WORLD, maps_dir)` →
`Explicit(path)` (wins) → `ActiveSlot(maps_dir/world.json)` → `Beginner`; `load_world(source)`
materializes a valid authored doc (the format already aligns via #53), keeps an invalid explicit
world a loud error, and falls back to beginner (logged) for an invalid active slot so a draft
doesn't brick startup. The server reads `OATHSTAR_MAPS_DIR` (the studio's save dir).

## Acceptance
See `docs/planning/pipeline/active/WORK-active-world-resolution-v1.spec.md` (EARS REQ-001..006).

## Safety
`maps/` gitignored; all tests use temp dirs / the baked beginner (never the real `maps/`).

## Out of scope
A studio "Set as active" UI (follow-on); hot-swap/live reload; entities/items/oaths authoring;
item ③ regions table; item ④ marquee paint.
