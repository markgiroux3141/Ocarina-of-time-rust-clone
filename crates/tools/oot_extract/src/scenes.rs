//! Scenes and rooms: every header command of every layer decoded to JSON, room meshes plus
//! collision in one .glb per scene, and prerendered backgrounds as .jpg.
//!
//! Output per scene, under `<out>/scenes/<xml category>/<scene>/`:
//! - `<scene>.glb`: a node per room (OPA / XLU children), `collision`, `water_boxes` and
//!   `actors` (empties, per layer). 1 glTF unit = 1 game unit, +Y up (same as OoT).
//! - `scene.json`: scene and room headers for all layers, the draw config bindings and the
//!   mesh statistics.
//! - `collision.json`: vertices, polys, surface types, bg cameras and water boxes.
//! - `backgrounds/*.jpg`: JFIF data of prerendered rooms (room shape type 1).
//!
//! Segments 8..0xD are bound the way the scene's draw-config function in
//! `src/code/z_scene_table.c` binds them. That function is read from the decomp and run by
//! a small C interpreter (`drawcfg`) with the state of a fresh child-day load at frame 0.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use eng_gbi::gbi::DrawList;
use glam::{EulerRot, Quat};
use oot_import::csrc::{define_rows, parse_defines, parse_enum, strip_comments};
use oot_import::drawcfg;
use oot_import::project::Project;
use oot_import::room::{Keeps, SceneCtx, code_ram_image, run_dls};
use oot_import::symbols::Symbol;
use serde_json::{Value, json};

use crate::gltf::Gltf;

const CMD_NAMES: [&str; 0x1A] = [
    "SPAWN_LIST",
    "ACTOR_LIST",
    "UNUSED_02",
    "COLLISION_HEADER",
    "ROOM_LIST",
    "WIND_SETTINGS",
    "ENTRANCE_LIST",
    "SPECIAL_FILES",
    "ROOM_BEHAVIOR",
    "UNDEFINED_09",
    "ROOM_SHAPE",
    "OBJECT_LIST",
    "LIGHT_LIST",
    "PATH_LIST",
    "TRANSITION_ACTOR_LIST",
    "LIGHT_SETTINGS_LIST",
    "TIME_SETTINGS",
    "SKYBOX_SETTINGS",
    "SKYBOX_DISABLES",
    "EXIT_LIST",
    "END",
    "SOUND_SETTINGS",
    "ECHO_SETTINGS",
    "CUTSCENE_DATA",
    "ALTERNATE_HEADER_LIST",
    "MISC_SETTINGS",
];

const UNITS: &str = "1 glTF unit = 1 OoT world unit (no scaling). Axes are OoT's: +Y up, right-handed, which matches glTF. \
Room vertices are stored in world space. Angles in the game data are binary angles (0x10000 = 360 degrees); *_degrees fields are converted.";

// ---------------------------------------------------------------------------------------------
// Byte helpers
// ---------------------------------------------------------------------------------------------

fn u8_at(d: &[u8], o: usize) -> Option<u8> {
    d.get(o).copied()
}
fn s8_at(d: &[u8], o: usize) -> Option<i8> {
    d.get(o).map(|&b| b as i8)
}
fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*d.get(o)?, *d.get(o + 1)?]))
}
fn s16_at(d: &[u8], o: usize) -> Option<i16> {
    u16_at(d, o).map(|v| v as i16)
}
fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes([*d.get(o)?, *d.get(o + 1)?, *d.get(o + 2)?, *d.get(o + 3)?]))
}
fn vec3s(d: &[u8], o: usize) -> Option<[i16; 3]> {
    Some([s16_at(d, o)?, s16_at(d, o + 2)?, s16_at(d, o + 4)?])
}
fn rgb(d: &[u8], o: usize) -> Option<[u8; 3]> {
    Some([u8_at(d, o)?, u8_at(d, o + 1)?, u8_at(d, o + 2)?])
}
fn hex32(v: u32) -> String {
    format!("0x{v:08X}")
}
fn hex16(v: u16) -> String {
    format!("0x{v:04X}")
}
fn binang_deg(v: i16) -> f64 {
    (v as f64 * 360.0 / 65536.0 * 1000.0).round() / 1000.0
}
fn binang_rad(v: i16) -> f32 {
    v as f32 * std::f32::consts::TAU / 65536.0
}

fn layer_name(i: usize) -> String {
    match i {
        0 => "child_day".into(),
        1 => "child_night".into(),
        2 => "adult_day".into(),
        3 => "adult_night".into(),
        n => format!("cutscene_{}", n - 4),
    }
}

// ---------------------------------------------------------------------------------------------
// Decomp tables (names)
// ---------------------------------------------------------------------------------------------

struct SceneDef {
    id: usize,
    file: String,
    title: String,
    enum_name: String,
    draw_config: String,
    unk_10: String,
    unk_12: String,
}

struct Tables {
    /// Actor id -> (enum, overlay name).
    actors: Vec<(String, String)>,
    /// Object id -> (enum, file name or "").
    objects: Vec<(String, String)>,
    /// Entrance index -> (enum, scene enum, spawn index).
    entrances: Vec<(String, String, String)>,
    scenes: Vec<SceneDef>,
    scene_ids: HashMap<String, i64>,
    /// SDC enum name -> draw config function name.
    sdc_funcs: HashMap<String, String>,
    seqs: HashMap<i64, String>,
    nature: HashMap<i64, String>,
    cam_settings: HashMap<i64, String>,
    skyboxes: HashMap<i64, String>,
    scene_cam_types: HashMap<i64, String>,
    light_types: HashMap<i64, String>,
    sfx_types: Vec<String>,
    wall_flags: Vec<u32>,
    drawcfg: drawcfg::Program,
}

fn read_text(p: &Path) -> Result<String> {
    std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))
}

impl Tables {
    fn load(decomp: &Path) -> Result<Tables> {
        let inc = decomp.join("include");
        let tables = inc.join("tables");
        let mut actors = Vec::new();
        for (mac, a) in define_rows(&read_text(&tables.join("actor_table.h"))?) {
            actors.push(match mac.as_str() {
                "DEFINE_ACTOR_UNSET" => (a.first().cloned().unwrap_or_default(), String::new()),
                _ => (a.get(1).cloned().unwrap_or_default(), a.first().cloned().unwrap_or_default()),
            });
        }
        let mut objects = Vec::new();
        for (mac, a) in define_rows(&read_text(&tables.join("object_table.h"))?) {
            objects.push(match mac.as_str() {
                "DEFINE_OBJECT_UNSET" => (a.first().cloned().unwrap_or_default(), String::new()),
                _ => (a.get(1).cloned().unwrap_or_default(), a.first().cloned().unwrap_or_default()),
            });
        }
        let entrances = define_rows(&read_text(&tables.join("entrance_table.h"))?)
            .into_iter()
            .map(|(_, a)| {
                (a.first().cloned().unwrap_or_default(), a.get(1).cloned().unwrap_or_default(), a.get(2).cloned().unwrap_or_default())
            })
            .collect();
        let mut scenes = Vec::new();
        let mut scene_ids = HashMap::new();
        for (i, (_, a)) in define_rows(&read_text(&tables.join("scene_table.h"))?).into_iter().enumerate() {
            let g = |k: usize| a.get(k).cloned().unwrap_or_default();
            scene_ids.insert(g(2), i as i64);
            scenes.push(SceneDef { id: i, file: g(0), title: g(1), enum_name: g(2), draw_config: g(3), unk_10: g(4), unk_12: g(5) });
        }
        let z64scene = read_text(&inc.join("z64scene.h"))?;
        let sdc_enum = parse_enum(&z64scene, "SDC_DEFAULT");
        let scene_table_src = read_text(&decomp.join("src/code/z_scene_table.c"))?;
        let drawcfg = drawcfg::Program::parse(&scene_table_src);
        let mut sdc_funcs = HashMap::new();
        if let Some(list) = drawcfg.arrays.get("sSceneDrawConfigs") {
            for (i, f) in list.names.iter().enumerate() {
                if let Some(sdc) = sdc_enum.get(&(i as i64)) {
                    sdc_funcs.insert(sdc.clone(), f.clone());
                }
            }
        }
        let seq_h = read_text(&inc.join("sequence.h")).unwrap_or_default();
        let z64 = read_text(&inc.join("z64.h")).unwrap_or_default();
        let cam = read_text(&inc.join("z64camera.h")).unwrap_or_default();
        let light = read_text(&inc.join("z64light.h")).unwrap_or_default();
        let bgcheck_c = read_text(&decomp.join("src/code/z_bgcheck.c")).unwrap_or_default();
        Ok(Tables {
            actors,
            objects,
            entrances,
            scenes,
            scene_ids,
            sdc_funcs,
            seqs: parse_defines(&seq_h, "NA_BGM_"),
            nature: parse_enum(&seq_h, "NATURE_ID_GENERAL_NIGHT"),
            cam_settings: parse_enum(&cam, "CAM_SET_NONE"),
            skyboxes: parse_enum(&z64, "SKYBOX_NONE"),
            scene_cam_types: parse_defines(&z64scene, "SCENE_CAM_TYPE_"),
            light_types: parse_enum(&light, "LIGHT_POINT_NOGLOW"),
            sfx_types: parse_c_array(&bgcheck_c, "D_80119E10"),
            wall_flags: parse_wall_flags(&bgcheck_c),
            drawcfg,
        })
    }

