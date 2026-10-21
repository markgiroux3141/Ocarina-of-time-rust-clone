//! `Effect_Ss_Bubble` (`ovl_Effect_Ss_Bubble/z_eff_ss_bubble.c`): an air bubble rising in water
//! (`gEffBubbleDL` with `gEffBubble1Tex` or `2Tex`, opaque, facing the camera), wobbling about
//! where it started, living while it's under a water surface: it's gone out of water at once,
//! and at the surface with a small ripple (`EffectSsGRipple_Spawn(.., 0, 80, 0)`). Its life is
//! held at 1 (each update puts back the frame `EffectSs_UpdateAll` took).
//!
//! Spawned by doors opened underwater (`En_Door`, 50 to 79 at once), the diving game's rupees
//! (`En_Ex_Ruppy`), Player and the bosses.

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SEG_TEX, SsDraw, SsSpawn, SsUpdate, colored};
use crate::pack::{BakeBody, BakeSegment, MeshBake};
use crate::play::PlayState;

// The overlay's regs.
pub const R_SCALE: usize = 0;

/// The two textures `this->gfx` is one of (`eff_bubble.h`).
pub const TEXTURES: [&str; 2] = ["gEffBubble1Tex", "gEffBubble2Tex"];

/// `EffectSsBubbleInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct BubbleInit {
    pub pos: Vec3,
    pub y_pos_offset: f32,
    pub y_pos_rand_scale: f32,
    pub xz_pos_rand_scale: f32,
    pub scale: f32,
}

/// `this->gfx = SEGMENTED_TO_VIRTUAL(Rand_ZeroOne() < 0.5f ? gEffBubble1Tex : gEffBubble2Tex)`
/// (`Effect_Ss_Bubble`'s and `Effect_Ss_Dt_Bubble`'s init).
///
/// @bug (game) (the C's): the macro reads its argument twice, so `Rand_ZeroOne` is called twice:
/// the first pick gives the segment (4 for either texture), the second the offset, the texture
/// drawn.
pub fn random_texture(rand: &mut crate::play::Rand) -> &'static str {
    // SEGMENT_NUMBER(..): segment 4 either way.
    let _ = rand.zero_one() < 0.5;
    // SEGMENT_OFFSET(..).
    if rand.zero_one() < 0.5 { TEXTURES[0] } else { TEXTURES[1] }
}

/// `EffectSsBubble_Init`: one of the two textures (two `Rand_ZeroOne` calls), at `pos` up to
/// `xzPosRandScale / 2` across and `yPosRandScale / 2` up or down from `yPosOffset` above it (`vec`
/// keeps it), scaled `scale` to `1.5 × scale` (× 100 in `rScale`, truncated), life 1.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &BubbleInit) -> bool {
    this.gfx = Some(("gameplay_keep", random_texture(s.rand)));
    this.pos.x = ((s.rand.zero_one() - 0.5) * p.xz_pos_rand_scale) + p.pos.x;
    this.pos.y = (((s.rand.zero_one() - 0.5) * p.y_pos_rand_scale) + p.y_pos_offset) + p.pos.y;
    this.pos.z = ((s.rand.zero_one() - 0.5) * p.xz_pos_rand_scale) + p.pos.z;
    this.vec = this.pos;
    this.life = 1;
    this.regs[R_SCALE] = ((((s.rand.zero_one() * 0.5) + 1.0) * p.scale) * 100.0) as i16;
    this.draw = Some(SsDraw::Bubble);
    this.update = Some(SsUpdate::Bubble);
    true
}

/// `EffectSsBubble_Update`: out of every water box (`BgCheck_GetWaterSurfaceAllHack`) it's gone
/// (`life` -1); above the surface, a ripple there and gone; else its life put back and, about
/// where it started (`vec`), x and z a quarter either way at random and next frame's rise
/// `(Rand_ZeroOne() - 0.3) × 0.2` (three `Rand_ZeroOne` calls, x, y, z).
pub fn update(play: &mut PlayState, index: usize, this: &mut EffectSs) {
    let Some(water_surface_y) = play.bg_check_get_water_surface_all_hack(this.pos.x, this.pos.z, this.pos.y) else {
        this.life = -1;
        return;
    };
    if water_surface_y < this.pos.y {
        let ripple_pos = Vec3::new(this.pos.x, water_surface_y, this.pos.z);
        play.effect_ss_spawn_in_update(index, this, |s| s.g_ripple_spawn(ripple_pos, 0, 80, 0));
        this.life = -1;
    } else {
        this.life += 1;
        this.pos.x = ((play.rand.zero_one() * 0.5) - 0.25) + this.vec.x;
        this.accel.y = (play.rand.zero_one() - 0.3) * 0.2;
        this.pos.z = ((play.rand.zero_one() * 0.5) - 0.25) + this.vec.z;
    }
}

/// The bake of `gfx`'s texture (`Effect_Ss_Dt_Bubble` draws with the same).
pub fn bake_name(tex: &str) -> String {
    format!("EffectSs/bubble/{}", if tex == TEXTURES[1] { 2 } else { 1 })
}

