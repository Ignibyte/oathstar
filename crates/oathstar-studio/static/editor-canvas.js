// Studio editor canvas model (ticket #45): pure geometry + cell classification +
// Hi-DPI sizing + a draw-plan + validate-result formatting for the Oathstar
// Studio `/editor` page. There is NO DOM/canvas/fetch here, so this module is
// unit-tested under `node --test`; the `ctx`/`fetch`/DOM calls live in the
// inline browser glue (render.rs EDITOR_GLUE), which is smoke-verified.
//
// It draws the AUTHORING MapDocument (terrain_palette / terrain / rooms / spawn
// from oathstar-content, ticket #43) — distinct from the runtime MapSnapshot the
// game client's canvas-map.js renders. Same pure-model + thin-seam split
// (Decision 050), different document shape, so this is a sibling, not a reuse.

// Cell kind -> colors. `text` is the on-tile glyph color (null where no glyph is
// drawn). Studio-themed; independent of the game map palette.
export const EDITOR_PALETTE = Object.freeze({
  empty: Object.freeze({ fill: "#10151a", stroke: "#28303a", text: null }),
  floor: Object.freeze({ fill: "#1b2425", stroke: "#344149", text: null }),
  wall: Object.freeze({ fill: "#2a2320", stroke: "#3a2e2e", text: null }),
  room: Object.freeze({ fill: "#2f3a2a", stroke: "#4a5a3f", text: "#dfe7d6" }),
  spawn: Object.freeze({ fill: "#e5c56f", stroke: "#f1d98f", text: "#101318" }),
});

/**
 * The room cell occupying a coordinate, or null. Pure helper.
 *
 * @param {object} doc a MapDocument
 * @param {number} x column
 * @param {number} y row
 * @param {number} z floor / z-plane
 * @returns {object | null}
 */
function roomAt(doc, x, y, z) {
  return doc.rooms.find((room) => room.x === x && room.y === y && room.z === z) || null;
}

/**
 * Classify a grid coordinate into a draw kind. Precedence is topmost-wins:
 * `spawn` > `room` > terrain (`floor`/`wall`) > `empty`. Terrain is `floor` when
 * its palette entry is passable, else `wall` (an unknown/missing palette entry —
 * an invalid document the server will reject — classifies `wall` conservatively).
 *
 * @param {object} doc a MapDocument
 * @param {number} x column
 * @param {number} y row
 * @param {number} z floor / z-plane
 * @returns {"empty"|"floor"|"wall"|"room"|"spawn"}
 */
export function editorCellKind(doc, x, y, z) {
  const spawn = doc.spawn;
  if (spawn && spawn.x === x && spawn.y === y && spawn.z === z) {
    return "spawn";
  }
  if (roomAt(doc, x, y, z)) {
    return "room";
  }
  const terrain = doc.terrain.find((cell) => cell.x === x && cell.y === y && cell.z === z);
  if (terrain) {
    const def = doc.terrain_palette[terrain.terrain];
    return def && def.passable === true ? "floor" : "wall";
  }
  return "empty";
}

/**
 * Build a draw-plan for one z-plane: one op per grid cell in deterministic order
 * (row-major, `y` outer / `x` inner), positioned in CSS pixels. Returned data is
 * plain so `node --test` can assert it without a canvas; the seam scales the
 * context by dpr and executes the ops. The on-tile mark is the room glyph (the
 * full title is voiced via {@link editorAriaLabel}); only room/spawn cells carry
 * a glyph (an authored room sits under the spawn marker).
 *
 * @param {object} doc a MapDocument
 * @param {{z?: number, tilePixels: number, focusSubregion?: (string|null)}} opts
 * @returns {{width: number, height: number, tile: number, ops: object[]}}
 */
