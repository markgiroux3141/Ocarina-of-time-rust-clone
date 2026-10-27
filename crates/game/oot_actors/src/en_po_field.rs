//! `En_Po_Field` (`ovl_En_Po_Field/z_en_po_field.c`): Hyrule Field's Poes and Big Poes, with their
//! souls (GAME-12 milestone 4). The adult's field (`spot00` layers 2 and 3) places ten, params
//! 0x16 to 0x1F: each a place and its switch flag.
//!
//! **One for all ten places** (`EnPoField_Init`): each instance records its place
//! (`sSpawnPositions`, its position as `Vec3s`) and its switch flag (`sSpawnSwitchFlags`) in the
//! overlay's statics (`sNumSpawned` counts them); the first keeps living, the other nine are
//! killed (`params` 0xFF, so their destroy frees nothing). The statics live as long as the
//! overlay: the first instance never dies, so they go with the scene (a new play state's start at
//! 0; [`Statics`] is dropped with the last instance, as `Actor_FreeOverlay` would).
//!
//! **Waiting** (`EnPoField_WaitForSpawn`): invisible (scale 0), not targetable, not in the
//! collision check, for 200 frames from its init (or its last disappearance); then each frame,
//! Link within 150 of a place on x and on z:
//! - its switch flag set: riding (`PLAYER_STATE1_23`), nothing; on foot a **Poe** 300 ahead of
//!   him;
//! - clear: riding, a **Big Poe** 480 ahead (no `Rand`); on foot a Big Poe on a 40% roll
//!   (`Rand_ZeroOne() < 0.4`), else a Poe 300 ahead.
//!
//! It's put on the floor below that point (`BgCheck_EntityRaycastDown4` from 1000 above Link),
//! or waits on if there's none. The C never checks the time of day: Big Poes appear by day too.
//! It doesn't break out of its loop over the places once one has appeared (the places are far
//! apart: no other is ever near).
//!
//! **Appearing** (`EnPoField_Appear`): `NA_SE_EN_PO_APPEAR`; growing over the appear animation's
//! first 10 frames (to 0.01, a Big Poe's 0.014), fading in after; a Big Poe's cylinder 35 by 100
//! up 10 (a Poe's 25 by 50 up 20), its shadow 45 (37), Navi's `NAVI_ENEMY_BIG_POE`
//! (`NAVI_ENEMY_POE_WASTELAND`). A Big Poe flees from the first frame.
//!
//! **A Poe circles Link** (`EnPoField_CirclePlayer`): 600 frames round a point that follows him
//! (at most 100 off), at its distance from him when it began, closing to 180; a flame left behind
//! it (`EnPoField_SpawnFlame`: at its light, 70 frames, sliding 2.5 a frame the way it faced; it
//! burns Link: `HIT_SPECIAL_EFFECT_FIRE`, 4 damage); then it disappears.
//!
//! **A Big Poe flees** (`EnPoField_Flee`): facing Link, moving away at 1.5, 1.25 or 1 times the
//! horse's speed plus 2 (within 300, 400, 500; at least 12), swaying, steered back towards the
//! field's middle (-1000, 0, 6500) past 3000 from it; 2000 frames, or over 1500 from Link, and it
//! disappears. Both float 13 above Link's height or the floor's (`EnPoField_CorrectYPos`); off any
//! floor they disappear.
//!
//! **Hit** (`EnPoField_TestForDamage`, `sDamageTable`: 4 health; the Master Sword 2, an arrow 2,
//! the Giant's Knife 4): knocked back at 5 (an arrow's way, else away from what hit it), red 16
//! frames, unhittable until its damage animation ends; then a Big Poe flees again, a Poe circles.
//! At 0 health (`Enemy_StartFinishingBlow`) it **burns** (`EnPoField_Death`: three columns of blue
//! flames for 7 frames, the body its burning shape, rising and shrinking from frame 19), and at
//! frame 28 only its **lantern** is left (`EnPoField_SetupSoulIdle`: `ACTORCAT_MISC`, falling):
//! on the floor it breaks (`EffectSsHahen_SpawnBurst`, 15 pieces; `NA_SE_EV_METAL_BOX_BOUND`, and
//! for a Big Poe `NA_SE_SY_TRE_BOX_APPEAR`) and the **soul** rises (`func_80AD587C`); falling for
//! 60 frames without a floor, it waits again.
//!
//! **The soul** (`func_80AD58D4`): 400 frames bobbing; Link touching it (`OC1_HIT` of its small
//! cylinder, 13 by 30) talks to it at once (`ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED`): 0x5005
//! (`EnPoField_SoulInteract`, crying, a choice): yes with an empty bottle, `NA_SE_EN_PO_BIG_GET`
//! and a Poe's 0x5008 (`ITEM_BOTTLE_POE`) or a Big Poe's 0x508F (`ITEM_BOTTLE_BIG_POE`, and the
//! place's switch flag set: its Big Poe is done); yes without one, 0x5006; no, 0x5007 (both
//! laughing). It fades as the answer shows (`EnPoField_SoulDisappear`) and waits again. Its 400
//! frames out untouched, it laughs and fades.
//!
//! **Draws** (`EnPoField_Draw`, `EnPoField_DrawSoul`, `EnPoField_DrawFlame`): the body opaque at
//! alpha 255 (or 0, drawing nothing), else translucent (`gActorSetupXluDL`), tinted by
//! `lightColor` (`func_80AD619C`: towards (80, 255, 225) as it flees, flashing while hurt, else
//! towards (255, 255, 210)); a Big Poe's face, cloak and body its own (limbs 1, 8, 9); the lantern
//! (limb 7's place, `sLimb7Mtx`) in `soulColor` (`func_80AD6330`: brightening over the appear
//! animation's first 12 frames, else flickering by `Rand_ZeroOne` every frame), with the light at
//! it (`Lights_PointGlowSetInfo`, 1400 up the limb); the falling lantern; the soul a camera-facing
//! scrolling flame (`gPoeFieldSoulDL`, a Big Poe's texture `gBigPoeSoulTex`); the flame
//! `gEffFire1DL`. Bakes (docs/adr/0012-actor-bakes.md): the bodies opaque and translucent (limb 7
//! nulled), the lantern, the two souls and the flame.
//!
//! The whole overlay is ported. Not ported (the framework's, as `En_Poh`'s): the light's glow
//! and the point lights' shading (BACKLOG #80); the lantern's fragments draw nothing if its list
//! is only a limb's (BACKLOG #81).
//!
//! @bug (game): `EnPoField_SetupFlee` sets `actionFunc` to `EnPoField_Flee` before it checks
//! whether it was `EnPoField_Damage`, so the check always passes: a hurt Big Poe's flight starts
//! over (2000 frames, turned from its facing) as a new one's does.
//!
//! @bug (game): `EnPoField_SetupSoulIdle` takes its shadow away (`shadowDraw` NULL) and nothing
//! gives it back: after the first Poe of a visit dies, the next ones have no shadow.
//!
//! @bug (game): `EnPoField_SetupWaitForSpawn` doesn't put back `actor.draw`: after a soul, the
//! waiting actor keeps `EnPoField_DrawSoul` (scale 0: nothing shows) until the next appears.
//!
//! Params: `PARAMS_GET_U(params, 0, 8)` the place's switch flag (0x16 to 0x1F placed), read at
//! init into `sSpawnSwitchFlags`. From then on the C reuses `params` as the Poe's size
//! (`EnPoFieldSize`: 0 `EN_PO_FIELD_SMALL`, 1 `EN_PO_FIELD_BIG`, set as one appears), and 0xFF for
//! the nine killed at init; that isn't a placement's field.
//! Flags: the places' switch flags (`sSpawnSwitchFlags`, from each placement's `switchFlag`): read
//! as Link nears one (set: no Big Poe there), set as a Big Poe's soul goes in a bottle.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::BGCHECK_Y_MIN;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{approach_f, approach_s, binang_to_rad, cos_s, sin_s, smooth_step_to_s, step_to_f, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_MISC, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2, cur_sfx_pos, enemy_start_finishing_blow};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, TextLine};
use oot_game::audio::sfx::{NA_SE_EN_EXTINCT, NA_SE_SY_TRE_BOX_APPEAR, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::gbi::Dl;
use oot_game::item::{ITEM_BOTTLE_BIG_POE, ITEM_BOTTLE_POE};
use oot_game::lights::{LightInfo, LightNode};
use oot_game::message::TEXT_STATE_CHOICE;
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};

// The Poe's sounds and its soul's texts are En_Poh's (`enemybank_table.h`,
// `environmentbank_table.h`; 0x5005 to 0x5008).
pub use crate::en_poh::{
    NA_SE_EN_PO_APPEAR, NA_SE_EN_PO_AWAY, NA_SE_EN_PO_BIG_CRY, NA_SE_EN_PO_BIG_GET, NA_SE_EN_PO_DAMAGE, NA_SE_EN_PO_DEAD, NA_SE_EN_PO_DEAD2, NA_SE_EN_PO_DISAPPEAR, NA_SE_EN_PO_FLY, NA_SE_EN_PO_LAUGH, NA_SE_EV_METAL_BOX_BOUND, TEXT_POE_BOTTLED, TEXT_POE_NO,
    TEXT_POE_NO_BOTTLE, TEXT_POE_SOUL,
};

