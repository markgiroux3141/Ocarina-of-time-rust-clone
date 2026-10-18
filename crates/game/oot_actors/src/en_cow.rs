//! `En_Cow` (`ovl_En_Cow/z_en_cow.c`): a cow (Impa's house, Link's house, the ranch and the
//! grottos), its body (`COW_TYPE_BODY`) and its tail (`COW_TYPE_TAIL`), the body spawning the
//! tail as its child (`Actor_SpawnAsChild` of its own type) at its own place, the tail moved 57 up
//! and 36 behind it (`EnCow_SetTailPos`).
//!
//! The body chews (`gCowBodyChewAnim`), now and then moves its head with a moo
//! (`gCowBodyMoveHeadAnim`, `NA_SE_EV_COW_CRY`), breathes (its scale's x and y swelling in turn,
//! `EnCow_UpdateAnimation`) and turns its head to Link within 150. In Link's house it is there only
//! for an adult with `EVENTCHKINF_HORSE_RACE_COW_UNLOCK` (Ingo's race won).
//!
//! Epona's Song: the message box sets `R_EPONAS_SONG_PLAYED` when it has been played
//! (`MSGMODE_SONG_PLAYED_ACT`, `PlayState::msg_ctx.r_eponas_song_played`); a cow with Link within
//! 150 and in front of it (`EnCow_Idle`, 25000 of its facing) clears it and talks at once
//! (`ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED`, 0x2006); with an empty bottle (0x2007) it gives Lon Lon
//! Milk (`GI_MILK` by `Actor_OfferGetItem`, `EnCow_GiveMilk`), without one 0x2013. A cow out of
//! reach marks itself (`COW_FLAG_FAILED_TO_GIVE_MILK`) and leaves the flag for another cow; the
//! next frame the first cow to update with its mark set clears it.
//!
//! The tail swishes (`gCowTailSwishAnim`) now and then, and once at once when Link first comes
//! within 150 behind the cow.
//!
//! Drawn as `EnCow_Draw` and `EnCow_DrawTail` do: the body's skeleton without its nose ring
//! (`EnCow_OverrideLimbDraw` nulls `COW_LIMB_NOSE_RING`), its head turned by `headRot`, its focus
//! at (0, -1300, 1100) on the head (`EnCow_PostLimbDraw`); the tail's skeleton as it is. From
//! meshes baked at import (docs/adr/0012-actor-bakes.md).
//!
//! The whole overlay is ported.
//!
//! Params: `type` whole (`COW_GET_TYPE`: 0 `COW_TYPE_BODY` the cow; 1 `COW_TYPE_TAIL` its tail,
//! spawned by the body). `world.rot.x` and `.z`: a rotation (Jabu-Jabu's cows lean on its walls).
//! Flags: reads EVENTCHKINF_HORSE_RACE_COW_UNLOCK (in Link's house); reads and clears
//! `R_EPONAS_SONG_PLAYED` (the message box's Epona's Song played).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::event_chk_inf;
use oot_game::collision_check::*;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::message::{OCARINA_MODE_00, OCARINA_MODE_04, TEXT_STATE_EVENT};
use oot_game::npc::{offer_talk, process_talk_request, textbox_is_closing};
use oot_game::pack::{BakeBody, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;
use oot_game::target::{pitch_to, yaw_to};

/// `ACTOR_EN_COW` (`actor_table.h`: 0x01C6).
pub const ACTOR_EN_COW: i16 = 0x01C6;
/// `OBJECT_COW` (`object_table.h`: 0x018B).
pub const OBJECT: &str = "object_cow";

/// `En_Cow_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_COW, name: "En_Cow", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT, schema: &SCHEMA };

/// `CowType` (`z_en_cow.h`).
pub const COW_TYPE_BODY: i16 = 0;
pub const COW_TYPE_TAIL: i16 = 1;
/// `COW_GET_TYPE`: `actor.params`.
pub const TYPE: Field = Field::whole("type", Kind::Named(&[(0, "COW_TYPE_BODY"), (1, "COW_TYPE_TAIL")]), "the cow, or its tail (spawned by the cow)");
/// `shape.rot.x`: the placement's pitch, drawn.
pub const ROT_X: Field = Field::rot("rot.x", Source::RotX, 0, 16, Kind::Rotation, "its pitch (Jabu-Jabu's cows)");
/// `shape.rot.z`: the placement's roll, drawn.
pub const ROT_Z: Field = Field::rot("rot.z", Source::RotZ, 0, 16, Kind::Rotation, "its roll (Jabu-Jabu's cows)");
/// `COW_FLAG_PLAYER_NEARBY`, `COW_FLAG_FAILED_TO_GIVE_MILK` (`z_en_cow.h`).
pub const COW_FLAG_PLAYER_NEARBY: u16 = 1 << 1;
pub const COW_FLAG_FAILED_TO_GIVE_MILK: u16 = 1 << 2;

/// `SCENE_LINKS_HOUSE` (`scene_table.h`: 0x34).
const SCENE_LINKS_HOUSE: u16 = 0x34;
/// `EVENTCHKINF_HORSE_RACE_COW_UNLOCK` (`save.h`: 0x1E).
pub const EVENTCHKINF_HORSE_RACE_COW_UNLOCK: u16 = 0x1E;

/// En_Cow's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, ROT_X, ROT_Z],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_HORSE_RACE_COW_UNLOCK)).reads().note("Link's house: there only for an adult with it"),
        FlagUse::new(Flag::Other("R_EPONAS_SONG_PLAYED")).reads().clears().note("Epona's Song played: milk"),
    ],
    todo: false,
    objects: &[],
};

