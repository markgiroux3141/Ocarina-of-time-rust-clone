//! Draw lists: triangles grouped into batches that share a snapshot of the N64 RSP/RDP state
//! (`Material`). This is the engine's mesh format. Every vertex remembers which *bone*
//! (skeleton limb matrix) it was loaded under, which is how OoT's flexible skeletons stitch
//! neighbouring limbs together. `eng_gbi` builds these by interpreting display lists; the
//! renderer (`eng_render`) draws them.

use std::collections::{BTreeMap, HashMap};

use glam::{Mat4, Vec2, Vec3, Vec4Swizzles};

use crate::combiner::Combiner;
use crate::texture::{DecodedImage, WrapMode};

// Geometry mode bits (F3DEX2).
pub const G_ZBUFFER: u32 = 0x0000_0001;
pub const G_SHADE: u32 = 0x0000_0004;
pub const G_CULL_FRONT: u32 = 0x0000_0200;
pub const G_CULL_BACK: u32 = 0x0000_0400;
pub const G_FOG: u32 = 0x0001_0000;
pub const G_LIGHTING: u32 = 0x0002_0000;
pub const G_TEXTURE_GEN: u32 = 0x0004_0000;
pub const G_TEXTURE_GEN_LINEAR: u32 = 0x0008_0000;
pub const G_SHADING_SMOOTH: u32 = 0x0020_0000;

// Other mode L render-mode bits.
pub const Z_CMP: u32 = 0x10;
pub const Z_UPD: u32 = 0x20;
pub const CVG_X_ALPHA: u32 = 0x1000;
pub const FORCE_BL: u32 = 0x4000;
pub const ZMODE_MASK: u32 = 0xC00;
pub const ZMODE_XLU: u32 = 0x800;
pub const ZMODE_DEC: u32 = 0xC00;

