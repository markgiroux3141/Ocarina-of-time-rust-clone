//! `En_Xc` (`ovl_En_Xc/z_en_xc.c`): Sheik. Her params pick one of ten uses; this phase ports the
//! Temple of Time's (types 9 and 1) and the pedestal block they lead to.
//!
//! **Type 9, the Temple of Time** (room 0 of every layer 0 to 3; `EnXc_InitTempleOfTime`): gone for
//! a child. An adult without `EVENTCHKINF_C5` sets it and starts `gTempleOfTimeFirstAdultCs` (the
//! scene's layer-4 script: the awakening), then runs type 1's actions. With `EVENTCHKINF_48` (the
//! Forest Temple's blue warp) and no `EVENTCHKINF_55` she sets 0x55, gives the Prelude of Light
//! (`Item_Give(ITEM_SONG_PRELUDE)`) and starts `gTempleOfTimePreludeCs`, running type 2's actions
//! (`SHEIK_ACTION_30` on): **milestone 3's**, a logged no-op here. Otherwise, without 0x55, she
//! blocks the pedestal; with it she's gone.
//!
//! **Type 1, the awakening** (`gTempleOfTimeFirstAdultCs`'s Sheik, layer 4): hidden
//! (`SHEIK_ACTION_20`, `_21`) until a cue other than 1 on channel 4 (`func_80B3EC0C`) puts her at
//! its start, turned its way, drawn. In `SHEIK_ACTION_22` she idles, blinks and stays on the floor
//! (`Actor_UpdateBgCheckInfo`, walls); a cue other than 6 then leaves her blocking the pedestal.
//! `SHEIK_ACTION_23` (a walk: `EnXc_CalcXZAccel`, her steps' sounds on frames 11 and 23) is in the
//! table but nothing sets it; 24 to 28 do nothing.
//!
//! **The pedestal block** (`func_80B3C9EC`, `EnXc_BlockingPedestalAction`): arms crossed
//! (`gSheikArmsCrossedIdleAnim`), her head and torso on Link (`Npc_TrackPoint`, preset 12), solid
//! to Link (her OC cylinder, 25 by 80), she offers to talk within 50 plus her cylinder's radius:
//! 0x700F ("…another skill… Head to Kakariko Village!") or, with the hookshot, 0x7010. Taken, she
//! talks (`SHEIK_ACTION_IN_DIALOGUE`) until the box closes.
//!
//! Drawn by `drawMode`: nothing, or `EnXc_DrawDefault` (`gSheikSkel` opaque, the eye on segments 8
//! and 9; once at the pedestal, `unk_30C`, her torso (limb 9) and head (16) turned by the
//! tracking; the focus 10 above her head). Her circle shadow (30) is the shape's. Not drawn:
//! `func_8002EBCC`'s look-at (the lists' reflections).
//!
//! Ported, besides the above: the shared code they use (`EnXc_InitCollider`, `_UpdateCollider`,
//! `_Destroy`, `_CalculateHeadTurn`, `_SetEyePattern`, `_BgCheck`, `_UpdateSkelAnime`,
//! `_GetCue`, `_ChangeAnimation`, `_SetWalkingSFX`, `_CalcXZAccel`, `func_80B3D6F0`,
//! `EnXc_SetupDialogueAction`, `func_80B41798`, `_Update`, `_Init`, `_OverrideLimbDraw`,
//! `_PostLimbDraw`, `_DrawNothing`, `_DrawDefault`, `_Draw`). **Not ported (the user's call):**
//! the other types, placeholders chosen at init (`EnXc_Init`'s switch: their checks, then a
//! `log::info!`): 2 (the Prelude's Sheik on layer 6, and the beta Serenade: milestone 3), 6 (the
//! Minuet, Sacred Forest Meadow: Phase 14), 7 (the Bolero, Death Mountain Crater: Phase 15), 8 (the
//! Serenade, the Ice Cavern: Phase 16), 3 (Lake Hylia after the Water Temple: Phase 16), 5 (the
//! Nocturne, Kakariko: Phase 17), 4 (the reveal, the Temple of Time's layers 12 and 13: Phase 19),
//! 0 (the songs' scripts on their cutscene layers: Phases 14 to 16) and any other. The harp's and
//! the Triforce's draws and the squint are theirs.
//!
//! Params: `params` whole, the type (`EnXcType`: 0 the songs' scripts, 1 the awakening, 2 the
//! Prelude, 3 Lake Hylia, 4 the reveal, 5 the Nocturne, 6 the Minuet, 7 the Bolero, 8 the
//! Serenade, 9 the Temple of Time).
//! Flags: type 9 reads and sets EVENTCHKINF_C5 (the awakening seen) and EVENTCHKINF_55 (the
//! Prelude learned), reads EVENTCHKINF_48 (the Forest Medallion's warp); the placeholders' checks
//! read EVENTCHKINF_50 (the Minuet learned, type 6), EVENTCHKINF_51 (the Bolero, 7) and
//! EVENTCHKINF_52 (the Serenade, 8, and the Iron Boots owned).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawImage, DrawParams, MeshKey, SourceImage};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_GROUND, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind, TextLine};
use oot_game::audio::sfx::{NA_SE_PL_WALK_GROUND, SfxPos};
use oot_game::collision_check::*;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::event_chk_inf;
use oot_game::item::{EQUIP_INV_BOOTS_IRON, EQUIP_TYPE_BOOTS, ITEM_HOOKSHOT, ITEM_NONE};
use oot_game::message::TEXT_STATE_CLOSING;
use oot_game::npc::{NpcTrack, offer_talk_default, process_talk_request, track_point};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;
use oot_game::surface::SurfaceType;

