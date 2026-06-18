//! Region & sub-region editing for an authored [`MapDocument`] (ticket #51, slice 2).
//!
//! Six pure edit methods on [`MapDocument`] — create / rename / delete a region or
//! sub-region — that return a *new* edited document (the original is left untouched
//! on refusal) or a typed [`RegionEditError`]. Each method runs targeted referential
//! checks for a precise refusal, then re-validates the whole document through
//! [`MapDocument::validate`] (which materializes against the engine) so a mutation
//! that would break the world is rejected and never reaches storage. The studio
//! sidecar drives these from its Editor-gated forms.

use std::fmt;

use oathstar_core::{RegionDefinition, SubregionDefinition};
use serde::Serialize;

use crate::{ContentCatalog, MapDocument, MapValidationError};

/// Why a region / sub-region edit was refused.
///
/// Each variant names the offending id (and the blocker, for a still-referenced
/// delete) so the editor can point the author at it. Mirrors the hand-rolled style
/// of [`MapValidationError`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RegionEditError {
    /// A required field (`id`, `name`, or `region`) was blank.
    BlankField {
        /// The name of the blank field.
        field: String,
    },
    /// Creating a region whose id is already taken (a silent overwrite otherwise).
    DuplicateRegionId {
        /// The duplicate region id.
        id: String,
    },
    /// Renaming or deleting a region the document does not declare.
    UnknownRegion {
        /// The missing region id.
        id: String,
    },
    /// Deleting a region a room still places itself in.
    RegionReferencedByRoom {
        /// The region id.
        id: String,
        /// A room that still references it.
        room_id: String,
    },
    /// Deleting a region that still parents a sub-region.
    RegionReferencedBySubregion {
        /// The region id.
        id: String,
        /// A child sub-region.
        subregion_id: String,
    },
    /// Creating a sub-region whose id is already taken.
    DuplicateSubregionId {
        /// The duplicate sub-region id.
        id: String,
    },
    /// Creating a sub-region whose parent region is not declared.
    UnknownParentRegion {
        /// The sub-region being created.
        subregion_id: String,
        /// The undeclared parent region id.
        region: String,
    },
    /// Renaming or deleting a sub-region the document does not declare.
    UnknownSubregion {
        /// The missing sub-region id.
        id: String,
    },
    /// Deleting a sub-region a room still references.
    SubregionReferencedByRoom {
        /// The sub-region id.
        id: String,
        /// A room that still references it.
        room_id: String,
    },
    /// The edit was structurally fine but the resulting document would not
    /// materialize into a valid world — so it is not persisted.
    WouldBreakWorld(MapValidationError),
}

impl fmt::Display for RegionEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BlankField { field } => write!(f, "the {field} must not be blank"),
            Self::DuplicateRegionId { id } => write!(f, "region id '{id}' already exists"),
            Self::UnknownRegion { id } => write!(f, "no region '{id}' exists"),
            Self::RegionReferencedByRoom { id, room_id } => {
                write!(f, "region '{id}' is still used by room '{room_id}'")
            }
            Self::RegionReferencedBySubregion { id, subregion_id } => {
                write!(f, "region '{id}' still has sub-region '{subregion_id}'")
            }
            Self::DuplicateSubregionId { id } => {
                write!(f, "sub-region id '{id}' already exists")
            }
            Self::UnknownParentRegion {
                subregion_id,
                region,
            } => write!(
                f,
                "sub-region '{subregion_id}' references unknown parent region '{region}'"
            ),
            Self::UnknownSubregion { id } => write!(f, "no sub-region '{id}' exists"),
            Self::SubregionReferencedByRoom { id, room_id } => {
                write!(f, "sub-region '{id}' is still used by room '{room_id}'")
            }
            Self::WouldBreakWorld(err) => write!(f, "the edit would break the map: {err}"),
        }
    }
}

