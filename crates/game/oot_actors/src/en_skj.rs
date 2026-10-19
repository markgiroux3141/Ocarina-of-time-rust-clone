//! `En_Skj` (`ovl_En_Skj/z_en_skj.c`): the Skull Kids of the Lost Woods (`spot10`), and the two
//! invisible stumps that run what Link plays to them. The type is `PARAMS_GET_U(params, 10, 6)`.
//!
//! - **Type 0** (room 9): the Skull Kid on the stump, playing his flute
//!   (`EnSkj_SariasSongKidIdle`). Until Link has played him Saria's Song (`ITEMGETINF_16`) he
//!   backflips away within 200 of Link (`EnSkj_Backflip`: back at 8, 8 up, fading 20 a frame,
//!   killed at 0). Link on the small stump (type 5's `EnSkj_RangeCheck`: within 26 across, not
//!   below it) makes him wait (`EnSkj_WaitInRange`); the ocarina out (`PLAYER_STATE2_24`) puts Link
//!   on the stump's middle, turns him to the stump's facing (`EnSkj_TurnPlayer`) and starts
//!   `Message_StartOcarinaSunsSongDisabled(OCARINA_ACTION_CHECK_SARIA)` (`EnSkj_WaitForSong`; he
//!   plays along while the song's played back). Saria's Song the first time: the chime, text
//!   0x10BB (`EnSkj_AfterSong`, `EnSkj_SariaSongTalk`), a piece of heart offered within the
//!   stump's reach (`EnSkj_GetItemXzRange`, `_YRange`: `func_80AFFE44`), then `ITEMGETINF_16` as
//!   its text closes (`EnSkj_ChangeModeAfterSong`). Another song before that: 0x1041
//!   (`EnSkj_WrongSong`). After it he talks from the stump (0x10BC; 0x10BD after the song again;
//!   with the Skull Mask sold, `MaskReaction_GetTextId` or 0x1020; wearing it, 0x101B). Talking to
//!   him wearing the Skull Mask before `ITEMGETINF_39`: the trade (`EnSkj_StartMaskTrade`: he jumps
//!   down, walks to Link, 0x101C's choice, 0x101E, then 10 rupees, `ITEMGETINF_39`, the mask taken
//!   (`Player_UnsetMask`), `ITEM_SOLD_OUT`, 0x101F, and he backflips away; no: 0x101D).
//! - **Hit** (`EnSkj_CollisionCheck`, outside his talks): Link's sword (damage reaction 0xF) draws
//!   blood (`EnSkj_SpawnBlood`: `Effect_Add`'s spark, blue) and a hit mark; from behind him (over
//!   0x6000 off his facing) while he plays or looks about, he dies at once; else he takes the
//!   damage (10 health), flashes red, and after the hit (`func_80AFF1F0`) stands to fight
//!   (`EnSkj_Fight`: round his `center`, 120 ahead of where he stood, facing Link), or backflips
//!   away after three hits within 60 frames. Fighting, every 60 frames he shoots three needles 4
//!   frames apart (`EnSkj_ShootNeedle`: `En_Skjneedle`, 24 a frame, gone after 100). The fight
//!   lasts while `battleExitTimer` does (600 from `EnSkj_ResetFight`); then, Link in front or
//!   noisy (`func_80AFEDF8`), he backflips away, else back to his flute. At 0 health he dies
//!   (`gSkullKidDieAnim`, then fading 4 a frame in `EffectSsDeadDb`'s flames), dropping an orange
//!   rupee (`ITEM00_RUPEE_ORANGE`) at the end. Any hit of damage reaction 0 (a nut, a seed, a
//!   spin, ...) makes him backflip away. Type 0's `battleExitTimer` is never set: after a hit he
//!   backflips away (Link in front) or goes back to his flute; only the adult's Skull Kids
//!   (`func_80AFF334`) shoot.
//! - **Types 1 and 2** (room 1): the memory game's Skull Kids (left and right), invisible until
//!   Link stands on the stump (`playerInRange`), then fading in; they play their flutes while
//!   `minigameState` says so, and backflip away when the game ends. (Their `shadowAlpha` is
//!   `ActorShape_Init`'s 255: their shadows show while they're invisible.)
//! - **Type 6** (room 1): the memory game's stump (invisible, `ACTORCAT_PROP`, no draw). Link on
//!   it with the ocarina out: 0x10BE, then `Message_StartOcarina(OCARINA_ACTION_MEMORY_GAME)`
//!   (`EnSkj_StartOcarinaMinigame`), the nature ambience (`func_800F5BF0`). It drives the message
//!   box's memory game (`EnSkj_WaitForPlayback`): the left Skull Kid's notes, then (when
//!   `NA_SE_SY_METRONOME` is done) the right one's, then Link's, within 160 frames
//!   (`songFailTimer`), then the next round (`MSGMODE_MEMORY_GAME_START_NEXT_ROUND`: it writes the
//!   box's mode and timer itself). A wrong note or the timeout: 0x102D, then 0x102E's offer of
//!   another try; all its notes: the chime, 0x10BF and the round's reward
//!   (`sOcarinaGameRewards[ocarinaGameRoundNum]`: a green, a blue rupee, a piece of heart, then red
//!   rupees), the round counted, `ITEMGETINF_17` after the third; then 0x102E again, or the end
//!   (`EnSkj_CleanupOcarinaGame`: the Skull Kids leave, the music back, the stump gone).
//! - **Type 5** (room 9): the small stump Link stands on for Saria's Song (invisible, a prop, no
//!   draw, no action of its own).
//! - **Any other type** (the adult layers' 0xFFFF, type 63): killed at init when the adult trade
//!   slot holds an item before the Poacher's Saw (`INV_CONTENT(ITEM_TRADE_ADULT) <
//!   ITEM_POACHERS_SAW`; an empty slot, `ITEM_NONE` 0xFF, isn't below it); else a Skull Kid who
//!   fights (`func_80AFEDF8` from his flute:
//!   `func_80AFF334`, `EnSkj_ResetFight`) and doesn't update during the ocarina (type 7 and up).
//!
//! The overlay's statics (`sSmallStumpSkullKid`, `sOcarinaMinigameSkullKids`, `D_80B01EA0`) are
//! [`Statics`] in the play state (ADR 0020, ADR 0059). An init doesn't know its handle yet, so it
//! leaves its number in the static and the next update fills the handle in
//! (`EnSkj::resolve_statics`). `D_80B01EA0` is written by every En_Skj's update
//! (`Actor_TalkOfferAccepted`) and read by whichever updates next, as in the C. The statics
//! are zeroed when the last En_Skj goes (the overlay unloads: `actor_free_overlay`).
//!
//! Drawn as `EnSkj_Draw` does: the skeleton with segment 0x0C's list setting the alpha on the
//! env colour (`EnSkj_OpaqueDL`), with the translucent render mode below 255
//! (`EnSkj_TranslucentDL`; @bug (game): both lists allocate one `Gfx` and write three or two,
//! harmless here), from two bakes (opaque, translucent; docs/adr/0012-actor-bakes.md); the Skull
//! Mask (`gSkullKidSkullMaskDL`) on limb 11 turned -0x4000 about x once it's sold
//! (`EnSkj_PostLimbDraw`). The circle shadow (40, at `shadowAlpha`) is the shape's
//! (`ActorShape_Init`'s, drawn by `Actor_Draw`). Types 5 and 6 have no draw (and no shadow). The translucent pass goes in the XLU list (as `En_Md`'s), where the C draws it in
//! `POLY_OPA_DISP` with the translucent render mode; and the mask's `Gfx_SetupDL_25Opa`, which in
//! the C resets the render mode for the limbs drawn after limb 11 while he fades, isn't modelled
//! (the bake is one pass).
//!
//! The whole overlay is ported, the debug display aside (`BREG(0)` 0). Masks aren't (BACKLOG
//! #39): `Player_GetMask` is Player's `currentMask` (`PLAYER_MASK_NONE`, unless a test sets it),
//! `MaskReaction_GetTextId(play, MASK_REACTION_SET_SKULL_KID)` gives 0.
//!
//! Params: `type` bits 10 to 15 (0 the stump's Skull Kid; 1 and 2 the memory game's left and
//! right; 5 the small stump; 6 the memory game's stump; 63, and any other, the adult's fighting
//! Skull Kid); init writes the type over the params for all but 5 and 6, and the code after reads
//! it there. Bits 0 to 9 are unused (0x3FF in the data).
//! Flags: reads and sets ITEMGETINF_16 (type 0: Saria's Song played to him, the heart piece
//! given), sets ITEMGETINF_17 (type 6: the memory game's third round won), reads and sets
//! ITEMGETINF_39 (type 0: the Skull Mask sold to him; read by the draw too); reads and sets
//! `ocarinaGameRoundNum` (the memory game's rounds won, its reward); sets `sunsSongState`
//! (`SUNSSONG_INACTIVE` after a song on the small stump).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey, SegmentValues};
use eng_math::{approach_f, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_NPC, ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, TextLine};
use oot_game::audio::NATURE_ID_KOKIRI_REGION;
use oot_game::audio::ocarina::{OCARINA_INSTRUMENT_FLUTE, OCARINA_SONG_MEMORY_GAME};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_METRONOME, NA_SE_SY_OCARINA_ERROR, NA_SE_SY_TRE_BOX_APPEAR};
use oot_game::collision_check::*;
use oot_game::effect::EffectInit;
use oot_game::effect::spark::EffectSparkInit;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::{GI_HEART_PIECE, GI_RUPEE_BLUE, GI_RUPEE_GREEN, GI_RUPEE_RED, ITEM_POACHERS_SAW, ITEM_POCKET_EGG, ITEM_SOLD_OUT, item_give, rupees_change_by};
use oot_game::item_get_inf;
use oot_game::message::*;
use oot_game::npc::{offer_talk, process_talk_request};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};
use oot_game::sys_matrix::MtxF;

use crate::en_item00::{ITEM00_RUPEE_ORANGE, item_drop_collectible};
use crate::en_skjneedle::{ACTOR_EN_SKJNEEDLE, EnSkjneedle};
use crate::player::{Player, STATE2_3, STATE2_23, STATE2_24, STATE2_25, STATE3_5};

/// `ACTOR_EN_SKJ` (`actor_table.h`: 0x0115).
pub const ACTOR_EN_SKJ: i16 = 0x0115;
/// `OBJECT_SKJ` (`object_table.h`: 0x010A).
pub const OBJECT: &str = "object_skj";

/// `En_Skj_Profile`: `ACTORCAT_ENEMY`, `FLAGS` (`ACTOR_FLAG_ATTENTION_ENABLED |
/// ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA`),
/// `OBJECT_SKJ`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_SKJ,
    name: "En_Skj",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: OBJECT,
    schema: &SCHEMA,
};

/// The type: `PARAMS_GET_U(thisx->params, 10, 6)`, written over the params at init (but 5, 6).
pub const TYPE: Field = Field::params(
    "type",
    10,
    6,
    Kind::Named(&[
        (0, "the Skull Kid on the stump (Saria's Song, the Skull Mask)"),
        (1, "the memory game's left Skull Kid"),
        (2, "the memory game's right Skull Kid"),
        (5, "the small stump Link plays Saria's Song on (invisible)"),
        (6, "the memory game's stump (invisible)"),
        (63, "the adult's Skull Kid, who fights (killed with a trade item before the Poacher's Saw)"),
    ]),
    "which Skull Kid or stump (3, 4 and 7 to 62 are the adult's too)",
);
/// The low ten bits: read by nothing.
pub const UNUSED: Field = Field::params("params bits 0 to 9", 0, 10, Kind::Unused, "0x3FF in the data; read by nothing");

/// `ITEMGETINF_16`, `ITEMGETINF_17`, `ITEMGETINF_39` (`save.h`).
pub const ITEMGETINF_16: u16 = 0x16;
pub const ITEMGETINF_17: u16 = 0x17;
pub const ITEMGETINF_39: u16 = 0x39;

