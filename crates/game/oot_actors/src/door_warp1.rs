//! `Door_Warp1` (`ovl_Door_Warp1/z_door_warp1.c`): the blue warp a boss leaves, and the warps
//! of the other kinds (the adult dungeons' crystal, the Sages', Ruto's, the coloured set pieces).
//!
//! This ROM's Deku Tree reaches two of them:
//! - **`WARP_DUNGEON_CHILD` (0)**, Queen Gohma's: it grows in (`DoorWarp1_WarpAppear`: the light
//!   rays and the ring to their size, `NA_SE_EV_WARP_HOLE`), then waits (`DoorWarp1_ChildWarpIdle`)
//!   until Link stands in it (within 60 across and 20 up or down): `NA_SE_EV_LINK_WARP`, one-point
//!   9703 (the camera round him), Link walking to its centre (`PLAYER_CSACTION_10`); then
//!   (`DoorWarp1_ChildWarpOut`) he floats up (his gravity 0.1 from 100 frames on), the rays fade
//!   and close in on him with its two lights, and 100 frames on: the Deku Tree's first time
//!   `EVENTCHKINF_07` and `_09`, the Kokiri Emerald (`Item_Give`), `ENTR_KOKIRI_FOREST_0` with
//!   cutscene 0xFFF1 (the Deku Tree's emerald, part 1); later `ENTR_KOKIRI_FOREST_11`; the slow
//!   white fade, white in.
//! - **`WARP_DESTINATION` (6)**, at a blue warp's arrival (Kokiri Forest's layer 5): killed at its
//!   init unless the entrance is an adult warp's arrival or a cutscene layer's, Link arrived by
//!   blue warp, and he's within 100; else it fades in and out over 80 frames
//!   (`DoorWarp1_Destination`).
//!
//! Its draw (`DoorWarp1_DrawWarp`) is `gWarpPortalDL` twice, a ring of 13 vertices at the warp
//! (segment 0x0A's matrix) joined to one raised and widened (segment 9's): the warp, then the
//! light rays; the textures scrolling (`Gfx_TwoTexScroll`). The matrices are the draw's bones
//! (`BakeSegment::Matrix`), its colours dynamic.
//!
//! Not this ROM's Deku Tree, logged: the adult dungeons' warp and its crystal
//! (`DoorWarp1_SetupAdultDungeonWarp` and its actions), the blue and purple crystals, the clear
//! flag's warp, Ruto's warp, `WARP_UNK_7`, `func_809998A4` (the Sages' and the yellow warp's fade),
//! the other bosses' rooms in `DoorWarp1_ChildWarpOut`.

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{smooth_step_to_f, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::*;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::lights::{LightInfo, LightNode};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::sys_matrix::MtxF;

use crate::boss_goma::ACTOR_DOOR_WARP1;

const OBJECT: &str = "object_warp1";

/// `Door_Warp1_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_DOOR_WARP1, name: "Door_Warp1", category: ACTORCAT_ITEMACTION, flags: 0, object: OBJECT };

/// `DoorWarp1Type`.
pub const WARP_BLUE_CRYSTAL: i16 = -2;
pub const WARP_DUNGEON_ADULT: i16 = -1;
pub const WARP_DUNGEON_CHILD: i16 = 0;
pub const WARP_CLEAR_FLAG: i16 = 1;
pub const WARP_SAGES: i16 = 2;
pub const WARP_PURPLE_CRYSTAL: i16 = 3;
pub const WARP_YELLOW: i16 = 4;
pub const WARP_BLUE_RUTO: i16 = 5;
pub const WARP_DESTINATION: i16 = 6;
pub const WARP_UNK_7: i16 = 7;
pub const WARP_ORANGE: i16 = 8;
pub const WARP_GREEN: i16 = 9;
pub const WARP_RED: i16 = 10;

/// `PLAYER_CSACTION_10` (`player.h`): walking to the warp's centre (`unk_450`).
const PLAYER_CSACTION_10: u8 = 10;
/// `SCENE_DEKU_TREE_BOSS` (`scene_table.h`).
const SCENE_DEKU_TREE_BOSS: u16 = 0x11;
/// `ITEM_KOKIRI_EMERALD` (`item.h`).
const ITEM_KOKIRI_EMERALD: u8 = 0x6C;
/// `CS_INDEX_1`, `NEXT_CS_INDEX_NONE`, `CS_INDEX_NONE` (`save.h`).
const CS_INDEX_1: u16 = 0xFFF1;
const NEXT_CS_INDEX_NONE: u16 = 0xFFEF;
const CS_INDEX_NONE: u16 = 0xFFEF;
/// The entrances an adult's warp arrives at (`DoorWarp1_SetupWarp`'s `WARP_DESTINATION` case).
const DESTINATION_ENTRANCES: [&str; 5] = ["ENTR_SACRED_FOREST_MEADOW_3", "ENTR_DEATH_MOUNTAIN_CRATER_5", "ENTR_LAKE_HYLIA_9", "ENTR_DESERT_COLOSSUS_8", "ENTR_GRAVEYARD_8"];

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WarpAppear,
    ChildWarpIdle,
    ChildWarpOut,
    Destination,
    DoNothing,
    /// An action of a warp this ROM's Deku Tree never makes (logged at its setup): nothing.
    NotPorted(&'static str),
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::WarpAppear => "DoorWarp1_WarpAppear",
            Action::ChildWarpIdle => "DoorWarp1_ChildWarpIdle",
            Action::ChildWarpOut => "DoorWarp1_ChildWarpOut",
            Action::Destination => "DoorWarp1_Destination",
            Action::DoNothing => "DoorWarp1_DoNothing",
            Action::NotPorted(n) => n,
        }
    }
}

