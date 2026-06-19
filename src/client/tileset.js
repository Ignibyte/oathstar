// Tileset metadata model. Pure and DOM-free: this module parses and validates
// an author-provided tile-sheet descriptor (see docs/tileset-contract.md) and
// resolves tile names to source pixel rects on the sheet. Image loading and
// drawImage live in the browser seam (client-app.js); everything
// decision-bearing is testable under `node --test`.

// Map-cell kind -> tile NAME on the sheet. Names are the author sheet's stable
// contract (see docs/tileset-contract.md): cells resolve art by name, so a new
// sheet under the same names is a drop-in swap. Future per-tile metadata
// extends by name without touching this table's consumers.
export const KIND_TILE_NAMES = Object.freeze({
  empty: "shadow_void",
  discovered: "stone_floor",
  blocked: "wall_face",
  current: "spawn_marker",
});

// The committed default tile sheet, served by the SPA host at this origin-relative
// path — Vite serves `public/` at `/` in dev, copies it into `dist/` on build, and
// Tauri bundles it. The client points here unless `VITE_OATHSTAR_TILESET` overrides
// it, so the game map renders real tiles by default (S3.1, ticket #54).
export const DEFAULT_TILESET_URL = "/tilesets/arctic.json";

/**
 * Resolve the map tileset descriptor URL: the trimmed `override` (e.g. the
 * build-time `VITE_OATHSTAR_TILESET`) when it is a non-empty string, otherwise the
 * committed {@link DEFAULT_TILESET_URL}. Pure and total — never throws, never
 * returns empty; the browser seam supplies the override and does the fetch, and a
 * missing/invalid sheet still falls back to flat colors in `loadTileset`.
 *
 * @param {unknown} override the configured override (a URL string) or anything falsy
 * @returns {string} a non-empty tileset descriptor URL
 */
export function resolveTilesetUrl(override) {
  const trimmed = typeof override === "string" ? override.trim() : "";
  return trimmed === "" ? DEFAULT_TILESET_URL : trimmed;
}

/**
 * Validate raw tileset JSON into a usable tileset. Never throws on any input
 * (the renderer must fall back, not break — REQ-003/004): returns
 * `{ ok: true, tileset }` where `tileset` adds a `byName` lookup, or
 * `{ ok: false, reason }` naming the first failed check.
 *
 * @param {unknown} raw parsed JSON (or anything — malformed input is a result)
 * @returns {{ok: true, tileset: object} | {ok: false, reason: string}}
 */
export function validateTileset(raw) {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    return { ok: false, reason: "tileset metadata is not an object" };
  }
  const { tileSize, columns, rows, image, tiles } = raw;
  // Integers only: a fractional sheet geometry or tile origin yields subpixel
  // drawImage source rects, which resample across atlas tile boundaries and
  // smear the pixel art no matter what smoothing flags say (REQ-005).
  if (!Number.isInteger(tileSize) || tileSize <= 0) {
    return { ok: false, reason: "tileSize must be a positive integer" };
  }
  if (!Number.isInteger(columns) || columns <= 0) {
    return { ok: false, reason: "columns must be a positive integer" };
  }
  if (!Number.isInteger(rows) || rows <= 0) {
    return { ok: false, reason: "rows must be a positive integer" };
  }
  if (typeof image !== "string" || image.length === 0) {
    return { ok: false, reason: "image must be a non-empty string" };
  }
  if (!Array.isArray(tiles)) {
    return { ok: false, reason: "tiles must be an array" };
  }
  const sheetWidth = columns * tileSize;
  const sheetHeight = rows * tileSize;
  // Null prototype: a tile named "__proto__" must become an ordinary own key,
  // not an Object.prototype setter hit that re-prototypes the lookup table
  // and lets the required-names check pass on inherited properties.
  const byName = Object.create(null);
  for (const tile of tiles) {
    if (typeof tile !== "object" || tile === null) {
      return { ok: false, reason: "every tile must be an object" };
    }
    if (typeof tile.name !== "string" || tile.name.length === 0) {
      return { ok: false, reason: "every tile must have a non-empty name" };
    }
    if (!Number.isInteger(tile.x) || !Number.isInteger(tile.y)) {
      return { ok: false, reason: `tile '${tile.name}' has a non-integer position` };
    }
    if (tile.x < 0 || tile.y < 0 || tile.x + tileSize > sheetWidth || tile.y + tileSize > sheetHeight) {
      return { ok: false, reason: `tile '${tile.name}' lies outside the sheet` };
    }
    byName[tile.name] = tile;
  }
  for (const name of Object.values(KIND_TILE_NAMES)) {
    if (!byName[name]) {
      return { ok: false, reason: `required tile '${name}' is missing` };
    }
  }
  return { ok: true, tileset: { tileSize, columns, rows, image, tiles, byName } };
}

/**
 * The source pixel rect of a named tile on the sheet, or null when the name
 * is unknown. `sSize` is the square source tile edge (the sheet's tileSize).
 *
 * @param {object} tileset a validated tileset from {@link validateTileset}
 * @param {string} name a tile name
 * @returns {{sx: number, sy: number, sSize: number} | null}
 */
export function tileRect(tileset, name) {
  const tile = tileset.byName[name];
  if (!tile) {
    return null;
  }
  return { sx: tile.x, sy: tile.y, sSize: tileset.tileSize };
}

/**
 * The four cell-kind source rects, resolved through {@link KIND_TILE_NAMES}.
 * On a validated tileset every kind resolves (validation requires the names).
 *
 * @param {object} tileset a validated tileset from {@link validateTileset}
 * @returns {{empty: object, discovered: object, current: object, blocked: object}}
 */
export function kindTileRects(tileset) {
  return {
    empty: tileRect(tileset, KIND_TILE_NAMES.empty),
    discovered: tileRect(tileset, KIND_TILE_NAMES.discovered),
    current: tileRect(tileset, KIND_TILE_NAMES.current),
    blocked: tileRect(tileset, KIND_TILE_NAMES.blocked),
  };
}
