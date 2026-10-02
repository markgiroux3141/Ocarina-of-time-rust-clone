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
use oot_game::camera::{CamModeData, CamSettingData, CameraData, OnePointCsFull, OnePointData};
use oot_game::data::{AgeProperties, Anim, AnimId, AttackAnim, BOOTS_KOKIRI, BOOTS_KOKIRI_CHILD, CsModeEntry, GameData, ItemTables, Regs};
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

        let mut anims = Vec::new();
        let mut anim_symbols = Vec::new();
        let mut by_name = HashMap::new();
        for (name, a) in crate::player::player_animations(p)? {
            let short = name.strip_prefix("gPlayerAnim_").unwrap_or(&name).to_string();
            by_name.insert(short.clone(), anims.len());
            anims.push(player_anim(&short, &a));
            anim_symbols.push(name);
        }

        let ages_init = find_initializer(&player, "sAgeProperties")?;
        let anim_ref = |i: &Init| -> Result<usize> {
            let n = i.atom().context("sAgeProperties animation")?.trim().trim_start_matches('&').trim_start_matches("gPlayerAnim_");
            by_name.get(n).copied().with_context(|| format!("sAgeProperties: animation {n} not in gameplay_keep"))
        };
        let vec3s = |i: &Init| -> Result<[i16; 3]> {
            let v: Vec<i16> = i.list().iter().map(|x| x.as_int().map(|v| v as i16).context("sAgeProperties Vec3s")).collect::<Result<_>>()?;
            v.try_into().map_err(|v: Vec<i16>| anyhow::anyhow!("sAgeProperties Vec3s of {}", v.len()))
        };
        let vec3s_n = |i: &Init, n: usize| -> Result<Vec<[i16; 3]>> {
            let v: Vec<[i16; 3]> = i.list().iter().map(&vec3s).collect::<Result<_>>()?;
            anyhow::ensure!(v.len() == n, "sAgeProperties: {} Vec3s where {n} were expected", v.len());
            Ok(v)
        };
        let anims_n = |i: &Init, n: usize| -> Result<Vec<usize>> {
            let v: Vec<usize> = i.list().iter().map(&anim_ref).collect::<Result<_>>()?;
            anyhow::ensure!(v.len() == n, "sAgeProperties: {} animations where {n} were expected", v.len());
            Ok(v)
        };
        let ages: Vec<AgeProperties> = ages_init
            .list()
            .iter()
            .map(|a| {
                let items = a.list();
                let f: Vec<f32> = items.iter().take(17).map(|i| i.as_f32().context("sAgeProperties float")).collect::<Result<_>>()?;
                if f.len() != 17 || items.len() != 33 {
                    bail!("sAgeProperties entry has {} leading floats and {} fields", f.len(), items.len());
                }
                let mut age = AgeProperties::from_floats(&f);
                let c = &mut age.climb;
                c.unk_44 = vec3s(&items[17])?;
                c.unk_4A = vec3s_n(&items[18], 4)?.try_into().unwrap();
                c.unk_62 = vec3s_n(&items[19], 4)?.try_into().unwrap();
                c.unk_7A = vec3s_n(&items[20], 2)?.try_into().unwrap();
                c.unk_86 = vec3s_n(&items[21], 2)?.try_into().unwrap();
                c.unk_92 = items[22].as_int().context("unk_92")? as u16;
                c.unk_94 = items[23].as_int().context("unk_94")? as u16;
                c.unk_98 = anim_ref(&items[24])?;
                c.unk_9C = anim_ref(&items[25])?;
                c.unk_A0 = anim_ref(&items[26])?;
                c.unk_A4 = anim_ref(&items[27])?;
                c.unk_A8 = anim_ref(&items[28])?;
                c.unk_AC = anims_n(&items[29], 4)?.try_into().unwrap();
                c.unk_BC = anims_n(&items[30], 2)?.try_into().unwrap();
                c.unk_C4 = anims_n(&items[31], 2)?.try_into().unwrap();
                c.unk_CC = anims_n(&items[32], 2)?.try_into().unwrap();
                Ok(age)
            })
            .collect::<Result<_>>()?;
        if ages.len() != 2 {
            bail!("sAgeProperties has {} entries", ages.len());
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
            // sItemActionParams: each item's action param (Player_ItemToActionParam).
            let item_action_params = find_initializer(&player, "sItemActionParams")?
                .flatten()
                .iter()
                .map(|n| {
                    let n = n.trim().trim_start_matches("PLAYER_AP_");
                    ap_names.iter().position(|m| m == n).map(|i| i as i32).with_context(|| format!("sItemActionParams: {n}"))
                })
                .collect::<Result<Vec<_>>>()?;
            ItemTables { model_group_names, model_group_anim_type, ap_names, action_model_group, change_anims, change_matrix, attacks, attack_by_dir, mwa_names, upper_body, item_action_params }
        };
        // D_80854B18 and D_80854E50: { type, ptr }, ptr NULL, &gPlayerAnim_*, a function or a
        // struct_80832924 table.
        let cs_modes = |name: &str| -> Result<Vec<CsModeEntry>> {
            find_initializer(&player, name)?
                .list()
                .iter()
                .map(|e| {
                    let a = e.flatten();
                    let [ty, ptr] = &a[..] else { bail!("{name}: {a:?}") };
                    let ty = Init::Atom(ty.clone()).as_int().with_context(|| format!("{name}: {ty}"))? as i8;
                    let ptr = ptr.trim();
                    Ok(if ptr == "NULL" {
                        CsModeEntry { ty, anim: None, name: String::new() }
                    } else if let Some(an) = ptr.strip_prefix("&gPlayerAnim_") {
                        let id = *by_name.get(an).with_context(|| format!("{name}: no animation {an}"))?;
                        CsModeEntry { ty, anim: Some(id), name: an.to_string() }
                    } else {
                        CsModeEntry { ty, anim: None, name: ptr.to_string() }
                    })
                })
                .collect()
        };
        let (cs_mode_starts, cs_mode_updates) = (cs_modes("D_80854B18")?, cs_modes("D_80854E50")?);
        anyhow::ensure!(cs_mode_starts.len() == cs_mode_updates.len(), "the cutscene mode tables differ in length");
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
            cs_mode_starts,
            cs_mode_updates,
        };
        gd.set_anims(anims);
        Ok(gd)
    }
}

