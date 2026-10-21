//! `En_Ex_Ruppy` (`ovl_En_Ex_Ruppy/z_en_ex_ruppy.c`): rupees other actors spawn. Here, the guards'
//! courtyard's eight round the path 3 guard (`En_Heishi1`): each green, or blue (3 in 31) or red
//! (1 in 31) by a `Rand` at its init, spinning, falling to the ground; Link within 30 takes it
//! (`NA_SE_SY_GET_RUPY`, an `En_Item00` of its colour dropped to be taken at once).
//!
//! **The diving game's** (type 0, `En_Diving_Game`'s children, thrown from the top of Zora's
//! Domain's falls): room -1, 0.01 big, blue (5 rupees) until the silver scale is won; after it by a
//! `Rand_ZeroFloat(200.99)`: green (1) under 40, blue under 170, red (20) under 190, orange (50)
//! under 200, else a big (0.02) pink 500. @bug (game): the init adds the parent's `extraWinCount`
//! (×10) to the range and a pink one zeroes it, but `Actor_SpawnAsChild` links the parent only
//! after the init has run: `actor.parent` is always NULL there, so neither ever happens. Its
//! `world.rot.z` is its `throwDistance` (zeroed). It **drops** (`EnExRuppy_DropIntoWater`: spinning
//! 0x7A8 more, gravity easing to -2, two yellow sparkles a frame, `Effect_Ss_KiraKira`,
//! `NA_SE_EV_RAINBOW_SHOWER`) until the parent's `throwTimer` is out, it's in water or its own 30
//! frames are; then it's hidden and stopped (`NA_SE_EV_BOMB_DROP_WATER`) and **waits**
//! (`EnExRuppy_EnterWater`) for the parent's sinking (`rupeePhase` 2): shown again at x -260 ± 150,
//! y 370 ± 100, z 50 back a `throwDistance` from -500 (± 40; after the scale from -300, ± 30) in the
//! pool below, falling (gravity -1). 15 under the surface it **sinks** (`EnExRuppy_Sink`: a splash
//! at the surface, `Effect_Ss_G_Splash` 800, the sound, -1 down, gravity -0.2) and **waits on the
//! bottom** (`EnExRuppy_WaitInGame`) breathing a bubble every 10 frames (`Effect_Ss_Bubble`). Link
//! within 30 takes it: `Rupees_ChangeBy` its value, `NA_SE_SY_GET_RUPY`, the parent's
//! `grabbedRupees` counted up, gone. The game over (the parent's `phase` 0) while it sinks or
//! waits, it **blinks out** (`EnExRuppy_Kill`, 20 frames); its parent gone, it's gone.
//!
//! The whole overlay is ported. The shooting gallery's progress markers are raised by their
//! `galleryFlag`, which `En_Syateki_Man` sets (not ported); the bomb rupees are unused in the game.
//!
//! Params: `type` whole (`EnExRuppyType`: 0 the diving game's, 1 and 2 the large and small bomb
//! rupees, 3 the courtyard's, 4 the shooting gallery's progress markers); spawned by
//! `En_Diving_Game` (0), `En_Dnt_Nomal` and `En_Heishi1` (3), `En_Syateki_Itm` (4).
//! `world.rot.z`: the throw's distance (type 0, `throwDistance`; init zeroes it).
//! Flags: reads EVENTCHKINF_OBTAINED_SILVER_SCALE (type 0: 5 rupees each until the scale is won,
//! and where they sink).

use eng_gfx::{DrawCmd, MeshKey};
use eng_math::approach_f;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_WATER, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind, Source};
use oot_game::event_chk_inf;
use oot_game::audio::sfx::{NA_SE_EV_BOMB_DROP_WATER, NA_SE_SY_GET_RUPY, SFX_FLAG, SfxPos};
use oot_game::item::rupees_change_by;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

