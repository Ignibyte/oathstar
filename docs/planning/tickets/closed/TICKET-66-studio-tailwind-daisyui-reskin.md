---
title: TICKET-66-studio-tailwind-daisyui-reskin
status: closed
ticket: 5dd351ba-7422-498a-8a50-1ef8df9c3019
ticket_number: 66
type: feature
created: 2026-06-19
intake:
pipeline_spec: docs/planning/pipeline/active/WORK-studio-tailwind-daisyui-reskin.spec.md
---

# TICKET-66-studio-tailwind-daisyui-reskin

## Summary

Re-skin the Oathstar **studio / Host Manager** UI (`crates/oathstar-studio`) onto
**Tailwind v4 + DaisyUI** as a reusable component system, replacing the
hand-written `studio.css` and the ticket-50 pixel-art `border-image` theme. The
player game client is **out of scope**.

## Why

The studio UI (the owner's day-to-day authoring tool — tickets #59–#65 built the
tabbed editor inside it) is clunky and hand-maintained as 251 lines of bespoke
CSS with class names hand-threaded through Rust `format!` strings. A component
kit (DaisyUI) gives consistent, modern admin surfaces and faster iteration.
**Decision 034 (Locked)** explicitly sanctions a component kit for the Host
Manager ("if admin velocity matters more than bespoke presentation"); it bars
DaisyUI only for the *player client*, which is why this ticket is studio-only.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The studio stylesheet shall be produced by a Tailwind v4 build with the DaisyUI plugin enabled, content-scanning the studio Rust sources, and the committed compiled CSS shall equal a fresh build (no drift). | `npm run` build + `git diff --exit-code`, run in `bin/gate.sh` |
| REQ-002 | Every authenticated studio page shall embed the compiled DaisyUI stylesheet (the embedded `STUDIO_CSS` shall contain a recognizable DaisyUI theme/base marker, asserted by element form not a bare substring). | cargo test on `STUDIO_CSS` |
| REQ-003 | The studio shell and editor surfaces shall render DaisyUI component classes — the nav as `navbar`, panels as `card`, actionable controls as `btn`, the regions list as `table`, the editor tab bar as `tabs`. | cargo test on each page-render fn |
| REQ-004 | The re-skin shall preserve every interactivity hook the inline glue reads — `data-tab`, `data-ok`, the regions-row data hooks (title/id/regions/subs), and `data-key` — so editor and regions interactions keep functioning. | cargo test asserting the hooks remain; existing JS module tests stay green |
| REQ-005 | No studio page shall reference the ticket-50 fantasy sprites (`/ui/panel-frame.png`, `/ui/button.png`) in its CSS — the `border-image` theme is superseded by DaisyUI. | cargo test asserting absence in `STUDIO_CSS` |
| REQ-006 | `bin/gate.sh` FULL shall remain green, including the new CSS-build no-drift gate, with mutation at 100% MSI. | `bin/gate.sh` FULL |

**Genuinely uncoverable:** the rendered *appearance* ("does it look clean and
un-clunky") — owner-judged after viewing the running loopback studio; it cannot
be self-screenshotted.

## Scope

- In: Tailwind v4 + DaisyUI build toolchain (node) for the studio; content-scan
  config over the studio Rust sources; compiled CSS kept `include_str!`-embedded
  (Decision 060); convert login, header/nav, dashboard, section stubs, the map
  editor, and the regions surfaces to DaisyUI components; preserve the fetch +
  inline-glue interactivity and its `data-*` hooks; wire a CSS no-drift check
  into `bin/gate.sh`; supersede ticket-50's studio theme; rewrite the affected
  HTML-substring tests.
- Out: the player game client and its `styles.css` (Decision 034 bars DaisyUI
  there); the player-client half of ticket-50; new bespoke art/sprites; removing
  the now-dormant `/ui/*.png` sprite routes (left intact to keep Decision 060
  unchanged — optional follow-up cleanup).

## Notes

- Forge ticket: 5dd351ba-7422-498a-8a50-1ef8df9c3019 (#66)
- Related docs: `docs/decisions.md` (Decision 034 UI/Datastar styling direction;
  Decision 060 content-vs-code asset embedding), `docs/planning/tickets/open/TICKET-50-fantasy-ui-kit.md`
  (its studio slice is superseded here), `docs/planning/pipeline/completed/WORK-studio-fantasy-theme-foundation-v1.*`
- Promoted from intake: none (direct owner request)
- Active pipeline: docs/planning/pipeline/active/WORK-studio-tailwind-daisyui-reskin.spec.md
