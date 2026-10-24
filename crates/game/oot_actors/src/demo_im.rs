//! `Demo_Im` (`ovl_Demo_Im/z_demo_im.c`): Impa. Ported for the child's story (the user's scope,
//! GAME-07): types 5, 3 and 4.
//! - **Type 5, Zelda's courtyard:** hidden until Link has Zelda's Letter
//!   (`EVENTCHKINF_OBTAINED_ZELDAS_LETTER`), then standing at the courtyard's east end, her head on
//!   Link. Link within 130 west of her (and no cutscene): `gZeldasCourtyardLullabyCs`,
//!   `EVENTCHKINF_59` and Zelda's Lullaby given at once (`Item_Give`). The script's cues (channel 5)
//!   move her: shown (2), whistling with her hand pointing (7), still (8), the whistle put down (9).
//!   Its ocarina texts teach the song and play it back; its terminator leaves for Hyrule Field. Once
//!   the lullaby is known, entering again finds her by the entrance: west of her by 30 she talks
//!   (0x708E) and on its close sends Link out to the field (`ENTR_HYRULE_FIELD_0`, a black
//!   starburst). Gone once Zelda has fled (`EVENTCHKINF_ZELDA_FLED_CASTLE`).
//! - **Type 3:** the lullaby's whistle on its own (the same cues).
//! - **Type 4, Hyrule Field's layer 12** (`gHyruleFieldImpaCs`): shown (2), the Deku Nut thrown
//!   (10: `En_Arrow`'s `ARROW_CS_NUT`, 30 in front of her, as the throw ends), gone (11).
//!
//! Her head and torso turn to Link (`Npc_TrackPoint`, preset 12) while she can talk; she blinks
//! every 60 to 120 frames. Not ported: types 0 to 2 and 6 (the Chamber of Sages' Shadow Medallion,
//! Ganon's sealing, the ending: `Door_Warp1`'s sages' warp, `Demo_6K`, the translucent draw); they
//! log and stay hidden.
//!
//! Params: `params` whole, the type (0 and any other not below: the Chamber of Sages, the Shadow
//! Medallion's; 2: Ganon's sealing and the Shadow trial's barrier, `Demo_6K`'s light ball; 3: the
//! lullaby's whistle; 4: Hyrule Field's Deku Nut; 5: Zelda's courtyard; 6: the ending).
//! `world.rot.x`: a rotation (Ganon's Castle's, 0x2000).
//! Flags: type 5 sets EVENTCHKINF_59 (the lullaby's cutscene seen) and reads it,
//! EVENTCHKINF_OBTAINED_ZELDAS_LETTER and EVENTCHKINF_ZELDA_FLED_CASTLE; an unreachable branch sets
//! EVENTCHKINF_4C (after reading the letter's flag). Type 0 reads `gSaveContext.chamberCutsceneNum`
//! (`CHAMBER_CS_SHADOW`: the Shadow Medallion's cutscene; not ported).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawImage, DrawParams, MeshKey, SourceImage};
use eng_math::smooth_step_to_s;
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_INSIDE_CULLING_VOLUME, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_GROUND, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source, TextLine};
use oot_game::event_chk_inf;
use oot_game::audio::sfx::{NA_SE_PL_WALK_GROUND, SfxPos};
use oot_game::collision_check::*;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::message::TEXT_STATE_CLOSING;
use oot_game::npc::{NpcTrack, offer_talk_default, process_talk_request, track_point};
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;
use oot_game::surface::SurfaceType;
use oot_game::transition::{TCA_STARBURST, TCC_BLACK, TCS_FAST, TRANS_TRIGGER_START, trans_type_circle};

use crate::en_arrow::{ACTOR_EN_ARROW, ARROW_CS_NUT};
use crate::en_heishi1::EVENTCHKINF_ZELDA_FLED_CASTLE;
use crate::en_zl4::EVENTCHKINF_OBTAINED_ZELDAS_LETTER;

/// `ACTOR_DEMO_IM` (`actor_table.h`: 0x00A9).
pub const ACTOR_DEMO_IM: i16 = 0x00A9;
pub const OBJECT: &str = "object_im";

/// `Demo_Im_Profile`: `FLAGS` `ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_DEMO_IM, name: "Demo_Im", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `EVENTCHKINF_4C`, `EVENTCHKINF_59` (`save.h`).
const EVENTCHKINF_4C: u16 = 0x4C;
pub const EVENTCHKINF_59: u16 = 0x59;

