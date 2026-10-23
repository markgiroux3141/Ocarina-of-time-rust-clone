//! `En_Fr` (`ovl_En_Fr/z_en_fr.c`): the frogs on Zora's River's log (`spot03` room 0, layers 0
//! and 1), and the invisible song spot that runs them. Six instances: params 0 the spot (at (990,
//! 205, -1220), where Link plays), params 1 to 5 the frogs (`frogIndex = params - 1`: yellow, blue,
//! red, purple, white, `sEnFrColor`). The overlay's `sEnFrPointers` (the frogs, and `flags`, the
//! count that sends them out of the water one by one and, at 12, back in) is [`Statics`] in the
//! play state (ADR 0020, ADR 0059); each frog puts itself there at its first update.
//!
//! - **The song spot** (`EnFr_UpdateIdle`, a prop, not drawn, not targetable): Link above it
//!   within 30 across makes his ocarina turn to it (`unk_6A8`). His ocarina out there
//!   (`PLAYER_STATE2_25`): one-point cutscene 4110 (the log, Link held), Link put on the spot
//!   facing its way, the box paused, and `flags` 1: the frogs come out (`EnFr_Activate`,
//!   `EnFr_ActivateCheckFrogSong`). All out (`flags` 11): with all six child songs played to them
//!   (`EVENTCHKINF_SONGS_FOR_FROGS_ZL` to `_STORMS`) text 0x40AB and **the song game**, else **the
//!   free play** (`OCARINA_ACTION_CHECK_NOWARP`, once all five sit on the log, `func_80A1BE98`).
//! - **The free play** (`EnFr_ListeningToOcarinaNotes`): a note (`OCARINA_MODE_01`) makes its frog
//!   jump (A blue, C-down yellow, C-right red, C-left purple, C-up white: `EnFr_SetupJumpingUp`,
//!   `NA_SE_EV_FROG_JUMP` transposed by the frog's note, `sSmallFrogNotes` or, grown,
//!   `sLargeFrogNotes`); a song (`OCARINA_MODE_05` to `_0A`: Saria's, Epona's, Zelda's Lullaby,
//!   the Sun's, the Song of Time, the Song of Storms) sets off 48 frames of jumps
//!   (`EnFr_ChildSong`, `sJumpOrder` every 4), then: the Song of Storms, the chime and text 0x40AA;
//!   a child song the first time, the chime, its frog (`sSongToFrog`) grows
//!   (`NA_SE_EV_FROG_GROW_UP`, `EnFr_SetScaleActive`: 150 to 180, 210, 240, 270 and back between)
//!   and then 0x40A9 with the treasure sound; played before, 0x40A9 with the treasure sound. The
//!   ocarina put away (`OCARINA_MODE_04`) ends it (`EnFr_OcarinaMistake`).
//! - **The song game** (`EnFr_SetupFrogSong`, `EnFr_ContinueFrogSong`): `OCARINA_ACTION_FROGS`,
//!   14 notes (`EnFr_GetNextNoteFrogSong`: `gFrogsSongPtr`'s until `EVENTCHKINF_SONGS_FOR_FROGS_CHOIR`,
//!   then random ones), the next note's frog lit with a butterfly over it
//!   (`EnFr_CheckOcarinaInputFrogSong`). Each note played (the box's `MSGMODE_FROGS_WAITING`) makes
//!   its frog jump (the choir's `NA_SE_EN_DODO_M_EAT` at the jump); the right one moves on, 40, 20,
//!   15 then 12 frames for the next (`sTimerFrogSong`). Out of time, or B with all five on the log,
//!   is a mistake (`NA_SE_SY_OCARINA_ERROR`, no reward). All 14: the chime and text 0x40AC.
//! - **The rewards** (`EnFr_SetupReward` to `EnFr_SetReward`): the text read, the frogs dive back
//!   in (`flags` 12), and when all are under, `NA_SE_EV_FROG_CRY_0` and the item
//!   (`Actor_OfferGetItem`, 30 across, 100 up): a child song the first time a purple rupee and its
//!   `EVENTCHKINF_SONGS_FOR_FROGS_*` (else a blue one); the Song of Storms the first time a piece
//!   of heart (`_STORMS`; else blue); the song game the first time a piece of heart (`_CHOIR`;
//!   else purple). Then back to waiting (`EnFr_SetIdle`, as the item's text closes).
//! - **The frogs** (`EnFr_UpdateActive`, from their first update with `gameplay_field_keep`
//!   loaded): under the water 80 out from their log spots (`sLogSpotToFromWater`), drawn only the
//!   purple one (`EnFr_DrawIdle`); at their turn of `flags` (`sTimerJumpingOutOfWater`; @bug
//!   (game): bumped each frame a jump holds, so they overlap) they jump
//!   out (`object_fr_Anim_0007BC`, 47 up), land on the spot, turn to Link, and sit
//!   (`EnFr_ChooseJumpFromLogSpot`): jumping up (25) when told, back into the water (speed 6) at
//!   `flags` 12. Splashes (`EffectSsGSplash`) and `NA_SE_EV_DIVE_INTO_WATER_L` /
//!   `NA_SE_EV_BOMB_DROP_WATER` as they cross the surface; blinking; the butterfly's path round
//!   the frog (`EnFr_ButterflyPath`). Already grown frogs (their song played before) start at
//!   270.
//!
//! The whole overlay is ported (the debug display aside: `BREG(0)` 0). Drawn as `EnFr_Draw` does:
//! the frog's skeleton (`object_fr_Skel_00B498`) in its colour (`gDPSetEnvColor`, a dynamic env
//! colour on segment 0x0E run first), both eye segments (8, 9) its eye
//! (`object_fr_Tex_0059A0` blinking, `_005BA0` open), limbs 7 and 8 facing the camera
//! (`EnFr_OverrideLimbDraw`, `EnFr_PostLimbDraw`: their bones given the billboard's rotation),
//! from baked meshes (docs/adr/0012-actor-bakes.md); its point light at the butterfly (radius 95
//! while the butterfly shows, else -1: `Lights_PointNoGlowSetInfo` in the draw, here
//! `draw_update`); the butterfly (`gButterflySkel`, En_Butte's bake, 0.015) while lit. The
//! butterfly's env colour is the frog's here as in the C (the bake's materials decide whether
//! that shows).
//!
//! Params: the whole params (0 the song spot; 1 to 5 the yellow, blue, red, purple and white
//! frogs; any other is the C's `ASSERT`, killed here).
//! Flags: reads EVENTCHKINF_SONGS_FOR_FROGS_ZL, _EPONA, _SARIA, _SUNS, _SOT, _STORMS (the spot:
//! all six start the song game; the frogs: theirs played, grown) and EVENTCHKINF_SONGS_FOR_FROGS_CHOIR
//! (the song game's notes random); sets each as its reward is given the first time (through
//! `sFrogSongIndexToEventChkInfSongsForFrogsMask`).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{approach_f, cos_s, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_NPC, ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2, audio_play_actor_sfx_at};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind, ObjectNeed, TextLine};
use oot_game::audio::ocarina::{OCARINA_BTN_A, OCARINA_BTN_C_DOWN, OCARINA_BTN_C_LEFT, OCARINA_BTN_C_RIGHT, OCARINA_BTN_C_UP, OCARINA_INSTRUMENT_OFF};
use oot_game::audio::sfx::{NA_SE_EV_BOMB_DROP_WATER, NA_SE_SY_CORRECT_CHIME, NA_SE_SY_OCARINA_ERROR, NA_SE_SY_TRE_BOX_APPEAR, SfxPos};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::event_chk_inf;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::{GI_HEART_PIECE, GI_NONE, GI_RUPEE_BLUE, GI_RUPEE_PURPLE};
use oot_game::lights::{LightInfo, LightNode};
use oot_game::message::{MSGMODE_FROGS_PLAYING, MSGMODE_FROGS_START, MSGMODE_FROGS_WAITING, MSGMODE_OCARINA_PLAYING, MSGMODE_PAUSED, OCARINA_ACTION_CHECK_NOWARP, OCARINA_ACTION_FROGS, OCARINA_MODE_00, OCARINA_MODE_01, OCARINA_MODE_04, OCARINA_MODE_08, TEXT_STATE_DONE, TEXT_STATE_EVENT};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};
use oot_game::sys_matrix::MtxF;

use crate::player::STATE2_25;

/// `ACTOR_EN_FR` (`actor_table.h`: 0x00ED).
pub const ACTOR_EN_FR: i16 = 0x00ED;
/// `OBJECT_FR` (`object_table.h`: 0x00E6).
pub const OBJECT: &str = "object_fr";
/// `OBJECT_GAMEPLAY_FIELD_KEEP` (`object_table.h`: 0x0002): the butterfly's.
pub const OBJECT_GAMEPLAY_FIELD_KEEP: i16 = 0x0002;
pub const FIELD_KEEP: &str = "gameplay_field_keep";

