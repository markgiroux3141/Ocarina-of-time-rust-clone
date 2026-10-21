//! `Bg_Spot06_Objects` (`ovl_Bg_Spot06_Objects/z_bg_spot06_objects.c`): Lake Hylia's objects
//! (`object_spot06_objects`), by type (`LakeHyliaObjectsType`, the params' high byte; the low byte
//! the switch flag).
//!
//! - **The lake's water** (`LHO_WATER_PLANE`, 0x02FF at (-1057, -1313, 5010)): scale 1, never
//!   culled. A child's lake (and an adult's once the Water Temple is cleared,
//!   `EVENTCHKINF_RESTORED_LAKE_HYLIA`) is full: `lakeHyliaWaterLevel` 0, nothing to do; the
//!   scene's water boxes stay as the scene has them, the main ones at `WATER_LEVEL_RAISED` (-1313).
//!   Drawn translucent (`BgSpot06Objects_DrawLakeHyliaWater`, `Gfx_SetupDL_25Xlu`): two scrolls by
//!   `play->state.frames` on segments 8 and 9 (`Gfx_TwoTexScroll`), env colour (255, 255, 255,
//!   128), and `gLakeHyliaHighWaterDL` (`gLakeHyliaLowWaterDL` when the level is under -680 outside
//!   a cutscene layer). Without it a child's lake shows no water.
//! - **The Water Temple's gate** (`LHO_WATER_TEMPLE_ENTRACE_GATE`, 0x001F at the lake's bottom): a
//!   DynaPoly actor (`gLakeHyliaWaterTempleGateCol`) waiting for its switch flag
//!   (`BgSpot06Objects_GateWaitForSwitch`); set, it jumps 3, bubbles (15 tries of
//!   `BgSpot06Objects_GateSpawnBubbles`: on frames divisible by 3, 160 across its face), waits 100
//!   frames (`BgSpot06Objects_GateWaitToOpen`) and rises 0.6 a frame to 120 above home with its
//!   bubbles and `NA_SE_EV_METALDOOR_SLIDE`, then `NA_SE_EV_METALDOOR_STOP`
//!   (`BgSpot06Objects_GateOpen`). An adult with the flag set finds it up already. A child meets it
//!   shut (only the adult's hookshot pulls the lock that sets the flag), or rising on entry once an
//!   adult has set it.
//! - **The gate's lock** (`LHO_WATER_TEMPLE_ENTRANCE_LOCK`, 0x011F): a fish-shaped key with one
//!   hookable sphere (18 at (0, 0, -160), only `DMG_HOOKSHOT`), immovable, pushing everything; a
//!   child's waits for the hookshot (`BgSpot06Objects_LockWait`).
//! - **The ice block** (`LHO_ICE_BLOCK`, 0x03FF on the adult's layers, over the tunnel to Zora's
//!   Domain): a DynaPoly actor (`gLakeHyliaZoraShortcutIceblockCol`) that does nothing, drawn
//!   `gLakeHyliaZoraShortcutIceblockDL`; killed for a child.
//!
//! Ported: the water whole for a child and for an adult's restored lake, the gate whole, the
//! lock's wait, the ice block whole (it does nothing but stand). **Placeholders chosen in the
//! init** (Phase 16, the adult's): the drained lake (an adult before
//! `EVENTCHKINF_RESTORED_LAKE_HYLIA`: the water boxes lowered, `WATER_LEVEL_LOWERED`, and the
//! level rising in `gLakeHyliaRestoredCs`: `BgSpot06Objects_WaterPlaneCutsceneWait`,
//! `_WaterPlaneCutsceneRise`) and the adult's lock (`BgSpot06Objects_LockFloat` with the flag set;
//! the hookshot's pull from `LockWait` without it). The child's lock is never hit (the hookshot is
//! an adult's and isn't ported): the hit's branch (`LockPullOutward`, `LockSwimToSurface`,
//! `LockFloat`, one-point 4120) is the adult's and logs.
//!
//! Params: `switchFlag` bits 0 to 7 (types 0 and 1: the gate's switch, set by the lock's pull;
//! 0x1F in the game's layers, 0xFF in the cutscene layers' copies); `type` bits 8 to 15
//! (`LakeHyliaObjectsType`: 0 the gate, 1 the lock, 2 the water, 3 the ice block); the water and the
//! ice block have 0xFF in the low byte, unread.
//! Flags: reads switch `switchFlag` (types 0 and 1: the gate open); reads
//! EVENTCHKINF_RESTORED_LAKE_HYLIA (type 2: an adult's lake full; type 1, an adult's lock: not
//! ported).

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, step_to_f};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_HOOKSHOT_PULLS_ACTOR, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::collision_check::*;
use oot_game::event_chk_inf;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

