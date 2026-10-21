//! `En_Eiyer` (`ovl_En_Eiyer/z_en_eiyer.c`): the Stinger on the floor, which swims under the
//! ground with its fin out and jumps out at Link (GAME-10 milestone 4). `object_ei`'s
//! `gStingerSkel` (18 limbs, `SkelAnime_Init`'s 19 with the root's), as `En_Weiyer`'s.
//!
//! Jabu-Jabu's Belly (Master Quest, layers 0 to 3) places two in room 14, the boomerang's room:
//! params 0xA (wanderers) at (1080, -1283, -2261) turned 0x8000 and (923, -1293, -2355). Its
//! unused cutscene layer 4 places two more wanderers in room 1 and two leaders (params 0) in
//! room 6. Room 14's clear (`Flags_SetTempClear`, the room's last `ACTORCAT_ENEMY` actor deleted:
//! the two Stingers, the Like Like and the Lizalfos; the Gold Skulltula is an NPC) brings the
//! boomerang's chest (`En_Box` 0x10C6: `ENBOX_TYPE_ROOM_CLEAR_BIG`, `GI_BOOMERANG`, chest flag 6);
//! each Stinger leaves the enemies' list at the end of its death (`EnEiyer_Dead`'s `Actor_Kill`).
//!
//! - **Under the ground** (the collider's AC flags `DMG_BOOMERANG | DMG_EXPLOSIVE | DMG_DEKU_NUT`,
//!   only the fin drawn: limbs 9 and 10, `EnEiyer_OverrideLimbDraw`; no lock-on, no shadow): it
//!   **comes up** (`EnEiyer_AppearFromGround`, `gStingerIdleAnim` looped) from 40 under its home
//!   at 0.75 a frame, then (lock-on on, AC on, no longer always updating) a **wanderer** (params
//!   0xA, `EnEiyer_WanderUnderground`) backs about at 0.5, turning (0xB6 a frame) back to its home
//!   past 100 from it, else now and then (`Rand`) by 0x2000 to 0x4000 either way; a **leader**
//!   (params 0) and its three clones **circle** (`EnEiyer_CircleUnderground`) 80 out round the
//!   home, 0x60 a frame, a quarter turn apart, faced along the circle. Touching Link (OC) it
//!   **jumps out** (`EnEiyer_Ambush`). A Deku nut or the boomerang (no damage) makes it jump out
//!   too; a bomb kills it outright (`EnEiyer_SetupDie`, its speed down 6: backwards).
//! - **Jumping out** (`EnEiyer_Ambush`, `gStingerBackflipAnim` once, `NA_SE_EN_OCTAROCK_JUMP`, a
//!   splash 700): every collider hit (`DMG_DEFAULT`), its shadow and offset back; a backflip of 40
//!   up and back over the first 12 frames, then up to 80 over its home at 0.5; done, AC on and it
//!   **glides** (`EnEiyer_Glide`, `gStingerHitAnim` looped): 85 over the floor bobbing 5,
//!   speeding up to 1 or 1.5, wheeling 0x100 a frame (`targetYaw` met), a wall turning it away;
//!   after 60 frames now and then a new turn. Link under it within 120 across (and the 60 frames
//!   out), it **starts its attack** (`EnEiyer_StartAttack`): a somersault back (0x1000 a frame)
//!   until it points down, then at Link's waist (20 up), turning to him (`Math_ApproachS` a half,
//!   0x71C), speeding to 5; aimed, it **dives** (`EnEiyer_DiveAttack`: AT on, 1.1 times faster each
//!   frame, `NA_SE_EN_EIER_FLY` the while). On a wall or the ground it **lands**
//!   (`EnEiyer_Land`: `gStingerDiveAnim`, AT off, always updating): turning straight down, sinking
//!   on at 7 into the floor; `NA_SE_EN_OCTAROCK_SINK` there (and a splash 700 on the ground), ten
//!   more frames, then back under (`EnEiyer_SetupAppearFromGround`).
//! - **Hit** (out of the ground, `EnEiyer_UpdateDamage`): health 2. A Deku nut or the boomerang
//!   (reaction 1) **stuns** it (`EnEiyer_Stunned`: blue 80 frames, `gStingerPopOutAnim`, falling,
//!   its cylinder 8 taller, `NA_SE_EN_GOMA_JR_FREEZE`; flutters and `NA_SE_EN_DODO_M_GND` landing;
//!   then gliding); damage left it **hurt** (`EnEiyer_Hurt`, `NA_SE_EN_EIER_DAMAGE`: red 40
//!   frames, AC off, backing off at 5 from Link or a wall, rolling, then gliding); none left it
//!   **dies** (`EnEiyer_Die`: the finishing blow, `NA_SE_EN_EIER_DEAD`, no lock-on, red 40 frames,
//!   `gStingerHitAnim` at 6 nose up, spinning, 20 frames or a ceiling), then **fades**
//!   (`EnEiyer_Dead`: drawn translucent at its shadow's alpha, down 5 and 2 lower a frame; gone,
//!   a drop from table 5).
//! - **The clones** (params 1 to 3; `EnEiyer_Init` of the leader spawns clone 1, each clone the
//!   next, every one at the leader's home; the leader takes all three as its own (`parent`);
//!   fewer than three and all are gone): no colliders of their own (only the leader and a
//!   wanderer update theirs), but lock-on and the enemies' list. Waiting (`EnEiyer_Inactive`:
//!   sinking to 50 under the home at 0.5), they come up with the leader (its
//!   `EnEiyer_AppearFromGround`), a quarter turn on from it each; they go back to waiting when it
//!   stops circling, and are gone when it's dead (`EnEiyer_Dead`).
//!
//! The whole overlay is ported. Its draw (`EnEiyer_Draw`) is `gStingerSkel` (`SkelAnime_Draw`,
//! the root 2500 further forward, `EnEiyer_OverrideLimbDraw`): opaque (`ACTOR_SETUP_OPA_DL`, env
//! 255 255 255 255) but dead, translucent then (`gActorSetupXluDL`, the env's alpha the shadow's);
//! under the ground only the fin. Its bakes are `En_Weiyer`'s too.
//!
//! A clone out of view (not updating) while its leader dies and is deleted would read its freed
//! leader (`this->actor.parent`); the port leaves such a clone waiting (none placed in layers 0
//! to 3).
//!
//! Params: `params` whole (0 a leader with three clones circling its home; 1 to 3 the clones,
//! spawned by it; 10 a wanderer. Jabu-Jabu places 0xA; its unused layer 4 0xA and 0).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{approach_f, approach_s, cos_s, scaled_step_to_s, sin_s, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EN_DODO_M_GND, NA_SE_EN_GOMA_JR_FREEZE, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::gbi::Dl;
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};

