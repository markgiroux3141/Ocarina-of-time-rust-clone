//! `En_Attack_Niw` (`ovl_En_Attack_Niw/z_en_attack_niw.c`): a cucco of the flock a struck cucco
//! calls (`EnNiw_SpawnAttackCucco`: up to 7 at a time, its `parent` the cucco that called them).
//!
//! Spawned somewhere about the camera, it flies at a point by the camera's eye (`func_809B5670`:
//! towards `view.at` plus its own offset `unk_298`, less the eye's distance, at speed 10, pitched
//! as it goes); there, on the ground or at a wall it drops and dives at Link (`func_809B59B0`:
//! hopping, turning to him, a little at random while its timer reads 50); once its time is up or a
//! wall stops it, it flies up and away (`func_809B5C18`). Off the screen (or behind the camera) in
//! either of the last two it's gone; fallen out of the world, too. Within 20 of Link it knocks him
//! down (`Actor_SetPlayerKnockbackLarge`, 16 damage), once each 70 frames for the flock (its
//! parent's `timer9`) and only while he isn't invincible. It cries every 30 frames and flaps every
//! 7. Gone, it's counted off its parent (`EnAttackNiw_Destroy`: `unk_296`).
//!
//! Drawn as `EnAttackNiw_Draw` does: `En_Niw`'s skeleton held at `gCuccoAnim`'s first frame (its
//! skeleton is never updated), the head, tail and wings turned (`func_809B5F98`); from En_Niw's
//! baked mesh (docs/adr/0012-actor-bakes.md). The circle shadow is the shape's (`ActorShape_Init`'s,
//! drawn by `Actor_Draw`).
//!

//! Params: `params` whole, as an `s16`: made 0 if negative, then read by nothing (`En_Niw`
//! spawns it with 0).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::BGCHECK_Y_MIN;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{
    ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_GROUND, BGCHECKFLAG_WALL, BGCHECKFLAG_WATER, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2,
    UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4,
};
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, actor_set_player_knockback_large, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;
use oot_game::target::{actor_screen_pos, pitch_to, yaw_to};

use crate::en_niw::{self, EnNiw};

/// `ACTOR_EN_ATTACK_NIW` (`actor_table.h`: 0x0144).
pub const ACTOR_EN_ATTACK_NIW: i16 = en_niw::ACTOR_EN_ATTACK_NIW;
/// `OBJECT_NIW` (`object_table.h`: 0x0013).
pub const OBJECT: &str = en_niw::OBJECT;

/// `En_Attack_Niw_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_ATTACK_NIW, name: "En_Attack_Niw", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `this->actor.params < 0`: the whole params as an `s16`.
pub const PARAMS: Field = Field::whole("params", Kind::Range(-0x8000, 0x7FFF), "made 0 if negative, then read by nothing (En_Niw spawns it with 0)");

/// En_Attack_Niw's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[PARAMS], flags: &[], todo: false, objects: &[], texts: &[] };

/// `ATTENTION_RANGE_1`.
const ATTENTION_RANGE_1: u8 = 1;
/// `SCREEN_WIDTH`, `SCREEN_HEIGHT`.
const SCREEN_WIDTH: i16 = 320;
const SCREEN_HEIGHT: i16 = 240;

// Sound effects (`sfx/environmentbank_table.h`: 0x2812; `enemybank_table.h`: 0x38FF).
const NA_SE_EV_CHICKEN_CRY_A: u16 = 0x2812;
const NA_SE_EN_DEKU_WAKEUP: u16 = 0x38FF;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Flying at the camera.
    Func809B5670,
    /// Down, diving at Link.
    Func809B59B0,
    /// Flying off.
    Func809B5C18,
}

pub struct EnAttackNiw {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub unk_254: i16,
    pub unk_256: i16,
    pub unk_258: i16,
    pub unk_25a: i16,
    pub unk_25c: i16,
    pub unk_25e: i16,
    pub unk_260: i16,
    pub unk_262: i16,
    pub unk_264: f32,
    pub unk_268: f32,
    pub unk_26c: f32,
    pub unk_278: f32,
    pub unk_27c: f32,
    pub unk_280: f32,
    pub unk_284: f32,
    pub unk_288: f32,
    pub unk_28c: i16,
    pub unk_28e: i16,
    pub unk_292: i16,
    /// `unk_298`: its offset from the camera's `at`.
    pub unk_298: Vec3,
    pub unk_2a4: Vec3,
    pub unk_2b0: Vec3,
    pub unk_2bc: f32,
    pub unk_2c0: f32,
    /// `unk_2D0`, `unk_2D4`: the pitch and yaw it turns to.
    pub unk_2d0: f32,
    pub unk_2d4: f32,
    /// `unk_2DC`: its turn's step.
    pub unk_2dc: f32,
    /// `unk_2E0`: the speed it approaches.
    pub unk_2e0: f32,
    /// `unk_2E4`: the focus's height (never set: 0).
    pub unk_2e4: f32,
}

