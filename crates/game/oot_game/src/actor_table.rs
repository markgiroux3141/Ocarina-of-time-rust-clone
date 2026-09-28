//! The actor table (`include/tables/actor_table.h`, `gActorOverlayTable`) with each actor's
//! `ActorInit` (`<Name>_InitVars`): its category, initial flags, object and functions. An
//! asset-pack record (`pack::keys::ACTORS`), read by the importer from the decomp's C.
//!
//! `Actor_Spawn` reads an actor's profile from here, so an actor that isn't ported yet still
//! spawns into the right category, obeys its object dependency and gets its flags: as a
//! placeholder (`crate::spawn`).

/// `ACTOROVL_ALLOC_*`.
pub const ACTOROVL_ALLOC_NORMAL: u8 = 0;
pub const ACTOROVL_ALLOC_ABSOLUTE: u8 = 1 << 0;
pub const ACTOROVL_ALLOC_PERSISTENT: u8 = 1 << 1;

/// An `ActorInit`, as the overlay defines it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActorInitInfo {
    /// `id`: the `ACTOR_*` id the actor gets (`Actor_Spawn` copies it). The row's own id,
    /// except where the overlay says another (@bug (game)): `Boss_Dodongo` is
    /// `ACTOR_EN_DODONGO`, `En_Goma` `ACTOR_BOSS_GOMA`, and `En_Bdfire`, `En_Fhg_Fire`,
    /// `En_Heishi1` and `En_Vb_Ball` are 0 (`ACTOR_PLAYER`).
    pub id: i16,
    /// `ACTORCAT_*`.
    pub category: u8,
    /// `FLAGS`, evaluated.
    pub flags: u32,
    /// `OBJECT_*` id.
    pub object_id: i16,
    /// The functions, by name (`None` for `NULL`).
    pub init: Option<String>,
    pub destroy: Option<String>,
    pub update: Option<String>,
    pub draw: Option<String>,
}

/// A row of the actor table.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActorInfo {
    /// `ACTOR_EN_HOLL`.
    pub enum_name: String,
    /// The overlay's name, e.g. `En_Holl` (empty for `DEFINE_ACTOR_UNSET`).
    pub name: String,
    /// `ACTOROVL_ALLOC_*`.
    pub alloc: u8,
    /// `DEFINE_ACTOR_INTERNAL`: in `code`, not an overlay.
    pub internal: bool,
    /// `None` for unset rows, and for actors whose `ActorInit` the importer couldn't read.
    pub init: Option<ActorInitInfo>,
}

/// `gActorOverlayTable`, indexed by `ACTOR_*` id.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActorTable {
    pub actors: Vec<ActorInfo>,
}

impl ActorTable {
    pub fn get(&self, id: i16) -> Option<&ActorInfo> {
        usize::try_from(id).ok().and_then(|i| self.actors.get(i))
    }

    /// An id by its `ACTOR_*` name.
    pub fn id(&self, enum_name: &str) -> Option<i16> {
        self.actors.iter().position(|a| a.enum_name == enum_name).map(|i| i as i16)
    }

    /// The overlay name of `id`, or its number.
    pub fn name(&self, id: i16) -> String {
        match self.get(id) {
            Some(a) if !a.name.is_empty() => a.name.clone(),
            _ => format!("actor {id:#06x}"),
        }
    }
}
