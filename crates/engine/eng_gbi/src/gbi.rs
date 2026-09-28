//! F3DEX2 display-list interpreter.
//!
//! Instead of rasterising, it records triangles grouped into batches that share a
//! snapshot of the RSP/RDP state (`Material`), into an `eng_gfx` draw list (re-exported
//! here). Every vertex remembers which *bone* (skeleton limb matrix) was current when it
//! was loaded, which is exactly how OoT's flexible skeletons stitch neighbouring limbs
//! together: a limb display list may `gSPMatrix` any other limb's matrix out of segment
//! 0x0D before loading vertices.

use std::sync::Arc;

pub use eng_gfx::draw::*;
use eng_gfx::combiner::Combiner;
use glam::{Mat4, Vec2, Vec3};

use crate::texture::{DecodedImage, TileDescriptor, Tmem, bits_per_texel};

/// Where a segment points.
#[derive(Clone)]
pub enum Segment {
    Data { buf: Arc<[u8]>, base: usize },
    /// OoT's flex-skeleton matrix array: 0x40-byte entries mapped to bones.
    Matrices(Vec<BoneId>),
    /// A display list supplied by the engine rather than the ROM (e.g. segment 0x0C).
    Builtin(Vec<(u32, u32)>),
}

#[derive(Clone, Copy)]
struct ModelView {
    bone: BoneId,
    local: Mat4,
}

#[derive(Clone, Copy, Default)]
struct RawVertex {
    bone: BoneId,
    pos: Vec3,
    normal: Vec3,
    color: [u8; 4],
    st: [i16; 2],
}

pub struct Interpreter {
    pub segments: [Option<Segment>; 16],
    pub draw: DrawList,
    vtx: [RawVertex; 64],
    mv: ModelView,
    mv_stack: Vec<ModelView>,
    geometry_mode: u32,
    othermode_h: u32,
    othermode_l: u32,
    combine: u64,
    prim: [u8; 4],
    prim_lod_frac: u8,
    env: [u8; 4],
    fog: [u8; 4],
    blend_color: [u8; 4],
    tex_on: bool,
    tex_tile: u8,
    tex_scale: [f32; 2],
    tiles: [TileDescriptor; 8],
    timg: (u8, u8, u16, u32),
    tmem: Tmem,
    /// The segment each TMEM address (in 8-byte words) was last loaded from, 0xFF if unknown:
    /// the provenance recorded in `TextureImage::source_segments`.
    tmem_src: Vec<u8>,
    rdphalf1: u32,
    current_textures: Option<[Option<(TextureSlot, TileDescriptor)>; 2]>,
    current_material: Option<usize>,
    pub max_commands: usize,
    /// Segments whose display lists change every frame (bit per segment). Tile sizes and
    /// colours set while running one are tagged on the materials that use them.
    pub dynamic_segments: u16,
    cur_dyn: Option<u8>,
    tile_dyn: [Option<u8>; 8],
    env_dyn: Option<u8>,
    prim_dyn: Option<u8>,
}


impl Interpreter {
    pub fn new() -> Interpreter {
        Interpreter {
            segments: Default::default(),
            draw: DrawList::default(),
            vtx: [RawVertex::default(); 64],
            mv: ModelView { bone: NO_BONE, local: Mat4::IDENTITY },
            mv_stack: Vec::new(),
            geometry_mode: 0,
            othermode_h: 0,
            othermode_l: 0,
            combine: 0,
            prim: [255; 4],
            prim_lod_frac: 0,
            env: [255; 4],
            fog: [0; 4],
            blend_color: [0; 4],
            tex_on: false,
            tex_tile: 0,
            tex_scale: [1.0, 1.0],
            tiles: [TileDescriptor::default(); 8],
            timg: (0, 0, 1, 0),
            tmem: Tmem::default(),
            tmem_src: vec![0xFF; 512],
            rdphalf1: 0,
            current_textures: None,
            current_material: None,
            max_commands: 200_000,
            dynamic_segments: 0,
            cur_dyn: None,
            tile_dyn: [None; 8],
            env_dyn: None,
            prim_dyn: None,
        }
    }

