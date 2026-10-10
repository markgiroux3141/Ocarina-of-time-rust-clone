//! Z-targeting's actor side: the target context (`Attention` in `z_actor.c`).
//!
//! - Every frame `Actor_UpdateAll` refreshes each actor's distance and yaw to Player
//!   (`Actor::update_distances`).
//! - `Attention_Update` then picks what the Z button would lock on to next (`arrowPointedActor`,
//!   via `Attention_FindActor` / `Attention_FindActorInCategory` / `Attention_WeightedDistToPlayerSq`), and steps the lock-on reticle
//!   (`reticleRadius` → 80, then `reticleSpinCounter` counts). Player turns to face a target only once `reticleSpinCounter != 0`.
//! - `Attention_Draw` (called from `Interface_Draw`) moves the reticle on the screen once per
//!   game frame (`draw_update`: three trail entries, `reticleFadeAlphaControl` fading it out after a target is
//!   lost) and draws it (`draw`): four `gLockOnReticleTriangleDL` triangles per entry in the
//!   orthographic overlay, and `gLockOnArrowDL` over the next candidate (`arrowHoverActor`), both in
//!   the colour of the actor's category (`sAttentionColors`).
//!
//! Targets are ordinary actors: `ACTOR_FLAG_ATTENTION_ENABLED` makes them targetable, with `ACTOR_FLAG_HOSTILE`
//! they're hostile, and their `targetMode` and focus decide the rest.
//!
//! Navi's part (`En_Elf` reads it): `naviRefPos`, where she flies (the pointed actor's focus,
//! or Player's), eased over four frames when the pointed actor changes (`naviMoveProgressFactor`), and her
//! colours for its category (`naviInner`, `naviOuter`, `Attention_SetNaviState`). The lock-on
//! sounds and the BGM-enemy tracking are not modelled.

#![allow(non_snake_case)] // reticleSpinCounter keeps the decomp's name

use eng_collision::bgcheck::{self, CollisionContext};
use eng_math::atan2_s;
use glam::{Mat4, Vec3};

pub use crate::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_HOSTILE, ACTOR_FLAG_LOCK_ON_DISABLED};
use eng_gfx::{DrawCmd, DrawLists, DrawParams, MeshKey, SegmentValues};

use crate::actor::Actor;
use crate::actor_ctx::{ActorContext, ActorHandle};
use crate::pack::{BakeBody, BakeSegment, MeshBake, keys};

/// `Attention` fields Player reads.
#[derive(Debug, Clone, Default)]
pub struct TargetCtx {
    /// `arrowPointedActor`: what a Z press locks on to.
    pub arrow_pointed: Option<ActorHandle>,
    /// `arrowHoverActor`: the next candidate while one is locked (Z again switches to it).
    pub arrow_hover_actor: Option<ActorHandle>,
    /// `targetedActor`.
    pub targeted: Option<ActorHandle>,
    /// `reticleRadius`: reticle size, stepping from 500 to 80.
    pub reticle_radius: f32,
    /// `reticleSpinCounter`: non-zero once the reticle has locked.
    pub reticle_spin_counter: u8,
    /// `reticleFadeAlphaControl`: the reticle's alpha after its target is lost (0x100 on a new target).
    pub reticle_fade_alpha_control: i16,
    /// `curReticle`: the trail entry written this frame (counts down 2, 1, 0).
    pub cur_reticle: i8,
    /// `targetCenterPos`.
    pub target_center_pos: Vec3,
    /// `arr_50`: the reticle's last three screen positions.
    pub arr_50: [TargetEntry; 3],
    /// `naviRefPos`, `naviInner`, `naviOuter` (`Color_RGBAf`): where Navi flies and her colours.
    pub navi_ref_pos: Vec3,
    pub navi_inner: [f32; 4],
    pub navi_outer: [f32; 4],
    /// `naviMoveProgressFactor`: 1 when the pointed actor (or its category) changes, stepping to 0 by 0.25 as
    /// `naviRefPos` eases over.
    pub navi_move_progress_factor: f32,
    /// `activeCategory`: the pointed actor's category (Player's without one).
    pub active_category: usize,
    /// `forcedLockOnActor`: an actor Navi is sent to for one frame (none of the ported actors sets it).
    pub forced_lock_on_actor: Option<ActorHandle>,
    /// `bgmEnemy`: the nearest hostile enemy within 500 (`Attention_FindActorInCategory`): Player
    /// plays the enemy music while there is one.
    pub bgm_enemy: Option<ActorHandle>,
    /// What `Attention_Draw` draws this frame (`None`: nothing).
    pub reticle: Option<ReticleDraw>,
}

