//! `En_Part` (`ovl_En_Part/z_en_part.c`): a piece of a dying enemy (a limb's list), thrown off
//! and fading in a puff; and `z_actor.c`'s `BodyBreak`, which takes an enemy's limbs' matrices
//! and lists as it's drawn and spawns them as parts (GAME-06 milestone 4).
//!
//! Params: the kind. 0 drifts and falls; 1, 4, 9, 10, 14 fly up and puff out (14 on fire); 2 flies
//! up; 3 and 11 jump out (11 on fire); 5 to 8 an Iron Knuckle's armour sliding off; 12 and 13 fall
//! and stay (13 till its maker goes); a part spawned by `Actor_SpawnPartsOnDeath`-like callers
//! with action 2 is a shock wave (`func_80ACE5C8`) or a Stalfos' head (`func_80ACE7E8`).
//!
//! The whole overlay is ported. `EffectSsDtBubble` (kind 4's bubbles) isn't: its `Rand` calls
//! are made. The Iron Knuckle's and the Tektite's segment colours and textures (kinds 5 to 7, 9
//! and 10 with `object_ik`'s and `object_tite`'s lists) aren't set: their actors aren't ported.
//! `func_8002EBCC` (the lists' shine) isn't, as for every actor. A part's list is a limb's, which
//! the pack keeps only in its skeleton's mesh: every ported enemy's breakable limbs are baked on
//! their own (`bakes`).

use eng_gfx::{DrawCmd, MeshKey};
use eng_math::smooth_step_to_f;
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage, audio_play_actor_sfx2};
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::sys_matrix::MtxF;

/// `ACTOR_EN_PART` (`actor_table.h`: 0x0007).
pub const ACTOR_EN_PART: i16 = 0x0007;

/// `En_Part_Profile`: `ACTORCAT_ITEMACTION`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_PART, name: "En_Part", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep" };

/// `NA_SE_EN_MONBLIN_GNDWAVE`, `NA_SE_EN_STAL_DAMAGE` (`enemybank_table.h`).
const NA_SE_EN_MONBLIN_GNDWAVE: u16 = 0x38E0;
const NA_SE_EN_STAL_DAMAGE: u16 = 0x383A;

/// A display list: its object file and symbol.
pub type DList = (&'static str, &'static str);

/// `object_ik_DL_015380`: the Iron Knuckle's piece that slides the other way.
const OBJECT_IK_DL_015380: DList = ("object_ik", "object_ik_DL_015380");

/// A part's bake: its list after `Gfx_SetupDL_25Opa`.
pub fn part_bake_name(file: &str, symbol: &str) -> String {
    format!("En_Part/{file}/{symbol}")
}

/// The parts the ported enemies break into: the Stalchild's limbs.
pub fn bakes() -> Vec<MeshBake> {
    crate::en_skb::LIMB_DLISTS
        .iter()
        .flatten()
        .map(|sym| MeshBake {
            name: part_bake_name(crate::en_skb::OBJECT, sym),
            object: crate::en_skb::OBJECT.into(),
            segments: Vec::new(),
            prelude: Vec::new(),
            body: BakeBody::DLists(vec![(crate::en_skb::OBJECT.into(), (*sym).into())]),
        })
        .collect()
}

pub struct EnPart {
    pub actor: Actor,
    pub action: u8,
    pub timer: i16,
    /// `displayList`.
    pub display_list: Option<DList>,
    pub rot_z: f32,
    pub rot_z_speed: f32,
}

impl EnPart {
    /// `EnPart_Init`: nothing (its spawner sets its list, rotation and scale).
    pub fn init(actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        Box::new(EnPart { actor, action: 0, timer: 0, display_list: None, rot_z: 0.0, rot_z_speed: 0.0 })
    }