    /// State used by `Gfx_SetupDL_25Opa`, which the game runs before drawing most actors
    /// (including Player).
    pub fn apply_setup_dl_25(&mut self) {
        self.tex_on = true;
        self.tex_tile = 0;
        self.tex_scale = [1.0, 1.0];
        self.combine = crate::combiner::cc_modulate_idecala_modulateia_prim2();
        // G_AD_NOTPATTERN | G_TF_BILERP | G_TT_NONE | G_TP_PERSP | G_CYC_2CYCLE
        self.othermode_h = (1 << 20) | (2 << 12) | (1 << 19) | (2 << 4);
        // G_RM_FOG_SHADE_A | G_RM_AA_ZB_OPA_SURF2
        let rm_fog_shade_a = (3u32 << 30) | (2 << 26) | (0 << 22) | (0 << 18);
        let rm_aa_zb_opa_surf2 = 0x8 | Z_CMP | Z_UPD | 0x40 | 0x2000 | (0 << 28) | (0 << 24) | (1 << 20) | (1 << 16);
        self.othermode_l = rm_fog_shade_a | rm_aa_zb_opa_surf2;
        self.geometry_mode = G_ZBUFFER | G_SHADE | G_CULL_BACK | G_FOG | G_LIGHTING | G_SHADING_SMOOTH;
        self.invalidate();
    }

    pub fn set_env_color(&mut self, rgba: [u8; 4]) {
        self.env = rgba;
        self.env_dyn = None;
        self.current_material = None;
    }

    /// Attach subsequent vertex loads to `bone` with an identity local transform.
    pub fn set_bone(&mut self, bone: BoneId) {
        self.mv = ModelView { bone, local: Mat4::IDENTITY };
        self.mv_stack.clear();
    }

    fn invalidate(&mut self) {
        self.current_textures = None;
        self.current_material = None;
    }

    fn resolve(&mut self, addr: u32, len: usize) -> Option<(Arc<[u8]>, usize)> {
        let seg = ((addr >> 24) & 0x0F) as usize;
        let off = (addr & 0x00FF_FFFF) as usize;
        let res = match &self.segments[seg] {
            Some(Segment::Data { buf, base }) if base + off + len <= buf.len() => Some((buf.clone(), base + off)),
            _ => None,
        };
        if res.is_none() {
            *self.draw.stats.unresolved_addresses.entry(format!("{addr:08X}")).or_default() += 1;
        }
        res
    }

    fn dyn_of(&self, addr: u32, current: Option<u8>) -> Option<u8> {
        let seg = ((addr >> 24) & 0xF) as u8;
        if self.dynamic_segments & (1 << seg) != 0 { Some(seg) } else { current }
    }

    pub fn run(&mut self, addr: u32) {
        let saved = self.cur_dyn;
        self.cur_dyn = self.dyn_of(addr, saved);
        self.run_inner(addr);
        self.cur_dyn = saved;
    }

    fn run_inner(&mut self, addr: u32) {
        let mut stack: Vec<(Arc<[u8]>, usize, Option<u8>)> = Vec::new();
        let (mut buf, mut pc) = match self.fetch_dl(addr) {
            Some(x) => x,
            None => return,
        };
        let mut budget = self.max_commands;
        loop {
            if budget == 0 {
                log::warn!("display list {addr:08X} exceeded command budget");
                return;
            }
            budget -= 1;
            if pc + 8 > buf.len() {
                log::warn!("display list ran off the end of its buffer");
                return;
            }
            let w0 = u32::from_be_bytes(buf[pc..pc + 4].try_into().unwrap());
            let w1 = u32::from_be_bytes(buf[pc + 4..pc + 8].try_into().unwrap());
            pc += 8;
            self.draw.stats.commands += 1;
            match (w0 >> 24) as u8 {
                0xDE => {
                    // G_DL: bits 16..23 == 1 means branch (no return).
                    self.draw.stats.dl_calls += 1;
                    let branch = (w0 >> 16) & 0xFF != 0;
                    let seg = ((w1 >> 24) & 0xF) as usize;
                    if let Some(Segment::Builtin(cmds)) = &self.segments[seg] {
                        let cmds = cmds.clone();
                        self.run_builtin(&cmds);
                        if branch {
                            match stack.pop() {
                                Some((b, p, d)) => (buf, pc, self.cur_dyn) = (b, p, d),
                                None => return,
                            }
                        }
                        continue;
                    }
                    match self.fetch_dl(w1) {
                        Some((b, p)) => {
                            if !branch {
                                if stack.len() >= 32 {
                                    log::warn!("display list stack overflow");
                                    return;
                                }
                                stack.push((buf, pc, self.cur_dyn));
                            }
                            (buf, pc) = (b, p);
                            self.cur_dyn = self.dyn_of(w1, self.cur_dyn);
                        }
                        None => {}
                    }
                }
                0xDF => match stack.pop() {
                    // G_ENDDL
                    Some((b, p, d)) => (buf, pc, self.cur_dyn) = (b, p, d),
                    None => return,
                },
                0x04 => {
                    // G_BRANCH_Z: assume the camera is close, so always take the near branch.
                    if let Some((b, p)) = self.fetch_dl(self.rdphalf1) {
                        (buf, pc) = (b, p);
                    }
                }
                _ => self.exec(w0, w1),
            }
        }
    }

