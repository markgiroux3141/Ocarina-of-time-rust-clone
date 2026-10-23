//! GAME-10 milestone 8 against the C: the Bari (`z_en_vali.c`). Master Quest's Jabu-Jabu (`bdan`)
//! room 6 places one (params 0xFFFF) at (-1367, 411, -2490).
//!
//! Child Link enters at `ENTR_JABU_JABU_0` on a `jabu-fish` save and the room changes to room 6
//! (`Room_RequestNewRoom`, as the game's `--room`). The tests run the placed Bari's update alone
//! (`Actor_UpdateAll`'s call), Link's distance and the hits set as `Actor_UpdateAll` and the
//! collision check would, so no fight is scripted and nothing waits on `Rand`. Expected values
//! are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::en_bili::{ACTOR_EN_BILI, EN_BILI_TYPE_SPAWNED_BY_BARI, EnBili, NA_SE_EN_BIRI_SPARK, OBJECT_BL};
use oot_actors::en_vali::*;
use oot_game::actor::*;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::{NA_SE_EN_DODO_M_GND, NA_SE_EN_GOMA_JR_FREEZE, SFX_FLAG, SfxPos};
use oot_game::collision_check::*;
use oot_game::effect::{EFFECT_SS_DT_BUBBLE, EFFECT_SS_EN_ICE, EFFECT_SS_LIGHTNING};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::shadow::ShadowDraw;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Room 6's Bari (`ootx scene-info --scene bdan --layer 0`).
const HOME: Vec3 = Vec3::new(-1367.0, 411.0, -2490.0);

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in Jabu-Jabu (`jabu-fish`), the sound log on, in room 6.
fn room_6(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("jabu-fish").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert!(w.room_request(6));
    idle(&mut w, 1);
    w.room_change_done();
    assert_eq!(w.room_ctx.cur.num, 6);
    // Room 6's objects come in over a few frames (its actors wait as `Uninit` till then).
    for _ in 0..30 {
        if w.object_ctx.get_index(OBJECT_VALI).is_some_and(|i| w.object_ctx.is_loaded(i)) && w.object_ctx.get_index(OBJECT_BL).is_some_and(|i| w.object_ctx.is_loaded(i)) && valis(&w).len() == 1 {
            break;
        }
        idle(&mut w, 1);
    }
    w
}

fn valis(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnVali>(h).is_some_and(|b| !b.actor.killed)).collect()
}

/// The Biri a Bari threw out (params 0).
fn thrown_bilis(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnBili>(h).is_some_and(|b| !b.actor.killed && b.actor.params == EN_BILI_TYPE_SPAWNED_BY_BARI)).collect()
}

fn vl(w: &PlayState, h: ActorHandle) -> &EnVali {
    w.actors.downcast::<EnVali>(h).expect("En_Vali")
}

fn vl_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnVali {
    w.actors.downcast_mut::<EnVali>(h).expect("En_Vali")
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<EnVali>(h).is_none_or(|b| b.actor.killed)
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// Its draw's writes (`EnVali_Draw`'s, after the frame's updates).
fn draw_update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    a.draw_update(w);
    w.actors.put_back(h, a);
}

/// `Actor_UpdateAll`'s distance: Link `dist` across.
fn set_link(w: &mut PlayState, h: ActorHandle, dist: f32) {
    vl_mut(w, h).actor.xz_dist_to_player = dist;
}

/// The live soft sprites of a type.
fn effects(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.ty == ty && e.life >= 0).count()
}

/// The times a sound was asked for at `h`.
fn sfx(w: &PlayState, h: ActorHandle, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == SfxPos::Actor(h)).count()
}

/// The times a sound was asked for anywhere.
fn sfx_any(w: &PlayState, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
}

/// Updates until the action changes; the updates run.
fn until_not(w: &mut PlayState, h: ActorHandle, action: Action, max: usize) -> usize {
    for n in 1..=max {
        update(w, h);
        if gone(w, h) || vl(w, h).action != action {
            return n;
        }
    }
    panic!("still {action:?} after {max}");
}

/// A hit this frame: `damage` and `reaction` (`CollisionCheck_ApplyDamage`'s), from `flags`.
fn hit(w: &mut PlayState, h: ActorHandle, damage: u8, reaction: u8, flags: u32) {
    let link = w.player.expect("Link");
    let b = vl_mut(w, h);
    b.collider.base.ac_flags |= AC_HIT;
    b.collider.base.ac = Some(link);
    b.collider.info.ac_hit_elem = Some(HitElem {
        elem: ElemRef { col: ColliderRef { actor: link, id: 0 }, elem: 0 },
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: flags, hit_special_effect: 0, damage },
        ac_dmg_info: Default::default(),
        elem_material: 0,
    });
    b.actor.col_chk_info.damage = damage;
    b.actor.col_chk_info.damage_reaction = reaction;
}

