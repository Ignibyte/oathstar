---
pipeline_id: 55e8d9e6-7546-4961-8557-8a1cbcbf1696
title: WORK-studio-tailwind-daisyui-reskin
ticket: 5dd351ba-7422-498a-8a50-1ef8df9c3019
ticket_number: 66
type: work
intake:
notes: WORK-studio-tailwind-daisyui-reskin.notes.md
status: Phase 5 — Complete PASS
---

# WORK-studio-tailwind-daisyui-reskin

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** Re-skin the studio / Host Manager UI (`crates/oathstar-studio`) onto
  Tailwind v4 + DaisyUI as a reusable component system — replacing the bespoke
  `studio.css` and the ticket-50 pixel-art `border-image` theme. Studio only.
- **Scope:**
  - **In:**
    1. A **node Tailwind v4 build** (DaisyUI plugin) that **content-scans the
       studio Rust sources** (`render.rs` and the other HTML-emitting modules,
       where class names live in `format!` string literals) and compiles a
       stylesheet.
    2. The compiled CSS stays **`include_str!`-embedded** and inlined per page —
       **no new served-asset route** (Decision 060: code/CSS embedded, only
       content PNGs runtime-served). Output overwrites `static/studio.css`; a new
       input file holds the `@import "tailwindcss"` + `@plugin "daisyui"` + theme.
    3. **Convert every studio surface** to DaisyUI components — login,
       header/nav, dashboard, section stubs, the map **editor** (canvas panel +
       rail + tabs), and the **regions** list + region editor — using
       `navbar` / `card` / `btn` / `table` / `tabs` / `menu` etc.
    4. **Preserve interactivity:** the studio uses plain `fetch` + inline glue
       (not Datastar). Keep the glue and every `data-*` hook it reads
       (`data-tab`, `data-ok`, regions-row `data-title/id/regions/subs`,
       `data-key`) intact through the re-skin.
    5. **Gate wiring:** a **CSS no-drift check** in `bin/gate.sh` — a fresh
       Tailwind build must produce byte-identical committed CSS.
    6. **Supersede ticket-50's studio theme:** remove the `border-image` rules;
       the two `/ui/*.png` sprite routes stay **dormant** (Decision 060 intact).
    7. **Rewrite the affected tests** — ~30 HTML-substring assertions in
       `render.rs` keyed off the old class names + the `pages_reference_the_ui_kit_assets`
       test.
  - **Out:** the player game client + `styles.css` (Decision 034 bars DaisyUI
    there); the player-client half of ticket-50; new bespoke art; **removing**
    the dormant `/ui/*.png` routes (optional follow-up). The exact visual look is
    owner-tuned after viewing.
- **Systems:** ui (studio / Host Manager); build/tooling (node Tailwind);
  gate (`bin/gate.sh`).

## Acceptance Criteria (EARS)
Each acceptance criterion uses EARS syntax, describes one observable behavior,
and includes a verification method.

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The studio stylesheet shall be produced by a Tailwind v4 build with the DaisyUI plugin enabled, content-scanning the studio Rust sources, and the committed compiled CSS shall equal a fresh build (no drift). | `npm run` build + `git diff --exit-code`, run in `bin/gate.sh` |
| REQ-002 | Every authenticated studio page shall embed the compiled DaisyUI stylesheet — the embedded `STUDIO_CSS` shall contain a recognizable DaisyUI theme/base marker, asserted by element form not a bare substring (`PR-claude-assert-element-form-not-substring-001`). | cargo test on `STUDIO_CSS` |
| REQ-003 | The studio shell and editor surfaces shall render DaisyUI component classes — the nav as `navbar`, panels as `card`, actionable controls as `btn`, the regions list as `table`, the editor tab bar as `tabs`. | cargo test on each page-render fn |
| REQ-004 | The re-skin shall preserve every interactivity hook the inline glue reads — `data-tab`, `data-ok`, the regions-row data hooks (title/id/regions/subs), and `data-key` — so editor and regions interactions keep functioning. | cargo test asserting the hooks remain; existing JS module tests stay green |
| REQ-005 | No studio page shall reference the ticket-50 fantasy sprites (`/ui/panel-frame.png`, `/ui/button.png`) in its CSS — the `border-image` theme is superseded by DaisyUI. | cargo test asserting absence in `STUDIO_CSS` |
| REQ-006 | `bin/gate.sh` FULL shall remain green, including the new CSS-build no-drift gate, with mutation at 100% MSI. | `bin/gate.sh` FULL |

**Genuinely uncoverable:** the rendered *appearance* ("does it look clean and
un-clunky") — owner-judged after viewing the running loopback studio (it cannot
self-screenshot). The gate verifies the wiring; the owner verifies the look.

## Locked-In Decisions
- **Studio only.** The player client is out (Decision 034 bars DaisyUI there);
  no `styles.css` / `index.html` / `src/` change.
- **DaisyUI on the studio is decision-sanctioned** (Decision 034 — a component
  kit is permitted for the Host Manager). No decision amendment is required.
- **CSS stays embedded** (`include_str!` → inlined `<style>`), consistent with
  **Decision 060** (only content PNGs are runtime-served). The Tailwind build's
  output overwrites `static/studio.css`; **no new HTTP route for CSS.**
- **Preserve the existing fetch + inline-glue interactivity** and its `data-*`
  hooks. (Discovery: the studio does **not** use Datastar today; Decision 034's
  Datastar/SSE model is not in play on this surface — so "keep Datastar intact"
  reduces to "keep the glue + data hooks intact.")
- **Tailwind content-scanning targets the Rust sources** (`crates/oathstar-studio/src/*.rs`),
  because class names are emitted from `format!` literals there; the static JS
  modules generate no DOM/classes at runtime (no need to scan them, though they
  may be included defensively).
- **Ticket-50's studio fantasy theme is superseded** by this re-skin. The
  `/ui/panel-frame.png` + `/ui/button.png` routes (`ui.rs`) remain but go
  dormant; Decision 060 is unchanged. Removing them is an optional follow-up.
- **A CSS no-drift gate** is added to `bin/gate.sh` so a forgotten rebuild can't
  ship stale styling (strict, no-baseline philosophy). Node is already a gate
  dependency.

## Linked Artifacts
- Design docs: `docs/decisions.md` (Decision 034 — UI styling direction;
  Decision 060 — content-vs-code asset embedding); `docs/ui-design.md`
- Intake doc: none (direct owner request)
- Ticket doc: `docs/planning/tickets/open/TICKET-66-studio-tailwind-daisyui-reskin.md`
- Superseded: `docs/planning/tickets/open/TICKET-50-fantasy-ui-kit.md` (studio
  slice only); `docs/planning/pipeline/completed/WORK-studio-fantasy-theme-foundation-v1.*`
- Forge ticket: 5dd351ba-7422-498a-8a50-1ef8df9c3019 (#66)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