/// En_Skj's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, UNUSED],
    flags: &[
        FlagUse::new(item_get_inf!(ITEMGETINF_16)).reads().sets().note("type 0: Saria's Song played to him (the heart piece given): he stays and talks"),
        FlagUse::new(item_get_inf!(ITEMGETINF_17)).sets().note("type 6: the memory game's third round won"),
        FlagUse::new(item_get_inf!(ITEMGETINF_39)).reads().sets().note("type 0: the Skull Mask sold to him (he wears it)"),
        FlagUse::new(Flag::Other("gSaveContext.save.info.playerData.ocarinaGameRoundNum")).reads().sets().note("type 6: the memory game's rounds won (its reward, its notes)"),
        FlagUse::new(Flag::Other("gSaveContext.sunsSongState")).sets().note("type 0: SUNSSONG_INACTIVE after another song on the small stump"),
    ],
    todo: false,
    objects: &[],
    texts: &[
        TextLine::new(0x1020).when("type", &[0]).cond("after Saria's Song, once the Skull Mask is sold to him").note("my buddies really want this mask!"),
        TextLine::new(0x10BC).when("type", &[0]).cond("after Saria's Song, Link wearing no mask").note("hi! is my face kind of plain?"),
        TextLine::new(0x101B).when("type", &[0]).cond("after Saria's Song, Link wearing the Skull Mask").note("under that mask, aren't you that Kokiri kid?"),
        TextLine::new(0x1041).when("type", &[0]).cond("another song played on the stump, before Saria's Song").note("that's not quite Saria's Song"),
        TextLine::new(0x10BB).when("type", &[0]).cond("Saria's Song played on the stump, the first time").note("you know Saria's Song! take this (the piece of heart)"),
        TextLine::new(0x10BD).when("type", &[0]).cond("a song played on the stump, after Saria's Song").note("let's play again sometime!"),
        TextLine::new(0x101C).when("type", &[0]).cond("talked to wearing the Skull Mask, before it's sold").note("an unusual mask: give it to me?"),
        TextLine::new(0x101E).when("type", &[0]).cond("answered Yes").note("yowza! I'm gonna wear this all the time"),
        TextLine::new(0x101F).when("type", &[0]).cond("after 0x101E, the mask sold").note("he gave you 10 Rupees for this 20-Rupee mask"),
        TextLine::new(0x101D).when("type", &[0]).cond("answered No").note("doh! that's too bad for me"),
        TextLine::new(0x10BE).when("type", &[6]).cond("the ocarina out on the stump").note("follow along with our song"),
        TextLine::new(0x102D).when("type", &[6]).cond("a wrong note, or too slow").note("too bad... heh heh!"),
        TextLine::new(0x10BF).when("type", &[6]).cond("the round played right").note("a nice session: take this (the round's reward)"),
        TextLine::new(0x102E).when("type", &[6]).cond("after 0x102D or 0x10BF").note("do you want to play some more?"),
    ],
};

/// `NAVI_ENEMY_SKULL_KID_ADULT`, `_SKULL_KID`, `_SKULL_KID_FRIENDLY`, `_SKULL_KID_MASK` (`actor.h`).
pub const NAVI_ENEMY_SKULL_KID_ADULT: u8 = 0x36;
pub const NAVI_ENEMY_SKULL_KID: u8 = 0x3F;
pub const NAVI_ENEMY_SKULL_KID_FRIENDLY: u8 = 0x40;
pub const NAVI_ENEMY_SKULL_KID_MASK: u8 = 0x41;

/// `ATTENTION_RANGE_2`, `ATTENTION_RANGE_7` (`actor.h`).
const ATTENTION_RANGE_2: u8 = 2;
const ATTENTION_RANGE_7: u8 = 7;
/// `PLAYER_MASK_NONE`, `PLAYER_MASK_SKULL` (`player.h`).
pub const PLAYER_MASK_NONE: u8 = 0;
pub const PLAYER_MASK_SKULL: u8 = 2;
/// `PLAYER_CSACTION_1`, `PLAYER_CSACTION_7` (`player.h`).
const PLAYER_CSACTION_1: u8 = 1;
const PLAYER_CSACTION_7: u8 = 7;
/// `OCARINA_MODE_05` (`ocarina.h`): a song other than the one checked (5 and up).
const OCARINA_MODE_05: u16 = 0x05;
/// `MASK_REACTION_SET_SKULL_KID` (`face_reaction.h`: 0x15).
const MASK_REACTION_SET_SKULL_KID: u8 = 0x15;

/// `SkullKidAnim`.
pub const SKJ_ANIM_BACKFLIP: u8 = 0;
pub const SKJ_ANIM_SHOOT_NEEDLE: u8 = 1;
pub const SKJ_ANIM_PLAY_FLUTE: u8 = 2;
pub const SKJ_ANIM_DIE: u8 = 3;
pub const SKJ_ANIM_HIT: u8 = 4;
pub const SKJ_ANIM_LAND: u8 = 5;
pub const SKJ_ANIM_LOOK_LEFT_RIGHT: u8 = 6;
pub const SKJ_ANIM_FIGHTING_STANCE: u8 = 7;
pub const SKJ_ANIM_WALK_TO_PLAYER: u8 = 8;
pub const SKJ_ANIM_WAIT: u8 = 9;

/// `SkullKidStumpSide`.
pub const SKULL_KID_LEFT: usize = 0;
pub const SKULL_KID_RIGHT: usize = 1;

/// `SkullKidOcarinaGameState`.
pub const SKULL_KID_OCARINA_WAIT: u8 = 0;
pub const SKULL_KID_OCARINA_PLAY_NOTES: u8 = 1;
pub const SKULL_KID_OCARINA_LEAVE_GAME: u8 = 2;

/// `SkullKidAction`.
pub const SKJ_ACTION_FADE: u8 = 0;
pub const SKJ_ACTION_WAIT_TO_SHOOT_NEEDLE: u8 = 1;
pub const SKJ_ACTION_SARIA_SONG_IDLE: u8 = 2;
pub const SKJ_ACTION_WAIT_FOR_DEATH_ANIM: u8 = 3;
pub const SKJ_ACTION_PICK_NEXT_FIHGT_ACTION: u8 = 4;
pub const SKJ_ACTION_WAIT_FOR_LAND_ANIM: u8 = 5;
pub const SKJ_ACTION_RESET_FIGHT: u8 = 6;
pub const SKJ_ACTION_FIGHT: u8 = 7;
pub const SKJ_ACTION_NEEDLE_RECOVER: u8 = 8;
pub const SKJ_ACTION_SPAWN_DEATH_EFFECT: u8 = 9;
pub const SKJ_ACTION_SARIA_SONG_WAIT_IN_RANGE: u8 = 10;
pub const SKJ_ACTION_SARIA_SONG_WAIT_FOR_SONG: u8 = 11;
pub const SKJ_ACTION_SARIA_SONG_AFTER_SONG: u8 = 12;
pub const SKJ_ACTION_SARIA_TALK: u8 = 13;
pub const SKJ_ACTION_UNK14: u8 = 14;
pub const SKJ_ACTION_SARIA_SONG_CHANGE_MODE: u8 = 15;
pub const SKJ_ACTION_SARIA_SONG_START_TRADE: u8 = 16;
pub const SKJ_ACTION_SARIA_SONG_WAIT_FOR_LANDING: u8 = 17;
pub const SKJ_ACTION_SARIA_SONG_WAIT_FOR_LANDING_ANIM: u8 = 18;
pub const SKJ_ACTION_SARIA_SONG_WALK_TO_PLAYER: u8 = 19;
pub const SKJ_ACTION_SARIA_SONG_ASK_FOR_MASK: u8 = 20;
pub const SKJ_ACTION_SARIA_SONG_TAKE_MASK: u8 = 21;
pub const SKJ_ACTION_SARIA_SONG_WAIT_MASK_TEXT: u8 = 22;
pub const SKJ_ACTION_SARIA_SONG_WRONG_SONG: u8 = 23;
pub const SKJ_ACTION_SARIA_SONG_WAIT_FOR_TEXT: u8 = 24;
pub const SKJ_ACTION_OCARINA_GAME_WAIT_FOR_PLAYER: u8 = 25;
pub const SKJ_ACTION_OCARINA_GAME_IDLE: u8 = 26;
pub const SKJ_ACTION_OCARINA_GAME_PLAY: u8 = 27;
pub const SKJ_ACTION_OCARINA_GAME_LEAVE: u8 = 28;

/// `sOcarinaGameRewards`, by `ocarinaGameRoundNum`.
pub const S_OCARINA_GAME_REWARDS: [i16; 4] = [GI_RUPEE_GREEN, GI_RUPEE_BLUE, GI_HEART_PIECE, GI_RUPEE_RED];

/// `sAnimationInfo` (`AnimationMinimalInfo`): the animation, its mode, its morph frames (all 0).
const ANIMATION_INFO: [(&str, u8, f32); 10] = [
    ("gSkullKidBackflipAnim", ANIMMODE_ONCE, 0.0),
    ("gSkullKidShootNeedleAnim", ANIMMODE_ONCE, 0.0),
    ("gSkullKidPlayFluteAnim", ANIMMODE_LOOP, 0.0),
    ("gSkullKidDieAnim", ANIMMODE_ONCE, 0.0),
    ("gSkullKidHitAnim", ANIMMODE_ONCE, 0.0),
    ("gSkullKidLandAnim", ANIMMODE_ONCE, 0.0),
    ("gSkullKidLookLeftAndRightAnim", ANIMMODE_LOOP, 0.0),
    ("gSkullKidFightingStanceAnim", ANIMMODE_LOOP, 0.0),
    ("gSkullKidWalkToPlayerAnim", ANIMMODE_LOOP, 0.0),
    ("gSkullKidWaitAnim", ANIMMODE_LOOP, 0.0),
];

/// `D_80B01678` (`ColliderCylinderInitType1`: `Collider_SetCylinderType1` makes `ocFlags2`
/// `OC2_TYPE_1`): an enemy's attack on Link (0xFFCFFFFF, 8), hit by Player, 8 by 48.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 8, height: 48, y_shift: 0, pos: [0, 0, 0] },
};

/// `sDamageTable`: the swords (0xF: a hit), the hammer and the spins and jumps hurt; everything
/// else (reaction 0) makes him backflip away.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x0), // Deku nut
        dmg_entry(0, 0x0), // Deku stick
        dmg_entry(0, 0x0), // Slingshot
        dmg_entry(0, 0x0), // Explosive
        dmg_entry(0, 0x0), // Boomerang
        dmg_entry(0, 0x0), // Normal arrow
        dmg_entry(2, 0x0), // Hammer swing
        dmg_entry(0, 0x0), // Hookshot
        dmg_entry(1, 0xF), // Kokiri sword
        dmg_entry(2, 0xF), // Master sword
        dmg_entry(4, 0xF), // Giant's Knife
        dmg_entry(0, 0x0), // Fire arrow
        dmg_entry(0, 0x0), // Ice arrow
        dmg_entry(0, 0x0), // Light arrow
        dmg_entry(0, 0x0), // Unk arrow 1
        dmg_entry(0, 0x0), // Unk arrow 2
        dmg_entry(0, 0x0), // Unk arrow 3
        dmg_entry(0, 0x0), // Fire magic
        dmg_entry(0, 0x0), // Ice magic
        dmg_entry(0, 0x0), // Light magic
        dmg_entry(0, 0x0), // Shield
        dmg_entry(0, 0x0), // Mirror Ray
        dmg_entry(1, 0x0), // Kokiri spin
        dmg_entry(4, 0x0), // Giant spin
        dmg_entry(2, 0x0), // Master spin
        dmg_entry(2, 0x0), // Kokiri jump
        dmg_entry(8, 0x0), // Giant jump
        dmg_entry(4, 0x0), // Master jump
        dmg_entry(0, 0x0), // Unknown 1
        dmg_entry(0, 0x0), // Unblockable
        dmg_entry(0, 0x0), // Hammer jump
        dmg_entry(0, 0x0), // Unknown 2
    ],
};

/// The limb `EnSkj_PostLimbDraw` puts the Skull Mask on (`limbIndex` 11, 1-based).
pub const SKJ_LIMB_MASK: usize = 11;
/// `gSPSegment(POLY_OPA_DISP++, 0x0C, ...)`: `EnSkj_TranslucentDL`'s render mode, or
/// `EnSkj_OpaqueDL`'s nothing (their env colour is the dynamic colour below).
const SEG_RENDER_MODE: u8 = 0x0C;
/// The `gDPSetEnvColor(0, 0, 0, alpha)` of both lists, as a dynamic colour run before the
/// skeleton.
const SEG_ALPHA: u8 = 0x0E;
/// `EnSkj_TranslucentDL`'s `gDPSetRenderMode(G_RM_FOG_SHADE_A, G_RM_AA_ZB_XLU_SURF2)` (no
/// `Z_UPD`, unlike `func_80034B54`'s).
const XLU_RENDER_MODE: (u32, u32) = (0xE200_001C, 0xC810_49D8);

/// The bake of a pass: opaque (`alpha` 255) or translucent.
pub fn bake_name(xlu: bool) -> String {
    format!("En_Skj/{}", if xlu { "xlu" } else { "opa" })
}

