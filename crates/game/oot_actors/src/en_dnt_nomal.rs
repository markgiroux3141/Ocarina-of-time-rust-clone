//! `En_Dnt_Nomal` (`ovl_En_Dnt_Nomal/z_en_dnt_nomal.c`): the Lost Woods' minigame scrubs. Two
//! kinds, by `type`:
//!
//! - **The target scrub** (`ENDNTNOMAL_TARGET`, the Lost Woods' room 1): a Deku Scrub
//!   (`object_hintnuts`) in its flower by the target, a fixed quad at (1340, 50, -30), 48 square,
//!   facing ±x, hit by Link's slingshot seeds and arrows (`EnDntNomal_TargetWait`). A hit within 8
//!   of its centre is a bullseye: the score (`EffectSsExtra_Spawn`, `EXTRA_SCORE_100`) and the chime
//!   (`NA_SE_SY_TRE_BOX_APPEAR`); within 24, the ring: `EXTRA_SCORE_30`, and the count of bullseyes
//!   starts over. Three in a row as a child without `ITEMGETINF_1D`: one-point cutscene 4140,
//!   Player held (`PLAYER_CSACTION_1`), and 50 frames later the scrub comes up
//!   (`EnDntNomal_SetupTargetUnburrow`: `EffectSsHahen_SpawnBurst`), runs under the target
//!   (`EnDntNomal_TargetWalk`), turns to Link and says 0x10AF (`EnDntNomal_TargetTalk`); the text's
//!   event ends the camera (`Camera_SetFinishedFlag`, `csId` 0, `PLAYER_CSACTION_8`), and it spits
//!   the prize (`En_Ex_Item` `EXITEM_BULLET_BAG` as its child at `mouthPos`, 10 off in x), runs
//!   back to its flower and burrows (`EnDntNomal_TargetBurrow`: waiting again, this ROM being
//!   gc-eu-mq-dbg, PAL 1.0 or later and not N64 PAL).
//! - **The stage scrubs** (`ENDNTNOMAL_STAGE`, types 1 to 9): the mask contest's audience in the
//!   Deku theatre grotto (`object_dnk`), spawned and driven by `En_Dnt_Demo` (not ported, Phase
//!   20), which writes their `stageSignal`, `action`, `stagePrize`, `targetPos` and `ignore`
//!   (`DNT_*`, `z_en_dnt_demo.h`). On a signal (`EnDntNomal_Update`) they come up and look at Link
//!   (`EnDntNomal_StageUp`; with a prize, at `targetPos`), celebrate (`EnDntNomal_StageCelebrate`:
//!   running to `targetPos`, hopping walls, the circle shadow), dance (`EnDntNomal_StageDance`),
//!   hide (`EnDntNomal_StageHide`: rupees, `En_Ex_Ruppy` 3, for `DNT_ACTION_LOW_RUPEES` and
//!   `_HIGH_RUPEES`), attack (`EnDntNomal_StageAttack`: an enemy now, nuts, `En_Nutsball` 4) and
//!   return to their flowers (`EnDntNomal_StageReturn`). Leaf colours by type (`sLeafColors`, the
//!   env colour of limbs 1, 3, 4, 5 and 6 and the flower), eyes by `eyeState` (segment 8).
//!
//! The whole overlay is ported (the `OOT_PAL_N64` branch, `EnDntNomal_DoNothing`, isn't this
//! ROM's). `@bug (game)`: the target scrub's `gHintNutsSkel` is a flex skeleton set up
//! (`SkelAnime_Init`) and drawn (`SkelAnime_DrawOpa`) as a normal one; its limbs' lists load no
//! matrices (no `gSPMatrix` of segment 0x0D), so it draws the same, and the port draws it as
//! `En_Hintnuts` does. The circle shadow is the shape's (`ActorShape_Init`'s, drawn by
//! `Actor_Draw`), of scale 0: nothing to see. A celebrating stage scrub's draw puts a circle under
//! it itself (`func_80033C30`, 0.25: `oot_game::shadow::func_80033c30`).
//!
//! Params: `type` the whole params as an s16 (below 0 read as 0: `ENDNTNOMAL_TARGET`, the Lost
//! Woods' 0xFFFF; 1 to 9: `ENDNTNOMAL_STAGE`, `En_Dnt_Demo`'s scrubs, the type picking the leaf
//! colour `sLeafColors[type - 1]` and the celebration's delays).
//! Flags: reads ITEMGETINF_1D (the bullet bag won: bullseyes no longer count).

use std::collections::HashMap;
use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::smooth_step_to_s;
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_PROP, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::audio::sfx::*;
use oot_game::camera::{CAM_ID_MAIN, f_atan2f};
use oot_game::collision_check::*;
use oot_game::effect::extra::{EXTRA_SCORE_30, EXTRA_SCORE_100};
use oot_game::effect::hahen::HAHEN_OBJECT_DEFAULT;
use oot_game::item_get_inf;
use oot_game::message::{TEXT_STATE_EVENT, TEXT_STATE_NONE};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};
use oot_game::sys_matrix::{MtxF, rad_to_binang};

use crate::en_ex_item::{ACTOR_EN_EX_ITEM, EXITEM_BULLET_BAG};
use crate::en_ex_ruppy::{ACTOR_EN_EX_RUPPY, ENEXRUPPY_TYPE_HYRULE_COURTYARD, EnExRuppy};
use crate::en_nutsball::{ACTOR_EN_NUTSBALL, EN_NUTSBALL_TYPE_DNK};

/// `ACTOR_EN_DNT_NOMAL` (`actor_table.h`: 0x01A3).
pub const ACTOR_EN_DNT_NOMAL: i16 = 0x01A3;

/// `En_Dnt_Nomal_Profile`: `ACTORCAT_PROP`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_DRAW_CULLING_DISABLED`, `OBJECT_GAMEPLAY_KEEP` (its own objects are waited for).
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_DNT_NOMAL,
    name: "En_Dnt_Nomal",
    category: ACTORCAT_PROP,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: "gameplay_keep",
    schema: &SCHEMA,
};

/// `this->type = this->actor.params` (an s16; below `ENDNTNOMAL_TARGET` it's made
/// `ENDNTNOMAL_TARGET`).
pub const TYPE: Field = Field::whole(
    "type",
    Kind::Named(&[
        (-1, "ENDNTNOMAL_TARGET (read from below 0: the Lost Woods' 0xFFFF)"),
        (0, "ENDNTNOMAL_TARGET"),
        (1, "ENDNTNOMAL_STAGE: En_Dnt_Demo's 1st scrub"),
        (2, "En_Dnt_Demo's 2nd scrub"),
        (3, "En_Dnt_Demo's 3rd scrub"),
        (4, "En_Dnt_Demo's 4th scrub"),
        (5, "En_Dnt_Demo's 5th scrub"),
        (6, "En_Dnt_Demo's 6th scrub"),
        (7, "En_Dnt_Demo's 7th scrub"),
        (8, "En_Dnt_Demo's 8th scrub"),
        (9, "En_Dnt_Demo's 9th scrub"),
    ]),
    "the target scrub, or a mask contest audience scrub (its leaf colour and delays)",
);

/// `ITEMGETINF_1D` (`save.h`): the target game's bullet bag won.
pub const ITEMGETINF_1D: u16 = 0x1D;

/// En_Dnt_Nomal's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE],
    flags: &[FlagUse::new(item_get_inf!(ITEMGETINF_1D)).reads().note("the target type: set, bullseyes don't count")],
    todo: false,
};

