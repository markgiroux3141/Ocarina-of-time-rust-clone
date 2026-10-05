//! `En_Dekubaba` (`ovl_En_Dekubaba/z_en_dekubaba.c`): the Deku Baba, a carnivorous plant on a
//! three-part stem that lies hidden in its leaves, rises and lunges at Link.
//!
//! Params: `EN_DEKUBABA_TYPE_NORMAL` (0) or `EN_DEKUBABA_TYPE_BIG` (1, two and a half times the
//! size, with 4 health).
//!
//! - **Waiting** (`EnDekubaba_WaitPlayerNear`): hidden, its collider hard (`AC_HARD`, nothing
//!   hurts it), until Link comes within 200 across and 30 up or down.
//! - **Rising** (`EnDekubaba_ExitGround`), then **lunging**: it draws back
//!   (`EnDekubaba_PrepareAttack`) and bites along the ground (`EnDekubaba_Attack`, its head an AT
//!   sphere: half a heart), or chomps in the air (`EnDekubaba_ChompAir`) while Link is between 80
//!   and 240 away. Past 240 it sinks back (`EnDekubaba_EnterGround`).
//! - **A missed bite** (`EnDekubaba_RecoverFromAttackMiss`) leaves it stuck to the ground,
//!   weakened: a hit then knocks it over to lie stretched out (`EnDekubaba_Vulnerable`), when a
//!   sword's hit on its stem kills it and leaves a Deku Stick (`EnDekubaba_DieDropStick`,
//!   `EnDekubaba_DekuStick`). A hit otherwise just knocks it back up straight
//!   (`EnDekubaba_Attacked`); out of health it shrinks into the ground and drops Deku Nuts
//!   (`EnDekubaba_Die`), or what a magic or arrow hit's drop table gives.
//! - **Its damage table** (`sDamageTableNormal`, `sDamageTableBig`): the Kokiri Sword's slash
//!   does 1, its jump attack 2 (4 for child Link: the init rewrites the table's entry); Deku
//!   Nuts stun it.
//!
//! The whole overlay is ported, with its effects (since GAME-05 milestone 3a: the dirt bursts
//! `EffectSsHahen_SpawnBurst`, the dust `func_8002829C` and `func_800286CC`, a fire hit's flames
//! `EffectSsEnFire_SpawnVec3f`, which follow the struct's unused `unk_14C`: zeroes, so they burn
//! at the world's origin, the C's own bug). Not ported: the generic circle shadow
//! (`ActorShadow_DrawCircle`, not ported for any actor; its own floor shadow,
//! `EnDekubaba_DrawShadow`, is drawn).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::PolyId;
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_s, cos_s, scaled_step_to_s, sin_s, smooth_step_to_s, step_to_f, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_MISC, ActorImpl, ActorProfile, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::audio::sfx::*;
use oot_game::collision_check::*;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};
use oot_game::sys_matrix::MtxF;

pub const ACTOR_EN_DEKUBABA: i16 = 0x0055;
const OBJECT: &str = "object_dekubaba";

/// `En_Dekubaba_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_DEKUBABA, name: "En_Dekubaba", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE, object: OBJECT };

/// `EnDekubabaType`.
pub const EN_DEKUBABA_TYPE_NORMAL: i16 = 0;
pub const EN_DEKUBABA_TYPE_BIG: i16 = 1;

/// `EnDekubabaDamageReaction`: the damage tables' reactions.
pub const EN_DEKUBABA_DMG_REACT_NONE: u8 = 0;
pub const EN_DEKUBABA_DMG_REACT_STUN: u8 = 1;
pub const EN_DEKUBABA_DMG_REACT_FIRE: u8 = 2;
pub const EN_DEKUBABA_DMG_REACT_BOOMERANG: u8 = 0xE;
pub const EN_DEKUBABA_DMG_REACT_SWORD: u8 = 0xF;

/// `EnDekubabaAttackedType`.
pub const EN_DEKUBABA_ATTACKED_TYPE_STRENGTHENED: i16 = 0;
pub const EN_DEKUBABA_ATTACKED_TYPE_WEAKENED: i16 = 1;
pub const EN_DEKUBABA_ATTACKED_TYPE_STUNNED: i16 = 2;

/// `DEKUBABA_HEAD_LIMB_ROOT` (`object_dekubaba.h`, from the XML's limbs).
const DEKUBABA_HEAD_LIMB_ROOT: u8 = 1;

/// `NAVI_ENEMY_DEKU_BABA`, `NAVI_ENEMY_BIG_DEKU_BABA` (`actor.h`).
const NAVI_ENEMY_DEKU_BABA: u8 = 0x07;
const NAVI_ENEMY_BIG_DEKU_BABA: u8 = 0x08;

/// `EnDekubaba_Attack`'s `sEffPrimColor` and `sEffEnvColor`: the bite's green dust.
const S_EFF_PRIM_COLOR: [u8; 4] = [105, 255, 105, 255];
const S_EFF_ENV_COLOR: [u8; 4] = [150, 250, 150, 0];

/// `ITEM00_NUTS`, `COLLECTIBLE_DROP_TABLE_3` (`z_en_item00.h`).
const ITEM00_NUTS: i16 = 0x0C;
const COLLECTIBLE_DROP_TABLE_3: i16 = 3;

const fn table(kokiri_jump: u8, hookshot: u8) -> DamageTable {
    DamageTable {
        table: [
            dmg_entry(0, EN_DEKUBABA_DMG_REACT_STUN),      // Deku nut
            dmg_entry(2, EN_DEKUBABA_DMG_REACT_NONE),      // Deku stick
            dmg_entry(1, EN_DEKUBABA_DMG_REACT_NONE),      // Slingshot
            dmg_entry(2, EN_DEKUBABA_DMG_REACT_NONE),      // Explosive
            dmg_entry(2, EN_DEKUBABA_DMG_REACT_BOOMERANG), // Boomerang
            dmg_entry(2, EN_DEKUBABA_DMG_REACT_NONE),      // Normal arrow
            dmg_entry(2, EN_DEKUBABA_DMG_REACT_NONE),      // Hammer swing
            hookshot,                                      // Hookshot
            dmg_entry(1, EN_DEKUBABA_DMG_REACT_SWORD),     // Kokiri sword
            dmg_entry(2, EN_DEKUBABA_DMG_REACT_SWORD),     // Master sword
            dmg_entry(4, EN_DEKUBABA_DMG_REACT_SWORD),     // Giant's Knife
            dmg_entry(4, EN_DEKUBABA_DMG_REACT_FIRE),      // Fire arrow
            dmg_entry(2, EN_DEKUBABA_DMG_REACT_NONE),      // Ice arrow
            dmg_entry(2, EN_DEKUBABA_DMG_REACT_NONE),      // Light arrow
            dmg_entry(2, EN_DEKUBABA_DMG_REACT_NONE),      // Unk arrow 1
            dmg_entry(2, EN_DEKUBABA_DMG_REACT_NONE),      // Unk arrow 2
            dmg_entry(2, EN_DEKUBABA_DMG_REACT_NONE),      // Unk arrow 3
            dmg_entry(4, EN_DEKUBABA_DMG_REACT_FIRE),      // Fire magic
            dmg_entry(0, EN_DEKUBABA_DMG_REACT_NONE),      // Ice magic
            dmg_entry(0, EN_DEKUBABA_DMG_REACT_NONE),      // Light magic
            dmg_entry(0, EN_DEKUBABA_DMG_REACT_NONE),      // Shield
            dmg_entry(0, EN_DEKUBABA_DMG_REACT_NONE),      // Mirror Ray
            dmg_entry(1, EN_DEKUBABA_DMG_REACT_SWORD),     // Kokiri spin
            dmg_entry(4, EN_DEKUBABA_DMG_REACT_SWORD),     // Giant spin
            dmg_entry(2, EN_DEKUBABA_DMG_REACT_SWORD),     // Master spin
            dmg_entry(2, EN_DEKUBABA_DMG_REACT_SWORD),     // Kokiri jump
            dmg_entry(8, EN_DEKUBABA_DMG_REACT_SWORD),     // Giant jump
            kokiri_jump,                                   // Master jump
            dmg_entry(0, EN_DEKUBABA_DMG_REACT_NONE),      // Unknown 1
            dmg_entry(0, EN_DEKUBABA_DMG_REACT_NONE),      // Unblockable
            dmg_entry(4, EN_DEKUBABA_DMG_REACT_NONE),      // Hammer jump
            dmg_entry(0, EN_DEKUBABA_DMG_REACT_NONE),      // Unknown 2
        ],
    }
}

