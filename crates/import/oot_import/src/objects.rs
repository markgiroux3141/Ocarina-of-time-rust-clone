//! Reading the assets the decomp's XMLs name inside a ROM file: textures (with their palettes),
//! skeletons, standard animations and standalone display lists. Shared by the importer
//! (`crate::pack`) and the extractor (`oot_extract`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use eng_anim::anim::StandardAnimation;
use eng_anim::skeleton::{LimbType, Skeleton};
use eng_gbi::gbi::{DrawList, Interpreter, Segment};
use eng_gbi::model::{Binding, BuildOptions, build_draw_list, cull_back_builtin, display_list_bytes};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake};
use eng_gbi::texture::{self, DecodedImage, decode_linear};

use crate::project::Project;
use crate::symbols::{AssetFile, Symbol};
use crate::z64::{ParseSkeleton, ParseStandardAnimation};

/// `Format` attribute → (`G_IM_FMT_*`, `G_IM_SIZ_*`).
pub fn parse_format(s: &str) -> Option<(u8, u8)> {
    use texture::*;
    Some(match s {
        "rgba16" => (G_IM_FMT_RGBA, G_IM_SIZ_16B),
        "rgba32" => (G_IM_FMT_RGBA, G_IM_SIZ_32B),
        "ci4" => (G_IM_FMT_CI, G_IM_SIZ_4B),
        "ci8" => (G_IM_FMT_CI, G_IM_SIZ_8B),
        "i4" => (G_IM_FMT_I, G_IM_SIZ_4B),
        "i8" => (G_IM_FMT_I, G_IM_SIZ_8B),
        "ia4" => (G_IM_FMT_IA, G_IM_SIZ_4B),
        "ia8" => (G_IM_FMT_IA, G_IM_SIZ_8B),
        "ia16" => (G_IM_FMT_IA, G_IM_SIZ_16B),
        _ => return None,
    })
}

fn hex_attr(v: Option<&str>) -> Option<usize> {
    let t = v?.trim();
    usize::from_str_radix(t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t), 16).ok()
}

/// A grey ramp palette (RGBA16) for colour-indexed textures whose palette is set by code.
fn grey_ramp(entries: usize) -> Vec<u8> {
    (0..entries)
        .flat_map(|i| {
            let v = (i * 31 / (entries - 1).max(1)) as u16;
            ((v << 11) | (v << 6) | (v << 1) | 1).to_be_bytes()
        })
        .collect()
}

/// Where a CI texture's palette came from.
#[derive(Debug, Clone, PartialEq)]
pub enum Palette {
    None,
    /// `TlutOffset` in the same file, or `ExternalTlut` + `ExternalTlutOffset`.
    File { file: String, offset: usize, entries: usize },
    /// Set by code at runtime: decoded against a grey ramp of the indices.
    Code,
}

/// A decoded XML texture.
pub struct DecodedTexture {
    pub fmt: u8,
    pub siz: u8,
    pub width: u32,
    pub height: u32,
    pub image: DecodedImage,
    pub texels: Vec<u8>,
    pub tlut: Option<Vec<u8>>,
    pub palette: Palette,
}

/// ROM files by name, cached, shared between threads.
pub struct Files<'a> {
    p: &'a Project,
    cache: Mutex<HashMap<String, Option<Arc<[u8]>>>>,
}