/// `object_fr_Skel_00B498` (24 limbs with the root's, `SkelAnime_InitFlex`).
pub const SKEL: &str = "object_fr_Skel_00B498";
/// The jump (out of the water, up, back in).
pub const ANIM_JUMP: &str = "object_fr_Anim_0007BC";
/// The landing after a jump up.
pub const ANIM_LAND: &str = "object_fr_Anim_0011C0";
/// Sitting (and under water).
pub const ANIM_IDLE: &str = "object_fr_Anim_001534";
/// `eyeTextures`: blinking, open.
pub const EYE_TEXTURES: [&str; 2] = ["object_fr_Tex_0059A0", "object_fr_Tex_005BA0"];

/// `En_Fr_Profile`: `ACTORCAT_NPC`, `FLAGS` (`ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY |
/// ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA`), `OBJECT_FR`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_FR,
    name: "En_Fr",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: OBJECT,
    schema: &SCHEMA,
};

/// The whole params: the song spot or a frog (`frogIndex = params - 1`).
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[
        (0, "the song spot (where Link plays; runs the frogs)"),
        (1, "the yellow frog (middle)"),
        (2, "the blue frog (front left)"),
        (3, "the red frog (front right)"),
        (4, "the purple frog (back left; seen under water)"),
        (5, "the white frog (back right)"),
    ]),
    "the song spot or which frog (6 and up: the C's ASSERT)",
);

/// `EVENTCHKINF_SONGS_FOR_FROGS_*` (`save.h`).
pub const EVENTCHKINF_SONGS_FOR_FROGS_CHOIR: u16 = 0xD0;
pub const EVENTCHKINF_SONGS_FOR_FROGS_ZL: u16 = 0xD1;
pub const EVENTCHKINF_SONGS_FOR_FROGS_EPONA: u16 = 0xD2;
pub const EVENTCHKINF_SONGS_FOR_FROGS_SUNS: u16 = 0xD3;
pub const EVENTCHKINF_SONGS_FOR_FROGS_SARIA: u16 = 0xD4;
pub const EVENTCHKINF_SONGS_FOR_FROGS_SOT: u16 = 0xD5;
pub const EVENTCHKINF_SONGS_FOR_FROGS_STORMS: u16 = 0xD6;

const SONG_SETS: &str = "set through sFrogSongIndexToEventChkInfSongsForFrogsMask (FROG_SET_SONG_PLAYED)";

/// En_Fr's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_SONGS_FOR_FROGS_ZL)).reads().sets().note("Zelda's Lullaby played to the frogs (the red frog grown; a purple rupee the first time)").indirect(SONG_SETS),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_SONGS_FOR_FROGS_EPONA)).reads().sets().note("Epona's Song played to them (the white frog grown)").indirect(SONG_SETS),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_SONGS_FOR_FROGS_SARIA)).reads().sets().note("Saria's Song played to them (the yellow frog grown)").indirect(SONG_SETS),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_SONGS_FOR_FROGS_SUNS)).reads().sets().note("the Sun's Song played to them (the blue frog grown)").indirect(SONG_SETS),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_SONGS_FOR_FROGS_SOT)).reads().sets().note("the Song of Time played to them (the purple frog grown)").indirect(SONG_SETS),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_SONGS_FOR_FROGS_STORMS)).reads().sets().note("the Song of Storms played to them (its piece of heart)").indirect(SONG_SETS),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_SONGS_FOR_FROGS_CHOIR)).reads().sets().note("the song game won (its piece of heart); after it the game's notes are random").indirect(SONG_SETS),
    ],
    todo: false,
    // The frogs' Object_GetSlot(OBJECT_GAMEPLAY_FIELD_KEEP): their butterfly; without it, killed.
    objects: &[ObjectNeed::new(FIELD_KEEP).when("params", &[1, 2, 3, 4, 5]).note("the butterfly over the frog with the next note")],
    texts: &[
        TextLine::new(0x40AB).when("params", &[0]).cond("the ocarina out on the spot, all six child songs played to the frogs").note("we're all huge frogs! make us jump to eat the bugs above us (the song game)"),
        TextLine::new(0x40AA).when("params", &[0]).cond("the Song of Storms played to them").note("that melody is so cool! a token of our gratitude (a piece of heart the first time, else a blue rupee)"),
        TextLine::new(0x40A9).when("params", &[0]).cond("Zelda's Lullaby, Epona's, Saria's, the Sun's Song or the Song of Time played to them").note("you play the ocarina well; take rupees as a souvenir (purple the first time, else blue)"),
        TextLine::new(0x40AC).when("params", &[0]).cond("the song game's 14 notes played").note("you did grrreat! we are stuffed (a piece of heart the first time, else a purple rupee); also the spot's textId"),
    ],
};

/// `FrogType`: the frogs' indices (`params - 1`).
pub const FROG_YELLOW: usize = 0;
pub const FROG_BLUE: usize = 1;
pub const FROG_RED: usize = 2;
pub const FROG_PURPLE: usize = 3;
pub const FROG_WHITE: usize = 4;

/// `FrogSongType`.
pub const FROG_ZL: u8 = 0;
pub const FROG_EPONA: u8 = 1;
pub const FROG_SARIA: u8 = 2;
pub const FROG_SUNS: u8 = 3;
pub const FROG_SOT: u8 = 4;
pub const FROG_STORMS: u8 = 5;
pub const FROG_CHOIR_SONG: u8 = 6;
pub const FROG_NO_SONG: u8 = 7;

/// `OCARINA_MODE_05`, `_06`, `_07`, `_09`, `_0A` (`ocarina.h`): the song played (Saria's,
/// Epona's, Zelda's Lullaby, the Song of Time, the Song of Storms; `_08` the Sun's).
pub const OCARINA_MODE_05: u16 = 0x05;
pub const OCARINA_MODE_06: u16 = 0x06;
pub const OCARINA_MODE_07: u16 = 0x07;
pub const OCARINA_MODE_09: u16 = 0x09;
pub const OCARINA_MODE_0A: u16 = 0x0A;
/// `ATTENTION_RANGE_2` (`actor.h`).
const ATTENTION_RANGE_2: u8 = 2;

/// `NA_SE_EV_FROG_JUMP` (`sfx/environmentbank_table.h`: 0x28B1).
pub const NA_SE_EV_FROG_JUMP: u16 = 0x28B1;
/// `NA_SE_EV_DIVE_INTO_WATER_L` (0x28C5).
pub const NA_SE_EV_DIVE_INTO_WATER_L: u16 = 0x28C5;
/// `NA_SE_EV_FROG_GROW_UP` (0x28CC).
pub const NA_SE_EV_FROG_GROW_UP: u16 = 0x28CC;
/// `NA_SE_EV_FROG_CRY_0` (0x28E3).
pub const NA_SE_EV_FROG_CRY_0: u16 = 0x28E3;
/// `NA_SE_EN_DODO_M_EAT` (`sfx/enemybank_table.h`: 0x3826).
pub const NA_SE_EN_DODO_M_EAT: u16 = 0x3826;

/// `sFrogSongIndexToEventChkInfSongsForFrogsMask`, as the flags (0: `FROG_NO_SONG`, none).
pub const S_FROG_SONG_INDEX_TO_EVENTCHKINF: [u16; 8] = [
    EVENTCHKINF_SONGS_FOR_FROGS_ZL,
    EVENTCHKINF_SONGS_FOR_FROGS_EPONA,
    EVENTCHKINF_SONGS_FOR_FROGS_SARIA,
    EVENTCHKINF_SONGS_FOR_FROGS_SUNS,
    EVENTCHKINF_SONGS_FOR_FROGS_SOT,
    EVENTCHKINF_SONGS_FOR_FROGS_STORMS,
    EVENTCHKINF_SONGS_FOR_FROGS_CHOIR,
    0,
];
/// `sFrogToFrogSongIndex`.
pub const S_FROG_TO_FROG_SONG_INDEX: [u8; 5] = [FROG_SARIA, FROG_SUNS, FROG_SOT, FROG_ZL, FROG_EPONA];
/// `sSongToFrog`.
pub const S_SONG_TO_FROG: [usize; 5] = [FROG_PURPLE, FROG_WHITE, FROG_YELLOW, FROG_BLUE, FROG_RED];
/// `sEnFrColor`.
pub const S_EN_FR_COLOR: [[u8; 4]; 5] = [[200, 170, 0, 255], [0, 170, 200, 255], [210, 120, 100, 255], [120, 130, 230, 255], [190, 190, 190, 255]];
/// `sLogSpotToFromWater`: (`xzDist`, `yaw`, `yDist`) by params.
pub const S_LOG_SPOT_TO_FROM_WATER: [(f32, f32, f32); 6] = [
    (0.0, 0.0, 0.0),
    (80.0, -0.375 * std::f32::consts::PI, -80.0),
    (80.0, -0.5 * std::f32::consts::PI, -80.0),
    (80.0, -0.25 * std::f32::consts::PI, -80.0),
    (80.0, 0.875 * std::f32::consts::PI, -80.0),
    (80.0, 0.5 * std::f32::consts::PI, -80.0),
];
/// `sTimerFrogSong`: the frames for the song game's next note (the first four longer).
pub const S_TIMER_FROG_SONG: [i16; 5] = [40, 20, 15, 12, 12];
/// `sTimerJumpingOutOfWater`: the `flags` at which each frog jumps out.
pub const S_TIMER_JUMPING_OUT_OF_WATER: [u8; 5] = [5, 1, 7, 3, 9];
/// `sGrowingScale`.
pub const S_GROWING_SCALE: [f32; 4] = [180.0, 210.0, 240.0, 270.0];
/// `sSmallFrogNotes`, `sLargeFrogNotes`: each frog's jump, transposed.
pub const S_SMALL_FROG_NOTES: [u8; 5] = [5, 2, 9, 11, 14];
pub const S_LARGE_FROG_NOTES: [i8; 5] = [-7, -10, -3, -1, 2];
/// `sJumpOrder`.
pub const S_JUMP_ORDER: [usize; 8] = [FROG_BLUE, FROG_YELLOW, FROG_RED, FROG_PURPLE, FROG_WHITE, FROG_BLUE, FROG_YELLOW, FROG_RED];
/// `sOcarinaNotes`: the song game's random notes.
pub const S_OCARINA_NOTES: [u8; 5] = [OCARINA_BTN_A, OCARINA_BTN_C_DOWN, OCARINA_BTN_C_RIGHT, OCARINA_BTN_C_LEFT, OCARINA_BTN_C_UP];

