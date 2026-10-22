//! `Boss_Va` (`ovl_Boss_Va/z_boss_va.c`): Barinade, Jabu-Jabu's Belly's boss (`SCENE_JABU_JABU_BOSS`,
//! room 1: the body placed at (10, 0, -220)), and every part of him, each its own instance of the
//! overlay by params (`BossVaParam`): the body (-1), three supports holding him to the ceiling
//! (0 to 2), three zappers on his sides (3 to 5), ten Bari (6 to 15: five upper, five lower), the
//! stumps a cut support leaves (16 to 18) and the room's door (19). The overlay's statics
//! (`sCsState`, `sFightPhase`, `sBodyState`, `sDoorState`, `sSubCamId`, the sub camera's vectors,
//! `sKillBari`, `sBodyBari`, `sPhase2Timer`, `sPhase4HP`, the 400 particles) are shared by every
//! instance (`Statics`, `PlayState::overlay_statics`), dropped with the last instance as the
//! overlay unloads.
//!
//! - **The body's init** (`BossVa_Init`): the door (`BOSSVA_DOOR`, its child, at (0, 80, 400)); a
//!   cleared room has `En_Ru1` type 0 (or, with `EVENTCHKINF_37`, `Door_Warp1` 0) at him and the
//!   heart container (`Item_B_Heart`) 160 east, the door shut (`sDoorState` 100), and he's gone.
//!   Else his damage table, the supports and zappers (his children, at `sInitPosOffsets` and
//!   `sInitRot`), and the intro from its start (`INTRO_START`, the door open at 5) or, with
//!   `EVENTCHKINF_BEGAN_BARINADE_BATTLE` set, from `INTRO_CALL_BARI` with the Bari spawned at once,
//!   the door shut, Link held (`PLAYER_CSACTION_1`) and the sub camera on him.
//! - **The intro** (`BossVa_BodyIntro`, a manual cutscene on his sub camera): Link turned north and
//!   held (`PLAYER_CSACTION_8`), the door closing behind him (8 a frame, `NA_SE_EV_BUYODOOR_CLOSE` at
//!   29), Link turning (`_2`), the ten Bari (his children) at their cutscene places; the first rises
//!   to 60 and the camera turns to the supports; the Bari called to him one by one (`sBodyBari`:
//!   `NA_SE_EN_BALINADE_STICK`, the room's light flashing, `BossVa_SetSparkEnv`), the camera
//!   circling him, `envCtx.lightSettingOverride` 1, `NA_BGM_BOSS`; his title card the first time
//!   (`gBarinadeTitleCardTex`, 128 by 40 of its PAL 128 by 120), sparks, the camera back
//!   (`PLAYER_CSACTION_7`) and `EVENTCHKINF_BEGAN_BARINADE_BATTLE`.
//! - **The fight** (`sFightPhase`): phase 1, only his supports can be hit (the boomerang:
//!   `DMG_BOOMERANG`), each cut (`BossVa_SetupSupportCut`, a stump at its tip, a quake) tilting him
//!   and taking him 60 lower; the zappers shoot lightning at Link (or where his boomerang will be)
//!   unless he's moving. With three cut (`PHASE_2`, 3) the upper Bari circle him on their tethers;
//!   the boomerang stuns him (blue, 160 frames: then a sword, the boomerang or a stick hit kills a
//!   Bari, `sKillBari`) and spark balls rain from his top. With the five dead (`PHASE_3`, 9) he
//!   chases Link spinning, the lower Bari spawned round him; stunned by the boomerang (131 frames)
//!   they stun too and fall to a hit. With all ten dead (`PHASE_4`, 15) he burrows and charges, the
//!   zappers enraged; a boomerang stun (`damageReaction` 1) and a sword's damage off `sPhase4HP` (4,
//!   then 3 more for each of his last three phases); at `PHASE_DEATH` (18) he dies.
//! - **The death** (`BossVa_BodyDeath`): the room cleared at once (`Flags_SetClear`), the camera
//!   circling him, tumours on his body, the zappers dying one by one (the camera on each), the
//!   supports' arms bursting, his core bursting (`NA_SE_EN_BALINADE_DEAD`, `BossVa_SetDeathEnv`: a
//!   white flash, fog), `NA_BGM_BOSS_CLEAR`, the camera back on Link, the heart container at him
//!   and Ruto (`En_Ru1` type 0) at one of `sWarpPos`: the C's pairwise loop (`:1674-1679`) picks
//!   the first of 0 and 1 that's nearer Link than the one after it, else 2: not always the nearest.
//!
//! **His particles** are `boss_va_effects`. **The environment**: `BossVa_SetSparkEnv` and
//! `BossVa_SetDeathEnv` write `envCtx.adjAmbientColor`, `adjLight1Color`, `adjFogColor` (the
//! body's update takes them back down: 1, 10 and 10 a frame), `adjFogNear` and `adjZFar` (the
//! port's `adj_fog_far`; the body's draw takes them back up 350 a frame), and `screenFillColor`
//! (`EnvCtx::screen_fill_color`; with `fillScreen` on, the port's `TransitionState::screen_fill`).
//!
//! **The draws**: the body (`gBarinadeBodySkel`, 63 limbs) in three bakes for its segments (its
//! limb 7 with the draw's first scroll on segment 8; the rest with limb 20's scroll and its glow
//! as the env colour; its limbs 29 to 55 again in the XLU list), the attached Bari
//! (`gBarinadeDL_008BB8`) at limbs 10 to 19, the tube (`gBarinadeDL_008D70`) at limb 25; the
//! supports (`gBarinadeSupportSkel`, `gBarinadeCutSupportSkel` once cut), the zappers (limb 7 turned
//! to aim: `Matrix_MtxFToZYXRotS`), the stumps, the Bari (limb 2's list replaced by the glow
//! `gBarinadeDL_000FA0` in the XLU list, limbs 3 and 4 drawn again there; their circle shadow
//! `func_80033C30`), the door's eight pieces (`BossVa_DrawDoor`, scaled by `sDoorState`). The
//! points the limb draws compute (`effectPos`, `armTip`, `zapNeckPos`, `zapHeadPos`, `unk_1D8`),
//! the spheres and the lightning quads are set in `draw_update`.
//!
//! Not ported: `func_80026A6C` and `func_80026860` around a stunned Bari's shadow (the scene's fog
//! and the blue filter's put back for it): the shadow is drawn without the filter's fog.
//!
//! **@bug (game)**, kept: the Bari's flex skeleton is set up as a normal one (`SkelAnime_Init`,
//! drawn by `SkelAnime_DrawOpa`: its standard limbs draw the same); `invincibilityTimer` (an `s8`)
//! is checked against 160 (`BossVa_BodyPhase2`: never true); `effectPos[10]` and `[11]` read past
//! the array (`unk_274`, `unk_280`); the gore's scale truncated to 0 (`boss_va_effects`).
//!
//! Params: the whole value (`BossVaParam`): -1 the body (the one the room places), 0 to 2 the
//! supports, 3 to 5 the zappers, 6 to 10 the upper Bari, 11 to 15 the lower, 16 to 18 the stumps,
//! 19 the door; all but the body are spawned by him (or a support), any other value is a Bari.
//! Flags: reads the room's clear flag (set: beaten, Ruto or the blue warp and the heart at once)
//! and sets it as he dies; reads `EVENTCHKINF_BEGAN_BARINADE_BATTLE` (set: a second try, the intro
//! from the Bari's call, no title card) and sets it as the fight starts; reads `EVENTCHKINF_37`
//! (set, in a cleared room: `Door_Warp1` in place of Ruto).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::{Cylinder16, Sphere16};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, scaled_step_to_s, sin_s, smooth_step_to_f, smooth_step_to_s, vec3f_pitch, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BOSS, ACTORCAT_MISC, ActorHandle, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage, audio_play_actor_sfx_at, audio_play_actor_sfx2, enemy_start_finishing_blow, math_sin_f};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::bgm::{seq_cmd1, start_seq};
use oot_game::audio::sfx::*;
use oot_game::audio::{NA_BGM_BOSS, NA_BGM_BOSS_CLEAR, SEQ_PLAYER_BGM_MAIN};
use oot_game::camera::{CAM_ID_MAIN, CAM_STAT_ACTIVE, CAM_STAT_WAIT};
use oot_game::collision_check::*;
use oot_game::event_chk_inf;
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_LOOP_INTERP, ANIMMODE_ONCE, ANIMMODE_ONCE_INTERP, Anim, SkelAnimeStd};
use oot_game::sys_matrix::{MtxF, binang_to_rad};

use crate::boss_va_effects::{self as fx, BossVaEffect, ParentState, SPARK_BARI, SPARK_BLAST, SPARK_BODY, SPARK_LINK, SPARK_TETHER, Spawner, TUMOR_ARM, TUMOR_BODY};

/// `ACTOR_BOSS_VA` (`actor_table.h`: 0x00BA).
pub const ACTOR_BOSS_VA: i16 = 0x00BA;
/// `OBJECT_BV` (`object_table.h`: 0x008C).
pub const OBJECT: &str = "object_bv";
pub const OBJECT_BV: i16 = 0x008C;
const ACTOR_EN_RU1: i16 = crate::en_ru1::ACTOR_EN_RU1;
const ACTOR_DOOR_WARP1: i16 = crate::boss_goma::ACTOR_DOOR_WARP1;
const ACTOR_ITEM_B_HEART: i16 = crate::boss_goma::ACTOR_ITEM_B_HEART;
const ACTOR_EN_BOOM: i16 = crate::en_boom::ACTOR_EN_BOOM;
const SUB_CAM_ID_DONE: i16 = crate::boss_goma::SUB_CAM_ID_DONE;

/// `EVENTCHKINF_BEGAN_BARINADE_BATTLE`, `EVENTCHKINF_37` (`save.h`).
pub const EVENTCHKINF_BEGAN_BARINADE_BATTLE: u16 = 0x76;
pub const EVENTCHKINF_37: u16 = 0x37;
/// `NAVI_ENEMY_BARINADE` (`actor.h`: 0x14), `ATTENTION_RANGE_5`.
const NAVI_ENEMY_BARINADE: u8 = 0x14;
const ATTENTION_RANGE_5: u8 = 5;
/// `PLAYER_CSACTION_*` (`player.h`).
const PLAYER_CSACTION_1: u8 = 1;
const PLAYER_CSACTION_2: u8 = 2;
const PLAYER_CSACTION_7: u8 = 7;
const PLAYER_CSACTION_8: u8 = 8;
/// `PLAYER_STATE1_26` (`player.h`): Link knocked back (or held by the lightning).
const PLAYER_STATE1_26: u32 = crate::en_ba::PLAYER_STATE1_26;

/// `NA_SE_EN_BALINADE_*` (`enemybank_table.h`: 0x393C to 0x3946).
pub const NA_SE_EN_BALINADE_LEVEL: u16 = 0x393C;
pub const NA_SE_EN_BALINADE_DAMAGE: u16 = 0x393D;
pub const NA_SE_EN_BALINADE_FAINT: u16 = 0x393E;
pub const NA_SE_EN_BALINADE_BREAK: u16 = 0x393F;
pub const NA_SE_EN_BALINADE_DEAD: u16 = 0x3940;
pub const NA_SE_EN_BALINADE_STICK: u16 = 0x3941;
pub const NA_SE_EN_BALINADE_THUNDER: u16 = 0x3942;
pub const NA_SE_EN_BALINADE_BL_SPARK: u16 = 0x3943;
pub const NA_SE_EN_BALINADE_BL_DEAD: u16 = 0x3944;
pub const NA_SE_EN_BALINADE_BREAK2: u16 = 0x3945;
pub const NA_SE_EN_BALINADE_HIT_RINK: u16 = 0x3946;

/// `BossVaParam`.
pub const BOSSVA_BODY: i16 = -1;
pub const BOSSVA_SUPPORT_1: i16 = 0;
pub const BOSSVA_SUPPORT_2: i16 = 1;
pub const BOSSVA_SUPPORT_3: i16 = 2;
pub const BOSSVA_ZAPPER_1: i16 = 3;
pub const BOSSVA_ZAPPER_2: i16 = 4;
pub const BOSSVA_ZAPPER_3: i16 = 5;
pub const BOSSVA_BARI_UPPER_1: i16 = 6;
pub const BOSSVA_BARI_UPPER_5: i16 = 10;
pub const BOSSVA_BARI_LOWER_1: i16 = 11;
pub const BOSSVA_BARI_LOWER_5: i16 = 15;
pub const BOSSVA_STUMP_1: i16 = 16;
pub const BOSSVA_STUMP_3: i16 = 18;
pub const BOSSVA_DOOR: i16 = 19;

/// `PHASE_2`, `PHASE_3`, `PHASE_4`, `PHASE_DEATH`: `sFightPhase`'s marks.
pub const PHASE_2: u8 = 3;
pub const PHASE_3: u8 = 9;
pub const PHASE_4: u8 = 15;
pub const PHASE_DEATH: u8 = 18;

/// `BossVaCutscene`: `sCsState`.
pub const INTRO_UNUSED_START: i8 = -5;
pub const INTRO_START: i8 = -4;
pub const INTRO_LOOK_DOOR: i8 = -3;
pub const INTRO_CLOSE_DOOR: i8 = -2;
pub const INTRO_DOOR_SHUT: i8 = -1;
pub const INTRO_CRACKLE: i8 = 0;
pub const INTRO_SPAWN_BARI: i8 = 1;
pub const INTRO_LOOK_BARI: i8 = 2;
pub const INTRO_REVERSE_CAMERA: i8 = 3;
pub const INTRO_SUPPORT_CAMERA: i8 = 4;
pub const INTRO_BODY_SOUND: i8 = 5;
pub const INTRO_LOOK_SUPPORT: i8 = 6;
pub const INTRO_UNUSED_CALL_BARI: i8 = 7;
pub const INTRO_CALL_BARI: i8 = 8;
pub const INTRO_ATTACH_BARI: i8 = 9;
pub const INTRO_TITLE: i8 = 10;
pub const INTRO_BRIGHTEN: i8 = 11;
pub const INTRO_FINISH: i8 = 12;
pub const BOSSVA_BATTLE: i8 = 13;
pub const DEATH_START: i8 = 14;
pub const DEATH_BODY_TUMORS: i8 = 15;
pub const DEATH_ZAPPER_1: i8 = 16;
pub const DEATH_ZAPPER_2: i8 = 17;
pub const DEATH_ZAPPER_3: i8 = 18;
pub const DEATH_SHELL_BURST: i8 = 19;
pub const DEATH_CORE_TUMORS: i8 = 20;
pub const DEATH_CORE_DEAD: i8 = 21;
pub const DEATH_CORE_BURST: i8 = 22;
pub const DEATH_MUSIC: i8 = 23;
pub const DEATH_FINISH: i8 = 24;

/// `Boss_Va_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_BOSS_VA,
    name: "Boss_Va",
    category: ACTORCAT_BOSS,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `params`: `BossVaParam`.
pub const PARAMS: Field = Field::whole(
    "params",
    Kind::Named(&[
        (-1, "BOSSVA_BODY: Barinade (the one the room places; spawns the rest)"),
        (0, "BOSSVA_SUPPORT_1"),
        (1, "BOSSVA_SUPPORT_2"),
        (2, "BOSSVA_SUPPORT_3"),
        (3, "BOSSVA_ZAPPER_1"),
        (4, "BOSSVA_ZAPPER_2"),
        (5, "BOSSVA_ZAPPER_3"),
        (6, "BOSSVA_BARI_UPPER_1"),
        (7, "BOSSVA_BARI_UPPER_2"),
        (8, "BOSSVA_BARI_UPPER_3"),
        (9, "BOSSVA_BARI_UPPER_4"),
        (10, "BOSSVA_BARI_UPPER_5"),
        (11, "BOSSVA_BARI_LOWER_1"),
        (12, "BOSSVA_BARI_LOWER_2"),
        (13, "BOSSVA_BARI_LOWER_3"),
        (14, "BOSSVA_BARI_LOWER_4"),
        (15, "BOSSVA_BARI_LOWER_5"),
        (16, "BOSSVA_STUMP_1 (a cut support's)"),
        (17, "BOSSVA_STUMP_2"),
        (18, "BOSSVA_STUMP_3"),
        (19, "BOSSVA_DOOR: the room's door"),
    ]),
    "which part of Barinade; all but the body are his (or a support's) spawns; any other value is a Bari",
);

/// Boss_Va's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(Flag::Clear).reads().sets().note("beaten: Ruto (or the blue warp) and the heart container at once; set as he dies"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_BEGAN_BARINADE_BATTLE)).reads().sets().note("the fight begun: a second try starts at the Bari's call, no title card"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_37)).reads().note("in a cleared room: Door_Warp1 in place of Ruto"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

/// The overlay's statics (its `.data` and `.bss`), shared by every instance.
#[derive(Debug, Clone)]
pub struct Statics {
    /// `sEffects`.
    pub effects: Vec<BossVaEffect>,
    /// `sBodyState`: 0x80 the body not to be hurt yet; the low bits a hit this frame (supports cut,
    /// 1 stunned, 2 recovering).
    pub body_state: u8,
    pub fight_phase: u8,
    pub cs_state: i8,
    pub sub_cam_id: i16,
    pub sub_cam_eye: Vec3,
    pub sub_cam_at: Vec3,
    pub sub_cam_eye_next: Vec3,
    pub sub_cam_at_next: Vec3,
    pub sub_cam_eye_max_vel_frac: Vec3,
    pub sub_cam_at_max_vel_frac: Vec3,
    pub door_state: i16,
    pub phase3_stop_moving: bool,
    pub phase2_timer: u16,
    pub phase4_hp: i8,
    pub kill_bari: u8,
    /// `sBodyBari`: the Bari attached at each of the body's limbs 10 to 19 (1, then 2 and 3 as the
    /// stick's sound and sparks play).
    pub body_bari: [u8; 10],
}

impl Default for Statics {
    fn default() -> Statics {
        Statics {
            effects: vec![BossVaEffect::default(); fx::BOSS_VA_EFFECT_COUNT],
            body_state: 0,
            fight_phase: 0,
            cs_state: 0,
            sub_cam_id: SUB_CAM_ID_DONE,
            sub_cam_eye: Vec3::ZERO,
            sub_cam_at: Vec3::ZERO,
            sub_cam_eye_next: Vec3::ZERO,
            sub_cam_at_next: Vec3::ZERO,
            sub_cam_eye_max_vel_frac: Vec3::ZERO,
            sub_cam_at_max_vel_frac: Vec3::ZERO,
            door_state: 0,
            phase3_stop_moving: false,
            phase2_timer: 0,
            phase4_hp: 0,
            kill_bari: 0,
            body_bari: [0; 10],
        }
    }
}

/// The statics, zeroed with the overlay's first instance.
pub fn st(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_BOSS_VA)
}

/// The statics as they stand (none before the first instance).
pub fn statics(play: &PlayState) -> Option<&Statics> {
    play.overlay_statics.get(&ACTOR_BOSS_VA).and_then(|s| s.downcast_ref::<Statics>())
}

/// The particles taken out of the statics while `f` runs (nothing it does spawns an actor).
fn with_effects<R>(play: &mut PlayState, f: impl FnOnce(&mut PlayState, &mut Vec<BossVaEffect>) -> R) -> R {
    let mut e = std::mem::take(&mut st(play).effects);
    let r = f(play, &mut e);
    st(play).effects = e;
    r
}

// shared: envCtx.screenFillColor and envCtx.fillScreen (the fill on is TransitionState::screen_fill)
/// `play->envCtx.screenFillColor` as it stands.
pub fn screen_fill_color(play: &PlayState) -> [u8; 4] {
    play.transition.screen_fill.unwrap_or(play.env_ctx.screen_fill_color)
}

/// `play->envCtx.screenFillColor = c`.
pub fn set_screen_fill_color(play: &mut PlayState, c: [u8; 4]) {
    play.env_ctx.screen_fill_color = c;
    if let Some(f) = play.transition.screen_fill.as_mut() {
        *f = c;
    }
}

/// `play->envCtx.fillScreen = on`.
pub fn set_fill_screen(play: &mut PlayState, on: bool) {
    if on {
        play.transition.screen_fill = Some(play.env_ctx.screen_fill_color);
    } else if let Some(c) = play.transition.screen_fill.take() {
        play.env_ctx.screen_fill_color = c;
    }
}

/// `sCylinderInit`: the body's cylinder (85 by 120), hurting 8 with `0xFFCFFFEF` (electric), hit by
/// `DMG_BOOMERANG` (0x10) alone.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFEF, hit_special_effect: HIT_SPECIAL_EFFECT_ELECTRIC, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0010, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 85, height: 120, y_shift: 0, pos: [0, 0, 0] },
};

/// `sJntSphElementsInitSupport`: a support's sphere (25, at limb 4's matrix: `Collider_UpdateSpheres(0)`
/// in its draw), hit by `DMG_BOOMERANG` alone.
pub const JNT_SPH_ELEMENTS_SUPPORT: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0010, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    limb: 0,
    model_sphere: Sphere16 { center: [0, 0, 0], radius: 25 },
    scale: 100,
}];

/// `sJntSphInitSupport`.
pub const JNT_SPH_INIT_SUPPORT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sJntSphElementsInitBari`: a Bari's sphere (30), hurting 4 (electric), hit by `0xFFCFFFFF`.
pub const JNT_SPH_ELEMENTS_BARI: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_ELECTRIC, damage: 0x04 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    limb: 0,
    model_sphere: Sphere16 { center: [0, 0, 0], radius: 30 },
    scale: 100,
}];

/// `sJntSphInitBari`.
pub const JNT_SPH_INIT_BARI: ColliderInit = ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sQuadInit`: the lightning (a zapper's, a Bari's tether), hurting 4 with `DMG_UNBLOCKABLE`.
pub const QUAD_INIT: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_METAL, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x2000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_ELECTRIC, damage: 0x04 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0010, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL | ATELEM_UNK7,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `DMG_ENTRY(damage, effect)`.
const fn dmg(damage: u8, effect: u8) -> u8 {
    (effect << 4) | damage
}