/// `ACTOR_EN_PO_FIELD` (`actor_table.h`: 0x0175).
pub const ACTOR_EN_PO_FIELD: i16 = 0x0175;

/// `OBJECT_PO_FIELD` (`object_table.h`: 0x006D).
pub const OBJECT_PO_FIELD: i16 = 0x006D;
pub const PO_FIELD: &str = "object_po_field";

/// `PARAMS_GET_U(params, 0, 8)`: the place's switch flag (`sSpawnSwitchFlags`).
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 8, Kind::SwitchFlag, "the place's switch flag: set, no Big Poe here (a Poe on foot); set as its Big Poe's soul is bottled");

/// The text a Big Poe's soul bottled shows (`message_data.h`).
pub const TEXT_BIG_POE_BOTTLED: u16 = 0x508F;

/// En_Po_Field's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("each place's (sSpawnSwitchFlags): set, no Big Poe there; set as its Big Poe's soul goes in a bottle")],
    todo: false,
    objects: &[],
    texts: &[
        TextLine::new(TEXT_POE_SOUL).cond("Link touching its soul").note("you defeated a Poe! catch it in a bottle? (a choice)"),
        TextLine::new(TEXT_POE_BOTTLED).cond("a Poe's soul, answered Yes, with an empty bottle").note("you caught a Poe!"),
        TextLine::new(TEXT_BIG_POE_BOTTLED).cond("a Big Poe's soul, answered Yes, with an empty bottle").note("you caught a Big Poe! (its place's switch flag set)"),
        TextLine::new(TEXT_POE_NO_BOTTLE).cond("answered Yes, with no empty bottle").note("no empty bottle"),
        TextLine::new(TEXT_POE_NO).cond("answered No").note("you let it go"),
    ],
};

/// `EnPoFieldSize`: what `params` holds once one appears.
pub const EN_PO_FIELD_SMALL: i16 = 0;
pub const EN_PO_FIELD_BIG: i16 = 1;
/// `params` of the nine killed at init.
pub const EN_PO_FIELD_KILLED: i16 = 0xFF;

/// `NAVI_ENEMY_BIG_POE`, `NAVI_ENEMY_POE_WASTELAND` (`actor.h`: 0x5A, 0x5C).
pub const NAVI_ENEMY_BIG_POE: u8 = 0x5A;
pub const NAVI_ENEMY_POE_WASTELAND: u8 = 0x5C;

/// `ACTOR_FLAG_IGNORE_QUAKE` (`actor.h`: 1 << 12).
const ACTOR_FLAG_IGNORE_QUAKE: u32 = 1 << 12;

/// `PLAYER_STATE1_23` (`player.h`): riding.
pub const PLAYER_STATE1_23: u32 = 1 << 23;

/// `FLAGS`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_PO_FIELD,
    name: "En_Po_Field",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | ACTOR_FLAG_IGNORE_QUAKE,
    object: PO_FIELD,
    schema: &SCHEMA,
};

/// `D_80AD7080`: the body, hurt by everything but the shield and the mirror's ray, 25 by 50 up 20.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT3, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 25, height: 50, y_shift: 20, pos: [0, 0, 0] },
};

/// `D_80AD70AC`: the flame, burning (`HIT_SPECIAL_EFFECT_FIRE`, 4), 10 by 30.
pub const FLAME_CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_FIRE, damage: 0x04 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 10, height: 30, y_shift: 0, pos: [0, 0, 0] },
};

/// `D_80AD70D8`: 4 health, 25 by 50, mass 40.
pub const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 4, cyl_radius: 25, cyl_height: 50, mass: 40 };

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x0), // Deku nut
        dmg_entry(2, 0x0), // Deku stick
        dmg_entry(1, 0x0), // Slingshot
        dmg_entry(2, 0x0), // Explosive
        dmg_entry(1, 0x1), // Boomerang
        dmg_entry(2, 0x0), // Normal arrow
        dmg_entry(2, 0x0), // Hammer swing
        dmg_entry(2, 0x1), // Hookshot
        dmg_entry(1, 0x0), // Kokiri sword
        dmg_entry(2, 0x0), // Master sword
        dmg_entry(4, 0x0), // Giant's Knife
        dmg_entry(2, 0x0), // Fire arrow
        dmg_entry(2, 0x0), // Ice arrow
        dmg_entry(2, 0x0), // Light arrow
        dmg_entry(2, 0x0), // Unk arrow 1
        dmg_entry(2, 0x0), // Unk arrow 2
        dmg_entry(2, 0x0), // Unk arrow 3
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
        dmg_entry(4, 0x0), // Hammer jump
        dmg_entry(0, 0x0), // Unknown 2
    ],
};

/// `sFieldMiddle`.
pub const S_FIELD_MIDDLE: Vec3 = Vec3::new(-1000.0, 0.0, 6500.0);
/// `D_80AD7114`, `D_80AD7120`: the death flames' velocity and acceleration.
const D_80AD7114: Vec3 = Vec3::new(0.0, 3.0, 0.0);
const D_80AD7120: Vec3 = Vec3::new(0.0, 0.0, 0.0);
/// `D_80AD714C`: the light's place on limb 7.
const D_80AD714C: Vec3 = Vec3::new(0.0, 1400.0, 0.0);

/// `EnPoFieldInfo`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnPoFieldInfo {
    pub prim_color: [u8; 3],
    pub light_color: [u8; 3],
    pub env_color: [u8; 3],
    /// `unk_9`: the soul's scroll rate (an `s8`: 248 is -8).
    pub unk_9: i8,
    pub soul_texture: &'static str,
}

/// `sPoFieldInfo`: a Poe's, a Big Poe's.
pub static S_PO_FIELD_INFO: [EnPoFieldInfo; 2] = [
    EnPoFieldInfo { prim_color: [255, 170, 255], light_color: [100, 0, 150], env_color: [255, 85, 0], unk_9: 248u8 as i8, soul_texture: "gPoeFieldSoulTex" },
    EnPoFieldInfo { prim_color: [255, 255, 170], light_color: [255, 200, 0], env_color: [160, 0, 255], unk_9: 241u8 as i8, soul_texture: "gBigPoeSoulTex" },
];

/// The overlay's statics (docs/adr/0059-castle-towns-actors.md): `sNumSpawned`,
/// `sSpawnPositions`, `sSpawnSwitchFlags`, `sLimb7Mtx` (all zero with a fresh overlay).
#[derive(Debug, Clone)]
pub struct Statics {
    pub num_spawned: i32,
    pub spawn_positions: [[i16; 3]; 10],
    pub spawn_switch_flags: [u8; 10],
    /// `sLimb7Mtx`: the lantern's matrix, from the last draw.
    pub limb7_mtx: Mat4,
}

impl Default for Statics {
    fn default() -> Self {
        Statics { num_spawned: 0, spawn_positions: [[0; 3]; 10], spawn_switch_flags: [0; 10], limb7_mtx: Mat4::ZERO }
    }
}

/// The statics, zeroed with a fresh overlay.
pub fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_EN_PO_FIELD)
}

// shared: Actor_FreeOverlay
/// `Actor_FreeOverlay` (`z_actor.c`, from `Actor_Delete`): when the last En_Po_Field goes the
/// overlay unloads (`ACTOROVL_ALLOC_NORMAL`) and its statics come back zeroed with the next load.
fn actor_free_overlay(play: &mut PlayState) {
    let others = play.actors.all().into_iter().any(|h| play.actors.actor(h).is_some_and(|a| a.id == ACTOR_EN_PO_FIELD));
    if !others {
        play.overlay_statics.remove(&ACTOR_EN_PO_FIELD);
    }
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitForSpawn,
    Appear,
    CirclePlayer,
    Damage,
    Flee,
    Death,
    Disappear,
    SoulIdle,
    Func80AD587C,
    Func80AD58D4,
    SoulDisappear,
    SoulInteract,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::WaitForSpawn => "EnPoField_WaitForSpawn",
            Action::Appear => "EnPoField_Appear",
            Action::CirclePlayer => "EnPoField_CirclePlayer",
            Action::Damage => "EnPoField_Damage",
            Action::Flee => "EnPoField_Flee",
            Action::Death => "EnPoField_Death",
            Action::Disappear => "EnPoField_Disappear",
            Action::SoulIdle => "EnPoField_SoulIdle",
            Action::Func80AD587C => "func_80AD587C",
            Action::Func80AD58D4 => "func_80AD58D4",
            Action::SoulDisappear => "EnPoField_SoulDisappear",
            Action::SoulInteract => "EnPoField_SoulInteract",
        }
    }
}

/// `actor.update`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Update {
    /// `EnPoField_Update`.
    Living,
    /// `EnPoField_UpdateDead`.
    Dead,
}

/// `actor.draw`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Draw {
    /// `EnPoField_Draw` (the profile's).
    Body,
    /// `EnPoField_DrawSoul`.
    Soul,
}

// The bakes (docs/adr/0012-actor-bakes.md).

