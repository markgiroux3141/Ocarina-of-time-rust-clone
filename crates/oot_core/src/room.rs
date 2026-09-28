//! Rooms and the scene's draw state, as `z_room.c` / `z_scene_table.c` / `z_play.c` set it up.
//!
//! - Room files are mapped to segment 3; their header (layer-selected like the scene's)
//!   carries the room shape (`SCENE_CMD_ID_ROOM_SHAPE`): type 0 (normal: OPA/XLU display-list
//!   pairs), 1 (image: one pair plus prerendered backgrounds) or 2 (cullable: pairs with a
//!   bounding sphere, z-sorted and depth-culled every frame by `Room_DrawCullable`).
//! - Segments while drawing: 0 = a RAM image of `code` (some DLs hold raw KSEG0 pointers),
//!   2 = scene, 3 = room, 4 = `gameplay_keep`, 5 = the keep object from
//!   `SCENE_CMD_ID_SPECIAL_FILES`, 8..0xD = whatever the scene's draw config binds.
//! - The draw config (`Scene_DrawConfig*` in `z_scene_table.c`) is run by
//!   [`crate::drawcfg`]. Display lists it allocates (`Gfx_TwoTexScroll` and friends) are
//!   bound as *dynamic* segments: the meshes are built once, and each frame the draw config is
//!   run again to get new tile sizes and colours ([`SceneDraw::segment_values`]).
//!
//! The segment binder here was the extractor's (`oot_extract::scenes`), moved into the core in
//! spike 04 so the game and the extractor share it.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use glam::{Mat4, Vec3};

use crate::csrc::{define_rows, parse_enum};
use crate::drawcfg;
use crate::gbi::{DrawList, Interpreter, Segment, SegmentValues};
use crate::project::Project;
use crate::scene::{self, ActorEntry, Scene, SceneCommand};

// ---------------------------------------------------------------------------------------------
// Decomp tables
// ---------------------------------------------------------------------------------------------

/// A `DEFINE_SCENE` row of `include/tables/scene_table.h`.
#[derive(Debug, Clone)]
pub struct SceneDef {
    pub id: usize,
    pub file: String,
    pub enum_name: String,
    pub draw_config: String,
}

/// The tables a scene load needs, read from the decomp.
pub struct SceneTables {
    pub scenes: Vec<SceneDef>,
    pub scene_ids: HashMap<String, i64>,
    /// Object id → file name ("" for `DEFINE_OBJECT_UNSET`), from `object_table.h`.
    pub objects: Vec<String>,
    /// `SDC_*` → draw config function, from `sSceneDrawConfigs`.
    pub sdc_funcs: HashMap<String, String>,
    pub drawcfg: drawcfg::Program,
}

impl SceneTables {
    pub fn load(decomp: &Path) -> Result<SceneTables> {
        let read = |p: &Path| std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()));
        let tables = decomp.join("include/tables");
        let mut scenes = Vec::new();
        let mut scene_ids = HashMap::new();
        for (i, (_, a)) in define_rows(&read(&tables.join("scene_table.h"))?).into_iter().enumerate() {
            let g = |k: usize| a.get(k).cloned().unwrap_or_default();
            scene_ids.insert(g(2), i as i64);
            scenes.push(SceneDef { id: i, file: g(0), enum_name: g(2), draw_config: g(3) });
        }
        let objects = define_rows(&read(&tables.join("object_table.h"))?)
            .into_iter()
            .map(|(mac, a)| if mac == "DEFINE_OBJECT_UNSET" { String::new() } else { a.first().cloned().unwrap_or_default() })
            .collect();
        let sdc_enum = parse_enum(&read(&decomp.join("include/z64scene.h"))?, "SDC_DEFAULT");
        let program = drawcfg::Program::parse(&read(&decomp.join("src/code/z_scene_table.c"))?);
        let mut sdc_funcs = HashMap::new();
        if let Some(list) = program.arrays.get("sSceneDrawConfigs") {
            for (i, f) in list.names.iter().enumerate() {
                if let Some(sdc) = sdc_enum.get(&(i as i64)) {
                    sdc_funcs.insert(sdc.clone(), f.clone());
                }
            }
        }
        Ok(SceneTables { scenes, scene_ids, objects, sdc_funcs, drawcfg: program })
    }

    pub fn scene(&self, file: &str) -> Option<&SceneDef> {
        self.scenes.iter().find(|s| s.file == file)
    }

    pub fn draw_config_fn(&self, sd: &SceneDef) -> String {
        self.sdc_funcs.get(&sd.draw_config).cloned().unwrap_or_else(|| "Scene_DrawConfigDefault".into())
    }
}

