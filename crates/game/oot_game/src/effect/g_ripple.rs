//! `Effect_Ss_G_Ripple` (`ovl_Effect_Ss_G_Ripple/z_eff_ss_g_ripple.c`): a ring spreading on the
//! water (`gEffWaterRippleDL`, flat, white): after `life` frames of waiting (unseen), it grows
//! from `radius` to `radiusMax` (`Math_SmoothStepToF`, a fifth of the gap, 1 to 30 a frame) while
//! its alpha fades from 255 (15 a frame at most, 7 at least), for 20 frames. It lies on the
//! surface of the water box it was spawned within 3 of (`BgCheck_FindWaterBox`), else at its own
//! height.
//!
//! Spawned by `Actor_UpdateBgCheckInfo` (three, 0, 4 and 8 frames apart, where an actor touches
//! the water), and by actors that land in water or walk on it (`En_Tite`, `En_Ishi`, `En_Kusa`,
//! `En_Niw`, `En_Goroiwa`, `En_Kanban`), and by `Effect_Ss_Bubble` reaching the surface.

use glam::{Mat4, Vec3};

use super::{DrawCtx, EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SsDraw, SsSpawn, SsUpdate, colored};
use crate::gbi::{G_AD_NOISE, G_CD_DISABLE, G_CD_NOISE, setup_dl};
use crate::pack::{BakeBody, BakeSegment, MeshBake};

// The overlay's regs.
pub const R_WATER_BOX_NUM: usize = 0;
pub const R_RADIUS: usize = 1;
pub const R_RADIUS_MAX: usize = 2;
pub const R_PRIM_COLOR_R: usize = 3;
pub const R_PRIM_COLOR_G: usize = 4;
pub const R_PRIM_COLOR_B: usize = 5;
pub const R_PRIM_COLOR_A: usize = 6;
pub const R_ENV_COLOR_R: usize = 7;
pub const R_ENV_COLOR_G: usize = 8;
pub const R_ENV_COLOR_B: usize = 9;
pub const R_ENV_COLOR_A: usize = 10;
/// `rLifespan`: the frames left to wait (`DECR`'d each update); drawn and grown at 0.
pub const R_LIFESPAN: usize = 11;

/// `EffectSsGRippleInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct GRippleInit {
    pub pos: Vec3,
    pub radius: i16,
    pub radius_max: i16,
    pub life: i16,
}

/// `EffectSsGRipple_Init`: still at `pos`, `life + 20` frames, white prim and env (alpha 255),
/// the water box within 3 of `pos` (`BgCheck_FindWaterBox(play, &play->colCtx, pos, 3.0f, ..)`,
/// -1 for none; none without a collision context).
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &GRippleInit) -> bool {
    this.velocity = Vec3::ZERO;
    this.accel = Vec3::ZERO;
    this.pos = p.pos;
    this.gfx = Some(("gameplay_keep", "gEffWaterRippleDL"));
    this.life = p.life.wrapping_add(20);
    this.flags = 0;
    this.draw = Some(SsDraw::GRipple);
    this.update = Some(SsUpdate::GRipple);
    let r = &mut this.regs;
    r[R_RADIUS] = p.radius;
    r[R_RADIUS_MAX] = p.radius_max;
    r[R_LIFESPAN] = p.life;
    r[R_PRIM_COLOR_R] = 255;
    r[R_PRIM_COLOR_G] = 255;
    r[R_PRIM_COLOR_B] = 255;
    r[R_PRIM_COLOR_A] = 255;
    r[R_ENV_COLOR_R] = 255;
    r[R_ENV_COLOR_G] = 255;
    r[R_ENV_COLOR_B] = 255;
    r[R_ENV_COLOR_A] = 255;
    // play->roomCtx.curRoom.num is the collision context's water room.
    r[R_WATER_BOX_NUM] = s.col.map_or(-1, |c| c.find_water_box(p.pos, 3.0, c.water_room)) as i16;
    true
}

/// `EffectSsGRipple_Update`: once `rLifespan` is down to 0 (`DECR`), the radius a fifth of the
/// way to `rRadiusMax` (between 1 and 30) and both alphas a fifth of the way to 0 (between 7 and
/// 15), each through a float truncated back.
pub fn update(this: &mut EffectSs) {
    let r = &mut this.regs;
    // DECR(this->rLifespan) == 0.
    if r[R_LIFESPAN] != 0 {
        r[R_LIFESPAN] -= 1;
    }
    if r[R_LIFESPAN] == 0 {
        let mut radius = r[R_RADIUS] as f32;
        eng_math::smooth_step_to_f(&mut radius, r[R_RADIUS_MAX] as f32, 0.2, 30.0, 1.0);
        r[R_RADIUS] = radius as i16;
        let mut prim_alpha = r[R_PRIM_COLOR_A] as f32;
        let mut env_alpha = r[R_ENV_COLOR_A] as f32;
        eng_math::smooth_step_to_f(&mut prim_alpha, 0.0, 0.2, 15.0, 7.0);
        eng_math::smooth_step_to_f(&mut env_alpha, 0.0, 0.2, 15.0, 7.0);
        r[R_PRIM_COLOR_A] = prim_alpha as i16;
        r[R_ENV_COLOR_A] = env_alpha as i16;
    }
}

