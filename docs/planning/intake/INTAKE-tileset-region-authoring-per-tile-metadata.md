---
title: INTAKE-tileset-region-authoring-per-tile-metadata
status: candidate
created: 2026-06-11
ticket:
pipeline_spec:
---

# INTAKE-tileset-region-authoring-per-tile-metadata

## Problem / Opportunity

Owner direction (2026-06-11, refined in discussion 2026-06-12): author the
world's regions in the Tiled format family instead of hand-writing room
grids in `world.toml`. The settled mental model (corrected from the first
sketch, which conflated the two formats):

- **`.tsx` = the tileset** — the shared art *vocabulary* (tile types + per-
  TYPE properties). It does not grow with the world. The runtime client
  keeps consuming its JSON twin for sheet rects (ticket #32's plumbing).
- **`.tmx` = the map** — tile placements on a grid, object layers, and
  map-level properties. **This is the per-sub-region authoring file.**
  Maps are what grow large, hence one `.tmx` per sub-region (and likely
  per floor — Tiled maps are 2D; the tower's z-planes are separate maps
  or separate layers).

Two metadata tiers (the "where do descriptions live" question, settled):

1. **Per tile TYPE — in the `.tsx`:** `collision` (already emitted),
   terrain class, and a generic `description` fragment ("Cracked
   flagstones, cold underfoot."). Trivial generator extension — it already
   writes name/tags/collision into both the `.tsx` and the JSON twin.
   Room prose can NOT live here: every `stone_floor` placement would
   share one description.
2. **Per ROOM — in the `.tmx` object layer:** named objects dropped on
   cells carry `id`, `title`, multiline `description`, and overrides
   (e.g. an explicit `reachable = false` for intentionally sealed rooms).
   The object IS the room; the tile under it is its art.

Region structure: each sub-region `.tmx` carries its own identity as
map-level custom properties (`region`, `subregion`, `floor`) — metadata
lives IN the file; a thin region manifest (or Tiled's native `.world`
JSON) composes sub-region maps into a region. File layout:
`modules/<region>/<subregion>.tmx` beside the existing TOML.

Reachability is two different things, neither a hand-authored flag on
tiles: tile-type `collision` (in the `.tsx`) says what is ever walkable;
room reachability is a **validation pass** — the importer graph-walks
passable cells from the spawn marker (the sheet already has
`spawn_marker` / `exit_marker` tiles) and loudly refuses a map with
orphaned rooms, matching `world.validate()`'s posture.

## Proposed Outcome

- **Build-time importer, not engine parsing (architecture decision,
  leaning):** Tiled is the authoring surface; an importer converts each
  sub-region map (prefer Tiled's native JSON export `.tmj` — serde-
  friendly, no XML parsing) into the module/world data the engine already
  eats. Engine and protocol stay graphics-free (Decisions 025/035
  intact); a broken map fails the BUILD, not the running game. Exits
  derive from passable-cell adjacency; stairs/up-down are explicit object
  links between maps.
- The generator emits per-tile-type `description` (+ terrain class) in
  both `.tsx` and JSON twin.
- The server map snapshot gains one additive field — the room's tile
  NAME — a deliberate, small protocol amendment; the client draws
  authored tiles with #32's kind-based mapping as the fallback (the
  name-keyed design was shaped for exactly this).
- The TOML format stays alive until the `.tmx` path proves round-trip
  parity on a real sub-region — no big-bang migration.

## Authoring Boundaries (settled in discussion, 2026-06-12)

**The description cascade doubles as the authoring model.** Any walkable
cell WITHOUT a room object is an **implicit room**: sub-region fallback
`description`, generated title (`title_fallback`), deterministic id
(`<subregion>:<x>,<y>,<z>` — saves reference room ids, so implicit ids
must survive re-import; moving a special room without keeping its
explicit id breaks saves). A room object makes a cell **explicit**: own
prose, stable hand-chosen id, overrides. Painting IS the authoring for
ordinary cells. The cascade is two-deep (room → sub-region) and is
**materialized by the importer at build time** — the engine never learns
what a fallback is; it receives plain fully-described rooms. The
tile-type fragment (`.tsx` description) is additive texture (e.g.
`examine ground`), not a third prose tier.

**What lives where (one source per concern):**

| Concern | Authored where | Rendered from |
|---|---|---|
| Terrain, structure, decor visuals | TMX tile/decoration layers | the map file |
| Room identity + prose (special places) | TMX object layer | — |
| Fixture/actor DEFINITIONS (description, aliases, inventory, dialogue, combat profile) | TOML entity registry | — |
| INITIAL placements (fixtures AND actor spawns) | the sub-region's spatial source — TMX objects by `entity = "<id>"` ref (TOML rooms for non-migrated sub-regions) | — |
| Live state (ground items, living enemies, anything that moves/changes) | nowhere — it is game state | runtime map overlay markers |

- The importer validates every placement ref resolves
  (`EntityItemMissing` posture); a fixture/actor object on a non-room
  cell refuses.
- Authoring lint (warn, not refuse): an interactive-looking prop tile
  (`.tsx` tags) painted with no entity ref on the cell — visual
  affordance should match interactivity.
- **Runtime overlay markers are Tiled-independent**: per-room
  `has_items`/`has_hostiles` presence flags in the map snapshot + generic
  icons client-side (the sheet's prop/marker tiles serve). Can ship as
  its own small ticket before any TMX work. Open design question for
  that ticket: marker visibility through fog (live state everywhere vs
  only in visited rooms, "what you last saw").

## Blank-Colors Mode (owner decision + spike, 2026-06-12)

**Worldbuilding happens in flat solid colors; art comes at ship time.**
The committed 64-tile pixel-art starter sheet is more fidelity than
worldbuilding needs. Because all resolution is by tile NAME (#32), the
swap is free in both directions: author maps against a lean flat sheet
now; "shipping" later = regenerating the sheet with real art under the
same names — zero map data or code churn. The textured sheet stays in
git history as the future sheet's first draft.

**Spike (ran 2026-06-12, `/tmp/oathstar-flat-spike/` — ephemeral, port
in the pilot ticket):** proved the full blank-colors loop on the REAL
beginner world: (1) generated an 11-tile flat tileset (png+json+tsx —
void, five subregion-flavored floors, wall, water, stairs ×2, spawn);
(2) parsed `modules/beginner/rooms.toml`; (3) emitted one real `.tmx`
per floor — terrain layer + room objects carrying
title/description/region/subregion/combat_enabled/up/down, map-level
fallback properties; (4) ran importer-style validation: exit↔adjacency
coherence, stair-link coherence, reachability from spawn — CLEAN;
(5) rendered a human preview PNG of all three floors. The existing
rooms' `region`/`subregion` fields map 1:1 onto the metadata model.

**Resequenced steps (supersedes the list below where they differ):**
- **A (small): flatten the committed tileset.** Generator rewrites to
  solid colors, lean name set; KEEP the four load-bearing names
  (`shadow_void`/`stone_floor`/`wall_face`/`spawn_marker`) so the
  client + KIND_TILE_NAMES need zero changes; add the flat extras
  (subregion floor variants, water, stairs, exit marker). Tests update
  (tile count + name cross-checks). Client untouched.
  **→ PROMOTED 2026-06-12:** ticket #36
  (`edd292c5-a346-49aa-86a8-f60191f2a081`), pipeline
  `WORK-tileset-flatten-v1`.
- **B: the pilot importer** (= step 2 below), now de-risked by the
  spike: Rust importer in the content-crate orbit, implicit-room
  cascade materialization, adjacency-derived exits, reachability
  validation, beginner-world round-trip parity.
  **→ PROMOTED 2026-06-12:** ticket #39
  (`b46517e6-9306-4a2e-a09e-18c04b013151`). Deferred behind C + W1 per
  the recommended sequence (authoring infra, not a visible step).
- **C: per-room tile names over the wire** (= step 3 below) — makes the
  subregion floor colors visible in the client.
  **→ PROMOTED 2026-06-12:** ticket #38
  (`51636b55-d4e7-4f22-a826-0a2b5fc04b76`). Recommended NEXT.
- Step 0 (runtime overlay markers) and step 1 (per-tile descriptions)
  remain independent and unordered against A–C.

## Candidate EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The tileset generator shall emit a non-empty `description` for every tile type, present in both the `.tsx` properties and the `.json` twin. | generator run + node --test on the committed JSON |
| REQ-002 | When the importer ingests a sub-region map, it shall derive rooms (id, title, description, coords, exits, region/subregion/floor) from the object layer and tile placements deterministically. | Rust test |
| REQ-003 | If any room object sits on a collision tile, any room is unreachable from the spawn marker (absent an explicit override), or required map properties are missing, the importer shall refuse the map with a typed error naming the room/cell. | Rust test (both arms) |
| REQ-004 | The pilot sub-region authored as a `.tmx`/`.tmj` shall import to world data behaviorally equivalent to its existing TOML definition (same rooms, exits, descriptions, passability). | Rust round-trip test |
| REQ-005 | When a room carries an authored tile name, the map snapshot shall expose it and the client shall draw that tile, falling back to kind-based tiles otherwise. | node --test + server test |

## Scope Notes

- In (candidate): generator description metadata; the `.tmj` importer +
  validation (content-crate orbit); one pilot sub-region authored in
  Tiled; the per-room tile-name protocol field + renderer consumption.
- Out (candidate): replacing `world.toml` wholesale; entities/items/oaths
  authored in Tiled (rooms first); fog/overlay/animation; multi-tileset
  atlases.
- **Sequencing (each step ships alone; 0 is Tiled-independent and can go
  first or anytime):**
  0. Runtime overlay markers — `has_items`/`has_hostiles` per room in the
     map snapshot + generic icons (sheet props) client-side. Decide
     fog/knowledge semantics in that ticket.
  1. Generator per-tile `description` metadata — tiny ticket.
  2. Pilot sub-region `.tmx` + importer (cascade materialization,
     placement refs, reachability validation) — prove TOML parity
     (REQ-004) before anything depends on it.
  3. Per-room tile names over the wire + renderer consumption
     (supersedes #32's by-kind mapping as the primary path).
- Depends on #32 (shipped 2026-06-11): the committed tileset, the
  name-keyed resolver, and the fallback machinery are the substrate.

## Promotion Checklist

- [ ] Forge ticket created.
- [ ] Pipeline spec/notes pair created.
- [ ] `ticket:` frontmatter updated.
- [ ] `pipeline_spec:` frontmatter updated.
- [ ] `status:` changed to `promoted`.
