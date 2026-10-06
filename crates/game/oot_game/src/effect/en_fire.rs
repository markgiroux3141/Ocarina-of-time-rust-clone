//! `Effect_Ss_En_Fire` (`ovl_Effect_Ss_En_Fire/z_eff_ss_en_fire.c`): a flame on an actor (an
//! enemy set alight), following it: at an offset turning with it, or at one of its fire
//! positions (`firePos`, the actor's table at 0x14C: `ActorImpl::effect_fire_pos`).

use eng_math::{atan2_s, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};

use super::{DrawCtx, EffectDraws, EffectSs, SEG_COLOR, SEG_TEX, SsActor, SsDraw, SsSpawn, SsUpdate};
use crate::pack::{BakeBody, BakeSegment, MeshBake, keys};
use crate::play::PlayState;
use crate::scene_table::gfx_two_tex_scroll;
use crate::sys_matrix::{MtxF, binang_to_rad};

// The overlay's regs.
const R_SCALE_MAX: usize = 0;
const R_SCALE: usize = 1;
const R_LIFESPAN: usize = 2;
const R_UNUSED: usize = 3;
const R_PITCH: usize = 4;
const R_YAW: usize = 5;
const R_REG6: usize = 6;
const R_BODY_PART: usize = 7;
const R_FLAGS: usize = 8;
const R_SCROLL: usize = 9;

/// `EffectSsEnFireInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct EnFireInit {
    pub actor: Option<SsActor>,
    pub pos: Vec3,
    pub scale: i16,
    pub unk_12: i16,
    pub flags: i16,
    pub body_part: i16,
}

/// `Math_Vec3f_Pitch`.
fn vec3f_pitch(a: Vec3, b: Vec3) -> i16 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    atan2_s((dx * dx + dz * dz).sqrt(), a.y - b.y)
}

/// `EffectSsEnFire_Init`: 20 frames; without a body part, the offset from the actor in its
/// own turn and pitch; growing from 0 unless `unk_12`'s 0x8000.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &EnFireInit) -> bool {
    this.pos = p.pos;
    this.velocity = Vec3::ZERO;
    this.accel = Vec3::ZERO;
    this.life = 20;
    this.regs[R_LIFESPAN] = this.life;
    this.actor = p.actor.map(|a| a.handle);
    this.regs[R_SCROLL] = (s.rand.zero_one() * 20.0) as i16;
    this.draw = Some(SsDraw::EnFire);
    this.update = Some(SsUpdate::EnFire);
    this.regs[R_UNUSED] = -15;
    if p.body_part < 0 {
        // The C reads the actor through initParams->actor (NULL would crash).
        if let Some(a) = p.actor {
            this.regs[R_YAW] = eng_math::vec3f_yaw(a.world_pos, p.pos).wrapping_sub(a.shape_rot[1]);
            this.regs[R_PITCH] = vec3f_pitch(a.world_pos, p.pos).wrapping_sub(a.shape_rot[0]);
            this.vec.z = (p.pos - a.world_pos).length();
        }
    }
    this.regs[R_SCALE_MAX] = p.scale;
    this.regs[R_SCALE] = if p.unk_12 as u16 & 0x8000 != 0 { p.scale } else { 0 };
    this.regs[R_REG6] = p.unk_12 & 0x7FFF;
    this.regs[R_BODY_PART] = p.body_part;
    this.regs[R_FLAGS] = p.flags;
    true
}