/// `ACTOR_BG_SPOT06_OBJECTS` (`actor_table.h`: 0x00D5).
pub const ACTOR_BG_SPOT06_OBJECTS: i16 = 0x00D5;
/// `OBJECT_SPOT06_OBJECTS` (`object_table.h`: 0x006A).
pub const OBJECT: &str = "object_spot06_objects";

/// `Bg_Spot06_Objects_Profile`: `ACTORCAT_PROP`, `ACTOR_FLAG_HOOKSHOT_PULLS_ACTOR`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_BG_SPOT06_OBJECTS, name: "Bg_Spot06_Objects", category: ACTORCAT_PROP, flags: ACTOR_FLAG_HOOKSHOT_PULLS_ACTOR, object: OBJECT, schema: &SCHEMA };

// `LakeHyliaObjectsType`.
pub const LHO_WATER_TEMPLE_ENTRACE_GATE: i16 = 0x0;
pub const LHO_WATER_TEMPLE_ENTRANCE_LOCK: i16 = 0x1;
pub const LHO_WATER_PLANE: i16 = 0x2;
pub const LHO_ICE_BLOCK: i16 = 0x3;

/// `PARAMS_GET_U(thisx->params, 0, 8)`: `switchFlag`.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 8, Kind::SwitchFlag, "the gate's switch: set, the gate opens (the lock's pull sets it)").when("type", &[0, 1]);
/// The low byte of the water and the ice block: stored as `switchFlag`, never read.
pub const UNUSED_LOW: Field = Field::params("unusedSwitchFlag", 0, 8, Kind::Unused, "0xFF in the data; stored as switchFlag, unread").when("type", &[2, 3]);
/// `PARAMS_GET_U(thisx->params, 8, 8)`: the type (`thisx->params` becomes it).
pub const TYPE: Field = Field::params(
    "type",
    8,
    8,
    Kind::Named(&[(0, "LHO_WATER_TEMPLE_ENTRACE_GATE"), (1, "LHO_WATER_TEMPLE_ENTRANCE_LOCK"), (2, "LHO_WATER_PLANE"), (3, "LHO_ICE_BLOCK")]),
    "the Water Temple's gate, its lock, the lake's water, the ice block",
);

/// `EVENTCHKINF_RESTORED_LAKE_HYLIA` (`save.h`: 0x69).
pub const EVENTCHKINF_RESTORED_LAKE_HYLIA: u16 = 0x69;

/// Bg_Spot06_Objects's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG, UNUSED_LOW, TYPE],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().note("the gate opens; an adult finds it open (types 0 and 1)"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_RESTORED_LAKE_HYLIA)).reads().note("an adult's lake full (type 2); the adult's lock's height (type 1, not ported)"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `WATER_LEVEL_RAISED`: the full lake's surface.
pub const WATER_LEVEL_RAISED: i16 = -1313;
/// `WATER_LEVEL_LOWERED` (`WATER_LEVEL_RAISED - 680`): the drained lake's (the adult's).
pub const WATER_LEVEL_LOWERED: i16 = WATER_LEVEL_RAISED - 680;

/// `NA_SE_EV_METALDOOR_SLIDE` (`environmentbank_table.h`: 0x2836).
pub const NA_SE_EV_METALDOOR_SLIDE: u16 = 0x2836;
/// `NA_SE_EV_METALDOOR_STOP` (`environmentbank_table.h`: 0x2837).
pub const NA_SE_EV_METALDOOR_STOP: u16 = 0x2837;

const GATE_COLLISION: &str = "gLakeHyliaWaterTempleGateCol";
const ICE_BLOCK_COLLISION: &str = "gLakeHyliaZoraShortcutIceblockCol";
const GATE_DL: &str = "gLakeHyliaWaterTempleGateDL";
const LOCK_DL: &str = "gLakeHyliaWaterTempleKeyDL";
const ICE_BLOCK_DL: &str = "gLakeHyliaZoraShortcutIceblockDL";

