//! `En_Cs` (`ovl_En_Cs/z_en_cs.c`): the graveyard kid, the boy with the stick who walks the
//! graveyard by day (`spot02_scene`, the child's day layer) and buys the Spooky Mask.
//!
//! By night he isn't there (`EnCs_Init`: `!IS_DAY`, killed). By day he **walks his path**
//! (`EnCs_Walk`, `EnCs_HandleWalking`: the scene path params bits 0 to 7 pick, round and round):
//! he heads for his waypoint, turning up to 2500 a frame, and takes the next when within 10.44 of
//! it; his speed is 1, but 0 on his walk's frames 0, 8 to 15 and 23 to 29 (he steps, stops,
//! steps). Each time his walk animation ends he walks on, or one time in five (one in ten once
//! he has the mask) he **waits** (`EnCs_Wait`) for one play of his stick grab (with the mask,
//! half the time his stick swing instead), then walks on. His steps sound on his animations'
//! frames (`NA_SE_EV_CHIBI_WALK`).
//!
//! He **talks** (`EnCs_HandleTalking`) within 100 while on screen: 0x2022 (by default), 0x2023
//! with the Spooky Mask on (a choice; he swings his stick and the chime sounds), 0x2028 once
//! he has bought it (`ITEMGETINF_3A`). At 0x2023's choice: yes, 0x2026 (then, read, the
//! **trade**: Link's mask off, `ITEM_SOLD_OUT` in its slot, `ITEMGETINF_3A`, 30 rupees, and
//! 0x2027); no, 0x2024. Talking he stops and turns to Link, head and torso following
//! (`EnCs_Talk`, `Npc_TrackPoint` with the whole body), and walks on after.
//!
//! He blinks (70 frames open, one half, one closed). Drawn as `EnCs_Draw` does: the eyes on
//! segment 8, his torso (limb 8) and head (limb 15) turned while talking
//! (`EnCs_OverrideLimbDraw`), the focus 500 along and 800 up his head, and once he has bought it
//! (`ITEMGETINF_3A`) the Spooky Mask on his face: `gLinkChildSpookyMaskDL` (from
//! `object_link_child`) placed by the matrix `EnCs_PostLimbDraw` keeps (his head's, 200 down it,
//! turned 100 degrees about z); from meshes baked per eye (docs/adr/0012-actor-bakes.md). The
//! circle shadow (19) is the shape's (`ActorShape_Init`'s, drawn by `Actor_Draw`).
//!
//! Not ported here: the masks themselves (Player's `currentMask` is always `PLAYER_MASK_NONE`,
//! so 0x2023 and the trade aren't reached in play; BACKLOG #39), and the mask reactions
//! (`MaskReaction_GetTextId(play, MASK_REACTION_SET_GRAVEYARD_KID)`: 0). Both branches are
//! ported ([`EnCs::text_id_for`]).
//!
//! Params: `path` bits 0 to 7 (the scene path he walks: spot02's 0).
//! Flags: reads and sets ITEMGETINF_3A (the Spooky Mask sold to him).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::smooth_step_to_s;
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR;
use oot_game::collision_check::*;
use oot_game::item::{ITEM_SOLD_OUT, item_give, rupees_change_by};
use oot_game::item_get_inf;
use oot_game::message::{TEXT_STATE_CHOICE, TEXT_STATE_DONE};
use oot_game::npc::{NpcTrack, offer_talk, process_talk_request, track_point};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::path::rad_to_binang_trunc;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_CS` (`actor_table.h`: 0x016C).
pub const ACTOR_EN_CS: i16 = 0x016C;
/// `OBJECT_CS` (`object_table.h`: 0x0145).
pub const OBJECT: &str = "object_cs";
/// `OBJECT_LINK_CHILD` (`object_table.h`: 0x0015): the Spooky Mask's.
const OBJECT_LINK_CHILD: i16 = 0x0015;

/// `En_Cs_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_CS, name: "En_Cs", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 8)`: `path`.
pub const PATH: Field = Field::params("path", 0, 8, Kind::PathIndex, "the scene path he walks");

/// En_Cs's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PATH],
    flags: &[FlagUse::new(item_get_inf!(ITEMGETINF_3A)).reads().sets().note("the Spooky Mask sold to him")],
    todo: false,
};

/// `ITEMGETINF_3A` (`save.h`: 0x3A): the Spooky Mask sold to the graveyard kid.
pub const ITEMGETINF_3A: u16 = 0x3A;
/// `PLAYER_MASK_SPOOKY` (`player.h`: 3).
pub const PLAYER_MASK_SPOOKY: u8 = 3;
/// `NA_SE_EV_CHIBI_WALK` (`sfx/environmentbank_table.h`: 0x28EF): his steps.
pub const NA_SE_EV_CHIBI_WALK: u16 = 0x28EF;

/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `NPC_TRACKING_FULL_BODY` (`actor.h`).
const NPC_TRACKING_FULL_BODY: i16 = 4;
/// `SCREEN_WIDTH`, `SCREEN_HEIGHT`.
const SCREEN_WIDTH: i16 = 320;
const SCREEN_HEIGHT: i16 = 240;

/// His talk's reach (`Actor_OfferTalk(.., 100.0f)`).
pub const TALK_RANGE: f32 = 100.0;
/// `EnCs_HandleWalking`: a waypoint is reached within this.
pub const WAYPOINT_REACHED: f32 = 10.44;
/// `EnCs_HandleWalking`'s `Math_SmoothStepToS(.., 1, 2500, 0)`: his turn a frame at most.
pub const TURN_STEP: i16 = 2500;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 18, height: 63, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit2`: immovable. (`sDamageTable` is all zeroes; `AC_NONE`.)
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

