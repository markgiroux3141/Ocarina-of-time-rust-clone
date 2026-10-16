//! `EffectBlure` (`z_eff_blure.c`): the trails behind a swung blade, a spinning Skulltula or a
//! flying arrow. Its owner adds an edge every frame (`EffectBlure_AddVertex`: the blade's tip
//! and base) or a gap (`EffectBlure_AddSpace`); `EffectBlure_Update` ages them, drops the
//! oldest once it's `elemDuration` old, and the draw joins the edges into quads fading from the
//! start colours to the end colours as they age.
//!
//! `EFFECT_BLURE1` (`EffectBlure_Init1`: the Skulltulas', the Lizalfos's) draws the plain strip
//! (`flags` 0); `EFFECT_BLURE2` (`EffectBlure_Init2`) with `drawMode` 0 or 1 the simple quads
//! (the arrows': textured, a crossed copy with `flags` 4), 2 and up the smooth one (Link's
//! sword: each quad between bending edges a Hermite curve of seven).
//!
//! ## Drawing
//!
//! The C builds the vertices every frame. The port draws them through bakes whose every vertex
//! is loaded at the origin under a matrix of its own (`BakeSegment::Matrices` on segment 0x0D):
//! the draw gives each bone the translation to its vertex's place (`DrawCmd::bones`) and each
//! triangle corner its colour (`DrawParams::vertex_colors`). A triangle the frame doesn't draw
//! is there with its corners' alpha 0. The smooth draw's `gSPMatrix` scales by 0.1 in the RSP's
//! s15.16 fixed point (6553 / 65536), which the vertex places are multiplied by.

use glam::{Mat4, Vec3};

use super::{EffectDraws, SEG_COLOR, SEG_DATA, SEG_SETUP, SEG_TEX, lerp_s16, lerp_u8};
use crate::gbi::{
    Dl, G_CYC_2CYCLE, G_FOG, G_IM_FMT_I, G_IM_SIZ_8B, G_LIGHTING, G_RM_PASS, G_RM_ZB_CLD_SURF2, G_SHADE, G_SHADING_SMOOTH, G_TEXTURE_GEN, G_TEXTURE_GEN_LINEAR, G_TX_CLAMP, G_TX_NOLOD,
    G_TX_NOMIRROR, G_TX_WRAP, G_ZBUFFER, ac, cc_ab, cc_c, cc_d, push_vtx, seg, setup_dl,
};
use crate::pack::{BakeBody, BakeSegment, MeshBake, keys};
use crate::skin;
use crate::sys_matrix::MtxF;

/// The table's size (`EffectBlure.elements`).
pub const ELEMENT_COUNT: usize = 16;

/// The segment the bakes' per-vertex matrices are on.
const SEG_MATRICES: u8 = 0x0C;

const BAKE_STRIP: &str = "Effect/blure_strip";
const BAKE_SMOOTH: &str = "Effect/blure_smooth";
const BAKE_SIMPLE: &str = "Effect/blure_simple";
const BAKE_SIMPLE_ALT_FIRST: &str = "Effect/blure_simple_alt_first";
const BAKE_SIMPLE_ALT: &str = "Effect/blure_simple_alt";

/// `EffectBlure_Draw`'s `Vtx[32]`: the strip's vertex slots.
const STRIP_SLOTS: usize = 32;
/// The smooth draw's vertices per quad (`EffectBlure_DrawElemHermiteInterpolation`'s `Vtx[16]`).
const SMOOTH_SLOTS: usize = 16;
/// The smooth draw's triangles per quad in the bake: the no-interpolation quad's two, then the
/// Hermite curve's fourteen.
const SMOOTH_TRIS: usize = 16;

/// `EffectBlureElement`. `state` 0 is a gap, 1 an edge, 2 an unused slot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectBlureElement {
    pub state: i32,
    pub timer: i32,
    pub p1: [i16; 3],
    pub p2: [i16; 3],
    pub flags: u16,
}

impl EffectBlureElement {
    /// `EffectBlure_InitElements`' and `EffectBlure_Update`'s emptied slot.
    const UNUSED: EffectBlureElement = EffectBlureElement { state: 2, timer: 0, p1: [0; 3], p2: [0; 3], flags: 0 };
}

/// `EffectBlureInit1` (`EFFECT_BLURE1`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectBlureInit1 {
    pub p1_start_color: [u8; 4],
    pub p2_start_color: [u8; 4],
    pub p1_end_color: [u8; 4],
    pub p2_end_color: [u8; 4],
    pub elem_duration: i32,
    pub unk_flag: i32,
    pub calc_mode: i32,
}

/// `EffectBlureInit2` (`EFFECT_BLURE2`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectBlureInit2 {
    pub calc_mode: i32,
    pub flags: u16,
    pub add_angle_change: i16,
    pub p1_start_color: [u8; 4],
    pub p2_start_color: [u8; 4],
    pub p1_end_color: [u8; 4],
    pub p2_end_color: [u8; 4],
    pub elem_duration: u8,
    pub unk_flag: u8,
    /// 0: simple; 1: simple with the alternate colours (textured); 2 and up: smooth.
    pub draw_mode: u8,
    pub mode4_param: u8,
    pub alt_prim_color: [u8; 4],
    pub alt_env_color: [u8; 4],
}

/// Which init `Effect_Add` runs (`EFFECT_BLURE1`, `EFFECT_BLURE2`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EffectBlureInit {
    One(EffectBlureInit1),
    Two(EffectBlureInit2),
}

/// `EffectBlure`.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectBlure {
    pub elements: [EffectBlureElement; ELEMENT_COUNT],
    pub calc_mode: i32,
    pub mode4_param: f32,
    pub flags: u16,
    pub add_angle_change: i16,
    pub add_angle: i16,
    pub p1_start_color: [u8; 4],
    pub p2_start_color: [u8; 4],
    pub p1_end_color: [u8; 4],
    pub p2_end_color: [u8; 4],
    /// `numElements` ("now_edge_num").
    pub num_elements: u8,
    pub elem_duration: u8,
    pub unk_flag: u8,
    pub draw_mode: u8,
    pub alt_prim_color: [u8; 4],
    pub alt_env_color: [u8; 4],
}

