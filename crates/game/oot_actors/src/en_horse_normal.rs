//! `En_Horse_Normal` (`ovl_En_Horse_Normal/z_en_horse_normal.c`): the horses that can't be ridden:
//! Lon Lon Ranch's, the stable's by night and the Gerudo Fortress's. A skin skeleton
//! (`gHorseNormalSkel`, `oot_game::skin`) playing nine animations (`sAnimations`: idle, whinny,
//! refuse, rearing, walking, trotting, galloping, jumping, jumping high) through the skin's
//! SkelAnime. They never notice Link.
//!
//! **The init** picks by the scene. On the ranch (`SCENE_LON_LON_RANCH`) a horse is there only by
//! day and by its `world.rot.z`: for a child 1 before Talon is back from the castle
//! (`EVENTCHKINF_TALON_RETURNED_FROM_CASTLE`), 3 after; for an adult 5 before Epona is his
//! (`EVENTCHKINF_EPONA_OBTAINED`, or the debug `R_DEBUG_FORCE_EPONA_OBTAINED`, 0 here), 7 after; 0
//! never; its rotation's z then zeroed. The two at (-730, 0, -1100) and (880, 0, -1170) stand with
//! a clone (`func_80A6C6B0`, `HORSE_WAIT_CLONE`). In the stable (`SCENE_STABLE`) every horse is
//! there by night only, each with a clone. In the Gerudo Fortress (`SCENE_GERUDOS_FORTRESS`) the one
//! at (3707, 1413, -665) stands (`func_80A6C4CC`, `HORSE_WAIT`). Any other with `params & 0xF0`
//! 0x10 and a path (`params & 0xF` not 0xF) gallops along it (`func_80A6B91C`,
//! `HORSE_FOLLOW_PATH`); the rest wander (`func_80A6BC48`, `HORSE_WANDER`).
//!
//! - `EnHorseNormal_FollowPath`: at speed 7, turning (`Math_SmoothStepToS`, 1/10, at most 0x7D0) to
//!   the next point of `play->pathList[params & 0xF]`, the next one within √600 across the
//!   ground, round and round; the gallop at speed / 5 × 1.5, `NA_SE_EV_HORSE_RUN` each loop.
//! - `EnHorseNormal_Wander`: standing (idle or whinny), or at a 1 in 4 chance at each standing
//!   animation's end walking off; walking, the speed drifting by `unk_218` (a new drift at a 1 in
//!   10 chance) to trot from 3, gallop from 6, at most 8, back to standing at 0; the yaw's turn
//!   rate `unk_21C` drifting by `unk_21E` (a new one at a 1 in 10 chance, or at a wall or an OC hit
//!   with none), within ±300, stopped at a 1 in 4 chance under 100.
//! - `EnHorseNormal_Wait`, `EnHorseNormal_WaitClone`: standing, at each animation's end idling
//!   (0.4), whinnying (0.4, `NA_SE_EV_HORSE_GROAN`) or rearing (0.2, `NA_SE_EV_HORSE_NEIGH`).
//! - `EnHorseNormal_CycleAnimations`: every animation in turn (reached by nothing after the init).
//!
//! Their sounds: the walk's hooves (`func_80A6B250`, frames 0 and 16, `NA_SE_EV_HORSE_WALK`), the
//! idle's sand (`NA_SE_EV_HORSE_SANDDUST` past 28) and the rearing's landing
//! (`NA_SE_EV_HORSE_LAND2` past 25) (`func_80A6BD7C`), the gallop's and trot's `NA_SE_EV_HORSE_RUN`
//! (`func_80A6BCEC`). Every frame: `Actor_MoveXZGravity`, the floor and walls (20, 35, 100), on the
//! ranch z kept at -2400 or more, the focus 70 up, the body's cylinder (OC), immovable while
//! standing (`MASS_IMMOVABLE`, else `MASS_HEAVY`).
//!
//! **The draw** (`EnHorseNormal_Draw`): the ground 30 ahead (`func_80A6C8E0`, every scene: the test
//! `sceneId != RANCH || sceneId != STABLE` is always true, @bug (game)) pitches it
//! (`shape.rot.x`); the skin (`func_800A6330`, `setTranslation` true); the post-draw puts the
//! head's sphere on limb 11 (`Skin_GetLimbPos`) and registers it for OC (@bug (game), as
//! `EnHorse_SkinCallback1`'s: after the frame's check). A waiting clone's horse is drawn twice: the
//! clone at its fixed place (the stable's (235, 0, 100) and (478, 0, 100) facing 0x7FFF, the
//! ranch's (780, 0, -80) and (-1000, 0, -70) facing 0), with the same pose
//! (`func_800A63CC` with `SKIN_DRAW_FLAG_CUSTOM_TRANSFORMS | SKIN_DRAW_FLAG_CUSTOM_MATRIX`), its
//! own sounds (`func_80A6CC88`), its OC cylinder (radius 60) and its shadow (`gHorseShadowDL`, XLU,
//! `Gfx_SetupDL_44Xlu`, black).
//!
//! The whole overlay is ported (`D_80A6D428`, `D_80A6D470`, two unused path tables, aren't).
//! Not ported: the horse's own shadow (`ActorShadow_DrawHorse`, not ported for any horse). The
//! sounds the C plays at positions of its own (`unk_204`: `projectedPos` 120 up; the clone's
//! `unk_1E8` and `unk_1F4`) play at the actor's `projectedPos` (the port's sound positions are
//! the actors', the sources' and the effects').
//!
//! Params: `path` bits 0 to 3 (`PARAMS_GET_U(params, 0, 4)`: the scene layer's path; 15 none;
//! read only with 0x10 in bits 4 to 7), `params & 0xF0` bits 4 to 7 (0x10 with a path: it gallops
//! the path; else it wanders), bits 8 to 15 unused (0xFF in the placements of 0xFFFF).
//! `world.rot.z`: on the ranch which of the ranch's horses it is (1 a child's before Talon's
//! return, 3 after; 5 an adult's before Epona, 7 after; 0 never), zeroed there by the init;
//! elsewhere read by nothing (the stable's are placed 170, and drawn with it).
//! Flags: reads EVENTCHKINF_TALON_RETURNED_FROM_CASTLE (the ranch, a child) and
//! EVENTCHKINF_EPONA_OBTAINED (the ranch, an adult); the debug `R_DEBUG_FORCE_EPONA_OBTAINED`
//! (`DREG(1)`, 0).

