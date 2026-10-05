//! `Effect_Ss_Hahen` (`ovl_Effect_Ss_Hahen/z_eff_ss_hahen.c`): a fragment (by default the
//! withered Deku fragment, `gEffFragments1DL`; or an object's display list) tumbling as it
//! falls, gone once it's below Player's floor after its minimum life.

use eng_gfx::{DrawCmd, MeshKey};
use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SsDraw, SsGfx, SsSpawn, SsUpdate};
use crate::object_ctx::ObjectContext;
use crate::pack::keys;
use crate::play::PlayState;

// The overlay's regs.
const R_PITCH: usize = 0;
const R_YAW: usize = 1;
const R_UNUSED: usize = 2;
const R_SCALE: usize = 3;
const R_OBJ_ID: usize = 4;
const R_OBJECT_SLOT: usize = 5;
const R_MIN_LIFE: usize = 6;

/// `HAHEN_OBJECT_DEFAULT`.
pub const HAHEN_OBJECT_DEFAULT: i16 = -1;
/// `OBJECT_HAKA_OBJECTS` (`object_table.h`): the Shadow Temple's skull pots, drawn grey.
const OBJECT_HAKA_OBJECTS: i16 = 0x0069;

/// `gEffFragments1DL`, the default fragment.
const DEFAULT_DL: (&str, &str) = ("gameplay_keep", "gEffFragments1DL");

/// `EffectSsHahenInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct HahenInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub dlist: SsGfx,
    pub unused: i16,
    pub scale: i16,
    pub obj_id: i16,
    pub life: i16,
}

/// `EffectSsHahen_CheckForObject`: without its object loaded, the fragment is freed and not
/// drawn.
fn check_for_object(this: &mut EffectSs, objects: Option<&ObjectContext>) {
    let slot = objects.and_then(|o| o.get_index(this.regs[R_OBJ_ID]).filter(|&b| o.is_loaded(b)));
    match slot {
        Some(b) => this.regs[R_OBJECT_SLOT] = b as i16,
        None => {
            this.regs[R_OBJECT_SLOT] = -1;
            this.life = -1;
            this.draw = None;
        }
    }
}

/// `EffectSsHahen_Init`: 200 frames, a random pitch and yaw (0 to 314, hundredths of a radian).
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &HahenInit) -> bool {
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.life = 200;
    if p.dlist.is_some() {
        this.gfx = p.dlist;
        this.regs[R_OBJ_ID] = p.obj_id;
        check_for_object(this, s.objects);
    } else {
        this.gfx = Some(DEFAULT_DL);
        this.regs[R_OBJ_ID] = -1;
    }
    this.draw = Some(if this.regs[R_OBJ_ID] == OBJECT_HAKA_OBJECTS && this.gfx.is_some_and(|g| g.1 == "gEffFragments2DL") { SsDraw::HahenGray } else { SsDraw::Hahen });
    this.update = Some(SsUpdate::Hahen);
    this.regs[R_UNUSED] = p.unused;
    this.regs[R_SCALE] = p.scale;
    this.regs[R_PITCH] = (s.rand.zero_one() * 314.0) as i16;
    this.regs[R_YAW] = (s.rand.zero_one() * 314.0) as i16;
    this.regs[R_MIN_LIFE] = 200 - p.life;
    // EffectSsHahen_CheckForObject may have freed the slot; the C returns success anyway.
    true
}

/// `EffectSsHahen_Update`: tumbling; at or below Player's floor after its minimum life, it ends.
pub fn update(play: &mut PlayState, this: &mut EffectSs) {
    this.regs[R_PITCH] = this.regs[R_PITCH].wrapping_add(55);
    this.regs[R_YAW] = this.regs[R_YAW].wrapping_add(10);
    let floor = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.floor_height).unwrap_or(f32::MIN);
    if this.pos.y <= floor && this.life < this.regs[R_MIN_LIFE] {
        this.life = 0;
    }
    if this.regs[R_OBJ_ID] != -1 {
        check_for_object(this, Some(&play.object_ctx));
    }
}

/// `EffectSsHahen_Draw`: at its position, turned by its yaw then pitch, scaled `rScale ×
/// 0.001`, after `Gfx_SetupDL_25Opa` (the list's mesh is baked so). `EffectSsHahen_DrawGray`'s
/// grey combiner isn't (no ported actor spawns the skull pots' fragments).
pub fn draw(this: &EffectSs, out: &mut EffectDraws) {
    let Some((file, symbol)) = this.gfx else { return };
    let scale = this.regs[R_SCALE] as f32 * 0.001;
    let m = Mat4::from_translation(this.pos) * Mat4::from_rotation_y(this.regs[R_YAW] as f32 * 0.01) * Mat4::from_rotation_x(this.regs[R_PITCH] as f32 * 0.01) * Mat4::from_scale(Vec3::splat(scale));
    out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(file, symbol)), m));
}
