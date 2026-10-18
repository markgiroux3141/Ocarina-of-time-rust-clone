//! `Bg_Dy_Yoseizo` (`ovl_Bg_Dy_Yoseizo/z_bg_dy_yoseizo.c`): the Great Fairy, in her fountains.
//! The fountain decides what she is: `SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC` (`daiyousei_izumi`: the
//! upgrades, sitting) or the spell fountains (`yousei_izumi_yoko`: the spells, lying down); and
//! the reward is the spawn Link came in by (`this->reward = play->spawn`): in the magic fountain 0
//! magic (Death Mountain's), 1 double magic (the crater's), 2 double defence (Ganon's Castle's); in
//! a spell fountain 0 Farore's Wind (Zora's Fountain's), 1 Din's Fire (Hyrule Castle's), 2 Nayru's
//! Love (the Colossus's).
//!
//! This ROM is after NTSC 1.1 (the debug ROM): she starts on `BgDyYoseizo_WaitSong`.
//! - **The song** (`BgDyYoseizo_WaitSong`): switch 0x38, which the fountain's `En_Okarina_Tag`
//!   (0x1CB8) sets on Zelda's Lullaby: `OCARINA_MODE_04`; killed without magic unless she's the
//!   magic fountain's spawn 0; Link held (`PLAYER_CSACTION_1`).
//! - **The choice** (`BgDyYoseizo_SetupCutscene`): a new reward (the spell's `ITEMGETINF` clear;
//!   magic, double magic or double defence not acquired; `BREG(2)` 0) starts its cutscene herself
//!   (`gGreatFairyMagicCs`, `...DoubleMagicCs`, `...DoubleDefenseCs`, `...FaroresWindCs`,
//!   `...DinsFireCs`, `...NayrusLoveCs`: `play->csCtx.script`, `cutsceneTrigger` 1), unless on a
//!   cutscene layer (`IS_CUTSCENE_LAYER`, a debug feature), and waits for it; else the revisit:
//!   `lightSettingOverride` 2, one-point 8603 (8604 in a spell fountain), `NA_SE_EV_GREAT_FAIRY_APPEAR`,
//!   drawn.
//! - **The revisit**: she rises 40 above her home and grows to 0.035, spinning 0xBB8 a frame until
//!   she faces front (`Revisit_Appear`, `_FinishAppear`); text 0xDB (`_SetupTalk`, `_Talk`); closed,
//!   the HUD's hearts and magic (`HUD_VISIBILITY_A_HEARTS_MAGIC_FORCE`), her giving animation, the
//!   mouth open (`_SetupRefillPlayerEnergy`); at its end the after-animation, `En_Dy_Extra`'s beam
//!   200 above Link (type 0, 1 in a spell fountain), and 40 frames on the refill
//!   (`healthAccumulator` 0x140, `Magic_Fill`); full (or 200 frames on) the beam fades; text 0xDA
//!   (`_RefillPlayerEnergy`, `_WaitTalkEnd`), then the camera told it's done
//!   (`Camera_SetFinishedFlag`).
//! - **The disappear** (`BgDyYoseizo_SetupDisappear`, `_Disappear`, `_Kill`): her spin and shrink
//!   back down, `NA_SE_VO_FR_LAUGH_0`, `NA_SE_EV_GREAT_FAIRY_VANISH`; 30 frames after: Link let go
//!   (`PLAYER_CSACTION_7`), `lightSettingOverride` 0, the first `En_Okarina_Tag` killed, switch
//!   0x38 cleared, and she's gone.
//! - **The reward's cutscene**, by her cue channel 0: cue 2 shows her (`Reward_WaitCutscene`) and
//!   she appears as on a revisit (`Reward_Appear`); cue 3 her giving animation
//!   (`Reward_GiveReward`); cues 4 to 6 the light (her effects of types 1 to 3, homing on Link, in
//!   the magic fountain; a `Demo_Effect` light in the spell's colour in a spell fountain); cues 10
//!   to 12 the magic fountain's reward (`isMagicAcquired` and `magicFillTarget` 0x30; double magic
//!   `isDoubleMagicAcquired`, 0x60 and `magicLevel` 0; double defence `isDoubleDefenseAcquired`),
//!   the HUD's hearts and magic, once the health refill and (double defence only) `Magic_Fill`;
//!   cues 14 to 16 a spell fountain's (`En_Ex_Item` with `sSpellRewardsEnExItemParams` held over
//!   Link, magic acquired or filled, `healthAccumulator` 0x140, the `ITEMGETINF`, `Item_Give`);
//!   cue 17 the spell put away; cue 18 the defence hearts filled one a frame to 20; cues 19 to 21
//!   a `Door_Warp1` at Link (`WARP_ORANGE`, `_GREEN`, `_RED`), once; cue 13 the disappear.
//! - **Her sounds** in a cutscene by its frame (`NA_SE_VO_FR_SMILE_0` on 32, 291, 426 and 851 in
//!   the magic fountain, 35, 181, 462 and 795 in a spell fountain; `NA_SE_VO_FR_LAUGH_0` on 101 or
//!   90); her blink every 20 to 80 frames (not while she refills Link); her head and torso turned
//!   to Link (`Actor_TrackPlayer`, `OverrideLimbDraw`'s limbs 8 and 15); her bob
//!   (`BgDyYoseizo_UpdatePosY`: `velocity.y` the sine of her animation's phase, 0.4 of it in a
//!   cutscene).
//! - **Her effects** (`BgDyYoseizo_SpawnEffectsType`, `_SpawnEffect`, `_UpdateEffects`,
//!   `_DrawEffects`): 200 sparkles, two a frame while she's at least 0.01 big: type 0 drifting
//!   (90 frames), types 1 to 3 (the magic fountain's light, 50 frames) homing on 150 below and 50
//!   behind Link at 3 a frame each axis's way, `NA_SE_EV_HEALING` each frame each; their alphas up
//!   30 a frame while they live, down 30 after. Drawn as `gGreatFairyParticleMaterialDL` and
//!   `...ModelDL`, billboarded, by their colours (`sInnerColors`, `sOuterColors`), turned by `rot`
//!   (3000 a frame, in radians: `Matrix_RotateZ` takes the float as it is).
//!
//! Drawn (`BgDyYoseizo_Draw`, from the revisit's start or cue 2: the profile has none) as
//! `gGreatFairySkel` with her eye on segment 8 (`sEyeTextures`) and mouth on 10 (`sMouthTextures`):
//! one bake per eye and mouth (segment 9, `unusedEyeTexIndex`, isn't read by the model); then her
//! effects. Not in `BgDyYoseizo_Kill`.
//!
//! @bug (game): `Actor_SetFocus` puts the focus `scale × 7500` above her, then
//! `focus.pos.y = focusY` makes it that height above the world's origin instead.
//!
//! The whole overlay is ported. Not drawn: nothing.
//!
//! Params: none (placed 0; the reward is `play->spawn`).
//! Flags: reads and clears switch 0x38 (Zelda's Lullaby played: `En_Okarina_Tag` 0x1CB8's);
//! reads and sets ITEMGETINF_FARORES_WIND, ITEMGETINF_DINS_FIRE, ITEMGETINF_NAYRUS_LOVE (the
//! spells).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{approach_f, approach_s, approach_zero_f, atan2_s, sin_s, smooth_step_to_s, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Flag, FlagUse};
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::item::{ITEM_DINS_FIRE, ITEM_FARORES_WIND, ITEM_NAYRUS_LOVE};
use oot_game::item_get_inf;
use oot_game::message::{OCARINA_MODE_04, TEXT_STATE_EVENT};
use oot_game::npc::actor_track_player;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;
use oot_game::sys_matrix::MtxF;

