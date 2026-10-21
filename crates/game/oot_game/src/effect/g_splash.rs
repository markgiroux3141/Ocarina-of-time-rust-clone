//! `Effect_Ss_G_Splash` (`ovl_Effect_Ss_G_Splash/z_eff_ss_g_splash.c`): a splash of water facing
//! the camera, `gEffWaterSplashDL` through its eight textures (`gEffWaterSplash1Tex` to `8Tex`), one
//! a frame, for 8 frames (`EffectSs_DrawGEffect`). Type 0 and 2 are white at alpha 200, type 1 at
//! 255, and a type 1 throws up a type 2 half its size above itself at its fourth frame (life 5);
//! with custom colours (`customColor`) its type only picks whether it draws (0 to 2) and throws.
//!
//! @bug (game) (the C's): `rType` is `regs[11]`, which `EffectSs_DrawGEffect` reads as
//! `rgObjectSlot` to set segment 6 to that object slot's file; nothing in the splash's list is on
//! segment 6, so it has no effect (not modelled).

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SEG_TEX, SsDraw, SsSpawn, SsUpdate, billboard_mtx, colored};
use crate::gbi::setup_dl;
use crate::pack::{BakeBody, BakeSegment, MeshBake};
use crate::play::PlayState;

// The common regs (`effect.h`'s `rg*`), and the overlay's `rType`.
pub const RG_TEX_IDX: usize = 0;
pub const RG_SCALE: usize = 1;
pub const RG_TEX_IDX_STEP: usize = 2;
pub const RG_PRIM_COLOR_R: usize = 3;
pub const RG_PRIM_COLOR_G: usize = 4;
pub const RG_PRIM_COLOR_B: usize = 5;
pub const RG_PRIM_COLOR_A: usize = 6;
pub const RG_ENV_COLOR_R: usize = 7;
pub const RG_ENV_COLOR_G: usize = 8;
pub const RG_ENV_COLOR_B: usize = 9;
pub const RG_ENV_COLOR_A: usize = 10;
pub const R_TYPE: usize = 11;

/// `EffectSsGSplashInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct GSplashInit {
    pub pos: Vec3,
    pub ty: u8,
    pub custom_color: bool,
    pub scale: i16,
    pub prim_color: [u8; 4],
    pub env_color: [u8; 4],
}

/// `EffectSsGSplash_Init`: still at `pos`, 8 frames, scale `scale` (600 for 0), the texture index
/// up 100 a frame; the colours and type given with `customColor`, else by type: 0 and 2 white at
/// alpha 200, 1 white at 255; another type sets neither (black, alpha 0, `rType` 0).
pub fn init(this: &mut EffectSs, p: &GSplashInit) -> bool {
    this.velocity = Vec3::ZERO;
    this.accel = Vec3::ZERO;
    this.pos = p.pos;
    this.draw = Some(SsDraw::GSplash);
    this.update = Some(SsUpdate::GSplash);
    let scale = if p.scale == 0 { 600 } else { p.scale };
    this.gfx = Some(("gameplay_keep", "gEffWaterSplashDL"));
    this.life = 8;
    let r = &mut this.regs;
    r[RG_SCALE] = scale;
    r[RG_TEX_IDX] = 0;
    r[RG_TEX_IDX_STEP] = 100;
    let set = |r: &mut [i16; 13], prim: [u8; 4], env: [u8; 4], ty: u8| {
        r[RG_PRIM_COLOR_R] = prim[0] as i16;
        r[RG_PRIM_COLOR_G] = prim[1] as i16;
        r[RG_PRIM_COLOR_B] = prim[2] as i16;
        r[RG_PRIM_COLOR_A] = prim[3] as i16;
        r[RG_ENV_COLOR_R] = env[0] as i16;
        r[RG_ENV_COLOR_G] = env[1] as i16;
        r[RG_ENV_COLOR_B] = env[2] as i16;
        r[RG_ENV_COLOR_A] = env[3] as i16;
        r[R_TYPE] = ty as i16;
    };
    if p.custom_color {
        set(r, p.prim_color, p.env_color, p.ty);
    } else {
        match p.ty {
            0 => set(r, [255, 255, 255, 200], [255, 255, 255, 200], 0),
            1 => set(r, [255, 255, 255, 255], [255, 255, 255, 255], 1),
            2 => set(r, [255, 255, 255, 200], [255, 255, 255, 200], 2),
            _ => {}
        }
    }
    true
}

