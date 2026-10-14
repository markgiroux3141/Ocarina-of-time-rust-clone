//! Each actor's configuration as data: what it takes from where it's placed (its params' bit
//! fields, the rotation components some use as settings) and the flags it reads and sets
//! (docs/adr/0066-the-actor-schema.md).
//!
//! - **One schema per actor**, a `pub const SCHEMA: ActorSchema` in the actor's module, next to
//!   its Params and Flags paragraphs (ADR 0062), and in its `ActorProfile` (`schema`).
//! - **The code reads through the fields.** Each field is a `pub const` of the module, and the
//!   actor's code reads its params with it (`TYPE.get(this.actor.params)`), as the C reads them
//!   with `PARAMS_GET_U`/`PARAMS_GET_S`. `oot_actors/tests/schemas.rs` checks that no module reads
//!   its params any other way, that every field is read, that the flag helpers the module calls
//!   match the schema's flags, and that every placement of the actor in the game's scenes decodes.
//! - **The editor reads them** from `oot_actors::PROFILES` (each profile's `schema`), or from the
//!   JSON `ootx actor-schemas` writes.

use serde::Serialize;

/// What a field is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Source {
    /// `actor.params`.
    Params,
    /// The placement's rotation, used as a setting (`home.rot`, `world.rot` and `shape.rot` are
    /// all the placement's at spawn).
    RotX,
    RotY,
    RotZ,
}

/// What a field's values mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Kind {
    /// A list of named values; no other value is placed.
    Named(&'static [(i32, &'static str)]),
    /// A number from `.0` to `.1` (inclusive).
    Range(i32, i32),
    /// A get-item id (`GI_*`, `sGetItemTable`).
    ItemId,
    /// A collectible's id (`ITEM00_*`, an `En_Item00`'s params); out of range is none.
    Drop,
    /// A text id.
    TextId,
    /// An index into the scene layer's paths.
    PathIndex,
    /// A switch flag (0x00 to 0x3F; 0x20 and up are temporary: the room's).
    SwitchFlag,
    /// A chest (treasure) flag (0x00 to 0x1F).
    ChestFlag,
    /// A collectible flag (0x00 to 0x3F).
    CollectibleFlag,
    /// A temporary switch flag, 0x20 to 0x3F (cleared on leaving the room).
    TempFlag,
    /// An object id (`OBJECT_*`).
    Object,
    /// A debug-only value: read only by debug code (`BREG`, prints), nothing for the editor to set.
    Debug,
    /// Set in the game's data, read by nothing.
    Unused,
    /// A rotation component placed non-zero and used as a rotation, not a setting.
    Rotation,
}

/// A field that applies only when another field has one of some values (`type`'s 4 and 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct When {
    pub field: &'static str,
    pub values: &'static [i32],
}

/// One field of an actor's configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Field {
    /// The C's name for it: its `PARAMS_GET_*` macro's, the struct member it's read into, or its
    /// local's.
    pub name: &'static str,
    pub source: Source,
    /// Its lowest bit.
    pub shift: u8,
    /// How many bits (16 and shift 0: the whole value).
    pub bits: u8,
    /// Sign-extended from its top bit (a whole params read as an `s16`).
    pub signed: bool,
    pub kind: Kind,
    /// What it does, in a few words.
    pub meaning: &'static str,
    pub when: Option<When>,
    /// Bits of another field that the code reads whole and then picks apart (`rdFlags & 0x7F`):
    /// that field's name. The code reads the whole one through its field.
    pub part_of: Option<&'static str>,
}

impl Field {
    /// `PARAMS_GET_U(params, shift, bits)` (and `PARAMS_GET_S`, the same value for a field
    /// below bit 16).
    pub const fn params(name: &'static str, shift: u8, bits: u8, kind: Kind, meaning: &'static str) -> Self {
        Field { name, source: Source::Params, shift, bits, signed: false, kind, meaning, when: None, part_of: None }
    }

    /// The whole params as an `s16`.
    pub const fn whole(name: &'static str, kind: Kind, meaning: &'static str) -> Self {
        Field { name, source: Source::Params, shift: 0, bits: 16, signed: true, kind, meaning, when: None, part_of: None }
    }

