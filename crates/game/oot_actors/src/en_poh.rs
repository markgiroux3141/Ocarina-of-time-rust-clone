//! `En_Poh` (`ovl_En_Poh/z_en_poh.c`): the Poes, a ghost carrying a lantern that fades in and out,
//! and the composer brothers Sharp and Flat. Four types by `params` (`EnPohType`):
//!
//! - **`EN_POH_NORMAL`** (0): a Poe (`object_poh`, `sPoeInfo[EN_POH_INFO_NORMAL]`). Dodongo's Cavern
//!   places three in room 4 and one in room 14, the Shadow Temple's beneath-the-graveyard one in
//!   room 2, and the graveyard's adult layers three or one.
//! - **`EN_POH_RUPEE`** (1): a Poe that `Bg_Haka` (a grave pulled back at night in the graveyard)
//!   spawns; at init it drops a blue rupee where it is (`Item_DropCollectible`, `0x4000 |
//!   ITEM00_RUPEE_BLUE`, not moving). Only two at a time (`D_80AE1A50`, the overlay's count): the
//!   third dies at init.
//! - **`EN_POH_SHARP`** (2), **`EN_POH_FLAT`** (3): the composer brothers (`object_po_composer`,
//!   `sPoeInfo[EN_POH_INFO_COMPOSER]`), spawned by `En_Wonder_Talk` at the graveyard's gravestone.
//!   Each dies at init once met (Sharp: the switch 0x29, Flat: 0x28, set at init) or once the Sun's
//!   Song's grave has opened (switch 9). Their lantern's top and their robes take their colours
//!   (`D_80AE1B4C` to `D_80AE1B58`); Flat's head is his own (`gPoeComposerFlatHeadDL`).
//!
//! Any params from 4 up is made `EN_POH_NORMAL` at init (a negative one stays: it's an `s16`, below
//! 4, read as a normal Poe all the same).
//!
//! **Waiting** (`EnPoh_Update`): it waits for its object (`Object_IsLoaded`), then sets up its
//! skeleton (`gPoeSkel`, 21 joints; the composers' flex `gPoeComposerSkel`, 12, turned -0x4000, a
//! taller cylinder and the lantern's sphere on limb 9, flipped), loses
//! `ACTOR_FLAG_UPDATE_CULLING_DISABLED` (from now on it updates only in view) and appears.
//!
//! **Appearing**: a Poe waits invisible (`func_80ADEF38`, its appear animation stopped) until Link
//! is within 280, then laughs in (`NA_SE_EN_PO_APPEAR`), fading in from the animation's frame 10;
//! a composer rises 20 and fades in at once (`EnPoh_ComposerAppear`, 25.5 a frame). Then targetable.
//!
//! **Floating** (`EnPoh_UpdateLiving`): bobbing towards Link's height (`EnPoh_MoveTowardsPlayerHeight`).
//! It idles (`EnPoh_Idle`, `func_80ADEAC4`, `func_80ADF5E0`: drifting at up to 1, back towards
//! home past 400 from it, turning about now and then), and within 200 of Link it comes at him at
//! 2 (`func_80ADEC9C`), keeping off his front; from 280 away it gives up. Within 140, with Link
//! not facing it (`Player_IsFacingActor`, 0x2AAA), it **attacks** (`EnPoh_Attack`: 12 swings of
//! the attack animation, turning to him, and from the third its lantern's sphere hits at 5 a frame
//! forward: 8 damage, half a heart). The lantern hitting Link (`AT_HIT`) sends it backing off,
//! spinning (`func_80ADF574`). It doesn't collide with the walls or the floor (no
//! `Actor_UpdateBgCheckInfo`): it floats through them.
//!
//! **Fading** (`EnPoh_UpdateVisibility`): it stays visible for 700 to 1000 frames
//! (`visibilityTimer`); then, or after 20 frames of Link's lock-on, it **disappears**
//! (`EnPoh_Disappear`, 32 frames, spinning) and stays invisible for 100 to 150 frames before it
//! **appears** again (`EnPoh_Appear`). Invisible, it can't be hit (`CollisionCheck_SetAC` only at
//! alpha 255) and isn't targetable, but it goes on moving and attacking.
//!
//! **Hit** (`func_80AE032C`, `sDamageTable`: 4 health; the Kokiri Sword 1, the jump slash 2, a
//! Deku Stick 2, a seed 1, a bomb 2, nuts nothing): knocked back at 5 (from an arrow or a seed the
//! way it flew, else away from what hit it), red for 16 frames, unhittable until its damage
//! animation ends (`func_80ADEECC`); then it **flees** (`func_80ADF894`, away from Link, swaying,
//! `NA_SE_EN_PO_AWAY`) until it's 250 away (its 200-frame timer is never counted down) or fades.
//! At 0 health it **burns** (`func_80ADF15C`: `Enemy_StartFinishingBlow`, three columns of blue
//! flames, `EffectSsDeadDb`, for 7 frames, the body gone into its burning shape
//! (`gPoeBurnDL`), rising and shrinking from frame 19); at frame 28 only its lantern is left
//! (`EnPoh_SetupDeath`: a `ACTORCAT_MISC` now, drawn by `EnPoh_DrawSoul`), which **falls**
//! (`EnPoh_Death`) and breaks on the floor (`EffectSsHahen_SpawnBurst`, 15 pieces) or is gone
//! after 60 frames. It drops nothing.
//!
//! **The soul** (`func_80ADE6D4`, `func_80ADFE28`): where the lantern broke, a flame rising and
//! growing (20 a frame) in its colours, then waiting 200 frames (`func_80ADFE80`), bobbing. Link
//! touching it (`OC1_HIT` of its small cylinder: 13 by 30) talks to it at once
//! (`ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED`): a Poe's 0x5005 asks to be bottled (`EnPoh_TalkRegular`):
//! yes with an empty bottle, 0x5008 and `ITEM_BOTTLE_POE` (`Item_Give`); yes without one, 0x5006;
//! no, 0x5007; it fades as the answer shows, and is gone. Its 200 frames out untouched, it
//! laughs and fades away (`func_80ADE950`, `func_80AE009C`). A composer's talk
//! (`EnPoh_TalkComposer`): the Sun's Song learnt, 0x5000 (closing it sets the switch 9: both
//! brothers gone for good); neither brother's switch (0xA Flat, 0xB Sharp), 0x500F; his own set,
//! 0x5013; else 0x5012. Each tale ends in 0x5011's choice ("hear it again?"): the first answer
//! tells it again (0x5010 with neither switch set, else 0x5014) and asks again; the second sets
//! his switch and he leaves.
//!
//! **Its light** (`lightNode`): a point light at its lantern, `envColor` (flickering by `Rand`,
//! `func_80AE089C`) at up to 200 radius (`Lights_PointGlowSetInfo` in `EnPoh_PostLimbDraw`); the
//! soul's, `Lights_PointNoGlowSetInfo` in its info's light colour at its alpha's 200.
//!
//! **Draws** (`EnPoh_DrawRegular`, `EnPoh_DrawComposer`, `EnPoh_DrawSoul`): the body opaque at
//! alpha 255 (and 0, drawing nothing), else translucent (`gActorSetupXluDL`), tinted by
//! `lightColor` (`func_80AE067C`: red while it attacks, cyan while it flees, flashing while hurt);
//! the lantern (`unk_368`, limb 18 or 9's matrix) always, in `envColor`; the soul a camera-facing
//! scrolling flame (`Gfx_TwoTexScroll`). Bakes (docs/adr/0012-actor-bakes.md): the bodies opaque
//! and translucent (with the lantern's limb nulled, and Flat's head), the lanterns and the souls.
//!
//! The whole overlay is ported. Not ported (the framework's): the light's glow
//! (`Lights_GlowCheck`, `drawGlow`) and the point lights' shading (`Lights_BindAll`'s lights
//! aren't drawn); a Poe's lantern's fragments (`EffectSsHahen` with `gPoeLanternDL`) spawn and
//! fall but draw nothing: the importer keeps a skeleton's limb lists only in the skeleton's mesh,
//! and `gPoeLanternDL` is limb 18's (a composer's, `gPoeComposerLanternDL`, isn't a limb's: they
//! draw); the circle shadow
//! (`ActorShadow_DrawCircle`, 30, `shadowAlpha`) is the shadow stand-in at full alpha.
//!
//! Params: `params` the whole params as an `s16` (0 `EN_POH_NORMAL`, the placed ones; 1
//! `EN_POH_RUPEE`, `Bg_Haka`'s; 2 `EN_POH_SHARP`, 3 `EN_POH_FLAT`, `En_Wonder_Talk`'s; 4 and up
//! read as 0).
//! Flags: Sharp reads and sets the switch 0x29 (at init; set: he's gone), Flat 0x28; both read the
//! switch 9 (set: both gone) and set it on the Sun's Song's text; they read 0xA (Flat's answer)
//! and 0xB (Sharp's), and set their own as they leave; they read the Sun's Song
//! (`QUEST_SONG_SUN`).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::{Cylinder16, Sphere16};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, scaled_step_to_s, sin_s, step_to_f, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_MISC, ActorImpl, ActorProfile, audio_play_actor_sfx2, cur_sfx_pos, enemy_start_finishing_blow};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_EN_EXTINCT, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::gbi::Dl;
use oot_game::lights::{LightInfo, LightNode};
use oot_game::message::TEXT_STATE_CHOICE;
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};

// `ITEM_BOTTLE_POE` (`item.h`: 0x20), `ITEM00_RUPEE_BLUE` (`z_en_item00.h`: 0x01).
use crate::en_item00::ITEM00_RUPEE_BLUE;
use oot_game::item::ITEM_BOTTLE_POE;

/// `ACTOR_EN_POH` (`actor_table.h`: 0x000D).
pub const ACTOR_EN_POH: i16 = 0x000D;

/// `this->actor.params` (an `s16`; 4 and up made `EN_POH_NORMAL` at init).
pub const PARAMS: Field = Field::whole(
    "params",
    Kind::Named(&[(0, "EN_POH_NORMAL: a Poe"), (1, "EN_POH_RUPEE: a Poe from a grave, with a blue rupee (Bg_Haka's)"), (2, "EN_POH_SHARP (En_Wonder_Talk's)"), (3, "EN_POH_FLAT (En_Wonder_Talk's)")]),
    "a Poe, a grave's Poe with a rupee, or Sharp or Flat",
);

