//! `Effect_Ss_Dt_Bubble` (`ovl_Effect_Ss_Dt_Bubble/z_eff_ss_dt_bubble.c`): a coloured bubble
//! (`gEffBubbleDL` with `gEffBubble1Tex` or `2Tex`, translucent, facing the camera) moving as
//! given and fading out over its life, wobbling across at random with `randXZ`. Its colours are
//! one of three profiles (`sPrimColors`, `sEnvColors`: 0 yellow on red, 1 pale cyan on green, 2
//! cyan on blue) or custom.
//!
//! Spawned by `En_Part` (a dying enemy's part, kind 4: profile 0, wobbling) and by actors not
//! ported yet (the bosses, `En_Bubble`, `En_Fr`, ...).

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SsDraw, SsSpawn, SsUpdate, bubble, colored};
use crate::play::PlayState;

// The overlay's regs.
pub const R_PRIM_COLOR_R: usize = 0;
pub const R_PRIM_COLOR_G: usize = 1;
pub const R_PRIM_COLOR_B: usize = 2;
pub const R_PRIM_COLOR_A: usize = 3;
pub const R_ENV_COLOR_R: usize = 4;
pub const R_ENV_COLOR_G: usize = 5;
pub const R_ENV_COLOR_B: usize = 6;
pub const R_ENV_COLOR_A: usize = 7;
pub const R_RAND_XZ: usize = 8;
pub const R_SCALE: usize = 9;
pub const R_LIFESPAN: usize = 10;

/// `sPrimColors`, `sEnvColors`: the colour profiles.
pub const S_PRIM_COLORS: [[u8; 4]; 3] = [[255, 255, 100, 255], [150, 255, 255, 255], [100, 255, 255, 255]];
pub const S_ENV_COLORS: [[u8; 4]; 3] = [[170, 0, 0, 255], [0, 100, 0, 255], [0, 0, 255, 255]];

/// `EffectSsDtBubbleInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct DtBubbleInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub prim_color: [u8; 4],
    pub env_color: [u8; 4],
    pub scale: i16,
    pub life: i16,
    pub color_profile: i16,
    /// `randXZ`: 1 wobbles across at random.
    pub rand_xz: i16,
    pub custom_color: bool,
}

/// `EffectSsDtBubble_Init`: one of the two textures (two `Rand_ZeroOne` calls,
/// `bubble::random_texture`), at `pos` moving as given for `life` frames (`rLifespan` keeps it),
/// the colours custom or the profile's.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &DtBubbleInit) -> bool {
    this.gfx = Some(("gameplay_keep", bubble::random_texture(s.rand)));
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.life = p.life;
    let (prim, env) = if !p.custom_color {
        match (S_PRIM_COLORS.get(p.color_profile as usize), S_ENV_COLORS.get(p.color_profile as usize)) {
            (Some(&prim), Some(&env)) => (prim, env),
            _ => {
                // The C reads past the arrays.
                log::warn!("Effect_Ss_Dt_Bubble: colour profile {} past the three", p.color_profile);
                ([0; 4], [0; 4])
            }
        }
    } else {
        (p.prim_color, p.env_color)
    };
    let r = &mut this.regs;
    r[R_PRIM_COLOR_R] = prim[0] as i16;
    r[R_PRIM_COLOR_G] = prim[1] as i16;
    r[R_PRIM_COLOR_B] = prim[2] as i16;
    r[R_PRIM_COLOR_A] = prim[3] as i16;
    r[R_ENV_COLOR_R] = env[0] as i16;
    r[R_ENV_COLOR_G] = env[1] as i16;
    r[R_ENV_COLOR_B] = env[2] as i16;
    r[R_ENV_COLOR_A] = env[3] as i16;
    r[R_RAND_XZ] = p.rand_xz;
    r[R_SCALE] = p.scale;
    r[R_LIFESPAN] = p.life;
    this.draw = Some(SsDraw::DtBubble);
    this.update = Some(SsUpdate::DtBubble);
    true
}

/// `EffectSsDtBubble_Update`: with `rRandXZ` 1, x then z moved up to 1 either way at random.
pub fn update(play: &mut PlayState, this: &mut EffectSs) {
    if this.regs[R_RAND_XZ] == 1 {
        let rand = play.rand.zero_one();
        this.pos.x += (rand * 2.0) - 1.0;
        let rand = play.rand.zero_one();
        this.pos.z += (rand * 2.0) - 1.0;
    }
}

/// The colours `EffectSsDtBubble_Draw` sets: each alpha `alpha × life / rLifespan` (integer
/// division; `rLifespan` 0 would divide by zero, taken as the full alpha, logged).
pub fn colors(this: &EffectSs) -> ([u8; 4], [u8; 4]) {
    let r = &this.regs;
    let fade = |a: i16| {
        if r[R_LIFESPAN] == 0 {
            log::warn!("Effect_Ss_Dt_Bubble: alpha / rLifespan with rLifespan 0");
            return a as u8;
        }
        ((a as i32 * this.life as i32) / r[R_LIFESPAN] as i32) as u8
    };
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, fade(r[R_PRIM_COLOR_A])];
    let env = [r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, r[R_ENV_COLOR_B] as u8, fade(r[R_ENV_COLOR_A])];
    (prim, env)
}