/// `sDamageTable` (the body's).
pub static DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg(0, 0x1), // Deku nut
        dmg(2, 0x0), // Deku stick
        dmg(1, 0x0), // Slingshot
        dmg(2, 0x0), // Explosive
        dmg(0, 0x1), // Boomerang
        dmg(2, 0x0), // Normal arrow
        dmg(2, 0x0), // Hammer swing
        dmg(0, 0x1), // Hookshot
        dmg(1, 0x0), // Kokiri sword
        dmg(2, 0x0), // Master sword
        dmg(4, 0x0), // Giant's Knife
        dmg(2, 0x0), // Fire arrow
        dmg(2, 0x0), // Ice arrow
        dmg(2, 0x0), // Light arrow
        dmg(2, 0x0), // Unk arrow 1
        dmg(2, 0x0), // Unk arrow 2
        dmg(2, 0x0), // Unk arrow 3
        dmg(0, 0xE), // Fire magic
        dmg(0, 0x6), // Ice magic
        dmg(0, 0xD), // Light magic
        dmg(0, 0x0), // Shield
        dmg(0, 0x0), // Mirror Ray
        dmg(1, 0x0), // Kokiri spin
        dmg(4, 0x0), // Giant spin
        dmg(2, 0x0), // Master spin
        dmg(2, 0x0), // Kokiri jump
        dmg(8, 0x0), // Giant jump
        dmg(4, 0x0), // Master jump
        dmg(0, 0x0), // Unknown 1
        dmg(0, 0x0), // Unblockable
        dmg(4, 0x0), // Hammer jump
        dmg(0, 0x0), // Unknown 2
    ],
};

/// `sInitPosOffsets`: each part's place from the body (supports, zappers, Bari on him), then the
/// Bari's cutscene places (`params + 10`).
pub const S_INIT_POS_OFFSETS: [Vec3; 26] = [
    Vec3::new(0.0, 175.35, 0.0),
    Vec3::new(0.0, 175.35, 0.0),
    Vec3::new(0.0, 175.35, 0.0),
    Vec3::new(120.0, 103.425, -67.0),
    Vec3::new(0.0, 103.425, 140.0),
    Vec3::new(-120.0, 103.425, -70.0),
    Vec3::new(-2.0, 16.0, 50.0),
    Vec3::new(48.0, 16.0, 15.0),
    Vec3::new(25.0, 16.0, -36.0),
    Vec3::new(-29.0, 16.0, -36.0),
    Vec3::new(-63.0, 16.0, 22.0),
    Vec3::new(0.0, -10.0, -64.0),
    Vec3::new(63.0, -10.0, -22.0),
    Vec3::new(35.0, -10.0, 46.0),
    Vec3::new(-36.0, -10.0, 46.0),
    Vec3::new(-49.0, -10.0, -17.0),
    Vec3::new(0.0, 160.0, 370.0),
    Vec3::new(65.0, 35.0, 370.0),
    Vec3::new(80.0, 70.0, -130.0),
    Vec3::new(-160.0, 100.0, -130.0),
    Vec3::new(-150.0, 130.0, 0.0),
    Vec3::new(230.0, 0.0, 0.0),
    Vec3::new(60.0, 140.0, 0.0),
    Vec3::new(0.0, 40.0, 270.0),
    Vec3::new(-100.0, 10.0, 200.0),
    Vec3::new(-90.0, 70.0, -310.0),
];

const fn s(v: u16) -> i16 {
    v as i16
}

/// `sInitRot`.
pub const S_INIT_ROT: [[i16; 3]; 16] = [
    [s(0x1FFE), s(0x0000), 0],
    [s(0x1FFE), s(0x5550), 0],
    [s(0x1FFE), s(0xAAB0), 0],
    [s(0xD558), s(0x5550), 0],
    [s(0xD558), s(0x0000), 0],
    [s(0xD558), s(0xAAB0), 0],
    [s(0x2AA8), s(0xFCCC), 0],
    [s(0x2AA8), s(0x3330), 0],
    [s(0x2AA8), s(0x6660), 0],
    [s(0x2AA8), s(0x99A0), 0],
    [s(0x2AA8), s(0xCCD0), 0],
    [s(0x4C98), s(0x81D0), 0],
    [s(0x4C98), s(0x4F70), 0],
    [s(0x4C98), s(0x1758), 0],
    [s(0x4C98), s(0xE8A8), 0],
    [s(0x4C98), s(0xB648), 0],
];

/// `sWarpPos`: where Ruto may rise after the fight.
pub const S_WARP_POS: [Vec3; 3] = [Vec3::new(10.0, 0.0, 30.0), Vec3::new(260.0, 0.0, -470.0), Vec3::new(-240.0, 0.0, -470.0)];

/// The door's place (`BossVa_Init`'s spawn and `BossVa_DrawDoor`'s matrix).
pub const DOOR_POS: Vec3 = Vec3::new(0.0, 80.0, 400.0);

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// None set (the cleared room's body, killed in its init).
    None,
    BodyIntro,
    BodyPhase1,
    BodyPhase2,
    BodyPhase3,
    BodyPhase4,
    BodyDeath,
    SupportIntro,
    SupportAttached,
    SupportCut,
    ZapperIntro,
    ZapperAttack,
    ZapperDamaged,
    ZapperHold,
    ZapperDeath,
    ZapperEnraged,
    Stump,
    Door,
    BariIntro,
    BariPhase2Attack,
    BariPhase3Attack,
    BariPhase3Stunned,
    BariDeath,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::None => "NULL",
            Action::BodyIntro => "BossVa_BodyIntro",
            Action::BodyPhase1 => "BossVa_BodyPhase1",
            Action::BodyPhase2 => "BossVa_BodyPhase2",
            Action::BodyPhase3 => "BossVa_BodyPhase3",
            Action::BodyPhase4 => "BossVa_BodyPhase4",
            Action::BodyDeath => "BossVa_BodyDeath",
            Action::SupportIntro => "BossVa_SupportIntro",
            Action::SupportAttached => "BossVa_SupportAttached",
            Action::SupportCut => "BossVa_SupportCut",
            Action::ZapperIntro => "BossVa_ZapperIntro",
            Action::ZapperAttack => "BossVa_ZapperAttack",
            Action::ZapperDamaged => "BossVa_ZapperDamaged",
            Action::ZapperHold => "BossVa_ZapperHold",
            Action::ZapperDeath => "BossVa_ZapperDeath",
            Action::ZapperEnraged => "BossVa_ZapperEnraged",
            Action::Stump => "BossVa_Stump",
            Action::Door => "BossVa_Door",
            Action::BariIntro => "BossVa_BariIntro",
            Action::BariPhase2Attack => "BossVa_BariPhase2Attack",
            Action::BariPhase3Attack => "BossVa_BariPhase3Attack",
            Action::BariPhase3Stunned => "BossVa_BariPhase3Stunned",
            Action::BariDeath => "BossVa_BariDeath",
        }
    }
}

/// `object_bv`'s animations.
#[derive(Default, Clone)]
struct Anims {
    body: Option<Anim>,
    support_attached: Option<Anim>,
    support_damage1: Option<Anim>,
    support_damage2: Option<Anim>,
    support_cut: Option<Anim>,
    support_detached: Option<Anim>,
    zapper_idle: Option<Anim>,
    zapper_damage1: Option<Anim>,
    zapper_damage2: Option<Anim>,
    stump: Option<Anim>,
    bari: Option<Anim>,
}

fn last(a: &Option<Anim>) -> f32 {
    a.as_ref().map(|a| a.last_frame()).unwrap_or(0.0)
}

/// Which part params `p` is.
pub fn is_support(p: i16) -> bool {
    (BOSSVA_SUPPORT_1..=BOSSVA_SUPPORT_3).contains(&p)
}
pub fn is_zapper(p: i16) -> bool {
    (BOSSVA_ZAPPER_1..=BOSSVA_ZAPPER_3).contains(&p)
}
pub fn is_stump(p: i16) -> bool {
    (BOSSVA_STUMP_1..=BOSSVA_STUMP_3).contains(&p)
}
/// The `default:` case: the Bari, and any params not named.
pub fn is_bari(p: i16) -> bool {
    p != BOSSVA_BODY && p != BOSSVA_DOOR && !is_support(p) && !is_zapper(p) && !is_stump(p)
}

fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    ((b.x - a.x) * (b.x - a.x) + (b.z - a.z) * (b.z - a.z)).sqrt()
}

/// `BossVa`.
pub struct BossVa {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    cut_skeleton: Option<Arc<Skeleton>>,
    anims: Anims,
    pub action: Action,
    /// `onCeiling`: a support attached; the body's flash counter (2, 6: not used by it).
    pub on_ceiling: u8,
    pub burst: u8,
    pub invincibility_timer: i8,
    pub is_dead: u8,
    pub timer: i32,
    pub timer2: i16,
    /// `unk_1A0`: the body's upper pulse; a Bari's spin radius.
    pub unk_1a0: f32,
    /// `unk_1A4`: the body's lower pulse; a Bari's bob phase.
    pub unk_1a4: f32,
    /// `unk_1A8`: the death camera's zoom (always 0).
    pub unk_1a8: f32,
    /// `unk_1AC`: the body's pulse phase, the death camera's turn, a Bari's spin step.
    pub unk_1ac: i16,
    pub body_glow: i16,
    /// `unk_1B0`: the glow's phase, the death's tumour count, a zapper's death camera turn.
    pub unk_1b0: i16,
    pub arm_tip: Vec3,
    pub zap_neck_pos: Vec3,
    pub zap_head_pos: Vec3,
    /// `unk_1D8`: the body's attach point for the Bari; a Bari's scale; a zapper's target.
    pub unk_1d8: Vec3,
    pub unk_1e4: i16,
    pub unk_1e6: i16,
    pub unk_1e8: i16,
    pub unk_1ea: i16,
    pub unk_1ec: i16,
    pub unk_1ee: i16,
    /// `unk_1F0`: a zapper's head pitch; a Bari's spark scale.
    pub unk_1f0: i16,
    /// `unk_1F2`: a zapper's head yaw; the intro camera's turn rate.
    pub unk_1f2: i16,
    pub unk_1f4: i16,
    /// `headRot`: `vaCamRotMod` (x) and `vaBodySpinRate` (y) on the body; a Bari's unused wobble.
    pub head_rot: [i16; 3],
    /// `effectPos[10]`, then `unk_274` and `unk_280` (which the C reads as `effectPos[10]` and
    /// `[11]`).
    pub effect_pos: [Vec3; 12],
    pub body_collider: ColliderCylinder,
    pub collider_jnt_sph: ColliderJntSph,
    pub collider_lightning: ColliderQuad,
    /// Not in the C: a support's skeleton is the cut one.
    pub cut: bool,
}

// shared: SkelAnime_DrawOpa / SkelAnime_DrawFlexOpa's limb walk in world matrices (MtxF), with
// the override's matrix and the post-limb callback
/// `SkelAnime_DrawOpa`'s (and `SkelAnime_DrawFlexOpa`'s) walk of the limbs from `model`: for each
/// limb (1-based), `ov` gets the current matrix (the parent's) to change (`Matrix_*` in
/// `OverrideLimbDraw`), its position and rotation; then the limb's matrix; then `post` with it.
pub fn walk(skeleton: &Skeleton, joints: &[[i16; 3]], model: &MtxF, mut ov: impl FnMut(usize, &mut MtxF, &mut Vec3, &mut [i16; 3]), mut post: impl FnMut(usize, &MtxF)) -> Vec<MtxF> {
    let mut out = vec![MtxF::IDENTITY; skeleton.limbs.len()];
    for &l in &skeleton.draw_order {
        let l = l as usize;
        let mut m = skeleton.parents[l].map(|p| out[p as usize]).unwrap_or(*model);
        let mut pos = if l == 0 {
            let r = joints.first().copied().unwrap_or([0; 3]);
            Vec3::new(r[0] as f32, r[1] as f32, r[2] as f32)
        } else {
            let p = skeleton.limbs[l].joint_pos;
            Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
        };
        let mut rot = joints.get(l + 1).copied().unwrap_or([0; 3]);
        ov(l + 1, &mut m, &mut pos, &mut rot);
        m.translate_rotate_zyx(pos, rot);
        out[l] = m;
        post(l + 1, &m);
    }
    out
}

/// The bones of a skeleton's draw from its limbs' matrices: each from `model`.
fn bones(model: &MtxF, world: &[MtxF]) -> Vec<Mat4> {
    let inv = model.to_mat4().inverse();
    world.iter().map(|m| inv * m.to_mat4()).collect()
}

/// `Actor_Draw`'s matrix of `a` as an `MtxF`.
fn model_of(a: &Actor) -> MtxF {
    MtxF::from_mat4(actor_draw_matrix(&RenderState::of(a)))
}

/// Player's `stateFlags1` (0 without him).
fn player_state1(play: &PlayState) -> u32 {
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.state_flags1())
}

fn player_actor(play: &PlayState) -> Option<&Actor> {
    play.player.and_then(|h| play.actors.actor(h))
}

fn player_pos(play: &PlayState) -> Vec3 {
    player_actor(play).map_or(Vec3::ZERO, |p| p.world_pos)
}

/// The id of the actor `h` (`collider.base.ac->id`).
fn actor_id(play: &PlayState, h: Option<ActorHandle>) -> Option<i16> {
    h.and_then(|h| play.actors.actor(h)).map(|a| a.id)
}

/// `BossVa_SetSparkEnv`.
pub fn set_spark_env(play: &mut PlayState) {
    let e = &mut play.env_ctx;
    e.adj_ambient_color = [10, 10, 10];
    e.adj_light1_color = [115, 65, 100];
    e.adj_fog_color = [120, 120, 70];
}

/// `BossVa_SetDeathEnv`.
pub fn set_death_env(play: &mut PlayState) {
    let e = &mut play.env_ctx;
    e.adj_fog_color = [220, 220, 150];
    e.adj_fog_near = -1000;
    e.adj_fog_far = -900;
    e.adj_ambient_color = [200, 200, 200];
    e.adj_light1_color = [215, 165, 200];
    set_screen_fill_color(play, [220, 220, 150, 100]);
}

/// `BossVa_FindBoomerang`: the first `En_Boom` in `ACTORCAT_MISC`.
pub fn find_boomerang(play: &PlayState) -> Option<ActorHandle> {
    play.actors.category(ACTORCAT_MISC).iter().copied().find(|&h| play.actors.actor(h).is_some_and(|a| a.id == ACTOR_EN_BOOM))
}

/// The boomerang `h` sent back to Link at once (`returnTimer` 0, `moveTo` Link, turned to him).
fn send_boomerang_back(play: &mut PlayState, h: Option<ActorHandle>) {
    let player = play.player;
    if let Some(b) = h.and_then(|h| play.actors.downcast_mut::<crate::en_boom::EnBoom>(h)) {
        b.return_timer = 0;
        b.move_to = player;
        b.actor.world_rot.y = b.actor.yaw_towards_player;
    }
}

impl BossVa {
    /// Which part this is (`BossVaParam`, the whole params).
    pub fn ty(&self) -> i16 {
        PARAMS.get(self.actor.params)
    }

