//! `En_Sa` (`ovl_En_Sa/z_en_sa.c`): Saria, in Kokiri Forest, her house and the Sacred Forest
//! Meadow.
//!
//! Her type comes from the scene, the save and the cutscene index (`EnSa_GetType`):
//! - **`SARIA_TYPE_GREETING`** (Kokiri Forest before `INFTABLE_SARIA_GREETED_LINK`, without the
//!   Kokiri Emerald, or any cutscene index but `CS_INDEX_D`): her init starts
//!   `gKokiriForestSariaGreetingCs` (`play->csCtx.script`, `gSaveContext.cutsceneTrigger = 1`). She
//!   follows the script's cues on channel 1 (`EnSa_HandleCutscene`): placed at the first cue's
//!   start, then each cue's animation (`sAnimationInfo1[cue->id]`: arms to her sides 4, running 3
//!   with her steps' sound every 8 frames, looking up 1, waving 2, her mouth open for 2 and 9), its
//!   rotation, and a velocity from its start to its end over its frames (with her gravity, -1).
//!   When the script ends (`EnSa_BecomeIdle`) she's back at home, facing as placed, and sets
//!   `INFTABLE_SARIA_GREETED_LINK`.
//! - **`SARIA_TYPE_STANDING`** (Kokiri Forest after the greeting; her house, `kokiri_home5`, for
//!   child Link with the Fairy Ocarina and without Zelda's Letter): she stands (`EnSa_Idle`), and in
//!   Kokiri Forest her arms follow her texts' boxes (`EnSa_Animate`'s groups: the right hand out,
//!   the hands to her face, behind her back and out again, on her hips and off, to her chest).
//! - **`SARIA_TYPE_MEADOW_WAITING`** (the Sacred Forest Meadow with Zelda's Letter, without Saria's
//!   Song, or any cutscene index but `CS_INDEX_D`): she plays the ocarina until Link is north of
//!   z -2220 outside a cutscene (`EnSa_AwaitTeachingSariasSong`), then starts
//!   `gMeadowSariasSongCs`, whose texts teach the song and hear it back. At its end she gives the
//!   song (`Item_Give(ITEM_SONG_SARIA)`) and plays on.
//! - **`SARIA_TYPE_MEADOW_PLAYING`** (the meadow with the song): she plays on; talking, she stops
//!   playing (`EnSa_TalkInForestMeadow`) and starts again after.
//! - **`SARIA_TYPE_NONE`**: killed at once. (`SARIA_TYPE_UNUSED`, never returned, waits for cues
//!   standing.)
//!
//! She spawns her fairy (`En_Elf`, `FAIRY_KOKIRI`) as her child, talks (`Npc_UpdateTalking`,
//! within her radius and 30; in the meadow only while facing Link within 0x1555): 0x10AD with
//! Saria's Song; with the Kokiri Emerald 0x1047, then 0x1048 (`INFTABLE_SARIA_SPOKE_IN_HER_HOUSE`);
//! after Mido's first talk (`EVENTCHKINF_MIDO_DENIED_DEKU_TREE_ACCESS`) 0x1031, then 0x1032
//! (`INFTABLE_SARIA_WAS_TOLD_ABOUT_MIDO`); after the greeting 0x1002, then 0x1003
//! (`INFTABLE_SARIA_NOTICED_FAIRY`); else 0x1001. Her head and torso follow Link
//! (`Npc_TrackPoint`, preset 2: the whole body in Kokiri Forest, the head in the meadow, none while
//! she plays or in the greeting), she blinks, and outside her cutscenes she fades out beyond 400
//! (`Actor_UpdateAlphaByDistance`).
//!
//! Drawn as `EnSa_Draw` does: her skeleton (`gSariaSkel`) opaque at full alpha (`func_80034BA0`)
//! and translucent while fading (`func_80034CC4`), her eyes (segments 8 and 9) and mouth (0xA)
//! swapped in per draw by their textures' segments (`DrawParams::texture_images`), her head and
//! torso turned (`EnSa_OverrideLimbDraw`), her right hand holding the ocarina in the meadow
//! (`gSariaRightHandAndOcarinaDL`), the focus 400 along her head (`EnSa_PostLimbDraw`); from four
//! bakes (pass and hand, docs/adr/0012-actor-bakes.md). The circle shadow (12, at her alpha) is
//! the shape's (`ActorShape_Init`'s, drawn by `Actor_Draw`).
//!
//! The whole overlay is ported. `MaskReaction_GetTextId(play, MASK_REACTION_SET_SARIA)` gives 0 (no
//! masks yet); `Actor_UpdateAlphaByDistance` is the module's own (its cutscene distance, a
//! quarter of the camera's, isn't in `oot_game::npc`'s).
//!
//! Params: none (her placements' are all 0; what she does is the scene's, the save's and the
//! cutscene index's).
//! Flags: reads and sets INFTABLE_SARIA_GREETED_LINK (the greeting over: set at its end),
//! INFTABLE_SARIA_NOTICED_FAIRY (0x1002 read), INFTABLE_SARIA_WAS_TOLD_ABOUT_MIDO (0x1031 read)
//! and INFTABLE_SARIA_SPOKE_IN_HER_HOUSE (0x1047 read); sets EVENTCHKINF_SARIA_WAS_TOLD_ABOUT_MIDO
//! (0x1031 read); reads EVENTCHKINF_MIDO_DENIED_DEKU_TREE_ACCESS (0x1031's) and
//! EVENTCHKINF_OBTAINED_ZELDAS_LETTER (her house, the meadow); reads `gSaveContext.save.cutsceneIndex`
//! (a cutscene layer: the greeting, the meadow's wait).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawImage, DrawParams, MeshKey, SegmentValues, SourceImage};
use eng_math::binang_to_rad;
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor, Rot, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Flag, FlagUse};
use oot_game::audio::sfx::{NA_SE_PL_WALK_GROUND, SfxPos};
use oot_game::collision_check::*;
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::item::{ITEM_OCARINA_FAIRY, QUEST_KOKIRI_EMERALD, QUEST_SONG_SARIA, item_give};
use oot_game::message::*;
use oot_game::npc::{NpcTrack, actor_update_alpha_by_distance, track_point};
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::save::EVENTCHKINF_OBTAINED_ZELDAS_LETTER;
use oot_game::skelanime_std::*;
use oot_game::{event_chk_inf, inf_table};

use crate::en_elf::{ACTOR_EN_ELF, FAIRY_KOKIRI};
use crate::en_md::EVENTCHKINF_MIDO_DENIED_DEKU_TREE_ACCESS;

/// `ACTOR_EN_SA` (`actor_table.h`: 0x0146).
pub const ACTOR_EN_SA: i16 = 0x0146;
/// `OBJECT_SA` (`object_table.h`).
pub const OBJECT: &str = "object_sa";

/// `En_Sa_Profile`: `ACTORCAT_NPC`, `FLAGS` (`ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY |
/// ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA`).
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_SA,
    name: "En_Sa",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: OBJECT,
    schema: &SCHEMA,
};