/// A `CameraModeValue[]` initializer: the arguments of its `CAM_FUNCDATA_*(...)` call.
fn cam_mode_values(src: &str, data: &str) -> Result<Vec<i16>> {
    let call = find_initializer(src, data)?.flatten().join(",");
    let open = call.find('(').with_context(|| format!("{data} is not a CAM_FUNCDATA_* call"))?;
    let args = call[open + 1..].strip_suffix(')').with_context(|| format!("{data}: unterminated"))?;
    args.split(',')
        .map(|a| Init::Atom(a.trim().to_string()).as_int().map(|v| v as i16))
        .collect::<Option<Vec<i16>>>()
        .with_context(|| format!("{data}: non-integer argument"))
}

/// A `CameraMode[]` initializer (`sCamSet*Modes`): `CAM_SETTING_MODE_ENTRY(func, data)`
/// entries (one atom each) and `{ CAM_FUNC_NONE, 0, NULL }`.
fn cam_setting_modes(src: &str, name: &str) -> Result<Vec<Option<CamModeData>>> {
    let mut modes = Vec::new();
    for item in find_initializer(src, name)?.list() {
        match item {
            Init::List(l) => {
                let f = l.first().and_then(|a| a.atom()).unwrap_or_default().trim();
                anyhow::ensure!(f == "CAM_FUNC_NONE", "{name}: unexpected entry {l:?}");
                modes.push(None);
            }
            Init::Atom(a) => {
                let args = a.trim().strip_prefix("CAM_SETTING_MODE_ENTRY(").and_then(|r| r.strip_suffix(')')).with_context(|| format!("{name}: {a}"))?;
                let (func, data) = args.split_once(',').with_context(|| format!("{name}: {a}"))?;
                let (func, data) = (func.trim().to_string(), data.trim().to_string());
                let values = cam_mode_values(src, &data)?;
                modes.push(Some(CamModeData { func, data, values }));
            }
        }
    }
    Ok(modes)
}

