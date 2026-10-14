//! `En_Toryo` (`ovl_En_Toryo/z_en_toryo.c`): Mutoh, the boss carpenter.
//!
//! `EnToryo_Init` keeps him (`stateFlags`) in Gerudo Valley for an adult (1), in Kakariko by day
//! for a child (2) and in the guest house by night for a child (4); anywhere else he's killed.
//! He stands (`object_toryo_Anim_000E50`), his head and torso following Link within 80 degrees
//! of his front (`Npc_TrackPoint`), his whole body while talking.
//!
//! His text (`EnToryo_GetTextId`): the adult's 0x606A, then 0x606B once `INFTABLE_170` is set
//! (the first ended), 0x606C once the four carpenters are rescued
//! (`GET_EVENTCHKINF_CARPENTERS_ALL_RESCUED`); the child's in Kakariko 0x5028, then 0x5029 once
//! `INFTABLE_172` is set (0x5028 ended); the guest house's 0x506C. He offers to talk for the
//! Poacher's Saw (`EXCH_ITEM_POACHERS_SAW`): shown it, `NA_SE_SY_TRE_BOX_APPEAR` and 0x606D (or
//! 0x606E once `INFTABLE_171` is set), with a choice: the first answer gives the Broken Goron's
//! Sword (0x601B, then `GI_BROKEN_GORONS_SWORD` offered within 100 across and 10 up or down until
//! Player takes it); the second 0x606F, which sets `INFTABLE_171`. Anything else shown: 0x200F.
//! (`EnToryo_HandleTalking`, `EnToryo_TalkRespond`, `EnToryo_DoneTalking`,
//! `EnToryo_ReactToExchangeItem`.)
//!
//! Drawn as `EnToryo_Draw` does: the skeleton with the torso (limb 8) and head (limb 15) turns
//! of `EnToryo_OverrideLimbDraw`, the focus 800 along and 1000 up the head
//! (`EnToryo_PostLimbDraw`). From a baked mesh (docs/adr/0012-actor-bakes.md).
//!
//! Not ported here: the mask reactions (`MaskReaction_GetTextId`: no masks yet, so none).
//!
//! Params: none read (placed as 0xFFFF, unused).
//! Flags: reads EVENTCHKINF_CARPENTER_0_RESCUED to _3_RESCUED (all four: the adult's text);
//! reads and sets INFTABLE_170 (after 0x606A), INFTABLE_171 (after 0x606F), INFTABLE_172 (after
//! 0x5028).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR;
use oot_game::collision_check::*;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::message::{MSGMODE_PAUSED, TEXT_STATE_CHOICE, TEXT_STATE_DONE};
use oot_game::npc::{EXCH_ITEM_NONE, NpcTrack, get_player_exchange_item_id, offer_talk_exchange, process_talk_request, track_point};
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;
use oot_game::{event_chk_inf, inf_table};

/// `ACTOR_EN_TORYO` (`actor_table.h`: 0x0132).
pub const ACTOR_EN_TORYO: i16 = 0x0132;
/// `OBJECT_TORYO` (`object_table.h`: 0x0121).
pub const OBJECT: &str = "object_toryo";

/// `En_Toryo_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_TORYO, name: "En_Toryo", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT, schema: &SCHEMA };

/// The placed params, read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed as 0xFFFF; read by nothing");

/// En_Toryo's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_CARPENTER_0_RESCUED)).reads().note("all four rescued: the adult's text").indirect("GET_EVENTCHKINF_CARPENTERS_ALL_RESCUED, a list of the four"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_CARPENTER_1_RESCUED)).reads().note("all four rescued: the adult's text").indirect("GET_EVENTCHKINF_CARPENTERS_ALL_RESCUED, a list of the four"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_CARPENTER_2_RESCUED)).reads().note("all four rescued: the adult's text").indirect("GET_EVENTCHKINF_CARPENTERS_ALL_RESCUED, a list of the four"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_CARPENTER_3_RESCUED)).reads().note("all four rescued: the adult's text").indirect("GET_EVENTCHKINF_CARPENTERS_ALL_RESCUED, a list of the four"),
        FlagUse::new(inf_table!(INFTABLE_170)).reads().sets().note("after text 0x606A"),
        FlagUse::new(inf_table!(INFTABLE_171)).reads().sets().note("after text 0x606F"),
        FlagUse::new(inf_table!(INFTABLE_172)).reads().sets().note("after text 0x5028"),
    ],
    todo: false,
};