    /// `func_80ACDDE8`: the kind's throw.
    fn func_80acdde8(&mut self, play: &mut PlayState) {
        let mut sign = 1.0;
        self.action = 1;
        self.actor.world_rot.y = (play.rand.zero_one() * 20000.0) as i32 as i16;
        let a = &mut self.actor;
        match a.params {
            0 => {
                a.velocity.y = 0.0;
                a.gravity = -0.3 - play.rand.zero_one() * 0.5;
                self.rot_z_speed = 0.3;
                self.timer = 25;
                self.actor.speed_xz = (play.rand.zero_one() - 0.5) * 2.0;
            }
            12 | 13 => {
                if self.actor.params == 13 {
                    self.timer = 400;
                }
                let a = &mut self.actor;
                a.speed_xz = play.rand.centered_float(6.0);
                a.home_pos = a.world_pos;
                self.timer += 60;
                let a = &mut self.actor;
                a.velocity.y = play.rand.zero_one() * 5.0 + 4.0;
                a.gravity = -0.6 - play.rand.zero_one() * 0.5;
                self.rot_z_speed = 0.15;
            }
            14 | 1 | 4 | 9 | 10 | 2 => {
                if a.params == 14 {
                    let pos = a.world_pos;
                    if let Some(me) = play.cur_actor {
                        let r = self.actor.shape_rot;
                        let actor = oot_game::effect::SsActor { handle: me, world_pos: pos, shape_rot: [r.x, r.y, r.z] };
                        play.with_ss(|s| s.en_fire_spawn_vec3f(Some(actor), pos, 40, 0x8001u16 as i16, 0, -1));
                    }
                }
                if self.actor.params != 2 {
                    self.timer += (play.rand.zero_one() * 17.0) as i16 + 5;
                }
                let a = &mut self.actor;
                a.velocity.y = play.rand.zero_one() * 5.0 + 4.0;
                a.gravity = -0.6 - play.rand.zero_one() * 0.5;
                self.rot_z_speed = 0.15;
            }
            11 | 3 => {
                if a.params == 11 {
                    let pos = a.world_pos;
                    if let Some(me) = play.cur_actor {
                        let r = self.actor.shape_rot;
                        let actor = oot_game::effect::SsActor { handle: me, world_pos: pos, shape_rot: [r.x, r.y, r.z] };
                        play.with_ss(|s| s.en_fire_spawn_vec3f(Some(actor), pos, 40, 0x8001u16 as i16, 0, -1));
                    }
                }
                let a = &mut self.actor;
                a.speed_xz = (play.rand.zero_one() - 0.5) * 3.0;
                self.timer = (play.rand.zero_one() * 17.0) as i16 + 10;
                let a = &mut self.actor;
                a.velocity.y = play.rand.zero_one() * 3.0 + 8.0;
                a.gravity = -0.6 - play.rand.zero_one() * 0.3;
                self.rot_z_speed = 0.15;
            }
            5..=8 => {
                if let Some(p) = a.parent.and_then(|h| play.actors.actor(h)) {
                    self.actor.world_rot.y = p.shape_rot.y;
                }
                if self.display_list == Some(OBJECT_IK_DL_015380) {
                    sign = -1.0;
                }
                let a = &mut self.actor;
                a.velocity.y = 0.0;
                a.speed_xz = 6.0 * sign;
                a.gravity = -1.2;
                self.rot_z_speed = 0.15 * sign;
                // ActorShape_Init(0, ActorShadow_DrawCircle, 30): the circle shadow isn't drawn.
                a.shape_y_offset = 0.0;
                self.timer = 18;
            }
            _ => {}
        }
    }