/// The body's segments: 8 `lightColor` (`Gfx_EnvColor`), 0xA the info's `envColor`, 0xC the
/// setup (`ACTOR_SETUP_OPA_DL` or `gActorSetupXluDL`).
const SEG_LIGHT: u8 = 0x08;
const SEG_INFO_ENV: u8 = 0x0A;
const SEG_SETUP: u8 = 0x0C;
/// The draw's own `gDPSetEnvColor` (and `gDPSetPrimColor`) before a list, as a dynamic colour run
/// before the body.
const SEG_ENV: u8 = 0x0E;
/// The soul's and the flame's `Gfx_TwoTexScroll`; the soul's texture.
const SEG_SCROLL: u8 = 0x08;
const SEG_SOUL_TEX: u8 = 0x09;
/// The flame's colours (`gDPSetPrimColor`, `gDPSetEnvColor`), run before `gEffFire1DL`.
const SEG_FLAME_COLORS: u8 = 0x0C;

/// A body's bake: a Poe's or a Big Poe's, opaque or translucent.
pub fn body_bake(big: bool, xlu: bool) -> String {
    format!("En_Po_Field/{}_{}", if big { "big_poe" } else { "poe" }, if xlu { "xlu" } else { "opa" })
}
/// `gPoeFieldLanternDL` then `gPoeFieldLanternTopDL`, in `soulColor` and the info's `envColor`.
pub const LANTERN_BAKE: &str = "En_Po_Field/lantern";
/// `gPoeFieldSoulDL` with a Poe's or a Big Poe's texture, the scroll and their colours.
pub fn soul_bake(big: bool) -> &'static str {
    if big { "En_Po_Field/big_soul" } else { "En_Po_Field/soul" }
}
/// `gEffFire1DL` with its scroll and colours.
pub const FLAME_BAKE: &str = "En_Po_Field/flame";

/// `gActorSetupXluDL` (`z_actor.c`): `gsDPSetRenderMode(G_RM_FOG_SHADE_A, G_RM_AA_ZB_XLU_SURF2)`,
/// `gsDPSetAlphaCompare(G_AC_THRESHOLD)`, the end (as `En_Poh`'s).
fn actor_setup_xlu_dl() -> Vec<(u32, u32)> {
    let mut d = Dl::default();
    d.0.push((0xE200_001C, 0xC810_49F8));
    d.othermode_l(0, 2, 1);
    d.end();
    d.0
}

/// The setup for a pass: `ACTOR_SETUP_OPA_DL` (just the end) or `gActorSetupXluDL`.
fn setup(xlu: bool) -> BakeSegment {
    BakeSegment::Commands(if xlu { actor_setup_xlu_dl() } else { vec![(0xDF00_0000, 0)] })
}

/// The soul's scroll: `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, 0, 0, 0x20, 0x40, 1, 0,
/// (gameplayFrames * unk_9) & 0x1FF, 0x20, 0x80)`.
pub fn soul_scroll(gameplay_frames: u32, unk_9: i8) -> Vec<(u32, u32)> {
    let y = gameplay_frames.wrapping_mul(unk_9 as i32 as u32) & 0x1FF;
    gfx_two_tex_scroll(0, 0, 0, 0x20, 0x40, 1, 0, y, 0x20, 0x80)
}

/// The flame's scroll: `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, 0, 0, 32, 64, 1, 0,
/// (gameplayFrames * -20) % 512, 32, 128)` (unsigned: `gameplayFrames` is a `u32`).
pub fn flame_scroll(gameplay_frames: u32) -> Vec<(u32, u32)> {
    let y = gameplay_frames.wrapping_mul(-20i32 as u32) % 512;
    gfx_two_tex_scroll(0, 0, 0, 32, 64, 1, 0, y, 32, 128)
}

/// The flame's colours: `gDPSetPrimColor(0x80, 0x80, 255, 255, 0, alpha)`, `gDPSetEnvColor(env_r,
/// 0, 0, 0)`.
pub fn flame_colors(alpha: u8, env_r: u8) -> Vec<(u32, u32)> {
    vec![(0xFA00_8080, u32::from_be_bytes([255, 255, 0, alpha])), (0xFB00_0000, u32::from_be_bytes([env_r, 0, 0, 0])), (0xDF00_0000, 0)]
}

pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for big in [false, true] {
        for xlu in [false, true] {
            // EnPoField_OverrideLimbDraw2: limb 7 (the lantern's) never drawn; a Big Poe's face,
            // cloak and body (limbs 1, 8, 9) its own.
            let mut limbs = vec![LimbOverride { limb: 6, file: PO_FIELD.into(), symbol: String::new() }];
            if big {
                limbs.push(LimbOverride { limb: 0, file: PO_FIELD.into(), symbol: "gBigPoeFaceDL".into() });
                limbs.push(LimbOverride { limb: 7, file: PO_FIELD.into(), symbol: "gBigPoeCloakDL".into() });
                limbs.push(LimbOverride { limb: 8, file: PO_FIELD.into(), symbol: "gBigPoeBodyDL".into() });
            }
            v.push(MeshBake {
                name: body_bake(big, xlu),
                object: PO_FIELD.into(),
                segments: vec![
                    (SEG_LIGHT, BakeSegment::DynamicColor { env: true, prim: false }),
                    (SEG_INFO_ENV, BakeSegment::DynamicColor { env: true, prim: false }),
                    (SEG_SETUP, setup(xlu)),
                ],
                prelude: vec![],
                body: BakeBody::Skeleton { file: PO_FIELD.into(), symbol: "gPoeFieldSkel".into(), limbs },
            });
        }
    }
    v.push(MeshBake {
        name: LANTERN_BAKE.into(),
        object: PO_FIELD.into(),
        segments: vec![(SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false }), (SEG_INFO_ENV, BakeSegment::DynamicColor { env: true, prim: false })],
        prelude: vec![SEG_ENV],
        body: BakeBody::DLists(vec![(PO_FIELD.into(), "gPoeFieldLanternDL".into()), (PO_FIELD.into(), "gPoeFieldLanternTopDL".into())]),
    });
    for (big, info) in [(false, &S_PO_FIELD_INFO[0]), (true, &S_PO_FIELD_INFO[1])] {
        v.push(MeshBake {
            name: soul_bake(big).into(),
            object: PO_FIELD.into(),
            segments: vec![
                (SEG_SCROLL, BakeSegment::Dynamic(soul_scroll(0, info.unk_9))),
                (SEG_SOUL_TEX, BakeSegment::Texture { file: PO_FIELD.into(), symbol: info.soul_texture.into() }),
                (SEG_ENV, BakeSegment::DynamicColor { env: true, prim: true }),
            ],
            prelude: vec![SEG_ENV],
            body: BakeBody::DLists(vec![(PO_FIELD.into(), "gPoeFieldSoulDL".into())]),
        });
    }
    v.push(MeshBake {
        name: FLAME_BAKE.into(),
        object: "gameplay_keep".into(),
        segments: vec![(SEG_FLAME_COLORS, BakeSegment::Dynamic(flame_colors(255, 255))), (SEG_SCROLL, BakeSegment::Dynamic(flame_scroll(0)))],
        prelude: vec![SEG_FLAME_COLORS],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffFire1DL".into())]),
    });
    v
}

/// The animations, loaded at init.
#[derive(Debug, Clone, Default)]
struct Anims {
    float: Option<Anim>,
    appear: Option<Anim>,
    flee: Option<Anim>,
    damaged: Option<Anim>,
    disappear: Option<Anim>,
}

/// `Animation_PlayLoop`.
fn play_loop(skel: &mut SkelAnimeStd, a: &Option<Anim>) {
    if let Some(a) = a.clone() {
        let last = a.last_frame();
        skel.change(a, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
    }
}
/// `Animation_MorphToLoop`.
fn morph_to_loop(skel: &mut SkelAnimeStd, a: &Option<Anim>, morph: f32) {
    if let Some(a) = a.clone() {
        skel.change(a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, morph);
    }
}
/// `Animation_PlayOnce`.
fn play_once(skel: &mut SkelAnimeStd, a: &Option<Anim>) {
    if let Some(a) = a.clone() {
        skel.play_once(a);
    }
}
/// `Animation_MorphToPlayOnce`.
fn morph_to_play_once(skel: &mut SkelAnimeStd, a: &Option<Anim>, morph: f32) {
    if let Some(a) = a.clone() {
        let last = a.last_frame();
        skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, morph);
    }
}

/// An `f32` stored to a `u8` as the console does (truncated to an `s32`, its low byte).
fn to_u8(f: f32) -> u8 {
    f as i32 as u8
}
/// An `f32` stored to an `s16`.
fn to_s16(f: f32) -> i16 {
    f as i32 as i16
}

/// `Actor_Draw`'s translation with `ACTOR_FLAG_IGNORE_QUAKE` (as `En_Poh`'s).
fn actor_model(rs: &RenderState, flags: u32, quake: Vec3) -> Mat4 {
    let mut m = actor_draw_matrix(rs);
    if flags & ACTOR_FLAG_IGNORE_QUAKE != 0 {
        m.w_axis += quake.extend(0.0);
    }
    m
}

/// What the Poe reads of Link (`GET_PLAYER(play)`).
#[derive(Debug, Clone, Copy, Default)]
struct PlayerInfo {
    pos: Vec3,
    yaw: i16,
    state1: u32,
    ride_speed: Option<f32>,
}