use crate::boss_goma::ACTOR_DOOR_WARP1;
use crate::demo_effect::{ACTOR_DEMO_EFFECT, DEMO_EFFECT_LIGHT, DEMO_EFFECT_LIGHT_BLUE, DEMO_EFFECT_LIGHT_GREEN, DEMO_EFFECT_LIGHT_RED};
use crate::door_warp1::WARP_ORANGE;
use crate::en_dy_extra::{ACTOR_EN_DY_EXTRA, EnDyExtra};
use crate::en_ex_item::{ACTOR_EN_EX_ITEM, EXITEM_MAGIC_DARK, EXITEM_MAGIC_FIRE, EXITEM_MAGIC_WIND, EnExItem};
use crate::en_okarina_tag::ACTOR_EN_OKARINA_TAG;

/// `ACTOR_BG_DY_YOSEIZO` (`actor_table.h`: 0x000B).
pub const ACTOR_BG_DY_YOSEIZO: i16 = 0x000B;
/// `OBJECT_DY_OBJ` (`object_table.h`: 0x000A).
pub const OBJECT: &str = "object_dy_obj";

/// `Bg_Dy_Yoseizo_Profile`: `ACTORCAT_PROP`, `FLAGS` (this ROM's, `OOT_VERSION >= NTSC_1_1`:
/// `ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED |
/// ACTOR_FLAG_UPDATE_DURING_OCARINA`), no draw.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_BG_DY_YOSEIZO,
    name: "Bg_Dy_Yoseizo",
    category: ACTORCAT_PROP,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `ITEMGETINF_FARORES_WIND`, `_DINS_FIRE`, `_NAYRUS_LOVE` (`save.h`: 0x18, 0x19, 0x1A).
pub const ITEMGETINF_FARORES_WIND: u16 = 0x18;
pub const ITEMGETINF_DINS_FIRE: u16 = 0x19;
pub const ITEMGETINF_NAYRUS_LOVE: u16 = 0x1A;

/// Bg_Dy_Yoseizo's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[],
    flags: &[
        FlagUse::new(Flag::Switch("0x38")).reads().clears().note("Zelda's Lullaby played (En_Okarina_Tag 0x1CB8 sets it): she comes; cleared as she goes"),
        FlagUse::new(item_get_inf!(ITEMGETINF_FARORES_WIND)).reads().sets().note("a spell fountain's spawn 0: Farore's Wind given").indirect("set through sSpellRewardsItemGetInfMask"),
        FlagUse::new(item_get_inf!(ITEMGETINF_DINS_FIRE)).reads().sets().note("a spell fountain's spawn 1: Din's Fire given").indirect("set through sSpellRewardsItemGetInfMask"),
        FlagUse::new(item_get_inf!(ITEMGETINF_NAYRUS_LOVE)).reads().sets().note("a spell fountain's spawn 2: Nayru's Love given").indirect("set through sSpellRewardsItemGetInfMask"),
    ],
    todo: false,
};

/// `SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC` (`scene_table.h`: 0x3B).
pub const SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC: u16 = 0x3B;
/// `SCENE_GREAT_FAIRYS_FOUNTAIN_SPELLS` (`scene_table.h`: 0x3D).
pub const SCENE_GREAT_FAIRYS_FOUNTAIN_SPELLS: u16 = 0x3D;

/// `BG_DY_YOSEIZO_EFFECT_COUNT`.
pub const EFFECT_COUNT: usize = 200;

/// `BgDyYoseizoRewardSpell`.
pub const BG_DY_YOSEIZO_REWARD_SPELL_FARORES_WIND: i16 = 0;
pub const BG_DY_YOSEIZO_REWARD_SPELL_DINS_FIRE: i16 = 1;
pub const BG_DY_YOSEIZO_REWARD_SPELL_NAYRUS_LOVE: i16 = 2;
/// `BgDyYoseizoRewardMagic`.
pub const BG_DY_YOSEIZO_REWARD_MAGIC_MAGIC: i16 = 0;
pub const BG_DY_YOSEIZO_REWARD_MAGIC_DOUBLE_MAGIC: i16 = 1;
pub const BG_DY_YOSEIZO_REWARD_MAGIC_DOUBLE_DEFENSE: i16 = 2;

/// `PLAYER_CSACTION_1`, `PLAYER_CSACTION_7` (`player.h`).
const PLAYER_CSACTION_1: u8 = 1;
const PLAYER_CSACTION_7: u8 = 7;
/// `HUD_VISIBILITY_A_HEARTS_MAGIC_FORCE`, `HUD_VISIBILITY_HEARTS_MAGIC` (`save.h`: 5, 9).
pub const HUD_VISIBILITY_A_HEARTS_MAGIC_FORCE: u16 = 5;
pub const HUD_VISIBILITY_HEARTS_MAGIC: u16 = 9;
/// `BREG(2)` (`regs.h`): a debug register, 0.
const BREG_2: i16 = 0;
/// `DEBUG_FEATURES`: this ROM is the debug ROM.
const DEBUG_FEATURES: bool = true;

/// `NA_SE_EV_HEALING` (`environmentbank_table.h`: 0x287E).
pub const NA_SE_EV_HEALING: u16 = 0x287E;
/// `NA_SE_EV_GREAT_FAIRY_APPEAR` (`environmentbank_table.h`: 0x287F).
pub const NA_SE_EV_GREAT_FAIRY_APPEAR: u16 = 0x287F;
/// `NA_SE_EV_GREAT_FAIRY_VANISH` (`environmentbank_table.h`: 0x2880).
pub const NA_SE_EV_GREAT_FAIRY_VANISH: u16 = 0x2880;
/// `NA_SE_VO_FR_LAUGH_0` (`voicebank_table.h`: 0x6858).
pub const NA_SE_VO_FR_LAUGH_0: u16 = 0x6858;
/// `NA_SE_VO_FR_SMILE_0` (`voicebank_table.h`: 0x6859).
pub const NA_SE_VO_FR_SMILE_0: u16 = 0x6859;

/// The reward cutscenes (`object_dy_obj`'s scenes: `daiyousei_izumi_scene.h`,
/// `yousei_izumi_yoko_scene.h`), by reward: the magic fountain's, then the spell fountains'.
pub const MAGIC_CUTSCENES: [&str; 3] = ["gGreatFairyMagicCs", "gGreatFairyDoubleMagicCs", "gGreatFairyDoubleDefenseCs"];
pub const SPELL_CUTSCENES: [&str; 3] = ["gGreatFairyFaroresWindCs", "gGreatFairyDinsFireCs", "gGreatFairyNayrusLoveCs"];

/// `sInnerColors`, `sOuterColors`: each effect type's (the table's 4 to 8 are never used).
const INNER_COLORS: [[u8; 3]; 9] = [[255, 255, 255], [255, 255, 100], [100, 255, 100], [255, 100, 100], [255, 255, 170], [255, 255, 100], [100, 255, 100], [255, 100, 100], [255, 255, 170]];
const OUTER_COLORS: [[u8; 3]; 9] = [[155, 255, 255], [255, 255, 100], [100, 255, 100], [255, 100, 100], [255, 100, 255], [255, 255, 100], [100, 255, 100], [255, 100, 100], [100, 255, 255]];

/// `sSpellRewardsDemoEffectLightColor`.
const SPELL_REWARDS_DEMO_EFFECT_LIGHT_COLOR: [u8; 3] = [DEMO_EFFECT_LIGHT_GREEN, DEMO_EFFECT_LIGHT_RED, DEMO_EFFECT_LIGHT_BLUE];
/// `sSpellRewardsEnExItemParams`.
pub const SPELL_REWARDS_EN_EX_ITEM_PARAMS: [i16; 3] = [EXITEM_MAGIC_WIND, EXITEM_MAGIC_FIRE, EXITEM_MAGIC_DARK];
/// `sSpellRewardsItemGetInfMask`'s flags (all in `ITEMGETINF_INDEX_SPELLS`, 1).
const SPELL_REWARDS_ITEM_GET_INF: [u16; 3] = [ITEMGETINF_FARORES_WIND, ITEMGETINF_DINS_FIRE, ITEMGETINF_NAYRUS_LOVE];
/// `sSpellRewardsItem`.
const SPELL_REWARDS_ITEM: [u8; 3] = [ITEM_FARORES_WIND, ITEM_DINS_FIRE, ITEM_NAYRUS_LOVE];

