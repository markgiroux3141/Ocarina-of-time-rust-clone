//! `EffectShieldParticle` (`z_eff_shield_particle.c`): streaks of light from a hit on
//! something solid (`CollisionCheck_SpawnShieldParticles`: a blocked hit on a metal shield or
//! a sword on stone), each shooting out in a random direction and drawing out behind it, with a
//! point light that halves every frame.

use glam::{Mat4, Vec3};

use super::spark::particle_bake;
use super::{EffectDraws, SEG_COLOR};
use crate::gbi::setup_dl;
use crate::lights::{LightInfo, LightNode};
use crate::pack::{BakeSegment, MeshBake};
use crate::play::PlayState;
use crate::sys_matrix::binang_to_rad;

const BAKE: &str = "Effect/shield_particle";

/// `EffectShieldParticleElement`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EffectShieldParticleElement {
    pub initial_speed: f32,
    pub end_x_change: f32,
    pub end_x: f32,
    pub start_x_change: f32,
    pub start_x: f32,
    pub yaw: i16,
    pub pitch: i16,
}

/// `EffectShieldParticleInit`.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectShieldParticleInit {
    pub num_elements: u8,
    /// `position` (a `Vec3s`).
    pub position: [i16; 3],
    pub prim_color_start: [u8; 4],
    pub env_color_start: [u8; 4],
    pub prim_color_mid: [u8; 4],
    pub env_color_mid: [u8; 4],
    pub prim_color_end: [u8; 4],
    pub env_color_end: [u8; 4],
    pub deceleration: f32,
    pub max_initial_speed: f32,
    pub length_cutoff: f32,
    pub duration: u8,
    /// `lightPoint`: a `LIGHT_POINT_NOGLOW`'s position, colour, glow and radius.
    pub light_point: LightInfo,
    /// `lightDecay`: the light, its radius halved every frame.
    pub light_decay: bool,
}

/// `EffectShieldParticle`.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectShieldParticle {
    pub elements: Vec<EffectShieldParticleElement>,
    pub position: [i16; 3],
    pub prim_color_start: [u8; 4],
    pub env_color_start: [u8; 4],
    pub prim_color_mid: [u8; 4],
    pub env_color_mid: [u8; 4],
    pub prim_color_end: [u8; 4],
    pub env_color_end: [u8; 4],
    pub deceleration: f32,
    pub max_initial_speed: f32,
    pub length_cutoff: f32,
    pub duration: u8,
    pub timer: u8,
    pub light_info: LightInfo,
    pub light_node: Option<LightNode>,
    pub light_decay: bool,
}

/// `ARRAY_COUNT(this->elements)`.
const MAX_ELEMENTS: usize = 16;

/// `EffectShieldParticle_Init`: each streak at half to all of the maximum speed in a random
/// yaw and pitch; with `lightDecay`, its light added (`LightContext_InsertLight`).
pub fn init(play: &mut PlayState, p: &EffectShieldParticleInit) -> EffectShieldParticle {
    let n = (p.num_elements as usize).min(MAX_ELEMENTS);
    if p.num_elements as usize > MAX_ELEMENTS {
        log::error!("EffectShieldParticle_ct(): Number of particles exceeded.");
    }
    let mut elements = Vec::with_capacity(n);
    for _ in 0..n {
        let initial_speed = (play.rand.zero_one() * (p.max_initial_speed * 0.5)) + (p.max_initial_speed * 0.5);
        let yaw = (play.rand.zero_one() * 65534.0) as i32 as i16;
        let pitch = (play.rand.zero_one() * 65534.0) as i32 as i16;
        elements.push(EffectShieldParticleElement { initial_speed, end_x_change: initial_speed, end_x: 0.0, start_x_change: 0.0, start_x: 0.0, yaw, pitch });
    }
    let light_info = p.light_point;
    let light_node = if p.light_decay { play.light_ctx.insert_light(light_info) } else { None };
    EffectShieldParticle {
        elements,
        position: p.position,
        prim_color_start: p.prim_color_start,
        env_color_start: p.env_color_start,
        prim_color_mid: p.prim_color_mid,
        env_color_mid: p.env_color_mid,
        prim_color_end: p.prim_color_end,
        env_color_end: p.env_color_end,
        deceleration: p.deceleration,
        max_initial_speed: p.max_initial_speed,
        length_cutoff: p.length_cutoff,
        duration: p.duration,
        timer: 0,
        light_info,
        light_node,
        light_decay: p.light_decay,
    }
}

