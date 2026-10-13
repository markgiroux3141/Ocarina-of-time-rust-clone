//! Scene transitions: the trigger and mode `Play_Update` steps through (`z_play.c`,
//! `z64.h`), and the fade (`z_fbdemo_fade.c`). `PlayState::update_transition` runs them.
//!
//! Ported: the fades (`TransitionFade`, every `TRANS_TYPE_FADE_*`), the circles
//! (`TransitionCircle`, `z_fbdemo_circle.c`: every `TRANS_TYPE_CIRCLE`), the white and brown fills
//! and the instant cut. The wipe and triforce transitions (`z_fbdemo_wipe1.c`, `_triforce.c`) are
//! drawn as a black fade of the same kind, and take the fade's time (`transFadeDuration`, 60)
//! rather than their own; the sandstorm and the cutscene fills end at once.

/// `TRANS_TRIGGER_*`.
pub const TRANS_TRIGGER_OFF: i8 = 0;
/// Leaving: the scene ends when the transition covers the screen.
pub const TRANS_TRIGGER_START: i8 = 20;
/// Arriving: `Play_Init` sets it, and the transition uncovers the new scene.
pub const TRANS_TRIGGER_END: i8 = -20;

/// `TransitionMode`.
pub const TRANS_MODE_OFF: u8 = 0;
pub const TRANS_MODE_SETUP: u8 = 1;
pub const TRANS_MODE_INSTANCE_INIT: u8 = 2;
pub const TRANS_MODE_INSTANCE_RUNNING: u8 = 3;
pub const TRANS_MODE_FILL_WHITE_INIT: u8 = 4;
pub const TRANS_MODE_FILL_IN: u8 = 5;
pub const TRANS_MODE_FILL_OUT: u8 = 6;
pub const TRANS_MODE_FILL_BROWN_INIT: u8 = 7;
pub const TRANS_MODE_INSTANT: u8 = 10;
pub const TRANS_MODE_INSTANCE_WAIT: u8 = 11;
pub const TRANS_MODE_SANDSTORM_INIT: u8 = 12;
pub const TRANS_MODE_SANDSTORM_END_INIT: u8 = 14;
pub const TRANS_MODE_CS_BLACK_FILL_INIT: u8 = 16;
pub const TRANS_MODE_CS_BLACK_FILL: u8 = 17;

/// `TransitionType`.
pub const TRANS_TYPE_WIPE: u8 = 0;
pub const TRANS_TYPE_TRIFORCE: u8 = 1;
pub const TRANS_TYPE_FADE_BLACK: u8 = 2;
pub const TRANS_TYPE_FADE_WHITE: u8 = 3;
pub const TRANS_TYPE_FADE_BLACK_FAST: u8 = 4;
pub const TRANS_TYPE_FADE_WHITE_FAST: u8 = 5;
pub const TRANS_TYPE_FADE_BLACK_SLOW: u8 = 6;
pub const TRANS_TYPE_FADE_WHITE_SLOW: u8 = 7;
pub const TRANS_TYPE_WIPE_FAST: u8 = 8;
pub const TRANS_TYPE_FILL_WHITE2: u8 = 9;
pub const TRANS_TYPE_FILL_WHITE: u8 = 10;
pub const TRANS_TYPE_INSTANT: u8 = 11;
pub const TRANS_TYPE_FILL_BROWN: u8 = 12;
pub const TRANS_TYPE_FADE_WHITE_CS_DELAYED: u8 = 13;
pub const TRANS_TYPE_SANDSTORM_PERSIST: u8 = 14;
pub const TRANS_TYPE_SANDSTORM_END: u8 = 15;
pub const TRANS_TYPE_CS_BLACK_FILL: u8 = 16;
pub const TRANS_TYPE_FADE_WHITE_INSTANT: u8 = 17;
pub const TRANS_TYPE_FADE_GREEN: u8 = 18;
pub const TRANS_TYPE_FADE_BLUE: u8 = 19;
pub const TRANS_TYPE_MAX: u8 = 56;

