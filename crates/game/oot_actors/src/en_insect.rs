//! `En_Insect` (`ovl_En_Insect/z_en_insect.c`, the decomp's "z_en_mushi.c" in its prints): the
//! bugs. `gameplay_keep`'s `gBugSkel` crawling (`gBugCrawlAnim`, looped), 0.01 big. Four types
//! (`EnInsectType`, `params & 3`), each starting with its `sInitInsectFlags`:
//!
//! - **0, `INSECT_TYPE_PERMANENT`**: an `Obj_Mure` group's (type 3, `sSpawnParams`); no gravity,
//!   no floor or wall checks (it crawls at its spawn height), never digs in or drowns.
//! - **1, `INSECT_TYPE_SPAWNED`**: from under a rock (`En_Ishi`) or a bush (`En_Kusa`);
//!   `INSECT_FLAG_0` (gravity -0.2 down to -2, the floor and the water) and
//!   `INSECT_FLAG_IS_SHORT_LIVED` (a life of 200 to 239 frames, `Rand_S16Offset(200, 40)`, and
//!   updating out of view).
//! - **2, `INSECT_TYPE_FIRST_DROPPED`**: the one Player's bottle lets out
//!   (`Player_Action_8084EFC0`'s `D_80854A28`, on frame 76, at his left hand, rotation
//!   (0x4000, his yaw, 0)); `INSECT_FLAG_1` too (the walls). Its init spawns two more at its
//!   position and rotation (z made 0 first): **3, `INSECT_TYPE_EXTRA_DROPPED`**, the same flags.
//!
//! **Crawling** (`INSECT_FLAG_CRAWLING`, `NA_SE_EN_MUSI_WALK` every `3 / playSpeed` frames, at
//! least 2, while on the ground or for type 0 always): three spells picked at random at init
//! (`Rand_ZeroOne`: under 0.3 slowing down, under 0.4 crawling, else running from Link), each
//! timed by `actionTimer` and handing to the next: slowing to a stop (`EnInsect_SlowDown`, 5 to
//! 39 frames) then crawling at 1.5 (`EnInsect_Crawl`, 10 to 54: back home when beyond 40 of it or
//! in its last 4 frames, else after the bug `Obj_Mure` set as its `actor.child`, not itself),
//! then slowing again; within 40 of Link it runs at 1.8 (`EnInsect_RunFromPlayer`, 10 to 49: away
//! from him, zigzagging by `play->state.frames`' bits 4 and 5, back home beyond 160 or at the
//! end) until he's 40 away. A short-lived bug whose life is out, or a dropped one on the ground
//! while four or more dropped bugs are about (`sDroppedCount`), **digs in** (`EnInsect_Dig`, 60
//! frames, `NA_SE_EN_MUSI_SINK`, uncatchable): it stops, tips down to 10922 (60°), shrinks
//! 0.0002 a frame to 0.001 and sinks 0.8 a frame, jittering about where it began, kicking up
//! brown dust (`func_800286CC`, 8 to 12 big) one frame in ten for its first 40; then it's gone.
//! Touching **water** (`BGCHECKFLAG_WATER_TOUCH`, the bugs with `INSECT_FLAG_0`) it walks on the
//! surface (`EnInsect_WalkOnWater`, 120 to 169 frames: at 0.6 for all but its last 80, slowing
//! after, turning at random, ripples one frame in 33) and then **drowns** (`EnInsect_Drown`, 100
//! frames, uncatchable: sinking at -0.04 to -0.8, spinning, shrinking, bubbles between 5 and 30
//! deep); out of the water first, it crawls again (or heads for its soil).
//!
//! **Dropped** (`EnInsect_Dropped`, 100 frames, 0.003 big growing 0.0008 a frame to 0.01, a
//! random heading and spin): falling it tumbles (`shape.rot.x` -2000 a frame) and drifts; on the
//! ground it turns upright (`world.rot.x` to 0, 2000 a frame; `NA_SE_EN_MUSI_LAND` once) and
//! crawls in spurts (one frame in 14 a new speed and heading: towards home give or take a half
//! turn, less as `D_80A7DEB0`, the overlay's shared 0 to 1 over 300 frames, grows; from 0.999
//! straight home at 0.6 to 1.2); upright, with no soil, it **crawls on as a normal bug**
//! (`EnInsect_SetupSlowDown`). With **soft soil** (`Obj_Makekinsuta`, the Gold Skulltula soil,
//! the nearest in its room within 80 across at its init: `EnInsect_TryFindNearbySoil`, which
//! also zeroes `D_80A7DEB0`) its home moves about the soil (one frame in 14, within ±half of
//! `(1.1 - D_80A7DEB0) × 100 + 20` while beyond 30 of it, `(1 - D_80A7DEB0) × 10` within, 0
//! from 0.999): within 30 its life is held (`INSECT_FLAG_SOIL_CLOSE`: no more pushing Link);
//! within 3, or its time or life out, or on the ground with four dropped bugs about, it digs in;
//! and a dig ending within 8 of the soil puts the bug in it (`soilActor->unk_152 = 1`: 60 frames
//! on the soil spawns its Gold Skulltula). The first dropped bug heading for a soil chimes
//! (`NA_SE_SY_TRE_BOX_APPEAR`) on its 16th frame unless that soil's token is taken
//! (`GET_GS_FLAGS`).
//!
//! **Caught**: within 50 of Link, until it's caught (`EnInsect_Caught`), each update it pushes
//! him (`CollisionCheck_SetOC` with its sphere: not near its soil, and only from 20 frames of a
//! short life in) and, catchable (not `INSECT_FLAG_UNCATCHABLE`), within 32 of him (the 16-out
//! point behind him test of `EnInsect_InBottleRange` is always met then) and fewer than four
//! offers this frame (`sCaughtCount`, zeroed by every bug's draw), **offers itself to the
//! bottle** (`Actor_OfferGetItem(GI_MAX, 60, 30)`: `GI_MAX` is what only a bottle's swing takes,
//! making it Player's `interactRangeActor`). Player's swing (`Player_Action_SwingBottle`,
//! `sBottleCatchInfo`'s `ACTOR_EN_INSECT` row) sets its `parent`; at its next update the bug
//! clears it (`Actor_HasParent`) and a dropped bug is gone, any other **caught**
//! (`EnInsect_SetupCaught`): undrawn at 0.001, still, for 200 frames; a short-lived one then gone,
//! a permanent one drawn again for its last 20, growing back to 0.01, and crawling on.
//!
//! The whole overlay is ported. Its draw is `EnInsect_Draw` (`Gfx_SetupDL_25Opa`,
//! `SkelAnime_DrawOpa`, from a bake: docs/adr/0012-actor-bakes.md); `draw_update` is the rest of
//! it (`Collider_UpdateSpheres` at the actor's matrix, `sCaughtCount = 0`). No shadow:
//! `EnInsect_Init` calls no `ActorShape_Init`, and `Actor_Init`'s leaves `shadowDraw` `NULL`. On
//! the water it leaves ripples (`EffectSsGRipple`), drowning bubbles (`EffectSsBubble`). The
//! overlay's statics (`D_80A7DEB0`, `sCaughtCount`,
//! `sDroppedCount`) are reset as the overlay's unload and reload would (`ACTOROVL_ALLOC_NORMAL`):
//! when the last bug is deleted.
//! @bug (game): `EnInsect_InBottleRange`'s second test (a point 16 behind Link, by the bug's yaw
//! to him, within 20 of the bug) always passes once the first (within 32) does: kept.
//! `EnInsect_Dropped` reads a soil's `GET_GS_FLAGS` with its index minus 1: a soil whose index
//! bits are 0 would read before the array (none does; the port reads it as no flag).
//!
//! Params: `type` bits 0 and 1 (`EnInsectType`: 0 `INSECT_TYPE_PERMANENT`, 1
//! `INSECT_TYPE_SPAWNED`, 2 `INSECT_TYPE_FIRST_DROPPED`, 3 `INSECT_TYPE_EXTRA_DROPPED`). Only
//! spawned: by `Obj_Mure` (0), `En_Ishi` and `En_Kusa` (1), Player's bottle (2) and the first
//! dropped bug (3).
//! Flags: `GET_GS_FLAGS(index - 1) & flag` of the soil a first dropped bug heads for (its
//! `Obj_Makekinsuta`'s params: `GS_FLAGS_INDEX`, `GS_FLAG`), read for the chime.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::BGCHECK_Y_MIN;
use eng_collision::math3d::{Sphere16, vec3f_dist_sq};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, scaled_step_to_s, sin_s, smooth_step_to_f, step_to_f, step_to_s, vec3f_yaw};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::collision_check::*;
use oot_game::item::GI_MAX;
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP_INTERP, Anim, SkelAnimeStd};

