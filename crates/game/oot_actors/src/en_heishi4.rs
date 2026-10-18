//! `En_Heishi4` (`ovl_En_Heishi4/z_en_heishi4.c`): the guards who talk. `params & 0xFF` is the type
//! (`Heishi4Type`), `params >> 8` a message number (kept, unread).
//!
//! - `HEISHI4_AT_KAKARIKO_ENTRANCE` (0) and `HEISHI4_AT_IMPAS_HOUSE` (4): Kakariko's guards, a text
//!   by the time of day and whether Link has talked once (`unk_284`).
//! - `HEISHI4_AT_MARKET_NIGHT` (8): Castle Town's guards (the entrance, the back alley, the market
//!   by night, the guard house): "Kids shouldn't be out at night"; gone for an adult.
//! - `HEISHI4_AT_MARKET_DYING` (7): the guard dying in the back alley after Zelda's escape, who
//!   talks once (`INFTABLE_6C`), with Link held, and dies.
//!
//! Each turns its whole body to Link (`Npc_TrackPoint` preset 2) and offers to talk; the others'
//! head and torso turns are drawn by `EnHeishi_OverrideLimbDraw`. The skeleton is a bake
//! (docs/adr/0012-actor-bakes.md).
//!
//! Not ported here: the mask reactions (`MaskReaction_GetTextId`: no masks yet, so none).
//!
//! Params: `type` bits 0 to 7 (`Heishi4Type`: 0 Kakariko's entrance, 4 Impa's house, 7 the dying
//! guard, 8 Castle Town by night; any other has no action); bits 8 to 15 unused (`unk_27C`, read
//! and never used; the debug print shows bits 8 to 11; the data places 0xFF).
//! Flags: reads EVENTCHKINF_ZELDA_FLED_CASTLE and EVENTCHKINF_OBTAINED_MASTER_SWORD (Kakariko's
//! texts; the dying guard is there between the two); reads and sets INFTABLE_6C (the dying
//! guard's last words heard).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::{event_chk_inf, inf_table};
use oot_game::collision_check::*;
use oot_game::message::{TEXT_STATE_DONE, TEXT_STATE_EVENT};
use oot_game::npc::{NpcTrack, actor_track_player, offer_talk, offer_talk_default, process_talk_request, track_point};
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_HEISHI4` (`actor_table.h`: 0x0178).
pub const ACTOR_EN_HEISHI4: i16 = 0x0178;
pub const OBJECT: &str = "object_sd";

/// `En_Heishi4_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_HEISHI4, name: "En_Heishi4", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 8)`: `type`.
pub const TYPE: Field = Field::params(
    "type",
    0,
    8,
    Kind::Named(&[(0, "HEISHI4_AT_KAKARIKO_ENTRANCE"), (4, "HEISHI4_AT_IMPAS_HOUSE"), (7, "HEISHI4_AT_MARKET_DYING"), (8, "HEISHI4_AT_MARKET_NIGHT")]),
    "which guard: Kakariko's two, the dying one, Castle Town's by night",
);
/// `PARAMS_GET_U(params, 8, 8)`: `unk_27C`, which nothing reads.
pub const UNK_27C: Field = Field::params("unk_27C", 8, 8, Kind::Unused, "a message number, kept and never read (placed 0xFF)");

/// En_Heishi4's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, UNK_27C],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_OBTAINED_MASTER_SWORD)).reads().note("Kakariko's text 0x5068; the dying guard gone"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_ZELDA_FLED_CASTLE)).reads().note("Kakariko's text 0x5065; the dying guard's words"),
        FlagUse::new(inf_table!(INFTABLE_6C)).reads().sets().note("the dying guard's last words heard (0x7007, then 0x7008)"),
    ],
    todo: false,
    objects: &[],
};

// `Heishi4Type`.
pub const HEISHI4_AT_KAKARIKO_ENTRANCE: i16 = 0;
pub const HEISHI4_AT_IMPAS_HOUSE: i16 = 4;
pub const HEISHI4_AT_MARKET_DYING: i16 = 7;
pub const HEISHI4_AT_MARKET_NIGHT: i16 = 8;

// `SCENE_*` (`scene_table.h`).
const SCENE_MARKET_NIGHT: u16 = 0x21;
const SCENE_MARKET_GUARD_HOUSE: u16 = 0x4D;

// `EVENTCHKINF_*`, `INFTABLE_*` (`save.h`).
const EVENTCHKINF_OBTAINED_MASTER_SWORD: u16 = 0x45;
const EVENTCHKINF_ZELDA_FLED_CASTLE: u16 = 0x80;
const INFTABLE_6C: u16 = 0x6C;

/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `NPC_TRACKING_FULL_BODY`.
const NPC_TRACKING_FULL_BODY: i16 = 4;
/// `PLAYER_CSACTION_7` (back to normal), `PLAYER_CSACTION_8` (held).
const PLAYER_CSACTION_7: u8 = 7;
const PLAYER_CSACTION_8: u8 = 8;

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
    dim: Cylinder16 { radius: 33, height: 40, y_shift: 0, pos: [0; 3] },
};

/// The guard's skeleton (`gEnHeishiSkel`), shared with `En_Heishi1` to `_3`.
pub const BAKE: &str = "En_Heishi/skel";

/// The mesh `EnHeishi4_Draw` draws.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: Vec::new(),
        prelude: Vec::new(),
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gEnHeishiSkel".into(), limbs: Vec::new() },
    }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80A56328`: Kakariko's guards' idle.
    SetupKakariko,
    /// `func_80A563BC`: Kakariko's guards' text.
    KakarikoText,
    /// `func_80A56544`: the market's guards' idle (gone for an adult).
    SetupMarketNight,
    /// `func_80A56614`: the market's guards' text.
    MarketNightText,
    /// `func_80A5673C`: the dying guard's setup.
    SetupDying,
    /// `func_80A56874`: the dying guard waiting to talk.
    DyingWait,
    /// `func_80A56900`: the dying guard's talk animation.
    DyingSetupTalk,
    /// `func_80A56994`: the dying guard talking.
    DyingTalk,
    /// `func_80A56A50`: the dying guard's death animation.
    DyingSetupDie,
    /// `func_80A56ACC`: the dying guard dying.
    DyingDie,
    /// `func_80A56B40`: offering to talk.
    Talk,
}

