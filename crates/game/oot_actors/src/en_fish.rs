//! `En_Fish` (`ovl_En_Fish/z_en_fish.c`): a small fish, `gameplay_keep`'s `gFishSkel` at 0.01, the
//! one Link catches in a bottle and lets go of again (GAME-10 milestone 1). Its type is its whole
//! params (`EnFishType`):
//!
//! - **The normal fish** (`EN_FISH_TYPE_NORMAL`, -1): `Obj_Mure`'s groups of fish (type 2: Lake
//!   Hylia, Zora's Domain and Fountain, Gerudo Valley, the Tektite grotto), and six placed on each
//!   of Zora's Fountain's cutscene layers 5 and 6 (params 0xFFFF). Bobbing
//!   (`EnFish_UpdateBobbingUpDown`: `shape.yOffset` by two waves, `phase1` up 0x111 and `phase2`
//!   0x500 a frame, held to ±200), it **stays** (`EnFish_Normal_Stay`, `Rand_S16Offset(5, 35)`
//!   frames, slowing to 0) and **swims** (`EnFish_Normal_Swim`, `Rand_S16Offset(15, 45)`, at 1.8:
//!   home at 0xBB8 a frame past 80 from it or in its last 4 frames, else after the fish its group
//!   gives it, `actor.child`) in turn. Within 60 of Link it **flees** (`EnFish_Normal_FleePlayer`,
//!   `Rand_S16Offset(10, 40)`, at 4.2: home past 160 from it, else after its fish, else away from
//!   him swerving 0x2000 by `state.frames`' bits 4 and 5) until he's gone or its time is up. When
//!   its group makes it its own `child` (`ObjMure_SetChildToFollowPlayer`: Link has stayed within
//!   120 for 80 frames) it **follows Link** (`EnFish_Normal_FollowPlayer`,
//!   `Rand_S16Offset(10, 40)`, at 1.8: to 20 beside him, 0x9000 or 0x7000 off his yaw by
//!   `state.frames`' bit 6; home past 80 from it).
//! - **The dropped fish** (`EN_FISH_TYPE_DROPPED`, 0): the one Player's bottle lets go
//!   (`Player_Action_8084EFC0`, spawned at frame 76 of the drop, 5 ahead of his left hand, turned
//!   0x4000 on x). Always updating, a circle shadow (8). It **falls** (`EnFish_Dropped_Fall`,
//!   gravity -1 to -10, `gFishInAirAnim`, tipping on its side) for up to 300 frames (then, below
//!   `BGCHECK_Y_MIN + 10`, it's lost through the floor and gone); on the ground it **flaps**
//!   (`EnFish_Dropped_FlapOnGround`, 400 frames: each time it lands a hop, high one time in ten,
//!   low one in ten, else none, `NA_SE_EV_FISH_LEAP` with a hop or one time in five of the rest,
//!   twisting by `state.frames`), blinking its last 60 frames and gone; in water it **swims off**
//!   (`EnFish_Dropped_Swim`, 200 frames at 2.8, turning back from walls and the water's edge, 4 or
//!   10 under where it went in, shrinking its last 100) and is gone.
//! - **The tiny pond's** (`EN_FISH_TYPE_TINY_POND`, 1): a grotto's one fish (`kakusiana`). It
//!   holds near its home (`EnFish_TinyPond_Swim`: still, 0 at 0.04, or a dart, 0.5 at 0.1, in its
//!   last 4 frames of each `Rand_S16Offset(5, 80)`, its last 12 with Link within 60; home at 0xC8
//!   past 15 from it, else turned to `gameplayFrames × 0x80` in its last 4). Caught, it's gone for
//!   good (`EnFish_UpdateWaitReappear` kills it).
//! - **The Jabu-Jabu cutscene's**: the first dropped fish that updates while a cutscene runs with
//!   a cue on channel 1 (`actorCues[1]`) becomes the overlay's one cutscene fish
//!   (`sJabuCutsceneFish`, `EnFish_JabuCutscene_Enter`: on its side, 600 up, always updating). It
//!   follows the cue from its start to its end (`Environment_LerpWeight` of the cutscene's frame)
//!   plus a hop (`sJabuCutsceneOffsetY`): cue 1 **flaps on the ground** (a hop of 2 to 5 one time
//!   in ten it's down, `NA_SE_EV_FISH_LEAP`, falling 0.4 a frame), cue 2 **flies** (turning, the
//!   hop eased to 0), cue 3 is gone (as is the fish when the cutscene's cue goes first).
//!
//! **The catch**: within 32 of Link and with his position 16 nearer it within 20 across
//! (`EnFish_IsInRangeForCatch`), each frame it offers itself (`Actor_OfferGetItem` with `GI_MAX`,
//! 80 across, 20 up or down: nothing to take by hand, but it makes the fish Player's
//! `interactRangeActor`). Player's bottle swing (`Player_Action_SwingBottle`, finding
//! `ACTOR_EN_FISH` in `sBottleCatchInfo`) sets its `parent`; the fish's next update
//! (`Actor_HasParent`) clears it and the dropped fish is gone, any other **disappears**
//! (`EnFish_Disappear`: no draw, 0.001 big, `reappearTimer` 400). While it's away
//! (`EnFish_UpdateWaitReappear`) its action keeps running and it keeps moving, its timers held;
//! with 20 left it's drawn again, growing 0.001 a frame back to 0.01.
//!
//! The whole overlay is ported. Its collider is one sphere of 5 at the root, pushing (OC) only with
//! Link within 70; the draw (`EnFish_Draw`) is the skeleton (`SkelAnime_DrawFlexOpa`, the
//! `En_Fish/skel` bake, docs/adr/0012-actor-bakes.md), and `draw_update` its
//! `Collider_UpdateSpheres(0, ..)` on the actor's matrix. The dropped fish's shadow is the shape's
//! (`ActorShape_Init`, drawn by `Actor_Draw` while it has a draw). It spawns no effect.
//! @bug (game) (the C's): the cutscene fish is meant for giving Jabu-Jabu a fish, but nothing
//! checks the scene, so a dropped fish joins any cutscene with a cue on channel 1.
//! @bug (game): `EnFish_TinyPond_Swim` clamps its animation's speed with the bounds swapped
//! (`CLAMP(x, 1.5f, 0.5)`): 1.5 under 1.5, else 0.5.
//! `sJabuCutsceneFish` lives in the overlay's statics; the C's overlay unloads (and forgets it)
//! once no fish is left, which isn't modelled (`PlayState::overlay_statics`).
//!
//! Params: `params` whole (`EnFishType`): -1 `EN_FISH_TYPE_NORMAL` (as `Obj_Mure` spawns it; any
//! value but 0 and 1 is the normal fish), 0 `EN_FISH_TYPE_DROPPED` (Player's bottle's), 1
//! `EN_FISH_TYPE_TINY_POND` (a grotto's, gone once caught).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::BGCHECK_Y_MIN;
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, scaled_step_to_s, sin_s, smooth_step_to_f, step_to_angle_s, step_to_f, vec3f_yaw};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::collision_check::*;
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP_INTERP, Anim, SkelAnimeStd};

