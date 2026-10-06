//! `Bg_Ydan_Hasi` (`ovl_Bg_Ydan_Hasi/z_bg_ydan_hasi.c`): the Deku Tree's puzzle pieces of
//! `object_ydan_objects`, DynaPoly actors but the water.
//!
//! Params: bits 0..7 the kind (`HasiType`, which the init leaves in `params`), bits 8..13 the
//! switch flag (`type`).
//!
//! - **`HASI_WATER_BLOCK` (0), the floating block** (`gDTSlidingPlatformCol`, scaled 0.15 across):
//!   slides ±165 along its facing over 256 `gameplayFrames` and bobs ±2 on a 50-frame timer, 20
//!   above the scene's `waterBoxes[1]` (`BgYdanHasi_UpdateFloatingBlock`). Master Quest's room 5
//!   places it (`0xFF00`).
//! - **`HASI_WATER` (1), the water it floats on**: the init lowers `home` by 5 and puts
//!   `waterBoxes[1]`'s surface there. On its switch flag (`BgYdanHasi_InitWater`) it sinks 47 at
//!   0.5 a frame, waits 600 frames with the timer's tick, rises back at 1 and unsets the flag
//!   (`BgYdanHasi_MoveWater`, `BgYdanHasi_DecWaterTimer`), writing the water box's surface each
//!   frame it moves. It draws `gDTWaterPlaneDL` translucent under a scroll on segment 8. Room 5's
//!   (`0xFF01`) waits on flag 0x3F, which nothing in Master Quest sets ("never runs in Master
//!   Quest", the C's comment): its sinking is reached by the tests.
//! - **`HASI_THREE_BLOCKS` (2), room 10's rising platforms** (`gDTRisingPlatformsCol`): undrawn
//!   until the switch flag (`0x3D02`: room 10's floor switch), then one-point cutscene 3040 for 30
//!   frames, up 120 at 3 a frame, 260 frames all told, back down, drawn no more, the flag unset
//!   (`BgYdanHasi_SetupThreeBlocks`, `BgYdanHasi_UpdateThreeBlocks`). Their collision is there,
//!   undrawn, all along.
//!
//! The water box is the scene's collision header, written in place
//! (`CollisionContext::set_water_box_surface`), as the C writes `colCtx.colHeader`.
//!
//! The sandbox's course places a floating block in its channel (`PlayExt::spawn_platform`); the
//! course's `waterBoxes[1]` is that channel's water.

use std::f64::consts::PI;
use std::sync::Arc;

use anyhow::Result;
use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_TRANSFORM_POS, ScaleRotPos, srt_matrix};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, step_to_f};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTOR_BG_YDAN_HASI, ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::pack::{BakeBody, BakeSegment, GamePack, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

pub const OBJECT: &str = "object_ydan_objects";
/// `gDTSlidingPlatformCol`, the floating block's.
pub const COLLISION: &str = "gDTSlidingPlatformCol";
/// `gDTRisingPlatformsCol`, the three platforms'.
pub const RISING_COLLISION: &str = "gDTRisingPlatformsCol";
/// `BgYdanHasi_Draw`'s `dLists`, by kind.
pub const DISPLAY_LIST: &str = "gDTSlidingPlatformDL";
pub const WATER_DISPLAY_LIST: &str = "gDTWaterPlaneDL";
pub const RISING_DISPLAY_LIST: &str = "gDTRisingPlatformsDL";

/// `HasiType`.
pub const HASI_WATER_BLOCK: i16 = 0;
pub const HASI_WATER: i16 = 1;
pub const HASI_THREE_BLOCKS: i16 = 2;

/// The water box the C reads and writes: `play->colCtx.colHeader->waterBoxes[1]`.
pub const WATER_BOX: usize = 1;

/// `NA_SE_EV_ELEVATOR_MOVE`, `NA_SE_EV_WATER_LEVEL_DOWN` (`environmentbank_table.h`: 0x2824,
/// 0x285E).
pub const NA_SE_EV_ELEVATOR_MOVE: u16 = 0x2824;
pub const NA_SE_EV_WATER_LEVEL_DOWN: u16 = 0x285E;

/// The water's bake: `gDTWaterPlaneDL` after `Gfx_SetupDL_25Xlu` (the bake's start), with
/// `Gfx_TwoTexScroll` on segment 8 (docs/adr/0012-actor-bakes.md).
pub const BAKE_WATER: &str = "Bg_Ydan_Hasi/water";
const SEG_SCROLL: u8 = 0x08;

/// `BgYdanHasi_Draw`'s scroll at `gameplayFrames` (a `u32`, so `-frames % 128` is taken
/// unsigned): `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, -frames % 128, frames % 128, 0x20, 0x20, 1,
/// frames % 128, frames % 128, 0x20, 0x20)`.
pub fn water_scroll(gameplay_frames: u32) -> Vec<(u32, u32)> {
    let f = gameplay_frames % 128;
    gfx_two_tex_scroll(0, gameplay_frames.wrapping_neg() % 128, f, 0x20, 0x20, 1, f, f, 0x20, 0x20)
}

/// The water plane (the opaque lists are the pack's meshes of the object, `Gfx_DrawDListOpa`).
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE_WATER.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_SCROLL, BakeSegment::Dynamic(water_scroll(0)))],
        prelude: vec![],
        body: BakeBody::DLists(vec![(OBJECT.into(), WATER_DISPLAY_LIST.into())]),
    }]
}

