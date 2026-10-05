//! `Effect_Ss_Fire_Tail` (`ovl_Effect_Ss_Fire_Tail/z_eff_ss_fire_tail.c`): a flame on
//! something burning: on Player, one per burning body part every frame
//! (`EffectSsFireTail_SpawnFlameOnPlayer` from `Player_UpdateBodyBurn`), leaning against the
//! actor's movement.

use eng_math::{cos_s, sin_s, vec3f_yaw};
use glam::{Mat4, Vec3};

use super::en_fire::flame_draw;
use super::{DrawCtx, EffectDraws, EffectSs, SsDraw, SsUpdate};
use crate::actor_ctx::ActorHandle;
use crate::sys_matrix::binang_to_rad;

// The overlay's regs.
const R_SCALE: usize = 0;
const R_LIFESPAN: usize = 1;
const R_REG2: usize = 2;
const R_REG3: usize = 3;
const R_PRIM_COLOR_R: usize = 4;
const R_PRIM_COLOR_G: usize = 5;
const R_PRIM_COLOR_B: usize = 6;
const R_ENV_COLOR_R: usize = 7;
const R_ENV_COLOR_G: usize = 8;
const R_ENV_COLOR_B: usize = 9;
const R_REG10: usize = 10;
const R_BODY_PART: usize = 11;
const R_TYPE: usize = 12;

/// `EffectSsFireTailInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct FireTailInit {
    pub actor: Option<ActorHandle>,
    pub pos: Vec3,
    pub scale: f32,
    pub unk_14: Vec3,
    pub unk_20: i16,
    pub prim_color: [u8; 4],
    pub env_color: [u8; 4],
    pub ty: i16,
    pub body_part: i16,
    pub life: i32,
}

/// `EffectSsFireTail_Init`.
pub fn init(this: &mut EffectSs, p: &FireTailInit) -> bool {
    this.pos = p.pos;
    this.vec = p.unk_14;
    this.velocity = Vec3::ZERO;
    this.accel = Vec3::ZERO;
    this.life = p.life as i16;
    this.actor = p.actor;
    this.draw = Some(SsDraw::FireTail);
    this.update = Some(SsUpdate::FireTail);
    let r = &mut this.regs;
    r[R_SCALE] = (p.scale * 1000.0) as i16;
    r[R_LIFESPAN] = p.life as i16;
    r[R_REG2] = -0xA;
    r[R_REG3] = -0xF;
    r[R_REG10] = if p.unk_20 == 0 { 1 } else { p.unk_20 };
    r[R_PRIM_COLOR_R] = p.prim_color[0] as i16;
    r[R_PRIM_COLOR_G] = p.prim_color[1] as i16;
    r[R_PRIM_COLOR_B] = p.prim_color[2] as i16;
    r[R_ENV_COLOR_R] = p.env_color[0] as i16;
    r[R_ENV_COLOR_G] = p.env_color[1] as i16;
    r[R_ENV_COLOR_B] = p.env_color[2] as i16;
    r[R_BODY_PART] = p.body_part;
    r[R_TYPE] = p.ty;
    true
}

/// `EffectSsFireTail_Update`: the scale × 0.9.
pub fn update(this: &mut EffectSs) {
    this.regs[R_SCALE] = (this.regs[R_SCALE] as f32 * 0.9) as i16;
}

/// `EffectSsFireTail_Draw`: following the actor (its velocity into `vec`; a body part's flame
/// moves to that part of Player's, 5 towards the camera), facing the camera, leaning with the
/// velocity across the view and stretched along it, shrinking over its life; `gEffFire2DL`
/// unless `rType` is 0.
pub fn draw(this: &mut EffectSs, ctx: &DrawCtx<'_>, out: &mut EffectDraws) {
    let cam = ctx.cam_dir_yaw;
    let actor = this.actor.and_then(|h| ctx.actors.actor(h));
    let t = match (this.actor, actor) {
        (Some(_), Some(a)) => {
            this.vec = a.velocity;
            if this.regs[R_BODY_PART] < 0 {
                this.pos + a.world_pos
            } else {
                let bp = ctx.player_body_parts.as_ref().and_then(|b| b.get(this.regs[R_BODY_PART] as usize).copied()).unwrap_or(this.pos);
                this.pos = Vec3::new(bp.x - sin_s(cam) * 5.0, bp.y, bp.z - cos_s(cam) * 5.0);
                this.pos
            }
        }
        _ => this.pos,
    };
    let r = &this.regs;
    let yaw = vec3f_yaw(Vec3::ZERO, this.vec).wrapping_sub(cam);
    let temp1 = cos_s(yaw).abs();
    let temp2 = sin_s(yaw);
    let dist = (this.vec.x * this.vec.x + this.vec.z * this.vec.z).sqrt() / (r[R_REG10] as f32 * 0.1);
    let roll = (temp2 * r[R_REG2] as f32 * dist) * (std::f32::consts::PI / 180.0);
    let t2 = 1.0 - ((this.life + 1) as f32 / r[R_LIFESPAN] as f32);
    let t2 = 1.0 - t2 * t2;
    let scale = t2 * (r[R_SCALE] as f32 * 0.000010000001);
    let stretch = ((r[R_REG3] as f32 * 0.01 * temp1 * dist) + 1.0).max(0.1);
    let m = Mat4::from_translation(t)
        * Mat4::from_rotation_y(binang_to_rad(cam.wrapping_add(i16::MIN)))
        * Mat4::from_rotation_z(roll)
        * Mat4::from_scale(Vec3::splat(scale))
        * Mat4::from_scale(Vec3::new(1.0, stretch, 1.0 / stretch));
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, 255];
    let env = [r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, r[R_ENV_COLOR_B] as u8, 0];
    let y2 = (ctx.state_frames as i32).wrapping_mul(-0x14) as u32 & 0x1FF;
    let bake = if r[R_TYPE] != 0 { "EffectSs/fire/2" } else { "EffectSs/fire/1" };
    out.xlu.push(flame_draw(bake, m, prim, env, y2));
}
