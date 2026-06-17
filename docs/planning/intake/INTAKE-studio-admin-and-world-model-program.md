---
title: INTAKE-studio-admin-and-world-model-program
status: promoted
created: 2026-06-17
ticket: 7a030671-ba5a-4a08-b257-aadb69528115, 7c6d2165-76eb-4954-b99f-e2e4ba89d3b5, 341c0863-3fdc-49cd-a438-18a4f5d827f2, 32db0cc9-0176-45a4-a599-6a6a37ff8c18
pipeline_spec:
---

# INTAKE-studio-admin-and-world-model-program

## Problem / Opportunity

The paint editor (paint-system S2+S3, ticket #48) is working. **Before** polishing
the tile map (paint S4 persistence / S5 metadata), the owner wants to step back and
build the structure the studio needs to be a real content tool, adopt a proper visual
theme, and reshape the world model away from MUD-style vertical movement toward a
real-RPG region model. These four pieces are the **pre-tilemap program** — they ship
before we return to the tile map.

## Proposed Outcome

When this program is done:
- The studio is a **navigable admin** (Maps · Regions · Items · Enemies · Game
  Settings), not a single map editor.
- Both the studio and the game client are themed with the **mini-medieval fantasy
  UI kit** (fantasy-first; sci-fi elements come later as Oathstar expands).
- A **region / sub-region dashboard** lets the owner edit region and sub-region
  attributes, and **each sub-region links into the tile-map editor**.
- The **world model is 2D**: no up/down movement, no floors for travel. Vertical
  traversal becomes a **warp** into another region/sub-region (e.g. walking north
  into a cave entrance). Tile **layers keep `z` purely for visual stacking**.

## The four tickets (build order)

| Order | Ticket | Title | Depends on |
|---|---|---|---|
| 1 | **#49** | Studio admin navigation shell | — |
| 2 | **#50** | Adopt the mini-medieval fantasy UI kit (studio + game) | #49 |
| 3 | **#51** | Region & sub-region dashboard (+ link to tile editor) | #49, #50 |
| 4 | **#52** | World model: 2D movement, drop up/down, region warps | (coordinates with #51) |

Local ticket docs: `docs/planning/tickets/open/TICKET-49..52-*.md`.

## Locked-in decisions (owner, 2026-06-17)

- **Build order:** nav (#49) → UI kit (#50) → regions (#51) → world model (#52).
  Tools-and-theme first, deepest engine change last.
- **z / floors:** movement and travel are **2D** (rooms drop up/down and floors);
  tile **layers keep `z` for visual sprite stacking only**. Consistent with the
  editor already built (its layers use `z`).
- **UI kit scope:** **both** the studio admin and the game client. Fantasy-first.
- The UI kit is `raw_assets/mini-medieval/user-interface/` (Frames, Banners,
  Bars-Sliders-Scrollbars, Inputs, Icons, Portraits, Emotes) — the same pack the
  arctic tiles come from. `raw_assets/` is gitignored, so chosen art is copied into
  `public/` or embedded (mirroring how `arctic.png` was handled).
- #52 **amends locked Decision 025** ("square grid with cardinal directions plus
  up/down").

## Open questions carried into design

- **#50:** nine-slice approach for resizable frames; icon→action mapping; a slicing
  descriptor for the UI sheets (like the tileset contract); whether to split into a
  theme-foundation slice then per-surface application.
- **#51:** which region attributes to edit beyond name/description (region-standing
  defaults?); how a sub-region maps to a tile-map document — linking a sub-region to
  *its own* map implies map persistence/identity (paint S4 territory); where edited
  region data is stored (the studio has no persistence layer yet).
- **#52:** is a warp a special exit (direction → region+room) or a distinct
  "entrance" object?; where warps are authored (tile editor vs region dashboard);
  migration of existing room/terrain `z` data and the `floors` field.

## Scope Notes

- In: the four tickets above (the pre-tilemap program).
- Out: paint S4 (save/load persistence) and S5 (per-tile/layer/room metadata) — they
  **resume after** this program; the full region-standing consequence system; the
  sci-fi UI variant.

## Promotion Checklist

- [x] Forge tickets created (#49, #50, #51, #52).
- [x] Local ticket docs created under `tickets/open/`.
- [ ] Pipeline spec/notes pair created per ticket (at `/work` time, in build order).
- [x] `status:` set to `promoted`.
