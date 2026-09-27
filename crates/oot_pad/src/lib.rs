//! Controller devices mapped onto an N64 pad (`oot_game::input::PadState`).
//!
//! Backend: `gilrs` (Windows.Gaming.Input on Windows). Each connected pad is matched to a
//! profile by USB vendor/product id; profiles bind N64 buttons either to raw native button
//! codes (the HID button index for WGI raw controllers) or to gilrs' semantic buttons, and
//! the N64 stick to an axis pair.
//!
//! Why raw codes: gilrs' semantic layer uses SDL's community mapping, which for the Retro-Bit
//! N64 pad (VID 2563, PID 0575) is a generic "PS3 Controller" layout. That puts C-Up on
//! Start and other N64 buttons on the wrong names. The raw HID order is what the pad
//! actually reports.
//!
//! Overrides can go in `oot.toml`:
//! ```toml
//! [pad]
//! stick_range = 80        # N64 units at full tilt (an original pad reads about ±80)
//! [pad.buttons]           # N64 button = "b<raw code>" or a gilrs Button name
//! A = "b2"
//! ```

use std::collections::HashMap;
use std::path::Path;

use gilrs::{Axis, Button, EventType, GamepadId, Gilrs, GilrsBuilder};
use oot_game::input::*;

/// A binding for one N64 button.
#[derive(Debug, Clone, PartialEq)]
pub enum Bind {
    /// Native code as reported in gilrs events (`Code::into_u32`), i.e. the HID button index.
    Raw(u32),
    Semantic(Button),
    /// A stick axis past a threshold (for pads whose C-buttons are the right stick).
    AxisPos(Axis),
    AxisNeg(Axis),
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    /// USB (vendor, product) this profile is for; `None` matches any pad.
    pub ids: Option<(u16, u16)>,
    pub buttons: Vec<(u16, Vec<Bind>)>,
    pub stick_x: Axis,
    pub stick_y: Axis,
    /// N64 units at full deflection.
    pub stick_range: f32,
}

impl Profile {
    /// Retro-Bit N64 USB (serial AZ-RB-N64P, "SWITCH CO.,LTD. Controller (Dinput)").
    /// Raw codes were confirmed by pressing each button on the user's pad (see the
    /// spike 03 findings).
    pub fn retro_bit_n64() -> Profile {
        use Bind::*;
        Profile {
            name: "Retro-Bit N64 (2563:0575, raw HID codes)".into(),
            ids: Some((0x2563, 0x0575)),
            buttons: vec![
                (BTN_A, vec![Raw(2)]),
                (BTN_B, vec![Raw(1)]),
                (BTN_Z, vec![Raw(6)]),
                (BTN_START, vec![Raw(12)]),
                (BTN_L, vec![Raw(4)]),
                (BTN_R, vec![Raw(5)]),
                (BTN_CUP, vec![Raw(9)]),
                (BTN_CDOWN, vec![Raw(3)]),
                (BTN_CLEFT, vec![Raw(0)]),
                (BTN_CRIGHT, vec![Raw(8)]),
                (BTN_DUP, vec![Semantic(Button::DPadUp)]),
                (BTN_DDOWN, vec![Semantic(Button::DPadDown)]),
                (BTN_DLEFT, vec![Semantic(Button::DPadLeft)]),
                (BTN_DRIGHT, vec![Semantic(Button::DPadRight)]),
            ],
            stick_x: Axis::LeftStickX,
            stick_y: Axis::LeftStickY,
            stick_range: 80.0,
        }
    }

    /// Any other pad (XInput and SDL-mapped): the usual N64-on-modern-pad layout. A = south,
    /// B = west, Z = left trigger, R = right trigger or bumper, L = left bumper, C = right
    /// stick, with north and east also giving C-Up and C-Right.
    pub fn generic() -> Profile {
        use Bind::*;
        Profile {
            name: "generic gamepad (semantic mapping)".into(),
            ids: None,
            buttons: vec![
                (BTN_A, vec![Semantic(Button::South)]),
                (BTN_B, vec![Semantic(Button::West)]),
                (BTN_Z, vec![Semantic(Button::LeftTrigger2)]),
                (BTN_START, vec![Semantic(Button::Start)]),
                (BTN_L, vec![Semantic(Button::LeftTrigger)]),
                (BTN_R, vec![Semantic(Button::RightTrigger2), Semantic(Button::RightTrigger)]),
                (BTN_CUP, vec![AxisPos(Axis::RightStickY), Semantic(Button::North)]),
                (BTN_CDOWN, vec![AxisNeg(Axis::RightStickY)]),
                (BTN_CLEFT, vec![AxisNeg(Axis::RightStickX)]),
                (BTN_CRIGHT, vec![AxisPos(Axis::RightStickX), Semantic(Button::East)]),
                (BTN_DUP, vec![Semantic(Button::DPadUp)]),
                (BTN_DDOWN, vec![Semantic(Button::DPadDown)]),
                (BTN_DLEFT, vec![Semantic(Button::DPadLeft)]),
                (BTN_DRIGHT, vec![Semantic(Button::DPadRight)]),
            ],
            stick_x: Axis::LeftStickX,
            stick_y: Axis::LeftStickY,
            stick_range: 80.0,
        }
    }