/// `sEyeTextures`, `sMouthTextures`.
pub const EYE_TEXTURES: [&str; 3] = ["gGreatFairyEyeOpenTex", "gGreatFairyEyeHalfTex", "gGreatFairyEyeClosedTex"];
pub const MOUTH_TEXTURES: [&str; 2] = ["gGreatFairyMouthClosedTex", "gGreatFairyMouthOpenTex"];

/// The animations, as `anim` indexes them.
pub const ANIMS: [&str; 10] = [
    "gGreatFairySittingTransitionAnim",
    "gGreatFairyLayingDownTransitionAnim",
    "gGreatFairySittingAnim",
    "gGreatFairyLayingSidewaysAnim",
    "gGreatFairyGivingUpgradeAnim",
    "gGreatFairyGivingSpellAnim",
    "gGreatFairyAfterUpgradeAnim",
    "gGreatFairyAfterSpellAnim",
    "gGreatFairyJewelFountainSpinShrinkAnim",
    "gGreatFairySpellFountainSpinShrinkAnim",
];
/// Each pair's magic fountain's in `ANIMS` (the spell fountain's the next).
const ANIM_TRANSITION: usize = 0;
const ANIM_IDLE: usize = 2;
const ANIM_GIVING: usize = 4;
const ANIM_AFTER: usize = 6;
const ANIM_SPIN_SHRINK: usize = 8;

/// `OverrideLimbDraw`'s limbs (1-based): the torso and the head.
const LIMB_TORSO: usize = 8;
const LIMB_HEAD: usize = 15;

/// The skeleton's bake with eye `eye` and mouth `mouth`.
pub fn bake_name(eye: usize, mouth: usize) -> String {
    format!("Bg_Dy_Yoseizo/eye{eye}_mouth{mouth}")
}
/// The effects' bake (`gGreatFairyParticleMaterialDL`, the colours, `gGreatFairyParticleModelDL`).
pub const PARTICLE_BAKE: &str = "Bg_Dy_Yoseizo/particle";

const SEG_EYES: u8 = 0x08;
const SEG_MOUTH: u8 = 0x0A;
const SEG_PARTICLE_COLOR: u8 = 0x0B;

/// The meshes `BgDyYoseizo_Draw` draws: the skeleton (`Gfx_SetupDL_25Opa`) with each eye and
/// mouth, and an effect (`Gfx_SetupDL_25Xlu`, the material, its colours, the model).
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for (eye, eye_tex) in EYE_TEXTURES.iter().enumerate() {
        for (mouth, mouth_tex) in MOUTH_TEXTURES.iter().enumerate() {
            v.push(MeshBake {
                name: bake_name(eye, mouth),
                object: OBJECT.into(),
                segments: vec![
                    (SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: (*eye_tex).into() }),
                    (SEG_MOUTH, BakeSegment::Texture { file: OBJECT.into(), symbol: (*mouth_tex).into() }),
                ],
                prelude: Vec::new(),
                body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gGreatFairySkel".into(), limbs: Vec::new() },
            });
        }
    }
    v.push(MeshBake {
        name: PARTICLE_BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_PARTICLE_COLOR, BakeSegment::DynamicColor { env: true, prim: true })],
        prelude: vec![SEG_PARTICLE_COLOR],
        body: BakeBody::DLists(vec![(OBJECT.into(), "gGreatFairyParticleMaterialDL".into()), (OBJECT.into(), "gGreatFairyParticleModelDL".into())]),
    });
    v
}

/// `BgDyYoseizoEffect`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BgDyYoseizoEffect {
    pub active: u8,
    pub pos: Vec3,
    /// For type 0.
    pub vel: Vec3,
    pub accel: Vec3,
    pub inner_color: [u8; 3],
    pub outer_color: [u8; 3],
    pub alpha: i16,
    pub scale: f32,
    pub life: i16,
    pub ty: i16,
    /// For the other types: binary angles kept as floats.
    pub pitch: f32,
    pub yaw: f32,
    pub rot: f32,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitSong,
    SetupCutscene,
    RevisitSetupAppear,
    RevisitAppear,
    RevisitFinishAppear,
    RevisitSetupTalk,
    RevisitTalk,
    RevisitSetupRefillPlayerEnergy,
    RevisitRefillPlayerEnergy,
    RevisitWaitTalkEnd,
    SetupDisappear,
    Disappear,
    Kill,
    RewardWaitCutscene,
    RewardAppear,
    RewardGiveReward,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::WaitSong => "BgDyYoseizo_WaitSong",
            Action::SetupCutscene => "BgDyYoseizo_SetupCutscene",
            Action::RevisitSetupAppear => "BgDyYoseizo_Revisit_SetupAppear",
            Action::RevisitAppear => "BgDyYoseizo_Revisit_Appear",
            Action::RevisitFinishAppear => "BgDyYoseizo_Revisit_FinishAppear",
            Action::RevisitSetupTalk => "BgDyYoseizo_Revisit_SetupTalk",
            Action::RevisitTalk => "BgDyYoseizo_Revisit_Talk",
            Action::RevisitSetupRefillPlayerEnergy => "BgDyYoseizo_Revisit_SetupRefillPlayerEnergy",
            Action::RevisitRefillPlayerEnergy => "BgDyYoseizo_Revisit_RefillPlayerEnergy",
            Action::RevisitWaitTalkEnd => "BgDyYoseizo_Revisit_WaitTalkEnd",
            Action::SetupDisappear => "BgDyYoseizo_SetupDisappear",
            Action::Disappear => "BgDyYoseizo_Disappear",
            Action::Kill => "BgDyYoseizo_Kill",
            Action::RewardWaitCutscene => "BgDyYoseizo_Reward_WaitCutscene",
            Action::RewardAppear => "BgDyYoseizo_Reward_Appear",
            Action::RewardGiveReward => "BgDyYoseizo_Reward_GiveReward",
        }
    }
}

pub struct BgDyYoseizo {
    pub actor: Actor,
    pub action: Action,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    anims: Vec<Anim>,
    /// `actor.draw`: `BgDyYoseizo_Draw` once set (the profile has none).
    pub drawn: bool,
    pub has_spawned_demo_effect: bool,
    pub give_double_defense: bool,
    pub energy_refilled: bool,
    pub disappear_timer: i16,
    pub is_new_acquire_magic: bool,
    /// `reward`: `play->spawn`.
    pub reward: i16,
    pub wait_message_state: u8,
    /// `unk_2F0`: counted up every update, read by nothing.
    pub unk_2f0: i16,
    pub eye_tex_index: i16,
    pub unused_eye_tex_index: i16,
    pub mouth_tex_index: i16,
    pub eye_timer: i16,
    /// `unk_2FA`: counted down, never set.
    pub unk_2fa: i16,
    pub has_switched_anim: bool,
    pub is_done_appearing: bool,
    pub has_spawned_item: bool,
    pub refill_timer: i16,
    /// `unk304`: the warp spawned.
    pub unk304: i16,
    pub refill_timeout_timer: i16,
    pub scale: f32,
    pub above_fountain_y: f32,
    pub home_pos_y: f32,
    pub y_approach_fraction: f32,
    pub scale_approach_fraction: f32,
    pub target_pos_y: f32,
    pub offset_y: f32,
    pub y_velocity_phase: f32,
    pub focus_y: f32,
    pub cur_anim_last_frame: f32,
    pub head_rot: [i16; 3],
    pub torso_rot: [i16; 3],
    /// `refillBeam`: the `En_Dy_Extra` she spawned.
    pub refill_beam: Option<ActorHandle>,
    /// `item`: the `En_Ex_Item` she spawned.
    pub item: Option<ActorHandle>,
    pub effects: Vec<BgDyYoseizoEffect>,
}

