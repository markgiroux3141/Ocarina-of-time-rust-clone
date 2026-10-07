//! `En_Arrow` (`ovl_En_Arrow/z_en_arrow.c`): the arrows, the Deku Seed and the Deku Nut, as
//! projectiles. `params` is the kind (`ArrowType`): the arrows (`ARROW_NORMAL_SILENT` -1 to
//! `ARROW_0E` 8: lit, horseback, normal, fire, ice, light and three unused ones), the seed
//! (`ARROW_SEED` 9) and the nut (`ARROW_NUT` 10; `ARROW_CS_NUT` -10 is a cutscene's nut, made a
//! nut that also updates in Player's cutscene modes).
//!
//! - **Init**: the arrows' skeleton (`gArrowSkel`, playing `gArrow2_Anim`), their trail
//!   (`Effect_Add(EFFECT_BLURE2)`), and for everything but the nut an attack quad (`AT_TYPE_PLAYER`,
//!   or `AT_TYPE_ENEMY` for -1) with the kind's damage (`dmgFlags`: the seed's `DMG_SLINGSHOT`).
//! - **Held** (`EnArrow_Shoot` with a `parent`): Player holds it and places it; it waits. Let go
//!   (no parent), a seed or an arrow flies only if Player's just shot it (`unk_A73`), else it's
//!   gone at once; a nut always flies. Seeds and nuts go at 80 along their pitch and yaw
//!   (`Actor_SetProjectileSpeed`), for 15 frames; arrows at 150 for 12.
//! - **Flying** (`EnArrow_Fly`): straight, then from a timer under 7.2 falling by 0.4 a frame,
//!   gone when the timer runs out. Each move is checked for walls, floors and ceilings
//!   (`BgCheck_ProjectileLineTest`); the quad, stretched each draw from the last draw's edge to
//!   the new one (`func_809B4800`, `Player_UpdateWeaponInfo`), is the attack. Next frame a seed
//!   or nut that hit anything bursts (`Effect_Ss_Stone1`): the seed with `NA_SE_IT_SLING_REFLECT`,
//!   the nut with `NA_SE_IT_DEKU`, a screen flash (`R_TRANS_FADE_FLASH_ALPHA_STEP` -1) and its
//!   stun (`En_M_Fire1`). An arrow sticks: into a wall for 20 frames (60 for the special ones),
//!   into an actor that can carry one (`ACTOR_FLAG_CAN_ATTACH_TO_ARROW`) carrying it along
//!   (`EnArrow_CarryActor`), off anything else bouncing away (`func_809B3CEC`).
//! - **Drawn**: an arrow as its skeleton (`SkelAnime_DrawLod`, the far model past `MREG(95)`), a
//!   flying seed or nut as a spinning sparkle (`gEffSparklesDL`, its alpha pulsing with the timer).
//!
//! The whole overlay is ported, but: the trails (`EffectBlure`, `z_eff_blure.c`) aren't, so
//! `Effect_Add`, `Effect_Delete` and `EffectBlure_AddVertex` keep their checks and log what they'd
//! do; the magic arrows' children (`Arrow_Fire`, `Arrow_Ice`, `Arrow_Light`) are logged, not
//! spawned; `Player_InBlockingCsMode`'s magic part isn't (magic isn't ported).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2, func_8002f9ec, player_play_sfx};
use oot_game::collision_check::*;
use oot_game::effect::EffectInit;
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_ONCE, Anim, SkelAnimeStd};
use oot_game::sys_matrix::{MtxF, binang_to_rad};
use oot_game::transition::TRANS_TRIGGER_START;

/// `ACTOR_EN_ARROW` (`actor_table.h`).
pub const ACTOR_EN_ARROW: i16 = 0x0016;
/// `ACTOR_ARROW_FIRE`, `ACTOR_ARROW_ICE`, `ACTOR_ARROW_LIGHT` (`actor_table.h`): the magic
/// arrows' children (not ported: logged).
pub const ACTOR_ARROW_FIRE: i16 = 0x010A;
pub const ACTOR_ARROW_ICE: i16 = 0x010B;
pub const ACTOR_ARROW_LIGHT: i16 = 0x010C;

/// `En_Arrow_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_ARROW, name: "En_Arrow", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED, object: "gameplay_keep" };

