//! `En_Vm` (`ovl_En_Vm/z_en_vm.c`): the Beamos, a stone pillar whose head turns looking for Link
//! and fires a laser at him (GAME-09 milestone 5). `object_vm`'s `gBeamosSkel`, 0.01 big (the
//! small, health 1) or 0.014 (the large, health 2). Dodongo's Cavern's room 2 (Master Quest)
//! places two small ones (params 0x0401: sight 160).
//!
//! - **Looking** (`EnVm_Wait`): the eye lowered (`beamRot.x` to 0), the head turning 500 a frame
//!   until Link is in sight: within `beamSightRange` across, within 0x2710 of where the head
//!   faces, the eye looking down at him at least 0x0E38 (`Math_Vec3f_Pitch` from the eye,
//!   `beamPos1`, held to 0x1B91), no more than 80 above it nor 160 below. In sight, the head
//!   follows him (`Math_SmoothStepToS` a whole step at a time, faster the nearer he is) and the
//!   eye tips down at him; ten frames close enough (the step's difference at most 5460: one the
//!   other way always is) and it **aims** (`NA_SE_EN_BIMOS_AIM`): `gBeamosAnim` twice at 2 a frame
//!   from frame 0, the head on him; then aimed at him (`beamRot` his yaw and the pitch, held to
//!   0x1B91). Under 0x0AAA (too flat) it looks again; else the eye flashes (`Effect_Ss_Dead_Dd`
//!   at `beamPos2`: blue in white, 150 big shrinking 25 a frame, its alpha up 16, 20 frames) and
//!   it **fires** (`EnVm_Attack`).
//! - **The beam** (`EnVm_Attack`): 0.6 wide narrowing to 0.1 (0.12 a frame), 305 frames
//!   (`timer`): its first four still, then following Link (the head, `beamRot`: a tenth, at most
//!   0xDAC a frame) and growing 40 a frame towards his feet (his floor's height under him),
//!   `NA_SE_EN_BIMOS_LAZER`. Its draw (`EnVm_PostLimbDraw`, limb 2) runs a line from the eye
//!   (`beamPos1`) to 500 past its end (`beamPos3`) against the walls and floors
//!   (`BgCheck_EntityLineTest1`, one-faced, no ceilings): a surface stops it 5 short (its length
//!   `beamScale.z`) and marks it touching (`unk_260` 4), where the next update scorches it
//!   (`Effect_Ss_Dead_Ds`, 20 big shrinking 1, 20 frames), raises dust (`func_80033480`) and
//!   sounds (`NA_SE_EN_BIMOS_LAZER_GND`); the draw puts the burning flame there (two of
//!   `gEffEnemyDeathFlameDL`, `gameplayFrames % 8`'s texture and four on). Its quad
//!   (`colliderQuad1`, ±5 across in model units ±500, from the eye to `beamPos3`) hurts 16
//!   (`0x10`) from its second frame of firing. Hitting anything (`AT_HIT`), at the timer's end or
//!   flatter than 0x0AAA it narrows to nothing (0.1 at most, 0.03 a frame) and looks again.
//! - **The eye** (`colliderQuad2`, on limb 2) hit by Player: red 8 frames and **stunned**
//!   (`EnVm_Stun`, `NA_SE_EN_GOMA_JR_FREEZE`): 180 frames the animation backwards and the eye
//!   lowering, then the animation forwards, ten frames more at 2 a frame, and it looks again.
//! - **Only explosives hurt it** (`EnVm_CheckHealth`: `Actor_GetCollidedExplosive` on its metal
//!   cylinder, which turns everything else away, `AC_HARD`; not while the colour filter's on): a
//!   blast takes 1 health; with some left, stunned as by the eye; at 0 a bomb with no fuse at it,
//!   20 up (`ACTOR_EN_BOM`, `BOMB_BODY`, `shape.rot.z` 0x601: 6 more damage, 1 more growth), and
//!   the head **flies off** (`EnVm_SetupDie`): the body gone (limb 10's list `NULL`), the model
//!   5000 down, up at 8 (gravity -0.5) 1 to 2 a frame a random way, spinning; 33 frames on a
//!   second bomb (`shape.rot.z` 0x6FF), a drop from table 10, and it's gone.
//!
//! The whole overlay is ported. `draw_update` is `EnVm_PostLimbDraw` (limb 2's matrix, as
//! `SkelAnime_DrawOpa` leaves it): `beamPos1` and `beamPos2` (the eye and 1600 out), the line test
//! and `beamPos3`, the two quads. The draw is `EnVm_Draw`: the skeleton (limb 2 turned by
//! `beamRot.x` and `headRotY`, limb 10 hidden while dying), a circle under it (`func_80033C30`,
//! 0.4: its own, `ActorShape_Init` gives it no shadow), the flame at `beamPos3` while firing, and
//! `gBeamosLaserDL` from the eye (`beamRot`, `beamScale.x × 0.1` across, `beamScale.z × 0.0015`
//! long) with its texture scrolling (`func_80094E78`, `beamTexScroll` up 12 a frame).
//! @bug (game): the flame's second copy is turned by `Matrix_RotateY(32767.0f)`, radians (about
//! 0.19 after whole turns, 11°), not a half turn; kept. `EnVm_Wait`'s ten frames count any frame
//! the head's difference is at most 5460, so one it turns the other way by any amount counts.
//!
//! Params: `type` bits 0 to 7 (`BeamosType`: 0 `BEAMOS_LARGE`, any other the small one,
//! `BEAMOS_SMALL` 1); `sightRange` bits 8 to 15, signed (`params >> 8`), × 40 the distance across
//! it sees Link within.
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::BGCHECK_Y_MIN;
use eng_collision::math3d::{Cylinder16, vec3f_dist_xyz};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{smooth_step_to_f, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, actor_get_collided_explosive, audio_play_actor_sfx2, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_ONCE, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};
use oot_game::sys_matrix::MtxF;

