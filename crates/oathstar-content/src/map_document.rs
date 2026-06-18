//! The Oathstar-native map document model.
//!
//! A renderer-agnostic authoring document on a source-tile grid that the
//! studio editor writes and the server [`validate`](MapDocument::validate)s and
//! [`materialize`](MapDocument::materialize)s into engine-compatible
//! [`WorldDefinition`] data. The document is the authoring artifact, not the
//! runtime map snapshot; tile geometry is expressed in source-tile units,
//! independent of any client render scale.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use oathstar_core::{
    Entity, Item, RegionDefinition, RoomDefinition, SubregionDefinition, WorldDefinition,
    WorldValidationError,
};
use serde::{Deserialize, Serialize};

/// The source tile sizes (in pixels) a map document may declare.
///
/// The author's tile sheet is drawn at one of these — the art unit only; how
/// large a cell renders is a separate client knob (Decision 059).
pub const SUPPORTED_TILE_SIZES: [u32; 3] = [8, 16, 32];

/// Title given to an ordinary room cell that does not override it.
const DEFAULT_ROOM_TITLE: &str = "Unnamed Room";
/// Long description given to an ordinary room cell that does not override it.
const DEFAULT_ROOM_DESCRIPTION: &str = "An unremarkable area.";
/// Map glyph used when a room cell does not specify one.
const DEFAULT_ROOM_GLYPH: char = '.';

/// A tile-grid coordinate, in source-tile units (never pixels).
///
/// Derives a total order so coordinate-keyed maps iterate deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Cell {
    /// Column, 0-based from the left.
    pub x: i32,
    /// Row, 0-based from the top.
    pub y: i32,
    /// Floor / z-plane, 0-based.
    pub z: i32,
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// A terrain palette entry — a named tile and whether it can be stood on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainDef {
    /// The tileset tile name this terrain paints (name-keyed; pixel resolution
    /// lives in the renderer, not here).
    pub tile: String,
    /// Whether a creature may occupy this terrain.
    pub passable: bool,
}

/// A painted terrain cell — a coordinate and the palette terrain at it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainCell {
    /// Column.
    pub x: i32,
    /// Row.
    pub y: i32,
    /// Floor / z-plane.
    pub z: i32,
    /// Key into [`MapDocument::terrain_palette`].
    pub terrain: String,
}

impl TerrainCell {
    /// The coordinate of this terrain cell.
    const fn cell(&self) -> Cell {
        Cell {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

/// An authored room placed on a single grid cell.
///
/// An *ordinary* room leaves `title`/`description` unset and takes engine
/// defaults at materialization; a *special* room overrides them. The `id` is
/// always explicit and stable so exits can target it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomCell {
    /// Column.
    pub x: i32,
    /// Row.
    pub y: i32,
    /// Floor / z-plane.
    pub z: i32,
    /// Stable room id; exit targets reference it.
    pub id: String,
    /// Title override; `None` materializes the ordinary-room default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Long-description override; `None` materializes the ordinary-room default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Region id this room belongs to; must be declared in the document.
    pub region: String,
    /// Optional subregion id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subregion: Option<String>,
    /// Map glyph override; `None` materializes [`DEFAULT_ROOM_GLYPH`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glyph: Option<char>,
    /// Whether combat may start here.
    #[serde(default)]
    pub combat_enabled: bool,
    /// Exits by direction name (`"north"`…`"west"`) to a target room id.
    #[serde(default)]
    pub exits: BTreeMap<String, String>,
    /// Ids of entities placed in this room (validated against the catalog).
    #[serde(default)]
    pub entities: Vec<String>,
    /// Ids of items placed in this room (validated against the catalog).
    #[serde(default)]
    pub items: Vec<String>,
    /// Ids of fixtures referenced here — validated against the catalog but not
    /// yet materialized (the engine has no fixture concept).
    #[serde(default)]
    pub fixtures: Vec<String>,
}

impl RoomCell {
    /// The coordinate of this room cell.
    const fn cell(&self) -> Cell {
        Cell {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

/// A registered tile sheet, sliced into `columns * rows` tiles.
///
/// The editor slices a raw sheet at `tile_size` into this grid; `tiles` carries
/// optional metadata only for the tiles that need it. Authoring-visual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tileset {
    /// Stable id; layer cells reference it.
    pub id: String,
    /// Sheet image path (resolved by the renderer, not here).
    pub image: String,
    /// Source tile edge in pixels; one of [`SUPPORTED_TILE_SIZES`].
    pub tile_size: u32,
    /// Sheet width in tiles.
    pub columns: u32,
    /// Sheet height in tiles.
    pub rows: u32,
    /// Sparse per-tile metadata, keyed by tile index (`0..columns*rows`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tiles: BTreeMap<u32, TileMeta>,
}

/// Optional authoring metadata for one tile of a [`Tileset`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileMeta {
    /// Human-facing tile name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether a creature may stand on this tile (authoring hint).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passable: Option<bool>,
    /// Free-form tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// The role a tile layer plays.
///
/// One generic kind for now; the editor paints onto `Tile` layers. Future kinds
/// (ground / decoration / overlay) extend this enum.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    /// A generic paintable tile layer.
    #[default]
    Tile,
}

/// A painted tile on a layer — a coordinate plus the tile it shows.
///
/// Flattened (not a `Cell`-keyed map) because a struct cannot be a JSON map key;
/// mirrors [`TerrainCell`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerCell {
    /// Column.
    pub x: i32,
    /// Row.
    pub y: i32,
    /// Floor / z-plane.
    pub z: i32,
    /// Id of the [`Tileset`] the tile comes from.
    pub tileset: String,
    /// Tile index within that tileset (`0..columns*rows`).
    pub index: u32,
}

impl LayerCell {
    /// The coordinate of this layer cell.
    const fn cell(&self) -> Cell {
        Cell {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

/// A named, stackable layer of painted tiles.
///
/// Authoring-visual: validated, but not yet materialized into the runtime world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    /// Stable layer id (unique within the document).
    pub id: String,
    /// Human-facing layer name.
    pub name: String,
    /// What the layer is for.
    #[serde(default)]
    pub kind: LayerKind,
    /// Whether the layer is drawn (an authoring toggle).
    #[serde(default = "default_true")]
    pub visible: bool,
    /// Painted cells; a coordinate may be painted at most once per layer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<LayerCell>,
}

/// Serde default for [`Layer::visible`] — a new layer is visible.
const fn default_true() -> bool {
    true
}

/// A complete map authoring document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapDocument {
    /// Map id (becomes the materialized world id).
    pub id: String,
    /// Human-facing map title.
    pub title: String,
    /// Source tile size in pixels — must be one of [`SUPPORTED_TILE_SIZES`].
    pub tile_size: u32,
    /// Grid width in tiles.
    pub width: u32,
    /// Grid height in tiles.
    pub height: u32,
    /// Number of floors / z-planes.
    pub floors: u32,
    /// Named terrain palette (terrain name → definition).
    #[serde(default)]
    pub terrain_palette: BTreeMap<String, TerrainDef>,
    /// Painted terrain cells.
    #[serde(default)]
    pub terrain: Vec<TerrainCell>,
    /// Region metadata, by id.
    #[serde(default)]
    pub regions: BTreeMap<String, RegionDefinition>,
    /// Subregion metadata, by id.
    #[serde(default)]
    pub subregions: BTreeMap<String, SubregionDefinition>,
    /// Authored room cells.
    #[serde(default)]
    pub rooms: Vec<RoomCell>,
    /// The spawn / start cell; must land on a room cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawn: Option<Cell>,
    /// Registered tile sheets that the layers paint from.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tilesets: Vec<Tileset>,
    /// Painted tile layers (authoring-visual; not yet materialized).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<Layer>,
}

/// The content a map document validates and materializes against.
///
/// `entities` and `items` supply both the existence check and the definitions
/// copied into the materialized world; `fixtures` is an id set only (fixtures
/// are validated but not yet materialized).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentCatalog {
    /// Known entity definitions, by id.
    pub entities: BTreeMap<String, Entity>,
    /// Known item definitions, by id.
    pub items: BTreeMap<String, Item>,
    /// Known fixture ids.
    pub fixtures: BTreeSet<String>,
}

/// Which catalog an [`MapValidationError::UnknownReference`] failed against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RefKind {
    /// An entity placement.
    Entity,
    /// An item placement.
    Item,
    /// A fixture reference.
    Fixture,
}

impl fmt::Display for RefKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Entity => "entity",
            Self::Item => "item",
            Self::Fixture => "fixture",
        };
        f.write_str(label)
    }
}

