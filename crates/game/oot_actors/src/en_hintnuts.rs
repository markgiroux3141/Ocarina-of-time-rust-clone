//! `En_Hintnuts` (`ovl_En_Hintnuts/z_en_hintnuts.c`): the Deku Tree's hint Deku Scrubs, which
//! spit nuts like a Mad Scrub (`En_Dekunuts`) but are only beaten by their own nut bounced back;
//! knocked out by it, one runs, and caught it gives a hint. Three of them (room 9, before the boss)
//! are a puzzle: knocked out in the order of their params (1, 2, 3), the third runs to talk.
//!
//! Params: the low byte 10 is the flower (spawned by the scrub as its child); else the low byte
//! is its place in the puzzle (0: on its own, talks when knocked out), the high byte its text
//! (`Actor_SetTextWithPrefix`: 0x1000 in the Deku Tree). Room 9's are 0x0001, 0x0002 and
//! 0x9C03 (text 0x109C; the first two's 0x1000 is never shown).
//!
//! - **Waiting**, **looking around**, **standing**, **spitting** (`ACTOR_EN_NUTSBALL` 1) and
//!   **burrowing** as `En_Dekunuts` does, but a round is one nut and Link within 120 always
//!   sends it down. A hit by anything but a nut (`EnHintnuts_ColliderCheck`) only sends it down.
//! - **Its own nut** back at it (or the hammer's shock wave): `EnHintnuts_HitByScrubProjectile1`
//!   makes a talking one (params 0, or the third of the puzzle with the first two done) friendly
//!   (`ACTORCAT_BG`), and `EnHintnuts_HitByScrubProjectile2` pops it out: a puzzle one still an
//!   enemy counts in `sPuzzleCounter` (the next in order: one more; else negative) and
//!   **freezes** (`EnHintnuts_BeginFreeze`, `EnHintnuts_Freeze`, blue, fainting); the rest
//!   **run** (`EnHintnuts_Run`: five loops away from Link, then home).
//! - **The puzzle**: three knocked out out of order (`sPuzzleCounter` -3) is `NA_SE_SY_ERROR`, and
//!   the frozen ones sink 35 and come back up to wait; in order, the third runs, and the first two
//!   wait frozen.
//! - **Caught** (a friendly one running, within 130: `Actor_OfferTalkNearColChkInfoCylinder`, taken
//!   by itself when it touches Link or he locks on), it **talks** (`EnHintnuts_Talk`); the text's
//!   event leaves (`EnHintnuts_SetupLeave`: a recovery heart, running off at 3 for 100 frames or
//!   until behind the camera). The third of the puzzle sets the room cleared (`Flags_SetClear`)
//!   and `sPuzzleCounter` 3: the frozen two sink and are gone, and every flower is a prop.
//!
//! The whole overlay is ported. Not ported: the circle shadow (`ActorShadow_DrawCircle`, 35),
//! not ported for any actor.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, approach_s, cos_s, scaled_step_to_s, sin_s, smooth_step_to_s, step_to_f, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_ENEMY, ACTORCAT_PROP, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::*;
use oot_game::collision_check::*;
use oot_game::message::TEXT_STATE_EVENT;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};

use crate::en_dekunuts::{change_category, spit_nose_scale};

/// `ACTOR_EN_HINTNUTS` (`actor_table.h`).
pub const ACTOR_EN_HINTNUTS: i16 = 0x0192;
const OBJECT: &str = "object_hintnuts";

/// `En_Hintnuts_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_HINTNUTS, name: "En_Hintnuts", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE, object: OBJECT };

/// The flower's params (`0xA` in the C).
pub const HINTNUTS_FLOWER: i16 = 0xA;

/// `NAVI_ENEMY_DEKU_SCRUB` (`actor.h`).
const NAVI_ENEMY_DEKU_SCRUB: u8 = 0x0A;

/// `sCylinderInit`: hit by everything but the shield and the mirror's ray; 18 by 32.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 18, height: 32, y_shift: 0, pos: [0, 0, 0] },
};

/// `sColChkInfoInit`: 1 health, 18 by 32, heavy.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 1, cyl_radius: 18, cyl_height: 32, mass: MASS_HEAVY };