/// `sJntSphElementsInit`: the lock's sphere, hit only by the hookshot (`0x00000080`), hookable.
pub const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0080, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE,
        oc_elem_flags: OCELEM_ON,
    },
    limb: 1,
    model_sphere: Sphere16 { center: [0, 0, -160], radius: 18 },
    scale: 100,
}];

/// `sJntSphInit`.
pub const JNT_SPH_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_JNTSPH };

/// The water's bakes: after `Gfx_SetupDL_25Xlu` (the bake's start), the env colour, the scrolls on
/// segments 8 and 9, the high or the low water.
pub const BAKE_HIGH_WATER: &str = "Bg_Spot06_Objects/high_water";
pub const BAKE_LOW_WATER: &str = "Bg_Spot06_Objects/low_water";
const SEG_SCROLL_8: u8 = 0x08;
const SEG_SCROLL_9: u8 = 0x09;
/// The draw's own `gDPSetEnvColor`, put on a segment the lists don't use.
const SEG_COLOR: u8 = 0x0E;

/// `BgSpot06Objects_DrawLakeHyliaWater`'s segment 8 at `play->state.frames`:
/// `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, -frames, frames, 32, 32, 1, frames, frames, 32, 32)`.
pub fn water_scroll_8(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, frames.wrapping_neg(), frames, 32, 32, 1, frames, frames, 32, 32)
}

/// Segment 9: `Gfx_TwoTexScroll(.., -frames, frames * 6, 32, 32, 1, frames, frames * 6, 32, 32)`.
pub fn water_scroll_9(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, frames.wrapping_neg(), frames.wrapping_mul(6), 32, 32, 1, frames, frames.wrapping_mul(6), 32, 32)
}

pub fn bakes() -> Vec<MeshBake> {
    // G_SETENVCOLOR (0xFB) 255, 255, 255, 128; G_ENDDL.
    let color = vec![(0xFB00_0000u32, u32::from_be_bytes([255, 255, 255, 128])), (0xDF00_0000u32, 0u32)];
    [(BAKE_HIGH_WATER, "gLakeHyliaHighWaterDL"), (BAKE_LOW_WATER, "gLakeHyliaLowWaterDL")]
        .into_iter()
        .map(|(name, dl)| MeshBake {
            name: name.into(),
            object: OBJECT.into(),
            segments: vec![
                (SEG_SCROLL_8, BakeSegment::Dynamic(water_scroll_8(0))),
                (SEG_SCROLL_9, BakeSegment::Dynamic(water_scroll_9(0))),
                (SEG_COLOR, BakeSegment::Commands(color.clone())),
            ],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![(OBJECT.into(), dl.into())]),
        })
        .collect()
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `BgSpot06Objects_DoNothing`.
    DoNothing,
    /// `BgSpot06Objects_GateWaitForSwitch`.
    GateWaitForSwitch,
    /// `BgSpot06Objects_GateWaitToOpen`.
    GateWaitToOpen,
    /// `BgSpot06Objects_GateOpen`.
    GateOpen,
    /// `BgSpot06Objects_LockWait`.
    LockWait,
}

