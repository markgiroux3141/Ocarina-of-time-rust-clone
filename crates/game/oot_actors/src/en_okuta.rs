//! `En_Okuta` (`ovl_En_Okuta/z_en_okuta.c`): the Octorok, which hides under water and comes up to
//! spit rocks at Link (GAME-10 milestone 2), and its rock: the same actor with params 0x10.
//! `object_okuta`'s `gOctorokSkel` (37 limbs). Zora's River places two for a child (params 0xFF00,
//! room 0), eight for an adult; Lake Hylia, Zora's Fountain (an adult's), a grotto and
//! Jabu-Jabu's Belly more.
//!
//! - **Under water** (`EnOkuta_WaitToAppear`, not drawn): its height held at the water's surface
//!   (`home.pos.y`, the surface over it, every frame); Link between 200 and 480 across, it
//!   **comes up** (`EnOkuta_Appear`): drawn, facing him, lock-on on, `gOctorokAppearAnim` (ten
//!   bubbles, `EffectSsBubble`), growing to its size over the first four frames, the jump's and
//!   landing's sounds (frames 2 and 12), a splash on frames 3 and 15 (`EffectSsGSplash` at the
//!   surface, 1300). Done, it hides again with Link under 160 across, else floats.
//! - **Floating** (`EnOkuta_WaitToShoot`, `gOctorokFloatAnim` looped, `NA_SE_EN_OCTAROCK_FLOAT`):
//!   turning to Link (`Math_SmoothStepToS`, a third, 0x71C at most, 0x38E at least); facing him
//!   within 0x38E, its count (`timer`: 2 after shooting, else 0, down one each loop) out and he
//!   less than 200 above it, it **shoots** (`EnOkuta_Shoot`: `gOctorokShootAnim`) `numShots`
//!   rocks in a row (params bits 8 to 15; 0 or 0xFF: one), each a jump of his height over it plus
//!   20 (at least 10; over 50 with a splash and the jump's sound at its start and a splash and
//!   landing at frame 13); the rock (`EnOkuta_SpawnProjectile`, frame 6) 25 ahead and 6 down,
//!   with a puff of dust 40 ahead (`func_8002829C`) and `NA_SE_EN_NUTS_THROW`. Link under 160
//!   across (or over 560 floating) and it **hides** (`EnOkuta_Hide`: `gOctorokHideAnim`, back to
//!   the surface, shrinking from frame 4, the sinking's sound at 2, a splash at 4; done, bubbles
//!   and `NA_SE_EN_OCTAROCK_BUBLE`, and under again).
//! - **Its head and snout** (`EnOkuta_UpdateHeadScale`, `EnOkuta_GetSnoutScale`) swell and squash
//!   with each animation (limbs 5 and 8, `EnOkuta_OverrideLimbDraw`); its cylinder's height
//!   follows the head (`(40 × headScale.y + 30) × scale`), and coming up or going under, its place
//!   and radius follow the root and the scale.
//! - **Hit** (`EnOkuta_ColliderCheck`): one hit of anything with damage or a reaction kills it
//!   (health 1; a Deku nut does nothing): the finishing blow's flash (`Enemy_StartFinishingBlow`),
//!   lock-on off. An ice arrow (reaction 3) **freezes** it (`EnOkuta_Freeze`: blue 80 frames,
//!   eight chunks of ice from frame 64 down, `EffectSsEnIce`), the rest flash it red 11 frames
//!   (`EnOkuta_WaitToDie`: `gOctorokHitAnim`, `NA_SE_EN_OCTAROCK_DEAD1`); then it **dies**
//!   (`EnOkuta_Die`: `gOctorokDieAnim` once, a splash at its frame 15, then a count of the frames
//!   since its end (`timer`): swelling to 1.5 times its size, squashing to 0.8 and back up, a puff
//!   of dust 40 up and `NA_SE_EN_OCTAROCK_DEAD2` at the fifth, its snout puffing (the animation's
//!   frame plus the count, 25 to 34); from the eleventh shrinking 0.0005 a frame to nothing:
//!   `NA_SE_EN_OCTAROCK_BUBLE`, a drop from table 7, twenty bubbles (`EffectSsDtBubble`, white in
//!   grey, 100 to 149 big, 25 frames), gone).
//! - **Out of water** (the surface gone or under its floor), alive, it's gone at once.
//! - **The rock** (params 0x10, `EnOkuta_ProjectileFly`; a prop, always updating, no lock-on, the
//!   circle shadow 18 at 1100 up): straight on at 10 for 30 frames, then falling (gravity -1),
//!   spinning on its face (`home.rot.z`, 0x1554 a frame); in water falling and slowing (0.1 a
//!   frame to 1). It hurts 8 (`AT_TYPE_ENEMY`, `ATELEM_SFX_HARD`). On a wall or the ground (but
//!   the polys projectiles go through: then put back where it was before the wall's push), off the
//!   world's floor, or touching anything (AT, AC or OC), it breaks (`EffectSsHahen` bursts of its
//!   own list, `NA_SE_EN_OCTAROCK_ROCK`), unless it bounced off Link's Deku Shield (or an adult's
//!   Hylian Shield): then it flies back the way the shield faces as Link's attack
//!   (`AT_TYPE_PLAYER`, `DMG_DEKU_STICK`), another 30 frames. 330 frames out, it's gone.
//! - Nothing of it updates while Link talks, is dead or in a cutscene's hold (`PLAYER_STATE1_28`,
//!   `_29`).
//!
//! The whole overlay is ported. Its draw (`EnOkuta_Draw`) is `gOctorokSkel` (`SkelAnime_DrawOpa`,
//! limbs 5 and 8 scaled first) while drawn (`actor.draw` is `NULL` under water), or the rock's
//! `gOctorokProjectileDL` facing the camera (`billboardMtxF`), turned by `home.rot.z`. Its ripples
//! (`EnOkuta_SpawnRipple`: every 7th frame while drawn, at the surface under it, 250 to 650; not
//! while it's over 50 up shooting) come after its colliders.
//!
//! Params: `type` bits 0 to 7 (0 the Octorok; any other value the rock, spawned with 0x10, which
//! alone hurts; `params &= 0xFF` at init); `numShots` bits 8 to 15 (the rocks it shoots in a row;
//! 0 and 0xFF one; placed 0xFF, or 0 by Jabu-Jabu's unused cutscene layer and the fountain's
//! unused one).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::BGCHECK_Y_MIN;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, approach_s, smooth_step_to_s, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_PROP, ActorImpl, ActorProfile, PLAYER_STATE1_28, PLAYER_STATE1_29, PLAYER_STATE1_DEAD, PLAYER_STATE1_TALKING, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EN_NUTS_THROW, NA_SE_EN_OCTAROCK_ROCK};
use oot_game::collision_check::*;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};
use oot_game::surface::SurfaceType;

