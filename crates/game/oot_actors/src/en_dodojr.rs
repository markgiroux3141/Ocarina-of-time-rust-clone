//! `En_Dodojr` (`ovl_En_Dodojr/z_en_dodojr.c`): the Baby Dodongo, `object_dodojr`'s
//! `object_dodojr_Skel_0020E0` at scale 0.02. Placed in Dodongo's Cavern (rooms 1, 7, 8, 10) and
//! the Spirit Temple.
//!
//! - **Buried** (`EnDodojr_WaitUnderground`): not drawn, not targetable, no collider. Link within
//!   320 across and less than 40 below it, it **surfaces** (`EnDodojr_EmergeFromGround`,
//!   `NA_SE_EN_DODO_M_UP`): 60 down and turned nose-down (`rot.x` -0x4000), it tips back up
//!   (`Math_SmoothStepToS`, a quarter at most 1000, at least 100 a frame) rising with it, dust
//!   round it (`EnDodojr_SpawnLargeDust`); level, its shadow on, it **crawls**.
//! - **The crawl** (`EnDodojr_CrawlTowardsTarget`): at 2.6 (`object_dodojr_Anim_000860`,
//!   `NA_SE_EN_DODO_M_MOVE` every 5 frames, a puff of dust a frame), turning (`Math_SmoothStepToS`
//!   a tenth, at most 1000) towards the nearest free bomb (`EnDodojr_CheckNearbyBombs`: an
//!   `ACTOR_EN_BOM` of params 0 with no parent; a Bomb Flower's isn't one), else Link; further than
//!   80, towards a point 80 off it by one of twelve angles (`EnDodojr_UpdateCrawl`: by its home and
//!   `state.frames / 30`, given to `sinf` and `cosf` as radians though they read as degrees). Link
//!   within 40 across: `NA_SE_EN_DODO_M_CRY` and it **jumps at him**
//!   (`EnDodojr_JumpAttackBounce`: up at 10, three bounces, its cylinder hurting 8,
//!   `ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT`), then crawls again. A wall: `NA_SE_EN_DODO_M_DOWN` and
//!   it **burrows away** for good (`EnDodojr_Despawn`, 30 frames sinking 60, gone).
//! - **A bomb** within 30 of it (`EnDodojr_TryEatBomb`, the bomb's `parent` set to it): it **eats
//!   it** (`EnDodojr_EatBomb`, `NA_SE_EN_DODO_M_EAT`, frames 8 to 12 of `object_dodojr_Anim_000724`
//!   holding it at its mouth, `headPos`, its fuse held), swallows (`NA_SE_EN_DODO_K_DRINK`, the bomb
//!   gone, 24 frames), and it blows up inside (`EnDodojr_DoSwallowedBombEffects`:
//!   `NA_SE_IT_BOMB_EXPLOSION`, red 8 frames, its body swelling 1.2 and shrinking with the
//!   filter), bounces three times and dies.
//! - **Hit** (`EnDodojr_CheckDamaged`, damage table `DamageTable_Get(4)`): damage with no reaction
//!   kills it (`Enemy_StartFinishingBlow`, two frames' freeze, `NA_SE_EN_DODO_M_DEAD`, three
//!   flipping bounces, `object_dodojr_Anim_0004A0`); reaction 1 (a Deku Nut) stuns it
//!   (`EnDodojr_StunnedBounce`, `NA_SE_EN_GOMA_JR_FREEZE`, blue 120 frames, three bounces, then
//!   shaking for its last 30) and it crawls again. The hammer's shock wave (`actorCtx.unk_02`)
//!   kills it once it's up.
//! - **Death** (`EnDodojr_DeathSequence`): flashing red ever faster (7 flashes), then a bomb
//!   (`ACTOR_EN_BOM`, `BOMB_BODY`) where it lies with its fuse at 0: it blows in its own blast; 8
//!   frames on a drop from table 4 (`Item_DropCollectibleRandom`) and it's gone.
//!
//! The whole overlay is ported. The draw (`EnDodojr_Draw`, not while buried or dropping its item)
//! is the skeleton (`SkelAnime_DrawOpa`), its root limb scaled by `rootScale`
//! (`EnDodojr_OverrideLimbDraw`); `draw_update` puts `headPos` (480, 620, 0 from limb 4's
//! parent's matrix), which the eating reads. The circle shadow (18, off at first) is the shape's
//! (`ActorShape_Init`'s, drawn by `Actor_Draw`). Nothing sets the hammer's `unk_02` yet (the hammer isn't ported). @bug
//! (game): `EnDodojr_CheckNearbyBombs` narrows its search box to the last bomb's world position
//! rather than its distance, so past the first bomb it finds, a nearer one may be passed over or a
//! further one taken; the surfacing dust is at y 0 (`floorHeight` not read yet, the C's comment).
//!
//! Params: `params` whole, read by nothing (every placement 0xFFFF).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::{Cylinder16, vec3f_dist_xyz};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, smooth_step_to_s, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_EXPLOSIVE, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EN_DODO_M_GND, NA_SE_EN_GOMA_JR_FREEZE, NA_SE_IT_BOMB_EXPLOSION};
use oot_game::collision_check::*;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_LOOP_INTERP, ANIMMODE_ONCE, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};

use crate::en_bom::{ACTOR_EN_BOM, EnBom};