/// `(s16)f`: a float to `s16` as the MIPS does it (truncated to `s32`, the low 16 bits).
fn f_to_s16(f: f32) -> i16 {
    f as i32 as i16
}

impl EnAttackNiw {
    /// `EnAttackNiw_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, "gCuccoSkel")?, assets.animation(OBJECT, "gCuccoAnim")?)) })();
        let (skeleton, anim) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Attack_Niw: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnAttackNiw {
            // SkelAnime_InitFlex(.., gCuccoSkel, gCuccoAnim, ..).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anim)),
            skeleton,
            action: Action::Func809B5670,
            unk_254: 0,
            unk_256: 0,
            unk_258: 0,
            unk_25a: 0,
            unk_25c: 0,
            unk_25e: 0,
            unk_260: 0,
            unk_262: 0,
            unk_264: 0.0,
            unk_268: 0.0,
            unk_26c: 0.0,
            unk_278: 0.0,
            unk_27c: 0.0,
            unk_280: 0.0,
            unk_284: 0.0,
            unk_288: 0.0,
            unk_28c: 0,
            unk_28e: 0,
            unk_292: 0,
            unk_298: Vec3::ZERO,
            unk_2a4: Vec3::ZERO,
            unk_2b0: Vec3::ZERO,
            unk_2bc: 0.0,
            unk_2c0: 0.0,
            unk_2d0: 0.0,
            unk_2d4: 0.0,
            unk_2dc: 0.0,
            unk_2e0: 0.0,
            unk_2e4: 0.0,
            actor,
        };
        let a = &mut this.actor;
        // sInitChain.
        a.target_mode = ATTENTION_RANGE_1;
        a.gravity = -2.0;
        a.target_arrow_offset = 0.0;
        a.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 25.0);
        if PARAMS.get(a.params) < 0 {
            a.params = 0;
        }
        a.scale = Vec3::splat(0.01);
        a.gravity = 0.0;
        this.unk_298.x = play.rand.centered_float(100.0);
        this.unk_298.y = play.rand.centered_float(10.0);
        this.unk_298.z = play.rand.centered_float(100.0);
        let a = &mut this.actor;
        a.scale = Vec3::splat(0.01);
        a.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        let yaw = f_to_s16((play.rand.zero_one() - 0.5) * 60000.0);
        a.world_rot.y = yaw;
        a.shape_rot.y = yaw;
        this.action = Action::Func809B5670;
        Box::new(this)
    }

    /// `func_809B5268`: the head, wings and tail's flapping, by `arg2` (0 still, 1 pecking, 2
    /// flapping, 3 to 5 others).
    fn func_809b5268(&mut self, play: &mut PlayState, arg2: i16) {
        if self.unk_254 == 0 {
            self.unk_264 = if arg2 == 0 { 0.0 } else { -10000.0 };
            self.unk_28e = self.unk_28e.wrapping_add(1);
            self.unk_254 = 3;
            if self.unk_28e & 1 == 0 {
                self.unk_264 = 0.0;
                if arg2 == 0 {
                    self.unk_254 = play.rand.zero_float(30.0) as i16;
                }
            }
        }

        if self.unk_258 == 0 {
            self.unk_292 = self.unk_292.wrapping_add(1);
            self.unk_292 &= 1;
            match arg2 {
                0 => {
                    self.unk_26c = 0.0;
                    self.unk_268 = 0.0;
                }
                1 => {
                    self.unk_258 = 3;
                    self.unk_26c = 7000.0;
                    self.unk_268 = 7000.0;
                    if self.unk_292 == 0 {
                        self.unk_26c = 0.0;
                        self.unk_268 = 0.0;
                    }
                }
                2 => {
                    self.unk_258 = 2;
                    self.unk_26c = -10000.0;
                    self.unk_268 = -10000.0;
                    self.unk_278 = 25000.0;
                    self.unk_280 = 25000.0;
                    self.unk_27c = 6000.0;
                    self.unk_284 = 6000.0;
                    if self.unk_292 == 0 {
                        self.unk_278 = 8000.0;
                        self.unk_280 = 8000.0;
                    }
                }
                3 => {
                    self.unk_258 = 2;
                    self.unk_278 = 10000.0;
                    self.unk_280 = 10000.0;
                    if self.unk_292 == 0 {
                        self.unk_278 = 3000.0;
                        self.unk_280 = 3000.0;
                    }
                }
                4 => {
                    self.unk_256 = 5;
                    self.unk_254 = 5;
                }
                5 => {
                    self.unk_258 = 5;
                    self.unk_278 = 14000.0;
                    self.unk_280 = 14000.0;
                    if self.unk_292 == 0 {
                        self.unk_278 = 10000.0;
                        self.unk_280 = 10000.0;
                    }
                }
                _ => {}
            }
        }

        if self.unk_288 != self.unk_2c0 {
            approach_f(&mut self.unk_2c0, self.unk_288, 0.5, 4000.0);
        }
        if self.unk_264 != self.unk_2bc {
            approach_f(&mut self.unk_2bc, self.unk_264, 0.5, 4000.0);
        }
        if self.unk_26c != self.unk_2a4.x {
            approach_f(&mut self.unk_2a4.x, self.unk_26c, 0.8, 7000.0);
        }
        if self.unk_280 != self.unk_2a4.y {
            approach_f(&mut self.unk_2a4.y, self.unk_280, 0.8, 7000.0);
        }
        if self.unk_284 != self.unk_2a4.z {
            approach_f(&mut self.unk_2a4.z, self.unk_284, 0.8, 7000.0);
        }
        if self.unk_268 != self.unk_2b0.x {
            approach_f(&mut self.unk_2b0.x, self.unk_268, 0.8, 7000.0);
        }
        if self.unk_278 != self.unk_2b0.y {
            approach_f(&mut self.unk_2b0.y, self.unk_278, 0.8, 7000.0);
        }
        if self.unk_27c != self.unk_2b0.z {
            approach_f(&mut self.unk_2b0.z, self.unk_27c, 0.8, 7000.0);
        }
    }

    /// `func_809B55EC`: on the screen (and not behind the camera).
    fn func_809b55ec(&mut self, play: &PlayState) -> bool {
        self.actor.set_focus(self.unk_2e4);
        let (sp1e, sp1c) = actor_screen_pos(play.view_proj, &self.actor);
        !(self.actor.projected_pos.z < -20.0 || sp1e < 0 || sp1e > SCREEN_WIDTH || sp1c < 0 || sp1c > SCREEN_HEIGHT)
    }

    /// `func_809B5670`: flying at the point by the camera.
    fn func_809b5670(&mut self, play: &mut PlayState) {
        self.actor.speed_xz = 10.0;

        let (eye, at) = (play.view.eye, play.view.at);
        let tmpf1 = (self.unk_298.x + at.x) - eye.x;
        let tmpf2 = (self.unk_298.y + at.y) - eye.y;
        let tmpf3 = (self.unk_298.z + at.z) - eye.z;
        let sp34 = Vec3::new(at.x + tmpf1, at.y + tmpf2, at.z + tmpf3);

        self.unk_2d4 = yaw_to(self.actor.world_pos, sp34) as f32;
        self.unk_2d0 = pitch_to(self.actor.world_pos, sp34) as f32 * -1.0;

        smooth_step_to_s(&mut self.actor.world_rot.y, f_to_s16(self.unk_2d4), 5, f_to_s16(self.unk_2dc), 0);
        smooth_step_to_s(&mut self.actor.world_rot.x, f_to_s16(self.unk_2d0), 5, f_to_s16(self.unk_2dc), 0);
        approach_f(&mut self.unk_2dc, 5000.0, 1.0, 100.0);

        // Actor_SetFocus, Actor_GetScreenPos (sp4E, sp4C: unread).
        self.actor.set_focus(self.unk_2e4);

        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.unk_2d4 = self.actor.yaw_towards_player as f32;
            self.unk_2d0 = self.actor.world_rot.x as f32 - 3000.0;
            self.unk_2dc = 0.0;
            self.unk_284 = 0.0;
            self.unk_27c = 0.0;
            self.unk_25a = 0;
            self.unk_258 = 0;
            self.unk_256 = 0;
            self.unk_254 = 0;
            self.unk_25c = 0x64;
            self.actor.gravity = -0.2;
            self.unk_2e0 = 5.0;
            self.unk_288 = 0.0;
            self.action = Action::Func809B59B0;
        } else if (self.actor.projected_pos.z > 0.0 && (sp34.x - self.actor.world_pos.x).abs() < 50.0 && (sp34.y - self.actor.world_pos.y).abs() < 50.0 && (sp34.z - self.actor.world_pos.z).abs() < 50.0)
            || self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0
        {
            self.unk_2d4 = self.actor.yaw_towards_player as f32;
            self.unk_2d0 = self.actor.world_rot.x as f32 - 2000.0;
            self.unk_2dc = 0.0;
            self.unk_27c = 0.0;
            self.unk_284 = 0.0;
            self.unk_25a = 0;
            self.unk_258 = 0;
            self.unk_256 = 0;
            self.unk_254 = 0;
            self.actor.gravity = -0.2;
            self.unk_2e0 = 5.0;
            self.unk_288 = 0.0;
            self.action = Action::Func809B59B0;
        } else {
            self.unk_254 = 10;
            self.unk_264 = -10000.0;
            self.unk_288 = -3000.0;
            self.func_809b5268(play, 2);
        }
    }

    /// `func_809B59B0`: down, diving at Link in hops.
    fn func_809b59b0(&mut self, play: &mut PlayState) {
        if !self.func_809b55ec(play) {
            self.actor.kill();
            return;
        }

        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            if self.unk_25a == 0 {
                self.unk_25a = 3;
                self.actor.velocity.y = 3.5;
            }
            if self.actor.gravity != -2.0 {
                self.unk_278 = 14000.0;
                self.unk_280 = 14000.0;
                self.unk_27c = 0.0;
                self.unk_284 = 0.0;
                self.unk_268 = 0.0;
                self.unk_26c = 0.0;
                self.unk_2d0 = 0.0;
                self.unk_2d4 = self.actor.yaw_towards_player as f32;
                self.unk_262 = 0x32;
                self.unk_25c = 0x64;
                self.actor.gravity = -2.0;
            }
        }
        if self.unk_25c == 0x32 {
            self.unk_2d4 = play.rand.centered_float(200.0) + self.actor.yaw_towards_player as f32;
        }
        smooth_step_to_s(&mut self.actor.world_rot.y, f_to_s16(self.unk_2d4), 2, f_to_s16(self.unk_2dc), 0);
        smooth_step_to_s(&mut self.actor.world_rot.x, f_to_s16(self.unk_2d0), 2, f_to_s16(self.unk_2dc), 0);
        approach_f(&mut self.unk_2dc, 10000.0, 1.0, 1000.0);
        approach_f(&mut self.actor.speed_xz, self.unk_2e0, 0.9, 1.0);
        if self.actor.gravity == -2.0 && self.unk_262 == 0 && (self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 || self.unk_25c == 0) {
            self.unk_2e0 = 0.0;
            self.actor.gravity = 0.0;
            self.unk_2dc = 0.0;
            self.unk_2d0 = self.actor.world_rot.x as f32 - 5000.0;
            self.action = Action::Func809B5C18;
        } else if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.func_809b5268(play, 5);
        } else {
            self.func_809b5268(play, 2);
        }
    }

    /// `func_809B5C18`: flying up and away.
    fn func_809b5c18(&mut self, play: &mut PlayState) {
        if !self.func_809b55ec(play) {
            self.actor.kill();
            return;
        }
        smooth_step_to_s(&mut self.actor.world_rot.x, f_to_s16(self.unk_2d0), 5, f_to_s16(self.unk_2dc), 0);
        approach_f(&mut self.unk_2dc, 5000.0, 1.0, 100.0);
        approach_f(&mut self.actor.velocity.y, 5.0, 0.3, 1.0);
        self.func_809b5268(play, 2);
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Func809B5670 => self.func_809b5670(play),
            Action::Func809B59B0 => self.func_809b59b0(play),
            Action::Func809B5C18 => self.func_809b5c18(play),
        }
    }

    /// `func_809B5F98`'s turns: `unk_2BC` (head, limb 13), `unk_2C0` (tail, 15), `unk_2B0`
    /// (11) and `unk_2A4` (7), the wings.
    fn turns(&self) -> [i16; 8] {
        [self.unk_2bc, self.unk_2c0, self.unk_2b0.x, self.unk_2b0.y, self.unk_2b0.z, self.unk_2a4.x, self.unk_2a4.y, self.unk_2a4.z].map(f_to_s16)
    }

    /// The limb matrices `EnAttackNiw_Draw` draws with: `func_809B5F98`'s turns. `t` is
    /// [`Self::turns`].
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], t: [i16; 8]) -> Vec<Mat4> {
        let [bc, c0, b0x, b0y, b0z, a4x, a4y, a4z] = t;
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if limb == 13 {
                rot[1] = rot[1].wrapping_add(bc);
            }
            if limb == 15 {
                rot[2] = rot[2].wrapping_add(c0);
            }
            if limb == 11 {
                rot[0] = rot[0].wrapping_add(b0z);
                rot[1] = rot[1].wrapping_add(b0y);
                rot[2] = rot[2].wrapping_add(b0x);
            }
            if limb == 7 {
                rot[0] = rot[0].wrapping_add(a4z);
                rot[1] = rot[1].wrapping_add(a4y);
                rot[2] = rot[2].wrapping_add(a4x);
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnAttackNiw {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnAttackNiw_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.unk_28c = self.unk_28c.wrapping_add(1);
        for t in [&mut self.unk_254, &mut self.unk_258, &mut self.unk_25a, &mut self.unk_25e, &mut self.unk_260, &mut self.unk_25c, &mut self.unk_262] {
            if *t != 0 {
                *t -= 1;
            }
        }

        self.actor.shape_rot = self.actor.world_rot;
        self.actor.shape_shadow_scale = 15.0;
        self.run_action(play);
        if self.actor.killed {
            return;
        }
        // Before the move, as the C has it.
        self.actor.update_bg_check_info(&play.col, 20.0, 20.0, 60.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);

        if self.action == Action::Func809B5670 {
            self.actor.move_xyz();
        } else {
            self.actor.move_forward();
        }

        if self.actor.floor_height <= BGCHECK_Y_MIN {
            self.actor.kill();
            return;
        }

        if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 && self.action != Action::Func809B5C18 {
            // EffectSsGSplash_Spawn(play, &sp30, NULL, NULL, 0, 0x190): at the surface.
            let mut sp30 = self.actor.world_pos;
            sp30.y += self.actor.y_dist_to_water;
            play.with_ss(|s| s.g_splash_spawn(sp30, None, None, 0, 0x190));
            self.unk_2dc = 0.0;
            self.actor.gravity = 0.0;
            self.unk_2e0 = 0.0;
            self.unk_2d0 = self.actor.world_rot.x as f32 - 5000.0;
            self.action = Action::Func809B5C18;
            return;
        }

        let tmpf1 = 20.0;
        if self.actor.xyz_dist_to_player_sq < tmpf1 * tmpf1 {
            // @bug (game): the C reads `parent->update` before checking `parent` for NULL (always
            // set: spawned as the cucco's child), and a parent deleted since is read through a
            // dangling pointer; here a parent gone counts as none (no knockdown).
            // OOT_VERSION >= NTSC_1_1 (this ROM): Player's invincibilityTimer.
            let invincibility_timer = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.invincibility_timer()).unwrap_or(0);
            let cucco_ready = self.actor.parent.and_then(|h| play.actors.downcast::<EnNiw>(h)).is_some_and(|c| !c.actor.killed && c.timer9 == 0);
            if cucco_ready && invincibility_timer == 0 {
                actor_set_player_knockback_large(play, 2.0, self.actor.world_rot.y, 0.0, 0x10);
                if let Some(c) = self.actor.parent.and_then(|h| play.actors.downcast_mut::<EnNiw>(h)) {
                    c.timer9 = 0x46;
                }
            }
        }
        if self.unk_25e == 0 {
            self.unk_25e = 30;
            audio_play_actor_sfx2(play, NA_SE_EV_CHICKEN_CRY_A);
        }
        if self.unk_260 == 0 {
            self.unk_260 = 7;
            audio_play_actor_sfx2(play, NA_SE_EN_DEKU_WAKEUP);
        }
    }
    /// `EnAttackNiw_Destroy`: counted off its parent.
    fn destroy(&mut self, play: &mut PlayState) {
        if let Some(cucco) = self.actor.parent.and_then(|h| play.actors.downcast_mut::<EnNiw>(h))
            && !cucco.actor.killed
            && cucco.unk_296 > 0
        {
            cucco.unk_296 -= 1;
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = self.turns().to_vec();
        rs
    }
    /// `EnAttackNiw_Draw`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let Ok(t) = <[i16; 8]>::try_from(rs.angles.as_slice()) else { return };
        let bones = Self::pose(&self.skeleton, &joints.rot, t);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(en_niw::BAKE)), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