impl<'a> Files<'a> {
    pub fn new(p: &'a Project) -> Files<'a> {
        Files { p, cache: Mutex::new(HashMap::new()) }
    }
    pub fn get(&self, name: &str) -> Option<Arc<[u8]>> {
        if let Some(hit) = self.cache.lock().unwrap().get(name) {
            return hit.clone();
        }
        let f = self.p.rom.file_by_name(name).ok();
        self.cache.lock().unwrap().insert(name.to_string(), f.clone());
        f
    }
}

/// Decodes the `Texture` symbol `t` of the file `f` (whose bytes are `data`).
pub fn decode_texture(f: &AssetFile, data: &Arc<[u8]>, t: &Symbol, files: &Files) -> Result<DecodedTexture> {
    let fmt_name = t.attr("Format").context("no Format")?;
    let (fmt, siz) = parse_format(fmt_name).with_context(|| format!("format {fmt_name}"))?;
    let w: u32 = t.attr("Width").context("no Width")?.parse()?;
    let h: u32 = t.attr("Height").context("no Height")?.parse()?;
    let off = t.offset as usize;
    let bytes = (w * h) as usize * texture::bits_per_texel(siz) / 8;
    if off + bytes > data.len() {
        bail!("texture {:X}+{:X} outside {} ({:X})", off, bytes, f.name, data.len());
    }
    let (tlut, palette) = if fmt == texture::G_IM_FMT_CI {
        let n = if siz == texture::G_IM_SIZ_4B { 16 } else { 256 };
        let src = match (hex_attr(t.attr("TlutOffset")), t.attr("ExternalTlut"), hex_attr(t.attr("ExternalTlutOffset"))) {
            (Some(o), _, _) => Some((f.name.clone(), data.clone(), o)),
            (None, Some(ext), Some(o)) => files.get(ext).map(|d| (ext.to_string(), d, o)),
            _ => None,
        };
        match src {
            Some((pf, pd, o)) if o < pd.len() => {
                let mut t = pd[o..(o + n * 2).min(pd.len())].to_vec();
                t.resize(n * 2, 0);
                (t, Palette::File { file: pf, offset: o, entries: n })
            }
            _ => (grey_ramp(n), Palette::Code),
        }
    } else {
        (Vec::new(), Palette::None)
    };
    let texels = data[off..off + bytes].to_vec();
    let image = decode_linear(&texels, fmt, siz, w, h, (!tlut.is_empty()).then_some(&tlut[..]));
    Ok(DecodedTexture {
        fmt,
        siz,
        width: w,
        height: h,
        image,
        texels,
        tlut: matches!(palette, Palette::File { .. }).then_some(tlut),
        palette,
    })
}

/// `file`'s XML symbol `symbol`.
/// An overlay file as RAM holds it at its link address (`segments.csv`'s VRAM start), for
/// segment 0 (KSEG0 pointers): zeros up to the address's low 24 bits, then the file. Returns
/// the image and the overlay's VRAM start.
fn overlay_ram_image(p: &Project, files: &Files, name: &str) -> Result<(std::sync::Arc<[u8]>, u32)> {
    let v = crate::version::VersionConfig::load(&p.config.decomp)?;
    let vram = v.segment_vram.get(name).copied().with_context(|| format!("segments.csv: no VRAM for {name}"))?;
    let data = files.get(name).with_context(|| format!("{name} not in the ROM"))?;
    let pad = (vram & 0x00FF_FFFF) as usize;
    let mut img = vec![0u8; pad + data.len()];
    img[pad..].copy_from_slice(&data);
    Ok((img.into(), vram))
}

pub fn symbol_in<'a>(p: &'a Project, file: &str, symbol: &str) -> Result<(&'a AssetFile, &'a Symbol)> {
    let f = p.symbols.file(file).with_context(|| format!("no XML for {file}"))?;
    let s = f.find(symbol).with_context(|| format!("{file} has no {symbol}"))?;
    Ok((f, s))
}

/// Why a skeleton symbol can't be read as a `Skeleton`.
pub fn unsupported_limb_type(s: &Symbol) -> Option<String> {
    match s.attr("LimbType") {
        Some("Standard") | Some("LOD") => None,
        other => Some(format!("limb type {} not supported", other.unwrap_or("(none)"))),
    }
}

/// Skeletons whose header is a flex one (the XML says `Flex`: it has a `dListCount`) but whose
/// actor reads it as a normal one, so the game draws it that way: En_Box's
/// `SkelAnime_Init(play, &this->skelanime, (SkeletonHeader*)&gTreasureChestSkel, ...)` and
/// `SkelAnime_Draw` (z_en_box.c).
const DRAWN_AS_NORMAL: [&str; 1] = ["gTreasureChestSkel"];