/// `ACTOR_EN_DODOJR` (`actor_table.h`: 0x002F).
pub const ACTOR_EN_DODOJR: i16 = 0x002F;
/// `OBJECT_DODOJR` (`object_table.h`: 0x001F).
pub const OBJECT: &str = "object_dodojr";
/// `object_dodojr_Skel_0020E0` (`object_dodojr.xml`): 14 limbs.
pub const SKEL: &str = "object_dodojr_Skel_0020E0";

/// The animations (`object_dodojr.xml`): the flip (hit, dying), the swallowed bomb's and the
/// stun's lying, the jump and the eating, the crawl, the idle (the init's).
pub const ANIM_FLIP: &str = "object_dodojr_Anim_0004A0";
pub const ANIM_LIE: &str = "object_dodojr_Anim_0005F0";
pub const ANIM_JUMP: &str = "object_dodojr_Anim_000724";
pub const ANIM_CRAWL: &str = "object_dodojr_Anim_000860";
pub const ANIM_INIT: &str = "object_dodojr_Anim_0009D4";

/// `En_Dodojr_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, `OBJECT_DODOJR`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_DODOJR, name: "En_Dodojr", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE, object: OBJECT, schema: &SCHEMA };

/// `params`: read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "read by nothing (every placement 0xFFFF)");

