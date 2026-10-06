//! `Effect_Ss_Stick` (`ovl_Effect_Ss_Stick/z_eff_ss_stick.c`): the broken half of a Deku Stick
//! (child Link) or of the Giant's Knife's blade (adult Link) flying off when it breaks
//! (`EffectSsStick_Spawn`, from Player's `func_80842AC4` and `func_80842B7C`): thrown up at 26
//! and out at 6 along its yaw, falling at 4 a frame, gone after 20 frames.

use eng_gfx::{DrawCmd, MeshKey};
use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SsDraw, SsSpawn, SsUpdate};
use crate::pack::keys;
use crate::sys_matrix::MtxF;

// The overlay's regs.
const R_OBJECT_SLOT: usize = 0;
const R_YAW: usize = 1;

/// `StickDrawInfo`, by `linkAge`: the adult's broken blade, the child's stick.
const DRAW_INFO: [(&str, &str); 2] = [("object_link_boy", "gLinkAdultBrokenGiantsKnifeBladeDL"), ("object_link_child", "gLinkChildLinkDekuStickDL")];

/// `EffectSsStickInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct StickInit {
    pub pos: Vec3,
    pub yaw: i16,
    /// `gSaveContext.save.linkAge` as the init reads it: true for adult Link.
    pub adult: bool,
}

/// `EffectSsStick_Init`: the age's piece (its object's slot, `Object_GetSlot`), 20 frames.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &StickInit) -> bool {
    let (object, dlist) = DRAW_INFO[usize::from(!p.adult)];
    this.regs[R_OBJECT_SLOT] = s.objects.and_then(|o| o.get_index(object_id(object))).map(|b| b as i16).unwrap_or(-1);
    this.gfx = Some((object, dlist));
    this.pos = p.pos;
    this.vec = p.pos;
    this.regs[R_YAW] = p.yaw;
    this.velocity.x = eng_math::sin_s(p.yaw) * 6.0;
    this.velocity.z = eng_math::cos_s(p.yaw) * 6.0;
    this.life = 20;
    this.draw = Some(SsDraw::Stick);
    this.update = Some(SsUpdate::Stick);
    this.velocity.y = 26.0;
    this.accel.y = -4.0;
    true
}

/// `OBJECT_LINK_BOY` and `OBJECT_LINK_CHILD` (`object_table.h`).
fn object_id(object: &str) -> i16 {
    if object == "object_link_boy" { 0x0014 } else { 0x0015 }
}

/// `EffectSsStick_Update`: nothing (`EffectSs_Update` moves it).
pub fn update(_this: &mut EffectSs) {}

/// `EffectSsStick_Draw`: at its position; the child's stick squashed to a quarter along its
/// length (`Matrix_Scale(0.01, 0.0025, 0.01)`) and turned by its yaw, the adult's blade at 0.01
/// spinning by `play->state.frames * 10000` about z.
pub fn draw(this: &EffectSs, state_frames: u32, out: &mut EffectDraws) {
    let Some((file, symbol)) = this.gfx else { return };
    let child = file == DRAW_INFO[1].0;
    let scale = if child { Vec3::new(0.01, 0.0025, 0.01) } else { Vec3::splat(0.01) };
    let roll = if child { 0 } else { (state_frames as i32).wrapping_mul(10000) as i16 };
    let mut mf = MtxF::from_mat4(Mat4::from_translation(this.pos) * Mat4::from_scale(scale));
    mf.rotate_zyx(0, this.regs[R_YAW], roll);
    let m = mf.to_mat4();
    out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(file, symbol)), m));
}
