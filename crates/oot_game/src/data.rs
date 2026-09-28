//! Player constants and animations, read at runtime from the decomp source and the ROM.
//!
//! - REG values: `sBootData` and the assignments in `Player_SetBootData` (`z_player_lib.c`).
//!   The function body is scanned for `XREG(n) = bootRegs[k];` and `XREG(n) = literal;`, so
//!   the mapping itself comes from the source.
//! - `sAgeProperties` (`z_player.c`): the 17 leading float fields.
//! - `D_80853914` (`GET_PLAYER_ANIM(group, type)`, `z_player.c`): animation group table.
//! - Link's animations: `gameplay_keep` headers into `link_animetion` (see `oot_core::anim`).

#![allow(non_snake_case)] // fields keep the decomp's unk_XXX names

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use oot_core::anim::{Animation, LinkAnimation};
use oot_core::csrc::{Init, enum_members, find_initializer, strip_comments};
use oot_core::project::Project;

use crate::math::Tables;

/// `PLAYER_BOOTS_*` rows of `sBootData`.
pub const BOOTS_KOKIRI: usize = 0;
pub const BOOTS_KOKIRI_CHILD: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Age {
    Adult = 0,
    Child = 1,
}

/// The `REG`-group debug registers Player reads, as set by `Player_SetBootData`.
#[derive(Debug, Clone, Default)]
pub struct Regs {
    /// e.g. `"REG(19)"` → 200.
    pub values: HashMap<String, i16>,
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
#[derive(Debug, Clone, Copy, Default)]
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
    fn from_floats(f: &[f32]) -> AgeProperties {
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
#[derive(Debug, Clone, Copy)]
pub struct AttackAnim {
    pub anim: AnimId,
    pub end: AnimId,
    pub end_locked: AnimId,
    pub active_start: f32,
    pub active_end: f32,
}

/// The item / model tables Player's item system reads (`z_player_lib.c`, `z_player.c`,
/// `z64player.h`).
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct Anim {
    pub name: String,
    pub frames: Vec<oot_core::anim::JointTable>,
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

#[derive(Debug, Clone)]
pub struct GameData {
    pub regs: [Regs; 2],
    pub ages: [AgeProperties; 2],
    pub anims: Vec<Anim>,
    by_name: HashMap<String, AnimId>,
    /// `D_80853914[group * PLAYER_ANIMTYPE_MAX + type]`.
    anim_table: Vec<AnimId>,
    anim_types: usize,
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

/// Link's skeleton for `age` (`gLinkAdultSkel` / `gLinkChildSkel`): parents and joint positions.
fn load_rig(p: &Project, age: oot_core::player::Age) -> Result<crate::footik::Rig> {
    let symbols = p.symbols.file(age.object()).with_context(|| format!("{}.xml", age.object()))?;
    let skel = symbols.of_kind("Skeleton").next().context("no skeleton in Link object")?;
    let object = p.rom.file_by_name(age.object())?;
    let s = oot_core::skeleton::Skeleton::parse(&object, 6, skel.offset as usize, oot_core::skeleton::LimbType::Lod, true)?;
    Ok(crate::footik::Rig { parents: s.parents.clone(), joint_pos: s.limbs.iter().map(|l| l.joint_pos).collect() })
}

fn read(decomp: &Path, rel: &str) -> Result<String> {
    let p = decomp.join(rel);
    Ok(strip_comments(&std::fs::read_to_string(&p).with_context(|| p.display().to_string())?))
}

/// Extracts the body of `name(...) { ... }` from comment-stripped source.
fn function_body<'a>(src: &'a str, name: &str) -> Result<&'a str> {
    let sig = format!("{name}(");
    let mut from = 0;
    while let Some(rel) = src[from..].find(&sig) {
        let at = from + rel;
        from = at + sig.len();
        let rest = &src[at..];
        let Some(open) = rest.find('{') else { break };
        // A definition has `)` then `{` with nothing but whitespace between.
        let close_paren = rest[..open].rfind(')').unwrap_or(0);
        if !rest[close_paren + 1..open].trim().is_empty() {
            continue;
        }
        let mut depth = 0;
        for (i, c) in rest[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(&rest[open + 1..open + i]);
                    }
                }
                _ => {}
            }
        }
    }
    bail!("no definition of {name}")
}

/// Reads `XREG(n) = <int or bootRegs[k]>;` assignments (first occurrence wins) from
/// `Player_SetBootData` and resolves them against a `sBootData` row.
fn boot_regs(lib: &str, boot_data: &[Vec<i16>], row: usize) -> Result<Regs> {
    let body = function_body(lib, "Player_SetBootData")?;
    let mut values = HashMap::new();
    for stmt in body.split(';') {
        let s = stmt.trim();
        let Some((lhs, rhs)) = s.split_once('=') else { continue };
        let (lhs, rhs) = (lhs.trim(), rhs.trim());
        if !lhs.ends_with(')') || !lhs.contains("REG(") || lhs.contains(' ') {
            continue;
        }
        let v = if let Some(k) = rhs.strip_prefix("bootRegs[").and_then(|r| r.strip_suffix(']')) {
            let k: usize = k.parse().context("bootRegs index")?;
            boot_data[row][k]
        } else if let Some(v) = Init::Atom(rhs.to_string()).as_int() {
            v as i16
        } else {
            continue;
        };
        values.entry(lhs.to_string()).or_insert(v);
    }
    Ok(Regs { values })
}