    /// `GET_BODY(this)`: the parent.
    fn body<'a>(&self, play: &'a PlayState) -> Option<&'a BossVa> {
        self.actor.parent.and_then(|h| play.actors.downcast::<BossVa>(h))
    }

    fn body_mut<'a>(&self, play: &'a mut PlayState) -> Option<&'a mut BossVa> {
        self.actor.parent.and_then(|h| play.actors.downcast_mut::<BossVa>(h))
    }

    /// This part as an effect's parent: its state (its body's `unk_1D8` the body's own for the
    /// body).
    pub fn parent_state(&self, body_unk_1d8: Vec3) -> ParentState {
        ParentState { world_pos: self.actor.world_pos, world_rot_y: self.actor.world_rot.y, y_offset: self.actor.shape_y_offset, scale_y: self.actor.scale.y, burst: self.burst, body_unk_1d8 }
    }

    fn spawner(&self, play: &PlayState) -> Spawner {
        let body_unk_1d8 = if self.ty() == BOSSVA_BODY { self.unk_1d8 } else { self.body(play).map_or(Vec3::ZERO, |b| b.unk_1d8) };
        Spawner { handle: play.cur_actor, state: self.parent_state(body_unk_1d8) }
    }

    /// `BossVa_Spark(play, this, ...)`.
    #[allow(clippy::too_many_arguments)]
    fn spark(&self, play: &mut PlayState, count: i32, scale: i16, xz_spread: f32, y_spread: f32, mode: u8, range: f32, fixed: bool) {
        let sp = self.spawner(play);
        let ep = self.effect_pos;
        with_effects(play, |play, e| fx::spark(play, e, &sp, &ep, count, scale, xz_spread, y_spread, mode, range, fixed));
    }

    /// `BossVa_Tumor(play, this, ...)`.
    #[allow(clippy::too_many_arguments)]
    fn tumor(&self, play: &mut PlayState, count: i32, scale: i16, xz_spread: f32, y_spread: f32, mode: u8, range: f32, fixed: bool) {
        let sp = self.spawner(play);
        let ep = self.effect_pos;
        with_effects(play, |play, e| fx::tumor(play, e, &sp, &ep, count, scale, xz_spread, y_spread, mode, range, fixed));
    }

    /// `Animation_Change(&skelAnime, anim, speed, start, end, mode, morph)`.
    fn change(&mut self, anim: Option<Anim>, speed: f32, start: f32, end: f32, mode: u8, morph: f32) {
        if let Some(a) = anim {
            self.skel.change(a, speed, start, end, mode, morph);
        }
    }

    fn me(play: &PlayState) -> Option<ActorHandle> {
        play.cur_actor
    }

    /// `Player_SetCsActionWithHaltedActors(play, &this->actor, mode)`.
    fn player_cs(play: &mut PlayState, mode: u8) {
        let me = Self::me(play);
        play.player_set_cs_action_with_halted_actors(me, mode);
    }

    /// `Actor_PlaySfx(&this->actor, NA_SE_EN_BALINADE_BL_SPARK - SFX_FLAG)` one time in ten.
    fn maybe_bl_spark(play: &mut PlayState) {
        if play.rand.zero_one() < 0.1 {
            audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_BL_SPARK - SFX_FLAG);
        }
    }

    /// `func_800F436C(&this->actor.projectedPos, NA_SE_EN_BALINADE_LEVEL - SFX_FLAG, freq)`.
    fn level_sfx(play: &mut PlayState, freq: f32) {
        if let Some(h) = play.cur_actor {
            play.audio.func_800f436c(SfxPos::Actor(h), NA_SE_EN_BALINADE_LEVEL - SFX_FLAG, freq);
        }
    }

    /// The body's pulse (`unk_1AC` on, `unk_1A0` and `unk_1A4` by it).
    fn pulse(&mut self) {
        self.unk_1ac = self.unk_1ac.wrapping_add(0xC31);
        self.unk_1a0 = (cos_s(self.unk_1ac) * 0.1) + 1.0;
        self.unk_1a4 = (sin_s(self.unk_1ac) * 0.05) + 1.0;
    }

    /// The glow's pulse.
    fn glow(&mut self) {
        self.unk_1b0 = self.unk_1b0.wrapping_add(0xCE4);
        self.body_glow = (sin_s(self.unk_1b0) * 50.0) as i16 + 150;
    }

    /// `Actor_SpawnAsChild` of the part `i` at `sInitPosOffsets[i]` and `sInitRot[i]` from him.
    fn spawn_part(&mut self, play: &mut PlayState, i: i16) {
        let o = S_INIT_POS_OFFSETS[i as usize];
        let r = S_INIT_ROT[i as usize];
        let w = self.actor.world_pos;
        let rot = [r[0].wrapping_add(self.actor.world_rot.x), r[1].wrapping_add(self.actor.world_rot.y), r[2].wrapping_add(self.actor.world_rot.z)];
        if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_BOSS_VA, o + w, rot, i) {
            log::warn!("Boss_Va: part {i} not spawned: {e:?}");
        }
    }

    /// `BossVa_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.scale = Vec3::splat(0.1);
        actor.target_mode = ATTENTION_RANGE_5;
        actor.col_chk_info.mass = MASS_IMMOVABLE;
        let p = PARAMS.get(actor.params);
        let mut anims = Anims::default();
        let (mut skeleton, mut cut_skeleton) = (None, None);
        if let Some(a) = play.assets.clone() {
            let anim = |name: &str| a.animation(OBJECT, name).map_err(|e| log::error!("Boss_Va: {e:#}")).ok();
            let skel = |name: &str| a.skeleton(OBJECT, name).map_err(|e| log::error!("Boss_Va: {e:#}")).ok();
            anims = Anims {
                body: anim("gBarinadeBodyAnim"),
                support_attached: anim("gBarinadeSupportAttachedAnim"),
                support_damage1: anim("gBarinadeSupportDamage1Anim"),
                support_damage2: anim("gBarinadeSupportDamage2Anim"),
                support_cut: anim("gBarinadeSupportCutAnim"),
                support_detached: anim("gBarinadeSupportDetachedAnim"),
                zapper_idle: anim("gBarinadeZapperIdleAnim"),
                zapper_damage1: anim("gBarinadeZapperDamage1Anim"),
                zapper_damage2: anim("gBarinadeZapperDamage2Anim"),
                stump: anim("gBarinadeStumpAnim"),
                bari: anim("gBarinadeBariAnim"),
            };
            skeleton = if p == BOSSVA_BODY {
                skel("gBarinadeBodySkel")
            } else if is_support(p) {
                cut_skeleton = skel("gBarinadeCutSupportSkel");
                skel("gBarinadeSupportSkel")
            } else if is_zapper(p) {
                skel("gBarinadeZapperSkel")
            } else if is_stump(p) {
                skel("gBarinadeStumpSkel")
            } else if p == BOSSVA_DOOR {
                None
            } else {
                // @bug (game): the flex skeleton set up as a normal one.
                skel("gBarinadeBariSkel")
            };
        }
        let limbs = skeleton.as_ref().map_or(1, |s| s.limbs.len());
        let first = if p == BOSSVA_BODY {
            anims.body.clone()
        } else if is_support(p) {
            anims.support_attached.clone()
        } else if is_zapper(p) {
            anims.zapper_idle.clone()
        } else if is_stump(p) {
            anims.stump.clone()
        } else if p == BOSSVA_DOOR {
            None
        } else {
            anims.bari.clone()
        };
        let skel = SkelAnimeStd::init_flex(limbs, first);
        if p == BOSSVA_BODY {
            actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
        } else if is_bari(p) {
            actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
            actor.shape_y_offset = 400.0;
        }
        actor.focus_pos = actor.world_pos;
        actor.navi_enemy_id = NAVI_ENEMY_BARINADE;
        let mut this = BossVa {
            actor,
            skel,
            skeleton,
            cut_skeleton,
            anims,
            action: Action::None,
            on_ceiling: 0,
            burst: 0,
            invincibility_timer: 0,
            is_dead: 0,
            timer: 0,
            timer2: 0,
            unk_1a0: 0.0,
            unk_1a4: 0.0,
            unk_1a8: 0.0,
            unk_1ac: 0,
            body_glow: 0,
            unk_1b0: 0,
            arm_tip: Vec3::ZERO,
            zap_neck_pos: Vec3::ZERO,
            zap_head_pos: Vec3::ZERO,
            unk_1d8: Vec3::ZERO,
            unk_1e4: 0,
            unk_1e6: 0,
            unk_1e8: 0,
            unk_1ea: 0,
            unk_1ec: 0,
            unk_1ee: 0,
            unk_1f0: 0,
            unk_1f2: 0,
            unk_1f4: 0,
            head_rot: [0; 3],
            effect_pos: [Vec3::ZERO; 12],
            body_collider: ColliderCylinder::default(),
            collider_jnt_sph: ColliderJntSph::default(),
            collider_lightning: ColliderQuad::default(),
            cut: false,
        };
        // The statics come with the overlay (its first instance).
        st(play);
        if p == BOSSVA_BODY {
            this.init_body(play);
        } else if is_support(p) {
            this.collider_jnt_sph = ColliderJntSph::new(&JNT_SPH_INIT_SUPPORT, &JNT_SPH_ELEMENTS_SUPPORT);
            if st(play).cs_state < BOSSVA_BATTLE {
                this.setup_support_intro();
            } else {
                this.setup_support_attached();
            }
            this.on_ceiling += 1;
        } else if is_zapper(p) {
            this.collider_lightning = ColliderQuad::new(&QUAD_INIT);
            if st(play).cs_state < BOSSVA_BATTLE {
                this.setup_zapper_intro();
            } else {
                this.setup_zapper_attack();
            }
        } else if is_stump(p) {
            this.setup_stump();
        } else if p == BOSSVA_DOOR {
            this.setup_door(play);
        } else {
            this.collider_jnt_sph = ColliderJntSph::new(&JNT_SPH_INIT_BARI, &JNT_SPH_ELEMENTS_BARI);
            this.collider_lightning = ColliderQuad::new(&QUAD_INIT);
            this.unk_1d8.x = 1.0;
            this.unk_1d8.y = 1.0;
            let (cs, phase) = (st(play).cs_state, st(play).fight_phase);
            if cs < BOSSVA_BATTLE {
                this.setup_bari_intro(play);
            } else if phase >= PHASE_3 {
                this.setup_bari_phase3_attack();
            } else {
                this.setup_bari_phase2_attack();
            }
        }
        Box::new(this)
    }

    /// `BossVa_Init`'s body.
    fn init_body(&mut self, play: &mut PlayState) {
        if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_BOSS_VA, DOOR_POS, [0; 3], BOSSVA_DOOR) {
            log::warn!("Boss_Va: the door not spawned: {e:?}");
        }
        let room = play.room_ctx.cur.num;
        if play.flags.get_clear(room) {
            // schema: EVENTCHKINF_37 picks the warp's actor
            let warp_id = if play.save.get_event_chk_inf(EVENTCHKINF_37) { ACTOR_DOOR_WARP1 } else { ACTOR_EN_RU1 };
            // @note (the C's): its params could be WARP_DUNGEON_CHILD, but they're also Ruto's type 0.
            if let Err(e) = play.actor_spawn(warp_id, self.actor.world_pos, [0; 3], 0) {
                log::warn!("Boss_Va: the cleared room's Ruto / warp not spawned: {e:?}");
            }
            let heart = self.actor.world_pos + Vec3::new(160.0, 0.0, 0.0);
            if let Err(e) = play.actor_spawn(ACTOR_ITEM_B_HEART, heart, [0; 3], 0) {
                log::warn!("Boss_Va: the heart container not spawned: {e:?}");
            }
            st(play).door_state = 100;
            self.actor.kill();
            return;
        }
        self.actor.col_chk_info.damage_table = Some(&DAMAGE_TABLE);
        st(play).phase2_timer = 0xFFFF;
        if play.save.get_event_chk_inf(EVENTCHKINF_BEGAN_BARINADE_BATTLE) {
            {
                let s = st(play);
                s.cs_state = INTRO_CALL_BARI;
                s.door_state = 100;
            }
            // (Its own handle isn't made yet: Player's cutscene actor is none.)
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_1);
            set_screen_fill_color(play, [0xDC, 0xDC, 0xBE, 0xD2]);
            play.start_manual();
            let cam = play.create_sub_camera();
            play.change_camera_status(CAM_ID_MAIN, CAM_STAT_WAIT);
            play.change_camera_status(cam, CAM_STAT_ACTIVE);
            let (eye, at) = (Vec3::new(140.0, 205.0, -20.0), Vec3::new(10.0, 50.0, -220.0));
            {
                let s = st(play);
                s.sub_cam_id = cam;
                s.sub_cam_eye = eye;
                s.sub_cam_eye_next = eye;
                s.sub_cam_at = at;
                s.sub_cam_at_next = at;
            }
            play.camera_set_at_eye(cam, at, eye);
            self.timer = 20;
            for i in (BOSSVA_BARI_UPPER_1..=BOSSVA_BARI_LOWER_5).rev() {
                self.spawn_part(play, i);
            }
            let s = st(play);
            s.sub_cam_eye_max_vel_frac = Vec3::ZERO;
            s.sub_cam_at_max_vel_frac = Vec3::ZERO;
        } else {
            let s = st(play);
            s.cs_state = INTRO_START;
            s.door_state = 5;
        }
        self.zap_head_pos.x = 1.0;
        self.body_collider = ColliderCylinder::new(&CYLINDER_INIT);
        for i in (BOSSVA_SUPPORT_1..=BOSSVA_ZAPPER_3).rev() {
            self.spawn_part(play, i);
        }
        // Lib_MemSet(sEffects).
        st(play).effects = vec![BossVaEffect::default(); fx::BOSS_VA_EFFECT_COUNT];
        if st(play).cs_state < BOSSVA_BATTLE {
            self.setup_intro();
        } else {
            self.setup_body_phase1(play);
        }
    }

    /// `BossVa_AttachToBody`: at `sInitPosOffsets[params]` from the body (turned by its x and z);
    /// a support on the ceiling keeps its turn, a cut one and the zappers follow the body's.
    fn attach_to_body(&mut self, play: &PlayState) {
        let Some(body) = self.body(play) else {
            log::warn!("Boss_Va: part {} without its body", self.ty());
            return;
        };
        let (bpos, brot, by) = (body.actor.world_pos, body.actor.shape_rot, body.actor.shape_y_offset);
        let p = self.ty();
        let mut m = MtxF::set_translate(bpos.x, bpos.y, bpos.z);
        m.rotate_zyx(brot.x, 0, brot.z);
        if let Some(o) = S_INIT_POS_OFFSETS.get(p as usize) {
            self.actor.world_pos = m.mult_vec3f(*o);
        }
        if is_support(p) {
            if self.on_ceiling == 0 {
                let r = S_INIT_ROT[p as usize];
                self.actor.shape_rot.x = r[0].wrapping_add(brot.x);
                self.actor.shape_rot.y = r[1];
                self.actor.shape_rot.z = r[2].wrapping_add(brot.z);
            }
        } else if is_zapper(p) {
            let r = S_INIT_ROT[p as usize];
            self.actor.shape_rot.y = r[1];
            let a = (cos_s(r[1]) * brot.x as f32) as i16;
            let b = (sin_s(r[1]) * brot.z as f32) as i16;
            self.actor.shape_rot.x = r[0].wrapping_add(a).wrapping_sub(b);
            let c = (cos_s(r[1]) * brot.z as f32) as i16;
            let d = (sin_s(r[1]) * brot.x as f32) as i16;
            self.actor.shape_rot.z = c.wrapping_add(r[2].wrapping_add(d));
        }
        self.actor.world_rot = self.actor.shape_rot;
        self.actor.shape_y_offset = by;
    }

    /// `BossVa_KillBari`: eight bubbles, the phase on, `BossVa_SetupBariDeath`.
    fn kill_bari(&mut self, play: &mut PlayState) {
        let accel = Vec3::ZERO;
        let mut velocity = Vec3::ZERO;
        for _ in 0..8 {
            let w = self.actor.world_pos;
            let x = play.rand.centered_float(60.0) + w.x;
            let y = play.rand.centered_float(50.0) + (w.y + (self.actor.shape_y_offset * self.actor.scale.y));
            let z = play.rand.centered_float(60.0) + w.z;
            velocity.y = play.rand.zero_one() + 1.0;
            let scale = play.rand.s16_offset(80, 100);
            let profile = if play.rand.zero_one() < 0.7 { 2 } else { 0 };
            let pos = Vec3::new(x, y, z);
            play.with_ss(|s| s.dt_bubble_spawn_color_profile(pos, velocity, accel, scale, 25, profile, 1));
        }
        st(play).fight_phase += 1;
        self.setup_bari_death(play);
    }

    // ------------------------------------------------------------------------------------------
    // The body
    // ------------------------------------------------------------------------------------------

    /// `BossVa_SetupIntro`.
    fn setup_intro(&mut self) {
        let l = last(&self.anims.body);
        self.change(self.anims.body.clone(), 1.0, l, l, ANIMMODE_ONCE, 0.0);
        self.actor.shape_y_offset = -450.0;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::BodyIntro;
    }

    /// The sub camera made (if none) and active, the main one waiting, its eye at (13, 124, 167)
    /// on Link, standing still (`INTRO_LOOK_DOOR`'s and `INTRO_SPAWN_BARI`'s).
    fn intro_camera_on_link(play: &mut PlayState) {
        play.start_manual();
        if st(play).sub_cam_id == SUB_CAM_ID_DONE {
            let id = play.create_sub_camera();
            st(play).sub_cam_id = id;
        }
        let id = st(play).sub_cam_id;
        play.change_camera_status(CAM_ID_MAIN, CAM_STAT_WAIT);
        play.change_camera_status(id, CAM_STAT_ACTIVE);
        let pp = player_pos(play);
        let s = st(play);
        s.sub_cam_eye = Vec3::new(13.0, 124.0, 167.0);
        s.sub_cam_eye_next = s.sub_cam_eye;
        s.sub_cam_at = pp;
        s.sub_cam_at_next = pp;
        s.sub_cam_at_max_vel_frac = Vec3::ZERO;
        s.sub_cam_eye_max_vel_frac = Vec3::ZERO;
    }

    /// `BossVa_BodyIntro`.
    fn body_intro(&mut self, play: &mut PlayState) {
        self.pulse();
        match st(play).cs_state {
            INTRO_UNUSED_START => {
                self.timer -= 1;
                if self.timer == 0 {
                    st(play).cs_state = INTRO_CLOSE_DOOR;
                    self.timer = 10;
                }
            }
            INTRO_START => {
                set_screen_fill_color(play, [0xDC, 0xDC, 0xBE, 0xD2]);
                Self::player_cs(play, PLAYER_CSACTION_8);
                if let Some(h) = play.player
                    && let Some(p) = play.actors.actor_mut(h)
                {
                    p.world_rot.y = 0x7FFF;
                    p.shape_rot.y = 0x7FFF;
                }
                st(play).cs_state += 1;
            }
            INTRO_LOOK_DOOR => {
                Self::intro_camera_on_link(play);
                self.timer = 10;
                st(play).cs_state += 1;
            }
            INTRO_CLOSE_DOOR => {
                self.timer -= 1;
                if self.timer == 0 {
                    Self::player_cs(play, PLAYER_CSACTION_2);
                    st(play).cs_state += 1;
                    self.timer = 30;
                }
            }
            INTRO_DOOR_SHUT => {
                self.timer -= 1;
                if self.timer == 0 {
                    st(play).cs_state += 1;
                }
                Self::maybe_bl_spark(play);
            }
            INTRO_CRACKLE => {
                Self::player_cs(play, PLAYER_CSACTION_1);
                st(play).cs_state += 1;
            }
            INTRO_SPAWN_BARI => {
                Self::intro_camera_on_link(play);
                for i in (BOSSVA_BARI_UPPER_1..=BOSSVA_BARI_LOWER_5).rev() {
                    self.spawn_part(play, i);
                }
                self.timer = 90;
                st(play).cs_state += 1;
            }
            INTRO_REVERSE_CAMERA => {
                let s = st(play);
                s.sub_cam_eye_next = Vec3::new(-92.0, 22.0, 360.0);
                s.sub_cam_at_next = Vec3::new(63.0, 104.0, 248.0);
                smooth_step_to_f(&mut s.sub_cam_eye_max_vel_frac.x, 7.0, 0.3, 0.7, 0.05);
                s.sub_cam_eye_max_vel_frac.z = s.sub_cam_eye_max_vel_frac.x;
                s.sub_cam_eye_max_vel_frac.y = s.sub_cam_eye_max_vel_frac.z;
                s.sub_cam_at_max_vel_frac = s.sub_cam_eye_max_vel_frac;
                self.timer -= 1;
                if self.timer == 0 {
                    s.cs_state += 1;
                    self.timer = 60;
                }
            }
            INTRO_SUPPORT_CAMERA => {
                let s = st(play);
                s.sub_cam_eye = Vec3::new(140.0, 205.0, -20.0);
                s.sub_cam_eye_next = s.sub_cam_eye;
                s.sub_cam_at = Vec3::new(10.0, 247.0, -220.0);
                s.sub_cam_at_next = s.sub_cam_at;
                s.cs_state += 1;
                self.timer = 1;
            }
            INTRO_BODY_SOUND => {
                let s = st(play);
                s.sub_cam_at_next = Vec3::new(10.0, 247.0, -220.0);
                smooth_step_to_f(&mut s.sub_cam_eye_max_vel_frac.x, 7.0, 0.3, 0.7, 0.05);
                s.sub_cam_eye_max_vel_frac.z = s.sub_cam_eye_max_vel_frac.x;
                s.sub_cam_eye_max_vel_frac.y = s.sub_cam_eye_max_vel_frac.z;
                s.sub_cam_at_max_vel_frac = s.sub_cam_eye_max_vel_frac;
                self.timer -= 1;
                if self.timer == 0 {
                    s.cs_state += 1;
                    self.timer = 40;
                }
            }
            INTRO_LOOK_SUPPORT => {
                self.timer -= 1;
                if self.timer == 0 {
                    let s = st(play);
                    s.sub_cam_at_next = Vec3::new(10.0, 50.0, -220.0);
                    s.sub_cam_at_max_vel_frac = Vec3::ZERO;
                    s.sub_cam_eye_max_vel_frac = Vec3::ZERO;
                    s.cs_state += 2;
                    self.timer = 20;
                }
            }
            INTRO_CALL_BARI => {
                let s = st(play);
                smooth_step_to_f(&mut s.sub_cam_eye_max_vel_frac.x, 14.0, 0.3, 1.0, 0.25);
                s.sub_cam_eye_max_vel_frac.y = s.sub_cam_eye_max_vel_frac.x * 0.7;
                s.sub_cam_eye_max_vel_frac.z = s.sub_cam_eye_max_vel_frac.x;
                s.sub_cam_at_max_vel_frac = s.sub_cam_eye_max_vel_frac;
                s.sub_cam_at_max_vel_frac.z *= 1.75;
                self.timer -= 1;
                if self.timer == 0 {
                    s.cs_state += 1;
                    self.timer = 7500;
                    self.unk_1f2 = 0;
                }
            }
            INTRO_ATTACH_BARI => self.intro_attach_bari(play),
            INTRO_TITLE => {
                self.spark(play, 3, 140, 50.0, 30.0, SPARK_BARI, 10.0, false);
                self.timer -= 1;
                if self.timer == 0 {
                    st(play).cs_state += 1;
                    self.timer = 45;
                }
            }
            INTRO_BRIGHTEN => {
                self.spark(play, 3, 140, 50.0, 30.0, SPARK_BARI, 10.0, false);
                self.timer -= 1;
                if self.timer == 0 {
                    st(play).cs_state += 1;
                    self.timer = 11;
                }
            }
            INTRO_FINISH => {
                self.timer -= 1;
                if self.timer == 0 {
                    let id = st(play).sub_cam_id;
                    play.clear_camera(id);
                    st(play).sub_cam_id = SUB_CAM_ID_DONE;
                    play.stop_manual();
                    play.change_camera_status(CAM_ID_MAIN, CAM_STAT_ACTIVE);
                    Self::player_cs(play, PLAYER_CSACTION_7);
                    st(play).cs_state += 1;
                    play.save.set_event_chk_inf(EVENTCHKINF_BEGAN_BARINADE_BATTLE);
                    let yaw = self.actor.yaw_towards_player.wrapping_add(i16::MIN);
                    if let Some(h) = play.player
                        && let Some(p) = play.actors.actor_mut(h)
                    {
                        p.shape_rot.y = yaw;
                        p.world_rot.y = yaw;
                    }
                }
            }
            BOSSVA_BATTLE => self.setup_body_phase1(play),
            _ => {}
        }
        let cs = st(play).cs_state;
        if cs >= INTRO_BODY_SOUND {
            Self::level_sfx(play, 1.0);
            if cs >= INTRO_CALL_BARI && play.gameplay_frames % 4 == 0 {
                self.spark(play, 1, 100, 50.0, 10.0, SPARK_BODY, 10.0, false);
            }
        }
        self.glow();
        let s = st(play);
        if s.sub_cam_id != SUB_CAM_ID_DONE && s.cs_state <= INTRO_TITLE {
            Self::step_sub_cam(s, 0.075);
            let (id, at, eye) = (s.sub_cam_id, s.sub_cam_at, s.sub_cam_eye);
            play.camera_set_at_eye(id, at, eye);
        }
    }

    /// The sub camera's eye and at stepped towards their next (`Math_SmoothStepToF`, 0.3 of the
    /// way, at most their max velocities, at least `min`).
    fn step_sub_cam(s: &mut Statics, min: f32) {
        smooth_step_to_f(&mut s.sub_cam_eye.x, s.sub_cam_eye_next.x, 0.3, s.sub_cam_eye_max_vel_frac.x, min);
        smooth_step_to_f(&mut s.sub_cam_eye.y, s.sub_cam_eye_next.y, 0.3, s.sub_cam_eye_max_vel_frac.y, min);
        smooth_step_to_f(&mut s.sub_cam_eye.z, s.sub_cam_eye_next.z, 0.3, s.sub_cam_eye_max_vel_frac.z, min);
        smooth_step_to_f(&mut s.sub_cam_at.x, s.sub_cam_at_next.x, 0.3, s.sub_cam_at_max_vel_frac.x, min);
        smooth_step_to_f(&mut s.sub_cam_at.y, s.sub_cam_at_next.y, 0.3, s.sub_cam_at_max_vel_frac.y, min);
        smooth_step_to_f(&mut s.sub_cam_at.z, s.sub_cam_at_next.z, 0.3, s.sub_cam_at_max_vel_frac.z, min);
    }

    /// `BossVa_BodyIntro`'s `INTRO_ATTACH_BARI`: each Bari attached (1: its stick and the light's
    /// flash; 2: six sparks at it), the camera circling him by `timer` (7500 on by `unk_1F2` a
    /// frame); 35000 the boss music, 45000 the light setting and Link held; past 65536 the title
    /// card (the first time).
    fn intro_attach_bari(&mut self, play: &mut PlayState) {
        for i in (1..=10usize).rev() {
            let b = st(play).body_bari[i - 1];
            if b != 0 {
                if b == 1 {
                    audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_STICK);
                    set_spark_env(play);
                    if self.on_ceiling == 0 {
                        self.on_ceiling = 2;
                    }
                } else if b == 2 {
                    self.spark(play, 6, 140, 50.0, 30.0, SPARK_BARI, i as f32, true);
                }
                let s = st(play);
                if s.body_bari[i - 1] <= 2 {
                    s.body_bari[i - 1] += 1;
                }
            }
        }
        smooth_step_to_s(&mut self.unk_1f2, 0x280, 1, 0x32, 0);
        {
            let s = st(play);
            smooth_step_to_f(&mut s.sub_cam_eye_max_vel_frac.x, 14.0, 0.3, 1.0, 0.25);
            s.sub_cam_eye_max_vel_frac.z = s.sub_cam_eye_max_vel_frac.x;
            s.sub_cam_at_max_vel_frac = s.sub_cam_eye_max_vel_frac;
        }
        if self.timer >= 45000 {
            play.env_ctx.light_setting_override = 1;
            Self::player_cs(play, PLAYER_CSACTION_8);
        } else if self.timer >= 35000 {
            play.audio.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_BOSS));
        }
        self.timer += self.unk_1f2 as i32;
        if self.timer >= 65536 {
            {
                let s = st(play);
                s.sub_cam_eye_max_vel_frac.y = 9.8;
                s.sub_cam_at_max_vel_frac.y = 9.8;
                s.cs_state += 1;
                s.sub_cam_eye_next.x = 10.0;
                s.sub_cam_eye_next.z = 0.0;
                s.sub_cam_at_next = Vec3::new(10.0, 140.0, -200.0);
            }
            if !play.save.get_event_chk_inf(EVENTCHKINF_BEGAN_BARINADE_BATTLE) {
                let &(object, offset, w, h) = &oot_game::title_card::BOSS_NAMES[2];
                play.title_ctx.init_boss_name(object, offset, 160, 180, w, h);
            }
            Self::maybe_bl_spark(play);
            self.timer = 40;
        } else {
            let s = st(play);
            s.sub_cam_eye_max_vel_frac.y = 1.6;
            s.sub_cam_eye_next.y = 5.0;
            s.sub_cam_eye_next.x = sin_s(self.timer as i16) * 200.0;
            s.sub_cam_eye_next.z = (cos_s(self.timer as i16) * 200.0) + -200.0;
        }
    }

    /// `BossVa_SetupBodyPhase1`.
    fn setup_body_phase1(&mut self, play: &mut PlayState) {
        let l = last(&self.anims.body);
        self.change(self.anims.body.clone(), 1.0, l, l, ANIMMODE_ONCE, 0.0);
        self.actor.shape_y_offset = -450.0;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.timer = 25;
        st(play).body_state = 0x80;
        self.action = Action::BodyPhase1;
    }

    /// The body's AT hit on Link: knocked back (8, 8), no damage of its own.
    fn knock_back_link(&mut self, play: &mut PlayState) -> bool {
        if self.body_collider.base.at_flags & AT_HIT != 0 {
            self.body_collider.base.at_flags &= !AT_HIT;
            if self.body_collider.base.at.is_some() && self.body_collider.base.at == play.player {
                actor_set_player_knockback_large_no_damage(play, 8.0, self.actor.yaw_towards_player, 8.0);
                return true;
            }
        }
        false
    }

    /// `BossVa_BodyPhase1`: waiting under his supports; their cuts (`sBodyState`'s low bits)
    /// restart his animation blue; at its end with three cut, phase 2.
    fn body_phase1(&mut self, play: &mut PlayState) {
        self.glow();
        if self.timer != 0 {
            self.timer -= 1;
            if self.timer == 0 {
                st(play).body_state &= !0x80u8;
            }
        }
        self.knock_back_link(play);
        if st(play).body_state & 0x7F != 0 {
            self.skel.cur_frame = 0.0;
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, 12);
            audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_DAMAGE);
        }
        if self.skel.update() && st(play).fight_phase >= PHASE_2 {
            self.setup_body_phase2(play);
        }
        smooth_step_to_s(&mut self.actor.shape_rot.x, self.actor.world_rot.x, 1, 0xC8, 0);
        smooth_step_to_s(&mut self.actor.shape_rot.z, self.actor.world_rot.z, 1, 0xC8, 0);
        self.pulse();
        if play.gameplay_frames % 4 == 0 {
            self.spark(play, 1, 100, 50.0, 10.0, SPARK_BARI, 10.0, false);
        }
        Self::maybe_bl_spark(play);
        self.body_collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.body_collider);
        play.collision_check_set_at(&self.actor, 0, &mut self.body_collider);
        Self::level_sfx(play, 1.0);
    }

    /// `BossVa_SetupBodyPhase2`: the phase on, the five upper Bari.
    fn setup_body_phase2(&mut self, play: &mut PlayState) {
        st(play).fight_phase += 1;
        for i in (BOSSVA_BARI_UPPER_1..=BOSSVA_BARI_UPPER_5).rev() {
            self.spawn_part(play, i);
        }
        self.invincibility_timer = 0;
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::BodyPhase2;
    }

    /// `BossVa_BodyPhase2`. @bug (game): `invincibilityTimer` (an `s8`) is checked to be over 160,
    /// which it can never be.
    fn body_phase2(&mut self, play: &mut PlayState) {
        if self.actor.color_filter_timer == 0 {
            let s = st(play);
            s.phase2_timer = s.phase2_timer.wrapping_add(1);
            if self.invincibility_timer != 0 && self.actor.color_filter_params & 0x4000 != 0 {
                self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, 160);
                self.actor.color_filter_timer = self.invincibility_timer as u8;
            } else {
                self.body_collider.info.ac_dmg_info.dmg_flags = DMG_BOOMERANG;
            }
        }
        if self.body_collider.base.ac_flags & AC_HIT != 0 {
            self.body_collider.base.ac_flags &= !AC_HIT;
            if actor_id(play, self.body_collider.base.ac) == Some(ACTOR_EN_BOOM) {
                let s = st(play);
                s.phase2_timer &= 0xFE00;
                self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, 160);
                self.body_collider.info.ac_dmg_info.dmg_flags = DMG_SWORD | DMG_BOOMERANG | DMG_DEKU_STICK;
            } else {
                st(play).kill_bari += 1;
                if self.actor.color_filter_timer != 0 && self.actor.color_filter_params & 0x4000 == 0 {
                    self.invincibility_timer = (self.actor.color_filter_timer as i32 - 5) as i8;
                    if self.invincibility_timer as i32 > 160 {
                        self.invincibility_timer = 0;
                    }
                }
                self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 12);
            }
            audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_FAINT);
        }
        if self.body_collider.base.at_flags & AT_HIT != 0 {
            self.body_collider.base.at_flags &= !AT_HIT;
            let s = st(play);
            s.phase2_timer = (s.phase2_timer.wrapping_add(0x18)) & 0xFFF0;
            if self.body_collider.base.at.is_some() && self.body_collider.base.at == play.player {
                actor_set_player_knockback_large_no_damage(play, 8.0, self.actor.yaw_towards_player, 8.0);
                if let Some(p) = play.player {
                    audio_play_actor_sfx_at(play, p, NA_SE_PL_BODY_HIT);
                }
            }
        }
        let t = st(play).phase2_timer;
        if t > 10 && t & 7 == 0 && self.actor.speed_xz == 1.0 {
            let mut sp48 = self.actor.world_pos;
            sp48.y += 310.0 + (self.actor.shape_y_offset * self.actor.scale.y);
            sp48.x += -10.0;
            sp48.z += 220.0;
            let sp = self.spawner(play);
            with_effects(play, |play, e| fx::spawn_spark_ball(play, e, &sp, sp48, 4));
        }
        Self::maybe_bl_spark(play);
        smooth_step_to_s(&mut self.actor.shape_rot.x, self.actor.world_rot.x, 1, 0xC8, 0);
        smooth_step_to_s(&mut self.actor.shape_rot.z, self.actor.world_rot.z, 1, 0xC8, 0);
        smooth_step_to_f(&mut self.actor.shape_y_offset, -1000.0, 1.0, 20.0, 0.0);
        if st(play).phase2_timer & 0x100 == 0 {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            self.actor.speed_xz = 1.0;
        } else {
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            self.actor.speed_xz = 0.0;
        }
        if self.skel.update() && st(play).fight_phase >= PHASE_3 {
            self.setup_body_phase3(play);
        }
        self.pulse();
        if play.gameplay_frames % 4 == 0 {
            self.spark(play, 1, 100, 50.0, 10.0, SPARK_BODY, 10.0, false);
        }
        self.actor.focus_pos = self.actor.world_pos;
        self.actor.focus_pos.y += 45.0;
        self.body_collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.body_collider);
        if self.actor.color_filter_timer == 0 {
            play.collision_check_set_at(&self.actor, 0, &mut self.body_collider);
        }
        if self.actor.color_filter_timer == 0 || self.actor.color_filter_params & 0x4000 == 0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.body_collider);
        }
        Self::level_sfx(play, (self.head_rot[1] as f32 * 0.00025) + 1.0);
    }

    /// `BossVa_SetupBodyPhase3`.
    fn setup_body_phase3(&mut self, play: &mut PlayState) {
        self.body_collider.info.ac_dmg_info.dmg_flags = DMG_BOOMERANG;
        self.actor.speed_xz = 0.0;
        st(play).phase3_stop_moving = false;
        self.action = Action::BodyPhase3;
    }

    /// `BossVa_BodyPhase3`: spinning after Link (from home, within 400); the boomerang stuns him
    /// 131 frames (`sBodyState` 1); the lower Bari spawned once he's risen to -500.
    fn body_phase3(&mut self, play: &mut PlayState) {
        let sp62 = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
        self.glow();
        if self.body_collider.base.at_flags & AT_HIT != 0 {
            self.body_collider.base.at_flags &= !AT_HIT;
            if self.body_collider.base.at.is_some() && self.body_collider.base.at == play.player {
                actor_set_player_knockback_large_no_damage(play, 8.0, self.actor.yaw_towards_player, 8.0);
                let r = (play.rand.centered_float(0x2EE0 as f32) as i32 as i16).wrapping_add(i16::MIN);
                self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(r);
                if let Some(p) = play.player {
                    audio_play_actor_sfx_at(play, p, NA_SE_PL_BODY_HIT);
                }
            }
        }
        if self.body_collider.base.ac_flags & AC_HIT != 0 {
            self.skel.cur_frame = 0.0;
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, 12);
            audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_FAINT);
            st(play).body_state = 1;
            self.timer = 131;
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        } else {
            st(play).body_state = 0;
            if self.timer == 0 {
                if smooth_step_to_s(&mut self.head_rot[1], 0xFA0, 1, 0x12C, 0) == 0 {
                    if self.actor.speed_xz == 0.0 {
                        self.actor.world_rot.y = self.actor.yaw_towards_player;
                    }
                    smooth_step_to_f(&mut self.actor.speed_xz, 3.0, 1.0, 0.15, 0.0);
                }
                self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            } else {
                self.timer -= 1;
                if self.timer < 35 {
                    st(play).body_state = 0x80;
                }
                smooth_step_to_s(&mut self.head_rot[1], 0, 1, 0x12C, 0);
                smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.2, 0.0);
                smooth_step_to_f(&mut self.actor.shape_y_offset, -1420.0, 1.0, 30.0, 0.0);
            }
        }
        let invincible = play.player.and_then(|h| play.actors.downcast::<crate::player::Player>(h)).is_some_and(|p| p.invincibility_timer != 0);
        if dist_xz(self.actor.world_pos, self.actor.home_pos) >= 400.0 {
            smooth_step_to_s(&mut self.actor.world_rot.y, sp62, 1, 0x3E8, 0);
        } else if invincible {
            smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player.wrapping_add(i16::MIN), 1, 0x12C, 0);
        } else if play.gameplay_frames & 0x80 == 0 {
            smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 1, 0x12C, 0);
        } else {
            smooth_step_to_s(&mut self.actor.world_rot.y, sp62, 1, 0x258, 0);
        }
        if st(play).phase3_stop_moving {
            self.actor.speed_xz = 0.0;
        }
        self.actor.move_forward();
        if self.skel.update() && st(play).fight_phase >= PHASE_4 {
            self.setup_body_phase4(play);
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.head_rot[1]);
        if st(play).fight_phase == PHASE_3 {
            smooth_step_to_f(&mut self.actor.shape_y_offset, -450.0, 1.0, 15.0, 0.0);
        } else {
            smooth_step_to_f(&mut self.actor.shape_y_offset, -810.0, 1.0, 15.0, 0.0);
        }
        if self.actor.shape_y_offset >= -500.0 && st(play).fight_phase == PHASE_3 {
            for i in (BOSSVA_BARI_LOWER_1..=BOSSVA_BARI_LOWER_5).rev() {
                self.spawn_part(play, i);
            }
            st(play).fight_phase += 1;
        }
        self.pulse();
        if play.gameplay_frames % 4 == 0 {
            self.spark(play, 1, 0x64, 50.0, 10.0, SPARK_BODY, 10.0, false);
        }
        self.actor.focus_pos = self.actor.world_pos;
        self.actor.focus_pos.y += 20.0;
        Self::maybe_bl_spark(play);
        self.body_collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.body_collider);
        play.collision_check_set_at(&self.actor, 0, &mut self.body_collider);
        if self.timer == 0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.body_collider);
        }
        Self::level_sfx(play, (self.head_rot[1] as f32 * 0.00025) + 1.0);
    }

    /// `BossVa_SetupBodyPhase4`.
    fn setup_body_phase4(&mut self, play: &mut PlayState) {
        self.unk_1ac = 0;
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        self.head_rot[1] = self.unk_1ac;
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        self.timer2 = (play.rand.zero_one() * 150.0) as i16 + 300;
        let s = st(play);
        s.body_state = 1;
        s.phase4_hp = 4;
        if self.actor.shape_y_offset != 0.0 {
            self.timer = -30;
        }
        self.body_collider.dim.radius = 55;
        self.action = Action::BodyPhase4;
    }

    /// `BossVa_BodyPhase4`: see the module's docs.
    fn body_phase4(&mut self, play: &mut PlayState) {
        let phase = |play: &mut PlayState| (st(play).fight_phase as i32 - PHASE_4 as i32 + 1) as f32;
        let k = phase(play);
        self.unk_1b0 = self.unk_1b0.wrapping_add((k * 1000.0) as i16).wrapping_add(0xCE4);
        self.body_glow = (sin_s(self.unk_1b0) * 50.0) as i16 + 150;
        if self.body_collider.base.at_flags & AT_HIT != 0 {
            self.body_collider.base.at_flags &= !AT_HIT;
            if self.body_collider.base.at.is_some() && self.body_collider.base.at == play.player {
                actor_set_player_knockback_large_no_damage(play, 8.0, self.actor.yaw_towards_player, 8.0);
                let r = (play.rand.centered_float(0x2EE0 as f32) as i32 as i16).wrapping_add(i16::MIN);
                self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(r);
                if let Some(p) = play.player {
                    audio_play_actor_sfx_at(play, p, NA_SE_PL_BODY_HIT);
                }
            }
        }
        Self::maybe_bl_spark(play);
        if self.body_collider.base.ac_flags & AC_HIT != 0 {
            self.body_collider.base.ac_flags &= !AC_HIT;
            self.skel.cur_frame = 0.0;
            if self.timer >= 0 {
                if self.invincibility_timer == 0 {
                    self.invincibility_timer = 8;
                    if self.actor.col_chk_info.damage_reaction != 1 {
                        self.actor.world_rot.y = self.actor.yaw_towards_player;
                        audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_DAMAGE);
                        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 12);
                        let damage = self.actor.col_chk_info.damage as i8;
                        let s = st(play);
                        s.phase4_hp = s.phase4_hp.wrapping_sub(damage);
                        if s.phase4_hp <= 0 {
                            self.timer = 0;
                            s.fight_phase += 1;
                            s.phase4_hp += 3;
                            if s.fight_phase >= PHASE_DEATH {
                                self.setup_body_death(play);
                                enemy_start_finishing_blow(play, &self.actor);
                                return;
                            }
                            self.actor.speed_xz = -10.0;
                            self.timer = -170 - (play.rand.zero_one() * 150.0) as i16 as i32;
                        }
                    } else {
                        self.timer = (play.rand.centered_float(40.0) as i16 + 160) as i32;
                        self.head_rot[1] = 0;
                        self.actor.speed_xz = 0.0;
                        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 125, COLORFILTER_BUFFLAG_OPA, 255);
                        audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_FAINT);
                    }
                }
            } else if actor_id(play, self.body_collider.base.ac) == Some(ACTOR_EN_BOOM) {
                send_boomerang_back(play, self.body_collider.base.ac);
                audio_play_actor_sfx2(play, NA_SE_IT_SHIELD_REFLECT_SW);
            }
        } else if self.timer2 == 0 && self.actor.shape_y_offset == 0.0 {
            self.timer = -220 - (play.rand.zero_one() * 200.0) as i16 as i32;
        } else if self.timer2 != 0 {
            self.timer2 -= 1;
        }
        self.skel.update();
        let k = phase(play);
        if self.timer == 0 {
            smooth_step_to_f(&mut self.actor.shape_y_offset, 0.0, 1.0, (k * 5.0) + 10.0, 0.0);
            if smooth_step_to_s(&mut self.head_rot[1], (k * 500.0) as i16 + 0xFA0, 1, 0x12C, 0) == 0 {
                if self.actor.speed_xz == 0.0 {
                    self.actor.color_filter_timer = 0;
                    self.actor.world_rot.y = self.actor.yaw_towards_player;
                    self.timer2 = (play.rand.zero_one() * 150.0) as i16 + 300;
                }
                smooth_step_to_f(&mut self.actor.speed_xz, (k * 1.5) + 4.0, 1.0, 0.25, 0.0);
            }
            self.body_collider.info.ac_dmg_info.dmg_flags = DMG_BOOMERANG;
        } else {
            smooth_step_to_s(&mut self.head_rot[1], 0, 1, 0x96, 0);
            let knocked = player_state1(play) & PLAYER_STATE1_26 != 0;
            if self.timer > 0 {
                if knocked && self.timer > 35 {
                    self.timer = 35;
                }
                smooth_step_to_f(&mut self.actor.shape_y_offset, -480.0, 1.0, 30.0, 0.0);
                self.body_collider.info.ac_dmg_info.dmg_flags = DMG_SWORD | DMG_BOOMERANG | DMG_DEKU_STICK;
                self.timer -= 1;
            } else {
                if knocked && self.timer < -60 {
                    self.timer = -59;
                }
                if play.gameplay_frames % 4 == 0 {
                    self.spark(play, 2, 0x64, 220.0, 5.0, SPARK_BODY, 12.0, true);
                }
                if self.timer < -30 {
                    if self.actor.speed_xz > 0.0 {
                        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
                    }
                    smooth_step_to_f(&mut self.actor.shape_y_offset, -1400.0, 1.0, 60.0, 0.0);
                } else {
                    if self.actor.speed_xz == 0.0 {
                        self.actor.world_rot.y = self.actor.yaw_towards_player.wrapping_add(i16::MIN);
                        self.timer2 = (play.rand.zero_one() * 150.0) as i16 + 330;
                    }
                    smooth_step_to_s(&mut self.head_rot[1], 0xFA0, 1, 0x1F4, 0);
                    let tmpf1 = phase(play);
                    smooth_step_to_f(&mut self.actor.speed_xz, (tmpf1 + tmpf1) + 4.0, 1.0, 0.25, 0.0);
                    smooth_step_to_f(&mut self.actor.shape_y_offset, 0.0, 1.0, 20.0, 0.0);
                }
                self.timer += 1;
            }
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.head_rot[1]);
        if self.actor.speed_xz < 0.0 {
            smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
        }
        self.pulse();
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.actor.bg_check_flags &= !BGCHECKFLAG_WALL;
            self.actor.world_rot.y = (play.rand.centered_float((30 * (0x10000 / 360)) as f32) as i16).wrapping_add(self.actor.wall_yaw);
        }
        if st(play).fight_phase <= PHASE_4 {
            let invincible = play.player.and_then(|h| play.actors.downcast::<crate::player::Player>(h)).is_some_and(|p| p.invincibility_timer != 0);
            if dist_xz(self.actor.world_pos, self.actor.home_pos) >= 400.0 {
                let yaw = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
                smooth_step_to_s(&mut self.actor.world_rot.y, yaw, 1, 0x5DC, 0);
            } else if invincible {
                smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player.wrapping_add(i16::MIN), 1, 0x12C, 0);
            } else if play.gameplay_frames & 0x80 == 0 {
                let k = phase(play);
                smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 1, (k * 100.0) as i16 + 0x64, 0);
            }
        }
        self.actor.move_forward();
        self.actor.focus_pos = self.actor.world_pos;
        self.actor.focus_pos.y += 60.0;
        if play.gameplay_frames % 2 == 0 && self.timer == 0 {
            self.spark(play, 2, 125, 40.0, 10.0, SPARK_BODY, 10.0, false);
            self.spark(play, 1, 100, 15.0, 10.0, SPARK_BARI, 11.0, true);
        }
        self.actor.update_bg_check_info(&play.col, 30.0, 70.0, 0.0, UPDBGCHECKINFO_FLAG_0);
        self.body_collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.body_collider);
        if self.invincibility_timer == 0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.body_collider);
        }
        if self.head_rot[1] > 0x3E8 || self.actor.shape_y_offset < -1200.0 {
            play.collision_check_set_at(&self.actor, 0, &mut self.body_collider);
        }
        Self::level_sfx(play, (self.head_rot[1] as f32 * 0.00025) + 1.0);
        let s = st(play);
        if self.invincibility_timer != 0 {
            self.invincibility_timer -= 1;
            s.body_state = (s.body_state & 0x80) | 2;
        } else {
            s.body_state = (s.body_state & 0x80) | 1;
        }
    }

    /// `BossVa_SetupBodyDeath`: the music stopped, the room cleared.
    fn setup_body_death(&mut self, play: &mut PlayState) {
        Self::level_sfx(play, 1.0);
        self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE);
        play.audio.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_MAIN, 1));
        self.head_rot[0] = 0xC31;
        st(play).cs_state = DEATH_START;
        self.actor.speed_xz = 0.0;
        self.unk_1a8 = 0.0;
        let room = play.room_ctx.cur.num;
        play.flags.set_clear(room);
        self.action = Action::BodyDeath;
    }

    /// `BossVa_BodyDeath`.
    fn body_death(&mut self, play: &mut PlayState) {
        let mut state = st(play).cs_state;
        if state == DEATH_START {
            Self::player_cs(play, PLAYER_CSACTION_1);
            play.start_manual();
            let id = play.create_sub_camera();
            play.change_camera_status(CAM_ID_MAIN, CAM_STAT_WAIT);
            play.change_camera_status(id, CAM_STAT_ACTIVE);
            let (eye, at) = (play.game_camera.eye, play.game_camera.at);
            let w = self.actor.world_pos;
            let s = st(play);
            s.sub_cam_id = id;
            s.sub_cam_at_next = w;
            s.sub_cam_at = at;
            s.sub_cam_eye = eye;
            s.sub_cam_eye_next = eye;
            s.sub_cam_eye_next.y = 40.0;
            s.sub_cam_at_next.y = 140.0;
            s.sub_cam_at_max_vel_frac = Vec3::ZERO;
            s.sub_cam_eye_max_vel_frac = Vec3::ZERO;
            self.unk_1ac = vec3f_yaw(s.sub_cam_eye, s.sub_cam_at_next).wrapping_sub(0x100);
            self.unk_1b0 = 15;
            set_screen_fill_color(play, [0xFF, 0xFF, 0xFF, 0]);
            set_fill_screen(play, true);
            st(play).cs_state += 1;
            state = DEATH_BODY_TUMORS;
        }
        match state {
            DEATH_BODY_TUMORS => {
                self.unk_1ac = self.unk_1ac.wrapping_add(0x100);
                let tmp16;
                {
                    let s = st(play);
                    s.sub_cam_eye_next.x = (sin_s(self.unk_1ac) * (160.0 + self.unk_1a8)) + s.sub_cam_at_next.x;
                    s.sub_cam_eye_next.z = (cos_s(self.unk_1ac) * (160.0 + self.unk_1a8)) + s.sub_cam_at_next.z;
                    smooth_step_to_f(&mut s.sub_cam_eye_max_vel_frac.x, 16.0, 0.4, 1.5, 0.5);
                    s.sub_cam_eye_max_vel_frac.z = s.sub_cam_eye_max_vel_frac.x;
                    s.sub_cam_eye_max_vel_frac.y = s.sub_cam_eye_max_vel_frac.x * 0.5;
                    s.sub_cam_at_max_vel_frac = s.sub_cam_eye_max_vel_frac;
                    tmp16 = s.sub_cam_eye_max_vel_frac.x;
                }
                let tmp16 = (play.rand.centered_float(0.5) + ((tmp16 * 0.5) + 0.6)) as i16;
                if play.gameplay_frames % 4 == 0 && self.unk_1b0 != 0 {
                    for i in (2..=6).rev() {
                        self.tumor(play, 1, tmp16, 0.0, 0.0, TUMOR_BODY, i as f32, true);
                    }
                    self.tumor(play, 1, tmp16, 0.0, 0.0, TUMOR_BODY, 11.0, true);
                    self.unk_1b0 -= 1;
                }
                if self.unk_1b0 == 0 {
                    let s = st(play);
                    s.cs_state += 1;
                    s.sub_cam_at_max_vel_frac = Vec3::ZERO;
                    s.sub_cam_eye_max_vel_frac = Vec3::ZERO;
                }
            }
            DEATH_CORE_DEAD => {
                self.unk_1ac = self.unk_1ac.wrapping_add(0x1862);
                self.unk_1a0 = (cos_s(self.unk_1ac) * 0.12) + 1.0;
                self.unk_1a4 = (sin_s(self.unk_1ac) * 0.075) + 1.0;
                if self.is_dead == 0 {
                    self.burst += 1;
                    self.is_dead += 1;
                    self.timer = 30;
                    st(play).cs_state += 1;
                    let pos = self.actor.projected_pos;
                    play.with_ss(|s| s.dead_sound_spawn_stationary(pos, NA_SE_EN_BALINADE_DEAD, 1, 1, 0x28));
                    self.on_ceiling = 2;
                    set_death_env(play);
                    Self::player_cs(play, PLAYER_CSACTION_8);
                }
            }
            DEATH_CORE_BURST => {
                if self.timer == 13 {
                    play.audio.queue_seq_cmd(start_seq(SEQ_PLAYER_BGM_MAIN, 0, NA_BGM_BOSS_CLEAR));
                }
                self.timer -= 1;
                if self.timer == 0 {
                    let (ppos, prot) = player_actor(play).map_or((Vec3::ZERO, 0), |p| (p.world_pos, p.shape_rot.y));
                    let w = self.actor.world_pos;
                    let s = st(play);
                    s.sub_cam_at_next = Vec3::new(w.x, w.y + 30.0, w.z);
                    s.sub_cam_eye_next.x = (sin_s(prot) * -130.0) + ppos.x;
                    s.sub_cam_eye_next.z = (cos_s(prot) * -130.0) + ppos.z;
                    s.sub_cam_eye_next.y = ppos.y + 55.0;
                    s.sub_cam_at_max_vel_frac = Vec3::ZERO;
                    s.sub_cam_eye_max_vel_frac = Vec3::ZERO;
                    s.cs_state += 1;
                    self.timer = 133;
                }
            }
            DEATH_MUSIC | DEATH_FINISH => {
                if state == DEATH_MUSIC {
                    {
                        let s = st(play);
                        smooth_step_to_f(&mut s.sub_cam_eye_max_vel_frac.x, 1.5, 0.3, 0.05, 0.015);
                        s.sub_cam_eye_max_vel_frac.z = s.sub_cam_eye_max_vel_frac.x;
                        s.sub_cam_eye_max_vel_frac.y = s.sub_cam_eye_max_vel_frac.z;
                        s.sub_cam_at_max_vel_frac = s.sub_cam_eye_max_vel_frac;
                    }
                    self.timer -= 1;
                    if self.timer == 0 {
                        self.death_finish(play);
                    }
                }
                // DEATH_FINISH.
                play.rand.centered_float(0.5);
                set_fill_screen(play, false);
            }
            _ => {}
        }
        let s = st(play);
        if s.sub_cam_id != SUB_CAM_ID_DONE {
            Self::step_sub_cam(s, 0.15);
            let (id, at, eye) = (s.sub_cam_id, s.sub_cam_at, s.sub_cam_eye);
            play.camera_set_at_eye(id, at, eye);
        }
        self.skel.update();
        smooth_step_to_f(&mut self.actor.shape_y_offset, -480.0, 1.0, 30.0, 0.0);
        smooth_step_to_s(&mut self.head_rot[1], 0, 1, 0xC8, 0);
        smooth_step_to_s(&mut self.head_rot[0], 0, 1, 0xC8, 0);
        smooth_step_to_s(&mut self.body_glow, 200, 1, 10, 0);
        let mut c = screen_fill_color(play);
        if c[3] != 0 {
            c[3] = c[3].wrapping_sub(50);
            set_screen_fill_color(play, c);
        }
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.head_rot[1]);
        self.unk_1ac = self.unk_1ac.wrapping_add(self.head_rot[0]);
        self.unk_1a0 = (cos_s(self.unk_1ac) * 0.1) + 1.0;
        self.unk_1a4 = (sin_s(self.unk_1ac) * 0.05) + 1.0;
    }

    /// `BossVa_BodyDeath`'s `DEATH_MUSIC` at 0: the camera back on the main one where the sub one
    /// was, Link let go, the heart container at him and Ruto (`En_Ru1` type 0) at `sWarpPos`.
    fn death_finish(&mut self, play: &mut PlayState) {
        let (id, eye, at) = {
            let s = st(play);
            (s.sub_cam_id, s.sub_cam_eye, s.sub_cam_at)
        };
        play.clear_camera(id);
        st(play).sub_cam_id = SUB_CAM_ID_DONE;
        play.stop_manual();
        play.change_camera_status(CAM_ID_MAIN, CAM_STAT_ACTIVE);
        play.game_camera.eye = eye;
        play.game_camera.eye_next = eye;
        play.game_camera.at = at;
        Self::player_cs(play, PLAYER_CSACTION_7);
        st(play).cs_state += 1;
        if let Err(e) = play.actor_spawn(ACTOR_ITEM_B_HEART, self.actor.world_pos, [0; 3], 0) {
            log::warn!("Boss_Va: the heart container not spawned: {e:?}");
        }
        let i = warp_index(player_pos(play));
        if let Err(e) = play.actor_spawn(ACTOR_EN_RU1, S_WARP_POS[i], [0; 3], 0) {
            log::warn!("Boss_Va: Ruto not spawned: {e:?}");
        }
    }

    // ------------------------------------------------------------------------------------------
    // The supports
    // ------------------------------------------------------------------------------------------

    /// `BossVa_SetupSupportIntro`.
    fn setup_support_intro(&mut self) {
        self.change(self.anims.support_attached.clone(), 0.0, 0.0, last(&self.anims.support_attached), ANIMMODE_LOOP_INTERP, 0.0);
        self.timer = 0;
        self.action = Action::SupportIntro;
    }

    /// `BossVa_SupportIntro`.
    fn support_intro(&mut self, play: &mut PlayState) {
        self.attach_to_body(play);
        let cs = st(play).cs_state;
        if cs == BOSSVA_BATTLE {
            self.setup_support_attached();
        } else if cs >= INTRO_REVERSE_CAMERA {
            self.timer += 1;
            if self.timer % 2 == 0 {
                self.spark(play, 2, 90, 5.0, 0.0, SPARK_BODY, ((self.timer & 0x20) >> 5) as f32 + 1.0, true);
            }
            self.skel.update();
            smooth_step_to_f(&mut self.skel.play_speed, 1.0, 1.0, 0.05, 0.0);
            Self::maybe_bl_spark(play);
        }
    }

    /// `BossVa_SetupSupportAttached`.
    fn setup_support_attached(&mut self) {
        self.change(self.anims.support_attached.clone(), 1.0, 0.0, last(&self.anims.support_attached), ANIMMODE_LOOP, 0.0);
        self.timer = self.ty() as i32 * 10;
        self.action = Action::SupportAttached;
    }

    /// `BossVa_SupportAttached`: holding him; a hit (the boomerang alone: its sphere's `0x10`) cuts
    /// it.
    fn support_attached(&mut self, play: &mut PlayState) {
        self.timer += 1;
        if st(play).body_state & 0x7F != 0 {
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, 12);
            let a = if play.rand.zero_one() > 0.5 { self.anims.support_damage1.clone() } else { self.anims.support_damage2.clone() };
            let l = last(&a);
            self.change(a, 1.0, 0.0, l, ANIMMODE_ONCE, 0.0);
        }
        if self.skel.update() {
            self.change(self.anims.support_attached.clone(), 1.0, 0.0, last(&self.anims.support_attached), ANIMMODE_LOOP, 0.0);
        }
        self.attach_to_body(play);
        Self::maybe_bl_spark(play);
        if self.collider_jnt_sph.base.ac_flags & AC_HIT != 0 {
            self.setup_support_cut(play);
        } else {
            if self.actor.color_filter_timer == 0 {
                play.collision_check_set_ac(&self.actor, 1, &mut self.collider_jnt_sph);
            }
            if self.timer % 2 == 0 {
                self.spark(play, 1, 100, 5.0, 0.0, SPARK_BODY, ((self.timer & 0x20) >> 5) as f32 + 1.0, true);
            }
        }
    }

    /// `BossVa_SetupSupportCut`: `sBodyState` and the phase on, its stump at its tip, a quake.
    fn setup_support_cut(&mut self, play: &mut PlayState) {
        let stump_params = self.ty() + BOSSVA_STUMP_1;
        {
            let s = st(play);
            s.body_state = s.body_state.wrapping_add(1);
            s.fight_phase += 1;
        }
        let pos = Vec3::new(self.arm_tip.x, self.arm_tip.y + 20.0, self.arm_tip.z);
        if let Err(e) = play.actor_spawn(ACTOR_BOSS_VA, pos, [0, self.actor.shape_rot.y, 0], stump_params) {
            log::warn!("Boss_Va: the stump not spawned: {e:?}");
        }
        play.camera_request_quake(CAM_ID_MAIN, 11, 8);
        self.burst = 0;
        self.timer2 = 0;
        self.action = Action::SupportCut;
    }

    /// `BossVa_SupportCut`.
    fn support_cut(&mut self, play: &mut PlayState) {
        self.attach_to_body(play);
        if self.on_ceiling != 0 {
            let l = last(&self.anims.support_cut);
            self.on_ceiling = 0;
            self.timer = (play.rand.zero_one() * 10.0) as i32 + 5;
            // SkelAnime_Free, SkelAnime_InitFlex(gBarinadeCutSupportSkel, gBarinadeSupportCutAnim).
            if let Some(cut) = self.cut_skeleton.clone() {
                self.skel = SkelAnimeStd::init_flex(cut.limbs.len(), self.anims.support_cut.clone());
                self.skeleton = Some(cut);
            }
            self.cut = true;
            self.change(self.anims.support_cut.clone(), 1.0, 0.0, l, ANIMMODE_ONCE, 0.0);
            st(play).body_state = 0;
            let p = self.ty();
            if let Some(b) = self.body_mut(play) {
                b.actor.shape_y_offset -= 60.0;
                match p {
                    BOSSVA_SUPPORT_1 => b.actor.world_rot.x = b.actor.world_rot.x.wrapping_add(0x4B0),
                    BOSSVA_SUPPORT_2 => {
                        b.actor.world_rot.x = b.actor.world_rot.x.wrapping_sub(0x258);
                        b.actor.world_rot.z = b.actor.world_rot.z.wrapping_sub(0x4E2);
                    }
                    BOSSVA_SUPPORT_3 => {
                        b.actor.world_rot.x = b.actor.world_rot.x.wrapping_sub(0x258);
                        b.actor.world_rot.z = b.actor.world_rot.z.wrapping_add(0x4E2);
                    }
                    _ => {}
                }
            }
        }
        let spin = self.body(play).map_or(0, |b| b.head_rot[1]);
        smooth_step_to_s(&mut self.head_rot[0], spin.wrapping_mul(-3), 1, 0x4B0, 0);
        if self.skel.update() {
            let l = last(&self.anims.support_detached);
            self.change(self.anims.support_detached.clone(), 1.0, 0.0, l, ANIMMODE_LOOP_INTERP, 0.0);
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        }
        let cs = st(play).cs_state;
        if self.timer == 0 && cs < DEATH_START {
            self.timer = (play.rand.zero_one() * 10.0) as i32 + 10;
            let (tip, phase, yaw) = (self.arm_tip, self.head_rot[0], self.actor.shape_rot.y);
            with_effects(play, |play, e| fx::blood_droplets(play, e, tip, phase, yaw));
        }
        if cs >= DEATH_START {
            smooth_step_to_f(&mut self.skel.play_speed, 0.0, 0.3, 0.25, 0.125);
        }
        let mut cs = st(play).cs_state;
        if cs == DEATH_SHELL_BURST {
            let s = st(play);
            s.sub_cam_eye = s.sub_cam_eye_next;
            s.sub_cam_at = s.sub_cam_at_next;
            smooth_step_to_f(&mut s.sub_cam_eye.x, s.sub_cam_at_next.x, 1.0, 10.0, 0.0);
            smooth_step_to_f(&mut s.sub_cam_eye.z, s.sub_cam_at_next.z, 1.0, 10.0, 0.0);
            s.sub_cam_eye.y += 20.0;
            s.cs_state += 1;
            cs = DEATH_CORE_TUMORS;
        }
        if (DEATH_CORE_TUMORS..=DEATH_CORE_BURST).contains(&cs) {
            if self.burst == 0 {
                if play.gameplay_frames % 2 != 0 {
                    let scale = play.rand.centered_float(5.0) as i16 + 6;
                    self.tumor(play, 1, scale, 7.0, 5.0, TUMOR_ARM, (self.timer2 >> 3) as f32 + 1.0, true);
                }
                self.timer2 += 1;
                if self.timer2 >= 32 {
                    self.burst += 1;
                    self.is_dead = 1;
                    audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_BREAK2);
                    if self.ty() == BOSSVA_SUPPORT_3 {
                        st(play).cs_state += 1;
                    }
                }
            } else {
                self.timer2 -= 1;
                if self.timer2 == 0 {
                    self.actor.kill();
                }
            }
        }
        self.timer -= 1;
    }

    // ------------------------------------------------------------------------------------------
    // The stumps and the door
    // ------------------------------------------------------------------------------------------

    /// `BossVa_SetupStump`.
    fn setup_stump(&mut self) {
        self.change(self.anims.stump.clone(), 1.0, 0.0, last(&self.anims.stump), ANIMMODE_ONCE, 0.0);
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::Stump;
    }

    /// `BossVa_Stump`: twitching (a 3 in 10 at its animation's end, up to 3 frames back); gone as
    /// he dies.
    fn stump(&mut self, play: &mut PlayState) {
        if self.skel.update() && play.rand.zero_one() < 0.3 {
            self.skel.cur_frame -= play.rand.zero_one() * 3.0;
        }
        if st(play).cs_state >= DEATH_START {
            self.actor.kill();
        }
    }

    /// `BossVa_SetupDoor`.
    fn setup_door(&mut self, play: &mut PlayState) {
        if st(play).cs_state >= INTRO_SPAWN_BARI {
            st(play).door_state = 100;
        }
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::Door;
    }

    /// `BossVa_Door`: closing 8 a frame to 100 until the intro's `INTRO_DOOR_SHUT`.
    fn door(&mut self, play: &mut PlayState) {
        if st(play).door_state == 29 {
            audio_play_actor_sfx2(play, NA_SE_EV_BUYODOOR_CLOSE);
        }
        let s = st(play);
        if s.cs_state <= INTRO_DOOR_SHUT {
            if s.door_state < 100 {
                s.door_state += 8;
            } else {
                s.door_state = 100;
            }
        }
    }

    // ------------------------------------------------------------------------------------------
    // The zappers
    // ------------------------------------------------------------------------------------------

    /// `BossVa_SetupZapperIntro`.
    fn setup_zapper_intro(&mut self) {
        let l = last(&self.anims.zapper_idle);
        self.change(self.anims.zapper_idle.clone(), 1.0, l - 1.0, l, ANIMMODE_LOOP_INTERP, -6.0);
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::ZapperIntro;
    }

    /// `BossVa_ZapperIntro`.
    fn zapper_intro(&mut self, play: &mut PlayState) {
        self.attach_to_body(play);
        match st(play).cs_state {
            INTRO_TITLE | INTRO_BRIGHTEN | INTRO_FINISH => {
                self.skel.update();
            }
            BOSSVA_BATTLE => self.setup_zapper_attack(),
            _ => {}
        }
        let t = self.actor.shape_rot.y.wrapping_sub(self.actor.shape_rot.x);
        smooth_step_to_s(&mut self.unk_1f2, t, 1, 0x2EE, 0);
        let j = self.skel.joint_table.get(7).map_or(0, |j| j[2]);
        smooth_step_to_s(&mut self.unk_1f0, j, 1, 0x2EE, 0);
    }

    /// `BossVa_SetupZapperAttack`.
    fn setup_zapper_attack(&mut self) {
        let l = last(&self.anims.zapper_idle);
        self.change(self.anims.zapper_idle.clone(), 1.0, l - 1.0, l, ANIMMODE_LOOP_INTERP, -6.0);
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::ZapperAttack;
    }

    /// `ABS` of an `s16` as the C's int.
    fn abs16(v: i16) -> i32 {
        (v as i32).abs()
    }

    /// The joints' sum the zappers aim from: `shape.rot.x` and joints 1 to 5's `k`.
    fn joint_sum(&self, k: usize) -> i16 {
        let mut v = self.actor.shape_rot.x;
        for i in 1..=5 {
            v = v.wrapping_add(self.skel.joint_table.get(i).map_or(0, |j| j[k]));
        }
        v
    }

    /// The lightning's hit or its end (`BossVa_ZapperAttack`'s and `_Enraged`'s else branch).
    fn zapper_release(&mut self, play: &mut PlayState, step: i16) {
        if self.burst != 0 || self.timer2 < 0 {
            if self.collider_lightning.base.at_flags & AT_HIT != 0 {
                if self.timer2 > 0 {
                    audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_HIT_RINK);
                    set_spark_env(play);
                    self.timer2 = -1;
                    if let Some(b) = self.body_mut(play) {
                        b.on_ceiling = 6;
                    }
                }
            } else if self.timer2 > 0 {
                self.timer2 = 0;
            }
            if self.timer2 < 0 && player_state1(play) & PLAYER_STATE1_26 != 0 {
                self.spark(play, 1, 30, 0.0, 0.0, SPARK_LINK, 0.0, true);
            }
        }
        smooth_step_to_s(&mut self.unk_1e6, 0, 1, step, 0);
        smooth_step_to_s(&mut self.unk_1ec, 0, 1, step, 0);
        smooth_step_to_s(&mut self.unk_1ea, 0, 1, step, 0);
        let t = self.actor.shape_rot.y.wrapping_sub(self.actor.shape_rot.x);
        smooth_step_to_s(&mut self.unk_1f2, t, 1, step, 0);
        let j = self.skel.joint_table.get(7).map_or(0, |j| j[2]);
        smooth_step_to_s(&mut self.unk_1f0, j, 1, step, 0);
        smooth_step_to_f(&mut self.skel.play_speed, 1.0, 1.0, 0.05, 0.0);
        self.burst = 0;
    }

    /// The aim at `target` (`BossVa_ZapperAttack`'s and `_Enraged`'s): its arm, neck and head
    /// turned (each at most `0x1770` off), the steps' sizes summed.
    fn zapper_aim(&mut self, target: Vec3, yaw_to: i16, arm_step: i16, head_step: i16, pitch_step: i16, joint_k: usize) -> u32 {
        let mut sp94 = yaw_to.wrapping_sub(self.actor.shape_rot.y);
        if Self::abs16(sp94) > 0x1770 {
            sp94 = if sp94 > 0 { 0x1770 } else { -0x1770 };
        }
        let mut sp88 = Self::abs16(smooth_step_to_s(&mut self.unk_1e6, sp94, 1, arm_step, 0)) as u32;
        let mut sp94b = yaw_to.wrapping_sub(sp94);
        if Self::abs16(sp94b) > 0x1770 {
            sp94b = if sp94b > 0 { 0x1770 } else { -0x1770 };
        }
        sp88 += Self::abs16(smooth_step_to_s(&mut self.unk_1ec, sp94b, 1, arm_step, 0)) as u32;
        let yaw = vec3f_yaw(self.zap_head_pos, target);
        sp88 += Self::abs16(smooth_step_to_s(&mut self.unk_1f2, yaw.wrapping_sub(0x4000), 1, head_step, 0)) as u32;
        let sp96 = self.joint_sum(joint_k);
        let yaw = vec3f_pitch(target, self.zap_neck_pos);
        sp88 += Self::abs16(smooth_step_to_s(&mut self.unk_1ea, yaw.wrapping_sub(sp96), 1, pitch_step, 0)) as u32;
        let yaw = vec3f_pitch(self.zap_head_pos, target);
        sp88 += Self::abs16(smooth_step_to_s(&mut self.unk_1f0, yaw.wrapping_neg(), 1, pitch_step, 0)) as u32;
        sp88
    }

    /// The charge and the lightning (`timer2` from 0): sparks along the arm, the room's flash,
    /// the charge at the head; from `blast_at` the bolt (three sparks at each of `effectPos[4..7]`,
    /// its quad's AT and AC); over at `end`.
    fn zapper_bolt(&mut self, play: &mut PlayState, blast_at: i16, thunder_at: i16, env_at: i16, charge_at: i16, shift: i16, end: i16) {
        if self.burst != 0 && self.burst != 2 {
            if self.timer2 >= blast_at {
                if self.timer2 == thunder_at {
                    audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_THUNDER);
                }
                for range in [5.0, 6.0, 7.0] {
                    self.spark(play, 2, 110, 15.0, 15.0, SPARK_BLAST, range, true);
                }
                play.collision_check_set_at(&self.actor, 2, &mut self.collider_lightning);
                play.collision_check_set_ac(&self.actor, 2, &mut self.collider_lightning);
            } else {
                self.spark(play, 2, 50, 15.0, 0.0, SPARK_BODY, (self.timer2 >> shift) as f32 + 1.0, true);
                if self.timer2 == env_at {
                    set_spark_env(play);
                }
                if self.timer2 == charge_at {
                    let sp = self.spawner(play);
                    let (pos, rot) = (self.zap_head_pos, self.head_rot);
                    with_effects(play, |play, e| fx::spawn_zapper_charge(play, e, &sp, pos, rot, 100, 0));
                }
            }
            self.timer2 += 1;
            if self.timer2 >= end {
                self.burst = 0;
            }
        }
    }

    /// `BossVa_ZapperAttack`: aiming at Link (10 up), or where the boomerang will be when it
    /// comes back from what it flies to (its path stepped on from `returnTimer` to 3); when aimed
    /// (the steps' sum under `0x1F4`, `0x3E80` for the boomerang), the charge and the bolt.
    fn zapper_attack(&mut self, play: &mut PlayState) {
        let player = play.player;
        let boom = find_boomerang(play).and_then(|h| play.actors.downcast::<crate::en_boom::EnBoom>(h));
        let mut sp90: i16 = 0x1F4;
        let (sp7c, sp8e) = match boom {
            Some(b) if b.move_to.is_some() && b.move_to != player => {
                let sp74 = play.r_update_rate as f32 * 0.5;
                let target = b.move_to.and_then(|h| play.actors.actor(h)).map_or(Vec3::ZERO, |a| a.focus_pos);
                let mut p = b.actor.world_pos;
                let mut sp6c = b.actor.world_rot.y;
                let mut sp56 = b.actor.world_rot.x;
                let mut i = b.return_timer as i32;
                while i >= 3 {
                    let sp6e = vec3f_yaw(p, target);
                    let mut sp5a = sp6c.wrapping_sub(sp6e);
                    let sp58 = vec3f_pitch(p, target);
                    let mut sp54 = sp56.wrapping_sub(sp58);
                    let mut sp50 = (200.0 - p.distance(target)) * 0.005;
                    if sp50 < 0.12 {
                        sp50 = 0.12;
                    }
                    if sp5a < 0 {
                        sp5a = sp5a.wrapping_neg();
                    }
                    if sp54 < 0 {
                        sp54 = sp54.wrapping_neg();
                    }
                    scaled_step_to_s(&mut sp6c, sp6e, (sp5a as f32 * sp50) as i16);
                    scaled_step_to_s(&mut sp56, sp58, (sp54 as f32 * sp50) as i16);
                    let sp68 = -sin_s(sp56) * 12.0;
                    let sp5c = cos_s(sp56) * 12.0;
                    let sp64 = sin_s(sp6c) * sp5c;
                    let sp60 = cos_s(sp6c);
                    p.x += sp64 * sp74;
                    p.y += sp68 * sp74;
                    p.z += sp60 * sp5c * sp74;
                    i -= 1;
                }
                sp90 = 0x3E80;
                (p, 0x4650i16)
            }
            _ => (player_pos(play) + Vec3::new(0.0, 10.0, 0.0), 0x3E80i16),
        };
        self.skel.update();
        self.attach_to_body(play);
        let (phase, body_state) = (st(play).fight_phase, st(play).body_state);
        if phase >= PHASE_4 {
            self.setup_zapper_enraged();
            return;
        }
        if body_state & 0x7F != 0 {
            self.setup_zapper_damaged(play);
            return;
        }
        if phase < PHASE_4 && self.body(play).is_some_and(|b| b.actor.speed_xz != 0.0) {
            self.setup_zapper_hold();
            return;
        }
        let sp98 = vec3f_yaw(sp7c, self.arm_tip);
        let tmp17 = sp98.wrapping_sub(self.actor.shape_rot.y);
        if (sp8e as i32 >= Self::abs16(tmp17) || self.burst != 0) && body_state & 0x80 == 0 && player_state1(play) & PLAYER_STATE1_26 == 0 {
            if self.burst == 0 {
                let sp88 = self.zapper_aim(sp7c, sp98, 0x6D6, 0x9C4, 0xFA0, 2);
                self.skel.play_speed = 0.0;
                if smooth_step_to_f(&mut self.skel.cur_frame, 0.0, 1.0, 2.0, 0.0) == 0.0 {
                    if sp88 < sp90 as u32 {
                        self.timer2 = 0;
                        self.burst += 1;
                        self.unk_1d8 = sp7c;
                    }
                    Self::maybe_bl_spark(play);
                }
            }
        } else {
            self.zapper_release(play, 0x6D6);
        }
        self.zapper_bolt(play, 32, 32, 30, 20, 3, 40);
    }

    /// `BossVa_SetupZapperDamaged`.
    fn setup_zapper_damaged(&mut self, play: &mut PlayState) {
        let a = if play.rand.zero_one() > 0.5 { self.anims.zapper_damage1.clone() } else { self.anims.zapper_damage2.clone() };
        let l = last(&a);
        self.change(a, 0.5, 0.0, l, ANIMMODE_ONCE_INTERP, 4.0);
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, 12);
        self.burst = 0;
        self.action = Action::ZapperDamaged;
    }

    /// `BossVa_ZapperDamaged`.
    fn zapper_damaged(&mut self, play: &mut PlayState) {
        self.attach_to_body(play);
        smooth_step_to_s(&mut self.unk_1e6, 0, 1, 0xFA0, 0);
        smooth_step_to_s(&mut self.unk_1e4, 0, 1, 0xFA0, 0);
        smooth_step_to_s(&mut self.unk_1ec, 0, 1, 0xFA0, 0);
        smooth_step_to_s(&mut self.unk_1ea, 0, 1, 0xFA0, 0);
        let t = self.actor.shape_rot.y.wrapping_sub(self.actor.shape_rot.x);
        smooth_step_to_s(&mut self.unk_1f2, t, 1, 0x2EE, 0);
        let j = self.skel.joint_table.get(7).map_or(0, |j| j[2]);
        smooth_step_to_s(&mut self.unk_1f0, j, 1, 0x2EE, 0);
        if self.skel.update() {
            if st(play).fight_phase >= PHASE_4 {
                self.setup_zapper_enraged();
            } else {
                self.setup_zapper_attack();
            }
        }
    }

    /// `BossVa_SetupZapperDeath`.
    fn setup_zapper_death(&mut self, play: &mut PlayState) {
        let l = last(&self.anims.zapper_idle);
        let speed = play.rand.zero_one() + 0.25;
        let start = play.rand.zero_one() * 3.0;
        self.change(self.anims.zapper_idle.clone(), speed, start, l, ANIMMODE_LOOP_INTERP, -6.0);
        self.burst = 0;
        self.timer2 = (self.ty() * -6) + 18;
        self.unk_1b0 = 0;
        self.action = Action::ZapperDeath;
    }

    /// `BossVa_ZapperDeath`: twitching; its turn in the death (`DEATH_ZAPPER_1` to `_3`) tumours
    /// along its arm and the camera on its neck, then it bursts.
    fn zapper_death(&mut self, play: &mut PlayState) {
        let mut sp3c = 55.0;
        self.attach_to_body(play);
        let cs = st(play).cs_state;
        if play.gameplay_frames % 32 == 0 && cs <= DEATH_BODY_TUMORS {
            self.unk_1e8 = play.rand.centered_float(0x4000 as f32) as i32 as i16;
            self.unk_1ee = play.rand.centered_float(0x4000 as f32) as i32 as i16;
            let r = play.rand.centered_float(0x4000 as f32) as i16;
            self.unk_1f4 = r.wrapping_add(self.actor.shape_rot.y).wrapping_sub(self.actor.shape_rot.x);
        } else {
            smooth_step_to_f(&mut self.skel.play_speed, 0.0, 1.0, 0.025, 0.0);
        }
        self.skel.update();
        let step = play.rand.centered_float(500.0) as i16 + 0x1F4;
        smooth_step_to_s(&mut self.unk_1e6, self.unk_1e8, 1, step, 0);
        smooth_step_to_s(&mut self.unk_1e4, 0, 1, 0x1F4, 0);
        let step = play.rand.centered_float(500.0) as i16 + 0x1F4;
        smooth_step_to_s(&mut self.unk_1ec, self.unk_1ee, 1, step, 0);
        smooth_step_to_s(&mut self.unk_1ea, 0, 1, 0x1F4, 0);
        let step = play.rand.centered_float(500.0) as i16 + 0x1F4;
        smooth_step_to_s(&mut self.unk_1f2, self.unk_1f4, 1, step, 0);
        let cs = st(play).cs_state;
        if !(DEATH_ZAPPER_1..=DEATH_ZAPPER_3).contains(&cs) {
            return;
        }
        if cs == DEATH_ZAPPER_2 {
            sp3c = -55.0;
        }
        let p = self.ty();
        if self.burst == 0 {
            if (p == BOSSVA_ZAPPER_1 && self.timer2 < 16) || (p == BOSSVA_ZAPPER_2 && self.timer2 < 24) || p == BOSSVA_ZAPPER_3 {
                if self.timer2 % 2 == 0 && self.timer2 >= 0 {
                    if self.timer2 < 8 {
                        let scale = play.rand.centered_float(5.0) as i16 + 0xD;
                        self.tumor(play, 1, scale, 0.0, 0.0, TUMOR_ARM, 0.6, true);
                    } else {
                        let scale = play.rand.centered_float(5.0) as i16 + 6;
                        self.tumor(play, 1, scale, 0.0, 7.0, TUMOR_ARM, (self.timer2 >> 3) as f32 + 1.0, true);
                    }
                    self.spark(play, 2, 50, 15.0, 0.0, SPARK_BODY, (self.timer2 >> 3) as f32 + 1.0, true);
                }
                self.timer2 += 1;
                if self.timer2 >= 32 {
                    self.burst += 1;
                    self.is_dead = 1;
                    set_death_env(play);
                    audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_BREAK2);
                }
            } else {
                self.burst += 1;
                self.is_dead = 1;
                self.timer2 = 32;
                st(play).cs_state += 1;
            }
            if (p - BOSSVA_ZAPPER_1 + DEATH_ZAPPER_1 as i16) == st(play).cs_state as i16 {
                let n = self.zap_neck_pos;
                let a = (self.actor.shape_rot.y.wrapping_add(self.unk_1b0)).wrapping_neg();
                let s = st(play);
                s.sub_cam_at.x = n.x;
                s.sub_cam_at.y = n.y;
                s.sub_cam_eye.y = n.y;
                s.sub_cam_at.z = n.z;
                s.sub_cam_eye.x = (cos_s(a) * sp3c) + n.x;
                s.sub_cam_eye.z = (sin_s(a) * sp3c) + n.z;
                self.unk_1b0 = self.unk_1b0.wrapping_add(0x15E);
            }
        } else {
            self.timer2 -= 1;
            if self.timer2 == 0 {
                if p == BOSSVA_ZAPPER_3 {
                    st(play).cs_state += 1;
                }
                self.actor.kill();
            }
        }
    }

    /// `BossVa_SetupZapperEnraged`.
    fn setup_zapper_enraged(&mut self) {
        let l = last(&self.anims.zapper_idle);
        self.change(self.anims.zapper_idle.clone(), 1.0, l - 1.0, l, ANIMMODE_LOOP_INTERP, -6.0);
        self.burst = 0;
        self.action = Action::ZapperEnraged;
    }

    /// `BossVa_ZapperEnraged`: as the attack, always at Link (35 up), quicker (the bolt from 16 of
    /// 24 frames).
    fn zapper_enraged(&mut self, play: &mut PlayState) {
        let mut sp54 = player_pos(play);
        sp54.y += 10.0;
        self.skel.update();
        self.attach_to_body(play);
        let (phase, body_state) = (st(play).fight_phase, st(play).body_state);
        if phase >= PHASE_DEATH {
            self.setup_zapper_death(play);
            return;
        }
        if body_state & 0x7E != 0 {
            self.setup_zapper_damaged(play);
            return;
        }
        sp54.y += 25.0;
        let sp6c = vec3f_yaw(sp54, self.arm_tip);
        let tmp16 = sp6c.wrapping_sub(self.actor.shape_rot.y);
        if (Self::abs16(tmp16) <= 0x4650 || self.burst != 0) && body_state & 0x80 == 0 && player_state1(play) & PLAYER_STATE1_26 == 0 {
            if self.burst == 0 {
                let sp60 = self.zapper_aim(sp54, sp6c, 0xDAC, 0xEA6, 0x1B58, 0);
                self.skel.play_speed = 0.0;
                if smooth_step_to_f(&mut self.skel.cur_frame, 0.0, 1.0, 3.0, 0.0) == 0.0 && sp60 < 0x258 {
                    self.timer2 = 0;
                    self.burst += 1;
                    self.unk_1d8 = sp54;
                    Self::maybe_bl_spark(play);
                }
            }
        } else {
            self.zapper_release(play, 0xEA6);
        }
        self.zapper_bolt(play, 16, 18, 14, 4, 1, 24);
    }

    /// `BossVa_SetupZapperHold`.
    fn setup_zapper_hold(&mut self) {
        self.change(self.anims.zapper_damage2.clone(), 0.0, 0.0, last(&self.anims.zapper_damage2), ANIMMODE_ONCE_INTERP, -6.0);
        self.burst = 0;
        self.action = Action::ZapperHold;
    }

    /// `BossVa_ZapperHold`: folded while the body moves.
    fn zapper_hold(&mut self, play: &mut PlayState) {
        self.skel.update();
        self.attach_to_body(play);
        smooth_step_to_s(&mut self.unk_1e6, 0, 1, 0x1770, 0);
        smooth_step_to_s(&mut self.unk_1e4, 0, 1, 0x1770, 0);
        smooth_step_to_s(&mut self.unk_1ec, 0, 1, 0x1770, 0);
        smooth_step_to_s(&mut self.unk_1ea, 0, 1, 0x1770, 0);
        smooth_step_to_s(&mut self.unk_1f2, self.actor.shape_rot.y.wrapping_sub(0x4000), 1, 0x2710, 0);
        let j = self.skel.joint_table.get(7).map_or(0, |j| j[2]);
        smooth_step_to_s(&mut self.unk_1f0, j.wrapping_sub(0x1388), 1, 0x1770, 0);
        if self.body(play).is_some_and(|b| b.actor.speed_xz == 0.0) {
            self.setup_zapper_attack();
        }
    }

    // ------------------------------------------------------------------------------------------
    // The Bari
    // ------------------------------------------------------------------------------------------

    /// `BossVa_SetupBariIntro`: at its cutscene place (`sInitPosOffsets[params + 10]` from home).
    fn setup_bari_intro(&mut self, play: &mut PlayState) {
        self.change(self.anims.bari.clone(), 1.0, 0.0, last(&self.anims.bari), ANIMMODE_LOOP, 0.0);
        self.unk_1a0 = 60.0;
        self.unk_1a4 = play.rand.zero_one() * 360.0;
        self.timer2 = 64;
        self.unk_1f0 = 120;
        self.unk_1a8 = 0.0;
        if let Some(o) = S_INIT_POS_OFFSETS.get((self.ty() + 10) as usize) {
            self.actor.world_pos = *o + self.actor.home_pos;
        }
        self.timer = 45;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::BariIntro;
    }

    /// `BossVa_BariIntro`.
    fn bari_intro(&mut self, play: &mut PlayState) {
        let mut sp54 = self.actor.home_pos;
        let mut sp50 = 40.0;
        if self.actor.home_pos.y >= 0.0 {
            sp54.y += 25.0;
        }
        self.unk_1a4 += play.rand.zero_one() * 0.25;
        let cs = st(play).cs_state;
        let p = self.ty();
        match cs {
            INTRO_LOOK_BARI => {
                if p == BOSSVA_BARI_UPPER_1 {
                    Self::player_cs(play, PLAYER_CSACTION_1);
                    if smooth_step_to_f(&mut self.actor.world_pos.y, 60.0, 0.3, 1.0, 0.15) == 0.0 {
                        self.timer -= 1;
                        if self.timer == 0 {
                            st(play).cs_state += 1;
                        }
                    }
                }
                self.actor.shape_rot.x = 0;
            }
            INTRO_REVERSE_CAMERA..=INTRO_LOOK_SUPPORT => {
                if p != BOSSVA_BARI_UPPER_1 {
                    let y = S_INIT_POS_OFFSETS.get((p + 10) as usize).map_or(0.0, |o| o.y) + self.actor.home_pos.y;
                    smooth_step_to_f(&mut self.actor.world_pos.y, y, 0.3, 1.0, 0.15);
                    self.actor.world_pos.x += math_sin_f(self.unk_1a4 * 0.25) * 0.5;
                } else {
                    smooth_step_to_f(&mut self.actor.world_pos.y, 60.0, 0.3, 1.0, 0.15);
                }
                self.actor.world_pos.y += math_sin_f(self.unk_1a4) * (2.0 - math_sin_f(self.unk_1a4));
            }
            INTRO_UNUSED_CALL_BARI | INTRO_CALL_BARI | INTRO_ATTACH_BARI => {
                if cs != INTRO_UNUSED_CALL_BARI && self.timer2 > 15 && self.timer < 0 {
                    smooth_step_to_f(&mut self.actor.world_pos.x, sp54.x, 1.0, 6.5, 0.0);
                    smooth_step_to_f(&mut self.actor.world_pos.y, sp54.y, 1.0, 6.5, 0.0);
                    smooth_step_to_f(&mut self.actor.world_pos.z, sp54.z, 1.0, 6.5, 0.0);
                    sp50 = sp54.distance(self.actor.world_pos);
                    if sp50 <= 60.0 {
                        let mut sp4e = Self::abs16(smooth_step_to_s(&mut self.actor.shape_rot.x, self.actor.home_rot.x, 1, 0x7D0, 0)) as i16;
                        sp4e = sp4e.wrapping_add(Self::abs16(smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.home_rot.y, 1, 0x7D0, 0)) as i16);
                        if sp50 == 0.0 && sp4e == 0 {
                            if self.is_dead == 0 {
                                let s = st(play);
                                if p >= BOSSVA_BARI_LOWER_1 {
                                    if p == BOSSVA_BARI_LOWER_1 {
                                        s.body_bari[0] += 1;
                                    } else if let Some(b) = s.body_bari.get_mut((p - BOSSVA_BARI_UPPER_1) as usize) {
                                        *b += 1;
                                    }
                                } else if let Some(b) = s.body_bari.get_mut((p - BOSSVA_BARI_UPPER_1 + 1) as usize) {
                                    *b += 1;
                                }
                                self.timer = -30;
                                self.is_dead += 1;
                            } else {
                                self.timer += 1;
                                if self.timer == 0 {
                                    self.actor.kill();
                                }
                            }
                            return;
                        }
                    }
                }
                // INTRO_UNUSED_CALL_BARI.
                self.timer -= 1;
                if self.timer == 0 {
                    self.timer2 = 0;
                } else {
                    let bpos = self.body(play).map_or(Vec3::ZERO, |b| b.actor.world_pos);
                    let r = crate::en_ba::func_80035844(bpos, self.actor.world_pos, false);
                    self.actor.world_rot.x = r[0];
                    self.actor.world_rot.y = r[1];
                    self.unk_1a0 = bpos.distance(self.actor.world_pos);
                    if sp50 > 30.0 {
                        self.spark(play, 1, 80, 15.0, 0.0, SPARK_BARI, 1.0, true);
                    }
                }
            }
            BOSSVA_BATTLE => {
                self.timer += 1;
                if self.timer == 0 {
                    self.actor.kill();
                }
                return;
            }
            _ => {}
        }
        if play.gameplay_frames % 4 == 0 && st(play).cs_state < INTRO_ATTACH_BARI {
            self.spark(play, 1, 70, 25.0, 20.0, SPARK_BARI, 2.0, true);
        }
        Self::maybe_bl_spark(play);
    }

    /// `BossVa_SetupBariPhase3Attack`.
    fn setup_bari_phase3_attack(&mut self) {
        self.change(self.anims.bari.clone(), 1.0, 0.0, last(&self.anims.bari), ANIMMODE_LOOP, 0.0);
        self.timer2 = 0x80;
        self.unk_1f0 = 0x78;
        self.unk_1a0 = 60.0;
        self.unk_1a8 = 0.0;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::BariPhase3Attack;
    }

    /// A Bari's AT on Link (its tether or itself): knocked back from the body.
    fn bari_knock_back(&mut self, play: &mut PlayState) {
        if self.collider_lightning.base.at_flags & AT_HIT != 0 || self.collider_jnt_sph.base.at_flags & AT_HIT != 0 {
            let player = play.player;
            if player.is_some() && (self.collider_lightning.base.at == player || self.collider_jnt_sph.base.at == player) {
                let yaw = self.body(play).map_or(0, |b| b.actor.yaw_towards_player);
                actor_set_player_knockback_large_no_damage(play, 8.0, yaw, 8.0);
                if let Some(p) = player {
                    audio_play_actor_sfx_at(play, p, NA_SE_PL_BODY_HIT);
                }
                self.collider_jnt_sph.base.at = None;
                self.collider_lightning.base.at = None;
            }
            self.collider_lightning.base.at_flags &= !AT_HIT;
            self.collider_jnt_sph.base.at_flags &= !AT_HIT;
        }
    }

    /// The unused wobble (`vaBariUnused`): a new target each 128 frames.
    fn bari_wobble(&mut self, play: &mut PlayState) {
        if play.gameplay_frames % 128 == 0 {
            self.head_rot[0] = (play.rand.zero_one() * 100.0) as i16 + 100;
        }
        let x = self.head_rot[0];
        smooth_step_to_s(&mut self.head_rot[2], x, 1, 0x1E, 0);
        self.head_rot[1] = self.head_rot[1].wrapping_add(self.head_rot[2]);
    }

    /// `BossVa_BariPhase3Attack`: circling the body on its tether (160 out), hittable only while
    /// its tether's up (`timer2 & 0x1FF` from 128; the boomerang thrown back); stunned with him.
    fn bari_phase3_attack(&mut self, play: &mut PlayState) {
        let sp54 = self.body(play).map_or(Vec3::ZERO, |b| b.unk_1d8);
        self.unk_1a4 += play.rand.zero_one() * 0.5;
        let sp52 = self.timer2 & 0x1FF;
        self.bari_wobble(play);
        self.bari_knock_back(play);
        if self.collider_jnt_sph.base.ac_flags & AC_HIT != 0 {
            self.collider_jnt_sph.base.ac_flags &= !AC_HIT;
            if actor_id(play, self.collider_jnt_sph.base.ac) == Some(ACTOR_EN_BOOM) && sp52 >= 128 {
                send_boomerang_back(play, self.collider_jnt_sph.base.ac);
                audio_play_actor_sfx2(play, NA_SE_IT_SHIELD_REFLECT_SW);
            }
        }
        self.actor.world_pos.x = (sin_s(self.actor.world_rot.y) * self.unk_1a0) + sp54.x;
        self.actor.world_pos.z = (cos_s(self.actor.world_rot.y) * self.unk_1a0) + sp54.z;
        smooth_step_to_f(&mut self.actor.world_pos.y, 4.0, 1.0, 2.0, 0.0);
        self.actor.world_pos.y += 2.0 * math_sin_f(self.unk_1a4);
        self.actor.world_rot.x = vec3f_pitch(sp54, self.actor.world_pos);
        smooth_step_to_f(&mut self.unk_1a0, 160.0, 1.0, 2.0, 0.0);
        smooth_step_to_s(&mut self.actor.shape_rot.x, 0, 1, 0x5DC, 0);
        self.unk_1ac = if self.timer2 & 0x200 == 0 { 0xBB8 } else { -0xBB8 };
        if sp52 >= 128 {
            self.spark(play, 1, 75, 15.0, 7.0, SPARK_TETHER, 1.0, true);
            play.collision_check_set_ac(&self.actor, 1, &mut self.collider_jnt_sph);
            st(play).phase3_stop_moving = false;
        } else {
            st(play).phase3_stop_moving = true;
        }
        play.collision_check_set_at(&self.actor, 2, &mut self.collider_lightning);
        play.collision_check_set_at(&self.actor, 1, &mut self.collider_jnt_sph);
        if play.gameplay_frames % 4 == 0 {
            smooth_step_to_s(&mut self.unk_1f0, 0x78, 1, 0xA, 0);
        }
        Self::maybe_bl_spark(play);
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(self.unk_1ac);
        if st(play).body_state & 0x7F != 0 {
            self.setup_bari_phase3_stunned(play);
        }
    }

    /// `BossVa_SetupBariPhase2Attack`.
    fn setup_bari_phase2_attack(&mut self) {
        self.change(self.anims.bari.clone(), 1.0, 0.0, last(&self.anims.bari), ANIMMODE_LOOP, 0.0);
        self.timer2 = 0x40;
        self.unk_1f0 = 0x78;
        self.unk_1a0 = 60.0;
        self.unk_1a8 = 0.0;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::BariPhase2Attack;
    }

    /// `BossVa_BariPhase2Attack`: circling the body (`sPhase2Timer`'s swing in and out, the odd
    /// ones the other way), the tether up; while he's stunned (blue) it stops, hittable: a hit or
    /// `sKillBari` kills it.
    fn bari_phase2_attack(&mut self, play: &mut PlayState) {
        let (sp54, body_cf) = self.body(play).map_or((Vec3::ZERO, 0), |b| (b.unk_1d8, b.actor.color_filter_timer));
        self.unk_1a4 += play.rand.zero_one() * 0.5;
        let sp52 = self.timer2 & 0x1FF;
        if play.gameplay_frames % 128 == 0 {
            self.head_rot[0] = (play.rand.zero_one() * 100.0) as i16 + 100;
        }
        let sp50 = (st(play).fight_phase as i16 * 70) - 280;
        let x = self.head_rot[0];
        smooth_step_to_s(&mut self.head_rot[2], x, 1, 0x1E, 0);
        self.head_rot[1] = self.head_rot[1].wrapping_add(self.head_rot[2]);
        if st(play).kill_bari != 0 {
            st(play).kill_bari -= 1;
            self.kill_bari(play);
            return;
        }
        self.bari_knock_back(play);
        smooth_step_to_f(&mut self.actor.world_pos.y, 4.0, 1.0, 2.0, 0.0);
        self.actor.world_rot.x = vec3f_pitch(sp54, self.actor.world_pos);
        if play.gameplay_frames % 8 == 0 {
            smooth_step_to_s(&mut self.unk_1f0, 0x28, 1, 0xA, 0);
            self.spark(play, 1, self.unk_1f0, 25.0, 20.0, SPARK_BARI, 2.0, true);
        }
        let t2 = st(play).phase2_timer;
        if t2 & 0x100 == 0 && body_cf == 0 {
            let mut sp4c = 200.0;
            self.spark(play, 1, 125, 15.0, 7.0, SPARK_TETHER, 1.0, true);
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            if self.ty() & 1 != 0 {
                sp4c = -200.0;
            }
            smooth_step_to_f(&mut self.unk_1a0, (sin_s((t2 as i32 * 0x190) as i16) * sp4c) + 320.0, 1.0, 10.0, 0.0);
            smooth_step_to_s(&mut self.unk_1ac, sp50.wrapping_add(0x1F4), 1, 0x3C, 0);
            self.actor.world_pos.y += 2.0 * math_sin_f(self.unk_1a4);
            if self.collider_jnt_sph.base.ac_flags & AC_HIT != 0 {
                self.collider_jnt_sph.base.ac_flags &= !AC_HIT;
                if actor_id(play, self.collider_jnt_sph.base.ac) == Some(ACTOR_EN_BOOM) && sp52 >= 64 {
                    send_boomerang_back(play, self.collider_jnt_sph.base.ac);
                    audio_play_actor_sfx2(play, NA_SE_IT_SHIELD_REFLECT_SW);
                }
            }
            play.collision_check_set_at(&self.actor, 2, &mut self.collider_lightning);
            play.collision_check_set_at(&self.actor, 1, &mut self.collider_jnt_sph);
        } else {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            smooth_step_to_s(&mut self.unk_1ac, sp50.wrapping_add(150), 1, 0x3C, 0);
            if body_cf == 0 {
                smooth_step_to_f(&mut self.unk_1a0, 180.0, 1.0, 1.5, 0.0);
            } else {
                self.unk_1ac = 0;
                if self.actor.color_filter_timer == 0 {
                    self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_XLU, body_cf as u16);
                }
            }
            self.actor.world_pos.y += math_sin_f(self.unk_1a4) * 4.0;
            if self.collider_jnt_sph.base.ac_flags & AC_HIT != 0 {
                self.kill_bari(play);
            }
        }
        smooth_step_to_s(&mut self.actor.shape_rot.x, 0, 1, 0x5DC, 0);
        play.collision_check_set_ac(&self.actor, 1, &mut self.collider_jnt_sph);
        if play.gameplay_frames % 4 == 0 {
            smooth_step_to_s(&mut self.unk_1f0, 0x78, 1, 0xA, 0);
        }
        Self::maybe_bl_spark(play);
        if body_cf == 0 {
            if self.timer2 & 0x400 == 0 {
                self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(self.unk_1ac);
            } else {
                self.actor.world_rot.y = self.actor.world_rot.y.wrapping_sub(self.unk_1ac);
            }
            self.actor.world_pos.x = (sin_s(self.actor.world_rot.y) * self.unk_1a0) + sp54.x;
            self.actor.world_pos.z = (cos_s(self.actor.world_rot.y) * self.unk_1a0) + sp54.z;
        }
    }

    /// `BossVa_SetupBariPhase3Stunned`: blue for the body's `timer`.
    fn setup_bari_phase3_stunned(&mut self, play: &mut PlayState) {
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        self.timer = self.body(play).map_or(0, |b| b.timer);
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_XLU, self.timer as u16);
        self.action = Action::BariPhase3Stunned;
    }

    /// `BossVa_BariPhase3Stunned`: a hit kills it; past its timer the tether's sparks, and from
    /// `timer2` 16 the attack again.
    fn bari_phase3_stunned(&mut self, play: &mut PlayState) {
        let (bpos, sp40) = self.body(play).map_or((Vec3::ZERO, Vec3::ZERO), |b| (b.actor.world_pos, b.unk_1d8));
        self.actor.world_rot.x = vec3f_pitch(bpos, self.actor.world_pos);
        if self.collider_jnt_sph.base.ac_flags & AC_HIT != 0 {
            self.kill_bari(play);
            return;
        }
        self.unk_1a4 += play.rand.zero_one() * 0.5;
        smooth_step_to_f(&mut self.actor.world_pos.y, 4.0, 1.0, 2.0, 0.0);
        self.actor.world_pos.y += math_sin_f(self.unk_1a4) * 3.0;
        play.collision_check_set_ac(&self.actor, 1, &mut self.collider_jnt_sph);
        if play.gameplay_frames % 4 == 0 {
            smooth_step_to_s(&mut self.unk_1f0, 0x28, 1, 0xA, 0);
            self.spark(play, 1, self.unk_1f0, 25.0, 20.0, SPARK_BARI, 2.0, true);
        }
        self.timer -= 1;
        self.actor.world_rot.x = vec3f_pitch(sp40, self.actor.world_pos);
        if self.timer <= 0 {
            if self.timer == 0 {
                self.timer2 = 0;
            } else {
                self.spark(play, 1, 85, 15.0, 0.0, SPARK_TETHER, 1.0, true);
                if self.timer2 >= 0x10 {
                    self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                    self.timer2 = 0x80;
                    self.action = Action::BariPhase3Attack;
                }
            }
        }
    }

    /// `BossVa_SetupBariDeath`.
    fn setup_bari_death(&mut self, play: &mut PlayState) {
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.timer = 30;
        audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_BL_DEAD);
        self.is_dead += 1;
        self.action = Action::BariDeath;
    }

    /// `BossVa_BariDeath`.
    fn bari_death(&mut self) {
        self.timer -= 1;
        if self.timer == 0 {
            self.actor.kill();
        }
    }

    // ------------------------------------------------------------------------------------------
    // The draws' points (`draw_update`)
    // ------------------------------------------------------------------------------------------

    /// `BossVa_BodyOverrideLimbDraw`'s matrix and rotation changes.
    fn body_override(&self, limb: usize, m: &mut MtxF, rot: &mut [i16; 3]) {
        if limb == 20 {
            m.rotate_x(-std::f32::consts::FRAC_PI_2);
        } else if (10..20).contains(&limb) {
            rot[0] = rot[0].wrapping_sub(0x4000);
        } else if limb == 6 {
            m.scale(self.unk_1a4, self.unk_1a4, self.unk_1a4);
        } else if limb == 61 {
            m.scale(self.unk_1a0, self.unk_1a0, self.unk_1a0);
        } else if limb == 7 {
            rot[0] = rot[0].wrapping_sub(0xCCC);
        }
    }

    /// `BossVa_ZapperOverrideLimbDraw`.
    fn zapper_override(&self, limb: usize, m: &mut MtxF, pos: &mut Vec3, rot: &mut [i16; 3]) {
        match limb {
            4 => {
                rot[1] = rot[1].wrapping_add(self.unk_1e6);
                rot[2] = rot[2].wrapping_add(self.unk_1e4);
            }
            5 => {
                rot[1] = rot[1].wrapping_add(self.unk_1ec);
                rot[2] = rot[2].wrapping_add(self.unk_1ea);
            }
            7 => {
                m.translate(pos.x, pos.y, pos.z);
                // sZapperRot: the head's turn so far, undone.
                let zr = m.to_zyx_rot_s(false);
                m.rotate_x(binang_to_rad(zr[0].wrapping_neg()));
                m.rotate_y(binang_to_rad(zr[1].wrapping_neg()));
                m.rotate_z(binang_to_rad(zr[2].wrapping_neg()));
                m.rotate_y(binang_to_rad(self.unk_1f2));
                m.rotate_z(binang_to_rad(self.unk_1f0));
                *pos = Vec3::ZERO;
                *rot = [0; 3];
            }
            _ => {}
        }
    }

    /// `BossVa_BariOverrideLimbDraw`'s scales (limb 2's list is the bake's).
    fn bari_override(&self, limb: usize, m: &mut MtxF) {
        match limb {
            3 => m.scale(self.unk_1d8.x, 1.0, self.unk_1d8.x),
            4 => m.scale(1.0, self.unk_1d8.y, 1.0),
            _ => {}
        }
    }

    /// The body's draw's points: `BossVa_BodyPostLimbDraw` (`unk_1D8`, the attached Bari's
    /// `effectPos`, `unk_274`/`unk_280`; at random once he's dying).
    fn body_draw_update(&mut self, play: &mut PlayState) {
        let Some(skeleton) = self.skeleton.clone() else { return };
        let model = model_of(&self.actor);
        let joints = self.skel.joint_table.clone();
        let (phase, cs, bari) = {
            let s = st(play);
            (s.fight_phase, s.cs_state, s.body_bari)
        };
        let mut points: Vec<(usize, MtxF)> = Vec::new();
        walk(&skeleton, &joints, &model, |limb, m, _, rot| self.body_override(limb, m, rot), |limb, m| points.push((limb, *m)));
        for (limb, m) in points {
            if limb == 6 {
                let x = if phase < PHASE_3 { -1000.0 } else { 200.0 };
                self.unk_1d8 = m.mult_vec3f(Vec3::new(x, 0.0, 0.0));
            } else if (10..20).contains(&limb) && bari[limb - 10] != 0 {
                let mut sp78 = Vec3::ZERO;
                if cs >= DEATH_START {
                    sp78.x = play.rand.centered_float(530.0);
                    sp78.y = play.rand.centered_float(530.0);
                    sp78.z = -60.0;
                }
                self.effect_pos[limb - 10] = m.mult_vec3f(sp78);
            } else if limb == 24 && cs < DEATH_START {
                self.effect_pos[11] = m.mult_vec3f(Vec3::new((self.actor.shape_y_offset + 450.0) + -140.0, 0.0, 0.0));
                self.effect_pos[10] = m.mult_vec3f(Vec3::new(200.0, 0.0, 0.0));
            }
            if limb == 7 && cs >= DEATH_START {
                let mut sp78 = Vec3::new(play.rand.centered_float(320.0) + -250.0, play.rand.centered_float(320.0), play.rand.centered_float(320.0));
                sp78.y += if sp78.y < 0.0 { -150.0 } else { 150.0 };
                sp78.z += if sp78.z < 0.0 { -150.0 } else { 150.0 };
                self.effect_pos[10] = m.mult_vec3f(sp78);
            }
        }
    }

    /// `BossVa_SupportPostLimbDraw`'s points.
    fn support_draw_update(&mut self) {
        let Some(skeleton) = self.skeleton.clone() else { return };
        let model = model_of(&self.actor);
        let joints = self.skel.joint_table.clone();
        let mut points: Vec<(usize, MtxF)> = Vec::new();
        let (on, hx) = (self.on_ceiling != 0, self.head_rot[0]);
        walk(
            &skeleton,
            &joints,
            &model,
            |limb, _, _, rot| {
                if !on && limb == 4 {
                    rot[2] = rot[2].wrapping_add(hx);
                }
            },
            |limb, m| points.push((limb, *m)),
        );
        let t = ((self.timer & 0x1F) >> 1) as f32;
        let t2 = (self.timer2 & 7) as f32;
        for (limb, m) in points {
            if on {
                match limb {
                    4 => {
                        self.actor.focus_pos = m.mult_vec3f(Vec3::ZERO);
                        self.collider_jnt_sph.update_spheres(0, &m.to_mat4());
                    }
                    7 => {
                        self.arm_tip = m.mult_vec3f(Vec3::ZERO);
                        self.effect_pos[0] = m.mult_vec3f(Vec3::new(t * -40.0, t * -7.0, 0.0));
                    }
                    9 => self.effect_pos[1] = m.mult_vec3f(Vec3::new(t * -60.0, t * -45.0, 0.0)),
                    _ => {}
                }
            } else {
                match limb {
                    5 => self.arm_tip = m.mult_vec3f(Vec3::ZERO),
                    8 => self.effect_pos[2] = m.mult_vec3f(Vec3::new(t2 * 90.0, 0.0, 0.0)),
                    9 => self.effect_pos[1] = m.mult_vec3f(Vec3::new(t2 * 50.0, 0.0, 0.0)),
                    10 => self.effect_pos[0] = m.mult_vec3f(Vec3::new(t2 * 46.0, 0.0, 0.0)),
                    _ => {}
                }
            }
        }
    }

    /// `BossVa_ZapperPostLimbDraw`'s points, its head's turn to its target (`headRot`) and the
    /// lightning's quad.
    fn zapper_draw_update(&mut self, play: &PlayState) {
        let Some(skeleton) = self.skeleton.clone() else { return };
        let model = model_of(&self.actor);
        let joints = self.skel.joint_table.clone();
        let mut points: Vec<(usize, MtxF)> = Vec::new();
        walk(&skeleton, &joints, &model, |limb, m, pos, rot| self.zapper_override(limb, m, pos, rot), |limb, m| points.push((limb, *m)));
        let t2 = self.timer2;
        let phase4 = statics(play).is_some_and(|s| s.fight_phase >= PHASE_4);
        for (limb, m) in points {
            match limb {
                3 => self.effect_pos[0] = m.mult_vec3f(Vec3::new((t2 & 7) as f32 * 30.0, 0.0, 0.0)),
                4 => {
                    self.arm_tip = m.mult_vec3f(Vec3::ZERO);
                    self.effect_pos[1] = m.mult_vec3f(Vec3::new((t2 & 7) as f32 * 30.0, 0.0, 0.0));
                }
                5 => {
                    self.zap_neck_pos = m.mult_vec3f(Vec3::ZERO);
                    self.effect_pos[2] = m.mult_vec3f(Vec3::new((t2 & 7) as f32 * 46.0, 0.0, 0.0));
                }
                7 => {
                    self.zap_head_pos = m.mult_vec3f(Vec3::ZERO);
                    self.effect_pos[3] = m.mult_vec3f(Vec3::new((t2 & 7) as f32 * 46.0, 0.0, 0.0));
                    self.effect_pos[9] = m.mult_vec3f(Vec3::new(20.0, 0.0, 0.0));
                    let r = crate::en_ba::func_80035844(self.effect_pos[9], self.unk_1d8, false);
                    self.head_rot[0] = r[0];
                    self.head_rot[1] = r[1];
                    let e9 = self.effect_pos[9];
                    let mut n = MtxF::set_translate(e9.x, e9.y, e9.z);
                    n.rotate_zyx(r[0], r[1], 0);
                    let mut sp70 = Vec3::ZERO;
                    sp70.z = if phase4 { (t2.wrapping_sub(16) & 7) as f32 * 120.0 } else { (t2.wrapping_sub(32) & 0xF) as f32 * 80.0 };
                    sp70.z += 40.0;
                    let far = sp70.z;
                    sp70.z += 50.0;
                    self.effect_pos[4] = n.mult_vec3f(sp70);
                    let back = if phase4 { 33.0 } else { 22.0 };
                    sp70.z = (sp70.z - back).max(0.0);
                    self.effect_pos[6] = n.mult_vec3f(sp70);
                    sp70.z = (sp70.z - back).max(0.0);
                    self.effect_pos[5] = n.mult_vec3f(sp70);
                    let q1 = n.mult_vec3f(Vec3::new(15.0, 0.0, 0.0));
                    let q0 = n.mult_vec3f(Vec3::new(-15.0, 0.0, 0.0));
                    let q3 = n.mult_vec3f(Vec3::new(15.0, 0.0, far));
                    let q2 = n.mult_vec3f(Vec3::new(-15.0, 0.0, far));
                    self.collider_lightning.set_vertices(q0, q1, q2, q3);
                }
                _ => {}
            }
        }
    }

    /// `BossVa_Draw`'s Bari points: its sphere at its model's origin, `effectPos[1]` there, the
    /// tether from the body (its place in the intro, `unk_1D8` after) to it: `effectPos[0]` along
    /// it by `timer2`, and the lightning's quad.
    fn bari_draw_update(&mut self, play: &PlayState) {
        let model = model_of(&self.actor);
        self.collider_jnt_sph.update_spheres(0, &model.to_mat4());
        let in_intro = statics(play).is_none_or(|s| s.cs_state < BOSSVA_BATTLE);
        let spbc = self.body(play).map_or(Vec3::ZERO, |b| if in_intro { b.actor.world_pos } else { b.unk_1d8 });
        self.effect_pos[1] = model.mult_vec3f(Vec3::ZERO);
        let mut m = MtxF::set_translate(spbc.x, spbc.y, spbc.z);
        m.rotate_zyx(self.actor.world_rot.x, self.actor.world_rot.y, 0);
        let z = (self.timer2 & 0xF) as f32 * (self.unk_1a0 * 0.0625);
        self.effect_pos[0] = m.mult_vec3f(Vec3::new(0.0, 45.0, z));
        let q1 = m.mult_vec3f(Vec3::new(15.0, 40.0, 0.0));
        let q0 = m.mult_vec3f(Vec3::new(-15.0, 40.0, 0.0));
        let q3 = m.mult_vec3f(Vec3::new(15.0, 40.0, self.unk_1a0));
        let q2 = m.mult_vec3f(Vec3::new(-15.0, 40.0, self.unk_1a0));
        self.collider_lightning.set_vertices(q0, q1, q2, q3);
    }
}

