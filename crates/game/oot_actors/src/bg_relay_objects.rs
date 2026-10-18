//! `Bg_Relay_Objects` (`ovl_Bg_Relay_Objects/z_bg_relay_objects.c`): the windmill's set pieces, DynaPoly
//! actors of `object_relay_objects`: the turning gear (`WINDMILL_ROTATING_GEAR`) and the stone
//! doors of Dampé's race (`WINDMILL_DAMPE_STONE_DOOR`), both kept across rooms (`room` -1) and
//! each spawned once (the overlay's `D_808A9508`: a bit per door, bit 1 the gear).
//!
//! The gear turns 0x80 a frame, 0x400 with `EVENTCHKINF_65` (the Song of Storms played there; the
//! cutscene flag 5 sets it), stepping its speed there by 8 a frame, with the windmill's music
//! (`Audio_PlayWindmillBgm`) and its wooden gear's sound pitched by the speed. Destroyed outside a
//! cutscene's entrance (`cutsceneIndex` below `CS_INDEX_0`) it clears `EVENTCHKINF_65`: leaving
//! the windmill slows it again.
//!
//! The doors (`unk_169`: the room it's for, from the room it's placed in plus 1, or in room 0 from
//! its switch flag less 0x33): Dampé's race opens them by their switch flags (`func_808A90F4`:
//! 120 up at 12 a frame, open 120 frames with the hookshot, else 160, the timer ticking), then
//! they fall shut (`func_808A9234`); shut on Link in another room than theirs, Dampé laughs and
//! Link is sent back (`Play_TriggerRespawn`). The last door (`unk_169` 5) starts open and shuts
//! once Link is in its room; any of room 6 or later (the one placed again in the windmill) does
//! nothing.
//!
//! As the last door shuts in room 4 the race's timer stops (`gSaveContext.timerState =
//! TIMER_STATE_UP_FREEZE`; `En_Po_Relay`, Dampé, starts it: Phase 14).
//!
//! Not ported here: `Rumble_Request` (no rumble).
//!
//! Params: `switchFlag` bits 0 to 5 (a door's switch flag; in room 0, less 0x33, the room it's
//! for; the gear doesn't read it); bits 6 and 7 unused (set in the gear's 0xFF); `type` bits 8
//! to 15 (`WindmillSetpiecesMode`: 0 `WINDMILL_ROTATING_GEAR`, 1 `WINDMILL_DAMPE_STONE_DOOR`),
//! which the init writes over the params.
//! Flags: reads and sets EVENTCHKINF_65 (the gear: set by the cutscene flag 5, cleared on its
//! destroy outside a cutscene); the doors read, set and clear their switch flag `switchFlag`.

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DPM_ROTATE, DYNA_TRANSFORM_POS};
use eng_math::{scaled_step_to_s, step_to_f};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, audio_play_actor_sfx2, cur_sfx_pos};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_EV_SLIDE_DOOR_CLOSE, NA_SE_EV_SLIDE_DOOR_OPEN, NA_SE_EV_STONE_BOUND, NA_SE_OC_ABYSS, SFX_FLAG};
use oot_game::event_chk_inf;
use oot_game::item::{ITEM_HOOKSHOT, ITEM_NONE};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_RELAY_OBJECTS` (`actor_table.h`: 0x0123).
pub const ACTOR_BG_RELAY_OBJECTS: i16 = 0x0123;
/// `OBJECT_RELAY_OBJECTS` (`object_table.h`: 0x006C).
pub const OBJECT: &str = "object_relay_objects";

/// `Bg_Relay_Objects_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_RELAY_OBJECTS, name: "Bg_Relay_Objects", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(thisx->params, 0, 6)`: `switchFlag`.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "a door's: set while it's open (in room 0, less 0x33, the room it's for); the gear's unread");
/// Bits 6 and 7: the gear's 0xFF sets them; read by nothing.
pub const UNUSED_6: Field = Field::params("params bits 6 and 7", 6, 2, Kind::Unused, "set in the gear's 0xFF; read by nothing");
/// `PARAMS_GET_U(thisx->params, 8, 8)`: the type (`WindmillSetpiecesMode`), written over the
/// params at init.
pub const TYPE: Field =
    Field::params("type", 8, 8, Kind::Named(&[(0, "WINDMILL_ROTATING_GEAR"), (1, "WINDMILL_DAMPE_STONE_DOOR")]), "the windmill's turning gear or a stone door of Dampé's race");

/// Bg_Relay_Objects's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG, UNUSED_6, TYPE],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_65)).reads().sets().clears().note("the gear: set by the cutscene flag 5, cleared on its destroy outside a cutscene"),
        FlagUse::new(Flag::Switch("switchFlag")).reads().sets().clears().note("the doors: open while set"),
    ],
    todo: false,
    objects: &[],
};

/// `WindmillSetpiecesMode`.
pub const WINDMILL_ROTATING_GEAR: i16 = 0;
pub const WINDMILL_DAMPE_STONE_DOOR: i16 = 1;

/// `EVENTCHKINF_65` (`save.h`: 0x65): the windmill turning faster.
pub const EVENTCHKINF_65: u16 = 0x65;
/// `CS_INDEX_0` (`z64cutscene.h`: 0xFFF0).
const CS_INDEX_0: u16 = 0xFFF0;
/// `NA_SE_EV_WOOD_GEAR` (`environmentbank_table.h`: 0x2876).
pub const NA_SE_EV_WOOD_GEAR: u16 = 0x2876;
/// `NA_SE_EN_PO_LAUGH` (`enemybank_table.h`: 0x38EC).
pub const NA_SE_EN_PO_LAUGH: u16 = 0x38EC;

/// The overlay's statics: `D_808A9508`, the pieces already spawned (bit `unk_169` for a door, bit
/// 0 for the last door and those of room 6 or later, bit 1 for the gear).
#[derive(Debug, Default)]
pub struct Statics {
    pub d_808a9508: u32,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_808A90F4`: a door shut, waiting for its switch flag, then rising.
    Func808a90f4,
    /// `func_808A91AC`: a door open, its timer running.
    Func808a91ac,
    /// `func_808A9234`: a door falling shut.
    Func808a9234,
    DoNothing,
    /// `func_808A932C`: shut with Link elsewhere: the respawn after 5 frames.
    Func808a932c,
    /// `func_808A939C`: the gear turning.
    Func808a939c,
}