fn player_info(play: &PlayState) -> PlayerInfo {
    let Some(h) = play.player else { return PlayerInfo::default() };
    let Some(a) = play.actors.actor(h) else { return PlayerInfo::default() };
    let p = play.actors.get(h).and_then(|p| p.as_player());
    let state1 = p.map(|p| p.state_flags1()).unwrap_or(0);
    let ride_speed = p.and_then(|p| p.ride_actor()).and_then(|r: ActorHandle| play.actors.actor(r)).map(|r| r.speed_xz);
    PlayerInfo { pos: a.world_pos, yaw: a.shape_rot.y, state1, ride_speed }
}

pub struct EnPoField {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Anims,
    pub action: Action,
    pub update_fn: Update,
    pub draw_fn: Draw,
    /// `unk_194`: the bob's phase (32 to 1).
    pub unk_194: u8,
    /// `spawnFlagIndex`: the place a Big Poe appeared at (`sSpawnSwitchFlags`' index).
    pub spawn_flag_index: u8,
    pub action_timer: i16,
    pub flame_rotation: i16,
    pub flame_timer: i16,
    pub light_color: [u8; 4],
    pub soul_color: [u8; 4],
    /// `scaleModifier`: the scale it grows to as it appears, then a Poe's distance from Link.
    pub scale_modifier: f32,
    pub flame_scale: f32,
    pub flame_position: Vec3,
    pub light_node: Option<LightNode>,
    /// `lightInfo`: what the node holds.
    pub light_info: LightInfo,
    pub collider: ColliderCylinder,
    pub flame_collider: ColliderCylinder,
}