use crate::en_bom::{ACTOR_EN_BOM, EnBom};

/// `ACTOR_EN_VM` (`actor_table.h`: 0x008A).
pub const ACTOR_EN_VM: i16 = 0x008A;
/// `OBJECT_VM` (`object_table.h`: 0x008B).
pub const OBJECT: &str = "object_vm";
/// `gBeamosSkel` (`object_vm.xml`): 10 limbs (`SkelAnime_Init`'s 11 with the root's).
pub const SKEL: &str = "gBeamosSkel";
/// `gBeamosAnim`: the eye opening (forwards) or closing (backwards).
pub const ANIM: &str = "gBeamosAnim";
/// `gBeamosLaserDL`.
const LASER_DL: &str = "gBeamosLaserDL";

/// `En_Vm_Profile`: `ACTORCAT_ENEMY`, attention-enabled, always updating, `OBJECT_VM`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_VM, name: "En_Vm", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `BeamosType` (`z_en_vm.h`).
pub const BEAMOS_LARGE: i16 = 0;
pub const BEAMOS_SMALL: i16 = 1;

/// `params & 0xFF` (`params &= 0xFF` at init): `BEAMOS_LARGE` or any other (the small one).
pub const TYPE: Field = Field::params("type", 0, 8, Kind::Named(&[(0, "BEAMOS_LARGE"), (1, "BEAMOS_SMALL")]), "the large one (0: 0.014 big, health 2) or the small (any other: 0.01, health 1)");
/// `PARAMS_GET_NOMASK(params, 8)`: `params >> 8`, signed; × 40 how far across it sees Link.
pub const SIGHT_RANGE: Field = Field::params("sightRange", 8, 8, Kind::Range(-128, 127), "how far across it sees Link, in 40s (4: 160)").signed();

/// En_Vm's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE, SIGHT_RANGE], flags: &[], todo: false, objects: &[], texts: &[] };

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_BIMOS_ROLL_HEAD` (0x3970).
pub const NA_SE_EN_BIMOS_ROLL_HEAD: u16 = 0x3970;
/// `NA_SE_EN_BIMOS_LAZER` (0x3971).
pub const NA_SE_EN_BIMOS_LAZER: u16 = 0x3971;
/// `NA_SE_EN_BIMOS_LAZER_GND` (0x3972).
pub const NA_SE_EN_BIMOS_LAZER_GND: u16 = 0x3972;
/// `NA_SE_EN_BIMOS_AIM` (0x3973).
pub const NA_SE_EN_BIMOS_AIM: u16 = 0x3973;

/// `NAVI_ENEMY_BEAMOS` (`actor.h`: 0x39).
pub const NAVI_ENEMY_BEAMOS: u8 = 0x39;
/// `BOMB_BODY` (`z_en_bom.h`'s `EnBomType`).
pub const BOMB_BODY: i16 = 0;
/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_10, false)` (`z_en_item00.h`: the table
/// × 16).
const DROP_TABLE_10: i16 = 10 * 16;

/// `sCylinderInit`: its metal body, turning Player's hits away (`AC_HARD`, `0xFFCFFFFF`),
/// pushing; 25 by 70.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_METAL, at_flags: AT_NONE, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 25, height: 70, y_shift: 0, pos: [0, 0, 0] },
};