/// `ENDNTNOMAL_TARGET`, `ENDNTNOMAL_STAGE` (`z_en_dnt_nomal.h`).
pub const ENDNTNOMAL_TARGET: i16 = 0;
pub const ENDNTNOMAL_STAGE: i16 = 1;

// `EnDntSignal` (`z_en_dnt_demo.h`): what `En_Dnt_Demo` tells a stage scrub (`stageSignal`).
pub const DNT_SIGNAL_NONE: i16 = 0;
pub const DNT_SIGNAL_LOOK: i16 = 1;
pub const DNT_SIGNAL_CELEBRATE: i16 = 2;
pub const DNT_SIGNAL_DANCE: i16 = 3;
pub const DNT_SIGNAL_HIDE: i16 = 4;
pub const DNT_SIGNAL_RETURN: i16 = 5;
pub const DNT_SIGNAL_UNUSED: i16 = 6;

// `EnDntPrize` (`stagePrize`).
pub const DNT_PRIZE_NONE: u8 = 0;
pub const DNT_PRIZE_NUTS: u8 = 1;
pub const DNT_PRIZE_STICK: u8 = 2;

// `EnDntAction` (`action`).
pub const DNT_ACTION_NONE: i16 = 0;
pub const DNT_ACTION_DANCE: i16 = 1;
pub const DNT_ACTION_ATTACK: i16 = 2;
pub const DNT_ACTION_LOW_RUPEES: i16 = 3;
pub const DNT_ACTION_HIGH_RUPEES: i16 = 4;
pub const DNT_ACTION_PRIZE: i16 = 5;

/// `OBJECT_HINTNUTS`, `OBJECT_DNK` (`object_table.h`: 0x0164, 0x0172).
pub const OBJECT_HINTNUTS: i16 = 0x0164;
pub const OBJECT_DNK: i16 = 0x0172;
const HINTNUTS: &str = "object_hintnuts";
const DNK: &str = "object_dnk";

/// `PLAYER_CSACTION_*` (`player.h`).
const PLAYER_CSACTION_1: u8 = 1;
const PLAYER_CSACTION_7: u8 = 7;
const PLAYER_CSACTION_8: u8 = 8;

/// `BREG(0)`: a debug register in the hit check (the `OOT_VERSION >= PAL_1_0` branch); 0.
const BREG_0: i32 = 0;

/// The target's centre (`EnDntNomal_TargetWait`'s `targetX`, `targetY`, `targetZ`).
pub const TARGET_POS: Vec3 = Vec3::new(1340.0, 50.0, -30.0);

/// `sBodyCylinderInit`: pushes only, 16 by 46.
pub const BODY_CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 16, height: 46, y_shift: 0, pos: [0, 0, 0] },
};

/// `sTargetQuadInit`: hit by Link's seeds and arrows (`0x0001F824`: `DMG_SLINGSHOT`,
/// `DMG_ARROW_NORMAL`, the magic arrows and the unknown ones).
pub const TARGET_QUAD_INIT: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0001_F824, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `sLeafColors`: by `type - ENDNTNOMAL_STAGE`.
pub const LEAF_COLORS: [[u8; 4]; 9] = [
    [255, 255, 255, 255],
    [255, 195, 175, 255],
    [210, 255, 0, 255],
    [255, 255, 255, 255],
    [210, 255, 0, 255],
    [255, 195, 175, 255],
    [255, 255, 255, 255],
    [255, 195, 175, 255],
    [210, 255, 0, 255],
];

/// `EnDntNomal_DrawStageScrub`'s `blinkTex`, by `eyeState`.
const BLINK_TEX: [&str; 3] = ["gDntStageEyeOpenTex", "gDntStageEyeHalfTex", "gDntStageEyeShutTex"];
/// The segment the eyes are on, and the one the bakes take the leaf colour from.
const SEG_EYE: u8 = 0x08;
const SEG_ENV: u8 = 0x0B;

/// The stage scrub drawn with eye `eye` (`gDntStageSkel` after its leaf colour).
pub fn stage_bake(eye: usize) -> String {
    format!("En_Dnt_Nomal/stage_eye{eye}")
}
/// `gDntStageFlowerDL` after its leaf colour.
pub const STAGE_FLOWER_BAKE: &str = "En_Dnt_Nomal/stage_flower";

/// The stage scrub's meshes: the skeleton with each eye texture on segment 8, and the flower, each
/// after `gDPSetEnvColor` of its leaf colour (`EnDntNomal_OverrideLimbDraw` sets it before limbs
/// 1, 3, 4, 5 and 6, the same colour each time, and no list sets its own: the whole skeleton
/// draws in it). The target scrub draws `object_hintnuts`' own meshes.
pub fn bakes() -> Vec<MeshBake> {
    let env = || (SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false });
    let mut v: Vec<MeshBake> = (0..BLINK_TEX.len())
        .map(|eye| MeshBake {
            name: stage_bake(eye),
            object: DNK.into(),
            segments: vec![(SEG_EYE, BakeSegment::Texture { file: DNK.into(), symbol: BLINK_TEX[eye].into() }), env()],
            prelude: vec![SEG_ENV],
            body: BakeBody::Skeleton { file: DNK.into(), symbol: "gDntStageSkel".into(), limbs: Vec::new() },
        })
        .collect();
    v.push(MeshBake { name: STAGE_FLOWER_BAKE.into(), object: DNK.into(), segments: vec![env()], prelude: vec![SEG_ENV], body: BakeBody::DLists(vec![(DNK.into(), "gDntStageFlowerDL".into())]) });
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitForObject,
    SetFlower,
    SetupTargetWait,
    TargetWait,
    SetupTargetUnburrow,
    TargetUnburrow,
    SetupTargetWalk,
    TargetWalk,
    TargetFacePlayer,
    SetupTargetTalk,
    TargetTalk,
    SetupTargetGivePrize,
    TargetGivePrize,
    TargetReturn,
    TargetBurrow,
    SetupStageWait,
    StageWait,
    SetupStageUp,
    StageUp,
    SetupStageUnburrow,
    StageUnburrow,
    SetupStageCelebrate,
    StageCelebrate,
    SetupStageDance,
    StageDance,
    SetupStageHide,
    StageHide,
    StageAttackHide,
    SetupStageAttack,
    StageAttack,
    StageSetupReturn,
    StageReturn,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::WaitForObject => "EnDntNomal_WaitForObject",
            Action::SetFlower => "EnDntNomal_SetFlower",
            Action::SetupTargetWait => "EnDntNomal_SetupTargetWait",
            Action::TargetWait => "EnDntNomal_TargetWait",
            Action::SetupTargetUnburrow => "EnDntNomal_SetupTargetUnburrow",
            Action::TargetUnburrow => "EnDntNomal_TargetUnburrow",
            Action::SetupTargetWalk => "EnDntNomal_SetupTargetWalk",
            Action::TargetWalk => "EnDntNomal_TargetWalk",
            Action::TargetFacePlayer => "EnDntNomal_TargetFacePlayer",
            Action::SetupTargetTalk => "EnDntNomal_SetupTargetTalk",
            Action::TargetTalk => "EnDntNomal_TargetTalk",
            Action::SetupTargetGivePrize => "EnDntNomal_SetupTargetGivePrize",
            Action::TargetGivePrize => "EnDntNomal_TargetGivePrize",
            Action::TargetReturn => "EnDntNomal_TargetReturn",
            Action::TargetBurrow => "EnDntNomal_TargetBurrow",
            Action::SetupStageWait => "EnDntNomal_SetupStageWait",
            Action::StageWait => "EnDntNomal_StageWait",
            Action::SetupStageUp => "EnDntNomal_SetupStageUp",
            Action::StageUp => "EnDntNomal_StageUp",
            Action::SetupStageUnburrow => "EnDntNomal_SetupStageUnburrow",
            Action::StageUnburrow => "EnDntNomal_StageUnburrow",
            Action::SetupStageCelebrate => "EnDntNomal_SetupStageCelebrate",
            Action::StageCelebrate => "EnDntNomal_StageCelebrate",
            Action::SetupStageDance => "EnDntNomal_SetupStageDance",
            Action::StageDance => "EnDntNomal_StageDance",
            Action::SetupStageHide => "EnDntNomal_SetupStageHide",
            Action::StageHide => "EnDntNomal_StageHide",
            Action::StageAttackHide => "EnDntNomal_StageAttackHide",
            Action::SetupStageAttack => "EnDntNomal_SetupStageAttack",
            Action::StageAttack => "EnDntNomal_StageAttack",
            Action::StageSetupReturn => "EnDntNomal_StageSetupReturn",
            Action::StageReturn => "EnDntNomal_StageReturn",
        }
    }
}

