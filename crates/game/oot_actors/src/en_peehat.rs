//! `En_Peehat` (`ovl_En_Peehat/z_en_peehat.c`): the Peahat, Hyrule Field's flying plant, and its
//! larvae (GAME-06 milestone 4).
//!
//! Params (`PeahatType`): -1 grounded (the field's), 0 flying (spawns larvae), 1 a larva.
//!
//! - **Grounded** by day: it rises when Link is within 740 (`EnPeehat_Ground_StateRise`), hovers
//!   (`_StateHover`), seeks him spinning its blades at him for 600 frames within 1,200 of home
//!   (`_StateSeekPlayer`), goes home and lands (`_StateReturnHome`, `_StateLanding`). Its blades
//!   (a quad, 16 damage) dig the earth where they cut the ground. By night it stays down, its
//!   root open: a hit there (`EnPeehat_HitWhenGrounded`) wobbles it and lets out its larvae
//!   (three at most), or one in sixteen frames drops three from table 4.
//! - **Flying** by day within 2,800: up and flying, a larva every 8 frames while Link's within
//!   1,400 (three at most; @bug (game): both its landing and its rising reset the count).
//! - **Larva**: falls on Link turning to him, its blades hurt him; it dies on the ground, on a hit
//!   (arrow or slingshot) or on hitting anything but Link (bouncing off his shield it recoils
//!   first), dropping from table 2.
//! - **Hit** (adults, on their root's sphere): a nut or light/ice does nothing (light and ice
//!   stop its updates for good: its damage reaction stays 6); the hookshot kills; the boomerang
//!   stuns it (blue, 80); other damage flashes it red (fire: five flames, 100). At no health it
//!   dies (`EnPeehat_Adult_StateDie`): it shrinks rising, then explodes (an `En_Bom` with no fuse)
//!   and drops three from table 4.
//!
//! The whole overlay is ported. `En_Bom` isn't: its explosion spawns a placeholder (logged).
//! `Actor_CullingVolumeTest` isn't ported: every actor counts as inside its culling volume, so
//! the blades dig wherever it is. The circle shadow isn't drawn (`shadowScale` kept).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::{Cylinder16, Sphere16};
use eng_gfx::{DrawCmd, DrawParams, MeshKey};
use eng_math::{sin_s, smooth_step_to_f, smooth_step_to_s, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, audio_play_actor_sfx2, func_80033480, math_cos_f, math_sin_f};
use oot_game::audio::sfx::*;
use oot_game::collision_check::*;
use oot_game::effect::hahen::HAHEN_OBJECT_DEFAULT;
use oot_game::pack::{BakeBody, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};

/// `ACTOR_EN_PEEHAT` (`actor_table.h`: 0x001D).
pub const ACTOR_EN_PEEHAT: i16 = 0x001D;
pub const OBJECT: &str = "object_peehat";
const SKEL: &str = "gPeehatSkel";

/// `En_Peehat_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, always updating, its body
/// hits sounding for Link's.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_PEEHAT,
    name: "En_Peehat",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT,
    object: OBJECT,
};

/// `ACTOR_EN_BOM` (`actor_table.h`: 0x0010), the bomb its death spawns (not ported).
const ACTOR_EN_BOM: i16 = 0x0010;

/// `PeahatType`.
pub const PEAHAT_TYPE_GROUNDED: i16 = -1;
pub const PEAHAT_TYPE_FLYING: i16 = 0;
pub const PEAHAT_TYPE_LARVA: i16 = 1;

/// `GROUND_HOVER_HEIGHT` (unused by the C as well), `MAX_LARVA`.
const MAX_LARVA: i16 = 3;

/// `NAVI_ENEMY_PEAHAT`, `NAVI_ENEMY_PEAHAT_LARVA` (`actor.h`).
const NAVI_ENEMY_PEAHAT: u8 = 0x48;
const NAVI_ENEMY_PEAHAT_LARVA: u8 = 0x49;

/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_2 / _4, false)`.
const DROP_TABLE_2: i16 = 2 * 16;
const DROP_TABLE_4: i16 = 4 * 16;

/// `PeahatState`.
pub const PEAHAT_STATE_DYING: i32 = 0;
pub const PEAHAT_STATE_EXPLODE: i32 = 1;
pub const PEAHAT_STATE_3: i32 = 3;
pub const PEAHAT_STATE_4: i32 = 4;
pub const PEAHAT_STATE_FLY: i32 = 5;
pub const PEAHAT_STATE_ATTACK_RECOIL: i32 = 7;
pub const PEAHAT_STATE_8: i32 = 8;
pub const PEAHAT_STATE_9: i32 = 9;
pub const PEAHAT_STATE_LANDING: i32 = 10;
pub const PEAHAT_STATE_RETURN_HOME: i32 = 12;
pub const PEAHAT_STATE_STUNNED: i32 = 13;
pub const PEAHAT_STATE_SEEK_PLAYER: i32 = 14;
pub const PEAHAT_STATE_15: i32 = 15;

/// `EnPeehatDamageReaction`.
pub const PEAHAT_DMG_REACT_ATTACK: u8 = 0;
pub const PEAHAT_DMG_REACT_LIGHT_ICE_ARROW: u8 = 6;
pub const PEAHAT_DMG_REACT_FIRE: u8 = 12;
pub const PEAHAT_DMG_REACT_HOOKSHOT: u8 = 13;
pub const PEAHAT_DMG_REACT_BOOMERANG: u8 = 14;
pub const PEAHAT_DMG_REACT_NUT: u8 = 15;

/// The limbs the draw changes (1-based): the body (jiggling), the blades' root (spinning), the
/// top (jiggling on the ground).
const LIMB_BODY: usize = 3;
const LIMB_BLADES: usize = 4;
const LIMB_TOP: usize = 23;

/// `sCylinderInit`: its root, hit by all but the shield and the mirror's ray.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_WOOD, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 50, height: 160, y_shift: -70, pos: [0, 0, 0] },
};

/// `sJntSphElementsInit`: its weak point, under the body.
fn jnt_sph_elements() -> [ColliderJntSphElementInit; 1] {
    [ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_ON,
        },
        limb: 0,
        model_sphere: Sphere16 { center: [0, 0, 0], radius: 20 },
        scale: 100,
    }]
}

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sQuadInit`: the blades, 16 damage, metal to the sword.
const QUAD_INIT: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_METAL, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x10 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, PEAHAT_DMG_REACT_NUT),             // Deku nut
        dmg_entry(2, PEAHAT_DMG_REACT_ATTACK),          // Deku stick
        dmg_entry(1, PEAHAT_DMG_REACT_ATTACK),          // Slingshot
        dmg_entry(2, PEAHAT_DMG_REACT_ATTACK),          // Explosive
        dmg_entry(0, PEAHAT_DMG_REACT_BOOMERANG),       // Boomerang
        dmg_entry(2, PEAHAT_DMG_REACT_ATTACK),          // Normal arrow
        dmg_entry(2, PEAHAT_DMG_REACT_ATTACK),          // Hammer swing
        dmg_entry(2, PEAHAT_DMG_REACT_HOOKSHOT),        // Hookshot
        dmg_entry(1, PEAHAT_DMG_REACT_ATTACK),          // Kokiri sword
        dmg_entry(2, PEAHAT_DMG_REACT_ATTACK),          // Master sword
        dmg_entry(4, PEAHAT_DMG_REACT_ATTACK),          // Giant's Knife
        dmg_entry(4, PEAHAT_DMG_REACT_FIRE),            // Fire arrow
        dmg_entry(2, PEAHAT_DMG_REACT_ATTACK),          // Ice arrow
        dmg_entry(2, PEAHAT_DMG_REACT_ATTACK),          // Light arrow
        dmg_entry(2, PEAHAT_DMG_REACT_ATTACK),          // Unk arrow 1
        dmg_entry(2, PEAHAT_DMG_REACT_ATTACK),          // Unk arrow 2
        dmg_entry(2, PEAHAT_DMG_REACT_ATTACK),          // Unk arrow 3
        dmg_entry(3, PEAHAT_DMG_REACT_FIRE),            // Fire magic
        dmg_entry(0, PEAHAT_DMG_REACT_LIGHT_ICE_ARROW), // Ice magic
        dmg_entry(0, PEAHAT_DMG_REACT_LIGHT_ICE_ARROW), // Light magic
        dmg_entry(0, PEAHAT_DMG_REACT_ATTACK),          // Shield
        dmg_entry(0, PEAHAT_DMG_REACT_ATTACK),          // Mirror Ray
        dmg_entry(1, PEAHAT_DMG_REACT_ATTACK),          // Kokiri spin
        dmg_entry(4, PEAHAT_DMG_REACT_ATTACK),          // Giant spin
        dmg_entry(2, PEAHAT_DMG_REACT_ATTACK),          // Master spin
        dmg_entry(2, PEAHAT_DMG_REACT_ATTACK),          // Kokiri jump
        dmg_entry(8, PEAHAT_DMG_REACT_ATTACK),          // Giant jump
        dmg_entry(4, PEAHAT_DMG_REACT_ATTACK),          // Master jump
        dmg_entry(0, PEAHAT_DMG_REACT_ATTACK),          // Unknown 1
        dmg_entry(0, PEAHAT_DMG_REACT_ATTACK),          // Unblockable
        dmg_entry(4, PEAHAT_DMG_REACT_ATTACK),          // Hammer jump
        dmg_entry(0, PEAHAT_DMG_REACT_ATTACK),          // Unknown 2
    ],
};

