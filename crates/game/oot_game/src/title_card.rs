//! The title cards (`z_actor.c`'s `TitleCardContext`, `play->actorCtx.titleCtx`): a scene's
//! place name, faded in on entering it (`Player_Init`) or when a cutscene asks
//! (`CS_MISC` 15), held, and faded out.
//!
//! A scene's place name is its title file (`scene_table.h`'s second column, `g_pn_06` for the
//! Deku Tree), which `TitleCard_InitPlaceName` loads whole: an IA8 texture per language, 144 by
//! 24. The importer bakes each scene's as a sprite (`bakes`), the first language's
//! (`gSaveContext.language` is 0, English); `TitleCard_Draw` draws it centred on (x, y) with the
//! intensity and alpha as its primitive colour.

use crate::gbi::{Dl, setup_dl};
use crate::sprite::{Load, Quad, Sprite, SpriteBake, TexSrc};

/// `G_IM_FMT_IA`, `G_IM_SIZ_8b`, `G_TX_WRAP` (`gbi.h`).
const G_IM_FMT_IA: u32 = 3;
const G_IM_SIZ_8B: u32 = 1;
const G_TX_WRAP: u32 = 0;

/// The place name's size (`TitleCard_InitPlaceName`'s callers: 144 by 24).
pub const PLACE_NAME_WIDTH: u32 = 144;
pub const PLACE_NAME_HEIGHT: u32 = 24;

/// A scene's place name sprite.
pub fn sprite_name(title_file: &str) -> String {
    format!("title/{title_file}")
}

/// `TitleCardContext`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TitleCardContext {
    /// `texture`: the place name's sprite (`None` before any, or for a scene without a title
    /// file, whose load the C skips, leaving whatever the segment held).
    pub texture: Option<String>,
    pub x: i16,
    pub y: i16,
    pub width: u8,
    pub height: u8,
    pub duration_timer: u8,
    pub delay_timer: u8,
    pub alpha: i16,
    pub intensity: i16,
}

/// `DECR`: 0 stays 0, else one less (the new value).
fn decr(v: &mut u8) -> u8 {
    if *v != 0 {
        *v -= 1;
    }
    *v
}

/// `Math_StepToS`.
fn step_to_s(v: &mut i16, target: i16, step: i16) {
    if *v < target {
        *v = (*v + step).min(target);
    } else if *v > target {
        *v = (*v - step).max(target);
    }
}

impl TitleCardContext {
    /// `TitleCard_Init` (`Actor_InitContext`).
    pub fn init(&mut self) {
        self.duration_timer = 0;
        self.delay_timer = 0;
        self.intensity = 0;
        self.alpha = 0;
    }

    /// `TitleCard_InitPlaceName`: the scene's place name (`title_file`, empty for none) at (x, y),
    /// shown for 80 frames after `delay`.
    #[allow(clippy::too_many_arguments)]
    pub fn init_place_name(&mut self, title_file: &str, x: i16, y: i16, width: u8, height: u8, delay: u8) {
        self.texture = (!title_file.is_empty()).then(|| sprite_name(title_file));
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
        self.duration_timer = 80;
        self.delay_timer = delay;
    }

    /// `TitleCard_Update` (the end of `Actor_UpdateAll`): after the delay, in by 10 (the
    /// intensity by 20) while the duration lasts, then out by 30 (70).
    pub fn update(&mut self) {
        if decr(&mut self.delay_timer) == 0 {
            if decr(&mut self.duration_timer) == 0 {
                step_to_s(&mut self.alpha, 0, 30);
                step_to_s(&mut self.intensity, 0, 70);
            } else {
                step_to_s(&mut self.alpha, 255, 10);
                step_to_s(&mut self.intensity, 255, 20);
            }
        }
    }

    /// `TitleCard_Clear`: true when nothing's shown or pending; else it's cut short.
    pub fn clear(&mut self) -> bool {
        if self.delay_timer != 0 || self.alpha != 0 {
            self.duration_timer = 0;
            self.delay_timer = 0;
            return false;
        }
        true
    }

    /// `TitleCard_Draw`: the texture rectangle centred on (x, y), `gSPTextureRectangle(x * 4 -
    /// width * 2, y * 4 - height * 2, .. + width * 4 - 4, .. + height * 4 - 1)`, its primitive
    /// colour the intensity, grey, and the alpha. (A texture taller than 0x1000 bytes is drawn in
    /// two blocks; the place names are 144 by 24, one.)
    pub fn draw(&self, out: &mut Vec<Sprite>) {
        if self.alpha == 0 {
            return;
        }
        let Some(tex) = &self.texture else { return };
        let (w, h) = (self.width as i32, self.height as i32);
        let x0 = self.x as i32 * 4 - w * 2;
        let y0 = self.y as i32 * 4 - h * 2;
        let x1 = w * 4 + x0 - 4;
        let y1 = y0 + h * 4 - 1;
        let i = self.intensity as u8;
        out.push(Sprite::rect(tex.clone(), x0 as f32 / 4.0, y0 as f32 / 4.0, x1 as f32 / 4.0, y1 as f32 / 4.0, Some([i, i, i, self.alpha as u8]), None));
    }
}

/// `SETUPDL_52` (`z_rcp.c`): `SETUPDL_39`'s texture, combiner (`G_CC_MODULATEIA_PRIM`) and other
/// modes, with `gsSPLoadGeometryMode(G_CULL_BACK)`; `Gfx_SetupDL_52NoCD` then sets
/// `G_CD_DISABLE` (the dither, which the renderer doesn't model). The card is a texture
/// rectangle, which the geometry mode doesn't reach (no culling), but the bake draws it as two
/// triangles: their geometry mode is left empty so the cull doesn't drop them.
fn setup_dl_52() -> Dl {
    let mut d = setup_dl::setup_dl_39();
    // The last command is SETUPDL_39's geometry mode.
    d.0.pop();
    d.0.push((0xD900_0000, 0));
    d
}

/// The place names to bake: each title file's first language, an IA8 144 by 24 texture
/// (`TitleCard_Draw`'s `gDPLoadTextureBlock(.., G_IM_FMT_IA, G_IM_SIZ_8b, width, height, 0,
/// G_TX_NOMIRROR | G_TX_WRAP, ..)` after `Gfx_SetupDL_52NoCD`).
pub fn bakes<'a>(title_files: impl IntoIterator<Item = &'a str>) -> Vec<SpriteBake> {
    let mut v: Vec<SpriteBake> = Vec::new();
    for f in title_files {
        if f.is_empty() || v.iter().any(|b| b.name == sprite_name(f)) {
            continue;
        }
        v.push(SpriteBake {
            name: sprite_name(f),
            tex: TexSrc::File { file: f.to_string(), offset: 0 },
            load: Load::new(G_IM_FMT_IA, G_IM_SIZ_8B, PLACE_NAME_WIDTH, PLACE_NAME_HEIGHT, G_TX_WRAP),
            setup: setup_dl_52(),
            prim: true,
            env: false,
            quad: Quad::Rect { s: PLACE_NAME_WIDTH, t: PLACE_NAME_HEIGHT },
        });
    }
    v
}