/// `LockOnReticle`: a reticle position on the screen (the overlay's coordinates, centred,
/// y up), its size (`radius`, `reticleRadius` when written) and colour.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TargetEntry {
    pub pos: Vec3,
    pub radius: f32,
    pub color: [u8; 3],
}

/// `Attention_Draw`'s triangles this frame: the first entry's alpha (`spCE`) and how many
/// entries it draws from `curReticle` (`spB8`: 3 while locking, 1 once locked).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReticleDraw {
    pub alpha: i16,
    pub count: usize,
}

/// `sAttentionColors[category].outer` (`z_actor.c`), with `inner`'s alpha 255 and this alpha 0.
pub const NAVI_OUTER: [[u8; 3]; 13] = [
    [0, 255, 0],
    [0, 255, 0],
    [0, 0, 255],
    [0, 255, 0],
    [150, 150, 255],
    [200, 155, 0],
    [0, 255, 0],
    [0, 255, 0],
    [0, 255, 0],
    [200, 155, 0],
    [0, 255, 0],
    [0, 255, 0],
    [0, 255, 0],
];

/// `sAttentionColors[category].inner`.
pub const NAVI_INNER: [[u8; 3]; 13] = [
    [0, 255, 0],
    [0, 255, 0],
    [255, 255, 255],
    [0, 255, 0],
    [150, 150, 255],
    [255, 255, 0],
    [0, 255, 0],
    [0, 255, 0],
    [0, 255, 0],
    [255, 255, 0],
    [0, 255, 0],
    [0, 255, 0],
    [0, 255, 0],
];

fn navi_inner(category: usize) -> [u8; 3] {
    NAVI_INNER.get(category).copied().unwrap_or([0, 255, 0])
}

/// The baked `gLockOnReticleTriangleDL`, after `Gfx_SetupDL(OVERLAY_DISP, SETUPDL_57)`.
pub const LOCK_ON_TRIANGLE: &str = "z_actor/lock_on_triangle";
/// The baked `gLockOnArrowDL`, after `Gfx_SetupDL(POLY_XLU_DISP, SETUPDL_7)`.
pub const TARGET_ARROW: &str = "z_actor/target_arrow";
/// The segment holding each bake's `sSetupDL` entry, and the one for `gDPSetPrimColor`.
const SEG_SETUP_DL: u8 = 0x0D;
const SEG_PRIM: u8 = 0x0E;

