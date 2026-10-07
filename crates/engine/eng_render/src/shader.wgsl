// N64 RDP colour-combiner emulation. Selector ids match oot_core::combiner::Input.

struct Globals {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    light2_dir: vec4<f32>,
    light2_color: vec4<f32>,
    ambient: vec4<f32>,
    fog_color: vec4<f32>,
    // x: fog multiplier (fm), y: fog offset (fo), z/w: near/far of the game's projection
    fog: vec4<f32>,
};

struct Material {
    // Cycle 0: colour a,b,c,d / alpha a,b,c,d; cycle 1 likewise.
    sel: array<vec4<u32>, 4>,
    prim: vec4<f32>,
    env: vec4<f32>,
    // x: prim LOD fraction, y: alpha-test threshold (0 = off)
    params: vec4<f32>,
    // x: flag bits (1 lit, 2 texgen, 4 opaque-output, 8 fog), y: two-cycle, z/w: the draw's
    // own fog multiplier and offset (f32 bits) when fog_color.w is 1
    flags: vec4<u32>,
    // Dynamic-segment texture scroll: slot 0 in xy, slot 1 in zw.
    uv_off: vec4<f32>,
    // params.z point lights bound for the draw: a direction, then a colour, each.
    lights: array<vec4<f32>, 6>,
    // The draw's own fog colour (gDPSetFogColor before it), w 1 when it has one.
    fog_color: vec4<f32>,
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
    @location(3) fog: f32,
    // The clip position's z and w (the far plane's test in fs_main).
    @location(4) zw: vec2<f32>,
};

@vertex
fn vs_main(v: VIn) -> VOut {
    var o: VOut;
    o.clip = g.view_proj * vec4<f32>(v.pos, 1.0);
    o.zw = o.clip.zw;
    let flags = m.flags.x;
    if ((flags & 1u) != 0u) {
        // With G_LIGHTING the vertex colour bytes hold the normal; only alpha is a colour.
        // F3DEX2: ambient + sum over directional lights of colour * max(0, n . l).
        let n = normalize(v.normal);
        let d1 = max(dot(n, g.light_dir.xyz), 0.0);
        let d2 = max(dot(n, g.light2_dir.xyz), 0.0);
        var lit = g.ambient.rgb + g.light_color.rgb * d1 + g.light2_color.rgb * d2;
        // The draw's point lights, bound as directional lights (Lights_BindPoint).
        let nl = u32(m.params.z);
        for (var i = 0u; i < nl; i++) {
            let dir = m.lights[2u * i].xyz;
            if (dot(dir, dir) > 0.0) {
                lit += m.lights[2u * i + 1u].rgb * max(dot(n, normalize(dir)), 0.0);
            }
        }
        o.shade = vec4<f32>(clamp(lit, vec3<f32>(0.0), vec3<f32>(1.0)), v.color.a);
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
        o.uv0 = v.uv0 + m.uv_off.xy;
        o.uv1 = v.uv1 + m.uv_off.zw;
    }
    // Vertex fog: OpenGL-style NDC depth of the game's projection, times fm plus fo, in
    // 1/256ths (gSPFogPosition maps fogNear..1000 onto 0..256).
    let depth = max(-(g.view * vec4<f32>(v.pos, 1.0)).z, 0.001);
    let nf = g.fog.zw;
    let z_ndc = (nf.y + nf.x) / (nf.y - nf.x) - 2.0 * nf.x * nf.y / ((nf.y - nf.x) * depth);
    var fm = g.fog.x;
    var fo = g.fog.y;
    if (m.fog_color.w > 0.5) {
        fm = bitcast<f32>(m.flags.z);
        fo = bitcast<f32>(m.flags.w);
    }
    o.fog = clamp((z_ndc * fm + fo) / 256.0, 0.0, 1.0);
    if ((flags & 16u) != 0u && (fm != 0.0 || fo != 0.0)) {
        // With G_FOG the RSP writes the fog factor into the vertex alpha.
        o.shade.a = o.fog;
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
    // The game's microcode is F3DZEX2's NoN variant (graph.c: gspF3DZEX2_NoN_fifo): nothing is
    // clipped at the near plane, only the far one. The pipelines draw with unclipped depth
    // (`DEPTH_CLIP_CONTROL`: a depth nearer than the near plane is clamped to it, per sample);
    // past the far plane a pixel is dropped here. (Without the feature the hardware clips both,
    // and this never fires.)
    if (i.zw.x > i.zw.y) {
        discard;
    }
    if ((m.flags.x & 8u) != 0u) {
        // G_RM_FOG_SHADE_A: fog colour weighted by the (fog-replaced) shade alpha.
        var fog_rgb = g.fog_color.rgb;
        if (m.fog_color.w > 0.5) {
            fog_rgb = m.fog_color.rgb;
        }
        c = vec4<f32>(mix(c.rgb, fog_rgb, i.fog), c.a);
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

// Clip-space triangles (the letterbox bars): the position is already in clip space.
@vertex
fn vs_fill(v: LIn) -> LOut {
    var o: LOut;
    o.clip = vec4<f32>(v.pos, 1.0);
    o.color = v.color;
    return o;
}

@fragment
fn fs_line(i: LOut) -> @location(0) vec4<f32> {
    return i.color;
}