/// `Math_Vec3f_Pitch`.
fn vec3f_pitch(a: Vec3, b: Vec3) -> i16 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    atan2_s((dx * dx + dz * dz).sqrt(), a.y - b.y)
}

/// `BINANG_TO_RAD_ALT`: `(f32)x * (M_PI / 0x8000)` for a float angle.
fn binang_to_rad_alt(x: f32) -> f32 {
    x * (std::f32::consts::PI / 32768.0)
}

/// `TRUNCF_BINANG`: `(s16)(s32)f`.
fn truncf_binang(f: f32) -> i16 {
    f as i32 as i16
}

impl BgDyYoseizo {
    /// `BgDyYoseizo_Init`: her reward is the spawn (0 at least); her home, 40 above it; the
    /// skeleton with the magic fountain's sitting transition (else the lying down one);
    /// `BgDyYoseizo_WaitSong`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.skeleton(OBJECT, "gGreatFairySkel")?;
            let anims = ANIMS.iter().map(|n| assets.animation(OBJECT, n)).collect::<anyhow::Result<Vec<_>>>()?;
            Ok((skeleton, anims))
        })();
        let (skeleton, anims) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("Bg_Dy_Yoseizo: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let magic = play.scene_id == SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC;
        let first = anims[ANIM_TRANSITION + if magic { 0 } else { 1 }].clone();
        // this->reward = play->spawn; (play->spawn < 0: 0; the port's spawn index can't be.)
        let reward = play.cur_spawn as i16;
        let reward = if reward < 0 { 0 } else { reward };
        if magic {
            log::debug!("☆☆☆☆☆ Great Fairy's Fountain ☆☆☆☆☆ {}", play.cur_spawn);
        } else {
            log::debug!("☆☆☆☆☆ Stone fairy's fountain ☆☆☆☆☆ {}", play.cur_spawn);
        }
        let mut this = BgDyYoseizo {
            home_pos_y: actor.world_pos.y,
            above_fountain_y: actor.world_pos.y + 40.0,
            actor,
            action: Action::WaitSong,
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(first)),
            skeleton,
            anims,
            drawn: false,
            has_spawned_demo_effect: false,
            give_double_defense: false,
            energy_refilled: false,
            disappear_timer: 0,
            is_new_acquire_magic: false,
            reward,
            wait_message_state: 0,
            unk_2f0: 0,
            eye_tex_index: 0,
            unused_eye_tex_index: 0,
            mouth_tex_index: 0,
            eye_timer: 0,
            unk_2fa: 0,
            has_switched_anim: false,
            is_done_appearing: false,
            has_spawned_item: false,
            refill_timer: 0,
            unk304: 0,
            refill_timeout_timer: 0,
            scale: 0.0,
            target_pos_y: 0.0,
            y_approach_fraction: 0.0,
            scale_approach_fraction: 0.0,
            offset_y: 0.0,
            y_velocity_phase: 0.0,
            focus_y: 0.0,
            cur_anim_last_frame: 0.0,
            head_rot: [0; 3],
            torso_rot: [0; 3],
            refill_beam: None,
            item: None,
            effects: vec![BgDyYoseizoEffect::default(); EFFECT_COUNT],
        };
        this.actor.focus_pos = this.actor.world_pos;
        Box::new(this)
    }

    fn in_magic_fountain(play: &PlayState) -> bool {
        play.scene_id == SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC
    }

    /// The fountain's animation of the pair at `base` (`ANIM_*`: the magic fountain's, else the
    /// spell fountain's).
    fn anim(&self, play: &PlayState, base: usize) -> Anim {
        self.anims[base + if Self::in_magic_fountain(play) { 0 } else { 1 }].clone()
    }

    /// `curAnimLastFrame = Animation_GetLastFrame(anim); Animation_Change(.., 1.0f, 0.0f,
    /// curAnimLastFrame, mode, -10.0f)`.
    fn change_anim(&mut self, play: &PlayState, base: usize, mode: u8) {
        let a = self.anim(play, base);
        self.cur_anim_last_frame = a.last_frame();
        self.skel.change(a, 1.0, 0.0, self.cur_anim_last_frame, mode, -10.0);
    }

    /// The cue on her channel (`play->csCtx.actorCues[0]`).
    fn cue(play: &PlayState) -> Option<CsCmdActorCue> {
        play.cs_ctx.npc_actions[0]
    }

    fn player_pos(play: &PlayState) -> Vec3 {
        play.player.and_then(|h| play.actors.actor(h)).map(|p| p.world_pos).unwrap_or_default()
    }

    fn set_cs_action(play: &mut PlayState, mode: u8) {
        let me = play.cur_actor;
        play.player_set_cs_action_with_halted_actors(me, mode);
    }

    /// `BgDyYoseizo_SpawnEffectsType`: two sparkles while she's at least 0.01 big: type 0 at her
    /// front, its height in her upper half by a random; another type over her head (the magic
    /// fountain's 50 above her shape, else 30 below and 60 out), 20 higher for an adult.
    pub fn spawn_effects_type(&mut self, play: &mut PlayState, eff_type: i16) {
        let vel = Vec3::ZERO;
        if self.scale < 0.01 {
            return;
        }
        let y_offset = self.scale * 3500.0;
        let accel = Vec3::new(play.rand.zero_one() - 0.5, play.rand.zero_one() - 0.5, play.rand.zero_one() - 0.5);
        for _ in 0..2 {
            let p = self.actor.world_pos;
            let mut pos = Vec3::ZERO;
            let (ty, scale, life): (i16, f32, i16);
            if eff_type == 0 {
                ty = 0;
                scale = 0.4;
                life = 90;
                pos = Vec3::new(p.x, p.y + y_offset + (y_offset * 0.5 * (play.rand.zero_one() - 0.5)), p.z + 30.0);
            } else {
                scale = 0.2;
                life = 50;
                ty = eff_type;
                pos.x = play.rand.centered_float(10.0) + p.x;
                if Self::in_magic_fountain(play) {
                    pos.y = p.y + y_offset + 50.0 + (y_offset * 0.1 * (play.rand.zero_one() - 0.5));
                    pos.z = p.z + 30.0;
                } else {
                    pos.y = p.y + y_offset - 30.0 + (y_offset * 0.1 * (play.rand.zero_one() - 0.5));
                    pos.z = p.z + 60.0;
                }
                if play.save.adult {
                    pos.y += 20.0;
                }
            }
            let t = ty as usize;
            self.spawn_effect(play, pos, vel, accel, INNER_COLORS[t], OUTER_COLORS[t], scale, life, ty);
        }
    }

    /// `BgDyYoseizo_SpawnEffect`: in the first free slot, its alpha 0, a random yaw.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_effect(&mut self, play: &mut PlayState, pos: Vec3, vel: Vec3, accel: Vec3, inner_color: [u8; 3], outer_color: [u8; 3], scale: f32, life: i16, ty: i16) {
        if let Some(e) = self.effects.iter_mut().find(|e| e.active == 0) {
            *e = BgDyYoseizoEffect {
                active: 1,
                pos,
                vel,
                accel,
                inner_color,
                alpha: 0,
                outer_color,
                scale,
                life,
                ty,
                pitch: 0.0,
                yaw: play.rand.centered_float(30000.0),
                rot: 0.0,
            };
        }
    }

    /// `BgDyYoseizo_UpdateEffects`: each turns 3000; type 0 drifts on its accelerating velocity,
    /// the others home on 150 below and 50 behind Link (their pitch and yaw towards it by 0.9, at
    /// most 5000), 3 a frame along each of the turned axes, each with `NA_SE_EV_HEALING`. Every
    /// slot's alpha: up 30 to 255 while it lives, then down 30 until it's free.
    pub fn update_effects(&mut self, play: &mut PlayState) {
        let player_pos = Self::player_pos(play);
        for i in 0..self.effects.len() {
            let mut e = self.effects[i];
            if e.active != 0 {
                e.rot += 3000.0;
                if e.ty == 0 {
                    e.pos += e.vel;
                    e.vel += e.accel;
                } else {
                    audio_play_actor_sfx2(play, NA_SE_EV_HEALING - SFX_FLAG);
                    let mut offset = player_pos;
                    offset.y = player_pos.y - 150.0;
                    offset.z = player_pos.z - 50.0;
                    let target_pitch = vec3f_pitch(e.pos, offset) as f32;
                    let target_yaw = vec3f_yaw(e.pos, offset) as f32;
                    approach_f(&mut e.pitch, target_pitch, 0.9, 5000.0);
                    approach_f(&mut e.yaw, target_yaw, 0.9, 5000.0);
                    let mut m = MtxF::set_translate(0.0, 0.0, 0.0);
                    m.rotate_y(binang_to_rad_alt(e.yaw));
                    m.rotate_x(binang_to_rad_alt(e.pitch));
                    let vel = m.mult_vec3f(Vec3::splat(3.0));
                    e.pos += vel;
                }
            }
            if e.life != 0 {
                e.life -= 1;
                e.alpha += 30;
                if e.alpha > 255 {
                    e.alpha = 255;
                }
            } else {
                e.alpha -= 30;
                if e.alpha <= 0 {
                    e.alpha = 0;
                    e.active = 0;
                }
            }
            self.effects[i] = e;
        }
    }

    /// `BgDyYoseizo_UpdatePosY`: towards 40 above home plus an offset growing to 10; the bob's
    /// velocity the sine of the phase (0.4 of it in a cutscene).
    fn update_pos_y(&mut self, play: &PlayState) {
        self.target_pos_y = self.above_fountain_y + self.offset_y;
        approach_f(&mut self.actor.world_pos.y, self.target_pos_y, 0.1, 10.0);
        approach_f(&mut self.offset_y, 10.0, 0.1, 0.5);
        if play.cs_ctx.state == CS_STATE_IDLE {
            self.actor.velocity.y = sin_s(truncf_binang(self.y_velocity_phase));
        } else {
            self.actor.velocity.y = sin_s(truncf_binang(self.y_velocity_phase)) * 0.4;
        }
    }

    /// `BgDyYoseizo_WaitSong` (`OOT_VERSION >= NTSC_1_1`): switch 0x38: `OCARINA_MODE_04`; without
    /// magic she's gone (but for the magic fountain's magic); else Link held.
    fn wait_song(&mut self, play: &mut PlayState) {
        if play.flags.get_switch(0x38) {
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            if Self::in_magic_fountain(play) {
                if !play.save.is_magic_acquired && self.reward != BG_DY_YOSEIZO_REWARD_MAGIC_MAGIC {
                    self.actor.kill();
                    return;
                }
            } else if !play.save.is_magic_acquired {
                self.actor.kill();
                return;
            }
            Self::set_cs_action(play, PLAYER_CSACTION_1);
            self.action = Action::SetupCutscene;
        }
    }

    /// Whether her reward is new (`BgDyYoseizo_SetupCutscene`'s choice); sets
    /// `isNewAcquireMagic` for the magic fountain's.
    pub fn is_new_acquire(&mut self, play: &PlayState) -> bool {
        let mut is_new_acquire = false;
        if !Self::in_magic_fountain(play) {
            match self.reward {
                BG_DY_YOSEIZO_REWARD_SPELL_FARORES_WIND => is_new_acquire = !play.save.get_item_get_inf(ITEMGETINF_FARORES_WIND),
                BG_DY_YOSEIZO_REWARD_SPELL_DINS_FIRE => is_new_acquire = !play.save.get_item_get_inf(ITEMGETINF_DINS_FIRE),
                BG_DY_YOSEIZO_REWARD_SPELL_NAYRUS_LOVE => is_new_acquire = !play.save.get_item_get_inf(ITEMGETINF_NAYRUS_LOVE),
                _ => {}
            }
        } else {
            match self.reward {
                BG_DY_YOSEIZO_REWARD_MAGIC_MAGIC => {
                    if !play.save.is_magic_acquired || BREG_2 != 0 {
                        log::debug!(" ☆☆☆☆☆ Spin attack speed UP ☆☆☆☆☆ ");
                        is_new_acquire = true;
                        self.is_new_acquire_magic = true;
                    }
                }
                BG_DY_YOSEIZO_REWARD_MAGIC_DOUBLE_MAGIC => {
                    if !play.save.is_double_magic_acquired {
                        log::debug!(" ☆☆☆☆☆ Magic gauge doubled ☆☆☆☆☆ ");
                        self.is_new_acquire_magic = true;
                        is_new_acquire = true;
                    }
                }
                BG_DY_YOSEIZO_REWARD_MAGIC_DOUBLE_DEFENSE => {
                    if !play.save.is_double_defense_acquired {
                        log::debug!(" ☆☆☆☆☆ Damage halved ☆☆☆☆☆ ");
                        self.is_new_acquire_magic = true;
                        is_new_acquire = true;
                    }
                }
                _ => {}
            }
        }
        is_new_acquire
    }

    /// The cutscene a new reward starts (`play->csCtx.script`), by fountain and reward.
    pub fn reward_cutscene(play: &PlayState, reward: i16) -> Option<&'static str> {
        let t = if Self::in_magic_fountain(play) { &MAGIC_CUTSCENES } else { &SPELL_CUTSCENES };
        usize::try_from(reward).ok().and_then(|r| t.get(r)).copied()
    }

    /// `BgDyYoseizo_SetupCutscene`: Link held; a new reward: its cutscene (not on a cutscene layer
    /// with the debug features), then waiting for it; else the revisit: the light setting 2, the
    /// one-point cutscene, her appearing sound, drawn.
    fn setup_cutscene(&mut self, play: &mut PlayState) {
        Self::set_cs_action(play, PLAYER_CSACTION_1);
        log::debug!("☆☆☆☆☆ mode ☆☆☆☆☆ {}", play.msg_ctx.ocarina_mode);
        if self.is_new_acquire(play) {
            if (!play.is_cutscene_layer() || !DEBUG_FEATURES)
                && let Some(name) = Self::reward_cutscene(play, self.reward)
            {
                play.cs_ctx.segment = play.cutscene_script(name);
                play.save.cutscene_trigger = 1;
            }
            self.action = Action::RewardWaitCutscene;
        } else {
            play.env_ctx.light_setting_override = 2;
            let cs_id = if Self::in_magic_fountain(play) { 8603 } else { 8604 };
            play.onepoint_cutscene_init(cs_id, -99, None, CAM_ID_MAIN);
            audio_play_actor_sfx2(play, NA_SE_EV_GREAT_FAIRY_APPEAR);
            self.drawn = true;
            self.action = Action::RevisitSetupAppear;
        }
    }

    /// `BgDyYoseizo_Revisit_SetupAppear`: the transition animation once, her laugh, Link held.
    fn revisit_setup_appear(&mut self, play: &mut PlayState) {
        self.change_anim(play, ANIM_TRANSITION, ANIMMODE_ONCE);
        audio_play_actor_sfx2(play, NA_SE_VO_FR_LAUGH_0);
        Self::set_cs_action(play, PLAYER_CSACTION_1);
        self.action = Action::RevisitAppear;
    }

    /// The rise and the growth (`Revisit_Appear`'s, `Reward_Appear`'s first half): true once she
    /// faces front. `revisit`: `Math_SmoothStepToS` and `-0x1F40` exclusive (the reward's
    /// `Math_ApproachS` from `-0x1F3F`).
    fn appear(&mut self, revisit: bool) -> bool {
        approach_f(&mut self.actor.world_pos.y, self.above_fountain_y, self.y_approach_fraction, 100.0);
        approach_f(&mut self.scale, 0.035, self.scale_approach_fraction, 0.005);
        approach_f(&mut self.y_approach_fraction, 0.8, 0.1, 0.02);
        approach_f(&mut self.scale_approach_fraction, 0.2, 0.03, 0.05);
        let y = self.actor.shape_rot.y;
        let facing = if revisit { y > -0x1F40 && y < 0x3E8 } else { y >= -0x1F3F && y < 0x3E8 };
        if self.scale >= 0.034 && facing {
            self.skel.update();
            if revisit {
                smooth_step_to_s(&mut self.actor.shape_rot.y, 0, 5, 0x3E8, 0);
            } else {
                approach_s(&mut self.actor.shape_rot.y, 0, 5, 0x3E8);
            }
            return (self.actor.shape_rot.y as f32).abs() < 50.0;
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0xBB8);
        false
    }

    /// `BgDyYoseizo_Revisit_Appear`.
    fn revisit_appear(&mut self, play: &mut PlayState) {
        Self::set_cs_action(play, PLAYER_CSACTION_1);
        if self.appear(true) {
            self.action = Action::RevisitFinishAppear;
        }
        self.spawn_effects_type(play, 0);
    }

    /// `BgDyYoseizo_Revisit_FinishAppear`: the transition played out.
    fn revisit_finish_appear(&mut self, play: &mut PlayState) {
        let anim_prev_frame = self.skel.cur_frame;
        Self::set_cs_action(play, PLAYER_CSACTION_1);
        if (self.cur_anim_last_frame * 1273.0) <= self.y_velocity_phase {
            self.y_velocity_phase = 0.0;
        }
        self.skel.update();
        if self.cur_anim_last_frame <= anim_prev_frame && !self.has_switched_anim {
            self.action = Action::RevisitSetupTalk;
        }
    }

    /// `BgDyYoseizo_Revisit_SetupTalk`: her idle looped, text 0xDB.
    fn revisit_setup_talk(&mut self, play: &mut PlayState) {
        Self::set_cs_action(play, PLAYER_CSACTION_1);
        self.change_anim(play, ANIM_IDLE, ANIMMODE_LOOP);
        self.actor.text_id = 0xDB;
        self.wait_message_state = TEXT_STATE_EVENT;
        play.start_textbox(self.actor.text_id, None);
        self.spawn_effects_type(play, 0);
        self.action = Action::RevisitTalk;
    }

    /// `Message_CloseTextbox`.
    fn close_textbox(play: &mut PlayState) {
        play.with_msg(|m, f| m.close_textbox(f.audio));
    }

    /// `BgDyYoseizo_Revisit_Talk`: the bob by her animation (1273 a frame); the text closed: the
    /// HUD's hearts and magic, the refill.
    fn revisit_talk(&mut self, play: &mut PlayState) {
        Self::set_cs_action(play, PLAYER_CSACTION_1);
        self.y_velocity_phase = self.skel.cur_frame * 1273.0;
        if self.y_velocity_phase >= (self.cur_anim_last_frame * 1273.0) {
            self.y_velocity_phase = 0.0;
        }
        self.skel.update();
        if self.wait_message_state == play.message_state() && play.message_should_advance() {
            Self::close_textbox(play);
            oot_game::interface::change_alpha(&mut play.save, HUD_VISIBILITY_A_HEARTS_MAGIC_FORCE);
            self.action = Action::RevisitSetupRefillPlayerEnergy;
        }
        self.update_pos_y(play);
        self.spawn_effects_type(play, 0);
    }

    /// `BgDyYoseizo_Revisit_SetupRefillPlayerEnergy`: the giving animation once, her smile, the
    /// mouth open.
    fn revisit_setup_refill_player_energy(&mut self, play: &mut PlayState) {
        self.change_anim(play, ANIM_GIVING, ANIMMODE_ONCE);
        audio_play_actor_sfx2(play, NA_SE_VO_FR_SMILE_0);
        self.mouth_tex_index = 1;
        self.action = Action::RevisitRefillPlayerEnergy;
    }

    /// `BgDyYoseizo_Revisit_RefillPlayerEnergy`: the giving animation over: the after-animation,
    /// the refill's 150 frames, the beam (unless the reward was new: `isNewAcquireMagic`); at 110 the
    /// health and magic; full (or the 200 frames' timeout) the timer runs on, the beam fading at 90;
    /// at 1 text 0xDA.
    fn revisit_refill_player_energy(&mut self, play: &mut PlayState) {
        let anim_prev_frame = self.skel.cur_frame;
        if self.has_switched_anim {
            self.y_velocity_phase = self.skel.cur_frame * 1300.0;
            if (self.cur_anim_last_frame * 1300.0) <= self.y_velocity_phase {
                self.y_velocity_phase = 0.0;
            }
        }
        self.skel.update();
        if self.cur_anim_last_frame <= anim_prev_frame && !self.has_switched_anim {
            self.change_anim(play, ANIM_AFTER, ANIMMODE_LOOP);
            self.refill_timer = 150;
            self.has_switched_anim = true;
            if !self.is_new_acquire_magic {
                let pp = Self::player_pos(play);
                let beam_pos = Vec3::new(pp.x, pp.y + 200.0, pp.z);
                let beam_type = if Self::in_magic_fountain(play) { 0 } else { 1 };
                self.refill_beam = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_DY_EXTRA, beam_pos, [0; 3], beam_type).ok();
            }
        }
        if self.refill_timeout_timer > 1 {
            self.refill_timeout_timer -= 1;
        }
        if self.refill_timer >= 110 {
            self.refill_timer -= 1;
        }
        if self.refill_timer == 110 {
            play.save.health_accumulator = 0x140;
            play.magic_fill();
            self.refill_timeout_timer = 200;
        }
        if (play.save.health_capacity == play.save.health && play.save.magic as i16 == play.save.magic_capacity) || self.refill_timeout_timer == 1 {
            self.refill_timer -= 1;
            if self.refill_timer == 90 {
                if !self.is_new_acquire_magic {
                    // this->refillBeam->startDisappearing = true (spawned with the after-animation).
                    if let Some(b) = self.refill_beam.and_then(|h| play.actors.downcast_mut::<EnDyExtra>(h)) {
                        b.start_disappearing = 1;
                    }
                }
                self.is_new_acquire_magic = false;
            }
        }
        if self.refill_timer == 1 {
            self.actor.text_id = 0xDA;
            self.wait_message_state = TEXT_STATE_EVENT;
            play.continue_textbox(self.actor.text_id);
            self.action = Action::RevisitWaitTalkEnd;
        } else {
            self.update_pos_y(play);
        }
    }

    /// `BgDyYoseizo_Revisit_WaitTalkEnd`: the text closed: the mouth shut, the disappear, the
    /// active camera done.
    fn revisit_wait_talk_end(&mut self, play: &mut PlayState) {
        self.y_velocity_phase = self.skel.cur_frame * 1400.0;
        if self.y_velocity_phase >= (self.cur_anim_last_frame * 1400.0) {
            self.y_velocity_phase = 0.0;
        }
        self.skel.update();
        if self.wait_message_state == play.message_state() && play.message_should_advance() {
            Self::close_textbox(play);
            self.mouth_tex_index = 0;
            self.action = Action::SetupDisappear;
            let active = play.active_cam_id;
            play.camera_set_finished_flag(active);
        }
        self.update_pos_y(play);
        self.spawn_effects_type(play, 0);
    }

    /// `BgDyYoseizo_SetupDisappear`: the spin and shrink once, 5 frames, her laugh and the vanish.
    fn setup_disappear(&mut self, play: &mut PlayState) {
        self.change_anim(play, ANIM_SPIN_SHRINK, ANIMMODE_ONCE);
        self.disappear_timer = 5;
        self.scale_approach_fraction = 0.0;
        self.y_approach_fraction = 0.0;
        audio_play_actor_sfx2(play, NA_SE_VO_FR_LAUGH_0);
        audio_play_actor_sfx2(play, NA_SE_EV_GREAT_FAIRY_VANISH);
        self.action = Action::Disappear;
    }

    /// `BgDyYoseizo_Disappear`: after the 5 frames, down to her home and shrinking, spinning; under
    /// 0.003 big, 30 frames to go.
    fn disappear(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.disappear_timer == 0 {
            if self.scale < 0.003 {
                self.disappear_timer = 30;
                self.action = Action::Kill;
            } else {
                approach_f(&mut self.actor.world_pos.y, self.home_pos_y, self.y_approach_fraction, 100.0);
                approach_zero_f(&mut self.scale, self.scale_approach_fraction, 0.005);
                approach_f(&mut self.y_approach_fraction, 0.8, 0.1, 0.02);
                approach_f(&mut self.scale_approach_fraction, 0.2, 0.03, 0.05);
                self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0xBB8);
                self.spawn_effects_type(play, 0);
            }
        }
    }

    /// `BgDyYoseizo_Kill`: the 30 frames over: Link let go, the light setting back, the first
    /// `En_Okarina_Tag` of the props killed, switch 0x38 cleared, gone.
    fn kill(&mut self, play: &mut PlayState) {
        if self.disappear_timer == 0 {
            Self::set_cs_action(play, PLAYER_CSACTION_7);
            play.env_ctx.light_setting_override = 0;
            let tag = play.actors.category(ACTORCAT_PROP).iter().copied().find(|&h| play.actors.actor(h).is_some_and(|a| a.id == ACTOR_EN_OKARINA_TAG));
            if let Some(a) = tag.and_then(|h| play.actors.actor_mut(h)) {
                a.kill();
            }
            play.flags.unset_switch(0x38);
            self.actor.kill();
        }
    }

    /// `BgDyYoseizo_Reward_WaitCutscene`: cue 2: drawn, Link held, the transition once, her
    /// appearing sound.
    fn reward_wait_cutscene(&mut self, play: &mut PlayState) {
        if play.cs_ctx.state != CS_STATE_IDLE && Self::cue(play).is_some_and(|c| c.action == 2) {
            self.drawn = true;
            Self::set_cs_action(play, PLAYER_CSACTION_1);
            self.is_done_appearing = false;
            self.change_anim(play, ANIM_TRANSITION, ANIMMODE_ONCE);
            audio_play_actor_sfx2(play, NA_SE_EV_GREAT_FAIRY_APPEAR);
            self.action = Action::RewardAppear;
        }
    }

    /// `BgDyYoseizo_Reward_Appear`: the rise; then the transition out, her idle looped; cue 3: the
    /// giving animation once, the mouth open.
    fn reward_appear(&mut self, play: &mut PlayState) {
        let anim_prev_frame = self.skel.cur_frame;
        if !self.is_done_appearing {
            if self.appear(false) {
                self.is_done_appearing = true;
            }
        } else {
            self.skel.update();
            if self.cur_anim_last_frame <= anim_prev_frame && !self.has_switched_anim {
                self.change_anim(play, ANIM_IDLE, ANIMMODE_LOOP);
                self.has_switched_anim = true;
            }
            if play.cs_ctx.state != CS_STATE_IDLE && Self::cue(play).is_some_and(|c| c.action == 3) {
                self.is_done_appearing = false;
                self.has_switched_anim = false;
                self.change_anim(play, ANIM_GIVING, ANIMMODE_ONCE);
                self.mouth_tex_index = 1;
                self.action = Action::RewardGiveReward;
            }
        }
        self.spawn_effects_type(play, 0);
    }

    /// `BgDyYoseizo_Reward_GiveReward`: the cues 4 to 21 (the module's doc).
    fn reward_give_reward(&mut self, play: &mut PlayState) {
        let anim_prev_frame = self.skel.cur_frame;
        if self.has_switched_anim {
            self.y_velocity_phase = self.skel.cur_frame * 1400.0;
            if self.y_velocity_phase >= (self.cur_anim_last_frame * 1400.0) {
                self.y_velocity_phase = 0.0;
            }
        }
        self.skel.update();
        if self.cur_anim_last_frame <= anim_prev_frame && !self.has_switched_anim {
            self.change_anim(play, ANIM_AFTER, ANIMMODE_LOOP);
            self.has_switched_anim = true;
        }
        let magic = Self::in_magic_fountain(play);
        // play->csCtx.actorCues[0]->id, read without a check: the cutscene's last cue (13) comes
        // before its end. With none (no script) the C reads through NULL; 0 here.
        let id = Self::cue(play).map_or(0, |c| c.action as i32);
        if id == 13 {
            self.action = Action::SetupDisappear;
            return;
        }
        if (4..=6).contains(&id) {
            let n = (id - 4) as usize;
            if magic {
                self.spawn_effects_type(play, n as i16 + 1);
            } else if !self.has_spawned_demo_effect {
                let demo_effect_params = ((SPELL_REWARDS_DEMO_EFFECT_LIGHT_COLOR[n] as i16) << 12) | DEMO_EFFECT_LIGHT as i16;
                let _ = play.actor_spawn(ACTOR_DEMO_EFFECT, self.actor.world_pos, [0; 3], demo_effect_params);
                self.has_spawned_demo_effect = true;
            }
        } else {
            self.spawn_effects_type(play, 0);
        }
        if magic && (10..=12).contains(&id) {
            let n = (id - 10) as i16;
            match n {
                BG_DY_YOSEIZO_REWARD_MAGIC_MAGIC => {
                    play.save.is_magic_acquired = true;
                    play.save.magic_fill_target = 0x30;
                    oot_game::interface::change_alpha(&mut play.save, HUD_VISIBILITY_HEARTS_MAGIC);
                }
                BG_DY_YOSEIZO_REWARD_MAGIC_DOUBLE_MAGIC => {
                    if !play.save.is_magic_acquired {
                        play.save.is_magic_acquired = true;
                    }
                    play.save.is_double_magic_acquired = true;
                    play.save.magic_fill_target = 0x60;
                    play.save.magic_level = 0;
                    oot_game::interface::change_alpha(&mut play.save, HUD_VISIBILITY_HEARTS_MAGIC);
                }
                BG_DY_YOSEIZO_REWARD_MAGIC_DOUBLE_DEFENSE => {
                    play.save.is_double_defense_acquired = true;
                    oot_game::interface::change_alpha(&mut play.save, HUD_VISIBILITY_HEARTS_MAGIC);
                }
                _ => {}
            }
            if !self.energy_refilled {
                play.save.health_accumulator = 0x140;
                self.energy_refilled = true;
                if n == 2 {
                    play.magic_fill();
                }
            }
        }
        if !magic && (14..=16).contains(&id) {
            let n = (id - 14) as usize;
            let pp = Self::player_pos(play);
            let item_y = if play.save.adult { pp.y + 73.0 } else { pp.y + 53.0 };
            if !self.has_spawned_item {
                let item_pos = Vec3::new(pp.x, item_y, pp.z);
                self.item = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_EX_ITEM, item_pos, [0; 3], SPELL_REWARDS_EN_EX_ITEM_PARAMS[n]).ok();
                if self.item.is_some() {
                    if !play.save.is_magic_acquired {
                        play.save.is_magic_acquired = true;
                    } else {
                        play.magic_fill();
                    }
                    self.has_spawned_item = true;
                    play.save.health_accumulator = 0x140;
                    oot_game::interface::change_alpha(&mut play.save, HUD_VISIBILITY_HEARTS_MAGIC);
                    // schema: itemGetInf[ITEMGETINF_INDEX_SPELLS] |= sSpellRewardsItemGetInfMask[n]
                    play.save.set_item_get_inf(SPELL_REWARDS_ITEM_GET_INF[n]);
                    oot_game::item::item_give(&mut play.save, Some(&mut play.audio), SPELL_REWARDS_ITEM[n]);
                }
            } else if let Some(item) = self.item.and_then(|h| play.actors.downcast_mut::<EnExItem>(h)) {
                item.actor.world_pos = Vec3::new(pp.x, item_y, pp.z);
                item.scale = 0.3;
            }
        }
        if !magic && id == 17 {
            if let Some(a) = self.item.and_then(|h| play.actors.actor_mut(h)) {
                a.kill();
            }
            self.item = None;
        }
        if magic && id == 18 {
            self.give_double_defense = true;
        }
        if self.give_double_defense && play.save.inventory.defense_hearts < 20 {
            play.save.inventory.defense_hearts += 1;
        }
        if (19..=21).contains(&id) && self.unk304 == 0 {
            let n = (id - (19 - WARP_ORANGE as i32)) as i16;
            let pp = Self::player_pos(play);
            let _ = play.actor_spawn(ACTOR_DOOR_WARP1, pp, [0; 3], n);
            self.unk304 = 1;
        }
        self.update_pos_y(play);
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::WaitSong => self.wait_song(play),
            Action::SetupCutscene => self.setup_cutscene(play),
            Action::RevisitSetupAppear => self.revisit_setup_appear(play),
            Action::RevisitAppear => self.revisit_appear(play),
            Action::RevisitFinishAppear => self.revisit_finish_appear(play),
            Action::RevisitSetupTalk => self.revisit_setup_talk(play),
            Action::RevisitTalk => self.revisit_talk(play),
            Action::RevisitSetupRefillPlayerEnergy => self.revisit_setup_refill_player_energy(play),
            Action::RevisitRefillPlayerEnergy => self.revisit_refill_player_energy(play),
            Action::RevisitWaitTalkEnd => self.revisit_wait_talk_end(play),
            Action::SetupDisappear => self.setup_disappear(play),
            Action::Disappear => self.disappear(play),
            Action::Kill => self.kill(play),
            Action::RewardWaitCutscene => self.reward_wait_cutscene(play),
            Action::RewardAppear => self.reward_appear(play),
            Action::RewardGiveReward => self.reward_give_reward(play),
        }
    }

    /// The skeleton's pose with `OverrideLimbDraw`'s turns: the torso's x by `torsoRot.y`, the
    /// head's x by `headRot.y` and z by `headRot.z`.
    fn pose(&self, joints: &[[i16; 3]], head: [i16; 2], torso_y: i16) -> Vec<Mat4> {
        self.skeleton.pose_override(joints, |limb, _pos, rot| {
            if limb == LIMB_TORSO {
                rot[0] = rot[0].wrapping_add(torso_y);
            }
            if limb == LIMB_HEAD {
                rot[0] = rot[0].wrapping_add(head[0]);
                rot[2] = rot[2].wrapping_add(head[1]);
            }
            Mat4::IDENTITY
        })
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `switches`: drawn, killed (`BgDyYoseizo_Kill`), the eye, the mouth.
    pub const DRAWN: usize = 0;
    pub const KILL: usize = 1;
    pub const EYE: usize = 2;
    pub const MOUTH: usize = 3;
    pub const SWITCHES: usize = 4;
    /// `angles`: `headRot.y`, `headRot.z`, `torsoRot.y`.
    pub const ANGLES: usize = 3;
}

