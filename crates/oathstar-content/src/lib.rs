use std::collections::BTreeMap;

use anyhow::{bail, Context};
use oathstar_core::{
    Entity, Item, OathDefinition, RegionDefinition, RoomDefinition, SubregionDefinition,
    WorldDefinition,
};
use serde::Deserialize;

const BEGINNER_MODULE: &str = include_str!("../../../modules/beginner/module.toml");
const BEGINNER_ROOMS: &str = include_str!("../../../modules/beginner/rooms.toml");
const BEGINNER_WORLD: &str = include_str!("../../../modules/beginner/world.toml");

#[derive(Debug, Deserialize)]
struct ModuleToml {
    id: String,
    name: String,
    start_room_id: String,
    /// The oath this module offers, if any (swearable via the `swear` command).
    #[serde(default)]
    oath_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RoomsToml {
    rooms: Vec<RoomToml>,
}

#[derive(Debug, Deserialize)]
struct RoomToml {
    id: String,
    title: String,
    region: String,
    subregion: Option<String>,
    description: String,
    x: i32,
    y: i32,
    z: i32,
    glyph: char,
    passable: bool,
    #[serde(default)]
    exits: BTreeMap<String, String>,
    #[serde(default)]
    entities: Vec<String>,
    #[serde(default)]
    items: Vec<String>,
    #[serde(default)]
    combat_enabled: bool,
}

/// Regions, subregions, entities, and items for a module, deserialized directly
/// into the core domain types. Every section is optional.
#[derive(Debug, Default, Deserialize)]
struct WorldToml {
    #[serde(default)]
    regions: Vec<RegionDefinition>,
    #[serde(default)]
    subregions: Vec<SubregionDefinition>,
    #[serde(default)]
    entities: Vec<Entity>,
    #[serde(default)]
    items: Vec<Item>,
    #[serde(default)]
    oaths: Vec<OathDefinition>,
}

pub fn load_beginner_world() -> anyhow::Result<WorldDefinition> {
    load_world_from_toml(BEGINNER_MODULE, BEGINNER_ROOMS, BEGINNER_WORLD)
}

/// Index a list of id-bearing content items into a `BTreeMap`, rejecting any
/// duplicate id with a typed error.
///
/// # Errors
/// Returns an error if two items share an id.
fn index_by_id<T>(
    items: Vec<T>,
    id_of: impl Fn(&T) -> String,
    kind: &str,
) -> anyhow::Result<BTreeMap<String, T>> {
    let mut map = BTreeMap::new();
    for item in items {
        let id = id_of(&item);
        if map.contains_key(&id) {
            bail!("duplicate {kind} id '{id}'");
        }
        map.insert(id, item);
    }
    Ok(map)
}

/// Parse, assemble, and validate a world from raw module + rooms + world TOML.
///
/// Split out from [`load_beginner_world`] so the loader's invariant branches
/// (duplicate ids, then core reference validation) are reachable from tests with
/// crafted input rather than only the embedded beginner module.
///
/// # Errors
/// Returns an error if any TOML is malformed, two rooms/regions/subregions/
/// entities/items share an id, or the assembled world fails
/// [`WorldDefinition::validate`].
fn load_world_from_toml(
    module_src: &str,
    rooms_src: &str,
    world_src: &str,
) -> anyhow::Result<WorldDefinition> {
    let module: ModuleToml = toml::from_str(module_src).context("invalid module TOML")?;
    let rooms_toml: RoomsToml = toml::from_str(rooms_src).context("invalid rooms TOML")?;
    let world_toml: WorldToml = toml::from_str(world_src).context("invalid world TOML")?;

    let mut rooms = BTreeMap::new();
    for room in rooms_toml.rooms {
        if rooms.contains_key(&room.id) {
            bail!("duplicate room id '{}'", room.id);
        }

        rooms.insert(
            room.id.clone(),
            RoomDefinition {
                id: room.id,
                title: room.title,
                region: room.region,
                subregion: room.subregion,
                description: room.description,
                exits: room.exits,
                x: room.x,
                y: room.y,
                z: room.z,
                glyph: room.glyph,
                passable: room.passable,
                entities: room.entities,
                items: room.items,
                combat_enabled: room.combat_enabled,
            },
        );
    }

    let world = WorldDefinition {
        id: module.id,
        title: module.name,
        start_room_id: module.start_room_id,
        rooms,
        regions: index_by_id(world_toml.regions, |r| r.id.clone(), "region")?,
        subregions: index_by_id(world_toml.subregions, |s| s.id.clone(), "subregion")?,
        entities: index_by_id(world_toml.entities, |e| e.id.clone(), "entity")?,
        items: index_by_id(world_toml.items, |i| i.id.clone(), "item")?,
        oaths: index_by_id(world_toml.oaths, |o| o.id.clone(), "oath")?,
        oath_id: module.oath_id,
    };

    // Validate through the core boundary — the single source of truth for world
    // invariants (ticket #2 / #6). A typed `WorldValidationError` converts into
    // `anyhow::Error` via `?`.
    world.validate()?;

    Ok(world)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_ROOM: &str = "[[rooms]]\n\
        id = \"a\"\ntitle = \"A\"\nregion = \"r\"\ndescription = \"d\"\n\
        x = 0\ny = 0\nz = 0\nglyph = \".\"\npassable = true\n";

    #[test]
    fn beginner_world_loads() {
        let world = load_beginner_world().expect("beginner module should load");
        assert_eq!(world.start_room_id, "hollowmere_square");
        assert!(world.rooms.contains_key("bell_eater_roost"));
    }

    #[test]
    fn load_rejects_duplicate_room_id() {
        let module = "id = \"m\"\nname = \"M\"\nstart_room_id = \"dup\"\n";
        let rooms = "[[rooms]]\n\
            id = \"dup\"\ntitle = \"A\"\nregion = \"r\"\ndescription = \"d\"\n\
            x = 0\ny = 0\nz = 0\nglyph = \".\"\npassable = true\n\
            [[rooms]]\n\
            id = \"dup\"\ntitle = \"B\"\nregion = \"r\"\ndescription = \"d\"\n\
            x = 1\ny = 0\nz = 0\nglyph = \".\"\npassable = true\n";
        let err =
            load_world_from_toml(module, rooms, "").expect_err("duplicate id must be rejected");
        assert!(
            err.to_string().contains("duplicate room id 'dup'"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_propagates_core_validation_error() {
        // Valid TOML with unique ids, but the start room does not exist — the
        // assembled world must be rejected by the core validator (REQ-007).
        let module = "id = \"m\"\nname = \"M\"\nstart_room_id = \"ghost\"\n";
        let err =
            load_world_from_toml(module, ONE_ROOM, "").expect_err("missing start must be rejected");
        assert!(
            err.to_string()
                .contains("start room 'ghost' does not exist"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_rejects_malformed_module_toml() {
        // `x = ` is a TOML syntax error (no value), so the module never parses.
        let err = load_world_from_toml("x = ", ONE_ROOM, "")
            .expect_err("malformed module TOML must be rejected");
        assert!(
            err.to_string().contains("invalid module TOML"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_rejects_malformed_rooms_toml() {
        // Module parses; rooms is the wrong shape (a string, not an array of tables).
        let module = "id = \"m\"\nname = \"M\"\nstart_room_id = \"a\"\n";
        let err = load_world_from_toml(module, "rooms = \"not a table array\"", "")
            .expect_err("malformed rooms TOML must be rejected");
        assert!(
            err.to_string().contains("invalid rooms TOML"),
            "unexpected error: {err}"
        );
    }

    // T7: the beginner world assembles its regions, subregions, and sample
    // entity/item, and passes core validation.
    #[test]
    fn beginner_world_has_regions_entities_items() {
        let world = load_beginner_world().expect("beginner module should load");
        assert_eq!(world.regions.len(), 3);
        assert!(world.regions.contains_key("hollowmere"));
        assert_eq!(world.subregions.len(), 5);
        assert!(world.entities.contains_key("mara"));
        assert!(world.items.contains_key("candle"));
    }

    // T8: a dangling reference in assembled content is rejected through the loader
    // (core validation runs inside the loader).
    #[test]
    fn load_rejects_missing_item_reference() {
        let module = "id = \"m\"\nname = \"M\"\nstart_room_id = \"a\"\n";
        let rooms = "[[rooms]]\n\
            id = \"a\"\ntitle = \"A\"\nregion = \"r1\"\ndescription = \"d\"\n\
            x = 0\ny = 0\nz = 0\nglyph = \".\"\npassable = true\n";
        let world = "[[regions]]\nid = \"r1\"\nname = \"R1\"\n\
            [[entities]]\nid = \"e1\"\nname = \"E\"\ndescription = \"d\"\n\
            kind = \"actor\"\ninventory = [\"ghost\"]\n";
        let err = load_world_from_toml(module, rooms, world)
            .expect_err("missing item reference must be rejected");
        assert!(
            err.to_string().contains("references missing item 'ghost'"),
            "unexpected error: {err}"
        );
    }

    // T8b: a duplicate id within a registry is rejected (covers `index_by_id`).
    #[test]
    fn load_rejects_duplicate_region_id() {
        let module = "id = \"m\"\nname = \"M\"\nstart_room_id = \"a\"\n";
        let rooms = "[[rooms]]\n\
            id = \"a\"\ntitle = \"A\"\nregion = \"r1\"\ndescription = \"d\"\n\
            x = 0\ny = 0\nz = 0\nglyph = \".\"\npassable = true\n";
        let world = "[[regions]]\nid = \"r1\"\nname = \"R1\"\n\
            [[regions]]\nid = \"r1\"\nname = \"Dup\"\n";
        let err = load_world_from_toml(module, rooms, world)
            .expect_err("duplicate region id must be rejected");
        assert!(
            err.to_string().contains("duplicate region id 'r1'"),
            "unexpected error: {err}"
        );
    }

    // ---- ticket #7: oath content + boss placement ----

    // REQ-007 / content: the real beginner world carries the designated oath, the
    // boss entity (boss role + owned clapper), and the boss placement.
    #[test]
    fn beginner_world_has_oath_and_boss() {
        let world = load_beginner_world().expect("beginner module should load");
        assert_eq!(world.oath_id.as_deref(), Some("hollow_bell"));
        assert!(world.oaths.contains_key("hollow_bell"));
        let boss = world.entities.get("bell_eater").expect("bell_eater entity");
        assert!(
            boss.roles.iter().any(|r| r == "boss"),
            "bell_eater carries the boss role"
        );
        assert!(
            boss.inventory.iter().any(|i| i == "bell_clapper"),
            "bell_eater owns the clapper"
        );
        assert!(
            world
                .rooms
                .get("bell_eater_roost")
                .expect("roost room")
                .entities
                .iter()
                .any(|e| e == "bell_eater"),
            "bell_eater is placed in the roost"
        );
    }

    // REQ-007: a designated oath the world never defines is rejected (dangling).
    #[test]
    fn load_rejects_missing_oath_reference() {
        let module = "id = \"m\"\nname = \"M\"\nstart_room_id = \"a\"\noath_id = \"ghost\"\n";
        let world = "[[regions]]\nid = \"r\"\nname = \"R\"\n";
        let err = load_world_from_toml(module, ONE_ROOM, world)
            .expect_err("missing designated oath must be rejected");
        assert!(
            err.to_string()
                .contains("designated oath 'ghost' does not exist"),
            "unexpected error: {err}"
        );
    }

    // REQ-007: a duplicate oath id is rejected by the loader (index_by_id "oath").
    #[test]
    fn load_rejects_duplicate_oath_id() {
        let module = "id = \"m\"\nname = \"M\"\nstart_room_id = \"a\"\n";
        let world = "[[oaths]]\nid = \"o\"\ntitle = \"O\"\ndescription = \"d\"\n\
            [[oaths]]\nid = \"o\"\ntitle = \"Dup\"\ndescription = \"d\"\n";
        let err = load_world_from_toml(module, ONE_ROOM, world)
            .expect_err("duplicate oath id must be rejected");
        assert!(
            err.to_string().contains("duplicate oath id 'o'"),
            "unexpected error: {err}"
        );
    }

    // A module with neither oath_id nor [[oaths]] loads: serde defaults give
    // None / empty (exercises the default paths for coverage).
    #[test]
    fn load_accepts_module_without_oath() {
        let module = "id = \"m\"\nname = \"M\"\nstart_room_id = \"a\"\n";
        let world = "[[regions]]\nid = \"r\"\nname = \"R\"\n";
        let loaded =
            load_world_from_toml(module, ONE_ROOM, world).expect("a module without an oath loads");
        assert!(loaded.oath_id.is_none(), "no designated oath");
        assert!(loaded.oaths.is_empty(), "no oaths defined");
    }

    // ---- ticket #19: NPC dialogue + oath issuer/source ----

    // T10 (REQ-001/006): the beginner world loads Mara's authored dialogue and the
    // oath's issuer/source metadata from TOML (asserted by exact value).
    #[test]
    fn beginner_world_loads_mara_dialogue_and_oath_issuer() {
        let world = load_beginner_world().expect("beginner module should load");

        let mara = world.entities.get("mara").expect("mara entity");
        let dialogue = mara.dialogue.as_ref().expect("mara has authored dialogue");
        let oath_lines = dialogue.oath.as_ref().expect("mara has oath dialogue");
        assert!(!oath_lines.offer.is_empty(), "the offer line is authored");

        let oath = world.oaths.get("hollow_bell").expect("hollow_bell oath");
        assert_eq!(oath.issuer_id.as_deref(), Some("mara"));
        assert_eq!(oath.source.as_deref(), Some("hollowmere"));
    }

    // T11 (REQ-006): an oath whose issuer is not a defined entity is rejected by
    // the loader (core validation runs inside the loader).
    #[test]
    fn load_rejects_oath_with_missing_issuer() {
        let module = "id = \"m\"\nname = \"M\"\nstart_room_id = \"a\"\n";
        let world = "[[regions]]\nid = \"r\"\nname = \"R\"\n\
            [[oaths]]\nid = \"o\"\ntitle = \"O\"\ndescription = \"d\"\nissuer_id = \"ghost\"\n";
        let err = load_world_from_toml(module, ONE_ROOM, world)
            .expect_err("an oath with a missing issuer must be rejected");
        assert!(
            err.to_string()
                .contains("references missing issuer 'ghost'"),
            "unexpected error: {err}"
        );
    }

    // T12 (REQ-006): an oath with no issuer/source loads, defaulting both to None.
    #[test]
    fn load_accepts_oath_without_issuer() {
        let module = "id = \"m\"\nname = \"M\"\nstart_room_id = \"a\"\n";
        let world = "[[regions]]\nid = \"r\"\nname = \"R\"\n\
            [[oaths]]\nid = \"o\"\ntitle = \"O\"\ndescription = \"d\"\n";
        let loaded =
            load_world_from_toml(module, ONE_ROOM, world).expect("an oath without an issuer loads");
        let oath = loaded.oaths.get("o").expect("oath o present");
        assert!(oath.issuer_id.is_none(), "no issuer by default");
        assert!(oath.source.is_none(), "no source by default");
    }

    // T10 (REQ-002): the beginner items load their authored kind/flags by value.
    #[test]
    fn beginner_items_load_kind_and_flags() {
        let world = load_beginner_world().expect("beginner module should load");
        let clapper = world.items.get("bell_clapper").expect("bell_clapper item");
        assert_eq!(clapper.kind.as_deref(), Some("quest"));
        assert_eq!(clapper.flags, vec!["oath".to_string()]);
        assert_eq!(
            world
                .items
                .get("candle")
                .expect("candle item")
                .kind
                .as_deref(),
            Some("light")
        );
        assert!(
            world
                .items
                .get("wax_stub")
                .expect("wax_stub item")
                .flags
                .is_empty(),
            "wax_stub carries no flags"
        );
    }

    // T8 (REQ-004): Mara loads as a talkable oath-giver and the world validates (so
    // her oath_giver contract — the issuer wiring — is satisfied).
    #[test]
    fn beginner_mara_is_talkable_and_oath_giver() {
        let world = load_beginner_world().expect("beginner module should load");
        let mara = world.entities.get("mara").expect("mara entity");
        assert!(
            mara.has_role(oathstar_core::Role::Talkable),
            "mara is talkable"
        );
        assert!(
            mara.has_role(oathstar_core::Role::OathGiver),
            "mara is an oath_giver"
        );
        assert_eq!(world.validate(), Ok(()));
    }

    // T9 (REQ-005): the Bell-Eater loads as combatant + boss with future-combat-ready
    // stats (by value); the world validates and boss progression is unchanged.
    #[test]
    fn beginner_bell_eater_is_combatant_boss_with_combat() {
        let world = load_beginner_world().expect("beginner module should load");
        let boss = world.entities.get("bell_eater").expect("bell_eater entity");
        assert!(boss.has_role(oathstar_core::Role::Combatant), "combatant");
        assert!(boss.has_role(oathstar_core::Role::Boss), "boss");
        assert_eq!(
            boss.combat,
            Some(oathstar_core::CombatProfile {
                health: 12,
                attack: 0,
            }),
            "future-combat-ready stats load by value (attack defaults to 0)"
        );
        assert_eq!(world.validate(), Ok(()));
    }

    // C23 (ticket #22): the Ashen Stray is the beginner combat encounter — an Actor
    // that is combatant + hostile with an authored combat profile (health 9 /
    // attack 3). That the world loads at all proves the hostile contract is met (C22).
    #[test]
    fn beginner_ashen_stray_is_a_hostile_combatant() {
        let world = load_beginner_world().expect("beginner module should load");
        let stray = world
            .entities
            .get("ashen_stray")
            .expect("ashen_stray entity");
        assert_eq!(stray.kind, oathstar_core::EntityKind::Actor);
        assert!(stray.has_role(oathstar_core::Role::Combatant), "combatant");
        assert!(stray.has_role(oathstar_core::Role::Hostile), "hostile");
        assert_eq!(
            stray.combat,
            Some(oathstar_core::CombatProfile {
                health: 9,
                attack: 3,
            }),
            "authored combat stats load by value"
        );
    }

    // C23 (ticket #22): the Ashen Road opts into combat and places the stray; the
    // Bell-Eater roost stays non-combat (REQ-007 — the boss is confront-only).
    #[test]
    fn beginner_ashen_road_is_combat_enabled_and_boss_roost_is_not() {
        let world = load_beginner_world().expect("beginner module should load");
        let road = world.rooms.get("ashen_road").expect("ashen_road room");
        assert!(road.combat_enabled, "the Ashen Road is combat-enabled");
        assert!(
            road.entities.iter().any(|id| id == "ashen_stray"),
            "the stray is placed on the road"
        );
        let roost = world
            .rooms
            .get("bell_eater_roost")
            .expect("bell_eater_roost room");
        assert!(
            !roost.combat_enabled,
            "the boss roost is not combat-enabled (Bell-Eater stays confront-only)"
        );
    }
}