/// `switch (this->actor.params)`: the type.
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[
        (0, "the Chamber of Sages: the Shadow Medallion (any other not listed too)"),
        (2, "Ganon's sealing; the Shadow trial's barrier"),
        (3, "the lullaby's whistle"),
        (4, "Hyrule Field: the Deku Nut"),
        (5, "Zelda's courtyard: the lullaby"),
        (6, "the ending"),
    ]),
    "which of Impa's scenes",
);
/// `world.rot.x`: the placement's pitch (`focus.rot` copies it), a rotation.
pub const ROT_X: Field = Field::rot("world.rot.x", Source::RotX, 0, 16, Kind::Rotation, "her pitch (Ganon's Castle's)");

/// Demo_Im's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, ROT_X],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_OBTAINED_ZELDAS_LETTER)).reads().note("type 5: shown once Link has the letter"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_59)).reads().sets().note("type 5: the lullaby's cutscene seen"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_ZELDA_FLED_CASTLE)).reads().note("type 5: gone once Zelda has fled"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_4C)).sets().note("an unreachable branch"),
        FlagUse::new(Flag::Other("gSaveContext.chamberCutsceneNum")).reads().note("type 0: CHAMBER_CS_SHADOW, the Shadow Medallion's (not ported)"),
    ],
    todo: false,
    objects: &[],
    // `func_80986AD0`: her only line (the lullaby's ocarina texts are the cutscene's).
    texts: &[TextLine::new(0x708E).when("params", &[5]).cond("once Zelda's Lullaby is known, before Zelda flees").note("let me lead you out of the castle")],
};
/// `ITEM_SONG_LULLABY` (`item.h`).
const ITEM_SONG_LULLABY: u8 = 0x60;
/// `ENTR_HYRULE_FIELD_0` (`entrance_table.h`).
const ENTR_HYRULE_FIELD_0: u16 = 0x0CD;
/// `PLAYER_CSACTION_8`.
const PLAYER_CSACTION_8: u8 = 8;
/// `NPC_TRACKING_NONE`, `NPC_TRACKING_HEAD_AND_TORSO`, `NPC_TRACKING_FULL_BODY` (`actor.h`).
const NPC_TRACKING_NONE: i16 = 1;
const NPC_TRACKING_HEAD_AND_TORSO: i16 = 2;
const NPC_TRACKING_FULL_BODY: i16 = 4;
/// `kREG(17) + 0xC`: the tracking preset (`kREG` 0 in the retail game).
const TRACKING_PRESET: usize = 0xC;

/// `IMPA_LIMB_CHEST`, `IMPA_LIMB_RIGHT_HAND`, `IMPA_LIMB_HEAD` (`object_im.xml`, 1-based).
const IMPA_LIMB_CHEST: usize = 9;
const IMPA_LIMB_RIGHT_HAND: usize = 15;
const IMPA_LIMB_HEAD: usize = 16;

/// `sEyeTextures`.
const EYE_TEXTURES: [&str; 3] = ["gImpaEyeOpenTex", "gImpaEyeHalfTex", "gImpaEyeClosedTex"];
/// The segments `DemoIm_DrawSolid` binds the eye to (both the same texture), where a draw puts the
/// blink's texture in the bake's place.
const SEG_EYE_8: u32 = 0x0800_0000;
const SEG_EYE_9: u32 = 0x0900_0000;

/// Her skeleton's bakes: as drawn, and with the pointing hand (`unk_2D0`: `gImpaHandPointingDL`).
pub const BAKE: &str = "Demo_Im/skel";
pub const BAKE_POINTING: &str = "Demo_Im/skel_pointing";

/// `D_809887D8`: the focus's offset from her head.
const FOCUS_OFFSET: Vec3 = Vec3::new(0.0, 10.0, 0.0);

