// Canvas map render model: pure geometry + Hi-DPI sizing + a draw-plan that a
// canvas-2D seam can execute. There is NO DOM/canvas access here, so this module
// is unit-tested under `node --test`; the `ctx` calls live in the browser glue
// (client-app.js drawMapCanvas), which is smoke-verified.
//
// Consumes the grid model from src/client/map.js `toMapModel`. The server map
// stays renderer-agnostic JSON (Decisions 025/035); this module owns how it draws.

import { kindTileRects } from "./tileset.js";

// Cell kind -> colors. Mirrors the former styles.css map-cell palette (removed in
// ticket #16); keep roughly in sync with the .legend-current / .legend-known
// swatches still in styles.css. `blocked` is a new muted variant for discovered
// non-passable cells.
export const MAP_PALETTE = Object.freeze({
  empty: Object.freeze({ fill: "#10151a", stroke: "#28303a", text: null }),
  discovered: Object.freeze({ fill: "#1b2425", stroke: "#344149", text: "#ddd4c4" }),
  current: Object.freeze({ fill: "#e5c56f", stroke: "#f1d98f", text: "#101318" }),
  blocked: Object.freeze({ fill: "#181c20", stroke: "#3a2e2e", text: "#8a8076" }),
});

// Presence-marker dots (ticket #33): ember for a live hostile, gold for
// ground items — the blank-colors vertical slice's "enemies and loot as
// colors". Drawn last with a dark outline so they read on any tile.
export const MAP_MARKER_COLORS = Object.freeze({
  hostile: "#ec682b",
  item: "#e6c85a",
  outline: "#0f1216",
});

/**
 * The marker dot radius for a tile (px). Pure — kept out of the seam so the
 * geometry is unit-tested; floored so dots stay visible at small tiles.
 *
 * @param {number} tile tile size in CSS pixels
 * @returns {number}
 */
function markerRadius(tile) {
  return Math.max(2, Math.round(tile * 0.14));
}

/**
 * The pre-resolved marker draw entries for one cell: hostile dot top-right,
 * item dot bottom-right. Every field is consumed by the seam's arc() calls
 * (PR-oathstar-render-plan-test-002 — ops carry only what is drawn).
 *
 * @param {object} cell a cell from {@link module:map.toMapModel}
 * @param {number} x cell origin x (CSS px)
 * @param {number} y cell origin y (CSS px)
 * @param {number} tile tile size (CSS px)
 * @returns {{cx: number, cy: number, r: number, fill: string}[]}
 */
function cellMarkers(cell, x, y, tile) {
  const r = markerRadius(tile);
  // Clamped so the two corner dots can never cross at tiny tiles (≤12px the
  // naive r+2 inset makes them collide; at 8px they would coincide exactly).
  const inset = Math.min(r + 2, Math.max(r, tile / 2 - r));
  const markers = [];
  if (cell.hasHostiles) {
    markers.push({ cx: x + tile - inset, cy: y + inset, r, fill: MAP_MARKER_COLORS.hostile });
  }
  if (cell.hasItems) {
    markers.push({ cx: x + tile - inset, cy: y + tile - inset, r, fill: MAP_MARKER_COLORS.item });
  }
  return markers;
}

/**
 * Hi-DPI canvas sizing for a grid model. The CSS size is `columns/rows *
 * tilePixels`; the backing store is scaled by `devicePixelRatio` so the draw
 * stays crisp on retina displays. A non-finite or non-positive dpr clamps to 1.
 *
 * @param {{columns: number, rows: number, tilePixels: number}} model
 * @param {number} [dpr] device pixel ratio
 * @returns {{cssWidth: number, cssHeight: number, backingWidth: number, backingHeight: number, dpr: number}}
 */
export function canvasSize(model, dpr = 1) {
  const ratio = Number.isFinite(dpr) && dpr > 0 ? dpr : 1;
  const cssWidth = model.columns * model.tilePixels;
  const cssHeight = model.rows * model.tilePixels;
  return {
    cssWidth,
    cssHeight,
    backingWidth: Math.round(cssWidth * ratio),
    backingHeight: Math.round(cssHeight * ratio),
    dpr: ratio,
  };
}