impl std::error::Error for RegionEditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::WouldBreakWorld(err) => Some(err),
            _ => None,
        }
    }
}

/// Trim `value`; refuse a blank field naming `field`.
fn non_blank<'a>(value: &'a str, field: &str) -> Result<&'a str, RegionEditError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(RegionEditError::BlankField {
            field: field.to_owned(),
        });
    }
    Ok(trimmed)
}

impl MapDocument {
    /// Re-validate an edited document against `catalog` (the materialize boundary),
    /// returning it on success or [`RegionEditError::WouldBreakWorld`] otherwise.
    ///
    /// Consumes the edited clone so a refused edit cannot be persisted by mistake.
    fn finish(self, catalog: &ContentCatalog) -> Result<Self, RegionEditError> {
        self.validate(catalog)
            .map_err(RegionEditError::WouldBreakWorld)?;
        Ok(self)
    }

    /// Add a region with `id` and `name`.
    ///
    /// # Errors
    /// [`RegionEditError::BlankField`] for a blank `id`/`name`,
    /// [`RegionEditError::DuplicateRegionId`] when the id is taken, or
    /// [`RegionEditError::WouldBreakWorld`] if the result would not materialize.
    pub fn create_region(
        &self,
        id: &str,
        name: &str,
        catalog: &ContentCatalog,
    ) -> Result<Self, RegionEditError> {
        let id = non_blank(id, "id")?;
        let name = non_blank(name, "name")?;
        if self.regions.contains_key(id) {
            return Err(RegionEditError::DuplicateRegionId { id: id.to_owned() });
        }
        let mut edited = self.clone();
        edited.regions.insert(
            id.to_owned(),
            RegionDefinition {
                id: id.to_owned(),
                name: name.to_owned(),
            },
        );
        edited.finish(catalog)
    }

    /// Rename region `id` to `name`.
    ///
    /// # Errors
    /// [`RegionEditError::BlankField`], [`RegionEditError::UnknownRegion`] when no
    /// such region exists, or [`RegionEditError::WouldBreakWorld`].
    pub fn rename_region(
        &self,
        id: &str,
        name: &str,
        catalog: &ContentCatalog,
    ) -> Result<Self, RegionEditError> {
        let id = non_blank(id, "id")?;
        let name = non_blank(name, "name")?;
        let mut edited = self.clone();
        let Some(region) = edited.regions.get_mut(id) else {
            return Err(RegionEditError::UnknownRegion { id: id.to_owned() });
        };
        region.name = name.to_owned();
        edited.finish(catalog)
    }

    /// Delete region `id`.
    ///
    /// # Errors
    /// [`RegionEditError::UnknownRegion`]; [`RegionEditError::RegionReferencedByRoom`]
    /// or [`RegionEditError::RegionReferencedBySubregion`] when it is still in use; or
    /// [`RegionEditError::WouldBreakWorld`].
    pub fn delete_region(
        &self,
        id: &str,
        catalog: &ContentCatalog,
    ) -> Result<Self, RegionEditError> {
        let id = non_blank(id, "id")?;
        if !self.regions.contains_key(id) {
            return Err(RegionEditError::UnknownRegion { id: id.to_owned() });
        }
        if let Some(room) = self.rooms.iter().find(|room| room.region == id) {
            return Err(RegionEditError::RegionReferencedByRoom {
                id: id.to_owned(),
                room_id: room.id.clone(),
            });
        }
        if let Some(sub) = self.subregions.values().find(|sub| sub.region == id) {
            return Err(RegionEditError::RegionReferencedBySubregion {
                id: id.to_owned(),
                subregion_id: sub.id.clone(),
            });
        }
        let mut edited = self.clone();
        edited.regions.remove(id);
        edited.finish(catalog)
    }

