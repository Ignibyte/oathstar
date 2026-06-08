pub mod awareness;
pub mod command;

use std::collections::{BTreeMap, BTreeSet};

use awareness::{AwarenessKind, RadiusConfig};
use command::{parse, Command, Direction};
use oathstar_protocol::{
    CommandRequest, CommandResponse, EventChannel, GameEvent, GameEventKind, GameSnapshot,
    MapRoomSnapshot, MapSnapshot, NearbySnapshot, OathSnapshot, OathStatus, OutputComponent,
    PackItemSnapshot, PlayerSnapshot, RoomSnapshot,
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
        }
    }

    pub const fn tick(&mut self) -> GameEvent {
        self.state.tick += 1;
        self.event(
            EventChannel::Debug,
            GameEventKind::Tick {
                value: self.state.tick,
            },
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
                    "Try: look, north, south, east, west, up, down, swear, confront, talk, take, drop, inventory.",
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
            return if entity.roles.iter().any(|role| role == "conversable") {
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

        self.state.current_room_id = next_room.id.clone();
        self.state.discovered_rooms.insert(next_room.id.clone());

        let mut events = vec![self.event(
            EventChannel::Room,
            GameEventKind::RoomEntered {
                room_id: next_room.id,
                title: next_room.title,
            },
        )];
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
            .find(|entity| entity.roles.iter().any(|role| role == "boss"))
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
            .map(|thing| NearbySnapshot {
                id: thing.id,
                name: thing.name,
                kind: thing.kind.as_str().to_string(),
                distance: thing.distance,
                proximity: thing.proximity.as_str().to_string(),
                interactable: thing.proximity.is_interactable(),
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
        let event = engine.tick();
        assert!(matches!(event.kind, GameEventKind::Tick { value } if value == 1));
        assert_eq!(engine.snapshot().tick, 1);
    }

    #[test]
    fn event_ids_increment_sequentially() {
        let mut engine = Engine::try_new(test_world()).expect("valid test world");
        let first = engine.tick();
        let second = engine.tick();
        assert_eq!(first.event_id, 1);
        assert_eq!(second.event_id, 2);
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
}
