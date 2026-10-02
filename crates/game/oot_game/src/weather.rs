//! The weather's draw (`z_kankyo.c`): `Environment_DrawRain`'s drops and the rings they leave
//! on the ground, and `Environment_DrawLightning`'s bolts. Their state is `crate::env`'s; this
//! is what `Play_Draw` makes of it, frame by frame.
//!
//! The rain is drawn where `Play_Draw` draws it (after the rooms and the skybox, before the
//! actors) and makes its `Rand_ZeroOne` calls there: four per drop, two per ring. The matrices
//! are kept on the play state (`rain`) and drawn from bakes (`gRaindropDL` after `SETUPDL_20`,
//! `gEffShockwaveDL` after `Gfx_SetupDL_25Xlu`, `gEffLightningDL` after `Gfx_SetupDL_61Xlu`
//! with each of the eight bolt textures on segment 8).
//!
//! Not drawn: the lightning's flash (`Environment_DrawLightningFlash`: a fill at the start of
//! `POLY_OPA_DISP`, under the rooms and the skybox, so only the background would show it; its
//! ambient light is in `crate::env`), the snow, the sandstorm.

use eng_gfx::{DrawCmd, MeshKey};
use eng_math::atan2_s;
use glam::{Mat4, Vec3};

use crate::env::{LightningBolt, PRECIP_RAIN_CUR, PRECIP_SNOW_CUR};
use crate::gbi::setup_dl;
use crate::pack::{BakeBody, BakeSegment, MeshBake, keys};
use crate::play::{DrawOut, PlayState, ViewInfo};
use crate::sys_matrix::binang_to_rad;

const RAINDROP_BAKE: &str = "Environment/raindrop";
const RING_BAKE: &str = "Environment/rain_ring";
fn lightning_bake(texture: usize) -> String {
    format!("Environment/lightning_{texture}")
}

/// `lightningTextures`.
const LIGHTNING_TEXTURES: [&str; 8] =
    ["gEffLightning1Tex", "gEffLightning2Tex", "gEffLightning3Tex", "gEffLightning4Tex", "gEffLightning5Tex", "gEffLightning6Tex", "gEffLightning7Tex", "gEffLightning8Tex"];

const SEG_TEX: u8 = 0x08;
const SEG_SETUP: u8 = 0x0D;

/// The weather's meshes.
pub fn bakes() -> Vec<MeshBake> {
    let keep = "gameplay_keep";
    let mut setup_20 = setup_dl::setup_dl_20();
    // gDPSetPrimColor(0, 0, 150, 255, 255, 30) before Gfx_SetupDL(SETUPDL_20).
    setup_20.0.insert(0, (0xFA00_0000, 0x96FF_FF1E));
    setup_20.end();
    let mut v = vec![
        MeshBake {
            name: RAINDROP_BAKE.into(),
            object: keep.into(),
            segments: vec![(SEG_SETUP, BakeSegment::Commands(setup_20.0))],
            prelude: vec![SEG_SETUP],
            body: BakeBody::DLists(vec![(keep.into(), "gRaindropDL".into())]),
        },
        MeshBake {
            name: RING_BAKE.into(),
            object: keep.into(),
            // Gfx_SetupDL_25Xlu (the bake's start), gDPSetEnvColor(155, 155, 155, 0),
            // gDPSetPrimColor(0, 0, 255, 255, 255, 120).
            segments: vec![(SEG_SETUP, BakeSegment::Commands(vec![(0xFB00_0000, 0x9B9B_9B00), (0xFA00_0000, 0xFFFF_FF78)]))],
            prelude: vec![SEG_SETUP],
            body: BakeBody::DLists(vec![(keep.into(), "gEffShockwaveDL".into())]),
        },
    ];
    for (i, t) in LIGHTNING_TEXTURES.iter().enumerate() {
        // gDPSetPrimColor(0, 0, 255, 255, 255, 128), gDPSetEnvColor(0, 255, 255, 128), the
        // texture on segment 8, Gfx_SetupDL_61Xlu.
        let mut s = setup_dl::setup_dl_61();
        s.0.insert(0, (0xFB00_0000, 0x00FF_FF80));
        s.0.insert(0, (0xFA00_0000, 0xFFFF_FF80));
        s.end();
        v.push(MeshBake {
            name: lightning_bake(i),
            object: keep.into(),
            segments: vec![(SEG_TEX, BakeSegment::Texture { file: keep.into(), symbol: (*t).into() }), (SEG_SETUP, BakeSegment::Commands(s.0))],
            prelude: vec![SEG_SETUP],
            body: BakeBody::DLists(vec![(keep.into(), "gEffLightningDL".into())]),
        });
    }
    v
}

