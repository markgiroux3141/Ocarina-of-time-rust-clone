//! `Effect_Ss_En_Ice` (`ovl_Effect_Ss_En_Ice/z_eff_ss_en_ice.c`): a clump of ice. Type 0 sits
//! on an actor frozen blue (`EffectSsEnIce_SpawnFlyingVec3f`: the Keese's frozen death) and
//! flies off when the freeze ends; type 1 is a free one with its own velocity.

use eng_math::{atan2_s, cos_s, sin_s, vec3f_yaw};
use glam::{Mat4, Vec3};

use super::{DrawCtx, EffectDraws, EffectSs, SEG_COLOR, SEG_TEX, SsActor, SsDraw, SsSpawn, SsUpdate};
use crate::pack::{BakeBody, BakeSegment, MeshBake, keys};
use crate::play::PlayState;
use crate::scene_table::gfx_two_tex_scroll;
use crate::sys_matrix::binang_to_rad;

// The overlay's regs.
const R_LIFESPAN: usize = 0;
const R_YAW: usize = 1;
const R_PITCH: usize = 2;
const R_ROT_SPEED: usize = 3;
const R_PRIM_COLOR_R: usize = 4;
const R_PRIM_COLOR_G: usize = 5;
const R_PRIM_COLOR_B: usize = 6;
const R_PRIM_COLOR_A: usize = 7;
const R_ENV_COLOR_R: usize = 8;
const R_ENV_COLOR_G: usize = 9;
const R_ENV_COLOR_B: usize = 10;
const R_ALPHA_MODE: usize = 11;
const R_SCALE: usize = 12;

const BAKE: &str = "EffectSs/en_ice";

/// `EffectSsEnIceInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct EnIceInit {
    pub actor: Option<SsActor>,
    pub pos: Vec3,
    pub scale: f32,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub prim_color: [u8; 4],
    pub env_color: [u8; 4],
    pub life: i32,
    pub ty: i16,
}

/// `EffectSsEnIce_Init`.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &EnIceInit) -> bool {
    let set_colors = |r: &mut [i16; 13]| {
        r[R_PRIM_COLOR_R] = p.prim_color[0] as i16;
        r[R_PRIM_COLOR_G] = p.prim_color[1] as i16;
        r[R_PRIM_COLOR_B] = p.prim_color[2] as i16;
        r[R_PRIM_COLOR_A] = p.prim_color[3] as i16;
        r[R_ENV_COLOR_R] = p.env_color[0] as i16;
        r[R_ENV_COLOR_G] = p.env_color[1] as i16;
        r[R_ENV_COLOR_B] = p.env_color[2] as i16;
    };
    match p.ty {
        0 => {
            this.pos = p.pos;
            // initParams->actor->world.pos (NULL would crash in the C).
            let apos = p.actor.map(|a| a.world_pos).unwrap_or(Vec3::ZERO);
            this.vec = this.pos - apos;
            this.velocity = Vec3::ZERO;
            this.accel = Vec3::ZERO;
            this.life = 10;
            this.actor = p.actor.map(|a| a.handle);
            this.draw = Some(SsDraw::EnIce);
            this.update = Some(SsUpdate::EnIceFlying);
            this.regs[R_SCALE] = (p.scale * 100.0) as i16;
            set_colors(&mut this.regs);
            this.regs[R_ALPHA_MODE] = 1;
            this.regs[R_PITCH] = s.rand.centered_float(65536.0) as i32 as i16;
        }
        1 => {
            this.pos = p.pos;
            this.vec = p.pos;
            this.velocity = p.velocity;
            this.accel = p.accel;
            this.life = p.life as i16;
            this.draw = Some(SsDraw::EnIce);
            this.update = Some(SsUpdate::EnIce);
            this.regs[R_LIFESPAN] = p.life as i16;
            this.regs[R_SCALE] = (p.scale * 100.0) as i16;
            this.regs[R_YAW] = atan2_s(p.velocity.z, p.velocity.x);
            this.regs[R_PITCH] = 0;
            set_colors(&mut this.regs);
            this.regs[R_ALPHA_MODE] = 0;
        }
        _ => {
            log::error!("Effect_Ss_En_Ice_ct():pid->mode_sw is an error.");
            return false;
        }
    }
    true
}