    fn run_builtin(&mut self, cmds: &[(u32, u32)]) {
        for &(w0, w1) in cmds {
            if (w0 >> 24) as u8 == 0xDF {
                return;
            }
            self.exec(w0, w1);
        }
    }

    fn fetch_dl(&mut self, addr: u32) -> Option<(Arc<[u8]>, usize)> {
        self.resolve(addr, 8)
    }

    fn exec(&mut self, w0: u32, w1: u32) {
        let op = (w0 >> 24) as u8;
        match op {
            0x00 | 0xE0 | 0xE6 | 0xE7 | 0xE8 | 0xE9 | 0xED | 0xEE | 0xEA | 0xEB | 0xEC | 0xFE | 0xFF | 0xF7 => {}
            0x01 => self.op_vtx(w0, w1),
            0x02 => self.op_modifyvtx(w0, w1),
            0x03 => {} // G_CULLDL: no view frustum here.
            0x05 => {
                self.tri((w0 >> 17) & 0x7F, (w0 >> 9) & 0x7F, (w0 >> 1) & 0x7F);
            }
            0x06 | 0x07 => {
                self.tri((w0 >> 17) & 0x7F, (w0 >> 9) & 0x7F, (w0 >> 1) & 0x7F);
                self.tri((w1 >> 17) & 0x7F, (w1 >> 9) & 0x7F, (w1 >> 1) & 0x7F);
            }
            0xD7 => {
                self.tex_on = (w0 >> 1) & 0x7F != 0;
                self.tex_tile = ((w0 >> 8) & 7) as u8;
                self.tex_scale = [(w1 >> 16) as f32 / 65536.0, (w1 & 0xFFFF) as f32 / 65536.0];
                self.invalidate();
            }
            0xD8 => {
                // G_POPMTX
                let n = (w1 / 64).max(1);
                for _ in 0..n {
                    if let Some(m) = self.mv_stack.pop() {
                        self.mv = m;
                    }
                }
            }
            0xD9 => {
                self.geometry_mode = (self.geometry_mode & (w0 & 0x00FF_FFFF)) | w1;
                self.current_material = None;
            }
            0xDA => self.op_mtx(w0, w1),
            0xDB => self.op_moveword(w0, w1),
            0xDC => self.ignore("G_MOVEMEM"),
            0xE1 => self.rdphalf1 = w1,
            0xF1 => {}
            0xE2 | 0xE3 => {
                let len = (w0 & 0xFF) + 1;
                let shift = 32u32.saturating_sub(((w0 >> 8) & 0xFF) + len);
                let mask = (((1u64 << len) - 1) << shift) as u32;
                let target = if op == 0xE3 { &mut self.othermode_h } else { &mut self.othermode_l };
                *target = (*target & !mask) | (w1 & mask);
                self.invalidate();
            }
            0xEF => {
                self.othermode_h = w0 & 0x00FF_FFFF;
                self.othermode_l = w1;
                self.invalidate();
            }
            0xE4 | 0xE5 => self.ignore("G_TEXRECT"),
            0xF6 => self.ignore("G_FILLRECT"),
            0xF0 => self.op_loadtlut(w1),
            0xF2 => {
                self.tile_dyn[((w1 >> 24) & 7) as usize] = self.cur_dyn;
                let t = &mut self.tiles[((w1 >> 24) & 7) as usize];
                t.uls = ((w0 >> 12) & 0xFFF) as u16;
                t.ult = (w0 & 0xFFF) as u16;
                t.lrs = ((w1 >> 12) & 0xFFF) as u16;
                t.lrt = (w1 & 0xFFF) as u16;
                self.invalidate();
            }
            0xF3 => self.op_loadblock(w0, w1),
            0xF4 => self.op_loadtile(w0, w1),
            0xF5 => {
                let t = &mut self.tiles[((w1 >> 24) & 7) as usize];
                t.fmt = ((w0 >> 21) & 7) as u8;
                t.siz = ((w0 >> 19) & 3) as u8;
                t.line = ((w0 >> 9) & 0x1FF) as u16;
                t.tmem = (w0 & 0x1FF) as u16;
                t.palette = ((w1 >> 20) & 0xF) as u8;
                t.cmt = ((w1 >> 18) & 3) as u8;
                t.maskt = ((w1 >> 14) & 0xF) as u8;
                t.shiftt = ((w1 >> 10) & 0xF) as u8;
                t.cms = ((w1 >> 8) & 3) as u8;
                t.masks = ((w1 >> 4) & 0xF) as u8;
                t.shifts = (w1 & 0xF) as u8;
                self.invalidate();
            }
            0xF8 => self.set_color(|s| &mut s.fog, w1),
            0xF9 => self.set_color(|s| &mut s.blend_color, w1),
            0xFA => {
                self.prim_dyn = self.cur_dyn;
                self.prim_lod_frac = (w0 & 0xFF) as u8;
                self.set_color(|s| &mut s.prim, w1);
            }
            0xFB => {
                self.env_dyn = self.cur_dyn;
                self.set_color(|s| &mut s.env, w1)
            }
            0xFC => {
                self.combine = ((w0 as u64 & 0x00FF_FFFF) << 32) | w1 as u64;
                self.current_material = None;
            }
            0xFD => {
                self.timg = (((w0 >> 21) & 7) as u8, ((w0 >> 19) & 3) as u8, ((w0 & 0xFFF) + 1) as u16, w1);
            }
            _ => {
                *self.draw.stats.unknown_opcodes.entry(format!("{op:02X}")).or_default() += 1;
            }
        }
    }

