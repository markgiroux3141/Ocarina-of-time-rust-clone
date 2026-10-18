//! `Bg_Gnd_Darkmeiro` (`ovl_Bg_Gnd_Darkmeiro/z_bg_gnd_darkmeiro.c`): the clear blocks, the shadow
//! trial's invisible path and the clear blocks' timer (GAME-09 milestone 3). Dodongo's Cavern
//! (Master Quest) places two clear blocks in room 8: 0xFF01 (always there) and 0x1901 (flag 0x19);
//! the Forest, Spirit and Shadow Temples, the Ice Cavern and Ganon's Castle more.
//!
//! - **The invisible path** (`DARKMEIRO_INVISIBLE_PATH`): `gShadowTrialPathDL`, reacting to the
//!   Lens (`ACTOR_FLAG_REACT_TO_LENS`); it does nothing else.
//! - **A clear block** (`DARKMEIRO_CLEAR_BLOCK`, `gClearBlockCol`): with flag 0x3F always there
//!   (`BgGndDarkmeiro_UpdateStaticBlock`, drawn opaque white). Otherwise there while its switch
//!   flag is set and for 64 frames after it's cleared (`timer1`), its collision on and off with it
//!   (`BgGndDarkmeiro_ToggleBlock`: `DynaPoly_EnableCollision`, `_DisableCollision`); drawn while
//!   `timer1` runs, with `timer2` as its alpha: fading in (8 a frame up to 127) while the flag is
//!   set, flickering in its last 64 frames, fading out in the last 16. @bug (game): the list
//!   (`gClearBlockDL`) sets its own prim colour (198, 202, 208, 178) after the draw's, so the
//!   alpha is never seen: the block is drawn the same throughout.
//! - **The timer** (`DARKMEIRO_BLOCK_TIMER`, its switch flag N, not drawn): a flag N+1 or N+2 set
//!   (by a switch) starts a timer of 304 for it (`NA_SE_EV_RED_EYE`), clearing that flag at its end;
//!   flag N is set while either timer is 64 or more (the clear blocks of flag N with it), the
//!   timer's tick (`Actor_PlaySfx_FlaggedTimer`) while either runs.
//!
//! The whole overlay is ported. The Lens (`play->actorCtx.lensActive`) isn't ported: as the C reads
//! it false, `Actor_DrawAll` doesn't draw the invisible path in a room that shows lens actors only
//! through the Lens (`LENS_MODE_SHOW_ACTORS`), or outside its room. The draws are
//! `Gfx_DrawDListXlu`'s (the lists set their own render modes).
//!
//! Params: `type` bits 0 to 7 (`DarkmeiroType`: 0 `DARKMEIRO_INVISIBLE_PATH`, 1
//! `DARKMEIRO_CLEAR_BLOCK`, 2 `DARKMEIRO_BLOCK_TIMER`; another does nothing and isn't drawn);
//! `switchFlag` bits 8 to 13 (a clear block's flag, 0x3F: always there; the timer's N); bits 14
//! and 15 unused (set with the always-there block's 0x3F). `shape.rot.x` and `shape.rot.z`: the
//! block's tilt (room 8 stands both on edge, the Ice Cavern its two).
//! Flags: a clear block reads its switch flag `switchFlag`; the timer reads, sets and clears
//! `switchFlag` and reads and clears `switchFlag + 1` and `switchFlag + 2`.

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::cos_s;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_REACT_TO_LENS, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_BG_GND_DARKMEIRO` (`actor_table.h`: 0x01B4).
pub const ACTOR_BG_GND_DARKMEIRO: i16 = 0x01B4;
/// `OBJECT_DEMO_KEKKAI` (`object_table.h`: 0x0179).
pub const OBJECT: &str = "object_demo_kekkai";
const COLLISION: &str = "gClearBlockCol";

/// `Bg_Gnd_Darkmeiro_Profile`: `ACTORCAT_PROP`, always updating and drawn, no draw of its own (its
/// init gives one by type).
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_BG_GND_DARKMEIRO,
    name: "Bg_Gnd_Darkmeiro",
    category: ACTORCAT_PROP,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `PARAMS_GET_U(params, 0, 8)`: the type (`DarkmeiroType`).
pub const TYPE: Field = Field::params(
    "type",
    0,
    8,
    Kind::Named(&[(0, "DARKMEIRO_INVISIBLE_PATH"), (1, "DARKMEIRO_CLEAR_BLOCK"), (2, "DARKMEIRO_BLOCK_TIMER")]),
    "the shadow trial's invisible path, a clear block, or the clear blocks' timer",
);
/// `PARAMS_GET_U(params, 8, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 8, 6, Kind::SwitchFlag, "a clear block's (0x3F: always there); the timer's N (N+1 and N+2 start it)");
/// Bits 14 and 15: read by nothing.
pub const UNUSED_14: Field = Field::params("params bits 14 and 15", 14, 2, Kind::Unused, "set with the always-there block's 0x3F (0xFF01); read by nothing");
/// `shape.rot.x`, `shape.rot.z`: the block's tilt.
pub const ROT_X: Field = Field::rot("shape.rot.x", Source::RotX, 0, 16, Kind::Rotation, "the block's pitch (Dodongo's Cavern's room 8 stands both on edge)");
pub const ROT_Z: Field = Field::rot("shape.rot.z", Source::RotZ, 0, 16, Kind::Rotation, "the block's roll (the Ice Cavern's stand on edge)");

