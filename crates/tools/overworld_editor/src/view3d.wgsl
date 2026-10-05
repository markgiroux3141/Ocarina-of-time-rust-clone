// The editor's 3D view: texture x baked vertex colour, as the N64 (and pd-walk) draws it, with
// a light distance fog into the sky colour.

struct Globals {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    fog: vec4<f32>,
    sky: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) dist: f32,
};

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var o: VsOut;
    o.clip = g.view_proj * vec4<f32>(v.pos, 1.0);
    o.uv = v.uv;
    o.color = v.color;
    o.dist = distance(v.pos, g.eye.xyz);
    return o;
}

fn fogged(rgb: vec3<f32>, dist: f32) -> vec3<f32> {
    let f = clamp((dist - g.fog.x) / max(g.fog.y - g.fog.x, 1.0), 0.0, 1.0);
    return mix(rgb, g.sky.rgb, f * 0.85);
}

@fragment
fn fs_opaque(in: VsOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    return vec4<f32>(fogged(t.rgb * in.color.rgb, in.dist), 1.0);
}

@fragment
fn fs_cutout(in: VsOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    if (t.a * in.color.a < 0.5) {
        discard;
    }
    return vec4<f32>(fogged(t.rgb * in.color.rgb, in.dist), 1.0);
}

@fragment
fn fs_blend(in: VsOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    let a = t.a * in.color.a;
    if (a < 0.02) {
        discard;
    }
    return vec4<f32>(fogged(t.rgb * in.color.rgb, in.dist), a);
}
