//! `Bg_Bdan_Switch` (`ovl_Bg_Bdan_Switch/z_bg_bdan_switch.c`): the switches inside Lord
//! Jabu-Jabu (GAME-10 milestone 4), of `object_bdan_objects`. A switch flag (params bits 8 to 13)
//! is what each sets.
//!
//! - **Blue** (type 0, `gJabuBlueFloorSwitchDL`): a floor switch (`gJabuFloorSwitchCol`,
//!   `DYNA_TRANSFORM_POS`) held while anything that presses switches stands on it
//!   (`DynaPolyActor_IsSwitchPressed`: Link, Ruto, a crate). Pressed, it sets its flag
//!   (`NA_SE_SY_TRE_BOX_APPEAR` with the attention camera, `OnePointCutscene_AttentionSetSfx`) and
//!   sinks (0.2 of its height a frame, once the attention camera has turned to it or 100 frames
//!   on) to 0.1, `NA_SE_EV_FOOT_SWITCH`; left alone 6 frames, it clears the flag and rises again.
//! - **Yellow** (type 2, `gJabuYellowFloorSwitchDL`): stepped on by Link, the same, but it stays
//!   down until something clears its flag (`NA_SE_SY_CORRECT_CHIME`).
//! - **Yellow, heavy** (type 1): Link alone sinks it to 0.6; Link standing on it holding Ruto
//!   (`ACTOR_EN_RU1`) for 6 frames sets the flag and sinks it for good. Master Quest places none.
//! - **Tall yellow** (types 3 and 4, half as wide, `ATTENTION_RANGE_4`): hit (a sphere, 370 at its
//!   scale, 120 up, `0xEFC1FFFE`: Player's attacks, the slingshot, the boomerang), it sets the flag
//!   and shrinks from twice its height to its height (0.3 a frame); type 3 then waits for the flag
//!   to clear and grows back; type 4 is a toggle: hit again, it clears the flag (with the attention
//!   camera) and grows back. A hit counts once each 10 frames that Link isn't in a cutscene.
//!
//! The height (`unk_1C8`: 1 up, 0.1 down; 2 and 1 for the tall ones) scales `scale.y` (its
//! collision) and, with a wobble (`unk_1CC` 0x5DC a frame while up, 0xFA0 while down), the drawn
//! size (`func_8086D0EC`: `unk_1D0` up, `unk_1D4` across).
//!
//! Jabu-Jabu (Master Quest) places eight: yellow 0x3D (room 0: under a boulder, the map's chest;
//! room 12), 0x19 (room 1's floor: door 2), 0x3A (room 3), 0x3B (room 13), 0x39 (room 14: its water,
//! `Bg_Bdan_Objects` type 2); blue 0x3C (room 3: Ruto's, door 12), 0x12 (room 4), 0x0D (room 7);
//! tall yellow 0x0F (type 3, room 1's bottom: the compass's chest).
//!
//! The whole overlay is ported but the rumble (`Rumble_Request`, the controller's).
//!
//! Params: `type` bits 0 to 7 (`BgBdanSwitchType`: 0 blue, 1 yellow heavy, 2 yellow, 3 tall yellow,
//! 4 tall yellow toggle; others kill it at init); `switchFlag` bits 8 to 13.
//! Flags: reads, sets and clears switch flag `switchFlag`.

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_INTERACT_ACTOR_SWITCH_PRESSED, DYNA_INTERACT_PLAYER_ON_TOP, DYNA_TRANSFORM_POS};
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::cos_s;
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_SWITCH, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_TRE_BOX_APPEAR};
use oot_game::collision_check::*;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::sys_matrix::MtxF;

