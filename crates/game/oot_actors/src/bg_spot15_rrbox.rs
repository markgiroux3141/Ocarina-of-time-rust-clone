//! `Bg_Spot15_Rrbox` (`ovl_Bg_Spot15_Rrbox/z_bg_spot15_rrbox.c`): Lon Lon's milk crates by Talon at
//! Hyrule Castle (and in the ranch's buildings), DynaPoly actors of `object_spot15_obj`
//! (`gLonLonMilkCrateCol`, `gLonLonMilkCrateDL`), 60 across at scale 0.1.
//!
//! Player at a crate's wall adds his push (or pull) to `dyna.unk_150` and his yaw to `unk_158`.
//! Resting, with its 10-frame wait over, a crate takes it if it may move that way
//! (`func_808B3AAC`) and nothing stands within its side's reach along it (`func_808B4010`:
//! `func_800435D8`'s two line tests, 29 out and 49 along): it slides 20 (0.5 faster a frame, up to
//! 2, `NA_SE_EV_ROCK_SLIDE`), `NA_SE_EV_WOOD_BOUND` against something at the end, the chime
//! (`NA_SE_SY_CORRECT_CHIME`) when it ends squarely on another crate. A step with none of its
//! bottom's corners and middle over a floor (`BgSpot15Rrbox_TrySnapToFloor`) makes it fall (gravity
//! -1) to the highest floor under them, landing with `NA_SE_EV_WOOD_BOUND`.
//!
//! At the castle a crate at x <= 930 and z >= -360 (by Talon) moves south (+z) or west (the push's
//! yaw between -0x6000 and 0x2000) only once Talon has gone home (`EVENTCHKINF_TALON_RETURNED_FROM_CASTLE`):
//! that way is the moat. A crate already on another crate stays (`func_808B3A40`). With its switch
//! flag (`params & 0x3F`) set, a crate starts stacked in the moat under the drain (`D_808B45C4`, the
//! next of two places) and never moves; nothing in the game sets the castle crates' flag.
//!
//! The whole overlay is ported.
//!
//! Params: `switchFlag` bits 0 to 5 (set: starts stacked in the moat; no "none" value, 0x3F is a
//! temp flag); bits 6 and 7 unused (3 in the Spirit Temple's).
//! Flags: reads the switch flag `switchFlag`; reads EVENTCHKINF_TALON_RETURNED_FROM_CASTLE (by Talon:
//! the way to the moat open).

use eng_collision::bgcheck::BGCHECK_Y_MIN;
use eng_collision::dyna::{BG_ACTOR_MAX, BGCHECK_SCENE, BgActorSource};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, step_to_f};
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, audio_play_actor_sfx2, dyna_poly_get_actor};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::event_chk_inf;
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, SFX_FLAG};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

use crate::obj_oshihiki::{NA_SE_EV_ROCK_SLIDE, rotate_xz};

/// `ACTOR_BG_SPOT15_RRBOX` (`actor_table.h`: 0x0107).
pub const ACTOR_BG_SPOT15_RRBOX: i16 = 0x0107;
pub const OBJECT: &str = "object_spot15_obj";

/// `Bg_Spot15_Rrbox_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT15_RRBOX, name: "Bg_Spot15_Rrbox", category: ACTORCAT_BG, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `NA_SE_EV_WOOD_BOUND` (`environmentbank_table.h`: 0x28C9).
pub const NA_SE_EV_WOOD_BOUND: u16 = 0x28C9;
/// `EVENTCHKINF_TALON_RETURNED_FROM_CASTLE` (`save.h`: 0x14).
pub const EVENTCHKINF_TALON_RETURNED_FROM_CASTLE: u16 = 0x14;

/// `PARAMS_GET_U(this->dyna.actor.params, 0, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set: starts stacked in the moat and never moves (no none: 0x3F is a temp flag)");
/// Bits 6 and 7: 3 in the Spirit Temple's data, read by nothing.
pub const BITS_6_7: Field = Field::params("params bits 6 and 7", 6, 2, Kind::Unused, "3 in the Spirit Temple's data; read by nothing");