/// Bg_Gnd_Darkmeiro's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, SWITCH_FLAG, UNUSED_14, ROT_X, ROT_Z],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).reads().sets().clears().note("a clear block: there while set (and 64 frames on); the timer: set while either timer is 64 or more"),
        FlagUse::new(Flag::Switch("switchFlag + 1")).reads().clears().note("the timer: set, its first timer runs 304 frames, then clears it"),
        FlagUse::new(Flag::Switch("switchFlag + 2")).reads().clears().note("the timer: the same for its second timer"),
    ],
    todo: false,
    objects: &[],
};

/// `DarkmeiroType` (`z_bg_gnd_darkmeiro.h`).
pub const DARKMEIRO_INVISIBLE_PATH: i16 = 0;
pub const DARKMEIRO_CLEAR_BLOCK: i16 = 1;
pub const DARKMEIRO_BLOCK_TIMER: i16 = 2;

/// `NA_SE_EV_RED_EYE` (`environmentbank_table.h`: 0x2881).
pub const NA_SE_EV_RED_EYE: u16 = 0x2881;

/// `LENS_MODE_SHOW_ACTORS` (`room.h`: 0): lens actors only seen through the Lens.
const LENS_MODE_SHOW_ACTORS: u8 = 0;

const BAKE_BLOCK: &str = "Bg_Gnd_Darkmeiro/block";
const BAKE_PATH: &str = "Bg_Gnd_Darkmeiro/path";

/// `Gfx_DrawDListXlu`'s meshes: `gClearBlockDL` and `gShadowTrialPathDL`. The draw's
/// `gDPSetPrimColor` before the block isn't baked: the list sets its own after it.
pub fn bakes() -> Vec<MeshBake> {
    let dl = |name: &str, sym: &str| MeshBake { name: name.into(), object: OBJECT.into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::DLists(vec![(OBJECT.into(), sym.into())]) };
    vec![dl(BAKE_BLOCK, "gClearBlockDL"), dl(BAKE_PATH, "gShadowTrialPathDL")]
}

/// `updateFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateFunc {
    Noop,
    UpdateBlockTimer,
    UpdateStaticBlock,
    UpdateSwitchBlock,
}

impl UpdateFunc {
    pub fn decomp_name(self) -> &'static str {
        match self {
            UpdateFunc::Noop => "BgGndDarkmeiro_Noop",
            UpdateFunc::UpdateBlockTimer => "BgGndDarkmeiro_UpdateBlockTimer",
            UpdateFunc::UpdateStaticBlock => "BgGndDarkmeiro_UpdateStaticBlock",
            UpdateFunc::UpdateSwitchBlock => "BgGndDarkmeiro_UpdateSwitchBlock",
        }
    }
}

/// `draw` (the init's, by type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawFunc {
    InvisiblePath,
    SwitchBlock,
    StaticBlock,
}

/// What `render_state`'s switch holds: nothing drawn, the path, the block.
mod rs {
    pub const NONE: u32 = 0;
    pub const PATH: u32 = 1;
    pub const BLOCK: u32 = 2;
}

