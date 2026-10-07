//! `Effect_Ss_Kakera` (`ovl_Effect_Ss_Kakera/z_eff_ss_kakera.c`): fragments, drawn with the
//! display list the spawner gives (a crate's planks, a bush's leaves, a rock's chips), tumbling,
//! slowed by the air, pulled by gravity and the forces `rReg0` picks, bouncing off the ground
//! or stopping on Player's floor or falling past it.
//!
//! Its regs, as the overlay names them: `rReg0` the forces (bits 0..1 a swirl round `vec`,
//! 2..4 a push up or down, 5..6 a pull to `vec`, 7..10 how the forces fade with the distance:
//! each 0 for none), `rGravity` (1/256 a frame), `rPitch` and `rYaw` (hundredths of a radian),
//! `rReg4` the flags (bits 0..1 the collision mode: 0 none, 1 bounce, 2 test only; bits 2..3
//! the size, the bounce sphere's radius and a margin under Player's floor; bit 4 stop on
//! Player's floor rather than fall 600 past it; bits 5..6 the tumble speed; bit 7 drawn
//! translucent), `rReg5` and `rReg6` the air's linear and quadratic drag (1/1024), `rScale`
//! (1/256), `rReg8` the bounces left, `rReg9` the forces' and the drag's jitter (1/1024),
//! `rObjId` and `rObjectSlot` the object its list is in (`KAKERA_OBJECT_DEFAULT` for the
//! keeps), `rColorIdx` the primitive colour (`KAKERA_COLOR_*`).
//!
//! Each display list a ported caller draws is baked (`bakes()`) with its file on segment 6 and a
//! dynamic primitive colour after `Gfx_SetupDL_25Opa` (the Xlu setup's commands are the same).
//!
//! The overlay's tables are one block of data in the C (`D_809AA530` to `D_809AA5B0`), and some
//! indices run past a table into the next: those reads are kept (`DATA`). No ported caller sets
//! `rReg0`, so the forces are reached only by the tests.

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SsDraw, SsGfx, SsSpawn, SsUpdate};
use crate::object_ctx::ObjectContext;
use crate::pack::{BakeBody, BakeSegment, MeshBake, keys};
use crate::play::{PlayState, Rand};

// The overlay's regs.
pub const R_REG0: usize = 0;
pub const R_GRAVITY: usize = 1;
pub const R_PITCH: usize = 2;
pub const R_YAW: usize = 3;
pub const R_REG4: usize = 4;
pub const R_REG5: usize = 5;
pub const R_REG6: usize = 6;
pub const R_SCALE: usize = 7;
pub const R_REG8: usize = 8;
pub const R_REG9: usize = 9;
pub const R_OBJ_ID: usize = 10;
pub const R_OBJECT_SLOT: usize = 11;
pub const R_COLOR_IDX: usize = 12;

/// `KAKERA_OBJECT_DEFAULT`.
pub const KAKERA_OBJECT_DEFAULT: i16 = -1;
/// `KakeraColorIndex`.
pub const KAKERA_COLOR_NONE: i16 = -1;
pub const KAKERA_COLOR_WHITE: i16 = 0;
pub const KAKERA_COLOR_BROWN: i16 = 1;

/// `OBJECT_GAMEPLAY_KEEP`, `OBJECT_GAMEPLAY_FIELD_KEEP`, `OBJECT_GAMEPLAY_DANGEON_KEEP`
/// (`object_table.h`): always loaded, so not checked.
pub const OBJECT_GAMEPLAY_KEEP: i16 = 0x0001;
pub const OBJECT_GAMEPLAY_FIELD_KEEP: i16 = 0x0002;
pub const OBJECT_GAMEPLAY_DANGEON_KEEP: i16 = 0x0003;

/// `EffectSsKakera_Draw`'s `colors`: white, brown.
const COLORS: [[u8; 3]; 2] = [[255, 255, 255], [235, 170, 130]];

