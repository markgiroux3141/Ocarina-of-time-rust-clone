//! Player constants and animations: the game tables Player, the camera and foot IK read, as
//! the asset pack stores them (`table/player`, and Link's animations under
//! `anim/gameplay_keep/`). `oot_import::tables` builds them from the decomp source and the
//! ROM:
//!
//! - REG values: `sBootData` and the assignments in `Player_SetBootData` (`z_player_lib.c`).
//!   The function body is scanned for `XREG(n) = bootRegs[k];` and `XREG(n) = literal;`, so
//!   the mapping itself comes from the source.
//! - `sAgeProperties` (`z_player.c`): the 17 leading float fields.
//! - `D_80853914` (`GET_PLAYER_ANIM(group, type)`, `z_player.c`): animation group table.
//! - Link's animations: `gameplay_keep` headers into `link_animetion` (see `eng_anim::anim`).

#![allow(non_snake_case)] // fields keep the decomp's unk_XXX names

use std::collections::{BTreeMap, HashMap};

/// `PLAYER_BOOTS_*` rows of `sBootData`.
pub const BOOTS_KOKIRI: usize = 0;
pub const BOOTS_KOKIRI_CHILD: usize = 5;

/// The `REG`-group debug registers Player reads, as set by `Player_SetBootData`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Regs {
    /// e.g. `"REG(19)"` → 200.
    pub values: BTreeMap<String, i16>,
}

impl Regs {
    pub fn get(&self, name: &str) -> i16 {
        *self.values.get(name).unwrap_or_else(|| panic!("{name} not set by Player_SetBootData"))
    }
    /// `REG(n)`.
    pub fn reg(&self, n: u32) -> i16 {
        self.get(&format!("REG({n})"))
    }
    /// `IREG(n)`.
    pub fn ireg(&self, n: u32) -> i16 {
        self.get(&format!("IREG({n})"))
    }
    /// `MREG(n)`.
    pub fn mreg(&self, n: u32) -> i16 {
        self.get(&format!("MREG({n})"))
    }
    /// `R_RUN_SPEED_LIMIT` (`REG(45)`, `regs.h`).
    pub fn run_speed_limit(&self) -> f32 {
        self.reg(45) as f32 / 100.0
    }
}

/// `PlayerAgeProperties` float fields `ceilingCheckHeight` .. `unk_40` (the struct has no names yet in
/// this decomp; the uses noted are from `z_player.c`).
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AgeProperties {
    /// ceilingCheckHeight: ceiling check height in `Player_ProcessSceneCollision`.
    pub ceiling_check_height: f32,
    /// unk_04: shadow scale (`ActorShape_Init`).
    pub shadow_scale: f32,
    /// unk_08: root translation scale for animation-driven movement.
    pub translation_scale: f32,
    /// unk_0C: height above Link the wall-top probe starts from.
    pub wall_probe_height: f32,
    pub unk_10: f32,
    pub unk_14: f32,
    pub unk_18: f32,
    pub unk_1C: f32,
    pub unk_20: f32,
    pub unk_24: f32,
    pub unk_28: f32,
    pub unk_2C: f32,
    pub unk_30: f32,
    /// unk_34: minimum drop for a ledge grab when walking off an edge (`func_8083AA10`).
    pub ledge_grab_min_drop: f32,
    /// wallCheckRadius: wall check radius.
    pub wall_radius: f32,
    pub unk_3C: f32,
    pub unk_40: f32,
    /// The rest of the struct: the climbing animations and their root offsets.
    pub climb: AgeClimb,
}

/// `PlayerAgeProperties` from `unk_44`: root translations the climbing animations start from
/// (`skelAnime.prevTransl`), and the animations (`Player_Action_8084BF1C` and friends).
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AgeClimb {
    pub unk_44: [i16; 3],
    pub unk_4A: [[i16; 3]; 4],
    pub unk_62: [[i16; 3]; 4],
    pub unk_7A: [[i16; 3]; 2],
    pub unk_86: [[i16; 3]; 2],
    pub unk_92: u16,
    pub unk_94: u16,
    /// Chest opening, the pedestal warps (unused here).
    pub unk_98: AnimId,
    pub time_travel_start_anim: AnimId,
    pub time_travel_end_anim: AnimId,
    /// `climb_startA` (onto a ladder from below), `climb_startB` (onto it from its top).
    pub unk_A4: AnimId,
    pub unk_A8: AnimId,
    /// `climb_upL/R`, `Fclimb_upL/R`: one rung.
    pub unk_AC: [AnimId; 4],
    /// `Fclimb_sideL/R`.
    pub unk_BC: [AnimId; 2],
    /// `climb_endAL/R` (off the bottom), `climb_endBR/L` (off the top).
    pub unk_C4: [AnimId; 2],
    pub unk_CC: [AnimId; 2],
}