/// `actor.draw`, once `EnDntNomal_WaitForObject` sets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Draw {
    /// `EnDntNomal_DrawTargetScrub`.
    Target,
    /// `EnDntNomal_DrawStageScrub`.
    Stage,
}

// The animations: the target scrub's (`object_hintnuts`) and the stage scrubs' (`object_dnk`).
const BURROW: &str = "gHintNutsBurrowAnim";
const UNBURROW: &str = "gHintNutsUnburrowAnim";
const RUN: &str = "gHintNutsRunAnim";
const TALK: &str = "gHintNutsTalkAnim";
const SPIT: &str = "gHintNutsSpitAnim";
const STAGE_HIDE: &str = "gDntStageHideAnim";
const STAGE_UP: &str = "gDntStageUpAnim";
const STAGE_UNBURROW: &str = "gDntStageUnburrowAnim";
const STAGE_WALK: &str = "gDntStageWalkAnim";
const STAGE_DANCE: &str = "gDntStageDanceAnim";
const STAGE_SPIT: &str = "gDntStageSpitAnim";

pub struct EnDntNomal {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: HashMap<&'static str, Anim>,
    pub action_func: Action,
    /// `isSolid`: pushes Link (the stage scrub's `CollisionCheck_SetOC`).
    pub is_solid: bool,
    pub flower_pos: Vec3,
    pub target_vtx: [Vec3; 4],
    pub timer1: i16,
    pub timer2: i16,
    pub timer4: i16,
    pub timer5: i16,
    pub blink_timer: i16,
    pub unk_counter: i16,
    pub timer3: i16,
    pub object_id: i16,
    pub eye_state: i16,
    pub ty: i16,
    pub hit_counter: i16,
    /// `endFrame`: an s16, the current animation's last frame.
    pub end_frame: i16,
    /// `stageSignal`, `action`, `ignore`, `stagePrize`, `targetPos`: written by `En_Dnt_Demo`.
    pub stage_signal: i16,
    pub rot_direction: i16,
    pub action: i16,
    pub ignore: u8,
    pub spawned_item: bool,
    pub stage_prize: u8,
    pub required_object_slot: Option<usize>,
    /// `mouthPos`: limb 5's (the target scrub) or limb 7's (a stage scrub) origin, from the last
    /// draw (`EnDntNomal_PostLimbDraw`).
    pub mouth_pos: Vec3,
    pub target_pos: Vec3,
    pub target_collider_quad: ColliderQuad,
    pub body_collider_cylinder: ColliderCylinder,
    pub draw: Option<Draw>,
}

impl EnDntNomal {
    /// `EnDntNomal_Init`: the type (from the params, below 0 the target), untargetable and
    /// immovable; the target scrub with its quad, facing Link (`yawTowardsPlayer`, 0 before the
    /// first update), waiting for `object_hintnuts`; a stage scrub with its cylinder, for
    /// `object_dnk`. Without its object in a bank, gone.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut ty = TYPE.get(actor.params);
        if ty < ENDNTNOMAL_TARGET {
            ty = ENDNTNOMAL_TARGET;
        }
        actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        actor.col_chk_info.mass = MASS_IMMOVABLE;
        let object_id;
        let mut target_collider_quad = ColliderQuad::default();
        let mut body_collider_cylinder = ColliderCylinder::default();
        if ty == ENDNTNOMAL_TARGET {
            log::debug!("☆☆☆☆☆ Deku Scrub target ☆☆☆☆☆");
            target_collider_quad = ColliderQuad::new(&TARGET_QUAD_INIT);
            actor.shape_rot.y = actor.yaw_towards_player;
            actor.world_rot.y = actor.shape_rot.y;
            object_id = OBJECT_HINTNUTS;
        } else {
            log::debug!("☆☆☆☆☆ Deku Scrub mask competition audience ☆☆☆☆☆");
            body_collider_cylinder = ColliderCylinder::new(&BODY_CYLINDER_INIT);
            object_id = OBJECT_DNK;
        }
        let (file, anims): (&str, &[&'static str]) =
            if ty == ENDNTNOMAL_TARGET { (HINTNUTS, &[BURROW, UNBURROW, RUN, TALK, SPIT]) } else { (DNK, &[STAGE_HIDE, STAGE_UP, STAGE_UNBURROW, STAGE_WALK, STAGE_DANCE, STAGE_SPIT]) };
        let mut loaded = HashMap::new();
        let mut skeleton = None;
        if let Some(a) = play.assets.clone() {
            for &n in anims {
                match a.animation(file, n) {
                    Ok(anim) => {
                        loaded.insert(n, anim);
                    }
                    Err(e) => log::error!("En_Dnt_Nomal: {e:#}"),
                }
            }
            let skel = if ty == ENDNTNOMAL_TARGET { "gHintNutsSkel" } else { "gDntStageSkel" };
            skeleton = a.skeleton(file, skel).map_err(|e| log::error!("En_Dnt_Nomal: {e:#}")).ok();
        }
        let required_object_slot = play.object_ctx.get_index(object_id);
        let mut this = EnDntNomal {
            actor,
            skel: SkelAnimeStd::init_flex(0, None),
            skeleton,
            anims: loaded,
            action_func: Action::WaitForObject,
            is_solid: false,
            flower_pos: Vec3::ZERO,
            target_vtx: [Vec3::ZERO; 4],
            timer1: 0,
            timer2: 0,
            timer4: 0,
            timer5: 0,
            blink_timer: 0,
            unk_counter: 0,
            timer3: 0,
            object_id,
            eye_state: 0,
            ty,
            hit_counter: 0,
            end_frame: 0,
            stage_signal: DNT_SIGNAL_NONE,
            rot_direction: 0,
            action: DNT_ACTION_NONE,
            ignore: 0,
            spawned_item: false,
            stage_prize: DNT_PRIZE_NONE,
            required_object_slot,
            mouth_pos: Vec3::ZERO,
            target_pos: Vec3::ZERO,
            target_collider_quad,
            body_collider_cylinder,
            draw: None,
        };
        if this.required_object_slot.is_none() {
            this.actor.kill();
            log::debug!("En_Dnt_Nomal: What? The bank is weird! (object {:#06x})", this.object_id);
            return Box::new(this);
        }
        this.action_func = Action::WaitForObject;
        Box::new(this)
    }

    fn anim(&self, name: &str) -> Option<Anim> {
        self.anims.get(name).cloned()
    }

    /// `(f32)Animation_GetLastFrame(anim)` into the s16 `endFrame`.
    fn last_frame(&self, name: &str) -> i16 {
        self.anims.get(name).map(|a| a.last_frame()).unwrap_or(0.0) as i16
    }

    /// `this->endFrame = Animation_GetLastFrame(anim)`, then `Animation_Change(&skelAnime, anim,
    /// speed, 0, endFrame, mode, -10)`.
    fn change_to_end(&mut self, name: &str, speed: f32, mode: u8) {
        self.end_frame = self.last_frame(name);
        if let Some(a) = self.anim(name) {
            self.skel.change(a, speed, 0.0, self.end_frame as f32, mode, -10.0);
        }
    }

    /// `Actor_PlaySfx(&this->actor, NA_SE_EN_NUTS_WALK)` on frames 0 and 6 of the run.
    fn walk_sfx(&self, play: &mut PlayState) {
        if self.skel.on_frame(0.0) || self.skel.on_frame(6.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_WALK);
        }
    }

