//! `SkelAnime` for Link animations (`LinkAnimation_*` in `z_skelanime.c`) and the
//! `AnimationContext` request queue.
//!
//! In the game, Link's frames are DMA'd from `link_animetion` straight into the frame tables
//! (`AnimationContext_SetLoadFrame` starts the transfer; the dmamgr thread outranks the game
//! thread), while copy / interpolate / move-actor requests are queued and run after every
//! actor has updated (`AnimationContext_Update`). This port applies loads immediately and
//! queues the rest, in order. `AnimationContext_DisableQueue` suppresses the queued
//! copy/interp requests of the current frame, as in the game.

use glam::Vec3;

use crate::actor::Actor;
use crate::data::{AnimId, GameData};
use crate::math::{UPDATE_SCALE, cos_s, sin_s};

/// Root translation + 21 limbs.
pub const LIMB_COUNT: usize = 22;

pub const ANIMMODE_LOOP: u8 = 0;
pub const ANIMMODE_LOOP_INTERP: u8 = 1;
pub const ANIMMODE_ONCE: u8 = 2;

// `moveFlags` (`ANIM_FLAG_*`)
pub const ANIM_FLAG_UPDATEXZ: u16 = 1 << 0;
pub const ANIM_FLAG_UPDATEY: u16 = 1 << 1;
pub const ANIM_FLAG_NOMOVE: u16 = 1 << 4;