/// The bake's name.
pub const BAKE: &str = "EffectSs/g_ripple";

/// `EffectSsGRipple_DrawRipple`'s mesh: `Gfx_SetupDL_60NoCDXlu` (`SETUPDL_60`, then
/// `gDPSetColorDither(G_CD_DISABLE)`), the colours dynamic, `gDPSetAlphaDither(G_AD_NOISE)`,
/// `gDPSetColorDither(G_CD_NOISE)`, `gEffWaterRippleDL` (which loads `gEffWaterRippleTex` itself:
/// the texture the draw passes as `segment` is unused).
pub fn bakes() -> Vec<MeshBake> {
    let mut setup = setup_dl::setup_dl_60();
    setup.color_dither(G_CD_DISABLE);
    setup.alpha_dither(G_AD_NOISE);
    setup.color_dither(G_CD_NOISE);
    setup.end();
    vec![MeshBake {
        name: BAKE.into(),
        object: "gameplay_keep".into(),
        segments: vec![(SEG_SETUP, BakeSegment::Commands(setup.0)), (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true })],
        prelude: vec![SEG_SETUP, SEG_COLOR],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffWaterRippleDL".into())]),
    }]
}

/// `EffectSsGRipple_DrawRipple`'s matrix: at its x and z, on its water box's surface (`ySurface`,
/// while `rWaterBoxNum` is a box of the scene's) or else at its own height, `rRadius × 0.0025`
/// every way (`SkinMatrix_SetTranslate` × `SkinMatrix_SetScale`).
pub fn ripple_matrix(this: &EffectSs, col: &eng_collision::bgcheck::CollisionContext) -> Mat4 {
    let r = &this.regs;
    let radius = r[R_RADIUS] as f32 * 0.0025;
    let n = r[R_WATER_BOX_NUM];
    let y_pos = match (n != -1 && (n as i32) < col.num_water_boxes() as i32).then(|| col.water_box_surface(n as usize)).flatten() {
        Some(y) => y as f32,
        None => this.pos.y,
    };
    Mat4::from_translation(Vec3::new(this.pos.x, y_pos, this.pos.z)) * Mat4::from_scale(Vec3::splat(radius))
}

/// `EffectSsGRipple_Draw`: nothing while it waits (`rLifespan` not 0); then
/// `EffectSsGRipple_DrawRipple`: the ring, prim and env their colours.
pub fn draw(this: &EffectSs, ctx: &DrawCtx<'_>, out: &mut EffectDraws) {
    let r = &this.regs;
    if r[R_LIFESPAN] != 0 {
        return;
    }
    let m = ripple_matrix(this, ctx.col);
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, r[R_PRIM_COLOR_A] as u8];
    let env = [r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, r[R_ENV_COLOR_B] as u8, r[R_ENV_COLOR_A] as u8];
    out.xlu.push(colored(BAKE, m, SEG_COLOR, prim, env));
}

