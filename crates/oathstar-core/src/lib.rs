pub mod awareness;
pub mod command;

use std::collections::{BTreeMap, BTreeSet};

use awareness::{AwarenessKind, RadiusConfig};
use command::{parse, Command, Direction};
use oathstar_protocol::{
    CombatOutcome, CombatSnapshot, CombatantSnapshot, CommandRequest, CommandResponse,
    EventChannel, GameEvent, GameEventKind, GameSnapshot, MapRoomSnapshot, MapSnapshot,
    NearbySnapshot, NearbyStatsSnapshot, NearbyThreatSnapshot, OathSnapshot, OathStatus,
    OutputComponent, PackItemSnapshot, PlayerSnapshot, RoomSnapshot,
};
use serde::{Deserialize, Serialize};

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
    /// Future-combat-ready stats for a `combatant` (ticket #21). Optional and
    /// authored; no combat system reads it yet (combat AI is out of scope), so the
    /// `combatant` contract does not require it — it is the forward hook for combat.
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
    /// A `confront` endpoint (tag `"boss"`).
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
}

/// Damage the player deals per strike (ticket #22). Fixed and deterministic (no
/// RNG) so combat stays fully testable and mutation-killable; tunable in a later
/// balance pass.
const PLAYER_STRIKE_DAMAGE: i32 = 4;

/// World ticks between combat pulses (ticket #24, Decision 023): with the 1s
/// world tick this is the ~2s default combat pulse. Copied onto each encounter
/// at start, so per-actor variation later is a one-line copy from the profile;
/// v2 ships the single default.
const DEFAULT_COMBAT_PULSE_TICKS: u64 = 2;

/// Damage a queued power strike deals in the Phase-2 skill window (ticket
/// #25). Heavier than the baseline strike, and just as fixed/deterministic.
const POWER_STRIKE_DAMAGE: i32 = 6;

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
    /// These three fields are plain (no `#[serde(default)]`): mid-encounter
    /// state is never persisted (`oathstar-storage` validates slot names only),
    /// and a defaulted `pulse_rate` of 0 would mean "pulse every tick" — a
    /// wrong-by-default worse than rejecting a payload shape that never exists.
    pub queued_action: Option<CombatAction>,
    /// A one-shot guard charge (ticket #25): set when a queued guard resolves
    /// in the Phase-2 window, consumed by the next enemy return strike from
    /// any source (pulse Phase 1 or a manual round), which it turns aside
    /// entirely. Plain like its siblings above — never persisted, and it dies
    /// with the encounter state on every combat end.
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
}