    /// `EffectSsHahen_SpawnBurst(play, pos, 4.0f, 0, 10, 3, 15, HAHEN_OBJECT_DEFAULT, 10, NULL)`.
    fn burst(play: &mut PlayState, pos: Vec3) {
        play.with_ss(|s| s.hahen_spawn_burst(pos, 4.0, 0, 10, 3, 15, HAHEN_OBJECT_DEFAULT, 10, None));
    }

    /// `RAD_TO_BINANG(Math_FAtan2F(dx, dz))`.
    fn yaw_to(dx: f32, dz: f32) -> i16 {
        rad_to_binang(f_atan2f(dx, dz))
    }

    /// `Actor_ChangeCategory` of itself (taken out of its slot while it updates: its own
    /// `category` and the arena's list, as `En_Hintnuts` does).
    fn change_category(&mut self, play: &mut PlayState, category: usize) {
        self.actor.category = category;
        if let Some(me) = play.cur_actor {
            play.actors.change_category(me, category);
        }
    }

    /// `EnDntNomal_WaitForObject`: once its object is in, shown (`draw`), gravity -2, scale 0.01,
    /// its skeleton with the burrow (target) or hide (stage) animation looped; then
    /// `EnDntNomal_SetFlower`.
    fn wait_for_object(&mut self, play: &mut PlayState) {
        let Some(slot) = self.required_object_slot else { return };
        if play.object_ctx.is_loaded(slot) {
            self.actor.obj_bank_index = Some(slot);
            self.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 0.0);
            self.actor.gravity = -2.0;
            self.actor.scale = Vec3::splat(0.01);
            let limbs = self.skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(if self.ty == ENDNTNOMAL_TARGET { 9 } else { 10 });
            if self.ty == ENDNTNOMAL_TARGET {
                // @bug (game): gHintNutsSkel is a flex skeleton, set up with SkelAnime_Init (10
                // joints) as a normal one.
                self.skel = SkelAnimeStd::init_flex(limbs, self.anim(BURROW));
                self.draw = Some(Draw::Target);
            } else {
                // SkelAnime_Init(&gDntStageSkel, &gDntStageHideAnim): 11 joints.
                self.skel = SkelAnimeStd::init_flex(limbs, self.anim(STAGE_HIDE));
                self.draw = Some(Draw::Stage);
            }
            self.action_func = Action::SetFlower;
        }
    }

    /// `EnDntNomal_SetFlower`: landed, its flower where it stands.
    fn set_flower(&mut self) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.flower_pos = self.actor.world_pos;
            self.action_func = if self.ty == ENDNTNOMAL_TARGET { Action::SetupTargetWait } else { Action::SetupStageWait };
        }
    }

    /// `EnDntNomal_SetupTargetWait`: in the ground, the burrow held at frame 8.
    fn setup_target_wait(&mut self) {
        self.change_to_end(BURROW, 0.0, ANIMMODE_ONCE);
        self.skel.cur_frame = 8.0;
        self.action_func = Action::TargetWait;
    }

    /// `EnDntNomal_TargetWait`: the target's quad (48 square at x 1340 around y 50, z -30); a hit
    /// (or `BREG(0)`) within 8 of its centre (`hitPos`) is a bullseye: `EXTRA_SCORE_100` 20 to
    /// the scrub's -x and 20 up, the chime; as a child without `ITEMGETINF_1D` it counts, and the
    /// third starts one-point cutscene 4140 with Player held, the scrub up 50 frames later.
    /// Within 24, `EXTRA_SCORE_30` 20 up, and the count starts over.
    fn target_wait(&mut self, play: &mut PlayState) {
        let (target_x, target_y, target_z) = (TARGET_POS.x, TARGET_POS.y, TARGET_POS.z);
        let score_accel = Vec3::ZERO;
        let mut score_vel = Vec3::ZERO;
        self.target_vtx[0] = Vec3::new(target_x, target_y - 24.0, target_z + 24.0);
        self.target_vtx[1] = Vec3::new(target_x, target_y - 24.0, target_z - 24.0);
        self.target_vtx[2] = Vec3::new(target_x, target_y + 24.0, target_z + 24.0);
        self.target_vtx[3] = Vec3::new(target_x, target_y + 24.0, target_z - 24.0);
        self.skel.update();
        if self.target_collider_quad.base.ac_flags & AC_HIT != 0 || BREG_0 != 0 {
            self.target_collider_quad.base.ac_flags &= !AC_HIT;
            let hit = self.target_collider_quad.info.ac_dmg_info.hit_pos;
            let dx = (target_x - hit[0] as f32).abs();
            let dy = (target_y - hit[1] as f32).abs();
            let dz = (target_z - hit[2] as f32).abs();
            score_vel.y = 5.0;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist < 8.0 {
                let score_pos = Vec3::new(self.actor.world_pos.x - 20.0, self.actor.world_pos.y + 20.0, self.actor.world_pos.z);
                play.with_ss(|s| s.extra_spawn(score_pos, score_vel, score_accel, 4, EXTRA_SCORE_100));
                play.audio.stop_sfx_by_id(NA_SE_SY_TRE_BOX_APPEAR);
                play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
                log::debug!("☆☆☆☆☆ Big hit ☆☆☆☆☆ {}", self.hit_counter);
                if !play.save.adult && !play.save.get_item_get_inf(ITEMGETINF_1D) {
                    self.hit_counter += 1;
                    if self.hit_counter >= 3 {
                        let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
                        play.onepoint_cutscene_init(4140, -99, me, CAM_ID_MAIN);
                        let cur = play.cur_actor;
                        play.player_set_cs_action_with_halted_actors(cur, PLAYER_CSACTION_1);
                        self.timer4 = 50;
                        self.action_func = Action::SetupTargetUnburrow;
                    }
                }
            } else if dist < 24.0 {
                let score_pos = Vec3::new(self.actor.world_pos.x, self.actor.world_pos.y + 20.0, self.actor.world_pos.z);
                play.with_ss(|s| s.extra_spawn(score_pos, score_vel, score_accel, 4, EXTRA_SCORE_30));
                self.hit_counter = 0;
            }
        }
    }

    /// `EnDntNomal_SetupTargetUnburrow`: its 50 frames out, up out of the ground (a burst of
    /// fragments 50 up, `NA_SE_EN_NUTS_UP`).
    fn setup_target_unburrow(&mut self, play: &mut PlayState) {
        if self.timer4 == 0 {
            self.change_to_end(UNBURROW, 1.0, ANIMMODE_ONCE);
            let mut spawn_pos = self.actor.world_pos;
            spawn_pos.y = self.actor.world_pos.y + 50.0;
            Self::burst(play, spawn_pos);
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_UP);
            self.action_func = Action::TargetUnburrow;
        }
    }

    /// `EnDntNomal_TargetUnburrow`.
    fn target_unburrow(&mut self) {
        let frame = self.skel.cur_frame;
        self.skel.update();
        if frame >= self.end_frame as f32 {
            self.action_func = Action::SetupTargetWalk;
        }
    }

    /// `EnDntNomal_SetupTargetWalk`: running at 1, no longer immovable.
    fn setup_target_walk(&mut self) {
        self.change_to_end(RUN, 1.0, ANIMMODE_LOOP);
        self.actor.speed_xz = 1.0;
        self.actor.col_chk_info.mass = 0;
        self.action_func = Action::TargetWalk;
    }

    /// `EnDntNomal_TargetWalk`: towards (1343, 0) until past z -30, under the target.
    fn target_walk(&mut self, play: &mut PlayState) {
        self.skel.update();
        let dx = 1340.0 + 3.0 - self.actor.world_pos.x;
        let dz = 0.0 - self.actor.world_pos.z;
        smooth_step_to_s(&mut self.actor.shape_rot.y, Self::yaw_to(dx, dz), 0x32, 0xBB8, 0);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.walk_sfx(play);
        if self.actor.world_pos.z > -30.0 {
            self.actor.speed_xz = 0.0;
            self.action_func = Action::TargetFacePlayer;
        }
    }

    /// `EnDntNomal_TargetFacePlayer`: turning to Link, until within 30 (the s16s' difference,
    /// unwrapped).
    fn target_face_player(&mut self, play: &mut PlayState) {
        self.skel.update();
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 3, 0x1388, 0);
        self.walk_sfx(play);
        if ((self.actor.shape_rot.y as i32 - self.actor.yaw_towards_player as i32) as f32).abs() < 30.0 {
            self.action_func = Action::SetupTargetTalk;
        }
    }

    /// `EnDntNomal_SetupTargetTalk`: text 0x10AF.
    fn setup_target_talk(&mut self, play: &mut PlayState) {
        self.change_to_end(TALK, 1.0, ANIMMODE_LOOP);
        self.actor.text_id = 0x10AF;
        play.start_textbox(self.actor.text_id, None);
        self.action_func = Action::TargetTalk;
    }

    /// `EnDntNomal_TargetTalk`: the text's event: closed, the cutscene's camera finished (its
    /// `csId` 0), Player `PLAYER_CSACTION_8`; the spit.
    fn target_talk(&mut self, play: &mut PlayState) {
        self.skel.update();
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            play.with_msg(|m, f| m.close_textbox(f.audio));
            let cam = play.active_cam_id;
            play.camera_set_finished_flag(cam);
            if let Some(c) = play.camera_mut(cam) {
                c.cs_id = 0;
            }
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
            self.action_func = Action::SetupTargetGivePrize;
        }
    }

    /// `EnDntNomal_SetupTargetGivePrize`.
    fn setup_target_give_prize(&mut self) {
        self.change_to_end(SPIT, 1.0, ANIMMODE_ONCE);
        self.action_func = Action::TargetGivePrize;
    }

    /// `EnDntNomal_TargetGivePrize`: from frame 8, once, the bullet bag (`En_Ex_Item`
    /// `EXITEM_BULLET_BAG`) as its child at its mouth, 10 to -x, `NA_SE_EN_NUTS_THROW` (if it
    /// can't spawn: Player `PLAYER_CSACTION_7`, and the scrub gone); the spit done, running home.
    fn target_give_prize(&mut self, play: &mut PlayState) {
        let frame = self.skel.cur_frame;
        self.skel.update();
        if frame >= 8.0 && !self.spawned_item {
            let pos = Vec3::new(self.mouth_pos.x - 10.0, self.mouth_pos.y, self.mouth_pos.z);
            if play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_EX_ITEM, pos, [0, 0, 0], EXITEM_BULLET_BAG).is_err() {
                play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_7);
                self.actor.kill();
            }
            self.spawned_item = true;
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_THROW);
        }
        if frame >= self.end_frame as f32 {
            self.change_to_end(RUN, 1.0, ANIMMODE_LOOP);
            self.action_func = Action::TargetReturn;
        }
    }

    /// `EnDntNomal_TargetReturn`: turning to (flower x, -180), running at 1 once within 20 of
    /// it; past z -172, at -173, burrowing.
    fn target_return(&mut self, play: &mut PlayState) {
        self.skel.update();
        let dx = self.flower_pos.x - self.actor.world_pos.x;
        let dz = -180.0 - self.actor.world_pos.z;
        smooth_step_to_s(&mut self.actor.shape_rot.y, Self::yaw_to(dx, dz), 3, 0x1388, 0);
        if ((self.actor.shape_rot.y as i32 - Self::yaw_to(dx, dz) as i32) as f32).abs() < 20.0 {
            self.actor.speed_xz = 1.0;
        }
        self.walk_sfx(play);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        if self.actor.world_pos.z < -172.0 {
            self.change_to_end(BURROW, 1.0, ANIMMODE_ONCE);
            self.actor.world_pos.z = -173.0;
            self.actor.speed_xz = 0.0;
            self.action_func = Action::TargetBurrow;
        }
    }

    /// `EnDntNomal_TargetBurrow`: down, then waiting again (`!OOT_PAL_N64`; the count and
    /// `spawnedItem` as they were).
    fn target_burrow(&mut self) {
        let frame = self.skel.cur_frame;
        self.skel.update();
        if frame >= self.end_frame as f32 {
            self.action_func = Action::SetupTargetWait;
        }
    }

    /// `EnDntNomal_SetupStageWait`: its `timer3` out, in the ground (the hide held at frame 8),
    /// not pushing.
    fn setup_stage_wait(&mut self) {
        if self.timer3 == 0 {
            self.change_to_end(STAGE_HIDE, 0.0, ANIMMODE_ONCE);
            self.skel.cur_frame = 8.0;
            self.is_solid = false;
            self.action_func = Action::StageWait;
        }
    }

    /// `EnDntNomal_StageWait`.
    fn stage_wait(&mut self) {
        self.skel.update();
    }

    /// `EnDntNomal_SetupStageUp`: its `timer3` out, up (a burst, `NA_SE_EN_NUTS_UP`), pushing;
    /// `rotDirection` -1 but for an attack.
    fn setup_stage_up(&mut self, play: &mut PlayState) {
        if self.timer3 == 0 {
            self.change_to_end(STAGE_UP, 1.0, ANIMMODE_ONCE);
            if self.action != DNT_ACTION_ATTACK {
                self.rot_direction = -1;
            }
            Self::burst(play, self.actor.world_pos);
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_UP);
            self.is_solid = true;
            self.action_func = Action::StageUp;
        }
    }

    /// `EnDntNomal_StageUp`: up for an attack, the attack; else turning to Link (with a prize:
    /// to `targetPos`, and less often looking about), now and then looking 0x1388 to either side
    /// (`timer4`, `timer2`); Link within 70, hiding.
    fn stage_up(&mut self, play: &mut PlayState) {
        let frame = self.skel.cur_frame;
        self.skel.update();
        if frame >= self.end_frame as f32 && self.action == DNT_ACTION_ATTACK {
            self.action_func = Action::SetupStageAttack;
        } else {
            if self.timer4 == 0 {
                let mut turn_mod = 0.0;
                if self.stage_prize == DNT_PRIZE_NONE {
                    smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 3, 0x1388, 0);
                } else {
                    let dx = self.target_pos.x - self.actor.world_pos.x;
                    let dz = self.target_pos.z - self.actor.world_pos.z;
                    smooth_step_to_s(&mut self.actor.shape_rot.y, Self::yaw_to(dx, dz), 1, 0xBB8, 0);
                    turn_mod = 90.0;
                }
                if play.rand.zero_float(10.0 + turn_mod) < 1.0 && self.action != DNT_ACTION_ATTACK {
                    self.timer4 = play.rand.zero_float(30.0) as i16 + 30;
                }
            } else {
                if self.timer2 == 0 {
                    self.rot_direction += 1;
                    if self.rot_direction > 1 {
                        self.rot_direction = -1;
                    }
                    self.timer2 = play.rand.zero_float(10.0) as i16 + 10;
                }
                let mut rot_target = self.actor.yaw_towards_player;
                if self.rot_direction != 0 {
                    rot_target = (rot_target as i32 + self.rot_direction as i32 * 0x1388) as i16;
                }
                smooth_step_to_s(&mut self.actor.shape_rot.y, rot_target, 3, 0x1388, 0);
            }
            if self.actor.xz_dist_to_player < 70.0 {
                self.action_func = Action::SetupStageHide;
            }
        }
    }

    /// `EnDntNomal_SetupStageUnburrow`: its `timer3` out, up out of the ground, not pushing.
    fn setup_stage_unburrow(&mut self, play: &mut PlayState) {
        if self.timer3 == 0 {
            self.change_to_end(STAGE_UNBURROW, 1.0, ANIMMODE_ONCE);
            self.is_solid = false;
            Self::burst(play, self.actor.world_pos);
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_UP);
            self.action_func = Action::StageUnburrow;
        }
    }

    /// `EnDntNomal_StageUnburrow`: up, celebrating after 0 or 1 frames plus half its type, or
    /// dancing for 300.
    fn stage_unburrow(&mut self, play: &mut PlayState) {
        let frame = self.skel.cur_frame;
        self.skel.update();
        if frame >= self.end_frame as f32 {
            if self.action != DNT_ACTION_DANCE {
                self.timer3 = (play.rand.zero_float(2.0) as i16).wrapping_add((self.ty as f32 * 0.5) as i16);
                self.action_func = Action::SetupStageCelebrate;
            } else {
                self.timer2 = 300;
                self.action_func = Action::SetupStageDance;
            }
        }
    }

    /// `EnDntNomal_SetupStageCelebrate`: walking at 3, pushing.
    fn setup_stage_celebrate(&mut self) {
        self.change_to_end(STAGE_WALK, 1.0, ANIMMODE_LOOP);
        self.actor.speed_xz = 3.0;
        self.is_solid = true;
        self.action_func = Action::StageCelebrate;
    }

    /// `EnDntNomal_StageCelebrate`: with `timer1` and `timer3` out, running to `targetPos`
    /// (there, with a text up: the prize's dance); else turning to Link (`timer1` at 1 starts a
    /// `timer3` of 20 to 39). `NA_SE_EN_NUTS_DAMAGE` every 20 frames (even types), a step every
    /// 4; against a wall on the ground, a hop (7.5).
    fn stage_celebrate(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.timer1 == 0 && self.timer3 == 0 {
            let dx = self.target_pos.x - self.actor.world_pos.x;
            let dz = self.target_pos.z - self.actor.world_pos.z;
            if dx.abs() < 10.0 && dz.abs() < 10.0 && play.message_state() != TEXT_STATE_NONE {
                self.action = DNT_ACTION_PRIZE;
                self.action_func = Action::SetupStageDance;
                self.actor.speed_xz = 0.0;
                return;
            }
            smooth_step_to_s(&mut self.actor.shape_rot.y, Self::yaw_to(dx, dz), 1, 0xBB8, 0);
            self.actor.world_rot.y = self.actor.shape_rot.y;
        } else {
            if self.timer1 == 1 {
                self.timer3 = (play.rand.zero_float(20.0) as i16 as f32 + 20.0) as i16;
            }
            smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 0x14, 0x1388, 0);
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 3, 0x1388, 0);
        }
        if self.timer5 == 0 {
            self.timer5 = 20;
            if self.ty & 1 == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_NUTS_DAMAGE);
            }
        } else if self.timer5 & 3 == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_WALK);
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.actor.velocity.y = 7.5;
        }
    }

    /// `EnDntNomal_SetupStageDance`: pushing, `timer3` 20 to 39, turning either way.
    fn setup_stage_dance(&mut self, play: &mut PlayState) {
        self.change_to_end(STAGE_DANCE, 1.0, ANIMMODE_LOOP);
        self.is_solid = true;
        self.timer3 = (play.rand.zero_float(20.0) as i16 as f32 + 20.0) as i16;
        self.rot_direction = -1;
        if play.rand.zero_float(1.99) < 1.0 {
            self.rot_direction = 1;
        }
        self.action_func = Action::StageDance;
    }

    /// `EnDntNomal_StageDance`: until `timer2` is out (then the dance's high rupees, or home),
    /// turns to Link (`timer3`) and spins 0x800 a frame (`timer4`) by turns.
    fn stage_dance(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.timer2 == 0 {
            if self.action == DNT_ACTION_DANCE {
                self.action = DNT_ACTION_HIGH_RUPEES;
                self.action_func = Action::SetupStageHide;
            } else {
                self.action = DNT_ACTION_NONE;
                self.action_func = Action::StageSetupReturn;
            }
        } else if self.timer3 != 0 {
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 3, 0x1388, 0);
            if self.timer3 == 1 {
                self.timer4 = (play.rand.zero_float(20.0) as i16 as f32 + 20.0) as i16;
                self.rot_direction = -self.rot_direction;
            }
        } else if self.timer4 != 0 {
            self.actor.shape_rot.y = (self.actor.shape_rot.y as i32 + self.rot_direction as i32 * 0x800) as i16;
            if self.timer4 == 1 {
                self.timer3 = (play.rand.zero_float(20.0) as i16 as f32 + 20.0) as i16;
            }
        }
    }

    /// `EnDntNomal_SetupStageHide`: waiting out `timer3` (`NA_SE_SY_ERROR` at its last frame
    /// for an ignored scrub), then down (`NA_SE_EN_NUTS_DOWN`), not pushing.
    fn setup_stage_hide(&mut self, play: &mut PlayState) {
        if self.timer3 != 0 {
            if self.timer3 == 1 && self.ignore == 1 {
                play.audio.play_sfx_centered(NA_SE_SY_ERROR);
            }
        } else {
            self.change_to_end(STAGE_HIDE, 1.0, ANIMMODE_ONCE);
            self.is_solid = false;
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_DOWN);
            self.action_func = Action::StageHide;
        }
    }

    /// `EnDntNomal_StageHide`: down (a burst, `NA_SE_EN_NUTS_UP`): waiting; after an attack, its
    /// wait for Link to go; with rupees, one (`En_Ex_Ruppy` 3, 20 up: green or, 1 in 4, blue for
    /// the low ones; blue or red for the high ones, red jumping 7, the rest 5) and the chime.
    /// `@bug (game)`: for `DNT_ACTION_DANCE` and `_PRIZE` there's no case: it stays, and bursts
    /// and sounds again every frame.
    fn stage_hide(&mut self, play: &mut PlayState) {
        let frame = self.skel.cur_frame;
        self.skel.update();
        if frame >= self.end_frame as f32 {
            Self::burst(play, self.actor.world_pos);
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_UP);
            match self.action {
                DNT_ACTION_NONE => self.action_func = Action::SetupStageWait,
                DNT_ACTION_ATTACK => self.action_func = Action::StageAttackHide,
                DNT_ACTION_LOW_RUPEES | DNT_ACTION_HIGH_RUPEES => {
                    let pos = Vec3::new(self.actor.world_pos.x, self.actor.world_pos.y + 20.0, self.actor.world_pos.z);
                    if let Ok(h) = play.actor_spawn(ACTOR_EN_EX_RUPPY, pos, [0, 0, 0], ENEXRUPPY_TYPE_HYRULE_COURTYARD) {
                        let rupee_color = self.action - DNT_ACTION_LOW_RUPEES;
                        let plus_one = play.rand.zero_float(3.99) < 1.0;
                        if let Some(rupee) = play.actors.downcast_mut::<EnExRuppy>(h) {
                            rupee.color_idx = rupee_color;
                            if plus_one {
                                rupee.color_idx = rupee_color + 1;
                            }
                            rupee.actor.velocity.y = 5.0;
                            if rupee.color_idx == 2 {
                                rupee.actor.velocity.y = 7.0;
                            }
                        }
                        play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
                    }
                    self.action = DNT_ACTION_NONE;
                    self.action_func = Action::SetupStageWait;
                }
                _ => {}
            }
        }
    }

    /// `EnDntNomal_StageAttackHide`: Link past 70, up again.
    fn stage_attack_hide(&mut self) {
        if self.actor.xz_dist_to_player > 70.0 {
            self.action_func = Action::SetupStageUp;
        }
    }

    /// `EnDntNomal_SetupStageAttack`: its `timer3` out, the spit; immovable, pushing, an enemy.
    fn setup_stage_attack(&mut self, play: &mut PlayState) {
        if self.timer3 == 0 {
            self.change_to_end(STAGE_SPIT, 1.0, ANIMMODE_ONCE);
            self.actor.col_chk_info.mass = MASS_IMMOVABLE;
            self.is_solid = true;
            self.timer2 = 0;
            self.change_category(play, ACTORCAT_ENEMY);
            self.action_func = Action::StageAttack;
        }
    }

    /// `EnDntNomal_StageAttack`: turning and pitching to Link (30 above his feet, from its mouth);
    /// the spit done, `timer2` 10 to 19 to the next (`spawnedItem` cleared); Link within 50,
    /// hiding (still attacking); from frame 8, once, a nut (`En_Nutsball` 4) 5 in front of its
    /// mouth along its facing, half that offset's y its velocity, `NA_SE_EN_NUTS_THROW`.
    fn stage_attack(&mut self, play: &mut PlayState) {
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO);
        let frame = self.skel.cur_frame;
        self.skel.update();
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 3, 0x1388, 0);
        let dx = player_pos.x - self.mouth_pos.x;
        let dy = player_pos.y + 30.0 - self.mouth_pos.y;
        let dz = player_pos.z - self.mouth_pos.z;
        let pitch = (-(rad_to_binang(f_atan2f(dy, (dx * dx + dz * dz).sqrt())) as i32)) as i16;
        smooth_step_to_s(&mut self.actor.shape_rot.x, pitch, 3, 0x1388, 0);
        if frame >= self.end_frame as f32 && self.timer2 == 0 {
            self.timer2 = play.rand.zero_float(10.0) as i16 + 10;
        }
        if self.timer2 == 1 {
            self.spawned_item = false;
            self.action_func = Action::SetupStageAttack;
        } else if self.actor.xz_dist_to_player < 50.0 {
            self.action = DNT_ACTION_ATTACK;
            self.action_func = Action::SetupStageHide;
        } else if frame >= 8.0 && !self.spawned_item {
            // Matrix_RotateY(BINANG_TO_RAD_ALT(rot.y), MTXMODE_NEW), Matrix_RotateX(BINANG_TO_RAD_ALT(rot.x)).
            let alt = |b: i16| (b as f32 / 32768.0) * std::f32::consts::PI;
            let mut m = MtxF::IDENTITY;
            m.rotate_y(alt(self.actor.shape_rot.y));
            m.rotate_x(alt(self.actor.shape_rot.x));
            let spawn_offset = m.mult_vec3f(Vec3::new(0.0, 0.0, 5.0));
            let spawn = self.mouth_pos + spawn_offset;
            let r = self.actor.shape_rot;
            if let Ok(h) = play.actor_spawn(ACTOR_EN_NUTSBALL, spawn, [r.x, r.y, r.z], EN_NUTSBALL_TYPE_DNK)
                && let Some(nut) = play.actors.actor_mut(h)
            {
                nut.velocity.y = spawn_offset.y * 0.5;
            }
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_THROW);
            self.spawned_item = true;
        }
    }

    /// `EnDntNomal_StageSetupReturn`: walking home at 4 (the walk at 1.5), not pushing.
    fn stage_setup_return(&mut self) {
        self.change_to_end(STAGE_WALK, 1.5, ANIMMODE_LOOP);
        self.actor.speed_xz = 4.0;
        self.is_solid = false;
        self.action_func = Action::StageReturn;
    }

    /// `EnDntNomal_StageReturn`: to its flower, a step every other frame of 10; within 7 of it,
    /// on it, hiding.
    fn stage_return(&mut self, play: &mut PlayState) {
        self.skel.update();
        let sp2c = self.flower_pos.x - self.actor.world_pos.x;
        let sp28 = self.flower_pos.z - self.actor.world_pos.z;
        smooth_step_to_s(&mut self.actor.shape_rot.y, Self::yaw_to(sp2c, sp28), 1, 0xBB8, 0);
        if self.timer5 == 0 {
            self.timer5 = 10;
        } else if self.timer5 & 1 == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_NUTS_WALK);
        }
        if sp2c.abs() < 7.0 && sp28.abs() < 7.0 {
            self.actor.world_pos.x = self.flower_pos.x;
            self.actor.world_pos.z = self.flower_pos.z;
            self.actor.speed_xz = 0.0;
            self.action_func = Action::SetupStageHide;
        }
    }

    /// The `actionFunc` call.
    fn run_action(&mut self, play: &mut PlayState) {
        match self.action_func {
            Action::WaitForObject => self.wait_for_object(play),
            Action::SetFlower => self.set_flower(),
            Action::SetupTargetWait => self.setup_target_wait(),
            Action::TargetWait => self.target_wait(play),
            Action::SetupTargetUnburrow => self.setup_target_unburrow(play),
            Action::TargetUnburrow => self.target_unburrow(),
            Action::SetupTargetWalk => self.setup_target_walk(),
            Action::TargetWalk => self.target_walk(play),
            Action::TargetFacePlayer => self.target_face_player(play),
            Action::SetupTargetTalk => self.setup_target_talk(play),
            Action::TargetTalk => self.target_talk(play),
            Action::SetupTargetGivePrize => self.setup_target_give_prize(),
            Action::TargetGivePrize => self.target_give_prize(play),
            Action::TargetReturn => self.target_return(play),
            Action::TargetBurrow => self.target_burrow(),
            Action::SetupStageWait => self.setup_stage_wait(),
            Action::StageWait => self.stage_wait(),
            Action::SetupStageUp => self.setup_stage_up(play),
            Action::StageUp => self.stage_up(play),
            Action::SetupStageUnburrow => self.setup_stage_unburrow(play),
            Action::StageUnburrow => self.stage_unburrow(play),
            Action::SetupStageCelebrate => self.setup_stage_celebrate(),
            Action::StageCelebrate => self.stage_celebrate(play),
            Action::SetupStageDance => self.setup_stage_dance(play),
            Action::StageDance => self.stage_dance(play),
            Action::SetupStageHide => self.setup_stage_hide(play),
            Action::StageHide => self.stage_hide(play),
            Action::StageAttackHide => self.stage_attack_hide(),
            Action::SetupStageAttack => self.setup_stage_attack(play),
            Action::StageAttack => self.stage_attack(play),
            Action::StageSetupReturn => self.stage_setup_return(),
            Action::StageReturn => self.stage_return(play),
        }
    }

    /// The limb whose origin is `mouthPos` (`EnDntNomal_PostLimbDraw`), 1-based.
    fn mouth_limb(&self) -> usize {
        if self.ty == ENDNTNOMAL_TARGET { 5 } else { 7 }
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `switches`: the draw (0 none, 1 the target scrub's, 2 a stage scrub's), `eyeState`, and
    /// whether it celebrates (`func_80033C30`).
    pub const DRAW: usize = 0;
    pub const EYE: usize = 1;
    pub const CELEBRATE: usize = 2;
}

