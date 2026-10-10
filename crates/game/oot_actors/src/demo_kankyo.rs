//! `Demo_Kankyo` (`ovl_Demo_Kankyo/z_demo_kankyo.c`): the cutscenes' surroundings. Its params
//! pick one of 18 types (`DemoKankyoType`):
//! - the blue rain (`BLUE_RAIN`, `_2`): the creation's in the cutscene map, the Temple of Time's,
//!   Hyrule Field's (which rises), thirty streaks of five drops kept in front of the eye;
//! - Din's rocks (`ROCK_1` to `_5`) in Gerudo Valley, moved by their cues, tumbling;
//! - the clouds (`CLOUDS`), thirty dust puffs circling the eye;
//! - the Door of Time (`DOOR_OF_TIME`), sliding open over 102 frames with cutscene flag 2, and the
//!   Temple of Time's light plane (`LIGHT_PLANE`);
//! - the warp songs' sparkles (`WARP_OUT`, `WARP_IN`), which start the warp's cutscene, and the
//!   sparkles (`SPARKLES`), climbing a camera spline around Link or the actor;
//! - five types removed from the game (`8` to `C`, `object_gi_melody`'s), which do nothing.
//!
//! Ported whole (GAME-06 milestone 1b, ADR 0054). The rain and the rocks hide the current room
//! (`roomCtx.curRoom.segment = NULL`: `Room::loaded` cleared, so it isn't drawn). The draws are
//! bakes (ADR 0006); what the draws change (the rain's and the sparkles' state, their `Rand`
//! calls, the warp's end) runs once per game frame in `draw_update`, which leaves the matrices
//! for `draw`; the warp-in sparkles' sound is played in `draw_sfx`.
//!
//! Not ported: `player->actor.draw = NULL` at the warp-out's white-out (Player has no draw to
//! take away: logged), `Interface_SetSubTimerToFinalSecond` (no sub-timer runs in the port: a
//! no-op). The Door of Time's child is `Door_Toki`, a placeholder until the Temple of Time
//! (Phase 8).

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, smooth_step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_ITEMACTION, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::*;
use oot_game::camera::{func_800bb2b4, sph_geo_to_vec3, vec3_to_sph_geo};
use oot_game::cutscene::{CS_CAM_STOP, CS_STATE_IDLE, CsCmdActorCue, CutsceneCameraPoint};
use oot_game::gbi::setup_dl;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::save::RESPAWN_MODE_RETURN;
use oot_game::scene_table::gfx_tex_scroll;
use oot_game::transition::{TRANS_TRIGGER_START, TRANS_TYPE_FADE_WHITE};

use crate::bg_treemouth::actor_is_facing_and_near_player;

/// `ACTOR_DEMO_KANKYO` (`actor_table.h`: 0x008C).
pub const ACTOR_DEMO_KANKYO: i16 = 0x008C;
/// `ACTOR_DOOR_TOKI` (0x0070).
const ACTOR_DOOR_TOKI: i16 = 0x0070;

/// `Demo_Kankyo_Profile`: `ACTORCAT_BG`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_DRAW_CULLING_DISABLED`, `OBJECT_GAMEPLAY_KEEP` (each type waits for its own).
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_DEMO_KANKYO, name: "Demo_Kankyo", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED, object: "gameplay_keep" };

/// `DemoKankyoType`.
pub const DEMOKANKYO_BLUE_RAIN: i16 = 0x00;
pub const DEMOKANKYO_BLUE_RAIN_2: i16 = 0x01;
pub const DEMOKANKYO_ROCK_1: i16 = 0x02;
pub const DEMOKANKYO_ROCK_2: i16 = 0x03;
pub const DEMOKANKYO_ROCK_3: i16 = 0x04;
pub const DEMOKANKYO_ROCK_4: i16 = 0x05;
pub const DEMOKANKYO_ROCK_5: i16 = 0x06;
pub const DEMOKANKYO_CLOUDS: i16 = 0x07;
pub const DEMOKANKYO_DOOR_OF_TIME: i16 = 0x0D;
pub const DEMOKANKYO_LIGHT_PLANE: i16 = 0x0E;
pub const DEMOKANKYO_WARP_OUT: i16 = 0x0F;
pub const DEMOKANKYO_WARP_IN: i16 = 0x10;
pub const DEMOKANKYO_SPARKLES: i16 = 0x11;

/// `object_table.h`.
const OBJECT_GAMEPLAY_KEEP: i16 = 0x0001;
const OBJECT_TOKI_OBJECTS: i16 = 0x005E;
const OBJECT_EFC_STAR_FIELD: i16 = 0x0092;
const OBJECT_GI_MELODY: i16 = 0x00B6;

/// `sObjectIds`: each type's object.
const S_OBJECT_IDS: [i16; 18] = [
    OBJECT_EFC_STAR_FIELD,
    OBJECT_EFC_STAR_FIELD,
    OBJECT_EFC_STAR_FIELD,
    OBJECT_EFC_STAR_FIELD,
    OBJECT_EFC_STAR_FIELD,
    OBJECT_EFC_STAR_FIELD,
    OBJECT_EFC_STAR_FIELD,
    OBJECT_GAMEPLAY_KEEP,
    OBJECT_GI_MELODY,
    OBJECT_GI_MELODY,
    OBJECT_GI_MELODY,
    OBJECT_GI_MELODY,
    OBJECT_GI_MELODY,
    OBJECT_TOKI_OBJECTS,
    OBJECT_TOKI_OBJECTS,
    OBJECT_GAMEPLAY_KEEP,
    OBJECT_GAMEPLAY_KEEP,
    OBJECT_GAMEPLAY_KEEP,
];

/// `sWarpSparkleEnvColors`: minuet, bolero, serenade, requiem, nocturne, prelude.
pub const S_WARP_SPARKLE_ENV_COLORS: [[u8; 3]; 6] = [[0, 200, 0], [255, 50, 0], [0, 150, 255], [255, 150, 0], [200, 50, 255], [200, 255, 0]];
/// `sSparkleEnvColors` (only the fourth is used).
const S_SPARKLE_ENV_COLORS: [[u8; 3]; 6] = [[0, 200, 0], [255, 50, 0], [0, 150, 255], [255, 150, 0], [0, 255, 255], [200, 255, 0]];

/// `CS_CAM_CONTINUE` (`cutscene.h`).
const CS_CAM_CONTINUE: i8 = 0;

const fn pt(continue_flag: i8, next_point_frame: u16, pos: [i16; 3]) -> CutsceneCameraPoint {
    CutsceneCameraPoint { continue_flag, camera_roll: 0, next_point_frame, view_angle: 45.0, pos }
}
const C: i8 = CS_CAM_CONTINUE;
const S: i8 = CS_CAM_STOP;