/// The eye segments (`gSPSegment(POLY_OPA_DISP++, 0x08 / 0x09, eyeTextures[eyeTexIndex])`).
const SEG_EYE_1: u8 = 0x08;
const SEG_EYE_2: u8 = 0x09;
/// The frog's `gDPSetEnvColor(sEnFrColor[frogIndex])` before the skeleton, as a dynamic colour run
/// first.
const SEG_COLOR: u8 = 0x0E;
/// `EnFr_OverrideLimbDraw`'s limbs 7 and 8 (1-based), drawn facing the camera.
pub const BILLBOARD_LIMBS: [usize; 2] = [7, 8];

/// The bake drawn with eye `eye` (`eyeTexIndex`).
pub fn bake_name(eye: usize) -> String {
    format!("En_Fr/eye{eye}")
}

/// `EnFr_Draw`'s meshes: the skeleton after `Gfx_SetupDL_25Opa`, its colour first, with each eye.
pub fn bakes() -> Vec<MeshBake> {
    (0..EYE_TEXTURES.len())
        .map(|eye| MeshBake {
            name: bake_name(eye),
            object: OBJECT.into(),
            segments: vec![
                (SEG_EYE_1, BakeSegment::Texture { file: OBJECT.into(), symbol: EYE_TEXTURES[eye].into() }),
                (SEG_EYE_2, BakeSegment::Texture { file: OBJECT.into(), symbol: EYE_TEXTURES[eye].into() }),
                (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: false }),
            ],
            prelude: vec![SEG_COLOR],
            // EnFr_OverrideLimbDraw's NULL lists for limbs 7 and 8 are drawn in its PostLimbDraw
            // at the same limb's matrix (rotation replaced): the bake keeps them, the draw turns
            // their bones.
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() },
        })
        .collect()
}

/// The overlay's statics (`PlayState::overlay_static`, by `ACTOR_EN_FR`): `sEnFrPointers`.
#[derive(Debug, Default)]
pub struct Statics {
    /// `sEnFrPointers.flags`: 1 to 11 the frogs coming out one by one, 12 going back in.
    pub flags: u8,
    /// `sEnFrPointers.frogs`, by `frogIndex`.
    pub frogs: [Option<ActorHandle>; 5],
}

pub fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_EN_FR)
}

// shared: Actor_FreeOverlay
/// `Actor_FreeOverlay` (`z_actor.c`, from `Actor_Delete`): when the last En_Fr goes the overlay
/// unloads (`ACTOROVL_ALLOC_NORMAL`) and `sEnFrPointers` is back to zero at the next load. Called
/// from every En_Fr's destroy (the spot has none in the C: `Actor_Delete` frees the overlay all
/// the same).
fn actor_free_overlay(play: &mut PlayState) {
    let others = play.actors.all().into_iter().any(|h| play.actors.actor(h).is_some_and(|a| a.id == ACTOR_EN_FR));
    if !others {
        play.overlay_statics.remove(&ACTOR_EN_FR);
    }
}

/// `RAD_TO_BINANG` (`z_math.h`): truncated (`TRUNCF_BINANG`).
pub fn rad_to_binang(radians: f32) -> i16 {
    (radians * (32768.0 / std::f32::consts::PI)) as i32 as i16
}

// shared: BINANG_TO_RAD_ALT
/// `BINANG_TO_RAD_ALT` (`z_math.h`) of an `int` (`world.rot.y + 0x8000` isn't wrapped):
/// `((f32)binang / (f32)0x8000) * M_PI`.
fn binang_to_rad_alt(binang: i32) -> f32 {
    (binang as f32 / 32768.0) * std::f32::consts::PI
}

/// `Matrix_RotateY(yaw, MTXMODE_NEW)`, `Matrix_MultVec3f(&v, ...)`.
fn rotate_y_mult(yaw: f32, v: Vec3) -> Vec3 {
    let mut m = MtxF::IDENTITY;
    m.rotate_y(yaw);
    m.mult_vec3f(v)
}

/// Which update the actor has (`actor.update`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateFn {
    /// `EnFr_Update`: a frog waiting for `gameplay_field_keep`.
    Update,
    /// `EnFr_UpdateIdle`: the song spot.
    UpdateIdle,
    /// `EnFr_UpdateActive`: a frog.
    UpdateActive,
}

/// `blinkFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlinkFunc {
    /// `EnFr_DecrementBlinkTimer`.
    DecrementBlinkTimer,
    /// `EnFr_DecrementBlinkTimerUpdate`.
    DecrementBlinkTimerUpdate,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `NULL` (a frog before its first update).
    None,
    // The frogs.
    SetupJumpingOutOfWater,
    JumpingOutOfWater,
    OrientOnLogSpot,
    ChooseJumpFromLogSpot,
    JumpingUp,
    JumpingBackIntoWater,
    // The song spot.
    Idle,
    Activate,
    ActivateCheckFrogSong,
    /// `func_80A1BE98`: waiting for all five on the log.
    Func80a1be98,
    ListeningToOcarinaNotes,
    ChildSong,
    ChildSongFirstTime,
    TalkBeforeFrogSong,
    SetupFrogSong,
    ContinueFrogSong,
    PrintTextBox,
    TalkBeforeReward,
    SetReward,
    Deactivate,
    GiveReward,
    SetIdle,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::None => "NULL",
            Action::SetupJumpingOutOfWater => "EnFr_SetupJumpingOutOfWater",
            Action::JumpingOutOfWater => "EnFr_JumpingOutOfWater",
            Action::OrientOnLogSpot => "EnFr_OrientOnLogSpot",
            Action::ChooseJumpFromLogSpot => "EnFr_ChooseJumpFromLogSpot",
            Action::JumpingUp => "EnFr_JumpingUp",
            Action::JumpingBackIntoWater => "EnFr_JumpingBackIntoWater",
            Action::Idle => "EnFr_Idle",
            Action::Activate => "EnFr_Activate",
            Action::ActivateCheckFrogSong => "EnFr_ActivateCheckFrogSong",
            Action::Func80a1be98 => "func_80A1BE98",
            Action::ListeningToOcarinaNotes => "EnFr_ListeningToOcarinaNotes",
            Action::ChildSong => "EnFr_ChildSong",
            Action::ChildSongFirstTime => "EnFr_ChildSongFirstTime",
            Action::TalkBeforeFrogSong => "EnFr_TalkBeforeFrogSong",
            Action::SetupFrogSong => "EnFr_SetupFrogSong",
            Action::ContinueFrogSong => "EnFr_ContinueFrogSong",
            Action::PrintTextBox => "EnFr_PrintTextBox",
            Action::TalkBeforeReward => "EnFr_TalkBeforeReward",
            Action::SetReward => "EnFr_SetReward",
            Action::Deactivate => "EnFr_Deactivate",
            Action::GiveReward => "EnFr_GiveReward",
            Action::SetIdle => "EnFr_SetIdle",
        }
    }
}

/// The frog's three animations.
#[derive(Clone)]
pub struct Anims {
    pub jump: Anim,
    pub land: Anim,
    pub idle: Anim,
}

pub struct EnFr {
    pub actor: Actor,
    pub update_fn: UpdateFn,
    /// `actor.draw != NULL`.
    pub has_draw: bool,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    /// `skelAnimeButterfly`.
    pub skel_butterfly: SkelAnimeStd,
    butterfly_skeleton: Option<Arc<Skeleton>>,
    pub action: Action,
    pub light_node: Option<LightNode>,
    pub light_info: LightInfo,
    /// `requiredObjectSlot`: `gameplay_field_keep`'s.
    pub required_object_slot: Option<usize>,
    pub jump_counter: i32,
    pub blink_timer: i32,
    pub blink_func: BlinkFunc,
    pub eye_tex_index: u8,
    pub is_jumping_up: bool,
    pub is_below_water_surface_current: bool,
    pub is_below_water_surface_previous: bool,
    pub is_deactivating: bool,
    pub is_active: bool,
    pub growing_scale_index: u8,
    pub is_growing: bool,
    pub ocarina_note_index: u8,
    pub ocarina_note: u8,
    pub song_index: u8,
    pub is_jumping_to_frog_song: bool,
    pub frog_song_timer: i16,
    pub reward: i16,
    pub pos_log_spot: Vec3,
    pub xz_dist_to_log_spot: f32,
    /// `unusedFloat`: only ever set to 0.
    pub unused_float: f32,
    pub scale: f32,
    pub is_butterfly_drawn: bool,
    pub xy_angle_butterfly: i16,
    pub pos_butterfly: Vec3,
    pub pos_butterfly_light: Vec3,
}

