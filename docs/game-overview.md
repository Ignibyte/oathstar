# Oathstar Game Overview

## Working Pitch

Oathstar is a single-player, MUD-inspired roleplaying game about exploring a ruined oath-bound city, recovering lost vows, and using language as both interface and fiction.

The player types commands into a terminal-like prompt, reads richly described room states, navigates a connected world, solves environmental problems, bargains with strange remnants, fights when necessary, and slowly restores a dead little star at the city center.

The game should feel like an old text MUD remembered through a modern desktop app: tactile, readable, mysterious, and systemic enough that the player believes the world is responding to them.

## Genre

- Single-player text-forward RPG
- MUD-inspired exploration adventure
- Turn-based command parser game
- Desktop-first Tauri app
- Local save, local world state, no required network

## Player Fantasy

The player is a lone oathbearer entering a broken city where promises have physical weight. Names, vows, memories, debts, and acts of mercy are not abstract ideas; they are materials that can open doors, bind enemies, repair machines, awaken stars, or doom settlements.

The fantasy is not just "be a hero." It is:

- Read a strange world closely.
- Learn its rules through language and consequence.
- Make promises that matter.
- Carry small objects with large moral weight.
- Become someone the city can trust.

## Design Pillars

### Language Is The Primary Tool

The prompt is not only a UI convention. It is part of the fiction. The player acts by naming intentions: take, swear, break, ask, bind, forgive, remember, offer, listen, light.

Commands should feel readable and grounded. When the parser fails, the game should fail gracefully and teach the player what kind of language it understands.

### The World Is A Machine Of Oaths

Rooms, items, NPCs, factions, enemies, and puzzles should often orbit a promise, debt, law, taboo, or ritual. The player should gradually learn that oaths are a world system, not flavor text.

### Small Spaces, Dense Consequences

The game should prefer compact regions with layered interactions over a huge map full of thin rooms. A room can change after a vow is spoken, an NPC is helped, a bell is rung, or an enemy is spared.

### Mystery Without Obscurity

The game can be strange, poetic, and mythic, but goals should remain legible. The player should usually know two or three plausible things to try next.

### Modern Comfort, Old-School Soul

The game can use clickable helpers, a visible map, inventory chips, quest/oath tracking, and save/load support. These should assist the command experience, not replace it.

## Tone

Oathstar should feel:

- Lyrical but not overwritten
- Haunting but not grimdark
- Mythic but human-scaled
- Serious with room for dry wit
- Strange in ways that become mechanically meaningful

Reference mood words:

- Ruined civic magic
- Brass, glass, ash, rain, iron, vellum
- Oaths as weather
- Mercy as a dangerous technology
- Stars as legal witnesses

## Current Prototype Premise

The current playable slice is a small proof of shape:

1. The player arrives at Starfall Gate.
2. A masked warden explains that three vows have gone dark.
3. The player recovers memory, mercy, and flame.
4. The player defeats or overcomes an oathless shade.
5. The player relights the Oathstar.

This is not necessarily the final story. It is a useful seed because it already contains the major motifs:

- A ruined city
- A central star
- Oaths as recoverable forces
- NPCs who preserve law, memory, and guilt
- Objects that operate as ritual tools
- A clear short-form win condition

## Desired Long-Term Structure

The game could be organized as a sequence of districts or chapters. Each district teaches and complicates one major oath principle.

Example structure:

1. Gatehouse: entry, basic navigation, taking, talking, examining
2. Civic Ruin: memory, records, witnesses, first oath mechanics
3. Reliquary: mercy, forgiveness, sacrifice, item transformation
4. Sanctum: flame, violence, resolve, combat alternatives
5. Market Of Debts: bargains, favors, reputation, factions
6. Flooded Court: law, loopholes, testimony, contradiction
7. Observatory: synthesis, final vows, endings

The map should grow outward from a strong hub, not sprawl immediately.

## Core Experience Loop

1. Enter or revisit a room.
2. Read the room state and notice available leads.
3. Try commands, movement, conversation, examination, or item use.
4. Change the world state.
5. Gain an object, oath, clue, relationship change, or route.
6. Return to earlier spaces with new context.
7. Resolve a local oath problem.
8. Push deeper toward the Oathstar.

## What Makes This Different From A Normal Parser Game

The design goal is not just "Zork with a fantasy skin."

Oathstar should distinguish itself through:

- Oaths as quests++: objectives with promises, witnesses, restrictions, and consequences
- A stateful world where promises create obligations
- MUD-inspired room/network readability
- Modern UI support for parser play
- Compact RPG progression without grind
- Local-first desktop feel

## Scope Guardrails

For the first real vertical slice, we should avoid:

- Procedural generation
- Large combat systems
- Crafting bloat
- Multiplayer
- Huge branching narrative trees
- AI-generated live content
- Full roguelike persistence or permadeath

The first strong target should be a polished 30-60 minute slice with:

- 10-20 rooms
- 3-5 NPCs
- 2-3 enemies or hostile obstacles
- 2 meaningful oath choices
- Several puzzles with multiple command paths
- Save/load
- A beginning, middle, and ending state

## Open Questions

- Is the protagonist defined, customizable, or mostly implied?
- Are oaths chosen from explicit options, typed freely, or discovered as verbs/items?
- Should combat be central, rare, or mostly avoidable?
- Should the game track morality, reputation, oath debt, or only concrete world state?
- Is the final game chapter-based, hub-based, or one continuous city?
- How much failure should be allowed before the game becomes frustrating?
- Should there be multiple endings based on kept and broken promises?