    /// Add a sub-region with `id` and `name` under parent region `region`.
    ///
    /// # Errors
    /// [`RegionEditError::BlankField`], [`RegionEditError::DuplicateSubregionId`],
    /// [`RegionEditError::UnknownParentRegion`] when the parent is not declared, or
    /// [`RegionEditError::WouldBreakWorld`].
    pub fn create_subregion(
        &self,
        id: &str,
        name: &str,
        region: &str,
        catalog: &ContentCatalog,
    ) -> Result<Self, RegionEditError> {
        let id = non_blank(id, "id")?;
        let name = non_blank(name, "name")?;
        let region = non_blank(region, "region")?;
        if self.subregions.contains_key(id) {
            return Err(RegionEditError::DuplicateSubregionId { id: id.to_owned() });
        }
        if !self.regions.contains_key(region) {
            return Err(RegionEditError::UnknownParentRegion {
                subregion_id: id.to_owned(),
                region: region.to_owned(),
            });
        }
        let mut edited = self.clone();
        edited.subregions.insert(
            id.to_owned(),
            SubregionDefinition {
                id: id.to_owned(),
                name: name.to_owned(),
                region: region.to_owned(),
            },
        );
        edited.finish(catalog)
    }

    /// Rename sub-region `id` to `name`.
    ///
    /// # Errors
    /// [`RegionEditError::BlankField`], [`RegionEditError::UnknownSubregion`], or
    /// [`RegionEditError::WouldBreakWorld`].
    pub fn rename_subregion(
        &self,
        id: &str,
        name: &str,
        catalog: &ContentCatalog,
    ) -> Result<Self, RegionEditError> {
        let id = non_blank(id, "id")?;
        let name = non_blank(name, "name")?;
        let mut edited = self.clone();
        let Some(sub) = edited.subregions.get_mut(id) else {
            return Err(RegionEditError::UnknownSubregion { id: id.to_owned() });
        };
        sub.name = name.to_owned();
        edited.finish(catalog)
    }

    /// Delete sub-region `id`.
    ///
    /// # Errors
    /// [`RegionEditError::UnknownSubregion`];
    /// [`RegionEditError::SubregionReferencedByRoom`] when a room still references it;
    /// or [`RegionEditError::WouldBreakWorld`].
    pub fn delete_subregion(
        &self,
        id: &str,
        catalog: &ContentCatalog,
    ) -> Result<Self, RegionEditError> {
        let id = non_blank(id, "id")?;
        if !self.subregions.contains_key(id) {
            return Err(RegionEditError::UnknownSubregion { id: id.to_owned() });
        }
        if let Some(room) = self
            .rooms
            .iter()
            .find(|room| room.subregion.as_deref() == Some(id))
        {
            return Err(RegionEditError::SubregionReferencedByRoom {
                id: id.to_owned(),
                room_id: room.id.clone(),
            });
        }
        let mut edited = self.clone();
        edited.subregions.remove(id);
        edited.finish(catalog)
    }
}

#[cfg(test)]
mod tests {
    use super::{non_blank, RegionEditError};
    use crate::{
        Cell, ContentCatalog, MapDocument, MapValidationError, RoomCell, TerrainCell, TerrainDef,
    };
    use oathstar_core::{RegionDefinition, SubregionDefinition};
    use std::collections::BTreeMap;

    fn cat() -> ContentCatalog {
        ContentCatalog::default()
    }

    fn region(id: &str, name: &str) -> (String, RegionDefinition) {
        (
            id.to_owned(),
            RegionDefinition {
                id: id.to_owned(),
                name: name.to_owned(),
            },
        )
    }

    fn subregion(id: &str, name: &str, parent: &str) -> (String, SubregionDefinition) {
        (
            id.to_owned(),
            SubregionDefinition {
                id: id.to_owned(),
                name: name.to_owned(),
                region: parent.to_owned(),
            },
        )
    }