/// `ACTOR_EN_OKUTA` (`actor_table.h`: 0x000E).
pub const ACTOR_EN_OKUTA: i16 = 0x000E;
/// `OBJECT_OKUTA` (`object_table.h`: 0x0007).
pub const OBJECT: &str = "object_okuta";
pub const OBJECT_OKUTA: i16 = 0x0007;
/// `gOctorokSkel` (`object_okuta.xml`): 37 limbs (`SkelAnime_Init`'s 38 with the root's).
pub const SKEL: &str = "gOctorokSkel";
/// `gOctorokProjectileDL`: the rock.
pub const PROJECTILE_DL: &str = "gOctorokProjectileDL";

/// The animations (`object_okuta.xml`).
pub const ANIM_APPEAR: &str = "gOctorokAppearAnim";
pub const ANIM_HIDE: &str = "gOctorokHideAnim";
pub const ANIM_FLOAT: &str = "gOctorokFloatAnim";
pub const ANIM_SHOOT: &str = "gOctorokShootAnim";
pub const ANIM_HIT: &str = "gOctorokHitAnim";
pub const ANIM_DIE: &str = "gOctorokDieAnim";

/// `En_Okuta_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, `OBJECT_OKUTA`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_OKUTA, name: "En_Okuta", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE, object: OBJECT, schema: &SCHEMA };

/// `params & 0xFF` (`params &= 0xFF` at init): 0 the Octorok, any other the rock.
pub const TYPE: Field = Field::params("type", 0, 8, Kind::Named(&[(0, "octorok"), (0x10, "projectile")]), "the Octorok (0) or its rock (any other; spawned as 0x10, which alone hurts)");
/// `PARAMS_GET_U(params, 8, 8)`: `numShots`.
pub const NUM_SHOTS: Field = Field::params("numShots", 8, 8, Kind::Range(0, 255), "the rocks it shoots in a row (0 and 0xFF: one)");

/// En_Okuta's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE, NUM_SHOTS], flags: &[], todo: false, objects: &[], texts: &[] };

/// The two types (`params`: the Octorok's 0, the rock `EnOkuta_SpawnProjectile` spawns, 0x10).
pub const TYPE_OCTOROK: i16 = 0;
pub const TYPE_PROJECTILE: i16 = 0x10;

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_OCTAROCK_FLOAT` (0x38C1).
pub const NA_SE_EN_OCTAROCK_FLOAT: u16 = 0x38C1;
/// `NA_SE_EN_OCTAROCK_JUMP` (0x38C2).
pub const NA_SE_EN_OCTAROCK_JUMP: u16 = 0x38C2;
/// `NA_SE_EN_OCTAROCK_LAND` (0x38C3).
pub const NA_SE_EN_OCTAROCK_LAND: u16 = 0x38C3;
/// `NA_SE_EN_OCTAROCK_SINK` (0x38C4).
pub const NA_SE_EN_OCTAROCK_SINK: u16 = 0x38C4;
/// `NA_SE_EN_OCTAROCK_BUBLE` (0x38C5).
pub const NA_SE_EN_OCTAROCK_BUBLE: u16 = 0x38C5;
/// `NA_SE_EN_OCTAROCK_DEAD1` (0x38C6).
pub const NA_SE_EN_OCTAROCK_DEAD1: u16 = 0x38C6;
/// `NA_SE_EN_OCTAROCK_DEAD2` (0x38C7).
pub const NA_SE_EN_OCTAROCK_DEAD2: u16 = 0x38C7;

/// `NAVI_ENEMY_OCTOROK` (`actor.h`: 0x42).
pub const NAVI_ENEMY_OCTOROK: u8 = 0x42;
/// `PLAYER_SHIELD_DEKU`, `PLAYER_SHIELD_HYLIAN` (`player.h`: 0x01, 0x02).
const PLAYER_SHIELD_DEKU: u8 = 0x01;
const PLAYER_SHIELD_HYLIAN: u8 = 0x02;
/// `DMG_DEKU_STICK` (`collision_check.h`: 1 << 1).
const DMG_DEKU_STICK: u32 = 1 << 1;
/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_7, false)` (`z_en_item00.h`: the table
/// × 16).
const DROP_TABLE_7: i16 = 7 * 16;

/// `sProjectileColliderInit`: the rock, hurting 8 (`ATELEM_SFX_HARD`), hit by Player's
/// `0xFFCFFFFF`, pushing as type 2; 13 by 20.
pub const PROJECTILE_COLLIDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_HARD,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 13, height: 20, y_shift: 0, pos: [0; 3] },
};

