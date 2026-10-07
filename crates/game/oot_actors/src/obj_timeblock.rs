//! `Obj_Timeblock` (`ovl_Obj_Timeblock/z_obj_timeblock.c`): the Song of Time's block, a DynaPoly
//! actor of `object_timeblock` (`gSongOfTimeBlockCol`, `DynaPolyActor_Init(0)`) drawn with
//! `gSongOfTimeBlockDL` in one of eight colours (`home.rot.z & 7`), shown or hidden with its
//! collision.
//!
//! Params: bits 0..5 a switch flag; bit 6 set: the block's own state is params bit 15
//! (`unk_177` 0, the song flips the bit), clear: the switch flag xor bit 15 (`unk_177` 1 for flags
//! 0x38 and up, 2 below them, xor child Link too); bit 8 the size (`sSizeOptions`: 1 or 0.6, its
//! focus 60 or 40 up, the song's `Demo_Effect` 0x18 or 0x19); bit 10 the "alt" behaviour (shown
//! by the switch flag xor bit 15, the song toggles the flag, and the flag set or cleared elsewhere
//! shows or hides it with the effect); bits 11..13 how far Link can be to play the song to it
//! (`sRanges`: 60 to 300); bit 15 shown (`unk_177` 0) or inverted.
//!
//! The song: Link within range (and not on top of it while it's shown) gets the ocarina's prompt
//! (`PLAYER_STATE2_23`); with the ocarina out (`PLAYER_STATE2_24`) the block starts the free play
//! (`Message_StartOcarina`) and watches `msgCtx.lastPlayedSong`: the staff's 254 then the Song of
//! Time (10) starts 110 frames, after which the block toggles: `Demo_Effect`, the attention camera
//! (`OnePointCutscene_Attention`), 12 frames to the switch, and `NA_SE_SY_TRE_BOX_APPEAR` 110
//! frames after the toggle (`demoEffectTimer` 50 of 160).
//!
//! The Master Quest Deku Tree's: room 2's four and room 7's five, 0x39FF (flag 0x3F, `unk_177` 0,
//! small, range 300, hidden: no collision), and room 5's one, 0xB9FF (shown), standing on the
//! purple rupee's chest. The ocarina isn't ported (`Message_StartOcarina` logs and the block
//! still waits for the song, as the C does) and `Demo_Effect` stays a placeholder; the tests set
//! `msgCtx.lastPlayedSong` and Player's `PLAYER_STATE2_24` as the ocarina would. The whole overlay
//! is ported; the cull zone isn't, for any actor.

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_INTERACT_PLAYER_ABOVE};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_LOCK_ON_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_OBJ_TIMEBLOCK` (`actor_table.h`: 0x01D1).
pub const ACTOR_OBJ_TIMEBLOCK: i16 = 0x01D1;
pub const OBJECT: &str = "object_timeblock";
/// `OBJECT_TIMEBLOCK` (`object_table.h`: 0x0190).
pub const OBJECT_TIMEBLOCK: i16 = 0x0190;
const COLLISION: &str = "gSongOfTimeBlockCol";
const DL: &str = "gSongOfTimeBlockDL";

/// `ACTOR_DEMO_EFFECT` (`actor_table.h`: 0x008B): the song's sparkles, not ported (a placeholder).
pub const ACTOR_DEMO_EFFECT: i16 = 0x008B;
/// `PLAYER_STATE2_23` (`player.h`: 1 << 23): an ocarina spot nearby (the prompt).
pub const PLAYER_STATE2_23: u32 = 1 << 23;
/// `PLAYER_STATE2_24` (`player.h`: 1 << 24): the ocarina out.
pub const PLAYER_STATE2_24: u32 = 1 << 24;
/// `OCARINA_MODE_04` (`ocarina.h`: 4).
pub const OCARINA_MODE_04: u16 = 4;
/// `OCARINA_SONG_TIME` (`ocarina.h`: 10).
pub const OCARINA_SONG_TIME: u16 = 10;
/// `OCARINA_ACTION_FREE_PLAY` (`ocarina.h`: 1).
pub const OCARINA_ACTION_FREE_PLAY: u16 = 1;
/// `ATTENTION_RANGE_2` (`z64actor.h`: 2).
const ATTENTION_RANGE_2: u8 = 2;

/// `Obj_Timeblock_Profile`: `ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_UPDATE_DURING_OCARINA | ACTOR_FLAG_LOCK_ON_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_OBJ_TIMEBLOCK,
    name: "Obj_Timeblock",
    category: ACTORCAT_ITEMACTION,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA | ACTOR_FLAG_LOCK_ON_DISABLED,
    object: OBJECT,
};

