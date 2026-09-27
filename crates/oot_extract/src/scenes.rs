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
use glam::{EulerRot, Quat};
use oot_core::gbi::{DrawList, Interpreter, Segment};
use oot_core::project::Project;
use oot_core::symbols::Symbol;
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

/// `DEFINE_X(a, b, ...)` rows in table order.
fn define_rows(text: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut l = line.trim();
        if l.starts_with("/*") {
            match l.find("*/") {
                Some(e) => l = l[e + 2..].trim(),
                None => continue,
            }
        }
        if !l.starts_with("DEFINE_") {
            continue;
        }
        let (Some(a), Some(b)) = (l.find('('), l.rfind(')')) else { continue };
        if b < a {
            continue;
        }
        let args = l[a + 1..b].split(',').map(|s| s.trim().trim_matches('"').to_string()).collect();
        out.push((l[..a].trim().to_string(), args));
    }
    out
}

fn strip_comments(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}

fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim().trim_end_matches(['u', 'U', 'l', 'L']);
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(h, 16).ok()
    } else {
        s.parse().ok()
    }
}

/// Values of the `typedef enum` that contains `member`.
fn parse_enum(text: &str, member: &str) -> HashMap<i64, String> {
    let clean = strip_comments(text);
    let mut out = HashMap::new();
    let mut search = 0;
    while let Some(rel) = clean[search..].find(member) {
        let pos = search + rel;
        search = pos + member.len();
        let before_ok = pos == 0 || !clean.as_bytes()[pos - 1].is_ascii_alphanumeric() && clean.as_bytes()[pos - 1] != b'_';
        let after = clean.as_bytes().get(pos + member.len()).copied().unwrap_or(b' ');
        if !before_ok || after.is_ascii_alphanumeric() || after == b'_' {
            continue;
        }
        let Some(open) = clean[..pos].rfind('{') else { continue };
        if !clean[open.saturating_sub(40)..open].contains("enum") {
            continue;
        }
        let Some(close_rel) = clean[pos..].find('}') else { continue };
        let body = &clean[open + 1..pos + close_rel];
        let mut next = 0i64;
        for item in body.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            let (name, val) = match item.split_once('=') {
                Some((n, e)) => (n.trim(), parse_int(e).unwrap_or(next)),
                None => (item, next),
            };
            out.insert(val, name.to_string());
            next = val + 1;
        }
        break;
    }
    out
}

fn parse_defines(text: &str, prefix: &str) -> HashMap<i64, String> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("#define ") else { continue };
        let mut it = rest.split_whitespace();
        let (Some(name), Some(val)) = (it.next(), it.next()) else { continue };
        if name.starts_with(prefix)
            && let Some(v) = parse_int(val)
        {
            out.entry(v).or_insert_with(|| name.to_string());
        }
    }
    out
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

/// VRAM start of `code` in gc-eu-mq-dbg. Room DLs sometimes hold raw KSEG0 pointers into
/// `code` (Jabu-Jabu loads `gMtxClear` at 0x8012DB20); the interpreter maps 0x80xxxxxx to
/// segment 0, so segment 0 gets a RAM image of `code`.
const CODE_VRAM: u32 = 0x8001_CE60;
const GMTXCLEAR_VRAM: u32 = 0x8012_DB20;

/// RAM image of `code` from 0x80000000, with ENDDL bytes below it so stray segment-0
/// references stop immediately. None unless gMtxClear is where this ROM layout expects it.
fn code_ram_image(p: &Project) -> Option<Arc<[u8]>> {
    let code = p.rom.file_by_name("code").ok()?;
    let at = (GMTXCLEAR_VRAM - CODE_VRAM) as usize;
    let m = code.get(at..at + 64)?;
    let ident = m[..32].chunks(2).enumerate().all(|(i, c)| u16::from_be_bytes([c[0], c[1]]) == if i % 5 == 0 { 1 } else { 0 }) && m[32..].iter().all(|&b| b == 0);
    if !ident {
        log::warn!("scenes: code layout differs from gc-eu-mq-dbg; KSEG0 pointers stay unresolved");
        return None;
    }
    let pad = (CODE_VRAM & 0x00FF_FFFF) as usize;
    let mut img = Vec::with_capacity(pad + code.len());
    for i in 0..pad {
        img.push(if i % 8 == 0 { 0xDF } else { 0 });
    }
    img.extend_from_slice(&code);
    Some(img.into())
}

fn identity_mtx() -> Arc<[u8]> {
    let mut b = vec![0u8; 64];
    for i in 0..4 {
        b[(i * 4 + i) * 2 + 1] = 1;
    }
    b.into()
}

fn cmds_to_bytes(cmds: &[(u32, u32)]) -> Vec<u8> {
    let mut b = Vec::with_capacity(cmds.len() * 8 + 8);
    for (w0, w1) in cmds {
        b.extend_from_slice(&w0.to_be_bytes());
        b.extend_from_slice(&w1.to_be_bytes());
    }
    if cmds.last().is_none_or(|c| c.0 >> 24 != 0xDF) {
        b.extend_from_slice(&0xDF00_0000u32.to_be_bytes());
        b.extend_from_slice(&0u32.to_be_bytes());
    }
    b
}

struct Keeps {
    code: Option<Arc<[u8]>>,
    gameplay_keep: Option<Arc<[u8]>>,
    scene_keep: Option<Arc<[u8]>>,
}

struct SceneCtx<'a> {
    p: &'a Project,
    scene_file: String,
    scene_buf: Arc<[u8]>,
    room_files: Vec<String>,
    room_bufs: Vec<Arc<[u8]>>,
}