/// `sOctorockColliderInit`: its body, hit by Player's `0xFFCFFFFF`, pushing as type 1; 20 by 40,
/// 30 down.
pub const OCTOROK_COLLIDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK1,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 40, y_shift: -30, pos: [0; 3] },
};

/// `sColChkInfoInit`: health 1, 15 by 60, mass 100.
pub const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 1, cyl_radius: 15, cyl_height: 60, mass: 100 };

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x0), // Deku nut
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
        dmg_entry(2, 0x0), // Fire arrow
        dmg_entry(4, 0x3), // Ice arrow
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

/// The limbs `EnOkuta_OverrideLimbDraw` scales (1-based): the head and the snout.
pub const LIMB_HEAD: usize = 5;
pub const LIMB_SNOUT: usize = 8;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitToAppear,
    Appear,
    Hide,
    WaitToShoot,
    Shoot,
    WaitToDie,
    Die,
    Freeze,
    ProjectileFly,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::WaitToAppear => "EnOkuta_WaitToAppear",
            Action::Appear => "EnOkuta_Appear",
            Action::Hide => "EnOkuta_Hide",
            Action::WaitToShoot => "EnOkuta_WaitToShoot",
            Action::Shoot => "EnOkuta_Shoot",
            Action::WaitToDie => "EnOkuta_WaitToDie",
            Action::Die => "EnOkuta_Die",
            Action::Freeze => "EnOkuta_Freeze",
            Action::ProjectileFly => "EnOkuta_ProjectileFly",
        }
    }
}

struct Anims {
    appear: Anim,
    hide: Anim,
    float: Anim,
    shoot: Anim,
    hit: Anim,
    die: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Okuta: {e:#}")).ok();
        Some(Anims { appear: get(ANIM_APPEAR)?, hide: get(ANIM_HIDE)?, float: get(ANIM_FLOAT)?, shoot: get(ANIM_SHOOT)?, hit: get(ANIM_HIT)?, die: get(ANIM_DIE)? })
    }
}

pub struct EnOkuta {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    pub action: Action,
    pub timer: i16,
    pub num_shots: i16,
    pub jump_height: f32,
    pub head_scale: Vec3,
    pub collider: ColliderCylinder,
    /// `actor.draw` set (`EnOkuta_Draw`), not `NULL`.
    pub drawn: bool,
}

impl EnOkuta {
    /// `EnOkuta_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: naviEnemyId NAVI_ENEMY_OCTOROK, lockOnArrowOffset 6500.
        actor.navi_enemy_id = NAVI_ENEMY_OCTOROK;
        actor.target_arrow_offset = 6500.0;
        let num_shots = NUM_SHOTS.get(actor.params);
        // schema: params &= 0xFF, the type alone kept
        actor.params &= 0xFF;
        let anims = Anims::load(play);
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Okuta: {e:#}")).ok());
        let mut this = EnOkuta {
            actor,
            skel: SkelAnimeStd::init_flex(37, None),
            skeleton: None,
            anims: None,
            action: Action::WaitToAppear,
            timer: 0,
            num_shots,
            jump_height: 0.0,
            head_scale: Vec3::ZERO,
            collider: ColliderCylinder::new(&OCTOROK_COLLIDER_INIT),
            drawn: true,
        };
        if TYPE.get(this.actor.params) == TYPE_OCTOROK {
            // SkelAnime_Init(.., &gOctorokSkel, &gOctorokAppearAnim, .., 38).
            this.skel = SkelAnimeStd::init_flex(skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(37), anims.as_ref().map(|a| a.appear.clone()));
            this.skeleton = skeleton;
            this.anims = anims;
            this.actor.col_chk_info.set_info(Some(&S_DAMAGE_TABLE), &COL_CHK_INFO_INIT);
            if this.num_shots == 0xFF || this.num_shots == 0 {
                this.num_shots = 1;
            }
            // BgCheck_EntityRaycastDown4.
            let (floor_height, floor_poly) = play.col.entity_raycast_down(this.actor.world_pos);
            this.actor.floor_height = floor_height;
            this.actor.floor_poly = floor_poly;
            let p = this.actor.world_pos;
            match play.col.water_surface(p.x, p.z, play.col.water_room) {
                Some(y_surface) if y_surface > this.actor.floor_height => this.actor.home_pos.y = y_surface,
                _ => this.actor.kill(),
            }
            this.setup_wait_to_appear();
        } else {
            this.actor.shape_init(1100.0, oot_game::shadow::ShadowDraw::Circle, 18.0);
            this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            this.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            this.collider = ColliderCylinder::new(&PROJECTILE_COLLIDER_INIT);
            // Actor_ChangeCategory(.., ACTORCAT_PROP): the arena adds it to the category the
            // init leaves (at its list's head, where the C moves it).
            this.actor.category = ACTORCAT_PROP;
            this.timer = 30;
            this.actor.shape_rot.y = 0;
            this.action = Action::ProjectileFly;
            this.actor.speed_xz = 10.0;
        }
        Box::new(this)
    }

    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `Animation_PlayOnce`.
    fn play_once(&mut self, f: impl Fn(&Anims) -> &Anim) {
        if let Some(a) = self.anim(f) {
            self.skel.play_once(a);
        }
    }