impl EnFr {
    /// The whole params (the spot 0, a frog 1 to 5).
    pub fn params(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    /// `frogIndex`: `params - 1`.
    fn frog_index(&self) -> usize {
        (self.params() - 1).clamp(0, 4) as usize
    }

    /// `EnFr_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnFr {
            actor,
            update_fn: UpdateFn::Update,
            has_draw: false,
            skel: SkelAnimeStd::init_flex(0, None),
            skeleton: None,
            anims: None,
            skel_butterfly: SkelAnimeStd::init_flex(0, None),
            butterfly_skeleton: None,
            action: Action::None,
            light_node: None,
            light_info: LightInfo::point_no_glow(0, 0, 0, [0, 0, 0], 0),
            required_object_slot: None,
            jump_counter: 0,
            blink_timer: 0,
            blink_func: BlinkFunc::DecrementBlinkTimerUpdate,
            eye_tex_index: 0,
            is_jumping_up: false,
            is_below_water_surface_current: false,
            is_below_water_surface_previous: false,
            is_deactivating: false,
            is_active: false,
            growing_scale_index: 0,
            is_growing: false,
            ocarina_note_index: 0,
            ocarina_note: 0,
            song_index: 0,
            is_jumping_to_frog_song: false,
            frog_song_timer: 0,
            reward: 0,
            pos_log_spot: Vec3::ZERO,
            xz_dist_to_log_spot: 0.0,
            unused_float: 0.0,
            scale: 0.0,
            is_butterfly_drawn: false,
            xy_angle_butterfly: 0,
            pos_butterfly: Vec3::ZERO,
            pos_butterfly_light: Vec3::ZERO,
        };
        if this.params() == 0 {
            // actor.destroy = NULL (the overlay's statics still go with the last En_Fr).
            this.has_draw = false;
            this.update_fn = UpdateFn::UpdateIdle;
            this.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED);
            // Actor_ChangeCategory(.., ACTORCAT_PROP): the spawn files it there.
            this.actor.category = ACTORCAT_PROP;
            this.actor.text_id = 0x40AC;
            this.action = Action::Idle;
        } else {
            if this.params() >= 6 || this.params() < 0 {
                // schema: printed
                log::error!("../z_en_fr.c[370] : The argument is wrong!! ({})", this.actor.params);
                this.actor.kill();
                return Box::new(this);
            }
            this.required_object_slot = play.object_ctx.get_index(OBJECT_GAMEPLAY_FIELD_KEEP);
            if this.required_object_slot.is_none() {
                this.actor.kill();
                log::error!("../z_en_fr.c[380] : There is no bank!!");
            }
        }
        Box::new(this)
    }

    /// `EnFr_OrientUnderwater`: under the water, `xzDist` out from the log spot at its yaw,
    /// `yDist` below it, facing the log, still.
    fn orient_underwater(&mut self) {
        let (xz_dist, yaw, y_dist) = S_LOG_SPOT_TO_FROM_WATER[self.params() as usize];
        self.xz_dist_to_log_spot = xz_dist;
        let vec2 = rotate_y_mult(yaw, Vec3::new(0.0, 0.0, xz_dist));
        self.actor.world_pos.x = self.pos_log_spot.x + vec2.x;
        self.actor.world_pos.z = self.pos_log_spot.z + vec2.z;
        self.actor.world_pos.y = y_dist + self.pos_log_spot.y;
        let rot = (rad_to_binang(yaw) as i32 + 0x8000) as i16;
        self.actor.world_rot.y = rot;
        self.actor.shape_rot.y = rot;
        self.actor.speed_xz = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.gravity = 0.0;
    }

    /// `EnFr_DrawIdle`: only the purple frog drawn while the ocarina isn't out on the spot.
    fn draw_idle(&mut self) {
        self.has_draw = self.frog_index() == FROG_PURPLE;
    }

    /// `EnFr_DrawActive`.
    fn draw_active(&mut self) {
        self.has_draw = true;
    }

    /// `EnFr_Update`: a frog's setup once `gameplay_field_keep` is loaded (in its slot,
    /// `sEnFrPointers.frogs`), then `EnFr_UpdateActive` from the next frame.
    fn update_setup(&mut self, play: &mut PlayState) {
        let Some(slot) = self.required_object_slot else { return };
        if !play.object_ctx.is_loaded(slot) {
            return;
        }
        self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        let frog_index = self.frog_index();
        let me = play.cur_actor;
        statics(play).frogs[frog_index] = me;
        // sInitChain: attentionRangeType ATTENTION_RANGE_2, lockOnArrowOffset 30.
        self.actor.target_mode = ATTENTION_RANGE_2;
        self.actor.target_arrow_offset = 30.0;
        // SkelAnime_InitFlex(.., object_fr_Skel_00B498, object_fr_Anim_001534, .., 24) and
        // SkelAnime_Init(.., gButterflySkel, gButterflyAnim, .., 8).
        if let Some(a) = play.assets.clone() {
            let load = |file: &str, name: &str| a.animation(file, name).map_err(|e| log::error!("En_Fr: {e:#}")).ok();
            let anims = match (load(OBJECT, ANIM_JUMP), load(OBJECT, ANIM_LAND), load(OBJECT, ANIM_IDLE)) {
                (Some(jump), Some(land), Some(idle)) => Some(Anims { jump, land, idle }),
                _ => None,
            };
            let skeleton = a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Fr: {e:#}")).ok();
            self.skel = SkelAnimeStd::init_flex(skeleton.as_ref().map_or(23, |s| s.limbs.len()), anims.as_ref().map(|x| x.idle.clone()));
            self.skeleton = skeleton;
            self.anims = anims;
            let butterfly = a.skeleton(FIELD_KEEP, "gButterflySkel").map_err(|e| log::error!("En_Fr: {e:#}")).ok();
            let butterfly_anim = load(FIELD_KEEP, "gButterflyAnim");
            self.skel_butterfly = SkelAnimeStd::init_flex(butterfly.as_ref().map_or(7, |s| s.limbs.len()), butterfly_anim);
            self.butterfly_skeleton = butterfly;
        }
        // When playing the song for the HP, the frog with the next note and the butterfly turns
        // on its light source.
        let h = self.actor.home_pos;
        self.light_info = LightInfo::point_no_glow(h.x as i32 as i16, h.y as i32 as i16, h.z as i32 as i16, [255, 255, 255], -1);
        self.light_node = play.light_ctx.insert_light(self.light_info);
        // The frog's song played before: grown.
        let song = S_FROG_TO_FROG_SONG_INDEX[frog_index];
        self.scale = if frog_has_song_been_played(play, song) { 270.0 } else { 150.0 };
        // Under water the frogs are a ten-thousandth of their scale.
        self.actor.scale = Vec3::splat(self.scale * 0.0001);
        self.actor.min_velocity_y = -9999.0;
        self.actor.set_focus(10.0);
        self.eye_tex_index = 1;
        self.blink_timer = (play.rand.zero_float(60.0) + 20.0) as i32 as i16 as i32;
        self.blink_func = BlinkFunc::DecrementBlinkTimerUpdate;
        self.is_below_water_surface_current = false;
        self.is_below_water_surface_previous = false;
        self.is_jumping_up = false;
        self.pos_log_spot = self.actor.world_pos;
        self.action = Action::SetupJumpingOutOfWater;
        self.is_deactivating = false;
        self.growing_scale_index = 0;
        self.is_active = false;
        self.is_jumping_to_frog_song = false;
        self.song_index = FROG_NO_SONG;
        // unusedButterflyActor = NULL.
        self.orient_underwater();
        self.draw_idle();
        self.update_fn = UpdateFn::UpdateActive;
        self.is_butterfly_drawn = false;
        self.xy_angle_butterfly = (0x1000 * (play.rand.zero_float(255.0) as i32 as i16 as i32)) as i16;
        let p = Vec3::new(self.pos_log_spot.x, self.pos_log_spot.y + 50.0, self.pos_log_spot.z);
        self.pos_butterfly = p;
        self.pos_butterfly_light = p;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
    }

    /// `EnFr_IsDivingIntoWater`: above or below the water's surface, this frame and last.
    fn is_diving_into_water(&mut self, play: &PlayState) {
        let p = self.actor.world_pos;
        if let Some(water_surface) = play.col.water_surface(p.x, p.z, play.col.water_room) {
            self.is_below_water_surface_previous = self.is_below_water_surface_current;
            self.is_below_water_surface_current = self.actor.world_pos.y <= water_surface;
        }
    }

    /// `EnFr_DivingIntoWater`: crossing the surface, a splash 10 below, and the dive's sound (out)
    /// or the drop's (in).
    fn diving_into_water(&self, play: &mut PlayState) {
        if self.is_below_water_surface_current != self.is_below_water_surface_previous {
            let vec = Vec3::new(self.actor.world_pos.x, self.actor.world_pos.y - 10.0, self.actor.world_pos.z);
            play.with_ss(|s| s.g_splash_spawn(vec, None, None, 1, 1));
            if !self.is_below_water_surface_current {
                audio_play_actor_sfx2(play, NA_SE_EV_DIVE_INTO_WATER_L);
            } else {
                audio_play_actor_sfx2(play, NA_SE_EV_BOMB_DROP_WATER);
            }
        }
    }

    /// `EnFr_IsBelowLogSpot`: the height over the log spot; falling onto it, put on it.
    fn is_below_log_spot(&mut self, y_dist_to_log_spot: &mut f32) -> bool {
        *y_dist_to_log_spot = self.actor.world_pos.y - self.pos_log_spot.y;
        if self.actor.velocity.y < 0.0 && *y_dist_to_log_spot <= 0.0 {
            self.actor.velocity.y = 0.0;
            self.actor.world_pos.y = self.pos_log_spot.y;
            true
        } else {
            false
        }
    }

    /// `EnFr_DecrementBlinkTimer`.
    fn decrement_blink_timer(&mut self) {
        if self.blink_timer != 0 {
            self.blink_timer -= 1;
        } else {
            self.blink_func = BlinkFunc::DecrementBlinkTimerUpdate;
        }
    }

    /// `EnFr_DecrementBlinkTimerUpdate`: open, then a blink of one frame, 20 to 79 frames apart.
    fn decrement_blink_timer_update(&mut self, play: &mut PlayState) {
        if self.blink_timer != 0 {
            self.blink_timer -= 1;
        } else if self.eye_tex_index != 0 {
            self.eye_tex_index = 0;
            self.blink_timer = (play.rand.zero_float(60.0) + 20.0) as i32 as i16 as i32;
            self.blink_func = BlinkFunc::DecrementBlinkTimer;
        } else {
            self.eye_tex_index = 1;
            self.blink_timer = 1;
        }
    }

    /// `Animation_Change(&skelAnime, anim, 1.0f, 0.0f, Animation_GetLastFrame(anim), mode, 0.0f)`.
    fn change_anim(&mut self, f: impl Fn(&Anims) -> &Anim, mode: u8) {
        if let Some(a) = self.anims.as_ref().map(|x| f(x).clone()) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, mode, 0.0);
        }
    }

    /// `EnFr_SetupJumpingOutOfWater`: at its turn of `flags`, the jump, drawn.
    fn setup_jumping_out_of_water(&mut self, play: &mut PlayState) {
        if statics(play).flags == S_TIMER_JUMPING_OUT_OF_WATER[self.frog_index()] {
            self.change_anim(|a| &a.jump, ANIMMODE_ONCE);
            self.draw_active();
            self.action = Action::JumpingOutOfWater;
        }
    }

    /// `EnFr_JumpingOutOfWater`: at frame 3, 47 up; at frame 6 the animation held and the next
    /// frog's turn; in, 10 a frame, to the log spot; on it, turning to Link.
    ///
    /// @bug (game): `flags` is bumped every frame the animation holds frame 6 (`playSpeed` 0 until
    /// the frog falls within 40 of its spot: four frames), not once; the frogs come out
    /// overlapping, each catching its `sTimerJumpingOutOfWater` value as it goes by, and `flags`
    /// runs on past 12 (26 with the river's update order) while the spot goes on at 11.
    fn jumping_out_of_water(&mut self, play: &mut PlayState) {
        if self.skel.cur_frame == 6.0 {
            let s = statics(play);
            s.flags = s.flags.wrapping_add(1);
            self.skel.play_speed = 0.0;
        } else if self.skel.cur_frame == 3.0 {
            self.actor.gravity = -10.0;
            self.actor.speed_xz = 0.0;
            self.actor.velocity.y = 47.0;
        }
        let mut vec2 = rotate_y_mult(binang_to_rad_alt(self.actor.world_rot.y as i32 + 0x8000), Vec3::new(0.0, 0.0, self.xz_dist_to_log_spot));
        self.actor.world_pos.x = self.pos_log_spot.x + vec2.x;
        self.actor.world_pos.z = self.pos_log_spot.z + vec2.z;
        if self.skel.cur_frame >= 3.0 {
            approach_f(&mut self.xz_dist_to_log_spot, 0.0, 1.0, 10.0);
        }
        if self.is_below_log_spot(&mut vec2.y) {
            self.actor.gravity = 0.0;
            self.action = Action::OrientOnLogSpot;
            self.unused_float = 0.0;
        }
        if self.actor.velocity.y <= 0.0 && vec2.y < 40.0 {
            self.skel.play_speed = 1.0;
        }
    }

    /// `EnFr_OrientOnLogSpot`: turned to Link and the landing done, the next frog's turn, and it
    /// sits.
    fn orient_on_log_spot(&mut self, play: &mut PlayState) {
        let rot_y_remaining = smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 2, 10000, 100);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        if rot_y_remaining == 0 && self.skel.cur_frame == self.skel.end_frame {
            let s = statics(play);
            s.flags = s.flags.wrapping_add(1);
            self.action = Action::ChooseJumpFromLogSpot;
            self.change_anim(|a| &a.idle, ANIMMODE_LOOP);
        }
    }

    /// `EnFr_ChooseJumpFromLogSpot`: sitting; `flags` 12, back into the water; told to, a jump up.
    fn choose_jump_from_log_spot(&mut self, play: &mut PlayState) {
        if statics(play).flags == 12 {
            self.actor.world_rot.y = rad_to_binang(S_LOG_SPOT_TO_FROM_WATER[self.params() as usize].1);
            self.change_anim(|a| &a.jump, ANIMMODE_ONCE);
            self.action = Action::JumpingBackIntoWater;
        } else if self.is_jumping_up {
            self.change_anim(|a| &a.jump, ANIMMODE_ONCE);
            self.action = Action::JumpingUp;
        }
    }

    /// `EnFr_JumpingUp`: 25 up at frame 3 (the choir's `NA_SE_EN_DODO_M_EAT`), held at 6, back
    /// on the log spot with the landing.
    fn jumping_up(&mut self, play: &mut PlayState) {
        let mut y_dist_to_log_spot = 0.0;
        if self.skel.cur_frame == 6.0 {
            self.skel.play_speed = 0.0;
        } else if self.skel.cur_frame == 3.0 {
            self.actor.gravity = -10.0;
            self.actor.velocity.y = 25.0;
            if self.is_jumping_to_frog_song {
                self.is_jumping_to_frog_song = false;
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_EAT);
            }
        }
        if self.is_below_log_spot(&mut y_dist_to_log_spot) {
            self.is_jumping_up = false;
            self.actor.gravity = 0.0;
            self.change_anim(|a| &a.land, ANIMMODE_LOOP);
            self.action = Action::ChooseJumpFromLogSpot;
        } else if self.actor.velocity.y <= 0.0 && y_dist_to_log_spot < 40.0 {
            self.skel.play_speed = 1.0;
        }
    }

    /// `EnFr_JumpingBackIntoWater`: turning away, at frame 3 off at 6 and 25 up; below the
    /// water's depth, under again (`isDeactivating`).
    fn jumping_back_into_water(&mut self) {
        let y_underwater = S_LOG_SPOT_TO_FROM_WATER[self.params() as usize].2 + self.pos_log_spot.y;
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.world_rot.y, 2, 10000, 100);
        if self.skel.cur_frame == 6.0 {
            self.skel.play_speed = 0.0;
        } else if self.skel.cur_frame == 3.0 {
            self.actor.speed_xz = 6.0;
            self.actor.gravity = -10.0;
            self.actor.velocity.y = 25.0;
        }
        // Final spot reached.
        if self.actor.velocity.y < 0.0 && self.actor.world_pos.y < y_underwater {
            self.change_anim(|a| &a.idle, ANIMMODE_LOOP);
            self.action = Action::SetupJumpingOutOfWater;
            self.draw_idle();
            self.is_deactivating = true;
            self.orient_underwater();
        }
    }

    /// `EnFr_SetScaleActive`: growing, 25 at most a frame, to the next of `sGrowingScale`, then
    /// back to 150, four times; at 270 it's done.
    fn set_scale_active(&mut self) {
        let i = self.growing_scale_index as usize;
        if !self.is_growing {
            approach_f(&mut self.scale, S_GROWING_SCALE[i], 2.0, 25.0);
            if self.scale >= S_GROWING_SCALE[i] {
                self.scale = S_GROWING_SCALE[i];
                if self.growing_scale_index < 3 {
                    self.is_growing = true;
                } else {
                    self.is_jumping_up = false;
                    self.is_active = false;
                }
            }
        } else {
            approach_f(&mut self.scale, 150.0, 2.0, 25.0);
            if self.scale <= 150.0 {
                self.scale = 150.0;
                self.growing_scale_index += 1;
                if self.growing_scale_index >= 4 {
                    self.growing_scale_index = 3;
                }
                self.is_growing = false;
            }
        }
    }

    /// `EnFr_ButterflyPath`: the butterfly 25 ahead of the frog, swaying 5 across and 2 up and
    /// down, 50 over the log spot; its light 15 under and 20 ahead of it.
    fn butterfly_path(&mut self) {
        let rot_y = self.actor.shape_rot.y;
        self.xy_angle_butterfly = self.xy_angle_butterfly.wrapping_add(0x1000);
        let w = self.actor.world_rot;
        let mut m = MtxF::set_translate(self.actor.world_pos.x, self.actor.world_pos.y, self.actor.world_pos.z);
        m.rotate_zyx(w.x, w.y, w.z);
        let vec2 = m.mult_vec3f(Vec3::new(0.0, 0.0, 25.0));
        let sin = sin_s((self.xy_angle_butterfly as i32 * 2) as i16) * 5.0;
        self.pos_butterfly.x = (sin_s(rot_y) * sin) + vec2.x;
        self.pos_butterfly.y = (2.0 * cos_s(self.xy_angle_butterfly)) + (self.pos_log_spot.y + 50.0);
        self.pos_butterfly.z = (cos_s(rot_y) * sin) + vec2.z;
        let mut m = MtxF::set_translate(self.pos_butterfly.x, self.pos_butterfly.y, self.pos_butterfly.z);
        m.rotate_zyx(w.x, w.y, w.z);
        self.pos_butterfly_light = m.mult_vec3f(Vec3::new(0.0, -15.0, 20.0));
    }

    /// `EnFr_UpdateActive`: growing, or the frog's blink, action, the water, its animations, the
    /// butterfly's path, and its move.
    fn update_active(&mut self, play: &mut PlayState) {
        self.jump_counter = self.jump_counter.wrapping_add(1);
        self.actor.scale = Vec3::splat(self.scale * 0.0001);
        if self.is_active {
            self.set_scale_active();
        } else {
            self.actor.set_focus(10.0);
            match self.blink_func {
                BlinkFunc::DecrementBlinkTimer => self.decrement_blink_timer(),
                BlinkFunc::DecrementBlinkTimerUpdate => self.decrement_blink_timer_update(play),
            }
            self.run_action(play);
            self.is_diving_into_water(play);
            self.diving_into_water(play);
            self.skel.update();
            self.skel_butterfly.update();
            self.butterfly_path();
            self.actor.move_forward();
        }
    }

    /// `EnFr_Idle` (the spot): Link's ocarina out here (`PLAYER_STATE2_25`), one-point 4110, the
    /// box paused, Link put on the spot facing its way; Link above it within 30, his ocarina
    /// turned to it.
    fn idle(&mut self, play: &mut PlayState) {
        let Some(ph) = play.player else { return };
        let state2 = play.actors.get(ph).and_then(|p| p.as_player()).map_or(0, |p| p.state_flags2());
        if state2 & STATE2_25 != 0 {
            if play.msg_ctx.ocarina_mode == OCARINA_MODE_04 {
                play.msg_ctx.ocarina_mode = OCARINA_MODE_00;
            }
            let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
            play.onepoint_cutscene_init(4110, -99, me, CAM_ID_MAIN);
            play.msg_ctx.msg_mode = MSGMODE_PAUSED;
            let rot = self.actor.world_rot.y;
            if let Some(a) = play.actors.actor_mut(ph) {
                // x = 990.0f, y = 205.0f, z = -1220.0f.
                a.world_pos = self.actor.world_pos;
                a.world_rot.y = rot;
                a.shape_rot.y = rot;
            }
            if let Some(p) = play.actors.get_mut(ph).and_then(|p| p.as_player_mut()) {
                p.set_yaw(rot);
            }
            self.reward = GI_NONE;
            self.action = Action::Activate;
        } else if play.actors.actor(ph).is_some_and(|p| is_above_and_within_30_dist_xz(p.world_pos, self.actor.world_pos)) {
            let me = play.cur_actor;
            if let Some(p) = play.actors.get_mut(ph).and_then(|p| p.as_player_mut()) {
                p.set_unk_6a8(me);
            }
        }
    }

    /// `EnFr_Activate`: the frogs out (`flags` 1) once the box is paused (or playing).
    fn activate(&mut self, play: &mut PlayState) {
        if play.msg_ctx.msg_mode == MSGMODE_OCARINA_PLAYING {
            play.msg_ctx.msg_mode = MSGMODE_PAUSED;
            statics(play).flags = 1;
            self.action = Action::ActivateCheckFrogSong;
        } else if play.msg_ctx.msg_mode == MSGMODE_PAUSED {
            // Goes to the frogs' song for the heart piece.
            statics(play).flags = 1;
            self.action = Action::ActivateCheckFrogSong;
        }
    }

    /// `EnFr_ActivateCheckFrogSong`: all five out (`flags` 11): all six child songs played, the
    /// song game (0x40AB); else the free play.
    fn activate_check_frog_song(&mut self, play: &mut PlayState) {
        if statics(play).flags == 11 {
            let s = &play.save;
            if s.get_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_ZL)
                && s.get_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_EPONA)
                && s.get_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_SARIA)
                && s.get_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_SUNS)
                && s.get_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_SOT)
                && s.get_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_STORMS)
            {
                self.action = Action::TalkBeforeFrogSong;
                self.song_index = FROG_CHOIR_SONG;
                play.start_textbox(0x40AB, play.cur_actor);
            } else {
                self.song_index = FROG_ZL;
                self.action = Action::Func80a1be98;
            }
        }
    }

    /// `func_80A1BE98`: all five frogs sitting on the log, the free play
    /// (`OCARINA_ACTION_CHECK_NOWARP`).
    fn func_80a1be98(&mut self, play: &mut PlayState) {
        for frog_index in 0..5 {
            if frog_action(play, frog_index) != Some(Action::ChooseJumpFromLogSpot) {
                return;
            }
        }
        play.message_start_ocarina(OCARINA_ACTION_CHECK_NOWARP);
        self.action = Action::ListeningToOcarinaNotes;
    }

    /// `EnFr_ListeningToOcarinaNotes`: a song played (`ocarinaMode`), its jumps; a note, its
    /// frog's jump; the ocarina away, the end.
    fn listening_to_ocarina_notes(&mut self, play: &mut PlayState) {
        self.song_index = FROG_NO_SONG;
        // The ocarina song played.
        match play.msg_ctx.ocarina_mode {
            OCARINA_MODE_07 => self.song_index = FROG_ZL,
            OCARINA_MODE_06 => self.song_index = FROG_EPONA,
            OCARINA_MODE_05 => self.song_index = FROG_SARIA,
            OCARINA_MODE_08 => self.song_index = FROG_SUNS,
            OCARINA_MODE_09 => self.song_index = FROG_SOT,
            OCARINA_MODE_0A => self.song_index = FROG_STORMS,
            OCARINA_MODE_04 => self.ocarina_mistake(play),
            // A note played, no song: its frog jumps.
            OCARINA_MODE_01 => {
                if let Some(frog) = note_frog(play.msg_ctx.last_ocarina_button_index) {
                    self.setup_jumping_up(play, frog);
                }
            }
            _ => {}
        }
        if self.song_index != FROG_NO_SONG {
            self.jump_counter = 0;
            self.action = Action::ChildSong;
        }
    }

    /// `EnFr_SetupJumpingUp`: frog `frog_index` jumps (unless it's jumping), its note
    /// (`NA_SE_EV_FROG_JUMP` transposed: `sLargeFrogNotes` grown to its last scale, else
    /// `sSmallFrogNotes`); in the song game, its jump's sound too.
    fn setup_jumping_up(&self, play: &mut PlayState, frog_index: usize) -> bool {
        let Some(h) = statics(play).frogs[frog_index] else { return false };
        let choir = self.song_index == FROG_CHOIR_SONG;
        let Some(frog) = play.actors.downcast_mut::<EnFr>(h) else { return false };
        if frog.is_jumping_up {
            return false;
        }
        let semitone = if frog.growing_scale_index == 3 { S_LARGE_FROG_NOTES[frog_index] as u8 } else { S_SMALL_FROG_NOTES[frog_index] };
        if choir {
            frog.is_jumping_to_frog_song = true;
        }
        frog.is_jumping_up = true;
        play.audio.play_sfx_transposed(SfxPos::Actor(h), NA_SE_EV_FROG_JUMP, semitone as i8);
        true
    }

    /// `EnFr_ChildSong`: 48 frames of jumps (`sJumpOrder`, every 4); then the Song of Storms'
    /// 0x40AA; a child song the first time, the chime and its frog grows; else 0x40A9.
    fn child_song(&mut self, play: &mut PlayState) {
        if self.jump_counter < 48 {
            if self.jump_counter % 4 == 0 {
                self.setup_jumping_up(play, S_JUMP_ORDER[((self.jump_counter >> 2) & 7) as usize]);
            }
        } else if self.song_index == FROG_STORMS {
            self.actor.text_id = 0x40AA;
            self.setup_reward(play, false);
        } else if !frog_has_song_been_played(play, self.song_index) {
            let frog_index = S_SONG_TO_FROG[self.song_index as usize];
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            // The C reads the frog without a check: a missing one is taken as not on the log.
            if let Some(h) = statics(play).frogs[frog_index]
                && frog_action(play, frog_index) == Some(Action::ChooseJumpFromLogSpot)
            {
                if let Some(frog) = play.actors.downcast_mut::<EnFr>(h) {
                    frog.is_jumping_up = true;
                    frog.is_active = true;
                }
                audio_play_actor_sfx_at(play, h, NA_SE_EV_FROG_GROW_UP);
                self.action = Action::ChildSongFirstTime;
            } else {
                self.jump_counter = 48;
            }
        } else {
            self.actor.text_id = 0x40A9;
            self.setup_reward(play, true);
        }
    }

    /// `EnFr_ChildSongFirstTime`: its frog grown, 0x40A9.
    fn child_song_first_time(&mut self, play: &mut PlayState) {
        let frog_index = S_SONG_TO_FROG[self.song_index as usize];
        let active = statics(play).frogs[frog_index].and_then(|h| play.actors.downcast::<EnFr>(h)).is_some_and(|f| f.is_active);
        if !active {
            self.actor.text_id = 0x40A9;
            self.setup_reward(play, true);
        }
    }

    /// `EnFr_TalkBeforeFrogSong`: 0x40AB read, the game in 2 frames.
    fn talk_before_frog_song(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            play.with_msg(|m, f| m.close_textbox(f.audio));
            self.frog_song_timer = 2;
            self.action = Action::SetupFrogSong;
        }
    }

    /// `EnFr_SetupFrogSong`: the song game (`OCARINA_ACTION_FROGS`), its first note, 40 frames.
    fn setup_frog_song(&mut self, play: &mut PlayState) {
        if self.frog_song_timer != 0 {
            self.frog_song_timer -= 1;
        } else {
            self.frog_song_timer = 40;
            self.ocarina_note_index = 0;
            play.message_start_ocarina(OCARINA_ACTION_FROGS);
            self.ocarina_note = get_next_note_frog_song(play, self.ocarina_note_index);
            check_ocarina_input_frog_song(play, self.ocarina_note);
            self.action = Action::ContinueFrogSong;
        }
    }

    /// `EnFr_IsFrogSongComplete`: the right note, the next (and its time, `sTimerFrogSong`); the
    /// 14th, done.
    fn is_frog_song_complete(&mut self, play: &mut PlayState) -> bool {
        if self.ocarina_note == play.msg_ctx.last_ocarina_button_index {
            self.ocarina_note_index += 1;
            let ocarina_note_index = self.ocarina_note_index;
            if ocarina_note_index >= 14 {
                // The frog song is completed.
                self.ocarina_note_index = 13;
                return true;
            }
            // The first four notes have more frames to receive an input.
            let index = if ocarina_note_index < 4 { ocarina_note_index as usize } else { 4 };
            let ocarina_note = get_next_note_frog_song(play, ocarina_note_index);
            self.ocarina_note = ocarina_note;
            check_ocarina_input_frog_song(play, ocarina_note);
            self.frog_song_timer = S_TIMER_FROG_SONG[index];
        }
        false
    }

    /// `EnFr_OcarinaMistake`: the box closed, no reward, the error, the frogs back in.
    fn ocarina_mistake(&mut self, play: &mut PlayState) {
        play.with_msg(|m, f| m.close_textbox(f.audio));
        self.reward = GI_NONE;
        play.audio.play_sfx_centered(NA_SE_SY_OCARINA_ERROR);
        play.audio.audio_ocarina_set_instrument(OCARINA_INSTRUMENT_OFF);
        statics(play).flags = 12;
        deactivate_butterfly(play);
        self.action = Action::Deactivate;
    }

    /// `EnFr_ContinueFrogSong`: out of time, a mistake; B with all five on the log, a mistake; a
    /// note played (`MSGMODE_FROGS_WAITING`), its frog jumps, the box listens again, and the
    /// note's checked.
    fn continue_frog_song(&mut self, play: &mut PlayState) {
        if self.frog_song_timer == 0 {
            self.ocarina_mistake(play);
            return;
        }
        self.frog_song_timer -= 1;
        if play.msg_ctx.msg_mode == MSGMODE_FROGS_PLAYING {
            let counter = (0..5).filter(|&i| frog_action(play, i) != Some(Action::ChooseJumpFromLogSpot)).count();
            if counter == 0 && play.input.press.held(eng_input::pad::BTN_B) {
                self.ocarina_mistake(play);
                return;
            }
        }
        if play.msg_ctx.msg_mode == MSGMODE_FROGS_WAITING {
            play.msg_ctx.msg_mode = MSGMODE_FROGS_START;
            if let Some(frog) = note_frog(play.msg_ctx.last_ocarina_button_index) {
                self.setup_jumping_up(play, frog);
            }
            if self.is_frog_song_complete(play) {
                self.actor.text_id = 0x40AC;
                self.setup_reward(play, false);
            }
        }
    }

    /// `EnFr_SetupReward`: no butterflies, the treasure's sound (`unkCondition`) or the chime, the
    /// ocarina off, the box paused.
    fn setup_reward(&mut self, play: &mut PlayState, unk_condition: bool) {
        deactivate_butterfly(play);
        play.audio.play_sfx_centered(if unk_condition { NA_SE_SY_TRE_BOX_APPEAR } else { NA_SE_SY_CORRECT_CHIME });
        play.audio.audio_ocarina_set_instrument(OCARINA_INSTRUMENT_OFF);
        play.msg_ctx.msg_mode = MSGMODE_PAUSED;
        self.action = Action::PrintTextBox;
    }

    /// `EnFr_PrintTextBox`: the spot's `textId`.
    fn print_text_box(&mut self, play: &mut PlayState) {
        play.start_textbox(self.actor.text_id, play.cur_actor);
        self.action = Action::TalkBeforeReward;
    }

    /// `EnFr_TalkBeforeReward`.
    fn talk_before_reward(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            self.frog_song_timer = 100;
            play.with_msg(|m, f| m.close_textbox(f.audio));
            self.action = Action::SetReward;
        }
    }

    /// `EnFr_SetReward`: the frogs back in; the reward by the song, and its flag the first time.
    fn set_reward(&mut self, play: &mut PlayState) {
        statics(play).flags = 12;
        let song_index = self.song_index;
        self.action = Action::Deactivate;
        self.reward = GI_NONE;
        if (FROG_ZL..=FROG_SOT).contains(&song_index) {
            if !frog_has_song_been_played(play, song_index) {
                frog_set_song_played(play, song_index);
                self.reward = GI_RUPEE_PURPLE;
            } else {
                self.reward = GI_RUPEE_BLUE;
            }
        } else if song_index == FROG_STORMS {
            if !frog_has_song_been_played(play, song_index) {
                frog_set_song_played(play, song_index);
                self.reward = GI_HEART_PIECE;
            } else {
                self.reward = GI_RUPEE_BLUE;
            }
        } else if song_index == FROG_CHOIR_SONG {
            if !frog_has_song_been_played(play, song_index) {
                frog_set_song_played(play, song_index);
                self.reward = GI_HEART_PIECE;
            } else {
                self.reward = GI_RUPEE_PURPLE;
            }
        }
    }

    /// `EnFr_Deactivate`: all five back under (`isDeactivating`), the ocarina's mode 4, the cry,
    /// and the reward offered (or waiting again).
    fn deactivate(&mut self, play: &mut PlayState) {
        // Originally a separate butterfly actor: its freezeTimer set to 10 here
        // (unusedButterflyActor, always NULL).
        let frogs = statics(play).frogs;
        for h in frogs {
            match h.and_then(|h| play.actors.downcast::<EnFr>(h)) {
                None => {
                    log::error!("../z_en_fr.c[1604] There are no frogs!?");
                    return;
                }
                Some(f) if !f.is_deactivating => return,
                Some(_) => {}
            }
        }
        for h in frogs.into_iter().flatten() {
            if let Some(f) = play.actors.downcast_mut::<EnFr>(h) {
                f.is_deactivating = false;
            }
        }
        play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
        audio_play_actor_sfx2(play, NA_SE_EV_FROG_CRY_0);
        if self.reward == GI_NONE {
            self.action = Action::Idle;
        } else {
            self.action = Action::GiveReward;
            offer_get_item_range(play, &self.actor, self.reward, 30.0, 100.0);
        }
    }

    /// `EnFr_GiveReward`: offered until Link has it.
    fn give_reward(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.action = Action::SetIdle;
        } else {
            offer_get_item_range(play, &self.actor, self.reward, 30.0, 100.0);
        }
    }

    /// `EnFr_SetIdle`: the item's text closed, waiting again.
    fn set_idle(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            self.action = Action::Idle;
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::None => {}
            Action::SetupJumpingOutOfWater => self.setup_jumping_out_of_water(play),
            Action::JumpingOutOfWater => self.jumping_out_of_water(play),
            Action::OrientOnLogSpot => self.orient_on_log_spot(play),
            Action::ChooseJumpFromLogSpot => self.choose_jump_from_log_spot(play),
            Action::JumpingUp => self.jumping_up(play),
            Action::JumpingBackIntoWater => self.jumping_back_into_water(),
            Action::Idle => self.idle(play),
            Action::Activate => self.activate(play),
            Action::ActivateCheckFrogSong => self.activate_check_frog_song(play),
            Action::Func80a1be98 => self.func_80a1be98(play),
            Action::ListeningToOcarinaNotes => self.listening_to_ocarina_notes(play),
            Action::ChildSong => self.child_song(play),
            Action::ChildSongFirstTime => self.child_song_first_time(play),
            Action::TalkBeforeFrogSong => self.talk_before_frog_song(play),
            Action::SetupFrogSong => self.setup_frog_song(play),
            Action::ContinueFrogSong => self.continue_frog_song(play),
            Action::PrintTextBox => self.print_text_box(play),
            Action::TalkBeforeReward => self.talk_before_reward(play),
            Action::SetReward => self.set_reward(play),
            Action::Deactivate => self.deactivate(play),
            Action::GiveReward => self.give_reward(play),
            Action::SetIdle => self.set_idle(play),
        }
    }

    /// `EnFr_UpdateIdle` (the spot; the debug display aside: `BREG(0)` 0).
    fn update_idle(&mut self, play: &mut PlayState) {
        self.jump_counter = self.jump_counter.wrapping_add(1);
        self.run_action(play);
    }
}