/// `ArrowType` (`z_en_arrow.h`).
pub const ARROW_CS_NUT: i16 = -10;
pub const ARROW_NORMAL_SILENT: i16 = -1;
pub const ARROW_NORMAL_LIT: i16 = 0;
pub const ARROW_NORMAL_HORSE: i16 = 1;
pub const ARROW_NORMAL: i16 = 2;
pub const ARROW_FIRE: i16 = 3;
pub const ARROW_ICE: i16 = 4;
pub const ARROW_LIGHT: i16 = 5;
pub const ARROW_0C: i16 = 6;
pub const ARROW_0D: i16 = 7;
pub const ARROW_0E: i16 = 8;
pub const ARROW_SEED: i16 = 9;
pub const ARROW_NUT: i16 = 10;

/// `ACTOR_FLAG_CAN_ATTACH_TO_ARROW`, `ACTOR_FLAG_ATTACHED_TO_ARROW` (`actor.h`).
pub const ACTOR_FLAG_CAN_ATTACH_TO_ARROW: u32 = 1 << 14;
pub const ACTOR_FLAG_ATTACHED_TO_ARROW: u32 = 1 << 15;

// The item bank's sounds (`sfx/itembank_table.h`).
pub const NA_SE_IT_ARROW_SHOT: u16 = 0x1804;
pub const NA_SE_IT_ARROW_STICK_CRE: u16 = 0x1814;
pub const NA_SE_IT_ARROW_STICK_OBJ: u16 = 0x1815;
pub const NA_SE_IT_SLING_SHOT: u16 = 0x1820;
pub const NA_SE_IT_SLING_REFLECT: u16 = 0x1825;
pub const NA_SE_IT_DEKU: u16 = 0x182B;
pub const NA_SE_IT_MAGIC_ARROW_SHOT: u16 = 0x1839;

/// `ACTOR_EN_M_FIRE1` (`actor_table.h`): the nut's stun.
const ACTOR_EN_M_FIRE1: i16 = crate::en_m_fire1::ACTOR_EN_M_FIRE1;

const SKEL: &str = "gArrowSkel";
const ANIM_1: &str = "gArrow1_Anim";
const ANIM_2: &str = "gArrow2_Anim";
/// The skeleton's limb with the lists (`gArrowSkelLimb_3`: `gArrowNearDL`, `gArrowFarDL`).
const LIMB_ARROW: u8 = 3;
/// The bakes: the skeleton with each LOD's list (`Gfx_SetupDL_25Opa`, the bake's start), and the
/// sparkle (`Gfx_SetupDL_25Xlu2`: the same setup, `SETUPDL_25`) with its colours dynamic.
const BAKE_LOD: [&str; 2] = ["EnArrow/lod0", "EnArrow/lod1"];
const BAKE_SPARKLES: &str = "EnArrow/sparkles";
const SEG_COLOR: u8 = 0x08;

/// `sColliderQuadInit`.
pub const QUAD_INIT: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_PLAYER, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_PLAYER, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0020, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x01 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_NEAREST | ATELEM_SFX_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `ATELEM_SFX_MASK` (`collision_check.h`).
const ATELEM_SFX_MASK: u8 = 3 << 3;

/// `EnArrow_Init`'s `dmgFlags`, by kind 0 to 9.
pub const DMG_FLAGS: [u32; 10] = [DMG_ARROW_FIRE, DMG_ARROW_NORMAL, DMG_ARROW_NORMAL, DMG_ARROW_FIRE, DMG_ARROW_ICE, DMG_ARROW_LIGHT, DMG_ARROW_UNK3, DMG_ARROW_UNK1, DMG_ARROW_UNK2, DMG_SLINGSHOT];