use std::sync::Arc;

use eng_anim::anim::JointTable;
use eng_collision::math3d::{Cylinder16, Sphere16};
use eng_gfx::{DrawCmd, DrawParams, MeshKey};
use eng_math::{binang_to_rad, cos_s, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_WALL, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::audio::sfx::{NA_SE_EV_HORSE_GROAN, NA_SE_EV_HORSE_LAND2, NA_SE_EV_HORSE_NEIGH, NA_SE_EV_HORSE_SANDDUST};
use oot_game::camera::f_atan2f;
use oot_game::collision_check::*;
use oot_game::event_chk_inf;
use oot_game::gbi::setup_dl;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::path::rad_to_binang_trunc;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_ONCE, Anim, SkelAnimeStd};
use oot_game::skin::{SkinPlace, SkinSkeleton, apply_anim_transformations, get_limb_pos, set_translate_rotate_yxz_scale, skin_bones, to_mat4};

/// `ACTOR_EN_HORSE_NORMAL` (`actor_table.h`: 0x003C).
pub const ACTOR_EN_HORSE_NORMAL: i16 = 0x003C;
/// `OBJECT_HORSE_NORMAL` (`object_table.h`: 0x0027).
pub const OBJECT: &str = "object_horse_normal";

/// `En_Horse_Normal_Profile` (`FLAGS` 0).
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_HORSE_NORMAL, name: "En_Horse_Normal", category: ACTORCAT_BG, flags: 0, object: OBJECT, schema: &SCHEMA };

// `SCENE_*` (`scene_table.h`).
pub const SCENE_STABLE: u16 = 0x36;
pub const SCENE_GERUDOS_FORTRESS: u16 = 0x5D;
pub const SCENE_LON_LON_RANCH: u16 = 0x63;

// `EVENTCHKINF_*` (`save.h`).
pub const EVENTCHKINF_TALON_RETURNED_FROM_CASTLE: u16 = 0x14;
pub const EVENTCHKINF_EPONA_OBTAINED: u16 = 0x18;
/// `R_DEBUG_FORCE_EPONA_OBTAINED` (`regs.h`: `DREG(1)`): the debug registers start zeroed.
const R_DEBUG_FORCE_EPONA_OBTAINED: i16 = 0;

/// `PARAMS_GET_U(params, 0, 4)`: the path (15: none).
pub const PATH: Field = Field::params("path", 0, 4, Kind::PathIndex, "the scene layer's path it gallops (15: none)").when("params & 0xF0", &[1]);
/// The same bits without `params & 0xF0` 0x10: nothing reads them.
pub const PATH_UNUSED: Field = Field::params("params & 0xF", 0, 4, Kind::Unused, "without 0x10 in bits 4 to 7: read by nothing (the Gerudo Fortress's cutscene layer places 1)").when("params & 0xF0", &[0, 15]);
/// `PARAMS_GET_NOSHIFT(params, 4, 4)`: 0x10 gallops the path.
pub const FOLLOW: Field = Field::params(
    "params & 0xF0",
    4,
    4,
    Kind::Named(&[(0, "wander"), (1, "gallop the path (0x10)"), (15, "wander (placed 0xFFFF)")]),
    "0x10 with a path: it gallops the path; else it wanders (the ranch's clones, the stable's and the Gerudo Fortress's standing one by their places)",
);
/// Bits 8 to 15: nothing reads them.
pub const HIGH: Field = Field::params("params >> 8", 8, 8, Kind::Unused, "0xFF in the placements of 0xFFFF; read by nothing");
/// `world.rot.z`: on the ranch, which horse it is.
pub const ROT_Z: Field = Field::rot(
    "rot.z",
    Source::RotZ,
    0,
    16,
    Kind::Named(&[
        (0, "never on the ranch"),
        (1, "the ranch: a child's, before Talon's return"),
        (3, "the ranch: a child's, after Talon's return"),
        (5, "the ranch: an adult's, before Epona"),
        (7, "the ranch: an adult's, after Epona"),
        (170, "the stable's (read by nothing there: drawn as its roll)"),
    ]),
    "on the ranch which of its horses it is (by age and the flags), zeroed there; elsewhere a roll",
);