/// `sQuadInit1`: the beam, hurting 16 (`0x10`).
pub const QUAD_INIT1: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_METAL, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x10 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL | ATELEM_UNK7,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `sQuadInit2`: the eye, hit by Player's (`0xFFCFFFFF`).
pub const QUAD_INIT2: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_METAL, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `D_80B2EB1C`, `D_80B2EB28`: the eye and 1600 out from it, on limb 2 (`beamPos1`, `beamPos2`).
const EYE: Vec3 = Vec3::ZERO;
const EYE_OUT: Vec3 = Vec3::new(0.0, 0.0, 1600.0);
/// `D_80B2EB64`, `D_80B2EB70`: the beam quad's near corners (3, 2); `D_80B2EB04`, `D_80B2EB10`
/// with the beam's length its far ones (1, 0).
const BEAM_NEAR: [Vec3; 2] = [Vec3::new(500.0, 0.0, 0.0), Vec3::new(-500.0, 0.0, 0.0)];
/// `D_80B2EB34`, `D_80B2EB40`, `D_80B2EB4C`, `D_80B2EB58`: the eye quad's corners 1, 0, 3, 2.
const EYE_QUAD: [Vec3; 4] = [Vec3::new(1000.0, 700.0, 2000.0), Vec3::new(1000.0, -700.0, 2000.0), Vec3::new(-1000.0, 700.0, 1500.0), Vec3::new(-1000.0, -700.0, 1500.0)];
/// `D_80B2EB7C`: `func_80033C30`'s circle.
const SHADOW_SCALE: Vec3 = Vec3::new(0.4, 0.4, 0.4);
/// `D_80B2EAEC`: the flash's velocity and acceleration.
const ZERO: Vec3 = Vec3::ZERO;

/// The limbs the draw changes (1-based, as `SkelAnime_DrawOpa` numbers them): the head
/// (`beamRot.x`, `headRotY`; the beam's matrix) and the body (hidden while dying).
pub const LIMB_HEAD: usize = 2;
pub const LIMB_BODY: usize = 10;

/// `Math_SmoothStepToS`'s difference `EnVm_Wait` counts its frames under.
const AIM_DIFF: i16 = 5460;
/// The pitches: in sight from 0x0E38, held to 0x1B91, firing from 0x0AAA.
const PITCH_MIN_SIGHT: i16 = 0x0E38;
const PITCH_MAX: i16 = 0x1B91;
const PITCH_MIN_FIRE: i16 = 0x0AAA;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Wait,
    Attack,
    Stun,
    Die,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Wait => "EnVm_Wait",
            Action::Attack => "EnVm_Attack",
            Action::Stun => "EnVm_Stun",
            Action::Die => "EnVm_Die",
        }
    }
}

/// The colliders' ids (`collider_mut`).
const COL_CYLINDER: u8 = 0;
const COL_QUAD1: u8 = 1;
const COL_QUAD2: u8 = 2;

pub struct EnVm {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anim: Option<Anim>,
    pub action: Action,
    /// `beamSightRange`: `(params >> 8) × 40`.
    pub beam_sight_range: f32,
    /// `unk_21C`: 0 looking, 1 firing, 2 stunned, 3 dying.
    pub unk_21c: i32,
    pub timer: i32,
    /// `beamPos1`: the eye (the draw's).
    pub beam_pos1: Vec3,
    /// `beamPos2`: 1600 out from the eye (the flash's place).
    pub beam_pos2: Vec3,
    /// `beamPos3`: the beam's end, 500 past it or where it meets a surface.
    pub beam_pos3: Vec3,
    /// `beamScale`: x its width (× 0.1), z its length.
    pub beam_scale: Vec3,
    pub beam_rot: [i16; 3],
    pub beam_tex_scroll: i16,
    pub head_rot_y: i16,
    /// `unk_25E`: looking, 0 watching, 1 aiming; stunned, the animation's stage.
    pub unk_25e: i16,
    /// `unk_260`: aiming, the animation's turns; firing, 3; 4 when the draw found the beam on a
    /// surface.
    pub unk_260: i16,
    pub beam_speed: f32,
    pub collider_cylinder: ColliderCylinder,
    pub collider_quad1: ColliderQuad,
    pub collider_quad2: ColliderQuad,
}