/// `sWarpOutCameraPoints`.
pub const S_WARP_OUT_CAMERA_POINTS: [CutsceneCameraPoint; 14] = [
    pt(C, 8, [0x0000, 0x0000, -0x1B]),
    pt(C, 8, [0x0000, 0x0000, -0x1B]),
    pt(C, 8, [-0x1A, 0x0000, 0x0000]),
    pt(C, 8, [0x0000, 0x0017, 0x0024]),
    pt(C, 8, [0x001C, 0x0032, -0x01]),
    pt(C, 8, [0x0001, 0x0018, -0x27]),
    pt(C, 8, [-0x1A, -0x06, 0x0003]),
    pt(C, 8, [0x0000, 0x0025, 0x0037]),
    pt(C, 8, [0x004F, 0x0066, 0x0029]),
    pt(C, 8, [0x00A6, 0x00AD, 0x0006]),
    pt(C, 5, [0x010D, 0x015A, -0xB4]),
    pt(C, 5, [0x019F, 0x0245, -0x1CB]),
    pt(S, 5, [0x01CE, 0x036F, -0x33E]),
    pt(S, 5, [0x01CE, 0x036F, -0x33E]),
];

/// `sWarpInCameraPoints`.
pub const S_WARP_IN_CAMERA_POINTS: [CutsceneCameraPoint; 14] = [
    pt(C, 5, [0x019F, 0x0245, -0x1CB]),
    pt(C, 5, [0x010D, 0x015A, -0xB4]),
    pt(C, 8, [0x00A6, 0x00AD, 0x0006]),
    pt(C, 8, [0x004F, 0x0066, 0x0029]),
    pt(C, 8, [0x0000, 0x0025, 0x0037]),
    pt(C, 8, [-0x1A, -0x06, 0x0003]),
    pt(C, 8, [0x0001, 0x0018, -0x27]),
    pt(C, 8, [0x001C, 0x0032, -0x01]),
    pt(C, 8, [0x0000, 0x0017, 0x0024]),
    pt(C, 8, [-0x1A, 0x0000, 0x0000]),
    pt(C, 8, [0x0000, 0x0000, -0x1B]),
    pt(C, 8, [0x0000, 0x0000, -0x1B]),
    pt(S, 5, [0x01CE, 0x036F, -0x33E]),
    pt(S, 5, [0x01CE, 0x036F, -0x33E]),
];

/// `sSparklesCameraPoints`: a spiral up 0x30 units.
pub const S_SPARKLES_CAMERA_POINTS: [CutsceneCameraPoint; 54] = [
    pt(C, 2, [-0x09, 0x0000, -0x30]),
    pt(C, 2, [-0x09, 0x0000, -0x30]),
    pt(C, 2, [-0x09, 0x0000, -0x30]),
    pt(C, 2, [-0x09, 0x0000, -0x30]),
    pt(C, 2, [-0x29, 0x0000, -0x17]),
    pt(C, 2, [-0x2D, 0x0000, 0x000A]),
    pt(C, 2, [-0x18, 0x0001, 0x0027]),
    pt(C, 2, [0x0015, 0x0000, 0x002B]),
    pt(C, 2, [0x002F, 0x0005, 0x000E]),
    pt(C, 2, [0x0031, 0x0005, -0x0B]),
    pt(C, 2, [0x0020, 0x0005, -0x26]),
    pt(C, 2, [-0x0B, 0x0005, -0x2F]),
    pt(C, 2, [-0x29, 0x0006, -0x16]),
    pt(C, 2, [-0x2B, 0x0009, 0x000D]),
    pt(C, 2, [-0x17, 0x0009, 0x0027]),
    pt(C, 2, [0x0014, 0x000B, 0x0029]),
    pt(C, 2, [0x002D, 0x000B, 0x000F]),
    pt(C, 2, [0x002E, 0x000B, -0x10]),
    pt(C, 2, [0x001E, 0x000B, -0x26]),
    pt(C, 2, [-0x06, 0x000E, -0x2D]),
    pt(C, 2, [-0x26, 0x000E, -0x15]),
    pt(C, 2, [-0x29, 0x0010, 0x0008]),
    pt(C, 2, [-0x17, 0x0010, 0x0024]),
    pt(C, 2, [0x0011, 0x0010, 0x0028]),
    pt(C, 2, [0x002C, 0x0010, 0x000D]),
    pt(C, 2, [0x002C, 0x0012, -0x0B]),
    pt(C, 2, [0x001F, 0x0011, -0x22]),
    pt(C, 2, [-0x05, 0x0014, -0x2B]),
    pt(C, 2, [-0x23, 0x0014, -0x14]),
    pt(C, 2, [-0x26, 0x0017, 0x0008]),
    pt(C, 2, [-0x18, 0x0014, 0x001F]),
    pt(C, 2, [0x000C, 0x0018, 0x0026]),
    pt(C, 2, [0x0027, 0x0018, 0x000D]),
    pt(C, 2, [0x0027, 0x001B, -0x0A]),
    pt(C, 2, [0x001C, 0x001A, -0x1E]),
    pt(C, 2, [-0x06, 0x000E, -0x2C]),
    pt(C, 2, [-0x27, 0x001B, -0x11]),
    pt(C, 2, [-0x29, 0x001B, 0x000A]),
    pt(C, 2, [-0x1A, 0x001B, 0x0022]),
    pt(C, 2, [0x000F, 0x001F, 0x002C]),
    pt(C, 2, [0x0032, 0x0020, 0x0009]),
    pt(C, 2, [0x0030, 0x0021, -0x10]),
    pt(C, 2, [0x001C, 0x0025, -0x27]),
    pt(C, 2, [-0x06, 0x0028, -0x2C]),
    pt(C, 2, [-0x28, 0x002B, -0x0B]),
    pt(C, 2, [-0x29, 0x002B, 0x0006]),
    pt(C, 2, [-0x21, 0x002B, 0x0019]),
    pt(C, 2, [0x000E, 0x002E, 0x002C]),
    pt(C, 2, [0x0032, 0x002E, 0x0003]),
    pt(C, 2, [0x002A, 0x0030, -0x19]),
    pt(C, 2, [-0x0A, 0x002B, -0x2C]),
    pt(C, 2, [-0x0A, 0x002B, -0x2C]),
    pt(S, 2, [-0x0A, 0x002B, -0x2C]),
    pt(S, 2, [-0x0A, 0x002B, -0x2C]),
];

/// `scene_table.h`.
const SCENE_TEMPLE_OF_TIME: u16 = 0x43;
const SCENE_CUTSCENE_MAP: u16 = 0x47;
const SCENE_HYRULE_FIELD: u16 = 0x51;

/// `entrance_table.h`.
const ENTR_CUTSCENE_MAP_0: u16 = 0x0A0;
const ENTR_HYRULE_FIELD_0: u16 = 0x0CD;
const ENTR_GRAVEYARD_0: u16 = 0x0E4;
const ENTR_SACRED_FOREST_MEADOW_0: u16 = 0x0FC;
const ENTR_LAKE_HYLIA_0: u16 = 0x102;
const ENTR_DESERT_COLOSSUS_0: u16 = 0x123;
const ENTR_DEATH_MOUNTAIN_CRATER_0: u16 = 0x147;
const ENTR_TEMPLE_OF_TIME_0: u16 = 0x053;

