use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use oathstar_core::{
    Entity, Item, OathDefinition, RegionDefinition, RoomDefinition, SubregionDefinition,
};
use serde::Deserialize;

mod map_document;
pub use map_document::{
    Cell, ContentCatalog, MapDocument, MapValidationError, RefKind, RoomCell, TerrainCell,
    TerrainDef, SUPPORTED_TILE_SIZES,
};
// `load_beginner_world`/`materialize` already return one, so callers must be able
// to name the type.
pub use oathstar_core::WorldDefinition;

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

/// Build the beginner world's content catalog — the entity/item registry a map
/// document validates its references against.
///
/// Fixtures are empty (the engine has no fixture concept yet).
///
/// # Errors
/// Returns an error if the embedded beginner module fails to load.
pub fn beginner_catalog() -> anyhow::Result<ContentCatalog> {
    let world = load_beginner_world()?;
    Ok(ContentCatalog {
        entities: world.entities,
        items: world.items,
        fixtures: BTreeSet::new(),
    })
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

/// Why loading an authored world from a [`MapDocument`] file failed.
///
/// The file is untrusted input, so each failure mode is a typed variant rather
/// than a panic: the path could not be read, its bytes were not a valid
/// `MapDocument`, or the document could not materialize into a valid world.
#[derive(Debug)]
pub enum AuthoredWorldError {
    /// The authored-world file could not be read.
    Read {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying IO error.
        source: std::io::Error,
    },
    /// The file's bytes were not a valid `MapDocument` JSON.
    Parse {
        /// The path whose contents failed to parse.
        path: PathBuf,
        /// The underlying JSON error.
        source: serde_json::Error,
    },
    /// The document parsed but did not materialize into a valid world.
    Materialize {
        /// The path whose document failed to materialize.
        path: PathBuf,
        /// The underlying validation error.
        source: MapValidationError,
    },
}

impl std::fmt::Display for AuthoredWorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, .. } => {
                write!(f, "failed to read authored world file {}", path.display())
            }
            Self::Parse { path, .. } => write!(
                f,
                "authored world file {} is not a valid map document",
                path.display()
            ),
            Self::Materialize { path, .. } => write!(
                f,
                "authored world file {} did not materialize into a valid world",
                path.display()
            ),
        }
    }
}

impl std::error::Error for AuthoredWorldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::Materialize { source, .. } => Some(source),
        }
    }
}