    /// `func_80ACE13C`: 12 and 13 fall till they land (13 forgets a gone maker); the others, their
    /// time out, puff out (white with green or blue, or bubbles, or five blue puffs) and go.
    fn func_80ace13c(&mut self, play: &mut PlayState) {
        let zero = Vec3::ZERO;
        let p = self.actor.params;
        if p == 12 || p == 13 {
            let flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4;
            self.actor.update_bg_check_info(&play.col, 5.0, 15.0, 0.0, flags);
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 || self.actor.world_pos.y <= self.actor.floor_height {
                self.action = 4;
                self.actor.speed_xz = 0.0;
                self.actor.gravity = 0.0;
                self.actor.velocity.y = 0.0;
            }
            if p == 13 && self.actor.parent.is_some_and(|h| play.actors.actor(h).is_none_or(|a| a.killed)) {
                self.actor.parent = None;
            }
        } else if self.timer <= 0 {
            let scale = ((self.actor.scale.y * 100.0) as i16).wrapping_mul(40);
            let pos = self.actor.world_pos;
            match p {
                1 | 9 | 10 | 14 => play.with_ss(|s| s.dead_db_spawn(pos, zero, zero, scale, 7, [255, 255, 255, 255], [0, 255, 0], 1, 9, 1)),
                3 | 11 => play.with_ss(|s| s.dead_db_spawn(pos, zero, zero, scale, 7, [255, 255, 255, 255], [0, 0, 255], 1, 9, 1)),
                4 => {
                    for _ in 0..8 {
                        // EffectSsDtBubble_SpawnColorProfile isn't ported: its Rand calls.
                        let _x = play.rand.centered_float(60.0);
                        let _y = play.rand.centered_float(50.0);
                        let _z = play.rand.centered_float(60.0);
                        let _vy = play.rand.zero_one() + 1.0;
                        let _scale = play.rand.s16_offset(80, 100);
                    }
                    log::debug!("En_Part: EffectSsDtBubble isn't ported");
                }
                5..=8 => {
                    for _ in 0..5 {
                        let x = pos.x + play.rand.centered_float(25.0);
                        let y = pos.y + play.rand.centered_float(40.0);
                        let z = pos.z + play.rand.centered_float(25.0);
                        let at = Vec3::new(x, y, z);
                        play.with_ss(|s| s.dead_db_spawn(at, zero, zero, 40, 7, [255, 255, 255, 255], [0, 0, 255], 1, 9, 1));
                    }
                }
                _ => {}
            }
            self.actor.kill();
            return;
        }
        self.timer -= 1;
        self.rot_z += self.rot_z_speed;
    }

    /// `func_80ACE5C8`: a shock wave along the ground for its timer: Link within 40 is hurt half a
    /// heart (unless invincible) and knocked back from its maker; dust, a piece, the rumble.
    fn func_80ace5c8(&mut self, play: &mut PlayState) {
        self.timer -= 1;
        if self.timer == 0 {
            self.actor.kill();
            return;
        }
        let mut velocity = Vec3::new(0.0, 8.0, 0.0);
        let accel = Vec3::new(0.0, -1.5, 0.0);
        if self.actor.xyz_dist_to_player_sq.sqrt() <= 40.0 {
            let ph = play.player;
            let prev = ph.and_then(|h| play.actors.downcast::<crate::player::Player>(h)).map(|p| p.invincibility_timer).unwrap_or(0);
            if prev <= 0 {
                if let Some(p) = ph.and_then(|h| play.actors.downcast_mut::<crate::player::Player>(h)) {
                    let was = p.invincibility_timer;
                    p.invincibility_timer = 0;
                    if was > -40 {
                        crate::player::play_damage_player(play, -8);
                    }
                }
            }
            if let Some(parent) = self.actor.parent.and_then(|h| play.actors.actor(h)).cloned() {
                actor_set_player_knockback_large_no_damage(play, (650.0 - parent.xz_dist_to_player) * 0.04 + 4.0, parent.world_rot.y, 8.0);
            }
            if let Some(p) = ph.and_then(|h| play.actors.downcast_mut::<crate::player::Player>(h)) {
                p.invincibility_timer = prev;
            }
            self.timer = 1;
        }
        let pos = self.actor.world_pos;
        oot_game::actor_ctx::func_80033480(play, pos, 0.0, 1, 300, 150, 1);
        velocity.x = play.rand.centered_float(16.0);
        let scale = ((play.rand.zero_one() * 5.0 + 12.0) * 2.0) as i32 as i16;
        play.with_ss(|s| s.hahen_spawn(pos, velocity, accel, 20, scale, -1, 10, None));
        audio_play_actor_sfx2(play, NA_SE_EN_MONBLIN_GNDWAVE - oot_game::audio::sfx::SFX_FLAG);
    }

