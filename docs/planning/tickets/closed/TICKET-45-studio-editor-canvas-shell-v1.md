---
title: TICKET-45-studio-editor-canvas-shell-v1
status: open
ticket: 549049dc-d7b4-4db8-9a6d-231718d5d55b
ticket_number: 45
type: feature
created: 2026-06-15
intake: docs/planning/intake/INTAKE-online-first-multiplayer-and-auth-gated-studio.md
pipeline_spec: docs/planning/pipeline/active/WORK-studio-editor-canvas-shell-v1.spec.md
---

# TICKET-45-studio-editor-canvas-shell-v1

## Summary

Build the first Oathstar Studio editor UI (intake Section D): an Editor-gated
`GET /editor` page in the `oathstar-studio` sidecar that renders a `MapDocument`
on an HTML canvas and validates it against the existing `POST
/editor/maps/validate` endpoint (#44), showing the typed `{ok, summary | error}`
result.

## Why

#43 gave us the authoring `MapDocument` model and #44 served it behind an
Editor gate, but nothing yet *renders* it for a human. This is the first
browser-visible Studio surface and the natural next consumer of the #44 endpoint
— it turns the validate API into something an author can see and use.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When a caller without an Editor-granting session requests `GET /editor`, the studio shall redirect to `/login` and not serve the editor page. | Rust handler test |
| REQ-002 | When an Editor or Owner session requests `GET /editor`, the studio shall respond `200` with an HTML page containing a `<canvas>`, the embedded starter `MapDocument`, and a Validate control. | Rust handler test |
| REQ-003 | When given a `MapDocument`, the pure editor model shall produce a deterministic draw plan classifying each current-z cell as empty, terrain floor, terrain wall, room, or spawn. | node --test |
| REQ-004 | When given a tile display size and grid bounds, the pure editor model shall compute a Hi-DPI canvas size whose backing store is `round(cssPx · devicePixelRatio)`. | node --test |
| REQ-005 | When given a `MapDocument`, the pure editor model shall produce an aria-label naming the map title and a content summary. | node --test |
| REQ-006 | When the Validate control is activated, the editor shall POST the document JSON to `/editor/maps/validate` and render the typed result — an `ok:true` summary or an `ok:false` error that names the offending cell/reference. | browser smoke + node --test (pure result-format) |
| REQ-007 | The editor shall not modify the game player client and shall add no game-engine dependency, using only the canvas 2D API and `fetch`. | code / package review |
| REQ-008 | The editor page shall expose only the authoring `MapDocument` and the validate result (no engine/internal leakage), and identical input shall yield an identical draw plan. | node --test + review |
| REQ-009 | When complete, `cargo test --workspace`, `node --test tests/*.test.js`, and `bin/gate.sh` (FULL) shall pass. | command output |

## Scope

- In: `GET /editor` (Editor-gated, redirect-to-login) in `oathstar-studio`;
  a canvas viewport that draws a server-embedded starter `MapDocument`
  (current z-plane: empty / floor / wall / room / spawn); a Validate control
  that POSTs the document JSON to the #44 endpoint and renders the typed
  result; a new studio-owned **pure** JS model (draw-plan/kind/size/aria/
  result-format) with node --test coverage + a thin canvas/fetch/DOM seam;
  light up the dashboard "Map editor" placeholder with a link; docs.
- Out: tile palette + paint/erase/select brushes; client-side draft state +
  a draft-save API (next slice, D-paint); the room inspector (E);
  publish-to-live-world (F); live-state overlays; any change to the game
  player client (`index.html`, `styles.css`, `src/client-app.js`) or the
  existing `src/client/*.js` game-canvas modules; any game server / protocol /
  engine change; a real user/content DB.

## Notes

- Forge ticket: `549049dc-d7b4-4db8-9a6d-231718d5d55b` (#45)
- Related docs: `docs/map-system.md` (Map Document Model + canvas sections);
  forge `AD-claude-map-document-model-001`, `AD-claude-studio-sidecar-001`,
  `AD-claude-studio-json-endpoint-001`; Decision 050 (first-party canvas).
- Reuses: #42 studio auth gate, #43 `MapDocument`, #44 validate endpoint.
  Mirrors (not reuses) the #16 game-canvas pure-model pattern.
- Promoted from intake: `INTAKE-online-first-multiplayer-and-auth-gated-studio.md`
  (Section D).
- Active pipeline: `docs/planning/pipeline/active/WORK-studio-editor-canvas-shell-v1.spec.md`