impl EnPoField {
    /// `EnPoField_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let p = actor.world_pos;
        let switch_flag = SWITCH_FLAG.get(actor.params) as u8;
        let st = statics(play);
        if st.num_spawned != 10 {
            let n = st.num_spawned as usize;
            st.spawn_positions[n] = [to_s16(p.x), to_s16(p.y), to_s16(p.z)];
            st.spawn_switch_flags[n] = switch_flag;
            st.num_spawned += 1;
        }
        let killed = st.num_spawned >= 2;
        let collider = ColliderCylinder::new(&CYLINDER_INIT);
        let flame_collider = ColliderCylinder::new(&FLAME_CYLINDER_INIT);
        let mut this = EnPoField {
            actor,
            skel: SkelAnimeStd::init_flex(0, None),
            skeleton: None,
            anims: Anims::default(),
            action: Action::WaitForSpawn,
            update_fn: Update::Living,
            draw_fn: Draw::Body,
            unk_194: 0,
            spawn_flag_index: 0,
            action_timer: 0,
            flame_rotation: 0,
            flame_timer: 0,
            light_color: [0; 4],
            soul_color: [0; 4],
            scale_modifier: 0.0,
            flame_scale: 0.0,
            flame_position: Vec3::ZERO,
            light_node: None,
            light_info: LightInfo::point_glow(0, 0, 0, [0; 3], 0),
            collider,
            flame_collider,
        };
        if killed {
            this.actor.params = EN_PO_FIELD_KILLED;
            this.actor.kill();
            return Box::new(this);
        }
        // sInitChain: lockOnArrowOffset 3200.
        this.actor.target_arrow_offset = 3200.0;
        // SkelAnime_Init(&gPoeFieldSkel, &gPoeFieldFloatAnim, 10 joints).
        if let Some(a) = play.assets.clone() {
            let load = |n: &str| a.animation(PO_FIELD, n).map_err(|e| log::error!("En_Po_Field: {e:#}")).ok();
            this.anims = Anims { float: load("gPoeFieldFloatAnim"), appear: load("gPoeFieldAppearAnim"), flee: load("gPoeFieldFleeAnim"), damaged: load("gPoeFieldDamagedAnim"), disappear: load("gPoeFieldDisappearAnim") };
            this.skeleton = a.skeleton(PO_FIELD, "gPoeFieldSkel").map_err(|e| log::error!("En_Po_Field: {e:#}")).ok();
        }
        let limbs = this.skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(9);
        this.skel = SkelAnimeStd::init_flex(limbs, this.anims.float.clone());
        this.actor.col_chk_info.set_info(Some(&S_DAMAGE_TABLE), &COL_CHK_INFO_INIT);
        let h = this.actor.home_pos;
        this.light_info = LightInfo::point_glow(to_s16(h.x), to_s16(h.y), to_s16(h.z), [255, 255, 255], 0);
        this.light_node = play.light_ctx.insert_light(this.light_info);
        this.actor.shape_shadow_draw = oot_game::shadow::ShadowDraw::Circle;
        this.setup_wait_for_spawn(play);
        Box::new(this)
    }

    /// The Poe's size: `params` as the C reuses it once one appears (`EnPoFieldSize`).
    pub fn size(&self) -> i16 {
        self.actor.params // schema: the runtime size (EN_PO_FIELD_SMALL or _BIG), not a placement's field
    }

    fn info(&self) -> &'static EnPoFieldInfo {
        &S_PO_FIELD_INFO[(self.size() as usize).min(1)]
    }

    /// The actor's `lightInfo` written (`Lights_Point*SetInfo` on it).
    fn set_light(&mut self, play: &mut PlayState, info: LightInfo) {
        self.light_info = info;
        play.light_ctx.set_info(self.light_node, info);
    }

    /// `Actor_ChangeCategory`.
    fn change_category(&mut self, play: &mut PlayState, category: usize) {
        self.actor.category = category;
        if let Some(me) = play.cur_actor {
            play.actors.change_category(me, category);
        }
    }

    /// `EnPoField_SetupWaitForSpawn`: invisible, unlit, 200 frames.
    fn setup_wait_for_spawn(&mut self, play: &mut PlayState) {
        self.update_fn = Update::Living;
        self.change_category(play, ACTORCAT_ENEMY);
        self.actor.shape_rot.x = 0;
        // Lights_PointSetColorAndRadius(&lightInfo, 0, 0, 0, 0).
        let mut l = self.light_info;
        l.color = [0, 0, 0];
        l.radius = 0;
        self.set_light(play, l);
        self.action_timer = 200;
        self.actor.scale = Vec3::ZERO;
        self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED);
        self.collider.base.ac_flags &= !AC_ON;
        self.collider.base.oc_flags1 = OC1_ON | OC1_TYPE_ALL;
        self.actor.col_chk_info.health = COL_CHK_INFO_INIT.health;
        self.actor.gravity = 0.0;
        self.actor.velocity.y = 0.0;
        self.action = Action::WaitForSpawn;
    }

    /// `EnPoField_SetupAppear`.
    fn setup_appear(&mut self, play: &mut PlayState) {
        play_once(&mut self.skel, &self.anims.appear);
        self.draw_fn = Draw::Body;
        self.light_color = [255, 255, 210, 0];
        self.actor.shape_shadow_alpha = 0;
        self.actor.shape_y_offset = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EN_PO_APPEAR);
        self.actor.home_pos.y = self.actor.world_pos.y;
        if self.size() == EN_PO_FIELD_BIG {
            self.actor.speed_xz = 12.0;
            self.collider.dim.radius = 35;
            self.collider.dim.height = 100;
            self.collider.dim.y_shift = 10;
            self.actor.shape_shadow_scale = 45.0;
            self.scale_modifier = 0.014;
            self.actor.navi_enemy_id = NAVI_ENEMY_BIG_POE;
        } else {
            self.actor.speed_xz = 0.0;
            self.collider.dim.radius = CYLINDER_INIT.dim.radius;
            self.collider.dim.height = CYLINDER_INIT.dim.height;
            self.collider.dim.y_shift = CYLINDER_INIT.dim.y_shift;
            self.actor.shape_shadow_scale = 37.0;
            self.scale_modifier = 0.01;
            self.actor.navi_enemy_id = NAVI_ENEMY_POE_WASTELAND;
        }
        self.action = Action::Appear;
    }

    /// `EnPoField_SetupCirclePlayer`: round Link at its distance from him.
    fn setup_circle_player(&mut self, play: &mut PlayState) {
        let pl = player_info(play);
        play_loop(&mut self.skel, &self.anims.float);
        self.collider.base.ac_flags |= AC_ON;
        self.scale_modifier = self.actor.xz_dist_to_player;
        self.actor.home_pos = pl.pos;
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        if self.action != Action::Damage {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            self.action_timer = 600;
            self.unk_194 = 32;
        }
        self.action = Action::CirclePlayer;
    }

    /// `EnPoField_SetupFlee`. @bug (game): `actionFunc` is set before it's checked (see the
    /// module's docs): the check always passes.
    fn setup_flee(&mut self) {
        morph_to_loop(&mut self.skel, &self.anims.flee, -5.0);
        self.collider.base.ac_flags |= AC_ON;
        self.action = Action::Flee;
        self.actor.speed_xz = 12.0;
        if self.action != Action::Damage {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(i16::MIN);
            self.action_timer = 2000;
            self.unk_194 = 32;
        }
    }

    /// `EnPoField_SetupDamage`: knocked back at 5 (an arrow's or a seed's way, else away from
    /// what hit it), unhittable, red for 16 frames.
    fn setup_damage(&mut self, play: &PlayState) {
        morph_to_play_once(&mut self.skel, &self.anims.damaged, -6.0);
        let dmg_flags = self.collider.info.ac_hit_elem.map(|e| e.at_dmg_info.dmg_flags).unwrap_or(0);
        if let Some(ac) = self.collider.base.ac.and_then(|h| play.actors.actor(h)) {
            if dmg_flags & (DMG_ARROW | DMG_SLINGSHOT) != 0 {
                self.actor.world_rot.y = ac.world_rot.y;
            } else {
                // Actor_WorldYawTowardActor(&this->actor, collider.base.ac) + 0x8000.
                self.actor.world_rot.y = vec3f_yaw(self.actor.world_pos, ac.world_pos).wrapping_add(i16::MIN);
            }
        }
        self.collider.base.ac_flags &= !(AC_HIT | AC_ON);
        self.actor.speed_xz = 5.0;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 16);
        self.action = Action::Damage;
    }

    /// `EnPoField_SetupDeath`.
    fn setup_death(&mut self) {
        self.action_timer = 0;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.actor.speed_xz = 0.0;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.actor.navi_enemy_id = NAVI_ENEMY_NONE;
        if self.flame_timer >= 20 {
            self.flame_timer = 19;
        }
        self.action = Action::Death;
    }

    /// `EnPoField_SetupDisappear`: 16 frames, laughing.
    fn setup_disappear(&mut self, play: &mut PlayState) {
        morph_to_loop(&mut self.skel, &self.anims.disappear, -6.0);
        self.action_timer = 16;
        self.collider.base.ac_flags &= !(AC_HIT | AC_ON);
        self.actor.speed_xz = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EN_PO_LAUGH);
        audio_play_actor_sfx2(play, NA_SE_EN_PO_DISAPPEAR);
        self.action = Action::Disappear;
    }

    /// `EnPoField_SetupSoulIdle`: only the lantern left, falling (`EnPoField_UpdateDead`,
    /// `EnPoField_DrawSoul`). @bug (game): its shadow is gone for good (see the module's docs).
    fn setup_soul_idle(&mut self, play: &mut PlayState) {
        self.update_fn = Update::Dead;
        self.draw_fn = Draw::Soul;
        self.actor.shape_shadow_draw = oot_game::shadow::ShadowDraw::None;
        self.actor.scale = Vec3::splat(0.01);
        self.actor.gravity = -1.0;
        self.actor.shape_y_offset = 1500.0;
        self.actor.shape_rot.x = i16::MIN;
        self.action_timer = 60;
        self.actor.world_pos.y -= 15.0;
        self.change_category(play, ACTORCAT_MISC);
        self.action = Action::SoulIdle;
    }

    /// `func_80AD42B0`: the lantern broken: the soul, unlit and nothing yet.
    fn func_80ad42b0(&mut self, play: &mut PlayState) {
        let p = self.actor.world_pos;
        self.set_light(play, LightInfo::point_no_glow(to_s16(p.x), to_s16(p.y), to_s16(p.z), [0, 0, 0], 0));
        self.actor.shape_rot.y = 0;
        self.light_color[3] = 0;
        self.actor.shape_rot.x = 0;
        self.actor.shape_y_offset = 0.0;
        self.actor.gravity = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.home_pos.y = self.actor.world_pos.y;
        self.actor.scale.x = 0.0;
        self.actor.scale.y = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EV_METAL_BOX_BOUND);
        if self.size() == EN_PO_FIELD_BIG {
            play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
        }
        self.action = Action::Func80AD587C;
    }

    /// `func_80AD4384`: the soul whole: its small cylinder (Link's only), 0x5005, 400 frames.
    fn func_80ad4384(&mut self) {
        self.actor.home_pos.y = self.actor.world_pos.y;
        self.actor.set_focus(-10.0);
        let p = self.actor.world_pos;
        self.collider.dim.radius = 13;
        self.collider.dim.height = 30;
        self.collider.dim.y_shift = 0;
        self.collider.dim.pos = [to_s16(p.x), to_s16(p.y - 20.0), to_s16(p.z)];
        self.collider.base.oc_flags1 = OC1_ON | OC1_TYPE_PLAYER;
        self.actor.text_id = TEXT_POE_SOUL;
        self.action_timer = 400;
        self.unk_194 = 32;
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::Func80AD58D4;
    }

    /// `EnPoField_SetupSoulDisappear`.
    fn setup_soul_disappear(&mut self) {
        self.action = Action::SoulDisappear;
    }

    /// `EnPoField_SetupInteractWithSoul`.
    fn setup_interact_with_soul(&mut self) {
        self.action = Action::SoulInteract;
        self.actor.home_pos.y = self.actor.world_pos.y - 15.0;
    }

    /// `EnPoField_CorrectYPos`: bobbing 13 about 13 above Link's height or the floor's, whichever
    /// is higher; off any floor, it disappears.
    fn correct_y_pos(&mut self, play: &mut PlayState) {
        let pl = player_info(play);
        if self.unk_194 == 0 {
            self.unk_194 = 32;
        }
        if self.unk_194 != 0 {
            self.unk_194 -= 1;
        }
        if self.actor.floor_height == BGCHECK_Y_MIN {
            self.setup_disappear(play);
            return;
        }
        let target = if pl.pos.y > self.actor.floor_height { pl.pos.y } else { self.actor.floor_height } + 13.0;
        approach_f(&mut self.actor.home_pos.y, target, 0.2, 5.0);
        self.actor.world_pos.y = sin_s((self.unk_194 as i32 * 0x800) as i16) * 13.0 + self.actor.home_pos.y;
    }

    /// `EnPoField_SetFleeSpeed`: by the horse's speed (12 on foot) and Link's distance; at least
    /// 12.
    fn set_flee_speed(&mut self, play: &PlayState) {
        let pl = player_info(play);
        let speed = if pl.state1 & PLAYER_STATE1_23 != 0
            && let Some(s) = pl.ride_speed
        {
            s
        } else {
            12.0
        };
        if self.actor.xz_dist_to_player < 300.0 {
            self.actor.speed_xz = speed * 1.5 + 2.0;
        } else if self.actor.xz_dist_to_player < 400.0 {
            self.actor.speed_xz = speed * 1.25 + 2.0;
        } else if self.actor.xz_dist_to_player < 500.0 {
            self.actor.speed_xz = speed + 2.0;
        } else {
            self.actor.speed_xz = 12.0;
        }
        if self.actor.speed_xz < 12.0 {
            self.actor.speed_xz = 12.0;
        }
    }

    /// `EnPoField_WaitForSpawn`: see the module's docs.
    fn wait_for_spawn(&mut self, play: &mut PlayState) {
        let pl = player_info(play);
        if self.action_timer != 0 {
            self.action_timer -= 1;
        }
        if self.action_timer != 0 {
            return;
        }
        let st = statics(play).clone();
        for i in 0..st.num_spawned.max(0) as usize {
            let place = st.spawn_positions[i];
            if (place[0] as f32 - pl.pos.x).abs() < 150.0 && (place[2] as f32 - pl.pos.z).abs() < 150.0 {
                let spawn_dist;
                // schema: the place's switch flag (sSpawnSwitchFlags, from its switchFlag)
                if play.flags.get_switch(st.spawn_switch_flags[i] as i32) {
                    if pl.state1 & PLAYER_STATE1_23 != 0 {
                        // Riding: nothing.
                        return;
                    }
                    self.actor.params = EN_PO_FIELD_SMALL;
                    spawn_dist = 300.0;
                } else if pl.state1 & PLAYER_STATE1_23 != 0 || play.rand.zero_one() < 0.4 {
                    self.actor.params = EN_PO_FIELD_BIG;
                    self.spawn_flag_index = i as u8;
                    spawn_dist = 480.0;
                } else {
                    self.actor.params = EN_PO_FIELD_SMALL;
                    spawn_dist = 300.0;
                }
                self.actor.world_pos.x = sin_s(pl.yaw) * spawn_dist + pl.pos.x;
                self.actor.world_pos.z = cos_s(pl.yaw) * spawn_dist + pl.pos.z;
                self.actor.world_pos.y = pl.pos.y + 1000.0;
                // BgCheck_EntityRaycastDown4.
                let (y, poly) = play.col.entity_raycast_down(self.actor.world_pos);
                self.actor.world_pos.y = y;
                self.actor.floor_poly = poly;
                if self.actor.world_pos.y != BGCHECK_Y_MIN {
                    self.actor.shape_rot.y = vec3f_yaw(self.actor.world_pos, pl.pos);
                    self.setup_appear(play);
                } else {
                    return;
                }
            }
        }
    }

    /// `EnPoField_Appear`: growing over its first 10 frames, fading in after; then a Big Poe
    /// flees, a Poe circles.
    fn appear(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.light_color[3] = 255;
            self.actor.scale = Vec3::splat(self.scale_modifier);
            if self.size() == EN_PO_FIELD_BIG {
                self.setup_flee();
            } else {
                self.setup_circle_player(play);
            }
        } else if self.skel.cur_frame > 10.0 {
            self.light_color[3] = to_u8(((self.skel.cur_frame - 10.0) * 0.05) * 255.0);
        } else {
            self.actor.scale.x += self.scale_modifier * 0.1;
            self.actor.scale.y = self.actor.scale.x;
            self.actor.scale.z = self.actor.scale.x;
        }
        self.actor.shape_shadow_alpha = self.light_color[3];
        self.actor.shape_rot.y = self.actor.yaw_towards_player;
        if self.size() == EN_PO_FIELD_BIG {
            self.actor.world_rot.y = self.actor.yaw_towards_player.wrapping_add(i16::MIN);
            self.set_flee_speed(play);
        }
    }

    /// `EnPoField_CirclePlayer`.
    fn circle_player(&mut self, play: &mut PlayState) {
        let pl = player_info(play);
        let temp_v1 = 16 - self.unk_194 as i32;
        self.skel.update();
        if self.action_timer != 0 {
            self.action_timer -= 1;
        }
        if temp_v1.abs() < 16 {
            // An s16 += f32: the sum as an f32, stored back.
            let add = 512.0 * sin_s((self.unk_194 as i32 * 0x800) as i16).abs();
            self.actor.world_rot.y = to_s16(self.actor.world_rot.y as f32 + add);
        }
        approach_f(&mut self.scale_modifier, 180.0, 0.5, 10.0);
        approach_f(&mut self.actor.home_pos.x, pl.pos.x, 0.2, 6.0);
        approach_f(&mut self.actor.home_pos.z, pl.pos.z, 0.2, 6.0);
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.world_rot.y, 1, 0x800, 0x200);
        if self.actor.home_pos.x - pl.pos.x > 100.0 {
            self.actor.home_pos.x = pl.pos.x + 100.0;
        } else if self.actor.home_pos.x - pl.pos.x < -100.0 {
            self.actor.home_pos.x = pl.pos.x + -100.0;
        }
        if self.actor.home_pos.z - pl.pos.z > 100.0 {
            self.actor.home_pos.z = pl.pos.z + 100.0;
        } else if self.actor.home_pos.z - pl.pos.z < -100.0 {
            self.actor.home_pos.z = pl.pos.z + -100.0;
        }
        self.actor.world_pos.x = self.actor.home_pos.x - sin_s(self.actor.world_rot.y) * self.scale_modifier;
        self.actor.world_pos.z = self.actor.home_pos.z - cos_s(self.actor.world_rot.y) * self.scale_modifier;
        if self.action_timer == 0 {
            self.setup_disappear(play);
        } else {
            self.spawn_flame();
        }
        self.correct_y_pos(play);
        self.actor.play_sfx_flagged(NA_SE_EN_PO_FLY - SFX_FLAG);
    }

    /// `EnPoField_Flee`.
    fn flee(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.action_timer != 0 {
            self.action_timer -= 1;
        }
        let w = self.actor.world_pos;
        let dist_middle = ((S_FIELD_MIDDLE.x - w.x) * (S_FIELD_MIDDLE.x - w.x) + (S_FIELD_MIDDLE.z - w.z) * (S_FIELD_MIDDLE.z - w.z)).sqrt();
        let phi_t0: i16 = if dist_middle > 3000.0 {
            // (s16)(yawTowardsPlayer - Actor_WorldYawTowardPoint(sFieldMiddle) - 0x8000) * 0.2f.
            let d = (self.actor.yaw_towards_player as i32 - vec3f_yaw(w, S_FIELD_MIDDLE) as i32 - 0x8000) as i16;
            to_s16(d as f32 * 0.2)
        } else {
            0
        };
        approach_s(&mut self.actor.shape_rot.y, (self.actor.yaw_towards_player as i32 - phi_t0 as i32) as i16, 6, 0x400);
        self.set_flee_speed(play);
        self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(i16::MIN);
        let temp_f6 = sin_s((self.action_timer as i32 * 0x800) as i16) * 3.0;
        self.actor.world_pos.x -= temp_f6 * cos_s(self.actor.shape_rot.y);
        self.actor.world_pos.z += temp_f6 * sin_s(self.actor.shape_rot.y);
        if self.action_timer == 0 || self.actor.xz_dist_to_player > 1500.0 {
            self.setup_disappear(play);
        } else {
            self.correct_y_pos(play);
        }
        self.actor.play_sfx_flagged(NA_SE_EN_PO_AWAY - SFX_FLAG);
    }

    /// `EnPoField_Damage`: slowing; its damage animation done, burning at 0 health, else a Big
    /// Poe flees, a Poe circles.
    fn damage(&mut self, play: &mut PlayState) {
        step_to_f(&mut self.actor.speed_xz, 0.0, 0.5);
        if self.skel.update() {
            if self.actor.col_chk_info.health == 0 {
                self.setup_death();
            } else if self.size() == EN_PO_FIELD_BIG {
                self.setup_flee();
            } else {
                self.setup_circle_player(play);
            }
        }
    }

    /// `EnPoField_Death`: three columns of flames for 7 frames (either side of it across the
    /// camera's view, and its middle), rising and shrinking from 19, the lantern left at 28.
    fn death(&mut self, play: &mut PlayState) {
        self.action_timer += 1;
        let p = self.actor.world_pos;
        if self.action_timer < 8 {
            let cam = play.cam_dir_yaw();
            let mut sp6c = Vec3::ZERO;
            if self.action_timer < 5 {
                let a = (self.action_timer as i32 * 0x1000 - 0x4000) as i16;
                sp6c.y = sin_s(a) * 23.0 + (p.y + 40.0);
                let sp68 = cos_s(a) * 23.0;
                sp6c.x = sin_s(cam.wrapping_add(0x4800)) * sp68 + p.x;
                sp6c.z = cos_s(cam.wrapping_add(0x4800)) * sp68 + p.z;
            } else {
                sp6c.y = p.y + 40.0 + 15.0 * (self.action_timer - 5) as f32;
                sp6c.x = sin_s(cam.wrapping_add(0x4800)) * 23.0 + p.x;
                sp6c.z = cos_s(cam.wrapping_add(0x4800)) * 23.0 + p.z;
            }
            let scale = self.action_timer * 10 + 80;
            let spawn = |play: &mut PlayState, v: Vec3| play.with_ss(|s| s.dead_db_spawn(v, D_80AD7114, D_80AD7120, scale, 0, [255, 255, 255, 255], [0, 0, 255], 1, 9, 1));
            spawn(play, sp6c);
            sp6c.x = (p.x + p.x) - sp6c.x;
            sp6c.z = (p.z + p.z) - sp6c.z;
            spawn(play, sp6c);
            sp6c.x = p.x;
            sp6c.z = p.z;
            spawn(play, sp6c);
            if self.action_timer == 1 {
                audio_play_actor_sfx2(play, NA_SE_EN_EXTINCT);
            }
        } else if self.action_timer == 28 {
            self.setup_soul_idle(play);
        } else if self.action_timer >= 19 {
            let temp_f0 = (28 - self.action_timer) as f32 * 0.001;
            self.actor.world_pos.y += 5.0;
            self.actor.scale = Vec3::splat(temp_f0);
        }
        if self.action_timer == 18 {
            audio_play_actor_sfx2(play, NA_SE_EN_PO_DEAD2);
        }
    }

    /// `EnPoField_Disappear`: 16 frames' fade, spinning; then waiting again.
    fn disappear(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.action_timer != 0 {
            self.action_timer -= 1;
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x1000);
        self.light_color[3] = to_u8(self.action_timer as f32 * 15.9375);
        self.actor.shape_shadow_alpha = self.light_color[3];
        if self.action_timer == 0 {
            self.setup_wait_for_spawn(play);
        }
    }

    /// `EnPoField_SoulIdle`: the lantern falling; on the floor it breaks and the soul comes; 60
    /// frames without, waiting again.
    fn soul_idle(&mut self, play: &mut PlayState) {
        if self.action_timer != 0 {
            self.action_timer -= 1;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            let pos = self.actor.world_pos;
            play.with_ss(|s| s.hahen_spawn_burst(pos, 6.0, 0, 1, 1, 15, OBJECT_PO_FIELD, 10, Some((PO_FIELD, "gPoeFieldLanternDL"))));
            self.func_80ad42b0(play);
        } else if self.action_timer == 0 {
            self.setup_wait_for_spawn(play);
        }
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 10.0, 10.0, 10.0, UPDBGCHECKINFO_FLAG_2);
    }

    /// `EnPoField_SoulUpdateProperties`: the soul's alpha by `arg1` (20 up, 13 down), its size
    /// (growing; or thinning and stretching as it fades) and light.
    fn soul_update_properties(&mut self, play: &mut PlayState, arg1: i32) {
        let info = self.info();
        self.light_color[3] = (self.light_color[3] as i32 + arg1).clamp(0, 255) as u8;
        let multiplier;
        if arg1 < 0 {
            multiplier = self.light_color[3] as f32 * (1.0 / 255.0);
            self.actor.scale.x = 0.0056000003 * multiplier + 0.0014000001;
            self.actor.scale.z = self.actor.scale.x;
            self.actor.scale.y = 0.007 - 0.007 * multiplier + 0.007;
        } else {
            multiplier = 1.0;
            self.actor.scale = Vec3::splat(self.light_color[3] as f32 * (0.007 / 255.0));
            self.actor.world_pos.y = self.actor.home_pos.y + ((1.0 / 17.0) * self.light_color[3] as f32);
        }
        self.light_color[0] = to_u8(info.light_color[0] as f32 * multiplier);
        self.light_color[1] = to_u8(info.light_color[1] as f32 * multiplier);
        self.light_color[2] = to_u8(info.light_color[2] as f32 * multiplier);
        let p = self.actor.world_pos;
        self.set_light(play, LightInfo::point_no_glow(to_s16(p.x), to_s16(p.y), to_s16(p.z), info.light_color, to_s16(self.light_color[3] as f32 * (200.0 / 255.0))));
    }

    /// `func_80AD587C`: the soul rising 2 a frame and growing; whole, it waits to be touched.
    fn func_80ad587c(&mut self, play: &mut PlayState) {
        self.actor.home_pos.y += 2.0;
        self.soul_update_properties(play, 20);
        if self.light_color[3] == 255 {
            self.func_80ad4384();
        }
    }

    /// `func_80AD58D4`: the soul waiting 400 frames, bobbing; Link touching it talks at once; its
    /// time out, it laughs and goes.
    fn func_80ad58d4(&mut self, play: &mut PlayState) {
        if self.action_timer != 0 {
            self.action_timer -= 1;
        }
        if oot_game::npc::process_talk_request(&mut self.actor) {
            self.setup_interact_with_soul();
            return;
        }
        if self.action_timer == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_PO_LAUGH);
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            self.setup_soul_disappear();
            return;
        }
        if self.collider.base.oc_flags1 & OC1_HIT != 0 {
            self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            let a = self.actor.clone();
            oot_game::npc::offer_talk_default(play, &a);
        } else {
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
        self.actor.world_pos.y = sin_s((self.unk_194 as i32 * 0x800) as i16) * 5.0 + self.actor.home_pos.y;
        if self.unk_194 != 0 {
            self.unk_194 -= 1;
        }
        if self.unk_194 == 0 {
            self.unk_194 = 32;
        }
        self.collider.dim.pos[1] = to_s16(self.actor.world_pos.y - 20.0);
        self.actor.set_focus(-10.0);
        let p = self.actor.world_pos;
        let c = self.light_info.color;
        self.set_light(play, LightInfo::point_no_glow(to_s16(p.x), to_s16(p.y), to_s16(p.z), c, to_s16(self.light_color[3] as f32 * (200.0 / 255.0))));
    }

    /// `EnPoField_SoulDisappear`: fading 13 a frame, then waiting again.
    fn soul_disappear(&mut self, play: &mut PlayState) {
        self.soul_update_properties(play, -13);
        if self.light_color[3] == 0 {
            self.setup_wait_for_spawn(play);
        }
    }

    /// `EnPoField_SoulInteract`: the soul's ask (crying); the answer: bottled (an empty bottle),
    /// or not; it fades as the answer shows.
    fn soul_interact(&mut self, play: &mut PlayState) {
        let asking = self.actor.text_id == TEXT_POE_SOUL;
        if !asking {
            self.soul_update_properties(play, -13);
        } else {
            self.actor.play_sfx_flagged(NA_SE_EN_PO_BIG_CRY - SFX_FLAG);
        }
        if play.message_state() == TEXT_STATE_CHOICE {
            if play.message_should_advance() {
                play.audio.stop_sfx_by_pos_and_id(cur_sfx_pos(play), NA_SE_EN_PO_BIG_CRY - SFX_FLAG);
                if play.msg_ctx.choice_index == 0 {
                    if oot_game::item::inventory_has_empty_bottle(&play.save) {
                        audio_play_actor_sfx2(play, NA_SE_EN_PO_BIG_GET);
                        if self.size() == EN_PO_FIELD_SMALL {
                            oot_game::item::item_give(&mut play.save, Some(&mut play.audio), ITEM_BOTTLE_POE);
                            self.actor.text_id = TEXT_POE_BOTTLED;
                        } else {
                            self.actor.text_id = TEXT_BIG_POE_BOTTLED;
                            oot_game::item::item_give(&mut play.save, Some(&mut play.audio), ITEM_BOTTLE_BIG_POE);
                            let flag = statics(play).spawn_switch_flags[self.spawn_flag_index as usize];
                            // schema: the place's switch flag (sSpawnSwitchFlags, from its switchFlag)
                            play.flags.set_switch(flag as i32);
                        }
                    } else {
                        audio_play_actor_sfx2(play, NA_SE_EN_PO_LAUGH);
                        self.actor.text_id = TEXT_POE_NO_BOTTLE;
                    }
                } else {
                    self.actor.text_id = TEXT_POE_NO;
                    audio_play_actor_sfx2(play, NA_SE_EN_PO_LAUGH);
                }
                play.continue_textbox(self.actor.text_id);
            }
        } else if oot_game::npc::textbox_is_closing(play) {
            self.setup_soul_disappear();
        }
    }

    /// `EnPoField_TestForDamage`: a hit with damage or a reaction: the finishing blow at 0 health
    /// (`NA_SE_EN_PO_DEAD`), else `NA_SE_EN_PO_DAMAGE`; hurt either way.
    fn test_for_damage(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            if self.actor.col_chk_info.damage_reaction != 0 || self.actor.col_chk_info.damage != 0 {
                if self.actor.apply_damage() == 0 {
                    enemy_start_finishing_blow(play, &self.actor);
                    audio_play_actor_sfx2(play, NA_SE_EN_PO_DEAD);
                } else {
                    audio_play_actor_sfx2(play, NA_SE_EN_PO_DAMAGE);
                }
                self.setup_damage(play);
            }
        }
    }

    /// `EnPoField_SpawnFlame`: at its light, 70 frames, the way it faces.
    fn spawn_flame(&mut self) {
        if self.flame_timer == 0 {
            self.flame_position = Vec3::new(self.light_info.x as f32, self.light_info.y as f32, self.light_info.z as f32);
            self.flame_timer = 70;
            self.flame_rotation = self.actor.shape_rot.y;
        }
    }

    /// `EnPoField_UpdateFlame`: growing to 0.003 and sliding 2.5 a frame, burning Link; from 19
    /// frames left (or once it has hit), shrinking.
    fn update_flame(&mut self, play: &mut PlayState) {
        if self.flame_timer != 0 {
            if self.flame_timer != 0 {
                self.flame_timer -= 1;
            }
            if self.flame_collider.base.at_flags & AT_HIT != 0 {
                self.flame_collider.base.at_flags &= !AT_HIT;
                self.flame_timer = 19;
            }
            if self.flame_timer < 20 {
                step_to_f(&mut self.flame_scale, 0.0, 0.00015);
                return;
            }
            if step_to_f(&mut self.flame_scale, 0.003, 0.0006) {
                self.flame_position.x += 2.5 * sin_s(self.flame_rotation);
                self.flame_position.z += 2.5 * cos_s(self.flame_rotation);
            }
            let f = self.flame_position;
            self.flame_collider.dim.pos = [to_s16(f.x), to_s16(f.y), to_s16(f.z)];
            play.collision_check_set_at(&self.actor, 1, &mut self.flame_collider);
        }
    }

    /// `func_80AD619C`: `lightColor` towards (80, 255, 225) while it flees, flashing while hurt,
    /// else towards (255, 255, 210).
    fn func_80ad619c(&mut self) {
        let c = &mut self.light_color;
        match self.action {
            Action::Flee => {
                c[0] = (c[0] as i16 + 5).min(80) as u8;
                c[1] = (c[1] as i16 + 5).min(255) as u8;
                c[2] = (c[2] as i16 + 5).min(225) as u8;
            }
            Action::Damage => {
                if self.actor.color_filter_timer & 2 != 0 {
                    *c = [0, 0, 0, c[3]];
                } else {
                    *c = [80, 255, 225, c[3]];
                }
            }
            _ => {
                c[0] = (c[0] as i16 + 5).min(255) as u8;
                c[1] = (c[1] as i16 + 5).min(255) as u8;
                if c[2] > 210 {
                    c[2] = (c[2] as i16 - 5).max(210) as u8;
                } else {
                    c[2] = (c[2] as i16 + 5).min(210) as u8;
                }
            }
        }
    }

    /// `func_80AD6330`: `soulColor`, the lantern's flame: brightening over the appear animation's
    /// first 12 frames, else flickering (`Rand_ZeroOne`).
    fn func_80ad6330(&mut self, play: &mut PlayState) {
        if self.action == Action::Appear && self.skel.cur_frame < 12.0 {
            let v = ((self.skel.cur_frame * 16.66) as i16 + 55) as u8;
            self.soul_color[0] = v;
            self.soul_color[1] = v;
            self.soul_color[2] = v;
            self.soul_color[3] = to_u8(self.skel.cur_frame * (100.0 / 6.0));
        } else {
            let rand = play.rand.zero_one();
            self.soul_color[0] = ((rand * 30.0) as i16 + 225) as u8;
            self.soul_color[1] = ((rand * 100.0) as i16 + 155) as u8;
            self.soul_color[2] = ((rand * 160.0) as i16 + 95) as u8;
            self.soul_color[3] = 200;
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::WaitForSpawn => self.wait_for_spawn(play),
            Action::Appear => self.appear(play),
            Action::CirclePlayer => self.circle_player(play),
            Action::Damage => self.damage(play),
            Action::Flee => self.flee(play),
            Action::Death => self.death(play),
            Action::Disappear => self.disappear(play),
            Action::SoulIdle => self.soul_idle(play),
            Action::Func80AD587C => self.func_80ad587c(play),
            Action::Func80AD58D4 => self.func_80ad58d4(play),
            Action::SoulDisappear => self.soul_disappear(play),
            Action::SoulInteract => self.soul_interact(play),
        }
    }

    /// `EnPoField_Update`.
    fn update_living(&mut self, play: &mut PlayState) {
        self.test_for_damage(play);
        self.run_action(play);
        self.update_flame(play);
        if matches!(self.action, Action::Flee | Action::Damage | Action::Appear) {
            self.actor.move_forward();
        }
        if self.action != Action::WaitForSpawn {
            self.actor.set_focus(42.0);
            self.actor.update_bg_check_info(&play.col, 0.0, 27.0, 60.0, UPDBGCHECKINFO_FLAG_2);
            self.func_80ad619c();
            self.func_80ad6330(play);
            self.collider.update(&self.actor);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            if self.collider.base.ac_flags & AC_ON != 0 {
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            }
        }
    }

    /// `EnPoField_UpdateDead`.
    fn update_dead(&mut self, play: &mut PlayState) {
        self.run_action(play);
        if self.action == Action::SoulIdle {
            self.func_80ad6330(play);
        }
        self.update_flame(play);
    }

    /// Whether `EnPoField_OverrideLimbDraw2` hides every limb: invisible, or burning from frame 2.
    fn body_hidden(&self) -> bool {
        self.light_color[3] == 0 || (self.action == Action::Death && self.action_timer >= 2)
    }

    /// The skeleton posed as `EnPoField_Draw` draws it, from `joints`: `EnPoField_OverrideLimbDraw2`
    /// shrinks limb 7 as it disappears (`actionTimer / 16`), and `EnPoField_PostLimDraw2`'s scale
    /// after limb 7 (keeping the lantern its size as the body shrinks, from burning frame 19)
    /// carries to limb 7's children. With limb 7's matrix before that scale (the light's) and
    /// after it (`sLimb7Mtx`).
    fn pose(&self, skeleton: &Skeleton, joints: &[[i16; 3]], model: Mat4) -> (Vec<Mat4>, Mat4, Mat4) {
        let disappear = (self.action == Action::Disappear).then(|| self.action_timer as f32 / 16.0);
        let post_scale = (self.action == Action::Death && self.action_timer >= 19 && self.actor.scale.x != 0.0).then(|| 0.01 / self.actor.scale.x);
        let parents = &skeleton.parents;
        let bones = skeleton.pose_override(joints, |limb, _, _| {
            let mut pre = Mat4::IDENTITY;
            if limb == 7
                && let Some(s) = disappear
            {
                pre = Mat4::from_scale(Vec3::splat(s));
            }
            if let Some(s) = post_scale
                && parents.get(limb - 1).copied().flatten() == Some(6)
            {
                pre *= Mat4::from_scale(Vec3::splat(s));
            }
            pre
        });
        let limb7 = model * bones.get(6).copied().unwrap_or(Mat4::IDENTITY);
        let lantern = match post_scale {
            Some(s) => limb7 * Mat4::from_scale(Vec3::splat(s)),
            None => limb7,
        };
        (bones, limb7, lantern)
    }
}