use crate::obj_makekinsuta::{self, ACTOR_OBJ_MAKEKINSUTA, ObjMakekinsuta};

/// `ACTOR_EN_INSECT` (`actor_table.h`: 0x0020).
pub const ACTOR_EN_INSECT: i16 = 0x0020;
/// `OBJECT_GAMEPLAY_KEEP` (`object_table.h`: 0x0001).
pub const OBJECT: &str = "gameplay_keep";
/// `gBugSkel` (`gameplay_keep.xml`).
pub const SKEL: &str = "gBugSkel";
/// `gBugCrawlAnim`.
pub const CRAWL_ANIM: &str = "gBugCrawlAnim";

/// `En_Insect_Profile`: `ACTORCAT_ITEMACTION`, no flags, `OBJECT_GAMEPLAY_KEEP`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_INSECT, name: "En_Insect", category: ACTORCAT_ITEMACTION, flags: 0, object: OBJECT, schema: &SCHEMA };

// `EnInsectType` (`z_en_insect.h`).
pub const INSECT_TYPE_PERMANENT: i16 = 0;
pub const INSECT_TYPE_SPAWNED: i16 = 1;
pub const INSECT_TYPE_FIRST_DROPPED: i16 = 2;
/// An additional bug spawned by dropping one from a bottle.
pub const INSECT_TYPE_EXTRA_DROPPED: i16 = 3;

/// `PARAMS_GET_U(this->actor.params, 0, 2)`: the type.
pub const TYPE: Field = Field::params(
    "type",
    0,
    2,
    Kind::Named(&[(0, "INSECT_TYPE_PERMANENT"), (1, "INSECT_TYPE_SPAWNED"), (2, "INSECT_TYPE_FIRST_DROPPED"), (3, "INSECT_TYPE_EXTRA_DROPPED")]),
    "an Obj_Mure group's (0), from under a rock or a bush (1: short-lived), let out of a bottle (2: two more with it, 3)",
);

/// En_Insect's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE],
    flags: &[FlagUse::new(Flag::Other("GET_GS_FLAGS(soil index - 1) & soil flag")).reads().note("type 2 heading for an Obj_Makekinsuta: no chime once that soil's token is taken")],
    todo: false,
    objects: &[],
    texts: &[],
};

// `insectFlags` (`z_en_insect.h`).
/// Gravity, the floor and the water.
pub const INSECT_FLAG_0: u16 = 1 << 0;
/// The walls.
pub const INSECT_FLAG_1: u16 = 1 << 1;
pub const INSECT_FLAG_IS_SHORT_LIVED: u16 = 1 << 2;
/// Set when the bug is about to disappear and can no longer be caught.
pub const INSECT_FLAG_UNCATCHABLE: u16 = 1 << 3;
pub const INSECT_FLAG_FOUND_SOIL: u16 = 1 << 4;
pub const INSECT_FLAG_SOIL_CLOSE: u16 = 1 << 5;
/// Set when the landing sound has been played.
pub const INSECT_FLAG_DROPPED_HAS_LANDED: u16 = 1 << 6;
/// The first dropped bug's chime done.
pub const INSECT_FLAG_7: u16 = 1 << 7;
pub const INSECT_FLAG_CRAWLING: u16 = 1 << 8;

