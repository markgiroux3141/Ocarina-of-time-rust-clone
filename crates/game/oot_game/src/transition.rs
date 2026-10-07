//! Scene transitions: the trigger and mode `Play_Update` steps through (`z_play.c`,
//! `z64.h`), and the fade (`z_fbdemo_fade.c`). `PlayState::update_transition` runs them.
//!
//! Ported: the fades (`TransitionFade`, every `TRANS_TYPE_FADE_*`), the white and brown fills
//! and the instant cut. The wipe, triforce and circle transitions (`z_fbdemo_wipe1.c`,
//! `_triforce.c`, `_circle.c`) are drawn as a black fade of the same kind, and take the fade's
//! time (`transFadeDuration`, 60) rather than their own; the sandstorm and the cutscene fills
//! end at once.

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