impl AgeProperties {
    pub fn from_floats(f: &[f32]) -> AgeProperties {
        AgeProperties {
            ceiling_check_height: f[0],
            shadow_scale: f[1],
            translation_scale: f[2],
            wall_probe_height: f[3],
            unk_10: f[4],
            unk_14: f[5],
            unk_18: f[6],
            unk_1C: f[7],
            unk_20: f[8],
            unk_24: f[9],
            unk_28: f[10],
            unk_2C: f[11],
            unk_30: f[12],
            ledge_grab_min_drop: f[13],
            wall_radius: f[14],
            unk_3C: f[15],
            unk_40: f[16],
            climb: AgeClimb::default(),
        }
    }
}

pub type AnimId = usize;

/// One entry of `D_80854190`: a melee attack's animation, its end animations (normal / locked
/// on), and the frames the weapon is active between (`unk_0C`, `unk_0D`).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AttackAnim {
    pub anim: AnimId,
    pub end: AnimId,
    pub end_locked: AnimId,
    pub active_start: f32,
    pub active_end: f32,
}

/// The item / model tables Player's item system reads (`z_player_lib.c`, `z_player.c`,
/// `player.h`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ItemTables {
    /// `PLAYER_MODELGROUP_*` names (no prefix) and each group's `PLAYER_ANIMTYPE`
    /// (`gPlayerModelTypes[group][PLAYER_MODELGROUPENTRY_ANIM]`).
    pub model_group_names: Vec<String>,
    pub model_group_anim_type: Vec<usize>,
    /// `PLAYER_AP_*` names (no prefix), and `sActionModelGroups` (action → model group).
    pub ap_names: Vec<String>,
    pub action_model_group: Vec<usize>,
    /// `sItemChangeInfo`: item change animations and the frame the item swaps on.
    pub change_anims: Vec<(AnimId, f32)>,
    /// `sItemChangeTypes[from anim type][to anim type]`: ± index into `change_anims` (negative plays it backwards).
    pub change_matrix: Vec<Vec<i32>>,
    /// `D_80854190` by `PLAYER_MWA_*`, and `D_80854480` (stick direction → attack).
    pub attacks: Vec<AttackAnim>,
    pub attack_by_dir: Vec<usize>,
    pub mwa_names: Vec<String>,
    /// `sUpperBodyLimbCopyMap`: joints copied from the upper-body animation (`skelAnime2`).
    pub upper_body: [u8; 22],
    /// `sItemActions` (`z_player.c`): each item's action param, by `ItemID`.
    pub item_action_params: Vec<i32>,
}

impl ItemTables {
    pub fn model_group(&self, name: &str) -> usize {
        self.model_group_names.iter().position(|n| n == name).unwrap_or_else(|| panic!("no model group {name}"))
    }
    pub fn ap(&self, name: &str) -> i32 {
        self.ap_names.iter().position(|n| n == name).unwrap_or_else(|| panic!("no action param {name}")) as i32
    }
    /// `Player_ItemToItemAction`: `PLAYER_IA_NONE` for `ITEM_NONE_FE` and up,
    /// `PLAYER_IA_SWORD_CS` for `ITEM_SWORD_CS`, `PLAYER_IA_FISHING_POLE` for the fishing rod.
    pub fn item_to_action_param(&self, item: u8) -> i32 {
        /// `ITEM_SWORD_CS`, `ITEM_NONE_FE`, `ITEM_FISHING_POLE` (item.h).
        const ITEM_SWORD_CS: u8 = 0xFC;
        const ITEM_NONE_FE: u8 = 0xFE;
        const ITEM_FISHING_POLE: u8 = 0x59;
        if item >= ITEM_NONE_FE {
            self.ap("NONE")
        } else if item == ITEM_SWORD_CS {
            self.ap("SWORD_CS")
        } else if item == ITEM_FISHING_POLE {
            self.ap("FISHING_POLE")
        } else {
            self.item_action_params.get(item as usize).copied().unwrap_or(0)
        }
    }
    pub fn mwa(&self, name: &str) -> usize {
        self.mwa_names.iter().position(|n| n == name).unwrap_or_else(|| panic!("no attack {name}"))
    }
}