    fn ignore(&mut self, name: &str) {
        *self.draw.stats.ignored_opcodes.entry(name.to_string()).or_default() += 1;
    }

    fn set_color(&mut self, f: impl FnOnce(&mut Self) -> &mut [u8; 4], w1: u32) {
        *f(self) = w1.to_be_bytes();
        self.current_material = None;
    }

    fn op_vtx(&mut self, w0: u32, w1: u32) {
        let n = ((w0 >> 12) & 0xFF) as usize;
        let end = ((w0 >> 1) & 0x7F) as usize;
        let Some(v0) = end.checked_sub(n) else { return };
        let Some((buf, off)) = self.resolve(w1, n * 16) else { return };
        let normal_mat = self.mv.local;
        for i in 0..n {
            let o = off + i * 16;
            let rd = |k: usize| i16::from_be_bytes([buf[o + k], buf[o + k + 1]]);
            let p = Vec3::new(rd(0) as f32, rd(2) as f32, rd(4) as f32);
            let nrm = Vec3::new(buf[o + 12] as i8 as f32, buf[o + 13] as i8 as f32, buf[o + 14] as i8 as f32);
            let slot = v0 + i;
            if slot >= self.vtx.len() {
                break;
            }
            self.vtx[slot] = RawVertex {
                bone: self.mv.bone,
                pos: self.mv.local.transform_point3(p),
                normal: normal_mat.transform_vector3(nrm).normalize_or_zero(),
                color: [buf[o + 12], buf[o + 13], buf[o + 14], buf[o + 15]],
                st: [rd(8), rd(10)],
            };
        }
        self.draw.stats.vertices_loaded += n;
    }

    fn op_modifyvtx(&mut self, w0: u32, w1: u32) {
        let idx = ((w0 & 0xFFFF) / 2) as usize;
        let which = (w0 >> 16) & 0xFF;
        if let Some(v) = self.vtx.get_mut(idx) {
            match which {
                0x10 => v.color = w1.to_be_bytes(),
                0x14 => v.st = [(w1 >> 16) as i16, w1 as i16],
                _ => {}
            }
        }
    }