/// `EffectBlure_Init1` or `EffectBlure_Init2`, after `EffectBlure_InitElements`.
pub fn init(p: &EffectBlureInit) -> EffectBlure {
    let elements = [EffectBlureElement::UNUSED; ELEMENT_COUNT];
    match *p {
        EffectBlureInit::One(p) => EffectBlure {
            elements,
            p1_start_color: p.p1_start_color,
            p2_start_color: p.p2_start_color,
            p1_end_color: p.p1_end_color,
            p2_end_color: p.p2_end_color,
            // elemDuration and unkFlag are s32 in the init, u8 in the effect.
            elem_duration: p.elem_duration as u8,
            unk_flag: p.unk_flag as u8,
            calc_mode: p.calc_mode,
            flags: 0,
            add_angle_change: 0,
            add_angle: 0,
            draw_mode: 0,
            alt_prim_color: [0; 4],
            alt_env_color: [0; 4],
            mode4_param: 1.0,
            num_elements: 0,
        },
        EffectBlureInit::Two(p) => EffectBlure {
            elements,
            p1_start_color: p.p1_start_color,
            p2_start_color: p.p2_start_color,
            p1_end_color: p.p1_end_color,
            p2_end_color: p.p2_end_color,
            elem_duration: p.elem_duration,
            unk_flag: p.unk_flag,
            calc_mode: p.calc_mode,
            flags: p.flags,
            draw_mode: p.draw_mode,
            add_angle_change: p.add_angle_change,
            add_angle: 0,
            mode4_param: p.mode4_param as f32,
            alt_prim_color: p.alt_prim_color,
            alt_env_color: p.alt_env_color,
            num_elements: 0,
        },
    }
}

/// A float into a `Vec3s` field: truncated, then the low 16 bits.
fn s16(v: f32) -> i16 {
    v as i32 as i16
}

fn s16v(v: Vec3) -> [i16; 3] {
    [s16(v.x), s16(v.y), s16(v.z)]
}

fn f32v(v: [i16; 3]) -> Vec3 {
    Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)
}

impl EffectBlure {
    /// `EffectBlure_AddVertex`: an edge from `p1` to `p2`; with `flags` 2, turned by `addAngle`
    /// about the line from the last edge's middle to this one's.
    pub fn add_vertex(&mut self, p1: Vec3, p2: Vec3) {
        let n = self.num_elements as usize;
        if n >= ELEMENT_COUNT {
            log::debug!("Blur - vertex addition processing: Table over {n}");
            return;
        }
        self.elements[n].state = 1;
        if self.flags & 2 == 0 {
            self.elements[n].p1 = s16v(p1);
            self.elements[n].p2 = s16v(p2);
        } else {
            // @bug (game): with no edge yet this reads the element before the table (the
            // port takes zeros).
            let prev = if n == 0 { EffectBlureElement { state: 0, timer: 0, p1: [0; 3], p2: [0; 3], flags: 0 } } else { self.elements[n - 1] };
            let sp16c = Vec3::new(
                (prev.p2[0] as f32 + prev.p1[0] as f32) * 0.5,
                (prev.p2[1] as f32 + prev.p1[1] as f32) * 0.5,
                (prev.p2[2] as f32 + prev.p1[2] as f32) * 0.5,
            );
            let sp160 = Vec3::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5, (p1.z + p2.z) * 0.5);
            let mut sp154 = sp160 - sp16c;
            let scale = (sp154.x * sp154.x + sp154.y * sp154.y + sp154.z * sp154.z).sqrt();
            // Else the edge keeps whatever its slot held.
            if scale.abs() >= 0.008 {
                let scale = 1.0 / scale;
                sp154 *= scale;
                let sp110 = skin::set_translate(sp160.x, sp160.y, sp160.z);
                let spd0 = skin::set_rotate_axis(self.add_angle, sp154.x, sp154.y, sp154.z);
                let sp90 = skin::mult(&sp110, &spd0);
                let sp110 = skin::set_translate(-sp160.x, -sp160.y, -sp160.z);
                let sp50 = skin::mult(&sp90, &sp110);
                self.elements[n].p1 = s16v(skin::mult_xyz(&sp50, p1));
                self.elements[n].p2 = s16v(skin::mult_xyz(&sp50, p2));
            }
        }
        self.elements[n].timer = 1;
        self.num_elements += 1;
    }

    /// `EffectBlure_AddSpace`: a gap in the trail.
    pub fn add_space(&mut self) {
        let n = self.num_elements as usize;
        if n >= ELEMENT_COUNT {
            log::debug!("Blur - space addition processing: Table over {n}");
            return;
        }
        self.elements[n].state = 0;
        self.elements[n].timer = 1;
        self.num_elements += 1;
    }

    /// The table moved down a slot, the last emptied.
    fn shift(&mut self) {
        for i in 0..ELEMENT_COUNT - 1 {
            self.elements[i] = self.elements[i + 1];
        }
        self.elements[ELEMENT_COUNT - 1] = EffectBlureElement::UNUSED;
    }

    /// `EffectBlure_GetComputedValues`: element `index`'s two points and colours `ratio` of the
    /// way through its life, by `calcMode`.
    pub fn computed_values(&self, index: usize, ratio: f32) -> ([i16; 3], [i16; 3], [u8; 4], [u8; 4]) {
        let e = &self.elements[index];
        let (vec1, vec2) = match self.calc_mode {
            1 => (std::array::from_fn(|k| lerp_s16(e.p1[k], e.p2[k], ratio)), e.p2),
            2 => (e.p1, std::array::from_fn(|k| lerp_s16(e.p2[k], e.p1[k], ratio))),
            3 => {
                let r = ratio * 0.5;
                (std::array::from_fn(|k| lerp_s16(e.p1[k], e.p2[k], r)), std::array::from_fn(|k| lerp_s16(e.p2[k], e.p1[k], r)))
            }
            4 => {
                // sp30 is a Vec3s: the difference in 16 bits.
                let sp30: [i16; 3] = std::array::from_fn(|k| e.p1[k].wrapping_sub(e.p2[k]));
                let mode4_param = self.mode4_param - 1.0;
                (
                    std::array::from_fn(|k| s16((sp30[k] as f32 * 0.5 * mode4_param * ratio) + e.p1[k] as f32)),
                    std::array::from_fn(|k| s16(-(sp30[k] as f32 * 0.5 * mode4_param * ratio) + e.p2[k] as f32)),
                )
            }
            _ => (e.p1, e.p2),
        };
        let (c1, c2) = if self.flags & 0x10 != 0 {
            ([255; 4], [255; 4])
        } else {
            (
                std::array::from_fn(|k| lerp_u8(self.p1_start_color[k], self.p1_end_color[k], ratio)),
                std::array::from_fn(|k| lerp_u8(self.p2_start_color[k], self.p2_end_color[k], ratio)),
            )
        };
        (vec1, vec2, c1, c2)
    }

    /// Element `i`'s `timer / elemDuration`.
    fn ratio(&self, i: usize) -> f32 {
        self.elements[i].timer as f32 / self.elem_duration as f32
    }
}

