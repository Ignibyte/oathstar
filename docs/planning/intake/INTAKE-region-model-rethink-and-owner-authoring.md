---
title: INTAKE-region-model-rethink-and-owner-authoring
status: candidate
created: 2026-06-17
ticket:
pipeline_spec:
---

# INTAKE-region-model-rethink-and-owner-authoring

> Owner direction (2026-06-17): drop the flat-colors map aesthetic, rethink the
> region model, build owner-facing region authoring ("let me create them"), and
> eventually **purge** the current hand-authored regions — but only AFTER the
> authoring tool + replacement content exist (the beginner/Hollowmere world is
> load-bearing for the game loop and ~30–50 tests). Continues the studio +
> world-model program (`INTAKE-studio-admin-and-world-model-program`, tickets
> #43–52). Supersedes the blank-colors map direction (Decisions 050/054/055) and
> shelves ticket #38 (see its forge comment, 2026-06-17).

## Problem / Opportunity

Regions today are **hand-authored TOML baked into the binary**, and the studio can
**validate but not persist** authored content — so the owner cannot actually
create a region and play it. Current state (file-anchored):

- **Model is thin + static.** `WorldDefinition` (rooms/regions/subregions/
  entities/items/oaths, `oathstar-core/src/lib.rs:22`) is assembled from
  `modules/beginner/*.toml` embedded via `include_str!` and loaded once at startup
  (`oathstar-content/src/lib.rs:126`). `RegionDefinition` is **id + name only**
  (`:75`); `SubregionDefinition` is id + name + parent region (`:83`).
- **The studio can author, but nothing sticks.** There is already a real authoring
  pipeline — `MapDocument` (terrain, rooms, regions, subregions, tilesets, layers;
  `oathstar-content/src/map_document.rs:245`) → `materialize()` →
  `WorldDefinition` via `POST /editor/maps/validate` (`oathstar-studio/src/editor.rs:54`).
  **But the document is discarded after validation — NO persistence, and NO
  region/sub-region CRUD UI** (the `/regions` dashboard is read-only,
  `sections.rs:37`). Server and studio are separate processes over separate data;
  the studio cannot write back.
- **Flat-colors is being retired.** The blank-colors fallback (`MAP_PALETTE`,
  `src/client/canvas-map.js:15`) was the map's look; the project moved to the
  arctic author sheet (`public/tilesets/arctic.json`, only 4 named tiles). The
  map-visual story for authored regions is unsettled.
- **The current world can't just be deleted.** ~30–50 Rust/JS tests load the
  beginner fixtures and the whole game loop runs on it — purging now reddens the gate.

## Proposed Outcome

The owner can **create a region in the studio, persist it, and play it**, and the
hand-authored beginner world is eventually replaced and purged — without ever
leaving the gate red. Concretely:

1. A **persistence layer** so authored maps/worlds survive (studio writes → game
   loads authored content, not just baked TOML).
2. **Region/sub-region authoring** in the studio (create/edit/delete, plus the
   room/terrain editing that already exists), built on the existing `MapDocument`
   → materialize pipeline.
3. A settled **region model** (keep World→Region→Subregion→Room; decide which
   attributes regions/subregions gain — standing defaults, danger, a visual key —
   and the subregion = zone-vs-floor question).
4. A **map-visual approach** replacing flat colors (per-map author tilesets / the
   arctic sheet), with the flat-color fallback retired deliberately.
5. A **content-reset path**: author replacement region(s), migrate tests to a
   minimal synthetic fixture, then purge the beginner world LAST.

## Open Design Questions (the "rethink")

- **Persistence shape:** studio writes `modules/<id>/*.toml` (reuse the existing
  format + loader) vs. persists `MapDocument` JSON vs. a new store/DB. How does the
  running game pick up authored content (restart-load vs. hot reload)? Where do
  authored files live, and who may write them (studio-only, loopback)?
- **Model enrichment:** does `RegionDefinition` gain attributes now (standing
  defaults per `docs/region-standing.md`, danger, biome/visual key) or stay
  id+name? Is a sub-region a spatial zone or a map/floor? (Awareness/proximity in
  `awareness.rs` gates on region+subregion co-membership — changes ripple.)
- **Authoring surface:** a dedicated region/sub-region CRUD UI on `/regions`, or
  author everything through the map-document editor (which already holds regions
  inline)? What is the minimum that lets the owner "create a region"?
- **Map visuals:** per-map author tileset (the studio already registers tilesets)
  vs. a shared sheet; what replaces the flat-color fallback, and is a fallback
  still needed for robustness?
- **Content reset:** what replaces Hollowmere (author a new starter region in the
  studio?), and what minimal fixture keeps the engine/content tests green when the
  beginner world is purged?
- **Authoring coverage:** does owner-authoring cover only the spatial map
  (regions/subregions/rooms/terrain) or also entities/items/oaths/combat
  placements (the full module)? The studio authors maps today, NOT NPCs/items/
  oaths — "rebuild Hollowmere through the UI" needs this settled (likely a later
  slice; S1/S2 can ship map+region authoring first and leave entities/oaths as
  TOML until an authoring surface exists for them).

## Proposed Direction (for steering)

Keep the existing **World → Region → Subregion → Room** model (sound and
load-bearing); the real gap is the **authoring LOOP** (persist + CRUD + game loads
it), not the data shape. Suggested sequence — purge LAST, per owner:

- **S1 — Persistence + load authored content** (keystone): the studio saves an
  authored world/map; the game loads authored content. Likely reuse the TOML
  module format + existing loader so nothing downstream changes.
- **S2 — Region/sub-region authoring UI**: extend the #51 read-only dashboard to
  create/edit/delete, leaning on the `MapDocument` materialize path.
- **S3 — Map visuals**: retire `MAP_PALETTE` flat-colors; settle author-tileset
  rendering for authored maps.
- **S4 — Model enrichment**: add only the attributes a real region needs (e.g.
  standing defaults), amending the region decisions.
- **S5 — Content reset**: author a replacement starter region; add a minimal test
  fixture; **then** purge `modules/beginner` and retire the blank-colors decisions.

## Scope Notes

- In: persistence for authored worlds/maps; studio region/sub-region authoring;
  map-visual replacement for flat-colors; region-model enrichment; a content-reset
  + test-fixture plan (purge last).
- Out (for now): multiplayer / online-first (separate stashed WIP); the
  region-standing consequence system itself (only its defaults may be authored);
  external `.tmx` import (#39) unless it falls out naturally.

## Notes

- **Owner confirmation (2026-06-17):** keep the World→Region→Subregion→Room model;
  drop the solid/flat tiles (paint with real tiles); author regions through the
  studio UI; **rebuild the current Hollowmere content via the tool**, then purge
  the hand-authored version (purge last).
- Supersedes the blank-colors map direction; shelves #38 (forge comment
  2026-06-17). Continues `INTAKE-studio-admin-and-world-model-program`.
- Blast radius to respect when the model changes: awareness/proximity
  (`awareness.rs` `Position{region,subregion,x,y,z}` / `same_plane`), warps (#52),
  `Engine::map_snapshot` (`lib.rs:3390`), saves (embed `WorldDefinition`),
  region-scoped announcements, future region-standing.
- Purge is LAST and gated on replacement + a minimal test fixture (~30–50 tests
  load `modules/beginner`).

## Promotion Status

This is a **living program intake** (S1–S5); it is promoted slice-by-slice and
its top-level `status:` stays `candidate` until the program completes.

- **S1 — PROMOTED (2026-06-17):** forge #53 (`9d39d561-de36-494a-93b5-cb2b7ce81698`),
  pipeline `docs/planning/pipeline/active/WORK-persist-authored-worlds-v1.spec.md`,
  ticket doc `docs/planning/tickets/open/TICKET-53-persist-authored-worlds.md`.
- **S2–S5:** not yet promoted (region/sub-region CRUD UI; map visuals; model
  enrichment; content reset + purge).