/// `EnFr_IsAboveAndWithin30DistXZ`: Link within 30 across the spot, not below it.
pub fn is_above_and_within_30_dist_xz(player: Vec3, spot: Vec3) -> bool {
    let x_dist_to_player = player.x - spot.x;
    let z_dist_to_player = player.z - spot.z;
    let y_dist_to_player = player.y - spot.y;
    (x_dist_to_player * x_dist_to_player + z_dist_to_player * z_dist_to_player) <= 30.0 * 30.0 && y_dist_to_player >= 0.0
}

/// `FROG_HAS_SONG_BEEN_PLAYED(frogSongIndex)`.
fn frog_has_song_been_played(play: &PlayState, frog_song_index: u8) -> bool {
    let flag = S_FROG_SONG_INDEX_TO_EVENTCHKINF[frog_song_index as usize];
    // schema: FROG_HAS_SONG_BEEN_PLAYED's table (FROG_NO_SONG: no flag)
    flag != 0 && play.save.get_event_chk_inf(flag)
}

/// `FROG_SET_SONG_PLAYED(frogSongIndex)`.
fn frog_set_song_played(play: &mut PlayState, frog_song_index: u8) {
    let flag = S_FROG_SONG_INDEX_TO_EVENTCHKINF[frog_song_index as usize];
    if flag != 0 {
        // schema: FROG_SET_SONG_PLAYED's table
        play.save.set_event_chk_inf(flag);
    }
}