/// `ACTOR_BG_BDAN_SWITCH` (`actor_table.h`: 0x00E6).
pub const ACTOR_BG_BDAN_SWITCH: i16 = 0x00E6;
/// `OBJECT_BDAN_OBJECTS` (`object_table.h`: 0x0096).
pub const OBJECT: &str = "object_bdan_objects";
/// `gJabuFloorSwitchCol`, `gJabuBlueFloorSwitchDL`, `gJabuYellowFloorSwitchDL`
/// (`object_bdan_objects.xml`).
pub const COLLISION: &str = "gJabuFloorSwitchCol";
pub const BLUE_DL: &str = "gJabuBlueFloorSwitchDL";
pub const YELLOW_DL: &str = "gJabuYellowFloorSwitchDL";

/// `ACTOR_EN_RU1` (`actor_table.h`: 0x00A1): Ruto, whom the heavy switch wants held.
const ACTOR_EN_RU1: i16 = 0x00A1;
/// `NA_SE_EV_FOOT_SWITCH` (`environmentbank_table.h`: 0x2815).
pub const NA_SE_EV_FOOT_SWITCH: u16 = 0x2815;
/// `ATTENTION_RANGE_4` (`actor.h`).
const ATTENTION_RANGE_4: u8 = 4;

/// `Bg_Bdan_Switch_Profile`: `ACTORCAT_SWITCH`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`,
/// `OBJECT_BDAN_OBJECTS`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_BDAN_SWITCH, name: "Bg_Bdan_Switch", category: ACTORCAT_SWITCH, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 8)`: `BgBdanSwitchType`.
pub const TYPE: Field = Field::params(
    "type",
    0,
    8,
    Kind::Named(&[(0, "blue (held while pressed)"), (1, "yellow, heavy (Link holding Ruto)"), (2, "yellow (stays down)"), (3, "tall yellow (hit)"), (4, "tall yellow, a toggle (hit)")]),
    "the switch's kind (any other kills it)",
);
/// `PARAMS_GET_U(params, 8, 6)`.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 8, 6, Kind::SwitchFlag, "set when it's pressed or hit; cleared when a blue one is let go or a toggle hit again");

/// Bg_Bdan_Switch's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, SWITCH_FLAG],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().sets().clears().note("set pressed or hit; a blue one clears it let go, the toggle hit again")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `BgBdanSwitchType` (`z_bg_bdan_switch.h`).
pub const BLUE: i16 = 0;
pub const YELLOW_HEAVY: i16 = 1;
pub const YELLOW: i16 = 2;
pub const YELLOW_TALL_1: i16 = 3;
pub const YELLOW_TALL_2: i16 = 4;

/// `sJntSphElementsInit`: one sphere on limb 0, `{0, 120, 0}`, 370, scale 100, hit by
/// `0xEFC1FFFE`, and OC.
fn jnt_sph_elements() -> [ColliderJntSphElementInit; 1] {
    [ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xEFC1_FFFE, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_ON,
        },
        limb: 0,
        model_sphere: Sphere16 { center: [0, 120, 0], radius: 370 },
        scale: 100,
    }]
}

/// `sJntSphInit`: AC by Player's attacks, OC with everything (`OC2_TYPE_2`).
const JNT_SPH_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_JNTSPH };

/// `D_8086E0E0`: the tall ones' focus, through the draw's matrix.
const FOCUS: Vec3 = Vec3::new(0.0, 140.0, 0.0);

