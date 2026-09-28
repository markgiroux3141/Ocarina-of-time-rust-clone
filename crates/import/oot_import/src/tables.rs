//! The game tables read from the decomp's C and the ROM, for the asset pack: the maths
//! tables, Player's constants and animations (`GameData`), the camera data, foot IK's
//! constants, and the environment's light configs. The record types live in the game
//! (`eng_math`, `oot_game`); these are their loaders, as extension traits (`GameData::load(p)`
//! with `LoadGameData` in scope). The asset pack stores what they return, and the tests compare
//! the pack's records with them.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use eng_anim::anim::{Animation, LinkAnimation};
use eng_anim::skeleton::{LimbType, Skeleton};
use eng_math::Tables;
use glam::Vec3;
use oot_game::actor_table::{ACTOROVL_ALLOC_ABSOLUTE, ACTOROVL_ALLOC_NORMAL, ACTOROVL_ALLOC_PERSISTENT, ActorInfo, ActorInitInfo, ActorTable};
use oot_game::camera::CameraData;
use oot_game::data::{AgeProperties, Anim, AnimId, AttackAnim, BOOTS_KOKIRI, BOOTS_KOKIRI_CHILD, GameData, ItemTables, Regs};
use oot_game::env::{EnvTables, TimeBasedLightEntry, clock_time};
use oot_game::footik::{FootIkData, Rig};
use oot_game::player_lib::Age;

use crate::csrc::{Init, enum_members, find_initializer, strip_comments};
use crate::project::Project;
use crate::z64::ParseSkeleton;

pub trait LoadGameData: Sized {
    fn load(p: &Project) -> Result<Self>;
}
pub trait LoadCameraData: Sized {
    fn load(decomp: &Path) -> Result<Self>;
}
pub trait LoadFootIkData: Sized {
    fn load(decomp: &Path) -> Result<Self>;
}
pub trait LoadEnvTables: Sized {
    fn load(decomp: &Path) -> Result<Self>;
}

/// One of Link's animations as `GameData` holds it: every frame sampled to a joint table.
pub fn player_anim(short_name: &str, a: &LinkAnimation) -> Anim {
    Anim { name: short_name.to_string(), frames: (0..a.frame_count()).map(|f| a.sample(f)).collect() }
}

/// Loads `eng_math::Tables` from the decomp: `Tables::load(decomp)` with this trait in scope.
pub trait LoadMathTables: Sized {
    fn load(decomp: &Path) -> Result<Self>;
}

impl LoadMathTables for Tables {
    /// `sintable[0x400]` from `src/libultra/gu/sintable.c` and `sATan2Tbl` from
    /// `src/code/sys_math_atan.c`.
    fn load(decomp: &Path) -> Result<Tables> {
        let read = |rel: &str| -> Result<String> {
            let p = decomp.join(rel);
            Ok(strip_comments(&std::fs::read_to_string(&p).with_context(|| p.display().to_string())?))
        };
        let ints = |src: &str, name: &str| -> Result<Vec<i64>> {
            find_initializer(src, name)?
                .flatten()
                .iter()
                .map(|s| Init::Atom(s.clone()).as_int().with_context(|| format!("{name}: {s}")))
                .collect()
        };
        let sin: Vec<i16> = ints(&read("src/libultra/gu/sintable.c")?, "sintable")?.into_iter().map(|v| v as i16).collect();
        let atan: Vec<u16> = ints(&read("src/code/sys_math_atan.c")?, "sATan2Tbl")?.into_iter().map(|v| v as u16).collect();
        if sin.len() != 0x400 || atan.len() != 0x401 {
            bail!("unexpected table sizes: sintable {} sATan2Tbl {}", sin.len(), atan.len());
        }
        Ok(Tables { sin, atan, from_decomp: true })
    }
}

