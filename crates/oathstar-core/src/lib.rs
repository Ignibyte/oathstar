pub mod command;

use std::collections::{BTreeMap, BTreeSet};

use command::{parse, Command, Direction};
use oathstar_protocol::{
    CommandRequest, CommandResponse, EventChannel, GameEvent, GameEventKind, GameSnapshot,
    MapRoomSnapshot, MapSnapshot, OathSnapshot, OathStatus, OutputComponent, PlayerSnapshot,
    RoomSnapshot,
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

        if let Some(oath_id) = &self.oath_id {
            if !self.oaths.contains_key(oath_id) {
                return Err(WorldValidationError::OathMissing {
                    oath_id: oath_id.clone(),
                });
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
                    "Try: look, north, south, east, west, up, down, swear, confront.",
                ));
            }
            Command::Look { target: None } => {
                events.extend(self.describe_current_room());
            }
            Command::Look {
                target: Some(target),
            } => {
                events.push(self.log(
                    EventChannel::Narrative,
                    OutputComponent::NarrativeMessage,
                    format!("You study {target}, but learn nothing new about it yet."),
                ));
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
        }
    }

    fn item(id: &str) -> Item {
        Item {
            id: id.to_string(),
            name: id.to_string(),
            description: "d".to_string(),
            aliases: Vec::new(),
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
}
