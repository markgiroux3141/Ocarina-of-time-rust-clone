//! `En_Zo` (`ovl_En_Zo/z_en_zo.c`, its effects `z_en_zo_eff.c` in the same file): the Zoras of
//! Zora's Domain (types 0 to 5 in room 1), and the lake's and the fountain's (6 to 8).
//!
//! **At his init** he stands (`EnZo_Standing`: a circle shadow of 24, `gZoraIdleAnim`, talking,
//! fully drawn) when he's less than 54 under water (`Actor_UpdateBgCheckInfo` at once) or of type
//! 8; else he waits submerged (`EnZo_Submerged`: unseen, alpha 0, not targetable). Type 8 is
//! killed for an adult.
//!
//! **Under water** (`EnZo_PlayerInProximity`: Link within 240 across and 80 up or down of the
//! water's surface above him) he rises at 4 (`EnZo_Surface`), from 80 deep easing to 2 and fading
//! in; at 54 he breaks the surface (`NA_SE_EV_OUT_OF_WATER`, 20 splashes), treads water
//! (`EnZo_TreadWater`: `gZoraSurfaceAnim` from 1 held at its end, bobbing round 54 deep, a ripple
//! every 12 frames and two between), and once the animation has played through talks and faces
//! Link whole (`NPC_TRACKING_FULL_BODY`). With Link gone (40 to 80 frames after he last was near)
//! he dives (`EnZo_Dive`: `gZoraSurfaceAnim` backwards from its end to frame 8, then
//! `NA_SE_EV_DIVE_WATER`, the splashes, down at 4 easing to 1 from 80 deep or on the floor,
//! fading out); faded, he's back at his home, submerged.
//!
//! **Standing**, his limbs sway (`Actor_UpdateFidgetTables`), and with Link in front of him (less
//! than 0x4718 off his facing) and near, his head and torso follow him
//! (`NPC_TRACKING_HEAD_AND_TORSO`), the whole body while talking. The tracking (`EnZo_Dialog`,
//! `Npc_TrackPoint` preset 11) looks at Link from 10 up for a child (-10 for an adult) standing,
//! at the Zora's own height in water.
//!
//! **His texts** (`EnZo_GetTextId`, within his radius and 30): a mask's reaction first
//! (`MASK_REACTION_SET_ZORA`); by type (the params' low six bits), after `EVENTCHKINF_30` (any
//! Zora's talk closed) and the Zora's Sapphire, else 0x4006: type 0 0x4007, 0x402D; 1 0x4008 then
//! 0x4009 (`INFTABLE_124`), 0x402E; 2 0x400A, then with Ruto's letter found (`EVENTCHKINF_31`)
//! 0x402F then 0x400B (`INFTABLE_129`), 0x402D; 3 0x400C and its choice (0x400D, 0x400E), 0x402E;
//! 4 0x400F, 0x4010 once King Zora has the letter, 0x402D; 5 0x4011, 0x402E; 6 0x4020; 7 0x4021;
//! 8 0x402A after `EVENTCHKINF_30`. Every talk closing sets `EVENTCHKINF_30` but 0x4020's and
//! 0x4021's (`EnZo_UpdateTalkState`). Talking, 0x4006 has him tap his foot
//! (`gZoraHandsOnHipsTappingFootAnim`), 0x4007 open his arms (`gZoraOpenArmsAnim`); done, back to
//! standing or treading (`EnZo_SetAnimation`).
//!
//! **His effects** (`EnZoEffect`, 15 slots, his own): bubbles rise from him every other 8 frames
//! (`play->state.frames & 8`) below the surface, wobbling round where they began, and become
//! ripples at it; ripples grow to their size and fade by 20 a frame; splashes fly under gravity
//! (to -20) and become ripples as they fall through the surface. `EnZo_SpawnSplash` takes the
//! first slot that isn't a splash, so 20 splashes take all 15 slots (`@bug (game)`: the ripples and
//! bubbles there are dropped, and splash after splash takes the last slot that isn't one).
//! Drawn translucent (`Gfx_SetupDL_25Xlu`): ripples flat (`gZoraRipplesMaterialDL`, env 155 grey,
//! prim white at the ripple's alpha), bubbles and splashes facing the camera (env 150 and 200
//! grey; prim white, and 180 grey at the splash's alpha), from bakes with their colours on a
//! segment called between material and model.
//!
//! Drawn as `EnZo_Draw` does: the effects, then while seen the skeleton (`gZoraSkel`) with the eyes
//! on segment 8 (`EnZo_Blink`: open, half, closed every 30 to 60 frames), opaque at alpha 255
//! (`func_80034BA0`) else translucent (`func_80034CC4`), from a bake per eye and pass
//! (docs/adr/0012-actor-bakes.md); `EnZo_OverrideLimbDraw`'s head (limb 15, about 1800 along it)
//! and torso (limb 8) turns and the sway of limbs 8, 9 and 12; the focus 600 above the head
//! (`EnZo_PostLimbDraw`). The shadow is the shape's (`ActorShape_Init`, set to a circle as he
//! stands).
//!
//! The whole overlay is ported.
//!
//! Params: `type` bits 0 to 5 (his texts: 0 to 5 Zora's Domain's six, 6 and 7 Lake Hylia's, 8
//! Zora's Fountain's, killed for an adult; any other says 0x4006); bits 6 to 15 unused (set in the
//! data, 0xFFC0).
//! Flags: reads and sets EVENTCHKINF_30 (a Zora's talk closed: their second texts); reads
//! EVENTCHKINF_31 (Ruto's letter found: type 2), EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA (type 4);
//! reads and sets INFTABLE_124 (type 1's 0x4008 read), INFTABLE_129 (type 2's 0x402F read).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{approach_f, binang_to_rad, cos_s, sin_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, Actor, BGCHECKFLAG_GROUND, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind, TextLine};
use oot_game::collision_check::*;
use oot_game::item::QUEST_ZORA_SAPPHIRE;
use oot_game::message::*;
use oot_game::npc::{NpcTrack, actor_update_fidget_tables, talk_update, track_point};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, SEGMENT_CALL, keys};
use oot_game::play::{DrawOut, PlayState, Rand, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;
use oot_game::{event_chk_inf, inf_table};

use crate::player::Player;

/// `ACTOR_EN_ZO` (`actor_table.h`: 0x01CE).
pub const ACTOR_EN_ZO: i16 = 0x01CE;
/// `OBJECT_ZO` (`object_table.h`: 0x00FE).
pub const OBJECT: &str = "object_zo";

/// `En_Zo_Profile`: `ACTORCAT_NPC`, `FLAGS` (`ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY`).
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_ZO, name: "En_Zo", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 6)`: which Zora (his texts).
pub const TYPE: Field = Field::params(
    "type",
    0,
    6,
    Kind::Named(&[
        (0, "Zora's Domain: by the shop"),
        (1, "Zora's Domain: the lower pool"),
        (2, "Zora's Domain: Ruto's letter"),
        (3, "Zora's Domain: the question"),
        (4, "Zora's Domain: King Zora's worry"),
        (5, "Zora's Domain: the deep pool"),
        (6, "Lake Hylia (0x4020)"),
        (7, "Lake Hylia (0x4021)"),
        (8, "Zora's Fountain (standing; killed for an adult)"),
    ]),
    "which Zora: his texts (any other value says 0x4006)",
);
/// Bits 6 to 15: read by nothing.
pub const UNUSED: Field = Field::params("unused", 6, 10, Kind::Unused, "read by nothing (all set in the data: 0xFFC0)");

// `EVENTCHKINF_*`, `INFTABLE_*` (`save.h`).
/// `EVENTCHKINF_30` (`save.h`: 0x30): a Zora's talk closed.
pub const EVENTCHKINF_30: u16 = 0x30;
/// `EVENTCHKINF_31` (`save.h`: 0x31): Ruto's letter picked up at Lake Hylia.
pub const EVENTCHKINF_31: u16 = 0x31;
/// `EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA` (`save.h`: 0x33).
pub const EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA: u16 = 0x33;
/// `INFTABLE_124` (`save.h`: 0x124): type 1's 0x4008 read.
pub const INFTABLE_124: u16 = 0x124;
/// `INFTABLE_129` (`save.h`: 0x129): type 2's 0x402F read.
pub const INFTABLE_129: u16 = 0x129;

/// En_Zo's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, UNUSED],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_30)).reads().sets().note("any Zora's talk closed (but 0x4020's and 0x4021's): their second texts"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_31)).reads().note("Ruto's letter found: type 2's texts"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA)).reads().note("King Zora moved aside: type 4's 0x4010"),
        FlagUse::new(inf_table!(INFTABLE_124)).reads().sets().note("type 1's 0x4008 read: 0x4009 next"),
        FlagUse::new(inf_table!(INFTABLE_129)).reads().sets().note("type 2's 0x402F read: 0x400B next"),
    ],
    todo: false,
    objects: &[],
    // `EnZo_GetTextId` (a mask's reaction first, then by type), then 0x400C's choice.
    texts: &[
        TextLine::new(0x7188).cond("Link wearing the Keaton Mask, the Skull Mask or the Bunny Hood").note("a mask"),
        TextLine::new(0x7189).cond("Link wearing the Spooky Mask, the Goron Mask or the Mask of Truth").note("a mask"),
        TextLine::new(0x718B).cond("Link wearing the Zora Mask").note("a Zora's face"),
        TextLine::new(0x718A).cond("Link wearing the Gerudo Mask").note("a Gerudo's face"),
        TextLine::new(0x402A).when("type", &[8]).cond("after a Zora's talk (EVENTCHKINF_30)").note("the fountain's Zora"),
        TextLine::new(0x4020).when("type", &[6]).note("the lake's Zora"),
        TextLine::new(0x4021).when("type", &[7]).note("the lake's Zora"),
        TextLine::new(0x402D).when("type", &[0, 2, 4]).cond("with the Zora's Sapphire").note("thanks for saving Ruto"),
        TextLine::new(0x4007).when("type", &[0]).cond("after a Zora's talk").note("the shop"),
        TextLine::new(0x402E).when("type", &[1, 3, 5]).cond("with the Zora's Sapphire").note("thanks for saving Ruto"),
        TextLine::new(0x4009).when("type", &[1]).cond("after 0x4008").note("the diving game"),
        TextLine::new(0x4008).when("type", &[1]).cond("after a Zora's talk").note("the diving game"),
        TextLine::new(0x400B).when("type", &[2]).cond("after Ruto's letter is found and 0x402F").note("Ruto's letter"),
        TextLine::new(0x402F).when("type", &[2]).cond("after Ruto's letter is found").note("Ruto's letter"),
        TextLine::new(0x400A).when("type", &[2]).cond("after a Zora's talk").note("Princess Ruto"),
        TextLine::new(0x400C).when("type", &[3]).cond("after a Zora's talk").note("a question: a choice"),
        TextLine::new(0x4010).when("type", &[4]).cond("after King Zora has the letter").note("the king moved"),
        TextLine::new(0x400F).when("type", &[4]).cond("after a Zora's talk").note("the king's worry"),
        TextLine::new(0x4011).when("type", &[5]).cond("after a Zora's talk").note("the deep pool"),
        TextLine::new(0x4006).cond("the first talk, and any type without a text of its own").note("welcome to Zora's Domain"),
        TextLine::new(0x400D).when("type", &[3]).cond("0x400C's first answer").note("the answer"),
        TextLine::new(0x400E).when("type", &[3]).cond("0x400C's second answer").note("the answer"),
    ],
};

/// `NA_SE_EV_OUT_OF_WATER` (`environmentbank_table.h`: 0x2808), `NA_SE_EV_DIVE_WATER` (0x2809).
pub const NA_SE_EV_OUT_OF_WATER: u16 = 0x2808;
pub const NA_SE_EV_DIVE_WATER: u16 = 0x2809;
/// `ATTENTION_RANGE_6` (`actor.h`).
const ATTENTION_RANGE_6: u8 = 6;
/// `FIDGET_AMPLITUDE` (`actor.h`: 200.0f).
const FIDGET_AMPLITUDE: f32 = 200.0;

/// `NpcTalkState` (`actor.h`).
pub const NPC_TALK_STATE_IDLE: i16 = 0;
pub const NPC_TALK_STATE_TALKING: i16 = 1;
pub const NPC_TALK_STATE_ACTION: i16 = 2;
/// `NpcTrackingMode` (`actor.h`).
pub const NPC_TRACKING_NONE: i16 = 1;
pub const NPC_TRACKING_HEAD_AND_TORSO: i16 = 2;
pub const NPC_TRACKING_FULL_BODY: i16 = 4;

/// `MASK_REACTION_SET_ZORA` (`face_reaction.h`: 0x1D).
const MASK_REACTION_SET_ZORA: usize = 0x1D;
/// `sMaskReactionSetTextIds[MASK_REACTION_SET_ZORA]` (`z_face_reaction.c`), by `PLAYER_MASK_*`.
const MASK_REACTION_ZORA: [u16; 9] = [0x0000, 0x7188, 0x7188, 0x7189, 0x7188, 0x7189, 0x718B, 0x718A, 0x7189];

/// `EnZoEffectType`.
pub const ENZO_EFFECT_NONE: u8 = 0;
pub const ENZO_EFFECT_RIPPLE: u8 = 1;
pub const ENZO_EFFECT_SPLASH: u8 = 2;
pub const ENZO_EFFECT_BUBBLE: u8 = 3;
/// `EN_ZO_EFFECT_COUNT` (`z_en_zo.h`: 15).
pub const EN_ZO_EFFECT_COUNT: usize = 15;

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
    dim: Cylinder16 { radius: 26, height: 64, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInit`: immovable.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `EnZoAnimation`: indices into `sAnimationInfo`.