// `EnCsAnimation`.
/// `ENCS_ANIM_0`: `gGraveyardKidWalkAnim`.
pub const ENCS_ANIM_0: i32 = 0;
/// `ENCS_ANIM_1`: `gGraveyardKidSwingStickUpAnim`.
pub const ENCS_ANIM_1: i32 = 1;
/// `ENCS_ANIM_2`: `gGraveyardKidGrabStickTwoHandsAnim`.
pub const ENCS_ANIM_2: i32 = 2;
/// `ENCS_ANIM_3`: `gGraveyardKidIdleAnim`.
pub const ENCS_ANIM_3: i32 = 3;

/// `sAnimationInfo`: (animation, `frameCount` (the play speed), mode, morph frames).
pub const ANIMATION_INFO: [(&str, f32, u8, f32); 4] = [
    ("gGraveyardKidWalkAnim", 1.0, ANIMMODE_ONCE, -10.0),
    ("gGraveyardKidSwingStickUpAnim", 1.0, ANIMMODE_ONCE, -10.0),
    ("gGraveyardKidGrabStickTwoHandsAnim", 1.0, ANIMMODE_ONCE, -10.0),
    ("gGraveyardKidIdleAnim", 1.0, ANIMMODE_ONCE, -10.0),
];

const SKEL: &str = "gGraveyardKidSkel";
/// `eyeTextures`, on segment 8.
const EYE_TEXTURES: [&str; 3] = ["gGraveyardKidEyesOpenTex", "gGraveyardKidEyesHalfTex", "gGraveyardKidEyesClosedTex"];
/// `eyeBlinkFrames` (`EnCs_Update`): how long each eye stays.
pub const EYE_BLINK_FRAMES: [i32; 3] = [70, 1, 1];
const SEG_EYES: u8 = 0x08;

/// His torso and head (`limbIndex`, 1-based).
const LIMB_TORSO: usize = 8;
const LIMB_HEAD: usize = 15;
/// `D_809E2970`: the focus on his head.
const FOCUS_OFFSET: Vec3 = Vec3::new(500.0, 800.0, 0.0);

/// His mesh with eye `eye`.
pub fn bake_name(eye: usize) -> String {
    format!("En_Cs/eye{eye}")
}

/// `gLinkChildSpookyMaskDL` from its second command: its first loads the matrix the draw puts on
/// segment 13 (`MATRIX_FINALIZE(..) - 7`, the list's `0x0D0001C0`: the mask's own matrix), which
/// the draw's transform stands for.
pub const SPOOKY_MASK_BAKE: &str = "En_Cs/spooky_mask";

