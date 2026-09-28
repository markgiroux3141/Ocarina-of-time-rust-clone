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

/// `PlayerAgeProperties` float fields `unk_00` .. `unk_40` (the struct has no names yet in
/// this decomp; the uses noted are from `z_player.c`).
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AgeProperties {
    /// unk_00: ceiling check height in `func_80847BA0`.
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
    /// unk_38: wall check radius.
    pub wall_radius: f32,
    pub unk_3C: f32,
    pub unk_40: f32,
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
/// `z64player.h`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ItemTables {
    /// `PLAYER_MODELGROUP_*` names (no prefix) and each group's `PLAYER_ANIMTYPE`
    /// (`gPlayerModelTypes[group][PLAYER_MODELGROUPENTRY_ANIM]`).
    pub model_group_names: Vec<String>,
    pub model_group_anim_type: Vec<usize>,
    /// `PLAYER_AP_*` names (no prefix), and `sActionModelGroups` (action → model group).
    pub ap_names: Vec<String>,
    pub action_model_group: Vec<usize>,
    /// `D_808540F4`: item change animations and the frame the item swaps on.
    pub change_anims: Vec<(AnimId, f32)>,
    /// `D_80854164[from anim type][to anim type]`: ± index into `change_anims` (negative plays it backwards).
    pub change_matrix: Vec<Vec<i32>>,
    /// `D_80854190` by `PLAYER_MWA_*`, and `D_80854480` (stick direction → attack).
    pub attacks: Vec<AttackAnim>,
    pub attack_by_dir: Vec<usize>,
    pub mwa_names: Vec<String>,
    /// `D_80853410`: joints copied from the upper-body animation (`skelAnime2`).
    pub upper_body: [u8; 22],
}

impl ItemTables {
    pub fn model_group(&self, name: &str) -> usize {
        self.model_group_names.iter().position(|n| n == name).unwrap_or_else(|| panic!("no model group {name}"))
    }
    pub fn ap(&self, name: &str) -> i32 {
        self.ap_names.iter().position(|n| n == name).unwrap_or_else(|| panic!("no action param {name}")) as i32
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
    /// `D_80853D7C`: idle fidget animations, [normal, alternate] per room behaviour/variant.
    pub idle_variants: Vec<[AnimId; 2]>,
    /// `PLAYER_LIMB_*` without prefix; index i is limb i (NONE at 0).
    pub limb_names: Vec<String>,
    pub tables_from_decomp: bool,
    pub table_mismatches: (usize, usize, i64),
    /// `z_camera_data.c`: OREG values and the NORMAL0 camera data.
    pub camera: crate::camera::CameraData,
    /// `func_8008F87C`'s constants (`z_player_lib.c`).
    pub foot_ik: crate::footik::FootIkData,
    /// Link's limb hierarchy, [adult, child], from the ROM skeletons.
    pub rigs: [crate::footik::Rig; 2],
    /// `D_80853D4C`: side hop / backflip animations per stick direction [jump, landing, landing locked on].
    pub side_hop_anims: Vec<[AnimId; 3]>,
    /// `D_80115FF8` (`z_actor.c`): per `targetMode`, (rangeSq, leashScale) = (SQ(range), range / leash).
    pub target_ranges: Vec<(f32, f32)>,
    pub items: ItemTables,
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