impl LoadCameraData for CameraData {
    /// `sOREGInit`, and `sCameraSettings` with every setting's `sCamSet*Modes` (each mode's
    /// function and `CAM_FUNCDATA_*` values), from `z_camera_data.c`; the setting names from
    /// `z64camera.h`'s `CAM_SET_*` enum.
    fn load(decomp: &Path) -> Result<CameraData> {
        let p = decomp.join("src/code/z_camera_data.c");
        let src = strip_comments(&std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?);
        let oreg = find_initializer(&src, "sOREGInit")?
            .list()
            .iter()
            .map(|i| i.as_int().map(|v| v as i16))
            .collect::<Option<Vec<_>>>()
            .context("sOREGInit: non-integer entry")?;
        let h = decomp.join("include/z64camera.h");
        let header = std::fs::read_to_string(&h).with_context(|| format!("reading {}", h.display()))?;
        let names = crate::csrc::parse_enum(&header, "CAM_SET_NONE");
        // sCameraSettings[] = { { { unk_00 } }, sCamSet*Modes or NULL }, one per CAM_SET_*.
        let mut settings = Vec::new();
        for (i, entry) in find_initializer(&src, "sCameraSettings")?.list().iter().enumerate() {
            let atoms = entry.flatten();
            let [flags, modes] = &atoms[..] else { bail!("sCameraSettings[{i}]: {atoms:?}") };
            let flags = Init::Atom(flags.clone()).as_int().with_context(|| format!("sCameraSettings[{i}]: {flags}"))? as u32;
            let modes = if modes.trim() == "NULL" { Vec::new() } else { cam_setting_modes(&src, modes.trim())? };
            let name = names.get(&(i as i64)).cloned().with_context(|| format!("no CAM_SET_* {i}"))?;
            settings.push(CamSettingData { name, flags, modes });
        }
        anyhow::ensure!(names.get(&(settings.len() as i64)).map(String::as_str) == Some("CAM_SET_MAX"), "sCameraSettings has {} entries, not CAM_SET_MAX", settings.len());
        let onepoint = load_onepoint_data(decomp, &src)?;
        Ok(CameraData { oreg, settings, onepoint })
    }
}

/// `Camera_Demo5`'s keyframe tables in `z_camera_data.c` (`D_8011D6AC`...).
const DEMO5_KEYFRAMES: [&str; 8] = ["D_8011D6AC", "D_8011D724", "D_8011D79C", "D_8011D83C", "D_8011D88C", "D_8011D8DC", "D_8011D954", "D_8011D9F4"];

/// An `OnePointCsFull` initializer: `{ actionFlags, unk_01, initFlags, timerInit,
/// rollTargetInit, fovTargetInit, lerpStepScale, { atTargetInit }, { eyeTargetInit } }`.
fn onepoint_cs_full(name: &str, i: &Init) -> Result<OnePointCsFull> {
    let a = i.flatten();
    anyhow::ensure!(a.len() == 13, "{name}: an OnePointCsFull with {} values", a.len());
    let int = |k: usize| Init::Atom(a[k].clone()).as_int().with_context(|| format!("{name}: {}", a[k]));
    let flt = |k: usize| Init::Atom(a[k].clone()).as_f32().with_context(|| format!("{name}: {}", a[k]));
    Ok(OnePointCsFull {
        action_flags: int(0)? as u8,
        unk_01: int(1)? as u8,
        init_flags: int(2)? as i16,
        timer_init: int(3)? as i16,
        roll_target_init: int(4)? as i16,
        fov_target_init: flt(5)?,
        lerp_step_scale: flt(6)?,
        at_target_init: Vec3::new(flt(7)?, flt(8)?, flt(9)?),
        eye_target_init: Vec3::new(flt(10)?, flt(11)?, flt(12)?),
    })
}

/// An `OnePointCsFull[]` or `[][]` initializer, rows flattened in order.
fn onepoint_keyframes(src: &str, name: &str) -> Result<Vec<OnePointCsFull>> {
    let init = find_initializer(src, name)?;
    let mut out = Vec::new();
    for item in init.list() {
        // A row of a two-dimensional array is a list of lists.
        if item.list().first().is_some_and(|f| matches!(f, Init::List(_))) && item.list().len() > 0 && item.flatten().len() % 13 == 0 && item.list().iter().all(|r| matches!(r, Init::List(_)) && r.flatten().len() == 13) {
            for row in item.list() {
                out.push(onepoint_cs_full(name, row)?);
            }
        } else {
            out.push(onepoint_cs_full(name, item)?);
        }
    }
    Ok(out)
}

/// A `CutsceneCameraPoint[]` initializer: `{ continueFlag, cameraRoll, nextPointFrame,
/// viewAngle, { pos } }`, with `CS_CMD_CONTINUE` 0 and `CS_CMD_STOP` -1 (`z64cutscene.h`).
fn cutscene_camera_points(src: &str, name: &str) -> Result<Vec<oot_game::cutscene::CutsceneCameraPoint>> {
    find_initializer(src, name)?
        .list()
        .iter()
        .map(|p| {
            let a = p.flatten();
            anyhow::ensure!(a.len() == 7, "{name}: a CutsceneCameraPoint with {} values", a.len());
            let flag = match a[0].trim() {
                "CS_CMD_CONTINUE" => 0,
                "CS_CMD_STOP" => -1,
                f => Init::Atom(f.to_string()).as_int().with_context(|| format!("{name}: {f}"))? as i8,
            };
            let int = |k: usize| Init::Atom(a[k].clone()).as_int().with_context(|| format!("{name}: {}", a[k]));
            Ok(oot_game::cutscene::CutsceneCameraPoint {
                continue_flag: flag,
                camera_roll: int(1)? as i8,
                next_point_frame: int(2)? as u16,
                view_angle: Init::Atom(a[3].clone()).as_f32().with_context(|| format!("{name}: {}", a[3]))?,
                pos: [int(4)? as i16, int(5)? as i16, int(6)? as i16],
            })
        })
        .collect()
}