/// Load an authored world from a [`MapDocument`] JSON file on disk.
///
/// The file is treated as untrusted input: it is read, parsed as a
/// `MapDocument`, then materialized against `catalog` into a validated
/// [`WorldDefinition`]. Every failure is a typed [`AuthoredWorldError`] — there
/// are no panics on the input path.
///
/// # Errors
/// Returns [`AuthoredWorldError`] if the file cannot be read, is not valid
/// `MapDocument` JSON, or does not materialize into a valid world.
pub fn load_authored_world(
    path: &Path,
    catalog: &ContentCatalog,
) -> Result<WorldDefinition, AuthoredWorldError> {
    let json = fs::read_to_string(path).map_err(|source| AuthoredWorldError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let document: MapDocument =
        serde_json::from_str(&json).map_err(|source| AuthoredWorldError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
    document
        .materialize(catalog)
        .map_err(|source| AuthoredWorldError::Materialize {
            path: path.to_path_buf(),
            source,
        })
}

/// Select and load the world the game should start with.
///
/// With no `authored` path the baked beginner world loads unchanged. With a
/// path, the authored `MapDocument` there is loaded and materialized against the
/// beginner catalog (entity/item authoring is a later slice). A configured but
/// invalid authored world is a loud error — the caller does NOT silently fall
/// back to the beginner world.
///
/// # Errors
/// Returns an error if the beginner world fails to load, or if a configured
/// authored world cannot be read, parsed, or materialized.
pub fn load_startup_world(authored: Option<&Path>) -> anyhow::Result<WorldDefinition> {
    match authored {
        None => load_beginner_world(),
        Some(path) => {
            let catalog = beginner_catalog()?;
            load_authored_world(path, &catalog).map_err(anyhow::Error::new)
        }
    }
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

    // A valid, reference-free map document (two rooms, spawn on `alpha`) that
    // materializes against any catalog.
    const AUTHORED_DOC: &str = r#"{
        "id":"authored","title":"Authored","tile_size":16,"width":4,"height":4,"floors":1,
        "terrain_palette":{"floor":{"tile":"f","passable":true}},
        "terrain":[{"x":0,"y":0,"z":0,"terrain":"floor"},{"x":1,"y":0,"z":0,"terrain":"floor"}],
        "regions":{"reg":{"id":"reg","name":"Reg"}},
        "rooms":[{"x":0,"y":0,"z":0,"id":"alpha","region":"reg"},{"x":1,"y":0,"z":0,"id":"beta","region":"reg"}],
        "spawn":{"x":0,"y":0,"z":0}
    }"#;

    // Parses as a `MapDocument` but fails to materialize (unsupported tile size).
    const UNMATERIALIZABLE_DOC: &str = r#"{
        "id":"bad","title":"Bad","tile_size":7,"width":4,"height":4,"floors":1,
        "terrain_palette":{},"terrain":[],"regions":{},"rooms":[],"spawn":null
    }"#;

    fn write_fixture(tag: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("oathstar-content-test-{tag}.json"));
        fs::write(&path, contents).expect("write fixture");
        path
    }

    #[test]
    fn load_authored_world_materializes_a_valid_document() {
        let path = write_fixture("valid", AUTHORED_DOC);
        let catalog = beginner_catalog().expect("catalog loads");
        let world = load_authored_world(&path, &catalog).expect("authored world loads");
        assert_eq!(world.start_room_id, "alpha");
        assert!(world.rooms.contains_key("beta"));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn load_authored_world_missing_file_is_read_error() {
        let path = std::env::temp_dir().join("oathstar-content-test-absent.json");
        fs::remove_file(&path).ok();
        let catalog = beginner_catalog().expect("catalog loads");
        let error = load_authored_world(&path, &catalog).expect_err("a missing file errors");
        assert!(matches!(error, AuthoredWorldError::Read { .. }));
        assert!(!error.to_string().is_empty(), "Read has a message");
        assert!(
            std::error::Error::source(&error).is_some(),
            "Read wraps a source"
        );
    }

    #[test]
    fn load_authored_world_malformed_json_is_parse_error() {
        let path = write_fixture("malformed", "{not json");
        let catalog = beginner_catalog().expect("catalog loads");
        let error = load_authored_world(&path, &catalog).expect_err("malformed JSON errors");
        assert!(matches!(error, AuthoredWorldError::Parse { .. }));
        assert!(!error.to_string().is_empty(), "Parse has a message");
        assert!(
            std::error::Error::source(&error).is_some(),
            "Parse wraps a source"
        );
        fs::remove_file(&path).ok();
    }

    #[test]
    fn load_authored_world_unmaterializable_is_materialize_error() {
        let path = write_fixture("unmaterializable", UNMATERIALIZABLE_DOC);
        let catalog = beginner_catalog().expect("catalog loads");
        let error = load_authored_world(&path, &catalog).expect_err("an invalid doc errors");
        assert!(matches!(error, AuthoredWorldError::Materialize { .. }));
        assert!(!error.to_string().is_empty(), "Materialize has a message");
        assert!(
            std::error::Error::source(&error).is_some(),
            "Materialize wraps a source"
        );
        fs::remove_file(&path).ok();
    }

    #[test]
    fn load_startup_world_none_loads_beginner() {
        let world = load_startup_world(None).expect("beginner loads");
        assert_eq!(world.start_room_id, "hollowmere_square");
    }

    #[test]
    fn load_startup_world_some_loads_the_authored_world() {
        let path = write_fixture("startup", AUTHORED_DOC);
        let world = load_startup_world(Some(&path)).expect("authored loads");
        assert_eq!(world.start_room_id, "alpha");
        fs::remove_file(&path).ok();
    }

    // RT-4 (REQ-003, ticket #52): the migrated beginner world is 2D — every room
    // sits on the z=0 plane and no room carries an up/down exit key (the former
    // vertical traversal is now cardinal exits + warps).
    #[test]
    fn beginner_world_is_two_dimensional() {
        let world = load_beginner_world().expect("beginner module should load");
        for (id, room) in &world.rooms {
            assert_eq!(room.z, 0, "room '{id}' must be on the z=0 plane (2D)");
            assert!(
                !room.exits.contains_key("up") && !room.exits.contains_key("down"),
                "room '{id}' must have no up/down exit (cardinal-only)"
            );
        }
    }

    // RT-6 (REQ-005, ticket #52): a successful load is a materialize+validate (the
    // loader runs the core validator), and the former vertical tower climb is now a
    // straight cardinal run, with the two warps crossing a region boundary
    // (ashen_road -> tower_foot) and a sub-region boundary (tower_landing ->
    // bell_eater_roost).
    #[test]
    fn beginner_tower_climbs_north_via_cardinal_warps() {
        let world = load_beginner_world().expect("beginner module should load");
        let climb = [
            "hollowmere_square",
            "north_gate",
            "ashen_road",
            "tower_foot",
            "tower_landing",
            "bell_eater_roost",
        ];
        for (from, to) in climb.iter().zip(climb.iter().skip(1)) {
            let room = world
                .rooms
                .get(*from)
                .unwrap_or_else(|| panic!("room '{from}' should exist"));
            assert_eq!(
                room.exits.get("north").map(String::as_str),
                Some(*to),
                "'{from}' should reach '{to}' going north"
            );
        }

        let room = |id: &str| {
            world
                .rooms
                .get(id)
                .unwrap_or_else(|| panic!("room '{id}' should exist"))
        };
        // Region warp: ashen_road and tower_foot are in different regions.
        assert_ne!(
            room("ashen_road").region,
            room("tower_foot").region,
            "ashen_road -> tower_foot crosses a region boundary (warp)"
        );
        // Sub-region warp: same region, different sub-region.
        assert_eq!(
            room("tower_landing").region,
            room("bell_eater_roost").region,
            "tower_landing and bell_eater_roost share a region"
        );
        assert_ne!(
            room("tower_landing").subregion,
            room("bell_eater_roost").subregion,
            "tower_landing -> bell_eater_roost crosses a sub-region boundary (warp)"
        );
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

    // T10/#35 (REQ-006): the beginner gear loads its authored equipment by
    // value, and Mara's counter stocks the starter pieces.
    #[test]
    fn beginner_gear_loads_equipment_by_value() {
        use oathstar_core::{EquipSlot, EquipmentProfile};
        let world = load_beginner_world().expect("beginner module should load");
        let profile = |id: &str| {
            world
                .items
                .get(id)
                .unwrap_or_else(|| panic!("{id} item"))
                .equipment
        };
        assert_eq!(
            profile("rust_edge_blade"),
            Some(EquipmentProfile {
                slot: EquipSlot::Weapon,
                attack: 2,
                defense: 0,
            })
        );
        assert_eq!(
            profile("waxed_coat"),
            Some(EquipmentProfile {
                slot: EquipSlot::Armor,
                attack: 0,
                defense: 1,
            })
        );
        assert_eq!(
            profile("stray_fang"),
            Some(EquipmentProfile {
                slot: EquipSlot::Weapon,
                attack: 1,
                defense: 0,
            }),
            "the first drop is also the first weapon"
        );
        assert_eq!(
            profile("candle"),
            None,
            "non-gear items stay non-equippable"
        );
        let mara = world.entities.get("mara").expect("mara entity");
        for stocked in ["rust_edge_blade", "waxed_coat"] {
            assert!(
                mara.inventory.contains(&stocked.to_string()),
                "Mara stocks {stocked}"
            );
        }
    }

    // #35 (REQ-006): an unknown equipment slot fails the module at parse time
    // — the EquipSlot enum is the validation.
    #[test]
    fn load_rejects_an_unknown_equipment_slot() {
        let module = "id = \"m\"\nname = \"M\"\nstart_room_id = \"a\"\n";
        let world = "[[items]]\nid = \"hat\"\nname = \"Hat\"\ndescription = \"h\"\n\
            equipment = { slot = \"head\", defense = 1 }\n";
        let err = load_world_from_toml(module, ONE_ROOM, world)
            .expect_err("an unknown slot must be rejected");
        assert!(
            err.to_string().contains("invalid world TOML"),
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

    #[test]
    fn beginner_catalog_mirrors_the_world_content() {
        let catalog = beginner_catalog().expect("catalog loads");
        let world = load_beginner_world().expect("world loads");
        assert_eq!(catalog.entities, world.entities);
        assert_eq!(catalog.items, world.items);
        assert!(catalog.entities.contains_key("mara"));
        assert!(catalog.items.contains_key("candle"));
        assert!(catalog.fixtures.is_empty());
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

    // T9 (REQ-005, re-pinned at ticket #29): the Bell-Eater loads as
    // combatant + boss with the AUTHORED FIGHT stats by value — confront now
    // starts a real encounter against them — and the world validates.
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
                attack: 4,
                disclose_stats: false,
                xp: 25,
                coins: 25,
            }),
            "the boss fight stats load by value (ticket #29; stats stay hidden)"
        );
        assert_eq!(world.validate(), Ok(()));
    }

    // B11 (ticket #29): the beginner oath names the clapper as its authored
    // recoverable objective — taking it while sworn is what fulfills.
    #[test]
    fn beginner_oath_names_the_clapper_objective() {
        let world = load_beginner_world().expect("beginner module should load");
        let oath = world.oaths.get("hollow_bell").expect("hollow_bell oath");
        assert_eq!(
            oath.objective_item_id.as_deref(),
            Some("bell_clapper"),
            "the oath's objective is the stolen clapper"
        );
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
                disclose_stats: true,
                xp: 5,
                coins: 4,
            }),
            "authored combat stats load by value (ashen_stray discloses its stats; #26 adds the XP reward)"
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

    // X13 (ticket #26, REQ-008): the beginner reward loop is authored — the
    // stray carries the fang, the fang is a real registered item, and the
    // world still validates with the new authoring.
    #[test]
    fn beginner_stray_authors_the_reward_loop() {
        let world = load_beginner_world().expect("beginner module should load");
        let stray = world.entities.get("ashen_stray").expect("ashen_stray");
        assert_eq!(
            stray.inventory,
            vec!["stray_fang".to_string()],
            "the stray carries its drop"
        );
        let fang = world
            .items
            .get("stray_fang")
            .expect("stray_fang registered");
        assert_eq!(fang.name, "Cracked Fang");
        assert_eq!(fang.kind.as_deref(), Some("trophy"));
        assert!(
            fang.aliases.iter().any(|a| a == "fang"),
            "takeable by its short alias"
        );
        assert_eq!(world.validate(), Ok(()));
    }

    // N10 (ticket #27, REQ-006): the beginner oath authors exactly the two
    // bell announcements — the world alarm (delivered in play) and the
    // hollowmere notice (the staged not-delivered arm) — by value.
    #[test]
    fn beginner_oath_authors_the_bell_announcements() {
        let world = load_beginner_world().expect("beginner module should load");
        let oath = world.oaths.get("hollow_bell").expect("hollow_bell oath");
        assert_eq!(oath.fulfillment_announcements.len(), 2);

        let alarm = &oath.fulfillment_announcements[0];
        assert_eq!(alarm.scope, oathstar_core::AnnouncementScope::World);
        assert_eq!(alarm.severity, oathstar_core::AnnouncementSeverity::Alarm);
        assert_eq!(
            alarm.text,
            "The bell of Hollowmere rings again. Its voice rolls out over every road and roof."
        );

        let notice = &oath.fulfillment_announcements[1];
        assert_eq!(
            notice.scope,
            oathstar_core::AnnouncementScope::Region("hollowmere".to_string())
        );
        assert_eq!(notice.severity, oathstar_core::AnnouncementSeverity::Notice);
        assert_eq!(
            notice.text,
            "Hollowmere's streets fill with voices as the bell's song returns."
        );
        assert_eq!(world.validate(), Ok(()));
    }
}