    pub fn for_ids(vid: Option<u16>, pid: Option<u16>) -> Profile {
        match (vid, pid) {
            (Some(0x2563), Some(0x0575)) => Profile::retro_bit_n64(),
            _ => Profile::generic(),
        }
    }

    /// Applies a `[pad]` table from `oot.toml`.
    pub fn apply_overrides(&mut self, cfg: &PadConfig) {
        if let Some(r) = cfg.stick_range {
            self.stick_range = r;
        }
        for (name, spec) in &cfg.buttons {
            let Some(bit) = button_bit(name) else {
                log::warn!("[pad] unknown N64 button {name}");
                continue;
            };
            let Some(bind) = parse_bind(spec) else {
                log::warn!("[pad] can't parse binding {spec:?} for {name}");
                continue;
            };
            if let Some(e) = self.buttons.iter_mut().find(|(b, _)| *b == bit) {
                e.1 = vec![bind];
            } else {
                self.buttons.push((bit, vec![bind]));
            }
        }
    }
}

pub fn button_bit(name: &str) -> Option<u16> {
    let n = name.replace(['-', '_', ' '], "").to_ascii_lowercase();
    BUTTON_NAMES.iter().find(|(_, s)| s.replace('-', "").to_ascii_lowercase() == n).map(|(b, _)| *b)
}

fn parse_bind(s: &str) -> Option<Bind> {
    let s = s.trim();
    if let Some(n) = s.strip_prefix('b').and_then(|r| r.parse().ok()) {
        return Some(Bind::Raw(n));
    }
    let buttons = [
        ("South", Button::South),
        ("East", Button::East),
        ("North", Button::North),
        ("West", Button::West),
        ("LeftTrigger", Button::LeftTrigger),
        ("LeftTrigger2", Button::LeftTrigger2),
        ("RightTrigger", Button::RightTrigger),
        ("RightTrigger2", Button::RightTrigger2),
        ("Select", Button::Select),
        ("Start", Button::Start),
        ("Mode", Button::Mode),
        ("DPadUp", Button::DPadUp),
        ("DPadDown", Button::DPadDown),
        ("DPadLeft", Button::DPadLeft),
        ("DPadRight", Button::DPadRight),
    ];
    buttons.iter().find(|(n, _)| n.eq_ignore_ascii_case(s)).map(|(_, b)| Bind::Semantic(*b))
}

/// The optional `[pad]` section of `oot.toml`.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PadConfig {
    pub stick_range: Option<f32>,
    #[serde(default)]
    pub buttons: HashMap<String, String>,
}

impl PadConfig {
    /// Reads `[pad]` from the first `oot.toml` in `start` or its ancestors.
    pub fn find(start: &Path) -> PadConfig {
        #[derive(serde::Deserialize)]
        struct File {
            pad: Option<PadConfig>,
        }
        let mut dir = Some(start);
        while let Some(d) = dir {
            let p = d.join("oot.toml");
            if let Ok(text) = std::fs::read_to_string(&p) {
                return match toml::from_str::<File>(&text) {
                    Ok(f) => f.pad.unwrap_or_default(),
                    Err(e) => {
                        log::warn!("[pad] ignoring [pad] in {}: {e}", p.display());
                        PadConfig::default()
                    }
                };
            }
            dir = d.parent();
        }
        PadConfig::default()
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: usize,
    pub name: String,
    pub os_name: String,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub mapping: String,
    pub profile: String,
    pub connected: bool,
}

/// Connected pads and the one being read.
pub struct Pads {
    gilrs: Gilrs,
    active: Option<GamepadId>,
    profile: Profile,
    config: PadConfig,
    raw_buttons: HashMap<u32, bool>,
    raw_axes: HashMap<u32, f32>,
    /// Raw events since the last `take_events` (for the device probe / calibration).
    events: Vec<String>,
    pub log_events: bool,
}

impl Pads {
    pub fn new(config: PadConfig) -> anyhow::Result<Pads> {
        // No deadzone/jitter filters: the game applies its own dead zone to the raw stick.
        let gilrs = GilrsBuilder::new().with_default_filters(false).build().map_err(|e| anyhow::anyhow!("gilrs: {e}"))?;
        let mut p = Pads {
            gilrs,
            active: None,
            profile: Profile::generic(),
            config,
            raw_buttons: HashMap::new(),
            raw_axes: HashMap::new(),
            events: Vec::new(),
            log_events: std::env::var_os("OOT_PAD_DEBUG").is_some(),
        };
        let first = p.gilrs.gamepads().next().map(|(id, _)| id);
        if let Some(id) = first {
            p.activate(id);
        }
        Ok(p)
    }

