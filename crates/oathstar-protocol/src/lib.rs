use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandRequest {
    pub input: String,
    #[serde(default)]
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResponse {
    pub accepted: bool,
    pub events: Vec<GameEvent>,
    pub snapshot: GameSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSnapshot {
    pub world_id: String,
    pub world_title: String,
    pub tick: u64,
    pub current_room_id: String,
    pub player: PlayerSnapshot,
    pub room: RoomSnapshot,
    pub map: MapSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSnapshot {
    pub id: String,
    pub name: String,
    pub level: u32,
    pub xp: u64,
    pub hp: i32,
    pub max_hp: i32,
    pub focus: i32,
    pub max_focus: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomSnapshot {
    pub id: String,
    pub title: String,
    pub region: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
#[serde(rename_all = "camelCase")]
pub struct MapSnapshot {
    pub region: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subregion: Option<String>,
    pub current_room_id: String,
    pub rooms: Vec<MapRoomSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapRoomSnapshot {
    pub id: String,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub glyph: char,
    pub passable: bool,
    pub discovered: bool,
    pub current: bool,
    pub exits: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameEvent {
    pub event_id: u64,
    pub tick: u64,
    pub channel: EventChannel,
    #[serde(flatten)]
    pub kind: GameEventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventChannel {
    Narrative,
    Room,
    Combat,
    Loot,
    Skill,
    Oath,
    Region,
    Inventory,
    Equipment,
    System,
    Dm,
    Debug,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameEventKind {
    LogMessage {
        component: OutputComponent,
        text: String,
    },
    Tick {
        value: u64,
    },
    RoomEntered {
        room_id: String,
        title: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputComponent {
    NarrativeMessage,
    RoomHeader,
    SystemMessage,
    CombatMessage,
    OathCard,
    RegionStandingCard,
    EntityChip,
    ItemCard,
    MapPatch,
}