/// `Environment_DrawRain`'s matrices for one frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RainDraw {
    pub drops: Vec<Mat4>,
    pub rings: Vec<Mat4>,
}

/// `Math_Atan2F(x, y)`: `Math_Atan2S` in radians.
fn math_atan2f(x: f32, y: f32) -> f32 {
    binang_to_rad(atan2_s(x, y))
}

impl PlayState {
    /// `Environment_DrawRain` (with `precipitation[PRECIP_RAIN_CUR]` drops): each drop 50 ahead
    /// of the eye, scattered by 100, leaning into the wind; rings on the ground around 280 ahead
    /// while Player is below the eye. The main camera's `stateFlags & 0x100` or snow skips it.
    pub fn environment_draw_rain(&mut self) -> RainDraw {
        let mut out = RainDraw::default();
        let n = self.env_ctx.precipitation[PRECIP_RAIN_CUR];
        if n == 0 || self.game_camera.state_flags & 0x100 != 0 || self.env_ctx.precipitation[PRECIP_SNOW_CUR] != 0 {
            return out;
        }
        let (eye, at) = (self.view.eye, self.view.at);
        let v = at - eye;
        let length = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
        let (t1, t2, t3) = (v.x / length, v.y / length, v.z / length);
        let p50 = Vec3::new(eye.x + t1 * 50.0, eye.y + t2 * 50.0, eye.z + t3 * 50.0);
        let (x280, z280) = (eye.x + t1 * 280.0, eye.z + t3 * 280.0);
        let wind = self.env_ctx.wind_direction;
        for _ in 0..n {
            let r2 = self.rand.zero_one();
            let r1 = self.rand.zero_one();
            let r3 = self.rand.zero_one();
            let t = Mat4::from_translation(Vec3::new((r2 - 0.7) * 100.0 + p50.x, (r1 - 0.7) * 100.0 + p50.y, (r3 - 0.7) * 100.0 + p50.z));
            let w = Vec3::new(wind[0] as f32, wind[1] as f32 + 500.0 + self.rand.zero_one() * 200.0, wind[2] as f32);
            let len = (w.x * w.x + w.z * w.z).sqrt();
            let rot_x = math_atan2f(len, -w.y);
            let rot_y = math_atan2f(w.z, w.x);
            out.drops.push(t * Mat4::from_rotation_y(-rot_y) * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2 - rot_x) * Mat4::from_scale(Vec3::new(0.4, 1.2, 0.4)));
        }
        let Some(ppos) = self.player.and_then(|h| self.actors.actor(h)).map(|a| a.world_pos) else { return out };
        if ppos.y < eye.y {
            let adult = self.save.adult;
            for _ in 0..n {
                // Environment_RandCentered, x then z.
                let x = (self.rand.zero_one() - 0.5) * 280.0 + x280;
                let z = (self.rand.zero_one() - 0.5) * 280.0 + z280;
                let t = Mat4::from_translation(Vec3::new(x, ppos.y + 2.0, z));
                let d = ppos.y + 2.0 - eye.y;
                let s = if (adult && d > -48.0) || (!adult && d > -30.0) { 0.02 } else { 0.1 };
                out.rings.push(t * Mat4::from_scale(Vec3::splat(s)));
            }
        }
        out
    }
}

/// The rain's and the bolts' draws into the XLU list (`gSPMatrix(&D_01000000)` puts the bolts
/// on the billboard: `view.billboard` last).
pub fn draw(rain: &RainDraw, bolts: &[LightningBolt], view: &ViewInfo, out: &mut DrawOut) {
    for m in &rain.drops {
        out.xlu.push(DrawCmd::new(MeshKey::named(keys::bake(RAINDROP_BAKE)), *m));
    }
    for m in &rain.rings {
        out.xlu.push(DrawCmd::new(MeshKey::named(keys::bake(RING_BAKE)), *m));
    }
    for b in bolts {
        let deg = |d: i8| d as f32 * (std::f32::consts::PI / 180.0);
        let m = Mat4::from_translation(b.pos + b.offset) * Mat4::from_rotation_x(deg(b.pitch)) * Mat4::from_rotation_z(deg(b.roll)) * Mat4::from_scale(Vec3::new(22.0, 100.0, 22.0)) * view.billboard;
        out.xlu.push(DrawCmd::new(MeshKey::named(keys::bake(&lightning_bake(b.texture_index.min(7) as usize))), m));
    }
}