/// `DEG_TO_BINANG`.
fn deg_to_binang(d: f32) -> i16 {
    (d * (32768.0f32 / 180.0)) as i32 as i16
}

/// The placed Bari, Link near: down on the floor and spinning, risen to 40 over the floor.
fn spinning(w: &mut PlayState) -> ActorHandle {
    let h = valis(w)[0];
    set_link(w, h, 100.0);
    update(w, h);
    until_not(w, h, Action::Appear, 100);
    assert_eq!(vl(w, h).action, Action::Spin);
    // spinHeight up 1.2 a frame to floorHeight + 40 (34 frames).
    for _ in 0..40 {
        update(w, h);
    }
    let b = vl(w, h);
    assert_eq!(b.spin_height, b.actor.floor_height + 40.0);
    h
}

#[test]
fn room_6s_bari_as_init_sets_it() {
    let Some(a) = assets() else { return };
    let w = room_6(&a);
    let placed = valis(&w);
    assert_eq!(placed.len(), 1);
    let b = vl(&w, placed[0]);
    assert_eq!((b.actor.home_pos.x, b.actor.home_pos.z), (HOME.x, HOME.z));
    // params written 0; FLAGS less the lock-on (EnVali_Init); sInitChain: NAVI_ENEMY_BARI, scale
    // 0.01, lockOnArrowOffset 5000; ActorShape_Init(0, ActorShadow_DrawCircle, 27), alpha 155.
    assert_eq!(b.actor.params, 0);
    let f = ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | oot_actors::en_bili::ACTOR_FLAG_IGNORE_QUAKE;
    assert_eq!(b.actor.flags & (f | ACTOR_FLAG_ATTENTION_ENABLED), f);
    assert_eq!((b.actor.navi_enemy_id, b.actor.scale, b.actor.target_arrow_offset), (NAVI_ENEMY_BARI, Vec3::splat(0.01), 5000.0));
    assert_eq!((b.actor.shape_shadow_draw, b.actor.shape_shadow_scale, b.actor.shape_shadow_alpha), (ShadowDraw::Circle, 27.0, 155));
    // EnVali_SetupWaitPlayerNear: gBariIdleAnim, not drawn, not hittable.
    assert_eq!((b.action, b.drawn, b.collider.base.ac_flags & AC_ON), (Action::WaitPlayerNear, false, 0));
    assert!(b.skel.is(ANIM_IDLE));
    use oot_game::actor_ctx::ActorImpl;
    assert_eq!(b.render_state().shadow, None);
    // Its floor found once (BgCheck_EntityRaycastDown4 from its place): under it.
    let (floor, _) = w.col.entity_raycast_down(b.actor.home_pos);
    assert_eq!(b.actor.floor_height, floor);
    assert!(floor < HOME.y && floor > HOME.y - 1000.0, "{floor}");
    // sColChkInfoInit (2, 18, 32, MASS_HEAVY); sCylinderInit 17 by 35, 15 down, hurting 8 with
    // HIT_SPECIAL_EFFECT_7, an electric backlash; the quads hurting 8.
    let c = &b.actor.col_chk_info;
    assert_eq!((c.health, c.cyl_radius, c.cyl_height, c.mass), (2, 18, 32, MASS_HEAVY));
    assert_eq!((b.collider.dim.radius, b.collider.dim.height, b.collider.dim.y_shift), (17, 35, -15));
    assert_eq!((b.collider.info.at_dmg_info.damage, b.collider.info.at_dmg_info.hit_special_effect), (8, HIT_SPECIAL_EFFECT_7));
    assert_eq!(b.collider.info.ac_dmg_info.hit_backlash, HIT_BACKLASH_ELECTRIC);
    assert_eq!((b.left_tentacle_collider.info.at_dmg_info.damage, b.right_tentacle_collider.base.ac_flags), (8, AC_NONE));
    // gBariSkel: 28 limbs, the nucleus's, the core's and the hood's lists where BariLimb says.
    let sk = a.skeleton(OBJECT, SKEL).unwrap();
    assert_eq!(sk.limbs.len(), 28);
    assert_eq!(sk.limbs[BARI_LIMB_NUCLEUS - 1].dlists[0], 0x0600_2740);
    assert_eq!(sk.limbs[BARI_LIMB_CORE - 1].dlists[0], 0x0600_2610);
    assert_eq!(sk.limbs[BARI_LIMB_HOOD - 1].dlists[0], 0x0600_27D8);
    // gBariLeftTentacleBodyRootLimb, gBariRightTentacleBodyRootLimb: under their joint roots,
    // under the tentacles' roots (0-based parents).
    let parent = |l: usize| sk.parents[l - 1].map(|p| p as usize + 1);
    assert_eq!(parent(BARI_LIMB_LEFT_TENTACLE_BODY_ROOT), Some(BARI_LIMB_LEFT_TENTACLE_ROOT + 1));
    assert_eq!(parent(BARI_LIMB_LEFT_TENTACLE_ROOT + 1), Some(BARI_LIMB_LEFT_TENTACLE_ROOT));
    assert_eq!(parent(BARI_LIMB_RIGHT_TENTACLE_BODY_ROOT), Some(BARI_LIMB_RIGHT_TENTACLE_ROOT + 1));
    assert_eq!(parent(BARI_LIMB_RIGHT_TENTACLE_ROOT + 1), Some(BARI_LIMB_RIGHT_TENTACLE_ROOT));
}