/// The target reticle's meshes (docs/adr/0012-actor-bakes.md). `sSetupDL` (`z_rcp.c`) is data
/// in `code`, so its two entries are written out here from their macros (`gbi.h` values).
pub fn bakes() -> Vec<MeshBake> {
    const PIPE_SYNC: (u32, u32) = (0xE700_0000, 0);
    // SETUPDL_57: gsSPTexture(0xFFFF, 0xFFFF, 0, G_TX_RENDERTILE, G_OFF),
    // gsDPSetCombineMode(G_CC_PRIMITIVE, G_CC_PRIMITIVE),
    // gsDPSetOtherMode(G_AD_DISABLE | G_TC_FILT | G_TF_BILERP (| 0 fields),
    //                  G_AC_THRESHOLD | G_ZS_PIXEL | G_RM_CLD_SURF | G_RM_CLD_SURF2),
    // gsSPLoadGeometryMode(G_SHADING_SMOOTH).
    let setup_dl_57 = vec![PIPE_SYNC, (0xD700_0000, 0xFFFF_FFFF), (0xFCFF_FFFF, 0xFFFD_F6FB), (0xEF00_2C30, 0x0050_4341), (0xD900_0000, 0x0020_0000)];
    // SETUPDL_7: gsSPTexture(0xFFFF, 0xFFFF, 0, G_TX_RENDERTILE, G_ON),
    // gsDPSetCombineMode(G_CC_MODULATEIA_PRIM, G_CC_MODULATEIA_PRIM),
    // gsDPSetOtherMode(G_AD_NOTPATTERN | G_TC_FILT | G_TF_BILERP | G_TP_PERSP (| 0 fields),
    //                  G_AC_NONE | G_ZS_PIXEL | G_RM_AA_XLU_SURF | G_RM_AA_XLU_SURF2),
    // gsSPLoadGeometryMode(G_SHADE | G_LIGHTING | G_SHADING_SMOOTH).
    let setup_dl_7 = vec![PIPE_SYNC, (0xD700_0002, 0xFFFF_FFFF), (0xFC11_9623, 0xFF2F_FFFF), (0xEF08_2C10, 0x0050_41C8), (0xD900_0000, 0x0022_0004)];
    let bake = |name: &str, setup: Vec<(u32, u32)>, dl: &str| MeshBake {
        name: name.into(),
        object: "gameplay_keep".into(),
        segments: vec![(SEG_SETUP_DL, BakeSegment::Commands(setup)), (SEG_PRIM, BakeSegment::DynamicColor { env: false, prim: true })],
        prelude: vec![SEG_SETUP_DL, SEG_PRIM],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), dl.into())]),
    };
    vec![bake(LOCK_ON_TRIANGLE, setup_dl_57, "gLockOnReticleTriangleDL"), bake(TARGET_ARROW, setup_dl_7, "gLockOnArrowDL")]
}

/// What Player needs from the targeting system when it updates.
#[derive(Debug, Clone, Copy, Default)]
pub struct TargetView {
    pub arrow_pointed: Option<ActorHandle>,
    pub arrow_hover_actor: Option<ActorHandle>,
    pub reticle_locked: bool,
}

impl TargetCtx {
    /// A context before `Actor_InitContext`'s `Attention_Init`: `Attention_InitReticle` with Player's
    /// category.
    pub fn new() -> TargetCtx {
        let mut ctx = TargetCtx::default();
        ctx.attention_init_reticle(crate::actor_ctx::ACTORCAT_PLAYER, Vec3::ZERO);
        ctx
    }

    /// `Attention_Init` (`Actor_InitContext`, with Player, once it's spawned): nothing pointed or
    /// targeted, Navi to `actor`, and `Attention_InitReticle`.
    pub fn attention_init(&mut self, actor: &Actor, view_eye: Vec3) {
        self.arrow_pointed = None;
        self.targeted = None;
        self.navi_move_progress_factor = 0.0;
        self.forced_lock_on_actor = None;
        self.bgm_enemy = None;
        self.reticle_spin_counter = 0;
        self.cur_reticle = 0;
        self.set_navi_to_actor(actor, actor.category);
        self.attention_init_reticle(actor.category, view_eye);
    }

    /// `Attention_SetNaviState`: Navi's point at `actor`'s focus (raised by its target arrow's
    /// offset), and the category's colours.
    pub fn set_navi_to_actor(&mut self, actor: &Actor, category: usize) {
        self.navi_ref_pos = Vec3::new(actor.focus_pos.x, actor.focus_pos.y + (actor.target_arrow_offset * actor.scale.y), actor.focus_pos.z);
        let (i, o) = (navi_inner(category), NAVI_OUTER.get(category).copied().unwrap_or([0, 255, 0]));
        self.navi_inner = [i[0] as f32, i[1] as f32, i[2] as f32, 255.0];
        self.navi_outer = [o[0] as f32, o[1] as f32, o[2] as f32, 0.0];
    }

    /// `Attention_InitReticle`: a new target. The reticle starts at the eye, at full size (500), and
    /// takes the category's colour.
    pub fn attention_init_reticle(&mut self, category: usize, view_eye: Vec3) {
        self.target_center_pos = view_eye;
        self.reticle_radius = 500.0;
        self.reticle_fade_alpha_control = 0x100;
        let color = navi_inner(category);
        for i in 0..self.arr_50.len() {
            self.attention_set_reticle_pos(i, Vec3::ZERO);
            self.arr_50[i].color = color;
        }
    }