pub struct BgSpot06Objects {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    /// `thisx->params` after the init: the type.
    pub ty: i16,
    pub action: Action,
    pub switch_flag: i16,
    pub timer: i16,
    pub lake_hylia_water_level: f32,
    /// The lock's (type 1's) sphere.
    pub collider: ColliderJntSph,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgSpot06Objects {
    /// `BgSpot06Objects_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let switch_flag = SWITCH_FLAG.get(actor.params);
        let ty = TYPE.get(actor.params);
        let mut this = BgSpot06Objects { actor, bg: BG_ACTOR_MAX, ty, action: Action::DoNothing, switch_flag, timer: 0, lake_hylia_water_level: 0.0, collider: ColliderJntSph::default() };
        // schema: thisx->params = PARAMS_GET_U(thisx->params, 8, 8), the C's write.
        this.actor.params = ty;
        log::debug!("spot06 obj nthisx->arg_data=[{ty}]");
        let adult = play.save.adult;
        match ty {
            LHO_WATER_TEMPLE_ENTRACE_GATE => {
                // sInitChain: scale 0.1.
                this.actor.scale = Vec3::splat(0.1);
                this.set_bg_actor(play, GATE_COLLISION);
                if adult && play.flags.get_switch(this.switch_flag as i32) {
                    this.actor.world_pos.y = this.actor.home_pos.y + 120.0;
                    this.action = Action::DoNothing;
                } else {
                    this.action = Action::GateWaitForSwitch;
                }
                play.col.dyna.set_source(this.bg, source(&this.actor));
            }
            LHO_WATER_TEMPLE_ENTRANCE_LOCK => {
                if adult {
                    // LINK_IS_ADULT && Flags_GetSwitch: the lock floating at the drained or the full
                    // lake's surface (BgSpot06Objects_LockFloat); else waiting for the hookshot.
                    log::info!("Bg_Spot06_Objects: the Water Temple's lock for an adult (the hookshot's pull, the lock floating, one-point 4120, Phase 16) isn't ported: a placeholder");
                    return oot_game::spawn::Placeholder::init(this.actor, play);
                }
                this.actor.scale = Vec3::splat(0.1);
                this.collider = ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS);
                this.action = Action::LockWait;
                let e = &mut this.collider.elements[0].dim;
                e.world_sphere.radius = e.model_sphere.radius;
                e.world_sphere.center[2] = this.actor.world_pos.z as i16;
                e.world_sphere.center[0] = this.actor.world_pos.x as i16;
                e.world_sphere.center[1] = this.actor.world_pos.y as i16;
                this.actor.col_chk_info.mass = MASS_IMMOVABLE;
            }
            LHO_WATER_PLANE => {
                // sInitChainWaterPlane: scale 1.
                this.actor.scale = Vec3::splat(1.0);
                this.actor.flags = ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED;
                if adult && !play.save.get_event_chk_inf(EVENTCHKINF_RESTORED_LAKE_HYLIA) {
                    log::info!("Bg_Spot06_Objects: the drained lake (an adult's before the Water Temple: the water boxes at WATER_LEVEL_LOWERED, the rise in gLakeHyliaRestoredCs, Phase 16) isn't ported: a placeholder");
                    return oot_game::spawn::Placeholder::init(this.actor, play);
                }
                this.lake_hylia_water_level = 0.0;
                this.action = Action::DoNothing;
            }
            LHO_ICE_BLOCK => {
                this.actor.scale = Vec3::splat(0.1);
                this.set_bg_actor(play, ICE_BLOCK_COLLISION);
                this.action = Action::DoNothing;
                if !adult {
                    this.actor.kill();
                }
                play.col.dyna.set_source(this.bg, source(&this.actor));
            }
            _ => {}
        }
        Box::new(this)
    }

    /// `DynaPolyActor_Init(&this->dyna, 0)` with `collision`.
    fn set_bg_actor(&mut self, play: &mut PlayState, collision: &str) {
        if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, collision) {
            self.bg = play.col.dyna.set_bg_actor(h, source(&self.actor), 0);
        }
    }

    /// `BgSpot06Objects_GateSpawnBubbles`: on frames divisible by 3, a bubble somewhere along the
    /// gate's 160 across.
    fn gate_spawn_bubbles(&mut self, play: &mut PlayState) {
        if play.gameplay_frames % 3 == 0 {
            let tmp = play.rand.centered_float(160.0);
            let yaw = self.actor.shape_rot.y.wrapping_add(0x4000);
            let pos = Vec3::new((sin_s(yaw) * tmp) + self.actor.world_pos.x, self.actor.world_pos.y, (cos_s(yaw) * tmp) + self.actor.world_pos.z);
            let scale = (play.rand.zero_one() * 0.05) + 0.175;
            play.with_ss(|s| s.bubble_spawn(pos, 50.0, 70.0, 10.0, scale));
        }
    }

    /// `BgSpot06Objects_GateWaitForSwitch`: its switch set, up 3, fifteen tries at a bubble, and
    /// 100 frames' wait.
    fn gate_wait_for_switch(&mut self, play: &mut PlayState) {
        if play.flags.get_switch(self.switch_flag as i32) {
            self.timer = 100;
            self.actor.world_pos.y += 3.0;
            self.action = Action::GateWaitToOpen;
            for _ in 0..15 {
                self.gate_spawn_bubbles(play);
            }
        }
    }

    /// `BgSpot06Objects_GateWaitToOpen`.
    fn gate_wait_to_open(&mut self) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.action = Action::GateOpen;
        }
    }

    /// `BgSpot06Objects_GateOpen`: up 0.6 a frame to 120 above home, bubbling and sliding.
    fn gate_open(&mut self, play: &mut PlayState) {
        self.gate_spawn_bubbles(play);
        if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 120.0, 0.6) {
            self.action = Action::DoNothing;
            self.timer = 0;
            oot_game::actor_ctx::audio_play_actor_sfx2(play, NA_SE_EV_METALDOOR_STOP);
        } else {
            self.actor.play_sfx_flagged(NA_SE_EV_METALDOOR_SLIDE - SFX_FLAG);
        }
    }

    /// `BgSpot06Objects_LockWait`: waiting for the hookshot (`CollisionCheck_SetAC`). Its hit's
    /// branch (pulled out, rising, floating: the adult's) isn't ported.
    fn lock_wait(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            log::warn!("Bg_Spot06_Objects: the lock hit (the hookshot's pull, an adult's: LockPullOutward, LockSwimToSurface, LockFloat, one-point 4120) isn't ported");
        } else {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
    }
}

