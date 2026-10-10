//! Curve skeletons (`z_fcurve_data_skelanime.c`) and their interpolation (`z_fcurve_data.c`):
//! skeletons whose limbs carry no position, animated by curves. Each limb has nine values
//! (three scales, three rotations, three positions); an animation gives each either a constant or
//! a list of knots to interpolate (`Curve_Interpolate`). A limb draws its first display list in
//! the opaque list and, at level of detail 1, its second in the translucent one.
//!
//! The importer reads the skeletons (`CurveSkeleton`, `keys::curve_skeleton`) and the animations
//! (`CurveAnimation`, `keys::curve_anim`) from the objects; `SkelCurve` is the C's struct with
//! its functions. `SkelCurve::draw` returns each limb's matrix and lists, which the actor turns
//! into draws of its bakes (the engine draws baked meshes only, ADR 0006).

use crate::sys_matrix::MtxF;
use glam::Vec3;

/// `LIMB_DONE`.
pub const LIMB_DONE: u8 = 0xFF;

/// `SkelCurveLimb`: its child and sibling, and its two display lists (segmented addresses, 0
/// for `NULL`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CurveLimb {
    pub child: u8,
    pub sibling: u8,
    pub dlists: [u32; 2],
}

/// `CurveSkeletonHeader`: its limbs, in order.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CurveSkeleton {
    pub limbs: Vec<CurveLimb>,
}

/// `CurveInterpKnot`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CurveInterpKnot {
    /// Only the bottom two bits are read (`FCURVE_INTERP_*`).
    pub flags: u16,
    pub abscissa: i16,
    pub left_gradient: i16,
    pub right_gradient: i16,
    pub ordinate: f32,
}

/// `CurveAnimationHeader`'s data: the knot counts (nine per limb), the knots, and the constants
/// (one per zero count). `constantData` is read as `u16` (`SkelCurve_Update`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CurveAnimation {
    pub knot_counts: Vec<u8>,
    pub interpolation_data: Vec<CurveInterpKnot>,
    pub constant_data: Vec<u16>,
    /// `unk_0C`: set but not read (always 1 in objects).
    pub unk_0c: i16,
    /// `frameCount`: not read.
    pub frame_count: i16,
}

/// `FCURVE_INTERP_NONE`, `FCURVE_INTERP_LINEAR` (`FCURVE_INTERP_CUBIC` is 0).
const FCURVE_INTERP_NONE: u16 = 1;
const FCURVE_INTERP_LINEAR: u16 = 2;

/// `Curve_CubicHermiteSpline`.
pub fn curve_cubic_hermite_spline(t: f32, interval: f32, y0: f32, y1: f32, m0: f32, m1: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let t3x2 = t3 * 2.0;
    let t2x3 = t2 * 3.0;
    let h00 = t3x2 - t2x3 + 1.0;
    let h01 = t2x3 - t3x2;
    let h10 = t3 - t2 * 2.0 + t;
    let h11 = t3 - t2;
    let mut ret = h00 * y0;
    ret += h01 * y1;
    ret += h10 * m0 * interval;
    ret += h11 * m1 * interval;
    ret
}

/// `Curve_Interpolate`: the value at `x` of the curve through `knots`.
pub fn curve_interpolate(x: f32, knots: &[CurveInterpKnot]) -> f32 {
    let n = knots.len();
    if x <= knots[0].abscissa as f32 {
        return knots[0].ordinate;
    } else if x >= knots[n - 1].abscissa as f32 {
        return knots[n - 1].ordinate;
    }
    let mut cur = 0;
    loop {
        let next = cur + 1;
        if x < knots[next].abscissa as f32 {
            let (a, b) = (knots[cur].abscissa as f32, knots[next].abscissa as f32);
            if knots[cur].flags & FCURVE_INTERP_NONE != 0 {
                return knots[cur].ordinate;
            } else if knots[cur].flags & FCURVE_INTERP_LINEAR != 0 {
                return knots[cur].ordinate + ((x - a) / (b - a)) * (knots[next].ordinate - knots[cur].ordinate);
            } else {
                let diff = b - a;
                let t = (x - a) / (b - a);
                return curve_cubic_hermite_spline(t, diff * (1.0 / 30.0), knots[cur].ordinate, knots[next].ordinate, knots[cur].right_gradient as f32, knots[next].left_gradient as f32);
            }
        }
        cur += 1;
    }
}

/// `SKELCURVE_SCALE_SCALE`, `SKELCURVE_SCALE_POSITION`.
pub const SKELCURVE_SCALE_SCALE: f32 = 1024.0;
pub const SKELCURVE_SCALE_POSITION: f32 = 100.0;