use crate::en_okuta::{NA_SE_EN_OCTAROCK_JUMP, NA_SE_EN_OCTAROCK_SINK};

/// `ACTOR_EN_EIYER` (`actor_table.h`: 0x003A).
pub const ACTOR_EN_EIYER: i16 = 0x003A;
/// `OBJECT_EI` (`object_table.h`: 0x0026).
pub const OBJECT: &str = "object_ei";
pub const OBJECT_EI: i16 = 0x0026;
/// `gStingerSkel` (`object_ei.xml`: a normal skeleton of 18 standard limbs).
pub const SKEL: &str = "gStingerSkel";
/// Its limbs (`SkelAnime_Init`'s 19 less the root's entry).
pub const LIMBS: usize = 18;

/// The animations (`object_ei.xml`).
pub const ANIM_IDLE: &str = "gStingerIdleAnim";
pub const ANIM_HIT: &str = "gStingerHitAnim";
pub const ANIM_POP_OUT: &str = "gStingerPopOutAnim";
pub const ANIM_BACKFLIP: &str = "gStingerBackflipAnim";
pub const ANIM_DIVE: &str = "gStingerDiveAnim";

/// `En_Eiyer_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, `OBJECT_EI`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_EIYER, name: "En_Eiyer", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE, object: OBJECT, schema: &SCHEMA };

/// `params`, whole: the leader (0), its clones (1 to 3), a wanderer (10).
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[(0, "leader (three clones circling its home)"), (1, "clone 1"), (2, "clone 2"), (3, "clone 3"), (10, "wanderer")]),
    "a leader circling its home with three clones it spawns (0; the clones 1 to 3), or a lone wanderer about its home (10)",
);

/// En_Eiyer's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false, objects: &[], texts: &[] };

/// The types (`params`).
pub const TYPE_LEADER: i16 = 0;
pub const TYPE_WANDERER: i16 = 0xA;

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_EIER_DAMAGE` (0x394C).
pub const NA_SE_EN_EIER_DAMAGE: u16 = 0x394C;
/// `NA_SE_EN_EIER_DEAD` (0x394D).
pub const NA_SE_EN_EIER_DEAD: u16 = 0x394D;
/// `NA_SE_EN_EIER_FLUTTER` (0x394E).
pub const NA_SE_EN_EIER_FLUTTER: u16 = 0x394E;
/// `NA_SE_EN_EIER_FLY` (0x394F).
pub const NA_SE_EN_EIER_FLY: u16 = 0x394F;
/// `NA_SE_EN_EIER_ATTACK` (0x39A2).
pub const NA_SE_EN_EIER_ATTACK: u16 = 0x39A2;
/// `NA_SE_EN_EIER_CRY` (0x39A3).
pub const NA_SE_EN_EIER_CRY: u16 = 0x39A3;

/// `NAVI_ENEMY_STINGER` (`actor.h`: 0x19).
pub const NAVI_ENEMY_STINGER: u8 = 0x19;
/// `ACTOR_FLAG_IGNORE_QUAKE` (`actor.h`: 1 << 12).
pub const ACTOR_FLAG_IGNORE_QUAKE: u32 = 1 << 12;
/// The AC flags under the ground: `DMG_BOOMERANG | DMG_EXPLOSIVE | DMG_DEKU_NUT` (0x19).
pub const DMG_UNDERGROUND: u32 = DMG_BOOMERANG | DMG_EXPLOSIVE | DMG_DEKU_NUT;
/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_5, false)` (`z_en_item00.h`: the table
/// × 16).
const DROP_TABLE_5: i16 = 5 * 16;

/// `sColliderCylinderInit`: hurting 8 with a knockback (`ATELEM_SFX_HARD`), at first hit only by
/// the boomerang, bombs and Deku nuts; 27 by 17, 10 down.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_KNOCKBACK, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0019, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_HARD,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 27, height: 17, y_shift: -10, pos: [0; 3] },
};

/// `sColChkInfoInit`: health 2, 45 by 15, mass 100.
pub const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 2, cyl_radius: 45, cyl_height: 15, mass: 100 };

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x1), // Deku nut
        dmg_entry(2, 0x0), // Deku stick
        dmg_entry(1, 0x0), // Slingshot
        dmg_entry(2, 0x0), // Explosive
        dmg_entry(0, 0x1), // Boomerang
        dmg_entry(2, 0x0), // Normal arrow
        dmg_entry(2, 0x0), // Hammer swing
        dmg_entry(2, 0x0), // Hookshot
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
        dmg_entry(4, 0x0), // Ice magic
        dmg_entry(4, 0x0), // Light magic
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