/// `EnSkj_Draw`'s meshes: `gSkullKidSkel` after `Gfx_SetupDL_25Opa`, opaque and translucent.
pub fn bakes() -> Vec<MeshBake> {
    [false, true]
        .into_iter()
        .map(|xlu| MeshBake {
            name: bake_name(xlu),
            object: OBJECT.into(),
            segments: vec![(SEG_RENDER_MODE, BakeSegment::Commands(if xlu { vec![XLU_RENDER_MODE] } else { Vec::new() })), (SEG_ALPHA, BakeSegment::DynamicColor { env: true, prim: false })],
            prelude: vec![SEG_ALPHA],
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gSkullKidSkel".into(), limbs: Vec::new() },
        })
        .collect()
}

/// `EnSkjUnkStruct`: a Skull Kid the overlay keeps.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct EnSkjUnkStruct {
    pub unk_0: u8,
    pub skullkid: Option<ActorHandle>,
    /// Not in the C: the init that set `skullkid` (`EnSkj::init_id`), its handle not known until
    /// it's in the actor context; `EnSkj::resolve_statics` fills `skullkid` in. 0: none.
    pub pending: u32,
}

/// The overlay's statics (`PlayState::overlay_static`, by `ACTOR_EN_SKJ`).
#[derive(Debug, Default)]
pub struct Statics {
    /// `sSmallStumpSkullKid`: the stump Link stands on (type 5, or type 6: the last to init).
    pub s_small_stump_skull_kid: EnSkjUnkStruct,
    /// `sOcarinaMinigameSkullKids`: the memory game's left and right Skull Kids (`unk_0` 1 there,
    /// 2 leaving).
    pub s_ocarina_minigame_skull_kids: [EnSkjUnkStruct; 2],
    /// `D_80B01EA0`: the last updated En_Skj's `Actor_TalkOfferAccepted`.
    pub d_80b01ea0: i32,
    /// Not in the C: the inits counted (`EnSkj::init_id`).
    pub inits: u32,
}

fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_EN_SKJ)
}

// shared: Actor_FreeOverlay
/// `Actor_FreeOverlay` (`z_actor.c`, from `Actor_Delete`): when the last En_Skj goes,
/// the overlay unloads and its statics are zeroed by the next load. (`ACTOROVL_ALLOC_NORMAL`:
/// freed at `numLoaded` 0.) Called from the destroy, which runs for every En_Skj here though
/// types 5 and 6 have none (`actor.destroy = NULL`): the overlay goes all the same.
fn actor_free_overlay(play: &mut PlayState) {
    let others = play.actors.all().into_iter().any(|h| play.actors.actor(h).is_some_and(|a| a.id == ACTOR_EN_SKJ));
    if !others {
        play.overlay_statics.remove(&ACTOR_EN_SKJ);
    }
}

// shared: BINANG_TO_RAD_ALT
/// `BINANG_TO_RAD_ALT` (`z_math.h`): `((f32)binang / (f32)0x8000) * M_PI`, the product in double
/// precision.
fn binang_to_rad_alt(b: i16) -> f32 {
    ((b as f32 / 32768.0) as f64 * std::f64::consts::PI) as f32
}

/// `Matrix_RotateY(BINANG_TO_RAD_ALT(yaw), MTXMODE_NEW)`, `Matrix_MultVec3f(&v, ...)`.
fn rotate_y_mult(yaw: i16, v: Vec3) -> Vec3 {
    let mut m = MtxF::IDENTITY;
    m.rotate_y(binang_to_rad_alt(yaw));
    m.mult_vec3f(v)
}

fn player(play: &PlayState) -> Option<&Player> {
    play.player.and_then(|h| play.actors.downcast::<Player>(h))
}

fn player_mut(play: &mut PlayState) -> Option<&mut Player> {
    let h = play.player?;
    play.actors.downcast_mut::<Player>(h)
}

fn player_state2(play: &PlayState) -> u32 {
    player(play).map_or(0, |p| p.state2)
}

/// `player->stateFlags2 |= flags`.
fn set_player_state2(play: &mut PlayState, flags: u32) {
    if let Some(p) = player_mut(play) {
        p.state2 |= flags;
    }
}

/// `player->unk_6A8 = actor`.
fn set_player_unk_6a8(play: &mut PlayState, actor: Option<ActorHandle>) {
    if let Some(p) = player_mut(play) {
        p.unk_6a8 = actor;
    }
}

/// `player->actor.world.pos = pos`.
fn set_player_pos(play: &mut PlayState, pos: Vec3) {
    if let Some(p) = player_mut(play) {
        p.actor.world_pos = pos;
    }
}

// shared: Player_GetMask
/// `Player_GetMask` (`z_player_lib.c`): `currentMask`.
fn player_get_mask(play: &PlayState) -> u8 {
    player(play).map_or(PLAYER_MASK_NONE, |p| p.current_mask)
}

// shared: func_8002DDE4
/// `func_8002DDE4` (`z_actor.c`): Player made a noise (`PLAYER_STATE2_3`).
fn func_8002dde4(play: &PlayState) -> bool {
    player_state2(play) & STATE2_3 != 0
}

// shared: MaskReaction_GetTextId
/// `MaskReaction_GetTextId` (`z_face_reaction.c`): no masks (BACKLOG #39), so 0.
fn mask_reaction_get_text_id(_play: &PlayState, _set: u8) -> u16 {
    0
}

// shared: Message_CloseTextbox
/// `Message_CloseTextbox` (`z_message.c`), as the other modules write it inline.
fn message_close_textbox(play: &mut PlayState) {
    play.with_msg(|m, f| m.close_textbox(f.audio));
}

/// `EnSkj_RangeCheck`: Player within 26 across the stump (`SQ` sum 676), not below it.
pub fn range_check(player_pos: Vec3, stump_pos: Vec3) -> bool {
    let x_diff = player_pos.x - stump_pos.x;
    let z_diff = player_pos.z - stump_pos.z;
    let y_diff = player_pos.y - stump_pos.y;
    (x_diff * x_diff + z_diff * z_diff <= 676.0) && (y_diff >= 0.0)
}

/// `EnSkj_SpawnBlood`: `Effect_Add`'s spark at `pos`, blue (0, 0, 128) to dark blue, 5 by 5, speed
/// 8, falling by 1, for 8 frames.
fn spawn_blood(play: &mut PlayState, pos: Vec3) {
    let start = [0, 0, 128, 255];
    let effect = EffectSparkInit {
        position: [pos.x as i32 as i16, pos.y as i32 as i16, pos.z as i32 as i16],
        u_div: 5,
        v_div: 5,
        color_start: [start, start, start, start],
        color_end: [[0, 0, 32, 0], [0, 0, 32, 0], [0, 0, 64, 0], [0, 0, 64, 0]],
        speed: 8.0,
        gravity: -1.0,
        timer: 0,
        duration: 8,
    };
    play.effect_add(EffectInit::Spark(effect));
}

/// Which update the actor has (`actor.update`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateFn {
    /// `EnSkj_Update`.
    Update,
    /// `EnSkj_SariasSongShortStumpUpdate` (type 5).
    SariasSongShortStump,
    /// `EnSkj_OcarinaMinigameShortStumpUpdate` (type 6).
    OcarinaMinigameShortStump,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    /// `NULL` (type 5: never set, never called).
    None,
    Fade,
    WaitToShootNeedle,
    SariasSongKidIdle,
    WaitForDeathAnim,
    PickNextFightAction,
    WaitForLandAnim,
    ResetFight,
    Fight,
    NeedleRecover,
    SpawnDeathEffect,
    WaitInRange,
    WaitForSong,
    AfterSong,
    SariaSongTalk,
    Func80affe44,
    ChangeModeAfterSong,
    StartMaskTrade,
    WaitForLanding,
    WaitForLandAnimFinish,
    WalkToPlayer,
    AskForMask,
    TakeMask,
    WaitForMaskTextClear,
    WrongSong,
    SariasSongWaitForTextClear,
    OcarinaGameWaitForPlayer,
    OcarinaGameIdle,
    PlayOcarinaGame,
    LeaveOcarinaGame,
    // The memory game's stump (type 6).
    SetupWaitForOcarina,
    WaitForOcarina,
    StartOcarinaMinigame,
    WaitForPlayback,
    FailedMiniGame,
    WaitForNextRound,
    WaitForOfferResponse,
    WonOcarinaMiniGame,
    WaitToGiveReward,
    GiveOcarinaGameReward,
    FinishOcarinaGameRound,
    CleanupOcarinaGame,
}

impl Func {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Func::None => "NULL",
            Func::Fade => "EnSkj_Fade",
            Func::WaitToShootNeedle => "EnSkj_WaitToShootNeedle",
            Func::SariasSongKidIdle => "EnSkj_SariasSongKidIdle",
            Func::WaitForDeathAnim => "EnSkj_WaitForDeathAnim",
            Func::PickNextFightAction => "EnSkj_PickNextFightAction",
            Func::WaitForLandAnim => "EnSkj_WaitForLandAnim",
            Func::ResetFight => "EnSkj_ResetFight",
            Func::Fight => "EnSkj_Fight",
            Func::NeedleRecover => "EnSkj_NeedleRecover",
            Func::SpawnDeathEffect => "EnSkj_SpawnDeathEffect",
            Func::WaitInRange => "EnSkj_WaitInRange",
            Func::WaitForSong => "EnSkj_WaitForSong",
            Func::AfterSong => "EnSkj_AfterSong",
            Func::SariaSongTalk => "EnSkj_SariaSongTalk",
            Func::Func80affe44 => "func_80AFFE44",
            Func::ChangeModeAfterSong => "EnSkj_ChangeModeAfterSong",
            Func::StartMaskTrade => "EnSkj_StartMaskTrade",
            Func::WaitForLanding => "EnSkj_WaitForLanding",
            Func::WaitForLandAnimFinish => "EnSkj_WaitForLandAnimFinish",
            Func::WalkToPlayer => "EnSkj_WalkToPlayer",
            Func::AskForMask => "EnSkj_AskForMask",
            Func::TakeMask => "EnSkj_TakeMask",
            Func::WaitForMaskTextClear => "EnSkj_WaitForMaskTextClear",
            Func::WrongSong => "EnSkj_WrongSong",
            Func::SariasSongWaitForTextClear => "EnSkj_SariasSongWaitForTextClear",
            Func::OcarinaGameWaitForPlayer => "EnSkj_OcarinaGameWaitForPlayer",
            Func::OcarinaGameIdle => "EnSkj_OcarinaGameIdle",
            Func::PlayOcarinaGame => "EnSkj_PlayOcarinaGame",
            Func::LeaveOcarinaGame => "EnSkj_LeaveOcarinaGame",
            Func::SetupWaitForOcarina => "EnSkj_SetupWaitForOcarina",
            Func::WaitForOcarina => "EnSkj_WaitForOcarina",
            Func::StartOcarinaMinigame => "EnSkj_StartOcarinaMinigame",
            Func::WaitForPlayback => "EnSkj_WaitForPlayback",
            Func::FailedMiniGame => "EnSkj_FailedMiniGame",
            Func::WaitForNextRound => "EnSkj_WaitForNextRound",
            Func::WaitForOfferResponse => "EnSkj_WaitForOfferResponse",
            Func::WonOcarinaMiniGame => "EnSkj_WonOcarinaMiniGame",
            Func::WaitToGiveReward => "EnSkj_WaitToGiveReward",
            Func::GiveOcarinaGameReward => "EnSkj_GiveOcarinaGameReward",
            Func::FinishOcarinaGameRound => "EnSkj_FinishOcarinaGameRound",
            Func::CleanupOcarinaGame => "EnSkj_CleanupOcarinaGame",
        }
    }
}

/// `sActionFuncs`, by `SkullKidAction`.
const S_ACTION_FUNCS: [Func; 29] = [
    Func::Fade,
    Func::WaitToShootNeedle,
    Func::SariasSongKidIdle,
    Func::WaitForDeathAnim,
    Func::PickNextFightAction,
    Func::WaitForLandAnim,
    Func::ResetFight,
    Func::Fight,
    Func::NeedleRecover,
    Func::SpawnDeathEffect,
    Func::WaitInRange,
    Func::WaitForSong,
    Func::AfterSong,
    Func::SariaSongTalk,
    Func::Func80affe44,
    Func::ChangeModeAfterSong,
    Func::StartMaskTrade,
    Func::WaitForLanding,
    Func::WaitForLandAnimFinish,
    Func::WalkToPlayer,
    Func::AskForMask,
    Func::TakeMask,
    Func::WaitForMaskTextClear,
    Func::WrongSong,
    Func::SariasSongWaitForTextClear,
    Func::OcarinaGameWaitForPlayer,
    Func::OcarinaGameIdle,
    Func::PlayOcarinaGame,
    Func::LeaveOcarinaGame,
];