impl ActorImpl for BgDyYoseizo {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgDyYoseizo_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.unk_2f0 = self.unk_2f0.wrapping_add(1);
        if self.disappear_timer != 0 {
            self.disappear_timer -= 1;
        }
        if self.eye_timer != 0 {
            self.eye_timer -= 1;
        }
        if self.unk_2fa != 0 {
            self.unk_2fa -= 1;
        }
        self.run_action(play);
        if play.cs_ctx.state != CS_STATE_IDLE {
            let f = play.cs_ctx.frames;
            let mut sfx = 0;
            if Self::in_magic_fountain(play) {
                if f == 32 || f == 291 || f == 426 || f == 851 {
                    sfx = 1;
                }
                if f == 101 {
                    sfx = 2;
                }
            } else {
                if f == 35 || f == 181 || f == 462 || f == 795 {
                    sfx = 1;
                }
                if f == 90 {
                    sfx = 2;
                }
            }
            if sfx == 1 {
                audio_play_actor_sfx2(play, NA_SE_VO_FR_SMILE_0);
            }
            if sfx == 2 {
                audio_play_actor_sfx2(play, NA_SE_VO_FR_LAUGH_0);
            }
        }
        if self.eye_timer == 0 && self.action != Action::RevisitRefillPlayerEnergy {
            self.eye_tex_index += 1;
            self.unused_eye_tex_index += 1;
            if self.eye_tex_index >= 3 {
                self.eye_tex_index = 0;
                self.unused_eye_tex_index = 0;
                self.eye_timer = play.rand.zero_float(60.0) as i16 + 20;
            }
        }
        self.actor.move_forward();
        self.focus_y = self.scale * 7500.0;
        self.actor.set_focus(self.focus_y);
        // @bug (game): the focus's height above the origin, not above her (the module's doc).
        self.actor.focus_pos.y = self.focus_y;
        let focus = self.actor.focus_pos;
        actor_track_player(play, &mut self.actor, &mut self.head_rot, &mut self.torso_rot, focus);
        self.update_effects(play);
        self.actor.scale = Vec3::splat(self.scale);
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        r.switches = vec![self.drawn as u32, (self.action == Action::Kill) as u32, self.eye_tex_index.clamp(0, 2) as u32, self.mouth_tex_index.clamp(0, 1) as u32];
        r.angles = vec![self.head_rot[1], self.head_rot[2], self.torso_rot[1]];
        r
    }

    /// `BgDyYoseizo_Draw` (once set): the skeleton (not in `BgDyYoseizo_Kill`), then
    /// `BgDyYoseizo_DrawEffects`: each active effect billboarded at its position, `scale` wide,
    /// turned by `rot`, its inner colour as prim (its alpha) and outer as env.
    fn draw(&self, rs: &RenderState, _play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.len() < rs::SWITCHES || rs.angles.len() < rs::ANGLES || rs.switches[rs::DRAWN] == 0 {
            return;
        }
        if rs.switches[rs::KILL] == 0
            && let Some(joints) = &rs.joints
        {
            let bones = self.pose(&joints.rot, [rs.angles[0], rs.angles[1]], rs.angles[2]);
            let key = MeshKey::named(keys::bake(&bake_name(rs.switches[rs::EYE] as usize, rs.switches[rs::MOUTH] as usize)));
            out.opa.push(DrawCmd { mesh: key, transform: actor_draw_matrix(rs), bones, params: Default::default() });
        }
        for e in self.effects.iter().filter(|e| e.active == 1) {
            let m = oot_game::draw::replace_rotation(Mat4::from_translation(e.pos), view.billboard) * Mat4::from_scale(Vec3::new(e.scale, e.scale, 1.0)) * Mat4::from_rotation_z(e.rot);
            let mut sv = SegmentValues::default();
            let (i, o) = (e.inner_color, e.outer_color);
            sv.prim[SEG_PARTICLE_COLOR as usize] = Some([i[0], i[1], i[2], e.alpha as u8]);
            sv.env[SEG_PARTICLE_COLOR as usize] = Some([o[0], o[1], o[2], 0]);
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(PARTICLE_BAKE)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
    }

    /// `BgDyYoseizo_Destroy`: nothing.
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
