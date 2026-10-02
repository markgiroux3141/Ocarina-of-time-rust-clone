//! `Bg_Ydan_Hasi` with params `HASI_WATER_BLOCK` (0): the Deku Tree B1 platform that floats
//! on the water, sliding back and forth along its facing and bobbing, ported from
//! `z_bg_ydan_hasi.c` (`BgYdanHasi_Init`, `BgYdanHasi_UpdateFloatingBlock`). Its collision is
//! `gDTSlidingPlatformCol` and its display list `gDTSlidingPlatformDL`, both in
//! `object_ydan_objects`, from the asset pack.
//!
//! The game reads the water height from the scene's `waterBoxes[1]`; here the caller passes
//! the surface the platform floats on.

use std::f32::consts::PI;
use std::sync::Arc;

use anyhow::Result;
use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BgActorSource, DYNA_TRANSFORM_POS, Dyna, ScaleRotPos, srt_matrix};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_DRAW_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTOR_BG_YDAN_HASI, ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::pack::{GamePack, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

pub const OBJECT: &str = "object_ydan_objects";
pub const COLLISION: &str = "gDTSlidingPlatformCol";
pub const DISPLAY_LIST: &str = "gDTSlidingPlatformDL";

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

#[derive(Debug, Clone)]
pub struct BgYdanHasi {
    /// `dyna.actor`: position, `world.rot` = `shape.rot` (from the spawn) and scale.
    pub actor: Actor,
    pub timer: i16,
    pub water_surface: f32,
    /// `dyna.bgId`.
    pub bg: u16,
}

impl BgYdanHasi {
    /// `BgYdanHasi_Init`: `ICHAIN_VEC3F_DIV1000(scale, 100)`, then x/z scale 0.15, the
    /// position 20 above the water, `DynaPolyActor_Init(DYNA_TRANSFORM_POS)`, `DynaPoly_SetBgActor`.
    pub fn new(dyna: &mut Dyna, header: Arc<CollisionHeader>, home: Vec3, yaw: i16, water_surface: f32) -> BgYdanHasi {
        let mut actor = Actor::new(home, yaw);
        PROFILE.apply(&mut actor);
        actor.world_pos = Vec3::new(home.x, water_surface + 20.0, home.z);
        actor.scale = Vec3::new(0.15, 0.1, 0.15);
        let mut p = BgYdanHasi { actor, timer: 0, water_surface, bg: 0 };
        p.bg = dyna.set_bg_actor(header, p.source(), DYNA_TRANSFORM_POS);
        p
    }

    pub fn pos(&self) -> Vec3 {
        self.actor.world_pos
    }

    /// The transform `DynaPoly_UpdateContext` reads from the actor.
    pub fn source(&self) -> BgActorSource {
        let r = self.actor.shape_rot;
        BgActorSource { pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z], scale: self.actor.scale, shape_y_offset: 0.0 }
    }

    /// `BgYdanHasi_UpdateFloatingBlock`: ±165 along the facing over 256 frames, and ±2 of
    /// bobbing over the 50-frame timer.
    pub fn update_floating_block(&mut self, gameplay_frames: u32) {
        let f = ((gameplay_frames & 0xFF) as f32 * (PI / 128.0)).sin() * 165.0;
        let (home, yaw) = (self.actor.home_pos, self.actor.shape_rot.y);
        self.actor.world_pos.x = sin_s(yaw) * f + home.x;
        self.actor.world_pos.z = cos_s(yaw) * f + home.z;
        self.actor.world_pos.y = self.water_surface + 20.0;
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.timer = 50;
        }
        self.actor.world_pos.y += 2.0 * (self.timer as f32 * (PI / 25.0)).sin();
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
    /// `BgYdanHasi_Update` for the floating block, and the new transform for
    /// `DynaPoly_UpdateContext` (which runs after the BG category).
    fn update(&mut self, play: &mut PlayState) {
        self.update_floating_block(play.gameplay_frames);
        play.col.dyna.set_source(self.bg, self.source());
    }
    /// `BgYdanHasi_Draw`: `Gfx_DrawDListOpa(gDTSlidingPlatformDL)` under `Actor_Draw`'s matrix
    /// (the same translate/rotate/scale DynaPoly uses).
    fn draw(&self, st: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let t = ScaleRotPos { scale: st.scale, rot: st.rot, pos: st.pos };
        out.opa.push(DrawCmd::new(MeshKey::named(oot_game::pack::keys::mesh(OBJECT, DISPLAY_LIST)), srt_matrix(&t)));
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