    fn op_mtx(&mut self, w0: u32, w1: u32) {
        self.draw.stats.matrix_loads += 1;
        let params = (w0 & 0xFF) ^ 1; // F3DEX2 stores params ^ G_MTX_PUSH
        let push = params & 1 != 0;
        let load = params & 2 != 0;
        let projection = params & 4 != 0;
        if projection {
            return;
        }
        let seg = ((w1 >> 24) & 0xF) as usize;
        if let Some(Segment::Matrices(map)) = &self.segments[seg] {
            let idx = ((w1 & 0xFF_FFFF) / 0x40) as usize;
            let bone = map.get(idx).copied().unwrap_or(NO_BONE);
            if bone == NO_BONE {
                *self.draw.stats.unresolved_addresses.entry(format!("{w1:08X} (matrix)")).or_default() += 1;
            }
            if push {
                self.mv_stack.push(self.mv);
            }
            self.mv = ModelView { bone, local: Mat4::IDENTITY };
            return;
        }
        let Some((buf, off)) = self.resolve(w1, 64) else { return };
        let m = read_n64_mtx(&buf[off..off + 64]);
        if push {
            self.mv_stack.push(self.mv);
        }
        self.mv.local = if load { m } else { self.mv.local * m };
    }

    fn op_moveword(&mut self, w0: u32, w1: u32) {
        let index = (w0 >> 16) & 0xFF;
        let offset = w0 & 0xFFFF;
        if index == 0x06 {
            // G_MW_SEGMENT: only meaningful here if it points into another segment.
            let seg = (offset / 4) as usize & 0xF;
            let target = ((w1 >> 24) & 0xF) as usize;
            if (w1 >> 24) <= 0x0F
                && target != 0
                && let Some(Segment::Data { buf, base }) = self.segments[target].clone()
            {
                self.segments[seg] = Some(Segment::Data { buf, base: base + (w1 & 0xFF_FFFF) as usize });
            }
        } else {
            self.ignore("G_MOVEWORD");
        }
    }

    fn op_loadtlut(&mut self, w1: u32) {
        let tile = self.tiles[((w1 >> 24) & 7) as usize];
        let count = (((w1 >> 14) & 0x3FF) + 1) as usize;
        let Some((buf, off)) = self.resolve(self.timg.3, count * 2) else { return };
        let start = (tile.tmem as usize).saturating_sub(256);
        for i in 0..count {
            let o = off + i * 2;
            if let Some(slot) = self.tmem.tlut.get_mut(start + i) {
                *slot = u16::from_be_bytes([buf[o], buf[o + 1]]);
            }
        }
        self.invalidate();
    }

    fn op_loadblock(&mut self, w0: u32, w1: u32) {
        let tile = self.tiles[((w1 >> 24) & 7) as usize];
        let uls = ((w0 >> 12) & 0xFFF) as usize;
        let ult = (w0 & 0xFFF) as usize;
        let lrs = ((w1 >> 12) & 0xFFF) as usize;
        let texels = lrs.saturating_sub(uls) + 1;
        let bpp_bits = bits_per_texel(tile.siz);
        let bytes = (texels * bpp_bits).div_ceil(8).min(4096);
        let (_, timg_siz, timg_w, addr) = self.timg;
        let start = (ult * timg_w as usize + uls) * bits_per_texel(timg_siz) / 8;
        let Some((buf, off)) = self.resolve(addr.wrapping_add(start as u32), bytes) else { return };
        let data = buf[off..off + bytes].to_vec();
        self.tmem.write(tile.tmem as usize * 8, &data);
        self.mark_tmem(tile.tmem as usize * 8, bytes, ((addr >> 24) & 0xF) as u8);
        self.invalidate();
    }

    fn op_loadtile(&mut self, w0: u32, w1: u32) {
        let tile = self.tiles[((w1 >> 24) & 7) as usize];
        let uls = ((w0 >> 12) & 0xFFF) as usize >> 2;
        let ult = (w0 & 0xFFF) as usize >> 2;
        let lrs = ((w1 >> 12) & 0xFFF) as usize >> 2;
        let lrt = (w1 & 0xFFF) as usize >> 2;
        let (_, timg_siz, timg_w, addr) = self.timg;
        let bpp = bits_per_texel(timg_siz);
        let row_bytes = ((lrs.saturating_sub(uls) + 1) * bpp).div_ceil(8);
        for (i, row) in (ult..=lrt).enumerate() {
            let start = (row * timg_w as usize + uls) * bpp / 8;
            let Some((buf, off)) = self.resolve(addr.wrapping_add(start as u32), row_bytes) else { return };
            let data = buf[off..off + row_bytes].to_vec();
            self.tmem.load_row(&tile, i, &data);
            self.mark_tmem(tile.tmem as usize * 8 + i * Tmem::row_stride(&tile), row_bytes, ((addr >> 24) & 0xF) as u8);
        }
        self.invalidate();
    }

