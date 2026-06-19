// Pure, DOM-free helpers for the regions dashboard table (ticket #58): filter and
// sort plain row records `{ title, id, regions, subs }` (a record may also carry an
// `el` DOM reference, which these functions ignore). The browser seam in
// `render.rs` (REGIONS_GLUE) builds records from the rendered `<tr>` data and calls
// these; everything decision-bearing is testable under `node --test`.

/** Row keys sorted numerically (the rest sort lexicographically). */
const NUMERIC_KEYS = new Set(["regions", "subs"]);

/**
 * The rows whose `title` or `id` contains `query` (case-insensitive, trimmed). A
 * blank query returns every row. Pure — returns a new array; never mutates input.
 *
 * @param {{title: string, id: string}[]} rows the row records
 * @param {string} query the search text
 * @returns {object[]} the matching rows, in input order
 */
export function filterRows(rows, query) {
  const q = String(query || "").trim().toLowerCase();
  if (q === "") {
    return rows.slice();
  }
  return rows.filter((row) => `${row.title} ${row.id}`.toLowerCase().includes(q));
}

/**
 * A new array of `rows` sorted by `key` — numeric for `regions`/`subs`, otherwise a
 * locale string compare — ascending (`dir !== "desc"`) or descending. Stable: equal
 * keys keep their input order (relies on `Array.prototype.sort` stability, ES2019+).
 * Pure — never mutates input.
 *
 * @param {object[]} rows the row records
 * @param {string} key the record field to sort by
 * @param {string} dir `"asc"` (default) or `"desc"`
 * @returns {object[]} a sorted copy
 */
export function sortRows(rows, key, dir) {
  const sign = dir === "desc" ? -1 : 1;
  const numeric = NUMERIC_KEYS.has(key);
  return rows.slice().sort((a, b) => {
    const cmp = numeric
      ? Number(a[key]) - Number(b[key])
      : String(a[key]).localeCompare(String(b[key]));
    return sign * cmp;
  });
}
