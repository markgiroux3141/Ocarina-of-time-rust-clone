//! `En_Ms` (`ovl_En_Ms/z_en_ms.c`): the Magic Bean Salesman, sitting by the fence at the foot of
//! Zora's River (a child's only: an adult's init kills him).
//!
//! He sits eating (`gBeanSalesmanEatingAnim`, looped), and offers to talk within 90 while Link is
//! in front of him (`EnMs_Wait`: within 0x2000 of his facing). His text is the offer for the beans
//! bought so far (`EnMs_SetOfferText`: `sOfferTextIDs[BEANS_BOUGHT]`, 0x405E to 0x4067, each a
//! choice; 0x406B once 10 are bought; a mask's reaction first). "Yes" (`EnMs_Talk`): with fewer
//! rupees than `sPrices[BEANS_BOUGHT]` (10, 20, ... 100) text 0x4069; else a bean is offered
//! (`Actor_OfferGetItem`, `GI_MAGIC_BEAN`, 90 across and 10 up) until Link takes it
//! (`EnMs_Sell`), and the price is charged then (`Rupees_ChangeBy`), before `Item_Give` counts the
//! bean (`BEANS_BOUGHT`, `AMMO(ITEM_MAGIC_BEAN + 1)`, goes up by one with the bean count). After
//! the get's text, 0x406C (`EnMs_TalkAfterPurchase`). "No": 0x4068.
//!
//! His cylinder (22 by 37) pushes (OC); it takes the player's hits (`AC_TYPE_PLAYER`) but is never
//! put up for them (no `CollisionCheck_SetAC`). In the ending (`ENTR_LON_LON_RANCH_0`, layer 8) he
//! rides his carpet: he falls and finds his floor (`Actor_MoveXZGravity`,
//! `Actor_UpdateBgCheckInfo`). Drawn with `Gfx_SetupDL_25Opa` and his skeleton, no limb callbacks
//! (`object_ms`'s `gBeanSalesmanSkel` mesh: no segments to bake); his circle shadow (35) is the
//! shape's.
//!
//! The whole overlay is ported. Left out: the print.
//!
//! Params: none read (Zora's River places him with 0xFFFF).
//! Flags: none (the beans bought are `BEANS_BOUGHT`, an inventory count: `Item_Give` raises it).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind, TextLine};
use oot_game::collision_check::*;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::ITEM_MAGIC_BEAN;
use oot_game::message::*;
use oot_game::npc::{offer_talk, process_talk_request};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;

use crate::player::Player;

/// `ACTOR_EN_MS` (`actor_table.h`: 0x013E).
pub const ACTOR_EN_MS: i16 = 0x013E;
/// `OBJECT_MS` (`object_table.h`: 0x0127).
pub const OBJECT: &str = "object_ms";
const SKEL: &str = "gBeanSalesmanSkel";
const ANIM: &str = "gBeanSalesmanEatingAnim";

/// `En_Ms_Profile`: `ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_MS, name: "En_Ms", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT, schema: &SCHEMA };

/// The placed params, read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed as 0xFFFF; read by nothing");

/// En_Ms's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[],
    todo: false,
    objects: &[],
    texts: &[
        TextLine::new(0x405E).cond("no beans bought").note("a bean for 10 rupees? (a choice)"),
        TextLine::new(0x405F).cond("1 bean bought").note("a bean for 20 rupees? (a choice)"),
        TextLine::new(0x4060).cond("2 beans bought").note("a bean for 30 rupees? (a choice)"),
        TextLine::new(0x4061).cond("3 beans bought").note("a bean for 40 rupees? (a choice)"),
        TextLine::new(0x4062).cond("4 beans bought").note("a bean for 50 rupees? (a choice)"),
        TextLine::new(0x4063).cond("5 beans bought").note("a bean for 60 rupees? (a choice)"),
        TextLine::new(0x4064).cond("6 beans bought").note("a bean for 70 rupees? (a choice)"),
        TextLine::new(0x4065).cond("7 beans bought").note("a bean for 80 rupees? (a choice)"),
        TextLine::new(0x4066).cond("8 beans bought").note("a bean for 90 rupees? (a choice)"),
        TextLine::new(0x4067).cond("9 beans bought").note("a bean for 100 rupees? (a choice)"),
        TextLine::new(0x406B).cond("10 beans bought").note("sold out"),
        TextLine::new(0x4069).cond("answered yes, without the price").note("not enough rupees"),
        TextLine::new(0x4068).cond("answered no"),
        TextLine::new(0x406C).cond("after the bean's get").note("after the sale"),
        TextLine::new(0x7184).cond("Link wearing the Keaton, Goron or Truth mask").note("a mask's reaction"),
        TextLine::new(0x7185).cond("Link wearing the Spooky Mask").note("a mask's reaction"),
        TextLine::new(0x7186).cond("Link wearing the Skull, Bunny or Gerudo mask").note("a mask's reaction"),
        TextLine::new(0x7187).cond("Link wearing the Zora Mask").note("a mask's reaction"),
    ],
};