/// The overlay's data block, in its order: `D_809AA530` (10, at 0), `D_809AA558` (2, at 10),
/// `D_809AA560` (7, at 12), `D_809AA57C` (3, at 19). An index past one table reads the next.
const DATA: [f32; 22] = [
    // D_809AA530: the distances the forces fade from (func_809A9DEC, func_809A9E28).
    1.0, 100.0, 40.0, 5.0, 100.0, 40.0, 5.0, 100.0, 40.0, 5.0, //
    // D_809AA558: the swirl's strength (func_809A9E88).
    0.05, 1.0, //
    // D_809AA560: the push up or down (func_809A9F10).
    4.0, 0.1, 0.3, 0.9, -0.1, -0.3, -0.9, //
    // D_809AA57C: the pull's strength (func_809A9F4C).
    0.1, 1.0, 6.0,
];
const D_809AA530: usize = 0;
const D_809AA558: usize = 10;
const D_809AA560: usize = 12;
const D_809AA57C: usize = 19;
/// `D_809AA5B0`: the bounce sphere's radius by size. (Size 3 reads past it, into the overlay's
/// read-only data: not reached by any caller; 40 here.)
const D_809AA5B0: [f32; 3] = [10.0, 20.0, 40.0];

/// The bakes' segment for the primitive colour.
const SEG_PRIM: u8 = SEG_COLOR;

/// `EffectSsKakeraInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct KakeraInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub unk_18: Vec3,
    pub gravity: i16,
    pub unk_26: i16,
    pub unk_28: i16,
    pub unk_2a: i16,
    pub unk_2c: i16,
    pub scale: i16,
    pub unk_30: i16,
    pub unk_32: i16,
    pub life: i32,
    pub color_idx: i16,
    pub obj_id: i16,
    pub dlist: SsGfx,
}

/// The display lists the ported callers spawn fragments with: `(file, symbol)`.
pub const KAKERA_DLISTS: [(&str, &str); 8] = [
    // Obj_Kibako2.
    ("object_kibako2", "gLargeCrateFragmentDL"),
    // Obj_Lift.
    ("object_d_lift", "gCollapsingPlatformDL"),
    // En_Kusa.
    ("gameplay_keep", "gCuttableShrubStalkDL"),
    ("gameplay_keep", "gCuttableShrubTipDL"),
    // En_Ishi.
    ("gameplay_field_keep", "gFieldKakeraDL"),
    ("gameplay_field_keep", "gSilverRockFragmentsDL"),
    // En_Goroiwa.
    ("gameplay_keep", "gBoulderFragmentsDL"),
    // Obj_Bombiwa.
    ("object_bombiwa", "object_bombiwa_DL_0009E0"),
];

/// A fragment list's bake.
pub fn bake_name(symbol: &str) -> String {
    format!("Effect_Ss_Kakera/{symbol}")
}

/// Each list of `KAKERA_DLISTS` after `Gfx_SetupDL_25Opa` and a dynamic `gDPSetPrimColor`
/// (`EffectSsKakera_Draw`'s, for a colour index of 0 or more; the lists that set their own
/// colour override it).
pub fn bakes() -> Vec<MeshBake> {
    KAKERA_DLISTS
        .iter()
        .map(|&(file, symbol)| MeshBake {
            name: bake_name(symbol),
            object: file.into(),
            segments: vec![(SEG_PRIM, BakeSegment::DynamicColor { env: false, prim: true })],
            prelude: vec![SEG_PRIM],
            body: BakeBody::DLists(vec![(file.into(), symbol.into())]),
        })
        .collect()
}

/// `func_809A9BA8`: the object's slot (`Object_GetSlot`); without it loaded, the fragment's
/// life ends and it isn't drawn.
fn check_object(this: &mut EffectSs, objects: Option<&ObjectContext>) {
    let slot = objects.and_then(|o| o.get_index(this.regs[R_OBJ_ID]));
    this.regs[R_OBJECT_SLOT] = slot.map_or(-1, |s| s as i16);
    if !slot.is_some_and(|s| objects.is_some_and(|o| o.is_loaded(s))) {
        this.life = 0;
        this.draw = None;
    }
}

