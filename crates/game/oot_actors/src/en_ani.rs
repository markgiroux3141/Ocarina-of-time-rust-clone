//! `En_Ani` (`ovl_En_Ani/z_en_ani.c`): the man on the roof in Kakariko (placed at (370, 483, 100)
//! in the child's layers and the adult's).
//!
//! He sets a text and offers to talk (within 100) while Link is within 350 and his yaw from the
//! man's facing (`yawTowardsPlayer - shape.rot.y`) is in [-0x3E7, 0x36B0): for a child 0x5050 by
//! day and 0x5051 by night (`func_809B064C`); for an adult 0x5052 until `EVENTCHKINF_2F` (set as
//! Death Mountain erupts, `Efc_Erupc`), then 0x5053, or 0x5054 once he has given his piece of heart
//! (`func_809B07F8`). With that yaw in [-0x36AF, 0), within 150 and not more than 80 below him (on
//! the roof beside him, which slopes from about 470 down to 440 there), it's 0x5055 instead: from
//! the talk he offers a piece of heart (`GI_HEART_PIECE`, within 10000 across and 200 up or down)
//! every frame until Player takes it (`func_809B05F0`, `func_809B0558`), and sets `ITEMGETINF_15`;
//! from then on that place gets 0x5056. Once he has set a text he tracks Link with his head for
//! good (`unk_2A8` bit 0, never cleared; `Actor_TrackPlayer`, the torso's turn zeroed).
//!
//! In a cutscene with a cue on channel 0 he runs the cutscene's states instead (`unk_2AA`): cue 2
//! knocks him over (`gRoofManKnockbackAnim`, no shadow), cue 4 gets him up
//! (`gRoofManGettingUpAfterKnockbackAnim`), and on the script's frame 100 the earthquake sounds
//! (`NA_SE_IT_EARTHQUAKE`).
//!
//! Drawn as `EnAni_Draw` does: `Gfx_SetupDL_37Opa`, the eyes on segment 8 (blinking:
//! `blinkTimer` 60 to 120, its last two frames the closed and half eyes), the head (limb 15)
//! turned by the tracking (`EnAni_OverrideLimbDraw`) and the focus at (800, 500, 0) in it
//! (`EnAni_PostLimbDraw`). From a baked mesh per eye (docs/adr/0012-actor-bakes.md).
//!
//! Not ported here: the mask reactions (`MaskReaction_GetTextId` with
//! `MASK_REACTION_SET_KAKARIKO_ROOF_MAN`: no masks yet, so none). The circle shadow (36) is the
//! shape's (`ActorShape_Init`'s, drawn by `Actor_Draw`; the knockback takes it away).
//!
//! Params: none (Kakariko places him with 0).
//! Flags: reads and sets ITEMGETINF_15 (the piece of heart given); reads EVENTCHKINF_2F (an
//! adult's text).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::smooth_step_to_s;
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, FlagUse};
use oot_game::collision_check::*;
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::GI_HEART_PIECE;
use oot_game::npc::{actor_track_player, offer_talk, process_talk_request, textbox_is_closing};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;
use oot_game::{event_chk_inf, item_get_inf};

/// `ACTOR_EN_ANI` (`actor_table.h`: 0x0167).
pub const ACTOR_EN_ANI: i16 = 0x0167;
/// `OBJECT_ANI` (`object_table.h`: 0x00EC).
pub const OBJECT: &str = "object_ani";

/// `En_Ani_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_ANI, name: "En_Ani", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT, schema: &SCHEMA };

/// En_Ani's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[],
    flags: &[
        FlagUse::new(item_get_inf!(ITEMGETINF_15)).reads().sets().note("his piece of heart given"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_2F)).reads().note("an adult's text"),
    ],
    todo: false,
};

/// `ITEMGETINF_15` (`save.h`: 0x15): the roof man's piece of heart given.
pub const ITEMGETINF_15: u16 = 0x15;
/// `EVENTCHKINF_2F` (`save.h`: 0x2F).
pub const EVENTCHKINF_2F: u16 = 0x2F;
/// `NA_SE_IT_EARTHQUAKE` (`itembank_table.h`: 0x1845).
pub const NA_SE_IT_EARTHQUAKE: u16 = 0x1845;

/// The texts (`func_809B064C`, `func_809B07F8`).
pub const TEXT_CHILD_DAY: u16 = 0x5050;
pub const TEXT_CHILD_NIGHT: u16 = 0x5051;
pub const TEXT_ADULT: u16 = 0x5052;
pub const TEXT_ADULT_2F: u16 = 0x5053;
pub const TEXT_ADULT_GIVEN: u16 = 0x5054;
pub const TEXT_HEART_PIECE: u16 = 0x5055;
pub const TEXT_HEART_PIECE_GIVEN: u16 = 0x5056;

