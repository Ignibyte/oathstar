# Region Standing

Oathstar should track how regions feel about the player in a coarse, readable way.

This is not meant to be a fiddly reputation spreadsheet. The player should usually understand the social state of a region at a glance: you are welcome, tolerated, disliked, or in real trouble.

## Core Direction

Region standing is affected by:

- Completing oaths tied to that region
- Breaking oaths witnessed by that region
- Helping or harming important regional actors
- Resolving region problems
- Grinding or fighting hostile regional threats
- Killing protected actors, if that ever becomes possible
- Boss outcomes

## Standing Shape

First version should use coarse states, not a fine numeric meter.

Suggested initial states:

- `unknown`
- `neutral`
- `liked`
- `disliked`
- `hostile`

We can keep the UI even simpler by presenting this as liked/not liked unless a region is actively hostile.

## Why Regions Instead Of Every NPC?

Regions are a better first layer because they are easier to reason about and easier to balance.

NPCs can still remember personal interactions, but region standing answers broader questions:

- Do shops trust you here?
- Do guards block you?
- Do witnesses agree to hear your oath?
- Do hostile encounters become more common?
- Are certain endings or routes available?

## Possible Effects

Liked regions might:

- Offer better prices
- Open shortcuts
- Provide safer rest
- Reveal rumors
- Witness stronger oaths
- Reduce hostile encounters

Disliked regions might:

- Refuse trade
- Increase guard hostility
- Lock certain doors
- Withhold oath witnessing
- Increase encounter pressure
- Change boss or faction dialogue

## Design Boundary

Region standing should not replace authored consequences. It is a broad social layer. Important NPCs and oaths can still have custom state.

## Open Questions

- Should standing be visible in the UI from the start?
- Should standing be purely state-based, or backed by hidden points that map to states?
- Can grinding improve standing in some regions?
- Can a region forgive broken oaths?
- Does standing affect enemy spawn rates?
