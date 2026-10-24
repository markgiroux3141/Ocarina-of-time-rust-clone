//! `En_Xc` (`ovl_En_Xc/z_en_xc.c`): Sheik. Her params pick one of ten uses; this phase ports the
//! Temple of Time's (types 9, 1 and 2) and the pedestal block they lead to.
//!
//! **Type 9, the Temple of Time** (room 0 of every layer 0 to 3; `EnXc_InitTempleOfTime`): gone for
//! a child. An adult without `EVENTCHKINF_C5` sets it and starts `gTempleOfTimeFirstAdultCs` (the
//! scene's layer-4 script: the awakening), then runs type 1's actions. With `EVENTCHKINF_48` (the
//! Forest Temple's blue warp) and no `EVENTCHKINF_55` she sets 0x55, gives the Prelude of Light
//! (`Item_Give(ITEM_SONG_PRELUDE)`) and starts `gTempleOfTimePreludeCs`, running type 2's actions
//! from `SHEIK_ACTION_30`. Otherwise, without 0x55, she blocks the pedestal; with it she's gone.
//!
//! **Type 2, the Prelude** (`gTempleOfTimePreludeCs`'s Sheik; placed on the Temple of Time's
//! layer 6; `func_80B3EE64`: `SHEIK_ACTION_SERENADE`, the beta Serenade's first action, whose
//! `EnXc_SerenadeCS` lets any type but 8 through at once to `SHEIK_ACTION_30`). Each action 30 to
//! 43 runs one of the songs' shared actions (6 to 18, the Bolero's and the Minuet's) and steps on
//! once that action has moved (`EnXc_CheckAndSetAction`), all on channel 4's cues:
//! - 30 (`EnXc_ActionFunc21`): hidden until a cue other than 1 puts her at its start, drawn.
//! - 31 (`EnXc_ActionFunc6`, `func_80B3C588`: held at the cue's start and rotation): idling until
//!   cue 3, 11, 12, 13 or 23 (`func_80B3DAF0`) has her pull out the harp
//!   (`gSheikPullingOutHarpAnim` once, morph -4; `SHEIK_DRAW_PULLING_OUT_HARP`).
//! - 32 (`EnXc_ActionFunc7`): that done, `gSheikInitialHarpAnim` once, `SHEIK_DRAW_HARP`.
//! - 33 (`EnXc_ActionFunc8`): that done, the cue's harp tune (`EnXc_GetCurrentHarpAnim`: 11
//!   `gSheikPlayingHarp3Anim`, 12 `_2`, 13 `_4`, 23 `gSheikPlayingHarpAnim`, else `_5`) looped,
//!   morph -8.
//! - 34 (`EnXc_ActionFunc9`): playing until cue 8 (`func_80B3DCA8`) holds the harp up
//!   (`gSheikInitialHarpAnim`'s last frame, speed 0, morph -8).
//! - 35 (`EnXc_ActionFunc10`, `EnXc_SetupHarpPutawayAction`): cue 5 puts the harp away (the
//!   initial harp played back, to 36); any cue but 8 plays again (back to 34, `func_80B3C7D4`).
//! - 36 (`EnXc_ActionFunc11`): that done, `gSheikPullingOutHarpAnim` backwards, the harp drawn
//!   from its frame 34. 37 (`EnXc_ActionFunc12`): that done, `gSheikIdleAnim`, `SHEIK_DRAW_DEFAULT`.
//! - 38 (`EnXc_ActionFunc13`): cue 4 (`EnXc_SetupReverseAccel`) turns her way of walking round
//!   (`world.rot.y + 0x8000`, her shape still on Link) and walks her backwards
//!   (`gSheikWalkingAnim` backwards, morph -12). 39 (`EnXc_ReverseAccelerate`): still for 9
//!   frames, then at 1.2 (the ramp never runs); on to 40 on the 12th. 40 (`EnXc_ActionFunc40`, `func_80B3F010`): until 140 from
//!   Link, then idling (morph -12). Her steps on frames 11 and 23.
//! - 41 (`EnXc_HaltAndWaitToThrowNut`): slowing to a stop over 3 frames; after 12,
//!   `gSheikThrowingNutAnim` once. 42 (`EnXc_ThrowNut`): her step on frame 7,
//!   `NA_SE_VO_SK_SHOUT` on 20; on the 30th frame a cutscene nut (`En_Arrow`'s `ARROW_CS_NUT`) 30
//!   in front of her, 3 up, pitched 0xFA0. 43 (`EnXc_Delete`): cue 9 and she's gone
//!   (`EnXc_SetupDisappear`; in Death Mountain Crater she'd fade instead). 44 does nothing.
//!
//! `EnXc_InitFlame` (the Bolero's flame, `En_Light` 5 on channel 0's cue, actions 13 to 19 and
//! 38 to 43) only acts in Death Mountain Crater: there, logged (type 7's, Phase 15).
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
//! Drawn by `drawMode`: nothing, `EnXc_DrawDefault` (`gSheikSkel` opaque, the eye on segments 8
//! and 9; once at the pedestal, `unk_30C`, her torso (limb 9) and head (16) turned by the
//! tracking; the focus 10 above her head), `EnXc_DrawPullingOutHarp` (limb 12's list
//! `gSheikHarpDL`, her hand holding the harp, from the animation's frame 34:
//! `EnXc_PullingOutHarpOverrideLimbDraw`, `func_80B3E87C`) or `EnXc_DrawHarp` (limb 12 always the
//! harp: `EnXc_HarpOverrideLimbDraw`); the harp's draws neither turn her nor move the focus. Their
//! `gDPSetPrimColor` (255, 255, 20, 0) and `gDPSetEnvColor` (60, 0, 0, 0) aren't baked: every
//! limb's list sets its own prim colour before its triangles, and the harp's sets the env colour
//! where its strings take it (`ootx dl-dump object_xc gSheikHarpDL`: 225, 225, 20 and 60, 0, 0),
//! as `EnXc_DrawDefault` (which sets neither) draws the same lists. Her circle shadow (30) is the shape's. Not drawn: `func_8002EBCC`'s look-at (the lists'
//! reflections).
//!
//! Ported, besides the above: the shared code they use (`EnXc_InitCollider`, `_UpdateCollider`,
//! `_Destroy`, `_CalculateHeadTurn`, `_SetEyePattern`, `_BgCheck`, `_UpdateSkelAnime`,
//! `_GetCue`, `_CheckForCue`, `_CheckForNoCue`, `func_80B3C588`, `_ChangeAnimation`,
//! `_CheckAndSetAction`, `func_80B3C7D4`, `_SerenadeCS` (type 2's way), `_SetWalkingSFX`,
//! `_SetNutThrowSFX`, `_SpawnNut`, `_InitFlame` (outside the crater), `_GetCurrentHarpAnim`,
//! `_CalcXZAccel`, `func_80B3D644`, `_CalcXZSpeed`, `func_80B3D6F0`, `func_80B3D710`,
//! `func_80B3D730`, actions 6 to 18 and their setups, `EnXc_SetupDialogueAction`,
//! `func_80B41798`, `_Update`, `_Init`, `_OverrideLimbDraw`, `_PostLimbDraw`, `_DrawNothing`,
//! `_DrawDefault`, `_Draw`). **Not ported (the user's call):** the other types, placeholders
//! chosen at init (`EnXc_Init`'s switch: their checks, then a `log::info!`): 6 (the Minuet, Sacred
//! Forest Meadow: Phase 14), 7 (the Bolero, Death Mountain Crater: Phase 15), 8 (the Serenade, the
//! Ice Cavern: Phase 16; `EnXc_SerenadeCS`'s boots check is theirs), 3 (Lake Hylia after the
//! Water Temple: Phase 16), 5 (the Nocturne, Kakariko: Phase 17), 4 (the reveal, the Temple of
//! Time's layers 12 and 13: Phase 19), 0 (the songs' scripts on their cutscene layers: Phases 14
//! to 16) and any other. The Triforce's draw and the squint are theirs.
//!
//! Params: `params` whole, the type (`EnXcType`: 0 the songs' scripts, 1 the awakening, 2 the
//! Prelude, 3 Lake Hylia, 4 the reveal, 5 the Nocturne, 6 the Minuet, 7 the Bolero, 8 the
//! Serenade, 9 the Temple of Time).
//! Flags: type 9 reads and sets EVENTCHKINF_C5 (the awakening seen) and EVENTCHKINF_55 (the
//! Prelude learned), reads EVENTCHKINF_48 (the Forest Medallion's warp); type 2 reads none; the
//! placeholders' checks read EVENTCHKINF_50 (the Minuet learned, type 6), EVENTCHKINF_51 (the
//! Bolero, 7) and EVENTCHKINF_52 (the Serenade, 8, and the Iron Boots owned).

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
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;
use oot_game::surface::SurfaceType;

