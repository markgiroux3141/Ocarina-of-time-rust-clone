//! `Bg_Haka` (`ovl_Bg_Haka/z_bg_haka.c`): the gravestone Link pulls back, a DynaPoly actor of
//! `object_haka` (`gGravestoneCol`, `DynaPolyActor_Init(0)`) drawn with `gGravestoneStoneDL` and,
//! translucent, the earth over the hole (`gGravestoneEarthDL`) left behind as it slides.
//!
//! Player at its wall adds his pull (-2 a frame) or push (+2) to `dyna.unk_150` and sets
//! `dyna.unk_158` to his yaw (`func_8002DFA4`). In `BgHaka_IdleClosed` a push is refused (its
//! `unk_150` zeroed, Player's `PLAYER_STATE2_4` cleared), and so is any pull in the graveyard by
//! day as a child (with the Graveyard Boy's warning, 0x5073, and 100 frames locked) or at Lake
//! Hylia as a child before switch flag 0x23; else a pull starts `BgHaka_Pull`: it slides away
//! from its face (`world.rot.y = shape.rot.y + 0x8000`) 60 from home, at 0.05 a frame faster each
//! frame up to 1.5, `NA_SE_EV_ROCK_SLIDE` all the way. At 60 it lets go of Link; params 1 plays
//! the chime, and at night in the graveyard a Poe (`En_Poh`, a placeholder) rises from the hole.
//! `minVelocityY` keeps the distance pulled. Link standing on the dirt patch behind it (in the
//! closed actions) walks on sand's sound (`PLAYER_STATE2_FORCE_SAND_FLOOR_SOUND`, Player's
//! `STATE2_9`).
//!
//! The Master Quest Deku Tree's are room 7's eight, params 0, sunk 15 into the floor (at -775,
//! the floor at -760): their faces stand under the 39 `Player_ActionHandler_5` needs to hold on, so
//! there Link climbs onto them and the pull is reached only by injection (`func_8002DFA4`); a
//! stone flush with the floor, as the graveyard places them, is pulled by Player
//! (`oot_actors --test push`). Category BG: it updates before Player, so the `unk_150` it reads is
//! the one Player wrote last frame. The whole overlay is ported; the cull zone isn't, for any
//! actor.

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, SFX_FLAG};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_BG_HAKA` (`actor_table.h`: 0x009D).
pub const ACTOR_BG_HAKA: i16 = 0x009D;
pub const OBJECT: &str = "object_haka";
/// `OBJECT_HAKA` (`object_table.h`: 0x00A2).
pub const OBJECT_HAKA: i16 = 0x00A2;
const COLLISION: &str = "gGravestoneCol";
const STONE_DL: &str = "gGravestoneStoneDL";
const EARTH_DL: &str = "gGravestoneEarthDL";

/// `ACTOR_EN_POH` (`actor_table.h`: 0x000D), the Poe: not ported (a placeholder spawns).
pub const ACTOR_EN_POH: i16 = 0x000D;
/// `NA_SE_EV_ROCK_SLIDE` (`environmentbank_table.h`: 0x280A).
pub const NA_SE_EV_ROCK_SLIDE: u16 = 0x280A;
/// `SCENE_GRAVEYARD`, `SCENE_LAKE_HYLIA` (`scene_table.h`: 0x53, 0x57).
pub const SCENE_GRAVEYARD: u16 = 0x53;
pub const SCENE_LAKE_HYLIA: u16 = 0x57;
/// `PLAYER_STATE2_4` (`player.h`: 1 << 4): Player pushing or pulling.
pub const PLAYER_STATE2_4: u32 = 1 << 4;
/// `PLAYER_STATE2_FORCE_SAND_FLOOR_SOUND` (`player.h`: 1 << 9; the port's `player::STATE2_9`).
pub const PLAYER_STATE2_FORCE_SAND_FLOOR_SOUND: u32 = 1 << 9;
/// The Graveyard Boy's warning (`Message_StartTextbox(play, 0x5073, NULL)`).
pub const TEXT_GRAVEYARD_BOY_WARNING: u16 = 0x5073;

/// `Bg_Haka_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_HAKA, name: "Bg_Haka", category: ACTORCAT_BG, flags: 0, object: OBJECT };

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `BgHaka_IdleClosed`.
    IdleClosed,
    /// `BgHaka_Pull`.
    Pull,
    /// `BgHaka_IdleOpened`.
    IdleOpened,
    /// `BgHaka_IdleLockedClosed`.
    IdleLockedClosed,
}