pub struct BgRelayObjects {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    pub switch_flag: u8,
    /// `unk_169`: the room the door is for.
    pub unk_169: i8,
    pub timer: i16,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

fn load_collision(play: &PlayState, symbol: &str) -> anyhow::Result<Arc<CollisionHeader>> {
    let assets = play.assets.as_ref().ok_or_else(|| anyhow::anyhow!("no asset pack"))?;
    Ok(Arc::new(assets.pack.collision(&keys::collision(OBJECT, symbol))?))
}

impl BgRelayObjects {
    /// `BgRelayObjects_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: gravity 5, ICHAIN_VEC3F_DIV1000(scale, 100).
        actor.gravity = 5.0;
        actor.scale = Vec3::splat(0.1);
        let switch_flag = SWITCH_FLAG.get(actor.params) as u8;
        actor.params = TYPE.get(actor.params);
        let mut this = BgRelayObjects { actor, bg: BG_ACTOR_MAX, action: Action::DoNothing, switch_flag, unk_169: 0, timer: 0 };
        // DynaPolyActor_Init(DYNA_TRANSFORM_POS | DYNA_TRANSFORM_ROT_Y).
        let symbol;
        // schema: the type, written over the params above
        if this.actor.params == WINDMILL_ROTATING_GEAR {
            symbol = "gWindmillRotatingPlatformCol";
            this.actor.world_rot.y = if play.save.get_event_chk_inf(EVENTCHKINF_65) { 0x400 } else { 0x80 };
            play.audio.audio_play_windmill_bgm();
            this.actor.room = -1;
            this.actor.flags |= ACTOR_FLAG_DRAW_CULLING_DISABLED;
            let s = play.overlay_static::<Statics>(ACTOR_BG_RELAY_OBJECTS);
            if s.d_808a9508 & 2 != 0 {
                this.actor.params = 0xFF;
                this.actor.kill();
            } else {
                s.d_808a9508 |= 2;
                this.action = Action::Func808a939c;
            }
        } else {
            symbol = "gDampeRaceDoorCol";
            this.unk_169 = if this.actor.room == 0 { (this.switch_flag as i32 - 0x33) as i8 } else { (this.actor.room as i32 + 1) as i8 };
            this.actor.room = -1;
            this.timer = 1;
            let unk_169 = this.unk_169;
            if unk_169 >= 6 {
                let s = play.overlay_static::<Statics>(ACTOR_BG_RELAY_OBJECTS);
                if s.d_808a9508 & 1 != 0 {
                    this.actor.kill();
                } else {
                    s.d_808a9508 |= 1;
                    this.action = Action::DoNothing;
                }
            } else if unk_169 != 5 {
                play.flags.unset_switch(this.switch_flag as i32);
                let s = play.overlay_static::<Statics>(ACTOR_BG_RELAY_OBJECTS);
                let bit = 1u32.wrapping_shl(unk_169 as u32);
                if s.d_808a9508 & bit != 0 {
                    this.actor.kill();
                } else {
                    s.d_808a9508 |= bit;
                    this.action = Action::Func808a90f4;
                }
            } else {
                play.flags.set_switch(this.switch_flag as i32);
                this.action = Action::Func808a91ac;
                this.actor.world_pos.y += 120.0;
                play.overlay_static::<Statics>(ACTOR_BG_RELAY_OBJECTS).d_808a9508 |= 1;
            }
        }
        match load_collision(play, symbol) {
            Ok(h) => this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), DYNA_TRANSFORM_POS | DPM_ROTATE),
            Err(e) => log::error!("Bg_Relay_Objects: {e:#}"),
        }
        Box::new(this)
    }

    /// `func_808A90F4`: the switch flag set: the door rises to 120 above home at 12 a frame (its
    /// time open set on the first frame: 120 with the hookshot, else 160).
    fn func_808a90f4(&mut self, play: &mut PlayState) {
        if play.flags.get_switch(self.switch_flag as i32) {
            if self.timer != 0 {
                audio_play_actor_sfx2(play, NA_SE_EV_SLIDE_DOOR_OPEN);
                self.timer = if play.save.inv_content(ITEM_HOOKSHOT) != ITEM_NONE { 120 } else { 160 };
            }
            if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 120.0, 12.0) {
                self.action = Action::Func808a91ac;
            }
        }
    }

    /// `func_808A91AC`: open, the timer ticking down (not the last door's), shut at 0 or once
    /// Link is in the door's room.
    fn func_808a91ac(&mut self, play: &mut PlayState) {
        if self.unk_169 != 5 {
            if self.timer != 0 {
                self.timer -= 1;
            }
            self.actor.play_sfx_flagged_timer(self.timer as i32);
        }
        if self.timer == 0 || self.unk_169 as i32 == play.room_ctx.cur.num as i32 {
            audio_play_actor_sfx2(play, NA_SE_EV_SLIDE_DOOR_CLOSE);
            self.action = Action::Func808a9234;
        }
    }

    /// `func_808A9234`: falling, 5 more a frame, to home; then with Link in another room, Dampé's
    /// laugh and the respawn, else the switch flag cleared and done.
    fn func_808a9234(&mut self, play: &mut PlayState) {
        self.actor.velocity.y += self.actor.gravity;
        if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, self.actor.velocity.y) {
            // Rumble_Request(xyzDistToPlayerSq, 180, 20, 100): no rumble.
            audio_play_actor_sfx2(play, NA_SE_EV_STONE_BOUND);
            let cur_room = play.room_ctx.cur.num as i32;
            if self.unk_169 as i32 != cur_room {
                play.audio.play_sfx_centered2(NA_SE_EN_PO_LAUGH);
                self.timer = 5;
                self.action = Action::Func808a932c;
                return;
            }
            play.flags.unset_switch(self.switch_flag as i32);
            self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            if cur_room == 4 {
                play.save.timer_state = oot_game::save::TIMER_STATE_UP_FREEZE;
            }
            self.action = Action::DoNothing;
        }
    }

    /// `func_808A932C`: 5 frames on, out of a cutscene, Link sent back (`Play_TriggerRespawn`).
    fn func_808a932c(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 && !play.player_in_cs_mode() {
            play.audio.play_sfx_centered(NA_SE_OC_ABYSS);
            play.trigger_respawn();
            self.action = Action::DoNothing;
        }
    }

    /// `func_808A939C`: the gear, its speed stepped to 0x400 with `EVENTCHKINF_65` (set by the
    /// cutscene flag 5), else to 0x80; the gear's sound pitched by it (1 at 0x80, 2 at 0x400).
    fn func_808a939c(&mut self, play: &mut PlayState) {
        if play.flags_get_env(5) {
            play.save.set_event_chk_inf(EVENTCHKINF_65);
        }
        let target = if play.save.get_event_chk_inf(EVENTCHKINF_65) { 0x400 } else { 0x80 };
        scaled_step_to_s(&mut self.actor.world_rot.y, target, 8);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.actor.world_rot.y);
        let pos = cur_sfx_pos(play);
        play.audio.func_800f436c(pos, NA_SE_EV_WOOD_GEAR - SFX_FLAG, ((self.actor.world_rot.y as i32 - 0x80) as f32 * (1.0 / 0x380 as f32)) + 1.0);
    }
}

impl ActorImpl for BgRelayObjects {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgRelayObjects_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Func808a90f4 => self.func_808a90f4(play),
            Action::Func808a91ac => self.func_808a91ac(play),
            Action::Func808a9234 => self.func_808a9234(play),
            Action::DoNothing => {}
            Action::Func808a932c => self.func_808a932c(play),
            Action::Func808a939c => self.func_808a939c(play),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }
    /// `BgRelayObjects_Destroy`: the gear, out of a cutscene's entrance, slows the windmill again.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
        // schema: the type, written over the params at init
        if self.actor.params == WINDMILL_ROTATING_GEAR && play.save.cutscene_index < CS_INDEX_0 {
            play.save.clear_event_chk_inf(EVENTCHKINF_65);
        }
    }
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `BgRelayObjects_Draw`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        // schema: the type, written over the params at init
        let symbol = if self.actor.params == WINDMILL_ROTATING_GEAR { "gWindmillRotatingPlatformDL" } else { "gDampeRaceDoorDL" };
        crate::gfx_draw_dlist_opa(out, OBJECT, symbol, rs);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
