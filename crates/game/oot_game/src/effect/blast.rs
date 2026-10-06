//! `Effect_Ss_Blast` (`ovl_Effect_Ss_Blast/z_eff_ss_blast.c`): a shockwave, the ring
//! `gEffShockwaveDL` lying on the floor under its position, spreading by `scaleStep` (slowing by
//! `scaleStepDecay` each frame) while its inner colour's alpha fades to 0 over its life.

use eng_collision::bgcheck::{BGCHECK_Y_MIN, CollisionContext, DOWN_CHECK_FLOORS, DOWN_CHECK_GROUND_ONLY, DOWN_CHECK_WALLS_SIMPLE, IGNORE_NONE};
use eng_math::step_to_s;
use glam::Vec3;

use super::{DrawCtx, EffectDraws, EffectSs, SEG_COLOR, SsDraw, SsUpdate, colored};
use crate::pack::{BakeBody, BakeSegment, MeshBake};
use crate::sys_matrix::MtxF;

// The overlay's regs.
const R_INNER_COLOR_R: usize = 0;
const R_INNER_COLOR_G: usize = 1;
const R_INNER_COLOR_B: usize = 2;
pub const R_INNER_COLOR_A: usize = 3;
const R_OUTER_COLOR_R: usize = 4;
const R_OUTER_COLOR_G: usize = 5;
const R_OUTER_COLOR_B: usize = 6;
const R_OUTER_COLOR_A: usize = 7;
pub const R_ALPHA_STEP: usize = 8;
pub const R_SCALE: usize = 9;
pub const R_SCALE_STEP: usize = 10;
pub const R_SCALE_STEP_DECAY: usize = 11;

const BAKE: &str = "EffectSs/blast";

/// `EffectSsBlastParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct BlastInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub inner_color: [u8; 4],
    pub outer_color: [u8; 4],
    pub scale: i16,
    pub scale_step: i16,
    pub scale_step_decay: i16,
    pub life: i16,
}

/// `EffectSsBlast_Init`: 5 above the position; the inner alpha's step `innerColor.a / life`
/// (integer division).
pub fn init(this: &mut EffectSs, p: &BlastInit) -> bool {
    this.pos = p.pos;
    this.pos.y += 5.0;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.gfx = Some(("gameplay_keep", "gEffShockwaveDL"));
    this.life = p.life;
    this.draw = Some(SsDraw::Blast);
    this.update = Some(SsUpdate::Blast);
    let r = &mut this.regs;
    r[R_INNER_COLOR_R] = p.inner_color[0] as i16;
    r[R_INNER_COLOR_G] = p.inner_color[1] as i16;
    r[R_INNER_COLOR_B] = p.inner_color[2] as i16;
    r[R_INNER_COLOR_A] = p.inner_color[3] as i16;
    r[R_OUTER_COLOR_R] = p.outer_color[0] as i16;
    r[R_OUTER_COLOR_G] = p.outer_color[1] as i16;
    r[R_OUTER_COLOR_B] = p.outer_color[2] as i16;
    r[R_OUTER_COLOR_A] = p.outer_color[3] as i16;
    r[R_ALPHA_STEP] = (p.inner_color[3] as i32 / p.life as i32) as i16;
    r[R_SCALE] = p.scale;
    r[R_SCALE_STEP] = p.scale_step;
    r[R_SCALE_STEP_DECAY] = p.scale_step_decay;
    true
}

/// `EffectSsBlast_Update`: the inner alpha down by its step to 0, the scale up by its step,
/// and the step (while not 0) down by its decay. (A decay that doesn't divide the step takes it
/// past 0 into shrinking, as the C does.)
pub fn update(this: &mut EffectSs) {
    let r = &mut this.regs;
    let step = r[R_ALPHA_STEP];
    step_to_s(&mut r[R_INNER_COLOR_A], 0, step);
    r[R_SCALE] = r[R_SCALE].wrapping_add(r[R_SCALE_STEP]);
    if r[R_SCALE_STEP] != 0 {
        r[R_SCALE_STEP] = r[R_SCALE_STEP].wrapping_sub(r[R_SCALE_STEP_DECAY]);
    }
}