    /// `Attention_SetReticlePos`.
    fn attention_set_reticle_pos(&mut self, index: usize, pos: Vec3) {
        self.arr_50[index].pos = pos;
        self.arr_50[index].radius = self.reticle_radius;
    }

    pub fn view(&self) -> TargetView {
        TargetView { arrow_pointed: self.arrow_pointed, arrow_hover_actor: self.arrow_hover_actor, reticle_locked: self.reticle_spin_counter != 0 }
    }
}

/// The inputs of the end-of-frame target update.
pub struct TargetFrame<'a> {
    pub col: &'a CollisionContext,
    pub ranges: &'a [(f32, f32)],
    /// Player itself (not a target).
    pub player: ActorHandle,
    /// Player's `focusActor` (locked target), `zTargetActiveTimer` (target timer), `controlStickDirections[controlStickDataIndex]`.
    pub player_target: Option<ActorHandle>,
    pub player_timer: i16,
    pub player_stick_dir: i8,
    pub player_shape_yaw: i16,
    /// `player->actor.focus.pos` (the head).
    pub player_focus: Vec3,
    /// `play->viewProjectionMtxF`.
    pub view_proj: Mat4,
    /// `play->view.eye`.
    pub view_eye: Vec3,
}

/// `Attention_ActorIsInRange`.
pub fn in_range(ranges: &[(f32, f32)], a: &Actor, dist: f32) -> bool {
    dist < ranges.get(a.target_mode as usize).map(|r| r.0).unwrap_or(0.0)
}

/// `Attention_WeightedDistToPlayerSq`: distance weighted by how far off Player's facing the actor is.
pub fn weighted_dist(a: &Actor, has_target: bool, player_yaw: i16) -> f32 {
    let yaw = a.yaw_towards_player.wrapping_sub(i16::MIN).wrapping_sub(player_yaw);
    let abs = (yaw as i32).abs();
    if has_target {
        if abs > 0x4000 || a.flags & ACTOR_FLAG_LOCK_ON_DISABLED != 0 {
            f32::MAX
        } else {
            a.xyz_dist_to_player_sq - a.xyz_dist_to_player_sq * 0.8 * ((0x4000 - abs) as f32 * (1.0 / 32768.0))
        }
    } else if abs > 0x2AAA {
        f32::MAX
    } else {
        a.xyz_dist_to_player_sq
    }
}

/// `Attention_ShouldReleaseLockOn`: should Player lose `a` (out of range, or behind and not locked with `flag`)?
pub fn lost(ranges: &[(f32, f32)], a: &Actor, player_has_target: bool, player_yaw: i16, flag: bool) -> bool {
    if a.flags & ACTOR_FLAG_ATTENTION_ENABLED == 0 {
        return true;
    }
    if !flag {
        let var = a.yaw_towards_player.wrapping_sub(i16::MIN).wrapping_sub(player_yaw);
        let dist = if !player_has_target && (var as i32).abs() > 0x2AAA { f32::MAX } else { a.xyz_dist_to_player_sq };
        let leash = ranges.get(a.target_mode as usize).map(|r| r.1).unwrap_or(1.0);
        return !in_range(ranges, a, leash * dist);
    }
    false
}

/// `Actor_GetScreenPos`: where the actor's focus is on the 320x240 screen (`Actor_ProjectPos`
/// with `cappedInvW`: 1 / w, or 1 when w < 1).
pub fn actor_screen_pos(view_proj: Mat4, a: &Actor) -> (i16, i16) {
    let p = view_proj * a.focus_pos.extend(1.0);
    let inv_w = if p.w < 1.0 { 1.0 } else { 1.0 / p.w };
    ((160.0 + p.x * inv_w * 160.0) as i16, (120.0 - p.y * inv_w * 120.0) as i16)
}