// ---------------------------------------------------------------------------------------------
// Rooms
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Normal,
    Image,
    Cullable,
}

/// One display-list pair of a room shape (`RoomShapeDListsEntry` / `RoomShapeCullableEntry`).
#[derive(Debug, Clone, Copy)]
pub struct ShapeEntry {
    /// Cullable entries: bounding sphere centre and radius.
    pub bounds: Option<([i16; 3], i16)>,
    pub opa: u32,
    pub xlu: u32,
}

#[derive(Debug, Clone)]
pub struct RoomShape {
    pub kind: ShapeKind,
    pub entries: Vec<ShapeEntry>,
}

fn be16(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*d.get(o)?, *d.get(o + 1)?]))
}
fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes([*d.get(o)?, *d.get(o + 1)?, *d.get(o + 2)?, *d.get(o + 3)?]))
}

/// Resolves a segment 2 / 3 address to (file, offset).
fn resolve<'a>(scene: &'a [u8], room: &'a [u8], addr: u32) -> Option<(&'a [u8], usize)> {
    let off = (addr & 0xFF_FFFF) as usize;
    let d = match addr >> 24 {
        2 => scene,
        3 => room,
        _ => return None,
    };
    (off < d.len()).then_some((d, off))
}

impl RoomShape {
    /// `RoomShape*` structs (`z64scene.h`): `{ u8 type, u8 numEntries, pad, entries*, entriesEnd* }`
    /// for types 0 and 2 (entries of 8 / 16 bytes), `{ u8 type, u8 amountType, pad, entry* , ... }`
    /// for type 1.
    pub fn parse(scene: &[u8], room: &[u8], ptr: u32) -> Option<RoomShape> {
        let (d, o) = resolve(scene, room, ptr)?;
        let ty = *d.get(o)?;
        let mut entries = Vec::new();
        let kind = match ty {
            0 | 2 => {
                let n = *d.get(o + 1)? as usize;
                let (ed, eo) = resolve(scene, room, be32(d, o + 4)?)?;
                for i in 0..n {
                    if ty == 0 {
                        let b = eo + i * 8;
                        entries.push(ShapeEntry { bounds: None, opa: be32(ed, b)?, xlu: be32(ed, b + 4)? });
                    } else {
                        let b = eo + i * 16;
                        let c = [be16(ed, b)? as i16, be16(ed, b + 2)? as i16, be16(ed, b + 4)? as i16];
                        entries.push(ShapeEntry { bounds: Some((c, be16(ed, b + 6)? as i16)), opa: be32(ed, b + 8)?, xlu: be32(ed, b + 12)? });
                    }
                }
                if ty == 0 { ShapeKind::Normal } else { ShapeKind::Cullable }
            }
            1 => {
                if let Some((ed, eo)) = resolve(scene, room, be32(d, o + 4)?) {
                    entries.push(ShapeEntry { bounds: None, opa: be32(ed, eo)?, xlu: be32(ed, eo + 4)? });
                }
                ShapeKind::Image
            }
            _ => return None,
        };
        Some(RoomShape { kind, entries })
    }
}

pub struct Room {
    pub index: usize,
    pub file_name: String,
    pub data: Arc<[u8]>,
    pub header: Vec<SceneCommand>,
    pub shape: Option<RoomShape>,
    /// `SCENE_CMD_ID_ROOM_BEHAVIOR`: behaviorType1, behaviorType2.
    pub behavior: [u8; 2],
    pub echo: u8,
    /// `SCENE_CMD_ID_TIME_SETTINGS`: hour, minute, time speed (0xFF = keep).
    pub time: Option<[u8; 3]>,
    pub skybox_disabled: bool,
    pub sun_moon_disabled: bool,
    pub actors: Vec<ActorEntry>,
}