impl ActorImpl for EnPoField {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    fn update(&mut self, play: &mut PlayState) {
        match self.update_fn {
            Update::Living => self.update_living(play),
            Update::Dead => self.update_dead(play),
        }
    }

    /// The draws' changes: `EnPoField_PostLimDraw2`'s (`sLimb7Mtx`; at burning frame 27 the actor
    /// moves to it; the light 1400 up limb 7 in `soulColor`), or `EnPoField_DrawSoul`'s light at
    /// the falling lantern.
    fn draw_update(&mut self, play: &mut PlayState) {
        match self.draw_fn {
            Draw::Body => {
                if self.action == Action::WaitForSpawn {
                    return;
                }
                let Some(skeleton) = self.skeleton.clone() else { return };
                let rs = RenderState::of(&self.actor);
                let model = actor_model(&rs, self.actor.flags, play.game_camera.quake_offset);
                let (_, limb7, lantern) = self.pose(&skeleton, &self.skel.joint_table, model);
                let vec = limb7.transform_point3(D_80AD714C);
                statics(play).limb7_mtx = lantern;
                if self.action == Action::Death && self.action_timer == 27 {
                    self.actor.world_pos = lantern.w_axis.truncate();
                }
                let s = self.soul_color;
                self.set_light(play, LightInfo::point_glow(to_s16(vec.x), to_s16(vec.y), to_s16(vec.z), [s[0], s[1], s[2]], to_s16(s[3] as f32 * (200.0 / 255.0))));
            }
            Draw::Soul => {
                if self.action == Action::SoulIdle {
                    let p = self.actor.world_pos;
                    let s = self.soul_color;
                    self.set_light(play, LightInfo::point_glow(to_s16(p.x), to_s16(p.y), to_s16(p.z), [s[0], s[1], s[2]], 200));
                }
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs
    }

    /// `EnPoField_Draw`, `EnPoField_DrawSoul`, each with `EnPoField_DrawFlame`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let model = actor_model(rs, self.actor.flags, play.game_camera.quake_offset);
        let info = self.info();
        let lc = self.light_color;
        let sc = self.soul_color;
        let env = |c: [u8; 3]| [c[0], c[1], c[2], 255];
        let lantern_params = || {
            let mut sv = SegmentValues::default();
            sv.env[SEG_ENV as usize] = Some([sc[0], sc[1], sc[2], 255]);
            sv.env[SEG_INFO_ENV as usize] = Some(env(info.env_color));
            DrawParams { segments: Some(sv), ..Default::default() }
        };
        match self.draw_fn {
            Draw::Body => {
                if self.action != Action::WaitForSpawn
                    && let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton)
                {
                    let (bones, _, lantern) = self.pose(skeleton, &joints.rot, model);
                    let xlu = !(lc[3] == 255 || lc[3] == 0);
                    let big = self.size() == EN_PO_FIELD_BIG;
                    let push = |out: &mut DrawOut, cmd: DrawCmd| if xlu { out.xlu.push(cmd) } else { out.opa.push(cmd) };
                    if !self.body_hidden() {
                        let mut sv = SegmentValues::default();
                        sv.env[SEG_LIGHT as usize] = Some(lc);
                        sv.env[SEG_INFO_ENV as usize] = Some(env(info.env_color));
                        push(out, DrawCmd { mesh: MeshKey::named(keys::bake(&body_bake(big, xlu))), transform: model, bones: bones.clone(), params: DrawParams { segments: Some(sv), ..Default::default() } });
                    }
                    // EnPoField_PostLimDraw2: burning from frame 2, the burning shape on limb 8.
                    if self.action == Action::Death && self.action_timer >= 2 {
                        let m = model * bones.get(7).copied().unwrap_or(Mat4::IDENTITY);
                        push(out, DrawCmd::new(MeshKey::named(keys::mesh(PO_FIELD, "gPoeFieldBurnDL")), m));
                    }
                    // The lantern at sLimb7Mtx, in soulColor.
                    out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(LANTERN_BAKE)), transform: lantern, bones: Vec::new(), params: lantern_params() });
                }
            }
            Draw::Soul => {
                if self.action == Action::SoulIdle {
                    out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(LANTERN_BAKE)), transform: model, bones: Vec::new(), params: lantern_params() });
                } else {
                    let mut sv = SegmentValues::default();
                    sv.read(SEG_SCROLL, &soul_scroll(play.gameplay_frames, info.unk_9));
                    sv.env[SEG_ENV as usize] = Some([lc[0], lc[1], lc[2], 255]);
                    sv.prim[SEG_ENV as usize] = Some([info.prim_color[0], info.prim_color[1], info.prim_color[2], lc[3]]);
                    // Matrix_RotateY((s16)(camDirYaw + 0x8000) * 9.58738e-05f, MTXMODE_APPLY).
                    let yaw = play.cam_dir_yaw().wrapping_add(i16::MIN) as f32 * 9.58738e-05;
                    let m = model * Mat4::from_rotation_y(yaw);
                    out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(soul_bake(self.size() == EN_PO_FIELD_BIG))), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
                }
            }
        }
        // EnPoField_DrawFlame.
        if self.flame_timer != 0 {
            let alpha_f = self.flame_scale * 85000.0;
            let alpha = to_u8(alpha_f);
            let rot = Mat4::from_rotation_y(binang_to_rad(play.cam_dir_yaw().wrapping_add(i16::MIN)));
            let (env_r, scale) = if self.flame_timer >= 20 {
                (255, Vec3::splat(self.flame_scale))
            } else {
                (alpha, Vec3::new((self.flame_scale * 0.7) + 0.00090000004, (0.003 - self.flame_scale) + 0.003, 0.003))
            };
            let m = Mat4::from_translation(self.flame_position) * rot * Mat4::from_scale(scale);
            let mut sv = SegmentValues::default();
            sv.read(SEG_SCROLL, &flame_scroll(play.gameplay_frames));
            sv.read(SEG_FLAME_COLORS, &flame_colors(alpha, env_r));
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(FLAME_BAKE)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
    }

    /// `EnPoField_Destroy`: the light out (not for the nine killed at init); the overlay's statics
    /// go with the last instance.
    fn destroy(&mut self, play: &mut PlayState) {
        if self.size() != EN_PO_FIELD_KILLED {
            play.light_ctx.remove_light(self.light_node.take());
        }
        actor_free_overlay(play);
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            0 => Some(ColliderMut::Cylinder(&mut self.collider)),
            1 => Some(ColliderMut::Cylinder(&mut self.flame_collider)),
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