/// `EffectBlure_Update`: gaps at the front dropped; each element a frame older, and the oldest
/// dropped past `elemDuration`; `addAngle` turned on. Never done (always 0).
pub fn update(this: &mut EffectBlure) -> bool {
    if this.num_elements == 0 {
        return false;
    }
    while this.elements[0].state == 0 {
        this.shift();
        this.num_elements -= 1;
        if this.num_elements == 0 {
            return false;
        }
    }
    if this.elements[0].state == 2 {
        return false;
    }
    for i in 0..this.num_elements as usize {
        this.elements[i].timer += 1;
    }
    if this.elem_duration as i32 > this.elements[0].timer || this.elem_duration as i32 == this.elements[0].timer {
        this.add_angle = this.add_angle.wrapping_add(this.add_angle_change);
        return false;
    }
    this.shift();
    this.num_elements -= 1;
    false
}

/// `Math3D_CosOut`: the cosine of the angle between `a` and `b`, or `None` (0, true) when
/// either is zero.
fn cos_out(a: Vec3, b: Vec3) -> Option<f32> {
    let mag = (a.x * a.x + a.y * a.y + a.z * a.z).sqrt() * (b.x * b.x + b.y * b.y + b.z * b.z).sqrt();
    if mag.abs() < 0.008 {
        return None;
    }
    Some(((a.x * b.x) + (a.y * b.y) + (a.z * b.z)) / mag)
}

/// `EffectBlure_UpdateFlags`: element `i`'s bend class (bits 0..1): 2 next to a gap, 1 where
/// both edges and the quad turn smoothly (a Hermite curve), else 0.
fn update_flags(elems: &[EffectBlureElement; ELEMENT_COUNT], flags: u16, i: usize) -> u16 {
    let (prev, e, next) = (&elems[i - 1], &elems[i], &elems[i + 1]);
    let class = if prev.state == 0 || next.state == 0 {
        2
    } else {
        let d = |a: [i16; 3], b: [i16; 3]| f32v(a) - f32v(b);
        let sp64 = d(e.p1, prev.p1);
        let sp58 = d(e.p2, prev.p2);
        let sp4c = d(next.p1, e.p1);
        let sp40 = d(next.p2, e.p2);
        match (cos_out(sp64, sp4c), cos_out(sp58, sp40), cos_out(sp4c, sp40)) {
            (Some(a), Some(b), Some(c)) if !(a <= -0.5 || b <= -0.5 || c <= 0.7071) => 1,
            // Math3D_CosOut's || stops at the first zero length.
            _ => 0,
        }
    };
    (flags & !3) | class
}

/// `Math_FNearbyIntF`: rounded to the nearest, halves to even.
fn nearby_int(v: f32) -> f32 {
    v.round_ties_even()
}

/// The RSP's 0.1 in s15.16 (`SkinMatrix_MtxFToMtx` truncates `0.1 * 0x10000`).
const SCALE_TENTH: f32 = 6553.0 / 65536.0;

/// What a draw puts out: the bones' places and the triangle corners' colours of one bake.
struct Shape {
    bones: Vec<Vec3>,
    colors: Vec<[u8; 4]>,
}

impl Shape {
    fn new(bones: usize, tris: usize) -> Shape {
        Shape { bones: vec![Vec3::ZERO; bones], colors: vec![[0; 4]; tris * 3] }
    }

    fn cmd(self, bake: &str) -> eng_gfx::DrawCmd {
        let mut cmd = eng_gfx::DrawCmd::new(eng_gfx::MeshKey::named(keys::bake(bake)), Mat4::IDENTITY);
        cmd.bones = self.bones.into_iter().map(Mat4::from_translation).collect();
        cmd.params.vertex_colors = Some(self.colors);
        cmd
    }
}

/// `EffectBlure_Draw`: the trail into the XLU list, by `flags` and `drawMode`.
pub fn draw(this: &EffectBlure, out: &mut EffectDraws) {
    // gSPMatrix(gIdentityMtx): world coordinates.
    if this.num_elements == 0 {
        return;
    }
    if this.flags == 0 {
        draw_strip(this, out);
    } else if this.draw_mode < 2 {
        draw_simple(this, out);
    } else {
        draw_smooth(this, out);
    }
}