    /// A rotation component (or some of its bits) used as a setting.
    pub const fn rot(name: &'static str, source: Source, shift: u8, bits: u8, kind: Kind, meaning: &'static str) -> Self {
        Field { name, source, shift, bits, signed: bits == 16, kind, meaning, when: None, part_of: None }
    }

    /// Sign-extended from its top bit (`PARAMS_GET_NOMASK`, `(s8)params`).
    pub const fn signed(self) -> Self {
        Field { signed: true, ..self }
    }

    /// Only for some values of another field.
    pub const fn when(self, field: &'static str, values: &'static [i32]) -> Self {
        Field { when: Some(When { field, values }), ..self }
    }

    /// Some bits of `field`, which the code reads whole.
    pub const fn part_of(self, field: &'static str) -> Self {
        Field { part_of: Some(field), ..self }
    }

    /// The bits it covers in its source.
    pub const fn mask(&self) -> u16 {
        let m: u32 = if self.bits >= 16 { 0xFFFF } else { (1u32 << self.bits) - 1 };
        (m << self.shift) as u16
    }

    /// Its value in `v` (the params, or the rotation component), as the code reads it: unsigned
    /// fields as their bits, signed ones sign-extended; a 16-bit unsigned field wraps to `i16`.
    pub const fn get(&self, v: i16) -> i16 {
        self.value(v) as i16
    }

    /// Its value in `v` as a number (a 16-bit unsigned field stays positive).
    pub const fn value(&self, v: i16) -> i32 {
        let raw = ((v as u16 as u32) >> self.shift) & (self.mask() as u32 >> self.shift);
        if self.signed && self.bits > 0 && raw & (1 << (self.bits - 1)) != 0 { raw as i32 - (1i32 << self.bits) } else { raw as i32 }
    }

    /// `v` with this field set to `value` (what the editor writes).
    pub const fn put(&self, v: i16, value: i32) -> i16 {
        let m = self.mask();
        ((v as u16 & !m) | (((value as u32) << self.shift) as u16 & m)) as i16
    }
}