/// The overlay's statics: `sWarpTimerTarget`.
#[derive(Debug, Default)]
struct Statics {
    warp_timer_target: i16,
}

pub struct DoorWarp1 {
    pub actor: Actor,
    pub action: Action,
    pub warp_timer: u16,
    pub unk_194: f32,
    pub unk_198: f32,
    pub unk_19c: f32,
    pub light_ray_alpha: f32,
    pub warp_alpha: f32,
    pub crystal_alpha: f32,
    pub scale: i16,
    pub unk_1ae: i16,
    pub unk_1b0: i16,
    pub unk_1b2: i16,
    pub unk_1b4: f32,
    pub unk_1b8: i16,
    pub unk_1bc: f32,
    pub upper_light: Option<LightNode>,
    pub lower_light: Option<LightNode>,
}

impl DoorWarp1 {
    /// `DoorWarp1_Init`: its two lights (but for the Sages', the blue crystal, the yellow warp and
    /// the destination), then `DoorWarp1_ChooseInitialAction`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: scale 1 (then the update's Actor_SetScale), the culling volumes (no actor
        // culls). ActorShape_Init(&shape, 0, NULL, 0).
        actor.scale = Vec3::ONE;
        actor.shape_y_offset = 0.0;
        log::debug!("BOSSWARP arg_data=[{}]", actor.params);
        let mut w = DoorWarp1 {
            actor,
            action: Action::DoNothing,
            warp_timer: 0,
            unk_194: 0.0,
            unk_198: 0.0,
            unk_19c: 0.0,
            light_ray_alpha: 0.0,
            warp_alpha: 0.0,
            crystal_alpha: 0.0,
            scale: 0,
            unk_1ae: 0,
            unk_1b0: 0,
            unk_1b2: 0,
            unk_1b4: 0.0,
            unk_1b8: 0,
            unk_1bc: 0.0,
            upper_light: None,
            lower_light: None,
        };
        let p = w.actor.params;
        if p != WARP_SAGES && p != WARP_BLUE_CRYSTAL && p != WARP_YELLOW && p != WARP_DESTINATION {
            let info = w.light_at(0, 0, 0, 0);
            w.upper_light = play.light_ctx.insert_light(info);
            w.lower_light = play.light_ctx.insert_light(info);
        }
        w.choose_initial_action(play);
        Box::new(w)
    }

    /// `Lights_PointNoGlowSetInfo(info, world.pos, r, g, b, radius)` at the warp.
    fn light_at(&self, r: u8, g: u8, b: u8, radius: i16) -> LightInfo {
        let p = self.actor.world_pos;
        LightInfo::point_no_glow(p.x as i16, p.y as i16, p.z as i16, [r, g, b], radius)
    }

    fn statics(play: &mut PlayState) -> &mut Statics {
        play.overlay_static::<Statics>(ACTOR_DOOR_WARP1)
    }

    /// `DoorWarp1_ChooseInitialAction`.
    fn choose_initial_action(&mut self, play: &mut PlayState) {
        match self.actor.params {
            WARP_DUNGEON_CHILD | WARP_CLEAR_FLAG | WARP_SAGES | WARP_YELLOW | WARP_BLUE_RUTO | WARP_DESTINATION | WARP_UNK_7 | WARP_ORANGE | WARP_GREEN | WARP_RED => self.setup_warp(play),
            WARP_DUNGEON_ADULT => self.not_ported("DoorWarp1_SetupAdultDungeonWarp"),
            WARP_BLUE_CRYSTAL => self.not_ported("DoorWarp1_SetupBlueCrystal"),
            WARP_PURPLE_CRYSTAL => self.not_ported("DoorWarp1_SetupPurpleCrystal"),
            _ => {}
        }
    }

    /// A warp's setup or action this ROM's Deku Tree never reaches: logged, and the warp does
    /// nothing.
    fn not_ported(&mut self, name: &'static str) {
        log::warn!("Door_Warp1 params {}: {name} isn't ported (not this ROM's Deku Tree)", self.actor.params);
        self.action = Action::NotPorted(name);
    }

    /// `DoorWarp1_SetupWarp`.
    fn setup_warp(&mut self, play: &mut PlayState) {
        self.scale = 0;
        self.unk_1ae = -140;
        self.unk_1b0 = -80;
        Self::statics(play).warp_timer_target = 100;
        self.unk_1bc = 1.0;
        self.light_ray_alpha = 0.0;
        self.warp_alpha = 0.0;
        self.crystal_alpha = 0.0;
        match self.actor.params {
            WARP_YELLOW | WARP_ORANGE | WARP_GREEN | WARP_RED => {
                self.unk_194 = 0.23;
                self.unk_198 = 0.6;
            }
            WARP_DESTINATION => {
                self.unk_194 = 0.0;
                self.unk_198 = 0.0;
            }
            WARP_UNK_7 => {
                self.scale = 100;
                self.unk_1ae = 120;
                self.unk_1b0 = 230;
                self.unk_194 = 0.3;
                self.unk_198 = 0.3;
            }
            _ => {
                self.unk_194 = 0.3;
                self.unk_198 = 0.3;
            }
        }
        self.unk_19c = 0.0;
        self.actor.shape_y_offset = 1.0;
        self.warp_timer = 0;
        match self.actor.params {
            WARP_CLEAR_FLAG | WARP_SAGES | WARP_YELLOW | WARP_DESTINATION | WARP_ORANGE | WARP_GREEN | WARP_RED => {}
            _ => {
                let info = self.light_at(200, 255, 255, 255);
                play.light_ctx.set_info(self.upper_light, info);
                play.light_ctx.set_info(self.lower_light, info);
            }
        }
        match self.actor.params {
            WARP_CLEAR_FLAG => self.not_ported("DoorWarp1_AwaitClearFlag"),
            WARP_DESTINATION => {
                let entrance = play.save.entrance_index;
                let arrival = DESTINATION_ENTRANCES.iter().any(|e| play.entrance_by_name(e) == Some(entrance));
                let cutscene_layer = play.save.scene_layer >= 4;
                let player = play.player.and_then(|h| play.actors.actor(h));
                // PARAMS_GET_NOSHIFT(GET_PLAYER(play)->actor.params, 8, 4): Link's start mode.
                let blue_warp = player.is_some_and(|p| (p.params as u16 & 0xF00) == 0x200);
                if (!arrival && !cutscene_layer) || !blue_warp {
                    self.actor.kill();
                }
                let near = player.is_some_and(|p| {
                    let (dx, dz) = (p.world_pos.x - self.actor.world_pos.x, p.world_pos.z - self.actor.world_pos.z);
                    (dx * dx + dz * dz).sqrt() <= 100.0
                });
                if !near {
                    self.actor.kill();
                }
                self.action = Action::Destination;
            }
            WARP_UNK_7 => self.not_ported("func_8099B020"),
            _ => self.action = Action::WarpAppear,
        }
    }

    /// `DoorWarp1_WarpAppear`: `NA_SE_EV_WARP_HOLE`, the rays and the warp to 255; the ring grows
    /// (2 a frame to 100), the rays' widths (4 a frame to 120 and 230); then idle.
    fn warp_appear(&mut self, play: &mut PlayState) {
        audio_play_actor_sfx2(play, NA_SE_EV_WARP_HOLE - SFX_FLAG);
        smooth_step_to_f(&mut self.light_ray_alpha, 255.0, 0.4, 10.0, 0.01);
        smooth_step_to_f(&mut self.warp_alpha, 255.0, 0.4, 10.0, 0.01);
        let p = self.actor.params;
        if p != WARP_YELLOW && p != WARP_ORANGE && p != WARP_GREEN && p != WARP_RED {
            if self.scale < 100 {
                self.scale += 2;
            }
            if self.unk_1ae < 120 {
                self.unk_1ae += 4;
            }
            if self.unk_1b0 < 230 {
                self.unk_1b0 += 4;
            } else if p == WARP_BLUE_RUTO {
                self.not_ported("DoorWarp1_RutoWarpIdle");
            } else if p != WARP_SAGES && p != WARP_YELLOW {
                self.action = Action::ChildWarpIdle;
            } else {
                self.not_ported("func_809998A4");
            }
        } else {
            if self.unk_1ae < -50 {
                self.unk_1ae += 4;
            }
            if self.unk_1b0 < 70 {
                self.unk_1b0 += 4;
            } else {
                self.not_ported("func_809998A4");
            }
        }
    }

    /// `DoorWarp1_PlayerInRange`: Link within 60 across, and within 20 up or down.
    fn player_in_range(&self, play: &PlayState) -> bool {
        let Some(py) = play.player.and_then(|h| play.actors.actor(h)).map(|p| p.world_pos.y) else { return false };
        self.actor.xz_dist_to_player.abs() < 60.0 && (py - 20.0) < self.actor.world_pos.y && self.actor.world_pos.y < (py + 20.0)
    }

    /// `DoorWarp1_ChildWarpIdle`: Link in it: `NA_SE_EV_LINK_WARP`, one-point 9703 for 999 frames,
    /// his walk to its centre (`PLAYER_CSACTION_10`, `unk_450`).
    fn child_warp_idle(&mut self, play: &mut PlayState) {
        audio_play_actor_sfx2(play, NA_SE_EV_WARP_HOLE - SFX_FLAG);
        if self.player_in_range(play) {
            let Some(ph) = play.player else { return };
            // SFX_PLAY_AT_POS(&player->actor.projectedPos, NA_SE_EV_LINK_WARP).
            play.audio.play_sfx_at_pos(SfxPos::Actor(ph), NA_SE_EV_LINK_WARP);
            let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
            play.onepoint_cutscene_init(9703, 999, me, CAM_ID_MAIN);
            let cur = play.cur_actor;
            play.player_set_cs_action_with_halted_actors(cur, PLAYER_CSACTION_10);
            let pos = self.actor.world_pos;
            if let Some(p) = play.actors.downcast_mut::<crate::player::Player>(ph) {
                p.unk_450.x = pos.x;
                p.unk_450.z = pos.z;
            }
            self.unk_1b2 = 1;
            self.action = Action::ChildWarpOut;
        }
    }

    /// `DoorWarp1_ChildWarpOut`: Link floating up (his gravity 0.1 while he rises slower than 10,
    /// from frame 101); the rays fading; past `sWarpTimerTarget` (100) with no next cutscene: the
    /// way out (the Deku Tree's room: its first time the emerald and the flags, then
    /// `ENTR_KOKIRI_FOREST_0` with 0xFFF1; later `ENTR_KOKIRI_FOREST_11`), the slow white fade,
    /// white in; the warp's widths and its two lights round him, its offset down to 0.
    fn child_warp_out(&mut self, play: &mut PlayState) {
        let Some(ph) = play.player else { return };
        if self.unk_1b2 >= 101 {
            if let Some(p) = play.actors.actor_mut(ph) {
                p.gravity = if p.velocity.y < 10.0 { 0.1 } else { 0.0 };
            }
        } else {
            self.unk_1b2 += 1;
        }
        smooth_step_to_f(&mut self.light_ray_alpha, 0.0, 0.2, 6.0, 0.01);
        self.warp_timer = self.warp_timer.wrapping_add(1);
        let target = Self::statics(play).warp_timer_target;
        if (target as i32) < self.warp_timer as i32 && play.save.next_cutscene_index == NEXT_CS_INDEX_NONE {
            log::debug!("The time has come, so it's over. fade_direction=[{}]", play.transition.trigger);
            if play.scene_id == SCENE_DEKU_TREE_BOSS {
                if !play.save.get_event_chk_inf(oot_game::save::EVENTCHKINF_07) {
                    play.save.set_event_chk_inf(oot_game::save::EVENTCHKINF_07);
                    play.save.set_event_chk_inf(oot_game::save::EVENTCHKINF_09);
                    oot_game::item::item_give(&mut play.save, Some(&mut play.audio), ITEM_KOKIRI_EMERALD);
                    self.go(play, "ENTR_KOKIRI_FOREST_0");
                    play.save.next_cutscene_index = CS_INDEX_1;
                } else {
                    self.go(play, "ENTR_KOKIRI_FOREST_11");
                    play.save.next_cutscene_index = CS_INDEX_NONE;
                }
            } else {
                // SCENE_DODONGOS_CAVERN_BOSS, SCENE_JABU_JABU_BOSS: not this ROM's Deku Tree.
                log::warn!("Door_Warp1: DoorWarp1_ChildWarpOut's way out of scene {:#x} isn't ported", play.scene_id);
            }
            log::debug!("The end The end");
            play.transition.trigger = oot_game::transition::TRANS_TRIGGER_START;
            play.transition.ty = oot_game::transition::TRANS_TYPE_FADE_WHITE_SLOW;
            play.save.next_transition_type = oot_game::transition::TRANS_TYPE_FADE_WHITE;
        }
        step_to_f(&mut self.unk_194, 2.0, 0.01);
        step_to_f(&mut self.unk_198, 10.0, 0.02);
        let pp = play.actors.actor(ph).map(|p| p.world_pos).unwrap_or_default();
        // (s16)player->actor.world.pos.x + 10.0f, as the light's s16 coordinates.
        let at = |v: f32, d: f32| ((v as i16) as f32 + d) as i16;
        play.light_ctx.set_info(self.upper_light, LightInfo::point_no_glow(at(pp.x, 10.0), at(pp.y, 10.0), at(pp.z, 10.0), [235, 255, 255], 255));
        play.light_ctx.set_info(self.lower_light, LightInfo::point_no_glow(at(pp.x, -10.0), at(pp.y, -10.0), at(pp.z, -10.0), [235, 255, 255], 255));
        smooth_step_to_f(&mut self.actor.shape_y_offset, 0.0, 0.5, 2.0, 0.1);
    }

    /// `play->nextEntranceIndex = entrance`.
    fn go(&self, play: &mut PlayState, entrance: &str) {
        match play.entrance_by_name(entrance) {
            Some(e) => play.transition.next_entrance_index = e,
            None => log::error!("Door_Warp1: no entrance {entrance}"),
        }
    }

    /// `DoorWarp1_Destination`: in over 20 frames, held, out over 20 from 60, then nothing;
    /// `NA_SE_EV_WARP_HOLE`.
    fn destination(&mut self, play: &mut PlayState) {
        let mut alpha_frac = 1.0f32;
        self.unk_194 = 5.0;
        self.warp_timer = self.warp_timer.wrapping_add(1);
        if self.warp_timer < 20 {
            alpha_frac = self.warp_timer as f32 / 20.0;
        } else if self.warp_timer >= 60 {
            alpha_frac = 1.0 - ((self.warp_timer as f32 - 60.0) / 20.0);
        }
        self.warp_alpha = 255.0 * alpha_frac;
        self.light_ray_alpha = 0.0;
        if self.warp_timer as f32 >= 80.0 {
            self.warp_alpha = 0.0;
            self.action = Action::DoNothing;
        }
        audio_play_actor_sfx2(play, NA_SE_EV_WARP_HOLE - SFX_FLAG);
    }
}