/// The fin's limbs (1-based), all `EnEiyer_OverrideLimbDraw` draws under the ground:
/// `gStingerFinUpperLimb`, `gStingerFinLowerLimb`.
pub const LIMB_FIN_UPPER: usize = 9;
pub const LIMB_FIN_LOWER: usize = 10;
/// `EnEiyer_OverrideLimbDraw`: the root (limb 1) this much further forward.
pub const ROOT_FORWARD: f32 = 2500.0;

/// The draw's segments: 8 the setup list (`ACTOR_SETUP_OPA_DL`, `&gActorSetupXluDL[2]`: just its
/// end; or `gActorSetupXluDL`), which every limb's list calls (`gStingerBodyDL`'s
/// `gsSPDisplayList(0x08000000)`); 0xE the env colour (`gDPSetEnvColor(255, 255, 255, alpha)`,
/// run before the skeleton).
const SEG_SETUP: u8 = 0x08;
const SEG_ENV: u8 = 0x0E;

/// The bakes of `gStingerSkel`: whole, opaque or translucent (`xlu`; `En_Weiyer`'s draw too), and
/// opaque with only the fin (under the ground).
pub fn bake_name(xlu: bool) -> &'static str {
    if xlu { "En_Eiyer/xlu" } else { "En_Eiyer/opa" }
}
pub const BAKE_FIN: &str = "En_Eiyer/fin";

/// `gActorSetupXluDL` (`z_actor.c`): `gsDPSetRenderMode(G_RM_FOG_SHADE_A, G_RM_AA_ZB_XLU_SURF2)`,
/// `gsDPSetAlphaCompare(G_AC_THRESHOLD)`, the end (as `En_Rd`'s).
fn actor_setup_xlu_dl() -> Vec<(u32, u32)> {
    let mut d = Dl::default();
    d.0.push((0xE200_001C, 0xC810_49F8));
    d.othermode_l(0, 2, 1);
    d.end();
    d.0
}

pub fn bakes() -> Vec<MeshBake> {
    let segments = |xlu: bool| vec![(SEG_SETUP, BakeSegment::Commands(if xlu { actor_setup_xlu_dl() } else { vec![(0xDF00_0000, 0)] })), (SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false })];
    let mut v = Vec::new();
    for xlu in [false, true] {
        v.push(MeshBake { name: bake_name(xlu).into(), object: OBJECT.into(), segments: segments(xlu), prelude: vec![SEG_ENV], body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() } });
    }
    // EnEiyer_OverrideLimbDraw: *dList = NULL but for limbs 9 and 10.
    let limbs = (1..=LIMBS).filter(|&l| l != LIMB_FIN_UPPER && l != LIMB_FIN_LOWER).map(|l| LimbOverride { limb: (l - 1) as u8, file: OBJECT.into(), symbol: String::new() }).collect();
    v.push(MeshBake { name: BAKE_FIN.into(), object: OBJECT.into(), segments: segments(false), prelude: vec![SEG_ENV], body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs } });
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    AppearFromGround,
    WanderUnderground,
    CircleUnderground,
    Inactive,
    Ambush,
    Glide,
    StartAttack,
    DiveAttack,
    Land,
    Hurt,
    Die,
    Dead,
    Stunned,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::AppearFromGround => "EnEiyer_AppearFromGround",
            Action::WanderUnderground => "EnEiyer_WanderUnderground",
            Action::CircleUnderground => "EnEiyer_CircleUnderground",
            Action::Inactive => "EnEiyer_Inactive",
            Action::Ambush => "EnEiyer_Ambush",
            Action::Glide => "EnEiyer_Glide",
            Action::StartAttack => "EnEiyer_StartAttack",
            Action::DiveAttack => "EnEiyer_DiveAttack",
            Action::Land => "EnEiyer_Land",
            Action::Hurt => "EnEiyer_Hurt",
            Action::Die => "EnEiyer_Die",
            Action::Dead => "EnEiyer_Dead",
            Action::Stunned => "EnEiyer_Stunned",
        }
    }
}

/// `object_ei`'s animations, as both Stingers use them.
pub(crate) struct Anims {
    pub idle: Anim,
    pub hit: Anim,
    pub pop_out: Anim,
    pub backflip: Anim,
    pub dive: Anim,
}

impl Anims {
    pub(crate) fn load(play: &PlayState, who: &str) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("{who}: {e:#}")).ok();
        Some(Anims { idle: get(ANIM_IDLE)?, hit: get(ANIM_HIT)?, pop_out: get(ANIM_POP_OUT)?, backflip: get(ANIM_BACKFLIP)?, dive: get(ANIM_DIVE)? })
    }
}

/// `gStingerSkel` and `SkelAnime_Init(.., &gStingerSkel, &gStingerIdleAnim, .., 19)`.
pub(crate) fn skel_init(play: &PlayState, who: &str, anims: Option<&Anims>) -> (Option<Arc<Skeleton>>, SkelAnimeStd) {
    let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("{who}: {e:#}")).ok());
    let skel = SkelAnimeStd::init_flex(skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(LIMBS), anims.map(|a| a.idle.clone()));
    (skeleton, skel)
}

/// `Actor_IsFacingPlayer`.
pub(crate) fn is_facing_player(a: &Actor, max_angle: i16) -> bool {
    (a.yaw_towards_player.wrapping_sub(a.shape_rot.y) as i32).abs() < max_angle as i32
}

/// Player's world position (`GET_PLAYER(play)->actor.world.pos`).
pub(crate) fn player_pos(play: &PlayState) -> Vec3 {
    play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default()
}

/// `Actor_WorldPitchTowardPoint` toward Player's waist (his position 20 up).
pub(crate) fn pitch_toward_player(a: &Actor, play: &PlayState) -> i16 {
    let p = player_pos(play);
    eng_math::vec3f_pitch(a.world_pos, Vec3::new(p.x, p.y + 20.0, p.z))
}

