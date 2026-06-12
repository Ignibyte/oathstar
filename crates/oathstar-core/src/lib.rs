pub mod awareness;
pub mod command;

use std::collections::{BTreeMap, BTreeSet};

use awareness::{AwarenessKind, RadiusConfig};
use command::{parse, Command, Direction};
use oathstar_protocol::{
    CombatOutcome, CombatSnapshot, CombatantSnapshot, CommandRequest, CommandResponse,
    EquippedItemSnapshot, EventChannel, GameEvent, GameEventKind, GameSnapshot, MapRoomSnapshot,
    MapSnapshot, NearbySnapshot, NearbyStatsSnapshot, NearbyThreatSnapshot, OathSnapshot,
    OathStatus, OutputComponent, PackItemSnapshot, PlayerSnapshot, RoomSnapshot,
};
use serde::{Deserialize, Serialize};

// Re-exported (and used throughout) so core dependents — content tests,
// future trigger authors — can name the severity carried by
// [`AuthoredAnnouncement`] without a direct protocol dependency (ticket #27).
pub use oathstar_protocol::AnnouncementSeverity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldDefinition {
    pub id: String,
    pub title: String,
    pub start_room_id: String,
    pub rooms: BTreeMap<String, RoomDefinition>,
    #[serde(default)]
    pub regions: BTreeMap<String, RegionDefinition>,
    #[serde(default)]
    pub subregions: BTreeMap<String, SubregionDefinition>,
    #[serde(default)]
    pub entities: BTreeMap<String, Entity>,
    #[serde(default)]
    pub items: BTreeMap<String, Item>,
    /// Oaths this module defines, by id. The player swears the one named by
    /// [`WorldDefinition::oath_id`].
    #[serde(default)]
    pub oaths: BTreeMap<String, OathDefinition>,
    /// The id of the oath this module offers (the `swear` command swears it), or
    /// `None` for a module with no oath. Validated to exist in `oaths`.
    #[serde(default)]
    pub oath_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDefinition {
    pub id: String,
    pub title: String,
    pub region: String,
    pub subregion: Option<String>,
    pub description: String,
    pub exits: BTreeMap<String, String>,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub glyph: char,
    pub passable: bool,
    /// Ids of entities placed in this room (entity *placement* by reference).
    #[serde(default)]
    pub entities: Vec<String>,
    /// Ids of items lying in this room (item *room-placement* by reference).
    #[serde(default)]
    pub items: Vec<String>,
    /// Whether combat may start in this room (ticket #22). Additive and authored;
    /// `#[serde(default)]` keeps every existing room non-combat (`false`), so
    /// `attack` is refused here unless a module opts the room in. Engine-only —
    /// not surfaced in the room snapshot.
    #[serde(default)]
    pub combat_enabled: bool,
}

/// A top-level region of the world. Rooms reference a region by id; the registry
/// lets region-level systems (laws, hazards, labels) attach later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionDefinition {
    pub id: String,
    pub name: String,
}

/// A subregion within a [`RegionDefinition`]. Rooms may reference a subregion by
/// id; `region` is the parent region's id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubregionDefinition {
    pub id: String,
    pub name: String,
    pub region: String,
}

/// What kind of thing an [`Entity`] is. Actors are person/creature-like (NPCs and
/// enemies alike — the difference is *roles*, not kind); fixtures are
/// interactable objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Actor,
    Fixture,
}

/// A world entity — one shared shape for NPCs, enemies, and interactables.
///
/// NPCs and enemies are both `Actor`s, distinguished by their `roles` (declared
/// capability tags such as `"conversable"` or `"combatant"`); interactables are
/// `Fixture`s. `inventory` holds the ids of items the entity owns. Behavior
/// dispatch and role contracts are intentionally outside v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub kind: EntityKind,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub inventory: Vec<String>,
    /// Reveal-rule placeholder (ticket #17): when `true`, this entity is not
    /// surfaced by proximity/awareness queries ([`awareness::perceive`]). Future
    /// stealth/perception will compute this; v1 reads the static flag. Defaults
    /// to visible.
    #[serde(default)]
    pub hidden: bool,
    /// Authored conversation lines (ticket #19). `None` for an NPC with no
    /// scripted dialogue (which falls back to a generic talk reply); when present,
    /// `talk` returns these lines, selected by oath state for an oath-giver.
    #[serde(default)]
    pub dialogue: Option<EntityDialogue>,
    /// Authored combat stats (ticket #21). Optional for a bare `combatant`;
    /// REQUIRED by the `hostile` (ticket #22) and `boss` (ticket #29) role
    /// contracts, whose encounters copy these stats into [`CombatState`].
    #[serde(default)]
    pub combat: Option<CombatProfile>,
}

/// Authored combat stats for a `combatant`/`hostile` entity (ticket #21,
/// consumed by the combat loop in #22).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatProfile {
    /// Hit points — the combatant's starting and maximum health.
    pub health: u32,
    /// Damage this combatant deals to the player per return strike (ticket #22).
    /// Additive and authored; `#[serde(default)]` keeps an older
    /// `combat = { health = N }` (no `attack`) valid, defaulting to 0 (a
    /// combatant that deals no damage).
    #[serde(default)]
    pub attack: u32,
    /// Whether the server discloses this combatant's stats to nearby inspection
    /// (ticket #23). `#[serde(default)]` ⇒ false (hidden — rendered as "unknown")
    /// unless a module opts in. Does not affect combat resolution; read only by the
    /// nearby/snapshot disclosure.
    #[serde(default)]
    pub disclose_stats: bool,
    /// Authored XP awarded to the player when this combatant falls to a combat
    /// victory (ticket #26). `#[serde(default)]` ⇒ 0, so older profiles stay
    /// valid and a missing reward is zero — never invented (REQ-002).
    #[serde(default)]
    pub xp: u64,
    /// Authored coins awarded on a combat victory (ticket #34) — the economy's
    /// faucet, `xp`'s exact template: serde-defaulted, saturating award,
    /// never invented when absent.
    #[serde(default)]
    pub coins: u64,
}

/// A typed interaction capability declared by an entity's role tags (ticket #21).
///
/// Parsed from the free-form [`Entity::roles`] strings so content stays additive
/// (Decision 004). `fixture` is the [`EntityKind::Fixture`] classification, not a
/// role — the contract validator rejects any interaction role on a non-actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Can be addressed with `talk` (tag `"talkable"`, or the synonym `"conversable"`).
    Talkable,
    /// Offers a swearable oath (tag `"oath_giver"`); must be named as some oath's issuer.
    OathGiver,
    /// Buys/sells (tag `"shopkeeper"`); shop metadata is a future ticket.
    Shopkeeper,
    /// Can fight (tag `"combatant"`); the optional [`CombatProfile`] is the future hook.
    Combatant,
    /// A `confront` endpoint (tag `"boss"`). Its contract requires a
    /// [`CombatProfile`] so the encounter can start (ticket #29).
    Boss,
    /// A hostile that `attack` engages to start combat (tag `"hostile"`, ticket
    /// #22). Its contract requires a [`CombatProfile`] so it can be fought.
    Hostile,
}

impl Role {
    /// The canonical role tag → typed role. `"conversable"` is accepted as a
    /// synonym for `talkable` (existing content + fixtures use it). An unknown tag
    /// returns `None` and is ignored by validation (forward-compatible).
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "talkable" | "conversable" => Some(Self::Talkable),
            "oath_giver" => Some(Self::OathGiver),
            "shopkeeper" => Some(Self::Shopkeeper),
            "combatant" => Some(Self::Combatant),
            "boss" => Some(Self::Boss),
            "hostile" => Some(Self::Hostile),
            _ => None,
        }
    }

    /// The canonical tag for this role (for diagnostics / error text).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Talkable => "talkable",
            Self::OathGiver => "oath_giver",
            Self::Shopkeeper => "shopkeeper",
            Self::Combatant => "combatant",
            Self::Boss => "boss",
            Self::Hostile => "hostile",
        }
    }
}

impl Entity {
    /// Whether this entity declares `role`, parsed from its role tags (ticket #21).
    /// The typed replacement for ad-hoc `roles.iter().any(|r| r == "…")` checks.
    #[must_use]
    pub fn has_role(&self, role: Role) -> bool {
        self.roles_typed().any(|declared| declared == role)
    }

    /// The typed roles this entity declares; unknown tags are skipped.
    fn roles_typed(&self) -> impl Iterator<Item = Role> + '_ {
        self.roles.iter().filter_map(|tag| Role::from_tag(tag))
    }
}

/// A world item — leaf content data. Placement is by reference *from* a container
/// (a room's `items` for ground placement, or an entity's `inventory` for
/// ownership), so rooms never inline full item state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Reveal-rule placeholder (ticket #17): when `true`, this item is not
    /// surfaced by proximity/awareness queries ([`awareness::perceive`]).
    /// Defaults to visible.
    #[serde(default)]
    pub hidden: bool,
    /// A coarse kind/type placeholder (ticket #20), e.g. `"light"` or `"quest"`.
    /// `None` falls back to the generic `"item"` in the pack snapshot. Authored
    /// content data; the engine never invents it. No taxonomy semantics in v1.
    #[serde(default)]
    pub kind: Option<String>,
    /// Basic authored item flags (ticket #20), e.g. `["oath"]` — a small free-text
    /// tag list. No equipment/weight/rarity/stacking semantics in v1.
    #[serde(default)]
    pub flags: Vec<String>,
    /// Authored trade value in coins (ticket #34): the buy price; the sell
    /// price is `max(1, value / 2)`. `0` (the default) means priceless — a
    /// vendor will neither sell nor buy it, so flavor items stay out of the
    /// economy unless a module opts them in.
    #[serde(default)]
    pub value: u64,
    /// What this item does when worn (ticket #35): `Some` makes it equipment
    /// with an authored slot and stat mods, in the [`CombatProfile`] table
    /// idiom (`equipment = { slot = "weapon", attack = 2 }`). `None` (the
    /// default, and absent from old saves) means the item cannot be equipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equipment: Option<EquipmentProfile>,
}

/// The slot a piece of equipment occupies (ticket #35).
///
/// Authored as lowercase strings in TOML (`"weapon"` / `"armor"`), so an
/// invalid slot fails the module at parse time. v1's two active slots; the
/// client's other gear-panel slots stay decorative until a future ticket
/// adds more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EquipSlot {
    Weapon,
    Armor,
}

impl EquipSlot {
    /// The lowercase wire/author-facing name (`"weapon"` / `"armor"`) — also
    /// what `unequip <slot>` matches against.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Weapon => "weapon",
            Self::Armor => "armor",
        }
    }
}

/// An item's authored equipment behavior (ticket #35).
///
/// Names the slot it fills plus its stat mods. Mods author as unsigned (no
/// cursed/negative gear in v1) and default to 0, so
/// `equipment = { slot = "armor", defense = 1 }` is complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentProfile {
    pub slot: EquipSlot,
    /// Added to every player strike while equipped in the weapon slot.
    #[serde(default)]
    pub attack: u32,
    /// Subtracted from every incoming hit (floored at 0 damage dealt) while
    /// equipped in the armor slot.
    #[serde(default)]
    pub defense: u32,
}

/// A swearable oath defined by a module — leaf content data.
///
/// The engine records the player's progress against it as oath state on the
/// [`GameState`]; the `title`/`description` are the promise text shown when the
/// oath is sworn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OathDefinition {
    pub id: String,
    pub title: String,
    pub description: String,
    /// The entity id of the oath-giver who offers this oath (ticket #19). `None`
    /// for an oath swearable without an offer; when set, it is validated to name a
    /// real entity at construction and must have been offered before `swear`.
    #[serde(default)]
    pub issuer_id: Option<String>,
    /// Free-text origin (e.g. a region or faction id) recorded for future
    /// oath-giver UI and region/faction effects (ticket #19). Authored, optional.
    #[serde(default)]
    pub source: Option<String>,
    /// Announcements emitted when this oath is fulfilled (ticket #27) — each
    /// is delivered only if the player's location matches its scope. Empty —
    /// and absent from TOML — for oaths that announce nothing.
    #[serde(default)]
    pub fulfillment_announcements: Vec<AuthoredAnnouncement>,
    /// The item whose recovery fulfills this oath (ticket #29): taking it
    /// while the oath is sworn flips the oath to fulfilled. `None` for an
    /// oath with no recoverable objective (valid, but nothing fulfills it).
    /// Validated at construction to name a real item.
    #[serde(default)]
    pub objective_item_id: Option<String>,
}

/// Where an announcement is heard (ticket #27): the delivery scopes from the
/// announcements intake, minus the deferred Area level.
///
/// Serde-authorable in module TOML (externally tagged): `scope = "world"`,
/// `scope = { region = "hollowmere" }`, or
/// `scope = { radius = { room_id = "bell_eater_roost", radius = 2 } }`.
/// Authored scope ids are validated at construction
/// ([`WorldValidationError::AnnouncementScopeMissing`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementScope {
    /// Every listener, everywhere.
    World,
    /// Listeners whose current room lies in this region.
    Region(String),
    /// Listeners whose current room lies in this subregion.
    Subregion(String),
    /// Listeners standing in exactly this room.
    Room(String),
    /// Listeners within `radius` cells of the origin room, on the same
    /// awareness plane (region + subregion + z) — the ticket #17 spatial
    /// model, reused as a delivery rule rather than a perception query.
    Radius { room_id: String, radius: u32 },
}

/// An authored announcement (ticket #27): what a content trigger says, how
/// loudly, and to which scope.
///
/// v1's only carrier is [`OathDefinition::fulfillment_announcements`]; future
/// triggers (room entry, schedulers, DM routes) reuse the same shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoredAnnouncement {
    /// Who receives it.
    pub scope: AnnouncementScope,
    /// How loudly it presents.
    pub severity: AnnouncementSeverity,
    /// The message line, exactly as the feed shows it.
    pub text: String,
}

/// Authored conversation lines for a conversable NPC (ticket #19).
///
/// `greeting` is the default reply; the optional `oath` block carries the lines
/// an oath-giver speaks, selected by the player's oath state. Dialogue stays
/// command-based (no trees) per the first-slice direction in
/// `docs/mechanics-and-systems.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityDialogue {
    /// Said when talked to and no oath-state line applies.
    pub greeting: String,
    /// Oath-flow lines, present when this NPC issues an oath.
    #[serde(default)]
    pub oath: Option<OathDialogue>,
}

/// The lines an oath-giver speaks across the oath's lifecycle (ticket #19),
/// selected by the player's oath state when this NPC issues the designated oath.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OathDialogue {
    /// Spoken while the oath is unsworn — introduces the problem and offers the
    /// oath; talking records the offer so `swear` becomes permitted.
    pub offer: String,
    /// Spoken while the player's oath is sworn but not yet fulfilled.
    pub sworn: String,
    /// Spoken once the oath is fulfilled.
    pub fulfilled: String,
}

/// Why an out-of-spec [`WorldDefinition`] was rejected at construction.
///
/// Returned by [`WorldDefinition::validate`] and [`Engine::try_new`] so that
/// malformed module data surfaces as a typed error at the construction boundary
/// instead of a later panic deep in the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldValidationError {
    /// `start_room_id` is not a key in `rooms`.
    StartRoomMissing { start_room_id: String },
    /// The start room exists but is not passable; the player could never stand in it.
    StartRoomImpassable { start_room_id: String },
    /// A room exit points at a room id that does not exist in the world.
    DanglingExit {
        room_id: String,
        direction: String,
        target_room_id: String,
    },
    /// A room is stored under a map key that differs from its own `id`.
    RoomKeyMismatch { key: String, room_id: String },
    /// A room references a region id that is not in the region registry.
    RoomRegionMissing { room_id: String, region: String },
    /// A room references a subregion id that is not in the subregion registry.
    RoomSubregionMissing { room_id: String, subregion: String },
    /// A subregion references a parent region id that does not exist.
    SubregionRegionMissing {
        subregion_id: String,
        region: String,
    },
    /// A room's own region disagrees with the parent region of its subregion.
    RoomSubregionRegionMismatch {
        room_id: String,
        room_region: String,
        subregion: String,
        subregion_region: String,
    },
    /// A room places an entity id that is not in the entity registry.
    RoomEntityMissing { room_id: String, entity_id: String },
    /// A room places an item id that is not in the item registry.
    RoomItemMissing { room_id: String, item_id: String },
    /// An entity owns an item id that is not in the item registry.
    EntityItemMissing { entity_id: String, item_id: String },
    /// `oath_id` designates an oath that is not in the oath registry.
    OathMissing { oath_id: String },
    /// An oath names an `issuer_id` that is not in the entity registry.
    OathIssuerMissing { oath_id: String, issuer_id: String },
    /// An oath's authored announcement names a scope id that is not in the
    /// corresponding registry (ticket #27).
    AnnouncementScopeMissing {
        oath_id: String,
        scope_kind: String,
        id: String,
    },
    /// An oath's `objective_item_id` names an item that is not in the item
    /// registry (ticket #29).
    OathObjectiveMissing { oath_id: String, item_id: String },
    /// An entity declares a role whose v1 contract is unmet (ticket #21): names the
    /// entity, the role, and the missing requirement.
    RoleContractUnmet {
        entity_id: String,
        role: String,
        missing: String,
    },
}

impl std::fmt::Display for WorldValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StartRoomMissing { start_room_id } => {
                write!(f, "start room '{start_room_id}' does not exist")
            }
            Self::StartRoomImpassable { start_room_id } => {
                write!(f, "start room '{start_room_id}' is not passable")
            }
            Self::DanglingExit {
                room_id,
                direction,
                target_room_id,
            } => write!(
                f,
                "room '{room_id}' exit '{direction}' points to missing room '{target_room_id}'"
            ),
            Self::RoomKeyMismatch { key, room_id } => {
                write!(
                    f,
                    "room stored under key '{key}' has mismatched id '{room_id}'"
                )
            }
            Self::RoomRegionMissing { room_id, region } => {
                write!(f, "room '{room_id}' references missing region '{region}'")
            }
            Self::RoomSubregionMissing { room_id, subregion } => {
                write!(
                    f,
                    "room '{room_id}' references missing subregion '{subregion}'"
                )
            }
            Self::SubregionRegionMissing {
                subregion_id,
                region,
            } => write!(
                f,
                "subregion '{subregion_id}' references missing region '{region}'"
            ),
            Self::RoomSubregionRegionMismatch {
                room_id,
                room_region,
                subregion,
                subregion_region,
            } => write!(
                f,
                "room '{room_id}' (region '{room_region}') references subregion '{subregion}' whose parent region is '{subregion_region}'"
            ),
            Self::RoomEntityMissing { room_id, entity_id } => {
                write!(
                    f,
                    "room '{room_id}' references missing entity '{entity_id}'"
                )
            }
            Self::RoomItemMissing { room_id, item_id } => {
                write!(f, "room '{room_id}' references missing item '{item_id}'")
            }
            Self::EntityItemMissing { entity_id, item_id } => {
                write!(
                    f,
                    "entity '{entity_id}' references missing item '{item_id}'"
                )
            }
            Self::OathMissing { oath_id } => {
                write!(f, "designated oath '{oath_id}' does not exist")
            }
            Self::OathIssuerMissing { oath_id, issuer_id } => {
                write!(f, "oath '{oath_id}' references missing issuer '{issuer_id}'")
            }
            Self::AnnouncementScopeMissing {
                oath_id,
                scope_kind,
                id,
            } => write!(
                f,
                "oath '{oath_id}' announces to missing {scope_kind} '{id}'"
            ),
            Self::OathObjectiveMissing { oath_id, item_id } => {
                write!(f, "oath '{oath_id}' names missing objective item '{item_id}'")
            }
            Self::RoleContractUnmet {
                entity_id,
                role,
                missing,
            } => write!(
                f,
                "entity '{entity_id}' declares role '{role}' but is missing {missing}"
            ),
        }
    }
}

impl std::error::Error for WorldValidationError {}

impl WorldDefinition {
    /// Check every world invariant the engine relies on.
    ///
    /// # Errors
    /// Returns a [`WorldValidationError`] when a room is stored under a key that
    /// differs from its own id, the start room is missing or not passable, any
    /// room exit points to a room that does not exist, or any room/subregion/
    /// entity references a missing region, subregion, entity, or item.
    pub fn validate(&self) -> Result<(), WorldValidationError> {
        for (key, room) in &self.rooms {
            if key != &room.id {
                return Err(WorldValidationError::RoomKeyMismatch {
                    key: key.clone(),
                    room_id: room.id.clone(),
                });
            }
        }

        let start = self.rooms.get(&self.start_room_id).ok_or_else(|| {
            WorldValidationError::StartRoomMissing {
                start_room_id: self.start_room_id.clone(),
            }
        })?;

        if !start.passable {
            return Err(WorldValidationError::StartRoomImpassable {
                start_room_id: self.start_room_id.clone(),
            });
        }

        for room in self.rooms.values() {
            for (direction, target_room_id) in &room.exits {
                if !self.rooms.contains_key(target_room_id) {
                    return Err(WorldValidationError::DanglingExit {
                        room_id: room.id.clone(),
                        direction: direction.clone(),
                        target_room_id: target_room_id.clone(),
                    });
                }
            }
            if !self.regions.contains_key(&room.region) {
                return Err(WorldValidationError::RoomRegionMissing {
                    room_id: room.id.clone(),
                    region: room.region.clone(),
                });
            }
            if let Some(subregion) = &room.subregion {
                if !self.subregions.contains_key(subregion) {
                    return Err(WorldValidationError::RoomSubregionMissing {
                        room_id: room.id.clone(),
                        subregion: subregion.clone(),
                    });
                }
            }
            for entity_id in &room.entities {
                if !self.entities.contains_key(entity_id) {
                    return Err(WorldValidationError::RoomEntityMissing {
                        room_id: room.id.clone(),
                        entity_id: entity_id.clone(),
                    });
                }
            }
            for item_id in &room.items {
                if !self.items.contains_key(item_id) {
                    return Err(WorldValidationError::RoomItemMissing {
                        room_id: room.id.clone(),
                        item_id: item_id.clone(),
                    });
                }
            }
        }

        for (subregion_id, subregion) in &self.subregions {
            if !self.regions.contains_key(&subregion.region) {
                return Err(WorldValidationError::SubregionRegionMissing {
                    subregion_id: subregion_id.clone(),
                    region: subregion.region.clone(),
                });
            }
        }

        // Room↔subregion region coherence (ticket #11): a room's own region must
        // match the parent region of its subregion. Checked after the subregion
        // loop so a *missing* parent region surfaces as `SubregionRegionMissing`,
        // not a mismatch. Subregion existence is already validated in the room loop.
        for room in self.rooms.values() {
            let Some(subregion_id) = &room.subregion else {
                continue;
            };
            if let Some(subregion) = self.subregions.get(subregion_id) {
                if subregion.region != room.region {
                    return Err(WorldValidationError::RoomSubregionRegionMismatch {
                        room_id: room.id.clone(),
                        room_region: room.region.clone(),
                        subregion: subregion_id.clone(),
                        subregion_region: subregion.region.clone(),
                    });
                }
            }
        }

        for (entity_id, entity) in &self.entities {
            for item_id in &entity.inventory {
                if !self.items.contains_key(item_id) {
                    return Err(WorldValidationError::EntityItemMissing {
                        entity_id: entity_id.clone(),
                        item_id: item_id.clone(),
                    });
                }
            }
        }

        self.validate_oaths()?;
        self.validate_entity_contracts()?;

        Ok(())
    }

    /// Validate oath invariants: a designated `oath_id` must name a known oath,
    /// and every oath's optional `issuer_id` must name a known entity (ticket #19,
    /// Decision 030). Split out of [`Self::validate`] so each validator stays one
    /// focused concern.
    ///
    /// # Errors
    /// [`WorldValidationError::OathMissing`] for a dangling designated oath;
    /// [`WorldValidationError::OathIssuerMissing`] for an oath whose `issuer_id`
    /// is not a registered entity.
    fn validate_oaths(&self) -> Result<(), WorldValidationError> {
        if let Some(oath_id) = &self.oath_id {
            if !self.oaths.contains_key(oath_id) {
                return Err(WorldValidationError::OathMissing {
                    oath_id: oath_id.clone(),
                });
            }
        }

        for (oath_id, oath) in &self.oaths {
            if let Some(issuer_id) = &oath.issuer_id {
                if !self.entities.contains_key(issuer_id) {
                    return Err(WorldValidationError::OathIssuerMissing {
                        oath_id: oath_id.clone(),
                        issuer_id: issuer_id.clone(),
                    });
                }
            }

            // Ticket #27: every authored announcement scope id must resolve in
            // its registry — fail fast on broken content, like the entity and
            // item contracts.
            for announcement in &oath.fulfillment_announcements {
                let missing = match &announcement.scope {
                    AnnouncementScope::World => None,
                    AnnouncementScope::Region(id) => {
                        (!self.regions.contains_key(id)).then_some(("region", id))
                    }
                    AnnouncementScope::Subregion(id) => {
                        (!self.subregions.contains_key(id)).then_some(("subregion", id))
                    }
                    AnnouncementScope::Room(id) | AnnouncementScope::Radius { room_id: id, .. } => {
                        (!self.rooms.contains_key(id)).then_some(("room", id))
                    }
                };
                if let Some((scope_kind, id)) = missing {
                    return Err(WorldValidationError::AnnouncementScopeMissing {
                        oath_id: oath_id.clone(),
                        scope_kind: scope_kind.to_string(),
                        id: id.clone(),
                    });
                }
            }

            // Ticket #29: a recoverable objective must name a real item, so
            // the fulfillment-on-take hook can never dangle.
            if let Some(item_id) = &oath.objective_item_id {
                if !self.items.contains_key(item_id) {
                    return Err(WorldValidationError::OathObjectiveMissing {
                        oath_id: oath_id.clone(),
                        item_id: item_id.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate entity role contracts (ticket #21, Decision 004): an interaction
    /// role must be on an `Actor`, and an `oath_giver` must be named as some oath's
    /// issuer. Unknown role tags are ignored (forward-compatible). Split out so
    /// `validate` stays one focused orchestrator.
    ///
    /// # Errors
    /// [`WorldValidationError::RoleContractUnmet`] naming the entity, role, and the
    /// missing requirement.
    fn validate_entity_contracts(&self) -> Result<(), WorldValidationError> {
        for (entity_id, entity) in &self.entities {
            for role in entity.roles_typed() {
                if entity.kind != EntityKind::Actor {
                    return Err(WorldValidationError::RoleContractUnmet {
                        entity_id: entity_id.clone(),
                        role: role.as_str().to_string(),
                        missing: "an Actor kind (interaction roles require an actor)".to_string(),
                    });
                }
                if role == Role::OathGiver
                    && !self
                        .oaths
                        .values()
                        .any(|oath| oath.issuer_id.as_deref() == Some(entity_id.as_str()))
                {
                    return Err(WorldValidationError::RoleContractUnmet {
                        entity_id: entity_id.clone(),
                        role: role.as_str().to_string(),
                        missing: "an oath whose issuer_id names this entity".to_string(),
                    });
                }
                if role == Role::Hostile && entity.combat.is_none() {
                    return Err(WorldValidationError::RoleContractUnmet {
                        entity_id: entity_id.clone(),
                        role: role.as_str().to_string(),
                        missing: "a combat profile (health) so the hostile can be fought"
                            .to_string(),
                    });
                }
                if role == Role::Boss && entity.combat.is_none() {
                    return Err(WorldValidationError::RoleContractUnmet {
                        entity_id: entity_id.clone(),
                        role: role.as_str().to_string(),
                        missing: "a combat profile (health) so the boss can be confronted"
                            .to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

/// The player's progress against a sworn oath — the engine's oath state, mapped
/// to an [`oathstar_protocol::OathSnapshot`] for the view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OathProgress {
    pub oath_id: String,
    pub title: String,
    pub status: OathStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub tick: u64,
    pub current_room_id: String,
    pub discovered_rooms: BTreeSet<String>,
    pub player: PlayerState,
    /// The player's oath once sworn; `None` until the `swear` command succeeds.
    #[serde(default)]
    pub oath: Option<OathProgress>,
    /// Ids of items the player is carrying (ticket #18). Minimal carried-item
    /// state: pickup-ordered item ids, resolved to names from `world.items` at
    /// snapshot time. Empty until a `take` succeeds; `#[serde(default)]` keeps an
    /// older saved state (without a `pack`) loadable.
    #[serde(default)]
    pub pack: Vec<String>,
    /// The oath the player has been offered but not yet sworn (ticket #19). Set
    /// when the player talks to the designated oath's issuer; `swear` requires it
    /// to match the designated oath before an issuer-offered oath can be sworn.
    /// `#[serde(default)]` keeps older saved state (without it) loadable.
    #[serde(default)]
    pub offered_oath_id: Option<String>,
    /// The active combat encounter (ticket #22). `None` outside combat; set when
    /// `attack` starts a fight and cleared the moment it resolves (win or loss).
    /// `#[serde(default)]` keeps older saved state (without it) loadable.
    #[serde(default)]
    pub combat: Option<CombatState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: String,
    pub name: String,
    pub level: u32,
    pub xp: u64,
    pub hp: i32,
    pub max_hp: i32,
    pub focus: i32,
    pub max_focus: i32,
    /// The player's coin purse (ticket #34). `#[serde(default)]` keeps a
    /// pre-commerce v2 save loadable as a coinless player — sound state, the
    /// additive-field posture (no format bump).
    #[serde(default)]
    pub coins: u64,
    /// The item id equipped in the weapon slot (ticket #35); `None` when
    /// bare-handed. Serde-additive like `coins`: a pre-equipment save loads
    /// with empty hands. Equipped ids live here INSTEAD of in the pack, so
    /// `drop`/`sell` (which search the pack) naturally refuse equipped gear.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equipped_weapon: Option<String>,
    /// The item id equipped in the armor slot (ticket #35); see
    /// [`Self::equipped_weapon`] for the storage contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equipped_armor: Option<String>,
}

/// Damage the player deals per strike (ticket #22). Fixed and deterministic (no
/// RNG) so combat stays fully testable and mutation-killable; tunable in a later
/// balance pass.
const PLAYER_STRIKE_DAMAGE: i32 = 4;

/// The XP milestones that grant levels (ticket #30): level 1 at 0 XP, +1 per
/// threshold crossed — level 5 when the table is exhausted (v1's cap; XP keeps
/// accumulating past it). Engine-const like [`PLAYER_STRIKE_DAMAGE`]:
/// module-authored curves are a future ticket. Beginner pacing: two strays
/// (5 XP each) reach level 2; adding the boss (25) lands level 3.
const LEVEL_XP_THRESHOLDS: [u64; 4] = [10, 30, 60, 100];

/// Maximum-HP growth per level gained (ticket #30). Each level also heals to
/// the new maximum — the milestone moment.
const LEVEL_UP_MAX_HP_GROWTH: i32 = 5;

/// The sell price a vendor pays for an item of authored `value` (ticket #34):
/// half the buy price, floored at one coin so any sellable item is worth
/// something. Callers refuse zero-value items before pricing them.
const fn sell_price(value: u64) -> u64 {
    let half = value / 2;
    if half == 0 {
        1
    } else {
        half
    }
}

/// The level the deterministic curve assigns to `xp` (ticket #30): 1 plus the
/// number of [`LEVEL_XP_THRESHOLDS`] crossed. Pure, total, RNG-free — identical
/// sessions level identically, and a save-loaded XP of any magnitude maps
/// without panicking.
fn level_for_xp(xp: u64) -> u32 {
    let crossed = LEVEL_XP_THRESHOLDS
        .iter()
        .filter(|&&threshold| xp >= threshold)
        .count();
    u32::try_from(crossed).unwrap_or(u32::MAX).saturating_add(1)
}

/// World ticks between combat pulses (ticket #24, Decision 023): with the 1s
/// world tick this is the ~2s default combat pulse. Copied onto each encounter
/// at start, so per-actor variation later is a one-line copy from the profile;
/// v2 ships the single default.
const DEFAULT_COMBAT_PULSE_TICKS: u64 = 2;

/// Damage a queued power strike deals in the Phase-2 skill window (ticket
/// #25). Heavier than the baseline strike, and just as fixed/deterministic.
const POWER_STRIKE_DAMAGE: i32 = 6;

/// Focus a power strike commits when it is queued (ticket #31). Spend happens
/// at the queue — the moment of commitment — and the engine refunds it if the
/// action is replaced or the fight ends before it fires. Like
/// [`POWER_STRIKE_DAMAGE`], an engine const until module-authored skill
/// economies land.
const POWER_STRIKE_FOCUS_COST: i32 = 2;

/// Focus a guard commits when it is queued (ticket #31); same spend/refund
/// rules as [`POWER_STRIKE_FOCUS_COST`]. With a 5-point pool the player
/// affords two power strikes and a guard exactly.
const GUARD_FOCUS_COST: i32 = 1;

/// An in-progress combat encounter (ticket #22), held on [`GameState::combat`].
///
/// Server-authoritative and deterministic. Enemy stats are copied from the
/// hostile's [`CombatProfile`] when combat starts, so resolution is
/// self-contained — it survives removing the defeated entity from the room on
/// victory. The player's HP lives on [`PlayerState`]; only the enemy's HP is
/// tracked here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatState {
    /// The enemy entity's id.
    pub enemy_id: String,
    /// The enemy's display name.
    pub enemy_name: String,
    /// The enemy's current hit points (clamped at zero).
    pub enemy_hp: i32,
    /// The enemy's maximum hit points.
    pub enemy_max_hp: i32,
    /// Damage the enemy deals to the player per return strike.
    pub enemy_attack: i32,
    /// Rounds resolved so far (1 after the opening round).
    pub round: u32,
    /// The battle play-by-play lines, oldest first.
    pub log: Vec<String>,
    /// World ticks between combat pulses (ticket #24); copied from
    /// [`DEFAULT_COMBAT_PULSE_TICKS`] when combat starts.
    pub pulse_rate: u64,
    /// The absolute world tick the next combat pulse is due at (ticket #24).
    /// Re-anchored after each pulse the encounter survives; manual commands
    /// never move it, so the cadence holds (REQ-004).
    pub next_pulse_at: u64,
    /// The player's queued between-pulse action (ticket #24), resolved by the
    /// next pulse's Phase 2 skill window — `None` skips the phase cleanly.
    ///
    /// These three fields are plain (no `#[serde(default)]`): a v1 save
    /// (ticket #28) always writes the complete struct, so a payload missing
    /// one is malformed — and a defaulted `pulse_rate` of 0 would mean "pulse
    /// every tick", a wrong-by-default worse than refusing the parse.
    pub queued_action: Option<CombatAction>,
    /// A one-shot guard charge (ticket #25): set when a queued guard resolves
    /// in the Phase-2 window, consumed by the next enemy return strike from
    /// any source (pulse Phase 1 or a manual round), which it turns aside
    /// entirely. Plain like its siblings above; it dies with the encounter
    /// state on every combat end and round-trips through a mid-combat save.
    pub guard_charge: bool,
}

/// A player action queued between combat pulses (ticket #24).
///
/// Resolved in the pulse's Phase 2 (the skill window). Ticket #25 adds the
/// first direct battle verbs (`guard`, `power strike`) alongside `flee`;
/// richer authored skills become further variants when the skills/classes
/// system lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatAction {
    /// Break away from the encounter: the resolving pulse ends combat with a
    /// fled outcome — the enemy survives in place and the player keeps their
    /// current HP.
    Flee,
    /// Brace against the enemy (ticket #25): the resolving pulse arms a
    /// one-shot [`CombatState::guard_charge`] that turns the next enemy
    /// return strike — pulse or manual round — aside entirely.
    Guard,
    /// A heavier deterministic blow (ticket #25): the resolving pulse strikes
    /// the enemy for [`POWER_STRIKE_DAMAGE`], ending the fight in victory on
    /// a kill.
    PowerStrike,
}

impl CombatAction {
    /// The action's wire label, surfaced as `CombatSnapshot::queued_action`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Flee => "flee",
            Self::Guard => "guard",
            Self::PowerStrike => "power_strike",
        }
    }

    /// The refusal line when the verb is used with no battle to use it in.
    const fn queue_refusal(self) -> &'static str {
        match self {
            Self::Flee => "There is nothing to flee from.",
            Self::Guard => "There is nothing to guard against.",
            Self::PowerStrike => "There is nothing to strike at.",
        }
    }

    /// The confirmation line when the action is queued for the next window.
    const fn queue_confirmation(self) -> &'static str {
        match self {
            Self::Flee => "You watch for an opening to flee.",
            Self::Guard => "You ready your guard for the next blow.",
            Self::PowerStrike => "You wind up a power strike.",
        }
    }

    /// The no-op line when the same action is queued again (REQ-005).
    const fn queue_already(self) -> &'static str {
        match self {
            Self::Flee => "You are already watching for an opening to flee.",
            Self::Guard => "You are already set to guard.",
            Self::PowerStrike => "You are already winding up a power strike.",
        }
    }

    /// The focus the action commits when queued (ticket #31). Flee is free —
    /// the locked "free verbs stay free" rule; its cost of 0 is what keeps it
    /// queueable at any focus, crafted negatives included.
    const fn focus_cost(self) -> i32 {
        match self {
            Self::Flee => 0,
            Self::Guard => GUARD_FOCUS_COST,
            Self::PowerStrike => POWER_STRIKE_FOCUS_COST,
        }
    }

    /// The refusal line when the player cannot afford to queue the action
    /// (ticket #31). The `Flee` arm is product-unreachable while flee costs
    /// nothing (the queue gate skips zero-cost actions); it exists so the
    /// table stays uniform if flee ever prices in.
    const fn focus_refusal(self) -> &'static str {
        match self {
            Self::Flee => "You lack the focus to flee.",
            Self::Guard => "You lack the focus to guard.",
            Self::PowerStrike => "You lack the focus for a power strike.",
        }
    }
}

/// An enemy resolved for a combat entry, with its authored stats copied out
/// so [`Engine::engage_enemy`] can build a self-contained [`CombatState`].
/// Built by `attack`'s hostile resolution (ticket #22) and `confront`'s boss
/// path (ticket #29) — the struct keeps the health/attack pair unswappable.
struct ResolvedHostile {
    id: String,
    name: String,
    health: u32,
    attack: u32,
}

/// The on-disk save format version (ticket #28).
///
/// [`Engine::from_save`] rejects any other value loudly; there is no migration
/// tooling, only the version field and the refusal. Bumped to 2 at ticket #29:
/// oath fulfillment moved to the authored `objective_item_id`, so a version-1
/// save's embedded world (which lacks the field) would leave a sworn oath
/// silently unfulfillable — loud refusal is the honest posture.
pub const SAVE_FORMAT_VERSION: u32 = 2;

/// A complete saved session (ticket #28): the mutated world, the game state,
/// and the event counter, under a format version.
///
/// Produced by [`Engine::save_data`] and consumed by [`Engine::from_save`].
/// The world is the SESSION's world — placements removed, inventories cleared,
/// and items dropped in play are all captured (the #26 lesson: [`GameState`]
/// alone misses world mutations). `next_event_id` rides along so post-load
/// event ids never collide with the loaded session's own history (loading an
/// OLDER save still rewinds ids relative to the live feed — consumers key on
/// event type, not id).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    /// The format version; must equal [`SAVE_FORMAT_VERSION`] to load.
    pub version: u32,
    /// The session's (possibly mutated) world.
    pub world: WorldDefinition,
    /// The session's game state.
    pub state: GameState,
    /// The next event id the engine would allocate.
    pub next_event_id: u64,
}

/// Why a [`SaveData`] payload was rejected by [`Engine::from_save`].
///
/// A save file is untrusted input (§14): every arm is a typed refusal at the
/// load boundary, so no crafted payload can reach an engine invariant — and
/// panic — later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The payload's format version is not [`SAVE_FORMAT_VERSION`].
    VersionMismatch { found: u32, supported: u32 },
    /// The payload's world fails [`WorldDefinition::validate`].
    InvalidWorld(WorldValidationError),
    /// The payload's state references its world incoherently; accepting it
    /// would violate an engine invariant later. `what` names the offender.
    StateIncoherent { what: String },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VersionMismatch { found, supported } => write!(
                f,
                "save format version {found} is not supported (this build reads version {supported})"
            ),
            Self::InvalidWorld(error) => write!(f, "saved world failed validation: {error}"),
            Self::StateIncoherent { what } => write!(f, "saved state is incoherent: {what}"),
        }
    }
}

impl std::error::Error for LoadError {}

#[derive(Debug, Clone)]
pub struct Engine {
    world: WorldDefinition,
    state: GameState,
    next_event_id: u64,
}

impl Engine {
    /// Build an engine from a world after validating its invariants.
    ///
    /// This is the only constructor, so an `Engine` can never hold a world that
    /// would later panic (e.g. a missing or impassable start room, or an exit
    /// into a non-existent room).
    ///
    /// # Errors
    /// Returns a [`WorldValidationError`] when `world` fails
    /// [`WorldDefinition::validate`].
    pub fn try_new(world: WorldDefinition) -> Result<Self, WorldValidationError> {
        world.validate()?;

        let mut discovered_rooms = BTreeSet::new();
        discovered_rooms.insert(world.start_room_id.clone());

        let state = GameState {
            tick: 0,
            current_room_id: world.start_room_id.clone(),
            discovered_rooms,
            player: PlayerState {
                id: "player".to_string(),
                name: "Oathbearer".to_string(),
                level: 1,
                xp: 0,
                hp: 20,
                max_hp: 20,
                focus: 5,
                max_focus: 5,
                coins: 0,
                equipped_weapon: None,
                equipped_armor: None,
            },
            oath: None,
            pack: Vec::new(),
            offered_oath_id: None,
            combat: None,
        };

        Ok(Self {
            world,
            state,
            next_event_id: 1,
        })
    }

    /// Capture the complete running session as a versioned [`SaveData`]
    /// (ticket #28).
    ///
    /// Clones the session; the running engine is never mutated by a save.
    #[must_use]
    pub fn save_data(&self) -> SaveData {
        SaveData {
            version: SAVE_FORMAT_VERSION,
            world: self.world.clone(),
            state: self.state.clone(),
            next_event_id: self.next_event_id,
        }
    }

    /// Reconstruct an engine from a saved session (ticket #28).
    ///
    /// A save file is untrusted input (§14). The version is checked first, the
    /// world is re-validated through the same boundary as [`Engine::try_new`],
    /// and then the state's world references are checked against the loaded
    /// world — the current room, the active combat enemy, and the sworn oath
    /// are exactly the state-reachable engine invariants, so a crafted payload
    /// is refused here instead of panicking later. Pack and equipped-slot ids
    /// (ticket #35) are deliberately NOT validated: every consumer tolerates a
    /// dangling id (name falls back to the id, mods to 0), so they are
    /// tolerance-class references, not invariants.
    ///
    /// # Errors
    /// Returns a [`LoadError`] naming the first rejection: a version mismatch,
    /// a world validation failure, or a state/world incoherence.
    pub fn from_save(data: SaveData) -> Result<Self, LoadError> {
        if data.version != SAVE_FORMAT_VERSION {
            return Err(LoadError::VersionMismatch {
                found: data.version,
                supported: SAVE_FORMAT_VERSION,
            });
        }
        data.world.validate().map_err(LoadError::InvalidWorld)?;
        if !data.world.rooms.contains_key(&data.state.current_room_id) {
            let room = &data.state.current_room_id;
            return Err(LoadError::StateIncoherent {
                what: format!("current room '{room}' is not in the world"),
            });
        }
        if let Some(combat) = data.state.combat.as_ref() {
            if !data.world.entities.contains_key(&combat.enemy_id) {
                let enemy = &combat.enemy_id;
                return Err(LoadError::StateIncoherent {
                    what: format!("combat enemy '{enemy}' is not in the world"),
                });
            }
        }
        if let Some(oath) = data.state.oath.as_ref() {
            if !data.world.oaths.contains_key(&oath.oath_id) {
                let oath_id = &oath.oath_id;
                return Err(LoadError::StateIncoherent {
                    what: format!("sworn oath '{oath_id}' is not in the world"),
                });
            }
        }
        Ok(Self {
            world: data.world,
            state: data.state,
            next_event_id: data.next_event_id,
        })
    }

    #[must_use]
    pub fn snapshot(&self) -> GameSnapshot {
        let room = self.current_room();
        let room_snapshot = self.room_snapshot(room);
        let map = self.map_snapshot(room);

        GameSnapshot {
            world_id: self.world.id.clone(),
            world_title: self.world.title.clone(),
            tick: self.state.tick,
            current_room_id: self.state.current_room_id.clone(),
            player: PlayerSnapshot {
                id: self.state.player.id.clone(),
                name: self.state.player.name.clone(),
                level: self.state.player.level,
                xp: self.state.player.xp,
                hp: self.state.player.hp,
                max_hp: self.state.player.max_hp,
                focus: self.state.player.focus,
                max_focus: self.state.player.max_focus,
                coins: self.state.player.coins,
                equipment: self.equipment_snapshot(),
            },
            room: room_snapshot,
            map,
            oath: self.state.oath.as_ref().map(|progress| OathSnapshot {
                oath_id: progress.oath_id.clone(),
                title: progress.title.clone(),
                status: progress.status,
            }),
            pack: self.pack_snapshot(),
            combat: self.combat_snapshot(),
        }
    }

    /// Advance the world one tick and resolve a due combat pulse (ticket #24).
    ///
    /// The tick stream is the engine's only clock: the server's 1s interval
    /// calls this in real time and tests call it directly, so combat pulses
    /// are deterministic and reproducible (REQ-006) — the engine never reads
    /// wall-clock time. Returns the `Tick` event followed by any pulse events.
    pub fn tick(&mut self) -> Vec<GameEvent> {
        // Saturating: a crafted save can carry `u64::MAX` (ticket #28).
        self.state.tick = self.state.tick.saturating_add(1);
        let mut events = vec![self.event(
            EventChannel::Debug,
            GameEventKind::Tick {
                value: self.state.tick,
            },
        )];
        self.combat_pulse_if_due(&mut events);
        events
    }

    /// Resolve one combat cycle when the active encounter's pulse is due
    /// (ticket #24): emit the typed [`GameEventKind::CombatPulse`] marker, run
    /// Phase 1 (the baseline exchange — the v1 round), then Phase 2 (the
    /// queued-action skill window, a clean skip when nothing is queued), and
    /// re-anchor the next pulse if the encounter survives. A no-op outside
    /// combat or before the due tick, so idle ticking emits nothing and an
    /// ended encounter stops pulsing (REQ-001/002/005).
    fn combat_pulse_if_due(&mut self, events: &mut Vec<GameEvent>) {
        let Some(combat) = self.state.combat.as_ref() else {
            return;
        };
        if self.state.tick < combat.next_pulse_at {
            return;
        }
        let round = combat.round.saturating_add(1);
        events.push(self.event(EventChannel::Combat, GameEventKind::CombatPulse { round }));
        self.resolve_combat_round(events);
        self.resolve_queued_action(events);
        if let Some(combat) = self.state.combat.as_mut() {
            combat.next_pulse_at = self.state.tick.saturating_add(combat.pulse_rate);
        }
    }

    /// Phase 2 — the skill window (ticket #24/#25): resolve the queued
    /// between-pulse action, or skip the phase cleanly when none is queued
    /// (REQ-002/003). Runs only while the encounter survived Phase 1 — a
    /// fallen side ends the fight first, and an unreached queued action is
    /// dropped with the cleared state. The queue is consumed (`take`), so an
    /// action fires exactly once (the AD-claude-combat-pulse-rides-tick-001
    /// take-vs-peek pin).
    fn resolve_queued_action(&mut self, events: &mut Vec<GameEvent>) {
        let Some(combat) = self.state.combat.as_mut() else {
            return;
        };
        match combat.queued_action.take() {
            None => {}
            Some(CombatAction::Flee) => self.end_combat(CombatOutcome::Fled, events),
            Some(CombatAction::Guard) => self.resolve_guard(events),
            Some(CombatAction::PowerStrike) => self.resolve_power_strike(events),
        }
    }

    /// Phase-2 guard resolution (ticket #25): arm the one-shot charge that
    /// turns the next enemy return strike aside.
    fn resolve_guard(&mut self, events: &mut Vec<GameEvent>) {
        let combat = self
            .state
            .combat
            .as_mut()
            .expect("resolve_guard is only called with an active encounter");
        combat.guard_charge = true;
        let line = "You raise your guard.";
        combat.log.push(line.to_string());
        events.push(self.log(EventChannel::Combat, OutputComponent::CombatMessage, line));
    }

    /// Phase-2 power-strike resolution (ticket #25): a heavier deterministic
    /// blow; landing on exactly zero ends the fight in victory from the
    /// window (the enemy never acts inside it, so a window defeat is
    /// impossible).
    fn resolve_power_strike(&mut self, events: &mut Vec<GameEvent>) {
        // The equipped weapon swings every strike (ticket #35) — computed
        // before the `&mut combat` borrow below.
        let damage = POWER_STRIKE_DAMAGE.saturating_add(self.player_attack_bonus());
        let (line, enemy_dead) = {
            let combat = self
                .state
                .combat
                .as_mut()
                .expect("resolve_power_strike is only called with an active encounter");
            combat.enemy_hp = combat.enemy_hp.saturating_sub(damage).max(0);
            let line = format!(
                "Your power strike slams into {} for {damage} ({}/{}).",
                combat.enemy_name, combat.enemy_hp, combat.enemy_max_hp
            );
            combat.log.push(line.clone());
            (line, combat.enemy_hp <= 0)
        };
        events.push(self.log(EventChannel::Combat, OutputComponent::CombatMessage, line));
        if enemy_dead {
            self.end_combat(CombatOutcome::Victory, events);
        }
    }

    /// Recover focus by resting (ticket #31): out of combat the pool refills
    /// to its maximum in one settled breath; mid-encounter rest refuses —
    /// recovery is a between-fights decision, not a combat action. An
    /// already-full pool refuses as a no-op, and the `>=` means a crafted
    /// above-max focus (ticket #28) is tolerated, never clamped down.
    fn rest(&mut self) -> (bool, Vec<GameEvent>) {
        if self.state.combat.is_some() {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    "There is no rest in the midst of battle.",
                )],
            );
        }
        if self.state.player.focus >= self.state.player.max_focus {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    "You are already fully focused.",
                )],
            );
        }
        self.state.player.focus = self.state.player.max_focus;
        let line = format!(
            "You rest. Focus returns to you ({}/{}).",
            self.state.player.focus, self.state.player.max_focus
        );
        (
            true,
            vec![self.log(
                EventChannel::Narrative,
                OutputComponent::NarrativeMessage,
                line,
            )],
        )
    }

    /// Produce the opening scene for a freshly started game (REQ-001).
    ///
    /// A session emits no events until the player acts, so this gives a client
    /// the start room to render the moment a game begins: the typed
    /// [`GameEventKind::RoomEntered`] for the start room followed by its
    /// description, reusing the same room path as movement.
    pub fn begin(&mut self) -> Vec<GameEvent> {
        let room = self.current_room().clone();
        let mut events = vec![self.event(
            EventChannel::Room,
            GameEventKind::RoomEntered {
                room_id: room.id,
                title: room.title,
            },
        )];
        events.extend(self.describe_current_room());
        events
    }

    pub fn handle_command(&mut self, request: CommandRequest) -> CommandResponse {
        let mut events = Vec::new();

        match parse(&request.input) {
            Command::Empty => {
                events.push(self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    "The world waits for a command.",
                ));
                return self.response(false, events);
            }
            Command::Help => {
                events.push(self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    "Try: look, north, south, east, west, up, down, swear, confront, attack, flee, guard, power strike, talk, take, drop, inventory, rest, shop, buy, sell, equip, unequip.",
                ));
            }
            Command::Look { target: None } => {
                events.extend(self.describe_current_room());
            }
            Command::Look {
                target: Some(target),
            } => {
                events.extend(self.look_at(&target));
            }
            Command::Move(direction) => {
                events.extend(self.move_direction(direction));
            }
            Command::Swear => return self.acted(events, Self::swear),
            Command::Confront => return self.acted(events, Self::confront),
            Command::Attack { target } => {
                return self.acted(events, |engine| engine.attack(target.as_deref()));
            }
            Command::Flee => {
                return self.acted(events, |engine| {
                    engine.queue_combat_action(CombatAction::Flee)
                });
            }
            Command::Guard => {
                return self.acted(events, |engine| {
                    engine.queue_combat_action(CombatAction::Guard)
                });
            }
            Command::PowerStrike => {
                return self.acted(events, |engine| {
                    engine.queue_combat_action(CombatAction::PowerStrike)
                });
            }
            Command::Talk { target } => {
                return self.acted(events, |engine| engine.talk_at(&target));
            }
            Command::Take { target } => {
                return self.acted(events, |engine| engine.take_at(&target));
            }
            Command::Drop { target } => {
                return self.acted(events, |engine| engine.drop_at(&target));
            }
            Command::Inventory => {
                events.extend(self.list_pack());
            }
            Command::Rest => return self.acted(events, Self::rest),
            Command::Shop => return self.acted(events, Self::shop),
            Command::Buy { target } => {
                return self.acted(events, |engine| engine.buy_at(&target));
            }
            Command::Sell { target } => {
                return self.acted(events, |engine| engine.sell_at(&target));
            }
            Command::Equip { target } => {
                return self.acted(events, |engine| engine.equip_at(&target));
            }
            Command::Unequip { target } => {
                return self.acted(events, |engine| engine.unequip_at(&target));
            }
            Command::Unknown { input } => {
                events.push(self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("I do not know how to '{input}' yet."),
                ));
                return self.response(false, events);
            }
        }

        self.response(true, events)
    }

    /// Run an `(accepted, events)`-shaped command action and fold its events
    /// into the response — the shared tail of every acting verb arm in
    /// [`Self::handle_command`] (extracted at ticket #34 to keep that match
    /// under the clippy line ceiling, the #19/#20 recurring class).
    fn acted(
        &mut self,
        mut events: Vec<GameEvent>,
        action: impl FnOnce(&mut Self) -> (bool, Vec<GameEvent>),
    ) -> CommandResponse {
        let (accepted, action_events) = action(self);
        events.extend(action_events);
        self.response(accepted, events)
    }

    fn response(&self, accepted: bool, events: Vec<GameEvent>) -> CommandResponse {
        CommandResponse {
            accepted,
            events,
            snapshot: self.snapshot(),
        }
    }

    fn describe_current_room(&mut self) -> Vec<GameEvent> {
        let room = self.current_room().clone();
        let exits = if room.exits.is_empty() {
            "none".to_string()
        } else {
            room.exits.keys().cloned().collect::<Vec<_>>().join(", ")
        };

        vec![
            self.log(
                EventChannel::Room,
                OutputComponent::RoomHeader,
                format!("{} [{}]", room.title, room.region),
            ),
            self.log(
                EventChannel::Narrative,
                OutputComponent::NarrativeMessage,
                room.description,
            ),
            self.log(
                EventChannel::System,
                OutputComponent::SystemMessage,
                format!("Exits: {exits}."),
            ),
        ]
    }

    /// Resolve and describe a `look <target>` against nearby things (ticket #17).
    ///
    /// Uses the proximity resolver ([`awareness::resolve_target`]): an
    /// interactable match — the same cell or within reach — is described in full;
    /// a match that is visible but out of reach is reported as too far to examine
    /// (REQ-003); no match yields a "nothing like that nearby" message. Bare
    /// `look` (no target) is handled separately and still describes the room, so
    /// the existing exact-room behavior is preserved (REQ-004/REQ-007).
    fn look_at(&mut self, target: &str) -> Vec<GameEvent> {
        let origin = self.current_room().clone();
        let radii = RadiusConfig::default();
        let text = match awareness::resolve_target(&self.world, &origin, &radii, target) {
            Some(found) if found.proximity.is_interactable() => {
                format!("You study {}. {}", found.name, found.description)
            }
            Some(found) => format!(
                "You can make out {} nearby, but it is too far off to examine closely.",
                found.name
            ),
            // Not nearby — fall back to the carried pack (ticket #20), then to
            // equipped gear (ticket #35): a worn item left the pack but is
            // still possessed, and a projection must not deny it.
            None => self
                .find_in_pack(target)
                .map(|item| {
                    format!(
                        "You examine the {} you are carrying. {}",
                        item.name, item.description
                    )
                })
                .or_else(|| {
                    self.find_equipped(target).map(|item| {
                        format!(
                            "You look over the {} you have equipped. {}",
                            item.name, item.description
                        )
                    })
                })
                .unwrap_or_else(|| format!("You see nothing like '{target}' nearby.")),
        };
        vec![self.log(
            EventChannel::Narrative,
            OutputComponent::NarrativeMessage,
            text,
        )]
    }

    /// Resolve and answer a `talk <target>` against nearby things (ticket #18).
    ///
    /// Reuses the interaction-gated proximity resolver
    /// ([`awareness::resolve_target`]) — no duplicated geometry. Returns
    /// `(accepted, events)` like `swear`/`confront`: addressing a reachable actor
    /// is accepted and emits a narrative response *without moving the player*
    /// (REQ-003); a non-actor, an actor that is visible but out of reach
    /// (REQ-004), or no match at all is refused with a clear line and no state
    /// change. A reachable actor's reply is its authored [`EntityDialogue`]
    /// (ticket #19) selected by oath state — talking to an unsworn oath's issuer
    /// records the offer (see [`Engine::npc_dialogue_line`]); an NPC without
    /// dialogue keeps the generic reply. Kind is checked before reach, so the
    /// too-far line is reserved for actual actors.
    fn talk_at(&mut self, target: &str) -> (bool, Vec<GameEvent>) {
        let origin = self.current_room().clone();
        let radii = RadiusConfig::default();
        let (accepted, text) = match awareness::resolve_target(&self.world, &origin, &radii, target)
        {
            None => (
                false,
                format!("There is no one like '{target}' here to talk to."),
            ),
            Some(found) if found.kind != AwarenessKind::Actor => (
                false,
                format!("You cannot hold a conversation with {}.", found.name),
            ),
            Some(found) if !found.proximity.is_interactable() => {
                (false, format!("{} is too far away to talk to.", found.name))
            }
            Some(found) => (true, self.npc_dialogue_line(&found.id)),
        };
        (
            accepted,
            vec![self.log(
                EventChannel::Narrative,
                OutputComponent::NarrativeMessage,
                text,
            )],
        )
    }

    /// Pick the authored line a conversable NPC speaks (ticket #19).
    ///
    /// An NPC with no [`EntityDialogue`] keeps the ticket #18 generic reply
    /// (`conversable` → ready to talk; otherwise nothing to say). When the NPC
    /// issues the module's designated oath and carries oath lines, the line is
    /// chosen by the player's oath state — and an unsworn oath is *offered*
    /// (recording `offered_oath_id` so `swear` is permitted, REQ-002/003).
    /// Otherwise the NPC's `greeting` is used (REQ-001/005).
    fn npc_dialogue_line(&mut self, entity_id: &str) -> String {
        let entity = self
            .world
            .entities
            .get(entity_id)
            .expect("talk resolved this entity from the world");

        let Some(dialogue) = entity.dialogue.as_ref() else {
            return if entity.has_role(Role::Talkable) {
                format!("{} turns to face you, ready to talk.", entity.name)
            } else {
                format!("{} has nothing to say to you.", entity.name)
            };
        };

        // The designated oath this NPC issues, if any (its `issuer_id` names it).
        let issued_oath_id = self
            .world
            .oath_id
            .as_deref()
            .filter(|oath_id| {
                self.world
                    .oaths
                    .get(*oath_id)
                    .and_then(|oath| oath.issuer_id.as_deref())
                    == Some(entity_id)
            })
            .map(str::to_owned);

        match (dialogue.oath.as_ref(), issued_oath_id) {
            (Some(lines), Some(oath_id)) => {
                match self.state.oath.as_ref().map(|progress| progress.status) {
                    Some(OathStatus::Fulfilled) => lines.fulfilled.clone(),
                    Some(OathStatus::Sworn) => lines.sworn.clone(),
                    None => {
                        // Offer (or re-offer) the oath: record it so `swear` is now
                        // permitted. Clone the line before mutating so the `world`
                        // and `state` borrows stay disjoint.
                        let line = lines.offer.clone();
                        self.state.offered_oath_id = Some(oath_id);
                        line
                    }
                }
            }
            _ => dialogue.greeting.clone(),
        }
    }

    /// Resolve and perform a `take <target>` against nearby things (ticket #18).
    ///
    /// Reuses the interaction-gated proximity resolver. Returns `(accepted,
    /// events)`: taking a reachable world item moves it into the player's pack and
    /// removes it from its placing room — so [`awareness::perceive`] then drops it
    /// from the snapshot's `contents` (REQ-005). A non-item, an item that is
    /// visible but out of reach, or a hidden/unknown target is refused with a clear
    /// line and no state change (REQ-006). Kind is checked before reach so the
    /// too-far line is reserved for actual items.
    fn take_at(&mut self, target: &str) -> (bool, Vec<GameEvent>) {
        let origin = self.current_room().clone();
        let radii = RadiusConfig::default();
        match awareness::resolve_target(&self.world, &origin, &radii, target) {
            None => (
                false,
                vec![self.log(
                    EventChannel::Narrative,
                    OutputComponent::NarrativeMessage,
                    format!("You see nothing like '{target}' here to take."),
                )],
            ),
            Some(found) if found.kind != AwarenessKind::Item => (
                false,
                vec![self.log(
                    EventChannel::Narrative,
                    OutputComponent::NarrativeMessage,
                    format!("You cannot carry {}.", found.name),
                )],
            ),
            Some(found) if !found.proximity.is_interactable() => (
                false,
                vec![self.log(
                    EventChannel::Narrative,
                    OutputComponent::NarrativeMessage,
                    format!("{} is too far away to reach.", found.name),
                )],
            ),
            Some(found) => {
                // Reachable world item: drop it from the exact placing room
                // (defensive `get_mut` — `room_id` came from the resolver, so
                // the room exists) and carry it. perceive() then excludes it.
                if let Some(room) = self.world.rooms.get_mut(&found.room_id) {
                    room.items.retain(|item_id| item_id != &found.id);
                }
                let name = found.name.clone();
                let item_id = found.id.clone();
                self.state.pack.push(found.id);
                let mut events = vec![self.log(
                    EventChannel::Inventory,
                    OutputComponent::ItemCard,
                    format!("You take the {name}."),
                )];
                // Ticket #29: recovering the sworn oath's authored objective
                // fulfills the oath — the pickup line lands first.
                events.extend(self.fulfill_oath_on_recovery(&item_id));
                (true, events)
            }
        }
    }

    /// Fulfill the sworn oath when its authored objective is recovered
    /// (ticket #29): taking the oath's `objective_item_id` while the oath is
    /// sworn flips it to fulfilled and emits the typed
    /// [`GameEventKind::OathFulfilled`] followed by the oath's authored
    /// announcements (ticket #27 — each delivered only where its scope
    /// matches).
    ///
    /// Returns no events when the take is not the sworn objective: no oath
    /// sworn, an already-fulfilled oath, a different item, or an oath with no
    /// objective at all.
    fn fulfill_oath_on_recovery(&mut self, item_id: &str) -> Vec<GameEvent> {
        let oath_id = match self.state.oath.as_ref() {
            Some(progress) if progress.status == OathStatus::Sworn => progress.oath_id.clone(),
            _ => return Vec::new(),
        };
        // The sworn oath id always resolves: `swear` only records the
        // try_new-validated designated oath.
        let definition = self
            .world
            .oaths
            .get(&oath_id)
            .expect("sworn oath is a try_new-validated invariant");
        if definition.objective_item_id.as_deref() != Some(item_id) {
            return Vec::new();
        }
        let announcements = definition.fulfillment_announcements.clone();
        if let Some(progress) = self.state.oath.as_mut() {
            progress.status = OathStatus::Fulfilled;
        }

        // No separate human line here: both renderers already print "Your
        // oath is fulfilled." for the typed event, so a log twin would show
        // the climactic beat twice. The pickup line + this + the authored
        // announcements are the feed narrative.
        let mut events =
            vec![self.event(EventChannel::Oath, GameEventKind::OathFulfilled { oath_id })];
        for announcement in &announcements {
            events.extend(self.announce(announcement));
        }
        events
    }

    /// Find a carried item whose name or an alias matches `query` (ticket #20).
    ///
    /// Shared by `drop` and the carried-item branch of `look`. Orphan pack ids
    /// (no `world.items` entry) are skipped by `filter_map`, so there is no
    /// unreachable lookup branch. Reuses the awareness name/alias matcher.
    fn find_in_pack(&self, query: &str) -> Option<&Item> {
        self.state
            .pack
            .iter()
            .filter_map(|item_id| self.world.items.get(item_id))
            .find(|item| awareness::name_or_alias_matches(&item.name, &item.aliases, query))
    }

    /// Return every carried item matching `query`, with its pack index.
    ///
    /// `drop` needs the index so it can remove exactly one carried item. Multiple
    /// matches are treated as ambiguous instead of silently dropping every copy.
    fn matching_pack_items(&self, query: &str) -> Vec<(usize, String, String)> {
        self.state
            .pack
            .iter()
            .enumerate()
            .filter_map(|(index, item_id)| {
                self.world.items.get(item_id).and_then(|item| {
                    awareness::name_or_alias_matches(&item.name, &item.aliases, query)
                        .then(|| (index, item.id.clone(), item.name.clone()))
                })
            })
            .collect()
    }

    /// Resolve and perform a `drop <target>` against the carried pack (ticket #20).
    ///
    /// The inverse of `take_at`: a carried item is removed from the pack and
    /// placed back into the current room's `items`, so `awareness::perceive` then
    /// surfaces it at the player's cell (REQ-005). A target the player is not
    /// carrying, or an ambiguous duplicate carried match, is refused with a clear
    /// line and no state change (REQ-006). The id/name are cloned out before the
    /// state mutation so the `world`/`state` borrows stay disjoint.
    fn drop_at(&mut self, target: &str) -> (bool, Vec<GameEvent>) {
        let matches = self.matching_pack_items(target);
        let (index, item_id, name) = match matches.as_slice() {
            [(index, item_id, name)] => (*index, item_id.clone(), name.clone()),
            [] => {
                // Equipped gear left the pack but is still possessed (ticket
                // #35) — the refusal must say so, not deny the item exists.
                if let Some(equipped) = self.find_equipped(target) {
                    let name = equipped.name.clone();
                    return (
                        false,
                        vec![self.log(
                            EventChannel::Narrative,
                            OutputComponent::NarrativeMessage,
                            format!("The {name} is equipped — unequip it before dropping it."),
                        )],
                    );
                }
                return (
                    false,
                    vec![self.log(
                        EventChannel::Narrative,
                        OutputComponent::NarrativeMessage,
                        format!("You aren't carrying anything like '{target}'."),
                    )],
                );
            }
            _ => {
                return (
                    false,
                    vec![self.log(
                        EventChannel::Narrative,
                        OutputComponent::NarrativeMessage,
                        format!("More than one carried item matches '{target}'."),
                    )],
                );
            }
        };

        self.state.pack.remove(index);
        let room_id = self.state.current_room_id.clone();
        if let Some(room) = self.world.rooms.get_mut(&room_id) {
            room.items.push(item_id);
        }
        (
            true,
            vec![self.log(
                EventChannel::Inventory,
                OutputComponent::ItemCard,
                format!("You drop the {name}."),
            )],
        )
    }

    /// List the player's carried items, or an honest empty state (ticket #20,
    /// REQ-003). Always accepted — a readout of `state.pack`, with an
    /// `Equipped:` clause when gear is worn (ticket #35): worn items left the
    /// pack but are still possessed, and the inventory must keep saying so.
    fn list_pack(&mut self) -> Vec<GameEvent> {
        let names: Vec<String> = self
            .state
            .pack
            .iter()
            .map(|item_id| self.item_name(item_id))
            .collect();
        let carrying = if names.is_empty() {
            "You are carrying nothing.".to_string()
        } else {
            format!("You are carrying: {}.", names.join(", "))
        };
        let equipped: Vec<String> = [EquipSlot::Weapon, EquipSlot::Armor]
            .into_iter()
            .filter_map(|slot| self.equipped_slot(slot))
            .map(String::from)
            .collect();
        let text = if equipped.is_empty() {
            carrying
        } else {
            let gear: Vec<String> = equipped.iter().map(|id| self.item_name(id)).collect();
            format!("{carrying} Equipped: {}.", gear.join(", "))
        };
        vec![self.log(
            EventChannel::Inventory,
            OutputComponent::SystemMessage,
            text,
        )]
    }

    /// The room's trading partner (ticket #34): the first placed, visible
    /// shopkeeper. Same-room only — trade happens standing in the shop — and
    /// a hidden vendor is skipped, because the reveal rule applies to every
    /// player-facing surface (the #33 lesson).
    fn find_vendor(&self) -> Option<&Entity> {
        self.current_room()
            .entities
            .iter()
            .filter_map(|entity_id| self.world.entities.get(entity_id))
            .find(|entity| !entity.hidden && entity.has_role(Role::Shopkeeper))
    }

    /// The typed mid-combat trade refusal shared by all three shop verbs
    /// (ticket #34) — combat is committed, like `rest`.
    fn trade_combat_refusal(&mut self) -> (bool, Vec<GameEvent>) {
        (
            false,
            vec![self.log(
                EventChannel::System,
                OutputComponent::SystemMessage,
                "There is no trading in the midst of battle.",
            )],
        )
    }

    /// The typed no-vendor refusal shared by all three shop verbs (ticket #34).
    fn trade_vendor_refusal(&mut self) -> (bool, Vec<GameEvent>) {
        (
            false,
            vec![self.log(
                EventChannel::System,
                OutputComponent::SystemMessage,
                "There is no shopkeeper here to trade with.",
            )],
        )
    }

    /// List the room vendor's stock with prices (ticket #34, REQ-001). The
    /// listing is one joined Inventory line (the `list_pack` shape); an empty
    /// stock is an honest typed refusal.
    fn shop(&mut self) -> (bool, Vec<GameEvent>) {
        if self.state.combat.is_some() {
            return self.trade_combat_refusal();
        }
        let Some(vendor) = self.find_vendor() else {
            return self.trade_vendor_refusal();
        };
        let vendor_name = vendor.name.clone();
        let lines: Vec<String> = vendor
            .inventory
            .iter()
            .filter_map(|item_id| self.world.items.get(item_id))
            // Hidden stock stays off the counter — the reveal rule applies to
            // every player-facing projection (the #33 lesson; `look` would
            // conceal the same item).
            .filter(|item| !item.hidden)
            .map(|item| {
                if item.value > 0 {
                    format!("{} — {} coins", item.name, item.value)
                } else {
                    format!("{} — not for sale", item.name)
                }
            })
            .collect();
        if lines.is_empty() {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("{vendor_name} has nothing to sell."),
                )],
            );
        }
        let text = format!(
            "{vendor_name} offers: {}. You have {} coins.",
            lines.join("; "),
            self.state.player.coins
        );
        (
            true,
            vec![self.log(
                EventChannel::Inventory,
                OutputComponent::SystemMessage,
                text,
            )],
        )
    }

    /// Buy a stocked item from the room's vendor (ticket #34, REQ-002/003).
    /// Settlement is exactly-once: the guards all pass before the single
    /// mutation block moves the item stock→pack and deducts the price.
    fn buy_at(&mut self, target: &str) -> (bool, Vec<GameEvent>) {
        if self.state.combat.is_some() {
            return self.trade_combat_refusal();
        }
        let Some(vendor) = self.find_vendor() else {
            return self.trade_vendor_refusal();
        };
        let vendor_id = vendor.id.clone();
        let vendor_name = vendor.name.clone();
        let found = vendor
            .inventory
            .iter()
            .filter_map(|item_id| self.world.items.get(item_id))
            // Hidden stock reads as unstocked, mirroring the shop listing.
            .filter(|item| !item.hidden)
            .find(|item| awareness::name_or_alias_matches(&item.name, &item.aliases, target))
            .map(|item| (item.id.clone(), item.name.clone(), item.value));
        let Some((item_id, item_name, price)) = found else {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("{vendor_name} does not have '{target}'."),
                )],
            );
        };
        if price == 0 {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("{vendor_name} won't part with {item_name}."),
                )],
            );
        }
        if self.state.player.coins < price {
            let coins = self.state.player.coins;
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("You cannot afford {item_name} ({price} coins; you have {coins})."),
                )],
            );
        }
        let stock = &mut self
            .world
            .entities
            .get_mut(&vendor_id)
            .expect("the vendor was resolved from this room moments ago")
            .inventory;
        if let Some(position) = stock.iter().position(|id| id == &item_id) {
            stock.remove(position);
        }
        self.state.pack.push(item_id.clone());
        // The affordability guard above makes underflow impossible; saturating
        // keeps crafted-save extremes panic-free regardless (ticket #28 posture).
        self.state.player.coins = self.state.player.coins.saturating_sub(price);
        let coins = self.state.player.coins;
        let mut events = vec![self.log(
            EventChannel::Narrative,
            OutputComponent::NarrativeMessage,
            format!("You buy {item_name} for {price} coins. ({coins} remain.)"),
        )];
        // Acquisition is acquisition (inspect finding): a purchased oath
        // objective fulfills the sworn oath exactly as a taken one does —
        // `take_at`'s ordering, purchase line first.
        events.extend(self.fulfill_oath_on_recovery(&item_id));
        (true, events)
    }

    /// Sell a carried item to the room's vendor (ticket #34, REQ-004).
    /// Oath-flagged items are never sellable (the clapper rule); a zero-value
    /// item is worthless to vendors. Mirrors `buy_at`'s exactly-once shape.
    fn sell_at(&mut self, target: &str) -> (bool, Vec<GameEvent>) {
        if self.state.combat.is_some() {
            return self.trade_combat_refusal();
        }
        let Some(vendor) = self.find_vendor() else {
            return self.trade_vendor_refusal();
        };
        let vendor_id = vendor.id.clone();
        let vendor_name = vendor.name.clone();
        // Selling is lossy (buy-back costs double), so it gets `drop`'s
        // ambiguity refusal rather than `find_in_pack`'s silent first match
        // (inspect finding — the irreversible action must not guess).
        let matches = self.matching_pack_items(target);
        if matches.len() > 1 {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("More than one carried item matches '{target}'."),
                )],
            );
        }
        let Some((item_id, item_name, value, oath_bound)) = matches
            .first()
            .and_then(|(_, item_id, _)| self.world.items.get(item_id))
            .map(|item| {
                (
                    item.id.clone(),
                    item.name.clone(),
                    item.value,
                    item.flags.iter().any(|flag| flag == "oath"),
                )
            })
        else {
            // Equipped gear is possessed but not sellable in place (ticket
            // #35) — name the real reason instead of denying the item.
            if let Some(equipped) = self.find_equipped(target) {
                let name = equipped.name.clone();
                return (
                    false,
                    vec![self.log(
                        EventChannel::System,
                        OutputComponent::SystemMessage,
                        format!("The {name} is equipped — unequip it before selling."),
                    )],
                );
            }
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("You are not carrying '{target}'."),
                )],
            );
        };
        if oath_bound {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("{item_name} is bound to your oath — you cannot sell it."),
                )],
            );
        }
        if value == 0 {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("{vendor_name} has no use for {item_name}."),
                )],
            );
        }
        let price = sell_price(value);
        if let Some(position) = self.state.pack.iter().position(|id| id == &item_id) {
            self.state.pack.remove(position);
        }
        self.world
            .entities
            .get_mut(&vendor_id)
            .expect("the vendor was resolved from this room moments ago")
            .inventory
            .push(item_id);
        self.state.player.coins = self.state.player.coins.saturating_add(price);
        let coins = self.state.player.coins;
        (
            true,
            vec![self.log(
                EventChannel::Narrative,
                OutputComponent::NarrativeMessage,
                format!("You sell {item_name} for {price} coins. ({coins} now.)"),
            )],
        )
    }

    /// The display name for `item_id`, falling back to the id itself for a
    /// dangling reference (the `list_pack` posture — never panic on state).
    fn item_name(&self, item_id: &str) -> String {
        self.world
            .items
            .get(item_id)
            .map_or_else(|| item_id.to_string(), |item| item.name.clone())
    }

    /// The slot's equipped item id (ticket #35) — with
    /// [`Self::equipped_slot_mut`], the single mapping from [`EquipSlot`] to
    /// its [`PlayerState`] field.
    fn equipped_slot(&self, slot: EquipSlot) -> Option<&str> {
        match slot {
            EquipSlot::Weapon => self.state.player.equipped_weapon.as_deref(),
            EquipSlot::Armor => self.state.player.equipped_armor.as_deref(),
        }
    }

    /// The slot's equipped-state storage (ticket #35) — the write half of the
    /// [`Self::equipped_slot`] mapping pair.
    const fn equipped_slot_mut(&mut self, slot: EquipSlot) -> &mut Option<String> {
        match slot {
            EquipSlot::Weapon => &mut self.state.player.equipped_weapon,
            EquipSlot::Armor => &mut self.state.player.equipped_armor,
        }
    }

    /// The first equipped item matching `query` by name/alias (ticket #35) —
    /// the gear counterpart of [`Self::find_in_pack`], used by the projections
    /// and refusal arms that must keep seeing possessed-but-worn gear.
    fn find_equipped(&self, query: &str) -> Option<&Item> {
        [EquipSlot::Weapon, EquipSlot::Armor]
            .into_iter()
            .filter_map(|slot| self.equipped_slot(slot))
            .filter_map(|item_id| self.world.items.get(item_id))
            .find(|item| awareness::name_or_alias_matches(&item.name, &item.aliases, query))
    }

    /// The stat mod contributed by the gear in `slot` (ticket #35): the
    /// weapon's `attack` or the armor's `defense`, 0 when the slot is empty,
    /// its id dangles, or a crafted save parked gear in the wrong slot (a
    /// slot only ever pays its own profile's stat). Saturating `u32 → i32` so
    /// a crafted `u32::MAX` mod can never overflow combat math.
    fn slot_mod(&self, slot: EquipSlot) -> i32 {
        self.equipped_slot(slot)
            .and_then(|item_id| self.world.items.get(item_id))
            .and_then(|item| item.equipment)
            .filter(|profile| profile.slot == slot)
            .map_or(0, |profile| {
                let raw = match slot {
                    EquipSlot::Weapon => profile.attack,
                    EquipSlot::Armor => profile.defense,
                };
                i32::try_from(raw).unwrap_or(i32::MAX)
            })
    }

    /// Damage added to every player strike by the equipped weapon (ticket #35).
    fn player_attack_bonus(&self) -> i32 {
        self.slot_mod(EquipSlot::Weapon)
    }

    /// Damage turned aside from every incoming hit by the equipped armor
    /// (ticket #35); the dealt amount floors at 0.
    fn player_defense(&self) -> i32 {
        self.slot_mod(EquipSlot::Armor)
    }

    /// The typed mid-combat gear refusal shared by `equip`/`unequip` (ticket
    /// #35) — changing gear is committed time, the trade-verb parity.
    fn gear_combat_refusal(&mut self) -> (bool, Vec<GameEvent>) {
        (
            false,
            vec![self.log(
                EventChannel::System,
                OutputComponent::SystemMessage,
                "There is no changing gear in the midst of battle.",
            )],
        )
    }

    /// Resolve and perform an `equip <target>` from the carried pack (ticket
    /// #35, REQ-001/002). The item's authored [`EquipmentProfile`] names its
    /// slot; equipping moves the id pack → slot, and a prior occupant swaps
    /// back into the pack in the same act. Every failure arm is a typed
    /// refusal with no state change: mid-combat, not carried, ambiguous, or
    /// not equipment.
    fn equip_at(&mut self, target: &str) -> (bool, Vec<GameEvent>) {
        if self.state.combat.is_some() {
            return self.gear_combat_refusal();
        }
        let matches = self.matching_pack_items(target);
        if matches.len() > 1 {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("More than one carried item matches '{target}'."),
                )],
            );
        }
        let Some((index, item_id, item_name)) = matches.into_iter().next() else {
            // An already-equipped match deserves the honest arm, not a denial
            // (ticket #35 inspect): the gear left the pack when it was donned.
            if let Some(equipped) = self.find_equipped(target) {
                let name = equipped.name.clone();
                return (
                    false,
                    vec![self.log(
                        EventChannel::System,
                        OutputComponent::SystemMessage,
                        format!("The {name} is already equipped."),
                    )],
                );
            }
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("You are not carrying '{target}'."),
                )],
            );
        };
        let Some(profile) = self
            .world
            .items
            .get(&item_id)
            .and_then(|item| item.equipment)
        else {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("{item_name} is not something you can equip."),
                )],
            );
        };
        self.state.pack.remove(index);
        let previous = self.equipped_slot_mut(profile.slot).replace(item_id);
        let verb = match profile.slot {
            EquipSlot::Weapon => "wield",
            EquipSlot::Armor => "wear",
        };
        let line = previous.map_or_else(
            || format!("You {verb} the {item_name}."),
            |previous_id| {
                let previous_name = self.item_name(&previous_id);
                self.stow_into_pack(previous_id);
                format!("You {verb} the {item_name}, stowing the {previous_name}.")
            },
        );
        (
            true,
            vec![self.log(EventChannel::Inventory, OutputComponent::ItemCard, line)],
        )
    }

    /// Resolve and perform an `unequip <target>` (ticket #35, REQ-001/002).
    /// The target names a slot (`weapon`/`armor`) or an equipped item; the
    /// freed item returns to the pack. Typed refusals, no state change:
    /// mid-combat, an empty named slot, no equipped match, or an ambiguous
    /// one.
    fn unequip_at(&mut self, target: &str) -> (bool, Vec<GameEvent>) {
        if self.state.combat.is_some() {
            return self.gear_combat_refusal();
        }
        let query = target.trim().to_lowercase();
        let slot_named = match query.as_str() {
            "weapon" => Some(EquipSlot::Weapon),
            "armor" => Some(EquipSlot::Armor),
            _ => None,
        };
        if let Some(slot) = slot_named {
            let Some(item_id) = self.equipped_slot_mut(slot).take() else {
                return (
                    false,
                    vec![self.log(
                        EventChannel::System,
                        OutputComponent::SystemMessage,
                        format!("Nothing is equipped as your {}.", slot.as_str()),
                    )],
                );
            };
            return self.finish_unequip(item_id);
        }
        let matches: Vec<EquipSlot> = [EquipSlot::Weapon, EquipSlot::Armor]
            .into_iter()
            .filter(|&slot| {
                self.equipped_slot(slot)
                    .and_then(|item_id| self.world.items.get(item_id))
                    .is_some_and(|item| {
                        awareness::name_or_alias_matches(&item.name, &item.aliases, target)
                    })
            })
            .collect();
        match matches.as_slice() {
            [slot] => {
                let item_id = self
                    .equipped_slot_mut(*slot)
                    .take()
                    .expect("the slot matched as occupied moments ago");
                self.finish_unequip(item_id)
            }
            [] => (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("You have nothing like '{target}' equipped."),
                )],
            ),
            _ => (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("More than one equipped item matches '{target}'."),
                )],
            ),
        }
    }

    /// Return an unequipped item to the pack and narrate it (ticket #35) —
    /// the shared success tail of both `unequip` arms.
    fn finish_unequip(&mut self, item_id: String) -> (bool, Vec<GameEvent>) {
        let name = self.item_name(&item_id);
        self.stow_into_pack(item_id);
        (
            true,
            vec![self.log(
                EventChannel::Inventory,
                OutputComponent::ItemCard,
                format!("You unequip the {name}."),
            )],
        )
    }

    /// Return a freed item id to the pack (ticket #35). Always pushes: both
    /// callers move a live reference OUT of a slot, so the push conserves the
    /// reference count exactly. (An earlier dedup guard here was inspect-caught
    /// as loss-only — a module may legally place one id twice, and "the id is
    /// already carried" then means two copies, not a duplicate to swallow.)
    fn stow_into_pack(&mut self, item_id: String) {
        self.state.pack.push(item_id);
    }

    /// The equipped-gear projection for the player snapshot (ticket #35):
    /// one entry per filled slot, weapon first, names resolved like the pack.
    fn equipment_snapshot(&self) -> Vec<EquippedItemSnapshot> {
        [EquipSlot::Weapon, EquipSlot::Armor]
            .into_iter()
            .filter_map(|slot| {
                let item_id = self.equipped_slot(slot)?;
                Some(EquippedItemSnapshot {
                    slot: slot.as_str().to_string(),
                    id: item_id.to_string(),
                    name: self.item_name(item_id),
                })
            })
            .collect()
    }

    fn move_direction(&mut self, direction: Direction) -> Vec<GameEvent> {
        let room = self.current_room().clone();
        let Some(next_room_id) = room.exits.get(direction.as_str()) else {
            return vec![self.log(
                EventChannel::System,
                OutputComponent::SystemMessage,
                "You cannot go that way.",
            )];
        };

        let Some(next_room) = self.world.rooms.get(next_room_id).cloned() else {
            return vec![self.log(
                EventChannel::System,
                OutputComponent::SystemMessage,
                "That exit points into unfinished world-data.",
            )];
        };

        if !next_room.passable {
            return vec![self.log(
                EventChannel::System,
                OutputComponent::SystemMessage,
                "Something blocks that way.",
            )];
        }

        // Leaving the encounter room breaks the fight off as a flee (ticket
        // #24): combat now advances on real-time pulses, so a lingering
        // encounter would keep striking the player from rooms away every
        // pulse. Walking out is the fled outcome — the same event and
        // semantics as a queued flee (enemy survives, HP kept, pulses stop).
        let mut events = Vec::new();
        if self.state.combat.is_some() {
            self.end_combat(CombatOutcome::Fled, &mut events);
        }

        self.state.current_room_id = next_room.id.clone();
        self.state.discovered_rooms.insert(next_room.id.clone());

        events.push(self.event(
            EventChannel::Room,
            GameEventKind::RoomEntered {
                room_id: next_room.id,
                title: next_room.title,
            },
        ));
        events.extend(self.describe_current_room());
        events
    }

    /// Swear the module's designated oath.
    ///
    /// Returns `(accepted, events)`. Refused (no state change) when an oath is
    /// already sworn or the module designates none. On success, records the oath
    /// as [`OathStatus::Sworn`] and emits a narrative line plus a typed
    /// [`GameEventKind::OathSworn`] on the `Oath` channel.
    fn swear(&mut self) -> (bool, Vec<GameEvent>) {
        if self.state.oath.is_some() {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    "You have already sworn your oath.",
                )],
            );
        }

        let Some(oath_id) = self.world.oath_id.clone() else {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    "There is no oath to swear here.",
                )],
            );
        };

        // Invariant: `Engine::try_new` validates that a designated `oath_id` is a
        // key in `oaths`, so this lookup is never `None` for a constructed engine
        // — the same construction-time guarantee `current_room()` relies on.
        let oath = self
            .world
            .oaths
            .get(&oath_id)
            .expect("designated oath is a try_new-validated invariant");
        let title = oath.title.clone();
        let description = oath.description.clone();
        let issuer_id = oath.issuer_id.clone();

        // Offer gate (REQ-003): an issuer-offered oath can only be sworn after the
        // player has been offered it by talking to the issuer. An issuer-less oath
        // stays globally swearable (the pre-#19 behavior).
        if let Some(issuer_id) = issuer_id {
            if self.state.offered_oath_id.as_deref() != Some(oath_id.as_str()) {
                let issuer_name = self
                    .world
                    .entities
                    .get(&issuer_id)
                    .expect("oath issuer is a try_new-validated invariant")
                    .name
                    .clone();
                return (
                    false,
                    vec![self.log(
                        EventChannel::System,
                        OutputComponent::SystemMessage,
                        format!(
                            "You cannot swear that oath until it has been offered. Seek out {issuer_name} to be offered it."
                        ),
                    )],
                );
            }
        }

        self.state.oath = Some(OathProgress {
            oath_id: oath_id.clone(),
            title: title.clone(),
            status: OathStatus::Sworn,
        });

        let narrative = self.log(
            EventChannel::Narrative,
            OutputComponent::NarrativeMessage,
            format!("You swear {title}: {description}"),
        );
        let sworn = self.event(
            EventChannel::Oath,
            GameEventKind::OathSworn { oath_id, title },
        );
        (true, vec![narrative, sworn])
    }

    /// Engage the boss at the current room's endpoint (ticket #29).
    ///
    /// Returns `(accepted, events)`. The boss is the placed entity carrying
    /// the `"boss"` role. With the oath sworn and the boss present, confront
    /// STARTS the real pulse-loop encounter via [`Engine::engage_enemy`] —
    /// fulfillment now rides recovering the oath's objective
    /// ([`Engine::fulfill_oath_on_recovery`]), not the confrontation. While a
    /// fight is already underway, confront presses the attack (resolves the
    /// next round) exactly like `attack`. Refused (no state change) when
    /// there is no boss here, when no oath is sworn, or when the oath is
    /// already kept. Deterministic — no RNG.
    fn confront(&mut self) -> (bool, Vec<GameEvent>) {
        // Mid-fight, confront presses the attack exactly like `attack` does
        // (ticket #29): re-entry resolves the next round instead of rebuilding
        // the encounter, which would reset the enemy's hp.
        if self.state.combat.is_some() {
            let mut events = Vec::new();
            self.resolve_combat_round(&mut events);
            return (true, events);
        }

        let room = self.current_room().clone();
        let boss = room
            .entities
            .iter()
            .filter_map(|entity_id| self.world.entities.get(entity_id))
            .find(|entity| entity.has_role(Role::Boss));

        let Some(boss) = boss else {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    "There is nothing here to confront.",
                )],
            );
        };

        match self.state.oath.as_ref().map(|progress| progress.status) {
            // The authored encounter (ticket #29): the sworn oath is the
            // gate, and the fight is the same pulse-loop combat the ambient
            // hostiles use. Fulfillment now rides recovering the oath's
            // objective, not the confrontation itself.
            Some(OathStatus::Sworn) => {
                let profile = boss.combat.as_ref().expect(
                    "the boss role contract guarantees a combat profile (validated in try_new)",
                );
                let resolved = ResolvedHostile {
                    id: boss.id.clone(),
                    name: boss.name.clone(),
                    health: profile.health,
                    attack: profile.attack,
                };
                (true, self.engage_enemy(resolved))
            }
            Some(OathStatus::Fulfilled) => {
                // Reachable only with a LIVING boss (victory removes the
                // placement), so the line must not claim it is broken.
                let message = format!(
                    "Your oath is already kept; there is no cause to confront {}.",
                    boss.name
                );
                (
                    false,
                    vec![self.log(
                        EventChannel::System,
                        OutputComponent::SystemMessage,
                        message,
                    )],
                )
            }
            None => {
                let message = format!(
                    "You face {}, but you have sworn no oath to see this through.",
                    boss.name
                );
                (
                    false,
                    vec![self.log(
                        EventChannel::System,
                        OutputComponent::SystemMessage,
                        message,
                    )],
                )
            }
        }
    }

    /// Resolve and perform an `attack`/`strike`/`fight` (ticket #22).
    ///
    /// While a fight is underway it resolves the next round against the active
    /// enemy; otherwise it tries to start one. Returns `(accepted, events)` like
    /// the other handlers; refusals carry `accepted = false` and mutate nothing.
    fn attack(&mut self, target: Option<&str>) -> (bool, Vec<GameEvent>) {
        if self.state.combat.is_some() {
            let mut events = Vec::new();
            self.resolve_combat_round(&mut events);
            return (true, events);
        }
        self.start_combat(target)
    }

    /// Queue a between-pulse combat action, or refuse cleanly when there is no
    /// battle to use it in (ticket #24 `flee`; ticket #25 direct battle
    /// verbs). The queue *is* the pulse boundary: the next pulse's Phase-2
    /// skill window resolves the action, and the cadence is never disturbed.
    /// Deterministic second-queue rule (REQ-005): re-queueing the SAME action
    /// is a no-op with its own line; queueing a DIFFERENT action replaces the
    /// queued one with a clear "change tack" line.
    fn queue_combat_action(&mut self, action: CombatAction) -> (bool, Vec<GameEvent>) {
        let Some(combat) = self.state.combat.as_mut() else {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    action.queue_refusal(),
                )],
            );
        };
        let line = match combat.queued_action {
            Some(queued) if queued == action => action.queue_already().to_string(),
            replaced => {
                // Ticket #31 — focus settles at the queue (spend-on-queue):
                // replacing a different action refunds its committed cost
                // before the new cost is charged, so change-tack can never
                // double-spend or profit. Saturating throughout: a crafted
                // save can carry focus at either i32 extreme (ticket #28).
                let cost = action.focus_cost();
                let effective = self
                    .state
                    .player
                    .focus
                    .saturating_add(replaced.map_or(0, CombatAction::focus_cost));
                // `cost > 0` keeps free verbs free at ANY focus — a crafted
                // negative pool must never soft-lock flee (REQ-002/003).
                if cost > 0 && effective < cost {
                    return (
                        false,
                        vec![self.log(
                            EventChannel::System,
                            OutputComponent::SystemMessage,
                            action.focus_refusal(),
                        )],
                    );
                }
                self.state.player.focus = effective.saturating_sub(cost);
                combat.queued_action = Some(action);
                if replaced.is_some() {
                    format!("You change tack. {}", action.queue_confirmation())
                } else {
                    action.queue_confirmation().to_string()
                }
            }
        };
        combat.log.push(line.clone());
        (
            true,
            vec![self.log(EventChannel::Combat, OutputComponent::CombatMessage, line)],
        )
    }

    /// Start a fight against a hostile in the current room (ticket #22, REQ-001).
    ///
    /// Gates on the room's `combat_enabled` flag (REQ-005), resolves the hostile
    /// (named, or the first one present), copies its authored stats into a
    /// self-contained [`CombatState`], emits [`GameEventKind::CombatStarted`], and
    /// resolves the opening round. A failed gate or resolution refuses cleanly with
    /// no state change.
    fn start_combat(&mut self, target: Option<&str>) -> (bool, Vec<GameEvent>) {
        let room = self.current_room().clone();
        if !room.combat_enabled {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    "This is no place for a fight.",
                )],
            );
        }

        let hostile = match self.find_hostile(&room, target) {
            Ok(hostile) => hostile,
            Err(message) => {
                return (
                    false,
                    vec![self.log(
                        EventChannel::Narrative,
                        OutputComponent::NarrativeMessage,
                        message,
                    )],
                );
            }
        };

        (true, self.engage_enemy(hostile))
    }

    /// Begin an encounter against a resolved enemy (ticket #29): build the
    /// self-contained [`CombatState`] from the authored stats, emit
    /// [`GameEventKind::CombatStarted`], and resolve the opening round.
    ///
    /// The shared tail of both combat entries — the ambient hostile path
    /// (`attack`, ticket #22) and the oath-gated boss path (`confront`,
    /// ticket #29) — so there is exactly one combat model behind two gates.
    fn engage_enemy(&mut self, enemy: ResolvedHostile) -> Vec<GameEvent> {
        let ResolvedHostile {
            id: enemy_id,
            name: enemy_name,
            health,
            attack,
        } = enemy;
        let enemy_max_hp = i32::try_from(health).unwrap_or(i32::MAX);
        let enemy_attack = i32::try_from(attack).unwrap_or(i32::MAX);
        self.state.combat = Some(CombatState {
            enemy_id: enemy_id.clone(),
            enemy_name: enemy_name.clone(),
            enemy_hp: enemy_max_hp,
            enemy_max_hp,
            enemy_attack,
            round: 0,
            log: Vec::new(),
            pulse_rate: DEFAULT_COMBAT_PULSE_TICKS,
            next_pulse_at: self.state.tick.saturating_add(DEFAULT_COMBAT_PULSE_TICKS),
            queued_action: None,
            guard_charge: false,
        });

        let mut events = vec![self.event(
            EventChannel::Combat,
            GameEventKind::CombatStarted {
                enemy_id,
                enemy_name: enemy_name.clone(),
                text: format!("{enemy_name} turns on you. Steel yourself."),
            },
        )];
        self.resolve_combat_round(&mut events);
        events
    }

    /// Find the hostile `attack` should engage (ticket #22). `attack <name>`
    /// resolves the named thing through the shared proximity resolver and requires
    /// a reachable, hostile actor; a bare `attack` engages the first hostile placed
    /// in this room. Returns the resolved hostile or a player-facing refusal line.
    fn find_hostile(
        &self,
        room: &RoomDefinition,
        target: Option<&str>,
    ) -> Result<ResolvedHostile, String> {
        let Some(name) = target else {
            return room
                .entities
                .iter()
                .find_map(|entity_id| self.hostile_stats(entity_id))
                .ok_or_else(|| "There is nothing here to fight.".to_string());
        };
        self.resolve_named_hostile(room, name)
    }

    /// Resolve `attack <name>` to a reachable hostile actor, or a refusal line.
    fn resolve_named_hostile(
        &self,
        room: &RoomDefinition,
        target: &str,
    ) -> Result<ResolvedHostile, String> {
        let radii = RadiusConfig::default();
        match awareness::resolve_target(&self.world, room, &radii, target) {
            None => Err(format!("You see nothing like '{target}' here to fight.")),
            Some(found) if found.kind != AwarenessKind::Actor => {
                Err(format!("You cannot fight {}.", found.name))
            }
            Some(found) if !found.proximity.is_interactable() => {
                Err(format!("{} is too far away to fight.", found.name))
            }
            Some(found) => self
                .hostile_stats(&found.id)
                .ok_or_else(|| format!("{} is not something you can attack.", found.name)),
        }
    }

    /// The combat stats of `entity_id` if it declares `Role::Hostile`, else `None`
    /// (a non-hostile is filtered by `has_role`). The entity lookup and the combat
    /// profile are construction invariants, not runtime conditions: callers pass a
    /// world-resolved id, and `validate_entity_contracts` (run in `try_new`) rejects
    /// any hostile lacking a combat profile — so both `expect`s are unreachable for
    /// a validated world (ticket #21/#22).
    fn hostile_stats(&self, entity_id: &str) -> Option<ResolvedHostile> {
        let entity = self
            .world
            .entities
            .get(entity_id)
            .expect("hostile_stats is only called with a world-resolved entity id");
        if !entity.has_role(Role::Hostile) {
            return None;
        }
        let combat = entity
            .combat
            .as_ref()
            .expect("the hostile role contract guarantees a combat profile (validated in try_new)");
        Some(ResolvedHostile {
            id: entity.id.clone(),
            name: entity.name.clone(),
            health: combat.health,
            attack: combat.attack,
        })
    }

    /// Resolve one combat round against the active enemy (ticket #22): the player
    /// strikes, and — if the enemy survives — the enemy strikes back. Either side
    /// reaching zero HP ends the fight (REQ-002/003/004). A no-op when no fight is
    /// active. Each line is recorded on the battle log (for the modal) and the feed.
    fn resolve_combat_round(&mut self, events: &mut Vec<GameEvent>) {
        // Gear-aware combat (ticket #35): the equipped weapon adds to every
        // strike. Computed before the `&mut combat` borrow below.
        let damage = PLAYER_STRIKE_DAMAGE.saturating_add(self.player_attack_bonus());
        let (player_line, enemy_dead, enemy_name, enemy_attack) = {
            let combat = self
                .state
                .combat
                .as_mut()
                .expect("resolve_combat_round is only called with an active encounter");
            combat.round = combat.round.saturating_add(1);
            combat.enemy_hp = combat.enemy_hp.saturating_sub(damage).max(0);
            let line = format!(
                "You strike {} for {damage} ({}/{}).",
                combat.enemy_name, combat.enemy_hp, combat.enemy_max_hp
            );
            combat.log.push(line.clone());
            (
                line,
                combat.enemy_hp <= 0,
                combat.enemy_name.clone(),
                combat.enemy_attack,
            )
        };
        events.push(self.log(
            EventChannel::Combat,
            OutputComponent::CombatMessage,
            player_line,
        ));
        if enemy_dead {
            self.end_combat(CombatOutcome::Victory, events);
            return;
        }

        // A one-shot guard charge (ticket #25) turns the return aside entirely:
        // consume it, narrate, and skip the damage — a blocked hit can never
        // defeat the player. Consumed by the next return from ANY source
        // (pulse Phase 1 or a manual round).
        let combat = self
            .state
            .combat
            .as_mut()
            .expect("combat remains active until a combatant falls");
        if combat.guard_charge {
            combat.guard_charge = false;
            let blocked = format!("{enemy_name} strikes, but your guard turns the blow aside.");
            combat.log.push(blocked.clone());
            events.push(self.log(
                EventChannel::Combat,
                OutputComponent::CombatMessage,
                blocked,
            ));
            return;
        }

        // Armor turns part of the return aside (ticket #35): the narrated
        // number is the damage DEALT after reduction (floored at 0), while a
        // disclosed enemy stat sheet keeps showing its raw attack.
        let dealt = enemy_attack.saturating_sub(self.player_defense()).max(0);
        self.state.player.hp = self.state.player.hp.saturating_sub(dealt).max(0);
        let player_hp = self.state.player.hp;
        let player_max_hp = self.state.player.max_hp;
        let enemy_line =
            format!("{enemy_name} hits you for {dealt} ({player_hp}/{player_max_hp}).");
        self.state
            .combat
            .as_mut()
            .expect("combat remains active until a combatant falls")
            .log
            .push(enemy_line.clone());
        events.push(self.log(
            EventChannel::Combat,
            OutputComponent::CombatMessage,
            enemy_line,
        ));
        if player_hp <= 0 {
            self.end_combat(CombatOutcome::Defeat, events);
        }
    }

    /// End the active encounter with `outcome` (ticket #22, REQ-004): emit the
    /// typed [`GameEventKind::CombatEnded`] marker carrying the compact feed
    /// summary, apply the outcome, and clear combat state. Victory removes the
    /// defeated enemy, drops its authored inventory into the room, and awards
    /// its authored XP (ticket #26); defeat resets the player to the world
    /// start room at full HP with a deterministic XP penalty (ticket #26 —
    /// deliberately replacing the #22 revive-in-place semantics); flee leaves
    /// the world as it stands.
    fn end_combat(&mut self, outcome: CombatOutcome, events: &mut Vec<GameEvent>) {
        let combat = self
            .state
            .combat
            .take()
            .expect("end_combat is only called mid-encounter (combat is active)");
        // Ticket #31: an action still queued when the fight ends never fired —
        // its committed focus comes back on EVERY outcome (a fired action was
        // already take()n by the skill window, so no double refund). Defeat's
        // full restore below overwrites this; the rule stays uniform.
        if let Some(unfired) = combat.queued_action {
            self.state.player.focus = self.state.player.focus.saturating_add(unfired.focus_cost());
        }
        let enemy_name = combat.enemy_name;
        let text = match outcome {
            CombatOutcome::Victory => {
                self.remove_entity_everywhere(&combat.enemy_id);
                self.drop_enemy_inventory(&combat.enemy_id, &enemy_name, events);
                let xp = self.enemy_xp_reward(&combat.enemy_id);
                let coins = self.enemy_coin_reward(&combat.enemy_id);
                // Saturating like the defeat penalty: a u64 award can never
                // overflow-panic, however absurd the authored numbers get.
                self.state.player.xp = self.state.player.xp.saturating_add(xp);
                self.state.player.coins = self.state.player.coins.saturating_add(coins);
                // Four authored-reward arms (ticket #34): the xp-only line
                // stays byte-identical to #26, and an unrewarded win to #22.
                match (xp > 0, coins > 0) {
                    (true, true) => format!(
                        "You have defeated {enemy_name}. Victory! You gain {xp} XP and {coins} coins."
                    ),
                    (true, false) => {
                        format!("You have defeated {enemy_name}. Victory! You gain {xp} XP.")
                    }
                    (false, true) => {
                        format!("You have defeated {enemy_name}. Victory! You gain {coins} coins.")
                    }
                    (false, false) => format!("You have defeated {enemy_name}. Victory!"),
                }
            }
            CombatOutcome::Defeat => {
                self.state.player.hp = self.state.player.max_hp;
                // Ticket #31: focus resets with hp — "battered but whole" is
                // a full reset (#26's defeat shape); victory and flee keep
                // the spent pool instead (the economy's bite).
                self.state.player.focus = self.state.player.max_focus;
                let lost = self.apply_defeat_penalty();
                self.state.current_room_id = self.world.start_room_id.clone();
                let start_title = self.current_room().title.clone();
                if lost > 0 {
                    format!(
                        "{enemy_name} has bested you. You wake at {start_title}, battered but whole. You lose {lost} XP."
                    )
                } else {
                    format!(
                        "{enemy_name} has bested you. You wake at {start_title}, battered but whole."
                    )
                }
            }
            // The fled outcome (ticket #24) leaves the world as it stands: the
            // enemy survives in place and the player keeps their current HP
            // (a stale save's lazy level convergence below is the one
            // exception — earned milestones surface even on a flee).
            CombatOutcome::Fled => format!("You break away from {enemy_name} and escape."),
        };
        events.push(self.event(
            EventChannel::Combat,
            GameEventKind::CombatEnded { outcome, text },
        ));
        // Ticket #30: every xp change syncs the level — victory's award can
        // cross thresholds (the LevelUp burst follows the summary); defeat's
        // penalty never de-levels (the ratchet makes sync a no-op there).
        self.sync_level(events);
        if matches!(outcome, CombatOutcome::Defeat) {
            // The wake-up arrival reuses the movement pattern (ticket #26):
            // RoomEntered + the room description keep the feed and the
            // client's room/map panels coherent with no new surface.
            let (room_id, title) = {
                let room = self.current_room();
                (room.id.clone(), room.title.clone())
            };
            events.push(self.event(
                EventChannel::Room,
                GameEventKind::RoomEntered { room_id, title },
            ));
            events.extend(self.describe_current_room());
        }
    }

    /// The authored XP a defeated enemy awards (ticket #26): its combat
    /// profile's `xp`, or 0 when no profile/reward exists — a total lookup
    /// that never invents a reward (REQ-002).
    fn enemy_xp_reward(&self, enemy_id: &str) -> u64 {
        self.world
            .entities
            .get(enemy_id)
            .and_then(|entity| entity.combat.as_ref())
            .map_or(0, |profile| profile.xp)
    }

    /// The authored coins a defeated enemy awards (ticket #34): its combat
    /// profile's `coins`, or 0 when no profile/reward exists — `enemy_xp_reward`'s
    /// exact twin, total and never invented.
    fn enemy_coin_reward(&self, enemy_id: &str) -> u64 {
        self.world
            .entities
            .get(enemy_id)
            .and_then(|entity| entity.combat.as_ref())
            .map_or(0, |profile| profile.coins)
    }

    /// Drop a defeated enemy's authored inventory into the current room as
    /// ground items (ticket #26, REQ-003): each id moves into the room's item
    /// placements in authored order — visible and takeable through the
    /// existing contents/`take` flow — and the entity's inventory clears, so
    /// drops cannot duplicate within a session (the take IS the guarantee).
    /// One feed line narrates each drop.
    fn drop_enemy_inventory(
        &mut self,
        enemy_id: &str,
        enemy_name: &str,
        events: &mut Vec<GameEvent>,
    ) {
        let entity = self
            .world
            .entities
            .get_mut(enemy_id)
            .expect("a defeated enemy's registry entry survives placement removal");
        let dropped = std::mem::take(&mut entity.inventory);
        if dropped.is_empty() {
            return;
        }
        // An authored name may carry its own article ("The Bell-Eater"); only
        // prefix one when it doesn't, so the line never reads "The The …"
        // (ticket #29 — the boss put this line on the played route).
        let subject = if enemy_name.starts_with("The ") || enemy_name.starts_with("the ") {
            enemy_name.to_string()
        } else {
            format!("The {enemy_name}")
        };
        for item_id in &dropped {
            let name = &self
                .world
                .items
                .get(item_id)
                .expect("entity inventory ids resolve in a validated world (EntityItemMissing)")
                .name;
            let line = format!("{subject} drops {name}.");
            events.push(self.log(EventChannel::Combat, OutputComponent::CombatMessage, line));
        }
        let room_id = self.state.current_room_id.clone();
        self.world
            .rooms
            .get_mut(&room_id)
            .expect("current room is a try_new-validated invariant")
            .items
            .extend(dropped);
    }

    /// Apply the deterministic defeat XP penalty (ticket #26, REQ-005/006):
    /// with any XP, lose `max(1, floor(xp / 10))`, saturating at zero; with
    /// zero XP nothing is lost. Returns the XP lost. The level RATCHET
    /// (ticket #30) means the loss never de-levels — `end_combat`'s
    /// [`Engine::sync_level`] call is a no-op after a penalty.
    fn apply_defeat_penalty(&mut self) -> u64 {
        let xp = self.state.player.xp;
        if xp == 0 {
            return 0;
        }
        let penalty = (xp / 10).max(1);
        self.state.player.xp = xp.saturating_sub(penalty);
        penalty
    }

    /// Raise the player's level to match the XP curve (ticket #30): one
    /// iteration — and one typed [`GameEventKind::LevelUp`] on the `Skill`
    /// channel — per level gained, each growing max HP by
    /// [`LEVEL_UP_MAX_HP_GROWTH`] and healing to the new maximum. Levels
    /// RATCHET: a curve below the stored level (the defeat penalty, a loaded
    /// save) changes nothing — milestones are kept. A stale save's earned
    /// levels surface at its next combat end of ANY outcome (lazy
    /// convergence, Decision 048) — in normal play the victory award syncs
    /// immediately, so only loaded saves ever converge late.
    fn sync_level(&mut self, events: &mut Vec<GameEvent>) {
        let target = level_for_xp(self.state.player.xp);
        while self.state.player.level < target {
            self.state.player.level = self.state.player.level.saturating_add(1);
            self.state.player.max_hp = self
                .state
                .player
                .max_hp
                .saturating_add(LEVEL_UP_MAX_HP_GROWTH);
            self.state.player.hp = self.state.player.max_hp;
            let (level, max_hp) = (self.state.player.level, self.state.player.max_hp);
            events.push(self.event(
                EventChannel::Skill,
                GameEventKind::LevelUp { level, max_hp },
            ));
        }
    }

    /// Remove an entity's room placement everywhere it appears (ticket #22), so a
    /// defeated enemy leaves no corpse to re-fight. Mirrors `take_at`'s removal of a
    /// taken item from its room.
    fn remove_entity_everywhere(&mut self, entity_id: &str) {
        for room in self.world.rooms.values_mut() {
            room.entities.retain(|id| id != entity_id);
        }
    }

    /// Whether the player's current location receives an announcement with
    /// `scope` (ticket #27). Pure and deterministic over the current room —
    /// the single delivery decision a multiplayer server later applies per
    /// session. Radius reuses the awareness plane + Chebyshev model
    /// ([`awareness::Position::cell_distance`]): a different region,
    /// subregion, or floor is "not near", and an unknown origin room
    /// delivers nothing.
    fn announcement_received(&self, scope: &AnnouncementScope) -> bool {
        let room = self.current_room();
        match scope {
            AnnouncementScope::World => true,
            AnnouncementScope::Region(id) => room.region == *id,
            AnnouncementScope::Subregion(id) => room.subregion.as_deref() == Some(id.as_str()),
            AnnouncementScope::Room(id) => room.id == *id,
            AnnouncementScope::Radius { room_id, radius } => self
                .world
                .rooms
                .get(room_id)
                .and_then(|origin| {
                    awareness::Position::from_room(room)
                        .cell_distance(&awareness::Position::from_room(origin))
                })
                .is_some_and(|distance| distance <= *radius),
        }
    }

    /// Emit an announcement iff the player receives it (ticket #27). The
    /// delivery decision happens at emission, so nothing scope-filtered ever
    /// reaches a client — clients render announcements and never decide
    /// receipt (REQ-005).
    fn announce(&mut self, announcement: &AuthoredAnnouncement) -> Option<GameEvent> {
        self.announcement_received(&announcement.scope).then(|| {
            self.event(
                EventChannel::Region,
                GameEventKind::Announcement {
                    severity: announcement.severity,
                    text: announcement.text.clone(),
                },
            )
        })
    }

    fn current_room(&self) -> &RoomDefinition {
        // Invariant (ticket #2 / REQ-004): `current_room_id` is always a key in
        // `world.rooms`. `Engine::try_new` rejects a world whose start room is
        // missing, and `move_direction` only sets `current_room_id` to a room it
        // has already fetched from `world.rooms`. This lookup is therefore
        // unreachable-as-`None` for any constructed `Engine`, so it is not a
        // malformed-data path.
        self.world
            .rooms
            .get(&self.state.current_room_id)
            .expect("current room is a try_new-validated invariant")
    }

    fn room_snapshot(&self, room: &RoomDefinition) -> RoomSnapshot {
        // Proximity/awareness is additive JSON state — the nearby things the
        // player can perceive from this cell, never canvas drawing instructions
        // (ticket #17, REQ-005). The client's Nearby panel reads `room.contents`.
        let contents = awareness::perceive(&self.world, room, &RadiusConfig::default())
            .into_iter()
            .map(|thing| {
                // Server-authored combat affordances (ticket #23): the client reads
                // these, never inferring hostility/attackability/stats. The perceived
                // `thing.id` resolves the entity for its typed roles + combat profile.
                let entity = self.world.entities.get(&thing.id);
                // `threat` is present iff hostile (the client flags an enemy by its
                // presence). Attackable needs a reachable hostile in a combat-enabled
                // room; `has_role(Role::Hostile)` already implies an Actor (contract).
                let threat = entity.filter(|e| e.has_role(Role::Hostile)).map(|_| {
                    let attackable = thing.proximity.is_interactable() && room.combat_enabled;
                    NearbyThreatSnapshot {
                        attack_command: attackable.then(|| format!("attack {}", thing.name)),
                        attackable,
                    }
                });
                // `stats` is present iff the entity has a combat profile (any
                // combatant); each stat is disclosed only when authored to (else
                // `None` → the client renders "unknown"). Nearby stats are the
                // authored maxima, not live HP.
                let stats = entity.and_then(|e| e.combat.as_ref()).map(|profile| {
                    let disclosed = profile.disclose_stats;
                    NearbyStatsSnapshot {
                        health: disclosed.then_some(profile.health),
                        max_health: disclosed.then_some(profile.health),
                        attack: disclosed.then_some(profile.attack),
                    }
                });
                NearbySnapshot {
                    id: thing.id,
                    name: thing.name,
                    kind: thing.kind.as_str().to_string(),
                    distance: thing.distance,
                    proximity: thing.proximity.as_str().to_string(),
                    interactable: thing.proximity.is_interactable(),
                    threat,
                    stats,
                }
            })
            .collect();
        RoomSnapshot {
            id: room.id.clone(),
            title: room.title.clone(),
            region: room.region.clone(),
            subregion: room.subregion.clone(),
            description: room.description.clone(),
            exits: room.exits.clone(),
            x: room.x,
            y: room.y,
            z: room.z,
            glyph: room.glyph,
            passable: room.passable,
            contents,
        }
    }

    fn map_snapshot(&self, room: &RoomDefinition) -> MapSnapshot {
        let rooms =
            self.world
                .rooms
                .values()
                .map(|map_room| {
                    let discovered = self.state.discovered_rooms.contains(&map_room.id);
                    // Ticket #33: presence markers are server-computed from LIVE
                    // placements (victory removes the entity, take/drop mutate the
                    // item list) and gated on `discovered`, so the payload never
                    // leaks fogged state — the client draws, never infers
                    // (Decision 041's principle). `hidden` things stay off the map
                    // exactly as the reveal rule keeps them out of the room view
                    // (#17 REQ-002): the map must never disclose what perceive
                    // conceals.
                    let has_hostiles = discovered
                        && map_room.entities.iter().any(|entity_id| {
                            self.world.entities.get(entity_id).is_some_and(|entity| {
                                !entity.hidden && entity.has_role(Role::Hostile)
                            })
                        });
                    let has_items = discovered
                        && map_room.items.iter().any(|item_id| {
                            self.world
                                .items
                                .get(item_id)
                                .is_some_and(|item| !item.hidden)
                        });
                    MapRoomSnapshot {
                        id: map_room.id.clone(),
                        title: map_room.title.clone(),
                        x: map_room.x,
                        y: map_room.y,
                        z: map_room.z,
                        glyph: map_room.glyph,
                        passable: map_room.passable,
                        discovered,
                        current: map_room.id == self.state.current_room_id,
                        exits: map_room.exits.clone(),
                        has_hostiles,
                        has_items,
                    }
                })
                .collect();

        MapSnapshot {
            region: room.region.clone(),
            subregion: room.subregion.clone(),
            current_room_id: self.state.current_room_id.clone(),
            rooms,
        }
    }

    /// The player's carried items as additive snapshot data (ticket #18, enriched
    /// in #20 with the authored `kind` placeholder + `flags`).
    ///
    /// Names/kind/flags are resolved from the item registry, which survives `take`
    /// (only a room's *placement* is removed, never the `world.items` entry), so
    /// the lookup resolves for every carried id; the id-name / `"item"`-kind /
    /// empty-flags fallbacks keep this off the panic path (§14) for an orphan id.
    /// `kind`/`flags` are authored item data — never invented here (REQ-002/007).
    fn pack_snapshot(&self) -> Vec<PackItemSnapshot> {
        self.state
            .pack
            .iter()
            .map(|item_id| {
                let item = self.world.items.get(item_id);
                PackItemSnapshot {
                    id: item_id.clone(),
                    name: item.map_or_else(|| item_id.clone(), |item| item.name.clone()),
                    kind: item
                        .and_then(|item| item.kind.clone())
                        .unwrap_or_else(|| "item".to_string()),
                    flags: item.map(|item| item.flags.clone()).unwrap_or_default(),
                }
            })
            .collect()
    }

    /// The active combat encounter as additive snapshot data (ticket #22), or
    /// `None` outside combat. Participants are a side-tagged list — the player and
    /// the enemy in v1 — so the client's battle modal renders both sides and the
    /// layout extends to multiple combatants later.
    fn combat_snapshot(&self) -> Option<CombatSnapshot> {
        let combat = self.state.combat.as_ref()?;
        let player = &self.state.player;
        Some(CombatSnapshot {
            round: combat.round,
            participants: vec![
                CombatantSnapshot {
                    id: player.id.clone(),
                    name: player.name.clone(),
                    hp: player.hp,
                    max_hp: player.max_hp,
                    side: "player".to_string(),
                },
                CombatantSnapshot {
                    id: combat.enemy_id.clone(),
                    name: combat.enemy_name.clone(),
                    hp: combat.enemy_hp,
                    max_hp: combat.enemy_max_hp,
                    side: "enemy".to_string(),
                },
            ],
            log: combat.log.clone(),
            queued_action: combat
                .queued_action
                .map(|action| action.as_str().to_string()),
        })
    }

    fn log(
        &mut self,
        channel: EventChannel,
        component: OutputComponent,
        text: impl Into<String>,
    ) -> GameEvent {
        self.event(
            channel,
            GameEventKind::LogMessage {
                component,
                text: text.into(),
            },
        )
    }

    const fn event(&mut self, channel: EventChannel, kind: GameEventKind) -> GameEvent {
        let event = GameEvent {
            event_id: self.next_event_id,
            tick: self.state.tick,
            channel,
            kind,
        };
        self.next_event_id = self.next_event_id.saturating_add(1);
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_world() -> WorldDefinition {
        let mut rooms = BTreeMap::new();
        rooms.insert(
            "a".to_string(),
            RoomDefinition {
                id: "a".to_string(),
                title: "A".to_string(),
                region: "test".to_string(),
                subregion: None,
                description: "Room A".to_string(),
                exits: BTreeMap::from([("east".to_string(), "b".to_string())]),
                x: 0,
                y: 0,
                z: 0,
                glyph: '.',
                passable: true,
                entities: Vec::new(),
                items: Vec::new(),
                combat_enabled: false,
            },
        );
        rooms.insert(
            "b".to_string(),
            RoomDefinition {
                id: "b".to_string(),
                title: "B".to_string(),
                region: "test".to_string(),
                subregion: None,
                description: "Room B".to_string(),
                exits: BTreeMap::new(),
                x: 1,
                y: 0,
                z: 0,
                glyph: '.',
                passable: true,
                entities: Vec::new(),
                items: Vec::new(),
                combat_enabled: false,
            },
        );

        let mut regions = BTreeMap::new();
        regions.insert(
            "test".to_string(),
            RegionDefinition {
                id: "test".to_string(),
                name: "Test".to_string(),
            },
        );

        WorldDefinition {
            id: "test".to_string(),
            title: "Test".to_string(),
            start_room_id: "a".to_string(),
            rooms,
            regions,
            subregions: BTreeMap::new(),
            entities: BTreeMap::new(),
            items: BTreeMap::new(),
            oaths: BTreeMap::new(),
            oath_id: None,
        }
    }

    fn cmd(input: &str) -> CommandRequest {
        CommandRequest {
            input: input.to_string(),
            actor_id: None,
        }
    }

    #[test]
    fn movement_discovers_rooms() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let response = engine.handle_command(cmd("east"));

        assert_eq!(response.snapshot.current_room_id, "b");
        assert!(response
            .snapshot
            .map
            .rooms
            .iter()
            .any(|room| room.id == "b" && room.discovered));
    }

    #[test]
    fn tick_increments_and_reports_value() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let events = engine.tick();
        assert_eq!(events.len(), 1, "an idle tick emits only the Tick event");
        assert!(matches!(events[0].kind, GameEventKind::Tick { value } if value == 1));
        assert_eq!(engine.snapshot().tick, 1);
    }

    #[test]
    fn event_ids_increment_sequentially() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let first = engine.tick();
        let second = engine.tick();
        assert_eq!(first[0].event_id, 1);
        assert_eq!(second[0].event_id, 2);
    }

    #[test]
    fn help_command_lists_directions() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let response = engine.handle_command(cmd("help"));
        assert!(response.events.iter().any(|e| matches!(
            &e.kind,
            GameEventKind::LogMessage { text, .. } if text.contains("look, north")
        )));
    }

    #[test]
    fn look_command_describes_current_room() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let response = engine.handle_command(cmd("look"));
        assert!(response.events.iter().any(|e| matches!(
            &e.kind,
            GameEventKind::LogMessage {
                component: OutputComponent::RoomHeader,
                text,
            } if text.contains("A [test]")
        )));
    }

    #[test]
    fn go_prefix_moves_between_rooms() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let response = engine.handle_command(cmd("go east"));
        assert_eq!(response.snapshot.current_room_id, "b");
    }

    #[test]
    fn unknown_command_is_reported_and_does_not_move() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let response = engine.handle_command(cmd("xyzzy"));
        assert!(response.events.iter().any(|e| matches!(
            &e.kind,
            GameEventKind::LogMessage { text, .. } if text.contains("I do not know how to")
        )));
        assert_eq!(response.snapshot.current_room_id, "a");
    }

    // H1: a movement command is accepted and moves the player.
    #[test]
    fn move_command_is_accepted() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let response = engine.handle_command(cmd("east"));
        assert!(response.accepted, "a movement command is accepted");
        assert_eq!(response.snapshot.current_room_id, "b");
    }

    // Review fix: a malformed bare direction (`east now`) must NOT move the player
    // even though `east` has a real exit, and is not accepted (no state mutation
    // on malformed input). Without the arity guard this would move to room "b".
    #[test]
    fn malformed_bare_direction_does_not_move() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let response = engine.handle_command(cmd("east now"));
        assert!(
            !response.accepted,
            "a malformed bare direction is not accepted"
        );
        assert_eq!(
            response.snapshot.current_room_id, "a",
            "a malformed bare direction does not move the player"
        );
    }

    // H2 (REQ-002): look <target> is accepted, echoes the preserved target, no move.
    #[test]
    fn look_with_target_is_accepted_and_echoes_target() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let response = engine.handle_command(cmd("look warden"));
        assert!(response.accepted, "look <target> is accepted");
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. } if text.contains("warden")
            )),
            "look <target> echoes the preserved target text"
        );
        assert_eq!(
            response.snapshot.current_room_id, "a",
            "looking at a target does not move the player"
        );
    }

    // H3 (REQ-004): unknown input is NOT accepted and mutates no state.
    #[test]
    fn unknown_command_is_not_accepted() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let response = engine.handle_command(cmd("xyzzy"));
        assert!(!response.accepted, "unknown input is not accepted");
        assert_eq!(
            response.snapshot.current_room_id, "a",
            "an unknown command does not change state"
        );
    }

    // H4: help is accepted.
    #[test]
    fn help_command_is_accepted() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let response = engine.handle_command(cmd("help"));
        assert!(response.accepted, "help is accepted");
    }

    // H5: bare look is accepted.
    #[test]
    fn look_command_is_accepted() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let response = engine.handle_command(cmd("look"));
        assert!(response.accepted, "look is accepted");
    }

    #[test]
    fn map_marks_only_the_current_room() {
        let engine = Engine::try_new(test_world()).expect("valid test world");
        let snapshot = engine.snapshot();
        let current = snapshot
            .map
            .rooms
            .iter()
            .find(|room| room.id == "a")
            .expect("start room present in map");
        let other = snapshot
            .map
            .rooms
            .iter()
            .find(|room| room.id == "b")
            .expect("other room present in map");
        assert!(current.current, "the start room is marked current");
        assert!(!other.current, "a non-start room is not marked current");
    }

    fn room_with(id: &str, passable: bool, exits: BTreeMap<String, String>) -> RoomDefinition {
        RoomDefinition {
            id: id.to_string(),
            title: id.to_string(),
            region: "test".to_string(),
            subregion: None,
            description: "d".to_string(),
            exits,
            x: 0,
            y: 0,
            z: 0,
            glyph: '.',
            passable,
            entities: Vec::new(),
            items: Vec::new(),
            combat_enabled: false,
        }
    }

    // Builds a world that auto-registers every region its rooms reference, so
    // tests exercising other invariants aren't tripped by the region check.
    fn world_with(start: &str, rooms: BTreeMap<String, RoomDefinition>) -> WorldDefinition {
        let regions = rooms
            .values()
            .map(|room| {
                (
                    room.region.clone(),
                    RegionDefinition {
                        id: room.region.clone(),
                        name: room.region.clone(),
                    },
                )
            })
            .collect();
        WorldDefinition {
            id: "w".to_string(),
            title: "W".to_string(),
            start_room_id: start.to_string(),
            rooms,
            regions,
            subregions: BTreeMap::new(),
            entities: BTreeMap::new(),
            items: BTreeMap::new(),
            oaths: BTreeMap::new(),
            oath_id: None,
        }
    }

    // ---- world-model v1 (ticket #6) helpers + tests ----

    fn region(id: &str) -> RegionDefinition {
        RegionDefinition {
            id: id.to_string(),
            name: id.to_string(),
        }
    }

    fn subregion(id: &str, parent_region: &str) -> SubregionDefinition {
        SubregionDefinition {
            id: id.to_string(),
            name: id.to_string(),
            region: parent_region.to_string(),
        }
    }

    fn entity(id: &str, kind: EntityKind, roles: &[&str], inventory: &[&str]) -> Entity {
        Entity {
            id: id.to_string(),
            name: id.to_string(),
            description: "d".to_string(),
            aliases: Vec::new(),
            kind,
            roles: roles.iter().copied().map(String::from).collect(),
            inventory: inventory.iter().copied().map(String::from).collect(),
            hidden: false,
            dialogue: None,
            combat: None,
        }
    }

    fn item(id: &str) -> Item {
        Item {
            id: id.to_string(),
            name: id.to_string(),
            description: "d".to_string(),
            aliases: Vec::new(),
            hidden: false,
            kind: None,
            flags: Vec::new(),
            value: 0,
            equipment: None,
        }
    }

    // A fully-valid world exercising regions, subregions, entities, and items.
    fn model_world() -> WorldDefinition {
        let mut room = room_with("a", true, BTreeMap::new());
        room.region = "r1".to_string();
        room.subregion = Some("s1".to_string());
        room.entities = vec!["npc".to_string()];
        room.items = vec!["it1".to_string()];

        let mut rooms = BTreeMap::new();
        rooms.insert("a".to_string(), room);

        let mut regions = BTreeMap::new();
        regions.insert("r1".to_string(), region("r1"));

        let mut subregions = BTreeMap::new();
        subregions.insert("s1".to_string(), subregion("s1", "r1"));

        let mut entities = BTreeMap::new();
        entities.insert(
            "npc".to_string(),
            entity("npc", EntityKind::Actor, &["combatant"], &[]),
        );
        entities.insert(
            "fix".to_string(),
            entity("fix", EntityKind::Fixture, &[], &[]),
        );
        entities.insert(
            "owner".to_string(),
            entity("owner", EntityKind::Actor, &["conversable"], &["it2"]),
        );

        let mut items = BTreeMap::new();
        items.insert("it1".to_string(), item("it1"));
        items.insert("it2".to_string(), item("it2"));

        WorldDefinition {
            id: "w".to_string(),
            title: "W".to_string(),
            start_room_id: "a".to_string(),
            rooms,
            regions,
            subregions,
            entities,
            items,
            oaths: BTreeMap::new(),
            oath_id: None,
        }
    }

    // T1 (REQ-001): a world with regions/subregions/entities/items validates, and
    // a room resolves its region and subregion through the registries.
    #[test]
    fn model_world_is_valid_and_refs_resolve() {
        let world = model_world();
        assert_eq!(world.validate(), Ok(()));
        let room = world.rooms.get("a").expect("room a");
        assert!(world.regions.contains_key(&room.region));
        assert!(world
            .subregions
            .contains_key(room.subregion.as_deref().expect("subregion")));
    }

    // T2 (REQ-002): the room model exposes title/description/passability/exits/
    // map-position metadata.
    #[test]
    fn room_exposes_metadata() {
        let world = model_world();
        let room = world.rooms.get("a").expect("room a");
        assert_eq!(room.title, "a");
        assert_eq!(room.description, "d");
        assert!(room.passable);
        assert!(room.exits.is_empty());
        assert_eq!((room.x, room.y, room.z), (0, 0, 0));
        assert_eq!(room.glyph, '.');
    }

    // T3 (REQ-003): one Entity type represents an NPC, an enemy (actor + the
    // combatant role), and an interactable (fixture) — distinguished by metadata.
    #[test]
    fn one_entity_type_carries_role_metadata() {
        let world = model_world();
        let actor = world.entities.get("npc").expect("npc");
        let fixture = world.entities.get("fix").expect("fix");
        assert_eq!(actor.kind, EntityKind::Actor);
        assert!(actor.roles.iter().any(|r| r.as_str() == "combatant"));
        assert_eq!(fixture.kind, EntityKind::Fixture);
    }

    // T4 (REQ-004): an item is referenced by a room (placement) and by an entity
    // (ownership); the registry holds the data, the containers hold only ids.
    #[test]
    fn items_are_referenced_by_room_and_owner() {
        let world = model_world();
        let room = world.rooms.get("a").expect("room a");
        assert_eq!(room.items, vec!["it1".to_string()]);
        let owner = world.entities.get("owner").expect("owner");
        assert_eq!(owner.inventory, vec!["it2".to_string()]);
        assert!(world.items.contains_key("it1") && world.items.contains_key("it2"));
    }

    // T5a (REQ-005): a room referencing a missing region is rejected.
    #[test]
    fn rejects_missing_room_region() {
        let mut world = model_world();
        world.rooms.get_mut("a").expect("a").region = "ghost".to_string();
        assert_eq!(
            world.validate(),
            Err(WorldValidationError::RoomRegionMissing {
                room_id: "a".to_string(),
                region: "ghost".to_string(),
            })
        );
    }

    // T5b (REQ-005): a room referencing a missing subregion is rejected.
    #[test]
    fn rejects_missing_room_subregion() {
        let mut world = model_world();
        world.rooms.get_mut("a").expect("a").subregion = Some("ghost".to_string());
        assert_eq!(
            world.validate(),
            Err(WorldValidationError::RoomSubregionMissing {
                room_id: "a".to_string(),
                subregion: "ghost".to_string(),
            })
        );
    }

    // T5c (REQ-005): a subregion referencing a missing parent region is rejected.
    #[test]
    fn rejects_missing_subregion_region() {
        let mut world = model_world();
        // Room `a` references subregion `s1`, whose parent region is now missing.
        // The subregion-parent check runs before the room↔subregion region-match
        // check (ticket #11 review), so this is SubregionRegionMissing — the room
        // stays attached to s1.
        world.subregions.get_mut("s1").expect("s1").region = "ghost".to_string();
        assert_eq!(
            world.validate(),
            Err(WorldValidationError::SubregionRegionMissing {
                subregion_id: "s1".to_string(),
                region: "ghost".to_string(),
            })
        );
    }

    // T5d (REQ-005): a room placing a missing entity is rejected.
    #[test]
    fn rejects_missing_room_entity() {
        let mut world = model_world();
        world.rooms.get_mut("a").expect("a").entities = vec!["ghost".to_string()];
        assert_eq!(
            world.validate(),
            Err(WorldValidationError::RoomEntityMissing {
                room_id: "a".to_string(),
                entity_id: "ghost".to_string(),
            })
        );
    }

    // T5e (REQ-005): a room placing a missing item is rejected.
    #[test]
    fn rejects_missing_room_item() {
        let mut world = model_world();
        world.rooms.get_mut("a").expect("a").items = vec!["ghost".to_string()];
        assert_eq!(
            world.validate(),
            Err(WorldValidationError::RoomItemMissing {
                room_id: "a".to_string(),
                item_id: "ghost".to_string(),
            })
        );
    }

    // T5f (REQ-005): an entity owning a missing item is rejected.
    #[test]
    fn rejects_missing_entity_item() {
        let mut world = model_world();
        world.entities.get_mut("owner").expect("owner").inventory = vec!["ghost".to_string()];
        assert_eq!(
            world.validate(),
            Err(WorldValidationError::EntityItemMissing {
                entity_id: "owner".to_string(),
                item_id: "ghost".to_string(),
            })
        );
    }

    // T6 (REQ-005): every new validation error's Display names the offending ids.
    #[test]
    fn new_validation_errors_name_the_offender() {
        assert!(WorldValidationError::RoomRegionMissing {
            room_id: "a".to_string(),
            region: "r".to_string(),
        }
        .to_string()
        .contains("room 'a' references missing region 'r'"));
        assert!(WorldValidationError::RoomSubregionMissing {
            room_id: "a".to_string(),
            subregion: "s".to_string(),
        }
        .to_string()
        .contains("room 'a' references missing subregion 's'"));
        assert!(WorldValidationError::SubregionRegionMissing {
            subregion_id: "s".to_string(),
            region: "r".to_string(),
        }
        .to_string()
        .contains("subregion 's' references missing region 'r'"));
        assert!(WorldValidationError::RoomEntityMissing {
            room_id: "a".to_string(),
            entity_id: "e".to_string(),
        }
        .to_string()
        .contains("room 'a' references missing entity 'e'"));
        assert!(WorldValidationError::RoomItemMissing {
            room_id: "a".to_string(),
            item_id: "i".to_string(),
        }
        .to_string()
        .contains("room 'a' references missing item 'i'"));
        assert!(WorldValidationError::EntityItemMissing {
            entity_id: "e".to_string(),
            item_id: "i".to_string(),
        }
        .to_string()
        .contains("entity 'e' references missing item 'i'"));
    }

    // REQ-006: a world whose invariants all hold constructs (no false rejection).
    #[test]
    fn validate_accepts_valid_world() {
        assert_eq!(test_world().validate(), Ok(()));
    }

    // REQ-005: a room stored under a key that differs from its own id is rejected.
    #[test]
    fn validate_rejects_key_id_mismatch() {
        let mut rooms = BTreeMap::new();
        rooms.insert(
            "wrong_key".to_string(),
            room_with("a", true, BTreeMap::new()),
        );
        assert_eq!(
            world_with("a", rooms).validate(),
            Err(WorldValidationError::RoomKeyMismatch {
                key: "wrong_key".to_string(),
                room_id: "a".to_string(),
            })
        );
    }

    // REQ-001: a missing start room is rejected.
    #[test]
    fn validate_rejects_missing_start_room() {
        let mut rooms = BTreeMap::new();
        rooms.insert("a".to_string(), room_with("a", true, BTreeMap::new()));
        assert_eq!(
            world_with("nope", rooms).validate(),
            Err(WorldValidationError::StartRoomMissing {
                start_room_id: "nope".to_string(),
            })
        );
    }

    // REQ-002: an impassable start room is rejected.
    #[test]
    fn validate_rejects_impassable_start_room() {
        let mut rooms = BTreeMap::new();
        rooms.insert("a".to_string(), room_with("a", false, BTreeMap::new()));
        assert_eq!(
            world_with("a", rooms).validate(),
            Err(WorldValidationError::StartRoomImpassable {
                start_room_id: "a".to_string(),
            })
        );
    }

    // REQ-003: an exit pointing at a non-existent room is rejected, naming the offender.
    #[test]
    fn validate_rejects_dangling_exit() {
        let mut rooms = BTreeMap::new();
        rooms.insert(
            "a".to_string(),
            room_with(
                "a",
                true,
                BTreeMap::from([("north".to_string(), "ghost_room".to_string())]),
            ),
        );
        assert_eq!(
            world_with("a", rooms).validate(),
            Err(WorldValidationError::DanglingExit {
                room_id: "a".to_string(),
                direction: "north".to_string(),
                target_room_id: "ghost_room".to_string(),
            })
        );
    }

    // REQ-001/003 detail: every variant's Display names the offending entity.
    #[test]
    fn validation_error_messages_name_the_offender() {
        assert!(WorldValidationError::StartRoomMissing {
            start_room_id: "x".to_string(),
        }
        .to_string()
        .contains("start room 'x' does not exist"));
        assert!(WorldValidationError::StartRoomImpassable {
            start_room_id: "x".to_string(),
        }
        .to_string()
        .contains("start room 'x' is not passable"));
        assert!(WorldValidationError::DanglingExit {
            room_id: "a".to_string(),
            direction: "north".to_string(),
            target_room_id: "z".to_string(),
        }
        .to_string()
        .contains("room 'a' exit 'north' points to missing room 'z'"));
        assert!(WorldValidationError::RoomKeyMismatch {
            key: "k".to_string(),
            room_id: "i".to_string(),
        }
        .to_string()
        .contains("room stored under key 'k' has mismatched id 'i'"));
    }

    // REQ-006 detail: try_new seeds the documented initial state.
    #[test]
    fn try_new_seeds_initial_state() {
        let engine = Engine::try_new(test_world()).expect("valid test world");
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.tick, 0);
        assert_eq!(snapshot.current_room_id, "a");
        assert_eq!(snapshot.player.level, 1);
        assert_eq!(snapshot.player.xp, 0);
        assert_eq!(snapshot.player.hp, 20);
        assert_eq!(snapshot.player.max_hp, 20);
        assert_eq!(snapshot.player.focus, 5);
        assert_eq!(snapshot.player.max_focus, 5);
    }

    // REQ-001/004: try_new surfaces the typed error instead of constructing+panicking.
    #[test]
    fn try_new_rejects_invalid_world() {
        let mut rooms = BTreeMap::new();
        rooms.insert("a".to_string(), room_with("a", true, BTreeMap::new()));
        assert_eq!(
            Engine::try_new(world_with("missing", rooms)).err(),
            Some(WorldValidationError::StartRoomMissing {
                start_room_id: "missing".to_string(),
            })
        );
    }

    // REQ-006 detail: the start room is discovered at construction.
    #[test]
    fn try_new_marks_start_discovered() {
        let engine = Engine::try_new(test_world()).expect("valid test world");
        let snapshot = engine.snapshot();
        let start = snapshot
            .map
            .rooms
            .iter()
            .find(|room| room.id == "a")
            .expect("start room present in map");
        assert!(start.discovered, "start room is discovered at construction");
    }

    // Empty input is rejected with a prompt and changes nothing.
    #[test]
    fn empty_command_input_waits() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let response = engine.handle_command(cmd("   "));
        assert!(!response.accepted, "blank input is not accepted");
        assert!(response.events.iter().any(|e| matches!(
            &e.kind,
            GameEventKind::LogMessage { text, .. } if text.contains("The world waits")
        )));
    }

    // Moving in a direction the room has no exit for is refused, room unchanged.
    #[test]
    fn move_with_no_exit_is_refused() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let response = engine.handle_command(cmd("north"));
        assert!(response.events.iter().any(|e| matches!(
            &e.kind,
            GameEventKind::LogMessage { text, .. } if text.contains("cannot go that way")
        )));
        assert_eq!(
            response.snapshot.current_room_id, "a",
            "a refused move does not change the current room"
        );
    }

    // Moving into an impassable (but existing) room is blocked, room unchanged.
    #[test]
    fn move_into_impassable_room_is_blocked() {
        let mut rooms = BTreeMap::new();
        rooms.insert(
            "a".to_string(),
            room_with(
                "a",
                true,
                BTreeMap::from([("east".to_string(), "c".to_string())]),
            ),
        );
        rooms.insert("c".to_string(), room_with("c", false, BTreeMap::new()));
        let mut engine = Engine::try_new(world_with("a", rooms)).expect("valid world");
        let response = engine.handle_command(cmd("east"));
        assert!(response.events.iter().any(|e| matches!(
            &e.kind,
            GameEventKind::LogMessage { text, .. } if text.contains("Something blocks that way")
        )));
        assert_eq!(
            response.snapshot.current_room_id, "a",
            "a blocked move does not change the current room"
        );
    }

    // ---- ticket #7: beginner vertical slice (oath lifecycle + boss) ----

    // A synthetic world for oath/boss mechanics: `town` (start) holds a non-boss
    // `bystander`; `town --east--> lair` where `warden` (role "boss") waits. One
    // designated oath `o1`.
    fn oath_world() -> WorldDefinition {
        let mut town = room_with(
            "town",
            true,
            BTreeMap::from([("east".to_string(), "lair".to_string())]),
        );
        town.region = "r".to_string();
        town.entities = vec!["bystander".to_string()];

        let mut lair = room_with("lair", true, BTreeMap::new());
        lair.region = "r".to_string();
        lair.entities = vec!["warden".to_string()];

        let mut rooms = BTreeMap::new();
        rooms.insert("town".to_string(), town);
        rooms.insert("lair".to_string(), lair);

        let mut regions = BTreeMap::new();
        regions.insert("r".to_string(), region("r"));

        let mut warden = entity("warden", EntityKind::Actor, &["boss"], &[]);
        warden.name = "The Warden".to_string();
        // Ticket #29: the boss role contract requires a combat profile. One
        // opening strike (4) fells the warden, so confront-based test flows
        // stay one round long; attack 0 keeps the player untouched.
        warden.combat = Some(CombatProfile {
            health: 4,
            attack: 0,
            disclose_stats: false,
            xp: 0,
            coins: 0,
        });
        let mut entities = BTreeMap::new();
        entities.insert("warden".to_string(), warden);
        entities.insert(
            "bystander".to_string(),
            entity("bystander", EntityKind::Actor, &["bystander"], &[]),
        );

        let mut oaths = BTreeMap::new();
        oaths.insert(
            "o1".to_string(),
            OathDefinition {
                id: "o1".to_string(),
                title: "Test Oath".to_string(),
                description: "Do the thing.".to_string(),
                issuer_id: None,
                source: None,
                fulfillment_announcements: Vec::new(),
                objective_item_id: None,
            },
        );

        WorldDefinition {
            id: "w".to_string(),
            title: "W".to_string(),
            start_room_id: "town".to_string(),
            rooms,
            regions,
            subregions: BTreeMap::new(),
            entities,
            items: BTreeMap::new(),
            oaths,
            oath_id: Some("o1".to_string()),
        }
    }

    // REQ-002: swearing records an active oath and emits a typed OathSworn on the
    // Oath channel plus a narrative carrying the oath title + description.
    #[test]
    fn swear_sets_oath_active_and_emits_oath_sworn() {
        let mut engine = Engine::try_new(oath_world()).expect("valid oath world");
        let response = engine.handle_command(cmd("swear"));
        assert!(response.accepted, "swear is accepted");

        let oath = response.snapshot.oath.expect("oath recorded in snapshot");
        assert_eq!(oath.oath_id, "o1");
        assert_eq!(oath.title, "Test Oath");
        assert_eq!(oath.status, OathStatus::Sworn);

        assert!(
            response.events.iter().any(|e| matches!(
                (&e.channel, &e.kind),
                (EventChannel::Oath, GameEventKind::OathSworn { oath_id, title })
                    if oath_id.as_str() == "o1" && title.as_str() == "Test Oath"
            )),
            "emits OathSworn{{o1, Test Oath}} on the Oath channel"
        );
        assert!(
            response.events.iter().any(|e| matches!(
                (&e.channel, &e.kind),
                (EventChannel::Narrative, GameEventKind::LogMessage { text, .. })
                    if text.contains("Test Oath") && text.contains("Do the thing.")
            )),
            "emits a Narrative line with the oath title and description"
        );
    }

    // REQ-002 branch: a second swear is refused and the oath is unchanged.
    #[test]
    fn swear_twice_is_refused_with_message() {
        let mut engine = Engine::try_new(oath_world()).expect("valid oath world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        let response = engine.handle_command(cmd("swear"));
        assert!(!response.accepted, "a second swear is refused");
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. } if text.contains("already sworn")
            )),
            "refusal explains the oath is already sworn"
        );
        assert_eq!(
            response.snapshot.oath.expect("still sworn").status,
            OathStatus::Sworn
        );
    }

    // REQ-002 branch: with no designated module oath, swear is refused, no oath.
    #[test]
    fn swear_without_designated_oath_is_refused() {
        let mut world = oath_world();
        world.oath_id = None;
        let mut engine = Engine::try_new(world).expect("valid world without an oath");
        let response = engine.handle_command(cmd("swear"));
        assert!(
            !response.accepted,
            "swearing with no module oath is refused"
        );
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. } if text.contains("no oath")
            )),
            "refusal explains there is no oath to swear"
        );
        assert!(response.snapshot.oath.is_none(), "no oath recorded");
    }

    // B1 (REQ-001, ticket #29): confront with the oath sworn + boss present
    // STARTS the real encounter — CombatStarted for the boss, the authored
    // stats land in CombatState BY VALUE (the asymmetric 12/4 pair kills
    // engage-arg-transposition mutants), the opening round resolves, and the
    // oath STAYS Sworn: fulfillment now rides recovery, not confrontation.
    #[test]
    fn confront_with_sworn_oath_starts_the_boss_encounter() {
        let mut engine = Engine::try_new(boss_world(12, 4)).expect("valid boss world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        assert!(
            engine.handle_command(cmd("east")).accepted,
            "move to the lair"
        );

        let response = engine.handle_command(cmd("confront"));
        assert!(response.accepted, "confront engages with boss + sworn oath");
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatStarted { enemy_id, enemy_name, .. }
                    if enemy_id == "warden" && enemy_name == "The Warden"
            )),
            "confront emits CombatStarted for the boss: {:?}",
            response.events
        );
        assert!(
            response.snapshot.combat.is_some(),
            "the encounter renders in the snapshot"
        );
        let combat = engine
            .state
            .combat
            .as_ref()
            .expect("the encounter is active");
        assert_eq!(combat.enemy_max_hp, 12, "authored boss health by value");
        assert_eq!(combat.enemy_attack, 4, "authored boss attack by value");
        assert_eq!(combat.enemy_hp, 8, "the opening strike landed (12 - 4)");
        assert_eq!(
            response.snapshot.player.hp, 16,
            "the boss's return landed (20 - 4)"
        );
        assert!(
            !response
                .events
                .iter()
                .any(|e| matches!(e.kind, GameEventKind::OathFulfilled { .. })),
            "a victoryless confrontation fulfills nothing"
        );
        assert_eq!(
            response.snapshot.oath.expect("oath present").status,
            OathStatus::Sworn,
            "the oath stays sworn at the start of the fight"
        );
    }

    // REQ-004 branch (T17): a non-boss entity in the room is not confrontable —
    // kills the `role == "boss"` / `.find(..).any(..)` mutants.
    #[test]
    fn confront_with_only_a_non_boss_entity_is_refused() {
        let mut engine = Engine::try_new(oath_world()).expect("valid oath world");
        // The start room holds only `bystander` (no "boss" role).
        let response = engine.handle_command(cmd("confront"));
        assert!(!response.accepted, "a non-boss entity is not confrontable");
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. } if text.contains("nothing here to confront")
            )),
            "reports nothing to confront when no boss is present"
        );
    }

    // REQ-004 branch: boss present but no oath sworn → refused, no oath created.
    #[test]
    fn confront_boss_without_swearing_is_refused() {
        let mut engine = Engine::try_new(oath_world()).expect("valid oath world");
        assert!(
            engine.handle_command(cmd("east")).accepted,
            "reach the lair"
        );
        let response = engine.handle_command(cmd("confront"));
        assert!(!response.accepted, "confront without an oath is refused");
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. } if text.contains("sworn no oath")
            )),
            "refusal says you have sworn no oath"
        );
        assert!(
            response.snapshot.oath.is_none(),
            "a refused confront creates no oath"
        );
    }

    // REQ-004 branch (premise inverted at #29): with the oath already kept
    // and the boss still ALIVE — the objective recovered loose, no fight —
    // confront refuses with the reworded line and starts nothing.
    #[test]
    fn confront_when_oath_already_fulfilled_is_refused() {
        let mut engine = Engine::try_new(loose_objective_world()).expect("valid world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        assert!(
            engine.handle_command(cmd("take sigil")).accepted,
            "the loose objective fulfills without a fight"
        );
        assert!(engine.handle_command(cmd("east")).accepted);
        let response = engine.handle_command(cmd("confront"));
        assert!(
            !response.accepted,
            "confronting with the oath already kept is refused"
        );
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. }
                    if text == "Your oath is already kept; there is no cause to confront The Warden."
            )),
            "the refusal is premise-neutral about the living boss: {:?}",
            response.events
        );
        assert!(
            response.snapshot.combat.is_none(),
            "no encounter starts against a kept oath"
        );
        assert_eq!(
            response.snapshot.oath.expect("oath present").status,
            OathStatus::Fulfilled,
            "the oath remains fulfilled"
        );
    }

    // REQ-007 (core): a designated oath absent from the registry is rejected.
    #[test]
    fn validate_rejects_missing_designated_oath() {
        let mut world = oath_world();
        world.oath_id = Some("ghost".to_string());
        assert_eq!(
            world.validate(),
            Err(WorldValidationError::OathMissing {
                oath_id: "ghost".to_string(),
            })
        );
    }

    // REQ-007 (core): a world whose designated oath exists validates.
    #[test]
    fn validate_accepts_world_with_a_valid_designated_oath() {
        assert_eq!(oath_world().validate(), Ok(()));
    }

    // REQ-007 (core): OathMissing's Display names the offending oath id.
    #[test]
    fn oath_missing_display_names_the_offender() {
        assert!(WorldValidationError::OathMissing {
            oath_id: "x".to_string(),
        }
        .to_string()
        .contains("designated oath 'x' does not exist"));
    }

    // REQ-001: a freshly started game produces a typed start-room event.
    #[test]
    fn begin_emits_start_room_entered_and_description() {
        let mut engine = Engine::try_new(oath_world()).expect("valid oath world");
        let events = engine.begin();
        assert!(
            events.iter().any(|e| matches!(
                (&e.channel, &e.kind),
                (EventChannel::Room, GameEventKind::RoomEntered { room_id, .. })
                    if room_id.as_str() == "town"
            )),
            "begin emits RoomEntered for the start room"
        );
        assert!(
            events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage {
                    component: OutputComponent::RoomHeader,
                    ..
                }
            )),
            "begin emits the room header/description"
        );
    }

    // REQ-002/004: help lists the new commands.
    #[test]
    fn help_lists_swear_and_confront() {
        let mut engine = Engine::try_new(oath_world()).expect("valid oath world");
        let response = engine.handle_command(cmd("help"));
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. }
                    if text.contains("swear") && text.contains("confront")
            )),
            "help mentions swear and confront"
        );
    }

    // ---- ticket #11: room region must match the subregion's parent region ----

    // REQ-001: a room whose own region differs from its subregion's parent region
    // is rejected with a typed error naming the room, its region, the subregion,
    // and the subregion's parent region. Both regions exist, so this isolates the
    // mismatch from RoomRegionMissing / SubregionRegionMissing.
    #[test]
    fn rejects_room_region_subregion_parent_mismatch() {
        let mut world = model_world();
        world.regions.insert("r2".to_string(), region("r2"));
        // room `a` sits in subregion `s1` (parent region `r1`) but declares `r2`.
        world.rooms.get_mut("a").expect("a").region = "r2".to_string();
        assert_eq!(
            world.validate(),
            Err(WorldValidationError::RoomSubregionRegionMismatch {
                room_id: "a".to_string(),
                room_region: "r2".to_string(),
                subregion: "s1".to_string(),
                subregion_region: "r1".to_string(),
            })
        );
    }

    // REQ-002: when a room's region and its subregion's parent region agree, the
    // world validates (no false rejection).
    #[test]
    fn accepts_room_region_subregion_parent_match() {
        let world = model_world();
        let room = world.rooms.get("a").expect("a");
        let sub = world
            .subregions
            .get(room.subregion.as_deref().expect("subregion"))
            .expect("s1");
        assert_eq!(
            room.region, sub.region,
            "precondition: room and subregion-parent regions agree"
        );
        assert_eq!(world.validate(), Ok(()));
    }

    // REQ-001 detail: the mismatch error's Display names all four fields.
    #[test]
    fn room_subregion_region_mismatch_display_names_fields() {
        assert_eq!(
            WorldValidationError::RoomSubregionRegionMismatch {
                room_id: "a".to_string(),
                room_region: "x".to_string(),
                subregion: "s".to_string(),
                subregion_region: "y".to_string(),
            }
            .to_string(),
            "room 'a' (region 'x') references subregion 's' whose parent region is 'y'"
        );
    }

    // ---- ticket #17: proximity look + snapshot contents ----

    // An engine whose start room "org" (0,0,0) holds an actor + item at distance 0,
    // and a room "far" (2,0,0) on the same plane holds an actor at distance 2.
    fn proximity_engine() -> Engine {
        let mut org = room_with("org", true, BTreeMap::new());
        org.entities = vec!["ally".to_string()];
        org.items = vec!["coin".to_string()];

        let mut far = room_with("far", true, BTreeMap::new());
        far.x = 2;
        far.entities = vec!["guard".to_string()];

        let mut rooms = BTreeMap::new();
        rooms.insert("org".to_string(), org);
        rooms.insert("far".to_string(), far);
        let mut world = world_with("org", rooms);

        let mut ally = entity("ally", EntityKind::Actor, &[], &[]);
        ally.name = "Ally".to_string();
        ally.description = "the loyal ally".to_string();
        world.entities.insert("ally".to_string(), ally);

        let mut guard = entity("guard", EntityKind::Actor, &[], &[]);
        guard.name = "Guard".to_string();
        guard.description = "a wary guard".to_string();
        world.entities.insert("guard".to_string(), guard);

        let mut coin = item("coin");
        coin.name = "Coin".to_string();
        coin.description = "a copper coin".to_string();
        world.items.insert("coin".to_string(), coin);

        Engine::try_new(world).expect("valid proximity world")
    }

    fn narrative_text(response: &CommandResponse) -> String {
        response
            .events
            .iter()
            .find_map(|event| match &event.kind {
                GameEventKind::LogMessage {
                    component: OutputComponent::NarrativeMessage,
                    text,
                } => Some(text.clone()),
                _ => None,
            })
            .expect("a narrative message in the response")
    }

    fn look(engine: &mut Engine, input: &str) -> String {
        narrative_text(&engine.handle_command(CommandRequest {
            input: input.to_string(),
            actor_id: None,
        }))
    }

    // REQ-004: `look <target>` resolves an interactable entity (same cell) and
    // reports its description (observed — deviation #2). Narrative channel.
    #[test]
    fn look_interactable_entity_describes_it() {
        let mut engine = proximity_engine();
        let text = look(&mut engine, "look ally");
        assert!(text.contains("You study Ally."), "study line: {text}");
        assert!(
            text.contains("the loyal ally"),
            "description observed: {text}"
        );
    }

    // REQ-004: the same resolver path works for an item (kills the item path).
    #[test]
    fn look_interactable_item_describes_it() {
        let mut engine = proximity_engine();
        let text = look(&mut engine, "look coin");
        assert!(text.contains("You study Coin."), "study line: {text}");
        assert!(text.contains("a copper coin"), "item description: {text}");
    }

    // REQ-003: a target within sight but beyond reach is visible-not-interactable.
    #[test]
    fn look_visible_but_out_of_reach_is_too_far() {
        let mut engine = proximity_engine();
        let text = look(&mut engine, "look guard");
        assert!(text.contains("Guard"), "names the target: {text}");
        assert!(
            text.contains("too far off to examine closely"),
            "too-far line: {text}"
        );
    }

    // REQ-004: an unmatched target names the query back, mutates nothing.
    #[test]
    fn look_unknown_target_reports_nothing_nearby() {
        let mut engine = proximity_engine();
        let text = look(&mut engine, "look dragon");
        assert!(
            text.contains("nothing like 'dragon' nearby"),
            "none line: {text}"
        );
    }

    // REQ-007: bare `look` is unchanged — it still describes the room.
    #[test]
    fn bare_look_still_describes_the_room() {
        let mut engine = proximity_engine();
        let text = look(&mut engine, "look");
        assert_eq!(text, "d");
        assert!(!text.contains("You study"));
    }

    // REQ-001/005: the snapshot exposes nearby things as JSON contents, with an
    // exact (interactable) AND a visible (not interactable) entry distinguished.
    #[test]
    fn snapshot_contents_list_exact_and_visible_things() {
        let engine = proximity_engine();
        let contents = engine.snapshot().room.contents;

        let ally = contents
            .iter()
            .find(|thing| thing.id == "ally")
            .expect("ally in contents");
        assert_eq!(ally.name, "Ally");
        assert_eq!(ally.kind, "actor");
        assert_eq!(ally.distance, 0);
        assert_eq!(ally.proximity, "exact");
        assert!(ally.interactable);

        let coin = contents
            .iter()
            .find(|thing| thing.id == "coin")
            .expect("coin in contents");
        assert_eq!(coin.kind, "item");

        let guard = contents
            .iter()
            .find(|thing| thing.id == "guard")
            .expect("guard in contents");
        assert_eq!(guard.distance, 2);
        assert_eq!(guard.proximity, "visible");
        assert!(!guard.interactable);
    }

    // REQ-002: nothing in sight → empty contents (honest empty state preserved).
    #[test]
    fn snapshot_contents_empty_when_nothing_in_sight() {
        let mut rooms = BTreeMap::new();
        rooms.insert("solo".to_string(), room_with("solo", true, BTreeMap::new()));
        let engine = Engine::try_new(world_with("solo", rooms)).expect("valid solo world");
        assert!(engine.snapshot().room.contents.is_empty());
    }

    // ---- ticket #18: talk / take commands ----

    // A world for the nearby actions. Origin cell `org` (0,0,0) holds the
    // conversable `mara`, the non-conversable actor `warden`, two ground items
    // `coin`+`gem`, and a hidden `buried`. The adjacent cell `near` (1,0,0; d1,
    // Interactable) holds `relic`. The far cell `far` (2,0,0; d2, Visible-only)
    // holds `idol` and the actor `scout`.
    fn interaction_engine() -> Engine {
        let mut org = room_with("org", true, BTreeMap::new());
        org.entities = vec!["mara".to_string(), "warden".to_string()];
        org.items = vec!["coin".to_string(), "gem".to_string(), "buried".to_string()];

        let mut near = room_with("near", true, BTreeMap::new());
        near.x = 1;
        near.items = vec!["relic".to_string()];

        let mut far = room_with("far", true, BTreeMap::new());
        far.x = 2;
        far.entities = vec!["scout".to_string()];
        far.items = vec!["idol".to_string()];

        let mut rooms = BTreeMap::new();
        rooms.insert("org".to_string(), org);
        rooms.insert("near".to_string(), near);
        rooms.insert("far".to_string(), far);
        let mut world = world_with("org", rooms);

        world.entities.insert(
            "mara".to_string(),
            entity("mara", EntityKind::Actor, &["conversable"], &[]),
        );
        world.entities.insert(
            "warden".to_string(),
            entity("warden", EntityKind::Actor, &[], &[]),
        );
        world.entities.insert(
            "scout".to_string(),
            entity("scout", EntityKind::Actor, &[], &[]),
        );
        for id in ["coin", "gem", "relic", "idol"] {
            world.items.insert(id.to_string(), item(id));
        }
        let mut buried = item("buried");
        buried.hidden = true;
        world.items.insert("buried".to_string(), buried);

        Engine::try_new(world).expect("valid interaction world")
    }

    // The first LogMessage text of any component (take success uses ItemCard, not
    // NarrativeMessage, so `narrative_text` would not see it).
    fn log_text(response: &CommandResponse) -> String {
        response
            .events
            .iter()
            .find_map(|event| match &event.kind {
                GameEventKind::LogMessage { text, .. } => Some(text.clone()),
                _ => None,
            })
            .expect("a log message in the response")
    }

    // REQ-003: talking to a reachable conversable actor responds and does NOT move.
    #[test]
    fn talk_to_conversable_actor_responds_without_moving() {
        let mut engine = interaction_engine();
        let response = engine.handle_command(cmd("talk mara"));
        assert!(
            response.accepted,
            "talking to a reachable actor is accepted"
        );
        let text = log_text(&response);
        assert!(
            text.contains("mara") && text.contains("ready to talk"),
            "conversable greeting: {text}"
        );
        assert_eq!(
            response.snapshot.current_room_id, "org",
            "talk does not move the player"
        );
    }

    // REQ-003: a reachable NON-conversable actor still gets a response (accepted),
    // with the no-conversation flavor — kills the `conversable` role branch.
    #[test]
    fn talk_to_non_conversable_actor_still_responds() {
        let mut engine = interaction_engine();
        let response = engine.handle_command(cmd("talk warden"));
        assert!(
            response.accepted,
            "a reachable actor is accepted regardless of role"
        );
        let text = log_text(&response);
        assert!(
            text.contains("warden") && text.contains("nothing to say"),
            "non-conversable flavor: {text}"
        );
    }

    // REQ-004: a visible-but-out-of-reach actor is too far to talk to; refused, no move.
    #[test]
    fn talk_to_too_far_actor_is_refused() {
        let mut engine = interaction_engine();
        let response = engine.handle_command(cmd("talk scout"));
        assert!(!response.accepted, "an out-of-reach actor is refused");
        assert!(
            log_text(&response).contains("too far away to talk"),
            "too-far line"
        );
        assert_eq!(response.snapshot.current_room_id, "org");
    }

    // REQ-003 (kind gate): you cannot talk to a non-actor (an item); refused.
    #[test]
    fn talk_to_non_actor_is_refused() {
        let mut engine = interaction_engine();
        let response = engine.handle_command(cmd("talk coin"));
        assert!(!response.accepted, "talking to an item is refused");
        assert!(
            log_text(&response).contains("cannot hold a conversation"),
            "non-actor line"
        );
    }

    // REQ-004 sibling: an unknown talk target names the query back; refused.
    #[test]
    fn talk_to_unknown_target_is_refused() {
        let mut engine = interaction_engine();
        let response = engine.handle_command(cmd("talk dragon"));
        assert!(!response.accepted);
        assert!(
            log_text(&response).contains("no one like 'dragon'"),
            "unknown line"
        );
    }

    // REQ-005: taking a reachable world item carries it (Inventory/ItemCard event),
    // removes it from the room so it leaves `contents`, leaves OTHER items, no move.
    #[test]
    fn take_reachable_item_carries_and_removes_from_contents() {
        let mut engine = interaction_engine();
        let response = engine.handle_command(cmd("take coin"));
        assert!(response.accepted, "taking a reachable item is accepted");
        assert!(
            response.events.iter().any(|e| matches!(
                (&e.channel, &e.kind),
                (
                    EventChannel::Inventory,
                    GameEventKind::LogMessage {
                        component: OutputComponent::ItemCard,
                        text,
                    }
                ) if text.contains("You take the coin")
            )),
            "emits an Inventory/ItemCard take line"
        );
        let pack_ids: Vec<&str> = response
            .snapshot
            .pack
            .iter()
            .map(|carried| carried.id.as_str())
            .collect();
        assert_eq!(pack_ids, vec!["coin"], "coin is now carried");
        let content_ids: Vec<&str> = response
            .snapshot
            .room
            .contents
            .iter()
            .map(|thing| thing.id.as_str())
            .collect();
        assert!(!content_ids.contains(&"coin"), "coin left nearby contents");
        assert!(content_ids.contains(&"gem"), "the other item remains");
        assert_eq!(
            response.snapshot.current_room_id, "org",
            "take does not move the player"
        );
    }

    // REQ-005 (room_id): an item one cell away (adjacent interactable) is removed
    // from THAT room — proving take uses the resolver's room_id, not the origin.
    #[test]
    fn take_item_from_adjacent_cell_removes_it_there() {
        let mut engine = interaction_engine();
        assert!(
            engine
                .snapshot()
                .room
                .contents
                .iter()
                .any(|thing| thing.id == "relic"),
            "relic is in nearby contents before take"
        );
        let response = engine.handle_command(cmd("take relic"));
        assert!(response.accepted);
        assert!(
            response
                .snapshot
                .pack
                .iter()
                .any(|carried| carried.id == "relic"),
            "relic is carried"
        );
        assert!(
            !response
                .snapshot
                .room
                .contents
                .iter()
                .any(|thing| thing.id == "relic"),
            "relic removed from the adjacent room's contents"
        );
    }

    // REQ-006: a visible-but-out-of-reach item is too far; refused, state preserved.
    #[test]
    fn take_too_far_item_is_refused_and_preserves_state() {
        let mut engine = interaction_engine();
        let response = engine.handle_command(cmd("take idol"));
        assert!(!response.accepted, "an out-of-reach item is refused");
        assert!(
            log_text(&response).contains("too far away to reach"),
            "too-far line"
        );
        assert!(response.snapshot.pack.is_empty(), "nothing carried");
        assert!(
            response
                .snapshot
                .room
                .contents
                .iter()
                .any(|thing| thing.id == "idol"),
            "idol still present (state preserved)"
        );
    }

    // REQ-006: you cannot take a non-item (an actor); refused, state preserved.
    #[test]
    fn take_non_item_is_refused() {
        let mut engine = interaction_engine();
        let response = engine.handle_command(cmd("take warden"));
        assert!(!response.accepted, "taking an actor is refused");
        assert!(
            log_text(&response).contains("cannot carry"),
            "non-item line"
        );
        assert!(response.snapshot.pack.is_empty());
    }

    // REQ-006: unknown and hidden take targets both resolve to nothing; refused.
    #[test]
    fn take_unknown_or_hidden_target_is_refused() {
        let mut engine = interaction_engine();
        let unknown = engine.handle_command(cmd("take dragon"));
        assert!(!unknown.accepted);
        assert!(log_text(&unknown).contains("nothing like 'dragon'"));
        // `buried` is hidden → excluded by perceive → resolves to nothing.
        let hidden = engine.handle_command(cmd("take buried"));
        assert!(!hidden.accepted, "a hidden item cannot be taken");
        assert!(hidden.snapshot.pack.is_empty());
    }

    // REQ-005: carried items keep pickup order (Vec push order).
    #[test]
    fn pack_preserves_pickup_order() {
        let mut engine = interaction_engine();
        engine.handle_command(cmd("take gem"));
        let response = engine.handle_command(cmd("take coin"));
        let ids: Vec<&str> = response
            .snapshot
            .pack
            .iter()
            .map(|carried| carried.id.as_str())
            .collect();
        assert_eq!(ids, vec!["gem", "coin"], "pack keeps pickup order");
    }

    // REQ-005/006: a taken item is gone from the world and cannot be taken twice.
    #[test]
    fn taking_the_same_item_twice_is_refused() {
        let mut engine = interaction_engine();
        assert!(engine.handle_command(cmd("take coin")).accepted);
        let again = engine.handle_command(cmd("take coin"));
        assert!(!again.accepted, "the item is gone after the first take");
        assert_eq!(again.snapshot.pack.len(), 1, "no double-carry");
    }

    // REQ-007: the snapshot after a take exposes the item with its registry NAME
    // resolved (not the id).
    #[test]
    fn snapshot_pack_resolves_item_name_after_take() {
        let mut engine = interaction_engine();
        // Distinct name + matching alias so `take coin` still resolves while the
        // carried NAME differs from the id — proving name-from-registry resolution.
        {
            let coin = engine.world.items.get_mut("coin").expect("coin");
            coin.name = "Copper Coin".to_string();
            coin.aliases = vec!["coin".to_string()];
        }
        let response = engine.handle_command(cmd("take coin"));
        let carried = response.snapshot.pack.first().expect("one carried item");
        assert_eq!(carried.id, "coin");
        assert_eq!(
            carried.name, "Copper Coin",
            "name is resolved from world.items"
        );
    }

    // REQ-007 (defensive, §14): pack_snapshot falls back to the id when a carried id
    // is absent from the item registry — the no-panic path.
    #[test]
    fn snapshot_pack_falls_back_to_id_when_item_missing() {
        let mut engine = interaction_engine();
        engine.state.pack.push("ghost".to_string());
        let pack = engine.snapshot().pack;
        let ghost = pack
            .iter()
            .find(|carried| carried.id == "ghost")
            .expect("ghost id present");
        assert_eq!(
            ghost.name, "ghost",
            "a missing registry entry falls back to the id"
        );
        assert_eq!(
            ghost.kind, "item",
            "a missing registry entry defaults kind to the placeholder"
        );
        assert!(
            ghost.flags.is_empty(),
            "a missing registry entry carries no flags"
        );
    }

    // REQ-002/004: help lists the new talk and take verbs.
    #[test]
    fn help_lists_talk_and_take() {
        let mut engine = interaction_engine();
        let response = engine.handle_command(cmd("help"));
        let text = log_text(&response);
        assert!(
            text.contains("talk") && text.contains("take"),
            "help mentions talk and take: {text}"
        );
    }

    // ---- ticket #19: NPC dialogue + oath offering ----

    // A world for the offered-oath flow. The start room `town` holds: the oath
    // ISSUER `mara` (conversable, full oath dialogue); `bram`, a conversable NPC
    // carrying an oath block that does NOT issue the designated oath (must fall
    // back to its greeting — guards the issuer filter); `clerk`, conversable with
    // no dialogue (generic line); `statue`, a non-conversable actor with no
    // dialogue (generic line); and the boss `warden`. `hollow_bell` is issued by
    // `mara`. All sit in the start cell, so each is interactable.
    fn dialogue_world() -> WorldDefinition {
        let mut town = room_with("town", true, BTreeMap::new());
        town.entities = vec![
            "mara".to_string(),
            "bram".to_string(),
            "clerk".to_string(),
            "statue".to_string(),
            "warden".to_string(),
        ];

        let mut rooms = BTreeMap::new();
        rooms.insert("town".to_string(), town);
        let mut world = world_with("town", rooms);

        let mut mara = entity("mara", EntityKind::Actor, &["conversable"], &[]);
        mara.name = "Mara".to_string();
        mara.dialogue = Some(EntityDialogue {
            greeting: "Mara nods at you.".to_string(),
            oath: Some(OathDialogue {
                offer: "Mara: the bell is hollow — will you swear to mend it?".to_string(),
                sworn: "Mara: you have sworn; go and mend the bell.".to_string(),
                fulfilled: "Mara: the bell rings again. Thank you.".to_string(),
            }),
        });
        world.entities.insert("mara".to_string(), mara);

        let mut bram = entity("bram", EntityKind::Actor, &["conversable"], &[]);
        bram.dialogue = Some(EntityDialogue {
            greeting: "Bram shrugs.".to_string(),
            oath: Some(OathDialogue {
                offer: "BRAM-OFFER-SHOULD-NOT-SHOW".to_string(),
                sworn: "BRAM-SWORN-SHOULD-NOT-SHOW".to_string(),
                fulfilled: "BRAM-FULFILLED-SHOULD-NOT-SHOW".to_string(),
            }),
        });
        world.entities.insert("bram".to_string(), bram);

        world.entities.insert(
            "clerk".to_string(),
            entity("clerk", EntityKind::Actor, &["conversable"], &[]),
        );
        world.entities.insert(
            "statue".to_string(),
            entity("statue", EntityKind::Actor, &[], &[]),
        );
        let mut dialogue_warden = entity("warden", EntityKind::Actor, &["boss"], &[]);
        // Ticket #29: the boss role contract requires a combat profile; one
        // opening strike fells it so dialogue flows stay short, and it
        // carries the oath's objective for the fulfilled-state line.
        dialogue_warden.combat = Some(CombatProfile {
            health: 4,
            attack: 0,
            disclose_stats: false,
            xp: 0,
            coins: 0,
        });
        dialogue_warden.inventory = vec!["relic".to_string()];
        world.entities.insert("warden".to_string(), dialogue_warden);
        world.items.insert("relic".to_string(), item("relic"));

        let mut oaths = BTreeMap::new();
        oaths.insert(
            "hollow_bell".to_string(),
            OathDefinition {
                id: "hollow_bell".to_string(),
                title: "The Hollow Bell".to_string(),
                description: "Mend the bell.".to_string(),
                issuer_id: Some("mara".to_string()),
                source: Some("hollowmere".to_string()),
                fulfillment_announcements: Vec::new(),
                objective_item_id: Some("relic".to_string()),
            },
        );
        world.oaths = oaths;
        world.oath_id = Some("hollow_bell".to_string());

        world
    }

    fn dialogue_engine() -> Engine {
        Engine::try_new(dialogue_world()).expect("valid dialogue world")
    }

    // T1 (REQ-001): a dialogue NPC that does NOT issue the designated oath returns
    // its authored greeting, never oath lines — guards the issuer filter.
    #[test]
    fn talk_dialogue_npc_that_is_not_issuer_returns_greeting() {
        let mut engine = dialogue_engine();
        let text = narrative_text(&engine.handle_command(cmd("talk bram")));
        assert!(text.contains("Bram shrugs."), "authored greeting: {text}");
        assert!(
            !text.contains("SHOULD-NOT-SHOW"),
            "a non-issuer must never speak oath lines: {text}"
        );
    }

    // T2a (REQ-001): a conversable NPC with no dialogue keeps the #18 generic line.
    #[test]
    fn talk_conversable_npc_without_dialogue_uses_generic_line() {
        let mut engine = dialogue_engine();
        let text = narrative_text(&engine.handle_command(cmd("talk clerk")));
        assert!(
            text.contains("ready to talk"),
            "generic conversable line: {text}"
        );
    }

    // T2b (REQ-001): a non-conversable actor with no dialogue keeps the generic
    // "nothing to say" line.
    #[test]
    fn talk_non_conversable_npc_without_dialogue_has_nothing_to_say() {
        let mut engine = dialogue_engine();
        let text = narrative_text(&engine.handle_command(cmd("talk statue")));
        assert!(
            text.contains("nothing to say"),
            "generic non-conversable line: {text}"
        );
    }

    // T3 (REQ-002): talking to the issuer while unsworn returns the offer line and
    // records the offer so swear becomes permitted.
    #[test]
    fn talk_issuer_offers_oath_and_records_offer() {
        let mut engine = dialogue_engine();
        let response = engine.handle_command(cmd("talk mara"));
        assert!(
            response.accepted,
            "talking to a reachable actor is accepted"
        );
        let text = narrative_text(&response);
        assert!(
            text.contains("swear to mend it"),
            "offer introduces the oath: {text}"
        );
        assert_eq!(
            engine.state.offered_oath_id.as_deref(),
            Some("hollow_bell"),
            "talking to the issuer records the offer"
        );
    }

    // T4 (REQ-003): swearing an issuer-offered oath before the offer is refused and
    // names the issuer; no oath recorded, nothing offered.
    #[test]
    fn swear_before_offer_is_refused_and_guides_to_issuer() {
        let mut engine = dialogue_engine();
        let response = engine.handle_command(cmd("swear"));
        assert!(
            !response.accepted,
            "an unoffered issuer-oath cannot be sworn"
        );
        let text = log_text(&response);
        assert!(
            text.contains("offered") && text.contains("Mara"),
            "refusal guides the player to the issuer: {text}"
        );
        assert!(response.snapshot.oath.is_none(), "no oath recorded");
        assert_eq!(engine.state.offered_oath_id, None, "nothing offered yet");
    }

    // T5 (REQ-003 backward-compat): an issuer-less oath is globally swearable with
    // no offer (the pre-#19 behavior).
    #[test]
    fn swear_oath_without_issuer_needs_no_offer() {
        let mut engine = Engine::try_new(oath_world()).expect("valid issuer-less oath world");
        assert!(
            engine.handle_command(cmd("swear")).accepted,
            "an issuer-less oath swears without an offer"
        );
    }

    // T6 (REQ-004): after the offer, swear binds the oath and emits the UNCHANGED
    // OathSworn shape.
    #[test]
    fn swear_after_offer_binds_and_emits_oath_sworn() {
        let mut engine = dialogue_engine();
        assert!(engine.handle_command(cmd("talk mara")).accepted);
        let response = engine.handle_command(cmd("swear"));
        assert!(response.accepted, "swear after the offer is accepted");
        let oath = response.snapshot.oath.expect("oath recorded in snapshot");
        assert_eq!(oath.oath_id, "hollow_bell");
        assert_eq!(oath.status, OathStatus::Sworn);
        assert!(
            response.events.iter().any(|e| matches!(
                (&e.channel, &e.kind),
                (EventChannel::Oath, GameEventKind::OathSworn { oath_id, title })
                    if oath_id.as_str() == "hollow_bell" && title.as_str() == "The Hollow Bell"
            )),
            "emits the unchanged OathSworn{{oath_id,title}} shape"
        );
    }

    // T7 (REQ-005): once sworn, the issuer's dialogue reflects the sworn state.
    #[test]
    fn dialogue_reflects_sworn_state() {
        let mut engine = dialogue_engine();
        assert!(engine.handle_command(cmd("talk mara")).accepted);
        assert!(engine.handle_command(cmd("swear")).accepted);
        let text = narrative_text(&engine.handle_command(cmd("talk mara")));
        assert!(text.contains("you have sworn"), "sworn-state line: {text}");
    }

    // T8 (REQ-005): once fulfilled, the issuer's dialogue reflects the fulfilled
    // state.
    #[test]
    fn dialogue_reflects_fulfilled_state() {
        let mut engine = dialogue_engine();
        assert!(engine.handle_command(cmd("talk mara")).accepted);
        assert!(engine.handle_command(cmd("swear")).accepted);
        // Ticket #29: confront now FIGHTS the boss (one strike fells the
        // fixture warden, dropping the relic) and recovery fulfills.
        assert!(
            engine.handle_command(cmd("confront")).accepted,
            "the boss is present and the oath is sworn"
        );
        assert!(
            engine.handle_command(cmd("take relic")).accepted,
            "the dropped objective is takeable"
        );
        let text = narrative_text(&engine.handle_command(cmd("talk mara")));
        assert!(text.contains("rings again"), "fulfilled-state line: {text}");
    }

    // T9 (REQ-006): an oath whose issuer is not a known entity is rejected at the
    // construction boundary.
    #[test]
    fn validate_rejects_oath_with_unknown_issuer() {
        let mut world = dialogue_world();
        world
            .oaths
            .get_mut("hollow_bell")
            .expect("oath present")
            .issuer_id = Some("ghost".to_string());
        let err = world
            .validate()
            .expect_err("an unknown issuer must be rejected");
        assert!(
            matches!(
                &err,
                WorldValidationError::OathIssuerMissing { oath_id, issuer_id }
                    if oath_id.as_str() == "hollow_bell" && issuer_id.as_str() == "ghost"
            ),
            "unexpected error: {err}"
        );
        assert!(
            Engine::try_new(world).is_err(),
            "try_new rejects a world with a dangling oath issuer"
        );
    }

    // ---- ticket #20: inventory v1 (list / look-carried / drop / enriched pack) ----

    // T1 (REQ-001): a taken item's id is stored in carried game state.
    #[test]
    fn take_stores_item_id_in_pack() {
        let mut engine = interaction_engine();
        assert!(engine.handle_command(cmd("take coin")).accepted);
        assert!(
            engine.state.pack.iter().any(|id| id == "coin"),
            "the carried id is stored in state"
        );
    }

    // T3a (REQ-002): an item with no authored kind/flags surfaces the placeholder
    // kind and empty flags in the snapshot.
    #[test]
    fn pack_snapshot_defaults_kind_and_empty_flags() {
        let mut engine = interaction_engine();
        let response = engine.handle_command(cmd("take coin"));
        let carried = response
            .snapshot
            .pack
            .iter()
            .find(|item| item.id == "coin")
            .expect("coin carried");
        assert_eq!(carried.kind, "item", "kind defaults to the placeholder");
        assert!(carried.flags.is_empty(), "no authored flags");
    }

    // T3b (REQ-002): authored kind/flags surface by value (never invented).
    #[test]
    fn pack_snapshot_surfaces_authored_kind_and_flags() {
        let mut engine = interaction_engine();
        {
            let coin = engine.world.items.get_mut("coin").expect("coin");
            coin.kind = Some("relic".to_string());
            coin.flags = vec!["bound".to_string()];
        }
        let response = engine.handle_command(cmd("take coin"));
        let carried = response
            .snapshot
            .pack
            .iter()
            .find(|item| item.id == "coin")
            .expect("coin carried");
        assert_eq!(carried.kind, "relic");
        assert_eq!(carried.flags, vec!["bound".to_string()]);
    }

    // T5 (REQ-003): `inventory`/`pack`/`i` list carried names or an honest empty
    // state (exact messages).
    #[test]
    fn inventory_lists_carried_items_or_empty() {
        let mut engine = interaction_engine();
        {
            // Rename but keep "coin" as an alias so `take coin` still resolves.
            let coin = engine.world.items.get_mut("coin").expect("coin");
            coin.name = "Copper Coin".to_string();
            coin.aliases = vec!["coin".to_string()];
        }
        assert_eq!(
            log_text(&engine.handle_command(cmd("inventory"))),
            "You are carrying nothing."
        );
        assert!(engine.handle_command(cmd("take coin")).accepted);
        assert_eq!(
            log_text(&engine.handle_command(cmd("i"))),
            "You are carrying: Copper Coin."
        );
    }

    // T6 (REQ-004): `look` resolves a carried item from the pack — by name AND by
    // alias — once it has left the room (nearby resolution still runs first).
    #[test]
    fn look_resolves_carried_item_by_name_and_alias() {
        let mut engine = interaction_engine();
        {
            let coin = engine.world.items.get_mut("coin").expect("coin");
            coin.name = "Copper Coin".to_string();
            coin.description = "a worn copper coin".to_string();
            // "coin" keeps `take coin` resolving; "penny" exercises alias lookup.
            coin.aliases = vec!["coin".to_string(), "penny".to_string()];
        }
        assert!(engine.handle_command(cmd("take coin")).accepted);
        let by_name = narrative_text(&engine.handle_command(cmd("look copper coin")));
        assert!(
            by_name.contains("You examine the Copper Coin you are carrying."),
            "pack fallback by name: {by_name}"
        );
        assert!(
            by_name.contains("a worn copper coin"),
            "carried description: {by_name}"
        );
        let by_alias = narrative_text(&engine.handle_command(cmd("look penny")));
        assert!(
            by_alias.contains("you are carrying"),
            "pack fallback resolves by alias: {by_alias}"
        );
    }

    // T8 (REQ-005): `drop` removes the item from the pack and places it in the
    // current cell, where awareness surfaces it again.
    #[test]
    fn drop_places_carried_item_in_cell_visible_via_awareness() {
        let mut engine = interaction_engine();
        assert!(engine.handle_command(cmd("take coin")).accepted);
        let dropped = engine.handle_command(cmd("drop coin"));
        assert!(dropped.accepted, "drop is accepted");
        assert_eq!(log_text(&dropped), "You drop the coin.");
        assert!(
            !engine.state.pack.iter().any(|id| id == "coin"),
            "no longer carried"
        );
        assert!(
            dropped
                .snapshot
                .room
                .contents
                .iter()
                .any(|thing| thing.id == "coin" && thing.interactable),
            "the dropped item is visible through awareness at the cell"
        );
    }

    // T9 (REQ-006): dropping an uncarried target — or an orphan pack id with no
    // registry entry — is refused with no state change.
    #[test]
    fn drop_uncarried_or_orphan_target_is_refused_without_state_change() {
        let mut engine = interaction_engine();
        let refused = engine.handle_command(cmd("drop coin"));
        assert!(!refused.accepted, "dropping an uncarried item is refused");
        assert!(
            log_text(&refused).contains("aren't carrying"),
            "refusal explains nothing is carried: {}",
            log_text(&refused)
        );
        assert!(engine.state.pack.is_empty(), "no state change");

        engine.state.pack.push("ghost".to_string());
        let orphan = engine.handle_command(cmd("drop ghost"));
        assert!(
            !orphan.accepted,
            "an orphan pack id has no name/alias to match"
        );
        assert_eq!(
            engine.state.pack,
            vec!["ghost".to_string()],
            "the orphan id is untouched"
        );
    }

    // REQ-006: duplicate carried references are ambiguous and refused without
    // losing either copy. This guards `drop` against removing every matching id.
    #[test]
    fn drop_duplicate_carried_match_is_refused_without_state_change() {
        let mut engine = interaction_engine();
        engine
            .world
            .rooms
            .get_mut("org")
            .expect("origin room")
            .items
            .retain(|item_id| item_id != "coin");
        engine.state.pack = vec!["coin".to_string(), "coin".to_string()];

        let before_pack = engine.state.pack.clone();
        let before_room_items = engine.current_room().items.clone();
        let response = engine.handle_command(cmd("drop coin"));

        assert!(!response.accepted, "duplicate carried refs are refused");
        assert!(
            log_text(&response).contains("More than one carried item matches"),
            "refusal identifies ambiguity: {}",
            log_text(&response)
        );
        assert_eq!(engine.state.pack, before_pack, "pack unchanged");
        assert_eq!(
            engine.current_room().items,
            before_room_items,
            "room placement unchanged"
        );
    }

    // T12 (REQ-003): help lists the new drop and inventory verbs.
    #[test]
    fn help_lists_drop_and_inventory() {
        let mut engine = interaction_engine();
        let text = log_text(&engine.handle_command(cmd("help")));
        assert!(
            text.contains("drop") && text.contains("inventory"),
            "help names the new verbs: {text}"
        );
    }

    // ---- ticket #21: entity role contracts ----

    // T1 (REQ-003): `from_tag` maps each known tag — and the `"conversable"` synonym —
    // to its `Role` by value; unknown tags are `None`.
    #[test]
    fn role_from_tag_maps_known_and_synonym_tags() {
        assert_eq!(Role::from_tag("talkable"), Some(Role::Talkable));
        assert_eq!(Role::from_tag("conversable"), Some(Role::Talkable));
        assert_eq!(Role::from_tag("oath_giver"), Some(Role::OathGiver));
        assert_eq!(Role::from_tag("shopkeeper"), Some(Role::Shopkeeper));
        assert_eq!(Role::from_tag("combatant"), Some(Role::Combatant));
        assert_eq!(Role::from_tag("boss"), Some(Role::Boss));
        assert_eq!(Role::from_tag("bystander"), None);
        assert_eq!(Role::from_tag(""), None);
    }

    // T-as_str (REQ-002): each `Role`'s canonical tag — covers every `as_str` arm.
    #[test]
    fn role_as_str_returns_each_canonical_tag() {
        assert_eq!(Role::Talkable.as_str(), "talkable");
        assert_eq!(Role::OathGiver.as_str(), "oath_giver");
        assert_eq!(Role::Shopkeeper.as_str(), "shopkeeper");
        assert_eq!(Role::Combatant.as_str(), "combatant");
        assert_eq!(Role::Boss.as_str(), "boss");
    }

    // T2 (REQ-003): `has_role` reflects a multi-tag entity — true AND false (kills a
    // `.any`→`.all` mutant); unknown tags grant nothing.
    #[test]
    fn entity_has_role_reflects_multiple_tags() {
        let mut npc = entity("npc", EntityKind::Actor, &["talkable", "shopkeeper"], &[]);
        assert!(npc.has_role(Role::Talkable));
        assert!(npc.has_role(Role::Shopkeeper));
        assert!(!npc.has_role(Role::Boss));
        assert!(!npc.has_role(Role::OathGiver));
        npc.roles = vec!["bystander".to_string()];
        assert!(
            !npc.has_role(Role::Talkable),
            "an unknown tag grants no role"
        );
    }

    // T6 (REQ-003): `talk` recognizes the `"talkable"` tag through the typed helper
    // (the legacy code matched only the literal `"conversable"`).
    #[test]
    fn talk_resolves_talkable_via_typed_role_tag() {
        let mut engine = interaction_engine();
        engine
            .world
            .entities
            .get_mut("warden")
            .expect("warden")
            .roles = vec!["talkable".to_string()];
        let text = narrative_text(&engine.handle_command(cmd("talk warden")));
        assert!(
            text.contains("ready to talk"),
            "talkable tag resolves: {text}"
        );
    }

    // A one-room world with an Actor `giver` declaring `oath_giver`, plus the given
    // oaths `(id, optional issuer)` — for the oath_giver contract matrix.
    fn oath_giver_world(oaths: &[(&str, Option<&str>)]) -> WorldDefinition {
        let mut rooms = BTreeMap::new();
        rooms.insert("a".to_string(), room_with("a", true, BTreeMap::new()));
        let mut world = world_with("a", rooms);
        world.entities.insert(
            "giver".to_string(),
            entity("giver", EntityKind::Actor, &["oath_giver"], &[]),
        );
        for (oath_id, issuer) in oaths {
            world.oaths.insert(
                (*oath_id).to_string(),
                OathDefinition {
                    id: (*oath_id).to_string(),
                    title: "T".to_string(),
                    description: "d".to_string(),
                    issuer_id: issuer.map(str::to_owned),
                    source: None,
                    fulfillment_announcements: Vec::new(),
                    objective_item_id: None,
                },
            );
        }
        world
    }

    // T3 (REQ-001/002): the oath_giver contract — must be named as some oath's
    // `issuer_id`. Covers 0 / 1 / multi-with-match / multi-none.
    #[test]
    fn validate_oath_giver_contract_matrix() {
        let err = oath_giver_world(&[])
            .validate()
            .expect_err("no issuing oath");
        assert!(
            matches!(&err, WorldValidationError::RoleContractUnmet { role, .. } if role.as_str() == "oath_giver"),
            "unexpected: {err}"
        );
        assert_eq!(
            oath_giver_world(&[("o1", Some("giver"))]).validate(),
            Ok(())
        );
        assert_eq!(
            oath_giver_world(&[("o1", None), ("o2", Some("giver"))]).validate(),
            Ok(()),
            "one matching oath among many satisfies the contract"
        );
        assert!(
            oath_giver_world(&[("o1", None), ("o2", None)])
                .validate()
                .is_err(),
            "no matching oath leaves the contract unmet"
        );
    }

    // T4 (REQ-001/002): an interaction role on a `Fixture` is rejected (naming the
    // entity + role); the same role on an `Actor` validates.
    #[test]
    fn validate_rejects_role_on_fixture_and_accepts_on_actor() {
        let mut rooms = BTreeMap::new();
        rooms.insert("a".to_string(), room_with("a", true, BTreeMap::new()));

        let mut fixture_world = world_with("a", rooms.clone());
        fixture_world.entities.insert(
            "statue".to_string(),
            entity("statue", EntityKind::Fixture, &["combatant"], &[]),
        );
        let err = fixture_world.validate().expect_err("role on a fixture");
        assert!(
            matches!(
                &err,
                WorldValidationError::RoleContractUnmet { entity_id, role, .. }
                    if entity_id.as_str() == "statue" && role.as_str() == "combatant"
            ),
            "unexpected: {err}"
        );

        let mut actor_world = world_with("a", rooms);
        actor_world.entities.insert(
            "brute".to_string(),
            entity("brute", EntityKind::Actor, &["combatant"], &[]),
        );
        assert_eq!(actor_world.validate(), Ok(()));
    }

    // T5 (REQ-002): the typed error's Display names the entity, role, and missing.
    #[test]
    fn role_contract_error_names_entity_role_and_missing() {
        let err = WorldValidationError::RoleContractUnmet {
            entity_id: "statue".to_string(),
            role: "combatant".to_string(),
            missing: "an Actor kind".to_string(),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("statue") && msg.contains("combatant") && msg.contains("Actor"),
            "message: {msg}"
        );
    }

    // ============================================================
    //  ticket #22 — combat encounter v1
    // ============================================================

    // A world for combat tests. Start room "field" (0,0,0, combat_enabled) holds,
    // in placement order: a hostile "stray" (health/attack from args), a SECOND
    // hostile "brute" (so bare-attack's first-hostile order is testable), a
    // non-hostile actor "elder", and a fixture "idol" — all at distance 0. "haven"
    // (combat_enabled = false) is the gate-refusal room; "clearing" (combat_enabled,
    // no hostile) is the no-target room; "ridge" (x = 2, combat_enabled) holds a
    // distant hostile "wolf" (visible, not interactable) for the too-far path.
    fn combat_world(stray_health: u32, stray_attack: u32) -> WorldDefinition {
        let mut field = room_with(
            "field",
            true,
            BTreeMap::from([
                ("east".to_string(), "haven".to_string()),
                ("south".to_string(), "clearing".to_string()),
            ]),
        );
        field.combat_enabled = true;
        field.entities = vec![
            "stray".to_string(),
            "brute".to_string(),
            "elder".to_string(),
            "idol".to_string(),
        ];

        let haven = room_with(
            "haven",
            true,
            BTreeMap::from([("west".to_string(), "field".to_string())]),
        ); // combat_enabled stays false — the gate-refusal room.

        let mut clearing = room_with("clearing", true, BTreeMap::new());
        clearing.combat_enabled = true; // enabled, but holds no hostile.

        let mut ridge = room_with("ridge", true, BTreeMap::new());
        ridge.x = 2; // distance 2 from "field" → visible, not interactable.
        ridge.combat_enabled = true;
        ridge.entities = vec!["wolf".to_string()];

        let mut rooms = BTreeMap::new();
        rooms.insert("field".to_string(), field);
        rooms.insert("haven".to_string(), haven);
        rooms.insert("clearing".to_string(), clearing);
        rooms.insert("ridge".to_string(), ridge);
        let mut world = world_with("field", rooms);

        let mut stray = entity("stray", EntityKind::Actor, &["combatant", "hostile"], &[]);
        stray.name = "Stray".to_string();
        stray.combat = Some(CombatProfile {
            health: stray_health,
            attack: stray_attack,
            disclose_stats: false,
            xp: 0,
            coins: 0,
        });
        world.entities.insert("stray".to_string(), stray);

        let mut brute = entity("brute", EntityKind::Actor, &["combatant", "hostile"], &[]);
        brute.name = "Brute".to_string();
        brute.combat = Some(CombatProfile {
            health: 99,
            attack: 99,
            disclose_stats: false,
            xp: 0,
            coins: 0,
        });
        world.entities.insert("brute".to_string(), brute);

        let mut elder = entity("elder", EntityKind::Actor, &["talkable"], &[]);
        elder.name = "Elder".to_string();
        world.entities.insert("elder".to_string(), elder);

        let mut idol = entity("idol", EntityKind::Fixture, &[], &[]);
        idol.name = "Idol".to_string();
        world.entities.insert("idol".to_string(), idol);

        let mut wolf = entity("wolf", EntityKind::Actor, &["combatant", "hostile"], &[]);
        wolf.name = "Wolf".to_string();
        wolf.combat = Some(CombatProfile {
            health: 5,
            attack: 1,
            disclose_stats: false,
            xp: 0,
            coins: 0,
        });
        world.entities.insert("wolf".to_string(), wolf);

        world
    }

    fn combat_engine(stray_health: u32, stray_attack: u32) -> Engine {
        Engine::try_new(combat_world(stray_health, stray_attack)).expect("valid combat world")
    }

    // The active combat sub-state of a response snapshot.
    fn active_combat(response: &CommandResponse) -> oathstar_protocol::CombatSnapshot {
        response
            .snapshot
            .combat
            .clone()
            .expect("combat is active in this snapshot")
    }

    fn enemy_of(
        combat: &oathstar_protocol::CombatSnapshot,
    ) -> &oathstar_protocol::CombatantSnapshot {
        combat
            .participants
            .iter()
            .find(|p| p.side == "enemy")
            .expect("an enemy participant")
    }

    fn player_of(
        combat: &oathstar_protocol::CombatSnapshot,
    ) -> &oathstar_protocol::CombatantSnapshot {
        combat
            .participants
            .iter()
            .find(|p| p.side == "player")
            .expect("a player participant")
    }

    fn room_has(response: &CommandResponse, name: &str) -> bool {
        response
            .snapshot
            .room
            .contents
            .iter()
            .any(|thing| thing.name == name)
    }

    // C1/REQ-001: `attack <hostile>` in a combat-enabled room starts an encounter
    // and emits CombatStarted naming the enemy by id.
    #[test]
    fn attack_named_hostile_starts_combat() {
        let mut engine = combat_engine(10, 3);
        let response = engine.handle_command(cmd("attack stray"));
        assert!(response.accepted, "attacking a hostile is accepted");
        assert_eq!(enemy_of(&active_combat(&response)).name, "Stray");
        assert!(
            response.events.iter().any(|e| matches!(
                (&e.channel, &e.kind),
                (EventChannel::Combat, GameEventKind::CombatStarted { enemy_id, enemy_name, .. })
                    if enemy_id == "stray" && enemy_name == "Stray"
            )),
            "emits CombatStarted{{stray, Stray}} on the Combat channel"
        );
    }

    // C2/REQ-001 + first-hostile order: a bare `attack` engages the FIRST hostile in
    // placement order (stray), not the second (brute) — locks find_hostile ordering.
    #[test]
    fn bare_attack_engages_first_hostile_in_room() {
        let mut engine = combat_engine(10, 3);
        let response = engine.handle_command(cmd("attack"));
        assert!(response.accepted);
        assert_eq!(
            enemy_of(&active_combat(&response)).name,
            "Stray",
            "bare attack engages the first-authored hostile, not Brute"
        );
    }

    // C3/REQ-002: a strike deals exactly PLAYER_STRIKE_DAMAGE to the enemy and emits
    // a Combat/CombatMessage event. (attack 0 keeps the player's HP out of it.)
    #[test]
    fn strike_deals_deterministic_damage_and_emits_combat_message() {
        let mut engine = combat_engine(10, 0);
        let response = engine.handle_command(cmd("strike"));
        assert_eq!(
            enemy_of(&active_combat(&response)).hp,
            10 - PLAYER_STRIKE_DAMAGE,
            "enemy HP drops by exactly the strike damage"
        );
        assert!(
            response.events.iter().any(|e| matches!(
                (&e.channel, &e.kind),
                (
                    EventChannel::Combat,
                    GameEventKind::LogMessage { component: OutputComponent::CombatMessage, text }
                ) if text.contains("You strike Stray")
            )),
            "emits a Combat/CombatMessage strike line"
        );
    }

    // C4/REQ-003: a surviving enemy returns deterministic damage to the player.
    #[test]
    fn surviving_enemy_returns_deterministic_damage() {
        let mut engine = combat_engine(20, 3);
        let response = engine.handle_command(cmd("fight"));
        assert_eq!(
            player_of(&active_combat(&response)).hp,
            20 - 3,
            "player HP drops by exactly the enemy's attack"
        );
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. } if text.contains("Stray hits you for 3")
            )),
            "emits the enemy's return-strike line"
        );
    }

    // C5/REQ-004 victory (enemy HP lands EXACTLY on 0 → kills `<= 0` → `>`) + enemy
    // removal + bystander survival (kills remove_entity_everywhere `!=` → `==`).
    #[test]
    fn victory_at_zero_hp_ends_combat_removes_enemy_keeps_bystander() {
        let mut engine = combat_engine(PLAYER_STRIKE_DAMAGE as u32, 0);
        let response = engine.handle_command(cmd("attack stray"));
        assert!(response.accepted);
        assert!(
            response.snapshot.combat.is_none(),
            "combat state is cleared on victory"
        );
        assert!(
            response.events.iter().any(|e| matches!(
                (&e.channel, &e.kind),
                (
                    EventChannel::Combat,
                    GameEventKind::CombatEnded {
                        outcome: CombatOutcome::Victory,
                        ..
                    }
                )
            )),
            "emits CombatEnded{{Victory}}"
        );
        assert!(
            !room_has(&response, "Stray"),
            "the defeated enemy is removed"
        );
        assert!(
            room_has(&response, "Elder"),
            "a bystander survives the removal"
        );
        assert_eq!(
            response.snapshot.player.hp, 20,
            "no damage taken (attack 0)"
        );
    }

    // C6/REQ-004 defeat (player HP lands EXACTLY on 0 → kills the player-side
    // `<= 0` → `>`) → ticket #26 semantics: reset to the start room at full
    // HP, combat cleared; at zero XP there is no penalty and no penalty
    // clause (the deliberate #26 rewrite of the #22 revive-in-place pin).
    #[test]
    fn defeat_at_zero_hp_resets_to_start_at_full_hp() {
        // health 99 survives the 4-damage strike; attack 20 drops the player 20→0.
        let mut engine = combat_engine(99, 20);
        let response = engine.handle_command(cmd("attack stray"));
        assert!(response.accepted);
        assert!(
            response.snapshot.combat.is_none(),
            "combat state is cleared on defeat"
        );
        assert!(
            response.events.iter().any(|e| matches!(
                (&e.channel, &e.kind),
                (
                    EventChannel::Combat,
                    GameEventKind::CombatEnded {
                        outcome: CombatOutcome::Defeat,
                        text,
                    }
                ) if text == "Stray has bested you. You wake at field, battered but whole."
            )),
            "the zero-XP defeat summary has no penalty clause: {:?}",
            response.events
        );
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::RoomEntered { room_id, .. } if room_id == "field"
            )),
            "the wake-up arrival narrates the start room"
        );
        assert_eq!(response.snapshot.player.hp, 20, "HP restored to max");
        assert_eq!(
            response.snapshot.current_room_id, "field",
            "the player wakes at the world start room"
        );
        assert_eq!(response.snapshot.player.xp, 0, "no penalty at zero XP");
    }

    // C7/REQ-005: `attack` in a non-combat-enabled room is refused with no state
    // change.
    #[test]
    fn attack_refused_when_room_not_combat_enabled() {
        let mut engine = combat_engine(10, 3);
        assert!(engine.handle_command(cmd("east")).accepted, "move to haven");
        let response = engine.handle_command(cmd("attack stray"));
        assert!(!response.accepted, "no combat in a non-combat-enabled room");
        assert!(
            log_text(&response).contains("no place for a fight"),
            "refusal explains the room is not combat-enabled: {}",
            log_text(&response)
        );
        assert!(response.snapshot.combat.is_none(), "no combat started");
        assert_eq!(response.snapshot.player.hp, 20, "no damage on a refusal");
    }

    // C8/REQ-005: `attack` in a combat-enabled room with no hostile is refused.
    #[test]
    fn attack_refused_when_no_hostile_present() {
        let mut engine = combat_engine(10, 3);
        assert!(
            engine.handle_command(cmd("south")).accepted,
            "move to the empty clearing"
        );
        let response = engine.handle_command(cmd("attack"));
        assert!(!response.accepted, "nothing to fight");
        assert!(
            log_text(&response).contains("nothing here to fight"),
            "refusal: {}",
            log_text(&response)
        );
        assert!(response.snapshot.combat.is_none());
    }

    // C9/REQ-005 + REQ-007 shape: a non-hostile actor and a fixture are not
    // attackable (the boss/NPC stays safe). Covers two resolve_named_hostile arms.
    #[test]
    fn attack_refused_on_non_hostile_actor_and_fixture() {
        let mut engine = combat_engine(10, 3);
        let on_elder = engine.handle_command(cmd("attack elder"));
        assert!(!on_elder.accepted, "a non-hostile actor is not attackable");
        assert!(
            log_text(&on_elder).contains("not something you can attack"),
            "elder refusal: {}",
            log_text(&on_elder)
        );
        assert!(on_elder.snapshot.combat.is_none());

        let on_idol = engine.handle_command(cmd("attack idol"));
        assert!(!on_idol.accepted, "a fixture is not attackable");
        assert!(
            log_text(&on_idol).contains("cannot fight"),
            "idol refusal: {}",
            log_text(&on_idol)
        );
    }

    // C10/REQ-005: an unknown name and a visible-but-out-of-reach hostile are both
    // refused — covers the remaining two resolve_named_hostile arms.
    #[test]
    fn attack_refused_on_unknown_and_too_far_targets() {
        let mut engine = combat_engine(10, 3);
        let unknown = engine.handle_command(cmd("attack dragon"));
        assert!(!unknown.accepted);
        assert!(
            log_text(&unknown).contains("nothing like 'dragon'"),
            "unknown refusal: {}",
            log_text(&unknown)
        );

        let far = engine.handle_command(cmd("attack wolf"));
        assert!(!far.accepted, "the distant wolf is out of reach");
        assert!(
            log_text(&far).contains("too far away to fight"),
            "too-far refusal: {}",
            log_text(&far)
        );
    }

    // In-combat: a follow-up `attack` (even naming a different entity) advances the
    // round against the ACTIVE enemy; the round counter increments 1 → 2.
    #[test]
    fn attack_in_combat_ignores_target_and_advances_round() {
        let mut engine = combat_engine(20, 1);
        let first = engine.handle_command(cmd("attack stray"));
        assert_eq!(active_combat(&first).round, 1, "opening round is 1");

        // Naming the other hostile must NOT switch enemies mid-fight.
        let second = engine.handle_command(cmd("attack brute"));
        let combat = active_combat(&second);
        assert_eq!(combat.round, 2, "the round counter advances to 2");
        assert_eq!(enemy_of(&combat).name, "Stray", "still fighting the Stray");
        assert_eq!(
            enemy_of(&combat).hp,
            20 - PLAYER_STRIKE_DAMAGE * 2,
            "two strikes have landed on the same enemy"
        );
    }

    // C18/REQ-008/009: the combat snapshot exposes both participants by value (kills
    // combat_snapshot's Some(Default) mutant) plus the round and the battle log.
    #[test]
    fn combat_snapshot_exposes_participants_round_and_log_by_value() {
        let mut engine = combat_engine(10, 3);
        let response = engine.handle_command(cmd("attack stray"));
        let combat = active_combat(&response);
        assert_eq!(combat.round, 1);
        assert_eq!(combat.participants.len(), 2);

        let player = player_of(&combat);
        assert_eq!(player.side, "player");
        assert_eq!(player.hp, 17);
        assert_eq!(player.max_hp, 20);

        let enemy = enemy_of(&combat);
        assert_eq!(enemy.side, "enemy");
        assert_eq!(enemy.name, "Stray");
        assert_eq!(enemy.hp, 6);
        assert_eq!(enemy.max_hp, 10);

        assert_eq!(combat.log.len(), 2, "one player line + one enemy line");
        assert!(combat.log[0].contains("You strike Stray"));
        assert!(combat.log[1].contains("Stray hits you"));
    }

    // C19/REQ-008: outside a fight the snapshot carries no combat sub-state.
    #[test]
    fn snapshot_has_no_combat_outside_a_fight() {
        let engine = combat_engine(10, 3);
        assert!(
            engine.snapshot().combat.is_none(),
            "no combat sub-state before any attack"
        );
    }

    // I5: a CombatProfile health beyond i32::MAX saturates to i32::MAX (covers the
    // `try_from(..).unwrap_or(i32::MAX)` path; attack 1 keeps the player alive).
    #[test]
    fn oversized_combat_profile_saturates_to_i32_max() {
        let mut field = room_with("field", true, BTreeMap::new());
        field.combat_enabled = true;
        field.entities = vec!["titan".to_string()];
        let mut rooms = BTreeMap::new();
        rooms.insert("field".to_string(), field);
        let mut world = world_with("field", rooms);
        let mut titan = entity("titan", EntityKind::Actor, &["combatant", "hostile"], &[]);
        titan.name = "Titan".to_string();
        titan.combat = Some(CombatProfile {
            health: u32::MAX,
            attack: 1,
            disclose_stats: false,
            xp: 0,
            coins: 0,
        });
        world.entities.insert("titan".to_string(), titan);
        let mut engine = Engine::try_new(world).expect("valid titan world");

        let response = engine.handle_command(cmd("attack titan"));
        let combat = active_combat(&response);
        let enemy = enemy_of(&combat);
        assert_eq!(enemy.max_hp, i32::MAX, "health saturates to i32::MAX");
        assert_eq!(enemy.hp, i32::MAX - PLAYER_STRIKE_DAMAGE);
    }

    // REQ-007 (core): a boss that is combatant-but-not-hostile is NOT attackable;
    // the confront/oath path is the only resolution. (Bell-Eater shape, in core.)
    #[test]
    fn a_boss_without_the_hostile_role_is_not_attackable() {
        // oath_world's "warden" is roles=["boss"] in a non-combat-enabled room.
        let mut engine = Engine::try_new(oath_world()).expect("valid oath world");
        assert!(
            engine.handle_command(cmd("east")).accepted,
            "reach the lair"
        );
        let response = engine.handle_command(cmd("attack warden"));
        assert!(!response.accepted, "the boss is not attackable via combat");
        assert!(response.snapshot.combat.is_none(), "no combat started");
        // confront still resolves the boss (REQ-007): swear first, then confront.
        let mut engine2 = Engine::try_new(oath_world()).expect("valid oath world");
        assert!(engine2.handle_command(cmd("swear")).accepted);
        assert!(engine2.handle_command(cmd("east")).accepted);
        assert!(
            engine2.handle_command(cmd("confront")).accepted,
            "confront still fulfills the oath"
        );
    }

    // C15: the `hostile` tag round-trips through the typed Role vocabulary.
    #[test]
    fn hostile_role_tag_round_trips() {
        assert_eq!(Role::from_tag("hostile"), Some(Role::Hostile));
        assert_eq!(Role::Hostile.as_str(), "hostile");
        let hostile = entity("h", EntityKind::Actor, &["hostile"], &[]);
        assert!(hostile.has_role(Role::Hostile));
        assert!(!hostile.has_role(Role::Boss));
    }

    // C14: the hostile contract requires a combat profile; a hostile without one is
    // rejected at validation, naming the entity/role/missing.
    #[test]
    fn hostile_without_combat_profile_is_rejected() {
        let mut field = room_with("field", true, BTreeMap::new());
        field.entities = vec!["ghoul".to_string()];
        let mut rooms = BTreeMap::new();
        rooms.insert("field".to_string(), field);
        let mut world = world_with("field", rooms);
        // hostile + Actor but NO combat profile → contract unmet.
        world.entities.insert(
            "ghoul".to_string(),
            entity("ghoul", EntityKind::Actor, &["combatant", "hostile"], &[]),
        );
        let err = world
            .validate()
            .expect_err("a profileless hostile is invalid");
        let msg = err.to_string();
        assert!(
            msg.contains("ghoul") && msg.contains("hostile") && msg.contains("combat profile"),
            "names entity/role/missing: {msg}"
        );

        // The same entity WITH a combat profile validates.
        let mut ok_world = world_with(
            "field",
            BTreeMap::from([("field".to_string(), {
                let mut r = room_with("field", true, BTreeMap::new());
                r.entities = vec!["ghoul".to_string()];
                r
            })]),
        );
        let mut ghoul = entity("ghoul", EntityKind::Actor, &["combatant", "hostile"], &[]);
        ghoul.combat = Some(CombatProfile {
            health: 3,
            attack: 1,
            disclose_stats: false,
            xp: 0,
            coins: 0,
        });
        ok_world.entities.insert("ghoul".to_string(), ghoul);
        assert_eq!(ok_world.validate(), Ok(()));
    }
    // C16 (CombatProfile.attack defaults to 0) and C17 (RoomDefinition.combat_enabled
    // defaults to false) are proven by real-TOML deserialization in the
    // oathstar-content tests (the Bell-Eater loads `attack: 0` from
    // `combat = { health = 12 }`, and the boss roost loads `combat_enabled = false`
    // from a flagless room) — core has no TOML/JSON dev-dependency to repeat them.

    // ============================================================
    //  ticket #23 — Nearby hostile affordances (server-authored)
    // ============================================================

    // A world for the Nearby-affordance snapshot tests. The start room "yard"
    // (0,0,0, combat_enabled) holds at distance 0: a hostile "Stray" (disclosed
    // stats), a non-hostile non-combatant "Warden", and a non-hostile combatant
    // "Sage" (hidden stats). "ridge" (x=2, combat_enabled) holds a hostile "Wolf"
    // perceived from the yard as visible-but-not-interactable (too far). "den"
    // (x=20, combat_enabled = FALSE) holds a hostile "Brute" — reachable only by
    // moving there (it is beyond the yard's awareness radius), to exercise a
    // reachable hostile in a non-combat area.
    fn nearby_world() -> WorldDefinition {
        let mut yard = room_with(
            "yard",
            true,
            BTreeMap::from([
                ("east".to_string(), "ridge".to_string()),
                ("south".to_string(), "den".to_string()),
            ]),
        );
        yard.combat_enabled = true;
        yard.entities = vec![
            "stray".to_string(),
            "warden".to_string(),
            "sage".to_string(),
        ];

        let mut ridge = room_with(
            "ridge",
            true,
            BTreeMap::from([("west".to_string(), "yard".to_string())]),
        );
        ridge.x = 2; // distance 2 from the yard → perceived as visible, not interactable.
        ridge.combat_enabled = true;
        ridge.entities = vec!["wolf".to_string()];

        let mut den = room_with(
            "den",
            true,
            BTreeMap::from([("north".to_string(), "yard".to_string())]),
        );
        den.x = 20; // far beyond awareness — only seen once the player moves in.
        den.combat_enabled = false;
        den.entities = vec!["brute".to_string()];

        let mut rooms = BTreeMap::new();
        rooms.insert("yard".to_string(), yard);
        rooms.insert("ridge".to_string(), ridge);
        rooms.insert("den".to_string(), den);
        let mut world = world_with("yard", rooms);

        let mut stray = entity("stray", EntityKind::Actor, &["combatant", "hostile"], &[]);
        stray.name = "Stray".to_string();
        stray.combat = Some(CombatProfile {
            health: 9,
            attack: 3,
            disclose_stats: true,
            xp: 0,
            coins: 0,
        });
        world.entities.insert("stray".to_string(), stray);

        let mut warden = entity("warden", EntityKind::Actor, &["talkable"], &[]);
        warden.name = "Warden".to_string();
        world.entities.insert("warden".to_string(), warden);

        let mut sage = entity("sage", EntityKind::Actor, &["combatant"], &[]);
        sage.name = "Sage".to_string();
        sage.combat = Some(CombatProfile {
            health: 7,
            attack: 2,
            disclose_stats: false,
            xp: 0,
            coins: 0,
        });
        world.entities.insert("sage".to_string(), sage);

        let mut wolf = entity("wolf", EntityKind::Actor, &["combatant", "hostile"], &[]);
        wolf.name = "Wolf".to_string();
        wolf.combat = Some(CombatProfile {
            health: 5,
            attack: 1,
            disclose_stats: false,
            xp: 0,
            coins: 0,
        });
        world.entities.insert("wolf".to_string(), wolf);

        let mut brute = entity("brute", EntityKind::Actor, &["combatant", "hostile"], &[]);
        brute.name = "Brute".to_string();
        brute.combat = Some(CombatProfile {
            health: 6,
            attack: 2,
            disclose_stats: false,
            xp: 0,
            coins: 0,
        });
        world.entities.insert("brute".to_string(), brute);

        world
    }

    fn nearby_engine() -> Engine {
        Engine::try_new(nearby_world()).expect("valid nearby world")
    }

    fn thing_named<'a>(snapshot: &'a GameSnapshot, name: &str) -> &'a NearbySnapshot {
        snapshot
            .room
            .contents
            .iter()
            .find(|thing| thing.name == name)
            .expect("a nearby thing with that name")
    }

    // N1/REQ-001 + N4/REQ-005: a reachable hostile in a combat-enabled room is
    // attackable, carries the server's `attack <name>` command, and discloses its
    // authored stats (health == max_health for a not-yet-engaged combatant).
    #[test]
    fn nearby_reachable_hostile_is_attackable_with_disclosed_stats() {
        let snapshot = nearby_engine().snapshot();
        let stray = thing_named(&snapshot, "Stray");
        assert_eq!(
            stray.threat,
            Some(NearbyThreatSnapshot {
                attackable: true,
                attack_command: Some("attack Stray".to_string()),
            }),
        );
        assert_eq!(
            stray.stats,
            Some(NearbyStatsSnapshot {
                health: Some(9),
                max_health: Some(9),
                attack: Some(3),
            }),
        );
    }

    // N2/REQ-002: a hostile that is visible but out of reach is hostile yet NOT
    // attackable (kills the `is_interactable && combat_enabled` → `||` mutant on the
    // interactable side: false && true == false, mutant false || true == true).
    #[test]
    fn nearby_hostile_out_of_reach_is_not_attackable() {
        let snapshot = nearby_engine().snapshot();
        let wolf = thing_named(&snapshot, "Wolf");
        assert!(!wolf.interactable, "the wolf is perceived at a distance");
        assert_eq!(
            wolf.threat,
            Some(NearbyThreatSnapshot {
                attackable: false,
                attack_command: None,
            }),
        );
    }

    // N2b/REQ-002: a reachable hostile in a NON-combat-enabled area is hostile yet
    // NOT attackable (kills the mutant on the combat_enabled side: true && false ==
    // false, mutant true || false == true).
    #[test]
    fn nearby_hostile_in_non_combat_area_is_not_attackable() {
        let mut engine = nearby_engine();
        assert!(
            engine.handle_command(cmd("south")).accepted,
            "move to the den"
        );
        let snapshot = engine.snapshot();
        let brute = thing_named(&snapshot, "Brute");
        assert!(brute.interactable, "the brute shares the player's cell");
        assert_eq!(
            brute.threat,
            Some(NearbyThreatSnapshot {
                attackable: false,
                attack_command: None,
            }),
        );
    }

    // N3/REQ-003: a non-hostile actor carries no threat (the UI shows no enemy state).
    #[test]
    fn nearby_non_hostile_actor_has_no_threat() {
        let snapshot = nearby_engine().snapshot();
        assert_eq!(thing_named(&snapshot, "Warden").threat, None);
    }

    // N5/REQ-005: an undisclosed combatant exposes a stats object whose values are
    // all `None` — the explicit "unknown" state, distinct from a non-combatant.
    #[test]
    fn nearby_undisclosed_combatant_stats_are_unknown() {
        let snapshot = nearby_engine().snapshot();
        let sage = thing_named(&snapshot, "Sage");
        assert_eq!(
            sage.stats,
            Some(NearbyStatsSnapshot {
                health: None,
                max_health: None,
                attack: None,
            }),
        );
        assert_eq!(
            sage.threat, None,
            "a non-hostile combatant is not attackable"
        );
    }

    // N6/REQ-005: a non-combatant exposes no stats object at all (distinct from the
    // hidden-stats "unknown" state above).
    #[test]
    fn nearby_non_combatant_has_no_stats() {
        let snapshot = nearby_engine().snapshot();
        assert_eq!(thing_named(&snapshot, "Warden").stats, None);
    }

    // N7/REQ-001/006: the server-authored `attack_command` is canonical — running it
    // verbatim through the engine starts combat with that hostile (server-authority
    // round-trip; the client never builds the command itself).
    #[test]
    fn nearby_attack_command_starts_combat_when_run() {
        let mut engine = nearby_engine();
        let command = thing_named(&engine.snapshot(), "Stray")
            .threat
            .as_ref()
            .expect("stray is hostile")
            .attack_command
            .clone()
            .expect("stray is attackable");
        assert_eq!(command, "attack Stray");

        let response = engine.handle_command(cmd(&command));
        assert!(response.accepted, "the server's attack command is accepted");
        let combat = response.snapshot.combat.expect("combat starts");
        assert!(
            combat
                .participants
                .iter()
                .any(|p| p.name == "Stray" && p.side == "enemy"),
            "running the server's attack_command engages the hostile",
        );
    }

    // ---- ticket #24: the real-time two-phase pulse loop ----

    // T13/REQ-006: starting combat initializes the pulse fields by value. The
    // anchor is taken off-zero (tick 1) so `tick + rate` is pinned as addition.
    #[test]
    fn start_combat_initializes_pulse_fields() {
        let mut engine = combat_engine(40, 1);
        let _ = engine.tick(); // tick 1 — keep the anchor off zero (inspect I4)
        let response = engine.handle_command(cmd("attack stray"));
        assert!(response.accepted);
        let combat = engine.state.combat.as_ref().expect("combat is active");
        assert_eq!(combat.pulse_rate, 2, "the default 2-tick combat pulse");
        assert_eq!(combat.next_pulse_at, 3, "anchored at start tick 1 + 2");
        assert_eq!(combat.queued_action, None, "nothing queued at start");
    }

    // T1/T2/T11/T12 (REQ-001/004/006/008) — the cadence keystone. Off-zero anchor
    // (inspect I4): combat starts at tick 1, so pulses are due at ticks 3 and 5
    // and the re-anchor `3 + 2 = 5` differs from the `3 * 2 = 6` mutant (a
    // zero-anchored fixture cannot tell them apart). Quiet ticks, the pulse
    // burst, and a manual mid-cadence round are all asserted by value.
    #[test]
    fn pulses_fire_on_schedule_and_manual_attack_keeps_cadence() {
        let mut engine = combat_engine(40, 1);
        let _ = engine.tick(); // tick 1
        engine.handle_command(cmd("attack stray")); // round 1; next pulse at tick 3

        let t2 = engine.tick();
        assert_eq!(t2.len(), 1, "tick 2 is quiet — only the Tick event: {t2:?}");

        let t3 = engine.tick();
        assert_eq!(
            t3.len(),
            4,
            "tick 3: Tick + CombatPulse + the exchange: {t3:?}"
        );
        assert!(
            matches!(
                (&t3[1].channel, &t3[1].kind),
                (
                    EventChannel::Combat,
                    GameEventKind::CombatPulse { round: 2 }
                )
            ),
            "the pulse marker leads the burst with the cycle it resolves"
        );
        let combat = engine.snapshot().combat.expect("combat continues");
        assert_eq!(
            combat.round, 2,
            "the pulse resolved the round its marker named"
        );
        assert_eq!(enemy_of(&combat).hp, 40 - 2 * PLAYER_STRIKE_DAMAGE);
        assert_eq!(player_of(&combat).hp, 18, "two enemy returns at attack 1");

        // A manual round between pulses must not move the schedule (REQ-004).
        let response = engine.handle_command(cmd("attack"));
        assert_eq!(
            active_combat(&response).round,
            3,
            "manual attack still advances"
        );

        let t4 = engine.tick();
        assert_eq!(
            t4.len(),
            1,
            "tick 4 is quiet — the manual round did not re-anchor the pulse: {t4:?}"
        );

        let t5 = engine.tick();
        assert!(
            t5.iter()
                .any(|e| matches!(e.kind, GameEventKind::CombatPulse { round: 4 })),
            "tick 5 pulses on the re-anchored schedule (3 + 2): {t5:?}"
        );
        let combat = engine.snapshot().combat.expect("combat continues");
        assert_eq!(enemy_of(&combat).hp, 40 - 4 * PLAYER_STRIKE_DAMAGE);
        assert_eq!(player_of(&combat).hp, 16);
    }

    // T5/T15 (REQ-004): `flee` queues the between-pulse action with its exact
    // confirmation line, surfaces it in the snapshot and the battle log, and
    // leaves the pulse schedule untouched — the queue IS the boundary.
    #[test]
    fn flee_queues_between_pulses_without_moving_the_schedule() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray")); // tick 0 → next pulse at 2
        let response = engine.handle_command(cmd("flee"));
        assert!(response.accepted);
        assert!(
            matches!(
                (&response.events[0].channel, &response.events[0].kind),
                (
                    EventChannel::Combat,
                    GameEventKind::LogMessage { component: OutputComponent::CombatMessage, text }
                ) if text == "You watch for an opening to flee."
            ),
            "the queue confirmation is a combat line: {:?}",
            response.events
        );
        let combat = active_combat(&response);
        assert_eq!(combat.queued_action.as_deref(), Some("flee"));
        assert_eq!(
            combat.log.last().map(String::as_str),
            Some("You watch for an opening to flee."),
            "the battle log mirrors the confirmation"
        );
        assert_eq!(
            engine.state.combat.as_ref().expect("active").next_pulse_at,
            2,
            "flee never re-anchors the pulse"
        );
        let t1 = engine.tick();
        assert_eq!(t1.len(), 1, "the queued flee waits for the pulse boundary");
    }

    // T6 (REQ-004): flee refuses cleanly outside combat — nothing mutated.
    #[test]
    fn flee_refuses_cleanly_outside_combat() {
        let mut engine = combat_engine(10, 3);
        let response = engine.handle_command(cmd("flee"));
        assert!(!response.accepted, "nothing to flee from");
        assert!(
            matches!(
                &response.events[0].kind,
                GameEventKind::LogMessage { component: OutputComponent::SystemMessage, text }
                    if text == "There is nothing to flee from."
            ),
            "refusal line by value: {:?}",
            response.events
        );
        assert!(response.snapshot.combat.is_none());
    }

    // Re-queueing is a no-op with its own line; the action stays queued once.
    #[test]
    fn flee_requeue_is_a_noop_with_its_own_line() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.handle_command(cmd("flee"));
        let again = engine.handle_command(cmd("flee"));
        assert!(again.accepted);
        assert!(
            matches!(
                &again.events[0].kind,
                GameEventKind::LogMessage { component: OutputComponent::CombatMessage, text }
                    if text == "You are already watching for an opening to flee."
            ),
            "the re-queue line by value: {:?}",
            again.events
        );
        assert_eq!(active_combat(&again).queued_action.as_deref(), Some("flee"));
    }

    // T3 (REQ-002/004/005): the queued flee resolves at the next pulse's skill
    // window — Phase 1 exchanges first, then CombatEnded{Fled}: the enemy
    // survives in place, the player keeps post-exchange HP (no revive), state
    // clears, and pulsing stops.
    #[test]
    fn queued_flee_resolves_at_the_pulse_boundary() {
        let mut engine = combat_engine(10, 3);
        engine.handle_command(cmd("attack stray")); // round 1: enemy 6, player 17
        engine.handle_command(cmd("flee"));
        let t1 = engine.tick();
        assert_eq!(t1.len(), 1);
        let t2 = engine.tick();
        assert_eq!(
            t2.len(),
            5,
            "Tick + CombatPulse + exchange pair + CombatEnded: {t2:?}"
        );
        assert!(matches!(
            t2[1].kind,
            GameEventKind::CombatPulse { round: 2 }
        ));
        assert!(
            matches!(
                &t2[4].kind,
                GameEventKind::CombatEnded { outcome: CombatOutcome::Fled, text }
                    if text == "You break away from Stray and escape."
            ),
            "the fled summary by value: {t2:?}"
        );
        let snapshot = engine.snapshot();
        assert!(snapshot.combat.is_none(), "fled clears combat state");
        assert_eq!(
            snapshot.player.hp, 14,
            "post-exchange HP is kept — no revive on flee"
        );
        assert!(
            snapshot.room.contents.iter().any(|t| t.name == "Stray"),
            "the enemy survives in place"
        );
        let t3 = engine.tick();
        assert_eq!(t3.len(), 1, "a fled encounter stops pulsing");
    }

    // T4 (REQ-002): nothing queued → the skill window skips cleanly and the
    // cycle continues.
    #[test]
    fn pulse_skill_window_skips_cleanly_when_nothing_queued() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        let _ = engine.tick();
        let t2 = engine.tick();
        assert_eq!(
            t2.len(),
            4,
            "Tick + CombatPulse + the Phase-1 exchange only: {t2:?}"
        );
        assert!(
            !t2.iter()
                .any(|e| matches!(e.kind, GameEventKind::CombatEnded { .. })),
            "no CombatEnded on a queue-less pulse"
        );
        let combat = engine.snapshot().combat.expect("the encounter continues");
        assert_eq!(combat.round, 2);
        assert_eq!(combat.queued_action, None, "the skip leaves nothing queued");
    }

    // T7 (REQ-005): a pulse driving the enemy to exactly zero ends in Victory,
    // removes the enemy (the bystanders survive), and stops pulsing.
    #[test]
    fn pulse_victory_at_exact_zero_stops_pulsing_and_removes_enemy() {
        let mut engine = combat_engine(8, 1);
        engine.handle_command(cmd("attack stray")); // round 1: enemy 4, player 19
        let _ = engine.tick();
        let t2 = engine.tick();
        assert_eq!(
            t2.len(),
            4,
            "Tick + CombatPulse + killing strike + CombatEnded (no return): {t2:?}"
        );
        assert!(matches!(
            &t2[3].kind,
            GameEventKind::CombatEnded {
                outcome: CombatOutcome::Victory,
                ..
            }
        ));
        let snapshot = engine.snapshot();
        assert!(snapshot.combat.is_none());
        assert_eq!(snapshot.player.hp, 19, "the dead enemy never strikes back");
        assert!(
            !snapshot.room.contents.iter().any(|t| t.name == "Stray"),
            "the defeated enemy is removed"
        );
        assert!(
            snapshot.room.contents.iter().any(|t| t.name == "Brute"),
            "bystanders survive the removal"
        );
        let t3 = engine.tick();
        assert_eq!(t3.len(), 1, "a won encounter stops pulsing");
    }

    // T8 (REQ-005): a pulse driving the player to exactly zero ends in Defeat
    // with the ticket #26 reset — start room at full HP, an XP penalty when
    // the player has XP, the wake-up arrival events — and stops pulsing.
    // (The deliberate #26 rewrite of the #24 revive-in-place pin.) The
    // direct-xp fixture doubles as ticket #30's lazy-convergence demo: 90
    // banked XP at level 1 converges to level 4 mid-defeat (LevelUps ride
    // the burst) and "full HP" means the NEW maximum.
    #[test]
    fn pulse_defeat_at_exact_zero_resets_and_stops_pulsing() {
        let mut engine = combat_engine(40, 10);
        engine.state.player.xp = 100; // earned XP to expose the penalty
        engine.handle_command(cmd("attack stray")); // round 1: enemy 36, player 10
        let _ = engine.tick();
        let t2 = engine.tick();
        assert!(
            t2.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded { outcome: CombatOutcome::Defeat, text }
                    if text
                        == "Stray has bested you. You wake at field, battered but whole. You lose 10 XP."
            )),
            "the defeat summary names the wake room and the exact penalty: {t2:?}"
        );
        assert!(
            t2.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::RoomEntered { room_id, .. } if room_id == "field"
            )),
            "the wake-up arrival follows the summary"
        );
        assert_eq!(
            t2.iter()
                .filter(|e| matches!(e.kind, GameEventKind::LevelUp { .. }))
                .count(),
            3,
            "90 banked XP converges level 1 → 4 in the defeat burst (ticket #30): {t2:?}"
        );
        let snapshot = engine.snapshot();
        assert!(snapshot.combat.is_none());
        assert_eq!(snapshot.player.level, 4, "lazy convergence lands level 4");
        assert_eq!(snapshot.player.max_hp, 35, "three level-ups grow 20 → 35");
        assert_eq!(snapshot.player.hp, 35, "HP restored to the NEW max");
        assert_eq!(snapshot.player.xp, 90, "lose max(1, 100/10) = 10 XP");
        assert_eq!(snapshot.current_room_id, "field");
        let t3 = engine.tick();
        assert_eq!(t3.len(), 1, "a lost encounter stops pulsing");
    }

    // T9 (REQ-005/008): a fled enemy re-engages from its authored health while
    // the player's HP carries between encounters.
    #[test]
    fn fled_enemy_reengages_at_authored_health() {
        let mut engine = combat_engine(10, 3);
        engine.handle_command(cmd("attack stray")); // enemy 6, player 17
        engine.handle_command(cmd("flee"));
        let _ = engine.tick();
        let _ = engine.tick(); // fled at the boundary; player 14
        let response = engine.handle_command(cmd("attack stray"));
        assert!(response.accepted, "a fled enemy can be fought again");
        let combat = active_combat(&response);
        assert_eq!(enemy_of(&combat).max_hp, 10);
        assert_eq!(
            enemy_of(&combat).hp,
            6,
            "the fresh encounter restarts from authored health (10 - one strike)"
        );
        assert_eq!(
            player_of(&combat).hp,
            11,
            "player HP carries between encounters (14 - one return)"
        );
    }

    // F6/inspect I5 (REQ-002/005): Phase 1 outranks the queued flee — a killing
    // exchange ends the fight before the skill window, and the unfound opening
    // is dropped with the cleared state.
    #[test]
    fn pulse_victory_preempts_a_queued_flee() {
        let mut engine = combat_engine(8, 1);
        engine.handle_command(cmd("attack stray")); // enemy 4, player 19
        engine.handle_command(cmd("flee"));
        let _ = engine.tick();
        let t2 = engine.tick();
        let ended: Vec<_> = t2
            .iter()
            .filter(|e| matches!(e.kind, GameEventKind::CombatEnded { .. }))
            .collect();
        assert_eq!(ended.len(), 1, "exactly one resolution: {t2:?}");
        assert!(
            matches!(
                &ended[0].kind,
                GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Victory,
                    ..
                }
            ),
            "death wins the pulse — Victory, not Fled"
        );
        let snapshot = engine.snapshot();
        assert!(snapshot.combat.is_none());
        assert!(!snapshot.room.contents.iter().any(|t| t.name == "Stray"));
    }

    // Inspect I1 (REQ-005): leaving the encounter room breaks the fight off as
    // Fled — under real-time pulses a lingering encounter would keep striking
    // the player from rooms away.
    #[test]
    fn moving_out_of_the_encounter_room_disengages_as_fled() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray")); // player 19
        let response = engine.handle_command(cmd("east")); // into haven
        assert!(
            matches!(
                &response.events[0].kind,
                GameEventKind::CombatEnded { outcome: CombatOutcome::Fled, text }
                    if text == "You break away from Stray and escape."
            ),
            "the break-away leads the move events: {:?}",
            response.events
        );
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::RoomEntered { room_id, .. } if room_id == "haven"
            )),
            "the move itself still happens"
        );
        assert!(response.snapshot.combat.is_none());
        assert_eq!(response.snapshot.player.hp, 19, "HP is kept on disengage");
        let t1 = engine.tick();
        assert_eq!(t1.len(), 1, "no pulses follow the player out");
        let back = engine.handle_command(cmd("west"));
        assert!(room_has(&back, "Stray"), "the enemy survives in its room");
    }

    // Inspect I1 (REQ-004): a refused move neither disengages nor disturbs the
    // cadence — the next pulse still lands on schedule.
    #[test]
    fn refused_move_keeps_the_encounter_and_cadence() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray")); // next pulse at tick 2
        let response = engine.handle_command(cmd("north")); // field has no north exit
        assert!(
            !response
                .events
                .iter()
                .any(|e| matches!(e.kind, GameEventKind::CombatEnded { .. })),
            "a blocked move does not end combat"
        );
        assert!(response.snapshot.combat.is_some());
        let _ = engine.tick();
        let t2 = engine.tick();
        assert!(
            t2.iter()
                .any(|e| matches!(e.kind, GameEventKind::CombatPulse { round: 2 })),
            "the cadence is intact after the refused move: {t2:?}"
        );
    }

    // ---- ticket #25: direct battle verbs ----

    /// The single Combat/CombatMessage line of a command response, by value.
    fn combat_line(response: &CommandResponse) -> String {
        match &response.events[0].kind {
            GameEventKind::LogMessage {
                component: OutputComponent::CombatMessage,
                text,
            } => text.clone(),
            other => panic!("expected a combat line, got {other:?}"),
        }
    }

    /// Count `CombatMessage` lines containing `needle` across an event burst.
    fn count_lines(events: &[GameEvent], needle: &str) -> usize {
        events
            .iter()
            .filter(|e| {
                matches!(
                    &e.kind,
                    GameEventKind::LogMessage { component: OutputComponent::CombatMessage, text }
                        if text.contains(needle)
                )
            })
            .count()
    }

    // V1 (REQ-001): `guard` queues with its exact line, surfaces on the wire,
    // mirrors to the battle log, and never moves the pulse schedule.
    #[test]
    fn guard_queues_between_pulses_without_moving_the_schedule() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray")); // tick 0 → next pulse at 2
        let response = engine.handle_command(cmd("guard"));
        assert!(response.accepted);
        assert_eq!(
            combat_line(&response),
            "You ready your guard for the next blow."
        );
        let combat = active_combat(&response);
        assert_eq!(combat.queued_action.as_deref(), Some("guard"));
        assert_eq!(
            combat.log.last().map(String::as_str),
            Some("You ready your guard for the next blow.")
        );
        assert_eq!(
            engine.state.combat.as_ref().expect("active").next_pulse_at,
            2,
            "queueing a verb never re-anchors the pulse"
        );
        let t1 = engine.tick();
        assert_eq!(t1.len(), 1, "the queued verb waits for the boundary");
    }

    // V2 (REQ-001): `power strike` queues with its exact line and wire value.
    #[test]
    fn power_strike_queues_with_its_wire_value() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        let response = engine.handle_command(cmd("power strike"));
        assert!(response.accepted);
        assert_eq!(combat_line(&response), "You wind up a power strike.");
        assert_eq!(
            active_combat(&response).queued_action.as_deref(),
            Some("power_strike")
        );
    }

    // V8 (REQ-004): outside combat both verbs refuse with their exact lines and
    // mutate nothing.
    #[test]
    fn battle_verbs_refuse_cleanly_outside_combat() {
        let mut engine = combat_engine(10, 3);
        for (input, refusal) in [
            ("guard", "There is nothing to guard against."),
            ("power strike", "There is nothing to strike at."),
        ] {
            let response = engine.handle_command(cmd(input));
            assert!(!response.accepted, "{input} refused outside combat");
            assert!(
                matches!(
                    &response.events[0].kind,
                    GameEventKind::LogMessage { component: OutputComponent::SystemMessage, text }
                        if text == refusal
                ),
                "exact refusal for {input}: {:?}",
                response.events
            );
            assert!(response.snapshot.combat.is_none());
            assert_eq!(response.snapshot.player.hp, 20, "no state mutation");
        }
    }

    // V9 + inspect I1 (REQ-005): same-action re-queue is a no-op with the exact
    // already-line — the exact string is the only killer for the match-guard
    // mutants, so assert it for BOTH new actions.
    #[test]
    fn same_action_requeue_is_a_noop_with_exact_lines() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.handle_command(cmd("guard"));
        let again = engine.handle_command(cmd("guard"));
        assert!(again.accepted);
        assert_eq!(combat_line(&again), "You are already set to guard.");
        assert_eq!(
            active_combat(&again).queued_action.as_deref(),
            Some("guard")
        );

        engine.handle_command(cmd("power strike")); // replace, then re-queue same
        let again = engine.handle_command(cmd("power strike"));
        assert_eq!(
            combat_line(&again),
            "You are already winding up a power strike."
        );
        assert_eq!(
            active_combat(&again).queued_action.as_deref(),
            Some("power_strike")
        );
    }

    // V10 (REQ-005): queueing a different action replaces with the change-tack
    // line — and crucially the replaced guard never arms its charge.
    #[test]
    fn different_action_replaces_with_change_tack() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray")); // round 1: enemy 36, player 19
        engine.handle_command(cmd("guard"));
        let switched = engine.handle_command(cmd("power strike"));
        assert!(switched.accepted);
        assert_eq!(
            combat_line(&switched),
            "You change tack. You wind up a power strike."
        );
        assert_eq!(
            active_combat(&switched).queued_action.as_deref(),
            Some("power_strike")
        );
        let _ = engine.tick();
        let t2 = engine.tick();
        assert_eq!(
            count_lines(&t2, "You raise your guard."),
            0,
            "guard never resolved"
        );
        assert_eq!(
            count_lines(&t2, "power strike slams"),
            1,
            "the replacement resolved"
        );
        assert!(
            !engine.state.combat.as_ref().expect("active").guard_charge,
            "the replaced guard never armed its charge"
        );
    }

    // V11 (REQ-005/007): the replace rule is uniform across flee in both
    // directions; flee's own #24 idempotent re-queue is untouched elsewhere.
    #[test]
    fn replace_rule_covers_flee_in_both_directions() {
        // flee → guard: the encounter does NOT end at the pulse.
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.handle_command(cmd("flee"));
        let switched = engine.handle_command(cmd("guard"));
        assert_eq!(
            combat_line(&switched),
            "You change tack. You ready your guard for the next blow."
        );
        let _ = engine.tick();
        let t2 = engine.tick();
        assert!(
            !t2.iter()
                .any(|e| matches!(e.kind, GameEventKind::CombatEnded { .. })),
            "the abandoned flee never resolves: {t2:?}"
        );
        assert!(engine.snapshot().combat.is_some());

        // guard → flee: the encounter ends fled at the pulse.
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.handle_command(cmd("guard"));
        let switched = engine.handle_command(cmd("flee"));
        assert_eq!(
            combat_line(&switched),
            "You change tack. You watch for an opening to flee."
        );
        let _ = engine.tick();
        let t2 = engine.tick();
        assert!(
            t2.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Fled,
                    ..
                }
            )),
            "the replacement flee resolves: {t2:?}"
        );
        assert!(
            !engine.state.combat.as_ref().is_some_and(|c| c.guard_charge),
            "no charge from the abandoned guard"
        );
    }

    // V13 (REQ-001/007): a manual attack round neither consumes nor disturbs
    // the queued verb — the window belongs to the pulse.
    #[test]
    fn manual_attack_leaves_the_queued_verb_alone() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.handle_command(cmd("power strike"));
        let manual = engine.handle_command(cmd("attack"));
        assert_eq!(active_combat(&manual).round, 2, "the manual round resolved");
        assert_eq!(
            active_combat(&manual).queued_action.as_deref(),
            Some("power_strike"),
            "the queue survives manual rounds"
        );
    }

    // V3 (REQ-002): the guard cycle — pulse 1's return lands (no charge yet),
    // Phase 2 arms; pulse 2's return is turned aside with the player's HP
    // numerically unchanged while the player's own strike still landed.
    #[test]
    fn guard_protects_forward_through_the_next_return() {
        let mut engine = combat_engine(40, 3);
        engine.handle_command(cmd("attack stray")); // r1: enemy 36, player 17
        engine.handle_command(cmd("guard"));
        let _ = engine.tick();
        let t2 = engine.tick(); // P1 r2: enemy 32, player 14; P2 arms the guard
        assert_eq!(count_lines(&t2, "You raise your guard."), 1);
        let combat = engine.snapshot().combat.expect("active");
        assert_eq!(
            player_of(&combat).hp,
            14,
            "pulse-2's return landed before the arm"
        );
        assert_eq!(
            combat.queued_action, None,
            "the resolved guard cleared the queue"
        );
        assert!(engine.state.combat.as_ref().expect("active").guard_charge);

        let _ = engine.tick();
        let t4 = engine.tick(); // P1 r3: strike lands, return blocked
        assert_eq!(
            count_lines(&t4, "your guard turns the blow aside"),
            1,
            "the block narrates: {t4:?}"
        );
        let combat = engine.snapshot().combat.expect("active");
        assert_eq!(player_of(&combat).hp, 14, "the blocked return cost nothing");
        assert_eq!(
            enemy_of(&combat).hp,
            40 - 3 * PLAYER_STRIKE_DAMAGE,
            "the player still struck"
        );
        assert!(
            !engine.state.combat.as_ref().expect("active").guard_charge,
            "one-shot: consumed"
        );
    }

    // V4 (REQ-002/003): single-fire across the whole fight — exactly one arm
    // line, exactly one block, and the post-guard pulse hits normally again.
    #[test]
    fn guard_fires_exactly_once_take_not_peek() {
        let mut engine = combat_engine(40, 3);
        engine.handle_command(cmd("attack stray")); // player 17
        engine.handle_command(cmd("guard"));
        let mut all_events = Vec::new();
        for _ in 0..6 {
            all_events.extend(engine.tick()); // pulses at t2 (arm), t4 (block), t6 (normal)
        }
        assert_eq!(
            count_lines(&all_events, "You raise your guard."),
            1,
            "one arm"
        );
        assert_eq!(
            count_lines(&all_events, "your guard turns the blow aside"),
            1,
            "one block"
        );
        let combat = engine.snapshot().combat.expect("active");
        assert_eq!(
            player_of(&combat).hp,
            17 - 3 - 3,
            "returns: r2 hit (14), r3 blocked, r4 hit (11)"
        );
    }

    // V14 (REQ-002): the armed charge guards a MANUAL round's return too — the
    // next return from any source consumes it.
    #[test]
    fn guard_charge_blocks_a_manual_round_return() {
        let mut engine = combat_engine(40, 3);
        engine.handle_command(cmd("attack stray")); // player 17
        engine.handle_command(cmd("guard"));
        let _ = engine.tick();
        let _ = engine.tick(); // r2 (player 14) + arm
        let manual = engine.handle_command(cmd("attack")); // r3: return blocked
        assert_eq!(
            count_lines(&manual.events, "your guard turns the blow aside"),
            1
        );
        assert_eq!(active_combat(&manual).round, 3);
        assert_eq!(
            player_of(&active_combat(&manual)).hp,
            14,
            "blocked manual return"
        );
        let _ = engine.tick();
        let t4 = engine.tick(); // r4: return hits normally — the charge is gone
        assert_eq!(count_lines(&t4, "your guard turns the blow aside"), 0);
        assert_eq!(
            player_of(&engine.snapshot().combat.expect("active")).hp,
            11,
            "post-consumption returns land again"
        );
    }

    // V5 (REQ-002): power strike resolves in the window by value and the
    // encounter continues.
    #[test]
    fn power_strike_resolves_in_the_window_by_value() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray")); // r1: enemy 36
        engine.handle_command(cmd("power strike"));
        let _ = engine.tick();
        let t2 = engine.tick(); // P1 r2: enemy 32; P2 slam: 26
        assert_eq!(
            count_lines(&t2, "Your power strike slams into Stray for 6 (26/40)."),
            1,
            "exact slam line: {t2:?}"
        );
        let combat = engine.snapshot().combat.expect("the fight continues");
        assert_eq!(enemy_of(&combat).hp, 26);
        assert_eq!(
            combat.queued_action, None,
            "the resolved strike cleared the queue"
        );
        assert!(
            !t2.iter()
                .any(|e| matches!(e.kind, GameEventKind::CombatEnded { .. })),
            "non-lethal window strike does not end the fight"
        );
    }

    // V6 (REQ-002/007): a window kill at exactly zero ends in Victory from
    // Phase 2 — enemy removed, pulses stop.
    #[test]
    fn power_strike_victory_at_exact_zero_from_the_window() {
        let mut engine = combat_engine(14, 1);
        engine.handle_command(cmd("attack stray")); // r1: enemy 10
        engine.handle_command(cmd("power strike"));
        let _ = engine.tick();
        let t2 = engine.tick(); // P1 r2: enemy 6; P2 slam: exactly 0
        assert_eq!(
            count_lines(&t2, "for 6 (0/14)."),
            1,
            "exact zero on the line: {t2:?}"
        );
        assert!(t2.iter().any(|e| matches!(
            &e.kind,
            GameEventKind::CombatEnded {
                outcome: CombatOutcome::Victory,
                ..
            }
        )));
        let snapshot = engine.snapshot();
        assert!(snapshot.combat.is_none());
        assert!(!snapshot.room.contents.iter().any(|t| t.name == "Stray"));
        let t3 = engine.tick();
        assert_eq!(t3.len(), 1, "a window victory stops pulsing");
    }

    // Inspect I3 (REQ-002): an overkill window strike clamps the display at
    // zero — never a negative HP.
    #[test]
    fn power_strike_overkill_clamps_at_zero() {
        let mut engine = combat_engine(12, 1);
        engine.handle_command(cmd("attack stray")); // r1: enemy 8
        engine.handle_command(cmd("power strike"));
        let _ = engine.tick();
        let t2 = engine.tick(); // P1 r2: enemy 4; P2 slam: 4 − 6 → clamped 0
        assert_eq!(
            count_lines(&t2, "for 6 (0/12)."),
            1,
            "clamped at zero: {t2:?}"
        );
        assert!(t2.iter().any(|e| matches!(
            &e.kind,
            GameEventKind::CombatEnded {
                outcome: CombatOutcome::Victory,
                ..
            }
        )));
    }

    // V12 (REQ-002/007): Phase 1 outranks the queued verb — a P1 kill drops it
    // silently with the cleared state.
    #[test]
    fn pulse_victory_preempts_a_queued_verb() {
        let mut engine = combat_engine(8, 1);
        engine.handle_command(cmd("attack stray")); // enemy 4
        engine.handle_command(cmd("guard"));
        let _ = engine.tick();
        let t2 = engine.tick(); // P1 kills at exactly 0
        assert!(t2.iter().any(|e| matches!(
            &e.kind,
            GameEventKind::CombatEnded {
                outcome: CombatOutcome::Victory,
                ..
            }
        )));
        assert_eq!(
            count_lines(&t2, "You raise your guard."),
            0,
            "the verb never fires"
        );
        assert!(engine.snapshot().combat.is_none());
    }

    // V16 (REQ-007): a fled encounter leaves no verb residue — re-engaging
    // starts with a clean queue and no charge.
    #[test]
    fn fled_encounter_leaves_no_verb_residue() {
        let mut engine = combat_engine(40, 3);
        engine.handle_command(cmd("attack stray")); // player 17
        engine.handle_command(cmd("guard"));
        let _ = engine.tick();
        let _ = engine.tick(); // r2 (player 14) + arm
        engine.handle_command(cmd("flee"));
        let _ = engine.tick();
        let t4 = engine.tick(); // r3: return blocked (charge), then P2 → Fled
        assert!(t4.iter().any(|e| matches!(
            &e.kind,
            GameEventKind::CombatEnded {
                outcome: CombatOutcome::Fled,
                ..
            }
        )));
        let response = engine.handle_command(cmd("attack stray"));
        let fresh = engine.state.combat.as_ref().expect("re-engaged");
        assert!(!fresh.guard_charge, "no charge leaks across encounters");
        assert_eq!(
            fresh.queued_action, None,
            "no queue leaks across encounters"
        );
        assert_eq!(active_combat(&response).queued_action, None);
    }

    // Inspect I2 (REQ-007): help teaches the new verbs.
    #[test]
    fn help_lists_the_battle_verbs() {
        let mut engine = combat_engine(10, 3);
        let response = engine.handle_command(cmd("help"));
        let text = match &response.events[0].kind {
            GameEventKind::LogMessage { text, .. } => text.clone(),
            other => panic!("help is a log line, got {other:?}"),
        };
        assert!(text.contains("guard"), "help lists guard: {text}");
        assert!(
            text.contains("power strike"),
            "help lists power strike: {text}"
        );
    }

    // ---- ticket #26: combat rewards + defeat consequences ----

    /// A combat engine whose stray carries authored spoils (ticket #26): an
    /// XP reward and, when `fang` is set, a droppable item registered in the
    /// world's item registry.
    fn spoils_engine(health: u32, attack: u32, xp: u64, fang: bool) -> Engine {
        let mut world = combat_world(health, attack);
        if fang {
            world.items.insert("fang".to_string(), item("fang"));
            world
                .entities
                .get_mut("stray")
                .expect("stray exists")
                .inventory = vec!["fang".to_string()];
        }
        world
            .entities
            .get_mut("stray")
            .expect("stray exists")
            .combat
            .as_mut()
            .expect("stray has a combat profile")
            .xp = xp;
        Engine::try_new(world).expect("valid spoils world")
    }

    // X1 (REQ-001): a pulse victory awards the authored XP exactly — by value
    // (the sole killer for the award's add mutants) — and the summary names
    // the gain.
    #[test]
    fn pulse_victory_awards_authored_xp() {
        let mut engine = spoils_engine(8, 1, 7, false);
        engine.handle_command(cmd("attack stray")); // round 1: enemy 4, player 19
        let _ = engine.tick();
        let t2 = engine.tick(); // pulse round 2 kills at exactly 0
        assert!(
            t2.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded { outcome: CombatOutcome::Victory, text }
                    if text == "You have defeated Stray. Victory! You gain 7 XP."
            )),
            "the victory summary names the gain: {t2:?}"
        );
        assert_eq!(engine.snapshot().player.xp, 7, "the award lands by value");
    }

    // X2 (REQ-001): the #25 Phase-2 victory path awards through the same
    // end_combat funnel — exactly once, never doubled.
    #[test]
    fn window_victory_awards_xp_exactly_once() {
        let mut engine = spoils_engine(14, 1, 7, false);
        engine.handle_command(cmd("attack stray")); // round 1: enemy 10
        engine.handle_command(cmd("power strike"));
        let _ = engine.tick();
        let t2 = engine.tick(); // P1: enemy 6; P2 slam lands exactly 0 → Victory
        assert!(
            t2.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded { outcome: CombatOutcome::Victory, text }
                    if text == "You have defeated Stray. Victory! You gain 7 XP."
            )),
            "{t2:?}"
        );
        assert_eq!(engine.snapshot().player.xp, 7, "awarded once, not doubled");
    }

    // X3 (REQ-002): an unrewarded victory keeps the pre-#26 summary
    // byte-identical and awards nothing — the in-core killer for the
    // `xp > 0` text-guard mutants.
    #[test]
    fn zero_xp_victory_keeps_the_original_summary() {
        let mut engine = combat_engine(8, 1);
        engine.handle_command(cmd("attack stray"));
        let _ = engine.tick();
        let t2 = engine.tick();
        assert!(
            t2.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded { outcome: CombatOutcome::Victory, text }
                    if text == "You have defeated Stray. Victory!"
            )),
            "no invented reward, no gain clause: {t2:?}"
        );
        assert_eq!(engine.snapshot().player.xp, 0);
    }

    // X4 (REQ-003): a victory drops the authored inventory into the room —
    // exact line, visible in contents, takeable into the pack, inventory
    // cleared for the session.
    #[test]
    fn victory_drops_inventory_into_the_room() {
        let mut engine = spoils_engine(8, 1, 0, true);
        engine.handle_command(cmd("attack stray"));
        let _ = engine.tick();
        let t2 = engine.tick();
        assert_eq!(count_lines(&t2, "The Stray drops fang."), 1, "{t2:?}");
        let snapshot = engine.snapshot();
        assert!(
            snapshot.room.contents.iter().any(|t| t.id == "fang"),
            "the drop is visible in the room contents"
        );
        assert!(
            engine
                .world
                .entities
                .get("stray")
                .expect("the registry survives removal")
                .inventory
                .is_empty(),
            "the inventory cleared on drop"
        );
        let take = engine.handle_command(cmd("take fang"));
        assert!(take.accepted, "the drop is takeable");
        assert!(
            take.snapshot.pack.iter().any(|p| p.id == "fang"),
            "the fang lands in the pack"
        );
    }

    // X5 (REQ-004): a victory over an inventory-less hostile drops nothing —
    // no lines, no phantom items.
    #[test]
    fn victory_without_inventory_drops_nothing() {
        let mut engine = combat_engine(8, 1);
        engine.handle_command(cmd("attack stray"));
        let _ = engine.tick();
        let t2 = engine.tick();
        assert_eq!(count_lines(&t2, " drops "), 0, "{t2:?}");
        assert!(
            !engine
                .snapshot()
                .room
                .contents
                .iter()
                .any(|t| t.kind == "item"),
            "no phantom drops"
        );
    }

    // X6 (REQ-003): fleeing leaves the spoils intact; the eventual victory
    // drops them exactly once — one line, one placement.
    #[test]
    fn drops_happen_exactly_once_after_a_flee() {
        let mut engine = spoils_engine(40, 1, 0, true);
        engine.handle_command(cmd("attack stray"));
        engine.handle_command(cmd("flee"));
        let _ = engine.tick();
        let _ = engine.tick(); // fled at the boundary; nothing dropped
        assert!(
            engine
                .snapshot()
                .room
                .contents
                .iter()
                .all(|t| t.id != "fang"),
            "fleeing drops nothing"
        );
        engine.handle_command(cmd("attack stray")); // re-engage at authored 40 hp
        let mut drop_lines = 0;
        for _ in 0..30 {
            let burst = engine.tick();
            drop_lines += count_lines(&burst, "The Stray drops fang.");
            if engine.snapshot().combat.is_none() {
                break;
            }
        }
        assert!(engine.snapshot().combat.is_none(), "the refight resolved");
        assert_eq!(drop_lines, 1, "the drop narrates exactly once");
        let fangs = engine
            .snapshot()
            .room
            .contents
            .iter()
            .filter(|t| t.id == "fang")
            .count();
        assert_eq!(fangs, 1, "exactly one fang placement");
    }

    /// The relocation fixture (inspect I1): the stray fights in the clearing,
    /// one move south of the start room, so the defeat reset is observable.
    fn relocated_stray_engine(health: u32, attack: u32) -> Engine {
        let mut world = combat_world(health, attack);
        world
            .rooms
            .get_mut("field")
            .expect("field exists")
            .entities
            .retain(|id| id != "stray");
        world
            .rooms
            .get_mut("clearing")
            .expect("clearing exists")
            .entities
            .push("stray".to_string());
        Engine::try_new(world).expect("valid relocated world")
    }

    // Inspect I1 (REQ-005, BINDING): a defeat away from the start room
    // relocates the player — wake room ≠ encounter room by value, the enemy
    // stays placed where it fought, and the penalty lands exactly.
    #[test]
    fn defeat_away_from_start_relocates_to_the_start_room() {
        let mut engine = relocated_stray_engine(99, 20);
        engine.state.player.xp = 40;
        let moved = engine.handle_command(cmd("south"));
        assert_eq!(moved.snapshot.current_room_id, "clearing");
        let response = engine.handle_command(cmd("attack stray")); // 20 dmg → defeat
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded { outcome: CombatOutcome::Defeat, text }
                    if text == "Stray has bested you. You wake at field, battered but whole. You lose 4 XP."
            )),
            "{:?}",
            response.events
        );
        assert_eq!(
            response.snapshot.current_room_id, "field",
            "defeat relocates to the start room, away from the clearing"
        );
        assert_eq!(response.snapshot.player.xp, 36, "lose max(1, 40/10) = 4");
        assert_eq!(
            response.snapshot.player.level, 3,
            "the banked-XP fixture lazily converges mid-defeat (ticket #30)"
        );
        assert_eq!(
            response.snapshot.player.hp, 30,
            "HP restored to the NEW max (20 + 2x5)"
        );
        assert!(
            engine
                .world
                .rooms
                .get("clearing")
                .expect("clearing")
                .entities
                .iter()
                .any(|id| id == "stray"),
            "the enemy is left in place where it fought"
        );
    }

    // X8 (REQ-005/006): the penalty floor and the one-point edge — 5 → lose
    // 1 → 4; 1 → lose 1 → 0 (never below zero).
    #[test]
    fn defeat_penalty_floors_at_one_and_zero() {
        for (start_xp, after) in [(5_u64, 4_u64), (1, 0)] {
            let mut engine = combat_engine(99, 20);
            engine.state.player.xp = start_xp;
            let response = engine.handle_command(cmd("attack stray"));
            assert!(
                response.events.iter().any(|e| matches!(
                    &e.kind,
                    GameEventKind::CombatEnded { outcome: CombatOutcome::Defeat, text }
                        if text.ends_with("You lose 1 XP.")
                )),
                "xp {start_xp}: {:?}",
                response.events
            );
            assert_eq!(response.snapshot.player.xp, after, "from xp {start_xp}");
        }
    }

    // X11 (REQ-005/009): after the defeat reset the player can walk back and
    // re-engage the surviving enemy — the encounter genuinely restarts.
    #[test]
    fn player_can_reengage_after_a_defeat_reset() {
        let mut engine = relocated_stray_engine(99, 20);
        engine.handle_command(cmd("south"));
        engine.handle_command(cmd("attack stray")); // defeated; reset to field
        assert_eq!(engine.snapshot().current_room_id, "field");
        engine.handle_command(cmd("south"));
        let again = engine.handle_command(cmd("attack stray"));
        assert!(again.accepted, "the surviving enemy can be fought again");
        assert!(
            again.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatStarted { enemy_id, .. } if enemy_id == "stray"
            )),
            "a fresh encounter starts: {:?}",
            again.events
        );
    }

    // ---- ticket #27: scoped announcements ----

    /// A world spanning two regions, two subregions, and two z-levels so every
    /// announcement scope has a received AND a not-received position
    /// (PR-claude-fixture-distinguishable-transitions-001). Tests reposition
    /// the player directly; no exits are needed.
    fn scoped_engine() -> Engine {
        let mut rooms = BTreeMap::new();
        for (id, region, subregion, x, z) in [
            ("a", "r1", Some("s1"), 0, 0),
            ("b", "r1", Some("s1"), 3, 0),
            ("c", "r1", Some("s2"), 1, 0),
            ("e", "r1", Some("s1"), 0, 1),
            ("f", "r1", None, 0, 0),
            ("g", "r2", None, 1, 0),
        ] {
            let mut room = room_with(id, true, BTreeMap::new());
            room.region = region.to_string();
            room.subregion = subregion.map(str::to_string);
            room.x = x;
            room.z = z;
            rooms.insert(id.to_string(), room);
        }
        let mut world = world_with("a", rooms);
        for (id, region) in [("s1", "r1"), ("s2", "r1")] {
            world.subregions.insert(
                id.to_string(),
                SubregionDefinition {
                    id: id.to_string(),
                    name: id.to_string(),
                    region: region.to_string(),
                },
            );
        }
        Engine::try_new(world).expect("valid scoped world")
    }

    /// Reposition the player (every id is a validated room of the fixture).
    fn at(engine: &mut Engine, room: &str) {
        engine.state.current_room_id = room.to_string();
    }

    // N1 (REQ-001): world scope is received everywhere — the only pin for the
    // `World => true` arm.
    #[test]
    fn world_scope_is_received_everywhere() {
        let mut engine = scoped_engine();
        for room in ["a", "c", "g"] {
            at(&mut engine, room);
            assert!(
                engine.announcement_received(&AnnouncementScope::World),
                "world scope reaches {room}"
            );
        }
    }

    // N2 (REQ-002): region scope both arms, plus no subregion-id crosstalk.
    #[test]
    fn region_scope_matches_only_the_named_region() {
        let mut engine = scoped_engine();
        at(&mut engine, "a");
        assert!(engine.announcement_received(&AnnouncementScope::Region("r1".to_string())));
        at(&mut engine, "g");
        assert!(
            !engine.announcement_received(&AnnouncementScope::Region("r1".to_string())),
            "another region does not receive it"
        );
        at(&mut engine, "a");
        assert!(
            !engine.announcement_received(&AnnouncementScope::Region("s1".to_string())),
            "a subregion id never matches as a region"
        );
    }

    // N3 (REQ-002): subregion both arms, the no-subregion room, and no
    // region-id crosstalk.
    #[test]
    fn subregion_scope_matches_only_the_named_subregion() {
        let mut engine = scoped_engine();
        at(&mut engine, "a");
        assert!(engine.announcement_received(&AnnouncementScope::Subregion("s1".to_string())));
        at(&mut engine, "c");
        assert!(
            !engine.announcement_received(&AnnouncementScope::Subregion("s1".to_string())),
            "a sibling subregion does not receive it"
        );
        at(&mut engine, "f");
        assert!(
            !engine.announcement_received(&AnnouncementScope::Subregion("s1".to_string())),
            "a room with no subregion receives no subregion scope"
        );
        at(&mut engine, "a");
        assert!(
            !engine.announcement_received(&AnnouncementScope::Subregion("r1".to_string())),
            "a region id never matches as a subregion"
        );
    }

    // N4 (REQ-002): room scope both arms.
    #[test]
    fn room_scope_matches_only_the_exact_room() {
        let mut engine = scoped_engine();
        at(&mut engine, "a");
        assert!(engine.announcement_received(&AnnouncementScope::Room("a".to_string())));
        assert!(
            !engine.announcement_received(&AnnouncementScope::Room("b".to_string())),
            "another room does not receive it"
        );
    }

    // N5 (REQ-003): radius follows the awareness plane model — inclusive
    // boundary, beyond, cross-floor, cross-subregion, cross-region, unknown
    // origin, and the origin cell itself.
    #[test]
    fn radius_scope_follows_the_awareness_plane_model() {
        let mut engine = scoped_engine();
        let from_a = |radius| AnnouncementScope::Radius {
            room_id: "a".to_string(),
            radius,
        };
        at(&mut engine, "b"); // (3,0,0) on a's plane
        assert!(
            engine.announcement_received(&from_a(3)),
            "distance == radius is received (inclusive boundary)"
        );
        assert!(
            !engine.announcement_received(&from_a(2)),
            "beyond the radius is not received"
        );
        at(&mut engine, "e"); // (0,0,1): a's x,y one floor up
        assert!(
            !engine.announcement_received(&from_a(99)),
            "a different floor is not near"
        );
        at(&mut engine, "c"); // (1,0,0) in the sibling subregion
        assert!(
            !engine.announcement_received(&from_a(99)),
            "a different subregion is not near"
        );
        at(&mut engine, "g"); // r2, subregion None
        assert!(
            !engine.announcement_received(&AnnouncementScope::Radius {
                room_id: "f".to_string(),
                radius: 99,
            }),
            "a different region is not near even with matching None subregions"
        );
        at(&mut engine, "a");
        assert!(
            !engine.announcement_received(&AnnouncementScope::Radius {
                room_id: "ghost".to_string(),
                radius: 99,
            }),
            "an unknown origin room delivers nothing"
        );
        assert!(
            engine.announcement_received(&AnnouncementScope::Radius {
                room_id: "a".to_string(),
                radius: 0,
            }),
            "radius 0 reaches the origin cell itself"
        );
    }

    /// `boss_objective_world` with authored fulfillment announcements: one
    /// scoped to the recovery room (delivered in play) and one to the start
    /// room (provably not delivered at the lair). Ticket #29: announcements
    /// now ride the objective's recovery, not the confrontation.
    fn announcing_oath_engine() -> Engine {
        let mut world = boss_objective_world(4, 0, 0);
        world
            .oaths
            .get_mut("o1")
            .expect("o1 exists")
            .fulfillment_announcements = vec![
            AuthoredAnnouncement {
                scope: AnnouncementScope::Room("lair".to_string()),
                severity: AnnouncementSeverity::Alarm,
                text: "The lair shudders.".to_string(),
            },
            AuthoredAnnouncement {
                scope: AnnouncementScope::Room("town".to_string()),
                severity: AnnouncementSeverity::Notice,
                text: "Town hears nothing of this.".to_string(),
            },
        ];
        Engine::try_new(world).expect("valid announcing world")
    }

    // N9/B4 (REQ-003, order amended at inspect): recovering the objective
    // emits pickup → OathFulfilled → the in-scope announcement IN ORDER —
    // with no OathCard log twin (the typed event is the human line) — and
    // the out-of-scope announcement emits nothing at all.
    #[test]
    fn fulfillment_delivers_only_in_scope_announcements() {
        let mut engine = announcing_oath_engine();
        engine.handle_command(cmd("swear"));
        engine.handle_command(cmd("east"));
        assert!(
            engine.handle_command(cmd("confront")).accepted,
            "the opening strike fells the warden and drops the objective"
        );
        let response = engine.handle_command(cmd("take sigil"));
        assert!(response.accepted, "the dropped objective is takeable");
        let pickup_at = response
            .events
            .iter()
            .position(|e| {
                matches!(
                    &e.kind,
                    GameEventKind::LogMessage {
                        component: OutputComponent::ItemCard,
                        text,
                    } if text == "You take the sigil."
                )
            })
            .expect("the pickup line lands");
        let fulfilled_at = response
            .events
            .iter()
            .position(|e| {
                matches!(
                    (&e.channel, &e.kind),
                    (EventChannel::Oath, GameEventKind::OathFulfilled { oath_id })
                        if oath_id.as_str() == "o1"
                )
            })
            .expect("the recovery fulfills");
        let announcement_at = response
            .events
            .iter()
            .position(|e| {
                matches!(
                    (&e.channel, &e.kind),
                    (
                        EventChannel::Region,
                        GameEventKind::Announcement {
                            severity: AnnouncementSeverity::Alarm,
                            text,
                        }
                    ) if text == "The lair shudders."
                )
            })
            .expect("the in-scope announcement is delivered");
        assert!(
            pickup_at < fulfilled_at && fulfilled_at < announcement_at,
            "order is pickup → fulfilled → announcement: {:?}",
            response.events
        );
        assert!(
            !response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage {
                    component: OutputComponent::OathCard,
                    ..
                }
            )),
            "no OathCard log twin duplicates the typed render"
        );
        assert!(
            !response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::Announcement { text, .. } if text == "Town hears nothing of this."
            )),
            "the out-of-scope announcement emits nothing at all"
        );
        assert_eq!(
            response.snapshot.oath.expect("oath present").status,
            OathStatus::Fulfilled
        );
    }

    // N8 (REQ-005): identical command sequences produce identical event
    // streams — fulfillment + announcement delivery is deterministic.
    #[test]
    fn announcement_delivery_is_deterministic() {
        let run = || {
            let mut engine = announcing_oath_engine();
            engine.handle_command(cmd("swear"));
            engine.handle_command(cmd("east"));
            engine.handle_command(cmd("confront"));
            format!("{:?}", engine.handle_command(cmd("take sigil")).events)
        };
        assert_eq!(run(), run());
    }

    // N12 (REQ-006) + inspect I5/I6: every missing scope id fails construction
    // with the exact error — including the Radius case reporting the "room"
    // label — and every scope kind validates when its ids resolve.
    #[test]
    fn announcement_scopes_are_validated_at_construction() {
        let cases = [
            (
                AnnouncementScope::Region("ghost".to_string()),
                "oath 'o1' announces to missing region 'ghost'",
            ),
            (
                AnnouncementScope::Subregion("ghost".to_string()),
                "oath 'o1' announces to missing subregion 'ghost'",
            ),
            (
                AnnouncementScope::Room("ghost".to_string()),
                "oath 'o1' announces to missing room 'ghost'",
            ),
            (
                AnnouncementScope::Radius {
                    room_id: "ghost".to_string(),
                    radius: 1,
                },
                "oath 'o1' announces to missing room 'ghost'",
            ),
        ];
        for (scope, display) in cases {
            let mut world = oath_world();
            world
                .oaths
                .get_mut("o1")
                .expect("o1")
                .fulfillment_announcements = vec![AuthoredAnnouncement {
                scope: scope.clone(),
                severity: AnnouncementSeverity::Notice,
                text: "x".to_string(),
            }];
            let err = world.validate().expect_err("a missing scope id must fail");
            assert_eq!(err.to_string(), display, "{scope:?}");
        }

        let mut world = oath_world();
        world.subregions.insert(
            "district".to_string(),
            SubregionDefinition {
                id: "district".to_string(),
                name: "District".to_string(),
                region: "r".to_string(),
            },
        );
        world
            .oaths
            .get_mut("o1")
            .expect("o1")
            .fulfillment_announcements = vec![
            AuthoredAnnouncement {
                scope: AnnouncementScope::World,
                severity: AnnouncementSeverity::Notice,
                text: "w".to_string(),
            },
            AuthoredAnnouncement {
                scope: AnnouncementScope::Region("r".to_string()),
                severity: AnnouncementSeverity::Notice,
                text: "r".to_string(),
            },
            AuthoredAnnouncement {
                scope: AnnouncementScope::Subregion("district".to_string()),
                severity: AnnouncementSeverity::Warning,
                text: "s".to_string(),
            },
            AuthoredAnnouncement {
                scope: AnnouncementScope::Room("lair".to_string()),
                severity: AnnouncementSeverity::Alarm,
                text: "rm".to_string(),
            },
            AuthoredAnnouncement {
                scope: AnnouncementScope::Radius {
                    room_id: "town".to_string(),
                    radius: 2,
                },
                severity: AnnouncementSeverity::Alarm,
                text: "rad".to_string(),
            },
        ];
        assert_eq!(world.validate(), Ok(()), "all scope kinds validate");
    }

    // ---- ticket #28: save & load — the engine persistence surface ----

    /// A mid-session spoils engine: fight to victory, take the dropped fang.
    /// The session has every mutation class the payload must carry — earned
    /// xp, a packed item, a removed placement, and a cleared inventory.
    fn played_spoils_engine() -> Engine {
        let mut engine = spoils_engine(8, 1, 7, true);
        engine.handle_command(cmd("attack stray"));
        let _ = engine.tick();
        let _ = engine.tick(); // pulse round 2: victory, fang drops, xp lands
        engine.handle_command(cmd("take fang"));
        engine
    }

    fn snapshot_value(engine: &Engine) -> serde_json::Value {
        serde_json::to_value(engine.snapshot()).expect("snapshots serialize")
    }

    // SL1 (REQ-001): the save payload carries the COMPLETE mutated session —
    // world mutations included (the #26 lesson) — and never mutates the
    // running engine. By-value asserts are the fn-replace killers for
    // save_data in this package.
    #[test]
    fn save_data_captures_the_mutated_session_without_mutating_it() {
        let engine = played_spoils_engine();
        let before = snapshot_value(&engine);
        let data = engine.save_data();
        assert_eq!(
            snapshot_value(&engine),
            before,
            "saving never mutates the running session"
        );

        assert_eq!(
            data.version, SAVE_FORMAT_VERSION,
            "stamped with the format version"
        );
        assert_eq!(data.state.player.xp, 7, "earned xp rides in the payload");
        assert_eq!(
            data.state.pack,
            vec!["fang".to_string()],
            "the taken fang rides in the pack"
        );
        let field = data.world.rooms.get("field").expect("field saved");
        assert!(
            !field.entities.iter().any(|id| id == "stray"),
            "the defeated stray's placement is gone from the SAVED world"
        );
        assert!(
            !field.items.iter().any(|id| id == "fang"),
            "the taken fang's ground placement is gone from the SAVED world"
        );
        assert!(
            data.world
                .entities
                .get("stray")
                .expect("the registry entry survives placement removal")
                .inventory
                .is_empty(),
            "the dropped inventory is cleared in the SAVED world"
        );
        assert_eq!(
            data.next_event_id, engine.next_event_id,
            "the event counter rides along"
        );
    }

    // SL2 (REQ-002): restoring a save reproduces the session byte-for-byte —
    // the restored snapshot equals the at-save snapshot as serde_json::Value.
    #[test]
    fn from_save_restores_a_byte_identical_snapshot() {
        let engine = played_spoils_engine();
        let at_save = snapshot_value(&engine);
        let restored = Engine::from_save(engine.save_data()).expect("a self-produced save loads");
        assert_eq!(
            snapshot_value(&restored),
            at_save,
            "the restored snapshot is byte-identical to the at-save snapshot"
        );
    }

    // SL3 (REQ-002, mid-combat variant): a save taken mid-encounter — queued
    // action armed, guard charge banked — restores an engine whose future
    // (events AND snapshot) replays the unsaved twin's exactly, pulse for
    // pulse. Event ids and ticks match because both persist.
    #[test]
    fn mid_combat_save_resumes_the_exact_cadence_and_queued_action() {
        let mut engine = spoils_engine(30, 1, 0, false);
        engine.handle_command(cmd("attack stray")); // round 1; pulse due at +2
        engine.handle_command(cmd("guard"));
        let _ = engine.tick();
        let _ = engine.tick(); // pulse 2: guard resolves, charge armed
        engine.handle_command(cmd("power strike")); // queued for pulse 3

        let data = engine.save_data();
        let combat = data
            .state
            .combat
            .as_ref()
            .expect("mid-combat save persists the encounter");
        assert_eq!(
            combat.queued_action,
            Some(CombatAction::PowerStrike),
            "the queued action rides in the payload"
        );
        assert!(combat.guard_charge, "the banked guard charge rides too");

        let mut restored = Engine::from_save(data).expect("a mid-combat save loads");
        let twin_events: Vec<GameEvent> = (0..2).flat_map(|_| engine.tick()).collect();
        let restored_events: Vec<GameEvent> = (0..2).flat_map(|_| restored.tick()).collect();
        assert_eq!(
            serde_json::to_value(&restored_events).expect("events serialize"),
            serde_json::to_value(&twin_events).expect("events serialize"),
            "the restored future replays the unsaved twin's, event for event"
        );
        assert_eq!(
            snapshot_value(&restored),
            snapshot_value(&engine),
            "and the post-pulse snapshots agree"
        );
    }

    // SL4 (REQ-003): every refusal arm, staged BOTH ways (the fixture-
    // distinguishability rule): the version gate by value, world
    // re-validation, and each of the three state-coherence gates naming its
    // offender — with the matching valid payload loading fine.
    #[test]
    fn from_save_rejects_a_version_mismatch_by_value() {
        let engine = played_spoils_engine();
        let mut data = engine.save_data();
        data.version = SAVE_FORMAT_VERSION + 1;
        assert_eq!(
            Engine::from_save(data.clone()).err(),
            Some(LoadError::VersionMismatch {
                found: SAVE_FORMAT_VERSION + 1,
                supported: SAVE_FORMAT_VERSION,
            }),
            "an unknown version is refused loudly, naming both sides"
        );
        data.version = SAVE_FORMAT_VERSION;
        assert!(
            Engine::from_save(data).is_ok(),
            "the same payload at the supported version loads"
        );
    }

    #[test]
    fn from_save_revalidates_the_world() {
        let engine = played_spoils_engine();
        let mut data = engine.save_data();
        data.world
            .rooms
            .get_mut("field")
            .expect("field saved")
            .exits
            .insert("west".to_string(), "ghost".to_string());
        assert_eq!(
            Engine::from_save(data).err(),
            Some(LoadError::InvalidWorld(
                WorldValidationError::DanglingExit {
                    room_id: "field".to_string(),
                    direction: "west".to_string(),
                    target_room_id: "ghost".to_string(),
                }
            )),
            "a corrupted world is refused through the same boundary as try_new"
        );
        assert!(
            Engine::from_save(engine.save_data()).is_ok(),
            "the uncorrupted payload loads"
        );
    }

    #[test]
    fn from_save_rejects_an_unknown_current_room() {
        let engine = played_spoils_engine();
        let mut data = engine.save_data();
        data.state.current_room_id = "ghost".to_string();
        assert_eq!(
            Engine::from_save(data).err(),
            Some(LoadError::StateIncoherent {
                what: "current room 'ghost' is not in the world".to_string(),
            }),
            "a state pointing at a missing room is refused, naming the room"
        );
        assert!(
            Engine::from_save(engine.save_data()).is_ok(),
            "the coherent payload loads"
        );
    }

    #[test]
    fn from_save_rejects_an_unknown_combat_enemy() {
        let mut engine = spoils_engine(30, 1, 0, false);
        engine.handle_command(cmd("attack stray"));
        let mut data = engine.save_data();
        data.state
            .combat
            .as_mut()
            .expect("encounter is active")
            .enemy_id = "ghost".to_string();
        assert_eq!(
            Engine::from_save(data).err(),
            Some(LoadError::StateIncoherent {
                what: "combat enemy 'ghost' is not in the world".to_string(),
            }),
            "a combat state naming a missing enemy is refused"
        );
        assert!(
            Engine::from_save(engine.save_data()).is_ok(),
            "the real mid-combat payload loads"
        );
    }

    #[test]
    fn from_save_rejects_an_unknown_sworn_oath() {
        let engine = Engine::try_new(oath_world()).expect("valid oath world");
        let mut data = engine.save_data();
        data.state.oath = Some(OathProgress {
            oath_id: "ghost".to_string(),
            title: "Ghost Oath".to_string(),
            status: OathStatus::Sworn,
        });
        assert_eq!(
            Engine::from_save(data).err(),
            Some(LoadError::StateIncoherent {
                what: "sworn oath 'ghost' is not in the world".to_string(),
            }),
            "an oath state naming a missing oath is refused"
        );
        let mut valid = engine.save_data();
        valid.state.oath = Some(OathProgress {
            oath_id: "o1".to_string(),
            title: "Test Oath".to_string(),
            status: OathStatus::Sworn,
        });
        assert!(
            Engine::from_save(valid).is_ok(),
            "the same oath state naming a real oath loads"
        );
    }

    // SL4b: the LoadError Display texts are part of the refusal contract the
    // server forwards verbatim to the player feed.
    #[test]
    fn load_error_messages_render() {
        assert_eq!(
            LoadError::VersionMismatch {
                found: 9,
                supported: 2
            }
            .to_string(),
            "save format version 9 is not supported (this build reads version 2)"
        );
        assert_eq!(
            LoadError::InvalidWorld(WorldValidationError::StartRoomMissing {
                start_room_id: "x".to_string()
            })
            .to_string(),
            "saved world failed validation: start room 'x' does not exist"
        );
        assert_eq!(
            LoadError::StateIncoherent {
                what: "current room 'ghost' is not in the world".to_string()
            }
            .to_string(),
            "saved state is incoherent: current room 'ghost' is not in the world"
        );
    }

    // SL5 (REQ-003, the tolerated class): orphan state ids that no engine
    // invariant depends on load fine and render through total fallbacks —
    // the audit's tolerance line, pinned.
    #[test]
    fn from_save_tolerates_orphan_pack_and_discovered_ids() {
        let engine = spoils_engine(8, 1, 0, false);
        let mut data = engine.save_data();
        data.state.pack.push("phantom_relic".to_string());
        data.state.discovered_rooms.insert("atlantis".to_string());
        let restored = Engine::from_save(data).expect("orphan ids are tolerated, not gated");
        let snapshot = restored.snapshot();
        let phantom = snapshot
            .pack
            .iter()
            .find(|item| item.id == "phantom_relic")
            .expect("the orphan pack id still renders");
        assert_eq!(
            phantom.name, "phantom_relic",
            "an unknown item falls back to its id instead of panicking"
        );
    }

    // SL6 (REQ-003 / inspect finding 1): the saturation rails. A crafted
    // save at the integer ceilings — tick and next_event_id at u64::MAX,
    // combat round at u32::MAX and due NOW — ticks, pulses, and emits
    // events without panicking; a debug build would otherwise abort on the
    // first overflow. Two ticks prove the rail holds, not just one step.
    #[test]
    fn loaded_integer_ceilings_saturate_instead_of_panicking() {
        let mut engine = spoils_engine(30, 1, 0, false);
        engine.handle_command(cmd("attack stray"));
        let mut data = engine.save_data();
        data.state.tick = u64::MAX;
        data.next_event_id = u64::MAX;
        {
            let combat = data.state.combat.as_mut().expect("encounter is active");
            combat.round = u32::MAX;
            combat.next_pulse_at = 0;
        }
        let mut restored =
            Engine::from_save(data).expect("ceiling values are coherent, just extreme");
        let events = restored.tick();
        assert!(
            events
                .iter()
                .any(|event| matches!(event.kind, GameEventKind::CombatPulse { round: u32::MAX })),
            "the overdue pulse fires at the round rail: {events:?}"
        );
        assert_eq!(
            restored.snapshot().tick,
            u64::MAX,
            "the tick clock saturates at the rail"
        );
        let _ = restored.tick();
    }

    // ---- ticket #29: the boss fight + fulfillment-on-recovery ----

    /// `oath_world` with the warden's combat profile overridden — the knobs
    /// for fight length and danger in the confront tests.
    fn boss_world(health: u32, attack: u32) -> WorldDefinition {
        let mut world = oath_world();
        world
            .entities
            .get_mut("warden")
            .expect("warden exists")
            .combat = Some(CombatProfile {
            health,
            attack,
            disclose_stats: false,
            xp: 0,
            coins: 0,
        });
        world
    }

    /// `boss_world` where the warden carries the oath's authored objective —
    /// the played shape: fight, drop, recover, fulfill.
    fn boss_objective_world(health: u32, attack: u32, xp: u64) -> WorldDefinition {
        let mut world = boss_world(health, attack);
        let warden = world.entities.get_mut("warden").expect("warden exists");
        warden.combat.as_mut().expect("profile present").xp = xp;
        warden.inventory = vec!["sigil".to_string()];
        world.items.insert("sigil".to_string(), item("sigil"));
        world
            .oaths
            .get_mut("o1")
            .expect("o1 exists")
            .objective_item_id = Some("sigil".to_string());
        world
    }

    /// The objective placed LOOSE in the start room (no fight needed): the
    /// synthetic shape for the fulfilled-with-a-living-boss arms, plus a
    /// non-objective `pebble` for the no-fulfill arm.
    fn loose_objective_world() -> WorldDefinition {
        let mut world = boss_world(4, 0);
        world.items.insert("sigil".to_string(), item("sigil"));
        world.items.insert("pebble".to_string(), item("pebble"));
        world.rooms.get_mut("town").expect("town exists").items =
            vec!["sigil".to_string(), "pebble".to_string()];
        world
            .oaths
            .get_mut("o1")
            .expect("o1 exists")
            .objective_item_id = Some("sigil".to_string());
        world
    }

    // B2 (REQ-001, inspect): confront mid-fight presses the attack — the
    // next round resolves against the SAME encounter (hp NOT reset to max,
    // the re-entry guard's killer) and no second CombatStarted is emitted.
    #[test]
    fn confront_mid_fight_resolves_a_round_without_resetting() {
        let mut engine = Engine::try_new(boss_world(12, 1)).expect("valid boss world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        assert!(engine.handle_command(cmd("east")).accepted);
        assert!(engine.handle_command(cmd("confront")).accepted);

        let response = engine.handle_command(cmd("confront"));
        assert!(response.accepted, "mid-fight confront presses the attack");
        let combat = engine.state.combat.as_ref().expect("the same encounter");
        assert_eq!(
            combat.enemy_hp, 4,
            "round 2 landed on the same fight (8 - 4), not a rebuilt 8"
        );
        assert_eq!(combat.round, 2, "the round counter advanced");
        assert!(
            !response
                .events
                .iter()
                .any(|e| matches!(e.kind, GameEventKind::CombatStarted { .. })),
            "no second CombatStarted: {:?}",
            response.events
        );
    }

    // B2b (inspect): the confront entry anchors the pulse exactly
    // DEFAULT_COMBAT_PULSE_TICKS out — tick 1 is silent, tick 2 pulses
    // round 2 (the moved engage_enemy pulse-anchor pin via the boss entry).
    #[test]
    fn confront_entry_anchors_the_pulse_two_ticks_out() {
        let mut engine = Engine::try_new(boss_world(12, 1)).expect("valid boss world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        assert!(engine.handle_command(cmd("east")).accepted);
        assert!(engine.handle_command(cmd("confront")).accepted);

        let first = engine.tick();
        assert!(
            !first
                .iter()
                .any(|e| matches!(e.kind, GameEventKind::CombatPulse { .. })),
            "one tick after confront no pulse is due: {first:?}"
        );
        let second = engine.tick();
        assert!(
            second
                .iter()
                .any(|e| matches!(e.kind, GameEventKind::CombatPulse { round: 2 })),
            "the pulse fires two ticks after the confront entry: {second:?}"
        );
    }

    // B3 (REQ-002): the boss falls through the existing victory funnel —
    // article-aware drop line (no "The The"), authored xp by value, the
    // placement removed, the objective on the floor — and the oath STAYS
    // Sworn (the no-fulfill-on-victory pin).
    #[test]
    fn boss_victory_drops_objective_and_awards_xp_without_fulfilling() {
        let mut engine =
            Engine::try_new(boss_objective_world(4, 0, 25)).expect("valid objective world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        assert!(engine.handle_command(cmd("east")).accepted);

        let response = engine.handle_command(cmd("confront"));
        assert!(response.accepted, "one strike fells the fixture warden");
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage {
                    component: OutputComponent::CombatMessage,
                    text,
                } if text == "The Warden drops sigil."
            )),
            "the drop line keeps the authored article (no 'The The'): {:?}",
            response.events
        );
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded { outcome: CombatOutcome::Victory, text }
                    if text == "You have defeated The Warden. Victory! You gain 25 XP."
            )),
            "{:?}",
            response.events
        );
        assert_eq!(response.snapshot.player.xp, 25, "authored boss xp by value");
        assert_eq!(
            response.snapshot.oath.expect("oath present").status,
            OathStatus::Sworn,
            "victory does NOT fulfill — recovery does"
        );
        assert!(response.snapshot.combat.is_none(), "the encounter is over");
        let lair = engine.world.rooms.get("lair").expect("lair exists");
        assert!(
            lair.items.iter().any(|id| id == "sigil"),
            "the objective lies on the lair floor"
        );
        assert!(
            !lair.entities.iter().any(|id| id == "warden"),
            "the boss placement is removed"
        );
    }

    // B5a (REQ-003 arm): a NON-objective take while sworn fulfills nothing.
    #[test]
    fn taking_a_non_objective_item_does_not_fulfill() {
        let mut engine = Engine::try_new(loose_objective_world()).expect("valid world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        let response = engine.handle_command(cmd("take pebble"));
        assert!(response.accepted, "the pebble is takeable");
        assert!(
            !response
                .events
                .iter()
                .any(|e| matches!(e.kind, GameEventKind::OathFulfilled { .. })),
            "a non-objective take emits no oath events"
        );
        assert_eq!(
            response.snapshot.oath.expect("oath present").status,
            OathStatus::Sworn,
            "the oath stays sworn"
        );
    }

    // B5b (REQ-003 arm): taking the objective with NO oath sworn is a plain
    // pickup — the Sworn gate's other arm.
    #[test]
    fn taking_the_objective_unsworn_is_a_plain_pickup() {
        let mut engine = Engine::try_new(loose_objective_world()).expect("valid world");
        let response = engine.handle_command(cmd("take sigil"));
        assert!(response.accepted, "the loose objective is takeable");
        assert!(
            !response
                .events
                .iter()
                .any(|e| matches!(e.kind, GameEventKind::OathFulfilled { .. })),
            "no oath, no fulfillment"
        );
        assert!(response.snapshot.oath.is_none(), "no oath is created");
    }

    // B5c (REQ-003 arm): once fulfilled, the objective is inert — a
    // drop-and-retake emits no second OathFulfilled and no announcements.
    #[test]
    fn refulfillment_is_inert() {
        let mut engine = Engine::try_new(loose_objective_world()).expect("valid world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        assert!(
            engine.handle_command(cmd("take sigil")).accepted,
            "fulfills"
        );
        assert!(engine.handle_command(cmd("drop sigil")).accepted);
        let response = engine.handle_command(cmd("take sigil"));
        assert!(response.accepted, "the re-take itself succeeds");
        assert!(
            !response.events.iter().any(|e| matches!(
                e.kind,
                GameEventKind::OathFulfilled { .. } | GameEventKind::Announcement { .. }
            )),
            "a kept oath cannot re-fire: {:?}",
            response.events
        );
        assert_eq!(
            response.snapshot.oath.expect("oath present").status,
            OathStatus::Fulfilled
        );
    }

    // B6b (REQ-004): after victory the boss placement is gone — re-confront
    // finds nothing to confront (the honest post-victory line).
    #[test]
    fn confront_after_victory_finds_nothing() {
        let mut engine =
            Engine::try_new(boss_objective_world(4, 0, 0)).expect("valid objective world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        assert!(engine.handle_command(cmd("east")).accepted);
        assert!(engine.handle_command(cmd("confront")).accepted, "victory");
        let response = engine.handle_command(cmd("confront"));
        assert!(!response.accepted, "there is no boss left to confront");
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. }
                    if text == "There is nothing here to confront."
            )),
            "{:?}",
            response.events
        );
        assert!(response.snapshot.combat.is_none());
    }

    // B7 (REQ-005): boss defeat runs the existing consequences — reset to
    // start with HP restored and the penalty applied — while the boss, its
    // inventory, and the SWORN oath all survive; the retry starts a FRESH
    // full-hp fight from the authored profile.
    #[test]
    fn boss_defeat_resets_player_and_leaves_the_world_intact_for_retry() {
        let mut engine =
            Engine::try_new(boss_objective_world(99, 10, 0)).expect("valid objective world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        assert!(engine.handle_command(cmd("east")).accepted);

        assert!(
            engine.handle_command(cmd("confront")).accepted,
            "round 1: the fight starts (player 10/20)"
        );
        let defeat = engine.handle_command(cmd("confront"));
        assert!(defeat.accepted, "round 2 is pressed");
        assert!(
            defeat.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Defeat,
                    ..
                }
            )),
            "the second 10-damage return fells the player: {:?}",
            defeat.events
        );
        assert_eq!(
            defeat.snapshot.current_room_id, "town",
            "defeat relocates to the start room"
        );
        assert_eq!(defeat.snapshot.player.hp, 20, "HP restored to max");
        assert_eq!(
            defeat.snapshot.player.xp, 0,
            "the penalty saturates at zero xp"
        );
        assert_eq!(
            defeat.snapshot.oath.expect("oath present").status,
            OathStatus::Sworn,
            "defeat does not break the oath"
        );
        let lair = engine.world.rooms.get("lair").expect("lair exists");
        assert!(
            lair.entities.iter().any(|id| id == "warden"),
            "the boss placement survives the defeat"
        );
        assert_eq!(
            engine
                .world
                .entities
                .get("warden")
                .expect("warden exists")
                .inventory,
            vec!["sigil".to_string()],
            "the boss keeps its objective for the retry"
        );

        assert!(engine.handle_command(cmd("east")).accepted, "re-climb");
        let retry = engine.handle_command(cmd("confront"));
        assert!(retry.accepted, "the retry engages a fresh encounter");
        let combat = engine.state.combat.as_ref().expect("a fresh encounter");
        assert_eq!(
            combat.enemy_hp, 95,
            "the retry fights a FRESH boss from the authored profile (99 - 4), not a stale resume"
        );
    }

    // B8 (REQ-007): a mid-boss-fight save round-trips through the #28
    // surface — byte-identical snapshot, and the restored engine's next
    // pulses replay the unsaved twin's exactly.
    #[test]
    fn mid_boss_fight_save_round_trips() {
        let mut engine = Engine::try_new(boss_world(12, 1)).expect("valid boss world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        assert!(engine.handle_command(cmd("east")).accepted);
        assert!(engine.handle_command(cmd("confront")).accepted);

        let at_save = snapshot_value(&engine);
        let mut restored = Engine::from_save(engine.save_data()).expect("mid-boss save loads");
        assert_eq!(
            snapshot_value(&restored),
            at_save,
            "the restored mid-fight snapshot is byte-identical"
        );
        let twin_events: Vec<GameEvent> = (0..2).flat_map(|_| engine.tick()).collect();
        let restored_events: Vec<GameEvent> = (0..2).flat_map(|_| restored.tick()).collect();
        assert_eq!(
            serde_json::to_value(&restored_events).expect("events serialize"),
            serde_json::to_value(&twin_events).expect("events serialize"),
            "the restored boss fight replays the twin's pulses exactly"
        );
    }

    // B9a (REQ-008 contract, both arms): a boss without a combat profile
    // fails construction with the exact missing text; the profiled fixture
    // passes. The boss-only (non-hostile) warden also kills a
    // `Role::Boss → Role::Hostile` arm-swap mutant.
    #[test]
    fn boss_without_combat_profile_fails_the_contract() {
        let mut world = oath_world();
        world
            .entities
            .get_mut("warden")
            .expect("warden exists")
            .combat = None;
        assert_eq!(
            world.validate(),
            Err(WorldValidationError::RoleContractUnmet {
                entity_id: "warden".to_string(),
                role: "boss".to_string(),
                missing: "a combat profile (health) so the boss can be confronted".to_string(),
            }),
            "a profile-less boss is rejected at construction"
        );
        assert_eq!(
            oath_world().validate(),
            Ok(()),
            "the profiled boss fixture validates"
        );
    }

    // B9b (REQ-008 contract, both arms): an oath objective must name a real
    // item — dangling rejected by value (+ Display), Some(real) and None
    // both validate.
    #[test]
    fn oath_objective_must_name_a_real_item() {
        let mut world = oath_world();
        world
            .oaths
            .get_mut("o1")
            .expect("o1 exists")
            .objective_item_id = Some("ghost".to_string());
        assert_eq!(
            world.validate(),
            Err(WorldValidationError::OathObjectiveMissing {
                oath_id: "o1".to_string(),
                item_id: "ghost".to_string(),
            }),
            "a dangling objective is rejected at construction"
        );
        assert_eq!(
            WorldValidationError::OathObjectiveMissing {
                oath_id: "o1".to_string(),
                item_id: "ghost".to_string(),
            }
            .to_string(),
            "oath 'o1' names missing objective item 'ghost'"
        );
        assert_eq!(
            boss_objective_world(4, 0, 0).validate(),
            Ok(()),
            "a real objective validates"
        );
        assert_eq!(
            oath_world().validate(),
            Ok(()),
            "an oath with no objective validates"
        );
    }

    // B10 (REQ-001/004): the boss is not ambushable around the oath gate —
    // `attack <boss>` refuses through the existing non-hostile path even in
    // a combat-enabled room.
    #[test]
    fn attacking_the_boss_directly_is_refused() {
        let mut world = boss_world(12, 4);
        world
            .rooms
            .get_mut("lair")
            .expect("lair exists")
            .combat_enabled = true;
        let mut engine = Engine::try_new(world).expect("valid boss world");
        assert!(engine.handle_command(cmd("east")).accepted);
        let response = engine.handle_command(cmd("attack the warden"));
        assert!(!response.accepted, "the boss is not hostile-attackable");
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. }
                    if text == "The Warden is not something you can attack."
            )),
            "{:?}",
            response.events
        );
        assert!(response.snapshot.combat.is_none(), "no encounter starts");
    }

    // ---- ticket #30: levels — the XP curve, the sync loop, the benefit ----

    // L1 (REQ-001): the curve's exact boundary ladder — both arms of every
    // threshold (the table-value and `>=`-flip killers) plus the base and
    // the rail.
    #[test]
    fn level_for_xp_boundary_ladder() {
        for (xp, level) in [
            (0_u64, 1_u32),
            (9, 1),
            (10, 2),
            (29, 2),
            (30, 3),
            (59, 3),
            (60, 4),
            (99, 4),
            (100, 5),
            (u64::MAX, 5),
        ] {
            assert_eq!(level_for_xp(xp), level, "xp {xp}");
        }
    }

    // L2 (REQ-001/002): a victory award crossing one threshold levels up —
    // the typed LevelUp follows CombatEnded in the same burst, by value, and
    // the benefit lands in state (max +5, healed to the new max from below).
    #[test]
    fn victory_award_levels_up_with_benefit() {
        let mut engine = spoils_engine(8, 1, 10, false);
        engine.handle_command(cmd("attack stray")); // round 1: enemy 4, player 19
        let _ = engine.tick();
        let t2 = engine.tick(); // pulse round 2: victory at exactly 0
        let ended_at = t2
            .iter()
            .position(|e| matches!(e.kind, GameEventKind::CombatEnded { .. }))
            .expect("the victory ends the fight");
        let level_up_at = t2
            .iter()
            .position(|e| {
                matches!(
                    (&e.channel, &e.kind),
                    (
                        EventChannel::Skill,
                        GameEventKind::LevelUp {
                            level: 2,
                            max_hp: 25,
                        }
                    )
                )
            })
            .expect("the 10-xp award crosses the first threshold");
        assert!(
            ended_at < level_up_at,
            "the level-up follows the victory summary: {t2:?}"
        );
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.player.level, 2, "level by value");
        assert_eq!(snapshot.player.max_hp, 25, "max grows by exactly 5");
        assert_eq!(
            snapshot.player.hp, 25,
            "the level-up heals to the NEW max (was 19/20 entering)"
        );
        assert_eq!(
            t2.iter()
                .filter(|e| matches!(e.kind, GameEventKind::LevelUp { .. }))
                .count(),
            1,
            "exactly one level gained, one event"
        );
    }

    // L3 (REQ-004): one award crossing TWO thresholds emits one LevelUp per
    // level, in ascending order, each with its own max — and the loop stops
    // exactly at the curve (the `<=` overshoot killer).
    #[test]
    fn multi_threshold_award_levels_once_per_level() {
        let mut engine = spoils_engine(8, 1, 35, false);
        engine.handle_command(cmd("attack stray"));
        let _ = engine.tick();
        let t2 = engine.tick(); // victory: 35 xp crosses 10 and 30
        let levels: Vec<(u32, i32)> = t2
            .iter()
            .filter_map(|e| match e.kind {
                GameEventKind::LevelUp { level, max_hp } => Some((level, max_hp)),
                _ => None,
            })
            .collect();
        assert_eq!(
            levels,
            vec![(2, 25), (3, 30)],
            "one event per level, ascending, each with its own max: {t2:?}"
        );
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.player.level, 3);
        assert_eq!(snapshot.player.max_hp, 30);
        assert_eq!(
            snapshot.player.hp, 30,
            "healed at each step, ends at the final max"
        );
    }

    // L4 (REQ-003): the ratchet — a defeat penalty dropping xp BELOW a held
    // threshold de-levels nothing and emits nothing; this is also the
    // timeout-killer for a `while level < target` → `!=` mutant (which
    // would spin forever here, since the stored level EXCEEDS the curve).
    #[test]
    fn defeat_penalty_never_delevels() {
        let mut engine = spoils_engine(8, 1, 10, false);
        engine.handle_command(cmd("attack stray"));
        let _ = engine.tick();
        let _ = engine.tick(); // victory: xp 10 → level 2, 25/25
        assert_eq!(engine.snapshot().player.level, 2);

        // Walk into the brute (99/99) and lose: penalty 10 → 9 xp, curve
        // says level 1 — the ratchet keeps 2.
        let response = engine.handle_command(cmd("attack brute"));
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Defeat,
                    ..
                }
            )),
            "the brute fells the player: {:?}",
            response.events
        );
        assert!(
            !response
                .events
                .iter()
                .any(|e| matches!(e.kind, GameEventKind::LevelUp { .. })),
            "a penalty below the threshold emits no level events"
        );
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.player.xp, 9, "the penalty landed (10 - 1)");
        assert_eq!(snapshot.player.level, 2, "the milestone is KEPT (ratchet)");
        assert_eq!(snapshot.player.max_hp, 25, "no benefit clawback");
    }

    // L5 (REQ-007): levels round-trip through the #28 surface byte-for-byte,
    // and a STALE pair (level 1 with banked xp — a pre-#30 save) converges
    // lazily at its next combat end, firing the earned LevelUps then.
    #[test]
    fn levels_round_trip_and_stale_saves_converge_lazily() {
        let mut engine = spoils_engine(8, 1, 10, false);
        engine.handle_command(cmd("attack stray"));
        let _ = engine.tick();
        let _ = engine.tick(); // level 2, 25/25, xp 10
        let at_save = snapshot_value(&engine);
        let restored = Engine::from_save(engine.save_data()).expect("a leveled save loads");
        assert_eq!(
            snapshot_value(&restored),
            at_save,
            "level and max_hp round-trip byte-for-byte"
        );

        // The stale pair: bank 35 xp at level 1 (a pre-#30 save's shape).
        let stale = Engine::try_new(combat_world(8, 1)).expect("valid combat world");
        let mut data = stale.save_data();
        data.state.player.xp = 35;
        let mut loaded = Engine::from_save(data).expect("a stale pair is sound, not gated");
        assert_eq!(
            loaded.snapshot().player.level,
            1,
            "loading converges NOTHING yet (lazy, not on-load)"
        );
        loaded.handle_command(cmd("attack stray"));
        let _ = loaded.tick();
        let t2 = loaded.tick(); // any combat end syncs: curve(35) = 3
        let levels: Vec<u32> = t2
            .iter()
            .filter_map(|e| match e.kind {
                GameEventKind::LevelUp { level, .. } => Some(level),
                _ => None,
            })
            .collect();
        assert_eq!(
            levels,
            vec![2, 3],
            "the banked milestones surface at the next combat end: {t2:?}"
        );
        assert_eq!(loaded.snapshot().player.level, 3);
    }

    // ============================================================
    //  ticket #31 — focus economy v1
    // ============================================================

    // The refusal line of a response, asserted by exact value (the refusal
    // family is System-channel — never a combat line, never logged).
    fn system_line(response: &CommandResponse) -> String {
        match &response.events[0].kind {
            GameEventKind::LogMessage {
                component: OutputComponent::SystemMessage,
                text,
            } => text.clone(),
            other => panic!("expected a system line, got {other:?}"),
        }
    }

    // F-T9 (REQ-001/002): the cost and refusal tables, every variant exact —
    // the only killer for the const-fn table mutants (enumerate-variant rule).
    #[test]
    fn focus_cost_and_refusal_tables_are_exact() {
        assert_eq!(CombatAction::Flee.focus_cost(), 0);
        assert_eq!(CombatAction::Guard.focus_cost(), GUARD_FOCUS_COST);
        assert_eq!(CombatAction::Guard.focus_cost(), 1);
        assert_eq!(
            CombatAction::PowerStrike.focus_cost(),
            POWER_STRIKE_FOCUS_COST
        );
        assert_eq!(CombatAction::PowerStrike.focus_cost(), 2);
        // Flee's refusal arm is product-unreachable while flee is free; the
        // unit assert is its coverage + mutation pin.
        assert_eq!(
            CombatAction::Flee.focus_refusal(),
            "You lack the focus to flee."
        );
        assert_eq!(
            CombatAction::Guard.focus_refusal(),
            "You lack the focus to guard."
        );
        assert_eq!(
            CombatAction::PowerStrike.focus_refusal(),
            "You lack the focus for a power strike."
        );
    }

    // F-T1/T2/T4 (REQ-001): queueing spends the authored cost exactly once,
    // observable in the snapshot, with the confirmation lines byte-identical
    // to #25's. Flee stays free.
    #[test]
    fn queueing_a_skill_spends_its_cost_in_the_snapshot() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        let strike = engine.handle_command(cmd("power strike"));
        assert!(strike.accepted);
        assert_eq!(combat_line(&strike), "You wind up a power strike.");
        assert_eq!(strike.snapshot.player.focus, 3, "5 - 2 on the queue");
        assert_eq!(strike.snapshot.player.max_focus, 5, "the pool cap holds");

        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        let guard = engine.handle_command(cmd("guard"));
        assert_eq!(
            combat_line(&guard),
            "You ready your guard for the next blow."
        );
        assert_eq!(guard.snapshot.player.focus, 4, "5 - 1 on the queue");

        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        let flee = engine.handle_command(cmd("flee"));
        assert_eq!(combat_line(&flee), "You watch for an opening to flee.");
        assert_eq!(flee.snapshot.player.focus, 5, "flee is free");
    }

    // F-T3 (REQ-001/003): re-queueing the same action charges nothing — the
    // focus ledger across already / change-tack / already shows each cost
    // taken exactly once.
    #[test]
    fn requeue_and_change_tack_keep_the_ledger_exact() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        assert_eq!(engine.handle_command(cmd("guard")).snapshot.player.focus, 4);
        let again = engine.handle_command(cmd("guard"));
        assert!(again.accepted);
        assert_eq!(
            again.snapshot.player.focus, 4,
            "the already-arm never re-charges"
        );
        let switched = engine.handle_command(cmd("power strike"));
        assert_eq!(
            switched.snapshot.player.focus, 3,
            "refund the guard (+1), charge the strike (-2)"
        );
        let again = engine.handle_command(cmd("power strike"));
        assert_eq!(
            again.snapshot.player.focus, 3,
            "already winding up: no charge"
        );
    }

    // F-T5 (REQ-001): the cost is committed at the queue — the firing window
    // charges nothing more.
    #[test]
    fn a_fired_power_strike_charges_nothing_more() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray")); // r1: enemy 36
        engine.handle_command(cmd("power strike")); // focus 3
        let _ = engine.tick();
        let t2 = engine.tick(); // P1: enemy 32; P2: strike fires → 26
        assert_eq!(count_lines(&t2, "power strike slams"), 1);
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.player.focus, 3, "no second charge at resolution");
        assert_eq!(enemy_of(&snapshot.combat.expect("active")).hp, 26);
    }

    // F-T6/T8a (REQ-002): the exact-affordability boundary queues — focus ==
    // cost accepts and lands on zero (the `<` vs `<=` mutant killer).
    #[test]
    fn an_exactly_affordable_skill_queues() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.state.player.focus = 2;
        let strike = engine.handle_command(cmd("power strike"));
        assert!(strike.accepted, "focus == cost is affordable");
        assert_eq!(strike.snapshot.player.focus, 0);
        assert_eq!(
            active_combat(&strike).queued_action.as_deref(),
            Some("power_strike")
        );

        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.state.player.focus = 1;
        let guard = engine.handle_command(cmd("guard"));
        assert!(guard.accepted);
        assert_eq!(guard.snapshot.player.focus, 0);
    }

    // F-T7/T8b (REQ-002): one short of the cost refuses with the typed line
    // and changes no state — focus, queue, and the persisted battle log all
    // hold (the refusal is System-channel, never logged).
    #[test]
    fn an_unaffordable_skill_refuses_and_mutates_nothing() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.state.player.focus = 1;
        let log_before = engine.state.combat.as_ref().expect("active").log.len();
        let strike = engine.handle_command(cmd("power strike"));
        assert!(!strike.accepted, "focus == cost - 1 refuses");
        assert_eq!(
            system_line(&strike),
            "You lack the focus for a power strike."
        );
        assert_eq!(strike.snapshot.player.focus, 1, "nothing spent");
        assert_eq!(active_combat(&strike).queued_action, None, "nothing queued");
        assert_eq!(
            engine.state.combat.as_ref().expect("active").log.len(),
            log_before,
            "the refusal never lands in the persisted battle log"
        );

        engine.state.player.focus = 0;
        let guard = engine.handle_command(cmd("guard"));
        assert!(!guard.accepted);
        assert_eq!(system_line(&guard), "You lack the focus to guard.");
        assert_eq!(guard.snapshot.player.focus, 0);
    }

    // F-T10 (REQ-002/003): flee queues at zero focus — the `cost > 0` arm of
    // the gate, exercised independently of the affordability arm.
    #[test]
    fn flee_queues_at_zero_focus() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.state.player.focus = 0;
        let flee = engine.handle_command(cmd("flee"));
        assert!(flee.accepted, "free verbs stay free");
        assert_eq!(combat_line(&flee), "You watch for an opening to flee.");
        assert_eq!(flee.snapshot.player.focus, 0);
    }

    // F-T11/T12/T13 (REQ-003): the change-tack chain settles deterministically
    // — refund the replaced cost, charge the new one; replacing a paid strike
    // with free flee lands exactly back at the pre-queue pool (no gain).
    #[test]
    fn change_tack_settles_refund_then_charge() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        assert_eq!(engine.handle_command(cmd("guard")).snapshot.player.focus, 4);
        let to_strike = engine.handle_command(cmd("power strike"));
        assert_eq!(
            combat_line(&to_strike),
            "You change tack. You wind up a power strike."
        );
        assert_eq!(to_strike.snapshot.player.focus, 3, "4 + 1 - 2");
        let to_guard = engine.handle_command(cmd("guard"));
        assert_eq!(to_guard.snapshot.player.focus, 4, "3 + 2 - 1");
        let to_flee = engine.handle_command(cmd("flee"));
        assert_eq!(
            combat_line(&to_flee),
            "You change tack. You watch for an opening to flee."
        );
        assert_eq!(
            to_flee.snapshot.player.focus, 5,
            "a full refund and a free queue: exactly the pre-queue pool, never more"
        );
    }

    // F-T14/T15 (REQ-003): the refund-adjusted boundary, both arms — a
    // replace the refund cannot cover refuses (old action and old charge
    // stand); one more point of focus and the same replace queues.
    #[test]
    fn change_tack_respects_the_refund_adjusted_boundary() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.state.player.focus = 1;
        assert!(engine.handle_command(cmd("guard")).accepted); // focus 0, guard queued
        let refused = engine.handle_command(cmd("power strike"));
        assert!(!refused.accepted, "0 + 1 refunded < 2");
        assert_eq!(
            system_line(&refused),
            "You lack the focus for a power strike."
        );
        assert_eq!(refused.snapshot.player.focus, 0, "the old charge stands");
        assert_eq!(
            active_combat(&refused).queued_action.as_deref(),
            Some("guard"),
            "the old action stands"
        );

        engine.state.player.focus = 1; // 1 + 1 refunded == 2: affordable
        let switched = engine.handle_command(cmd("power strike"));
        assert!(switched.accepted);
        assert_eq!(switched.snapshot.player.focus, 0);
        assert_eq!(
            active_combat(&switched).queued_action.as_deref(),
            Some("power_strike")
        );
    }

    // F-T16 (REQ-004): rest out of combat restores the pool to its maximum
    // with the exact narrative line.
    #[test]
    fn rest_restores_focus_out_of_combat() {
        let mut engine = combat_engine(40, 1);
        engine.state.player.focus = 1;
        let rest = engine.handle_command(cmd("rest"));
        assert!(rest.accepted);
        assert!(
            matches!(
                &rest.events[0].kind,
                GameEventKind::LogMessage { component: OutputComponent::NarrativeMessage, text }
                    if text == "You rest. Focus returns to you (5/5)."
            ),
            "the narrative line by value: {:?}",
            rest.events
        );
        assert_eq!(rest.snapshot.player.focus, 5);
    }

    // F-T17 (REQ-004): rest refuses mid-combat with the typed line and no
    // recovery (the pool staged off-max so the no-op is distinguishable).
    #[test]
    fn rest_refuses_mid_combat() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.state.player.focus = 1;
        let rest = engine.handle_command(cmd("rest"));
        assert!(!rest.accepted);
        assert_eq!(
            system_line(&rest),
            "There is no rest in the midst of battle."
        );
        assert_eq!(rest.snapshot.player.focus, 1, "no recovery mid-fight");
    }

    // F-T18 (REQ-004): an already-full pool refuses as a no-op (`>=` — the
    // full-pool arm of the gate).
    #[test]
    fn rest_refuses_when_already_full() {
        let mut engine = combat_engine(40, 1);
        let rest = engine.handle_command(cmd("rest"));
        assert!(!rest.accepted);
        assert_eq!(system_line(&rest), "You are already fully focused.");
        assert_eq!(rest.snapshot.player.focus, 5);
    }

    // F-T20 (REQ-004): help lists the recovery verb.
    #[test]
    fn help_lists_rest() {
        let mut engine = combat_engine(10, 3);
        let response = engine.handle_command(cmd("help"));
        let text = match &response.events[0].kind {
            GameEventKind::LogMessage { text, .. } => text.clone(),
            other => panic!("help is a log line, got {other:?}"),
        };
        assert!(text.contains("rest"), "help lists rest: {text}");
    }

    // F-T21 (REQ-005): victory keeps the spent pool — the economy's bite.
    #[test]
    fn victory_keeps_the_spent_pool() {
        let mut engine = combat_engine(14, 1);
        engine.handle_command(cmd("attack stray")); // r1: enemy 10
        engine.handle_command(cmd("power strike")); // focus 3
        let _ = engine.tick();
        let t2 = engine.tick(); // P1: enemy 6; P2: strike → 0, victory
        assert!(
            t2.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Victory,
                    ..
                }
            )),
            "the strike fells the stray: {t2:?}"
        );
        assert_eq!(engine.snapshot().player.focus, 3, "spent stays spent");
    }

    // F-T24 (REQ-003/005): a phase-1 kill ends the fight with the strike
    // still queued — the unfired cost comes back.
    #[test]
    fn a_phase_one_kill_refunds_the_unfired_queue() {
        let mut engine = combat_engine(8, 1);
        engine.handle_command(cmd("attack stray")); // r1: enemy 4
        engine.handle_command(cmd("power strike")); // focus 3, queued
        let _ = engine.tick();
        let t2 = engine.tick(); // P1: enemy 0 → victory; the window never runs
        assert!(
            t2.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Victory,
                    ..
                }
            )),
            "phase 1 fells the stray: {t2:?}"
        );
        assert_eq!(
            count_lines(&t2, "power strike slams"),
            0,
            "the strike never fired"
        );
        assert_eq!(
            engine.snapshot().player.focus,
            5,
            "the unfired cost refunds"
        );
    }

    // F-T22 (REQ-005): defeat restores focus beside hp — battered but whole —
    // and executes the refund-then-restore order with an action still queued.
    #[test]
    fn defeat_restores_focus_with_hp() {
        let mut engine = combat_engine(40, 5);
        engine.handle_command(cmd("attack stray")); // r1: enemy 36, player 15
        engine.handle_command(cmd("power strike")); // focus 3, queued
        engine.state.player.hp = 3; // the next return (5) is lethal
        let _ = engine.tick();
        let t2 = engine.tick(); // P1: the return fells the player → defeat
        assert!(
            t2.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Defeat,
                    ..
                }
            )),
            "the return fells the player: {t2:?}"
        );
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.player.hp, 20, "defeat restores hp (the #26 reset)");
        assert_eq!(snapshot.player.focus, 5, "defeat restores focus with it");
        assert!(snapshot.combat.is_none());
    }

    // F-T23 (REQ-005): fleeing keeps the spent pool — a fired guard's cost
    // stays spent through the fled outcome.
    #[test]
    fn fleeing_keeps_the_spent_pool() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.handle_command(cmd("guard")); // focus 4
        let _ = engine.tick();
        let _ = engine.tick(); // P2 arms the guard; the queue clears
        engine.handle_command(cmd("flee")); // free
        let _ = engine.tick();
        let t2 = engine.tick(); // P2 resolves the flee
        assert!(
            t2.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Fled,
                    ..
                }
            )),
            "the flee resolves: {t2:?}"
        );
        assert_eq!(
            engine.snapshot().player.focus,
            4,
            "the guard's point stays spent"
        );
    }

    // F-T28a (REQ-002/007): a crafted negative pool never blocks flee — the
    // `cost > 0` conjunct holds at -1 directly and at i32::MIN through the
    // load boundary (the required `>` → `>=` mutant killer).
    #[test]
    fn crafted_negative_focus_never_blocks_flee() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.state.player.focus = -1;
        let strike = engine.handle_command(cmd("power strike"));
        assert!(!strike.accepted);
        assert_eq!(
            system_line(&strike),
            "You lack the focus for a power strike."
        );
        let guard = engine.handle_command(cmd("guard"));
        assert!(!guard.accepted);
        assert_eq!(system_line(&guard), "You lack the focus to guard.");
        let flee = engine.handle_command(cmd("flee"));
        assert!(flee.accepted, "flee is free even in deficit");
        assert_eq!(combat_line(&flee), "You watch for an opening to flee.");
        assert_eq!(flee.snapshot.player.focus, -1, "free means untouched");

        let mut donor = combat_engine(40, 1);
        donor.handle_command(cmd("attack stray"));
        let mut data = donor.save_data();
        data.state.player.focus = i32::MIN;
        let mut loaded = Engine::from_save(data).expect("a crafted pool is tolerated");
        let flee = loaded.handle_command(cmd("flee"));
        assert!(
            flee.accepted,
            "flee survives the extreme through the load boundary"
        );
        assert_eq!(flee.snapshot.player.focus, i32::MIN);
    }

    // F-T28b (REQ-007): every new arithmetic site survives crafted extremes —
    // the end-of-fight refund and the change-tack refund saturate at i32::MAX
    // (no overflow panic), rest never clamps an above-max pool, a zero-max
    // pool refuses rest, and rest recovers from i32::MIN.
    #[test]
    fn crafted_extremes_survive_the_new_arithmetic() {
        // End-of-fight refund at i32::MAX: phase-1 kill with a queued strike.
        let mut donor = combat_engine(8, 1);
        donor.handle_command(cmd("attack stray")); // r1: enemy 4
        let mut data = donor.save_data();
        data.state.player.focus = i32::MAX;
        data.state
            .combat
            .as_mut()
            .expect("mid-fight save")
            .queued_action = Some(CombatAction::PowerStrike);
        let mut loaded = Engine::from_save(data).expect("crafted extremes load");
        let _ = loaded.tick();
        let _ = loaded.tick(); // P1 kill → the refund saturates, no panic
        assert!(loaded.snapshot().combat.is_none(), "the fight resolved");
        assert_eq!(
            loaded.snapshot().player.focus,
            i32::MAX,
            "saturated, not wrapped"
        );

        // Change-tack refund at i32::MAX: replace the crafted queued strike.
        let mut donor = combat_engine(40, 1);
        donor.handle_command(cmd("attack stray"));
        let mut data = donor.save_data();
        data.state.player.focus = i32::MAX;
        data.state
            .combat
            .as_mut()
            .expect("mid-fight save")
            .queued_action = Some(CombatAction::PowerStrike);
        let mut loaded = Engine::from_save(data).expect("crafted extremes load");
        let switched = loaded.handle_command(cmd("guard"));
        assert!(switched.accepted);
        assert_eq!(
            switched.snapshot.player.focus,
            i32::MAX - 1,
            "saturated refund, exact charge, no panic"
        );

        // Rest tolerance: above-max refuses (never clamps down) …
        let mut engine = combat_engine(40, 1);
        engine.state.player.focus = 7;
        let rest = engine.handle_command(cmd("rest"));
        assert!(!rest.accepted);
        assert_eq!(system_line(&rest), "You are already fully focused.");
        assert_eq!(
            rest.snapshot.player.focus, 7,
            "an above-max pool is tolerated"
        );
        // … a zero-max pool has nothing to recover …
        engine.state.player.focus = 0;
        engine.state.player.max_focus = 0;
        assert!(!engine.handle_command(cmd("rest")).accepted);
        // … and a deficit recovers to the true maximum.
        let mut engine = combat_engine(40, 1);
        engine.state.player.focus = i32::MIN;
        let rest = engine.handle_command(cmd("rest"));
        assert!(rest.accepted);
        assert_eq!(rest.snapshot.player.focus, 5);
    }

    // F-T29 (REQ-007): a spent pool round-trips the save boundary — 5/5 was
    // the only value ever pinned before the economy.
    #[test]
    fn spent_focus_round_trips_through_save() {
        let mut engine = combat_engine(40, 1);
        engine.handle_command(cmd("attack stray"));
        engine.handle_command(cmd("power strike")); // focus 3
        let at_save = snapshot_value(&engine);
        let restored = Engine::from_save(engine.save_data()).expect("mid-fight save loads");
        assert_eq!(snapshot_value(&restored), at_save, "byte-identical restore");
        assert_eq!(
            restored.snapshot().player.focus,
            3,
            "the spent value survives"
        );
    }

    // ============================================================
    //  ticket #33 — map entity markers
    // ============================================================

    // The map-room entry for `id` in a snapshot.
    fn map_room_of(engine: &Engine, id: &str) -> oathstar_protocol::MapRoomSnapshot {
        engine
            .snapshot()
            .map
            .rooms
            .into_iter()
            .find(|room| room.id == id)
            .expect("the room is on the map")
    }

    // M-T1 (REQ-001): both arms of the discovered gate — the discovered start
    // room flags live placements; the undiscovered ridge stays dark even with
    // a hostile (wolf) AND an inserted item placed there (fog never leaks).
    #[test]
    fn marker_flags_gate_on_discovery() {
        let mut world = combat_world(10, 3);
        world.items.insert("relic".to_string(), item("relic"));
        world
            .rooms
            .get_mut("ridge")
            .expect("fixture room")
            .items
            .push("relic".to_string());
        world
            .rooms
            .get_mut("field")
            .expect("fixture room")
            .items
            .push("relic".to_string());
        let engine = Engine::try_new(world).expect("valid world");

        let field = map_room_of(&engine, "field");
        assert!(field.discovered, "the start room is discovered");
        assert!(field.has_hostiles, "the stray is a live hostile placement");
        assert!(field.has_items, "the inserted relic placement flags");

        let ridge = map_room_of(&engine, "ridge");
        assert!(!ridge.discovered, "ridge is never visited");
        assert!(
            !ridge.has_hostiles,
            "fog conceals the wolf (kills && -> ||)"
        );
        assert!(!ridge.has_items, "fog conceals the relic");
    }

    // M-T2 (REQ-001): the presence arms — a discovered room with no hostiles
    // and no items flags neither.
    #[test]
    fn marker_flags_stay_false_without_presence() {
        let mut engine = combat_engine(10, 3);
        assert!(engine.handle_command(cmd("south")).accepted, "to clearing");
        let clearing = map_room_of(&engine, "clearing");
        assert!(clearing.discovered);
        assert!(!clearing.has_hostiles, "no hostile placed in the clearing");
        assert!(!clearing.has_items, "no items placed in the clearing");
    }

    // M-T3 (REQ-001): a discovered room whose only actors are NON-hostile
    // never flags — the role filter, not mere entity presence.
    #[test]
    fn non_hostile_actors_do_not_flag() {
        let mut world = combat_world(10, 3);
        world
            .rooms
            .get_mut("field")
            .expect("fixture room")
            .entities
            .retain(|id| id == "elder" || id == "idol");
        let engine = Engine::try_new(world).expect("valid world");
        let field = map_room_of(&engine, "field");
        assert!(field.discovered);
        assert!(!field.has_hostiles, "the elder and idol are not hostiles");
    }

    // M-T4 (REQ-002): the lifecycle — defeat clears the hostile flag and the
    // dropped loot raises the item flag; taking it clears that too. Staged so
    // every before differs from its after.
    #[test]
    fn marker_flags_track_defeat_loot_and_take() {
        let mut world = combat_world(8, 1);
        world
            .rooms
            .get_mut("field")
            .expect("fixture room")
            .entities
            .retain(|id| id != "brute");
        world.items.insert("fang".to_string(), item("fang"));
        world
            .entities
            .get_mut("stray")
            .expect("fixture stray")
            .inventory
            .push("fang".to_string());
        let mut engine = Engine::try_new(world).expect("valid world");

        let before = map_room_of(&engine, "field");
        assert!(before.has_hostiles, "the stray prowls before the fight");
        assert!(!before.has_items, "no ground items yet");

        assert!(engine.handle_command(cmd("attack stray")).accepted); // 8 -> 4
        let won = engine.handle_command(cmd("attack")); // 4 -> 0: victory
        assert!(
            won.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::CombatEnded {
                    outcome: CombatOutcome::Victory,
                    ..
                }
            )),
            "two strikes fell the 8hp stray: {:?}",
            won.events
        );

        let after = map_room_of(&engine, "field");
        assert!(!after.has_hostiles, "victory removes the placement");
        assert!(after.has_items, "the dropped fang flags the room");

        assert!(engine.handle_command(cmd("take fang")).accepted);
        assert!(
            !map_room_of(&engine, "field").has_items,
            "taken loot unflags"
        );
    }

    // M-T5 (REQ-002): dropping a carried item raises the destination room's
    // flag — the other direction of the item transition.
    #[test]
    fn dropping_an_item_flags_the_room() {
        let mut world = combat_world(10, 3);
        world.items.insert("relic".to_string(), item("relic"));
        world
            .rooms
            .get_mut("field")
            .expect("fixture room")
            .items
            .push("relic".to_string());
        let mut engine = Engine::try_new(world).expect("valid world");
        assert!(engine.handle_command(cmd("take relic")).accepted);
        assert!(!map_room_of(&engine, "field").has_items, "taken away");
        assert!(engine.handle_command(cmd("south")).accepted);
        assert!(engine.handle_command(cmd("drop relic")).accepted);
        let clearing = map_room_of(&engine, "clearing");
        assert!(clearing.has_items, "the dropped relic flags the clearing");
    }

    // M-T13 (REQ-001, inspect finding #3): hidden things never reach the map —
    // the reveal rule's mirror. A discovered room whose ONLY hostile and ONLY
    // item are hidden flags neither.
    #[test]
    fn hidden_content_never_flags_the_map() {
        let mut world = combat_world(10, 3);
        let mut lurker = entity("lurker", EntityKind::Actor, &["combatant", "hostile"], &[]);
        lurker.hidden = true;
        lurker.combat = Some(CombatProfile {
            health: 5,
            attack: 1,
            disclose_stats: false,
            xp: 0,
            coins: 0,
        });
        world.entities.insert("lurker".to_string(), lurker);
        let mut secret = item("secret");
        secret.hidden = true;
        world.items.insert("secret".to_string(), secret);
        let field = world.rooms.get_mut("field").expect("fixture room");
        field.entities.retain(|id| id == "elder" || id == "idol");
        field.entities.push("lurker".to_string());
        field.items.push("secret".to_string());
        let engine = Engine::try_new(world).expect("valid world");

        let room = map_room_of(&engine, "field");
        assert!(room.discovered);
        assert!(
            !room.has_hostiles,
            "the hidden lurker stays off the map (look conceals it too)"
        );
        assert!(!room.has_items, "the hidden secret stays off the map");
    }

    // ============================================================
    //  ticket #34 — commerce v1
    // ============================================================

    // A commerce world: the combat fixture plus a vendor "keeper" placed in
    // the start room ("field") stocking a priced lamp (8), a priceless relic
    // (0), and nothing else; a sellable trinket (value 2) lies on the ground.
    fn commerce_world() -> WorldDefinition {
        let mut world = combat_world(8, 1);
        // The fight stays available but the room's OTHER hostile leaves so a
        // single victory clears the hostile presence.
        world
            .rooms
            .get_mut("field")
            .expect("fixture room")
            .entities
            .retain(|id| id != "brute");
        let mut lamp = item("lamp");
        lamp.value = 8;
        world.items.insert("lamp".to_string(), lamp);
        let mut relic = item("relic");
        relic.value = 0;
        world.items.insert("relic".to_string(), relic);
        let mut trinket = item("trinket");
        trinket.value = 2;
        world.items.insert("trinket".to_string(), trinket);
        let mut keeper = entity(
            "keeper",
            EntityKind::Actor,
            &["shopkeeper"],
            &["lamp", "relic"],
        );
        keeper.name = "Keeper".to_string();
        world.entities.insert("keeper".to_string(), keeper);
        let field = world.rooms.get_mut("field").expect("fixture room");
        field.entities.push("keeper".to_string());
        field.items.push("trinket".to_string());
        world
    }

    fn commerce_engine() -> Engine {
        Engine::try_new(commerce_world()).expect("valid commerce world")
    }

    // C-T1 (REQ-001): the listing — priced and priceless arms exact, the
    // coins footer, and the empty-stock refusal.
    #[test]
    fn shop_lists_stock_with_prices() {
        let mut engine = commerce_engine();
        engine.state.player.coins = 3;
        let listed = engine.handle_command(cmd("shop"));
        assert!(listed.accepted);
        assert_eq!(
            log_text(&listed),
            "Keeper offers: lamp — 8 coins; relic — not for sale. You have 3 coins."
        );

        // Empty the stock: the listing becomes the typed refusal.
        engine
            .world
            .entities
            .get_mut("keeper")
            .expect("vendor")
            .inventory
            .clear();
        let empty = engine.handle_command(cmd("browse"));
        assert!(!empty.accepted);
        assert_eq!(system_line(&empty), "Keeper has nothing to sell.");
    }

    // C-T1b (inspect #1): hidden stock stays off the counter and reads as
    // unstocked at buy — the reveal rule on the commerce projection.
    #[test]
    fn hidden_stock_is_neither_listed_nor_buyable() {
        let mut engine = commerce_engine();
        engine.state.player.coins = 50;
        engine
            .world
            .items
            .get_mut("lamp")
            .expect("authored item")
            .hidden = true;
        let listed = engine.handle_command(cmd("shop"));
        assert!(
            !log_text(&listed).contains("lamp"),
            "hidden stock is not listed: {}",
            log_text(&listed)
        );
        let bought = engine.handle_command(cmd("buy lamp"));
        assert!(!bought.accepted);
        assert_eq!(system_line(&bought), "Keeper does not have 'lamp'.");
        assert_eq!(bought.snapshot.player.coins, 50, "nothing spent");
    }

    // C-T1c (inspect #4): a HIDDEN shopkeeper does not trade — the no-vendor
    // refusal fires even though a visible non-shopkeeper (the stray) stands
    // in the room (kills find_vendor's `&&` -> `||` mutant).
    #[test]
    fn a_hidden_shopkeeper_does_not_trade() {
        let mut engine = commerce_engine();
        engine
            .world
            .entities
            .get_mut("keeper")
            .expect("vendor")
            .hidden = true;
        let listed = engine.handle_command(cmd("shop"));
        assert!(!listed.accepted);
        assert_eq!(
            system_line(&listed),
            "There is no shopkeeper here to trade with."
        );
    }

    // C-T1d (inspect #4): `shop` mid-combat refuses like every trade verb.
    #[test]
    fn shop_refuses_mid_combat_and_without_a_vendor() {
        let mut engine = commerce_engine();
        engine.handle_command(cmd("attack stray"));
        let fighting = engine.handle_command(cmd("shop"));
        assert!(!fighting.accepted);
        assert_eq!(
            system_line(&fighting),
            "There is no trading in the midst of battle."
        );

        // A vendor-less room with a visible bystander (the elder walked past
        // at fixture time) still refuses honestly.
        let mut engine = combat_engine(8, 1);
        let refused = engine.handle_command(cmd("shop"));
        assert!(!refused.accepted);
        assert_eq!(
            system_line(&refused),
            "There is no shopkeeper here to trade with."
        );
    }

    // C-T2 (REQ-002/003): the affordability boundary, both arms exact — and
    // the settlement is exactly-once on BOTH sides of the counter.
    #[test]
    fn buying_settles_exactly_once_at_the_boundary() {
        let mut engine = commerce_engine();
        engine.state.player.coins = 7; // price - 1
        let refused = engine.handle_command(cmd("buy lamp"));
        assert!(!refused.accepted);
        assert_eq!(
            system_line(&refused),
            "You cannot afford lamp (8 coins; you have 7)."
        );
        assert_eq!(refused.snapshot.player.coins, 7, "nothing spent");
        assert!(refused.snapshot.pack.is_empty(), "nothing gained");

        engine.state.player.coins = 8; // exactly the price
        let bought = engine.handle_command(cmd("buy lamp"));
        assert!(bought.accepted);
        assert_eq!(log_text(&bought), "You buy lamp for 8 coins. (0 remain.)");
        assert_eq!(bought.snapshot.player.coins, 0);
        assert_eq!(
            bought.snapshot.pack.len(),
            1,
            "the lamp moved to the pack exactly once"
        );
        assert!(
            !engine
                .world
                .entities
                .get("keeper")
                .expect("vendor")
                .inventory
                .contains(&"lamp".to_string()),
            "the lamp left the stock exactly once"
        );
    }

    // C-T3 (REQ-003): the remaining buy refusal arms, exact and stateless.
    #[test]
    fn buy_refusals_change_nothing() {
        let mut engine = commerce_engine();
        engine.state.player.coins = 50;

        let unstocked = engine.handle_command(cmd("buy moonbeam"));
        assert!(!unstocked.accepted);
        assert_eq!(system_line(&unstocked), "Keeper does not have 'moonbeam'.");

        let priceless = engine.handle_command(cmd("buy relic"));
        assert!(!priceless.accepted);
        assert_eq!(system_line(&priceless), "Keeper won't part with relic.");

        engine.handle_command(cmd("attack stray"));
        let fighting = engine.handle_command(cmd("buy lamp"));
        assert!(!fighting.accepted);
        assert_eq!(
            system_line(&fighting),
            "There is no trading in the midst of battle."
        );

        let mut vendorless = combat_engine(8, 1);
        let nobody = vendorless.handle_command(cmd("buy lamp"));
        assert!(!nobody.accepted);
        assert_eq!(
            system_line(&nobody),
            "There is no shopkeeper here to trade with."
        );
        assert_eq!(
            engine.snapshot().player.coins,
            50,
            "no refusal spent a coin"
        );
    }

    // C-T4 (REQ-004): selling settles exactly once, and the sell-price floor
    // pins value 1 -> 1, value 2 -> 1, value 6 -> 3 (the `/`->`%`/`*` and
    // max-floor mutant killers).
    #[test]
    fn selling_credits_the_floored_half_price() {
        assert_eq!(sell_price(1), 1, "the floor");
        assert_eq!(sell_price(2), 1);
        assert_eq!(sell_price(6), 3);

        let mut engine = commerce_engine();
        assert!(engine.handle_command(cmd("take trinket")).accepted);
        let sold = engine.handle_command(cmd("sell trinket"));
        assert!(sold.accepted);
        assert_eq!(log_text(&sold), "You sell trinket for 1 coins. (1 now.)");
        assert_eq!(sold.snapshot.player.coins, 1);
        assert!(sold.snapshot.pack.is_empty(), "the trinket left the pack");
        assert!(
            engine
                .world
                .entities
                .get("keeper")
                .expect("vendor")
                .inventory
                .contains(&"trinket".to_string()),
            "the trinket joined the stock exactly once"
        );
    }

    // C-T5 (REQ-004): the sell refusal arms — not carried, oath-bound,
    // worthless, ambiguous (inspect #3: the lossy verb refuses ambiguity
    // like `drop`), mid-combat, vendor-less.
    #[test]
    fn sell_refusals_change_nothing() {
        let mut engine = commerce_engine();

        let missing = engine.handle_command(cmd("sell moonbeam"));
        assert!(!missing.accepted);
        assert_eq!(system_line(&missing), "You are not carrying 'moonbeam'.");

        // Oath-bound: craft a flagged, valued item into the pack.
        let mut sigil = item("sigil");
        sigil.value = 9;
        sigil.flags.push("oath".to_string());
        engine.world.items.insert("sigil".to_string(), sigil);
        engine.state.pack.push("sigil".to_string());
        let bound = engine.handle_command(cmd("sell sigil"));
        assert!(!bound.accepted);
        assert_eq!(
            system_line(&bound),
            "sigil is bound to your oath — you cannot sell it."
        );

        // Worthless: a zero-value carried item.
        engine.state.pack.push("relic_copy".to_string());
        let mut relic_copy = item("relic_copy");
        relic_copy.value = 0;
        engine
            .world
            .items
            .insert("relic_copy".to_string(), relic_copy);
        let worthless = engine.handle_command(cmd("sell relic_copy"));
        assert!(!worthless.accepted);
        assert_eq!(system_line(&worthless), "Keeper has no use for relic_copy.");

        // Ambiguous: two carried items sharing an alias refuse the sale.
        let mut fang_a = item("fang_a");
        fang_a.value = 4;
        fang_a.aliases.push("fang".to_string());
        let mut fang_b = item("fang_b");
        fang_b.value = 4;
        fang_b.aliases.push("fang".to_string());
        engine.world.items.insert("fang_a".to_string(), fang_a);
        engine.world.items.insert("fang_b".to_string(), fang_b);
        engine.state.pack.push("fang_a".to_string());
        engine.state.pack.push("fang_b".to_string());
        let ambiguous = engine.handle_command(cmd("sell fang"));
        assert!(!ambiguous.accepted);
        assert_eq!(
            system_line(&ambiguous),
            "More than one carried item matches 'fang'."
        );
        assert_eq!(
            engine.state.pack.len(),
            4,
            "every refusal left the pack whole"
        );
        assert_eq!(engine.snapshot().player.coins, 0, "no refusal paid a coin");
    }

    // C-T5b (inspect #2): a PURCHASED oath objective fulfills the sworn oath
    // exactly as a taken one — acquisition is acquisition.
    #[test]
    fn buying_the_oath_objective_fulfills_it() {
        // The boss-objective fixture authors a "sigil" objective; restock it
        // on a vendor instead (valued, unflagged) — the ransom-the-relic shape.
        let mut world = boss_objective_world(12, 1, 0);
        let mut keeper = entity("keeper", EntityKind::Actor, &["shopkeeper"], &["sigil"]);
        keeper.name = "Keeper".to_string();
        world.entities.insert("keeper".to_string(), keeper);
        world
            .items
            .get_mut("sigil")
            .expect("authored objective")
            .value = 5;
        let start = world.start_room_id.clone();
        world
            .rooms
            .get_mut(&start)
            .expect("start room")
            .entities
            .push("keeper".to_string());
        let mut engine = Engine::try_new(world).expect("valid world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        engine.state.player.coins = 5;
        let bought = engine.handle_command(cmd("buy sigil"));
        assert!(bought.accepted, "{:?}", bought.events);
        assert!(
            bought
                .events
                .iter()
                .any(|e| matches!(&e.kind, GameEventKind::OathFulfilled { .. })),
            "the purchase fulfills the sworn oath: {:?}",
            bought.events
        );
    }

    // C-T6 (REQ-005): the victory coin award and ALL FOUR line-composition
    // arms exact (the coins-only arm needs a crafted profile — no authored
    // content has xp 0, coins > 0).
    #[test]
    fn victory_lines_compose_per_authored_rewards() {
        let line_for = |xp: u64, coins: u64| {
            let mut world = combat_world(4, 1);
            if let Some(profile) = world
                .entities
                .get_mut("stray")
                .expect("fixture stray")
                .combat
                .as_mut()
            {
                profile.xp = xp;
                profile.coins = coins;
            }
            let mut engine = Engine::try_new(world).expect("valid world");
            let won = engine.handle_command(cmd("attack stray")); // 4hp: one strike
            let text = won
                .events
                .iter()
                .find_map(|e| match &e.kind {
                    GameEventKind::CombatEnded { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .expect("the fight ends in one strike");
            (text, engine.snapshot().player.coins)
        };

        let (both, coins) = line_for(5, 4);
        assert_eq!(
            both,
            "You have defeated Stray. Victory! You gain 5 XP and 4 coins."
        );
        assert_eq!(coins, 4, "the award lands in the purse");
        let (xp_only, _) = line_for(5, 0);
        assert_eq!(xp_only, "You have defeated Stray. Victory! You gain 5 XP.");
        let (coins_only, _) = line_for(0, 7);
        assert_eq!(
            coins_only,
            "You have defeated Stray. Victory! You gain 7 coins."
        );
        let (neither, _) = line_for(0, 0);
        assert_eq!(neither, "You have defeated Stray. Victory!");
    }

    // C-T7 (REQ-007): buy+sell mutations round-trip a save byte-identically,
    // and a pre-commerce payload without the new keys loads as coinless.
    #[test]
    fn commerce_state_round_trips_saves() {
        let mut engine = commerce_engine();
        engine.state.player.coins = 8;
        assert!(engine.handle_command(cmd("buy lamp")).accepted);
        assert!(engine.handle_command(cmd("take trinket")).accepted);
        assert!(engine.handle_command(cmd("sell trinket")).accepted);
        let at_save = snapshot_value(&engine);
        let restored = Engine::from_save(engine.save_data()).expect("save loads");
        assert_eq!(snapshot_value(&restored), at_save, "byte-identical restore");
        assert_eq!(restored.snapshot().player.coins, 1);
        assert!(
            restored
                .world
                .entities
                .get("keeper")
                .expect("vendor")
                .inventory
                .contains(&"trinket".to_string()),
            "the stock mutation persisted"
        );

        // Old payload: strip the new keys from the serialized save.
        let mut payload = serde_json::to_value(commerce_engine().save_data()).expect("serializes");
        let stripped = payload["state"]["player"]
            .as_object_mut()
            .expect("player object")
            .remove("coins");
        assert!(stripped.is_some(), "the key existed to strip");
        let data: SaveData = serde_json::from_value(payload).expect("old payload deserializes");
        let old = Engine::from_save(data).expect("old save loads");
        assert_eq!(old.snapshot().player.coins, 0, "coinless default");
    }

    // C-T8 (REQ-007): crafted extremes through every new arithmetic site —
    // award, credit, and the buy guard all saturate, never panic.
    #[test]
    fn crafted_coin_extremes_never_panic() {
        // Victory award at a full purse saturates.
        let mut world = combat_world(4, 1);
        if let Some(profile) = world
            .entities
            .get_mut("stray")
            .expect("fixture stray")
            .combat
            .as_mut()
        {
            profile.coins = u64::MAX;
        }
        let mut engine = Engine::try_new(world).expect("valid world");
        engine.state.player.coins = 2;
        engine.handle_command(cmd("attack stray"));
        assert_eq!(engine.snapshot().player.coins, u64::MAX, "saturated award");

        // Sell credit at a full purse saturates; the extreme value's price.
        let mut engine = commerce_engine();
        engine
            .world
            .items
            .get_mut("trinket")
            .expect("authored item")
            .value = u64::MAX;
        engine.state.player.coins = u64::MAX;
        assert!(engine.handle_command(cmd("take trinket")).accepted);
        let sold = engine.handle_command(cmd("sell trinket"));
        assert!(sold.accepted);
        assert_eq!(sold.snapshot.player.coins, u64::MAX, "saturated credit");

        // Buying at an extreme price with an exactly-extreme purse.
        let mut engine = commerce_engine();
        engine
            .world
            .items
            .get_mut("lamp")
            .expect("authored item")
            .value = u64::MAX;
        engine.state.player.coins = u64::MAX;
        let bought = engine.handle_command(cmd("buy lamp"));
        assert!(bought.accepted);
        assert_eq!(bought.snapshot.player.coins, 0, "MAX - MAX, no panic");
    }

    // ---- ticket #35: equipment ----

    /// A piece of authored gear for the fixtures below.
    const fn gear(slot: EquipSlot, attack: u32, defense: u32) -> EquipmentProfile {
        EquipmentProfile {
            slot,
            attack,
            defense,
        }
    }

    /// The combat world plus a wardrobe: weapons `fang` (+1) and `blade`
    /// (+2), armor `coat` (−1), `plate` (−2), `aegis` (−3), the plain `rock`,
    /// and the `keeper` vendor (so trade arms are reachable). The stray's
    /// stats come from the caller like `combat_world`.
    fn gear_world(stray_health: u32, stray_attack: u32) -> WorldDefinition {
        let mut world = combat_world(stray_health, stray_attack);
        for (id, profile, value) in [
            ("fang", Some(gear(EquipSlot::Weapon, 1, 0)), 6),
            ("blade", Some(gear(EquipSlot::Weapon, 2, 0)), 6),
            ("coat", Some(gear(EquipSlot::Armor, 0, 1)), 4),
            ("plate", Some(gear(EquipSlot::Armor, 0, 2)), 4),
            ("aegis", Some(gear(EquipSlot::Armor, 0, 3)), 4),
            ("rock", None, 0),
        ] {
            let mut piece = item(id);
            piece.equipment = profile;
            piece.value = value;
            world.items.insert(id.to_string(), piece);
        }
        let mut keeper = entity("keeper", EntityKind::Actor, &["shopkeeper"], &[]);
        keeper.name = "Keeper".to_string();
        world.entities.insert("keeper".to_string(), keeper);
        world
            .rooms
            .get_mut("field")
            .expect("fixture room")
            .entities
            .push("keeper".to_string());
        world
    }

    /// An engine in the gear world carrying `ids`, fighting nothing.
    fn gear_engine(ids: &[&str]) -> Engine {
        let mut engine = Engine::try_new(gear_world(9, 3)).expect("valid gear world");
        for id in ids {
            engine.state.pack.push((*id).to_string());
        }
        engine
    }

    // G-T1 (REQ-001): equip moves the item pack → slot, narrates per slot
    // kind, and the snapshot lists it under its semantic slot name.
    #[test]
    fn equip_moves_carried_gear_into_its_slot() {
        let mut engine = gear_engine(&["fang", "coat"]);
        let wielded = engine.handle_command(cmd("equip fang"));
        assert!(wielded.accepted);
        assert_eq!(log_text(&wielded), "You wield the fang.");
        assert_eq!(engine.state.player.equipped_weapon.as_deref(), Some("fang"));

        let worn = engine.handle_command(cmd("wear coat"));
        assert!(worn.accepted);
        assert_eq!(log_text(&worn), "You wear the coat.");
        assert_eq!(engine.state.player.equipped_armor.as_deref(), Some("coat"));

        assert!(engine.state.pack.is_empty(), "both moved out of the pack");
        let equipment = engine.snapshot().player.equipment;
        let listed: Vec<(String, String, String)> = equipment
            .iter()
            .map(|entry| (entry.slot.clone(), entry.id.clone(), entry.name.clone()))
            .collect();
        assert_eq!(
            listed,
            vec![
                ("weapon".to_string(), "fang".to_string(), "fang".to_string()),
                ("armor".to_string(), "coat".to_string(), "coat".to_string()),
            ],
            "weapon first, slot/id/name exact"
        );
    }

    // G-T1b (REQ-004): the snapshot lists weapon before armor regardless of
    // the order the gear was donned (kills the projection-order mutant).
    #[test]
    fn equipment_snapshot_orders_weapon_first() {
        let mut engine = gear_engine(&["fang", "coat"]);
        assert!(engine.handle_command(cmd("wear coat")).accepted);
        assert!(engine.handle_command(cmd("equip fang")).accepted);
        let slots: Vec<String> = engine
            .snapshot()
            .player
            .equipment
            .iter()
            .map(|entry| entry.slot.clone())
            .collect();
        assert_eq!(slots, vec!["weapon".to_string(), "armor".to_string()]);
    }

    // G-T2 (REQ-001): equipping onto an occupied slot swaps — the prior
    // occupant returns to the pack and the line names both.
    #[test]
    fn equip_swaps_the_occupied_slot_back_into_the_pack() {
        let mut engine = gear_engine(&["fang", "blade"]);
        assert!(engine.handle_command(cmd("equip fang")).accepted);
        let swapped = engine.handle_command(cmd("equip blade"));
        assert!(swapped.accepted);
        assert_eq!(log_text(&swapped), "You wield the blade, stowing the fang.");
        assert_eq!(
            engine.state.player.equipped_weapon.as_deref(),
            Some("blade")
        );
        assert_eq!(engine.state.pack, vec!["fang".to_string()]);
    }

    // G-T3a (REQ-002): both gear verbs refuse mid-combat with no state change.
    #[test]
    fn gear_changes_refuse_mid_combat() {
        let mut engine = gear_engine(&["fang"]);
        engine.handle_command(cmd("attack stray"));
        let equip = engine.handle_command(cmd("equip fang"));
        assert!(!equip.accepted);
        assert_eq!(
            system_line(&equip),
            "There is no changing gear in the midst of battle."
        );
        assert_eq!(engine.state.player.equipped_weapon, None);
        assert_eq!(engine.state.pack, vec!["fang".to_string()]);

        let unequip = engine.handle_command(cmd("unequip weapon"));
        assert!(!unequip.accepted);
        assert_eq!(
            system_line(&unequip),
            "There is no changing gear in the midst of battle."
        );
    }

    // G-T3b (REQ-002): the not-carried, not-equipment, and ambiguous arms,
    // each with zero state change.
    #[test]
    fn equip_refuses_missing_plain_and_ambiguous_targets() {
        let mut engine = gear_engine(&["rock", "fang", "fang"]);
        let missing = engine.handle_command(cmd("equip torch"));
        assert!(!missing.accepted);
        assert_eq!(system_line(&missing), "You are not carrying 'torch'.");

        let plain = engine.handle_command(cmd("equip rock"));
        assert!(!plain.accepted);
        assert_eq!(system_line(&plain), "rock is not something you can equip.");

        let ambiguous = engine.handle_command(cmd("equip fang"));
        assert!(!ambiguous.accepted);
        assert_eq!(
            system_line(&ambiguous),
            "More than one carried item matches 'fang'."
        );
        assert_eq!(
            engine.state.pack,
            vec!["rock".to_string(), "fang".to_string(), "fang".to_string()],
            "every refusal left the pack untouched"
        );
        assert_eq!(engine.state.player.equipped_weapon, None);
    }

    // G-T3c (REQ-002, inspect #2): an already-equipped target gets the honest
    // arm — not the "not carrying" denial.
    #[test]
    fn equip_names_an_already_equipped_target_honestly() {
        let mut engine = gear_engine(&["fang"]);
        assert!(engine.handle_command(cmd("equip fang")).accepted);
        let again = engine.handle_command(cmd("equip fang"));
        assert!(!again.accepted);
        assert_eq!(system_line(&again), "The fang is already equipped.");
    }

    // G-T3d (REQ-002): unequip's empty-slot arms, one per slot keyword
    // (kills the EquipSlot::as_str arm mutants).
    #[test]
    fn unequip_refuses_empty_slots_by_name() {
        let mut engine = gear_engine(&[]);
        let weapon = engine.handle_command(cmd("unequip weapon"));
        assert!(!weapon.accepted);
        assert_eq!(system_line(&weapon), "Nothing is equipped as your weapon.");
        let armor = engine.handle_command(cmd("unequip armor"));
        assert!(!armor.accepted);
        assert_eq!(system_line(&armor), "Nothing is equipped as your armor.");
    }

    // G-T3e (REQ-002): unequip's no-match and ambiguous arms.
    #[test]
    fn unequip_refuses_unknown_and_ambiguous_gear() {
        let mut engine = gear_engine(&["fang", "coat"]);
        assert!(engine.handle_command(cmd("equip fang")).accepted);
        assert!(engine.handle_command(cmd("wear coat")).accepted);

        let unknown = engine.handle_command(cmd("unequip banana"));
        assert!(!unknown.accepted);
        assert_eq!(
            system_line(&unknown),
            "You have nothing like 'banana' equipped."
        );

        // Both equipped pieces share an alias → ambiguous, nothing taken off.
        for id in ["fang", "coat"] {
            engine
                .world
                .items
                .get_mut(id)
                .expect("fixture item")
                .aliases
                .push("kit".to_string());
        }
        let ambiguous = engine.handle_command(cmd("unequip kit"));
        assert!(!ambiguous.accepted);
        assert_eq!(
            system_line(&ambiguous),
            "More than one equipped item matches 'kit'."
        );
        assert_eq!(engine.state.player.equipped_weapon.as_deref(), Some("fang"));
        assert_eq!(engine.state.player.equipped_armor.as_deref(), Some("coat"));
    }

    // G-T3f (REQ-001): unequip succeeds by slot keyword and by item name,
    // returning the piece to the pack.
    #[test]
    fn unequip_frees_gear_by_slot_or_name() {
        let mut engine = gear_engine(&["fang", "coat"]);
        assert!(engine.handle_command(cmd("equip fang")).accepted);
        assert!(engine.handle_command(cmd("wear coat")).accepted);

        let by_name = engine.handle_command(cmd("unequip fang"));
        assert!(by_name.accepted);
        assert_eq!(log_text(&by_name), "You unequip the fang.");
        assert_eq!(engine.state.player.equipped_weapon, None);

        let by_slot = engine.handle_command(cmd("unequip armor"));
        assert!(by_slot.accepted);
        assert_eq!(log_text(&by_slot), "You unequip the coat.");
        assert_eq!(engine.state.player.equipped_armor, None);
        assert_eq!(
            engine.state.pack,
            vec!["fang".to_string(), "coat".to_string()]
        );
    }

    // G-T3g (REQ-002, inspect #2): drop and sell name the real reason for an
    // equipped item instead of denying possession.
    #[test]
    fn drop_and_sell_refuse_equipped_gear_honestly() {
        let mut engine = gear_engine(&["fang"]);
        assert!(engine.handle_command(cmd("equip fang")).accepted);

        let dropped = engine.handle_command(cmd("drop fang"));
        assert!(!dropped.accepted);
        assert_eq!(
            log_text(&dropped),
            "The fang is equipped — unequip it before dropping it."
        );

        let sold = engine.handle_command(cmd("sell fang"));
        assert!(!sold.accepted);
        assert_eq!(
            system_line(&sold),
            "The fang is equipped — unequip it before selling."
        );
        assert_eq!(engine.state.player.equipped_weapon.as_deref(), Some("fang"));
    }

    // G-T4 (REQ-003): the weapon bonus lands on every strike — basic 4/5/6
    // bare/fang/blade (boundary ±1) and the power strike at 6 + mod.
    #[test]
    fn weapon_mods_raise_every_strike_exactly() {
        // Bare hands: 4 (the existing contract, re-pinned here as the base).
        let mut engine = gear_engine(&[]);
        let bare = engine.handle_command(cmd("attack stray"));
        assert_eq!(
            count_lines(&bare.events, "You strike Stray for 4 (5/9)."),
            1
        );

        // Fang (+1): 5.
        let mut engine = gear_engine(&["fang"]);
        assert!(engine.handle_command(cmd("equip fang")).accepted);
        let fanged = engine.handle_command(cmd("attack stray"));
        assert_eq!(
            count_lines(&fanged.events, "You strike Stray for 5 (4/9)."),
            1
        );

        // Blade (+2): 6 — and the queued power strike slams for 6 + 2 = 8.
        let mut engine = Engine::try_new(gear_world(40, 1)).expect("valid gear world");
        engine.state.pack.push("blade".to_string());
        assert!(engine.handle_command(cmd("equip blade")).accepted);
        let bladed = engine.handle_command(cmd("attack stray"));
        assert_eq!(
            count_lines(&bladed.events, "You strike Stray for 6 (34/40)."),
            1
        );
        engine.handle_command(cmd("power strike"));
        let _ = engine.tick();
        let pulse = engine.tick(); // P1 round 2 (strike 6 → 28), then P2 slams 8 → 20
        assert_eq!(
            count_lines(&pulse, "Your power strike slams into Stray for 8 (20/40)."),
            1,
            "power strike carries the weapon mod: {pulse:?}"
        );
    }

    // G-T5 (REQ-003): armor reduces every return by its defense, narrating
    // the DEALT number, with the boundary swept across 1/2/3 vs attack 3.
    #[test]
    fn armor_reduces_incoming_hits_to_the_floor() {
        for (piece, dealt, hp_after) in [("coat", 2, 18), ("plate", 1, 19), ("aegis", 0, 20)] {
            let mut engine = gear_engine(&[piece]);
            assert!(
                engine
                    .handle_command(CommandRequest {
                        input: format!("wear {piece}"),
                        actor_id: None,
                    })
                    .accepted
            );
            let round = engine.handle_command(cmd("attack stray"));
            let line = format!("Stray hits you for {dealt} ({hp_after}/20).");
            assert_eq!(
                count_lines(&round.events, &line),
                1,
                "defense sweep for {piece}: {:?}",
                round.events
            );
            assert_eq!(engine.state.player.hp, hp_after);
        }
    }

    // G-T9 (REQ-003/005): crafted extremes — a u32::MAX mod saturates instead
    // of overflowing, and cross-slot crafted gear pays nothing.
    #[test]
    fn crafted_gear_extremes_never_panic_and_wrong_slots_pay_zero() {
        // MAX attack: the strike floors the enemy without panicking.
        let mut engine = gear_engine(&["blade"]);
        engine
            .world
            .items
            .get_mut("blade")
            .expect("fixture item")
            .equipment = Some(gear(EquipSlot::Weapon, u32::MAX, 0));
        assert!(engine.handle_command(cmd("equip blade")).accepted);
        let strike = engine.handle_command(cmd("attack stray"));
        assert!(strike.accepted);
        assert!(engine.state.combat.is_none(), "one saturated blow ended it");

        // MAX defense: every return deals exactly 0.
        let mut engine = gear_engine(&["coat"]);
        engine
            .world
            .items
            .get_mut("coat")
            .expect("fixture item")
            .equipment = Some(gear(EquipSlot::Armor, 0, u32::MAX));
        assert!(engine.handle_command(cmd("wear coat")).accepted);
        let round = engine.handle_command(cmd("attack stray"));
        assert_eq!(
            count_lines(&round.events, "Stray hits you for 0 (20/20)."),
            1
        );

        // Cross-slot crafted state: an armor profile parked in the weapon
        // slot (and vice versa) pays 0 — a slot only pays its own stat.
        let mut engine = gear_engine(&[]);
        engine.state.player.equipped_weapon = Some("aegis".to_string());
        engine.state.player.equipped_armor = Some("blade".to_string());
        let round = engine.handle_command(cmd("attack stray"));
        assert_eq!(
            count_lines(&round.events, "You strike Stray for 4 (5/9)."),
            1,
            "armor-in-weapon-slot adds nothing"
        );
        assert_eq!(
            count_lines(&round.events, "Stray hits you for 3 (17/20)."),
            1,
            "weapon-in-armor-slot blocks nothing"
        );
    }

    // G-T8 (REQ-005): the save round-trip preserves both slots; a legacy
    // payload without the keys loads bare-handed; and the crafted
    // pack+slot duplicate is CONSERVED by unequip (inspect #1 — the dedup
    // guard that destroyed a copy is gone).
    #[test]
    fn equipment_state_round_trips_saves() {
        let mut engine = gear_engine(&["fang", "coat"]);
        assert!(engine.handle_command(cmd("equip fang")).accepted);
        assert!(engine.handle_command(cmd("wear coat")).accepted);
        let saved = engine.save_data();
        let loaded = Engine::from_save(saved).expect("round-trip loads");
        assert_eq!(loaded.state.player.equipped_weapon.as_deref(), Some("fang"));
        assert_eq!(loaded.state.player.equipped_armor.as_deref(), Some("coat"));

        // Legacy payload: strip the equipment keys entirely (the #34 coins
        // pattern) — a pre-#35 save loads with empty hands.
        let mut value = serde_json::to_value(engine.save_data()).expect("save serializes to JSON");
        let player = value
            .get_mut("state")
            .and_then(|state| state.get_mut("player"))
            .and_then(serde_json::Value::as_object_mut)
            .expect("player object");
        player.remove("equipped_weapon");
        player.remove("equipped_armor");
        let legacy: SaveData = serde_json::from_value(value).expect("legacy payload parses");
        let loaded = Engine::from_save(legacy).expect("legacy payload loads");
        assert_eq!(loaded.state.player.equipped_weapon, None);
        assert_eq!(loaded.state.player.equipped_armor, None);

        // Crafted duplicate: the id in the pack AND the slot means two
        // copies; unequip must conserve both.
        let mut engine = gear_engine(&["fang"]);
        engine.state.player.equipped_weapon = Some("fang".to_string());
        let freed = engine.handle_command(cmd("unequip weapon"));
        assert!(freed.accepted);
        assert_eq!(
            engine.state.pack,
            vec!["fang".to_string(), "fang".to_string()],
            "both copies survive"
        );
    }

    // G-T10 (REQ-004, inspect #2): the text projections keep seeing worn
    // gear — look falls back to equipped, and the inventory appends the
    // Equipped clause (including over an empty pack).
    #[test]
    fn text_projections_keep_seeing_equipped_gear() {
        let mut engine = gear_engine(&["rock", "fang", "coat"]);
        assert!(engine.handle_command(cmd("equip fang")).accepted);
        assert!(engine.handle_command(cmd("wear coat")).accepted);

        let looked = engine.handle_command(cmd("look fang"));
        assert_eq!(
            log_text(&looked),
            "You look over the fang you have equipped. d"
        );

        let pack = engine.handle_command(cmd("inventory"));
        assert_eq!(
            log_text(&pack),
            "You are carrying: rock. Equipped: fang, coat."
        );

        engine.handle_command(cmd("drop rock"));
        let bare_pack = engine.handle_command(cmd("inventory"));
        assert_eq!(
            log_text(&bare_pack),
            "You are carrying nothing. Equipped: fang, coat."
        );

        // Unequipped, the carried fallback resumes.
        assert!(engine.handle_command(cmd("unequip fang")).accepted);
        let carried = engine.handle_command(cmd("look fang"));
        assert_eq!(
            log_text(&carried),
            "You examine the fang you are carrying. d"
        );
    }

    // G-T11 (REQ-006 composition): gear trades through the existing shop —
    // buying a blade stocks the pack, equipping it arms the slot.
    #[test]
    fn bought_gear_equips_like_any_carried_item() {
        let mut engine = gear_engine(&[]);
        engine
            .world
            .entities
            .get_mut("keeper")
            .expect("vendor")
            .inventory
            .push("blade".to_string());
        engine.state.player.coins = 6;
        assert!(engine.handle_command(cmd("buy blade")).accepted);
        assert_eq!(engine.state.player.coins, 0);
        let wielded = engine.handle_command(cmd("equip blade"));
        assert!(wielded.accepted);
        assert_eq!(
            engine.state.player.equipped_weapon.as_deref(),
            Some("blade")
        );
    }
}