/// `gDTSlidingPlatformCol`, from `object_ydan_objects`.
pub fn load_collision(pack: &GamePack) -> Result<Arc<CollisionHeader>> {
    Ok(Arc::new(pack.collision(&keys::collision(OBJECT, COLLISION))?))
}

/// `Bg_Ydan_Hasi_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_BG_YDAN_HASI,
    name: "Bg_Ydan_Hasi",
    category: ACTORCAT_BG,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    InitWater,
    UpdateFloatingBlock,
    SetupThreeBlocks,
    MoveWater,
    DecWaterTimer,
    UpdateThreeBlocks,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::InitWater => "BgYdanHasi_InitWater",
            Action::UpdateFloatingBlock => "BgYdanHasi_UpdateFloatingBlock",
            Action::SetupThreeBlocks => "BgYdanHasi_SetupThreeBlocks",
            Action::MoveWater => "BgYdanHasi_MoveWater",
            Action::DecWaterTimer => "BgYdanHasi_DecWaterTimer",
            Action::UpdateThreeBlocks => "BgYdanHasi_UpdateThreeBlocks",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BgYdanHasi {
    /// `dyna.actor`: position, `world.rot` = `shape.rot` (from the spawn) and scale.
    pub actor: Actor,
    /// `dyna.bgId` (`BG_ACTOR_MAX` for the water, which sets none: `BGACTOR_NEG_ONE`).
    pub bg: u16,
    pub action: Action,
    /// `type`: the switch flag.
    pub ty: u8,
    /// `timer`.
    pub timer: i16,
    /// `dyna.actor.draw` isn't `NULL` (the three platforms are undrawn until their flag).
    pub drawn: bool,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `waterBoxes[1].ySurface`, as the float the C's `f32` arithmetic reads.
fn water_surface(play: &PlayState) -> Option<f32> {
    let y = play.col.water_box_surface(WATER_BOX);
    if y.is_none() {
        log::error!("Bg_Ydan_Hasi: the scene has no waterBoxes[{WATER_BOX}]");
    }
    y.map(|y| y as f32)
}

/// `waterBox->ySurface = y`: the float truncated to the `s16`.
fn set_water_surface(play: &mut PlayState, y: f32) {
    if !play.col.set_water_box_surface(WATER_BOX, y as i32 as i16) {
        log::error!("Bg_Ydan_Hasi: the scene has no waterBoxes[{WATER_BOX}]");
    }
}

impl BgYdanHasi {
    /// `BgYdanHasi_Init` from `Actor_Spawn`: the kind's collision from the pack
    /// (`CollisionHeader_GetVirtual`), then [`BgYdanHasi::init_with`].
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let symbol = match (actor.params as u16 & 0xFF) as i16 {
            HASI_WATER => None,
            HASI_WATER_BLOCK => Some(COLLISION),
            _ => Some(RISING_COLLISION),
        };
        let header = symbol.and_then(|s| {
            let r = play.assets.as_ref()?.pack.collision(&keys::collision(OBJECT, s));
            r.map(Arc::new).map_err(|e| log::error!("Bg_Ydan_Hasi: {e:#}")).ok()
        });
        Box::new(Self::init_with(actor, play, header))
    }

    /// `BgYdanHasi_Init` on the collision `col_header` (the kind's: `gDTSlidingPlatformCol` or
    /// `gDTRisingPlatformsCol`, none for the water): scale 0.1 (`sInitChain`); the flag and the
    /// kind from the params; `DynaPolyActor_Init(DYNA_TRANSFORM_POS)`. The water lowers its home
    /// by 5 and puts `waterBoxes[1]`'s surface there; the block is 0.15 across, 20 above that
    /// surface; the platforms are undrawn with their focus 40 up. Both set their bg actor.
    pub fn init_with(mut actor: Actor, play: &mut PlayState, col_header: Option<Arc<CollisionHeader>>) -> BgYdanHasi {
        // ICHAIN_VEC3F_DIV1000(scale, 100).
        actor.scale = Vec3::splat(0.1);
        let params = actor.params as u16;
        // PARAMS_GET_U(params, 8, 6), PARAMS_GET_U(params, 0, 8).
        let ty = ((params >> 8) & 0x3F) as u8;
        actor.params = (params & 0xFF) as i16;
        let mut drawn = true;
        let mut bg = BG_ACTOR_MAX;
        let action;
        if actor.params == HASI_WATER {
            // waterBox->ySurface = world.pos.y = home.pos.y += -5.0f.
            actor.home_pos.y += -5.0;
            actor.world_pos.y = actor.home_pos.y;
            set_water_surface(play, actor.world_pos.y);
            action = Action::InitWater;
        } else {
            if actor.params == HASI_WATER_BLOCK {
                actor.scale.z = 0.15;
                actor.scale.x = 0.15;
                if let Some(y) = water_surface(play) {
                    actor.world_pos.y = y + 20.0;
                }
                action = Action::UpdateFloatingBlock;
            } else {
                drawn = false;
                action = Action::SetupThreeBlocks;
                actor.set_focus(40.0);
            }
            // DynaPoly_SetBgActor.
            bg = match col_header {
                Some(h) => play.col.dyna.set_bg_actor(h, source(&actor), DYNA_TRANSFORM_POS),
                None => BG_ACTOR_MAX,
            };
        }
        BgYdanHasi { actor, bg, action, ty, timer: 0, drawn }
    }

    pub fn pos(&self) -> Vec3 {
        self.actor.world_pos
    }

    /// The transform `DynaPoly_UpdateContext` reads from the actor.
    pub fn source(&self) -> BgActorSource {
        source(&self.actor)
    }

    /// `BgYdanHasi_UpdateFloatingBlock`: ±165 along the facing over 256 frames, 20 above the
    /// water, and ±2 of bobbing over the 50-frame timer.
    fn update_floating_block(&mut self, play: &PlayState) {
        // sinf((gameplayFrames & 0xFF) * (M_PI / 128)) * 165.0f: the product in double.
        let frames_after_math = (((play.gameplay_frames & 0xFF) as f64 * (PI / 128.0)) as f32).sin() * 165.0;
        let (home, yaw) = (self.actor.home_pos, self.actor.world_rot.y);
        self.actor.world_pos.x = (sin_s(yaw) * frames_after_math) + home.x;
        self.actor.world_pos.z = (cos_s(yaw) * frames_after_math) + home.z;
        if let Some(y) = water_surface(play) {
            self.actor.world_pos.y = y + 20.0;
        }
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.timer = 50;
        }
        self.actor.world_pos.y += 2.0 * ((self.timer as f64 * (PI / 25.0)) as f32).sin();
    }

    /// `BgYdanHasi_InitWater`: on the flag, 600 frames and `BgYdanHasi_MoveWater`.
    fn init_water(&mut self, play: &PlayState) {
        if play.flags.get_switch(self.ty as i32) {
            self.timer = 600;
            self.action = Action::MoveWater;
        }
    }

    /// `BgYdanHasi_MoveWater`: with the timer out, up to home at 1 a frame, then the flag unset
    /// and `BgYdanHasi_InitWater`; else down to home - 47 at 0.5, then
    /// `BgYdanHasi_DecWaterTimer`. The sound each frame, and the water box's surface at the
    /// new height.
    fn move_water(&mut self, play: &mut PlayState) {
        if self.timer == 0 {
            if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 1.0) {
                play.flags.unset_switch(self.ty as i32);
                self.action = Action::InitWater;
            }
            self.actor.play_sfx_flagged_centered2(NA_SE_EV_WATER_LEVEL_DOWN - SFX_FLAG);
        } else {
            if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y - 47.0, 0.5) {
                self.action = Action::DecWaterTimer;
            }
            self.actor.play_sfx_flagged_centered2(NA_SE_EV_WATER_LEVEL_DOWN - SFX_FLAG);
        }
        set_water_surface(play, self.actor.world_pos.y);
    }

    /// `BgYdanHasi_DecWaterTimer`: the timer down with its tick, then `BgYdanHasi_MoveWater`.
    fn dec_water_timer(&mut self) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        self.actor.play_sfx_flagged_timer(self.timer as i32);
        if self.timer == 0 {
            self.action = Action::MoveWater;
        }
    }

    /// `BgYdanHasi_SetupThreeBlocks`: on the flag, 260 frames, drawn, and one-point cutscene
    /// 3040 on them for 30 frames.
    fn setup_three_blocks(&mut self, play: &mut PlayState) {
        if play.flags.get_switch(self.ty as i32) {
            self.timer = 260;
            self.drawn = true;
            self.action = Action::UpdateThreeBlocks;
            let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
            play.onepoint_cutscene_init(3040, 30, me, CAM_ID_MAIN);
        }
    }

    /// `BgYdanHasi_UpdateThreeBlocks`: up to home + 120 at 3 a frame (the elevator's sound while
    /// moving, then the timer's tick); with the timer out, down to home at 3, then the flag unset,
    /// undrawn, and `BgYdanHasi_SetupThreeBlocks`.
    fn update_three_blocks(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 3.0) {
                play.flags.unset_switch(self.ty as i32);
                self.drawn = false;
                self.action = Action::SetupThreeBlocks;
            } else {
                self.actor.play_sfx_flagged_centered2(NA_SE_EV_ELEVATOR_MOVE - SFX_FLAG);
            }
        } else if !step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 120.0, 3.0) {
            self.actor.play_sfx_flagged_centered2(NA_SE_EV_ELEVATOR_MOVE - SFX_FLAG);
        } else {
            self.actor.play_sfx_flagged_timer(self.timer as i32);
        }
    }
}