/// `save.h`.
const EVENTCHKINF_OPENED_DOOR_OF_TIME: u16 = 0x4B;
const EVENTCHKINF_A7: u16 = 0xA7;
const EVENTCHKINF_B1: u16 = 0xB1;
const EVENTCHKINF_B6: u16 = 0xB6;
const EVENTCHKINF_B8: u16 = 0xB8;
const EVENTCHKINF_B9: u16 = 0xB9;
/// `CS_INDEX_NONE`.
const CS_INDEX_NONE: u16 = 0x0000;

/// `DemoKankyoUnk150`: one of the thirty raindrops, clouds or sparkles (the rocks and the door
/// use the first).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DemoKankyoUnk150 {
    pub unk_0: Vec3,
    pub unk_c: Vec3,
    /// The Door of Time's opening, a cloud's speed, a raindrop's, a sparkle's size.
    pub unk_18: f32,
    /// A sparkle's spline frame.
    pub unk_1c: f32,
    /// A cloud's angle, a sparkle's spline keyframe.
    pub unk_20: i16,
    /// The mode.
    pub unk_22: u8,
    /// A sparkle's size mode.
    pub unk_23: u8,
    /// A sparkle's roll.
    pub unk_24: i16,
}

/// `DemoKankyoActionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    SetupType,
    UpdateClouds,
    UpdateRock,
    UpdateWarpIn,
    UpdateDoorOfTime,
    DoNothing,
    KillDoorOfTimeCollision,
}

/// The overlay's statics: the rain's speed and scale (`D_8098CF80`, `sRainScale`), the warp-in's
/// end frame (`D_8098CF84`), and the sparkle draws' (`sWarpRoll`, `sWarpFoV`, `D_8098CF98`,
/// `sSparklesRoll`, `sSparklesFoV`, `D_8098CFB8`).
#[derive(Debug, Default)]
pub struct Statics {
    pub d_8098cf80: i16,
    pub rain_scale: i16,
    pub d_8098cf84: i16,
    pub warp_roll: f32,
    pub warp_fov: f32,
    pub d_8098cf98: Vec3,
    pub sparkles_roll: f32,
    pub sparkles_fov: f32,
    pub d_8098cfb8: Vec3,
}

pub fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_DEMO_KANKYO)
}

/// One sparkle's draw: its matrix before the billboard, its roll (`unk_24`, in degrees), its env
/// colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SparkleDraw {
    pub mtx: Mat4,
    pub roll: i16,
    pub env: [u8; 3],
}

pub struct DemoKankyo {
    pub actor: Actor,
    /// `requiredObjectSlot`.
    pub required_object_slot: Option<usize>,
    pub sparkle_counter: u8,
    pub warp_timer: u8,
    pub unk_150: [DemoKankyoUnk150; 30],
    pub action: Action,
    /// This frame's `DemoKankyo_Draw` drew (`actor.objectSlot == requiredObjectSlot`, and for the
    /// Temple of Time's rain, its conditions).
    pub drawn: bool,
    /// The rain's 150 matrices and its colours, as `DemoKankyo_DrawRain` left them.
    pub rain: Vec<Mat4>,
    pub rain_colors: ([u8; 4], [u8; 4]),
    /// The sparkles' draws, as `DemoKankyo_DrawWarpSparkles` or `_DrawSparkles` left them.
    pub sparkles: Vec<SparkleDraw>,
    /// The warp-in sparkles' `SFX_PLAY_CENTERED(NA_SE_EV_LINK_WARP_OUT - SFX_FLAG)` calls this
    /// frame (one per sparkle on its spline).
    warp_in_sfx: u32,
}

impl DemoKankyo {
    pub fn params(&self) -> i16 {
        self.actor.params
    }