/// `sInitInsectFlags`: by type.
pub const INIT_INSECT_FLAGS: [u16; 4] = [
    0,
    INSECT_FLAG_0 | INSECT_FLAG_IS_SHORT_LIVED,
    INSECT_FLAG_0 | INSECT_FLAG_1 | INSECT_FLAG_IS_SHORT_LIVED,
    INSECT_FLAG_0 | INSECT_FLAG_1 | INSECT_FLAG_IS_SHORT_LIVED,
];

/// `UPDBGCHECKINFO_FLAG_6` (`actor.h`: 1 << 6): no ripples on touching water.
const UPDBGCHECKINFO_FLAG_6: u32 = 1 << 6;

// The sounds.
/// `NA_SE_EN_MUSI_LAND` (`sfx/enemybank_table.h`: 0x39A0).
pub const NA_SE_EN_MUSI_LAND: u16 = 0x39A0;
/// `NA_SE_EN_MUSI_SINK` (`sfx/enemybank_table.h`: 0x39E9).
pub const NA_SE_EN_MUSI_SINK: u16 = 0x39E9;
/// `NA_SE_EN_MUSI_WALK` (`sfx/enemybank_table.h`: 0x39F0).
pub const NA_SE_EN_MUSI_WALK: u16 = 0x39F0;
pub use oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR;

/// `sColliderElementsInit`: one sphere of 5 at the root, pushing only.
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    limb: 0,
    model_sphere: Sphere16 { center: [0, 0, 0], radius: 5 },
    scale: 100,
}];

/// `sColliderJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_PLAYER | OC1_TYPE_1, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// The overlay's statics (its `.data`, reloaded with the overlay).
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Statics {
    /// `D_80A7DEB0`: the dropped bugs' homing, 0 to 1 by 1/300 a dropped bug's update; zeroed
    /// when a dropped bug finds soil.
    pub d_80a7deb0: f32,
    /// `sCaughtCount`: the bugs offered to the bottle this frame (zeroed by every bug's draw).
    pub caught_count: i16,
    /// `sDroppedCount`: the dropped bugs about.
    pub dropped_count: i16,
}

fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_EN_INSECT)
}

/// The overlay's statics as they stand (zero before any bug).
pub fn overlay_statics(play: &PlayState) -> Statics {
    play.overlay_statics.get(&ACTOR_EN_INSECT).and_then(|s| s.downcast_ref::<Statics>()).copied().unwrap_or_default()
}

/// The bug (`gBugSkel`).
pub const BAKE: &str = "En_Insect/skel";

/// The mesh `EnInsect_Draw` draws.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake { name: BAKE.into(), object: OBJECT.into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() } }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    SlowDown,
    Crawl,
    RunFromPlayer,
    Caught,
    Dig,
    WalkOnWater,
    Drown,
    Dropped,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::SlowDown => "EnInsect_SlowDown",
            Action::Crawl => "EnInsect_Crawl",
            Action::RunFromPlayer => "EnInsect_RunFromPlayer",
            Action::Caught => "EnInsect_Caught",
            Action::Dig => "EnInsect_Dig",
            Action::WalkOnWater => "EnInsect_WalkOnWater",
            Action::Drown => "EnInsect_Drown",
            Action::Dropped => "EnInsect_Dropped",
        }
    }
}

pub struct EnInsect {
    pub actor: Actor,
    pub collider: ColliderJntSph,
    pub skel: SkelAnimeStd,
    pub skeleton: Option<Arc<Skeleton>>,
    crawl_anim: Option<Anim>,
    pub action: Action,
    pub insect_flags: u16,
    /// `unk_316`: on the water the heading's turn a frame; dropped, the spin while falling.
    pub unk_316: i16,
    /// `unk_318`: on the water the body's turn a frame.
    pub unk_318: i16,
    pub action_timer: i16,
    pub life_timer: i16,
    pub crawl_sound_delay: i16,
    /// `soilActor`: the `Obj_Makekinsuta` a dropped bug heads for.
    pub soil_actor: Option<ActorHandle>,
    /// `unk_324`: dropped, the speed it steps to.
    pub unk_324: f32,
    /// `unk_328`: dropped, the heading it turns to.
    pub unk_328: i16,
    /// `unk_32A`: the first dropped bug's frames to its chime.
    pub unk_32a: u8,
    /// `actor.draw` isn't `NULL` (`EnInsect_SetupCaught` clears it).
    pub drawn: bool,
}