/// The Stingers' draw: `gStingerSkel` with the root `forward` further forward (limb 1's
/// position), opaque (env 255 255 255 255) or translucent at `alpha`, whole or only the fin.
pub(crate) fn draw_stinger(skeleton: &Skeleton, rs: &RenderState, forward: f32, xlu: bool, fin_only: bool, alpha: u8, out: &mut DrawOut) {
    let Some(joints) = &rs.joints else { return };
    let bones = skeleton.pose_override(&joints.rot, |limb, pos, _rot| {
        if limb == 1 {
            pos.z += forward;
        }
        Mat4::IDENTITY
    });
    let mut sv = SegmentValues::default();
    sv.env[SEG_ENV as usize] = Some([255, 255, 255, if xlu { alpha } else { 255 }]);
    let mesh = if fin_only && !xlu { BAKE_FIN } else { bake_name(xlu) };
    let cmd = DrawCmd { mesh: MeshKey::named(keys::bake(mesh)), transform: actor_draw_matrix(rs), bones, params: DrawParams { segments: Some(sv), ..Default::default() } };
    if xlu {
        out.xlu.push(cmd);
    } else {
        out.opa.push(cmd);
    }
}

pub struct EnEiyer {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    pub action: Action,
    pub timer: i16,
    pub target_yaw: i16,
    pub base_pos: Vec3,
    pub collider: ColliderCylinder,
}