/// `sPuzzleCounter`: the overlay's static, shared by the scrubs (`PlayState::overlay_static`).
#[derive(Debug, Default)]
pub struct Statics {
    pub puzzle_counter: i16,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Wait,
    LookAround,
    Stand,
    ThrowNut,
    Burrow,
    BeginRun,
    BeginFreeze,
    Run,
    Talk,
    Leave,
    Freeze,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Wait => "EnHintnuts_Wait",
            Action::LookAround => "EnHintnuts_LookAround",
            Action::Stand => "EnHintnuts_Stand",
            Action::ThrowNut => "EnHintnuts_ThrowNut",
            Action::Burrow => "EnHintnuts_Burrow",
            Action::BeginRun => "EnHintnuts_BeginRun",
            Action::BeginFreeze => "EnHintnuts_BeginFreeze",
            Action::Run => "EnHintnuts_Run",
            Action::Talk => "EnHintnuts_Talk",
            Action::Leave => "EnHintnuts_Leave",
            Action::Freeze => "EnHintnuts_Freeze",
        }
    }
}

/// The animations of `object_hintnuts` it plays.
struct Anims {
    up: Anim,
    look_around: Anim,
    spit: Anim,
    stand: Anim,
    burrow: Anim,
    unburrow: Anim,
    run: Anim,
    talk: Anim,
    freeze: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Hintnuts: {e:#}")).ok();
        Some(Anims {
            up: get("gHintNutsUpAnim")?,
            look_around: get("gHintNutsLookAroundAnim")?,
            spit: get("gHintNutsSpitAnim")?,
            stand: get("gHintNutsStandAnim")?,
            burrow: get("gHintNutsBurrowAnim")?,
            unburrow: get("gHintNutsUnburrowAnim")?,
            run: get("gHintNutsRunAnim")?,
            talk: get("gHintNutsTalkAnim")?,
            freeze: get("gHintNutsFreezeAnim")?,
        })
    }
}

/// `Math_Vec3f_DistXZ`.
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

/// `sPuzzleCounter`.
fn puzzle_counter(play: &mut PlayState) -> &mut i16 {
    &mut play.overlay_static::<Statics>(ACTOR_EN_HINTNUTS).puzzle_counter
}

pub struct EnHintnuts {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    pub action: Action,
    /// `animFlagAndTimer`: "0x1000 bit denotes that projectile has been thrown".
    pub anim_flag_and_timer: i16,
    /// `unk_196`: the way it runs.
    pub unk_196: i16,
    /// `textIdCopy`.
    pub text_id_copy: u16,
    pub collider: ColliderCylinder,
}

