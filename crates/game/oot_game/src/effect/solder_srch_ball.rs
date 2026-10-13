//! `Effect_Ss_Solder_Srch_Ball` (`ovl_Effect_Ss_Solder_Srch_Ball/z_eff_ss_solder_srch_ball.c`):
//! the guards' line of sight in the castle's courtyard (`EffectSsSolderSrchBall_Spawn`, from
//! `En_Heishi1`'s update). An invisible ball flies from the guard's head along his look; while it
//! touches no poly (a sphere of 30), Player within 70 of it is seen: it sets the guard's
//! `linkDetected` (`ActorImpl::solder_srch_ball_detect`). Touching a poly ends it the next frame.
//! It has no draw.

use glam::Vec3;

use super::{EffectSs, SsUpdate};
use crate::actor_ctx::ActorHandle;
use crate::play::PlayState;

// The overlay's regs.
/// `rUnused` (`regs[1]`).
const R_UNUSED: usize = 1;

/// `EffectSsSolderSrchBallInitParams`. `linkDetected` points into the spawner (the C keeps it
/// in the effect's `actor` field); here it's the spawner's handle.
#[derive(Debug, Clone, PartialEq)]
pub struct SolderSrchBallInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub unused: i16,
    pub link_detected: Option<ActorHandle>,
}

/// `EffectSsSolderSrchBall_Init`: 100 frames.
pub fn init(this: &mut EffectSs, p: &SolderSrchBallInit) -> bool {
    this.pos = p.pos;
    this.velocity = p.velocity;
    this.accel = p.accel;
    this.update = Some(SsUpdate::SolderSrchBall);
    this.life = 100;
    this.regs[R_UNUSED] = p.unused;
    // "actor field was incorrectly used as a pointer to something else".
    this.actor = p.link_detected;
    true
}

/// `EffectSsSolderSrchBall_Update`.
pub fn update(play: &mut PlayState, this: &mut EffectSs) {
    let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO);
    let d = player_pos - this.pos;
    if !play.col.sph_vs_first_poly(this.pos, 30.0) {
        if (d.x * d.x + d.y * d.y + d.z * d.z).sqrt() < 70.0
            && let Some(a) = this.actor.and_then(|h| play.actors.get_mut(h))
        {
            a.solder_srch_ball_detect();
        }
    } else if this.life > 1 {
        this.life = 1;
    }
}