pub struct EnSkj {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    /// `sAnimationInfo`'s animations, by `SkullKidAnim`.
    anims: Vec<Anim>,
    pub func: Func,
    pub update_fn: UpdateFn,
    /// `actor.draw != NULL`.
    pub has_draw: bool,
    pub collider: ColliderCylinder,
    pub text_id: u16,
    pub timer: i16,
    pub unk_2c8: i16,
    /// `multiuseTimer`: `songFailTimer` (the stump), `battleExitTimer` (the fight).
    pub multiuse_timer: i16,
    pub needle_shoot_timer: i16,
    pub dodge_reset_timer: i16,
    pub anim_index: u8,
    pub action: u8,
    pub backflip_flag: u8,
    /// `unk_2D3`: 1 in the fight's and his idle's actions (hit and hitting), 0 in the others.
    pub unk_2d3: u8,
    pub needles_to_shoot: u8,
    pub hits_until_dodge: u8,
    /// `unk_2D6`: 1 while he plays along with Link's song; 2 is never set (`EnSkj_Fade` reads it).
    pub unk_2d6: u8,
    pub player_in_range: u8,
    pub minigame_state: u8,
    pub alpha: u32,
    pub center: Vec3,
    pub unk_2ec: f32,
    pub unk_2f0: f32,
    /// `posCopy`: set at init (types 1, 2), read by nothing.
    pub pos_copy: Vec3,
    /// Not in the C: this init's number, for the statics' handles (`resolve_statics`).
    pub init_id: u32,
}

impl EnSkj {
    fn new(actor: Actor) -> EnSkj {
        EnSkj {
            actor,
            skel: SkelAnimeStd::init_flex(0, None),
            skeleton: None,
            anims: Vec::new(),
            func: Func::None,
            update_fn: UpdateFn::Update,
            has_draw: true,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            text_id: 0,
            timer: 0,
            unk_2c8: 0,
            multiuse_timer: 0,
            needle_shoot_timer: 0,
            dodge_reset_timer: 0,
            anim_index: 0,
            action: 0,
            backflip_flag: 0,
            unk_2d3: 0,
            needles_to_shoot: 0,
            hits_until_dodge: 0,
            unk_2d6: 0,
            player_in_range: 0,
            minigame_state: 0,
            alpha: 0,
            center: Vec3::ZERO,
            unk_2ec: 0.0,
            unk_2f0: 0.0,
            pos_copy: Vec3::ZERO,
            init_id: 0,
        }
    }

    /// The type init wrote over the params (types 0 to 4 and 7 to 63).
    fn params(&self) -> i16 {
        // schema: the type, written over the params at init.
        self.actor.params
    }

    /// The overlay static's Skull Kid set to this one (`.unk_0 = 1; .skullkid = this`): its
    /// handle filled in at the next update.
    fn register(&mut self, play: &mut PlayState, slot: impl Fn(&mut Statics) -> &mut EnSkjUnkStruct) {
        let s = statics(play);
        if self.init_id == 0 {
            s.inits += 1;
            self.init_id = s.inits;
        }
        let e = slot(s);
        e.unk_0 = 1;
        e.skullkid = None;
        e.pending = self.init_id;
    }

    /// Not in the C: the statics' Skull Kids set by inits get their handles.
    fn resolve_statics(&self, play: &mut PlayState) {
        let s = statics(play);
        let slots = [s.s_small_stump_skull_kid.pending, s.s_ocarina_minigame_skull_kids[0].pending, s.s_ocarina_minigame_skull_kids[1].pending];
        if slots.iter().all(|&p| p == 0) {
            return;
        }
        let find = |play: &PlayState, id: u32| -> Option<ActorHandle> {
            if id == self.init_id {
                return play.cur_actor;
            }
            play.actors.all().into_iter().find(|&h| play.actors.downcast::<EnSkj>(h).is_some_and(|e| e.init_id == id))
        };
        let found: Vec<Option<ActorHandle>> = slots.iter().map(|&p| if p == 0 { None } else { find(play, p) }).collect();
        let s = statics(play);
        for (i, h) in found.into_iter().enumerate() {
            let e = match i {
                0 => &mut s.s_small_stump_skull_kid,
                i => &mut s.s_ocarina_minigame_skull_kids[i - 1],
            };
            if e.pending != 0 && h.is_some() {
                e.skullkid = h;
                e.pending = 0;
            }
        }
    }

    /// `sSmallStumpSkullKid.skullkid`'s handle and its `world.pos` and `world.rot.y`.
    fn small_stump(play: &mut PlayState) -> Option<(ActorHandle, Vec3, i16)> {
        let h = statics(play).s_small_stump_skull_kid.skullkid?;
        if Some(h) == play.cur_actor {
            return None;
        }
        play.actors.actor(h).map(|a| (h, a.world_pos, a.world_rot.y))
    }

    /// `sOcarinaMinigameSkullKids[side].skullkid`.
    fn minigame_kid(play: &mut PlayState, side: usize) -> Option<&mut EnSkj> {
        let h = statics(play).s_ocarina_minigame_skull_kids[side].skullkid?;
        play.actors.downcast_mut::<EnSkj>(h)
    }

    fn set_minigame_state(play: &mut PlayState, side: usize, state: u8) {
        if let Some(k) = Self::minigame_kid(play, side) {
            k.minigame_state = state;
        }
    }

    /// `Animation_GetLastFrame(sAnimationInfo[index].animation)` as the `s16` the C keeps.
    fn last_frame(&self, index: u8) -> i16 {
        self.anims.get(index as usize).map_or(0, |a| a.last_frame() as i16)
    }

    /// `skelAnime.curFrame == lastFrame` (an `s16` last frame).
    fn on_last_frame(&self, index: u8) -> bool {
        self.skel.cur_frame == self.last_frame(index) as f32
    }

    /// `EnSkj_ChangeAnim`: from frame 0 to the last at speed 1, the info's mode, no morph.
    fn change_anim(&mut self, index: u8) {
        self.anim_index = index;
        if let Some(a) = self.anims.get(index as usize).cloned() {
            let (_, mode, morph) = ANIMATION_INFO[index as usize];
            let end_frame = a.last_frame() as i16 as f32;
            self.skel.change(a, 1.0, 0.0, end_frame, mode, morph);
        }
    }

    // shared: Animation_Reverse
    /// `Animation_Reverse` (`z_skelanime.c`; `SkelAnimeStd` has none).
    fn animation_reverse(&mut self) {
        let start_frame = self.skel.start_frame;
        self.skel.start_frame = self.skel.end_frame;
        self.skel.play_speed = -self.skel.play_speed;
        self.skel.end_frame = start_frame;
    }

    /// `EnSkj_SetupAction`: `sActionFuncs[action]`; `unk_2D3` 0 for the fade, the death, the
    /// trade's, the texts' and the memory game's, 1 for the rest.
    fn setup_action(&mut self, action: u8) {
        self.action = action;
        self.func = S_ACTION_FUNCS[action as usize];
        self.unk_2d3 = match action {
            SKJ_ACTION_FADE
            | SKJ_ACTION_WAIT_FOR_DEATH_ANIM
            | SKJ_ACTION_PICK_NEXT_FIHGT_ACTION
            | SKJ_ACTION_SPAWN_DEATH_EFFECT
            | SKJ_ACTION_SARIA_SONG_START_TRADE
            | SKJ_ACTION_SARIA_SONG_WAIT_FOR_LANDING
            | SKJ_ACTION_SARIA_SONG_WAIT_FOR_LANDING_ANIM
            | SKJ_ACTION_SARIA_SONG_WALK_TO_PLAYER
            | SKJ_ACTION_SARIA_SONG_ASK_FOR_MASK
            | SKJ_ACTION_SARIA_SONG_TAKE_MASK
            | SKJ_ACTION_SARIA_SONG_WAIT_MASK_TEXT
            | SKJ_ACTION_SARIA_SONG_WRONG_SONG
            | SKJ_ACTION_SARIA_SONG_WAIT_FOR_TEXT
            | SKJ_ACTION_OCARINA_GAME_WAIT_FOR_PLAYER
            | SKJ_ACTION_OCARINA_GAME_IDLE
            | SKJ_ACTION_OCARINA_GAME_PLAY
            | SKJ_ACTION_OCARINA_GAME_LEAVE => 0,
            _ => 1,
        };
    }

    /// `EnSkj_CalculateCenter`: 120 ahead of him.
    fn calculate_center(&mut self) {
        let c = rotate_y_mult(self.actor.shape_rot.y, Vec3::new(0.0, 0.0, 120.0));
        self.center = Vec3::new(c.x + self.actor.world_pos.x, c.y, c.z + self.actor.world_pos.z);
    }

    /// `EnSkj_SetNaviId`.
    fn set_navi_id(&mut self, play: &PlayState) {
        self.actor.navi_enemy_id = match self.params() {
            0 => {
                if play.save.get_item_get_inf(ITEMGETINF_39) {
                    NAVI_ENEMY_SKULL_KID_MASK
                } else if play.save.get_item_get_inf(ITEMGETINF_16) {
                    NAVI_ENEMY_SKULL_KID_FRIENDLY
                } else {
                    // No Saria's Song, no Skull Mask.
                    NAVI_ENEMY_SKULL_KID
                }
            }
            1 | 2 => NAVI_ENEMY_SKULL_KID,
            _ => NAVI_ENEMY_SKULL_KID_ADULT,
        };
    }