/// The meshes `DemoIm_DrawSolid` draws (`SETUPDL_25` opaque; `gDPSetEnvColor(0, 0, 0, 255)` and
/// `ACTOR_SETUP_OPA_DL` on segment 0xC are what the lists read).
pub fn bakes() -> Vec<MeshBake> {
    let tex = |s: &str| BakeSegment::Texture { file: OBJECT.into(), symbol: s.into() };
    [(BAKE, None), (BAKE_POINTING, Some("gImpaHandPointingDL"))]
        .into_iter()
        .map(|(name, hand)| MeshBake {
            name: name.into(),
            object: OBJECT.into(),
            // Segment 9 on the half-shut eye so a draw can tell the two apart.
            segments: vec![(0x08, tex(EYE_TEXTURES[0])), (0x09, tex(EYE_TEXTURES[1]))],
            prelude: Vec::new(),
            body: BakeBody::Skeleton {
                file: OBJECT.into(),
                symbol: "gImpaSkel".into(),
                limbs: hand.map(|s| vec![LimbOverride { limb: (IMPA_LIMB_RIGHT_HAND - 1) as u8, file: OBJECT.into(), symbol: s.into() }]).unwrap_or_default(),
            },
        })
        .collect()
}

/// `sCylinderInit` (`ColliderCylinderInitType1`: `OC2_TYPE_1`).
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 25, height: 80, y_shift: 0, pos: [0; 3] },
};

pub struct DemoIm {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub eye_index: i16,
    pub blink_timer: i16,
    /// `action`: the index into `sActionFuncs`.
    pub action: i32,
    /// `drawConfig`: 0 nothing, 1 solid, 2 translucent.
    pub draw_config: i32,
    pub unk_268: f32,
    pub alpha: i32,
    pub unk_270: i32,
    pub cue_id: u32,
    pub unk_278: f32,
    pub unk_27c: i32,
    /// `unk_280`: the head and torso turns drawn.
    pub unk_280: i32,
    pub collider: ColliderCylinder,
    /// `unk_2D0`: the pointing hand drawn.
    pub unk_2d0: i32,
    pub interact_info: NpcTrack,
    idle: Anim,
    whistling: Anim,
    start_whistling: Anim,
    throw_deku_nut: Anim,
    eyes: Vec<DrawImage>,
    not_ported_logged: bool,
}

/// `DECR`.
fn decr(x: &mut i16) -> i16 {
    if *x == 0 {
        0
    } else {
        *x -= 1;
        *x
    }
}