impl EnHintnuts {
    /// `EnHintnuts_Init`: the flower untargetable; the scrub with its skeleton
    /// (`@bug (game)`: `gHintNutsSkel` is a flex skeleton, set up and drawn as a normal one; its
    /// limbs' lists load no matrices, so it draws the same), collider, text, `sPuzzleCounter` reset;
    /// a scrub with text 0x109B is gone once room 9 is cleared; waiting, its flower spawned as its
    /// child.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: gravity -1, NAVI_ENEMY_DEKU_SCRUB, lockOnArrowOffset 2600.
        actor.gravity = -1.0;
        actor.navi_enemy_id = NAVI_ENEMY_DEKU_SCRUB;
        actor.target_arrow_offset = 2600.0;
        let collider = ColliderCylinder::new(&CYLINDER_INIT);
        if actor.params == HINTNUTS_FLOWER {
            actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE);
            return Box::new(EnHintnuts {
                actor,
                skel: SkelAnimeStd::init_flex(0, None),
                skeleton: None,
                anims: None,
                action: Action::Wait,
                anim_flag_and_timer: 0,
                unk_196: 0,
                text_id_copy: 0,
                collider,
            });
        }
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 35): the circle shadow isn't ported.
        actor.shape_y_offset = 0.0;
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, "gHintNutsSkel").map_err(|e| log::error!("En_Hintnuts: {e:#}")).ok());
        let anims = Anims::load(play);
        // SkelAnime_Init(&gHintNutsSkel, &gHintNutsStandAnim): 10 entries.
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(9);
        let skel = SkelAnimeStd::init_flex(limbs, anims.as_ref().map(|a| a.stand.clone()));
        actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
        let base_text = ((actor.params as u16 >> 8) & 0xFF) as i16;
        oot_game::npc::actor_set_text_with_prefix(play.scene_id, &mut actor, base_text);
        let text_id_copy = actor.text_id;
        actor.params &= 0xFF;
        *puzzle_counter(play) = 0;
        let mut n = EnHintnuts { actor, skel, skeleton, anims, action: Action::Wait, anim_flag_and_timer: 0, unk_196: 0, text_id_copy, collider };
        if n.actor.text_id == 0x109B && play.flags.get_clear(0x9) {
            n.actor.kill();
            return Box::new(n);
        }
        n.setup_wait(play);
        let (pos, rot_y) = (n.actor.world_pos, n.actor.world_rot.y);
        if let Err(e) = play.actor_spawn_as_child(&mut n.actor, ACTOR_EN_HINTNUTS, pos, [0, rot_y, 0], HINTNUTS_FLOWER) {
            log::debug!("En_Hintnuts: its flower: {e:?}");
        }
        Box::new(n)
    }

    fn is_flower(&self) -> bool {
        self.actor.params == HINTNUTS_FLOWER
    }

    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    fn change(&mut self, a: Option<Anim>, speed: f32, mode: u8, morph: f32, to_last: bool) {
        if let Some(a) = a {
            let end = if to_last { a.last_frame() } else { 0.0 };
            self.skel.change(a, speed, 0.0, end, mode, morph);
        }
    }

    /// `Animation_GetLastFrame(anim)`.
    fn last_frame(&self, f: impl Fn(&Anims) -> &Anim) -> f32 {
        self.anims.as_ref().map(|a| f(a).last_frame()).unwrap_or(0.0)
    }

    /// `EnHintnuts_HitByScrubProjectile1`: a talking one (any text; params 0, or the third with
    /// the puzzle at 2) still an enemy becomes friendly, a `ACTORCAT_BG` actor.
    fn hit_by_scrub_projectile1(&mut self, play: &mut PlayState) {
        let counter = *puzzle_counter(play);
        if self.actor.text_id != 0 && self.actor.category == ACTORCAT_ENEMY && (self.actor.params == 0 || counter == 2) {
            self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE);
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY;
            self.actor.category = ACTORCAT_BG;
            if let Some(me) = play.cur_actor {
                play.actors.change_category(me, ACTORCAT_BG);
            }
        }
    }

    /// `EnHintnuts_SetupWait`: as `EnDekunuts_SetupWait`.
    fn setup_wait(&mut self, play: &mut PlayState) {
        self.change(self.anim(|a| &a.up), 0.0, ANIMMODE_ONCE, 0.0, true);
        self.anim_flag_and_timer = play.rand.s16_offset(100, 50);
        self.collider.dim.height = 5;
        self.actor.world_pos = self.actor.home_pos;
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::Wait;
    }

    /// `EnHintnuts_SetupLookAround`.
    fn setup_look_around(&mut self) {
        self.change(self.anim(|a| &a.look_around), 1.0, ANIMMODE_LOOP, 0.0, true);
        self.anim_flag_and_timer = 2;
        self.action = Action::LookAround;
    }

    /// `EnHintnuts_SetupThrowScrubProjectile`.
    fn setup_throw_scrub_projectile(&mut self) {
        self.change(self.anim(|a| &a.spit), 1.0, ANIMMODE_ONCE, 0.0, true);
        self.action = Action::ThrowNut;
    }

    /// `EnHintnuts_SetupStand`.
    fn setup_stand(&mut self) {
        self.change(self.anim(|a| &a.stand), 1.0, ANIMMODE_LOOP, -3.0, false);
        self.anim_flag_and_timer = if self.action == Action::ThrowNut { 2 | 0x1000 } else { 1 };
        self.action = Action::Stand;
    }

    /// `EnHintnuts_SetupBurrow`.
    fn setup_burrow(&mut self, play: &mut PlayState) {
        self.change(self.anim(|a| &a.burrow), 1.0, ANIMMODE_ONCE, -5.0, true);
        audio_play_actor_sfx2(play, NA_SE_EN_NUTS_DOWN);
        self.action = Action::Burrow;
    }

    /// `EnHintnuts_HitByScrubProjectile2`: popping out, 37 high, the collider off; a puzzle one
    /// still an enemy counts (the next in order: one more; else the count goes negative, one
    /// lower each wrong one; -4 starts over) and freezes; the rest run.
    fn hit_by_scrub_projectile2(&mut self, play: &mut PlayState) {
        self.change(self.anim(|a| &a.unburrow), 1.0, ANIMMODE_ONCE, -3.0, true);
        self.collider.dim.height = 37;
        audio_play_actor_sfx2(play, NA_SE_EN_NUTS_DAMAGE);
        self.collider.base.ac_flags &= !AC_ON;
        if self.actor.params > 0 && self.actor.params < 4 && self.actor.category == ACTORCAT_ENEMY {
            let params = self.actor.params;
            let c = puzzle_counter(play);
            if *c == -4 {
                *c = 0;
            }
            if params == *c + 1 {
                *c += 1;
            } else {
                if *c > 0 {
                    *c = -*c;
                }
                *c -= 1;
            }
            self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            self.action = Action::BeginFreeze;
        } else {
            self.action = Action::BeginRun;
        }
    }

    /// `EnHintnuts_SetupRun`: five loops.
    fn setup_run(&mut self) {
        self.change(self.anim(|a| &a.run), 1.0, ANIMMODE_LOOP, 0.0, true);
        self.anim_flag_and_timer = 5;
        self.action = Action::Run;
    }

    /// `EnHintnuts_SetupTalk`.
    fn setup_talk(&mut self) {
        self.change(self.anim(|a| &a.talk), 1.0, ANIMMODE_LOOP, -5.0, false);
        self.action = Action::Talk;
        self.actor.speed_xz = 0.0;
    }

    /// `EnHintnuts_SetupLeave`: running off at 3 for 100 frames, no longer pushing; a recovery
    /// heart left (`ACTOR_EN_ITEM00` 3).
    fn setup_leave(&mut self, play: &mut PlayState) {
        self.change(self.anim(|a| &a.run), 1.0, ANIMMODE_LOOP, -5.0, false);
        self.actor.speed_xz = 3.0;
        self.anim_flag_and_timer = 100;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.collider.base.oc_flags1 &= !OC1_ON;
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        audio_play_actor_sfx2(play, NA_SE_EN_NUTS_DAMAGE);
        let pos = self.actor.world_pos;
        // ITEM00_RECOVERY_HEART.
        if let Err(e) = play.actor_spawn(crate::en_item00::ACTOR_EN_ITEM00, pos, [0, 0, 0], 0x3) {
            log::debug!("En_Hintnuts: the heart: {e:?}");
        }
        self.action = Action::Leave;
    }

    /// `EnHintnuts_SetupFreeze`: fainting, blue (held by its timer at 1), untargetable; the third
    /// wrong one plays `NA_SE_SY_ERROR` and starts the puzzle over (-4).
    fn setup_freeze(&mut self, play: &mut PlayState) {
        self.change(self.anim(|a| &a.freeze), 1.0, ANIMMODE_LOOP, 0.0, true);
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, 100);
        self.actor.color_filter_timer = 1;
        self.anim_flag_and_timer = 0;
        audio_play_actor_sfx2(play, NA_SE_EN_NUTS_FAINT);
        let c = puzzle_counter(play);
        if *c == -3 {
            *c = -4;
            play.audio.play_sfx_centered(NA_SE_SY_ERROR);
        }
        self.action = Action::Freeze;
    }

    /// `EnHintnuts_Wait`: as `EnDekunuts_Wait`.
    fn wait(&mut self, play: &mut PlayState) {
        let has_slow_playback_speed = self.skel.play_speed < 0.5;
        if has_slow_playback_speed && self.anim_flag_and_timer != 0 {
            self.anim_flag_and_timer -= 1;
        }
        if self.skel.on_frame(9.0) {
            self.collider.base.ac_flags |= AC_ON;
        } else if self.skel.on_frame(8.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_UP);
        }
        self.collider.dim.height = (5.0 + ((self.skel.cur_frame.clamp(9.0, 12.0) - 9.0) * 9.0)) as i16;
        if !has_slow_playback_speed && self.actor.xz_dist_to_player < 120.0 {
            self.setup_burrow(play);
        } else if self.skel.update() {
            if self.actor.xz_dist_to_player < 120.0 {
                self.setup_burrow(play);
            } else if self.anim_flag_and_timer == 0 && self.actor.xz_dist_to_player > 320.0 {
                self.setup_look_around();
            } else {
                self.setup_stand();
            }
        }
        if has_slow_playback_speed && 160.0 < self.actor.xz_dist_to_player && self.actor.y_dist_to_player.abs() < 120.0 && (self.anim_flag_and_timer == 0 || self.actor.xz_dist_to_player < 480.0) {
            self.skel.play_speed = 1.0;
        }
    }

    /// `EnHintnuts_LookAround`.
    fn look_around(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(0.0) && self.anim_flag_and_timer != 0 {
            self.anim_flag_and_timer -= 1;
        }
        if self.actor.xz_dist_to_player < 120.0 || self.anim_flag_and_timer == 0 {
            self.setup_burrow(play);
        }
    }

    /// `EnHintnuts_Stand`: turning to Link (unless just after a spit); Link within 120 or after
    /// a spit's two loops, down; its loop done, a spit.
    fn stand(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(0.0) && self.anim_flag_and_timer != 0 {
            self.anim_flag_and_timer -= 1;
        }
        if self.anim_flag_and_timer & 0x1000 == 0 {
            approach_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 2, 0xE38);
        }
        if self.actor.xz_dist_to_player < 120.0 || self.anim_flag_and_timer == 0x1000 {
            self.setup_burrow(play);
        } else if self.anim_flag_and_timer == 0 {
            self.setup_throw_scrub_projectile();
        }
    }

    /// `EnHintnuts_ThrowNut`: turning to Link; Link within 120, down; the nut at frame 6
    /// (`ACTOR_EN_NUTSBALL` 1, `NA_SE_EN_NUTS_THROW` if it spawned).
    fn throw_nut(&mut self, play: &mut PlayState) {
        approach_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 2, 0xE38);
        if self.actor.xz_dist_to_player < 120.0 {
            self.setup_burrow(play);
        } else if self.skel.update() {
            self.setup_stand();
        } else if self.skel.on_frame(6.0) {
            let r = self.actor.shape_rot;
            let nut_pos = Vec3::new(self.actor.world_pos.x + (sin_s(r.y) * 23.0), self.actor.world_pos.y + 12.0, self.actor.world_pos.z + (cos_s(r.y) * 23.0));
            if play.actor_spawn(crate::en_nutsball::ACTOR_EN_NUTSBALL, nut_pos, [r.x, r.y, r.z], crate::en_nutsball::EN_NUTSBALL_TYPE_HINTNUTS).is_ok() {
                audio_play_actor_sfx2(play, NA_SE_EN_NUTS_THROW);
            }
        }
    }

    /// `EnHintnuts_Burrow`: as `EnDekunuts_Burrow`.
    fn burrow(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.setup_wait(play);
        } else {
            self.collider.dim.height = (5.0 + ((3.0 - self.skel.cur_frame.clamp(1.0, 3.0)) * 12.0)) as i16;
        }
        if self.skel.on_frame(4.0) {
            self.collider.base.ac_flags &= !AC_ON;
        }
        approach_f(&mut self.actor.world_pos.x, self.actor.home_pos.x, 0.5, 3.0);
        approach_f(&mut self.actor.world_pos.z, self.actor.home_pos.z, 0.5, 3.0);
    }

    /// `EnHintnuts_BeginRun`.
    fn begin_run(&mut self) {
        if self.skel.update() {
            self.unk_196 = self.actor.yaw_towards_player.wrapping_add(i16::MIN);
            self.setup_run();
        }
        approach_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 2, 0xE38);
    }

    /// `EnHintnuts_BeginFreeze`.
    fn begin_freeze(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.setup_freeze(play);
        }
    }

    /// `EnHintnuts_CheckProximity`: a friendly one offers to talk within 130 (with its text),
    /// taken at once when it touches Link or he locks on.
    fn check_proximity(&mut self, play: &mut PlayState) {
        if self.actor.category != ACTORCAT_ENEMY {
            if self.collider.base.oc_flags1 & OC1_HIT != 0 || self.actor.is_targeted {
                self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            } else {
                self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            }
            if self.actor.xz_dist_to_player < 130.0 {
                self.actor.text_id = self.text_id_copy;
                let a = self.actor.clone();
                oot_game::npc::offer_talk_default(play, &a);
            }
        }
    }

    /// `EnHintnuts_Run`: up to 7.5, a step on frames 0 and 6; once turned its way, a new way
    /// (home out of water, along a wall, away from Link for its five loops, then home or 45°
    /// off Link's way); facing back the way it runs. A talk taken, it talks; home and level
    /// after its loops, an enemy again and down; else it offers to talk.
    fn run(&mut self, play: &mut PlayState) {
        self.skel.update();
        let temp_ret = self.skel.on_frame(0.0);
        if temp_ret && self.anim_flag_and_timer != 0 {
            self.anim_flag_and_timer -= 1;
        }
        if temp_ret || self.skel.on_frame(6.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_WALK);
        }
        step_to_f(&mut self.actor.speed_xz, 7.5, 1.0);
        if smooth_step_to_s(&mut self.actor.world_rot.y, self.unk_196, 1, 0xE38, 0xB6) == 0 {
            if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 {
                self.unk_196 = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
            } else if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
                self.unk_196 = self.actor.wall_yaw;
            } else if self.anim_flag_and_timer == 0 {
                let diff_rot_init = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
                let diff_rot = diff_rot_init.wrapping_sub(self.actor.yaw_towards_player);
                if (diff_rot as i32).abs() >= 0x2001 {
                    self.unk_196 = diff_rot_init;
                } else {
                    let phi_f0 = if 0.0 <= diff_rot as f32 { 1.0 } else { -1.0 };
                    self.unk_196 = ((phi_f0 * -8192.0) + self.actor.yaw_towards_player as f32) as i32 as i16;
                }
            } else {
                self.unk_196 = self.actor.yaw_towards_player.wrapping_add(i16::MIN);
            }
        }
        self.actor.shape_rot.y = self.actor.world_rot.y.wrapping_add(i16::MIN);
        if oot_game::npc::process_talk_request(&mut self.actor) {
            self.setup_talk();
        } else if self.anim_flag_and_timer == 0 && dist_xz(self.actor.world_pos, self.actor.home_pos) < 20.0 && (self.actor.world_pos.y - self.actor.home_pos.y).abs() < 2.0 {
            self.actor.speed_xz = 0.0;
            if self.actor.category == ACTORCAT_BG {
                self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED);
                self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE;
                self.actor.category = ACTORCAT_ENEMY;
                if let Some(me) = play.cur_actor {
                    play.actors.change_category(me, ACTORCAT_ENEMY);
                }
            }
            self.setup_burrow(play);
        } else {
            self.check_proximity(play);
        }
    }

    /// `EnHintnuts_Talk`: turning to Link; the text's event, it leaves.
    fn talk(&mut self, play: &mut PlayState) {
        self.skel.update();
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 0x3, 0x400, 0x100);
        if play.message_state() == TEXT_STATE_EVENT {
            self.setup_leave(play);
        }
    }

    /// `EnHintnuts_Leave`: running off, a step on frames 0 and 6, turning along a wall, else
    /// away from the camera (up to 90° off its back towards Link's side), 0x800 a frame; its 100
    /// frames done or behind the camera: the text closed, the third of the puzzle clears the room
    /// (`sPuzzleCounter` 3), its flower a prop, gone.
    fn leave(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.anim_flag_and_timer != 0 {
            self.anim_flag_and_timer -= 1;
        }
        if self.skel.on_frame(0.0) || self.skel.on_frame(6.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_WALK);
        }
        let temp_a1 = if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.actor.wall_yaw
        } else {
            let cam_yaw = play.cam_dir_yaw();
            let t = self.actor.yaw_towards_player.wrapping_sub(cam_yaw).wrapping_sub(i16::MIN);
            if (t as i32).abs() >= 0x4001 { cam_yaw.wrapping_add(i16::MIN) } else { cam_yaw.wrapping_sub(t >> 1).wrapping_add(i16::MIN) }
        };
        scaled_step_to_s(&mut self.actor.shape_rot.y, temp_a1, 0x800);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        if self.anim_flag_and_timer == 0 || self.actor.projected_pos.z < 0.0 {
            play.with_msg(|m, f| m.close_textbox(f.audio));
            if self.actor.params == 3 {
                let room = self.actor.room;
                play.flags.set_clear(room);
                *puzzle_counter(play) = 3;
            }
            if let Some(c) = self.actor.child {
                change_category(play, c, ACTORCAT_PROP);
            }
            self.actor.kill();
        }
    }

    /// `EnHintnuts_Freeze`: blue, fainting each loop; with the puzzle solved (3) its flower a
    /// prop and down to stay, with it started over (-4) down to come back; sinking 35 at 7, then
    /// gone, or targetable again with its health back, waiting.
    fn freeze(&mut self, play: &mut PlayState) {
        self.actor.color_filter_timer = 1;
        self.skel.update();
        if self.skel.on_frame(0.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_FAINT);
        }
        if self.anim_flag_and_timer == 0 {
            let c = *puzzle_counter(play);
            if c == 3 {
                if let Some(ch) = self.actor.child {
                    change_category(play, ch, ACTORCAT_PROP);
                }
                self.anim_flag_and_timer = 1;
            } else if c == -4 {
                self.anim_flag_and_timer = 2;
            }
        } else if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y - 35.0, 7.0) {
            if self.anim_flag_and_timer == 1 {
                self.actor.kill();
            } else {
                self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
                self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
                self.actor.col_chk_info.health = COL_CHK_INFO_INIT.health;
                self.actor.color_filter_timer = 0;
                self.setup_wait(play);
            }
        }
    }

    /// `EnHintnuts_ColliderCheck`: hit (its drop flag set) by anything but a scrub's nut, down;
    /// by a nut (or the hammer's shock wave), knocked out.
    fn collider_check(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            let elem = self.collider.info;
            self.actor.set_drop_flag(&elem, true);
            let ac_id = self.collider.base.ac.and_then(|h| play.actors.actor(h)).map(|a| a.id);
            if ac_id != Some(crate::en_nutsball::ACTOR_EN_NUTSBALL) {
                self.setup_burrow(play);
            } else {
                self.hit_by_scrub_projectile1(play);
                self.hit_by_scrub_projectile2(play);
            }
        } else if play.actors.unk_02 != 0 {
            self.hit_by_scrub_projectile1(play);
            self.hit_by_scrub_projectile2(play);
        }
    }
}

