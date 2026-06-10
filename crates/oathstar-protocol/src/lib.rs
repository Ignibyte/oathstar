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
    /// The active combat encounter, if one is underway (ticket #22). `None` — and
    /// omitted from JSON — outside combat, so a combatless snapshot is
    /// byte-identical to before and an old payload without a `combat` key still
    /// deserializes (the same additive pattern as `oath`/`pack`). Present only
    /// between combat start and resolution: the client opens the battle modal
    /// while it is present and closes it when it clears.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combat: Option<CombatSnapshot>,
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
    /// Hostile-combat affordances (ticket #23). `Some` iff the thing is a hostile
    /// (`Role::Hostile`) — its presence is how the client flags an enemy; absent for
    /// non-hostiles, so a non-hostile entry is byte-identical to before. Server-authored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threat: Option<NearbyThreatSnapshot>,
    /// Server-disclosed combat stats for entity inspection (ticket #23). `Some` iff the
    /// thing has a combat profile (any combatant); an absent field is omitted, so
    /// non-combatants stay byte-identical. Within it, an absent stat is the explicit
    /// "unknown" (hidden) state — the client renders unknown, never an invented value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<NearbyStatsSnapshot>,
}

/// Hostile-combat affordances for a [`NearbySnapshot`] (ticket #23).
///
/// Present only on a hostile entry; the server decides attackability so the client
/// never infers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NearbyThreatSnapshot {
    /// Whether the player can attack this hostile right now — the server's verdict
    /// (hostile, within interactable reach, and the current area allows combat).
    pub attackable: bool,
    /// The canonical command the Attack action sends (`"attack <name>"`); `Some` only
    /// when `attackable`, so the client never offers an enabled attack otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack_command: Option<String>,
}

/// Server-disclosed combat stats for a [`NearbySnapshot`] (ticket #23).
///
/// Present iff the entity has a combat profile; each stat is `Some` when the server
/// discloses it and `None` when hidden (rendered as "unknown"). For a not-yet-engaged
/// nearby combatant these are the authored maxima — live HP belongs to the active
/// battle snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NearbyStatsSnapshot {
    /// Disclosed current/authored health, or `None` if hidden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<u32>,
    /// Disclosed maximum health, or `None` if hidden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_health: Option<u32>,
    /// Disclosed attack damage, or `None` if hidden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<u32>,
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

/// The result of a resolved combat encounter, from the player's perspective
/// (ticket #22). Carried by [`GameEventKind::CombatEnded`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatOutcome {
    /// The enemy reached zero HP.
    Victory,
    /// The player reached zero HP.
    Defeat,
    /// The player fled the encounter (ticket #24): combat ends, the enemy
    /// survives in place, and the player keeps their current HP.
    Fled,
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

/// The active combat encounter as exposed in a [`GameSnapshot`] (ticket #22).
///
/// Renderer-agnostic JSON state for the client's battle modal — never drawing
/// instructions. `participants` is a side-tagged list (the player plus the
/// enemy in v1) so the layout extends to multiple allies/enemies later; `log`
/// is the running battle play-by-play; `round` counts the rounds resolved so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatSnapshot {
    /// Combat rounds resolved so far (1 after the opening round).
    pub round: u32,
    /// The combatants and their current state (player + enemy in v1).
    pub participants: Vec<CombatantSnapshot>,
    /// The battle play-by-play lines, oldest first. Omitted from JSON when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub log: Vec<String>,
    /// The player's queued between-pulse action — `"flee"` (ticket #24),
    /// `"guard"` or `"power_strike"` (ticket #25) — resolved at the next
    /// combat pulse's skill window. `None` — and omitted from JSON — when
    /// nothing is queued, so a queueless snapshot is byte-identical to the
    /// #22 shape and an old payload still deserializes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queued_action: Option<String>,
}