/// `EffectBlure_Draw`'s own strip (`flags` 0): `Gfx_SetupDL_38Xlu`, the edges in order, a quad
/// between each two not parted by a gap (every other one with `unkFlag` 1).
fn draw_strip(this: &EffectBlure, out: &mut EffectDraws) {
    let n = this.num_elements as usize;
    let mut slots: Vec<(Vec3, [u8; 4])> = vec![(Vec3::ZERO, [0; 4]); STRIP_SLOTS];
    let mut j = 0;
    for i in 0..n {
        let e = &this.elements[i];
        if e.state != 1 {
            continue;
        }
        let ratio = this.ratio(i);
        // As computed_values but for mode 4 (the strip's switch has no case 4).
        let (v1, v2): ([i16; 3], [i16; 3]) = match this.calc_mode {
            1 => (std::array::from_fn(|k| lerp_s16(e.p1[k], e.p2[k], ratio)), e.p2),
            2 => (e.p1, std::array::from_fn(|k| lerp_s16(e.p2[k], e.p1[k], ratio))),
            3 => {
                let r = ratio * 0.5;
                (std::array::from_fn(|k| lerp_s16(e.p1[k], e.p2[k], r)), std::array::from_fn(|k| lerp_s16(e.p2[k], e.p1[k], r)))
            }
            _ => (e.p1, e.p2),
        };
        let c1 = std::array::from_fn(|k| lerp_u8(this.p1_start_color[k], this.p1_end_color[k], ratio));
        let c2 = std::array::from_fn(|k| lerp_u8(this.p2_start_color[k], this.p2_end_color[k], ratio));
        slots[j] = (f32v(v1), c1);
        slots[j + 1] = (f32v(v2), c2);
        j += 2;
    }
    let mut s = Shape::new(STRIP_SLOTS, (STRIP_SLOTS / 2 - 1) * 2);
    for (k, slot) in slots.iter().enumerate() {
        s.bones[k] = slot.0;
    }
    let (mut flag, mut j) = (false, 0);
    for i in 0..n {
        if this.elements[i].state == 0 {
            flag = false;
            continue;
        }
        if !flag {
            flag = true;
        } else {
            // gSP1Quadrangle(j - 2, j - 1, j + 1, j, 0): (j-2, j-1, j+1), (j-2, j+1, j).
            let q = j / 2 - 1;
            for (c, v) in [j - 2, j - 1, j + 1, j - 2, j + 1, j].into_iter().enumerate() {
                s.colors[q * 6 + c] = slots[v].1;
            }
            if this.unk_flag == 1 {
                flag = false;
            }
        }
        j += 2;
    }
    out.xlu.push(s.cmd(BAKE_STRIP));
}

/// `EffectBlure_DrawSmooth`: the bend classes, then each quad between two edges either straight
/// (`EffectBlure_DrawElemNoInterpolation`) or a Hermite curve of seven
/// (`EffectBlure_DrawElemHermiteInterpolation`), from the first edge's second point scaled by
/// 0.1.
fn draw_smooth(this: &EffectBlure, out: &mut EffectDraws) {
    let n = this.num_elements as usize;
    if n < 2 {
        return;
    }
    // The bend classes (written into the elements by the C; nothing but this draw reads them).
    let mut flags: [u16; ELEMENT_COUNT] = std::array::from_fn(|i| this.elements[i].flags);
    flags[0] = (flags[0] & !3) | 2;
    for i in 1..n - 1 {
        flags[i] = update_flags(&this.elements, flags[i], i);
    }
    flags[n - 1] = (flags[n - 1] & !3) | 2;
    let base = f32v(this.elements[0].p2);
    let place = |ob: [i16; 3]| base + f32v(ob) * SCALE_TENTH;
    let segments = ELEMENT_COUNT - 1;
    let mut s = Shape::new(segments * SMOOTH_SLOTS, segments * SMOOTH_TRIS);
    for b in s.bones.iter_mut() {
        *b = base;
    }
    let class = |i: usize| flags[i] & 3;
    for i in 0..n - 1 {
        if this.elements[i].state == 0 || this.elements[i + 1].state == 0 {
            continue;
        }
        let (a, b) = (class(i), class(i + 1));
        let bone0 = i * SMOOTH_SLOTS;
        let tri0 = i * SMOOTH_TRIS;
        if (a == 0 || a == 2) && (b == 0 || b == 2) {
            // EffectBlure_DrawElemNoInterpolation: (v2, v1) of this edge, (v1, v2) of the next,
            // each (p - base) * 10 into the Vtx (truncated); triangles (0, 1, 2), (0, 2, 3).
            let ob = |v: [i16; 3]| s16v((f32v(v) - base) * 10.0);
            let (v1, v2, c1, c2) = this.computed_values(i, this.ratio(i));
            let (w1, w2, d1, d2) = this.computed_values(i + 1, this.ratio(i + 1));
            let vtx = [(ob(v2), c2), (ob(v1), c1), (ob(w1), d1), (ob(w2), d2)];
            for (k, v) in vtx.iter().enumerate() {
                s.bones[bone0 + k] = place(v.0);
            }
            for (c, k) in [0, 1, 2, 0, 2, 3].into_iter().enumerate() {
                s.colors[tri0 * 3 + c] = vtx[k].1;
            }
        } else {
            let vtx = hermite(this, i, &flags, base);
            for (k, v) in vtx.iter().enumerate() {
                s.bones[bone0 + k] = place(v.0);
            }
            for (t, tri) in HERMITE_TRIS.iter().enumerate() {
                for (c, &k) in tri.iter().enumerate() {
                    s.colors[(tri0 + 2 + t) * 3 + c] = vtx[k as usize].1;
                }
            }
        }
    }
    out.xlu.push(s.cmd(BAKE_SMOOTH));
}