    /// `DemoKankyo_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = DemoKankyo {
            actor,
            required_object_slot: None,
            sparkle_counter: 0,
            warp_timer: 0,
            unk_150: [DemoKankyoUnk150::default(); 30],
            action: Action::SetupType,
            drawn: false,
            rain: Vec::new(),
            rain_colors: ([0; 4], [0; 4]),
            sparkles: Vec::new(),
            warp_in_sfx: 0,
        };
        let params = this.actor.params;
        let object = usize::try_from(params).ok().and_then(|p| S_OBJECT_IDS.get(p)).copied();
        let slot = object.and_then(|o| play.object_ctx.get_index(o));
        log::debug!("bank_ID = {slot:?}");
        match slot {
            None => log::error!("Demo_Kankyo: type {params:#x}'s object {object:?} isn't loaded (ASSERT in z_demo_kankyo.c)"),
            Some(s) => this.required_object_slot = Some(s),
        }
        match params {
            DEMOKANKYO_BLUE_RAIN | DEMOKANKYO_BLUE_RAIN_2 => match play.scene_id {
                SCENE_CUTSCENE_MAP => {
                    // play->roomCtx.curRoom.segment = NULL.
                    play.room_ctx.cur.loaded = false;
                    let s = statics(play);
                    s.d_8098cf80 = 10;
                    s.rain_scale = 8;
                }
                SCENE_TEMPLE_OF_TIME => {
                    let s = statics(play);
                    s.d_8098cf80 = 14;
                    s.rain_scale = 8;
                }
                SCENE_HYRULE_FIELD => {
                    let s = statics(play);
                    s.d_8098cf80 = 1;
                    s.rain_scale = 5;
                }
                _ => this.actor.kill(),
            },
            DEMOKANKYO_ROCK_1..=DEMOKANKYO_ROCK_5 => {
                play.room_ctx.cur.loaded = false;
                this.actor.scale = Vec3::splat(play.rand.zero_one() * 0.5 + 0.5);
                let x = play.rand.zero_one() * 3.0 + 1.0;
                let y = play.rand.zero_one() * 3.0 + 1.0;
                let z = play.rand.zero_one() * 3.0 + 1.0;
                this.unk_150[0].unk_0 = Vec3::new(x, y, z);
            }
            DEMOKANKYO_CLOUDS => {
                for e in this.unk_150.iter_mut() {
                    e.unk_20 = (play.rand.zero_one() * 65535.0) as i32 as i16;
                    e.unk_18 = play.rand.zero_one() * 100.0 + 60.0;
                }
            }
            DEMOKANKYO_DOOR_OF_TIME => {
                this.actor.scale = Vec3::ONE;
                this.unk_150[0].unk_18 = 0.0;
                if !play.save.get_event_chk_inf(EVENTCHKINF_OPENED_DOOR_OF_TIME) {
                    let pos = this.actor.world_pos;
                    let _ = play.actor_spawn_as_child(&mut this.actor, ACTOR_DOOR_TOKI, pos, [0, 0, 0], 0x0000);
                } else {
                    if let Some(s) = &mut play.scene {
                        s.draw.room_draw_params[1] = 0xFF;
                    }
                    this.actor.kill();
                }
            }
            DEMOKANKYO_LIGHT_PLANE => {
                this.actor.scale = Vec3::ONE;
                this.unk_150[0].unk_18 = 0.0;
            }
            DEMOKANKYO_WARP_OUT | DEMOKANKYO_WARP_IN => {
                // Actor_ChangeCategory(.., ACTORCAT_ITEMACTION).
                this.actor.category = ACTORCAT_ITEMACTION;
                this.actor.flags |= ACTOR_FLAG_UPDATE_DURING_OCARINA;
                this.actor.room = -1;
                this.warp_timer = 35;
                this.sparkle_counter = 0;
                this.actor.scale = Vec3::ONE;
                if params == DEMOKANKYO_WARP_OUT {
                    play.audio.play_sfx_centered(NA_SE_EV_SARIA_MELODY);
                }
            }
            DEMOKANKYO_SPARKLES => {
                this.warp_timer = 35;
                this.sparkle_counter = 0;
                this.actor.scale = Vec3::ONE;
            }
            _ => {}
        }
        for e in this.unk_150.iter_mut() {
            e.unk_22 = 0;
        }
        this.action = Action::SetupType;
        Box::new(this)
    }

    /// `DemoKankyo_SetupType`: once the object is in, the type's action; the warps' start.
    fn setup_type(&mut self, play: &mut PlayState) {
        if self.actor.obj_bank_index != self.required_object_slot {
            return;
        }
        match self.actor.params {
            DEMOKANKYO_ROCK_1..=DEMOKANKYO_ROCK_5 => self.action = Action::UpdateRock,
            DEMOKANKYO_CLOUDS => self.action = Action::UpdateClouds,
            DEMOKANKYO_DOOR_OF_TIME => {
                if play.flags_get_env(2) {
                    self.action = Action::UpdateDoorOfTime;
                }
            }
            DEMOKANKYO_WARP_OUT => {
                // envCtx.screenFillColor's colour white, fillScreen false, then the fade.
                let mut fill = None;
                if self.warp_timer < 21 && self.warp_timer >= 15 {
                    let temp = (self.warp_timer as f32 - 15.0) / 5.0;
                    fill = Some((255.0 - 255.0 * temp) as u8);
                }
                if self.warp_timer < 15 && self.warp_timer >= 4 {
                    let temp = (self.warp_timer as f32 - 4.0) / 10.0;
                    fill = Some((255.0 * temp) as u8);
                }
                play.transition.screen_fill = fill.map(|a| [0xFF, 0xFF, 0xFF, a]);
                if self.warp_timer == 15 {
                    log::warn!("Demo_Kankyo: player->actor.draw = NULL isn't ported");
                }
                if self.warp_timer != 0 {
                    self.warp_timer -= 1;
                }
                if self.warp_timer == 1 {
                    let script = if play.scene_id == SCENE_TEMPLE_OF_TIME {
                        statics(play).d_8098cf84 = 25;
                        if !play.save.adult { "gChildWarpInToTCS" } else { "gAdultWarpInToTCS" }
                    } else {
                        statics(play).d_8098cf84 = 32;
                        if !play.save.adult { "gChildWarpInCS" } else { "gAdultWarpInCS" }
                    };
                    play.cs_ctx.segment = play.cutscene_script(script);
                    if play.cam_is_not_fixed() {
                        play.save.cutscene_trigger = 1;
                    }
                    self.action = Action::DoNothing;
                }
            }
            DEMOKANKYO_WARP_IN => {
                let script = if play.scene_id == SCENE_TEMPLE_OF_TIME {
                    if !play.save.adult { "gChildWarpOutToTCS" } else { "gAdultWarpOutToTCS" }
                } else if !play.save.adult {
                    "gChildWarpOutCS"
                } else {
                    "gAdultWarpOutCS"
                };
                play.cs_ctx.segment = play.cutscene_script(script);
                play.save.cutscene_trigger = 1;
                self.action = Action::UpdateWarpIn;
            }
            _ => {}
        }
    }

    /// `DemoKankyo_UpdateWarpIn` (the sound is before `PAL_1_0` only).
    fn update_warp_in(&mut self) {
        self.action = Action::DoNothing;
    }

    /// `DemoKankyo_SetPosFromCue`.
    fn set_pos_from_cue(&mut self, play: &PlayState, cue: &CsCmdActorCue) {
        let start = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
        let end = Vec3::new(cue.end_pos.x as f32, cue.end_pos.y as f32, cue.end_pos.z as f32);
        let lerp = oot_game::env::lerp_weight(cue.end_frame, cue.start_frame, play.cs_ctx.frames);
        self.actor.world_pos = Vec3::new((end.x - start.x) * lerp + start.x, (end.y - start.y) * lerp + start.y, (end.z - start.z) * lerp + start.z);
    }

    /// `DemoKankyo_UpdateRock`: moved by its cue (channel `params - ROCK_1`), tumbling.
    fn update_rock(&mut self, play: &PlayState) {
        let channel = (self.actor.params - DEMOKANKYO_ROCK_1) as usize;
        if play.cs_ctx.state != CS_STATE_IDLE
            && let Some(cue) = play.cs_ctx.npc_actions.get(channel).copied().flatten()
        {
            self.set_pos_from_cue(play, &cue);
        }
        let e = &mut self.unk_150[0];
        e.unk_c += e.unk_0;
    }

    /// `DemoKankyo_UpdateClouds`.
    fn update_clouds(&mut self) {
        for e in self.unk_150.iter_mut() {
            e.unk_20 = e.unk_20.wrapping_add(e.unk_18 as i32 as i16);
        }
    }

    /// `DemoKankyo_UpdateDoorOfTime`: a unit a frame to 102, then the flag and the collision gone.
    fn update_door_of_time(&mut self, play: &mut PlayState) {
        audio_play_actor_sfx2(play, NA_SE_EV_STONE_STATUE_OPEN - SFX_FLAG);
        self.unk_150[0].unk_18 += 1.0;
        if self.unk_150[0].unk_18 >= 102.0 {
            audio_play_actor_sfx2(play, NA_SE_EV_STONEDOOR_STOP);
            play.save.set_event_chk_inf(EVENTCHKINF_OPENED_DOOR_OF_TIME);
            self.kill_child(play);
            self.action = Action::KillDoorOfTimeCollision;
        }
    }

    /// `Actor_Kill(this->actor.child)`.
    fn kill_child(&self, play: &mut PlayState) {
        if let Some(a) = self.actor.child.and_then(|h| play.actors.actor_mut(h)) {
            a.kill();
        }
    }

    /// `func_80989B54`: a raindrop's new start, by scene, and its speed.
    fn func_80989b54(&mut self, play: &mut PlayState, i: usize) {
        let cf80 = statics(play).d_8098cf80;
        let e = &mut self.unk_150[i];
        match play.scene_id {
            SCENE_CUTSCENE_MAP => {
                e.unk_0.x = (play.rand.zero_one() - 0.5) * 500.0;
                e.unk_0.y = 500.0;
                e.unk_0.z = (play.rand.zero_one() - 0.5) * 500.0;
            }
            SCENE_TEMPLE_OF_TIME => {
                e.unk_c = Vec3::ZERO;
                e.unk_0.x = (play.rand.zero_one() - 0.5) * 180.0;
                e.unk_0.y = 10.0;
                e.unk_0.z = (play.rand.zero_one() - 0.5) * 180.0;
            }
            SCENE_HYRULE_FIELD => {
                e.unk_0.x = (play.rand.zero_one() - 0.5) * 600.0;
                e.unk_0.y = -500.0;
                e.unk_0.z = (play.rand.zero_one() - 0.5) * 600.0;
            }
            _ => {}
        }
        e.unk_18 = play.rand.zero_one() * (cf80 as f32 * 4.0) + cf80 as f32;
    }

    /// `DemoKankyo_DrawRain`'s state: each streak kept 350 ahead of the eye (80 up), falling (in
    /// the cutscene map; rising elsewhere) until 300 past the eye's line, then restarted; its five
    /// drops' matrices.
    fn draw_rain_update(&mut self, play: &mut PlayState) {
        let rain_scale = statics(play).rain_scale;
        let entr = play.save.entrance_index;
        self.rain.clear();
        self.rain_colors = if entr == ENTR_HYRULE_FIELD_0 { ([255, 255, 255, 255], [255, 255, 0, 255]) } else { ([200, 255, 255, 255], [0, 150, 255, 255]) };
        for i in 0..30 {
            let (eye, at) = (play.view.eye, play.view.at);
            let (dx, dy, dz) = (at.x - eye.x, at.y - eye.y, at.z - eye.z);
            let norm = (dx * dx + dy * dy + dz * dz).sqrt();
            if play.scene_id != SCENE_TEMPLE_OF_TIME {
                self.unk_150[i].unk_c = Vec3::new(eye.x + (dx / norm) * 350.0, eye.y + (dy / norm) * 80.0, eye.z + (dz / norm) * 350.0);
            }
            match self.unk_150[i].unk_22 {
                0 => {
                    self.func_80989b54(play, i);
                    self.unk_150[i].unk_0.y = if entr == ENTR_CUTSCENE_MAP_0 { play.rand.zero_one() * 500.0 } else { play.rand.zero_one() * -500.0 };
                    self.unk_150[i].unk_22 += 1;
                }
                1 => {
                    let temp = eye.y + (dy / norm) * 150.0;
                    let e = &mut self.unk_150[i];
                    if entr == ENTR_CUTSCENE_MAP_0 {
                        e.unk_0.y -= e.unk_18;
                    } else {
                        e.unk_0.y += e.unk_18;
                    }
                    let y = e.unk_c.y + e.unk_0.y;
                    let done = if entr == ENTR_CUTSCENE_MAP_0 {
                        y < temp - 300.0
                    } else if entr == ENTR_HYRULE_FIELD_0 {
                        temp + 300.0 < y
                    } else {
                        1000.0 < y
                    };
                    if done {
                        e.unk_22 += 1;
                    }
                }
                2 => {
                    self.func_80989b54(play, i);
                    self.unk_150[i].unk_22 -= 1;
                }
                _ => {}
            }
            let e = self.unk_150[i];
            let mut m = Mat4::from_translation(e.unk_c + e.unk_0);
            if entr != ENTR_CUTSCENE_MAP_0 {
                m *= Mat4::from_rotation_x(std::f32::consts::PI);
            }
            let s = rain_scale as f32 * 0.001;
            m *= Mat4::from_scale(Vec3::splat(s));
            for j in 0..5i32 {
                let (tx, ty, tz) = if play.scene_id != SCENE_TEMPLE_OF_TIME {
                    let tx = if e.unk_0.x >= 0.0 { (-j) as f32 * 1500.0 } else { j as f32 * 1500.0 };
                    let tz = if e.unk_0.z >= 0.0 { (-j) as f32 * 1500.0 } else { j as f32 * 1500.0 };
                    let ty = if j % 2 != 0 { j as f32 * 4000.0 } else { (-j) as f32 * 4000.0 };
                    (tx, ty, tz)
                } else {
                    (0.0, j as f32 * 10.0, 0.0)
                };
                m *= Mat4::from_translation(Vec3::new(tx, ty, tz));
                self.rain.push(m);
            }
        }
    }

    /// The sparkles' shared step (`DemoKankyo_DrawWarpSparkles`, `_DrawSparkles`): the size
    /// growing to 1 and shrinking to 0, then a new offset.
    fn sparkle_size(play: &mut PlayState, e: &mut DemoKankyoUnk150, temp: f32) {
        if e.unk_23 == 0 {
            e.unk_18 = play.rand.zero_one();
            e.unk_23 += 1;
        }
        match e.unk_23 {
            1 => {
                smooth_step_to_f(&mut e.unk_18, 1.0, 0.5, 0.4, 0.2);
                if e.unk_18 >= 1.0 {
                    e.unk_23 = 2;
                }
            }
            2 => {
                smooth_step_to_f(&mut e.unk_18, 0.0, 0.5, 0.3, 0.2);
                if e.unk_18 <= 0.0 {
                    e.unk_0 = sparkle_offset(play, temp);
                    e.unk_18 = 0.0;
                    e.unk_23 = 1;
                }
            }
            _ => {}
        }
    }

    /// `DemoKankyo_DrawWarpSparkles`'s state: two more sparkles a frame to 30, each on the warp
    /// camera spline around Link; the warp-out's first, at its end, leaves
    /// (`Environment_WarpSongLeave`); the warp-in's last, once the cutscene is over, is the
    /// actor's end.
    fn draw_warp_sparkles_update(&mut self, play: &mut PlayState) {
        self.sparkles.clear();
        let adult = play.save.adult;
        if self.sparkle_counter < 30 {
            self.sparkle_counter += 2;
        }
        let player = play.player.and_then(|h| play.actors.actor(h)).map(|a| (a.world_pos, a.world_rot.y)).unwrap_or((Vec3::ZERO, 0));
        let params = self.actor.params;
        let mut i = self.sparkle_counter as i32 - 1;
        while i >= 0 {
            let iu = i as usize;
            let temp = 1.0 - (i as f32 / self.sparkle_counter as f32);
            if self.unk_150[iu].unk_22 == 0 {
                let e = &mut self.unk_150[iu];
                e.unk_20 = 0;
                e.unk_1c = 0.0;
                e.unk_0 = sparkle_offset(play, temp);
                e.unk_23 = 0;
                e.unk_22 += 1;
            }
            match self.unk_150[iu].unk_22 {
                1 => {
                    let points: &[CutsceneCameraPoint] = if params == DEMOKANKYO_WARP_OUT { &S_WARP_OUT_CAMERA_POINTS } else { &S_WARP_IN_CAMERA_POINTS };
                    if params != DEMOKANKYO_WARP_OUT {
                        self.warp_in_sfx += 1;
                    }
                    let e = &mut self.unk_150[iu];
                    let s = statics(play);
                    let (done, r) = func_800bb2b4(points, &mut e.unk_20, &mut e.unk_1c, &mut s.warp_fov);
                    // @bug (game): camPos is left unset when the spline returns early; not
                    // reached (a sparkle leaves state 1 as its spline ends).
                    let mut cam_pos = Vec3::ZERO;
                    if let Some((p, roll)) = r {
                        cam_pos = p;
                        s.warp_roll = roll;
                    }
                    if done {
                        e.unk_22 += 1;
                    }
                    if params == DEMOKANKYO_WARP_OUT {
                        if play.scene_id == SCENE_TEMPLE_OF_TIME && play.cs_ctx.frames == 25 {
                            self.unk_150[iu].unk_22 += 1;
                        }
                    } else if (statics(play).d_8098cf84 as i32) < play.cs_ctx.frames as i32 && params == DEMOKANKYO_WARP_OUT {
                        self.unk_150[iu].unk_22 += 1;
                    }
                    statics(play).d_8098cf98 = vec3f_add_pos_rot(player, cam_pos);
                }
                2 => {
                    if params == DEMOKANKYO_WARP_OUT {
                        if i == 0 {
                            environment_warp_song_leave(play);
                            self.unk_150[iu].unk_22 += 1;
                        }
                    } else if i + 1 == self.sparkle_counter as i32 && play.cs_ctx.state == CS_STATE_IDLE {
                        // Interface_SetSubTimerToFinalSecond: no sub-timer runs.
                        self.actor.kill();
                    }
                }
                _ => {}
            }
            self.unk_150[iu].unk_c = statics(play).d_8098cf98;
            let mut e = self.unk_150[iu];
            Self::sparkle_size(play, &mut e, temp);
            let t = e.unk_c + e.unk_0;
            if e.unk_22 < 2 {
                let t = if !adult { t } else { Vec3::new(t.x, t.y + 15.0, t.z) };
                let s = e.unk_18 * (0.018 * temp);
                let mtx = Mat4::from_translation(t) * Mat4::from_scale(Vec3::splat(s));
                let env = if params == DEMOKANKYO_WARP_OUT {
                    // sWarpSparkleEnvColors[msgCtx.lastPlayedSong]: a warp song's.
                    S_WARP_SPARKLE_ENV_COLORS.get(play.msg_ctx.last_played_song as usize).copied().unwrap_or([255, 255, 255])
                } else {
                    let data = play.save.respawn[RESPAWN_MODE_RETURN].data;
                    S_WARP_SPARKLE_ENV_COLORS.get(data as usize).copied().unwrap_or([255, 255, 255])
                };
                self.sparkles.push(SparkleDraw { mtx, roll: e.unk_24, env });
                e.unk_24 = e.unk_24.wrapping_add(0x190);
            }
            self.unk_150[iu] = e;
            i -= 1;
        }
    }

    /// `DemoKankyo_DrawSparkles`'s state: one more sparkle a frame to 20, on the sparkles' spline
    /// around the actor; the last, once the cutscene is over, is the actor's end.
    fn draw_sparkles_update(&mut self, play: &mut PlayState) {
        self.sparkles.clear();
        if self.sparkle_counter < 20 {
            self.sparkle_counter += 1;
        }
        let world = (self.actor.world_pos, self.actor.world_rot.y);
        let mut i = self.sparkle_counter as i32 - 1;
        while i >= 0 {
            let iu = i as usize;
            let temp = 1.0 - (i as f32 / self.sparkle_counter as f32);
            if self.unk_150[iu].unk_22 == 0 {
                let e = &mut self.unk_150[iu];
                e.unk_20 = 0;
                e.unk_1c = 0.0;
                e.unk_0 = sparkle_offset(play, temp);
                e.unk_23 = 0;
                e.unk_22 += 1;
            }
            match self.unk_150[iu].unk_22 {
                1 => {
                    let e = &mut self.unk_150[iu];
                    let s = statics(play);
                    let (done, r) = func_800bb2b4(&S_SPARKLES_CAMERA_POINTS, &mut e.unk_20, &mut e.unk_1c, &mut s.sparkles_fov);
                    // @bug (game): camPos unset on an early return, as for the warps'.
                    let mut cam_pos = Vec3::ZERO;
                    if let Some((p, roll)) = r {
                        cam_pos = p;
                        s.sparkles_roll = roll;
                    }
                    if done {
                        e.unk_22 += 1;
                    }
                    statics(play).d_8098cfb8 = vec3f_add_pos_rot(world, cam_pos);
                }
                2 => {
                    if i + 1 == self.sparkle_counter as i32 && play.cs_ctx.state == CS_STATE_IDLE {
                        self.actor.kill();
                    }
                }
                _ => {}
            }
            self.unk_150[iu].unk_c = statics(play).d_8098cfb8;
            let mut e = self.unk_150[iu];
            Self::sparkle_size(play, &mut e, temp);
            let t = e.unk_c + e.unk_0;
            if e.unk_22 < 2 {
                let scale = e.unk_18 * (0.02 * temp);
                let mtx = Mat4::from_translation(t) * Mat4::from_scale(Vec3::splat(scale));
                self.sparkles.push(SparkleDraw { mtx, roll: e.unk_24, env: S_SPARKLE_ENV_COLORS[3] });
                e.unk_24 = e.unk_24.wrapping_add(0x190);
            }
            self.unk_150[iu] = e;
            i -= 1;
        }
    }

    /// `DemoKankyo_Draw`'s Temple of Time rain: only with cutscene flag 1, Link facing it within
    /// 300, and from the cutscene's frame 170 (120 as an adult).
    fn temple_rain_shown(&self, play: &PlayState) -> bool {
        if play.scene_id != SCENE_TEMPLE_OF_TIME {
            return true;
        }
        if !play.flags_get_env(1) || !actor_is_facing_and_near_player(&self.actor, 300.0, 0x7530) {
            return false;
        }
        let from = if !play.save.adult { 170 } else { 120 };
        !(play.cs_ctx.frames < from || play.cs_ctx.state == CS_STATE_IDLE)
    }
}