/// `ObjTimeblockSizeOptions`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizeOptions {
    pub scale: f32,
    pub height: f32,
    pub demo_effect_params: i16,
}

/// `sSizeOptions`.
pub const SIZE_OPTIONS: [SizeOptions; 2] = [SizeOptions { scale: 1.0, height: 60.0, demo_effect_params: 0x0018 }, SizeOptions { scale: 0.60, height: 40.0, demo_effect_params: 0x0019 }];

/// `sRanges`.
pub const RANGES: [f32; 8] = [60.0, 100.0, 140.0, 180.0, 220.0, 260.0, 300.0, 300.0];

/// `sPrimColors`.
pub const PRIM_COLORS: [[u8; 3]; 8] = [[100, 120, 140], [80, 140, 200], [100, 150, 200], [100, 200, 240], [80, 110, 140], [70, 160, 225], [80, 100, 130], [100, 110, 190]];

/// The segment the bake takes `gDPSetPrimColor` from (the draw's own command, before the list).
const SEG_PRIM: u8 = 0x0B;

fn bake_name() -> String {
    format!("Obj_Timeblock/{DL}")
}

/// `ObjTimeblock_Draw`'s `gSongOfTimeBlockDL` after `Gfx_SetupDL_25Opa` and a dynamic
/// `gDPSetPrimColor` (`sPrimColors[home.rot.z & 7]`, alpha 255).
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: bake_name(),
        object: OBJECT.into(),
        segments: vec![(SEG_PRIM, BakeSegment::DynamicColor { env: false, prim: true })],
        prelude: vec![SEG_PRIM],
        body: BakeBody::DLists(vec![(OBJECT.into(), DL.into())]),
    }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `ObjTimeblock_DoNothing`.
    DoNothing,
    /// `ObjTimeblock_Normal`.
    Normal,
    /// `ObjTimeblock_AltBehaviorVisible`.
    AltBehaviorVisible,
    /// `ObjTimeblock_AltBehaviourNotVisible`.
    AltBehaviourNotVisible,
}

/// `songObserverFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SongObserver {
    /// `ObjTimeblock_WaitForOcarina`.
    WaitForOcarina,
    /// `ObjTimeblock_WaitForSong`.
    WaitForSong,
}

pub struct ObjTimeblock {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    pub song_observer: SongObserver,
    pub demo_effect_timer: i16,
    pub song_end_timer: i16,
    pub demo_effect_first_part_timer: i16,
    /// `unk_172`: last frame's `msgCtx.lastPlayedSong`.
    pub unk_172: u16,
    /// `unk_174`: the switch flag, as last read.
    pub unk_174: bool,
    /// `unk_175`: params bit 15, as last read.
    pub unk_175: bool,
    /// `unk_176`: the switch flag, as last read by `func_80BA06AC`.
    pub unk_176: bool,
    /// `unk_177`: 0 from params bit 15, 1 or 2 from the switch flag (2: xor child Link).
    pub unk_177: u8,
    pub is_visible: bool,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `PARAMS_GET_U(params, shift, n)`.
fn params_get(params: i16, shift: u32, n: u32) -> u16 {
    (params as u16 >> shift) & ((1 << n) - 1)
}

/// `player->stateFlags2`.
fn player_state2(play: &PlayState) -> u32 {
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.state_flags2())
}

impl ObjTimeblock {
    /// `PARAMS_GET_U(params, 0, 6)`: the switch flag.
    pub fn switch_flag(&self) -> i32 {
        params_get(self.actor.params, 0, 6) as i32
    }

    /// `sSizeOptions[PARAMS_GET_U(params, 8, 1)]`.
    pub fn size_options(&self) -> SizeOptions {
        SIZE_OPTIONS[params_get(self.actor.params, 8, 1) as usize]
    }

    /// `PARAMS_GET_U(params, 15, 1) ? true : false`.
    fn bit_15(&self) -> bool {
        params_get(self.actor.params, 15, 1) != 0
    }