/// `TransitionCircleAppearance` (`transition_circle.h`).
pub const TCA_NORMAL: u8 = 0;
pub const TCA_WAVE: u8 = 1;
pub const TCA_RIPPLE: u8 = 2;
pub const TCA_STARBURST: u8 = 3;
/// `TransitionCircleColor`.
pub const TCC_BLACK: u8 = 0;
pub const TCC_WHITE: u8 = 1;
pub const TCC_GRAY: u8 = 2;
pub const TCC_SPECIAL: u8 = 3;
/// `TransitionCircleSpeed`.
pub const TCS_FAST: u8 = 0;
pub const TCS_SLOW: u8 = 1;

/// `TRANS_TYPE_CIRCLE(appearance, color, speed)` (`transition.h`): `(1 << 5) | ((color & 3) << 3) |
/// ((appearance & 3) << 1) | (speed & 1)`.
pub const fn trans_type_circle(appearance: u8, color: u8, speed: u8) -> u8 {
    (1 << 5) | ((color & 3) << 3) | ((appearance & 3) << 1) | (speed & 1)
}

/// Is `t` one of the "instance" transitions (`Play_SetupTransition` gives it a
/// `TransitionContext`), rather than a fill or cut `Play_Update` runs itself?
pub fn is_instance_type(t: u8) -> bool {
    t >> 5 == 1 || matches!(t, TRANS_TYPE_WIPE | TRANS_TYPE_TRIFORCE | TRANS_TYPE_WIPE_FAST) || is_fade_type(t)
}

/// The types `Play_SetupTransition` gives `TransitionFade`.
pub fn is_fade_type(t: u8) -> bool {
    matches!(
        t,
        TRANS_TYPE_FADE_BLACK
            | TRANS_TYPE_FADE_WHITE
            | TRANS_TYPE_FADE_BLACK_FAST
            | TRANS_TYPE_FADE_WHITE_FAST
            | TRANS_TYPE_FADE_BLACK_SLOW
            | TRANS_TYPE_FADE_WHITE_SLOW
            | TRANS_TYPE_FADE_WHITE_CS_DELAYED
            | TRANS_TYPE_FADE_WHITE_INSTANT
            | TRANS_TYPE_FADE_GREEN
            | TRANS_TYPE_FADE_BLUE
    )
}

/// `TransitionFade` (`z_fbdemo_fade.c`), the instance every in-game transition here uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransitionFade {
    /// `TransitionFadeType`: 0 none, 1 a timed fade (`TRANS_FADE_TYPE_ONE_WAY`), 2 the flash
    /// `R_TRANS_FADE_FLASH_ALPHA_STEP` drives (`TRANS_FADE_TYPE_FLASH`: Play's
    /// `transitionFadeFlash`).
    pub fade_type: u8,
    /// 1: from covered to clear (arriving), 0: from clear to covered (leaving).
    pub fade_direction: u8,
    pub fade_timer: u16,
    pub color: [u8; 4],
    pub is_done: bool,
}

impl TransitionFade {
    /// `TransitionFade_SetType`.
    pub fn set_type(&mut self, ty: i32) {
        match ty {
            1 => {
                self.fade_type = 1;
                self.fade_direction = 1;
            }
            2 => {
                self.fade_type = 1;
                self.fade_direction = 0;
            }
            3 => self.fade_type = 2,
            _ => self.fade_type = 0,
        }
    }

    /// `TransitionFade_SetColor` (RGBA8).
    pub fn set_color(&mut self, rgba: u32) {
        self.color = rgba.to_be_bytes();
    }

    /// `TransitionFade_Start`.
    pub fn start(&mut self) {
        match self.fade_type {
            1 => {
                self.fade_timer = 0;
                self.color[3] = if self.fade_direction != 0 { 255 } else { 0 };
            }
            2 => self.color[3] = 0,
            _ => {}
        }
        self.is_done = false;
    }