/// `GI_MILK` (`item.h`: 0x50).
pub const GI_MILK: i16 = 0x50;
/// `NA_SE_EV_COW_CRY` (`environmentbank_table.h`: 0x28DF).
pub const NA_SE_EV_COW_CRY: u16 = 0x28DF;
/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;

// `CowLimb`, `CowTailLimb` (`object_cow.h`), 1-based as `EnCow_OverrideLimbDraw` sees them.
const COW_LIMB_HEAD: usize = 2;
const COW_LIMB_NOSE_RING: usize = 5;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_ENEMY, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 30, height: 40, y_shift: 0, pos: [0; 3] },
};

/// `sHeadFocusOffset`.
const HEAD_FOCUS_OFFSET: Vec3 = Vec3::new(0.0, -1300.0, 1100.0);

const BODY_SKEL: &str = "gCowBodySkel";
const TAIL_SKEL: &str = "gCowTailSkel";
const CHEW_ANIM: &str = "gCowBodyChewAnim";
const MOVE_HEAD_ANIM: &str = "gCowBodyMoveHeadAnim";
const TAIL_IDLE_ANIM: &str = "gCowTailIdleAnim";
const TAIL_SWISH_ANIM: &str = "gCowTailSwishAnim";

/// The body's mesh (no nose ring) and the tail's.
pub const BAKE_BODY: &str = "En_Cow/body";
pub const BAKE_TAIL: &str = "En_Cow/tail";

/// The meshes `EnCow_Draw` and `EnCow_DrawTail` draw.
pub fn bakes() -> Vec<MeshBake> {
    vec![
        MeshBake {
            name: BAKE_BODY.into(),
            object: OBJECT.into(),
            segments: Vec::new(),
            prelude: Vec::new(),
            body: BakeBody::Skeleton {
                file: OBJECT.into(),
                symbol: BODY_SKEL.into(),
                // EnCow_OverrideLimbDraw: *dList = NULL for the nose ring.
                limbs: vec![LimbOverride { limb: (COW_LIMB_NOSE_RING - 1) as u8, file: OBJECT.into(), symbol: String::new() }],
            },
        },
        MeshBake { name: BAKE_TAIL.into(), object: OBJECT.into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::Skeleton { file: OBJECT.into(), symbol: TAIL_SKEL.into(), limbs: Vec::new() } },
    ]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Idle,
    Talk,
    CheckForEmptyBottle,
    GiveMilk,
    GiveMilkWait,
    GiveMilkEnd,
    TalkEnd,
    /// `EnCow_IdleTail`: the tail's only action.
    IdleTail,
}