/// `func_809B4800`'s statics: the quad's edge ahead of the projectile (`sPosAOffset`,
/// `sPosBOffset`), and `unk_21C`'s point behind it (`D_809B4EA0`), in model space.
const S_POS_A_OFFSET: Vec3 = Vec3::new(0.0, 400.0, 1500.0);
const S_POS_B_OFFSET: Vec3 = Vec3::new(0.0, -400.0, 1500.0);
const D_809B4EA0: Vec3 = Vec3::new(0.0, 0.0, -300.0);

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnArrow_Shoot`: held, until let go.
    Shoot,
    /// `EnArrow_Fly`.
    Fly,
    /// `func_809B45E0`: an arrow stuck in a wall.
    StuckInWall,
    /// `func_809B4640`: an arrow bouncing off an actor.
    Bounce,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Shoot => "EnArrow_Shoot",
            Action::Fly => "EnArrow_Fly",
            Action::StuckInWall => "func_809B45E0",
            Action::Bounce => "func_809B4640",
        }
    }
}

pub struct EnArrow {
    pub actor: Actor,
    /// `skelAnime` (the arrows'), and its skeleton and animations.
    pub skel: Option<SkelAnimeStd>,
    skeleton: Option<Arc<Skeleton>>,
    anims: [Option<Anim>; 2],
    pub collider: ColliderQuad,
    /// `unk_210`: the position before this frame's move.
    pub unk_210: Vec3,
    /// `unk_21C`: 3 behind it (as drawn): where a lit arrow's flame burns.
    pub unk_21c: Vec3,
    /// `effectIndex`: the trail's (`TOTAL_EFFECT_COUNT`: `EffectBlure` isn't ported).
    pub effect_index: usize,
    pub weapon_info: WeaponInfo,
    /// `timer`: the frames left flying, or stuck.
    pub timer: u8,
    pub hit_flags: u8,
    /// `touchedPoly`: last frame's move hit a poly.
    pub touched_poly: bool,
    pub is_cs_nut: bool,
    /// `hitActor`: the actor it carries.
    pub hit_actor: Option<ActorHandle>,
    /// `unk_250`: the carried actor's offset from it.
    pub unk_250: Vec3,
    pub action: Action,
    /// Draw-time: the LOD `EnArrow_Draw` picks, and `play->gameplayFrames` as drawn.
    lod: u8,
    drawn_frames: u32,
}

/// `DECR(x)`: down by 1 unless 0, then the value.
fn decr(t: &mut u8) -> u8 {
    if *t != 0 {
        *t -= 1;
    }
    *t
}

/// What `En_Arrow` reads of Player: his handle, `unk_A73` and `Player_InBlockingCsMode` (the
/// transition's part with Player's; magic isn't ported).
fn player_state(play: &PlayState) -> (Option<ActorHandle>, u8, bool) {
    let h = play.player;
    let p = h.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player());
    let a73 = p.map(|p| p.unk_a73()).unwrap_or(0);
    let blocking = p.is_some_and(|p| p.in_blocking_cs_mode()) || play.transition.trigger == TRANS_TRIGGER_START;
    (h, a73, blocking)
}

/// The element `r` points at, as it is now (`atHitElem`, read through its pointer).
fn live_element(play: &mut PlayState, r: ElemRef) -> Option<ColliderElement> {
    let a = play.actors.get_mut(r.col.actor)?;
    let i = r.elem as usize;
    match a.collider_mut(r.col.id)? {
        ColliderMut::JntSph(c) => c.elements.get(i).map(|e| e.info),
        ColliderMut::Cylinder(c) => (i == 0).then_some(c.info),
        ColliderMut::Tris(c) => c.elements.get(i).map(|e| e.info),
        ColliderMut::Quad(c) => (i == 0).then_some(c.info),
    }
}

/// The meshes the arrows draw with.
pub fn bakes() -> Vec<MeshBake> {
    let skel = |name: &str, limbs: Vec<LimbOverride>| MeshBake {
        name: name.into(),
        object: "gameplay_keep".into(),
        segments: Vec::new(),
        prelude: Vec::new(),
        body: BakeBody::Skeleton { file: "gameplay_keep".into(), symbol: SKEL.into(), limbs },
    };
    vec![
        skel(BAKE_LOD[0], Vec::new()),
        skel(BAKE_LOD[1], vec![LimbOverride { limb: LIMB_ARROW, file: "gameplay_keep".into(), symbol: "gArrowFarDL".into() }]),
        MeshBake {
            name: BAKE_SPARKLES.into(),
            object: "gameplay_keep".into(),
            segments: vec![(SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true })],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffSparklesDL".into())]),
        },
    ]
}

impl EnArrow {
    fn params(&self) -> i16 {
        self.actor.params
    }

    /// `EnArrow_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // Actor_ProcessInitChain: ICHAIN_F32(minVelocityY, -150).
        actor.min_velocity_y = -150.0;
        let mut is_cs_nut = false;
        if actor.params == ARROW_CS_NUT {
            is_cs_nut = true;
            actor.params = ARROW_NUT;
        }
        let params = actor.params;
        let mut s = EnArrow {
            actor,
            skel: None,
            skeleton: None,
            anims: [None, None],
            collider: ColliderQuad::default(),
            unk_210: Vec3::ZERO,
            unk_21c: Vec3::ZERO,
            effect_index: 0,
            weapon_info: WeaponInfo::default(),
            timer: 0,
            hit_flags: 0,
            touched_poly: false,
            is_cs_nut,
            hit_actor: None,
            unk_250: Vec3::ZERO,
            action: Action::Shoot,
            lod: 0,
            drawn_frames: 0,
        };
        if params <= ARROW_SEED {
            if params <= ARROW_0E {
                // SkelAnime_Init(play, &skelAnime, &gArrowSkel, &gArrow2_Anim, NULL, NULL, 0):
                // gArrow2_Anim looped.
                if let Some(a) = play.assets.clone() {
                    s.skeleton = a.skeleton("gameplay_keep", SKEL).map_err(|e| log::error!("En_Arrow: {e:#}")).ok();
                    s.anims = [ANIM_1, ANIM_2].map(|n| a.animation("gameplay_keep", n).map_err(|e| log::error!("En_Arrow: {e:#}")).ok());
                }
                let limbs = s.skeleton.as_ref().map(|k| k.limbs.len()).unwrap_or(4);
                s.skel = Some(SkelAnimeStd::init_flex(limbs, s.anims[1].clone()));
            }
            // The trail: Effect_Add(EFFECT_BLURE2) with the kind's init (z_eff_blure.c isn't
            // ported: no slot).
            let blure = if params <= ARROW_NORMAL {
                // blureNormal.elemDuration, a static the init rewrites: 4 on horseback, else 16.
                let elem_duration = if params == ARROW_NORMAL_HORSE { 4 } else { 16 };
                Some(format!("blureNormal (elemDuration {elem_duration})"))
            } else if params == ARROW_FIRE {
                Some("blureFire".to_string())
            } else if params == ARROW_ICE {
                Some("blureIce".to_string())
            } else if params == ARROW_LIGHT {
                Some("blureLight".to_string())
            } else {
                None
            };
            if let Some(b) = blure {
                log::debug!("En_Arrow: Effect_Add(EFFECT_BLURE2, {b})");
                s.effect_index = play.effect_add(EffectInit::Blure);
            }
            s.collider = ColliderQuad::new(&QUAD_INIT);
            if params <= ARROW_NORMAL {
                s.collider.info.at_elem_flags &= !ATELEM_SFX_MASK;
                s.collider.info.at_elem_flags |= ATELEM_SFX_NORMAL;
            }
            if params < 0 {
                s.collider.base.at_flags = AT_ON | AT_TYPE_ENEMY;
            } else if params <= ARROW_SEED {
                s.collider.info.at_dmg_info.dmg_flags = DMG_FLAGS[params as usize];
                log::debug!("this->at_info.cl_elem.at_btl_info.at_type = {:#x}", s.collider.info.at_dmg_info.dmg_flags);
            }
        }
        // EnArrow_SetupAction(this, EnArrow_Shoot).
        s.action = Action::Shoot;
        Box::new(s)
    }

    /// `Actor_SetProjectileSpeed`: `speed` along the pitch (`world.rot.x`, positive down).
    fn set_projectile_speed(&mut self, speed: f32) {
        self.actor.speed_xz = speed * cos_s(self.actor.world_rot.x);
        self.actor.velocity.y = speed * -sin_s(self.actor.world_rot.x);
    }

    /// `Animation_PlayOnce`.
    fn play_once(&mut self, i: usize) {
        if let (Some(sk), Some(a)) = (&mut self.skel, self.anims[i].clone()) {
            let last = a.last_frame();
            sk.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, 0.0);
        }
    }

    /// `EnArrow_Shoot`.
    fn shoot(&mut self, play: &mut PlayState) {
        let (player, a73, _) = player_state(play);
        if self.actor.parent.is_none() {
            if self.params() != ARROW_NUT && a73 == 0 {
                self.actor.kill();
                return;
            }
            let sfx = match self.params() {
                ARROW_SEED => Some(NA_SE_IT_SLING_SHOT),
                ARROW_NORMAL_LIT | ARROW_NORMAL_HORSE | ARROW_NORMAL => Some(NA_SE_IT_ARROW_SHOT),
                ARROW_FIRE | ARROW_ICE | ARROW_LIGHT => Some(NA_SE_IT_MAGIC_ARROW_SHOT),
                _ => None,
            };
            // Player_PlaySfx: at Player's projectedPos.
            if let (Some(id), Some(p)) = (sfx, player) {
                player_play_sfx(play, p, id);
            }
            self.action = Action::Fly;
            self.unk_210 = self.actor.world_pos;
            if self.params() >= ARROW_SEED {
                self.set_projectile_speed(80.0);
                self.timer = 15;
                self.actor.shape_rot = Rot { x: 0, y: 0, z: 0 };
            } else {
                self.set_projectile_speed(150.0);
                self.timer = 12;
            }
        }
    }

    /// `func_809B3CEC`: an arrow off an actor: bouncing back about the way it came (turned by
    /// 0x8000 and up to ±0x3000), up by 0.4 to 0.8 of its speed, slowed to 0.04 to 0.34 of it,
    /// falling by 1.5, for 50 frames, `gArrow1_Anim` once.
    fn func_809b3cec(&mut self, play: &mut PlayState) {
        self.action = Action::Bounce;
        self.play_once(0);
        let r = (24576.0 * (play.rand.zero_one() - 0.5)) as i32;
        self.actor.world_rot.y = (self.actor.world_rot.y as i32 + r + 0x8000) as i16;
        self.actor.velocity.y += self.actor.speed_xz * (0.4 + (0.4 * play.rand.zero_one()));
        self.actor.speed_xz *= 0.04 + 0.3 * play.rand.zero_one();
        self.timer = 50;
        self.actor.gravity = -1.5;
    }

    /// `EnArrow_CarryActor`: the hit actor moved on with it, along this frame's move, as far as
    /// it's ahead of the actor (stopping 1 short of a wall in the way).
    fn carry_actor(&mut self, play: &mut PlayState) {
        let Some(h) = self.hit_actor else { return };
        let Some(hit_pos) = play.actors.actor(h).map(|a| a.world_pos) else { return };
        let mut pos_diff_last_frame = self.actor.world_pos - self.unk_210;
        let d = self.actor.world_pos - hit_pos;
        let temp_f12 = (d.x * pos_diff_last_frame.x) + (d.y * pos_diff_last_frame.y) + (d.z * pos_diff_last_frame.z);
        if !(temp_f12 < 0.0) {
            let mut scale = pos_diff_last_frame.length_squared();
            if !(scale < 1.0) {
                scale = temp_f12 / scale;
                pos_diff_last_frame *= scale;
                let actor_next_pos = hit_pos + pos_diff_last_frame;
                let new_pos = match play.col.entity_line_test(hit_pos, actor_next_pos, true, true, true, true) {
                    Some((p, _)) => Vec3::new(
                        p.x + if actor_next_pos.x <= p.x { 1.0 } else { -1.0 },
                        p.y + if actor_next_pos.y <= p.y { 1.0 } else { -1.0 },
                        p.z + if actor_next_pos.z <= p.z { 1.0 } else { -1.0 },
                    ),
                    None => actor_next_pos,
                };
                if let Some(a) = play.actors.actor_mut(h) {
                    a.world_pos = new_pos;
                }
            }
        }
    }

    /// `EnArrow_Fly`.
    fn fly(&mut self, play: &mut PlayState) {
        if decr(&mut self.timer) == 0 {
            self.actor.kill();
            return;
        }
        if (self.timer as f32) < 7.2000003 {
            self.actor.gravity = -0.4;
        }
        let params = self.params();
        let at_touched = params != ARROW_NORMAL_LIT && params <= ARROW_SEED && self.collider.base.at_flags & AT_HIT != 0;
        if at_touched || self.touched_poly {
            if params >= ARROW_SEED {
                if at_touched {
                    // (prevPos is where Actor_UpdateAll found it this frame, before any move:
                    // the midpoint is the position.)
                    self.actor.world_pos.x = (self.actor.world_pos.x + self.actor.prev_pos.x) * 0.5;
                    self.actor.world_pos.y = (self.actor.world_pos.y + self.actor.prev_pos.y) * 0.5;
                    self.actor.world_pos.z = (self.actor.world_pos.z + self.actor.prev_pos.z) * 0.5;
                }
                let pos = self.actor.world_pos;
                let sfx_id = if params == ARROW_NUT {
                    play.trans_fade_flash_alpha_step = -1;
                    if let Err(e) = play.actor_spawn(ACTOR_EN_M_FIRE1, pos, [0, 0, 0], 0) {
                        log::debug!("En_Arrow: En_M_Fire1 not spawned: {e:?}");
                    }
                    NA_SE_IT_DEKU
                } else {
                    NA_SE_IT_SLING_REFLECT
                };
                play.with_ss(|s| s.stone1_spawn(pos, false));
                play.sfx_source_play_sfx_at_fixed_world_pos(pos, 20, sfx_id);
                self.actor.kill();
            } else {
                let pos = self.actor.world_pos;
                play.with_ss(|s| s.hit_mark_spawn_custom_scale(0, 150, pos));
                let hit_elem = if at_touched { self.collider.info.at_hit_elem } else { None };
                let live = match hit_elem {
                    Some(he) => live_element(play, he.elem).or(Some(ColliderElement { elem_material: he.elem_material, ..ColliderElement::default() })),
                    None => None,
                };
                if let Some(elem) = live.filter(|e| at_touched && e.elem_material != ELEM_MATERIAL_UNK4) {
                    let hit_actor = self.collider.base.at;
                    let attachable = hit_actor.and_then(|h| play.actors.actor(h)).filter(|a| !a.killed).is_some_and(|a| a.flags & ACTOR_FLAG_CAN_ATTACH_TO_ARROW != 0);
                    if attachable && self.collider.base.at_flags & AT_BOUNCED == 0 {
                        let h = hit_actor.unwrap();
                        self.hit_actor = Some(h);
                        self.carry_actor(play);
                        let a = play.actors.actor_mut(h).unwrap();
                        self.unk_250 = a.world_pos - self.actor.world_pos;
                        a.flags |= ACTOR_FLAG_ATTACHED_TO_ARROW;
                        self.collider.base.at_flags &= !AT_HIT;
                        self.actor.speed_xz /= 2.0;
                        self.actor.velocity.y /= 2.0;
                    } else {
                        self.hit_flags |= 1;
                        self.hit_flags |= 2;
                        if elem.ac_elem_flags & ACELEM_HIT != 0 {
                            let p = elem.ac_dmg_info.hit_pos;
                            self.actor.world_pos = Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
                        }
                        self.func_809b3cec(play);
                        audio_play_actor_sfx2(play, NA_SE_IT_ARROW_STICK_CRE);
                    }
                } else if self.touched_poly {
                    self.action = Action::StuckInWall;
                    self.play_once(1);
                    self.timer = if params >= ARROW_NORMAL_LIT { 60 } else { 20 };
                    audio_play_actor_sfx2(play, NA_SE_IT_ARROW_STICK_OBJ);
                    self.hit_flags |= 1;
                }
            }
        } else {
            self.unk_210 = self.actor.world_pos;
            self.actor.move_forward();
            let hit = play.col.projectile_line_test(self.actor.prev_pos, self.actor.world_pos, true, true, true, true);
            self.actor.wall_poly = hit.map(|(_, p)| p);
            self.touched_poly = hit.is_some();
            if let Some((hit_point, poly)) = hit {
                func_8002f9ec(play, poly, hit_point);
                // (posCopy: kept, never read.)
                self.actor.world_pos = hit_point;
            }
            if params <= ARROW_0E {
                self.actor.shape_rot.x = eng_math::atan2_s(self.actor.speed_xz, -self.actor.velocity.y);
            }
        }
        if let Some(h) = self.hit_actor {
            if play.actors.actor(h).is_some_and(|a| !a.killed) {
                let sp60 = self.unk_210 + self.unk_250;
                let sp54 = self.actor.world_pos + self.unk_250;
                match play.col.entity_line_test(sp60, sp54, true, true, true, true) {
                    Some((p, _)) => {
                        let a = play.actors.actor_mut(h).unwrap();
                        a.world_pos.x = p.x + if sp54.x <= p.x { 1.0 } else { -1.0 };
                        a.world_pos.y = p.y + if sp54.y <= p.y { 1.0 } else { -1.0 };
                        a.world_pos.z = p.z + if sp54.z <= p.z { 1.0 } else { -1.0 };
                        self.unk_250 = a.world_pos - self.actor.world_pos;
                        a.flags &= !ACTOR_FLAG_ATTACHED_TO_ARROW;
                        self.hit_actor = None;
                    }
                    None => {
                        let a = play.actors.actor_mut(h).unwrap();
                        a.world_pos = self.actor.world_pos + self.unk_250;
                    }
                }
                if self.touched_poly
                    && let Some(h) = self.hit_actor
                {
                    if let Some(a) = play.actors.actor_mut(h) {
                        a.flags &= !ACTOR_FLAG_ATTACHED_TO_ARROW;
                    }
                    self.hit_actor = None;
                }
            } else {
                self.hit_actor = None;
            }
        }
    }

    /// `func_809B45E0`: stuck, its animation running, gone when the timer runs out.
    fn func_809b45e0(&mut self) {
        if let Some(sk) = &mut self.skel {
            sk.update();
        }
        if decr(&mut self.timer) == 0 {
            self.actor.kill();
        }
    }

    /// `func_809B4640`: bouncing (`Actor_MoveXZGravity`), gone when the timer runs out.
    fn func_809b4640(&mut self) {
        if let Some(sk) = &mut self.skel {
            sk.update();
        }
        self.actor.move_forward();
        if decr(&mut self.timer) == 0 {
            self.actor.kill();
        }
    }

    /// `Actor_Draw`'s matrix (`Matrix_SetTranslateRotateYXZ`, `Matrix_Scale`) as `EnArrow_Draw`
    /// leaves it for `func_809B4800`: a flying seed or nut's turned by its yaw after the
    /// sparkle (`Matrix_RotateY(world.rot.y)`); the arrows' is back to the actor's after the
    /// skeleton.
    fn draw_mtx(&self) -> MtxF {
        let a = &self.actor;
        let mut m = MtxF::set_translate_rotate_yxz(a.world_pos.x, a.world_pos.y + a.shape_y_offset * a.scale.y, a.world_pos.z, [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z]);
        m.scale(a.scale.x, a.scale.y, a.scale.z);
        if self.params() > ARROW_0E && self.actor.speed_xz != 0.0 {
            m.rotate_y(binang_to_rad(a.world_rot.y));
        }
        m
    }

    /// `func_809B4800`: `unk_21C`, and while flying the quad's new edge (`sPosAOffset`,
    /// `sPosBOffset` through the matrix): the seed's and the arrows' attack
    /// (`Player_UpdateWeaponInfo`; while carrying an actor only compared), and the trail's vertex
    /// for the arrows up to the light one (`EffectBlure_AddVertex`: logged).
    fn func_809b4800(&mut self, play: &mut PlayState, m: &MtxF) {
        self.unk_21c = m.mult_vec3f(D_809B4EA0);
        if self.action == Action::Fly {
            let pos_a = m.mult_vec3f(S_POS_A_OFFSET);
            let pos_b = m.mult_vec3f(S_POS_B_OFFSET);
            if self.params() <= ARROW_SEED {
                let mut add_blure_vertex = self.params() <= ARROW_LIGHT;
                if self.hit_actor.is_none() {
                    add_blure_vertex &= player_update_weapon_info(play, &self.actor, Some((0, &mut self.collider)), &mut self.weapon_info, pos_a, pos_b);
                } else if add_blure_vertex {
                    let w = &self.weapon_info;
                    if pos_a.x == w.pos_a.x && pos_a.y == w.pos_a.y && pos_a.z == w.pos_a.z && pos_b.x == w.pos_b.x && pos_b.y == w.pos_b.y && pos_b.z == w.pos_b.z {
                        add_blure_vertex = false;
                    }
                }
                if add_blure_vertex {
                    log::debug!("En_Arrow: EffectBlure_AddVertex(effect {}, {pos_a}, {pos_b})", self.effect_index);
                }
            }
        }
    }
}

