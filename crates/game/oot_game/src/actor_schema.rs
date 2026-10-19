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

/// An object an actor needs loaded besides its profile's: one it finds a slot for itself
/// (`Object_GetSlot`) for its model, its animations or what it spawns, always or for some values
/// of a field (`Obj_Tsubo`'s pot by bit 8). Without it the actor waits, or kills itself; a room
/// it's placed in lists it, as the game's own rooms do (the editor puts it there).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ObjectNeed {
    /// The object's file: "object_tsubo".
    pub object: &'static str,
    pub when: Option<When>,
    /// What it's for, in a few words.
    pub note: &'static str,
}

impl ObjectNeed {
    pub const fn new(object: &'static str) -> Self {
        ObjectNeed { object, when: None, note: "" }
    }

    /// Only for some values of a field.
    pub const fn when(self, field: &'static str, values: &'static [i32]) -> Self {
        ObjectNeed { when: Some(When { field, values }), ..self }
    }

    pub const fn note(self, note: &'static str) -> Self {
        ObjectNeed { note, ..self }
    }
}

/// A line an actor can say: a text id its code shows (`Message_StartTextbox`,
/// `Message_ContinueTextbox`, the `textId` it offers to talk with), when, and a note. The lines
/// of a schema are in the order its code uses them; the texts a message goes on to by itself
/// (its `{TEXTID}`) aren't listed, the editor follows them.
///
/// The ids are the ones a custom level plays (a scene of its own: docs/EDITOR-PLAY-IN-THE-GAME.md);
/// lines only some scenes show say so in `cond`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TextLine {
    /// The text id; with `plus`, the base the field's value goes into (`0x300 | params`).
    pub id: u16,
    /// A field whose value is or-ed into `id` (a sign's text, a Gossip Stone's): the C's
    /// `0x300 | params`, `0x100 + text` (the same, its bits being below the base's).
    pub plus: Option<&'static str>,
    /// Only for some values of a field (a type's).
    pub when: Option<When>,
    /// When, in words, as the code decides it: "none of the cuccos in the pen", "after the Kokiri
    /// Sword". Empty: whenever it talks.
    pub cond: &'static str,
    /// What it's for, in a few words.
    pub note: &'static str,
}

impl TextLine {
    pub const fn new(id: u16) -> Self {
        TextLine { id, plus: None, when: None, cond: "", note: "" }
    }
    /// `base | field`'s value.
    pub const fn field(base: u16, field: &'static str) -> Self {
        TextLine { id: base, plus: Some(field), when: None, cond: "", note: "" }
    }
    /// Only for some values of a field.
    pub const fn when(self, field: &'static str, values: &'static [i32]) -> Self {
        TextLine { when: Some(When { field, values }), ..self }
    }
    pub const fn cond(self, cond: &'static str) -> Self {
        TextLine { cond, ..self }
    }
    pub const fn note(self, note: &'static str) -> Self {
        TextLine { note, ..self }
    }
}

/// An actor's configuration: its fields, its flags, and the objects it needs besides its
/// profile's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ActorSchema {
    pub fields: &'static [Field],
    pub flags: &'static [FlagUse],
    /// Not written yet (BACKLOG #52's pass); the schema test fails on it.
    pub todo: bool,
    /// The objects it finds slots for itself, beyond its profile's (`ObjectNeed`).
    pub objects: &'static [ObjectNeed],
    /// The lines it can say (`TextLine`), in the order its code uses them.
    pub texts: &'static [TextLine],
}

impl ActorSchema {
    /// An actor that takes nothing from its placement and uses no flags.
    pub const NONE: ActorSchema = ActorSchema { fields: &[], flags: &[], todo: false, objects: &[], texts: &[] };
    /// Not written yet.
    pub const TODO: ActorSchema = ActorSchema { fields: &[], flags: &[], todo: true, objects: &[], texts: &[] };

    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// The value of the field `name` in a placement's params and rotation.
    pub fn value_of(&self, name: &str, params: i16, rot: [i16; 3]) -> Option<i32> {
        let f = self.field(name)?;
        let v = match f.source {
            Source::Params => params,
            Source::RotX => rot[0],
            Source::RotY => rot[1],
            Source::RotZ => rot[2],
        };
        Some(f.value(v))
    }

