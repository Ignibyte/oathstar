# Entity Model

This document describes the intended entity direction for Oathstar.

> **v1 implemented** (ticket #6): `oathstar-core::Entity` provides the base shape
> (id, name, description, aliases, `kind` = Actor/Fixture, `roles` tags,
> `inventory`, optional `dialogue` and `combat`). Roles are stored as free-form
> tags but are now parsed into a **typed `Role` vocabulary with validated contracts
> (ticket #21)** — see [Role Contracts (v1)](#role-contracts-v1-ticket-21). The
> code-behind behavior layer below remains future work.

## Core Idea

Rooms contain entities. An entity is anything the player can inspect, target, use, talk to, fight, buy from, open, bind, swear to, or otherwise interact with.

Entities should share a common base shape, then gain behavior through roles, components, and code-behind handlers.

## Base Entity Shape

Every entity should have:

- Id
- Type
- Name
- Description
- Aliases
- Attributes/tags
- Optional region or room placement
- Optional behavior/script references

Possible entity types:

- Actor
- Item
- Fixture
- Hazard
- Portal

Actors include ordinary NPCs, enemies, shopkeepers, oath witnesses, guards, ghosts, and any other person-like or creature-like entity.

## Actors

NPCs and enemies should follow the same actor model.

An actor may become hostile, friendly, conversational, mercantile, oathbound, factional, or scripted through roles and behaviors.

This means an enemy is not a separate data shape from an NPC. It is an actor with combat-related behavior.

Example actor roles:

- Conversable
- Combatant
- Shopkeeper
- OathWitness
- QuestGiver
- Trainer
- Guard
- FactionMember

## Roles And Contracts

Roles are declared capabilities. If an entity declares a role, it must provide the metadata required by that role.

Examples:

Shopkeeper role requires:

- Inventory or stock table
- Currency or trade rule
- Buy/sell behavior
- Greeting or shop text
- Conditions that allow or deny trade

Combatant role requires:

- Health or durability
- Attack profile
- Hostility state
- Defeat behavior
- Stat disclosure rules for what the player can inspect before or during combat

Conversable role requires:

- Dialogue topics or conversation handler
- Default response
- Optional memory hooks

OathWitness role requires:

- Oath ids it can witness
- Acceptance/refusal rules
- Response text for sworn, kept, and broken oaths

Contracts should eventually be validated so broken content fails early during development.

## Role Contracts (v1, ticket #21)

The first slice of the above is implemented in `oathstar-core`: a typed `Role`
vocabulary parsed from the free-form `roles` tags, validated at the world
construction boundary (`WorldDefinition::validate`). A failed contract surfaces as a
typed `WorldValidationError::RoleContractUnmet { entity_id, role, missing }`, so
broken content fails fast during development.

v1 vocabulary and contracts — the minimum each role needs *where applicable* today:

| Role (tag) | v1 contract |
|---|---|
| `talkable` (synonym `conversable`) | must be an `Actor` |
| `oath_giver` | must be an `Actor` **and** be named as some oath's `issuer_id` |
| `shopkeeper` | must be an `Actor` (shop stock/economy deferred) |
| `combatant` | must be an `Actor`; the optional `combat = { health, attack }` is the combat profile (read by combat v1, ticket #22) |
| `boss` | must be an `Actor` (a `confront` endpoint) |
| `hostile` (ticket #22) | must be an `Actor` **and** carry a `combat` profile so it can be fought; `attack` engages a hostile in a `combat_enabled` room |
| `fixture` | the `EntityKind::Fixture` classification — carries no interaction role |

Command handlers check capability through `Entity::has_role(Role)` instead of
ad-hoc string matches (e.g. `talk` uses `has_role(Role::Talkable)`, `confront` uses
`has_role(Role::Boss)`). Unknown role tags are ignored (forward-compatible), so a
new role can be authored before it is typed.

The richer per-role metadata sketched above (shop stock, a full combat profile,
oath acceptance rules) and the code-behind hooks below attach in later tickets
without changing this foundation: a role gains required metadata by extending its
contract in `validate`, and behavior by referencing a registered behavior id.

Ticket #23 added the first player-facing hostile affordances: each `NearbySnapshot`
carries an optional server-authored `threat` (its presence = hostile; it holds
`attackable` + the canonical `attack <name>` command) and `stats` (combat
disclosure — a present stat is disclosed, an absent stat is the explicit
"unknown"), gated by a new `CombatProfile.disclose_stats` flag. A generic entity
detail dialog renders them (disclosed values, else "unknown") and sends no command.
The client infers none of it locally. See `decisions.md` Decision 041.

## Code-Behind Behavior

Entities may reference code-behind behavior handlers.

This is similar to a Unity-style script concept, but safer for a data-driven game:

- Entity data stays serializable.
- Entity data references behavior ids.
- Behavior code lives in registered modules.
- Save files store entity state and behavior ids, not raw functions.
- Systems call known behavior hooks at specific times.

Possible behavior hooks:

- `onLook`
- `onEnterRoom`
- `onTake`
- `onUse`
- `onTalk`
- `onAsk`
- `onGive`
- `onAttack`
- `onDefeat`
- `onSwearOath`
- `onKeepOath`
- `onBreakOath`
- `onTurn`

The handler can inspect game state and return messages, state changes, or command outcomes.

## Why Not Put Functions Directly In Data?

Raw functions attached directly to entity objects are convenient early, but they become awkward for:

- Save/load
- Validation
- Modding or content tooling
- Testing
- Serialization
- Future data-file authoring

The preferred direction is data plus behavior references. During early prototyping, JavaScript modules can still export entities and behavior registries, but the conceptual split should remain clear.

## Example Sketch

```js
{
  id: "maraCandlekeep",
  type: "actor",
  name: "Mara Candlekeep",
  aliases: ["mara", "shopkeeper", "candlekeep"],
  description: "A severe woman with wax on her sleeves and a ledger chained to her belt.",
  roles: ["conversable", "shopkeeper", "oathWitness"],
  attributes: ["human", "merchant", "wary"],
  behaviors: ["maraShop", "maraOathWitness"],
  shop: {
    currency: "rings",
    stock: ["blackCandle", "saltThread", "blankWarrant"]
  },
  conversation: {
    defaultTopic: "work",
    topics: ["work", "oaths", "flooded court"]
  },
  oathWitness: {
    oathIds: ["oathOfHonestTrade"]
  }
}
```

## Open Questions

- Should items and fixtures use the same role system as actors?
- Should behavior hooks be synchronous only, or can they trigger queued events?
- Should entities support inheritance/templates, or should composition be enough?
- How early should we add schema validation?
- Should behaviors be authored in TypeScript once the model stabilizes?