/// The collider's id (`collider_mut`).
const COLLIDER: u8 = 0;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_8086D5E0`: blue or yellow, up.
    Up,
    /// `func_8086D694`: blue or yellow, sinking.
    Sinking,
    /// `func_8086D754`: blue or yellow, down.
    Down,
    /// `func_8086D80C`: blue or yellow, rising.
    Rising,
    /// `func_8086D888`: heavy, up.
    HeavyUp,
    /// `func_8086D8CC`: heavy, sinking to 0.6.
    HeavySinking,
    /// `func_8086D95C`: heavy, sinking to the bottom.
    HeavySinkingAll,
    /// `func_8086DA1C`: heavy, at 0.6.
    HeavyHalf,
    /// `func_8086DAC4`: heavy, rising.
    HeavyRising,
    /// `func_8086DB40`: heavy, down for good.
    HeavyDown,
    /// `func_8086DB68`: tall, up.
    TallUp,
    /// `func_8086DC48`: tall, shrinking.
    TallShrinking,
    /// `func_8086DCE8`: tall, down.
    TallDown,
    /// `func_8086DDC0`: tall, growing.
    TallGrowing,
}

pub struct BgBdanSwitch {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    /// `collider` (the tall ones').
    pub collider: ColliderJntSph,
    /// `unk_1C8`: the height (1 up, 0.1 down; the heavy one's 0.6 half; the tall ones' 2 and 1).
    pub unk_1c8: f32,
    /// `unk_1CC`: the wobble's phase.
    pub unk_1cc: i16,
    /// `unk_1D0`, `unk_1D4`: the drawn size up and across.
    pub unk_1d0: f32,
    pub unk_1d4: f32,
    /// `unk_1D8`: the frames left before a release (blue), Ruto's hold (heavy) or the next hit
    /// counts (tall).
    pub unk_1d8: i16,
    /// `unk_1DA`: the frames left before it moves without the attention camera.
    pub unk_1da: i16,
    /// `unk_1DC`: last update's AC flags (the tall ones').
    pub unk_1dc: u8,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

fn load_collision(play: &PlayState) -> anyhow::Result<Arc<CollisionHeader>> {
    let assets = play.assets.as_ref().ok_or_else(|| anyhow::anyhow!("no asset pack"))?;
    Ok(Arc::new(assets.pack.collision(&keys::collision(OBJECT, COLLISION))?))
}

impl BgBdanSwitch {
    fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    fn switch_flag(&self) -> i32 {
        SWITCH_FLAG.get(self.actor.params) as i32
    }

    fn is_floor(&self) -> bool {
        matches!(self.ty(), BLUE | YELLOW_HEAVY | YELLOW)
    }

    fn is_tall(&self) -> bool {
        matches!(self.ty(), YELLOW_TALL_1 | YELLOW_TALL_2)
    }

    /// `DynaPolyActor_IsPlayerOnTop`, `DynaPolyActor_IsSwitchPressed`.
    fn interact(&self, play: &PlayState, flag: u8) -> bool {
        self.bg != BG_ACTOR_MAX && play.col.dyna.interact_flag(self.bg, flag)
    }

    /// `BgBdanSwitch_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgBdanSwitch {
            actor,
            bg: BG_ACTOR_MAX,
            action: Action::Up,
            collider: ColliderJntSph::default(),
            unk_1c8: 0.0,
            unk_1cc: 0,
            unk_1d0: 0.0,
            unk_1d4: 0.0,
            unk_1d8: 0,
            unk_1da: 0,
            unk_1dc: 0,
        };
        let ty = this.ty();
        // sInitChain: cullingVolumeDistance 1400, cullingVolumeScale 500, cullingVolumeDownward 1200.
        this.actor.culling_volume_distance = 1400.0;
        this.actor.culling_volume_scale = 500.0;
        this.actor.culling_volume_downward = 1200.0;
        let across = if ty == YELLOW_TALL_1 || ty == YELLOW_TALL_2 { 0.05 } else { 0.1 };
        this.actor.scale.z = across;
        this.actor.scale.x = across;
        this.actor.scale.y = 0.0;
        this.actor.set_focus(10.0);
        match ty {
            BLUE | YELLOW_HEAVY | YELLOW => this.init_dyna_poly(play),
            YELLOW_TALL_1 | YELLOW_TALL_2 => {
                // BgBdanSwitch_InitCollision.
                this.collider = ColliderJntSph::new(&JNT_SPH_INIT, &jnt_sph_elements());
                this.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
                this.actor.target_mode = ATTENTION_RANGE_4;
            }
            _ => {}
        }
        let flag = play.flags.get_switch(this.switch_flag());
        match ty {
            BLUE | YELLOW => {
                if flag {
                    this.func_8086d730();
                } else {
                    this.func_8086d5c4();
                }
            }
            YELLOW_HEAVY => {
                if flag {
                    this.func_8086db24();
                } else {
                    this.func_8086d86c();
                }
            }
            YELLOW_TALL_1 | YELLOW_TALL_2 => {
                if flag {
                    this.func_8086dccc();
                } else {
                    this.func_8086db4c();
                }
            }
            _ => {
                // schema: the whole params printed.
                log::warn!("Invalid ARG_DATA(arg_data {:#06x})(../z_bg_bdan_switch.c 454)", this.actor.params);
                this.actor.kill();
                return Box::new(this);
            }
        }
        // schema: the whole params printed.
        log::debug!("(Giant Fish Dungeon Special Switch)(arg_data {:#06x})", this.actor.params);
        if this.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(this.bg, source(&this.actor));
        }
        Box::new(this)
    }

    /// `BgBdanSwitch_InitDynaPoly`: `DynaPolyActor_Init(DYNA_TRANSFORM_POS)` and the bg actor.
    fn init_dyna_poly(&mut self, play: &mut PlayState) {
        self.bg = match load_collision(play) {
            Ok(h) => play.col.dyna.set_bg_actor(h, source(&self.actor), DYNA_TRANSFORM_POS),
            Err(e) => {
                log::error!("Bg_Bdan_Switch: {e:#}");
                BG_ACTOR_MAX
            }
        };
        if self.bg == BG_ACTOR_MAX {
            // schema: the whole params printed.
            log::warn!("Warning : move BG registration failed (../z_bg_bdan_switch.c 325)(name {})(arg_data {:#06x})", self.actor.id, self.actor.params);
        }
    }

    /// `func_8086D0EC`: the wobble's phase on (0x5DC a frame while it's positive, else 0xFA0), the
    /// drawn size from it and the height, `scale.y` the height's tenth, `shape.yOffset` 1.2 over
    /// the drawn height.
    pub fn func_8086d0ec(&mut self) {
        if self.unk_1cc > 0 {
            self.unk_1cc = self.unk_1cc.wrapping_add(0x5DC);
        } else {
            self.unk_1cc = self.unk_1cc.wrapping_add(0xFA0);
        }
        let c = cos_s(self.unk_1cc);
        match self.ty() {
            BLUE | YELLOW_HEAVY | YELLOW => {
                self.unk_1d4 = ((c * 0.5) + (53.000004 / 6.0)) * 0.012;
                self.unk_1d0 = ((c * 0.5) + 20.5) * (self.unk_1c8 * 0.0050000004);
                self.actor.scale.y = self.unk_1c8 * 0.1;
            }
            YELLOW_TALL_1 | YELLOW_TALL_2 => {
                self.unk_1d4 = ((c * 0.5) + (43.0 / 6.0)) * 0.0075000003;
                self.unk_1d0 = ((c * 0.5) + 20.5) * (self.unk_1c8 * 0.0050000004);
                self.actor.scale.y = self.unk_1c8 * 0.1;
            }
            _ => {}
        }
        self.actor.shape_y_offset = 1.2 / self.unk_1d0;
    }

    /// The switch as the one-point cutscenes read it (it's out of the arena in its update).
    fn cam_actor(&self, play: &PlayState) -> Option<oot_game::camera::CamActor> {
        play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor))
    }

    /// `func_8086D4B4`: the flag set (if it wasn't), with the attention camera on the switch and
    /// `NA_SE_SY_TRE_BOX_APPEAR` (blue, the toggle) or `NA_SE_SY_CORRECT_CHIME`.
    fn func_8086d4b4(&mut self, play: &mut PlayState) {
        let flag = self.switch_flag();
        if !play.flags.get_switch(flag) {
            let ty = self.ty();
            play.flags.set_switch(flag);
            let sfx = if ty == BLUE || ty == YELLOW_TALL_2 { NA_SE_SY_TRE_BOX_APPEAR } else { NA_SE_SY_CORRECT_CHIME };
            if let Some(me) = self.cam_actor(play) {
                play.onepoint_attention_set_sfx(me, sfx);
            }
        }
    }

    /// `func_8086D548`: the flag cleared (if it was set); the toggle's with the attention camera.
    fn func_8086d548(&mut self, play: &mut PlayState) {
        let flag = self.switch_flag();
        if play.flags.get_switch(flag) {
            play.flags.unset_switch(flag);
            if self.ty() == YELLOW_TALL_2
                && let Some(me) = self.cam_actor(play)
            {
                play.onepoint_attention_set_sfx(me, NA_SE_SY_TRE_BOX_APPEAR);
            }
        }
    }

    /// Whether a moving switch may move this frame: the attention camera has turned to it
    /// (`func_8005B198() == category`) or `unk_1DA` ran out.
    fn may_move(&self, play: &PlayState) -> bool {
        play.func_8005b198() == self.actor.category as i32 || self.unk_1da <= 0
    }

    /// `func_8086D5C4`: up.
    fn func_8086d5c4(&mut self) {
        self.action = Action::Up;
        self.unk_1c8 = 1.0;
    }

    /// `func_8086D5E0`: blue pressed (`DynaPolyActor_IsSwitchPressed`), yellow stepped on by Link:
    /// sinking, the flag set.
    fn func_8086d5e0(&mut self, play: &mut PlayState) {
        let pressed = match self.ty() {
            BLUE => self.interact(play, DYNA_INTERACT_ACTOR_SWITCH_PRESSED),
            YELLOW => self.interact(play, DYNA_INTERACT_PLAYER_ON_TOP),
            _ => false,
        };
        if pressed {
            self.func_8086d67c();
            self.func_8086d4b4(play);
        }
    }

    /// `func_8086D67C`: sinking, 100 frames' wait for the attention camera.
    fn func_8086d67c(&mut self) {
        self.action = Action::Sinking;
        self.unk_1da = 0x64;
    }

    /// `func_8086D694`: down 0.2 a frame to 0.1, `NA_SE_EV_FOOT_SWITCH`.
    fn func_8086d694(&mut self, play: &mut PlayState) {
        if self.may_move(play) {
            self.unk_1c8 -= 0.2;
            if self.unk_1c8 <= 0.1 {
                self.func_8086d730();
                audio_play_actor_sfx2(play, NA_SE_EV_FOOT_SWITCH);
                // not ported: Rumble_Request(xyzDistToPlayerSq, 120, 20, 10).
            }
        }
    }

    /// `func_8086D730`: down.
    fn func_8086d730(&mut self) {
        self.unk_1c8 = 0.1;
        self.action = Action::Down;
        self.unk_1d8 = 6;
    }

    /// `func_8086D754`: blue, let go 6 frames: rising, the flag cleared; yellow: rising once the flag
    /// is cleared.
    fn func_8086d754(&mut self, play: &mut PlayState) {
        match self.ty() {
            BLUE => {
                if !self.interact(play, DYNA_INTERACT_ACTOR_SWITCH_PRESSED) {
                    if self.unk_1d8 <= 0 {
                        self.func_8086d7fc();
                        self.func_8086d548(play);
                    }
                } else {
                    self.unk_1d8 = 6;
                }
            }
            YELLOW => {
                if !play.flags.get_switch(self.switch_flag()) {
                    self.func_8086d7fc();
                }
            }
            _ => {}
        }
    }

    /// `func_8086D7FC`: rising.
    fn func_8086d7fc(&mut self) {
        self.action = Action::Rising;
    }

    /// `func_8086D80C`: up 0.2 a frame to 1, `NA_SE_EV_FOOT_SWITCH`.
    fn func_8086d80c(&mut self, play: &mut PlayState) {
        self.unk_1c8 += 0.2;
        if self.unk_1c8 >= 1.0 {
            self.func_8086d5c4();
            audio_play_actor_sfx2(play, NA_SE_EV_FOOT_SWITCH);
        }
    }

    /// `func_8086D86C`: the heavy one up.
    fn func_8086d86c(&mut self) {
        self.action = Action::HeavyUp;
        self.unk_1c8 = 1.0;
    }

    /// `func_8086D888`: stepped on by Link: sinking.
    fn func_8086d888(&mut self, play: &mut PlayState) {
        if self.interact(play, DYNA_INTERACT_PLAYER_ON_TOP) {
            self.func_8086d8bc();
        }
    }

    /// `func_8086D8BC`.
    fn func_8086d8bc(&mut self) {
        self.action = Action::HeavySinking;
    }

    /// `func_8086D8CC`: down 0.2 a frame to 0.6, `NA_SE_EV_FOOT_SWITCH`.
    fn func_8086d8cc(&mut self, play: &mut PlayState) {
        self.unk_1c8 -= 0.2;
        if self.unk_1c8 <= 0.6 {
            self.func_8086d9f8();
            audio_play_actor_sfx2(play, NA_SE_EV_FOOT_SWITCH);
            // not ported: Rumble_Request(xyzDistToPlayerSq, 120, 20, 10).
        }
    }

    /// `func_8086D944`: sinking to the bottom, 100 frames' wait for the attention camera.
    fn func_8086d944(&mut self) {
        self.action = Action::HeavySinkingAll;
        self.unk_1da = 0x64;
    }

    /// `func_8086D95C`: down 0.2 a frame to 0.1, `NA_SE_EV_FOOT_SWITCH`, down for good.
    fn func_8086d95c(&mut self, play: &mut PlayState) {
        if self.may_move(play) {
            self.unk_1c8 -= 0.2;
            if self.unk_1c8 <= 0.1 {
                self.func_8086db24();
                audio_play_actor_sfx2(play, NA_SE_EV_FOOT_SWITCH);
                // not ported: Rumble_Request(xyzDistToPlayerSq, 120, 20, 10).
            }
        }
    }

    /// `func_8086D9F8`: at 0.6.
    fn func_8086d9f8(&mut self) {
        self.unk_1c8 = 0.6;
        self.action = Action::HeavyHalf;
        self.unk_1d8 = 6;
    }

    /// `func_8086DA1C`: Link on it holding Ruto 6 frames: sinking to the bottom, the flag set; Link
    /// off it 6 frames: rising.
    fn func_8086da1c(&mut self, play: &mut PlayState) {
        // GET_PLAYER(play)->heldActor.
        let held = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).and_then(|p| p.held_actor()).and_then(|h| play.actors.actor(h)).map(|a| a.id);
        if self.interact(play, DYNA_INTERACT_PLAYER_ON_TOP) {
            if held == Some(ACTOR_EN_RU1) {
                if self.unk_1d8 <= 0 {
                    self.func_8086d944();
                    self.func_8086d4b4(play);
                }
            } else {
                self.unk_1d8 = 6;
            }
        } else if self.unk_1d8 <= 0 {
            self.func_8086dab4();
        }
    }

    /// `func_8086DAB4`.
    fn func_8086dab4(&mut self) {
        self.action = Action::HeavyRising;
    }

    /// `func_8086DAC4`: up 0.2 a frame to 1, `NA_SE_EV_FOOT_SWITCH`.
    fn func_8086dac4(&mut self, play: &mut PlayState) {
        self.unk_1c8 += 0.2;
        if self.unk_1c8 >= 1.0 {
            self.func_8086d86c();
            audio_play_actor_sfx2(play, NA_SE_EV_FOOT_SWITCH);
        }
    }

    /// `func_8086DB24`: the heavy one down for good.
    fn func_8086db24(&mut self) {
        self.unk_1c8 = 0.1;
        self.action = Action::HeavyDown;
    }

    /// `func_8086DB4C`: the tall one up (twice its height).
    fn func_8086db4c(&mut self) {
        self.action = Action::TallUp;
        self.unk_1c8 = 2.0;
    }

    /// `func_8086DB68`: hit (type 3: any hit; type 4: a new one) with no hit in the last 10
    /// frames: shrinking, the flag set.
    fn func_8086db68(&mut self, play: &mut PlayState) {
        let hit = self.collider.base.ac_flags & AC_HIT != 0;
        let fresh = match self.ty() {
            YELLOW_TALL_1 => hit,
            YELLOW_TALL_2 => hit && self.unk_1dc & AC_HIT == 0,
            _ => return,
        };
        if fresh && self.unk_1d8 <= 0 {
            self.unk_1d8 = 0xA;
            self.func_8086dc30();
            self.func_8086d4b4(play);
        }
    }

    /// `func_8086DC30`: shrinking, 100 frames' wait for the attention camera.
    fn func_8086dc30(&mut self) {
        self.action = Action::TallShrinking;
        self.unk_1da = 0x64;
    }

    /// `func_8086DC48`: down 0.3 a frame to 1, `NA_SE_EV_FOOT_SWITCH`.
    fn func_8086dc48(&mut self, play: &mut PlayState) {
        if self.may_move(play) {
            self.unk_1c8 -= 0.3;
            if self.unk_1c8 <= 1.0 {
                self.func_8086dccc();
                audio_play_actor_sfx2(play, NA_SE_EV_FOOT_SWITCH);
            }
        }
    }

    /// `func_8086DCCC`: the tall one down.
    fn func_8086dccc(&mut self) {
        self.action = Action::TallDown;
        self.unk_1c8 = 1.0;
    }

    /// `func_8086DCE8`: type 3 grows back once the flag is cleared; type 4 hit anew (10 frames on)
    /// grows back and clears the flag.
    fn func_8086dce8(&mut self, play: &mut PlayState) {
        match self.ty() {
            YELLOW_TALL_1 => {
                if !play.flags.get_switch(self.switch_flag()) {
                    self.func_8086dda8();
                }
            }
            YELLOW_TALL_2 => {
                if self.collider.base.ac_flags & AC_HIT != 0 && self.unk_1dc & AC_HIT == 0 && self.unk_1d8 <= 0 {
                    self.unk_1d8 = 0xA;
                    self.func_8086dda8();
                    self.func_8086d548(play);
                }
            }
            _ => {}
        }
    }

    /// `func_8086DDA8`: growing, 100 frames' wait for the attention camera (the toggle's).
    fn func_8086dda8(&mut self) {
        self.action = Action::TallGrowing;
        self.unk_1da = 0x64;
    }

    /// `func_8086DDC0`: up 0.3 a frame to 2, `NA_SE_EV_FOOT_SWITCH` (the toggle once the attention
    /// camera has turned to it).
    fn func_8086ddc0(&mut self, play: &mut PlayState) {
        if self.ty() != YELLOW_TALL_2 || self.may_move(play) {
            self.unk_1c8 += 0.3;
            if self.unk_1c8 >= 2.0 {
                self.func_8086db4c();
                audio_play_actor_sfx2(play, NA_SE_EV_FOOT_SWITCH);
            }
        }
    }

    /// `func_8086DF58`'s matrix: at the position, `shape.yOffset × unk_1D0` up, turned by the shape,
    /// scaled `unk_1D4`, `unk_1D0`, `unk_1D4`.
    fn draw_matrix(pos: Vec3, rot: [i16; 3], y_offset: f32, up: f32, across: f32) -> MtxF {
        let mut m = MtxF::set_translate_rotate_yxz(pos.x, pos.y + y_offset * up, pos.z, rot);
        m.scale(across, up, across);
        m
    }
}

