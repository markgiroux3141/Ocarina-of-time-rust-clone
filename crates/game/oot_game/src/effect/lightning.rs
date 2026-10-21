//! `Effect_Ss_Lightning` (`ovl_Effect_Ss_Lightning/z_eff_ss_lightning.c`): a bolt of lightning, a
//! tall quad facing the camera turned by its yaw about the view axis (`gEffLightningDL`, its eight
//! textures `gEffLightning1Tex` to `8Tex` in turn, one a frame, the last held), branching: on its
//! first update a bolt with bolts left (`rNumBolts`) inserts the next at its end (`80 × rScale /
//! 100` along it, the next turned 20 to 38 degrees either way), and one time in ten a second
//! mirrored about its own yaw (`EffectSsLightning_NewLightning`, `EffectSs_Insert`). An inserted
//! bolt goes into the table as it is (no init); in a later slot it updates, and so branches, the
//! same frame.
//!
//! Spawned by actors not ported yet (`Boss_Va`, `En_Bili`'s and `En_Vali`'s shocks, ...).

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SEG_TEX, SsDraw, SsSpawn, SsUpdate, colored};
use crate::gbi::setup_dl;
use crate::pack::{BakeBody, BakeSegment, MeshBake};
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
pub const R_NUM_BOLTS: usize = 8;
pub const R_SCALE: usize = 9;
pub const R_YAW: usize = 10;
pub const R_LIFESPAN: usize = 11;

/// `EffectSsLightningInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct LightningInit {
    pub pos: Vec3,
    pub prim_color: [u8; 4],
    pub env_color: [u8; 4],
    pub scale: i16,
    pub yaw: i16,
    pub life: i16,
    pub num_bolts: i16,
}

/// `EffectSsLightning_Init`: at `pos` (still: the velocity and acceleration `EffectSs_Reset`'s
/// zeros), `life` frames (`rLifespan` keeps it), its colours, bolts, scale and yaw.
pub fn init(this: &mut EffectSs, p: &LightningInit) -> bool {
    this.pos = p.pos;
    this.gfx = Some(("gameplay_keep", "gEffLightningDL"));
    this.life = p.life;
    this.draw = Some(SsDraw::Lightning);
    this.update = Some(SsUpdate::Lightning);
    let r = &mut this.regs;
    r[R_PRIM_COLOR_R] = p.prim_color[0] as i16;
    r[R_PRIM_COLOR_G] = p.prim_color[1] as i16;
    r[R_PRIM_COLOR_B] = p.prim_color[2] as i16;
    r[R_PRIM_COLOR_A] = p.prim_color[3] as i16;
    r[R_ENV_COLOR_R] = p.env_color[0] as i16;
    r[R_ENV_COLOR_G] = p.env_color[1] as i16;
    r[R_ENV_COLOR_B] = p.env_color[2] as i16;
    r[R_ENV_COLOR_A] = p.env_color[3] as i16;
    r[R_NUM_BOLTS] = p.num_bolts;
    r[R_SCALE] = p.scale;
    r[R_YAW] = p.yaw;
    r[R_LIFESPAN] = p.life;
    true
}

/// `EffectSsLightning_NewLightning`: a copy of this bolt (the C first runs `EffectSs_Delete` on
/// its uninitialised stack copy: no sound is at that address, so nothing) at `pos`, a bolt fewer,
/// turned to `yaw`, its life whole, inserted into the table (`EffectSs_Insert`).
fn new_lightning(play: &mut PlayState, index: usize, this: &mut EffectSs, pos: Vec3, yaw: i16) {
    let mut new_lightning = this.clone();
    new_lightning.pos = pos;
    new_lightning.regs[R_NUM_BOLTS] = new_lightning.regs[R_NUM_BOLTS].wrapping_sub(1);
    new_lightning.regs[R_YAW] = yaw;
    new_lightning.life = new_lightning.regs[R_LIFESPAN];
    // `this` is the table's slot: the insert can take it (a full table).
    play.effect_ss.table[index] = this.clone();
    play.effect_ss.insert(new_lightning);
    *this = play.effect_ss.table[index].clone();
}

/// `EffectSsLightning_Update`: on the first update (`life + 1 == rLifespan`) of a bolt with bolts
/// left, the next bolt's yaw this one's plus or minus (a `Rand_ZeroOne` under 0.5: minus)
/// `(s16)(Rand_ZeroOne() × 3640) + 0xE38`; its position this bolt's end, `rScale × 0.01 × 80`
/// along the yaw less a quarter turn, across the screen by the active camera's input yaw
/// (`Camera_GetInputDirYaw`); inserted, and when a third `Rand_ZeroOne` is under 0.1, a second
/// at the yaw mirrored about this one's (`rYaw × 2 - yaw`).
pub fn update(play: &mut PlayState, index: usize, this: &mut EffectSs) {
    if this.regs[R_NUM_BOLTS] != 0 && (this.life as i32 + 1) == this.regs[R_LIFESPAN] as i32 {
        let r_yaw = this.regs[R_YAW];
        let sign = if play.rand.zero_one() < 0.5 { -1 } else { 1 };
        let yaw = (r_yaw as i32 + sign * (((play.rand.zero_one() * 3640.0) as i16) as i32 + 0xE38)) as i16;
        let scale = (this.regs[R_SCALE] as f32 * 0.01) * 80.0;
        let mut pos = Vec3::ZERO;
        pos.y = this.pos.y + (eng_math::sin_s(r_yaw.wrapping_sub(0x4000)) * scale);
        let scale = eng_math::cos_s(r_yaw.wrapping_sub(0x4000)) * scale;
        let input_yaw = play.input_dir_yaw();
        pos.x = this.pos.x - (eng_math::cos_s(input_yaw) * scale);
        pos.z = this.pos.z + (eng_math::sin_s(input_yaw) * scale);
        new_lightning(play, index, this, pos, yaw);
        if play.rand.zero_one() < 0.1 {
            // (this->rYaw * 2) - yaw: the slot's yaw, re-read.
            let mirrored = (this.regs[R_YAW] as i32 * 2 - yaw as i32) as i16;
            new_lightning(play, index, this, pos, mirrored);
        }
    }
}

