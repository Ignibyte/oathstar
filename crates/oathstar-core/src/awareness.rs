//! Spatial awareness / proximity foundation (ticket #17).
//!
//! Oathstar's "blast radius" model: things are discoverable by *distance* within
//! the current subregion and z-plane, instead of requiring exact room
//! co-location. In v1 the grid cell IS the room (Decision 025) — entities and
//! items have no coordinates of their own, so they inherit the position of the
//! room that places them. This module owns the geometry ([`Position`]), the
//! action-specific radiuses ([`RadiusConfig`]), the distance classification
//! ([`Proximity`]), and the read-only queries ([`perceive`], [`resolve_target`])
//! over a [`WorldDefinition`].
//!
//! It is server-authoritative and renderer-agnostic: it returns structured data
//! ([`Awareness`]), never drawing instructions. Future systems (sight, hearing,
//! detection, combat aggro, stealth, map overlays) extend this foundation by
//! adding radius kinds and reveal rules — not by re-deriving geometry.

use crate::{EntityKind, RoomDefinition, WorldDefinition};

/// A discrete grid-cell position in world space.
///
/// Scoped by `region` + `subregion` so two cells only count as "near" each other
/// when they share the same plane (see [`Position::same_plane`]). Derived from a
/// room via [`Position::from_room`] — rooms are the grid cells in v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    /// The region id this cell belongs to.
    pub region: String,
    /// The subregion id, or `None` for a region-level cell.
    pub subregion: Option<String>,
    /// Horizontal grid coordinate, matching `RoomDefinition::x`.
    pub x: i32,
    /// Depth grid coordinate, matching `RoomDefinition::y`.
    pub y: i32,
    /// Floor / z-plane, matching `RoomDefinition::z`.
    pub z: i32,
}

impl Position {
    /// The cell a room occupies.
    #[must_use]
    pub fn from_room(room: &RoomDefinition) -> Self {
        Self {
            region: room.region.clone(),
            subregion: room.subregion.clone(),
            x: room.x,
            y: room.y,
            z: room.z,
        }
    }

    /// Whether two cells lie on the same awareness plane — the same region, the
    /// same subregion, and the same z-level.
    ///
    /// Proximity is only ever computed within a plane; a different floor or
    /// subregion is "not near", never merely "far" (ticket #17, REQ-001/002).
    #[must_use]
    pub fn same_plane(&self, other: &Self) -> bool {
        self.region == other.region && self.subregion == other.subregion && self.z == other.z
    }

    /// The Chebyshev (king-move) distance in cells to `other`, or `None` when the
    /// two cells are not on the same plane.
    ///
    /// Chebyshev — `max(|dx|, |dy|)` — makes a radius a *square* ring, matching
    /// the square-grid map (Decision 025) and an 8-neighbour tactical feel.
    /// Integer-only via `i32::abs_diff`, so it is deterministic and cannot
    /// overflow (no floating point, no `as` cast).
    #[must_use]
    pub fn cell_distance(&self, other: &Self) -> Option<u32> {
        if !self.same_plane(other) {
            return None;
        }
        Some(self.x.abs_diff(other.x).max(self.y.abs_diff(other.y)))
    }
}

/// Default cells the player can *see* (the perception / sight radius).
pub const DEFAULT_SIGHT_RADIUS: u32 = 3;
/// Default cells the player can directly *reach* (talk / take / interact).
pub const DEFAULT_INTERACTION_RADIUS: u32 = 1;

/// Action-specific awareness radiuses, in grid cells.
///
/// `interaction <= sight` is the intended relationship (you reach no farther than
/// you see); the classifier tolerates any values. Hearing or detection radiuses
/// can be added here later without changing call sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusConfig {
    /// How far the observer can perceive things at all.
    pub sight: u32,
    /// How far the observer can directly interact (`<= sight`).
    pub interaction: u32,
}

impl Default for RadiusConfig {
    fn default() -> Self {
        Self {
            sight: DEFAULT_SIGHT_RADIUS,
            interaction: DEFAULT_INTERACTION_RADIUS,
        }
    }
}

/// How a perceived thing relates to the observer, by distance band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proximity {
    /// The same cell as the observer (distance 0). Always interactable.
    Exact,
    /// Within the interaction radius (reachable), but not the same cell.
    Interactable,
    /// Within sight but beyond reach — seen, not yet interactable.
    Visible,
}