/// A sparkle's offset: `(s16)((Rand_ZeroOne() - 0.5f) * 16.0f * temp)` on each axis.
fn sparkle_offset(play: &mut PlayState, temp: f32) -> Vec3 {
    let x = ((play.rand.zero_one() - 0.5) * 16.0 * temp) as i16 as f32;
    let y = ((play.rand.zero_one() - 0.5) * 16.0 * temp) as i16 as f32;
    let z = ((play.rand.zero_one() - 0.5) * 16.0 * temp) as i16 as f32;
    Vec3::new(x, y, z)
}

/// `DemoKankyo_Vec3fAddPosRot`: `vec` turned by the yaw, from the position
/// (`OLib_Vec3fToVecGeo`, then `DemoKankyo_AddVecGeoToVec3f`).
pub fn vec3f_add_pos_rot((pos, yaw): (Vec3, i16), vec: Vec3) -> Vec3 {
    let mut geo = vec3_to_sph_geo(vec);
    geo.yaw = geo.yaw.wrapping_add(yaw);
    let b = sph_geo_to_vec3(geo);
    Vec3::new(pos.x + b.x, pos.y + b.y, pos.z + b.z)
}

/// `Environment_WarpSongLeave` (`z_kankyo.c`): the weather cleared, out to the warp's return
/// entrance in a white fade, its arrival flag set.
pub fn environment_warp_song_leave(play: &mut PlayState) {
    play.env_statics.weather_mode = oot_game::env::WEATHER_MODE_CLEAR;
    play.save.cutscene_index = CS_INDEX_NONE;
    play.save.respawn_flag = -3;
    play.transition.next_entrance_index = play.save.respawn[RESPAWN_MODE_RETURN].entrance_index;
    play.transition.trigger = TRANS_TRIGGER_START;
    play.transition.ty = TRANS_TYPE_FADE_WHITE;
    play.save.next_transition_type = TRANS_TYPE_FADE_WHITE;
    match play.transition.next_entrance_index {
        ENTR_DEATH_MOUNTAIN_CRATER_0 => play.save.set_event_chk_inf(EVENTCHKINF_B9),
        ENTR_LAKE_HYLIA_0 => play.save.set_event_chk_inf(EVENTCHKINF_B1),
        ENTR_DESERT_COLOSSUS_0 => play.save.set_event_chk_inf(EVENTCHKINF_B8),
        ENTR_GRAVEYARD_0 => play.save.set_event_chk_inf(EVENTCHKINF_B6),
        ENTR_TEMPLE_OF_TIME_0 => play.save.set_event_chk_inf(EVENTCHKINF_A7),
        ENTR_SACRED_FOREST_MEADOW_0 => {}
        _ => {}
    }
}

