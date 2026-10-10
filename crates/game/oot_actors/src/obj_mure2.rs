//! `Obj_Mure2` (`ovl_Obj_Mure2/z_obj_mure2.c`): a group of bushes or rocks spawned when the
//! camera comes near and taken away when it leaves (GAME-06 milestone 4).
//!
//! Params: bits 0..1 the kind (0 a circle of nine bushes, 1 twelve scattered bushes, 2 a circle
//! of eight rocks: `En_Kusa`, `En_Kusa`, `En_Ishi`), bits 8..11 their drop table (13 and up: 0).
//!
//! Waiting, it spawns its group once its `projectedPos` (`Actor_DrawAll`'s) is within 1,600 across
//! the view's x and z (four times that in a cutscene); with its group out, it takes back what's
//! left when that passes 1,705. A member lifted (`Actor_HasParent`) or destroyed stays gone.
//!
//! The whole overlay is ported.

use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_OBJ_MURE2` (`actor_table.h`: 0x0151).
pub const ACTOR_OBJ_MURE2: i16 = 0x0151;

/// `Obj_Mure2_Profile`: `ACTORCAT_PROP`, no flags, `OBJECT_GAMEPLAY_KEEP`, no draw.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_MURE2, name: "Obj_Mure2", category: ACTORCAT_PROP, flags: 0, object: "gameplay_keep" };

/// `sDistSquared1`, `sDistSquared2`: the squared distances to spawn within and to take back past.
const DIST_SQUARED1: [f32; 3] = [1600.0 * 1600.0; 3];
const DIST_SQUARED2: [f32; 3] = [1705.0 * 1705.0; 3];
/// `D_80B9A818`: how many of each kind.
const COUNTS: [usize; 3] = [9, 12, 8];
/// `sActorSpawnIDs`.
const SPAWN_IDS: [i16; 3] = [crate::en_kusa::ACTOR_EN_KUSA, crate::en_kusa::ACTOR_EN_KUSA, crate::en_ishi::ACTOR_EN_ISHI];

/// `sScatteredShrubInfo`: (radius, angle) of the scattered bushes.
const SCATTERED: [(i16, u16); 12] =
    [(40, 0x0666), (40, 0x2CCC), (40, 0x5999), (40, 0x8666), (20, 0xC000), (80, 0x1333), (80, 0x4000), (80, 0x6CCC), (80, 0x9333), (80, 0xACCC), (80, 0xC666), (60, 0xE000)];

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `ObjMure2_Wait`.
    Wait,
    /// `func_80B9A668`: waiting for the camera to come near.
    WaitNear,
    /// `func_80B9A6F8`: the group out, waiting for the camera to leave.
    WaitFar,
}

pub struct ObjMure2 {
    pub actor: Actor,
    pub action: Action,
    /// `actorSpawnPtrList`.
    pub spawned: [Option<ActorHandle>; 12],
    /// `currentActorNum`: the members gone for good, by bit.
    pub current_actor_num: u16,
    /// `unk_184`: 1, or 4 in a cutscene.
    pub unk_184: f32,
}

impl ObjMure2 {
    fn kind(&self) -> usize {
        (self.actor.params & 3) as usize
    }

    /// `ObjMure2_Init`: its culling volume (100, 2100, 100; 1200 further in a cutscene), waiting.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.culling_volume_distance = 100.0;
        actor.culling_volume_scale = 2100.0;
        actor.culling_volume_downward = 100.0;
        if play.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE {
            actor.culling_volume_distance += 1200.0;
        }
        // ObjMure2_SetupWait.
        Box::new(ObjMure2 { actor, action: Action::Wait, spawned: [None; 12], current_actor_num: 0, unk_184: 0.0 })
    }

    /// `ObjMure2_SetPosShrubCircle`, `ObjMure2_SetPosShrubScattered`, `ObjMure2_SetPosRockCircle`.
    fn positions(&self) -> Vec<Vec3> {
        let p = self.actor.world_pos;
        let n = COUNTS.get(self.kind()).copied().unwrap_or(0);
        match self.kind() {
            0 => (0..n)
                .map(|i| if i == 0 { p } else { p + Vec3::new(80.0 * eng_math::sin_s(((i - 1) * 0x2000) as u16 as i16), 0.0, 80.0 * eng_math::cos_s(((i - 1) * 0x2000) as u16 as i16)) })
                .collect(),
            1 => (0..n)
                .map(|i| {
                    let (r, a) = SCATTERED[i];
                    Vec3::new(p.x + r as f32 * eng_math::cos_s(a as i16), p.y, p.z - r as f32 * eng_math::sin_s(a as i16))
                })
                .collect(),
            _ => (0..n).map(|i| p + Vec3::new(80.0 * eng_math::sin_s((i * 0x2000) as u16 as i16), 0.0, 80.0 * eng_math::cos_s((i * 0x2000) as u16 as i16))).collect(),
        }
    }

    /// `ObjMure2_SetActorSpawnParams`: `actorSpawnParams` (all 0) with the drop table in bits 8..11.
    fn spawn_params(&self) -> i16 {
        let mut drop_table = (self.actor.params >> 8) & 0xF;
        if drop_table >= 13 {
            drop_table = 0;
        }
        (drop_table << 8) & 0x0F00
    }

    /// `ObjMure2_SpawnActors`: each member not already out and not gone for good, in its place,
    /// turned by the group's x and z, in the group's room.
    fn spawn_actors(&mut self, play: &mut PlayState) {
        let kind = self.kind();
        let pos = self.positions();
        let params = self.spawn_params();
        for (i, p) in pos.iter().enumerate() {
            if self.spawned[i].is_some() {
                log::warn!("Warning : I already have a child (../z_obj_mure2.c 269)(arg_data {:#06x})", self.actor.params);
                continue;
            }
            if (self.current_actor_num >> i) & 1 == 0 {
                let rot = [self.actor.world_rot.x, 0, self.actor.world_rot.z];
                match play.actor_spawn(SPAWN_IDS[kind], *p, rot, params) {
                    Ok(h) => {
                        if let Some(a) = play.actors.actor_mut(h) {
                            a.room = self.actor.room;
                        }
                        self.spawned[i] = Some(h);
                    }
                    Err(e) => log::debug!("Obj_Mure2: member {i}: {e:?}"),
                }
            }
        }
    }

    /// `ObjMure2_CleanupAndDie`: the members still out are taken back (a lifted one stays gone).
    fn cleanup_and_die(&mut self, play: &mut PlayState) {
        for i in 0..COUNTS[self.kind()] {
            if (self.current_actor_num >> i) & 1 == 0 {
                if let Some(h) = self.spawned[i].take() {
                    let lifted = play.actors.actor(h).is_some_and(oot_game::get_item::actor_has_parent);
                    if lifted {
                        self.current_actor_num |= 1 << i;
                    } else if let Some(a) = play.actors.actor_mut(h) {
                        a.kill();
                    }
                }
            } else {
                self.spawned[i] = None;
            }
        }
    }

    /// `func_80B9A534`: a member whose update is gone (destroyed) is gone for good.
    fn func_80b9a534(&mut self, play: &PlayState) {
        for i in 0..COUNTS[self.kind()] {
            if let Some(h) = self.spawned[i]
                && (self.current_actor_num >> i) & 1 == 0
                && play.actors.actor(h).is_none_or(|a| a.killed)
            {
                self.current_actor_num |= 1 << i;
                self.spawned[i] = None;
            }
        }
    }

    /// `Math3D_Dist1DSq(projectedPos.x, projectedPos.z)`.
    fn dist_sq(&self) -> f32 {
        let p = self.actor.projected_pos;
        p.x * p.x + p.z * p.z
    }
}

impl ActorImpl for ObjMure2 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjMure2_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.unk_184 = if play.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE { 1.0 } else { 4.0 };
        let kind = self.kind();
        match self.action {
            // ObjMure2_Wait: func_80B9A658.
            Action::Wait => self.action = Action::WaitNear,
            Action::WaitNear => {
                if self.dist_sq() < DIST_SQUARED1[kind] * self.unk_184 {
                    self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
                    self.spawn_actors(play);
                    self.action = Action::WaitFar;
                }
            }
            Action::WaitFar => {
                self.func_80b9a534(play);
                if DIST_SQUARED2[kind] * self.unk_184 <= self.dist_sq() {
                    self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
                    self.cleanup_and_die(play);
                    self.action = Action::WaitNear;
                }
            }
        }
    }

    /// No draw.
    fn draw(&self, _rs: &RenderState, _play: &PlayState, _view: &ViewInfo, _out: &mut DrawOut) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
