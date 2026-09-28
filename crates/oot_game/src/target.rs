//! Z-targeting's actor side: dummy target actors and the target context
//! (`TargetContext` in `z_actor.c`).
//!
//! - Every frame after the actors update, `Actor_UpdateAll` refreshes each actor's distance and
//!   yaw to Player (`xzDistToPlayer`, `yDistToPlayer`, `xyzDistToPlayerSq`, `yawTowardsPlayer`).
//! - `func_8002C7BC` then picks what the Z button would lock on to next (`arrowPointedActor`,
//!   via `func_80032AF0` / `func_800328D4` / `func_8002EFC0`), and steps the lock-on reticle
//!   (`unk_44` → 80, then `unk_4B` counts). Player turns to face a target only once `unk_4B != 0`.
//!
//! The dummy target stands in for an enemy: `ACTOR_FLAG_0 | ACTOR_FLAG_2` (targetable, hostile),
//! a `targetMode`, and a focus point above its position. Navi, the reticle's drawing and the
//! BGM-enemy tracking are not modelled.

#![allow(non_snake_case)] // unk_4B keeps the decomp's name

use glam::{Mat4, Vec3};

use crate::bgcheck::{self, StaticCollision};
use crate::math::atan2_s;

pub const ACTOR_FLAG_0: u32 = 1 << 0;
pub const ACTOR_FLAG_2: u32 = 1 << 2;
pub const ACTOR_FLAG_27: u32 = 1 << 27;

/// An actor Player can target.
#[derive(Debug, Clone)]
pub struct TargetActor {
    pub pos: Vec3,
    pub focus: Vec3,
    pub flags: u32,
    pub target_mode: u8,
    /// `targetPriority` (0 = normal).
    pub target_priority: u8,
    pub xz_dist_to_player: f32,
    pub y_dist_to_player: f32,
    pub xyz_dist_to_player_sq: f32,
    pub yaw_towards_player: i16,
}

impl TargetActor {
    /// A dummy enemy standing at `pos`, focus 40 units up, `targetMode` 3 (350 range).
    pub fn dummy(pos: Vec3) -> TargetActor {
        TargetActor {
            pos,
            focus: pos + Vec3::Y * 40.0,
            flags: ACTOR_FLAG_0 | ACTOR_FLAG_2,
            target_mode: 3,
            target_priority: 0,
            xz_dist_to_player: f32::MAX,
            y_dist_to_player: 0.0,
            xyz_dist_to_player_sq: f32::MAX,
            yaw_towards_player: 0,
        }
    }

    pub fn is_hostile(&self) -> bool {
        self.flags & (ACTOR_FLAG_0 | ACTOR_FLAG_2) == ACTOR_FLAG_0 | ACTOR_FLAG_2
    }

    /// `Actor_UpdateAll`'s per-actor distances to Player.
    pub fn update_distances(&mut self, player_pos: Vec3) {
        let d = player_pos - self.pos;
        self.xz_dist_to_player = (d.x * d.x + d.z * d.z).sqrt();
        self.y_dist_to_player = player_pos.y - self.pos.y;
        self.xyz_dist_to_player_sq = self.xz_dist_to_player * self.xz_dist_to_player + self.y_dist_to_player * self.y_dist_to_player;
        // Actor_WorldYawTowardActor: Math_Vec3f_Yaw(actor, player).
        self.yaw_towards_player = atan2_s(d.z, d.x);
    }
}

/// `TargetContext` fields Player reads.
#[derive(Debug, Clone, Default)]
pub struct TargetCtx {
    /// `arrowPointedActor`: what a Z press locks on to.
    pub arrow_pointed: Option<usize>,
    /// `unk_94`: the next candidate while one is locked (Z again switches to it).
    pub unk_94: Option<usize>,
    /// `targetedActor`.
    pub targeted: Option<usize>,
    /// `unk_44`: reticle size, stepping from 500 to 80.
    pub unk_44: f32,
    /// `unk_4B`: non-zero once the reticle has locked.
    pub unk_4B: u8,
}

/// What Player needs from the targeting system when it updates.
#[derive(Debug, Clone, Copy, Default)]
pub struct TargetView {
    pub arrow_pointed: Option<usize>,
    pub unk_94: Option<usize>,
    pub reticle_locked: bool,
}

impl TargetCtx {
    pub fn new() -> TargetCtx {
        TargetCtx { unk_44: 500.0, ..Default::default() }
    }

    pub fn view(&self) -> TargetView {
        TargetView { arrow_pointed: self.arrow_pointed, unk_94: self.unk_94, reticle_locked: self.unk_4B != 0 }
    }
}

/// The inputs of the end-of-frame target update.
pub struct TargetFrame<'a> {
    pub col: &'a StaticCollision,
    pub ranges: &'a [(f32, f32)],
    /// Player's `unk_664` (locked target), `unk_66C` (target timer), `unk_84B[unk_846]`.
    pub player_target: Option<usize>,
    pub player_timer: i16,
    pub player_stick_dir: i8,
    pub player_shape_yaw: i16,
    /// `player->actor.focus.pos` (the head).
    pub player_focus: Vec3,
    /// `play->viewProjectionMtxF` (the game's projection of the last frame).
    pub view_proj: Mat4,
}

/// `func_8002F090`.
pub fn in_range(ranges: &[(f32, f32)], a: &TargetActor, dist: f32) -> bool {
    dist < ranges.get(a.target_mode as usize).map(|r| r.0).unwrap_or(0.0)
}