export function editorDrawPlan(doc, { z = 0, tilePixels, focusSubregion = null }) {
  const tile = tilePixels;
  const tilesets = tilesetsById(doc);
  const spritesByCell = layerSpritesByCell(doc, z, tilesets);
  const ops = [];
  for (let y = 0; y < doc.height; y += 1) {
    for (let x = 0; x < doc.width; x += 1) {
      const kind = editorCellKind(doc, x, y, z);
      const palette = EDITOR_PALETTE[kind];
      const room = roomAt(doc, x, y, z);
      ops.push({
        x: x * tile,
        y: y * tile,
        size: tile,
        kind,
        fill: palette.fill,
        stroke: palette.stroke,
        textColor: palette.text,
        glyph: room ? room.glyph || "." : null,
        // Whether this room is in the focused sub-region (#51c) — set when the editor
        // is opened via `/editor?subregion=<id>` from the regions dashboard; the seam
        // outlines focused cells. Non-room cells and a null/unmatched focus stay false.
        focused: room !== null && focusSubregion != null && room.subregion === focusSubregion,
        // Painted layer tiles at this cell, bottom layer first (ticket #48); the
        // seam blits these beneath the stroke/glyph, falling back to `fill` when
        // the cell carries no sprite.
        sprites: spritesByCell.get(`${x},${y}`) || [],
      });
    }
  }
  return { width: doc.width * tile, height: doc.height * tile, tile, ops };
}

/**
 * Index the document's tileset registry by id (ticket #47 model). Pure.
 *
 * @param {object} doc a MapDocument
 * @returns {Map<string, object>}
 */
function tilesetsById(doc) {
  const map = new Map();
  for (const ts of doc.tilesets || []) {
    map.set(ts.id, ts);
  }
  return map;
}

/**
 * Group the painted layer cells of plane `z` by `"x,y"` into their sprite source
 * rects, bottom layer first. A cell whose tileset is not registered is skipped.
 *
 * @param {object} doc a MapDocument
 * @param {number} z floor / z-plane
 * @param {Map<string, object>} tilesets from {@link tilesetsById}
 * @returns {Map<string, object[]>}
 */
function layerSpritesByCell(doc, z, tilesets) {
  const byCell = new Map();
  for (const layer of doc.layers || []) {
    for (const cell of layer.cells || []) {
      if (cell.z !== z) {
        continue;
      }
      const tileset = tilesets.get(cell.tileset);
      if (!tileset) {
        continue;
      }
      const rect = tileIndexToSourceRect(cell.index, tileset.columns, tileset.tile_size);
      const key = `${cell.x},${cell.y}`;
      const list = byCell.get(key) || [];
      list.push({ sx: rect.sx, sy: rect.sy, sSize: rect.size, tileset: cell.tileset });
      byCell.set(key, list);
    }
  }
  return byCell;
}

/**
 * Hi-DPI canvas sizing. CSS size is `width/height * tilePixels`; the backing
 * store is scaled by `devicePixelRatio` so the draw stays crisp on retina
 * displays. A non-finite or non-positive dpr clamps to 1 (mirrors canvas-map.js).
 *
 * @param {object} doc a MapDocument
 * @param {{tilePixels: number, devicePixelRatio?: number}} opts
 * @returns {{cssWidth: number, cssHeight: number, backingWidth: number, backingHeight: number, dpr: number}}
 */
export function editorCanvasSize(doc, { tilePixels, devicePixelRatio = 1 }) {
  const ratio = Number.isFinite(devicePixelRatio) && devicePixelRatio > 0 ? devicePixelRatio : 1;
  const cssWidth = doc.width * tilePixels;
  const cssHeight = doc.height * tilePixels;
  return {
    cssWidth,
    cssHeight,
    backingWidth: Math.round(cssWidth * ratio),
    backingHeight: Math.round(cssHeight * ratio),
    dpr: ratio,
  };
}

/**
 * A concise text summary of the document for the canvas's `aria-label`. The
 * canvas is one element, so per-cell labels are summarized: title + room count +
 * terrain-cell count + spawn coordinate.
 *
 * @param {object} doc a MapDocument
 * @returns {string}
 */