/// Why a map document failed validation or materialization.
///
/// Each variant names the offending cell and/or reference so an editor can point
/// the author at it. Mirrors the engine's hand-rolled error style.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MapValidationError {
    /// `tile_size` is not one of [`SUPPORTED_TILE_SIZES`].
    UnsupportedTileSize {
        /// The unsupported size found.
        found: u32,
    },
    /// A terrain, room, or layer cell sits outside the declared grid bounds.
    CellOutOfBounds {
        /// The offending cell.
        cell: Cell,
    },
    /// A terrain cell names a terrain that is not in the palette.
    UnknownTerrain {
        /// The offending cell.
        cell: Cell,
        /// The unknown terrain name.
        name: String,
    },
    /// Two terrain cells, two room cells, or two cells of one layer occupy the
    /// same coordinate.
    DuplicateCell {
        /// The doubly-occupied cell.
        cell: Cell,
    },
    /// A room cell has no terrain painted under it.
    MissingTerrain {
        /// The room cell lacking terrain.
        cell: Cell,
    },
    /// A room cell sits on non-passable terrain.
    RoomOnBlockedTerrain {
        /// The offending cell.
        cell: Cell,
        /// The room id at that cell.
        room_id: String,
        /// The non-passable terrain name.
        terrain: String,
    },
    /// Two room cells share a room id.
    DuplicateRoomId {
        /// The second cell carrying the id.
        cell: Cell,
        /// The duplicated room id.
        room_id: String,
    },
    /// A room references a region that the document does not declare.
    RoomRegionMissing {
        /// The room cell.
        cell: Cell,
        /// The room id.
        room_id: String,
        /// The undeclared region id.
        region: String,
    },
    /// A room references a subregion that the document does not declare.
    RoomSubregionMissing {
        /// The room cell.
        cell: Cell,
        /// The room id.
        room_id: String,
        /// The undeclared subregion id.
        subregion: String,
    },
    /// A room exit targets a room id that does not exist.
    DanglingExit {
        /// The room the exit leaves from.
        room_id: String,
        /// The exit direction.
        direction: String,
        /// The missing target room id.
        target_room_id: String,
    },
    /// A room places or references a content id not in the catalog.
    UnknownReference {
        /// The room id.
        room_id: String,
        /// The room cell.
        cell: Cell,
        /// Which catalog the lookup failed against.
        kind: RefKind,
        /// The unknown id.
        id: String,
    },
    /// The document declares no spawn point.
    NoSpawnPoint,
    /// The spawn point does not land on a room cell.
    SpawnNotARoom {
        /// The spawn cell.
        cell: Cell,
    },
    /// Two tilesets share an id.
    DuplicateTilesetId {
        /// The duplicated tileset id.
        id: String,
    },
    /// A tileset declares an unsupported tile size or empty dimensions.
    UnsupportedTilesetGeometry {
        /// The offending tileset id.
        id: String,
    },
    /// Tileset metadata names a tile index outside the sheet.
    TileMetaIndexOutOfRange {
        /// The tileset id.
        tileset: String,
        /// The out-of-range index.
        index: u32,
    },
    /// Two layers share an id.
    DuplicateLayerId {
        /// The duplicated layer id.
        id: String,
    },
    /// A layer cell references a tileset the document does not declare.
    UnknownTilesetRef {
        /// The layer id.
        layer: String,
        /// The offending cell.
        cell: Cell,
        /// The unknown tileset id.
        tileset: String,
    },
    /// A layer cell references a tile index outside its tileset.
    TileIndexOutOfRange {
        /// The layer id.
        layer: String,
        /// The offending cell.
        cell: Cell,
        /// The out-of-range index.
        index: u32,
    },
    /// The materialized world failed the engine's own invariant check.
    WorldInvalid(#[serde(serialize_with = "serialize_world_error")] WorldValidationError),
}

impl fmt::Display for MapValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTileSize { found } => {
                write!(
                    f,
                    "tile size {found} is unsupported (expected one of {SUPPORTED_TILE_SIZES:?})"
                )
            }
            Self::CellOutOfBounds { cell } => write!(f, "cell {cell} is out of bounds"),
            Self::UnknownTerrain { cell, name } => {
                write!(f, "cell {cell} uses undeclared terrain '{name}'")
            }
            Self::DuplicateCell { cell } => write!(f, "cell {cell} is occupied twice"),
            Self::MissingTerrain { cell } => write!(f, "room cell {cell} has no terrain"),
            Self::RoomOnBlockedTerrain {
                cell,
                room_id,
                terrain,
            } => write!(
                f,
                "room '{room_id}' at {cell} sits on non-passable terrain '{terrain}'"
            ),
            Self::DuplicateRoomId { cell, room_id } => {
                write!(f, "room id '{room_id}' is reused at cell {cell}")
            }
            Self::RoomRegionMissing {
                cell,
                room_id,
                region,
            } => write!(
                f,
                "room '{room_id}' at {cell} references undeclared region '{region}'"
            ),
            Self::RoomSubregionMissing {
                cell,
                room_id,
                subregion,
            } => write!(
                f,
                "room '{room_id}' at {cell} references undeclared subregion '{subregion}'"
            ),
            Self::DanglingExit {
                room_id,
                direction,
                target_room_id,
            } => write!(
                f,
                "room '{room_id}' exit '{direction}' targets missing room '{target_room_id}'"
            ),
            Self::UnknownReference {
                room_id,
                cell,
                kind,
                id,
            } => write!(
                f,
                "room '{room_id}' at {cell} references unknown {kind} '{id}'"
            ),
            Self::NoSpawnPoint => write!(f, "the map declares no spawn point"),
            Self::SpawnNotARoom { cell } => write!(f, "spawn cell {cell} is not a room"),
            Self::DuplicateTilesetId { id } => write!(f, "tileset id '{id}' is reused"),
            Self::UnsupportedTilesetGeometry { id } => write!(
                f,
                "tileset '{id}' has unsupported geometry (tile size must be one of {SUPPORTED_TILE_SIZES:?}, columns and rows must be positive)"
            ),
            Self::TileMetaIndexOutOfRange { tileset, index } => write!(
                f,
                "tileset '{tileset}' metadata names out-of-range tile index {index}"
            ),
            Self::DuplicateLayerId { id } => write!(f, "layer id '{id}' is reused"),
            Self::UnknownTilesetRef {
                layer,
                cell,
                tileset,
            } => write!(
                f,
                "layer '{layer}' cell {cell} references unknown tileset '{tileset}'"
            ),
            Self::TileIndexOutOfRange { layer, cell, index } => write!(
                f,
                "layer '{layer}' cell {cell} references out-of-range tile index {index}"
            ),
            Self::WorldInvalid(err) => write!(f, "materialized world is invalid: {err}"),
        }
    }
}

impl std::error::Error for MapValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::WorldInvalid(err) => Some(err),
            _ => None,
        }
    }
}

/// Serialize a wrapped [`WorldValidationError`] as its Display string.
///
/// The engine error is not `Serialize`; its `Display` already names the offending
/// item, which is what an editor needs.
fn serialize_world_error<S: serde::Serializer>(
    error: &WorldValidationError,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&error.to_string())
}

impl MapDocument {
    /// Whether `cell` lies within the declared grid bounds.
    fn in_bounds(&self, cell: Cell) -> bool {
        cell.x >= 0
            && cell.y >= 0
            && cell.z >= 0
            && i64::from(cell.x) < i64::from(self.width)
            && i64::from(cell.y) < i64::from(self.height)
            && i64::from(cell.z) < i64::from(self.floors)
    }