/// `Actor_ProjectPos`: `pos` through the view, and `cappedInvW` (1 / w, or 1 when w < 1).
pub fn project_pos(view_proj: Mat4, pos: Vec3) -> (Vec3, f32) {
    let p = view_proj * pos.extend(1.0);
    (p.truncate(), if p.w < 1.0 { 1.0 } else { 1.0 / p.w })
}

/// `Attention_ActorOnScreen`: `Actor_GetScreenPos` inside (-20..340, -160..400) on the 320x240 screen.
fn on_screen(view_proj: Mat4, a: &Actor) -> bool {
    let (sx, sy) = actor_screen_pos(view_proj, a);
    sx > -20 && sx < 340 && sy > -160 && sy < 400
}

/// `Attention_Update`, called with the actor Player keeps targeted; the lock's sounds.
pub fn update(ctx: &mut TargetCtx, actors: &ActorContext, f: &TargetFrame, audio: &mut crate::audio::GameAudio) {
    use crate::audio::sfx::{NA_SE_SY_LOCK_OFF, NA_SE_SY_LOCK_ON, NA_SE_SY_LOCK_ON_HUMAN};
    // Actor_UpdateAll: only a locked target with zTargetActiveTimer >= 5 counts; losing the lock sounds.
    let mut locked = f.player_target.filter(|&h| actors.actor(h).is_some_and(|a| !a.killed));
    if locked.is_none() || f.player_timer < 5 {
        locked = None;
        if ctx.reticle_spin_counter != 0 {
            ctx.reticle_spin_counter = 0;
            audio.play_sfx_centered(NA_SE_SY_LOCK_OFF);
        }
    }
    let mut candidate = None;
    if !(f.player_target.is_some() && f.player_stick_dir == 2) {
        // Attention_FindActor → Attention_FindActorInCategory over the actors, in list order.
        let mut best = f32::MAX;
        let mut best_prio: Option<(u8, ActorHandle)> = None;
        // sBgmEnemyDistSq: the nearest hostile enemy within 500, the locked one too.
        ctx.bgm_enemy = None;
        let mut bgm_enemy_dist_sq = f32::MAX;
        for h in actors.all() {
            let Some(a) = actors.actor(h) else { continue };
            if h == f.player || a.killed || a.flags & ACTOR_FLAG_ATTENTION_ENABLED == 0 {
                continue;
            }
            if a.category == crate::actor_ctx::ACTORCAT_ENEMY && a.flags & ACTOR_FLAG_HOSTILE != 0 && a.xyz_dist_to_player_sq < 500.0 * 500.0 && a.xyz_dist_to_player_sq < bgm_enemy_dist_sq {
                ctx.bgm_enemy = Some(h);
                bgm_enemy_dist_sq = a.xyz_dist_to_player_sq;
            }
            if Some(h) == f.player_target {
                continue;
            }
            let var = weighted_dist(a, f.player_target.is_some(), f.player_shape_yaw);
            let visible = f
                .col
                .check_line(bgcheck::IGNORE_CAMERA, bgcheck::IGNORE_NONE, f.player_focus, a.focus_pos, 1.0, bgcheck::CHECK_WALL | bgcheck::CHECK_FLOOR | bgcheck::CHECK_CEILING | bgcheck::CHECK_ONE_FACE | bgcheck::CHECK_DYNA)
                .is_none();
            if var < best && in_range(f.ranges, a, var) && on_screen(f.view_proj, a) && visible {
                if a.target_priority != 0 {
                    if best_prio.is_none_or(|(p, _)| a.target_priority < p) {
                        best_prio = Some((a.target_priority, h));
                    }
                } else {
                    candidate = Some(h);
                    best = var;
                }
            }
        }
        if candidate.is_none() {
            candidate = best_prio.map(|(_, h)| h);
        }
    }
    ctx.arrow_hover_actor = candidate;
    let pointed = match ctx.forced_lock_on_actor.take() {
        Some(h) => Some(h),
        None => locked.or(candidate),
    };
    let player_category = actors.actor(f.player).map(|a| a.category).unwrap_or(crate::actor_ctx::ACTORCAT_PLAYER);
    let category = pointed.and_then(|h| actors.actor(h)).map(|a| a.category).unwrap_or(player_category);
    if pointed != ctx.arrow_pointed || category != ctx.active_category {
        ctx.arrow_pointed = pointed;
        ctx.active_category = category;
        ctx.navi_move_progress_factor = 1.0;
    }
    // Navi's point: eased towards the pointed actor (or Player) while naviMoveProgressFactor falls, then on it.
    if let Some(a) = pointed.or(Some(f.player)).and_then(|h| actors.actor(h)) {
        if !eng_math::step_to_f(&mut ctx.navi_move_progress_factor, 0.0, 0.25) {
            let temp1 = 0.25 / ctx.navi_move_progress_factor;
            let d = Vec3::new(a.world_pos.x, a.world_pos.y + (a.target_arrow_offset * a.scale.y), a.world_pos.z) - ctx.navi_ref_pos;
            ctx.navi_ref_pos += d * temp1;
        } else {
            ctx.set_navi_to_actor(a, category);
        }
    }
    // Reticle: drop the lock if the target's focus is behind the eye or off the view before
    // locking.
    let mut arg = locked;
    if let Some(h) = arg
        && ctx.reticle_spin_counter == 0
    {
        let visible = actors.actor(h).is_some_and(|a| {
            let (p, inv_w) = project_pos(f.view_proj, a.focus_pos);
            !(p.z <= 0.0 || 1.0 <= (p.x * inv_w).abs() || 1.0 <= (p.y * inv_w).abs())
        });
        if !visible {
            arg = None;
        }
    }
    if let Some(h) = arg
        && let Some(a) = actors.actor(h)
    {
        if ctx.targeted != Some(h) {
            ctx.attention_init_reticle(a.category, f.view_eye);
            ctx.targeted = Some(h);
            // ACTOR_EN_BOOM (reticleFadeAlphaControl 0): the boomerang isn't ported.
            audio.play_sfx_centered(if a.is_hostile() { NA_SE_SY_LOCK_ON } else { NA_SE_SY_LOCK_ON_HUMAN });
        }
        ctx.target_center_pos = Vec3::new(a.world_pos.x, a.world_pos.y - (a.shape_y_offset * a.scale.y), a.world_pos.z);
        if ctx.reticle_spin_counter == 0 {
            let t5 = (500.0 - ctx.reticle_radius) * 3.0;
            let t6 = t5.clamp(30.0, 100.0);
            if eng_math::step_to_f(&mut ctx.reticle_radius, 80.0, t6) {
                ctx.reticle_spin_counter += 1;
            }
        } else {
            ctx.reticle_spin_counter = (ctx.reticle_spin_counter.wrapping_add(3)) | 0x80;
            ctx.reticle_radius = 120.0;
        }
    } else {
        ctx.targeted = None;
        eng_math::step_to_f(&mut ctx.reticle_radius, 500.0, 80.0);
    }
}

