//! `En_Dog` (`ovl_En_Dog/z_en_dog.c`): the town's dogs. `params`: bit 15 following Link, bits 8 to 11
//! the dog's index (placed ones count from 0, made 1-based at their init), bits 4 to 7 the path (15
//! for none), bits 0 to 3 the colour (`colors`: cream, brown).
//!
//! A placed dog sits, then walks or runs its path (`EnDog_ChooseMovement`, `EnDog_FollowPath`,
//! with a sit, bow or bark between). Touched by Link (outside the market by day) it follows him
//! (`EnDog_CanFollow` writes `gSaveContext.dogParams`; `EnDog_FollowPlayer`): it runs, walks and
//! bows by his distance, and stops following beyond 400. Player respawns a following dog behind
//! Link in the next scene (`Player_Update`, `dogParams` bit 15). Another dog runs from Link while
//! one follows him (`EnDog_Wait`, `EnDog_RunAway`, `EnDog_FaceLink`). In the market by day the
//! dog lady's dog sets `EVENTINF_30` while it's near her on its path (her text, `En_Hy`). Richard
//! (index 1) is in the market by night only while `dogIsLost`; in the dog lady's house he sits
//! there once found.
//!
//! Drawn with its colour as the env colour (`gDPSetEnvColor`), from a bake per colour
//! (docs/adr/0012-actor-bakes.md).
//!
//! Params: `color` bits 0 to 3 (`colors`: 0 cream, 1 brown); `path` bits 4 to 7 (15: none);
//! `index` bits 8 to 11 (the dog's number, placed from 0 and made 1-based at init: placed 0 is
//! Richard, the dog lady's; a placed dog whose number is the following dog's is killed at init);
//! `following` bit 15 (1: the dog Player respawns behind Link, `dogParams` with bit 15; never
//! placed). Bits 0 to 14 become `gSaveContext.dogParams` when it starts to follow.
//! Flags: sets and clears EVENTINF_30 (the dog lady's dog near her, market by day); reads, sets
//! and clears `gSaveContext.dogParams` (the following dog); reads `gSaveContext.dogIsLost`
//! (Richard in the market by night, or sitting at home).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, smooth_step_to_f, smooth_step_to_s};
use glam::Vec3;
use oot_game::actor::{Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::event_inf;
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::path::{path_get_by_index, path_orient_and_get_dist_sq};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_DOG` (`actor_table.h`: 0x019B).
pub const ACTOR_EN_DOG: i16 = 0x019B;
pub const OBJECT: &str = "object_dog";

/// `En_Dog_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_DOG, name: "En_Dog", category: ACTORCAT_NPC, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_S(params, 0, 4)` (`EnDog_Draw`'s `colors[...]`).
pub const COLOR: Field = Field::params("color", 0, 4, Kind::Named(&[(0, "cream"), (1, "brown")]), "its coat's colour (the env colour)");
/// `PARAMS_GET_S(params, 4, 4)` (`Path_GetByIndex(play, ..., 0xF)`).
pub const PATH: Field = Field::params("path", 4, 4, Kind::PathIndex, "the path it walks and runs (15: none)");
/// `PARAMS_GET_S(params, 8, 4)`: the dog's number (`followingDog` compares it with `dogParams`).
pub const INDEX: Field = Field::params("index", 8, 4, Kind::Range(0, 14), "the dog's number, placed from 0 (made 1-based at init; placed 0: Richard)");
/// `PARAMS_GET_NOSHIFT(params, 15, 1)`: the dog following Link, respawned by Player.
pub const FOLLOWING: Field =
    Field::params("following", 15, 1, Kind::Named(&[(0, "a placed dog"), (1, "the dog following Link (Player's respawn)")]), "whether it's the dog Player respawns behind Link");

/// En_Dog's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[COLOR, PATH, INDEX, FOLLOWING],
    flags: &[
        FlagUse::new(event_inf!(EVENTINF_30)).sets().clears().note("the market by day: the dog lady's dog near her on its path"),
        FlagUse::new(Flag::Other("gSaveContext.dogParams")).reads().sets().clears().note("the dog following Link: its params, bits 0 to 14; cleared beyond 400"),
        FlagUse::new(Flag::Other("gSaveContext.dogIsLost")).reads().note("Richard (1): in the market by night only while lost; sitting at home once found"),
    ],
    todo: false,
    objects: &[],
};

// `SCENE_*` (`scene_table.h`).
const SCENE_MARKET_DAY: u16 = 0x20;
const SCENE_MARKET_NIGHT: u16 = 0x21;
const SCENE_DOG_LADY_HOUSE: u16 = 0x35;

/// `EVENTINF_30` (`save.h`): the dog lady's dog near her on its path.
const EVENTINF_30: u16 = 0x30;

/// `NA_SE_EV_SMALL_DOG_BARK`, `NA_SE_EV_CHIBI_WALK` (`environmentbank_table.h`: 0x28D8, 0x28EF).
pub const NA_SE_EV_SMALL_DOG_BARK: u16 = 0x28D8;
pub const NA_SE_EV_CHIBI_WALK: u16 = 0x28EF;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 16, height: 20, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`: mass 50.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: 50 };

