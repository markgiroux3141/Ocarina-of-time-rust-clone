//! `Effect_Ss_Fhg_Flash` (`ovl_Effect_Ss_Fhg_Flash/z_eff_ss_fhg_flash.c`): Phantom Ganon's
//! light balls, and the shock's sparks (`sShockDL`, the overlay's own list): on Player while he's
//! electrified (`Player_UpdateBodyShock`), jumping to a random body part every frame.

use glam::{Mat4, Vec3};

use super::{EffectDraws, EffectSs, SEG_COLOR, SEG_SETUP, SsDraw, SsSpawn, SsUpdate, colored};
use crate::actor_ctx::{ActorHandle, PLAYER_BODYPART_MAX};
use crate::gbi::{G_RM_AA_ZB_XLU_DECAL2, G_RM_AA_ZB_XLU_SURF2, G_RM_PASS, setup_dl};
use crate::pack::{BakeBody, BakeSegment, MeshBake};
use crate::play::PlayState;

// The overlay's regs.
const R_ALPHA: usize = 0;
const R_OBJECT_SLOT: usize = 2;
const R_XZ_ROT: usize = 3;
const R_PARAM: usize = 4;
const R_SCALE: usize = 8;

/// `FhgFlashType`.
pub const FHGFLASH_LIGHTBALL: u8 = 0;
pub const FHGFLASH_SHOCK: u8 = 1;
/// `FhgFlashLightningParam`.
pub const FHGFLASH_SHOCK_NO_ACTOR: u8 = 0;
pub const FHGFLASH_SHOCK_PLAYER: u8 = 1;
pub const FHGFLASH_SHOCK_PG: u8 = 2;

/// `OBJECT_FHG` (`object_table.h`).
const OBJECT_FHG: i16 = 0x005A;

/// `EffectSsFhgFlashInitParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct FhgFlashInit {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub scale: i16,
    pub param: u8,
    pub actor: Option<ActorHandle>,
    pub ty: u8,
}

/// `EffectSsFhgFlash_Init`. A light ball needs `object_fhg` loaded; a shock lives 111 to 120
/// frames at `scale` to twice it, starting out of sight when it follows an actor.
pub fn init(s: &mut SsSpawn<'_>, this: &mut EffectSs, p: &FhgFlashInit) -> bool {
    if p.ty == FHGFLASH_LIGHTBALL {
        let slot = s.objects.and_then(|o| o.get_index(OBJECT_FHG).filter(|&b| o.is_loaded(b)));
        let Some(slot) = slot else {
            log::error!("Effect_Ss_Fhg_Flash_ct():pffd->mode error");
            return false;
        };
        this.regs[R_OBJECT_SLOT] = slot as i16;
        this.pos = p.pos;
        this.velocity = p.velocity;
        this.accel = p.accel;
        this.regs[R_PARAM] = p.param as i16;
        this.life = 100;
        this.regs[R_SCALE] = p.scale;
        this.regs[R_ALPHA] = 255;
        // EffectSsFhgFlash_DrawLightBall isn't ported (no ported actor has object_fhg).
        log::debug!("Effect_Ss_Fhg_Flash: the light ball's draw is not ported");
        this.draw = None;
        this.update = Some(SsUpdate::FhgFlashLightBall);
        this.gfx = Some(("object_fhg", "gPhantomEnergyBallDL"));
    } else {
        this.actor = p.actor;
        this.velocity = Vec3::ZERO;
        this.accel = Vec3::ZERO;
        this.life = (s.rand.zero_one() * 10.0) as i16 + 111;
        this.regs[R_SCALE] = (s.rand.zero_float(p.scale as f32) as i16).wrapping_add(p.scale);
        this.regs[R_ALPHA] = 255;
        this.draw = Some(SsDraw::FhgFlashShock);
        this.update = Some(SsUpdate::FhgFlashShock);
        this.regs[R_PARAM] = p.param as i16;
        // Out of sight until it's on the actor.
        this.pos = if p.param != FHGFLASH_SHOCK_NO_ACTOR { Vec3::new(0.0, -1000.0, 0.0) } else { p.pos };
        this.gfx = Some(("ovl_Effect_Ss_Fhg_Flash", "sShockDL"));
    }
    true
}

/// `EffectSsFhgFlash_UpdateLightBall`.
pub fn update_light_ball(play: &mut PlayState, this: &mut EffectSs) {
    let rand = (play.rand.zero_one() * 20000.0) as i16;
    let r = &mut this.regs;
    r[R_XZ_ROT] = r[R_XZ_ROT].wrapping_add(rand).wrapping_add(0x4000);
    if r[R_SCALE] > 0 {
        r[R_SCALE] -= 10;
        if r[R_SCALE] <= 0 {
            r[R_SCALE] = 0;
            this.life = 0;
        }
    }
    if r[R_ALPHA] > 0 {
        r[R_ALPHA] -= 10;
        if r[R_ALPHA] <= 0 {
            r[R_ALPHA] = 0;
        }
    }
}