/// `CLAMP(x, min, max)` (`z_math.h`).
fn clamp<T: PartialOrd>(x: T, min: T, max: T) -> T {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

/// `Math3D_Dist2DSq`.
fn dist_2d_sq(x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    let (dx, dy) = (x0 - x1, y0 - y1);
    dx * dx + dy * dy
}

/// `EnInsect_XZDistanceSquared`.
pub fn xz_distance_squared(v1: Vec3, v2: Vec3) -> f32 {
    (v1.x - v2.x) * (v1.x - v2.x) + (v1.z - v2.z) * (v1.z - v2.z)
}

fn is_dropped(ty: i16) -> bool {
    ty == INSECT_TYPE_FIRST_DROPPED || ty == INSECT_TYPE_EXTRA_DROPPED
}

impl EnInsect {
    /// `EnInsect_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut skeleton = None;
        let mut crawl_anim = None;
        if let Some(a) = play.assets.clone() {
            skeleton = a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Insect: {e:#}")).ok();
            crawl_anim = a.animation(OBJECT, CRAWL_ANIM).map_err(|e| log::error!("En_Insect: {e:#}")).ok();
        }
        let mut this = EnInsect {
            collider: ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS),
            // SkelAnime_Init(.., &gBugSkel, &gBugCrawlAnim, .., 24).
            skel: SkelAnimeStd::init_flex(skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(24), crawl_anim.clone()),
            skeleton,
            crawl_anim,
            action: Action::SlowDown,
            insect_flags: 0,
            unk_316: 0,
            unk_318: 0,
            action_timer: 0,
            life_timer: 0,
            crawl_sound_delay: 0,
            soil_actor: None,
            unk_324: 0.0,
            unk_328: 0,
            unk_32a: 0,
            drawn: true,
            actor,
        };
        // sInitChain.
        let a = &mut this.actor;
        a.scale = Vec3::splat(0.01);
        a.culling_volume_distance = 700.0;
        a.culling_volume_scale = 20.0;
        a.culling_volume_downward = 600.0;
        // EnInsect_InitFlags.
        let ty = this.ty();
        this.insect_flags = INIT_INSECT_FLAGS[ty as usize];

        this.actor.col_chk_info.mass = 30;
        if this.insect_flags & INSECT_FLAG_0 != 0 {
            this.actor.gravity = -0.2;
            this.actor.min_velocity_y = -2.0;
        }
        if this.insect_flags & INSECT_FLAG_IS_SHORT_LIVED != 0 {
            this.life_timer = play.rand.s16_offset(200, 40);
            this.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        }

        if is_dropped(ty) {
            if this.try_find_nearby_soil(play) {
                this.insect_flags |= INSECT_FLAG_FOUND_SOIL;
                statics(play).d_80a7deb0 = 0.0;
            }
            if ty == INSECT_TYPE_FIRST_DROPPED {
                this.actor.world_rot.z = 0;
                this.actor.shape_rot.z = this.actor.world_rot.z;
                let (pos, r) = (this.actor.world_pos, this.actor.shape_rot);
                for _ in 0..2 {
                    if let Err(e) = play.actor_spawn(ACTOR_EN_INSECT, pos, [r.x, r.y, r.z], INSECT_TYPE_EXTRA_DROPPED) {
                        log::warn!("En_Insect: an extra bug not spawned: {e:?}");
                    }
                }
            }
            this.setup_dropped(play);
            let s = statics(play);
            s.dropped_count = s.dropped_count.wrapping_add(1);
        } else {
            let rand = play.rand.zero_one();
            if rand < 0.3 {
                this.setup_slow_down(play);
            } else if rand < 0.4 {
                this.setup_crawl(play);
            } else {
                this.setup_run_from_player(play);
            }
        }
        Box::new(this)
    }

    /// `PARAMS_GET_U(this->actor.params, 0, 2)`.
    pub fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    /// `EnInsect_SetCrawlAnim`: `gBugCrawlAnim` from frame 0, looped.
    fn set_crawl_anim(&mut self) {
        if let Some(a) = self.crawl_anim.clone() {
            self.skel.change(a, 1.0, 0.0, 0.0, ANIMMODE_LOOP_INTERP, 0.0);
        }
    }

    fn player_pos(play: &PlayState) -> Vec3 {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO)
    }

    /// `EnInsect_InBottleRange`.
    fn in_bottle_range(&self, play: &PlayState) -> bool {
        if self.actor.xz_dist_to_player < 32.0 {
            let p = Self::player_pos(play);
            let yaw = self.actor.yaw_towards_player.wrapping_add(i16::MIN);
            let pos = Vec3::new(sin_s(yaw) * 16.0 + p.x, p.y, cos_s(yaw) * 16.0 + p.z);
            // @bug (game): always met once the first test is (see the module docs).
            if xz_distance_squared(pos, self.actor.world_pos) <= 20.0 * 20.0 {
                return true;
            }
        }
        false
    }

    /// `EnInsect_TryFindNearbySoil`: the nearest `Obj_Makekinsuta` in its room within 80 across.
    fn try_find_nearby_soil(&mut self, play: &PlayState) -> bool {
        let mut best_distance_sq = 80.0f32 * 80.0;
        let mut ret = false;
        self.soil_actor = None;
        for &h in play.actors.category(ACTORCAT_ITEMACTION) {
            let Some(current) = play.actors.actor(h) else { continue };
            if current.id == ACTOR_OBJ_MAKEKINSUTA {
                let current_distance_sq = dist_2d_sq(self.actor.world_pos.x, self.actor.world_pos.z, current.world_pos.x, current.world_pos.z);
                if current_distance_sq < best_distance_sq && current.room == self.actor.room {
                    ret = true;
                    best_distance_sq = current_distance_sq;
                    self.soil_actor = Some(h);
                }
            }
        }
        ret
    }

    /// The soil's position and params (`soilActor`; `NULL` once it's gone).
    fn soil(&self, play: &PlayState) -> Option<(Vec3, i16)> {
        // schema: the soil's params whole, read through Obj_Makekinsuta's fields
        self.soil_actor.and_then(|h| play.actors.actor(h)).map(|a| (a.world_pos, a.params))
    }

    /// `EnInsect_UpdateCrawlSfx`.
    fn update_crawl_sfx(&mut self, play: &mut PlayState) {
        if self.crawl_sound_delay > 0 {
            self.crawl_sound_delay -= 1;
            return;
        }
        audio_play_actor_sfx2(play, NA_SE_EN_MUSI_WALK);
        let play_speed = if self.skel.play_speed < 0.1 { 0.1 } else { self.skel.play_speed };
        self.crawl_sound_delay = (3.0 / play_speed) as i32 as i16;
        if self.crawl_sound_delay < 2 {
            self.crawl_sound_delay = 2;
        }
    }

    /// The test `EnInsect_SlowDown` and `EnInsect_Crawl` end with: dig, walk on the water, or
    /// run from Link.
    fn dig_water_or_run(&mut self, play: &mut PlayState) {
        let ty = self.ty();
        let dropped_count = statics(play).dropped_count;
        if (self.insect_flags & INSECT_FLAG_IS_SHORT_LIVED != 0 && self.life_timer <= 0)
            || (is_dropped(ty) && self.insect_flags & INSECT_FLAG_0 != 0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && dropped_count >= 4)
        {
            self.setup_dig(play);
        } else if self.insect_flags & INSECT_FLAG_0 != 0 && self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH != 0 {
            self.setup_walk_on_water(play);
        } else if self.actor.xz_dist_to_player < 40.0 {
            self.setup_run_from_player(play);
        }
    }

    /// `EnInsect_SetupSlowDown`.
    fn setup_slow_down(&mut self, play: &mut PlayState) {
        self.action_timer = play.rand.s16_offset(5, 35);
        self.set_crawl_anim();
        self.action = Action::SlowDown;
        self.insect_flags |= INSECT_FLAG_CRAWLING;
    }

    /// `EnInsect_SlowDown`.
    fn slow_down(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 0.1, 0.5, 0.0);
        let play_speed = (play.rand.zero_one() * 0.8) + (self.actor.speed_xz * 1.2);
        self.skel.play_speed = clamp(play_speed, 0.0, 1.9);
        self.skel.update();
        self.actor.shape_rot.y = self.actor.world_rot.y;
        if self.action_timer <= 0 {
            self.setup_crawl(play);
        }
        self.dig_water_or_run(play);
    }

    /// `EnInsect_SetupCrawl`.
    fn setup_crawl(&mut self, play: &mut PlayState) {
        self.action_timer = play.rand.s16_offset(10, 45);
        self.set_crawl_anim();
        self.action = Action::Crawl;
        self.insect_flags |= INSECT_FLAG_CRAWLING;
    }

    /// `EnInsect_Crawl`.
    fn crawl(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.speed_xz, 1.5, 0.1, 0.5, 0.0);
        let child_pos = self.actor.child.filter(|&c| Some(c) != play.cur_actor).and_then(|c| play.actors.actor(c)).map(|a| a.world_pos);
        if xz_distance_squared(self.actor.world_pos, self.actor.home_pos) > 1600.0 || self.action_timer < 4 {
            let yaw = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
            scaled_step_to_s(&mut self.actor.world_rot.y, yaw, 2000);
        } else if let Some(c) = child_pos {
            let yaw = vec3f_yaw(self.actor.world_pos, c);
            scaled_step_to_s(&mut self.actor.world_rot.y, yaw, 2000);
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
        self.skel.play_speed = clamp(self.actor.speed_xz * 1.4, 0.7, 1.9);
        self.skel.update();
        if self.action_timer <= 0 {
            self.setup_slow_down(play);
        }
        self.dig_water_or_run(play);
    }

    /// `EnInsect_SetupRunFromPlayer`.
    fn setup_run_from_player(&mut self, play: &mut PlayState) {
        self.action_timer = play.rand.s16_offset(10, 40);
        self.set_crawl_anim();
        self.action = Action::RunFromPlayer;
        self.insect_flags |= INSECT_FLAG_CRAWLING;
    }

    /// `EnInsect_RunFromPlayer`.
    fn run_from_player(&mut self, play: &mut PlayState) {
        let player_is_close = self.actor.xz_dist_to_player < 40.0;
        smooth_step_to_f(&mut self.actor.speed_xz, 1.8, 0.1, 0.5, 0.0);
        if xz_distance_squared(self.actor.world_pos, self.actor.home_pos) > 25600.0 || self.action_timer < 4 {
            let yaw = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
            scaled_step_to_s(&mut self.actor.world_rot.y, yaw, 2000);
        } else if player_is_close {
            let frames = play.state_frames as i16;
            let mut yaw = self.actor.yaw_towards_player.wrapping_add(i16::MIN);
            if frames & 0x10 != 0 {
                if frames & 0x20 != 0 {
                    yaw = yaw.wrapping_add(0x2000);
                }
            } else if frames & 0x20 != 0 {
                yaw = yaw.wrapping_sub(0x2000);
            }
            scaled_step_to_s(&mut self.actor.world_rot.y, yaw, 2000);
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
        self.skel.play_speed = clamp(self.actor.speed_xz * 1.6, 0.8, 1.9);
        self.skel.update();
        if self.action_timer <= 0 || !player_is_close {
            self.setup_slow_down(play);
        } else if self.insect_flags & INSECT_FLAG_0 != 0 && self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH != 0 {
            self.setup_walk_on_water(play);
        }
    }

    /// `EnInsect_SetupCaught`: undrawn, tiny and still for 200 frames.
    fn setup_caught(&mut self) {
        self.action_timer = 200;
        self.actor.scale = Vec3::splat(0.001);
        self.drawn = false;
        self.actor.speed_xz = 0.0;
        self.set_crawl_anim();
        self.skel.play_speed = 0.3;
        self.action = Action::Caught;
        self.insect_flags &= !INSECT_FLAG_CRAWLING;
    }

    /// `EnInsect_Caught`.
    fn caught(&mut self, play: &mut PlayState) {
        if self.action_timer == 20 && self.insect_flags & INSECT_FLAG_IS_SHORT_LIVED == 0 {
            self.drawn = true;
        } else if self.action_timer == 0 {
            if self.insect_flags & INSECT_FLAG_IS_SHORT_LIVED != 0 {
                self.actor.kill();
            } else {
                self.actor.scale = Vec3::splat(0.01);
                self.setup_slow_down(play);
            }
        } else if self.action_timer < 20 {
            let s = self.actor.scale.x + 0.001;
            self.actor.scale = Vec3::splat(if s > 0.01 { 0.01 } else { s });
            self.skel.update();
        }
    }

    /// `EnInsect_SetupDig`.
    fn setup_dig(&mut self, play: &mut PlayState) {
        self.action_timer = 60;
        self.set_crawl_anim();
        self.skel.play_speed = 1.9;
        audio_play_actor_sfx2(play, NA_SE_EN_MUSI_SINK);
        self.actor.home_pos = self.actor.world_pos;
        self.action = Action::Dig;
        self.insect_flags &= !INSECT_FLAG_CRAWLING;
        self.insect_flags |= INSECT_FLAG_UNCATCHABLE;
    }

    /// `EnInsect_Dig`.
    fn dig(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 0.1, 0.5, 0.0);
        step_to_s(&mut self.actor.shape_rot.x, 10922, 352);
        let s = self.actor.scale.x - 0.0002;
        self.actor.scale = Vec3::splat(if s < 0.001 { 0.001 } else { s });
        self.actor.shape_y_offset -= 0.8;
        self.actor.world_pos.x = play.rand.zero_one() + self.actor.home_pos.x - 0.5;
        self.actor.world_pos.z = play.rand.zero_one() + self.actor.home_pos.z - 0.5;
        self.skel.update();

        if self.action_timer > 20 && play.rand.zero_one() < 0.1 {
            let velocity = Vec3::new(sin_s(self.actor.shape_rot.y) * -0.6, sin_s(self.actor.shape_rot.x) * 0.6, cos_s(self.actor.shape_rot.y) * -0.6);
            // func_800286CC(play, &pos, &velocity, &accel, Rand_ZeroOne() * 5.0f + 8.0f, ditto):
            // the two arguments alike, so their order of evaluation doesn't matter.
            let scale = (play.rand.zero_one() * 5.0 + 8.0) as i32 as i16;
            let scale_step = (play.rand.zero_one() * 5.0 + 8.0) as i32 as i16;
            let pos = self.actor.world_pos;
            play.with_ss(|ss| ss.func_800286cc(pos, velocity, Vec3::ZERO, scale, scale_step));
        }

        if self.action_timer <= 0 {
            if self.insect_flags & INSECT_FLAG_FOUND_SOIL != 0
                && let Some((soil_pos, _)) = self.soil(play)
                && vec3f_dist_sq(soil_pos, self.actor.world_pos) < 8.0 * 8.0
                && let Some(soil) = self.soil_actor.and_then(|h| play.actors.downcast_mut::<ObjMakekinsuta>(h))
            {
                soil.unk_152 = 1;
            }
            self.actor.kill();
        }
    }

    /// `EnInsect_SetupWalkOnWater`.
    fn setup_walk_on_water(&mut self, play: &mut PlayState) {
        self.action_timer = play.rand.s16_offset(120, 50);
        self.set_crawl_anim();
        self.unk_318 = 0;
        self.unk_316 = 0;
        self.action = Action::WalkOnWater;
        self.insect_flags &= !INSECT_FLAG_CRAWLING;
    }

    /// `EnInsect_WalkOnWater`.
    fn walk_on_water(&mut self, play: &mut PlayState) {
        let ty = self.ty();
        if self.action_timer > 80 {
            step_to_f(&mut self.actor.speed_xz, 0.6, 0.08);
        } else {
            step_to_f(&mut self.actor.speed_xz, 0.0, 0.02);
        }
        self.actor.velocity.y = 0.0;
        self.actor.world_pos.y += self.actor.y_dist_to_water;
        self.skel.play_speed = clamp(self.action_timer as f32 * 0.018, 0.1, 1.9);
        self.skel.update();

        if self.action_timer > 80 {
            self.unk_316 = self.unk_316.wrapping_add(play.rand.s16_offset(-50, 100));
            self.unk_318 = self.unk_318.wrapping_add(play.rand.s16_offset(-300, 600));
        }

        let temp_v1 = (self.skel.play_speed * 200.0) as i32 as i16;
        self.unk_316 = clamp(self.unk_316, temp_v1.wrapping_neg(), temp_v1);
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(self.unk_316);

        let temp_v1 = (self.skel.play_speed * 1000.0) as i32 as i16;
        self.unk_318 = clamp(self.unk_318, temp_v1.wrapping_neg(), temp_v1);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.unk_318);

        scaled_step_to_s(&mut self.actor.world_rot.x, 0, 3000);
        self.actor.shape_rot.x = self.actor.world_rot.x;

        if play.rand.zero_one() < 0.03 {
            let ripple_point = Vec3::new(self.actor.world_pos.x, self.actor.world_pos.y + self.actor.y_dist_to_water, self.actor.world_pos.z);
            let (mut ss, _) = play.ss();
            ss.g_ripple_spawn(ripple_point, 20, 100, 4);
            ss.g_ripple_spawn(ripple_point, 40, 200, 8);
        }

        let dropped_count = statics(play).dropped_count;
        if self.action_timer <= 0 || (self.insect_flags & INSECT_FLAG_IS_SHORT_LIVED != 0 && self.life_timer <= 0) || (is_dropped(ty) && self.insect_flags & INSECT_FLAG_0 != 0 && dropped_count >= 4) {
            self.setup_drown();
        } else if self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH == 0 {
            if self.insect_flags & INSECT_FLAG_FOUND_SOIL != 0 {
                self.setup_dropped(play);
            } else {
                self.setup_slow_down(play);
            }
        }
    }

    /// `EnInsect_SetupDrown`.
    fn setup_drown(&mut self) {
        self.action_timer = 100;
        self.set_crawl_anim();
        self.actor.velocity.y = 0.0;
        self.actor.speed_xz = 0.0;
        self.actor.min_velocity_y = -0.8;
        self.actor.gravity = -0.04;
        self.insect_flags &= !(INSECT_FLAG_0 | INSECT_FLAG_1);
        self.action = Action::Drown;
        self.insect_flags &= !INSECT_FLAG_CRAWLING;
        self.insect_flags |= INSECT_FLAG_UNCATCHABLE;
    }

    /// `EnInsect_Drown`.
    fn drown(&mut self, play: &mut PlayState) {
        self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_sub(500);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(200);
        let s = self.actor.scale.x - 0.00005;
        self.actor.scale = Vec3::splat(if s < 0.001 { 0.001 } else { s });

        if self.actor.y_dist_to_water > 5.0 && self.actor.y_dist_to_water < 30.0 && play.rand.zero_one() < 0.3 {
            let pos = self.actor.world_pos;
            let scale = (play.rand.zero_one() * 0.04) + 0.02;
            play.ss().0.bubble_spawn(pos, -5.0, 5.0, 5.0, scale);
        }

        if self.action_timer <= 0 {
            self.actor.kill();
        }
    }

    /// `EnInsect_SetupDropped`.
    fn setup_dropped(&mut self, play: &mut PlayState) {
        self.set_crawl_anim();
        self.action_timer = 100;
        self.unk_324 = 1.5;
        self.unk_328 = (play.rand.zero_one() * (0xFFFF as f32 + 0.5)) as i32 as i16;
        self.unk_316 = ((play.rand.zero_one() - 0.5) * 1500.0) as i32 as i16;
        self.actor.world_rot.y = (play.rand.zero_one() * (0xFFFF as f32 + 0.5)) as i32 as i16;
        self.actor.scale = Vec3::splat(0.003);
        self.action = Action::Dropped;
        self.insect_flags |= INSECT_FLAG_CRAWLING;
    }

    /// `EnInsect_Dropped`.
    fn dropped(&mut self, play: &mut PlayState) {
        let mut sp50 = false;
        let ty = self.ty();
        let soil = self.soil(play);

        let distance_sq = match soil {
            Some((pos, _)) => vec3f_dist_sq(self.actor.world_pos, pos),
            None => {
                if self.insect_flags & INSECT_FLAG_FOUND_SOIL != 0 {
                    log::warn!("En_Insect: warning: target Actor is NULL (../z_en_mushi.c 1046)");
                }
                40.0
            }
        };

        let d = {
            let s = statics(play);
            s.d_80a7deb0 += 0.99999994f32 / 300.0;
            if s.d_80a7deb0 > 1.0 {
                s.d_80a7deb0 = 1.0;
            }
            s.d_80a7deb0
        };

        let phi_f2 = if d > 0.999 {
            0.0
        } else if distance_sq > 30.0 * 30.0 {
            ((1.1 - d) * 100.0) + 20.0
        } else {
            (1.0 - d) * 10.0
        };

        if let Some((soil_pos, _)) = soil
            && play.rand.zero_one() < 0.07
        {
            self.actor.home_pos.x = (play.rand.zero_one() - 0.5) * phi_f2 + soil_pos.x;
            self.actor.home_pos.y = soil_pos.y;
            self.actor.home_pos.z = (play.rand.zero_one() - 0.5) * phi_f2 + soil_pos.z;
        }

        if d > 0.999 {
            self.unk_328 = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
            self.unk_324 = play.rand.zero_one() * 0.6 + 0.6;
        } else if play.rand.zero_one() < 0.07 {
            if self.unk_324 > 1.0 {
                self.unk_324 = 0.1;
            } else {
                self.unk_324 = play.rand.zero_one() * 0.8 + 1.0;
            }
            let sp34 = clamp(1.3 - d, 0.0, 1.0);
            let sp38 = ((play.rand.zero_one() - 0.5) * 65535.0 * sp34) as i32 as i16;
            self.unk_328 = vec3f_yaw(self.actor.world_pos, self.actor.home_pos).wrapping_add(sp38);
        }

        let s = self.actor.scale.x + 0.0008;
        self.actor.scale = Vec3::splat(if s > 0.01 { 0.01 } else { s });

        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            smooth_step_to_f(&mut self.actor.speed_xz, self.unk_324, 0.1, 0.5, 0.0);
            scaled_step_to_s(&mut self.actor.world_rot.y, self.unk_328, 2000);
            sp50 = scaled_step_to_s(&mut self.actor.world_rot.x, 0, 2000);
            self.actor.shape_rot.y = self.actor.world_rot.y;
            self.actor.shape_rot.x = self.actor.world_rot.x;
        } else {
            smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 0.1, 0.5, 0.0);
            self.actor.speed_xz += (play.rand.zero_one() - 0.5) * 0.14;
            self.actor.velocity.y += play.rand.zero_one() * 0.12;
            self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(self.unk_316);
            self.actor.shape_rot.y = self.actor.world_rot.y;
            self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_sub(2000);
        }

        let phi_f2 = play.rand.zero_one() * 0.5 + self.actor.speed_xz * 1.3;
        self.skel.play_speed = if phi_f2 < 0.0 { 0.0 } else if phi_f2 > 1.9 { 1.9 } else { phi_f2 };

        self.skel.update();
        if self.insect_flags & INSECT_FLAG_DROPPED_HAS_LANDED == 0 && self.insect_flags & INSECT_FLAG_0 != 0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_MUSI_LAND);
            self.insect_flags |= INSECT_FLAG_DROPPED_HAS_LANDED;
        }

        if ty == INSECT_TYPE_FIRST_DROPPED && self.insect_flags & INSECT_FLAG_FOUND_SOIL != 0 && self.insect_flags & INSECT_FLAG_7 == 0 {
            if self.unk_32a >= 15 {
                if let Some((_, soil_params)) = soil {
                    let index = obj_makekinsuta::GS_FLAGS_INDEX.get(soil_params) as i32 - 1;
                    let flag = obj_makekinsuta::GS_FLAG.get(soil_params) as u32;
                    // @bug (game): index -1 reads before gsFlags (no soil has it): no flag here.
                    let taken = index >= 0 && play.save.get_gs_flags(index) & flag != 0;
                    if !taken {
                        play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
                    }
                }
                self.insect_flags |= INSECT_FLAG_7;
            } else {
                self.unk_32a += 1;
            }
        }

        let dropped_count = statics(play).dropped_count;
        if self.insect_flags & INSECT_FLAG_0 != 0 && self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH != 0 {
            self.setup_walk_on_water(play);
        } else if self.insect_flags & INSECT_FLAG_FOUND_SOIL != 0 {
            if distance_sq < 3.0 * 3.0 {
                self.setup_dig(play);
            } else if self.action_timer <= 0
                || self.life_timer <= 0
                || (self.insect_flags & INSECT_FLAG_0 != 0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && dropped_count >= 4 && is_dropped(ty))
            {
                self.setup_dig(play);
            } else if distance_sq < 30.0 * 30.0 {
                self.life_timer = self.life_timer.wrapping_add(1);
                self.insect_flags |= INSECT_FLAG_SOIL_CLOSE;
            } else {
                self.action_timer = 100;
            }
        } else if sp50 {
            self.setup_slow_down(play);
        } else if is_dropped(ty) && self.insect_flags & INSECT_FLAG_0 != 0 && self.life_timer <= 0 && self.action_timer <= 0 && self.actor.floor_height < BGCHECK_Y_MIN + 10.0 {
            log::warn!("En_Insect: BG missing? To do Actor_delete (../z_en_mushi.c 1197)");
            self.actor.kill();
        }
    }
}