use crate::en_md::EVENTCHKINF_48;

/// `ACTOR_EN_XC` (`actor_table.h`: 0x0048).
pub const ACTOR_EN_XC: i16 = 0x0048;
/// `OBJECT_XC` (`object_table.h`: 0x008A).
pub const OBJECT: &str = "object_xc";

/// `En_Xc_Profile`: `ACTORCAT_NPC`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_XC, name: "En_Xc", category: ACTORCAT_NPC, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `EVENTCHKINF_50`, `_51`, `_52`, `_55`, `_C5` (`save.h`).
pub const EVENTCHKINF_50: u16 = 0x50;
pub const EVENTCHKINF_51: u16 = 0x51;
pub const EVENTCHKINF_52: u16 = 0x52;
pub const EVENTCHKINF_55: u16 = 0x55;
pub const EVENTCHKINF_C5: u16 = 0xC5;
/// `ITEM_SONG_PRELUDE` (`item.h`: 0x5F).
pub const ITEM_SONG_PRELUDE: u8 = 0x5F;

/// `EnXcType`.
pub const SHEIK_TYPE_0: i16 = 0;
pub const SHEIK_TYPE_1: i16 = 1;
pub const SHEIK_TYPE_2: i16 = 2;
pub const SHEIK_TYPE_3: i16 = 3;
pub const SHEIK_TYPE_4: i16 = 4;
pub const SHEIK_TYPE_5: i16 = 5;
pub const SHEIK_TYPE_MINUET: i16 = 6;
pub const SHEIK_TYPE_BOLERO: i16 = 7;
pub const SHEIK_TYPE_SERENADE: i16 = 8;
pub const SHEIK_TYPE_9: i16 = 9;

/// `this->actor.params` (`EnXc_Init`'s switch): which of Sheik's scenes.
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[
        (0, "the songs' scripts on their cutscene layers (SHEIK_TYPE_0)"),
        (1, "the Temple of Time: the awakening's script (layer 4)"),
        (2, "the Temple of Time: the Prelude's script (layer 6)"),
        (3, "Lake Hylia after the Water Temple"),
        (4, "the Temple of Time: the reveal (layers 12, 13)"),
        (5, "Kakariko: the Nocturne"),
        (6, "Sacred Forest Meadow: the Minuet"),
        (7, "Death Mountain Crater: the Bolero"),
        (8, "the Ice Cavern: the Serenade"),
        (9, "the Temple of Time: the awakening, the Prelude, the pedestal"),
    ]),
    "which of Sheik's scenes (1 and 9 ported, the others placeholders)",
);