impl ActorImpl for BgBdanSwitch {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgBdanSwitch_Update`: `unk_1DA` down, the action, the size; the floors' `unk_1D8` down; the
    /// tall ones' down while Link isn't in a cutscene, their AC flags kept and cleared, the sphere's
    /// radius `unk_1D4 × 370`, AC and OC.
    fn update(&mut self, play: &mut PlayState) {
        if self.unk_1da > 0 {
            self.unk_1da -= 1;
        }
        match self.action {
            Action::Up => self.func_8086d5e0(play),
            Action::Sinking => self.func_8086d694(play),
            Action::Down => self.func_8086d754(play),
            Action::Rising => self.func_8086d80c(play),
            Action::HeavyUp => self.func_8086d888(play),
            Action::HeavySinking => self.func_8086d8cc(play),
            Action::HeavySinkingAll => self.func_8086d95c(play),
            Action::HeavyHalf => self.func_8086da1c(play),
            Action::HeavyRising => self.func_8086dac4(play),
            Action::HeavyDown => {}
            Action::TallUp => self.func_8086db68(play),
            Action::TallShrinking => self.func_8086dc48(play),
            Action::TallDown => self.func_8086dce8(play),
            Action::TallGrowing => self.func_8086ddc0(play),
        }
        self.func_8086d0ec();
        if !self.is_tall() {
            self.unk_1d8 = self.unk_1d8.wrapping_sub(1);
        } else {
            if !play.player_in_cs_mode() && self.unk_1d8 > 0 {
                self.unk_1d8 -= 1;
            }
            self.unk_1dc = self.collider.base.ac_flags;
            self.collider.base.ac_flags &= !AC_HIT;
            self.collider.elements[0].dim.model_sphere.radius = (self.unk_1d4 * 370.0) as i16;
            play.collision_check_set_ac(&self.actor, COLLIDER, &mut self.collider);
            play.collision_check_set_oc(&self.actor, COLLIDER, &mut self.collider);
        }
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(self.bg, source(&self.actor));
        }
    }

    /// `BgBdanSwitch_Draw`'s collider and focus for the tall ones (`Collider_UpdateSpheres(0)` and
    /// `Matrix_MultVec3f(D_8086E0E0, focus.pos)` through the draw's matrix).
    fn draw_update(&mut self, _play: &mut PlayState) {
        if !self.is_tall() {
            return;
        }
        let a = &self.actor;
        let r = a.shape_rot;
        let m = Self::draw_matrix(a.world_pos, [r.x, r.y, r.z], a.shape_y_offset, self.unk_1d0, self.unk_1d4).to_mat4();
        self.collider.update_spheres(0, &m);
        self.actor.focus_pos = m.transform_point3(FOCUS);
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.values = vec![self.unk_1d0, self.unk_1d4];
        rs
    }

    /// `BgBdanSwitch_Draw`: the yellow or blue switch (`func_8086DF58`).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let [up, across] = rs.values.as_slice() else { return };
        let dl = match self.ty() {
            BLUE => BLUE_DL,
            YELLOW_HEAVY | YELLOW | YELLOW_TALL_1 | YELLOW_TALL_2 => YELLOW_DL,
            _ => return,
        };
        let m: Mat4 = Self::draw_matrix(rs.pos, rs.rot, rs.y_offset, *up, *across).to_mat4();
        out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, dl)), m));
    }

    /// `BgBdanSwitch_Destroy`: the floors' bg actor (the tall ones' collider is the actor's own).
    fn destroy(&mut self, play: &mut PlayState) {
        if self.is_floor() && self.bg != BG_ACTOR_MAX {
            play.col.dyna.delete_bg_actor(self.bg);
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == COLLIDER).then_some(ColliderMut::JntSph(&mut self.collider))
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