/// A hostile resolved as the target of `attack`, with its authored stats copied
/// out so [`Engine::start_combat`] can build a self-contained [`CombatState`].
struct ResolvedHostile {
    id: String,
    name: String,
    health: u32,
    attack: u32,
}

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
        self.state.tick += 1;
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
        let round = combat.round + 1;
        events.push(self.event(EventChannel::Combat, GameEventKind::CombatPulse { round }));
        self.resolve_combat_round(events);
        self.resolve_queued_action(events);
        if let Some(combat) = self.state.combat.as_mut() {
            combat.next_pulse_at = self.state.tick + combat.pulse_rate;
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
        let (line, enemy_dead) = {
            let combat = self
                .state
                .combat
                .as_mut()
                .expect("resolve_power_strike is only called with an active encounter");
            combat.enemy_hp = combat.enemy_hp.saturating_sub(POWER_STRIKE_DAMAGE).max(0);
            let line = format!(
                "Your power strike slams into {} for {POWER_STRIKE_DAMAGE} ({}/{}).",
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
                    "Try: look, north, south, east, west, up, down, swear, confront, attack, flee, guard, power strike, talk, take, drop, inventory.",
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
            Command::Swear => {
                let (accepted, swear_events) = self.swear();
                events.extend(swear_events);
                return self.response(accepted, events);
            }
            Command::Confront => {
                let (accepted, confront_events) = self.confront();
                events.extend(confront_events);
                return self.response(accepted, events);
            }
            Command::Attack { target } => {
                let (accepted, attack_events) = self.attack(target.as_deref());
                events.extend(attack_events);
                return self.response(accepted, events);
            }
            Command::Flee => {
                let (accepted, flee_events) = self.queue_combat_action(CombatAction::Flee);
                events.extend(flee_events);
                return self.response(accepted, events);
            }
            Command::Guard => {
                let (accepted, guard_events) = self.queue_combat_action(CombatAction::Guard);
                events.extend(guard_events);
                return self.response(accepted, events);
            }
            Command::PowerStrike => {
                let (accepted, strike_events) = self.queue_combat_action(CombatAction::PowerStrike);
                events.extend(strike_events);
                return self.response(accepted, events);
            }
            Command::Talk { target } => {
                let (accepted, talk_events) = self.talk_at(&target);
                events.extend(talk_events);
                return self.response(accepted, events);
            }
            Command::Take { target } => {
                let (accepted, take_events) = self.take_at(&target);
                events.extend(take_events);
                return self.response(accepted, events);
            }
            Command::Drop { target } => {
                let (accepted, drop_events) = self.drop_at(&target);
                events.extend(drop_events);
                return self.response(accepted, events);
            }
            Command::Inventory => {
                events.extend(self.list_pack());
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
            // Not nearby — fall back to the carried pack (ticket #20): a carried
            // item has no cell, so it resolves from inventory (REQ-004).
            None => self.find_in_pack(target).map_or_else(
                || format!("You see nothing like '{target}' nearby."),
                |item| {
                    format!(
                        "You examine the {} you are carrying. {}",
                        item.name, item.description
                    )
                },
            ),
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
        let (accepted, channel, component, text) =
            match awareness::resolve_target(&self.world, &origin, &radii, target) {
                None => (
                    false,
                    EventChannel::Narrative,
                    OutputComponent::NarrativeMessage,
                    format!("You see nothing like '{target}' here to take."),
                ),
                Some(found) if found.kind != AwarenessKind::Item => (
                    false,
                    EventChannel::Narrative,
                    OutputComponent::NarrativeMessage,
                    format!("You cannot carry {}.", found.name),
                ),
                Some(found) if !found.proximity.is_interactable() => (
                    false,
                    EventChannel::Narrative,
                    OutputComponent::NarrativeMessage,
                    format!("{} is too far away to reach.", found.name),
                ),
                Some(found) => {
                    // Reachable world item: drop it from the exact placing room
                    // (defensive `get_mut` — `room_id` came from the resolver, so
                    // the room exists) and carry it. perceive() then excludes it.
                    if let Some(room) = self.world.rooms.get_mut(&found.room_id) {
                        room.items.retain(|item_id| item_id != &found.id);
                    }
                    let name = found.name.clone();
                    self.state.pack.push(found.id);
                    (
                        true,
                        EventChannel::Inventory,
                        OutputComponent::ItemCard,
                        format!("You take the {name}."),
                    )
                }
            };
        (accepted, vec![self.log(channel, component, text)])
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
    /// REQ-003). Always accepted — a readout of `state.pack`.
    fn list_pack(&mut self) -> Vec<GameEvent> {
        let names: Vec<String> = self
            .state
            .pack
            .iter()
            .map(|item_id| {
                self.world
                    .items
                    .get(item_id)
                    .map_or_else(|| item_id.clone(), |item| item.name.clone())
            })
            .collect();
        let text = if names.is_empty() {
            "You are carrying nothing.".to_string()
        } else {
            format!("You are carrying: {}.", names.join(", "))
        };
        vec![self.log(
            EventChannel::Inventory,
            OutputComponent::SystemMessage,
            text,
        )]
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

    /// Resolve the boss at the current room's endpoint.
    ///
    /// Returns `(accepted, events)`. The boss is the placed entity carrying the
    /// `"boss"` role. Refused (no state change) when there is no boss here, when
    /// no oath is active, or when the oath is already fulfilled. On success, emits
    /// the authored combat outcome plus a typed [`GameEventKind::OathFulfilled`]
    /// and marks the oath fulfilled. Deterministic — no RNG.
    fn confront(&mut self) -> (bool, Vec<GameEvent>) {
        let room = self.current_room().clone();
        let boss_name = room
            .entities
            .iter()
            .filter_map(|entity_id| self.world.entities.get(entity_id))
            .find(|entity| entity.has_role(Role::Boss))
            .map(|boss| boss.name.clone());

        let Some(boss_name) = boss_name else {
            return (
                false,
                vec![self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    "There is nothing here to confront.",
                )],
            );
        };

        // Flip the oath to fulfilled only when it is active, holding the mutable
        // borrow just long enough to copy the id out so the event helpers below
        // can re-borrow `self`.
        let resolved_oath_id = match self.state.oath.as_mut() {
            Some(progress) if progress.status == OathStatus::Sworn => {
                progress.status = OathStatus::Fulfilled;
                Some(progress.oath_id.clone())
            }
            _ => None,
        };

        if let Some(oath_id) = resolved_oath_id {
            let outcome = self.log(
                EventChannel::Combat,
                OutputComponent::CombatMessage,
                format!("You overcome {boss_name}, and your oath is fulfilled."),
            );
            let fulfilled =
                self.event(EventChannel::Oath, GameEventKind::OathFulfilled { oath_id });
            return (true, vec![outcome, fulfilled]);
        }

        // Boss present, but the oath is not swearable-to-fulfilled here: either it
        // was never sworn, or it is already fulfilled (a `Sworn` oath would have
        // resolved above). Distinguish for the refusal message.
        let message = match self.state.oath.as_ref().map(|progress| progress.status) {
            Some(OathStatus::Fulfilled) => {
                format!("{boss_name} is already broken; your oath is kept.")
            }
            _ => format!("You face {boss_name}, but you have sworn no oath to see this through."),
        };
        (
            false,
            vec![self.log(
                EventChannel::System,
                OutputComponent::SystemMessage,
                message,
            )],
        )
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
            Some(_) => {
                combat.queued_action = Some(action);
                format!("You change tack. {}", action.queue_confirmation())
            }
            None => {
                combat.queued_action = Some(action);
                action.queue_confirmation().to_string()
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

        let enemy_max_hp = i32::try_from(hostile.health).unwrap_or(i32::MAX);
        let enemy_attack = i32::try_from(hostile.attack).unwrap_or(i32::MAX);
        self.state.combat = Some(CombatState {
            enemy_id: hostile.id.clone(),
            enemy_name: hostile.name.clone(),
            enemy_hp: enemy_max_hp,
            enemy_max_hp,
            enemy_attack,
            round: 0,
            log: Vec::new(),
            pulse_rate: DEFAULT_COMBAT_PULSE_TICKS,
            next_pulse_at: self.state.tick + DEFAULT_COMBAT_PULSE_TICKS,
            queued_action: None,
            guard_charge: false,
        });

        let mut events = vec![self.event(
            EventChannel::Combat,
            GameEventKind::CombatStarted {
                enemy_id: hostile.id,
                enemy_name: hostile.name.clone(),
                text: format!("{} turns on you. Steel yourself.", hostile.name),
            },
        )];
        self.resolve_combat_round(&mut events);
        (true, events)
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
        let damage = PLAYER_STRIKE_DAMAGE;
        let (player_line, enemy_dead, enemy_name, enemy_attack) = {
            let combat = self
                .state
                .combat
                .as_mut()
                .expect("resolve_combat_round is only called with an active encounter");
            combat.round += 1;
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

        self.state.player.hp = self.state.player.hp.saturating_sub(enemy_attack).max(0);
        let player_hp = self.state.player.hp;
        let player_max_hp = self.state.player.max_hp;
        let enemy_line =
            format!("{enemy_name} hits you for {enemy_attack} ({player_hp}/{player_max_hp}).");
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
        let enemy_name = combat.enemy_name;
        let text = match outcome {
            CombatOutcome::Victory => {
                self.remove_entity_everywhere(&combat.enemy_id);
                self.drop_enemy_inventory(&combat.enemy_id, &enemy_name, events);
                let xp = self.enemy_xp_reward(&combat.enemy_id);
                // Saturating like the defeat penalty: a u64 award can never
                // overflow-panic, however absurd the authored numbers get.
                self.state.player.xp = self.state.player.xp.saturating_add(xp);
                if xp > 0 {
                    format!("You have defeated {enemy_name}. Victory! You gain {xp} XP.")
                } else {
                    // Byte-identical to the pre-#26 victory line, so an
                    // unrewarded win behaves exactly as before (REQ-002).
                    format!("You have defeated {enemy_name}. Victory!")
                }
            }
            CombatOutcome::Defeat => {
                self.state.player.hp = self.state.player.max_hp;
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
            // enemy survives in place and the player keeps their current HP.
            CombatOutcome::Fled => format!("You break away from {enemy_name} and escape."),
        };
        events.push(self.event(
            EventChannel::Combat,
            GameEventKind::CombatEnded { outcome, text },
        ));
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
        for item_id in &dropped {
            let name = &self
                .world
                .items
                .get(item_id)
                .expect("entity inventory ids resolve in a validated world (EntityItemMissing)")
                .name;
            let line = format!("The {enemy_name} drops {name}.");
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
    /// zero XP nothing is lost. Returns the XP lost.
    fn apply_defeat_penalty(&mut self) -> u64 {
        let xp = self.state.player.xp;
        if xp == 0 {
            return 0;
        }
        let penalty = (xp / 10).max(1);
        self.state.player.xp = xp.saturating_sub(penalty);
        penalty
    }

    /// Remove an entity's room placement everywhere it appears (ticket #22), so a
    /// defeated enemy leaves no corpse to re-fight. Mirrors `take_at`'s removal of a
    /// taken item from its room.
    fn remove_entity_everywhere(&mut self, entity_id: &str) {
        for room in self.world.rooms.values_mut() {
            room.entities.retain(|id| id != entity_id);
        }
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
        let rooms = self
            .world
            .rooms
            .values()
            .map(|map_room| MapRoomSnapshot {
                id: map_room.id.clone(),
                title: map_room.title.clone(),
                x: map_room.x,
                y: map_room.y,
                z: map_room.z,
                glyph: map_room.glyph,
                passable: map_room.passable,
                discovered: self.state.discovered_rooms.contains(&map_room.id),
                current: map_room.id == self.state.current_room_id,
                exits: map_room.exits.clone(),
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
        self.next_event_id += 1;
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

    // REQ-004: confronting the boss with an active oath resolves it — Combat
    // outcome naming the boss + typed OathFulfilled, oath becomes Fulfilled.
    #[test]
    fn confront_fulfills_active_oath_and_emits_oath_fulfilled() {
        let mut engine = Engine::try_new(oath_world()).expect("valid oath world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        assert!(
            engine.handle_command(cmd("east")).accepted,
            "move to the lair"
        );

        let response = engine.handle_command(cmd("confront"));
        assert!(
            response.accepted,
            "confront is accepted with boss + active oath"
        );

        assert!(
            response.events.iter().any(|e| matches!(
                (&e.channel, &e.kind),
                (
                    EventChannel::Combat,
                    GameEventKind::LogMessage {
                        component: OutputComponent::CombatMessage,
                        text,
                    }
                ) if text.contains("The Warden")
            )),
            "emits a Combat outcome naming the boss"
        );
        assert!(
            response.events.iter().any(|e| matches!(
                (&e.channel, &e.kind),
                (EventChannel::Oath, GameEventKind::OathFulfilled { oath_id })
                    if oath_id.as_str() == "o1"
            )),
            "emits OathFulfilled{{o1}} on the Oath channel"
        );
        assert_eq!(
            response.snapshot.oath.expect("oath present").status,
            OathStatus::Fulfilled
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

    // REQ-004 branch: confronting after the oath is already fulfilled is refused.
    #[test]
    fn confront_when_oath_already_fulfilled_is_refused() {
        let mut engine = Engine::try_new(oath_world()).expect("valid oath world");
        assert!(engine.handle_command(cmd("swear")).accepted);
        assert!(engine.handle_command(cmd("east")).accepted);
        assert!(
            engine.handle_command(cmd("confront")).accepted,
            "first confront fulfills the oath"
        );
        let response = engine.handle_command(cmd("confront"));
        assert!(
            !response.accepted,
            "confronting an already-broken boss is refused"
        );
        assert!(
            response.events.iter().any(|e| matches!(
                &e.kind,
                GameEventKind::LogMessage { text, .. } if text.contains("already broken")
            )),
            "refusal says the boss is already broken"
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
        world.entities.insert(
            "warden".to_string(),
            entity("warden", EntityKind::Actor, &["boss"], &[]),
        );

        let mut oaths = BTreeMap::new();
        oaths.insert(
            "hollow_bell".to_string(),
            OathDefinition {
                id: "hollow_bell".to_string(),
                title: "The Hollow Bell".to_string(),
                description: "Mend the bell.".to_string(),
                issuer_id: Some("mara".to_string()),
                source: Some("hollowmere".to_string()),
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
        assert!(
            engine.handle_command(cmd("confront")).accepted,
            "the boss is present and the oath is sworn"
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
        });
        world.entities.insert("stray".to_string(), stray);

        let mut brute = entity("brute", EntityKind::Actor, &["combatant", "hostile"], &[]);
        brute.name = "Brute".to_string();
        brute.combat = Some(CombatProfile {
            health: 99,
            attack: 99,
            disclose_stats: false,
            xp: 0,
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
        });
        world.entities.insert("sage".to_string(), sage);

        let mut wolf = entity("wolf", EntityKind::Actor, &["combatant", "hostile"], &[]);
        wolf.name = "Wolf".to_string();
        wolf.combat = Some(CombatProfile {
            health: 5,
            attack: 1,
            disclose_stats: false,
            xp: 0,
        });
        world.entities.insert("wolf".to_string(), wolf);

        let mut brute = entity("brute", EntityKind::Actor, &["combatant", "hostile"], &[]);
        brute.name = "Brute".to_string();
        brute.combat = Some(CombatProfile {
            health: 6,
            attack: 2,
            disclose_stats: false,
            xp: 0,
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
    // (The deliberate #26 rewrite of the #24 revive-in-place pin.)
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
        let snapshot = engine.snapshot();
        assert!(snapshot.combat.is_none());
        assert_eq!(snapshot.player.hp, 20, "HP restored to max");
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
        assert_eq!(response.snapshot.player.hp, 20, "HP restored to max");
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
}