    /// `ObjTimeblock_CalculateIsVisible`: normal blocks: `unk_175` (`unk_177` 0), or the switch
    /// flag xor params bit 15 (1), xor child Link too (2); the alt ones: bit 15 xor the flag.
    pub fn calculate_is_visible(&self, play: &PlayState) -> bool {
        if params_get(self.actor.params, 10, 1) == 0 {
            if self.unk_177 == 0 {
                self.unk_175
            } else {
                let temp = self.bit_15();
                if self.unk_177 == 1 {
                    self.unk_174 ^ temp
                } else {
                    // LINK_AGE_IN_YEARS == YEARS_CHILD.
                    let link_is_child = !play.save.adult;
                    self.unk_174 ^ temp ^ link_is_child
                }
            }
        } else {
            self.bit_15() ^ self.unk_174
        }
    }

    /// `ObjTimeblock_SpawnDemoEffect`: `Demo_Effect` at the block with the size's params (a
    /// placeholder: not ported).
    fn spawn_demo_effect(&self, play: &mut PlayState) {
        let _ = play.actor_spawn(ACTOR_DEMO_EFFECT, self.actor.world_pos, [0, 0, 0], self.size_options().demo_effect_params);
    }

    /// `ObjTimeblock_ToggleSwitchFlag`.
    fn toggle_switch_flag(play: &mut PlayState, flag: i32) {
        if play.flags.get_switch(flag) {
            play.flags.unset_switch(flag);
        } else {
            play.flags.set_switch(flag);
        }
    }

    /// `OnePointCutscene_Attention(play, &this->dyna.actor)`.
    fn attention(&self, play: &mut PlayState) {
        if let Some(h) = play.cur_actor {
            let me = play.cam_actor_of(h, &self.actor);
            play.onepoint_attention(me);
        }
        log::debug!("Time Block Attention Camera (frame counter  {})", play.gameplay_frames);
    }