/// En_Horse_Normal's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PATH, PATH_UNUSED, FOLLOW, HIGH, ROT_Z],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE)).reads().note("the ranch, a child: rot.z 1 before, 3 after"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_EPONA_OBTAINED)).reads().note("the ranch, an adult: rot.z 5 before, 7 after"),
        FlagUse::new(Flag::Other("R_DEBUG_FORCE_EPONA_OBTAINED")).reads().note("the debug register DREG(1): Epona's as obtained (0)"),
    ],
    todo: false,
};

// `NA_SE_EV_HORSE_WALK`, `NA_SE_EV_HORSE_RUN` (`environmentbank_table.h`: 0x2803, 0x2804).
pub const NA_SE_EV_HORSE_WALK: u16 = 0x2803;
pub const NA_SE_EV_HORSE_RUN: u16 = 0x2804;

/// `sAnimations`.
pub const ANIMATIONS: [&str; 9] = [
    "gHorseNormalIdleAnim",
    "gHorseNormalWhinnyAnim",
    "gHorseNormalRefuseAnim",
    "gHorseNormalRearingAnim",
    "gHorseNormalWalkingAnim",
    "gHorseNormalTrottingAnim",
    "gHorseNormalGallopingAnim",
    "gHorseNormalJumpingAnim",
    "gHorseNormalJumpingHighAnim",
];
const SKEL: &str = "gHorseNormalSkel";

/// `EnHorseNormalAction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `HORSE_CYCLE_ANIMATIONS`: `EnHorseNormal_CycleAnimations`.
    CycleAnimations,
    /// `HORSE_WANDER`: `EnHorseNormal_Wander`.
    Wander,
    /// `HORSE_WAIT`: `EnHorseNormal_Wait`.
    Wait,
    /// `HORSE_WAIT_CLONE`: `EnHorseNormal_WaitClone`.
    WaitClone,
    /// `HORSE_FOLLOW_PATH`: `EnHorseNormal_FollowPath`.
    FollowPath,
}

/// A cylinder's init (`sCylinderInit1`, `sCylinderInit2`): OC only, of `radius` and height 100.
const fn cylinder_init(radius: i16) -> ColliderCylinderInit {
    ColliderCylinderInit {
        base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_NONE,
            oc_elem_flags: OCELEM_ON,
        },
        dim: Cylinder16 { radius, height: 100, y_shift: 0, pos: [0; 3] },
    }
}

/// `sCylinderInit1`: the body.
const CYLINDER_INIT1: ColliderCylinderInit = cylinder_init(40);
/// `sCylinderInit2`: the clone.
const CYLINDER_INIT2: ColliderCylinderInit = cylinder_init(60);

/// `sJntSphElementsInit`: limb 11, centre 0, radius 20, scale 100.
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    limb: 11,
    model_sphere: Sphere16 { center: [0, 0, 0], radius: 20 },
    scale: 100,
}];

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 10, cyl_radius: 35, cyl_height: 100, mass: MASS_HEAVY };

/// `D_80A6D4C8`: each animation's speed factor (`func_80A6B30C`).
const D_80A6D4C8: [f32; 9] = [1.0, 1.0, 1.0, 1.0, 1.5, 1.5, 1.5, 1.5, 1.0];
/// `D_80A6D4C0`: the walk's hoof frames (`func_80A6B250`).
const D_80A6D4C0: [i32; 2] = [0, 16];
/// `D_80A6D4F4`: the wander's next animations (only the first two are picked).
const D_80A6D4F4: [i32; 7] = [0, 1, 4, 5, 6, 2, 3];
/// `D_80A6D510`: each animation's kind in the wander: 0 standing, 1 moving, 2 and 3 neither.
const D_80A6D510: [i32; 9] = [0, 0, 2, 2, 1, 1, 1, 3, 3];

/// `shape.shadowScale` (`ActorShape_Init(.., 20)`).
const SHADOW_SCALE: f32 = 20.0;

/// The horse's mesh: the skin as `func_800A6330` draws it.
pub const BAKE: &str = "En_Horse_Normal/skin";
/// The clone's shadow: `gHorseShadowDL` after `Gfx_SetupDL_44Xlu` and
/// `gDPSetPrimColor(0, 0, 0, 0, 0, 255)`.
pub const BAKE_SHADOW: &str = "En_Horse_Normal/clone_shadow";
const SEG_SETUP: u8 = 0x08;

/// The meshes `EnHorseNormal_Draw` draws.
pub fn bakes() -> Vec<MeshBake> {
    let mut d = setup_dl::setup_dl_44();
    // G_SETPRIMCOLOR (0xFA): 0, 0, 0, 255.
    d.0.push((0xFA00_0000, 0x0000_00FF));
    d.end();
    vec![
        MeshBake { name: BAKE.into(), object: OBJECT.into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::Skin { file: OBJECT.into(), symbol: SKEL.into() } },
        MeshBake {
            name: BAKE_SHADOW.into(),
            object: "gameplay_keep".into(),
            segments: vec![(SEG_SETUP, BakeSegment::Commands(d.0))],
            prelude: vec![SEG_SETUP],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gHorseShadowDL".into())]),
        },
    ]
}