use crate::en_arrow::{ACTOR_EN_ARROW, ARROW_CS_NUT};
use crate::en_md::EVENTCHKINF_48;

/// `NA_SE_VO_SK_SHOUT` (`voicebank_table.h`: 0x6874).
pub const NA_SE_VO_SK_SHOUT: u16 = 0x6874;
/// `SCENE_DEATH_MOUNTAIN_CRATER` (`scene_table.h`: 0x61).
const SCENE_DEATH_MOUNTAIN_CRATER: u16 = 0x61;

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
    "which of Sheik's scenes (1, 2 and 9 ported, the others placeholders)",
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
pub const SHEIK_ACTION_STOPPED: i32 = 6;
pub const SHEIK_ACTION_7: i32 = 7;
pub const SHEIK_ACTION_HARP_READY: i32 = 8;
pub const SHEIK_PLAYING_HARP: i32 = 9;
pub const SHEIK_ACTION_10: i32 = 10;
pub const SHEIK_ACTION_PUT_HARP_AWAY: i32 = 11;
pub const SHEIK_ACTION_12: i32 = 12;
pub const SHEIK_ACTION_13: i32 = 13;
pub const SHEIK_ACTION_REVERSE_ACCEL: i32 = 14;
pub const SHEIK_ACTION_REVERSE_WALK: i32 = 15;
pub const SHEIK_ACTION_REVERSE_HALT: i32 = 16;
pub const SHEIK_ACTION_THROW_NUT: i32 = 17;
pub const SHEIK_ACTION_DELETE: i32 = 18;
pub const SHEIK_ACTION_FADE: i32 = 19;
pub const SHEIK_ACTION_20: i32 = 20;
pub const SHEIK_ACTION_21: i32 = 21;
pub const SHEIK_ACTION_22: i32 = 22;
pub const SHEIK_ACTION_23: i32 = 23;
pub const SHEIK_ACTION_24: i32 = 24;
pub const SHEIK_ACTION_28: i32 = 28;
pub const SHEIK_ACTION_SERENADE: i32 = 29;
pub const SHEIK_ACTION_30: i32 = 30;
pub const SHEIK_ACTION_31: i32 = 31;
pub const SHEIK_ACTION_32: i32 = 32;
pub const SHEIK_ACTION_33: i32 = 33;
pub const SHEIK_ACTION_34: i32 = 34;
pub const SHEIK_ACTION_35: i32 = 35;
pub const SHEIK_ACTION_36: i32 = 36;
pub const SHEIK_ACTION_37: i32 = 37;
pub const SHEIK_ACTION_38: i32 = 38;
pub const SHEIK_ACTION_39: i32 = 39;
pub const SHEIK_ACTION_40: i32 = 40;
pub const SHEIK_ACTION_41: i32 = 41;
pub const SHEIK_ACTION_42: i32 = 42;
pub const SHEIK_ACTION_43: i32 = 43;
pub const SHEIK_ACTION_44: i32 = 44;
pub const SHEIK_ACTION_BLOCK_PEDESTAL: i32 = 79;
pub const SHEIK_ACTION_IN_DIALOGUE: i32 = 80;
/// `ARRAY_COUNT(sActionFuncs)`.
const ACTION_COUNT: i32 = 81;