impl Room {
    /// Loads room `index` of `scene` for the scene's layer.
    pub fn load(p: &Project, scene: &Scene, index: usize) -> Result<Room> {
        let r = scene.rooms.get(index).with_context(|| format!("{}: no room {index}", scene.name))?;
        let fi = p.rom.file_containing_vrom(r.vrom_start).with_context(|| format!("room {index}: vrom {:08X} not in dmadata", r.vrom_start))?;
        let file_name = p.rom.name_of(fi).map(|s| s.to_string()).unwrap_or_else(|| format!("file_{fi}"));
        let data = p.rom.file(fi)?;
        // ZAPD HackMode="syotes_room": the file starts with a room shape, not a header.
        let headerless = p.symbols.file(&file_name).and_then(|f| f.symbols.iter().find(|s| s.offset == 0)).and_then(|s| s.attr("HackMode")) == Some("syotes_room");
        let header = if headerless { Vec::new() } else { scene::layer_commands(&data, scene::ROOM_SEGMENT, scene.layer)?.1 };
        let find = |code: u8| header.iter().find(|c| c.code == code).copied();
        let shape_ptr = if headerless { Some(0x0300_0000) } else { find(scene::CMD_ROOM_SHAPE).map(|c| c.data2) };
        let shape = shape_ptr.and_then(|ptr| RoomShape::parse(&scene.file, &data, ptr));
        let behavior = find(scene::CMD_ROOM_BEHAVIOR).map(|c| [c.data1, (c.data2 & 0xFF) as u8]).unwrap_or([0, 0]);
        let echo = find(scene::CMD_ECHO_SETTINGS).map(|c| (c.data2 & 0xFF) as u8).unwrap_or(0);
        let time = find(scene::CMD_TIME_SETTINGS).map(|c| {
            let b = c.data2.to_be_bytes();
            [b[0], b[1], b[2]]
        });
        let (skybox_disabled, sun_moon_disabled) = find(scene::CMD_SKYBOX_DISABLES)
            .map(|c| {
                let b = c.data2.to_be_bytes();
                (b[0] != 0, b[1] != 0)
            })
            .unwrap_or((false, false));
        let actors = find(scene::CMD_ACTOR_LIST).map(|c| scene::actor_entries(&data, (c.data2 & 0xFF_FFFF) as usize, c.data1 as usize)).unwrap_or_default();
        Ok(Room { index, file_name, data, header, shape, behavior, echo, time, skybox_disabled, sun_moon_disabled, actors })
    }
}

// ---------------------------------------------------------------------------------------------
// Segment binding (shared with the extractor)
// ---------------------------------------------------------------------------------------------

/// VRAM start of `code` in gc-eu-mq-dbg. Room DLs sometimes hold raw KSEG0 pointers into
/// `code` (Jabu-Jabu loads `gMtxClear` at 0x8012DB20); the interpreter maps 0x80xxxxxx to
/// segment 0, so segment 0 gets a RAM image of `code`.
const CODE_VRAM: u32 = 0x8001_CE60;
const GMTXCLEAR_VRAM: u32 = 0x8012_DB20;

