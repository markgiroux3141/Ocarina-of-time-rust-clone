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

/// A joint whose angles move more than this between two frames is blended as a rotation
/// (`slerp_zyx`): animations can write the same orientation as two Euler triples far apart (x
/// and z half a turn on, y mirrored), which a per-angle blend would swing through a wrong pose.
const SLERP_ABOVE: i32 = 0x2000;

fn rad(a: i16) -> f32 {
    a as f32 * (std::f32::consts::PI / 32768.0)
}

fn binang(r: f32) -> i16 {
    (r * (32768.0 / std::f32::consts::PI)).round() as i32 as i16
}

/// `a` to `b` by `t` as rotations (`Matrix_RotateZYX`'s order: Z, then Y, then X), the short
/// way, back as a ZYX triple.
fn slerp_zyx(a: [i16; 3], b: [i16; 3], t: f32) -> [i16; 3] {
    use glam::{EulerRot, Quat};
    let qa = Quat::from_euler(EulerRot::ZYX, rad(a[2]), rad(a[1]), rad(a[0]));
    let qb = Quat::from_euler(EulerRot::ZYX, rad(b[2]), rad(b[1]), rad(b[0]));
    let (z, y, x) = qa.slerp(qb, t).to_euler(EulerRot::ZYX);
    [binang(x), binang(y), binang(z)]
}

impl JointTable {
    pub fn zeroed(n: usize) -> JointTable {
        JointTable { rot: vec![[0; 3]; n], face: 0 }
    }

    /// Per-component interpolation with wrap-around, like `SkelAnime_InterpFrameTable`; a
    /// joint whose angles are far apart is blended as a rotation instead (`slerp_zyx`). The game
    /// draws only its own frames; this is the renderer's in-between.
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
                if i > 0 && (0..3).any(|k| (b[k].wrapping_sub(a[k]) as i32).abs() > SLERP_ABOVE) {
                    o = slerp_zyx(*a, *b, t);
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

#[cfg(test)]
mod blend_tests {
    use super::*;
    use glam::{EulerRot, Quat};

    fn quat(r: [i16; 3]) -> Quat {
        Quat::from_euler(EulerRot::ZYX, rad(r[2]), rad(r[1]), rad(r[0]))
    }

    #[test]
    fn a_flipped_joint_stays_on_its_orientation() {
        // Link's limb 10 in link_demo_furimuki from frame 12 to 13.5: one orientation as two
        // triples (x and z half a turn on, y mirrored) about 7 degrees apart.
        let (a, b) = ([30271i16, -2343, 16501], [-1275i16, -30433, -16288]);
        assert!(quat(a).angle_between(quat(b)) < 0.2);
        let ja = JointTable { rot: vec![[0; 3], a], face: 0 };
        let jb = JointTable { rot: vec![[0; 3], b], face: 0 };
        for k in 0..=4 {
            let m = ja.lerp(&jb, k as f32 / 4.0).rot[1];
            assert!(quat(m).angle_between(quat(a)) < 0.2, "t {}: {m:?}", k as f32 / 4.0);
        }
        // A small move keeps the per-angle blend.
        let jc = JointTable { rot: vec![[0; 3], [a[0] + 100, a[1], a[2]]], face: 0 };
        assert_eq!(ja.lerp(&jc, 0.5).rot[1], [a[0] + 50, a[1], a[2]]);
    }
}