impl Proximity {
    /// Classify a same-plane cell `distance` against `radii`, or `None` when the
    /// distance is beyond the sight radius (not perceivable).
    ///
    /// Bands: `0` → [`Exact`](Self::Exact); `<= interaction` →
    /// [`Interactable`](Self::Interactable); `<= sight` →
    /// [`Visible`](Self::Visible); otherwise `None`.
    #[must_use]
    pub const fn classify(distance: u32, radii: &RadiusConfig) -> Option<Self> {
        if distance == 0 {
            Some(Self::Exact)
        } else if distance <= radii.interaction {
            Some(Self::Interactable)
        } else if distance <= radii.sight {
            Some(Self::Visible)
        } else {
            None
        }
    }

    /// Whether the observer can directly interact (same cell or within reach).
    #[must_use]
    pub const fn is_interactable(self) -> bool {
        matches!(self, Self::Exact | Self::Interactable)
    }

    /// The stable lowercase wire token (`"exact" | "interactable" | "visible"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Interactable => "interactable",
            Self::Visible => "visible",
        }
    }
}

/// What kind of thing was perceived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwarenessKind {
    /// A person/creature-like entity (NPC or enemy).
    Actor,
    /// An interactable fixture entity.
    Fixture,
    /// A world item.
    Item,
}

impl AwarenessKind {
    /// The stable lowercase wire token (`"actor" | "fixture" | "item"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Actor => "actor",
            Self::Fixture => "fixture",
            Self::Item => "item",
        }
    }

    /// The awareness kind for an entity, from its [`EntityKind`].
    const fn from_entity_kind(kind: EntityKind) -> Self {
        match kind {
            EntityKind::Actor => Self::Actor,
            EntityKind::Fixture => Self::Fixture,
        }
    }
}

/// One perceived thing and how it relates to the observer — the structured
/// awareness result (ticket #17).
///
/// Owns its strings so it can outlive the borrow of the world it was derived
/// from. `description` lets a command (e.g. `look`) report the thing without a
/// second lookup; the snapshot view need not carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Awareness {
    /// The thing's world id (entity id or item id).
    pub id: String,
    /// The id of the room that places this thing — the grid cell it occupies.
    /// Lets a command (e.g. `take`, ticket #18) mutate the exact placing room
    /// without re-deriving geometry, even when the thing is in an adjacent
    /// interactable cell rather than the observer's own.
    pub room_id: String,
    /// The thing's display name.
    pub name: String,
    /// The thing's description text.
    pub description: String,
    /// Whether it is an actor, fixture, or item.
    pub kind: AwarenessKind,
    /// Chebyshev cell distance from the observer (0 = same cell).
    pub distance: u32,
    /// The distance band (exact / interactable / visible).
    pub proximity: Proximity,
}

/// A perceived candidate that still borrows the source world data, so a query can
/// match on names/aliases before allocating an [`Awareness`].
struct Candidate<'w> {
    id: &'w str,
    room_id: &'w str,
    name: &'w str,
    description: &'w str,
    aliases: &'w [String],
    kind: AwarenessKind,
    distance: u32,
    proximity: Proximity,
}

impl Candidate<'_> {
    fn into_awareness(self) -> Awareness {
        Awareness {
            id: self.id.to_string(),
            room_id: self.room_id.to_string(),
            name: self.name.to_string(),
            description: self.description.to_string(),
            kind: self.kind,
            distance: self.distance,
            proximity: self.proximity,
        }
    }

    /// Case-insensitive match of `query` against the name or any alias.
    fn matches(&self, query: &str) -> bool {
        name_or_alias_matches(self.name, self.aliases, query)
    }
}

/// Case-insensitive match of `query` against a display `name` or any of its
/// `aliases` (ticket #20). Shared by the proximity resolver ([`Candidate::matches`])
/// and the engine's carried-pack resolver, so name/alias matching has one home.
pub(crate) fn name_or_alias_matches(name: &str, aliases: &[String], query: &str) -> bool {
    name.eq_ignore_ascii_case(query)
        || aliases
            .iter()
            .any(|alias| alias.eq_ignore_ascii_case(query))
}