    /// `Animation_PlayLoop`.
    fn play_loop(&mut self, f: impl Fn(&Anims) -> &Anim) {
        if let Some(a) = self.anim(f) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
        }
    }

    /// `Animation_MorphToPlayOnce`.
    fn morph_to_play_once(&mut self, f: impl Fn(&Anims) -> &Anim, morph_frames: f32) {
        if let Some(a) = self.anim(f) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, morph_frames);
        }
    }

    /// `Actor_SetScale`.
    fn set_scale(&mut self, scale: f32) {
        self.actor.scale = Vec3::splat(scale);
    }

    /// `EnOkuta_SpawnBubbles`: ten bubbles at it (10 up or down, 30 across, 0.25).
    fn spawn_bubbles(&self, play: &mut PlayState) {
        let pos = self.actor.world_pos;
        for _ in 0..10 {
            play.with_ss(|s| s.bubble_spawn(pos, -10.0, 10.0, 30.0, 0.25));
        }
    }

    /// `EnOkuta_SpawnDust`: `func_8002829C` with no acceleration, white in grey, 400 big.
    fn spawn_dust(pos: Vec3, velocity: Vec3, scale_step: i16, play: &mut PlayState) {
        play.with_ss(|s| s.func_8002829c(pos, velocity, Vec3::ZERO, [255, 255, 255, 255], [150, 150, 150, 255], 0x190, scale_step));
    }

    /// `EnOkuta_SpawnSplash`: at the surface over it (`home.pos`), the default colours, 1300.
    fn spawn_splash(&self, play: &mut PlayState) {
        let pos = self.actor.home_pos;
        play.with_ss(|s| s.g_splash_spawn(pos, None, None, 0, 1300));
    }

    /// `EnOkuta_SpawnRipple`: every 7th frame at the surface under it (not over 50 up while
    /// shooting), 250 to 650.
    fn spawn_ripple(&self, play: &mut PlayState) {
        let pos = Vec3::new(self.actor.world_pos.x, self.actor.home_pos.y, self.actor.world_pos.z);
        if play.gameplay_frames % 7 == 0 && (self.action != Action::Shoot || (self.actor.world_pos.y - self.actor.home_pos.y) < 50.0) {
            play.with_ss(|s| s.g_ripple_spawn(pos, 250, 650, 0));
        }
    }

    /// `EnOkuta_SetupWaitToAppear`: not drawn, lock-on off, at the surface.
    fn setup_wait_to_appear(&mut self) {
        self.drawn = false;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::WaitToAppear;
        self.actor.world_pos.y = self.actor.home_pos.y;
    }

    /// `EnOkuta_SetupAppear`: drawn, facing Link, lock-on on, `gOctorokAppearAnim`, bubbles.
    fn setup_appear(&mut self, play: &mut PlayState) {
        self.drawn = true;
        self.actor.shape_rot.y = self.actor.yaw_towards_player;
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        self.play_once(|a| &a.appear);
        self.spawn_bubbles(play);
        self.action = Action::Appear;
    }

    /// `EnOkuta_SetupHide`.
    fn setup_hide(&mut self) {
        self.play_once(|a| &a.hide);
        self.action = Action::Hide;
    }

    /// `EnOkuta_SetupWaitToShoot`: `gOctorokFloatAnim` looped; two loops to wait after shooting.
    fn setup_wait_to_shoot(&mut self) {
        self.play_loop(|a| &a.float);
        self.timer = if self.action == Action::Shoot { 2 } else { 0 };
        self.action = Action::WaitToShoot;
    }

    /// `EnOkuta_SetupShoot`: `gOctorokShootAnim`; the count of rocks on a first shot; the jump's
    /// height (Link's over it plus 20, at least 10; over 50 a splash and the jump's sound).
    fn setup_shoot(&mut self, play: &mut PlayState) {
        self.play_once(|a| &a.shoot);
        if self.action != Action::Shoot {
            self.timer = self.num_shots;
        }
        self.jump_height = self.actor.y_dist_to_player + 20.0;
        self.jump_height = self.jump_height.max(10.0);
        if self.jump_height > 50.0 {
            self.spawn_splash(play);
        }
        if self.jump_height > 50.0 {
            audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_JUMP);
        }
        self.action = Action::Shoot;
    }

    /// `EnOkuta_SetupWaitToDie`: `gOctorokHitAnim` (a 5-frame morph), red 11 frames, its hit
    /// forgotten, its full size, `NA_SE_EN_OCTAROCK_DEAD1`.
    fn setup_wait_to_die(&mut self, play: &mut PlayState) {
        self.morph_to_play_once(|a| &a.hit, -5.0);
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 11);
        self.collider.base.ac_flags &= !AC_HIT;
        self.set_scale(0.01);
        audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_DEAD1);
        self.action = Action::WaitToDie;
    }

    /// `EnOkuta_SetupDie`: `gOctorokDieAnim` (a 3-frame morph), its loops counted from 0.
    fn setup_die(&mut self) {
        self.morph_to_play_once(|a| &a.die, -3.0);
        self.timer = 0;
        self.action = Action::Die;
    }

    /// `EnOkuta_SetupFreeze`: blue 80 frames.
    fn setup_freeze(&mut self) {
        self.timer = 80;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, 80);
        self.action = Action::Freeze;
    }

    /// `EnOkuta_SpawnProjectile`: the rock (params 0x10) 25 ahead and 6 down, turned as it is;
    /// spawned, a puff of dust 40 ahead drifting on at 1.5; `NA_SE_EN_NUTS_THROW` either way.
    fn spawn_projectile(&mut self, play: &mut PlayState) {
        let r = self.actor.shape_rot;
        let (sin_y, cos_y) = (eng_math::sin_s(r.y), eng_math::cos_s(r.y));
        let w = self.actor.world_pos;
        let pos = Vec3::new(w.x + (25.0 * sin_y), w.y - 6.0, w.z + (25.0 * cos_y));
        match play.actor_spawn(ACTOR_EN_OKUTA, pos, [r.x, r.y, r.z], TYPE_PROJECTILE) {
            Ok(_) => {
                let pos = Vec3::new(w.x + (40.0 * sin_y), w.y, w.z + (40.0 * cos_y));
                let velocity = Vec3::new(1.5 * sin_y, 0.0, 1.5 * cos_y);
                Self::spawn_dust(pos, velocity, 20, play);
            }
            Err(e) => log::debug!("En_Okuta: no rock: {e:?}"),
        }
        audio_play_actor_sfx2(play, NA_SE_EN_NUTS_THROW);
    }

    /// `EnOkuta_WaitToAppear`: at the surface; Link between 200 and 480 across, it comes up.
    fn wait_to_appear(&mut self, play: &mut PlayState) {
        self.actor.world_pos.y = self.actor.home_pos.y;
        if self.actor.xz_dist_to_player < 480.0 && self.actor.xz_dist_to_player > 200.0 {
            self.setup_appear(play);
        }
    }

    /// `EnOkuta_Appear`: see the module's docs.
    fn appear(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            if self.actor.xz_dist_to_player < 160.0 {
                self.setup_hide();
            } else {
                self.setup_wait_to_shoot();
            }
        } else if self.skel.cur_frame <= 4.0 {
            self.set_scale(self.skel.cur_frame * 0.25 * 0.01);
        } else if self.skel.on_frame(5.0) {
            self.set_scale(0.01);
        }
        if self.skel.on_frame(2.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_JUMP);
        }
        if self.skel.on_frame(12.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_LAND);
        }
        if self.skel.on_frame(3.0) || self.skel.on_frame(15.0) {
            self.spawn_splash(play);
        }
    }

    /// `EnOkuta_Hide`: see the module's docs.
    fn hide(&mut self, play: &mut PlayState) {
        approach_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 0.5, 30.0);
        if self.skel.update() {
            audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_BUBLE);
            self.spawn_bubbles(play);
            self.setup_wait_to_appear();
        } else if self.skel.cur_frame >= 4.0 {
            self.set_scale((6.0 - self.skel.cur_frame) * 0.5 * 0.01);
        }
        if self.skel.on_frame(2.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_SINK);
        }
        if self.skel.on_frame(4.0) {
            self.spawn_splash(play);
        }
    }

    /// `EnOkuta_WaitToShoot`: see the module's docs.
    fn wait_to_shoot(&mut self, play: &mut PlayState) {
        self.actor.world_pos.y = self.actor.home_pos.y;
        self.skel.update();
        if self.skel.on_frame(0.0) && self.timer != 0 {
            self.timer -= 1;
        }
        if self.skel.on_frame(0.5) {
            audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_FLOAT);
        }
        if self.actor.xz_dist_to_player < 160.0 || self.actor.xz_dist_to_player > 560.0 {
            self.setup_hide();
        } else {
            let yaw_diff = smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 3, 0x71C, 0x38E);
            let abs_yaw_diff = (yaw_diff as i32).abs();
            if abs_yaw_diff < 0x38E && self.timer == 0 && self.actor.y_dist_to_player < 200.0 {
                self.setup_shoot(play);
            }
        }
    }

    /// `EnOkuta_Shoot`: see the module's docs.
    fn shoot(&mut self, play: &mut PlayState) {
        approach_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 3, 0x71C);
        if self.skel.update() {
            if self.timer != 0 {
                self.timer -= 1;
            }
            if self.timer == 0 {
                self.setup_wait_to_shoot();
            } else {
                self.setup_shoot(play);
            }
        } else {
            let cur_frame = self.skel.cur_frame;
            if cur_frame < 13.0 {
                // sinf((0.08333f * M_PI) * curFrame): M_PI a float (`libc/math.h`).
                self.actor.world_pos.y = (((0.08333f32 * std::f32::consts::PI) * cur_frame).sin() * self.jump_height) + self.actor.home_pos.y;
            }
            if self.skel.on_frame(6.0) {
                self.spawn_projectile(play);
            }
            if self.jump_height > 50.0 && self.skel.on_frame(13.0) {
                self.spawn_splash(play);
            }
            if self.jump_height > 50.0 && self.skel.on_frame(13.0) {
                audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_LAND);
            }
        }
        if self.actor.xz_dist_to_player < 160.0 {
            self.setup_hide();
        }
    }

    /// `EnOkuta_WaitToDie`: the hit's animation, then dying; back to the surface (a half, 5 at
    /// most).
    fn wait_to_die(&mut self) {
        if self.skel.update() {
            self.setup_die();
        }
        approach_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 0.5, 5.0);
    }

    /// `EnOkuta_Die`: see the module's docs.
    fn die(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.timer += 1;
        }
        approach_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 0.5, 5.0);
        if self.timer == 5 {
            let w = self.actor.world_pos;
            let pos = Vec3::new(w.x, w.y + 40.0, w.z);
            Self::spawn_dust(pos, Vec3::new(0.0, -0.5, 0.0), -0x14, play);
            audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_DEAD2);
        }
        if self.skel.on_frame(15.0) {
            self.spawn_splash(play);
            audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_LAND);
        }
        let t = self.timer as f32;
        if self.timer < 3 {
            self.set_scale(((t * 0.25) + 1.0) * 0.01);
        } else if self.timer < 6 {
            self.set_scale((1.5 - ((t - 2.0) * 0.2333)) * 0.01);
        } else if self.timer < 11 {
            self.set_scale((((t - 5.0) * 0.04) + 0.8) * 0.01);
        } else {
            if step_to_f(&mut self.actor.scale.x, 0.0, 0.0005) {
                let pos = self.actor.world_pos;
                play.sfx_source_play_sfx_at_fixed_world_pos(pos, 30, NA_SE_EN_OCTAROCK_BUBLE);
                crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, DROP_TABLE_7);
                for _ in 0..20 {
                    let vx = (play.rand.zero_one() - 0.5) * 7.0;
                    let vy = play.rand.zero_one() * 7.0;
                    let vz = (play.rand.zero_one() - 0.5) * 7.0;
                    let scale = play.rand.s16_offset(100, 50);
                    let velocity = Vec3::new(vx, vy, vz);
                    play.with_ss(|s| s.dt_bubble_spawn_custom_color(pos, velocity, Vec3::new(0.0, -0.5, 0.0), [255, 255, 255, 255], [150, 150, 150, 0], scale, 25, 0));
                }
                self.actor.kill();
            }
            self.actor.scale.y = self.actor.scale.x;
            self.actor.scale.z = self.actor.scale.x;
        }
    }

    /// `EnOkuta_Freeze`: 80 frames, then dying; from frame 64 down on odd frames a chunk of ice
    /// (eight: up its side, the four corners 10 out), back to the surface.
    fn freeze(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.setup_die();
        }
        if self.timer >= 64 && self.timer & 1 != 0 {
            let pos_param = (self.timer - 64) >> 1;
            let w = self.actor.world_pos;
            let pos = Vec3::new(
                w.x + if pos_param & 2 != 0 { 10.0 } else { -10.0 },
                (w.y - 32.0) + (8.0 * (8 - pos_param) as f32),
                w.z + if pos_param & 1 != 0 { 10.0 } else { -10.0 },
            );
            let scale = (play.rand.zero_one() * 0.2) + 1.9;
            let r = self.actor.shape_rot;
            let me = play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: w, shape_rot: [r.x, r.y, r.z] });
            play.effect_ss_en_ice_spawn_flying_vec3f(me, pos, [150, 150, 150, 250], [235, 245, 255], scale);
        }
        approach_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 0.5, 5.0);
    }

    /// `EnOkuta_ProjectileFly`: see the module's docs.
    fn projectile_fly(&mut self, play: &mut PlayState) {
        let player = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| (p.current_shield(), p.adult(), p.shield_mf()));
        self.timer = self.timer.wrapping_sub(1);
        if self.timer == 0 {
            self.actor.gravity = -1.0;
        }
        self.actor.home_rot.z = self.actor.home_rot.z.wrapping_add(0x1554);
        if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 {
            self.actor.gravity = -1.0;
            self.actor.speed_xz -= 0.1;
            self.actor.speed_xz = self.actor.speed_xz.max(1.0);
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0
            || self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0
            || self.collider.base.at_flags & AT_HIT != 0
            || self.collider.base.ac_flags & AC_HIT != 0
            || self.collider.base.oc_flags1 & OC1_HIT != 0
            || self.actor.floor_height == BGCHECK_Y_MIN
        {
            if let Some((shield, adult, shield_mf)) = player
                && (shield == PLAYER_SHIELD_DEKU || (shield == PLAYER_SHIELD_HYLIAN && adult))
                && self.collider.base.at_flags & AT_HIT != 0
                && self.collider.base.at_flags & AT_TYPE_ENEMY != 0
                && self.collider.base.at_flags & AT_BOUNCED != 0
            {
                self.collider.base.at_flags &= !(AT_HIT | AT_BOUNCED | AT_TYPE_ENEMY);
                self.collider.base.at_flags |= AT_TYPE_PLAYER;
                self.collider.info.at_dmg_info.dmg_flags = DMG_DEKU_STICK;
                let shield_rot = shield_mf.to_yxz_rot_s(false);
                self.actor.world_rot.y = shield_rot[1].wrapping_add(-0x8000i16);
                self.timer = 30;
            } else {
                let w = self.actor.world_pos;
                let pos = Vec3::new(w.x, w.y + 11.0, w.z);
                play.with_ss(|s| s.hahen_spawn_burst(pos, 6.0, 0, 1, 2, 15, OBJECT_OKUTA, 10, Some((OBJECT, PROJECTILE_DL))));
                play.sfx_source_play_sfx_at_fixed_world_pos(w, 20, NA_SE_EN_OCTAROCK_ROCK);
                self.actor.kill();
            }
        } else if self.timer == -300 {
            self.actor.kill();
        }
    }

    /// `EnOkuta_UpdateHeadScale`: the head's swelling by the action and its animation's frame.
    fn update_head_scale(&mut self) {
        let cur_frame = self.skel.cur_frame;
        let h = &mut self.head_scale;
        match self.action {
            Action::Appear => {
                if cur_frame < 8.0 {
                    *h = Vec3::ONE;
                } else if cur_frame < 10.0 {
                    h.x = 1.0;
                    h.z = 1.0;
                    h.y = ((cur_frame - 7.0) * 0.4) + 1.0;
                } else if cur_frame < 14.0 {
                    h.x = ((cur_frame - 9.0) * 0.075) + 1.0;
                    h.z = h.x;
                    h.y = 1.8 - ((cur_frame - 9.0) * 0.25);
                } else {
                    h.x = 1.3 - ((cur_frame - 13.0) * 0.05);
                    h.z = h.x;
                    h.y = ((cur_frame - 13.0) * 0.0333) + 0.8;
                }
            }
            Action::Hide => {
                if cur_frame < 3.0 {
                    h.y = 1.0;
                } else if cur_frame < 4.0 {
                    h.y = (cur_frame - 2.0) + 1.0;
                } else {
                    h.y = 2.0 - ((cur_frame - 3.0) * 0.333);
                }
                h.x = 1.0;
                h.z = 1.0;
            }
            Action::Shoot => {
                if cur_frame < 5.0 {
                    *h = Vec3::splat((cur_frame * 0.125) + 1.0);
                } else if cur_frame < 7.0 {
                    *h = Vec3::splat(1.5 - ((cur_frame - 4.0) * 0.35));
                } else if cur_frame < 17.0 {
                    h.x = ((cur_frame - 6.0) * 0.05) + 0.8;
                    h.z = h.x;
                    h.y = 0.8;
                } else {
                    h.x = 1.3 - ((cur_frame - 16.0) * 0.1);
                    h.z = h.x;
                    h.y = ((cur_frame - 16.0) * 0.0666) + 0.8;
                }
            }
            Action::WaitToShoot => {
                h.x = 1.0;
                h.z = 1.0;
                // sinf((M_PI / 16) * curFrame): M_PI a float.
                h.y = (((std::f32::consts::PI / 16.0) * cur_frame).sin() * 0.2) + 1.0;
            }
            _ => *h = Vec3::ONE,
        }
    }

    /// `EnOkuta_ColliderCheck`: a hit with damage or a reaction kills it: the finishing blow, no
    /// health, lock-on off; frozen by reaction 3, else the hit's animation.
    fn collider_check(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            let elem = self.collider.info;
            self.actor.set_drop_flag(&elem, true);
            if self.actor.col_chk_info.damage_reaction != 0 || self.actor.col_chk_info.damage != 0 {
                enemy_start_finishing_blow(play, &self.actor);
                self.actor.col_chk_info.health = 0;
                self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                if self.actor.col_chk_info.damage_reaction == 3 {
                    self.setup_freeze();
                } else {
                    self.setup_wait_to_die(play);
                }
            }
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::WaitToAppear => self.wait_to_appear(play),
            Action::Appear => self.appear(play),
            Action::Hide => self.hide(play),
            Action::WaitToShoot => self.wait_to_shoot(play),
            Action::Shoot => self.shoot(play),
            Action::WaitToDie => self.wait_to_die(),
            Action::Die => self.die(play),
            Action::Freeze => self.freeze(play),
            Action::ProjectileFly => self.projectile_fly(play),
        }
    }

    /// `EnOkuta_GetSnoutScale`: the snout's (limb 8) by the action at `cur_frame` (the dying one's
    /// plus its loops), or none.
    pub fn snout_scale(action: Action, cur_frame: f32) -> Option<Vec3> {
        match action {
            Action::WaitToShoot => {
                // sinf((M_PI / 16) * curFrame).
                Some(Vec3::new(1.0, (((std::f32::consts::PI / 16.0) * cur_frame).sin() * 0.4) + 1.0, 1.0))
            }
            Action::Shoot => {
                if cur_frame < 5.0 {
                    let yz = (cur_frame * 0.25) + 1.0;
                    Some(Vec3::new(1.0, yz, yz))
                } else if cur_frame < 7.0 {
                    let yz = 2.0 - (cur_frame - 4.0) * 0.5;
                    Some(Vec3::new((cur_frame - 4.0) * 0.5 + 1.0, yz, yz))
                } else {
                    Some(Vec3::new(2.0 - ((cur_frame - 6.0) * 0.0769), 1.0, 1.0))
                }
            }
            Action::Die => {
                if cur_frame >= 35.0 || cur_frame < 25.0 {
                    return None;
                }
                if cur_frame < 27.0 {
                    let yz = ((cur_frame - 24.0) * 0.5) + 1.0;
                    Some(Vec3::new(1.0, yz, yz))
                } else if cur_frame < 30.0 {
                    let yz = 2.0 - (cur_frame - 26.0) * 0.333;
                    Some(Vec3::new((cur_frame - 26.0) * 0.333 + 1.0, yz, yz))
                } else {
                    Some(Vec3::new(2.0 - ((cur_frame - 29.0) * 0.2), 1.0, 1.0))
                }
            }
            _ => None,
        }
    }

    /// `EnOkuta_OverrideLimbDraw`'s scales: the head's (limb 5, when it isn't 1) and the snout's
    /// (limb 8).
    pub fn limb_scales(&self) -> (Option<Vec3>, Option<Vec3>) {
        let mut cur_frame = self.skel.cur_frame;
        if self.action == Action::Die {
            cur_frame += self.timer as f32;
        }
        let head = (self.head_scale != Vec3::ONE).then_some(self.head_scale);
        (head, Self::snout_scale(self.action, cur_frame))
    }
}