/// En_Poh's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(Flag::Switch("0x28")).reads().sets().note("Flat, at init: set, he's gone; else set"),
        FlagUse::new(Flag::Switch("0x29")).reads().sets().note("Sharp, at init: set, he's gone; else set"),
        FlagUse::new(Flag::Switch("0x09")).reads().sets().note("the composers: set, both gone (at init); set as the Sun's Song's text 0x5000 closes"),
        FlagUse::new(Flag::Switch("0x0A")).reads().sets().note("Flat's answer: the texts by it; set as he leaves"),
        FlagUse::new(Flag::Switch("0x0B")).reads().sets().note("Sharp's answer: the texts by it; set as he leaves"),
    ],
    todo: false,
};

/// `EnPohType`.
pub const EN_POH_NORMAL: i16 = 0;
pub const EN_POH_RUPEE: i16 = 1;
pub const EN_POH_SHARP: i16 = 2;
pub const EN_POH_FLAT: i16 = 3;

/// `EnPohInfoType`.
pub const EN_POH_INFO_NORMAL: u8 = 0;
pub const EN_POH_INFO_COMPOSER: u8 = 1;

/// `OBJECT_POH`, `OBJECT_PO_COMPOSER` (`object_table.h`: 0x0009, 0x006E).
pub const OBJECT_POH: i16 = 0x0009;
pub const OBJECT_PO_COMPOSER: i16 = 0x006E;
pub const POH: &str = "object_poh";
pub const PO_COMPOSER: &str = "object_po_composer";

/// `NAVI_ENEMY_POE_COMPOSER`, `NAVI_ENEMY_POE` (`actor.h`: 0x43, 0x44).
pub const NAVI_ENEMY_POE_COMPOSER: u8 = 0x43;
pub const NAVI_ENEMY_POE: u8 = 0x44;

/// `ACTOR_FLAG_IGNORE_QUAKE` (`actor.h`: 1 << 12).
const ACTOR_FLAG_IGNORE_QUAKE: u32 = 1 << 12;

/// The sounds (`enemybank_table.h`, `environmentbank_table.h`).
pub const NA_SE_EN_PO_KANTERA: u16 = 0x3870;
pub const NA_SE_EN_PO_FLY: u16 = 0x3871;
pub const NA_SE_EN_PO_AWAY: u16 = 0x3872;
pub const NA_SE_EN_PO_APPEAR: u16 = 0x3873;
pub const NA_SE_EN_PO_DISAPPEAR: u16 = 0x3874;
pub const NA_SE_EN_PO_DAMAGE: u16 = 0x3875;
pub const NA_SE_EN_PO_DEAD: u16 = 0x3876;
pub const NA_SE_EN_PO_DEAD2: u16 = 0x3877;
pub const NA_SE_EN_PO_BIG_GET: u16 = 0x3883;
pub const NA_SE_EN_PO_LAUGH: u16 = 0x38EC;
pub const NA_SE_EN_PO_BIG_CRY: u16 = 0x39E8;
pub const NA_SE_EV_METAL_BOX_BOUND: u16 = 0x28E0;

/// `QUEST_SONG_SUN` (`item.h`).
const QUEST_SONG_SUN: u32 = 0x0F;

/// The texts (`message_data.h`): the Poe's soul's ask, answers; the composers'.
pub const TEXT_POE_SOUL: u16 = 0x5005;
pub const TEXT_POE_NO_BOTTLE: u16 = 0x5006;
pub const TEXT_POE_NO: u16 = 0x5007;
pub const TEXT_POE_BOTTLED: u16 = 0x5008;
pub const TEXT_COMPOSER_SUNS_SONG: u16 = 0x5000;
pub const TEXT_COMPOSER_FIRST: u16 = 0x500F;
pub const TEXT_COMPOSER_YES_FIRST: u16 = 0x5010;
pub const TEXT_COMPOSER_OTHER_ANSWERED: u16 = 0x5012;
pub const TEXT_COMPOSER_OWN_ANSWERED: u16 = 0x5013;
pub const TEXT_COMPOSER_YES_AGAIN: u16 = 0x5014;

/// `FLAGS`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_POH,
    name: "En_Poh",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_IGNORE_QUAKE,
    object: "gameplay_keep",
    schema: &SCHEMA,
};

/// `sCylinderInit`: the body, hurt by everything but the shield and the mirror's ray, 20 by 40.
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
    dim: Cylinder16 { radius: 20, height: 40, y_shift: 20, pos: [0, 0, 0] },
};