pub const BAKE_RAIN: &str = "Demo_Kankyo/rain";
pub const BAKE_ROCK: &str = "Demo_Kankyo/rock";
pub const BAKE_CLOUDS: &str = "Demo_Kankyo/clouds";
const BAKE_DOOR_LEFT: &str = "Demo_Kankyo/door_of_time_left";
const BAKE_DOOR_RIGHT: &str = "Demo_Kankyo/door_of_time_right";
const BAKE_LIGHT_PLANE: &str = "Demo_Kankyo/light_plane";

/// The segments: the draw's colours (0x0A), its setup list (0x09), the scroll or the texture
/// (8, as the lists call it).
const SEG_TEX: u8 = 0x08;
const SEG_SETUP: u8 = 0x09;
const SEG_COLOR: u8 = 0x0A;

fn prim(m: u8, l: u8, c: [u8; 4]) -> (u32, u32) {
    (0xFA00_0000 | (m as u32) << 8 | l as u32, u32::from_be_bytes(c))
}
fn env(c: [u8; 4]) -> (u32, u32) {
    (0xFB00_0000, u32::from_be_bytes(c))
}
const END: (u32, u32) = (0xDF00_0000, 0);

fn with_end(mut d: oot_game::gbi::Dl) -> Vec<(u32, u32)> {
    d.end();
    d.0
}

