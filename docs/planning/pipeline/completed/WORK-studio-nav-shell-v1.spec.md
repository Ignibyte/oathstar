---
pipeline_id: 35d08eb1-52ab-4a95-9a34-727e38ca8ee4
title: WORK-studio-nav-shell-v1
ticket: 7a030671-ba5a-4a08-b257-aadb69528115
type: work
intake: docs/planning/intake/INTAKE-studio-admin-and-world-model-program.md
notes: WORK-studio-nav-shell-v1.notes.md
status: Phase 5 — Complete PASS
---

# WORK-studio-nav-shell-v1

> Pipeline spec (always-loaded contract). Detailed per-phase work lives in the
> paired `.notes.md`. Phase advance = updating the `status:` frontmatter above.

## Work Spec
- **Title:** The studio admin navigation shell (ticket #49) — a persistent nav
  (Maps · Regions · Items · Enemies · Game Settings) on every authenticated studio
  page; the dashboard + `/editor` reparented under it; unbuilt sections are
  Editor-gated "Coming soon" stubs. First slice of the pre-tilemap program; the
  shell that #51 (regions) and future editors slot into and #50 (UI kit) re-skins.
- **Scope:**
  - **In:**
    1. A reusable nav helper in `render.rs` (e.g. `nav(active)`) emitting the five
       links, marking the active section, injected into every authenticated page.
    2. The nav threaded into `dashboard_page` and `editor_page`.
    3. Editor-gated stub pages + routes for `/regions`, `/items`, `/enemies`,
       `/settings` — a single generic "section stub" renderer ("Coming soon"),
       **HTTP 200** (not 404), gated like the dashboard.
    4. `studio.css` nav styles.
  - **Out (explicit):** the actual item/enemy/settings editors; the region
    dashboard (#51); the UI re-skin (#50); any new persisted data; a collapsible/
    responsive nav beyond basic layout.
- **Systems:** ui (studio admin shell)

## Acceptance Criteria (EARS)

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | When an Editor loads any authenticated studio page (dashboard, editor, or a section page), the studio shall render a persistent navigation listing Maps, Regions, Items, Enemies, and Game Settings. | studio render tests assert all five nav labels + their hrefs on each page |
| REQ-002 | The navigation's Maps entry shall link to `/editor`, and each other entry shall link to its section route (`/regions`, `/items`, `/enemies`, `/settings`). | render test asserts each `href` |
| REQ-003 | When an Editor opens a not-yet-built section route, the studio shall serve an Editor-gated "Coming soon" stub page (HTTP 200) naming that section, not a 404. | handler tests: Editor/Owner session → 200 + body names the section + "Coming soon" |
| REQ-004 | When a request without a valid Editor session reaches any section route, the studio shall redirect to `/login` and not serve the page. | handler tests per route: a Player session AND a missing session both redirect (pins `require_role` — PR-claude-gated-page-role-mutant-001) |
| REQ-005 | The navigation shall mark the current section as active so the page indicates where the user is. | render test asserts an active marker (e.g. `aria-current="page"`) on the current section and not on the others |
| REQ-006 | The full gate shall stay green with mutation 100% MSI (the nav/section `format!` markers pinned by the render + handler tests). | `bin/gate.sh` FULL |

## Locked-In Decisions
- **The nav is a shared `render.rs` helper** (`nav(active)`), not a framework —
  reusable verbatim by #50/#51. The active section is passed in by each page.
- **One generic section-stub renderer** (`section_stub_page`) backs all four
  unbuilt sections — no bespoke per-section pages this slice.
- **Editor-gated exactly like the dashboard** (`principal_from_cookie` +
  `require_role(Editor)`); a small shared gate helper is acceptable if it mirrors
  the existing pattern (decide at design).
- **Maps = `/editor`** (the live map editor is the Maps section); the other four
  are stubs.
- **Heed `PR-claude-gated-page-role-mutant-001`** — every new gated route gets a
  Player-redirect test AND an Editor-200 test; **pin nav markers** in the render
  tests so the `format!` blank-body / dropped-link mutant dies (the #45/#48
  pattern).

## Linked Artifacts
- Design docs: `docs/ui-design.md` (studio admin surface — brief nav note at
  complete if warranted). The Maps section is the `/editor` from
  `docs/map-system.md`.
- Intake doc: `docs/planning/intake/INTAKE-studio-admin-and-world-model-program.md`
- Ticket doc: `docs/planning/tickets/open/TICKET-49-studio-nav-shell.md`
- Forge ticket: `7a030671-ba5a-4a08-b257-aadb69528115` (#49)

## Phase Plan
| Phase | Status |
|---|---|
| 1 — Plan | PASS |
| 2 — Design | PASS |
| 3 — Implement | PASS |
| 3.5 — Inspect | PASS |
| 4 — Validate | PASS |
| 5 — Complete | PASS |