    fn actor(&self, id: u16) -> (String, String) {
        self.actors.get(id as usize).cloned().unwrap_or_else(|| (format!("ACTOR_0x{id:04X}"), String::new()))
    }
    fn object(&self, id: u16) -> (String, String) {
        self.objects.get(id as usize).cloned().unwrap_or_else(|| (format!("OBJECT_0x{id:04X}"), String::new()))
    }
    fn entrance(&self, idx: u16) -> Value {
        match self.entrances.get(idx as usize) {
            Some((e, s, sp)) => json!({ "entrance": e, "scene": s, "spawn": sp.parse::<i64>().ok() }),
            None => json!({ "entrance": match idx {
                0x7FF9 => "ENTR_RETURN_YOUSEI_IZUMI_YOKO",
                0x7FFA => "ENTR_RETURN_SYATEKIJYOU",
                0x7FFB => "ENTR_RETURN_2",
                0x7FFC => "ENTR_RETURN_SHOP1",
                0x7FFD => "ENTR_RETURN_4",
                0x7FFE => "ENTR_RETURN_DAIYOUSEI_IZUMI",
                0x7FFF => "ENTR_RETURN_GROTTO",
                _ => "unknown",
            } }),
        }
    }
}

/// Element names of a `u16 NAME[...] = { A - SFX_FLAG, ... };` table.
fn parse_c_array(src: &str, name: &str) -> Vec<String> {
    let Some(p) = src.find(&format!("{name}[")) else { return Vec::new() };
    let Some(o) = src[p..].find('{') else { return Vec::new() };
    let Some(c) = src[p + o..].find("};") else { return Vec::new() };
    strip_comments(&src[p + o + 1..p + o + c])
        .split(',')
        .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// `D_80119D90[WALL_TYPE_MAX]`: wall type -> WALL_FLAG_* bits (z_bgcheck.c).
fn parse_wall_flags(src: &str) -> Vec<u32> {
    let Some(p) = src.find("D_80119D90[") else { return Vec::new() };
    let Some(o) = src[p..].find('{') else { return Vec::new() };
    let Some(c) = src[p + o..].find("};") else { return Vec::new() };
    strip_comments(&src[p + o + 1..p + o + c])
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.split('|')
                .filter_map(|f| f.trim().strip_prefix("WALL_FLAG_").and_then(|n| n.parse::<u32>().ok()).map(|b| 1u32 << b))
                .fold(0, |a, b| a | b)
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Segment-addressed data
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Files<'a> {
    scene: &'a [u8],
    room: Option<&'a [u8]>,
}

impl<'a> Files<'a> {
    fn resolve(&self, addr: u32) -> Option<(&'a [u8], usize)> {
        let off = (addr & 0x00FF_FFFF) as usize;
        let d = match addr >> 24 {
            2 => self.scene,
            3 => self.room?,
            _ => return None,
        };
        (off < d.len()).then_some((d, off))
    }
}

/// Reads 8-byte commands until END (0x14), an invalid code, or 64 commands.
fn read_commands(d: &[u8], off: usize) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for i in 0..64 {
        let (Some(w0), Some(w1)) = (u32_at(d, off + i * 8), u32_at(d, off + i * 8 + 4)) else { break };
        out.push((w0, w1));
        let code = w0 >> 24;
        if code == 0x14 || code >= 0x1A {
            break;
        }
    }
    out
}

/// Header offsets per layer: index 0 is the main header (offset 0), then the alternate
/// header list (command 0x18). `None` = no header for that layer.
fn read_layers(d: &[u8], seg: u8, known: &BTreeSet<usize>) -> (Vec<Option<usize>>, Option<u32>) {
    let main = read_commands(d, 0);
    let mut layers = vec![Some(0)];
    let Some(&(_, list)) = main.iter().find(|(w0, _)| w0 >> 24 == 0x18) else { return (layers, None) };
    if list >> 24 != seg as u32 {
        return (layers, Some(list));
    }
    let base = (list & 0xFF_FFFF) as usize;
    let mut targets = BTreeSet::new();
    for i in 0..20 {
        let off = base + i * 4;
        if i > 0 && (known.contains(&off) || targets.contains(&off)) {
            break;
        }
        let Some(v) = u32_at(d, off) else { break };
        if v == 0 {
            layers.push(None);
            continue;
        }
        let t = (v & 0xFF_FFFF) as usize;
        if v >> 24 != seg as u32 || t + 8 > d.len() || d[t] >= 0x1A || t == 0 {
            break;
        }
        targets.insert(t);
        layers.push(Some(t));
    }
    while layers.len() > 1 && layers.last() == Some(&None) {
        layers.pop();
    }
    (layers, Some(list))
}

/// Pointer-carrying commands: w1 is a segment address.
fn cmd_has_pointer(code: u32) -> bool {
    matches!(code, 0x00 | 0x01 | 0x02 | 0x03 | 0x04 | 0x06 | 0x0A | 0x0B | 0x0C | 0x0D | 0x0E | 0x0F | 0x13 | 0x17 | 0x18)
}

// ---------------------------------------------------------------------------------------------
// Collision
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Poly {
    ty: u16,
    via: u16,
    vib: u16,
    vic: u16,
    normal: [i16; 3],
    dist: i16,
}

struct Collision {
    ptr: u32,
    verts: Vec<[i16; 3]>,
    polys: Vec<Poly>,
    surfaces: Vec<[u32; 2]>,
    water: Vec<Value>,
    water_raw: Vec<([i16; 5], u32)>,
    json: Value,
}

fn surface_decode(t: &Tables, s: [u32; 2]) -> Value {
    let (a, b) = (s[0], s[1]);
    let wall_type = (a >> 21) & 0x1F;
    let wall_flags = t.wall_flags.get(wall_type as usize).copied().unwrap_or(0);
    let floor_property = (a >> 26) & 0xF;
    let sfx = b & 0xF;
    let conveyor_dir = (b >> 21) & 0x3F;
    json!({
        "raw": [hex32(a), hex32(b)],
        "bg_cam_index": a & 0xFF,
        "exit_index": (a >> 8) & 0x1F,
        "exit_index_note": "1-based index into the scene exit list; 0 = none",
        "floor_type": (a >> 13) & 0x1F,
        "unk_18_bits": (a >> 18) & 7,
        "wall_type": wall_type,
        "wall_flags": wall_flags,
        "wall_type_hint": match wall_type {
            0 => "none", 1 => "no ledge grab", 2 => "ladder", 3 => "ladder top", 4 => "climbable (vines)",
            5 => "crawlspace 1", 6 => "crawlspace 2", 7 => "push block", _ => "other",
        },
        "floor_property": floor_property,
        "floor_property_hint": match floor_property {
            5 => "respawn (FUNC_80041EA4_RESPAWN)", 6 => "mount wall (FUNC_80041EA4_MOUNT_WALL)",
            8 => "stop (FUNC_80041EA4_STOP)", 12 => "void out (FUNC_80041EA4_VOID_OUT)", 0 => "none", _ => "other",
        },
        "is_soft": (a >> 30) & 1 == 1,
        "is_horse_blocked": (a >> 31) & 1 == 1,
        "sfx_type": sfx,
        "sfx": t.sfx_types.get(sfx as usize).cloned().unwrap_or_else(|| "NA_SE_PL_WALK_GROUND".into()),
        "floor_effect": (b >> 4) & 3,
        "light_setting": (b >> 6) & 0x1F,
        "echo": (b >> 11) & 0x3F,
        "can_hookshot": (b >> 17) & 1 == 1,
        "conveyor_speed": (b >> 18) & 7,
        "conveyor_direction": conveyor_dir,
        "conveyor_direction_degrees": conveyor_dir as f64 * 360.0 / 64.0,
        "unk_27": (b >> 27) & 1,
    })
}