/// `peahatBladeTip`: where the blades' ends are on their limb.
const PEAHAT_BLADE_TIP: [Vec3; 2] = [Vec3::new(0.0, 0.0, 5500.0), Vec3::new(0.0, 0.0, -5500.0)];
/// `D_80AD285C`: the blades' quad on the model.
const D_80AD285C: [Vec3; 4] = [Vec3::new(0.0, 0.0, -4500.0), Vec3::new(-4500.0, 0.0, 0.0), Vec3::new(4500.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 4500.0)];

/// The skeleton's bake without its body and top (drawn on their own), the body's and the top's.
const BAKE: &str = "En_Peehat/skeleton";
const BAKE_BODY: &str = "En_Peehat/body";
const BAKE_TOP: &str = "En_Peehat/top";

pub fn bakes() -> Vec<MeshBake> {
    let none = |limb: usize| LimbOverride { limb: (limb - 1) as u8, file: OBJECT.into(), symbol: String::new() };
    let dl = |name: &str, sym: &str| MeshBake { name: name.into(), object: OBJECT.into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::DLists(vec![(OBJECT.into(), sym.into())]) };
    vec![
        MeshBake {
            name: BAKE.into(),
            object: OBJECT.into(),
            segments: Vec::new(),
            prelude: Vec::new(),
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: vec![none(LIMB_BODY), none(LIMB_TOP)] },
        },
        dl(BAKE_BODY, "gPeehatBodyDL"),
        dl(BAKE_TOP, "gPeehatTopDL"),
    ]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    GroundStateGround,
    FlyingStateGrounded,
    FlyingStateFly,
    GroundStateRise,
    FlyingStateRise,
    GroundStateSeekPlayer,
    LarvaStateSeekPlayer,
    GroundStateLanding,
    FlyingStateLanding,
    GroundStateHover,
    GroundStateReturnHome,
    StateAttackRecoil,
    StateBoomerangStunned,
    AdultStateDie,
    StateExplode,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::GroundStateGround => "EnPeehat_Ground_StateGround",
            Action::FlyingStateGrounded => "EnPeehat_Flying_StateGrounded",
            Action::FlyingStateFly => "EnPeehat_Flying_StateFly",
            Action::GroundStateRise => "EnPeehat_Ground_StateRise",
            Action::FlyingStateRise => "EnPeehat_Flying_StateRise",
            Action::GroundStateSeekPlayer => "EnPeehat_Ground_StateSeekPlayer",
            Action::LarvaStateSeekPlayer => "EnPeehat_Larva_StateSeekPlayer",
            Action::GroundStateLanding => "EnPeehat_Ground_StateLanding",
            Action::FlyingStateLanding => "EnPeehat_Flying_StateLanding",
            Action::GroundStateHover => "EnPeehat_Ground_StateHover",
            Action::GroundStateReturnHome => "EnPeehat_Ground_StateReturnHome",
            Action::StateAttackRecoil => "EnPeehat_StateAttackRecoil",
            Action::StateBoomerangStunned => "EnPeehat_StateBoomerangStunned",
            Action::AdultStateDie => "EnPeehat_Adult_StateDie",
            Action::StateExplode => "EnPeehat_StateExplode",
        }
    }
}

struct Anims {
    landing: Anim,
    flying: Anim,
    recoil: Anim,
    rising: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Peehat: {e:#}")).ok();
        Some(Anims { landing: get("gPeehatLandingAnim")?, flying: get("gPeehatFlyingAnim")?, recoil: get("gPeehatRecoilAnim")?, rising: get("gPeehatRisingAnim")? })
    }
}

/// Where the draw's matrices put things (`EnPeehat_PostLimbDraw`, `EnPeehat_Draw`).
struct Pose {
    /// Every limb's matrix in the model's space.
    limbs: Vec<Mat4>,
    /// The stack after the body (its parent's: the override drew it).
    body_stack: Mat4,
    /// The stack after the blades' root.
    blades_stack: Mat4,
}

pub struct EnPeehat {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    pub state: i32,
    pub is_state_die_first_update: bool,
    pub action: Action,
    /// `bladeTip`: where the blades' ends are, to dig the earth.
    pub blade_tip: [Vec3; 2],
    pub unk_2d4: i32,
    /// `xzDistMax`: how far from home it follows Link (grounded), or spawns larvae (flying).
    pub xz_dist_max: f32,
    /// `xzDistToRise`.
    pub xz_dist_to_rise: f32,
    pub unk_2e0: f32,
    pub jiggle_rot: f32,
    pub jiggle_rot_inc: f32,
    /// `scaleShift`: the jiggle's squash.
    pub scale_shift: f32,
    pub blade_rot_vel: i16,
    pub blade_rot: i16,
    pub unk_2f4: i16,
    pub rise_delay_timer: i16,
    pub seek_player_timer: i16,
    /// `unk_2FA`: its larvae (flying, grounded), and which way it turns (grounded).
    pub unk_2fa: i16,
    pub anim_timer: i16,
    pub collider_cylinder: ColliderCylinder,
    pub collider_jnt_sph: ColliderJntSph,
    pub collider_quad: ColliderQuad,
    /// `shape.shadowScale` (the circle shadow isn't drawn).
    pub shadow_scale: f32,
}

const COL_CYLINDER: u8 = 0;
const COL_JNT_SPH: u8 = 1;
const COL_QUAD: u8 = 2;

impl EnPeehat {
    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `Animation_Change(gPeehatRisingAnim, 0, 3, last, ONCE, 0)`: held on its fourth frame.
    fn hold_rising(&mut self) {
        if let Some(a) = self.anim(|a| &a.rising) {
            let last = a.last_frame();
            self.skel.change(a, 0.0, 3.0, last, ANIMMODE_ONCE, 0.0);
        }
    }