/// `ACTOR_EN_FISH` (`actor_table.h`: 0x0021).
pub const ACTOR_EN_FISH: i16 = 0x0021;
/// `OBJECT_GAMEPLAY_KEEP` (`object_table.h`: 0x0001).
pub const OBJECT: &str = "gameplay_keep";
/// `gFishSkel` (`gameplay_keep.xml`, Flex): 6 limbs (`SkelAnime_InitFlex`'s 7 with the root's).
pub const SKEL: &str = "gFishSkel";
/// `gFishInWaterAnim`, `gFishInAirAnim` (`gameplay_keep.xml`).
pub const ANIM_IN_WATER: &str = "gFishInWaterAnim";
pub const ANIM_IN_AIR: &str = "gFishInAirAnim";

/// `En_Fish_Profile`: `ACTORCAT_ITEMACTION`, no flags, `OBJECT_GAMEPLAY_KEEP`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_FISH, name: "En_Fish", category: ACTORCAT_ITEMACTION, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `EnFishType` (`z_en_fish.h`).
pub const EN_FISH_TYPE_NORMAL: i16 = -1;
pub const EN_FISH_TYPE_DROPPED: i16 = 0;
pub const EN_FISH_TYPE_TINY_POND: i16 = 1;

/// `params`: `EnFishType`, compared whole.
pub const TYPE: Field = Field::whole(
    "type",
    Kind::Named(&[(-1, "EN_FISH_TYPE_NORMAL"), (0, "EN_FISH_TYPE_DROPPED"), (1, "EN_FISH_TYPE_TINY_POND")]),
    "-1 (or any other value) a group's fish (Obj_Mure's), 0 dropped from Player's bottle, 1 a grotto's tiny pond's (gone once caught)",
);

/// En_Fish's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false, objects: &[], texts: &[] };

/// `NA_SE_EV_FISH_LEAP` (`sfx/environmentbank_table.h`: 0x2869).
pub const NA_SE_EV_FISH_LEAP: u16 = 0x2869;

/// `sJntSphElementsInit`: one sphere of 5 at the root, hit by nothing, pushing.
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    limb: 0,
    model_sphere: Sphere16 { center: [0, 0, 0], radius: 5 },
    scale: 100,
}];

/// `sJntSphInit`: pushing everything (`OC1_TYPE_ALL`), pushed as type 1.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `EnFishSpeedParams` (`target`, `fraction`, `step`): the tiny pond's fish still
/// (`sStaySpeedParams`) and darting (`sMoveSpeedParams`).
type SpeedParams = (f32, f32, f32);
const STAY_SPEED_PARAMS: SpeedParams = (0.0, 0.04, 0.09);
const MOVE_SPEED_PARAMS: SpeedParams = (0.5, 0.1, 0.15);

/// The skeleton (`gFishSkel`).
pub const BAKE: &str = "En_Fish/skel";

/// The mesh `EnFish_Draw` draws: `gFishSkel` after `Gfx_SetupDL_25Opa` (the bakes' start).
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake { name: BAKE.into(), object: OBJECT.into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() } }]
}

/// The overlay's statics: `sJabuCutsceneFish`, `sJabuCutsceneOffsetY`,
/// `sJabuCutsceneOffsetYSpeed`.
#[derive(Debug, Default)]
pub struct Statics {
    pub jabu_cutscene_fish: Option<ActorHandle>,
    pub jabu_cutscene_offset_y: f32,
    pub jabu_cutscene_offset_y_speed: f32,
}