/// `sJntSphElementsInit`: the lantern, on limb 18 (`{0, 1400, 0}`, 10), hitting for 8.
fn jnt_sph_elements() -> [ColliderJntSphElementInit; 1] {
    [ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
            ac_elem_flags: ACELEM_NONE,
            oc_elem_flags: OCELEM_ON,
        },
        limb: 18,
        model_sphere: Sphere16 { center: [0, 1400, 0], radius: 10 },
        scale: 100,
    }]
}

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_HIT3, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sColChkInfoInit`: 4 health, 25 by 50, mass 40.
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

/// `EnPohInfo`: a Poe's or a composer's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnPohInfo {
    pub prim_color: [u8; 3],
    pub light_color: [u8; 3],
    /// `unk_6`: the lantern's limb (1-based), not drawn; the lantern is drawn at its matrix.
    pub unk_6: u8,
    /// `unk_7`: the limb the burning shape is drawn on (1-based).
    pub unk_7: u8,
    /// `unk_8`: the soul's scroll rate (an `s8`: 248 is -8).
    pub unk_8: i8,
    pub idle_anim: &'static str,
    pub idle_anim2: &'static str,
    pub damage_anim: &'static str,
    pub flee_anim: &'static str,
    pub lantern_display_list: &'static str,
    pub burn_display_list: &'static str,
    pub soul_display_list: &'static str,
    /// Not in the struct: the object the lists are in.
    pub object: &'static str,
}

/// `sPoeInfo`.
pub static S_POE_INFO: [EnPohInfo; 2] = [
    EnPohInfo {
        prim_color: [255, 170, 255],
        light_color: [100, 0, 150],
        unk_6: 18,
        unk_7: 5,
        unk_8: 248u8 as i8,
        idle_anim: "gPoeDisappearAnim",
        idle_anim2: "gPoeFloatAnim",
        damage_anim: "gPoeDamagedAnim",
        flee_anim: "gPoeFleeAnim",
        lantern_display_list: "gPoeLanternDL",
        burn_display_list: "gPoeBurnDL",
        soul_display_list: "gPoeSoulDL",
        object: POH,
    },
    EnPohInfo {
        prim_color: [255, 255, 170],
        light_color: [0, 150, 0],
        unk_6: 9,
        unk_7: 1,
        unk_8: 244u8 as i8,
        idle_anim: "gPoeComposerDisappearAnim",
        idle_anim2: "gPoeComposerFloatAnim",
        damage_anim: "gPoeComposerDamagedAnim",
        flee_anim: "gPoeComposerFleeAnim",
        lantern_display_list: "gPoeComposerLanternDL",
        burn_display_list: "gPoeComposerBurnDL",
        soul_display_list: "gPoeComposerSoulDL",
        object: PO_COMPOSER,
    },
];

/// `D_80AE1B4C` (Sharp's lantern top and robe, segment 0xA), `D_80AE1B50` (Flat's),
/// `D_80AE1B54` (Sharp's segment 0xB), `D_80AE1B58` (Flat's).
pub const D_80AE1B4C: [u8; 4] = [75, 20, 25, 255];
pub const D_80AE1B50: [u8; 4] = [80, 110, 90, 255];
pub const D_80AE1B54: [u8; 4] = [90, 85, 50, 255];
pub const D_80AE1B58: [u8; 4] = [100, 90, 100, 255];

/// `D_80AE1B60`, `D_80AE1B6C`: the flames' velocity and acceleration.
const D_80AE1B60: Vec3 = Vec3::new(0.0, 3.0, 0.0);
const D_80AE1B6C: Vec3 = Vec3::new(0.0, 0.0, 0.0);

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Func80ADEAC4,
    Idle,
    Func80ADEC9C,
    Attack,
    Func80ADEECC,
    Func80ADF894,
    ComposerAppear,
    Func80ADEF38,
    Func80ADF15C,
    Func80ADF574,
    Func80ADF5E0,
    Disappear,
    Appear,
    Death,
    Func80ADFE28,
    Func80ADFE80,
    Func80AE009C,
    TalkRegular,
    TalkComposer,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Func80ADEAC4 => "func_80ADEAC4",
            Action::Idle => "EnPoh_Idle",
            Action::Func80ADEC9C => "func_80ADEC9C",
            Action::Attack => "EnPoh_Attack",
            Action::Func80ADEECC => "func_80ADEECC",
            Action::Func80ADF894 => "func_80ADF894",
            Action::ComposerAppear => "EnPoh_ComposerAppear",
            Action::Func80ADEF38 => "func_80ADEF38",
            Action::Func80ADF15C => "func_80ADF15C",
            Action::Func80ADF574 => "func_80ADF574",
            Action::Func80ADF5E0 => "func_80ADF5E0",
            Action::Disappear => "EnPoh_Disappear",
            Action::Appear => "EnPoh_Appear",
            Action::Death => "EnPoh_Death",
            Action::Func80ADFE28 => "func_80ADFE28",
            Action::Func80ADFE80 => "func_80ADFE80",
            Action::Func80AE009C => "func_80AE009C",
            Action::TalkRegular => "EnPoh_TalkRegular",
            Action::TalkComposer => "EnPoh_TalkComposer",
        }
    }
}

/// `actor.update`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Update {
    /// `EnPoh_Update`: waiting for the object.
    Wait,
    /// `EnPoh_UpdateLiving`.
    Living,
    /// `EnPoh_UpdateDead`.
    Dead,
}

/// `actor.draw`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Draw {
    None,
    Regular,
    Composer,
    Soul,
}

/// The overlay's statics (`PlayState::overlay_static`, by `ACTOR_EN_POH`).
#[derive(Debug, Default)]
pub struct Statics {
    /// `D_80AE1A50`: the rupee Poes alive (counted at init, down at destroy).
    pub d_80ae1a50: i16,
}

// The bakes (docs/adr/0012-actor-bakes.md).

/// Segment 8 of a Poe's limbs (`ACTOR_SETUP_OPA_DL` or `gActorSetupXluDL`), and 0xC of a
/// composer's.
const SEG_SETUP_POE: u8 = 0x08;
const SEG_SETUP_COMPOSER: u8 = 0x0C;
/// The composers' segments 8, 0xA and 0xB: `Gfx_EnvColor`'s lists, the colours their limbs take.
const SEG_COMPOSER_LIGHT: u8 = 0x08;
const SEG_COMPOSER_A: u8 = 0x0A;
const SEG_COMPOSER_B: u8 = 0x0B;
/// The draw's own `gDPSetEnvColor` (and `gDPSetPrimColor`) before a list, as a dynamic colour run
/// before the body.
const SEG_ENV: u8 = 0x0E;
/// The soul's `Gfx_TwoTexScroll`.
const SEG_SCROLL: u8 = 0x08;

/// A body's bake: the Poe's (`None`) or a composer's (Sharp, `Some(false)`; Flat, `Some(true)`),
/// opaque or translucent.
pub fn body_bake(composer: Option<bool>, xlu: bool) -> String {
    let who = match composer {
        None => "poe",
        Some(false) => "sharp",
        Some(true) => "flat",
    };
    format!("En_Poh/{who}_{}", if xlu { "xlu" } else { "opa" })
}
/// `gPoeLanternDL` with its env colour.
pub const LANTERN_BAKE: &str = "En_Poh/lantern";
/// `gPoeComposerLanternDL` then `gPoeComposerLanternBottomDL`, with the env colour.
pub const COMPOSER_LANTERN_BAKE: &str = "En_Poh/composer_lantern";
/// `gPoeComposerLanternTopDL` with its env colour (the brother's).
pub const COMPOSER_LANTERN_TOP_BAKE: &str = "En_Poh/composer_lantern_top";
/// The souls (`gPoeSoulDL`, `gPoeComposerSoulDL`) with the scroll and their colours.
pub fn soul_bake(composer: bool) -> &'static str {
    if composer { "En_Poh/composer_soul" } else { "En_Poh/soul" }
}

/// `gActorSetupXluDL` (`z_actor.c`): `gsDPSetRenderMode(G_RM_FOG_SHADE_A, G_RM_AA_ZB_XLU_SURF2)`,
/// `gsDPSetAlphaCompare(G_AC_THRESHOLD)`, the end (as `En_Rd`'s).
fn actor_setup_xlu_dl() -> Vec<(u32, u32)> {
    let mut d = Dl::default();
    d.0.push((0xE200_001C, 0xC810_49F8));
    d.othermode_l(0, 2, 1);
    d.end();
    d.0
}

/// The segment-8 setup for a pass: `ACTOR_SETUP_OPA_DL` (just the end) or `gActorSetupXluDL`.
fn setup(xlu: bool) -> BakeSegment {
    BakeSegment::Commands(if xlu { actor_setup_xlu_dl() } else { vec![(0xDF00_0000, 0)] })
}

/// The soul's scroll at `visibilityTimer * unk_8` (`Gfx_TwoTexScroll(.., G_TX_RENDERTILE, 0, 0,
/// 0x20, 0x40, 1, 0, (visibilityTimer * unk_8) % 512U, 0x20, 0x80)`: the product, an `s32`,
/// taken as unsigned).
pub fn soul_scroll(visibility_timer: i16, unk_8: i8) -> Vec<(u32, u32)> {
    let y = ((visibility_timer as i32 * unk_8 as i32) as u32) % 512;
    gfx_two_tex_scroll(0, 0, 0, 0x20, 0x40, 1, 0, y, 0x20, 0x80)
}

pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for xlu in [false, true] {
        // EnPoh_OverrideLimbDraw: limb 18 (the lantern's, unk_6) not drawn.
        v.push(MeshBake {
            name: body_bake(None, xlu),
            object: POH.into(),
            segments: vec![(SEG_SETUP_POE, setup(xlu)), (SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false })],
            prelude: vec![SEG_ENV],
            body: BakeBody::Skeleton { file: POH.into(), symbol: "gPoeSkel".into(), limbs: vec![LimbOverride { limb: 17, file: POH.into(), symbol: String::new() }] },
        });
        for flat in [false, true] {
            // Limb 9 (the lantern's) not drawn; Flat's head (limb 10) his own.
            let mut limbs = vec![LimbOverride { limb: 8, file: PO_COMPOSER.into(), symbol: String::new() }];
            if flat {
                limbs.push(LimbOverride { limb: 9, file: PO_COMPOSER.into(), symbol: "gPoeComposerFlatHeadDL".into() });
            }
            v.push(MeshBake {
                name: body_bake(Some(flat), xlu),
                object: PO_COMPOSER.into(),
                segments: vec![
                    (SEG_COMPOSER_LIGHT, BakeSegment::DynamicColor { env: true, prim: false }),
                    (SEG_COMPOSER_A, BakeSegment::DynamicColor { env: true, prim: false }),
                    (SEG_COMPOSER_B, BakeSegment::DynamicColor { env: true, prim: false }),
                    (SEG_SETUP_COMPOSER, setup(xlu)),
                ],
                prelude: vec![],
                body: BakeBody::Skeleton { file: PO_COMPOSER.into(), symbol: "gPoeComposerSkel".into(), limbs },
            });
        }
    }
    let env = || vec![(SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false })];
    v.push(MeshBake { name: LANTERN_BAKE.into(), object: POH.into(), segments: env(), prelude: vec![SEG_ENV], body: BakeBody::DLists(vec![(POH.into(), "gPoeLanternDL".into())]) });
    v.push(MeshBake {
        name: COMPOSER_LANTERN_BAKE.into(),
        object: PO_COMPOSER.into(),
        segments: env(),
        prelude: vec![SEG_ENV],
        body: BakeBody::DLists(vec![(PO_COMPOSER.into(), "gPoeComposerLanternDL".into()), (PO_COMPOSER.into(), "gPoeComposerLanternBottomDL".into())]),
    });
    v.push(MeshBake { name: COMPOSER_LANTERN_TOP_BAKE.into(), object: PO_COMPOSER.into(), segments: env(), prelude: vec![SEG_ENV], body: BakeBody::DLists(vec![(PO_COMPOSER.into(), "gPoeComposerLanternTopDL".into())]) });
    for (composer, info) in [(false, &S_POE_INFO[0]), (true, &S_POE_INFO[1])] {
        v.push(MeshBake {
            name: soul_bake(composer).into(),
            object: info.object.into(),
            segments: vec![(SEG_SCROLL, BakeSegment::Dynamic(soul_scroll(0, info.unk_8))), (SEG_ENV, BakeSegment::DynamicColor { env: true, prim: true })],
            prelude: vec![SEG_ENV],
            body: BakeBody::DLists(vec![(info.object.into(), info.soul_display_list.into())]),
        });
    }
    v
}

/// The animations a type plays, loaded at init.
#[derive(Debug, Clone, Default)]
struct Anims {
    idle: Option<Anim>,
    idle2: Option<Anim>,
    damage: Option<Anim>,
    flee: Option<Anim>,
    attack: Option<Anim>,
    appear: Option<Anim>,
}