/// `BossVa_BodyDeath`'s choice of `sWarpPos` for Ruto (`:1674-1679`): from 2 down, each that's
/// nearer Link than the one before it makes that one before it the choice.
pub fn warp_index(player: Vec3) -> usize {
    let mut sp7c = 2;
    for i in (1..=2).rev() {
        if S_WARP_POS[i].distance(player) < S_WARP_POS[i - 1].distance(player) {
            sp7c = i - 1;
        }
    }
    sp7c
}

// ----------------------------------------------------------------------------------------------
// Bakes
// ----------------------------------------------------------------------------------------------

pub const BAKE_BODY_A: &str = "Boss_Va/body_a";
pub const BAKE_BODY_B: &str = "Boss_Va/body_b";
pub const BAKE_BODY_XLU: &str = "Boss_Va/body_xlu";
pub const BAKE_BODY_BARI: &str = "Boss_Va/body_bari";
pub const BAKE_BODY_TUBE: &str = "Boss_Va/body_tube";
pub const BAKE_SUPPORT: &str = "Boss_Va/support";
pub const BAKE_CUT_SUPPORT: &str = "Boss_Va/cut_support";
pub const BAKE_ZAPPER: &str = "Boss_Va/zapper";
pub const BAKE_STUMP: &str = "Boss_Va/stump";
pub const BAKE_BARI: &str = "Boss_Va/bari";
pub const BAKE_BARI_GLOW: &str = "Boss_Va/bari_glow";
/// `BossVa_DrawDoor`'s piece `i`.
pub fn door_bake(i: usize) -> String {
    format!("Boss_Va/door{i}")
}

