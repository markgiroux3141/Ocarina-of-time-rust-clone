// N64 RDP colour-combiner emulation. Selector ids match oot_core::combiner::Input.

struct Globals {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    ambient: vec4<f32>,
};

struct Material {
    // Cycle 0: colour a,b,c,d / alpha a,b,c,d; cycle 1 likewise.
    sel: array<vec4<u32>, 4>,
    prim: vec4<f32>,
    env: vec4<f32>,
    // x: prim LOD fraction, y: alpha-test threshold (0 = off)
    params: vec4<f32>,
    // x: flag bits (1 lit, 2 texgen, 4 opaque-output), y: two-cycle
    flags: vec4<u32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var<uniform> m: Material;
@group(2) @binding(0) var t0: texture_2d<f32>;
@group(2) @binding(1) var s0: sampler;
@group(2) @binding(2) var t1: texture_2d<f32>;
@group(2) @binding(3) var s1: sampler;

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv0: vec2<f32>,
    @location(4) uv1: vec2<f32>,
};

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) shade: vec4<f32>,
    @location(1) uv0: vec2<f32>,
    @location(2) uv1: vec2<f32>,
};

@vertex
fn vs_main(v: VIn) -> VOut {
    var o: VOut;
    o.clip = g.view_proj * vec4<f32>(v.pos, 1.0);
    let flags = m.flags.x;
    if ((flags & 1u) != 0u) {
        // With G_LIGHTING the vertex colour bytes hold the normal; only alpha is a colour.
        let n = normalize(v.normal);
        let d = max(dot(n, normalize(g.light_dir.xyz)), 0.0);
        o.shade = vec4<f32>(clamp(g.ambient.rgb + g.light_color.rgb * d, vec3<f32>(0.0), vec3<f32>(1.0)), v.color.a);
    } else {
        o.shade = v.color;
    }
    if ((flags & 2u) != 0u) {
        // G_TEXTURE_GEN: spherical environment mapping from the view-space normal.
        let vn = normalize((g.view * vec4<f32>(v.normal, 0.0)).xyz);
        let uv = vn.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
        o.uv0 = uv;
        o.uv1 = uv;
    } else {
        o.uv0 = v.uv0;
        o.uv1 = v.uv1;
    }
    return o;
}

fn csel(i: u32, comb: vec4<f32>, a: vec4<f32>, b: vec4<f32>, shade: vec4<f32>) -> vec3<f32> {
    switch i {
        case 0u: { return comb.rgb; }
        case 1u: { return a.rgb; }
        case 2u: { return b.rgb; }
        case 3u: { return m.prim.rgb; }
        case 4u: { return shade.rgb; }
        case 5u: { return m.env.rgb; }
        case 6u: { return vec3<f32>(1.0); }
        case 12u: { return vec3<f32>(comb.a); }
        case 13u: { return vec3<f32>(a.a); }
        case 14u: { return vec3<f32>(b.a); }
        case 15u: { return vec3<f32>(m.prim.a); }
        case 16u: { return vec3<f32>(shade.a); }
        case 17u: { return vec3<f32>(m.env.a); }
        case 19u: { return vec3<f32>(m.params.x); }
        default: { return vec3<f32>(0.0); }
    }
}

fn asel(i: u32, comb: vec4<f32>, a: vec4<f32>, b: vec4<f32>, shade: vec4<f32>) -> f32 {
    switch i {
        case 0u: { return comb.a; }
        case 1u: { return a.a; }
        case 2u: { return b.a; }
        case 3u: { return m.prim.a; }
        case 4u: { return shade.a; }
        case 5u: { return m.env.a; }
        case 6u: { return 1.0; }
        case 19u: { return m.params.x; }
        default: { return 0.0; }
    }
}

fn run_cycle(cs: vec4<u32>, al: vec4<u32>, comb: vec4<f32>, a: vec4<f32>, b: vec4<f32>, shade: vec4<f32>) -> vec4<f32> {
    let rgb = (csel(cs.x, comb, a, b, shade) - csel(cs.y, comb, a, b, shade)) * csel(cs.z, comb, a, b, shade)
        + csel(cs.w, comb, a, b, shade);
    let alpha = (asel(al.x, comb, a, b, shade) - asel(al.y, comb, a, b, shade)) * asel(al.z, comb, a, b, shade)
        + asel(al.w, comb, a, b, shade);
    return clamp(vec4<f32>(rgb, alpha), vec4<f32>(0.0), vec4<f32>(1.0));
}

@fragment
fn fs_main(i: VOut) -> @location(0) vec4<f32> {
    // Always sample (unbound slots get a white texture) to stay in uniform control flow.
    let a = textureSample(t0, s0, i.uv0);
    let b = textureSample(t1, s1, i.uv1);
    var c = run_cycle(m.sel[0], m.sel[1], vec4<f32>(0.0), a, b, i.shade);
    if (m.flags.y != 0u) {
        c = run_cycle(m.sel[2], m.sel[3], c, a, b, i.shade);
    }
    if (m.params.y > 0.0 && c.a < m.params.y) {
        discard;
    }
    if ((m.flags.x & 4u) != 0u) {
        c.a = 1.0;
    }
    return c;
}

// Debug lines (bones, grid).
struct LIn {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
};
struct LOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_line(v: LIn) -> LOut {
    var o: LOut;
    o.clip = g.view_proj * vec4<f32>(v.pos, 1.0);
    o.color = v.color;
    return o;
}

@fragment
fn fs_line(i: LOut) -> @location(0) vec4<f32> {
    return i.color;
}