/// Gather every perceivable thing within `origin`'s sight, on the same plane,
/// nearest first. Reveal-blocked (`hidden`) things are excluded (REQ-002).
///
/// Stable-sorted by distance only; equal-distance things keep world order
/// (rooms by id, entities before items, placement order) for determinism.
fn perceived_candidates<'w>(
    world: &'w WorldDefinition,
    origin: &RoomDefinition,
    radii: &RadiusConfig,
) -> Vec<Candidate<'w>> {
    let origin_pos = Position::from_room(origin);
    let mut found = Vec::new();

    for room in world.rooms.values() {
        let Some(distance) = origin_pos.cell_distance(&Position::from_room(room)) else {
            continue; // different region / subregion / z-plane
        };
        let Some(proximity) = Proximity::classify(distance, radii) else {
            continue; // beyond sight
        };

        for entity_id in &room.entities {
            if let Some(entity) = world.entities.get(entity_id) {
                if entity.hidden {
                    continue; // reveal-rule placeholder (REQ-002)
                }
                found.push(Candidate {
                    id: entity.id.as_str(),
                    room_id: room.id.as_str(),
                    name: entity.name.as_str(),
                    description: entity.description.as_str(),
                    aliases: entity.aliases.as_slice(),
                    kind: AwarenessKind::from_entity_kind(entity.kind),
                    distance,
                    proximity,
                });
            }
        }

        for item_id in &room.items {
            if let Some(item) = world.items.get(item_id) {
                if item.hidden {
                    continue; // reveal-rule placeholder (REQ-002)
                }
                found.push(Candidate {
                    id: item.id.as_str(),
                    room_id: room.id.as_str(),
                    name: item.name.as_str(),
                    description: item.description.as_str(),
                    aliases: item.aliases.as_slice(),
                    kind: AwarenessKind::Item,
                    distance,
                    proximity,
                });
            }
        }
    }

    found.sort_by_key(|candidate| candidate.distance);
    found
}

/// Everything the observer at `origin` perceives within `radii.sight`, on the
/// same subregion and z-plane, nearest first (ticket #17, REQ-001).
///
/// Read-only and deterministic. Things beyond sight, on another plane, or hidden
/// by the reveal placeholder are absent (REQ-002).
#[must_use]
pub fn perceive(
    world: &WorldDefinition,
    origin: &RoomDefinition,
    radii: &RadiusConfig,
) -> Vec<Awareness> {
    perceived_candidates(world, origin, radii)
        .into_iter()
        .map(Candidate::into_awareness)
        .collect()
}

/// Resolve a free-text `query` to the nearest perceivable thing whose name or an
/// alias matches case-insensitively, exact cell first (ticket #17, REQ-004).
///
/// Returns `None` when nothing in sight matches (including an empty query, which
/// matches no name). The returned [`Awareness`] carries the [`Proximity`] so the
/// caller can decide whether the match is close enough to interact with (REQ-003).
#[must_use]
pub fn resolve_target(
    world: &WorldDefinition,
    origin: &RoomDefinition,
    radii: &RadiusConfig,
    query: &str,
) -> Option<Awareness> {
    perceived_candidates(world, origin, radii)
        .into_iter()
        .find(|candidate| candidate.matches(query))
        .map(Candidate::into_awareness)
}

#[cfg(test)]
mod tests {
    use super::{
        perceive, resolve_target, AwarenessKind, Position, Proximity, RadiusConfig,
        DEFAULT_INTERACTION_RADIUS, DEFAULT_SIGHT_RADIUS,
    };
    use crate::{Entity, EntityKind, Item, RoomDefinition, WorldDefinition};
    use std::collections::BTreeMap;

    // ---- pure geometry ----

    fn pos(subregion: Option<&str>, x: i32, y: i32, z: i32) -> Position {
        Position {
            region: "r".to_string(),
            subregion: subregion.map(String::from),
            x,
            y,
            z,
        }
    }

    // Chebyshev = max(|dx|,|dy|): diagonal is the max not the sum, each axis can
    // dominate, and negatives/extremes are overflow-safe (abs_diff → u32).
    #[test]
    fn cell_distance_is_chebyshev_king_move() {
        let origin = pos(Some("s"), 0, 0, 0);
        assert_eq!(origin.cell_distance(&pos(Some("s"), 0, 0, 0)), Some(0));
        assert_eq!(origin.cell_distance(&pos(Some("s"), 3, 1, 0)), Some(3)); // x dominates
        assert_eq!(origin.cell_distance(&pos(Some("s"), 1, 3, 0)), Some(3)); // y dominates
        assert_eq!(origin.cell_distance(&pos(Some("s"), 2, 2, 0)), Some(2)); // max, not sum (4)
        assert_eq!(origin.cell_distance(&pos(Some("s"), 3, 0, 0)), Some(3));
        assert_eq!(origin.cell_distance(&pos(Some("s"), -3, -2, 0)), Some(3));
        assert_eq!(
            pos(Some("s"), i32::MIN, 0, 0).cell_distance(&pos(Some("s"), i32::MAX, 0, 0)),
            Some(u32::MAX)
        );
    }

