//! Material uniforms: the combiner's selectors, prim/env colours, the alpha threshold and
//! flags, and the texture-coordinate offsets from dynamic segments.

use bytemuck::{Pod, Zeroable};
use eng_gfx::{BlendMode, FogOverride, MAX_POINT_LIGHTS, Material, PointLight, SegmentValues};

pub(crate) const MATERIAL_STRIDE: u64 = 256;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct MaterialUniform {
    pub(crate) sel: [[u32; 4]; 4],
    pub(crate) prim: [f32; 4],
    pub(crate) env: [f32; 4],
    pub(crate) params: [f32; 4],
    /// x: flag bits, y: two-cycle, z and w: the draw's fog multiplier and offset (as f32 bits)
    /// when `fog_color.w` is 1.
    pub(crate) flags: [u32; 4],
    /// Texture coordinate offsets from dynamic segments: slot 0 in xy, slot 1 in zw.
    pub(crate) uv_off: [f32; 4],
    /// The draw's point lights (`params.z` of them): each a direction (xyz), then a colour (rgb).
    pub(crate) lights: [[f32; 4]; 2 * MAX_POINT_LIGHTS],
    /// The draw's fog colour (`eng_gfx::DrawParams::fog`), w 1 when it has one.
    pub(crate) fog_color: [f32; 4],
}

pub(crate) fn material_uniform(m: &Material) -> MaterialUniform {
    material_uniform_with(m, None, &[], None)
}

/// Whether the material's output depends on the fog (`G_FOG` or the fog blender).
pub(crate) fn uses_fog(m: &Material) -> bool {
    m.fog_blend || m.geometry_mode & eng_gfx::G_FOG != 0
}

pub(crate) fn material_uniform_with(m: &Material, dynamic: Option<&SegmentValues>, point_lights: &[PointLight], fog: Option<&FogOverride>) -> MaterialUniform {
    let s = m.combiner.selectors();
    let (env, prim) = dynamic.map(|v| m.colors(v)).unwrap_or((m.env, m.prim));
    let uv = dynamic.map(|v| m.uv_offsets(v)).unwrap_or_default();
    let c = |c: [u8; 4]| c.map(|x| x as f32 / 255.0);
    let threshold = match m.blend {
        BlendMode::Cutout(t) => t as f32 / 255.0,
        _ => 0.0,
    };
    let mut flags = 0u32;
    if m.lit {
        flags |= 1;
    }
    if m.texgen {
        flags |= 2;
    }
    if m.blend != BlendMode::Translucent {
        flags |= 4;
    }
    if m.fog_blend {
        flags |= 8;
    }
    if m.geometry_mode & eng_gfx::G_FOG != 0 {
        flags |= 16;
    }
    let mut lights = [[0.0f32; 4]; 2 * MAX_POINT_LIGHTS];
    let n = if m.lit { point_lights.len().min(MAX_POINT_LIGHTS) } else { 0 };
    for (i, l) in point_lights.iter().take(n).enumerate() {
        lights[2 * i] = [l.dir[0] as f32, l.dir[1] as f32, l.dir[2] as f32, 0.0];
        lights[2 * i + 1] = [l.color[0] as f32 / 255.0, l.color[1] as f32 / 255.0, l.color[2] as f32 / 255.0, 0.0];
    }
    MaterialUniform {
        sel: [[s[0], s[1], s[2], s[3]], [s[4], s[5], s[6], s[7]], [s[8], s[9], s[10], s[11]], [s[12], s[13], s[14], s[15]]],
        prim: c(prim),
        env: c(env),
        params: [m.prim_lod_frac as f32 / 255.0, threshold, n as f32, 0.0],
        flags: [flags, m.two_cycle as u32, fog.map(|f| (f.multiplier as f32).to_bits()).unwrap_or(0), fog.map(|f| (f.offset as f32).to_bits()).unwrap_or(0)],
        uv_off: [uv[0].x, uv[0].y, uv[1].x, uv[1].y],
        lights,
        fog_color: fog.map(|f| [f.color[0] as f32 / 255.0, f.color[1] as f32 / 255.0, f.color[2] as f32 / 255.0, 1.0]).unwrap_or([0.0; 4]),
    }
}