impl EnEiyer {
    /// `EnEiyer_Init`: see the module's docs.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: NAVI_ENEMY_STINGER, scale 5 / 1000, lockOnArrowOffset 2500.
        actor.navi_enemy_id = NAVI_ENEMY_STINGER;
        actor.scale = Vec3::splat(0.005);
        actor.target_arrow_offset = 2500.0;
        actor.shape_init(600.0, oot_game::shadow::ShadowDraw::Circle, 65.0);
        let anims = Anims::load(play, PROFILE.name);
        let (skeleton, skel) = skel_init(play, PROFILE.name, anims.as_ref());
        actor.col_chk_info.set_info(Some(&S_DAMAGE_TABLE), &COL_CHK_INFO_INIT);
        let mut this = EnEiyer { actor, skel, skeleton, anims, action: Action::Inactive, timer: 0, target_yaw: 0, base_pos: Vec3::ZERO, collider: ColliderCylinder::new(&CYLINDER_INIT) };
        let ty = TYPE.get(this.actor.params);
        if ty < 3 {
            // Each clone spawns another clone.
            let h = this.actor.home_pos;
            let rot = [0, this.actor.shape_rot.y.wrapping_add(0x4000), 0];
            if let Err(e) = play.actor_spawn_as_child(&mut this.actor, ACTOR_EN_EIYER, h, rot, ty + 1) {
                log::debug!("En_Eiyer: no clone {}: {e:?}", ty + 1);
                this.actor.kill();
                return Box::new(this);
            }
            if ty == TYPE_LEADER {
                let mut chain = Vec::new();
                let mut child = this.actor.child;
                for _ in 0..3 {
                    let Some(c) = child else { break };
                    chain.push(c);
                    child = play.actors.actor(c).and_then(|a| a.child);
                }
                if chain.len() != 3 {
                    // for (child = &this->actor; child != NULL; child = child->child) Actor_Kill(child).
                    this.actor.kill();
                    let mut child = this.actor.child;
                    while let Some(c) = child {
                        let Some(a) = play.actors.actor_mut(c) else { break };
                        a.kill();
                        child = a.child;
                    }
                    return Box::new(this);
                }
                // this->actor.child(->child(->child))->parent = &this->actor: clone 1 is its own
                // child already; clones 2 and 3 linked to it once it's spawned.
                play.adopt_from_init(chain[1]);
                play.adopt_from_init(chain[2]);
            }
        }
        if ty == TYPE_LEADER || ty == TYPE_WANDERER {
            this.setup_appear_from_ground(play);
        } else {
            this.setup_inactive();
        }
        Box::new(this)
    }

    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `Animation_Change`.
    fn change(&mut self, f: impl Fn(&Anims) -> &Anim, play_speed: f32, start: f32, end: f32, mode: u8, morph: f32) {
        if let Some(a) = self.anim(f) {
            self.skel.change(a, play_speed, start, end, mode, morph);
        }
    }

    /// `Animation_MorphToLoop`.
    fn morph_to_loop(&mut self, f: impl Fn(&Anims) -> &Anim, morph: f32) {
        self.change(f, 1.0, 0.0, 0.0, ANIMMODE_LOOP, morph);
    }

    /// The leader of a clone (`this->actor.parent`, an `En_Eiyer`): its action, and its yaw.
    fn parent(&self, play: &PlayState) -> Option<(Action, i16)> {
        let p = self.actor.parent.and_then(|h| play.actors.downcast::<EnEiyer>(h));
        if p.is_none() {
            log::debug!("En_Eiyer: clone {} has no leader", TYPE.get(self.actor.params));
        }
        p.map(|p| (p.action, p.actor.world_rot.y))
    }

    /// `EnEiyer_RotateAroundHome`: 80 out from the home at its yaw, faced along the circle.
    fn rotate_around_home(&mut self) {
        let a = &mut self.actor;
        a.world_pos.x = sin_s(a.world_rot.y) * 80.0 + a.home_pos.x;
        a.world_pos.z = cos_s(a.world_rot.y) * 80.0 + a.home_pos.z;
        a.shape_rot.y = a.world_rot.y.wrapping_add(0x4000);
    }

    /// `EnEiyer_SetupAppearFromGround`: 40 under its home, under-the-ground hits, a random yaw (a
    /// clone: its leader's and a quarter turn per number), no lock-on, no shadow.
    pub fn setup_appear_from_ground(&mut self, play: &mut PlayState) {
        self.collider.info.ac_dmg_info.dmg_flags = DMG_UNDERGROUND;
        if let Some(a) = self.anim(|a| &a.idle) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
        }
        let h = self.actor.home_pos;
        self.actor.world_pos = Vec3::new(h.x, h.y - 40.0, h.z);
        self.actor.velocity.y = 0.0;
        self.actor.speed_xz = 0.0;
        let ty = TYPE.get(self.actor.params);
        if ty != TYPE_WANDERER {
            if ty == TYPE_LEADER {
                self.actor.world_rot.y = (play.rand.zero_one() * 65536.0) as i32 as i16;
            } else {
                let parent_yaw = self.parent(play).map(|p| p.1).unwrap_or(0);
                self.actor.world_rot.y = parent_yaw.wrapping_add((ty as i32 * 0x4000) as i16);
            }
            self.rotate_around_home();
        } else {
            let yaw = (play.rand.zero_one() * 65536.0) as i32 as i16;
            self.actor.world_rot.y = yaw;
            self.actor.shape_rot.y = yaw;
        }
        self.collider.base.at_flags &= !AT_ON;
        self.collider.base.ac_flags &= !AC_ON;
        self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_IGNORE_QUAKE);
        self.actor.shape_shadow_scale = 0.0;
        self.actor.shape_y_offset = 0.0;
        self.action = Action::AppearFromGround;
    }

    /// `EnEiyer_SetupUnderground`: wandering (backing at 0.5) or circling; AC and lock-on on.
    pub fn setup_underground(&mut self) {
        if TYPE.get(self.actor.params) == TYPE_WANDERER {
            self.actor.speed_xz = -0.5;
            self.action = Action::WanderUnderground;
        } else {
            self.action = Action::CircleUnderground;
        }
        self.collider.base.ac_flags |= AC_ON;
        self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
    }

    /// `EnEiyer_SetupInactive`.
    pub fn setup_inactive(&mut self) {
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.action = Action::Inactive;
    }

    /// `EnEiyer_SetupAmbush`: see the module's docs.
    pub fn setup_ambush(&mut self, play: &mut PlayState) {
        self.actor.speed_xz = 0.0;
        if let Some(a) = self.anim(|a| &a.backflip) {
            self.skel.play_once(a);
        }
        self.collider.info.ac_dmg_info.dmg_flags = DMG_DEFAULT;
        self.base_pos = self.actor.world_pos;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.actor.flags |= ACTOR_FLAG_IGNORE_QUAKE;
        self.collider.base.ac_flags &= !AC_ON;
        self.actor.shape_shadow_scale = 65.0;
        self.actor.shape_y_offset = 600.0;
        audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_JUMP);
        let pos = self.actor.world_pos;
        play.with_ss(|s| s.g_splash_spawn(pos, None, None, 1, 700));
        self.action = Action::Ambush;
    }

    /// `EnEiyer_SetupGlide`: 5 over where it is (the bob's low point), `gStingerHitAnim`, 60 frames.
    pub fn setup_glide(&mut self) {
        self.target_yaw = self.actor.shape_rot.y;
        // cosf(-M_PI / 8): M_PI a float.
        self.base_pos.y = ((-std::f32::consts::PI / 8.0).cos() * 5.0) + self.actor.world_pos.y;
        self.morph_to_loop(|a| &a.hit, -5.0);
        self.timer = 60;
        self.action = Action::Glide;
    }

    /// `EnEiyer_SetupStartAttack`.
    pub fn setup_start_attack(&mut self) {
        self.action = Action::StartAttack;
    }

    /// `EnEiyer_SetupDiveAttack`: AT on (its last hit forgotten).
    pub fn setup_dive_attack(&mut self, play: &PlayState) {
        self.actor.velocity.y = 0.0;
        self.base_pos.y = player_pos(play).y + 15.0;
        self.collider.base.at_flags |= AT_ON;
        self.collider.base.at_flags &= !AT_HIT;
        self.action = Action::DiveAttack;
    }

    /// `EnEiyer_SetupLand`: `gStingerDiveAnim`, AT off, always updating; the first frame's check
    /// to come (`timer` -1).
    pub fn setup_land(&mut self) {
        if let Some(a) = self.anim(|a| &a.dive) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, -3.0);
        }
        self.collider.base.at_flags &= !AT_ON;
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.timer = -1;
        self.actor.gravity = 0.0;
        self.collider.dim.height = CYLINDER_INIT.dim.height;
        self.action = Action::Land;
    }

    /// `EnEiyer_SetupHurt`: red 40 frames, AC off, backing off at 5.
    pub fn setup_hurt(&mut self) {
        self.base_pos.y = self.actor.world_pos.y;
        self.change(|a| &a.hit, 2.0, 0.0, 0.0, ANIMMODE_LOOP, -3.0);
        self.timer = 40;
        self.actor.gravity = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.speed_xz = 5.0;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 200, COLORFILTER_BUFFLAG_OPA, 40);
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::Hurt;
    }

    /// `EnEiyer_SetupDie`: red 40 frames; out of the ground at 6 (`gStingerHitAnim`), under it 6
    /// slower (backwards); every hit again, AT and AC off.
    pub fn setup_die(&mut self) {
        self.timer = 20;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 200, COLORFILTER_BUFFLAG_OPA, 40);
        if self.collider.info.ac_dmg_info.dmg_flags != DMG_UNDERGROUND {
            self.actor.speed_xz = 6.0;
            self.morph_to_loop(|a| &a.hit, -3.0);
        } else {
            self.actor.speed_xz -= 6.0;
        }
        self.collider.info.ac_dmg_info.dmg_flags = DMG_DEFAULT;
        self.collider.base.at_flags &= !AT_ON;
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::Die;
    }

    /// `EnEiyer_SetupDead`: still; drawn translucent (`colorFilterParams |= 0x2000`).
    pub fn setup_dead(&mut self) {
        self.actor.speed_xz = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.color_filter_params |= 0x2000;
        self.action = Action::Dead;
    }

    /// `EnEiyer_SetupStunned`: see the module's docs.
    pub fn setup_stunned(&mut self, play: &mut PlayState) {
        self.change(|a| &a.pop_out, 2.0, 0.0, 0.0, ANIMMODE_LOOP, -8.0);
        self.timer = 80;
        self.actor.speed_xz = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.gravity = -1.0;
        self.collider.dim.height = CYLINDER_INIT.dim.height + 8;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 200, COLORFILTER_BUFFLAG_OPA, 80);
        self.collider.base.at_flags &= !AT_ON;
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.action = Action::Stunned;
    }

    /// `EnEiyer_AppearFromGround`: up to its home at 0.75.
    fn appear_from_ground(&mut self) {
        self.skel.update();
        if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 0.75) {
            self.setup_underground();
        }
    }

    /// `EnEiyer_CheckPlayerCollision`: touching Link, it jumps out.
    fn check_player_collision(&mut self, play: &mut PlayState) {
        if self.collider.base.oc_flags2 & OC2_HIT_PLAYER != 0 {
            self.collider.base.oc_flags2 &= !OC2_HIT_PLAYER;
            self.setup_ambush(play);
        }
    }

    /// `EnEiyer_CircleUnderground`: 0x60 a frame round the home; a clone waits once its leader
    /// stops.
    fn circle_underground(&mut self, play: &mut PlayState) {
        self.skel.update();
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(-0x60);
        self.rotate_around_home();
        self.check_player_collision(play);
        // Clones disappear when the main Eiyer leaves the ground.
        if TYPE.get(self.actor.params) != TYPE_LEADER && self.parent(play).is_some_and(|p| p.0 != Action::CircleUnderground) {
            self.setup_inactive();
        }
    }

    /// `EnEiyer_WanderUnderground`: back toward its home past 100 from it, else now and then a
    /// new turn.
    fn wander_underground(&mut self, play: &mut PlayState) {
        self.skel.update();
        let (w, h) = (self.actor.world_pos, self.actor.home_pos);
        // Actor_WorldDistXZToPoint.
        let dist = ((h.x - w.x) * (h.x - w.x) + (h.z - w.z) * (h.z - w.z)).sqrt();
        if dist > 100.0 {
            // Actor_WorldYawTowardPoint.
            self.target_yaw = eng_math::vec3f_yaw(w, h).wrapping_add(-0x8000i16);
        } else if self.target_yaw == self.actor.world_rot.y && play.rand.zero_one() > 0.99 {
            let sign: f32 = if play.rand.zero_one() < 0.5 { -1.0 } else { 1.0 };
            let turn = sign * (play.rand.zero_one() * 8192.0 + 8192.0);
            self.target_yaw = (self.actor.world_rot.y as f32 + turn) as i32 as i16;
        }
        scaled_step_to_s(&mut self.actor.world_rot.y, self.target_yaw, 0xB6);
        self.check_player_collision(play);
    }

    /// `EnEiyer_Inactive`: sinking to 50 under its home; gone with its leader dead, up with it.
    fn inactive(&mut self, play: &mut PlayState) {
        if self.actor.home_pos.y - 50.0 < self.actor.world_pos.y {
            self.actor.world_pos.y -= 0.5;
        }
        match self.parent(play).map(|p| p.0) {
            Some(Action::Dead) => self.actor.kill(),
            Some(Action::AppearFromGround) => self.setup_appear_from_ground(play),
            _ => {}
        }
    }

    /// `EnEiyer_Ambush`: see the module's docs.
    fn ambush(&mut self, play: &mut PlayState) {
        let anim_finished = self.skel.update();
        let cur_frame = self.skel.cur_frame;
        if self.skel.cur_frame < 12.0 {
            // (0.996f * M_PI / 12.0f) * curFrame: M_PI a float.
            let a = ((0.996f32 * std::f32::consts::PI) / 12.0) * cur_frame;
            self.actor.world_pos.y = ((1.0 - a.cos()) * 40.0) + self.actor.home_pos.y;
            let xz_offset = a.sin() * -40.0;
            self.actor.world_pos.x = (sin_s(self.actor.shape_rot.y) * xz_offset) + self.base_pos.x;
            self.actor.world_pos.z = (cos_s(self.actor.shape_rot.y) * xz_offset) + self.base_pos.z;
        } else {
            step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 80.0, 0.5);
            self.actor.speed_xz = 0.8;
        }
        if anim_finished {
            self.collider.base.ac_flags |= AC_ON;
            self.setup_glide();
        } else {
            // BgCheck_EntityRaycastDown4.
            let (floor_height, floor_poly) = play.col.entity_raycast_down(self.actor.world_pos);
            self.actor.floor_height = floor_height;
            self.actor.floor_poly = floor_poly;
        }
    }

    /// `EnEiyer_Glide`: see the module's docs.
    fn glide(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.timer != 0 {
            self.timer -= 1;
        }
        let cur_frame = self.skel.cur_frame;
        approach_f(&mut self.base_pos.y, self.actor.floor_height + 80.0 + 5.0, 0.3, self.actor.speed_xz);
        // cosf((curFrame - 5.0f) * (M_PI / 40)).
        self.actor.world_pos.y = self.base_pos.y - ((cur_frame - 5.0) * (std::f32::consts::PI / 40.0)).cos() * 5.0;
        if cur_frame <= 45.0 {
            step_to_f(&mut self.actor.speed_xz, 1.0, 0.03);
        } else {
            step_to_f(&mut self.actor.speed_xz, 1.5, 0.03);
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.target_yaw = self.actor.wall_yaw;
        }
        if scaled_step_to_s(&mut self.actor.world_rot.y, self.target_yaw, 0xB6) {
            if self.timer != 0 || play.rand.zero_one() > 0.05 {
                self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(0x100);
            } else {
                let yaw_change = play.rand.s16_offset(0x2000, 0x2000);
                let sign = if play.rand.zero_one() < 0.5 { -1 } else { 1 };
                self.target_yaw = (sign * yaw_change as i32 + self.actor.world_rot.y as i32) as i16;
            }
        }
        if self.timer == 0 && self.actor.y_dist_to_player < 0.0 && self.actor.xz_dist_to_player < 120.0 {
            self.setup_start_attack();
        }
        self.actor.play_sfx_flagged(NA_SE_EN_EIER_FLY - SFX_FLAG);
    }

    /// `EnEiyer_StartAttack`: the somersault, then aiming at Link's waist.
    fn start_attack(&mut self, play: &mut PlayState) {
        self.skel.update();
        // `shape.rot.x < 0x8000` always holds of an s16.
        if self.actor.shape_rot.x > 0 {
            let pitch = pitch_toward_player(&self.actor, play);
            if scaled_step_to_s(&mut self.actor.shape_rot.x, pitch, 0x1000) {
                self.setup_dive_attack(play);
            }
        } else {
            self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_sub(0x1000);
        }
        self.actor.world_rot.x = self.actor.shape_rot.x.wrapping_neg();
        step_to_f(&mut self.actor.speed_xz, 5.0, 0.3);
        approach_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 2, 0x71C);
        self.actor.play_sfx_flagged(NA_SE_EN_EIER_FLY - SFX_FLAG);
    }

    /// `EnEiyer_DiveAttack`: 1.1 times faster a frame; a wall or the ground, it lands.
    fn dive_attack(&mut self) {
        self.skel.update();
        self.actor.speed_xz *= 1.1;
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 || self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.setup_land();
        }
        if self.collider.base.at_flags & AT_HIT != 0 {
            self.collider.base.at_flags &= !(AT_ON | AT_HIT);
        }
        self.actor.play_sfx_flagged(NA_SE_EN_EIER_FLY - SFX_FLAG);
    }

    /// `EnEiyer_Land`: see the module's docs.
    fn land(&mut self, play: &mut PlayState) {
        self.skel.update();
        scaled_step_to_s(&mut self.actor.world_rot.x, -0x4000, 0x450);
        step_to_f(&mut self.actor.speed_xz, 7.0, 1.0);
        if self.timer == -1 {
            if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 || self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.timer = 10;
                let pos = self.actor.world_pos;
                play.sfx_source_play_sfx_at_fixed_world_pos(pos, 30, NA_SE_EN_OCTAROCK_SINK);
                if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                    play.with_ss(|s| s.g_splash_spawn(pos, None, None, 1, 700));
                }
            }
        } else {
            if self.timer != 0 {
                self.timer -= 1;
            }
            if self.timer == 0 {
                self.actor.shape_rot.x = 0;
                self.actor.world_rot.x = 0;
                self.setup_appear_from_ground(play);
            }
        }
    }

    /// `EnEiyer_Hurt`: see the module's docs.
    fn hurt(&mut self) {
        self.skel.update();
        if self.timer != 0 {
            self.timer -= 1;
        }
        approach_f(&mut self.base_pos.y, self.actor.floor_height + 80.0 + 5.0, 0.5, self.actor.speed_xz);
        self.actor.world_pos.y = self.base_pos.y - 5.0;
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.target_yaw = self.actor.wall_yaw;
        } else {
            self.target_yaw = self.actor.yaw_towards_player.wrapping_add(-0x8000i16);
        }
        scaled_step_to_s(&mut self.actor.world_rot.y, self.target_yaw, 0x38E);
        scaled_step_to_s(&mut self.actor.shape_rot.x, 0, 0x200);
        // sinf(timer * (M_PI / 5)) * 5120.0f.
        self.actor.shape_rot.z = ((self.timer as f32 * (std::f32::consts::PI / 5.0)).sin() * 5120.0) as i32 as i16;
        if self.timer == 0 {
            self.actor.shape_rot.x = 0;
            self.actor.shape_rot.z = 0;
            self.collider.base.ac_flags |= AC_ON;
            self.setup_glide();
        }
        self.actor.world_rot.x = self.actor.shape_rot.x.wrapping_neg();
    }

    /// `EnEiyer_Die`: nose up (or down, backing), spinning, 20 frames or a ceiling.
    fn die(&mut self) {
        self.skel.update();
        if self.actor.speed_xz > 0.0 {
            scaled_step_to_s(&mut self.actor.shape_rot.x, -0x4000, 0x400);
        } else {
            scaled_step_to_s(&mut self.actor.shape_rot.x, 0x4000, 0x400);
        }
        self.actor.shape_rot.z = self.actor.shape_rot.z.wrapping_add(0x1000);
        if self.timer != 0 {
            self.timer -= 1;
        }
        self.actor.world_rot.x = self.actor.shape_rot.x.wrapping_neg();
        if self.timer == 0 || self.actor.bg_check_flags & BGCHECKFLAG_CEILING != 0 {
            self.setup_dead();
        }
    }

    /// `EnEiyer_Dead`: fading 5 a frame and sinking 2; gone, a drop from table 5.
    fn dead(&mut self, play: &mut PlayState) {
        self.actor.shape_shadow_alpha = (self.actor.shape_shadow_alpha as i16 - 5).max(0) as u8;
        self.actor.world_pos.y -= 2.0;
        if self.actor.shape_shadow_alpha == 0 {
            let pos = self.actor.world_pos;
            crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, DROP_TABLE_5);
            self.actor.kill();
        }
    }

    /// `EnEiyer_Stunned`: 80 frames, fluttering, then gliding.
    fn stunned(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        scaled_step_to_s(&mut self.actor.shape_rot.x, 0, 0x200);
        self.skel.update();
        if self.skel.on_frame(0.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_EIER_FLUTTER);
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
        }
        if self.timer == 0 {
            self.actor.gravity = 0.0;
            self.actor.velocity.y = 0.0;
            self.collider.dim.height = CYLINDER_INIT.dim.height;
            self.setup_glide();
        }
    }

    /// `EnEiyer_UpdateDamage`: see the module's docs.
    fn update_damage(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT == 0 {
            return;
        }
        self.collider.base.ac_flags &= !AC_HIT;
        let elem = self.collider.info;
        self.actor.set_drop_flag(&elem, true);
        let c = &self.actor.col_chk_info;
        if c.damage_reaction == 0 && c.damage == 0 {
            return;
        }
        if self.actor.apply_damage() == 0 {
            enemy_start_finishing_blow(play, &self.actor);
            audio_play_actor_sfx2(play, NA_SE_EN_EIER_DEAD);
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        }
        // If underground, one hit kill.
        if self.collider.info.ac_dmg_info.dmg_flags == DMG_UNDERGROUND {
            if self.actor.col_chk_info.damage == 0 {
                self.setup_ambush(play);
            } else {
                self.setup_die();
            }
        } else if self.actor.col_chk_info.damage_reaction == 1 {
            if self.action != Action::Stunned {
                self.setup_stunned(play);
            }
        } else if self.actor.col_chk_info.health != 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_EIER_DAMAGE);
            self.setup_hurt();
        } else {
            self.collider.dim.height = CYLINDER_INIT.dim.height;
            self.setup_die();
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::AppearFromGround => self.appear_from_ground(),
            Action::WanderUnderground => self.wander_underground(play),
            Action::CircleUnderground => self.circle_underground(play),
            Action::Inactive => self.inactive(play),
            Action::Ambush => self.ambush(play),
            Action::Glide => self.glide(play),
            Action::StartAttack => self.start_attack(play),
            Action::DiveAttack => self.dive_attack(),
            Action::Land => self.land(play),
            Action::Hurt => self.hurt(),
            Action::Die => self.die(),
            Action::Dead => self.dead(play),
            Action::Stunned => self.stunned(play),
        }
    }

    /// Only the fin is drawn (`EnEiyer_OverrideLimbDraw`: the under-the-ground hits).
    pub fn fin_only(&self) -> bool {
        self.collider.info.ac_dmg_info.dmg_flags == DMG_UNDERGROUND
    }
}