/// The body's limbs with a list (1-based, `gBarinadeBodySkel`'s).
pub const BODY_DL_LIMBS: [usize; 16] = [7, 20, 24, 25, 26, 29, 31, 35, 37, 41, 43, 47, 49, 53, 55, 62];
/// Those its post-limb draw draws again in the XLU list (29 to 55).
pub const BODY_XLU_LIMBS: [usize; 10] = [29, 31, 35, 37, 41, 43, 47, 49, 53, 55];
/// `doorPieceDispList`.
pub const DOOR_PIECES: [&str; 8] = [
    "gBarinadeDoorPiece1DL",
    "gBarinadeDoorPiece2DL",
    "gBarinadeDoorPiece3DL",
    "gBarinadeDoorPiece4DL",
    "gBarinadeDoorPiece5DL",
    "gBarinadeDoorPiece6DL",
    "gBarinadeDoorPiece7DL",
    "gBarinadeDoorPiece8DL",
];
/// `doorPieceLength`.
pub const DOOR_PIECE_LENGTH: [i16; 8] = [836, 900, 836, 1016, 800, 1016, 836, 900];

const SEG_8: u8 = 0x08;
const SEG_9: u8 = 0x09;
const SEG_A: u8 = 0x0A;
/// The body's glow (`gDPSetEnvColor(0, 0, 0, bodyGlow)` before limb 20) on its own segment.
const SEG_GLOW: u8 = 0x0E;