    // Each plane field gates independently: a difference in subregion (Some vs Some
    // AND Some vs None), z, or region excludes entirely (None, never Some(0)).
    #[test]
    fn cell_distance_none_across_planes() {
        let origin = pos(Some("s"), 0, 0, 0);
        assert_eq!(origin.cell_distance(&pos(Some("s"), 0, 0, 1)), None); // z only
        assert_eq!(origin.cell_distance(&pos(Some("s2"), 0, 0, 0)), None); // subregion Some/Some
        assert_eq!(origin.cell_distance(&pos(None, 0, 0, 0)), None); // subregion Some/None
        assert_eq!(
            Position {
                region: "other".to_string(),
                subregion: Some("s".to_string()),
                x: 0,
                y: 0,
                z: 0,
            }
            .cell_distance(&origin),
            None // region only
        );
        // same plane (both subregion None) → measured, not excluded.
        assert_eq!(
            pos(None, 0, 0, 0).cell_distance(&pos(None, 1, 0, 0)),
            Some(1)
        );
    }

    // Bands measured through the real Default radii (pins the consts + Default impl):
    // 0→Exact, ==interaction→Interactable, ==sight→Visible, sight+1→None.
    #[test]
    fn classify_bands_through_default_radii() {
        let radii = RadiusConfig::default();
        assert_eq!(Proximity::classify(0, &radii), Some(Proximity::Exact));
        assert_eq!(
            Proximity::classify(1, &radii),
            Some(Proximity::Interactable)
        );
        assert_eq!(Proximity::classify(2, &radii), Some(Proximity::Visible));
        assert_eq!(Proximity::classify(3, &radii), Some(Proximity::Visible));
        assert_eq!(Proximity::classify(4, &radii), None);
    }

    #[test]
    fn default_radii_are_three_and_one() {
        assert_eq!(
            RadiusConfig::default(),
            RadiusConfig {
                sight: 3,
                interaction: 1
            }
        );
        assert_eq!(DEFAULT_SIGHT_RADIUS, 3);
        assert_eq!(DEFAULT_INTERACTION_RADIUS, 1);
    }

    #[test]
    fn proximity_is_interactable_and_as_str() {
        assert!(Proximity::Exact.is_interactable());
        assert!(Proximity::Interactable.is_interactable());
        assert!(!Proximity::Visible.is_interactable());
        assert_eq!(Proximity::Exact.as_str(), "exact");
        assert_eq!(Proximity::Interactable.as_str(), "interactable");
        assert_eq!(Proximity::Visible.as_str(), "visible");
    }

    #[test]
    fn awareness_kind_as_str() {
        assert_eq!(AwarenessKind::Actor.as_str(), "actor");
        assert_eq!(AwarenessKind::Fixture.as_str(), "fixture");
        assert_eq!(AwarenessKind::Item.as_str(), "item");
    }

    // ---- query fixtures ----

    fn room_at(id: &str, subregion: Option<&str>, x: i32, y: i32, z: i32) -> RoomDefinition {
        RoomDefinition {
            id: id.to_string(),
            title: id.to_string(),
            region: "r".to_string(),
            subregion: subregion.map(String::from),
            description: "d".to_string(),
            exits: BTreeMap::new(),
            x,
            y,
            z,
            glyph: '.',
            passable: true,
            entities: Vec::new(),
            items: Vec::new(),
        }
    }

    fn placed(mut room: RoomDefinition, entities: &[&str], items: &[&str]) -> RoomDefinition {
        room.entities = entities.iter().copied().map(String::from).collect();
        room.items = items.iter().copied().map(String::from).collect();
        room
    }

    fn make_entity(
        id: &str,
        name: &str,
        kind: EntityKind,
        aliases: &[&str],
        hidden: bool,
    ) -> Entity {
        Entity {
            id: id.to_string(),
            name: name.to_string(),
            description: format!("desc-{id}"),
            aliases: aliases.iter().copied().map(String::from).collect(),
            kind,
            roles: Vec::new(),
            inventory: Vec::new(),
            hidden,
            dialogue: None,
        }
    }

    fn make_item(id: &str, name: &str, aliases: &[&str], hidden: bool) -> Item {
        Item {
            id: id.to_string(),
            name: name.to_string(),
            description: format!("desc-{id}"),
            aliases: aliases.iter().copied().map(String::from).collect(),
            hidden,
            kind: None,
            flags: Vec::new(),
        }
    }