impl SceneCtx<'_> {
    /// (file name, offset) of a C symbol: decomp XML first (this scene's files, then all),
    /// then ZAPD's generated `<file>DL_XXXXXX` / `<file>Tex_XXXXXX` names.
    fn find_symbol(&self, name: &str) -> Option<(String, u32)> {
        let own: Vec<&str> = std::iter::once(self.scene_file.as_str()).chain(self.room_files.iter().map(|s| s.as_str())).collect();
        for f in &self.p.symbols.files {
            if own.contains(&f.name.as_str())
                && let Some(s) = f.find(name)
            {
                return Some((f.name.clone(), s.offset));
            }
        }
        for f in &self.p.symbols.files {
            if let Some(s) = f.find(name) {
                return Some((f.name.clone(), s.offset));
            }
        }
        for f in own {
            if let Some(rest) = name.strip_prefix(f) {
                let hexpart = rest.rsplit('_').next().unwrap_or("");
                if hexpart.len() == 6
                    && let Ok(off) = u32::from_str_radix(hexpart, 16)
                {
                    return Some((f.to_string(), off));
                }
            }
        }
        None
    }

    fn file_buf(&self, file: &str) -> Option<Arc<[u8]>> {
        if file == self.scene_file {
            return Some(self.scene_buf.clone());
        }
        if let Some(i) = self.room_files.iter().position(|r| r == file) {
            return Some(self.room_bufs[i].clone());
        }
        self.p.rom.file_by_name(file).ok()
    }

    /// Segment address of a symbol as seen while drawing `room`.
    fn seg_addr(&self, name: &str) -> Option<u32> {
        let (file, off) = self.find_symbol(name)?;
        if file == self.scene_file {
            Some(0x0200_0000 | off)
        } else if self.room_files.contains(&file) {
            Some(0x0300_0000 | off)
        } else {
            None
        }
    }

    fn materialize(&self, cmds: &[drawcfg::Cmd], notes: &mut BTreeSet<String>) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        for c in cmds {
            match c {
                drawcfg::Cmd::Raw(a, b) => out.push((*a, *b)),
                drawcfg::Cmd::CallSym(s) if s == "gEmptyDL" => {}
                drawcfg::Cmd::CallSym(s) => match self.seg_addr(s) {
                    Some(addr) => out.push((0xDE00_0000, addr)),
                    None => {
                        notes.insert(format!("unresolved symbol {s}"));
                    }
                },
            }
        }
        out
    }

    /// Interpreter segments for one draw buffer: 2 scene, 3 room, 4 gameplay_keep, 5 keep, 8..D draw config.
    fn segments(
        &self,
        room: usize,
        keeps: &Keeps,
        buf: &drawcfg::Buf,
        notes: &mut BTreeSet<String>,
    ) -> ([Option<Segment>; 16], Vec<(u32, u32)>) {
        let mut segs: [Option<Segment>; 16] = Default::default();
        if let Some(c) = &keeps.code {
            segs[0] = Some(Segment::Data { buf: c.clone(), base: 0 });
        }
        let (keep4, keep5) = (&keeps.gameplay_keep, &keeps.scene_keep);
        segs[2] = Some(Segment::Data { buf: self.scene_buf.clone(), base: 0 });
        segs[3] = Some(Segment::Data { buf: self.room_bufs[room].clone(), base: 0 });
        if let Some(k) = keep4 {
            segs[4] = Some(Segment::Data { buf: k.clone(), base: 0 });
        }
        if let Some(k) = keep5 {
            segs[5] = Some(Segment::Data { buf: k.clone(), base: 0 });
        }
        for (&s, b) in &buf.segments {
            let seg = match &b.bind {
                drawcfg::Bind::Sym(name) if name == "gEmptyDL" => Some(Segment::Data { buf: cmds_to_bytes(&[]).into(), base: 0 }),
                drawcfg::Bind::Sym(name) => match self.find_symbol(name).and_then(|(f, o)| Some((self.file_buf(&f)?, o))) {
                    Some((fb, off)) => Some(Segment::Data { buf: fb, base: off as usize }),
                    None => {
                        notes.insert(format!("segment 0x{s:02X}: symbol {name} not found"));
                        None
                    }
                },
                drawcfg::Bind::Dl(cmds) => Some(Segment::Data { buf: cmds_to_bytes(&self.materialize(cmds, notes)).into(), base: 0 }),
                drawcfg::Bind::Mtx => Some(Segment::Data { buf: identity_mtx(), base: 0 }),
                drawcfg::Bind::Unknown(_) => None,
            };
            if seg.is_some() {
                segs[s as usize & 0xF] = seg;
            }
        }
        let pre = self.materialize(&buf.pre, notes);
        (segs, pre)
    }
}