pub struct EnHorseNormal {
    pub actor: Actor,
    pub action: Action,
    /// `animationIdx`: `sAnimations`' index.
    pub animation_idx: i32,
    /// `skin.skelAnime`.
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<SkinSkeleton>,
    /// `unk_1E4`: the sounds played this animation (1 the sand, 2 the landing; the clone's 8 and
    /// 0x10) and the clone's next voice (0x20).
    pub unk_1e4: u16,
    /// `unk_1E8`: the clone's projected position (`func_80A6CC88`).
    pub unk_1e8: Vec3,
    /// `unk_1F4`: `unk_1E8` 120 up.
    pub unk_1f4: Vec3,
    /// `unk_200`: the walk's next hoof (`D_80A6D4C0`).
    pub unk_200: i32,
    /// `unk_204`: `projectedPos` 120 up (its voices').
    pub unk_204: Vec3,
    /// `unk_218`: the wander's speed drift.
    pub unk_218: f32,
    /// `unk_21C`: the wander's turn rate.
    pub unk_21c: i16,
    /// `unk_21E`: the turn rate's drift.
    pub unk_21e: i16,
    /// `unk_220`: the ground 30 ahead (`func_80A6C8E0`).
    pub unk_220: f32,
    pub body_collider: ColliderCylinder,
    pub head_collider: ColliderJntSph,
    pub clone_collider: ColliderCylinder,
    /// `waypoint`: the path's point it heads for.
    pub waypoint: i32,
    anims: Vec<Anim>,
}