    fn room(x: i32, y: i32, z: i32, id: &str, region: &str, subregion: Option<&str>) -> RoomCell {
        RoomCell {
            x,
            y,
            z,
            id: id.to_owned(),
            title: None,
            description: None,
            region: region.to_owned(),
            subregion: subregion.map(str::to_owned),
            glyph: None,
            combat_enabled: false,
            exits: BTreeMap::new(),
            entities: Vec::new(),
            items: Vec::new(),
            fixtures: Vec::new(),
        }
    }

    /// A minimal materializable document: one passable `floor` room `start` in
    /// region `reg`, spawned on it.
    fn base_doc() -> MapDocument {
        MapDocument {
            id: "m".to_owned(),
            title: "M".to_owned(),
            tile_size: 16,
            width: 4,
            height: 4,
            floors: 1,
            terrain_palette: BTreeMap::from([(
                "floor".to_owned(),
                TerrainDef {
                    tile: "f".to_owned(),
                    passable: true,
                },
            )]),
            terrain: vec![TerrainCell {
                x: 0,
                y: 0,
                z: 0,
                terrain: "floor".to_owned(),
            }],
            regions: BTreeMap::from([region("reg", "Region")]),
            subregions: BTreeMap::new(),
            rooms: vec![room(0, 0, 0, "start", "reg", None)],
            spawn: Some(Cell { x: 0, y: 0, z: 0 }),
            tilesets: Vec::new(),
            layers: Vec::new(),
        }
    }

    #[test]
    fn base_doc_is_materializable() {
        assert_eq!(base_doc().validate(&cat()), Ok(()));
    }

    // ---- C1: region create + duplicate (REQ-001) ----

    #[test]
    fn create_region_adds_with_id_and_name() {
        let base = base_doc();
        let edited = base
            .create_region("forest", "Forest", &cat())
            .expect("creates");
        let added = edited.regions.get("forest").expect("region added");
        assert_eq!(added.id, "forest");
        assert_eq!(added.name, "Forest");
        assert!(edited.regions.contains_key("reg"), "keeps existing region");
        assert!(
            !base.regions.contains_key("forest"),
            "the original document is left unchanged"
        );
    }

    #[test]
    fn create_region_refuses_a_duplicate_id() {
        let err = base_doc().create_region("reg", "Dup", &cat()).unwrap_err();
        assert_eq!(
            err,
            RegionEditError::DuplicateRegionId {
                id: "reg".to_owned()
            }
        );
    }

    // ---- C2: subregion create (REQ-002) ----

    #[test]
    fn create_subregion_adds_under_a_valid_parent() {
        let edited = base_doc()
            .create_subregion("vale", "Vale", "reg", &cat())
            .expect("creates");
        let sub = edited.subregions.get("vale").expect("subregion added");
        assert_eq!(sub.id, "vale");
        assert_eq!(sub.name, "Vale");
        assert_eq!(sub.region, "reg");
    }

    #[test]
    fn create_subregion_refuses_an_unknown_parent() {
        let err = base_doc()
            .create_subregion("vale", "Vale", "ghost", &cat())
            .unwrap_err();
        assert_eq!(
            err,
            RegionEditError::UnknownParentRegion {
                subregion_id: "vale".to_owned(),
                region: "ghost".to_owned()
            }
        );
    }

    #[test]
    fn create_subregion_refuses_a_duplicate_id() {
        let mut base = base_doc();
        base.subregions.extend([subregion("vale", "Vale", "reg")]);
        let err = base
            .create_subregion("vale", "Other", "reg", &cat())
            .unwrap_err();
        assert_eq!(
            err,
            RegionEditError::DuplicateSubregionId {
                id: "vale".to_owned()
            }
        );
    }

    // ---- C3: rename (REQ-003) ----

    #[test]
    fn rename_region_updates_the_name_and_keeps_the_id() {
        let edited = base_doc()
            .rename_region("reg", "Renamed", &cat())
            .expect("renames");
        let region = edited.regions.get("reg").expect("region");
        assert_eq!(region.name, "Renamed");
        assert_eq!(region.id, "reg");
    }

