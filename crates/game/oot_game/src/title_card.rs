//! The title cards (`z_actor.c`'s `TitleCardContext`, `play->actorCtx.titleCtx`): a scene's
//! place name, faded in on entering it (`Player_Init`) or when a cutscene asks
//! (`CS_MISC` 15), held, and faded out.
//!
//! A scene's place name is its title file (`scene_table.h`'s second column, `g_pn_06` for the
//! Deku Tree), which `TitleCard_InitPlaceName` loads whole: an IA8 texture per language, 144 by
//! 24. The importer bakes each scene's as a sprite (`bakes`), the first language's
//! (`gSaveContext.language` is 0, English); `TitleCard_Draw` draws it centred on (x, y) with the
//! intensity and alpha as its primitive colour.
//!
//! A boss's name (`TitleCard_InitBossName`, Queen Gohma's `gGohmaTitleCardTex`) is a texture in
//! the boss's object, 128 by 40, more than 0x1000 bytes: `TitleCard_Draw` draws it in two blocks,
//! 32 rows then the last 8, each a sprite of its own (`boss_bakes`).

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

/// The boss names `TitleCard_InitBossName` shows: the object, the texture's offset in it, its
/// width and height (`BossGoma_Encounter`'s call: `gGohmaTitleCardTex`, 128 by 40; on PAL the
/// texture holds the three languages one after another, English first).
pub const BOSS_NAMES: &[(&str, u32, u8, u8)] = &[("object_goma", 0x19BA8, 128, 40)];

/// The sprite of a boss name's block starting `offset` bytes into its texture (0, then 0x1000
/// for the rest of one bigger than that).
pub fn boss_sprite_name(object: &str, tex_offset: u32, offset: u32) -> String {
    format!("title/boss/{object}/{tex_offset:X}+{offset:X}")
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

    /// `TitleCard_InitBossName`: a boss's name (`BOSS_NAMES`' `object` and texture `offset`) at
    /// (x, y), shown for 80 frames at once.
    #[allow(clippy::too_many_arguments)]
    pub fn init_boss_name(&mut self, object: &str, offset: u32, x: i16, y: i16, width: u8, height: u8) {
        self.texture = Some(boss_sprite_name(object, offset, 0));
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
        self.duration_timer = 80;
        self.delay_timer = 0;
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
    /// colour the intensity, grey, and the alpha. A texture bigger than 0x1000 bytes (a boss's
    /// name) is drawn in two blocks: `0x1000 / width` rows, then the rest below them from 0x1000
    /// on (the place names are 144 by 24, one block).
    pub fn draw(&self, out: &mut Vec<Sprite>) {
        if self.alpha == 0 {
            return;
        }
        let Some(tex) = &self.texture else { return };
        let (w, mut h) = (self.width as i32, self.height as i32);
        let x0 = self.x as i32 * 4 - w * 2;
        let y0 = self.y as i32 * 4 - h * 2;
        let x1 = w * 4 + x0 - 4;
        if w * h > 0x1000 {
            h = 0x1000 / w;
        }
        let y2 = y0 + h * 4;
        let i = self.intensity as u8;
        let color = Some([i, i, i, self.alpha as u8]);
        out.push(Sprite::rect(tex.clone(), x0 as f32 / 4.0, y0 as f32 / 4.0, x1 as f32 / 4.0, (y2 - 1) as f32 / 4.0, color, None));
        let rest = self.height as i32 - h;
        if rest > 0 {
            // The second block's sprite: the name's, 0x1000 bytes on.
            let second = tex.strip_suffix("+0").map(|base| format!("{base}+1000")).unwrap_or_else(|| tex.clone());
            out.push(Sprite::rect(second, x0 as f32 / 4.0, y2 as f32 / 4.0, x1 as f32 / 4.0, (y2 + rest * 4 - 1) as f32 / 4.0, color, None));
        }
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

/// The boss names to bake (`BOSS_NAMES`): each a block of `0x1000 / width` rows, then one of the
/// rest, English (the texture's first language), as `TitleCard_Draw` loads them.
pub fn boss_bakes() -> Vec<SpriteBake> {
    let mut v = Vec::new();
    for &(object, offset, width, height) in BOSS_NAMES {
        let (w, h) = (width as u32, height as u32);
        let first = if w * h > 0x1000 { 0x1000 / w } else { h };
        for (block, rows) in [(0u32, first), (0x1000, h - first)] {
            if rows == 0 {
                continue;
            }
            v.push(SpriteBake {
                name: boss_sprite_name(object, offset, block),
                tex: TexSrc::File { file: object.to_string(), offset: offset + block },
                load: Load::new(G_IM_FMT_IA, G_IM_SIZ_8B, w, rows, G_TX_WRAP),
                setup: setup_dl_52(),
                prim: true,
                env: false,
                quad: Quad::Rect { s: w, t: rows },
            });
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_boss_name_is_drawn_in_two_blocks() {
        // BossGoma_Encounter: TitleCard_InitBossName(gGohmaTitleCardTex, 160, 180, 128, 40): shown
        // for 80 frames at once.
        let mut t = TitleCardContext::default();
        let (object, offset, w, h) = BOSS_NAMES[0];
        t.init_boss_name(object, offset, 160, 180, w, h);
        assert_eq!((t.duration_timer, t.delay_timer), (80, 0));
        t.update();
        assert_eq!((t.alpha, t.intensity), (10, 20));
        let mut out = Vec::new();
        t.draw(&mut out);
        // TitleCard_Draw: 128 * 40 > 0x1000, so 0x1000 / 128 = 32 rows from (96, 160), then the
        // other 8 from 0x1000 on, below them: x 160 * 4 - 128 * 2 = 384 (96), to 384 + 512 - 4.
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, boss_sprite_name(object, offset, 0));
        assert_eq!(out[1].name, boss_sprite_name(object, offset, 0x1000));
        assert_eq!(out[0].transform, crate::sprite::rect_transform(96.0, 160.0, 223.0, 191.75));
        assert_eq!(out[1].transform, crate::sprite::rect_transform(96.0, 192.0, 223.0, 199.75));
        // The sprites to bake: the two blocks, IA8, 128 by 32 and 128 by 8.
        let b = boss_bakes();
        assert_eq!(b.len(), 2);
        assert_eq!((b[0].name.as_str(), b[1].name.as_str()), (out[0].name.as_str(), out[1].name.as_str()));
    }
}