pub struct BgHaka {
    /// `dyna.actor`, `dyna.bgId`. `actor.speed_xz` is the C's `actor.speed`, `actor.min_velocity_y`
    /// its `minVelocityY` (the distance pulled), `actor.params` the warning's cooldown once locked.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `player->stateFlags2`: `set` added, `clear` taken off.
fn change_player_state2(play: &mut PlayState, set: u32, clear: u32) {
    if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        p.change_state_flags2(set, clear);
    }
}

impl BgHaka {
    /// `BgHaka_Init`: `minVelocityY` 0, scale 0.1 (`sInitChain`), `DynaPolyActor_Init(0)`,
    /// `gGravestoneCol`, `BgHaka_IdleClosed`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ICHAIN_F32(minVelocityY, 0), ICHAIN_VEC3F_DIV1000(scale, 100).
        actor.min_velocity_y = 0.0;
        actor.scale = Vec3::splat(0.1);
        // DynaPolyActor_Init(&this->dyna, 0): no transform flags; unk_150 and the interact flags
        // zeroed with the bg actor.
        let bg = match crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            Some(h) => play.col.dyna.set_bg_actor(h, source(&actor), 0),
            None => BG_ACTOR_MAX,
        };
        Box::new(BgHaka { actor, bg, action: Action::IdleClosed })
    }

    /// `dyna.unk_150`.
    fn unk_150(&self, play: &PlayState) -> f32 {
        play.col.dyna.unk_150(self.bg)
    }

    /// `this->dyna.unk_150 = 0.0f; player->stateFlags2 &= ~PLAYER_STATE2_4;`: Link let go of.
    fn release(&self, play: &mut PlayState) {
        play.col.dyna.set_unk_150(self.bg, 0.0);
        change_player_state2(play, 0, PLAYER_STATE2_4);
    }

    /// `BgHaka_CheckPlayerOnDirtPatch`: Link within 34.6 across of its middle and 36 to 112.8
    /// behind it (in its frame, `Actor_WorldToActorCoords`) walks on sand's sound.
    fn check_player_on_dirt_patch(&self, play: &mut PlayState) {
        let Some(player_pos) = play.player.and_then(|h| play.actors.actor(h)).map(|p| p.world_pos) else { return };
        let player_relative_pos = self.actor.world_to_actor_coords(player_pos);
        if player_relative_pos.x.abs() < 34.6 && player_relative_pos.z > -112.8 && player_relative_pos.z < -36.0 {
            change_player_state2(play, PLAYER_STATE2_FORCE_SAND_FLOOR_SOUND, 0);
        }
    }

    /// `BgHaka_IdleClosed`: a push or pull on it (`unk_150` not 0): in the graveyard by day as a
    /// child, refused with the Graveyard Boy's warning (unless a cutscene's on: then only
    /// refused) and locked 100 frames (`params` the cooldown); a push (`unk_150` > 0), or a pull at
    /// Lake Hylia as a child before switch flag 0x23, refused; else the pull starts, away from
    /// its face. Then the dirt patch.
    fn idle_closed(&mut self, play: &mut PlayState) {
        let unk_150 = self.unk_150(play);
        if unk_150 != 0.0 {
            if play.scene_id == SCENE_GRAVEYARD && !play.save.adult && play.save.is_day() {
                self.release(play);
                if !play.play_in_cs_mode() {
                    play.start_textbox(TEXT_GRAVEYARD_BOY_WARNING, None);
                    // Used as a cooldown for displaying Graveyard Boy's warning.
                    self.actor.params = 100;
                    self.action = Action::IdleLockedClosed;
                }
            } else if 0.0 < unk_150 || (play.scene_id == SCENE_LAKE_HYLIA && !play.save.adult && !play.flags.get_switch(0x23)) {
                self.release(play);
            } else {
                self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(i16::MIN);
                self.action = Action::Pull;
            }
        }
        self.check_player_on_dirt_patch(play);
    }

    /// `BgHaka_Pull`: `speed` 0.05 faster, up to 1.5; `minVelocityY` (the distance pulled) that much
    /// nearer 60 (`Math_StepToF`); the position that far from home along `world.rot.y`. At 60: Link
    /// let go of, params 1's chime, or at night in the graveyard a Poe (`En_Poh` params 1) at home
    /// turned as the stone; `BgHaka_IdleOpened`. `NA_SE_EV_ROCK_SLIDE` each frame.
    fn pull(&mut self, play: &mut PlayState) {
        let a = &mut self.actor;
        a.speed_xz += 0.05;
        // CLAMP_MAX(speed, 1.5f).
        a.speed_xz = if a.speed_xz > 1.5 { 1.5 } else { a.speed_xz };
        let reached_max_pull_dist = step_to_f(&mut a.min_velocity_y, 60.0, a.speed_xz);
        a.world_pos.x = sin_s(a.world_rot.y) * a.min_velocity_y + a.home_pos.x;
        a.world_pos.z = cos_s(a.world_rot.y) * a.min_velocity_y + a.home_pos.z;
        if reached_max_pull_dist {
            self.release(play);
            if self.actor.params == 1 {
                play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            } else if !play.save.is_day() && play.scene_id == SCENE_GRAVEYARD {
                let h = self.actor.home_pos;
                let _ = play.actor_spawn(ACTOR_EN_POH, h, [0, self.actor.shape_rot.y, 0], 1);
            }
            self.action = Action::IdleOpened;
        }
        // Actor_PlaySfx_Flagged(&this->dyna.actor, NA_SE_EV_ROCK_SLIDE - SFX_FLAG).
        self.actor.play_sfx_flagged(NA_SE_EV_ROCK_SLIDE - SFX_FLAG);
    }

    /// `BgHaka_IdleOpened`: any push or pull refused.
    fn idle_opened(&mut self, play: &mut PlayState) {
        if self.unk_150(play) != 0.0 {
            self.release(play);
        }
    }

    /// `BgHaka_IdleLockedClosed`: the warning's cooldown down; any push or pull refused; back to
    /// `BgHaka_IdleClosed` at 0. Then the dirt patch.
    fn idle_locked_closed(&mut self, play: &mut PlayState) {
        if self.actor.params != 0 {
            self.actor.params -= 1;
        }
        if self.unk_150(play) != 0.0 {
            self.release(play);
        }
        if self.actor.params == 0 {
            self.action = Action::IdleClosed;
        }
        self.check_player_on_dirt_patch(play);
    }
}

impl ActorImpl for BgHaka {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgHaka_Update`: the action; the new position for `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::IdleClosed => self.idle_closed(play),
            Action::Pull => self.pull(play),
            Action::IdleOpened => self.idle_opened(play),
            Action::IdleLockedClosed => self.idle_locked_closed(play),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `BgHaka_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// `minVelocityY` blended, for the earth's offset.
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.values = vec![self.actor.min_velocity_y];
        rs
    }

    /// `BgHaka_Draw`: `gGravestoneStoneDL` opaque at the actor's matrix, then `gGravestoneEarthDL`
    /// translucent, moved `minVelocityY × 10` along its z in model space (`Matrix_Translate(..,
    /// MTXMODE_APPLY)`: 1 a unit pulled at scale 0.1).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let m = actor_draw_matrix(rs);
        out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, STONE_DL)), m));
        let pulled = rs.values.first().copied().unwrap_or(0.0);
        let m = m * Mat4::from_translation(Vec3::new(0.0, 0.0, pulled * 10.0));
        out.xlu.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, EARTH_DL)), m));
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
