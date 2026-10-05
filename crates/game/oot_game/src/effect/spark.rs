//! `EffectSpark` (`z_eff_spark.c`): drops flying out from a point and falling, each a small
//! billboard of `gUnknownCircle6Tex` whose four corners' colours fade from their start to their
//! end over the effect's duration: the hit's blood (`CollisionCheck_BlueBlood`,
//! `CollisionCheck_GreenBlood`).

use glam::{Mat4, Vec3};

use super::{DrawCtx, EffectDraws, SEG_DATA, SEG_SETUP, SEG_TEX};
use crate::gbi::{
    Dl, G_CULL_BOTH, G_CYC_2CYCLE, G_FOG, G_IM_FMT_I, G_IM_SIZ_8B, G_LIGHTING, G_RM_PASS, G_RM_ZB_CLD_SURF2, G_SHADE, G_SHADING_SMOOTH, G_TEXTURE_GEN, G_TEXTURE_GEN_LINEAR, G_TX_NOLOD, G_TX_NOMIRROR,
    G_TX_WRAP, G_ZBUFFER, ac, cc_ab, cc_c, cc_d, push_vtx, seg, setup_dl,
};
use crate::pack::{BakeBody, BakeSegment, MeshBake, keys};
use crate::play::Rand;

const BAKE: &str = "Effect/spark";

/// `EffectSparkElement`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EffectSparkElement {
    pub velocity: Vec3,
    pub position: Vec3,
    pub unk_velocity: [i16; 3],
    pub unk_position: [i16; 3],
}

/// `EffectSparkInit`.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectSparkInit {
    /// `position` (a `Vec3s`).
    pub position: [i16; 3],
    pub speed: f32,
    pub gravity: f32,
    pub u_div: u32,
    pub v_div: u32,
    pub color_start: [[u8; 4]; 4],
    pub color_end: [[u8; 4]; 4],
    pub timer: i32,
    pub duration: i32,
}

/// `EffectSpark`.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectSpark {
    pub position: [i16; 3],
    pub elements: Vec<EffectSparkElement>,
    pub speed: f32,
    pub gravity: f32,
    pub u_div: u32,
    pub v_div: u32,
    pub color_start: [[u8; 4]; 4],
    pub color_end: [[u8; 4]; 4],
    pub timer: i32,
    pub duration: i32,
}

/// `ARRAY_COUNT(this->elements)`.
const MAX_ELEMENTS: usize = 32;

impl EffectSpark {
    /// What an init that stopped early leaves in the slot (`Effect_Add` marks it active anyway):
    /// the C's slot keeps its last contents; here, no elements, ending on its first update.
    pub fn failed(p: &EffectSparkInit) -> EffectSpark {
        EffectSpark { position: p.position, elements: Vec::new(), speed: 0.0, gravity: 0.0, u_div: 0, v_div: 0, color_start: p.color_start, color_end: p.color_end, timer: 0, duration: -1 }
    }
}

/// `EffectSpark_Init`: `uDiv × vDiv + 2` drops at the position, each flying at `speed` in a
/// random direction (straight up for a near-zero one), with its unused spin.
pub fn init(p: &EffectSparkInit, rand: &mut Rand) -> Option<EffectSpark> {
    if p.u_div == 0 || p.v_div == 0 {
        log::error!("spark():u_div,v_div 0 is not good.");
        return None;
    }
    let n = (p.u_div * p.v_div + 2) as usize;
    if n > MAX_ELEMENTS {
        log::error!("over table_size");
        return None;
    }
    let pos = Vec3::new(p.position[0] as f32, p.position[1] as f32, p.position[2] as f32);
    let mut elements = Vec::with_capacity(n);
    for _ in 0..n {
        let mut v = Vec3::new(rand.zero_one() - 0.5, rand.zero_one() - 0.5, rand.zero_one() - 0.5);
        let norm = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
        if norm.abs() >= 0.008 {
            v *= p.speed * (1.0 / norm);
        } else {
            v = Vec3::new(0.0, p.speed, 0.0);
        }
        let uv = [(30000.0 - rand.zero_one() * 15000.0) as i32 as i16, (30000.0 - rand.zero_one() * 15000.0) as i32 as i16, (30000.0 - rand.zero_one() * 15000.0) as i32 as i16];
        let up = [(rand.zero_one() * 65534.0) as i32 as i16, (rand.zero_one() * 65534.0) as i32 as i16, (rand.zero_one() * 65534.0) as i32 as i16];
        elements.push(EffectSparkElement { velocity: v, position: pos, unk_velocity: uv, unk_position: up });
    }
    Some(EffectSpark {
        position: p.position,
        elements,
        speed: p.speed,
        gravity: p.gravity,
        u_div: p.u_div,
        v_div: p.v_div,
        color_start: p.color_start,
        color_end: p.color_end,
        timer: 0,
        duration: p.duration,
    })
}

/// `EffectSpark_Update`: each drop moves and falls; done once past its duration.
pub fn update(s: &mut EffectSpark) -> bool {
    for e in &mut s.elements {
        e.position += e.velocity;
        e.velocity.y += s.gravity;
        for k in 0..3 {
            e.unk_position[k] = e.unk_position[k].wrapping_add(e.unk_velocity[k]);
        }
    }
    s.timer += 1;
    s.duration < s.timer
}

