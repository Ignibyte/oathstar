use std::collections::BTreeMap;

use anyhow::{bail, Context};
use oathstar_core::{RoomDefinition, WorldDefinition};
use serde::Deserialize;

const BEGINNER_MODULE: &str = include_str!("../../../modules/beginner/module.toml");
const BEGINNER_ROOMS: &str = include_str!("../../../modules/beginner/rooms.toml");

#[derive(Debug, Deserialize)]
struct ModuleToml {
    id: String,
    name: String,
    start_room_id: String,
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
}

pub fn load_beginner_world() -> anyhow::Result<WorldDefinition> {
    load_world_from_toml(BEGINNER_MODULE, BEGINNER_ROOMS)
}

/// Parse, assemble, and validate a world from raw module + rooms TOML.
///
/// Split out from [`load_beginner_world`] so the loader's invariant branches
/// (duplicate room id, then core validation) are reachable from tests with
/// crafted input rather than only the embedded beginner module.
///
/// # Errors
/// Returns an error if either TOML is malformed, two rooms share an id, or the
/// assembled world fails [`WorldDefinition::validate`].
fn load_world_from_toml(module_src: &str, rooms_src: &str) -> anyhow::Result<WorldDefinition> {
    let module: ModuleToml = toml::from_str(module_src).context("invalid module TOML")?;
    let rooms_toml: RoomsToml = toml::from_str(rooms_src).context("invalid rooms TOML")?;

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
            },
        );
    }

    let world = WorldDefinition {
        id: module.id,
        title: module.name,
        start_room_id: module.start_room_id,
        rooms,
    };

    // Validate through the core boundary — the single source of truth for world
    // invariants (ticket #2 / REQ-007). A typed `WorldValidationError` converts
    // into `anyhow::Error` via `?`.
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
        let err = load_world_from_toml(module, rooms).expect_err("duplicate id must be rejected");
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
            load_world_from_toml(module, ONE_ROOM).expect_err("missing start must be rejected");
        assert!(
            err.to_string()
                .contains("start room 'ghost' does not exist"),
            "unexpected error: {err}"
        );
    }
}
