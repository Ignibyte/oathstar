// Map render model. The server sends renderer-agnostic map JSON
// (docs/map-system.md + Decision 031); the client owns how it is drawn.
//
// The render config (tile size + mode) lives HERE, not in server state, so a
// later canvas or sprite renderer can swap in without changing the MapSnapshot
// shape (REQ-007). `toMapModel` never mutates its input.

export const MAP_RENDER_MODES = ["glyph", "ascii", "sprite"];

export const DEFAULT_MAP_CONFIG = Object.freeze({ tilePixels: 32, mode: "glyph" });

function normalizeConfig(config) {
  const mode = MAP_RENDER_MODES.includes(config?.mode) ? config.mode : DEFAULT_MAP_CONFIG.mode;
  const tilePixels =
    Number.isFinite(config?.tilePixels) && config.tilePixels > 0
      ? config.tilePixels
      : DEFAULT_MAP_CONFIG.tilePixels;
  return { mode, tilePixels };
}

/**
 * Build a grid model from a MapSnapshot and a render config.
 *
 * The minimap is a single z-plane (docs/map-system.md: a square grid with
 * up/down). Rooms can stack at the same `(x, y)` on different floors (the
 * beginner tower does), so this renders the **current room's** plane: the
 * "here" marker is always present and rooms on other floors never collide into
 * one cell. `planes` lists every floor present for later up/down affordances.
 * Cells cover the plane's bounding box; gaps are `present: false`. The input
 * snapshot is never mutated.
 *
 * @param {object} mapSnapshot the `/state` snapshot's `map` field
 * @param {{tilePixels?: number, mode?: string}} [config]
 * @returns {object} grid model with `mode`, `tilePixels`, `z`, `planes`, `cells`
 */
export function toMapModel(mapSnapshot, config = DEFAULT_MAP_CONFIG) {
  const { mode, tilePixels } = normalizeConfig(config);
  const allRooms = Array.isArray(mapSnapshot?.rooms) ? mapSnapshot.rooms : [];
  const region = mapSnapshot?.region ?? "";
  const currentRoomId = mapSnapshot?.currentRoomId ?? null;

  const planes = [...new Set(allRooms.map((room) => room.z ?? 0))].sort((a, b) => a - b);
  const currentRoom =
    allRooms.find((room) => room.current) ??
    allRooms.find((room) => room.id === currentRoomId) ??
    null;
  const z = currentRoom?.z ?? (planes.length > 0 ? planes[0] : 0);
  const rooms = allRooms.filter((room) => (room.z ?? 0) === z);

  if (rooms.length === 0) {
    return { mode, tilePixels, columns: 0, rows: 0, minX: 0, minY: 0, z, planes, cells: [], region, currentRoomId };
  }

  const xs = rooms.map((room) => room.x);
  const ys = rooms.map((room) => room.y);
  const minX = Math.min(...xs);
  const maxX = Math.max(...xs);
  const minY = Math.min(...ys);
  const maxY = Math.max(...ys);
  const byPosition = new Map(rooms.map((room) => [`${room.x},${room.y}`, room]));

  const cells = [];
  for (let y = minY; y <= maxY; y += 1) {
    for (let x = minX; x <= maxX; x += 1) {
      const room = byPosition.get(`${x},${y}`);
      cells.push({
        x,
        y,
        present: Boolean(room),
        id: room?.id ?? null,
        title: room?.title ?? "",
        glyph: room?.glyph ?? ".",
        discovered: Boolean(room?.discovered),
        current: Boolean(room?.current),
        passable: room?.passable,
        // Ticket #33: server-computed presence markers; the wire omits
        // false, so absence coerces to false here.
        hasHostiles: Boolean(room?.hasHostiles),
        hasItems: Boolean(room?.hasItems),
      });
    }
  }

  return {
    mode,
    tilePixels,
    columns: maxX - minX + 1,
    rows: maxY - minY + 1,
    minX,
    minY,
    z,
    planes,
    cells,
    region,
    currentRoomId,
  };
}