pub const ENZO_ANIM_0: usize = 0;
pub const ENZO_ANIM_1: usize = 1;
pub const ENZO_ANIM_2: usize = 2;
pub const ENZO_ANIM_3: usize = 3;
pub const ENZO_ANIM_4: usize = 4;
pub const ENZO_ANIM_5: usize = 5;
pub const ENZO_ANIM_6: usize = 6;
pub const ENZO_ANIM_7: usize = 7;

/// `gZoraHandsOnHipsTappingFootAnim`, `gZoraOpenArmsAnim`: what `EnZo_SetAnimation` compares
/// `skelAnime.animation` with.
pub const TAPPING_FOOT_ANIM: &str = "gZoraHandsOnHipsTappingFootAnim";
pub const OPEN_ARMS_ANIM: &str = "gZoraOpenArmsAnim";

/// `sAnimationInfo`: animation, speed, start, end (-1: its last frame), mode, morph frames.
const ANIMATION_INFO: [(&str, f32, f32, f32, u8, f32); 8] = [
    ("gZoraIdleAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gZoraIdleAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gZoraSurfaceAnim", 0.0, 1.0, 1.0, ANIMMODE_ONCE, 0.0),
    ("gZoraSurfaceAnim", 1.0, 1.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gZoraSurfaceAnim", 1.0, 8.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gZoraThrowRupeesAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    (TAPPING_FOOT_ANIM, 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    (OPEN_ARMS_ANIM, 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
];

/// `EnZo_Draw`'s `eyeTextures`, on segment 8.
const EYE_TEXTURES: [&str; 3] = ["gZoraEyeOpenTex", "gZoraEyeHalfTex", "gZoraEyeClosedTex"];
const SEG_EYES: u8 = 0x08;
/// `gSPSegment(0x0C, ...)`: `func_80034B28`'s empty list (opaque) or `func_80034B54`'s render
/// mode (translucent).
const SEG_RENDER_MODE: u8 = 0x0C;
/// The `gDPSetEnvColor(0, 0, 0, alpha)` of `func_80034BA0` / `func_80034CC4`, as a dynamic
/// colour run before the skeleton.
const SEG_ALPHA: u8 = 0x0E;
/// `func_80034B54`'s `gDPSetRenderMode(G_RM_FOG_SHADE_A, AA_EN | Z_CMP | Z_UPD | IM_RD |
/// CLR_ON_CVG | CVG_DST_WRAP | ZMODE_XLU | FORCE_BL | GBL_c2(G_BL_CLR_IN, G_BL_A_IN,
/// G_BL_CLR_MEM, G_BL_1MA))`.
const XLU_RENDER_MODE: (u32, u32) = (0xE200_001C, 0xC810_49F8);
/// The effects' colours (`gDPSetEnvColor` and `gDPSetPrimColor` between material and model).
const SEG_EFFECT_COLOR: u8 = 0x0E;

/// `ZoraLimb` (1-based, as `limbIndex`): the torso, the left arm, the right arm, the head.
const ZORA_LIMB_TORSO: usize = 8;
const ZORA_LIMB_LEFT_ARM: usize = 9;
const ZORA_LIMB_RIGHT_ARM: usize = 12;
const ZORA_LIMB_HEAD: usize = 15;
/// `ZORA_LIMB_MAX` (`jointTable[20]`).
const ZORA_LIMB_MAX: usize = 20;
/// `EnZo_OverrideLimbDraw`'s swaying limbs.
const SWAY_LIMBS: [usize; 3] = [ZORA_LIMB_TORSO, ZORA_LIMB_LEFT_ARM, ZORA_LIMB_RIGHT_ARM];

/// The bake of an eye (0..2) in a pass.
pub fn bake_name(eye: usize, xlu: bool) -> String {
    format!("En_Zo/{}_eye{eye}", if xlu { "xlu" } else { "opa" })
}
/// The effects' bakes.
pub const BAKE_RIPPLE: &str = "En_Zo/ripple";
pub const BAKE_BUBBLE: &str = "En_Zo/bubble";
pub const BAKE_SPLASH: &str = "En_Zo/splash";

/// The meshes `EnZo_Draw` draws: each eye opaque and translucent; the ripple, the bubble and the
/// splash (`Gfx_SetupDL_25Xlu`, the material, the colours, the model).
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for (eye, tex) in EYE_TEXTURES.iter().enumerate() {
        for xlu in [false, true] {
            v.push(MeshBake {
                name: bake_name(eye, xlu),
                object: OBJECT.into(),
                segments: vec![
                    (SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() }),
                    (SEG_RENDER_MODE, BakeSegment::Commands(if xlu { vec![XLU_RENDER_MODE] } else { Vec::new() })),
                    (SEG_ALPHA, BakeSegment::DynamicColor { env: true, prim: false }),
                ],
                prelude: vec![SEG_ALPHA],
                body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gZoraSkel".into(), limbs: Vec::new() },
            });
        }
    }
    for (name, material, model) in [
        (BAKE_RIPPLE, "gZoraRipplesMaterialDL", "gZoraRipplesModelDL"),
        (BAKE_BUBBLE, "gZoraBubblesMaterialDL", "gZoraBubblesModelDL"),
        (BAKE_SPLASH, "gZoraSplashesMaterialDL", "gZoraSplashesModelDL"),
    ] {
        v.push(MeshBake {
            name: name.into(),
            object: OBJECT.into(),
            segments: vec![(SEG_EFFECT_COLOR, BakeSegment::DynamicColor { env: true, prim: true })],
            prelude: Vec::new(),
            body: BakeBody::DLists(vec![(OBJECT.into(), material.into()), (SEGMENT_CALL.into(), format!("{SEG_EFFECT_COLOR:x}")), (OBJECT.into(), model.into())]),
        });
    }
    v
}

