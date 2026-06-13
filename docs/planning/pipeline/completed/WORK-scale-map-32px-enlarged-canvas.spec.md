---
pipeline_id: 649f5994-f284-4910-99fd-bd6fbc2ad2d7
title: WORK-scale-map-32px-enlarged-canvas
ticket: 5a348bfb-4c21-4696-bea3-4597c32da5da
type: work
intake:
notes: WORK-scale-map-32px-enlarged-canvas.notes.md
status: Phase 5 — Complete PASS
---

# WORK-scale-map-32px-enlarged-canvas

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Scale the map — render from native 32px tiles and present the
  tilemap physically larger and crisper on the page (client + asset only).
- **Scope:**
  - **In:** regenerate the committed starter tileset at 32px native (generator
    `TILE=32` → 128×96 sheet, solid colors); update the committed-asset contract
    + uniformity tests to the new geometry; enlarge the on-page map (cell px
    and/or canvas CSS/device-pixel size — design picks the mechanism); update
    the tileset asset path/dir reference; preserve the four `KIND_TILE_NAMES`,
    the name→rect resolution, the flat-color fallback, and crisp pixel-art
    scaling; tests + docs.
  - **Out:** per-room biome colors over the wire (#38); the `.tmx` importer
    (#39); new world content (#40); real art; ANY engine/server/protocol/storage
    change; entity/fog/DM overlays; pan/zoom/camera.
- **Systems:** ui (map renderer), frontend build/assets. **No** engine, server,
  protocol, or storage changes.

## Acceptance Criteria (EARS)
Each acceptance criterion describes one observable behavior with a verification
method.

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When the map renders a discovered cell with the committed tileset loaded, the client shall draw it from the committed **32px native source sheet** (tileSize 32, 128×96), blitted crisply (nearest-neighbor, no smoothing) to the enlarged cell, with the four cell kinds still resolving by tile name. | node --test (tileset `tileRect` `sSize == 32`; canvas-map sprite rect carries `sSize 32`) + visual smoke |
| REQ-002 | The default render config shall enlarge the on-page cell to 64px (2× the prior 32px) so `canvasSize` cssWidth and backingWidth are 2× the prior build (backing scaled by `devicePixelRatio` for crispness) — a physically larger, crisper map. | node --test (`DEFAULT_MAP_CONFIG.tilePixels === 64`; `canvasSize` == columns×64 css, ×dpr backing) + visual smoke |
| REQ-003 | When the tileset is regenerated at 32px, the committed-asset contract test shall pin the new geometry (tileSize 32, 128×96 sheet) and the blank-colors uniformity pin shall still hold — every named tile one uniform opaque color, now 32×32 = 1024 identical pixels. | node --test (tileset contract + per-tile uniformity decode) + deterministic regen (re-run reproduces committed bytes) |
| REQ-004 | The engine and wire protocol shall be unchanged: rooms carry grid coordinates, not pixels — no `MapSnapshot`/`MapRoomSnapshot` field is added or modified. | cargo test (protocol unchanged) + review |

## Locked-In Decisions
- **Client + asset only — no wire/engine change.** The map payload stays
  renderer-agnostic grid coordinates (Decisions 025/035); confirmed pixel-free
  in `crates/oathstar-protocol` (`MapRoomSnapshot` carries x/y/z, no tile size).
  [REQ-004]
- **Tiles stay name-keyed (Decision 050) and flat uniform color (Decision 054).**
  The four load-bearing `KIND_TILE_NAMES`
  (`shadow_void`/`stone_floor`/`wall_face`/`spawn_marker`) and the flat extras
  survive; the uniformity pin is **re-scaled to 32px, not retired** (retirement
  waits for real art per Decision 054's "revisit when").
- **Lean to 32px-native source regen** (generator `TILE=32` → 128×96 sheet),
  NOT a 2× canvas upscale of the 16px source — so real art later isn't
  upscaled. Generation stays deterministic/guarded (no rng, pinned
  compress_level, checksum-verified re-runs). *(Ticket lean; Design confirms.)*
- **Note — reconciled at plan:** destination cells already render at 32px
  (`src/client/map.js` `DEFAULT_MAP_CONFIG.tilePixels = 32`); today the renderer
  *upscales* 16px source → 32px dest. Because #36 made every tile a uniform
  solid color, that upscale is visually identical to native 32px, so the
  *visible* "bigger map" comes from the on-page-enlargement lever (REQ-002),
  while the 32px-native regen (REQ-001) is the future-art-crispness lever.

## Design Resolutions (Phase 2 — full detail in notes)
- **Enlargement = `DEFAULT_MAP_CONFIG.tilePixels` 32 → 64** (the existing
  node-testable knob; raises css + backing). NOT CSS-scaling. *(D-1)*
- **Rename** the asset dir `oathstar-starter-16x16` → `oathstar-starter-32x32`
  (ripples enumerated: generator, `client-app.js` ×2, tests ×6, 2 docs, README).
  *(D-2)*
- **Source sheet = 32px native** (`TILE=32` → 128×96); dest = 64 (crisp integer
  2× blit). `canvas-map.js`/`tileset.js` unchanged (pure, parameterized).
- Contract assertions that move: `tileset.test.js` tileSize/sSize/rect literals
  + paths; `canvas-map.test.js` committed-path + `sSize`; new REQ-002 enlargement
  test. The `.tsx` `tilecount="12" columns="4"` pin is UNCHANGED (grid stays
  4×3); the uniformity pin auto-scales (reads `raw.tileSize`). No PNG
  byte/checksum pin exists (the blank-colors pin is pixel-based).

## Linked Artifacts
- Design docs: `docs/ui-design.md` (Map Direction), `docs/map-system.md`
  (Rendering Direction), `docs/decisions.md` (050 name-keyed tiles, 054
  blank-colors; 025/035 renderer-agnostic payload).
- Intake doc: none — direct owner direction (2026-06-12).
- Ticket doc: `docs/planning/tickets/open/TICKET-37-scale-map-32px-tiles-enlarged-canvas.md`
- Forge ticket: `5a348bfb-4c21-4696-bea3-4597c32da5da` (#37)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
