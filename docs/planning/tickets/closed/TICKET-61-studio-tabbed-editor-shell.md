# TICKET-61 — Tabbed editor shell: controls to a right rail + a Tiles/Regions/Rooms/Map tab bar

- **Forge ID:** `e87f802b-cd8e-40c8-b382-6c9be8d26388` (#61)
- **Type:** feature · **Status:** open (pipeline `WORK-studio-tabbed-editor-shell-v1`)
- **Program:** pivot item ③ slice a of the studio-editable-world program (memory `studio-editable-world-pivot`)

## Why
The owner asked to move the editor's Save/Validate to a right rail and add a tabbed system "to begin
being able to edit." This is the structural foundation for the editor-UX redesign; later slices fill
the tabs (regions, room metadata, map expansion).

## What
Layout-only restructure of `render.rs editor_page`: LEFT stage (`canvas-panel`) + RIGHT
`aside.editor-rail` with the controls moved up (ids `#map-name/#save/#validate/#activate/#result`
preserved), a `role="tablist"` bar (Tiles | Regions | Rooms | Map, Tiles active), and four tabpanels
— Tiles = the palette (visible), Regions/Rooms/Map = hidden "Coming soon" stubs. A pure
`tabPanelStates(tabIds, activeId)` helper (node-tested, first-tab fallback) drives a tablist click
handler; `studio.css` styles the rail/tabs on the `:root` tokens. All ids + behavior preserved.

## Acceptance
See `docs/planning/pipeline/active/WORK-studio-tabbed-editor-shell-v1.spec.md` (EARS REQ-001..006).

## Out of scope
Filling the Regions/Rooms/Map tabs (later slices); the room-metadata inspector; map expansion; the
quick-edit modal; curating the palette.