/// `EffectSsEnIce_UpdateFlying`: held on its actor while it's frozen blue (the life held at 9
/// or more); then thrown off away from the actor, or in a random direction once the actor's
/// gone (the NTSC 1.1 and later check, which gc-eu-mq-dbg has).
pub fn update_flying(play: &mut PlayState, this: &mut EffectSs) {
    let actor = this.actor.and_then(|h| play.actors.actor(h)).filter(|a| !a.killed).map(|a| (a.world_pos, a.color_filter_timer, a.color_filter_params));
    match actor {
        Some((apos, timer, params)) => {
            if this.life >= 9 && timer != 0 && params & 0xC000 == 0 {
                this.pos = apos + this.vec;
                this.life += 1;
            } else if this.life == 9 {
                let yaw = vec3f_yaw(apos, this.pos);
                this.accel.x = sin_s(yaw) * (play.rand.zero_one() + 1.0);
                this.accel.z = cos_s(vec3f_yaw(apos, this.pos)) * (play.rand.zero_one() + 1.0);
                this.accel.y = -1.5;
                this.velocity.y = 5.0;
            }
        }
        None => {
            if this.life >= 9 {
                let rand = play.rand.centered_float(65535.0) as i32 as i16;
                this.accel.x = sin_s(rand) * (play.rand.zero_one() + 1.0);
                this.accel.z = cos_s(rand) * (play.rand.zero_one() + 1.0);
                this.life = 8;
                this.accel.y = -1.5;
                this.velocity.y = 5.0;
            }
        }
    }
}

/// `EffectSsEnIce_Update`: `rRotSpeed` is never set, so the pitch stays.
pub fn update(this: &mut EffectSs) {
    this.regs[R_PITCH] = this.regs[R_PITCH].wrapping_add(this.regs[R_ROT_SPEED]);
}

/// `Gfx_TwoTexScroll(.., 0, 0, frames & 0xFF, 0x20, 0x10, 1, 0, (frames * 2) & 0xFF, 0x40, 0x20)`.
fn ice_scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, frames & 0xFF, 0x20, 0x10, 1, 0, frames.wrapping_mul(2) & 0xFF, 0x40, 0x20)
}

/// `gEffIceFragment2DL` after `Gfx_SetupDL_25Xlu`, the colours (prim LOD fraction 0x80) and the
/// scroll dynamic. (`func_8002EB44`'s highlight tiles aren't baked.)
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: "gameplay_keep".into(),
        segments: vec![(SEG_COLOR, BakeSegment::Dynamic(vec![(0xFA00_0080, 0x0000_00FF), (0xFB00_0000, 0x0000_0000), (0xDF00_0000, 0)])), (SEG_TEX, BakeSegment::Dynamic(ice_scroll(0)))],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffIceFragment2DL".into())]),
    }]
}

/// `EffectSsEnIce_Draw`: scaled `rScale × 0.01`, turned by its yaw and pitch; type 0 fades by
/// `life × 12`, type 1 over the second half of its life.
pub fn draw(this: &EffectSs, ctx: &DrawCtx<'_>, out: &mut EffectDraws) {
    let r = &this.regs;
    let scale = r[R_SCALE] as f32 * 0.01;
    let alpha = if r[R_ALPHA_MODE] != 0 {
        (this.life as i32 * 12) as f32
    } else if r[R_LIFESPAN] > 0 && this.life < (r[R_LIFESPAN] >> 1) {
        ((this.life as f32 * 2.0) / r[R_LIFESPAN] as f32) * 255.0
    } else {
        255.0
    };
    let m = Mat4::from_translation(this.pos) * Mat4::from_scale(Vec3::splat(scale)) * Mat4::from_rotation_y(binang_to_rad(r[R_YAW])) * Mat4::from_rotation_x(binang_to_rad(r[R_PITCH]));
    let mut sv = eng_gfx::SegmentValues::default();
    sv.read(SEG_TEX, &ice_scroll(ctx.gameplay_frames));
    sv.prim[SEG_COLOR as usize] = Some([r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, r[R_PRIM_COLOR_B] as u8, r[R_PRIM_COLOR_A] as u8]);
    sv.env[SEG_COLOR as usize] = Some([r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, r[R_ENV_COLOR_B] as u8, alpha as u32 as u8]);
    out.xlu.push(eng_gfx::DrawCmd { mesh: eng_gfx::MeshKey::named(keys::bake(BAKE)), transform: m, bones: Vec::new(), params: eng_gfx::DrawParams { segments: Some(sv), ..Default::default() } });
}
