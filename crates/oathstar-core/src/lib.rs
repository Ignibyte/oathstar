use std::collections::{BTreeMap, BTreeSet};

use oathstar_protocol::{
    CommandRequest, CommandResponse, EventChannel, GameEvent, GameEventKind, GameSnapshot,
    MapRoomSnapshot, MapSnapshot, OutputComponent, PlayerSnapshot, RoomSnapshot,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldDefinition {
    pub id: String,
    pub title: String,
    pub start_room_id: String,
    pub rooms: BTreeMap<String, RoomDefinition>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub tick: u64,
    pub current_room_id: String,
    pub discovered_rooms: BTreeSet<String>,
    pub player: PlayerState,
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
    pub fn new(world: WorldDefinition) -> Self {
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
        };

        Self {
            world,
            state,
            next_event_id: 1,
        }
    }

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
        }
    }

    pub fn tick(&mut self) -> GameEvent {
        self.state.tick += 1;
        self.event(
            EventChannel::Debug,
            GameEventKind::Tick {
                value: self.state.tick,
            },
        )
    }

    pub fn handle_command(&mut self, request: CommandRequest) -> CommandResponse {
        let input = request.input.trim();
        let mut events = Vec::new();

        if input.is_empty() {
            events.push(self.log(
                EventChannel::System,
                OutputComponent::SystemMessage,
                "The world waits for a command.",
            ));
            return self.response(false, events);
        }

        let normalized = input.to_lowercase();
        match normalized.as_str() {
            "help" => {
                events.push(self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    "Try: look, north, south, east, west, up, down.",
                ));
            }
            "look" | "l" => {
                events.extend(self.describe_current_room());
            }
            "north" | "n" | "south" | "s" | "east" | "e" | "west" | "w" | "up" | "u" | "down"
            | "d" => {
                events.extend(self.move_direction(direction_alias(&normalized)));
            }
            _ if normalized.starts_with("go ") => {
                let direction = normalized.trim_start_matches("go ").trim();
                events.extend(self.move_direction(direction_alias(direction)));
            }
            _ => {
                events.push(self.log(
                    EventChannel::System,
                    OutputComponent::SystemMessage,
                    format!("I do not know how to '{input}' yet."),
                ));
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

    fn move_direction(&mut self, direction: &str) -> Vec<GameEvent> {
        let room = self.current_room().clone();
        let Some(next_room_id) = room.exits.get(direction) else {
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

    fn current_room(&self) -> &RoomDefinition {
        self.world
            .rooms
            .get(&self.state.current_room_id)
            .expect("current room must exist in world definition")
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

    fn event(&mut self, channel: EventChannel, kind: GameEventKind) -> GameEvent {
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

fn direction_alias(input: &str) -> &str {
    match input {
        "n" => "north",
        "s" => "south",
        "e" => "east",
        "w" => "west",
        "u" => "up",
        "d" => "down",
        direction => direction,
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
            },
        );

        WorldDefinition {
            id: "test".to_string(),
            title: "Test".to_string(),
            start_room_id: "a".to_string(),
            rooms,
        }
    }

    #[test]
    fn movement_discovers_rooms() {
        let mut engine = Engine::new(test_world());
        let response = engine.handle_command(CommandRequest {
            input: "east".to_string(),
            actor_id: None,
        });

        assert_eq!(response.snapshot.current_room_id, "b");
        assert!(response
            .snapshot
            .map
            .rooms
            .iter()
            .any(|room| room.id == "b" && room.discovered));
    }
}