    /// Validate the tileset registry.
    ///
    /// Returns each tileset id mapped to its tile capacity (`columns*rows`, as
    /// `u64` to avoid overflow) so the layer pass can range-check tile indices.
    fn check_tilesets(&self) -> Result<BTreeMap<&str, u64>, MapValidationError> {
        let mut tileset_capacity: BTreeMap<&str, u64> = BTreeMap::new();
        for ts in &self.tilesets {
            if !SUPPORTED_TILE_SIZES.contains(&ts.tile_size) || ts.columns == 0 || ts.rows == 0 {
                return Err(MapValidationError::UnsupportedTilesetGeometry { id: ts.id.clone() });
            }
            let capacity = u64::from(ts.columns) * u64::from(ts.rows);
            for &index in ts.tiles.keys() {
                if u64::from(index) >= capacity {
                    return Err(MapValidationError::TileMetaIndexOutOfRange {
                        tileset: ts.id.clone(),
                        index,
                    });
                }
            }
            if tileset_capacity.insert(ts.id.as_str(), capacity).is_some() {
                return Err(MapValidationError::DuplicateTilesetId { id: ts.id.clone() });
            }
        }
        Ok(tileset_capacity)
    }

    /// Validate the tile layers against the tileset capacities.
    ///
    /// Authoring-visual: layers are validated but never reach `build_world`, so
    /// the runtime world is unaffected by their presence.
    fn check_layers(
        &self,
        tileset_capacity: &BTreeMap<&str, u64>,
    ) -> Result<(), MapValidationError> {
        let mut layer_ids: BTreeSet<&str> = BTreeSet::new();
        for layer in &self.layers {
            if !layer_ids.insert(layer.id.as_str()) {
                return Err(MapValidationError::DuplicateLayerId {
                    id: layer.id.clone(),
                });
            }
            let mut painted: BTreeSet<Cell> = BTreeSet::new();
            for lc in &layer.cells {
                let cell = lc.cell();
                if !self.in_bounds(cell) {
                    return Err(MapValidationError::CellOutOfBounds { cell });
                }
                if !painted.insert(cell) {
                    return Err(MapValidationError::DuplicateCell { cell });
                }
                let Some(&capacity) = tileset_capacity.get(lc.tileset.as_str()) else {
                    return Err(MapValidationError::UnknownTilesetRef {
                        layer: layer.id.clone(),
                        cell,
                        tileset: lc.tileset.clone(),
                    });
                };
                if u64::from(lc.index) >= capacity {
                    return Err(MapValidationError::TileIndexOutOfRange {
                        layer: layer.id.clone(),
                        cell,
                        index: lc.index,
                    });
                }
            }
        }
        Ok(())
    }

    /// Validate the document against `catalog`, returning the resolved start room
    /// id used by [`materialize`](Self::materialize).
    fn check(&self, catalog: &ContentCatalog) -> Result<String, MapValidationError> {
        if !SUPPORTED_TILE_SIZES.contains(&self.tile_size) {
            return Err(MapValidationError::UnsupportedTileSize {
                found: self.tile_size,
            });
        }

        let tileset_capacity = self.check_tilesets()?;
        self.check_layers(&tileset_capacity)?;

        // Terrain layer: every cell in bounds, palette-known, and unique. Store the
        // resolved (name, passable) so the room loop needs no second palette lookup
        // (which would be an unreachable, unkillable branch).
        let mut terrain_at: BTreeMap<Cell, (&str, bool)> = BTreeMap::new();
        for tc in &self.terrain {
            let cell = tc.cell();
            if !self.in_bounds(cell) {
                return Err(MapValidationError::CellOutOfBounds { cell });
            }
            let Some(def) = self.terrain_palette.get(&tc.terrain) else {
                return Err(MapValidationError::UnknownTerrain {
                    cell,
                    name: tc.terrain.clone(),
                });
            };
            if terrain_at
                .insert(cell, (tc.terrain.as_str(), def.passable))
                .is_some()
            {
                return Err(MapValidationError::DuplicateCell { cell });
            }
        }

        // Room layer: bounds, unique cell + id, declared region, passable terrain,
        // and known content references.
        let mut room_id_at: BTreeMap<Cell, String> = BTreeMap::new();
        let mut room_ids: BTreeSet<&str> = BTreeSet::new();
        for rc in &self.rooms {
            let cell = rc.cell();
            if !self.in_bounds(cell) {
                return Err(MapValidationError::CellOutOfBounds { cell });
            }
            if room_id_at.insert(cell, rc.id.clone()).is_some() {
                return Err(MapValidationError::DuplicateCell { cell });
            }
            if !room_ids.insert(rc.id.as_str()) {
                return Err(MapValidationError::DuplicateRoomId {
                    cell,
                    room_id: rc.id.clone(),
                });
            }
            if !self.regions.contains_key(&rc.region) {
                return Err(MapValidationError::RoomRegionMissing {
                    cell,
                    room_id: rc.id.clone(),
                    region: rc.region.clone(),
                });
            }
            if let Some(subregion) = &rc.subregion {
                if !self.subregions.contains_key(subregion) {
                    return Err(MapValidationError::RoomSubregionMissing {
                        cell,
                        room_id: rc.id.clone(),
                        subregion: subregion.clone(),
                    });
                }
            }
            let Some((terrain_name, passable)) = terrain_at.get(&cell).copied() else {
                return Err(MapValidationError::MissingTerrain { cell });
            };
            if !passable {
                return Err(MapValidationError::RoomOnBlockedTerrain {
                    cell,
                    room_id: rc.id.clone(),
                    terrain: terrain_name.to_owned(),
                });
            }
            check_refs(rc, cell, catalog)?;
        }

        // Exits resolve to a known room id (rooms may be forward-referenced).
        for rc in &self.rooms {
            for (direction, target) in &rc.exits {
                if !room_ids.contains(target.as_str()) {
                    return Err(MapValidationError::DanglingExit {
                        room_id: rc.id.clone(),
                        direction: direction.clone(),
                        target_room_id: target.clone(),
                    });
                }
            }
        }

        // Spawn must exist and land on a room cell; that room is the start room.
        let Some(spawn) = self.spawn else {
            return Err(MapValidationError::NoSpawnPoint);
        };
        let Some(start_room_id) = room_id_at.get(&spawn).cloned() else {
            return Err(MapValidationError::SpawnNotARoom { cell: spawn });
        };
        Ok(start_room_id)
    }

    /// Validate the document against a content catalog.
    ///
    /// # Errors
    /// Returns the first [`MapValidationError`] encountered, including
    /// [`MapValidationError::WorldInvalid`] if the materialized world would fail
    /// the engine's own invariant check.
    pub fn validate(&self, catalog: &ContentCatalog) -> Result<(), MapValidationError> {
        self.materialize(catalog).map(|_| ())
    }

    /// Materialize the document into an engine-compatible [`WorldDefinition`].
    ///
    /// Deterministic: the same document and catalog always produce byte-identical
    /// world data.
    ///
    /// # Errors
    /// Returns a [`MapValidationError`] if the document fails validation or the
    /// resulting world fails [`WorldDefinition::validate`].
    pub fn materialize(
        &self,
        catalog: &ContentCatalog,
    ) -> Result<WorldDefinition, MapValidationError> {
        let start_room_id = self.check(catalog)?;
        let world = self.build_world(start_room_id, catalog);
        world.validate().map_err(MapValidationError::WorldInvalid)?;
        Ok(world)
    }