fn parse_collision(t: &Tables, ptr: u32, known: &BTreeSet<usize>, extra_cams: usize, files: Files) -> Option<Collision> {
    let (d, o) = files.resolve(ptr)?;
    let min_b = vec3s(d, o)?;
    let max_b = vec3s(d, o + 6)?;
    let nv = u16_at(d, o + 0x0C)? as usize;
    let vtx_p = u32_at(d, o + 0x10)?;
    let np = u16_at(d, o + 0x14)? as usize;
    let poly_p = u32_at(d, o + 0x18)?;
    let surf_p = u32_at(d, o + 0x1C)?;
    let cam_p = u32_at(d, o + 0x20)?;
    let nw = u16_at(d, o + 0x24)? as usize;
    let water_p = u32_at(d, o + 0x28)?;
    let mut verts = Vec::with_capacity(nv);
    if let Some((vd, vo)) = files.resolve(vtx_p) {
        for i in 0..nv {
            match vec3s(vd, vo + i * 6) {
                Some(v) => verts.push(v),
                None => break,
            }
        }
    }
    let mut polys = Vec::with_capacity(np);
    if let Some((pd, po)) = files.resolve(poly_p) {
        for i in 0..np {
            let b = po + i * 16;
            let (Some(ty), Some(via), Some(vib), Some(vic), Some(normal), Some(dist)) =
                (u16_at(pd, b), u16_at(pd, b + 2), u16_at(pd, b + 4), u16_at(pd, b + 6), vec3s(pd, b + 8), s16_at(pd, b + 14))
            else {
                break;
            };
            polys.push(Poly { ty, via, vib, vic, normal, dist });
        }
    }
    let n_surf = polys.iter().map(|p| p.ty as usize + 1).max().unwrap_or(0);
    let mut surfaces = Vec::new();
    if let Some((sd, so)) = files.resolve(surf_p) {
        for i in 0..n_surf {
            let (Some(a), Some(b)) = (u32_at(sd, so + i * 8), u32_at(sd, so + i * 8 + 4)) else { break };
            surfaces.push([a, b]);
        }
    }
    let mut water = Vec::new();
    let mut water_raw = Vec::new();
    if let Some((wd, wo)) = files.resolve(water_p) {
        for i in 0..nw {
            let b = wo + i * 16;
            let (Some(x), Some(y), Some(z), Some(xl), Some(zl), Some(props)) =
                (s16_at(wd, b), s16_at(wd, b + 2), s16_at(wd, b + 4), s16_at(wd, b + 6), s16_at(wd, b + 8), u32_at(wd, b + 12))
            else {
                break;
            };
            water_raw.push(([x, y, z, xl, zl], props));
            water.push(json!({
                "index": i, "x_min": x, "y_surface": y, "z_min": z, "x_length": xl, "z_length": zl,
                "properties": hex32(props),
                "bg_cam_index": props & 0xFF,
                "light_index": (props >> 8) & 0x1F,
                "room": (props >> 13) & 0x3F,
                "room_note": "63 = all rooms",
                "flag_19": (props >> 19) & 1 == 1,
            }));
        }
    }
    // Bg cameras: the count isn't stored. Take at least every index referenced by surfaces,
    // water boxes, doors and room images, then extend while entries stay valid and don't run
    // into other known data.
    let mut needed = extra_cams;
    for s in &surfaces {
        needed = needed.max((s[0] & 0xFF) as usize + 1);
    }
    for (_, props) in &water_raw {
        let c = (props & 0xFF) as usize;
        if c != 0xFF {
            needed = needed.max(c + 1);
        }
    }
    let mut cams = Vec::new();
    if let Some((cd, co)) = files.resolve(cam_p) {
        let mut stop: BTreeSet<usize> = known.clone();
        for i in 0..256 {
            let b = co + i * 8;
            if i >= needed && i > 0 && stop.contains(&b) {
                break;
            }
            let (Some(setting), Some(count), Some(data_p)) = (u16_at(cd, b), s16_at(cd, b + 2), u32_at(cd, b + 4)) else { break };
            let valid = (setting as i64) < 0x42 && (0..=0x400).contains(&count) && (data_p == 0 || (data_p >> 24 == 2 && files.resolve(data_p).is_some()));
            if i >= needed && !valid {
                break;
            }
            if data_p >> 24 == 2 {
                stop.insert((data_p & 0xFF_FFFF) as usize);
            }
            let mut cam = json!({
                "index": i,
                "setting": setting,
                "setting_name": t.cam_settings.get(&(setting as i64)).cloned().unwrap_or_default(),
                "count": count,
                "data_ptr": hex32(data_p),
            });
            if let Some((dd, dof)) = files.resolve(data_p) {
                let pts: Vec<[i16; 3]> = (0..count.max(0) as usize).filter_map(|k| vec3s(dd, dof + k * 6)).collect();
                if t.cam_settings.get(&(setting as i64)).is_some_and(|n| n == "CAM_SET_CRAWLSPACE") || count != 3 {
                    cam["points"] = json!(pts);
                } else if pts.len() == 3 {
                    let fov = pts[2][0];
                    cam["pos"] = json!(pts[0]);
                    cam["rot_raw"] = json!(pts[1]);
                    cam["fov_raw"] = json!(fov);
                    cam["fov_degrees"] = json!(if fov > 360 { fov as f64 / 100.0 } else { fov as f64 });
                    cam["flags_or_room_image_override_bg_cam_index"] = json!(pts[2][1]);
                    cam["unk_10"] = json!(pts[2][2]);
                }
            }
            cams.push(cam);
        }
    }
    let poly_json: Vec<Value> = polys
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let n = p.normal.map(|v| v as f64 / 32767.0);
            json!({
                "index": i,
                "surface_type": p.ty,
                "vertices": [p.via & 0x1FFF, p.vib & 0x1FFF, p.vic],
                "flags_a": p.via >> 13,
                "flags_b": p.vib >> 13,
                "ignore_camera": p.via & 0x2000 != 0,
                "ignore_entities": p.via & 0x4000 != 0,
                "ignore_projectiles": p.via & 0x8000 != 0,
                "floor_conveyor": p.vib & 0x2000 != 0,
                "normal": [(n[0] * 1e5).round() / 1e5, (n[1] * 1e5).round() / 1e5, (n[2] * 1e5).round() / 1e5],
                "normal_raw": p.normal,
                "dist": p.dist,
            })
        })
        .collect();
    let surf_json: Vec<Value> = surfaces
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut v = surface_decode(t, *s);
            v["index"] = json!(i);
            v
        })
        .collect();
    let json = json!({
        "units": UNITS,
        "header": {
            "ptr": hex32(ptr),
            "min_bounds": min_b, "max_bounds": max_b,
            "num_vertices": nv, "vertices_ptr": hex32(vtx_p),
            "num_polys": np, "polys_ptr": hex32(poly_p),
            "surface_types_ptr": hex32(surf_p),
            "bg_cams_ptr": hex32(cam_p),
            "num_water_boxes": nw, "water_boxes_ptr": hex32(water_p),
        },
        "counts_note": "surface types = max poly type + 1; bg cams = every referenced index, extended while entries stay valid before the next known data",
        "surface_type_hints_note": "wall_flags come from D_80119D90 in z_bgcheck.c and floor_property hints from the FUNC_80041EA4_* defines; wall_type_hint names are community names, not decomp names",
        "vertices": verts,
        "polys": poly_json,
        "surface_types": surf_json,
        "bg_cams": cams,
        "water_boxes": water,
    });
    Some(Collision { ptr, verts, polys, surfaces, water, water_raw, json })
}

// ---------------------------------------------------------------------------------------------
// Room shapes
// ---------------------------------------------------------------------------------------------

struct Background {
    index: usize,
    bg_cam_index: Option<u8>,
    unk_00: Option<u16>,
    source: u32,
    unk_0c: u32,
    tlut: u32,
    width: u16,
    height: u16,
    fmt: u8,
    siz: u8,
    tlut_mode: u16,
    tlut_count: u16,
}

#[derive(Default)]
struct Shape {
    ty: u8,
    opa: Vec<u32>,
    xlu: Vec<u32>,
    backgrounds: Vec<Background>,
    json: Value,
}

fn read_bg(d: &[u8], o: usize, index: usize, multi: bool) -> Option<Background> {
    // Single: fields start at 0x08 of the shape. Multi entry: unk_00, bgCamIndex, then the same fields.
    let (b, bg_cam_index, unk_00) = if multi { (o + 4, Some(u8_at(d, o + 2)?), Some(u16_at(d, o)?)) } else { (o, None, None) };
    Some(Background {
        index,
        bg_cam_index,
        unk_00,
        source: u32_at(d, b)?,
        unk_0c: u32_at(d, b + 4)?,
        tlut: u32_at(d, b + 8)?,
        width: u16_at(d, b + 12)?,
        height: u16_at(d, b + 14)?,
        fmt: u8_at(d, b + 16)?,
        siz: u8_at(d, b + 17)?,
        tlut_mode: u16_at(d, b + 18)?,
        tlut_count: u16_at(d, b + 20)?,
    })
}

fn parse_shape(files: Files, ptr: u32) -> Option<Shape> {
    let (d, o) = files.resolve(ptr)?;
    let ty = u8_at(d, o)?;
    let mut s = Shape { ty, ..Default::default() };
    let dl = |v: u32| if v == 0 { Value::Null } else { json!(hex32(v)) };
    match ty {
        0 | 2 => {
            let n = u8_at(d, o + 1)? as usize;
            let start = u32_at(d, o + 4)?;
            let end = u32_at(d, o + 8)?;
            let mut entries = Vec::new();
            if let Some((ed, eo)) = files.resolve(start) {
                let stride = if ty == 0 { 8 } else { 16 };
                for i in 0..n {
                    let b = eo + i * stride;
                    if ty == 0 {
                        let (Some(opa), Some(xlu)) = (u32_at(ed, b), u32_at(ed, b + 4)) else { break };
                        s.opa.push(opa);
                        s.xlu.push(xlu);
                        entries.push(json!({ "opa": dl(opa), "xlu": dl(xlu) }));
                    } else {
                        let (Some(c), Some(r), Some(opa), Some(xlu)) = (vec3s(ed, b), s16_at(ed, b + 6), u32_at(ed, b + 8), u32_at(ed, b + 12)) else {
                            break;
                        };
                        s.opa.push(opa);
                        s.xlu.push(xlu);
                        entries.push(json!({ "bounds_sphere_center": c, "bounds_sphere_radius": r, "opa": dl(opa), "xlu": dl(xlu) }));
                    }
                }
            }
            s.json = json!({
                "ptr": hex32(ptr),
                "type": ty,
                "type_name": if ty == 0 { "ROOM_SHAPE_TYPE_NORMAL" } else { "ROOM_SHAPE_TYPE_CULLABLE" },
                "num_entries": n,
                "entries_ptr": hex32(start),
                "entries_end_ptr": hex32(end),
                "entries": entries,
            });
        }
        1 => {
            let amount = u8_at(d, o + 1)?;
            let entry = u32_at(d, o + 4)?;
            let (mut opa, mut xlu) = (0, 0);
            if let Some((ed, eo)) = files.resolve(entry) {
                opa = u32_at(ed, eo).unwrap_or(0);
                xlu = u32_at(ed, eo + 4).unwrap_or(0);
            }
            s.opa.push(opa);
            s.xlu.push(xlu);
            let mut extra = json!({});
            if amount == 1 {
                if let Some(bg) = read_bg(d, o + 8, 0, false) {
                    s.backgrounds.push(bg);
                }
            } else if amount == 2 {
                let n = u8_at(d, o + 8)? as usize;
                let list = u32_at(d, o + 12)?;
                extra = json!({ "num_backgrounds": n, "backgrounds_ptr": hex32(list) });
                if let Some((bd, bo)) = files.resolve(list) {
                    for i in 0..n {
                        if let Some(bg) = read_bg(bd, bo + i * 0x1C, i, true) {
                            s.backgrounds.push(bg);
                        }
                    }
                }
            }
            s.json = json!({
                "ptr": hex32(ptr),
                "type": 1,
                "type_name": "ROOM_SHAPE_TYPE_IMAGE",
                "amount_type": amount,
                "amount_type_name": match amount { 1 => "ROOM_SHAPE_IMAGE_AMOUNT_SINGLE", 2 => "ROOM_SHAPE_IMAGE_AMOUNT_MULTI", _ => "invalid" },
                "entry_ptr": hex32(entry),
                "opa": dl(opa),
                "xlu": dl(xlu),
                "multi": extra,
                "backgrounds_note": "backgrounds are drawn only while the active camera uses CAM_SET_PREREND_FIXED (Room_DrawImage)",
            });
        }
        _ => {
            s.json = json!({ "ptr": hex32(ptr), "type": ty, "type_name": "invalid" });
        }
    }
    Some(s)
}

