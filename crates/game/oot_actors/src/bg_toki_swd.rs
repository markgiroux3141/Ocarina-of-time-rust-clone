//! `Bg_Toki_Swd` (`ovl_Bg_Toki_Swd/z_bg_toki_swd.c`): the Master Sword in the Pedestal of Time,
//! behind the Door of Time (the Temple of Time's room 0), and its three cutscenes.
//!
//! - **The reveal.** Facing it within 800, outside the cutscene layers and any cutscene, the
//!   first time (`EVENTCHKINF_REVEALED_MASTER_SWORD` clear), it sets the flag and plays
//!   `gRevealMasterSwordCs`.
//! - **The pull and the putting back.** A child, or an adult with `EVENTCHKINF_55` (the Prelude
//!   of Light learned), facing it within 0x2000 is offered it to carry (`Actor_OfferCarry`).
//!   Player's pedestal branch takes it (`Player_ActionHandler_2`, `func_8083A0F4`, which makes
//!   Player its parent). Then the child gets the Master Sword (`Item_Give`) and
//!   `gPullMasterSwordCs`, the adult `gPlaceMasterSwordCs`; both with `NA_BGM_MASTER_SWORD`.
//! - **In the cutscene** it waits to be taken again (Player's mode 18, `func_80851A50`, parents it
//!   on the child's frame 87 or the adult's 70), keeping itself Player's `interactRangeActor`
//!   meanwhile. Then the child's sword is gone (not drawn, `NA_SE_IT_SWORD_PUTAWAY_STN`) and the
//!   adult's is drawn in the pedestal; and on cutscene flag 1 the temple dims
//!   (`roomCtx.drawParams[0]` up by 5 a frame to 0xFF, which `Scene_DrawConfigTempleOfTime`
//!   reads).
//!
//! An adult sees no sword at first (`actor.draw = NULL`): his is in his hand. On the cutscene
//! layer 5 (the credits) the temple starts dark (`drawParams[0]` 0xFF) and lightens by 1 a frame.
//! The body (a cylinder) only pushes Link (OC).
//!
//! The whole overlay is ported. Not drawn, as elsewhere: the blade's highlight
//! (`func_8002EBCC`'s look-at for its texgen: the renderer's texgen reads the view).
//!
//! Params: none read (the scene places 0xFFFF).
//! Flags: reads and sets EVENTCHKINF_REVEALED_MASTER_SWORD (the reveal's cutscene, once); reads
//! EVENTCHKINF_55 (an adult may put the sword back); reads the cutscene's flag 1 (the temple
//! dims).

use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::bgm::{seq_cmd1, start_seq};
use oot_game::collision_check::*;
use oot_game::event_chk_inf;
use oot_game::item::ITEM_SWORD_MASTER;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::scene_table::gfx_tex_scroll;

/// `ACTOR_BG_TOKI_SWD` (`actor_table.h`: 0x006C).
pub const ACTOR_BG_TOKI_SWD: i16 = 0x006C;
pub const OBJECT: &str = "object_toki_objects";

/// `Bg_Toki_Swd_Profile`: `FLAGS` `ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_TOKI_SWD, name: "Bg_Toki_Swd", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `EVENTCHKINF_REVEALED_MASTER_SWORD`, `EVENTCHKINF_55` (`save.h`: 0x4F, 0x55).
pub const EVENTCHKINF_REVEALED_MASTER_SWORD: u16 = 0x4F;
pub const EVENTCHKINF_55: u16 = 0x55;
/// `SEQ_PLAYER_BGM_MAIN`, `NA_BGM_MASTER_SWORD` (`sequence.h`; `sequence_table.h`'s
/// `Sequence_83`).
const SEQ_PLAYER_BGM_MAIN: u8 = 0;
const NA_BGM_MASTER_SWORD: u16 = 0x53;

/// `params`: placed as 0xFFFF, read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed as 0xFFFF; read by nothing");

/// Bg_Toki_Swd's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_REVEALED_MASTER_SWORD)).reads().sets().note("the reveal's cutscene, the first time Link faces it"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_55)).reads().note("an adult may put the sword back (the Prelude of Light learned)"),
        FlagUse::new(Flag::Other("CutsceneFlags 1")).reads().note("the temple dims as the sword is pulled or put back"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

/// The sword in the pedestal: `object_toki_objects_DL_001BD0`, segment 8 scrolling.
pub const BAKE: &str = "Bg_Toki_Swd/sword";

/// `Gfx_TexScroll(gfxCtx, 0, -(gameplayFrames % 0x80), 32, 32)`.
fn scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_tex_scroll(0, (frames % 0x80).wrapping_neg(), 32, 32)
}