    /// `func_80ACE7E8`: a Stalfos' head waiting to go back: its maker gone, a green puff and it
    /// goes; its timer out, it eases home and tells its maker; it goes when its maker is alive.
    fn func_80ace7e8(&mut self, play: &mut PlayState) {
        let parent = self.actor.parent.and_then(|h| play.actors.actor(h)).filter(|a| !a.killed).cloned();
        let Some(parent) = parent else {
            let scale = ((self.actor.scale.y * 100.0) as i16).wrapping_mul(40);
            let pos = self.actor.world_pos;
            play.with_ss(|s| s.dead_db_spawn(pos, Vec3::ZERO, Vec3::ZERO, scale, 7, [255, 255, 255, 255], [0, 255, 0], 1, 9, 1));
            self.actor.kill();
            return;
        };
        if self.timer == 0 {
            let home = self.actor.home_pos;
            let a = &mut self.actor;
            let mut sum = smooth_step_to_f(&mut a.world_pos.x, home.x, 1.0, 5.0, 0.0);
            sum += smooth_step_to_f(&mut a.world_pos.y, home.y, 1.0, 5.0, 0.0);
            sum += smooth_step_to_f(&mut a.world_pos.z, home.z, 1.0, 5.0, 0.0);
            sum += smooth_step_to_f(&mut self.rot_z, 0.0, 1.0, 0.25, 0.0);
            if sum == 0.0 {
                if let Some(p) = self.actor.parent.and_then(|h| play.actors.actor_mut(h)) {
                    p.home_rot.x = p.home_rot.x.wrapping_sub(1);
                }
                self.timer -= 1;
                audio_play_actor_sfx2(play, NA_SE_EN_STAL_DAMAGE);
            }
        } else if self.timer > 0 {
            self.timer -= 1;
        }
        if parent.col_chk_info.health != 0 {
            self.actor.kill();
        }
    }
}