/// `EffectSsKakera_Init`: priority 101; the list, its object checked unless it's a keep's; a
/// random pitch and yaw (0 to 32767).
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &KakeraInit) -> bool {
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.life = p.life as i16;
    this.priority = 101;
    if p.dlist.is_some() {
        this.gfx = p.dlist;
        let obj_id = p.obj_id;
        if obj_id == OBJECT_GAMEPLAY_KEEP || obj_id == OBJECT_GAMEPLAY_FIELD_KEEP || obj_id == OBJECT_GAMEPLAY_DANGEON_KEEP {
            this.regs[R_OBJ_ID] = KAKERA_OBJECT_DEFAULT;
        } else {
            this.regs[R_OBJ_ID] = p.obj_id;
            check_object(this, s.objects);
        }
    } else {
        // "shape_model is NULL": LogUtils_HungupThread, the game stops.
        log::error!("Effect_Ss_Kakera: shape_model is NULL (the game hangs here)");
    }
    // Set after func_809A9BA8, so an unloaded object's fragment keeps its draw (with life 0, it
    // goes in the next EffectSs_UpdateAll, before any draw).
    this.draw = Some(SsDraw::Kakera);
    this.update = Some(SsUpdate::Kakera);
    this.vec = p.unk_18;
    let r = &mut this.regs;
    r[R_REG0] = p.unk_2c;
    r[R_GRAVITY] = p.gravity;
    r[R_PITCH] = (s.rand.zero_one() * 32767.0) as i16;
    r[R_YAW] = (s.rand.zero_one() * 32767.0) as i16;
    r[R_REG4] = p.unk_26;
    r[R_REG5] = p.unk_28;
    r[R_REG6] = p.unk_2a;
    r[R_SCALE] = p.scale;
    r[R_REG8] = p.unk_30;
    r[R_REG9] = p.unk_32;
    r[R_COLOR_IDX] = p.color_idx;
    true
}

/// `func_809A9818` (`randomD_sectionUniformity`): `arg0` give or take `arg1`, uniformly.
pub fn func_809a9818(rand: &mut Rand, arg0: f32, arg1: f32) -> f32 {
    if arg1 < 0.0 {
        log::debug!("The range is negative!! (randomD_sectionUniformity)");
    }
    let temp_f2 = rand.zero_one() * arg1;
    ((temp_f2 * 2.0) - arg1) + arg0
}

/// `func_809A9C10`: the air's drag, linear (`rReg5`) and quadratic (`rReg6`), against the
/// velocity less a jitter of `rReg9 × 4` (one `Rand` a component).
/// `@bug (game)`: z takes y's drag (`temp_f0`, `temp_f2`), only its sign test is z's.
fn func_809a9c10(this: &mut EffectSs, rand: &mut Rand) {
    let temp_f18 = this.regs[R_REG5] as f32 / 1024.0;
    let temp_f20 = this.regs[R_REG6] as f32 / 1024.0;
    let temp_f14 = (this.regs[R_REG9] as f32 / 1024.0) * 4.0;
    let mut temp_f2 = this.velocity.x - func_809a9818(rand, 0.0, temp_f14);
    let temp_f16 = this.velocity.y - func_809a9818(rand, 0.0, temp_f14);
    let temp_f12 = this.velocity.z - func_809a9818(rand, 0.0, temp_f14);
    if temp_f2 > 0.0 {
        this.velocity.x -= (temp_f2 * temp_f18) + (temp_f2 * temp_f2 * temp_f20);
    } else {
        this.velocity.x -= (temp_f2 * temp_f18) - (temp_f2 * temp_f2 * temp_f20);
    }
    let temp_f0 = temp_f16 * temp_f18;
    temp_f2 = temp_f16 * temp_f16 * temp_f20;
    if temp_f16 > 0.0 {
        this.velocity.y -= temp_f0 + temp_f2;
    } else {
        this.velocity.y -= temp_f0 - temp_f2;
    }
    if temp_f12 > 0.0 {
        this.velocity.z -= temp_f0 + temp_f2;
    } else {
        this.velocity.z -= temp_f0 - temp_f2;
    }
}

/// `func_809A9DEC`: 1, or `D_809AA530[arg1] / arg0` beyond that distance.
fn func_809a9dec(arg0: f32, arg1: usize) -> f32 {
    let d = DATA[D_809AA530 + arg1];
    if d < arg0 { d / arg0 } else { 1.0 }
}

/// `func_809A9E28` (and `func_809A9E68`, which calls it): the same against the distance squared.
fn func_809a9e28(arg0: f32, arg1: usize) -> f32 {
    let temp = arg0 * arg0;
    let d = DATA[D_809AA530 + arg1];
    if d < temp { d / temp } else { 1.0 }
}

/// `func_809A9E88`: the swirl round `vec` (`rReg0` bits 0..1), across the line to it.
fn func_809a9e88(this: &mut EffectSs, diff: Vec3, dist: f32) -> bool {
    let temp_v0 = (this.regs[R_REG0] & 3) as usize;
    if temp_v0 != 0 {
        let phi_f0 = if dist > 1.0 { 1.0 / dist } else { 1.0 };
        // D_809AA558[temp_v0 - 1]: 3 reads D_809AA560[0] (4).
        let k = DATA[D_809AA558 + temp_v0 - 1];
        this.accel.x += (k * diff.z) * phi_f0;
        this.accel.z -= (k * diff.x) * phi_f0;
    }
    true
}