/// The segments of its bake: the two matrices (`gSPMatrix` of 0x0A, then of 9), the scroll (8),
/// and the colours (`gDPSetPrimColor`, `gDPSetEnvColor` before the list).
const SEG_MTX_BASE: u8 = 0x0A;
const SEG_MTX_RING: u8 = 0x09;
const SEG_SCROLL: u8 = 0x08;
const SEG_COLOR: u8 = 0x0B;
const BAKE_PORTAL: &str = "Door_Warp1/portal";

/// `Gfx_TwoTexScroll(G_TX_RENDERTILE, x, y, 0x100, 0x100, 1, x, y, 0x100, 0x100)`: both tiles
/// scrolled alike.
fn portal_scroll(x: u32, y: i32) -> Vec<(u32, u32)> {
    oot_game::scene_table::gfx_two_tex_scroll(0, x, y as u32, 0x100, 0x100, 1, x, y as u32, 0x100, 0x100)
}

/// `DoorWarp1_DrawWarp`'s mesh: `gWarpPortalDL` after `Gfx_SetupDL_25Xlu`, its two matrices on
/// bones 0 (segment 0x0A) and 1 (segment 9), its scroll and colours dynamic.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE_PORTAL.into(),
        object: OBJECT.into(),
        segments: vec![
            (SEG_MTX_BASE, BakeSegment::Matrix(0)),
            (SEG_MTX_RING, BakeSegment::Matrix(1)),
            (SEG_SCROLL, BakeSegment::Dynamic(portal_scroll(0, 0))),
            (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
        ],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(vec![(OBJECT.into(), "gWarpPortalDL".into())]),
    }]
}