    /// `Animation_PlayLoop`.
    fn play_loop(&mut self, f: impl Fn(&Anims) -> &Anim) {
        if let Some(a) = self.anim(f) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
        }
    }

    /// `Animation_PlayOnce` / `Animation_MorphToPlayOnce`.
    fn play_once(&mut self, f: impl Fn(&Anims) -> &Anim, morph: f32) {
        if let Some(a) = self.anim(f) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, morph);
        }
    }

    fn rising_last_frame(&self) -> f32 {
        self.anims.as_ref().map(|a| a.rising.last_frame()).unwrap_or(0.0)
    }

    fn player_pos(play: &PlayState) -> Vec3 {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default()
    }

    /// `EnPeehat_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: lockOnArrowOffset 700.
        actor.target_arrow_offset = 700.0;
        actor.scale = Vec3::splat(36.0 * 0.001);
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Peehat: {e:#}")).ok());
        let anims = Anims::load(play);
        let skel = SkelAnimeStd::init_flex(24, anims.as_ref().map(|a| a.rising.clone()));
        // ActorShape_Init(100, ActorShadow_DrawCircle, 27).
        actor.shape_y_offset = 100.0;
        actor.focus_pos = actor.world_pos;
        actor.world_rot.y = 0;
        actor.col_chk_info.mass = MASS_HEAVY;
        actor.col_chk_info.health = 6;
        actor.col_chk_info.damage_table = Some(&S_DAMAGE_TABLE);
        actor.floor_height = actor.world_pos.y;
        let collider_cylinder = ColliderCylinder::new(&CYLINDER_INIT);
        let collider_quad = ColliderQuad::new(&QUAD_INIT);
        let collider_jnt_sph = ColliderJntSph::new(&JNT_SPH_INIT, &jnt_sph_elements());
        actor.navi_enemy_id = NAVI_ENEMY_PEAHAT;
        actor.culling_volume_distance = 4000.0;
        actor.culling_volume_scale = 800.0;
        actor.culling_volume_downward = 1800.0;
        let mut this = EnPeehat {
            actor,
            skel,
            skeleton,
            anims,
            state: 0,
            is_state_die_first_update: false,
            action: Action::GroundStateGround,
            blade_tip: [Vec3::ZERO; 2],
            unk_2d4: 0,
            xz_dist_max: 1200.0,
            xz_dist_to_rise: 740.0,
            unk_2e0: 0.0,
            jiggle_rot: 0.0,
            jiggle_rot_inc: 0.0,
            scale_shift: 0.0,
            blade_rot_vel: 0,
            blade_rot: 0,
            unk_2f4: 0,
            rise_delay_timer: 0,
            seek_player_timer: 0,
            unk_2fa: 0,
            anim_timer: 0,
            collider_cylinder,
            collider_jnt_sph,
            collider_quad,
            shadow_scale: 27.0,
        };
        match this.actor.params {
            PEAHAT_TYPE_GROUNDED => this.ground_set_state_ground(),
            PEAHAT_TYPE_FLYING => {
                this.actor.culling_volume_distance = 4200.0;
                this.xz_dist_to_rise = 2800.0;
                this.xz_dist_max = 1400.0;
                this.flying_set_state_ground();
                this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            }
            PEAHAT_TYPE_LARVA => {
                this.actor.scale.x = 0.006;
                this.actor.scale.z = 0.006;
                this.actor.scale.y = 0.003;
                this.collider_cylinder.dim.radius = 25;
                this.collider_cylinder.dim.height = 15;
                this.collider_cylinder.dim.y_shift = -5;
                this.collider_cylinder.info.ac_dmg_info.dmg_flags = DMG_ARROW | DMG_SLINGSHOT;
                this.collider_quad.base.at_flags = AT_ON | AT_TYPE_ENEMY;
                this.collider_quad.base.ac_flags = AC_ON | AC_TYPE_PLAYER;
                this.actor.navi_enemy_id = NAVI_ENEMY_PEAHAT_LARVA;
                this.larva_set_state_seek_player();
            }
            _ => {}
        }
        Box::new(this)
    }

    /// `EnPeehat_SpawnDust`: a piece of earth (`EffectSsHahen`) `arg3` from `pos` at a random
    /// angle.
    #[allow(clippy::too_many_arguments)]
    fn spawn_dust(&self, play: &mut PlayState, pos: Vec3, arg3: f32, arg4: i16, arg5: f32, arg6: f32) {
        let mut dust_vel = Vec3::new(0.0, 8.0, 0.0);
        let mut dust_accel = Vec3::new(0.0, -1.5, 0.0);
        let rot = (play.rand.zero_one() - 0.5) * 6.28;
        let dust_pos = Vec3::new(math_sin_f(rot) * arg3 + pos.x, self.actor.floor_height, math_cos_f(rot) * arg3 + pos.z);
        dust_accel.x = (play.rand.zero_one() - 0.5) * arg5;
        dust_accel.z = (play.rand.zero_one() - 0.5) * arg5;
        dust_vel.y += (play.rand.zero_one() - 0.5) * 4.0;
        let p_scale = ((play.rand.zero_one() * 5.0 + 12.0) * arg6) as i32;
        play.with_ss(|s| s.hahen_spawn(dust_pos, dust_vel, dust_accel, arg4, p_scale as i16, HAHEN_OBJECT_DEFAULT, 10, None));
    }

    /// `Actor_SpawnAsChild(..., ACTOR_EN_PEEHAT, ..., PEAHAT_TYPE_LARVA)`, turned a random way.
    fn spawn_larva(&mut self, play: &mut PlayState, pos: Vec3, velocity_y: Option<f32>) -> bool {
        match play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_PEEHAT, pos, [0, 0, 0], PEAHAT_TYPE_LARVA) {
            Ok(h) => {
                let yaw = play.rand.centered_float(65535.0) as i32 as i16;
                if let Some(larva) = play.actors.actor_mut(h) {
                    if let Some(vy) = velocity_y {
                        larva.velocity.y = vy;
                    }
                    larva.world_rot.y = yaw;
                    larva.shape_rot.y = yaw;
                }
                true
            }
            Err(e) => {
                log::debug!("En_Peehat: larva not spawned: {e:?}");
                false
            }
        }
    }

    /// `EnPeehat_HitWhenGrounded`: one frame in sixteen, three drops from table 4 (and 240
    /// frames' wobble); otherwise its larvae out (up to three), and 8.
    fn hit_when_grounded(&mut self, play: &mut PlayState) {
        self.collider_cylinder.base.ac_flags &= !AC_HIT;
        if play.gameplay_frames & 0xF == 0 {
            let mut item_drop_pos = self.actor.world_pos;
            item_drop_pos.y += 70.0;
            for _ in 0..3 {
                crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), item_drop_pos, DROP_TABLE_4);
            }
            self.unk_2d4 = 240;
        } else {
            self.collider_cylinder.base.ac_flags &= !AC_HIT;
            let mut i = MAX_LARVA - self.unk_2fa;
            while i > 0 {
                let w = self.actor.world_pos;
                let x = play.rand.centered_float(25.0) + w.x;
                let y = play.rand.centered_float(25.0) + (w.y + 50.0);
                let z = play.rand.centered_float(25.0) + w.z;
                if self.spawn_larva(play, Vec3::new(x, y, z), Some(6.0)) {
                    self.unk_2fa += 1;
                }
                i -= 1;
            }
            self.unk_2d4 = 8;
        }
        audio_play_actor_sfx2(play, NA_SE_EN_PIHAT_DAMAGE);
    }

    /// `EnPeehat_Ground_SetStateGround`.
    fn ground_set_state_ground(&mut self) {
        self.hold_rising();
        self.seek_player_timer = 600;
        self.unk_2d4 = 0;
        self.unk_2fa = 0;
        self.state = PEAHAT_STATE_3;
        self.collider_cylinder.base.ac_flags &= !AC_HIT;
        self.action = Action::GroundStateGround;
    }

    /// The night's wobble, or a hit on its root (`EnPeehat_Ground_StateGround`,
    /// `EnPeehat_Flying_StateGrounded`).
    fn grounded_night(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.shape_y_offset, -1000.0, 1.0, 50.0, 0.0);
        if self.unk_2d4 != 0 {
            self.unk_2d4 -= 1;
            if self.unk_2d4 & 4 != 0 {
                smooth_step_to_f(&mut self.scale_shift, 0.205, 1.0, 0.235, 0.0);
            } else {
                smooth_step_to_f(&mut self.scale_shift, 0.0, 1.0, 0.005, 0.0);
            }
        } else if self.collider_cylinder.base.ac_flags & AC_HIT != 0 {
            self.hit_when_grounded(play);
        }
    }

    /// `EnPeehat_Ground_StateGround`.
    fn ground_state_ground(&mut self, play: &mut PlayState) {
        if play.save.is_day() {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            if self.rise_delay_timer == 0 {
                if self.actor.xz_dist_to_player < self.xz_dist_to_rise {
                    self.ground_set_state_rise(play);
                }
            } else {
                smooth_step_to_f(&mut self.actor.shape_y_offset, -1000.0, 1.0, 10.0, 0.0);
                self.rise_delay_timer -= 1;
            }
        } else {
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            self.grounded_night(play);
        }
    }

    /// `EnPeehat_Flying_SetStateGround`.
    fn flying_set_state_ground(&mut self) {
        self.hold_rising();
        self.seek_player_timer = 400;
        self.unk_2d4 = 0;
        // @bug (game): overwrites its larvae's count, allowing more than MAX_LARVA.
        self.unk_2fa = 0;
        self.state = PEAHAT_STATE_4;
        self.action = Action::FlyingStateGrounded;
    }

    /// `EnPeehat_Flying_StateGrounded`.
    fn flying_state_grounded(&mut self, play: &mut PlayState) {
        if play.save.is_day() {
            if self.actor.xz_dist_to_player < self.xz_dist_to_rise {
                self.flying_set_state_rise(play);
            }
        } else {
            self.grounded_night(play);
        }
    }

    /// `EnPeehat_Flying_SetStateFly`.
    fn flying_set_state_fly(&mut self) {
        self.play_loop(|a| &a.flying);
        self.state = PEAHAT_STATE_FLY;
        self.action = Action::FlyingStateFly;
    }

    /// `EnPeehat_Flying_StateFly`.
    fn flying_state_fly(&mut self, play: &mut PlayState) {
        audio_play_actor_sfx2(play, NA_SE_EN_PIHAT_FLY - SFX_FLAG);
        self.skel.update();
        if !play.save.is_day() || self.xz_dist_to_rise < self.actor.xz_dist_to_player {
            self.flying_set_state_landing();
        } else if self.actor.xz_dist_to_player < self.xz_dist_max && self.unk_2fa < MAX_LARVA && play.gameplay_frames & 7 == 0 {
            let w = self.actor.world_pos;
            let x = play.rand.centered_float(25.0) + w.x;
            let y = play.rand.centered_float(5.0) + w.y;
            let z = play.rand.centered_float(25.0) + w.z;
            if self.spawn_larva(play, Vec3::new(x, y, z), None) {
                self.unk_2fa += 1;
            }
        }
        self.blade_rot = self.blade_rot.wrapping_add(self.blade_rot_vel);
    }

    /// `EnPeehat_Ground_SetStateRise`.
    fn ground_set_state_rise(&mut self, play: &mut PlayState) {
        let last_frame = self.rising_last_frame();
        if self.state != PEAHAT_STATE_STUNNED {
            self.hold_rising();
        }
        self.state = PEAHAT_STATE_8;
        self.anim_timer = last_frame as i16;
        audio_play_actor_sfx2(play, NA_SE_EN_PIHAT_UP);
        self.action = Action::GroundStateRise;
    }

    /// The rise's common steps (`EnPeehat_Ground_StateRise`, `EnPeehat_Flying_StateRise`): its
    /// blades up to speed, then 40 frames of the animation (from its held frame), rising `step`
    /// a frame; dust near the ground. True when it's done.
    fn rise(&mut self, play: &mut PlayState, step: f32) -> bool {
        let mut done = false;
        smooth_step_to_f(&mut self.actor.shape_y_offset, 0.0, 1.0, 50.0, 0.0);
        if smooth_step_to_s(&mut self.blade_rot_vel, 4000, 1, 800, 0) == 0 {
            if self.anim_timer != 0 {
                self.anim_timer -= 1;
                if self.skel.play_speed == 0.0 && self.anim_timer == 0 {
                    self.anim_timer = 40;
                    self.skel.play_speed = 1.0;
                }
            }
            if self.skel.update() || self.anim_timer == 0 {
                done = true;
            } else {
                self.actor.world_pos.y += step;
            }
            if self.actor.world_pos.y - self.actor.floor_height < 80.0 {
                let mut pos = self.actor.world_pos;
                pos.y = self.actor.floor_height;
                func_80033480(play, pos, 90.0, 1, 0x96, 100, 1);
            }
        }
        done
    }

    /// `EnPeehat_Ground_StateRise`.
    fn ground_state_rise(&mut self, play: &mut PlayState) {
        if self.rise(play, 6.5) {
            self.ground_set_state_hover(play);
        }
        let pos = self.actor.world_pos;
        self.spawn_dust(play, pos, 75.0, 2, 1.05, 2.0);
        smooth_step_to_f(&mut self.scale_shift, 0.075, 1.0, 0.005, 0.0);
        self.blade_rot = self.blade_rot.wrapping_add(self.blade_rot_vel);
    }

    /// `EnPeehat_Flying_SetStateRise`.
    fn flying_set_state_rise(&mut self, play: &mut PlayState) {
        let last_frame = self.rising_last_frame();
        if self.state != PEAHAT_STATE_STUNNED {
            self.hold_rising();
        }
        self.state = PEAHAT_STATE_9;
        self.anim_timer = last_frame as i16;
        audio_play_actor_sfx2(play, NA_SE_EN_PIHAT_UP);
        self.action = Action::FlyingStateRise;
    }

    /// `EnPeehat_Flying_StateRise`.
    fn flying_state_rise(&mut self, play: &mut PlayState) {
        if self.rise(play, 18.0) {
            // @bug (game): overwrites its larvae's count, allowing more than MAX_LARVA.
            self.unk_2fa = 0;
            self.flying_set_state_fly();
        }
        let pos = self.actor.world_pos;
        self.spawn_dust(play, pos, 75.0, 2, 1.05, 2.0);
        smooth_step_to_f(&mut self.scale_shift, 0.075, 1.0, 0.005, 0.0);
        self.blade_rot = self.blade_rot.wrapping_add(self.blade_rot_vel);
    }

    /// `EnPeehat_Ground_SetStateSeekPlayer`.
    fn ground_set_state_seek_player(&mut self) {
        self.play_loop(|a| &a.flying);
        self.state = PEAHAT_STATE_SEEK_PLAYER;
        self.unk_2e0 = 0.0;
        self.action = Action::GroundStateSeekPlayer;
    }

    /// `EnPeehat_Ground_StateSeekPlayer`.
    fn ground_state_seek_player(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.speed_xz, 3.0, 1.0, 0.25, 0.0);
        let target = self.actor.floor_height + 80.0;
        smooth_step_to_f(&mut self.actor.world_pos.y, target, 1.0, 3.0, 0.0);
        if self.seek_player_timer <= 0 {
            self.ground_set_state_landing();
            self.rise_delay_timer = 40;
        } else {
            self.seek_player_timer -= 1;
        }
        if play.save.is_day() && dist_xz(self.actor.home_pos, Self::player_pos(play)) < self.xz_dist_max {
            smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 1, 1000, 0);
            if self.unk_2fa != 0 {
                self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x1C2);
            } else {
                self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_sub(0x1C2);
            }
        } else {
            self.ground_set_state_return_home();
        }
        self.skel.update();
        smooth_step_to_s(&mut self.blade_rot_vel, 4000, 1, 500, 0);
        self.blade_rot = self.blade_rot.wrapping_add(self.blade_rot_vel);
        smooth_step_to_f(&mut self.scale_shift, 0.075, 1.0, 0.005, 0.0);
        audio_play_actor_sfx2(play, NA_SE_EN_PIHAT_FLY - SFX_FLAG);
    }

    /// `EnPeehat_Larva_SetStateSeekPlayer`.
    fn larva_set_state_seek_player(&mut self) {
        self.play_loop(|a| &a.flying);
        self.state = PEAHAT_STATE_SEEK_PLAYER;
        self.unk_2d4 = 0;
        self.action = Action::LarvaStateSeekPlayer;
    }

    /// `EnPeehat_SpawnDeadDb`s of the larva's death: five white puffs.
    fn larva_puffs(&self, play: &mut PlayState) {
        for _ in 0..5 {
            let w = self.actor.world_pos;
            let x = play.rand.centered_float(20.0) + w.x;
            let y = play.rand.centered_float(10.0) + w.y;
            let z = play.rand.centered_float(20.0) + w.z;
            play.with_ss(|s| s.dead_db_spawn(Vec3::new(x, y, z), Vec3::ZERO, Vec3::ZERO, 40, 7, [255, 255, 255, 255], [255, 0, 0], 1, 9, 1));
        }
    }

    /// `EnPeehat_Larva_StateSeekPlayer`.
    fn larva_state_seek_player(&mut self, play: &mut PlayState) {
        let mut speed_xz = 5.3;
        if self.actor.xz_dist_to_player <= 5.3 {
            speed_xz = self.actor.xz_dist_to_player + 0.0005;
        }
        if self.actor.parent.is_some_and(|h| play.actors.actor(h).is_none_or(|a| a.killed)) {
            self.actor.parent = None;
        }
        self.actor.speed_xz = speed_xz;
        if self.actor.world_pos.y - self.actor.floor_height >= 70.0 {
            smooth_step_to_f(&mut self.actor.velocity.y, -1.3, 1.0, 0.5, 0.0);
        } else {
            smooth_step_to_f(&mut self.actor.velocity.y, -0.135, 1.0, 0.05, 0.0);
        }
        if self.unk_2d4 == 0 {
            smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 1, 830, 0);
        } else {
            self.unk_2d4 -= 1;
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x15E);
        self.skel.update();
        smooth_step_to_s(&mut self.blade_rot_vel, 4000, 1, 500, 0);
        self.blade_rot = self.blade_rot.wrapping_add(self.blade_rot_vel);
        smooth_step_to_f(&mut self.scale_shift, 0.075, 1.0, 0.005, 0.0);
        audio_play_actor_sfx2(play, NA_SE_EN_PIHAT_SM_FLY - SFX_FLAG);
        if self.collider_quad.base.at_flags & AT_BOUNCED != 0 {
            self.actor.col_chk_info.health = 0;
            self.collider_quad.base.ac_flags &= !AC_BOUNCED;
            self.set_state_attack_recoil();
        } else if self.collider_quad.base.at_flags & AT_HIT != 0 || self.collider_cylinder.base.ac_flags & AC_HIT != 0 || self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.collider_quad.base.at_flags &= !AT_HIT;
            let hit_player = play.player.is_some() && self.collider_quad.base.at == play.player;
            if self.collider_cylinder.base.ac_flags & AC_HIT == 0 && hit_player {
                if play.rand.zero_one() > 0.5 {
                    self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(0x2000);
                } else {
                    self.actor.world_rot.y = self.actor.world_rot.y.wrapping_sub(0x2000);
                }
                self.unk_2d4 = 40;
            } else if self.collider_cylinder.base.ac_flags & AC_HIT != 0 || self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.larva_puffs(play);
            }
            if !hit_player || self.collider_cylinder.base.ac_flags & AC_HIT != 0 {
                if self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
                    let pos = self.actor.projected_pos;
                    play.with_ss(|s| s.dead_sound_spawn_stationary(pos, NA_SE_EN_PIHAT_SM_DEAD, 1, 1, 40));
                }
                let pos = self.actor.world_pos;
                crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, DROP_TABLE_2);
                self.actor.kill();
            }
        }
    }

    /// `EnPeehat_Ground_SetStateLanding`.
    fn ground_set_state_landing(&mut self) {
        self.state = PEAHAT_STATE_LANDING;
        self.play_once(|a| &a.landing, 0.0);
        self.action = Action::GroundStateLanding;
    }

    /// The landing's steps (`EnPeehat_Ground_StateLanding`, `EnPeehat_Flying_StateLanding`):
    /// down at `step`, dust under 60. True when it's down.
    fn land(&mut self, play: &mut PlayState, step: f32) -> bool {
        smooth_step_to_f(&mut self.actor.shape_y_offset, -1000.0, 1.0, 50.0, 0.0);
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 1.0, 0.0);
        smooth_step_to_s(&mut self.actor.shape_rot.x, 0, 1, 50, 0);
        let mut landed = false;
        if self.skel.update() {
            landed = true;
        } else if self.actor.floor_height < self.actor.world_pos.y {
            let floor = self.actor.floor_height;
            smooth_step_to_f(&mut self.actor.world_pos.y, floor, 0.3, step, 0.25);
            if self.actor.world_pos.y - self.actor.floor_height < 60.0 {
                let mut pos = self.actor.world_pos;
                pos.y = self.actor.floor_height;
                func_80033480(play, pos, 80.0, 1, 150, 100, 1);
                self.spawn_dust(play, pos, 75.0, 2, 1.05, 2.0);
            }
        }
        landed
    }

    /// `EnPeehat_Ground_StateLanding`.
    fn ground_state_landing(&mut self, play: &mut PlayState) {
        if self.land(play, 3.5) {
            self.ground_set_state_ground();
            self.actor.world_pos.y = self.actor.floor_height;
            audio_play_actor_sfx2(play, NA_SE_EN_PIHAT_LAND);
        }
        smooth_step_to_s(&mut self.blade_rot_vel, 0, 1, 100, 0);
        self.blade_rot = self.blade_rot.wrapping_add(self.blade_rot_vel);
    }

    /// `EnPeehat_Flying_SetStateLanding`.
    fn flying_set_state_landing(&mut self) {
        self.play_once(|a| &a.landing, 0.0);
        self.state = PEAHAT_STATE_LANDING;
        self.action = Action::FlyingStateLanding;
    }

    /// `EnPeehat_Flying_StateLanding`.
    fn flying_state_landing(&mut self, play: &mut PlayState) {
        if self.land(play, 13.5) {
            self.flying_set_state_ground();
            audio_play_actor_sfx2(play, NA_SE_EN_PIHAT_LAND);
            self.actor.world_pos.y = self.actor.floor_height;
        }
        smooth_step_to_s(&mut self.blade_rot_vel, 0, 1, 100, 0);
        self.blade_rot = self.blade_rot.wrapping_add(self.blade_rot_vel);
    }

    /// `EnPeehat_Ground_SetStateHover`.
    fn ground_set_state_hover(&mut self, play: &mut PlayState) {
        self.play_loop(|a| &a.flying);
        self.actor.speed_xz = play.rand.zero_one() * 0.5 + 2.5;
        self.unk_2d4 = (play.rand.zero_one() * 10.0 + 10.0) as i32;
        self.state = PEAHAT_STATE_15;
        self.action = Action::GroundStateHover;
    }

    /// The hover's bob (`unk_2E0`).
    fn bob(&mut self) {
        self.actor.world_pos.y += math_cos_f(self.unk_2e0) * 1.4;
        let cos = math_cos_f(self.unk_2e0) * 0.18;
        self.unk_2e0 += (if 0.0 <= cos { cos } else { -cos }) + 0.07;
    }

    /// `EnPeehat_Ground_StateHover`.
    fn ground_state_hover(&mut self, play: &mut PlayState) {
        if self.actor.world_pos.y - self.actor.floor_height > 75.0 {
            self.actor.world_pos.y -= 1.0;
        }
        self.bob();
        self.unk_2d4 -= 1;
        if self.unk_2d4 <= 0 {
            self.actor.speed_xz = play.rand.zero_one() * 0.5 + 2.5;
            self.unk_2d4 = (play.rand.zero_one() * 10.0 + 10.0) as i32;
            self.unk_2f4 = ((play.rand.zero_one() - 0.5) * 1000.0) as i16;
        }
        self.skel.update();
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(self.unk_2f4);
        if self.seek_player_timer <= 0 {
            self.ground_set_state_landing();
            self.rise_delay_timer = 40;
        } else {
            self.seek_player_timer -= 1;
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x15E);
        if play.save.is_day() && dist_xz(self.actor.home_pos, Self::player_pos(play)) < self.xz_dist_max {
            self.actor.world_rot.y = self.actor.yaw_towards_player;
            self.ground_set_state_seek_player();
            self.unk_2fa = (play.gameplay_frames & 1) as i16;
        } else {
            self.ground_set_state_return_home();
        }
        smooth_step_to_s(&mut self.blade_rot_vel, 4000, 1, 500, 0);
        self.blade_rot = self.blade_rot.wrapping_add(self.blade_rot_vel);
        smooth_step_to_f(&mut self.scale_shift, 0.075, 1.0, 0.005, 0.0);
        audio_play_actor_sfx2(play, NA_SE_EN_PIHAT_FLY - SFX_FLAG);
    }

    /// `EnPeehat_Ground_SetStateReturnHome`.
    fn ground_set_state_return_home(&mut self) {
        self.state = PEAHAT_STATE_RETURN_HOME;
        self.actor.speed_xz = 2.5;
        self.action = Action::GroundStateReturnHome;
    }

    /// `EnPeehat_Ground_StateReturnHome`.
    fn ground_state_return_home(&mut self, play: &mut PlayState) {
        if self.actor.world_pos.y - self.actor.floor_height > 75.0 {
            self.actor.world_pos.y -= 1.0;
        } else {
            self.actor.world_pos.y += 1.0;
        }
        self.bob();
        let y_rot = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
        smooth_step_to_s(&mut self.actor.world_rot.y, y_rot, 1, 600, 0);
        smooth_step_to_s(&mut self.actor.shape_rot.x, 4500, 1, 600, 0);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x15E);
        self.blade_rot = self.blade_rot.wrapping_add(self.blade_rot_vel);
        if dist_xz(self.actor.world_pos, self.actor.home_pos) < 2.0 {
            self.ground_set_state_landing();
            self.rise_delay_timer = 60;
        }
        if play.save.is_day() && dist_xz(self.actor.home_pos, Self::player_pos(play)) < self.xz_dist_max {
            self.seek_player_timer = 400;
            self.ground_set_state_seek_player();
            self.unk_2fa = (play.gameplay_frames & 1) as i16;
        }
        audio_play_actor_sfx2(play, NA_SE_EN_PIHAT_FLY - SFX_FLAG);
    }

    /// `EnPeehat_SetStateAttackRecoil`.
    fn set_state_attack_recoil(&mut self) {
        self.play_once(|a| &a.recoil, -4.0);
        self.state = PEAHAT_STATE_ATTACK_RECOIL;
        self.actor.speed_xz = -9.0;
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        self.action = Action::StateAttackRecoil;
    }

    /// `EnPeehat_StateAttackRecoil`: back to seeking once its speed's back to 0 (a larva dies).
    fn state_attack_recoil(&mut self, play: &mut PlayState) {
        self.blade_rot = self.blade_rot.wrapping_add(self.blade_rot_vel);
        self.skel.update();
        self.actor.speed_xz += 0.5;
        if self.actor.speed_xz == 0.0 {
            if self.actor.params > 0 {
                self.larva_puffs(play);
                self.actor.kill();
            } else {
                self.ground_set_state_seek_player();
                if self.actor.params < 0 {
                    self.unk_2fa = if self.unk_2fa != 0 { 0 } else { 1 };
                }
            }
        }
        audio_play_actor_sfx2(play, NA_SE_EN_PIHAT_FLY - SFX_FLAG);
    }

    /// `EnPeehat_SetStateBoomerangStunned`.
    fn set_state_boomerang_stunned(&mut self, play: &mut PlayState) {
        self.state = PEAHAT_STATE_STUNNED;
        if self.actor.floor_height < self.actor.world_pos.y {
            self.actor.speed_xz = -9.0;
        }
        self.blade_rot_vel = 0;
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 200, COLORFILTER_BUFFLAG_OPA, 80);
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.action = Action::StateBoomerangStunned;
    }

    /// `EnPeehat_StateBoomerangStunned`.
    fn state_boomerang_stunned(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 1.0, 0.0);
        let floor = self.actor.floor_height;
        smooth_step_to_f(&mut self.actor.world_pos.y, floor, 1.0, 8.0, 0.0);
        if self.actor.color_filter_timer == 0 {
            self.ground_set_state_rise(play);
        }
    }

    /// `EnPeehat_Adult_SetStateDie`.
    fn adult_set_state_die(&mut self) {
        self.blade_rot_vel = 0;
        self.is_state_die_first_update = true;
        self.actor.speed_xz = 0.0;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 8);
        self.state = PEAHAT_STATE_DYING;
        self.scale_shift = 0.0;
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        self.action = Action::AdultStateDie;
    }

    /// `EnPeehat_Adult_StateDie`: wobbling, then up spinning (shrinking at no health) to 88.5
    /// over the floor; then it explodes, or hovers (grounded), or flies.
    fn adult_state_die(&mut self, play: &mut PlayState) {
        if self.is_state_die_first_update {
            self.unk_2d4 -= 1;
            if self.unk_2d4 <= 0 || self.actor.col_chk_info.health == 0 {
                self.play_once(|a| &a.recoil, -4.0);
                self.blade_rot_vel = 4000;
                self.unk_2d4 = 14;
                self.actor.speed_xz = 0.0;
                self.actor.velocity.y = 6.0;
                self.is_state_die_first_update = false;
                self.actor.shape_rot.x = 0;
                self.actor.shape_rot.z = 0;
            } else if self.actor.color_filter_timer & 4 != 0 {
                smooth_step_to_f(&mut self.scale_shift, 0.205, 1.0, 0.235, 0.0);
            } else {
                smooth_step_to_f(&mut self.scale_shift, 0.0, 1.0, 0.005, 0.0);
            }
        } else {
            self.skel.update();
            self.blade_rot = self.blade_rot.wrapping_add(self.blade_rot_vel);
            smooth_step_to_s(&mut self.blade_rot_vel, 4000, 1, 250, 0);
            if self.actor.col_chk_info.health == 0 {
                self.actor.scale.x -= 0.0015;
                self.actor.scale = Vec3::splat(self.actor.scale.x);
            }
            let target = self.actor.floor_height + 88.5;
            if smooth_step_to_f(&mut self.actor.world_pos.y, target, 1.0, 3.0, 0.0) == 0.0 && self.actor.world_pos.y - self.actor.floor_height < 59.0 {
                let mut pos = self.actor.world_pos;
                pos.y = self.actor.floor_height;
                func_80033480(play, pos, 80.0, 1, 150, 100, 1);
                self.spawn_dust(play, pos, 75.0, 2, 1.05, 2.0);
            }
            if self.actor.speed_xz < 0.0 {
                self.actor.speed_xz += 0.25;
            }
            self.unk_2d4 -= 1;
            if self.unk_2d4 <= 0 {
                if self.actor.col_chk_info.health == 0 {
                    self.set_state_explode();
                } else if self.actor.params < 0 {
                    self.ground_set_state_hover(play);
                    self.rise_delay_timer = 60;
                } else {
                    self.flying_set_state_fly();
                }
            }
        }
    }

    /// `EnPeehat_SetStateExplode`.
    fn set_state_explode(&mut self) {
        self.play_loop(|a| &a.flying);
        self.state = PEAHAT_STATE_EXPLODE;
        self.anim_timer = 5;
        self.unk_2e0 = 0.0;
        self.action = Action::StateExplode;
    }

    /// `EnPeehat_StateExplode`: a bomb with no fuse (`En_Bom`, `timer` 0), then three drops from
    /// table 4 five frames on.
    fn state_explode(&mut self, play: &mut PlayState) {
        if self.anim_timer == 5 {
            match play.actor_spawn(ACTOR_EN_BOM, self.actor.world_pos, [0, 0, 0x602], 0) {
                // The bomb's timer = 0: En_Bom isn't ported (its placeholder has no timer).
                Ok(_) => log::info!("En_Peehat: its explosion (En_Bom) is a placeholder"),
                Err(e) => log::debug!("En_Peehat: no bomb: {e:?}"),
            }
        }
        self.anim_timer -= 1;
        if self.anim_timer == 0 {
            let pos = self.actor.world_pos;
            for _ in 0..3 {
                crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, DROP_TABLE_4);
            }
            self.actor.kill();
        }
    }

    /// `EnPeehat_Adult_CollisionCheck`.
    fn adult_collision_check(&mut self, play: &mut PlayState) {
        if self.collider_cylinder.base.ac_flags & AC_BOUNCED != 0 || self.collider_quad.base.ac_flags & AC_BOUNCED != 0 {
            self.collider_quad.base.ac_flags &= !AC_BOUNCED;
            self.collider_cylinder.base.ac_flags &= !AC_BOUNCED;
            self.collider_jnt_sph.base.ac_flags &= !AC_HIT;
        } else if self.collider_jnt_sph.base.ac_flags & AC_HIT != 0 {
            self.collider_jnt_sph.base.ac_flags &= !AC_HIT;
            self.actor.set_drop_flag_jnt_sph(&self.collider_jnt_sph, true);
            let reaction = self.actor.col_chk_info.damage_reaction;
            if reaction == PEAHAT_DMG_REACT_NUT || reaction == PEAHAT_DMG_REACT_LIGHT_ICE_ARROW {
                return;
            }
            if reaction == PEAHAT_DMG_REACT_HOOKSHOT {
                self.actor.col_chk_info.health = 0;
            } else if reaction == PEAHAT_DMG_REACT_BOOMERANG {
                if self.state != PEAHAT_STATE_STUNNED {
                    self.set_state_boomerang_stunned(play);
                }
                return;
            } else {
                self.actor.apply_damage();
                self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 8);
                audio_play_actor_sfx2(play, NA_SE_EN_PIHAT_DAMAGE);
            }
            if reaction == PEAHAT_DMG_REACT_FIRE {
                for _ in 0..5 {
                    let w = self.actor.world_pos;
                    let x = play.rand.centered_float(20.0) + w.x;
                    let y = play.rand.zero_one() * 25.0 + w.y;
                    let z = play.rand.centered_float(20.0) + w.z;
                    if let Some(me) = play.cur_actor {
                        let r = self.actor.shape_rot;
                        let actor = oot_game::effect::SsActor { handle: me, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] };
                        play.with_ss(|s| s.en_fire_spawn_vec3f(Some(actor), Vec3::new(x, y, z), 70, 0, 0, -1));
                    }
                }
                self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 200, COLORFILTER_BUFFLAG_OPA, 100);
            }
            if self.actor.col_chk_info.health == 0 {
                self.adult_set_state_die();
            }
        }
    }

    /// `EnPeehat_OverrideLimbDraw` and `EnPeehat_PostLimbDraw`'s matrices: the blades turned by
    /// `bladeRot`, the body (and the top on the ground or dying) drawn jiggling.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], blade_rot: i16, jiggle_rot: f32, scale_shift: f32, state: i32) -> Pose {
        let jiggle = Mat4::from_rotation_x(jiggle_rot * 0.115)
            * Mat4::from_rotation_y(jiggle_rot * 0.13)
            * Mat4::from_rotation_z(jiggle_rot * 0.1)
            * Mat4::from_scale(Vec3::new(1.0 - scale_shift, scale_shift + 1.0, 1.0 - scale_shift))
            * Mat4::from_rotation_z(-(jiggle_rot * 0.1))
            * Mat4::from_rotation_y(-(jiggle_rot * 0.13))
            * Mat4::from_rotation_x(-(jiggle_rot * 0.115));
        let mut body_stack = Mat4::IDENTITY;
        let mut blades_stack = Mat4::IDENTITY;
        let limbs = draw_opa_pose(
            skeleton,
            joints,
            |limb, _pos, rot| {
                if limb == LIMB_BLADES {
                    rot[0] = blade_rot.wrapping_neg();
                }
                if limb == LIMB_BODY || (limb == LIMB_TOP && (state == PEAHAT_STATE_DYING || state == PEAHAT_STATE_3 || state == PEAHAT_STATE_4)) {
                    LimbDraw::Drawn(jiggle)
                } else {
                    LimbDraw::Default(Mat4::IDENTITY)
                }
            },
            |limb, m| match limb {
                LIMB_BODY => body_stack = m,
                LIMB_BLADES => blades_stack = m,
                _ => {}
            },
        );
        Pose { limbs, body_stack, blades_stack }
    }

    /// `EnPeehat_PostLimbDraw`'s second body (the weak point under it): 1,000 back for the
    /// sphere, 500 on, turned 3.2 (shaking while it's red) and flattened.
    fn weak_point_matrix(body_stack: Mat4, color_filter: (u16, u8)) -> (Mat4, Mat4) {
        let sphere = body_stack * Mat4::from_translation(Vec3::new(-1000.0, 0.0, 0.0));
        let mut damage_y_rot = 0.0;
        let (params, timer) = color_filter;
        if timer != 0 && params & 0x4000 != 0 {
            damage_y_rot = sin_s((timer as i32 * 0x4E20) as i16) * 0.35;
        }
        let draw = sphere * Mat4::from_translation(Vec3::new(500.0, 0.0, 0.0)) * Mat4::from_rotation_y(3.2 + damage_y_rot) * Mat4::from_scale(Vec3::new(0.3, 0.2, 0.2));
        (sphere, draw)
    }
}