/// The overlay's statics (`Default` the C's zeroed `.bss`).
pub fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_EN_FISH)
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    NormalStay,
    NormalSwim,
    NormalFleePlayer,
    NormalFollowPlayer,
    DroppedFall,
    DroppedFlapOnGround,
    DroppedSwim,
    TinyPondSwim,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::NormalStay => "EnFish_Normal_Stay",
            Action::NormalSwim => "EnFish_Normal_Swim",
            Action::NormalFleePlayer => "EnFish_Normal_FleePlayer",
            Action::NormalFollowPlayer => "EnFish_Normal_FollowPlayer",
            Action::DroppedFall => "EnFish_Dropped_Fall",
            Action::DroppedFlapOnGround => "EnFish_Dropped_FlapOnGround",
            Action::DroppedSwim => "EnFish_Dropped_Swim",
            Action::TinyPondSwim => "EnFish_TinyPond_Swim",
        }
    }
}

/// `EnFish_DistXZSq`.
fn dist_xz_sq(a: Vec3, b: Vec3) -> f32 {
    let dx = a.x - b.x;
    let dz = a.z - b.z;
    dx * dx + dz * dz
}

pub struct EnFish {
    pub actor: Actor,
    pub collider: ColliderJntSph,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anim_in_water: Option<Anim>,
    anim_in_air: Option<Anim>,
    pub action: Action,
    pub timer: i16,
    pub reappear_timer: i16,
    pub phase1: i16,
    pub phase2: i16,
    /// `updBgCheckInfoFlags`: `Actor_UpdateBgCheckInfo`'s flags, 0 for none.
    pub upd_bg_check_info_flags: u32,
    /// `actor.draw` isn't NULL (`EnFish_Disappear` and the last frames' blinking clear it).
    pub drawn: bool,
}