impl SsSpawn<'_> {
    /// `EffectSsGRipple_Spawn`: priority 128.
    pub fn g_ripple_spawn(&mut self, pos: Vec3, radius: i16, radius_max: i16, life: i16) {
        let p = GRippleInit { pos, radius, radius_max, life };
        self.spawn(super::EFFECT_SS_G_RIPPLE, 128, super::SsInit::GRipple(p));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{EffectSsInfo, SsInit};
    use crate::play::Rand;
    use eng_collision::bgcheck::CollisionContext;
    use eng_collision::collision::CollisionBuilder;

    /// A scene with two water boxes: room 1's at 50 (x and z from 0 to 100), and one for all
    /// rooms at -20 (x and z from -100 to 0).
    fn col() -> CollisionContext {
        let mut b = CollisionBuilder::new();
        b.water_box(0, 0, 100, 100, 50, 1);
        b.water_box(-100, -100, 100, 100, -20, 0x3F);
        let mut c = CollisionContext::new(b.finish());
        c.water_room = 1;
        c
    }

    fn spawn(col: &CollisionContext, pos: Vec3, radius: i16, radius_max: i16, life: i16) -> EffectSs {
        let mut info = EffectSsInfo::default();
        let mut rand = Rand::default();
        let mut stops = Vec::new();
        let mut s = SsSpawn { info: &mut info, rand: &mut rand, objects: None, stops: &mut stops, state_frames: 0, col: Some(col) };
        s.spawn(crate::effect::EFFECT_SS_G_RIPPLE, 128, SsInit::GRipple(GRippleInit { pos, radius, radius_max, life }));
        assert_eq!(s.info.search_start_index, 1);
        s.info.table[0].clone()
    }

    #[test]
    fn init_finds_the_water_box_within_3_and_waits_life_frames() {
        let c = col();
        // At (50, 52, 50): room 1's box, its surface (50) within 3 of 52.
        let e = spawn(&c, Vec3::new(50.0, 52.0, 50.0), 100, 500, 4);
        assert_eq!((e.life, e.priority, e.ty), (24, 128, crate::effect::EFFECT_SS_G_RIPPLE));
        assert_eq!(&e.regs[..12], &[0, 100, 500, 255, 255, 255, 255, 255, 255, 255, 255, 4]);
        assert_eq!((e.velocity, e.accel, e.flags), (Vec3::ZERO, Vec3::ZERO, 0));
        // 53 is 3 above: `pos.y - 3 < ySurface` fails (open bound).
        assert_eq!(spawn(&c, Vec3::new(50.0, 53.0, 50.0), 0, 500, 0).regs[R_WATER_BOX_NUM], -1);
        // The all-rooms box (index 1).
        assert_eq!(spawn(&c, Vec3::new(-50.0, -21.0, -50.0), 0, 500, 0).regs[R_WATER_BOX_NUM], 1);
        // Room 2 doesn't see room 1's box.
        let mut c2 = col();
        c2.water_room = 2;
        assert_eq!(spawn(&c2, Vec3::new(50.0, 50.0, 50.0), 0, 500, 0).regs[R_WATER_BOX_NUM], -1);
    }

    #[test]
    fn update_waits_then_grows_and_fades() {
        let c = col();
        let mut e = spawn(&c, Vec3::new(50.0, 52.0, 50.0), 100, 500, 2);
        // DECR: 2 -> 1, nothing else.
        update(&mut e);
        assert_eq!((e.regs[R_LIFESPAN], e.regs[R_RADIUS], e.regs[R_PRIM_COLOR_A]), (1, 100, 255));
        // 1 -> 0: grows this frame. Radius: (500 - 100) × 0.2 = 80, held to 30: 130. Alpha:
        // -51 held to -15: 240.
        update(&mut e);
        assert_eq!((e.regs[R_LIFESPAN], e.regs[R_RADIUS], e.regs[R_PRIM_COLOR_A], e.regs[R_ENV_COLOR_A]), (0, 130, 240, 240));
        // 30 a frame while (500 - r) × 0.2 >= 30, i.e. up to 350: 160 .. 340, then 370; then
        // 370 + 26 = 396, 396 + 20.8 = 416.8 (416).
        for _ in 0..7 {
            update(&mut e);
        }
        assert_eq!(e.regs[R_RADIUS], 340);
        update(&mut e);
        assert_eq!(e.regs[R_RADIUS], 370);
        update(&mut e);
        assert_eq!(e.regs[R_RADIUS], 396);
        update(&mut e);
        assert_eq!(e.regs[R_RADIUS], 416);
        // Alpha: 15 a frame down to 75 (twelve frames, the eleventh now), then 75 - 15 = 60,
        // 60 - 12 = 48, 48 - 9.6 = 38.4 (38), 38 - 7.6 = 30.4 (30), then the step 6 is under 7:
        // up 7 to 37, past the target so 0; then down 7 to -7, past it so 0.
        assert_eq!(e.regs[R_PRIM_COLOR_A], 255 - 15 * 11);
        let mut seen = Vec::new();
        for _ in 0..6 {
            update(&mut e);
            seen.push(e.regs[R_PRIM_COLOR_A]);
        }
        assert_eq!(seen, [75, 60, 48, 38, 30, 0]);
        assert_eq!(e.regs[R_ENV_COLOR_A], 0);
    }

    #[test]
    fn drawn_flat_on_the_water_box_surface_once_waited() {
        let c = col();
        // 1.5 under room 1's surface: drawn at 50.
        let mut e = spawn(&c, Vec3::new(10.0, 51.5, 20.0), 400, 500, 0);
        let m = ripple_matrix(&e, &c);
        // rRadius × 0.0025 = 1.
        let want = Mat4::from_translation(Vec3::new(10.0, 50.0, 20.0)) * Mat4::from_scale(Vec3::splat(1.0));
        assert!(m.abs_diff_eq(want, 1e-6), "{m:?}");
        // No box: its own height.
        e.regs[R_WATER_BOX_NUM] = -1;
        assert!((ripple_matrix(&e, &c).w_axis.y - 51.5).abs() < 1e-6);
        // A box past the scene's count: its own height too.
        e.regs[R_WATER_BOX_NUM] = 2;
        assert!((ripple_matrix(&e, &c).w_axis.y - 51.5).abs() < 1e-6);
    }
}