/// `func_809A9F10`: the push up or down (`rReg0` bits 2..4).
fn func_809a9f10(this: &mut EffectSs) -> bool {
    let temp_v0 = ((this.regs[R_REG0] >> 2) & 7) as usize;
    if temp_v0 != 0 {
        // D_809AA560[temp_v0]: 7 reads D_809AA57C[0] (0.1).
        this.accel.y += DATA[D_809AA560 + temp_v0];
    }
    true
}

/// `func_809A9F4C`: the pull towards `vec` (`rReg0` bits 5..6).
fn func_809a9f4c(this: &mut EffectSs, diff: Vec3, dist: f32) -> bool {
    let temp_v0 = ((this.regs[R_REG0] >> 5) & 3) as usize;
    if temp_v0 != 0 {
        let phi_f0 = if dist > 1.0 { 1.0 / dist } else { 1.0 };
        let k = DATA[D_809AA57C + temp_v0 - 1];
        this.accel.x -= (diff.x * k) * phi_f0;
        this.accel.z -= (diff.z * k) * phi_f0;
    }
    true
}

/// `func_809A9FD8`: the forces scaled by how they fade with the distance (`rReg0` bits 7..10:
/// `D_809AA588`'s function), jittered by `rReg9`, plus a hundredth of that each way.
fn func_809a9fd8(this: &mut EffectSs, rand: &mut Rand, dist: f32) -> bool {
    let temp_a1 = ((this.regs[R_REG0] >> 7) & 0xF) as usize;
    // D_809AA588: func_809A9DD8 (1), func_809A9DEC ×3, func_809A9E28 ×3, func_809A9E68 ×3.
    let mut temp_f0 = match temp_a1 {
        0 => 1.0,
        1..=3 => func_809a9dec(dist, temp_a1),
        4..=9 => func_809a9e28(dist, temp_a1),
        _ => {
            // @bug (game): 10 to 15 read past the table (D_809AA5B0's floats as a function
            // pointer: the game crashes). No caller sets them.
            log::warn!("Effect_Ss_Kakera: rReg0 {:#x} reads past D_809AA588", this.regs[R_REG0]);
            1.0
        }
    };
    temp_f0 = func_809a9818(rand, temp_f0, (this.regs[R_REG9] as f32 * temp_f0) / 1024.0);
    this.accel *= temp_f0;
    this.accel.x += temp_f0 * 0.01;
    this.accel.y += temp_f0 * 0.01;
    this.accel.z += temp_f0 * 0.01;
    true
}

/// `func_809AA0B8`: gravity.
fn func_809aa0b8(this: &mut EffectSs) -> bool {
    this.accel.y += this.regs[R_GRAVITY] as f32 / 256.0;
    true
}

/// `func_809AA0EC`: the next frame's acceleration (`func_809A9DC0` zeroes it): the forces of
/// `rReg0` from `vec`, then gravity. False (the end of its life) past 1000 from `vec`.
fn func_809aa0ec(this: &mut EffectSs, rand: &mut Rand) -> bool {
    // func_809A9DC0.
    this.accel = Vec3::ZERO;
    let diff = Vec3::new(this.pos.x - this.vec.x, this.pos.y - this.vec.y, this.pos.z - this.vec.z);
    let dist = (diff.x * diff.x + diff.y * diff.y + diff.z * diff.z).sqrt();
    if dist > 1000.0 {
        return false;
    }
    if this.regs[R_REG0] != 0 {
        if !func_809a9e88(this, diff, dist) {
            return false;
        }
        if !func_809a9f10(this) {
            return false;
        }
        if !func_809a9f4c(this, diff, dist) {
            return false;
        }
        if !func_809a9fd8(this, rand, dist) {
            return false;
        }
    }
    func_809aa0b8(this)
}