pub type FrameTable = [[i16; 3]; LIMB_COUNT];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table {
    Joint,
    Morph,
    Blend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateFn {
    Loop,
    Once,
    Morph,
}

#[derive(Debug, Clone, Copy)]
pub enum Request {
    CopyAll { dst: Table, src: Table },
    Interp { base: Table, other: Table, weight: f32 },
    CopyTrue { dst: Table, src: Table, flags: &'static [u8; LIMB_COUNT] },
    CopyFalse { dst: Table, src: Table, flags: &'static [u8; LIMB_COUNT] },
    MoveActor { y_scale: f32 },
    /// A copy from another SkelAnime's joint table (Player's `skelAnime2`) into this one's:
    /// `AnimationContext_SetCopyTrue` with a mask, or `SetCopyAll` without. The source is
    /// captured when queued; nothing writes it between then and the queue running.
    CopyExternal { src: FrameTable, mask: Option<[u8; LIMB_COUNT]> },
}

#[derive(Debug, Clone)]
pub struct SkelAnime {
    pub joint: FrameTable,
    pub morph: FrameTable,
    /// `this->blendTable` (Player owns it; kept here with the other tables).
    pub blend: FrameTable,
    /// Face index loaded with the joint table (the 2 bytes after the Vec3s).
    pub face: u16,
    pub animation: AnimId,
    pub cur_frame: f32,
    pub start_frame: f32,
    pub end_frame: f32,
    pub anim_length: f32,
    pub play_speed: f32,
    pub mode: u8,
    pub morph_weight: f32,
    pub morph_rate: f32,
    update: UpdateFn,
    pub move_flags: u16,
    pub prev_transl: [i16; 3],
    pub prev_rot: i16,
    pub base_transl: [i16; 3],
    queue: Vec<Request>,
    queue_disabled: bool,
}

/// `SkelAnime_InterpFrameTable`.
fn interp(dst: &mut FrameTable, start: &FrameTable, target: &FrameTable, weight: f32) {
    if weight < 1.0 {
        for i in 0..LIMB_COUNT {
            for k in 0..3 {
                let base = start[i][k];
                let diff = target[i][k].wrapping_sub(base);
                dst[i][k] = ((diff as f32 * weight) as i32 as i16).wrapping_add(base);
            }
        }
    } else {
        *dst = *target;
    }
}

impl SkelAnime {
    /// `SkelAnime_InitLink`: starts `anim` looping from frame 0.
    pub fn new_link(data: &GameData, anim: AnimId, base_transl: [i16; 3]) -> SkelAnime {
        let mut s = SkelAnime {
            joint: [[0; 3]; LIMB_COUNT],
            morph: [[0; 3]; LIMB_COUNT],
            blend: [[0; 3]; LIMB_COUNT],
            face: 0,
            animation: anim,
            cur_frame: 0.0,
            start_frame: 0.0,
            end_frame: 0.0,
            anim_length: 0.0,
            play_speed: 0.0,
            mode: 0,
            morph_weight: 0.0,
            morph_rate: 0.0,
            update: UpdateFn::Loop,
            move_flags: 0,
            prev_transl: [0; 3],
            prev_rot: 0,
            base_transl,
            queue: Vec::new(),
            queue_disabled: false,
        };
        s.change(data, anim, 1.0, 0.0, 0.0, ANIMMODE_LOOP, 0.0);
        s
    }

    fn table(&mut self, t: Table) -> &mut FrameTable {
        match t {
            Table::Joint => &mut self.joint,
            Table::Morph => &mut self.morph,
            Table::Blend => &mut self.blend,
        }
    }

    /// `AnimationContext_SetLoadFrame`: the frame lands in the table immediately (see module
    /// docs).
    fn load(&mut self, data: &GameData, anim: AnimId, frame: i32, dst: Table) {
        let a = &data.anims[anim];
        let f = &a.frames[(frame.max(0) as usize).min(a.frames.len() - 1)];
        let t = self.table(dst);
        for (i, r) in f.rot.iter().take(LIMB_COUNT).enumerate() {
            t[i] = *r;
        }
        if dst == Table::Joint {
            self.face = f.face;
        }
    }

    fn push(&mut self, r: Request) {
        self.queue.push(r);
    }

    /// `AnimationContext_DisableQueue`.
    pub fn disable_queue(&mut self) {
        self.queue_disabled = true;
    }

    /// `AnimationContext_Update` for this skeleton: runs the queued requests and resets the
    /// queue flags.
    pub fn run_queue(&mut self, actor: &mut Actor) {
        let queue = std::mem::take(&mut self.queue);
        let disabled = self.queue_disabled;
        for r in queue {
            match r {
                Request::CopyAll { dst, src } if !disabled => {
                    let s = *self.table(src);
                    *self.table(dst) = s;
                }
                Request::Interp { base, other, weight } if !disabled => {
                    let (b, o) = (*self.table(base), *self.table(other));
                    interp(self.table(base), &b, &o, weight);
                }
                Request::CopyTrue { dst, src, flags } if !disabled => {
                    let s = *self.table(src);
                    let d = self.table(dst);
                    for i in 0..LIMB_COUNT {
                        if flags[i] != 0 {
                            d[i] = s[i];
                        }
                    }
                }
                Request::CopyFalse { dst, src, flags } if !disabled => {
                    let s = *self.table(src);
                    let d = self.table(dst);
                    for i in 0..LIMB_COUNT {
                        if flags[i] == 0 {
                            d[i] = s[i];
                        }
                    }
                }
                Request::CopyExternal { src, mask } if !disabled => {
                    for i in 0..LIMB_COUNT {
                        if mask.is_none_or(|m| m[i] != 0) {
                            self.joint[i] = src[i];
                        }
                    }
                }
                Request::MoveActor { y_scale } => {
                    // `AnimationContext_MoveActor`
                    let d = self.update_translation(actor.shape_rot.y);
                    actor.world_pos.x += d.x * actor.scale.x;
                    actor.world_pos.y += d.y * actor.scale.y * y_scale;
                    actor.world_pos.z += d.z * actor.scale.z;
                }
                _ => {}
            }
        }
        self.queue_disabled = false;
    }

    /// `LinkAnimation_SetUpdateFunction`.
    fn set_update_function(&mut self) {
        self.update = if self.mode <= ANIMMODE_LOOP_INTERP { UpdateFn::Loop } else { UpdateFn::Once };
        self.morph_weight = 0.0;
    }

    /// `LinkAnimation_Update`: advances the animation; for play-once modes returns true on
    /// the frame the end is reached.
    pub fn update(&mut self, data: &GameData) -> bool {
        match self.update {
            UpdateFn::Loop => self.update_loop(data),
            UpdateFn::Once => self.update_once(data),
            UpdateFn::Morph => self.update_morph(),
        }
    }

    /// `LinkAnimation_Morph`.
    fn update_morph(&mut self) -> bool {
        let prev = self.morph_weight;
        self.morph_weight -= self.morph_rate * UPDATE_SCALE;
        if self.morph_weight <= 0.0 {
            self.set_update_function();
        }
        self.push(Request::Interp { base: Table::Joint, other: Table::Morph, weight: 1.0 - (self.morph_weight / prev) });
        false
    }

    /// `LinkAnimation_AnimateFrame`.
    fn animate_frame(&mut self, data: &GameData) {
        self.load(data, self.animation, self.cur_frame as i32, Table::Joint);
        if self.morph_weight != 0.0 {
            self.morph_weight -= self.morph_rate * UPDATE_SCALE;
            if self.morph_weight <= 0.0 {
                self.morph_weight = 0.0;
            } else {
                self.push(Request::Interp { base: Table::Joint, other: Table::Morph, weight: self.morph_weight });
            }
        }
    }

    /// `LinkAnimation_Loop`.
    fn update_loop(&mut self, data: &GameData) -> bool {
        self.cur_frame += self.play_speed * UPDATE_SCALE;
        if self.cur_frame < 0.0 {
            self.cur_frame += self.anim_length;
        } else if self.anim_length <= self.cur_frame {
            self.cur_frame -= self.anim_length;
        }
        self.animate_frame(data);
        false
    }

    /// `LinkAnimation_Once`.
    fn update_once(&mut self, data: &GameData) -> bool {
        if self.cur_frame == self.end_frame {
            self.animate_frame(data);
            return true;
        }
        self.cur_frame += self.play_speed * UPDATE_SCALE;
        if (self.cur_frame - self.end_frame) * self.play_speed > 0.0 {
            self.cur_frame = self.end_frame;
        } else if self.cur_frame < 0.0 {
            self.cur_frame += self.anim_length;
        } else if self.anim_length <= self.cur_frame {
            self.cur_frame -= self.anim_length;
        }
        self.animate_frame(data);
        false
    }

    /// `LinkAnimation_Change`. Positive `morph_frames` morph from the current pose to the
    /// new animation's start before playing; negative ones start playing at once, blending
    /// out of the old pose.
    #[allow(clippy::too_many_arguments)]
    pub fn change(&mut self, data: &GameData, anim: AnimId, play_speed: f32, start: f32, end: f32, mode: u8, morph_frames: f32) {
        self.mode = mode;
        if morph_frames != 0.0 && (anim != self.animation || start != self.cur_frame) {
            let mut mf = morph_frames;
            if mf < 0.0 {
                self.set_update_function();
                // `SkelAnime_CopyFrameTable` (synchronous)
                self.morph = self.joint;
                mf = -mf;
            } else {
                self.update = UpdateFn::Morph;
                self.load(data, anim, start as i32, Table::Morph);
            }
            self.morph_weight = 1.0;
            self.morph_rate = 1.0 / mf;
        } else {
            self.set_update_function();
            self.load(data, anim, start as i32, Table::Joint);
            self.morph_weight = 0.0;
        }
        self.animation = anim;
        self.start_frame = start;
        self.cur_frame = start;
        self.end_frame = end;
        self.anim_length = data.anims[anim].length();
        self.play_speed = play_speed;
    }

    /// `LinkAnimation_PlayOnce`.
    pub fn play_once(&mut self, data: &GameData, anim: AnimId) {
        self.play_once_set_speed(data, anim, 1.0);
    }
    /// `LinkAnimation_PlayOnceSetSpeed`.
    pub fn play_once_set_speed(&mut self, data: &GameData, anim: AnimId, speed: f32) {
        let last = data.anims[anim].last_frame();
        self.change(data, anim, speed, 0.0, last, ANIMMODE_ONCE, 0.0);
    }
    /// `LinkAnimation_PlayLoop`.
    pub fn play_loop(&mut self, data: &GameData, anim: AnimId) {
        self.play_loop_set_speed(data, anim, 1.0);
    }
    /// `LinkAnimation_PlayLoopSetSpeed`.
    pub fn play_loop_set_speed(&mut self, data: &GameData, anim: AnimId, speed: f32) {
        let last = data.anims[anim].last_frame();
        self.change(data, anim, speed, 0.0, last, ANIMMODE_LOOP, 0.0);
    }

    /// `LinkAnimation_CopyJointToMorph`.
    pub fn copy_joint_to_morph(&mut self) {
        self.push(Request::CopyAll { dst: Table::Morph, src: Table::Joint });
    }
    /// `LinkAnimation_LoadToMorph`.
    pub fn load_to_morph(&mut self, data: &GameData, anim: AnimId, frame: f32) {
        self.load(data, anim, frame as i32, Table::Morph);
    }
    /// `LinkAnimation_LoadToJoint`.
    pub fn load_to_joint(&mut self, data: &GameData, anim: AnimId, frame: f32) {
        self.load(data, anim, frame as i32, Table::Joint);
    }
    /// `LinkAnimation_InterpJointMorph`.
    pub fn interp_joint_morph(&mut self, weight: f32) {
        self.push(Request::Interp { base: Table::Joint, other: Table::Morph, weight });
    }
    /// `LinkAnimation_BlendToJoint`.
    #[allow(clippy::too_many_arguments)]
    pub fn blend_to_joint(&mut self, data: &GameData, a1: AnimId, f1: f32, a2: AnimId, f2: f32, weight: f32) {
        self.load(data, a1, f1 as i32, Table::Joint);
        self.load(data, a2, f2 as i32, Table::Blend);
        self.push(Request::Interp { base: Table::Joint, other: Table::Blend, weight });
    }
    /// `LinkAnimation_BlendToMorph`.
    #[allow(clippy::too_many_arguments)]
    pub fn blend_to_morph(&mut self, data: &GameData, a1: AnimId, f1: f32, a2: AnimId, f2: f32, weight: f32) {
        self.load(data, a1, f1 as i32, Table::Morph);
        self.load(data, a2, f2 as i32, Table::Blend);
        self.push(Request::Interp { base: Table::Morph, other: Table::Blend, weight });
    }
    /// `AnimationContext_SetCopyTrue` / `SetCopyAll` from another skeleton's joint table.
    pub fn request_copy_external(&mut self, src: &FrameTable, mask: Option<[u8; LIMB_COUNT]>) {
        self.push(Request::CopyExternal { src: *src, mask });
    }

    /// `AnimationContext_SetMoveActor`.
    pub fn request_move_actor(&mut self, y_scale: f32) {
        self.push(Request::MoveActor { y_scale });
    }

    /// `Animation_OnFrameImpl` / `LinkAnimation_OnFrame`: true on the update that crossed
    /// `frame`.
    pub fn on_frame(&self, frame: f32) -> bool {
        let speed = self.play_speed * UPDATE_SCALE;
        let mut prev = self.cur_frame - speed;
        if prev < 0.0 {
            prev += self.anim_length;
        } else if prev >= self.anim_length {
            prev -= self.anim_length;
        }
        let frame = if frame == 0.0 && speed > 0.0 { self.anim_length } else { frame };
        let cur_diff = prev + speed - frame;
        let prev_diff = cur_diff - speed;
        cur_diff * speed >= 0.0 && prev_diff * speed < 0.0
    }

    /// `SkelAnime_UpdateTranslation`: root motion since the last call, rotated into world
    /// space, and resets the root x/z (and y with `ANIM_FLAG_UPDATEY`) to `base_transl`.
    pub fn update_translation(&mut self, angle: i16) -> Vec3 {
        let mut d = Vec3::ZERO;
        if self.move_flags & ANIM_FLAG_NOMOVE == 0 {
            let (x, z) = (self.joint[0][0] as f32, self.joint[0][2] as f32);
            let (s, c) = (sin_s(angle), cos_s(angle));
            d.x = x * c + z * s;
            d.z = z * c - x * s;
            let (px, pz) = (self.prev_transl[0] as f32, self.prev_transl[2] as f32);
            let (s, c) = (sin_s(self.prev_rot), cos_s(self.prev_rot));
            d.x -= px * c + pz * s;
            d.z -= pz * c - px * s;
        }
        self.prev_rot = angle;
        self.prev_transl[0] = self.joint[0][0];
        self.joint[0][0] = self.base_transl[0];
        self.prev_transl[2] = self.joint[0][2];
        self.joint[0][2] = self.base_transl[2];
        if self.move_flags & ANIM_FLAG_UPDATEY != 0 {
            d.y = if self.move_flags & ANIM_FLAG_NOMOVE != 0 { 0.0 } else { (self.joint[0][1] as i32 - self.prev_transl[1] as i32) as f32 };
            self.prev_transl[1] = self.joint[0][1];
            self.joint[0][1] = self.base_transl[1];
        } else {
            d.y = 0.0;
            self.prev_transl[1] = self.joint[0][1];
        }
        self.move_flags &= !ANIM_FLAG_NOMOVE;
        d
    }
}