/// The head (1-based), turned by `EnAni_OverrideLimbDraw` and holding the focus.
const HEAD_LIMB: usize = 15;
/// `EnAni_PostLimbDraw`'s `sMultVec`.
const FOCUS_IN_HEAD: Vec3 = Vec3::new(800.0, 500.0, 0.0);

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

/// `EnAni_Draw`'s `eyeTextures`.
const EYE_TEXTURES: [&str; 3] = ["gRoofManEyeOpenTex", "gRoofManEyeHalfTex", "gRoofManEyeClosedTex"];
const SEG_EYES: u8 = 0x08;
const SEG_SETUP: u8 = 0x0C;

/// The skeleton with eye `eye` on segment 8, after `Gfx_SetupDL_37Opa`.
pub fn bake_name(eye: usize) -> String {
    format!("En_Ani/eye{eye}")
}

/// The meshes `EnAni_Draw` draws: `gRoofManSkel` with each eye.
pub fn bakes() -> Vec<MeshBake> {
    (0..EYE_TEXTURES.len())
        .map(|eye| MeshBake {
            name: bake_name(eye),
            object: OBJECT.into(),
            segments: vec![(SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: EYE_TEXTURES[eye].into() }), (SEG_SETUP, BakeSegment::Commands(crate::en_owl::setup_dl_37()))],
            prelude: vec![SEG_SETUP],
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gRoofManSkel".into(), limbs: Vec::new() },
        })
        .collect()
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_809B064C`: a child's idle, offering his texts.
    ChildIdle,
    /// `func_809B07F8`: an adult's idle.
    AdultIdle,
    /// `func_809B04F0`: a child's talk, until the box closes.
    ChildTalk,
    /// `func_809B0524`: an adult's talk.
    AdultTalk,
    /// `func_809B05F0`: the piece of heart's text, offering it.
    HeartPieceText,
    /// `func_809B0558`: offering the piece of heart until Player takes it.
    GiveHeartPiece,
}

struct Anims {
    idle: Anim,
    knockback: Anim,
    getting_up: Anim,
}

pub struct EnAni {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    /// `unk_29C`: the head's turn (`Actor_TrackPlayer`'s head).
    pub unk_29c: [i16; 3],
    /// `unk_2A2`: the torso's (zeroed after each track).
    pub unk_2a2: [i16; 3],
    /// `unk_2A8`: bit 0, a text offered (he tracks Link from then on).
    pub unk_2a8: u16,
    /// `unk_2AA`: the cutscene's state (0 to 4).
    pub unk_2aa: u16,
    pub eye_index: i16,
    pub blink_timer: i16,
    pub action: Action,
    anims: Anims,
}

impl EnAni {
    /// `EnAni_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.skeleton(OBJECT, "gRoofManSkel")?;
            let anims = Anims {
                idle: assets.animation(OBJECT, "gRoofManIdleAnim")?,
                knockback: assets.animation(OBJECT, "gRoofManKnockbackAnim")?,
                getting_up: assets.animation(OBJECT, "gRoofManGettingUpAfterKnockbackAnim")?,
            };
            Ok((skeleton, anims))
        })();
        let (skeleton, anims) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Ani: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        // SkelAnime_InitFlex(.., gRoofManSkel, gRoofManIdleAnim, .., 0x10), then
        // Animation_PlayOnce(gRoofManIdleAnim).
        let mut skel = SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anims.idle.clone()));
        skel.play_once(anims.idle.clone());
        let adult = play.save.adult;
        let mut this = EnAni {
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            skel,
            skeleton,
            unk_29c: [0; 3],
            unk_2a2: [0; 3],
            unk_2a8: 0,
            unk_2aa: 0,
            eye_index: 0,
            blink_timer: 0,
            // !LINK_IS_ADULT: func_809B064C; else func_809B07F8.
            action: if adult { Action::AdultIdle } else { Action::ChildIdle },
            anims,
            actor,
        };
        // sInitChain: scale 0.01, cullingVolumeDistance 850.
        this.actor.scale = Vec3::splat(0.01);
        this.actor.culling_volume_distance = 850.0;
        this.actor.shape_init(-2800.0, oot_game::shadow::ShadowDraw::Circle, 36.0);
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.min_velocity_y = -1.0;
        this.actor.velocity.y = -1.0;
        Box::new(this)
    }

    /// `EnAni_SetText`: the text, the tracking on, and the offer within 100.
    fn set_text(&mut self, play: &mut PlayState, text_id: u16) {
        self.actor.text_id = text_id;
        self.unk_2a8 |= 1;
        let a = self.actor.clone();
        offer_talk(play, &a, 100.0);
    }

    /// `Actor_OfferGetItem(&actor, play, GI_HEART_PIECE, 10000, 200)`.
    fn offer_heart_piece(&self, play: &mut PlayState) {
        let a = self.actor.clone();
        offer_get_item_range(play, &a, GI_HEART_PIECE, 10000.0, 200.0);
    }

    /// `func_809B04F0`.
    fn child_talk(&mut self, play: &PlayState) {
        if textbox_is_closing(play) {
            self.action = Action::ChildIdle;
        }
    }

    /// `func_809B0524`.
    fn adult_talk(&mut self, play: &PlayState) {
        if textbox_is_closing(play) {
            self.action = Action::AdultIdle;
        }
    }

    /// `func_809B0558`: once Player has it (`Actor_HasParent`), `ITEMGETINF_15` and back to the
    /// talk (its box is the item's).
    fn give_heart_piece(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.action = if !play.save.adult { Action::ChildTalk } else { Action::AdultTalk };
            play.save.set_item_get_inf(ITEMGETINF_15);
        } else {
            self.offer_heart_piece(play);
        }
    }

    /// `func_809B05F0`: the offer runs while the text is up, too.
    fn heart_piece_text(&mut self, play: &mut PlayState) {
        if textbox_is_closing(play) {
            self.action = Action::GiveHeartPiece;
        }
        self.offer_heart_piece(play);
    }

    /// `this->actor.yawTowardsPlayer - this->actor.shape.rot.y`.
    fn yaw_diff(&self) -> i16 {
        self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y)
    }

    /// `func_809B064C`: a child's idle.
    fn child_idle(&mut self, play: &mut PlayState) {
        // MaskReaction_GetTextId(play, MASK_REACTION_SET_KAKARIKO_ROOF_MAN): no masks, so 0.
        let text_id = if !play.save.is_day() { TEXT_CHILD_NIGHT } else { TEXT_CHILD_DAY };
        let yaw_diff = self.yaw_diff();
        if process_talk_request(&mut self.actor) {
            self.action = match self.actor.text_id {
                TEXT_HEART_PIECE_GIVEN => Action::ChildTalk,
                TEXT_HEART_PIECE => Action::HeartPieceText,
                _ => Action::ChildTalk,
            };
        } else if (-0x36AF..0).contains(&yaw_diff) && self.actor.xz_dist_to_player < 150.0 && -80.0 < self.actor.y_dist_to_player {
            let t = if play.save.get_item_get_inf(ITEMGETINF_15) { TEXT_HEART_PIECE_GIVEN } else { TEXT_HEART_PIECE };
            self.set_text(play, t);
        } else if (-0x3E7..0x36B0).contains(&yaw_diff) && self.actor.xz_dist_to_player < 350.0 {
            self.set_text(play, text_id);
        }
    }

    /// `func_809B07F8`: an adult's idle.
    fn adult_idle(&mut self, play: &mut PlayState) {
        let yaw_diff = self.yaw_diff();
        if process_talk_request(&mut self.actor) {
            self.action = match self.actor.text_id {
                TEXT_HEART_PIECE_GIVEN => Action::AdultTalk,
                TEXT_HEART_PIECE => Action::HeartPieceText,
                _ => Action::AdultTalk,
            };
        } else if yaw_diff > -0x36B0 && yaw_diff < 0 && self.actor.xz_dist_to_player < 150.0 && -80.0 < self.actor.y_dist_to_player {
            let t = if play.save.get_item_get_inf(ITEMGETINF_15) { TEXT_HEART_PIECE_GIVEN } else { TEXT_HEART_PIECE };
            self.set_text(play, t);
        } else if yaw_diff > -0x3E8 && yaw_diff < 0x36B0 && self.actor.xz_dist_to_player < 350.0 {
            let t = if !play.save.get_event_chk_inf(EVENTCHKINF_2F) {
                TEXT_ADULT
            } else if play.save.get_item_get_inf(ITEMGETINF_15) {
                TEXT_ADULT_GIVEN
            } else {
                TEXT_ADULT_2F
            };
            self.set_text(play, t);
        }
    }

    /// The cue on channel 0's id (`play->csCtx.actorCues[0]->id`).
    fn cue_id(play: &PlayState) -> Option<u16> {
        play.cs_ctx.npc_actions[0].map(|c| c.action)
    }

    /// `func_809B0988`: up again; nothing.
    fn func_809b0988(&mut self) {}

    /// `func_809B0994`: knocked over, until cue 4 gets him up.
    fn func_809b0994(&mut self, play: &PlayState) {
        if Self::cue_id(play) == Some(4) {
            let anim = self.anims.getting_up.clone();
            let last = anim.last_frame();
            self.skel.change(anim, 1.0, 0.0, last, ANIMMODE_ONCE, -4.0);
            self.unk_2aa += 1;
            self.actor.shape_shadow_draw = oot_game::shadow::ShadowDraw::Circle;
        }
    }

    /// `func_809B0A28`: the animation to its end.
    fn func_809b0a28(&mut self) {
        if self.skel.update() {
            self.unk_2aa += 1;
        }
    }

    /// `func_809B0A6C`: the idle looped until cue 2 knocks him over.
    fn func_809b0a6c(&mut self, play: &PlayState) {
        if self.skel.update() {
            self.skel.cur_frame = 0.0;
        }
        if Self::cue_id(play) == Some(2) {
            let anim = self.anims.knockback.clone();
            let last = anim.last_frame();
            self.skel.change(anim, 1.0, 0.0, last, ANIMMODE_ONCE, 0.0);
            self.actor.shape_shadow_draw = oot_game::shadow::ShadowDraw::None;
            self.unk_2aa += 1;
        }
    }

    /// The pose with `EnAni_OverrideLimbDraw`'s head turn: limb 15's x by the head's y and its z by
    /// the head's x.
    pub fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 3]) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if limb == HEAD_LIMB {
                rot[0] = rot[0].wrapping_add(head[1]);
                rot[2] = rot[2].wrapping_add(head[0]);
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnAni {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnAni_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        if play.cs_ctx.state != CS_STATE_IDLE && play.cs_ctx.npc_actions[0].is_some() {
            match self.unk_2aa {
                0 => self.func_809b0a6c(play),
                1 => self.func_809b0a28(),
                2 => self.func_809b0994(play),
                3 => self.func_809b0a28(),
                4 => self.func_809b0988(),
                _ => {}
            }
            if play.cs_ctx.frames == 100 {
                play.audio.play_sfx_centered2(NA_SE_IT_EARTHQUAKE);
            }
        } else {
            if self.skel.update() {
                self.skel.cur_frame = 0.0;
            }
            match self.action {
                Action::ChildIdle => self.child_idle(play),
                Action::AdultIdle => self.adult_idle(play),
                Action::ChildTalk => self.child_talk(play),
                Action::AdultTalk => self.adult_talk(play),
                Action::HeartPieceText => self.heart_piece_text(play),
                Action::GiveHeartPiece => self.give_heart_piece(play),
            }
        }
        if self.unk_2a8 & 1 != 0 {
            let focus = self.actor.focus_pos;
            actor_track_player(play, &mut self.actor, &mut self.unk_29c, &mut self.unk_2a2, focus);
            self.unk_2a2 = [0; 3];
        } else {
            smooth_step_to_s(&mut self.unk_29c[0], 0, 6, 6200, 100);
            smooth_step_to_s(&mut self.unk_29c[1], 0, 6, 6200, 100);
            smooth_step_to_s(&mut self.unk_2a2[0], 0, 6, 6200, 100);
            smooth_step_to_s(&mut self.unk_2a2[1], 0, 6, 6200, 100);
        }
        // DECR(blinkTimer) == 0: a new wait of 60 to 120; the eyes are the timer's last 3 frames.
        if self.blink_timer != 0 {
            self.blink_timer -= 1;
        }
        if self.blink_timer == 0 {
            self.blink_timer = play.rand.s16_offset(60, 60);
        }
        self.eye_index = self.blink_timer;
        if self.eye_index >= 3 {
            self.eye_index = 0;
        }
    }
    /// `EnAni_PostLimbDraw`, the head: the focus at (800, 500, 0) in it.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, self.unk_29c);
        if let Some(head) = bones.get(HEAD_LIMB - 1) {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(FOCUS_IN_HEAD);
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = self.unk_29c.to_vec();
        rs.switches = vec![self.eye_index as u32];
        rs
    }
    /// `EnAni_Draw`: `Gfx_SetupDL_37Opa`, the eyes on segment 8, the skeleton.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let ([h0, h1, h2], [eye]) = (rs.angles.as_slice(), rs.switches.as_slice()) else { return };
        let bones = Self::pose(&self.skeleton, &joints.rot, [*h0, *h1, *h2]);
        let eye = (*eye as usize).min(EYE_TEXTURES.len() - 1);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(eye))), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
    }
    /// `EnAni_Destroy`: `Collider_DestroyCylinder`.
    fn destroy(&mut self, _play: &mut PlayState) {}
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