/// `sDamageTableNormal`, as the overlay's data has it.
pub static S_DAMAGE_TABLE_NORMAL: DamageTable = table(dmg_entry(4, EN_DEKUBABA_DMG_REACT_SWORD), dmg_entry(2, EN_DEKUBABA_DMG_REACT_NONE));
/// `sDamageTableBig`: the hookshot stuns.
pub static S_DAMAGE_TABLE_BIG: DamageTable = table(dmg_entry(4, EN_DEKUBABA_DMG_REACT_SWORD), dmg_entry(0, EN_DEKUBABA_DMG_REACT_STUN));
/// The two tables as `EnDekubaba_Init` leaves them for child Link (`!LINK_IS_ADULT`): entry 27
/// (`DMG_JUMP_MASTER`'s) rewritten to `DMG_ENTRY(4, EN_DEKUBABA_DMG_REACT_NONE)`. The C writes the
/// overlay's static tables, which stay so until the overlay is reloaded; Link's age doesn't
/// change in between, so a child's init picks these.
pub static S_DAMAGE_TABLE_NORMAL_CHILD: DamageTable = table(dmg_entry(4, EN_DEKUBABA_DMG_REACT_NONE), dmg_entry(2, EN_DEKUBABA_DMG_REACT_NONE));
pub static S_DAMAGE_TABLE_BIG_CHILD: DamageTable = table(dmg_entry(4, EN_DEKUBABA_DMG_REACT_NONE), dmg_entry(0, EN_DEKUBABA_DMG_REACT_STUN));

fn elem(at: (u32, u8), ac_on: bool, at_on: bool, oc_on: bool, limb: u8, center: [i16; 3], radius: i16) -> ColliderJntSphElementInit {
    ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: at.0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: at.1 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: if at_on { ATELEM_ON | ATELEM_SFX_HARD } else { ATELEM_NONE },
            ac_elem_flags: if ac_on { ACELEM_ON } else { ACELEM_NONE },
            oc_elem_flags: if oc_on { OCELEM_ON } else { OCELEM_NONE },
        },
        limb,
        model_sphere: Sphere16 { center, radius },
        scale: 100,
    }
}

/// `sJntSphElementsInit`: the head (the bite, AT `0xFFCFFFFF` for 8), then the stem's six spheres
/// (limbs 51 to 56: the stem parts' matrices in `EnDekubaba_DrawStem`).
fn jnt_sph_elements() -> [ColliderJntSphElementInit; 7] {
    [
        elem((0xFFCF_FFFF, 0x08), true, true, true, DEKUBABA_HEAD_LIMB_ROOT, [0, 100, 1000], 15),
        elem((0, 0), false, false, true, 51, [0, 0, 1500], 8),
        elem((0, 0), false, false, false, 52, [0, 0, 500], 8),
        elem((0, 0), false, false, false, 53, [0, 0, 1500], 8),
        elem((0, 0), false, false, false, 54, [0, 0, 500], 8),
        elem((0, 0), false, false, false, 55, [0, 0, 1500], 8),
        elem((0, 0), false, false, false, 56, [0, 0, 500], 8),
    ]
}

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit = ColliderInit {
    col_type: COL_MATERIAL_HIT6,
    at_flags: AT_ON | AT_TYPE_ENEMY,
    ac_flags: AC_ON | AC_TYPE_PLAYER,
    oc_flags1: OC1_ON | OC1_TYPE_ALL,
    oc_flags2: OC2_TYPE_1,
    shape: COLSHAPE_JNTSPH,
};

/// `sColChkInfoInit`: 2 health, a 25 by 25 cylinder, immovable.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 2, cyl_radius: 25, cyl_height: 25, mass: MASS_IMMOVABLE };

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitPlayerNear,
    ExitGround,
    EnterGround,
    ChompAir,
    Attack,
    PrepareAttack,
    RecoverFromAttackMiss,
    Soothe,
    Attacked,
    Vulnerable,
    Wobble,
    DieDropStick,
    Die,
    DekuStick,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::WaitPlayerNear => "EnDekubaba_WaitPlayerNear",
            Action::ExitGround => "EnDekubaba_ExitGround",
            Action::EnterGround => "EnDekubaba_EnterGround",
            Action::ChompAir => "EnDekubaba_ChompAir",
            Action::Attack => "EnDekubaba_Attack",
            Action::PrepareAttack => "EnDekubaba_PrepareAttack",
            Action::RecoverFromAttackMiss => "EnDekubaba_RecoverFromAttackMiss",
            Action::Soothe => "EnDekubaba_Soothe",
            Action::Attacked => "EnDekubaba_Attacked",
            Action::Vulnerable => "EnDekubaba_Vulnerable",
            Action::Wobble => "EnDekubaba_Wobble",
            Action::DieDropStick => "EnDekubaba_DieDropStick",
            Action::Die => "EnDekubaba_Die",
            Action::DekuStick => "EnDekubaba_DekuStick",
        }
    }
}

/// `Math_Vec3f_DistXZ`.
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

/// `sinf(x * M_PI)`, the product in double precision as the C's `M_PI` makes it.
fn sin_pi(x: f32) -> f32 {
    ((x as f64 * std::f64::consts::PI) as f32).sin()
}

pub struct EnDekubaba {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    chomp: Option<Anim>,
    idle: Option<Anim>,
    pub action: Action,
    /// `actionState`: a timer, or the attacked type.
    pub action_state: i16,
    /// `wobbleTarget`.
    pub wobble_target: i16,
    /// `stemPartsRot`: each stem part's pitch.
    pub stem_parts_rot: [i16; 3],
    /// `scaleFac`: 1, or 2.5 for the big one.
    pub scale_fac: f32,
    /// `floorPoly`: the floor under it, the first one found (its shadow's).
    pub floor_poly: Option<PolyId>,
    pub collider: ColliderJntSph,
}

impl EnDekubaba {
    fn anim(&self, a: &Option<Anim>) -> Option<Anim> {
        a.clone()
    }