    /// `TransitionFade_Update`: `update_rate` is `R_UPDATE_RATE` (3), `fade_duration`
    /// `gSaveContext.transFadeDuration`, `flash_alpha_step` `R_TRANS_FADE_FLASH_ALPHA_STEP`
    /// (`iREG(50)`, `regs.h`).
    ///
    /// The flash (`TRANS_FADE_TYPE_FLASH`): a negative step starts it, the alpha straight to 255
    /// (`Math_StepToS(.., 255, 255)`) and the step to 150; a positive one steps itself towards 20
    /// by 60 (150, 90, 30, 20, 20 ...) and the alpha down by it to 0, where the step goes back to
    /// 0 and the fade is done. A step of 0 leaves the alpha as it is.
    pub fn update(&mut self, update_rate: u16, fade_duration: u16, flash_alpha_step: &mut i16) {
        match self.fade_type {
            1 => {
                self.fade_timer += update_rate;
                if self.fade_timer >= fade_duration {
                    self.fade_timer = fade_duration;
                    self.is_done = true;
                }
                let alpha = ((255.0 * self.fade_timer as f32) / fade_duration.max(1) as f32) as i32;
                self.color[3] = if self.fade_direction != 0 { 255 - alpha } else { alpha } as u8;
            }
            2 => {
                let mut new_alpha = self.color[3] as i16;
                if *flash_alpha_step != 0 {
                    if *flash_alpha_step < 0 {
                        if eng_math::step_to_s(&mut new_alpha, 255, 255) {
                            *flash_alpha_step = 150;
                        }
                    } else {
                        eng_math::step_to_s(flash_alpha_step, 20, 60);
                        if eng_math::step_to_s(&mut new_alpha, 0, *flash_alpha_step) {
                            *flash_alpha_step = 0;
                            self.is_done = true;
                        }
                    }
                }
                self.color[3] = new_alpha as u8;
            }
            _ => {}
        }
    }

    /// What `TransitionFade_Draw` fills the screen with (nothing when clear).
    pub fn fill(&self) -> Option<[u8; 4]> {
        (self.color[3] > 0).then_some(self.color)
    }
}

/// `TRANS_INSTANCE_TYPE_FADE_FLASH` (`transition_instances.h`): what `TransitionFade_SetType`
/// makes a flash.
pub const TRANS_INSTANCE_TYPE_FADE_FLASH: i32 = 3;

/// `RGBA8(r, g, b, a)`.
pub const fn rgba8(r: u8, g: u8, b: u8, a: u8) -> u32 {
    u32::from_be_bytes([r, g, b, a])
}

/// `TRANS_INSTANCE_TYPE_FILL_OUT`, `_FILL_IN` (`transition_instances.h`): arriving (the screen
/// uncovered) and leaving (covered).
pub const TRANS_INSTANCE_TYPE_FILL_OUT: i32 = 1;
pub const TRANS_INSTANCE_TYPE_FILL_IN: i32 = 2;
/// `TC_SET_PARAMS` (`transition.h`): `TransitionCircle_SetType` takes the type's appearance,
/// colour and speed.
pub const TC_SET_PARAMS: i32 = 1 << 7;
/// `TransitionCircleDirection`.
pub const TRANS_CIRCLE_DIR_IN: u8 = 0;
pub const TRANS_CIRCLE_DIR_OUT: u8 = 1;
/// `NA_SE_OC_SECRET_WARP_IN`, `_OUT` (`ocarinabank_table.h`: 0x5803, 0x5804): the ripple's.
pub const NA_SE_OC_SECRET_WARP_IN: u16 = 0x5803;
pub const NA_SE_OC_SECRET_WARP_OUT: u16 = 0x5804;

/// Is `t` a circle (`(transitionType >> 5) == 1`)?
pub fn is_circle_type(t: u8) -> bool {
    t >> 5 == 1
}