/// `EffectBlure_DrawElemHermiteInterpolation`'s seven quads: (0, 1, 3), (0, 3, 2), (2, 3, 5),
/// ... (12, 15, 14).
const HERMITE_TRIS: [[u32; 3]; 14] =
    [[0, 1, 3], [0, 3, 2], [2, 3, 5], [2, 5, 4], [4, 5, 7], [4, 7, 6], [6, 7, 9], [6, 9, 8], [8, 9, 11], [8, 11, 10], [10, 11, 13], [10, 13, 12], [12, 13, 15], [12, 15, 14]];

/// `EffectBlure_DrawElemHermiteInterpolation`'s sixteen vertices for the quad from element `i`
/// to `i + 1`: their `ob` ((p - base) * 10, rounded) and colours.
fn hermite(this: &EffectBlure, i: usize, flags: &[u16; ELEMENT_COUNT], base: Vec3) -> [([i16; 3], [u8; 4]); SMOOTH_SLOTS] {
    let (v, w, sp1a4, sp1a0) = this.computed_values(i, this.ratio(i));
    let (sp1cc, sp1c0) = (f32v(v), f32v(w));
    let (v, w, sp19c, sp198) = this.computed_values(i + 1, this.ratio(i + 1));
    let (sp18c, sp180) = (f32v(v), f32v(w));
    let (mut sp1b4, mut sp1a8) = if flags[i] & 3 == 2 {
        (sp18c - sp1cc, sp180 - sp1c0)
    } else {
        let (v, w, _, _) = this.computed_values(i - 1, this.ratio(i - 1));
        (sp18c - f32v(v), sp180 - f32v(w))
    };
    sp1b4 *= 0.5;
    sp1a8 *= 0.5;
    let (mut sp174, mut sp168) = if flags[i + 1] & 3 == 2 {
        (sp18c - sp1cc, sp180 - sp1c0)
    } else {
        let (v, w, _, _) = this.computed_values(i + 2, this.ratio(i + 2));
        (f32v(v) - sp1cc, f32v(w) - sp1c0)
    };
    sp174 *= 0.5;
    sp168 *= 0.5;
    let ob = |p: Vec3| {
        let d = (p - base) * 10.0;
        [nearby_int(d.x) as i32 as i16, nearby_int(d.y) as i32 as i16, nearby_int(d.z) as i32 as i16]
    };
    let mut vtx = [([0i16; 3], [255u8; 4]); SMOOTH_SLOTS];
    vtx[0] = (ob(sp1cc), sp1a4);
    vtx[1] = (ob(sp1c0), sp1a0);
    for k in 1..8 {
        let t = k as f32 / 7.0;
        let t2 = t * t;
        let t3 = t2 * t;
        let f20 = t3 - t2;
        let f22 = t3 - 2.0 * t2 + t;
        let f24 = 2.0 * t3 - t2 * 3.0 + 1.0;
        let f26 = t2 * 3.0 - 2.0 * t3;
        let curve = |p0: Vec3, p1: Vec3, m0: Vec3, m1: Vec3| {
            Vec3::new(
                (f24 * p0.x) + (f26 * p1.x) + (f22 * m0.x) + (f20 * m1.x),
                (f24 * p0.y) + (f26 * p1.y) + (f22 * m0.y) + (f20 * m1.y),
                (f24 * p0.z) + (f26 * p1.z) + (f22 * m0.z) + (f20 * m1.z),
            )
        };
        let lerp = |a: [u8; 4], b: [u8; 4]| std::array::from_fn(|c| lerp_u8(a[c], b[c], t));
        vtx[2 * k] = (ob(curve(sp1cc, sp18c, sp1b4, sp174)), lerp(sp1a4, sp19c));
        vtx[2 * k + 1] = (ob(curve(sp1c0, sp180, sp1a8, sp168)), lerp(sp1a0, sp198));
    }
    vtx
}

/// `EffectBlure_DrawSimple`: four vertices an edge pair, a quad (0, 1, 3), (0, 3, 2) between
/// each edge and the next, by `EffectBlure_DrawSimpleVertices`: drawMode 1's prim alpha fading
/// with each quad's first edge's age, and with `flags` 4 the quad again turned a quarter about
/// the line between its edges' middles.
fn draw_simple(this: &EffectBlure, out: &mut EffectDraws) {
    let n = this.num_elements as usize;
    if n < 2 {
        return;
    }
    // vtx[4i] and [4i + 1] are element i's points; [4i + 2] and [4i + 3] element i + 1's.
    let vals: Vec<_> = (0..n).map(|i| this.computed_values(i, this.ratio(i))).collect();
    for i in 0..n - 1 {
        let (a, b) = (&vals[i], &vals[i + 1]);
        let vtx = [(f32v(a.0), a.2), (f32v(a.1), a.3), (f32v(b.0), b.2), (f32v(b.1), b.3)];
        let bake = match (this.draw_mode, i) {
            (1, 0) => BAKE_SIMPLE_ALT_FIRST,
            (1, _) => BAKE_SIMPLE_ALT,
            _ => BAKE_SIMPLE,
        };
        let colors = |s: &mut Shape| {
            for (c, k) in [0, 1, 3, 0, 3, 2].into_iter().enumerate() {
                s.colors[c] = vtx[k].1;
            }
        };
        let prim = (this.draw_mode == 1).then(|| {
            let alpha_ratio = this.elements[i].timer as f32 / this.elem_duration as f32;
            let p = this.alt_prim_color;
            [p[0], p[1], p[2], (p[3] as f32 * (1.0 - alpha_ratio)) as i32 as u8]
        });
        let push = |out: &mut EffectDraws, places: [Vec3; 4]| {
            let mut s = Shape::new(4, 2);
            s.bones.copy_from_slice(&places);
            colors(&mut s);
            let mut cmd = s.cmd(bake);
            if let Some(prim) = prim {
                let mut sv = eng_gfx::SegmentValues::default();
                sv.prim[SEG_COLOR as usize] = Some(prim);
                sv.env[SEG_COLOR as usize] = Some(this.alt_env_color);
                cmd.params.segments = Some(sv);
            }
            out.xlu.push(cmd);
        };
        // The Vtx ob are s16s.
        let places = vtx.map(|v| f32v(s16v(v.0)));
        push(out, places);
        if this.flags & 4 != 0 {
            let sp1b0 = (places[0] + places[1]) * 0.5;
            let sp1a4 = (places[2] + places[3]) * 0.5;
            let mut sp198 = sp1a4 - sp1b0;
            let scale = (sp198.x * sp198.x + sp198.y * sp198.y + sp198.z * sp198.z).sqrt();
            if scale.abs() > 0.0005 {
                sp198 *= 1.0 / scale;
                let sp154 = skin::set_translate(sp1b0.x, sp1b0.y, sp1b0.z);
                let sp114 = skin::set_rotate_axis(0x3FFF, sp198.x, sp198.y, sp198.z);
                let spd4 = skin::mult(&sp154, &sp114);
                let sp154 = skin::set_translate(-sp1b0.x, -sp1b0.y, -sp1b0.z);
                let sp94: MtxF = skin::mult(&spd4, &sp154);
                // The RSP's fixed-point matrix: the turn is on the same Vtx.
                push(out, places.map(|p| skin::mult_xyz(&sp94, p)));
            }
        }
    }
}