use crate::en_diving_game::{ENDIVINGGAME_PHASE_ENDED, ENDIVINGGAME_RUPEE_PHASE_SINKING, EnDivingGame};
use crate::en_item00::{ITEM00_RUPEE_BLUE, ITEM00_RUPEE_GREEN, ITEM00_RUPEE_ORANGE, ITEM00_RUPEE_PURPLE, ITEM00_RUPEE_RED, item_drop_collectible, rupee_bake};

/// `ACTOR_EN_EX_RUPPY` (`actor_table.h`: 0x0131).
pub const ACTOR_EN_EX_RUPPY: i16 = 0x0131;

/// `En_Ex_Ruppy_Profile`: `FLAGS` `ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_EX_RUPPY, name: "En_Ex_Ruppy", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep", schema: &SCHEMA };

// EnExRuppyType.
pub const ENEXRUPPY_TYPE_DIVING_GAME: i16 = 0;
pub const ENEXRUPPY_TYPE_BOMB_LARGE: i16 = 1;
pub const ENEXRUPPY_TYPE_BOMB_SMALL: i16 = 2;
pub const ENEXRUPPY_TYPE_HYRULE_COURTYARD: i16 = 3;
pub const ENEXRUPPY_TYPE_SHOOTING_GALLERY_PROGRESS: i16 = 4;

/// `EVENTCHKINF_OBTAINED_SILVER_SCALE` (`save.h`: 0x38).
const EVENTCHKINF_OBTAINED_SILVER_SCALE: u16 = 0x38;

/// `this->type = this->actor.params`.
pub const TYPE: Field = Field::whole(
    "type",
    Kind::Named(&[
        (0, "ENEXRUPPY_TYPE_DIVING_GAME"),
        (1, "ENEXRUPPY_TYPE_BOMB_LARGE"),
        (2, "ENEXRUPPY_TYPE_BOMB_SMALL"),
        (3, "ENEXRUPPY_TYPE_HYRULE_COURTYARD"),
        (4, "ENEXRUPPY_TYPE_SHOOTING_GALLERY_PROGRESS"),
    ]),
    "which rupee",
);
/// `this->throwDistance = this->actor.world.rot.z`.
pub const THROW_DISTANCE: Field =
    Field::rot("throwDistance", Source::RotZ, 0, 16, Kind::Range(-0x8000, 0x7FFF), "the diving game's throw: where it lands, 50 back a unit (En_Diving_Game's rupees left)").when("type", &[0]);

/// En_Ex_Ruppy's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, THROW_DISTANCE],
    flags: &[FlagUse::new(event_chk_inf!(EVENTCHKINF_OBTAINED_SILVER_SCALE)).reads().note("type 0: 5 rupees each until it's won, and where it sinks")],
    todo: false,
    objects: &[],
    texts: &[],
};
/// `NA_SE_EV_RAINBOW_SHOWER` (`environmentbank_table.h`: 0x28BC).
const NA_SE_EV_RAINBOW_SHOWER: u16 = 0x28BC;
/// `NA_SE_IT_BOMB_EXPLOSION` (`itembank_table.h`: 0x180E).
const NA_SE_IT_BOMB_EXPLOSION: u16 = 0x180E;

/// `sEnExRuppyCollectibleTypes`: by `colorIdx`.
const S_EN_EX_RUPPY_COLLECTIBLE_TYPES: [i16; 5] = [ITEM00_RUPEE_GREEN, ITEM00_RUPEE_BLUE, ITEM00_RUPEE_RED, ITEM00_RUPEE_ORANGE, ITEM00_RUPEE_PURPLE];

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// The type had no case at init: `actionFunc` is NULL (a crash in the C; no type does this).
    None,
    DropIntoWater,
    WaitToBlowUp,
    WaitAsCollectible,
    GalleryTarget,
    EnterWater,
    Sink,
    WaitInGame,
    Kill,
}