/// What `Attention_Draw` reads from Player.
#[derive(Debug, Clone, Copy)]
pub struct ReticlePlayer {
    /// `stateFlags1 & PLAYER_STATE1_TALKING`.
    pub state1_6: bool,
    /// `focusActor`.
    pub target: Option<ActorHandle>,
}

/// `Attention_Draw`'s state changes, once per game frame after the view is set up: the
/// reticle follows the target's focus, flying in from the screen's centre as `reticleRadius` shrinks
/// (`var1`), or fades out (`reticleFadeAlphaControl` −120 a frame) where it was when the target was lost.
pub fn draw_update(ctx: &mut TargetCtx, actors: &ActorContext, view_proj: Mat4, player: ReticlePlayer) {
    ctx.reticle = None;
    if ctx.reticle_fade_alpha_control == 0 {
        return;
    }
    let actor = ctx.targeted.and_then(|h| actors.actor(h));
    let mut alpha = 0xFF;
    let mut var1 = 1.0;
    let count = if ctx.reticle_spin_counter != 0 { 1 } else { 3 };
    if let Some(a) = actor {
        ctx.target_center_pos = a.focus_pos;
        var1 = (500.0 - ctx.reticle_radius) / 420.0;
    } else {
        ctx.reticle_fade_alpha_control -= 120;
        if ctx.reticle_fade_alpha_control < 0 {
            ctx.reticle_fade_alpha_control = 0;
        }
        alpha = ctx.reticle_fade_alpha_control;
    }
    let (mut p, inv_w) = project_pos(view_proj, ctx.target_center_pos);
    p.x = ((160.0 * (p.x * inv_w)) * var1).clamp(-320.0, 320.0);
    p.y = ((120.0 * (p.y * inv_w)) * var1).clamp(-240.0, 240.0);
    p.z *= var1;
    ctx.cur_reticle -= 1;
    if ctx.cur_reticle < 0 {
        ctx.cur_reticle = 2;
    }
    ctx.attention_set_reticle_pos(ctx.cur_reticle as usize, p);
    if !player.state1_6 || ctx.targeted != player.target {
        ctx.reticle = Some(ReticleDraw { alpha, count });
    }
}

