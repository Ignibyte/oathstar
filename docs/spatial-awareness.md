# Spatial Awareness & Proximity

How Oathstar decides what a player can perceive and reach around them — the
"blast radius" model introduced in ticket #17. This is the foundation later
systems (sight, hearing, detection, combat aggro, stealth, map overlays) build
on, kept server-authoritative and renderer-agnostic.

## Why

Classic MUD rooms are atomic: you are either in the same room as a thing or you
are not. Oathstar's map is a square grid (Decision 025) where a place can hold
many walkable cells, walls, fixtures, NPCs, and items. Spatial awareness makes
that grid meaningful: a player can notice an NPC two cells west, or see items
nearby, without standing on the exact same tile.

## The model (v1)

The model lives in [`crates/oathstar-core/src/awareness.rs`](../crates/oathstar-core/src/awareness.rs)
and is pure, deterministic, and read-only over the world.

- **Rooms are the grid cells.** Entities and items have no coordinates of their
  own; they are *placed* into rooms by id, so a thing inherits the
  `(region, subregion, x, y, z)` of the room that holds it. `Position::from_room`
  captures that cell. (Per-entity intra-room coordinates are deliberately future
  work — see *Scope* below.)
- **Same-plane gating, then distance.** `Position::same_plane` requires the same
  region, the same subregion, and the same z-level. Something on another floor or
  in another subregion is never "near", only absent. Within a plane,
  `Position::cell_distance` is the **Chebyshev** (king-move) distance,
  `max(|dx|, |dy|)` — a square radius that matches the square grid and an
  8-neighbour tactical feel. It is integer-only (`i32::abs_diff`), so it is
  deterministic and cannot overflow.
- **Action-specific radiuses.** `RadiusConfig { sight, interaction }` (defaults
  `sight = 3`, `interaction = 1` cells). `interaction <= sight` is the intended
  relationship — you reach no farther than you see. New radius kinds (hearing,
  detection) slot in here without changing call sites.
- **Proximity bands.** `Proximity::classify(distance, radii)` returns:
  - `Exact` — the same cell (distance 0); always interactable.
  - `Interactable` — within the interaction radius (reachable), not the same cell.
  - `Visible` — within sight but beyond reach: seen, not yet interactable.
  - `None` — beyond sight: not perceived at all.
- **Structured result.** `Awareness { id, name, description, kind, distance,
  proximity }` is one perceived thing. `AwarenessKind` is `Actor | Fixture | Item`.
- **Reveal placeholder.** Entities and items carry a `hidden: bool` flag
  (`#[serde(default)]`, defaults to visible). Hidden things are excluded from all
  awareness results. This is the seam where future stealth/perception/line-of-sight
  rules will compute concealment dynamically; v1 reads a static flag. `hidden`
  gates **perception** — `perceive`, the snapshot `contents`, and `look <target>`
  (all route through the same query). It does **not** yet gate scripted same-room
  actions: `confront` resolves the boss directly by role, so a hidden roled entity
  would still be confrontable. Aligning scripted actions with the reveal flag is
  future work.

### Queries

- `perceive(world, origin, radii) -> Vec<Awareness>` — everything perceivable from
  `origin` within `sight`, on the same plane, nearest first, hidden things
  excluded.
- `resolve_target(world, origin, radii, query) -> Option<Awareness>` — the nearest
  perceivable thing whose name or an alias matches `query` (case-insensitive),
  exact cell first. The returned `proximity` lets the caller decide whether the
  match is close enough to act on.

## Server-authoritative & renderer-agnostic

The engine emits **structured JSON, never drawing instructions** (Decisions
034/035). Awareness is exposed additively on the state snapshot:

- `RoomSnapshot.contents: Vec<NearbySnapshot>` (in `oathstar-protocol`). Each
  `NearbySnapshot` carries `id`, `name`, `kind` (`"actor" | "fixture" | "item"`),
  `distance`, `proximity` (`"exact" | "interactable" | "visible"`), and the
  convenience boolean `interactable`. It is omitted from JSON when empty, so empty
  rooms stay byte-identical to before.
- The player client's **Nearby panel** already reads `room.contents`
  (`src/client/snapshot.js` `toNearby`), so it becomes data-driven with no client
  logic change. The map payload is untouched — entity/event markers on the canvas
  are a future overlay, not part of this foundation.

## Commands

`look <target>` resolves through `resolve_target`:

- an interactable match (same cell or within reach) is described in full;
- a visible-but-out-of-reach match is reported as too far to examine closely;
- no match yields "you see nothing like that nearby".

Bare `look` (no target) still describes the current room, so existing behavior is
preserved.

`talk <target>` and `take <target>` (ticket #18) consume the same resolver, gating
on `interaction` rather than `sight`:

- `talk <actor>` — a reachable actor responds without moving the player; an actor in
  sight but out of reach is reported as too far; a non-actor or no match is refused.
- `take <item>` — a reachable world item is moved into the player's pack and removed
  from its room so it drops out of the nearby `contents`; an item out of reach,
  hidden, unknown, or a non-item is refused with state preserved.

## How this underpins future systems

| Future system | How this foundation supports it |
|---|---|
| Rooms-as-areas | Extend `Position` with intra-room cell coords; the same distance/plane math applies at finer granularity. |
| NPC / item proximity | Already here — `perceive` lists nearby actors/fixtures/items with distance + band. |
| Combat aggro | A monster "notices" the player when within its sight radius (reuse `classify`); aggro range is just another radius. |
| Stealth / noise | `hidden` becomes computed (stealth vs perception); a hearing radius is added to `RadiusConfig`. |
| Detection / reveal | Line-of-sight and passability occlusion plug into the reveal seam that `hidden` stands in for today. |
| Map overlays | `distance` + `proximity` per thing drive client-side markers/fog without any server drawing instructions. |

## Scope

- **In (v1):** room/cell-granularity position, sight + interaction radiuses,
  Chebyshev distance with region/subregion/z gating, the proximity bands, the
  reveal `hidden` placeholder, the `perceive`/`resolve_target` queries, the
  additive `contents` snapshot, and `look <target>` resolution.
- **Out (future):** per-entity intra-room coordinates; full combat aggro, stealth,
  sound propagation, pathfinding, and final line-of-sight blockers; dialogue
  trees, shops; multiplayer and DM controls; canvas
  entity/event overlays.

## Testing

Radius math, region/subregion/z boundaries, the proximity bands (with their
exact/interaction/sight edges), exact-vs-nearby resolution, alias/case matching,
and the `hidden` reveal placeholder are covered by deterministic Rust unit tests
in `awareness.rs`; the snapshot wiring and `look` command paths are covered in
`oathstar-core` and the server slice smoke; the Nearby-panel rendering is covered
by `node --test` on `toNearby`.