/// `(gameplayFrames * k) % m` with the C's `u32` arithmetic.
fn fm(frames: u32, k: i32, m: u32) -> u32 {
    frames.wrapping_mul(k as u32) % m
}

/// The body's segment 8 at the draw's start: `Gfx_TwoTexScroll(0, 0, 0, 8, 16, 1, 0, (f × -10) % 16, 16, 16)`.
pub fn body_scroll_8a(f: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, 0, 8, 16, 1, 0, fm(f, -10, 16), 16, 16)
}
/// Limb 20's: `(0, 0, 0, 8, 16, 1, 0, (f × -2) % 64, 16, 16)`.
pub fn body_scroll_8b(f: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, 0, 8, 16, 1, 0, fm(f, -2, 64), 16, 16)
}
/// Segment 9 at the draw's start: `(0, 0, (f × -10) % 32, 16, 32, 1, 0, (f × -5) % 32, 16, 32)`.
pub fn body_scroll_9(f: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, fm(f, -10, 32), 16, 32, 1, 0, fm(f, -5, 32), 16, 32)
}
/// Limb 25's XLU segment 9: `(0, 0, (f × 10) % 128, 16, 32, 1, 0, (f × 5) % 128, 16, 32)`.
pub fn body_scroll_9_xlu(f: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, fm(f, 10, 128), 16, 32, 1, 0, fm(f, 5, 128), 16, 32)
}
/// A Bari's segment 0x0A: `(0, 0, (f × 10) % 32, 16, 32, 1, 0, (f × -5) % 32, 16, 32)`.
pub fn bari_scroll(f: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, fm(f, 10, 32), 16, 32, 1, 0, fm(f, -5, 32), 16, 32)
}