    fn world(
        rooms: Vec<RoomDefinition>,
        entities: Vec<Entity>,
        items: Vec<Item>,
    ) -> WorldDefinition {
        let start = rooms
            .first()
            .map_or_else(String::new, |room| room.id.clone());
        WorldDefinition {
            id: "w".to_string(),
            title: "W".to_string(),
            start_room_id: start,
            rooms: rooms
                .into_iter()
                .map(|room| (room.id.clone(), room))
                .collect(),
            regions: BTreeMap::new(),
            subregions: BTreeMap::new(),
            entities: entities.into_iter().map(|e| (e.id.clone(), e)).collect(),
            items: items.into_iter().map(|i| (i.id.clone(), i)).collect(),
            oaths: BTreeMap::new(),
            oath_id: None,
        }
    }

    // Room ids are chosen so BTreeMap (id) order ≠ distance order — a deleted
    // `sort_by_key` would reorder the result and fail the nearest-first assert.
    fn nearby_world() -> WorldDefinition {
        let rooms = vec![
            placed(room_at("m_org", Some("s"), 0, 0, 0), &["ally"], &[]),
            placed(room_at("z_near", Some("s"), 1, 0, 0), &["lever"], &["pin"]),
            placed(room_at("k_mid", Some("s"), 2, 0, 0), &["guard"], &["coin"]),
            placed(room_at("a_far", Some("s"), 3, 0, 0), &["scout"], &[]),
            placed(room_at("e_out", Some("s"), 4, 0, 0), &["stranger"], &[]),
            placed(
                room_at("g_hidden", Some("s"), 1, 1, 0),
                &["ghost"],
                &["relic"],
            ),
            placed(room_at("u_up", Some("s"), 0, 0, 1), &["flyer"], &[]),
            placed(room_at("s_sub", Some("s2"), 1, 0, 0), &["other"], &[]),
        ];
        let entities = vec![
            make_entity("ally", "Ally", EntityKind::Actor, &[], false),
            make_entity("lever", "Lever", EntityKind::Fixture, &["handle"], false),
            make_entity("guard", "Guard", EntityKind::Actor, &[], false),
            make_entity("scout", "Scout", EntityKind::Actor, &[], false),
            make_entity("stranger", "Stranger", EntityKind::Actor, &[], false),
            make_entity("ghost", "Ghost", EntityKind::Actor, &[], true),
            make_entity("flyer", "Flyer", EntityKind::Actor, &[], false),
            make_entity("other", "Other", EntityKind::Actor, &[], false),
        ];
        let items = vec![
            make_item("pin", "Pin", &[], false),
            make_item("coin", "Coin", &["gold"], false),
            make_item("relic", "Relic", &[], true),
        ];
        world(rooms, entities, items)
    }

    fn origin_of(world: &WorldDefinition, id: &str) -> RoomDefinition {
        world.rooms.get(id).expect("origin room present").clone()
    }

    // ---- perceive ----

    #[test]
    fn perceive_lists_nearby_sorted_excludes_offplane_and_hidden() {
        let world = nearby_world();
        let origin = origin_of(&world, "m_org");
        let got = perceive(&world, &origin, &RadiusConfig::default());
        let ids: Vec<&str> = got.iter().map(|thing| thing.id.as_str()).collect();
        // nearest-first; ties keep world order (entities before items).
        assert_eq!(ids, vec!["ally", "lever", "pin", "guard", "coin", "scout"]);
        let distances: Vec<u32> = got.iter().map(|thing| thing.distance).collect();
        assert_eq!(distances, vec![0, 1, 1, 2, 2, 3]);
        // excluded: d4, hidden entity + hidden item, off-plane z, off-plane subregion.
        for absent in ["stranger", "ghost", "relic", "flyer", "other"] {
            assert!(!ids.contains(&absent), "{absent} must not be perceived");
        }
    }

    #[test]
    fn perceive_reports_actor_fixture_and_item_kinds() {
        let world = nearby_world();
        let origin = origin_of(&world, "m_org");
        let got = perceive(&world, &origin, &RadiusConfig::default());
        let kind_of = |id: &str| {
            got.iter()
                .find(|thing| thing.id == id)
                .map(|thing| thing.kind)
        };
        assert_eq!(kind_of("ally"), Some(AwarenessKind::Actor));
        assert_eq!(kind_of("lever"), Some(AwarenessKind::Fixture));
        assert_eq!(kind_of("pin"), Some(AwarenessKind::Item));
    }