/// The mesh `BgTokiSwd_Draw` draws.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(0x08, BakeSegment::Dynamic(scroll(0)))],
        prelude: Vec::new(),
        body: BakeBody::DLists(vec![(OBJECT.into(), "object_toki_objects_DL_001BD0".into())]),
    }]
}

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1 | OC2_UNK1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 10, height: 70, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 10, cyl_radius: 35, cyl_height: 100, mass: MASS_IMMOVABLE };

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_808BAF40`: in the pedestal: the reveal, then offered to the child (or the adult with
    /// 0x55) until taken.
    Func808BAF40,
    /// `func_808BB0AC`: in the cutscene, until Player takes it out or lets go of it.
    Func808BB0AC,
    /// `func_808BB128`: done; the temple dims on cutscene flag 1.
    Func808BB128,
}

pub struct BgTokiSwd {
    pub actor: Actor,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `actor.draw != NULL`.
    pub drawn: bool,
}

/// `IS_CUTSCENE_LAYER`.
fn is_cutscene_layer(play: &PlayState) -> bool {
    play.save.scene_layer >= 4
}

/// `play->roomCtx.drawParams[0]`.
fn draw_params_0(play: &mut PlayState) -> Option<&mut i16> {
    play.scene.as_mut().map(|s| &mut s.draw.room_draw_params[0])
}

impl BgTokiSwd {
    /// `BgTokiSwd_Init`: scale 0.025, `shape.yOffset` 800; not drawn for an adult; on layer 5 the
    /// temple starts dark; the body.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 25).
        actor.scale = Vec3::splat(0.025);
        actor.shape_y_offset = 800.0;
        let drawn = !play.save.adult;
        if play.save.scene_layer == 5
            && let Some(p) = draw_params_0(play)
        {
            *p = 0xFF;
        }
        let mut collider = ColliderCylinder::new(&CYLINDER_INIT);
        collider.update(&actor);
        actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
        Box::new(BgTokiSwd { actor, action: Action::Func808BAF40, collider, drawn })
    }

    /// `func_808BAF40`.
    fn func_808baf40(&mut self, play: &mut PlayState) {
        if !play.save.get_event_chk_inf(EVENTCHKINF_REVEALED_MASTER_SWORD)
            && !is_cutscene_layer(play)
            && crate::bg_treemouth::actor_is_facing_and_near_player(&self.actor, 800.0, 0x7530)
            && !play.play_in_cs_mode()
        {
            play.save.set_event_chk_inf(EVENTCHKINF_REVEALED_MASTER_SWORD);
            play.cs_ctx.segment = play.cutscene_script("gRevealMasterSwordCs");
            play.save.cutscene_trigger = 1;
        }
        if !play.save.adult || play.save.get_event_chk_inf(EVENTCHKINF_55) {
            if oot_game::get_item::actor_has_parent(&self.actor) {
                if !play.save.adult {
                    oot_game::item::item_give(&mut play.save, Some(&mut play.audio), ITEM_SWORD_MASTER);
                    play.cs_ctx.segment = play.cutscene_script("gPullMasterSwordCs");
                } else {
                    play.cs_ctx.segment = play.cutscene_script("gPlaceMasterSwordCs");
                }
                // SEQCMD_STOP_SEQUENCE(SEQ_PLAYER_BGM_MAIN, 0), then
                // SEQCMD_PLAY_SEQUENCE(SEQ_PLAYER_BGM_MAIN, 0, 0, NA_BGM_MASTER_SWORD).
                play.audio.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_MAIN, 0));
                play.audio.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_MASTER_SWORD));
                play.save.cutscene_trigger = 1;
                self.actor.parent = None;
                self.action = Action::Func808BB0AC;
            } else if crate::en_eiyer::is_facing_player(&self.actor, 0x2000) {
                oot_game::get_item::offer_pick_up(play, &self.actor);
            }
        }
        if play.save.scene_layer == 5
            && let Some(p) = draw_params_0(play)
        {
            if *p > 0 {
                *p -= 1;
            } else {
                *p = 0;
            }
        }
    }

    /// `func_808BB0AC`: once Player has it again (the sword pulled or put back), the child's is no
    /// longer drawn (its sound) and the adult's is; meanwhile it stays Player's
    /// `interactRangeActor`.
    fn func_808bb0ac(&mut self, play: &mut PlayState) {
        if oot_game::get_item::actor_has_parent(&self.actor) {
            if !play.save.adult {
                oot_game::actor_ctx::audio_play_actor_sfx2(play, oot_game::audio::sfx::NA_SE_IT_SWORD_PUTAWAY_STN);
                self.drawn = false;
            } else {
                self.drawn = true;
            }
            self.action = Action::Func808BB128;
        } else if let Some(me) = play.cur_actor
            && let Some(pi) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut())
        {
            pi.set_interact_range_actor(me);
        }
    }

    /// `func_808BB128`: on cutscene flag 1 the temple dims, `drawParams[0]` up by 5 while under
    /// 0xFF.
    fn func_808bb128(&mut self, play: &mut PlayState) {
        if play.flags_get_env(1)
            && let Some(p) = draw_params_0(play)
            && *p < 0xFF
        {
            *p += 5;
        }
    }
}

impl ActorImpl for BgTokiSwd {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgTokiSwd_Update`: the action, then the body pushes (OC).
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Func808BAF40 => self.func_808baf40(play),
            Action::Func808BB0AC => self.func_808bb0ac(play),
            Action::Func808BB128 => self.func_808bb128(play),
        }
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }

    /// `BgTokiSwd_Draw`: `object_toki_objects_DL_001BD0` (`Gfx_SetupDL_25Opa`), segment 8
    /// scrolling down a texel a frame.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if !self.drawn {
            return;
        }
        let mut sv = SegmentValues::default();
        sv.read(0x08, &scroll(play.gameplay_frames));
        let m = oot_game::play::actor_draw_matrix(rs);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
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
