//! Builds a skinned draw list from a skeleton by running each limb's display list through
//! the interpreter in the game's draw order, with the RDP state carried across limbs.

use std::sync::Arc;

use anyhow::Result;
use eng_anim::skeleton::Skeleton;

use crate::gbi::{BoneId, DrawList, G_CULL_BACK, Interpreter, Segment};

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
    /// Per-limb segment rebindings while that limb's list runs (an `OverrideLimbDraw` that
    /// points segment 6 at another object for one limb, then back).
    pub limb_segments: Vec<(u8, u8, Segment)>,
    /// Segments whose colours are this draw's parameters (`Interpreter::dynamic_segments`).
    pub dynamic_segments: u16,
    /// Display lists run after the setup, before the limbs (the draw code's own commands).
    pub prelude: Vec<u32>,
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
            limb_segments: Vec::new(),
            dynamic_segments: 0,
            prelude: Vec::new(),
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
    it.dynamic_segments = opts.dynamic_segments;
    for &dl in &opts.prelude {
        it.run(dl);
    }
    for &limb in &skel.draw_order {
        let dl = effective(limb);
        if dl == 0 {
            continue;
        }
        it.set_bone(limb as BoneId);
        let rebound: Vec<(usize, Option<Segment>)> = opts
            .limb_segments
            .iter()
            .filter(|o| o.0 == limb)
            .map(|(_, seg, s)| {
                let i = *seg as usize & 0xF;
                (i, std::mem::replace(&mut it.segments[i], Some(s.clone())))
            })
            .collect();
        it.run(dl);
        for (i, old) in rebound.into_iter().rev() {
            it.segments[i] = old;
        }
    }
    Ok(it.draw)
}

/// A small display list as bytes, for a `Segment::Data` binding (e.g. a colour command that a
/// dynamic segment holds, or `gsDPSetRenderMode`), ending in `gsSPEndDisplayList`.
pub fn display_list_bytes(cmds: &[(u32, u32)]) -> Arc<[u8]> {
    let mut v = Vec::with_capacity((cmds.len() + 1) * 8);
    for &(w0, w1) in cmds.iter().chain(std::iter::once(&(0xDF00_0000, 0))) {
        v.extend_from_slice(&w0.to_be_bytes());
        v.extend_from_slice(&w1.to_be_bytes());
    }
    v.into()
}
