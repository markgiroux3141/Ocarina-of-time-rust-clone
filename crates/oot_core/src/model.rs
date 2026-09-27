//! Builds a skinned draw list from a skeleton by running each limb's display list through
//! the interpreter in the game's draw order, with the RDP state carried across limbs.

use std::sync::Arc;

use anyhow::Result;

use crate::gbi::{BoneId, DrawList, G_CULL_BACK, Interpreter, Segment};
use crate::skeleton::Skeleton;

/// Engine-side display list bound to segment 0x0C by Player: `gsSPSetGeometryMode(G_CULL_BACK)`.
pub fn cull_back_builtin() -> Segment {
    Segment::Builtin(vec![(0xD9FF_FFFF, G_CULL_BACK), (0xDF00_0000, 0)])
}

#[derive(Clone)]
pub struct Binding {
    pub segment: u8,
    pub buf: Arc<[u8]>,
    pub base: usize,
}

pub struct BuildOptions {
    pub lod: usize,
    pub env_color: Option<[u8; 4]>,
    pub bindings: Vec<Binding>,
    /// Segment holding the flex-skeleton matrix array (0x0D in OoT).
    pub matrix_segment: u8,
    pub builtin_segments: Vec<(u8, Segment)>,
    /// Per-limb display-list replacements, like an `OverrideLimbDraw` callback setting
    /// `*dList` (0 = draw nothing).
    pub limb_dlists: Vec<(u8, u32)>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        BuildOptions {
            lod: 0,
            env_color: None,
            bindings: Vec::new(),
            matrix_segment: 0x0D,
            builtin_segments: vec![(0x0C, cull_back_builtin())],
            limb_dlists: Vec::new(),
        }
    }
}

pub fn build_draw_list(skel: &Skeleton, opts: &BuildOptions) -> Result<DrawList> {
    let mut it = Interpreter::new();
    for b in &opts.bindings {
        it.segments[b.segment as usize & 0xF] = Some(Segment::Data { buf: b.buf.clone(), base: b.base });
    }
    for (seg, s) in &opts.builtin_segments {
        it.segments[*seg as usize & 0xF] = Some(s.clone());
    }
    let original = |limb: u8| skel.limbs[limb as usize].dlists.get(opts.lod).copied().unwrap_or(0);
    let effective = |limb: u8| opts.limb_dlists.iter().rev().find(|o| o.0 == limb).map(|o| o.1).unwrap_or(original(limb));
    if skel.flex {
        // SkelAnime_DrawFlexLimbLod allocates a matrix when either the limb's own DL or the
        // override is non-NULL, so overrides can shift the map.
        let map = skel.draw_order.iter().filter(|&&l| original(l) != 0 || effective(l) != 0).map(|&l| l as u16).collect();
        it.segments[opts.matrix_segment as usize & 0xF] = Some(Segment::Matrices(map));
    }
    it.apply_setup_dl_25();
    if let Some(env) = opts.env_color {
        it.set_env_color(env);
    }
    for &limb in &skel.draw_order {
        let dl = effective(limb);
        if dl == 0 {
            continue;
        }
        it.set_bone(limb as BoneId);
        it.run(dl);
    }
    Ok(it.draw)
}