/// En_Xc's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_C5)).reads().sets().note("type 9: the awakening's script seen"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_48)).reads().note("type 9: the Forest Medallion's warp, for the Prelude"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_55)).reads().sets().note("type 9: the Prelude of Light learned"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_50)).reads().note("type 6 (a placeholder): the Minuet learned, gone"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_51)).reads().note("type 7 (a placeholder): the Bolero learned, gone"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_52)).reads().note("type 8 (a placeholder): the Serenade learned, gone"),
    ],
    todo: false,
    objects: &[],
    // EnXc_SetupDialogueAction: her lines at the pedestal.
    texts: &[
        TextLine::new(0x7010).when("params", &[1, 9]).cond("at the pedestal, with the hookshot").note("go to Kakariko"),
        TextLine::new(0x700F).when("params", &[1, 9]).cond("at the pedestal").note("another skill... head to Kakariko Village"),
    ],
};

/// `EnXcAction` (the ported ones).
pub const SHEIK_ACTION_INIT: i32 = 0;
pub const SHEIK_ACTION_20: i32 = 20;
pub const SHEIK_ACTION_21: i32 = 21;
pub const SHEIK_ACTION_22: i32 = 22;
pub const SHEIK_ACTION_23: i32 = 23;
pub const SHEIK_ACTION_24: i32 = 24;
pub const SHEIK_ACTION_28: i32 = 28;
pub const SHEIK_ACTION_SERENADE: i32 = 29;
pub const SHEIK_ACTION_30: i32 = 30;
pub const SHEIK_ACTION_44: i32 = 44;
pub const SHEIK_ACTION_BLOCK_PEDESTAL: i32 = 79;
pub const SHEIK_ACTION_IN_DIALOGUE: i32 = 80;
/// `ARRAY_COUNT(sActionFuncs)`.
const ACTION_COUNT: i32 = 81;

/// `EnXcDrawMode`.
pub const SHEIK_DRAW_NOTHING: i32 = 0;
pub const SHEIK_DRAW_DEFAULT: i32 = 1;

/// `sEyeTextures`.
pub const EYE_TEXTURES: [&str; 3] = ["gSheikEyeOpenTex", "gSheikEyeHalfClosedTex", "gSheikEyeShutTex"];
/// The eye's segment (8; `EnXc_DrawDefault` binds 9 to the same, her lists only call 8), where a
/// draw puts the blink's texture in the bake's place.
const SEG_EYE: u32 = 0x0800_0000;
/// `EnXc_DrawDefault`'s mesh.
pub const BAKE: &str = "En_Xc/skel";

/// The limbs `EnXc_OverrideLimbDraw` and `_PostLimbDraw` name (1-based): the torso, the head.
const SHEIK_LIMB_TORSO: usize = 9;
const SHEIK_LIMB_HEAD: usize = 16;
/// `EnXc_PostLimbDraw`'s `src`: the focus's offset from her head.
const FOCUS_OFFSET: Vec3 = Vec3::new(0.0, 10.0, 0.0);

/// `NPC_TRACKING_HEAD_AND_TORSO` (`actor.h`).
const NPC_TRACKING_HEAD_AND_TORSO: i16 = 2;
/// `kREG(17) + 0xC`: the tracking preset; `kREG(16) - 3.0`: the eyes' height; `kREG(2)`: 0
/// (the debug registers start zeroed).
const TRACKING_PRESET: usize = 0xC;
const KREG_16: f32 = 0.0;
const KREG_2: f32 = 0.0;

/// Her mesh (`Gfx_SetupDL_25Opa`; the eye on segments 8 and 9, the draw swapping its texture).
pub fn bakes() -> Vec<MeshBake> {
    let tex = |s: &str| BakeSegment::Texture { file: OBJECT.into(), symbol: s.into() };
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(0x08, tex(EYE_TEXTURES[0])), (0x09, tex(EYE_TEXTURES[0]))],
        prelude: Vec::new(),
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gSheikSkel".into(), limbs: Vec::new() },
    }]
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