    /// `EnDekubaba_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ICHAIN_F32(lockOnArrowOffset, 1500).
        actor.target_arrow_offset = 1500.0;
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 22): the circle shadow isn't ported.
        actor.shape_y_offset = 0.0;
        let (skeleton, chomp, idle) = match play.assets.clone() {
            Some(a) => {
                let s = a.skeleton(OBJECT, "gDekubabaHeadSkel").map_err(|e| log::error!("En_Dekubaba: {e:#}")).ok();
                let c = a.animation(OBJECT, "gDekubabaChompAnim").map_err(|e| log::error!("En_Dekubaba: {e:#}")).ok();
                let i = a.animation(OBJECT, "gDekubabaIdleAnim").map_err(|e| log::error!("En_Dekubaba: {e:#}")).ok();
                (s, c, i)
            }
            None => (None, None, None),
        };
        // SkelAnime_Init(&gDekubabaHeadSkel, &gDekubabaChompAnim): DEKUBABA_HEAD_LIMB_MAX entries.
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(7);
        let skel = SkelAnimeStd::init_flex(limbs, chomp.clone());
        let elements = jnt_sph_elements();
        let mut collider = ColliderJntSph::new(&JNT_SPH_INIT, &elements);
        let adult = play.save.adult;
        let mut b = EnDekubaba { actor, skel, skeleton, chomp, idle, action: Action::WaitPlayerNear, action_state: 0, wobble_target: 0, stem_parts_rot: [0; 3], scale_fac: 1.0, floor_poly: None, collider: ColliderJntSph::default() };
        if b.actor.params == EN_DEKUBABA_TYPE_BIG {
            b.scale_fac = 2.50;
            for (e, init) in collider.elements.iter_mut().zip(elements.iter()) {
                let r = (init.model_sphere.radius as f32 * 2.5) as i16;
                e.dim.model_sphere.radius = r;
                e.dim.world_sphere.radius = r;
            }
            let t = if adult { &S_DAMAGE_TABLE_BIG } else { &S_DAMAGE_TABLE_BIG_CHILD };
            b.actor.col_chk_info.set_info(Some(t), &COL_CHK_INFO_INIT);
            b.actor.col_chk_info.health = 4;
            b.actor.navi_enemy_id = NAVI_ENEMY_BIG_DEKU_BABA;
            // ATTENTION_RANGE_2.
            b.actor.target_mode = 2;
        } else {
            b.scale_fac = 1.0;
            for e in collider.elements.iter_mut() {
                e.dim.world_sphere.radius = e.dim.model_sphere.radius;
            }
            let t = if adult { &S_DAMAGE_TABLE_NORMAL } else { &S_DAMAGE_TABLE_NORMAL_CHILD };
            b.actor.col_chk_info.set_info(Some(t), &COL_CHK_INFO_INIT);
            b.actor.navi_enemy_id = NAVI_ENEMY_DEKU_BABA;
            // ATTENTION_RANGE_1.
            b.actor.target_mode = 1;
        }
        b.collider = collider;
        b.setup_wait_player_near();
        b.action_state = 0;
        b.floor_poly = None;
        Box::new(b)
    }

    /// `EffectSsHahen_SpawnBurst(play, pos, scaleFac * 3, 0, scaleFac * 12, scaleFac * 5, count,
    /// HAHEN_OBJECT_DEFAULT, 10, NULL)`: the withered fragments its calls throw.
    fn hahen_burst(&self, play: &mut PlayState, pos: Vec3, count: i16) {
        let f = self.scale_fac;
        play.with_ss(|s| s.hahen_spawn_burst(pos, f * 3.0, 0, (f * 12.0) as i16, (f * 5.0) as i16, count, -1, 10, None));
    }

    /// `&this->actor` for an effect's spawn.
    fn ss_actor(&self, play: &PlayState) -> Option<oot_game::effect::SsActor> {
        let r = self.actor.shape_rot;
        play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] })
    }

    /// `EnDekubaba_DisableStemColliderAC`.
    fn disable_stem_collider_ac(&mut self) {
        for e in self.collider.elements.iter_mut().skip(1) {
            e.info.ac_elem_flags &= !ACELEM_ON;
        }
    }

    /// `EnDekubaba_SetupWaitPlayerNear`: down in its leaves, upright and half size, hard; the
    /// stem's spheres gathered at its foot.
    fn setup_wait_player_near(&mut self) {
        self.stem_parts_rot = [-0x4000; 3];
        self.actor.shape_rot.x = -0x4000;
        self.actor.world_pos.x = self.actor.home_pos.x;
        self.actor.world_pos.z = self.actor.home_pos.z;
        self.actor.world_pos.y = self.actor.home_pos.y + 14.0 * self.scale_fac;
        self.actor.scale = Vec3::splat(self.scale_fac * 0.01 * 0.5);
        self.collider.base.col_type = COL_MATERIAL_HARD;
        self.collider.base.ac_flags |= AC_HARD;
        self.action_state = 45;
        let p = self.actor.world_pos;
        for e in self.collider.elements.iter_mut().skip(1) {
            e.dim.world_sphere.center = [p.x as i16, (p.y as i16).wrapping_sub(7), p.z as i16];
        }
        self.action = Action::WaitPlayerNear;
    }

    /// `EnDekubaba_SetupExitGround`.
    fn setup_exit_ground(&mut self, play: &mut PlayState) {
        if let Some(a) = self.anim(&self.chomp) {
            let last = a.last_frame();
            self.skel.change(a, last * (1.0 / 15.0), 0.0, last, ANIMMODE_ONCE, 0.0);
        }
        self.action_state = 15;
        for e in self.collider.elements.iter_mut().skip(2) {
            e.info.oc_elem_flags |= OCELEM_ON;
        }
        self.collider.base.col_type = COL_MATERIAL_HIT6;
        self.collider.base.ac_flags &= !AC_HARD;
        audio_play_actor_sfx2(play, NA_SE_EN_DUMMY482);
        self.action = Action::ExitGround;
    }

    /// `EnDekubaba_SetupEnterGround`.
    fn setup_enter_ground(&mut self) {
        if let Some(a) = self.anim(&self.chomp) {
            let last = a.last_frame();
            self.skel.change(a, -1.5, last, 0.0, ANIMMODE_ONCE, -3.0);
        }
        self.action_state = 15;
        for e in self.collider.elements.iter_mut().skip(2) {
            e.info.oc_elem_flags &= !OCELEM_ON;
        }
        self.action = Action::EnterGround;
    }

    /// `EnDekubaba_SetupChompAir`.
    fn setup_chomp_air(&mut self) {
        if let Some(a) = self.anim(&self.chomp) {
            self.action_state = (a.last_frame() * 2.0) as i16;
            // Animation_MorphToLoop.
            self.skel.change(a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -3.0);
        }
        self.action = Action::ChompAir;
    }

    /// `EnDekubaba_SetupPrepareAttack`.
    fn setup_prepare_attack(&mut self) {
        self.action_state = 8;
        self.action = Action::PrepareAttack;
        self.skel.play_speed = 0.0;
    }

    /// `EnDekubaba_SetupAttack`.
    fn setup_attack(&mut self) {
        if let Some(a) = self.anim(&self.idle) {
            // Animation_PlayOnce.
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, 0.0);
        }
        self.action_state = 0;
        self.action = Action::Attack;
    }

    /// `EnDekubaba_SetupRecoverFromAttackMiss`.
    fn setup_recover_from_attack_miss(&mut self) {
        if let Some(a) = self.anim(&self.idle) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 15.0, last, ANIMMODE_ONCE, -3.0);
        }
        self.action_state = 0;
        self.action = Action::RecoverFromAttackMiss;
    }

    /// `EnDekubaba_SetupSoothe`.
    fn setup_soothe(&mut self) {
        self.action_state = 9;
        self.collider.base.ac_flags |= AC_ON;
        self.action = Action::Soothe;
        self.skel.play_speed = -1.0;
    }

    /// `EnDekubaba_SetupAttacked`: knocked upright, full size, flashing (blue when stunned, red
    /// otherwise), not to be hit until upright.
    fn setup_attacked(&mut self, ty: i16) {
        if let Some(a) = self.anim(&self.idle) {
            // Animation_MorphToPlayOnce.
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, -5.0);
        }
        self.action_state = ty;
        self.collider.base.ac_flags &= !AC_ON;
        self.actor.scale = Vec3::splat(self.scale_fac * 0.01);
        if ty == EN_DEKUBABA_ATTACKED_TYPE_STUNNED {
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 155, COLORFILTER_BUFFLAG_OPA, 62);
        } else {
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 42);
        }
        self.action = Action::Attacked;
    }

    /// `EnDekubaba_SetupDieDropStick`: the head flies off backwards.
    fn setup_die_drop_stick(&mut self) {
        self.action_state = 0;
        self.skel.play_speed = 0.0;
        self.actor.gravity = -0.8;
        self.actor.velocity.y = 4.0;
        self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(i16::MIN);
        self.actor.speed_xz = self.scale_fac * 3.0;
        self.collider.base.ac_flags &= !AC_ON;
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED;
        self.action = Action::DieDropStick;
    }

    /// `EnDekubaba_SetupDie`.
    fn setup_die(&mut self) {
        if let Some(a) = self.anim(&self.chomp) {
            let last = a.last_frame();
            self.skel.change(a, -1.5, last, 0.0, ANIMMODE_ONCE, -3.0);
        }
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::Die;
    }

    /// `EnDekubaba_SetupVulnerable`: stretched out, the stem hittable: the weakened one chomping
    /// for 40 frames, the stunned one still for 60.
    fn setup_vulnerable(&mut self) {
        for e in self.collider.elements.iter_mut().skip(1) {
            e.info.ac_elem_flags |= ACELEM_ON;
        }
        if let Some(a) = self.anim(&self.chomp) {
            let last = a.last_frame();
            if self.action_state == EN_DEKUBABA_ATTACKED_TYPE_WEAKENED {
                self.skel.change(a, 4.0, 0.0, last, ANIMMODE_LOOP, -3.0);
                self.action_state = 40;
            } else {
                self.skel.change(a, 0.0, 0.0, last, ANIMMODE_LOOP, -3.0);
                self.action_state = 60;
            }
        } else {
            self.action_state = if self.action_state == EN_DEKUBABA_ATTACKED_TYPE_WEAKENED { 40 } else { 60 };
        }
        self.actor.world_pos.x = self.actor.home_pos.x;
        self.actor.world_pos.y = self.actor.home_pos.y + 60.0 * self.scale_fac;
        self.actor.world_pos.z = self.actor.home_pos.z;
        self.action = Action::Vulnerable;
    }

    /// `EnDekubaba_SetupWobble`.
    fn setup_wobble(&mut self) {
        self.wobble_target = -0x6000;
        self.stem_parts_rot[2] = -0x5000;
        self.stem_parts_rot[1] = -0x4800;
        self.disable_stem_collider_ac();
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 35);
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::Wobble;
    }

    /// `EnDekubaba_SetupDekuStick`: what's left is a Deku Stick lying on the ground, a misc actor.
    fn setup_deku_stick(&mut self, play: &mut PlayState) {
        self.actor.scale = Vec3::splat(0.03);
        self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_sub(0x4000);
        self.actor.shape_y_offset = 1000.0;
        self.actor.gravity = 0.0;
        self.actor.velocity.y = 0.0;
        // (shape.shadowScale = 3.)
        // Actor_ChangeCategory(play, &play->actorCtx, &this->actor, ACTORCAT_MISC).
        self.actor.category = ACTORCAT_MISC;
        if let Some(me) = play.cur_actor {
            play.actors.change_category(me, ACTORCAT_MISC);
        }
        self.actor.flags &= !ACTOR_FLAG_DRAW_CULLING_DISABLED;
        self.action_state = 200;
        self.action = Action::DekuStick;
    }

    /// `EnDekubaba_WaitPlayerNear`.
    fn wait_player_near(&mut self, play: &mut PlayState) {
        if self.action_state != 0 {
            self.action_state -= 1;
        }
        self.actor.world_pos.x = self.actor.home_pos.x;
        self.actor.world_pos.z = self.actor.home_pos.z;
        self.actor.world_pos.y = self.actor.home_pos.y + 14.0 * self.scale_fac;
        if self.action_state == 0 && self.actor.xz_dist_to_player < 200.0 * self.scale_fac && self.actor.y_dist_to_player.abs() < 30.0 * self.scale_fac {
            self.setup_exit_ground(play);
        }
    }

    /// The horizontal reach of a stem rising or sinking, from its parts' pitches (`dxz` in
    /// `EnDekubaba_ExitGround` and `EnDekubaba_EnterGround`'s last two branches).
    fn stem_reach(&self, dy: f32) -> f32 {
        let s = &self.stem_parts_rot;
        (20.0 * (cos_s(s[0]) + cos_s(s[1]))) + (((dy - (20.0 * (-sin_s(s[0]) - sin_s(s[1])))) * cos_s(s[2])) / -sin_s(s[2]))
    }

    /// `EnDekubaba_ExitGround`: 15 frames up out of the leaves, growing to full size, the stem
    /// unfolding; then the lunge if Link is within 240, else back down.
    fn exit_ground(&mut self, play: &mut PlayState) {
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default();
        if self.action_state != 0 {
            self.action_state -= 1;
        }
        self.skel.update();
        self.actor.scale = Vec3::splat(self.scale_fac * 0.01 * (0.5 + (((15 - self.action_state) as f32 * 0.5) / 15.0)));
        scaled_step_to_s(&mut self.actor.shape_rot.x, 0x1800, 0x800);
        let dy = (sin_pi((((15 - self.action_state) as f32) * (1.0 / 15.0)).min(0.7)) * 32.0) + 14.0;
        let dxz;
        if self.actor.shape_rot.x < -0x38E3 {
            dxz = 0.0;
        } else if self.actor.shape_rot.x < -0x238E {
            scaled_step_to_s(&mut self.stem_parts_rot[0], -0x5555, 0x38E);
            dxz = 20.0 * cos_s(self.stem_parts_rot[0]);
        } else if self.actor.shape_rot.x < -0xE38 {
            scaled_step_to_s(&mut self.stem_parts_rot[0], -0xAAA, 0x38E);
            scaled_step_to_s(&mut self.stem_parts_rot[1], -0x5555, 0x38E);
            scaled_step_to_s(&mut self.stem_parts_rot[2], -0x5555, 0x222);
            dxz = self.stem_reach(dy);
        } else {
            scaled_step_to_s(&mut self.stem_parts_rot[0], -0xAAA, 0x38E);
            scaled_step_to_s(&mut self.stem_parts_rot[1], -0x31C7, 0x222);
            scaled_step_to_s(&mut self.stem_parts_rot[2], -0x5555, 0x222);
            dxz = self.stem_reach(dy);
        }
        if self.action_state < 10 {
            approach_s(&mut self.actor.shape_rot.y, vec3f_yaw(self.actor.home_pos, player_pos), 2, 0xE38);
        }
        self.actor.world_pos.y = self.actor.home_pos.y + dy * self.scale_fac;
        let dx = dxz * self.scale_fac * sin_s(self.actor.shape_rot.y);
        let dz = dxz * self.scale_fac * cos_s(self.actor.shape_rot.y);
        self.actor.world_pos.x = self.actor.home_pos.x + dx;
        self.actor.world_pos.z = self.actor.home_pos.z + dz;
        self.hahen_burst(play, self.actor.home_pos, 1);
        if self.action_state == 0 {
            if dist_xz(self.actor.home_pos, player_pos) < 240.0 * self.scale_fac {
                self.setup_prepare_attack();
            } else {
                self.setup_enter_ground();
            }
        }
    }

    /// `EnDekubaba_EnterGround`: 15 frames back down into the leaves, shrinking, the stem
    /// folding; then waiting again.
    fn enter_ground(&mut self, play: &mut PlayState) {
        if self.action_state != 0 {
            self.action_state -= 1;
        }
        self.skel.update();
        self.actor.scale = Vec3::splat(self.scale_fac * 0.01 * (0.5 + (self.action_state as f32 * (1.0 / 30.0))));
        scaled_step_to_s(&mut self.actor.shape_rot.x, -0x4000, 0x300);
        let dy = (sin_pi((self.action_state as f32 * 0.033).min(0.7)) * 32.0) + 14.0;
        let dxz;
        if self.actor.shape_rot.x < -0x38E3 {
            dxz = 0.0;
        } else if self.actor.shape_rot.x < -0x238E {
            scaled_step_to_s(&mut self.stem_parts_rot[0], -0x4000, 0x555);
            dxz = cos_s(self.stem_parts_rot[0]) * 20.0;
        } else if self.actor.shape_rot.x < -0xE38 {
            scaled_step_to_s(&mut self.stem_parts_rot[0], -0x5555, 0x555);
            scaled_step_to_s(&mut self.stem_parts_rot[1], -0x4000, 0x555);
            scaled_step_to_s(&mut self.stem_parts_rot[2], -0x4000, 0x333);
            dxz = self.stem_reach(dy);
        } else {
            scaled_step_to_s(&mut self.stem_parts_rot[0], -0x5555, 0x555);
            scaled_step_to_s(&mut self.stem_parts_rot[1], -0x5555, 0x333);
            scaled_step_to_s(&mut self.stem_parts_rot[2], -0x4000, 0x333);
            dxz = self.stem_reach(dy);
        }
        self.actor.world_pos.y = self.actor.home_pos.y + dy * self.scale_fac;
        let dx = dxz * self.scale_fac * sin_s(self.actor.shape_rot.y);
        let dz = dxz * self.scale_fac * cos_s(self.actor.shape_rot.y);
        self.actor.world_pos.x = self.actor.home_pos.x + dx;
        self.actor.world_pos.z = self.actor.home_pos.z + dz;
        self.hahen_burst(play, self.actor.home_pos, 1);
        if self.action_state == 0 {
            self.setup_wait_player_near();
        }
    }

    /// `EnDekubaba_ComputeHeadPos`: the head at the end of the stem.
    fn compute_head_pos(&mut self) {
        let s = &self.stem_parts_rot;
        let dxz = (cos_s(s[0]) + cos_s(s[1]) + cos_s(s[2])) * 20.0;
        self.actor.world_pos.x = self.actor.home_pos.x + (dxz * self.scale_fac * sin_s(self.actor.shape_rot.y));
        self.actor.world_pos.y = self.actor.home_pos.y - ((sin_s(s[0]) + sin_s(s[1]) + sin_s(s[2])) * 20.0 * self.scale_fac);
        self.actor.world_pos.z = self.actor.home_pos.z + (dxz * self.scale_fac * cos_s(self.actor.shape_rot.y));
    }

    /// `EnDekubaba_ChompAir`: chomping and swaying for two of the chomp's lengths, turning to Link;
    /// then the lunge, or sooner with Link within 80; back down if he's past 240.
    fn chomp_air(&mut self, play: &mut PlayState) {
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default();
        self.skel.update();
        if self.skel.on_frame(0.0) || self.skel.on_frame(12.0) {
            audio_play_actor_sfx2(play, if self.actor.params == EN_DEKUBABA_TYPE_BIG { NA_SE_EN_DEKU_MOUTH } else { NA_SE_EN_DEKU_JR_MOUTH });
        }
        if self.action_state != 0 {
            self.action_state -= 1;
        }
        approach_s(&mut self.actor.shape_rot.y, vec3f_yaw(self.actor.home_pos, player_pos), 2, (self.action_state % 5).wrapping_mul(0x222));
        let s = &mut self.stem_parts_rot;
        let r = &mut self.actor.shape_rot.x;
        if self.action_state < 10 {
            s[0] = s[0].wrapping_add(0x16C);
            s[1] = s[1].wrapping_add(0x16C);
            s[2] = s[2].wrapping_add(0xB6);
            *r = r.wrapping_add(0x222);
        } else if self.action_state < 20 {
            s[0] = s[0].wrapping_sub(0x16C);
            s[1] = s[1].wrapping_add(0x111);
            *r = r.wrapping_add(0x16C);
        } else if self.action_state < 30 {
            s[1] = s[1].wrapping_sub(0x111);
            *r = r.wrapping_sub(0xB6);
        } else {
            s[1] = s[1].wrapping_sub(0xB6);
            s[2] = s[2].wrapping_add(0xB6);
            *r = r.wrapping_sub(0x16C);
        }
        self.compute_head_pos();
        if dist_xz(self.actor.home_pos, player_pos) > 240.0 * self.scale_fac {
            self.setup_enter_ground();
        } else if self.action_state == 0 || self.actor.xz_dist_to_player < 80.0 * self.scale_fac {
            self.setup_prepare_attack();
        }
    }

    /// `EnDekubaba_Attack`: the bite: the stem stretches out flat along the ground (the head's AT
    /// on), then it chomps for ten frames, turning after Link; then the miss's recovery.
    fn attack(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.action_state == 0 {
            if self.skel.on_frame(1.0) {
                audio_play_actor_sfx2(play, if self.actor.params == EN_DEKUBABA_TYPE_BIG { NA_SE_EN_DEKU_ATTACK } else { NA_SE_EN_DEKU_JR_ATTACK });
            }
            scaled_step_to_s(&mut self.actor.shape_rot.x, 0, 0x222);
            let step_base = (self.skel.cur_frame * 10.0) as i16;
            let mut lying_down = true;
            lying_down &= scaled_step_to_s(&mut self.stem_parts_rot[0], -0xE38, step_base.wrapping_add(0x38E));
            lying_down &= scaled_step_to_s(&mut self.stem_parts_rot[1], -0xE38, step_base.wrapping_add(0x71C));
            lying_down &= scaled_step_to_s(&mut self.stem_parts_rot[2], -0xE38, step_base.wrapping_add(0xE38));
            if lying_down {
                if let Some(a) = self.anim(&self.chomp) {
                    // Animation_PlayLoopSetSpeed(4).
                    let last = a.last_frame();
                    self.skel.change(a, 4.0, 0.0, last, ANIMMODE_LOOP, 0.0);
                }
                // func_8002829C: the bite's dust, ahead at 5 (sEffPrimColor, sEffEnvColor).
                let vel = Vec3::new(sin_s(self.actor.shape_rot.y) * 5.0, 0.0, cos_s(self.actor.shape_rot.y) * 5.0);
                let (pos, scale) = (self.actor.world_pos, (self.scale_fac * 100.0) as i16);
                play.with_ss(|s| s.func_8002829c(pos, vel, Vec3::ZERO, S_EFF_PRIM_COLOR, S_EFF_ENV_COLOR, 1, scale));
                self.action_state = 1;
                self.collider.base.ac_flags |= AC_ON;
            }
        } else if self.action_state > 10 {
            self.setup_recover_from_attack_miss();
        } else {
            self.action_state += 1;
            // Actor_IsFacingPlayer(&this->actor, 0x16C).
            let facing = (self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y) as i32).abs() < 0x16C;
            if self.action_state >= 4 && !facing {
                approach_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 15, 0x71C);
            }
            if self.skel.on_frame(0.0) || self.skel.on_frame(12.0) {
                audio_play_actor_sfx2(play, if self.actor.params == EN_DEKUBABA_TYPE_BIG { NA_SE_EN_DEKU_MOUTH } else { NA_SE_EN_DEKU_JR_MOUTH });
            }
        }
        self.compute_head_pos();
    }

    /// `EnDekubaba_PrepareAttack`: 8 frames drawing back, the head turned to Link.
    fn prepare_attack(&mut self, play: &mut PlayState) {
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default();
        if self.action_state != 0 {
            self.action_state -= 1;
        }
        smooth_step_to_s(&mut self.actor.shape_rot.x, 0x1800, 2, 0xE38, 0x71C);
        approach_s(&mut self.actor.shape_rot.y, vec3f_yaw(self.actor.home_pos, player_pos), 2, 0xE38);
        scaled_step_to_s(&mut self.stem_parts_rot[0], 0xAAA, 0x444);
        scaled_step_to_s(&mut self.stem_parts_rot[1], -0x4718, 0x888);
        scaled_step_to_s(&mut self.stem_parts_rot[2], -0x6AA4, 0x888);
        if self.action_state == 0 {
            self.setup_attack();
        }
        self.compute_head_pos();
    }

    /// `EnDekubaba_RecoverFromAttackMiss`: stuck flat on the ground (the dust where it struck),
    /// it scrapes free at frame 10, rears up over 11 to 27, and after three more frames lunges
    /// again with Link within 80, else chomps in the air.
    fn recover_from_attack_miss(&mut self, play: &mut PlayState) {
        self.skel.update();
        let s = &mut self.stem_parts_rot;
        let r = &mut self.actor.shape_rot.x;
        match self.action_state {
            0 => {
                scaled_step_to_s(r, -0x93E, 0x38E);
                scaled_step_to_s(&mut s[0], -0x888, 0x16C);
                scaled_step_to_s(&mut s[1], -0x888, 0x16C);
                if scaled_step_to_s(&mut s[2], -0x888, 0x16C) {
                    // func_800286CC three times along the stem, 30 apart from home.
                    let dx = sin_s(self.actor.shape_rot.y) * 30.0 * self.scale_fac;
                    let dz = cos_s(self.actor.shape_rot.y) * 30.0 * self.scale_fac;
                    let mut eff_pos = self.actor.home_pos;
                    let (scale, step) = ((self.scale_fac * 500.0) as i16, (self.scale_fac * 50.0) as i16);
                    play.with_ss(|ss| {
                        for _ in 0..3 {
                            ss.func_800286cc(eff_pos, Vec3::ZERO, Vec3::ZERO, scale, step);
                            eff_pos.x += dx;
                            eff_pos.z += dz;
                        }
                    });
                    self.action_state = 1;
                }
            }
            11 => {
                scaled_step_to_s(r, -0x93E, 0x200);
                scaled_step_to_s(&mut s[0], -0xAAA, 0x200);
                scaled_step_to_s(&mut s[2], -0x5C71, 0x200);
                if scaled_step_to_s(&mut s[1], 0x238C, 0x200) {
                    self.action_state = 12;
                }
            }
            18 => {
                scaled_step_to_s(r, 0x2AA8, 0xAAA);
                if scaled_step_to_s(&mut s[0], 0x1554, 0x5B0) {
                    self.action_state = 25;
                }
                scaled_step_to_s(&mut s[1], -0x38E3, 0xAAA);
                scaled_step_to_s(&mut s[2], -0x5C71, 0x2D8);
            }
            25 => {
                scaled_step_to_s(r, -0x5550, 0xAAA);
                if scaled_step_to_s(&mut s[0], -0x6388, 0x93E) {
                    self.action_state = 26;
                }
                scaled_step_to_s(&mut s[1], -0x3FFC, 0x4FA);
                scaled_step_to_s(&mut s[2], -0x238C, 0x444);
            }
            26 => {
                scaled_step_to_s(r, 0x1800, 0x93E);
                if scaled_step_to_s(&mut s[0], -0x1555, 0x71C) {
                    self.action_state = 27;
                }
                scaled_step_to_s(&mut s[1], -0x38E3, 0x2D8);
                scaled_step_to_s(&mut s[2], -0x5C71, 0x5B0);
            }
            st if st >= 27 => {
                self.action_state += 1;
                if self.action_state > 30 {
                    if self.actor.xz_dist_to_player < 80.0 * self.scale_fac {
                        self.setup_prepare_attack();
                    } else {
                        self.setup_chomp_air();
                    }
                }
            }
            _ => {
                self.action_state += 1;
                if self.action_state == 10 {
                    audio_play_actor_sfx2(play, NA_SE_EN_DEKU_SCRAPE);
                }
                if self.action_state >= 12 {
                    scaled_step_to_s(&mut self.stem_parts_rot[2], -0x5C71, 0x88);
                }
            }
        }
        self.compute_head_pos();
    }

    /// `EnDekubaba_Soothe`: back to the chomping pose, then a few frames to settle.
    fn soothe(&mut self) {
        self.skel.update();
        if self.action_state >= 9 {
            let mut remains = 0;
            remains |= smooth_step_to_s(&mut self.actor.shape_rot.x, 0x1800, 1, 0x11C6, 0x71C);
            remains |= smooth_step_to_s(&mut self.stem_parts_rot[0], -0x1555, 1, 0xAAA, 0x71C);
            remains |= smooth_step_to_s(&mut self.stem_parts_rot[1], -0x38E3, 1, 0xE38, 0x71C);
            remains |= smooth_step_to_s(&mut self.stem_parts_rot[2], -0x5C71, 1, 0x11C6, 0x71C);
            if remains == 0 {
                self.action_state = 8;
            }
        } else {
            if self.action_state != 0 {
                self.action_state -= 1;
            }
            if self.action_state == 0 {
                self.setup_chomp_air();
            }
        }
        self.compute_head_pos();
    }

    /// `EnDekubaba_Attacked`: knocked upright; then dying out of health, else back to biting
    /// (strengthened) or stretched out to be hit (weakened, stunned).
    fn attacked(&mut self) {
        self.skel.update();
        let mut is_upright = true;
        is_upright &= scaled_step_to_s(&mut self.actor.shape_rot.x, -0x4000, 0xE38);
        is_upright &= scaled_step_to_s(&mut self.stem_parts_rot[0], -0x4000, 0xE38);
        is_upright &= scaled_step_to_s(&mut self.stem_parts_rot[1], -0x4000, 0xE38);
        is_upright &= scaled_step_to_s(&mut self.stem_parts_rot[2], -0x4000, 0xE38);
        if is_upright {
            if self.actor.col_chk_info.health == 0 {
                self.setup_die();
            } else {
                self.collider.base.ac_flags |= AC_ON;
                if self.action_state == EN_DEKUBABA_ATTACKED_TYPE_STRENGTHENED {
                    if self.actor.xz_dist_to_player < 80.0 * self.scale_fac {
                        self.setup_prepare_attack();
                    } else {
                        self.setup_soothe();
                    }
                } else {
                    self.setup_vulnerable();
                }
            }
        }
        self.compute_head_pos();
    }

    /// `EnDekubaba_Vulnerable`: stretched out until the timer runs out.
    fn vulnerable(&mut self) {
        self.skel.update();
        if self.action_state != 0 {
            self.action_state -= 1;
        }
        if self.action_state == 0 {
            self.disable_stem_collider_ac();
            if self.actor.xz_dist_to_player < 80.0 * self.scale_fac {
                self.setup_prepare_attack();
            } else {
                self.setup_soothe();
            }
        }
    }

    /// `EnDekubaba_Wobble`: a stem hit that didn't kill: the stem sways, each sway 0.8 of the
    /// last, until it's nearly upright.
    fn wobble(&mut self) {
        self.skel.update();
        let s = &mut self.stem_parts_rot;
        let s0 = s[0];
        scaled_step_to_s(&mut self.actor.shape_rot.x, s0, 0x71C);
        let s1 = s[1];
        scaled_step_to_s(&mut s[0], s1, 0x71C);
        let s2 = s[2];
        scaled_step_to_s(&mut s[1], s2, 0x71C);
        if scaled_step_to_s(&mut s[2], self.wobble_target, 0x71C) {
            self.wobble_target = (-16384.0 - ((self.wobble_target as i32 + 0x4000) as f32 * 0.8)) as i32 as i16;
        }
        let temp_v0 = self.wobble_target.wrapping_add(0x4000);
        if (temp_v0 as i32).abs() < 0x100 {
            self.collider.base.ac_flags |= AC_ON;
            if self.actor.xz_dist_to_player < 80.0 * self.scale_fac {
                self.setup_prepare_attack();
            } else {
                self.setup_soothe();
            }
        }
        self.compute_head_pos();
    }

    /// `EnDekubaba_DieDropStick`: the cut head flies off, the stem falling; on the ground it's a
    /// Deku Stick.
    fn die_drop_stick(&mut self, play: &mut PlayState) {
        step_to_f(&mut self.actor.speed_xz, 0.0, self.scale_fac * 0.1);
        if self.action_state == 0 {
            scaled_step_to_s(&mut self.actor.shape_rot.x, 0x4800, 0x71C);
            scaled_step_to_s(&mut self.stem_parts_rot[0], 0x4800, 0x71C);
            scaled_step_to_s(&mut self.stem_parts_rot[1], 0x4800, 0x71C);
            self.hahen_burst(play, self.actor.world_pos, 1);
            if self.actor.scale.x > 0.005 && (self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 || self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0) {
                self.actor.scale = Vec3::ZERO;
                self.actor.speed_xz = 0.0;
                self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE);
                self.hahen_burst(play, self.actor.world_pos, 15);
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
                self.action_state = 1;
            }
        } else if self.action_state == 1 {
            // func_800286CC four times along the fallen stem (20 apart), then at home.
            let mut eff_pos = self.actor.world_pos;
            let r = self.actor.shape_rot;
            let dy = sin_s(r.x) * 20.0;
            let dx = -20.0 * cos_s(r.x) * sin_s(r.y);
            let dz = -20.0 * cos_s(r.x) * cos_s(r.y);
            let (home, scale, step) = (self.actor.home_pos, (self.scale_fac * 500.0) as i16, (self.scale_fac * 100.0) as i16);
            play.with_ss(|ss| {
                for _ in 0..4 {
                    ss.func_800286cc(eff_pos, Vec3::ZERO, Vec3::ZERO, 500, 50);
                    eff_pos.x += dx;
                    eff_pos.y += dy;
                    eff_pos.z += dz;
                }
                ss.func_800286cc(home, Vec3::ZERO, Vec3::ZERO, scale, step);
            });
            self.setup_deku_stick(play);
        }
    }

    /// `EnDekubaba_Die`: shrinking into the ground, spinning; then the Deku Nuts (three for the
    /// big one), or a hit's drop (`dropFlag`) from table 3.
    fn die(&mut self, play: &mut PlayState) {
        step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, self.scale_fac * 5.0);
        if step_to_f(&mut self.actor.scale.x, self.scale_fac * 0.1 * 0.01, self.scale_fac * 0.1 * 0.01) {
            let (home, scale, step) = (self.actor.home_pos, (self.scale_fac * 500.0) as i16, (self.scale_fac * 100.0) as i16);
            play.with_ss(|ss| ss.func_800286cc(home, Vec3::ZERO, Vec3::ZERO, scale, step));
            let pos = self.actor.world_pos;
            if self.actor.drop_flag == 0 {
                crate::en_item00::item_drop_collectible(play, pos, ITEM00_NUTS);
                if self.actor.params == EN_DEKUBABA_TYPE_BIG {
                    crate::en_item00::item_drop_collectible(play, pos, ITEM00_NUTS);
                    crate::en_item00::item_drop_collectible(play, pos, ITEM00_NUTS);
                }
            } else {
                // COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_3, false).
                crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, COLLECTIBLE_DROP_TABLE_3 * 16);
            }
            self.actor.kill();
        }
        self.actor.scale.y = self.actor.scale.x;
        self.actor.scale.z = self.actor.scale.x;
        self.actor.shape_rot.z = self.actor.shape_rot.z.wrapping_add(0x1C70);
        self.hahen_burst(play, self.actor.home_pos, 1);
    }

    /// `EnDekubaba_DekuStick`: the stick lies for 200 frames (blinking for the last 40), to be
    /// picked up (`GI_DEKU_STICKS_1`).
    fn deku_stick(&mut self, play: &mut PlayState) {
        if self.action_state != 0 {
            self.action_state -= 1;
        }
        if oot_game::get_item::actor_has_parent(&self.actor) || self.action_state == 0 {
            self.actor.kill();
        } else {
            let a = self.actor.clone();
            oot_game::get_item::offer_get_item(play, &a, oot_game::item::GI_DEKU_STICKS_1);
        }
    }

    /// `EnDekubaba_CheckCollide`: this frame's hit (`AC_HIT`), by the damage table's reaction:
    /// - not stretched out: stunned by a Deku Nut or the boomerang (no damage from the latter),
    ///   weakened if it missed a bite (and left at 1 health), else strengthened;
    /// - stretched out (`EnDekubaba_Vulnerable`): a sword or the boomerang cuts it (it wobbles,
    ///   or out of health dies leaving a stick); anything but a Deku Nut knocks it upright.
    ///
    /// A hammer's shock wave (`actorCtx.unk_02`) weakens it too. Then its cry, or the finishing
    /// blow (`Enemy_StartFinishingBlow`) and its death cry. Not when hard (waiting in its leaves).
    fn check_collide(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            let jnt = self.collider.clone();
            self.actor.set_drop_flag_jnt_sph(&jnt, true);
            let reaction = self.actor.col_chk_info.damage_reaction;
            if self.collider.base.col_type != COL_MATERIAL_HARD && (reaction != EN_DEKUBABA_DMG_REACT_NONE || self.actor.col_chk_info.damage != 0) {
                let mut new_health = self.actor.col_chk_info.health as i32 - self.actor.col_chk_info.damage as i32;
                if self.action != Action::Vulnerable {
                    if reaction == EN_DEKUBABA_DMG_REACT_BOOMERANG || reaction == EN_DEKUBABA_DMG_REACT_STUN {
                        if reaction == EN_DEKUBABA_DMG_REACT_BOOMERANG {
                            new_health = self.actor.col_chk_info.health as i32;
                        }
                        self.setup_attacked(EN_DEKUBABA_ATTACKED_TYPE_STUNNED);
                    } else if self.action == Action::RecoverFromAttackMiss {
                        if new_health <= 0 {
                            new_health = 1;
                        }
                        self.setup_attacked(EN_DEKUBABA_ATTACKED_TYPE_WEAKENED);
                    } else {
                        self.setup_attacked(EN_DEKUBABA_ATTACKED_TYPE_STRENGTHENED);
                    }
                } else if reaction == EN_DEKUBABA_DMG_REACT_BOOMERANG || reaction == EN_DEKUBABA_DMG_REACT_SWORD {
                    if new_health > 0 {
                        self.setup_wobble();
                    } else {
                        self.setup_die_drop_stick();
                    }
                } else if reaction != EN_DEKUBABA_DMG_REACT_STUN {
                    self.setup_attacked(EN_DEKUBABA_ATTACKED_TYPE_STRENGTHENED);
                } else {
                    return;
                }
                self.actor.col_chk_info.health = new_health.max(0) as u8;
                if reaction == EN_DEKUBABA_DMG_REACT_FIRE {
                    // EffectSsEnFire_SpawnVec3f four times, on body parts 0 to 3: the struct's
                    // unused fields at 0x14C (EnDekubaba::effect_fire_pos).
                    let scale = (self.scale_fac * 70.0) as i16;
                    let me = self.ss_actor(play);
                    for i in 0..4 {
                        play.effect_ss_en_fire_spawn_vec3f(me, self.actor.world_pos, scale, 0, 0, i);
                    }
                }
            } else {
                return;
            }
        } else if play.actors.unk_02 != 0 && self.collider.base.col_type != COL_MATERIAL_HARD && self.action != Action::Vulnerable && self.action != Action::Attacked && self.actor.col_chk_info.health != 0 {
            self.actor.col_chk_info.health -= 1;
            self.actor.drop_flag = 0;
            self.setup_attacked(EN_DEKUBABA_ATTACKED_TYPE_WEAKENED);
        } else {
            return;
        }
        if self.actor.col_chk_info.health != 0 {
            if self.action_state == 2 {
                audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
            } else {
                audio_play_actor_sfx2(play, NA_SE_EN_DEKU_DAMAGE);
            }
        } else {
            enemy_start_finishing_blow(play, &self.actor);
            audio_play_actor_sfx2(play, if self.actor.params == EN_DEKUBABA_TYPE_BIG { NA_SE_EN_DEKU_DEAD } else { NA_SE_EN_DEKU_JR_DEAD });
        }
    }

    /// `EnDekubaba_DrawStem0` and `EnDekubaba_DrawStem`'s matrices: the stem's part `i`'s
    /// (`MtxF`, as the C builds it, which the spheres follow), for the stem `n` parts long, and
    /// the focus from the first. `EnDekubaba_DrawStem2`'s third part of the falling stem follows
    /// the second's matrix.
    fn stem_matrices(&self, waiting: bool, dropping: bool) -> (Vec<MtxF>, Vec3) {
        let scale = self.scale_fac * 0.01;
        let a = &self.actor;
        if waiting {
            // EnDekubaba_DrawStem0: at the leaves, 6 down, the first part only.
            let mut m = MtxF::set_translate(a.home_pos.x, a.home_pos.y + (-6.0 * self.scale_fac), a.home_pos.z);
            m.rotate_zyx(self.stem_parts_rot[0], a.shape_rot.y, 0);
            m.scale(scale, scale, scale);
            // Actor_SetFocus(&this->actor, 0.0f).
            return (vec![m], a.world_pos);
        }
        let n = if dropping { 2 } else { 3 };
        let mut mf = MtxF::set_translate(a.world_pos.x, a.world_pos.y, a.world_pos.z);
        mf.scale(scale, scale, scale);
        let mut out = Vec::new();
        let mut focus = a.focus_pos;
        for i in 0..n {
            mf.yw += 20.0 * sin_s(self.stem_parts_rot[i]) * self.scale_fac;
            let dxz = cos_s(self.stem_parts_rot[i]) * 20.0 * self.scale_fac;
            mf.xw -= dxz * sin_s(a.shape_rot.y);
            mf.zw -= dxz * cos_s(a.shape_rot.y);
            let mut m = mf;
            m.rotate_zyx(self.stem_parts_rot[i], a.shape_rot.y, 0);
            if i == 0 {
                focus = if self.action != Action::Wobble { Vec3::new(mf.xw, mf.yw, mf.zw) } else { Vec3::new(a.home_pos.x, a.home_pos.y + 40.0 * self.scale_fac, a.home_pos.z) };
            }
            out.push(m);
        }
        if dropping {
            // EnDekubaba_DrawStem2: Matrix_RotateZYX on the matrix the leaves' draw left (the base's,
            // MTXMODE_NEW at home): the C's current matrix there.
            let mut m = self.base_leaves_matrix();
            m.rotate_zyx(self.stem_parts_rot[2], a.shape_rot.y, 0);
            out.push(m);
        }
        (out, focus)
    }

    /// The base leaves' matrix: at home, turned by `home.rot.y`, scaled.
    fn base_leaves_matrix(&self) -> MtxF {
        let scale = self.scale_fac * 0.01;
        let a = &self.actor;
        let mut m = MtxF::set_translate(a.home_pos.x, a.home_pos.y, a.home_pos.z);
        // Matrix_RotateY(BINANG_TO_RAD(this->actor.home.rot.y), MTXMODE_APPLY).
        m.rotate_y(oot_game::sys_matrix::binang_to_rad(a.home_rot.y));
        m.scale(scale, scale, scale);
        m
    }

    /// `Actor_Draw`'s model matrix (`Matrix_SetTranslateRotateYXZ`, then the scale).
    fn actor_mtx(&self) -> MtxF {
        let a = &self.actor;
        let mut m = MtxF::set_translate_rotate_yxz(a.world_pos.x, a.world_pos.y + a.shape_y_offset * a.scale.y, a.world_pos.z, [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z]);
        m.scale(a.scale.x, a.scale.y, a.scale.z);
        m
    }
}