/// One combatant in a [`CombatSnapshot`] (ticket #22).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatantSnapshot {
    /// The combatant's id (`"player"` or the enemy's entity id).
    pub id: String,
    /// The combatant's display name.
    pub name: String,
    /// Current hit points (clamped at zero).
    pub hp: i32,
    /// Maximum hit points.
    pub max_hp: i32,
    /// The side this combatant fights on: `"player"` or `"enemy"` (future: `"ally"`).
    pub side: String,
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
    /// Combat began against a hostile (emitted on the `Combat` channel, ticket #22).
    /// A typed lifecycle marker bracketing the play-by-play combat messages so a
    /// future collapsible combat log can group an encounter; `text` is the
    /// human-facing opening line shown in the feed.
    CombatStarted {
        enemy_id: String,
        enemy_name: String,
        text: String,
    },
    /// Combat resolved with a win/loss `outcome` (emitted on the `Combat` channel,
    /// ticket #22). `text` is the compact feed summary the battle modal leaves
    /// behind when it closes.
    CombatEnded {
        outcome: CombatOutcome,
        text: String,
    },
    /// A combat pulse began resolving (ticket #24) — the typed cycle marker for
    /// the real-time loop, emitted on the `Combat` channel once per due pulse
    /// before its exchange events. `round` is the cycle the pulse resolves.
    /// Carries no feed text: the play-by-play `CombatMessage`s narrate the
    /// exchange, and a JSON client uses this marker to refresh state so the
    /// battle modal updates live without a command.
    CombatPulse {
        round: u32,
    },
    /// A scoped world announcement delivered to this player (ticket #27),
    /// emitted on the `Region` channel. The engine's delivery decision already
    /// matched the player's location at emission — nothing scope-filtered ever
    /// serializes, so a client renders announcements and never decides
    /// receipt. `severity` drives the feed presentation; `text` is the line.
    Announcement {
        severity: AnnouncementSeverity,
        text: String,
    },
    /// The player reached a new level (ticket #30), emitted on the `Skill`
    /// channel — one event per level gained, even when a single XP award
    /// crosses several thresholds. `max_hp` carries the new maximum after the
    /// level's growth so a client can show the benefit without refetching.
    /// The typed render IS the feed line ("You reach level N.") — the engine
    /// emits no duplicate log message.
    LevelUp {
        level: u32,
        max_hp: i32,
    },
}

