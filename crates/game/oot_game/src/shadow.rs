//! Actor shadows (`z_actor.c`'s `ActorShape` shadow and `ActorShadow_*`).
//!
//! An actor's `ActorShape_Init` picks its shadow (`Actor::shape_shadow_draw`: none, the circle,
//! the white circle, the horse's, the feet's) with its `shadowScale`; `shadowAlpha` starts at 255.
//! `Actor_Draw` draws it after the actor's own draw and colour filter, from the actor's state as
//! the frame left it (`ShadowRs` in its render state): `ActorShadow_Draw` lays the display list
//! (`gCircleShadowDL`, `gHorseShadowDL`) on the floor polygon's plane under the actor
//! (`func_80038A28`), shrunk and faded with the height above it; `ActorShadow_DrawFeet` draws a
//! circle while in the air and, near the floor, one `gFootShadowDL` per foot per light that shines
//! down on it (`ActorShadow_DrawFoot`), laid on the floor under each foot (`func_800BFCB8`).
//!
//! The lists are drawn under `SETUPDL_44` (`G_RM_ZB_OVL_SURF2`: drawn over the floor) with the
//! prim colour (black, white for the white circle) at the shadow's alpha.
//!
//! Not modelled: `feetFloorFlag` (which feet touch the floor, written by the feet's draw): only one
//! of Player's cutscene actions reads it.

use eng_collision::bgcheck::{CollisionContext, DOWN_CHECK_FLOORS, DOWN_CHECK_GROUND_ONLY, DOWN_CHECK_WALLS_SIMPLE, IGNORE_NONE};
use eng_collision::bgcheck::PolyId;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::{Mat4, Vec3};

use crate::gbi::{ac, cc_ab, cc_c, cc_d, setup_dl};
use crate::pack::{BakeBody, BakeSegment, MeshBake, keys};
use crate::sys_matrix::MtxF;

/// `actor->shape.shadowDraw`: which of `z_actor.c`'s shadow functions an actor's
/// `ActorShape_Init` gave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadowDraw {
    /// `NULL`.
    #[default]
    None,
    /// `ActorShadow_DrawCircle`.
    Circle,
    /// `ActorShadow_DrawWhiteCircle`.
    WhiteCircle,
    /// `ActorShadow_DrawHorse`.
    Horse,
    /// `ActorShadow_DrawFeet`.
    Feet,
}

/// What an actor's shadow draws from (its render state's part): the shape's shadow, its floor, its
/// feet. Blended between game frames as the rest of the render state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowRs {
    pub draw: ShadowDraw,
    pub scale: f32,
    pub alpha: u8,
    pub floor_height: f32,
    pub floor_poly: Option<PolyId>,
    /// `shape.feetPos` (`Actor_SetFeetPos`): left, right.
    pub feet_pos: [Vec3; 2],
}

impl ShadowRs {
    pub fn lerp(&self, next: &ShadowRs, t: f32) -> ShadowRs {
        let pick = if t < 0.5 { self } else { next };
        ShadowRs {
            draw: pick.draw,
            scale: self.scale + (next.scale - self.scale) * t,
            alpha: (self.alpha as f32 + (next.alpha as f32 - self.alpha as f32) * t) as u8,
            floor_height: self.floor_height + (next.floor_height - self.floor_height) * t,
            floor_poly: pick.floor_poly,
            feet_pos: [self.feet_pos[0].lerp(next.feet_pos[0], t), self.feet_pos[1].lerp(next.feet_pos[1], t)],
        }
    }
}

pub const BAKE_CIRCLE: &str = "ActorShadow/circle";
pub const BAKE_HORSE: &str = "ActorShadow/horse";
pub const BAKE_FOOT: &str = "ActorShadow/foot";
/// The setup (`SETUPDL_44` and the combiner) and the prim colour.
const SEG_SETUP: u8 = 0x0A;
const SEG_COLOR: u8 = 0x0B;