/// `EffectShieldParticle_Destroy`: the light removed.
pub fn destroy(play: &mut PlayState, s: &EffectShieldParticle) {
    if s.light_decay {
        play.light_ctx.remove_light(s.light_node);
    }
}

/// `EffectShieldParticle_Update`: each streak's head slows by `deceleration`; its tail starts
/// after it once the streak is longer than `lengthCutoff`; the light's radius halves. Done once
/// past its duration.
pub fn update(play: &mut PlayState, s: &mut EffectShieldParticle) -> bool {
    for e in &mut s.elements {
        e.end_x_change -= s.deceleration;
        if e.end_x_change < 0.0 {
            e.end_x_change = 0.0;
        }
        if e.start_x_change > 0.0 {
            e.start_x_change -= s.deceleration;
            if e.start_x_change < 0.0 {
                e.start_x_change = 0.0;
            }
        }
        e.end_x += e.end_x_change;
        e.start_x += e.start_x_change;
        if e.start_x_change == 0.0 && s.length_cutoff < e.end_x - e.start_x {
            e.start_x_change = e.initial_speed;
        }
    }
    if s.light_decay {
        // The node points at lightInfo: the context sees the new radius.
        s.light_info.radius /= 2;
        play.light_ctx.set_info(s.light_node, s.light_info);
    }
    s.timer = s.timer.wrapping_add(1);
    s.duration < s.timer
}

/// `EffectShieldParticle_GetColors`: from the start to the middle colours over the first half
/// of the duration, then to the end ones.
pub fn get_colors(s: &EffectShieldParticle) -> ([u8; 4], [u8; 4]) {
    let half = (s.duration as f32 * 0.5) as i32;
    let lerp = |a: [u8; 4], b: [u8; 4], t: f32| -> [u8; 4] { std::array::from_fn(|i| (a[i] as i32 as f32 + (b[i] as i32 - a[i] as i32) as f32 * t) as i32 as u8) };
    if half == 0 {
        (s.prim_color_start, s.env_color_start)
    } else if (s.timer as i32) < half {
        let t = s.timer as f32 / half as f32;
        (lerp(s.prim_color_start, s.prim_color_mid, t), lerp(s.env_color_start, s.env_color_mid, t))
    } else {
        let t = (s.timer as i32 - half) as f32 / half as f32;
        (lerp(s.prim_color_mid, s.prim_color_end, t), lerp(s.env_color_mid, s.env_color_end, t))
    }
}

/// The bake: as the spark's, with the combiner `PRIMITIVE, ENVIRONMENT, TEXEL0, ENVIRONMENT,
/// PRIMITIVE, 0, TEXEL0, 0, 0, 0, 0, COMBINED, 0, 0, 0, COMBINED`, the colours dynamic, the quad
/// as (0, 1, 2) and (0, 3, 1).
pub fn bakes() -> Vec<MeshBake> {
    let mut b = particle_bake(BAKE, [[0, 1, 2], [0, 3, 1]], |d| d.combine_lerp(setup_dl::PRIM_ENV_TEXEL0, setup_dl::PASS2));
    b.segments.push((SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }));
    // The colours after the setup (gDPSetPrimColor and gDPSetEnvColor follow the modes).
    b.prelude = vec![SEG_COLOR, super::SEG_SETUP];
    vec![b]
}

/// `EffectShieldParticle_Draw`: each streak a quad from its tail to its head, at the
/// position turned by its yaw then its pitch about z, 0.02 thick.
pub fn draw(s: &EffectShieldParticle, out: &mut EffectDraws) {
    let (prim, env) = get_colors(s);
    let pos = Vec3::new(s.position[0] as f32, s.position[1] as f32, s.position[2] as f32);
    for e in &s.elements {
        let temp1 = ((e.end_x + e.start_x) * 0.5) as i32 as i16 as f32;
        let temp2 = e.end_x - e.start_x;
        let mut temp3 = ((temp2 * (1.0 / 64.0)) / 0.02) as i32 as i16 as f32;
        if temp3 < 1.0 {
            temp3 = 1.0;
        }
        let m = Mat4::from_translation(pos)
            * Mat4::from_rotation_y(binang_to_rad(e.yaw))
            * Mat4::from_rotation_z(binang_to_rad(e.pitch))
            * Mat4::from_translation(Vec3::new(temp1, 0.0, 0.0))
            * Mat4::from_scale(Vec3::new(temp3 * 0.02, 0.02, 0.02));
        out.xlu.push(super::colored(BAKE, m, SEG_COLOR, prim, env));
    }
}