/// `TransitionCircle` (`z_fbdemo_circle.c`): a ring textured with a gradient (`sTransCircleDL`,
/// one of four 16x64 textures by appearance), its texture scrolled down the ring (`texY`, in
/// quarter texels: 500 open, 0 closed) 20 a frame (10 slow), the hole growing or shrinking. Drawn
/// in its own view (`guPerspective` 60, 4:3, 10 to 12800; `guLookAt` from (0, 0, 400); scaled
/// 14.8), over the scene and under the HUD (`crate::play`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransitionCircle {
    pub color: [u8; 4],
    pub unk_color: [u8; 4],
    pub tex_x: i32,
    pub tex_y: i32,
    pub speed: i32,
    pub direction: u8,
    pub color_type: u8,
    pub speed_type: u8,
    pub appearance_type: u8,
    pub is_done: bool,
    /// `frame`: the draw's matrix buffer, swapped each draw.
    pub frame: u8,
}

impl TransitionCircle {
    /// `TransitionCircle_Start`. Returns the ripple's sound for a closing one
    /// (`SFX_PLAY_CENTERED(NA_SE_OC_SECRET_WARP_OUT)`).
    pub fn start(&mut self) -> Option<u16> {
        self.is_done = false;
        // (texture: the appearance's, `TransitionCircle::bake`.)
        self.speed = if self.speed_type == TCS_FAST { 20 } else { 10 };
        match self.color_type {
            TCC_BLACK => self.color = [0, 0, 0, 255],
            TCC_WHITE => self.color = [160, 160, 160, 255],
            TCC_GRAY => self.color = [100, 100, 100, 255],
            _ => {
                self.speed = 40;
                self.color = if self.appearance_type == TCA_WAVE { [0, 0, 0, 255] } else { [160, 160, 160, 255] };
            }
        }
        let mut sfx = None;
        if self.direction != TRANS_CIRCLE_DIR_IN {
            self.tex_y = 0;
            if self.color_type == TCC_SPECIAL {
                self.tex_y = (62.5 * 4.0) as i32;
            }
        } else {
            self.tex_y = (125.0 * 4.0) as i32;
            if self.appearance_type == TCA_RIPPLE {
                sfx = Some(NA_SE_OC_SECRET_WARP_OUT);
            }
        }
        // (guPerspective, guLookAt: TransitionCircle::view.)
        sfx
    }

    /// `TransitionCircle_Init`: zeroed.
    pub fn init() -> TransitionCircle {
        TransitionCircle::default()
    }

    /// `TransitionCircle_Update`: `speed * 3 / updateRate` a frame, open to 500 (`TRANS_CIRCLE_DIR_OUT`)
    /// or shut to 0 (`_IN`; `TCC_SPECIAL` stops at 250). Returns the ripple's sound as it starts
    /// opening (`NA_SE_OC_SECRET_WARP_IN`).
    pub fn update(&mut self, update_rate: i32) -> Option<u16> {
        let mut sfx = None;
        if self.direction != TRANS_CIRCLE_DIR_IN {
            if self.tex_y == 0 && self.appearance_type == TCA_RIPPLE {
                sfx = Some(NA_SE_OC_SECRET_WARP_IN);
            }
            self.tex_y += self.speed * 3 / update_rate;
            if self.tex_y >= (125.0 * 4.0) as i32 {
                self.tex_y = (125.0 * 4.0) as i32;
                self.is_done = true;
            }
        } else {
            self.tex_y -= self.speed * 3 / update_rate;
            if self.color_type != TCC_SPECIAL {
                if self.tex_y <= 0 {
                    self.tex_y = 0;
                    self.is_done = true;
                }
            } else if self.tex_y <= (62.5 * 4.0) as i32 {
                self.tex_y = (62.5 * 4.0) as i32;
                self.is_done = true;
            }
        }
        sfx
    }