impl EnFish {
    /// `EnFish_Init`: `sInitChain` (0.01 big; culling 900, 40, 700), `gFishSkel` with
    /// `gFishInWaterAnim`, the sphere, mass 50, the two phases at random; the dropped fish always
    /// updating with a circle shadow (8), falling; the tiny pond's swimming; any other staying.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let (mut skeleton, mut anim_in_water, mut anim_in_air) = (None, None, None);
        if let Some(a) = play.assets.clone() {
            skeleton = a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Fish: {e:#}")).ok();
            anim_in_water = a.animation(OBJECT, ANIM_IN_WATER).map_err(|e| log::error!("En_Fish: {e:#}")).ok();
            anim_in_air = a.animation(OBJECT, ANIM_IN_AIR).map_err(|e| log::error!("En_Fish: {e:#}")).ok();
        }
        let ty = TYPE.get(actor.params);
        // sInitChain.
        actor.scale = Vec3::splat(0.01);
        actor.culling_volume_distance = 900.0;
        actor.culling_volume_scale = 40.0;
        actor.culling_volume_downward = 700.0;
        // SkelAnime_InitFlex(.., &gFishSkel, &gFishInWaterAnim, .., 7).
        let skel = SkelAnimeStd::init_flex(skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(6), anim_in_water.clone());
        actor.col_chk_info.mass = 50;
        let mut this = EnFish {
            collider: ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS),
            skel,
            skeleton,
            anim_in_water,
            anim_in_air,
            action: Action::NormalStay,
            timer: 0,
            reappear_timer: 0,
            phase1: 0,
            phase2: 0,
            upd_bg_check_info_flags: 0,
            drawn: true,
            actor,
        };
        this.phase1 = (play.rand.zero_one() * 65535.5) as i32 as i16;
        this.phase2 = (play.rand.zero_one() * 65535.5) as i32 as i16;
        if ty == EN_FISH_TYPE_DROPPED {
            this.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 8.0);
            this.dropped_setup_fall();
        } else if ty == EN_FISH_TYPE_TINY_POND {
            this.tiny_pond_setup_swim(play);
        } else {
            this.normal_setup_stay(play);
        }
        Box::new(this)
    }

    fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    /// `Animation_Change(&skelAnime, anim, 1.0f, 0.0f, Animation_GetLastFrame(anim),
    /// ANIMMODE_LOOP_INTERP, 2.0f)`.
    fn change_anim(&mut self, anim: Option<Anim>) {
        if let Some(a) = anim {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_LOOP_INTERP, 2.0);
        }
    }

    /// `EnFish_ChangeAnimInWater`.
    fn change_anim_in_water(&mut self) {
        self.change_anim(self.anim_in_water.clone());
    }

    /// `EnFish_ChangeAnimInAir`.
    fn change_anim_in_air(&mut self) {
        self.change_anim(self.anim_in_air.clone());
    }

    /// `EnFish_Disappear`: away 400 frames, 0.001 big, not drawn.
    fn disappear(&mut self) {
        self.reappear_timer = 400;
        self.actor.scale = Vec3::splat(0.001);
        self.drawn = false;
    }

    /// `EnFish_JabuCutscene_Enter`: the first dropped fish into a cutscene becomes the cutscene's,
    /// drawn on its side 600 up, always updating, in the air.
    fn jabu_cutscene_enter(&mut self, play: &mut PlayState) {
        let me = play.cur_actor;
        let s = statics(play);
        if s.jabu_cutscene_fish.is_none() {
            s.jabu_cutscene_fish = me;
            s.jabu_cutscene_offset_y = 10.0;
            s.jabu_cutscene_offset_y_speed = 0.0;
            self.actor.scale = Vec3::splat(0.01);
            self.drawn = true;
            self.actor.shape_rot.x = 0;
            self.actor.shape_rot.y = -0x6410;
            self.actor.shape_rot.z = 0x4000;
            self.actor.shape_y_offset = 600.0;
            self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            self.change_anim_in_air();
        }
    }

    /// `EnFish_JabuCutscene_Clear`.
    fn jabu_cutscene_clear(play: &mut PlayState) {
        let s = statics(play);
        s.jabu_cutscene_fish = None;
        s.jabu_cutscene_offset_y = 0.0;
        s.jabu_cutscene_offset_y_speed = 0.0;
    }

    /// `EnFish_UpdateBobbingUpDown`.
    fn update_bobbing_up_down(&mut self) {
        self.actor.shape_y_offset += (sin_s(self.phase1) * 10.0) + (sin_s(self.phase2) * 5.0);
        self.actor.shape_y_offset = self.actor.shape_y_offset.clamp(-200.0, 200.0);
    }

    fn player_pos(play: &PlayState) -> Vec3 {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO)
    }

    /// `EnFish_IsInRangeForCatch`: within 32 of Link, and within 20 across of him moved 16 its
    /// way.
    fn is_in_range_for_catch(&self, play: &PlayState) -> bool {
        if self.actor.xz_dist_to_player < 32.0 {
            let p = Self::player_pos(play);
            let yaw = self.actor.yaw_towards_player.wrapping_add(i16::MIN);
            let player_pos = Vec3::new((sin_s(yaw) * 16.0) + p.x, p.y, (cos_s(yaw) * 16.0) + p.z);
            if dist_xz_sq(player_pos, self.actor.world_pos) <= 20.0 * 20.0 {
                return true;
            }
        }
        false
    }

    /// `EnFish_IsCloseToPlayer`.
    fn is_close_to_player(&self) -> bool {
        self.actor.xz_dist_to_player < 60.0
    }

    /// `actor.child == &actor`: `Obj_Mure` has it follow Link.
    fn child_is_self(&self, play: &PlayState) -> bool {
        self.actor.child.is_some() && self.actor.child == play.cur_actor
    }

    /// The position of the fish it follows (`actor.child`, not itself), if any.
    fn other_child_pos(&self, play: &PlayState) -> Option<Vec3> {
        let c = self.actor.child.filter(|&c| Some(c) != play.cur_actor)?;
        play.actors.actor(c).map(|a| a.world_pos)
    }

    /// `this->actor.child != NULL && this->actor.child->update == NULL && child != &actor`: a
    /// fish followed that's gone is forgotten.
    fn forget_gone_child(&mut self, play: &PlayState) {
        if let Some(c) = self.actor.child
            && Some(c) != play.cur_actor
            && play.actors.actor(c).is_none_or(|a| a.killed)
        {
            self.actor.child = None;
        }
    }

    /// `EnFish_Normal_SetupStay`.
    fn normal_setup_stay(&mut self, play: &mut PlayState) {
        self.actor.gravity = 0.0;
        self.actor.min_velocity_y = 0.0;
        self.timer = play.rand.s16_offset(5, 35);
        self.upd_bg_check_info_flags = 0;
        self.change_anim_in_water();
        self.action = Action::NormalStay;
    }

    /// `EnFish_Normal_Stay`: slowing to a stop; then swimming, following Link or fleeing him.
    fn normal_stay(&mut self, play: &mut PlayState) {
        self.update_bobbing_up_down();
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 0.05, 0.3, 0.0);
        self.skel.play_speed = ((self.actor.speed_xz * 1.4) + 0.8).min(2.0);
        self.skel.update();
        self.actor.shape_rot.y = self.actor.world_rot.y;
        if self.timer <= 0 {
            self.normal_setup_swim(play);
        } else if self.child_is_self(play) {
            self.normal_setup_follow_player(play);
        } else if self.is_close_to_player() {
            self.normal_setup_flee_player(play);
        }
    }

    /// `EnFish_Normal_SetupSwim`.
    fn normal_setup_swim(&mut self, play: &mut PlayState) {
        self.actor.gravity = 0.0;
        self.actor.min_velocity_y = 0.0;
        self.timer = play.rand.s16_offset(15, 45);
        self.upd_bg_check_info_flags = 0;
        self.change_anim_in_water();
        self.action = Action::NormalSwim;
    }

    /// `EnFish_Normal_Swim`: at 1.8, home past 80 or in its last frames, else after its fish.
    fn normal_swim(&mut self, play: &mut PlayState) {
        self.update_bobbing_up_down();
        smooth_step_to_f(&mut self.actor.speed_xz, 1.8, 0.08, 0.4, 0.0);
        if dist_xz_sq(self.actor.world_pos, self.actor.home_pos) > 80.0 * 80.0 || self.timer < 4 {
            // Move towards home.
            step_to_angle_s(&mut self.actor.world_rot.y, vec3f_yaw(self.actor.world_pos, self.actor.home_pos), 0xBB8);
        } else if let Some(child_pos) = self.other_child_pos(play) {
            // Follow another fish.
            step_to_angle_s(&mut self.actor.world_rot.y, vec3f_yaw(self.actor.world_pos, child_pos), 0xBB8);
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
        self.skel.play_speed = ((self.actor.speed_xz * 1.5) + 0.8).min(4.0);
        self.skel.update();
        if self.timer <= 0 {
            self.normal_setup_stay(play);
        } else if self.child_is_self(play) {
            self.normal_setup_follow_player(play);
        } else if self.is_close_to_player() {
            self.normal_setup_flee_player(play);
        }
    }

    /// `EnFish_Normal_SetupFleePlayer`.
    fn normal_setup_flee_player(&mut self, play: &mut PlayState) {
        self.actor.gravity = 0.0;
        self.actor.min_velocity_y = 0.0;
        self.timer = play.rand.s16_offset(10, 40);
        self.upd_bg_check_info_flags = 0;
        self.change_anim_in_water();
        self.action = Action::NormalFleePlayer;
    }

    /// `EnFish_Normal_FleePlayer`: much faster, home past 160, else after its fish, else away
    /// from Link (swerving 0x2000 one way with `state.frames`' bits 4 and 5 both set, the other
    /// with bit 5 alone), until he's not close or its time's up; or following Link.
    fn normal_flee_player(&mut self, play: &mut PlayState) {
        self.update_bobbing_up_down();
        let is_close_to_player = self.is_close_to_player();
        smooth_step_to_f(&mut self.actor.speed_xz, 4.2, 0.08, 1.4, 0.0);
        if dist_xz_sq(self.actor.world_pos, self.actor.home_pos) > 160.0 * 160.0 {
            // Too far from home.
            let yaw = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
            step_to_angle_s(&mut self.actor.world_rot.y, yaw, 0xBB8);
        } else if let Some(child_pos) = self.other_child_pos(play) {
            // Follow another fish.
            let yaw = vec3f_yaw(self.actor.world_pos, child_pos);
            step_to_angle_s(&mut self.actor.world_rot.y, yaw, 0x7D0);
        } else if is_close_to_player {
            let frames = play.state_frames as i16;
            let mut yaw = self.actor.yaw_towards_player.wrapping_add(i16::MIN);
            if frames & 0x10 != 0 {
                if frames & 0x20 != 0 {
                    yaw = yaw.wrapping_add(0x2000);
                }
            } else if frames & 0x20 != 0 {
                yaw = yaw.wrapping_sub(0x2000);
            }
            step_to_angle_s(&mut self.actor.world_rot.y, yaw, 0x7D0);
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
        self.skel.play_speed = ((self.actor.speed_xz * 1.5) + 0.8).min(4.0);
        self.skel.update();
        if self.timer <= 0 || !is_close_to_player {
            self.normal_setup_stay(play);
        } else if self.child_is_self(play) {
            self.normal_setup_follow_player(play);
        }
    }

    /// `EnFish_Normal_SetupFollowPlayer`.
    fn normal_setup_follow_player(&mut self, play: &mut PlayState) {
        self.actor.gravity = 0.0;
        self.actor.min_velocity_y = 0.0;
        self.change_anim_in_water();
        self.timer = play.rand.s16_offset(10, 40);
        self.upd_bg_check_info_flags = 0;
        self.action = Action::NormalFollowPlayer;
    }

    /// `EnFish_Normal_FollowPlayer`: home past 80, else to 20 beside Link (0x9000 off his yaw
    /// towards it with `state.frames`' bit 6, else 0x7000).
    fn normal_follow_player(&mut self, play: &mut PlayState) {
        self.update_bobbing_up_down();
        smooth_step_to_f(&mut self.actor.speed_xz, 1.8, 0.1, 0.5, 0.0);
        if dist_xz_sq(self.actor.world_pos, self.actor.home_pos) > 80.0 * 80.0 {
            // Too far from home.
            let yaw = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
            step_to_angle_s(&mut self.actor.world_rot.y, yaw, 0xBB8);
        } else {
            // Follow the player.
            let angle = if (play.state_frames as i16) & 0x40 != 0 { self.actor.yaw_towards_player.wrapping_add(0x9000u16 as i16) } else { self.actor.yaw_towards_player.wrapping_add(0x7000) };
            // Add a slight offset approximately towards the fish to the player position.
            let p = Self::player_pos(play);
            let player_pos = Vec3::new((sin_s(angle) * 20.0) + p.x, p.y, (cos_s(angle) * 20.0) + p.z);
            let yaw = vec3f_yaw(self.actor.world_pos, player_pos);
            step_to_angle_s(&mut self.actor.world_rot.y, yaw, 0xBB8);
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
        self.skel.play_speed = ((self.actor.speed_xz * 1.5) + 0.8).min(4.0);
        self.skel.update();
        if self.timer <= 0 {
            self.normal_setup_stay(play);
        }
    }

    /// `EnFish_Dropped_SetupFall`: gravity -1 to -10, in the air, walls and floor checked, 300
    /// frames.
    fn dropped_setup_fall(&mut self) {
        self.actor.gravity = -1.0;
        self.actor.min_velocity_y = -10.0;
        self.actor.shape_y_offset = 0.0;
        self.change_anim_in_air();
        self.upd_bg_check_info_flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2;
        self.action = Action::DroppedFall;
        self.timer = 300;
    }

    /// `EnFish_Dropped_Fall`: slowing, tipping on its side (x to 0x4000, z to -0x4000, 0x64 a
    /// frame); on the ground flapping 400 frames, in water swimming off; its time up and below
    /// the world (`BGCHECK_Y_MIN + 10`), gone.
    fn dropped_fall(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 0.1, 0.1, 0.0);
        step_to_angle_s(&mut self.actor.world_rot.x, 0x4000, 0x64);
        step_to_angle_s(&mut self.actor.world_rot.z, -0x4000, 0x64);
        self.actor.shape_rot.x = self.actor.world_rot.x;
        self.actor.shape_rot.y = self.actor.world_rot.y;
        self.actor.shape_rot.z = self.actor.world_rot.z;
        self.skel.update();
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.timer = 400;
            self.dropped_setup_flap_on_ground(play);
        } else if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 {
            self.dropped_setup_swim();
        } else if self.timer <= 0 && self.ty() == EN_FISH_TYPE_DROPPED && self.actor.floor_height < BGCHECK_Y_MIN + 10.0 {
            log::warn!("En_Fish: through the BG? Actor_delete (../z_en_sakana.c 822)");
            self.actor.kill();
        }
    }

    /// `EnFish_Dropped_SetupFlapOnGround`: a hop (one time in ten 2.5 to 5.5 up, one in ten 0.2
    /// to 1.4, else none), `NA_SE_EV_FISH_LEAP` with a hop or one time in five without (while
    /// drawn).
    fn dropped_setup_flap_on_ground(&mut self, play: &mut PlayState) {
        self.actor.gravity = -1.0;
        self.actor.min_velocity_y = -10.0;
        let rng = play.rand.zero_one();
        let play_leap_sfx = if rng < 0.1 {
            self.actor.velocity.y = (play.rand.zero_one() * 3.0) + 2.5;
            true
        } else if rng < 0.2 {
            self.actor.velocity.y = (play.rand.zero_one() * 1.2) + 0.2;
            true
        } else {
            self.actor.velocity.y = 0.0;
            play.rand.zero_one() < 0.2
        };
        self.actor.shape_y_offset = 300.0;
        self.change_anim_in_air();
        self.action = Action::DroppedFlapOnGround;
        self.upd_bg_check_info_flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2;
        if play_leap_sfx && self.drawn {
            audio_play_actor_sfx2(play, NA_SE_EV_FISH_LEAP);
        }
    }

    /// `EnFish_Dropped_FlapOnGround`: drifting, twisting on x by `state.frames` (0, 0x800, 0x1000
    /// or 0x1800 × 0.3 from bits 2 and 6, negated with bit 2) and back to z 0x4000, turning at
    /// random; its time up, gone; its last 60 frames blinking (drawn with `state.frames`' bit 2);
    /// before, in water swimming off, on the ground another hop.
    fn dropped_flap_on_ground(&mut self, play: &mut PlayState) {
        let frames = play.state_frames as i16;
        let target = play.rand.zero_one() * 0.2;
        smooth_step_to_f(&mut self.actor.speed_xz, target, 0.1, 0.1, 0.0);
        let bits = (((frames as i32) >> 5) & 2) | (((frames as i32) >> 2) & 1);
        let mut target_rot_x = (((bits * 0x800) as i16) as f32 * 0.3) as i32 as i16;
        if frames & 4 != 0 {
            target_rot_x = target_rot_x.wrapping_mul(-1);
        }
        step_to_angle_s(&mut self.actor.world_rot.x, target_rot_x, 0xFA0);
        step_to_angle_s(&mut self.actor.world_rot.z, 0x4000, 0x3E8);
        let wave = (sin_s(self.phase1) * 2000.0) + (sin_s(self.phase2) * 1000.0);
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add((wave * play.rand.zero_one()) as i32 as i16);
        self.actor.shape_rot = self.actor.world_rot;
        self.skel.update();
        if self.timer <= 0 {
            self.actor.kill();
            return;
        }
        if self.timer <= 60 {
            self.drawn = frames & 4 != 0;
        } else if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 {
            self.dropped_setup_swim();
        } else if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.dropped_setup_flap_on_ground(play);
        }
    }

    /// `EnFish_Dropped_SetupSwim`: home where it went in, no gravity, always updating, 200
    /// frames, in the water.
    fn dropped_setup_swim(&mut self) {
        self.actor.home_pos = self.actor.world_pos;
        self.actor.gravity = 0.0;
        self.actor.min_velocity_y = 0.0;
        self.actor.shape_y_offset = 0.0;
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.timer = 200;
        self.change_anim_in_water();
        self.action = Action::DroppedSwim;
        self.upd_bg_check_info_flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2;
    }

    /// `EnFish_Dropped_Swim`: at 2.8, turned back towards home (`home.rot.y`) at half speed off
    /// a wall or out of the water; upright; sinking to 4 under home on the ground, else 10; its
    /// last 100 frames shrinking (0.982 a frame); gone.
    fn dropped_swim(&mut self) {
        smooth_step_to_f(&mut self.actor.speed_xz, 2.8, 0.1, 0.4, 0.0);
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 || self.actor.bg_check_flags & BGCHECKFLAG_WATER == 0 {
            self.actor.home_rot.y = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
            self.actor.speed_xz *= 0.5;
        }
        step_to_angle_s(&mut self.actor.world_rot.x, 0, 0x5DC);
        step_to_angle_s(&mut self.actor.world_rot.y, self.actor.home_rot.y, 0xBB8);
        step_to_angle_s(&mut self.actor.world_rot.z, 0, 0x3E8);
        self.actor.shape_rot = self.actor.world_rot;
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y - 4.0, 2.0);
        } else {
            step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y - 10.0, 2.0);
        }
        if self.timer < 100 {
            self.actor.scale = Vec3::splat(self.actor.scale.x * 0.982);
        }
        self.skel.play_speed = ((self.actor.speed_xz * 1.5) + 1.0).min(4.0);
        self.skel.update();
        if self.timer <= 0 {
            self.actor.kill();
        }
    }

    /// `EnFish_TinyPond_SetupSwim`.
    fn tiny_pond_setup_swim(&mut self, play: &mut PlayState) {
        self.actor.gravity = 0.0;
        self.actor.min_velocity_y = 0.0;
        self.timer = play.rand.s16_offset(5, 35);
        self.upd_bg_check_info_flags = 0;
        self.change_anim_in_water();
        self.action = Action::TinyPondSwim;
    }

    /// `EnFish_TinyPond_Swim`: still, or darting in its last frames (12 with Link within 60,
    /// else 4); home at 0xC8 past 15 from it, else in its last 4 frames turned to
    /// `gameplayFrames × 0x80`, the animation faster while turning; its time up, another
    /// `Rand_S16Offset(5, 80)`.
    fn tiny_pond_swim(&mut self, play: &mut PlayState) {
        let frames = play.gameplay_frames;
        let speed_params = if self.actor.xz_dist_to_player < 60.0 {
            if self.timer < 12 { MOVE_SPEED_PARAMS } else { STAY_SPEED_PARAMS }
        } else if self.timer < 4 {
            MOVE_SPEED_PARAMS
        } else {
            STAY_SPEED_PARAMS
        };
        self.update_bobbing_up_down();
        smooth_step_to_f(&mut self.actor.speed_xz, speed_params.0, speed_params.1, speed_params.2, 0.0);
        let mut anim_play_speed_boost = 0.0;
        if dist_xz_sq(self.actor.world_pos, self.actor.home_pos) > 15.0 * 15.0 {
            if !scaled_step_to_s(&mut self.actor.world_rot.y, vec3f_yaw(self.actor.world_pos, self.actor.home_pos), 0xC8) {
                anim_play_speed_boost = 0.5;
            }
        } else if self.timer < 4 && !scaled_step_to_s(&mut self.actor.world_rot.y, frames.wrapping_mul(0x80) as i16, 0x64) {
            anim_play_speed_boost = 0.5;
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
        // @bug (game): CLAMP(x, 1.5f, 0.5), the bounds swapped: 1.5 under 1.5, else 0.5.
        let x = (self.actor.speed_xz * 1.2) + 0.2 + anim_play_speed_boost;
        self.skel.play_speed = if x < 1.5 { 1.5 } else { 0.5 };
        self.skel.update();
        if self.timer <= 0 {
            self.timer = play.rand.s16_offset(5, 80);
        }
    }

    /// `EnFish_JabuCutscene_CueFlapOnGround`: the hop (`sJabuCutsceneOffsetY`) falling 0.4 a
    /// frame to 1; down, one time in ten another (2 to 5, `NA_SE_EV_FISH_LEAP`).
    fn jabu_cutscene_cue_flap_on_ground(&mut self, play: &mut PlayState) {
        let sp24 = sin_s(self.phase1);
        let sp20 = sin_s(self.phase2);
        let s = statics(play);
        s.jabu_cutscene_offset_y += s.jabu_cutscene_offset_y_speed;
        if s.jabu_cutscene_offset_y <= 1.0 {
            s.jabu_cutscene_offset_y = 1.0;
            if play.rand.zero_one() < 0.1 {
                let speed = (play.rand.zero_one() * 3.0) + 2.0;
                statics(play).jabu_cutscene_offset_y_speed = speed;
                audio_play_actor_sfx2(play, NA_SE_EV_FISH_LEAP);
            } else {
                statics(play).jabu_cutscene_offset_y_speed = 0.0;
            }
        } else {
            statics(play).jabu_cutscene_offset_y_speed -= 0.4;
        }
        self.skel.play_speed = ((sp24 + sp20) * 0.5) + 2.0;
        self.skel.update();
    }

    /// `EnFish_JabuCutscene_CueFly`: turning (x -0x1F4, z +0x64 a frame), the hop eased to 0.
    fn jabu_cutscene_cue_fly(&mut self, play: &mut PlayState) {
        let sp28 = sin_s(self.phase1);
        let sp24 = sin_s(self.phase2);
        self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_sub(0x1F4);
        self.actor.shape_rot.z = self.actor.shape_rot.z.wrapping_add(0x64);
        step_to_f(&mut statics(play).jabu_cutscene_offset_y, 0.0, 1.0);
        self.skel.play_speed = ((sp28 + sp24) * 0.5) + 2.0;
        self.skel.update();
    }

    /// `EnFish_JabuCutscene_Update`: the cue on channel 1 (gone: the fish is too); its action,
    /// then from the cue's start to its end by the cutscene's frame, the hop on top; the floor
    /// under it (`BgCheck_EntityRaycastDown4`).
    fn jabu_cutscene_update(&mut self, play: &mut PlayState) {
        let Some(cue) = play.cs_ctx.npc_actions[1] else {
            // schema: printed
            log::warn!("En_Fish: Warning: the demo ended without dousa 3 (gone) being called (../z_en_sakana.c 1169)(arg_data {:#06x})", self.actor.params);
            Self::jabu_cutscene_clear(play);
            self.actor.kill();
            return;
        };
        self.phase1 = self.phase1.wrapping_add(0x111);
        self.phase2 = self.phase2.wrapping_add(0x500);
        match cue.action {
            1 => self.jabu_cutscene_cue_flap_on_ground(play),
            2 => self.jabu_cutscene_cue_fly(play),
            3 => {
                log::debug!("En_Fish: demo fish gone");
                Self::jabu_cutscene_clear(play);
                self.actor.kill();
                return;
            }
            // schema: printed
            _ => log::warn!("En_Fish: invalid demo action (../z_en_sakana.c 1200)(arg_data {:#06x})", self.actor.params),
        }
        let start = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
        let end = Vec3::new(cue.end_pos.x as f32, cue.end_pos.y as f32, cue.end_pos.z as f32);
        let factor = oot_game::env::lerp_weight(cue.end_frame, cue.start_frame, play.cs_ctx.frames);
        let offset_y = statics(play).jabu_cutscene_offset_y;
        self.actor.world_pos.x = ((end.x - start.x) * factor) + start.x;
        self.actor.world_pos.y = ((end.y - start.y) * factor) + start.y + offset_y;
        self.actor.world_pos.z = ((end.z - start.z) * factor) + start.z;
        let (y, poly) = play.col.entity_raycast_down(self.actor.world_pos);
        self.actor.floor_height = y;
        self.actor.floor_poly = poly;
    }

    /// The action (`actionFunc`).
    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::NormalStay => self.normal_stay(play),
            Action::NormalSwim => self.normal_swim(play),
            Action::NormalFleePlayer => self.normal_flee_player(play),
            Action::NormalFollowPlayer => self.normal_follow_player(play),
            Action::DroppedFall => self.dropped_fall(play),
            Action::DroppedFlapOnGround => self.dropped_flap_on_ground(play),
            Action::DroppedSwim => self.dropped_swim(),
            Action::TinyPondSwim => self.tiny_pond_swim(play),
        }
    }

    /// `EnFish_UpdateImpl`: its timer down, the phases on, a gone fish forgotten; the action;
    /// then (still here) moving (`Actor_MoveXZGravity`), the walls, floor and water (its flags),
    /// pushing with Link within 70, the focus on the shape's offset; taken by Player (its
    /// `parent`): the dropped fish gone, any other away; else in reach, offered (`GI_MAX`).
    fn update_impl(&mut self, play: &mut PlayState) {
        if self.timer > 0 {
            self.timer -= 1;
        }
        self.phase1 = self.phase1.wrapping_add(0x111);
        self.phase2 = self.phase2.wrapping_add(0x500);
        self.forget_gone_child(play);
        self.run_action(play);
        if self.actor.killed {
            return;
        }
        self.actor.move_forward();
        if self.upd_bg_check_info_flags != 0 {
            self.actor.update_bg_check_info(&play.col, 17.5, 4.0, 0.0, self.upd_bg_check_info_flags);
        }
        if self.actor.xz_dist_to_player < 70.0 {
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
        let y = self.actor.shape_y_offset * 0.01;
        self.actor.set_focus(y);
        if oot_game::get_item::actor_has_parent(&self.actor) {
            self.actor.parent = None;
            if self.ty() == EN_FISH_TYPE_DROPPED {
                self.actor.kill();
            } else {
                self.disappear();
            }
        } else if self.is_in_range_for_catch(play) {
            oot_game::get_item::offer_get_item_range(play, &self.actor, oot_game::item::GI_MAX, 80.0, 20.0);
        }
    }

    /// `EnFish_UpdateWaitReappear`: the tiny pond's fish gone for good; else a gone fish
    /// forgotten, the action and the move (the timers held); drawn again with 20 left, growing
    /// 0.001 a frame, at 0 its full 0.01.
    fn update_wait_reappear(&mut self, play: &mut PlayState) {
        if self.ty() == EN_FISH_TYPE_TINY_POND {
            self.actor.kill();
            return;
        }
        self.forget_gone_child(play);
        // Note the action keeps running and the fish keeps moving while it's away (not drawn).
        self.run_action(play);
        if self.actor.killed {
            return;
        }
        self.actor.move_forward();
        if self.reappear_timer == 20 {
            self.drawn = true;
        } else if self.reappear_timer == 0 {
            self.actor.scale = Vec3::splat(0.01);
        } else if self.reappear_timer < 20 {
            let scale = (self.actor.scale.x + 0.001).min(0.01);
            self.actor.scale = Vec3::splat(scale);
        }
    }
}

