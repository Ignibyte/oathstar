---
title: TICKET-48-paint-editor-render-select-draw
status: closed
ticket: 42024e47-caba-44b7-865b-c09f4a8a941d
ticket_number: 48
type: feature
created: 2026-06-17
closed: 2026-06-16
intake: docs/planning/intake/INTAKE-paint-system-tile-editor.md
pipeline_spec: docs/planning/pipeline/completed/WORK-paint-editor-render-select-draw-v1.spec.md
---

# TICKET-48-paint-editor-render-select-draw

## Summary

The paintable editor: make the studio `/editor` visibly render the arctic
tileset, let the author select a tile from a palette, and paint it onto the
tilemap (sprites drawn per painted layer cell). Paint-system S2+S3 on the #47
tileset/layer model.

## Why

Slice 1 (#47) gave the document a tileset registry + tile layers but nothing
visible. This is the slice the owner can SEE and click: select an 8px arctic
tile, draw it onto the map, watch it appear.

## EARS Requirements

See the pipeline spec (REQ-001..008): the pure node-tested units
(`tileIndexToSourceRect`, `canvasPointToCell`, `paletteIndexAtPoint`, the
`paintCell` layer mutation, the sprite-augmented `editorDrawPlan`), the committed
arctic descriptor, the `/editor` palette + paint wiring (seam), and a green gate
at 100% MSI.

## Scope

- In: arctic descriptor; the pure editor logic (carries the gate); the palette +
  click-to-paint seam in the studio `/editor`.
- Out: persistence (S4); metadata panels (S5); undo/redo; multi-layer management;
  runtime materialization of layers (#38).

## Notes

- Forge ticket: 42024e47-caba-44b7-865b-c09f4a8a941d (#48)
- Program intake: docs/planning/intake/INTAKE-paint-system-tile-editor.md (S2+S3)
- Builds on: #47 (AD-claude-paint-layer-model-001)
- Active pipeline: WORK-paint-editor-render-select-draw-v1