pub struct BgGndDarkmeiro {
    /// `dyna.actor`.
    pub actor: Actor,
    /// `dyna.bgId`: a clear block's `gClearBlockCol` (`BG_ACTOR_MAX` if it failed); the other
    /// types never register one (the zeroed actor's 0, which their destroy doesn't touch).
    pub bg: u16,
    /// `actionFlags`: 2 the block's collision on, 4 timer 1 running, 8 timer 2.
    pub action_flags: u16,
    /// `timer1`: a clear block's vanish countdown; the timer's for flag N+1.
    pub timer1: i16,
    /// `timer2`: a clear block's alpha (unseen); the timer's for flag N+2.
    pub timer2: i16,
    pub update_func: UpdateFunc,
    pub draw_func: Option<DrawFunc>,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgGndDarkmeiro {
    fn switch_flag(&self) -> i32 {
        SWITCH_FLAG.get(self.actor.params) as i32
    }

    /// `BgGndDarkmeiro_ToggleBlock`: the collision off once `timer1` runs out, on while it runs.
    fn toggle_block(&mut self, play: &mut PlayState) {
        if self.action_flags & 2 != 0 {
            if self.timer1 == 0 {
                play.col.dyna.set_collision_disabled(self.bg, true);
                self.action_flags &= !2;
            }
        } else if self.timer1 != 0 {
            play.col.dyna.set_collision_disabled(self.bg, false);
            self.action_flags |= 2;
        }
    }

    /// `BgGndDarkmeiro_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgGndDarkmeiro { actor, bg: 0, action_flags: 0, timer1: 0, timer2: 0, update_func: UpdateFunc::Noop, draw_func: None };
        this.actor.scale = Vec3::splat(0.1);
        match TYPE.get(this.actor.params) {
            DARKMEIRO_INVISIBLE_PATH => {
                this.draw_func = Some(DrawFunc::InvisiblePath);
                this.actor.flags |= ACTOR_FLAG_REACT_TO_LENS;
                log::debug!("Bg_Gnd_Darkmeiro: the Lens of Truth (actorCtx.lensActive) isn't ported: the invisible path is drawn only where Actor_DrawAll draws lens actors without it");
            }
            DARKMEIRO_CLEAR_BLOCK => {
                this.bg = match crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
                    Some(h) => play.col.dyna.set_bg_actor(h, source(&this.actor), 0),
                    None => BG_ACTOR_MAX,
                };
                if this.switch_flag() == 0x3F {
                    this.update_func = UpdateFunc::UpdateStaticBlock;
                    this.draw_func = Some(DrawFunc::StaticBlock);
                } else {
                    this.action_flags = 0;
                    this.timer1 = 0;
                    this.timer2 = 0;
                    this.draw_func = Some(DrawFunc::SwitchBlock);
                    this.update_func = UpdateFunc::UpdateSwitchBlock;
                    if !play.flags.get_switch(this.switch_flag()) {
                        play.col.dyna.set_collision_disabled(this.bg, true);
                    } else {
                        this.timer1 = 64;
                        this.action_flags |= 2;
                    }
                }
            }
            DARKMEIRO_BLOCK_TIMER => {
                this.action_flags = 0;
                this.timer1 = 0;
                this.timer2 = 0;
                this.update_func = UpdateFunc::UpdateBlockTimer;
                this.draw_func = None;
                let n = this.switch_flag();
                if play.flags.get_switch(n + 1) {
                    this.timer1 = 64;
                    this.action_flags |= 4;
                }
                if play.flags.get_switch(n + 2) {
                    this.timer2 = 64;
                    this.action_flags |= 8;
                }
                if this.timer1 != 0 || this.timer2 != 0 {
                    play.flags.set_switch(n);
                } else {
                    play.flags.unset_switch(n);
                }
            }
            _ => {}
        }
        Box::new(this)
    }

    /// One of the timer's two (`BgGndDarkmeiro_UpdateBlockTimer`): its flag set, it starts (304,
    /// `NA_SE_EV_RED_EYE`) or counts down, clearing the flag at its end.
    fn update_timer(play: &mut PlayState, flag: i32, action_flags: &mut u16, bit: u16, timer: &mut i16) {
        if play.flags.get_switch(flag) {
            if *action_flags & bit != 0 {
                if *timer > 0 {
                    *timer -= 1;
                } else {
                    play.flags.unset_switch(flag);
                    *action_flags &= !bit;
                }
            } else {
                *action_flags |= bit;
                *timer = 304;
                play.audio.play_sfx_centered(NA_SE_EV_RED_EYE);
            }
        }
    }

    /// `BgGndDarkmeiro_UpdateBlockTimer`.
    fn update_block_timer(&mut self, play: &mut PlayState) {
        let n = self.switch_flag();
        Self::update_timer(play, n + 1, &mut self.action_flags, 4, &mut self.timer1);
        Self::update_timer(play, n + 2, &mut self.action_flags, 8, &mut self.timer2);
        // CLAMP_MIN(timer1, timer2): the larger.
        let time_left = self.timer1.max(self.timer2);
        if time_left > 0 {
            self.actor.play_sfx_flagged_timer(time_left as i32);
        }
        if self.timer1 >= 64 || self.timer2 >= 64 {
            play.flags.set_switch(n);
        } else {
            play.flags.unset_switch(n);
        }
    }

    /// `BgGndDarkmeiro_UpdateSwitchBlock`: `timer1` down, 64 while the flag is set; the collision
    /// with it.
    fn update_switch_block(&mut self, play: &mut PlayState) {
        if self.timer1 > 0 {
            self.timer1 -= 1;
        }
        if play.flags.get_switch(self.switch_flag()) {
            self.timer1 = 64;
        }
        self.toggle_block(play);
    }

    /// `BgGndDarkmeiro_DrawSwitchBlock`'s `timer2` (the alpha): up 8 a frame to 127 while the flag
    /// is set (`timer1` over 64: never, it's 64 at most), flickering over its last 64 frames
    /// (`cos(timer1 × 0x1000) × 64 + 127`, at most 127), fading over the last 16.
    fn switch_block_alpha(&mut self) {
        let vanish_timer = self.timer1;
        if vanish_timer != 0 {
            if vanish_timer > 64 {
                self.timer2 = if self.timer2 < 120 { self.timer2 + 8 } else { 127 };
            } else if vanish_timer > 16 {
                self.timer2 = ((cos_s(((self.timer1 as u16 as i32) * 0x1000) as i16) * 64.0) + 127.0) as i16;
                if self.timer2 > 127 {
                    self.timer2 = 127;
                }
            } else {
                self.timer2 = vanish_timer * 8;
            }
        }
    }

    /// `Actor_DrawAll`'s test for `ACTOR_FLAG_REACT_TO_LENS` with the Lens off (`lensActive`
    /// false: the Lens isn't ported): not drawn in a room that shows lens actors only through it,
    /// nor out of its room (those go to `invisibleActors`, drawn only with the Lens on).
    // shared: Actor_DrawAll's lens check (z_actor.c), as `en_rd.rs` has it.
    fn lens_hidden(&self, play: &PlayState) -> bool {
        self.actor.flags & ACTOR_FLAG_REACT_TO_LENS != 0 && (play.room_ctx.cur.lens_mode == LENS_MODE_SHOW_ACTORS || self.actor.room != play.room_ctx.cur.num)
    }
}