impl GameData {
    pub fn load(p: &Project) -> Result<GameData> {
        let decomp = &p.config.decomp;
        let tables = Tables::load(decomp)?;
        let mismatches = tables.compare(&Tables::computed());
        crate::math::install(tables);

        let lib = read(decomp, "src/code/z_player_lib.c")?;
        let player = read(decomp, "src/overlays/actors/ovl_player_actor/z_player.c")?;
        let header = std::fs::read_to_string(decomp.join("include/z64player.h")).context("z64player.h")?;

        let boot_data: Vec<Vec<i16>> = find_initializer(&lib, "sBootData")?
            .list()
            .iter()
            .map(|row| row.flatten().iter().map(|s| Init::Atom(s.clone()).as_int().map(|v| v as i16).context("sBootData")).collect())
            .collect::<Result<_>>()?;
        let regs = [boot_regs(&lib, &boot_data, BOOTS_KOKIRI)?, boot_regs(&lib, &boot_data, BOOTS_KOKIRI_CHILD)?];

        let ages_init = find_initializer(&player, "sAgeProperties")?;
        let ages: Vec<AgeProperties> = ages_init
            .list()
            .iter()
            .map(|a| {
                let f: Vec<f32> = a.list().iter().take(17).map(|i| i.as_f32().context("sAgeProperties float")).collect::<Result<_>>()?;
                if f.len() != 17 {
                    bail!("sAgeProperties entry has {} leading floats", f.len());
                }
                Ok(AgeProperties::from_floats(&f))
            })
            .collect::<Result<_>>()?;
        if ages.len() != 2 {
            bail!("sAgeProperties has {} entries", ages.len());
        }

        let mut anims = Vec::new();
        let mut by_name = HashMap::new();
        for (name, a) in oot_core::player::player_animations(p)? {
            let short = name.strip_prefix("gPlayerAnim_").unwrap_or(&name).to_string();
            by_name.insert(short.clone(), anims.len());
            anims.push(Anim { name: short, frames: (0..a.frame_count()).map(|f| a.sample(f)).collect() });
            let _: &LinkAnimation = &a;
        }

        let anim_types = enum_members(&header, "PLAYER_ANIMTYPE_").iter().filter(|m| !m.ends_with("_MAX")).count();
        let anim_group_names: Vec<String> = enum_members(&header, "PLAYER_ANIMGROUP_").into_iter().filter(|m| !m.ends_with("_MAX")).collect();
        let entries = find_initializer(&player, "D_80853914")?.flatten();
        if anim_types == 0 || entries.len() != anim_types * anim_group_names.len() {
            bail!("D_80853914 has {} entries for {} groups × {} types", entries.len(), anim_group_names.len(), anim_types);
        }
        let anim_table = entries
            .iter()
            .map(|e| {
                let n = e.trim_start_matches('&').trim_start_matches("gPlayerAnim_");
                by_name.get(n).copied().with_context(|| format!("animation {n} from D_80853914 not in gameplay_keep"))
            })
            .collect::<Result<_>>()?;
        let resolve = |e: &str| -> Result<AnimId> {
            let n = e.trim_start_matches('&').trim_start_matches("gPlayerAnim_");
            by_name.get(n).copied().with_context(|| format!("animation {n} not in gameplay_keep"))
        };
        let idle_variants = find_initializer(&player, "D_80853D7C")?
            .list()
            .iter()
            .map(|pair| {
                let f = pair.flatten();
                Ok([resolve(&f[0])?, resolve(&f[1])?])
            })
            .collect::<Result<_>>()?;
        let side_hop_anims = find_initializer(&player, "D_80853D4C")?
            .list()
            .iter()
            .map(|row| {
                let f = row.flatten();
                let a = |k: usize| resolve(f.get(k).map(|s| s.trim_start_matches('&')).unwrap_or(""));
                Ok([a(0)?, a(1)?, a(2)?])
            })
            .collect::<Result<_>>()?;
        let actor_c = read(decomp, "src/code/z_actor.c")?;
        let target_ranges = find_initializer(&actor_c, "D_80115FF8")?
            .flatten()
            .iter()
            .map(|a| {
                // TARGET_RANGE(range, leash) = { SQ(range), (f32)range / leash }.
                let args = a.trim().strip_prefix("TARGET_RANGE(").and_then(|r| r.strip_suffix(')')).with_context(|| format!("D_80115FF8 entry {a}"))?;
                let (r, l) = args.split_once(',').context("TARGET_RANGE args")?;
                let r = oot_core::csrc::eval_expr(r).context("range")?;
                let l = oot_core::csrc::eval_expr(l).context("leash")?;
                Ok((r * r, r / l))
            })
            .collect::<Result<_>>()?;
        let items = {
            let strip_max = |v: Vec<String>, p: &str| -> Vec<String> {
                v.into_iter().filter(|m| !m.ends_with("_MAX")).map(|m| m[p.len()..].to_string()).collect()
            };
            let model_group_names = strip_max(enum_members(&header, "PLAYER_MODELGROUP_"), "PLAYER_MODELGROUP_");
            let ap_names = strip_max(enum_members(&header, "PLAYER_AP_"), "PLAYER_AP_");
            // The PLAYER_MWA_* enum is commented with decimal indices, so read it by value.
            let mwa_names = {
                let e = oot_core::csrc::parse_enum(&header, "PLAYER_MWA_FORWARD_SLASH_1H");
                let mut v: Vec<(i64, String)> = e.into_iter().filter(|(_, n)| !n.ends_with("_MAX")).collect();
                v.sort();
                v.into_iter().map(|(_, n)| n["PLAYER_MWA_".len()..].to_string()).collect::<Vec<_>>()
            };
            let num_suffix = |s: &str| -> Result<i32> {
                let t = s.trim();
                let (neg, t) = match t.strip_prefix('-') {
                    Some(r) => (true, r.trim()),
                    None => (false, t),
                };
                let n: i32 = t.rsplit('_').next().and_then(|d| d.parse().ok()).with_context(|| format!("no index in {s}"))?;
                Ok(if neg { -n } else { n })
            };
            let model_group_anim_type = find_initializer(&lib, "gPlayerModelTypes")?
                .list()
                .iter()
                .map(|row| num_suffix(&row.flatten()[0]).map(|v| v as usize))
                .collect::<Result<Vec<_>>>()?;
            let action_model_group = find_initializer(&lib, "sActionModelGroups")?
                .flatten()
                .iter()
                .map(|n| {
                    let n = n.trim().trim_start_matches("PLAYER_MODELGROUP_");
                    model_group_names.iter().position(|m| m == n).with_context(|| format!("model group {n}"))
                })
                .collect::<Result<Vec<_>>>()?;
            let change_anims = find_initializer(&player, "D_808540F4")?
                .list()
                .iter()
                .map(|row| {
                    let f = row.flatten();
                    Ok((resolve(&f[0])?, f[1].trim().parse::<f32>().context("D_808540F4 frame")?))
                })
                .collect::<Result<Vec<_>>>()?;
            let change_matrix = find_initializer(&player, "D_80854164")?
                .list()
                .iter()
                .map(|row| row.flatten().iter().map(|a| num_suffix(a)).collect::<Result<Vec<_>>>())
                .collect::<Result<Vec<_>>>()?;
            let attacks = find_initializer(&player, "D_80854190")?
                .list()
                .iter()
                .map(|row| {
                    let f = row.flatten();
                    let n = |k: usize| f[k].trim().parse::<f32>().context("D_80854190 frame");
                    Ok(AttackAnim { anim: resolve(&f[0])?, end: resolve(&f[1])?, end_locked: resolve(&f[2])?, active_start: n(3)?, active_end: n(4)? })
                })
                .collect::<Result<Vec<_>>>()?;
            let attack_by_dir = find_initializer(&player, "D_80854480")?
                .flatten()
                .iter()
                .map(|n| {
                    let n = n.trim().trim_start_matches("PLAYER_MWA_");
                    mwa_names.iter().position(|m| m == n).with_context(|| format!("attack {n}"))
                })
                .collect::<Result<Vec<_>>>()?;
            let mask: Vec<u8> = find_initializer(&player, "D_80853410")?.flatten().iter().map(|a| a.trim().parse().unwrap_or(0)).collect();
            let mut upper_body = [0u8; 22];
            for (i, v) in mask.iter().take(22).enumerate() {
                upper_body[i] = *v;
            }
            ItemTables { model_group_names, model_group_anim_type, ap_names, action_model_group, change_anims, change_matrix, attacks, attack_by_dir, mwa_names, upper_body }
        };
        let limb_names = enum_members(&header, "PLAYER_LIMB_").into_iter().map(|m| m["PLAYER_LIMB_".len()..].to_string()).collect();

        Ok(GameData {
            regs,
            ages: [ages[0], ages[1]],
            anims,
            by_name,
            anim_table,
            anim_types,
            anim_group_names,
            idle_variants,
            limb_names,
            tables_from_decomp: true,
            table_mismatches: mismatches,
            camera: crate::camera::CameraData::load(decomp)?,
            foot_ik: crate::footik::FootIkData::load(decomp)?,
            rigs: [load_rig(p, oot_core::player::Age::Adult)?, load_rig(p, oot_core::player::Age::Child)?],
            side_hop_anims,
            target_ranges,
            items,
        })
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