    fn activate(&mut self, id: GamepadId) {
        let g = self.gilrs.gamepad(id);
        let mut prof = Profile::for_ids(g.vendor_id(), g.product_id());
        prof.apply_overrides(&self.config);
        log::info!("[pad] using \"{}\" ({:04x?}:{:04x?}) with profile {}", g.name(), g.vendor_id(), g.product_id(), prof.name);
        self.profile = prof;
        self.active = Some(id);
        self.raw_buttons.clear();
        self.raw_axes.clear();
    }

    pub fn devices(&self) -> Vec<DeviceInfo> {
        self.gilrs
            .gamepads()
            .map(|(id, g)| DeviceInfo {
                id: usize::from(id),
                name: g.name().to_string(),
                os_name: g.os_name().to_string(),
                vid: g.vendor_id(),
                pid: g.product_id(),
                mapping: format!("{:?}", g.mapping_source()),
                profile: Profile::for_ids(g.vendor_id(), g.product_id()).name,
                connected: g.is_connected(),
            })
            .collect()
    }

    pub fn active_name(&self) -> Option<String> {
        self.active.map(|id| self.gilrs.gamepad(id).name().to_string())
    }

    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    /// Drains device events. Call once per display frame.
    pub fn update(&mut self) {
        while let Some(ev) = self.gilrs.next_event() {
            let mine = self.active == Some(ev.id);
            if self.log_events {
                log::info!("[pad] event {:?} (active {:?})", ev.event, self.active);
            }
            match ev.event {
                EventType::Connected => {
                    if self.active.is_none() {
                        self.activate(ev.id);
                    }
                }
                EventType::Disconnected => {
                    if mine {
                        self.active = None;
                        let next = self.gilrs.gamepads().find(|(_, g)| g.is_connected()).map(|(id, _)| id);
                        if let Some(n) = next {
                            self.activate(n);
                        }
                    }
                }
                EventType::ButtonPressed(b, code) if mine => {
                    self.raw_buttons.insert(code.into_u32(), true);
                    self.events.push(format!("button b{} ({b:?}) pressed", code.into_u32()));
                }
                EventType::ButtonReleased(b, code) if mine => {
                    self.raw_buttons.insert(code.into_u32(), false);
                    self.events.push(format!("button b{} ({b:?}) released", code.into_u32()));
                }
                EventType::ButtonChanged(_, v, code) if mine => {
                    self.raw_buttons.insert(code.into_u32(), v > 0.5);
                }
                EventType::AxisChanged(a, v, code) if mine => {
                    self.raw_axes.insert(code.into_u32(), v);
                    if v.abs() > 0.5 || self.log_events {
                        self.events.push(format!("axis {:#x} ({a:?}) = {v:+.2}", code.into_u32()));
                    }
                }
                _ => {}
            }
            if self.log_events {
                for e in self.events.drain(..) {
                    log::info!("[pad] {e}");
                }
            }
        }
    }

    pub fn take_events(&mut self) -> Vec<String> {
        std::mem::take(&mut self.events)
    }

    /// The raw button codes currently held.
    pub fn raw_held(&self) -> Vec<u32> {
        let mut v: Vec<u32> = self.raw_buttons.iter().filter(|(_, d)| **d).map(|(c, _)| *c).collect();
        v.sort();
        v
    }

    fn bind_down(&self, b: &Bind) -> bool {
        let Some(id) = self.active else { return false };
        let g = self.gilrs.gamepad(id);
        match b {
            Bind::Raw(c) => *self.raw_buttons.get(c).unwrap_or(&false),
            Bind::Semantic(s) => g.is_pressed(*s),
            Bind::AxisPos(a) => g.value(*a) > 0.5,
            Bind::AxisNeg(a) => g.value(*a) < -0.5,
        }
    }

