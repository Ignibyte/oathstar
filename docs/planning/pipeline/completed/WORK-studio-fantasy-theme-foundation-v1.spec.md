---
pipeline_id: 3e5a5e6a-0859-4b7a-abdd-f8dc1f511b2c
title: WORK-studio-fantasy-theme-foundation-v1
ticket: 7c6d2165-76eb-4954-b99f-e2e4ba89d3b5
type: work
intake: docs/planning/intake/INTAKE-studio-admin-and-world-model-program.md
notes: WORK-studio-fantasy-theme-foundation-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-studio-fantasy-theme-foundation-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** The fantasy theme foundation for the studio (ticket #50, **slice 1 of
  N**) — adopt the mini-medieval UI kit's panel frame + button on the studio's
  shared surfaces (the #49 nav header, `.panel`s, buttons) via committed,
  served sprite assets + a `border-image` theme layer. The reusable foundation the
  later #50 slices extend.
- **Scope:**
  - **In:**
    1. **Two committed UI crops** under `public/ui/`: one nine-sliceable **panel
       frame** cropped from `Frames.png` (a brown/gold fantasy frame) and one
       **button** background cropped from `Inputs.png`. Cropped with ImageMagick/
       PIL (both available); `raw_assets/` stays gitignored.
    2. **Embedded + served** at routes (e.g. `/ui/panel-frame.png`, `/ui/button.png`)
       via the #48 arctic pattern (`include_bytes!` + handler + route + a serves-it
       test) — heed `PR-claude-serve-every-asset-the-glue-fetches-001`.
    3. **A theme CSS layer** in `static/studio.css`: `border-image` (nine-slice) on
       `.panel` + `.studio-header` using the frame asset; the button asset as the
       button background; a warmer fantasy backdrop via the existing `:root`
       tokens. Applies studio-wide (dashboard, editor, section stubs all use
       `.panel` + buttons).
  - **Out (explicit, → later #50 slices):**
    - **#50b** — the game client (`index.html`/`styles.css`/`src/`) re-skin.
    - **#50c** — the rest of the kit: Portraits, Icons, Emotes, Banners,
      Bars-Sliders-Scrollbars; per-component bespoke theming.
    - Exact visual fine-tuning (which frame, slice insets, palette) is
      **owner-adjustable after viewing** — the gate verifies the wiring, the owner
      verifies the look.
- **Systems:** ui (studio theme)

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The chosen UI crops (a panel frame + a button) shall be committed under `public/ui/` and each served at its route as `200` with `Content-Type: image/png` and a non-empty body. | Rust serves-it tests (one per asset) + file check |
| REQ-002 | The studio stylesheet shall theme `.panel` and `.studio-header` with a `border-image` nine-slice referencing the served frame asset. | studio render/CSS marker test (assert the specific `border-image: url("/ui/…")` rule) |
| REQ-003 | The studio stylesheet shall theme buttons using the served button asset. | CSS marker test (the button rule references `/ui/…`) |
| REQ-004 | Every authenticated studio page shall reference the served UI asset route(s), so the theme is wired end-to-end (no asset the page needs goes unserved). | render-marker test (the page/CSS carries the `/ui/…` reference) + the serves-it tests |
| REQ-005 | The full gate shall stay green; mutation rides on the new asset route handler(s) + the render markers. | `bin/gate.sh` FULL |

## Locked-In Decisions
- **Crop single frames out of the sheets** (a clean single image per asset) —
  `border-image`/`background` need one frame, not a multi-frame sheet sub-region.
  Tool: ImageMagick or PIL (both present). The exact crop rects + which frame/button
  are **design decisions** (pin at Phase 2 from the rendered art).
- **Embed + serve via the #48 arctic pattern** (`include_bytes!` const + a handler
  returning `image/png` + a route + a serves-it test). A small `ui.rs` module (or
  reuse `editor.rs`'s pattern) — design decides.
- **Additive theme over the existing `:root` tokens** — keep the dark studio base,
  add the fantasy frame + button + a warmer accent; not a full color overhaul this
  slice.
- **CSS itself is the genuinely-uncoverable surface** (not mutation-tested); the
  gate rides on the asset route handler(s) + the render markers. Assert **specific
  element/url forms**, not bare substrings (`PR-claude-assert-element-form-not-substring-001`
  — `STUDIO_CSS` is embedded in every page).
- **Visual quality is owner-judged** (the loopback studio can't be self-screenshot);
  the owner tunes the look after viewing.

## Linked Artifacts
- Design docs: `docs/ui-design.md` (theme note at complete if warranted),
  `docs/tileset-contract.md` (the sibling author-asset pattern).
- Intake doc: `docs/planning/intake/INTAKE-studio-admin-and-world-model-program.md`
- Ticket doc: `docs/planning/tickets/open/TICKET-50-fantasy-ui-kit.md`
- Forge ticket: `7c6d2165-76eb-4954-b99f-e2e4ba89d3b5` (#50)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