    #[test]
    fn rename_region_refuses_unknown() {
        let err = base_doc().rename_region("ghost", "X", &cat()).unwrap_err();
        assert_eq!(
            err,
            RegionEditError::UnknownRegion {
                id: "ghost".to_owned()
            }
        );
    }

    #[test]
    fn rename_subregion_updates_the_name() {
        let mut base = base_doc();
        base.subregions.extend([subregion("vale", "Vale", "reg")]);
        let edited = base
            .rename_subregion("vale", "Glen", &cat())
            .expect("renames");
        assert_eq!(edited.subregions.get("vale").expect("sub").name, "Glen");
    }

    #[test]
    fn rename_subregion_refuses_unknown() {
        let err = base_doc()
            .rename_subregion("ghost", "X", &cat())
            .unwrap_err();
        assert_eq!(
            err,
            RegionEditError::UnknownSubregion {
                id: "ghost".to_owned()
            }
        );
    }

    // ---- C4: region delete referential integrity (REQ-004) ----

    #[test]
    fn delete_region_removes_an_unreferenced_region() {
        let mut base = base_doc();
        base.regions.extend([region("spare", "Spare")]);
        let edited = base.delete_region("spare", &cat()).expect("deletes");
        assert!(!edited.regions.contains_key("spare"));
        assert!(edited.regions.contains_key("reg"), "keeps the used region");
    }

    #[test]
    fn delete_region_refuses_when_a_room_uses_it() {
        let err = base_doc().delete_region("reg", &cat()).unwrap_err();
        assert_eq!(
            err,
            RegionEditError::RegionReferencedByRoom {
                id: "reg".to_owned(),
                room_id: "start".to_owned()
            }
        );
    }

    #[test]
    fn delete_region_refuses_when_a_subregion_parents_it() {
        // `zone` has no room but parents `sub`; deleting it is refused before any
        // mutation (the room check passes, the subregion check fires).
        let mut base = base_doc();
        base.regions.extend([region("zone", "Zone")]);
        base.subregions.extend([subregion("sub", "Sub", "zone")]);
        let err = base.delete_region("zone", &cat()).unwrap_err();
        assert_eq!(
            err,
            RegionEditError::RegionReferencedBySubregion {
                id: "zone".to_owned(),
                subregion_id: "sub".to_owned()
            }
        );
    }

    #[test]
    fn delete_region_refuses_unknown() {
        let err = base_doc().delete_region("ghost", &cat()).unwrap_err();
        assert_eq!(
            err,
            RegionEditError::UnknownRegion {
                id: "ghost".to_owned()
            }
        );
    }

    // ---- C5: subregion delete referential integrity (REQ-004) ----

    #[test]
    fn delete_subregion_removes_an_unreferenced_subregion() {
        let mut base = base_doc();
        base.subregions.extend([subregion("vale", "Vale", "reg")]);
        let edited = base.delete_subregion("vale", &cat()).expect("deletes");
        assert!(!edited.subregions.contains_key("vale"));
    }

    #[test]
    fn delete_subregion_refuses_when_a_room_uses_it() {
        let mut base = base_doc();
        base.subregions.extend([subregion("vale", "Vale", "reg")]);
        base.rooms = vec![room(0, 0, 0, "start", "reg", Some("vale"))];
        let err = base.delete_subregion("vale", &cat()).unwrap_err();
        assert_eq!(
            err,
            RegionEditError::SubregionReferencedByRoom {
                id: "vale".to_owned(),
                room_id: "start".to_owned()
            }
        );
    }

    #[test]
    fn delete_subregion_refuses_unknown() {
        let err = base_doc().delete_subregion("ghost", &cat()).unwrap_err();
        assert_eq!(
            err,
            RegionEditError::UnknownSubregion {
                id: "ghost".to_owned()
            }
        );
    }

