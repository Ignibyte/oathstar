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
    /// The player's current oath, if one has been sworn. `None` until the player
    /// swears the module's oath.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oath: Option<OathSnapshot>,
    /// The player's carried items (ticket #18). Minimal additive pack state: one
    /// `id` + display `name` per carried item, in pickup order. Empty — and
    /// omitted from JSON — until the player takes something, so a packless
    /// snapshot is byte-identical to before and an old payload without a `pack`
    /// key still deserializes (same additive pattern as `oath`/`room.contents`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pack: Vec<PackItemSnapshot>,
}

/// One carried item in the player's pack, as renderer-agnostic snapshot data
/// (ticket #18).
///
/// Minimal by design — `id`, display `name`, coarse `kind`, and basic authored
/// `flags`; quantities, stacks, weight, and equipment slots are out of scope.
/// The client's Pack panel reads this server-authored data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackItemSnapshot {
    /// The item's world id.
    pub id: String,
    /// The item's display name.
    pub name: String,
    /// A coarse kind/type placeholder (ticket #20), e.g. `"item"` — always present.
    pub kind: String,
    /// Basic authored item flags (ticket #20); omitted from JSON when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<String>,
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
    /// Things perceivable from this room within the player's awareness radius.
    ///
    /// The structured spatial-awareness data (ticket #17): things in this cell
    /// (`exact`) and in nearby cells on the same subregion/z-plane, each entry
    /// carrying its `distance` and `proximity` so a client can present "here" vs
    /// "nearby". Empty — and omitted from JSON — when nothing is in sight, which
    /// keeps the payload byte-identical for empty rooms.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contents: Vec<NearbySnapshot>,
}

/// One thing perceived near the player, as renderer-agnostic snapshot data.
///
/// (Ticket #17.) `kind` is `"actor" | "fixture" | "item"`; `proximity` is
/// `"exact" | "interactable" | "visible"`; `interactable` is the convenience
/// boolean (`exact` or `interactable`). Carries no drawing instructions — a
/// client decides how to render the awareness, including the `look <name>` action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NearbySnapshot {
    /// The thing's world id (entity id or item id).
    pub id: String,
    /// The thing's display name.
    pub name: String,
    /// `"actor" | "fixture" | "item"`.
    pub kind: String,
    /// Chebyshev cell distance from the player (0 = same cell).
    pub distance: u32,
    /// `"exact" | "interactable" | "visible"`.
    pub proximity: String,
    /// Whether the player can directly interact (same cell or within reach).
    pub interactable: bool,
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

/// Lifecycle state of a sworn oath, surfaced in the snapshot and carried by oath
/// events. `Broken` is intentionally absent until a break path exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OathStatus {
    /// Sworn and active — the player has taken the oath but not yet fulfilled it.
    Sworn,
    /// Fulfilled — the oath's objective has been met.
    Fulfilled,
}