/// Parses a `Skeleton` symbol (standard or LOD limbs). Some XMLs label flex skeletons as
/// normal; the header's `dListCount` gives it away.
pub fn parse_skeleton(data: &[u8], seg: u8, s: &Symbol) -> Result<Skeleton> {
    let lt = match s.attr("LimbType") {
        Some("Standard") => LimbType::Standard,
        Some("LOD") => LimbType::Lod,
        other => bail!("limb type {other:?} not supported"),
    };
    if DRAWN_AS_NORMAL.contains(&s.name.as_str()) {
        return Skeleton::parse(data, seg, s.offset as usize, lt, false);
    }
    let flex = s.attr("Type") == Some("Flex");
    let sk = Skeleton::parse(data, seg, s.offset as usize, lt, flex)?;
    let o = s.offset as usize + 8;
    if !flex && data.get(o).is_some_and(|&n| n != 0 && n as usize == sk.flex_matrix_map(0).len()) {
        return Skeleton::parse(data, seg, s.offset as usize, lt, true);
    }
    Ok(sk)
}

fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// Joint count implied by the layout ZAPD/the original tools use: the joint index table sits
/// directly before the header.
pub fn anim_joint_count(data: &[u8], seg: u8, off: usize) -> Option<usize> {
    if off + 12 > data.len() {
        return None;
    }
    let idx = be32(data, off + 8);
    if (idx >> 24) as u8 != seg {
        return None;
    }
    let idx = (idx & 0xFF_FFFF) as usize;
    (idx < off && (off - idx) % 6 == 0).then(|| (off - idx) / 6)
}

/// Decodes the `Animation` symbol `a` for a skeleton of `limbs` limbs.
pub fn parse_animation(data: &[u8], seg: u8, a: &Symbol, limbs: usize) -> Result<StandardAnimation> {
    StandardAnimation::parse(data, seg, a.offset as usize, limbs)
}

/// The segments an object's display lists are interpreted with when no actor binds any:
/// 0 = a RAM image of `code` (for raw KSEG0 pointers), 4 = `gameplay_keep`, 5 =
/// `gameplay_field_keep` (the keep of the overworld scenes; dungeon objects that read
/// `gameplay_dangeon_keep` would resolve against the wrong file, and say so in their
/// unresolved references), the file's own segment (6 for objects), and 0xC = the engine's
/// `G_CULL_BACK` list Player binds.
pub struct ObjectSegments {
    pub code: Option<Arc<[u8]>>,
    pub gameplay_keep: Option<Arc<[u8]>>,
    pub field_keep: Option<Arc<[u8]>>,
}

impl ObjectSegments {
    pub fn load(p: &Project) -> ObjectSegments {
        ObjectSegments {
            code: crate::room::code_ram_image(p),
            gameplay_keep: p.rom.file_by_name("gameplay_keep").ok(),
            field_keep: p.rom.file_by_name("gameplay_field_keep").ok(),
        }
    }

    pub fn bindings(&self, f: &AssetFile, data: &Arc<[u8]>) -> Vec<Binding> {
        let mut b = Vec::new();
        if let Some(d) = &self.code {
            b.push(Binding { segment: 0, buf: d.clone(), base: 0 });
        }
        if let Some(d) = &self.gameplay_keep {
            b.push(Binding { segment: 4, buf: d.clone(), base: 0 });
        }
        if let Some(d) = &self.field_keep {
            b.push(Binding { segment: 5, buf: d.clone(), base: 0 });
        }
        b.push(Binding { segment: f.segment.unwrap_or(6), buf: data.clone(), base: 0 });
        b
    }

    /// A skeleton's full mesh (LOD 0), drawn in the game's limb order.
    pub fn skeleton_mesh(&self, f: &AssetFile, data: &Arc<[u8]>, skel: &Skeleton) -> Result<DrawList> {
        build_draw_list(skel, &BuildOptions { bindings: self.bindings(f, data), ..Default::default() })
    }