/// The frog an ocarina button makes jump (A blue, C-down yellow, C-right red, C-left purple, C-up
/// white).
fn note_frog(button: u8) -> Option<usize> {
    match button {
        OCARINA_BTN_A => Some(FROG_BLUE),
        OCARINA_BTN_C_DOWN => Some(FROG_YELLOW),
        OCARINA_BTN_C_RIGHT => Some(FROG_RED),
        OCARINA_BTN_C_LEFT => Some(FROG_PURPLE),
        OCARINA_BTN_C_UP => Some(FROG_WHITE),
        _ => None,
    }
}

/// `sEnFrPointers.frogs[frog_index]->actionFunc` (`None`: no frog there).
fn frog_action(play: &mut PlayState, frog_index: usize) -> Option<Action> {
    let h = statics(play).frogs[frog_index]?;
    play.actors.downcast::<EnFr>(h).map(|f| f.action)
}

/// `EnFr_CheckOcarinaInputFrogSong`: the butterfly over the frog with the note `ocarina_note`, off
/// the others. @bug (game): `frogIndexButterfly` is left unset for any other button (the song's
/// notes are always one of the five).
fn check_ocarina_input_frog_song(play: &mut PlayState, ocarina_note: u8) {
    let frog_index_butterfly = note_frog(ocarina_note);
    // Turn on or off the butterfly above each frog.
    let frogs = statics(play).frogs;
    for (frog_index, h) in frogs.into_iter().enumerate() {
        if let Some(frog) = h.and_then(|h| play.actors.downcast_mut::<EnFr>(h)) {
            frog.is_butterfly_drawn = Some(frog_index) == frog_index_butterfly;
        }
    }
}

