//! N64 controller state and the game's input processing.
//!
//! - Button bits: `include/ultra64/controller.h`.
//! - `PadMgr`: `padmgr.c`. The pad is polled at the device rate; presses and releases
//!   accumulate between game frames and are handed out once per frame
//!   (`PadMgr_UpdateInputs` / `PadMgr_RequestPadData(.., gameRequest = true)`), so a tap
//!   shorter than a game frame still registers as a press.
//! - `Input::update_rel`: `PadUtils_UpdateRelXY` (dead zone ±7, range 60).
//! - `stick_to_mag_angle`: `func_80077D10` in `z_lib.c` (magnitude clamped to 60).

use crate::math::atan2_s;

pub const BTN_CRIGHT: u16 = 0x0001;
pub const BTN_CLEFT: u16 = 0x0002;
pub const BTN_CDOWN: u16 = 0x0004;
pub const BTN_CUP: u16 = 0x0008;
pub const BTN_R: u16 = 0x0010;
pub const BTN_L: u16 = 0x0020;
pub const BTN_DRIGHT: u16 = 0x0100;
pub const BTN_DLEFT: u16 = 0x0200;
pub const BTN_DDOWN: u16 = 0x0400;
pub const BTN_DUP: u16 = 0x0800;
pub const BTN_START: u16 = 0x1000;
pub const BTN_Z: u16 = 0x2000;
pub const BTN_B: u16 = 0x4000;
pub const BTN_A: u16 = 0x8000;

pub const BUTTON_NAMES: [(u16, &str); 14] = [
    (BTN_A, "A"),
    (BTN_B, "B"),
    (BTN_Z, "Z"),
    (BTN_START, "Start"),
    (BTN_L, "L"),
    (BTN_R, "R"),
    (BTN_CUP, "C-Up"),
    (BTN_CDOWN, "C-Down"),
    (BTN_CLEFT, "C-Left"),
    (BTN_CRIGHT, "C-Right"),
    (BTN_DUP, "D-Up"),
    (BTN_DDOWN, "D-Down"),
    (BTN_DLEFT, "D-Left"),
    (BTN_DRIGHT, "D-Right"),
];

/// `OSContPad`: what the controller reports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PadState {
    pub button: u16,
    pub stick_x: i8,
    pub stick_y: i8,
}

impl PadState {
    pub fn held(&self, b: u16) -> bool {
        self.button & b == b
    }
    pub fn names(&self) -> String {
        let v: Vec<&str> = BUTTON_NAMES.iter().filter(|(b, _)| self.button & b != 0).map(|(_, n)| *n).collect();
        v.join("+")
    }
}

/// `Input`: the per-frame view the game reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Input {
    pub cur: PadState,
    pub prev: PadState,
    pub press: PadState,
    pub rel: PadState,
}

impl Input {
    /// `PadUtils_UpdateRelXY`: dead zone of 7 each side, clamped at 0x43 - 7 = 60.
    pub fn update_rel(&mut self) {
        let f = |c: i8| -> i8 {
            let c = c as i32;
            let r = if c > 7 {
                if c < 0x43 { c - 7 } else { 0x43 - 7 }
            } else if c < -7 {
                if c > -0x43 { c + 7 } else { -0x43 + 7 }
            } else {
                0
            };
            r as i8
        };
        self.rel.stick_x = f(self.cur.stick_x);
        self.rel.stick_y = f(self.cur.stick_y);
    }
}

/// `PadMgr` for one controller.
#[derive(Debug, Clone, Default)]
pub struct PadMgr {
    input: Input,
}

impl PadMgr {
    /// `PadMgr_UpdateInputs`: one poll of the controller.
    pub fn poll(&mut self, pad: PadState) {
        let i = &mut self.input;
        i.prev = i.cur;
        i.cur = pad;
        let diff = i.prev.button ^ i.cur.button;
        i.press.button |= diff & i.cur.button;
        i.rel.button |= diff & i.prev.button;
        i.update_rel();
        i.press.stick_x = i.press.stick_x.wrapping_add(i.cur.stick_x.wrapping_sub(i.prev.stick_x));
        i.press.stick_y = i.press.stick_y.wrapping_add(i.cur.stick_y.wrapping_sub(i.prev.stick_y));
    }

    /// `PadMgr_RequestPadData(.., gameRequest = true)`: the game's per-frame copy. Press and
    /// release are cleared so they are only read once.
    pub fn request(&mut self) -> Input {
        let out = self.input;
        self.input.press = PadState::default();
        self.input.rel.button = 0;
        out
    }
}

/// `func_80077D10`: stick magnitude (0..=60) and angle relative to "up".
/// Up is 0, right is -0x4000 (the game adds the camera yaw to get a world direction).
pub fn stick_to_mag_angle(input: &Input) -> (f32, i16) {
    let rx = input.rel.stick_x as f32;
    let ry = input.rel.stick_y as f32;
    let mag = (rx * rx + ry * ry).sqrt().min(60.0);
    (mag, atan2_s(ry, -rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rel_stick_dead_zone_and_range() {
        let mut i = Input::default();
        for (raw, rel) in [(0, 0), (7, 0), (8, 1), (-8, -1), (66, 59), (67, 60), (80, 60), (-127, -60)] {
            i.cur.stick_x = raw;
            i.update_rel();
            assert_eq!(i.rel.stick_x, rel, "raw {raw}");
        }
    }

    #[test]
    fn short_tap_is_seen_once() {
        let mut p = PadMgr::default();
        p.poll(PadState { button: BTN_A, ..Default::default() });
        p.poll(PadState::default());
        let f = p.request();
        assert_eq!(f.press.button, BTN_A);
        assert_eq!(f.cur.button, 0);
        assert_eq!(p.request().press.button, 0);
    }

    #[test]
    fn stick_angle_convention() {
        let mut i = Input::default();
        i.cur.stick_y = 80;
        i.update_rel();
        assert_eq!(stick_to_mag_angle(&i), (60.0, 0));
        i.cur = PadState { stick_x: 80, ..Default::default() };
        i.update_rel();
        assert_eq!(stick_to_mag_angle(&i).1, -0x4000);
    }
}