/// RAM image of `code` from 0x80000000, with ENDDL bytes below it so stray segment-0
/// references stop immediately. None unless gMtxClear is where this ROM layout expects it.
pub fn code_ram_image(p: &Project) -> Option<Arc<[u8]>> {
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

pub fn identity_mtx() -> Arc<[u8]> {
    let mut b = vec![0u8; 64];
    for i in 0..4 {
        b[(i * 4 + i) * 2 + 1] = 1;
    }
    b.into()
}

pub fn cmds_to_bytes(cmds: &[(u32, u32)]) -> Vec<u8> {
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

/// Files bound to fixed segments: `code` (0), `gameplay_keep` (4) and the scene's keep (5).
#[derive(Clone, Default)]
pub struct Keeps {
    pub code: Option<Arc<[u8]>>,
    pub gameplay_keep: Option<Arc<[u8]>>,
    pub scene_keep: Option<Arc<[u8]>>,
}

/// Resolves draw-config symbols and builds interpreter segments for one scene.
pub struct SceneCtx<'a> {
    pub p: &'a Project,
    pub scene_file: String,
    pub scene_buf: Arc<[u8]>,
    pub room_files: Vec<String>,
    pub room_bufs: Vec<Arc<[u8]>>,
}

/// Interpreter segments for one draw buffer, the commands the draw config wrote straight
/// into that buffer, and which segments hold per-frame display lists.
pub struct BufferSegments {
    pub segments: [Option<Segment>; 16],
    pub pre: Vec<(u32, u32)>,
    pub dynamic: u16,
}

impl SceneCtx<'_> {
    /// (file name, offset) of a C symbol: decomp XML first (this scene's files, then all),
    /// then ZAPD's generated `<file>DL_XXXXXX` / `<file>Tex_XXXXXX` names.
    pub fn find_symbol(&self, name: &str) -> Option<(String, u32)> {
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

    pub fn file_buf(&self, file: &str) -> Option<Arc<[u8]>> {
        if file == self.scene_file {
            return Some(self.scene_buf.clone());
        }
        if let Some(i) = self.room_files.iter().position(|r| r == file) {
            return Some(self.room_bufs[i].clone());
        }
        self.p.rom.file_by_name(file).ok()
    }

    /// Segment address of a symbol as seen while drawing a room.
    pub fn seg_addr(&self, name: &str) -> Option<u32> {
        let (file, off) = self.find_symbol(name)?;
        if file == self.scene_file {
            Some(0x0200_0000 | off)
        } else if self.room_files.contains(&file) {
            Some(0x0300_0000 | off)
        } else {
            None
        }
    }

    pub fn materialize(&self, cmds: &[drawcfg::Cmd], notes: &mut BTreeSet<String>) -> Vec<(u32, u32)> {
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

    /// Interpreter segments for one draw buffer: 0 code, 2 scene, 3 room, 4 gameplay_keep,
    /// 5 keep, 8..D draw config.
    pub fn segments(&self, room: usize, keeps: &Keeps, buf: &drawcfg::Buf, notes: &mut BTreeSet<String>) -> BufferSegments {
        let mut segs: [Option<Segment>; 16] = Default::default();
        if let Some(c) = &keeps.code {
            segs[0] = Some(Segment::Data { buf: c.clone(), base: 0 });
        }
        segs[2] = Some(Segment::Data { buf: self.scene_buf.clone(), base: 0 });
        segs[3] = Some(Segment::Data { buf: self.room_bufs[room].clone(), base: 0 });
        if let Some(k) = &keeps.gameplay_keep {
            segs[4] = Some(Segment::Data { buf: k.clone(), base: 0 });
        }
        if let Some(k) = &keeps.scene_keep {
            segs[5] = Some(Segment::Data { buf: k.clone(), base: 0 });
        }
        let mut dynamic = 0u16;
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
                drawcfg::Bind::Dl(cmds) => {
                    dynamic |= 1 << (s & 0xF);
                    Some(Segment::Data { buf: cmds_to_bytes(&self.materialize(cmds, notes)).into(), base: 0 })
                }
                drawcfg::Bind::Mtx => Some(Segment::Data { buf: identity_mtx(), base: 0 }),
                drawcfg::Bind::Unknown(_) => None,
            };
            if seg.is_some() {
                segs[s as usize & 0xF] = seg;
            }
        }
        let pre = self.materialize(&buf.pre, notes);
        BufferSegments { segments: segs, pre, dynamic }
    }
}

/// Runs `dls` the way `Room_Draw*` does for one buffer: the draw config's direct commands,
/// `Gfx_SetupDL_25Opa`/`Xlu` (`sSetupDL[SETUPDL_25]`), then the room's display lists.
pub fn run_dls(segs: &BufferSegments, dls: &[u32]) -> DrawList {
    let mut it = Interpreter::new();
    it.segments = segs.segments.clone();
    it.dynamic_segments = segs.dynamic;
    if !segs.pre.is_empty() {
        it.segments[0xF] = Some(Segment::Data { buf: cmds_to_bytes(&segs.pre).into(), base: 0 });
        it.run(0x0F00_0000);
        it.segments[0xF] = None;
    }
    it.apply_setup_dl_25();
    for &dl in dls {
        if dl != 0 {
            it.run(dl);
        }
    }
    it.draw
}

// ---------------------------------------------------------------------------------------------
// A scene ready to draw
// ---------------------------------------------------------------------------------------------

/// One entry of a room shape, interpreted.
pub struct EntryMesh {
    pub bounds: Option<(Vec3, f32)>,
    pub opa: Option<DrawList>,
    pub xlu: Option<DrawList>,
}

pub struct RoomMesh {
    pub index: usize,
    pub kind: ShapeKind,
    pub entries: Vec<EntryMesh>,
}

/// A scene with its rooms, keeps and draw config.
pub struct SceneDraw {
    pub scene: Scene,
    pub def: SceneDef,
    pub draw_fn: String,
    pub rooms: Vec<Room>,
    pub keeps: Keeps,
    pub keep_file: Option<String>,
}

impl SceneDraw {
    pub fn load(p: &Project, tables: &SceneTables, name: &str, layer: usize) -> Result<SceneDraw> {
        let scene = Scene::load_layer(&p.rom, name, layer)?;
        let def = tables.scene(&scene.name).cloned().with_context(|| format!("{} is not in scene_table.h", scene.name))?;
        let draw_fn = tables.draw_config_fn(&def);
        let rooms = (0..scene.rooms.len()).map(|i| Room::load(p, &scene, i)).collect::<Result<Vec<_>>>()?;
        let keep_file = scene.keep_object.and_then(|k| tables.objects.get(k as usize).cloned()).filter(|f| !f.is_empty());
        let keeps = Keeps {
            code: code_ram_image(p),
            gameplay_keep: p.rom.file_by_name("gameplay_keep").ok(),
            scene_keep: keep_file.as_ref().and_then(|f| p.rom.file_by_name(f).ok()),
        };
        Ok(SceneDraw { scene, def, draw_fn, rooms, keeps, keep_file })
    }