    /// `TransitionCircle_SetType`: with `TC_SET_PARAMS`, the type's direction (overwritten by the
    /// second call), colour, speed and appearance; else `TRANS_INSTANCE_TYPE_FILL_OUT` opens and
    /// anything else shuts.
    pub fn set_type(&mut self, ty: i32) {
        if ty & TC_SET_PARAMS != 0 {
            self.direction = ((ty >> 5) & 1) as u8;
            self.color_type = ((ty >> 3) & 3) as u8;
            self.speed_type = (ty & 1) as u8;
            self.appearance_type = ((ty >> 1) & 3) as u8;
        } else if ty == TRANS_INSTANCE_TYPE_FILL_OUT {
            self.direction = TRANS_CIRCLE_DIR_OUT;
        } else {
            self.direction = TRANS_CIRCLE_DIR_IN;
        }
    }

    /// `TransitionCircle_SetColor` (RGBA8).
    pub fn set_color(&mut self, rgba: u32) {
        self.color = rgba.to_be_bytes();
    }

    /// `TransitionCircle_SetUnkColor`.
    pub fn set_unk_color(&mut self, rgba: u32) {
        self.unk_color = rgba.to_be_bytes();
    }

    /// The bake `TransitionCircle_Draw` draws: `sTransCircleDL` with the appearance's texture
    /// (`TCA_WAVE` the wave's, `_RIPPLE`, `_STARBURST`, anything else the normal one).
    pub fn bake(&self) -> String {
        circle_bake_name(match self.appearance_type {
            TCA_WAVE => 1,
            TCA_RIPPLE => 2,
            TCA_STARBURST => 3,
            _ => 0,
        })
    }

    /// `TransitionCircle_Draw`'s segments: 9 the texture's scroll (`Gfx_BranchTexScroll(texX, texY,
    /// 16, 64)`), and the prim and env colours (`gDPSetColor`, the bake's prelude).
    pub fn segment_values(&self) -> eng_gfx::SegmentValues {
        let mut sv = eng_gfx::SegmentValues::default();
        sv.read(CIRCLE_SEG_SCROLL, &branch_tex_scroll(self.tex_x as u32, self.tex_y as u32, 16, 64));
        sv.read(CIRCLE_SEG_COLORS, &circle_colors(u32::from_be_bytes(self.color)));
        sv
    }

    /// The draw's matrices as the pause pass takes them (`eng_gfx::DrawLists::pause`): the view,
    /// `guLookAt(0, 0, 400, 0, 0, 0, 0, 1, 0)`, times the model, `guScale(14.8, 14.8, 1)` (no
    /// rotation or translation).
    pub fn transform() -> glam::Mat4 {
        let view = glam::camera::rh::view::look_at_mat4(glam::Vec3::new(0.0, 0.0, 400.0), glam::Vec3::ZERO, glam::Vec3::Y);
        view * glam::Mat4::from_scale(glam::Vec3::new(14.8, 14.8, 1.0))
    }
}

/// `TransitionCircle_Start`'s `guPerspective(.., 60, 4 / 3, 10, 12800, 1)`: the vertical field of
/// view is the game's (a wider target sees more to the sides, which the pillarbox covers).
pub const CIRCLE_VIEW: eng_gfx::Perspective = eng_gfx::Perspective { fovy: 60.0, near: 10.0, far: 12800.0 };

/// The circle's segments: 8 the texture (`gSPSegment(gfx++, 8, this->texture)`), 9 the scroll
/// (called from `sTransCircleDL`), 0xC the colours run first.
pub const CIRCLE_SEG_TEX: u8 = 0x08;
pub const CIRCLE_SEG_SCROLL: u8 = 0x09;
pub const CIRCLE_SEG_COLORS: u8 = 0x0C;
/// `sTransCircleNormalTex`, `WaveTex`, `RippleTex`, `StarburstTex` (`code`'s `fbdemo_circle`).
pub const CIRCLE_TEXTURES: [&str; 4] = ["sTransCircleNormalTex", "sTransCircleWaveTex", "sTransCircleRippleTex", "sTransCircleStarburstTex"];