/// One of Link's animations, frames decoded to joint tables (22 Vec3s + face).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Anim {
    pub name: String,
    pub frames: Vec<eng_anim::anim::JointTable>,
}

impl Anim {
    /// `Animation_GetLength`.
    pub fn length(&self) -> f32 {
        self.frames.len() as f32
    }
    /// `Animation_GetLastFrame`.
    pub fn last_frame(&self) -> f32 {
        (self.frames.len() as f32 - 1.0).max(0.0)
    }
}

/// The asset pack stores this without `anims` (each animation is its own record, named by
/// `anim_symbols`); `set_anims` puts them back.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameData {
    pub regs: [Regs; 2],
    pub ages: [AgeProperties; 2],
    #[serde(skip)]
    pub anims: Vec<Anim>,
    /// `anims` by short name (`set_anims` fills it).
    #[serde(skip)]
    pub by_name: HashMap<String, AnimId>,
    /// The `gameplay_keep` symbol of each of `anims` (`gPlayerAnim_*`), in `AnimId` order.
    pub anim_symbols: Vec<String>,
    /// `D_80853914[group * PLAYER_ANIMTYPE_MAX + type]`.
    pub anim_table: Vec<AnimId>,
    pub anim_types: usize,
    pub anim_group_names: Vec<String>,
    /// `sFidgetAnimations`: idle fidget animations, [normal, alternate] per room behaviour/variant.
    pub idle_variants: Vec<[AnimId; 2]>,
    /// `PLAYER_LIMB_*` without prefix; index i is limb i (NONE at 0).
    pub limb_names: Vec<String>,
    pub tables_from_decomp: bool,
    pub table_mismatches: (usize, usize, i64),
    /// `z_camera_data.inc.c`: OREG values and the NORMAL0 camera data.
    pub camera: crate::camera::CameraData,
    /// `func_8008F87C`'s constants (`z_player_lib.c`).
    pub foot_ik: crate::footik::FootIkData,
    /// Link's limb hierarchy, [adult, child], from the ROM skeletons.
    pub rigs: [crate::footik::Rig; 2],
    /// `D_80853D4C`: side hop / backflip animations per stick direction [jump, landing, landing locked on].
    pub side_hop_anims: Vec<[AnimId; 3]>,
    /// `sAttentionRanges` (`z_actor.c`): per `targetMode`, (rangeSq, leashScale) = (SQ(range), range / leash).
    pub target_ranges: Vec<(f32, f32)>,
    pub items: ItemTables,
    /// Player's cutscene modes: `D_80854B18` (each `csMode`'s start) and `D_80854E50` (its
    /// update), in mode order.
    pub cs_mode_starts: Vec<CsModeEntry>,
    pub cs_mode_updates: Vec<CsModeEntry>,
}

/// A `struct_80854B18` (`z_player.c`): a cutscene mode's start or update. `ty` > 0 is a handler
/// of `D_80854AA4` with `anim` (or, for 18, the sound table `name`); `ty` < 0 calls the function
/// `name`; 0 does nothing.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CsModeEntry {
    pub ty: i8,
    pub anim: Option<AnimId>,
    /// The function (`func_808515A4`), the sound table (`D_80854AF0`), or the animation's short
    /// name; empty for NULL.
    pub name: String,
}

impl GameData {
    /// Installs Link's animations (in `anim_symbols` order) and indexes them by short name.
    pub fn set_anims(&mut self, anims: Vec<Anim>) {
        self.by_name = anims.iter().enumerate().map(|(i, a)| (a.name.clone(), i)).collect();
        self.anims = anims;
    }

    /// `GET_PLAYER_ANIM(group, type)`.
    pub fn player_anim(&self, group: usize, anim_type: usize) -> AnimId {
        self.anim_table[group * self.anim_types + anim_type]
    }

    /// A named animation (`gPlayerAnim_` prefix optional).
    pub fn anim(&self, name: &str) -> AnimId {
        let n = name.strip_prefix("gPlayerAnim_").unwrap_or(name);
        *self.by_name.get(n).unwrap_or_else(|| panic!("no Player animation {n}"))
    }

    pub fn anim_name(&self, id: AnimId) -> &str {
        &self.anims[id].name
    }

    /// `PLAYER_LIMB_<name>` as a skeleton limb index (the enum starts with NONE).
    pub fn limb(&self, name: &str) -> usize {
        self.limb_names.iter().position(|n| n == name).map(|i| i - 1).unwrap_or_else(|| panic!("no limb {name}"))
    }
}