// ---------------------------------------------------------------------------------------------
// Header command decoding
// ---------------------------------------------------------------------------------------------

struct Dec<'a> {
    t: &'a Tables,
    files: Files<'a>,
    own: u8,
    known: &'a BTreeSet<usize>,
    syms: &'a HashMap<u32, Symbol>,
    max_exit_ref: usize,
    entrance_needed: usize,
    room_names: &'a [String],
}

impl<'a> Dec<'a> {
    fn own_len(&self) -> usize {
        if self.own == 3 { self.files.room.map(|r| r.len()).unwrap_or(0) } else { self.files.scene.len() }
    }
    /// Bytes available from `off` (in the own file) until the next known data.
    fn span(&self, addr: u32) -> usize {
        if addr >> 24 != self.own as u32 {
            return 0;
        }
        let off = (addr & 0xFF_FFFF) as usize;
        let next = self.known.range(off + 1..).next().copied().unwrap_or(self.own_len());
        next.saturating_sub(off)
    }

    fn actor(&self, d: &[u8], o: usize) -> Option<Value> {
        let id = u16_at(d, o)?;
        let pos = vec3s(d, o + 2)?;
        let rot = vec3s(d, o + 8)?;
        let params = u16_at(d, o + 14)?;
        let (e, ovl) = self.t.actor(id);
        Some(json!({
            "id": id, "actor": e, "overlay": ovl,
            "pos": pos, "rot": rot, "rot_degrees": rot.map(binang_deg),
            "params": hex16(params),
        }))
    }