    #[test]
    fn perceive_classifies_proximity_bands() {
        let world = nearby_world();
        let origin = origin_of(&world, "m_org");
        let got = perceive(&world, &origin, &RadiusConfig::default());
        let prox = |id: &str| {
            got.iter()
                .find(|thing| thing.id == id)
                .map(|thing| thing.proximity)
        };
        assert_eq!(prox("ally"), Some(Proximity::Exact)); // d0
        assert_eq!(prox("lever"), Some(Proximity::Interactable)); // d1
        assert_eq!(prox("guard"), Some(Proximity::Visible)); // d2
        assert_eq!(prox("scout"), Some(Proximity::Visible)); // d3
                                                             // description is carried for the look command (REQ-004 / deviation #2).
        let ally = got.iter().find(|thing| thing.id == "ally").expect("ally");
        assert_eq!(ally.description, "desc-ally");
    }

    // ---- resolve_target ----

    #[test]
    fn resolve_matches_by_name_only() {
        let world = nearby_world();
        let origin = origin_of(&world, "m_org");
        // "Guard" has no aliases — a name-only match (kills the name-side drop).
        let found = resolve_target(&world, &origin, &RadiusConfig::default(), "guard")
            .expect("guard resolves by name");
        assert_eq!(found.id, "guard");
    }

    #[test]
    fn resolve_matches_by_alias_only_case_insensitive() {
        let world = nearby_world();
        let origin = origin_of(&world, "m_org");
        // "Lever" with alias "handle" — query "HANDLE" matches the alias only.
        let found = resolve_target(&world, &origin, &RadiusConfig::default(), "HANDLE")
            .expect("lever resolves by alias");
        assert_eq!(found.id, "lever");
    }

    #[test]
    fn resolve_none_for_unknown_hidden_or_out_of_sight() {
        let world = nearby_world();
        let origin = origin_of(&world, "m_org");
        let radii = RadiusConfig::default();
        assert!(resolve_target(&world, &origin, &radii, "dragon").is_none()); // unknown
        assert!(resolve_target(&world, &origin, &radii, "Ghost").is_none()); // hidden
        assert!(resolve_target(&world, &origin, &radii, "Stranger").is_none()); // d4, out of sight
        assert!(resolve_target(&world, &origin, &radii, "").is_none()); // empty query
    }

    #[test]
    fn resolve_exact_beats_nearby() {
        // Two "Echo"s: room "a" (d2) sorts BEFORE origin "m" (d0) by id, so without
        // the distance sort the far one would win — the exact (nearest) must win.
        let rooms = vec![
            placed(room_at("m", Some("s"), 0, 0, 0), &["echo_here"], &[]),
            placed(room_at("a", Some("s"), 2, 0, 0), &["echo_far"], &[]),
        ];
        let entities = vec![
            make_entity("echo_here", "Echo", EntityKind::Actor, &[], false),
            make_entity("echo_far", "Echo", EntityKind::Actor, &[], false),
        ];
        let world = world(rooms, entities, Vec::new());
        let origin = origin_of(&world, "m");
        let found = resolve_target(&world, &origin, &RadiusConfig::default(), "echo")
            .expect("echo resolves");
        assert_eq!(found.id, "echo_here");
        assert_eq!(found.distance, 0);
        assert_eq!(found.proximity, Proximity::Exact);
    }

    // ---- ticket #18: room_id is the placing room ----

    // The resolver and perceive carry `room_id` = the room that PLACES the thing
    // (not the observer's origin) — for an origin-cell entity, an adjacent-cell
    // entity, and an adjacent-cell item — so `take` can mutate the exact room.
    #[test]
    fn awareness_carries_the_placing_room_id() {
        let world = nearby_world();
        let origin = origin_of(&world, "m_org");
        let radii = RadiusConfig::default();
        // ally is in the origin cell m_org (distance 0).
        let ally =
            resolve_target(&world, &origin, &radii, "ally").expect("ally resolves in origin cell");
        assert_eq!(ally.room_id, "m_org");
        // lever (entity) is in the adjacent cell z_near — room_id is its cell, not origin.
        let lever =
            resolve_target(&world, &origin, &radii, "lever").expect("lever resolves nearby");
        assert_eq!(lever.room_id, "z_near");
        // pin (item) is also in z_near — covers the item push site.
        let pin = perceive(&world, &origin, &radii)
            .into_iter()
            .find(|thing| thing.id == "pin")
            .expect("pin perceived");
        assert_eq!(pin.room_id, "z_near");
    }
}