impl ActorImpl for BgYdanHasi {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgYdanHasi_Update`, and the new transform for `DynaPoly_UpdateContext` (which runs after
    /// the BG category).
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::InitWater => self.init_water(play),
            Action::UpdateFloatingBlock => self.update_floating_block(play),
            Action::SetupThreeBlocks => self.setup_three_blocks(play),
            Action::MoveWater => self.move_water(play),
            Action::DecWaterTimer => self.dec_water_timer(),
            Action::UpdateThreeBlocks => self.update_three_blocks(play),
        }
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(self.bg, self.source());
        }
    }

    /// `BgYdanHasi_Destroy`: `DynaPoly_DeleteBgActor` (nothing for the water's
    /// `BGACTOR_NEG_ONE`).
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    /// `DynaPolyActor_IsPlayerOnTop` and the like read the interact flags `Actor_UpdateAll`
    /// clears after it.
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.drawn as u32];
        rs
    }

    /// `BgYdanHasi_Draw`: the block's and the platforms' lists with `Gfx_DrawDListOpa` under
    /// `Actor_Draw`'s matrix (the same translate/rotate/scale DynaPoly uses); the water plane
    /// translucent (`Gfx_SetupDL_25Xlu`), its scroll on segment 8. Nothing while `draw` is
    /// `NULL`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.first() != Some(&1) {
            return;
        }
        match self.actor.params {
            HASI_WATER_BLOCK | HASI_THREE_BLOCKS => {
                let dl = if self.actor.params == HASI_WATER_BLOCK { DISPLAY_LIST } else { RISING_DISPLAY_LIST };
                let t = ScaleRotPos { scale: rs.scale, rot: rs.rot, pos: rs.pos };
                out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, dl)), srt_matrix(&t)));
            }
            _ => {
                let mut sv = SegmentValues::default();
                sv.read(SEG_SCROLL, &water_scroll(play.gameplay_frames));
                out.xlu.push(DrawCmd {
                    mesh: MeshKey::named(keys::bake(BAKE_WATER)),
                    transform: actor_draw_matrix(rs),
                    bones: Vec::new(),
                    params: DrawParams { segments: Some(sv), ..Default::default() },
                });
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