/**
 * Classify a grid cell into a palette kind. An undiscovered cell stays fog
 * ("empty") even when a room occupies it, matching the prior DOM renderer; a
 * discovered non-passable cell is "blocked".
 *
 * @param {object} cell a cell from {@link module:map.toMapModel}
 * @returns {"empty"|"discovered"|"current"|"blocked"}
 */
export function cellKind(cell) {
  if (!cell.present) {
    return "empty";
  }
  if (cell.current) {
    return "current";
  }
  if (cell.discovered && cell.passable === false) {
    return "blocked";
  }
  if (cell.discovered) {
    return "discovered";
  }
  return "empty";
}

/**
 * The glyph font size (px) for a tile, floored at 9px so small tiles stay
 * legible. Pure — kept out of the canvas seam so it is unit-tested.
 *
 * @param {number} tile tile size in CSS pixels
 * @returns {number}
 */
function glyphFontPx(tile) {
  return Math.max(9, Math.round(tile * 0.4));
}

/**
 * Build a draw-plan from a grid model: one op per cell, positioned in CSS pixels
 * (the seam scales the context by dpr). Returned data is plain, so `node --test`
 * can assert the plan without a real canvas. The on-tile mark is the room glyph
 * (a single char fits a 32px tile); the full room title is surfaced via the
 * canvas aria-label (see {@link mapAriaLabel}), not drawn on the tile.
 *
 * With a validated tileset (ticket #32) each op also carries `sprite` — the
 * sheet source rect for the cell's kind — while keeping the palette fields:
 * one plan serves both render modes, because the seam falls back to the flat
 * fill for every op whenever the sheet image is not ready (REQ-003). Ops
 * carry ONLY what the seam draws (PR-oathstar-render-plan-test-002): cell
 * classification is observable through the kind-distinct palette fields and
 * sprite rect, and is unit-tested directly on {@link cellKind}.
 *
 * @param {object} model a model from {@link module:map.toMapModel}
 * @param {object | null} [tileset] a validated tileset from {@link module:tileset.validateTileset}
 * @returns {{width: number, height: number, tile: number, glyphFontPx: number, ops: object[]}}
 */
export function toDrawPlan(model, tileset = null) {
  const tile = model.tilePixels;
  const kindRects = tileset ? kindTileRects(tileset) : null;
  const ops = model.cells.map((cell) => {
    const kind = cellKind(cell);
    const palette = MAP_PALETTE[kind];
    const x = (cell.x - model.minX) * tile;
    const y = (cell.y - model.minY) * tile;
    return {
      x,
      y,
      size: tile,
      fill: palette.fill,
      stroke: palette.stroke,
      textColor: palette.text,
      glyph: cell.glyph,
      sprite: kindRects ? kindRects[kind] : null,
      markers: cellMarkers(cell, x, y, tile),
    };
  });
  return {
    width: model.columns * tile,
    height: model.rows * tile,
    tile,
    glyphFontPx: glyphFontPx(tile),
    ops,
  };
}

/**
 * A concise text summary of the map for the canvas's `aria-label`. The canvas is
 * a single element, so the per-cell labels of the old DOM grid are summarized here.
 *
 * @param {object} model a model from {@link module:map.toMapModel}
 * @returns {string}
 */
export function mapAriaLabel(model) {
  const region = model.region || "Map";
  const floor = model.planes && model.planes.length > 1 ? `, floor ${model.z}` : "";
  const discovered = model.cells.filter((cell) => cell.present && cell.discovered).length;
  const rooms = `${discovered} ${discovered === 1 ? "room" : "rooms"} discovered`;
  // Ticket #33: voice the color-only marker signal. Zero counts append
  // nothing, keeping the marker-less label byte-identical to pre-#33.
  const hostiles = model.cells.filter((cell) => cell.hasHostiles).length;
  const loot = model.cells.filter((cell) => cell.hasItems).length;
  const threats = hostiles ? `, hostiles in ${hostiles} ${hostiles === 1 ? "room" : "rooms"}` : "";
  const spoils = loot ? `, loot in ${loot} ${loot === 1 ? "room" : "rooms"}` : "";
  const current = model.cells.find((cell) => cell.current);
  const here = current ? ` — here: ${current.title || current.glyph}` : "";
  return `Map: ${region}${floor} — ${rooms}${threats}${spoils}${here}`;
}