/// `gCircleShadowDL`, `gHorseShadowDL`, `gFootShadowDL` (`gameplay_keep`), each after
/// `Gfx_SetupDL(SETUPDL_44)`, `gDPSetCombineLERP(0, 0, 0, PRIMITIVE, TEXEL0, 0, PRIMITIVE, 0, 0, 0,
/// 0, COMBINED, 0, 0, 0, COMBINED)` and the prim colour (dynamic).
pub fn bakes() -> Vec<MeshBake> {
    let mut setup = setup_dl::setup_dl_44();
    let c1 = [cc_ab::ZERO, cc_ab::ZERO, cc_c::ZERO, cc_d::PRIMITIVE, ac::TEXEL0, ac::ZERO, ac::PRIMITIVE, ac::ZERO];
    let c2 = [cc_ab::ZERO, cc_ab::ZERO, cc_c::ZERO, cc_d::COMBINED, ac::ZERO, ac::ZERO, ac::ZERO, ac::COMBINED];
    setup.combine_lerp(c1, c2);
    setup.end();
    let bake = |name: &str, dl: &str| MeshBake {
        name: name.into(),
        object: "gameplay_keep".into(),
        segments: vec![(SEG_SETUP, BakeSegment::Commands(setup.0.clone())), (SEG_COLOR, BakeSegment::DynamicColor { env: false, prim: true })],
        prelude: vec![SEG_SETUP, SEG_COLOR],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), dl.into())]),
    };
    vec![bake(BAKE_CIRCLE, "gCircleShadowDL"), bake(BAKE_HORSE, "gHorseShadowDL"), bake(BAKE_FOOT, "gFootShadowDL")]
}

/// A shadow list's draw under `m`, its prim colour `rgb` at `alpha`.
fn cmd(bake: &str, m: Mat4, rgb: [u8; 3], alpha: u8) -> DrawCmd {
    shadow_cmd(bake, m, rgb, alpha)
}

/// One of the shadow bakes (`BAKE_CIRCLE`, `BAKE_HORSE`, `BAKE_FOOT`) under `m`, its prim colour
/// `rgb` at `alpha`: for an actor that draws `gCircleShadowDL` itself after `SETUPDL_44` and the
/// shadow's combiner (`EnDekubaba_DrawShadow`, `func_80033C30`'s circle). Into the opaque list.
pub fn shadow_cmd(bake: &str, m: Mat4, rgb: [u8; 3], alpha: u8) -> DrawCmd {
    let mut sv = SegmentValues::default();
    sv.prim[SEG_COLOR as usize] = Some([rgb[0], rgb[1], rgb[2], alpha]);
    DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } }
}

/// `func_80033C30` (`z_actor.c`): the circle an actor's own draw puts under `pos` (`En_Heishi1`,
/// `En_Niw`, `En_Niw_Girl`, `En_Rd`, `En_Dnt_Nomal`): `gCircleShadowDL` on the plane of the floor
/// below (`BgCheck_EntityRaycastDown2` from 1 up; with none, at `pos`), scaled by `scale`'s x and z,
/// black at `alpha`, into the opaque list. It sets `SETUPDL_44` and the prim colour but not the
/// shadow's combiner: `SETUPDL_44`'s `G_CC_MODULATEIA_PRIM` with a black prim is the same black at
/// `TEXEL0` alpha × `alpha`, so it's the circle's bake.
// shared:
pub fn func_80033c30(col: &CollisionContext, pos: Vec3, scale: Vec3, alpha: u8) -> DrawCmd {
    let (y, poly) = col.entity_raycast_down(pos + Vec3::Y);
    let m = match poly {
        Some(p) => func_80038a28(col.poly(p).normal, Vec3::new(pos.x, y, pos.z)),
        None => Mat4::from_translation(pos),
    };
    shadow_cmd(BAKE_CIRCLE, m * Mat4::from_scale(Vec3::new(scale.x, 1.0, scale.z)), [0, 0, 0], alpha)
}

/// `CollisionPoly_GetNormalF`.
fn normal_f(n: [i16; 3]) -> [f32; 3] {
    n.map(|v| v as f32 * (1.0 / 32767.0))
}

/// `func_80038A28` (`z_bgcheck.c`): a matrix at `t` whose y axis is the floor's normal.
pub fn func_80038a28(normal: [i16; 3], t: Vec3) -> Mat4 {
    let [nx, ny, nz] = normal_f(normal);
    let mut xx = (1.0 - nx * nx).sqrt();
    let (zz, yz);
    if !eng_math::is_zero(xx) {
        let xx_inv = 1.0 / xx;
        zz = ny * xx_inv;
        yz = -(nz * xx_inv);
    } else {
        let z = (1.0 - ny * ny).sqrt();
        if !eng_math::is_zero(z) {
            let zz_inv = 1.0 / z;
            yz = nx * zz_inv;
            xx = -(nz * zz_inv);
        } else {
            yz = 0.0;
            xx = 0.0;
        }
        zz = z;
    }
    MtxF { xx, yx: -nx * zz, zx: nx * yz, wx: 0.0, xy: nx, yy: ny, zy: nz, wy: 0.0, xz: 0.0, yz, zz, wz: 0.0, xw: t.x, yw: t.y, zw: t.z, ww: 1.0 }.to_mat4()
}