/// A flag an actor reads or writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Flag {
    /// A switch flag (`Flags_GetSwitch`): a field's, by its name, or a fixed one ("0x1F").
    Switch(&'static str),
    /// A chest flag (`Flags_GetTreasure`): a field's name or a fixed one.
    Chest(&'static str),
    /// A collectible flag (`Flags_GetCollectible`): a field's name or a fixed one.
    Collectible(&'static str),
    /// The room's clear flag (`Flags_GetClear`).
    Clear,
    /// The room's temporary clear flag (`Flags_GetTempClear`).
    TempClear,
    /// `EVENTCHKINF_*`, by name and value.
    EventChkInf(&'static str, u16),
    /// `INFTABLE_*`.
    InfTable(&'static str, u16),
    /// `ITEMGETINF_*`.
    ItemGetInf(&'static str, u16),
    /// `EVENTINF_*`.
    EventInf(&'static str, u16),
    /// Anything else kept as a flag: `gSaveContext.sunsSongState`, `gInterruptSongOfStorms`.
    Other(&'static str),
}

/// How an actor uses a flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FlagUse {
    pub flag: Flag,
    pub reads: bool,
    pub sets: bool,
    pub clears: bool,
    /// When or why, in a few words ("type 6", "once dead").
    pub note: &'static str,
    /// Not through a call in the actor's module (a shared helper's, a table of flags, a computed
    /// one): why. The test then doesn't look for the call.
    pub indirect: &'static str,
}

impl FlagUse {
    pub const fn new(flag: Flag) -> Self {
        FlagUse { flag, reads: false, sets: false, clears: false, note: "", indirect: "" }
    }
    pub const fn reads(self) -> Self {
        FlagUse { reads: true, ..self }
    }
    pub const fn sets(self) -> Self {
        FlagUse { sets: true, ..self }
    }
    pub const fn clears(self) -> Self {
        FlagUse { clears: true, ..self }
    }
    pub const fn note(self, note: &'static str) -> Self {
        FlagUse { note, ..self }
    }
    pub const fn indirect(self, why: &'static str) -> Self {
        FlagUse { indirect: why, ..self }
    }
}

/// An actor's configuration: its fields and its flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ActorSchema {
    pub fields: &'static [Field],
    pub flags: &'static [FlagUse],
    /// Not written yet (BACKLOG #52's pass); the schema test fails on it.
    pub todo: bool,
}

impl ActorSchema {
    /// An actor that takes nothing from its placement and uses no flags.
    pub const NONE: ActorSchema = ActorSchema { fields: &[], flags: &[], todo: false };
    /// Not written yet.
    pub const TODO: ActorSchema = ActorSchema { fields: &[], flags: &[], todo: true };

    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }
}

/// `Flag::EventChkInf` from a constant: its name and value.
#[macro_export]
macro_rules! event_chk_inf {
    ($c:ident) => {
        $crate::actor_schema::Flag::EventChkInf(stringify!($c), $c)
    };
}

/// `Flag::InfTable` from a constant.
#[macro_export]
macro_rules! inf_table {
    ($c:ident) => {
        $crate::actor_schema::Flag::InfTable(stringify!($c), $c)
    };
}

/// `Flag::ItemGetInf` from a constant.
#[macro_export]
macro_rules! item_get_inf {
    ($c:ident) => {
        $crate::actor_schema::Flag::ItemGetInf(stringify!($c), $c)
    };
}

/// `Flag::EventInf` from a constant.
#[macro_export]
macro_rules! event_inf {
    ($c:ident) => {
        $crate::actor_schema::Flag::EventInf(stringify!($c), $c)
    };
}

/// `ITEM00_*` (`z_en_item00.h`), for `Kind::Named` fields that list the collectibles.
pub const ITEM00_NAMES: &[(i32, &str)] = &[
    (0x00, "ITEM00_RUPEE_GREEN"),
    (0x01, "ITEM00_RUPEE_BLUE"),
    (0x02, "ITEM00_RUPEE_RED"),
    (0x03, "ITEM00_RECOVERY_HEART"),
    (0x04, "ITEM00_BOMBS_A"),
    (0x05, "ITEM00_ARROWS_SINGLE"),
    (0x06, "ITEM00_HEART_PIECE"),
    (0x07, "ITEM00_HEART_CONTAINER"),
    (0x08, "ITEM00_ARROWS_SMALL"),
    (0x09, "ITEM00_ARROWS_MEDIUM"),
    (0x0A, "ITEM00_ARROWS_LARGE"),
    (0x0B, "ITEM00_BOMBS_B"),
    (0x0C, "ITEM00_NUTS"),
    (0x0D, "ITEM00_STICK"),
    (0x0E, "ITEM00_MAGIC_LARGE"),
    (0x0F, "ITEM00_MAGIC_SMALL"),
    (0x10, "ITEM00_SEEDS"),
    (0x11, "ITEM00_SMALL_KEY"),
    (0x12, "ITEM00_FLEXIBLE"),
    (0x13, "ITEM00_RUPEE_ORANGE"),
    (0x14, "ITEM00_RUPEE_PURPLE"),
    (0x15, "ITEM00_SHIELD_DEKU"),
    (0x16, "ITEM00_SHIELD_HYLIAN"),
    (0x17, "ITEM00_TUNIC_ZORA"),
    (0x18, "ITEM00_TUNIC_GORON"),
    (0x19, "ITEM00_BOMBS_SPECIAL"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_read_as_the_c() {
        // PARAMS_GET_U(params, 10, 6) of 0x1C3F (En_Okarina_Tag's type 7, any song, no flag).
        let ty = Field::params("type", 10, 6, Kind::Range(0, 63), "");
        assert_eq!(ty.get(0x1FFFu16 as i16), 7);
        assert_eq!(ty.mask(), 0xFC00);
        // The top bits of a negative params.
        assert_eq!(ty.get(0xFC00u16 as i16), 0x3F);
        // A whole params as an s16; En_Rd's type as an s8.
        assert_eq!(Field::whole("p", Kind::Debug, "").get(-3), -3);
        let rd = Field::params("type", 0, 8, Kind::Debug, "").signed();
        assert_eq!(rd.value(0x7FFD), -3);
        assert_eq!(rd.value(0x7F02), 2);
        // A 16-bit unsigned field.
        let u = Field::params("u", 0, 16, Kind::Debug, "");
        assert_eq!(u.value(-1), 0xFFFF);
        assert_eq!(u.get(-1), -1);
        // Putting a value back.
        assert_eq!(ty.put(0x003F, 7), 0x1C3F);
        assert_eq!(rd.put(0, -3), 0x00FD);
    }
}