/// A C float-to-`s16` store: `(s16)(s32)value`.
fn to_s16(v: f32) -> i16 {
    v as i32 as i16
}

/// `DEG_TO_BINANG`: `(s16)(s32)(degrees * (0x8000 / 180.0f))`.
fn deg_to_binang(degrees: f32) -> i16 {
    to_s16(degrees * (32768.0f32 / 180.0))
}

/// `SkelCurve`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SkelCurve {
    pub limb_count: u8,
    /// `skeleton`: the limbs (`None` until `SkelCurve_Init`).
    pub skeleton: Option<std::sync::Arc<CurveSkeleton>>,
    pub animation: Option<std::sync::Arc<CurveAnimation>>,
    /// `unk_0C`: set, never read.
    pub unk_0c: f32,
    pub end_frame: f32,
    pub play_speed: f32,
    pub cur_frame: f32,
    /// `jointTable`: per limb, scale (×1024), rotation (binary angles), position (×100).
    pub joint_table: Option<Vec<[i16; 9]>>,
}

/// One list `SkelCurve_Draw` draws: the limb, which of its two lists (0 in the opaque list, 1 in
/// the translucent one), and its matrix (the draw's at the call times the limbs' down to it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveLimbDraw {
    pub limb: usize,
    pub list: usize,
    pub dlist: u32,
    pub mtx: MtxF,
}

impl SkelCurve {
    /// `SkelCurve_Clear`.
    pub fn clear(&mut self) {
        *self = SkelCurve::default();
    }

    /// `SkelCurve_Init`: the skeleton's limbs and a joint table for them (`ZELDA_ARENA_MALLOC`:
    /// its contents are whatever the arena held; zeros here, and `SkelCurve_Update` writes every
    /// entry before a draw reads it). Always true.
    pub fn init(&mut self, skeleton: std::sync::Arc<CurveSkeleton>) -> bool {
        self.limb_count = skeleton.limbs.len() as u8;
        self.joint_table = Some(vec![[0; 9]; self.limb_count as usize]);
        self.skeleton = Some(skeleton);
        self.cur_frame = 0.0;
        true
    }

    /// `SkelCurve_Destroy`: the joint table freed.
    pub fn destroy(&mut self) {
        self.joint_table = None;
    }

    /// `SkelCurve_SetAnim`.
    pub fn set_anim(&mut self, animation: std::sync::Arc<CurveAnimation>, arg2: f32, end_frame: f32, cur_frame: f32, play_speed: f32) {
        self.unk_0c = arg2 - self.play_speed;
        self.end_frame = end_frame;
        self.cur_frame = cur_frame;
        self.play_speed = play_speed;
        self.animation = Some(animation);
    }

    /// `SkelCurve_Update`: the frame stepped (`playSpeed × R_UPDATE_RATE × 0.5`) and held at
    /// `endFrame`, then every limb's nine values from the animation. True once past the end.
    pub fn update(&mut self, r_update_rate: i32) -> bool {
        let mut ret = false;
        self.cur_frame += self.play_speed * r_update_rate as f32 * 0.5;
        if (self.play_speed >= 0.0 && self.cur_frame > self.end_frame) || (self.play_speed < 0.0 && self.cur_frame < self.end_frame) {
            self.cur_frame = self.end_frame;
            ret = true;
        }
        let (Some(anim), Some(table)) = (self.animation.clone(), self.joint_table.as_mut()) else { return ret };
        let (mut counts, mut knot, mut constant) = (anim.knot_counts.iter(), 0usize, anim.constant_data.iter());
        for joint in table.iter_mut().take(self.limb_count as usize) {
            for vec_type in 0..3 {
                for coord in 0..3 {
                    let count = *counts.next().unwrap_or(&0) as usize;
                    let slot = &mut joint[vec_type * 3 + coord];
                    if count == 0 {
                        // transformValue = *constantData (a u16 as a float), stored as an s16.
                        *slot = to_s16(*constant.next().unwrap_or(&0) as f32);
                    } else {
                        let v = curve_interpolate(self.cur_frame, &anim.interpolation_data[knot..knot + count]);
                        knot += count;
                        *slot = match vec_type {
                            0 => to_s16(v * SKELCURVE_SCALE_SCALE),
                            1 => deg_to_binang(v),
                            _ => to_s16(v * SKELCURVE_SCALE_POSITION),
                        };
                    }
                }
            }
        }
        ret
    }