/// `EnXcDrawMode`.
pub const SHEIK_DRAW_NOTHING: i32 = 0;
pub const SHEIK_DRAW_DEFAULT: i32 = 1;
pub const SHEIK_DRAW_PULLING_OUT_HARP: i32 = 2;
pub const SHEIK_DRAW_HARP: i32 = 3;

/// `sEyeTextures`.
pub const EYE_TEXTURES: [&str; 3] = ["gSheikEyeOpenTex", "gSheikEyeHalfClosedTex", "gSheikEyeShutTex"];
/// The eye's segment (8; `EnXc_DrawDefault` binds 9 to the same, her lists only call 8), where a
/// draw puts the blink's texture in the bake's place.
const SEG_EYE: u32 = 0x0800_0000;
/// `EnXc_DrawDefault`'s mesh (and `EnXc_DrawPullingOutHarp`'s before frame 34).
pub const BAKE: &str = "En_Xc/skel";
/// `EnXc_DrawHarp`'s mesh (and `EnXc_DrawPullingOutHarp`'s from frame 34): limb 12's list
/// `gSheikHarpDL`.
pub const HARP_BAKE: &str = "En_Xc/harp";

/// The limbs `EnXc_OverrideLimbDraw` and `_PostLimbDraw` name (1-based): the torso, the head;
/// the harp's draws' limb 12 (her hand, the harp in it).
const SHEIK_LIMB_TORSO: usize = 9;
const SHEIK_LIMB_HEAD: usize = 16;
const SHEIK_LIMB_HARP: usize = 12;
/// `func_80B3E87C`: the harp in her hand from this frame of `gSheikPullingOutHarpAnim`.
const HARP_OUT_FRAME: f32 = 34.0;
/// `EnXc_PostLimbDraw`'s `src`: the focus's offset from her head.
const FOCUS_OFFSET: Vec3 = Vec3::new(0.0, 10.0, 0.0);

/// `NPC_TRACKING_HEAD_AND_TORSO` (`actor.h`).
const NPC_TRACKING_HEAD_AND_TORSO: i16 = 2;
/// `kREG(17) + 0xC`: the tracking preset; `kREG(16) - 3.0`: the eyes' height; `kREG(2)`: 0,
/// `kREG(5)`: 0 (the debug registers start zeroed).
const TRACKING_PRESET: usize = 0xC;
const KREG_16: f32 = 0.0;
const KREG_2: f32 = 0.0;
const KREG_5: f32 = 0.0;

/// Her mesh (`Gfx_SetupDL_25Opa`; the eye on segments 8 and 9, the draw swapping its texture),
/// and the same with limb 12's list the harp (`EnXc_HarpOverrideLimbDraw`).
pub fn bakes() -> Vec<MeshBake> {
    let tex = |s: &str| BakeSegment::Texture { file: OBJECT.into(), symbol: s.into() };
    let skel = |name: &str, limbs: Vec<LimbOverride>| MeshBake {
        name: name.into(),
        object: OBJECT.into(),
        segments: vec![(0x08, tex(EYE_TEXTURES[0])), (0x09, tex(EYE_TEXTURES[0]))],
        prelude: Vec::new(),
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gSheikSkel".into(), limbs },
    };
    vec![
        skel(BAKE, Vec::new()),
        skel(HARP_BAKE, vec![LimbOverride { limb: (SHEIK_LIMB_HARP - 1) as u8, file: OBJECT.into(), symbol: "gSheikHarpDL".into() }]),
    ]
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
    anims: Anims,
    eyes: Vec<DrawImage>,
    logged: bool,
    logged_flame: bool,
}

/// The animations the ported actions play (`object_xc`).
struct Anims {
    idle: Anim,
    arms_crossed: Anim,
    walking: Anim,
    pulling_out_harp: Anim,
    initial_harp: Anim,
    /// `gSheikPlayingHarpAnim`, `_2Anim` .. `_5Anim`.
    playing_harp: [Anim; 5],
    throwing_nut: Anim,
}