/// `func_800BFCB8` (`z_play.c`): a matrix at `pos` on the floor below it
/// (`BgCheck_AnyRaycastDown1`), its y axis the floor's normal; with no floor, a matrix at `pos`
/// that flattens everything onto its y axis. Returns the matrix and the floor's height.
///
/// The C multiplies by `sqrtf(1 - nx²)` where `func_80038A28` divides by it: kept as written.
pub fn func_800bfcb8(col: &CollisionContext, pos: Vec3) -> (MtxF, f32) {
    let (floor_y, poly) = col.raycast_down(pos, IGNORE_NONE, DOWN_CHECK_WALLS_SIMPLE | DOWN_CHECK_FLOORS | DOWN_CHECK_GROUND_ONLY, 1.0);
    let mut mf = MtxF::IDENTITY;
    if floor_y > BGCHECK_Y_MIN
        && let Some(p) = poly
    {
        // COLPOLY_GET_NORMAL.
        let n = col.poly(p).normal;
        let (nx, ny, nz) = (n[0] as f32 * (1.0 / 32767.0), n[1] as f32 * (1.0 / 32767.0), n[2] as f32 * (1.0 / 32767.0));
        let temp1 = (1.0 - nx * nx).sqrt();
        let (temp2, temp3) = if temp1 != 0.0 { (ny * temp1, -nz * temp1) } else { (0.0, 0.0) };
        mf.xx = temp1;
        mf.yx = -nx * temp2;
        mf.zx = nx * temp3;
        mf.xy = nx;
        mf.yy = ny;
        mf.zy = nz;
        mf.yz = temp3;
        mf.zz = temp2;
        mf.wx = 0.0;
        mf.wy = 0.0;
        mf.xz = 0.0;
        mf.wz = 0.0;
        mf.xw = pos.x;
        mf.yw = floor_y;
        mf.zw = pos.z;
        mf.ww = 1.0;
    } else {
        mf.xy = 0.0;
        mf.zx = 0.0;
        mf.yx = 0.0;
        mf.xx = 0.0;
        mf.wz = 0.0;
        mf.xz = 0.0;
        mf.wy = 0.0;
        mf.wx = 0.0;
        mf.zz = 0.0;
        mf.yz = 0.0;
        mf.zy = 0.0;
        mf.yy = 1.0;
        mf.xw = pos.x;
        mf.yw = pos.y;
        mf.zw = pos.z;
        mf.ww = 1.0;
    }
    (mf, floor_y)
}

/// `gEffShockwaveDL` (which sets its own texture, combiner and render mode) after
/// `Gfx_SetupDL_25Xlu` (the bake's start), the colours dynamic.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: "gameplay_keep".into(),
        segments: vec![(SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true })],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffShockwaveDL".into())]),
    }]
}

/// `EffectSsBlast_Draw`: the outer colour as env, the inner as prim, on the floor under its
/// position (`func_800BFCB8`), scaled `rScale / 400`.
pub fn draw(this: &EffectSs, ctx: &DrawCtx<'_>, out: &mut EffectDraws) {
    let r = &this.regs;
    let scale = r[R_SCALE] as f32 * (1.0 / 400.0);
    let env = [r[R_OUTER_COLOR_R] as u8, r[R_OUTER_COLOR_G] as u8, r[R_OUTER_COLOR_B] as u8, r[R_OUTER_COLOR_A] as u8];
    let (mut mf, _) = func_800bfcb8(ctx.col, this.pos);
    let prim = [r[R_INNER_COLOR_R] as u8, r[R_INNER_COLOR_G] as u8, r[R_INNER_COLOR_B] as u8, r[R_INNER_COLOR_A] as u8];
    // Matrix_Put(&mf), Matrix_Scale(scale, scale, scale, MTXMODE_APPLY).
    mf.scale(scale, scale, scale);
    out.xlu.push(colored(BAKE, mf.to_mat4(), SEG_COLOR, prim, env));
}