/// `EffectSsFhgFlash_UpdateShock`: spinning; on Player, to a random body part give or take 10
/// across and 15 up; fading out over its last frames under 100.
pub fn update_shock(play: &mut PlayState, this: &mut EffectSs) {
    let rot_step = (play.rand.zero_one() * 20000.0) as i16;
    this.regs[R_XZ_ROT] = this.regs[R_XZ_ROT].wrapping_add(rot_step.wrapping_add(0x4000));
    match this.regs[R_PARAM] as u8 {
        FHGFLASH_SHOCK_PLAYER => {
            let bp = play.rand.zero_float(PLAYER_BODYPART_MAX as f32 - 0.1) as i16 as usize;
            let part = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.body_part(bp)).unwrap_or(Vec3::ZERO);
            this.pos.x = part.x + play.rand.centered_float(10.0);
            this.pos.y = part.y + play.rand.centered_float(15.0);
            this.pos.z = part.z + play.rand.centered_float(10.0);
        }
        FHGFLASH_SHOCK_PG => {
            // Phantom Ganon's body parts: Boss_Ganondrof isn't ported, so the parts read as the
            // origin; the Rand calls are the C's.
            let _ = play.rand.zero_float(23.9);
            this.pos.x = play.rand.centered_float(15.0);
            this.pos.y = play.rand.centered_float(20.0);
            this.pos.z = play.rand.centered_float(15.0);
        }
        _ => {}
    }
    if this.life < 100 {
        this.regs[R_ALPHA] -= 50;
        if this.regs[R_ALPHA] < 0 {
            this.regs[R_ALPHA] = 0;
            this.life = 0;
        }
    }
}

const SHOCK_ACTOR: &str = "EffectSs/fhg_shock/actor";
const SHOCK_FIXED: &str = "EffectSs/fhg_shock/fixed";

/// `sShockDL`: on an actor after `Gfx_SetupDL_44Xlu` with `G_RM_PASS, G_RM_AA_ZB_XLU_DECAL2`,
/// fixed after `Gfx_SetupDL_25Xlu` (the bake's start) with `G_RM_PASS, G_RM_AA_ZB_XLU_SURF2`;
/// the colours dynamic.
pub fn bakes() -> Vec<MeshBake> {
    let mut actor = setup_dl::setup_dl_44();
    actor.render_mode(G_RM_PASS, G_RM_AA_ZB_XLU_DECAL2);
    actor.pipe_sync();
    actor.end();
    let mut fixed = crate::gbi::Dl::default();
    fixed.render_mode(G_RM_PASS, G_RM_AA_ZB_XLU_SURF2);
    fixed.pipe_sync();
    fixed.end();
    [(SHOCK_ACTOR, actor), (SHOCK_FIXED, fixed)]
        .into_iter()
        .map(|(name, d)| MeshBake {
            name: name.into(),
            object: "gameplay_keep".into(),
            segments: vec![(SEG_SETUP, BakeSegment::Commands(d.0)), (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true })],
            prelude: vec![SEG_SETUP, SEG_COLOR],
            body: BakeBody::DLists(vec![("ovl_Effect_Ss_Fhg_Flash".into(), "sShockDL".into())]),
        })
        .collect()
}

/// `EffectSsFhgFlash_DrawShock`: scaled `rScale / 100`; on an actor turned about x by its spin
/// (× 1.1416), else facing the camera; then about z by its spin (× 3.1416).
pub fn draw_shock(this: &EffectSs, billboard: Mat4, out: &mut EffectDraws) {
    let r = &this.regs;
    let scale = r[R_SCALE] as f32 / 100.0;
    let spin = r[R_XZ_ROT] as f32 / 0x8000 as f32;
    let (bake, rot) = if r[R_PARAM] as u8 != FHGFLASH_SHOCK_NO_ACTOR {
        (SHOCK_ACTOR, Mat4::from_rotation_x(spin * 1.1416))
    } else {
        // Matrix_ReplaceRotation(&play->billboardMtxF): the billboard in place of the rotation,
        // each column keeping its length (the uniform scale here).
        (SHOCK_FIXED, billboard)
    };
    let m = Mat4::from_translation(this.pos) * rot * Mat4::from_scale(Vec3::splat(scale)) * Mat4::from_rotation_z(spin * 3.1416);
    out.xlu.push(colored(bake, m, SEG_COLOR, [255, 255, 255, r[R_ALPHA] as u8], [0, 255, 155, 0]));
}