/// A list of `n` vertices at the origin, each loaded under its own matrix of the segment's
/// (`gSPMatrix(G_MTX_NOPUSH | G_MTX_LOAD | G_MTX_MODELVIEW)` then `gSPVertex(1)` into slot
/// `slot0 + k`), so each follows its bone.
fn load_bones(d: &mut Dl, first_bone: u32, n: u32, slot0: u32) {
    for k in 0..n {
        d.0.push((0xDA38_0003, seg(SEG_MATRICES as u32, (first_bone + k) * 0x40)));
        d.vertex(seg(SEG_DATA as u32, 0), 1, slot0 + k);
    }
}

fn bake(name: &str, bones: u32, vtx: Vec<u8>, extra: Vec<(u8, BakeSegment)>, setup: Dl, body: Dl) -> MeshBake {
    let mut d = setup;
    d.extend(&body.0);
    d.end();
    let mut segments = vec![(SEG_DATA, BakeSegment::Bytes(vtx)), (SEG_MATRICES, BakeSegment::Matrices((0..bones as u16).collect())), (SEG_SETUP, BakeSegment::Commands(d.0))];
    segments.extend(extra);
    MeshBake { name: name.into(), object: "gameplay_keep".into(), segments, prelude: vec![SEG_SETUP], body: BakeBody::DLists(Vec::new()) }
}

/// `Gfx_SetupDL_38Xlu` (`EffectBlure_Draw`'s strip, `EffectBlure_SetupSmooth`,
/// `EffectBlure_SetupSimple`), then `gDPPipeSync`.
fn setup_38() -> Dl {
    let mut d = setup_dl::setup_dl_38();
    d.pipe_sync();
    d
}

/// `EffectBlure_SetupSimpleAlt`: `SETUPDL_38` in two cycles, `gUnknownEffBlureTex` (I8, 64 × 32,
/// clamped across, wrapping down), the combiner `TEXEL0, PRIMITIVE, PRIM_LOD_FRAC, TEXEL0,
/// TEXEL0, 0, PRIMITIVE, 0, PRIMITIVE, ENVIRONMENT, COMBINED, ENVIRONMENT, 0, 0, 0, COMBINED`,
/// `G_RM_PASS, G_RM_ZB_CLD_SURF2`; the env colour and each quad's prim (lod fraction 0x80)
/// the draw's.
fn setup_simple_alt() -> Dl {
    let mut d = Dl::default();
    d.pipe_sync();
    d.extend(&setup_dl::setup_dl_38().0);
    d.cycle_type(G_CYC_2CYCLE);
    // gDPSetTextureLUT(G_TT_NONE): G_SETOTHERMODE_H at G_MDSFT_TEXTLUT (14), 2 bits, 0.
    d.othermode_h(14, 2, 0);
    d.texture_on(true);
    d.load_texture_block(seg(SEG_TEX as u32, 0), G_IM_FMT_I, G_IM_SIZ_8B, 64, 32, 0, G_TX_NOMIRROR | G_TX_CLAMP, G_TX_NOMIRROR | G_TX_WRAP, 6, 5, G_TX_NOLOD, G_TX_NOLOD);
    let c0 = [cc_ab::TEXEL0, cc_ab::PRIMITIVE, cc_c::PRIM_LOD_FRAC, cc_d::TEXEL0, ac::TEXEL0, ac::ZERO, ac::PRIMITIVE, ac::ZERO];
    let c1 = [cc_ab::PRIMITIVE, cc_ab::ENVIRONMENT, cc_c::COMBINED, cc_d::ENVIRONMENT, ac::ZERO, ac::ZERO, ac::ZERO, ac::COMBINED];
    d.combine_lerp(c0, c1);
    d.render_mode(G_RM_PASS, G_RM_ZB_CLD_SURF2);
    d.clear_geometry_mode(G_FOG | G_LIGHTING | G_TEXTURE_GEN | G_TEXTURE_GEN_LINEAR);
    d.set_geometry_mode(G_ZBUFFER | G_SHADE | G_SHADING_SMOOTH);
    d.pipe_sync();
    // The env colour, then (per quad, EffectBlure_DrawSimpleVertices) gDPSetPrimColor(0, 0x80)
    // and gDPPipeSync: the draw's.
    d.display_list(seg(SEG_COLOR as u32, 0));
    d.pipe_sync();
    d
}

/// One vertex record at the origin (`ob` 0) with texture coordinates `tc`, white.
fn origin_vtx(out: &mut Vec<u8>, tc: [i32; 2]) {
    push_vtx(out, [0, 0, 0], tc, [255; 4]);
}