    fn ctx<'a>(&self, p: &'a Project) -> SceneCtx<'a> {
        SceneCtx {
            p,
            scene_file: self.scene.name.clone(),
            scene_buf: self.scene.file.clone(),
            room_files: self.rooms.iter().map(|r| r.file_name.clone()).collect(),
            room_bufs: self.rooms.iter().map(|r| r.data.clone()).collect(),
        }
    }

    pub fn run_draw_config(&self, tables: &SceneTables, state: &drawcfg::State) -> drawcfg::Output {
        tables.drawcfg.run_with(&self.draw_fn, self.def.id as i64, &tables.scene_ids, state)
    }

    /// Interprets every room entry with the draw config's bindings for `state`. Each entry
    /// runs in a fresh interpreter (setup DL 25 plus the draw config's direct commands),
    /// since `Room_DrawCullable` draws entries in a per-frame order.
    pub fn build(&self, p: &Project, tables: &SceneTables, state: &drawcfg::State, notes: &mut BTreeSet<String>) -> Vec<RoomMesh> {
        let cfg = self.run_draw_config(tables, state);
        notes.extend(cfg.errors.iter().cloned());
        let ctx = self.ctx(p);
        let mut out = Vec::new();
        for (ri, r) in self.rooms.iter().enumerate() {
            let Some(shape) = &r.shape else { continue };
            let opa = ctx.segments(ri, &self.keeps, &cfg.opa, notes);
            let xlu = ctx.segments(ri, &self.keeps, &cfg.xlu, notes);
            let entries = shape
                .entries
                .iter()
                .map(|e| EntryMesh {
                    bounds: e.bounds.map(|(c, rad)| (Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32), rad as f32)),
                    opa: (e.opa != 0).then(|| run_dls(&opa, &[e.opa])),
                    xlu: (e.xlu != 0).then(|| run_dls(&xlu, &[e.xlu])),
                })
                .collect();
            out.push(RoomMesh { index: ri, kind: shape.kind, entries });
        }
        out
    }

    /// This frame's contents of the dynamic segments, for the OPA and XLU buffers.
    pub fn segment_values(&self, p: &Project, tables: &SceneTables, state: &drawcfg::State) -> [SegmentValues; 2] {
        let cfg = self.run_draw_config(tables, state);
        let ctx = self.ctx(p);
        let mut notes = BTreeSet::new();
        [&cfg.opa, &cfg.xlu].map(|buf| {
            let mut v = SegmentValues::default();
            for (&s, b) in &buf.segments {
                if let drawcfg::Bind::Dl(cmds) = &b.bind {
                    v.read(s, &ctx.materialize(cmds, &mut notes));
                }
            }
            v
        })
    }
}

/// `Room_DrawCullable`'s entry selection and order: project each bounding-sphere centre with
/// the view-projection matrix; keep entries with `-radius < z` and `z - radius < fogFar`;
/// draw them by ascending `z - radius` (stable for ties, as the insertion into the linked list
/// puts equal keys after existing ones). `clip_z` maps a world point to its clip-space z.
pub fn cullable_order(bounds: &[Option<(Vec3, f32)>], clip_z: impl Fn(Vec3) -> f32, fog_far: f32) -> Vec<usize> {
    let mut keep: Vec<(f32, usize)> = bounds
        .iter()
        .enumerate()
        .filter_map(|(i, b)| {
            let (c, r) = (*b)?;
            let z = clip_z(c);
            (-r < z && z - r < fog_far).then_some((z - r, i))
        })
        .collect();
    keep.sort_by(|a, b| a.0.total_cmp(&b.0));
    keep.into_iter().map(|k| k.1).collect()
}

/// `guPerspective` (libultra `gu/perspective.c`) as a column-vector matrix, for clip-space z
/// the way the game computes it (OpenGL-style depth, -w..w).
pub fn gu_perspective(fovy_deg: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let cot = 1.0 / (fovy_deg.to_radians() / 2.0).tan();
    Mat4::from_cols_array(&[
        cot / aspect, 0.0, 0.0, 0.0,
        0.0, cot, 0.0, 0.0,
        0.0, 0.0, (near + far) / (near - far), -1.0,
        0.0, 0.0, 2.0 * near * far / (near - far), 0.0,
    ])
}