/// `Math_Vec3f_DistXZ`.
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

impl ActorImpl for EnPeehat {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnPeehat_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.actor.params <= 0 {
            self.adult_collision_check(play);
        }
        if self.actor.col_chk_info.damage_reaction != PEAHAT_DMG_REACT_LIGHT_ICE_ARROW {
            if self.actor.speed_xz != 0.0 || self.actor.velocity.y != 0.0 {
                self.actor.move_forward();
                self.actor.update_bg_check_info(&play.col, 25.0, 30.0, 30.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
            }
            match self.action {
                Action::GroundStateGround => self.ground_state_ground(play),
                Action::FlyingStateGrounded => self.flying_state_grounded(play),
                Action::FlyingStateFly => self.flying_state_fly(play),
                Action::GroundStateRise => self.ground_state_rise(play),
                Action::FlyingStateRise => self.flying_state_rise(play),
                Action::GroundStateSeekPlayer => self.ground_state_seek_player(play),
                Action::LarvaStateSeekPlayer => self.larva_state_seek_player(play),
                Action::GroundStateLanding => self.ground_state_landing(play),
                Action::FlyingStateLanding => self.flying_state_landing(play),
                Action::GroundStateHover => self.ground_state_hover(play),
                Action::GroundStateReturnHome => self.ground_state_return_home(play),
                Action::StateAttackRecoil => self.state_attack_recoil(play),
                Action::StateBoomerangStunned => self.state_boomerang_stunned(play),
                Action::AdultStateDie => self.adult_state_die(play),
                Action::StateExplode => self.state_explode(play),
            }
            if play.gameplay_frames & 0x7F == 0 {
                self.jiggle_rot_inc = (play.rand.zero_one() * 0.25) + 0.5;
            }
            self.jiggle_rot += self.jiggle_rot_inc;
        }
        if self.actor.params < 0 {
            // The lock-on's point on its weak point.
            let c = self.collider_jnt_sph.elements[0].dim.world_sphere.center;
            self.actor.focus_pos = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
            if self.state == PEAHAT_STATE_SEEK_PLAYER {
                smooth_step_to_s(&mut self.actor.shape_rot.x, 6000, 1, 300, 0);
            } else {
                smooth_step_to_s(&mut self.actor.shape_rot.x, 0, 1, 300, 0);
            }
        } else {
            self.actor.focus_pos = self.actor.world_pos;
        }
        self.collider_cylinder.update(&self.actor);
        if self.actor.col_chk_info.health > 0 {
            if self.actor.params <= 0 {
                play.collision_check_set_oc(&self.actor, COL_CYLINDER, &mut self.collider_cylinder);
                play.collision_check_set_oc(&self.actor, COL_JNT_SPH, &mut self.collider_jnt_sph);
                if (self.actor.color_filter_timer == 0 || self.actor.color_filter_params & 0x4000 == 0) && self.state != PEAHAT_STATE_EXPLODE {
                    play.collision_check_set_ac(&self.actor, COL_JNT_SPH, &mut self.collider_jnt_sph);
                }
            }
            if self.actor.params != PEAHAT_TYPE_FLYING && self.collider_quad.base.at_flags & AT_HIT != 0 {
                self.collider_quad.base.at_flags &= !AT_HIT;
                if play.player.is_some() && self.collider_quad.base.at == play.player {
                    self.set_state_attack_recoil();
                }
            }
        }
        if matches!(self.state, PEAHAT_STATE_15 | PEAHAT_STATE_SEEK_PLAYER | PEAHAT_STATE_FLY | PEAHAT_STATE_RETURN_HOME | PEAHAT_STATE_EXPLODE) {
            if self.actor.params != PEAHAT_TYPE_FLYING {
                play.collision_check_set_at(&self.actor, COL_QUAD, &mut self.collider_quad);
                play.collision_check_set_ac(&self.actor, COL_QUAD, &mut self.collider_quad);
            }
            if self.actor.params < 0 && self.actor.flags & ACTOR_FLAG_INSIDE_CULLING_VOLUME != 0 {
                for i in (0..2).rev() {
                    // BgCheck_EntityLineTest1(world.pos, bladeTip[i], wall, floor, no ceiling, one face).
                    if let Some((pos_result, _)) = play.col.entity_line_test(self.actor.world_pos, self.blade_tip[i], true, true, false, true) {
                        func_80033480(play, pos_result, 0.0, 1, 300, 150, 1);
                        self.spawn_dust(play, pos_result, 0.0, 3, 1.05, 1.5);
                    }
                }
            } else if self.actor.params != PEAHAT_TYPE_FLYING {
                play.collision_check_set_ac(&self.actor, COL_CYLINDER, &mut self.collider_cylinder);
            }
        } else {
            play.collision_check_set_ac(&self.actor, COL_CYLINDER, &mut self.collider_cylinder);
        }
        smooth_step_to_f(&mut self.scale_shift, 0.0, 1.0, 0.001, 0.0);
    }