/// The trails' bakes.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    // The strip: 32 slots, the 15 possible quads.
    {
        let mut vtx = Vec::new();
        origin_vtx(&mut vtx, [0, 0]);
        let mut d = Dl::default();
        load_bones(&mut d, 0, STRIP_SLOTS as u32, 0);
        for q in 0..(STRIP_SLOTS as u32 / 2 - 1) {
            let j = 2 * (q + 1);
            d.quad0(j - 2, j - 1, j + 1, j);
        }
        v.push(bake(BAKE_STRIP, STRIP_SLOTS as u32, vtx, Vec::new(), setup_38(), d));
    }
    // The smooth one: per quad 16 slots, the straight quad's two triangles then the curve's 14.
    {
        let mut vtx = Vec::new();
        origin_vtx(&mut vtx, [0, 0]);
        let mut d = Dl::default();
        for q in 0..(ELEMENT_COUNT as u32 - 1) {
            load_bones(&mut d, q * SMOOTH_SLOTS as u32, SMOOTH_SLOTS as u32, 0);
            d.tri2([0, 1, 2], [0, 2, 3]);
            for t in HERMITE_TRIS.chunks(2) {
                d.tri2(t[0], t[1]);
            }
        }
        v.push(bake(BAKE_SMOOTH, (ELEMENT_COUNT as u32 - 1) * SMOOTH_SLOTS as u32, vtx, Vec::new(), setup_38(), d));
    }
    // The simple quads: one quad of four slots (0, 1, 3), (0, 3, 2); textured the alternate's,
    // its first quad with D_8011578C's texture coordinates, the others with D_801157CC's.
    let quad = |tcs: [[i32; 2]; 4]| {
        let mut vtx = Vec::new();
        for tc in tcs {
            origin_vtx(&mut vtx, tc);
        }
        let mut d = Dl::default();
        for k in 0..4 {
            d.0.push((0xDA38_0003, seg(SEG_MATRICES as u32, k * 0x40)));
            d.vertex(seg(SEG_DATA as u32, k * 16), 1, k);
        }
        d.tri2([0, 1, 3], [0, 3, 2]);
        (vtx, d)
    };
    let (vtx, d) = quad([[0, 0]; 4]);
    v.push(bake(BAKE_SIMPLE, 4, vtx, Vec::new(), setup_38(), d));
    let tex = || (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: "gUnknownEffBlureTex".into() });
    let color = || {
        let mut c = Dl::default();
        c.0.push((0xFA00_0080, 0x0000_00FF));
        c.env_color([0, 0, 0, 255]);
        c.end();
        (SEG_COLOR, BakeSegment::Dynamic(c.0))
    };
    let (vtx, d) = quad([[0, 1024], [0, 0], [2048, 1024], [2048, 0]]);
    v.push(bake(BAKE_SIMPLE_ALT_FIRST, 4, vtx, vec![tex(), color()], setup_simple_alt(), d));
    let (vtx, d) = quad([[2048, 1024], [2048, 0], [2048, 1024], [2048, 0]]);
    v.push(bake(BAKE_SIMPLE_ALT, 4, vtx, vec![tex(), color()], setup_simple_alt(), d));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sword() -> EffectBlure {
        // Player's D_8085470C: calcMode 0, flags 8, white from 255/64 to 0, 4 frames, drawMode 2.
        init(&EffectBlureInit::Two(EffectBlureInit2 {
            calc_mode: 0,
            flags: 8,
            add_angle_change: 0,
            p1_start_color: [255, 255, 255, 255],
            p2_start_color: [255, 255, 255, 64],
            p1_end_color: [255, 255, 255, 0],
            p2_end_color: [255, 255, 255, 0],
            elem_duration: 4,
            unk_flag: 0,
            draw_mode: 2,
            mode4_param: 0,
            alt_prim_color: [0; 4],
            alt_env_color: [0; 4],
        }))
    }

    #[test]
    fn edges_age_and_drop_after_their_duration() {
        let mut b = sword();
        b.add_vertex(Vec3::new(10.7, -3.2, 5.0), Vec3::new(-1.5, 2.0, 3.0));
        // The C stores the floats truncated into the Vec3s.
        assert_eq!((b.elements[0].p1, b.elements[0].p2, b.elements[0].timer, b.num_elements), ([10, -3, 5], [-1, 2, 3], 1, 1));
        b.add_space();
        b.add_vertex(Vec3::ZERO, Vec3::X);
        // Timer 1 at the add; +1 each update; dropped when past elemDuration (4): at timer 5.
        for t in 2..=4 {
            update(&mut b);
            assert_eq!((b.elements[0].timer, b.num_elements), (t, 3));
        }
        update(&mut b);
        // The first edge dropped; the gap now in front is dropped by the next update.
        assert_eq!((b.num_elements, b.elements[0].state, b.elements[2].state), (2, 0, 2));
        // The same update ages the edge now in front (timer 6) and drops it too.
        update(&mut b);
        assert_eq!(b.num_elements, 0);
        // Past 16 an add is refused.
        let mut b = sword();
        for _ in 0..20 {
            b.add_space();
        }
        assert_eq!(b.num_elements, 16);
    }

    #[test]
    fn computed_values_by_calc_mode() {
        let mut b = sword();
        b.add_vertex(Vec3::new(0.0, 0.0, 0.0), Vec3::new(100.0, 0.0, 0.0));
        // Mode 0: the points as they are; the colours a quarter of the way at ratio 0.25.
        assert_eq!(b.computed_values(0, 0.25), ([0, 0, 0], [100, 0, 0], [255, 255, 255, 191], [255, 255, 255, 48]));
        b.calc_mode = 1;
        assert_eq!(b.computed_values(0, 0.25).0, [25, 0, 0]);
        b.calc_mode = 2;
        assert_eq!(b.computed_values(0, 0.25).1, [75, 0, 0]);
        b.calc_mode = 3;
        // Mode 3 halves the ratio for the points, not the colours.
        let (v1, v2, c1, _) = b.computed_values(0, 0.5);
        assert_eq!((v1, v2, c1), ([25, 0, 0], [75, 0, 0], [255, 255, 255, 127]));
        b.calc_mode = 4;
        b.mode4_param = 3.0;
        // sp30 = p1 - p2 = -100; (mode4Param - 1) = 2: p1 + (-100 * 0.5 * 2 * 0.5) = -50.
        assert_eq!(b.computed_values(0, 0.5).0, [-50, 0, 0]);
        b.flags = 0x10;
        assert_eq!(b.computed_values(0, 0.5).2, [255; 4]);
    }

    #[test]
    fn a_smooth_trail_curves_and_a_reversing_one_draws_straight_quads() {
        let close = |a: Vec3, b: Vec3| (a - b).length() < 0.01;
        // A straight sweep: the middle edges' points move in parallel and on (class 1), so every
        // quad is a Hermite curve (the straight quad's two triangles hidden).
        let mut b = sword();
        for k in 0..4 {
            b.add_vertex(Vec3::new(k as f32 * 20.0, 0.0, 0.0), Vec3::new(k as f32 * 20.0, 50.0, 0.0));
        }
        let mut out = EffectDraws::default();
        draw(&b, &mut out);
        assert_eq!(out.xlu.len(), 1);
        let c = out.xlu[0].params.vertex_colors.as_ref().unwrap();
        for q in 0..3 {
            assert!(c[q * 48..q * 48 + 6].iter().all(|c| c[3] == 0), "quad {q}");
            assert!(c[q * 48 + 6..q * 48 + 48].iter().all(|c| c[3] > 0), "quad {q}");
        }
        // The curve's points stay on the sweep's lines (y 0 and 50), between the edges.
        let bones = &out.xlu[0].bones;
        let p = |k: usize| bones[k].w_axis.truncate();
        for k in 0..16 {
            let v = p(16 + k);
            assert!((v.y - if k % 2 == 0 { 0.0 } else { 50.0 }).abs() < 0.01 && v.x >= 19.99 && v.x <= 40.01, "{k}: {v:?}");
        }
        // A trail that turns back (element 1 and 2 move against the edges before: cos -1): class
        // 0, and every quad straight.
        let mut b = sword();
        for x in [0.0, 20.0, 0.0, 20.0] {
            b.add_vertex(Vec3::new(x, 0.0, 0.0), Vec3::new(x, 50.0, 0.0));
        }
        let mut out = EffectDraws::default();
        draw(&b, &mut out);
        let c = out.xlu[0].params.vertex_colors.as_ref().unwrap();
        for q in 0..3 {
            assert!(c[q * 48..q * 48 + 6].iter().all(|c| c[3] > 0), "quad {q}");
            assert!(c[q * 48 + 6..q * 48 + 48].iter().all(|c| c[3] == 0), "quad {q}");
        }
        // The first quad's corners: (v2, v1) of edge 0, (v1, v2) of edge 1, at base + ob * 0.1 in
        // s15.16 (base edge 0's p2, (0, 50, 0)).
        let bones = &out.xlu[0].bones;
        let p = |k: usize| bones[k].w_axis.truncate();
        assert_eq!(p(0), Vec3::new(0.0, 50.0, 0.0));
        assert!(close(p(1), Vec3::new(0.0, 0.0, 0.0)), "{:?}", p(1));
        assert!(close(p(2), Vec3::new(20.0, 0.0, 0.0)), "{:?}", p(2));
        assert!(close(p(3), Vec3::new(20.0, 50.0, 0.0)), "{:?}", p(3));
        // Colours a quarter of the way (timer 1 of 4): p2's alpha 64 to 48, p1's 255 to 191.
        assert_eq!((c[0], c[1]), ([255, 255, 255, 48], [255, 255, 255, 191]));
    }

    #[test]
    fn the_strip_joins_edges_not_parted_by_a_gap() {
        // En_St's EFFECT_BLURE1: white 75 to 0 alpha, 6 frames, calcMode 3.
        let mut b = init(&EffectBlureInit::One(EffectBlureInit1 {
            p1_start_color: [255, 255, 255, 75],
            p2_start_color: [255, 255, 255, 75],
            p1_end_color: [255, 255, 255, 0],
            p2_end_color: [255, 255, 255, 0],
            elem_duration: 6,
            unk_flag: 0,
            calc_mode: 3,
        }));
        b.add_vertex(Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0));
        b.add_vertex(Vec3::X * 10.0, Vec3::new(10.0, 10.0, 0.0));
        b.add_space();
        b.add_vertex(Vec3::X * 20.0, Vec3::new(20.0, 10.0, 0.0));
        b.add_vertex(Vec3::X * 30.0, Vec3::new(30.0, 10.0, 0.0));
        let mut out = EffectDraws::default();
        draw(&b, &mut out);
        let c = out.xlu[0].params.vertex_colors.as_ref().unwrap();
        // Slots 0..3 the first two edges, 4..7 the last two: quads 0 and 2 drawn, 1 (across the
        // gap) not; alpha at ratio 1/6 of 75.
        assert!(c[0..6].iter().all(|c| c[3] == 62));
        assert!(c[6..12].iter().all(|c| c[3] == 0));
        assert!(c[12..18].iter().all(|c| c[3] == 62));
        assert!(c[18..].iter().all(|c| c[3] == 0));
        // calcMode 3 at ratio 1/6: each point 1/12 of the way to the other (lerp_s16 truncates).
        let bones = &out.xlu[0].bones;
        assert_eq!(bones[0].w_axis.truncate(), Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(bones[1].w_axis.truncate(), Vec3::new(0.0, 9.0, 0.0));
    }
}