/// `func_800BFCB8` (`z_play.c`): the floor under `pos` (`BgCheck_AnyRaycastDown1`) and a matrix on
/// its plane there. @bug (game): where `func_80038A28` divides by `sqrt(1 - nx²)`, this multiplies,
/// so the matrix leans a little off the plane on a slope; kept. With no floor the matrix is the
/// identity's rotation at the floor's `BGCHECK_Y_MIN`.
pub fn func_800bfcb8(col: &CollisionContext, pos: Vec3) -> (f32, Mat4) {
    let (floor_y, poly) = col.raycast_down(pos, IGNORE_NONE, DOWN_CHECK_WALLS_SIMPLE | DOWN_CHECK_FLOORS | DOWN_CHECK_GROUND_ONLY, 1.0);
    match poly.filter(|_| floor_y > eng_collision::bgcheck::BGCHECK_Y_MIN) {
        Some(p) => {
            let [nx, ny, nz] = normal_f(col.poly(p).normal);
            let temp1 = (1.0 - nx * nx).sqrt();
            let (temp2, temp3) = if temp1 != 0.0 { (ny * temp1, -nz * temp1) } else { (0.0, 0.0) };
            let m = MtxF { xx: temp1, yx: -nx * temp2, zx: nx * temp3, wx: 0.0, xy: nx, yy: ny, zy: nz, wy: 0.0, xz: 0.0, yz: temp3, zz: temp2, wz: 0.0, xw: pos.x, yw: floor_y, zw: pos.z, ww: 1.0 };
            (floor_y, m.to_mat4())
        }
        None => (floor_y, Mat4::from_translation(Vec3::new(pos.x, floor_y, pos.z))),
    }
}

/// `ActorShadow_Draw`: with a floor, from 50 below it to 500 above, `dlist` on the floor's plane
/// under the actor (turned to its yaw unless it's the circle), its scale `shadowScale × scale`
/// shrunk by the height (clamped to 0..150) over 350, its alpha `shadowAlpha` faded the same way.
fn actor_shadow_draw(s: &ShadowRs, pos: Vec3, scale: Vec3, yaw: i16, col: &CollisionContext, bake: &str, rgb: [u8; 3], out: &mut Vec<DrawCmd>) {
    let Some(poly) = s.floor_poly else { return };
    let temp1 = pos.y - s.floor_height;
    if !(-50.0..500.0).contains(&temp1) {
        return;
    }
    let temp1 = temp1.clamp(0.0, 150.0);
    let temp2 = 1.0 - temp1 * (1.0 / 350.0);
    let alpha = (s.alpha as f32 * temp2) as u32 as u8;
    let mut m = func_80038a28(col.poly(poly).normal, Vec3::new(pos.x, s.floor_height, pos.z));
    if bake != BAKE_CIRCLE {
        m *= Mat4::from_rotation_y(eng_math::binang_to_rad(yaw));
    }
    let k = temp2 * s.scale;
    m *= Mat4::from_scale(Vec3::new(k * scale.x, 1.0, k * scale.z));
    out.push(cmd(bake, m, rgb, alpha));
}

/// A light as `ActorShadow_DrawFeet` reads it from the actor's `Lights`: its direction and colour.
#[derive(Debug, Clone, Copy)]
pub struct FootLight {
    pub dir: [i8; 3],
    pub color: [u8; 3],
}

/// `ActorShadow_DrawFoot`: one foot's shadow for one light (`lightNum`, its strength: the colour's
/// sum by the light's downward part), stretched away from the light along its yaw, longer the lower
/// it is.
fn actor_shadow_draw_foot(light: &FootLight, floor_mtx: Mat4, light_num: i32, alpha: f32, scale_x: f32, mut scale_z: f32, out: &mut Vec<DrawCmd>) {
    let temp = light_num as f32 * 0.00005;
    let a = (alpha * if temp > 1.0 { 1.0 } else { temp }) as u32 as u8;
    let yaw = (light.dir[0] as f32).atan2(light.dir[2] as f32);
    scale_z *= 4.5 - (light.dir[1] as f32 * 0.035);
    if scale_z < 1.0 {
        scale_z = 1.0;
    }
    let m = floor_mtx * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::new(scale_x, 1.0, scale_x * scale_z));
    out.push(cmd(BAKE_FOOT, m, [0, 0, 0], a));
}