pub struct EnPoh {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Anims,
    pub action: Action,
    pub update_fn: Update,
    pub draw_fn: Draw,
    /// `unk_194`: the fade's counter (32 frames), and Link's lock-on's (to 20).
    pub unk_194: u8,
    /// `unk_195`: the bob's phase (32 to 1).
    pub unk_195: u8,
    pub required_object_slot: Option<usize>,
    pub info_idx: u8,
    pub unk_198: i16,
    pub visibility_timer: i16,
    pub unk_19c: i16,
    pub light_color: [u8; 4],
    pub env_color: [u8; 4],
    pub light_node: Option<LightNode>,
    /// `lightInfo`: what the node holds.
    pub light_info: LightInfo,
    pub collider_cylinder: ColliderCylinder,
    pub collider_jnt_sph: ColliderJntSph,
    /// `unk_368`: the lantern's matrix, from the last draw.
    pub unk_368: Mat4,
    /// `shape.shadowDraw` (`ActorShadow_DrawCircle`, scale 30), and `shape.shadowAlpha`.
    pub shadow: bool,
    pub shadow_alpha: u8,
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
/// `Animation_PlayOnceSetSpeed`.
fn play_once_set_speed(skel: &mut SkelAnimeStd, a: &Option<Anim>, speed: f32) {
    if let Some(a) = a.clone() {
        let last = a.last_frame();
        skel.change(a, speed, 0.0, last, ANIMMODE_ONCE, 0.0);
    }
}

/// An `f32` stored to a `u8` as the console does (truncated to an `s32`, its low byte).
fn to_u8(f: f32) -> u8 {
    f as i32 as u8
}
/// An `f32` passed as an `s16`.
fn to_s16(f: f32) -> i16 {
    f as i32 as i16
}

/// `Actor_Draw`'s translation with `ACTOR_FLAG_IGNORE_QUAKE` (as `En_Firefly`'s).
fn actor_model(rs: &RenderState, flags: u32, quake: Vec3) -> Mat4 {
    let mut m = actor_draw_matrix(rs);
    if flags & ACTOR_FLAG_IGNORE_QUAKE != 0 {
        m.w_axis += quake.extend(0.0);
    }
    m
}

impl EnPoh {
    /// `EnPoh_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: lockOnArrowOffset 3200.
        actor.target_arrow_offset = 3200.0;
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 30).
        actor.shape_y_offset = 0.0;
        let elements = jnt_sph_elements();
        let mut collider_jnt_sph = ColliderJntSph::new(&JNT_SPH_INIT, &elements);
        let p = actor.world_pos;
        collider_jnt_sph.elements[0].dim.world_sphere.radius = 0;
        collider_jnt_sph.elements[0].dim.world_sphere.center = [p.x as i16, p.y as i16, p.z as i16];
        let collider_cylinder = ColliderCylinder::new(&CYLINDER_INIT);
        actor.col_chk_info.set_info(Some(&S_DAMAGE_TABLE), &COL_CHK_INFO_INIT);
        let visibility_timer = play.rand.s16_offset(700, 300);
        let h = actor.home_pos;
        let light_info = LightInfo::point_glow(to_s16(h.x), to_s16(h.y), to_s16(h.z), [255, 255, 255], 0);
        let light_node = play.light_ctx.insert_light(light_info);
        if PARAMS.get(actor.params) >= 4 {
            actor.params = EN_POH_NORMAL;
        }
        let ty = PARAMS.get(actor.params);
        if ty == EN_POH_RUPEE {
            let st = play.overlay_static::<Statics>(ACTOR_EN_POH);
            st.d_80ae1a50 += 1;
            if st.d_80ae1a50 >= 3 {
                actor.kill();
            } else if let Some(c) = crate::en_item00::item_drop_collectible(play, actor.world_pos, 0x4000 | ITEM00_RUPEE_BLUE)
                && let Some(c) = play.actors.actor_mut(c)
            {
                c.speed_xz = 0.0;
            }
        } else if ty == EN_POH_FLAT {
            if play.flags.get_switch(0x28) || play.flags.get_switch(0x9) {
                actor.kill();
            } else {
                play.flags.set_switch(0x28);
            }
        } else if ty == EN_POH_SHARP {
            if play.flags.get_switch(0x29) || play.flags.get_switch(0x9) {
                actor.kill();
            } else {
                play.flags.set_switch(0x29);
            }
        }
        let (object, info_idx, navi) = if ty < EN_POH_SHARP { (OBJECT_POH, EN_POH_INFO_NORMAL, NAVI_ENEMY_POE) } else { (OBJECT_PO_COMPOSER, EN_POH_INFO_COMPOSER, NAVI_ENEMY_POE_COMPOSER) };
        let required_object_slot = play.object_ctx.get_index(object);
        actor.navi_enemy_id = navi;
        let info = &S_POE_INFO[info_idx as usize];
        let (mut skeleton, mut anims) = (None, Anims::default());
        if let Some(a) = play.assets.clone() {
            let file = info.object;
            let load = |n: &str| a.animation(file, n).map_err(|e| log::error!("En_Poh: {e:#}")).ok();
            anims = Anims {
                idle: load(info.idle_anim),
                idle2: load(info.idle_anim2),
                damage: load(info.damage_anim),
                flee: load(info.flee_anim),
                attack: load(if info_idx == EN_POH_INFO_NORMAL { "gPoeAttackAnim" } else { "gPoeComposerAttackAnim" }),
                appear: load(if info_idx == EN_POH_INFO_NORMAL { "gPoeAppearAnim" } else { "gPoeComposerAppearAnim" }),
            };
            let skel = if info_idx == EN_POH_INFO_NORMAL { "gPoeSkel" } else { "gPoeComposerSkel" };
            skeleton = a.skeleton(file, skel).map_err(|e| log::error!("En_Poh: {e:#}")).ok();
        }
        if required_object_slot.is_none() {
            actor.kill();
        }
        Box::new(EnPoh {
            actor,
            skel: SkelAnimeStd::init_flex(0, None),
            skeleton,
            anims,
            action: Action::Idle,
            update_fn: Update::Wait,
            draw_fn: Draw::None,
            unk_194: 0,
            unk_195: 32,
            required_object_slot,
            info_idx,
            unk_198: 0,
            visibility_timer,
            unk_19c: 0,
            light_color: [0; 4],
            env_color: [0; 4],
            light_node,
            light_info,
            collider_cylinder,
            collider_jnt_sph,
            unk_368: Mat4::IDENTITY,
            shadow: true,
            shadow_alpha: 255,
        })
    }

    pub fn info(&self) -> &'static EnPohInfo {
        &S_POE_INFO[self.info_idx as usize]
    }

    /// The actor's `lightInfo` written (`Lights_Point*SetInfo` on it).
    fn set_light(&mut self, play: &mut PlayState, info: LightInfo) {
        self.light_info = info;
        play.light_ctx.set_info(self.light_node, info);
    }

    fn player_pos_and_yaw(play: &PlayState) -> (Vec3, i16) {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| (a.world_pos, a.shape_rot.y)).unwrap_or_default()
    }

    /// `func_80ADE114`.
    fn func_80ade114(&mut self, play: &mut PlayState) {
        play_loop(&mut self.skel, &self.anims.idle);
        self.unk_198 = play.rand.s16_offset(2, 3);
        self.action = Action::Func80ADEAC4;
        self.actor.speed_xz = 0.0;
    }

    /// `EnPoh_SetupIdle`.
    fn setup_idle(&mut self, play: &mut PlayState) {
        play_loop(&mut self.skel, &self.anims.idle2);
        self.unk_198 = play.rand.s16_offset(15, 3);
        self.action = Action::Idle;
    }

    /// `func_80ADE1BC`: at Link at 2.
    fn func_80ade1bc(&mut self) {
        play_loop(&mut self.skel, &self.anims.idle2);
        self.action = Action::Func80ADEC9C;
        self.unk_198 = 0;
        self.actor.speed_xz = 2.0;
    }

    /// `EnPoh_SetupAttack`: 12 swings.
    fn setup_attack(&mut self, play: &mut PlayState) {
        if self.info_idx == EN_POH_INFO_NORMAL {
            morph_to_loop(&mut self.skel, &self.anims.attack, -6.0);
        } else {
            play_loop(&mut self.skel, &self.anims.attack);
        }
        self.unk_198 = 12;
        self.actor.speed_xz = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EN_PO_LAUGH);
        self.action = Action::Attack;
    }

    /// `func_80ADE28C`: hurt, knocked back at 5 (an arrow's or a seed's way, else away from what
    /// hit it), unhittable, red for 16 frames.
    fn func_80ade28c(&mut self, play: &mut PlayState) {
        if self.info_idx == EN_POH_INFO_NORMAL {
            morph_to_play_once(&mut self.skel, &self.anims.damage, -6.0);
        } else {
            play_once(&mut self.skel, &self.anims.damage);
        }
        let dmg_flags = self.collider_cylinder.info.ac_hit_elem.map(|e| e.at_dmg_info.dmg_flags).unwrap_or(0);
        if let Some(ac) = self.collider_cylinder.base.ac.and_then(|h| play.actors.actor(h)) {
            if dmg_flags & (DMG_ARROW | DMG_SLINGSHOT) != 0 {
                self.actor.world_rot.y = ac.world_rot.y;
            } else {
                // Actor_WorldYawTowardActor(&this->actor, colliderCylinder.base.ac) + 0x8000.
                self.actor.world_rot.y = vec3f_yaw(self.actor.world_pos, ac.world_pos).wrapping_add(i16::MIN);
            }
        }
        self.collider_cylinder.base.ac_flags &= !AC_ON;
        self.actor.speed_xz = 5.0;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 16);
        self.action = Action::Func80ADEECC;
    }

    /// `func_80ADE368`: fleeing at 5, hittable again.
    fn func_80ade368(&mut self) {
        morph_to_loop(&mut self.skel, &self.anims.flee, -5.0);
        self.actor.speed_xz = 5.0;
        self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(i16::MIN);
        self.collider_cylinder.base.ac_flags |= AC_ON;
        self.unk_198 = 200;
        self.action = Action::Func80ADF894;
    }

    /// `EnPoh_SetupInitialAction`: invisible, not targetable; a Poe's appear animation held, a
    /// composer's played (20 up, laughing).
    fn setup_initial_action(&mut self, play: &mut PlayState) {
        self.light_color[3] = 0;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        if self.info_idx == EN_POH_INFO_NORMAL {
            play_once_set_speed(&mut self.skel, &self.anims.appear, 0.0);
            self.action = Action::Func80ADEF38;
        } else {
            play_once_set_speed(&mut self.skel, &self.anims.appear, 1.0);
            self.actor.world_pos.y = self.actor.home_pos.y + 20.0;
            audio_play_actor_sfx2(play, NA_SE_EN_PO_LAUGH);
            audio_play_actor_sfx2(play, NA_SE_EN_PO_APPEAR);
            self.action = Action::ComposerAppear;
        }
    }

    /// `func_80ADE48C`: burning.
    fn func_80ade48c(&mut self) {
        self.actor.speed_xz = 0.0;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.unk_198 = 0;
        self.actor.navi_enemy_id = NAVI_ENEMY_NONE;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::Func80ADF15C;
    }

    /// `func_80ADE4C8`: backing off at 5, spinning.
    fn func_80ade4c8(&mut self) {
        play_once(&mut self.skel, &self.anims.idle2);
        self.action = Action::Func80ADF574;
        self.actor.speed_xz = -5.0;
    }

    /// `func_80ADE514`: turning round.
    fn func_80ade514(&mut self) {
        play_loop(&mut self.skel, &self.anims.idle);
        self.unk_19c = self.actor.world_rot.y.wrapping_add(i16::MIN);
        self.action = Action::Func80ADF5E0;
        self.actor.speed_xz = 0.0;
    }

    /// `EnPoh_SetupDisappear`.
    fn setup_disappear(&mut self, play: &mut PlayState) {
        self.unk_194 = 32;
        self.actor.speed_xz = 0.0;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        audio_play_actor_sfx2(play, NA_SE_EN_PO_DISAPPEAR);
        audio_play_actor_sfx2(play, NA_SE_EN_PO_LAUGH);
        self.action = Action::Disappear;
    }

    /// `EnPoh_SetupAppear`.
    fn setup_appear(&mut self, play: &mut PlayState) {
        self.unk_194 = 0;
        self.actor.speed_xz = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EN_PO_APPEAR);
        audio_play_actor_sfx2(play, NA_SE_EN_PO_LAUGH);
        self.action = Action::Appear;
    }

    /// `EnPoh_SetupDeath`: only the lantern left, falling (`EnPoh_UpdateDead`, `EnPoh_DrawSoul`).
    fn setup_death(&mut self, play: &mut PlayState) {
        self.update_fn = Update::Dead;
        self.draw_fn = Draw::Soul;
        self.shadow = false;
        self.actor.scale = Vec3::splat(0.01);
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.actor.gravity = -1.0;
        self.actor.shape_y_offset = 1500.0;
        self.actor.world_pos.y -= 15.0;
        if self.info_idx != EN_POH_INFO_COMPOSER {
            self.actor.shape_rot.x = i16::MIN;
        }
        // Actor_ChangeCategory(.., ACTORCAT_MISC).
        self.actor.category = ACTORCAT_MISC;
        if let Some(me) = play.cur_actor {
            play.actors.change_category(me, ACTORCAT_MISC);
        }
        self.unk_198 = 60;
        self.action = Action::Death;
    }

    /// `func_80ADE6D4`: the soul, unlit and nothing yet, where the lantern broke.
    fn func_80ade6d4(&mut self, play: &mut PlayState) {
        let p = self.actor.world_pos;
        self.set_light(play, LightInfo::point_no_glow(to_s16(p.x), to_s16(p.y), to_s16(p.z), [0, 0, 0], 0));
        self.visibility_timer = 0;
        self.actor.shape_rot.y = 0;
        self.light_color[0] = 0;
        self.light_color[3] = 0;
        self.actor.shape_y_offset = 0.0;
        self.actor.gravity = 0.0;
        self.actor.velocity.y = 0.0;
        if PARAMS.get(self.actor.params) >= EN_POH_SHARP {
            self.light_color[1] = 200;
            self.light_color[2] = 0;
        } else {
            self.light_color[1] = 0;
            self.light_color[2] = 200;
        }
        self.actor.scale.x = 0.0;
        self.actor.scale.y = 0.0;
        self.actor.shape_rot.x = 0;
        self.actor.home_pos.y = self.actor.world_pos.y;
        audio_play_actor_sfx2(play, NA_SE_EV_METAL_BOX_BOUND);
        self.action = Action::Func80ADFE28;
    }

    /// `EnPoh_Talk`: the soul's small cylinder (Link's only), its text, 200 frames.
    fn talk(&mut self, play: &mut PlayState) {
        self.actor.home_pos.y = self.actor.world_pos.y;
        self.actor.set_focus(-10.0);
        let p = self.actor.world_pos;
        self.collider_cylinder.dim.radius = 13;
        self.collider_cylinder.dim.height = 30;
        self.collider_cylinder.dim.y_shift = 0;
        self.collider_cylinder.dim.pos = [p.x as i16, (p.y - 20.0) as i16, p.z as i16];
        self.collider_cylinder.base.oc_flags1 = OC1_ON | OC1_TYPE_PLAYER;
        let ty = PARAMS.get(self.actor.params);
        if ty == EN_POH_FLAT || ty == EN_POH_SHARP {
            if play.save.check_quest_item(QUEST_SONG_SUN) {
                self.actor.text_id = TEXT_COMPOSER_SUNS_SONG;
            } else if !play.flags.get_switch(0xA) && !play.flags.get_switch(0xB) {
                self.actor.text_id = TEXT_COMPOSER_FIRST;
            } else if (ty == EN_POH_FLAT && play.flags.get_switch(0xA)) || (ty == EN_POH_SHARP && play.flags.get_switch(0xB)) {
                self.actor.text_id = TEXT_COMPOSER_OWN_ANSWERED;
            } else {
                self.actor.text_id = TEXT_COMPOSER_OTHER_ANSWERED;
            }
        } else {
            self.actor.text_id = TEXT_POE_SOUL;
        }
        self.unk_198 = 200;
        self.unk_195 = 32;
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::Func80ADFE80;
    }

    /// `func_80ADE950`: fading away (laughing, its cry stopped, if `arg1`).
    fn func_80ade950(&mut self, play: &mut PlayState, arg1: bool) {
        if arg1 {
            play.audio.stop_sfx_by_pos_and_id(cur_sfx_pos(play), NA_SE_EN_PO_BIG_CRY - SFX_FLAG);
            audio_play_actor_sfx2(play, NA_SE_EN_PO_LAUGH);
        }
        self.action = Action::Func80AE009C;
    }

    /// `func_80ADE998`.
    fn func_80ade998(&mut self) {
        self.action = Action::TalkRegular;
        self.actor.home_pos.y = self.actor.world_pos.y - 15.0;
    }

    /// `func_80ADE9BC`.
    fn func_80ade9bc(&mut self) {
        self.action = Action::TalkComposer;
    }

    /// `EnPoh_MoveTowardsPlayerHeight`: 1 a frame to Link's height, bobbing 2.5.
    fn move_towards_player_height(&mut self, play: &PlayState) {
        let (pp, _) = Self::player_pos_and_yaw(play);
        step_to_f(&mut self.actor.world_pos.y, pp.y, 1.0);
        self.actor.world_pos.y += 2.5 * sin_s((self.unk_195 as i32 * 0x800) as i16);
        if self.unk_195 != 0 {
            self.unk_195 -= 1;
        }
        if self.unk_195 == 0 {
            self.unk_195 = 32;
        }
    }

    /// `func_80ADEA5C`: past 400 from home, back towards it.
    fn func_80adea5c(&mut self) {
        let (w, h) = (self.actor.world_pos, self.actor.home_pos);
        let dist = ((h.x - w.x) * (h.x - w.x) + (h.z - w.z) * (h.z - w.z)).sqrt();
        if dist > 400.0 {
            self.unk_19c = vec3f_yaw(w, h);
        }
        scaled_step_to_s(&mut self.actor.world_rot.y, self.unk_19c, 0x71C);
    }

    /// `func_80ADEAC4`: still, a few loops.
    fn func_80adeac4(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(0.0) && self.unk_198 != 0 {
            self.unk_198 -= 1;
        }
        self.move_towards_player_height(play);
        if self.actor.xz_dist_to_player < 200.0 {
            self.func_80ade1bc();
        } else if self.unk_198 == 0 {
            self.setup_idle(play);
        }
        if self.light_color[3] == 255 {
            self.actor.play_sfx_flagged(NA_SE_EN_PO_FLY - SFX_FLAG);
        }
    }

    /// `EnPoh_Idle`: drifting at up to 1.
    fn idle(&mut self, play: &mut PlayState) {
        self.skel.update();
        step_to_f(&mut self.actor.speed_xz, 1.0, 0.2);
        if self.skel.on_frame(0.0) && self.unk_198 != 0 {
            self.unk_198 -= 1;
        }
        self.func_80adea5c();
        self.move_towards_player_height(play);
        if self.actor.xz_dist_to_player < 200.0 && self.unk_198 < 19 {
            self.func_80ade1bc();
        } else if self.unk_198 == 0 {
            if play.rand.zero_one() < 0.1 {
                self.func_80ade514();
            } else {
                self.func_80ade114(play);
            }
        }
        if self.light_color[3] == 255 {
            self.actor.play_sfx_flagged(NA_SE_EN_PO_FLY - SFX_FLAG);
        }
    }

    /// `func_80ADEC9C`: at Link, off his front (within 0x3000 of his facing's opposite); away past
    /// 280; within 140 and him not facing it, the attack.
    fn func_80adec9c(&mut self, play: &mut PlayState) {
        let (_, player_yaw) = Self::player_pos_and_yaw(play);
        self.skel.update();
        if self.unk_198 != 0 {
            self.unk_198 -= 1;
        }
        let facing_diff = self.actor.yaw_towards_player.wrapping_sub(player_yaw);
        if facing_diff >= 0x3001 {
            scaled_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player.wrapping_add(0x3000), 0x71C);
        } else if facing_diff < -0x3000 {
            scaled_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player.wrapping_sub(0x3000), 0x71C);
        } else {
            scaled_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 0x71C);
        }
        self.move_towards_player_height(play);
        if self.actor.xz_dist_to_player > 280.0 {
            self.setup_idle(play);
        } else if self.unk_198 == 0 && self.actor.xz_dist_to_player < 140.0 && !self.actor.player_is_facing(player_yaw, 0x2AAA) {
            self.setup_attack(play);
        }
        if self.light_color[3] == 255 {
            self.actor.play_sfx_flagged(NA_SE_EN_PO_FLY - SFX_FLAG);
        }
    }

    /// `EnPoh_Attack`: turning to Link for two swings, then at 5 at double speed; idle after 12.
    fn attack(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(0.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_PO_KANTERA);
            if self.unk_198 != 0 {
                self.unk_198 -= 1;
            }
        }
        self.move_towards_player_height(play);
        if self.unk_198 >= 10 {
            scaled_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 0xE38);
        } else if self.unk_198 == 9 {
            self.actor.speed_xz = 5.0;
            self.skel.play_speed = 2.0;
        } else if self.unk_198 == 0 {
            self.setup_idle(play);
            self.unk_198 = 23;
        }
    }

    /// `func_80ADEECC`: slowing; its damage animation done, fleeing (or burning at 0 health).
    fn func_80adeecc(&mut self) {
        step_to_f(&mut self.actor.speed_xz, 0.0, 0.5);
        if self.skel.update() {
            if self.actor.col_chk_info.health != 0 {
                self.func_80ade368();
            } else {
                self.func_80ade48c();
            }
        }
    }

    /// `func_80ADEF38`: a Poe's appearing: held until Link is within 280, fading in from frame 10.
    fn func_80adef38(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.light_color[3] = 255;
            self.visibility_timer = play.rand.s16_offset(700, 300);
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            self.setup_idle(play);
        } else if self.skel.cur_frame > 10.0 {
            self.light_color[3] = to_u8(((self.skel.cur_frame - 10.0) * 0.05) * 255.0);
        }
        if self.skel.play_speed < 0.5 && self.actor.xz_dist_to_player < 280.0 {
            audio_play_actor_sfx2(play, NA_SE_EN_PO_APPEAR);
            self.skel.play_speed = 1.0;
        }
    }

    /// `EnPoh_ComposerAppear`.
    fn composer_appear(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.light_color[3] = 255;
            self.visibility_timer = play.rand.s16_offset(700, 300);
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            self.setup_idle(play);
        } else {
            self.light_color[3] = ((self.skel.cur_frame * 25.5) as i32).min(255) as u8;
        }
    }

    /// `func_80ADF15C`: burning: three columns of flames for 7 frames (either side of it across
    /// the camera's view, and its middle), rising and shrinking from 19, the lantern left at 28.
    fn func_80adf15c(&mut self, play: &mut PlayState) {
        self.unk_198 += 1;
        let p = self.actor.world_pos;
        if self.unk_198 < 8 {
            let cam = play.cam_dir_yaw();
            let mut vec = Vec3::ZERO;
            if self.unk_198 < 5 {
                let a = ((self.unk_198 as i32 * 0x1000) - 0x4000) as i16;
                vec.y = sin_s(a) * 23.0 + (p.y + 40.0);
                let multiplier = cos_s(a) * 23.0;
                vec.x = sin_s(cam.wrapping_add(0x4800)) * multiplier + p.x;
                vec.z = cos_s(cam.wrapping_add(0x4800)) * multiplier + p.z;
            } else {
                vec.y = (p.y + 40.0) + (15.0 * (self.unk_198 - 5) as f32);
                vec.x = sin_s(cam.wrapping_add(0x4800)) * 23.0 + p.x;
                vec.z = cos_s(cam.wrapping_add(0x4800)) * 23.0 + p.z;
            }
            let scale = self.unk_198 * 10 + 80;
            let spawn = |play: &mut PlayState, v: Vec3| play.with_ss(|s| s.dead_db_spawn(v, D_80AE1B60, D_80AE1B6C, scale, 0, [255, 255, 255, 255], [0, 0, 255], 1, 9, 1));
            spawn(play, vec);
            vec.x = (p.x + p.x) - vec.x;
            vec.z = (p.z + p.z) - vec.z;
            spawn(play, vec);
            vec.x = p.x;
            vec.z = p.z;
            spawn(play, vec);
            if self.unk_198 == 1 {
                audio_play_actor_sfx2(play, NA_SE_EN_EXTINCT);
            }
        } else if self.unk_198 == 28 {
            self.setup_death(play);
        } else if self.unk_198 >= 19 {
            let new_scale = (28 - self.unk_198) as f32 * 0.001;
            self.actor.world_pos.y += 5.0;
            self.actor.scale = Vec3::splat(new_scale);
        }
        if self.unk_198 == 18 {
            audio_play_actor_sfx2(play, NA_SE_EN_PO_DEAD2);
        }
    }

    /// `func_80ADF574`: backing off, spinning 0x1000 a frame; idle once its animation ends.
    fn func_80adf574(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.actor.world_rot.y = self.actor.shape_rot.y;
            self.setup_idle(play);
            self.unk_198 = 23;
        } else {
            step_to_f(&mut self.actor.speed_xz, 0.0, 0.5);
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x1000);
        }
    }

    /// `func_80ADF5E0`: turning to `unk_19C`.
    fn func_80adf5e0(&mut self, play: &mut PlayState) {
        self.skel.update();
        if scaled_step_to_s(&mut self.actor.world_rot.y, self.unk_19c, 1820) {
            self.setup_idle(play);
        }
        if self.actor.xz_dist_to_player < 200.0 {
            self.func_80ade1bc();
        }
        self.move_towards_player_height(play);
    }

    /// `EnPoh_Disappear`: 32 frames' fade, spinning; invisible for 100 to 150 frames.
    fn disappear(&mut self, play: &mut PlayState) {
        if self.unk_194 != 0 {
            self.unk_194 -= 1;
        }
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(0x1000);
        self.move_towards_player_height(play);
        self.light_color[3] = to_u8(self.unk_194 as f32 * 7.96875);
        if self.unk_194 == 0 {
            self.visibility_timer = play.rand.s16_offset(100, 50);
            self.setup_idle(play);
        }
    }

    /// `EnPoh_Appear`: 32 frames' fade in, spinning back; visible for 700 to 1000 frames.
    fn appear(&mut self, play: &mut PlayState) {
        self.unk_194 = self.unk_194.wrapping_add(1);
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_sub(0x1000);
        self.move_towards_player_height(play);
        self.light_color[3] = to_u8(self.unk_194 as f32 * 7.96875);
        if self.unk_194 == 32 {
            self.visibility_timer = play.rand.s16_offset(700, 300);
            self.unk_194 = 0;
            self.setup_idle(play);
        }
    }

    /// `func_80ADF894`: fleeing, swaying, away from Link; idle once 250 away. Its `unk_198` (200)
    /// is never counted down.
    fn func_80adf894(&mut self, play: &mut PlayState) {
        self.skel.update();
        let multiplier = sin_s((self.unk_195 as i32 * 0x800) as i16) * 3.0;
        self.actor.world_pos.x -= multiplier * cos_s(self.actor.shape_rot.y);
        self.actor.world_pos.z += multiplier * sin_s(self.actor.shape_rot.y);
        scaled_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player.wrapping_add(i16::MIN), 0x71C);
        self.move_towards_player_height(play);
        if self.unk_198 == 0 || self.actor.xz_dist_to_player > 250.0 {
            self.actor.world_rot.y = self.actor.shape_rot.y;
            self.setup_idle(play);
        }
        self.actor.play_sfx_flagged(NA_SE_EN_PO_AWAY - SFX_FLAG);
    }

    /// `EnPoh_Death`: the lantern falling; on the floor it breaks and the soul comes; else gone
    /// after 60 frames.
    fn death(&mut self, play: &mut PlayState) {
        if self.unk_198 != 0 {
            self.unk_198 -= 1;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            let object_id = if self.info_idx == EN_POH_INFO_COMPOSER { OBJECT_PO_COMPOSER } else { OBJECT_POH };
            let info = self.info();
            let pos = self.actor.world_pos;
            // The fragments draw nothing: the lantern's list is a limb's (see the module docs).
            play.with_ss(|s| s.hahen_spawn_burst(pos, 6.0, 0, 1, 1, 15, object_id, 10, Some((info.object, info.lantern_display_list))));
            self.func_80ade6d4(play);
        } else if self.unk_198 == 0 {
            self.actor.kill();
            return;
        }
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 10.0, 10.0, 10.0, UPDBGCHECKINFO_FLAG_2);
    }

    /// `func_80ADFA90`: the soul's alpha by `arg1` (20 up, 13 down), its size (growing; or
    /// thinning and stretching as it fades) and light.
    fn func_80adfa90(&mut self, play: &mut PlayState, arg1: i32) {
        self.light_color[3] = (self.light_color[3] as i32 + arg1).clamp(0, 255) as u8;
        let multiplier;
        if arg1 < 0 {
            multiplier = self.light_color[3] as f32 * (1.0 / 255.0);
            self.actor.scale.x = 0.0056000002 * multiplier + 0.0014000001;
            self.actor.scale.z = self.actor.scale.x;
            self.actor.scale.y = (0.007 - 0.007 * multiplier) + 0.007;
        } else {
            multiplier = 1.0;
            self.actor.scale = Vec3::splat(self.light_color[3] as f32 * (0.007 / 255.0));
            self.actor.world_pos.y = self.actor.home_pos.y + (1.0 / 17.0) * self.light_color[3] as f32;
        }
        let lc = self.info().light_color;
        self.light_color[0] = to_u8(lc[0] as f32 * multiplier);
        self.light_color[1] = to_u8(lc[1] as f32 * multiplier);
        self.light_color[2] = to_u8(lc[2] as f32 * multiplier);
        let p = self.actor.world_pos;
        self.set_light(play, LightInfo::point_no_glow(to_s16(p.x), to_s16(p.y), to_s16(p.z), lc, to_s16(self.light_color[3] as f32 * (200.0 / 255.0))));
    }

    /// `func_80ADFE28`: the soul rising 2 a frame and growing; whole, it talks.
    fn func_80adfe28(&mut self, play: &mut PlayState) {
        self.actor.home_pos.y += 2.0;
        self.func_80adfa90(play, 20);
        if self.light_color[3] == 255 {
            self.talk(play);
        }
    }

    /// `func_80ADFE80`: the soul waiting 200 frames, bobbing; Link touching it talks at once; its
    /// time out, it laughs and goes.
    fn func_80adfe80(&mut self, play: &mut PlayState) {
        if self.unk_198 != 0 {
            self.unk_198 -= 1;
        }
        if oot_game::npc::process_talk_request(&mut self.actor) {
            if PARAMS.get(self.actor.params) >= EN_POH_SHARP {
                self.func_80ade9bc();
            } else {
                self.func_80ade998();
            }
            return;
        }
        if self.unk_198 == 0 {
            self.func_80ade950(play, true);
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            return;
        }
        if self.collider_cylinder.base.oc_flags1 & OC1_HIT != 0 {
            self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            let a = self.actor.clone();
            oot_game::npc::offer_talk_default(play, &a);
        } else {
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider_cylinder);
        }
        self.actor.world_pos.y = sin_s((self.unk_195 as i32 * 0x800) as i16) * 5.0 + self.actor.home_pos.y;
        if self.unk_195 != 0 {
            self.unk_195 -= 1;
        }
        if self.unk_195 == 0 {
            self.unk_195 = 32;
        }
        self.collider_cylinder.dim.pos[1] = (self.actor.world_pos.y - 20.0) as i16;
        self.actor.set_focus(-10.0);
        let p = self.actor.world_pos;
        let lc = self.info().light_color;
        self.set_light(play, LightInfo::point_no_glow(to_s16(p.x), to_s16(p.y), to_s16(p.z), lc, to_s16(self.light_color[3] as f32 * (200.0 / 255.0))));
    }

    /// `func_80AE009C`: fading 13 a frame, then gone.
    fn func_80ae009c(&mut self, play: &mut PlayState) {
        self.func_80adfa90(play, -13);
        if self.light_color[3] == 0 {
            self.actor.kill();
        }
    }

    /// `EnPoh_TalkRegular`: the Poe's ask (crying); the answer: bottled (an empty bottle), or not.
    fn talk_regular(&mut self, play: &mut PlayState) {
        if self.actor.text_id != TEXT_POE_SOUL {
            self.func_80adfa90(play, -13);
        } else {
            self.actor.play_sfx_flagged(NA_SE_EN_PO_BIG_CRY - SFX_FLAG);
        }
        if play.message_state() == TEXT_STATE_CHOICE {
            if play.message_should_advance() {
                play.audio.stop_sfx_by_pos_and_id(cur_sfx_pos(play), NA_SE_EN_PO_BIG_CRY - SFX_FLAG);
                if play.msg_ctx.choice_index == 0 {
                    if oot_game::item::inventory_has_empty_bottle(&play.save) {
                        self.actor.text_id = TEXT_POE_BOTTLED;
                        oot_game::item::item_give(&mut play.save, Some(&mut play.audio), ITEM_BOTTLE_POE);
                        audio_play_actor_sfx2(play, NA_SE_EN_PO_BIG_GET);
                    } else {
                        self.actor.text_id = TEXT_POE_NO_BOTTLE;
                        audio_play_actor_sfx2(play, NA_SE_EN_PO_LAUGH);
                    }
                } else {
                    self.actor.text_id = TEXT_POE_NO;
                    audio_play_actor_sfx2(play, NA_SE_EN_PO_LAUGH);
                }
                play.continue_textbox(self.actor.text_id);
            }
        } else if oot_game::npc::textbox_is_closing(play) {
            self.func_80ade950(play, false);
        }
    }

    /// `EnPoh_TalkComposer`: the brother's talk; the second answer sets his switch and he goes.
    fn talk_composer(&mut self, play: &mut PlayState) {
        self.actor.play_sfx_flagged(NA_SE_EN_PO_BIG_CRY - SFX_FLAG);
        if play.message_state() == TEXT_STATE_CHOICE {
            if play.message_should_advance() {
                if play.msg_ctx.choice_index == 0 {
                    if !play.flags.get_switch(0xB) && !play.flags.get_switch(0xA) {
                        self.actor.text_id = TEXT_COMPOSER_YES_FIRST;
                    } else {
                        self.actor.text_id = TEXT_COMPOSER_YES_AGAIN;
                    }
                    play.continue_textbox(self.actor.text_id);
                } else {
                    if PARAMS.get(self.actor.params) == EN_POH_SHARP {
                        play.flags.set_switch(0xB);
                    } else {
                        play.flags.set_switch(0xA);
                    }
                    self.func_80ade950(play, true);
                }
            }
        } else if oot_game::npc::textbox_is_closing(play) {
            if self.actor.text_id == TEXT_COMPOSER_SUNS_SONG {
                play.flags.set_switch(9);
            }
            self.func_80ade950(play, true);
        }
    }

    /// `func_80AE032C`: a hit with damage or a reaction: the finishing blow at 0 health
    /// (`NA_SE_EN_PO_DEAD`), else `NA_SE_EN_PO_DAMAGE`; hurt either way.
    fn func_80ae032c(&mut self, play: &mut PlayState) {
        if self.collider_cylinder.base.ac_flags & AC_HIT != 0 {
            self.collider_cylinder.base.ac_flags &= !AC_HIT;
            if self.actor.col_chk_info.damage_reaction != 0 || self.actor.col_chk_info.damage != 0 {
                if self.actor.apply_damage() == 0 {
                    enemy_start_finishing_blow(play, &self.actor);
                    audio_play_actor_sfx2(play, NA_SE_EN_PO_DEAD);
                } else {
                    audio_play_actor_sfx2(play, NA_SE_EN_PO_DAMAGE);
                }
                self.func_80ade28c(play);
            }
        }
    }

    /// `EnPoh_UpdateVisibility`: outside the fades, `visibilityTimer` down; visible, Link's
    /// lock-on counted (20 frames) or the timer out makes it disappear; invisible with the timer
    /// out, it appears.
    fn update_visibility(&mut self, play: &mut PlayState) {
        if matches!(self.action, Action::Appear | Action::Disappear | Action::Func80ADEF38 | Action::ComposerAppear) {
            return;
        }
        if self.visibility_timer != 0 {
            self.visibility_timer -= 1;
        }
        let moving = matches!(self.action, Action::Func80ADEAC4 | Action::Idle | Action::Func80ADEC9C | Action::Func80ADF5E0);
        if self.light_color[3] == 255 {
            if self.actor.is_targeted {
                self.unk_194 = self.unk_194.wrapping_add(1).min(20);
            } else {
                self.unk_194 = 0;
            }
            if (self.unk_194 == 20 || self.visibility_timer == 0) && (moving || self.action == Action::Func80ADF894) {
                self.setup_disappear(play);
            }
        } else if self.light_color[3] == 0 && self.visibility_timer == 0 && moving {
            self.setup_appear(play);
        }
    }

    /// `func_80AE067C` (from the draws): `lightColor` towards red while it attacks, towards cyan
    /// while it flees, flashing while hurt, else towards (255, 255, 210).
    fn func_80ae067c(&mut self) {
        let c = &mut self.light_color;
        match self.action {
            Action::Attack => {
                c[0] = (c[0] as i16 + 5).min(255) as u8;
                c[1] = (c[1] as i16 - 5).max(50) as u8;
                c[2] = (c[2] as i16 - 5).max(0) as u8;
            }
            Action::Func80ADF894 => {
                c[0] = (c[0] as i16 + 5).min(80) as u8;
                c[1] = (c[1] as i16 + 5).min(255) as u8;
                c[2] = (c[2] as i16 + 5).min(225) as u8;
            }
            Action::Func80ADEECC => {
                if self.actor.color_filter_timer & 2 != 0 {
                    c[0] = 0;
                    c[1] = 0;
                    c[2] = 0;
                } else {
                    c[0] = 80;
                    c[1] = 255;
                    c[2] = 225;
                }
            }
            _ => {
                c[0] = (c[0] as i16 + 5).min(255) as u8;
                c[1] = (c[1] as i16 + 5).min(255) as u8;
                if c[2] >= 211 {
                    c[2] = (c[2] as i16 - 5).max(210) as u8;
                } else {
                    c[2] = (c[2] as i16 + 5).min(210) as u8;
                }
            }
        }
    }

    /// `func_80AE089C`: `envColor`, the lantern's flame: brightening over the appear animation's
    /// first 12 frames, else flickering (`Rand_ZeroOne` every frame).
    fn func_80ae089c(&mut self, play: &mut PlayState) {
        if matches!(self.action, Action::Func80ADEF38 | Action::ComposerAppear) && self.skel.cur_frame < 12.0 {
            let v = ((self.skel.cur_frame * 16.66) as i16 + 55) as u8;
            self.env_color[0] = v;
            self.env_color[1] = v;
            self.env_color[2] = v;
            self.env_color[3] = to_u8(self.skel.cur_frame * (100.0 / 6.0));
        } else {
            let rand = play.rand.zero_one();
            self.env_color[0] = ((rand * 30.0) as i16 + 225) as u8;
            self.env_color[1] = ((rand * 100.0) as i16 + 155) as u8;
            self.env_color[2] = ((rand * 160.0) as i16 + 95) as u8;
            self.env_color[3] = 200;
        }
    }

    /// `EnPoh_Update`: its object in, the skeleton set up and the appearance begun.
    fn update_wait(&mut self, play: &mut PlayState) {
        let Some(slot) = self.required_object_slot else { return };
        if !play.object_ctx.is_loaded(slot) {
            return;
        }
        self.actor.obj_bank_index = Some(slot);
        self.update_fn = Update::Living;
        let limbs = self.skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(if self.info_idx == EN_POH_INFO_NORMAL { 20 } else { 11 });
        // SkelAnime_Init(&gPoeSkel, &gPoeFloatAnim, 21) or SkelAnime_InitFlex(&gPoeComposerSkel,
        // &gPoeComposerFloatAnim, 12): the float animation looped.
        self.skel = SkelAnimeStd::init_flex(limbs, self.anims.idle2.clone());
        if self.info_idx == EN_POH_INFO_NORMAL {
            self.draw_fn = Draw::Regular;
        } else {
            self.draw_fn = Draw::Composer;
            self.collider_jnt_sph.elements[0].dim.limb = 9;
            self.collider_jnt_sph.elements[0].dim.model_sphere.center[1] *= -1;
            self.actor.shape_rot.y = -0x4000;
            self.actor.world_rot.y = -0x4000;
            self.collider_cylinder.dim.radius = 20;
            self.collider_cylinder.dim.height = 55;
            self.collider_cylinder.dim.y_shift = 15;
        }
        self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.setup_initial_action(play);
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Func80ADEAC4 => self.func_80adeac4(play),
            Action::Idle => self.idle(play),
            Action::Func80ADEC9C => self.func_80adec9c(play),
            Action::Attack => self.attack(play),
            Action::Func80ADEECC => self.func_80adeecc(),
            Action::Func80ADF894 => self.func_80adf894(play),
            Action::ComposerAppear => self.composer_appear(play),
            Action::Func80ADEF38 => self.func_80adef38(play),
            Action::Func80ADF15C => self.func_80adf15c(play),
            Action::Func80ADF574 => self.func_80adf574(play),
            Action::Func80ADF5E0 => self.func_80adf5e0(play),
            Action::Disappear => self.disappear(play),
            Action::Appear => self.appear(play),
            Action::Death => self.death(play),
            Action::Func80ADFE28 => self.func_80adfe28(play),
            Action::Func80ADFE80 => self.func_80adfe80(play),
            Action::Func80AE009C => self.func_80ae009c(play),
            Action::TalkRegular => self.talk_regular(play),
            Action::TalkComposer => self.talk_composer(play),
        }
    }

    /// `EnPoh_UpdateLiving`.
    fn update_living(&mut self, play: &mut PlayState) {
        if self.collider_jnt_sph.base.at_flags & AT_HIT != 0 {
            self.collider_jnt_sph.base.at_flags &= !AT_HIT;
            self.func_80ade4c8();
        }
        self.func_80ae032c(play);
        self.update_visibility(play);
        self.run_action(play);
        self.actor.move_forward();
        if self.action == Action::Attack && self.unk_198 < 10 {
            self.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
            play.collision_check_set_at(&self.actor, 1, &mut self.collider_jnt_sph);
        }
        self.collider_cylinder.update(&self.actor);
        if self.collider_cylinder.base.ac_flags & AC_ON != 0 && self.light_color[3] == 255 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider_cylinder);
        }
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider_cylinder);
        play.collision_check_set_oc(&self.actor, 1, &mut self.collider_jnt_sph);
        self.actor.set_focus(42.0);
        if self.action != Action::Func80ADEECC && self.action != Action::Func80ADF574 {
            if self.action == Action::Func80ADF894 {
                self.actor.shape_rot.y = self.actor.world_rot.y.wrapping_add(i16::MIN);
            } else {
                self.actor.shape_rot.y = self.actor.world_rot.y;
            }
        }
        // BgCheck_EntityRaycastDown4 from 20 up.
        let (floor, poly) = play.col.entity_raycast_down(self.actor.world_pos + Vec3::Y * 20.0);
        self.actor.floor_height = floor;
        self.actor.floor_poly = poly;
        self.func_80ae089c(play);
        self.shadow_alpha = self.light_color[3];
    }

    /// `EnPoh_UpdateDead`.
    fn update_dead(&mut self, play: &mut PlayState) {
        self.run_action(play);
        if self.action != Action::Death {
            self.visibility_timer = self.visibility_timer.wrapping_add(1);
        }
        self.func_80ae089c(play);
    }

    /// Whether `EnPoh_OverrideLimbDraw` hides every limb: invisible, or burning from frame 2.
    fn body_hidden(&self) -> bool {
        self.light_color[3] == 0 || (self.action == Action::Func80ADF15C && self.unk_198 >= 2)
    }

    /// The lantern's matrix (`unk_368`, `EnPoh_PostLimbDraw` at limb `unk_6`): the limb's, kept at
    /// its size while the body shrinks (`0.01 / scale.x` from burning frame 19).
    fn lantern_matrix(&self, model: Mat4, bones: &[Mat4]) -> Mat4 {
        let mut m = model * bones.get(self.info().unk_6 as usize - 1).copied().unwrap_or(Mat4::IDENTITY);
        if self.action == Action::Func80ADF15C && self.unk_198 >= 19 && self.actor.scale.x != 0.0 {
            let s = 0.01 / self.actor.scale.x;
            m *= Mat4::from_scale(Vec3::splat(s));
        }
        m
    }
}