impl ActorImpl for EnPart {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnPart_Update`: moved; the armour (5 to 8) and the negative kinds bounce off the floor
    /// (the armour slowing), then the action (`sActionFuncs`).
    fn update(&mut self, play: &mut PlayState) {
        self.actor.move_forward();
        let p = self.actor.params;
        if (p > 4 && p < 9) || p < 0 {
            self.actor.update_bg_check_info(&play.col, 5.0, 15.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
            if p >= 0 {
                smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
                if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                    self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND;
                    self.actor.velocity.y = 6.0;
                }
            }
        }
        match self.action {
            0 => self.func_80acdde8(play),
            1 => self.func_80ace13c(play),
            // func_80ACE5B8.
            2 => self.action = 3,
            3 => self.func_80ace5c8(play),
            4 => self.func_80ace7e8(play),
            _ => {}
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.values = vec![self.rot_z];
        rs
    }

    /// `EnPart_Draw`: its list at its matrix, turned by `rotZ` (kinds above 0), after
    /// `Gfx_SetupDL_25Opa`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some((file, symbol)) = self.display_list else { return };
        let mut m = actor_draw_matrix(rs);
        if self.actor.params > 0 {
            m *= Mat4::from_rotation_z(rs.values.first().copied().unwrap_or(0.0));
        }
        out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(&part_bake_name(file, symbol))), m));
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// `BODYBREAK_OBJECT_SLOT_DEFAULT`, `BODYBREAK_STATUS_READY`, `BODYBREAK_STATUS_FINISHED`.
pub const BODYBREAK_OBJECT_SLOT_DEFAULT: i16 = -1;
pub const BODYBREAK_STATUS_READY: i16 = -1;
pub const BODYBREAK_STATUS_FINISHED: i16 = 0;

/// `BodyBreak` (`z_actor.c`): the limbs an enemy breaks into, taken as it's drawn.
#[derive(Debug, Clone, Default)]
pub struct BodyBreak {
    /// `matrices`: each piece's matrix in the world (the draw's, model times limb).
    pub matrices: Vec<Mat4>,
    pub dlists: Vec<Option<DList>>,
    pub object_slots: Vec<i16>,
    pub val: i16,
    pub prev_limb_index: i16,
    pub count: i16,
}

impl BodyBreak {
    /// `BodyBreak_Alloc`: room for `count` pieces (and the unused 0th), zeroed; `val` 1.
    pub fn alloc(&mut self, count: usize) {
        self.matrices = vec![Mat4::ZERO; count + 1];
        self.dlists = vec![None; count + 1];
        self.object_slots = vec![0; count + 1];
        self.val = 1;
    }

    /// `BodyBreak_SetInfo` for limb `limb_index` drawn with `dlist` at `matrix` (no freeze flash
    /// running): a limb from `min` to `max` with a list is taken; after `count` limbs, ready.
    #[allow(clippy::too_many_arguments)]
    pub fn set_info(&mut self, freeze_flash_timer: u8, limb_index: i32, min: i32, max: i32, count: u32, dlist: Option<DList>, object_slot: i16, matrix: Mat4) {
        if freeze_flash_timer == 0 && self.val > 0 {
            if limb_index >= min && limb_index <= max && dlist.is_some() {
                let v = self.val as usize;
                if v < self.dlists.len() {
                    self.dlists[v] = dlist;
                    self.matrices[v] = matrix;
                    self.object_slots[v] = object_slot;
                }
                self.val += 1;
            }
            if self.prev_limb_index as i32 != limb_index {
                self.count += 1;
            }
            if self.count as u32 >= count {
                self.count = self.val - 1;
                self.val = BODYBREAK_STATUS_READY;
            }
        }
        self.prev_limb_index = limb_index as i16;
    }

    /// `BodyBreak_SpawnParts`: once ready, each piece (from the last) as an `En_Part` of `ty`, the
    /// actor's child, where its matrix (without the actor's scale) puts it, turned as it is
    /// (PAL: `Matrix_MtxFToYXZRotS`), the actor's scale. True once done.
    pub fn spawn_parts(&mut self, actor: &mut Actor, play: &mut PlayState, ty: i16) -> bool {
        if self.val != BODYBREAK_STATUS_READY {
            return false;
        }
        while self.count > 0 {
            let i = self.count as usize;
            let m = self.matrices[i] * Mat4::from_scale(Vec3::new(1.0 / actor.scale.x, 1.0 / actor.scale.y, 1.0 / actor.scale.z));
            self.matrices[i] = m;
            // (The object slot is the En_Part's rotation's z: its list names its object here.)
            let pos = m.w_axis.truncate();
            match play.actor_spawn_as_child(actor, ACTOR_EN_PART, pos, [0, 0, 0], ty) {
                Ok(h) => {
                    let rot = MtxF::from_mat4(m).to_yxz_rot_s(false);
                    let (dl, scale) = (self.dlists[i], actor.scale);
                    if let Some(p) = play.actors.downcast_mut::<EnPart>(h) {
                        p.actor.shape_rot = Rot { x: rot[0], y: rot[1], z: rot[2] };
                        p.display_list = dl;
                        p.actor.scale = scale;
                    }
                }
                Err(e) => log::debug!("BodyBreak_SpawnParts: {e:?}"),
            }
            self.count -= 1;
        }
        self.val = BODYBREAK_STATUS_FINISHED;
        self.matrices.clear();
        self.dlists.clear();
        self.object_slots.clear();
        true
    }
}
