# Vendored third-party runtime — Datastar

This directory holds the pinned Datastar browser runtime, self-hosted and served
verbatim by vite (`public/` is copied into `dist/` on `npm run build`). Self-hosting
is Datastar's recommended distribution; the npm package is stale (1.0.0-beta.11,
old API), so we vendor the exact bytes for reproducibility (no network at build,
no runtime CDN dependency). Ticket #15 / Decision 034.

| Field | Value |
|---|---|
| Package | Datastar (the hypermedia framework) |
| Version | **v1.0.2** |
| File | `datastar.js` (ESM browser bundle) |
| Source | `https://cdn.jsdelivr.net/gh/starfederation/datastar@v1.0.2/bundles/datastar.js` |
| SHA-256 | `2837d87acf6ee0ba8e4e63765926c25a98d63883b02f88be194a86b81d3fd24a` |
| Vendored | 2026-06-07 |
| Upstream license | MIT (starfederation/datastar) |

## Re-vendor / verify

```
npm run vendor:datastar
```

That script re-fetches the pinned URL and fails unless the SHA-256 matches the
value above. To bump the version, change the tag in `package.json`'s
`vendor:datastar` script, run it, and update the Version + SHA-256 here in the
same commit.