/// Every table of `z_onepointdemo_data.c` (its `static OnePointCsFull`, `CutsceneCameraPoint`
/// and `s16` definitions, by their declarations) and `Camera_Demo5`'s of `z_camera_data.c`.
fn load_onepoint_data(decomp: &Path, camera_data_src: &str) -> Result<OnePointData> {
    let p = decomp.join("src/code/z_onepointdemo_data.c");
    let src = strip_comments(&std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?);
    let mut out = OnePointData::default();
    for line in src.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("static ") else { continue };
        let mut words = rest.split_whitespace();
        let (Some(ty), Some(name)) = (words.next(), words.next()) else { continue };
        let name = name.split(['[', '=', ';']).next().unwrap_or(name).to_string();
        match ty {
            "OnePointCsFull" => out.keyframes.push((name.clone(), onepoint_keyframes(&src, &name)?)),
            "CutsceneCameraPoint" => out.points.push((name.clone(), cutscene_camera_points(&src, &name)?)),
            "s16" => {
                let v = rest.split_once('=').and_then(|(_, v)| v.trim().strip_suffix(';')).with_context(|| format!("{name}: {line}"))?;
                let v = Init::Atom(v.trim().to_string()).as_int().with_context(|| format!("{name}: {v}"))?;
                out.shorts.push((name, v as i16));
            }
            _ => bail!("z_onepointdemo_data.c: unexpected definition {line}"),
        }
    }
    for name in DEMO5_KEYFRAMES {
        out.keyframes.push((name.to_string(), onepoint_keyframes(camera_data_src, name)?));
    }
    Ok(out)
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

/// `sItemDropIds` and `sDropQuantities` from `src/code/z_en_item00.c`, the drop ids through
/// `Item00Type` (`include/z64actor.h`).
pub fn load_item_drops(decomp: &Path) -> Result<oot_game::item::ItemDropTables> {
    let src = read(decomp, "src/code/z_en_item00.c")?;
    let header = read(decomp, "include/z64actor.h")?;
    let names: HashMap<String, i64> = crate::csrc::parse_enum(&header, "ITEM00_RUPEE_GREEN").into_iter().map(|(v, n)| (n, v)).collect();
    let ids = find_initializer(&src, "sItemDropIds")?
        .flatten()
        .iter()
        .map(|n| names.get(n.trim()).map(|&v| v as u8).with_context(|| format!("sItemDropIds: {n} isn't an Item00Type")))
        .collect::<Result<Vec<u8>>>()?;
    let quantities = find_initializer(&src, "sDropQuantities")?
        .flatten()
        .iter()
        .map(|n| crate::csrc::parse_int(n).map(|v| v as u8).with_context(|| format!("sDropQuantities: {n}")))
        .collect::<Result<Vec<u8>>>()?;
    // 15 tables of 16 ids; sDropQuantities has 4 more (unread) entries.
    anyhow::ensure!(ids.len() % 16 == 0 && quantities.len() >= ids.len(), "the drop tables have {} ids and {} quantities", ids.len(), quantities.len());
    Ok(oot_game::item::ItemDropTables { ids, quantities })
}