impl ActorImpl for BgGndDarkmeiro {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgGndDarkmeiro_Update`: `updateFunc`.
    fn update(&mut self, play: &mut PlayState) {
        match self.update_func {
            UpdateFunc::Noop => {}
            UpdateFunc::UpdateBlockTimer => self.update_block_timer(play),
            // BgGndDarkmeiro_UpdateStaticBlock: nothing.
            UpdateFunc::UpdateStaticBlock => {}
            UpdateFunc::UpdateSwitchBlock => self.update_switch_block(play),
        }
    }

    /// `BgGndDarkmeiro_DrawSwitchBlock`'s change: `timer2`.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.draw_func == Some(DrawFunc::SwitchBlock) {
            self.switch_block_alpha();
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        let what = match self.draw_func {
            Some(DrawFunc::InvisiblePath) => rs::PATH,
            Some(DrawFunc::StaticBlock) => rs::BLOCK,
            Some(DrawFunc::SwitchBlock) if self.timer1 != 0 => rs::BLOCK,
            _ => rs::NONE,
        };
        r.switches = vec![what];
        r
    }

    /// `BgGndDarkmeiro_DrawInvisiblePath` (`gShadowTrialPathDL`, unless the Lens would be needed),
    /// `_DrawSwitchBlock` (while `timer1` runs) and `_DrawStaticBlock` (`gClearBlockDL`):
    /// `Gfx_DrawDListXlu`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let bake = match rs.switches.first().copied() {
            Some(rs::PATH) if !self.lens_hidden(play) => BAKE_PATH,
            Some(rs::BLOCK) => BAKE_BLOCK,
            _ => return,
        };
        out.xlu.push(DrawCmd::new(MeshKey::named(keys::bake(bake)), actor_draw_matrix(rs)));
    }

    /// `BgGndDarkmeiro_Destroy`: a clear block's collision deleted.
    fn destroy(&mut self, play: &mut PlayState) {
        if TYPE.get(self.actor.params) != DARKMEIRO_CLEAR_BLOCK {
            return;
        }
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (TYPE.get(self.actor.params) == DARKMEIRO_CLEAR_BLOCK && self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