    /// `ObjTimeblock_Init`: `DynaPolyActor_Init(0)`, `rot.z` zeroed (`home.rot.z`, the colour,
    /// kept), `gSongOfTimeBlockCol`, `ATTENTION_RANGE_2` (`sInitChain`), the size's scale and focus,
    /// `unk_177`, `ObjTimeblock_WaitForOcarina`, the flag and bit 15 read and the visibility
    /// from them; normal, or alt shown or hidden.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.world_rot.z = 0;
        actor.shape_rot.z = 0;
        let bg = match crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            Some(h) => play.col.dyna.set_bg_actor(h, source(&actor), 0),
            None => BG_ACTOR_MAX,
        };
        // sInitChain: attentionRangeType ATTENTION_RANGE_2; cullingVolumeDistance 1800,
        // cullingVolumeScale 300, cullingVolumeDownward 1500 (the cull zone isn't ported).
        actor.target_mode = ATTENTION_RANGE_2;
        let mut t = ObjTimeblock {
            actor,
            bg,
            action: Action::Normal,
            song_observer: SongObserver::WaitForOcarina,
            demo_effect_timer: 0,
            song_end_timer: 0,
            demo_effect_first_part_timer: 0,
            unk_172: 0,
            unk_174: false,
            unk_175: false,
            unk_176: false,
            unk_177: 0,
            is_visible: false,
        };
        let size = t.size_options();
        t.actor.scale = Vec3::splat(size.scale);
        t.unk_177 = if params_get(t.actor.params, 6, 1) != 0 {
            0
        } else if params_get(t.actor.params, 0, 6) < 0x38 {
            2
        } else {
            1
        };
        t.song_observer = SongObserver::WaitForOcarina;
        t.actor.set_focus(size.height);
        t.unk_174 = play.flags.get_switch(t.switch_flag());
        t.unk_175 = t.bit_15();
        t.is_visible = t.calculate_is_visible(play);
        t.action = if params_get(t.actor.params, 10, 1) == 0 {
            Action::Normal
        } else if t.is_visible {
            Action::AltBehaviorVisible
        } else {
            Action::AltBehaviourNotVisible
        };
        log::debug!(
            "Time Block (<arg> {:04x}H <type> save:{} color:{} range:{} move:{})",
            t.actor.params as u16,
            t.unk_177,
            t.actor.home_rot.z & 7,
            params_get(t.actor.params, 11, 3),
            params_get(t.actor.params, 10, 1)
        );
        play.col.dyna.set_source(t.bg, source(&t.actor));
        Box::new(t)
    }

    /// `ObjTimeblock_PlayerIsInRange`: not while Link's on top of it shown
    /// (`DynaPolyActor_IsPlayerAbove`); within its range (`sRanges[PARAMS_GET_U(params, 11, 3)]`)
    /// and outside its square (its half-size, `scale.x × 50 + 6`, in its frame).
    pub fn player_is_in_range(&self, play: &PlayState) -> bool {
        if self.is_visible && play.col.dyna.interact_flag(self.bg, DYNA_INTERACT_PLAYER_ABOVE) {
            return false;
        }
        if self.actor.xz_dist_to_player <= RANGES[params_get(self.actor.params, 11, 3) as usize] {
            let Some(player_pos) = play.player.and_then(|h| play.actors.actor(h)).map(|p| p.world_pos) else { return false };
            let player_relative_pos = self.actor.world_to_actor_coords(player_pos);
            let block_size = self.actor.scale.x * 50.0 + 6.0;
            // Return true if player's xz position is not inside the block.
            if block_size < player_relative_pos.x.abs() || block_size < player_relative_pos.z.abs() {
                return true;
            }
        }
        false
    }

    /// `ObjTimeblock_WaitForOcarina`: Link in range with the ocarina out starts the free play
    /// (`Message_StartOcarina(OCARINA_ACTION_FREE_PLAY)`: not ported, logged) and the block waits
    /// for the song; in range without it, he gets the ocarina's prompt (`PLAYER_STATE2_23`).
    fn wait_for_ocarina(&mut self, play: &mut PlayState) -> bool {
        if self.player_is_in_range(play) {
            if player_state2(play) & PLAYER_STATE2_24 != 0 {
                log::info!("Obj_Timeblock: Message_StartOcarina(OCARINA_ACTION_FREE_PLAY {OCARINA_ACTION_FREE_PLAY}) isn't ported");
                self.song_observer = SongObserver::WaitForSong;
            } else if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
                p.change_state_flags2(PLAYER_STATE2_23, 0);
            }
        }
        false
    }

    /// `ObjTimeblock_WaitForSong`: the ocarina put away (`OCARINA_MODE_04`) goes back to waiting
    /// for it; the Song of Time played (`lastPlayedSong` 10) right after the staff (`unk_172`, last
    /// frame's, 254) starts 110 frames, each later frame of it counts one off, and true at 0.
    fn wait_for_song(&mut self, play: &mut PlayState) -> bool {
        if play.msg_ctx.ocarina_mode == OCARINA_MODE_04 {
            self.song_observer = SongObserver::WaitForOcarina;
        }
        if play.msg_ctx.last_played_song == OCARINA_SONG_TIME {
            if self.unk_172 == 254 {
                self.song_end_timer = 110;
            } else {
                self.song_end_timer = self.song_end_timer.wrapping_sub(1);
                if self.song_end_timer == 0 {
                    return true;
                }
            }
        }
        false
    }

    /// `this->songObserverFunc(this, play)`.
    fn song_observer_func(&mut self, play: &mut PlayState) -> bool {
        match self.song_observer {
            SongObserver::WaitForOcarina => self.wait_for_ocarina(play),
            SongObserver::WaitForSong => self.wait_for_song(play),
        }
    }

    /// `ObjTimeblock_Normal`: the song heard with no effect running: `Demo_Effect`, 160 frames,
    /// the attention camera, 12 frames to the switch, and params bit 15 flipped (`unk_177` 0) or
    /// the switch flag toggled. `unk_172` this frame's song; at the 12 frames' end the new bit 15
    /// or flag read; the visibility (a flag block, `unk_177` 1, changing it does nothing more:
    /// `ObjTimeblock_DoNothing`); the chime at `demoEffectTimer` 50.
    fn normal(&mut self, play: &mut PlayState) {
        if self.song_observer_func(play) && self.demo_effect_timer <= 0 {
            self.spawn_demo_effect(play);
            self.demo_effect_timer = 160;
            // Possibly points the camera to this actor.
            self.attention(play);
            self.demo_effect_first_part_timer = 12;
            if self.unk_177 == 0 {
                self.actor.params ^= 0x8000u16 as i16;
            } else {
                Self::toggle_switch_flag(play, self.switch_flag());
            }
        }
        self.unk_172 = play.msg_ctx.last_played_song;
        if self.demo_effect_first_part_timer > 0 {
            self.demo_effect_first_part_timer -= 1;
            if self.demo_effect_first_part_timer == 0 {
                if self.unk_177 == 0 {
                    self.unk_175 = self.bit_15();
                } else {
                    self.unk_174 = play.flags.get_switch(self.switch_flag());
                }
            }
        }
        let new_is_visible = self.calculate_is_visible(play);
        if self.unk_177 == 1 && new_is_visible != self.is_visible {
            // ObjTimeblock_SetupDoNothing.
            self.action = Action::DoNothing;
        }
        self.is_visible = new_is_visible;
        if self.demo_effect_timer == 50 {
            play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
        }
    }

    /// `func_80BA06AC`: the alt blocks' common part: `unk_172`, the flag read at the 12 frames'
    /// end, the visibility, `unk_176` the flag now.
    fn func_80ba06ac(&mut self, play: &mut PlayState) {
        let switch_flag = self.switch_flag();
        self.unk_172 = play.msg_ctx.last_played_song;
        if self.demo_effect_first_part_timer > 0 {
            self.demo_effect_first_part_timer -= 1;
            if self.demo_effect_first_part_timer == 0 {
                self.unk_174 = play.flags.get_switch(switch_flag);
            }
        }
        self.is_visible = self.calculate_is_visible(play);
        self.unk_176 = play.flags.get_switch(switch_flag);
    }

    /// `ObjTimeblock_AltBehaviorVisible`: the song heard with no effect running toggles the flag
    /// (12 frames, `Demo_Effect`, 160, the attention camera); the common part; the chime at 50;
    /// hidden with the effect over, `ObjTimeblock_AltBehaviourNotVisible`.
    fn alt_behavior_visible(&mut self, play: &mut PlayState) {
        if self.song_observer_func(play) && self.demo_effect_timer <= 0 {
            self.demo_effect_first_part_timer = 12;
            self.spawn_demo_effect(play);
            self.demo_effect_timer = 160;
            self.attention(play);
            Self::toggle_switch_flag(play, self.switch_flag());
        }
        self.func_80ba06ac(play);
        if self.demo_effect_timer == 50 {
            play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
        }
        if !self.is_visible && self.demo_effect_timer <= 0 {
            self.action = Action::AltBehaviourNotVisible;
        }
    }

    /// `ObjTimeblock_AltBehaviourNotVisible`: the flag changed since last frame, to differ from
    /// bit 15 (the block to show): `Demo_Effect` and 160 (if none running) and 12 frames; the
    /// common part; shown with the effect over, `ObjTimeblock_AltBehaviorVisible`. (No song here:
    /// the observer isn't called.)
    fn alt_behaviour_not_visible(&mut self, play: &mut PlayState) {
        let switch_flag = self.switch_flag();
        let switch_flag_is_set = play.flags.get_switch(switch_flag);
        // `this->unk_176 ^ switchFlagIsSet && switchFlagIsSet ^ bit15`: ^ binds tighter than &&.
        if (self.unk_176 ^ switch_flag_is_set) && (switch_flag_is_set ^ self.bit_15()) {
            if self.demo_effect_timer <= 0 {
                self.spawn_demo_effect(play);
                self.demo_effect_timer = 160;
            }
            self.demo_effect_first_part_timer = 12;
        }
        self.func_80ba06ac(play);
        if self.is_visible && self.demo_effect_timer <= 0 {
            self.action = Action::AltBehaviorVisible;
        }
    }
}

