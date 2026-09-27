//! Skeletons (`SkeletonHeader` / `FlexSkeletonHeader`) and posing them from a joint table.

use anyhow::{Context, Result, bail};
use glam::{Mat4, Vec3};

use crate::anim::JointTable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum LimbType {
    Standard,
    Lod,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Limb {
    pub joint_pos: [i16; 3],
    pub child: Option<u8>,
    pub sibling: Option<u8>,
    /// Segmented display-list addresses: [near] for standard limbs, [near, far] for LOD limbs.
    pub dlists: Vec<u32>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Skeleton {
    pub limbs: Vec<Limb>,
    pub limb_type: LimbType,
    pub flex: bool,
    pub dlist_count: u8,
    /// Parent limb of each limb (None for the root).
    pub parents: Vec<Option<u8>>,
    /// Limbs in draw order (depth-first: self, children, then siblings), as the game walks them.
    pub draw_order: Vec<u8>,
}

fn be16(b: &[u8], o: usize) -> i16 {
    i16::from_be_bytes([b[o], b[o + 1]])
}
fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

impl Skeleton {
    /// Parse a skeleton header at `offset` within `file`, which is mapped to `segment`.
    pub fn parse(file: &[u8], segment: u8, offset: usize, limb_type: LimbType, flex: bool) -> Result<Skeleton> {
        let local = |addr: u32| -> Result<usize> {
            if (addr >> 24) as u8 != segment {
                bail!("pointer {addr:08X} is not in segment {segment:02X}");
            }
            let o = (addr & 0xFF_FFFF) as usize;
            if o >= file.len() {
                bail!("pointer {addr:08X} outside file");
            }
            Ok(o)
        };
        if offset + 8 > file.len() {
            bail!("skeleton header outside file");
        }
        let limb_table = local(be32(file, offset)).context("skeleton limb table")?;
        let limb_count = file[offset + 4] as usize;
        let dlist_count = if flex { file[offset + 8] } else { 0 };
        let mut limbs = Vec::with_capacity(limb_count);
        for i in 0..limb_count {
            let lo = local(be32(file, limb_table + i * 4)).with_context(|| format!("limb {i}"))?;
            let opt = |v: u8| if v == 0xFF { None } else { Some(v) };
            let n_dl = if limb_type == LimbType::Lod { 2 } else { 1 };
            limbs.push(Limb {
                joint_pos: [be16(file, lo), be16(file, lo + 2), be16(file, lo + 4)],
                child: opt(file[lo + 6]),
                sibling: opt(file[lo + 7]),
                dlists: (0..n_dl).map(|k| be32(file, lo + 8 + k * 4)).collect(),
            });
        }
        let mut parents = vec![None; limb_count];
        let mut draw_order = Vec::with_capacity(limb_count);
        // Iterative version of the game's recursive walk.
        let mut stack: Vec<(u8, Option<u8>)> = vec![(0, None)];
        while let Some((idx, parent)) = stack.pop() {
            let Some(limb) = limbs.get(idx as usize) else { bail!("limb index {idx} out of range") };
            if draw_order.contains(&idx) {
                bail!("limb graph has a cycle at {idx}");
            }
            parents[idx as usize] = parent;
            draw_order.push(idx);
            if let Some(s) = limb.sibling {
                stack.push((s, parent));
            }
            if let Some(c) = limb.child {
                stack.push((c, Some(idx)));
            }
        }
        Ok(Skeleton { limbs, limb_type, flex, dlist_count, parents, draw_order })
    }

    /// Mapping from index in the segment-0x0D matrix array to limb, following
    /// `SkelAnime_DrawFlexLod`: a matrix is allocated for every limb that has a display list.
    pub fn flex_matrix_map(&self, lod: usize) -> Vec<u16> {
        self.draw_order
            .iter()
            .filter(|&&l| self.limbs[l as usize].dlists.get(lod).copied().unwrap_or(0) != 0)
            .map(|&l| l as u16)
            .collect()
    }

    /// World-space (model-space) matrix of every limb for a joint table.
    ///
    /// joint_table[0] is the root translation; joint_table[i + 1] is limb i's rotation.
    /// Each limb applies `Translate(joint_pos) * Rz * Ry * Rx` (Matrix_TranslateRotateZYX).
    pub fn pose(&self, joints: &JointTable) -> Vec<Mat4> {
        let mut out = vec![Mat4::IDENTITY; self.limbs.len()];
        for &l in &self.draw_order {
            let limb = &self.limbs[l as usize];
            let parent = self.parents[l as usize].map(|p| out[p as usize]).unwrap_or(Mat4::IDENTITY);
            let pos = if l == 0 {
                let r = joints.rot.first().copied().unwrap_or([0; 3]);
                Vec3::new(r[0] as f32, r[1] as f32, r[2] as f32)
            } else {
                Vec3::new(limb.joint_pos[0] as f32, limb.joint_pos[1] as f32, limb.joint_pos[2] as f32)
            };
            let rot = joints.rot.get(l as usize + 1).copied().unwrap_or([0; 3]);
            out[l as usize] = parent * local_transform(pos, rot);
        }
        out
    }

    /// The rest pose (all rotations zero, root at its joint position).
    pub fn bind_pose(&self) -> Vec<Mat4> {
        let mut jt = JointTable::zeroed(self.limbs.len() + 1);
        let r = self.limbs[0].joint_pos;
        jt.rot[0] = r;
        self.pose(&jt)
    }
}

pub fn binang_to_rad(a: i16) -> f32 {
    a as f32 * (std::f32::consts::PI / 32768.0)
}

pub fn local_transform(pos: Vec3, rot: [i16; 3]) -> Mat4 {
    Mat4::from_translation(pos)
        * Mat4::from_rotation_z(binang_to_rad(rot[2]))
        * Mat4::from_rotation_y(binang_to_rad(rot[1]))
        * Mat4::from_rotation_x(binang_to_rad(rot[0]))
}