#[test]
fn link_near_it_drops_spins_up_and_link_away_it_goes_back_up() {
    let Some(a) = assets() else { return };
    let mut w = room_6(&a);
    let h = valis(&w)[0];
    // 150 or over: waiting.
    set_link(&mut w, h, 150.0);
    update(&mut w, h);
    assert_eq!(vl(&w, h).action, Action::WaitPlayerNear);
    // Under 150: EnVali_SetupAppear (drawn, lock-on, velocity.y 1).
    set_link(&mut w, h, 149.0);
    update(&mut w, h);
    let b = vl(&w, h);
    assert_eq!((b.action, b.drawn, b.actor.velocity.y), (Action::Appear, true, 1.0));
    assert_ne!(b.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    // EnVali_Appear: 1.5 times faster each frame.
    let y0 = b.actor.world_pos.y;
    update(&mut w, h);
    assert_eq!((vl(&w, h).actor.velocity.y, vl(&w, h).actor.world_pos.y), (1.5, y0 - 1.5));
    update(&mut w, h);
    assert_eq!((vl(&w, h).actor.velocity.y, vl(&w, h).actor.world_pos.y), (2.25, y0 - 3.75));
    // On the floor: NA_SE_EN_DODO_M_GND, EnVali_SetupSpin (gBariAttackAnim, the quads' corners
    // 10 under it, updating only near, hittable, spinHeight its height).
    until_not(&mut w, h, Action::Appear, 100);
    let b = vl(&w, h);
    assert_eq!((b.action, b.actor.world_pos.y, b.spin_height, b.spin_timer), (Action::Spin, b.actor.floor_height, b.actor.floor_height, 0));
    assert_eq!(sfx(&w, h, NA_SE_EN_DODO_M_GND), 1);
    assert!(b.skel.is(ANIM_ATTACK));
    let p = b.actor.world_pos - Vec3::Y * 10.0;
    assert_eq!((b.left_tentacle_collider.dim.quad, b.right_tentacle_collider.dim.quad), ([p; 4], [p; 4]));
    assert_eq!((b.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, b.collider.base.ac_flags & AC_ON), (0, AC_ON));
    // EnVali_Spin: spinHeight up 1.2, bobbing 8 down with the frame, turning (frame + 4) × 0.4
    // degrees (the frame folded at 40).
    let (rot0, sh0) = (b.actor.shape_rot.y, b.spin_height);
    update(&mut w, h);
    let b = vl(&w, h);
    let f = b.skel.cur_frame as i32;
    assert_eq!(b.spin_height, sh0 + 1.2);
    assert_eq!(b.actor.world_pos.y, b.spin_height - (((f as f64 * std::f64::consts::PI * 0.0125f32 as f64) as f32).sin() * 8.0));
    let fold = if f > 40 { 80 - f } else { f };
    assert_eq!(b.actor.shape_rot.y, rot0.wrapping_add(deg_to_binang((fold + 4) as f32 * 0.4)));
    // NA_SE_EN_BARI_ROLL at frames 16, 30, 42 and 55 of each loop.
    let rolls = sfx(&w, h, NA_SE_EN_BARI_ROLL);
    let mut on = 0;
    for _ in 0..80 {
        update(&mut w, h);
        if matches!(vl(&w, h).skel.cur_frame as i32, 16 | 30 | 42 | 55) {
            on += 1;
        }
    }
    assert_eq!(sfx(&w, h, NA_SE_EN_BARI_ROLL), rolls + on);
    assert!(on >= 4);
    // Over 250: EnVali_SetupBury (always updating, no lock-on), back to its place by halves.
    set_link(&mut w, h, 251.0);
    update(&mut w, h);
    let b = vl(&w, h);
    assert_eq!(b.action, Action::Bury);
    assert_ne!(b.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    assert_eq!(b.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    until_not(&mut w, h, Action::Bury, 200);
    let b = vl(&w, h);
    assert_eq!((b.action, b.drawn, b.collider.base.ac_flags & AC_ON), (Action::WaitPlayerNear, false, 0));
    assert!((b.actor.world_pos.y - HOME.y).abs() < 0.1, "{}", b.actor.world_pos.y);
}

#[test]
fn touching_link_it_shocks_20_frames_then_spins_on() {
    let Some(a) = assets() else { return };
    let mut w = room_6(&a);
    let h = spinning(&mut w);
    let bolts = effects(&w, EFFECT_SS_LIGHTNING);
    let y0 = vl(&w, h).actor.world_pos.y;
    vl_mut(&mut w, h).left_tentacle_collider.base.at_flags |= AT_HIT;
    update(&mut w, h);
    // EnVali_SetupElectrify (the AT hit forgotten): 20, no lock-on, not hittable; its first
    // frame: 19, no bolts ((19 + 2i) % 4 never 0), the spark, up 1 (odd).
    let b = vl(&w, h);
    assert_eq!((b.action, b.electrify_timer, b.left_tentacle_collider.base.at_flags & AT_HIT), (Action::Electrify, 19, 0));
    assert_eq!((b.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, b.collider.base.ac_flags & AC_ON), (0, 0));
    assert_eq!(b.actor.sfx, NA_SE_EN_BIRI_SPARK - SFX_FLAG);
    assert_eq!(effects(&w, EFFECT_SS_LIGHTNING), bolts);
    assert_eq!(b.actor.world_pos.y, y0 + 1.0);
    // The core's colours inverted on odd counts.
    assert_eq!(b.render_state().switches[1], 1);
    // 18: two bolts (i 1 and 3), down 1.
    update(&mut w, h);
    assert_eq!(effects(&w, EFFECT_SS_LIGHTNING), bolts + 2);
    assert_eq!((vl(&w, h).actor.world_pos.y, vl(&w, h).render_state().switches[1]), (y0, 0));
    // The count out (20 frames in all): spinning again (no new animation), lock-on, hittable;
    // shaken up one more time than down.
    assert_eq!(until_not(&mut w, h, Action::Electrify, 100), 18);
    let b = vl(&w, h);
    assert_eq!((b.action, b.electrify_timer, b.collider.base.ac_flags & AC_ON), (Action::Spin, 0, AC_ON));
    assert_ne!(b.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(effects(&w, EFFECT_SS_LIGHTNING), bolts + 20);
    // (Its world y is the spin's again on this frame.)
    assert!(b.skel.is(ANIM_ATTACK));
}

#[test]
fn a_sword_sets_off_its_shock_then_damages_it_and_two_kill_it_into_three_biri() {
    let Some(a) = assets() else { return };
    let mut w = room_6(&a);
    let h = spinning(&mut w);
    // Its cylinder shocks a sword back (the collision check's backlash on Link).
    assert_eq!(vl(&w, h).collider.info.ac_dmg_info.hit_backlash, HIT_BACKLASH_ELECTRIC);
    hit(&mut w, h, 1, EN_VALI_DMG_REACT_SWORD_SLASH, DMG_SLASH_KOKIRI);
    update(&mut w, h);
    // EnVali_CheckCollide: 2 - 1, NA_SE_EN_BARI_DAMAGE; red 150 30 frames, params 1, its shock.
    let b = vl(&w, h);
    assert_eq!((b.action, b.actor.col_chk_info.health, b.actor.params, b.electrify_timer), (Action::Electrify, 1, 1, 19));
    assert_eq!((b.actor.color_filter_params & COLORFILTER_COLORFLAG_RED, b.actor.color_filter_timer), (COLORFILTER_COLORFLAG_RED, 30));
    assert_eq!(sfx(&w, h, NA_SE_EN_BARI_DAMAGE), 1);
    // The shock out: EnVali_SetupDamaged (gBariDamagedAnim, params 0, not hittable).
    until_not(&mut w, h, Action::Electrify, 100);
    let b = vl(&w, h);
    assert_eq!((b.action, b.actor.params, b.collider.base.ac_flags & AC_ON), (Action::Damaged, 0, 0));
    assert!(b.skel.is(ANIM_DAMAGED));
    // Health left: EnVali_Recover (gBariRecoverAnim), then spinning.
    until_not(&mut w, h, Action::Damaged, 200);
    assert_eq!(vl(&w, h).action, Action::Recover);
    assert!(vl(&w, h).skel.is(ANIM_RECOVER));
    until_not(&mut w, h, Action::Recover, 200);
    assert_eq!((vl(&w, h).action, vl(&w, h).collider.base.ac_flags & AC_ON), (Action::Spin, AC_ON));
    // A second (the Master Sword's 2): its last health, NA_SE_EN_BARI_DEAD, the finishing blow,
    // lock-on off.
    hit(&mut w, h, 2, EN_VALI_DMG_REACT_SWORD_SLASH, DMG_SLASH_MASTER);
    w.actors.freeze_flash_timer = 0;
    update(&mut w, h);
    let b = vl(&w, h);
    assert_eq!((b.action, b.actor.col_chk_info.health, b.actor.params), (Action::Electrify, 0, 1));
    assert_eq!(b.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!((sfx(&w, h, NA_SE_EN_BARI_DEAD), sfx(&w, h, NA_SE_EN_BARI_DAMAGE)), (1, 1));
    assert_eq!(w.actors.freeze_flash_timer, 5);
    until_not(&mut w, h, Action::Electrify, 100);
    assert_eq!(vl(&w, h).action, Action::Damaged);
    // The animation done with no health: EnVali_SetupDie: three Biri (params 0) where it is,
    // its yaw 0x5555 apart, NA_SE_EN_BARI_SPLIT, gone from view, not hittable.
    let yaw = vl(&w, h).actor.world_rot.y;
    let splits = sfx_any(&w, NA_SE_EN_BARI_SPLIT);
    assert!(thrown_bilis(&w).is_empty());
    until_not(&mut w, h, Action::Damaged, 200);
    let b = vl(&w, h);
    assert_eq!((b.action, b.drawn, b.collider.base.ac_flags & AC_ON), (Action::Die, false, 0));
    assert!((10..20).contains(&b.timer), "{}", b.timer);
    assert_eq!(sfx_any(&w, NA_SE_EN_BARI_SPLIT), splits + 1);
    let p = b.actor.world_pos;
    let bilis = thrown_bilis(&w);
    assert_eq!(bilis.len(), 3);
    let mut yaws: Vec<i16> = bilis.iter().map(|&bh| w.actors.downcast::<EnBili>(bh).unwrap().actor.world_rot.y).collect();
    yaws.sort();
    let mut want = vec![yaw, yaw.wrapping_add(0x5555), yaw.wrapping_add(0x5555).wrapping_add(0x5555)];
    want.sort();
    assert_eq!(yaws, want);
    for &bh in &bilis {
        let bb = w.actors.downcast::<EnBili>(bh).unwrap();
        assert_eq!((bb.actor.id, bb.actor.home_pos), (ACTOR_EN_BILI, p));
        assert_eq!(bb.action, oot_actors::en_bili::Action::SpawnedByBari);
    }
    assert_eq!(vl(&w, h).actor.world_rot.y, yaw.wrapping_add(0x5555).wrapping_add(0x5555).wrapping_add(0x5555));
    // EnVali_Die: two bubbles a frame, gone when the count's out.
    vl_mut(&mut w, h).timer = 3;
    let bubbles = effects(&w, EFFECT_SS_DT_BUBBLE);
    update(&mut w, h);
    update(&mut w, h);
    assert!(!gone(&w, h));
    assert_eq!(effects(&w, EFFECT_SS_DT_BUBBLE), bubbles + 4);
    update(&mut w, h);
    assert!(gone(&w, h));
    assert_eq!(effects(&w, EFFECT_SS_DT_BUBBLE), bubbles + 6);
}

#[test]
fn a_deku_nut_stuns_it_80_frames_and_a_sword_then_damages_it_at_once() {
    let Some(a) = assets() else { return };
    let mut w = room_6(&a);
    let h = spinning(&mut w);
    let y0 = vl(&w, h).actor.world_pos.y;
    hit(&mut w, h, 0, EN_VALI_DMG_REACT_STUN, DMG_DEKU_NUT);
    update(&mut w, h);
    // EnVali_SetupStunned: gBariAttackAnim, 80 (79 after its frame), blue 255 80 frames, no
    // backlash, NA_SE_EN_GOMA_JR_FREEZE (no NA_SE_EN_BARI_DAMAGE), falling from 1 (then 2).
    let b = vl(&w, h);
    assert_eq!((b.action, b.timer, b.actor.col_chk_info.health), (Action::Stunned, 79, 2));
    assert_eq!(b.collider.info.ac_dmg_info.hit_backlash, HIT_BACKLASH_NONE);
    assert_eq!((b.actor.color_filter_params & COLORFILTER_COLORFLAG_RED, b.actor.color_filter_timer), (0, 80));
    assert_eq!((sfx(&w, h, NA_SE_EN_GOMA_JR_FREEZE), sfx(&w, h, NA_SE_EN_BARI_DAMAGE)), (1, 0));
    assert_eq!((b.actor.world_pos.y, b.actor.velocity.y), (y0 - 1.0, 2.0));
    // Squashed by the count (EnVali_GetHoodScale, _GetCoreScale).
    let f = ((79.0f64 * (std::f64::consts::PI / 10.0)) as f32).sin() * 0.08;
    assert_eq!(b.body_scales(), [Vec3::new(1.0 - f, 1.0 + f, 1.0 - f), Vec3::new(1.0 + f, 1.0 - f, 1.0 + f)]);
    // A second nut doesn't start it again.
    hit(&mut w, h, 0, EN_VALI_DMG_REACT_STUN, DMG_DEKU_NUT);
    update(&mut w, h);
    assert_eq!(vl(&w, h).timer, 78);
    // Down on the floor: NA_SE_EN_DODO_M_GND, still.
    let lands = sfx(&w, h, NA_SE_EN_DODO_M_GND);
    for _ in 0..20 {
        update(&mut w, h);
    }
    let b = vl(&w, h);
    assert_eq!((b.actor.world_pos.y, b.actor.velocity.y), (b.actor.floor_height, 0.0));
    assert_eq!(sfx(&w, h, NA_SE_EN_DODO_M_GND), lands + 1);
    // A sword while stunned: EnVali_SetupDamaged at once (no shock).
    hit(&mut w, h, 1, EN_VALI_DMG_REACT_SWORD_SLASH, DMG_SLASH_KOKIRI);
    update(&mut w, h);
    let b = vl(&w, h);
    assert_eq!((b.action, b.actor.col_chk_info.health, b.actor.params), (Action::Damaged, 1, 0));
}

#[test]
fn stunned_80_frames_it_spins_again_with_its_backlash() {
    let Some(a) = assets() else { return };
    let mut w = room_6(&a);
    let h = spinning(&mut w);
    // The boomerang stuns it too.
    hit(&mut w, h, 0, EN_VALI_DMG_REACT_STUN, DMG_BOOMERANG);
    update(&mut w, h);
    assert_eq!(until_not(&mut w, h, Action::Stunned, 100), 79);
    let b = vl(&w, h);
    assert_eq!((b.action, b.collider.info.ac_dmg_info.hit_backlash, b.spin_height), (Action::Spin, HIT_BACKLASH_ELECTRIC, b.actor.floor_height));
}

#[test]
fn fire_burns_it_into_three_biri_in_two_frames() {
    let Some(a) = assets() else { return };
    let mut w = room_6(&a);
    let h = spinning(&mut w);
    hit(&mut w, h, 4, EN_VALI_DMG_REACT_FIRE, DMG_ARROW_FIRE);
    update(&mut w, h);
    // EnVali_SetupBurnt: 2 (1 after its frame), not hittable, red; 2 - 4: its last health.
    let b = vl(&w, h);
    assert_eq!((b.action, b.timer, b.actor.col_chk_info.health, b.collider.base.ac_flags & AC_ON), (Action::Burnt, 1, 0, 0));
    assert_eq!(sfx(&w, h, NA_SE_EN_BARI_DEAD), 1);
    update(&mut w, h);
    assert_eq!(vl(&w, h).action, Action::Die);
    assert_eq!(thrown_bilis(&w).len(), 3);
}

#[test]
fn ice_freezes_it_then_it_falls_and_splits() {
    let Some(a) = assets() else { return };
    let mut w = room_6(&a);
    let h = spinning(&mut w);
    let ice = effects(&w, EFFECT_SS_EN_ICE);
    hit(&mut w, h, 4, EN_VALI_DMG_REACT_ICE, DMG_ARROW_ICE);
    update(&mut w, h);
    // EnVali_SetupFrozen: still, blue 36, not hittable, 36 (35: a chunk of ice on odd counts
    // over 20, the tint held at 36).
    let b = vl(&w, h);
    assert_eq!((b.action, b.timer, b.actor.velocity.y, b.collider.base.ac_flags & AC_ON), (Action::Frozen, 35, 0.0, 0));
    assert_eq!(b.actor.color_filter_timer, 36);
    assert_eq!(effects(&w, EFFECT_SS_EN_ICE), ice + 1);
    for _ in 0..34 {
        update(&mut w, h);
    }
    // 35 to 21's odd counts: eight chunks; at 1 still up.
    let b = vl(&w, h);
    assert_eq!((b.timer, b.actor.velocity.y), (1, 0.0));
    assert_eq!(effects(&w, EFFECT_SS_EN_ICE), ice + 8);
    // At 0 it falls (1, 2, ...) 40 to the floor, then dies, the tint off.
    let n = until_not(&mut w, h, Action::Frozen, 100);
    assert!(n > 5 && n < 12, "{n}");
    let b = vl(&w, h);
    assert_eq!((b.action, b.actor.color_filter_timer, b.actor.world_pos.y), (Action::Die, 0, b.actor.floor_height));
    assert_eq!(thrown_bilis(&w).len(), 3);
}

#[test]
fn a_slingshot_seed_spins_it_fast_20_frames_and_other_hits() {
    let Some(a) = assets() else { return };
    let mut w = room_6(&a);
    let h = spinning(&mut w);
    let rot0 = vl(&w, h).actor.shape_rot.y;
    hit(&mut w, h, 0, EN_VALI_DMG_REACT_SLINGSHOT, DMG_SLINGSHOT);
    update(&mut w, h);
    // spinTimer 20 (19 after its frame): 0x800 a frame more; no sound of damage, no damage.
    let b = vl(&w, h);
    assert_eq!((b.action, b.spin_timer, b.actor.col_chk_info.health), (Action::Spin, 19, 2));
    assert_eq!(sfx(&w, h, NA_SE_EN_BARI_DAMAGE), 0);
    let f = b.skel.cur_frame as i32;
    let fold = if f > 40 { 80 - f } else { f };
    assert_eq!(b.actor.shape_rot.y, rot0.wrapping_add(0x800).wrapping_add(deg_to_binang((fold + 4) as f32 * 0.4)));
    // Light magic (0, no reaction): nothing.
    hit(&mut w, h, 0, EN_VALI_DMG_REACT_NONE, DMG_MAGIC_LIGHT);
    update(&mut w, h);
    assert_eq!((vl(&w, h).action, vl(&w, h).actor.col_chk_info.health), (Action::Spin, 2));
    // A Deku stick (2, no reaction): damaged at once, its last health.
    hit(&mut w, h, 2, EN_VALI_DMG_REACT_NONE, DMG_DEKU_STICK);
    update(&mut w, h);
    let b = vl(&w, h);
    assert_eq!((b.action, b.actor.col_chk_info.health), (Action::Damaged, 0));
    assert_eq!(sfx(&w, h, NA_SE_EN_BARI_DEAD), 1);
}

#[test]
fn its_tentacles_stretch_and_their_quads_sweep_in_the_draw() {
    let Some(a) = assets() else { return };
    let mut w = room_6(&a);
    let h = spinning(&mut w);
    // EnVali_UpdateTentaclesLengthFactor: spinning, frame 10 to 70 → 1.5; 0.1 a step, a step at
    // each root (left, then right).
    let b = vl_mut(&mut w, h);
    b.skel.cur_frame = 30.0;
    b.tentacles_length_factor = 1.0;
    let p = b.actor.world_pos - Vec3::Y * 10.0;
    b.left_tentacle_collider.dim.quad = [p; 4];
    b.right_tentacle_collider.dim.quad = [p; 4];
    draw_update(&mut w, h);
    let b = vl(&w, h);
    assert!((b.tentacles_length_factor - 1.2).abs() < 1e-6);
    assert!((b.drawn_factors[0] - 1.1).abs() < 1e-6 && (b.drawn_factors[1] - 1.2).abs() < 1e-6, "{:?}", b.drawn_factors);
    // EnVali_PostLimbDraw: each quad's new edge from its tentacle's body root, last frame's
    // edge behind it (Collider_SetQuadVertices(.., base, tip, quad[0], quad[1])).
    let (l, r) = (b.left_tentacle_collider.dim.quad, b.right_tentacle_collider.dim.quad);
    assert_eq!((l[2], l[3], r[2], r[3]), (p, p, p, p));
    assert_ne!(l[0], l[1]);
    assert_ne!(l[0], r[0]);
    draw_update(&mut w, h);
    let b = vl(&w, h);
    assert_eq!((b.left_tentacle_collider.dim.quad[2], b.left_tentacle_collider.dim.quad[3]), (l[0], l[1]));
    // The targets: spinning by the frame; damaged and recovering in and out; shocking or frozen
    // held; else 1.
    use std::f64::consts::PI;
    assert_eq!(tentacles_length_target(Action::Spin, 4.0, 0.0), (4.0f32 * 0.05) + 1.0);
    assert_eq!(tentacles_length_target(Action::Spin, 76.0, 0.0), (4.0 * 0.05) + 1.0);
    assert_eq!(tentacles_length_target(Action::Damaged, 5.0, 0.0), 1.0 - ((((PI / 10.0) * 5.0) as f32).sin() * 0.35));
    assert_eq!(tentacles_length_target(Action::Electrify, 5.0, 1.3), 1.3);
    assert_eq!(tentacles_length_target(Action::Bury, 5.0, 1.3), 1.0);
    // Not drawn (waiting, dying): no step.
    let b = vl_mut(&mut w, h);
    b.drawn = false;
    let f = b.tentacles_length_factor;
    draw_update(&mut w, h);
    assert_eq!(vl(&w, h).tentacles_length_factor, f);
}

#[test]
fn its_hood_and_core_squash_with_the_action() {
    use std::f64::consts::PI;
    // EnVali_GetHoodScale / _GetCoreScale: spinning by the frame (folded at 40).
    let s = |x: f64| (x as f32).sin();
    assert_eq!(hood_scale(Action::Spin, 50.0, 0, 0), Vec3::new(1.0, 1.0 - 0.2 * s((PI / 40.0) * 10.0), 1.0));
    assert_eq!(core_scale(Action::Spin, 10.0, 0, 0), Vec3::new(1.0, 1.0 - 0.13 * s((PI / 40.0) * 10.0), 1.0));
    // Shocking: by the count's place in its 10.
    assert_eq!(hood_scale(Action::Electrify, 0.0, 17, 0), Vec3::new(1.0, 1.0 - 0.2 * s(3.0 * (PI / 10.0)), 1.0));
    assert_eq!(core_scale(Action::Electrify, 0.0, 5, 0), Vec3::new(1.0, 1.0 - 0.13 * s(5.0 * (PI / 10.0)), 1.0));
    // Damaged and recovering: x and z with y.
    let f = s((PI / 10.0) * 4.0);
    assert_eq!(hood_scale(Action::Damaged, 4.0, 0, 0), Vec3::new(1.0 - 0.13 * f, 1.0 - 0.24 * f, 1.0 - 0.13 * f));
    let f = ((PI / 50.0) * 7.0) as f32;
    let f = f.cos();
    assert_eq!(core_scale(Action::Recover, 7.0, 0, 0), Vec3::new(1.0 - 0.1 * f, 1.0 - 0.18 * f, 1.0 - 0.1 * f));
}

#[test]
fn its_draw_is_the_core_three_nuclei_the_hood_and_the_skeleton() {
    let Some(a) = assets() else { return };
    // Each bake whole (nothing unresolved).
    for b in bakes() {
        let d: eng_gfx::DrawList = a.pack.assets.get(&oot_game::pack::keys::bake(&b.name)).expect(&b.name);
        assert!(d.triangle_count() > 0 && d.stats.unresolved_addresses.is_empty(), "{}", b.name);
    }
    // The combiners: the C's gsDPSetCombineLERP.
    use oot_game::gbi::{Dl, ac, cc_ab, cc_c, cc_d};
    let mut d = Dl::default();
    d.combine_lerp([cc_ab::ONE, cc_ab::TEXEL0, cc_c::SHADE, cc_d::ZERO, ac::TEXEL0, ac::ZERO, ac::PRIMITIVE, ac::ZERO], [cc_ab::COMBINED, cc_ab::ZERO, cc_c::PRIMITIVE, cc_d::ZERO, ac::TEXEL1, ac::ZERO, 6, ac::COMBINED]);
    d.combine_lerp([cc_ab::TEXEL0, cc_ab::ZERO, cc_c::SHADE, cc_d::ZERO, ac::TEXEL0, ac::ZERO, ac::PRIMITIVE, ac::ZERO], [cc_ab::COMBINED, cc_ab::ZERO, cc_c::PRIMITIVE, cc_d::ZERO, ac::TEXEL1, ac::ZERO, 6, ac::COMBINED]);
    assert_eq!((d.0[0], d.0[1]), (INVERT_COMBINER_DL[0], NORMAL_COMBINER_DL[0]));
    let mut w = room_6(&a);
    let h = spinning(&mut w);
    vl_mut(&mut w, h).left_tentacle_collider.base.at_flags |= AT_HIT;
    update(&mut w, h);
    draw_update(&mut w, h);
    // Shocking on an odd count: the inverted core, translucent; three nuclei, the hood, the
    // skeleton's pose.
    let b = vl(&w, h);
    let view = oot_game::play::ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
    let mut out = oot_game::play::DrawOut::default();
    b.draw(&b.render_state(), &w, &view, &mut out);
    assert_eq!((out.opa.len(), out.xlu.len()), (0, 6));
    let key = |n: &str| eng_gfx::MeshKey::named(oot_game::pack::keys::bake(n));
    assert_eq!(out.xlu[0].mesh, key(core_bake_name(true)));
    assert!(out.xlu[1..4].iter().all(|c| c.mesh == key(BAKE_NUCLEUS)));
    assert_eq!((out.xlu[4].mesh.clone(), out.xlu[5].mesh.clone()), (key(BAKE_HOOD), key(BAKE_SKEL)));
    assert_eq!(out.xlu[5].bones.len(), 28);
    // The nuclei: turned back from its yaw, the second 1114 up (model units) from the first.
    let m = &out.xlu[1..4];
    let (p0, p1) = (m[0].transform.w_axis.truncate(), m[1].transform.w_axis.truncate());
    assert!(((p1.y - p0.y) - 11.14).abs() < 1e-3, "{p0} {p1}");
    // Not drawn: nothing (nor a shadow).
    vl_mut(&mut w, h).drawn = false;
    let b = vl(&w, h);
    let mut out = oot_game::play::DrawOut::default();
    b.draw(&b.render_state(), &w, &view, &mut out);
    assert_eq!((out.opa.len(), out.xlu.len()), (0, 0));
}