    /// Build the world from a checked document (rooms are all passable by
    /// construction — a non-passable cell is never a room).
    fn build_world(&self, start_room_id: String, catalog: &ContentCatalog) -> WorldDefinition {
        let mut rooms: BTreeMap<String, RoomDefinition> = BTreeMap::new();
        let mut referenced_entities: BTreeSet<&str> = BTreeSet::new();
        let mut referenced_items: BTreeSet<&str> = BTreeSet::new();

        for rc in &self.rooms {
            for id in &rc.entities {
                referenced_entities.insert(id.as_str());
            }
            for id in &rc.items {
                referenced_items.insert(id.as_str());
            }
            let cell = rc.cell();
            rooms.insert(
                rc.id.clone(),
                RoomDefinition {
                    id: rc.id.clone(),
                    title: rc
                        .title
                        .clone()
                        .unwrap_or_else(|| DEFAULT_ROOM_TITLE.to_owned()),
                    region: rc.region.clone(),
                    subregion: rc.subregion.clone(),
                    description: rc
                        .description
                        .clone()
                        .unwrap_or_else(|| DEFAULT_ROOM_DESCRIPTION.to_owned()),
                    exits: rc.exits.clone(),
                    x: cell.x,
                    y: cell.y,
                    z: cell.z,
                    glyph: rc.glyph.unwrap_or(DEFAULT_ROOM_GLYPH),
                    passable: true,
                    entities: rc.entities.clone(),
                    items: rc.items.clone(),
                    combat_enabled: rc.combat_enabled,
                },
            );
        }

        // World entities are the placed ones; iterating the catalog (rather than
        // looking each id up) keeps both filter branches reachable for testing. A
        // placed entity's inventory items must travel with it (the engine checks
        // entity→item references), so fold them into the item set here.
        let mut entities: BTreeMap<String, Entity> = BTreeMap::new();
        for (id, entity) in &catalog.entities {
            if referenced_entities.contains(id.as_str()) {
                for item_id in &entity.inventory {
                    referenced_items.insert(item_id.as_str());
                }
                entities.insert(id.clone(), entity.clone());
            }
        }

        let items: BTreeMap<String, Item> = catalog
            .items
            .iter()
            .filter(|(id, _)| referenced_items.contains(id.as_str()))
            .map(|(id, item)| (id.clone(), item.clone()))
            .collect();

        WorldDefinition {
            id: self.id.clone(),
            title: self.title.clone(),
            start_room_id,
            rooms,
            regions: self.regions.clone(),
            subregions: self.subregions.clone(),
            entities,
            items,
            oaths: BTreeMap::new(),
            oath_id: None,
        }
    }
}

