//! The letterbox bars (`shrink_window.c`): black bars `size` rows high at the top and bottom
//! of the 320x240 frame, stepping towards a target the camera sets (`Camera_UpdateInterface`).
//! `View_ApplyLetterbox` scissors the 3D lists (`POLY_OPA_DISP`, `POLY_XLU_DISP`) to the rows
//! between them; the overlay list (the HUD and the Z-target reticle) isn't scissored.

/// `LetterboxState`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LetterboxState {
    #[default]
    Idle,
    Growing,
    Shrinking,
}

/// `sLetterboxState`, `sLetterboxSizeTarget`, `sLetterboxSize`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Letterbox {
    pub state: LetterboxState,
    pub size_target: i32,
    pub size: i32,
}

impl Letterbox {
    /// `Letterbox_Init`.
    pub fn new() -> Letterbox {
        Letterbox::default()
    }

    /// `Letterbox_SetSizeTarget`.
    pub fn set_size_target(&mut self, target: i32) {
        self.size_target = target;
    }

    /// `Letterbox_SetSize`.
    pub fn set_size(&mut self, size: i32) {
        self.size = size;
    }

    /// `Letterbox_Update(R_UPDATE_RATE)`: at 20 Hz (`R_UPDATE_RATE` 3) the step is 10 rows.
    pub fn update(&mut self, update_rate: i32) {
        let step = if update_rate == 3 { 10 } else { 30 / update_rate };
        if self.size < self.size_target {
            self.state = LetterboxState::Growing;
            if self.size + step < self.size_target {
                self.size += step;
            } else {
                self.size = self.size_target;
            }
        } else if self.size_target < self.size {
            self.state = LetterboxState::Shrinking;
            if self.size_target < self.size - step {
                self.size -= step;
            } else {
                self.size = self.size_target;
            }
        } else {
            self.state = LetterboxState::Idle;
        }
    }

    /// `View_ApplyLetterbox`'s rows: the bar height, clamped to half the screen (`SCREEN_HEIGHT` 240).
    pub fn rows(&self) -> i32 {
        self.size.clamp(0, 240 / 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_ten_rows_a_frame_at_20hz() {
        let mut l = Letterbox::new();
        l.set_size_target(27);
        let sizes: Vec<i32> = (0..4)
            .map(|_| {
                l.update(3);
                l.size
            })
            .collect();
        assert_eq!(sizes, vec![10, 20, 27, 27]);
        assert_eq!(l.state, LetterboxState::Idle);
        l.set_size_target(0);
        l.update(3);
        assert_eq!((l.size, l.state), (17, LetterboxState::Shrinking));
        l.update(3);
        l.update(3);
        assert_eq!(l.size, 0);
    }
}
