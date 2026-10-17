//! `Effect_Ss_G_Magma2` (`ovl_Effect_Ss_G_Magma2/z_eff_ss_g_magma2.c`): a splash of lava (or a
//! puff of dark smoke) on King Dodongo's floor: `object_kingdodongo`'s `DL_025A90` through ten
//! textures in turn, as fast as its update rate says, facing the camera (the list loads the
//! billboard from segment 1). Draw mode 0 is
//! the lava (`SETUPDL_61`, its prim green and env red fading 26 a texture), any other mode the
//! smoke (`SETUPDL_0`, fogged). Not spawned without `OBJECT_KINGDODONGO` loaded.

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SEG_TEX, SsDraw, SsSpawn, SsUpdate, colored};
use crate::gbi::setup_dl;
use crate::pack::{BakeBody, BakeSegment, MeshBake};

// The overlay's regs.
const R_PRIM_COLOR_R: usize = 0;
const R_PRIM_COLOR_G: usize = 1;
const R_PRIM_COLOR_A: usize = 2;
const R_ENV_COLOR_R: usize = 3;
const R_ENV_COLOR_G: usize = 4;
const R_ENV_COLOR_A: usize = 5;
const R_TEX_INDEX: usize = 6;
const R_TIMER: usize = 7;
const R_UPDATE_RATE: usize = 8;
const R_DRAW_MODE: usize = 9;
const R_OBJECT_SLOT: usize = 10;
const R_SCALE: usize = 11;

/// `OBJECT_KINGDODONGO` (`object_table.h`).
pub const OBJECT_KINGDODONGO: i16 = 0x0019;
const OBJECT: &str = "object_kingdodongo";
const DLIST: &str = "object_kingdodongo_DL_025A90";

/// `sTextures`: ten textures, the last repeated (the index never reaches past 9 drawn).
const TEXTURES: [&str; 13] = [
    "object_kingdodongo_Tex_02E4E0",
    "object_kingdodongo_Tex_02E8E0",
    "object_kingdodongo_Tex_02ECE0",
    "object_kingdodongo_Tex_02F0E0",
    "object_kingdodongo_Tex_02F4E0",
    "object_kingdodongo_Tex_02F8E0",
    "object_kingdodongo_Tex_02FCE0",
    "object_kingdodongo_Tex_0300E0",
    "object_kingdodongo_Tex_0304E0",
    "object_kingdodongo_Tex_0308E0",
    "object_kingdodongo_Tex_0308E0",
    "object_kingdodongo_Tex_0308E0",
    "object_kingdodongo_Tex_0308E0",
];

/// `EffectSsGMagma2InitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct GMagma2Init {
    pub pos: Vec3,
    pub prim_color: [u8; 4],
    pub env_color: [u8; 4],
    pub update_rate: i16,
    pub draw_mode: i16,
    pub scale: i16,
}

/// `EffectSsGMagma2_Init`: with `OBJECT_KINGDODONGO` loaded, still at `pos` for up to 100 frames,
/// the colours' red, green and alpha (blue is always 0); else nothing spawns.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &GMagma2Init) -> bool {
    let slot = s.objects.and_then(|o| o.get_index(OBJECT_KINGDODONGO).filter(|&b| o.is_loaded(b)));
    let Some(slot) = slot else { return false };
    this.regs[R_OBJECT_SLOT] = slot as i16;
    this.pos = p.pos;
    this.velocity = Vec3::ZERO;
    this.accel = Vec3::ZERO;
    this.life = 100;
    this.draw = Some(SsDraw::GMagma2);
    this.update = Some(SsUpdate::GMagma2);
    this.gfx = Some((OBJECT, DLIST));
    let r = &mut this.regs;
    r[R_TEX_INDEX] = 0;
    r[R_DRAW_MODE] = p.draw_mode;
    r[R_UPDATE_RATE] = p.update_rate;
    r[R_SCALE] = p.scale;
    r[R_PRIM_COLOR_R] = p.prim_color[0] as i16;
    r[R_PRIM_COLOR_G] = p.prim_color[1] as i16;
    r[R_PRIM_COLOR_A] = p.prim_color[3] as i16;
    r[R_ENV_COLOR_R] = p.env_color[0] as i16;
    r[R_ENV_COLOR_G] = p.env_color[1] as i16;
    r[R_ENV_COLOR_A] = p.env_color[3] as i16;
    true
}