    /// `EnPeehat_Draw`'s matrices at `Play_Draw`'s time: the weak point's sphere
    /// (`Collider_UpdateSpheres(0)`, adults), the blades' tips, and the blades' quad while it
    /// moves.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let Some(skeleton) = self.skeleton.clone() else { return };
        let rs = RenderState::of(&self.actor);
        let model = actor_draw_matrix(&rs);
        let pose = Self::pose(&skeleton, &self.skel.joint_table, self.blade_rot, self.jiggle_rot, self.scale_shift, self.state);
        let blades = model * pose.blades_stack;
        self.blade_tip = [blades.transform_point3(PEAHAT_BLADE_TIP[0]), blades.transform_point3(PEAHAT_BLADE_TIP[1])];
        if self.actor.params <= 0 {
            let (sphere, _) = Self::weak_point_matrix(pose.body_stack, (self.actor.color_filter_params, self.actor.color_filter_timer));
            self.collider_jnt_sph.update_spheres(0, &(model * sphere));
        }
        if self.actor.speed_xz != 0.0 || self.actor.velocity.y != 0.0 {
            let q = D_80AD285C.map(|v| model.transform_point3(v));
            self.collider_quad.set_vertices(q[1], q[0], q[3], q[2]);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.values = vec![self.jiggle_rot, self.scale_shift];
        rs.angles = vec![self.blade_rot];
        rs.switches = vec![self.state as u32];
        rs
    }