impl ActorImpl for BgSpot06Objects {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgSpot06Objects_Update`: the action, and the lock's push (`CollisionCheck_SetOC`).
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::DoNothing => {}
            Action::GateWaitForSwitch => self.gate_wait_for_switch(play),
            Action::GateWaitToOpen => self.gate_wait_to_open(),
            Action::GateOpen => self.gate_open(play),
            Action::LockWait => self.lock_wait(play),
        }
        if self.ty == LHO_WATER_TEMPLE_ENTRANCE_LOCK {
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(self.bg, source(&self.actor));
        }
    }

    /// `BgSpot06Objects_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        match self.ty {
            LHO_WATER_TEMPLE_ENTRACE_GATE | LHO_ICE_BLOCK => play.col.dyna.delete_bg_actor(self.bg),
            // Collider_DestroyJntSph: nothing to free.
            _ => {}
        }
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// `BgSpot06Objects_Draw`: the gate, the lock (`Collider_UpdateSpheres` only while it swims
    /// up: the adult's), the water (`BgSpot06Objects_DrawLakeHyliaWater`) or the ice block.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        match self.ty {
            LHO_WATER_TEMPLE_ENTRACE_GATE => crate::gfx_draw_dlist_opa(out, OBJECT, GATE_DL, rs),
            LHO_WATER_TEMPLE_ENTRANCE_LOCK => crate::gfx_draw_dlist_opa(out, OBJECT, LOCK_DL, rs),
            LHO_WATER_PLANE => {
                let frames = play.state_frames;
                let mut sv = SegmentValues::default();
                sv.read(SEG_SCROLL_8, &water_scroll_8(frames));
                sv.read(SEG_SCROLL_9, &water_scroll_9(frames));
                // (lakeHyliaWaterLevel < -680.0f) && !IS_CUTSCENE_LAYER: the low water.
                let low = self.lake_hylia_water_level < -680.0 && !play.is_cutscene_layer();
                let bake = if low { BAKE_LOW_WATER } else { BAKE_HIGH_WATER };
                out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: actor_draw_matrix(rs), bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
            }
            LHO_ICE_BLOCK => crate::gfx_draw_dlist_opa(out, OBJECT, ICE_BLOCK_DL, rs),
            _ => {}
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0 && self.ty == LHO_WATER_TEMPLE_ENTRANCE_LOCK).then_some(ColliderMut::JntSph(&mut self.collider))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_water_scrolls_as_the_c() {
        // Gfx_TwoTexScroll at frames 3: x1 = (u32)-3 % 2048 = 2045, y1 = 3; segment 9's y = 18.
        let mut sv = SegmentValues::default();
        sv.read(SEG_SCROLL_8, &water_scroll_8(3));
        sv.read(SEG_SCROLL_9, &water_scroll_9(3));
        assert_eq!(sv.tiles[8][0], Some((2045, 3)));
        assert_eq!(sv.tiles[8][1], Some((3, 3)));
        assert_eq!(sv.tiles[9][0], Some((2045, 18)));
        assert_eq!(sv.tiles[9][1], Some((3, 18)));
        assert_eq!(WATER_LEVEL_LOWERED, -1993);
    }
}