impl ActorImpl for EnDekubaba {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnDekubaba_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.collider.base.at_flags & AT_HIT != 0 {
            self.collider.base.at_flags &= !AT_HIT;
            self.setup_soothe();
        }
        self.check_collide(play);
        match self.action {
            Action::WaitPlayerNear => self.wait_player_near(play),
            Action::ExitGround => self.exit_ground(play),
            Action::EnterGround => self.enter_ground(play),
            Action::ChompAir => self.chomp_air(play),
            Action::Attack => self.attack(play),
            Action::PrepareAttack => self.prepare_attack(play),
            Action::RecoverFromAttackMiss => self.recover_from_attack_miss(play),
            Action::Soothe => self.soothe(),
            Action::Attacked => self.attacked(),
            Action::Vulnerable => self.vulnerable(),
            Action::Wobble => self.wobble(),
            Action::DieDropStick => self.die_drop_stick(play),
            Action::Die => self.die(play),
            Action::DekuStick => self.deku_stick(play),
        }
        if self.action == Action::DieDropStick {
            self.actor.move_forward();
            self.actor.update_bg_check_info(&play.col, 10.0, self.scale_fac * 15.0, 10.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        } else if self.action != Action::DekuStick {
            self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
            if self.floor_poly.is_none() {
                self.floor_poly = self.actor.floor_poly;
            }
        }
        if self.action == Action::Attack {
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            self.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
        }
        if self.collider.base.ac_flags & AC_ON != 0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
        if self.action != Action::DekuStick {
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
    }

    /// `EnDekubaba_Draw`'s effects on the actor: the head's sphere follows its root limb
    /// (`EnDekubaba_PostLimbDraw`), the stem's its parts (`Collider_UpdateSpheres` in
    /// `EnDekubaba_DrawStem`), and the focus is set.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.action == Action::DekuStick || self.actor.killed {
            return;
        }
        // SkelAnime_DrawOpa's root limb: Matrix_TranslateRotateZYX(jointTable[0], jointTable[1]).
        let j = &self.skel.joint_table;
        let mut root = self.actor_mtx();
        root.translate_rotate_zyx(Vec3::new(j[0][0] as f32, j[0][1] as f32, j[0][2] as f32), j[1]);
        self.collider.update_spheres(DEKUBABA_HEAD_LIMB_ROOT, &root.to_mat4());
        let waiting = self.action == Action::WaitPlayerNear;
        let dropping = self.action == Action::DieDropStick;
        let (stems, focus) = self.stem_matrices(waiting, dropping);
        if waiting {
            self.actor.set_focus(0.0);
        } else {
            for (i, m) in stems.iter().enumerate().take(if dropping { 2 } else { 3 }) {
                let mm = m.to_mat4();
                self.collider.update_spheres(51 + i as u8 * 2, &mm);
                self.collider.update_spheres(52 + i as u8 * 2, &mm);
            }
            self.actor.focus_pos = focus;
            if dropping && let Some(m) = stems.get(2) {
                let mm = m.to_mat4();
                self.collider.update_spheres(55, &mm);
                self.collider.update_spheres(56, &mm);
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.stem_parts_rot[0], self.stem_parts_rot[1], self.stem_parts_rot[2], self.actor.home_rot.y];
        rs.values = vec![self.scale_fac, self.actor.home_pos.x, self.actor.home_pos.y, self.actor.home_pos.z];
        let action = match self.action {
            Action::WaitPlayerNear => 1,
            Action::DieDropStick => 2,
            Action::DekuStick => 3,
            Action::Wobble => 4,
            _ => 0,
        };
        rs.switches = vec![action, self.action_state as u16 as u32];
        rs
    }