pub struct EnCow {
    pub actor: Actor,
    /// `colliders[COW_COLLIDER_FRONT]`, `colliders[COW_COLLIDER_REAR]` (the body's only).
    pub colliders: [ColliderCylinder; 2],
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub head_rot: [i16; 3],
    pub cow_flags: u16,
    pub animation_timer: u16,
    pub breath_timer: u16,
    pub action: Action,
    /// The body's chew and head move, or the tail's idle and swish.
    anims: [Anim; 2],
}

/// `EnCow_RotateY`: `vec` turned by `rot_y` about y.
fn rotate_y(vec: &mut Vec3, rot_y: i16) {
    let mut rot_calc_temp = cos_s(rot_y);
    let x_calc = (sin_s(rot_y) * vec.z) + (rot_calc_temp * vec.x);
    rot_calc_temp = sin_s(rot_y);
    vec.z = (cos_s(rot_y) * vec.z) + (-rot_calc_temp * vec.x);
    vec.x = x_calc;
}

impl EnCow {
    fn is_tail(&self) -> bool {
        TYPE.get(self.actor.params) == COW_TYPE_TAIL
    }

    /// `EnCow_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let tail = TYPE.get(actor.params) == COW_TYPE_TAIL;
        let (skel_name, anim_names) = if tail { (TAIL_SKEL, [TAIL_IDLE_ANIM, TAIL_SWISH_ANIM]) } else { (BODY_SKEL, [CHEW_ANIM, MOVE_HEAD_ANIM]) };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, skel_name)?, assets.animation(OBJECT, anim_names[0])?, assets.animation(OBJECT, anim_names[1])?)) })();
        let (skeleton, a0, a1) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Cow: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnCow {
            // SkelAnime_InitFlex(.., NULL, ..), then Animation_PlayLoop(chew or the tail's idle).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(a0.clone())),
            skeleton,
            colliders: [ColliderCylinder::new(&CYLINDER_INIT), ColliderCylinder::new(&CYLINDER_INIT)],
            head_rot: [0; 3],
            cow_flags: 0,
            animation_timer: 0,
            breath_timer: 0,
            action: Action::Idle,
            anims: [a0, a1],
            actor,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 72.0);

        match TYPE.get(this.actor.params) {
            COW_TYPE_BODY => {
                this.set_collider_pos();
                this.action = Action::Idle;
                if play.scene_id == SCENE_LINKS_HOUSE && (!play.save.adult || !play.save.get_event_chk_inf(EVENTCHKINF_HORSE_RACE_COW_UNLOCK)) {
                    this.actor.kill();
                    return Box::new(this);
                }
                let (pos, rot_y) = (this.actor.world_pos, this.actor.shape_rot.y);
                if let Err(e) = play.actor_spawn_as_child(&mut this.actor, ACTOR_EN_COW, pos, [0, rot_y, 0], COW_TYPE_TAIL) {
                    log::debug!("En_Cow: the tail not spawned: {e:?}");
                }
                this.animation_timer = (play.rand.zero_float(1000.0) + 40.0) as u16;
                this.breath_timer = 0;
                this.actor.target_mode = ATTENTION_RANGE_6;
                play.msg_ctx.r_eponas_song_played = false;
            }
            COW_TYPE_TAIL => {
                this.action = Action::IdleTail;
                this.set_tail_pos();
                this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                // (u16)Rand_ZeroFloat(1000) + 40.
                this.animation_timer = ((play.rand.zero_float(1000.0) as u16) as f32 + 40.0) as u16;
            }
            _ => {}
        }

        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.scale = Vec3::splat(0.01);
        this.cow_flags = 0;
        Box::new(this)
    }

    /// `EnCow_SetColliderPos`: the front cylinder 30 ahead, the rear one 20 behind.
    fn set_collider_pos(&mut self) {
        let p = self.actor.world_pos;
        let rot_y = self.actor.shape_rot.y;
        let mut vec = Vec3::new(0.0, 0.0, 30.0);
        rotate_y(&mut vec, rot_y);
        self.colliders[0].dim.pos = [(p.x + vec.x) as i16, p.y as i16, (p.z + vec.z) as i16];
        let mut vec = Vec3::new(0.0, 0.0, -20.0);
        rotate_y(&mut vec, rot_y);
        self.colliders[1].dim.pos = [(p.x + vec.x) as i16, p.y as i16, (p.z + vec.z) as i16];
    }

    /// `EnCow_SetTailPos`.
    fn set_tail_pos(&mut self) {
        let mut vec = Vec3::new(0.0, 57.0, -36.0);
        rotate_y(&mut vec, self.actor.shape_rot.y);
        self.actor.world_pos += vec;
    }

    /// `Animation_GetLastFrame` of animation `i`.
    fn last_frame(&self, i: usize) -> f32 {
        self.anims[i].last_frame()
    }

    /// `EnCow_UpdateAnimation`: the next chew's wait, a chew at once when Link first comes within
    /// 150, and the breath.
    fn update_animation(&mut self, play: &mut PlayState) {
        if self.animation_timer > 0 {
            self.animation_timer -= 1;
        } else {
            self.animation_timer = (play.rand.zero_float(500.0) + 40.0) as u16;
            let (cur, last) = (self.skel.cur_frame, self.last_frame(0));
            self.skel.change(self.anims[0].clone(), 1.0, cur, last, ANIMMODE_ONCE, 1.0);
        }

        if self.actor.xz_dist_to_player < 150.0 && self.cow_flags & COW_FLAG_PLAYER_NEARBY == 0 {
            self.cow_flags |= COW_FLAG_PLAYER_NEARBY;
            if self.skel.is(CHEW_ANIM) {
                self.animation_timer = 0;
            }
        }

        self.breath_timer += 1;
        if self.breath_timer > 48 {
            self.breath_timer = 0;
        }
        let t = self.breath_timer as i32;
        self.actor.scale.x = if t < 32 { ((sin_s((t * 0x0400) as i16) * (1.0 / 100.0)) + 1.0) * 0.01 } else { 0.01 };
        self.actor.scale.y = if t > 16 { ((sin_s(((t * 0x0400) - 0x4000) as i16) * (1.0 / 100.0)) + 1.0) * 0.01 } else { 0.01 };
    }

    fn message_close_textbox(play: &mut PlayState) {
        play.with_msg(|m, f| m.close_textbox(f.audio));
    }

    /// `EnCow_TalkEnd`.
    fn talk_end(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            Self::message_close_textbox(play);
            self.action = Action::Idle;
        }
    }

    /// `EnCow_GiveMilkEnd`.
    fn give_milk_end(&mut self, play: &mut PlayState) {
        if textbox_is_closing(play) {
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            self.action = Action::Idle;
        }
    }

    /// `EnCow_GiveMilkWait`.
    fn give_milk_wait(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.action = Action::GiveMilkEnd;
        } else {
            offer_get_item_range(play, &self.actor, GI_MILK, 10000.0, 100.0);
        }
    }

    /// `EnCow_GiveMilk`.
    fn give_milk(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            Self::message_close_textbox(play);
            self.action = Action::GiveMilkWait;
            offer_get_item_range(play, &self.actor, GI_MILK, 10000.0, 100.0);
        }
    }

    /// `EnCow_CheckForEmptyBottle`.
    fn check_for_empty_bottle(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            if oot_game::item::inventory_has_empty_bottle(&play.save) {
                play.continue_textbox(0x2007);
                self.action = Action::GiveMilk;
            } else {
                play.continue_textbox(0x2013);
                self.action = Action::TalkEnd;
            }
        }
    }

    /// `EnCow_Talk`.
    fn talk(&mut self, play: &mut PlayState) {
        if process_talk_request(&mut self.actor) {
            self.action = Action::CheckForEmptyBottle;
        } else {
            self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            offer_talk(play, &self.actor, 170.0);
            self.actor.text_id = 0x2006;
        }
        self.update_animation(play);
    }

    /// `EnCow_Idle`: Epona's Song played (`R_EPONAS_SONG_PLAYED`), with no song being played or
    /// one done (`OCARINA_MODE_00`, `_04`).
    fn idle(&mut self, play: &mut PlayState) {
        let mode = play.msg_ctx.ocarina_mode;
        if mode == OCARINA_MODE_00 || mode == OCARINA_MODE_04 {
            if play.msg_ctx.r_eponas_song_played {
                if self.cow_flags & COW_FLAG_FAILED_TO_GIVE_MILK != 0 {
                    self.cow_flags &= !COW_FLAG_FAILED_TO_GIVE_MILK;
                    play.msg_ctx.r_eponas_song_played = false;
                } else if self.actor.xz_dist_to_player < 150.0 && (self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y) as i32).abs() < 25000 {
                    play.msg_ctx.r_eponas_song_played = false;
                    self.action = Action::Talk;
                    self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
                    offer_talk(play, &self.actor, 170.0);
                    self.actor.text_id = 0x2006;
                } else {
                    self.cow_flags |= COW_FLAG_FAILED_TO_GIVE_MILK;
                }
            } else {
                self.cow_flags &= !COW_FLAG_FAILED_TO_GIVE_MILK;
            }
        }
        self.update_animation(play);
    }

    /// `EnCow_IdleTail`: the next swish's wait, a swish at once when Link first comes within 150
    /// behind the cow.
    fn idle_tail(&mut self, play: &mut PlayState) {
        if self.animation_timer > 0 {
            self.animation_timer -= 1;
        } else {
            self.animation_timer = (play.rand.zero_float(200.0) + 40.0) as u16;
            let (cur, last) = (self.skel.cur_frame, self.last_frame(0));
            self.skel.change(self.anims[0].clone(), 1.0, cur, last, ANIMMODE_ONCE, 1.0);
        }
        if self.actor.xz_dist_to_player < 150.0
            && (self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y) as i32).abs() > 25000
            && self.cow_flags & COW_FLAG_PLAYER_NEARBY == 0
        {
            self.cow_flags |= COW_FLAG_PLAYER_NEARBY;
            if self.skel.is(TAIL_IDLE_ANIM) {
                self.animation_timer = 0;
            }
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Idle => self.idle(play),
            Action::Talk => self.talk(play),
            Action::CheckForEmptyBottle => self.check_for_empty_bottle(play),
            Action::GiveMilk => self.give_milk(play),
            Action::GiveMilkWait => self.give_milk_wait(play),
            Action::GiveMilkEnd => self.give_milk_end(play),
            Action::TalkEnd => self.talk_end(play),
            Action::IdleTail => self.idle_tail(play),
        }
    }

    /// `EnCow_Update`: the body.
    fn update_body(&mut self, play: &mut PlayState) {
        play.collision_check_set_oc(&self.actor, 0, &mut self.colliders[0]);
        play.collision_check_set_oc(&self.actor, 1, &mut self.colliders[1]);
        // Actor_MoveXZGravity.
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);

        if self.skel.update() {
            if self.skel.is(CHEW_ANIM) {
                audio_play_actor_sfx2(play, NA_SE_EV_COW_CRY);
                let last = self.last_frame(1);
                self.skel.change(self.anims[1].clone(), 1.0, 0.0, last, ANIMMODE_ONCE, 1.0);
            } else {
                let last = self.last_frame(0);
                self.skel.change(self.anims[0].clone(), 1.0, 0.0, last, ANIMMODE_LOOP, 1.0);
            }
        }

        self.run_action(play);

        // ABS(Math_Vec3f_Yaw(world.pos, player's)) < 0xC000 always holds (an s16's ABS is at most
        // 0x8000): only the distance counts.
        let player = play.player.and_then(|h| play.actors.actor(h)).map(|p| (p.world_pos, p.focus_pos));
        let (mut target_x, mut target_y) = (0i16, 0i16);
        if let Some((ppos, pfocus)) = player
            && self.actor.xz_dist_to_player < 150.0
            && (yaw_to(self.actor.world_pos, ppos) as i32).abs() < 0xC000
        {
            target_x = pitch_to(self.actor.focus_pos, pfocus).clamp(-0x1000, 0x1000);
            target_y = yaw_to(self.actor.focus_pos, pfocus).wrapping_sub(self.actor.shape_rot.y).clamp(-0x2500, 0x2500);
        }
        smooth_step_to_s(&mut self.head_rot[0], target_x, 10, 200, 10);
        smooth_step_to_s(&mut self.head_rot[1], target_y, 10, 200, 10);
    }

    /// `EnCow_UpdateTail`.
    fn update_tail(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            if self.skel.is(TAIL_IDLE_ANIM) {
                let last = self.last_frame(1);
                self.skel.change(self.anims[1].clone(), 1.0, 0.0, last, ANIMMODE_ONCE, 1.0);
            } else {
                let last = self.last_frame(0);
                self.skel.change(self.anims[0].clone(), 1.0, 0.0, last, ANIMMODE_LOOP, 1.0);
            }
        }
        self.run_action(play);
    }

    /// The limb matrices: the body's head turned by `head_rot` (`EnCow_OverrideLimbDraw`).
    fn pose(&self, joints: &[[i16; 3]], head_rot: [i16; 2]) -> Vec<Mat4> {
        let tail = self.is_tail();
        self.skeleton.pose_override(joints, |limb, _pos, rot| {
            if !tail && limb == COW_LIMB_HEAD {
                rot[1] = rot[1].wrapping_add(head_rot[1]);
                rot[0] = rot[0].wrapping_add(head_rot[0]);
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnCow {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnCow_Update` for the body, `EnCow_UpdateTail` for the tail.
    fn update(&mut self, play: &mut PlayState) {
        if self.is_tail() { self.update_tail(play) } else { self.update_body(play) }
    }
    /// `EnCow_PostLimbDraw`: the body's focus on its head.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.is_tail() {
            return;
        }
        let bones = self.pose(&self.skel.joint_table, [self.head_rot[0], self.head_rot[1]]);
        if let Some(head) = bones.get(COW_LIMB_HEAD - 1) {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(HEAD_FOCUS_OFFSET);
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.head_rot[0], self.head_rot[1]];
        rs
    }
    /// `EnCow_Draw`, `EnCow_DrawTail`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let head = if rs.angles.len() >= 2 { [rs.angles[0], rs.angles[1]] } else { [0; 2] };
        let bones = self.pose(&joints.rot, head);
        let bake = if self.is_tail() { BAKE_TAIL } else { BAKE_BODY };
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
    }
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        if self.is_tail() {
            return None;
        }
        self.colliders.get_mut(id as usize).map(ColliderMut::Cylinder)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_y_turns_by_the_yaw() {
        // A quarter turn (0x4000): (0, 0, 30) to (30, 0, 0), as Math_SinS / Math_CosS give.
        let mut v = Vec3::new(0.0, 0.0, 30.0);
        rotate_y(&mut v, 0x4000);
        assert!((v.x - 30.0).abs() < 1e-3 && v.z.abs() < 1e-3, "{v}");
    }
}