/// `func_8002EFC0`: distance weighted by how far off Player's facing the actor is.
pub fn weighted_dist(a: &TargetActor, has_target: bool, player_yaw: i16) -> f32 {
    let yaw = a.yaw_towards_player.wrapping_sub(i16::MIN).wrapping_sub(player_yaw);
    let abs = (yaw as i32).abs();
    if has_target {
        if abs > 0x4000 || a.flags & ACTOR_FLAG_27 != 0 {
            f32::MAX
        } else {
            a.xyz_dist_to_player_sq - a.xyz_dist_to_player_sq * 0.8 * ((0x4000 - abs) as f32 * (1.0 / 32768.0))
        }
    } else if abs > 0x2AAA {
        f32::MAX
    } else {
        a.xyz_dist_to_player_sq
    }
}

/// `func_8002F0C8`: should Player lose `a` (out of range, or behind and not locked with `flag`)?
pub fn lost(ranges: &[(f32, f32)], a: &TargetActor, player_has_target: bool, player_yaw: i16, flag: bool) -> bool {
    if a.flags & ACTOR_FLAG_0 == 0 {
        return true;
    }
    if !flag {
        let var = a.yaw_towards_player.wrapping_sub(i16::MIN).wrapping_sub(player_yaw);
        let dist = if !player_has_target && (var as i32).abs() > 0x2AAA { f32::MAX } else { a.xyz_dist_to_player_sq };
        let leash = ranges.get(a.target_mode as usize).map(|r| r.1).unwrap_or(1.0);
        return !in_range(ranges, a, leash * dist);
    }
    false
}

/// `func_80032880`: `Actor_GetScreenPos` inside (-20..340, -160..400) on the 320x240 screen.
fn on_screen(view_proj: Mat4, a: &TargetActor) -> bool {
    let p = view_proj * a.focus.extend(1.0);
    // Actor_GetScreenPos uses the focus position: x = 160 + x/w * 160, y = 120 - y/w * 120.
    let inv_w = if p.w < 1.0 { 1.0 } else { 1.0 / p.w };
    let sx = (160.0 + p.x * inv_w * 160.0) as i16;
    let sy = (120.0 - p.y * inv_w * 120.0) as i16;
    sx > -20 && sx < 340 && sy > -160 && sy < 400
}

/// `func_8002C7BC` (without Navi and sounds), called with the actor Player keeps targeted.
pub fn update(ctx: &mut TargetCtx, actors: &[TargetActor], f: &TargetFrame) {
    // Actor_UpdateAll: only a locked target with unk_66C >= 5 counts.
    let mut locked = f.player_target;
    if locked.is_none() || f.player_timer < 5 {
        locked = None;
        ctx.unk_4B = 0;
    }
    let mut candidate = None;
    if !(f.player_target.is_some() && f.player_stick_dir == 2) {
        // func_80032AF0 → func_800328D4 over the target list.
        let mut best = f32::MAX;
        let mut best_prio: Option<(u8, usize)> = None;
        for (i, a) in actors.iter().enumerate() {
            if a.flags & ACTOR_FLAG_0 == 0 || Some(i) == f.player_target {
                continue;
            }
            let var = weighted_dist(a, f.player_target.is_some(), f.player_shape_yaw);
            let visible = f
                .col
                .check_line(bgcheck::IGNORE_CAMERA, bgcheck::IGNORE_NONE, f.player_focus, a.focus, 1.0, bgcheck::CHECK_WALL | bgcheck::CHECK_FLOOR | bgcheck::CHECK_CEILING | bgcheck::CHECK_ONE_FACE | bgcheck::CHECK_DYNA)
                .is_none();
            if var < best && in_range(f.ranges, a, var) && on_screen(f.view_proj, a) && visible {
                if a.target_priority != 0 {
                    if best_prio.is_none_or(|(p, _)| a.target_priority < p) {
                        best_prio = Some((a.target_priority, i));
                    }
                } else {
                    candidate = Some(i);
                    best = var;
                }
            }
        }
        if candidate.is_none() {
            candidate = best_prio.map(|(_, i)| i);
        }
    }
    ctx.unk_94 = candidate;
    let pointed = locked.or(candidate);
    if pointed != ctx.arrow_pointed {
        ctx.arrow_pointed = pointed;
    }
    // Reticle: drop the lock if the target left the screen before locking.
    let mut arg = locked;
    if let Some(i) = arg
        && ctx.unk_4B == 0
        && !on_screen(f.view_proj, &actors[i])
    {
        arg = None;
    }
    if let Some(i) = arg {
        if ctx.targeted != Some(i) {
            ctx.targeted = Some(i);
        }
        if ctx.unk_4B == 0 {
            let t5 = (500.0 - ctx.unk_44) * 3.0;
            let t6 = t5.clamp(30.0, 100.0);
            if crate::math::step_to_f(&mut ctx.unk_44, 80.0, t6) {
                ctx.unk_4B += 1;
            }
        } else {
            ctx.unk_4B = (ctx.unk_4B.wrapping_add(3)) | 0x80;
            ctx.unk_44 = 120.0;
        }
    } else {
        ctx.targeted = None;
        crate::math::step_to_f(&mut ctx.unk_44, 500.0, 80.0);
    }
}

/// `Math_Vec3f_Yaw(a, b)`.
pub fn yaw_to(a: Vec3, b: Vec3) -> i16 {
    atan2_s(b.z - a.z, b.x - a.x)
}

/// `Math_Vec3f_Pitch(a, b)`: `Math_Atan2S(xz distance, a.y - b.y)`.
pub fn pitch_to(a: Vec3, b: Vec3) -> i16 {
    let d = ((b.x - a.x).powi(2) + (b.z - a.z).powi(2)).sqrt();
    atan2_s(d, a.y - b.y)
}