/// `EffectSsDtBubble_Draw`: at its position scaled `rScale × 0.004` (then the list's billboard),
/// translucent (`Gfx_SetupDL_25Xlu2`), its colours fading; `Effect_Ss_Bubble`'s bakes.
pub fn draw(this: &EffectSs, billboard: Mat4, out: &mut EffectDraws) {
    let scale = this.regs[R_SCALE] as f32 * 0.004;
    let m = bubble::bubble_matrix(this.pos, scale, billboard);
    let (prim, env) = colors(this);
    let tex = this.gfx.map_or(bubble::TEXTURES[0], |g| g.1);
    out.xlu.push(colored(&bubble::bake_name(tex), m, SEG_COLOR, prim, env));
}

impl SsSpawn<'_> {
    /// `EffectSsDtBubble_SpawnColorProfile`: priority 128, the colours of `color_profile` (the C
    /// leaves the init params' colours unset: zero here, unread).
    #[allow(clippy::too_many_arguments)]
    pub fn dt_bubble_spawn_color_profile(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, scale: i16, life: i16, color_profile: i16, rand_xz: i16) {
        let p = DtBubbleInit { pos, velocity, accel, prim_color: [0; 4], env_color: [0; 4], scale, life, color_profile, rand_xz, custom_color: false };
        self.spawn(super::EFFECT_SS_DT_BUBBLE, 128, super::SsInit::DtBubble(p));
    }

    /// `EffectSsDtBubble_SpawnCustomColor`: priority 128 (the C leaves `colorProfile` unset: zero
    /// here, unread).
    #[allow(clippy::too_many_arguments)]
    pub fn dt_bubble_spawn_custom_color(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, prim: [u8; 4], env: [u8; 4], scale: i16, life: i16, rand_xz: i16) {
        let p = DtBubbleInit { pos, velocity, accel, prim_color: prim, env_color: env, scale, life, color_profile: 0, rand_xz, custom_color: true };
        self.spawn(super::EFFECT_SS_DT_BUBBLE, 128, super::SsInit::DtBubble(p));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::EffectSsInfo;
    use crate::play::Rand;

    fn spawn(f: impl FnOnce(&mut SsSpawn<'_>)) -> (EffectSs, Rand) {
        let mut info = EffectSsInfo::default();
        let mut rand = Rand { state: 7 };
        let mut stops = Vec::new();
        let mut s = SsSpawn { info: &mut info, rand: &mut rand, objects: None, stops: &mut stops, state_frames: 0, col: None };
        f(&mut s);
        (info.table[0].clone(), rand)
    }

    #[test]
    fn init_by_profile_and_custom() {
        let (pos, vel, acc) = (Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.0, 1.5, 0.0), Vec3::ZERO);
        // En_Part's kind 4: profile 0, 25 frames, wobbling.
        let (e, rand) = spawn(|s| s.dt_bubble_spawn_color_profile(pos, vel, acc, 120, 25, 0, 1));
        let mut r = Rand { state: 7 };
        let tex = bubble::random_texture(&mut r);
        assert_eq!(rand, r, "the two Rand_ZeroOne calls of the texture's macro");
        assert_eq!((e.pos, e.velocity, e.accel, e.life, e.gfx), (pos, vel, acc, 25, Some(("gameplay_keep", tex))));
        assert_eq!(&e.regs[..11], &[255, 255, 100, 255, 170, 0, 0, 255, 1, 120, 25]);
        let (e, _) = spawn(|s| s.dt_bubble_spawn_color_profile(pos, vel, acc, 80, 10, 2, 0));
        assert_eq!(&e.regs[..11], &[100, 255, 255, 255, 0, 0, 255, 255, 0, 80, 10]);
        let (e, _) = spawn(|s| s.dt_bubble_spawn_custom_color(pos, vel, acc, [1, 2, 3, 4], [5, 6, 7, 8], 50, 12, 0));
        assert_eq!(&e.regs[..11], &[1, 2, 3, 4, 5, 6, 7, 8, 0, 50, 12]);
    }

    #[test]
    fn fades_with_its_life_and_draws_translucent() {
        let (mut e, _) = spawn(|s| s.dt_bubble_spawn_color_profile(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, 250, 25, 1, 0));
        // 255 × 25 / 25; then 255 × 10 / 25 = 102; 255 × 1 / 25 = 10.
        assert_eq!(colors(&e), ([150, 255, 255, 255], [0, 100, 0, 255]));
        e.life = 10;
        assert_eq!(colors(&e), ([150, 255, 255, 102], [0, 100, 0, 102]));
        e.life = 1;
        assert_eq!(colors(&e).0[3], 10);
        let bb = Mat4::from_rotation_x(0.3);
        let mut out = EffectDraws::default();
        draw(&e, bb, &mut out);
        assert_eq!((out.opa.len(), out.xlu.len()), (0, 1));
        // 250 × 0.004 = 1.
        let want = Mat4::from_translation(e.pos) * bb;
        assert!(out.xlu[0].transform.abs_diff_eq(want, 1e-6));
    }
}