impl EnVm {
    /// `EnVm_Init`: `gBeamosSkel` with `gBeamosAnim`, no shadow (`ActorShape_Init(.., NULL, ..)`),
    /// the cylinder and the two quads, the sight range from the params' high byte, the params cut
    /// to their low byte; the large one health 2 at 0.014, the small health 1 at 0.01; looking.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut anim = None;
        let mut skeleton = None;
        if let Some(a) = play.assets.clone() {
            skeleton = a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Vm: {e:#}")).ok();
            anim = a.animation(OBJECT, ANIM).map_err(|e| log::error!("En_Vm: {e:#}")).ok();
        }
        // SkelAnime_Init(.., &gBeamosSkel, &gBeamosAnim, .., 11).
        let skel = SkelAnimeStd::init_flex(skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(10), anim.clone());
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::None, 0.0);
        let beam_sight_range = SIGHT_RANGE.get(actor.params) as f32 * 40.0;
        // params &= 0xFF.
        actor.params = TYPE.get(actor.params);
        actor.navi_enemy_id = NAVI_ENEMY_BEAMOS;
        if TYPE.get(actor.params) == BEAMOS_LARGE {
            actor.col_chk_info.health = 2;
            actor.scale = Vec3::splat(0.014);
        } else {
            actor.col_chk_info.health = 1;
            actor.scale = Vec3::splat(0.01);
        }
        let mut this = EnVm {
            actor,
            skel,
            skeleton,
            anim,
            action: Action::Wait,
            beam_sight_range,
            unk_21c: 0,
            timer: 0,
            beam_pos1: Vec3::ZERO,
            beam_pos2: Vec3::ZERO,
            beam_pos3: Vec3::ZERO,
            beam_scale: Vec3::ZERO,
            beam_rot: [0; 3],
            beam_tex_scroll: 0,
            head_rot_y: 0,
            unk_25e: 0,
            unk_260: 0,
            beam_speed: 0.0,
            collider_cylinder: ColliderCylinder::new(&CYLINDER_INIT),
            collider_quad1: ColliderQuad::new(&QUAD_INIT1),
            collider_quad2: ColliderQuad::new(&QUAD_INIT2),
        };
        this.setup_wait();
        Box::new(this)
    }

    /// `Animation_GetLastFrame(&gBeamosAnim)`.
    fn last_frame(&self) -> f32 {
        self.anim.as_ref().map(|a| a.last_frame()).unwrap_or(0.0)
    }

    /// `Animation_Change(&skelAnime, &gBeamosAnim, speed, start, end, ANIMMODE_ONCE, 0)`.
    fn change(&mut self, speed: f32, start: f32, end: f32) {
        if let Some(a) = self.anim.clone() {
            self.skel.change(a, speed, start, end, ANIMMODE_ONCE, 0.0);
        }
    }

    /// `EnVm_SetupWait`: the animation held at its last frame, not aiming, ten frames to aim.
    fn setup_wait(&mut self) {
        let frame_count = self.last_frame();
        self.change(1.0, frame_count, frame_count);
        self.unk_260 = 0;
        self.unk_25e = 0;
        self.unk_21c = 0;
        self.timer = 10;
        self.action = Action::Wait;
    }

    /// Player's `world.pos` and `floorHeight`.
    fn player(play: &PlayState) -> (Vec3, f32) {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| (a.world_pos, a.floor_height)).unwrap_or((Vec3::ZERO, BGCHECK_Y_MIN))
    }

    /// `EnVm_Wait`: watching (`unk_25E` 0) and aiming (1); see the module's docs.
    fn wait(&mut self, play: &mut PlayState) {
        let (player_pos, _) = Self::player(play);
        match self.unk_25e {
            0 => {
                smooth_step_to_s(&mut self.beam_rot[0], 0, 10, 1500, 0);
                let head_rot = self.actor.yaw_towards_player.wrapping_sub(self.head_rot_y).wrapping_sub(self.actor.shape_rot.y);
                let mut pitch = eng_math::vec3f_pitch(self.beam_pos1, player_pos);
                if pitch > PITCH_MAX {
                    pitch = PITCH_MAX;
                }
                let dist = self.beam_sight_range - self.actor.xz_dist_to_player;
                if self.actor.xz_dist_to_player <= self.beam_sight_range
                    && (head_rot as i32).abs() <= 0x2710
                    && pitch >= PITCH_MIN_SIGHT
                    && self.actor.y_dist_to_player <= 80.0
                    && self.actor.y_dist_to_player >= -160.0
                {
                    smooth_step_to_s(&mut self.beam_rot[0], pitch, 10, 0xFA0, 0);
                    // (ABS((s16)(dist * 180.0f)) / 3) + 0xFA0, as the s16 step.
                    let step = (((dist * 180.0) as i32 as i16 as i32).abs() / 3 + 0xFA0) as i16;
                    let target = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
                    if smooth_step_to_s(&mut self.head_rot_y, target, 1, step, 0) <= AIM_DIFF {
                        self.timer -= 1;
                        if self.timer == 0 {
                            self.unk_25e += 1;
                            self.skel.cur_frame = 0.0;
                            self.skel.start_frame = 0.0;
                            self.skel.play_speed = 2.0;
                            audio_play_actor_sfx2(play, NA_SE_EN_BIMOS_AIM);
                        }
                    }
                } else {
                    self.head_rot_y = self.head_rot_y.wrapping_sub(0x1F4);
                }
                self.skel.update();
                return;
            }
            1 => {}
            _ => return,
        }
        let target = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        smooth_step_to_s(&mut self.head_rot_y, target, 1, 0x1F40, 0);
        if self.skel.update() {
            self.unk_260 += 1;
            self.skel.cur_frame = 0.0;
        }
        if self.unk_260 == 2 {
            self.beam_rot[1] = self.actor.yaw_towards_player;
            self.beam_rot[0] = eng_math::vec3f_pitch(self.beam_pos1, player_pos);
            if self.beam_rot[0] > PITCH_MAX {
                self.beam_rot[0] = PITCH_MAX;
            }
            if self.beam_rot[0] < PITCH_MIN_FIRE {
                self.skel.cur_frame = self.skel.end_frame;
                self.skel.start_frame = self.skel.cur_frame;
                self.unk_260 = 0;
                self.unk_25e = 0;
                self.timer = 10;
                self.skel.play_speed = 1.0;
            } else {
                self.skel.cur_frame = 6.0;
                let pos = self.beam_pos2;
                play.with_ss(|s| s.dead_dd_spawn(pos, ZERO, ZERO, 150, -25, [0, 0, 255], 0, [255, 255, 255], 16, 20));
                self.setup_attack();
            }
        }
    }

    /// `EnVm_SetupAttack`: frames 3 to 7 at 3 a frame, 305 frames, the beam 0.6 wide growing 40 a
    /// frame, its last hit forgotten.
    fn setup_attack(&mut self) {
        self.change(3.0, 3.0, 7.0);
        self.timer = 305;
        self.beam_scale.x = 0.6;
        self.beam_speed = 40.0;
        self.unk_21c = 1;
        self.collider_quad1.base.at_flags &= !AT_HIT;
        self.action = Action::Attack;
    }

    /// `EnVm_Attack`: see the module's docs.
    fn attack(&mut self, play: &mut PlayState) {
        let (player_pos, player_floor) = Self::player(play);
        let mut pitch = eng_math::vec3f_pitch(self.beam_pos1, player_pos);
        if pitch > PITCH_MAX {
            pitch = PITCH_MAX;
        }
        if self.collider_quad1.base.at_flags & AT_HIT != 0 {
            self.collider_quad1.base.at_flags &= !AT_HIT;
            self.timer = 0;
            if self.beam_scale.x > 0.1 {
                self.beam_scale.x = 0.1;
            }
        }
        if self.beam_rot[0] < PITCH_MIN_FIRE || self.timer == 0 {
            smooth_step_to_f(&mut self.beam_scale.x, 0.0, 1.0, 0.03, 0.0);
            self.unk_260 = 0;
            if self.beam_scale.x == 0.0 {
                self.beam_scale.y = 0.0;
                self.beam_scale.z = 0.0;
                self.setup_wait();
            }
        } else {
            self.timer -= 1;
            if self.timer > 300 {
                return;
            }
            let yaw = self.actor.yaw_towards_player;
            smooth_step_to_s(&mut self.head_rot_y, self.actor.shape_rot.y.wrapping_neg().wrapping_add(yaw), 10, 0xDAC, 0);
            smooth_step_to_s(&mut self.beam_rot[1], yaw, 10, 0xDAC, 0);
            smooth_step_to_s(&mut self.beam_rot[0], pitch, 10, 0xDAC, 0);
            let mut target_pos = player_pos;
            if player_floor > BGCHECK_Y_MIN {
                target_pos.y = player_floor;
            }
            let dist = vec3f_dist_xyz(self.beam_pos1, target_pos);
            smooth_step_to_f(&mut self.beam_scale.z, dist, 1.0, self.beam_speed, 0.0);
            smooth_step_to_f(&mut self.beam_scale.x, 0.1, 1.0, 0.12, 0.0);
            audio_play_actor_sfx2(play, NA_SE_EN_BIMOS_LAZER - SFX_FLAG);
            if self.unk_260 > 2 {
                play.collision_check_set_at(&self.actor, COL_QUAD1, &mut self.collider_quad1);
            }
            self.unk_260 = 3;
        }
        if self.skel.update() {
            self.skel.cur_frame = self.skel.start_frame;
        }
    }

    /// `EnVm_SetupStun`: the animation backwards from its last frame, 180 frames, the beam gone,
    /// `NA_SE_EN_GOMA_JR_FREEZE`.
    fn setup_stun(&mut self, play: &mut PlayState) {
        let last = self.last_frame();
        self.change(-1.0, last, 0.0);
        self.unk_260 = 0;
        self.timer = 180;
        self.unk_25e = self.unk_260;
        self.unk_21c = 2;
        self.beam_scale.z = 0.0;
        self.beam_scale.y = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.action = Action::Stun;
    }

    /// `EnVm_Stun`: while the timer runs the eye lowers (a tenth, at most 0x5DC) and the
    /// animation plays; then each time it ends the next stage: forwards from 0, then ten frames at
    /// 2 from 0, then looking.
    fn stun(&mut self) {
        if self.timer == 0 {
            if self.skel.update() {
                self.unk_25e += 1;
                if self.unk_25e == 3 {
                    self.setup_wait();
                } else if self.unk_25e == 1 {
                    let last = self.last_frame();
                    self.change(1.0, 0.0, last);
                } else {
                    self.timer = 10;
                    self.skel.cur_frame = 0.0;
                    self.skel.play_speed = 2.0;
                }
            }
        } else {
            smooth_step_to_s(&mut self.beam_rot[0], 0, 10, 0x5DC, 0);
            self.timer -= 1;
            self.skel.update();
        }
    }

    /// `EnVm_SetupDie`: the animation backwards, 33 frames, the beam gone, the model 5000 down and
    /// the actor as much up (its head where it was), up at 8 with gravity -0.5, 1 to 2 a frame a
    /// random way.
    fn setup_die(&mut self, play: &mut PlayState) {
        let last = self.last_frame();
        self.change(-1.0, last, 0.0);
        self.timer = 33;
        self.unk_260 = 0;
        self.unk_25e = 0;
        self.unk_21c = 3;
        self.beam_scale.z = 0.0;
        self.beam_scale.y = 0.0;
        self.actor.shape_y_offset = -5000.0;
        self.actor.world_pos.y += 5000.0 * self.actor.scale.y;
        self.actor.velocity.y = 8.0;
        self.actor.gravity = -0.5;
        self.actor.speed_xz = play.rand.zero_one() + 1.0;
        self.actor.world_rot.y = play.rand.centered_float(65535.0) as i32 as i16;
        self.action = Action::Die;
    }

    /// `EnVm_Die`: spinning (`beamRot.x` 0x5DC, `headRotY` 0x9C4 a frame), moving
    /// (`Actor_MoveXZGravity`); at the timer's end a bomb with no fuse (`shape.rot.z` 0x6FF), a
    /// drop from table 10, gone.
    fn die(&mut self, play: &mut PlayState) {
        self.beam_rot[0] = self.beam_rot[0].wrapping_add(0x5DC);
        self.head_rot_y = self.head_rot_y.wrapping_add(0x9C4);
        self.actor.move_forward();
        self.timer -= 1;
        if self.timer == 0 {
            Self::spawn_bomb(play, self.actor.world_pos, 0x6FF);
            let pos = self.actor.world_pos;
            crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, DROP_TABLE_10);
            self.actor.kill();
        }
    }

    /// `Actor_Spawn(ACTOR_EN_BOM, pos, rot (0, 0, rot_z), BOMB_BODY)`, its `timer` 0.
    fn spawn_bomb(play: &mut PlayState, pos: Vec3, rot_z: i16) {
        match play.actor_spawn(ACTOR_EN_BOM, pos, [0, 0, rot_z], BOMB_BODY) {
            Ok(h) => {
                if let Some(bomb) = play.actors.downcast_mut::<EnBom>(h) {
                    bomb.timer = 0;
                }
            }
            Err(e) => log::debug!("En_Vm: no bomb: {e:?}"),
        }
    }

    /// `EnVm_CheckHealth`: an explosive on the cylinder takes 1 health; else the eye's hit (not
    /// while stunned) is taken. Health left: red 8 frames, stunned; none: a bomb with no fuse 20
    /// up (`shape.rot.z` 0x601) and the head flies off.
    fn check_health(&mut self, play: &mut PlayState) {
        if actor_get_collided_explosive(play, &mut self.collider_cylinder.base).is_some() {
            self.actor.col_chk_info.health = self.actor.col_chk_info.health.wrapping_sub(1);
            log::debug!("hp down {}", self.actor.col_chk_info.health);
        } else {
            if self.collider_quad2.base.ac_flags & AC_HIT == 0 || self.unk_21c == 2 {
                return;
            }
            self.collider_quad2.base.ac_flags &= !AC_HIT;
        }
        if self.actor.col_chk_info.health != 0 {
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 8);
            self.setup_stun(play);
        } else {
            let w = self.actor.world_pos;
            Self::spawn_bomb(play, Vec3::new(w.x, w.y + 20.0, w.z), 0x601);
            self.setup_die(play);
        }
    }

    /// `EnVm_PostLimbDraw` for limb 2, on its matrix `m` (in the world): the eye and the flash's
    /// place; firing, the beam's end 500 past its length, stopped at a wall or floor (5 short,
    /// `unk_260` 4), and the beam's quad from the eye to there; the eye's quad.
    fn post_limb_draw_head(&mut self, play: &PlayState, m: Mat4) {
        self.beam_pos1 = m.transform_point3(EYE);
        self.beam_pos2 = m.transform_point3(EYE_OUT);
        if self.unk_260 >= 3 {
            let sp80 = Vec3::new(0.0, 0.0, (self.beam_scale.z + 500.0) * (self.actor.scale.y * 10000.0));
            self.beam_pos3 = m.transform_point3(sp80);
            // BgCheck_EntityLineTest1(.., checkWall true, checkFloor true, checkCeil false,
            // checkOneFace true, ..).
            if let Some((pos_result, _)) = play.col.entity_line_test(self.beam_pos1, self.beam_pos3, true, true, false, true) {
                self.beam_scale.z = vec3f_dist_xyz(self.beam_pos1, pos_result) - 5.0;
                self.unk_260 = 4;
                self.beam_pos3 = pos_result;
            }
            if self.beam_scale.z != 0.0 {
                let dist = if self.actor.scale.y > 0.01 { 70.0 } else { 100.0 };
                let len = vec3f_dist_xyz(self.beam_pos1, self.beam_pos3) * dist;
                let q3 = m.transform_point3(BEAM_NEAR[0]);
                let q2 = m.transform_point3(BEAM_NEAR[1]);
                let q1 = m.transform_point3(Vec3::new(500.0, 0.0, len));
                let q0 = m.transform_point3(Vec3::new(-500.0, 0.0, len));
                self.collider_quad1.set_vertices(q0, q1, q2, q3);
            }
        }
        let q1 = m.transform_point3(EYE_QUAD[0]);
        let q0 = m.transform_point3(EYE_QUAD[1]);
        let q3 = m.transform_point3(EYE_QUAD[2]);
        let q2 = m.transform_point3(EYE_QUAD[3]);
        self.collider_quad2.set_vertices(q0, q1, q2, q3);
    }

    /// `SkelAnime_DrawOpa`'s matrices with `EnVm_OverrideLimbDraw`'s turn of the head (limb 2:
    /// `beamRot.x` on its x, `headRotY` on its y), and limb 2's matrix as `EnVm_PostLimbDraw`
    /// gets it.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], beam_rot_x: i16, head_rot_y: i16) -> (Vec<Mat4>, Mat4) {
        let mut head = Mat4::IDENTITY;
        let bones = draw_opa_pose(
            skeleton,
            joints,
            |limb, _pos, rot| {
                if limb == LIMB_HEAD {
                    rot[0] = rot[0].wrapping_add(beam_rot_x);
                    rot[1] = rot[1].wrapping_add(head_rot_y);
                }
                LimbDraw::Default(Mat4::IDENTITY)
            },
            |limb, m| {
                if limb == LIMB_HEAD {
                    head = m;
                }
            },
        );
        (bones, head)
    }
}