    /// `EnSkj_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let ty = TYPE.get(actor.params);
        // sInitChain: attentionRangeType ATTENTION_RANGE_2, lockOnArrowOffset 30.
        actor.target_mode = ATTENTION_RANGE_2;
        actor.target_arrow_offset = 30.0;
        let mut this = EnSkj::new(actor);
        match ty {
            5 => {
                // Invisible on the small stump (Saria's Song).
                this.register(play, |s| &mut s.s_small_stump_skull_kid);
                this.has_draw = false;
                this.update_fn = UpdateFn::SariasSongShortStump;
                this.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE);
                // Actor_ChangeCategory: the spawn files it there.
                this.actor.category = ACTORCAT_PROP;
            }
            6 => {
                // Invisible on the short stump (the memory game).
                this.register(play, |s| &mut s.s_small_stump_skull_kid);
                this.has_draw = false;
                this.update_fn = UpdateFn::OcarinaMinigameShortStump;
                this.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE);
                this.actor.category = ACTORCAT_PROP;
                this.actor.focus_pos = Vec3::new(1230.0, -90.0, 450.0);
                this.func = Func::SetupWaitForOcarina;
            }
            _ => {
                // schema: init writes the type over the params; the code after reads it there.
                this.actor.params = ty;
                if this.params() != 0 && this.params() != 1 && this.params() != 2 && play.save.inv_content(ITEM_POCKET_EGG) < ITEM_POACHERS_SAW {
                    // INV_CONTENT(ITEM_TRADE_ADULT) (ITEM_TRADE_ADULT is ITEM_POCKET_EGG): an empty
                    // slot (ITEM_NONE, 0xFF) isn't below the saw.
                    this.actor.kill();
                    return Box::new(this);
                }
                this.set_navi_id(play);
                // SkelAnime_InitFlex(.., gSkullKidSkel, gSkullKidPlayFluteAnim, .., 19).
                let loaded = play.assets.clone().map(|a| {
                    let skeleton = a.skeleton(OBJECT, "gSkullKidSkel").map_err(|e| log::error!("En_Skj: {e:#}")).ok();
                    let anims: Vec<Anim> = ANIMATION_INFO.iter().filter_map(|(n, ..)| a.animation(OBJECT, n).map_err(|e| log::error!("En_Skj: {e:#}")).ok()).collect();
                    (skeleton, anims)
                });
                if let Some((skeleton, anims)) = loaded
                    && anims.len() == ANIMATION_INFO.len()
                {
                    let limbs = skeleton.as_ref().map_or(18, |s| s.limbs.len());
                    this.skel = SkelAnimeStd::init_flex(limbs, Some(anims[SKJ_ANIM_PLAY_FLUTE as usize].clone()));
                    this.skeleton = skeleton;
                    this.anims = anims;
                }
                if (0..3).contains(&ty) {
                    this.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE);
                    this.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY;
                    this.actor.category = ACTORCAT_NPC;
                }
                if !(0..7).contains(&ty) {
                    this.actor.flags &= !ACTOR_FLAG_UPDATE_DURING_OCARINA;
                }
                if ty > 0 && ty < 3 {
                    this.actor.target_mode = ATTENTION_RANGE_7;
                    this.pos_copy = this.actor.world_pos;
                    this.register(play, |s| &mut s.s_ocarina_minigame_skull_kids[ty as usize - 1]);
                    this.minigame_state = 0;
                    this.alpha = 0;
                    this.ocarina_game_setup_wait_for_player();
                } else {
                    this.alpha = 255;
                    this.setup_reset_fight();
                }
                this.actor.col_chk_info.damage_table = Some(&S_DAMAGE_TABLE);
                this.actor.col_chk_info.health = 10;
                this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 40.0);
                this.actor.scale = Vec3::splat(0.01);
                this.text_id = 0;
                this.actor.text_id = 0;
                this.multiuse_timer = 0;
                this.backflip_flag = 0;
                this.needles_to_shoot = 3;
                this.hits_until_dodge = 3;
                this.actor.speed_xz = 0.0;
                this.actor.velocity.y = 0.0;
                this.actor.gravity = -1.0;
                this.calculate_center();
                let pp = player(play).map(|p| p.actor.world_pos).unwrap_or_default();
                log::debug!(
                    "Player_X : {}\nPlayer_Z : {}\nWorld_X  : {}\nWorld_Z  : {}\nCenter_X : {}\nCenter_Z : {}\n",
                    pp.x,
                    pp.z,
                    this.actor.world_pos.x,
                    this.actor.world_pos.z,
                    this.center.x,
                    this.center.z
                );
            }
        }
        Box::new(this)
    }

    /// `EnSkj_GetItemXzRange`: his distance across from the small stump, and 26.
    fn get_item_xz_range(&self, play: &mut PlayState) -> f32 {
        let s = Self::small_stump(play).map_or(self.actor.world_pos, |s| s.1);
        let x_diff = s.x - self.actor.world_pos.x;
        let z_diff = s.z - self.actor.world_pos.z;
        (x_diff * x_diff + z_diff * z_diff).sqrt() + 26.0
    }

    /// `EnSkj_GetItemYRange`: his height from the small stump, and 10.
    fn get_item_y_range(&self, play: &mut PlayState) -> f32 {
        let s = Self::small_stump(play).map_or(self.actor.world_pos, |s| s.1);
        (s.y - self.actor.world_pos.y).abs() + 10.0
    }

    /// `Actor_OfferTalk(&this->actor, play, EnSkj_GetItemXzRange(this))`.
    fn offer_talk_from_stump(&self, play: &mut PlayState) {
        let r = self.get_item_xz_range(play);
        offer_talk(play, &self.actor, r);
    }

    /// `Actor_OfferGetItem(&this->actor, play, GI_HEART_PIECE, EnSkj_GetItemXzRange(this),
    /// EnSkj_GetItemYRange(this))`.
    fn offer_heart_piece(&self, play: &mut PlayState) {
        let (xz, y) = (self.get_item_xz_range(play), self.get_item_y_range(play));
        offer_get_item_range(play, &self.actor, GI_HEART_PIECE, xz, y);
    }

    /// `EnSkj_ShootNeedle`: an `En_Skjneedle` 40 ahead (1.5 to the side) and 27 up, flying at
    /// 24, gone after 100 frames.
    fn shoot_needle(&mut self, play: &mut PlayState) -> bool {
        let mut pos2 = rotate_y_mult(self.actor.shape_rot.y, Vec3::new(1.5, 0.0, 40.0));
        pos2.x += self.actor.world_pos.x;
        pos2.z += self.actor.world_pos.z;
        pos2.y = self.actor.world_pos.y + 27.0;
        let r = self.actor.shape_rot;
        let Ok(h) = play.actor_spawn(ACTOR_EN_SKJNEEDLE, pos2, [r.x, r.y, r.z], 0) else { return false };
        match play.actors.downcast_mut::<EnSkjneedle>(h) {
            Some(needle) => {
                needle.kill_timer = 100;
                needle.actor.speed_xz = 24.0;
                true
            }
            None => false,
        }
    }

    /// `EnSkj_CollisionCheck`: a hit outside his talks (`unk_2D3`, `D_80B01EA0`): the swords' (0xF)
    /// blood and hit mark, death from behind while he plays or looks about, else the damage and
    /// the hit (or death at 0); reaction 0 a backflip away.
    fn collision_check(&mut self, play: &mut PlayState) -> bool {
        let d_80b01ea0 = statics(play).d_80b01ea0;
        if !((self.unk_2d3 == 0) || (d_80b01ea0 != 0) || (self.collider.base.ac_flags & AC_HIT == 0)) {
            self.collider.base.ac_flags &= !AC_HIT;
            match self.actor.col_chk_info.damage_reaction {
                0xF => {
                    let p = self.collider.info.ac_dmg_info.hit_pos;
                    let effect_pos = Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
                    spawn_blood(play, effect_pos);
                    play.with_ss(|s| s.hit_mark_spawn_fixed_scale(1, effect_pos));
                    let yaw_diff = self.actor.yaw_towards_player.wrapping_sub(self.actor.world_rot.y);
                    if (self.action == SKJ_ACTION_SARIA_SONG_IDLE || self.action == SKJ_ACTION_RESET_FIGHT) && (yaw_diff > 0x6000 || yaw_diff < -0x6000) {
                        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 8);
                        self.setup_die();
                        return true;
                    }
                    self.actor.apply_damage();
                    self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 8);
                    if self.actor.col_chk_info.health != 0 {
                        if self.hits_until_dodge != 0 {
                            self.hits_until_dodge -= 1;
                        }
                        if self.dodge_reset_timer == 0 {
                            self.dodge_reset_timer = 60;
                        }
                        self.func_80aff1f0();
                        return true;
                    }
                    self.setup_die();
                    true
                }
                0 => {
                    self.backflip_flag = 1;
                    self.backflip();
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    /// `func_80AFEDF8`: Link noisy within `unk_2EC`, or in front within `unk_2C8`.
    fn func_80afedf8(&self, play: &PlayState) -> bool {
        if self.actor.xz_dist_to_player < self.unk_2ec && func_8002dde4(play) {
            return true;
        }
        let yaw_diff = self.actor.yaw_towards_player.wrapping_sub(self.actor.world_rot.y);
        (yaw_diff as i32) < self.unk_2c8 as i32 && -(self.unk_2c8 as i32) < yaw_diff as i32
    }

    /// `EnSkj_Backflip`: back at 8, 8 up.
    fn backflip(&mut self) {
        self.actor.velocity.y = 8.0;
        self.actor.speed_xz = -8.0;
        self.change_anim(SKJ_ANIM_BACKFLIP);
        self.setup_action(SKJ_ACTION_FADE);
    }

    /// `EnSkj_Fade`: fading 20 a frame (with `backflipFlag`); landed, `func_80AFF2A0`.
    fn fade(&mut self, play: &mut PlayState) {
        let mut alpha = self.alpha;
        if self.unk_2d6 == 2 {
            play.msg_ctx.ocarina_mode = OCARINA_MODE_00;
            self.unk_2d6 = 0;
        }
        alpha = alpha.wrapping_sub(20);
        if self.backflip_flag != 0 {
            if alpha > 255 {
                alpha = 0;
            }
            self.alpha = alpha;
            self.actor.shape_shadow_alpha = alpha as u8;
        }
        if self.actor.velocity.y <= 0.0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND_TOUCH;
            self.func_80aff2a0();
        }
    }

    /// `EnSkj_SetupWaitToShootNeedle`.
    fn setup_wait_to_shoot_needle(&mut self) {
        self.needles_to_shoot = 3;
        self.needle_shoot_timer = 0;
        self.change_anim(SKJ_ANIM_SHOOT_NEEDLE);
        self.setup_action(SKJ_ACTION_WAIT_TO_SHOOT_NEEDLE);
    }

    /// `EnSkj_WaitToShootNeedle`: at the animation's end, a needle every 4 frames, three; then
    /// back (`EnSkj_SetupNeedleRecover`).
    fn wait_to_shoot_needle(&mut self, play: &mut PlayState) {
        if self.on_last_frame(SKJ_ANIM_SHOOT_NEEDLE) && self.needle_shoot_timer == 0 {
            let mut val = self.needles_to_shoot;
            if self.needles_to_shoot != 0 {
                self.shoot_needle(play);
                self.needle_shoot_timer = 4;
                val -= 1;
                self.needles_to_shoot = val;
            } else {
                self.setup_needle_recover();
            }
        }
    }

    /// `EnSkj_SetupResetFight`: playing his flute, watching 200 around and 0xAAA ahead.
    fn setup_reset_fight(&mut self) {
        self.unk_2c8 = 0xAAA;
        self.unk_2ec = 200.0;
        self.change_anim(SKJ_ANIM_PLAY_FLUTE);
        self.setup_action(SKJ_ACTION_SARIA_SONG_IDLE);
    }

    /// `EnSkj_SariasSongKidIdle`: type 0 flees before `ITEMGETINF_16` (within 200), else waits
    /// for Link on the small stump; the others watch for him (`func_80AFEDF8`).
    fn sarias_song_kid_idle(&mut self, play: &mut PlayState) {
        if self.params() == 0 {
            let small = statics(play).s_small_stump_skull_kid;
            if !play.save.get_item_get_inf(ITEMGETINF_16) && self.actor.xz_dist_to_player < 200.0 {
                self.backflip_flag = 1;
                self.backflip();
            } else if small.unk_0 != 0 {
                let pp = player(play).map(|p| p.actor.world_pos);
                if let (Some(pp), Some((h, sp, _))) = (pp, Self::small_stump(play))
                    && range_check(pp, sp)
                {
                    self.setup_wait_in_range();
                    set_player_state2(play, STATE2_23);
                    set_player_unk_6a8(play, Some(h));
                }
            }
        } else if self.func_80afedf8(play) {
            self.func_80aff334();
        }
    }

    /// `EnSkj_SetupDie`.
    fn setup_die(&mut self) {
        self.change_anim(SKJ_ANIM_DIE);
        self.setup_action(SKJ_ACTION_WAIT_FOR_DEATH_ANIM);
    }

    /// `EnSkj_WaitForDeathAnim`.
    fn wait_for_death_anim(&mut self) {
        if self.on_last_frame(SKJ_ANIM_DIE) {
            self.setup_spawn_death_effect();
        }
    }

    /// `func_80AFF1F0`: the hit.
    fn func_80aff1f0(&mut self) {
        self.change_anim(SKJ_ANIM_HIT);
        self.setup_action(SKJ_ACTION_PICK_NEXT_FIHGT_ACTION);
    }

    /// `EnSkj_PickNextFightAction`: after the hit, a backflip after three, else the stance.
    fn pick_next_fight_action(&mut self) {
        if self.on_last_frame(SKJ_ANIM_HIT) {
            if self.hits_until_dodge == 0 {
                self.hits_until_dodge = 3;
                self.backflip();
            } else {
                self.setup_stand();
            }
        }
    }

    /// `func_80AFF2A0`: landed: a new centre, the landing.
    fn func_80aff2a0(&mut self) {
        self.calculate_center();
        self.actor.speed_xz = 0.0;
        self.change_anim(SKJ_ANIM_LAND);
        self.setup_action(SKJ_ACTION_WAIT_FOR_LAND_ANIM);
    }

    /// `EnSkj_WaitForLandAnim`.
    fn wait_for_land_anim(&mut self) {
        if self.on_last_frame(SKJ_ANIM_LAND) {
            self.setup_stand();
        }
    }

    /// `func_80AFF334`: looking about, 400 frames, watching 600 around and 0x2000 ahead.
    fn func_80aff334(&mut self) {
        self.unk_2c8 = 0x2000;
        self.multiuse_timer = 400;
        self.unk_2ec = 600.0;
        self.change_anim(SKJ_ANIM_LOOK_LEFT_RIGHT);
        self.setup_action(SKJ_ACTION_RESET_FIGHT);
    }

    /// `EnSkj_ResetFight`: back to his flute when `battleExitTimer` runs out; Link seen, 600
    /// frames of fight.
    fn reset_fight(&mut self, play: &PlayState) {
        if self.multiuse_timer == 0 {
            self.setup_reset_fight();
        } else if self.func_80afedf8(play) {
            self.multiuse_timer = 600;
            self.setup_stand();
        }
    }

    /// `EnSkj_SetupStand`: the stance, a volley in 60 frames.
    fn setup_stand(&mut self) {
        self.needle_shoot_timer = 60;
        self.unk_2c8 = 0x2000;
        self.unk_2f0 = 0.0;
        self.unk_2ec = 600.0;
        self.change_anim(SKJ_ANIM_FIGHTING_STANCE);
        self.setup_action(SKJ_ACTION_FIGHT);
    }

    /// `EnSkj_Fight`: the volley when its timer's out; while `battleExitTimer` lasts, round his
    /// centre facing Link (the stance faster the further he moved, backwards turning left); after
    /// it, a backflip away if Link's seen, else his flute.
    fn fight(&mut self, play: &PlayState) {
        if self.needle_shoot_timer == 0 {
            self.setup_wait_to_shoot_needle();
        } else if self.multiuse_timer != 0 {
            let yaw_dist_to_player = smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 5, self.unk_2f0 as i32 as i16, 0);
            self.actor.world_rot.y = self.actor.shape_rot.y;
            approach_f(&mut self.unk_2f0, 2000.0, 1.0, 200.0);
            let pos2 = rotate_y_mult(self.actor.shape_rot.y, Vec3::new(0.0, 0.0, -120.0));
            let prev_pos_x = self.actor.world_pos.x;
            let prev_pos_z = self.actor.world_pos.z;
            self.actor.world_pos.x = self.center.x + pos2.x;
            self.actor.world_pos.z = self.center.z + pos2.z;
            let dx = self.actor.world_pos.x - prev_pos_x;
            let dz = self.actor.world_pos.z - prev_pos_z;
            let mut phi_f14 = (dx * dx + dz * dz).sqrt();
            phi_f14 = phi_f14.min(10.0);
            phi_f14 /= 10.0;
            self.skel.play_speed = if yaw_dist_to_player < 0 { -(1.0 + phi_f14) } else { 1.0 + phi_f14 };
        } else if self.func_80afedf8(play) {
            self.backflip_flag = 1;
            self.backflip();
        } else {
            self.setup_reset_fight();
        }
    }

    /// `EnSkj_SetupNeedleRecover`: the shot played back.
    fn setup_needle_recover(&mut self) {
        self.animation_reverse();
        self.setup_action(SKJ_ACTION_NEEDLE_RECOVER);
    }

    /// `EnSkj_NeedleRecover`.
    fn needle_recover(&mut self) {
        if self.skel.cur_frame == 0.0 {
            self.setup_stand();
        }
    }

    /// `EnSkj_SetupSpawnDeathEffect`.
    fn setup_spawn_death_effect(&mut self) {
        self.backflip_flag = 1;
        self.setup_action(SKJ_ACTION_SPAWN_DEATH_EFFECT);
    }

    /// `EnSkj_SpawnDeathEffect`: fading 4 a frame, a blue flame (`EffectSsDeadDb`) within 15 of
    /// him each frame.
    fn spawn_death_effect(&mut self, play: &mut PlayState) {
        let mut phi_v0 = self.alpha.wrapping_sub(4);
        if phi_v0 > 255 {
            phi_v0 = 0;
        }
        self.alpha = phi_v0;
        self.actor.shape_shadow_alpha = phi_v0 as u8;
        let x = play.rand.centered_float(30.0) + self.actor.world_pos.x;
        let y = play.rand.centered_float(30.0) + self.actor.world_pos.y;
        let z = play.rand.centered_float(30.0) + self.actor.world_pos.z;
        let effect_pos = Vec3::new(x, y, z);
        play.with_ss(|s| s.dead_db_spawn(effect_pos, Vec3::ZERO, Vec3::ZERO, 100, 10, [255, 255, 255, 255], [0, 0, 255], 1, 9, 1));
    }

    /// `EnSkj_SetupWaitInRange`.
    fn setup_wait_in_range(&mut self) {
        self.text_id = 0x10BC;
        self.change_anim(SKJ_ANIM_WAIT);
        self.setup_action(SKJ_ACTION_SARIA_SONG_WAIT_IN_RANGE);
    }

    /// `EnSkj_WaitInRange`: Link on the small stump. The ocarina out: Link centred and turned,
    /// the song's check; a talk accepted: the trade (the Skull Mask worn, not yet sold) or the
    /// talk; off the stump: back to his flute; else his text by the save and the mask, offered
    /// once he's heard Saria's Song.
    fn wait_in_range(&mut self, play: &mut PlayState) {
        let stump = Self::small_stump(play);
        let stump_pos = stump.map(|s| s.1);
        if player_state2(play) & STATE2_24 != 0 {
            // When Link pulls out the ocarina, center him on the stump. (Link was probably supposed
            // to be pointed towards the Skull Kid as well.)
            set_player_state2(play, STATE2_25);
            set_player_unk_6a8(play, stump.map(|s| s.0));
            if let Some((_, pos, rot_y)) = stump {
                set_player_pos(play, pos);
                turn_player(play, rot_y);
            }
            play.message_start_ocarina_suns_song_disabled(OCARINA_ACTION_CHECK_SARIA);
            self.setup_wait_for_song();
        } else if statics(play).d_80b01ea0 != 0 {
            if let Some(pos) = stump_pos {
                set_player_pos(play, pos);
            }
            if player_get_mask(play) == PLAYER_MASK_SKULL && !play.save.get_item_get_inf(ITEMGETINF_39) {
                play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
                self.setup_mask_trade();
            } else {
                self.setup_talk();
            }
        } else if !player(play).map(|p| p.actor.world_pos).zip(stump_pos).is_some_and(|(pp, sp)| range_check(pp, sp)) {
            self.setup_reset_fight();
        } else {
            set_player_state2(play, STATE2_23);
            if play.save.get_item_get_inf(ITEMGETINF_16) {
                if play.save.get_item_get_inf(ITEMGETINF_39) {
                    self.text_id = mask_reaction_get_text_id(play, MASK_REACTION_SET_SKULL_KID);
                    if self.text_id == 0 {
                        self.text_id = 0x1020;
                    }
                } else if player_get_mask(play) == PLAYER_MASK_NONE {
                    self.text_id = 0x10BC;
                } else if player_get_mask(play) == PLAYER_MASK_SKULL {
                    self.text_id = 0x101B;
                } else {
                    self.text_id = mask_reaction_get_text_id(play, MASK_REACTION_SET_SKULL_KID);
                }
                self.offer_talk_from_stump(play);
            }
        }
    }

    /// `EnSkj_SetupWaitForSong`.
    fn setup_wait_for_song(&mut self) {
        self.unk_2d6 = 0;
        self.setup_action(SKJ_ACTION_SARIA_SONG_WAIT_FOR_SONG);
    }

    /// `EnSkj_WaitForSong`: the check's outcome (`msgCtx.ocarinaMode`): another song before
    /// `ITEMGETINF_16` (the box failing), 0x1041; he plays along while the song's played back;
    /// put away, waiting again; Saria's Song the first time, the chime and 0x10BB; after that, any
    /// song, 0x10BD.
    fn wait_for_song(&mut self, play: &mut PlayState) {
        let msg_mode = play.msg_ctx.msg_mode;
        // Played a song that's not Saria's Song.
        if !play.save.get_item_get_inf(ITEMGETINF_16) && (msg_mode == MSGMODE_OCARINA_FAIL || msg_mode == MSGMODE_OCARINA_FAIL_NO_TEXT) {
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            message_close_textbox(play);
            let me = play.cur_actor;
            set_player_unk_6a8(play, me);
            self.offer_talk_from_stump(play);
            self.setup_wrong_song();
            return;
        }
        if msg_mode == MSGMODE_OCARINA_CORRECT_PLAYBACK && self.unk_2d6 == 0 {
            self.unk_2d6 = 1;
            self.change_anim(SKJ_ANIM_PLAY_FLUTE);
        } else if self.unk_2d6 != 0 && msg_mode == MSGMODE_SONG_DEMONSTRATION_DONE {
            self.unk_2d6 = 0;
            self.change_anim(SKJ_ANIM_WAIT);
        }
        let ocarina_mode = play.msg_ctx.ocarina_mode;
        if ocarina_mode == OCARINA_MODE_04 {
            play.msg_ctx.ocarina_mode = OCARINA_MODE_00;
            self.unk_2d6 = 0;
            self.change_anim(SKJ_ANIM_WAIT);
            self.setup_action(SKJ_ACTION_SARIA_SONG_WAIT_IN_RANGE);
        } else if ocarina_mode == OCARINA_MODE_03 {
            if !play.save.get_item_get_inf(ITEMGETINF_16) {
                // Saria's Song has been played for the first time.
                play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
                play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
                let me = play.cur_actor;
            set_player_unk_6a8(play, me);
                self.offer_talk_from_stump(play);
                self.text_id = 0x10BB;
                self.setup_after_song();
            } else {
                play.msg_ctx.ocarina_mode = OCARINA_MODE_05;
            }
        } else if ocarina_mode == OCARINA_MODE_02 {
            if let Some(p) = player_mut(play) {
                p.state2 &= !STATE2_24;
            }
            self.actor.kill();
        } else if ocarina_mode == OCARINA_MODE_01 {
            set_player_state2(play, STATE2_23);
        } else if ocarina_mode >= OCARINA_MODE_05 {
            play.save.suns_song_state = oot_game::clock::SUNSSONG_INACTIVE;
            if play.save.get_item_get_inf(ITEMGETINF_16) {
                play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
                let me = play.cur_actor;
            set_player_unk_6a8(play, me);
                self.offer_talk_from_stump(play);
                self.text_id = 0x10BD;
                self.setup_after_song();
            } else {
                play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
                let me = play.cur_actor;
            set_player_unk_6a8(play, me);
                self.offer_talk_from_stump(play);
                self.setup_wrong_song();
            }
        }
    }

    /// `EnSkj_SetupAfterSong`.
    fn setup_after_song(&mut self) {
        self.unk_2d6 = 0;
        self.change_anim(SKJ_ANIM_WAIT);
        self.setup_action(SKJ_ACTION_SARIA_SONG_AFTER_SONG);
    }

    /// `EnSkj_AfterSong`: his text offered until it's taken.
    fn after_song(&mut self, play: &mut PlayState) {
        if statics(play).d_80b01ea0 != 0 {
            self.setup_talk();
        } else {
            self.offer_talk_from_stump(play);
        }
    }

    /// `EnSkj_SetupTalk`.
    fn setup_talk(&mut self) {
        self.setup_action(SKJ_ACTION_SARIA_TALK);
    }

    /// `EnSkj_SariaSongTalk`: the text read: waiting again, or (before `ITEMGETINF_16`) the
    /// piece of heart.
    fn saria_song_talk(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            if play.save.get_item_get_inf(ITEMGETINF_16) {
                self.setup_wait_in_range();
            } else {
                self.func_80affe24();
                self.offer_heart_piece(play);
            }
        }
    }

    /// `func_80AFFE24`.
    fn func_80affe24(&mut self) {
        self.setup_action(SKJ_ACTION_UNK14);
    }

    /// `func_80AFFE44`: the piece of heart offered until Link has it.
    fn func_80affe44(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.setup_post_sarias_song();
        } else {
            self.offer_heart_piece(play);
        }
    }

    /// `EnSkj_SetupPostSariasSong`.
    fn setup_post_sarias_song(&mut self) {
        self.setup_action(SKJ_ACTION_SARIA_SONG_CHANGE_MODE);
    }

    /// `EnSkj_ChangeModeAfterSong`: the item's text read: `ITEMGETINF_16`.
    fn change_mode_after_song(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            play.save.set_item_get_inf(ITEMGETINF_16);
            self.set_navi_id(play);
            self.setup_wait_in_range();
        }
    }

    /// `EnSkj_SetupMaskTrade`.
    fn setup_mask_trade(&mut self) {
        self.setup_action(SKJ_ACTION_SARIA_SONG_START_TRADE);
    }

    /// `EnSkj_StartMaskTrade`: Link held (`PLAYER_CSACTION_1`) while the text's read; then he jumps
    /// down.
    fn start_mask_trade(&mut self, play: &mut PlayState) {
        let sp1f = play.message_state();
        play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_1);
        if sp1f == TEXT_STATE_DONE && play.message_should_advance() {
            self.jump_from_stump();
        }
    }

    /// `EnSkj_JumpFromStump`: the backflip played backwards, forward at 2 and 8 up.
    fn jump_from_stump(&mut self) {
        self.actor.velocity.y = 8.0;
        self.actor.speed_xz = 2.0;
        self.change_anim(SKJ_ANIM_BACKFLIP);
        self.animation_reverse();
        self.skel.cur_frame = self.skel.start_frame;
        self.setup_action(SKJ_ACTION_SARIA_SONG_WAIT_FOR_LANDING);
    }

    /// `EnSkj_WaitForLanding`.
    fn wait_for_landing(&mut self) {
        if self.actor.velocity.y <= 0.0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND_TOUCH;
            self.actor.speed_xz = 0.0;
            self.setup_wait_for_land_anim_finish();
        }
    }

    /// `EnSkj_SetupWaitForLandAnimFinish`.
    fn setup_wait_for_land_anim_finish(&mut self) {
        self.change_anim(SKJ_ANIM_LAND);
        self.setup_action(SKJ_ACTION_SARIA_SONG_WAIT_FOR_LANDING_ANIM);
    }

    /// `EnSkj_WaitForLandAnimFinish`.
    fn wait_for_land_anim_finish(&mut self) {
        if self.on_last_frame(SKJ_ANIM_LAND) {
            self.setup_walk_to_player();
        }
    }

    /// `EnSkj_SetupWalkToPlayer`.
    fn setup_walk_to_player(&mut self) {
        self.unk_2f0 = 0.0;
        self.actor.speed_xz = 2.0;
        self.change_anim(SKJ_ANIM_WALK_TO_PLAYER);
        self.setup_action(SKJ_ACTION_SARIA_SONG_WALK_TO_PLAYER);
    }

    /// `EnSkj_WalkToPlayer`: to within 120 of Link, turning to him.
    fn walk_to_player(&mut self, play: &mut PlayState) {
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 0xA, self.unk_2f0 as i32 as i16, 0);
        approach_f(&mut self.unk_2f0, 2000.0, 1.0, 100.0);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        if self.actor.xz_dist_to_player < 120.0 {
            self.actor.speed_xz = 0.0;
            self.setup_ask_for_mask(play);
        }
    }

    /// `EnSkj_SetupAskForMask`: 0x101C.
    fn setup_ask_for_mask(&mut self, play: &mut PlayState) {
        play.start_textbox(0x101C, play.cur_actor);
        self.change_anim(SKJ_ANIM_WAIT);
        self.setup_action(SKJ_ACTION_SARIA_SONG_ASK_FOR_MASK);
    }

    /// `EnSkj_AskForMask`: yes, 0x101E; no, 0x101D.
    fn ask_for_mask(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_CHOICE && play.message_should_advance() {
            match play.msg_ctx.choice_index {
                // Yes.
                0 => self.setup_take_mask(play),
                // No.
                1 => {
                    play.continue_textbox(0x101D);
                    self.setup_wait_for_mask_text_clear();
                }
                _ => {}
            }
        }
    }

    /// `EnSkj_SetupTakeMask`.
    fn setup_take_mask(&mut self, play: &mut PlayState) {
        play.continue_textbox(0x101E);
        self.setup_action(SKJ_ACTION_SARIA_SONG_TAKE_MASK);
    }

    /// `EnSkj_TakeMask`: 10 rupees, `ITEMGETINF_39`, the mask off and sold out, 0x101F.
    fn take_mask(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            rupees_change_by(&mut play.save, 10);
            play.save.set_item_get_inf(ITEMGETINF_39);
            self.set_navi_id(play);
            // Player_UnsetMask: masks aren't ported (BACKLOG #39); Player's currentMask cleared.
            log::debug!("En_Skj: Player_UnsetMask (the Skull Mask taken)");
            if let Some(p) = player_mut(play) {
                oot_game::actor_ctx::PlayerIface::unset_mask(p);
            }
            item_give(&mut play.save, Some(&mut play.audio), ITEM_SOLD_OUT);
            play.continue_textbox(0x101F);
            self.setup_wait_for_mask_text_clear();
        }
    }

    /// `EnSkj_SetupWaitForMaskTextClear`.
    fn setup_wait_for_mask_text_clear(&mut self) {
        self.setup_action(SKJ_ACTION_SARIA_SONG_WAIT_MASK_TEXT);
    }

    /// `EnSkj_WaitForMaskTextClear`: Link let go (`PLAYER_CSACTION_7`), and he backflips away.
    fn wait_for_mask_text_clear(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_7);
            self.backflip_flag = 1;
            self.backflip();
        }
    }

    /// `EnSkj_SetupWrongSong`: 0x1041.
    fn setup_wrong_song(&mut self) {
        self.text_id = 0x1041;
        self.change_anim(SKJ_ANIM_WAIT);
        self.setup_action(SKJ_ACTION_SARIA_SONG_WRONG_SONG);
    }

    /// `EnSkj_WrongSong`.
    fn wrong_song(&mut self, play: &mut PlayState) {
        if statics(play).d_80b01ea0 != 0 {
            self.setup_wait_for_text_clear();
        } else {
            self.offer_talk_from_stump(play);
        }
    }

    /// `EnSkj_SetupWaitForTextClear`.
    fn setup_wait_for_text_clear(&mut self) {
        self.setup_action(SKJ_ACTION_SARIA_SONG_WAIT_FOR_TEXT);
    }

    /// `EnSkj_SariasSongWaitForTextClear`: read, waiting again, the ocarina asked for to the
    /// stump.
    fn sarias_song_wait_for_text_clear(&mut self, play: &mut PlayState) {
        let state = play.message_state();
        if state == TEXT_STATE_DONE && play.message_should_advance() {
            self.setup_wait_in_range();
            set_player_state2(play, STATE2_23);
            let stump = statics(play).s_small_stump_skull_kid.skullkid;
            set_player_unk_6a8(play, stump);
        }
    }

    /// `EnSkj_OcarinaGameSetupWaitForPlayer`.
    fn ocarina_game_setup_wait_for_player(&mut self) {
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.change_anim(SKJ_ANIM_WAIT);
        self.setup_action(SKJ_ACTION_OCARINA_GAME_WAIT_FOR_PLAYER);
    }

    /// `EnSkj_OcarinaGameWaitForPlayer`: Link on the stump once, targetable.
    fn ocarina_game_wait_for_player(&mut self) {
        if self.player_in_range != 0 {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            self.setup_action(SKJ_ACTION_OCARINA_GAME_IDLE);
        }
    }

    /// `EnSkj_IsLeavingGame`: his static's `unk_0` 2: he leaves.
    fn is_leaving_game(&mut self, play: &mut PlayState) -> bool {
        let param_decr = (self.params() - 1) as usize;
        if statics(play).s_ocarina_minigame_skull_kids.get(param_decr).is_some_and(|k| k.unk_0 == 2) {
            self.setup_leave_ocarina_game();
            return true;
        }
        false
    }

    /// `EnSkj_SetupIdle`.
    fn setup_idle(&mut self) {
        self.change_anim(SKJ_ANIM_WAIT);
        self.setup_action(SKJ_ACTION_OCARINA_GAME_IDLE);
    }

    /// `EnSkj_Appear`: 20 more alpha a frame, to 255.
    fn appear(&mut self) {
        if self.alpha != 255 {
            self.alpha += 20;
            if self.alpha > 255 {
                self.alpha = 255;
            }
        }
    }

    /// `EnSkj_OcarinaGameIdle`.
    fn ocarina_game_idle(&mut self, play: &mut PlayState) {
        self.appear();
        if !self.is_leaving_game(play) && self.minigame_state != 0 {
            self.setup_play_ocarina_game();
        }
    }

    /// `EnSkj_SetupPlayOcarinaGame`.
    fn setup_play_ocarina_game(&mut self) {
        self.change_anim(SKJ_ANIM_PLAY_FLUTE);
        self.setup_action(SKJ_ACTION_OCARINA_GAME_PLAY);
    }

    /// `EnSkj_PlayOcarinaGame`.
    fn play_ocarina_game(&mut self, play: &mut PlayState) {
        self.appear();
        if !self.is_leaving_game(play) && self.minigame_state == 0 {
            self.setup_idle();
        }
    }

    /// `EnSkj_SetupLeaveOcarinaGame`.
    fn setup_leave_ocarina_game(&mut self) {
        self.actor.velocity.y = 8.0;
        self.actor.speed_xz = -8.0;
        self.change_anim(SKJ_ANIM_BACKFLIP);
        self.setup_action(SKJ_ACTION_OCARINA_GAME_LEAVE);
    }

    /// `EnSkj_LeaveOcarinaGame`: out of the statics, and the backflip away.
    fn leave_ocarina_game(&mut self, play: &mut PlayState) {
        let params_decr = (self.params() - 1) as usize;
        if let Some(k) = statics(play).s_ocarina_minigame_skull_kids.get_mut(params_decr) {
            k.unk_0 = 0;
            k.skullkid = None;
            k.pending = 0;
        }
        self.backflip_flag = 1;
        self.backflip();
    }

    /// `EnSkj_SetupWaitForOcarina` (the stump's action, every frame): Link on it, the Skull Kids
    /// told; the ocarina out, 0x10BE and the nature ambience, else waiting for it.
    fn setup_wait_for_ocarina(&mut self, play: &mut PlayState) {
        let pp = player(play).map(|p| p.actor.world_pos);
        if pp.is_some_and(|pp| range_check(pp, self.actor.world_pos)) {
            for side in [SKULL_KID_LEFT, SKULL_KID_RIGHT] {
                if let Some(k) = Self::minigame_kid(play, side) {
                    k.player_in_range = 1;
                }
            }
            if player_state2(play) & STATE2_24 != 0 {
                self.ocarina_out(play);
            } else {
                self.func = Func::WaitForOcarina;
            }
        }
    }

    /// The ocarina out on the stump (`EnSkj_SetupWaitForOcarina`, `EnSkj_WaitForOcarina`):
    /// `PLAYER_STATE2_25`, the nature ambience (`func_800F5BF0(NATURE_ID_KOKIRI_REGION)`), Link
    /// turned, 0x10BE.
    fn ocarina_out(&mut self, play: &mut PlayState) {
        set_player_state2(play, STATE2_25);
        play.audio.func_800f5bf0(NATURE_ID_KOKIRI_REGION);
        turn_player(play, self.actor.world_rot.y);
        let me = play.cur_actor;
            set_player_unk_6a8(play, me);
        play.start_textbox(0x10BE, play.cur_actor);
        self.func = Func::StartOcarinaMinigame;
    }

    /// `EnSkj_WaitForOcarina`.
    fn wait_for_ocarina(&mut self, play: &mut PlayState) {
        if player_state2(play) & STATE2_24 != 0 {
            self.ocarina_out(play);
        } else if player(play).is_some_and(|p| range_check(p.actor.world_pos, self.actor.world_pos)) {
            set_player_state2(play, STATE2_23);
        }
    }

    /// `EnSkj_StartOcarinaMinigame`: 0x10BE closing: the memory game, the left Skull Kid playing,
    /// 160 frames.
    fn start_ocarina_minigame(&mut self, play: &mut PlayState) {
        let dialog_state = play.message_state();
        turn_player(play, self.actor.world_rot.y);
        if dialog_state == TEXT_STATE_CLOSING {
            play.message_start_ocarina(OCARINA_ACTION_MEMORY_GAME);
            Self::set_minigame_state(play, SKULL_KID_LEFT, SKULL_KID_OCARINA_PLAY_NOTES);
            self.multiuse_timer = 160;
            self.func = Func::WaitForPlayback;
        }
    }

    /// `EnSkj_WaitForPlayback`: the memory game, through the message box's modes.
    fn wait_for_playback(&mut self, play: &mut PlayState) {
        turn_player(play, self.actor.world_rot.y);
        if play.msg_ctx.ocarina_mode == OCARINA_MODE_03 {
            // Failed the game.
            message_close_textbox(play);
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            let me = play.cur_actor;
            set_player_unk_6a8(play, me);
            offer_talk(play, &self.actor, 26.0);
            self.text_id = 0x102D;
            self.func = Func::FailedMiniGame;
        } else if play.msg_ctx.ocarina_mode == OCARINA_MODE_0F {
            // Completed the game.
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            message_close_textbox(play);
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            let me = play.cur_actor;
            set_player_unk_6a8(play, me);
            offer_talk(play, &self.actor, 26.0);
            self.text_id = 0x10BF;
            self.func = Func::WonOcarinaMiniGame;
        } else {
            // Playing the game.
            match play.msg_ctx.msg_mode {
                MSGMODE_MEMORY_GAME_LEFT_SKULLKID_WAIT => {
                    Self::set_minigame_state(play, SKULL_KID_LEFT, SKULL_KID_OCARINA_WAIT);
                    if !play.audio.is_sfx_playing(NA_SE_SY_METRONOME) {
                        Self::set_minigame_state(play, SKULL_KID_RIGHT, SKULL_KID_OCARINA_PLAY_NOTES);
                        play.message_update_ocarina_memory_game();
                    }
                }
                MSGMODE_MEMORY_GAME_RIGHT_SKULLKID_WAIT => {
                    Self::set_minigame_state(play, SKULL_KID_RIGHT, SKULL_KID_OCARINA_WAIT);
                    if !play.audio.is_sfx_playing(NA_SE_SY_METRONOME) {
                        play.message_update_ocarina_memory_game();
                        self.multiuse_timer = 160;
                    }
                }
                MSGMODE_MEMORY_GAME_PLAYER_PLAYING => {
                    if self.multiuse_timer != 0 {
                        self.multiuse_timer -= 1;
                    } else {
                        // Took too long, the game failed.
                        play.audio.play_sfx_centered(NA_SE_SY_OCARINA_ERROR);
                        message_close_textbox(play);
                        play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
                        let me = play.cur_actor;
            set_player_unk_6a8(play, me);
                        offer_talk(play, &self.actor, 26.0);
                        self.text_id = 0x102D;
                        self.func = Func::FailedMiniGame;
                    }
                }
                MSGMODE_MEMORY_GAME_START_NEXT_ROUND => {
                    if !play.audio.is_sfx_playing(NA_SE_SY_METRONOME) {
                        Self::set_minigame_state(play, SKULL_KID_LEFT, SKULL_KID_OCARINA_PLAY_NOTES);
                        self.multiuse_timer = 160;
                        play.audio.audio_ocarina_set_instrument(OCARINA_INSTRUMENT_FLUTE);
                        play.audio.audio_ocarina_set_playback_song(OCARINA_SONG_MEMORY_GAME as i8 + 1, 1);
                        play.msg_ctx.msg_mode = MSGMODE_MEMORY_GAME_LEFT_SKULLKID_PLAYING;
                        play.msg_ctx.state_timer = 2;
                    }
                }
                _ => {}
            }
        }
    }

    /// `EnSkj_FailedMiniGame`: 0x102D offered until it's taken.
    fn failed_mini_game(&mut self, play: &mut PlayState) {
        if statics(play).d_80b01ea0 != 0 {
            self.func = Func::WaitForNextRound;
        } else {
            offer_talk(play, &self.actor, 26.0);
        }
    }

    /// `EnSkj_WaitForNextRound`.
    fn wait_for_next_round(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            self.offer_next_round(play);
        }
    }

    /// `EnSkj_OfferNextRound`: 0x102E.
    fn offer_next_round(&mut self, play: &mut PlayState) {
        play.continue_textbox(0x102E);
        self.func = Func::WaitForOfferResponse;
    }

    /// `EnSkj_WaitForOfferResponse`: yes, the ocarina out as the box closes
    /// (`PLAYER_STATE3_5`) and the game again; no, the end.
    fn wait_for_offer_response(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_CHOICE && play.message_should_advance() {
            match play.msg_ctx.choice_index {
                // Yes.
                0 => {
                    // Makes Player take the ocarina out right away after closing the box.
                    if let Some(p) = player_mut(play) {
                        p.state3 |= STATE3_5;
                    }
                    self.func = Func::SetupWaitForOcarina;
                }
                // No.
                1 => self.func = Func::CleanupOcarinaGame,
                _ => {}
            }
        }
    }

    /// `EnSkj_WonOcarinaMiniGame`: 0x10BF offered until it's taken.
    fn won_ocarina_mini_game(&mut self, play: &mut PlayState) {
        if statics(play).d_80b01ea0 != 0 {
            self.func = Func::WaitToGiveReward;
        } else {
            offer_talk(play, &self.actor, 26.0);
        }
    }

    /// `sOcarinaGameRewards[gSaveContext.save.info.playerData.ocarinaGameRoundNum]`.
    fn reward(play: &PlayState) -> i16 {
        S_OCARINA_GAME_REWARDS[play.save.ocarina_game_round_num as usize]
    }

    /// `EnSkj_WaitToGiveReward`.
    fn wait_to_give_reward(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            let gi = Self::reward(play);
            offer_get_item_range(play, &self.actor, gi, 26.0, 26.0);
            self.func = Func::GiveOcarinaGameReward;
        }
    }

    /// `EnSkj_GiveOcarinaGameReward`.
    fn give_ocarina_game_reward(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.func = Func::FinishOcarinaGameRound;
        } else {
            let gi = Self::reward(play);
            offer_get_item_range(play, &self.actor, gi, 26.0, 26.0);
        }
    }

    /// `EnSkj_FinishOcarinaGameRound`: the item's text read: the round counted (to 3); the third
    /// round, `ITEMGETINF_17` and the end; else 0x102E.
    fn finish_ocarina_game_round(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            let ocarina_game_round_num = play.save.ocarina_game_round_num;
            if play.save.ocarina_game_round_num < 3 {
                play.save.ocarina_game_round_num += 1;
            }
            if ocarina_game_round_num == 2 {
                play.save.set_item_get_inf(ITEMGETINF_17);
                self.func = Func::CleanupOcarinaGame;
            } else {
                self.offer_next_round(play);
            }
        }
    }

    /// `EnSkj_CleanupOcarinaGame`: the Skull Kids told to leave; once both are, the music back
    /// (`func_800F5C2C`) and the stump gone.
    fn cleanup_ocarina_game(&mut self, play: &mut PlayState) {
        let s = statics(play);
        for side in [SKULL_KID_LEFT, SKULL_KID_RIGHT] {
            if s.s_ocarina_minigame_skull_kids[side].skullkid.is_some() {
                s.s_ocarina_minigame_skull_kids[side].unk_0 = 2;
            }
        }
        if s.s_ocarina_minigame_skull_kids[SKULL_KID_LEFT].unk_0 == 2 && s.s_ocarina_minigame_skull_kids[SKULL_KID_RIGHT].unk_0 == 2 {
            play.audio.func_800f5c2c();
            self.actor.kill();
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.func {
            Func::None => {}
            Func::Fade => self.fade(play),
            Func::WaitToShootNeedle => self.wait_to_shoot_needle(play),
            Func::SariasSongKidIdle => self.sarias_song_kid_idle(play),
            Func::WaitForDeathAnim => self.wait_for_death_anim(),
            Func::PickNextFightAction => self.pick_next_fight_action(),
            Func::WaitForLandAnim => self.wait_for_land_anim(),
            Func::ResetFight => self.reset_fight(play),
            Func::Fight => self.fight(play),
            Func::NeedleRecover => self.needle_recover(),
            Func::SpawnDeathEffect => self.spawn_death_effect(play),
            Func::WaitInRange => self.wait_in_range(play),
            Func::WaitForSong => self.wait_for_song(play),
            Func::AfterSong => self.after_song(play),
            Func::SariaSongTalk => self.saria_song_talk(play),
            Func::Func80affe44 => self.func_80affe44(play),
            Func::ChangeModeAfterSong => self.change_mode_after_song(play),
            Func::StartMaskTrade => self.start_mask_trade(play),
            Func::WaitForLanding => self.wait_for_landing(),
            Func::WaitForLandAnimFinish => self.wait_for_land_anim_finish(),
            Func::WalkToPlayer => self.walk_to_player(play),
            Func::AskForMask => self.ask_for_mask(play),
            Func::TakeMask => self.take_mask(play),
            Func::WaitForMaskTextClear => self.wait_for_mask_text_clear(play),
            Func::WrongSong => self.wrong_song(play),
            Func::SariasSongWaitForTextClear => self.sarias_song_wait_for_text_clear(play),
            Func::OcarinaGameWaitForPlayer => self.ocarina_game_wait_for_player(),
            Func::OcarinaGameIdle => self.ocarina_game_idle(play),
            Func::PlayOcarinaGame => self.play_ocarina_game(play),
            Func::LeaveOcarinaGame => self.leave_ocarina_game(play),
            Func::SetupWaitForOcarina => self.setup_wait_for_ocarina(play),
            Func::WaitForOcarina => self.wait_for_ocarina(play),
            Func::StartOcarinaMinigame => self.start_ocarina_minigame(play),
            Func::WaitForPlayback => self.wait_for_playback(play),
            Func::FailedMiniGame => self.failed_mini_game(play),
            Func::WaitForNextRound => self.wait_for_next_round(play),
            Func::WaitForOfferResponse => self.wait_for_offer_response(play),
            Func::WonOcarinaMiniGame => self.won_ocarina_mini_game(play),
            Func::WaitToGiveReward => self.wait_to_give_reward(play),
            Func::GiveOcarinaGameReward => self.give_ocarina_game_reward(play),
            Func::FinishOcarinaGameRound => self.finish_ocarina_game_round(play),
            Func::CleanupOcarinaGame => self.cleanup_ocarina_game(play),
        }
    }

    /// `EnSkj_Update`.
    fn update_skull_kid(&mut self, play: &mut PlayState) {
        statics(play).d_80b01ea0 = process_talk_request(&mut self.actor) as i32;
        self.timer = self.timer.wrapping_add(1);
        if self.multiuse_timer != 0 {
            self.multiuse_timer -= 1;
        }
        if self.needle_shoot_timer != 0 {
            self.needle_shoot_timer -= 1;
        }
        if self.dodge_reset_timer != 0 {
            self.dodge_reset_timer -= 1;
        }
        if self.dodge_reset_timer == 0 {
            self.hits_until_dodge = 3;
        }
        if self.backflip_flag != 0 && self.alpha == 0 {
            if self.action == SKJ_ACTION_SPAWN_DEATH_EFFECT {
                let drop_pos = self.actor.world_pos;
                item_drop_collectible(play, drop_pos, ITEM00_RUPEE_ORANGE);
            }
            self.actor.kill();
            return;
        }
        self.actor.set_focus(30.0);
        self.actor.scale = Vec3::splat(0.01);
        self.run_action(play);
        self.actor.text_id = self.text_id;
        self.collision_check(play);
        self.collider.update(&self.actor);
        if self.unk_2d3 != 0 && statics(play).d_80b01ea0 == 0 {
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            if self.actor.color_filter_timer == 0 {
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            }
        }
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.skel.update();
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 20.0, 20.0, 20.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2);
    }

    /// `EnSkj_SariasSongShortStumpUpdate` (the debug display aside: `BREG(0)` 0).
    fn sarias_song_short_stump_update(&mut self, play: &mut PlayState) {
        statics(play).d_80b01ea0 = process_talk_request(&mut self.actor) as i32;
    }

    /// `EnSkj_OcarinaMinigameShortStumpUpdate`: its focus fixed at (1230, -90, 450), its action,
    /// its text, and `xzDistToPlayer` 50 for the rest of the frame (the debug display aside).
    fn ocarina_minigame_short_stump_update(&mut self, play: &mut PlayState) {
        statics(play).d_80b01ea0 = process_talk_request(&mut self.actor) as i32;
        self.timer = self.timer.wrapping_add(1);
        self.actor.focus_pos = Vec3::new(1230.0, -90.0, 450.0);
        self.run_action(play);
        self.actor.text_id = self.text_id;
        self.actor.xz_dist_to_player = 50.0;
    }
}