/// Bg_Spot15_Rrbox's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG, BITS_6_7],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().note("set: starts in the moat"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE)).reads().note("by Talon: the way to the moat open"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};
/// `SCENE_LON_LON_BUILDINGS` (`scene_table.h`: 0x4C).
const SCENE_LON_LON_BUILDINGS: u16 = 0x4C;
/// `PLAYER_STATE2_4` (pushing or pulling).
const PLAYER_STATE2_4: u32 = 1 << 4;

/// `D_808B45C4`: where a crate with its flag set starts, in the moat under the drain.
pub const STACKED: [Vec3; 2] = [Vec3::new(770.0, 1490.0, -299.0), Vec3::new(770.0, 1550.0, -299.0)];

/// `sBoxGroundCheckPoints`: the bottom's corners and middle, in tenths of model space.
const GROUND_CHECK_POINTS: [Vec3; 5] = [Vec3::new(29.99, 0.01, -29.99), Vec3::new(-29.99, 0.01, -29.99), Vec3::new(-29.99, 0.01, 29.99), Vec3::new(29.99, 0.01, 29.99), Vec3::new(0.0, 0.01, 0.0)];

/// The overlay's static: `D_808B4590`, how many crates have started in the moat (zeroed by any
/// crate's destroy).
#[derive(Debug, Default)]
pub struct Statics {
    pub d_808b4590: i16,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_808B40AC`: resting.
    Rest,
    /// `func_808B4194`: sliding.
    Slide,
    /// `func_808B43D0`: falling.
    Fall,
    /// `func_808B44CC`: stacked in the moat for good.
    Stuck,
}

pub struct BgSpot15Rrbox {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    /// `unk_168`: the wait after a slide.
    pub unk_168: i16,
    /// `unk_16C`, `unk_170`: the push's yaw's sine and cosine.
    pub unk_16c: f32,
    pub unk_170: f32,
    /// `unk_174`: the slide's speed; `unk_178`: how far it has slid.
    pub unk_174: f32,
    pub unk_178: f32,
    /// `unk_17C`: the push taken (its sign the way).
    pub unk_17c: f32,
    /// `bgId`: the bg actor under it.
    pub under_bg: u16,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `player->stateFlags2 &= ~PLAYER_STATE2_4`.
fn clear_player_state2_4(play: &mut PlayState) {
    if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        p.change_state_flags2(0, PLAYER_STATE2_4);
    }
}

impl BgSpot15Rrbox {
    /// `BgSpot15Rrbox_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgSpot15Rrbox { actor, bg: BG_ACTOR_MAX, action: Action::Rest, unk_168: 0, unk_16c: 0.0, unk_170: 0.0, unk_174: 0.0, unk_178: 0.0, unk_17c: 0.0, under_bg: BG_ACTOR_MAX };
        // func_808B3960: DynaPolyActor_Init(&this->dyna, 0), gLonLonMilkCrateCol.
        if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, "gLonLonMilkCrateCol") {
            this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0);
        }
        if this.bg == BG_ACTOR_MAX {
            // schema: printed
            log::warn!("Warning : move BG registration failed (../z_bg_spot15_rrbox.c 171)(name {})(arg_data 0x{:04x})", this.actor.id, this.actor.params);
        }
        // sInitChain.
        this.actor.scale = Vec3::splat(0.1);
        this.actor.culling_volume_distance = 1000.0;
        this.actor.culling_volume_scale = 500.0;
        this.actor.culling_volume_downward = 1000.0;
        this.func_808b3a34();
        if play.flags.get_switch(SWITCH_FLAG.get(this.actor.params) as i32) {
            this.action = Action::Stuck;
            let s = play.overlay_static::<Statics>(ACTOR_BG_SPOT15_RRBOX);
            // D_808B45C4[D_808B4590]: a third would read past the table (only two crates).
            this.actor.world_pos = STACKED[(s.d_808b4590 as usize).min(1)];
            s.d_808b4590 += 1;
        } else {
            this.func_808b4084();
        }
        // schema: printed
        log::debug!("(spot15 Lon Lon Wooden Box)(arg_data 0x{:04x})", this.actor.params);
        play.col.dyna.set_source(this.bg, source(&this.actor));
        Box::new(this)
    }

    /// `func_808B3A34`: no bg actor under it.
    fn func_808b3a34(&mut self) {
        self.under_bg = BG_ACTOR_MAX;
    }

    /// `func_808B3A40`: the bg actor under it is squarely under it (within 0.1 across).
    fn func_808b3a40(&self, play: &PlayState) -> bool {
        if let Some(h) = dyna_poly_get_actor(&play.actors, &play.col, self.under_bg)
            && let Some(a) = play.actors.actor(h)
        {
            let (dx, dz) = (a.world_pos.x - self.actor.world_pos.x, a.world_pos.z - self.actor.world_pos.z);
            return dx * dx + dz * dz < 0.01;
        }
        false
    }

    /// `func_808B3AAC`: may it move the way it's pushed?
    fn func_808b3aac(&self, play: &PlayState) -> bool {
        if play.scene_id == SCENE_LON_LON_BUILDINGS {
            return true;
        } else if self.func_808b3a40(play) {
            return false;
        }
        let a = &self.actor;
        if a.world_pos.x <= 930.0 && a.world_pos.z >= -360.0 {
            let rot_y = if play.col.dyna.unk_150(self.bg) >= 0.0 { a.world_rot.y } else { a.world_rot.y.wrapping_add(i16::MIN) };
            if rot_y < 0x2000 && rot_y > -0x6000 {
                return play.save.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE);
            }
            return true;
        }
        true
    }

    /// A check point's position: scaled, turned along the push, at last frame's height.
    fn check_pos(&self, i: usize) -> Vec3 {
        let p = GROUND_CHECK_POINTS[i];
        let a = &self.actor;
        let rel = Vec3::new(p.x * (a.scale.x * 10.0), p.y * (a.scale.y * 10.0), p.z * (a.scale.z * 10.0));
        let mut c = rotate_xz(rel, self.unk_16c, self.unk_170);
        c.x += a.world_pos.x;
        c.y += a.prev_pos.y;
        c.z += a.world_pos.z;
        c
    }

    /// `BgSpot15Rrbox_TrySnapToCheckedPoint`: the floor under check point `i` (its own collision
    /// skipped, `BgCheck_EntityRaycastDown6`); on (or above) it: set on it.
    fn try_snap_to_checked_point(&mut self, play: &PlayState, i: usize) -> bool {
        self.func_808b3a34();
        let c = self.check_pos(i);
        let (y, poly) = play.col.entity_raycast_down6(c, self.bg, 0.0);
        self.actor.floor_height = y;
        self.actor.floor_poly = poly;
        self.under_bg = poly.map_or(BGCHECK_SCENE, |p| p.bg);
        if (self.actor.floor_height - self.actor.world_pos.y) >= -0.001 {
            self.actor.world_pos.y = self.actor.floor_height;
            return true;
        }
        false
    }

    /// `BgSpot15Rrbox_GetFloorHeight`: the highest floor under the five points (the first highest's
    /// bg actor kept).
    fn get_floor_height(&mut self, play: &mut PlayState) -> f32 {
        self.func_808b3a34();
        let mut floor_y = BGCHECK_Y_MIN;
        for i in 0..GROUND_CHECK_POINTS.len() {
            let c = self.check_pos(i);
            let (y, poly) = play.col.entity_raycast_down6(c, self.bg, 0.0);
            // (actor->floorPoly is written by each test.)
            self.actor.floor_poly = poly;
            if floor_y < y {
                floor_y = y;
                self.under_bg = poly.map_or(BGCHECK_SCENE, |p| p.bg);
            }
        }
        floor_y
    }

    /// `BgSpot15Rrbox_TrySnapToFloor`: the five points in turn until one is over a floor.
    fn try_snap_to_floor(&mut self, play: &PlayState) -> bool {
        (0..5).any(|i| self.try_snap_to_checked_point(play, i))
    }

    /// `func_808B4010`: something in the way (`!func_800435D8(play, &dyna, 29, 49, 1)`).
    fn func_808b4010(&self, play: &PlayState) -> bool {
        let arg2 = (self.actor.scale.x * 290.0) as i16;
        let arg3 = (self.actor.scale.x * 290.0 + 20.0) as i16;
        !func_800435d8(play, &self.actor, self.bg, arg2, arg3, 1)
    }

    /// `func_808B4084`: at rest.
    fn func_808b4084(&mut self) {
        self.actor.gravity = 0.0;
        self.actor.velocity = Vec3::ZERO;
        self.action = Action::Rest;
    }

    /// `func_808B40AC`: a push taken, or dropped (with Player's `PLAYER_STATE2_4`).
    fn rest(&mut self, play: &mut PlayState) {
        let unk_150 = play.col.dyna.unk_150(self.bg);
        if self.unk_168 <= 0 && unk_150.abs() > 0.001 {
            if self.func_808b3aac(play) && !self.func_808b4010(play) {
                self.unk_17c = unk_150;
                self.func_808b4178();
            } else {
                clear_player_state2_4(play);
                play.col.dyna.set_unk_150(self.bg, 0.0);
            }
        } else {
            clear_player_state2_4(play);
            play.col.dyna.set_unk_150(self.bg, 0.0);
        }
    }

    /// `func_808B4178`.
    fn func_808b4178(&mut self) {
        self.action = Action::Slide;
        self.actor.gravity = 0.0;
    }

    /// `func_808B4194`: 20 along the push from `home`; off every floor it falls; at the end the
    /// sounds, `home` here and a 10-frame wait.
    fn slide(&mut self, play: &mut PlayState) {
        self.unk_174 += 0.5;
        self.unk_174 = self.unk_174.min(2.0);
        let approx_f_result = step_to_f(&mut self.unk_178, 20.0, self.unk_174);
        let sign = if self.unk_17c >= 0.0 { 1.0 } else { -1.0 };
        let temp_unk_178 = sign * self.unk_178;
        self.actor.world_pos.x = self.actor.home_pos.x + (temp_unk_178 * self.unk_16c);
        self.actor.world_pos.z = self.actor.home_pos.z + (temp_unk_178 * self.unk_170);
        if !self.try_snap_to_floor(play) {
            self.actor.home_pos.x = self.actor.world_pos.x;
            self.actor.home_pos.z = self.actor.world_pos.z;
            clear_player_state2_4(play);
            play.col.dyna.set_unk_150(self.bg, 0.0);
            self.unk_178 = 0.0;
            self.unk_174 = 0.0;
            self.func_808b4380(play);
        } else if approx_f_result {
            if self.func_808b4010(play) {
                audio_play_actor_sfx2(play, NA_SE_EV_WOOD_BOUND);
            }
            if self.func_808b3a40(play) {
                play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            }
            self.actor.home_pos.x = self.actor.world_pos.x;
            self.actor.home_pos.z = self.actor.world_pos.z;
            clear_player_state2_4(play);
            play.col.dyna.set_unk_150(self.bg, 0.0);
            self.unk_178 = 0.0;
            self.unk_174 = 0.0;
            self.unk_168 = 10;
            self.func_808b4084();
        }
        audio_play_actor_sfx2(play, NA_SE_EV_ROCK_SLIDE - SFX_FLAG);
    }

    /// `func_808B4380`: falling (gravity -1) to the highest floor under it.
    fn func_808b4380(&mut self, play: &mut PlayState) {
        self.actor.velocity = Vec3::ZERO;
        self.actor.gravity = -1.0;
        self.actor.floor_height = self.get_floor_height(play);
        self.action = Action::Fall;
    }

    /// `func_808B43D0`: a push on it now is dropped; landed, at rest with `NA_SE_EV_WOOD_BOUND`.
    fn fall(&mut self, play: &mut PlayState) {
        if play.col.dyna.unk_150(self.bg).abs() > 0.001 {
            play.col.dyna.set_unk_150(self.bg, 0.0);
            clear_player_state2_4(play);
        }
        self.actor.move_forward();
        if self.actor.world_pos.y <= BGCHECK_Y_MIN + 10.0 {
            // schema: printed
            log::warn!("Warning : Lon Lon Wooden Box fell too far (../z_bg_spot15_rrbox.c 599)(arg_data 0x{:04x})", self.actor.params);
            self.actor.kill();
            return;
        }
        let floor_height = self.actor.floor_height;
        if (floor_height - self.actor.world_pos.y) >= -0.001 {
            self.actor.world_pos.y = floor_height;
            self.func_808b4084();
            audio_play_actor_sfx2(play, NA_SE_EV_WOOD_BOUND);
        }
    }

    /// `func_808B44CC`: pushes dropped.
    fn stuck(&mut self, play: &mut PlayState) {
        clear_player_state2_4(play);
        play.col.dyna.set_unk_150(self.bg, 0.0);
    }
}