impl ActorImpl for EnFish {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnFish_Update`: a dropped fish while a cutscene runs with a cue on channel 1 becomes the
    /// cutscene's (if it has none); the cutscene's fish follows its cue, a fish away waits to come
    /// back, any other `EnFish_UpdateImpl`.
    fn update(&mut self, play: &mut PlayState) {
        let me = play.cur_actor;
        // @bug (game) (the C's): no scene check, so any cutscene with a cue on channel 1 will do.
        if statics(play).jabu_cutscene_fish.is_none() && self.ty() == EN_FISH_TYPE_DROPPED && play.cs_ctx.state != CS_STATE_IDLE && play.cs_ctx.npc_actions[1].is_some() {
            self.jabu_cutscene_enter(play);
        }
        let jabu = statics(play).jabu_cutscene_fish;
        if jabu.is_some() && jabu == me {
            self.jabu_cutscene_update(play);
        } else if self.reappear_timer > 0 {
            self.reappear_timer -= 1;
            self.update_wait_reappear(play);
        } else {
            self.update_impl(play);
        }
    }

    /// `EnFish_Destroy`: the sphere (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `EnFish_Draw`'s `Collider_UpdateSpheres(0, &collider)` after the skeleton: the sphere at
    /// the actor's model matrix.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.drawn {
            let m = actor_draw_matrix(&RenderState::of(&self.actor));
            self.collider.update_spheres(0, &m);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.drawn as u32];
        // The shape's shadow: none while draw is NULL (Actor_Draw isn't called).
        if !self.drawn {
            rs.shadow = None;
        }
        rs
    }

    /// `EnFish_Draw`: `Gfx_SetupDL_25Opa`, the skeleton (`SkelAnime_DrawFlexOpa`, no limb
    /// callbacks).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(&drawn), Some(joints), Some(skeleton)) = (rs.switches.first(), &rs.joints, &self.skeleton) else { return };
        if drawn == 0 {
            return;
        }
        let bones = skeleton.pose(joints);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: actor_draw_matrix(rs), bones, params: Default::default() });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::JntSph(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