/// `Gfx_TexScroll(0, frames & 0x7F, 64, 32)`: the light plane's.
fn light_plane_scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_tex_scroll(0, frames & 0x7F, 64, 32)
}

/// Its bakes.
pub fn bakes() -> Vec<MeshBake> {
    let dl = |name: &str, object: &str, list: &str, segments: Vec<(u8, BakeSegment)>, prelude: Vec<u8>| MeshBake {
        name: name.into(),
        object: object.into(),
        segments,
        prelude,
        body: BakeBody::DLists(vec![(object.into(), list.into())]),
    };
    vec![
        // DemoKankyo_DrawRain: its colours (dynamic: Hyrule Field's differ), Gfx_SetupDL(SETUPDL_20).
        dl(
            BAKE_RAIN,
            "object_efc_star_field",
            "object_efc_star_field_DL_000080",
            vec![(SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }), (SEG_SETUP, BakeSegment::Commands(with_end(setup_dl::setup_dl_20())))],
            vec![SEG_COLOR, SEG_SETUP],
        ),
        // DemoKankyo_DrawRock: Gfx_SetupDL_25Opa, (0, 0, 255, 155, 55, 255), env (155, 255, 55, 255).
        dl(
            BAKE_ROCK,
            "object_efc_star_field",
            "object_efc_star_field_DL_000DE0",
            vec![(SEG_COLOR, BakeSegment::Commands(vec![prim(0, 0, [255, 155, 55, 255]), env([155, 255, 55, 255]), END]))],
            vec![SEG_COLOR],
        ),
        // DemoKankyo_DrawClouds: (0, 0, 210, 210, 255, 255), env white, gDust5Tex on 8,
        // Gfx_SetupDL_61Xlu (its other modes replace the dither set before it).
        dl(
            BAKE_CLOUDS,
            "gameplay_keep",
            "gEffDustDL",
            vec![
                (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: "gDust5Tex".into() }),
                (SEG_COLOR, BakeSegment::Commands(vec![prim(0, 0, [210, 210, 255, 255]), env([255, 255, 255, 255]), END])),
                (SEG_SETUP, BakeSegment::Commands(with_end(setup_dl::setup_dl_61()))),
            ],
            vec![SEG_COLOR, SEG_SETUP],
        ),
        // DemoKankyo_DrawDoorOfTime: Gfx_SetupDL_25Opa, the two halves.
        dl(BAKE_DOOR_LEFT, "object_toki_objects", "object_toki_objects_DL_007440", vec![], vec![]),
        dl(BAKE_DOOR_RIGHT, "object_toki_objects", "object_toki_objects_DL_007578", vec![], vec![]),
        // DemoKankyo_DrawLightPlane: Gfx_SetupDL_25Xlu, segment 8's scroll.
        dl(BAKE_LIGHT_PLANE, "object_toki_objects", "object_toki_objects_DL_008390", vec![(SEG_TEX, BakeSegment::Dynamic(light_plane_scroll(0)))], vec![]),
    ]
}

fn bake_cmd(name: &str, m: Mat4, sv: Option<SegmentValues>) -> DrawCmd {
    DrawCmd { mesh: MeshKey::named(keys::bake(name)), transform: m, bones: Vec::new(), params: DrawParams { segments: sv, ..Default::default() } }
}

/// `DEG_TO_RAD`.
fn deg_to_rad(d: f32) -> f32 {
    (d as f64 * (std::f64::consts::PI / 180.0)) as f32
}