    /// `EnPeehat_Draw`: the skeleton after `Gfx_SetupDL_25Opa`, its body jiggling, its blades
    /// spinning; an adult's weak point a second, flattened body.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton), [jiggle_rot, scale_shift], [blade_rot], [state]) = (&rs.joints, &self.skeleton, rs.values.as_slice(), rs.angles.as_slice(), rs.switches.as_slice()) else {
            return;
        };
        let model = actor_draw_matrix(rs);
        let pose = Self::pose(skeleton, &joints.rot, *blade_rot, *jiggle_rot, *scale_shift, *state as i32);
        let body = model * pose.limbs[LIMB_BODY - 1];
        let top = model * pose.limbs[LIMB_TOP - 1];
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: model, bones: pose.limbs, params: DrawParams::default() });
        out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(BAKE_BODY)), body));
        out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(BAKE_TOP)), top));
        if self.actor.params <= 0 {
            let (_, weak_point) = Self::weak_point_matrix(pose.body_stack, rs.color_filter);
            out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(BAKE_BODY)), model * weak_point));
        }
    }

    /// `EnPeehat_Destroy`: a larva counted off its parent.
    fn destroy(&mut self, play: &mut PlayState) {
        if self.actor.params > 0
            && let Some(parent) = self.actor.parent.and_then(|h| play.actors.downcast_mut::<EnPeehat>(h))
            && !parent.actor.killed
        {
            parent.unk_2fa -= 1;
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_CYLINDER => Some(ColliderMut::Cylinder(&mut self.collider_cylinder)),
            COL_JNT_SPH => Some(ColliderMut::JntSph(&mut self.collider_jnt_sph)),
            COL_QUAD => Some(ColliderMut::Quad(&mut self.collider_quad)),
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