export function editorAriaLabel(doc) {
  const rooms = doc.rooms.length;
  const terrainCount = doc.terrain.length;
  const spawn = doc.spawn;
  const where = spawn ? `, spawn at (${spawn.x}, ${spawn.y}, ${spawn.z})` : ", no spawn";
  return (
    `Map editor: ${doc.title} — ` +
    `${rooms} ${rooms === 1 ? "room" : "rooms"}, ` +
    `${terrainCount} terrain ${terrainCount === 1 ? "cell" : "cells"}${where}`
  );
}

/**
 * Turn a `/editor/maps/validate` JSON response into a display model. A success
 * (`ok:true`) summarizes the materialized world; any other shape (a failed
 * validation, or an auth/parse refusal) renders the server `message`, which
 * names the offending cell/reference, with a fallback when it is absent.
 *
 * @param {object} resp the parsed JSON body from POST /editor/maps/validate
 * @returns {{ok: boolean, headline: string, detail: string}}
 */
export function formatValidateResult(resp) {
  if (resp && resp.ok === true) {
    return {
      ok: true,
      headline: "Valid map",
      detail:
        `${resp.room_count} ${resp.room_count === 1 ? "room" : "rooms"}, ` +
        `${resp.region_count} ${resp.region_count === 1 ? "region" : "regions"}, ` +
        `start: ${resp.start_room_id}`,
    };
  }
  return {
    ok: false,
    headline: "Invalid map",
    detail: (resp && resp.message) || "validation failed",
  };
}

/**
 * Turn a `/editor/maps` save response into a display model (sibling of
 * {@link formatValidateResult}). A success (`ok:true`) names the saved storage
 * slot; any other shape (a slot/auth/parse refusal) renders the server `message`,
 * with a fallback when it is absent.
 *
 * @param {object} resp the parsed JSON body from POST /editor/maps
 * @returns {{ok: boolean, headline: string, detail: string}}
 */
export function formatSaveResult(resp) {
  if (resp && resp.ok === true) {
    return { ok: true, headline: "Saved", detail: `as ${resp.id}` };
  }
  return {
    ok: false,
    headline: "Not saved",
    detail: (resp && resp.message) || "save failed",
  };
}

/**
 * Turn a `/editor/maps/activate` response into a display model (sibling of
 * {@link formatSaveResult}). A success (`ok:true`) confirms the world is the active
 * slot and reminds the owner a server restart loads it; any other shape (a
 * non-materializing doc, or an auth/parse refusal) renders the server `message`.
 *
 * @param {object} resp the parsed JSON body from POST /editor/maps/activate
 * @returns {{ok: boolean, headline: string, detail: string}}
 */
export function formatActivateResult(resp) {
  if (resp && resp.ok === true) {
    return {
      ok: true,
      headline: "Active world set",
      detail: "Restart the game server to play it.",
    };
  }
  return {
    ok: false,
    headline: "Not activated",
    detail: (resp && resp.message) || "the world does not materialize",
  };
}

// ---- ticket #48: paint helpers (palette / point math / mutation) ----

/**
 * The source pixel rect of a tile `index` on its sheet — a `columns`-wide grid
 * of `tileSize` tiles. `size` is the source tile edge. Pure.
 *
 * @param {number} index 0-based tile index
 * @param {number} columns sheet width in tiles
 * @param {number} tileSize source tile edge (px)
 * @returns {{sx: number, sy: number, size: number}}
 */
export function tileIndexToSourceRect(index, columns, tileSize) {
  return {
    sx: (index % columns) * tileSize,
    sy: Math.floor(index / columns) * tileSize,
    size: tileSize,
  };
}

/**
 * The grid cell under a canvas point (CSS px), or null when the point lies
 * outside the `width` x `height` grid. Pure.
 *
 * @param {number} px canvas-relative x (CSS px)
 * @param {number} py canvas-relative y (CSS px)
 * @param {number} tilePixels rendered cell edge (CSS px)
 * @param {number} width grid width in cells
 * @param {number} height grid height in cells
 * @returns {{x: number, y: number} | null}
 */