impl DemoIm {
    /// `DemoIm_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.skeleton(OBJECT, "gImpaSkel")?;
            let anim = |s: &str| assets.animation(OBJECT, s);
            let anims = (anim("gImpaIdleAnim")?, anim("gImpaWhistlingAnim")?, anim("gImpaStartWhistlingAnim")?, anim("gImpaThrowDekuNutAnim")?);
            let mut eyes = Vec::new();
            for s in EYE_TEXTURES {
                let t = assets.pack.texture(OBJECT, s)?;
                eyes.push(DrawImage { width: t.width, height: t.height, rgba: t.rgba.clone().into() });
            }
            Ok((skeleton, anims, eyes))
        })();
        let (skeleton, (idle, whistling, start_whistling, throw_deku_nut), eyes) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("Demo_Im: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = DemoIm {
            // SkelAnime_InitFlex(.., gImpaSkel, NULL, .., IMPA_LIMB_MAX).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            eye_index: 0,
            blink_timer: 0,
            action: 0,
            draw_config: 0,
            unk_268: 0.0,
            alpha: 0,
            unk_270: 0,
            cue_id: 0,
            unk_278: 0.0,
            unk_27c: 0,
            unk_280: 0,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            unk_2d0: 0,
            interact_info: NpcTrack::default(),
            idle,
            whistling,
            start_whistling,
            throw_deku_nut,
            eyes,
            not_ported_logged: false,
            actor,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 30.0);
        this.collider.update(&this.actor);
        this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        match TYPE.get(this.actor.params) {
            3 => this.func_80985e60(),
            4 => this.func_8098652c(),
            5 => this.func_809869b0(),
            p => {
                // func_80985860 (2), func_80987018 (6), func_80985310 (0, 1): the Chamber of Sages and
                // the ending, not ported.
                log::warn!("Demo_Im: type {p} (the Chamber of Sages, Ganon's sealing, the ending) isn't ported");
                this.action = -1;
                // func_8098544C (func_80985310's types: 0, 1 and any other but 2 and 6).
                if !matches!(p, 2 | 6) && play.save.chamber_cutscene_num == oot_game::save::CHAMBER_CS_SHADOW && !play.is_cutscene_layer() {
                    log::warn!("Demo_Im: chamberCutsceneNum CHAMBER_CS_SHADOW: the Shadow Medallion's cutscene (gShadowMedallionCs) would start (Phase 17)");
                }
            }
        }
        Box::new(this)
    }

    /// `DemoIm_ChangeAnim`.
    fn change_anim(&mut self, anim: Anim, mode: u8, morph_frames: f32, play_backwards: bool) {
        let frame_count = anim.last_frame();
        let (start, end, speed) = if !play_backwards { (0.0, frame_count, 1.0) } else { (frame_count, 0.0, -1.0) };
        self.skel.change(anim, speed, start, end, mode, morph_frames);
    }

    /// `Animation_Change(&skelAnime, anim, speed, start, end, mode, morph)`.
    fn anim_change(&mut self, anim: Anim, speed: f32, start: f32, end: f32, mode: u8, morph: f32) {
        self.skel.change(anim, speed, start, end, mode, morph);
    }

    /// `func_80984BE0`: the blink, every 60 to 120 frames.
    fn func_80984be0(&mut self, play: &mut PlayState) {
        if decr(&mut self.blink_timer) == 0 {
            self.blink_timer = play.rand.s16_offset(60, 60);
        }
        self.eye_index = self.blink_timer;
        if self.eye_index >= 3 {
            self.eye_index = 0;
        }
    }

    /// `func_80984DB8`: the head and torso eased back to rest.
    fn func_80984db8(&mut self) {
        let t = &mut self.interact_info;
        smooth_step_to_s(&mut t.head[0], 0, 20, 6200, 100);
        smooth_step_to_s(&mut t.head[1], 0, 20, 6200, 100);
        smooth_step_to_s(&mut t.torso[0], 0, 20, 6200, 100);
        smooth_step_to_s(&mut t.torso[1], 0, 20, 6200, 100);
    }

    fn player_pos(play: &PlayState) -> Vec3 {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO)
    }

    /// `func_80984E58`: the head and torso on Link while he's within a quarter of her facing.
    fn func_80984e58(&mut self, play: &mut PlayState) {
        self.interact_info.target = Self::player_pos(play);
        self.interact_info.eye_height = 4.0;
        let yaw_diff = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        let mode = if (yaw_diff as i32).abs() < 0x18E3 { NPC_TRACKING_HEAD_AND_TORSO } else { NPC_TRACKING_NONE };
        track_point(play, &mut self.actor, &mut self.interact_info, TRACKING_PRESET, mode);
    }

    /// `func_80984F10`.
    fn func_80984f10(&mut self, play: &mut PlayState) {
        self.interact_info.target = Self::player_pos(play);
        self.interact_info.eye_height = 12.0;
        track_point(play, &mut self.actor, &mut self.interact_info, TRACKING_PRESET, NPC_TRACKING_HEAD_AND_TORSO);
    }

    /// `func_80984F94`.
    fn func_80984f94(&mut self, play: &mut PlayState) {
        self.interact_info.target = Self::player_pos(play);
        self.interact_info.eye_height = 4.0;
        track_point(play, &mut self.actor, &mut self.interact_info, TRACKING_PRESET, NPC_TRACKING_FULL_BODY);
    }

    /// `DemoIm_UpdateBgCheckInfo`.
    fn update_bg_check_info(&mut self, play: &PlayState) {
        self.actor.update_bg_check_info(&play.col, 75.0, 30.0, 30.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
    }

    /// `DemoIm_GetCue`: the cue on `channel` while a script runs.
    fn get_cue(play: &PlayState, channel: usize) -> Option<CsCmdActorCue> {
        if play.cs_ctx.state == CS_STATE_IDLE {
            return None;
        }
        play.cs_ctx.npc_actions[channel]
    }

    /// `func_80985180`: at the cue's start, turned its way.
    fn func_80985180(&mut self, play: &PlayState, channel: usize) {
        if let Some(cue) = Self::get_cue(play, channel) {
            self.actor.world_pos = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
            self.actor.shape_rot.y = cue.rot[1];
            self.actor.world_rot.y = cue.rot[1];
        }
    }

    /// `DemoIm_UpdateCollider`.
    fn update_collider(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }

    // Types 3 and 5: the lullaby.

    /// `func_80985E60` (type 3's init).
    fn func_80985e60(&mut self) {
        self.change_anim(self.idle.clone(), ANIMMODE_LOOP, 0.0, false);
        self.action = 10;
        self.unk_280 = 1;
    }

    /// `func_80985EAC`: on Link between frames 80 and 243 of the script, else her head eased back.
    fn func_80985eac(&mut self, play: &mut PlayState) {
        if play.cs_ctx.frames >= 80 && play.cs_ctx.frames < 243 {
            self.func_80984f10(play);
        } else {
            self.func_80984db8();
        }
    }

    /// `func_80985EF4`: the whistle held on its next-to-last frame.
    fn func_80985ef4(&mut self) {
        if !self.skel.on_frame(self.whistling.last_frame() - 1.0) {
            self.skel.update();
        }
    }

    /// `func_80985F54`: hidden, waiting for cues.
    fn func_80985f54(&mut self) {
        self.action = 10;
        self.draw_config = 0;
    }

    /// `func_80985F64`: shown at the cue.
    fn func_80985f64(&mut self, play: &PlayState) {
        let a = self.idle.clone();
        let end = a.last_frame();
        self.anim_change(a, 1.0, 0.0, end, ANIMMODE_LOOP, 0.0);
        self.func_80985180(play, 5);
        self.action = 11;
        self.draw_config = 1;
    }

    /// `func_80985FE8`: whistling on once the start's done.
    fn func_80985fe8(&mut self, anim_finished: bool) {
        if anim_finished {
            let a = self.whistling.clone();
            let end = a.last_frame();
            self.anim_change(a, 1.0, 0.0, end, ANIMMODE_LOOP, -8.0);
        }
    }

    /// `func_8098604C`: the whistle raised, the hand pointing.
    fn func_8098604c(&mut self) {
        let a = self.start_whistling.clone();
        let end = a.last_frame();
        self.anim_change(a, 1.0, 0.0, end, ANIMMODE_ONCE, -8.0);
        self.action = 12;
        self.draw_config = 1;
        self.unk_2d0 = 1;
    }

    /// `func_809860C8`.
    fn func_809860c8(&mut self) {
        self.action = 13;
        self.draw_config = 1;
    }

    /// `func_809860DC`: the idle once the whistle's put down, the hand back.
    fn func_809860dc(&mut self, anim_finished: bool) {
        if anim_finished {
            let a = self.idle.clone();
            let end = a.last_frame();
            self.anim_change(a, 1.0, 0.0, end, ANIMMODE_LOOP, -8.0);
            self.unk_2d0 = 0;
        }
    }

    /// `func_80986148`: the whistle put down (the start played backwards).
    fn func_80986148(&mut self) {
        let a = self.start_whistling.clone();
        let start = a.last_frame();
        self.anim_change(a, -1.0, start, 0.0, ANIMMODE_ONCE, -8.0);
        self.action = 14;
        self.draw_config = 1;
    }

    /// `func_809861C4`: action 13's cues.
    fn func_809861c4(&mut self, play: &PlayState) {
        if let Some(cue) = Self::get_cue(play, 5) {
            let next = cue.action as u32;
            if next != self.cue_id {
                match next {
                    9 => self.func_80986148(),
                    7 => {
                        let a = self.whistling.clone();
                        let end = a.last_frame();
                        self.anim_change(a, 1.0, 0.0, end, ANIMMODE_LOOP, -8.0);
                        self.action = 12;
                    }
                    _ => log::debug!("Demo_Im_Ocarina_Check_DemoMode: There is no such action!!!!!!!!"),
                }
                self.cue_id = next;
            }
        }
    }

    /// `func_8098629C`: the script over, talkative.
    fn func_8098629c(&mut self, play: &PlayState) {
        if play.cs_ctx.state == CS_STATE_IDLE {
            self.action = 21;
            self.draw_config = 1;
            self.unk_280 = 1;
        }
    }

    /// `func_809862E0`: the lullaby's cues.
    fn func_809862e0(&mut self, play: &PlayState) {
        if let Some(cue) = Self::get_cue(play, 5) {
            let next = cue.action as u32;
            if next != self.cue_id {
                match next {
                    1 => self.func_80985f54(),
                    2 => self.func_80985f64(play),
                    7 => self.func_8098604c(),
                    8 => self.func_809860c8(),
                    9 => self.func_80986148(),
                    _ => log::debug!("Demo_Im_Ocarina_Check_DemoMode: There is no such action!!!!!!!!"),
                }
                self.cue_id = next;
            }
        }
    }

    // Type 4: Hyrule Field.

    /// `func_8098652C` (type 4's init).
    fn func_8098652c(&mut self) {
        self.change_anim(self.idle.clone(), ANIMMODE_LOOP, 0.0, false);
        self.action = 15;
    }

    /// `func_80986570`: her step on frame 7 of the throw, on the ground.
    fn func_80986570(&mut self, play: &mut PlayState) {
        if self.skel.on_frame(7.0) && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            let offset = play.audio.tables.surface_sfx_id(self.actor.floor_poly.map(|p| play.col.sfx_type(p)).unwrap_or(0));
            let pos = play.cur_actor.map(SfxPos::Actor).unwrap_or(SfxPos::Default);
            play.audio.play_sfx_at_pos(pos, NA_SE_PL_WALK_GROUND.wrapping_add(offset));
        }
    }

    /// `func_809865F8`: the Deku Nut thrown 30 in front of her once the throw's done.
    fn func_809865f8(&mut self, play: &mut PlayState, anim_finished: bool) {
        if anim_finished {
            if self.unk_278 >= 0.0 {
                if self.unk_27c == 0 {
                    let y = self.actor.shape_rot.y;
                    let pos = Vec3::new(self.actor.world_pos.x + eng_math::sin_s(y) * 30.0, self.actor.world_pos.y, self.actor.world_pos.z + eng_math::cos_s(y) * 30.0);
                    let _ = play.actor_spawn(ACTOR_EN_ARROW, pos, [0xFA0, y, 0], ARROW_CS_NUT);
                    self.unk_27c = 1;
                }
            } else {
                self.unk_278 += 1.0;
            }
        }
    }

    /// `func_80986700`.
    fn func_80986700(&mut self) {
        self.action = 15;
        self.draw_config = 0;
    }

    /// `func_80986710`.
    fn func_80986710(&mut self, play: &PlayState) {
        let a = self.idle.clone();
        let end = a.last_frame();
        self.anim_change(a, 1.0, 0.0, end, ANIMMODE_LOOP, 0.0);
        self.func_80985180(play, 5);
        self.action = 16;
        self.draw_config = 1;
    }

    /// `func_80986794`.
    fn func_80986794(&mut self) {
        let a = self.throw_deku_nut.clone();
        let end = a.last_frame();
        self.anim_change(a, 1.0, 0.0, end, ANIMMODE_ONCE, -8.0);
        self.action = 17;
        self.draw_config = 1;
    }

    /// `func_8098680C`: the field's cues.
    fn func_8098680c(&mut self, play: &PlayState) {
        if let Some(cue) = Self::get_cue(play, 5) {
            let next = cue.action as u32;
            if next != self.cue_id {
                match next {
                    1 => self.func_80986700(),
                    2 => self.func_80986710(play),
                    10 => self.func_80986794(),
                    11 => self.actor.kill(),
                    _ => log::debug!("Demo_Im_Spot00_Check_DemoMode: There is no such action!!!!!!!!"),
                }
                self.cue_id = next;
            }
        }
    }

    // Type 5: Zelda's courtyard.

    /// `func_809869B0` (type 5's init).
    fn func_809869b0(&mut self) {
        self.change_anim(self.idle.clone(), ANIMMODE_LOOP, 0.0, false);
        self.action = 18;
        self.actor.shape_shadow_alpha = 0;
    }

    /// `func_809869F8`: Link 30 west of her, she out of the culling volume.
    fn func_809869f8(&self, play: &PlayState) -> bool {
        Self::player_pos(play).x < self.actor.world_pos.x - 30.0 && self.actor.flags & ACTOR_FLAG_INSIDE_CULLING_VOLUME == 0
    }

    /// `func_80986A5C`: Link within 130 west of her, no cutscene.
    fn func_80986a5c(&self, play: &PlayState) -> bool {
        self.actor.world_pos.x - 130.0 < Self::player_pos(play).x && !play.play_in_cs_mode()
    }

    /// `func_80986AD0`: her talk (0x708E) offered; true once taken.
    fn func_80986ad0(&mut self, play: &mut PlayState) -> bool {
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY;
        if !process_talk_request(&mut self.actor) {
            self.actor.text_id = 0x708E;
            let a = self.actor.clone();
            offer_talk_default(play, &a);
            false
        } else {
            true
        }
    }

    /// `func_80986B2C`: as the box closes, Link out to Hyrule Field with a black starburst.
    fn func_80986b2c(play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_CLOSING {
            play.transition.next_entrance_index = ENTR_HYRULE_FIELD_0;
            play.transition.ty = trans_type_circle(TCA_STARBURST, TCC_BLACK, TCS_FAST);
            play.transition.trigger = TRANS_TRIGGER_START;
            let player = play.player;
            play.player_set_cs_action_with_halted_actors(player, PLAYER_CSACTION_8);
        }
    }

    /// `func_80986BA0`.
    fn func_80986ba0(&mut self, play: &PlayState) {
        if self.func_809869f8(play) {
            self.action = 21;
            self.draw_config = 1;
            self.unk_280 = 1;
            self.actor.shape_shadow_alpha = 255;
        }
    }

    /// `func_80986BF8`: shown once Link has the letter.
    fn func_80986bf8(&mut self, play: &PlayState) {
        if play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER) {
            self.action = 24;
            self.draw_config = 1;
            self.unk_280 = 1;
            self.actor.shape_shadow_alpha = 255;
        }
    }

    /// `func_80986C30`: Link near: the lullaby's script, the song given.
    fn func_80986c30(&mut self, play: &mut PlayState) {
        if self.func_80986a5c(play) {
            play.cs_ctx.segment = play.cutscene_script("gZeldasCourtyardLullabyCs");
            play.save.cutscene_trigger = 1;
            play.save.set_event_chk_inf(EVENTCHKINF_59);
            oot_game::item::item_give(&mut play.save, Some(&mut play.audio), ITEM_SONG_LULLABY);
            self.func_80985f54();
        }
    }

    /// `func_80986CC8`.
    fn func_80986cc8(&mut self, play: &PlayState) {
        if play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER) {
            self.action = 26;
            self.draw_config = 1;
            self.unk_280 = 1;
            self.actor.shape_shadow_alpha = 255;
        }
    }

    /// `func_80986CFC`.
    fn func_80986cfc(&mut self, play: &mut PlayState) {
        if self.func_80986a5c(play) {
            play.save.set_event_chk_inf(EVENTCHKINF_4C);
            self.action = 19;
        }
    }

    /// `func_80986D40`: gone once Zelda has fled; waiting for the letter, or (the lullaby's seen)
    /// for Link by the entrance.
    fn func_80986d40(&mut self, play: &PlayState) {
        // (DEBUG_FEATURES: on scene layer 6, action 19 at once.)
        if play.save.scene_layer == 6 {
            self.action = 19;
            self.draw_config = 1;
            return;
        }
        if play.save.get_event_chk_inf(EVENTCHKINF_ZELDA_FLED_CASTLE) {
            self.actor.kill();
        } else if !play.save.get_event_chk_inf(EVENTCHKINF_59) {
            self.action = 23;
        } else {
            self.action = 20;
        }
    }

    /// `sActionFuncs[action]` for the ported types' actions.
    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            // func_809863BC.
            10 => self.func_809862e0(play),
            // func_809863DC.
            11 => {
                self.update_bg_check_info(play);
                self.skel.update();
                self.func_80985eac(play);
                self.func_80984be0(play);
                self.func_809862e0(play);
            }
            // func_80986430.
            12 => {
                self.update_bg_check_info(play);
                let done = self.skel.update();
                self.func_80985eac(play);
                self.func_80984be0(play);
                self.func_80985fe8(done);
                self.func_809862e0(play);
            }
            // func_80986494.
            13 => {
                self.update_bg_check_info(play);
                self.func_80985ef4();
                self.func_80984be0(play);
                self.func_809861c4(play);
            }
            // func_809864D4.
            14 => {
                self.update_bg_check_info(play);
                let done = self.skel.update();
                self.func_80984be0(play);
                self.func_809860dc(done);
                self.func_8098629c(play);
            }
            // func_809868E8.
            15 => self.func_8098680c(play),
            // func_80986908.
            16 => {
                self.update_bg_check_info(play);
                self.skel.update();
                self.func_80984be0(play);
                self.func_8098680c(play);
            }
            // func_80986948.
            17 => {
                self.update_bg_check_info(play);
                let done = self.skel.update();
                self.func_80986570(play);
                self.func_80984be0(play);
                self.func_809865f8(play, done);
                self.func_8098680c(play);
            }
            // func_80986D40.
            18 => self.func_80986d40(play),
            // func_80986DC8.
            19 => {
                self.update_bg_check_info(play);
                self.skel.update();
                self.func_80984be0(play);
                self.func_80984e58(play);
                self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY);
            }
            // func_80986E20.
            20 => self.func_80986ba0(play),
            // func_80986E40.
            21 => {
                self.update_bg_check_info(play);
                self.skel.update();
                self.func_80984be0(play);
                self.func_80984e58(play);
                self.update_collider(play);
                if self.func_80986ad0(play) {
                    // func_80986BE4.
                    self.action = 22;
                }
            }
            // func_80986EAC.
            22 => {
                self.update_bg_check_info(play);
                self.skel.update();
                self.func_80984be0(play);
                self.func_80984f94(play);
                self.update_collider(play);
                Self::func_80986b2c(play);
            }
            // func_80986F08.
            23 => self.func_80986bf8(play),
            // func_80986F28.
            24 => {
                self.update_bg_check_info(play);
                self.skel.update();
                self.func_80984be0(play);
                self.func_80984e58(play);
                self.update_collider(play);
                self.func_80986c30(play);
            }
            // func_80986F88.
            25 => self.func_80986cc8(play),
            // func_80986FA8.
            26 => {
                self.update_bg_check_info(play);
                self.skel.update();
                self.func_80984be0(play);
                self.func_80984e58(play);
                self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY);
                self.update_collider(play);
                self.func_80986cfc(play);
            }
            a => {
                if !self.not_ported_logged {
                    log::warn!("Demo_Im: action {a} isn't ported");
                    self.not_ported_logged = true;
                }
            }
        }
    }

    /// Her skeleton posed as `DemoIm_OverrideLimbDraw` turns it while `unk_280` is set: the chest by
    /// the torso's turn, the head by the head's.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], turns: Option<([i16; 3], [i16; 3])>) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if let Some((head, torso)) = turns {
                match limb {
                    IMPA_LIMB_CHEST => {
                        rot[0] = rot[0].wrapping_add(torso[1]);
                        rot[1] = rot[1].wrapping_sub(torso[0]);
                    }
                    IMPA_LIMB_HEAD => {
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

impl ActorImpl for DemoIm {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `DemoIm_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if !(0..31).contains(&self.action) {
            if !self.not_ported_logged {
                log::debug!("The main mode is wrong!!!!!!!!!!!!!!!!!!!!!!!!!");
                self.not_ported_logged = true;
            }
            return;
        }
        self.run_action(play);
    }
    /// `DemoIm_PostLimbDraw`: the focus 10 above her head.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.draw_config != 1 {
            return;
        }
        let turns = (self.unk_280 != 0).then_some((self.interact_info.head, self.interact_info.torso));
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, turns);
        if let Some(head) = bones.get(IMPA_LIMB_HEAD - 1) {
            let m = actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(FOCUS_OFFSET);
            self.actor.focus_rot = self.actor.world_rot;
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let (h, t) = (self.interact_info.head, self.interact_info.torso);
        rs.angles = vec![h[0], h[1], h[2], t[0], t[1], t[2]];
        rs.switches = vec![self.draw_config.max(0) as u32, self.eye_index.clamp(0, 2) as u32, self.unk_280 as u32, self.unk_2d0 as u32];
        rs
    }
    /// `DemoIm_Draw`: by `drawConfig`, nothing or `DemoIm_DrawSolid` (the translucent draw is type 2's
    /// and 6's, not ported).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.len() < 4 {
            return;
        }
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 6 || rs.switches[0] != 1 {
            return;
        }
        let a = &rs.angles;
        let turns = (rs.switches[2] != 0).then_some(([a[0], a[1], a[2]], [a[3], a[4], a[5]]));
        let bones = Self::pose(&self.skeleton, &joints.rot, turns);
        let eye = self.eyes[(rs.switches[1] as usize).min(EYE_TEXTURES.len() - 1)].clone();
        let texture_images = vec![SourceImage { source: SEG_EYE_8, image: eye.clone() }, SourceImage { source: SEG_EYE_9, image: eye }];
        let bake = if rs.switches[3] != 0 { BAKE_POINTING } else { BAKE };
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: actor_draw_matrix(rs), bones, params: DrawParams { texture_images, ..Default::default() } });
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