impl ActorImpl for EnArrow {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnArrow_Update`: the action, unless Player is in a cutscene mode
    /// (`Player_InBlockingCsMode`) and it's neither a cutscene's nut nor a seed or arrow just
    /// shot; then a magic arrow's child (logged) or a lit arrow's flame.
    fn update(&mut self, play: &mut PlayState) {
        let (_, a73, blocking) = player_state(play);
        if self.is_cs_nut || (self.params() >= ARROW_NORMAL_LIT && a73 != 0) || !blocking {
            match self.action {
                Action::Shoot => self.shoot(play),
                Action::Fly => self.fly(play),
                Action::StuckInWall => self.func_809b45e0(),
                Action::Bounce => self.func_809b4640(),
            }
        }
        let params = self.params();
        if (ARROW_FIRE..=ARROW_0E).contains(&params) {
            // elementalActorIds.
            const ELEMENTAL_ACTOR_IDS: [i16; 6] = [ACTOR_ARROW_FIRE, ACTOR_ARROW_ICE, ACTOR_ARROW_LIGHT, ACTOR_ARROW_FIRE, ACTOR_ARROW_FIRE, ACTOR_ARROW_FIRE];
            if self.actor.child.is_none() {
                log::debug!("En_Arrow: Actor_SpawnAsChild({:#06x}) at {} (not ported)", ELEMENTAL_ACTOR_IDS[(params - 3) as usize], self.actor.world_pos);
            }
        } else if params == ARROW_NORMAL_LIT {
            // The flame's dust (func_8002836C at unk_21C).
            let pos = self.unk_21c;
            let v = Vec3::new(0.0, 0.5, 0.0);
            play.with_ss(|s| s.func_8002836c(pos, v, v, [255, 255, 100, 255], [255, 50, 0, 0], 100, 0, 8));
        }
    }

    /// `EnArrow_Destroy`: the trail deleted (the arrows up to the light one), and a carried
    /// actor let go.
    fn destroy(&mut self, play: &mut PlayState) {
        if self.params() <= ARROW_LIGHT {
            play.effect_delete(self.effect_index);
        }
        // SkelAnime_Free, Collider_DestroyQuad: nothing to free.
        if let Some(h) = self.hit_actor
            && let Some(a) = play.actors.actor_mut(h).filter(|a| !a.killed)
        {
            a.flags &= !ACTOR_FLAG_ATTACHED_TO_ARROW;
        }
    }

    /// `EnArrow_Draw`'s effects on the actor: the LOD (`projectedPos.z < MREG(95)`: the near
    /// model), and `func_809B4800`.
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.actor.killed {
            return;
        }
        // MREG(95), as Player_SetBootData leaves it for Link's age (the Kokiri boots' rows).
        let mreg95 = play.data.regs[if play.save.adult { 0 } else { 1 }].mreg(95) as f32;
        self.lod = if self.actor.projected_pos.z < mreg95 { 0 } else { 1 };
        self.drawn_frames = play.gameplay_frames;
        let m = self.draw_mtx();
        self.func_809b4800(play, &m);
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        if let Some(sk) = &self.skel {
            rs.joints = Some(eng_anim::anim::JointTable { rot: sk.joint_table.clone(), face: 0 });
        }
        rs.switches = vec![self.timer as u32, self.lod as u32, self.drawn_frames, (self.actor.speed_xz != 0.0) as u32];
        rs
    }

    /// `EnArrow_Draw`: an arrow's skeleton (`Gfx_SetupDL_25Opa`, `SkelAnime_DrawLod`); a seed or
    /// nut, once moving, a sparkle (`Gfx_SetupDL_25Xlu2`) facing the camera
    /// (`billboardMtxF`), spun by `(gameplayFrames & 0xFF) × 4000`, 50 across for a seed (white,
    /// cyan) and 150 for a nut (dark red, yellow), its env alpha
    /// `cosS(timer × 5000) × 127.5 + 127.5`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let &[timer, lod, frames, moving] = rs.switches.as_slice() else { return };
        if self.params() <= ARROW_0E {
            let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
            let bones = skeleton.pose(joints);
            out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_LOD[(lod as usize).min(1)])), transform: actor_draw_matrix(rs), bones, params: DrawParams::default() });
        } else if moving != 0 {
            let alpha = ((cos_s((timer as i32 * 5000) as i16) * 127.5) + 127.5) as u8;
            let (prim, env, scale) = if self.params() == ARROW_SEED { ([255, 255, 255, 255], [0, 255, 255, alpha], 50.0) } else { ([12, 0, 0, 255], [250, 250, 0, alpha], 150.0) };
            // BINANG_TO_RAD of the product, not wrapped to 16 bits.
            let rot_z = (((frames & 0xFF) * 4000) as f32 as f64 * (std::f64::consts::PI / 32768.0)) as f32;
            let m = actor_draw_matrix(rs) * play.billboard_mtx() * Mat4::from_rotation_z(rot_z) * Mat4::from_scale(Vec3::splat(scale));
            let mut sv = SegmentValues::default();
            sv.prim[SEG_COLOR as usize] = Some(prim);
            sv.env[SEG_COLOR as usize] = Some(env);
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_SPARKLES)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Quad(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// The sparkle's env alpha for `timer` (`EnArrow_Draw`), for the tests.
pub fn sparkle_alpha(timer: u8) -> u8 {
    ((cos_s((timer as i32 * 5000) as i16) * 127.5) + 127.5) as u8
}