pub struct EnXc {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub eye_idx: i16,
    pub blink_timer: i16,
    /// `action`: the index into `sActionFuncs`.
    pub action: i32,
    pub draw_mode: i32,
    pub timer: f32,
    pub collider: ColliderCylinder,
    /// `unk_30C`: the head and torso turns drawn.
    pub unk_30c: i32,
    pub interact_info: NpcTrack,
    arms_crossed: Anim,
    eyes: Vec<DrawImage>,
    logged: bool,
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

impl EnXc {
    /// `EnXc_Init`: the circle shadow (30), `gSheikSkel` idling, the collider, then the type's
    /// init; the types not ported are placeholders after their checks.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut actor = actor;
        let ty = TYPE.get(actor.params);
        let what = match ty {
            SHEIK_TYPE_1 | SHEIK_TYPE_9 => None,
            SHEIK_TYPE_2 => Some("func_80B3EE64: the Prelude's script, the beta Serenade (Phase 12, milestone 3)"),
            SHEIK_TYPE_3 => Some("func_80B3F3C8: Lake Hylia after the Water Temple (Phase 16)"),
            SHEIK_TYPE_4 => Some("func_80B3FA08: the reveal, the Temple of Time (Phase 19)"),
            SHEIK_TYPE_5 => Some("func_80B40590: the Nocturne, Kakariko (Phase 17)"),
            SHEIK_TYPE_MINUET => {
                // func_80B3CA38: an adult without the Minuet, else gone.
                if !(!play.save.get_event_chk_inf(EVENTCHKINF_50) && play.save.adult) {
                    actor.kill();
                }
                Some("func_80B3CA38: the Minuet, Sacred Forest Meadow (Phase 14)")
            }
            SHEIK_TYPE_BOLERO => {
                // func_80B3CB58: an adult without the Bolero, else gone.
                if !(!play.save.get_event_chk_inf(EVENTCHKINF_51) && play.save.adult) {
                    actor.kill();
                }
                Some("func_80B3CB58: the Bolero, Death Mountain Crater (Phase 15)")
            }
            SHEIK_TYPE_SERENADE => {
                // EnXc_SetupSerenadeAction: (DEBUG_FEATURES) gone once the Iron Boots are owned; an
                // adult without the Serenade, else gone.
                let iron_boots = play.save.check_owned_equip(EQUIP_TYPE_BOOTS, EQUIP_INV_BOOTS_IRON);
                if !(!iron_boots && !play.save.get_event_chk_inf(EVENTCHKINF_52) && play.save.adult) {
                    actor.kill();
                }
                Some("EnXc_SetupSerenadeAction: the Serenade, the Ice Cavern (Phase 16)")
            }
            SHEIK_TYPE_0 => Some("EnXc_DoNothing: the songs' scripts on their cutscene layers (Phases 14 to 16)"),
            _ => Some("En_Oa2 arg_data is wrong (EnXc_DoNothing)"),
        };
        if let Some(what) = what {
            log::info!("En_Xc: type {ty}, {what}, isn't ported: a placeholder");
            return oot_game::spawn::Placeholder::init(actor, play);
        }
        let Some(assets) = play.assets.clone() else {
            actor.kill();
            return oot_game::spawn::Placeholder::init(actor, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.skeleton(OBJECT, "gSheikSkel")?;
            let idle = assets.animation(OBJECT, "gSheikIdleAnim")?;
            let arms_crossed = assets.animation(OBJECT, "gSheikArmsCrossedIdleAnim")?;
            let mut eyes = Vec::new();
            for s in EYE_TEXTURES {
                let t = assets.pack.texture(OBJECT, s)?;
                eyes.push(DrawImage { width: t.width, height: t.height, rgba: t.rgba.clone().into() });
            }
            Ok((skeleton, idle, arms_crossed, eyes))
        })();
        let (skeleton, idle, arms_crossed, eyes) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Xc: {e:#}");
                actor.kill();
                return oot_game::spawn::Placeholder::init(actor, play);
            }
        };
        let mut this = EnXc {
            actor,
            // SkelAnime_InitFlex(.., gSheikSkel, gSheikIdleAnim, jointTable, morphTable, 17).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(idle)),
            skeleton,
            eye_idx: 0,
            blink_timer: 0,
            action: SHEIK_ACTION_INIT,
            draw_mode: SHEIK_DRAW_NOTHING,
            timer: 0.0,
            // EnXc_InitCollider.
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            unk_30c: 0,
            interact_info: NpcTrack::default(),
            arms_crossed,
            eyes,
            logged: false,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 30.0);
        this.collider.update(&this.actor);
        match ty {
            SHEIK_TYPE_1 => this.func_80b3ebf0(),
            _ => this.init_temple_of_time(play),
        }
        Box::new(this)
    }

    /// `EnXc_InitTempleOfTime` (type 9): gone for a child; the awakening's script the first time
    /// as an adult; the Prelude's once the Forest Medallion's warp is taken; else the pedestal
    /// blocked until the Prelude is learned.
    fn init_temple_of_time(&mut self, play: &mut PlayState) {
        if !play.save.adult {
            self.actor.kill();
            return;
        }
        if !play.save.get_event_chk_inf(EVENTCHKINF_C5) {
            play.save.set_event_chk_inf(EVENTCHKINF_C5);
            play.cs_ctx.segment = play.cutscene_script("gTempleOfTimeFirstAdultCs");
            play.save.cutscene_trigger = 1;
            self.func_80b3ebf0();
        } else if !play.save.get_event_chk_inf(EVENTCHKINF_55) && play.save.get_event_chk_inf(EVENTCHKINF_48) {
            play.save.set_event_chk_inf(EVENTCHKINF_55);
            oot_game::item::item_give(&mut play.save, Some(&mut play.audio), ITEM_SONG_PRELUDE);
            play.cs_ctx.segment = play.cutscene_script("gTempleOfTimePreludeCs");
            play.save.cutscene_trigger = 1;
            // Type 2's actions from here: milestone 3's.
            self.action = SHEIK_ACTION_30;
        } else if !play.save.get_event_chk_inf(EVENTCHKINF_55) {
            self.func_80b3c9ec();
        } else {
            self.actor.kill();
        }
    }

    /// `EnXc_UpdateCollider`.
    fn update_collider(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }

    fn player_pos(play: &PlayState) -> Vec3 {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO)
    }

    /// `EnXc_CalculateHeadTurn`: her head and torso on Link.
    fn calculate_head_turn(&mut self, play: &mut PlayState) {
        self.interact_info.target = Self::player_pos(play);
        self.interact_info.eye_height = KREG_16 - 3.0;
        track_point(play, &mut self.actor, &mut self.interact_info, TRACKING_PRESET, NPC_TRACKING_HEAD_AND_TORSO);
    }

    /// `EnXc_SetEyePattern`: the blink, every 60 to 120 frames.
    fn set_eye_pattern(&mut self, play: &mut PlayState) {
        if decr(&mut self.blink_timer) == 0 {
            self.blink_timer = play.rand.s16_offset(60, 60);
        }
        self.eye_idx = self.blink_timer;
        if self.eye_idx >= EYE_TEXTURES.len() as i16 {
            self.eye_idx = 0;
        }
    }

    /// `EnXc_BgCheck`: walls only.
    fn bg_check(&mut self, play: &PlayState) {
        self.actor.update_bg_check_info(&play.col, 75.0, 30.0, 30.0, UPDBGCHECKINFO_FLAG_2);
    }

    /// `EnXc_UpdateSkelAnime`.
    fn update_skel_anime(&mut self) -> bool {
        self.skel.update()
    }

    /// `EnXc_GetCue`: the cue on `channel` while a script runs.
    pub fn get_cue(play: &PlayState, channel: usize) -> Option<CsCmdActorCue> {
        if play.cs_ctx.state == CS_STATE_IDLE {
            return None;
        }
        play.cs_ctx.npc_actions[channel]
    }

    /// `EnXc_ChangeAnimation`: from the start (or, reversed, from the end) at speed 1.
    fn change_animation(&mut self, animation: Anim, mode: u8, morph_frames: f32, reverse: bool) {
        let frame_count = animation.last_frame();
        let (start, end, speed) = if !reverse { (0.0, frame_count, 1.0) } else { (frame_count, 0.0, -1.0) };
        self.skel.change(animation, speed, start, end, mode, morph_frames);
    }

    /// `func_80B3C9EC`: arms crossed before the pedestal.
    fn func_80b3c9ec(&mut self) {
        self.change_animation(self.arms_crossed.clone(), ANIMMODE_LOOP, 0.0, false);
        self.action = SHEIK_ACTION_BLOCK_PEDESTAL;
        self.draw_mode = SHEIK_DRAW_DEFAULT;
        self.unk_30c = 1;
    }

    /// `EnXc_SetWalkingSFX`: her steps on frames 11 and 23, on the ground.
    fn set_walking_sfx(&mut self, play: &mut PlayState) {
        if (self.skel.on_frame(11.0) || self.skel.on_frame(23.0)) && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            let offset = play.audio.tables.surface_sfx_id(self.actor.floor_poly.map(|p| play.col.sfx_type(p)).unwrap_or(0));
            let pos = play.cur_actor.map(SfxPos::Actor).unwrap_or(SfxPos::Default);
            play.audio.play_sfx_at_pos(pos, NA_SE_PL_WALK_GROUND.wrapping_add(offset));
        }
    }

    /// `EnXc_CalcXZAccel`: still before 9 frames, then `kREG(2) * 0.01 + 1.2`; moved.
    fn calc_xz_accel(&mut self) {
        let timer = self.timer;
        if timer < 9.0 {
            self.actor.speed_xz = 0.0;
        } else if timer < 3.0 {
            // @bug (game): never true after `timer < 9`: no ramp.
            self.actor.speed_xz = (((KREG_2 * 0.01) + 1.2) / 3.0) * (timer - 9.0);
        } else {
            self.actor.speed_xz = (KREG_2 * 0.01) + 1.2;
        }
        // Actor_MoveXZGravity.
        self.actor.move_forward();
    }

    // Type 1 (and type 9's first visit).

    /// `func_80B3EBF0`.
    fn func_80b3ebf0(&mut self) {
        self.action = SHEIK_ACTION_20;
    }

    /// `func_80B3EC00`.
    fn func_80b3ec00(&mut self) {
        self.action = SHEIK_ACTION_21;
    }

    /// `func_80B3EC0C`: a cue other than 1 on channel 4: at its start, turned its way, drawn.
    fn func_80b3ec0c(&mut self, play: &PlayState) {
        if play.cs_ctx.state != CS_STATE_IDLE
            && let Some(cue) = play.cs_ctx.npc_actions[4]
            && cue.action != 1
        {
            self.actor.world_pos = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
            self.actor.shape_rot.y = cue.rot[1];
            self.actor.world_rot.y = cue.rot[1];
            self.action = SHEIK_ACTION_22;
            self.draw_mode = SHEIK_DRAW_DEFAULT;
        }
    }

    /// `func_80B3EC90`: a cue other than 6 on channel 4: to the pedestal.
    fn func_80b3ec90(&mut self, play: &PlayState) {
        if play.cs_ctx.state != CS_STATE_IDLE
            && let Some(cue) = play.cs_ctx.npc_actions[4]
            && cue.action != 6
        {
            self.func_80b3c9ec();
        }
    }

    /// `func_80B3ECD8`: walking at full speed after 12 frames.
    fn func_80b3ecd8(&mut self) {
        self.timer += 1.0;
        if self.timer >= 12.0 {
            self.actor.speed_xz = KREG_2 * 0.01 + 1.2;
            self.action = SHEIK_ACTION_24;
        }
    }

    // The pedestal.

    /// `EnXc_SetupDialogueAction`: talking once taken; else offering 0x7010 (the hookshot owned)
    /// or 0x700F.
    fn setup_dialogue_action(&mut self, play: &mut PlayState) {
        if process_talk_request(&mut self.actor) {
            self.action = SHEIK_ACTION_IN_DIALOGUE;
        } else {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY;
            self.actor.text_id = if play.save.inv_content(ITEM_HOOKSHOT) != ITEM_NONE { 0x7010 } else { 0x700F };
            let a = self.actor.clone();
            offer_talk_default(play, &a);
        }
    }

    /// `func_80B41798`: back to blocking as the box closes.
    fn func_80b41798(&mut self, play: &PlayState) {
        if play.message_state() == TEXT_STATE_CLOSING {
            self.action = SHEIK_ACTION_BLOCK_PEDESTAL;
            self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY);
        }
    }

    /// Her skeleton posed as `EnXc_OverrideLimbDraw` turns it while `unk_30C` is set: the torso by
    /// the torso's turn, the head by the head's.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], turns: Option<([i16; 3], [i16; 3])>) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if let Some((head, torso)) = turns {
                match limb {
                    SHEIK_LIMB_TORSO => {
                        rot[0] = rot[0].wrapping_add(torso[1]);
                        rot[1] = rot[1].wrapping_sub(torso[0]);
                    }
                    SHEIK_LIMB_HEAD => {
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

impl ActorImpl for EnXc {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnXc_Update`: `sActionFuncs[action]`.
    fn update(&mut self, play: &mut PlayState) {
        if !(0..ACTION_COUNT).contains(&self.action) {
            if !self.logged {
                log::debug!("En_Xc: The main mode is wrong!!!!!!!!!!!!!!!!!!!!!!!!!");
                self.logged = true;
            }
            return;
        }
        match self.action {
            // EnXc_ActionFunc20.
            SHEIK_ACTION_20 => self.func_80b3ec00(),
            // EnXc_ActionFunc21.
            SHEIK_ACTION_21 => self.func_80b3ec0c(play),
            // EnXc_ActionFunc22.
            SHEIK_ACTION_22 => {
                self.update_skel_anime();
                self.bg_check(play);
                self.set_eye_pattern(play);
                self.func_80b3ec90(play);
            }
            // EnXc_ActionFunc23 (func_80B3D6F0: EnXc_CalcXZAccel).
            SHEIK_ACTION_23 => {
                self.calc_xz_accel();
                self.update_skel_anime();
                self.bg_check(play);
                self.set_eye_pattern(play);
                self.set_walking_sfx(play);
                self.func_80b3ecd8();
            }
            // EnXc_ActionFunc24 .. 28: nothing.
            SHEIK_ACTION_24..=SHEIK_ACTION_28 => {}
            // EnXc_ActionFunc30 .. 44: type 2's, the Prelude's script.
            SHEIK_ACTION_30..=SHEIK_ACTION_44 => {
                // milestone 3: type 2's actions (the Prelude: the harp, the song taught).
                if !self.logged {
                    log::warn!("En_Xc: action {} (type 2's, the Prelude) isn't ported yet: milestone 3", self.action);
                    self.logged = true;
                }
            }
            // EnXc_BlockingPedestalAction.
            SHEIK_ACTION_BLOCK_PEDESTAL => {
                self.bg_check(play);
                self.update_collider(play);
                self.calculate_head_turn(play);
                self.update_skel_anime();
                self.set_eye_pattern(play);
                self.setup_dialogue_action(play);
            }
            // EnXc_ActionFunc80.
            SHEIK_ACTION_IN_DIALOGUE => {
                self.bg_check(play);
                self.update_collider(play);
                self.calculate_head_turn(play);
                self.update_skel_anime();
                self.set_eye_pattern(play);
                self.func_80b41798(play);
            }
            a => {
                // The other types' actions: their types are placeholders, never here.
                if !self.logged {
                    log::warn!("En_Xc: action {a} isn't ported");
                    self.logged = true;
                }
            }
        }
    }

    /// `EnXc_PostLimbDraw`: the focus 10 above her head (`EnXc_DrawDefault`'s).
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.draw_mode != SHEIK_DRAW_DEFAULT {
            return;
        }
        let turns = (self.unk_30c != 0).then_some((self.interact_info.head, self.interact_info.torso));
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, turns);
        if let Some(head) = bones.get(SHEIK_LIMB_HEAD - 1) {
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
        rs.switches = vec![self.draw_mode.max(0) as u32, self.eye_idx.clamp(0, 2) as u32, self.unk_30c as u32];
        rs
    }

    /// `EnXc_Draw`: by `drawMode`, nothing or `EnXc_DrawDefault` (the harp's, the Triforce's and
    /// the squint's are the other types').
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.len() < 3 || rs.angles.len() < 6 {
            return;
        }
        let Some(joints) = &rs.joints else { return };
        match rs.switches[0] as i32 {
            SHEIK_DRAW_NOTHING => return,
            SHEIK_DRAW_DEFAULT => {}
            m => {
                log::debug!("En_Xc: draw mode {m} (the other types') isn't ported");
                return;
            }
        }
        let a = &rs.angles;
        let turns = (rs.switches[2] != 0).then_some(([a[0], a[1], a[2]], [a[3], a[4], a[5]]));
        let bones = Self::pose(&self.skeleton, &joints.rot, turns);
        let eye = self.eyes[(rs.switches[1] as usize).min(EYE_TEXTURES.len() - 1)].clone();
        out.opa.push(DrawCmd {
            mesh: MeshKey::named(keys::bake(BAKE)),
            transform: actor_draw_matrix(rs),
            bones,
            params: DrawParams { texture_images: vec![SourceImage { source: SEG_EYE, image: eye }], ..Default::default() },
        });
    }

    /// `EnXc_Destroy`: `Collider_DestroyCylinder`.
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