impl ActorImpl for EnHintnuts {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnHintnuts_Update` (nothing for the flower; frozen, it doesn't move).
    fn update(&mut self, play: &mut PlayState) {
        if self.is_flower() {
            return;
        }
        self.collider_check(play);
        match self.action {
            Action::Wait => self.wait(play),
            Action::LookAround => self.look_around(play),
            Action::Stand => self.stand(play),
            Action::ThrowNut => self.throw_nut(play),
            Action::Burrow => self.burrow(play),
            Action::BeginRun => self.begin_run(),
            Action::BeginFreeze => self.begin_freeze(play),
            Action::Run => self.run(play),
            Action::Talk => self.talk(play),
            Action::Leave => self.leave(play),
            Action::Freeze => self.freeze(play),
        }
        if self.action != Action::Freeze && self.action != Action::BeginFreeze {
            self.actor.move_forward();
            let (r, h) = (self.collider.dim.radius as f32, self.collider.dim.height as f32);
            self.actor.update_bg_check_info(&play.col, 20.0, r, h, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        }
        self.collider.update(&self.actor);
        if self.collider.base.ac_flags & AC_ON != 0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        if self.action == Action::Wait {
            self.actor.set_focus(self.skel.cur_frame);
        } else if self.action == Action::Burrow {
            self.actor.set_focus(20.0 - ((self.skel.cur_frame * 20.0) / self.last_frame(|a| &a.burrow)));
        } else {
            self.actor.set_focus(20.0);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.is_flower() as u32, (self.action == Action::ThrowNut) as u32];
        rs.values = vec![self.skel.cur_frame];
        rs
    }

    /// `EnHintnuts_Draw`: the flower (`gHintNutsFlowerDL`), or the skeleton, its nose (limb 5)
    /// swelling while it spits (`EnHintnuts_OverrideLimbDraw`: along on y, across on x and z).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let ([flower, throwing], [cur_frame]) = (rs.switches.as_slice(), rs.values.as_slice()) else { return };
        if *flower != 0 {
            crate::gfx_draw_dlist_opa(out, OBJECT, "gHintNutsFlowerDL", rs);
            return;
        }
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let nose = if *throwing != 0 { spit_nose_scale(*cur_frame) } else { None };
        let bones = skeleton.pose_override(&joints.rot, |limb, _pos, _rot| match nose {
            Some((across, along)) if limb == 5 => Mat4::from_scale(Vec3::new(across, along, across)),
            _ => Mat4::IDENTITY,
        });
        let model = oot_game::play::actor_draw_matrix(rs);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::mesh(OBJECT, "gHintNutsSkel")), transform: model, bones, params: Default::default() });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