/// `EnFr_DeactivateButterfly`.
fn deactivate_butterfly(play: &mut PlayState) {
    let frogs = statics(play).frogs;
    for h in frogs.into_iter().flatten() {
        if let Some(frog) = play.actors.downcast_mut::<EnFr>(h) {
            frog.is_butterfly_drawn = false;
        }
    }
}

/// `EnFr_GetNextNoteFrogSong`: `gFrogsSongPtr`'s note until the song game's been won, then a
/// random one of the five.
pub fn get_next_note_frog_song(play: &mut PlayState, ocarina_note_index: u8) -> u8 {
    if !play.save.get_event_chk_inf(EVENTCHKINF_SONGS_FOR_FROGS_CHOIR) {
        play.audio.ocarina.frogs_song_notes.get(ocarina_note_index as usize).copied().unwrap_or(0)
    } else {
        S_OCARINA_NOTES[(play.rand.zero_float(60.0) as i32 % 5) as usize]
    }
}

impl ActorImpl for EnFr {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `actor.update`: `EnFr_Update` (a frog waiting for its butterfly's object), `EnFr_UpdateIdle`
    /// (the spot) or `EnFr_UpdateActive` (a frog).
    fn update(&mut self, play: &mut PlayState) {
        match self.update_fn {
            UpdateFn::Update => self.update_setup(play),
            UpdateFn::UpdateIdle => self.update_idle(play),
            UpdateFn::UpdateActive => self.update_active(play),
        }
    }

    /// `EnFr_Draw`'s light: `Lights_PointNoGlowSetInfo` at the butterfly's light, 95 while it
    /// shows, else -1.
    fn draw_update(&mut self, play: &mut PlayState) {
        if !self.has_draw {
            return;
        }
        let light_radius: i16 = if self.is_butterfly_drawn { 95 } else { -1 };
        let p = self.pos_butterfly_light;
        self.light_info = LightInfo::point_no_glow(p.x as i32 as i16, p.y as i32 as i16, p.z as i32 as i16, [255, 255, 255], light_radius);
        play.light_ctx.set_info(self.light_node, self.light_info);
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.has_draw as u32, self.eye_tex_index.min(1) as u32, self.is_butterfly_drawn as u32, self.frog_index() as u32];
        rs.values = vec![self.pos_butterfly.x, self.pos_butterfly.y, self.pos_butterfly.z];
        rs
    }