/// `render_state`'s layout: switches dead (translucent), only the fin; values the alpha.
mod rs {
    pub const DEAD: usize = 0;
    pub const FIN: usize = 1;
}

impl ActorImpl for EnEiyer {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnEiyer_Update`: its hits, the action, the move (straight on, or along its pitch), the
    /// floor and walls in the air, the shape's yaw, the colliders (the leader's and a
    /// wanderer's), the focus 12.5 ahead.
    fn update(&mut self, play: &mut PlayState) {
        self.update_damage(play);
        self.run_action(play);
        if self.actor.world_rot.x == 0 || self.action == Action::Stunned {
            self.actor.move_forward();
        } else {
            self.actor.move_xyz();
        }
        let a = self.action;
        if a == Action::Glide || a == Action::DiveAttack || a == Action::Stunned || a == Action::Die || a == Action::Hurt || (a == Action::Land && self.timer == -1) {
            self.actor.update_bg_check_info(&play.col, 5.0, 27.0, 30.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2);
        }
        let ty = TYPE.get(self.actor.params);
        if ty == TYPE_WANDERER || (a != Action::AppearFromGround && a != Action::CircleUnderground) {
            self.actor.shape_rot.y = self.actor.world_rot.y;
        }
        // Only the main Eiyer can ambush the player.
        if ty == TYPE_LEADER || ty == TYPE_WANDERER {
            self.collider.update(&self.actor);
            if self.collider.base.at_flags & AT_ON != 0 {
                play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            }
            if self.collider.base.ac_flags & AC_ON != 0 {
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            }
            if self.action != Action::Ambush {
                play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            }
        }
        if self.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED != 0 {
            let w = self.actor.world_pos;
            self.actor.focus_pos = Vec3::new(w.x + sin_s(self.actor.shape_rot.y) * 12.5, w.y, w.z + cos_s(self.actor.shape_rot.y) * 12.5);
        }
    }

    /// `EnEiyer_Destroy`: the cylinder (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        r.switches = vec![(self.action == Action::Dead) as u32, self.fin_only() as u32];
        r.values = vec![self.actor.shape_shadow_alpha as f32];
        r
    }

    /// `EnEiyer_Draw`: see the module's docs.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(skeleton) = &self.skeleton else { return };
        let (Some(&dead), Some(&fin), Some(&alpha)) = (rs.switches.get(rs::DEAD), rs.switches.get(rs::FIN), rs.values.first()) else { return };
        draw_stinger(skeleton, rs, ROOT_FORWARD, dead != 0, fin != 0, alpha.clamp(0.0, 255.0) as u8, out);
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

/// The handles of a leader's clones, by following `child` (clone 1, 2, 3).
pub fn clones(play: &PlayState, leader: ActorHandle) -> Vec<ActorHandle> {
    let mut v = Vec::new();
    let mut child = play.actors.actor(leader).and_then(|a| a.child);
    while let Some(c) = child {
        if v.len() == 3 {
            break;
        }
        v.push(c);
        child = play.actors.actor(c).and_then(|a| a.child);
    }
    v
}