impl ActorImpl for EnInsect {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnInsect_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if let Some(c) = self.actor.child
            && play.actors.actor(c).is_none_or(|a| a.killed)
            && Some(c) != play.cur_actor
        {
            self.actor.child = None;
        }
        if self.action_timer > 0 {
            self.action_timer -= 1;
        }
        if self.life_timer > 0 {
            self.life_timer -= 1;
        }

        match self.action {
            Action::SlowDown => self.slow_down(play),
            Action::Crawl => self.crawl(play),
            Action::RunFromPlayer => self.run_from_player(play),
            Action::Caught => self.caught(play),
            Action::Dig => self.dig(play),
            Action::WalkOnWater => self.walk_on_water(play),
            Action::Drown => self.drown(play),
            Action::Dropped => self.dropped(play),
        }

        if self.actor.killed {
            return;
        }
        self.actor.move_forward();
        if self.insect_flags & INSECT_FLAG_CRAWLING != 0 {
            if self.insect_flags & INSECT_FLAG_0 != 0 {
                if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                    self.update_crawl_sfx(play);
                }
            } else {
                self.update_crawl_sfx(play);
            }
        }

        let mut tmp = 0;
        if self.insect_flags & INSECT_FLAG_0 != 0 {
            tmp = UPDBGCHECKINFO_FLAG_2;
        }
        if self.insect_flags & INSECT_FLAG_1 != 0 {
            tmp |= UPDBGCHECKINFO_FLAG_0;
        }
        if tmp != 0 {
            tmp |= UPDBGCHECKINFO_FLAG_6;
            self.actor.update_bg_check_info(&play.col, 8.0, 5.0, 0.0, tmp);
        }

        if oot_game::get_item::actor_has_parent(&self.actor) {
            self.actor.parent = None;
            if is_dropped(self.ty()) {
                self.actor.kill();
            } else {
                self.setup_caught();
            }
        } else if self.actor.xz_dist_to_player < 50.0 && self.action != Action::Caught {
            if self.insect_flags & INSECT_FLAG_SOIL_CLOSE == 0 && self.life_timer < 180 {
                play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            }
            // GI_MAX in this case allows the player to catch the actor in a bottle.
            if self.insect_flags & INSECT_FLAG_UNCATCHABLE == 0 && statics(play).caught_count < 4 && self.in_bottle_range(play) {
                let me = self.actor.clone();
                if oot_game::get_item::offer_get_item_range(play, &me, GI_MAX, 60.0, 30.0) {
                    let s = statics(play);
                    s.caught_count = s.caught_count.wrapping_add(1);
                }
            }
        }

        self.actor.set_focus(0.0);
    }
    /// `EnInsect_Destroy`; then, the last bug gone, the overlay's unload (`Actor_Delete`'s
    /// `Actor_FreeOverlay`): its statics reset as its next load would have them.
    fn destroy(&mut self, play: &mut PlayState) {
        let s = statics(play);
        if is_dropped(self.ty()) && s.dropped_count > 0 {
            s.dropped_count -= 1;
        }
        // This bug is out of the context while it's destroyed: any other still loaded counts.
        let others = play.actors.all().into_iter().any(|h| play.actors.actor(h).is_some_and(|a| a.id == ACTOR_EN_INSECT));
        if !others {
            play.overlay_statics.remove(&ACTOR_EN_INSECT);
        }
    }
    /// The rest of `EnInsect_Draw`: `Collider_UpdateSpheres(0, &collider)` at the actor's
    /// matrix (after `SkelAnime_DrawOpa`), and `sCaughtCount = 0`.
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.drawn {
            let m = actor_draw_matrix(&RenderState::of(&self.actor));
            self.collider.update_spheres(0, &m);
            statics(play).caught_count = 0;
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.drawn as u32];
        rs
    }
    /// `EnInsect_Draw`: `Gfx_SetupDL_25Opa`, `SkelAnime_DrawOpa`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.first().copied().unwrap_or(0) == 0 {
            return;
        }
        if let (Some(skeleton), Some(joints)) = (&self.skeleton, &rs.joints) {
            let bones = skeleton.pose(joints);
            out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: actor_draw_matrix(rs), bones, params: Default::default() });
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