    fn texture_slots(&mut self) -> [Option<(TextureSlot, TileDescriptor)>; 2] {
        if let Some(t) = self.current_textures {
            return t;
        }
        let comb = Combiner::decode(self.combine);
        let two_cycle = self.two_cycle();
        let uses = [comb.uses_texel(0, two_cycle), comb.uses_texel(1, two_cycle)];
        let tlut_mode = ((self.othermode_h >> 14) & 3) as u8;
        let mut out = [None, None];
        if self.tex_on {
            for (i, used) in uses.iter().enumerate() {
                if !used {
                    continue;
                }
                let tile = self.tiles[((self.tex_tile + i as u8) & 7) as usize];
                let img = self.tmem.decode(&tile, tlut_mode);
                let hash = hash_image(&img, tile.fmt, tile.siz);
                let source_segments = self.tmem_sources(&tile);
                let image = self.draw.intern_texture(hash, || TextureImage { image: img, fmt: tile.fmt, siz: tile.siz, hash, source_segments });
                out[i] = Some((TextureSlot { image, wrap_s: tile.wrap_s(), wrap_t: tile.wrap_t() }, tile));
            }
        }
        self.current_textures = Some(out);
        out
    }

    /// Records that TMEM bytes `start..start + len` now hold data from `segment`.
    fn mark_tmem(&mut self, start: usize, len: usize, segment: u8) {
        if len == 0 {
            return;
        }
        for w in start / 8..=(start + len - 1) / 8 {
            self.tmem_src[w & 511] = segment;
        }
    }

    /// The segments the texels a tile decodes were loaded from, as a bit mask.
    fn tmem_sources(&self, tile: &TileDescriptor) -> u16 {
        let mut mask = 0u16;
        for (start, len) in Tmem::footprint(tile) {
            for w in start / 8..=(start + len - 1) / 8 {
                let s = self.tmem_src[w & 511];
                if s != 0xFF {
                    mask |= 1 << (s & 0xF);
                }
            }
        }
        mask
    }

    fn two_cycle(&self) -> bool {
        (self.othermode_h >> 20) & 3 == 1
    }

    fn material(&mut self) -> usize {
        if let Some(m) = self.current_material {
            return m;
        }
        let tex = self.texture_slots();
        let two_cycle = self.two_cycle();
        let l = self.othermode_l;
        let gm = self.geometry_mode;
        // In 2-cycle mode the second blender cycle produces the final pixel.
        let (p, a, m, b) = if two_cycle {
            ((l >> 28) & 3, (l >> 24) & 3, (l >> 20) & 3, (l >> 16) & 3)
        } else {
            ((l >> 30) & 3, (l >> 26) & 3, (l >> 22) & 3, (l >> 18) & 3)
        };
        let alpha_blend = p == 0 && a == 0 && m == 1 && b == 0;
        let blend = if alpha_blend && (l & FORCE_BL != 0 || l & ZMODE_MASK == ZMODE_XLU) {
            BlendMode::Translucent
        } else if l & 3 == 1 {
            BlendMode::Cutout(self.blend_color[3].max(1))
        } else if l & CVG_X_ALPHA != 0 {
            BlendMode::Cutout(128)
        } else {
            BlendMode::Opaque
        };
        let cull = match (gm & G_CULL_FRONT != 0, gm & G_CULL_BACK != 0) {
            (false, false) => CullMode::None,
            (false, true) => CullMode::Back,
            (true, false) => CullMode::Front,
            (true, true) => CullMode::Both,
        };
        let mat = Material {
            combiner: Combiner::decode(self.combine),
            two_cycle,
            prim: self.prim,
            prim_lod_frac: self.prim_lod_frac,
            env: self.env,
            fog: self.fog,
            blend_color: self.blend_color,
            geometry_mode: gm,
            othermode_h: self.othermode_h,
            othermode_l: l,
            textures: [tex[0].map(|t| t.0), tex[1].map(|t| t.0)],
            blend,
            cull,
            depth_test: gm & G_ZBUFFER != 0 && l & Z_CMP != 0,
            depth_write: gm & G_ZBUFFER != 0 && l & Z_UPD != 0,
            decal: l & ZMODE_MASK == ZMODE_DEC,
            lit: gm & G_LIGHTING != 0,
            texgen: gm & G_TEXTURE_GEN != 0,
            bilinear: (self.othermode_h >> 12) & 3 != 0,
            uv_dyn: [0usize, 1].map(|i| {
                let (_, tile) = tex[i]?;
                let ti = ((self.tex_tile + i as u8) & 7) as usize;
                self.tile_dyn[ti].map(|segment| DynTile {
                    segment,
                    tile: ti as u8,
                    uls: tile.uls,
                    ult: tile.ult,
                    width: tile.image_width() as u16,
                    height: tile.image_height() as u16,
                })
            }),
            env_dyn: self.env_dyn,
            prim_dyn: self.prim_dyn,
            // gbi.h: G_RM_FOG_SHADE_A = GBL_c1(G_BL_CLR_FOG, G_BL_A_SHADE, G_BL_CLR_IN, G_BL_1MA).
            fog_blend: two_cycle && gm & G_FOG != 0 && (l >> 30) & 3 == 3 && (l >> 26) & 3 == 2,
        };
        let id = self.draw.intern_material(mat);
        self.current_material = Some(id);
        id
    }