    /// `EnFr_Draw`: the frog in its colour with its eye, limbs 7 and 8 facing the camera
    /// (`EnFr_PostLimbDraw`: `Matrix_ReplaceRotation(&play->billboardMtxF)`); the butterfly
    /// (`Matrix_Translate(posButterfly)`, `Matrix_Scale(0.015)`, `Matrix_RotateZYX(shape.rot)`)
    /// while lit.
    fn draw(&self, rs: &RenderState, _play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let [drawn, eye, butterfly, frog] = rs.switches.as_slice() else { return };
        if *drawn == 0 {
            return;
        }
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let model = actor_draw_matrix(rs);
        let mut bones = skeleton.pose(joints);
        let inv = model.inverse();
        for limb in BILLBOARD_LIMBS {
            if let Some(b) = bones.get_mut(limb - 1) {
                *b = inv * oot_game::draw::replace_rotation(model * *b, view.billboard);
            }
        }
        let mut sv = SegmentValues::default();
        sv.env[SEG_COLOR as usize] = Some(S_EN_FR_COLOR[(*frog as usize).min(4)]);
        out.opa.push(DrawCmd {
            mesh: MeshKey::named(keys::bake(&bake_name((*eye as usize).min(1)))),
            transform: model,
            bones,
            params: DrawParams { segments: Some(sv), ..Default::default() },
        });
        if *butterfly != 0
            && let Some(bs) = &self.butterfly_skeleton
            && let [x, y, z] = rs.values.as_slice()
        {
            let s = rs.rot;
            let mut r = MtxF::IDENTITY;
            r.rotate_zyx(s[0], s[1], s[2]);
            let m = Mat4::from_translation(Vec3::new(*x, *y, *z)) * Mat4::from_scale(Vec3::splat(0.015)) * r.to_mat4();
            let bones = bs.pose(&eng_anim::anim::JointTable { rot: self.skel_butterfly.joint_table.clone(), face: 0 });
            out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(crate::en_butte::BAKE)), transform: m, bones, params: Default::default() });
        }
    }

    /// `EnFr_Destroy` (the spot has none): the light; then the overlay's unload when it's the
    /// last.
    fn destroy(&mut self, play: &mut PlayState) {
        if self.params() != 0 {
            play.light_ctx.remove_light(self.light_node.take());
        }
        actor_free_overlay(play);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