/// `sGetItemTable` (`z_player.c`, its `GET_ITEM(itemId, objectId, drawId, textId, field,
/// chestAnim)` rows as the macro packs them: `gi = (chestAnim != CHEST_ANIM_SHORT ? 1 : -1) *
/// (drawId + 1)`) and `sDrawItemTable` (`z_draw.c`: each draw id's `GetItem_Draw*` function
/// and display lists), the names through `z64item.h`'s enums and `object_table.h`.
pub fn load_items(decomp: &Path) -> Result<oot_game::item::ItemTables> {
    let player = read(decomp, "src/overlays/actors/ovl_player_actor/z_player.c")?;
    let draw = read(decomp, "src/code/z_draw.c")?;
    let header = read(decomp, "include/z64item.h")?;
    let by_name = |e: HashMap<i64, String>| -> HashMap<String, i64> { e.into_iter().map(|(v, n)| (n, v)).collect() };
    let items = by_name(crate::csrc::parse_enum(&header, "ITEM_STICK"));
    let gids = by_name(crate::csrc::parse_enum(&header, "GID_BOTTLE"));
    let objects: HashMap<String, i64> = crate::csrc::define_rows(&read(decomp, "include/tables/object_table.h")?)
        .into_iter()
        .enumerate()
        .filter_map(|(i, (mac, a))| {
            let name = if mac == "DEFINE_OBJECT" { a.get(1) } else { a.first() };
            name.map(|n| (n.clone(), i as i64))
        })
        .collect();
    let mut get_items = Vec::new();
    for atom in find_initializer(&player, "sGetItemTable")?.flatten() {
        let atom = atom.trim();
        if atom == "GET_ITEM_NONE" {
            // { ITEM_NONE, 0, 0, 0, OBJECT_INVALID }.
            get_items.push(oot_game::item::GetItemEntry { item_id: 0xFF, field: 0, gi: 0, text_id: 0, object_id: 0 });
            continue;
        }
        let args = atom.strip_prefix("GET_ITEM(").and_then(|r| r.strip_suffix(')')).with_context(|| format!("sGetItemTable: {atom}"))?;
        let a: Vec<&str> = args.split(',').map(str::trim).collect();
        anyhow::ensure!(a.len() == 6, "sGetItemTable: {atom}");
        let look = |m: &HashMap<String, i64>, n: &str| m.get(n).copied().with_context(|| format!("sGetItemTable: unknown {n}"));
        let int = |n: &str| crate::csrc::parse_int(n).with_context(|| format!("sGetItemTable: {n}"));
        let draw_id = look(&gids, a[2])?;
        let gi = if a[5] != "CHEST_ANIM_SHORT" { 1 } else { -1 } * (draw_id + 1);
        get_items.push(oot_game::item::GetItemEntry {
            item_id: look(&items, a[0])? as u8,
            object_id: look(&objects, a[1])? as i16,
            gi: gi as i8,
            text_id: int(a[3])? as u8,
            field: int(a[4])? as u8,
        });
    }
    let mut draw_items = Vec::new();
    for row in find_initializer(&draw, "sDrawItemTable")?.list() {
        let l = row.list();
        anyhow::ensure!(l.len() == 2, "sDrawItemTable: {row:?}");
        let func = l[0].atom().context("sDrawItemTable: no draw function")?.trim().to_string();
        let dlists = l[1].flatten().iter().map(|s| s.trim().to_string()).collect();
        draw_items.push(oot_game::item::DrawItemEntry { func, dlists });
    }
    // One draw function per GID_* (up to GID_MAXIMUM).
    anyhow::ensure!(draw_items.len() == gids.len() - gids.contains_key("GID_MAX") as usize, "sDrawItemTable has {} entries for {} draw ids", draw_items.len(), gids.len());
    Ok(oot_game::item::ItemTables { get_items, draw_items })
}

/// `sRestrictionFlags` (`z_parameter.c`): `{ sceneId, flags1, flags2, flags3 }`, the scene ids
/// from `scene_table.h`'s order, ending with `0xFF`.
pub fn load_interface(decomp: &Path, scenes: &crate::room::SceneTables) -> Result<oot_game::interface::InterfaceTables> {
    let src = read(decomp, "src/code/z_parameter.c")?;
    let ids: HashMap<&str, usize> = scenes.scenes.iter().map(|s| (s.enum_name.as_str(), s.id)).collect();
    let mut restrictions = Vec::new();
    for entry in find_initializer(&src, "sRestrictionFlags")?.list() {
        let f = entry.flatten();
        anyhow::ensure!(f.len() == 4, "sRestrictionFlags: {f:?}");
        let name = f[0].trim();
        let id = match ids.get(name) {
            Some(&id) => id as u8,
            None => crate::csrc::parse_int(name).map(|v| v as u8).with_context(|| format!("sRestrictionFlags: {name} isn't a scene"))?,
        };
        let flag = |i: usize| crate::csrc::parse_int(&f[i]).map(|v| v as u8).with_context(|| format!("sRestrictionFlags: {}", f[i]));
        restrictions.push([id, flag(1)?, flag(2)?, flag(3)?]);
    }
    anyhow::ensure!(restrictions.last().is_some_and(|r| r[0] == 0xFF), "sRestrictionFlags doesn't end with 0xFF");
    Ok(oot_game::interface::InterfaceTables { restrictions })
}