/// The meshes `EnCs_Draw` draws: `gGraveyardKidSkel` with each eye, and the Spooky Mask.
pub fn bakes() -> Vec<MeshBake> {
    let mut v: Vec<MeshBake> = EYE_TEXTURES
        .iter()
        .enumerate()
        .map(|(eye, tex)| MeshBake {
            name: bake_name(eye),
            object: OBJECT.into(),
            segments: vec![(SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() })],
            prelude: Vec::new(),
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() },
        })
        .collect();
    v.push(MeshBake {
        name: SPOOKY_MASK_BAKE.into(),
        object: "object_link_child".into(),
        segments: Vec::new(),
        prelude: Vec::new(),
        body: BakeBody::DLists(vec![("object_link_child".into(), "gLinkChildSpookyMaskDL+0x8".into())]),
    });
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnCs_Walk`.
    Walk,
    /// `EnCs_Talk`.
    Talk,
    /// `EnCs_Wait`.
    Wait,
}

pub struct EnCs {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `talkState`: 0 not talking, 1 talking, 2 a text to continue with next frame.
    pub talk_state: i32,
    /// `eyeIndex`, `eyeBlinkTimer`.
    pub eye_index: i32,
    pub eye_blink_timer: i32,
    /// `unk_1EC`: set 0, never read.
    pub unk_1ec: i32,
    /// `path`: the scene path (params bits 0 to 7).
    pub path: i32,
    /// `waypoint`: the point he heads for.
    pub waypoint: i32,
    /// `animLoopCount`: plays of a wait's animation left (never set: always 0).
    pub anim_loop_count: i32,
    /// `walkAngle` (an `f32` holding the yaw to the waypoint), `walkDist`, `walkSpeed`.
    pub walk_angle: f32,
    pub walk_dist: f32,
    pub walk_speed: f32,
    pub current_anim_index: i32,
    /// `interactInfo`: the head and torso tracking (only `EnCs_Talk` runs it).
    pub interact_info: NpcTrack,
    /// `flag`: bit 0 while talking (the head and torso turns drawn).
    pub flag: i32,
    anims: Vec<Anim>,
}

/// Player's `currentMask` (`PLAYER_MASK_NONE` here: masks aren't ported, BACKLOG #39).
fn player_mask(play: &PlayState) -> u8 {
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.current_mask())
}

impl EnCs {
    /// `EnCs_Init`: by night, killed; by day his skeleton, the walk, his path.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.skeleton(OBJECT, SKEL)?;
            let anims = ANIMATION_INFO.iter().map(|(name, ..)| assets.animation(OBJECT, name)).collect::<anyhow::Result<Vec<_>>>()?;
            Ok((skeleton, anims))
        })();
        let (skeleton, anims) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Cs: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnCs {
            // SkelAnime_InitFlex(.., gGraveyardKidSkel, NULL, .., 16).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            action: Action::Walk,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            talk_state: 0,
            eye_index: 0,
            eye_blink_timer: 0,
            unk_1ec: 0,
            path: 0,
            waypoint: 0,
            anim_loop_count: 0,
            walk_angle: 0.0,
            walk_dist: 0.0,
            walk_speed: 0.0,
            current_anim_index: 0,
            interact_info: NpcTrack::default(),
            flag: 0,
            anims,
            actor,
        };
        if !play.save.is_day() {
            this.actor.kill();
            return Box::new(this);
        }
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 19); Actor_Init's scale 0.01.
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 19.0);
        this.actor.scale = Vec3::splat(0.01);
        this.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        this.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        // Animation_Change(sAnimationInfo[ENCS_ANIM_0]), morphing (undone by EnCs_ChangeAnim below).
        let (_, speed, mode, morph) = ANIMATION_INFO[ENCS_ANIM_0 as usize];
        let a0 = this.anims[ENCS_ANIM_0 as usize].clone();
        let last = a0.last_frame();
        this.skel.change(a0, speed, 0.0, last, mode, morph);
        this.actor.target_mode = ATTENTION_RANGE_6;
        this.path = PATH.get(this.actor.params) as i32;
        this.unk_1ec = 0;
        this.talk_state = 0;
        this.current_anim_index = -1;
        this.actor.gravity = -1.0;
        this.change_anim(ENCS_ANIM_0);
        this.action = Action::Walk;
        this.walk_speed = 1.0;
        Box::new(this)
    }

    /// `EnCs_ChangeAnim`: no morph from none or to the same.
    pub fn change_anim(&mut self, index: i32) {
        let (_, frame_count, mode, morph) = ANIMATION_INFO[index as usize];
        let morph_frames = if self.current_anim_index < 0 || index == self.current_anim_index { 0.0 } else { morph };
        let anim = self.anims[index as usize].clone();
        let last = anim.last_frame();
        if frame_count >= 0.0 {
            self.skel.change(anim, frame_count, 0.0, last, mode, morph_frames);
        } else {
            self.skel.change(anim, frame_count, last, 0.0, mode, morph_frames);
        }
        self.current_anim_index = index;
    }

    /// `EnCs_GetTalkState`: the choice (yes 0x2026, no 0x2024) and, 0x2026 read, the trade.
    fn get_talk_state(&mut self, play: &mut PlayState) -> i32 {
        let mut talk_state = 1;
        match play.message_state() {
            TEXT_STATE_CHOICE => {
                if play.message_should_advance() {
                    if play.msg_ctx.choice_index == 0 {
                        self.actor.text_id = 0x2026;
                        self.change_anim(ENCS_ANIM_3);
                        talk_state = 2;
                    } else {
                        self.actor.text_id = 0x2024;
                        self.change_anim(ENCS_ANIM_1);
                        talk_state = 2;
                    }
                }
            }
            TEXT_STATE_DONE => {
                if play.message_should_advance() {
                    if self.actor.text_id == 0x2026 {
                        if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
                            p.unset_mask();
                        }
                        item_give(&mut play.save, Some(&mut play.audio), ITEM_SOLD_OUT);
                        play.save.set_item_get_inf(ITEMGETINF_3A);
                        rupees_change_by(&mut play.save, 30);
                        self.actor.text_id = 0x2027;
                        talk_state = 2;
                    } else {
                        talk_state = 0;
                    }
                }
            }
            _ => {}
        }
        talk_state
    }

    /// `EnCs_GetTextId`'s choice, from what it reads: `ITEMGETINF_3A`, Player's mask and the
    /// mask reaction (`MaskReaction_GetTextId`, 0 for none).
    pub fn text_id_for(itemgetinf_3a: bool, current_mask: u8, mask_reaction: u16) -> u16 {
        let mut text_id = mask_reaction;
        if itemgetinf_3a {
            if text_id == 0 {
                text_id = 0x2028;
            }
        } else if current_mask == PLAYER_MASK_SPOOKY {
            text_id = 0x2023;
        } else if text_id == 0 {
            text_id = 0x2022;
        }
        text_id
    }

    /// `EnCs_GetTextId`.
    pub fn get_text_id(play: &PlayState) -> u16 {
        // MaskReaction_GetTextId(play, MASK_REACTION_SET_GRAVEYARD_KID): no masks, so 0.
        Self::text_id_for(play.save.get_item_get_inf(ITEMGETINF_3A), player_mask(play), 0)
    }

    /// `EnCs_HandleTalking`.
    fn handle_talking(&mut self, play: &mut PlayState) {
        if self.talk_state == 2 {
            play.continue_textbox(self.actor.text_id);
            self.talk_state = 1;
        } else if self.talk_state == 1 {
            self.talk_state = self.get_talk_state(play);
        } else if process_talk_request(&mut self.actor) {
            let t = self.actor.text_id;
            // (The C's `(t == 0x2022) || ((t != 0x2022) && (t != 0x2028))`: anything but 0x2028.)
            if t == 0x2022 || (t != 0x2022 && t != 0x2028) {
                self.change_anim(ENCS_ANIM_3);
            }
            if t == 0x2023 || t == 0x2028 {
                self.change_anim(ENCS_ANIM_1);
            }
            if t == 0x2023 {
                play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
            }
            self.talk_state = 1;
        } else {
            let (x, y) = oot_game::target::actor_screen_pos(play.view_proj, &self.actor);
            if (0..=SCREEN_WIDTH).contains(&x) && (0..=SCREEN_HEIGHT).contains(&y) {
                let a = self.actor.clone();
                if offer_talk(play, &a, TALK_RANGE) {
                    self.actor.text_id = Self::get_text_id(play);
                }
            }
        }
    }

    /// `EnCs_GetwaypointCount` and `EnCs_GetPathPoint`: his path's point `waypoint` and its count.
    fn path_point(play: &PlayState, path: i32, waypoint: i32) -> Option<(Vec3, i32)> {
        let p = play.setup_path_list().get(path as usize)?;
        Some((p.point(waypoint as usize), p.count() as i32))
    }

    /// `EnCs_HandleWalking`: towards the waypoint (the next once within 10.44, round again after
    /// the last), turning up to 2500, at `walkSpeed`, falling.
    fn handle_walking(&mut self, play: &mut PlayState) {
        let Some((mut path_pos, waypoint_count)) = Self::path_point(play, self.path, self.waypoint) else {
            log::error!("En_Cs: no path {} (waypoint {})", self.path, self.waypoint);
            return;
        };
        let mut x_diff = path_pos.x - self.actor.world_pos.x;
        let mut z_diff = path_pos.z - self.actor.world_pos.z;
        self.walk_angle = rad_to_binang_trunc(oot_game::camera::f_atan2f(x_diff, z_diff)) as f32;
        self.walk_dist = (x_diff * x_diff + z_diff * z_diff).sqrt();
        while self.walk_dist <= WAYPOINT_REACHED {
            self.waypoint += 1;
            if self.waypoint < 0 || self.waypoint >= waypoint_count {
                self.waypoint = 0;
            }
            path_pos = Self::path_point(play, self.path, self.waypoint).map(|(p, _)| p).unwrap_or(path_pos);
            x_diff = path_pos.x - self.actor.world_pos.x;
            z_diff = path_pos.z - self.actor.world_pos.z;
            self.walk_angle = rad_to_binang_trunc(oot_game::camera::f_atan2f(x_diff, z_diff)) as f32;
            self.walk_dist = (x_diff * x_diff + z_diff * z_diff).sqrt();
        }
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.walk_angle as i16, 1, TURN_STEP, 0);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.actor.speed_xz = self.walk_speed;
        // Actor_MoveXZGravity.
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
    }

    /// Whether `EnCs_Walk` stands still on his walk's frame `frame` (0, 8 to 15, 23 to 29).
    pub fn stands_on_frame(frame: i32) -> bool {
        (8..16).contains(&frame) || (23..30).contains(&frame) || frame == 0
    }

    /// `EnCs_Walk`.
    fn walk(&mut self, play: &mut PlayState) {
        if self.talk_state != 0 {
            self.action = Action::Talk;
            return;
        }
        if self.skel.update() {
            let mut anim_index = self.current_anim_index;
            if self.talk_state == 0 {
                let has_mask = play.save.get_item_get_inf(ITEMGETINF_3A);
                let rnd = if has_mask { (play.rand.zero_one() * 10.0) as i32 } else { (play.rand.zero_one() * 5.0) as i32 };
                if rnd == 0 {
                    if has_mask {
                        anim_index = (2.0 * play.rand.zero_one()) as i32;
                        anim_index = if anim_index == 0 { ENCS_ANIM_2 } else { ENCS_ANIM_1 };
                    } else {
                        anim_index = ENCS_ANIM_2;
                    }
                    self.action = Action::Wait;
                } else {
                    anim_index = ENCS_ANIM_0;
                }
            }
            self.change_anim(anim_index);
        }
        if self.talk_state == 0 {
            let cur_anim_frame = self.skel.cur_frame as i32;
            self.walk_speed = if Self::stands_on_frame(cur_anim_frame) { 0.0 } else { 1.0 };
            self.handle_walking(play);
        }
    }

    /// `EnCs_Wait`: one play of the animation (`animLoopCount` more, never set), then walking.
    fn wait(&mut self, _play: &mut PlayState) {
        if self.talk_state != 0 {
            self.action = Action::Talk;
            return;
        }
        if self.skel.update() {
            let mut anim_index = self.current_anim_index;
            if self.talk_state == 0 {
                if self.anim_loop_count > 0 {
                    self.anim_loop_count -= 1;
                    anim_index = self.current_anim_index;
                } else {
                    anim_index = ENCS_ANIM_0;
                    self.action = Action::Walk;
                }
            }
            self.change_anim(anim_index);
        }
    }

    /// `EnCs_Talk`: the animation again and again, turned to Link's focus; walking when done.
    fn talk(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            let i = self.current_anim_index;
            self.change_anim(i);
        }
        self.flag |= 1;
        self.interact_info.target = play.player.and_then(|h| play.actors.actor(h)).map_or(self.actor.focus_pos, |p| p.focus_pos);
        let mut t = self.interact_info;
        track_point(play, &mut self.actor, &mut t, 0, NPC_TRACKING_FULL_BODY);
        self.interact_info = t;
        if self.talk_state == 0 {
            self.change_anim(ENCS_ANIM_0);
            self.action = Action::Walk;
            self.flag &= !1;
        }
    }

    /// The limb matrices `EnCs_Draw` draws with (`EnCs_OverrideLimbDraw`): while talking
    /// (`flag` bit 0) the torso (limb 8: x by its yaw, y less its pitch) and the head (limb 15:
    /// x by its yaw, z by its pitch).
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], talking: bool, head: [i16; 2], torso: [i16; 2]) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if talking {
                match limb {
                    LIMB_TORSO => {
                        rot[0] = rot[0].wrapping_add(torso[1]);
                        rot[1] = rot[1].wrapping_sub(torso[0]);
                    }
                    LIMB_HEAD => {
                        rot[0] = rot[0].wrapping_add(head[1]);
                        rot[2] = rot[2].wrapping_add(head[0]);
                    }
                    _ => {}
                }
            }
            Mat4::IDENTITY
        })
    }

    /// `EnCs_PostLimbDraw`'s `spookyMaskMtx` relative to the model: the head's matrix, 200 down
    /// it, turned 100 degrees about z.
    fn spooky_mask_mtx(head: Mat4) -> Mat4 {
        // Matrix_Translate(0, -200, 0), Matrix_RotateY(0), Matrix_RotateX(0), Matrix_RotateZ(100°).
        head * Mat4::from_translation(Vec3::new(0.0, -200.0, 0.0)) * Mat4::from_rotation_z(100f32.to_radians())
    }

    fn pose_angles(&self) -> Vec<i16> {
        let t = &self.interact_info;
        vec![t.head[0], t.head[1], t.torso[0], t.torso[1]]
    }
}

impl ActorImpl for EnCs {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnCs_Update`.
    fn update(&mut self, play: &mut PlayState) {
        let f = self.skel.cur_frame as i32;
        let step = match self.current_anim_index {
            0 => f == 9 || f == 23,
            1 => f == 10 || f == 25,
            2 => f == 20,
            _ => false,
        };
        if step {
            audio_play_actor_sfx2(play, NA_SE_EV_CHIBI_WALK);
        }
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        match self.action {
            Action::Walk => self.walk(play),
            Action::Talk => self.talk(play),
            Action::Wait => self.wait(play),
        }
        self.handle_talking(play);
        self.eye_blink_timer -= 1;
        if self.eye_blink_timer < 0 {
            self.eye_index += 1;
            if self.eye_index >= 3 {
                self.eye_index = 0;
            }
            self.eye_blink_timer = EYE_BLINK_FRAMES[self.eye_index as usize];
        }
    }
    /// `EnCs_PostLimbDraw`, limb 15: the focus.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let a = self.pose_angles();
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, self.flag & 1 != 0, [a[0], a[1]], [a[2], a[3]]);
        if let Some(head) = bones.get(LIMB_HEAD - 1) {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(FOCUS_OFFSET);
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = self.pose_angles();
        rs.switches = vec![self.eye_index.max(0) as u32, (self.flag & 1) as u32];
        rs
    }
    /// `EnCs_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let ([eye, talking], [h0, h1, t0, t1]) = (rs.switches.as_slice(), rs.angles.as_slice()) else { return };
        let bones = Self::pose(&self.skeleton, &joints.rot, *talking != 0, [*h0, *h1], [*t0, *t1]);
        let m = oot_game::play::actor_draw_matrix(rs);
        let head = bones.get(LIMB_HEAD - 1).copied();
        let eye = (*eye as usize).min(EYE_TEXTURES.len() - 1);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(eye))), transform: m, bones, params: Default::default() });
        // The Spooky Mask on his face once he has it (and object_link_child is in).
        if play.save.get_item_get_inf(ITEMGETINF_3A)
            && play.object_ctx.get_index(OBJECT_LINK_CHILD).is_some()
            && let Some(head) = head
        {
            out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(SPOOKY_MASK_BAKE)), m * Self::spooky_mask_mtx(head)));
        }
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