    /// `EnDekubaba_Draw`: the head (`SkelAnime_DrawOpa`), the stem (`EnDekubaba_DrawStem0` while
    /// waiting, else `EnDekubaba_DrawStem`), the base leaves, the falling stem's third part, and
    /// the shadow on its floor; or, as a Deku Stick, the stick (blinking for its last 40
    /// frames).
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), [s0, s1, s2, home_rot_y], [scale_fac, hx, hy, hz], [action, action_state]) =
            (&rs.joints, rs.angles.as_slice(), rs.values.as_slice(), rs.switches.as_slice())
        else {
            return;
        };
        let (scale_fac, home, stem) = (*scale_fac, Vec3::new(*hx, *hy, *hz), [*s0, *s1, *s2]);
        let model = oot_game::play::actor_draw_matrix(rs);
        let mesh = |name: &str| MeshKey::named(keys::mesh(OBJECT, name));
        if *action == 3 {
            let st = *action_state as u16 as i16;
            if st > 40 || st & 1 != 0 {
                out.opa.push(DrawCmd::new(mesh("gDekubabaDekuStickDL"), model * Mat4::from_translation(Vec3::new(0.0, 0.0, 200.0))));
            }
            return;
        }
        if let Some(skeleton) = &self.skeleton {
            let bones = skeleton.pose(joints);
            out.opa.push(DrawCmd { mesh: mesh("gDekubabaHeadSkel"), transform: model, bones, params: Default::default() });
        }
        let scale = scale_fac * 0.01;
        let yaw = rs.rot[1];
        let rot = |pitch: i16| Mat4::from_rotation_y(eng_math::binang_to_rad(yaw)) * Mat4::from_rotation_x(eng_math::binang_to_rad(pitch));
        let stems = ["gDekubabaStem0DL", "gDekubabaStem1DL", "gDekubabaStem2DL"];
        if *action == 1 {
            let m = Mat4::from_translation(Vec3::new(home.x, home.y - 6.0 * scale_fac, home.z)) * rot(stem[0]) * Mat4::from_scale(Vec3::splat(scale));
            out.opa.push(DrawCmd::new(mesh(stems[0]), m));
        } else {
            let n = if *action == 2 { 2 } else { 3 };
            let mut p = rs.pos;
            for i in 0..n {
                p.y += 20.0 * sin_s(stem[i]) * scale_fac;
                let dxz = cos_s(stem[i]) * 20.0 * scale_fac;
                p.x -= dxz * sin_s(yaw);
                p.z -= dxz * cos_s(yaw);
                // Matrix_Put(mf) with the scale already in, then Matrix_RotateZYX.
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(scale)) * rot(stem[i]);
                out.opa.push(DrawCmd::new(mesh(stems[i]), m));
            }
        }
        let leaves = Mat4::from_translation(home) * Mat4::from_rotation_y(eng_math::binang_to_rad(*home_rot_y)) * Mat4::from_scale(Vec3::splat(scale));
        out.opa.push(DrawCmd::new(mesh("gDekubabaBaseLeavesDL"), leaves));
        if *action == 2 {
            out.opa.push(DrawCmd::new(mesh(stems[2]), leaves * rot(stem[2])));
        }
        // EnDekubaba_DrawShadow: gCircleShadowDL (a circle of radius 74 on one triangle) on the
        // floor's plane at home (func_80038A28), scaled by 0.15; drawn as the builtin disc.
        if let Some(f) = self.floor_poly {
            let n = play.col.poly(f).normal;
            let r = 74.0 * scale_fac * 0.15;
            let m = floor_matrix([n[0], n[1], n[2]], home) * Mat4::from_scale(Vec3::new(r, 1.0, r));
            out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), m));
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::JntSph(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// `func_80038A28` (`z_bgcheck.c`): a matrix at `t` whose y axis is the floor's normal (the
/// normal's `s16` components over 32767, `CollisionPoly_GetNormalF`).
pub(crate) fn floor_matrix(normal: [i16; 3], t: Vec3) -> Mat4 {
    let (nx, ny, nz) = (normal[0] as f32 * (1.0 / 32767.0), normal[1] as f32 * (1.0 / 32767.0), normal[2] as f32 * (1.0 / 32767.0));
    let mut xx = (1.0 - nx * nx).sqrt();
    let (zz, yz);
    if !eng_math::is_zero(xx) {
        let xx_inv = 1.0 / xx;
        zz = ny * xx_inv;
        yz = -(nz * xx_inv);
    } else {
        let z = (1.0 - ny * ny).sqrt();
        if !eng_math::is_zero(z) {
            let zz_inv = 1.0 / z;
            yz = nx * zz_inv;
            xx = -(nz * zz_inv);
        } else {
            yz = 0.0;
            xx = 0.0;
        }
        zz = z;
    }
    let m = MtxF { xx, yx: -nx * zz, zx: nx * yz, wx: 0.0, xy: nx, yy: ny, zy: nz, wy: 0.0, xz: 0.0, yz, zz, wz: 0.0, xw: t.x, yw: t.y, zw: t.z, ww: 1.0 };
    m.to_mat4()
}