/// En_Dodojr's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[PARAMS], flags: &[], todo: false, objects: &[] };

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_DODO_M_CRY` (0x3820).
pub const NA_SE_EN_DODO_M_CRY: u16 = 0x3820;
/// `NA_SE_EN_DODO_M_DEAD` (0x3821).
pub const NA_SE_EN_DODO_M_DEAD: u16 = 0x3821;
/// `NA_SE_EN_DODO_M_MOVE` (0x3822).
pub const NA_SE_EN_DODO_M_MOVE: u16 = 0x3822;
/// `NA_SE_EN_DODO_M_DOWN` (0x3823).
pub const NA_SE_EN_DODO_M_DOWN: u16 = 0x3823;
/// `NA_SE_EN_DODO_M_UP` (0x3824).
pub const NA_SE_EN_DODO_M_UP: u16 = 0x3824;
/// `NA_SE_EN_DODO_M_EAT` (0x3826).
pub const NA_SE_EN_DODO_M_EAT: u16 = 0x3826;
/// `NA_SE_EN_DODO_K_DRINK` (0x3850).
pub const NA_SE_EN_DODO_K_DRINK: u16 = 0x3850;

/// `NAVI_ENEMY_BABY_DODONGO` (`actor.h`: 0x0E).
pub const NAVI_ENEMY_BABY_DODONGO: u8 = 0x0E;
/// `COLLECTIBLE_DROP_TABLE_4` (`z_en_item00.h`).
const COLLECTIBLE_DROP_TABLE_4: i16 = 4;
/// `BOMB_BODY` (`z_en_bom.h`'s `EnBomType`).
pub const BOMB_BODY: i16 = 0;

/// `sCylinderInit`: biting Link (`0xFFCFFFFF` for 8), hit by Player's (`0xFFC5FFFF`), pushing; 18
/// by 20.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFC5_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 18, height: 20, y_shift: 0, pos: [0, 0, 0] },
};

/// `sColChkInit`: 1 health, 2 by 25, 25 up, immovable.
pub const COL_CHK_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 1, cyl_radius: 2, cyl_height: 25, cyl_y_shift: 25, mass: MASS_IMMOVABLE };

/// `DamageTable_Get(4)` (`z_collision_btltbls.c`'s `sDamageTablePresets[4]`, "Used by
/// En_Dodojr").
pub static S_DAMAGE_TABLE_PRESET_4: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x1), // Deku nut
        dmg_entry(2, 0x0), // Deku stick
        dmg_entry(1, 0x0), // Slingshot
        dmg_entry(2, 0x0), // Explosive
        dmg_entry(1, 0x0), // Boomerang
        dmg_entry(2, 0x0), // Normal arrow
        dmg_entry(2, 0x0), // Hammer swing
        dmg_entry(2, 0x0), // Hookshot
        dmg_entry(1, 0x0), // Kokiri sword
        dmg_entry(2, 0x0), // Master sword
        dmg_entry(4, 0x0), // Giant's Knife
        dmg_entry(4, 0x0), // Fire arrow
        dmg_entry(4, 0x0), // Ice arrow
        dmg_entry(4, 0x0), // Light arrow
        dmg_entry(0, 0x0), // Unk arrow 1
        dmg_entry(0, 0x0), // Unk arrow 2
        dmg_entry(0, 0x0), // Unk arrow 3
        dmg_entry(0, 0x0), // Fire magic
        dmg_entry(0, 0x0), // Ice magic
        dmg_entry(0, 0x0), // Light magic
        dmg_entry(0, 0x0), // Shield
        dmg_entry(0, 0x0), // Mirror Ray
        dmg_entry(1, 0x0), // Kokiri spin
        dmg_entry(2, 0x0), // Giant spin
        dmg_entry(4, 0x0), // Master spin
        dmg_entry(2, 0x0), // Kokiri jump
        dmg_entry(4, 0x0), // Giant jump
        dmg_entry(8, 0x0), // Master jump
        dmg_entry(0, 0x0), // Unknown 1
        dmg_entry(0, 0x0), // Unblockable
        dmg_entry(0, 0x0), // Hammer jump
        dmg_entry(0, 0x0), // Unknown 2
    ],
};

/// `EnDodojr_UpdateCrawl`'s `angles` (in degrees' numbers; `sinf` and `cosf` take them as
/// radians).
pub const CRAWL_ANGLES: [f32; 12] = [0.0, 210.0, 60.0, 270.0, 120.0, 330.0, 180.0, 30.0, 240.0, 90.0, 300.0, 150.0];

/// `EnDodojr_OverrideLimbDraw`'s `D_809F7F64`: its mouth, on limb 4's parent's matrix.
pub const HEAD_OFFSET: Vec3 = Vec3::new(480.0, 620.0, 0.0);

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitUnderground,
    DropItem,
    EmergeFromGround,
    CrawlTowardsTarget,
    StunnedBounce,
    JumpAttackBounce,
    Stunned,
    SwallowBomb,
    SwallowedBombDeathBounce,
    SwallowedBombDeathSequence,
    StandardDeathBounce,
    Despawn,
    DeathSequence,
    WaitFreezeFrames,
    EatBomb,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::WaitUnderground => "EnDodojr_WaitUnderground",
            Action::DropItem => "EnDodojr_DropItem",
            Action::EmergeFromGround => "EnDodojr_EmergeFromGround",
            Action::CrawlTowardsTarget => "EnDodojr_CrawlTowardsTarget",
            Action::StunnedBounce => "EnDodojr_StunnedBounce",
            Action::JumpAttackBounce => "EnDodojr_JumpAttackBounce",
            Action::Stunned => "EnDodojr_Stunned",
            Action::SwallowBomb => "EnDodojr_SwallowBomb",
            Action::SwallowedBombDeathBounce => "EnDodojr_SwallowedBombDeathBounce",
            Action::SwallowedBombDeathSequence => "EnDodojr_SwallowedBombDeathSequence",
            Action::StandardDeathBounce => "EnDodojr_StandardDeathBounce",
            Action::Despawn => "EnDodojr_Despawn",
            Action::DeathSequence => "EnDodojr_DeathSequence",
            Action::WaitFreezeFrames => "EnDodojr_WaitFreezeFrames",
            Action::EatBomb => "EnDodojr_EatBomb",
        }
    }
}

/// The animations of `object_dodojr`.
#[derive(Default)]
struct Anims {
    flip: Option<Anim>,
    lie: Option<Anim>,
    jump: Option<Anim>,
    crawl: Option<Anim>,
    init: Option<Anim>,
}

// shared: DECR (`z_math.h`)
/// `DECR(x)`: down to 0, the new value.
fn decr(x: &mut i16) -> i16 {
    if *x != 0 {
        *x -= 1;
    }
    *x
}

pub struct EnDodojr {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Anims,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `bomb`: the bomb it's after or eating.
    pub bomb: Option<ActorHandle>,
    /// `headPos`: its mouth, from the last draw.
    pub head_pos: Vec3,
    pub dust_pos: Vec3,
    /// `counter`: bounces left, or flashes left dying.
    pub counter: i16,
    pub stun_timer: i16,
    pub freeze_frame_timer: i16,
    /// `timer`: burrowing, swallowing, dying.
    pub timer: i16,
    pub crawl_sfx_timer: i16,
    /// `rootScale`: the root limb's swell.
    pub root_scale: f32,
}

impl EnDodojr {
    /// `EnDodojr_Init`: no shadow (18 when on), the skeleton playing `object_dodojr_Anim_0009D4`,
    /// the cylinder, `DamageTable_Get(4)`, the Baby Dodongo's Navi text, not targetable, scale
    /// 0.02, buried.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::None, 18.0);
        let mut anims = Anims::default();
        let mut skeleton = None;
        if let Some(a) = play.assets.clone() {
            let anim = |name: &str| a.animation(OBJECT, name).map_err(|e| log::error!("En_Dodojr: {e:#}")).ok();
            skeleton = a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Dodojr: {e:#}")).ok();
            anims = Anims { flip: anim(ANIM_FLIP), lie: anim(ANIM_LIE), jump: anim(ANIM_JUMP), crawl: anim(ANIM_CRAWL), init: anim(ANIM_INIT) };
        }
        // SkelAnime_Init(.., &object_dodojr_Skel_0020E0, &object_dodojr_Anim_0009D4, .., 15).
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(14);
        let skel = SkelAnimeStd::init_flex(limbs, anims.init.clone());
        let collider = ColliderCylinder::new(&CYLINDER_INIT);
        actor.col_chk_info.set_info2(Some(&S_DAMAGE_TABLE_PRESET_4), &COL_CHK_INIT);
        actor.navi_enemy_id = NAVI_ENEMY_BABY_DODONGO;
        actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        actor.scale = Vec3::splat(0.02);
        Box::new(EnDodojr {
            actor,
            skel,
            skeleton,
            anims,
            action: Action::WaitUnderground,
            collider,
            bomb: None,
            head_pos: Vec3::ZERO,
            dust_pos: Vec3::ZERO,
            counter: 0,
            stun_timer: 0,
            freeze_frame_timer: 0,
            timer: 0,
            crawl_sfx_timer: 0,
            root_scale: 0.0,
        })
    }

    /// `Animation_Change(&skelAnime, anim, speed, start, Animation_GetLastFrame(anim) or end,
    /// mode, morph)`.
    fn change(&mut self, anim: Option<Anim>, speed: f32, start: f32, end: Option<f32>, mode: u8, morph: f32) {
        if let Some(a) = anim {
            let end = end.unwrap_or_else(|| a.last_frame());
            self.skel.change(a, speed, start, end, mode, morph);
        }
    }

    /// `EnDodojr_DoSwallowedBombEffects`: the blast's sound, red for 8 frames.
    fn do_swallowed_bomb_effects(&mut self, play: &mut PlayState) {
        audio_play_actor_sfx2(play, NA_SE_IT_BOMB_EXPLOSION);
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 200, COLORFILTER_BUFFLAG_OPA, 8);
    }

    /// `EnDodojr_SpawnLargeDust`: `count + 1` puffs (`func_8002836C`, 120 big, 40 frames) 22 round
    /// `dustPos` from a random angle, `0x10000 / count` apart, drifting up 0.3 and up to 2 across.
    fn spawn_large_dust(&self, play: &mut PlayState, count: i32) {
        let prim = [170, 130, 90, 255];
        let env = [100, 60, 20, 0];
        let velocity = Vec3::ZERO;
        let dust_pos = self.dust_pos;
        play.with_ss(|ss| {
            let mut accel = Vec3::new(0.0, 0.3, 0.0);
            let mut angle = ((ss.rand.zero_one() - 0.5) * 65536.0) as i32 as i16;
            let mut pos = Vec3::new(0.0, dust_pos.y, 0.0);
            let mut i = count;
            while i >= 0 {
                accel.x = (ss.rand.zero_one() - 0.5) * 4.0;
                accel.z = (ss.rand.zero_one() - 0.5) * 4.0;
                pos.x = (sin_s(angle) * 22.0) + dust_pos.x;
                pos.z = (cos_s(angle) * 22.0) + dust_pos.z;
                ss.func_8002836c(pos, velocity, accel, prim, env, 120, 40, 10);
                i -= 1;
                angle = angle.wrapping_add((0x10000 / count) as i16);
            }
        });
    }

    /// `EnDodojr_SpawnSmallDust`: a puff (100 big, 60 frames) 11 from `arg2` on its floor.
    fn spawn_small_dust(&self, play: &mut PlayState, arg2: Vec3) {
        let prim = [170, 130, 90, 255];
        let env = [100, 60, 20, 0];
        let velocity = Vec3::ZERO;
        let floor_height = self.actor.floor_height;
        play.with_ss(|ss| {
            let mut accel = Vec3::new(0.0, 0.3, 0.0);
            let angle = ((ss.rand.zero_one() - 0.5) * 65536.0) as i32 as i16;
            accel.x = (ss.rand.zero_one() - 0.5) * 2.0;
            accel.z = (ss.rand.zero_one() - 0.5) * 2.0;
            let pos = Vec3::new((sin_s(angle) * 11.0) + arg2.x, floor_height, (cos_s(angle) * 11.0) + arg2.z);
            ss.func_8002836c(pos, velocity, accel, prim, env, 100, 60, 8);
        });
    }

    /// `EnDodojr_UpdateBounces`: falling with bounces left, on the ground: `NA_SE_EN_DODO_M_GND`,
    /// dust, up again at `10 / (4 - counter)`; the last bounce stops it and says so.
    fn update_bounces(&mut self, play: &mut PlayState) -> bool {
        if self.actor.velocity.y >= 0.0 {
            return false;
        }
        if self.counter == 0 {
            return false;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
            self.dust_pos = self.actor.world_pos;
            self.spawn_large_dust(play, 10);
            self.actor.velocity.y = 10.0 / (4 - self.counter) as f32;
            self.counter -= 1;
            if self.counter == 0 {
                self.actor.velocity.y = 0.0;
                return true;
            }
        }
        false
    }

    /// `EnDodojr_SetupCrawlTowardsTarget`: the crawl at 1.8, speed 2.6, gravity -0.8.
    fn setup_crawl_towards_target(&mut self) {
        self.change(self.anims.crawl.clone(), 1.8, 0.0, None, ANIMMODE_LOOP_INTERP, -10.0);
        self.actor.velocity.y = 0.0;
        self.actor.speed_xz = 2.6;
        self.actor.gravity = -0.8;
    }

    /// `EnDodojr_SetupFlipBounce`: the flip once, stopped; three bounces from up at 10 unless
    /// some are left.
    fn setup_flip_bounce(&mut self) {
        self.change(self.anims.flip.clone(), 1.0, 0.0, None, ANIMMODE_ONCE, -10.0);
        self.actor.speed_xz = 0.0;
        self.actor.velocity.x = 0.0;
        self.actor.velocity.z = 0.0;
        self.actor.gravity = -0.8;
        if self.counter == 0 {
            self.counter = 3;
            self.actor.velocity.y = 10.0;
        }
    }

    /// `EnDodojr_SetupSwallowedBombDeathSequence`: lying (`object_dodojr_Anim_0005F0`).
    fn setup_swallowed_bomb_death_sequence(&mut self) {
        self.change(self.anims.lie.clone(), 1.0, 0.0, None, ANIMMODE_LOOP, 0.0);
        self.actor.velocity.y = 0.0;
        self.actor.gravity = -0.8;
    }

    /// `EnDodojr_SetupJumpAttackBounce`: the jump, three bounces from up at 10.
    fn setup_jump_attack_bounce(&mut self) {
        self.change(self.anims.jump.clone(), 1.0, 0.0, None, ANIMMODE_LOOP, -10.0);
        self.actor.gravity = -0.8;
        self.counter = 3;
        self.actor.velocity.y = 10.0;
    }

    /// `EnDodojr_SetupDespawn`: no shadow, not targetable, its home where it is, 30 frames.
    fn setup_despawn(&mut self) {
        self.actor.shape_shadow_draw = oot_game::shadow::ShadowDraw::None;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.actor.home_pos = self.actor.world_pos;
        self.actor.speed_xz = 0.0;
        self.actor.gravity = -0.8;
        self.timer = 30;
        self.dust_pos = self.actor.world_pos;
    }

    /// `EnDodojr_SetupEatBomb`: frames 8 to 12 of the jump once, `NA_SE_EN_DODO_M_EAT`, stopped.
    fn setup_eat_bomb(&mut self, play: &mut PlayState) {
        self.change(self.anims.jump.clone(), 1.0, 8.0, Some(12.0), ANIMMODE_ONCE, 0.0);
        audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_EAT);
        self.actor.speed_xz = 0.0;
        self.actor.velocity.x = 0.0;
        self.actor.velocity.z = 0.0;
        self.actor.gravity = -0.8;
    }

    /// `EnDodojr_CheckNearbyBombs`: the explosives' list from its head (newest first) for a bomb
    /// (`ACTOR_EN_BOM`, params 0, no parent, not killed) inside the box: the first within 99999 on
    /// each axis, then (@bug (game)) within the last one found's world position's magnitudes.
    fn check_nearby_bombs(&mut self, play: &PlayState) -> bool {
        let mut max_bomb_range = Vec3::new(99999.0, 99999.0, 99999.0);
        let mut found_bomb = false;
        self.bomb = None;
        for &h in play.actors.category(ACTORCAT_EXPLOSIVE) {
            let Some(bomb) = play.actors.actor(h) else { continue };
            // schema: another actor's params read whole, as the C does.
            if bomb.params != 0 || bomb.parent.is_some() || bomb.killed {
                continue;
            }
            if bomb.id != ACTOR_EN_BOM {
                continue;
            }
            let d = bomb.world_pos - self.actor.world_pos;
            if d.x.abs() >= max_bomb_range.x.abs() || d.y.abs() >= max_bomb_range.y.abs() || d.z.abs() >= max_bomb_range.z.abs() {
                continue;
            }
            self.bomb = Some(h);
            max_bomb_range = bomb.world_pos;
            found_bomb = true;
        }
        found_bomb
    }

    /// `EnDodojr_TryEatBomb`: its bomb, free, within 30: the bomb's `parent` set to it.
    fn try_eat_bomb(&mut self, play: &mut PlayState) -> bool {
        let Some(h) = self.bomb else { return false };
        let me = play.cur_actor;
        let Some(bomb) = play.actors.actor_mut(h) else { return false };
        if bomb.parent.is_some() {
            false
        } else if vec3f_dist_xyz(self.actor.world_pos, bomb.world_pos) > 30.0 {
            false
        } else {
            bomb.parent = me;
            true
        }
    }

    /// `EnDodojr_UpdateCrawl`: its bomb gone or taken, it looks again; it turns (a tenth, at most
    /// 1000, at least 1) towards the bomb, else Link, or past 80 a point 80 off it by
    /// `angles[(home.x + home.y + home.z + state.frames / 30) % 12]`.
    fn update_crawl(&mut self, play: &mut PlayState) {
        let stale = match self.bomb {
            None => true,
            Some(h) => play.actors.actor(h).is_none_or(|b| b.killed || b.parent.is_some()),
        };
        if stale {
            self.check_nearby_bombs(play);
        }
        let mut crawl_target_pos = match self.bomb.and_then(|h| play.actors.actor(h)) {
            Some(b) => b.world_pos,
            None => play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default(),
        };
        if vec3f_dist_xyz(self.actor.world_pos, crawl_target_pos) > 80.0 {
            let h = self.actor.home_pos;
            // (s16)(home.x + home.y + home.z + play->state.frames / 30) % 12, then ABS.
            let sum = h.x + h.y + h.z + (play.state_frames / 30) as f32;
            let mut angle_index = (sum as i32 as i16) % 12;
            angle_index = angle_index.abs();
            let a = CRAWL_ANGLES[angle_index as usize];
            crawl_target_pos.x += 80.0 * a.sin();
            crawl_target_pos.z += 80.0 * a.cos();
        }
        let yaw = vec3f_yaw(self.actor.world_pos, crawl_target_pos);
        smooth_step_to_s(&mut self.actor.world_rot.y, yaw, 10, 1000, 1);
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    /// `EnDodojr_IsPlayerWithinAttackRange`: Link within 40 across.
    fn is_player_within_attack_range(&self) -> bool {
        self.actor.xz_dist_to_player <= 40.0
    }

    /// `EnDodojr_SetupStandardDeathBounce`: `NA_SE_EN_DODO_M_DEAD`, not targetable, flipping.
    fn setup_standard_death_bounce(&mut self, play: &mut PlayState) {
        audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_DEAD);
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.setup_flip_bounce();
        self.action = Action::StandardDeathBounce;
    }

    /// `EnDodojr_CheckDamaged`: not while dying or eating a bomb. The hammer's shock wave
    /// (`actorCtx.unk_02`) kills it once it's up. A hit: its shadow on if it was buried or
    /// surfacing; damage with no reaction, the finishing blow and two frames' freeze; reaction 1
    /// (not stunned already), stunned: `NA_SE_EN_GOMA_JR_FREEZE`, blue 120 frames, flipping.
    fn check_damaged(&mut self, play: &mut PlayState) -> bool {
        use Action::*;
        if matches!(self.action, SwallowBomb | SwallowedBombDeathBounce | SwallowedBombDeathSequence | Despawn | StandardDeathBounce | DeathSequence | DropItem) {
            return false;
        }
        if play.actors.unk_02 != 0 {
            if self.action != WaitUnderground {
                if self.action == EmergeFromGround {
                    self.actor.shape_shadow_draw = oot_game::shadow::ShadowDraw::Circle;
                }
                self.setup_standard_death_bounce(play);
            }
            return false;
        }
        if self.collider.base.ac_flags & AC_HIT == 0 {
            return false;
        }
        self.collider.base.ac_flags &= !AC_HIT;
        if self.action == WaitUnderground || self.action == EmergeFromGround {
            self.actor.shape_shadow_draw = oot_game::shadow::ShadowDraw::Circle;
        }
        if self.actor.col_chk_info.damage_reaction == 0 && self.actor.col_chk_info.damage != 0 {
            enemy_start_finishing_blow(play, &self.actor);
            self.freeze_frame_timer = 2;
            self.action = WaitFreezeFrames;
            return true;
        }
        if self.actor.col_chk_info.damage_reaction == 1 && self.action != Stunned && self.action != StunnedBounce {
            audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
            self.stun_timer = 120;
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 200, COLORFILTER_BUFFLAG_OPA, 120);
            self.setup_flip_bounce();
            self.action = StunnedBounce;
        }
        false
    }

    /// `EnDodojr_UpdateCollider`: placed; not while buried or dropping: hurting while surfacing,
    /// crawling or jumping; hit while those and stunned; pushing always.
    fn update_collider(&mut self, play: &mut PlayState) {
        use Action::*;
        self.collider.update(&self.actor);
        if self.action != WaitUnderground && self.action != DropItem {
            if matches!(self.action, EmergeFromGround | CrawlTowardsTarget | JumpAttackBounce) {
                play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            }
            if matches!(self.action, EmergeFromGround | CrawlTowardsTarget | StunnedBounce | Stunned | JumpAttackBounce) {
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            }
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
    }

    /// `EnDodojr_WaitUnderground`: Link within 320 across and less than 40 below it: the crawl at
    /// 1.8, `NA_SE_EN_DODO_M_UP`, 60 down, targetable, nose-down (`rot.x` less 0x4000); the dust
    /// at its x and z, y `floorHeight` (@bug (game): 0, not read yet); surfacing.
    fn wait_underground(&mut self, play: &mut PlayState) {
        let player_y = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos.y).unwrap_or_default();
        if !(self.actor.xz_dist_to_player >= 320.0) {
            let dist = self.actor.world_pos.y - player_y;
            if !(dist >= 40.0) {
                self.change(self.anims.crawl.clone(), 1.8, 0.0, None, ANIMMODE_LOOP_INTERP, -10.0);
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_UP);
                self.actor.world_pos.y -= 60.0;
                self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
                self.actor.world_rot.x = self.actor.world_rot.x.wrapping_sub(0x4000);
                self.actor.shape_rot.x = self.actor.world_rot.x;
                self.dust_pos = self.actor.world_pos;
                // @bug (game) (the C's comment): floorHeight is always 0 at this point, so the
                // dust is consistently drawn at y=0.
                self.dust_pos.y = self.actor.floor_height;
                self.action = Action::EmergeFromGround;
            }
        }
    }

    /// `EnDodojr_EmergeFromGround`: tipping level (a quarter, 100 to 1000 a frame), rising with
    /// it (60 at -0x4000), dust; level: its shadow, crawling at 2.6.
    fn emerge_from_ground(&mut self, play: &mut PlayState) {
        smooth_step_to_s(&mut self.actor.shape_rot.x, 0, 4, 0x3E8, 0x64);
        let mut sp2c = self.actor.shape_rot.x as f32;
        sp2c /= 16384.0;
        self.actor.world_pos.y = self.actor.home_pos.y + (60.0 * sp2c);
        self.spawn_large_dust(play, 3);
        if sp2c == 0.0 {
            self.actor.shape_shadow_draw = oot_game::shadow::ShadowDraw::Circle;
            self.actor.world_rot.x = self.actor.shape_rot.x;
            self.actor.speed_xz = 2.6;
            self.action = Action::CrawlTowardsTarget;
        }
    }

    /// `EnDodojr_CrawlTowardsTarget`: its velocity, dust, `NA_SE_EN_DODO_M_MOVE` every 5 frames;
    /// a bomb within reach, eating it; else turning; Link within 40, `NA_SE_EN_DODO_M_CRY` and the
    /// jump; a wall, `NA_SE_EN_DODO_M_DOWN` and burrowing away.
    fn crawl_towards_target(&mut self, play: &mut PlayState) {
        self.actor.update_velocity();
        let p = self.actor.world_pos;
        self.spawn_small_dust(play, p);
        if decr(&mut self.crawl_sfx_timer) == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_MOVE);
            self.crawl_sfx_timer = 5;
        }
        if self.try_eat_bomb(play) {
            self.setup_eat_bomb(play);
            self.action = Action::EatBomb;
            return;
        }
        self.update_crawl(play);
        if self.is_player_within_attack_range() {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_CRY);
            self.setup_jump_attack_bounce();
            self.action = Action::JumpAttackBounce;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_DOWN);
            self.setup_despawn();
            self.action = Action::Despawn;
        }
    }

    /// `EnDodojr_EatBomb`: to frame 12 the bomb at its mouth, its fuse held (up one a frame);
    /// then `NA_SE_EN_DODO_K_DRINK`, the bomb gone, 24 frames to swallow.
    fn eat_bomb(&mut self, play: &mut PlayState) {
        let Some(h) = self.bomb else { return };
        if (self.skel.cur_frame as i16 - 8) < 4 {
            let head_pos = self.head_pos;
            if let Some(bomb) = play.actors.downcast_mut::<EnBom>(h) {
                bomb.timer = bomb.timer.wrapping_add(1);
                bomb.actor.world_pos = head_pos;
            }
        } else {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_DRINK);
            if let Some(b) = play.actors.actor_mut(h) {
                b.kill();
            }
            self.timer = 24;
            self.counter = 0;
            self.action = Action::SwallowBomb;
        }
    }

    /// `EnDodojr_SwallowBomb`: after 24 frames the blast inside, not targetable, flipping.
    fn swallow_bomb(&mut self, play: &mut PlayState) {
        if decr(&mut self.timer) == 0 {
            self.do_swallowed_bomb_effects(play);
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            self.setup_flip_bounce();
            self.action = Action::SwallowedBombDeathBounce;
        }
    }

    /// `EnDodojr_SwallowedBombDeathBounce`: swollen (`rootScale` 1.2 shrinking with the red
    /// filter's frames), bouncing; landed for good, lying 60 frames, seven flashes, dying.
    fn swallowed_bomb_death_bounce(&mut self, play: &mut PlayState) {
        // Scale up briefly to expand from the swallowed bomb exploding.
        self.root_scale = 1.2;
        self.root_scale *= self.actor.color_filter_timer as f32 / 8.0;
        self.actor.update_velocity();
        if self.update_bounces(play) {
            self.timer = 60;
            self.setup_swallowed_bomb_death_sequence();
            self.counter = 7;
            self.action = Action::SwallowedBombDeathSequence;
        }
    }

    /// `EnDodojr_StunnedBounce`: bouncing, turning to face yaw 0 (a quarter, 10 to 1000), the blue
    /// held to its stun; landed for good, lying stunned.
    fn stunned_bounce(&mut self, play: &mut PlayState) {
        self.actor.update_velocity();
        if self.update_bounces(play) {
            self.setup_swallowed_bomb_death_sequence();
            self.action = Action::Stunned;
        }
        smooth_step_to_s(&mut self.actor.shape_rot.y, 0, 4, 1000, 10);
        self.actor.world_rot.x = self.actor.shape_rot.x;
        self.actor.color_filter_timer = self.stun_timer as u8;
    }

    /// `EnDodojr_Stunned`: the stun down; its last 30 frames shaking 1.5 on x and z; out, crawling.
    fn stunned(&mut self) {
        if decr(&mut self.stun_timer) != 0 {
            if self.stun_timer < 30 {
                // Shake back and forth to indicate recovering from stun.
                if self.stun_timer & 1 != 0 {
                    self.actor.world_pos.x += 1.5;
                    self.actor.world_pos.z += 1.5;
                } else {
                    self.actor.world_pos.x -= 1.5;
                    self.actor.world_pos.z -= 1.5;
                }
            }
        } else {
            self.setup_crawl_towards_target();
            self.action = Action::CrawlTowardsTarget;
        }
    }

    /// `EnDodojr_JumpAttackBounce`: its hits sound as Link's body hit, bouncing; then crawling.
    fn jump_attack_bounce(&mut self, play: &mut PlayState) {
        self.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
        self.actor.update_velocity();
        if self.update_bounces(play) {
            self.setup_crawl_towards_target();
            self.action = Action::CrawlTowardsTarget;
        }
    }

    /// `EnDodojr_Despawn`: tipping nose-down (a quarter, 100 to 1000), sinking 60 over 30 frames,
    /// dust; gone.
    fn despawn(&mut self, play: &mut PlayState) {
        smooth_step_to_s(&mut self.actor.shape_rot.x, 0x4000, 4, 1000, 100);
        if decr(&mut self.timer) != 0 {
            let burrow_step = (30 - self.timer) as f32 / 30.0;
            self.actor.world_pos.y = self.actor.home_pos.y - (60.0 * burrow_step);
        } else {
            self.actor.kill();
        }
        self.spawn_large_dust(play, 3);
    }

    /// `EnDodojr_StandardDeathBounce`: bouncing, turning to yaw 0; landed for good, lying 60
    /// frames, seven flashes, dying.
    fn standard_death_bounce(&mut self, play: &mut PlayState) {
        self.actor.update_velocity();
        smooth_step_to_s(&mut self.actor.shape_rot.y, 0, 4, 1000, 10);
        self.actor.world_rot.x = self.actor.shape_rot.x;
        if self.update_bounces(play) {
            self.timer = 60;
            self.setup_swallowed_bomb_death_sequence();
            self.counter = 7;
            self.action = Action::DeathSequence;
        }
    }

    /// `EnDodojr_DeathSequence`: a red flash of `counter` frames each time the last ends, 7 to 1;
    /// then a bomb where it lies (`ACTOR_EN_BOM`, `BOMB_BODY`) with its fuse at 0, and 8 frames to
    /// its drop.
    fn death_sequence(&mut self, play: &mut PlayState) {
        if self.counter != 0 {
            if self.actor.color_filter_timer == 0 {
                self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 200, COLORFILTER_BUFFLAG_OPA, self.counter as u16);
                self.counter -= 1;
            }
        } else {
            match play.actor_spawn(ACTOR_EN_BOM, self.actor.world_pos, [0, 0, 0], BOMB_BODY) {
                Ok(h) => {
                    if let Some(bomb) = play.actors.downcast_mut::<EnBom>(h) {
                        bomb.timer = 0;
                    }
                }
                Err(e) => log::debug!("En_Dodojr: no bomb: {e:?}"),
            }
            self.timer = 8;
            self.action = Action::DropItem;
        }
    }

    /// `EnDodojr_DropItem`: after 8 frames a drop from table 4, gone.
    fn drop_item(&mut self, play: &mut PlayState) {
        if decr(&mut self.timer) == 0 {
            let pos = self.actor.world_pos;
            // COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_4, false), no actor.
            crate::en_item00::item_drop_collectible_random(play, None, pos, COLLECTIBLE_DROP_TABLE_4 * 16);
            self.actor.kill();
        }
    }

    /// `EnDodojr_WaitFreezeFrames`: two frames, then dying.
    fn wait_freeze_frames(&mut self, play: &mut PlayState) {
        if decr(&mut self.freeze_frame_timer) == 0 {
            self.setup_standard_death_bounce(play);
        }
    }

    /// `EnDodojr_Draw`'s condition: not buried, not dropping its item.
    fn drawn(&self) -> bool {
        self.action != Action::WaitUnderground && self.action != Action::DropItem
    }

    /// `SkelAnime_DrawOpa`'s matrices with `EnDodojr_OverrideLimbDraw`'s root swell: limb 1
    /// scaled (`rootScale × 0.5 + 1` across, `rootScale + 1` up) before its own transform.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], root_scale: f32) -> Vec<Mat4> {
        draw_opa_pose(
            skeleton,
            joints,
            |limb, _pos, _rot| {
                if limb == 1 {
                    LimbDraw::Default(Mat4::from_scale(Vec3::new((root_scale * 0.5) + 1.0, root_scale + 1.0, (root_scale * 0.5) + 1.0)))
                } else {
                    LimbDraw::Default(Mat4::IDENTITY)
                }
            },
            |_, _| {},
        )
    }
}