/// `EnVm_Draw`'s meshes: `gBeamosLaserDL` with its texture's scroll on segment 8 dynamic (it sets
/// its own modes). The skeleton is the pack's `gBeamosSkel`; the flame at the beam's end is
/// `Effect_Ss_Dead_Db`'s bakes (the same setup, colours and list).
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE_LASER.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_08, BakeSegment::Dynamic(laser_scroll(0)))],
        prelude: Vec::new(),
        body: BakeBody::DLists(vec![(OBJECT.into(), LASER_DL.into())]),
    }]
}

const SEG_08: u8 = 0x08;
/// The colour segment of `Effect_Ss_Dead_Db`'s bakes (`effect::SEG_COLOR`).
const SEG_FLAME_COLOR: u8 = 0x0E;
const BAKE_LASER: &str = "En_Vm/laser";

/// `func_80094E78(gfxCtx, 0, beamTexScroll)`: `Gfx_TexScroll(gfxCtx, 0, y, 0, 0)` (the s16 as a
/// u32).
fn laser_scroll(y: u32) -> Vec<(u32, u32)> {
    oot_game::scene_table::gfx_tex_scroll(0, y, 0, 0)
}

/// `render_state`'s layout: switches `unk_21C`, `unk_260`, `beamTexScroll`; angles `beamRot` x, y,
/// z, `headRotY`; values `beamPos1`, `beamPos3`, `beamScale` (x, y, z each).
mod rs {
    pub const POS1: usize = 0;
    pub const POS3: usize = 3;
    pub const SCALE: usize = 6;
}