impl EnHorseNormal {
    /// `EnHorseNormal_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            actor.kill();
            return oot_game::spawn::Placeholder::init(actor, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.pack.skin_skeleton(OBJECT, SKEL)?;
            let anims = ANIMATIONS.iter().map(|a| assets.animation(OBJECT, a)).collect::<anyhow::Result<Vec<_>>>()?;
            Ok((skeleton, anims))
        })();
        let (skeleton, anims) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Horse_Normal: {e:#}");
                actor.kill();
                return oot_game::spawn::Placeholder::init(actor, play);
            }
        };
        // sInitChain: cullingVolumeScale 1200, cullingVolumeDownward 300.
        actor.culling_volume_scale = 1200.0;
        actor.culling_volume_downward = 300.0;
        actor.scale = Vec3::splat(0.01);
        actor.gravity = -3.5;
        // ActorShape_Init(&shape, 0, ActorShadow_DrawHorse, 20): the shadow isn't ported.
        actor.shape_y_offset = 0.0;
        actor.speed_xz = 0.0;
        actor.focus_pos = actor.world_pos + Vec3::Y * 70.0;
        let mut this = EnHorseNormal {
            actor,
            action: Action::CycleAnimations,
            animation_idx: 0,
            // Skin_Init happens below, in the scene's branch (here before it: the actor's struct).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton: Arc::new(skeleton),
            unk_1e4: 0,
            unk_1e8: Vec3::ZERO,
            unk_1f4: Vec3::ZERO,
            unk_200: 0,
            unk_204: Vec3::ZERO,
            unk_218: 0.0,
            unk_21c: 0,
            unk_21e: 0,
            unk_220: 0.0,
            body_collider: ColliderCylinder::new(&CYLINDER_INIT1),
            head_collider: ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS),
            clone_collider: ColliderCylinder::new(&CYLINDER_INIT2),
            waypoint: 0,
            anims,
        };
        this.actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);

        let p = this.actor.world_pos;
        if play.scene_id == SCENE_LON_LON_RANCH {
            let rot_z = ROT_Z.get(this.actor.world_rot.z);
            if rot_z == 0 || !play.save.is_day() {
                this.actor.kill();
                return Box::new(this);
            }
            if !play.save.adult {
                if play.save.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE) {
                    if rot_z != 3 {
                        this.actor.kill();
                        return Box::new(this);
                    }
                } else if rot_z != 1 {
                    this.actor.kill();
                    return Box::new(this);
                }
            } else if play.save.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) || R_DEBUG_FORCE_EPONA_OBTAINED != 0 {
                if rot_z != 7 {
                    this.actor.kill();
                    return Box::new(this);
                }
            } else if rot_z != 5 {
                this.actor.kill();
                return Box::new(this);
            }
            this.actor.home_rot.z = 0;
            this.actor.world_rot.z = 0;
            this.actor.shape_rot.z = 0;
            this.skin_init();
            if (p.x == -730.0 && p.y == 0.0 && p.z == -1100.0) || (p.x == 880.0 && p.y == 0.0 && p.z == -1170.0) {
                this.func_80a6c6b0();
                return Box::new(this);
            }
        } else if play.scene_id == SCENE_STABLE {
            if play.save.is_day() {
                this.actor.kill();
                return Box::new(this);
            }
            this.skin_init();
            this.func_80a6c6b0();
            return Box::new(this);
        } else if play.scene_id == SCENE_GERUDOS_FORTRESS {
            if p.x == 3707.0 && p.y == 1413.0 && p.z == -665.0 {
                this.skin_init();
                this.func_80a6c4cc();
                return Box::new(this);
            }
            this.skin_init();
        } else {
            this.skin_init();
        }
        if FOLLOW.get(this.actor.params) == 0x10 >> 4 && PATH.get(this.actor.params) != 0xF {
            this.func_80a6b91c();
        } else {
            this.func_80a6bc48();
        }
        Box::new(this)
    }

    /// `Skin_Init(play, &skin, &gHorseNormalSkel, &gHorseNormalIdleAnim)` (`SkelAnime_InitSkin`
    /// plays it looped), then `Animation_PlayOnce(sAnimations[animationIdx])`.
    fn skin_init(&mut self) {
        self.skel = SkelAnimeStd::init_flex(self.skeleton.limbs.len(), Some(self.anims[0].clone()));
        let a = self.anims[self.animation_idx as usize].clone();
        self.skel.play_once(a);
    }

    /// `Animation_Change(&skin.skelAnime, sAnimations[animationIdx], speed, 0,
    /// Animation_GetLastFrame(..), ANIMMODE_ONCE, morph)`.
    fn change(&mut self, morph: f32) {
        let a = self.anims[self.animation_idx as usize].clone();
        let last = a.last_frame();
        let speed = self.func_80a6b30c();
        self.skel.change(a, speed, 0.0, last, ANIMMODE_ONCE, morph);
    }

    /// `func_80A6B250`: the walk's hooves, at frames past 0 and 16.
    fn func_80a6b250(&mut self, play: &mut PlayState) {
        let cur = self.skel.cur_frame;
        if (D_80A6D4C0[self.unk_200 as usize] as f32) < cur && (self.unk_200 != 0 || !((D_80A6D4C0[1] as f32) < cur)) {
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_WALK);
            self.unk_200 += 1;
            if self.unk_200 >= D_80A6D4C0.len() as i32 {
                self.unk_200 = 0;
            }
        }
    }

    /// `func_80A6B30C`: the animation's speed: the walk, trot and gallop by `speed` (/2, /3, /5).
    pub fn func_80a6b30c(&self) -> f32 {
        let i = self.animation_idx as usize;
        match self.animation_idx {
            4 => D_80A6D4C8[i] * self.actor.speed_xz * (1.0 / 2.0),
            5 => D_80A6D4C8[i] * self.actor.speed_xz * (1.0 / 3.0),
            6 => D_80A6D4C8[i] * self.actor.speed_xz * (1.0 / 5.0),
            _ => D_80A6D4C8[i],
        }
    }

    /// `func_80A6B91C`: galloping the path (`HORSE_FOLLOW_PATH`) at 7, from its first point; never
    /// update-culled.
    fn func_80a6b91c(&mut self) {
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.action = Action::FollowPath;
        self.animation_idx = 6;
        self.waypoint = 0;
        self.actor.speed_xz = 7.0;
        self.change(0.0);
    }

    /// `EnHorseNormal_FollowPath`.
    fn follow_path(&mut self, play: &mut PlayState) {
        // play->pathList[PARAMS_GET_U(params, 0, 4)] (no Path_GetByIndex check).
        let index = PATH.get(self.actor.params) as usize;
        let Some(path) = play.setup_path_list().get(index) else {
            log::error!("En_Horse_Normal: no path {index} in the scene layer");
            return;
        };
        let count = path.count() as i32;
        let point = path.point(self.waypoint as usize);
        let dx = point.x - self.actor.world_pos.x;
        let dz = point.z - self.actor.world_pos.z;
        smooth_step_to_s(&mut self.actor.world_rot.y, rad_to_binang_trunc(f_atan2f(dx, dz)), 0xA, 0x7D0, 1);
        self.actor.shape_rot.y = self.actor.world_rot.y;
        if dx * dx + dz * dz < 600.0 {
            self.waypoint += 1;
            if self.waypoint >= count {
                self.waypoint = 0;
            }
        }
        self.skel.play_speed = self.func_80a6b30c();
        if self.skel.update() {
            self.change(0.0);
            self.func_80a6bcec(play);
        }
    }

    /// `EnHorseNormal_NextAnimation`.
    fn next_animation(&mut self) {
        self.action = Action::CycleAnimations;
        self.animation_idx += 1;
        if self.animation_idx >= ANIMATIONS.len() as i32 {
            self.animation_idx = 0;
        }
        let a = self.anims[self.animation_idx as usize].clone();
        self.skel.play_once(a);
    }

    /// `EnHorseNormal_CycleAnimations`.
    fn cycle_animations(&mut self, _play: &mut PlayState) {
        self.actor.speed_xz = 0.0;
        if self.skel.update() {
            self.next_animation();
        }
    }

    /// `func_80A6BC48`: wandering (`HORSE_WANDER`), standing.
    fn func_80a6bc48(&mut self) {
        self.action = Action::Wander;
        self.animation_idx = 0;
        self.actor.speed_xz = 0.0;
        self.unk_218 = 0.0;
        self.unk_21c = 0;
        self.unk_21e = 0;
        self.change(0.0);
    }

    /// `func_80A6BCEC`: the trot's and gallop's `NA_SE_EV_HORSE_RUN`.
    fn func_80a6bcec(&mut self, play: &mut PlayState) {
        if self.animation_idx == 5 || self.animation_idx == 6 {
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_RUN);
        }
    }

    /// `func_80A6BD7C`: the idle's sand (past frame 28) and the rearing's landing (past 25), once
    /// each.
    fn func_80a6bd7c(&mut self, play: &mut PlayState) {
        let frame = self.skel.cur_frame;
        if self.animation_idx == 0 && frame > 28.0 && self.unk_1e4 & 1 == 0 {
            self.unk_1e4 |= 1;
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_SANDDUST);
        } else if self.animation_idx == 3 && frame > 25.0 && self.unk_1e4 & 2 == 0 {
            self.unk_1e4 |= 2;
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_LAND2);
        }
    }

    /// `SFX_PLAY_AT_POS(&this->unk_204, sfx)`: at the actor's `projectedPos` (the port has no
    /// sound position of an actor's own; `unk_204` is it 120 up).
    fn sfx_at_unk_204(play: &mut PlayState, sfx: u16) {
        audio_play_actor_sfx2(play, sfx);
    }

    /// The voice of a new animation (the whinny's groan, the rearing's neigh) or the run.
    fn voice(&mut self, play: &mut PlayState, phi_t0: i32) {
        if phi_t0 == 1 {
            Self::sfx_at_unk_204(play, NA_SE_EV_HORSE_GROAN);
        } else if phi_t0 == 3 {
            Self::sfx_at_unk_204(play, NA_SE_EV_HORSE_NEIGH);
        } else {
            self.func_80a6bcec(play);
        }
    }

    /// `EnHorseNormal_Wander`.
    fn wander(&mut self, play: &mut PlayState) {
        let mut phi_t0 = self.animation_idx;
        match D_80A6D510[self.animation_idx as usize] {
            0 => {
                self.func_80a6bd7c(play);
                self.actor.speed_xz = 0.0;
                self.unk_218 = 0.0;
            }
            1 => {
                if play.rand.zero_one() < 0.1 {
                    self.unk_218 = 2.0 * play.rand.zero_one() - 1.0;
                }
                self.actor.speed_xz += self.unk_218;
                if self.actor.speed_xz <= 0.0 {
                    self.actor.speed_xz = 0.0;
                    self.unk_218 = 0.0;
                    phi_t0 = 0;
                } else if self.actor.speed_xz < 3.0 {
                    self.func_80a6b250(play);
                    phi_t0 = 4;
                } else if self.actor.speed_xz < 6.0 {
                    phi_t0 = 5;
                } else if self.actor.speed_xz < 8.0 {
                    phi_t0 = 6;
                } else {
                    self.actor.speed_xz = 8.0;
                    phi_t0 = 6;
                }
                if play.rand.zero_one() < 0.1
                    || (self.unk_21e == 0
                        && (self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 || self.body_collider.base.oc_flags1 & OC1_HIT != 0 || self.head_collider.base.oc_flags1 & OC1_HIT != 0))
                {
                    // s16 += f32: the sum truncated.
                    self.unk_21e = (self.unk_21e as f32 + ((play.rand.zero_one() * 30.0) - 15.0)) as i16;
                    if self.unk_21e > 50 {
                        self.unk_21e = 50;
                    } else if self.unk_21e < -50 {
                        self.unk_21e = -50;
                    }
                }
                self.unk_21c = self.unk_21c.wrapping_add(self.unk_21e);
                if self.unk_21c < -300 {
                    self.unk_21c = -300;
                } else if self.unk_21c > 300 {
                    self.unk_21c = 300;
                } else if play.rand.zero_one() < 0.25 && (self.unk_21c as f32).abs() < 100.0 {
                    self.unk_21c = 0;
                    self.unk_21e = 0;
                }
                self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(self.unk_21c);
                self.actor.shape_rot.y = self.actor.world_rot.y;
            }
            _ => {}
        }

        // `phi_t0 != animationIdx || SkelAnime_Update(..)`: the animation isn't advanced on a change.
        if phi_t0 != self.animation_idx || self.skel.update() {
            if phi_t0 != self.animation_idx {
                self.animation_idx = phi_t0;
                self.unk_1e4 &= !1;
                self.unk_1e4 &= !2;
                self.voice(play, phi_t0);
                self.change(-3.0);
            } else {
                if D_80A6D510[self.animation_idx as usize] == 0 {
                    if play.rand.zero_one() < 0.25 {
                        self.unk_218 = 1.0;
                        phi_t0 = 4;
                    } else {
                        phi_t0 = D_80A6D4F4[(play.rand.zero_one() * 2.0) as i32 as usize];
                        self.actor.speed_xz = 0.0;
                        self.unk_218 = 0.0;
                    }
                }
                self.unk_1e4 &= !1;
                self.unk_1e4 &= !2;
                self.voice(play, phi_t0);
                if phi_t0 != self.animation_idx {
                    self.animation_idx = phi_t0;
                    self.change(-3.0);
                } else {
                    self.change(0.0);
                }
            }
        }
    }

    /// `func_80A6C4CC`: standing (`HORSE_WAIT`).
    fn func_80a6c4cc(&mut self) {
        self.action = Action::Wait;
        self.animation_idx = 0;
        self.actor.speed_xz = 0.0;
        self.unk_218 = 0.0;
        self.unk_21c = 0;
        self.unk_21e = 0;
        self.change(0.0);
    }

    /// The next standing animation: idle (0.4), whinny (0.4) or rearing.
    fn pick_standing(&mut self, play: &mut PlayState, clone_voice: bool) {
        let rand = play.rand.zero_one();
        if rand < 0.4 {
            self.animation_idx = 0;
        } else if rand < 0.8 {
            self.animation_idx = 1;
            if clone_voice {
                self.unk_1e4 |= 0x20;
            }
            Self::sfx_at_unk_204(play, NA_SE_EV_HORSE_GROAN);
        } else {
            self.animation_idx = 3;
            if clone_voice {
                self.unk_1e4 |= 0x20;
            }
            Self::sfx_at_unk_204(play, NA_SE_EV_HORSE_NEIGH);
        }
        self.change(0.0);
    }

    /// `EnHorseNormal_Wait`.
    fn wait(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.pick_standing(play, false);
        }
    }

    /// `func_80A6C6B0`: standing with a clone (`HORSE_WAIT_CLONE`); never culled.
    fn func_80a6c6b0(&mut self) {
        self.action = Action::WaitClone;
        self.animation_idx = 0;
        self.actor.speed_xz = 0.0;
        self.unk_218 = 0.0;
        self.unk_21c = 0;
        self.unk_21e = 0;
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED;
        self.change(0.0);
    }

    /// `EnHorseNormal_WaitClone`.
    fn wait_clone(&mut self, play: &mut PlayState) {
        self.func_80a6bd7c(play);
        if self.skel.update() {
            self.pick_standing(play, true);
            self.unk_1e4 &= !1;
            self.unk_1e4 &= !2;
            self.unk_1e4 &= !8;
            self.unk_1e4 &= !0x10;
        }
    }

    /// `func_80A6C8E0`: the ground 30 ahead (`BgCheck_EntityRaycastDown3` from 60 up) pitches the
    /// horse: `shape.rot.x = RAD_TO_BINANG(Math_FAtan2F(world.pos.y - unk_220, 30))`.
    fn func_80a6c8e0(&mut self, play: &PlayState) {
        let check_pos = Vec3::new(
            (sin_s(self.actor.shape_rot.y) * 30.0) + self.actor.world_pos.x,
            self.actor.world_pos.y + 60.0,
            (cos_s(self.actor.shape_rot.y) * 30.0) + self.actor.world_pos.z,
        );
        self.unk_220 = play.col.entity_raycast_down(check_pos).0;
        self.actor.shape_rot.x = rad_to_binang_trunc(f_atan2f(self.actor.world_pos.y - self.unk_220, 30.0));
    }

    /// The clone's place and yaw (`EnHorseNormal_Draw`): by the scene and the horse's exact place.
    /// @bug (game): a waiting clone elsewhere (any other stable horse) draws its clone at the
    /// origin with `cloneRotY` never set (here 0).
    pub fn clone_place(scene_id: u16, p: Vec3) -> (Vec3, i16) {
        let mut clone_pos = Vec3::ZERO;
        let mut clone_rot_y: i16 = 0;
        if scene_id == SCENE_STABLE {
            if p.x == 355.0 && p.y == 0.0 && p.z == -245.0 {
                clone_pos = Vec3::new(235.0, 0.0, 100.0);
                clone_rot_y = 0x7FFF;
            } else if p.x == 238.0 && p.y == 0.0 && p.z == -245.0 {
                clone_pos = Vec3::new(478.0, 0.0, 100.0);
                clone_rot_y = 0x7FFF;
            }
        } else if scene_id == SCENE_LON_LON_RANCH {
            if p.x == -730.0 && p.y == 0.0 && p.z == -1100.0 {
                clone_pos = Vec3::new(780.0, 0.0, -80.0);
                clone_rot_y = 0;
            } else if p.x == 880.0 && p.y == 0.0 && p.z == -1170.0 {
                clone_pos = Vec3::new(-1000.0, 0.0, -70.0);
                clone_rot_y = 0;
            }
        }
        (clone_pos, clone_rot_y)
    }

    /// `func_80A6CC88`: the clone's projected position (`SkinMatrix_Vec3fMtxFMultXYZW` on
    /// `viewProjectionMtxF`) and its sounds: the sand and the landing as the horse's, its voice
    /// (`unk_1E4` 0x20) at the rearing or the whinny.
    fn func_80a6cc88(&mut self, play: &mut PlayState, clone_pos: Vec3) {
        let cur_frame = self.skel.cur_frame;
        self.unk_1e8 = (play.view_proj * clone_pos.extend(1.0)).truncate();
        self.unk_1f4 = self.unk_1e8;
        self.unk_1f4.y += 120.0;
        // At unk_1E8 and unk_1F4: the actor's projectedPos here (see `sfx_at_unk_204`).
        if self.animation_idx == 0 && cur_frame > 28.0 && self.unk_1e4 & 8 == 0 {
            self.unk_1e4 |= 8;
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_SANDDUST);
        } else if self.animation_idx == 3 && cur_frame > 25.0 && self.unk_1e4 & 0x10 == 0 {
            self.unk_1e4 |= 0x10;
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_LAND2);
        } else if self.animation_idx == 3 && self.unk_1e4 & 0x20 != 0 {
            self.unk_1e4 &= !0x20;
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_NEIGH);
        } else if self.animation_idx == 1 && self.unk_1e4 & 0x20 != 0 {
            self.unk_1e4 &= !0x20;
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_GROAN);
        }
    }

    /// What `Skin_ApplyAnimTransformations` reads of the actor now.
    fn place(&self) -> SkinPlace {
        SkinPlace { scale: self.actor.scale, rot: [self.actor.shape_rot.x, self.actor.shape_rot.y, self.actor.shape_rot.z], pos: self.actor.world_pos, y_offset: self.actor.shape_y_offset }
    }

    /// The clone's `skinMtx` (`SkinMatrix_SetTranslateRotateYXZScale`): the horse's scale and
    /// pitch and roll, the clone's yaw and place.
    fn clone_mtx(scale: Vec3, rot: [i16; 3], y_offset: f32, clone_pos: Vec3, clone_rot_y: i16) -> Mat4 {
        to_mat4(&set_translate_rotate_yxz_scale(scale.x, scale.y, scale.z, rot[0], clone_rot_y, rot[2], clone_pos.x, (y_offset * scale.y) + clone_pos.y, clone_pos.z))
    }
}