// `INFTABLE_*` (`save.h`).
/// The greeting's cutscene over (`EnSa_BecomeIdle`).
pub const INFTABLE_SARIA_GREETED_LINK: u16 = 0x00;
/// 0x1002 read: 0x1003 from then on.
pub const INFTABLE_SARIA_NOTICED_FAIRY: u16 = 0x01;
/// 0x1031 read: 0x1032 from then on.
pub const INFTABLE_SARIA_WAS_TOLD_ABOUT_MIDO: u16 = 0x03;
/// 0x1047 read: 0x1048 from then on.
pub const INFTABLE_SARIA_SPOKE_IN_HER_HOUSE: u16 = 0x05;
/// `EVENTCHKINF_SARIA_WAS_TOLD_ABOUT_MIDO` (`save.h`: 0x03): 0x1031 read.
pub const EVENTCHKINF_SARIA_WAS_TOLD_ABOUT_MIDO: u16 = 0x03;

/// En_Sa's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[],
    flags: &[
        FlagUse::new(inf_table!(INFTABLE_SARIA_GREETED_LINK)).reads().sets().note("Kokiri Forest: the greeting's cutscene over (set at its end): standing, 0x1002"),
        FlagUse::new(inf_table!(INFTABLE_SARIA_NOTICED_FAIRY)).reads().sets().note("0x1002 read: 0x1003 next; her hands behind her back"),
        FlagUse::new(inf_table!(INFTABLE_SARIA_WAS_TOLD_ABOUT_MIDO)).reads().sets().note("0x1031 read: 0x1032 next; her hands behind her back"),
        FlagUse::new(inf_table!(INFTABLE_SARIA_SPOKE_IN_HER_HOUSE)).reads().sets().note("0x1047 read: 0x1048 next"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_MIDO_DENIED_DEKU_TREE_ACCESS)).reads().note("Mido's first talk read: 0x1031"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_SARIA_WAS_TOLD_ABOUT_MIDO)).sets().note("0x1031 read"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_OBTAINED_ZELDAS_LETTER)).reads().note("her house: gone with it; the meadow: there with it"),
        FlagUse::new(Flag::Other("gSaveContext.save.cutsceneIndex")).reads().note("a cutscene layer (CS_INDEX_0 and up, not CS_INDEX_D): the greeting in Kokiri Forest, the wait in the meadow"),
    ],
    todo: false,
};

// `SCENE_*` (`scene_table.h`).
pub const SCENE_SARIAS_HOUSE: u16 = 0x29;
pub const SCENE_KOKIRI_FOREST: u16 = 0x55;
pub const SCENE_SACRED_FOREST_MEADOW: u16 = 0x56;

/// `CS_INDEX_0`, `CS_INDEX_D` (`save.h`).
const CS_INDEX_0: u16 = 0xFFF0;
const CS_INDEX_D: u16 = 0xFFFD;
/// `ITEM_SONG_SARIA` (`item.h`: 0x62).
pub const ITEM_SONG_SARIA: u8 = 0x62;
/// `SURFACE_SFX_OFFSET_DIRT` (`bgcheck.h`: 0).
const SURFACE_SFX_OFFSET_DIRT: u16 = 0;

/// `SariaType`.
pub const SARIA_TYPE_NONE: i32 = 0;
pub const SARIA_TYPE_STANDING: i32 = 1;
pub const SARIA_TYPE_MEADOW_PLAYING: i32 = 2;
pub const SARIA_TYPE_UNUSED: i32 = 3;
pub const SARIA_TYPE_GREETING: i32 = 4;
pub const SARIA_TYPE_MEADOW_WAITING: i32 = 5;

/// `SariaEyes`.
pub const SARIA_EYE_OPEN: i16 = 0;
pub const SARIA_EYE_HALF: i16 = 1;
pub const SARIA_EYE_CLOSED: i16 = 2;
pub const SARIA_EYE_SURPRISED: i16 = 3;
pub const SARIA_EYE_SAD: i16 = 4;
/// `SariaMouth`.
pub const SARIA_MOUTH_CLOSED2: i16 = 0;
pub const SARIA_MOUTH_SMILING_OPEN: i16 = 1;
pub const SARIA_MOUTH_FROWNING: i16 = 2;
pub const SARIA_MOUTH_SURPRISED: i16 = 3;
pub const SARIA_MOUTH_CLOSED: i16 = 4;