    /// The active pad as an N64 pad, or `None` without a pad.
    pub fn state(&self) -> Option<PadState> {
        let id = self.active?;
        let g = self.gilrs.gamepad(id);
        if !g.is_connected() {
            return None;
        }
        let mut button = 0u16;
        for (bit, binds) in &self.profile.buttons {
            if binds.iter().any(|b| self.bind_down(b)) {
                button |= bit;
            }
        }
        let r = self.profile.stick_range;
        if self.log_events {
            let d = |a: Axis| g.axis_data(a).map(|d| (d.value(), d.counter()));
            log::debug!("[pad] stick x {:?} y {:?}", d(self.profile.stick_x), d(self.profile.stick_y));
        }
        let to_s8 = |v: f32| (v.clamp(-1.0, 1.0) * r).round().clamp(-128.0, 127.0) as i8;
        Some(PadState { button, stick_x: to_s8(g.value(self.profile.stick_x)), stick_y: to_s8(g.value(self.profile.stick_y)) })
    }
}

/// Keyboard fallback, filled in by the app from its key events.
#[derive(Debug, Clone, Copy, Default)]
pub struct Keyboard {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    /// Half tilt (walk).
    pub walk: bool,
    pub a: bool,
    pub b: bool,
    pub z: bool,
    pub r: bool,
    pub l: bool,
    pub start: bool,
    pub c_left: bool,
    pub c_right: bool,
    pub c_up: bool,
    pub c_down: bool,
}

impl Keyboard {
    /// Keys as an N64 pad: full tilt is ±80 like an original stick, diagonals normalised
    /// so they reach the same magnitude.
    pub fn state(&self) -> PadState {
        let mut x = (self.right as i32 - self.left as i32) as f32;
        let mut y = (self.up as i32 - self.down as i32) as f32;
        let len = (x * x + y * y).sqrt();
        if len > 0.0 {
            x /= len;
            y /= len;
        }
        let r = if self.walk { 32.0 } else { 80.0 };
        let mut button = 0;
        for (on, bit) in [
            (self.a, BTN_A),
            (self.b, BTN_B),
            (self.z, BTN_Z),
            (self.r, BTN_R),
            (self.l, BTN_L),
            (self.start, BTN_START),
            (self.c_left, BTN_CLEFT),
            (self.c_right, BTN_CRIGHT),
            (self.c_up, BTN_CUP),
            (self.c_down, BTN_CDOWN),
        ] {
            if on {
                button |= bit;
            }
        }
        PadState { button, stick_x: (x * r).round() as i8, stick_y: (y * r).round() as i8 }
    }

    pub fn any(&self) -> bool {
        self.state() != PadState::default()
    }
}

/// Combines pad and keyboard: buttons OR together, and the keyboard stick wins while any
/// direction key is held.
pub fn merge(pad: Option<PadState>, kb: &Keyboard) -> PadState {
    let k = kb.state();
    let mut p = pad.unwrap_or_default();
    p.button |= k.button;
    if kb.up || kb.down || kb.left || kb.right {
        p.stick_x = k.stick_x;
        p.stick_y = k.stick_y;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_selection_and_overrides() {
        assert!(Profile::for_ids(Some(0x2563), Some(0x0575)).name.starts_with("Retro-Bit"));
        assert!(Profile::for_ids(Some(0x045e), Some(0x028e)).name.starts_with("generic"));
        let mut p = Profile::retro_bit_n64();
        let cfg = PadConfig { stick_range: Some(70.0), buttons: [("C-Up".to_string(), "b11".to_string()), ("A".to_string(), "South".to_string())].into() };
        p.apply_overrides(&cfg);
        assert_eq!(p.stick_range, 70.0);
        assert_eq!(p.buttons.iter().find(|(b, _)| *b == BTN_CUP).unwrap().1, vec![Bind::Raw(11)]);
        assert_eq!(p.buttons.iter().find(|(b, _)| *b == BTN_A).unwrap().1, vec![Bind::Semantic(Button::South)]);
    }

    #[test]
    fn keyboard_diagonal_matches_full_tilt() {
        let k = Keyboard { up: true, right: true, ..Default::default() };
        let s = k.state();
        assert_eq!((s.stick_x, s.stick_y), (57, 57));
        let k = Keyboard { up: true, a: true, ..Default::default() };
        assert_eq!(k.state(), PadState { button: BTN_A, stick_x: 0, stick_y: 80 });
    }
}