// The render state's extras.
mod rs {
    /// `values`: the clone's shadow's scale (`(1 - distFromGround * 0.01) * shadowScale`).
    pub const SHADOW: usize = 0;
}

impl ActorImpl for EnHorseNormal {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnHorseNormal_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::CycleAnimations => self.cycle_animations(play),
            Action::Wander => self.wander(play),
            Action::Wait => self.wait(play),
            Action::WaitClone => self.wait_clone(play),
            Action::FollowPath => self.follow_path(play),
        }
        // Actor_MoveXZGravity.
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 20.0, 35.0, 100.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        if play.scene_id == SCENE_LON_LON_RANCH && self.actor.world_pos.z < -2400.0 {
            self.actor.world_pos.z = -2400.0;
        }
        self.actor.focus_pos = self.actor.world_pos + Vec3::Y * 70.0;
        self.unk_204 = self.actor.projected_pos;
        self.unk_204.y += 120.0;
        self.body_collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.body_collider);
        self.actor.col_chk_info.mass = if self.actor.speed_xz == 0.0 { MASS_IMMOVABLE } else { MASS_HEAVY };
    }

    /// `EnHorseNormal_Draw`'s state: the pitch (`func_80A6C8E0`), the post-draw
    /// (`EnHorseNormal_PostDraw`: the head's sphere on limb 11 by `Skin_GetLimbPos`, for OC), and
    /// the clone's sounds (`func_80A6CC88`) and its cylinder at its place (OC).
    fn draw_update(&mut self, play: &mut PlayState) {
        // @bug (game): `sceneId != SCENE_LON_LON_RANCH || sceneId != SCENE_STABLE` always holds.
        self.func_80a6c8e0(play);
        let (mats, skin_mtx) = apply_anim_transformations(&self.skeleton, &self.skel.joint_table, &self.place(), true);
        for e in &mut self.head_collider.elements {
            let c = e.dim.model_sphere.center;
            let center = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
            let new_center = get_limb_pos(&skin_mtx, &mats, e.dim.limb as usize, center);
            e.dim.world_sphere.center = [new_center.x as i16, new_center.y as i16, new_center.z as i16];
            e.dim.world_sphere.radius = (e.dim.model_sphere.radius as f32 * e.dim.scale) as i16;
        }
        // @bug (game) (see `EnHorse_SkinCallback1`): set from the draw, after this frame's check.
        play.collision_check_set_oc(&self.actor, 1, &mut self.head_collider);

        if self.action == Action::WaitClone {
            let (clone_pos, _) = Self::clone_place(play.scene_id, self.actor.world_pos);
            self.func_80a6cc88(play, clone_pos);
            self.clone_collider.dim.pos = [clone_pos.x as i16, clone_pos.y as i16, clone_pos.z as i16];
            play.collision_check_set_oc(&self.actor, 2, &mut self.clone_collider);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let dist_from_ground = self.actor.world_pos.y - self.actor.floor_height;
        rs.values = vec![(1.0 - (dist_from_ground * 0.01)) * SHADOW_SCALE];
        rs
    }

    /// `EnHorseNormal_Draw`: `Gfx_SetupDL_25Opa`, the skin (`func_800A6330`, `setTranslation`);
    /// waiting with a clone, the clone with the same limbs under its own matrix, and its shadow.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let place = SkinPlace { scale: rs.scale, rot: rs.rot, pos: rs.pos, y_offset: rs.y_offset };
        let (mats, skin_mtx) = apply_anim_transformations(&self.skeleton, &joints.rot, &place, true);
        let bones = skin_bones(&self.skeleton, &mats, false);
        let mesh = MeshKey::named(keys::bake(BAKE));
        out.opa.push(DrawCmd { mesh: mesh.clone(), transform: to_mat4(&skin_mtx), bones: bones.clone(), params: DrawParams::default() });

        if self.action == Action::WaitClone {
            let (clone_pos, clone_rot_y) = Self::clone_place(play.scene_id, self.actor.world_pos);
            // SKIN_DRAW_FLAG_CUSTOM_TRANSFORMS | SKIN_DRAW_FLAG_CUSTOM_MATRIX: the limbs (and the
            // animated limbs' vertices) as the horse's draw left them, under skinMtx.
            let clone = Self::clone_mtx(rs.scale, rs.rot, rs.y_offset, clone_pos, clone_rot_y);
            out.opa.push(DrawCmd { mesh, transform: clone, bones, params: DrawParams::default() });
            // The shadow: Matrix_Translate(clonePos), Matrix_Scale(scale.x * t, 1, scale.z * t),
            // Matrix_RotateY(cloneRotY).
            let t = rs.values.get(rs::SHADOW).copied().unwrap_or(SHADOW_SCALE);
            let m = Mat4::from_translation(clone_pos) * Mat4::from_scale(Vec3::new(rs.scale.x * t, 1.0, rs.scale.z * t)) * Mat4::from_rotation_y(binang_to_rad(clone_rot_y));
            out.xlu.push(DrawCmd::new(MeshKey::named(keys::bake(BAKE_SHADOW)), m));
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            0 => Some(ColliderMut::Cylinder(&mut self.body_collider)),
            1 => Some(ColliderMut::JntSph(&mut self.head_collider)),
            2 => Some(ColliderMut::Cylinder(&mut self.clone_collider)),
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