/// `NpcTalkState` (`actor.h`).
const NPC_TALK_STATE_IDLE: i16 = 0;
const NPC_TALK_STATE_TALKING: i16 = 1;
/// `NpcTrackingMode` (`actor.h`).
const NPC_TRACKING_PLAYER_AUTO_TURN: i16 = 0;
const NPC_TRACKING_NONE: i16 = 1;
const NPC_TRACKING_HEAD: i16 = 3;
const NPC_TRACKING_FULL_BODY: i16 = 4;

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
    dim: Cylinder16 { radius: 20, height: 46, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

// The animations the code compares `skelAnime.animation` with.
const ANIM_WAIT_ARMS_TO_SIDE: &str = "gSariaWaitArmsToSideAnim";
const ANIM_PLAYING_OCARINA: &str = "gSariaPlayingOcarinaAnim";
const ANIM_STOP_PLAYING_OCARINA: &str = "gSariaStopPlayingOcarinaAnim";
const ANIM_OCARINA_TO_MOUTH: &str = "gSariaOcarinaToMouthAnim";
const ANIM_HANDS_BEHIND_BACK_WAIT: &str = "gSariaHandsBehindBackWaitAnim";

/// `EnSaAnimation1`.
pub const ENSA_ANIM1_WAIT_ARMS_TO_SIDE: usize = 0;
pub const ENSA_ANIM1_LOOK_UP_ARMS_EXTENDED: usize = 1;
pub const ENSA_ANIM1_WAVE: usize = 2;
pub const ENSA_ANIM1_RUN: usize = 3;
pub const ENSA_ANIM1_MOVE_ARMS_TO_SIDE: usize = 4;
pub const ENSA_ANIM1_LOOK_OVER_SHOULDER: usize = 5;
pub const ENSA_ANIM1_PLAYING_OCARINA: usize = 6;
pub const ENSA_ANIM1_STOP_PLAYING_OCARINA: usize = 7;
pub const ENSA_ANIM1_OCARINA_TO_MOUTH: usize = 8;
pub const ENSA_ANIM1_LINK_LEARNED_SARIAS_SONG: usize = 9;
pub const ENSA_ANIM1_RETURN_TO_OCARINA: usize = 10;
pub const ENSA_ANIM1_START_PLAYING_OCARINA: usize = 11;

/// `sAnimationInfo1` (`AnimationFrameCountInfo`): animation, (frame count: unused by
/// `EnSa_ChangeAnim`), mode, morph frames.
const ANIMATION_INFO1: [(&str, f32, u8, f32); 12] = [
    (ANIM_WAIT_ARMS_TO_SIDE, 1.0, ANIMMODE_LOOP, 0.0),
    ("gSariaLookUpArmExtendedAnim", 1.0, ANIMMODE_ONCE, -10.0),
    ("gSariaWaveAnim", 1.0, ANIMMODE_LOOP, -10.0),
    ("gSariaRunAnim", 1.0, ANIMMODE_LOOP, -10.0),
    (ANIM_WAIT_ARMS_TO_SIDE, 1.0, ANIMMODE_LOOP, -10.0),
    ("gSariaLookOverShoulderAnim", 1.0, ANIMMODE_LOOP, -10.0),
    (ANIM_PLAYING_OCARINA, 1.0, ANIMMODE_LOOP, -10.0),
    (ANIM_STOP_PLAYING_OCARINA, 1.0, ANIMMODE_ONCE, -10.0),
    (ANIM_OCARINA_TO_MOUTH, 1.0, ANIMMODE_ONCE, -10.0),
    ("gSariaLinkLearnedSariasSongAnim", 1.0, ANIMMODE_ONCE, -10.0),
    ("gSariaReturnToOcarinaAnim", 1.0, ANIMMODE_ONCE, -10.0),
    (ANIM_PLAYING_OCARINA, 1.0, ANIMMODE_LOOP, 0.0),
];

/// `EnSaAnimation2`.
pub const ENSA_ANIM2_TRANS_HANDS_SIDE_CHEST_SIDE: usize = 0;
pub const ENSA_ANIM2_TRANS_HANDS_SIDE_TO_BACK: usize = 1;
pub const ENSA_ANIM2_RIGHT_ARM_EXTENDED_WAIT: usize = 2;
pub const ENSA_ANIM2_HANDS_OUT: usize = 3;
pub const ENSA_ANIM2_STAND_HANDS_ON_HIPS: usize = 4;
pub const ENSA_ANIM2_EXTEND_RIGHT_ARM: usize = 5;
pub const ENSA_ANIM2_TRANS_HANDS_SIDE_TO_HIPS: usize = 6;
pub const ENSA_ANIM2_BEHIND_BACK_WAIT: usize = 7;
pub const ENSA_ANIM2_HANDS_ON_FACE: usize = 8;
pub const ENSA_ANIM2_WAIT_ARMS_TO_SIDE: usize = 9;

/// `sAnimationInfo2` (`AnimationInfo`): animation, speed, start, end, mode, morph frames.
const ANIMATION_INFO2: [(&str, f32, f32, f32, u8, f32); 10] = [
    ("gSariaTransitionHandsSideToChestToSideAnim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, -1.0),
    ("gSariaTransitionHandsSideToBackAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -4.0),
    ("gSariaRightArmExtendedWaitAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -1.0),
    ("gSariaHandsOutAnim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, -1.0),
    ("gSariaStandHandsOnHipsAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -1.0),
    ("gSariaExtendRightArmAnim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, -1.0),
    ("gSariaTransitionHandsSideToHipsAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -1.0),
    (ANIM_HANDS_BEHIND_BACK_WAIT, 1.0, 0.0, -1.0, ANIMMODE_LOOP, -1.0),
    ("gSariaHandsOnFaceAnim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, -1.0),
    (ANIM_WAIT_ARMS_TO_SIDE, 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
];

/// `EnSaAnimGroup`.
pub const ENSA_ANIMGROUP_NONE: u8 = 0;
pub const ENSA_ANIMGROUP_EXTEND_RIGHT_HAND: u8 = 1;
pub const ENSA_ANIMGROUP_HANDS_TO_FACE: u8 = 2;
pub const ENSA_ANIMGROUP_HANDS_BEHIND_BACK: u8 = 3;
pub const ENSA_ANIMGROUP_HANDS_OUT_FROM_BEHIND_BACK: u8 = 4;
pub const ENSA_ANIMGROUP_HANDS_ON_HIPS: u8 = 5;
pub const ENSA_ANIMGROUP_HANDS_OFF_HIPS: u8 = 6;
pub const ENSA_ANIMGROUP_HAND_TO_CHEST: u8 = 7;

/// The limbs `EnSa_OverrideLimbDraw` and `EnSa_PostLimbDraw` name (`object_sa.xml`, 1-based as
/// `limbIndex`): the right chest (the torso's turn), the right hand (the ocarina's), the head.
const SARIA_LIMB_RIGHT_CHEST: usize = 9;
const SARIA_LIMB_RIGHT_HAND: usize = 15;
const SARIA_LIMB_HEAD: usize = 16;

/// `EnSa_Draw`'s `eyeTextures` and `mouthTextures`.
const EYE_TEXTURES: [&str; 5] = ["gSariaEyeOpenTex", "gSariaEyeHalfTex", "gSariaEyeClosedTex", "gSariaEyeSuprisedTex", "gSariaEyeSadTex"];
const MOUTH_TEXTURES: [&str; 5] = ["gSariaMouthClosed2Tex", "gSariaMouthSmilingOpenTex", "gSariaMouthFrowningTex", "gSariaMouthSuprisedTex", "gSariaMouthClosedTex"];
/// The segments `EnSa_Draw` binds: the right eye (8), the left eye (9), the mouth (0xA); a draw
/// puts its textures in the bake's place by these addresses. The bake binds each to a different
/// texture so the draw can tell them apart.
const SEG_RIGHT_EYE: u32 = 0x0800_0000;
const SEG_LEFT_EYE: u32 = 0x0900_0000;
const SEG_MOUTH: u32 = 0x0A00_0000;
/// `gSPSegment(0x0C, ...)`: `func_80034B28`'s empty list (opaque) or `func_80034B54`'s render
/// mode (translucent).
const SEG_RENDER_MODE: u8 = 0x0C;
/// The `gDPSetEnvColor(0, 0, 0, alpha)` of `func_80034BA0` / `func_80034CC4`, as a dynamic
/// colour run before the skeleton.
const SEG_ALPHA: u8 = 0x0E;
/// `func_80034B54`'s `gDPSetRenderMode(G_RM_FOG_SHADE_A, G_RM_AA_ZB_XLU_SURF2 | Z_UPD)`.
const XLU_RENDER_MODE: (u32, u32) = (0xE200_001C, 0xC810_49F8);

/// The bake of a pass (`xlu`), with the meadow's hand holding the ocarina or not.
pub fn bake_name(xlu: bool, ocarina: bool) -> String {
    format!("En_Sa/{}{}", if xlu { "xlu" } else { "opa" }, if ocarina { "_ocarina" } else { "" })
}

/// The meshes `EnSa_Draw` draws: opaque and translucent, each with and without
/// `gSariaRightHandAndOcarinaDL` on limb 15 (`EnSa_OverrideLimbDraw` in the meadow).
pub fn bakes() -> Vec<MeshBake> {
    let tex = |s: &str| BakeSegment::Texture { file: OBJECT.into(), symbol: s.into() };
    let mut v = Vec::new();
    for xlu in [false, true] {
        for ocarina in [false, true] {
            v.push(MeshBake {
                name: bake_name(xlu, ocarina),
                object: OBJECT.into(),
                segments: vec![
                    (0x08, tex(EYE_TEXTURES[SARIA_EYE_OPEN as usize])),
                    (0x09, tex(EYE_TEXTURES[SARIA_EYE_HALF as usize])),
                    (0x0A, tex(MOUTH_TEXTURES[SARIA_MOUTH_CLOSED2 as usize])),
                    (SEG_RENDER_MODE, BakeSegment::Commands(if xlu { vec![XLU_RENDER_MODE] } else { Vec::new() })),
                    (SEG_ALPHA, BakeSegment::DynamicColor { env: true, prim: false }),
                ],
                prelude: vec![SEG_ALPHA],
                body: BakeBody::Skeleton {
                    file: OBJECT.into(),
                    symbol: "gSariaSkel".into(),
                    limbs: if ocarina { vec![LimbOverride { limb: (SARIA_LIMB_RIGHT_HAND - 1) as u8, file: OBJECT.into(), symbol: "gSariaRightHandAndOcarinaDL".into() }] } else { Vec::new() },
                },
            });
        }
    }
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnSa_Idle`.
    Idle,
    /// `EnSa_TalkInForestMeadow`.
    TalkInForestMeadow,
    /// `EnSa_AwaitTeachingSariasSong`.
    AwaitTeachingSariasSong,
    /// `EnSa_HandleCutscene`.
    HandleCutscene,
    /// `EnSa_BecomeIdle`.
    BecomeIdle,
}

pub struct EnSa {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `interactInfo`: the talk state and the head and torso tracking.
    pub interact_info: NpcTrack,
    /// `messageIndex`: how many of the conversation's boxes have gone by; `prevTextState` the
    /// last `Message_GetState`.
    pub message_index: u8,
    pub prev_text_state: u8,
    /// `animPhase`: the animation group's step; `animGroup` the group (`EnSa_Animate`).
    pub anim_phase: u8,
    pub anim_group: u8,
    pub step_timer: i16,
    pub blink_timer: i16,
    pub cue_id: i16,
    pub right_eye_index: i16,
    pub left_eye_index: i16,
    pub mouth_index: i16,
    pub alpha: i16,
    /// `initRot`: `shape.rot` as placed.
    pub init_rot: Rot,
    animations1: Vec<Anim>,
    animations2: Vec<AnimationInfo>,
    /// The face textures as images, by `EYE_TEXTURES` then `MOUTH_TEXTURES`.
    face: Vec<DrawImage>,
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

fn player_pos(play: &PlayState) -> Option<Vec3> {
    play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos)
}

/// `EnSa_GetType`: her type from the scene, the save and the cutscene index.
pub fn get_type(play: &PlayState) -> i32 {
    let s = &play.save;
    if s.cutscene_index >= CS_INDEX_0 && s.cutscene_index != CS_INDEX_D {
        if play.scene_id == SCENE_KOKIRI_FOREST {
            return SARIA_TYPE_GREETING;
        }
        if play.scene_id == SCENE_SACRED_FOREST_MEADOW {
            return SARIA_TYPE_MEADOW_WAITING;
        }
    }
    if play.scene_id == SCENE_SARIAS_HOUSE && !s.adult && s.inv_content(ITEM_OCARINA_FAIRY) == ITEM_OCARINA_FAIRY && !s.get_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER) {
        return SARIA_TYPE_STANDING;
    }
    if play.scene_id == SCENE_SACRED_FOREST_MEADOW && s.get_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER) {
        return if s.check_quest_item(QUEST_SONG_SARIA) { SARIA_TYPE_MEADOW_PLAYING } else { SARIA_TYPE_MEADOW_WAITING };
    }
    if play.scene_id == SCENE_KOKIRI_FOREST && !s.check_quest_item(QUEST_KOKIRI_EMERALD) {
        if s.get_inf_table(INFTABLE_SARIA_GREETED_LINK) {
            return SARIA_TYPE_STANDING;
        }
        return SARIA_TYPE_GREETING;
    }
    SARIA_TYPE_NONE
}

/// `EnSa_UpdateTextState`: counts the conversation's boxes (`messageIndex`): each time the state
/// leaves waiting for the next box, an event, the closing, or a box with a next text.
fn update_text_state(play: &PlayState, message_index: &mut u8, prev_text_state: &mut u8) -> u8 {
    let text_state = play.message_state();
    if matches!(*prev_text_state, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_EVENT | TEXT_STATE_CLOSING | TEXT_STATE_DONE_HAS_NEXT) && text_state != *prev_text_state {
        *message_index = message_index.wrapping_add(1);
    }
    *prev_text_state = text_state;
    text_state
}

/// `EnSa_GetTextId`: her text for the save's progress, resetting the box count (but for 0x10AD
/// and 0x1001).
pub fn get_text_id(play: &PlayState, message_index: &mut u8, prev_text_state: &mut u8) -> u16 {
    let s = &play.save;
    // MaskReaction_GetTextId(play, MASK_REACTION_SET_SARIA): no masks, so 0.
    let text_id = 0u16;
    if text_id != 0 {
        return text_id;
    }
    if s.check_quest_item(QUEST_SONG_SARIA) {
        return 0x10AD;
    }
    if s.check_quest_item(QUEST_KOKIRI_EMERALD) {
        *message_index = 0;
        *prev_text_state = TEXT_STATE_NONE;
        return if s.get_inf_table(INFTABLE_SARIA_SPOKE_IN_HER_HOUSE) { 0x1048 } else { 0x1047 };
    }
    if s.get_event_chk_inf(EVENTCHKINF_MIDO_DENIED_DEKU_TREE_ACCESS) {
        *message_index = 0;
        *prev_text_state = TEXT_STATE_NONE;
        return if s.get_inf_table(INFTABLE_SARIA_WAS_TOLD_ABOUT_MIDO) { 0x1032 } else { 0x1031 };
    }
    if s.get_inf_table(INFTABLE_SARIA_GREETED_LINK) {
        *message_index = 0;
        *prev_text_state = TEXT_STATE_NONE;
        return if s.get_inf_table(INFTABLE_SARIA_NOTICED_FAIRY) { 0x1003 } else { 0x1002 };
    }
    0x1001
}

/// `EnSa_UpdateTalkState`: talking until the box closes; as it closes, the flags 0x1002, 0x1031
/// and 0x1047 set, then idle.
fn update_talk_state(play: &mut PlayState, actor: &Actor, message_index: &mut u8, prev_text_state: &mut u8) -> i16 {
    let mut talk_state = NPC_TALK_STATE_TALKING;
    if update_text_state(play, message_index, prev_text_state) == TEXT_STATE_CLOSING {
        match actor.text_id {
            0x1002 => {
                play.save.set_inf_table(INFTABLE_SARIA_NOTICED_FAIRY);
                talk_state = NPC_TALK_STATE_IDLE;
            }
            0x1031 => {
                play.save.set_event_chk_inf(EVENTCHKINF_SARIA_WAS_TOLD_ABOUT_MIDO);
                play.save.set_inf_table(INFTABLE_SARIA_WAS_TOLD_ABOUT_MIDO);
                talk_state = NPC_TALK_STATE_IDLE;
            }
            0x1047 => {
                play.save.set_inf_table(INFTABLE_SARIA_SPOKE_IN_HER_HOUSE);
                talk_state = NPC_TALK_STATE_IDLE;
            }
            _ => talk_state = NPC_TALK_STATE_IDLE,
        }
    }
    // TEXT_STATE_NONE, _DONE_HAS_NEXT, _DONE_FADING, _CHOICE, _EVENT, _SONG_DEMO_DONE, _8, _9 (and
    // the others): still talking.
    talk_state
}

impl EnSa {
    /// `EnSa_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.skeleton(OBJECT, "gSariaSkel")?;
            let animations1 = ANIMATION_INFO1.iter().map(|&(name, ..)| assets.animation(OBJECT, name)).collect::<anyhow::Result<Vec<_>>>()?;
            let animations2 = ANIMATION_INFO2
                .iter()
                .map(|&(name, play_speed, start_frame, frame_count, mode, morph_frames)| assets.animation(OBJECT, name).map(|animation| AnimationInfo { animation, play_speed, start_frame, frame_count, mode, morph_frames }))
                .collect::<anyhow::Result<Vec<_>>>()?;
            let mut face = Vec::new();
            for s in EYE_TEXTURES.iter().chain(MOUTH_TEXTURES.iter()) {
                let t = assets.pack.texture(OBJECT, s)?;
                face.push(DrawImage { width: t.width, height: t.height, rgba: t.rgba.clone().into() });
            }
            Ok((skeleton, animations1, animations2, face))
        })();
        let (skeleton, animations1, animations2, face) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Sa: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnSa {
            actor,
            // SkelAnime_InitFlex(.., &gSariaSkel, NULL, jointTable, morphTable, 17).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            action: Action::Idle,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            interact_info: NpcTrack::default(),
            message_index: 0,
            prev_text_state: TEXT_STATE_NONE,
            anim_phase: 0,
            anim_group: ENSA_ANIMGROUP_NONE,
            step_timer: 0,
            blink_timer: 0,
            cue_id: 0,
            right_eye_index: SARIA_EYE_OPEN,
            left_eye_index: SARIA_EYE_OPEN,
            mouth_index: SARIA_MOUTH_CLOSED2,
            alpha: 0,
            init_rot: Rot::default(),
            animations1,
            animations2,
            face,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 12.0);
        // Collider_SetCylinder (sCylinderInit, above), CollisionCheck_SetInfo2.
        this.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);

        match get_type(play) {
            SARIA_TYPE_MEADOW_PLAYING => {
                this.change_anim(ENSA_ANIM1_START_PLAYING_OCARINA);
                this.action = Action::Idle;
            }
            SARIA_TYPE_MEADOW_WAITING => {
                this.change_anim(ENSA_ANIM1_START_PLAYING_OCARINA);
                this.action = Action::AwaitTeachingSariasSong;
            }
            SARIA_TYPE_STANDING => {
                this.actor.gravity = -1.0;
                this.change_anim(ENSA_ANIM1_WAIT_ARMS_TO_SIDE);
                this.action = Action::Idle;
            }
            SARIA_TYPE_GREETING => {
                this.cue_id = 0;
                this.actor.gravity = -1.0;
                play.cs_ctx.segment = play.cutscene_script("gKokiriForestSariaGreetingCs");
                play.save.cutscene_trigger = 1;
                this.change_anim(ENSA_ANIM1_MOVE_ARMS_TO_SIDE);
                this.action = Action::HandleCutscene;
            }
            SARIA_TYPE_UNUSED => {
                this.cue_id = 0;
                this.actor.gravity = -1.0;
                this.change_anim(ENSA_ANIM1_WAIT_ARMS_TO_SIDE);
                this.action = Action::HandleCutscene;
            }
            _ => {
                // SARIA_TYPE_NONE.
                this.actor.kill();
                return Box::new(this);
            }
        }

        this.actor.scale = Vec3::splat(0.01);
        // attentionRangeType = ATTENTION_RANGE_6.
        this.actor.target_mode = 6;
        this.interact_info.talk_state = NPC_TALK_STATE_IDLE;
        this.alpha = 255;
        this.init_rot = this.actor.shape_rot;
        let pos = this.actor.world_pos;
        if let Err(e) = play.actor_spawn_as_child(&mut this.actor, ACTOR_EN_ELF, pos, [0; 3], FAIRY_KOKIRI) {
            log::debug!("En_Sa's fairy: {e:?}");
        }
        Box::new(this)
    }

    /// `EnSa_ChangeAnim`: `sAnimationInfo1[index]` from its first frame to its last at speed 1.
    fn change_anim(&mut self, index: usize) {
        let Some(&(_, _, mode, morph_frames)) = ANIMATION_INFO1.get(index) else {
            log::debug!("En_Sa: no sAnimationInfo1[{index}]");
            return;
        };
        let a = self.animations1[index].clone();
        let last = a.last_frame();
        self.skel.change(a, 1.0, 0.0, last, mode, morph_frames);
    }

    /// `Animation_ChangeByInfo(&skelAnime, sAnimationInfo2, index)`.
    fn change_anim2(&mut self, index: usize) {
        if let Some(info) = self.animations2.get(index) {
            self.skel.change_by_info(info);
        }
    }

    fn anim(&self, index: usize) -> Anim {
        self.animations1[index].clone()
    }

    /// `EnSa_ReverseAnimation`: the animation played backwards from its end; returns its old
    /// start.
    fn reverse_animation(&mut self) -> f32 {
        let s = &mut self.skel;
        let end_frame = s.end_frame;
        let start_frame = s.start_frame;
        s.start_frame = end_frame;
        s.cur_frame = end_frame;
        s.end_frame = start_frame;
        s.play_speed = -1.0;
        start_frame
    }

    /// An animation group's steps: at phase 0 `first` (backwards if `reversed`), falling through
    /// to phase 1, where at its end frame `then`.
    fn anim_group_step(&mut self, first: usize, reversed: bool, then: usize) {
        if self.anim_phase == 0 {
            self.change_anim2(first);
            if reversed {
                self.reverse_animation();
            }
            self.anim_phase += 1;
        }
        if self.anim_phase == 1 && self.skel.on_frame(self.skel.end_frame) {
            self.change_anim2(then);
            self.anim_phase += 1;
        }
    }

    /// `EnSa_ExtendRightHand`.
    fn extend_right_hand(&mut self) {
        self.anim_group_step(ENSA_ANIM2_HANDS_OUT, false, ENSA_ANIM2_RIGHT_ARM_EXTENDED_WAIT);
    }

    /// `EnSa_HandsToFace`.
    fn hands_to_face(&mut self) {
        self.anim_group_step(ENSA_ANIM2_HANDS_ON_FACE, false, ENSA_ANIM2_WAIT_ARMS_TO_SIDE);
    }

    /// `EnSa_HandsBehindBack`.
    fn hands_behind_back(&mut self) {
        self.anim_group_step(ENSA_ANIM2_TRANS_HANDS_SIDE_TO_BACK, false, ENSA_ANIM2_BEHIND_BACK_WAIT);
    }

    /// `EnSa_HandsOutFromBehindBack`.
    fn hands_out_from_behind_back(&mut self) {
        self.anim_group_step(ENSA_ANIM2_TRANS_HANDS_SIDE_TO_BACK, true, ENSA_ANIM2_WAIT_ARMS_TO_SIDE);
    }

    /// `EnSa_HandsOnHips`.
    fn hands_on_hips(&mut self) {
        self.anim_group_step(ENSA_ANIM2_TRANS_HANDS_SIDE_TO_HIPS, false, ENSA_ANIM2_STAND_HANDS_ON_HIPS);
    }

    /// `EnSa_HandsOffHips`.
    fn hands_off_hips(&mut self) {
        self.anim_group_step(ENSA_ANIM2_TRANS_HANDS_SIDE_TO_HIPS, true, ENSA_ANIM2_WAIT_ARMS_TO_SIDE);
    }

    /// `EnSa_HandToChest`.
    fn hand_to_chest(&mut self) {
        self.anim_group_step(ENSA_ANIM2_EXTEND_RIGHT_ARM, false, ENSA_ANIM2_TRANS_HANDS_SIDE_CHEST_SIDE);
    }

    /// `EnSa_SetAnimGroup`.
    fn set_anim_group(&mut self, anim_group: u8) {
        self.anim_group = anim_group;
        self.anim_phase = 0;
    }

    /// `EnSa_Animate`.
    fn animate(&mut self) {
        match self.anim_group {
            ENSA_ANIMGROUP_EXTEND_RIGHT_HAND => self.extend_right_hand(),
            ENSA_ANIMGROUP_HANDS_TO_FACE => self.hands_to_face(),
            ENSA_ANIMGROUP_HANDS_BEHIND_BACK => self.hands_behind_back(),
            ENSA_ANIMGROUP_HANDS_OUT_FROM_BEHIND_BACK => self.hands_out_from_behind_back(),
            ENSA_ANIMGROUP_HANDS_ON_HIPS => self.hands_on_hips(),
            ENSA_ANIMGROUP_HANDS_OFF_HIPS => self.hands_off_hips(),
            ENSA_ANIMGROUP_HAND_TO_CHEST => self.hand_to_chest(),
            _ => {}
        }
    }

    /// `EnSa_UpdateTalking`: `Npc_UpdateTalking` within her radius and 30 (in the meadow only
    /// while she faces Link within 0x1555, or already talking).
    fn update_talking(&mut self, play: &mut PlayState) {
        let yaw_diff = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if play.scene_id != SCENE_SACRED_FOREST_MEADOW || (yaw_diff as i32).abs() < 0x1555 || self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            let range = self.collider.dim.radius as f32 + 30.0;
            let mut talk_state = self.interact_info.talk_state;
            // Both callbacks read and write her box count (messageIndex, prevTextState).
            let counts = std::cell::Cell::new((self.message_index, self.prev_text_state));
            oot_game::npc::talk_update(
                play,
                &mut self.actor,
                &mut talk_state,
                range,
                |play, _| {
                    let (mut a, mut b) = counts.get();
                    let text = get_text_id(play, &mut a, &mut b);
                    counts.set((a, b));
                    text
                },
                |play, actor| {
                    let (mut a, mut b) = counts.get();
                    let r = update_talk_state(play, actor, &mut a, &mut b);
                    counts.set((a, b));
                    r
                },
            );
            (self.message_index, self.prev_text_state) = counts.get();
            self.interact_info.talk_state = talk_state;
        }
    }

    /// `EnSa_UpdateTrackingMode`: her head (and in Kokiri Forest her body) on Link, but not while
    /// she plays the ocarina or in the greeting.
    fn update_tracking_mode(&mut self, play: &mut PlayState) {
        let mut tracking_mode = NPC_TRACKING_PLAYER_AUTO_TURN;
        if play.scene_id == SCENE_KOKIRI_FOREST {
            tracking_mode = if self.action == Action::HandleCutscene { NPC_TRACKING_NONE } else { NPC_TRACKING_FULL_BODY };
        }
        if play.scene_id == SCENE_SACRED_FOREST_MEADOW {
            tracking_mode = if self.skel.is(ANIM_PLAYING_OCARINA) { NPC_TRACKING_NONE } else { NPC_TRACKING_HEAD };
        }
        if play.scene_id == SCENE_SACRED_FOREST_MEADOW && self.action == Action::Idle && self.skel.is(ANIM_STOP_PLAYING_OCARINA) {
            tracking_mode = NPC_TRACKING_NONE;
        }
        if play.scene_id == SCENE_SACRED_FOREST_MEADOW && self.action == Action::HandleCutscene && self.skel.is(ANIM_OCARINA_TO_MOUTH) {
            tracking_mode = NPC_TRACKING_NONE;
        }
        self.interact_info.target = player_pos(play).unwrap_or(self.actor.world_pos);
        self.interact_info.eye_height = 4.0;
        let mut t = self.interact_info;
        track_point(play, &mut self.actor, &mut t, 2, tracking_mode);
        self.interact_info = t;
    }

    /// `EnSa_EyesStayClosed`: her eyes held shut while she plays (or raises) the ocarina, not
    /// talking (the blink timer held at 0).
    fn eyes_stay_closed(&mut self) -> bool {
        if !self.skel.is(ANIM_PLAYING_OCARINA) && !self.skel.is(ANIM_OCARINA_TO_MOUTH) {
            return false;
        }
        if self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            return false;
        }
        self.blink_timer = 0;
        if self.right_eye_index != SARIA_EYE_CLOSED {
            return false;
        }
        true
    }

    /// `EnSa_Blink`: open, half, closed, then open for 30 to 60 frames.
    fn blink(&mut self, play: &mut PlayState) {
        if !self.eyes_stay_closed() {
            let blink_timer = decr(&mut self.blink_timer);
            if blink_timer == 0 {
                self.right_eye_index += 1;
                if self.right_eye_index < SARIA_EYE_SURPRISED {
                    self.left_eye_index = self.right_eye_index;
                } else {
                    self.blink_timer = play.rand.s16_offset(30, 30);
                    self.left_eye_index = SARIA_EYE_OPEN;
                    self.right_eye_index = self.left_eye_index;
                }
            }
        }
    }

    /// `EnSa_Idle`: in Kokiri Forest her arms by her text's boxes (or behind her back once she's
    /// noticed the fairy or heard of Mido); the ocarina's stop played back to playing; in the
    /// meadow a talk stops her playing.
    fn idle(&mut self, play: &PlayState) {
        if play.scene_id == SCENE_KOKIRI_FOREST {
            if self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
                let m = self.message_index;
                match self.actor.text_id {
                    0x1002 => {
                        if m == 0 && self.anim_group != ENSA_ANIMGROUP_EXTEND_RIGHT_HAND {
                            self.set_anim_group(ENSA_ANIMGROUP_EXTEND_RIGHT_HAND);
                            self.mouth_index = SARIA_MOUTH_SMILING_OPEN;
                        }
                        if m == 2 && self.anim_group != ENSA_ANIMGROUP_HANDS_TO_FACE {
                            self.set_anim_group(ENSA_ANIMGROUP_HANDS_TO_FACE);
                            self.mouth_index = SARIA_MOUTH_SMILING_OPEN;
                        }
                        if m == 5 {
                            self.mouth_index = SARIA_MOUTH_CLOSED2;
                        }
                    }
                    0x1003 => {
                        if m == 0 && self.anim_group != ENSA_ANIMGROUP_HANDS_OUT_FROM_BEHIND_BACK {
                            self.set_anim_group(ENSA_ANIMGROUP_HANDS_OUT_FROM_BEHIND_BACK);
                        }
                    }
                    0x1031 => {
                        if m == 0 && self.anim_group != ENSA_ANIMGROUP_HANDS_OUT_FROM_BEHIND_BACK && self.skel.is(ANIM_HANDS_BEHIND_BACK_WAIT) {
                            self.set_anim_group(ENSA_ANIMGROUP_HANDS_OUT_FROM_BEHIND_BACK);
                            self.mouth_index = SARIA_MOUTH_SURPRISED;
                        }
                        if m == 2 && self.anim_group != ENSA_ANIMGROUP_HANDS_ON_HIPS {
                            self.set_anim_group(ENSA_ANIMGROUP_HANDS_ON_HIPS);
                            self.mouth_index = SARIA_MOUTH_FROWNING;
                        }
                        if m == 4 && self.anim_group != ENSA_ANIMGROUP_HANDS_OFF_HIPS {
                            self.set_anim_group(ENSA_ANIMGROUP_HANDS_OFF_HIPS);
                            self.mouth_index = SARIA_MOUTH_CLOSED2;
                        }
                    }
                    0x1032 => {
                        if m == 0 && self.anim_group != ENSA_ANIMGROUP_HANDS_OUT_FROM_BEHIND_BACK && self.skel.is(ANIM_HANDS_BEHIND_BACK_WAIT) {
                            self.set_anim_group(ENSA_ANIMGROUP_HANDS_OUT_FROM_BEHIND_BACK);
                        }
                    }
                    0x1047 => {
                        if m == 1 && self.anim_group != ENSA_ANIMGROUP_HAND_TO_CHEST {
                            self.set_anim_group(ENSA_ANIMGROUP_HAND_TO_CHEST);
                        }
                    }
                    0x1048 => {
                        if m == 0 && self.anim_group != ENSA_ANIMGROUP_HAND_TO_CHEST {
                            self.set_anim_group(ENSA_ANIMGROUP_HAND_TO_CHEST);
                        }
                    }
                    _ => {}
                }
            } else if !play.save.check_quest_item(QUEST_KOKIRI_EMERALD) && (play.save.get_inf_table(INFTABLE_SARIA_NOTICED_FAIRY) || play.save.get_inf_table(INFTABLE_SARIA_WAS_TOLD_ABOUT_MIDO)) {
                if self.anim_group != ENSA_ANIMGROUP_HANDS_BEHIND_BACK {
                    self.set_anim_group(ENSA_ANIMGROUP_HANDS_BEHIND_BACK);
                }
            } else {
                self.set_anim_group(ENSA_ANIMGROUP_NONE);
            }
            self.animate();
        }
        if self.skel.is(ANIM_STOP_PLAYING_OCARINA) {
            self.skel.play_speed = -1.0;
            if self.skel.cur_frame as i32 == 0 {
                self.change_anim(ENSA_ANIM1_PLAYING_OCARINA);
            }
        }
        if self.interact_info.talk_state != NPC_TALK_STATE_IDLE && play.scene_id == SCENE_SACRED_FOREST_MEADOW {
            let a = self.anim(ENSA_ANIM1_STOP_PLAYING_OCARINA);
            self.skel.change(a, 1.0, 0.0, 10.0, ANIMMODE_ONCE, -10.0);
            self.action = Action::TalkInForestMeadow;
        }
    }

    /// `EnSa_TalkInForestMeadow`: the talk over, the ocarina's stop played back from frame 10.
    fn talk_in_forest_meadow(&mut self) {
        if self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            return;
        }
        let a = self.anim(ENSA_ANIM1_STOP_PLAYING_OCARINA);
        self.skel.change(a, 0.0, 10.0, 0.0, ANIMMODE_ONCE, -10.0);
        self.action = Action::Idle;
    }

    /// `EnSa_AwaitTeachingSariasSong`: Link north of z -2220 outside a cutscene: the song's script.
    fn await_teaching_sarias_song(&mut self, play: &mut PlayState) {
        let z = player_pos(play).map(|p| p.z).unwrap_or(0.0);
        if !(z >= -2220.0) && !play.play_in_cs_mode() {
            play.cs_ctx.segment = play.cutscene_script("gMeadowSariasSongCs");
            play.save.cutscene_trigger = 1;
            self.action = Action::HandleCutscene;
        }
    }

    /// `EnSa_HandleCutscene`: the script's cue on channel 1: placed at its start the first time,
    /// its animation and mouth when it changes, her steps while running (cue 3), its rotation, and
    /// a velocity from its start to its end over its frames.
    fn handle_cutscene(&mut self, play: &mut PlayState) {
        if play.cs_ctx.state == CS_STATE_IDLE {
            self.action = Action::BecomeIdle;
            return;
        }
        let Some(cue) = play.cs_ctx.npc_actions[1] else { return };
        // EnSa_CopyCueStartPos, EnSa_CopyCueEndPos.
        let start_pos = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
        let end_pos = Vec3::new(cue.end_pos.x as f32, cue.end_pos.y as f32, cue.end_pos.z as f32);
        if self.cue_id == 0 {
            self.actor.world_pos = start_pos;
        }
        if self.cue_id as i32 != cue.action as i32 {
            self.mouth_index = match cue.action {
                2 | 9 => SARIA_MOUTH_SMILING_OPEN,
                _ => SARIA_MOUTH_CLOSED2,
            };
            self.change_anim(cue.action as usize);
            self.cue_id = cue.action as i16;
        }
        if cue.action == 3 {
            let step_timer = decr(&mut self.step_timer);
            if step_timer == 0 {
                // SFX_PLAY_AT_POS(&projectedPos, NA_SE_PL_WALK_GROUND + SURFACE_SFX_OFFSET_DIRT).
                let pos = play.cur_actor.map(SfxPos::Actor).unwrap_or(SfxPos::Default);
                play.audio.play_sfx_at_pos(pos, NA_SE_PL_WALK_GROUND + SURFACE_SFX_OFFSET_DIRT);
                self.step_timer = 8;
            }
        }
        self.actor.shape_rot = Rot { x: cue.rot[0], y: cue.rot[1], z: cue.rot[2] };
        self.actor.velocity = Vec3::ZERO;
        if play.cs_ctx.frames < cue.end_frame {
            let duration = (cue.end_frame as i32 - cue.start_frame as i32) as f32;
            self.actor.velocity.x = (end_pos.x - start_pos.x) / duration;
            self.actor.velocity.y = (end_pos.y - start_pos.y) / duration;
            let mut gravity = self.actor.gravity;
            if play.scene_id == SCENE_SACRED_FOREST_MEADOW {
                gravity = 0.0;
            }
            self.actor.velocity.y += gravity;
            if self.actor.velocity.y < self.actor.min_velocity_y {
                self.actor.velocity.y = self.actor.min_velocity_y;
            }
            self.actor.velocity.z = (end_pos.z - start_pos.z) / duration;
        }
    }

    /// `EnSa_BecomeIdle`: the script over. In the meadow Saria's Song given and the ocarina
    /// played on; in Kokiri Forest home, as placed, the greeting done.
    fn become_idle(&mut self, play: &mut PlayState) {
        if play.scene_id == SCENE_SACRED_FOREST_MEADOW {
            item_give(&mut play.save, Some(&mut play.audio), ITEM_SONG_SARIA);
            self.change_anim(ENSA_ANIM1_PLAYING_OCARINA);
        }
        if play.scene_id == SCENE_KOKIRI_FOREST {
            self.change_anim(ENSA_ANIM1_MOVE_ARMS_TO_SIDE);
            self.actor.world_pos = self.actor.home_pos;
            self.actor.world_rot = self.init_rot;
            self.mouth_index = SARIA_MOUTH_CLOSED2;
            play.save.set_inf_table(INFTABLE_SARIA_GREETED_LINK);
        }
        self.action = Action::Idle;
    }

    /// Her skeleton posed as `EnSa_OverrideLimbDraw` turns it: the head (limb 16) about 900 along
    /// it by the head's turn (x by its yaw, z by its pitch), the right chest (limb 9) by the
    /// torso's (y, then x).
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 3], torso: [i16; 3]) -> Vec<Mat4> {
        let r = binang_to_rad;
        skeleton.pose_override(joints, |limb, _pos, _rot| {
            if limb == SARIA_LIMB_HEAD {
                return Mat4::from_translation(Vec3::new(900.0, 0.0, 0.0)) * Mat4::from_rotation_x(r(head[1])) * Mat4::from_rotation_z(r(head[0])) * Mat4::from_translation(Vec3::new(-900.0, 0.0, 0.0));
            }
            if limb == SARIA_LIMB_RIGHT_CHEST {
                return Mat4::from_rotation_y(r(torso[1])) * Mat4::from_rotation_x(r(torso[0]));
            }
            Mat4::IDENTITY
        })
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `angles`: the head's x, y, z, then the torso's.
    pub const HEAD: usize = 0;
    pub const TORSO: usize = 3;
    /// `values`: the alpha.
    pub const ALPHA: usize = 0;
    /// `switches`: the right eye, the left eye, the mouth.
    pub const RIGHT_EYE: usize = 0;
    pub const LEFT_EYE: usize = 1;
    pub const MOUTH: usize = 2;
}

impl ActorImpl for EnSa {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnSa_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.skel.update();
        if self.skel.is(ANIM_OCARINA_TO_MOUTH) && self.skel.cur_frame >= self.anim(ENSA_ANIM1_OCARINA_TO_MOUTH).last_frame() {
            self.change_anim(ENSA_ANIM1_PLAYING_OCARINA);
        }
        if self.action != Action::HandleCutscene {
            self.alpha = actor_update_alpha_by_distance(&mut self.actor, play, self.alpha, 400.0);
        } else {
            self.alpha = 255;
        }
        self.actor.shape_shadow_alpha = self.alpha as u8;
        if self.action == Action::HandleCutscene {
            self.actor.world_pos += self.actor.velocity;
        } else {
            self.actor.update_pos();
        }
        if play.scene_id != SCENE_SACRED_FOREST_MEADOW {
            self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        }
        self.blink(play);
        match self.action {
            Action::Idle => self.idle(play),
            Action::TalkInForestMeadow => self.talk_in_forest_meadow(),
            Action::AwaitTeachingSariasSong => self.await_teaching_sarias_song(play),
            Action::HandleCutscene => self.handle_cutscene(play),
            Action::BecomeIdle => self.become_idle(play),
        }
        self.update_talking(play);
        self.update_tracking_mode(play);
    }

    /// `EnSa_PostLimbDraw`: the focus 400 along her head (limb 16).
    fn draw_update(&mut self, _play: &mut PlayState) {
        let t = &self.interact_info;
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, t.head, t.torso);
        if let Some(head) = bones.get(SARIA_LIMB_HEAD - 1) {
            let m = actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(Vec3::new(400.0, 0.0, 0.0));
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let (h, t) = (self.interact_info.head, self.interact_info.torso);
        rs.angles = vec![h[0], h[1], h[2], t[0], t[1], t[2]];
        rs.values = vec![self.alpha as f32];
        rs.switches = vec![self.right_eye_index.max(0) as u32, self.left_eye_index.max(0) as u32, self.mouth_index.max(0) as u32];
        rs
    }

    /// `EnSa_Draw`: opaque at full alpha (`func_80034BA0`), translucent while fading
    /// (`func_80034CC4`), nothing at 0; the eyes on 8 and 9 and the mouth on 0xA; in the meadow her
    /// right hand holds the ocarina.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 6 || rs.values.is_empty() || rs.switches.len() < 3 {
            return;
        }
        let alpha = rs.values[rs::ALPHA] as i32 as i16;
        if alpha == 0 {
            return;
        }

        let a = &rs.angles;
        let bones = Self::pose(&self.skeleton, &joints.rot, [a[rs::HEAD], a[rs::HEAD + 1], a[rs::HEAD + 2]], [a[rs::TORSO], a[rs::TORSO + 1], a[rs::TORSO + 2]]);
        let eye = |i: u32| self.face[(i as usize).min(EYE_TEXTURES.len() - 1)].clone();
        let mouth = self.face[EYE_TEXTURES.len() + (rs.switches[rs::MOUTH] as usize).min(MOUTH_TEXTURES.len() - 1)].clone();
        let texture_images = vec![
            SourceImage { source: SEG_RIGHT_EYE, image: eye(rs.switches[rs::RIGHT_EYE]) },
            SourceImage { source: SEG_LEFT_EYE, image: eye(rs.switches[rs::LEFT_EYE]) },
            SourceImage { source: SEG_MOUTH, image: mouth },
        ];
        let xlu = alpha != 255;
        let ocarina = play.scene_id == SCENE_SACRED_FOREST_MEADOW;
        let mut sv = SegmentValues::default();
        sv.env[SEG_ALPHA as usize] = Some([0, 0, 0, alpha.clamp(0, 255) as u8]);
        let cmd = DrawCmd {
            mesh: MeshKey::named(keys::bake(&bake_name(xlu, ocarina))),
            transform: actor_draw_matrix(rs),
            bones,
            params: DrawParams { segments: Some(sv), texture_images, ..Default::default() },
        };
        if xlu {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
    }

    /// `EnSa_Destroy`: `Collider_DestroyCylinder`.
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
