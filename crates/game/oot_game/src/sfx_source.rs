//! `z_sfx_source.c`: sounds at a fixed place in the world, for as long as a countdown. The play
//! state's sixteen sources each keep a world position and its `projectedPos`, which the sound
//! effects read (`SfxPos::Source`) as they read an actor's.

use glam::Vec3;

use crate::audio::sfx::{SfxF32, SfxPos, SfxS8};
use crate::play::PlayState;

/// `ARRAY_COUNT(play->sfxSources)` (`z64.h`).
pub const NUM_SFX_SOURCES: usize = 16;

/// `SfxSource` (`z64.h`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SfxSource {
    pub countdown: u16,
    pub world_pos: Vec3,
    pub projected_pos: Vec3,
}

impl PlayState {
    /// `SfxSource_InitAll`.
    pub fn sfx_source_init_all(&mut self) {
        for s in self.sfx_sources.iter_mut() {
            s.countdown = 0;
        }
    }

    /// `SfxSource_UpdateAll`: each running source counts down; at zero its sounds stop, else it's
    /// projected again through `viewProjectionMtxF` (`SkinMatrix_Vec3fMtxFMultXYZ`: no divide).
    pub fn sfx_source_update_all(&mut self) {
        for i in 0..NUM_SFX_SOURCES {
            let s = &mut self.sfx_sources[i];
            if s.countdown != 0 {
                s.countdown -= 1;
                if s.countdown == 0 {
                    self.audio.stop_sfx_by_pos(SfxPos::Source(i as u8));
                } else {
                    s.projected_pos = project_xyz(self.view_proj, s.world_pos);
                }
            }
        }
    }

    /// `SfxSource_PlaySfxAtFixedWorldPos`: the first free source, or failing that the one with the
    /// least time left (its sounds stopped), plays `sfx_id` at `world_pos` for `duration` frames.
    pub fn sfx_source_play_sfx_at_fixed_world_pos(&mut self, world_pos: Vec3, duration: u16, sfx_id: u16) {
        let mut smallest_countdown: i32 = 0xFFFF;
        // backupSource is uninitialised in the C until a source is busy; every source is busy
        // when it's read.
        let mut backup = 0;
        let mut i = 0;
        while i < NUM_SFX_SOURCES {
            let countdown = self.sfx_sources[i].countdown as i32;
            if countdown == 0 {
                break;
            }
            if countdown < smallest_countdown {
                smallest_countdown = countdown;
                backup = i;
            }
            i += 1;
        }
        if i >= NUM_SFX_SOURCES {
            i = backup;
            self.audio.stop_sfx_by_pos(SfxPos::Source(i as u8));
        }
        let vp = self.view_proj;
        let s = &mut self.sfx_sources[i];
        s.world_pos = world_pos;
        s.countdown = duration;
        s.projected_pos = project_xyz(vp, world_pos);
        self.audio.play_sfx_general(sfx_id, SfxPos::Source(i as u8), 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
    }
}

/// `SkinMatrix_Vec3fMtxFMultXYZ`: `mf` times (`v`, 1), without the divide by w.
fn project_xyz(mf: glam::Mat4, v: Vec3) -> Vec3 {
    (mf * v.extend(1.0)).truncate()
}