pub struct EnExRuppy {
    pub actor: Actor,
    pub action: Action,
    /// `colorIdx`: the texture (`gRupeeGreenTex`, `Blue`, `Red`, `Pink`, `Orange`).
    pub color_idx: i16,
    pub ty: i16,
    pub invisible: i16,
    pub timer: i16,
    pub rupee_value: i16,
    pub throw_distance: i16,
    /// `galleryFlag`: set by the shooting gallery's man (not ported).
    pub gallery_flag: i16,
    pub scale: f32,
}

impl EnExRuppy {
    /// `EnExRuppy_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnExRuppy { action: Action::None, color_idx: 0, ty: TYPE.get(actor.params), invisible: 0, timer: 0, rupee_value: 0, throw_distance: 0, gallery_flag: 0, scale: 0.0, actor };
        log::debug!("☆☆☆☆☆ Index ☆☆☆☆☆ {:x}", this.ty);
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 25.0);
        match this.ty {
            ENEXRUPPY_TYPE_DIVING_GAME => {
                this.scale = 0.01;
                this.actor.scale = Vec3::splat(this.scale);
                this.actor.room = -1;
                this.actor.gravity = 0.0;
                // Until the diving game has been won, always 5 rupees.
                if !play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE) {
                    this.rupee_value = 5;
                    this.color_idx = 1;
                } else {
                    // @bug (game): the parent isn't linked yet (Actor_SpawnAsChild sets it after the
                    // init), so the extraWinCount never adds to the range.
                    let mut rand_scale = 200.99;
                    if let Some(diving_game) = this.actor.parent.and_then(|h| play.actors.downcast::<EnDivingGame>(h)).filter(|g| !g.actor.killed) {
                        let extra_win_factor = diving_game.extra_win_count as f32 * 10.0;
                        rand_scale += extra_win_factor;
                    }
                    let diving_rupee_type = play.rand.zero_float(rand_scale) as i16;
                    let (value, color) = match diving_rupee_type {
                        0..40 => (1, 0),
                        40..170 => (5, 1),
                        170..190 => (20, 2),
                        190..200 => (50, 4),
                        _ => {
                            this.scale = 0.02;
                            this.actor.scale = Vec3::splat(this.scale);
                            // As above: never a parent here to zero.
                            if let Some(diving_game) = this.actor.parent.and_then(|h| play.actors.downcast_mut::<EnDivingGame>(h)).filter(|g| !g.actor.killed) {
                                diving_game.extra_win_count = 0;
                            }
                            (500, 3)
                        }
                    };
                    this.rupee_value = value;
                    this.color_idx = color;
                }
                this.actor.shape_shadow_scale = 7.0;
                this.actor.shape_y_offset = 700.0;
                this.throw_distance = THROW_DISTANCE.get(this.actor.world_rot.z);
                this.actor.world_rot.z = 0;
                this.timer = 30;
                this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                this.action = Action::DropIntoWater;
            }
            ENEXRUPPY_TYPE_BOMB_LARGE | ENEXRUPPY_TYPE_BOMB_SMALL => {
                if this.ty == ENEXRUPPY_TYPE_BOMB_LARGE {
                    this.actor.scale = Vec3::splat(0.1);
                    this.color_idx = 4;
                } else {
                    this.actor.scale = Vec3::splat(0.02);
                    this.color_idx = (play.rand.zero_float(3.99) as i16) + 1;
                }
                this.actor.gravity = -3.0;
                log::debug!("☆☆☆☆☆ Wow Coin ☆☆☆☆☆");
                this.actor.shape_shadow_scale = 6.0;
                this.actor.shape_y_offset = 700.0;
                this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                this.action = Action::WaitToBlowUp;
            }
            ENEXRUPPY_TYPE_HYRULE_COURTYARD => {
                this.actor.scale = Vec3::splat(0.02);
                this.color_idx = 0;
                match play.rand.zero_float(30.99) as i16 {
                    0 => this.color_idx = 2,
                    10 | 20 | 30 => this.color_idx = 1,
                    _ => {}
                }
                this.actor.gravity = -3.0;
                log::debug!("☆☆☆☆☆ Normal rupee ☆☆☆☆☆");
                this.actor.shape_shadow_scale = 6.0;
                this.actor.shape_y_offset = 700.0;
                this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                this.action = Action::WaitAsCollectible;
            }
            ENEXRUPPY_TYPE_SHOOTING_GALLERY_PROGRESS => {
                this.actor.gravity = -3.0;
                this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                this.actor.scale = Vec3::splat(0.01);
                this.actor.shape_shadow_scale = 6.0;
                this.actor.shape_y_offset = -700.0;
                this.action = Action::GalleryTarget;
            }
            _ => {}
        }
        Box::new(this)
    }

    /// The parent as the C reads it: `(divingGame != NULL) && (divingGame->actor.update != NULL)`.
    /// `Err` for a parent whose update is gone (killed, or deleted).
    fn diving_game<'a>(&self, play: &'a mut PlayState) -> Option<Result<&'a mut EnDivingGame, ()>> {
        let h = self.actor.parent?;
        Some(play.actors.downcast_mut::<EnDivingGame>(h).filter(|g| !g.actor.killed).ok_or(()))
    }

    /// `EnExRuppy_SpawnSparkles`: `numSparkles` yellow sparkles about it (±5, `scale` × 600 up),
    /// rising (movement type 0) or scattering (1: a random spread, larger and longer).
    fn spawn_sparkles(&mut self, play: &mut PlayState, num_sparkles: i16, movement_type: i32) {
        // velocities, accelerations.
        const VELOCITIES: [Vec3; 2] = [Vec3::new(0.0, 0.1, 0.0), Vec3::ZERO];
        const ACCELERATIONS: [Vec3; 2] = [Vec3::new(0.0, 0.01, 0.0), Vec3::ZERO];
        let num_sparkles = num_sparkles.max(1);
        // @bug (game): neither colour's alpha is set (the stack's leftovers; the env alpha is the
        // sparkle's starting pulse): 0 here.
        let prim_color = [255, 255, 0, 0];
        let env_color = [255, 255, 255, 0];
        let velocity = VELOCITIES[movement_type as usize];
        let mut accel = ACCELERATIONS[movement_type as usize];
        let mut scale = 3000;
        let mut life = 16;
        for _ in 0..num_sparkles {
            if movement_type == 1 {
                accel.x = play.rand.centered_float(20.0);
                accel.z = play.rand.centered_float(20.0);
                scale = 5000;
                life = 20;
            }
            let p = self.actor.world_pos;
            let x = (play.rand.zero_one() - 0.5) * 10.0 + p.x;
            let y = (play.rand.zero_one() - 0.5) * 10.0 + (p.y + self.scale * 600.0);
            let z = (play.rand.zero_one() - 0.5) * 10.0 + p.z;
            play.with_ss(|s| s.kirakira_spawn_dispersed(Vec3::new(x, y, z), velocity, accel, prim_color, env_color, scale, life));
        }
    }

    /// `EnExRuppy_DropIntoWater`: spinning down, sparkling, until the parent's `throwTimer` is
    /// out, it's in water or its own time is up: then hidden and stopped, waiting.
    fn drop_into_water(&mut self, play: &mut PlayState) {
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x7A8);
        approach_f(&mut self.actor.gravity, -2.0, 0.3, 1.0);
        self.spawn_sparkles(play, 2, 0);
        play.audio.play_sfx_centered(NA_SE_EV_RAINBOW_SHOWER - SFX_FLAG);
        let (in_water, timer) = (self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0, self.timer);
        if let Some(Ok(diving_game)) = self.diving_game(play)
            && (diving_game.throw_timer == 0 || in_water || timer == 0)
        {
            self.invisible = 1;
            self.actor.speed_xz = 0.0;
            self.actor.velocity = Vec3::ZERO;
            self.actor.gravity = 0.0;
            if let Some(me) = play.cur_actor {
                play.audio.play_sfx_at_pos(SfxPos::Actor(me), NA_SE_EV_BOMB_DROP_WATER);
            }
            self.action = Action::EnterWater;
        }
    }

    /// `EnExRuppy_EnterWater`: hidden until the parent's rupees sink (`rupeePhase` 2): then shown in
    /// the pool below the falls, by its `throwDistance`, falling.
    fn enter_water(&mut self, play: &mut PlayState) {
        let sinking = matches!(self.diving_game(play), Some(Ok(g)) if g.rupee_phase == ENDIVINGGAME_RUPEE_PHASE_SINKING);
        if sinking {
            self.invisible = 0;
            self.actor.world_pos.x = ((play.rand.zero_one() - 0.5) * 300.0) + -260.0;
            self.actor.world_pos.y = ((play.rand.zero_one() - 0.5) * 200.0) + 370.0;
            let mut throw_distance = self.throw_distance as f32 * -50.0;
            if !play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE) {
                throw_distance += -500.0;
                self.actor.world_pos.z = ((play.rand.zero_one() - 0.5) * 80.0) + throw_distance;
            } else {
                throw_distance += -300.0;
                self.actor.world_pos.z = ((play.rand.zero_one() - 0.5) * 60.0) + throw_distance;
            }
            self.action = Action::Sink;
            self.actor.gravity = -1.0;
        }
    }

    /// `EnExRuppy_Sink`: more than 15 under the surface, a splash there and the slow sinking to the
    /// bottom; the game over, blinking out.
    fn sink(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 && self.actor.y_dist_to_water > 15.0 {
            let mut pos = self.actor.world_pos;
            pos.y += self.actor.y_dist_to_water;
            self.actor.velocity.y = -1.0;
            self.actor.gravity = -0.2;
            play.with_ss(|s| s.g_splash_spawn(pos, None, None, 0, 800));
            if let Some(me) = play.cur_actor {
                play.audio.play_sfx_at_pos(SfxPos::Actor(me), NA_SE_EV_BOMB_DROP_WATER);
            }
            self.action = Action::WaitInGame;
        }
        if matches!(self.diving_game(play), Some(Ok(g)) if g.phase == ENDIVINGGAME_PHASE_ENDED) {
            self.timer = 20;
            self.action = Action::Kill;
        }
    }

    /// `EnExRuppy_WaitInGame`: a bubble every 10 frames; the game over, blinking out; Link within
    /// 30 takes it and the parent counts it; the parent gone, gone.
    fn wait_in_game(&mut self, play: &mut PlayState) {
        let collect_radius = 30.0;
        if self.timer == 0 {
            self.timer = 10;
            let scale = play.rand.zero_float(0.03) + 0.07;
            let pos = self.actor.world_pos;
            play.with_ss(|s| s.bubble_spawn(pos, 0.0, 5.0, 5.0, scale));
        }
        let (dist_sq, value) = (self.actor.xyz_dist_to_player_sq, self.rupee_value);
        match self.diving_game(play) {
            None => {}
            Some(Ok(diving_game)) => {
                if diving_game.phase == ENDIVINGGAME_PHASE_ENDED {
                    self.timer = 20;
                    self.action = Action::Kill;
                } else if dist_sq < collect_radius * collect_radius {
                    diving_game.grabbed_rupees += 1;
                    rupees_change_by(&mut play.save, value);
                    play.audio.play_sfx_centered(NA_SE_SY_GET_RUPY);
                    self.actor.kill();
                }
            }
            Some(Err(())) => self.actor.kill(),
        }
    }

    /// `EnExRuppy_Kill`: blinking (shown every other frame) until the timer's out, then gone.
    fn kill(&mut self) {
        self.invisible += 1;
        // Net effect is invisible = !invisible.
        self.invisible &= 1;
        if self.timer == 0 {
            self.actor.kill();
        }
    }

    /// `EnExRuppy_WaitToBlowUp`: Link within 50 (30 for the small one): the parent's
    /// `linkDetected` set (an `En_Heishi1`'s, supposedly), the explosion, Link knocked back.
    fn wait_to_blow_up(&mut self, play: &mut PlayState) {
        let dist_to_blow_up = if self.ty == ENEXRUPPY_TYPE_BOMB_SMALL { 30.0 } else { 50.0 };
        if self.actor.xyz_dist_to_player_sq < dist_to_blow_up * dist_to_blow_up {
            match self.actor.parent.and_then(|h| play.actors.get_mut(h)) {
                Some(parent) => parent.solder_srch_ball_detect(),
                None => log::debug!("☆☆☆☆☆ That's stupid! Error!!!!! ☆☆☆☆☆"),
            }
            log::debug!("☆☆☆☆☆ Stupid! ☆☆☆☆☆");
            let (scale, step) = if self.ty == ENEXRUPPY_TYPE_BOMB_SMALL { (20, 6) } else { (100, 30) };
            let pos = self.actor.world_pos;
            // EnExRuppy_WaitToBlowUp's `accel` { 0, 0.1, 0 }, `velocity` 0.
            play.with_ss(|s| s.bomb2_spawn_layered(pos, Vec3::ZERO, Vec3::new(0.0, 0.1, 0.0), scale, step));
            actor_set_player_knockback_large_no_damage(play, 2.0, self.actor.yaw_towards_player, 0.0);
            audio_play_actor_sfx2(play, NA_SE_IT_BOMB_EXPLOSION);
            self.actor.kill();
        }
    }

    /// `EnExRuppy_WaitAsCollectible`: Link within 30 takes it.
    fn wait_as_collectible(&mut self, play: &mut PlayState) {
        let collect_radius = 30.0;
        if self.actor.xyz_dist_to_player_sq < collect_radius * collect_radius {
            play.audio.play_sfx_centered(NA_SE_SY_GET_RUPY);
            item_drop_collectible(play, self.actor.world_pos, S_EN_EX_RUPPY_COLLECTIBLE_TYPES[self.color_idx as usize % 5] | i16::MIN);
            self.actor.kill();
        }
    }

    /// `EnExRuppy_GalleryTarget`: raised while its `galleryFlag` is set.
    fn gallery_target(&mut self) {
        let target = if self.gallery_flag != 0 { 700.0 } else { -700.0 };
        approach_f(&mut self.actor.shape_y_offset, target, 0.5, 200.0);
    }
}