/// `EnZoEffect` (`z_en_zo.h`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EnZoEffect {
    /// `type` (`EnZoEffectType`).
    pub type_: u8,
    pub scale: f32,
    pub target_scale: f32,
    /// `color.a` (the only component used).
    pub alpha: u8,
    pub pos: Vec3,
    pub vel: Vec3,
    /// `vec`: a bubble's start, a splash's acceleration.
    pub vec: Vec3,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnZo_Standing`.
    Standing,
    /// `EnZo_Submerged`: waiting for Link near.
    Submerged,
    /// `EnZo_Surface`: rising.
    Surface,
    /// `EnZo_TreadWater`.
    TreadWater,
    /// `EnZo_Dive`.
    Dive,
}

pub struct EnZo {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    /// `interactInfo`: the talk state and the head and torso tracking.
    pub interact_info: NpcTrack,
    pub collider: ColliderCylinder,
    /// `canSpeak`.
    pub can_speak: bool,
    pub effects: [EnZoEffect; EN_ZO_EFFECT_COUNT],
    /// `dialogRadius`: his radius and 30.
    pub dialog_radius: f32,
    pub alpha: f32,
    pub tracking_mode: i16,
    pub ripple_timer: i16,
    /// `timeToDive`: frames left with Link away before he dives.
    pub time_to_dive: i16,
    pub blink_timer: i16,
    /// `eyeTexture`: 0 open, 1 half, 2 closed.
    pub eye_texture: i16,
    pub fidget_table_y: [i16; ZORA_LIMB_MAX],
    pub fidget_table_z: [i16; ZORA_LIMB_MAX],
    animations: Vec<AnimationInfo>,
}

/// `DECR`: 0 when it was 0, else the decremented value.
fn decr(x: &mut i16) -> i16 {
    if *x == 0 {
        0
    } else {
        *x -= 1;
        *x
    }
}

fn player(play: &PlayState) -> Option<&Player> {
    play.player.and_then(|h| play.actors.downcast::<Player>(h))
}

// shared: MaskReaction_GetTextId
/// `MaskReaction_GetTextId(play, MASK_REACTION_SET_ZORA)` (`z_face_reaction.c`):
/// `sMaskReactionSetTextIds[set][Player_GetMask]`, the Zoras' row.
fn mask_reaction_get_text_id(play: &PlayState, set: usize) -> u16 {
    let mask = player(play).map_or(0, |p| p.current_mask) as usize;
    match set {
        MASK_REACTION_SET_ZORA => MASK_REACTION_ZORA.get(mask).copied().unwrap_or(0),
        _ => 0,
    }
}

/// `EnZo_GetTextId`: a mask's reaction, else by type and the story; 0x4006 by default.
pub fn get_text_id(play: &PlayState, actor: &Actor) -> u16 {
    let text_id = mask_reaction_get_text_id(play, MASK_REACTION_SET_ZORA);
    if text_id != 0 {
        return text_id;
    }
    let s = &play.save;
    let sapphire = s.check_quest_item(QUEST_ZORA_SAPPHIRE);
    let talked = s.get_event_chk_inf(EVENTCHKINF_30);
    match TYPE.get(actor.params) {
        8 => {
            if talked {
                return 0x402A;
            }
        }
        6 => return 0x4020,
        7 => return 0x4021,
        0 => {
            if sapphire {
                return 0x402D;
            }
            if talked {
                return 0x4007;
            }
        }
        1 => {
            if sapphire {
                return 0x402E;
            }
            if talked {
                return if s.get_inf_table(INFTABLE_124) { 0x4009 } else { 0x4008 };
            }
        }
        2 => {
            if sapphire {
                return 0x402D;
            }
            if s.get_event_chk_inf(EVENTCHKINF_31) {
                return if s.get_inf_table(INFTABLE_129) { 0x400B } else { 0x402F };
            }
            if talked {
                return 0x400A;
            }
        }
        3 => {
            if sapphire {
                return 0x402E;
            }
            if talked {
                return 0x400C;
            }
        }
        4 => {
            if sapphire {
                return 0x402D;
            }
            if s.get_event_chk_inf(EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA) {
                return 0x4010;
            }
            if talked {
                return 0x400F;
            }
        }
        5 => {
            if sapphire {
                return 0x402E;
            }
            if talked {
                return 0x4011;
            }
        }
        _ => {}
    }
    0x4006
}

/// `EnZo_UpdateTalkState`: talking until the box closes; closing, 0x4020 and 0x4021 just end,
/// 0x4008 sets `INFTABLE_124`, 0x402F `INFTABLE_129`, and all the others `EVENTCHKINF_30`;
/// 0x400C's choice continues with 0x400D or 0x400E; an event text advanced is his action.
pub fn update_talk_state(play: &mut PlayState, actor: &mut Actor) -> i16 {
    match play.message_state() {
        TEXT_STATE_CLOSING => {
            match actor.text_id {
                0x4020 | 0x4021 => return NPC_TALK_STATE_IDLE,
                0x4008 => play.save.set_inf_table(INFTABLE_124),
                0x402F => play.save.set_inf_table(INFTABLE_129),
                _ => {}
            }
            play.save.set_event_chk_inf(EVENTCHKINF_30);
            NPC_TALK_STATE_IDLE
        }
        TEXT_STATE_CHOICE => {
            if play.message_should_advance() && actor.text_id == 0x400C {
                actor.text_id = if play.msg_ctx.choice_index == 0 { 0x400D } else { 0x400E };
                play.continue_textbox(actor.text_id);
            }
            NPC_TALK_STATE_TALKING
        }
        TEXT_STATE_EVENT => {
            if play.message_should_advance() {
                NPC_TALK_STATE_ACTION
            } else {
                NPC_TALK_STATE_TALKING
            }
        }
        // TEXT_STATE_NONE, _DONE_HAS_NEXT, _DONE_FADING, _DONE, _SONG_DEMO_DONE, _8, _9.
        _ => NPC_TALK_STATE_TALKING,
    }
}

impl EnZo {
    /// `EnZo_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = assets.skeleton(OBJECT, "gZoraSkel").and_then(|s| {
            let animations = ANIMATION_INFO
                .iter()
                .map(|&(name, play_speed, start_frame, frame_count, mode, morph_frames)| assets.animation(OBJECT, name).map(|animation| AnimationInfo { animation, play_speed, start_frame, frame_count, mode, morph_frames }))
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok((s, animations))
        });
        let (skeleton, animations) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Zo: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnZo {
            actor,
            // SkelAnime_InitFlex(.., &gZoraSkel, NULL, jointTable, morphTable, 20).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            action: Action::Submerged,
            interact_info: NpcTrack::default(),
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            can_speak: false,
            effects: [EnZoEffect::default(); EN_ZO_EFFECT_COUNT],
            dialog_radius: 0.0,
            alpha: 0.0,
            tracking_mode: 0,
            ripple_timer: 0,
            time_to_dive: 0,
            blink_timer: 0,
            eye_texture: 0,
            fidget_table_y: [0; ZORA_LIMB_MAX],
            fidget_table_z: [0; ZORA_LIMB_MAX],
            animations,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::None, 0.0);
        // Collider_InitCylinder, Collider_SetCylinder: above.
        this.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);

        if play.save.adult && this.ty() == 8 {
            this.actor.kill();
            return Box::new(this);
        }

        this.change_anim(ENZO_ANIM_2);
        this.actor.scale = Vec3::splat(0.01);
        this.actor.target_mode = ATTENTION_RANGE_6;
        this.dialog_radius = this.collider.dim.radius as f32 + 30.0;
        this.tracking_mode = NPC_TRACKING_NONE;
        this.can_speak = false;
        this.interact_info.talk_state = NPC_TALK_STATE_IDLE;
        let (height, radius) = (this.collider.dim.height as f32, this.collider.dim.radius as f32);
        this.actor.update_bg_check_info(&play.col, height * 0.5, radius, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);

        if this.actor.y_dist_to_water < 54.0 || this.ty() == 8 {
            this.actor.shape_shadow_draw = oot_game::shadow::ShadowDraw::Circle;
            this.actor.shape_shadow_scale = 24.0;
            this.change_anim(ENZO_ANIM_1);
            this.can_speak = true;
            this.alpha = 255.0;
            this.action = Action::Standing;
        } else {
            this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            this.action = Action::Submerged;
        }
        Box::new(this)
    }

    /// `PARAMS_GET_U(params, 0, 6)`.
    fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    /// `Animation_ChangeByInfo(&skelAnime, sAnimationInfo, index)`.
    fn change_anim(&mut self, index: usize) {
        if let Some(info) = self.animations.get(index) {
            self.skel.change_by_info(info);
        }
    }

    /// The water's surface above him: `world.pos.y + depthInWater`.
    fn water_surface(&self) -> f32 {
        self.actor.world_pos.y + self.actor.y_dist_to_water
    }

    /// `EnZo_SpawnRipple`: in the first free slot.
    pub fn spawn_ripple(&mut self, pos: Vec3, scale: f32, target_scale: f32, alpha: u8) {
        if let Some(e) = self.effects.iter_mut().find(|e| e.type_ == ENZO_EFFECT_NONE) {
            e.type_ = ENZO_EFFECT_RIPPLE;
            e.pos = pos;
            e.scale = scale;
            e.target_scale = target_scale;
            e.alpha = alpha;
        }
    }

    /// `EnZo_SpawnBubble`: in the first free slot, if below the surface: rising at 1, its scale
    /// 0.11 to 0.13 (a `Rand_ZeroOne` only when it's spawned).
    pub fn spawn_bubble(&mut self, rand: &mut Rand, pos: Vec3) {
        let water_surface = self.water_surface();
        for e in self.effects.iter_mut() {
            if e.type_ != ENZO_EFFECT_NONE {
                continue;
            }
            if !(water_surface <= pos.y) {
                e.type_ = ENZO_EFFECT_BUBBLE;
                e.pos = pos;
                e.vec = pos;
                e.vel = Vec3::new(0.0, 1.0, 0.0);
                e.scale = ((rand.zero_one() - 0.5) * 0.02) + 0.12;
                break;
            }
        }
    }

    /// `EnZo_SpawnSplash`: in the first slot that isn't a splash (whatever it holds), falling at
    /// 1 a frame², its alpha 100 to 200.
    pub fn spawn_splash(&mut self, rand: &mut Rand, pos: Vec3, vel: Vec3, scale: f32) {
        if let Some(e) = self.effects.iter_mut().find(|e| e.type_ != ENZO_EFFECT_SPLASH) {
            e.type_ = ENZO_EFFECT_SPLASH;
            e.pos = pos;
            e.vec = Vec3::new(0.0, -1.0, 0.0);
            e.vel = vel;
            e.alpha = ((rand.zero_one() * 100.0) + 100.0) as u8;
            e.scale = scale;
        }
    }

    /// `EnZo_UpdateEffectsRipples`: growing to their size, fading by 20; gone at 0.
    pub fn update_effects_ripples(&mut self) {
        for e in self.effects.iter_mut() {
            if e.type_ == ENZO_EFFECT_RIPPLE {
                approach_f(&mut e.scale, e.target_scale, 0.2, 0.8);
                if e.alpha > 20 {
                    e.alpha -= 20;
                } else {
                    e.alpha = 0;
                }
                if e.alpha == 0 {
                    e.type_ = ENZO_EFFECT_NONE;
                }
            }
        }
    }

    /// `EnZo_UpdateEffectsBubbles`: wobbling within a quarter of their start across, rising; at
    /// the surface a ripple.
    pub fn update_effects_bubbles(&mut self, rand: &mut Rand) {
        for i in 0..EN_ZO_EFFECT_COUNT {
            if self.effects[i].type_ == ENZO_EFFECT_BUBBLE {
                let e = &mut self.effects[i];
                e.pos.x = ((rand.zero_one() * 0.5) - 0.25) + e.vec.x;
                e.pos.z = ((rand.zero_one() * 0.5) - 0.25) + e.vec.z;
                e.pos.y += e.vel.y;
                // Bubbles turn into ripples when they reach the surface.
                let water_surface = self.water_surface();
                let e = &mut self.effects[i];
                if water_surface <= e.pos.y {
                    e.type_ = ENZO_EFFECT_NONE;
                    e.pos.y = water_surface;
                    let pos = e.pos;
                    self.spawn_ripple(pos, 0.06, 0.12, 200);
                }
            }
        }
    }

    /// `EnZo_UpdateEffectsSplashes`: flying, falling (to -20); through the surface a ripple.
    pub fn update_effects_splashes(&mut self) {
        for i in 0..EN_ZO_EFFECT_COUNT {
            if self.effects[i].type_ == ENZO_EFFECT_SPLASH {
                let e = &mut self.effects[i];
                e.pos += e.vel;
                if e.vel.y >= -20.0 {
                    e.vel.y += e.vec.y;
                } else {
                    e.vel.y = -20.0;
                    e.vec.y = 0.0;
                }
                // Splash particles turn into ripples when they hit the surface.
                let water_surface = self.water_surface();
                let e = &mut self.effects[i];
                if e.pos.y < water_surface {
                    e.type_ = ENZO_EFFECT_NONE;
                    e.pos.y = water_surface;
                    let pos = e.pos;
                    self.spawn_ripple(pos, 0.06, 0.12, 200);
                }
            }
        }
    }

    /// `EnZo_TreadWaterRipples`: a ripple at the surface above him.
    fn tread_water_ripples(&mut self, scale: f32, target_scale: f32, alpha: u8) {
        let pos = Vec3::new(self.actor.world_pos.x, self.water_surface(), self.actor.world_pos.z);
        self.spawn_ripple(pos, scale, target_scale, alpha);
    }

    /// `EnZo_SpawnSplashes`: 20 splashes round him at the surface, out at 0.5 to 2 and up at 3 to
    /// 6.
    pub fn spawn_splashes(&mut self, rand: &mut Rand) {
        for _ in 0..20 {
            let speed = rand.zero_one() * 1.5 + 0.5;
            let angle = rand.zero_one() * 6.28;
            let y = rand.zero_one() * 3.0 + 3.0;
            let vel = Vec3::new(angle.sin() * speed, y, angle.cos() * speed);
            let mut pos = self.actor.world_pos;
            pos.x += vel.x * 6.0;
            pos.z += vel.z * 6.0;
            pos.y += self.actor.y_dist_to_water;
            self.spawn_splash(rand, pos, vel, 0.08);
        }
    }

    /// `EnZo_Blink`.
    fn blink(&mut self, play: &mut PlayState) {
        if decr(&mut self.blink_timer) == 0 {
            self.eye_texture += 1;
            if self.eye_texture >= 3 {
                self.blink_timer = play.rand.s16_offset(30, 30);
                self.eye_texture = 0;
            }
        }
    }

    /// `EnZo_Dialog`: the head and torso on Link (preset 11): from 10 up for a child, -10 for an
    /// adult, standing; at his own height in water. Talking when he can.
    fn dialog(&mut self, play: &mut PlayState) {
        self.interact_info.target = player(play).map_or(self.actor.world_pos, |p| p.actor.world_pos);
        if self.action == Action::Standing {
            // Look down at link if child, look up if adult.
            self.interact_info.eye_height = if !play.save.adult { 10.0 } else { -10.0 };
        } else {
            self.interact_info.target.y = self.actor.world_pos.y;
        }
        let mut t = self.interact_info;
        track_point(play, &mut self.actor, &mut t, 11, self.tracking_mode);
        self.interact_info = t;
        if self.can_speak {
            let mut talk_state = self.interact_info.talk_state;
            talk_update(play, &mut self.actor, &mut talk_state, self.dialog_radius, get_text_id, update_talk_state);
            self.interact_info.talk_state = talk_state;
        }
    }

    /// `EnZo_PlayerInProximity`: Link within 240 across and 80 up or down of the surface above him.
    pub fn player_in_proximity(&self, play: &PlayState) -> bool {
        let player_pos = player(play).map_or(self.actor.world_pos, |p| p.actor.world_pos);
        let surface_pos = Vec3::new(self.actor.world_pos.x, self.water_surface(), self.actor.world_pos.z);
        let d = surface_pos - player_pos;
        let h_dist = (d.x * d.x + d.z * d.z).sqrt();
        let y_dist = (player_pos.y - surface_pos.y).abs();
        h_dist < 240.0 && y_dist < 80.0
    }

    /// `EnZo_SetAnimation`: done talking, the foot's tap or the open arms back to standing (or
    /// treading, held at its end); talking 0x4006, the tap; 0x4007, the open arms.
    fn set_animation(&mut self) {
        let mut anim_id = ANIMATION_INFO.len();
        if (self.skel.is(TAPPING_FOOT_ANIM) || self.skel.is(OPEN_ARMS_ANIM)) && self.interact_info.talk_state == NPC_TALK_STATE_IDLE {
            anim_id = if self.action == Action::Standing { ENZO_ANIM_0 } else { ENZO_ANIM_3 };
        }
        if self.interact_info.talk_state != NPC_TALK_STATE_IDLE && self.actor.text_id == 0x4006 && !self.skel.is(TAPPING_FOOT_ANIM) {
            anim_id = ENZO_ANIM_6;
        }
        if self.interact_info.talk_state != NPC_TALK_STATE_IDLE && self.actor.text_id == 0x4007 && !self.skel.is(OPEN_ARMS_ANIM) {
            anim_id = ENZO_ANIM_7;
        }
        if anim_id != ANIMATION_INFO.len() {
            self.change_anim(anim_id);
            if anim_id == ENZO_ANIM_3 {
                self.skel.cur_frame = self.skel.end_frame;
                self.skel.play_speed = 0.0;
            }
        }
    }

    /// `EnZo_Standing`: the sway; talking, the whole body on Link; else head and torso while Link
    /// is in front and near.
    fn standing(&mut self, play: &mut PlayState) {
        actor_update_fidget_tables(play, &mut self.fidget_table_y, &mut self.fidget_table_z, ZORA_LIMB_MAX);
        self.set_animation();
        if self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            self.tracking_mode = NPC_TRACKING_FULL_BODY;
            return;
        }
        // ABS((s16)((f32)yawTowardsPlayer - (f32)shape.rot.y)).
        let angle = ((self.actor.yaw_towards_player as f32 - self.actor.shape_rot.y as f32) as i32 as i16 as i32).abs();
        self.tracking_mode = if angle < 0x4718 && self.player_in_proximity(play) { NPC_TRACKING_HEAD_AND_TORSO } else { NPC_TRACKING_NONE };
    }

    /// `EnZo_Submerged`: Link near: up at 4.
    fn submerged(&mut self, play: &mut PlayState) {
        if self.player_in_proximity(play) {
            self.action = Action::Surface;
            self.actor.velocity.y = 4.0;
        }
    }

    /// `EnZo_Surface`: less than 54 deep, out of the water (the sound, the splashes, treading);
    /// less than 80, easing to 2 and fading in.
    fn surface(&mut self, play: &mut PlayState) {
        if self.actor.y_dist_to_water < 54.0 {
            audio_play_actor_sfx2(play, NA_SE_EV_OUT_OF_WATER);
            self.spawn_splashes(&mut play.rand);
            self.change_anim(ENZO_ANIM_3);
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            self.action = Action::TreadWater;
            self.actor.velocity.y = 0.0;
            self.alpha = 255.0;
        } else if self.actor.y_dist_to_water < 80.0 {
            approach_f(&mut self.actor.velocity.y, 2.0, 0.4, 0.6);
            approach_f(&mut self.alpha, 255.0, 0.3, 10.0);
        }
    }

    /// `EnZo_TreadWater`: the sway; the surfacing played, talking and tracking; bobbing about 54
    /// deep; ripples; Link near, the dive put off 40 to 80; else counted down: the dive
    /// (`gZoraSurfaceAnim` backwards from its end to 8).
    fn tread_water(&mut self, play: &mut PlayState) {
        actor_update_fidget_tables(play, &mut self.fidget_table_y, &mut self.fidget_table_z, ZORA_LIMB_MAX);
        if self.skel.on_frame(self.skel.end_frame) {
            self.can_speak = true;
            self.tracking_mode = NPC_TRACKING_FULL_BODY;
            self.skel.play_speed = 0.0;
        }
        self.set_animation();
        let target = if self.actor.y_dist_to_water < 54.0 { -0.6 } else { 0.6 };
        approach_f(&mut self.actor.velocity.y, target, 0.3, 0.2);
        if self.ripple_timer != 0 {
            self.ripple_timer -= 1;
            if self.ripple_timer == 3 || self.ripple_timer == 6 {
                self.tread_water_ripples(0.2, 1.0, 200);
            }
        } else {
            self.tread_water_ripples(0.2, 1.0, 200);
            self.ripple_timer = 12;
        }
        if self.player_in_proximity(play) {
            self.time_to_dive = play.rand.s16_offset(40, 40);
        } else if decr(&mut self.time_to_dive) == 0 {
            self.change_anim(ENZO_ANIM_4);
            self.can_speak = false;
            self.tracking_mode = NPC_TRACKING_NONE;
            self.action = Action::Dive;
            let start_frame = self.skel.start_frame;
            self.skel.start_frame = self.skel.end_frame;
            self.skel.cur_frame = self.skel.end_frame;
            self.skel.end_frame = start_frame;
            self.skel.play_speed = -1.0;
        }
    }

    /// `EnZo_Dive`: at frame 8, under (the sound, the splashes, down at 4); deeper than 80 or on
    /// the floor, easing to 1 down and fading out; faded, back home submerged.
    fn dive(&mut self, play: &mut PlayState) {
        if self.skel.on_frame(self.skel.end_frame) {
            audio_play_actor_sfx2(play, NA_SE_EV_DIVE_WATER);
            self.spawn_splashes(&mut play.rand);
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            self.actor.velocity.y = -4.0;
            self.skel.play_speed = 0.0;
        }
        if self.skel.play_speed > 0.0 {
            return;
        }
        if self.actor.y_dist_to_water > 80.0 || self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            approach_f(&mut self.actor.velocity.y, -1.0, 0.4, 0.6);
            approach_f(&mut self.alpha, 0.0, 0.3, 10.0);
        }
        if self.alpha as i16 == 0 {
            self.change_anim(ENZO_ANIM_2);
            self.actor.world_pos = self.actor.home_pos;
            self.alpha = 0.0;
            self.action = Action::Submerged;
        }
    }

    /// His skeleton posed as `EnZo_OverrideLimbDraw` turns it: the head (limb 15) about 1800
    /// along it (x by the head's yaw, z by its pitch), the torso (limb 8) by -yaw and pitch, the
    /// sway of limbs 8, 9 and 12.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 2], torso: [i16; 2], sway: &[(usize, i16, i16)]) -> Vec<Mat4> {
        // BINANG_TO_RAD_ALT.
        let r = binang_to_rad;
        skeleton.pose_override(joints, |limb, _pos, rot| {
            let mut pre = Mat4::IDENTITY;
            if limb == ZORA_LIMB_HEAD {
                // limbRot = headRot: RotateX(limbRot.y), RotateZ(limbRot.x) about (1800, 0, 0).
                pre = Mat4::from_translation(Vec3::new(1800.0, 0.0, 0.0)) * Mat4::from_rotation_x(r(head[1])) * Mat4::from_rotation_z(r(head[0])) * Mat4::from_translation(Vec3::new(-1800.0, 0.0, 0.0));
            }
            if limb == ZORA_LIMB_TORSO {
                // limbRot = torsoRot: RotateX(-limbRot.y), RotateZ(limbRot.x).
                pre = Mat4::from_rotation_x(r(torso[1].wrapping_neg())) * Mat4::from_rotation_z(r(torso[0]));
            }
            if let Some(&(_, a, b)) = sway.iter().find(|s| s.0 == limb) {
                rot[1] = (rot[1] as f32 + sin_s(a) * FIDGET_AMPLITUDE) as i32 as i16;
                rot[2] = (rot[2] as f32 + cos_s(b) * FIDGET_AMPLITUDE) as i32 as i16;
            }
            pre
        })
    }

    fn sway_of(y: &[i16; ZORA_LIMB_MAX], z: &[i16; ZORA_LIMB_MAX]) -> [(usize, i16, i16); 3] {
        SWAY_LIMBS.map(|l| (l, y[l], z[l]))
    }

    /// `EnZo_DrawEffectsRipples`, `_Bubbles`, `_Splashes`: each in its order, translucent.
    fn draw_effects(&self, view: &ViewInfo, out: &mut DrawOut) {
        let push = |out: &mut DrawOut, bake: &str, m: Mat4, env: [u8; 4], prim: [u8; 4]| {
            let mut sv = SegmentValues::default();
            sv.env[SEG_EFFECT_COLOR as usize] = Some(env);
            sv.prim[SEG_EFFECT_COLOR as usize] = Some(prim);
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        };
        for e in self.effects.iter().filter(|e| e.type_ == ENZO_EFFECT_RIPPLE) {
            let m = Mat4::from_translation(e.pos) * Mat4::from_scale(Vec3::new(e.scale, 1.0, e.scale));
            push(out, BAKE_RIPPLE, m, [155, 155, 155, 0], [255, 255, 255, e.alpha]);
        }
        for e in self.effects.iter().filter(|e| e.type_ == ENZO_EFFECT_BUBBLE) {
            let m = oot_game::draw::replace_rotation(Mat4::from_translation(e.pos), view.billboard) * Mat4::from_scale(Vec3::new(e.scale, e.scale, 1.0));
            push(out, BAKE_BUBBLE, m, [150, 150, 150, 0], [255, 255, 255, 255]);
        }
        for e in self.effects.iter().filter(|e| e.type_ == ENZO_EFFECT_SPLASH) {
            let m = oot_game::draw::replace_rotation(Mat4::from_translation(e.pos), view.billboard) * Mat4::from_scale(Vec3::new(e.scale, e.scale, 1.0));
            push(out, BAKE_SPLASH, m, [200, 200, 200, 0], [180, 180, 180, e.alpha]);
        }
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `angles`: head x, y; torso x, y; the sway (limbs 8, 9, 12: y, z each).
    pub const HEAD: usize = 0;
    pub const TORSO: usize = 2;
    pub const SWAY: usize = 4;
    pub const ANGLES: usize = 10;
    /// `values`: the alpha.
    pub const ALPHA: usize = 0;
    /// `switches`: the eye.
    pub const EYE: usize = 0;
}

