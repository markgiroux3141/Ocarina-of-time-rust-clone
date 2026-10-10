//! `Effect_Ss_Dead_Sound` (`ovl_Effect_Ss_Dead_Sound/z_eff_ss_dead_sound.c`): a sound played at
//! a place for a while after its maker is gone (an enemy's death cry). No draw.

use glam::Vec3;

use super::{EffectSs, SsUpdate};
use crate::audio::sfx::SfxPos;
use crate::play::PlayState;

// The overlay's regs.
const R_SFX_ID: usize = 10;
const R_REPEAT_MODE: usize = 11;

/// `DEADSOUND_REPEAT_MODE_OFF`, `_ON`.
pub const DEADSOUND_REPEAT_MODE_OFF: i16 = 1;
pub const DEADSOUND_REPEAT_MODE_ON: i16 = 2;

/// `EffectSsDeadSoundInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct DeadSoundInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub sfx_id: u16,
    pub lower_priority: i16,
    pub repeat_mode: i16,
    pub life: i32,
}

/// `EffectSsDeadSound_Init`: flag 2 (its sound at `pos` stops with it), no draw.
pub fn init(this: &mut EffectSs, p: &DeadSoundInit) -> bool {
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.flags = 2;
    this.life = p.life as i16;
    this.draw = None;
    this.update = Some(SsUpdate::DeadSound);
    this.regs[R_REPEAT_MODE] = p.repeat_mode;
    this.regs[R_SFX_ID] = p.sfx_id as i16;
    log::trace!("Constructor 3");
    true
}

/// `EffectSsDeadSound_Update`: the sound at its place once (repeat mode off, counted down to 0,
/// which plays nothing), or every update (on; unused in the game).
pub fn update(play: &mut PlayState, index: usize, this: &mut EffectSs) {
    match this.regs[R_REPEAT_MODE] {
        DEADSOUND_REPEAT_MODE_OFF => this.regs[R_REPEAT_MODE] -= 1,
        DEADSOUND_REPEAT_MODE_ON => {}
        _ => return,
    }
    play.audio.play_sfx_at_pos(SfxPos::EffectSsPos(index as u8), this.regs[R_SFX_ID] as u16);
}