impl ActorImpl for ObjTimeblock {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjTimeblock_Update`: the action, `demoEffectTimer` down, the collision on while shown
    /// (`DynaPoly_EnableCollision`) and off while hidden (`DynaPoly_DisableCollision`).
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::DoNothing => {}
            Action::Normal => self.normal(play),
            Action::AltBehaviorVisible => self.alt_behavior_visible(play),
            Action::AltBehaviourNotVisible => self.alt_behaviour_not_visible(play),
        }
        if self.demo_effect_timer > 0 {
            self.demo_effect_timer -= 1;
        }
        play.col.dyna.set_collision_disabled(self.bg, !self.is_visible);
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `ObjTimeblock_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    /// `DynaPolyActor_IsPlayerAbove` reads the interact flags `Actor_UpdateAll` clears after it.
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// Shown, and the colour (`home.rot.z & 7`).
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.is_visible as u32, (self.actor.home_rot.z & 7) as u32];
        rs
    }

    /// `ObjTimeblock_Draw`: while shown, `Gfx_SetupDL_25Opa`, `gDPSetPrimColor(0, 0,
    /// sPrimColors[home.rot.z & 7], 255)`, `gSongOfTimeBlockDL`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.first() != Some(&1) {
            return;
        }
        let [r, g, b] = PRIM_COLORS[rs.switches.get(1).copied().unwrap_or(0) as usize & 7];
        let mut sv = SegmentValues::default();
        sv.prim[SEG_PRIM as usize] = Some([r, g, b, 255]);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name())), transform: actor_draw_matrix(rs), bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