impl ActorImpl for EnZo {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnZo_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.alpha as i32 != 0 {
            self.skel.update();
            self.blink(play);
        }
        self.actor.move_forward();
        let (height, radius) = (self.collider.dim.height as f32, self.collider.dim.radius as f32);
        self.actor.update_bg_check_info(&play.col, radius, height * 0.25, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        match self.action {
            Action::Standing => self.standing(play),
            Action::Submerged => self.submerged(play),
            Action::Surface => self.surface(play),
            Action::TreadWater => self.tread_water(play),
            Action::Dive => self.dive(play),
        }
        self.dialog(play);

        // Spawn air bubbles.
        if play.state_frames & 8 != 0 {
            let mut pos = self.actor.world_pos;
            pos.y += (play.rand.zero_one() - 0.5) * 10.0 + 18.0;
            pos.x += (play.rand.zero_one() - 0.5) * 28.0;
            pos.z += (play.rand.zero_one() - 0.5) * 28.0;
            self.spawn_bubble(&mut play.rand, pos);
        }

        if self.alpha as i32 != 0 {
            self.collider.update(&self.actor);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }

        self.update_effects_ripples();
        self.update_effects_bubbles(&mut play.rand);
        self.update_effects_splashes();
    }

    /// `EnZo_PostLimbDraw`: the focus 600 above the head (limb 15).
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.alpha as i32 == 0 {
            return;
        }
        let t = &self.interact_info;
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, [t.head[0], t.head[1]], [t.torso[0], t.torso[1]], &Self::sway_of(&self.fidget_table_y, &self.fidget_table_z));
        if let Some(head) = bones.get(ZORA_LIMB_HEAD - 1) {
            let m = actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(Vec3::new(0.0, 600.0, 0.0));
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let t = &self.interact_info;
        let sway = Self::sway_of(&self.fidget_table_y, &self.fidget_table_z);
        rs.angles = vec![t.head[0], t.head[1], t.torso[0], t.torso[1], sway[0].1, sway[0].2, sway[1].1, sway[1].2, sway[2].1, sway[2].2];
        rs.values = vec![self.alpha];
        rs.switches = vec![self.eye_texture.clamp(0, 2) as u32];
        rs
    }

    /// `EnZo_Draw`: the effects; seen, the skeleton opaque at 255, else translucent.
    fn draw(&self, rs: &RenderState, _play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        self.draw_effects(view, out);
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < rs::ANGLES || rs.values.is_empty() || rs.switches.is_empty() {
            return;
        }
        let alpha = rs.values[rs::ALPHA];
        if alpha as i32 == 0 {
            return;
        }
        let a = &rs.angles;
        let sway = [(SWAY_LIMBS[0], a[rs::SWAY], a[rs::SWAY + 1]), (SWAY_LIMBS[1], a[rs::SWAY + 2], a[rs::SWAY + 3]), (SWAY_LIMBS[2], a[rs::SWAY + 4], a[rs::SWAY + 5])];
        let bones = Self::pose(&self.skeleton, &joints.rot, [a[rs::HEAD], a[rs::HEAD + 1]], [a[rs::TORSO], a[rs::TORSO + 1]], &sway);
        let xlu = alpha != 255.0;
        let mut sv = SegmentValues::default();
        // gDPSetEnvColor's alpha: (u8)alpha.
        sv.env[SEG_ALPHA as usize] = Some([0, 0, 0, alpha as i32 as u8]);
        let key = MeshKey::named(keys::bake(&bake_name(rs.switches[rs::EYE] as usize, xlu)));
        let cmd = DrawCmd { mesh: key, transform: actor_draw_matrix(rs), bones, params: DrawParams { segments: Some(sv), ..Default::default() } };
        if xlu {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
    }

    /// `EnZo_Destroy`: nothing.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bakes_are_every_eye_in_both_passes_and_the_three_effects() {
        let b = bakes();
        assert_eq!(b.len(), 3 * 2 + 3);
        assert!(b.iter().any(|m| m.name == bake_name(2, true)));
        assert_eq!(MASK_REACTION_ZORA[6], 0x718B);
    }
}