/// `EffectSsEnFire_Update`: the scroll steps; held while the actor flashes (`colorFilterTimer`
/// ≥ 22); while the actor lives the flame grows to its size and follows it; once it's gone,
/// the flame ends (`rReg6`) or stays where it was.
pub fn update(play: &mut PlayState, this: &mut EffectSs) {
    this.regs[R_SCROLL] = this.regs[R_SCROLL].wrapping_add(1);
    let Some(h) = this.actor else { return };
    // An actor deleted since reads as it was left: update NULL (killed), its filter's timer.
    let actor = play.actors.get(h);
    let base = actor.map(|a| a.base());
    if base.is_some_and(|a| a.color_filter_timer >= 22) {
        this.life += 1;
    }
    if let (Some(a), Some(b)) = (actor, base.filter(|b| !b.killed)) {
        let max = this.regs[R_SCALE_MAX];
        smooth_step_to_s(&mut this.regs[R_SCALE], max, 1, max >> 3, 0);
        if this.regs[R_BODY_PART] < 0 {
            let mut m = MtxF::set_translate(b.world_pos.x, b.world_pos.y, b.world_pos.z);
            m.rotate_y(binang_to_rad(this.regs[R_YAW].wrapping_add(b.shape_rot.y)));
            m.rotate_x(binang_to_rad(this.regs[R_PITCH].wrapping_add(b.shape_rot.x)));
            this.pos = m.mult_vec3f(this.vec);
        } else {
            let vec3s = this.regs[R_FLAGS] as u16 & 0x8000 != 0;
            this.pos = a.effect_fire_pos(this.regs[R_BODY_PART] as usize, vec3s);
        }
    } else if this.regs[R_REG6] != 0 {
        this.life = 0;
    } else {
        this.actor = None;
    }
}

pub(crate) fn bake_name(dl: u8) -> String {
    format!("EffectSs/fire/{dl}")
}

/// `Gfx_TwoTexScroll(.., 0, 0, 0, 32, 64, 1, 0, y2, 32, 128)`: the flames' scroll.
pub(crate) fn fire_scroll(y2: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, 0, 32, 64, 1, 0, y2 & 0x1FF, 32, 128)
}

/// `gEffFire1DL` and `gEffFire2DL` after `Gfx_SetupDL_25Xlu` (the bake's start), their colours
/// (prim with LOD fraction 0x80) and the scroll on segment 8 dynamic.
pub fn bakes() -> Vec<MeshBake> {
    [1u8, 2]
        .iter()
        .map(|&n| MeshBake {
            name: bake_name(n),
            object: "gameplay_keep".into(),
            segments: vec![(SEG_COLOR, BakeSegment::Dynamic(vec![(0xFA00_0080, 0x0000_00FF), (0xFB00_0000, 0x0000_0000), (0xDF00_0000, 0)])), (SEG_TEX, BakeSegment::Dynamic(fire_scroll(0)))],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), format!("gEffFire{n}DL"))]),
        })
        .collect()
}

/// A flame's draw: `bake` at `m` with the colours and the scroll.
pub(crate) fn flame_draw(bake: &str, m: Mat4, prim: [u8; 4], env: [u8; 4], scroll_y2: u32) -> eng_gfx::DrawCmd {
    let mut sv = eng_gfx::SegmentValues::default();
    sv.read(SEG_TEX, &fire_scroll(scroll_y2));
    sv.prim[SEG_COLOR as usize] = Some(prim);
    sv.env[SEG_COLOR as usize] = Some(env);
    eng_gfx::DrawCmd { mesh: eng_gfx::MeshKey::named(keys::bake(bake)), transform: m, bones: Vec::new(), params: eng_gfx::DrawParams { segments: Some(sv), ..Default::default() } }
}

/// `EffectSsEnFire_Draw`: turned to face the camera, swelling and shrinking over its life (`sin(life × 0x333)`),
/// red and yellow fading with its last 15 frames; `gEffFire2DL` with flags or under 18 frames
/// left, else `gEffFire1DL`.
pub fn draw(this: &EffectSs, ctx: &DrawCtx<'_>, out: &mut EffectDraws) {
    let r = &this.regs;
    let cam_yaw = ctx.cam_dir_yaw.wrapping_add(i16::MIN);
    let scale = sin_s((this.life as i32 * 0x333) as i16) * (r[R_SCALE] as f32 * 0.00005);
    let m = Mat4::from_translation(this.pos) * Mat4::from_rotation_y(binang_to_rad(cam_yaw)) * Mat4::from_scale(Vec3::splat(scale));
    let intensity = (this.life - 5).max(0);
    let rg = (intensity as f32 * 12.7) as i32 as u8;
    let env = [rg, 0, 0, 0];
    let prim = [rg, rg, 0, 255];
    let y2 = (r[R_SCROLL] as i32).wrapping_mul(-0x14) as u32 & 0x1FF;
    let dl = if r[R_FLAGS] & 0x7FFF != 0 || this.life < 18 { 2 } else { 1 };
    out.xlu.push(flame_draw(&bake_name(dl), m, prim, env, y2));
}