pub struct EnHeishi4 {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub head_rot: [i16; 3],
    pub torso_rot: [i16; 3],
    /// `pos`: where it stands (kept every frame).
    pub pos: Vec3,
    /// `height`: the focus's height.
    pub height: f32,
    pub unk_27c: i16,
    pub unk_27e: i16,
    pub ty: i16,
    /// `unk_282`: the message state the dying guard's text ends on.
    pub unk_282: u8,
    /// `unk_284`: Kakariko's guards: talked once; the dying guard: about to talk.
    pub unk_284: i16,
    /// `unk_288`: the death animation's last frame.
    pub unk_288: f32,
    pub interact_info: NpcTrack,
    /// `unk_2B4`: the text is a mask reaction.
    pub unk_2b4: u8,
    pub collider: ColliderCylinder,
    anims: Anims,
}

struct Anims {
    idle: Anim,
    dying: Anim,
    dying_talk: Anim,
    dying_die: Anim,
}

impl EnHeishi4 {
    /// `EnHeishi4_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.skeleton(OBJECT, "gEnHeishiSkel")?;
            let anims = Anims {
                idle: assets.animation(OBJECT, "gEnHeishiIdleAnim")?,
                dying: assets.animation(OBJECT, "gEnHeishiDyingGuardAnim_00C444")?,
                dying_talk: assets.animation(OBJECT, "gEnHeishiDyingGuardTalkAnim")?,
                dying_die: assets.animation(OBJECT, "gEnHeishiDyingGuardDieAnim")?,
            };
            Ok((skeleton, anims))
        })();
        let (skeleton, anims) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Heishi4: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let ty = TYPE.get(actor.params);
        let dying = ty == HEISHI4_AT_MARKET_DYING;
        let mut this = EnHeishi4 {
            // SkelAnime_Init(.., 17), playing the dying guard's or the idle animation.
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(if dying { anims.dying.clone() } else { anims.idle.clone() })),
            skeleton,
            action: Action::Talk,
            head_rot: [0; 3],
            torso_rot: [0; 3],
            pos: actor.world_pos,
            height: if dying { 30.0 } else { 60.0 },
            unk_27c: 0,
            unk_27e: 0,
            ty,
            unk_282: 0,
            unk_284: 0,
            unk_288: 0.0,
            interact_info: NpcTrack::default(),
            unk_2b4: 0,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            anims,
            actor,
        };
        this.actor.scale = Vec3::splat(0.01);
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.target_mode = ATTENTION_RANGE_6;
        // ActorShape_Init(&shape, 0, the dying guard NULL, others ActorShadow_DrawCircle, 30).
        let shadow = if ty == HEISHI4_AT_MARKET_DYING { oot_game::shadow::ShadowDraw::None } else { oot_game::shadow::ShadowDraw::Circle };
        this.actor.shape_init(0.0, shadow, 30.0);
        this.collider.dim.y_shift = 0;
        this.collider.dim.radius = 15;
        this.collider.dim.height = 70;
        match ty {
            HEISHI4_AT_KAKARIKO_ENTRANCE | HEISHI4_AT_IMPAS_HOUSE => this.action = Action::SetupKakariko,
            HEISHI4_AT_MARKET_DYING => {
                this.collider.dim.radius = 28;
                this.collider.dim.height = 5;
                this.action = Action::SetupDying;
            }
            HEISHI4_AT_MARKET_NIGHT => this.action = Action::SetupMarketNight,
            // Any other type keeps actionFunc NULL: the update would jump to address 0.
            _ => log::warn!("En_Heishi4: type {ty} has no action (actionFunc NULL)"),
        }
        this.unk_27c = UNK_27C.get(this.actor.params);

        Box::new(this)
    }

    /// `Animation_Change(&skelAnime, anim, 1, 0, (s16)lastFrame, LOOP, -10)`.
    fn change_loop(&mut self, anim: Anim) {
        let frames = anim.last_frame() as i16 as f32;
        self.skel.change(anim, 1.0, 0.0, frames, ANIMMODE_LOOP, -10.0);
    }

    /// `reactionOffset`: the mask reaction set for the type (no masks: never read).
    fn mask_reaction_text(&self) -> u16 {
        // MaskReaction_GetTextId(play, sMaskReactionSets[reactionOffset]): no masks, so 0.
        0
    }

    /// `func_80A56328`.
    fn setup_kakariko(&mut self) {
        self.change_loop(self.anims.idle.clone());
        self.action = Action::KakarikoText;
    }

    /// `func_80A563BC`: Kakariko's guards' text.
    fn kakariko_text(&mut self, play: &PlayState) {
        self.unk_2b4 = 0;
        let mask = self.mask_reaction_text();
        if mask != 0 {
            self.actor.text_id = mask;
            self.unk_2b4 = 1;
            self.action = Action::Talk;
            return;
        }
        let s = &play.save;
        if s.get_event_chk_inf(EVENTCHKINF_ZELDA_FLED_CASTLE) {
            self.actor.text_id = 0x5065;
            self.action = Action::Talk;
            return;
        }
        if s.get_event_chk_inf(EVENTCHKINF_OBTAINED_MASTER_SWORD) {
            self.actor.text_id = 0x5068;
            self.action = Action::Talk;
            return;
        }
        let first = self.unk_284 == 0;
        self.actor.text_id = if self.ty == HEISHI4_AT_IMPAS_HOUSE {
            if first { 0x5079 } else { 0x507A }
        } else if s.is_day() {
            if first { 0x5063 } else { 0x5064 }
        } else if first {
            0x5066
        } else {
            0x5067
        };
        self.action = Action::Talk;
    }

    /// `func_80A56544`.
    fn setup_market_night(&mut self, play: &PlayState) {
        self.change_loop(self.anims.idle.clone());
        if play.save.adult {
            log::debug!("En_Heishi4: Oh, no! I'm an adult!");
            self.actor.kill();
        } else {
            self.action = Action::MarketNightText;
        }
    }

    /// `func_80A56614`: the market's guards' text.
    fn market_night_text(&mut self, play: &PlayState) {
        self.unk_2b4 = 0;
        let mask = self.mask_reaction_text();
        if mask != 0 {
            self.actor.text_id = mask;
            self.unk_2b4 = 1;
            self.action = Action::Talk;
            return;
        }
        let day = play.save.is_day();
        self.actor.text_id = if play.scene_id == SCENE_MARKET_GUARD_HOUSE {
            if day { 0x7004 } else { 0x709A }
        } else if play.scene_id != SCENE_MARKET_NIGHT {
            if day { 0x7002 } else { 0x7003 }
        } else {
            0x7003
        };
        self.action = Action::Talk;
    }

    /// `func_80A5673C`: the dying guard, there after Zelda's escape until the Master Sword.
    fn setup_dying(&mut self, play: &PlayState) {
        let s = &play.save;
        if s.get_event_chk_inf(EVENTCHKINF_OBTAINED_MASTER_SWORD) {
            self.actor.kill();
            return;
        }
        self.unk_284 = 0;
        if s.get_event_chk_inf(EVENTCHKINF_ZELDA_FLED_CASTLE) {
            if !s.get_inf_table(INFTABLE_6C) {
                self.change_loop(self.anims.dying.clone());
                self.actor.text_id = 0x7007;
                self.unk_282 = TEXT_STATE_EVENT;
                self.unk_284 = 1;
            } else {
                self.actor.text_id = 0x7008;
                self.unk_282 = TEXT_STATE_DONE;
            }
            self.action = Action::DyingWait;
        } else {
            self.actor.kill();
        }
    }

    /// `func_80A56874`.
    fn dying_wait(&mut self, play: &mut PlayState) {
        if self.unk_284 != 0 {
            self.skel.update();
        }
        if process_talk_request(&mut self.actor) {
            self.action = if self.unk_284 == 0 { Action::SetupDying } else { Action::DyingSetupTalk };
        } else {
            let a = self.actor.clone();
            offer_talk(play, &a, 100.0);
        }
    }

    /// `func_80A56900`.
    fn dying_setup_talk(&mut self) {
        self.change_loop(self.anims.dying_talk.clone());
        self.action = Action::DyingTalk;
    }

    /// `func_80A56994`: when his text ends, Link held and the death.
    fn dying_talk(&mut self, play: &mut PlayState) {
        self.skel.update();
        let focus = self.actor.focus_pos;
        actor_track_player(play, &mut self.actor, &mut self.head_rot, &mut self.torso_rot, focus);
        if self.unk_282 == play.message_state() && play.message_should_advance() {
            play.with_msg(|m, f| m.close_textbox(f.audio));
            play.save.set_inf_table(INFTABLE_6C);
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
            self.action = Action::DyingSetupDie;
        }
    }

    /// `func_80A56A50`.
    fn dying_setup_die(&mut self) {
        let anim = self.anims.dying_die.clone();
        let frames = anim.last_frame();
        self.unk_288 = frames;
        self.skel.change(anim, 1.0, 0.0, frames, ANIMMODE_ONCE, -10.0);
        self.action = Action::DyingDie;
    }

    /// `func_80A56ACC`.
    fn dying_die(&mut self, play: &mut PlayState) {
        let current_frame = self.skel.cur_frame;
        self.skel.update();
        if self.unk_288 <= current_frame {
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_7);
            self.action = Action::SetupDying;
        }
    }

    /// `func_80A56B40`: offering to talk; back to the text when a mask goes on or off, or after a
    /// talk.
    fn talk(&mut self, play: &mut PlayState) {
        self.skel.update();
        let kakariko = self.ty == HEISHI4_AT_KAKARIKO_ENTRANCE || self.ty == HEISHI4_AT_IMPAS_HOUSE;
        let market = self.ty == HEISHI4_AT_MARKET_NIGHT;
        if self.mask_reaction_text() != 0 {
            if self.unk_2b4 == 0 {
                if kakariko {
                    self.action = Action::KakarikoText;
                    return;
                }
                if market {
                    self.action = Action::MarketNightText;
                    return;
                }
            }
        } else if self.unk_2b4 != 0 {
            if kakariko {
                self.action = Action::KakarikoText;
                return;
            }
            if market {
                self.action = Action::MarketNightText;
                return;
            }
        }
        if process_talk_request(&mut self.actor) {
            if kakariko {
                self.unk_284 = 1;
                self.action = Action::KakarikoText;
                return;
            }
            if market {
                self.action = Action::MarketNightText;
                return;
            }
        }
        let a = self.actor.clone();
        offer_talk_default(play, &a);
    }

    /// The pose with `EnHeishi_OverrideLimbDraw`'s turns: limb 9's x by the torso's yaw, limb 16's
    /// x and z by the head's yaw and roll.
    pub fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 3], torso: [i16; 3]) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if limb == 9 {
                rot[0] = rot[0].wrapping_add(torso[1]);
            }
            if limb == 16 {
                rot[0] = rot[0].wrapping_add(head[1]);
                rot[2] = rot[2].wrapping_add(head[2]);
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnHeishi4 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnHeishi4_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.world_pos = self.pos;
        self.actor.set_focus(self.height);
        if self.ty != HEISHI4_AT_MARKET_DYING {
            let pp = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
            self.interact_info.target = pp;
            if !play.save.adult {
                self.interact_info.target.y = pp.y - 10.0;
            }
            let mut t = self.interact_info;
            track_point(play, &mut self.actor, &mut t, 2, NPC_TRACKING_FULL_BODY);
            self.interact_info = t;
            self.head_rot = self.interact_info.head;
            self.torso_rot = self.interact_info.torso;
        }
        self.unk_27e = self.unk_27e.wrapping_add(1);
        match self.action {
            Action::SetupKakariko => self.setup_kakariko(),
            Action::KakarikoText => self.kakariko_text(play),
            Action::SetupMarketNight => self.setup_market_night(play),
            Action::MarketNightText => self.market_night_text(play),
            Action::SetupDying => self.setup_dying(play),
            Action::DyingWait => self.dying_wait(play),
            Action::DyingSetupTalk => self.dying_setup_talk(),
            Action::DyingTalk => self.dying_talk(play),
            Action::DyingSetupDie => self.dying_setup_die(),
            Action::DyingDie => self.dying_die(play),
            Action::Talk => self.talk(play),
        }
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 10.0, 10.0, 30.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.head_rot[0], self.head_rot[1], self.head_rot[2], self.torso_rot[0], self.torso_rot[1], self.torso_rot[2]];
        rs
    }
    /// `EnHeishi4_Draw`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 6 {
            return;
        }
        let a = &rs.angles;
        let bones = Self::pose(&self.skeleton, &joints.rot, [a[0], a[1], a[2]], [a[3], a[4], a[5]]);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
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