/// `GI_MAGIC_BEAN` (`item.h`: 0x16).
pub const GI_MAGIC_BEAN: i16 = 0x16;
/// `ATTENTION_RANGE_2` (`actor.h`: 2).
const ATTENTION_RANGE_2: u8 = 2;
/// `ENTR_LON_LON_RANCH_0` (`entrance_table.h`: 0x157): the ending's carpet ride, layer 8.
const ENTR_LON_LON_RANCH_0: u16 = 0x157;

/// `sPrices`: a bean's price by the beans bought.
pub const PRICES: [i16; 10] = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
/// `sOfferTextIDs`: the offer by the beans bought.
pub const OFFER_TEXT_IDS: [u16; 10] = [0x405E, 0x405F, 0x4060, 0x4061, 0x4062, 0x4063, 0x4064, 0x4065, 0x4066, 0x4067];

/// `sMaskReactionSetTextIds[MASK_REACTION_SET_MAGIC_BEAN_SALESMAN]` (`z_face_reaction.c`), by
/// `Player_GetMask` (`PLAYER_MASK_NONE` .. `PLAYER_MASK_TRUTH`).
const MASK_REACTION_MAGIC_BEAN_SALESMAN: [u16; 9] = [0x0000, 0x7184, 0x7186, 0x7185, 0x7186, 0x7184, 0x7187, 0x7186, 0x7184];

