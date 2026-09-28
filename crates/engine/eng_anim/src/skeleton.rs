//! Skeletons (`SkeletonHeader` / `FlexSkeletonHeader`) and posing them from a joint table.

use glam::{Mat4, Vec3};

use crate::anim::JointTable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LimbType {
    Standard,
    Lod,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Limb {
    pub joint_pos: [i16; 3],
    pub child: Option<u8>,
    pub sibling: Option<u8>,
    /// Segmented display-list addresses: [near] for standard limbs, [near, far] for LOD limbs.
    pub dlists: Vec<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

impl Skeleton {
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