impl ActorImpl for EnDntNomal {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnDntNomal_Update`: the timers down; the stage scrubs' signal from `En_Dnt_Demo` (look:
    /// up, or with a prize straight to looking; celebrate: no action, movable, `timer3` 0 to 2
    /// plus half its type, up; dance: up to dance; hide; return; the unused one: a dance), then
    /// cleared; the blink; the action; moving, the floor; the target's quad (`AC`), or the stage
    /// scrub's cylinder (`OC` while solid).
    fn update(&mut self, play: &mut PlayState) {
        for t in [&mut self.timer1, &mut self.timer2, &mut self.timer3, &mut self.timer4, &mut self.timer5, &mut self.blink_timer] {
            if *t != 0 {
                *t -= 1;
            }
        }
        self.actor.world_rot.x = self.actor.shape_rot.x;
        if self.action_func != Action::StageCelebrate {
            self.actor.world_rot.y = self.actor.shape_rot.y;
        }
        self.unk_counter = self.unk_counter.wrapping_add(1);
        if self.ty != ENDNTNOMAL_TARGET {
            match self.stage_signal {
                DNT_SIGNAL_LOOK => {
                    self.action_func = if self.stage_prize == DNT_PRIZE_NONE { Action::SetupStageUp } else { Action::StageUp };
                }
                DNT_SIGNAL_CELEBRATE => {
                    self.action = DNT_ACTION_NONE;
                    self.actor.col_chk_info.mass = 0;
                    self.timer3 = (play.rand.zero_float(3.0) as i16).wrapping_add((self.ty as f32 * 0.5) as i16);
                    self.action_func = Action::SetupStageUnburrow;
                }
                DNT_SIGNAL_DANCE => {
                    self.action = DNT_ACTION_DANCE;
                    self.action_func = Action::SetupStageUnburrow;
                }
                DNT_SIGNAL_HIDE => self.action_func = Action::SetupStageHide,
                DNT_SIGNAL_RETURN => self.action_func = Action::StageSetupReturn,
                DNT_SIGNAL_UNUSED => self.action_func = Action::SetupStageDance,
                _ => {}
            }
        }
        if self.stage_signal != DNT_SIGNAL_NONE {
            self.stage_signal = DNT_SIGNAL_NONE;
        }
        if self.blink_timer == 0 {
            self.eye_state += 1;
            if self.eye_state >= 3 {
                self.eye_state = 0;
                self.blink_timer = play.rand.zero_float(60.0) as i16 + 20;
            }
        }
        self.run_action(play);
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 20.0, 20.0, 60.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        if self.ty == ENDNTNOMAL_TARGET {
            let v = self.target_vtx;
            self.target_collider_quad.set_vertices(v[0], v[1], v[2], v[3]);
            play.collision_check_set_ac(&self.actor, 0, &mut self.target_collider_quad);
        } else {
            self.body_collider_cylinder.update(&self.actor);
            if self.is_solid {
                play.collision_check_set_oc(&self.actor, 0, &mut self.body_collider_cylinder);
            }
        }
    }

    /// `EnDntNomal_PostLimbDraw`'s change to the actor: `mouthPos`, the origin of limb 5 (the
    /// target scrub) or 7 (a stage scrub) at this frame's draw.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let (Some(_), Some(skeleton)) = (self.draw, &self.skeleton) else { return };
        let joints = eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 };
        let bones = skeleton.pose(&joints);
        if let Some(b) = bones.get(self.mouth_limb() - 1) {
            self.mouth_pos = (actor_draw_matrix(&RenderState::of(&self.actor)) * *b).transform_point3(Vec3::ZERO);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let draw = match self.draw {
            None => 0,
            Some(Draw::Target) => 1,
            Some(Draw::Stage) => 2,
        };
        rs.switches = vec![draw, self.eye_state.clamp(0, 2) as u32, (self.action_func == Action::StageCelebrate) as u32];
        rs.values = vec![self.flower_pos.x, self.flower_pos.y, self.flower_pos.z];
        rs
    }

    /// `EnDntNomal_DrawTargetScrub` (`gHintNutsSkel`, then `gHintNutsFlowerDL` at `flowerPos` at
    /// 0.01) or `EnDntNomal_DrawStageScrub` (its eyes on segment 8, `gDntStageSkel` and
    /// `gDntStageFlowerDL` in its leaf colour; celebrating, `func_80033C30`'s circle at 0.25).
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(&draw), Some(&eye), Some(&celebrate)) = (rs.switches.get(rs::DRAW), rs.switches.get(rs::EYE), rs.switches.get(rs::CELEBRATE)) else { return };
        let ([fx, fy, fz], Some(joints), Some(skeleton)) = (rs.values.as_slice(), &rs.joints, &self.skeleton) else { return };
        let bones = skeleton.pose(joints);
        let model = actor_draw_matrix(rs);
        let flower = Mat4::from_translation(Vec3::new(*fx, *fy, *fz)) * Mat4::from_scale(Vec3::splat(0.01));
        match draw {
            1 => {
                out.opa.push(DrawCmd { mesh: MeshKey::named(keys::mesh(HINTNUTS, "gHintNutsSkel")), transform: model, bones, params: Default::default() });
                out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(HINTNUTS, "gHintNutsFlowerDL")), flower));
            }
            2 => {
                let color = LEAF_COLORS.get((self.ty - ENDNTNOMAL_STAGE) as usize).copied().unwrap_or([255; 4]);
                let mut sv = SegmentValues::default();
                sv.env[SEG_ENV as usize] = Some([color[0], color[1], color[2], 255]);
                let params = || DrawParams { segments: Some(sv.clone()), ..Default::default() };
                let eye = (eye as usize).min(BLINK_TEX.len() - 1);
                out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&stage_bake(eye))), transform: model, bones, params: params() });
                out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(STAGE_FLOWER_BAKE)), transform: flower, bones: Vec::new(), params: params() });
                if celebrate != 0 {
                    // func_80033C30(&world.pos, dustScale { 0.25, 0.25, 0.25 }, 255).
                    out.opa.push(oot_game::shadow::func_80033c30(&play.col, rs.pos, Vec3::splat(0.25), 255));
                }
            }
            _ => {}
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        if id != 0 {
            return None;
        }
        if self.ty == ENDNTNOMAL_TARGET { Some(ColliderMut::Quad(&mut self.target_collider_quad)) } else { Some(ColliderMut::Cylinder(&mut self.body_collider_cylinder)) }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