fn run_dls(segs: &[Option<Segment>; 16], pre: &[(u32, u32)], dls: &[u32]) -> DrawList {
    let mut it = Interpreter::new();
    it.segments = segs.clone();
    it.apply_setup_dl_25();
    if !pre.is_empty() {
        it.segments[0xF] = Some(Segment::Data { buf: cmds_to_bytes(pre).into(), base: 0 });
        it.run(0x0F00_0000);
        it.segments[0xF] = None;
    }
    for &dl in dls {
        if dl != 0 {
            it.run(dl);
        }
    }
    it.draw
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
        let (opa_segs, opa_pre) = ctx.segments(ri, &keeps, &cfg.opa, &mut notes);
        let (xlu_segs, xlu_pre) = ctx.segments(ri, &keeps, &cfg.xlu, &mut notes);
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
            for (kind, segs, pre, dls) in [("opa", &opa_segs, &opa_pre, &shape.opa), ("xlu", &xlu_segs, &xlu_pre, &shape.xlu)] {
                let draw = run_dls(segs, pre, dls);
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
    let in_scenes = |f: &&oot_core::symbols::AssetFile| f.xml_path.starts_with(&scenes_xml);
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

// ---------------------------------------------------------------------------------------------
// Draw configs: a small interpreter for the C in z_scene_table.c
// ---------------------------------------------------------------------------------------------

mod drawcfg {
    //! Runs `Scene_DrawConfig*` functions from the decomp source well enough to learn what
    //! they bind to segments 8..0xD and which colours they set before the rooms are drawn.
    //! Unknown runtime state evaluates to 0 and is reported.

    use std::collections::{BTreeMap, BTreeSet, HashMap};

    use serde_json::{Value, json};

    #[derive(Clone, Debug, PartialEq)]
    enum K {
        Id,
        Num(f64, bool),
        Str,
        P,
    }

    #[derive(Clone, Debug)]
    struct Tok {
        k: K,
        s: String,
    }

    fn lex(src: &str) -> Vec<Tok> {
        let b = src.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        let mut line_start = true;
        const P3: [&str; 2] = ["<<=", ">>="];
        const P2: [&str; 19] = ["->", "++", "--", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^="];
        while i < b.len() {
            let c = b[i];
            if c == b'\n' {
                line_start = true;
                i += 1;
                continue;
            }
            if c.is_ascii_whitespace() {
                i += 1;
                continue;
            }
            if c == b'#' && line_start {
                while i < b.len() && b[i] != b'\n' {
                    if b[i] == b'\\' && b.get(i + 1) == Some(&b'\n') {
                        i += 1;
                    } else if b[i] == b'\\' && b.get(i + 1) == Some(&b'\r') {
                        i += 2;
                    }
                    i += 1;
                }
                continue;
            }
            line_start = false;
            if c == b'/' && b.get(i + 1) == Some(&b'/') {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            if c == b'/' && b.get(i + 1) == Some(&b'*') {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
                continue;
            }
            if c == b'"' || c == b'\'' {
                let q = c;
                let st = i;
                i += 1;
                while i < b.len() && b[i] != q {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
                let s = String::from_utf8_lossy(&b[st..i.min(b.len())]).to_string();
                out.push(if q == b'"' { Tok { k: K::Str, s } } else { Tok { k: K::Num(0.0, false), s } });
                continue;
            }
            if c.is_ascii_digit() || (c == b'.' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
                let st = i;
                let (v, f);
                if c == b'0' && matches!(b.get(i + 1), Some(b'x') | Some(b'X')) {
                    i += 2;
                    let hs = i;
                    while i < b.len() && b[i].is_ascii_hexdigit() {
                        i += 1;
                    }
                    v = i64::from_str_radix(std::str::from_utf8(&b[hs..i]).unwrap_or("0"), 16).unwrap_or(0) as f64;
                    f = false;
                } else {
                    let mut is_f = false;
                    while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.' || ((b[i] == b'e' || b[i] == b'E') && i > st)) {
                        if b[i] == b'.' || b[i] == b'e' || b[i] == b'E' {
                            is_f = true;
                            if (b[i] == b'e' || b[i] == b'E') && matches!(b.get(i + 1), Some(b'+') | Some(b'-')) {
                                i += 1;
                            }
                        }
                        i += 1;
                    }
                    let txt = std::str::from_utf8(&b[st..i]).unwrap_or("0");
                    v = txt.trim_end_matches('.').parse::<f64>().unwrap_or(0.0);
                    f = is_f;
                }
                let mut fl = f;
                while i < b.len() && matches!(b[i], b'f' | b'F' | b'u' | b'U' | b'l' | b'L') {
                    if b[i] == b'f' || b[i] == b'F' {
                        fl = true;
                    }
                    i += 1;
                }
                out.push(Tok { k: K::Num(v, fl), s: String::from_utf8_lossy(&b[st..i]).to_string() });
                continue;
            }
            if c.is_ascii_alphabetic() || c == b'_' {
                let st = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                out.push(Tok { k: K::Id, s: String::from_utf8_lossy(&b[st..i]).to_string() });
                continue;
            }
            let rest = &src[i..];
            let p = P3.iter().chain(P2.iter()).find(|p| rest.starts_with(**p)).map(|p| p.to_string()).unwrap_or_else(|| (c as char).to_string());
            i += p.len();
            out.push(Tok { k: K::P, s: p });
        }
        out
    }

    #[derive(Clone, Debug)]
    pub enum Expr {
        Num(f64, bool),
        Name(String),
        Str,
        Member(Box<Expr>, String),
        Index(Box<Expr>, Box<Expr>),
        Call(String, Vec<Expr>, String),
        Un(String, Box<Expr>),
        Bin(String, Box<Expr>, Box<Expr>),
        Cast(String, Box<Expr>),
        Tern(Box<Expr>, Box<Expr>, Box<Expr>),
        PostInc(Box<Expr>, i64),
        PreInc(Box<Expr>, i64),
        Assign(String, Box<Expr>, Box<Expr>),
        Comma(Vec<Expr>),
    }

    #[derive(Clone, Debug)]
    enum Stmt {
        Expr(Expr),
        Decl(String, Vec<(String, Option<Expr>)>),
        If(Expr, Box<Stmt>, Option<Box<Stmt>>),
        Block(Vec<Stmt>),
        Skip,
    }

    const TYPES: [&str; 26] = [
        "u8", "s8", "u16", "s16", "u32", "s32", "u64", "s64", "f32", "f64", "Gfx", "void", "Vec3f", "Vec3s", "Mtx", "Player", "Camera", "int", "char",
        "unsigned", "signed", "float", "double", "const", "static", "volatile",
    ];

    fn is_type(s: &str) -> bool {
        TYPES.contains(&s)
    }

    struct Parser<'a> {
        t: &'a [Tok],
        i: usize,
    }

    impl<'a> Parser<'a> {
        fn peek(&self) -> &str {
            self.t.get(self.i).map(|t| t.s.as_str()).unwrap_or("")
        }
        fn peek_at(&self, k: usize) -> &str {
            self.t.get(self.i + k).map(|t| t.s.as_str()).unwrap_or("")
        }
        fn eof(&self) -> bool {
            self.i >= self.t.len()
        }
        fn next(&mut self) -> Option<&'a Tok> {
            let t = self.t.get(self.i);
            self.i += 1;
            t
        }
        fn eat(&mut self, s: &str) -> bool {
            if self.peek() == s {
                self.i += 1;
                true
            } else {
                false
            }
        }
        fn skip_balanced(&mut self, open: &str, close: &str) {
            if !self.eat(open) {
                return;
            }
            let mut depth = 1;
            while !self.eof() && depth > 0 {
                let s = self.peek().to_string();
                if s == open {
                    depth += 1;
                } else if s == close {
                    depth -= 1;
                }
                self.i += 1;
            }
        }
        fn text(&self, a: usize, b: usize) -> String {
            let mut s = String::new();
            let mut prev_word = false;
            for t in &self.t[a..b.min(self.t.len())] {
                let word = !matches!(t.k, K::P);
                if word && prev_word {
                    s.push(' ');
                }
                s.push_str(&t.s);
                if t.s == "," {
                    s.push(' ');
                }
                prev_word = word;
            }
            s
        }

        fn stmts_until_eof(&mut self) -> Vec<Stmt> {
            let mut v = Vec::new();
            while !self.eof() {
                let before = self.i;
                v.push(self.stmt());
                if self.i == before {
                    self.i += 1;
                }
            }
            v
        }

        fn stmt(&mut self) -> Stmt {
            let s = self.peek().to_string();
            match s.as_str() {
                "{" => {
                    self.i += 1;
                    let mut v = Vec::new();
                    while !self.eof() && self.peek() != "}" {
                        let before = self.i;
                        v.push(self.stmt());
                        if self.i == before {
                            self.i += 1;
                        }
                    }
                    self.eat("}");
                    Stmt::Block(v)
                }
                ";" => {
                    self.i += 1;
                    Stmt::Skip
                }
                "if" => {
                    self.i += 1;
                    self.eat("(");
                    let c = self.expr();
                    self.eat(")");
                    let a = self.stmt();
                    let b = if self.eat("else") { Some(Box::new(self.stmt())) } else { None };
                    Stmt::If(c, Box::new(a), b)
                }
                "switch" | "while" | "for" => {
                    self.i += 1;
                    self.skip_balanced("(", ")");
                    let _ = self.stmt();
                    Stmt::Skip
                }
                "do" => {
                    self.i += 1;
                    let _ = self.stmt();
                    self.eat("while");
                    self.skip_balanced("(", ")");
                    self.eat(";");
                    Stmt::Skip
                }
                "case" | "default" => {
                    while !self.eof() && self.peek() != ":" {
                        self.i += 1;
                    }
                    self.eat(":");
                    Stmt::Skip
                }
                "return" | "break" | "continue" | "goto" => {
                    while !self.eof() && self.peek() != ";" {
                        self.i += 1;
                    }
                    self.eat(";");
                    Stmt::Skip
                }
                _ if is_type(&s) && !(self.peek_at(1) == "(" && s != "static") => self.decl(),
                _ => {
                    let e = self.expr();
                    self.eat(";");
                    Stmt::Expr(e)
                }
            }
        }

        fn decl(&mut self) -> Stmt {
            let mut ty = String::new();
            while is_type(self.peek()) || self.peek() == "*" {
                let s = self.peek().to_string();
                if !matches!(s.as_str(), "static" | "const" | "volatile" | "*" | "unsigned" | "signed") {
                    ty = s;
                }
                self.i += 1;
            }
            let mut vars = Vec::new();
            loop {
                while self.eat("*") {}
                let Some(name) = self.next().map(|t| t.s.clone()) else { break };
                while self.peek() == "[" {
                    self.skip_balanced("[", "]");
                }
                let init = if self.eat("=") {
                    if self.peek() == "{" {
                        self.skip_balanced("{", "}");
                        None
                    } else {
                        Some(self.assign())
                    }
                } else {
                    None
                };
                vars.push((name, init));
                if !self.eat(",") {
                    break;
                }
            }
            self.eat(";");
            Stmt::Decl(ty, vars)
        }

        fn expr(&mut self) -> Expr {
            let first = self.assign();
            if self.peek() != "," {
                return first;
            }
            let mut v = vec![first];
            while self.eat(",") {
                v.push(self.assign());
            }
            Expr::Comma(v)
        }

        fn assign(&mut self) -> Expr {
            let lhs = self.tern();
            let op = self.peek().to_string();
            if matches!(op.as_str(), "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=") {
                self.i += 1;
                let rhs = self.assign();
                return Expr::Assign(op, Box::new(lhs), Box::new(rhs));
            }
            lhs
        }

        fn tern(&mut self) -> Expr {
            let c = self.binary(1);
            if self.eat("?") {
                let a = self.expr();
                self.eat(":");
                let b = self.tern();
                return Expr::Tern(Box::new(c), Box::new(a), Box::new(b));
            }
            c
        }

        fn prec(op: &str) -> u8 {
            match op {
                "||" => 1,
                "&&" => 2,
                "|" => 3,
                "^" => 4,
                "&" => 5,
                "==" | "!=" => 6,
                "<" | ">" | "<=" | ">=" => 7,
                "<<" | ">>" => 8,
                "+" | "-" => 9,
                "*" | "/" | "%" => 10,
                _ => 0,
            }
        }

        fn binary(&mut self, min: u8) -> Expr {
            let mut lhs = self.unary();
            loop {
                let op = self.peek().to_string();
                let p = Self::prec(&op);
                if p == 0 || p < min || self.t.get(self.i).is_some_and(|t| t.k != K::P) {
                    break;
                }
                self.i += 1;
                let rhs = self.binary(p + 1);
                lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
            }
            lhs
        }

        fn is_cast(&self) -> bool {
            if self.peek() != "(" || !is_type(self.peek_at(1)) {
                return false;
            }
            let mut k = 1;
            while is_type(self.peek_at(k)) || self.peek_at(k) == "*" {
                k += 1;
            }
            self.peek_at(k) == ")"
        }

        fn unary(&mut self) -> Expr {
            let s = self.peek().to_string();
            match s.as_str() {
                "-" | "!" | "~" | "+" | "&" | "*" if self.t.get(self.i).is_some_and(|t| t.k == K::P) => {
                    self.i += 1;
                    let e = self.unary();
                    Expr::Un(s, Box::new(e))
                }
                "++" | "--" => {
                    self.i += 1;
                    let e = self.unary();
                    Expr::PreInc(Box::new(e), if s == "++" { 1 } else { -1 })
                }
                "sizeof" => {
                    self.i += 1;
                    if self.peek() == "(" {
                        self.skip_balanced("(", ")");
                    } else {
                        let _ = self.unary();
                    }
                    Expr::Num(8.0, false)
                }
                "(" if self.is_cast() => {
                    self.i += 1;
                    let mut ty = String::new();
                    while self.peek() != ")" && !self.eof() {
                        let t = self.peek().to_string();
                        if t != "*" && t != "const" {
                            ty = t;
                        }
                        self.i += 1;
                    }
                    self.eat(")");
                    let e = self.unary();
                    Expr::Cast(ty, Box::new(e))
                }
                _ => self.postfix(),
            }
        }

        fn postfix(&mut self) -> Expr {
            let start = self.i;
            let mut e = self.primary();
            loop {
                match self.peek() {
                    "(" => {
                        self.i += 1;
                        let mut args = Vec::new();
                        while !self.eof() && self.peek() != ")" {
                            let before = self.i;
                            args.push(self.assign());
                            if !self.eat(",") && self.peek() != ")" && self.i == before {
                                self.i += 1;
                            }
                        }
                        self.eat(")");
                        let name = match &e {
                            Expr::Name(n) => n.clone(),
                            _ => "?".into(),
                        };
                        e = Expr::Call(name, args, self.text(start, self.i));
                    }
                    "[" => {
                        self.i += 1;
                        let idx = self.expr();
                        self.eat("]");
                        e = Expr::Index(Box::new(e), Box::new(idx));
                    }
                    "." | "->" => {
                        self.i += 1;
                        let f = self.next().map(|t| t.s.clone()).unwrap_or_default();
                        e = Expr::Member(Box::new(e), f);
                    }
                    "++" => {
                        self.i += 1;
                        e = Expr::PostInc(Box::new(e), 1);
                    }
                    "--" => {
                        self.i += 1;
                        e = Expr::PostInc(Box::new(e), -1);
                    }
                    _ => break,
                }
            }
            e
        }

        fn primary(&mut self) -> Expr {
            let Some(t) = self.next() else { return Expr::Num(0.0, false) };
            match &t.k {
                K::Num(v, f) => Expr::Num(*v, *f),
                K::Id => Expr::Name(t.s.clone()),
                K::Str => {
                    while self.t.get(self.i).is_some_and(|t| t.k == K::Str) {
                        self.i += 1;
                    }
                    Expr::Str
                }
                K::P if t.s == "(" => {
                    let e = self.expr();
                    self.eat(")");
                    e
                }
                K::P => Expr::Num(0.0, false),
            }
        }
    }

    /// A static array: identifier elements (`void* x[] = { a, b }`) or Gfx macros.
    pub struct Array {
        pub names: Vec<String>,
        gfx: Vec<Expr>,
        is_gfx: bool,
    }

    pub struct Program {
        pub arrays: HashMap<String, Array>,
        funcs: HashMap<String, Vec<Stmt>>,
    }

    impl Program {
        pub fn parse(src: &str) -> Program {
            let toks = lex(src);
            let mut arrays = HashMap::new();
            let mut funcs = HashMap::new();
            let mut i = 0;
            let mut buf_start = 0;
            while i < toks.len() {
                let s = toks[i].s.as_str();
                if s == ";" {
                    i += 1;
                    buf_start = i;
                    continue;
                }
                if s != "{" {
                    i += 1;
                    continue;
                }
                // Find the matching brace.
                let mut depth = 0;
                let mut j = i;
                while j < toks.len() {
                    match toks[j].s.as_str() {
                        "{" => depth += 1,
                        "}" => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                let head = &toks[buf_start..i];
                let body = &toks[(i + 1).min(toks.len())..j.min(toks.len())];
                if head.last().is_some_and(|t| t.s == "=") {
                    // Array initializer: name is the identifier before the first '['.
                    let name = head.iter().position(|t| t.s == "[").and_then(|k| k.checked_sub(1)).map(|k| head[k].s.clone());
                    let ty = head.iter().find(|t| t.k == K::Id && t.s != "static").map(|t| t.s.clone()).unwrap_or_default();
                    let is_ptr = head.iter().any(|t| t.s == "*");
                    if let Some(name) = name {
                        if ty == "Gfx" && !is_ptr {
                            let mut p = Parser { t: body, i: 0 };
                            let mut gfx = Vec::new();
                            while !p.eof() {
                                let before = p.i;
                                gfx.push(p.assign());
                                p.eat(",");
                                if p.i == before {
                                    p.i += 1;
                                }
                            }
                            arrays.insert(name, Array { names: Vec::new(), gfx, is_gfx: true });
                        } else if ty == "void" && is_ptr {
                            let names = body.iter().filter(|t| t.k == K::Id).map(|t| t.s.clone()).collect();
                            arrays.insert(name, Array { names, gfx: Vec::new(), is_gfx: false });
                        }
                    }
                    i = j + 1;
                    if toks.get(i).is_some_and(|t| t.s == ";") {
                        i += 1;
                    }
                } else if head.last().is_some_and(|t| t.s == ")") {
                    if let Some(k) = head.iter().position(|t| t.s == "(")
                        && k > 0
                    {
                        let name = head[k - 1].s.clone();
                        let mut p = Parser { t: body, i: 0 };
                        funcs.insert(name, p.stmts_until_eof());
                    }
                    i = j + 1;
                } else {
                    i = j + 1;
                }
                buf_start = i;
            }
            Program { arrays, funcs }
        }

        pub fn run(&self, func: &str, scene_id: i64, scene_ids: &HashMap<String, i64>) -> Output {
            let mut m = Machine {
                prog: self,
                scene_id,
                scene_ids,
                vars: HashMap::new(),
                dls: Vec::new(),
                opa: RawBuf::default(),
                xlu: RawBuf::default(),
                assumed: BTreeSet::new(),
                errors: Vec::new(),
                depth: 0,
            };
            match self.funcs.get(func) {
                Some(body) => {
                    for s in body {
                        m.exec(s);
                    }
                }
                None => m.errors.push(format!("function {func} not found in z_scene_table.c")),
            }
            let opa = m.finish(&m.opa);
            let xlu = m.finish(&m.xlu);
            Output { opa, xlu, assumed: m.assumed.into_iter().collect(), errors: m.errors }
        }
    }

    #[derive(Clone, Debug)]
    pub enum Cmd {
        Raw(u32, u32),
        CallSym(String),
    }

    #[derive(Clone, Debug)]
    pub enum Bind {
        Sym(String),
        Dl(Vec<Cmd>),
        Mtx,
        Unknown(String),
    }

    #[derive(Clone, Debug)]
    pub struct Binding {
        pub bind: Bind,
        pub source: String,
        pub variants: Vec<String>,
    }

    #[derive(Default, Clone, Debug)]
    pub struct Buf {
        pub segments: BTreeMap<u8, Binding>,
        pub pre: Vec<Cmd>,
    }

    fn cmd_json(c: &Cmd) -> Value {
        match c {
            Cmd::Raw(a, b) => json!(format!("{a:08X} {b:08X}")),
            Cmd::CallSym(s) => json!(format!("gSPDisplayList({s})")),
        }
    }

    impl Buf {
        pub fn to_json(&self) -> Value {
            let segs: BTreeMap<String, Value> = self
                .segments
                .iter()
                .map(|(s, b)| {
                    let v = match &b.bind {
                        Bind::Sym(n) => json!({ "kind": "pointer", "symbol": n, "variants": b.variants, "c_expr": b.source }),
                        Bind::Dl(c) => json!({ "kind": "display_list", "commands": c.iter().map(cmd_json).collect::<Vec<_>>(), "c_expr": b.source }),
                        Bind::Mtx => json!({ "kind": "matrix", "note": "runtime matrix; bound to identity", "c_expr": b.source }),
                        Bind::Unknown(w) => json!({ "kind": "unresolved", "why": w, "c_expr": b.source }),
                    };
                    (format!("0x{s:02X}"), v)
                })
                .collect();
            json!({ "segments": segs, "pre_commands": self.pre.iter().map(cmd_json).collect::<Vec<_>>() })
        }
    }

    pub struct Output {
        pub opa: Buf,
        pub xlu: Buf,
        pub assumed: Vec<String>,
        pub errors: Vec<String>,
    }

    #[derive(Clone, Debug)]
    enum Val {
        Num(f64, bool),
        Name(String),
        Elem(String, String),
        Arr(String),
        Dl(usize, usize),
        Mtx,
    }

    #[derive(Clone, Debug)]
    enum RawBind {
        Sym(String, Vec<String>),
        DlRef(usize, usize),
        Mtx,
        Unknown(String),
    }

    #[derive(Default)]
    struct RawBuf {
        segments: BTreeMap<u8, (RawBind, String)>,
        pre: Vec<Cmd>,
    }

    enum Target {
        Opa,
        Xlu,
        Dl(usize, usize),
        None,
    }

    struct Machine<'a> {
        prog: &'a Program,
        scene_id: i64,
        scene_ids: &'a HashMap<String, i64>,
        vars: HashMap<String, (String, Val)>,
        dls: Vec<Vec<Option<Cmd>>>,
        opa: RawBuf,
        xlu: RawBuf,
        assumed: BTreeSet<String>,
        errors: Vec<String>,
        depth: usize,
    }

    fn wrap(ty: &str, v: f64, f: bool) -> (f64, bool) {
        let i = if v.is_finite() { v.trunc() as i64 } else { 0 };
        match ty {
            "u8" => (i as u8 as f64, false),
            "s8" => (i as i8 as f64, false),
            "u16" => (i as u16 as f64, false),
            "s16" => (i as i16 as f64, false),
            "u32" | "unsigned" => (i as u32 as f64, false),
            "s32" | "int" | "signed" => (i as i32 as f64, false),
            "f32" | "f64" | "float" | "double" => (v, true),
            "void" => (0.0, false),
            _ => (v, f),
        }
    }

    fn rgba(r: f64, g: f64, b: f64, a: f64) -> u32 {
        let c = |v: f64| (v.trunc() as i64 as u32) & 0xFF;
        (c(r) << 24) | (c(g) << 16) | (c(b) << 8) | c(a)
    }

    fn tile_size(tile: f64, x: f64, y: f64, w: f64, h: f64) -> Cmd {
        let wrap = |v: f64| ((v.trunc() as i64) as u32) % (512 << 2);
        let (x, y) = (wrap(x), wrap(y));
        let (w, h) = (w.trunc() as i64, h.trunc() as i64);
        let lrs = (x as i64 + ((w - 1) << 2)) as u32 & 0xFFF;
        let lrt = (y as i64 + ((h - 1) << 2)) as u32 & 0xFFF;
        Cmd::Raw(0xF200_0000 | ((x & 0xFFF) << 12) | (y & 0xFFF), ((tile as u32 & 7) << 24) | (lrs << 12) | lrt)
    }

    impl Machine<'_> {
        fn finish(&self, b: &RawBuf) -> Buf {
            let mut out = Buf { segments: BTreeMap::new(), pre: b.pre.clone() };
            for (&s, (rb, src)) in &b.segments {
                let (bind, variants) = match rb {
                    RawBind::Sym(n, v) => (Bind::Sym(n.clone()), v.clone()),
                    RawBind::DlRef(id, pos) => {
                        let mut cmds = Vec::new();
                        for c in self.dls.get(*id).map(|d| &d[(*pos).min(d.len())..]).unwrap_or(&[]) {
                            match c {
                                Some(c) => {
                                    let end = matches!(c, Cmd::Raw(w0, _) if w0 >> 24 == 0xDF);
                                    cmds.push(c.clone());
                                    if end {
                                        break;
                                    }
                                }
                                None => break,
                            }
                        }
                        (Bind::Dl(cmds), Vec::new())
                    }
                    RawBind::Mtx => (Bind::Mtx, Vec::new()),
                    RawBind::Unknown(w) => (Bind::Unknown(w.clone()), Vec::new()),
                };
                out.segments.insert(s, Binding { bind, source: src.clone(), variants });
            }
            out
        }

        fn exec(&mut self, s: &Stmt) {
            self.depth += 1;
            if self.depth > 200 {
                self.depth -= 1;
                return;
            }
            match s {
                Stmt::Expr(e) => {
                    let _ = self.eval(e);
                }
                Stmt::Decl(ty, vars) => {
                    for (n, init) in vars {
                        let v = match init {
                            Some(e) => self.eval(e),
                            None => Val::Num(0.0, false),
                        };
                        let v = match v {
                            Val::Num(x, f) => {
                                let (x, f) = wrap(ty, x, f);
                                Val::Num(x, f)
                            }
                            other => other,
                        };
                        self.vars.insert(n.clone(), (ty.clone(), v));
                    }
                }
                Stmt::If(c, a, b) => {
                    let cv = self.eval(c);
                    if self.num(&cv) != 0.0 {
                        self.exec(a);
                    } else if let Some(b) = b {
                        self.exec(b);
                    }
                }
                Stmt::Block(v) => {
                    for s in v {
                        self.exec(s);
                    }
                }
                Stmt::Skip => {}
            }
            self.depth -= 1;
        }

        fn known_name(&self, n: &str) -> Option<f64> {
            if let Some(&id) = self.scene_ids.get(n) {
                return Some(id as f64);
            }
            Some(match n {
                "play.sceneId" | "play->sceneId" => self.scene_id as f64,
                "true" | "LINK_IS_CHILD" | "IS_DAY" | "LINK_AGE_CHILD" | "gSaveContext.linkAge" => 1.0,
                "false" | "NULL" | "G_TX_RENDERTILE" | "LINK_IS_ADULT" | "IS_NIGHT" | "IS_CUTSCENE_LAYER" | "LINK_AGE_ADULT" | "gSaveContext.nightFlag"
                | "gSaveContext.sceneLayer" | "play.gameplayFrames" | "gSaveContext.cutsceneIndex" => 0.0,
                "gSaveContext.dayTime" | "gSaveContext.skyboxTime" => 0x8000 as f64,
                "M_PI" => std::f64::consts::PI,
                _ => return None,
            })
        }

        fn num(&mut self, v: &Val) -> f64 {
            match v {
                Val::Num(x, _) => *x,
                Val::Name(n) | Val::Elem(n, _) => match self.known_name(n) {
                    Some(x) => x,
                    None => {
                        self.assumed.insert(n.clone());
                        0.0
                    }
                },
                Val::Arr(_) | Val::Dl(..) | Val::Mtx => 0.0,
            }
        }

        fn is_float(&self, v: &Val) -> bool {
            matches!(v, Val::Num(_, true))
        }

        fn lookup(&self, name: &str) -> Option<Val> {
            self.vars.get(name).map(|v| v.1.clone())
        }

        /// Path string for an lvalue / name expression.
        fn path(&mut self, e: &Expr) -> Option<String> {
            match e {
                Expr::Name(n) => Some(n.clone()),
                Expr::Member(b, f) => Some(format!("{}.{f}", self.path(b)?)),
                Expr::Index(b, i) => {
                    let base = self.path(b)?;
                    let iv = self.eval(i);
                    let idx = self.num(&iv);
                    Some(format!("{base}[{idx}]"))
                }
                Expr::Un(op, x) if op == "*" || op == "&" => self.path(x),
                _ => None,
            }
        }

        fn store(&mut self, path: &str, v: Val) {
            let ty = self.vars.get(path).map(|x| x.0.clone()).unwrap_or_default();
            let v = match v {
                Val::Num(x, f) => {
                    let (x, f) = wrap(&ty, x, f);
                    Val::Num(x, f)
                }
                o => o,
            };
            self.vars.insert(path.to_string(), (ty, v));
        }

        fn new_dl(&mut self, cmds: Vec<Cmd>) -> Val {
            self.dls.push(cmds.into_iter().map(Some).collect());
            Val::Dl(self.dls.len() - 1, 0)
        }

        fn target(&mut self, e: &Expr) -> Target {
            let (inner, inc) = match e {
                Expr::PostInc(x, n) => (&**x, *n),
                _ => (e, 0),
            };
            let Some(p) = self.path(inner) else { return Target::None };
            match p.as_str() {
                "POLY_OPA_DISP" => Target::Opa,
                "POLY_XLU_DISP" => Target::Xlu,
                _ => match self.lookup(&p) {
                    Some(Val::Dl(id, pos)) => {
                        if inc != 0 {
                            let ty = self.vars.get(&p).map(|x| x.0.clone()).unwrap_or_default();
                            self.vars.insert(p, (ty, Val::Dl(id, (pos as i64 + inc).max(0) as usize)));
                        }
                        Target::Dl(id, pos)
                    }
                    _ => Target::None,
                },
            }
        }

        fn emit(&mut self, target: &Target, c: Cmd) {
            // Syncs and ENDDL only matter inside allocated display lists.
            let is_sync = matches!(c, Cmd::Raw(w0, _) if matches!(w0 >> 24, 0xDF | 0xE7 | 0xE8));
            if is_sync && matches!(target, Target::Opa | Target::Xlu) {
                return;
            }
            match target {
                Target::Opa => self.opa.pre.push(c),
                Target::Xlu => self.xlu.pre.push(c),
                Target::Dl(id, pos) => {
                    if let Some(d) = self.dls.get_mut(*id) {
                        if d.len() <= *pos {
                            d.resize(*pos + 1, None);
                        }
                        d[*pos] = Some(c);
                    }
                }
                Target::None => {}
            }
        }

        /// GBI macro `name` (without the g/gs prefix) with the target already resolved.
        fn gbi(&mut self, name: &str, target: Target, args: &[Expr], src: &str) {
            let n = |m: &mut Self, k: usize| -> f64 {
                match args.get(k) {
                    Some(e) => {
                        let v = m.eval(e);
                        m.num(&v)
                    }
                    None => 0.0,
                }
            };
            match name {
                "SPSegment" => {
                    let seg = n(self, 0) as u8;
                    let v = args.get(1).map(|e| self.eval(e)).unwrap_or(Val::Num(0.0, false));
                    let bind = match v {
                        Val::Name(s) if self.known_name(&s).is_none() && !s.contains('.') => RawBind::Sym(s, Vec::new()),
                        Val::Elem(s, arr) => {
                            let variants = self.prog.arrays.get(&arr).map(|a| a.names.clone()).unwrap_or_default();
                            RawBind::Sym(s, variants)
                        }
                        Val::Dl(id, pos) => RawBind::DlRef(id, pos),
                        Val::Arr(a) => RawBind::Sym(a, Vec::new()),
                        Val::Mtx => RawBind::Mtx,
                        other => RawBind::Unknown(format!("{other:?}")),
                    };
                    let src = src.to_string();
                    match target {
                        Target::Opa => {
                            self.opa.segments.insert(seg, (bind, src));
                        }
                        Target::Xlu => {
                            self.xlu.segments.insert(seg, (bind, src));
                        }
                        _ => {}
                    }
                }
                "DPSetEnvColor" => {
                    let c = rgba(n(self, 0), n(self, 1), n(self, 2), n(self, 3));
                    self.emit(&target, Cmd::Raw(0xFB00_0000, c));
                }
                "DPSetPrimColor" => {
                    let (m, l) = (n(self, 0) as u32 & 0xFF, n(self, 1) as u32 & 0xFF);
                    let c = rgba(n(self, 2), n(self, 3), n(self, 4), n(self, 5));
                    self.emit(&target, Cmd::Raw(0xFA00_0000 | (m << 8) | l, c));
                }
                "DPSetTileSize" => {
                    let (uls, ult, lrs, lrt) = (n(self, 1) as u32, n(self, 2) as u32, n(self, 3) as u32, n(self, 4) as u32);
                    let tile = n(self, 0) as u32 & 7;
                    self.emit(&target, Cmd::Raw(0xF200_0000 | ((uls & 0xFFF) << 12) | (ult & 0xFFF), (tile << 24) | ((lrs & 0xFFF) << 12) | (lrt & 0xFFF)));
                }
                "DPPipeSync" => self.emit(&target, Cmd::Raw(0xE700_0000, 0)),
                "DPTileSync" => self.emit(&target, Cmd::Raw(0xE800_0000, 0)),
                "SPEndDisplayList" => self.emit(&target, Cmd::Raw(0xDF00_0000, 0)),
                "SPDisplayList" => {
                    let v = args.first().map(|e| self.eval(e)).unwrap_or(Val::Num(0.0, false));
                    match v {
                        Val::Arr(a) if self.prog.arrays.get(&a).is_some_and(|x| x.is_gfx) && matches!(target, Target::Opa | Target::Xlu) => {
                            let items = self.prog.arrays[&a].gfx.clone();
                            let is_opa = matches!(target, Target::Opa);
                            for it in &items {
                                if let Expr::Call(cn, cargs, csrc) = it
                                    && let Some(short) = cn.strip_prefix("gs")
                                {
                                    self.gbi(short, if is_opa { Target::Opa } else { Target::Xlu }, cargs, csrc);
                                }
                            }
                        }
                        Val::Name(s) | Val::Elem(s, _) => self.emit(&target, Cmd::CallSym(s)),
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        fn call(&mut self, name: &str, args: &[Expr], src: &str) -> Val {
            if let Some(short) = name.strip_prefix('g').filter(|s| s.starts_with("SP") || s.starts_with("DP")) {
                let target = match args.first() {
                    Some(t) => self.target(t),
                    None => Target::None,
                };
                let src_arg = if short == "SPSegment" {
                    // Keep the bound expression's text for the report.
                    src.split_once(',').and_then(|(_, r)| r.split_once(',')).map(|(_, r)| r.trim().strip_suffix(')').unwrap_or(r).trim().to_string()).unwrap_or_default()
                } else {
                    src.to_string()
                };
                self.gbi(short, target, &args[1.min(args.len())..], &src_arg);
                return Val::Num(0.0, false);
            }
            let a = |m: &mut Self, k: usize| -> f64 {
                match args.get(k) {
                    Some(e) => {
                        let v = m.eval(e);
                        m.num(&v)
                    }
                    None => 0.0,
                }
            };
            match name {
                "SEGMENTED_TO_VIRTUAL" | "VIRTUAL_TO_PHYSICAL" | "OS_K0_TO_PHYSICAL" | "OS_PHYSICAL_TO_K0" => {
                    args.first().map(|e| self.eval(e)).unwrap_or(Val::Num(0.0, false))
                }
                "Graph_Alloc" => self.new_dl(Vec::new()),
                "Matrix_NewMtx" => Val::Mtx,
                "Gfx_TexScroll" => {
                    let (x, y, w, h) = (a(self, 1), a(self, 2), a(self, 3), a(self, 4));
                    self.new_dl(vec![Cmd::Raw(0xE800_0000, 0), tile_size(0.0, x, y, w, h), Cmd::Raw(0xDF00_0000, 0)])
                }
                "Gfx_TwoTexScroll" | "Gfx_TwoTexScrollEnvColor" | "Gfx_TwoTexScrollPrimColor" => {
                    let v: Vec<f64> = (1..args.len()).map(|k| a(self, k)).collect();
                    let g = |k: usize| v.get(k).copied().unwrap_or(0.0);
                    let mut cmds = vec![
                        Cmd::Raw(0xE800_0000, 0),
                        tile_size(g(0), g(1), g(2), g(3), g(4)),
                        Cmd::Raw(0xE800_0000, 0),
                        tile_size(g(5), g(6), g(7), g(8), g(9)),
                    ];
                    if name == "Gfx_TwoTexScrollEnvColor" {
                        cmds.push(Cmd::Raw(0xFB00_0000, rgba(g(10), g(11), g(12), g(13))));
                    } else if name == "Gfx_TwoTexScrollPrimColor" {
                        cmds.push(Cmd::Raw(0xFA00_0000, rgba(g(10), g(11), g(12), g(13))));
                    }
                    cmds.push(Cmd::Raw(0xDF00_0000, 0));
                    self.new_dl(cmds)
                }
                "Gfx_EnvColor" => {
                    let c = rgba(a(self, 1), a(self, 2), a(self, 3), a(self, 4));
                    self.new_dl(vec![Cmd::Raw(0xFB00_0000, c), Cmd::Raw(0xDF00_0000, 0)])
                }
                "CLOCK_TIME" => {
                    let (h, m) = (a(self, 0), a(self, 1));
                    Val::Num(((h * 60.0 + m) * 65536.0 / 1440.0).trunc(), false)
                }
                "coss" | "sins" => {
                    let x = a(self, 0);
                    let r = x * std::f64::consts::TAU / 65536.0;
                    let f = if name == "coss" { r.cos() } else { r.sin() };
                    Val::Num((f * 32767.0).trunc(), false)
                }
                "Math_CosS" | "Math_SinS" => {
                    let r = a(self, 0) * std::f64::consts::TAU / 65536.0;
                    Val::Num(if name == "Math_CosS" { r.cos() } else { r.sin() }, true)
                }
                "sinf" | "cosf" | "Math_SinF" | "Math_CosF" => {
                    let r = a(self, 0);
                    Val::Num(if name.contains("in") { r.sin() } else { r.cos() }, true)
                }
                "ABS" | "fabsf" => Val::Num(a(self, 0).abs(), false),
                "OPEN_DISPS" | "CLOSE_DISPS" => Val::Num(0.0, false),
                _ => {
                    for e in args {
                        let _ = self.eval(e);
                    }
                    self.assumed.insert(format!("{name}()"));
                    Val::Num(0.0, false)
                }
            }
        }

        fn eval(&mut self, e: &Expr) -> Val {
            match e {
                Expr::Num(v, f) => Val::Num(*v, *f),
                Expr::Str => Val::Num(0.0, false),
                Expr::Name(n) => {
                    if let Some(v) = self.lookup(n) {
                        return v;
                    }
                    if self.prog.arrays.contains_key(n) {
                        return Val::Arr(n.clone());
                    }
                    Val::Name(n.clone())
                }
                Expr::Member(..) => {
                    let p = self.path(e).unwrap_or_else(|| "?".into());
                    let p = p.replace("->", ".");
                    self.lookup(&p).unwrap_or(Val::Name(p))
                }
                Expr::Index(b, i) => {
                    let bv = self.eval(b);
                    let iv = self.eval(i);
                    let idx = self.num(&iv);
                    match bv {
                        Val::Arr(a) => {
                            let el = self.prog.arrays.get(&a).and_then(|x| x.names.get(idx.max(0.0) as usize).cloned());
                            match el {
                                Some(el) => Val::Elem(el, a),
                                None => Val::Num(0.0, false),
                            }
                        }
                        _ => {
                            let p = self.path(e).unwrap_or_else(|| "?".into());
                            self.lookup(&p).unwrap_or(Val::Name(p))
                        }
                    }
                }
                Expr::Call(n, args, src) => self.call(n, args, src),
                Expr::Un(op, x) => {
                    let v = self.eval(x);
                    match op.as_str() {
                        "&" | "*" | "+" => v,
                        "-" => {
                            let f = self.is_float(&v);
                            Val::Num(-self.num(&v), f)
                        }
                        "!" => Val::Num(if self.num(&v) == 0.0 { 1.0 } else { 0.0 }, false),
                        "~" => Val::Num(!(self.num(&v) as i64) as f64, false),
                        _ => v,
                    }
                }
                Expr::Cast(ty, x) => {
                    let v = self.eval(x);
                    match v {
                        Val::Num(..) | Val::Name(_) | Val::Elem(..) if !matches!(ty.as_str(), "Gfx") => {
                            let f = self.is_float(&v);
                            let n = self.num(&v);
                            let (n, f) = wrap(ty, n, f);
                            Val::Num(n, f)
                        }
                        o => o,
                    }
                }
                Expr::Bin(op, a, b) => {
                    let av = self.eval(a);
                    if op == "&&" || op == "||" {
                        let x = self.num(&av) != 0.0;
                        if (op == "&&" && !x) || (op == "||" && x) {
                            return Val::Num(x as i32 as f64, false);
                        }
                        let bv = self.eval(b);
                        return Val::Num((self.num(&bv) != 0.0) as i32 as f64, false);
                    }
                    let bv = self.eval(b);
                    let f = self.is_float(&av) || self.is_float(&bv);
                    let (x, y) = (self.num(&av), self.num(&bv));
                    let (xi, yi) = (x.trunc() as i64, y.trunc() as i64);
                    let r = match op.as_str() {
                        "+" => x + y,
                        "-" => x - y,
                        "*" => x * y,
                        "/" => {
                            if f {
                                if y == 0.0 { 0.0 } else { x / y }
                            } else if yi == 0 {
                                0.0
                            } else {
                                (xi / yi) as f64
                            }
                        }
                        "%" => {
                            if yi == 0 {
                                0.0
                            } else {
                                (xi % yi) as f64
                            }
                        }
                        "<<" => (xi << (yi & 63)) as f64,
                        ">>" => (xi >> (yi & 63)) as f64,
                        "&" => (xi & yi) as f64,
                        "|" => (xi | yi) as f64,
                        "^" => (xi ^ yi) as f64,
                        "==" => (x == y) as i32 as f64,
                        "!=" => (x != y) as i32 as f64,
                        "<" => (x < y) as i32 as f64,
                        ">" => (x > y) as i32 as f64,
                        "<=" => (x <= y) as i32 as f64,
                        ">=" => (x >= y) as i32 as f64,
                        _ => 0.0,
                    };
                    let is_cmp = matches!(op.as_str(), "==" | "!=" | "<" | ">" | "<=" | ">=" | "<<" | ">>" | "&" | "|" | "^" | "%");
                    Val::Num(r, f && !is_cmp)
                }
                Expr::Tern(c, a, b) => {
                    let cv = self.eval(c);
                    if self.num(&cv) != 0.0 { self.eval(a) } else { self.eval(b) }
                }
                Expr::PostInc(x, n) | Expr::PreInc(x, n) => {
                    let pre = matches!(e, Expr::PreInc(..));
                    let Some(p) = self.path(x) else { return Val::Num(0.0, false) };
                    let cur = self.lookup(&p).unwrap_or(Val::Name(p.clone()));
                    let new = match &cur {
                        Val::Dl(id, pos) => Val::Dl(*id, (*pos as i64 + n).max(0) as usize),
                        other => {
                            let f = self.is_float(other);
                            Val::Num(self.num(other) + *n as f64, f)
                        }
                    };
                    self.store(&p, new.clone());
                    if pre { new } else { cur }
                }
                Expr::Assign(op, l, r) => {
                    let rv = self.eval(r);
                    let Some(p) = self.path(l) else { return rv };
                    let p = p.replace("->", ".");
                    let v = if op == "=" {
                        rv
                    } else {
                        let cur = self.lookup(&p).unwrap_or(Val::Name(p.clone()));
                        let bop = op.trim_end_matches('=').to_string();
                        let f = self.is_float(&cur) || self.is_float(&rv);
                        let (x, y) = (self.num(&cur), self.num(&rv));
                        let (xi, yi) = (x.trunc() as i64, y.trunc() as i64);
                        let r = match bop.as_str() {
                            "+" => x + y,
                            "-" => x - y,
                            "*" => x * y,
                            "/" => {
                                if y == 0.0 {
                                    0.0
                                } else if f {
                                    x / y
                                } else {
                                    (xi / yi) as f64
                                }
                            }
                            "%" => {
                                if yi == 0 {
                                    0.0
                                } else {
                                    (xi % yi) as f64
                                }
                            }
                            "&" => (xi & yi) as f64,
                            "|" => (xi | yi) as f64,
                            "^" => (xi ^ yi) as f64,
                            "<<" => (xi << (yi & 63)) as f64,
                            ">>" => (xi >> (yi & 63)) as f64,
                            _ => y,
                        };
                        Val::Num(r, f)
                    };
                    self.store(&p, v.clone());
                    v
                }
                Expr::Comma(v) => {
                    let mut last = Val::Num(0.0, false);
                    for x in v {
                        last = self.eval(x);
                    }
                    last
                }
            }
        }
    }
}