/// `EffectSsGSplash_Update`: a type 1 at life 5 throws up a type 2 (`EffectSsGSplash_Spawn`, no
/// custom colours) `rgScale × 20 × 0.002` above itself, half its scale (truncated); then the
/// texture index steps.
pub fn update(play: &mut PlayState, index: usize, this: &mut EffectSs) {
    if this.regs[R_TYPE] == 1 && this.life == 5 {
        let mut new_splash_pos = this.pos;
        let scale = this.regs[RG_SCALE];
        new_splash_pos.y += (scale as i32 * 20) as f32 * 0.002;
        play.effect_ss_spawn_in_update(index, this, |s| s.g_splash_spawn(new_splash_pos, None, None, 2, scale / 2));
    }
    this.regs[RG_TEX_IDX] = this.regs[RG_TEX_IDX].wrapping_add(this.regs[RG_TEX_IDX_STEP]);
}

fn bake_name(n: usize) -> String {
    format!("EffectSs/g_splash/{n}")
}

/// One bake per texture (`waterSplashTextures`: `gEffWaterSplash1Tex` to `8Tex` on segment 8):
/// `EffectSs_DrawGEffect`'s `Gfx_SetupDL_61Xlu`, the colours dynamic, `gEffWaterSplashDL`.
pub fn bakes() -> Vec<MeshBake> {
    let mut setup = setup_dl::setup_dl_61();
    setup.end();
    (1..=8)
        .map(|n| MeshBake {
            name: bake_name(n),
            object: "gameplay_keep".into(),
            segments: vec![
                (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: format!("gEffWaterSplash{n}Tex") }),
                (SEG_SETUP, BakeSegment::Commands(setup.0.clone())),
                (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
            ],
            prelude: vec![SEG_SETUP, SEG_COLOR],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffWaterSplashDL".into())]),
        })
        .collect()
}

/// The texture `EffectSsGSplash_Draw` draws (1 to 8): `rgTexIdx / 100`, at most 7; `None` for a
/// type past 2 (drawn as nothing).
pub fn texture(this: &EffectSs) -> Option<usize> {
    let r = &this.regs;
    matches!(r[R_TYPE], 0..=2).then(|| (r[RG_TEX_IDX] / 100).min(7) as usize + 1)
}

/// `EffectSsGSplash_Draw`: types 0 to 2 through `EffectSs_DrawGEffect`: at its position facing
/// the camera, `rgScale × 0.0025` every way, its colours.
pub fn draw(this: &EffectSs, billboard: Mat4, out: &mut EffectDraws) {
    let Some(tex) = texture(this) else { return };
    let r = &this.regs;
    let scale = r[RG_SCALE] as f32 * 0.0025;
    let m = billboard_mtx(this.pos, billboard, Vec3::splat(scale));
    let prim = [r[RG_PRIM_COLOR_R] as u8, r[RG_PRIM_COLOR_G] as u8, r[RG_PRIM_COLOR_B] as u8, r[RG_PRIM_COLOR_A] as u8];
    let env = [r[RG_ENV_COLOR_R] as u8, r[RG_ENV_COLOR_G] as u8, r[RG_ENV_COLOR_B] as u8, r[RG_ENV_COLOR_A] as u8];
    out.xlu.push(colored(&bake_name(tex), m, SEG_COLOR, prim, env));
}