/// Link's skeleton for `age` (`gLinkAdultSkel` / `gLinkChildSkel`): parents and joint positions.
fn load_rig(p: &Project, age: Age) -> Result<Rig> {
    let symbols = p.symbols.file(age.object()).with_context(|| format!("{}.xml", age.object()))?;
    let skel = symbols.of_kind("Skeleton").next().context("no skeleton in Link object")?;
    let object = p.rom.file_by_name(age.object())?;
    let s = Skeleton::parse(&object, 6, skel.offset as usize, LimbType::Lod, true)?;
    Ok(Rig { parents: s.parents.clone(), joint_pos: s.limbs.iter().map(|l| l.joint_pos).collect() })
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
    let mut values = std::collections::BTreeMap::new();
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

impl LoadGameData for GameData {
    fn load(p: &Project) -> Result<GameData> {
        let decomp = &p.config.decomp;
        let tables = Tables::load(decomp)?;
        let mismatches = tables.compare(&Tables::computed());
        eng_math::install(tables);

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
        let mut anim_symbols = Vec::new();
        let mut by_name = HashMap::new();
        for (name, a) in crate::player::player_animations(p)? {
            let short = name.strip_prefix("gPlayerAnim_").unwrap_or(&name).to_string();
            by_name.insert(short.clone(), anims.len());
            anims.push(player_anim(&short, &a));
            anim_symbols.push(name);
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
                let r = crate::csrc::eval_expr(r).context("range")?;
                let l = crate::csrc::eval_expr(l).context("leash")?;
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
                let e = crate::csrc::parse_enum(&header, "PLAYER_MWA_FORWARD_SLASH_1H");
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

        let mut gd = GameData {
            regs,
            ages: [ages[0], ages[1]],
            anims: Vec::new(),
            by_name: HashMap::new(),
            anim_symbols,
            anim_table,
            anim_types,
            anim_group_names,
            idle_variants,
            limb_names,
            tables_from_decomp: true,
            table_mismatches: mismatches,
            camera: CameraData::load(decomp)?,
            foot_ik: FootIkData::load(decomp)?,
            rigs: [load_rig(p, Age::Adult)?, load_rig(p, Age::Child)?],
            side_hop_anims,
            target_ranges,
            items,
        };
        gd.set_anims(anims);
        Ok(gd)
    }
}

impl LoadCameraData for CameraData {
    /// `sOREGInit` and `sSetNormal0ModeNormalData` from `z_camera_data.c`.
    fn load(decomp: &Path) -> Result<CameraData> {
        let p = decomp.join("src/code/z_camera_data.c");
        let src = strip_comments(&std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?);
        let oreg = find_initializer(&src, "sOREGInit")?
            .list()
            .iter()
            .map(|i| i.as_int().map(|v| v as i16))
            .collect::<Option<Vec<_>>>()
            .context("sOREGInit: non-integer entry")?;
        let norm = find_initializer(&src, "sSetNormal0ModeNormalData")?;
        let call = norm.flatten().join(",");
        let args = call
            .strip_prefix("CAM_FUNCDATA_NORM1(")
            .and_then(|s| s.strip_suffix(')'))
            .context("sSetNormal0ModeNormalData is not a CAM_FUNCDATA_NORM1")?;
        let v: Vec<i16> = args
            .split(',')
            .map(|a| {
                let a = a.trim();
                match a.strip_prefix("0x") {
                    Some(h) => i16::from_str_radix(h, 16).ok(),
                    None => a.parse().ok(),
                }
            })
            .collect::<Option<_>>()
            .context("CAM_FUNCDATA_NORM1 arguments")?;
        let normal0 = v.try_into().map_err(|_| anyhow::anyhow!("CAM_FUNCDATA_NORM1 wants 10 arguments"))?;
        Ok(CameraData { oreg, normal0 })
    }
}

fn atom_f32(i: &Init) -> Option<f32> {
    let a = i.atom()?.trim();
    // SQ(x) = ((x) * (x)) (macros.h).
    if let Some(inner) = a.strip_prefix("SQ(").and_then(|r| r.strip_suffix(')')) {
        let v = crate::csrc::eval_expr(inner)?;
        return Some(v * v);
    }
    i.as_f32()
}

impl LoadFootIkData for FootIkData {
    /// `D_80126038`..`D_80126070` from `z_player_lib.c`.
    fn load(decomp: &Path) -> Result<FootIkData> {
        let p = decomp.join("src/code/z_player_lib.c");
        let src = strip_comments(&std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?);
        let pair = |name: &str| -> Result<[f32; 2]> {
            let v: Vec<f32> = find_initializer(&src, name)?.list().iter().map(atom_f32).collect::<Option<_>>().with_context(|| name.to_string())?;
            v.try_into().map_err(|_| anyhow::anyhow!("{name}: want 2 values"))
        };
        let vec3 = |i: &Init| -> Option<Vec3> {
            let v: Vec<f32> = i.list().iter().map(atom_f32).collect::<Option<_>>()?;
            Some(Vec3::new(*v.first()?, *v.get(1)?, *v.get(2)?))
        };
        let shin = find_initializer(&src, "D_80126038")?;
        let shin_offset = [vec3(&shin.list()[0]).context("D_80126038[0]")?, vec3(&shin.list()[1]).context("D_80126038[1]")?];
        let footprint = vec3(&find_initializer(&src, "D_80126070")?).context("D_80126070")?;
        Ok(FootIkData {
            shin_offset,
            foot_x: pair("D_80126050")?,
            thigh_len_sq: pair("D_80126058")?,
            len_diff: pair("D_80126060")?,
            floor_offset: pair("D_80126068")?,
            footprint,
        })
    }
}

/// Expands `CLOCK_TIME(h, m)` in a constant expression and evaluates it.
fn eval_time(expr: &str) -> Option<i32> {
    let mut s = expr.to_string();
    while let Some(at) = s.find("CLOCK_TIME(") {
        let close = at + s[at..].find(')')?;
        let args: Vec<i32> = s[at + 11..close].split(',').map(|a| a.trim().parse().ok()).collect::<Option<_>>()?;
        s.replace_range(at..=close, &clock_time(args[0], args[1]).to_string());
    }
    crate::csrc::eval_expr(&s).map(|v| v as i32)
}

impl LoadEnvTables for EnvTables {
    fn load(decomp: &Path) -> Result<EnvTables> {
        let p = decomp.join("src/code/z_kankyo.c");
        let src = strip_comments(&std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?);
        let init = find_initializer(&src, "sTimeBasedLightConfigs")?;
        let entry = |e: &Init| -> Option<TimeBasedLightEntry> {
            let f = e.list();
            Some(TimeBasedLightEntry {
                start: eval_time(f.first()?.atom()?)? as u16,
                end: eval_time(f.get(1)?.atom()?)? as u16,
                light_setting: f.get(2)?.as_int()? as u8,
                next_light_setting: f.get(3)?.as_int()? as u8,
            })
        };
        let time_based = init
            .list()
            .iter()
            .map(|cfg| cfg.list().iter().map(entry).collect::<Option<Vec<_>>>())
            .collect::<Option<Vec<_>>>()
            .context("sTimeBasedLightConfigs: unexpected entry")?;
        Ok(EnvTables { time_based })
    }
}

/// Loads `oot_game::actor_table::ActorTable` from the decomp: `ActorTable::load(decomp)`.
pub trait LoadActorTable: Sized {
    fn load(decomp: &Path) -> Result<Self>;
}

/// `FLAGS` as the overlays define it: `0`, `ACTOR_FLAG_n`, or `(ACTOR_FLAG_a | ACTOR_FLAG_b ...)`
/// (`ACTOR_FLAG_n` is `(1 << n)` in `z64actor.h`).
fn eval_actor_flags(expr: &str) -> Option<u32> {
    let e = expr.trim().trim_start_matches('(').trim_end_matches(')');
    let mut v = 0u32;
    for part in e.split('|') {
        let part = part.trim();
        if let Some(n) = part.strip_prefix("ACTOR_FLAG_") {
            v |= 1 << n.parse::<u32>().ok()?;
        } else {
            v |= crate::csrc::parse_int(part)? as u32;
        }
    }
    Some(v)
}

impl LoadActorTable for ActorTable {
    /// `include/tables/actor_table.h`, and every `ActorInit <Name>_InitVars` in `src/` (the
    /// overlays and the three actors in `code`) with its file's `#define FLAGS`.
    fn load(decomp: &Path) -> Result<ActorTable> {
        let read = |p: &Path| std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()));
        let z64actor = read(&decomp.join("include/z64actor.h"))?;
        let cats: HashMap<String, i64> = crate::csrc::parse_enum(&z64actor, "ACTORCAT_SWITCH").into_iter().map(|(v, n)| (n, v)).collect();
        let objects: HashMap<String, i16> = crate::csrc::define_rows(&read(&decomp.join("include/tables/object_table.h"))?)
            .into_iter()
            .enumerate()
            .filter_map(|(i, (_, a))| a.last().map(|e| (e.clone(), i as i16)))
            .collect();
        let table = crate::csrc::define_rows(&read(&decomp.join("include/tables/actor_table.h"))?);
        let ids: HashMap<String, i16> = table.iter().enumerate().filter_map(|(i, (mac, a))| {
            let e = if mac == "DEFINE_ACTOR_UNSET" { a.first() } else { a.get(1) };
            e.map(|e| (e.clone(), i as i16))
        }).collect();
        // ActorInit definitions, by their symbol (`<Name>_InitVars`).
        let mut inits: HashMap<String, ActorInitInfo> = HashMap::new();
        let mut stack = vec![decomp.join("src")];
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
                let path = e?.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|x| x != "c") {
                    continue;
                }
                let text = read(&path)?;
                if !text.contains("ActorInit ") {
                    continue;
                }
                let src = strip_comments(&text);
                let flags_define = src.lines().find_map(|l| l.trim().strip_prefix("#define FLAGS ").map(|r| r.trim().to_string()));
                for line in src.lines() {
                    let Some(rest) = line.trim().strip_prefix("const ActorInit ").or_else(|| line.trim().strip_prefix("ActorInit ")) else { continue };
                    let Some(name) = rest.split_whitespace().next() else { continue };
                    let init = find_initializer(&src, name)?;
                    let f: Vec<String> = init.list().iter().map(|i| i.atom().unwrap_or("").trim().to_string()).collect();
                    if f.len() < 9 {
                        bail!("{}: {name} has {} fields", path.display(), f.len());
                    }
                    let flags_expr = if f[2] == "FLAGS" { flags_define.clone().with_context(|| format!("{}: no #define FLAGS", path.display()))? } else { f[2].clone() };
                    let func = |s: &str| {
                        let s = s.trim().trim_start_matches("(ActorFunc)").trim();
                        (s != "NULL" && !s.is_empty()).then(|| s.to_string())
                    };
                    inits.insert(
                        name.to_string(),
                        ActorInitInfo {
                            id: ids.get(&f[0]).copied().or_else(|| crate::csrc::parse_int(&f[0]).map(|v| v as i16)).with_context(|| format!("{name}: id {}", f[0]))?,
                            category: *cats.get(&f[1]).with_context(|| format!("{name}: category {}", f[1]))? as u8,
                            flags: eval_actor_flags(&flags_expr).with_context(|| format!("{name}: flags {flags_expr}"))?,
                            object_id: *objects.get(&f[3]).with_context(|| format!("{name}: object {}", f[3]))?,
                            init: func(&f[5]),
                            destroy: func(&f[6]),
                            update: func(&f[7]),
                            draw: func(&f[8]),
                        },
                    );
                }
            }
        }
        let mut actors = Vec::new();
        for (mac, a) in table {
            let alloc = |s: &str| match s {
                "ACTOROVL_ALLOC_ABSOLUTE" => ACTOROVL_ALLOC_ABSOLUTE,
                "ACTOROVL_ALLOC_PERSISTENT" => ACTOROVL_ALLOC_PERSISTENT,
                _ => ACTOROVL_ALLOC_NORMAL,
            };
            actors.push(match mac.as_str() {
                "DEFINE_ACTOR" | "DEFINE_ACTOR_INTERNAL" => ActorInfo {
                    enum_name: a[1].clone(),
                    name: a[0].clone(),
                    alloc: alloc(&a[2]),
                    internal: mac == "DEFINE_ACTOR_INTERNAL",
                    init: inits.remove(&format!("{}_InitVars", a[0])),
                },
                _ => ActorInfo { enum_name: a.first().cloned().unwrap_or_default(), name: String::new(), alloc: 0, internal: false, init: None },
            });
        }
        Ok(ActorTable { actors })
    }
}
