//! Animations: the standard compressed `AnimationHeader` format used by most actors, and
//! Link's uncompressed per-frame format stored in the `link_animetion` file.

use anyhow::{Result, bail};

/// A sampled pose: entry 0 is the root translation, entry i + 1 is limb i's rotation.
#[derive(Debug, Clone, PartialEq)]
pub struct JointTable {
    pub rot: Vec<[i16; 3]>,
    /// Link only: packed eye (low nibble) and mouth (high nibble) indices, 1-based; 0 = default.
    pub face: u16,
}

impl JointTable {
    pub fn zeroed(n: usize) -> JointTable {
        JointTable { rot: vec![[0; 3]; n], face: 0 }
    }

    /// Per-component interpolation with wrap-around, like `SkelAnime_InterpFrameTable`.
    pub fn lerp(&self, other: &JointTable, t: f32) -> JointTable {
        let rot = self
            .rot
            .iter()
            .zip(&other.rot)
            .enumerate()
            .map(|(i, (a, b))| {
                let mut o = [0i16; 3];
                for k in 0..3 {
                    if i == 0 {
                        // Root translation: plain lerp.
                        o[k] = (a[k] as f32 + (b[k] as f32 - a[k] as f32) * t) as i16;
                    } else {
                        let diff = b[k].wrapping_sub(a[k]);
                        o[k] = a[k].wrapping_add((diff as f32 * t) as i16);
                    }
                }
                o
            })
            .collect();
        JointTable { rot, face: if t < 0.5 { self.face } else { other.face } }
    }

    pub fn eye_index(&self) -> Option<usize> {
        let e = (self.face & 0xF) as i32 - 1;
        (e >= 0).then_some(e as usize)
    }
    pub fn mouth_index(&self) -> Option<usize> {
        let m = ((self.face >> 4) & 0xF) as i32 - 1;
        (m >= 0).then_some(m as usize)
    }
}

pub trait Animation {
    fn frame_count(&self) -> usize;
    fn sample(&self, frame: usize) -> JointTable;

    /// Sample at a fractional frame, optionally looping from the last frame back to the first.
    fn sample_smooth(&self, frame: f32, looping: bool) -> JointTable {
        let n = self.frame_count().max(1);
        let f0 = (frame.floor() as isize).rem_euclid(n as isize) as usize;
        let f1 = if f0 + 1 < n { f0 + 1 } else if looping { 0 } else { f0 };
        let t = frame - frame.floor();
        let a = self.sample(f0);
        if t <= 1e-4 || f1 == f0 { a } else { a.lerp(&self.sample(f1), t) }
    }
}

fn be16(b: &[u8], o: usize) -> i16 {
    i16::from_be_bytes([b[o], b[o + 1]])
}
fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// Standard animation (`AnimationHeader`), decompressed into full per-frame tables.
#[derive(Debug, Clone)]
pub struct StandardAnimation {
    pub frames: Vec<JointTable>,
}

impl StandardAnimation {
    /// `limb_count` is the skeleton's limb count; the animation has limb_count + 1 joint entries.
    pub fn parse(file: &[u8], segment: u8, offset: usize, limb_count: usize) -> Result<StandardAnimation> {
        let local = |addr: u32| -> Result<usize> {
            if (addr >> 24) as u8 != segment {
                bail!("pointer {addr:08X} is not in segment {segment:02X}");
            }
            Ok((addr & 0xFF_FFFF) as usize)
        };
        if offset + 0x10 > file.len() {
            bail!("animation header outside file");
        }
        let frame_count = be16(file, offset).max(1) as usize;
        let data = local(be32(file, offset + 4))?;
        let indices = local(be32(file, offset + 8))?;
        let static_max = u16::from_be_bytes([file[offset + 12], file[offset + 13]]) as usize;
        let joints = limb_count + 1;
        let rd = |i: usize| -> i16 {
            let o = data + i * 2;
            if o + 2 <= file.len() { be16(file, o) } else { 0 }
        };
        let mut frames = Vec::with_capacity(frame_count);
        for f in 0..frame_count {
            let mut jt = JointTable::zeroed(joints);
            for j in 0..joints {
                let io = indices + j * 6;
                if io + 6 > file.len() {
                    bail!("joint index table outside file");
                }
                for k in 0..3 {
                    let idx = u16::from_be_bytes([file[io + k * 2], file[io + k * 2 + 1]]) as usize;
                    jt.rot[j][k] = if idx >= static_max { rd(idx + f) } else { rd(idx) };
                }
            }
            frames.push(jt);
        }
        Ok(StandardAnimation { frames })
    }
}

impl Animation for StandardAnimation {
    fn frame_count(&self) -> usize {
        self.frames.len()
    }
    fn sample(&self, frame: usize) -> JointTable {
        self.frames[frame.min(self.frames.len() - 1)].clone()
    }
}

/// Number of joint entries Link's animations store per frame (root + 21 limbs).
pub const LINK_ANIM_JOINTS: usize = 22;
pub const LINK_ANIM_FRAME_BYTES: usize = LINK_ANIM_JOINTS * 6 + 2;

/// Link animation (`LinkAnimationHeader` in gameplay_keep pointing into link_animetion).
#[derive(Debug, Clone)]
pub struct LinkAnimation {
    pub frames: Vec<JointTable>,
    pub data_offset: usize,
}

impl LinkAnimation {
    /// `header` is the LinkAnimationHeader bytes (frame count + segment-7 pointer).
    pub fn parse(header: &[u8], link_animetion: &[u8]) -> Result<LinkAnimation> {
        let frame_count = be16(header, 0).max(1) as usize;
        let ptr = be32(header, 4);
        if ptr >> 24 != 0x07 {
            bail!("Link animation pointer {ptr:08X} is not in segment 07");
        }
        let data_offset = (ptr & 0xFF_FFFF) as usize;
        Self::from_data(link_animetion, data_offset, frame_count)
    }

    pub fn from_data(link_animetion: &[u8], data_offset: usize, frame_count: usize) -> Result<LinkAnimation> {
        let end = data_offset + frame_count * LINK_ANIM_FRAME_BYTES;
        if end > link_animetion.len() {
            bail!("Link animation data {data_offset:X}..{end:X} outside link_animetion");
        }
        let frames = (0..frame_count)
            .map(|f| {
                let base = data_offset + f * LINK_ANIM_FRAME_BYTES;
                let rot = (0..LINK_ANIM_JOINTS)
                    .map(|j| {
                        let o = base + j * 6;
                        [be16(link_animetion, o), be16(link_animetion, o + 2), be16(link_animetion, o + 4)]
                    })
                    .collect();
                let face = u16::from_be_bytes([
                    link_animetion[base + LINK_ANIM_JOINTS * 6],
                    link_animetion[base + LINK_ANIM_JOINTS * 6 + 1],
                ]);
                JointTable { rot, face }
            })
            .collect();
        Ok(LinkAnimation { frames, data_offset })
    }
}

impl Animation for LinkAnimation {
    fn frame_count(&self) -> usize {
        self.frames.len()
    }
    fn sample(&self, frame: usize) -> JointTable {
        self.frames[frame.min(self.frames.len() - 1)].clone()
    }
}
