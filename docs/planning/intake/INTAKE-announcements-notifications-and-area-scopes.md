---
title: INTAKE-announcements-notifications-and-area-scopes
status: promoted
created: 2026-06-07
ticket: 5e945705-2a54-4add-a7d2-cce9b60cefc4
pipeline_spec: docs/planning/pipeline/active/WORK-scoped-announcements-v1.spec.md
---

> **Promoted (2026-06-10):** Candidate Future Tickets item 2 became ticket
> #27 (`WORK-scoped-announcements-v1`) — the notification API + scoped
> delivery, WITHOUT the Area hierarchy. Items 1 and 3–5 (Area scopes,
> bulletin boards, region event scheduler, tray/board UI) remain future
> candidates of this intake.

# INTAKE-announcements-notifications-and-area-scopes

## Summary

Capture the future direction for announcements, bulletin boards, region events,
and scoped notifications.

The player should be notified when something relevant happens in the world scope
they occupy: a town warning, a regional event, a shop notice, a faction bulletin,
or a DM/LLM-driven announcement. This should work in single-player now and remain
compatible with multiplayer/DM sessions later.

## Ideas Captured

- Announcement system: authored or dynamic messages that can appear in the event
  feed, a notification tray, a bulletin board, or a region board.
- Bulletin boards: persistent/local message surfaces players can inspect in a
  town, guild hall, shop, tavern, outpost, or region hub.
- Region events: changes scoped to a region/subregion/area, such as raids,
  festivals, shortages, wars, boss attacks, market changes, or oath consequences.
- Notification API: a server-authoritative way to deliver events only to players
  whose current scope should hear/see them.
- DM/LLM future seam: scripted, human-DM, or LLM-DM systems can create
  announcements without bypassing the normal event lifecycle.

## Spatial Scope Direction

Do not make every small place a top-level `region`.

Preferred mental model:

```text
World
  Region          large geographic / political / rules scope
    Subregion     district, wilderness zone, tower, town section
      Area        optional future nested scope: shop, tavern, house, dungeon wing
        Room/Cell current grid cell the player can occupy
```

Current code has `region`, `subregion`, and room/cell coordinates. A shop is
currently a room/cell inside a subregion. Conceptually, though, a shop should
become an **area scope** when it needs local rules, announcements, interiors,
access control, shop behavior, or its own bulletin board.

An `Area` can contain many rooms/cells. For example:

```text
World
  Region: Hollowmere
    Subregion: Hollowmere Town
      Area: Mara's Candle Shop
        Room/Cell: Sales Floor
        Room/Cell: Counter
        Room/Cell: Back Room
        Room/Cell: Wall / Door / Storage cells
```

This keeps buildings and small places self-aware without promoting every shop,
tavern, house, or dungeon chamber into a top-level region.

This lets the game say:

- "Everyone in Hollowmere hears the bell alarm."
- "Everyone in Hollowmere Town sees the market notice."
- "Only players inside Mara's Candle Shop see the shop ledger update."
- "Players within a radius hear the explosion."
- "A DM broadcasts an announcement to all players in the tower."
- "Mara yells from inside the shop and only nearby players hear it."

## Candidate Notification Scopes

- `world`: all players/sessions.
- `region`: all players in a region.
- `subregion`: all players in a district/zone.
- `area`: future nested scope such as a shop, tavern, house, camp, outpost, or
  dungeon wing.
- `room`: current exact grid cell.
- `radius`: players within X cells of an origin, using the spatial-awareness
  model.
- `actor`: a direct notification to one player/party/NPC.

## Speech And Noise Radius

Announcements do not always come from the world itself. They can come from an
area, a fixture, an item, an NPC, a monster, a DM, or a scripted event. The source
and delivery radius should be explicit.

Candidate speech/noise levels:

- `whisper`: only adjacent or extremely close listeners, likely radius 1.
- `say`: normal room/nearby speech, likely same area or radius 1 depending on
  local acoustics.
- `call`: projected speech, likely radius 2.
- `yell`: loud speech, likely radius 2 or more and may cross nearby rooms/cells
  inside the same area.
- `alarm`: area/subregion/region broadcast depending on source, such as bells,
  sirens, horns, ward stones, or town criers.
- `global`: DM/system/world event broadcast.

Examples:

- NPC source: Mara yells from the sales floor; players within radius 2 hear it.
- Area source: Mara's Candle Shop posts a notice; players inside the area see it
  on the shop board.
- Region source: Hollowmere declares a curfew; all players in the region receive
  it.
- Radius source: an explosion happens at `(x, y, z)`; listeners within the
  configured radius receive the event.

This should reuse the spatial-awareness distance model for radius delivery where
possible, but it should be a separate notification-delivery layer. Spatial
awareness answers "what can I perceive nearby?" Speech/notifications answer "who
should receive this message?"

## Candidate Notification Shape

Fields to consider later:

- `id`
- `scope`
- `channel`
- `severity`
- `title`
- `body`
- `source`
- `origin`
- `radius`
- `audibility`
- `created_tick`
- `expires_tick`
- `persistent`
- `read_state`
- `actions`

The API should produce structured events first. UI rendering can choose whether
to show the message inline, in a notification tray, on a board, or as a modal.

## Candidate Future Tickets

1. Area/scope hierarchy model.
2. Notification API and scoped event delivery.
3. Bulletin board data model and inspect command.
4. Region event scheduler/hooks.
5. UI notification tray and bulletin board component.

## Notes

- This is not part of ticket #18.
- Region/subregion awareness already exists in spatial awareness, but that model
  answers "what can I perceive nearby?" Scoped notifications answer "who should
  be told when something happens?"
- Keep the server authoritative. Clients render notifications; they do not decide
  who receives them.