fn skel_bake(name: &str, symbol: &str, segments: Vec<(u8, BakeSegment)>, prelude: Vec<u8>, nulled: &[usize]) -> MeshBake {
    MeshBake {
        name: name.into(),
        object: OBJECT.into(),
        segments,
        prelude,
        body: BakeBody::Skeleton {
            file: OBJECT.into(),
            symbol: symbol.into(),
            limbs: nulled.iter().map(|&l| LimbOverride { limb: (l - 1) as u8, file: OBJECT.into(), symbol: String::new() }).collect(),
        },
    }
}

fn dl_bake(name: &str, segments: Vec<(u8, BakeSegment)>, dl: &str) -> MeshBake {
    MeshBake { name: name.into(), object: OBJECT.into(), segments, prelude: Vec::new(), body: BakeBody::DLists(vec![(OBJECT.into(), dl.into())]) }
}

/// Every mesh `BossVa_Draw` draws (after `Gfx_SetupDL_25Opa`/`25Xlu`), and the particles'.
pub fn bakes() -> Vec<MeshBake> {
    let not = |keep: &[usize]| -> Vec<usize> { BODY_DL_LIMBS.iter().copied().filter(|l| !keep.contains(l)).collect() };
    let seg = |s: u8, d: Vec<(u32, u32)>| (s, BakeSegment::Dynamic(d));
    let mut v = vec![
        skel_bake(BAKE_BODY_A, "gBarinadeBodySkel", vec![seg(SEG_8, body_scroll_8a(0)), seg(SEG_9, body_scroll_9(0))], vec![], &not(&[7])),
        skel_bake(
            BAKE_BODY_B,
            "gBarinadeBodySkel",
            vec![seg(SEG_8, body_scroll_8b(0)), seg(SEG_9, body_scroll_9(0)), (SEG_GLOW, BakeSegment::DynamicColor { env: true, prim: false })],
            vec![SEG_GLOW],
            &[7],
        ),
        skel_bake(BAKE_BODY_XLU, "gBarinadeBodySkel", vec![seg(SEG_8, body_scroll_8b(0)), seg(SEG_9, body_scroll_9_xlu(0))], vec![], &not(&BODY_XLU_LIMBS)),
        dl_bake(BAKE_BODY_BARI, vec![], "gBarinadeDL_008BB8"),
        dl_bake(BAKE_BODY_TUBE, vec![seg(SEG_9, body_scroll_9_xlu(0))], "gBarinadeDL_008D70"),
        skel_bake(BAKE_SUPPORT, "gBarinadeSupportSkel", vec![], vec![], &[]),
        skel_bake(BAKE_CUT_SUPPORT, "gBarinadeCutSupportSkel", vec![], vec![], &[]),
        skel_bake(BAKE_ZAPPER, "gBarinadeZapperSkel", vec![], vec![], &[]),
        skel_bake(BAKE_STUMP, "gBarinadeStumpSkel", vec![], vec![], &[]),
        skel_bake(BAKE_BARI, "gBarinadeBariSkel", vec![], vec![], &[2]),
        dl_bake(BAKE_BARI_GLOW, vec![seg(SEG_A, bari_scroll(0))], "gBarinadeDL_000FA0"),
    ];
    for (i, dl) in DOOR_PIECES.iter().enumerate() {
        v.push(dl_bake(&door_bake(i), vec![], dl));
    }
    v.extend(fx::bakes());
    v
}