    // ---- C6: materialize net, blank fields, trimming, error surface (REQ-006) ----

    #[test]
    fn edit_is_refused_when_the_document_would_not_materialize() {
        // A spawn-less document never materializes; a structurally-fine edit is still
        // refused by the materialize net rather than persisted.
        let mut base = base_doc();
        base.spawn = None;
        let err = base.create_region("forest", "Forest", &cat()).unwrap_err();
        assert!(
            matches!(err, RegionEditError::WouldBreakWorld(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn blank_fields_are_refused_by_name() {
        let c = cat();
        assert_eq!(
            base_doc().create_region("", "X", &c).unwrap_err(),
            RegionEditError::BlankField {
                field: "id".to_owned()
            }
        );
        assert_eq!(
            base_doc().create_region("   ", "X", &c).unwrap_err(),
            RegionEditError::BlankField {
                field: "id".to_owned()
            },
            "whitespace-only is blank after trimming"
        );
        assert_eq!(
            base_doc().create_region("ok", "", &c).unwrap_err(),
            RegionEditError::BlankField {
                field: "name".to_owned()
            }
        );
        assert_eq!(
            base_doc()
                .create_subregion("s", "S", "   ", &c)
                .unwrap_err(),
            RegionEditError::BlankField {
                field: "region".to_owned()
            }
        );
    }

    #[test]
    fn non_blank_trims_and_keeps_the_trimmed_value() {
        assert_eq!(non_blank("  forest  ", "id"), Ok("forest"));
        let edited = base_doc()
            .create_region("  forest  ", "  Forest  ", &cat())
            .expect("creates");
        let added = edited.regions.get("forest").expect("trimmed id is the key");
        assert_eq!(added.id, "forest");
        assert_eq!(added.name, "Forest");
    }

    #[test]
    fn error_messages_name_the_offender() {
        use RegionEditError as E;
        assert!(E::BlankField {
            field: "qqq".to_owned()
        }
        .to_string()
        .contains("qqq"));
        assert!(E::DuplicateRegionId {
            id: "zzz".to_owned()
        }
        .to_string()
        .contains("zzz"));
        assert!(E::UnknownRegion {
            id: "zzz".to_owned()
        }
        .to_string()
        .contains("zzz"));
        assert!(E::RegionReferencedByRoom {
            id: "zzz".to_owned(),
            room_id: "rrr".to_owned()
        }
        .to_string()
        .contains("rrr"));
        assert!(E::RegionReferencedBySubregion {
            id: "zzz".to_owned(),
            subregion_id: "sss".to_owned()
        }
        .to_string()
        .contains("sss"));
        assert!(E::DuplicateSubregionId {
            id: "zzz".to_owned()
        }
        .to_string()
        .contains("zzz"));
        assert!(E::UnknownParentRegion {
            subregion_id: "zzz".to_owned(),
            region: "ppp".to_owned()
        }
        .to_string()
        .contains("ppp"));
        assert!(E::UnknownSubregion {
            id: "zzz".to_owned()
        }
        .to_string()
        .contains("zzz"));
        assert!(E::SubregionReferencedByRoom {
            id: "zzz".to_owned(),
            room_id: "rrr".to_owned()
        }
        .to_string()
        .contains("rrr"));
        let wrapped = E::WouldBreakWorld(MapValidationError::NoSpawnPoint);
        assert!(wrapped.to_string().contains("break"));
        assert!(
            wrapped.to_string().contains("spawn"),
            "wraps the inner error's message"
        );
    }

    #[test]
    fn source_is_the_wrapped_world_error_only() {
        use std::error::Error;
        assert!(
            RegionEditError::WouldBreakWorld(MapValidationError::NoSpawnPoint)
                .source()
                .is_some()
        );
        assert!(RegionEditError::UnknownRegion { id: "x".to_owned() }
            .source()
            .is_none());
    }
}