/// How loudly an announcement presents (ticket #27): the announcements
/// intake's audibility ladder trimmed to render-meaningful levels. Carried by
/// [`GameEventKind::Announcement`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementSeverity {
    /// Ambient world news.
    Notice,
    /// Something the player should heed.
    Warning,
    /// Urgent, region-shaking news.
    Alarm,
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
        AnnouncementSeverity, CombatOutcome, CombatSnapshot, CombatantSnapshot, EventChannel,
        GameEvent, GameEventKind, GameSnapshot, MapSnapshot, NearbySnapshot, NearbyStatsSnapshot,
        NearbyThreatSnapshot, PackItemSnapshot, PlayerSnapshot, RoomSnapshot,
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
            threat: None,
            stats: None,
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

    // ---- ticket #23: additive hostile affordances + disclosure ----

    // P1 (REQ-001/005): the threat + stats objects serialize as camelCase
    // (`attackCommand`, `maxHealth`), omit `None` fields, and round-trip by value.
    #[test]
    fn nearby_threat_and_stats_round_trip_camel_case() {
        let mut thing = nearby("stray");
        thing.threat = Some(NearbyThreatSnapshot {
            attackable: true,
            attack_command: Some("attack Stray".to_string()),
        });
        thing.stats = Some(NearbyStatsSnapshot {
            health: Some(9),
            max_health: Some(9),
            attack: Some(3),
        });
        let value = serde_json::to_value(&thing).expect("serialize");
        assert_eq!(value["threat"]["attackable"], true);
        assert_eq!(value["threat"]["attackCommand"], "attack Stray");
        assert_eq!(value["stats"]["maxHealth"], 9);
        assert_eq!(value["stats"]["attack"], 3);

        let back: NearbySnapshot =
            serde_json::from_str(&serde_json::to_string(&thing).expect("ser")).expect("de");
        assert_eq!(back, thing, "threat + stats round-trip by value");
    }

    // P1 (REQ-002/005): a not-attackable threat omits `attackCommand`, and an
    // undisclosed combatant's stats object omits every inner value (→ "unknown").
    #[test]
    fn nearby_optional_inner_fields_are_omitted_when_none() {
        let mut thing = nearby("wolf");
        thing.threat = Some(NearbyThreatSnapshot {
            attackable: false,
            attack_command: None,
        });
        thing.stats = Some(NearbyStatsSnapshot {
            health: None,
            max_health: None,
            attack: None,
        });
        let json = serde_json::to_string(&thing).expect("serialize");
        assert!(
            !json.contains("attackCommand"),
            "attack_command omitted: {json}"
        );
        assert!(!json.contains("health"), "hidden health omitted: {json}");
        assert!(
            json.contains("\"attackable\":false"),
            "attackable kept: {json}"
        );
        assert!(
            json.contains("\"stats\":{}"),
            "hidden combatant keeps an empty stats: {json}"
        );
    }

    // P2 (REQ-007): a non-hostile non-combatant omits threat + stats entirely, and
    // an old NearbySnapshot JSON without those keys still deserializes (both None).
    #[test]
    fn nearby_without_threat_or_stats_is_byte_identical_and_loads() {
        let json = serde_json::to_string(&nearby("mara")).expect("serialize");
        assert!(!json.contains("threat"), "threat omitted: {json}");
        assert!(!json.contains("stats"), "stats omitted: {json}");

        let legacy = r#"{"id":"mara","name":"Mara","kind":"actor","distance":1,"proximity":"interactable","interactable":true}"#;
        let back: NearbySnapshot = serde_json::from_str(legacy).expect("deserialize legacy");
        assert_eq!(back.threat, None);
        assert_eq!(back.stats, None);
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
            combat: None,
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

    // ---- ticket #22: combat snapshot + events ----

    // C20: the combat snapshot is camelCase JSON and round-trips by value.
    #[test]
    fn combat_snapshot_is_camel_case_and_round_trips() {
        let combat = CombatSnapshot {
            round: 2,
            participants: vec![
                CombatantSnapshot {
                    id: "player".to_string(),
                    name: "Oathbearer".to_string(),
                    hp: 17,
                    max_hp: 20,
                    side: "player".to_string(),
                },
                CombatantSnapshot {
                    id: "stray".to_string(),
                    name: "Stray".to_string(),
                    hp: 6,
                    max_hp: 10,
                    side: "enemy".to_string(),
                },
            ],
            log: vec!["You strike Stray for 4 (6/10).".to_string()],
            queued_action: None,
        };
        let value = serde_json::to_value(&combat).expect("serialize");
        assert_eq!(value["round"], 2);
        assert_eq!(value["participants"][1]["maxHp"], 10, "camelCase maxHp");
        assert_eq!(value["participants"][1]["side"], "enemy");
        let back: CombatSnapshot =
            serde_json::from_str(&serde_json::to_string(&combat).expect("ser")).expect("de");
        assert_eq!(back, combat, "round-trips by value");
    }

    // C20: an empty battle log is omitted from JSON.
    #[test]
    fn combat_snapshot_omits_empty_log() {
        let combat = CombatSnapshot {
            round: 1,
            participants: Vec::new(),
            log: Vec::new(),
            queued_action: None,
        };
        let json = serde_json::to_string(&combat).expect("serialize");
        assert!(!json.contains("log"), "empty log omitted: {json}");
    }

    // C19/REQ-008 (wire): a combatless GameSnapshot omits `combat`; an old payload
    // without the key still deserializes.
    #[test]
    fn snapshot_combat_is_omitted_and_optional() {
        let json = serde_json::to_string(&bare_snapshot()).expect("serialize");
        assert!(!json.contains("\"combat\""), "omitted when None: {json}");
        let back: GameSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert!(back.combat.is_none());
    }

    // C20: the typed combat lifecycle events serialize with a snake_case `type`
    // tag on the Combat channel; CombatOutcome is snake_case.
    #[test]
    fn combat_events_serialize_with_snake_case_tags() {
        let started = GameEvent {
            event_id: 1,
            tick: 1,
            channel: EventChannel::Combat,
            kind: GameEventKind::CombatStarted {
                enemy_id: "stray".to_string(),
                enemy_name: "Stray".to_string(),
                text: "Stray turns on you.".to_string(),
            },
        };
        let value = serde_json::to_value(&started).expect("serialize");
        assert_eq!(value["type"], "combat_started");
        assert_eq!(value["channel"], "combat");
        assert_eq!(value["text"], "Stray turns on you.");

        let ended = GameEvent {
            event_id: 2,
            tick: 2,
            channel: EventChannel::Combat,
            kind: GameEventKind::CombatEnded {
                outcome: CombatOutcome::Defeat,
                text: "Stray bested you.".to_string(),
            },
        };
        let value = serde_json::to_value(&ended).expect("serialize");
        assert_eq!(value["type"], "combat_ended");
        assert_eq!(value["outcome"], "defeat", "CombatOutcome is snake_case");

        // Victory serializes to the other snake_case variant.
        let victory = serde_json::to_value(CombatOutcome::Victory).expect("serialize");
        assert_eq!(victory, serde_json::json!("victory"));
    }

    // P1 (ticket #24): the combat-pulse marker serializes with the snake_case
    // tag, its round, and the camelCase envelope — and round-trips.
    #[test]
    fn combat_pulse_serializes_with_snake_case_tag_and_round() {
        let pulse = GameEvent {
            event_id: 7,
            tick: 4,
            channel: EventChannel::Combat,
            kind: GameEventKind::CombatPulse { round: 3 },
        };
        let value = serde_json::to_value(&pulse).expect("serialize");
        assert_eq!(value["type"], "combat_pulse");
        assert_eq!(value["round"], 3);
        assert_eq!(value["channel"], "combat");
        assert_eq!(value["eventId"], 7, "camelCase envelope");
        let back: GameEvent = serde_json::from_value(value).expect("deserialize");
        assert!(matches!(back.kind, GameEventKind::CombatPulse { round: 3 }));
    }

    // P2 (ticket #24): the fled outcome is snake_case and rides CombatEnded.
    #[test]
    fn fled_outcome_serializes_snake_case_and_round_trips() {
        let fled = serde_json::to_value(CombatOutcome::Fled).expect("serialize");
        assert_eq!(fled, serde_json::json!("fled"));
        let ended = GameEvent {
            event_id: 9,
            tick: 6,
            channel: EventChannel::Combat,
            kind: GameEventKind::CombatEnded {
                outcome: CombatOutcome::Fled,
                text: "You break away from Stray and escape.".to_string(),
            },
        };
        let value = serde_json::to_value(&ended).expect("serialize");
        assert_eq!(value["outcome"], "fled");
        let back: GameEvent = serde_json::from_value(value).expect("deserialize");
        assert!(matches!(
            back.kind,
            GameEventKind::CombatEnded {
                outcome: CombatOutcome::Fled,
                ..
            }
        ));
    }

    // N6 (ticket #27): the announcement event is snake_case on the wire and
    // every severity round-trips.
    #[test]
    fn announcement_serializes_with_snake_case_tag_and_severity() {
        let event = GameEvent {
            event_id: 11,
            tick: 9,
            channel: EventChannel::Region,
            kind: GameEventKind::Announcement {
                severity: AnnouncementSeverity::Alarm,
                text: "The bell rings.".to_string(),
            },
        };
        let value = serde_json::to_value(&event).expect("serialize");
        assert_eq!(value["type"], "announcement");
        assert_eq!(value["severity"], "alarm");
        assert_eq!(value["channel"], "region");
        assert_eq!(value["text"], "The bell rings.");
        let back: GameEvent = serde_json::from_value(value).expect("deserialize");
        assert!(matches!(
            back.kind,
            GameEventKind::Announcement {
                severity: AnnouncementSeverity::Alarm,
                ..
            }
        ));
        for (severity, wire) in [
            (AnnouncementSeverity::Notice, "notice"),
            (AnnouncementSeverity::Warning, "warning"),
            (AnnouncementSeverity::Alarm, "alarm"),
        ] {
            assert_eq!(
                serde_json::to_value(severity).expect("serialize"),
                serde_json::json!(wire)
            );
        }
    }

    // P3 (ticket #24): `queuedAction` is additive camelCase — omitted when None
    // (the #22 byte shape is preserved) and round-tripping when queued.
    #[test]
    fn combat_snapshot_queued_action_is_additive_camel_case() {
        let mut combat = CombatSnapshot {
            round: 1,
            participants: Vec::new(),
            log: Vec::new(),
            queued_action: None,
        };
        let json = serde_json::to_string(&combat).expect("serialize");
        assert!(!json.contains("queuedAction"), "omitted when None: {json}");

        combat.queued_action = Some("flee".to_string());
        let value = serde_json::to_value(&combat).expect("serialize");
        assert_eq!(
            value["queuedAction"], "flee",
            "camelCase key, raw action value"
        );
        let back: CombatSnapshot = serde_json::from_value(value).expect("deserialize");
        assert_eq!(back, combat, "round-trips by value");
    }
}