    /// The objects a placement with these params and rotation needs besides the profile's: the
    /// `objects` whose `when` holds.
    pub fn needed_objects(&self, params: i16, rot: [i16; 3]) -> Vec<&'static str> {
        self.objects
            .iter()
            .filter(|n| n.when.is_none_or(|w| self.value_of(w.field, params, rot).is_some_and(|v| w.values.contains(&v))))
            .map(|n| n.object)
            .collect()
    }

    /// The lines a placement with these params and rotation can say: each one whose `when` holds,
    /// with its id (`plus`'s field or-ed in).
    pub fn lines(&self, params: i16, rot: [i16; 3]) -> Vec<(u16, &'static TextLine)> {
        self.texts
            .iter()
            .filter(|t| t.when.is_none_or(|w| self.value_of(w.field, params, rot).is_some_and(|v| w.values.contains(&v))))
            .map(|t| (t.plus.and_then(|f| self.value_of(f, params, rot)).map_or(t.id, |v| t.id | v as u16), t))
            .collect()
    }

    /// The rotation components a placement's actor reads as settings, kept from its rotation
    /// (the rest 0): what tells two of its configurations apart, beside the params.
    pub fn setting_rot(&self, rot: [i16; 3]) -> [i16; 3] {
        let mut out = [0i16; 3];
        for f in self.fields.iter().filter(|f| f.kind != Kind::Rotation) {
            let k = match f.source {
                Source::Params => continue,
                Source::RotX => 0,
                Source::RotY => 1,
                Source::RotZ => 2,
            };
            out[k] = ((out[k] as u16) | (rot[k] as u16 & f.mask())) as i16;
        }
        out
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

/// `GetItemID` (`include/item.h`), for `Kind::ItemId` fields: the get-item ids by name.
pub const GI_NAMES: &[(i32, &str)] = &[
    (0x00, "GI_NONE"),
    (0x01, "GI_BOMBS_5"),
    (0x02, "GI_DEKU_NUTS_5"),
    (0x03, "GI_BOMBCHUS_10"),
    (0x04, "GI_BOW"),
    (0x05, "GI_SLINGSHOT"),
    (0x06, "GI_BOOMERANG"),
    (0x07, "GI_DEKU_STICKS_1"),
    (0x08, "GI_HOOKSHOT"),
    (0x09, "GI_LONGSHOT"),
    (0x0A, "GI_LENS_OF_TRUTH"),
    (0x0B, "GI_ZELDAS_LETTER"),
    (0x0C, "GI_OCARINA_OF_TIME"),
    (0x0D, "GI_HAMMER"),
    (0x0E, "GI_COJIRO"),
    (0x0F, "GI_BOTTLE_EMPTY"),
    (0x10, "GI_BOTTLE_POTION_RED"),
    (0x11, "GI_BOTTLE_POTION_GREEN"),
    (0x12, "GI_BOTTLE_POTION_BLUE"),
    (0x13, "GI_BOTTLE_FAIRY"),
    (0x14, "GI_BOTTLE_MILK_FULL"),
    (0x15, "GI_BOTTLE_RUTOS_LETTER"),
    (0x16, "GI_MAGIC_BEAN"),
    (0x17, "GI_MASK_SKULL"),
    (0x18, "GI_MASK_SPOOKY"),
    (0x19, "GI_CHICKEN"),
    (0x1A, "GI_MASK_KEATON"),
    (0x1B, "GI_MASK_BUNNY_HOOD"),
    (0x1C, "GI_MASK_TRUTH"),
    (0x1D, "GI_POCKET_EGG"),
    (0x1E, "GI_POCKET_CUCCO"),
    (0x1F, "GI_ODD_MUSHROOM"),
    (0x20, "GI_ODD_POTION"),
    (0x21, "GI_POACHERS_SAW"),
    (0x22, "GI_BROKEN_GORONS_SWORD"),
    (0x23, "GI_PRESCRIPTION"),
    (0x24, "GI_EYEBALL_FROG"),
    (0x25, "GI_EYE_DROPS"),
    (0x26, "GI_CLAIM_CHECK"),
    (0x27, "GI_SWORD_KOKIRI"),
    (0x28, "GI_SWORD_KNIFE"),
    (0x29, "GI_SHIELD_DEKU"),
    (0x2A, "GI_SHIELD_HYLIAN"),
    (0x2B, "GI_SHIELD_MIRROR"),
    (0x2C, "GI_TUNIC_GORON"),
    (0x2D, "GI_TUNIC_ZORA"),
    (0x2E, "GI_BOOTS_IRON"),
    (0x2F, "GI_BOOTS_HOVER"),
    (0x30, "GI_QUIVER_40"),
    (0x31, "GI_QUIVER_50"),
    (0x32, "GI_BOMB_BAG_20"),
    (0x33, "GI_BOMB_BAG_30"),
    (0x34, "GI_BOMB_BAG_40"),
    (0x35, "GI_SILVER_GAUNTLETS"),
    (0x36, "GI_GOLD_GAUNTLETS"),
    (0x37, "GI_SCALE_SILVER"),
    (0x38, "GI_SCALE_GOLDEN"),
    (0x39, "GI_STONE_OF_AGONY"),
    (0x3A, "GI_GERUDOS_CARD"),
    (0x3B, "GI_OCARINA_FAIRY"),
    (0x3C, "GI_DEKU_SEEDS_5"),
    (0x3D, "GI_HEART_CONTAINER"),
    (0x3E, "GI_HEART_PIECE"),
    (0x3F, "GI_BOSS_KEY"),
    (0x40, "GI_COMPASS"),
    (0x41, "GI_DUNGEON_MAP"),
    (0x42, "GI_SMALL_KEY"),
    (0x43, "GI_MAGIC_JAR_SMALL"),
    (0x44, "GI_MAGIC_JAR_LARGE"),
    (0x45, "GI_WALLET_ADULT"),
    (0x46, "GI_WALLET_GIANT"),
    (0x47, "GI_WEIRD_EGG"),
    (0x48, "GI_RECOVERY_HEART"),
    (0x49, "GI_ARROWS_5"),
    (0x4A, "GI_ARROWS_10"),
    (0x4B, "GI_ARROWS_30"),
    (0x4C, "GI_RUPEE_GREEN"),
    (0x4D, "GI_RUPEE_BLUE"),
    (0x4E, "GI_RUPEE_RED"),
    (0x4F, "GI_HEART_CONTAINER_2"),
    (0x50, "GI_MILK"),
    (0x51, "GI_MASK_GORON"),
    (0x52, "GI_MASK_ZORA"),
    (0x53, "GI_MASK_GERUDO"),
    (0x54, "GI_GORONS_BRACELET"),
    (0x55, "GI_RUPEE_PURPLE"),
    (0x56, "GI_RUPEE_GOLD"),
    (0x57, "GI_SWORD_BIGGORON"),
    (0x58, "GI_ARROW_FIRE"),
    (0x59, "GI_ARROW_ICE"),
    (0x5A, "GI_ARROW_LIGHT"),
    (0x5B, "GI_SKULL_TOKEN"),
    (0x5C, "GI_DINS_FIRE"),
    (0x5D, "GI_FARORES_WIND"),
    (0x5E, "GI_NAYRUS_LOVE"),
    (0x5F, "GI_BULLET_BAG_30"),
    (0x60, "GI_BULLET_BAG_40"),
    (0x61, "GI_DEKU_STICKS_5"),
    (0x62, "GI_DEKU_STICKS_10"),
    (0x63, "GI_DEKU_NUTS_5_2"),
    (0x64, "GI_DEKU_NUTS_10"),
    (0x65, "GI_BOMBS_1"),
    (0x66, "GI_BOMBS_10"),
    (0x67, "GI_BOMBS_20"),
    (0x68, "GI_BOMBS_30"),
    (0x69, "GI_DEKU_SEEDS_30"),
    (0x6A, "GI_BOMBCHUS_5"),
    (0x6B, "GI_BOMBCHUS_20"),
    (0x6C, "GI_BOTTLE_FISH"),
    (0x6D, "GI_BOTTLE_BUGS"),
    (0x6E, "GI_BOTTLE_BLUE_FIRE"),
    (0x6F, "GI_BOTTLE_POE"),
    (0x70, "GI_BOTTLE_BIG_POE"),
    (0x71, "GI_DOOR_KEY"),
    (0x72, "GI_RUPEE_GREEN_LOSE"),
    (0x73, "GI_RUPEE_BLUE_LOSE"),
    (0x74, "GI_RUPEE_RED_LOSE"),
    (0x75, "GI_RUPEE_PURPLE_LOSE"),
    (0x76, "GI_HEART_PIECE_WIN"),
    (0x77, "GI_DEKU_STICK_UPGRADE_20"),
    (0x78, "GI_DEKU_STICK_UPGRADE_30"),
    (0x79, "GI_DEKU_NUT_UPGRADE_30"),
    (0x7A, "GI_DEKU_NUT_UPGRADE_40"),
    (0x7B, "GI_BULLET_BAG_50"),
    (0x7C, "GI_ICE_TRAP"),
    (0x7D, "GI_TEXT_0"),
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