/// `sAnimationInfo` (`EnDogAnimation` 0..7): animation, speed, start, end, mode, morph frames.
const ANIMATION_INFO: [(&str, f32, f32, f32, u8, f32); 8] = [
    ("gDogWalkAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gDogWalkAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -6.0),
    ("gDogRunAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -6.0),
    ("gDogBarkAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -6.0),
    ("gDogSitAnim", 1.0, 0.0, 4.0, ANIMMODE_ONCE, -6.0),
    ("gDogSitAnim", 1.0, 5.0, 25.0, ANIMMODE_LOOP_PARTIAL, -6.0),
    ("gDogBowAnim", 1.0, 0.0, 6.0, ANIMMODE_ONCE, -6.0),
    ("gDogBow2Anim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -6.0),
];

// `DogBehavior`.
pub const DOG_WALK: i16 = 0;
pub const DOG_RUN: i16 = 1;
pub const DOG_BARK: i16 = 2;
pub const DOG_SIT: i16 = 3;
pub const DOG_SIT_2: i16 = 4;
pub const DOG_BOW: i16 = 5;
pub const DOG_BOW_2: i16 = 6;

/// `colors` (`EnDog_Draw`): cream, brown.
const COLORS: [[u8; 4]; 2] = [[255, 255, 200, 0], [150, 100, 50, 0]];

/// The bake of colour `i`.
pub fn bake_name(i: usize) -> String {
    format!("En_Dog/color{i}")
}

/// The meshes `EnDog_Draw` draws: the skeleton after `gDPSetEnvColor` of each colour.
pub fn bakes() -> Vec<MeshBake> {
    (0..COLORS.len())
        .map(|i| MeshBake {
            name: bake_name(i),
            object: OBJECT.into(),
            // gDPSetEnvColor before the skeleton, as a prelude on segment 12 (segment 13 is the
            // skeleton's matrices).
            segments: vec![(0x0C, BakeSegment::Commands(vec![(0xFB00_0000, u32::from_be_bytes(COLORS[i])), (0xDF00_0000, 0)]))],
            prelude: vec![0x0C],
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gDogSkel".into(), limbs: Vec::new() },
        })
        .collect()
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    FollowPath,
    ChooseMovement,
    FollowPlayer,
    RunAway,
    FaceLink,
    Wait,
}

pub struct EnDog {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `path`: the scene path's index.
    pub path: Option<usize>,
    pub reverse: bool,
    pub waypoint: i16,
    pub unused_angle: i16,
    pub behavior_timer: i16,
    pub next_behavior: i16,
    pub behavior: i16,
    animations: Vec<AnimationInfo>,
}

impl EnDog {
    /// `EnDog_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = assets.skeleton(OBJECT, "gDogSkel").and_then(|s| {
            let animations = ANIMATION_INFO
                .iter()
                .map(|&(name, play_speed, start_frame, frame_count, mode, morph_frames)| {
                    assets.animation(OBJECT, name).map(|animation| AnimationInfo { animation, play_speed, start_frame, frame_count, mode, morph_frames })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok((s, animations))
        });
        let (skeleton, animations) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Dog: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnDog {
            actor,
            // SkelAnime_InitFlex(.., 13).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            action: Action::ChooseMovement,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            path: None,
            reverse: false,
            waypoint: 0,
            unused_angle: 0,
            behavior_timer: 0,
            next_behavior: DOG_WALK,
            behavior: DOG_WALK,
            animations,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 24.0);
        this.change_anim(0);
        let following = FOLLOWING.get(this.actor.params) != 0;
        if !following {
            // The placed dogs' index made 1-based.
            let index = INDEX.get(this.actor.params);
            this.actor.params = (this.actor.params & !(0xF << 8)) | ((index + 1) << 8); // schema: the C's write
        }
        let following_dog = INDEX.get(play.save.dog_params);
        if following_dog == INDEX.get(this.actor.params) && !following {
            this.actor.kill();
            return Box::new(this);
        }
        this.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        this.actor.scale = Vec3::splat(0.0075);
        this.waypoint = 0;
        this.actor.gravity = -1.0;
        this.path = path_get_by_index(play.setup_path_list(), PATH.get(this.actor.params), 0xF);
        let index = INDEX.get(this.actor.params);
        match play.scene_id {
            SCENE_MARKET_NIGHT => {
                if !play.save.dog_is_lost && index == 1 {
                    this.actor.kill();
                }
            }
            SCENE_DOG_LADY_HOUSE => {
                if !following {
                    if !play.save.dog_is_lost {
                        this.next_behavior = DOG_SIT;
                        this.action = Action::Wait;
                        this.actor.speed_xz = 0.0;
                    } else {
                        this.actor.kill();
                    }
                    return Box::new(this);
                }
            }
            _ => {}
        }
        if following {
            this.next_behavior = DOG_WALK;
            this.action = Action::FollowPlayer;
        } else {
            this.next_behavior = DOG_SIT;
            this.action = Action::ChooseMovement;
        }
        Box::new(this)
    }

    /// `Animation_ChangeByInfo(&skelAnime, sAnimationInfo, i)`.
    fn change_anim(&mut self, i: usize) {
        if let Some(info) = self.animations.get(i) {
            self.skel.change_by_info(info);
        }
    }

    /// `EnDog_PlayWalkSFX`, `EnDog_PlayRunSFX`, `EnDog_PlayBarkSFX`: on these frames of the animation.
    fn sfx_on(&self, play: &mut PlayState, anim: &str, frames: [f32; 2], sfx: u16) {
        if self.skel.is(anim) && (self.skel.cur_frame == frames[0] || self.skel.cur_frame == frames[1]) {
            audio_play_actor_sfx2(play, sfx);
        }
    }

    /// `EnDog_PlayAnimAndSFX`: the animation for a new behaviour, the sit's and bow's second part,
    /// the steps' and barks' sounds.
    fn play_anim_and_sfx(&mut self, play: &mut PlayState) {
        if self.behavior != self.next_behavior {
            if self.next_behavior == DOG_SIT_2 {
                self.next_behavior = DOG_SIT;
            }
            if self.next_behavior == DOG_BOW_2 {
                self.next_behavior = DOG_BOW;
            }
            self.behavior = self.next_behavior;
            let animation = match self.behavior {
                DOG_WALK => Some(1),
                DOG_RUN => Some(2),
                DOG_BARK => Some(3),
                DOG_SIT => Some(4),
                DOG_BOW => Some(6),
                // @bug (game): another behaviour leaves `animation` unset (uninitialised in the C);
                // none reaches here.
                _ => None,
            };
            if let Some(a) = animation {
                self.change_anim(a);
            }
        }
        match self.behavior {
            DOG_SIT => {
                if self.skel.on_frame(self.skel.end_frame) {
                    self.change_anim(5);
                    self.behavior = DOG_SIT_2;
                    self.next_behavior = DOG_SIT_2;
                }
            }
            DOG_BOW => {
                if self.skel.on_frame(self.skel.end_frame) {
                    self.change_anim(7);
                    self.behavior = DOG_BOW_2;
                    self.next_behavior = DOG_BOW_2;
                }
            }
            DOG_WALK => self.sfx_on(play, "gDogWalkAnim", [1.0, 7.0], NA_SE_EV_CHIBI_WALK),
            DOG_RUN => self.sfx_on(play, "gDogRunAnim", [2.0, 4.0], NA_SE_EV_CHIBI_WALK),
            DOG_BARK => self.sfx_on(play, "gDogBarkAnim", [13.0, 19.0], NA_SE_EV_SMALL_DOG_BARK),
            _ => {}
        }
    }

    /// `EnDog_CanFollow`: 2 when hit; 1 when Link touches it (not in the market by day) and no dog
    /// follows him yet (`dogParams` becomes its params); else 0.
    fn can_follow(&mut self, play: &mut PlayState) -> i8 {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            return 2;
        }
        if play.scene_id == SCENE_MARKET_DAY {
            return 0;
        }
        if self.collider.base.oc_flags2 & OC2_HIT_PLAYER != 0 {
            self.collider.base.oc_flags2 &= !OC2_HIT_PLAYER;
            if play.save.dog_params != 0 {
                return 0;
            }
            play.save.dog_params = self.actor.params & 0x7FFF; // schema: PARAMS_GET_S(params, 0, 15), kept for Player and En_Hy
            return 1;
        }
        0
    }

    /// `EnDog_UpdateWaypoint`.
    fn update_waypoint(&mut self, play: &PlayState) -> bool {
        let Some(path) = self.path.and_then(|i| play.setup_path_list().get(i)) else { return false };
        let count = path.count() as i16;
        self.waypoint += if self.reverse { -1 } else { 1 };
        if self.reverse {
            if self.waypoint < 0 {
                self.waypoint = count - 1;
            }
        } else if count - 1 < self.waypoint {
            self.waypoint = 0;
        }
        true
    }

    /// `EnDog_Orient`.
    fn orient(&mut self, play: &PlayState) -> bool {
        let path = self.path.and_then(|i| play.setup_path_list().get(i));
        let (dist_sq, yaw) = path_orient_and_get_dist_sq(&self.actor, path, self.waypoint);
        // @bug (game): without a path the target yaw is never written (the C reads its stack);
        // here it doesn't turn.
        if let Some(yaw) = yaw {
            smooth_step_to_s(&mut self.actor.world_rot.y, yaw, 10, 1000, 1);
        }
        if dist_sq > 0.0 && dist_sq < 1000.0 { self.update_waypoint(play) } else { false }
    }

    /// `EnDog_FollowPath`.
    fn follow_path(&mut self, play: &mut PlayState) {
        const BEHAVIORS: [i16; 3] = [DOG_SIT, DOG_BOW, DOG_BARK];
        if self.can_follow(play) == 1 {
            self.action = Action::FollowPlayer;
        }
        // DECR(behaviorTimer) != 0.
        let running = if self.behavior_timer != 0 {
            self.behavior_timer -= 1;
            self.behavior_timer != 0
        } else {
            false
        };
        if running {
            let speed = if self.next_behavior == DOG_WALK { 1.0 } else { 4.0 };
            smooth_step_to_f(&mut self.actor.speed_xz, speed, 0.4, 1.0, 0.0);
            self.orient(play);
            self.actor.shape_rot = self.actor.world_rot;
            // The dog lady's text (En_Hy) by where her dog is on its path.
            if self.waypoint < 9 {
                play.save.set_event_inf(EVENTINF_30);
            } else {
                play.save.clear_event_inf(EVENTINF_30);
            }
        } else {
            let frame = (play.state_frames % 3) as usize;
            self.next_behavior = BEHAVIORS[frame];
            // The behaviour's id as the timer's range (the unused { 40, 80, 20 } would make sense).
            self.behavior_timer = play.rand.s16_offset(60, BEHAVIORS[frame]);
            self.action = Action::ChooseMovement;
        }
    }

    /// `EnDog_ChooseMovement`.
    fn choose_movement(&mut self, play: &mut PlayState) {
        if self.can_follow(play) == 1 {
            self.action = Action::FollowPlayer;
        }
        // DECR(behaviorTimer) == 0.
        let done = if self.behavior_timer != 0 {
            self.behavior_timer -= 1;
            self.behavior_timer == 0
        } else {
            true
        };
        if done {
            self.behavior_timer = play.rand.s16_offset(200, 100);
            self.next_behavior = if play.state_frames % 2 != 0 { DOG_WALK } else { DOG_RUN };
            if self.next_behavior == DOG_RUN {
                self.behavior_timer /= 2;
            }
            self.action = Action::FollowPath;
        }
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 0.4, 1.0, 0.0);
    }

    /// `EnDog_FollowPlayer`.
    fn follow_player(&mut self, play: &mut PlayState) {
        if play.save.dog_params == 0 {
            self.next_behavior = DOG_SIT;
            self.action = Action::Wait;
            self.actor.speed_xz = 0.0;
            return;
        }
        let d = self.actor.xz_dist_to_player;
        let speed = if d > 400.0 {
            if self.next_behavior != DOG_SIT && self.next_behavior != DOG_SIT_2 {
                self.next_behavior = DOG_BOW;
            }
            play.save.dog_params = 0;
            0.0
        } else if d > 100.0 {
            self.next_behavior = DOG_RUN;
            4.0
        } else if d < 40.0 {
            if self.next_behavior != DOG_BOW && self.next_behavior != DOG_BOW_2 {
                self.next_behavior = DOG_BOW;
            }
            0.0
        } else {
            self.next_behavior = DOG_WALK;
            1.0
        };
        approach_f(&mut self.actor.speed_xz, speed, 0.6, 1.0);
        if !(d > 400.0) {
            smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 10, 1000, 1);
            self.actor.shape_rot = self.actor.world_rot;
        }
    }

    /// `EnDog_RunAway`.
    fn run_away(&mut self) {
        if self.actor.xz_dist_to_player < 200.0 {
            approach_f(&mut self.actor.speed_xz, 4.0, 0.6, 1.0);
            smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player ^ (0x8000u16 as i16), 10, 1000, 1);
        } else {
            self.action = Action::FaceLink;
        }
        self.actor.shape_rot = self.actor.world_rot;
    }

    /// `EnDog_FaceLink`: 200 away, it turns to Link and sits once turned; nearer, it runs.
    fn face_link(&mut self) {
        if 200.0 <= self.actor.xz_dist_to_player {
            self.next_behavior = DOG_WALK;
            approach_f(&mut self.actor.speed_xz, 1.0, 0.6, 1.0);
            let prev_rot_y = self.actor.world_rot.y;
            smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 10, 1000, 1);
            let abs_angle_diff = (self.actor.world_rot.y as f32 - prev_rot_y as f32).abs();
            if abs_angle_diff < 200.0 {
                self.next_behavior = DOG_SIT;
                self.action = Action::Wait;
                self.actor.speed_xz = 0.0;
            }
        } else {
            self.next_behavior = DOG_RUN;
            self.action = Action::RunAway;
        }
        self.actor.shape_rot = self.actor.world_rot;
    }

    /// `EnDog_Wait`: runs off when Link, with another dog following, comes within 200.
    fn wait(&mut self, play: &PlayState) {
        self.unused_angle = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if play.save.dog_params != 0 && self.actor.xz_dist_to_player < 200.0 {
            self.next_behavior = DOG_RUN;
            self.action = Action::RunAway;
        }
    }
}

impl ActorImpl for EnDog {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnDog_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.play_anim_and_sfx(play);
        self.skel.update();
        self.actor.update_bg_check_info(&play.col, self.collider.dim.radius as f32, self.collider.dim.height as f32 * 0.5, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        self.actor.move_forward();
        match self.action {
            Action::FollowPath => self.follow_path(play),
            Action::ChooseMovement => self.choose_movement(play),
            Action::FollowPlayer => self.follow_player(play),
            Action::RunAway => self.run_away(),
            Action::FaceLink => self.face_link(),
            Action::Wait => self.wait(play),
        }
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs
    }
    /// `EnDog_Draw`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        // @bug (game): a colour index past 1 reads past `colors`; none is placed.
        let color = COLOR.get(self.actor.params) as usize;
        let bones = self.skeleton.pose(joints);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(color.min(COLORS.len() - 1)))), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
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
