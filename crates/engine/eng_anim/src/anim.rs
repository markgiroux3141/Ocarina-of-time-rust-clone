//! Animations: the standard compressed `AnimationHeader` format used by most actors, and
//! Link's uncompressed per-frame format stored in the `link_animetion` file, both held as
//! per-frame joint tables. The binary decoders are in `oot_import::z64`.

/// A sampled pose: entry 0 is the root translation, entry i + 1 is limb i's rotation.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

/// Standard animation (`AnimationHeader`), decompressed into full per-frame tables.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StandardAnimation {
    pub frames: Vec<JointTable>,
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
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LinkAnimation {
    pub frames: Vec<JointTable>,
    pub data_offset: usize,
}

impl Animation for LinkAnimation {
    fn frame_count(&self) -> usize {
        self.frames.len()
    }
    fn sample(&self, frame: usize) -> JointTable {
        self.frames[frame.min(self.frames.len() - 1)].clone()
    }
}
