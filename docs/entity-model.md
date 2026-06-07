# Entity Model

This document describes the intended entity direction for Oathstar.

> **v1 implemented** (ticket #6): `oathstar-core::Entity` provides the base shape
> (id, name, description, aliases, `kind` = Actor/Fixture, `roles` tags,
> `inventory`). Roles are free-form tags in v1; role contracts and the code-behind
> behavior layer below are future work.

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

Conversable role requires:

- Dialogue topics or conversation handler
- Default response
- Optional memory hooks

OathWitness role requires:

- Oath ids it can witness
- Acceptance/refusal rules
- Response text for sworn, kept, and broken oaths

Contracts should eventually be validated so broken content fails early during development.

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