/// One bake per texture on segment 8: `gEffBubbleDL` after `Gfx_SetupDL_25Opa` (the bakes' own
/// setup; the list sets its combiner and render mode), the colours dynamic (`Effect_Ss_Dt_Bubble`
/// sets its own, `Gfx_SetupDL_25Xlu2` being the same `SETUPDL_25`); the list loads the billboard
/// from segment 1 (the draw multiplies it in).
pub fn bakes() -> Vec<MeshBake> {
    TEXTURES
        .iter()
        .map(|tex| MeshBake {
            name: bake_name(tex),
            object: "gameplay_keep".into(),
            segments: vec![
                (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: (*tex).into() }),
                (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
                // gSPMatrix(0x01000000): the billboard, multiplied in by the draw.
                (0x01, BakeSegment::Bytes(super::identity_mtx())),
            ],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffBubbleDL".into())]),
        })
        .collect()
}

/// `Matrix_Translate(pos, MTXMODE_NEW)`, `Matrix_Scale(scale)`, then the list's billboard
/// (`Effect_Ss_Bubble`'s and `Effect_Ss_Dt_Bubble`'s draws).
pub fn bubble_matrix(pos: Vec3, scale: f32, billboard: Mat4) -> Mat4 {
    Mat4::from_translation(pos) * Mat4::from_scale(Vec3::splat(scale)) * billboard
}

/// `EffectSsBubble_Draw`: at its position scaled `rScale / 100`, opaque, prim white, env grey
/// (150, alpha 0).
pub fn draw(this: &EffectSs, billboard: Mat4, out: &mut EffectDraws) {
    let scale = this.regs[R_SCALE] as f32 / 100.0;
    let m = bubble_matrix(this.pos, scale, billboard);
    let tex = this.gfx.map_or(TEXTURES[0], |g| g.1);
    out.opa.push(colored(&bake_name(tex), m, SEG_COLOR, [255, 255, 255, 255], [150, 150, 150, 0]));
}

impl SsSpawn<'_> {
    /// `EffectSsBubble_Spawn`: priority 128.
    pub fn bubble_spawn(&mut self, pos: Vec3, y_pos_offset: f32, y_pos_rand_scale: f32, xz_pos_rand_scale: f32, scale: f32) {
        let p = BubbleInit { pos, y_pos_offset, y_pos_rand_scale, xz_pos_rand_scale, scale };
        self.spawn(super::EFFECT_SS_BUBBLE, 128, super::SsInit::Bubble(p));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::EffectSsInfo;
    use crate::play::Rand;

    #[test]
    fn init_places_and_scales_from_the_rand_calls() {
        let mut info = EffectSsInfo::default();
        let mut rand = Rand { state: 0x1234_5678 };
        // The calls the C makes, in order, from the same generator.
        let mut r = rand;
        let _segment = r.zero_one();
        let tex = if r.zero_one() < 0.5 { TEXTURES[0] } else { TEXTURES[1] };
        let x = ((r.zero_one() - 0.5) * 50.0) + 10.0;
        let y = (((r.zero_one() - 0.5) * 100.0) + 60.0) + 20.0;
        let z = ((r.zero_one() - 0.5) * 50.0) + 30.0;
        let scale = ((((r.zero_one() * 0.5) + 1.0) * 0.15) * 100.0) as i16;
        let mut stops = Vec::new();
        let mut s = SsSpawn { info: &mut info, rand: &mut rand, objects: None, stops: &mut stops, state_frames: 0, col: None };
        // En_Door's: 60 up, 100 and 50 of spread, 0.15.
        s.bubble_spawn(Vec3::new(10.0, 20.0, 30.0), 60.0, 100.0, 50.0, 0.15);
        let e = &info.table[0];
        assert_eq!(rand, r, "seven Rand_ZeroOne calls");
        assert_eq!((e.pos, e.vec, e.life, e.gfx), (Vec3::new(x, y, z), Vec3::new(x, y, z), 1, Some(("gameplay_keep", tex))));
        // 0.15 to 0.225, × 100: 15 to 22.
        assert_eq!(e.regs[R_SCALE], scale);
        assert!((15..=22).contains(&scale));
    }

    #[test]
    fn drawn_opaque_scaled_with_the_billboard_after() {
        let mut e = EffectSs { pos: Vec3::new(1.0, 2.0, 3.0), gfx: Some(("gameplay_keep", TEXTURES[1])), ..Default::default() };
        e.regs[R_SCALE] = 20;
        let bb = Mat4::from_rotation_y(1.1);
        let mut out = EffectDraws::default();
        draw(&e, bb, &mut out);
        assert_eq!((out.opa.len(), out.xlu.len()), (1, 0));
        let want = Mat4::from_translation(e.pos) * Mat4::from_scale(Vec3::splat(0.2)) * bb;
        assert!(out.opa[0].transform.abs_diff_eq(want, 1e-6));
        assert_eq!(out.opa[0].mesh, eng_gfx::MeshKey::named(crate::pack::keys::bake("EffectSs/bubble/2")));
    }
}