impl ActorImpl for EnDodojr {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnDodojr_Update`: the animation, `Actor_MoveXZGravity`, the hits, the floor and walls
    /// (once up: 18 up, 20 across), the action, the focus 10 up, the collider.
    fn update(&mut self, play: &mut PlayState) {
        self.skel.update();
        self.actor.move_forward();
        self.check_damaged(play);
        if self.action != Action::WaitUnderground {
            let (r, h) = (self.collider.dim.radius as f32, self.collider.dim.height as f32);
            self.actor.update_bg_check_info(&play.col, r, h, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        }
        match self.action {
            Action::WaitUnderground => self.wait_underground(play),
            Action::DropItem => self.drop_item(play),
            Action::EmergeFromGround => self.emerge_from_ground(play),
            Action::CrawlTowardsTarget => self.crawl_towards_target(play),
            Action::StunnedBounce => self.stunned_bounce(play),
            Action::JumpAttackBounce => self.jump_attack_bounce(play),
            Action::Stunned => self.stunned(),
            Action::SwallowBomb => self.swallow_bomb(play),
            Action::SwallowedBombDeathBounce => self.swallowed_bomb_death_bounce(play),
            // EnDodojr_SwallowedBombDeathSequence: EnDodojr_DeathSequence.
            Action::SwallowedBombDeathSequence => self.death_sequence(play),
            Action::StandardDeathBounce => self.standard_death_bounce(play),
            Action::Despawn => self.despawn(play),
            Action::DeathSequence => self.death_sequence(play),
            Action::WaitFreezeFrames => self.wait_freeze_frames(play),
            Action::EatBomb => self.eat_bomb(play),
        }
        self.actor.set_focus(10.0);
        self.update_collider(play);
    }

    /// `EnDodojr_Destroy`: the cylinder (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `EnDodojr_OverrideLimbDraw`'s effect on the actor: `headPos`, `D_809F7F64` on limb 4's
    /// parent's matrix (the matrix its override is called with), once per drawn game frame.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if !self.drawn() {
            return;
        }
        let Some(skeleton) = self.skeleton.clone() else { return };
        let model = actor_draw_matrix(&RenderState::of(&self.actor));
        let m = Self::pose(&skeleton, &self.skel.joint_table, self.root_scale);
        // Limb 4 (1-based): index 3; its override's matrix is its parent's.
        let parent = skeleton.parents.get(3).copied().flatten().map(|p| m[p as usize]).unwrap_or(Mat4::IDENTITY);
        self.head_pos = (model * parent).transform_point3(HEAD_OFFSET);
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.values = vec![self.root_scale];
        rs.switches = vec![self.drawn() as u32];
        rs
    }

    /// `EnDodojr_Draw`: not buried or dropping, `Gfx_SetupDL_25Opa` and the skeleton
    /// (`SkelAnime_DrawOpa`) with the root's swell.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let ([root_scale], [drawn]) = (rs.values.as_slice(), rs.switches.as_slice()) else { return };
        if *drawn == 0 {
            return;
        }
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let bones = Self::pose(skeleton, &joints.rot, *root_scale);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::mesh(OBJECT, SKEL)), transform: actor_draw_matrix(rs), bones, params: Default::default() });
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