impl ActorImpl for EnExRuppy {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnExRuppy_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(1960);
        match self.action {
            Action::None => log::error!("En_Ex_Ruppy: type {} has no action (a NULL actionFunc in the C)", self.ty),
            Action::DropIntoWater => self.drop_into_water(play),
            Action::WaitToBlowUp => self.wait_to_blow_up(play),
            Action::WaitAsCollectible => self.wait_as_collectible(play),
            Action::GalleryTarget => self.gallery_target(),
            Action::EnterWater => self.enter_water(play),
            Action::Sink => self.sink(play),
            Action::WaitInGame => self.wait_in_game(play),
            Action::Kill => self.kill(),
        }
        if self.timer != 0 {
            self.timer -= 1;
        }
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 20.0, 20.0, 50.0, UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![(self.invisible != 0) as u32, self.color_idx as u32];
        rs
    }
    /// `EnExRuppy_Draw`: `gRupeeDL` with its colour's texture on segment 8 (`func_8002EBCC`'s
    /// highlight is the mesh's texgen), `En_Item00`'s rupee bakes.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.first().copied().unwrap_or(1) != 0 {
            return;
        }
        let i = rs.switches.get(1).copied().unwrap_or(0) as usize % 5;
        out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(&rupee_bake(i))), actor_draw_matrix(rs)));
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