/// `Attention_Draw`'s draws: the lock-on triangles into the overlay list (`OVERLAY_DISP`) and
/// the arrow over `arrowHoverActor` into the XLU list.
pub fn draw(ctx: &TargetCtx, actors: &ActorContext, gameplay_frames: u32, out: &mut DrawLists) {
    use std::f32::consts::PI;
    let prim = |c: [u8; 3], a: u8| {
        let mut sv = SegmentValues::default();
        sv.prim[SEG_PRIM as usize] = Some([c[0], c[1], c[2], a]);
        DrawParams { segments: Some(sv), ..Default::default() }
    };
    if let Some(r) = ctx.reticle {
        let mut alpha = r.alpha;
        let mut idx = ctx.cur_reticle as usize;
        for _ in 0..r.count {
            let e = ctx.arr_50[idx];
            if e.radius < 500.0 {
                let var2 = if e.radius <= 120.0 { 0.15 } else { ((e.radius - 120.0) * 0.001) + 0.15 };
                let mut m = Mat4::from_translation(Vec3::new(e.pos.x, e.pos.y, 0.0))
                    * Mat4::from_scale(Vec3::new(var2, 0.15, 1.0))
                    * Mat4::from_rotation_z((ctx.reticle_spin_counter & 0x7F) as f32 * (PI / 64.0));
                for _ in 0..4 {
                    m *= Mat4::from_rotation_z(PI / 2.0);
                    let t = m * Mat4::from_translation(Vec3::new(e.radius, e.radius, 0.0));
                    out.overlay_2d.push(DrawCmd { mesh: MeshKey::named(keys::bake(LOCK_ON_TRIANGLE)), transform: t, bones: Vec::new(), params: prim(e.color, alpha as u8) });
                }
            }
            alpha -= 0xFF / 3;
            if alpha < 0 {
                alpha = 0;
            }
            idx = (idx + 1) % 3;
        }
    }
    if let Some(a) = ctx.arrow_hover_actor.and_then(|h| actors.actor(h))
        && a.flags & ACTOR_FLAG_LOCK_ON_DISABLED == 0
    {
        // iREG(27..29) are 0.
        let m = Mat4::from_translation(Vec3::new(a.focus_pos.x, a.focus_pos.y + (a.target_arrow_offset * a.scale.y) + 17.0, a.focus_pos.z))
            * Mat4::from_rotation_y(gameplay_frames.wrapping_mul(3000) as u16 as f32 * (PI / 32768.0))
            * Mat4::from_scale(Vec3::new(35.0 / 1000.0, 60.0 / 1000.0, 50.0 / 1000.0));
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(TARGET_ARROW)), transform: m, bones: Vec::new(), params: prim(navi_inner(a.category), 255) });
    }
}

/// `Math_Vec3f_Yaw(a, b)`.
pub fn yaw_to(a: Vec3, b: Vec3) -> i16 {
    atan2_s(b.z - a.z, b.x - a.x)
}

/// `Math_Vec3f_Pitch(a, b)`: `Math_Atan2S(xz distance, a.y - b.y)`.
pub fn pitch_to(a: Vec3, b: Vec3) -> i16 {
    let d = ((b.x - a.x).powi(2) + (b.z - a.z).powi(2)).sqrt();
    atan2_s(d, a.y - b.y)
}
