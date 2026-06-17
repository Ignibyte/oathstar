---
title: TICKET-50-fantasy-ui-kit
status: open
ticket: 7c6d2165-76eb-4954-b99f-e2e4ba89d3b5
ticket_number: 50
type: feature
created: 2026-06-17
intake: docs/planning/intake/INTAKE-studio-admin-and-world-model-program.md
pipeline_spec:
---

# TICKET-50-fantasy-ui-kit

## Summary

Re-skin **both** the studio admin and the player-facing game client with the
mini-medieval fantasy UI kit (`raw_assets/mini-medieval/user-interface/`: Frames,
Banners, Bars-Sliders-Scrollbars, Inputs, Icons, Portraits, Emotes), backed by a
reusable theme/component layer.

## Why

Oathstar is a fantasy RPG (sci-fi elements arrive later as it expands), so it needs a
cohesive fantasy look to replace the placeholder styling. The arctic map tiles already
come from this same pack. Done **after** the nav shell (#49) so the studio is themed
once its structure exists. Build order 2 of 4.

## EARS Requirements

| ID | EARS Requirement | Verification |
|---|---|---|
| REQ-001 | The studio admin shall render its panels, buttons, and bars using the mini-medieval UI kit. | visual review + asset-present check |
| REQ-002 | The game client shell shall render using the same UI kit theme. | visual review |
| REQ-003 | The UI assets used shall be committed into the repo (not referenced from gitignored `raw_assets/`). | file check |
| REQ-004 | A reusable theme/component layer (shared CSS + sliced sprites) shall back both surfaces. | review |

## Scope

- In: bring the chosen UI sheets in as committed assets (copy into `public/` or embed,
  mirroring `arctic.png`); a component/theme layer for panels, buttons, bars, frames,
  icons; apply to the studio nav/dashboard/editor and the game client shell.
- Out: bespoke per-screen art; the later sci-fi variant theme.

## Notes

- Forge ticket: #50 `7c6d2165-76eb-4954-b99f-e2e4ba89d3b5`
- Build order: **2 of 4**. Depends on: #49 (a nav exists to theme).
- Scope decision (owner): **both** studio + game client. Fantasy-first.
- Open questions (design): nine-slice for resizable `Frames.png`; icon→action mapping;
  a slicing descriptor for the UI sheets (like `docs/tileset-contract.md`). **May be
  split at planning** (theme foundation → apply to studio → apply to game).
- Promoted from intake: yes. Active pipeline: not yet.