// `SCENE_*` (`scene_table.h`).
/// `SCENE_KAKARIKO_CENTER_GUEST_HOUSE` (0x2A, `kakariko_scene`).
pub const SCENE_KAKARIKO_CENTER_GUEST_HOUSE: u16 = 0x2A;
/// `SCENE_KAKARIKO_VILLAGE` (0x52, `spot01_scene`).
pub const SCENE_KAKARIKO_VILLAGE: u16 = 0x52;
/// `SCENE_GERUDO_VALLEY` (0x5A, `spot09_scene`).
pub const SCENE_GERUDO_VALLEY: u16 = 0x5A;

// `INFTABLE_*` (`save.h`).
pub const INFTABLE_170: u16 = 0x170;
pub const INFTABLE_171: u16 = 0x171;
pub const INFTABLE_172: u16 = 0x172;
// `EVENTCHKINF_CARPENTER_*_RESCUED` (`save.h`: 0x90 to 0x93).
pub const EVENTCHKINF_CARPENTER_0_RESCUED: u16 = 0x90;
pub const EVENTCHKINF_CARPENTER_1_RESCUED: u16 = 0x91;
pub const EVENTCHKINF_CARPENTER_2_RESCUED: u16 = 0x92;
pub const EVENTCHKINF_CARPENTER_3_RESCUED: u16 = 0x93;

/// `EXCH_ITEM_POACHERS_SAW` (`item.h`: 0x0A).
pub const EXCH_ITEM_POACHERS_SAW: u8 = 0x0A;
/// `GI_BROKEN_GORONS_SWORD` (`item.h`: 0x22).
pub const GI_BROKEN_GORONS_SWORD: i16 = 0x22;

/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `NPC_TRACKING_NONE`, `NPC_TRACKING_HEAD_AND_TORSO`, `NPC_TRACKING_FULL_BODY` (`actor.h`).
const NPC_TRACKING_NONE: i16 = 1;
const NPC_TRACKING_HEAD_AND_TORSO: i16 = 2;
const NPC_TRACKING_FULL_BODY: i16 = 4;
/// `DEG_TO_BINANG2(80.0f)`: `(s32)(80 * (0x8000 / 180.0f))` off N64.
const TRACK_YAW_RANGE: f32 = 14563.0;
/// `SCREEN_WIDTH`, `SCREEN_HEIGHT`.
const SCREEN_WIDTH: i16 = 320;
const SCREEN_HEIGHT: i16 = 240;
/// The torso's and the head's limbs (1-based).
const TORSO_LIMB: usize = 8;
const HEAD_LIMB: usize = 15;
/// `sMultVec`: the focus on the head.
const FOCUS_OFFSET: Vec3 = Vec3::new(800.0, 1000.0, 0.0);

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

/// `sColChkInfoInit`: immovable. (`sDamageTable` is all zeroes.)
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// Mutoh's mesh.
pub const BAKE: &str = "En_Toryo/skel";

/// The mesh `EnToryo_Draw` draws: `object_toryo_Skel_007150`.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: Vec::new(),
        prelude: Vec::new(),
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "object_toryo_Skel_007150".into(), limbs: Vec::new() },
    }]
}

pub struct EnToryo {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub collider: ColliderCylinder,
    /// `exchangeItemId`: what Player showed.
    pub exchange_item_id: u8,
    /// `messageState`: 0 idle, 1 talking, 2 the next text, 3 the sword's text, 4 offering the
    /// sword, 5 waiting for the text's end.
    pub message_state: i32,
    pub state_flags: u16,
    pub interact_info: NpcTrack,
}