impl ActorImpl for EnPoh {
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
            Update::Wait => self.update_wait(play),
            Update::Living => self.update_living(play),
            Update::Dead => self.update_dead(play),
        }
    }

    /// The draws' changes to the actor: `func_80AE067C`'s colours (a body's draw), and
    /// `EnPoh_PostLimbDraw`'s: the lantern's sphere (`Collider_UpdateSpheres`), its matrix
    /// (`unk_368`; at burning frame 27 the actor moves to it), its light at the sphere in
    /// `envColor`; or `EnPoh_DrawSoul`'s light at the falling lantern.
    fn draw_update(&mut self, play: &mut PlayState) {
        match self.draw_fn {
            Draw::None => {}
            Draw::Regular | Draw::Composer => {
                self.func_80ae067c();
                let Some(skeleton) = self.skeleton.clone() else { return };
                let rs = RenderState::of(&self.actor);
                let model = actor_model(&rs, self.actor.flags, play.game_camera.quake_offset);
                let bones = skeleton.pose_override(&self.skel.joint_table, |_, _, _| Mat4::IDENTITY);
                for &l in &skeleton.draw_order {
                    let limb = l as u8 + 1;
                    let m = model * bones[l as usize];
                    self.collider_jnt_sph.update_spheres(limb, &m);
                    if limb == self.info().unk_6 {
                        self.unk_368 = self.lantern_matrix(model, &bones);
                        if self.action == Action::Func80ADF15C && self.unk_198 == 27 {
                            self.actor.world_pos = self.unk_368.w_axis.truncate();
                        }
                        let c = self.collider_jnt_sph.elements[0].dim.world_sphere.center;
                        let e = self.env_color;
                        self.set_light(play, LightInfo::point_glow(c[0], c[1], c[2], [e[0], e[1], e[2]], to_s16(e[3] as f32 * (200.0 / 255.0))));
                    }
                }
            }
            Draw::Soul => {
                if self.action == Action::Death {
                    let p = self.actor.world_pos;
                    let e = self.env_color;
                    self.set_light(play, LightInfo::point_glow(to_s16(p.x), to_s16(p.y), to_s16(p.z), [e[0], e[1], e[2]], 200));
                }
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs
    }

    /// `EnPoh_DrawRegular`, `EnPoh_DrawComposer`, `EnPoh_DrawSoul`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let model = actor_model(rs, self.actor.flags, play.game_camera.quake_offset);
        let info = self.info();
        let lc = self.light_color;
        let push = |out: &mut DrawOut, xlu: bool, cmd: DrawCmd| if xlu { out.xlu.push(cmd) } else { out.opa.push(cmd) };
        let env_params = |env: [u8; 4]| {
            let mut sv = SegmentValues::default();
            sv.env[SEG_ENV as usize] = Some(env);
            DrawParams { segments: Some(sv), ..Default::default() }
        };
        match self.draw_fn {
            Draw::None => {}
            Draw::Regular | Draw::Composer => {
                let Some(joints) = &rs.joints else { return };
                let Some(skeleton) = &self.skeleton else { return };
                let bones = skeleton.pose_override(&joints.rot, |_, _, _| Mat4::IDENTITY);
                let xlu = !(lc[3] == 255 || lc[3] == 0);
                let flat = PARAMS.get(self.actor.params) == EN_POH_FLAT;
                let (sp90, phi_t0) = if PARAMS.get(self.actor.params) == EN_POH_SHARP { (D_80AE1B4C, D_80AE1B54) } else { (D_80AE1B50, D_80AE1B58) };
                if !self.body_hidden() {
                    let mut sv = SegmentValues::default();
                    let mesh = if self.draw_fn == Draw::Regular {
                        // The opaque pass's env colour is lightColor; the translucent one's white
                        // at its alpha (limb 19's own lightColor reaches only limbs whose
                        // materials take its alpha, the same).
                        sv.env[SEG_ENV as usize] = Some(if xlu { [255, 255, 255, lc[3]] } else { lc });
                        body_bake(None, xlu)
                    } else {
                        sv.env[SEG_COMPOSER_LIGHT as usize] = Some(lc);
                        sv.env[SEG_COMPOSER_A as usize] = Some([sp90[0], sp90[1], sp90[2], lc[3]]);
                        sv.env[SEG_COMPOSER_B as usize] = Some([phi_t0[0], phi_t0[1], phi_t0[2], lc[3]]);
                        body_bake(Some(flat), xlu)
                    };
                    push(out, xlu, DrawCmd { mesh: MeshKey::named(keys::bake(&mesh)), transform: model, bones: bones.clone(), params: DrawParams { segments: Some(sv), ..Default::default() } });
                }
                // EnPoh_PostLimbDraw: burning from frame 2, its burning shape on limb unk_7.
                if self.action == Action::Func80ADF15C && self.unk_198 >= 2 {
                    let m = model * bones.get(info.unk_7 as usize - 1).copied().unwrap_or(Mat4::IDENTITY);
                    push(out, xlu, DrawCmd::new(MeshKey::named(keys::mesh(info.object, info.burn_display_list)), m));
                }
                // The lantern at unk_368, in envColor.
                let lantern = self.lantern_matrix(model, &bones);
                let env = [self.env_color[0], self.env_color[1], self.env_color[2], 255];
                if self.draw_fn == Draw::Regular {
                    out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(LANTERN_BAKE)), transform: lantern, bones: Vec::new(), params: env_params(env) });
                } else {
                    out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(COMPOSER_LANTERN_BAKE)), transform: lantern, bones: Vec::new(), params: env_params(env) });
                    out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(COMPOSER_LANTERN_TOP_BAKE)), transform: lantern, bones: Vec::new(), params: env_params(sp90) });
                }
                if self.shadow && self.shadow_alpha > 0 {
                    // ActorShadow_DrawCircle (shadowScale 30): the circle shadow's stand-in.
                    let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
                    let s = 30.0 * rs.scale.x * 100.0;
                    let m = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(s, 1.0, s));
                    out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), m));
                }
            }
            Draw::Soul => {
                if self.action == Action::Death {
                    let env = [self.env_color[0], self.env_color[1], self.env_color[2], 255];
                    if self.info_idx == EN_POH_INFO_COMPOSER {
                        let top = if PARAMS.get(self.actor.params) == EN_POH_SHARP { D_80AE1B4C } else { D_80AE1B50 };
                        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(COMPOSER_LANTERN_BAKE)), transform: model, bones: Vec::new(), params: env_params(env) });
                        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(COMPOSER_LANTERN_TOP_BAKE)), transform: model, bones: Vec::new(), params: env_params(top) });
                    } else {
                        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(LANTERN_BAKE)), transform: model, bones: Vec::new(), params: env_params(env) });
                    }
                } else {
                    let mut sv = SegmentValues::default();
                    sv.read(SEG_SCROLL, &soul_scroll(self.visibility_timer, info.unk_8));
                    sv.env[SEG_ENV as usize] = Some([lc[0], lc[1], lc[2], 255]);
                    sv.prim[SEG_ENV as usize] = Some([info.prim_color[0], info.prim_color[1], info.prim_color[2], lc[3]]);
                    // Matrix_RotateY((s16)(camDirYaw + 0x8000) * 9.58738e-05f, MTXMODE_APPLY).
                    let yaw = play.cam_dir_yaw().wrapping_add(i16::MIN) as f32 * 9.58738e-05;
                    let m = model * Mat4::from_rotation_y(yaw);
                    out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(soul_bake(self.info_idx == EN_POH_INFO_COMPOSER))), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
                }
            }
        }
    }

    /// `EnPoh_Destroy`: its light out; a rupee Poe uncounted.
    fn destroy(&mut self, play: &mut PlayState) {
        play.light_ctx.remove_light(self.light_node.take());
        if PARAMS.get(self.actor.params) == EN_POH_RUPEE {
            play.overlay_static::<Statics>(ACTOR_EN_POH).d_80ae1a50 -= 1;
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            0 => Some(ColliderMut::Cylinder(&mut self.collider_cylinder)),
            1 => Some(ColliderMut::JntSph(&mut self.collider_jnt_sph)),
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