pub fn circle_bake_name(texture: usize) -> String {
    format!("TransitionCircle/{}", CIRCLE_TEXTURES[texture])
}

/// `gDPSetColor(G_SETPRIMCOLOR, rgba)`, `gDPSetColor(G_SETENVCOLOR, rgba)`.
fn circle_colors(rgba: u32) -> Vec<(u32, u32)> {
    vec![(0xFA00_0000, rgba), (0xFB00_0000, rgba), (0xDF00_0000, 0)]
}

/// `Gfx_BranchTexScroll` (`z_rcp.c`): the render tile at `x`, `y` (quarter texels, not wrapped),
/// `width` by `height`.
pub fn branch_tex_scroll(x: u32, y: u32, width: i32, height: i32) -> Vec<(u32, u32)> {
    use crate::scene_table::{g_dp_set_tile_size, g_dp_tile_sync, g_sp_end_display_list};
    vec![g_dp_tile_sync(), g_dp_set_tile_size(0, x, y, x.wrapping_add(((width - 1) << 2) as u32), y.wrapping_add(((height - 1) << 2) as u32)), g_sp_end_display_list()]
}

/// The circle's bakes, one per texture: `sTransCircleDL` (a `code` list, its vertices at the
/// code's own addresses) after the colours, the scroll dynamic.
pub fn bakes() -> Vec<crate::pack::MeshBake> {
    use crate::pack::{BakeBody, BakeSegment, MeshBake};
    (0..CIRCLE_TEXTURES.len())
        .map(|t| MeshBake {
            name: circle_bake_name(t),
            object: "gameplay_keep".into(),
            segments: vec![
                (CIRCLE_SEG_COLORS, BakeSegment::Dynamic(circle_colors(0x0000_00FF))),
                (CIRCLE_SEG_TEX, BakeSegment::Texture { file: "code".into(), symbol: CIRCLE_TEXTURES[t].into() }),
                (CIRCLE_SEG_SCROLL, BakeSegment::Dynamic(branch_tex_scroll(0, 0, 16, 64))),
            ],
            prelude: vec![CIRCLE_SEG_COLORS],
            body: BakeBody::DLists(vec![("code".into(), "sTransCircleDL".into())]),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_types_pack_appearance_colour_and_speed() {
        // TRANS_TYPE_CIRCLE(appearance, color, speed) = 1 << 5 | (color & 3) << 3 |
        // (appearance & 3) << 1 | (speed & 1): the guards' courtyard's CIRCLE(WAVE, BLACK, FAST)
        // 34, the throw-out's CIRCLE(STARBURST, WHITE, FAST) 46.
        assert_eq!(trans_type_circle(TCA_WAVE, TCC_BLACK, TCS_FAST), 34);
        assert_eq!(trans_type_circle(TCA_STARBURST, TCC_WHITE, TCS_FAST), 46);
        assert!(is_circle_type(34) && is_circle_type(55) && !is_circle_type(31) && !is_circle_type(64));
        // TransitionCircle_SetType: the first call (TC_SET_PARAMS) takes the type's fields, the
        // second the direction (TRANS_INSTANCE_TYPE_FILL_OUT opens, anything else shuts).
        let mut c = TransitionCircle::init();
        c.set_type(TC_SET_PARAMS | 46);
        assert_eq!((c.direction, c.color_type, c.speed_type, c.appearance_type), (TRANS_CIRCLE_DIR_OUT, TCC_WHITE, TCS_FAST, TCA_STARBURST));
        c.set_type(TRANS_INSTANCE_TYPE_FILL_IN);
        assert_eq!(c.direction, TRANS_CIRCLE_DIR_IN);
        c.set_type(TRANS_INSTANCE_TYPE_FILL_OUT);
        assert_eq!(c.direction, TRANS_CIRCLE_DIR_OUT);
        assert_eq!(c.bake(), "TransitionCircle/sTransCircleStarburstTex");
    }

    /// A circle of `ty` started in `direction`, updated at R_UPDATE_RATE 3 until done: the
    /// updates it took and its last texY.
    fn run(ty: u8, direction: i32) -> (TransitionCircle, i32) {
        let mut c = TransitionCircle::init();
        c.set_type(TC_SET_PARAMS | ty as i32);
        c.set_type(direction);
        c.start();
        let mut n = 0;
        while !c.is_done {
            c.update(3);
            n += 1;
            assert!(n < 100);
        }
        (c, n)
    }

    #[test]
    fn a_circle_shuts_and_opens_at_its_speed() {
        // TransitionCircle_Start: fast 20, slow 10 (speed * 3 / updateRate a frame at rate 3);
        // shutting from texY 500 (125 texels) to 0, opening from 0 to 500. White is 160 grey.
        let (c, n) = run(46, TRANS_INSTANCE_TYPE_FILL_IN);
        assert_eq!((c.speed, c.color, c.tex_y, n), (20, [160, 160, 160, 255], 0, 25));
        let (c, n) = run(46, TRANS_INSTANCE_TYPE_FILL_OUT);
        assert_eq!((c.tex_y, n), (500, 25));
        let (c, n) = run(trans_type_circle(TCA_NORMAL, TCC_BLACK, TCS_SLOW), TRANS_INSTANCE_TYPE_FILL_IN);
        assert_eq!((c.speed, c.color, n), (10, [0, 0, 0, 255], 50));
        // TCC_SPECIAL: 40 a frame, between 250 (62.5 texels) and 500; black for the wave, else
        // white.
        let (c, n) = run(trans_type_circle(TCA_WAVE, TCC_SPECIAL, TCS_FAST), TRANS_INSTANCE_TYPE_FILL_IN);
        assert_eq!((c.speed, c.color, c.tex_y, n), (40, [0, 0, 0, 255], 250, 7));
        let (c, n) = run(trans_type_circle(TCA_NORMAL, TCC_SPECIAL, TCS_FAST), TRANS_INSTANCE_TYPE_FILL_OUT);
        assert_eq!((c.color, c.tex_y, n), ([160, 160, 160, 255], 500, 7));
    }

    #[test]
    fn the_ripple_plays_the_secret_warp() {
        // TransitionCircle_Start shutting a ripple: NA_SE_OC_SECRET_WARP_OUT; TransitionCircle_Update
        // opening one from texY 0: NA_SE_OC_SECRET_WARP_IN, once.
        let ripple = trans_type_circle(TCA_RIPPLE, TCC_BLACK, TCS_FAST) as i32;
        let mut c = TransitionCircle::init();
        c.set_type(TC_SET_PARAMS | ripple);
        c.set_type(TRANS_INSTANCE_TYPE_FILL_IN);
        assert_eq!(c.start(), Some(NA_SE_OC_SECRET_WARP_OUT));
        c.set_type(TRANS_INSTANCE_TYPE_FILL_OUT);
        assert_eq!(c.start(), None);
        assert_eq!(c.update(3), Some(NA_SE_OC_SECRET_WARP_IN));
        assert_eq!(c.update(3), None);
    }

    #[test]
    fn the_circles_texture_scrolls_down_its_ring() {
        // Gfx_BranchTexScroll(texX, texY, 16, 64): the render tile from (texX, texY) to
        // (texX + 15 * 4, texY + 63 * 4).
        let c = TransitionCircle { tex_y: 240, ..Default::default() };
        let sv = c.segment_values();
        let expected = branch_tex_scroll(0, 240, 16, 64);
        assert_eq!(expected[1], crate::scene_table::g_dp_set_tile_size(0, 0, 240, 60, 240 + 252));
        // Segment 9's render tile at (0, 240); the colours on 0xC.
        assert_eq!(sv.tiles[CIRCLE_SEG_SCROLL as usize][0], Some((0, 240)));
        assert_eq!(sv.prim[CIRCLE_SEG_COLORS as usize], Some([0, 0, 0, 0]));
    }
}