fn draw_cmd(bake: &str, transform: Mat4, bones: Vec<Mat4>, sv: Option<SegmentValues>) -> DrawCmd {
    DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform, bones, params: DrawParams { segments: sv, ..Default::default() } }
}

/// `BossVa_DrawDoor`: the eight pieces round (0, 80, 400), turned to face south, each pushed out
/// its length (at the door's scale) along its own angle; `scale` (`sDoorState`) 100 shut.
pub fn door_matrices(scale: i16) -> Vec<Mat4> {
    let mut m = MtxF::set_translate(DOOR_POS.x, DOOR_POS.y, DOOR_POS.z);
    m.rotate_y(std::f32::consts::PI);
    let mut y_scale = (scale as f32 * 0.01) * 0.1;
    m.scale(0.1, y_scale, 0.1);
    y_scale = if y_scale != 0.0 { 0.1 / y_scale } else { 0.0 };
    let mut seg_angle = 0.0f32;
    let mut out = Vec::new();
    for len in DOOR_PIECE_LENGTH {
        let mut p = m;
        p.rotate_z(seg_angle);
        p.translate(0.0, len as f32 * y_scale, 0.0);
        out.push(p.to_mat4());
        seg_angle -= std::f32::consts::FRAC_PI_4;
    }
    out
}

impl ActorImpl for BossVa {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BossVa_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::None => {}
            Action::BodyIntro => self.body_intro(play),
            Action::BodyPhase1 => self.body_phase1(play),
            Action::BodyPhase2 => self.body_phase2(play),
            Action::BodyPhase3 => self.body_phase3(play),
            Action::BodyPhase4 => self.body_phase4(play),
            Action::BodyDeath => self.body_death(play),
            Action::SupportIntro => self.support_intro(play),
            Action::SupportAttached => self.support_attached(play),
            Action::SupportCut => self.support_cut(play),
            Action::ZapperIntro => self.zapper_intro(play),
            Action::ZapperAttack => self.zapper_attack(play),
            Action::ZapperDamaged => self.zapper_damaged(play),
            Action::ZapperHold => self.zapper_hold(play),
            Action::ZapperDeath => self.zapper_death(play),
            Action::ZapperEnraged => self.zapper_enraged(play),
            Action::Stump => self.stump(play),
            Action::Door => self.door(play),
            Action::BariIntro => self.bari_intro(play),
            Action::BariPhase2Attack => self.bari_phase2_attack(play),
            Action::BariPhase3Attack => self.bari_phase3_attack(play),
            Action::BariPhase3Stunned => self.bari_phase3_stunned(play),
            Action::BariDeath => self.bari_death(),
        }
        let p = self.ty();
        if p == BOSSVA_BODY {
            if self.body_collider.base.ac_flags & AC_HIT != 0 {
                self.body_collider.base.ac_flags &= !AC_HIT;
                if actor_id(play, self.body_collider.base.ac) == Some(ACTOR_EN_BOOM)
                    && let Some(b) = self.body_collider.base.ac.and_then(|h| play.actors.downcast_mut::<crate::en_boom::EnBoom>(h))
                {
                    b.return_timer = 0;
                }
            }
            // BossVa_UpdateEffects: each part's state read from it (the body's from itself).
            let me = play.cur_actor;
            let own = self.parent_state(self.unk_1d8);
            let body_1d8 = self.unk_1d8;
            let cs = st(play).cs_state;
            let resolve = move |play: &PlayState, h: ActorHandle| -> Option<ParentState> {
                if Some(h) == me {
                    return Some(own);
                }
                play.actors.downcast::<BossVa>(h).map(|b| b.parent_state(body_1d8))
            };
            with_effects(play, |play, e| fx::update_effects(play, e, cs, &resolve));
            let e = &mut play.env_ctx;
            for i in (0..3).rev() {
                e.adj_ambient_color[i] = (e.adj_ambient_color[i] - 1).max(0);
                e.adj_light1_color[i] = (e.adj_light1_color[i] - 10).max(0);
                e.adj_fog_color[i] = (e.adj_fog_color[i] - 10).max(0);
            }
            if self.on_ceiling > 0 {
                self.on_ceiling -= 1;
            }
        } else if is_bari(p) {
            self.timer2 = self.timer2.wrapping_add(1);
            self.actor.focus_pos = self.actor.world_pos;
            self.actor.focus_pos.y += 45.0;
            let a = (self.timer2 as i32 * 0xFA4) as i16;
            self.unk_1d8.y = (cos_s(a) * 0.24) + 0.76;
            self.unk_1d8.x = (sin_s(a) * 0.2) + 1.0;
        }
    }

    /// `BossVa_Destroy`; then, the last instance gone, the overlay's unload (`Actor_FreeOverlay`):
    /// its statics come back zeroed with its next load.
    fn destroy(&mut self, play: &mut PlayState) {
        let others = play.actors.all().into_iter().any(|h| play.actors.actor(h).is_some_and(|a| a.id == ACTOR_BOSS_VA));
        if !others {
            play.overlay_statics.remove(&ACTOR_BOSS_VA);
        }
    }

    /// `BossVa_Draw`'s effects on the game: the body's fog and far plane taken back up (350 a
    /// frame) and its limbs' points; the parts' points, spheres and quads.
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.actor.killed {
            return;
        }
        let p = self.ty();
        if p == BOSSVA_BODY {
            let e = &mut play.env_ctx;
            if e.adj_fog_near != 0 {
                e.adj_fog_near = e.adj_fog_near.wrapping_add(350);
                if e.adj_fog_near > 0 {
                    e.adj_fog_near = 0;
                }
            }
            if e.adj_fog_far != 0 {
                e.adj_fog_far = e.adj_fog_far.wrapping_add(350);
                if e.adj_fog_far > 0 {
                    e.adj_fog_far = 0;
                }
            }
            if self.is_dead == 0 {
                self.body_draw_update(play);
            }
        } else if is_support(p) {
            if self.is_dead == 0 {
                self.support_draw_update();
            }
        } else if is_zapper(p) {
            if self.is_dead == 0 {
                self.zapper_draw_update(play);
            }
        } else if is_bari(p) && self.is_dead == 0 {
            self.bari_draw_update(play);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        if self.skeleton.is_some() {
            r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        }
        r
    }

    /// `BossVa_Draw`: see the module's docs.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let p = self.ty();
        let frames = play.gameplay_frames;
        let s = statics(play);
        if p == BOSSVA_DOOR {
            let scale = s.map_or(0, |s| s.door_state);
            for (i, m) in door_matrices(scale).into_iter().enumerate() {
                out.opa.push(draw_cmd(&door_bake(i), m, Vec::new(), None));
            }
            return;
        }
        let model_m = actor_draw_matrix(rs);
        let model = MtxF::from_mat4(model_m);
        let joints = rs.joints.as_ref().map(|j| j.rot.clone()).unwrap_or_default();
        if p == BOSSVA_BODY {
            if self.is_dead == 0
                && let Some(sk) = &self.skeleton
            {
                let world = walk(sk, &joints, &model, |limb, m, _, rot| self.body_override(limb, m, rot), |_, _| {});
                let b = bones(&model, &world);
                let mut sv = SegmentValues::default();
                sv.read(SEG_8, &body_scroll_8a(frames));
                sv.read(SEG_9, &body_scroll_9(frames));
                out.opa.push(draw_cmd(BAKE_BODY_A, model_m, b.clone(), Some(sv)));
                let mut sv = SegmentValues::default();
                sv.read(SEG_8, &body_scroll_8b(frames));
                sv.read(SEG_9, &body_scroll_9(frames));
                sv.env[SEG_GLOW as usize] = Some([0, 0, 0, self.body_glow as u8]);
                out.opa.push(draw_cmd(BAKE_BODY_B, model_m, b.clone(), Some(sv)));
                let mut sv = SegmentValues::default();
                sv.read(SEG_8, &body_scroll_8b(frames));
                sv.read(SEG_9, &body_scroll_9_xlu(frames));
                out.xlu.push(draw_cmd(BAKE_BODY_XLU, model_m, b, Some(sv)));
                let (phase, bari) = s.map_or((0, [0; 10]), |s| (s.fight_phase, s.body_bari));
                for limb in 10..20usize {
                    if bari[limb - 10] == 0 {
                        continue;
                    }
                    let drawn = ((limb >= 16 || limb == 10) && phase <= PHASE_3) || (limb >= 11 && phase <= PHASE_2);
                    if drawn {
                        out.xlu.push(draw_cmd(BAKE_BODY_BARI, world[limb - 1].to_mat4(), Vec::new(), None));
                    }
                }
                let mut sv = SegmentValues::default();
                sv.read(SEG_9, &body_scroll_9_xlu(frames));
                out.xlu.push(draw_cmd(BAKE_BODY_TUBE, world[24].to_mat4(), Vec::new(), Some(sv)));
            }
            if let Some(s) = s {
                let eye = play.camera(s.sub_cam_id).map_or(view.eye, |c| c.eye);
                fx::draw_effects(&s.effects, view, eye, out);
            }
            return;
        }
        let Some(sk) = &self.skeleton else { return };
        if is_support(p) {
            if self.is_dead == 0 {
                let (on, hx) = (self.on_ceiling != 0, self.head_rot[0]);
                let world = walk(
                    sk,
                    &joints,
                    &model,
                    |limb, _, _, rot| {
                        if !on && limb == 4 {
                            rot[2] = rot[2].wrapping_add(hx);
                        }
                    },
                    |_, _| {},
                );
                let bake = if self.cut { BAKE_CUT_SUPPORT } else { BAKE_SUPPORT };
                out.opa.push(draw_cmd(bake, model_m, bones(&model, &world), None));
            }
        } else if is_zapper(p) {
            if self.is_dead == 0 {
                let world = walk(sk, &joints, &model, |limb, m, pos, rot| self.zapper_override(limb, m, pos, rot), |_, _| {});
                out.opa.push(draw_cmd(BAKE_ZAPPER, model_m, bones(&model, &world), None));
            }
        } else if is_stump(p) {
            let world = walk(sk, &joints, &model, |_, _, _, _| {}, |_, _| {});
            out.opa.push(draw_cmd(BAKE_STUMP, model_m, bones(&model, &world), None));
        } else if self.is_dead == 0 {
            // A Bari.
            let world = walk(sk, &joints, &model, |limb, m, _, _| self.bari_override(limb, m), |_, _| {});
            let b = bones(&model, &world);
            out.opa.push(draw_cmd(BAKE_BARI, model_m, b.clone(), None));
            let mut sv = SegmentValues::default();
            sv.read(SEG_A, &bari_scroll(frames));
            if let Some(w) = world.get(1) {
                out.xlu.push(draw_cmd(BAKE_BARI_GLOW, w.to_mat4(), Vec::new(), Some(sv)));
            }
            out.xlu.push(draw_cmd(BAKE_BARI, model_m, b, None));
            // not ported: func_80026A6C before the shadow and func_80026860 after it (a stunned
            // Bari's XLU fog: the scene's for the shadow, the blue filter's back).
            // func_80033C30: its circle shadow, 9 over it, 0.4.
            let pos = rs.pos + Vec3::new(0.0, 9.0, 0.0);
            out.opa.push(oot_game::shadow::func_80033c30(&play.col, pos, Vec3::splat(0.4), 0xFF));
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            0 => Some(ColliderMut::Cylinder(&mut self.body_collider)),
            1 => Some(ColliderMut::JntSph(&mut self.collider_jnt_sph)),
            2 => Some(ColliderMut::Quad(&mut self.collider_lightning)),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
