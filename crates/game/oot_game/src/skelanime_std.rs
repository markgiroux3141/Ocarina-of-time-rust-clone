//! `SkelAnime` for standard animations (`z_skelanime.c`): what every actor but Player plays.
//! `SkelAnime_InitFlex`, `Animation_Change` / `Animation_ChangeByInfo`, `SkelAnime_Update` with
//! its update functions (`SkelAnime_LoopFull`, `_LoopPartial`, `_Once`, `_Morph`,
//! `_MorphTaper`, `SkelAnime_AnimateFrame`) and `Animation_OnFrame`.
//!
//! The animations come from the pack decompressed (`StandardAnimation`: a joint table per
//! frame), so `SkelAnime_GetFrameData` is a lookup. The update rate `R_UPDATE_RATE / 3` is 1 at
//! the game's 20 Hz.

use std::sync::Arc;

use eng_anim::anim::StandardAnimation;

pub const ANIMMODE_LOOP: u8 = 0;
pub const ANIMMODE_LOOP_INTERP: u8 = 1;
pub const ANIMMODE_ONCE: u8 = 2;
pub const ANIMMODE_ONCE_INTERP: u8 = 3;
pub const ANIMMODE_LOOP_PARTIAL: u8 = 4;
pub const ANIMMODE_LOOP_PARTIAL_INTERP: u8 = 5;
/// `ANIM_INTERP`: the mode's low bit.
const ANIM_INTERP: u8 = 1;
/// `ANIMTAPER_NONE`, `ANIMTAPER_DECEL`.
pub const ANIMTAPER_NONE: i8 = 0;
pub const ANIMTAPER_DECEL: i8 = -1;

/// `R_UPDATE_RATE * (1.0f / 3.0f)` with `R_UPDATE_RATE` 3.
const UPDATE_RATE: f32 = 3.0 * (1.0 / 3.0);

/// An animation and the name it was loaded by (the actors compare `skelAnime.animation`).
#[derive(Debug, Clone)]
pub struct Anim {
    pub name: String,
    pub data: Arc<StandardAnimation>,
}

impl Anim {
    /// `Animation_GetLength`: the frame count.
    pub fn length(&self) -> f32 {
        self.data.frames.len() as f32
    }
    /// `Animation_GetLastFrame`.
    pub fn last_frame(&self) -> f32 {
        (self.data.frames.len() as u16).wrapping_sub(1) as f32
    }
}