    /// An actor's `MeshBake` (docs/adr/0012-actor-bakes.md): the object on segment 6, the keeps
    /// on 4 and 5, the bake's own segments; `Gfx_SetupDL_25Opa`, the prelude, then the body.
    pub fn bake_mesh(&self, p: &Project, files: &Files, bake: &MeshBake) -> Result<DrawList> {
        let file = p.symbols.file(&bake.object).with_context(|| format!("no XML for {}", bake.object))?;
        let data = files.get(&bake.object).with_context(|| format!("{} not in the ROM", bake.object))?;
        let mut bindings = self.bindings(file, &data);
        let mut builtin = vec![(0x0C, cull_back_builtin())];
        let mut dynamic = 0u16;
        let mut matrices: Vec<(u8, eng_gfx::BoneId)> = Vec::new();
        for (seg, s) in &bake.segments {
            match s {
                BakeSegment::Texture { file: f, symbol } => {
                    let (sym_file, sym) = symbol_in(p, f, symbol)?;
                    let buf = files.get(&sym_file.name).with_context(|| format!("{f} not in the ROM"))?;
                    bindings.push(Binding { segment: *seg, buf, base: sym.offset as usize });
                }
                BakeSegment::DynamicColor { env, prim } => {
                    let mut cmds = Vec::new();
                    if *prim {
                        cmds.push((0xFA00_0000, 0x0000_00FF));
                    }
                    if *env {
                        cmds.push((0xFB00_0000, 0x0000_00FF));
                    }
                    bindings.push(Binding { segment: *seg, buf: display_list_bytes(&cmds), base: 0 });
                    dynamic |= 1 << (seg & 0xF);
                }
                BakeSegment::Commands(cmds) => {
                    // As data (not a builtin) so a call to it is an ordinary list.
                    builtin.retain(|b| b.0 != *seg);
                    bindings.push(Binding { segment: *seg, buf: display_list_bytes(cmds), base: 0 });
                }
                BakeSegment::Dynamic(cmds) => {
                    builtin.retain(|b| b.0 != *seg);
                    bindings.push(Binding { segment: *seg, buf: display_list_bytes(cmds), base: 0 });
                    dynamic |= 1 << (seg & 0xF);
                }
                BakeSegment::File(name) => {
                    let buf = files.get(name).with_context(|| format!("{name} not in the ROM"))?;
                    bindings.push(Binding { segment: *seg, buf, base: 0 });
                }
                BakeSegment::Files(names) => {
                    let mut all = Vec::new();
                    for name in names {
                        all.extend_from_slice(&files.get(name).with_context(|| format!("{name} not in the ROM"))?);
                    }
                    bindings.push(Binding { segment: *seg, buf: all.as_slice().into(), base: 0 });
                }
                BakeSegment::GrayRgba32 { file: name, offset, pixels } => {
                    let buf = files.get(name).with_context(|| format!("{name} not in the ROM"))?;
                    let (start, len) = (*offset as usize, *pixels as usize * 4);
                    let mut img = buf.get(start..start + len).with_context(|| format!("{name}: no {len} bytes at {start:#x}"))?.to_vec();
                    oot_game::kaleido::gray_out_texture_rgba32(&mut img);
                    bindings.push(Binding { segment: *seg, buf: img.as_slice().into(), base: 0 });
                }
                BakeSegment::Bytes(bytes) => {
                    bindings.push(Binding { segment: *seg, buf: bytes.as_slice().into(), base: 0 });
                }
                BakeSegment::Matrix(bone) => {
                    anyhow::ensure!(matches!(bake.body, BakeBody::DLists(_)), "bake {}: a matrix segment outside display lists", bake.name);
                    matrices.push((*seg, *bone));
                }
            }
        }
        let prelude: Vec<u32> = bake.prelude.iter().map(|&s| (s as u32) << 24).collect();
        match &bake.body {
            BakeBody::DLists(lists) => {
                let mut it = Interpreter::new();
                for b in &bindings {
                    it.segments[b.segment as usize & 0xF] = Some(Segment::Data { buf: b.buf.clone(), base: b.base });
                }
                for (seg, s) in &builtin {
                    it.segments[*seg as usize & 0xF] = Some(s.clone());
                }
                // A matrix the draw sets: the list's gSPMatrix of it puts the vertices on its bone.
                for &(seg, bone) in &matrices {
                    it.segments[seg as usize & 0xF] = Some(Segment::Matrices(vec![bone]));
                }
                it.apply_setup_dl_25();
                it.dynamic_segments = dynamic;
                // Where each vertex came from: what a draw rebuilds the colours of the vertices
                // the game writes in its object's RAM from (`Demo_Effect`'s time warp).
                it.track_vertex_sources = true;
                for &dl in &prelude {
                    it.run(dl);
                }
                for (f, symbol) in lists {
                    let (sym_file, sym) = symbol_in(p, f, symbol)?;
                    if sym_file.segment.is_none() && sym_file.name.starts_with("ovl_") {
                        // A list in an overlay (`z_eff_ss_fhg_flash.c`'s `sShockDL`): its
                        // pointers are the overlay's link-time KSEG0 addresses, so the overlay
                        // goes on segment 0 where it links (`segments.csv`), as `code` does.
                        let (img, vram) = overlay_ram_image(p, files, &sym_file.name)?;
                        it.segments[0] = Some(Segment::Data { buf: img, base: 0 });
                        it.run(vram + sym.offset);
                        continue;
                    }
                    it.run(((sym_file.segment.unwrap_or(6) as u32) << 24) | sym.offset);
                }
                Ok(it.draw)
            }
            BakeBody::Skeleton { file: sf, symbol, limbs } => {
                let (skel_file, s) = symbol_in(p, sf, symbol)?;
                let skel_data = files.get(&skel_file.name).with_context(|| format!("{sf} not in the ROM"))?;
                let skel = parse_skeleton(&skel_data, skel_file.segment.unwrap_or(6), s)?;
                let mut limb_dlists = Vec::new();
                let mut limb_segments = Vec::new();
                for l in limbs {
                    if l.symbol.is_empty() {
                        // *dList = NULL.
                        limb_dlists.push((l.limb, 0));
                        continue;
                    }
                    let (lf, ls) = symbol_in(p, &l.file, &l.symbol)?;
                    let buf = files.get(&lf.name).with_context(|| format!("{} not in the ROM", l.file))?;
                    limb_dlists.push((l.limb, (6u32 << 24) | ls.offset));
                    limb_segments.push((l.limb, 6, Segment::Data { buf, base: 0 }));
                }
                let opts = BuildOptions { bindings, builtin_segments: builtin, limb_dlists, limb_segments, dynamic_segments: dynamic, prelude, ..Default::default() };
                build_draw_list(&skel, &opts)
            }
            BakeBody::Skin { file: sf, symbol } => {
                let (skel_file, s) = symbol_in(p, sf, symbol)?;
                let skel_data = files.get(&skel_file.name).with_context(|| format!("{sf} not in the ROM"))?;
                let raw = crate::skin::parse(&skel_data, skel_file.segment.unwrap_or(6), s.offset as usize)?;
                let mut it = Interpreter::new();
                for b in &bindings {
                    it.segments[b.segment as usize & 0xF] = Some(Segment::Data { buf: b.buf.clone(), base: b.base });
                }
                for (seg, s) in &builtin {
                    it.segments[*seg as usize & 0xF] = Some(s.clone());
                }
                it.apply_setup_dl_25();
                it.dynamic_segments = dynamic;
                for &dl in &prelude {
                    it.run(dl);
                }
                Ok(crate::skin::draw(it, &raw))
            }
        }
    }

    /// A standalone display list as `Gfx_DrawDListOpa` draws it: `Gfx_SetupDL_25Opa`, then the
    /// list.
    pub fn dlist_mesh(&self, f: &AssetFile, data: &Arc<[u8]>, addr: u32) -> DrawList {
        let mut it = Interpreter::new();
        for b in self.bindings(f, data) {
            it.segments[b.segment as usize & 0xF] = Some(Segment::Data { buf: b.buf, base: b.base });
        }
        it.segments[0x0C] = Some(cull_back_builtin());
        it.apply_setup_dl_25();
        it.run(addr);
        it.draw
    }
}