/// Check a room cell's entity/item/fixture references against the catalog.
fn check_refs(
    rc: &RoomCell,
    cell: Cell,
    catalog: &ContentCatalog,
) -> Result<(), MapValidationError> {
    for id in &rc.entities {
        if !catalog.entities.contains_key(id) {
            return Err(MapValidationError::UnknownReference {
                room_id: rc.id.clone(),
                cell,
                kind: RefKind::Entity,
                id: id.clone(),
            });
        }
    }
    for id in &rc.items {
        if !catalog.items.contains_key(id) {
            return Err(MapValidationError::UnknownReference {
                room_id: rc.id.clone(),
                cell,
                kind: RefKind::Item,
                id: id.clone(),
            });
        }
    }
    for id in &rc.fixtures {
        if !catalog.fixtures.contains(id) {
            return Err(MapValidationError::UnknownReference {
                room_id: rc.id.clone(),
                cell,
                kind: RefKind::Fixture,
                id: id.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        Cell, ContentCatalog, Layer, LayerCell, LayerKind, MapDocument, MapValidationError,
        RefKind, RoomCell, TerrainCell, TerrainDef, TileMeta, Tileset, SUPPORTED_TILE_SIZES,
    };
    use oathstar_core::{
        Engine, Entity, EntityKind, Item, RegionDefinition, SubregionDefinition,
        WorldValidationError,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn palette() -> BTreeMap<String, TerrainDef> {
        BTreeMap::from([
            (
                "floor".to_owned(),
                TerrainDef {
                    tile: "stone_floor".to_owned(),
                    passable: true,
                },
            ),
            (
                "wall".to_owned(),
                TerrainDef {
                    tile: "wall_face".to_owned(),
                    passable: false,
                },
            ),
        ])
    }

    fn regions() -> BTreeMap<String, RegionDefinition> {
        BTreeMap::from([(
            "reg".to_owned(),
            RegionDefinition {
                id: "reg".to_owned(),
                name: "Region".to_owned(),
                description: String::new(),
            },
        )])
    }

    fn terrain(x: i32, y: i32, z: i32, name: &str) -> TerrainCell {
        TerrainCell {
            x,
            y,
            z,
            terrain: name.to_owned(),
        }
    }

    fn room(x: i32, y: i32, z: i32, id: &str) -> RoomCell {
        RoomCell {
            x,
            y,
            z,
            id: id.to_owned(),
            title: None,
            description: None,
            region: "reg".to_owned(),
            subregion: None,
            glyph: None,
            combat_enabled: false,
            exits: BTreeMap::new(),
            entities: Vec::new(),
            items: Vec::new(),
            fixtures: Vec::new(),
        }
    }

    fn entity(id: &str) -> Entity {
        Entity {
            id: id.to_owned(),
            name: id.to_owned(),
            description: String::new(),
            aliases: Vec::new(),
            kind: EntityKind::Actor,
            roles: Vec::new(),
            inventory: Vec::new(),
            hidden: false,
            dialogue: None,
            combat: None,
        }
    }

    fn item(id: &str) -> Item {
        Item {
            id: id.to_owned(),
            name: id.to_owned(),
            description: String::new(),
            aliases: Vec::new(),
            hidden: false,
            kind: None,
            flags: Vec::new(),
            value: 0,
            equipment: None,
        }
    }

    /// A minimal valid document: one passable `floor` room at the origin, spawned.
    fn valid_doc() -> MapDocument {
        MapDocument {
            id: "map1".to_owned(),
            title: "Test Map".to_owned(),
            tile_size: 16,
            width: 4,
            height: 4,
            floors: 2,
            terrain_palette: palette(),
            terrain: vec![terrain(0, 0, 0, "floor")],
            regions: regions(),
            subregions: BTreeMap::new(),
            rooms: vec![room(0, 0, 0, "start")],
            spawn: Some(Cell { x: 0, y: 0, z: 0 }),
            tilesets: Vec::new(),
            layers: Vec::new(),
        }
    }

    fn empty_catalog() -> ContentCatalog {
        ContentCatalog::default()
    }

    // ---- REQ-001: representation of every facet ----

    #[test]
    fn represents_all_facets() {
        // T1: identity, grid size, floors, region+subregion, terrain+passability,
        // room cells, exits, spawn, and entity/item/fixture references.
        let mut doc = valid_doc();
        doc.subregions = BTreeMap::from([(
            "sub".to_owned(),
            SubregionDefinition {
                id: "sub".to_owned(),
                name: "Sub".to_owned(),
                region: "reg".to_owned(),
                description: String::new(),
            },
        )]);
        doc.terrain.push(terrain(1, 0, 0, "floor"));
        let mut start = room(0, 0, 0, "start");
        start.subregion = Some("sub".to_owned());
        start.exits = BTreeMap::from([("east".to_owned(), "hall".to_owned())]);
        start.entities = vec!["guard".to_owned()];
        start.items = vec!["key".to_owned()];
        start.fixtures = vec!["statue".to_owned()];
        doc.rooms = vec![start, room(1, 0, 0, "hall")];

        let catalog = ContentCatalog {
            entities: BTreeMap::from([("guard".to_owned(), entity("guard"))]),
            items: BTreeMap::from([("key".to_owned(), item("key"))]),
            fixtures: BTreeSet::from(["statue".to_owned()]),
        };

        let world = doc.materialize(&catalog).expect("rich doc materializes");
        assert_eq!(world.id, "map1");
        assert_eq!(world.start_room_id, "start");
        assert!(world.regions.contains_key("reg"));
        assert!(world.subregions.contains_key("sub"));
        let hall = world.rooms.get("hall").expect("hall room exists");
        assert_eq!(hall.x, 1);
        let start_room = world.rooms.get("start").expect("start room exists");
        assert_eq!(start_room.exits.get("east"), Some(&"hall".to_owned()));
        assert_eq!(start_room.entities, vec!["guard".to_owned()]);
        assert_eq!(start_room.items, vec!["key".to_owned()]);
        // Fixtures are validated but not materialized (no engine target).
        assert!(world.entities.contains_key("guard"));
        assert!(world.items.contains_key("key"));
    }

    // ---- REQ-002: supported source tile sizes ----

    #[test]
    fn supported_tile_sizes_are_8_16_32() {
        assert_eq!(SUPPORTED_TILE_SIZES, [8, 16, 32]);
    }

    #[test]
    fn every_supported_tile_size_validates() {
        for size in SUPPORTED_TILE_SIZES {
            let mut doc = valid_doc();
            doc.tile_size = size;
            assert_eq!(
                doc.validate(&empty_catalog()),
                Ok(()),
                "tile_size {size} should validate"
            );
        }
    }

    #[test]
    fn unsupported_tile_sizes_are_refused() {
        for size in [0_u32, 7, 24, 64] {
            let mut doc = valid_doc();
            doc.tile_size = size;
            assert_eq!(
                doc.validate(&empty_catalog()),
                Err(MapValidationError::UnsupportedTileSize { found: size }),
                "tile_size {size} should be refused"
            );
        }
    }

    // ---- REQ-003: deterministic, engine-compatible materialization ----

    #[test]
    fn materialize_yields_engine_constructable_world() {
        let world = valid_doc()
            .materialize(&empty_catalog())
            .expect("valid doc materializes");
        let room = world.rooms.get("start").expect("start room");
        assert_eq!(room.id, "start");
        assert_eq!((room.x, room.y, room.z), (0, 0, 0));
        assert!(room.passable);
        Engine::try_new(world).expect("materialized world builds an engine");
    }

    #[test]
    fn materialized_room_keeps_authored_coordinates() {
        let mut doc = valid_doc();
        doc.terrain = vec![terrain(2, 3, 1, "floor")];
        doc.rooms = vec![room(2, 3, 1, "here")];
        doc.spawn = Some(Cell { x: 2, y: 3, z: 1 });
        let world = doc.materialize(&empty_catalog()).expect("materializes");
        let here = world.rooms.get("here").expect("room here");
        assert_eq!((here.x, here.y, here.z), (2, 3, 1));
    }

    #[test]
    fn materialize_is_deterministic_regardless_of_input_order() {
        let mut doc = valid_doc();
        doc.terrain = vec![terrain(0, 0, 0, "floor"), terrain(1, 0, 0, "floor")];
        doc.rooms = vec![room(0, 0, 0, "a"), room(1, 0, 0, "b")];
        doc.spawn = Some(Cell { x: 0, y: 0, z: 0 });

        let mut reversed = doc.clone();
        reversed.terrain.reverse();
        reversed.rooms.reverse();

        let w1 = doc.materialize(&empty_catalog()).expect("materializes");
        let w2 = reversed
            .materialize(&empty_catalog())
            .expect("materializes");
        let j1 = serde_json::to_string(&w1).expect("serializes");
        let j2 = serde_json::to_string(&w2).expect("serializes");
        assert_eq!(j1, j2);
    }

    // ---- REQ-004: one isolated refusal per typed-error variant ----

    #[test]
    fn refuses_terrain_cell_out_of_bounds() {
        let mut doc = valid_doc();
        doc.terrain.push(terrain(4, 0, 0, "floor"));
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::CellOutOfBounds {
                cell: Cell { x: 4, y: 0, z: 0 }
            })
        );
    }

    #[test]
    fn refuses_room_cell_out_of_bounds() {
        // Drives the room-loop in_bounds call site (no terrain at the OOB cell).
        let mut doc = valid_doc();
        doc.rooms.push(room(4, 0, 0, "oob"));
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::CellOutOfBounds {
                cell: Cell { x: 4, y: 0, z: 0 }
            })
        );
    }

    #[test]
    fn in_bounds_boundary_matrix() {
        // One axis perturbed at a time. validate of a terrain-only doc returns
        // CellOutOfBounds for an out-of-range cell, NoSpawnPoint for an in-range one.
        let oob = |x, y, z| {
            let mut doc = valid_doc();
            doc.terrain = vec![terrain(x, y, z, "floor")];
            doc.rooms = Vec::new();
            doc.spawn = None;
            doc.validate(&empty_catalog())
        };
        for (x, y, z) in [
            (4, 0, 0),
            (-1, 0, 0),
            (0, 4, 0),
            (0, -1, 0),
            (0, 0, 2),
            (0, 0, -1),
        ] {
            assert_eq!(
                oob(x, y, z),
                Err(MapValidationError::CellOutOfBounds {
                    cell: Cell { x, y, z }
                }),
                "({x},{y},{z}) must be out of bounds"
            );
        }
        // width=4,height=4,floors=2 → max in-bounds is (3,3,1); origin is (0,0,0).
        for (x, y, z) in [(3, 3, 1), (0, 0, 0)] {
            assert_eq!(
                oob(x, y, z),
                Err(MapValidationError::NoSpawnPoint),
                "({x},{y},{z}) must be in bounds"
            );
        }
    }

    #[test]
    fn refuses_unknown_terrain() {
        let mut doc = valid_doc();
        doc.terrain = vec![terrain(0, 0, 0, "lava")];
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::UnknownTerrain {
                cell: Cell { x: 0, y: 0, z: 0 },
                name: "lava".to_owned()
            })
        );
    }

    #[test]
    fn refuses_duplicate_terrain_cell() {
        let mut doc = valid_doc();
        doc.terrain = vec![terrain(0, 0, 0, "floor"), terrain(0, 0, 0, "wall")];
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::DuplicateCell {
                cell: Cell { x: 0, y: 0, z: 0 }
            })
        );
    }

    #[test]
    fn refuses_duplicate_room_cell() {
        // Same cell, different ids → DuplicateCell fires before DuplicateRoomId.
        let mut doc = valid_doc();
        doc.rooms = vec![room(0, 0, 0, "a"), room(0, 0, 0, "b")];
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::DuplicateCell {
                cell: Cell { x: 0, y: 0, z: 0 }
            })
        );
    }

    #[test]
    fn refuses_room_without_terrain() {
        let mut doc = valid_doc();
        doc.rooms = vec![room(1, 1, 0, "floating")];
        doc.spawn = Some(Cell { x: 1, y: 1, z: 0 });
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::MissingTerrain {
                cell: Cell { x: 1, y: 1, z: 0 }
            })
        );
    }

    #[test]
    fn refuses_room_on_blocked_terrain() {
        let mut doc = valid_doc();
        doc.terrain = vec![terrain(0, 0, 0, "wall")];
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::RoomOnBlockedTerrain {
                cell: Cell { x: 0, y: 0, z: 0 },
                room_id: "start".to_owned(),
                terrain: "wall".to_owned()
            })
        );
    }

    #[test]
    fn refuses_duplicate_room_id() {
        // Different cells, same id → DuplicateRoomId (not DuplicateCell).
        let mut doc = valid_doc();
        doc.terrain = vec![terrain(0, 0, 0, "floor"), terrain(1, 0, 0, "floor")];
        doc.rooms = vec![room(0, 0, 0, "dup"), room(1, 0, 0, "dup")];
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::DuplicateRoomId {
                cell: Cell { x: 1, y: 0, z: 0 },
                room_id: "dup".to_owned()
            })
        );
    }

    #[test]
    fn refuses_undeclared_region() {
        let mut doc = valid_doc();
        let mut r = room(0, 0, 0, "start");
        r.region = "nowhere".to_owned();
        doc.rooms = vec![r];
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::RoomRegionMissing {
                cell: Cell { x: 0, y: 0, z: 0 },
                room_id: "start".to_owned(),
                region: "nowhere".to_owned()
            })
        );
    }

    #[test]
    fn refuses_undeclared_subregion() {
        let mut doc = valid_doc();
        let mut r = room(0, 0, 0, "start");
        r.subregion = Some("ghost".to_owned());
        doc.rooms = vec![r];
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::RoomSubregionMissing {
                cell: Cell { x: 0, y: 0, z: 0 },
                room_id: "start".to_owned(),
                subregion: "ghost".to_owned()
            })
        );
    }

    #[test]
    fn refuses_dangling_exit() {
        let mut doc = valid_doc();
        let mut r = room(0, 0, 0, "start");
        r.exits = BTreeMap::from([("north".to_owned(), "void".to_owned())]);
        doc.rooms = vec![r];
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::DanglingExit {
                room_id: "start".to_owned(),
                direction: "north".to_owned(),
                target_room_id: "void".to_owned()
            })
        );
    }

    #[test]
    fn forward_referenced_exit_validates() {
        let mut doc = valid_doc();
        doc.terrain = vec![terrain(0, 0, 0, "floor"), terrain(1, 0, 0, "floor")];
        let mut a = room(0, 0, 0, "a");
        a.exits = BTreeMap::from([("east".to_owned(), "b".to_owned())]);
        // "b" is defined after "a" in the Vec — must still resolve.
        doc.rooms = vec![a, room(1, 0, 0, "b")];
        assert_eq!(doc.validate(&empty_catalog()), Ok(()));
    }

    #[test]
    fn refuses_unknown_entity_item_and_fixture_refs() {
        // Each kind isolated: the other two categories resolve.
        let mut bad_entity = valid_doc();
        let mut r = room(0, 0, 0, "start");
        r.entities = vec!["ghost".to_owned()];
        bad_entity.rooms = vec![r];
        assert_eq!(
            bad_entity.validate(&empty_catalog()),
            Err(MapValidationError::UnknownReference {
                room_id: "start".to_owned(),
                cell: Cell { x: 0, y: 0, z: 0 },
                kind: RefKind::Entity,
                id: "ghost".to_owned()
            })
        );

        let mut bad_item = valid_doc();
        let mut r = room(0, 0, 0, "start");
        r.items = vec!["phantom".to_owned()];
        bad_item.rooms = vec![r];
        assert_eq!(
            bad_item.validate(&empty_catalog()),
            Err(MapValidationError::UnknownReference {
                room_id: "start".to_owned(),
                cell: Cell { x: 0, y: 0, z: 0 },
                kind: RefKind::Item,
                id: "phantom".to_owned()
            })
        );

        let mut bad_fixture = valid_doc();
        let mut r = room(0, 0, 0, "start");
        r.fixtures = vec!["mirage".to_owned()];
        bad_fixture.rooms = vec![r];
        assert_eq!(
            bad_fixture.validate(&empty_catalog()),
            Err(MapValidationError::UnknownReference {
                room_id: "start".to_owned(),
                cell: Cell { x: 0, y: 0, z: 0 },
                kind: RefKind::Fixture,
                id: "mirage".to_owned()
            })
        );
    }

    #[test]
    fn refuses_when_no_spawn() {
        let mut doc = valid_doc();
        doc.spawn = None;
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::NoSpawnPoint)
        );
    }

    #[test]
    fn refuses_spawn_that_is_not_a_room() {
        let mut doc = valid_doc();
        doc.spawn = Some(Cell { x: 3, y: 3, z: 0 });
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::SpawnNotARoom {
                cell: Cell { x: 3, y: 3, z: 0 }
            })
        );
    }

    #[test]
    fn refuses_world_invalid_on_subregion_parent_mismatch() {
        // Passes the doc-level check (subregion + regions are declared) but the
        // engine rejects the parent-region mismatch → WorldInvalid net.
        let mut doc = valid_doc();
        doc.regions = BTreeMap::from([
            (
                "reg".to_owned(),
                RegionDefinition {
                    id: "reg".to_owned(),
                    name: "Reg".to_owned(),
                    description: String::new(),
                },
            ),
            (
                "other".to_owned(),
                RegionDefinition {
                    id: "other".to_owned(),
                    name: "Other".to_owned(),
                    description: String::new(),
                },
            ),
        ]);
        doc.subregions = BTreeMap::from([(
            "sub".to_owned(),
            SubregionDefinition {
                id: "sub".to_owned(),
                name: "Sub".to_owned(),
                region: "other".to_owned(), // parent is "other"...
                description: String::new(),
            },
        )]);
        let mut r = room(0, 0, 0, "start");
        r.region = "reg".to_owned(); // ...but the room is in "reg"
        r.subregion = Some("sub".to_owned());
        doc.rooms = vec![r];

        let err = doc.materialize(&empty_catalog()).unwrap_err();
        assert!(
            matches!(err, MapValidationError::WorldInvalid(_)),
            "expected WorldInvalid, got {err:?}"
        );
    }

    // ---- REQ-005: ordinary defaults vs special overrides ----

    #[test]
    fn ordinary_room_uses_defaults() {
        let world = valid_doc()
            .materialize(&empty_catalog())
            .expect("materializes");
        let room = world.rooms.get("start").expect("start room");
        assert_eq!(room.title, "Unnamed Room");
        assert_eq!(room.description, "An unremarkable area.");
        assert_eq!(room.glyph, '.');
    }

    #[test]
    fn special_room_uses_overrides() {
        let mut doc = valid_doc();
        let mut r = room(0, 0, 0, "start");
        r.title = Some("Throne Room".to_owned());
        r.description = Some("Gilded and vast.".to_owned());
        r.glyph = Some('T');
        doc.rooms = vec![r];
        let world = doc.materialize(&empty_catalog()).expect("materializes");
        let room = world.rooms.get("start").expect("start room");
        assert_eq!(room.title, "Throne Room");
        assert_eq!(room.description, "Gilded and vast.");
        assert_eq!(room.glyph, 'T');
    }

    // ---- REQ-006: serde round-trip ----

    #[test]
    fn map_document_round_trips_through_json() {
        let mut doc = valid_doc();
        doc.terrain.push(terrain(1, 0, 0, "wall"));
        let mut special = room(0, 0, 0, "start");
        special.title = Some("Hall".to_owned());
        special.exits = BTreeMap::from([("east".to_owned(), "start".to_owned())]);
        doc.rooms = vec![special];

        let json = serde_json::to_string(&doc).expect("serializes");
        let back: MapDocument = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(doc, back);
        assert!(!json.contains("pixel"));
        assert!(!json.contains("render"));
    }

    // ---- mutation pins: materialization field fidelity ----

    #[test]
    fn materialized_rooms_are_passable() {
        let world = valid_doc()
            .materialize(&empty_catalog())
            .expect("materializes");
        assert!(world.rooms.get("start").expect("start room").passable);
    }

    #[test]
    fn combat_enabled_propagates_both_values() {
        let mut doc = valid_doc();
        doc.terrain = vec![terrain(0, 0, 0, "floor"), terrain(1, 0, 0, "floor")];
        let mut fighty = room(0, 0, 0, "arena");
        fighty.combat_enabled = true;
        doc.rooms = vec![fighty, room(1, 0, 0, "calm")];
        doc.spawn = Some(Cell { x: 0, y: 0, z: 0 });
        let world = doc.materialize(&empty_catalog()).expect("materializes");
        assert!(world.rooms.get("arena").expect("arena").combat_enabled);
        assert!(!world.rooms.get("calm").expect("calm").combat_enabled);
    }

    #[test]
    fn start_room_id_is_the_spawn_room() {
        let mut doc = valid_doc();
        doc.terrain = vec![terrain(0, 0, 0, "floor"), terrain(1, 0, 0, "floor")];
        doc.rooms = vec![room(0, 0, 0, "alpha"), room(1, 0, 0, "omega")];
        doc.spawn = Some(Cell { x: 1, y: 0, z: 0 });
        let world = doc.materialize(&empty_catalog()).expect("materializes");
        assert_eq!(world.start_room_id, "omega");
    }

    #[test]
    fn validate_rejects_an_invalid_document() {
        // validate() (not materialize) on a bad doc must surface the error.
        let mut doc = valid_doc();
        doc.tile_size = 7;
        assert!(doc.validate(&empty_catalog()).is_err());
    }

    #[test]
    fn placed_refs_and_inventory_travel_into_the_world() {
        // A placed entity owns an item the room does not itself place; both the
        // placed item and the inventory item must reach world.items.
        let mut doc = valid_doc();
        let mut r = room(0, 0, 0, "start");
        r.entities = vec!["guard".to_owned()];
        r.items = vec!["coin".to_owned()];
        doc.rooms = vec![r];

        let mut guard = entity("guard");
        guard.inventory = vec!["sword".to_owned()];
        let catalog = ContentCatalog {
            entities: BTreeMap::from([("guard".to_owned(), guard)]),
            items: BTreeMap::from([
                ("coin".to_owned(), item("coin")),
                ("sword".to_owned(), item("sword")),
            ]),
            fixtures: BTreeSet::new(),
        };

        let world = doc.materialize(&catalog).expect("materializes");
        assert!(world.entities.contains_key("guard"));
        assert!(world.items.contains_key("coin"));
        assert!(
            world.items.contains_key("sword"),
            "an inventory item must travel with its entity"
        );
    }

    #[test]
    fn unreferenced_catalog_entries_are_excluded() {
        // Only placed content is materialized — the filter's false branch.
        let mut doc = valid_doc();
        let mut r = room(0, 0, 0, "start");
        r.entities = vec!["used".to_owned()];
        doc.rooms = vec![r];

        let catalog = ContentCatalog {
            entities: BTreeMap::from([
                ("used".to_owned(), entity("used")),
                ("unused".to_owned(), entity("unused")),
            ]),
            items: BTreeMap::from([("idle".to_owned(), item("idle"))]),
            fixtures: BTreeSet::new(),
        };

        let world = doc.materialize(&catalog).expect("materializes");
        assert!(world.entities.contains_key("used"));
        assert!(!world.entities.contains_key("unused"));
        assert!(!world.items.contains_key("idle"));
    }

    // ---- ticket #47: tilesets + tile layers ----

    fn tileset(id: &str) -> Tileset {
        Tileset {
            id: id.to_owned(),
            image: format!("{id}.png"),
            tile_size: 16,
            columns: 2,
            rows: 3,
            tiles: BTreeMap::new(),
        }
    }

    fn layer_cell(x: i32, y: i32, z: i32, tileset: &str, index: u32) -> LayerCell {
        LayerCell {
            x,
            y,
            z,
            tileset: tileset.to_owned(),
            index,
        }
    }

    fn layer(id: &str, cells: Vec<LayerCell>) -> Layer {
        Layer {
            id: id.to_owned(),
            name: id.to_owned(),
            kind: LayerKind::Tile,
            visible: true,
            cells,
        }
    }

    #[test]
    fn tileset_and_layer_validate() {
        // RT-1: a registered tileset (with sparse metadata) + an in-range cell.
        let mut doc = valid_doc();
        let mut ts = tileset("ts");
        ts.tiles.insert(
            0,
            TileMeta {
                name: Some("ice".to_owned()),
                passable: Some(true),
                tags: vec!["cold".to_owned()],
            },
        );
        doc.tilesets = vec![ts];
        doc.layers = vec![layer("ground", vec![layer_cell(0, 0, 0, "ts", 0)])];
        assert_eq!(doc.validate(&empty_catalog()), Ok(()));
    }

    #[test]
    fn layer_cell_unknown_tileset_is_refused() {
        // RT-2.
        let mut doc = valid_doc();
        doc.tilesets = vec![tileset("ts")];
        doc.layers = vec![layer("ground", vec![layer_cell(0, 0, 0, "missing", 0)])];
        assert_eq!(
            doc.validate(&empty_catalog()),
            Err(MapValidationError::UnknownTilesetRef {
                layer: "ground".to_owned(),
                cell: Cell { x: 0, y: 0, z: 0 },
                tileset: "missing".to_owned(),
            })
        );
    }

    #[test]
    fn layer_tile_index_boundary_is_exact() {
        // RT-3: capacity 6 (the helper tileset is 2x3 — product != sum, so this
        // also kills the `*` -> `+` capacity mutant; keep it non-square). index 5
        // validates; index 6 is refused — the exact boundary also kills `>=`->`>`.
        let mut ok = valid_doc();
        ok.tilesets = vec![tileset("ts")];
        ok.layers = vec![layer("g", vec![layer_cell(0, 0, 0, "ts", 5)])];
        assert_eq!(ok.validate(&empty_catalog()), Ok(()));

        let mut bad = valid_doc();
        bad.tilesets = vec![tileset("ts")];
        bad.layers = vec![layer("g", vec![layer_cell(0, 0, 0, "ts", 6)])];
        assert_eq!(
            bad.validate(&empty_catalog()),
            Err(MapValidationError::TileIndexOutOfRange {
                layer: "g".to_owned(),
                cell: Cell { x: 0, y: 0, z: 0 },
                index: 6,
            })
        );
    }

    #[test]
    fn duplicate_tileset_and_layer_ids_are_refused() {
        // RT-4: both copies have valid geometry so control reaches the dup check.
        let mut dt = valid_doc();
        dt.tilesets = vec![tileset("ts"), tileset("ts")];
        assert_eq!(
            dt.validate(&empty_catalog()),
            Err(MapValidationError::DuplicateTilesetId {
                id: "ts".to_owned()
            })
        );

        let mut dl = valid_doc();
        dl.tilesets = vec![tileset("ts")];
        dl.layers = vec![layer("L", vec![]), layer("L", vec![])];
        assert_eq!(
            dl.validate(&empty_catalog()),
            Err(MapValidationError::DuplicateLayerId { id: "L".to_owned() })
        );
    }

    #[test]
    fn layer_duplicate_cell_and_out_of_bounds_are_refused() {
        // RT-5: the dup-cell case paints the same IN-BOUNDS coord twice; the
        // out-of-bounds case reuses the shared CellOutOfBounds variant.
        let mut dup = valid_doc();
        dup.tilesets = vec![tileset("ts")];
        dup.layers = vec![layer(
            "g",
            vec![layer_cell(1, 1, 0, "ts", 0), layer_cell(1, 1, 0, "ts", 1)],
        )];
        assert_eq!(
            dup.validate(&empty_catalog()),
            Err(MapValidationError::DuplicateCell {
                cell: Cell { x: 1, y: 1, z: 0 }
            })
        );

        let mut oob = valid_doc();
        oob.tilesets = vec![tileset("ts")];
        oob.layers = vec![layer("g", vec![layer_cell(9, 9, 0, "ts", 0)])];
        assert_eq!(
            oob.validate(&empty_catalog()),
            Err(MapValidationError::CellOutOfBounds {
                cell: Cell { x: 9, y: 9, z: 0 }
            })
        );
    }

    #[test]
    fn unsupported_tileset_geometry_isolates_each_predicate() {
        // RT-6: each tileset is bad on exactly ONE predicate (valid on the other
        // two), so the `||` -> `&&` mutants die.
        let bad = [
            Tileset {
                id: "rows".to_owned(),
                image: "x.png".to_owned(),
                tile_size: 16,
                columns: 4,
                rows: 0,
                tiles: BTreeMap::new(),
            },
            Tileset {
                id: "cols".to_owned(),
                image: "x.png".to_owned(),
                tile_size: 16,
                columns: 0,
                rows: 4,
                tiles: BTreeMap::new(),
            },
            Tileset {
                id: "size".to_owned(),
                image: "x.png".to_owned(),
                tile_size: 7,
                columns: 4,
                rows: 4,
                tiles: BTreeMap::new(),
            },
        ];
        for ts in bad {
            let id = ts.id.clone();
            let mut doc = valid_doc();
            doc.tilesets = vec![ts];
            assert_eq!(
                doc.validate(&empty_catalog()),
                Err(MapValidationError::UnsupportedTilesetGeometry { id })
            );
        }
    }

    #[test]
    fn tileset_tile_meta_index_boundary_is_exact() {
        // RT-7: capacity 6 (2x3). metadata index 5 validates; index 6 is refused.
        let mut ok = valid_doc();
        let mut ts_ok = tileset("ts");
        ts_ok.tiles.insert(5, TileMeta::default());
        ok.tilesets = vec![ts_ok];
        assert_eq!(ok.validate(&empty_catalog()), Ok(()));

        let mut bad = valid_doc();
        let mut ts_bad = tileset("ts");
        ts_bad.tiles.insert(6, TileMeta::default());
        bad.tilesets = vec![ts_bad];
        assert_eq!(
            bad.validate(&empty_catalog()),
            Err(MapValidationError::TileMetaIndexOutOfRange {
                tileset: "ts".to_owned(),
                index: 6,
            })
        );
    }

    #[test]
    fn pre_slice_document_round_trips_without_tilesets_or_layers() {
        // RT-8: a document authored before this slice still loads + validates and
        // re-serializes WITHOUT the new keys (serde-additive backward-compat).
        let json = r#"{
            "id":"m","title":"M","tile_size":16,"width":4,"height":4,"floors":1,
            "terrain_palette":{"floor":{"tile":"f","passable":true}},
            "terrain":[{"x":0,"y":0,"z":0,"terrain":"floor"}],
            "regions":{"reg":{"id":"reg","name":"Reg"}},
            "rooms":[{"x":0,"y":0,"z":0,"id":"start","region":"reg"}],
            "spawn":{"x":0,"y":0,"z":0}
        }"#;
        let doc: MapDocument = serde_json::from_str(json).expect("pre-slice doc deserializes");
        assert!(doc.tilesets.is_empty());
        assert!(doc.layers.is_empty());
        assert_eq!(doc.validate(&empty_catalog()), Ok(()));
        let value = serde_json::to_value(&doc).expect("serializes");
        assert!(
            value.get("tilesets").is_none(),
            "tilesets omitted when empty"
        );
        assert!(value.get("layers").is_none(), "layers omitted when empty");
    }

    #[test]
    fn layers_do_not_affect_materialization() {
        // RT-9: layers are authoring-visual; the materialized world is identical.
        let base = valid_doc()
            .materialize(&empty_catalog())
            .expect("base materializes");
        let mut layered_doc = valid_doc();
        layered_doc.tilesets = vec![tileset("ts")];
        layered_doc.layers = vec![layer("g", vec![layer_cell(0, 0, 0, "ts", 0)])];
        let layered = layered_doc
            .materialize(&empty_catalog())
            .expect("layered materializes");
        // WorldDefinition is not PartialEq; its deterministic (BTreeMap) Debug
        // form is a faithful structural-equality proxy.
        assert_eq!(format!("{base:?}"), format!("{layered:?}"));
    }

    #[test]
    fn new_variant_error_messages_render() {
        // RT-10: exact Display strings for the 6 new variants (mutation pins).
        let cell = Cell { x: 1, y: 2, z: 3 };
        let cases = [
            (
                MapValidationError::DuplicateTilesetId {
                    id: "ts".to_owned(),
                },
                "tileset id 'ts' is reused",
            ),
            (
                MapValidationError::UnsupportedTilesetGeometry {
                    id: "ts".to_owned(),
                },
                "tileset 'ts' has unsupported geometry (tile size must be one of [8, 16, 32], columns and rows must be positive)",
            ),
            (
                MapValidationError::TileMetaIndexOutOfRange {
                    tileset: "ts".to_owned(),
                    index: 9,
                },
                "tileset 'ts' metadata names out-of-range tile index 9",
            ),
            (
                MapValidationError::DuplicateLayerId { id: "g".to_owned() },
                "layer id 'g' is reused",
            ),
            (
                MapValidationError::UnknownTilesetRef {
                    layer: "g".to_owned(),
                    cell,
                    tileset: "ts".to_owned(),
                },
                "layer 'g' cell (1, 2, 3) references unknown tileset 'ts'",
            ),
            (
                MapValidationError::TileIndexOutOfRange {
                    layer: "g".to_owned(),
                    cell,
                    index: 9,
                },
                "layer 'g' cell (1, 2, 3) references out-of-range tile index 9",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.to_string(), expected);
        }
    }

    #[test]
    fn layer_visible_and_kind_default_when_omitted() {
        // RT-11: a Layer JSON omitting `visible`/`kind` defaults to true/Tile
        // (kills the `default_true` -> false mutant).
        let parsed: Layer =
            serde_json::from_str(r#"{"id":"g","name":"Ground"}"#).expect("layer deserializes");
        assert!(parsed.visible, "visible defaults to true");
        assert_eq!(parsed.kind, LayerKind::Tile, "kind defaults to Tile");
        assert!(parsed.cells.is_empty());
    }

    // ---- mutation pins: Display / RefKind / source ----

    #[test]
    fn cell_display_is_a_coordinate_triple() {
        assert_eq!(Cell { x: 1, y: 2, z: 3 }.to_string(), "(1, 2, 3)");
    }

    #[test]
    fn ref_kind_labels() {
        assert_eq!(RefKind::Entity.to_string(), "entity");
        assert_eq!(RefKind::Item.to_string(), "item");
        assert_eq!(RefKind::Fixture.to_string(), "fixture");
    }

    #[test]
    fn error_messages_render_each_variant() {
        let cell = Cell { x: 1, y: 2, z: 3 };
        let cases = [
            (
                MapValidationError::UnsupportedTileSize { found: 7 },
                "tile size 7 is unsupported (expected one of [8, 16, 32])",
            ),
            (
                MapValidationError::CellOutOfBounds { cell },
                "cell (1, 2, 3) is out of bounds",
            ),
            (
                MapValidationError::UnknownTerrain {
                    cell,
                    name: "lava".to_owned(),
                },
                "cell (1, 2, 3) uses undeclared terrain 'lava'",
            ),
            (
                MapValidationError::DuplicateCell { cell },
                "cell (1, 2, 3) is occupied twice",
            ),
            (
                MapValidationError::MissingTerrain { cell },
                "room cell (1, 2, 3) has no terrain",
            ),
            (
                MapValidationError::RoomOnBlockedTerrain {
                    cell,
                    room_id: "r".to_owned(),
                    terrain: "wall".to_owned(),
                },
                "room 'r' at (1, 2, 3) sits on non-passable terrain 'wall'",
            ),
            (
                MapValidationError::DuplicateRoomId {
                    cell,
                    room_id: "r".to_owned(),
                },
                "room id 'r' is reused at cell (1, 2, 3)",
            ),
            (
                MapValidationError::RoomRegionMissing {
                    cell,
                    room_id: "r".to_owned(),
                    region: "reg".to_owned(),
                },
                "room 'r' at (1, 2, 3) references undeclared region 'reg'",
            ),
            (
                MapValidationError::RoomSubregionMissing {
                    cell,
                    room_id: "r".to_owned(),
                    subregion: "sub".to_owned(),
                },
                "room 'r' at (1, 2, 3) references undeclared subregion 'sub'",
            ),
            (
                MapValidationError::DanglingExit {
                    room_id: "r".to_owned(),
                    direction: "north".to_owned(),
                    target_room_id: "void".to_owned(),
                },
                "room 'r' exit 'north' targets missing room 'void'",
            ),
            (
                MapValidationError::UnknownReference {
                    room_id: "r".to_owned(),
                    cell,
                    kind: RefKind::Item,
                    id: "x".to_owned(),
                },
                "room 'r' at (1, 2, 3) references unknown item 'x'",
            ),
            (
                MapValidationError::NoSpawnPoint,
                "the map declares no spawn point",
            ),
            (
                MapValidationError::SpawnNotARoom { cell },
                "spawn cell (1, 2, 3) is not a room",
            ),
            (
                MapValidationError::WorldInvalid(WorldValidationError::StartRoomMissing {
                    start_room_id: "s".to_owned(),
                }),
                "materialized world is invalid: start room 's' does not exist",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.to_string(), expected);
        }
    }

    #[test]
    fn only_world_invalid_carries_a_source() {
        use std::error::Error;
        let wrapped = MapValidationError::WorldInvalid(WorldValidationError::StartRoomMissing {
            start_room_id: "s".to_owned(),
        });
        assert!(wrapped.source().is_some());
        assert!(MapValidationError::NoSpawnPoint.source().is_none());
    }

    #[test]
    fn world_invalid_serializes_as_the_inner_message() {
        // `serialize_world_error` emits the inner engine error's Display (the
        // wrapper's "materialized world is invalid:" prefix is not applied here).
        let error = MapValidationError::WorldInvalid(WorldValidationError::StartRoomMissing {
            start_room_id: "s".to_owned(),
        });
        assert_eq!(
            serde_json::to_value(&error).expect("serializes"),
            serde_json::json!({ "WorldInvalid": "start room 's' does not exist" })
        );
    }

    #[test]
    fn unknown_reference_serializes_with_named_fields() {
        // Pins the external tag, the field names, and RefKind's capitalized form.
        let error = MapValidationError::UnknownReference {
            room_id: "r".to_owned(),
            cell: Cell { x: 1, y: 2, z: 3 },
            kind: RefKind::Entity,
            id: "x".to_owned(),
        };
        assert_eq!(
            serde_json::to_value(&error).expect("serializes"),
            serde_json::json!({
                "UnknownReference": {
                    "room_id": "r",
                    "cell": { "x": 1, "y": 2, "z": 3 },
                    "kind": "Entity",
                    "id": "x"
                }
            })
        );
    }
}
