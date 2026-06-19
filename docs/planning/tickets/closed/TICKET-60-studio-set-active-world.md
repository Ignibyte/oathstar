# TICKET-60 — Set as active world: promote an authored map to the game's active slot

- **Forge ID:** `057bcd35-bdae-4996-a557-755ee6434844` (#60)
- **Type:** feature · **Status:** open (pipeline `WORK-studio-set-active-world-v1`)
- **Program:** pivot item ② slice 1 of the studio-editable-world program (memory `studio-editable-world-pivot`)

## Why
An authored map only becomes the game's world if saved under the magic id `world` (the #56
active-world resolver loads `maps_dir/world.json` at startup), with no guard against promoting a
broken map (which would silently fall back to baked beginner).

## What
A clean promotion action: a studio `set_active` handler (Editor-gated; parse → materialize-or-400
without writing → write the doc to the **fixed** active slot `world` → `200 {active:"world"}`), a
`POST /editor/maps/activate` route, a **"Set as active world"** editor button, and a pure
`formatActivateResult` (node-tested). Slot name fixed; activation requires materialize; reuse
`materialize`/`refuse`/`editor_refusal`.

## Acceptance
See `docs/planning/pipeline/active/WORK-studio-set-active-world-v1.spec.md` (EARS REQ-001..006).

## Out of scope
Converting baked beginner → an editable map; world hot-reload (server loads at startup — a restart
is still needed); the editor tabbed UX (pivot ③); an active-world indicator beyond button feedback.
