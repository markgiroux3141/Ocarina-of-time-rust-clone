//! `En_Gm` (`ovl_En_Gm/z_en_gm.c`): Medigoron, the big Goron in Goron City's room 2, behind two
//! bombable walls. A child gets his talk; an adult can buy the Giant's Knife from him
//! (`GI_SWORD_KNIFE`, 200 rupees).
//!
//! His profile's object is `object_oF1d_map` (the Gorons' skeleton, `gGoronSkel`), and his own
//! animation is in `object_gm`, which his init looks for in the room's banks (`Object_GetSlot`;
//! without it the C asserts). He waits for it to load (`func_80A3D838`), then sets up: the
//! skeleton playing `object_gm_Anim_0002B8` looped (read with segment 6 swapped to `object_gm`), a
//! cylinder (100 by 120, `ColliderCylinderInitType1`, pushing everything), scale 0.05, immovable,
//! the draw, text 0x3049, and he's culled from then on.
//!
//! He offers to talk (415 across) while Link touches his cylinder (OC) or stands within 100 of
//! his talk point (`talkPos`, 4300 model units in front of him). Until his switch flag is set (the
//! whole params: 6 in Goron City, the inner wall's) his text is 0x3049 (`func_80A3DB04`). Once
//! it is set, his text is by Link (`func_80A3D7C8`, `EnGm_SetTextID`): a child 0x304A, then 0x304B
//! once `INFTABLE_B0` is set (talking sets it); an adult without the knife 0x304C, then 0x304F
//! (`INFTABLE_B1`), the sale's choice; with the knife broken (`EQUIP_INV_SWORD_BROKENGIANTKNIFE`)
//! 0x304E, the choice again; with the knife whole or Biggoron's Sword 0x304D. The choice
//! (`EnGm_ProcessChoiceIndex`): "yes" with under 200 rupees text 0xC8, else the knife offered
//! (`Actor_OfferGetItem`, 415 across and 10 up) until Link has it, the 200 rupees taken once its
//! text is done (`func_80A3DF60`); "no" text 0x3050.
//!
//! His cylinder, talk point and focus (3800 in front, 100 up) are placed in his draw
//! (`func_80A3E090`, through the model matrix): the port's `draw_update`. Drawn as `EnGm_Draw`
//! does: his eyes on segment 8 (open, half, closed: `EnGm_UpdateEye`'s blink) and the neutral mouth
//! on segment 9, from meshes baked per eye (docs/adr/0012-actor-bakes.md). His circle shadow (35)
//! is the shape's (`ActorShape_Init`'s), which `Actor_Draw` never draws: nothing looks for his
//! floor (no `Actor_UpdateBgCheckInfo`), so `floorPoly` stays NULL.
//!
//! The whole overlay is ported.
//!
//! Params: `switchFlag` the whole params (`Flags_GetSwitch(play, this->actor.params)`: set, his
//! talk by Link; Goron City's 6, the flag of room 2's inner wall, `Bg_Breakwall` 0xA006).
//! Flags: reads switch flag `switchFlag`; reads and sets INFTABLE_B0 (a child's talk) and
//! INFTABLE_B1 (an adult's first offer).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, ObjectNeed, TextLine};
use oot_game::collision_check::*;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::inf_table;
use oot_game::item::{EQUIP_INV_SWORD_BIGGORON, EQUIP_INV_SWORD_BROKENGIANTKNIFE, EQUIP_TYPE_SWORD};
use oot_game::message::*;
use oot_game::npc::{offer_talk, process_talk_request};
use oot_game::pack::{BakeBody, BakeSegment, ForeignAnim, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_GM` (`actor_table.h`: 0x013D).
pub const ACTOR_EN_GM: i16 = 0x013D;
/// The profile's object, `OBJECT_OF1D_MAP` (`object_table.h`: 0x00C9).
pub const OBJECT: &str = "object_oF1d_map";
pub const OBJECT_OF1D_MAP: i16 = 0x00C9;
/// His animation's object, `OBJECT_GM` (`object_table.h`: 0x0126).
pub const OBJECT_GM_FILE: &str = "object_gm";
pub const OBJECT_GM: i16 = 0x0126;
const SKEL: &str = "gGoronSkel";
const ANIM: &str = "object_gm_Anim_0002B8";

/// `En_Gm_Profile`: `ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY |
/// ACTOR_FLAG_UPDATE_CULLING_DISABLED`; no draw until his object is in.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_GM,
    name: "En_Gm",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->actor.params`, whole: the switch flag `func_80A3DB04` reads.
pub const SWITCH_FLAG: Field = Field::whole("switchFlag", Kind::SwitchFlag, "set: his talk by Link (Goron City's 6: room 2's inner wall's)");

/// `INFTABLE_B0`, `INFTABLE_B1` (`save.h`: 0xB0, 0xB1).
pub const INFTABLE_B0: u16 = 0xB0;
pub const INFTABLE_B1: u16 = 0xB1;

/// En_Gm's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().note("set: his talk by Link"),
        FlagUse::new(inf_table!(INFTABLE_B0)).reads().sets().note("a child's talk: 0x304A, then 0x304B"),
        FlagUse::new(inf_table!(INFTABLE_B1)).reads().sets().note("an adult's first offer of the knife: 0x304C, then 0x304F"),
    ],
    todo: false,
    // `Object_GetSlot(OBJECT_GM)` at init: he waits for it (no draw) and never appears without it
    objects: &[ObjectNeed::new(OBJECT_GM_FILE).note("his skeleton and animation")],
    texts: &[
        TextLine::new(0x3049).cond("before his switch flag is set (the wall bombed)").note("tight fit..."),
        TextLine::new(0x304A).cond("Link a child, before he's been talked to").note("working on something cool; goes on to 0x304B"),
        TextLine::new(0x304B).cond("Link a child, once he's been talked to").note("wait five or six years"),
        TextLine::new(0x304C).cond("Link an adult without the Giant's Knife, the first time").note("a small weapon; goes on to the sale"),
        TextLine::new(0x304F).cond("Link an adult without the knife, offered before").note("the knife for 200 rupees? (a choice)"),
        TextLine::new(0x304E).cond("the Giant's Knife broken").note("a problem with its durability; goes on to the sale"),
        TextLine::new(0x304D).cond("with the Giant's Knife whole or Biggoron's Sword").note("how do you like the feel of it?"),
        TextLine::new(0x00C8).cond("answered Buy, with under 200 rupees").note("not enough rupees"),
        TextLine::new(0x3050).cond("answered Don't buy").note("come back again"),
    ],
};

/// `GI_SWORD_KNIFE` (`item.h`: 0x28): the Giant's Knife.
pub const GI_SWORD_KNIFE: i16 = 0x28;
/// The knife's price.
pub const PRICE: i16 = 200;
/// `ATTENTION_RANGE_5` (`actor.h`: 5).
const ATTENTION_RANGE_5: u8 = 5;

/// `sCylinderInit` (`ColliderCylinderInitType1`: `OC2_TYPE_1`): 100 by 120, pushing everything.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0000, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 100, height: 120, y_shift: 0, pos: [0; 3] },
};

/// `eyeTextures`.
const EYE_TEXTURES: [&str; 3] = ["gGoronCsEyeOpenTex", "gGoronCsEyeHalfTex", "gGoronCsEyeClosedTex"];
/// The mouth on segment 9.
const MOUTH_TEXTURE: &str = "gGoronCsMouthNeutralTex";

/// The bake drawn with eye `eye`.
pub fn bake_name(eye: usize) -> String {
    format!("En_Gm/eye{eye}")
}

/// The meshes `EnGm_Draw` draws: `gGoronSkel` with each eye and the neutral mouth.
pub fn bakes() -> Vec<MeshBake> {
    (0..EYE_TEXTURES.len())
        .map(|eye| MeshBake {
            name: bake_name(eye),
            object: OBJECT.into(),
            segments: vec![(0x08, BakeSegment::Texture { file: OBJECT.into(), symbol: EYE_TEXTURES[eye].into() }), (0x09, BakeSegment::Texture { file: OBJECT.into(), symbol: MOUTH_TEXTURE.into() })],
            prelude: Vec::new(),
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() },
        })
        .collect()
}

/// His animation, in `object_gm` (no skeleton there), on `gGoronSkel`.
pub fn foreign_anims() -> Vec<ForeignAnim> {
    vec![ForeignAnim { anim_file: OBJECT_GM_FILE.into(), anim: ANIM.into(), skel_file: OBJECT.into(), skel: SKEL.into() }]
}

/// `updateFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateFunc {
    /// `func_80A3D838`: waiting for `object_gm`.
    WaitForObject,
    /// `func_80A3DFBC`.
    Main,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80A3DB04`: before the switch flag.
    BeforeFlag,
    /// `func_80A3DBF4`: talking, before the flag.
    TalkBeforeFlag,
    /// `func_80A3DC44`: after the flag.
    AfterFlag,
    /// `func_80A3DD7C`: a text to its end.
    Talk,
    /// `EnGm_ProcessChoiceIndex`: the sale's choice.
    ProcessChoiceIndex,
    /// `func_80A3DF00`: the knife offered.
    OfferKnife,
    /// `func_80A3DF60`: the knife's text, then the price.
    Paid,
}

pub struct EnGm {
    pub actor: Actor,
    pub skeleton: Arc<Skeleton>,
    anim: Anim,
    /// `skelAnime` (set up once `object_gm` is in).
    pub skel_anime: Option<SkelAnimeStd>,
    pub update_func: UpdateFunc,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `gmObjectSlot` (`None`: the C's -1).
    pub gm_object_slot: Option<usize>,
    pub timer: i16,
    pub blink_timer: i16,
    pub eye_tex_index: u8,
    pub talk_pos: Vec3,
    /// `actor.draw` set (`EnGm_Draw`).
    pub drawn: bool,
}

/// `func_80A3D7C8`: 0 a child; an adult 1 without the knife (`EQUIP_INV_SWORD_BIGGORON`'s bit), 2
/// with it broken, 3 with it whole or Biggoron's Sword.
pub fn func_80a3d7c8(play: &PlayState) -> i32 {
    let s = &play.save;
    // LINK_AGE_IN_YEARS == YEARS_CHILD.
    if !s.adult {
        0
    } else if !s.check_owned_equip(EQUIP_TYPE_SWORD, EQUIP_INV_SWORD_BIGGORON) {
        1
    } else if s.check_owned_equip(EQUIP_TYPE_SWORD, EQUIP_INV_SWORD_BROKENGIANTKNIFE) {
        2
    } else {
        3
    }
}

impl EnGm {
    /// `EnGm_Init`: `sInitChain` (`ATTENTION_RANGE_5`, the arrow 30 up), `object_gm`'s slot
    /// (none: the C's `ASSERT`, logged), waiting for it.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            actor.kill();
            return oot_game::spawn::Placeholder::init(actor, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, SKEL)?, assets.animation(OBJECT_GM_FILE, ANIM)?)) })();
        let (skeleton, anim) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Gm: {e:#}");
                actor.kill();
                return oot_game::spawn::Placeholder::init(actor, play);
            }
        };
        // sInitChain.
        actor.target_mode = ATTENTION_RANGE_5;
        actor.target_arrow_offset = 30.0;
        // schema: the whole params printed.
        log::debug!("../z_en_gm.c[133] : Medi Goron [{}]", actor.params);
        let gm_object_slot = play.object_ctx.get_index(OBJECT_GM);
        if gm_object_slot.is_none() {
            log::error!("There is no model bank!! (Medi Goron) (../z_en_gm.c 145: ASSERT)");
        }
        Box::new(EnGm {
            actor,
            skeleton,
            anim,
            skel_anime: None,
            update_func: UpdateFunc::WaitForObject,
            action: Action::BeforeFlag,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            gm_object_slot,
            timer: 0,
            blink_timer: 0,
            eye_tex_index: 0,
            talk_pos: Vec3::ZERO,
            drawn: false,
        })
    }

    /// `PARAMS`: the switch flag.
    fn switch_flag(&self) -> i32 {
        SWITCH_FLAG.get(self.actor.params) as i32
    }

    /// `func_80A3D838`: once `object_gm` is loaded: culled from now on, the skeleton playing
    /// `object_gm_Anim_0002B8` looped, the draw, the cylinder (`Collider_SetCylinderType1`), the
    /// circle shadow (35), scale 0.05, immovable, the eyes open with 20 to the blink, text 0x3049,
    /// `func_80A3DB04`, still (speed 0, gravity -1, no fall: nothing moves him).
    fn func_80a3d838(&mut self, play: &mut PlayState) {
        if !self.gm_object_slot.is_some_and(|s| play.object_ctx.is_loaded(s)) {
            return;
        }
        self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        let mut skel = SkelAnimeStd::init_flex(self.skeleton.limbs.len(), None);
        let anim = self.anim.clone();
        let last = anim.last_frame();
        skel.change(anim, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
        self.skel_anime = Some(skel);
        self.drawn = true;
        self.collider = ColliderCylinder::new(&CYLINDER_INIT);
        self.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 35.0);
        self.actor.scale = Vec3::splat(0.05);
        self.actor.col_chk_info.mass = MASS_IMMOVABLE;
        self.eye_tex_index = 0;
        self.blink_timer = 20;
        self.actor.text_id = 0x3049;
        self.update_func = UpdateFunc::Main;
        self.action = Action::BeforeFlag;
        self.actor.speed_xz = 0.0;
        self.actor.gravity = -1.0;
        self.actor.velocity.y = 0.0;
    }

    /// `EnGm_UpdateEye`: the blink: open, half, closed, then 20 to 79 frames open.
    fn update_eye(&mut self, play: &mut PlayState) {
        if self.blink_timer != 0 {
            self.blink_timer -= 1;
        } else {
            self.eye_tex_index += 1;
            if self.eye_tex_index >= 3 {
                self.eye_tex_index = 0;
                self.blink_timer = (play.rand.zero_float(60.0) + 20.0) as i32 as i16;
            }
        }
    }

    /// `EnGm_SetTextID`: his text by `func_80A3D7C8`.
    pub fn set_text_id(&mut self, play: &PlayState) {
        match func_80a3d7c8(play) {
            0 => self.actor.text_id = if play.save.get_inf_table(INFTABLE_B0) { 0x304B } else { 0x304A },
            1 => self.actor.text_id = if play.save.get_inf_table(INFTABLE_B1) { 0x304F } else { 0x304C },
            2 => self.actor.text_id = 0x304E,
            3 => self.actor.text_id = 0x304D,
            _ => {}
        }
    }

    /// Link touching his cylinder or within 100 of his talk point.
    fn near(&self, play: &PlayState) -> bool {
        let p = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO);
        let dx = self.talk_pos.x - p.x;
        let dz = self.talk_pos.z - p.z;
        self.collider.base.oc_flags1 & OC1_HIT != 0 || (dx * dx) + (dz * dz) < 100.0 * 100.0
    }

    /// `Actor_OfferTalk(&this->actor, play, 415.0f)`, after clearing `AC_HIT` (the C's, on a
    /// cylinder that takes no hits).
    fn offer_talk(&mut self, play: &mut PlayState) {
        self.collider.base.ac_flags &= !AC_HIT;
        let a = self.actor.clone();
        offer_talk(play, &a, 415.0);
    }

    /// `func_80A3DB04`: the flag set, his text by Link (`func_80A3DC44`); else a talk started
    /// goes to `func_80A3DBF4`, and near him he offers it.
    fn func_80a3db04(&mut self, play: &mut PlayState) {
        if play.flags.get_switch(self.switch_flag()) {
            self.set_text_id(play);
            self.action = Action::AfterFlag;
        } else if process_talk_request(&mut self.actor) {
            self.action = Action::TalkBeforeFlag;
        } else if self.near(play) {
            self.offer_talk(play);
        }
    }

    /// `func_80A3DBF4`: the text done, back to `func_80A3DB04`.
    fn func_80a3dbf4(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            self.action = Action::BeforeFlag;
        }
    }

    /// `func_80A3DC44`: his text by Link; a talk started: a child sets `INFTABLE_B0` and an adult
    /// with the knife or the sword just talks (`func_80A3DD7C`); an adult without it sets
    /// `INFTABLE_B1` and one with it broken gets the choice (`EnGm_ProcessChoiceIndex`). Near him
    /// he offers the talk.
    fn func_80a3dc44(&mut self, play: &mut PlayState) {
        self.set_text_id(play);
        if process_talk_request(&mut self.actor) {
            match func_80a3d7c8(play) {
                0 => {
                    play.save.set_inf_table(INFTABLE_B0);
                    self.action = Action::Talk;
                }
                3 => self.action = Action::Talk,
                1 => {
                    play.save.set_inf_table(INFTABLE_B1);
                    self.action = Action::ProcessChoiceIndex;
                }
                2 => self.action = Action::ProcessChoiceIndex,
                _ => {}
            }
            // Every case returns; `this->actionFunc = EnGm_ProcessChoiceIndex` after the switch is
            // never reached.
            return;
        }
        if self.near(play) {
            self.offer_talk(play);
        }
    }

    /// `func_80A3DD7C`: the text done (or an event's), back to `func_80A3DC44`; an event's text
    /// is closed (`MSGMODE_TEXT_CLOSING`, 4 frames).
    fn func_80a3dd7c(&mut self, play: &mut PlayState) {
        let dialog_state = play.message_state();
        if (dialog_state == TEXT_STATE_DONE || dialog_state == TEXT_STATE_EVENT) && play.message_should_advance() {
            self.action = Action::AfterFlag;
            if dialog_state == TEXT_STATE_EVENT {
                play.msg_ctx.msg_mode = MSGMODE_TEXT_CLOSING;
                play.msg_ctx.state_timer = 4;
            }
        }
    }

    /// `EnGm_ProcessChoiceIndex`: "yes": under 200 rupees text 0xC8, else the knife offered; "no":
    /// text 0x3050.
    fn process_choice_index(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_CHOICE && play.message_should_advance() {
            match play.msg_ctx.choice_index {
                0 => {
                    if play.save.rupees < PRICE {
                        play.continue_textbox(0xC8);
                        self.action = Action::Talk;
                    } else {
                        let a = self.actor.clone();
                        offer_get_item_range(play, &a, GI_SWORD_KNIFE, 415.0, 10.0);
                        self.action = Action::OfferKnife;
                    }
                }
                1 => {
                    play.continue_textbox(0x3050);
                    self.action = Action::Talk;
                }
                _ => {}
            }
        }
    }

    /// `func_80A3DF00`: Link has the knife (a parent): on to its text; else offered again.
    fn func_80a3df00(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.action = Action::Paid;
        } else {
            let a = self.actor.clone();
            offer_get_item_range(play, &a, GI_SWORD_KNIFE, 415.0, 10.0);
        }
    }

    /// `func_80A3DF60`: the knife's text done, the 200 rupees taken (`Rupees_ChangeBy(-200)`).
    fn func_80a3df60(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            oot_game::item::rupees_change_by(&mut play.save, -PRICE);
            self.action = Action::AfterFlag;
        }
    }

    /// `func_80A3DFBC`: the timer, the action, the focus's rotation his, the blink, the animation,
    /// his cylinder pushing (OC).
    fn func_80a3dfbc(&mut self, play: &mut PlayState) {
        self.timer = self.timer.wrapping_add(1);
        match self.action {
            Action::BeforeFlag => self.func_80a3db04(play),
            Action::TalkBeforeFlag => self.func_80a3dbf4(play),
            Action::AfterFlag => self.func_80a3dc44(play),
            Action::Talk => self.func_80a3dd7c(play),
            Action::ProcessChoiceIndex => self.process_choice_index(play),
            Action::OfferKnife => self.func_80a3df00(play),
            Action::Paid => self.func_80a3df60(play),
        }
        self.actor.focus_rot = self.actor.world_rot;
        self.update_eye(play);
        if let Some(s) = &mut self.skel_anime {
            s.update();
        }
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }

    /// `func_80A3E090`: from the model matrix, the cylinder 2600 in front of him, the talk point
    /// 4300 in front, the focus 3800 in front and 100 up.
    fn func_80a3e090(&mut self) {
        let m = actor_draw_matrix(&RenderState::of(&self.actor));
        let at = |z: f32| m.transform_point3(Vec3::new(0.0, 0.0, z));
        let c = at(2600.0);
        self.collider.dim.pos = [c.x as i16, c.y as i16, c.z as i16];
        self.talk_pos = at(4300.0);
        self.actor.focus_pos = at(3800.0);
        self.actor.focus_pos.y += 100.0;
    }
}

impl ActorImpl for EnGm {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnGm_Update`: `updateFunc`.
    fn update(&mut self, play: &mut PlayState) {
        match self.update_func {
            UpdateFunc::WaitForObject => self.func_80a3d838(play),
            UpdateFunc::Main => self.func_80a3dfbc(play),
        }
    }

    /// `EnGm_Destroy`: the cylinder (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `EnGm_Draw`'s `func_80A3E090`, once he draws.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.drawn {
            self.func_80a3e090();
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        if let Some(s) = &self.skel_anime {
            rs.joints = Some(eng_anim::anim::JointTable { rot: s.joint_table.clone(), face: 0 });
        }
        rs.switches = vec![self.drawn as u32, self.eye_tex_index.min(2) as u32];
        rs
    }

    /// `EnGm_Draw`: `Gfx_SetupDL_25Opa`, the eye on segment 8, the neutral mouth on 9, the
    /// skeleton (`SkelAnime_DrawFlexOpa`, no limb callbacks).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.first() != Some(&1) {
            return;
        }
        let Some(joints) = &rs.joints else { return };
        let bones = self.skeleton.pose(joints);
        let eye = rs.switches.get(1).copied().unwrap_or(0) as usize;
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(eye.min(2)))), transform: actor_draw_matrix(rs), bones, params: Default::default() });
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
