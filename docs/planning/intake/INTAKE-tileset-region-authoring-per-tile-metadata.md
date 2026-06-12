---
title: INTAKE-tileset-region-authoring-per-tile-metadata
status: candidate
created: 2026-06-11
ticket:
pipeline_spec:
---

# INTAKE-tileset-region-authoring-per-tile-metadata

## Problem / Opportunity

Owner direction (2026-06-11, alongside ticket #32): the tileset `.tsx`
should grow into an **authoring surface for the world**, not just a sprite
atlas. Two gaps in the current generated metadata:

1. **Per-tile descriptions.** The `.tsx`/`.json` carry `name`, `tags`, and
   `collision` per tile, but no `description` — the prose a tile could feed
   into room/look text or a DM/debug overlay. The generator
   (`bin/generate_oathstar_tileset.py`) would need to emit it.
2. **Tilesets as regions/sub-regions.** Today the world's regions and
   sub-regions live in `modules/beginner/world.toml`; the map's cells map to
   tiles purely by render kind (#32 v1). The direction: author a `.tsx` (or
   a Tiled `.tmx` map referencing it) **per region/sub-region**, with
   per-tile metadata rich enough that the world model — room placement,
   passability, region boundaries, descriptions — can be derived from or
   validated against the tile data, rather than tiles being a purely
   cosmetic afterthought.

## Proposed Outcome

- The generator emits a `description` per tile (and any other authoring
  metadata design wants: region affinity, terrain class, hazard flags), in
  both the `.tsx` properties and the `.json` twin.
- A documented region/sub-region authoring model: how a `.tsx`/`.tmx` maps
  onto Oathstar's `world.toml` regions, sub-regions, rooms, and exits —
  either generating world data, decorating it, or validating against it
  (decide which).
- The map renderer can consume per-region tile assignments (supersedes #32's
  by-kind mapping) and surface tile descriptions where the UX wants them
  (look/examine, DM overlay, tooltips).
- Engine/server impact decided explicitly: if tile metadata feeds world
  validation or room text, the renderer-agnostic-JSON boundary (Decisions
  025/035) needs a deliberate amendment, not drift.

## Candidate EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The tileset generator shall emit a non-empty `description` for every tile, present in both the `.tsx` properties and the `.json` twin. | generator test / doc check |
| REQ-002 | When a region or sub-region is authored as a tileset-backed definition, the build shall derive or validate the region's rooms/passability against the tile metadata deterministically. | Rust test (validation boundary) |
| REQ-003 | When the player examines a location whose tile carries a description, the surfaced text shall include the authored tile description. | Rust/server test |
| REQ-004 | The map renderer shall draw per-region tile assignments when present, falling back to kind-based tiles otherwise. | node --test |

## Scope Notes

- In (candidate): generator metadata extension; region/sub-region authoring
  model design (likely its own design doc); renderer per-region assignments;
  examine/overlay surfacing of descriptions.
- Out (candidate): replacing `world.toml` wholesale; full Tiled `.tmx`
  level-editing pipeline (may be a later step of this same direction);
  animation; non-map art.
- **Sequencing:** depends on #32 (tileset rendering v1) landing first —
  that ticket proves the sheet + JSON + renderer plumbing this builds on.
  Likely splits into 2–3 pipelines: (a) generator metadata + descriptions,
  (b) region authoring model + engine seam, (c) renderer/UX consumption.

## Promotion Checklist

- [ ] Forge ticket created.
- [ ] Pipeline spec/notes pair created.
- [ ] `ticket:` frontmatter updated.
- [ ] `pipeline_spec:` frontmatter updated.
- [ ] `status:` changed to `promoted`.