/// `AnimationInfo`.
#[derive(Debug, Clone)]
pub struct AnimationInfo {
    pub animation: Anim,
    pub play_speed: f32,
    pub start_frame: f32,
    /// `frameCount`: the end frame, or the animation's last frame when not above 0.
    pub frame_count: f32,
    pub mode: u8,
    pub morph_frames: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateFn {
    LoopFull,
    LoopPartial,
    Once,
    Morph,
    MorphTaper,
}

/// `SkelAnime` with a standard animation.
#[derive(Debug, Clone)]
pub struct SkelAnimeStd {
    /// `limbCount`: the skeleton's limbs plus the root translation entry.
    pub limb_count: usize,
    pub joint_table: Vec<[i16; 3]>,
    pub morph_table: Vec<[i16; 3]>,
    pub animation: Option<Anim>,
    pub cur_frame: f32,
    pub start_frame: f32,
    pub end_frame: f32,
    pub anim_length: f32,
    pub play_speed: f32,
    pub mode: u8,
    pub morph_weight: f32,
    pub morph_rate: f32,
    pub taper: i8,
    update: UpdateFn,
}

impl SkelAnimeStd {
    /// `SkelAnime_InitFlex` for a skeleton of `limbs` limbs, playing `animation` looped if given.
    pub fn init_flex(limbs: usize, animation: Option<Anim>) -> SkelAnimeStd {
        let limb_count = limbs + 1;
        let mut s = SkelAnimeStd {
            limb_count,
            joint_table: vec![[0; 3]; limb_count],
            morph_table: vec![[0; 3]; limb_count],
            animation: None,
            cur_frame: 0.0,
            start_frame: 0.0,
            end_frame: 0.0,
            anim_length: 0.0,
            play_speed: 0.0,
            mode: ANIMMODE_LOOP,
            morph_weight: 0.0,
            morph_rate: 0.0,
            taper: ANIMTAPER_NONE,
            update: UpdateFn::LoopFull,
        };
        if let Some(a) = animation {
            // Animation_PlayLoop.
            let last = a.last_frame();
            s.change(a, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
        }
        s
    }

    /// Whether the current animation is `name` (`skelAnime->animation == &name`).
    pub fn is(&self, name: &str) -> bool {
        self.animation.as_ref().is_some_and(|a| a.name == name)
    }

    /// `SkelAnime_GetFrameData`.
    fn frame_data(anim: &Anim, frame: i32, limb_count: usize, out: &mut [[i16; 3]]) {
        let f = &anim.data.frames[(frame.max(0) as usize).min(anim.data.frames.len().saturating_sub(1))];
        for (i, o) in out.iter_mut().enumerate().take(limb_count) {
            *o = f.rot.get(i).copied().unwrap_or([0; 3]);
        }
    }

    /// `SkelAnime_SetUpdate`.
    fn set_update(&mut self) {
        self.update = if self.mode <= ANIMMODE_LOOP_INTERP {
            UpdateFn::LoopFull
        } else if self.mode <= ANIMMODE_ONCE_INTERP {
            UpdateFn::Once
        } else {
            UpdateFn::LoopPartial
        };
    }

    /// `SkelAnime_Update`: advances the animation; true when a once-animation has finished.
    pub fn update(&mut self) -> bool {
        match self.update {
            UpdateFn::LoopFull => self.loop_full(),
            UpdateFn::LoopPartial => self.loop_partial(),
            UpdateFn::Once => self.once(),
            UpdateFn::Morph => self.morph(),
            UpdateFn::MorphTaper => self.morph_taper(),
        }
    }

    /// `SkelAnime_Morph`.
    fn morph(&mut self) -> bool {
        let prev = self.morph_weight;
        self.morph_weight -= self.morph_rate * UPDATE_RATE;
        if self.morph_weight <= 0.0 {
            self.set_update();
            self.morph_weight = 0.0;
        }
        let w = 1.0 - (self.morph_weight / prev);
        interp_frame_table(&mut self.joint_table, &self.morph_table.clone(), w);
        false
    }

    /// `SkelAnime_MorphTaper`.
    fn morph_taper(&mut self) -> bool {
        let prev_phase = (self.morph_weight * 16384.0) as i32 as i16;
        self.morph_weight -= self.morph_rate * UPDATE_RATE;
        if self.morph_weight <= 0.0 {
            self.set_update();
            self.morph_weight = 0.0;
        }
        let cur_phase = (self.morph_weight * 16384.0) as i32 as i16;
        let (prev_w, mut cur_w) = if self.taper <= ANIMTAPER_DECEL {
            (1.0 - eng_math::cos_s(prev_phase), 1.0 - eng_math::cos_s(cur_phase))
        } else {
            (eng_math::sin_s(prev_phase), eng_math::sin_s(cur_phase))
        };
        if cur_w != 0.0 {
            cur_w /= prev_w;
        } else {
            cur_w = 0.0;
        }
        interp_frame_table(&mut self.joint_table, &self.morph_table.clone(), 1.0 - cur_w);
        false
    }

    /// `SkelAnime_AnimateFrame`.
    fn animate_frame(&mut self) {
        let Some(anim) = self.animation.clone() else { return };
        Self::frame_data(&anim, self.cur_frame as i32, self.limb_count, &mut self.joint_table);
        if self.mode & ANIM_INTERP != 0 {
            let mut frame = self.cur_frame as i32;
            let partial = self.cur_frame - frame as f32;
            frame += 1;
            if frame >= self.anim_length as i32 {
                frame = 0;
            }
            let mut next = vec![[0i16; 3]; self.limb_count];
            Self::frame_data(&anim, frame, self.limb_count, &mut next);
            interp_frame_table(&mut self.joint_table, &next, partial);
        }
        if self.morph_weight != 0.0 {
            self.morph_weight -= self.morph_rate * UPDATE_RATE;
            if self.morph_weight <= 0.0 {
                self.morph_weight = 0.0;
            } else {
                let w = self.morph_weight;
                interp_frame_table(&mut self.joint_table, &self.morph_table.clone(), w);
            }
        }
    }

    /// `SkelAnime_LoopFull`.
    fn loop_full(&mut self) -> bool {
        self.cur_frame += self.play_speed * UPDATE_RATE;
        if self.cur_frame < 0.0 {
            self.cur_frame += self.anim_length;
        } else if self.anim_length <= self.cur_frame {
            self.cur_frame -= self.anim_length;
        }
        self.animate_frame();
        false
    }

    /// `SkelAnime_LoopPartial`.
    fn loop_partial(&mut self) -> bool {
        self.cur_frame += self.play_speed * UPDATE_RATE;
        if self.cur_frame < self.start_frame {
            self.cur_frame = (self.cur_frame - self.start_frame) + self.end_frame;
        } else if self.end_frame <= self.cur_frame {
            self.cur_frame = (self.cur_frame - self.end_frame) + self.start_frame;
        }
        self.animate_frame();
        false
    }

    /// `SkelAnime_Once`.
    fn once(&mut self) -> bool {
        if self.cur_frame == self.end_frame {
            if let Some(anim) = self.animation.clone() {
                Self::frame_data(&anim, self.cur_frame as i32, self.limb_count, &mut self.joint_table);
            }
            self.animate_frame();
            return true;
        }
        self.cur_frame += self.play_speed * UPDATE_RATE;
        if (self.cur_frame - self.end_frame) * self.play_speed > 0.0 {
            self.cur_frame = self.end_frame;
        } else if self.cur_frame < 0.0 {
            self.cur_frame += self.anim_length;
        } else if self.anim_length <= self.cur_frame {
            self.cur_frame -= self.anim_length;
        }
        self.animate_frame();
        false
    }

    /// `Animation_ChangeImpl`.
    pub fn change_impl(&mut self, animation: Anim, play_speed: f32, start_frame: f32, end_frame: f32, mode: u8, mut morph_frames: f32, taper: i8) {
        self.mode = mode;
        let other = !self.is(&animation.name) || start_frame != self.cur_frame;
        if morph_frames != 0.0 && other {
            if morph_frames < 0.0 {
                self.set_update();
                self.morph_table.clone_from(&self.joint_table);
                morph_frames = -morph_frames;
            } else {
                self.update = if taper != ANIMTAPER_NONE { UpdateFn::MorphTaper } else { UpdateFn::Morph };
                self.taper = taper;
                Self::frame_data(&animation, start_frame as i32, self.limb_count, &mut self.morph_table);
            }
            self.morph_weight = 1.0;
            self.morph_rate = 1.0 / morph_frames;
        } else {
            self.set_update();
            Self::frame_data(&animation, start_frame as i32, self.limb_count, &mut self.joint_table);
            self.morph_weight = 0.0;
        }
        self.anim_length = animation.length();
        self.animation = Some(animation);
        self.start_frame = start_frame;
        self.end_frame = end_frame;
        if self.mode >= ANIMMODE_LOOP_PARTIAL {
            self.cur_frame = 0.0;
        } else {
            self.cur_frame = start_frame;
            if self.mode <= ANIMMODE_LOOP_INTERP {
                self.end_frame = self.anim_length - 1.0;
            }
        }
        self.play_speed = play_speed;
    }

    /// `Animation_Change`.
    pub fn change(&mut self, animation: Anim, play_speed: f32, start_frame: f32, end_frame: f32, mode: u8, morph_frames: f32) {
        self.change_impl(animation, play_speed, start_frame, end_frame, mode, morph_frames, ANIMTAPER_NONE);
    }

    /// `Animation_ChangeByInfo`.
    pub fn change_by_info(&mut self, info: &AnimationInfo) {
        let frame_count = if info.frame_count > 0.0 { info.frame_count } else { info.animation.last_frame() };
        self.change(info.animation.clone(), info.play_speed, info.start_frame, frame_count, info.mode, info.morph_frames);
    }

    /// `Animation_OnFrame`.
    pub fn on_frame(&self, frame: f32) -> bool {
        on_frame_impl(self.cur_frame, self.play_speed, self.anim_length, frame, 1.0)
    }
}

/// `SkelAnime_InterpFrameTable` with `dst` also the start.
fn interp_frame_table(dst: &mut [[i16; 3]], target: &[[i16; 3]], weight: f32) {
    if weight < 1.0 {
        for (d, t) in dst.iter_mut().zip(target) {
            for k in 0..3 {
                let base = d[k];
                let diff = t[k].wrapping_sub(base);
                d[k] = ((diff as f32 * weight) as i32 as i16).wrapping_add(base);
            }
        }
    } else {
        for (d, t) in dst.iter_mut().zip(target) {
            *d = *t;
        }
    }
}

/// `Animation_OnFrameImpl`.
pub fn on_frame_impl(cur_frame: f32, play_speed: f32, anim_length: f32, mut frame: f32, update_rate: f32) -> bool {
    let speed = play_speed * update_rate;
    let mut prev = cur_frame - speed;
    if prev < 0.0 {
        prev += anim_length;
    } else if prev >= anim_length {
        prev -= anim_length;
    }
    if frame == 0.0 && speed > 0.0 {
        frame = anim_length;
    }
    let cur_diff = prev + speed - frame;
    let prev_diff = cur_diff - speed;
    cur_diff * speed >= 0.0 && prev_diff * speed < 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use eng_anim::anim::JointTable;

    /// A 10-frame animation whose joint 1 x rotation is 100 * frame.
    fn ramp() -> Anim {
        let frames = (0..10).map(|f| JointTable { rot: vec![[0; 3], [100 * f as i16, 0, 0]], face: 0 }).collect();
        Anim { name: "ramp".into(), data: Arc::new(StandardAnimation { frames }) }
    }

    #[test]
    fn a_loop_advances_one_frame_per_update_and_wraps() {
        let mut s = SkelAnimeStd::init_flex(1, Some(ramp()));
        // Animation_PlayLoop: frame 0, end at the last frame (9), length 10.
        assert_eq!((s.cur_frame, s.end_frame, s.anim_length), (0.0, 9.0, 10.0));
        for f in 1..=9 {
            s.update();
            assert_eq!(s.joint_table[1][0], 100 * f);
        }
        s.update();
        // SkelAnime_LoopFull: 10 >= animLength wraps to 0.
        assert_eq!((s.cur_frame, s.joint_table[1][0]), (0.0, 0));
    }

    #[test]
    fn a_loop_partial_starts_at_zero_and_wraps_into_its_range() {
        let mut s = SkelAnimeStd::init_flex(1, None);
        // En_Ko's ENKO_ANIM_BLOCKING_NOMORPH: LOOP_PARTIAL from 2 to 14 (here 2 to 8). Animation_ChangeImpl
        // sets curFrame 0 for the partial modes; the first update's 1 < startFrame 2 wraps to
        // (1 - 2) + 8 = 7.
        s.change(ramp(), 1.0, 2.0, 8.0, ANIMMODE_LOOP_PARTIAL, 0.0);
        assert_eq!(s.cur_frame, 0.0);
        s.update();
        assert_eq!((s.cur_frame, s.joint_table[1][0]), (7.0, 700));
    }

    #[test]
    fn a_negative_morph_blends_from_the_old_pose() {
        let mut s = SkelAnimeStd::init_flex(1, Some(ramp()));
        for _ in 0..5 {
            s.update();
        }
        // At frame 5 (500). A change with morphFrames -10 keeps the pose in morphTable and
        // blends it in by morphWeight, which starts at 1 and drops by 1/10 per update.
        let other = Anim { name: "other".into(), data: ramp().data };
        s.change(other, 1.0, 0.0, 9.0, ANIMMODE_LOOP, -10.0);
        assert_eq!((s.morph_weight, s.morph_rate), (1.0, 0.1));
        s.update();
        // Frame 1 (100); weight 0.9 towards the morph table's 500: 100 + (500 - 100) * 0.9.
        assert_eq!(s.joint_table[1][0], 460);
    }

    #[test]
    fn on_frame_is_true_once_as_the_frame_passes() {
        let mut s = SkelAnimeStd::init_flex(1, Some(ramp()));
        let mut hits = Vec::new();
        for i in 0..12 {
            s.update();
            if s.on_frame(3.0) {
                hits.push(i);
            }
        }
        assert_eq!(hits, vec![2]);
    }
}

/// What an `OverrideLimbDraw` does with a limb (`SkelAnime_DrawLimbOpa`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LimbDraw {
    /// It returns false: the limb's `Matrix_TranslateRotateZYX` (after the override's own
    /// `Matrix_*` calls, this matrix) and its list.
    Default(glam::Mat4),
    /// It returns true, having drawn the list itself at this matrix (relative to the parent's):
    /// the limb's transform isn't applied, and its children hang from the parent's matrix.
    Drawn(glam::Mat4),
}

/// `SkelAnime_DrawOpa`'s matrices: every limb's (the matrix its list is drawn with, in the
/// model's space), walking the skeleton as `SkelAnime_DrawLimbOpa` does. `override_limb` gets
/// the limb's 1-based index, its position and rotation to change, and says what it did;
/// `post_limb` gets the limb's 1-based index and the matrix the stack holds after it (the
/// limb's, or the parent's for a limb the override drew).
pub fn draw_opa_pose(
    skeleton: &eng_anim::skeleton::Skeleton,
    joints: &[[i16; 3]],
    mut override_limb: impl FnMut(usize, &mut glam::Vec3, &mut [i16; 3]) -> LimbDraw,
    mut post_limb: impl FnMut(usize, glam::Mat4),
) -> Vec<glam::Mat4> {
    use glam::{Mat4, Vec3};
    let n = skeleton.limbs.len();
    let mut draw = vec![Mat4::IDENTITY; n];
    // The matrix each limb's children start from.
    let mut stack = vec![Mat4::IDENTITY; n];
    for &l in &skeleton.draw_order {
        let l = l as usize;
        let limb = &skeleton.limbs[l];
        let parent = skeleton.parents[l].map(|p| stack[p as usize]).unwrap_or(Mat4::IDENTITY);
        let mut pos = if l == 0 {
            let r = joints.first().copied().unwrap_or([0; 3]);
            Vec3::new(r[0] as f32, r[1] as f32, r[2] as f32)
        } else {
            Vec3::new(limb.joint_pos[0] as f32, limb.joint_pos[1] as f32, limb.joint_pos[2] as f32)
        };
        let mut rot = joints.get(l + 1).copied().unwrap_or([0; 3]);
        match override_limb(l + 1, &mut pos, &mut rot) {
            LimbDraw::Default(pre) => {
                let m = parent * pre * eng_anim::skeleton::local_transform(pos, rot);
                draw[l] = m;
                stack[l] = m;
            }
            LimbDraw::Drawn(m) => {
                draw[l] = parent * m;
                stack[l] = parent;
            }
        }
        post_limb(l + 1, stack[l]);
    }
    draw
}