    fn decode(&self, w0: u32, w1: u32, offset: usize) -> Value {
        let code = (w0 >> 24) as usize;
        let d1 = ((w0 >> 16) & 0xFF) as usize;
        let mut v = json!({
            "code": format!("0x{code:02X}"),
            "command": CMD_NAMES.get(code).copied().unwrap_or("INVALID"),
            "offset": format!("0x{offset:X}"),
            "raw": [hex32(w0), hex32(w1)],
        });
        let res = self.files.resolve(w1);
        match code {
            0x00 | 0x01 => {
                let list: Vec<Value> = res.map(|(d, o)| (0..d1).filter_map(|i| self.actor(d, o + i * 16)).collect()).unwrap_or_default();
                v["count"] = json!(d1);
                v["ptr"] = json!(hex32(w1));
                v[if code == 0 { "spawns" } else { "actors" }] = json!(list);
                if code == 0 {
                    v["note"] = json!("player entry actors; the entrance list picks one by spawn index");
                }
            }
            0x02 | 0x09 => {
                v["data1"] = json!(d1);
                v["data2"] = json!(hex32(w1));
                v["note"] = json!("unused by the game");
            }
            0x03 => {
                v["ptr"] = json!(hex32(w1));
                v["see"] = json!("collision.json");
            }
            0x04 => {
                let mut rooms = Vec::new();
                if let Some((d, o)) = res {
                    for i in 0..d1 {
                        let (Some(s), Some(e)) = (u32_at(d, o + i * 8), u32_at(d, o + i * 8 + 4)) else { break };
                        rooms.push(json!({ "index": i, "vrom_start": hex32(s), "vrom_end": hex32(e), "file": self.room_names.get(i) }));
                    }
                }
                v["count"] = json!(d1);
                v["ptr"] = json!(hex32(w1));
                v["rooms"] = json!(rooms);
            }
            0x05 => {
                v["wind_direction"] = json!([(w1 >> 24) as u8 as i8, (w1 >> 16) as u8 as i8, (w1 >> 8) as u8 as i8]);
                v["wind_strength"] = json!(w1 & 0xFF);
            }
            0x06 => {
                let avail = self.span(w1) / 2;
                let mut n = avail.min(64);
                let mut list = Vec::new();
                if let Some((d, o)) = res {
                    let raw: Vec<(u8, u8)> = (0..n).map_while(|i| Some((u8_at(d, o + i * 2)?, u8_at(d, o + i * 2 + 1)?))).collect();
                    n = raw.len();
                    while n > self.entrance_needed && raw[n - 1] == (0, 0) {
                        n -= 1;
                    }
                    for (i, (spawn, room)) in raw[..n].iter().enumerate() {
                        list.push(json!({ "index": i, "spawn": spawn, "room": room }));
                    }
                }
                v["ptr"] = json!(hex32(w1));
                v["count"] = json!(list.len());
                v["count_basis"] = json!(format!(
                    "not stored: entries up to the next known data, trailing zero padding trimmed; the entrance table needs {}",
                    self.entrance_needed
                ));
                v["entrances"] = json!(list);
            }
            0x07 => {
                let keep = (w1 & 0xFFFF) as u16;
                let (e, f) = self.t.object(keep);
                v["navi_hint_file_index"] = json!(d1);
                v["navi_hint_file"] = json!(match d1 {
                    0 => "none",
                    1 => "elf_message_field",
                    2 => "elf_message_ydan",
                    _ => "unknown",
                });
                v["keep_object_id"] = json!(keep);
                v["keep_object"] = json!(e);
                v["keep_object_file"] = json!(f);
            }
            0x08 => {
                v["behavior_type1"] = json!(d1);
                v["behavior_type2"] = json!(w1 & 0xFF);
                v["lens_mode"] = json!((w1 >> 8) & 1);
                v["lens_mode_note"] = json!("1 = invisible actors are shown without the Lens of Truth");
                v["disable_warp_songs"] = json!((w1 >> 10) & 1 == 1);
            }
            0x0A => {
                v["ptr"] = json!(hex32(w1));
                if let Some(s) = parse_shape(self.files, w1) {
                    v["shape"] = s.json;
                }
            }
            0x0B => {
                let list: Vec<Value> = res
                    .map(|(d, o)| {
                        (0..d1)
                            .filter_map(|i| {
                                let id = u16_at(d, o + i * 2)?;
                                let (e, f) = self.t.object(id);
                                Some(json!({ "id": id, "object": e, "file": f }))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                v["count"] = json!(d1);
                v["ptr"] = json!(hex32(w1));
                v["objects"] = json!(list);
            }
            0x0C => {
                let mut list = Vec::new();
                if let Some((d, o)) = res {
                    for i in 0..d1 {
                        let b = o + i * 0xE;
                        let Some(ty) = u8_at(d, b) else { break };
                        let name = self.t.light_types.get(&(ty as i64)).cloned().unwrap_or_default();
                        if ty == 1 {
                            let (Some(x), Some(y), Some(z), Some(c)) = (s8_at(d, b + 2), s8_at(d, b + 3), s8_at(d, b + 4), rgb(d, b + 5)) else { break };
                            list.push(json!({ "index": i, "type": ty, "type_name": name, "direction": [x, y, z], "color": c }));
                        } else {
                            let (Some(p), Some(c), Some(g), Some(r)) = (vec3s(d, b + 2), rgb(d, b + 8), u8_at(d, b + 11), s16_at(d, b + 12)) else { break };
                            list.push(json!({ "index": i, "type": ty, "type_name": name, "pos": p, "color": c, "draw_glow": g, "radius": r }));
                        }
                    }
                }
                v["count"] = json!(d1);
                v["ptr"] = json!(hex32(w1));
                v["lights"] = json!(list);
            }
            0x0D => {
                let mut paths = Vec::new();
                if let Some((d, o)) = res {
                    let xml_n = self.syms.get(&(w1 & 0xFF_FFFF)).and_then(|s| s.attr("NumPaths")).and_then(|n| n.parse::<usize>().ok());
                    let max = xml_n.unwrap_or_else(|| (self.span(w1) / 8).min(64));
                    for i in 0..max {
                        let (Some(n), Some(pp)) = (u8_at(d, o + i * 8), u32_at(d, o + i * 8 + 4)) else { break };
                        let Some((pd, po)) = self.files.resolve(pp) else { break };
                        if xml_n.is_none() && n == 0 {
                            break;
                        }
                        let pts: Vec<[i16; 3]> = (0..n as usize).filter_map(|k| vec3s(pd, po + k * 6)).collect();
                        paths.push(json!({ "index": i, "num_points": n, "points_ptr": hex32(pp), "points": pts }));
                    }
                    v["count_basis"] = json!(if xml_n.is_some() { "decomp XML NumPaths" } else { "entries until the next known data" });
                }
                v["ptr"] = json!(hex32(w1));
                v["paths"] = json!(paths);
            }
            0x0E => {
                let mut list = Vec::new();
                if let Some((d, o)) = res {
                    for i in 0..d1 {
                        let b = o + i * 16;
                        let (Some(fr), Some(fc), Some(br), Some(bc), Some(id), Some(pos), Some(ry), Some(params)) = (
                            s8_at(d, b),
                            s8_at(d, b + 1),
                            s8_at(d, b + 2),
                            s8_at(d, b + 3),
                            u16_at(d, b + 4),
                            vec3s(d, b + 6),
                            s16_at(d, b + 12),
                            u16_at(d, b + 14),
                        ) else {
                            break;
                        };
                        let (e, ovl) = self.t.actor(id);
                        list.push(json!({
                            "index": i,
                            "front": { "room": fr, "bg_cam_index": fc },
                            "back": { "room": br, "bg_cam_index": bc },
                            "id": id, "actor": e, "overlay": ovl,
                            "pos": pos, "rot_y": ry, "rot_y_degrees": binang_deg(ry),
                            "params": hex16(params),
                        }));
                    }
                }
                v["count"] = json!(d1);
                v["ptr"] = json!(hex32(w1));
                v["transition_actors"] = json!(list);
            }
            0x0F => {
                let mut list = Vec::new();
                if let Some((d, o)) = res {
                    for i in 0..d1 {
                        let b = o + i * 0x16;
                        let (Some(amb), Some(d1v), Some(c1), Some(d2v), Some(c2), Some(fog), Some(near), Some(far)) = (
                            rgb(d, b),
                            rgb(d, b + 3),
                            rgb(d, b + 6),
                            rgb(d, b + 9),
                            rgb(d, b + 12),
                            rgb(d, b + 15),
                            u16_at(d, b + 18),
                            s16_at(d, b + 20),
                        ) else {
                            break;
                        };
                        list.push(json!({
                            "index": i,
                            "ambient_color": amb,
                            "light1_direction": d1v.map(|x| x as i8),
                            "light1_color": c1,
                            "light2_direction": d2v.map(|x| x as i8),
                            "light2_color": c2,
                            "fog_color": fog,
                            "fog_near": near & 0x3FF,
                            "blend_rate": (near >> 10) * 4,
                            "fog_near_raw": hex16(near),
                            "fog_far": far,
                        }));
                    }
                }
                v["count"] = json!(d1);
                v["ptr"] = json!(hex32(w1));
                v["note"] = json!("fog_near = raw & 0x3FF, blend_rate = (raw >> 10) * 4 (z_kankyo.c). With LIGHT_MODE_TIME the first entries are the time-of-day configs.");
                v["light_settings"] = json!(list);
            }
            0x10 => {
                let (h, m, s) = (w1 >> 24, (w1 >> 16) & 0xFF, (w1 >> 8) & 0xFF);
                v["hour"] = json!(h);
                v["minute"] = json!(m);
                v["time_speed"] = json!(s);
                v["note"] = json!("0xFF hour/minute keeps the current time; 0xFF speed = 0 (time stopped)");
            }
            0x11 => {
                let id = (w1 >> 24) as i64;
                v["skybox_id"] = json!(id);
                v["skybox"] = json!(self.t.skyboxes.get(&id).cloned().unwrap_or_default());
                v["skybox_config"] = json!((w1 >> 16) & 0xFF);
                let mode = (w1 >> 8) & 0xFF;
                v["env_light_mode"] = json!(mode);
                v["env_light_mode_name"] = json!(match mode {
                    0 => "LIGHT_MODE_TIME",
                    1 => "LIGHT_MODE_SETTINGS",
                    _ => "unknown",
                });
            }
            0x12 => {
                v["skybox_disabled"] = json!(w1 >> 24 != 0);
                v["sun_moon_disabled"] = json!((w1 >> 16) & 0xFF != 0);
            }
            0x13 => {
                let avail = (self.span(w1) / 2).min(32);
                let mut list = Vec::new();
                if let Some((d, o)) = res {
                    let raw: Vec<u16> = (0..avail).map_while(|i| u16_at(d, o + i * 2)).collect();
                    let mut n = raw.len();
                    while n > self.max_exit_ref && raw[n - 1] == 0 {
                        n -= 1;
                    }
                    for (i, &e) in raw[..n].iter().enumerate() {
                        let mut x = self.t.entrance(e);
                        x["exit_index"] = json!(i + 1);
                        x["entrance_index"] = json!(hex16(e));
                        list.push(x);
                    }
                }
                v["ptr"] = json!(hex32(w1));
                v["count"] = json!(list.len());
                v["count_basis"] = json!(format!(
                    "not stored: entries up to the next known data (max 31), trailing zeros trimmed; collision references up to {}",
                    self.max_exit_ref
                ));
                v["exits"] = json!(list);
            }
            0x14 => {}
            0x15 => {
                let seq = (w1 & 0xFF) as i64;
                let nat = ((w1 >> 8) & 0xFF) as i64;
                v["spec_id"] = json!(d1);
                v["nature_ambience_id"] = json!(nat);
                v["nature_ambience"] = json!(self.t.nature.get(&nat).cloned().unwrap_or_default());
                v["seq_id"] = json!(seq);
                v["seq"] = json!(self.t.seqs.get(&seq).cloned().unwrap_or_default());
            }
            0x16 => {
                v["echo"] = json!(w1 & 0xFF);
            }
            0x17 => {
                v["ptr"] = json!(hex32(w1));
                v["symbol"] = json!(self.syms.get(&(w1 & 0xFF_FFFF)).map(|s| s.name.clone()));
                v["note"] = json!("cutscene script not decoded");
            }
            0x18 => {
                v["ptr"] = json!(hex32(w1));
                v["note"] = json!("layers are listed as separate headers");
            }
            0x19 => {
                let ct = d1 as i64;
                v["scene_cam_type"] = json!(ct);
                v["scene_cam_type_name"] = json!(self.t.scene_cam_types.get(&ct).cloned().unwrap_or_else(|| "SCENE_CAM_TYPE_DEFAULT".into()));
                v["world_map_area"] = json!(w1);
            }
            _ => {
                v["note"] = json!("invalid command code; parsing stopped");
            }
        }
        v
    }
}

// ---------------------------------------------------------------------------------------------
// Scene extraction
// ---------------------------------------------------------------------------------------------

#[derive(Default)]
struct Totals {
    scenes: usize,
    rooms: usize,
    triangles: usize,
    tris_opa: usize,
    tris_xlu: usize,
    collision_polys: usize,
    actors_main: usize,
    actors_all_layers: usize,
    transition_actors: usize,
    backgrounds: usize,
    layers: usize,
    unknown_opcodes: BTreeMap<String, usize>,
    unresolved_distinct: BTreeMap<String, usize>,
    unresolved_refs: BTreeMap<String, usize>,
    textures: usize,
    materials: usize,
    errors: usize,
}

struct RoomData {
    index: usize,
    file: String,
    buf: Arc<[u8]>,
    layers: Vec<Option<usize>>,
    alt_list: Option<u32>,
    /// ZAPD `HackMode="syotes_room"`: the file starts with a room shape, not a header.
    headerless_shape: bool,
}

fn draw_stats(draw: &DrawList) -> Value {
    let mut unresolved: BTreeMap<String, usize> = BTreeMap::new();
    for (k, n) in &draw.stats.unresolved_addresses {
        *unresolved.entry(format!("0x{}", &k[..2.min(k.len())])).or_default() += n;
    }
    json!({
        "triangles": draw.triangle_count(),
        "materials": draw.materials.len(),
        "textures": draw.textures.len(),
        "commands": draw.stats.commands,
        "unknown_opcodes": draw.stats.unknown_opcodes,
        "unresolved_by_segment": unresolved,
        "unresolved_addresses": draw.stats.unresolved_addresses.iter().collect::<BTreeMap<_, _>>(),
        "ignored_opcodes": draw.stats.ignored_opcodes,
    })
}

struct SceneResult {
    summary: Value,
}

fn scene_short(file: &str) -> String {
    file.strip_suffix("_scene").unwrap_or(file).to_string()
}

fn decode_layers(dec: &Dec, layers: &[Option<usize>], headers: &[Option<Vec<(u32, u32)>>]) -> Vec<Value> {
    layers
            .iter()
            .zip(headers)
            .enumerate()
            .map(|(li, (off, h))| match (off, h) {
                (Some(o), Some(h)) => json!({
                    "layer": li,
                    "layer_name": layer_name(li),
                    "offset": format!("0x{o:X}"),
                    "commands": h.iter().enumerate().map(|(k, (w0, w1))| dec.decode(*w0, *w1, o + k * 8)).collect::<Vec<_>>(),
                }),
                _ => json!({ "layer": li, "layer_name": layer_name(li), "offset": null, "note": "no header for this layer (the game falls back to the main header; adult night falls back to adult day)" }),
            })
        .collect()
}

#[allow(clippy::too_many_lines)]
fn extract_scene(p: &Project, t: &Tables, sd: &SceneDef, base: &Path, shared: &Keeps, tot: &mut Totals) -> Result<SceneResult> {
    let scene_buf = p.rom.file_by_name(&sd.file).with_context(|| format!("scene file {}", sd.file))?;
    let short = scene_short(&sd.file);
    let xml_file = p.symbols.files.iter().find(|f| f.name == sd.file);
    let scenes_xml = crate::xml_root(p).join("scenes");
    let category = xml_file
        .and_then(|f| f.xml_path.strip_prefix(&scenes_xml).ok())
        .and_then(|r| r.parent())
        .map(|r| r.to_string_lossy().replace('\\', "/"))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "uncategorised".into());
    let dir = base.join(&category).join(crate::sanitize(&short));
    std::fs::create_dir_all(&dir)?;

    // XML symbols by file (for NumPaths / cutscene names / known offsets).
    let mut xml_syms: HashMap<String, HashMap<u32, Symbol>> = HashMap::new();
    let mut xml_room_count = 0usize;
    if let Some(xf) = xml_file {
        for f in p.symbols.files.iter().filter(|f| f.xml_path == xf.xml_path) {
            xml_room_count += f.of_kind("Room").count();
            xml_syms.insert(f.name.clone(), f.symbols.iter().map(|s| (s.offset, s.clone())).collect());
        }
    }
    let empty_syms = HashMap::new();

    // Known offsets in the scene file.
    let scene: &[u8] = &scene_buf;
    let mut known_scene: BTreeSet<usize> = xml_syms.get(&sd.file).map(|m| m.keys().map(|&k| k as usize).collect()).unwrap_or_default();
    let collect_ptrs = |cmds: &[(u32, u32)], seg: u32, set: &mut BTreeSet<usize>| {
        for &(w0, w1) in cmds {
            if cmd_has_pointer(w0 >> 24) && w1 >> 24 == seg {
                set.insert((w1 & 0xFF_FFFF) as usize);
            }
        }
    };
    collect_ptrs(&read_commands(scene, 0), 2, &mut known_scene);
    let (scene_layers, scene_alt_list) = read_layers(scene, 2, &known_scene);
    let scene_headers: Vec<Option<Vec<(u32, u32)>>> = scene_layers.iter().map(|l| l.map(|o| read_commands(scene, o))).collect();
    for (l, h) in scene_layers.iter().zip(&scene_headers) {
        if let (Some(o), Some(h)) = (l, h) {
            known_scene.insert(*o);
            collect_ptrs(h, 2, &mut known_scene);
        }
    }
    if let Some(a) = scene_alt_list {
        known_scene.insert((a & 0xFF_FFFF) as usize);
    }
    let main = scene_headers[0].clone().unwrap_or_default();
    let find_cmd = |h: &[(u32, u32)], code: u32| h.iter().find(|c| c.0 >> 24 == code).copied();

    // Rooms.
    let mut rooms: Vec<RoomData> = Vec::new();
    if let Some((w0, w1)) = find_cmd(&main, 0x04)
        && let Some((d, o)) = (Files { scene, room: None }).resolve(w1)
    {
        for i in 0..((w0 >> 16) & 0xFF) as usize {
            let Some(vs) = u32_at(d, o + i * 8) else { break };
            let Some(fi) = p.rom.file_containing_vrom(vs) else {
                log::warn!("{short}: room {i} vrom {vs:08X} not in dmadata");
                continue;
            };
            let name = p.rom.name_of(fi).map(|s| s.to_string()).unwrap_or_else(|| format!("file_{fi}"));
            let buf = p.rom.file(fi)?;
            let mut known: BTreeSet<usize> = xml_syms.get(&name).map(|m| m.keys().map(|&k| k as usize).collect()).unwrap_or_default();
            collect_ptrs(&read_commands(&buf, 0), 3, &mut known);
            let headerless_shape = xml_syms.get(&name).and_then(|m| m.get(&0)).and_then(|s| s.attr("HackMode")) == Some("syotes_room");
            let (layers, alt_list) = if headerless_shape { (vec![None], None) } else { read_layers(&buf, 3, &known) };
            rooms.push(RoomData { index: i, file: name, buf, layers, alt_list, headerless_shape });
        }
    }
    let room_names: Vec<String> = rooms.iter().map(|r| r.file.clone()).collect();
    let room_headers: Vec<Vec<Option<Vec<(u32, u32)>>>> =
        rooms.iter().map(|r| r.layers.iter().map(|l| l.map(|o| read_commands(&r.buf, o))).collect()).collect();
    let mut room_known: Vec<BTreeSet<usize>> = Vec::new();
    for (r, hs) in rooms.iter().zip(&room_headers) {
        let mut known: BTreeSet<usize> = xml_syms.get(&r.file).map(|m| m.keys().map(|&k| k as usize).collect()).unwrap_or_default();
        for (l, h) in r.layers.iter().zip(hs) {
            if let (Some(o), Some(h)) = (l, h) {
                known.insert(*o);
                collect_ptrs(h, 3, &mut known);
            }
        }
        if let Some(a) = r.alt_list {
            known.insert((a & 0xFF_FFFF) as usize);
        }
        room_known.push(known);
    }

    // Room shapes (distinct per room across layers) and camera indices they need.
    let mut extra_cams = 0usize;
    let mut room_shapes: Vec<Vec<(Vec<usize>, Shape)>> = Vec::new();
    for (ri, hs) in room_headers.iter().enumerate() {
        let files = Files { scene, room: Some(&rooms[ri].buf[..]) };
        let mut shapes: Vec<(u32, Vec<usize>)> = Vec::new();
        for (li, h) in hs.iter().enumerate() {
            if let Some(h) = h
                && let Some((_, w1)) = find_cmd(h, 0x0A)
            {
                match shapes.iter_mut().find(|s| s.0 == w1) {
                    Some(s) => s.1.push(li),
                    None => shapes.push((w1, vec![li])),
                }
            }
        }
        if rooms[ri].headerless_shape {
            shapes.push((0x0300_0000, vec![0]));
        }
        let parsed: Vec<(Vec<usize>, Shape)> = shapes.into_iter().filter_map(|(ptr, ls)| Some((ls, parse_shape(files, ptr)?))).collect();
        for (_, s) in &parsed {
            for b in &s.backgrounds {
                if let Some(c) = b.bg_cam_index {
                    extra_cams = extra_cams.max(c as usize + 1);
                }
            }
        }
        room_shapes.push(parsed);
    }
    // Door camera indices.
    for h in scene_headers.iter().flatten() {
        if let Some((w0, w1)) = find_cmd(h, 0x0E)
            && let Some((d, o)) = (Files { scene, room: None }).resolve(w1)
        {
            for i in 0..((w0 >> 16) & 0xFF) as usize {
                for k in [1, 3] {
                    if let Some(c) = s8_at(d, o + i * 16 + k)
                        && c >= 0
                    {
                        extra_cams = extra_cams.max(c as usize + 1);
                    }
                }
            }
        }
    }

    // Collision (distinct headers across layers).
    let mut col_ptrs: Vec<(u32, Vec<usize>)> = Vec::new();
    for (li, h) in scene_headers.iter().enumerate() {
        if let Some(h) = h
            && let Some((_, w1)) = find_cmd(h, 0x03)
        {
            match col_ptrs.iter_mut().find(|c| c.0 == w1) {
                Some(c) => c.1.push(li),
                None => col_ptrs.push((w1, vec![li])),
            }
        }
    }
    let files_scene = Files { scene, room: None };
    for (ptr, _) in &col_ptrs {
        if let Some((d, o)) = files_scene.resolve(*ptr) {
            for k in [0x10, 0x18, 0x1C, 0x20, 0x28] {
                if let Some(v) = u32_at(d, o + k)
                    && v >> 24 == 2
                {
                    known_scene.insert((v & 0xFF_FFFF) as usize);
                }
            }
        }
    }
    let collisions: Vec<(Vec<usize>, Collision)> = col_ptrs
        .iter()
        .filter_map(|(ptr, ls)| Some((ls.clone(), parse_collision(t, *ptr, &known_scene, extra_cams, files_scene)?)))
        .collect();
    let max_exit_ref = collisions.iter().flat_map(|(_, c)| c.surfaces.iter()).map(|s| ((s[0] >> 8) & 0x1F) as usize).max().unwrap_or(0);
    let entrance_needed = t
        .entrances
        .iter()
        .filter(|e| e.1 == sd.enum_name)
        .filter_map(|e| e.2.parse::<usize>().ok())
        .map(|s| s + 1)
        .max()
        .unwrap_or(0);

    // Decode all headers.
    let scene_syms = xml_syms.get(&sd.file).unwrap_or(&empty_syms);
    let dec_scene = Dec {
        t,
        files: files_scene,
        own: 2,
        known: &known_scene,
        syms: scene_syms,
        max_exit_ref,
        entrance_needed,
        room_names: &room_names,
    };
    let scene_headers_json = decode_layers(&dec_scene, &scene_layers, &scene_headers);

    // Draw config.
    let func = t.sdc_funcs.get(&sd.draw_config).cloned().unwrap_or_else(|| "Scene_DrawConfigDefault".into());
    let cfg = t.drawcfg.run(&func, sd.id as i64, &t.scene_ids);
    let keeps = Keeps {
        code: shared.code.clone(),
        gameplay_keep: shared.gameplay_keep.clone(),
        scene_keep: find_cmd(&main, 0x07).and_then(|(_, w1)| {
            let (_, f) = t.object((w1 & 0xFFFF) as u16);
            if f.is_empty() { None } else { p.rom.file_by_name(&f).ok() }
        }),
    };
    let ctx = SceneCtx {
        p,
        scene_file: sd.file.clone(),
        scene_buf: scene_buf.clone(),
        room_files: room_names.clone(),
        room_bufs: rooms.iter().map(|r| r.buf.clone()).collect(),
    };

    // Meshes.
    let mut g = Gltf::new();
    let root = g.node(json!({ "name": short }));
    let mut rooms_json = Vec::new();
    let mut unresolved_scene: BTreeMap<String, (BTreeSet<String>, usize)> = BTreeMap::new();
    let mut unknown_scene: BTreeMap<String, usize> = BTreeMap::new();
    let mut tri_scene = 0usize;
    let mut bg_count = 0usize;
    let mut notes: BTreeSet<String> = BTreeSet::new();
    let bg_dir = dir.join("backgrounds");
    for (ri, r) in rooms.iter().enumerate() {
        let opa_segs = ctx.segments(ri, &keeps, &cfg.opa, &mut notes);
        let xlu_segs = ctx.segments(ri, &keeps, &cfg.xlu, &mut notes);
        let room_node = g.node(json!({ "name": format!("room_{ri}"), "extras": { "room": ri, "file": r.file } }));
        g.add_child(root, room_node);
        let mut shapes_json = Vec::new();
        for (si, (layers, shape)) in room_shapes[ri].iter().enumerate() {
            let suffix = if si == 0 { String::new() } else { format!("_layer{}", layers[0]) };
            let parent = if si == 0 {
                room_node
            } else {
                let n = g.node(json!({ "name": format!("room_{ri}{suffix}"), "extras": { "layers": layers } }));
                g.add_child(room_node, n);
                n
            };
            let mut stats = json!({});
            for (kind, segs, dls) in [("opa", &opa_segs, &shape.opa), ("xlu", &xlu_segs, &shape.xlu)] {
                let draw = run_dls(segs, dls);
                let tris = draw.triangle_count();
                tri_scene += tris;
                if kind == "opa" {
                    tot.tris_opa += tris;
                } else {
                    tot.tris_xlu += tris;
                }
                tot.textures += draw.textures.len();
                tot.materials += draw.materials.len();
                for (k, n) in &draw.stats.unknown_opcodes {
                    *unknown_scene.entry(k.clone()).or_default() += n;
                }
                for (k, n) in &draw.stats.unresolved_addresses {
                    let e = unresolved_scene.entry(format!("0x{}", &k[..2.min(k.len())])).or_default();
                    e.0.insert(k.clone());
                    e.1 += n;
                }
                stats[kind] = draw_stats(&draw);
                let name = format!("room_{ri}{suffix}_{kind}");
                if let Some(mesh) = g.add_draw_list(&draw, &name, None, None) {
                    let n = g.node(json!({ "name": name, "mesh": mesh, "extras": { "draw_buffer": kind.to_uppercase(), "triangles": tris } }));
                    g.add_child(parent, n);
                }
            }
            // Prerendered backgrounds.
            let files = Files { scene, room: Some(&r.buf[..]) };
            let mut bgs = Vec::new();
            for b in &shape.backgrounds {
                let mut bj = json!({
                    "index": b.index,
                    "bg_cam_index": b.bg_cam_index,
                    "unk_00": b.unk_00,
                    "source": hex32(b.source),
                    "unk_0c": hex32(b.unk_0c),
                    "tlut": hex32(b.tlut),
                    "width": b.width, "height": b.height,
                    "fmt": b.fmt, "siz": b.siz,
                    "fmt_note": "G_IM_FMT_* / G_IM_SIZ_*; RGBA16 is drawn with S2DEX gSPBgRectCopy after JPEG decode",
                    "tlut_mode": b.tlut_mode, "tlut_count": b.tlut_count,
                });
                if let Some((d, o)) = files.resolve(b.source) {
                    if d.get(o..o + 4) == Some(&[0xFF, 0xD8, 0xFF, 0xE0][..]) {
                        let end = (o + 2..d.len().saturating_sub(1)).find(|&k| d[k] == 0xFF && d[k + 1] == 0xD9).map(|k| k + 2).unwrap_or(d.len());
                        let fname = match b.bg_cam_index {
                            Some(c) => format!("room_{ri}{suffix}_bg{}_cam{c}.jpg", b.index),
                            None => format!("room_{ri}{suffix}_bg{}.jpg", b.index),
                        };
                        std::fs::create_dir_all(&bg_dir)?;
                        std::fs::write(bg_dir.join(&fname), &d[o..end])?;
                        bj["file"] = json!(format!("backgrounds/{fname}"));
                        bj["jpeg_bytes"] = json!(end - o);
                        bj["source_file"] = json!(if b.source >> 24 == 2 { sd.file.clone() } else { r.file.clone() });
                        bg_count += 1;
                    } else {
                        bj["note"] = json!("source does not start with a JFIF marker");
                    }
                } else {
                    bj["note"] = json!("source pointer does not resolve");
                }
                bgs.push(bj);
            }
            let bg_files: Vec<Value> = bgs.iter().filter_map(|b| b.get("file").cloned()).collect();
            if !bg_files.is_empty() {
                g.nodes[parent]["extras"]["backgrounds"] = json!(bg_files);
            }
            g.nodes[parent]["extras"]["shape_type"] = json!(shape.ty);
            shapes_json.push(json!({ "layers": layers, "type": shape.ty, "shape": shape.json, "mesh_stats": stats, "backgrounds": bgs }));
        }
        let rsyms = xml_syms.get(&r.file).unwrap_or(&empty_syms);
        let dec_room = Dec {
            t,
            files: Files { scene, room: Some(&r.buf[..]) },
            own: 3,
            known: &room_known[ri],
            syms: rsyms,
            max_exit_ref,
            entrance_needed,
            room_names: &room_names,
        };
        rooms_json.push(json!({
            "index": r.index,
            "file": r.file,
            "size": r.buf.len(),
            "alternate_header_list": r.alt_list.map(hex32),
            "headerless_shape": r.headerless_shape.then_some("XML HackMode=syotes_room: no command header, the room shape is at offset 0"),
            "headers": decode_layers(&dec_room, &r.layers, &room_headers[ri]),
            "shapes": shapes_json,
        }));
    }

    // Collision in the glb.
    let mut col_json = Vec::new();
    let mut poly_count = 0usize;
    for (ci, (layers, c)) in collisions.iter().enumerate() {
        if ci == 0 {
            poly_count = c.polys.len();
        }
        let name = if ci == 0 { "collision".to_string() } else { format!("collision_layer{}", layers[0]) };
        let mut groups: BTreeMap<(u16, u16, bool), Vec<[f32; 3]>> = BTreeMap::new();
        for p in &c.polys {
            let idx = [(p.via & 0x1FFF) as usize, (p.vib & 0x1FFF) as usize, p.vic as usize];
            let Some(tri) = idx.iter().map(|&i| c.verts.get(i).map(|v| v.map(|x| x as f32))).collect::<Option<Vec<_>>>() else { continue };
            groups.entry((p.ty, p.via >> 13, p.vib & 0x2000 != 0)).or_default().extend(tri);
        }
        let prims: Vec<(Vec<[f32; 3]>, Value)> = groups
            .into_iter()
            .map(|((ty, xp, conv), tris)| {
                let mut e = c.surfaces.get(ty as usize).map(|s| surface_decode(t, *s)).unwrap_or_else(|| json!({}));
                e["surface_type"] = json!(ty);
                e["poly_flags_a"] = json!(xp);
                e["ignore_camera"] = json!(xp & 1 != 0);
                e["ignore_entities"] = json!(xp & 2 != 0);
                e["ignore_projectiles"] = json!(xp & 4 != 0);
                e["floor_conveyor"] = json!(conv);
                e["triangles"] = json!(tris.len() / 3);
                (tris, e)
            })
            .collect();
        if let Some(mesh) = g.add_triangles(&name, &prims, [0.25, 0.85, 0.35, 0.45]) {
            let n = g.node(json!({ "name": name, "mesh": mesh, "extras": { "layers": layers, "polys": c.polys.len(), "surface_types": c.surfaces.len(), "ptr": hex32(c.ptr) } }));
            g.add_child(root, n);
        }
        let wprims: Vec<(Vec<[f32; 3]>, Value)> = c
            .water_raw
            .iter()
            .zip(&c.water)
            .map(|(([x, y, z, xl, zl], _), j)| {
                let (x0, y, z0) = (*x as f32, *y as f32, *z as f32);
                let (x1, z1) = (x0 + *xl as f32, z0 + *zl as f32);
                (vec![[x0, y, z0], [x0, y, z1], [x1, y, z1], [x0, y, z0], [x1, y, z1], [x1, y, z0]], j.clone())
            })
            .collect();
        if let Some(mesh) = g.add_triangles(&format!("water_boxes{}", if ci == 0 { String::new() } else { format!("_layer{}", layers[0]) }), &wprims, [0.2, 0.4, 1.0, 0.4]) {
            let n = g.node(json!({ "name": if ci == 0 { "water_boxes".to_string() } else { format!("water_boxes_layer{}", layers[0]) }, "mesh": mesh }));
            g.add_child(root, n);
        }
        let file = if ci == 0 { "collision.json".to_string() } else { format!("collision_layer{}.json", layers[0]) };
        std::fs::write(dir.join(&file), serde_json::to_string(&c.json)?)?;
        col_json.push(json!({ "layers": layers, "ptr": hex32(c.ptr), "file": file, "vertices": c.verts.len(), "polys": c.polys.len(), "surface_types": c.surfaces.len(), "water_boxes": c.water.len(), "bg_cams": c.json["bg_cams"].as_array().map(|a| a.len()) }));
    }

    // Actors as empties, per layer.
    let actors_node = g.node(json!({ "name": "actors" }));
    g.add_child(root, actors_node);
    let mut actors_main = 0usize;
    let mut actors_all = 0usize;
    let mut trans_count = 0usize;
    let empty = |g: &mut Gltf, parent: usize, label: &str, pos: [i16; 3], rot: [i16; 3], extras: Value| {
        let q = Quat::from_euler(EulerRot::YXZ, binang_rad(rot[1]), binang_rad(rot[0]), binang_rad(rot[2]));
        let n = g.node(json!({
            "name": label,
            "translation": pos.map(|v| v as f32),
            "rotation": [q.x, q.y, q.z, q.w],
            "extras": extras,
        }));
        g.add_child(parent, n);
    };
    let actor_nodes = |g: &mut Gltf, parent: usize, d: &[u8], o: usize, n: usize, kind: &str| -> usize {
        let mut c = 0;
        for i in 0..n {
            let b = o + i * 16;
            let (Some(id), Some(pos), Some(rot), Some(params)) = (u16_at(d, b), vec3s(d, b + 2), vec3s(d, b + 8), u16_at(d, b + 14)) else { break };
            let (e, ovl) = t.actor(id);
            let label = format!("{}_{:04X}", if ovl.is_empty() { e.clone() } else { ovl.clone() }, params);
            empty(g, parent, &label, pos, rot, json!({ "kind": kind, "index": i, "actor_id": id, "actor": e, "params": hex16(params), "rot_raw": rot }));
            c += 1;
        }
        c
    };
    for (li, h) in scene_headers.iter().enumerate() {
        let Some(h) = h else { continue };
        let ln = g.node(json!({ "name": format!("layer_{li}_{}", layer_name(li)) }));
        g.add_child(actors_node, ln);
        if let Some((w0, w1)) = find_cmd(h, 0x00)
            && let Some((d, o)) = files_scene.resolve(w1)
        {
            let sp = g.node(json!({ "name": format!("layer_{li}_spawns") }));
            g.add_child(ln, sp);
            actor_nodes(&mut g, sp, d, o, ((w0 >> 16) & 0xFF) as usize, "spawn");
        }
        if let Some((w0, w1)) = find_cmd(h, 0x0E)
            && let Some((d, o)) = files_scene.resolve(w1)
        {
            let tn = g.node(json!({ "name": format!("layer_{li}_transition_actors") }));
            g.add_child(ln, tn);
            for i in 0..((w0 >> 16) & 0xFF) as usize {
                let b = o + i * 16;
                let (Some(id), Some(pos), Some(ry), Some(params)) = (u16_at(d, b + 4), vec3s(d, b + 6), s16_at(d, b + 12), u16_at(d, b + 14)) else { break };
                let (e, ovl) = t.actor(id);
                let label = format!("{}_{:04X}", if ovl.is_empty() { e.clone() } else { ovl.clone() }, params);
                empty(&mut g, tn, &label, pos, [0, ry, 0], json!({ "kind": "transition_actor", "index": i, "actor_id": id, "actor": e, "params": hex16(params), "front_room": s8_at(d, b), "back_room": s8_at(d, b + 2) }));
                if li == 0 {
                    trans_count += 1;
                }
            }
        }
        for (ri, r) in rooms.iter().enumerate() {
            // Room layer selection as in Scene_CommandAlternateHeaderList.
            let hs = &room_headers[ri];
            let pick = |k: usize| hs.get(k).and_then(|x| x.as_ref());
            let rh = if li == 0 { pick(0) } else { pick(li).or(if li == 3 { pick(2) } else { None }).or(pick(0)) };
            let Some(rh) = rh else { continue };
            if let Some((w0, w1)) = find_cmd(rh, 0x01)
                && let Some((d, o)) = (Files { scene, room: Some(&r.buf[..]) }).resolve(w1)
            {
                let rn = g.node(json!({ "name": format!("layer_{li}_room_{ri}_actors") }));
                g.add_child(ln, rn);
                let c = actor_nodes(&mut g, rn, d, o, ((w0 >> 16) & 0xFF) as usize, "actor");
                if li == 0 {
                    actors_main += c;
                }
            }
        }
    }
    // Count every actor entry in every distinct header (for the summary).
    for hs in &room_headers {
        for h in hs.iter().flatten() {
            if let Some((w0, _)) = find_cmd(h, 0x01) {
                actors_all += ((w0 >> 16) & 0xFF) as usize;
            }
        }
    }

    let unresolved_json: BTreeMap<String, Value> =
        unresolved_scene.iter().map(|(k, (set, n))| (k.clone(), json!({ "distinct_addresses": set.len(), "references": n }))).collect();
    let summary = json!({
        "scene": short,
        "category": category,
        "scene_id": sd.id,
        "rooms": rooms.len(),
        "xml_rooms": xml_room_count,
        "layers": scene_layers.iter().filter(|l| l.is_some()).count(),
        "triangles": tri_scene,
        "collision_polys": poly_count,
        "actors_main_layer": actors_main,
        "actors_all_layers": actors_all,
        "transition_actors": trans_count,
        "backgrounds": bg_count,
        "unknown_opcodes": unknown_scene,
        "unresolved_by_segment": unresolved_json,
        "draw_config": func,
    });
    g.extras = Some(json!({ "scene": short, "file": sd.file, "units": UNITS, "summary": summary }));
    let glb = dir.join(format!("{}.glb", crate::sanitize(&short)));
    g.write_glb(&glb, &[root])?;

    let scene_json = json!({
        "scene": short,
        "file": sd.file,
        "scene_id": sd.id,
        "enum": sd.enum_name,
        "title_card": sd.title,
        "scene_table_unk_10": sd.unk_10,
        "scene_table_unk_12": sd.unk_12,
        "category": category,
        "units": UNITS,
        "glb": glb.file_name().map(|f| f.to_string_lossy().to_string()),
        "size": scene_buf.len(),
        "alternate_header_list": scene_alt_list.map(hex32),
        "headers": scene_headers_json,
        "rooms": rooms_json,
        "collision": col_json,
        "draw_config": {
            "sdc": sd.draw_config,
            "function": func,
            "source": "src/code/z_scene_table.c, run with the state of a fresh child-day load at frame 0 (gameplayFrames = 0, nightFlag = 0, sceneLayer = 0)",
            "opa": cfg.opa.to_json(),
            "xlu": cfg.xlu.to_json(),
            "unknown_runtime_state": cfg.assumed,
            "unknown_runtime_state_note": "names read as 0 (runtime state not modelled); name() = call not modelled, skipped, result read as 0",
            "errors": cfg.errors,
            "binding_notes": notes,
        },
        "summary": summary,
    });
    crate::write_json(&dir.join("scene.json"), &scene_json)?;

    tot.scenes += 1;
    tot.rooms += rooms.len();
    tot.triangles += tri_scene;
    tot.collision_polys += poly_count;
    tot.actors_main += actors_main;
    tot.actors_all_layers += actors_all;
    tot.transition_actors += trans_count;
    tot.backgrounds += bg_count;
    tot.layers += scene_layers.iter().filter(|l| l.is_some()).count();
    for (k, n) in unknown_scene {
        *tot.unknown_opcodes.entry(k).or_default() += n;
    }
    for (k, (set, n)) in unresolved_scene {
        *tot.unresolved_distinct.entry(k.clone()).or_default() += set.len();
        *tot.unresolved_refs.entry(k).or_default() += n;
    }
    Ok(SceneResult { summary })
}

pub fn extract(p: &Project, out: &Path) -> Result<Value> {
    let t = Tables::load(&p.config.decomp)?;
    let base = out.join("scenes");
    std::fs::create_dir_all(&base)?;
    let shared = Keeps { code: code_ram_image(p), gameplay_keep: p.rom.file_by_name("gameplay_keep").ok(), scene_keep: None };
    let mut tot = Totals::default();
    let mut index = Vec::new();
    for sd in &t.scenes {
        match extract_scene(p, &t, sd, &base, &shared, &mut tot) {
            Ok(r) => index.push(r.summary),
            Err(e) => {
                tot.errors += 1;
                log::warn!("scene {}: {e:#}", sd.file);
                index.push(json!({ "scene": sd.file, "error": format!("{e:#}") }));
            }
        }
    }
    let scenes_xml = crate::xml_root(p).join("scenes");
    let in_scenes = |f: &&oot_import::symbols::AssetFile| f.xml_path.starts_with(&scenes_xml);
    let xml_scenes: usize = p.symbols.files.iter().filter(in_scenes).map(|f| f.of_kind("Scene").count()).sum();
    let xml_rooms: usize = p.symbols.files.iter().filter(in_scenes).map(|f| f.of_kind("Room").count()).sum();
    let summary = json!({
        "scenes": tot.scenes,
        "scene_table_entries": t.scenes.len(),
        "xml_scenes": xml_scenes,
        "rooms": tot.rooms,
        "xml_rooms": xml_rooms,
        "scene_layers": tot.layers,
        "triangles": tot.triangles,
        "triangles_opa": tot.tris_opa,
        "triangles_xlu": tot.tris_xlu,
        "collision_polys": tot.collision_polys,
        "actors_main_layer": tot.actors_main,
        "actors_all_layers": tot.actors_all_layers,
        "transition_actors_main_layer": tot.transition_actors,
        "backgrounds": tot.backgrounds,
        "unknown_opcodes": tot.unknown_opcodes,
        "unresolved_distinct_by_segment": tot.unresolved_distinct,
        "unresolved_refs_by_segment": tot.unresolved_refs,
        "errors": tot.errors,
    });
    crate::write_json(&base.join("index.json"), &json!({ "units": UNITS, "summary": summary, "scenes": index }))?;
    Ok(summary)
}