/// Which transform a vertex is attached to.
pub type BoneId = u16;
pub const NO_BONE: BoneId = u16::MAX;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vertex {
    pub bone: BoneId,
    /// Position in the bone's space (already multiplied by any local DL matrices).
    pub pos: Vec3,
    /// Normal in the bone's space; only meaningful when the material is lit.
    pub normal: Vec3,
    pub color: [u8; 4],
    /// Normalised texture coordinates for TEXEL0 and TEXEL1.
    pub uv: [Vec2; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BlendMode {
    Opaque,
    /// Alpha test with threshold in 0..=255.
    Cutout(u8),
    Translucent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CullMode {
    None,
    Back,
    Front,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TextureSlot {
    /// Index into `DrawList::textures`.
    pub image: usize,
    pub wrap_s: WrapMode,
    pub wrap_t: WrapMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Material {
    pub combiner: Combiner,
    pub two_cycle: bool,
    pub prim: [u8; 4],
    pub prim_lod_frac: u8,
    pub env: [u8; 4],
    pub fog: [u8; 4],
    pub blend_color: [u8; 4],
    pub geometry_mode: u32,
    pub othermode_h: u32,
    pub othermode_l: u32,
    pub textures: [Option<TextureSlot>; 2],
    pub blend: BlendMode,
    pub cull: CullMode,
    pub depth_test: bool,
    pub depth_write: bool,
    pub decal: bool,
    pub lit: bool,
    pub texgen: bool,
    pub bilinear: bool,
    /// Per texture slot: the tile size came from a display list in a dynamic segment (a
    /// scene draw config's `Gfx_TexScroll` / `Gfx_TwoTexScroll`), so the UVs move every frame.
    pub uv_dyn: [Option<DynTile>; 2],
    /// The env / prim colour was last set by a display list in this dynamic segment.
    pub env_dyn: Option<u8>,
    pub prim_dyn: Option<u8>,
    /// Fog is blended in: `G_FOG` is on and the first blender cycle is `G_RM_FOG_SHADE_A`
    /// (fog colour weighted by the shade alpha, which `G_FOG` replaces with the fog factor).
    pub fog_blend: bool,
}

/// A tile whose size (origin) is rewritten every frame by a dynamic segment. The UVs were
/// baked with `uls`/`ult`; a new origin shifts them by `-(new - old) / 4 / size`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct DynTile {
    pub segment: u8,
    pub tile: u8,
    pub uls: u16,
    pub ult: u16,
    pub width: u16,
    pub height: u16,
}

/// What the dynamic segments contain this frame: per segment, the tile sizes and colours
/// their display lists set. Built from the draw config's output with [`SegmentValues::read`].
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SegmentValues {
    pub tiles: [[Option<(u16, u16)>; 8]; 16],
    pub env: [Option<[u8; 4]>; 16],
    pub prim: [Option<[u8; 4]>; 16],
}

impl SegmentValues {
    /// Records the `G_SETTILESIZE`, `G_SETENVCOLOR` and `G_SETPRIMCOLOR` commands of the
    /// display list bound to `segment`.
    pub fn read(&mut self, segment: u8, cmds: &[(u32, u32)]) {
        let s = segment as usize & 0xF;
        for &(w0, w1) in cmds {
            match (w0 >> 24) as u8 {
                0xF2 => self.tiles[s][((w1 >> 24) & 7) as usize] = Some((((w0 >> 12) & 0xFFF) as u16, (w0 & 0xFFF) as u16)),
                0xFB => self.env[s] = Some(w1.to_be_bytes()),
                0xFA => self.prim[s] = Some(w1.to_be_bytes()),
                0xDF => break,
                _ => {}
            }
        }
    }
}

impl Material {
    /// UV offsets for texture slots 0 and 1 under this frame's segment values.
    pub fn uv_offsets(&self, v: &SegmentValues) -> [Vec2; 2] {
        self.uv_dyn.map(|d| match d {
            Some(d) => match v.tiles[d.segment as usize & 0xF][d.tile as usize & 7] {
                Some((uls, ult)) => Vec2::new(
                    -((uls as f32 - d.uls as f32) / 4.0) / d.width.max(1) as f32,
                    -((ult as f32 - d.ult as f32) / 4.0) / d.height.max(1) as f32,
                ),
                None => Vec2::ZERO,
            },
            None => Vec2::ZERO,
        })
    }
    /// The env and prim colours under this frame's segment values.
    pub fn colors(&self, v: &SegmentValues) -> ([u8; 4], [u8; 4]) {
        let env = self.env_dyn.and_then(|s| v.env[s as usize & 0xF]).unwrap_or(self.env);
        let prim = self.prim_dyn.and_then(|s| v.prim[s as usize & 0xF]).unwrap_or(self.prim);
        (env, prim)
    }
    pub fn is_dynamic(&self) -> bool {
        self.uv_dyn.iter().any(|d| d.is_some()) || self.env_dyn.is_some() || self.prim_dyn.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Batch {
    pub material: usize,
    /// Triangle list: every three vertices form one triangle.
    pub vertices: Vec<Vertex>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TextureImage {
    pub image: DecodedImage,
    pub fmt: u8,
    pub siz: u8,
    pub hash: u64,
    /// The segments the texels were loaded from (`G_SETTIMG`'s address), one bit per segment:
    /// every TMEM word the texture reads counts, since a texture can read another's leftovers.
    /// Textures from segments the draw code binds per frame (Link's eyes on 8, mouth on 9) can
    /// be swapped by it.
    pub source_segments: u16,
}

#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    pub commands: usize,
    pub triangles: usize,
    pub vertices_loaded: usize,
    pub dl_calls: usize,
    pub matrix_loads: usize,
    pub unknown_opcodes: BTreeMap<String, usize>,
    pub unresolved_addresses: BTreeMap<String, usize>,
    pub ignored_opcodes: BTreeMap<String, usize>,
}

/// Deserialized draw lists have empty intern lookups: they are for drawing, not for adding to.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DrawList {
    pub batches: Vec<Batch>,
    pub materials: Vec<Material>,
    pub textures: Vec<TextureImage>,
    pub stats: Stats,
    #[serde(skip)]
    material_lookup: HashMap<Material, usize>,
    #[serde(skip)]
    texture_lookup: HashMap<u64, usize>,
}

/// Equal content: the same batches, materials, textures and statistics (the intern lookups
/// are only an index of those).
impl PartialEq for DrawList {
    fn eq(&self, o: &DrawList) -> bool {
        self.batches == o.batches && self.materials == o.materials && self.textures == o.textures && self.stats == o.stats
    }
}

impl DrawList {
    pub fn triangle_count(&self) -> usize {
        self.batches.iter().map(|b| b.vertices.len() / 3).sum()
    }

    /// Index of `mat` in `materials`, adding it if it's new.
    pub fn intern_material(&mut self, mat: Material) -> usize {
        *self.material_lookup.entry(mat.clone()).or_insert_with(|| {
            self.materials.push(mat);
            self.materials.len() - 1
        })
    }

    /// Index of the texture with this content hash in `textures`, adding the one `make`
    /// builds if it's new.
    pub fn intern_texture(&mut self, hash: u64, make: impl FnOnce() -> TextureImage) -> usize {
        *self.texture_lookup.entry(hash).or_insert_with(|| {
            self.textures.push(make());
            self.textures.len() - 1
        })
    }
}

/// Axis-aligned bounds of a draw list after posing with `bone_mats`.
pub fn posed_bounds(draw: &DrawList, bone_mats: &[Mat4]) -> (Vec3, Vec3) {
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for b in &draw.batches {
        for v in &b.vertices {
            let m = bone_mats.get(v.bone as usize).copied().unwrap_or(Mat4::IDENTITY);
            let p = (m * v.pos.extend(1.0)).xyz();
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    (lo, hi)
}
