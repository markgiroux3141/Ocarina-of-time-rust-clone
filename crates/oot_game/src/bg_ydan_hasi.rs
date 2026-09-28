//! `Bg_Ydan_Hasi` with params `HASI_WATER_BLOCK` (0): the Deku Tree B1 platform that floats
//! on the water, sliding back and forth along its facing and bobbing, ported from
//! `z_bg_ydan_hasi.c` (`BgYdanHasi_Init`, `BgYdanHasi_UpdateFloatingBlock`). Its collision is
//! `gDTSlidingPlatformCol` and its display list `gDTSlidingPlatformDL`, both in
//! `object_ydan_objects`, read from the ROM at the offsets the decomp's XML gives.
//!
//! The game reads the water height from the scene's `waterBoxes[1]`; here the caller passes
//! the surface the platform floats on.

use std::f32::consts::PI;
use std::sync::Arc;

use anyhow::{Context, Result};
use glam::Vec3;
use oot_core::collision::CollisionHeader;
use oot_core::project::Project;

use crate::dyna::{BgActorSource, DPM_PLAYER, Dyna};
use crate::math::{cos_s, sin_s};

pub const OBJECT: &str = "object_ydan_objects";
pub const COLLISION: &str = "gDTSlidingPlatformCol";
pub const DISPLAY_LIST: &str = "gDTSlidingPlatformDL";

/// `gDTSlidingPlatformCol`, from `object_ydan_objects` (segment 6).
pub fn load_collision(p: &Project) -> Result<Arc<CollisionHeader>> {
    let sym = p.symbols.file(OBJECT).with_context(|| format!("{OBJECT}.xml"))?.find(COLLISION).with_context(|| COLLISION.to_string())?;
    let file = p.rom.file_by_name(OBJECT)?;
    Ok(Arc::new(CollisionHeader::parse(&file, 6, sym.offset as usize)?))
}

#[derive(Debug, Clone)]
pub struct BgYdanHasi {
    pub home: Vec3,
    pub pos: Vec3,
    /// `world.rot` = `shape.rot` (from the spawn).
    pub rot: [i16; 3],
    pub scale: Vec3,
    pub timer: i16,
    pub water_surface: f32,
    pub bg: u16,
}

impl BgYdanHasi {
    /// `BgYdanHasi_Init`: `ICHAIN_VEC3F_DIV1000(scale, 100)`, then x/z scale 0.15, the
    /// position 20 above the water, `DynaPolyActor_Init(DPM_PLAYER)`, `DynaPoly_SetBgActor`.
    pub fn spawn(dyna: &mut Dyna, header: Arc<CollisionHeader>, home: Vec3, yaw: i16, water_surface: f32) -> BgYdanHasi {
        let mut p = BgYdanHasi {
            home,
            pos: Vec3::new(home.x, water_surface + 20.0, home.z),
            rot: [0, yaw, 0],
            scale: Vec3::new(0.15, 0.1, 0.15),
            timer: 0,
            water_surface,
            bg: 0,
        };
        p.bg = dyna.set_bg_actor(header, p.source(), DPM_PLAYER);
        p
    }

    pub fn source(&self) -> BgActorSource {
        BgActorSource { pos: self.pos, shape_rot: self.rot, scale: self.scale, shape_y_offset: 0.0 }
    }

    /// `BgYdanHasi_UpdateFloatingBlock`: ±165 along the facing over 256 frames, and ±2 of
    /// bobbing over the 50-frame timer.
    pub fn update(&mut self, gameplay_frames: u32) {
        let f = ((gameplay_frames & 0xFF) as f32 * (PI / 128.0)).sin() * 165.0;
        self.pos.x = sin_s(self.rot[1]) * f + self.home.x;
        self.pos.z = cos_s(self.rot[1]) * f + self.home.z;
        self.pos.y = self.water_surface + 20.0;
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.timer = 50;
        }
        self.pos.y += 2.0 * (self.timer as f32 * (PI / 25.0)).sin();
    }
}