/// `sCylinderInit` (`ColliderCylinderInitType1`: `OC2_TYPE_1`): 22 by 37.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 22, height: 37, y_shift: 0, pos: [0; 3] },
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnMs_Wait`.
    Wait,
    /// `EnMs_Talk`.
    Talk,
    /// `EnMs_Sell`.
    Sell,
    /// `EnMs_TalkAfterPurchase`.
    TalkAfterPurchase,
}

pub struct EnMs {
    pub actor: Actor,
    pub skeleton: Arc<Skeleton>,
    pub skel_anime: SkelAnimeStd,
    pub action: Action,
    pub collider: ColliderCylinder,
    pub active_timer: i16,
}

/// `BEANS_BOUGHT` (`save.h`: `AMMO(ITEM_MAGIC_BEAN + 1)`).
pub fn beans_bought(play: &PlayState) -> i8 {
    play.save.ammo(ITEM_MAGIC_BEAN + 1)
}

// shared: MaskReaction_GetTextId
/// `MaskReaction_GetTextId(play, MASK_REACTION_SET_MAGIC_BEAN_SALESMAN)` (`z_face_reaction.c`):
/// the salesman's row by the mask Link wears.
fn mask_reaction_get_text_id(play: &PlayState) -> u16 {
    let mask = play.player.and_then(|h| play.actors.downcast::<Player>(h)).map_or(0, |p| p.current_mask) as usize;
    MASK_REACTION_MAGIC_BEAN_SALESMAN.get(mask).copied().unwrap_or(0)
}

impl EnMs {
    /// `EnMs_Init`: an adult's killed; `sInitChain` (`ATTENTION_RANGE_2`, the arrow 500 up), the
    /// skeleton eating, the cylinder, the circle shadow (35), scale 0.015, immovable, still with
    /// gravity -1, the offer's text, `EnMs_Wait`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // LINK_AGE_IN_YEARS != YEARS_CHILD.
        if play.save.adult {
            actor.kill();
            return oot_game::spawn::Placeholder::init(actor, play);
        }
        let Some(assets) = play.assets.clone() else {
            actor.kill();
            return oot_game::spawn::Placeholder::init(actor, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, SKEL)?, assets.animation(OBJECT, ANIM)?)) })();
        let (skeleton, anim) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Ms: {e:#}");
                actor.kill();
                return oot_game::spawn::Placeholder::init(actor, play);
            }
        };
        // sInitChain.
        actor.target_mode = ATTENTION_RANGE_2;
        actor.target_arrow_offset = 500.0;
        // SkelAnime_InitFlex(.., &gBeanSalesmanSkel, &gBeanSalesmanEatingAnim, .., 9): looped.
        let skel_anime = SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anim));
        let mut this = EnMs { actor, skeleton, skel_anime, action: Action::Wait, collider: ColliderCylinder::new(&CYLINDER_INIT), active_timer: 0 };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 35.0);
        this.actor.scale = Vec3::splat(0.015);
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.speed_xz = 0.0;
        this.actor.velocity.y = 0.0;
        this.actor.gravity = -1.0;
        this.set_offer_text(play);
        this.action = Action::Wait;
        Box::new(this)
    }

    /// `EnMs_SetOfferText`: a mask's reaction, else the offer for the beans bought (0x406B from
    /// 10 on).
    pub fn set_offer_text(&mut self, play: &PlayState) {
        self.actor.text_id = mask_reaction_get_text_id(play);
        if self.actor.text_id == 0 {
            let bought = beans_bought(play);
            self.actor.text_id = if bought >= 10 { 0x406B } else { OFFER_TEXT_IDS[bought.max(0) as usize] };
        }
    }

    /// `sPrices[BEANS_BOUGHT]`.
    fn price(play: &PlayState) -> i16 {
        // (Ten bought: the offer is 0x406B, no choice, so this isn't reached; the C would read
        // past the table.)
        PRICES[(beans_bought(play).max(0) as usize).min(PRICES.len() - 1)]
    }

    /// `EnMs_Wait`: the offer's text kept up; a talk started goes to `EnMs_Talk`; within 90 and
    /// in front of him (0x2000 of his facing) he offers it.
    fn wait(&mut self, play: &mut PlayState) {
        let yaw_diff = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        self.set_offer_text(play);
        if process_talk_request(&mut self.actor) {
            self.action = Action::Talk;
        } else if self.actor.xz_dist_to_player < 90.0 && (yaw_diff as i32).abs() < 0x2000 {
            let a = self.actor.clone();
            offer_talk(play, &a, 90.0);
        }
    }

    /// `EnMs_Talk`: a text's end back to `EnMs_Wait`; on the choice, "yes": under the price
    /// 0x4069, else the bean offered (`EnMs_Sell`); "no": 0x4068.
    fn talk(&mut self, play: &mut PlayState) {
        let dialog_state = play.message_state();
        if dialog_state != TEXT_STATE_CHOICE {
            if dialog_state == TEXT_STATE_DONE && play.message_should_advance() {
                self.action = Action::Wait;
            }
        } else if play.message_should_advance() {
            match play.msg_ctx.choice_index {
                0 => {
                    if play.save.rupees < Self::price(play) {
                        play.continue_textbox(0x4069);
                        return;
                    }
                    let a = self.actor.clone();
                    offer_get_item_range(play, &a, GI_MAGIC_BEAN, 90.0, 10.0);
                    self.action = Action::Sell;
                }
                1 => play.continue_textbox(0x4068),
                _ => {}
            }
        }
    }

    /// `EnMs_Sell`: Link has the bean (a parent): the price charged, on to
    /// `EnMs_TalkAfterPurchase`; else offered again. (The get's `Item_Give` comes with its text,
    /// after this: `Item_CheckObtainability(ITEM_MAGIC_BEAN)` is always `ITEM_NONE`, the fanfare
    /// get, so the price is the one before the bean is counted.)
    fn sell(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            let price = Self::price(play);
            oot_game::item::rupees_change_by(&mut play.save, -price);
            self.actor.parent = None;
            self.action = Action::TalkAfterPurchase;
        } else {
            let a = self.actor.clone();
            offer_get_item_range(play, &a, GI_MAGIC_BEAN, 90.0, 10.0);
        }
    }

    /// `EnMs_TalkAfterPurchase`: the get's text done, 0x406C, back to `EnMs_Talk`.
    fn talk_after_purchase(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            play.continue_textbox(0x406C);
            self.action = Action::Talk;
        }
    }
}

impl ActorImpl for EnMs {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnMs_Update`: the timer, the focus 20 up, the arrow, the scale, the animation, the action;
    /// the ending's carpet ride; the cylinder pushing (OC).
    fn update(&mut self, play: &mut PlayState) {
        self.active_timer = self.active_timer.wrapping_add(1);
        self.actor.set_focus(20.0);
        self.actor.target_arrow_offset = 500.0;
        self.actor.scale = Vec3::splat(0.015);
        self.skel_anime.update();
        match self.action {
            Action::Wait => self.wait(play),
            Action::Talk => self.talk(play),
            Action::Sell => self.sell(play),
            Action::TalkAfterPurchase => self.talk_after_purchase(play),
        }
        // The ending: ENTR_LON_LON_RANCH_0, layer 8.
        if play.save.entrance_index == ENTR_LON_LON_RANCH_0 && play.save.scene_layer == 8 {
            self.actor.move_forward();
            self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        }
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }

    /// `EnMs_Destroy`: the cylinder (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel_anime.joint_table.clone(), face: 0 });
        rs
    }

    /// `EnMs_Draw`: `Gfx_SetupDL_25Opa`, the skeleton (`SkelAnime_DrawFlexOpa`, no callbacks).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let bones = self.skeleton.pose(joints);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::mesh(OBJECT, SKEL)), transform: actor_draw_matrix(rs), bones, params: Default::default() });
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