impl EnToryo {
    /// `EnToryo_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, "object_toryo_Skel_007150")?, assets.animation(OBJECT, "object_toryo_Anim_000E50")?)) })();
        let (skeleton, anim) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Toryo: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnToryo {
            // SkelAnime_InitFlex(.., object_toryo_Skel_007150, NULL, .., 17).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            exchange_item_id: EXCH_ITEM_NONE,
            message_state: 0,
            state_flags: 0,
            interact_info: NpcTrack::default(),
            actor,
        };
        let (adult, night) = (play.save.adult, play.save.night_flag);
        match play.scene_id {
            SCENE_GERUDO_VALLEY => {
                if adult {
                    this.state_flags |= 1;
                }
            }
            SCENE_KAKARIKO_VILLAGE => {
                if !adult && !night {
                    this.state_flags |= 2;
                }
            }
            SCENE_KAKARIKO_CENTER_GUEST_HOUSE => {
                if !adult && night {
                    this.state_flags |= 4;
                }
            }
            _ => {}
        }
        if this.state_flags & 7 == 0 {
            this.actor.kill();
        }
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 42); Actor_Init's scale 0.01.
        this.actor.shape_y_offset = 0.0;
        this.actor.scale = Vec3::splat(0.01);
        this.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        this.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        // sEnToryoAnimation: speed 1, ANIMMODE_LOOP, no morph.
        let last = anim.last_frame();
        this.skel.change(anim, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
        this.state_flags |= 8;
        this.actor.target_mode = ATTENTION_RANGE_6;
        Box::new(this)
    }

    /// `EnToryo_TalkRespond`: the next `messageState`.
    fn talk_respond(&mut self, play: &mut PlayState) -> i32 {
        let mut ret = 1;
        match play.message_state() {
            TEXT_STATE_CHOICE => {
                if play.message_should_advance() {
                    if play.msg_ctx.choice_index == 0 {
                        play.with_msg(|m, f| m.close_textbox(f.audio));
                        self.actor.parent = None;
                        if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
                            p.set_exchange_item_id(EXCH_ITEM_NONE);
                        }
                        play.msg_ctx.msg_mode = MSGMODE_PAUSED;
                        self.actor.text_id = 0x601B;
                        ret = 3;
                    } else {
                        self.actor.text_id = 0x606F;
                        ret = 2;
                    }
                }
            }
            TEXT_STATE_DONE => {
                let advance = play.message_should_advance();
                match self.actor.text_id {
                    0x5028 => {
                        if advance {
                            play.save.set_inf_table(INFTABLE_172);
                            ret = 0;
                        }
                    }
                    0x601B => {
                        if advance {
                            ret = 4;
                        }
                    }
                    0x606F => {
                        if advance {
                            play.save.set_inf_table(INFTABLE_171);
                            ret = 0;
                        }
                    }
                    0x606A => {
                        if advance {
                            play.save.set_inf_table(INFTABLE_170);
                            ret = 0;
                        }
                    }
                    // 0x606B to 0x606E and any other.
                    _ => {
                        if advance {
                            ret = 0;
                        }
                    }
                }
            }
            // TEXT_STATE_NONE, _DONE_HAS_NEXT, _CLOSING, _DONE_FADING, _EVENT; the others leave
            // it at 1 too.
            _ => {}
        }
        ret
    }

    /// `EnToryo_DoneTalking`: 0 when the text ends, else 5.
    fn done_talking(&mut self, play: &mut PlayState) -> i32 {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            return 0;
        }
        5
    }

    /// `EnToryo_ReactToExchangeItem` for `exchange_item_id` (`this->exchangeItemId`): the saw's
    /// text, or 0x200F for anything else shown.
    pub fn react_to_exchange_item(exchange_item_id: u8, play: &mut PlayState) -> u16 {
        // @bug (game): with nothing shown the return value is uninitialised (here 0); it's only
        // ever called with something shown.
        if exchange_item_id == EXCH_ITEM_NONE {
            return 0;
        }
        if exchange_item_id == EXCH_ITEM_POACHERS_SAW {
            play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
            if play.save.get_inf_table(INFTABLE_171) { 0x606E } else { 0x606D }
        } else {
            0x200F
        }
    }

    /// `EnToryo_GetTextId`.
    pub fn get_text_id(&self, play: &PlayState) -> u16 {
        // MaskReaction_GetTextId(play, MASK_REACTION_SET_CARPENTER_BOSS): no masks, so 0.
        let text_id = 0;
        let s = &play.save;
        if self.state_flags & 1 != 0 {
            // GET_EVENTCHKINF_CARPENTERS_ALL_RESCUED. schema: the four flags from a list
            let all_rescued = [EVENTCHKINF_CARPENTER_0_RESCUED, EVENTCHKINF_CARPENTER_1_RESCUED, EVENTCHKINF_CARPENTER_2_RESCUED, EVENTCHKINF_CARPENTER_3_RESCUED].iter().all(|&f| s.get_event_chk_inf(f));
            if all_rescued {
                0x606C
            } else if s.get_inf_table(INFTABLE_170) {
                0x606B
            } else {
                0x606A
            }
        } else if self.state_flags & 2 != 0 {
            if s.get_inf_table(INFTABLE_172) { 0x5029 } else { 0x5028 }
        } else if self.state_flags & 4 != 0 {
            0x506C
        } else {
            text_id
        }
    }

    /// `EnToryo_HandleTalking`.
    fn handle_talking(&mut self, play: &mut PlayState) {
        if self.message_state == 3 {
            process_talk_request(&mut self.actor);
            play.continue_textbox(self.actor.text_id);
            self.message_state = 1;
        }
        if self.message_state == 1 {
            self.message_state = self.talk_respond(play);
        }
        if self.message_state == 5 {
            self.message_state = self.done_talking(play);
            return;
        }
        if self.message_state == 2 {
            play.continue_textbox(self.actor.text_id);
            self.message_state = 1;
        }
        if self.message_state == 4 {
            if actor_has_parent(&self.actor) {
                self.actor.parent = None;
                self.message_state = 5;
            } else {
                let a = self.actor.clone();
                offer_get_item_range(play, &a, GI_BROKEN_GORONS_SWORD, 100.0, 10.0);
            }
            return;
        }
        if self.message_state == 0 {
            if process_talk_request(&mut self.actor) {
                self.exchange_item_id = get_player_exchange_item_id(play);
                if self.exchange_item_id != EXCH_ITEM_NONE {
                    let id = Self::react_to_exchange_item(self.exchange_item_id, play);
                    if let Some(p) = play.player.and_then(|h| play.actors.actor_mut(h)) {
                        p.text_id = id;
                    }
                    self.actor.text_id = id;
                }
                self.message_state = 1;
                return;
            }
            let (x, y) = oot_game::target::actor_screen_pos(play.view_proj, &self.actor);
            if (0..=SCREEN_WIDTH).contains(&x) && (0..=SCREEN_HEIGHT).contains(&y) {
                self.actor.text_id = self.get_text_id(play);
                let a = self.actor.clone();
                offer_talk_exchange(play, &a, 100.0, EXCH_ITEM_POACHERS_SAW);
            }
        }
    }

    /// `EnToryo_Idle`.
    fn idle(&mut self, play: &mut PlayState) {
        self.skel.update();
        self.handle_talking(play);
        if self.message_state != 0 {
            self.state_flags |= 0x10;
        } else {
            self.state_flags &= !0x10;
        }
    }

    /// The pose with `EnToryo_OverrideLimbDraw`'s turns (with `stateFlags` 8): limb 8's x by the
    /// torso's y and its y less the torso's x; limb 15's x by the head's y and its z by the head's x.
    pub fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 3], torso: [i16; 3], on: bool) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if on {
                match limb {
                    TORSO_LIMB => {
                        rot[0] = rot[0].wrapping_add(torso[1]);
                        rot[1] = rot[1].wrapping_sub(torso[0]);
                    }
                    HEAD_LIMB => {
                        rot[0] = rot[0].wrapping_add(head[1]);
                        rot[2] = rot[2].wrapping_add(head[0]);
                    }
                    _ => {}
                }
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnToryo {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnToryo_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.idle(play);
        if self.state_flags & 8 != 0 {
            self.interact_info.target = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.focus_pos).unwrap_or(self.actor.focus_pos);
            let mut t = self.interact_info;
            if self.state_flags & 0x10 != 0 {
                track_point(play, &mut self.actor, &mut t, 0, NPC_TRACKING_FULL_BODY);
            } else {
                // rot = yawTowardsPlayer - shape.rot.y, as ints (not wrapped).
                let rot = (self.actor.yaw_towards_player as i32 - self.actor.shape_rot.y as i32) as f32;
                let mode = if rot < TRACK_YAW_RANGE && rot > -TRACK_YAW_RANGE { NPC_TRACKING_HEAD_AND_TORSO } else { NPC_TRACKING_NONE };
                track_point(play, &mut self.actor, &mut t, 0, mode);
            }
            self.interact_info = t;
        }
    }
    /// `EnToryo_PostLimbDraw`, limb 15: the focus.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let t = &self.interact_info;
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, t.head, t.torso, self.state_flags & 8 != 0);
        if let Some(head) = bones.get(HEAD_LIMB - 1) {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(FOCUS_OFFSET);
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let t = &self.interact_info;
        rs.angles = vec![t.head[0], t.head[1], t.head[2], t.torso[0], t.torso[1], t.torso[2]];
        rs
    }
    /// `EnToryo_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 6 {
            return;
        }
        let a = &rs.angles;
        let bones = Self::pose(&self.skeleton, &joints.rot, [a[0], a[1], a[2]], [a[3], a[4], a[5]], self.state_flags & 8 != 0);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
        // ActorShadow_DrawCircle (shadowScale 42): the circle shadow's stand-in.
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(42.0, 1.0, 42.0));
        out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), shadow));
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