/// `EffectSsGMagma2_Update`: the timer up by the rate; each 10 the next texture (the tenth ends
/// it), and in draw mode 0 the prim's green and the env's red down 26 (not below 0).
pub fn update(this: &mut EffectSs) {
    let r = &mut this.regs;
    r[R_TIMER] = r[R_TIMER].wrapping_add(r[R_UPDATE_RATE]);
    if r[R_TIMER] >= 10 {
        r[R_TIMER] -= 10;
        r[R_TEX_INDEX] += 1;
        if r[R_TEX_INDEX] >= 10 {
            this.life = 0;
        }
        if r[R_DRAW_MODE] == 0 {
            r[R_PRIM_COLOR_G] -= 26;
            if r[R_PRIM_COLOR_G] <= 0 {
                r[R_PRIM_COLOR_G] = 0;
            }
            r[R_ENV_COLOR_R] -= 26;
            if r[R_ENV_COLOR_R] <= 0 {
                r[R_ENV_COLOR_R] = 0;
            }
        }
    }
}

fn bake_name(mode: usize, tex: usize) -> String {
    format!("EffectSs/g_magma2/{mode}/{tex}")
}

/// A bake per draw mode (0: `SETUPDL_61`; else `SETUPDL_0`) and texture (`sTextures`' ten), the
/// colours dynamic, `DL_025A90`.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for mode in 0..2 {
        let mut setup = if mode == 0 { setup_dl::setup_dl_61() } else { setup_dl::setup_dl_0() };
        setup.end();
        for (i, tex) in TEXTURES.iter().take(10).enumerate() {
            v.push(MeshBake {
                name: bake_name(mode, i),
                object: OBJECT.into(),
                segments: vec![
                    (SEG_TEX, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() }),
                    (SEG_SETUP, BakeSegment::Commands(setup.0.clone())),
                    (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
                    // gSPMatrix(0x01000000): the billboard, multiplied in by the draw.
                    (0x01, BakeSegment::Bytes(super::identity_mtx())),
                ],
                prelude: vec![SEG_SETUP, SEG_COLOR],
                body: BakeBody::DLists(vec![(OBJECT.into(), DLIST.into())]),
            });
        }
    }
    v
}

/// `EffectSsGMagma2_Draw`: at its position scaled `rScale / 100`, then the list's billboard; the
/// texture `rTexIndex`, prim (red, green, 0, alpha) and env (red, green, 0, alpha).
pub fn draw(this: &EffectSs, billboard: Mat4, out: &mut EffectDraws) {
    let r = &this.regs;
    let scale = r[R_SCALE] as f32 / 100.0;
    let m = Mat4::from_translation(this.pos) * Mat4::from_scale(Vec3::splat(scale)) * billboard;
    let mode = (r[R_DRAW_MODE] != 0) as usize;
    let tex = TEXTURES.get(r[R_TEX_INDEX].max(0) as usize).map_or(9, |t| TEXTURES.iter().position(|x| x == t).unwrap_or(9));
    let prim = [r[R_PRIM_COLOR_R] as u8, r[R_PRIM_COLOR_G] as u8, 0, r[R_PRIM_COLOR_A] as u8];
    let env = [r[R_ENV_COLOR_R] as u8, r[R_ENV_COLOR_G] as u8, 0, r[R_ENV_COLOR_A] as u8];
    out.xlu.push(colored(&bake_name(mode, tex), m, SEG_COLOR, prim, env));
}