/// `func_809AA230`: with no bounces left, it stops on Player's floor (`rReg4` bit 4, less the
/// size's margin) or ends 600 below it; with bounces left, by the collision mode: none (0: no
/// more bounces), a bounce off any poly its sphere touches while falling (1: up at 0.8, across
/// at 0.9 ± 0.2 each), or the test alone (2).
fn func_809aa230(this: &mut EffectSs, play: &mut PlayState) {
    let floor_height = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.floor_height).unwrap_or(f32::MIN);
    let r4 = this.regs[R_REG4];
    let size = ((r4 >> 2) & 3) as usize;
    if this.regs[R_REG8] == 0 {
        if ((r4 >> 4) & 1) * 0x10 == 0x10 {
            if this.pos.y <= floor_height - size as f32 {
                this.regs[R_REG9] = 0;
                this.regs[R_REG0] = 0;
                this.regs[R_REG4] &= !0x60;
                this.accel = Vec3::ZERO;
                this.velocity = Vec3::ZERO;
                this.regs[R_REG5] = this.regs[R_REG9];
                this.regs[R_GRAVITY] = this.regs[R_REG9];
            }
        } else if this.pos.y <= (floor_height - size as f32) - 600.0 {
            this.life = 0;
        }
    } else {
        let radius = D_809AA5B0.get(size).copied().unwrap_or(40.0);
        match r4 & 3 {
            0 => this.regs[R_REG8] = 0,
            1 => {
                if this.velocity.y < 0.0 && play.col.sph_vs_first_poly(this.pos, radius) {
                    this.velocity.x *= func_809a9818(&mut play.rand, 0.9, 0.2);
                    this.velocity.y *= -0.8;
                    this.velocity.z *= func_809a9818(&mut play.rand, 0.9, 0.2);
                    if this.regs[R_REG8] > 0 {
                        this.regs[R_REG8] -= 1;
                    }
                }
            }
            2 => {
                // The test alone: its result unused.
                let _ = play.col.sph_vs_first_poly(this.pos, radius);
            }
            _ => {}
        }
    }
}

/// `EffectSsKakera_Update`: the tumble (`rReg4` bits 5..6), the drag, the next frame's forces
/// (or the end of its life), the floor; the object checked.
pub fn update(play: &mut PlayState, this: &mut EffectSs) {
    match ((this.regs[R_REG4] >> 5) & 3) << 5 {
        0x20 => {
            this.regs[R_PITCH] = this.regs[R_PITCH].wrapping_add(0xB);
            this.regs[R_YAW] = this.regs[R_YAW].wrapping_add(3);
        }
        0x40 => {
            this.regs[R_PITCH] = this.regs[R_PITCH].wrapping_add(0x41);
            this.regs[R_YAW] = this.regs[R_YAW].wrapping_add(0xB);
        }
        0x60 => {
            this.regs[R_PITCH] = this.regs[R_PITCH].wrapping_add(0x9B);
            this.regs[R_YAW] = this.regs[R_YAW].wrapping_add(0x1F);
        }
        _ => {}
    }
    func_809a9c10(this, &mut play.rand);
    if !func_809aa0ec(this, &mut play.rand) {
        this.life = 0;
    }
    func_809aa230(this, play);
    if this.regs[R_OBJ_ID] != KAKERA_OBJECT_DEFAULT {
        check_object(this, Some(&play.object_ctx));
    }
}

/// `EffectSsKakera_Draw`: at its position, turned by its yaw then pitch (hundredths of a
/// radian), scaled `rScale / 256`; translucent with `rReg4` bit 7, else opaque; the colour of
/// `rColorIdx` when it's 0 or more.
pub fn draw(this: &EffectSs, out: &mut EffectDraws) {
    let Some((_, symbol)) = this.gfx else { return };
    let scale = this.regs[R_SCALE] as f32 / 256.0;
    let color_idx = this.regs[R_COLOR_IDX];
    let m = Mat4::from_translation(this.pos) * Mat4::from_rotation_y(this.regs[R_YAW] as f32 * 0.01) * Mat4::from_rotation_x(this.regs[R_PITCH] as f32 * 0.01) * Mat4::from_scale(Vec3::splat(scale));
    let mut sv = SegmentValues::default();
    if color_idx >= 0
        && let Some(c) = COLORS.get(color_idx as usize)
    {
        sv.prim[SEG_PRIM as usize] = Some([c[0], c[1], c[2], 255]);
    }
    let cmd = DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(symbol))), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } };
    if (((this.regs[R_REG4] >> 7) & 1) << 7) == 0x80 {
        out.xlu.push(cmd);
    } else {
        out.opa.push(cmd);
    }
}