impl DemoKankyo {
    /// `DemoKankyo_DrawRain`'s draws.
    fn draw_rain(&self, out: &mut DrawOut) {
        let mut sv = SegmentValues::default();
        sv.prim[SEG_COLOR as usize] = Some(self.rain_colors.0);
        sv.env[SEG_COLOR as usize] = Some(self.rain_colors.1);
        for m in &self.rain {
            out.xlu.push(bake_cmd(BAKE_RAIN, *m, Some(sv.clone())));
        }
    }

    /// `DemoKankyo_DrawRock`: at its position, turned by its tumble (in degrees), scaled.
    fn draw_rock(&self, rs: &RenderState, out: &mut DrawOut) {
        let c = self.unk_150[0].unk_c;
        let m = Mat4::from_translation(rs.pos)
            * Mat4::from_rotation_x(deg_to_rad(c.x))
            * Mat4::from_rotation_y(deg_to_rad(c.y))
            * Mat4::from_rotation_z(deg_to_rad(c.z))
            * Mat4::from_scale(self.actor.scale);
        out.opa.push(bake_cmd(BAKE_ROCK, m, None));
    }

    /// `DemoKankyo_DrawClouds`: thirty puffs around the eye, 1200 up, a tier of 300 each.
    fn draw_clouds(&self, view: &ViewInfo, out: &mut DrawOut) {
        for (i, e) in self.unk_150.iter().enumerate() {
            let a = e.unk_20.wrapping_sub(0x8000u16 as i16);
            let r = 30.0 + (i as f32 / 30.0) * 10.0;
            let dx = -(sin_s(a) * 120.0) * r;
            let dy = cos_s(a) * 5.0 + 1200.0;
            let dz = (cos_s(a) * 120.0) * r;
            let eye = view.eye;
            let m = Mat4::from_translation(Vec3::new(eye.x + dx, eye.y + dy + ((i as f32 - 12.0) * 300.0), eye.z + dz)) * Mat4::from_scale(Vec3::new(125.0, 60.0, 125.0)) * view.billboard;
            out.xlu.push(bake_cmd(BAKE_CLOUDS, m, None));
        }
    }

    /// `DemoKankyo_DrawDoorOfTime`: the halves slid apart by `unk_18`.
    fn draw_door_of_time(&self, m: Mat4, out: &mut DrawOut) {
        let d = self.unk_150[0].unk_18;
        let left = m * Mat4::from_translation(Vec3::new(-d, 0.0, 0.0));
        out.opa.push(bake_cmd(BAKE_DOOR_LEFT, left, None));
        let right = left * Mat4::from_translation(Vec3::new(d + d, 0.0, 0.0));
        out.opa.push(bake_cmd(BAKE_DOOR_RIGHT, right, None));
    }

    /// `DemoKankyo_DrawLightPlane`: with no cutscene running or in a cutscene layer.
    fn draw_light_plane(&self, m: Mat4, play: &PlayState, out: &mut DrawOut) {
        if play.cs_ctx.state == CS_STATE_IDLE || play.save.scene_layer >= 4 {
            let mut sv = SegmentValues::default();
            sv.read(SEG_TEX, &light_plane_scroll(play.state_frames));
            out.xlu.push(bake_cmd(BAKE_LIGHT_PLANE, m, Some(sv)));
        }
    }

    /// The sparkles' draws: `gEffFlash1DL` facing the camera, rolled, (0, 0x80, white), their env.
    fn draw_sparkles(&self, view: &ViewInfo, out: &mut DrawOut) {
        for s in &self.sparkles {
            let m = s.mtx * view.billboard * Mat4::from_rotation_z(deg_to_rad(s.roll as f32));
            let mut sv = SegmentValues::default();
            sv.prim[SEG_COLOR as usize] = Some([255, 255, 255, 255]);
            sv.env[SEG_COLOR as usize] = Some([s.env[0], s.env[1], s.env[2], 255]);
            out.xlu.push(bake_cmd(crate::demo_effect::BAKE_FLASH, m, Some(sv)));
        }
    }
}

impl ActorImpl for DemoKankyo {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `DemoKankyo_Update`: its action.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::SetupType => self.setup_type(play),
            Action::UpdateClouds => self.update_clouds(),
            Action::UpdateRock => self.update_rock(play),
            Action::UpdateWarpIn => self.update_warp_in(),
            Action::UpdateDoorOfTime => self.update_door_of_time(play),
            Action::DoNothing => {}
            Action::KillDoorOfTimeCollision => self.kill_child(play),
        }
    }

    /// `DemoKankyo_Draw`'s changes, once per game frame: whether it draws (its object slot
    /// before this frame's check), the rain's and the sparkles' state, then the object slot.
    fn draw_update(&mut self, play: &mut PlayState) {
        self.drawn = false;
        self.warp_in_sfx = 0;
        if self.actor.killed {
            return;
        }
        if self.actor.obj_bank_index == self.required_object_slot {
            match self.actor.params {
                DEMOKANKYO_BLUE_RAIN | DEMOKANKYO_BLUE_RAIN_2 => {
                    if self.temple_rain_shown(play) {
                        self.draw_rain_update(play);
                        self.drawn = true;
                    }
                }
                DEMOKANKYO_WARP_OUT | DEMOKANKYO_WARP_IN => {
                    self.draw_warp_sparkles_update(play);
                    self.drawn = true;
                }
                DEMOKANKYO_SPARKLES => {
                    self.draw_sparkles_update(play);
                    self.drawn = true;
                }
                _ => self.drawn = true,
            }
        }
        if self.required_object_slot.is_some_and(|s| play.object_ctx.is_loaded(s)) {
            self.actor.obj_bank_index = self.required_object_slot;
        }
    }

    /// The warp-in sparkles' sound, once for each sparkle on its spline
    /// (`SFX_PLAY_CENTERED(NA_SE_EV_LINK_WARP_OUT - SFX_FLAG)`).
    fn draw_sfx(&mut self, play: &mut PlayState) {
        for _ in 0..self.warp_in_sfx {
            play.audio.play_sfx_centered(NA_SE_EV_LINK_WARP_OUT - SFX_FLAG);
        }
    }

    /// `actor.draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        if !self.drawn {
            return;
        }
        match self.actor.params {
            DEMOKANKYO_BLUE_RAIN | DEMOKANKYO_BLUE_RAIN_2 => self.draw_rain(out),
            DEMOKANKYO_ROCK_1..=DEMOKANKYO_ROCK_5 => self.draw_rock(rs, out),
            DEMOKANKYO_CLOUDS => self.draw_clouds(view, out),
            DEMOKANKYO_DOOR_OF_TIME => self.draw_door_of_time(actor_draw_matrix(rs), out),
            DEMOKANKYO_LIGHT_PLANE => self.draw_light_plane(actor_draw_matrix(rs), play, out),
            DEMOKANKYO_WARP_OUT | DEMOKANKYO_WARP_IN | DEMOKANKYO_SPARKLES => self.draw_sparkles(view, out),
            _ => {}
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