/// `EnSkj_TurnPlayer`: Player turned towards `world_rot_y` (the stump's facing), a fifth of the
/// way and at most 2000 a frame, its `yaw` with it.
fn turn_player(play: &mut PlayState, world_rot_y: i16) {
    if let Some(p) = player_mut(play) {
        smooth_step_to_s(&mut p.actor.shape_rot.y, world_rot_y, 5, 2000, 0);
        p.actor.world_rot.y = p.actor.shape_rot.y;
        p.current_yaw = p.actor.shape_rot.y;
    }
}

impl ActorImpl for EnSkj {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `actor.update`: `EnSkj_Update`, or the stumps' own.
    fn update(&mut self, play: &mut PlayState) {
        self.resolve_statics(play);
        match self.update_fn {
            UpdateFn::Update => self.update_skull_kid(play),
            UpdateFn::SariasSongShortStump => self.sarias_song_short_stump_update(play),
            UpdateFn::OcarinaMinigameShortStump => self.ocarina_minigame_short_stump_update(play),
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.values = vec![self.alpha.min(255) as f32];
        rs
    }

    /// `EnSkj_Draw` (none for types 5 and 6): the skeleton with segment 0x0C's alpha (translucent
    /// below 255), the Skull Mask on limb 11 once it's sold (`EnSkj_PostLimbDraw`).
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if !self.has_draw {
            return;
        }
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let [alpha] = rs.values.as_slice() else { return };
        let alpha = *alpha as u32;
        // EnSkj_OverrideLimbDraw changes nothing.
        let bones = skeleton.pose_override(&joints.rot, |_, _, _| Mat4::IDENTITY);
        let model = actor_draw_matrix(rs);
        // EnSkj_PostLimbDraw: limb 11 with ITEMGETINF_39, gSkullKidSkullMaskDL after
        // Gfx_SetupDL_25Opa, Matrix_RotateZYX(-0x4000, 0, 0, MTXMODE_APPLY).
        if play.save.get_item_get_inf(ITEMGETINF_39)
            && let Some(limb) = bones.get(SKJ_LIMB_MASK - 1)
        {
            let m = model * *limb * Mat4::from_rotation_x(oot_game::sys_matrix::binang_to_rad(-0x4000));
            out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, "gSkullKidSkullMaskDL")), m));
        }
        let xlu = alpha < 255;
        let mut sv = SegmentValues::default();
        sv.env[SEG_ALPHA as usize] = Some([0, 0, 0, alpha as u8]);
        let cmd = DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(xlu))), transform: model, bones, params: eng_gfx::DrawParams { segments: Some(sv), ..Default::default() } };
        if xlu {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
    }

    /// `EnSkj_Destroy` (`Collider_DestroyCylinder`: nothing to free here; types 5 and 6 have
    /// none), then the overlay's unload when it's the last.
    fn destroy(&mut self, play: &mut PlayState) {
        actor_free_overlay(play);
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