fn bake_name(n: usize) -> String {
    format!("EffectSs/lightning/{n}")
}

/// One bake per texture (`lightningTextures`: `gEffLightning1Tex` to `8Tex` on segment 8):
/// `Gfx_SetupDL_61Xlu`, the colours dynamic, `gEffLightningDL`.
pub fn bakes() -> Vec<MeshBake> {
    let mut setup = setup_dl::setup_dl_61();
    setup.end();
    (1..=8)
        .map(|n| MeshBake {
            name: bake_name(n),
            object: "gameplay_keep".into(),
            segments: vec![
                (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: format!("gEffLightning{n}Tex") }),
                (SEG_SETUP, BakeSegment::Commands(setup.0.clone())),
                (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
            ],
            prelude: vec![SEG_SETUP, SEG_COLOR],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffLightningDL".into())]),
        })
        .collect()
}

/// The texture `EffectSsLightning_Draw` draws (1 to 8): `rLifespan - life`, at most 7 (never
/// below 0: `life` only falls from `rLifespan`).
pub fn texture(this: &EffectSs) -> usize {
    (this.regs[R_LIFESPAN] as i32 - this.life as i32).clamp(0, 7) as usize + 1
}

/// `EffectSsLightning_Draw`'s matrix: `SkinMatrix_SetTranslate` × `billboardMtxF` ×
/// `SkinMatrix_SetRotateZYX(vec.x, vec.y, rYaw)` (`vec` is never set: 0) × `SkinMatrix_SetScale`
/// (`rScale × 0.01` tall, 0.6 of that across and deep).
pub fn lightning_matrix(this: &EffectSs, billboard: Mat4) -> Mat4 {
    let y_scale = this.regs[R_SCALE] as f32 * 0.01;
    let xz_scale = y_scale * 0.6;
    let rotate = crate::skin::to_mat4(&crate::skin::set_rotate_zyx(this.vec.x as i16, this.vec.y as i16, this.regs[R_YAW]));
    Mat4::from_translation(this.pos) * billboard * rotate * Mat4::from_scale(Vec3::new(xz_scale, y_scale, xz_scale))
}

/// `EffectSsLightning_Draw`: the bolt, its colours.
pub fn draw(this: &EffectSs, billboard: Mat4, out: &mut EffectDraws) {
    let r = &this.regs;
    let m = lightning_matrix(this, billboard);
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, r[R_PRIM_COLOR_A] as u8];
    let env = [r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, r[R_ENV_COLOR_B] as u8, r[R_ENV_COLOR_A] as u8];
    out.xlu.push(colored(&bake_name(texture(this)), m, SEG_COLOR, prim, env));
}

impl SsSpawn<'_> {
    /// `EffectSsLightning_Spawn`: priority 128.
    #[allow(clippy::too_many_arguments)]
    pub fn lightning_spawn(&mut self, pos: Vec3, prim: [u8; 4], env: [u8; 4], scale: i16, yaw: i16, life: i16, num_bolts: i16) {
        let p = LightningInit { pos, prim_color: prim, env_color: env, scale, yaw, life, num_bolts };
        self.spawn(super::EFFECT_SS_LIGHTNING, 128, super::SsInit::Lightning(p));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bolt(life: i16, yaw: i16) -> EffectSs {
        let mut e = EffectSs::default();
        let p = LightningInit { pos: Vec3::new(5.0, 6.0, 7.0), prim_color: [255, 255, 255, 255], env_color: [0, 0, 255, 255], scale: 100, yaw, life, num_bolts: 3 };
        assert!(init(&mut e, &p));
        e
    }

    #[test]
    fn init_keeps_its_parameters_in_the_regs() {
        let e = bolt(8, 0x1000);
        assert_eq!((e.life, e.pos, e.velocity, e.accel, e.vec), (8, Vec3::new(5.0, 6.0, 7.0), Vec3::ZERO, Vec3::ZERO, Vec3::ZERO));
        assert_eq!(&e.regs[..12], &[255, 255, 255, 255, 0, 0, 255, 255, 3, 100, 0x1000, 8]);
    }

    #[test]
    fn a_texture_a_frame_from_the_first_the_eighth_held() {
        let mut e = bolt(12, 0);
        let mut seen = Vec::new();
        for life in (0..=12).rev() {
            e.life = life;
            seen.push(texture(&e));
        }
        assert_eq!(seen, [1, 2, 3, 4, 5, 6, 7, 8, 8, 8, 8, 8, 8]);
    }

    #[test]
    fn drawn_facing_the_camera_turned_by_its_yaw() {
        let e = bolt(8, 0x4000);
        let bb = Mat4::from_rotation_y(0.5);
        // A quarter turn about z (the view axis once the billboard's applied); 1 tall, 0.6 across.
        let rz = crate::skin::to_mat4(&crate::skin::set_rotate_zyx(0, 0, 0x4000));
        let want = Mat4::from_translation(e.pos) * bb * rz * Mat4::from_scale(Vec3::new(0.6, 1.0, 0.6));
        assert!(lightning_matrix(&e, bb).abs_diff_eq(want, 1e-6));
        // rz takes x to y: the bolt's x axis (scaled 0.6) points up the screen.
        assert!((rz * Vec3::X.extend(0.0)).truncate().abs_diff_eq(Vec3::Y, 1e-3));
    }
}
