// Canvas map render model: pure geometry + Hi-DPI sizing + a draw-plan that a
// canvas-2D seam can execute. There is NO DOM/canvas access here, so this module
// is unit-tested under `node --test`; the `ctx` calls live in the browser glue
// (client-app.js drawMapCanvas), which is smoke-verified.
//
// Consumes the grid model from src/client/map.js `toMapModel`. The server map
// stays renderer-agnostic JSON (Decisions 025/035); this module owns how it draws.

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
 * @param {object} model a model from {@link module:map.toMapModel}
 * @returns {{width: number, height: number, tile: number, glyphFontPx: number, ops: object[]}}
 */
export function toDrawPlan(model) {
  const tile = model.tilePixels;
  const ops = model.cells.map((cell) => {
    const kind = cellKind(cell);
    const palette = MAP_PALETTE[kind];
    return {
      x: (cell.x - model.minX) * tile,
      y: (cell.y - model.minY) * tile,
      size: tile,
      kind,
      fill: palette.fill,
      stroke: palette.stroke,
      textColor: palette.text,
      glyph: cell.glyph,
      here: Boolean(cell.current),
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
  const current = model.cells.find((cell) => cell.current);
  const here = current ? ` — here: ${current.title || current.glyph}` : "";
  return `Map: ${region}${floor} — ${rooms}${here}`;
}