/// `Actor_Draw`'s `shape.shadowDraw`: the actor's shadow from its render state, into the opaque
/// list (`POLY_OPA_DISP`). `lights` is the actor's `Lights` as `ActorShadow_DrawFeet` reads them:
/// the bound point lights, then the environment's two directional lights.
pub fn draw(s: &ShadowRs, pos: Vec3, scale: Vec3, yaw: i16, col: &CollisionContext, lights: &[FootLight], out: &mut Vec<DrawCmd>) {
    match s.draw {
        ShadowDraw::None => {}
        ShadowDraw::Circle => actor_shadow_draw(s, pos, scale, yaw, col, BAKE_CIRCLE, [0, 0, 0], out),
        ShadowDraw::WhiteCircle => actor_shadow_draw(s, pos, scale, yaw, col, BAKE_CIRCLE, [255, 255, 255], out),
        ShadowDraw::Horse => actor_shadow_draw(s, pos, scale, yaw, col, BAKE_HORSE, [0, 0, 0], out),
        ShadowDraw::Feet => {
            let dist_to_floor = pos.y - s.floor_height;
            if dist_to_floor > 20.0 {
                // In the air: the circle, at 0.3 of the scale, fading in from 20 up.
                let alpha_ratio = (dist_to_floor - 20.0) * 0.02;
                let c = ShadowRs { scale: s.scale * 0.3, alpha: (s.alpha as f32 * alpha_ratio.min(1.0)) as u8, ..*s };
                actor_shadow_draw(&c, pos, scale, yaw, col, BAKE_CIRCLE, [0, 0, 0], out);
            }
            if dist_to_floor < 200.0 {
                let num_lights = lights.len().saturating_sub(2);
                for foot in s.feet_pos {
                    let (floor_height, floor_mtx) = func_800bfcb8(col, foot + Vec3::Y * 50.0);
                    let mut d = foot.y - floor_height;
                    if (-1.0..500.0).contains(&d) {
                        if d > 30.0 {
                            d = 30.0;
                        }
                        let shadow_alpha = s.alpha as f32 * (1.0 - d * (1.0 / 30.0));
                        let shadow_scale_z = 1.0 - d * (1.0 / (30.0 + 40.0));
                        let shadow_scale_x = shadow_scale_z * s.scale * scale.x;
                        let mut light_num_max = 0;
                        let strength = |l: &FootLight| (l.color[0] as i32 + l.color[1] as i32 + l.color[2] as i32) * (l.dir[1] as i32).abs();
                        for l in &lights[..num_lights] {
                            if l.dir[1] > 0 {
                                let n = strength(l);
                                if n > 0 {
                                    light_num_max += n;
                                    actor_shadow_draw_foot(l, floor_mtx, n, shadow_alpha, shadow_scale_x, shadow_scale_z, out);
                                }
                            }
                        }
                        for l in &lights[num_lights..] {
                            if l.dir[1] > 0 {
                                let n = strength(l) - light_num_max * 8;
                                if n > 0 {
                                    actor_shadow_draw_foot(l, floor_mtx, n, shadow_alpha, shadow_scale_x, shadow_scale_z, out);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_floor_s_matrix_is_upright() {
        let m = func_80038a28([0, 32767, 0], Vec3::new(1.0, 2.0, 3.0));
        let p = m.transform_point3(Vec3::new(10.0, 0.0, 5.0));
        assert!((p - Vec3::new(11.0, 2.0, 8.0)).length() < 1e-3, "{p}");
        // A floor sloping up towards +x (its normal leaning to -x): the circle lies on it.
        let n = Vec3::new(-0.5, 1.0, 0.0).normalize();
        let m = func_80038a28([(n.x * 32767.0) as i16, (n.y * 32767.0) as i16, 0], Vec3::ZERO);
        let up = m.transform_vector3(Vec3::Y);
        assert!((up - n).length() < 1e-3, "{up}");
        let along = m.transform_vector3(Vec3::X);
        assert!(along.dot(n).abs() < 1e-3, "{along}");
    }
}