/// The player's oath as exposed in a [`GameSnapshot`] (the view of the engine's
/// oath state).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OathSnapshot {
    pub oath_id: String,
    pub title: String,
    pub status: OathStatus,
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
    /// The player swore an oath (emitted on the `Oath` channel).
    OathSworn {
        oath_id: String,
        title: String,
    },
    /// A sworn oath was fulfilled (emitted on the `Oath` channel).
    OathFulfilled {
        oath_id: String,
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

#[cfg(test)]
mod tests {
    use super::{
        GameSnapshot, MapSnapshot, NearbySnapshot, PackItemSnapshot, PlayerSnapshot, RoomSnapshot,
    };
    use std::collections::BTreeMap;

    fn bare_room() -> RoomSnapshot {
        RoomSnapshot {
            id: "r".to_string(),
            title: "R".to_string(),
            region: "reg".to_string(),
            subregion: None,
            description: "d".to_string(),
            exits: BTreeMap::new(),
            x: 0,
            y: 0,
            z: 0,
            glyph: '.',
            passable: true,
            contents: Vec::new(),
        }
    }

    fn nearby(id: &str) -> NearbySnapshot {
        NearbySnapshot {
            id: id.to_string(),
            name: "Mara".to_string(),
            kind: "actor".to_string(),
            distance: 1,
            proximity: "interactable".to_string(),
            interactable: true,
        }
    }

    // REQ-005: a NearbySnapshot serializes as JSON-friendly state (no drawing ops).
    #[test]
    fn nearby_snapshot_serializes_expected_fields() {
        let value = serde_json::to_value(nearby("mara")).expect("serialize");
        assert_eq!(value["id"], "mara");
        assert_eq!(value["name"], "Mara");
        assert_eq!(value["kind"], "actor");
        assert_eq!(value["distance"], 1);
        assert_eq!(value["proximity"], "interactable");
        assert!(value["interactable"]
            .as_bool()
            .expect("interactable is bool"));
    }

    // REQ-005/007: empty contents is omitted, so empty-room payloads are unchanged.
    #[test]
    fn empty_contents_is_omitted_from_json() {
        let json = serde_json::to_string(&bare_room()).expect("serialize");
        assert!(!json.contains("contents"), "omitted when empty: {json}");
    }

    // REQ-005: an old snapshot without a `contents` key still deserializes (empty).
    #[test]
    fn room_without_contents_deserializes_to_empty() {
        let json = r#"{"id":"r","title":"R","region":"reg","description":"d","exits":{},"x":0,"y":0,"z":0,"glyph":".","passable":true}"#;
        let room: RoomSnapshot = serde_json::from_str(json).expect("deserialize");
        assert!(room.contents.is_empty());
    }

    // REQ-005: populated contents survives a round trip.
    #[test]
    fn populated_contents_round_trips() {
        let mut room = bare_room();
        room.contents = vec![nearby("mara")];
        let json = serde_json::to_string(&room).expect("serialize");
        assert!(json.contains("\"contents\""));
        let back: RoomSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.contents.len(), 1);
        assert_eq!(back.contents[0].id, "mara");
        assert!(back.contents[0].interactable);
    }

    // ---- ticket #18: additive pack snapshot ----

    fn bare_snapshot() -> GameSnapshot {
        GameSnapshot {
            world_id: "w".to_string(),
            world_title: "W".to_string(),
            tick: 0,
            current_room_id: "r".to_string(),
            player: PlayerSnapshot {
                id: "p".to_string(),
                name: "P".to_string(),
                level: 1,
                xp: 0,
                hp: 1,
                max_hp: 1,
                focus: 0,
                max_focus: 0,
            },
            room: bare_room(),
            map: MapSnapshot {
                region: "reg".to_string(),
                subregion: None,
                current_room_id: "r".to_string(),
                rooms: Vec::new(),
            },
            oath: None,
            pack: Vec::new(),
        }
    }

    fn pack_item(id: &str) -> PackItemSnapshot {
        PackItemSnapshot {
            id: id.to_string(),
            name: "Wax Stub".to_string(),
            kind: "item".to_string(),
            flags: Vec::new(),
        }
    }

    // REQ-007: a PackItemSnapshot serializes its id + camelCase name.
    #[test]
    fn pack_item_snapshot_serializes_id_and_name() {
        let value = serde_json::to_value(pack_item("wax_stub")).expect("serialize");
        assert_eq!(value["id"], "wax_stub");
        assert_eq!(value["name"], "Wax Stub");
    }

    // REQ-007/008: an empty pack is omitted from JSON (byte-identical to before).
    #[test]
    fn empty_pack_is_omitted_from_json() {
        let json = serde_json::to_string(&bare_snapshot()).expect("serialize");
        assert!(!json.contains("\"pack\""), "omitted when empty: {json}");
    }

    // REQ-008: an old snapshot JSON without a `pack` key deserializes (empty pack).
    #[test]
    fn snapshot_without_pack_deserializes_to_empty() {
        let json = serde_json::to_string(&bare_snapshot()).expect("serialize");
        assert!(!json.contains("\"pack\""));
        let back: GameSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert!(back.pack.is_empty());
    }

    // REQ-007: a populated pack round-trips in pickup order.
    #[test]
    fn populated_pack_round_trips() {
        let mut snapshot = bare_snapshot();
        snapshot.pack = vec![pack_item("wax_stub"), pack_item("candle")];
        let json = serde_json::to_string(&snapshot).expect("serialize");
        assert!(json.contains("\"pack\""));
        let back: GameSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.pack.len(), 2);
        assert_eq!(back.pack[0].id, "wax_stub");
        assert_eq!(back.pack[1].id, "candle");
    }

    // T2 (REQ-002): a PackItemSnapshot serializes `kind` + `flags` (camelCase),
    // omits empty flags, and round-trips (kind is always present — server-produced).
    #[test]
    fn pack_item_snapshot_serializes_kind_and_flags() {
        let value = serde_json::to_value(PackItemSnapshot {
            id: "lamp".to_string(),
            name: "Brass Lamp".to_string(),
            kind: "light".to_string(),
            flags: vec!["lit".to_string()],
        })
        .expect("serialize");
        assert_eq!(value["kind"], "light");
        assert_eq!(value["flags"], serde_json::json!(["lit"]));

        // Empty flags are omitted; kind is always present.
        let plain = serde_json::to_value(pack_item("coin")).expect("serialize");
        assert_eq!(plain["kind"], "item");
        assert!(plain.get("flags").is_none(), "empty flags omitted: {plain}");

        // Round-trips; a payload without `flags` deserializes to empty.
        let back: PackItemSnapshot =
            serde_json::from_str(r#"{"id":"x","name":"X","kind":"light"}"#).expect("deserialize");
        assert_eq!(back.kind, "light");
        assert!(back.flags.is_empty());
    }
}