export function canvasPointToCell(px, py, tilePixels, width, height) {
  if (px < 0 || py < 0) {
    return null;
  }
  const x = Math.floor(px / tilePixels);
  const y = Math.floor(py / tilePixels);
  if (x >= width || y >= height) {
    return null;
  }
  return { x, y };
}

/**
 * The palette tile index under a point — the palette draws the sheet as a
 * `columns`-wide grid of `tileSize * scale` cells — or null outside the tiles.
 * Pure.
 *
 * @param {number} px palette-relative x (CSS px)
 * @param {number} py palette-relative y (CSS px)
 * @param {number} columns sheet width in tiles
 * @param {number} tileSize source tile edge (px)
 * @param {number} scale palette zoom factor
 * @param {number} tileCount total tiles on the sheet
 * @returns {number | null}
 */
export function paletteIndexAtPoint(px, py, columns, tileSize, scale, tileCount) {
  if (px < 0 || py < 0) {
    return null;
  }
  const cell = tileSize * scale;
  const col = Math.floor(px / cell);
  if (col >= columns) {
    return null;
  }
  const index = Math.floor(py / cell) * columns + col;
  return index < tileCount ? index : null;
}

/**
 * A copy of `doc` with `tileRef` painted onto layer `layerId` at `cell` — any
 * existing tile at that coordinate on that layer is replaced (one tile per cell
 * per layer). A missing layer leaves the document unchanged. Pure — `doc` is not
 * mutated.
 *
 * @param {object} doc a MapDocument
 * @param {string} layerId the target layer's id
 * @param {{x: number, y: number, z: number}} cell the painted coordinate
 * @param {{tileset: string, index: number}} tileRef the tile to place
 * @returns {object} a new MapDocument
 */
export function paintCell(doc, layerId, cell, tileRef) {
  const layers = (doc.layers || []).map((layer) => {
    if (layer.id !== layerId) {
      return layer;
    }
    const cells = (layer.cells || []).filter(
      (c) => !(c.x === cell.x && c.y === cell.y && c.z === cell.z),
    );
    cells.push({
      x: cell.x,
      y: cell.y,
      z: cell.z,
      tileset: tileRef.tileset,
      index: tileRef.index,
    });
    return { ...layer, cells };
  });
  return { ...doc, layers };
}

/**
 * The cells of the inclusive rectangle spanned by corners `a` and `b`, normalized
 * (any drag direction yields the same set), in deterministic row-major order
 * (`y` outer, `x` inner). Pure — the marquee paint seam folds {@link paintCell}
 * over these (#57).
 *
 * @param {{x: number, y: number}} a one corner cell
 * @param {{x: number, y: number}} b the opposite corner cell
 * @returns {{x: number, y: number}[]}
 */
export function cellsInRect(a, b) {
  const x0 = Math.min(a.x, b.x);
  const x1 = Math.max(a.x, b.x);
  const y0 = Math.min(a.y, b.y);
  const y1 = Math.max(a.y, b.y);
  const cells = [];
  for (let y = y0; y <= y1; y += 1) {
    for (let x = x0; x <= x1; x += 1) {
      cells.push({ x, y });
    }
  }
  return cells;
}

/**
 * Paint `tileRef` into every cell of the rectangle `a`→`b` on `layerId` at plane
 * `z`, folding the immutable {@link paintCell}. Returns a new document (the input is
 * unchanged); a 1×1 rectangle (`a == b`) paints exactly the one cell. Pure (#57).
 *
 * @param {object} doc a MapDocument
 * @param {string} layerId the target layer's id
 * @param {{x: number, y: number}} a one corner cell
 * @param {{x: number, y: number}} b the opposite corner cell
 * @param {number} z the z-plane the painted cells carry
 * @param {{tileset: string, index: number}} tileRef the tile to place
 * @returns {object} a new MapDocument
 */
export function paintRect(doc, layerId, a, b, z, tileRef) {
  let next = doc;
  for (const cell of cellsInRect(a, b)) {
    next = paintCell(next, layerId, { x: cell.x, y: cell.y, z }, tileRef);
  }
  return next;
}