impl ActorImpl for EnVm {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnVm_Update`: its hits (alive); the beam's scorch where the draw found it on a surface
    /// (`unk_260` 4); the action; the laser's scroll; `NA_SE_EN_BIMOS_ROLL_HEAD` (alive, not
    /// stunned); the cylinder pushing, hit while the colour filter's off and it's alive; the eye's
    /// quad hit; the focus 6500 up (model units) over the shape's offset.
    fn update(&mut self, play: &mut PlayState) {
        if self.actor.col_chk_info.health != 0 {
            self.check_health(play);
        }
        if self.unk_260 == 4 {
            let p = self.beam_pos3;
            play.with_ss(|s| s.dead_ds_spawn_stationary(p, 20, -1, 255, 20));
            func_80033480(play, p, 6.0, 1, 120, 20, 1);
            audio_play_actor_sfx2(play, NA_SE_EN_BIMOS_LAZER_GND - SFX_FLAG);
        }
        match self.action {
            Action::Wait => self.wait(play),
            Action::Attack => self.attack(play),
            Action::Stun => self.stun(),
            Action::Die => self.die(play),
        }
        self.beam_tex_scroll = self.beam_tex_scroll.wrapping_add(3 << 2);
        if self.actor.col_chk_info.health != 0 && self.unk_21c != 2 {
            audio_play_actor_sfx2(play, NA_SE_EN_BIMOS_ROLL_HEAD - SFX_FLAG);
        }
        self.collider_cylinder.update(&self.actor);
        play.collision_check_set_oc(&self.actor, COL_CYLINDER, &mut self.collider_cylinder);
        if self.actor.color_filter_timer == 0 && self.actor.col_chk_info.health != 0 {
            play.collision_check_set_ac(&self.actor, COL_CYLINDER, &mut self.collider_cylinder);
        }
        play.collision_check_set_ac(&self.actor, COL_QUAD2, &mut self.collider_quad2);
        self.actor.focus_pos = self.actor.world_pos;
        self.actor.focus_pos.y += (6500.0 + self.actor.shape_y_offset) * self.actor.scale.y;
    }

    /// `EnVm_Destroy`: the cylinder (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `EnVm_Draw`'s effects on the actor, once per drawn game frame: `EnVm_PostLimbDraw` on limb
    /// 2's matrix (the model's, `Actor_Draw`'s, times the limb's).
    fn draw_update(&mut self, play: &mut PlayState) {
        let Some(skeleton) = self.skeleton.clone() else { return };
        let model = actor_draw_matrix(&RenderState::of(&self.actor));
        let (_, head) = Self::pose(&skeleton, &self.skel.joint_table, self.beam_rot[0], self.head_rot_y);
        self.post_limb_draw_head(play, model * head);
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        r.switches = vec![self.unk_21c as u32, self.unk_260 as u16 as u32, self.beam_tex_scroll as u16 as u32];
        r.angles = vec![self.beam_rot[0], self.beam_rot[1], self.beam_rot[2], self.head_rot_y];
        let (p1, p3, s) = (self.beam_pos1, self.beam_pos3, self.beam_scale);
        r.values = vec![p1.x, p1.y, p1.z, p3.x, p3.y, p3.z, s.x, s.y, s.z];
        r
    }

    /// `EnVm_Draw`: the skeleton (`SkelAnime_DrawOpa`, the head turned, the body hidden while
    /// dying); `func_80033C30`'s circle under it (0.4, 255); firing, the flame at the beam's end
    /// (10 up, 0.8: `gEffEnemyDeathFlameDL` with `D_80B2EB88[gameplayFrames % 8]`, prim (255, 255,
    /// 255, 168), env (0, 0, 255, 0), then turned `Matrix_RotateY(32767.0f)` with texture
    /// `(gameplayFrames + 4) % 8`); the laser from the eye.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let ([unk_21c, unk_260, tex_scroll], [rot_x, rot_y, rot_z, head_rot_y], Some(joints), Some(skeleton)) =
            (rs.switches.as_slice(), rs.angles.as_slice(), &rs.joints, &self.skeleton)
        else {
            return;
        };
        let v = &rs.values;
        if v.len() < 9 {
            return;
        }
        let (mut bones, _) = Self::pose(skeleton, &joints.rot, *rot_x, *head_rot_y);
        if *unk_21c == 3 {
            // EnVm_OverrideLimbDraw: *dList = NULL for limb 10.
            if let Some(b) = bones.get_mut(LIMB_BODY - 1) {
                *b = Mat4::ZERO;
            }
        }
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::mesh(OBJECT, SKEL)), transform: actor_draw_matrix(rs), bones, params: Default::default() });
        // func_80033C30(&actorPos, &D_80B2EB7C, 255, play).
        out.opa.push(oot_game::shadow::func_80033c30(&play.col, rs.pos, SHADOW_SCALE, 255));
        let unk_260 = *unk_260 as u16 as i16;
        if unk_260 >= 3 {
            let p3 = Vec3::new(v[rs::POS3], v[rs::POS3 + 1], v[rs::POS3 + 2]);
            let mut m = MtxF::set_translate(p3.x, p3.y + 10.0, p3.z);
            m.scale(0.8, 0.8, 0.8);
            let prim = [255, 255, 255, 168];
            let env = [0, 0, 255, 0];
            let frames = play.gameplay_frames;
            let flame = |m: &MtxF, n: u32| {
                // gSPSegment(0x08, D_80B2EB88[n % 8]): gEffEnemyDeathFlame1Tex to 8Tex.
                let bake = oot_game::effect::dead_db::bake_name((n % 8) as usize + 1);
                let mut sv = SegmentValues::default();
                sv.prim[SEG_FLAME_COLOR as usize] = Some(prim);
                sv.env[SEG_FLAME_COLOR as usize] = Some(env);
                // The list's gSPMatrix(D_01000000, G_MTX_MUL): the billboard after.
                DrawCmd { mesh: MeshKey::named(keys::bake(&bake)), transform: m.to_mat4() * view.billboard, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } }
            };
            out.xlu.push(flame(&m, frames));
            // @bug (game): 32767.0f radians, not a half turn.
            m.rotate_y(32767.0);
            out.xlu.push(flame(&m, frames.wrapping_add(4)));
        }
        let p1 = Vec3::new(v[rs::POS1], v[rs::POS1 + 1], v[rs::POS1 + 2]);
        let scale = Vec3::new(v[rs::SCALE], v[rs::SCALE + 1], v[rs::SCALE + 2]);
        let mut m = MtxF::set_translate(p1.x, p1.y, p1.z);
        m.rotate_zyx(*rot_x, *rot_y, *rot_z);
        m.scale(scale.x * 0.1, scale.x * 0.1, scale.z * 0.0015);
        let mut sv = SegmentValues::default();
        sv.read(SEG_08, &laser_scroll(*tex_scroll as u16 as i16 as i32 as u32));
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_LASER)), transform: m.to_mat4(), bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_CYLINDER => Some(ColliderMut::Cylinder(&mut self.collider_cylinder)),
            COL_QUAD1 => Some(ColliderMut::Quad(&mut self.collider_quad1)),
            COL_QUAD2 => Some(ColliderMut::Quad(&mut self.collider_quad2)),
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