/// The bake: `SETUPDL_38` in two cycles, `gUnknownCircle6Tex` (I8, 32 × 32, wrapping), the
/// combiner `G_CC_SHADEDECALA, G_CC_PASS2`, `G_RM_PASS, G_RM_ZB_CLD_SURF2`, and one drop's quad
/// (`ob` ±32, `tc` 0 or 1024) as two triangles (2, 0, 3) and (2, 3, 1), its corners' colours
/// the draw's.
pub fn bakes() -> Vec<MeshBake> {
    vec![particle_bake(BAKE, [[2, 0, 3], [2, 3, 1]], |d| {
        // G_CC_SHADEDECALA: 0, 0, 0, SHADE, 0, 0, 0, TEXEL0.
        let c = [cc_ab::ZERO, cc_ab::ZERO, cc_c::ZERO, cc_d::SHADE, ac::ZERO, ac::ZERO, ac::ZERO, ac::TEXEL0];
        d.combine_lerp(c, setup_dl::PASS2);
    })]
}

/// The particles' quad bake: `SETUPDL_38`, `gDPSetCycleType(G_CYC_2CYCLE)`, the texture, the
/// combiner `combine` writes, the render and geometry modes, the quad as `tris`.
pub(crate) fn particle_bake(name: &str, tris: [[u32; 3]; 2], combine: impl Fn(&mut Dl)) -> MeshBake {
    let mut d = setup_dl::setup_dl_38();
    d.cycle_type(G_CYC_2CYCLE);
    d.pipe_sync();
    d.texture_on(true);
    // gDPLoadTextureBlock(gUnknownCircle6Tex, G_IM_FMT_I, G_IM_SIZ_8b, 32, 32, 0, G_TX_NOMIRROR |
    // G_TX_WRAP, G_TX_NOMIRROR | G_TX_WRAP, 5, 5, G_TX_NOLOD, G_TX_NOLOD).
    d.load_texture_block(seg(SEG_TEX as u32, 0), G_IM_FMT_I, G_IM_SIZ_8B, 32, 32, 0, G_TX_NOMIRROR | G_TX_WRAP, G_TX_NOMIRROR | G_TX_WRAP, 5, 5, G_TX_NOLOD, G_TX_NOLOD);
    combine(&mut d);
    d.render_mode(G_RM_PASS, G_RM_ZB_CLD_SURF2);
    d.clear_geometry_mode(G_CULL_BOTH | G_FOG | G_LIGHTING | G_TEXTURE_GEN | G_TEXTURE_GEN_LINEAR);
    d.set_geometry_mode(G_ZBUFFER | G_SHADE | G_SHADING_SMOOTH);
    d.pipe_sync();
    d.vertex(seg(SEG_DATA as u32, 0), 4, 0);
    d.tri2(tris[0], tris[1]);
    d.end();
    let mut vtx = Vec::new();
    push_vtx(&mut vtx, [-32, -32, 0], [0, 1024], [255; 4]);
    push_vtx(&mut vtx, [32, 32, 0], [1024, 0], [255; 4]);
    push_vtx(&mut vtx, [-32, 32, 0], [0, 0], [255; 4]);
    push_vtx(&mut vtx, [32, -32, 0], [1024, 1024], [255; 4]);
    MeshBake {
        name: name.into(),
        object: "gameplay_keep".into(),
        segments: vec![
            (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: "gUnknownCircle6Tex".into() }),
            (SEG_DATA, BakeSegment::Bytes(vtx)),
            (SEG_SETUP, BakeSegment::Commands(d.0)),
        ],
        prelude: vec![SEG_SETUP],
        body: BakeBody::DLists(Vec::new()),
    }
}

/// `EffectSpark_Draw`: the corners' colours at `timer / duration` of the way from start to
/// end; each drop a billboard of a random size, `(Rand × 2.5 + 1.5) / 64`.
pub fn draw(s: &EffectSpark, ctx: &mut DrawCtx<'_>, out: &mut EffectDraws) {
    let ratio = s.timer as f32 / s.duration as f32;
    let c: Vec<[u8; 4]> = (0..4).map(|k| std::array::from_fn(|ch| (s.color_start[k][ch] as f32 + (s.color_end[k][ch] as f32 - s.color_start[k][ch] as f32) * ratio) as i32 as u8)).collect();
    // The quad's two triangles' corners, (2, 0, 3) and (2, 3, 1).
    let colors = vec![c[2], c[0], c[3], c[2], c[3], c[1]];
    for e in &s.elements {
        let t = ((ctx.rand.zero_one() * 2.5) + 1.5) / 64.0;
        let m = Mat4::from_translation(e.position) * ctx.billboard * Mat4::from_scale(Vec3::new(t, t, 1.0));
        let mut cmd = eng_gfx::DrawCmd::new(eng_gfx::MeshKey::named(keys::bake(BAKE)), m);
        cmd.params.vertex_colors = Some(colors.clone());
        out.xlu.push(cmd);
    }
}
