import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

// REQ-001: the Datastar runtime is vendored + pinned through the project build in a
// reproducible way. These checks pin the exact bytes (sha256) and the page wiring so
// a silent CDN swap, a corrupted re-vendor, or a dropped <script> fails the suite.

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const RUNTIME = join(ROOT, "public", "vendor", "datastar", "datastar.js");
const PROVENANCE = join(ROOT, "public", "vendor", "datastar", "PROVENANCE.md");
const INDEX = join(ROOT, "index.html");

// The pinned digest (also recorded in PROVENANCE.md and the package.json vendor:datastar script).
const EXPECTED_SHA256 =
  "2837d87acf6ee0ba8e4e63765926c25a98d63883b02f88be194a86b81d3fd24a";

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

test("datastar runtime is vendored and is a real bundle", () => {
  const bytes = readFileSync(RUNTIME);
  assert.ok(bytes.length > 1000, `expected a real bundle, got ${bytes.length} bytes`);
  assert.ok(bytes.toString("utf8").includes("Datastar"), "bundle identifies as Datastar");
});

test("vendored datastar.js matches the pinned sha256 (reproducible)", () => {
  assert.equal(sha256(RUNTIME), EXPECTED_SHA256);
});

test("PROVENANCE records the same pinned sha256 and version", () => {
  const prov = readFileSync(PROVENANCE, "utf8");
  assert.ok(prov.includes(EXPECTED_SHA256), "PROVENANCE pins the sha256");
  assert.ok(prov.includes("v1.0.2"), "PROVENANCE records the pinned version");
});

test("index.html loads the vendored runtime and opens the Datastar feed", () => {
  const html = readFileSync(INDEX, "utf8");
  assert.ok(
    html.includes("/vendor/datastar/datastar.js"),
    "index.html references the self-hosted runtime (not a CDN)",
  );
  assert.ok(
    html.includes('data-init="@get(\'/events/datastar\')"'),
    "index.html opens the Datastar SSE feed on Datastar init",
  );
});