    /// `SkelCurve_Draw` from `mtx` (the current matrix): `SkelCurve_DrawLimb` from limb 0, each
    /// limb's place (`Matrix_TranslateRotateZYX`, then its scale), its lists by `lod` (0: the
    /// first, in the opaque list; 1: both, the second in the translucent list). `override_limb`
    /// is `OverrideCurveLimbDraw` (false skips the limb's lists, not its children); it may change
    /// the joint table, as the C's do. `post_limb` is `PostCurveLimbDraw`.
    pub fn draw(&mut self, mtx: MtxF, lod: i32, override_limb: &mut dyn FnMut(&mut SkelCurve, usize) -> bool, post_limb: &mut dyn FnMut(&SkelCurve, usize)) -> Vec<CurveLimbDraw> {
        let mut out = Vec::new();
        if self.joint_table.is_some() && self.skeleton.is_some() {
            self.draw_limb(0, mtx, lod, override_limb, post_limb, &mut out);
        }
        out
    }

    /// `SkelCurve_DrawLimb`.
    fn draw_limb(
        &mut self,
        limb_index: usize,
        mtx: MtxF,
        lod: i32,
        override_limb: &mut dyn FnMut(&mut SkelCurve, usize) -> bool,
        post_limb: &mut dyn FnMut(&SkelCurve, usize),
        out: &mut Vec<CurveLimbDraw>,
    ) {
        let Some(limb) = self.skeleton.as_ref().and_then(|s| s.limbs.get(limb_index)).copied() else { return };
        // Matrix_Push.
        let mut m = mtx;
        if override_limb(self, limb_index) {
            let j = self.joint_table.as_ref().map_or([0; 9], |t| t[limb_index]);
            let scale = Vec3::new(j[0] as f32 / SKELCURVE_SCALE_SCALE, j[1] as f32 / SKELCURVE_SCALE_SCALE, j[2] as f32 / SKELCURVE_SCALE_SCALE);
            let rot = [j[3], j[4], j[5]];
            let pos = Vec3::new(j[6] as f32, j[7] as f32, j[8] as f32);
            m.translate_rotate_zyx(pos, rot);
            m.scale(scale.x, scale.y, scale.z);
            match lod {
                0 => {
                    if limb.dlists[0] != 0 {
                        out.push(CurveLimbDraw { limb: limb_index, list: 0, dlist: limb.dlists[0], mtx: m });
                    }
                }
                1 => {
                    for (list, &dlist) in limb.dlists.iter().enumerate() {
                        if dlist != 0 {
                            out.push(CurveLimbDraw { limb: limb_index, list, dlist, mtx: m });
                        }
                    }
                }
                _ => log::debug!("FcSkeletonInfo_draw_child(): Not supported"),
            }
        }
        post_limb(self, limb_index);
        if limb.child != LIMB_DONE {
            self.draw_limb(limb.child as usize, m, lod, override_limb, post_limb, out);
        }
        // Matrix_Pop.
        if limb.sibling != LIMB_DONE {
            self.draw_limb(limb.sibling as usize, mtx, lod, override_limb, post_limb, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn knot(flags: u16, abscissa: i16, left: i16, right: i16, ordinate: f32) -> CurveInterpKnot {
        CurveInterpKnot { flags, abscissa, left_gradient: left, right_gradient: right, ordinate }
    }

    #[test]
    fn interpolate_holds_its_ends_and_follows_each_knots_kind() {
        let ks = [knot(FCURVE_INTERP_LINEAR, 0, 0, 0, 10.0), knot(FCURVE_INTERP_NONE, 10, 0, 0, 30.0), knot(0, 20, 0, 0, 50.0), knot(0, 30, 0, 0, 20.0)];
        // Outside the knots: the near end's value.
        assert_eq!(curve_interpolate(-5.0, &ks), 10.0);
        assert_eq!(curve_interpolate(40.0, &ks), 20.0);
        // Linear from the first: 10 + (5 / 10) × 20.
        assert_eq!(curve_interpolate(5.0, &ks), 20.0);
        // None from the second: its own value.
        assert_eq!(curve_interpolate(15.0, &ks), 30.0);
        // Cubic from the third, zero gradients: h00 × 50 + h01 × 20 at t = 0.5 (both 0.5).
        assert_eq!(curve_interpolate(25.0, &ks), 35.0);
    }

    #[test]
    fn hermite_basis_at_the_ends() {
        assert_eq!(curve_cubic_hermite_spline(0.0, 1.0, 3.0, 7.0, 100.0, 100.0), 3.0);
        assert_eq!(curve_cubic_hermite_spline(1.0, 1.0, 3.0, 7.0, 100.0, 100.0), 7.0);
        // h10 at t = 0.5 is 0.125 and h11 is -0.125: the gradients times the interval.
        assert_eq!(curve_cubic_hermite_spline(0.5, 2.0, 0.0, 0.0, 4.0, 0.0), 1.0);
    }

    #[test]
    fn update_scales_each_kind_of_value_and_steps_to_the_end() {
        // One limb: scale x curved (1.5 → 1536), rotation y curved (90° → 0x4000), position z
        // curved (2 → 200); the other six constants (a u16 0xFFFF stores as -1).
        let mut counts = vec![0u8; 9];
        counts[0] = 2;
        counts[4] = 2;
        counts[8] = 2;
        let flat = |v: f32| [knot(FCURVE_INTERP_LINEAR, 0, 0, 0, v), knot(FCURVE_INTERP_LINEAR, 10, 0, 0, v)];
        let mut knots = Vec::new();
        knots.extend(flat(1.5));
        knots.extend(flat(90.0));
        knots.extend(flat(2.0));
        let anim = std::sync::Arc::new(CurveAnimation { knot_counts: counts, interpolation_data: knots, constant_data: vec![1024, 1024, 0xFFFF, 7, 8, 9], unk_0c: 1, frame_count: 10 });
        let skel = std::sync::Arc::new(CurveSkeleton { limbs: vec![CurveLimb { child: LIMB_DONE, sibling: LIMB_DONE, dlists: [0x0600_0000, 0] }] });
        let mut sc = SkelCurve::default();
        assert!(sc.init(skel));
        sc.set_anim(anim, 1.0, 3.0, 1.0, 1.0);
        // unk_0C = 1 - playSpeed before (0).
        assert_eq!(sc.unk_0c, 1.0);
        // R_UPDATE_RATE 3: 1 + 1 × 3 × 0.5 = 2.5, not past 3.
        assert!(!sc.update(3));
        assert_eq!(sc.cur_frame, 2.5);
        assert_eq!(sc.joint_table.as_ref().unwrap()[0], [1536, 1024, 1024, -1, 0x4000, 7, 8, 9, 200]);
        // 4.0 > 3: held at the end.
        assert!(sc.update(3));
        assert_eq!(sc.cur_frame, 3.0);
    }

    #[test]
    fn draw_walks_children_under_their_parent_and_siblings_beside() {
        // Limb 0 (child 1), limb 1 (sibling 2), limb 2: 1 and 2 both under 0.
        let limbs = vec![
            CurveLimb { child: 1, sibling: LIMB_DONE, dlists: [0x0600_0010, 0x0600_0020] },
            CurveLimb { child: LIMB_DONE, sibling: 2, dlists: [0, 0x0600_0030] },
            CurveLimb { child: LIMB_DONE, sibling: LIMB_DONE, dlists: [0x0600_0040, 0] },
        ];
        let mut sc = SkelCurve::default();
        sc.init(std::sync::Arc::new(CurveSkeleton { limbs }));
        let t = sc.joint_table.as_mut().unwrap();
        t[0] = [1024, 1024, 1024, 0, 0, 0, 0, 100, 0];
        t[1] = [1024, 1024, 1024, 0, 0, 0, 5, 0, 0];
        t[2] = [1024, 1024, 1024, 0, 0, 0, 0, 0, 7];
        let mut seen = Vec::new();
        let draws = sc.draw(MtxF::IDENTITY, 1, &mut |_, _| true, &mut |_, i| seen.push(i));
        assert_eq!(seen, vec![0, 1, 2]);
        let got: Vec<_> = draws.iter().map(|d| (d.limb, d.list, d.dlist, d.mtx.xw, d.mtx.yw, d.mtx.zw)).collect();
        assert_eq!(got, vec![(0, 0, 0x0600_0010, 0.0, 100.0, 0.0), (0, 1, 0x0600_0020, 0.0, 100.0, 0.0), (1, 1, 0x0600_0030, 5.0, 100.0, 0.0), (2, 0, 0x0600_0040, 0.0, 100.0, 7.0)]);
        // LOD 0: first lists only; an override returning false skips a limb's lists.
        let draws = sc.draw(MtxF::IDENTITY, 0, &mut |_, i| i != 2, &mut |_, _| {});
        assert_eq!(draws.iter().map(|d| (d.limb, d.list)).collect::<Vec<_>>(), vec![(0, 0)]);
    }
}