impl Anims {
    fn load(assets: &oot_game::play_scene::GameAssets) -> anyhow::Result<Anims> {
        let a = |s: &str| assets.animation(OBJECT, s);
        Ok(Anims {
            idle: a("gSheikIdleAnim")?,
            arms_crossed: a("gSheikArmsCrossedIdleAnim")?,
            walking: a("gSheikWalkingAnim")?,
            pulling_out_harp: a("gSheikPullingOutHarpAnim")?,
            initial_harp: a("gSheikInitialHarpAnim")?,
            playing_harp: [a("gSheikPlayingHarpAnim")?, a("gSheikPlayingHarp2Anim")?, a("gSheikPlayingHarp3Anim")?, a("gSheikPlayingHarp4Anim")?, a("gSheikPlayingHarp5Anim")?],
            throwing_nut: a("gSheikThrowingNutAnim")?,
        })
    }
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
            SHEIK_TYPE_1 | SHEIK_TYPE_2 | SHEIK_TYPE_9 => None,
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
            let anims = Anims::load(&assets)?;
            let mut eyes = Vec::new();
            for s in EYE_TEXTURES {
                let t = assets.pack.texture(OBJECT, s)?;
                eyes.push(DrawImage { width: t.width, height: t.height, rgba: t.rgba.clone().into() });
            }
            Ok((skeleton, anims, eyes))
        })();
        let (skeleton, anims, eyes) = match loaded {
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
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anims.idle.clone())),
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
            anims,
            eyes,
            logged: false,
            logged_flame: false,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 30.0);
        this.collider.update(&this.actor);
        match ty {
            SHEIK_TYPE_1 => this.func_80b3ebf0(),
            // The beta Serenade's or the Prelude's.
            SHEIK_TYPE_2 => this.func_80b3ee64(),
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
            // Type 2's actions from here.
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

    /// `EnXc_CheckForCue`: cue `cue_id` on `channel`.
    fn check_for_cue(play: &PlayState, cue_id: u16, channel: usize) -> bool {
        Self::get_cue(play, channel).is_some_and(|c| c.action == cue_id)
    }

    /// `EnXc_CheckForNoCue`: a cue on `channel`, not `cue_id`.
    fn check_for_no_cue(play: &PlayState, cue_id: u16, channel: usize) -> bool {
        Self::get_cue(play, channel).is_some_and(|c| c.action != cue_id)
    }

    /// `func_80B3C588`: at the cue's start, turned as it is.
    fn func_80b3c588(&mut self, play: &PlayState, channel: usize) {
        if let Some(cue) = Self::get_cue(play, channel) {
            self.actor.world_pos = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
            self.actor.shape_rot.x = cue.rot[0];
            self.actor.world_rot.x = cue.rot[0];
            self.actor.shape_rot.y = cue.rot[1];
            self.actor.world_rot.y = cue.rot[1];
            self.actor.shape_rot.z = cue.rot[2];
            self.actor.world_rot.z = cue.rot[2];
        }
    }

    /// `EnXc_CheckAndSetAction`: on to `set` once the action isn't `check` any more.
    fn check_and_set_action(&mut self, check: i32, set: i32) {
        if check != self.action {
            self.action = set;
        }
    }

    /// `func_80B3C7D4`: once the action isn't `action1`, to `action2` if the harp's being put
    /// away, else `action3`.
    fn func_80b3c7d4(&mut self, action1: i32, action2: i32, action3: i32) {
        if action1 != self.action {
            self.action = if self.action == SHEIK_ACTION_PUT_HARP_AWAY { action2 } else { action3 };
        }
    }

    /// `EnXc_GetCurrentHarpAnim`: the tune by the cue on `index` (11, 12, 13, 23; else, or no
    /// cue, `gSheikPlayingHarp5Anim`).
    fn get_current_harp_anim(&self, play: &PlayState, index: usize) -> Anim {
        let h = &self.anims.playing_harp;
        match Self::get_cue(play, index).map(|c| c.action) {
            Some(11) => h[2].clone(),
            Some(12) => h[1].clone(),
            Some(13) => h[3].clone(),
            Some(23) => h[0].clone(),
            _ => h[4].clone(),
        }
    }

    /// `Animation_Change` from 0 to the last frame at speed 1.
    fn change_forward(&mut self, animation: Anim, mode: u8, morph_frames: f32) {
        let last = animation.last_frame();
        self.skel.change(animation, 1.0, 0.0, last, mode, morph_frames);
    }

    /// `Animation_Change` from the last frame back to 0 at speed -1.
    fn change_backward(&mut self, animation: Anim, mode: u8, morph_frames: f32) {
        let last = animation.last_frame();
        self.skel.change(animation, -1.0, last, 0.0, mode, morph_frames);
    }

    /// `EnXc_ChangeAnimation`: from the start (or, reversed, from the end) at speed 1.
    fn change_animation(&mut self, animation: Anim, mode: u8, morph_frames: f32, reverse: bool) {
        let frame_count = animation.last_frame();
        let (start, end, speed) = if !reverse { (0.0, frame_count, 1.0) } else { (frame_count, 0.0, -1.0) };
        self.skel.change(animation, speed, start, end, mode, morph_frames);
    }

    /// `func_80B3C9EC`: arms crossed before the pedestal.
    fn func_80b3c9ec(&mut self) {
        self.change_animation(self.anims.arms_crossed.clone(), ANIMMODE_LOOP, 0.0, false);
        self.action = SHEIK_ACTION_BLOCK_PEDESTAL;
        self.draw_mode = SHEIK_DRAW_DEFAULT;
        self.unk_30c = 1;
    }

    /// `Sfx_PlaySfxAtPos(&this->actor.projectedPos, ..)`.
    fn play_sfx(play: &mut PlayState, sfx: u16) {
        let pos = play.cur_actor.map(SfxPos::Actor).unwrap_or(SfxPos::Default);
        play.audio.play_sfx_at_pos(pos, sfx);
    }

    /// A step on her floor (`NA_SE_PL_WALK_GROUND` plus `SurfaceType_GetSfxOffset`), on the
    /// ground.
    fn step_sfx(&self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            let offset = play.audio.tables.surface_sfx_id(self.actor.floor_poly.map(|p| play.col.sfx_type(p)).unwrap_or(0));
            Self::play_sfx(play, NA_SE_PL_WALK_GROUND.wrapping_add(offset));
        }
    }

    /// `EnXc_SetWalkingSFX`: her steps on frames 11 and 23, on the ground.
    fn set_walking_sfx(&mut self, play: &mut PlayState) {
        if self.skel.on_frame(11.0) || self.skel.on_frame(23.0) {
            self.step_sfx(play);
        }
    }

    /// `EnXc_SetNutThrowSFX`: her step on frame 7 (on the ground), her shout on 20.
    fn set_nut_throw_sfx(&mut self, play: &mut PlayState) {
        if self.skel.on_frame(7.0) {
            self.step_sfx(play);
        }
        if self.skel.on_frame(20.0) {
            Self::play_sfx(play, NA_SE_VO_SK_SHOUT);
        }
    }

    /// `EnXc_SpawnNut`: a cutscene's Deku Nut 30 in front of her, 3 up, pitched 0xFA0.
    fn spawn_nut(&self, play: &mut PlayState) {
        let pos = self.actor.world_pos;
        let angle = self.actor.shape_rot.y;
        let at = Vec3::new(eng_math::sin_s(angle) * 30.0 + pos.x, pos.y + 3.0, eng_math::cos_s(angle) * 30.0 + pos.z);
        let _ = play.actor_spawn(ACTOR_EN_ARROW, at, [0xFA0, self.actor.shape_rot.y, 0], ARROW_CS_NUT);
    }

    /// `EnXc_InitFlame`: in Death Mountain Crater only, the Bolero's flame (`En_Light` 5) kept on
    /// channel 0's cue, and Sheik gone with it on cue 1. Not ported (type 7's, Phase 15): logged.
    fn init_flame(&mut self, play: &PlayState) {
        if play.scene_id == SCENE_DEATH_MOUNTAIN_CRATER && Self::get_cue(play, 0).is_some() && !self.logged_flame {
            // not ported: EnXc_SpawnFlame, _SetupFlamePos, _DestroyFlame (the Bolero, Phase 15).
            log::warn!("En_Xc: the Bolero's flame (EnXc_InitFlame) isn't ported");
            self.logged_flame = true;
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

    /// `EnXc_CalcXZSpeed`: slowing from `kREG(2) * 0.01 + 1.2` to a stop over 3 frames; moved.
    fn calc_xz_speed(&mut self) {
        let timer = self.timer;
        if timer < 3.0 {
            self.actor.speed_xz = (((KREG_2 * 0.01) + 1.2) / 3.0) * (3.0 - timer);
        } else {
            self.actor.speed_xz = 0.0;
        }
        // Actor_MoveXZGravity.
        self.actor.move_forward();
    }

    /// `func_80B3D710` (and `func_80B3D644`): `Actor_MoveXZGravity`.
    fn func_80b3d710(&mut self) {
        self.actor.move_forward();
    }

    // The songs' shared actions (6 to 18; type 2's 31 to 43 run them).

    /// `func_80B3DAF0`: cue 3, 11, 12, 13 or 23 on channel 4: pulling out the harp.
    fn func_80b3daf0(&mut self, play: &PlayState) {
        if let Some(cue) = Self::get_cue(play, 4)
            && matches!(cue.action, 3 | 11 | 12 | 13 | 23)
        {
            self.change_forward(self.anims.pulling_out_harp.clone(), ANIMMODE_ONCE, -4.0);
            self.action = SHEIK_ACTION_7;
            self.draw_mode = SHEIK_DRAW_PULLING_OUT_HARP;
        }
    }

    /// `EnXc_SetupInitialHarpAction`: the harp out, raised (`gSheikInitialHarpAnim` once).
    fn setup_initial_harp_action(&mut self, anim_finished: bool) {
        if anim_finished {
            self.change_forward(self.anims.initial_harp.clone(), ANIMMODE_ONCE, 0.0);
            self.action = SHEIK_ACTION_HARP_READY;
            self.draw_mode = SHEIK_DRAW_HARP;
        }
    }

    /// `EnXc_SetupPlayingHarpAction`: the cue's tune, looped (morph -8).
    fn setup_playing_harp_action(&mut self, play: &PlayState, anim_finished: bool) {
        if anim_finished {
            let animation = self.get_current_harp_anim(play, 4);
            self.change_forward(animation, ANIMMODE_LOOP, -8.0);
            self.action = SHEIK_PLAYING_HARP;
            self.draw_mode = SHEIK_DRAW_HARP;
        }
    }

    /// `func_80B3DCA8`: cue 8: the harp held up (`gSheikInitialHarpAnim`'s last frame, speed 0).
    fn func_80b3dca8(&mut self, play: &PlayState) {
        if play.cs_ctx.state != CS_STATE_IDLE
            && let Some(cue) = play.cs_ctx.npc_actions[4]
            && cue.action == 8
        {
            let frame_count = self.anims.initial_harp.last_frame();
            self.skel.change(self.anims.initial_harp.clone(), 0.0, frame_count, frame_count, ANIMMODE_LOOP, -8.0);
            self.action = SHEIK_ACTION_10;
        }
    }

    /// `EnXc_SetupHarpPutawayAction`: cue 5, the hold done: the initial harp played back; any
    /// cue but 8: playing again.
    fn setup_harp_putaway_action(&mut self, play: &PlayState) {
        if Self::check_for_cue(play, 5, 4) {
            if self.skel.cur_frame >= self.skel.end_frame {
                self.change_backward(self.anims.initial_harp.clone(), ANIMMODE_ONCE, 0.0);
                self.action = SHEIK_ACTION_PUT_HARP_AWAY;
            }
        } else if Self::check_for_no_cue(play, 8, 4) {
            self.setup_playing_harp_action(play, true);
        }
    }

    /// `func_80B3DE00`: that done, the harp put away (`gSheikPullingOutHarpAnim` backwards).
    fn func_80b3de00(&mut self, anim_finished: bool) {
        if anim_finished {
            self.change_backward(self.anims.pulling_out_harp.clone(), ANIMMODE_ONCE, 0.0);
            self.action = SHEIK_ACTION_12;
            self.draw_mode = SHEIK_DRAW_PULLING_OUT_HARP;
        }
    }

    /// `func_80B3DE78`: that done, idling, drawn as ever.
    fn func_80b3de78(&mut self, anim_finished: bool) {
        if anim_finished {
            self.change_forward(self.anims.idle.clone(), ANIMMODE_LOOP, 0.0);
            self.action = SHEIK_ACTION_13;
            self.draw_mode = SHEIK_DRAW_DEFAULT;
            self.timer = 0.0;
        }
    }

    /// `EnXc_SetupReverseAccel`: cue 4: walking backwards away (her walk turned round, her
    /// shape not).
    fn setup_reverse_accel(&mut self, play: &PlayState) {
        if play.cs_ctx.state != CS_STATE_IDLE
            && let Some(cue) = play.cs_ctx.npc_actions[4]
            && cue.action == 4
        {
            self.change_backward(self.anims.walking.clone(), ANIMMODE_LOOP, -12.0);
            self.action = SHEIK_ACTION_REVERSE_ACCEL;
            self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(0x8000u16 as i16);
            self.timer = 0.0;
        }
    }

    /// `EnXc_SetupReverseWalkAction`: at full speed after 12 frames.
    fn setup_reverse_walk_action(&mut self) {
        self.timer += 1.0;
        if self.timer >= 12.0 {
            self.actor.speed_xz = (KREG_2 * 0.01) + 1.2;
            self.action = SHEIK_ACTION_REVERSE_WALK;
        }
    }

    /// `EnXc_SetupReverseHaltAction`: 140 from Link: idling (morph -12).
    fn setup_reverse_halt_action(&mut self) {
        if self.actor.xz_dist_to_player >= KREG_5 + 140.0 {
            self.change_forward(self.anims.idle.clone(), ANIMMODE_LOOP, -12.0);
            self.action = SHEIK_ACTION_REVERSE_HALT;
            self.timer = 0.0;
        }
    }

    /// `EnXc_SetupNutThrow`: after 12 frames, throwing the nut (once), stopped.
    fn setup_nut_throw(&mut self) {
        self.timer += 1.0;
        if self.timer >= 12.0 {
            self.change_forward(self.anims.throwing_nut.clone(), ANIMMODE_ONCE, 0.0);
            self.action = SHEIK_ACTION_THROW_NUT;
            self.timer = 0.0;
            self.actor.speed_xz = 0.0;
        }
    }

    /// `func_80B3E164`: the nut on the 30th frame.
    fn func_80b3e164(&mut self, play: &mut PlayState) {
        self.timer += 1.0;
        if self.timer >= 30.0 {
            self.action = SHEIK_ACTION_DELETE;
            self.spawn_nut(play);
        }
    }

    /// `EnXc_SetupDisappear`: cue 9: fading in Death Mountain Crater (the Bolero's end), else gone.
    fn setup_disappear(&mut self, play: &PlayState) {
        if play.cs_ctx.state != CS_STATE_IDLE
            && let Some(cue) = play.cs_ctx.npc_actions[4]
            && cue.action == 9
        {
            if play.scene_id == SCENE_DEATH_MOUNTAIN_CRATER {
                self.action = SHEIK_ACTION_FADE;
                self.draw_mode = SHEIK_DRAW_NOTHING;
                self.actor.shape_shadow_alpha = 0;
            } else {
                self.actor.kill();
            }
        }
    }

    /// `EnXc_ActionFunc6`: idling until the harp comes out.
    fn action_func6(&mut self, play: &mut PlayState) {
        self.update_skel_anime();
        self.bg_check(play);
        self.set_eye_pattern(play);
        self.func_80b3daf0(play);
    }

    /// `EnXc_ActionFunc7`: pulling out the harp.
    fn action_func7(&mut self, play: &mut PlayState) {
        let anim_finished = self.update_skel_anime();
        self.bg_check(play);
        self.set_eye_pattern(play);
        self.setup_initial_harp_action(anim_finished);
    }

    /// `EnXc_ActionFunc8`: raising it.
    fn action_func8(&mut self, play: &mut PlayState) {
        let anim_finished = self.update_skel_anime();
        self.bg_check(play);
        self.set_eye_pattern(play);
        self.setup_playing_harp_action(play, anim_finished);
    }

    /// `EnXc_ActionFunc9`: playing.
    fn action_func9(&mut self, play: &mut PlayState) {
        self.update_skel_anime();
        self.bg_check(play);
        self.set_eye_pattern(play);
        self.func_80b3dca8(play);
    }

    /// `EnXc_ActionFunc10`: holding it up.
    fn action_func10(&mut self, play: &mut PlayState) {
        self.update_skel_anime();
        self.bg_check(play);
        self.set_eye_pattern(play);
        self.setup_harp_putaway_action(play);
    }

    /// `EnXc_ActionFunc11`: lowering it.
    fn action_func11(&mut self, play: &mut PlayState) {
        let anim_finished = self.update_skel_anime();
        self.bg_check(play);
        self.set_eye_pattern(play);
        self.func_80b3de00(anim_finished);
    }

    /// `EnXc_ActionFunc12`: putting it away.
    fn action_func12(&mut self, play: &mut PlayState) {
        let anim_finished = self.update_skel_anime();
        self.bg_check(play);
        self.set_eye_pattern(play);
        self.func_80b3de78(anim_finished);
    }

    /// `EnXc_ActionFunc13`: idling until she leaves.
    fn action_func13(&mut self, play: &mut PlayState) {
        self.update_skel_anime();
        self.bg_check(play);
        self.set_eye_pattern(play);
        self.init_flame(play);
        self.setup_reverse_accel(play);
    }

    /// `EnXc_ReverseAccelerate` (`func_80B3D6F0`: `EnXc_CalcXZAccel`).
    fn reverse_accelerate(&mut self, play: &mut PlayState) {
        self.calc_xz_accel();
        self.update_skel_anime();
        self.bg_check(play);
        self.set_eye_pattern(play);
        self.set_walking_sfx(play);
        self.init_flame(play);
        self.setup_reverse_walk_action();
    }

    /// `EnXc_ActionFunc15`: walking backwards until 140 from Link.
    fn action_func15(&mut self, play: &mut PlayState) {
        self.func_80b3d710();
        self.update_skel_anime();
        self.bg_check(play);
        self.set_eye_pattern(play);
        self.set_walking_sfx(play);
        self.init_flame(play);
        self.setup_reverse_halt_action();
    }

    /// `EnXc_HaltAndWaitToThrowNut` (`func_80B3D730`: `EnXc_CalcXZSpeed`).
    fn halt_and_wait_to_throw_nut(&mut self, play: &mut PlayState) {
        self.calc_xz_speed();
        self.update_skel_anime();
        self.bg_check(play);
        self.set_eye_pattern(play);
        self.set_walking_sfx(play);
        self.init_flame(play);
        self.setup_nut_throw();
    }

    /// `EnXc_ThrowNut`.
    fn throw_nut(&mut self, play: &mut PlayState) {
        self.update_skel_anime();
        self.bg_check(play);
        self.set_eye_pattern(play);
        self.set_nut_throw_sfx(play);
        self.init_flame(play);
        self.func_80b3e164(play);
    }

    /// `EnXc_Delete`: waiting for cue 9.
    fn delete(&mut self, play: &mut PlayState) {
        self.update_skel_anime();
        self.bg_check(play);
        self.set_eye_pattern(play);
        self.init_flame(play);
        self.setup_disappear(play);
    }

    // Type 2.

    /// `func_80B3EE64`.
    fn func_80b3ee64(&mut self) {
        self.action = SHEIK_ACTION_SERENADE;
    }

    /// `EnXc_SerenadeCS`: true at once for any type but 8 (the Ice Cavern's, which waits for the
    /// Iron Boots taken: a placeholder, never here).
    fn serenade_cs(&self) -> bool {
        if TYPE.get(self.actor.params) == SHEIK_TYPE_SERENADE {
            // not ported: type 8's boots check (gIceCavernSerenadeCs, EVENTCHKINF_52,
            // ITEM_SONG_SERENADE: Phase 16).
            return false;
        }
        true
    }

    /// `func_80B3EE74`: on to the Prelude's actions.
    fn func_80b3ee74(&mut self) {
        if self.serenade_cs() {
            self.action = SHEIK_ACTION_30;
        }
    }

    /// `func_80B3F010`: 140 from Link: idling (morph -12), to 41.
    fn func_80b3f010(&mut self) {
        if KREG_5 + 140.0 <= self.actor.xz_dist_to_player {
            self.change_forward(self.anims.idle.clone(), ANIMMODE_LOOP, -12.0);
            self.action = SHEIK_ACTION_41;
            self.timer = 0.0;
        }
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

    /// Whether limb 12 is drawn as the harp: always in `EnXc_DrawHarp`
    /// (`EnXc_HarpOverrideLimbDraw`), from frame 34 in `EnXc_DrawPullingOutHarp`
    /// (`EnXc_PullingOutHarpOverrideLimbDraw`, `func_80B3E87C`).
    pub fn harp_drawn(&self) -> bool {
        match self.draw_mode {
            SHEIK_DRAW_HARP => true,
            SHEIK_DRAW_PULLING_OUT_HARP => self.skel.cur_frame >= HARP_OUT_FRAME,
            _ => false,
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
            // The songs' shared actions (the Minuet's and the Bolero's, placeholders; type 2's
            // 31 to 43 run them).
            SHEIK_ACTION_STOPPED => self.action_func6(play),
            SHEIK_ACTION_7 => self.action_func7(play),
            SHEIK_ACTION_HARP_READY => self.action_func8(play),
            SHEIK_PLAYING_HARP => self.action_func9(play),
            SHEIK_ACTION_10 => self.action_func10(play),
            SHEIK_ACTION_PUT_HARP_AWAY => self.action_func11(play),
            SHEIK_ACTION_12 => self.action_func12(play),
            SHEIK_ACTION_13 => self.action_func13(play),
            SHEIK_ACTION_REVERSE_ACCEL => self.reverse_accelerate(play),
            SHEIK_ACTION_REVERSE_WALK => self.action_func15(play),
            SHEIK_ACTION_REVERSE_HALT => self.halt_and_wait_to_throw_nut(play),
            SHEIK_ACTION_THROW_NUT => self.throw_nut(play),
            SHEIK_ACTION_DELETE => self.delete(play),
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
            // EnXc_Serenade.
            SHEIK_ACTION_SERENADE => self.func_80b3ee74(),
            // EnXc_ActionFunc30 .. 44: type 2's, the Prelude's script.
            SHEIK_ACTION_30 => {
                // EnXc_ActionFunc21, func_80B3EEA4.
                self.func_80b3ec0c(play);
                self.check_and_set_action(SHEIK_ACTION_30, SHEIK_ACTION_31);
            }
            SHEIK_ACTION_31 => {
                // func_80B3EEC8.
                self.action_func6(play);
                self.func_80b3c588(play, 4);
                self.check_and_set_action(SHEIK_ACTION_31, SHEIK_ACTION_32);
            }
            SHEIK_ACTION_32 => {
                // func_80B3EEEC.
                self.action_func7(play);
                self.check_and_set_action(SHEIK_ACTION_32, SHEIK_ACTION_33);
            }
            SHEIK_ACTION_33 => {
                // func_80B3EF10.
                self.action_func8(play);
                self.check_and_set_action(SHEIK_ACTION_33, SHEIK_ACTION_34);
            }
            SHEIK_ACTION_34 => {
                // func_80B3EF34.
                self.action_func9(play);
                self.check_and_set_action(SHEIK_ACTION_34, SHEIK_ACTION_35);
            }
            SHEIK_ACTION_35 => {
                // func_80B3EF58: put away to 36, playing again back to 34.
                self.action_func10(play);
                self.func_80b3c7d4(SHEIK_ACTION_35, SHEIK_ACTION_36, SHEIK_ACTION_34);
            }
            SHEIK_ACTION_36 => {
                // func_80B3EF80.
                self.action_func11(play);
                self.check_and_set_action(SHEIK_ACTION_36, SHEIK_ACTION_37);
            }
            SHEIK_ACTION_37 => {
                // func_80B3EFA4.
                self.action_func12(play);
                self.check_and_set_action(SHEIK_ACTION_37, SHEIK_ACTION_38);
            }
            SHEIK_ACTION_38 => {
                // func_80B3EFC8.
                self.action_func13(play);
                self.check_and_set_action(SHEIK_ACTION_38, SHEIK_ACTION_39);
            }
            SHEIK_ACTION_39 => {
                // func_80B3EFEC.
                self.reverse_accelerate(play);
                self.check_and_set_action(SHEIK_ACTION_39, SHEIK_ACTION_40);
            }
            SHEIK_ACTION_40 => {
                // EnXc_ActionFunc40.
                self.func_80b3d710();
                self.update_skel_anime();
                self.bg_check(play);
                self.set_eye_pattern(play);
                self.set_walking_sfx(play);
                self.func_80b3f010();
            }
            SHEIK_ACTION_41 => {
                // func_80B3F0B8.
                self.halt_and_wait_to_throw_nut(play);
                self.check_and_set_action(SHEIK_ACTION_41, SHEIK_ACTION_42);
            }
            SHEIK_ACTION_42 => {
                // func_80B3F0DC.
                self.throw_nut(play);
                self.check_and_set_action(SHEIK_ACTION_42, SHEIK_ACTION_43);
            }
            SHEIK_ACTION_43 => {
                // func_80B3F100.
                self.delete(play);
                self.check_and_set_action(SHEIK_ACTION_43, SHEIK_ACTION_44);
            }
            // EnXc_ActionFunc44: nothing.
            SHEIK_ACTION_44 => {}
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
        rs.switches = vec![self.draw_mode.max(0) as u32, self.eye_idx.clamp(0, 2) as u32, self.unk_30c as u32, self.harp_drawn() as u32];
        rs
    }

    /// `EnXc_Draw`: by `drawMode`, nothing, `EnXc_DrawDefault`, `EnXc_DrawPullingOutHarp` or
    /// `EnXc_DrawHarp` (the Triforce's and the squint's are the other types').
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.len() < 4 || rs.angles.len() < 6 {
            return;
        }
        let Some(joints) = &rs.joints else { return };
        // EnXc_DrawDefault turns her (EnXc_OverrideLimbDraw); the harp's draws only swap limb
        // 12's list.
        let default = match rs.switches[0] as i32 {
            SHEIK_DRAW_NOTHING => return,
            SHEIK_DRAW_DEFAULT => true,
            SHEIK_DRAW_PULLING_OUT_HARP | SHEIK_DRAW_HARP => false,
            m => {
                log::debug!("En_Xc: draw mode {m} (the other types') isn't ported");
                return;
            }
        };
        let a = &rs.angles;
        let turns = (default && rs.switches[2] != 0).then_some(([a[0], a[1], a[2]], [a[3], a[4], a[5]]));
        let bones = Self::pose(&self.skeleton, &joints.rot, turns);
        let eye = self.eyes[(rs.switches[1] as usize).min(EYE_TEXTURES.len() - 1)].clone();
        let mesh = if rs.switches[3] != 0 { HARP_BAKE } else { BAKE };
        out.opa.push(DrawCmd {
            mesh: MeshKey::named(keys::bake(mesh)),
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