    fn tri(&mut self, a: u32, b: u32, c: u32) {
        let mat = self.material();
        let tex = self.texture_slots();
        let scale = self.tex_scale;
        let make = |rv: &RawVertex| -> Vertex {
            let mut uv = [Vec2::ZERO; 2];
            for (i, slot) in tex.iter().enumerate() {
                if let Some((_, tile)) = slot {
                    // S10.5 coordinates scaled by gSPTexture, shifted per tile, relative to the tile origin.
                    let s = rv.st[0] as f32 / 32.0 * scale[0] * TileDescriptor::shift_scale(tile.shifts);
                    let t = rv.st[1] as f32 / 32.0 * scale[1] * TileDescriptor::shift_scale(tile.shiftt);
                    let s = s - tile.uls as f32 / 4.0;
                    let t = t - tile.ult as f32 / 4.0;
                    uv[i] = Vec2::new(s / tile.image_width() as f32, t / tile.image_height() as f32);
                }
            }
            Vertex { bone: rv.bone, pos: rv.pos, normal: rv.normal, color: rv.color, uv }
        };
        // Callers pass vertex indices (the command stores index * 2; they shift by 17/9/1).
        let (a, b, c) = (a as usize, b as usize, c as usize);
        if a >= 64 || b >= 64 || c >= 64 {
            return;
        }
        let verts = [make(&self.vtx[a]), make(&self.vtx[b]), make(&self.vtx[c])];
        match self.draw.batches.last_mut() {
            Some(batch) if batch.material == mat => batch.vertices.extend_from_slice(&verts),
            _ => self.draw.batches.push(Batch { material: mat, vertices: verts.to_vec() }),
        }
        self.draw.stats.triangles += 1;
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// N64 `Mtx`: 16 s16 integer parts followed by 16 u16 fractions, row-major, row-vector convention.
pub fn read_n64_mtx(b: &[u8]) -> Mat4 {
    let mut m = [0f32; 16];
    for (i, v) in m.iter_mut().enumerate() {
        let int = i16::from_be_bytes([b[i * 2], b[i * 2 + 1]]) as i32;
        let frac = u16::from_be_bytes([b[32 + i * 2], b[32 + i * 2 + 1]]) as i32;
        *v = ((int << 16) | frac) as f32 / 65536.0;
    }
    // Row-major rows become glam columns, converting row-vector to column-vector convention.
    Mat4::from_cols_array(&m)
}

fn hash_image(img: &DecodedImage, fmt: u8, siz: u8) -> u64 {
    // FNV-1a over dimensions and pixels.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    };
    for b in img.width.to_le_bytes().iter().chain(img.height.to_le_bytes().iter()) {
        feed(*b);
    }
    feed(fmt);
    feed(siz);
    for b in &img.rgba {
        feed(*b);
    }
    h
}
