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
    let module: ModuleToml =
        toml::from_str(BEGINNER_MODULE).context("invalid beginner module TOML")?;
    let rooms_toml: RoomsToml =
        toml::from_str(BEGINNER_ROOMS).context("invalid beginner rooms TOML")?;

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

    validate_world(&module.start_room_id, &rooms)?;

    Ok(WorldDefinition {
        id: module.id,
        title: module.name,
        start_room_id: module.start_room_id,
        rooms,
    })
}

fn validate_world(
    start_room_id: &str,
    rooms: &BTreeMap<String, RoomDefinition>,
) -> anyhow::Result<()> {
    if !rooms.contains_key(start_room_id) {
        bail!("start room '{}' does not exist", start_room_id);
    }

    for room in rooms.values() {
        for (direction, target_room_id) in &room.exits {
            if !rooms.contains_key(target_room_id) {
                bail!(
                    "room '{}' exit '{}' points to missing room '{}'",
                    room.id,
                    direction,
                    target_room_id
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beginner_world_loads() {
        let world = load_beginner_world().expect("beginner module should load");
        assert_eq!(world.start_room_id, "hollowmere_square");
        assert!(world.rooms.contains_key("bell_eater_roost"));
    }
}