/// `func_800435D8` (`z_bg_item.c`): no wall along the push for `arg3` (less 0.1) from two points
/// `arg2` (less 0.1) either side of the actor's middle, `arg4` up, its own collision skipped
/// (`BgCheck_EntityLineTest3`, walls and one-sided).
pub fn func_800435d8(play: &PlayState, actor: &Actor, own_bg: u16, arg2: i16, arg3: i16, arg4: i16) -> bool {
    let unk_158 = play.col.dyna.unk_158(own_bg);
    let sin = sin_s(unk_158);
    let cos = cos_s(unk_158);
    let sign = if 0.0 <= play.col.dyna.unk_150(own_bg) { 1.0 } else { -1.0 };
    let a2 = arg2 as f32 - 0.1;
    let mut pos_a = Vec3::new(actor.world_pos.x + (a2 * cos), actor.world_pos.y + arg4 as f32, actor.world_pos.z - (a2 * sin));
    let a3 = arg3 as f32 - 0.1;
    let mut pos_b = Vec3::new(sign * a3 * sin + pos_a.x, pos_a.y, sign * a3 * cos + pos_a.z);
    if play.col.entity_line_test3(pos_a, pos_b, true, false, false, true, own_bg, 0.0).is_some() {
        return false;
    }
    pos_a.x = (actor.world_pos.x * 2.0) - pos_a.x;
    pos_a.z = (actor.world_pos.z * 2.0) - pos_a.z;
    pos_b.x = sign * a3 * sin + pos_a.x;
    pos_b.z = sign * a3 * cos + pos_a.z;
    if play.col.entity_line_test3(pos_a, pos_b, true, false, false, true, own_bg, 0.0).is_some() {
        return false;
    }
    true
}

impl ActorImpl for BgSpot15Rrbox {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgSpot15Rrbox_Update`: the wait down, `world.rot.y` the push's yaw (`dyna.unk_158`) and its
    /// sine and cosine, then the action; the new transform for `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        if self.unk_168 > 0 {
            self.unk_168 -= 1;
        }
        self.actor.world_rot.y = play.col.dyna.unk_158(self.bg);
        self.unk_16c = sin_s(self.actor.world_rot.y);
        self.unk_170 = cos_s(self.actor.world_rot.y);
        match self.action {
            Action::Rest => self.rest(play),
            Action::Slide => self.slide(play),
            Action::Fall => self.fall(play),
            Action::Stuck => self.stuck(play),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }
    /// `BgSpot15Rrbox_Destroy`: `D_808B4590` back to 0.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
        play.overlay_static::<Statics>(ACTOR_BG_SPOT15_RRBOX).d_808b4590 = 0;
    }
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `BgSpot15Rrbox_Draw`: `Gfx_DrawDListOpa(gLonLonMilkCrateDL)`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, "gLonLonMilkCrateDL")), oot_game::play::actor_draw_matrix(rs)));
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