/// `render_state`'s values: `unk_194`, `unk_198`, `unk_19C`, `lightRayAlpha`, `warpAlpha`, `scale`
/// as a float... (see `draw`); switches: the params, `unk_1AE`, `unk_1B0`, `play->state.frames`.
mod rs {
    pub const UNK_194: usize = 0;
    pub const UNK_198: usize = 1;
    pub const UNK_19C: usize = 2;
    pub const RAY_ALPHA: usize = 3;
    pub const WARP_ALPHA: usize = 4;
}

impl ActorImpl for DoorWarp1 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `DoorWarp1_Update`: its action, then its scale (`scale / 100`) but for the purple crystal.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::WarpAppear => self.warp_appear(play),
            Action::ChildWarpIdle => self.child_warp_idle(play),
            Action::ChildWarpOut => self.child_warp_out(play),
            Action::Destination => self.destination(play),
            Action::DoNothing | Action::NotPorted(_) => {}
        }
        if self.actor.params != WARP_PURPLE_CRYSTAL {
            self.actor.scale = Vec3::splat(self.scale as f32 / 100.0);
        }
    }

    /// `DoorWarp1_Destroy`: its lights out, the room's light adjustments back to 0. (The crystals'
    /// `SkelAnime_Free` missing is the C's `//! @bug`: no skeleton here.)
    fn destroy(&mut self, play: &mut PlayState) {
        play.light_ctx.remove_light(self.upper_light.take());
        play.light_ctx.remove_light(self.lower_light.take());
        for i in 0..3 {
            play.env_ctx.adj_ambient_color[i] = 0;
            play.env_ctx.adj_fog_color[i] = 0;
            play.env_ctx.adj_light1_color[i] = 0;
        }
    }

    /// `DoorWarp1_DrawWarp`'s change to the warp: `unk_19C` turned by `(s16)(temp_f0 × 15)` (but
    /// for the coloured and destination warps), and by `-(s16)(temp_f0 × 2)` for the destination,
    /// once per game frame.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.actor.killed || matches!(self.action, Action::NotPorted(_)) {
            return;
        }
        let p = self.actor.params;
        let temp_f0 = 1.0 - (2.0 - self.unk_194) / 1.7;
        if p != WARP_YELLOW && p != WARP_DESTINATION && p != WARP_ORANGE && p != WARP_GREEN && p != WARP_RED {
            self.unk_19c += (temp_f0 * 15.0) as i16 as f32;
        }
        if p == WARP_DESTINATION {
            self.unk_19c -= (temp_f0 * 2.0) as i16 as f32;
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.values = vec![self.unk_194, self.unk_198, self.unk_19c, self.light_ray_alpha, self.warp_alpha];
        r.switches = vec![self.actor.params as u16 as u32, self.unk_1ae as u16 as u32, self.unk_1b0 as u16 as u32, matches!(self.action, Action::NotPorted(_)) as u32];
        r
    }

    /// `DoorWarp1_Draw` for the warps (`DoorWarp1_DrawWarp`): the warp's ring raised by
    /// `unk_194 × 230` and widened by `unk_1AE`, then, with rays, the rays' raised by `unk_198 × 60`
    /// and widened by `unk_1B0`; both from the warp 1 above its position, their textures scrolling
    /// by `play->state.frames` (twice as fast for the rays) and `unk_19C`, prim and env colours by
    /// `temp_f0` (or the coloured warps' own). The crystals' draws aren't this ROM's Deku Tree.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let ([params, unk_1ae, unk_1b0, not_ported], v) = (rs.switches.as_slice(), rs.values.as_slice()) else { return };
        if *not_ported != 0 || v.len() < 5 {
            return;
        }
        let params = *params as u16 as i16;
        let (unk_1ae, unk_1b0) = (*unk_1ae as u16 as i16, *unk_1b0 as u16 as i16);
        let (unk_194, unk_198, unk_19c, ray_alpha, warp_alpha) = (v[rs::UNK_194], v[rs::UNK_198], v[rs::UNK_19C], v[rs::RAY_ALPHA], v[rs::WARP_ALPHA]);
        let sp_ec = play.state_frames.wrapping_mul(10);
        let sp_e8 = if unk_194 >= 1.0 { 0.0 } else { 1.0 - unk_194 };
        let sp_e4 = if unk_198 >= 1.0 { 0.0 } else { 1.0 - unk_198 };
        let temp_f0 = 1.0 - (2.0 - unk_194) / 1.7;
        // The coloured warps' prim and env, or the blue warp's by temp_f0.
        let colors = |alpha: f32| -> ([u8; 4], [u8; 4]) {
            let a = alpha as u8;
            match params {
                WARP_YELLOW => ([255, 255, 255, a], [200, 255, 0, 255]),
                WARP_ORANGE => ([255, 255, 255, a], [255, 150, 0, 255]),
                WARP_GREEN => ([255, 255, 255, a], [0, 200, 0, 255]),
                WARP_RED => ([255, 255, 255, a], [255, 50, 0, 255]),
                _ => ([(255.0 * temp_f0) as u8, 255, 255, a], [0, (255.0 * temp_f0) as u8, 255, 255]),
            }
        };
        // Matrix_Translate(world.x, world.y + 1, world.z, MTXMODE_NEW): segment 0x0A.
        let p = rs.pos;
        let base = MtxF::set_translate(p.x, p.y + 1.0, p.z);
        let draw_ring = |out: &mut DrawOut, raise: f32, width: i16, widen: f32, scroll_y: i32, scroll_x: u32, alpha: f32| {
            let mut m = base;
            m.translate(0.0, raise, 0.0);
            let xz = (width as f32 * widen) / 100.0 + 1.0;
            m.scale(xz, 1.0, xz);
            let (prim, env) = colors(alpha);
            let mut sv = SegmentValues::default();
            sv.read(SEG_SCROLL, &portal_scroll(scroll_x, scroll_y));
            sv.prim[SEG_COLOR as usize] = Some(prim);
            sv.env[SEG_COLOR as usize] = Some(env);
            out.xlu.push(DrawCmd {
                mesh: MeshKey::named(keys::bake(BAKE_PORTAL)),
                transform: Mat4::IDENTITY,
                bones: vec![base.to_mat4(), m.to_mat4()],
                params: DrawParams { segments: Some(sv), ..Default::default() },
            });
        };
        // The warp: -((s16)(unk_19C + unk_19C) & 511).
        let y1 = -(((unk_19c + unk_19c) as i32 as i16 as i32) & 511);
        draw_ring(out, unk_194 * 230.0, unk_1ae, sp_e8, y1, sp_ec & 0xFF, warp_alpha);
        if ray_alpha > 0.0 {
            let sp_ec2 = sp_ec.wrapping_mul(2);
            let y2 = -((unk_19c as i32 as i16 as i32) & 511);
            draw_ring(out, unk_198 * 60.0, unk_1b0, sp_e4, y2, sp_ec2 & 0xFF, ray_alpha);
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