/// `render_state`'s layout: switches drawn, the rock, the head scaled, the snout scaled; values the
/// head's scale, the snout's (x, y, z each); angles `home.rot.z` (the rock's spin).
mod rs {
    pub const HEAD: usize = 0;
    pub const SNOUT: usize = 3;
}

impl ActorImpl for EnOkuta {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnOkuta_Update`: see the module's docs. Nothing while Link talks, is dead or held.
    fn update(&mut self, play: &mut PlayState) {
        let state1 = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.state_flags1()).unwrap_or(0);
        if state1 & (PLAYER_STATE1_TALKING | PLAYER_STATE1_DEAD | PLAYER_STATE1_28 | PLAYER_STATE1_29) != 0 {
            return;
        }
        let octorok = TYPE.get(self.actor.params) == TYPE_OCTOROK;
        if octorok {
            self.collider_check(play);
            let p = self.actor.world_pos;
            match play.col.water_surface(p.x, p.z, play.col.water_room) {
                Some(y_surface) if y_surface >= self.actor.floor_height => self.actor.home_pos.y = y_surface,
                _ => {
                    if self.actor.col_chk_info.health != 0 {
                        self.actor.kill();
                        return;
                    }
                }
            }
        }
        self.run_action(play);
        if octorok {
            self.update_head_scale();
            let h = OCTOROK_COLLIDER_INIT.dim;
            self.collider.dim.height = (((h.height as f32 * self.head_scale.y) - self.collider.dim.y_shift as f32) * self.actor.scale.y * 100.0) as i32 as i16;
        } else {
            let mut can_restore_prev_pos = false;
            self.actor.move_forward();
            let prev_pos = self.actor.world_pos;
            self.actor.update_bg_check_info(&play.col, 10.0, 15.0, 30.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
            if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && self.actor.wall_poly.is_some_and(|w| play.col.is_ignored_by_projectiles(w)) {
                can_restore_prev_pos = true;
                self.actor.bg_check_flags &= !BGCHECKFLAG_WALL;
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && self.actor.floor_poly.is_some_and(|f| play.col.is_ignored_by_projectiles(f)) {
                can_restore_prev_pos = true;
                self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND;
            }
            if can_restore_prev_pos && self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_WALL) == 0 {
                self.actor.world_pos = prev_pos;
            }
        }
        self.collider.update(&self.actor);
        if self.action == Action::Appear || self.action == Action::Hide {
            let root_y = self.skel.joint_table.first().map(|j| j[1]).unwrap_or(0);
            self.collider.dim.pos[1] = (self.actor.world_pos.y + (root_y as f32 * self.actor.scale.y)) as i32 as i16;
            self.collider.dim.radius = (OCTOROK_COLLIDER_INIT.dim.radius as f32 * self.actor.scale.x * 100.0) as i32 as i16;
        }
        if TYPE.get(self.actor.params) == TYPE_PROJECTILE {
            self.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        }
        if self.action != Action::WaitToAppear {
            if self.action != Action::Die && self.action != Action::WaitToDie && self.action != Action::Freeze {
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            }
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
        self.actor.set_focus(15.0);
        if octorok && self.drawn {
            self.spawn_ripple(play);
        }
    }

    /// `EnOkuta_Destroy`: the cylinder (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let (head, snout) = self.limb_scales();
        let projectile = TYPE.get(self.actor.params) != TYPE_OCTOROK;
        r.switches = vec![self.drawn as u32, projectile as u32, head.is_some() as u32, snout.is_some() as u32];
        let (h, s) = (head.unwrap_or(Vec3::ONE), snout.unwrap_or(Vec3::ONE));
        r.values = vec![h.x, h.y, h.z, s.x, s.y, s.z];
        r.angles = vec![self.actor.home_rot.z];
        r
    }

    /// `EnOkuta_Draw`: the Octorok (`SkelAnime_DrawOpa`, `EnOkuta_OverrideLimbDraw` scaling the
    /// head and the snout before their own transforms), or the rock facing the camera turned by
    /// `home.rot.z` (`BINANG_TO_RAD`).
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let [drawn, projectile, head, snout] = rs.switches.as_slice() else { return };
        if *drawn == 0 {
            return;
        }
        if *projectile != 0 {
            let rot_z = rs.angles.first().copied().unwrap_or(0) as f32 * (std::f32::consts::PI / 32768.0);
            let m = actor_draw_matrix(rs) * play.billboard_mtx() * Mat4::from_rotation_z(rot_z);
            out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, PROJECTILE_DL)), m));
            return;
        }
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let v = &rs.values;
        if v.len() < 6 {
            return;
        }
        let head = (*head != 0).then(|| Vec3::new(v[rs::HEAD], v[rs::HEAD + 1], v[rs::HEAD + 2]));
        let snout = (*snout != 0).then(|| Vec3::new(v[rs::SNOUT], v[rs::SNOUT + 1], v[rs::SNOUT + 2]));
        let bones = draw_opa_pose(
            skeleton,
            &joints.rot,
            |limb, _pos, _rot| {
                // Matrix_Scale(scale, MTXMODE_APPLY) before the limb's own transform.
                let s = match limb {
                    LIMB_HEAD => head,
                    LIMB_SNOUT => snout,
                    _ => None,
                };
                LimbDraw::Default(s.map(Mat4::from_scale).unwrap_or(Mat4::IDENTITY))
            },
            |_, _| {},
        );
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
