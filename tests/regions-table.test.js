// node --test suite for the regions dashboard's pure table helpers (ticket #58).
// The module is DOM-free, so these tests load it directly and assert filter/sort
// over plain row records; the browser drag/glue (render.rs REGIONS_GLUE) is a
// separate smoke-/review-verified seam and is not exercised here.

import test from "node:test";
import assert from "node:assert/strict";
import { filterRows, sortRows } from "../crates/oathstar-studio/static/regions-table.js";

const ROWS = [
  { title: "Beta", id: "b1", regions: "9", subs: "1" },
  { title: "alpha", id: "a2", regions: "10", subs: "2" },
  { title: "Gamma", id: "g3", regions: "2", subs: "0" },
];

test("filterRows: case-insensitive title|id substring; blank → all (a copy); no match → [] (#58)", () => {
  assert.deepEqual(filterRows(ROWS, "ALP").map((r) => r.id), ["a2"]); // title match, case-insensitive
  assert.deepEqual(filterRows(ROWS, "g3").map((r) => r.id), ["g3"]); // id match
  const all = filterRows(ROWS, "   "); // blank/whitespace → all
  assert.equal(all.length, 3);
  assert.notEqual(all, ROWS, "returns a new array, not the input ref");
  assert.deepEqual(filterRows(ROWS, "zzz"), []); // no match
});

test("sortRows: numeric regions, locale title, dir, stable, immutable (#58)", () => {
  // regions sort NUMERICALLY (2, 9, 10) — not lexicographic ("10","2","9")
  assert.deepEqual(sortRows(ROWS, "regions", "asc").map((r) => r.regions), ["2", "9", "10"]);
  assert.deepEqual(sortRows(ROWS, "regions", "desc").map((r) => r.regions), ["10", "9", "2"]);
  // title via localeCompare
  assert.deepEqual(sortRows(ROWS, "title", "asc").map((r) => r.title), ["alpha", "Beta", "Gamma"]);
  assert.deepEqual(sortRows(ROWS, "title", "desc").map((r) => r.title), ["Gamma", "Beta", "alpha"]);
  // STABLE: two rows with the same key keep input order in BOTH directions
  const ties = [
    { id: "x", subs: "5" },
    { id: "y", subs: "5" },
    { id: "z", subs: "1" },
  ];
  assert.deepEqual(sortRows(ties, "subs", "asc").map((r) => r.id), ["z", "x", "y"]);
  assert.deepEqual(sortRows(ties, "subs", "desc").map((r) => r.id), ["x", "y", "z"]);
  // input array is not mutated
  const before = ROWS.map((r) => r.id);
  sortRows(ROWS, "title", "asc");
  assert.deepEqual(ROWS.map((r) => r.id), before, "input order unchanged");
});