impl SsSpawn<'_> {
    /// `EffectSsGSplash_Spawn`: priority 128; with `prim` (and `env`) given, custom colours (the C
    /// reads `envColor` whenever `primColor` isn't NULL: a missing one is taken as black, logged).
    pub fn g_splash_spawn(&mut self, pos: Vec3, prim: Option<[u8; 4]>, env: Option<[u8; 4]>, ty: i16, scale: i16) {
        let p = match prim {
            Some(prim_color) => {
                let env_color = env.unwrap_or_else(|| {
                    log::warn!("EffectSsGSplash_Spawn: a prim colour without an env colour (the C reads NULL)");
                    [0; 4]
                });
                GSplashInit { pos, ty: ty as u8, custom_color: true, scale, prim_color, env_color }
            }
            None => GSplashInit { pos, ty: ty as u8, custom_color: false, scale, prim_color: [0; 4], env_color: [0; 4] },
        };
        self.spawn(super::EFFECT_SS_G_SPLASH, 128, super::SsInit::GSplash(p));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn splash(ty: u8, custom: bool, scale: i16) -> EffectSs {
        let mut e = EffectSs::default();
        assert!(init(&mut e, &GSplashInit { pos: Vec3::new(1.0, 2.0, 3.0), ty, custom_color: custom, scale, prim_color: [1, 2, 3, 4], env_color: [5, 6, 7, 8] }));
        e
    }

    #[test]
    fn init_colours_by_type() {
        // Scale 0 is 600; 8 frames; the index from 0 by 100.
        let e = splash(0, false, 0);
        assert_eq!((e.life, e.pos, e.velocity), (8, Vec3::new(1.0, 2.0, 3.0), Vec3::ZERO));
        assert_eq!(&e.regs[..12], &[0, 600, 100, 255, 255, 255, 200, 255, 255, 255, 200, 0]);
        assert_eq!(&splash(1, false, 350).regs[..12], &[0, 350, 100, 255, 255, 255, 255, 255, 255, 255, 255, 1]);
        assert_eq!(&splash(2, false, 400).regs[3..12], &[255, 255, 255, 200, 255, 255, 255, 200, 2]);
        // Type 3 sets no colours and no type: all 0, drawn as type 0.
        assert_eq!(&splash(3, false, 400).regs[3..12], &[0; 9]);
        // Custom: as given, the type kept (5: not drawn).
        let c = splash(5, true, 400);
        assert_eq!(&c.regs[3..12], &[1, 2, 3, 4, 5, 6, 7, 8, 5]);
        assert_eq!(texture(&c), None);
    }

    #[test]
    fn a_texture_a_frame_and_the_last_held() {
        let mut e = splash(0, false, 400);
        let mut seen = vec![texture(&e).unwrap()];
        for _ in 0..9 {
            // The type 0 update spawns nothing: only the index steps.
            e.regs[RG_TEX_IDX] += e.regs[RG_TEX_IDX_STEP];
            seen.push(texture(&e).unwrap());
        }
        assert_eq!(seen, [1, 2, 3, 4, 5, 6, 7, 8, 8, 8]);
    }

    #[test]
    fn drawn_facing_the_camera_at_scale_over_400() {
        let e = splash(0, false, 800);
        let bb = Mat4::from_rotation_y(0.7);
        let mut out = EffectDraws::default();
        draw(&e, bb, &mut out);
        assert_eq!(out.xlu.len(), 1);
        let want = Mat4::from_translation(e.pos) * bb * Mat4::from_scale(Vec3::splat(2.0));
        assert!(out.xlu[0].transform.abs_diff_eq(want, 1e-6));
        let sv = out.xlu[0].params.segments.as_ref().unwrap();
        assert_eq!((sv.prim[SEG_COLOR as usize], sv.env[SEG_COLOR as usize]), (Some([255, 255, 255, 200]), Some([255, 255, 255, 200])));
    }
}
