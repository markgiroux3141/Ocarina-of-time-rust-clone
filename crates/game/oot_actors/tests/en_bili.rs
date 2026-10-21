//! GAME-10 milestone 4 against the C: the Biri (`z_en_bili.c`). Master Quest's Jabu-Jabu (`bdan`)
//! room 1 places two (params 0xFFFF, `EN_BILI_TYPE_NORMAL`) at (146, -708, -1788) and (-134, -72,
//! -1614).
//!
//! Child Link enters at `ENTR_JABU_JABU_0` on a `jabu-fish` save and the room changes to room 1
//! (`Room_RequestNewRoom`, as the game's `--room`). The tests spawn a Biri of their own
//! (`object_bl` is room 1's) and run its update alone (`Actor_UpdateAll`'s call), Link's
//! distances and the hits set as `Actor_UpdateAll` and the collision check would, so no fight is
//! scripted and nothing waits on `Rand`. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_bili::*;
use oot_game::actor::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, SFX_FLAG, SfxPos};
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

/// Room 1's two (`ootx scene-info --scene bdan --layer 0`).
const HOMES: [Vec3; 2] = [Vec3::new(146.0, -708.0, -1788.0), Vec3::new(-134.0, -72.0, -1614.0)];

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in Jabu-Jabu (`jabu-fish`), the sound log on, in room 1.
fn room_1(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("jabu-fish").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert!(w.room_request(1));
    idle(&mut w, 1);
    w.room_change_done();
    assert_eq!(w.room_ctx.cur.num, 1);
    // Room 1's objects come in over a few frames (its actors wait as `Uninit` till then).
    for _ in 0..30 {
        if w.object_ctx.get_index(OBJECT_BL).is_some_and(|i| w.object_ctx.is_loaded(i)) && bilis(&w).len() == 2 {
            break;
        }
        idle(&mut w, 1);
    }
    w
}

fn bilis(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnBili>(h).is_some_and(|b| !b.actor.killed)).collect()
}

fn bl(w: &PlayState, h: ActorHandle) -> &EnBili {
    w.actors.downcast::<EnBili>(h).expect("En_Bili")
}

fn bl_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnBili {
    w.actors.downcast_mut::<EnBili>(h).expect("En_Bili")
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<EnBili>(h).is_none_or(|b| b.actor.killed)
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// `Actor_UpdateAll`'s distances: Link `dist` across at `yaw`, `y` above it.
fn set_link(w: &mut PlayState, h: ActorHandle, dist: f32, y: f32, yaw: i16) {
    let b = bl_mut(w, h);
    b.actor.xz_dist_to_player = dist;
    b.actor.y_dist_to_player = y;
    b.actor.yaw_towards_player = yaw;
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
        if gone(w, h) || bl(w, h).action != action {
            return n;
        }
    }
    panic!("still {action:?} after {max}");
}

/// A hit this frame: `damage` and `reaction` (`CollisionCheck_ApplyDamage`'s), from `flags`.
fn hit(w: &mut PlayState, h: ActorHandle, damage: u8, reaction: u8, flags: u32) {
    let link = w.player.expect("Link");
    let b = bl_mut(w, h);
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

/// A normal Biri of the test's own 150 over the floor under the higher home, Link far.
fn drifting(w: &mut PlayState) -> ActorHandle {
    let p = HOMES[0];
    let (floor, _) = w.col.entity_raycast_down(p + Vec3::Y * 50.0);
    let h = w.actor_spawn(ACTOR_EN_BILI, Vec3::new(p.x, floor + 150.0, p.z), [0, 0, 0], EN_BILI_TYPE_NORMAL).expect("the Biri");
    set_link(w, h, 1000.0, 0.0, 0);
    h
}

#[test]
fn room_1s_two_and_a_new_one_as_init_sets_them() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let placed = bilis(&w);
    assert_eq!(placed.len(), 2);
    for home in HOMES {
        let h = *placed.iter().find(|&&h| bl(&w, h).actor.home_pos.x == home.x && bl(&w, h).actor.home_pos.z == home.z).expect("placed");
        assert_eq!(bl(&w, h).actor.params, EN_BILI_TYPE_NORMAL);
    }
    let h = drifting(&mut w);
    let b = bl(&w, h);
    // FLAGS; sInitChain: NAVI_ENEMY_BIRI, lockOnArrowOffset 2000; ActorShape_Init(0,
    // ActorShadow_DrawCircle, 17), alpha 155.
    let f = ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_IGNORE_QUAKE | ACTOR_FLAG_CAN_ATTACH_TO_ARROW;
    assert_eq!(b.actor.flags & f, f);
    assert_eq!((b.actor.navi_enemy_id, b.actor.target_arrow_offset), (NAVI_ENEMY_BIRI, 2000.0));
    assert_eq!((b.actor.shape_shadow_draw, b.actor.shape_shadow_scale, b.actor.shape_shadow_alpha), (ShadowDraw::Circle, 17.0, 155));
    // EnBili_SetupIdle: 0.7, the backlash electric, 32, home at its height, no gravity, AT and AC.
    assert_eq!((b.action, b.actor.speed_xz, b.timer, b.actor.gravity), (Action::Idle, 0.7, 32, 0.0));
    assert_eq!(b.actor.home_pos.y, b.actor.world_pos.y);
    assert_eq!(b.collider.info.ac_dmg_info.hit_backlash, HIT_BACKLASH_ELECTRIC);
    assert_eq!((b.collider.base.at_flags & AT_ON, b.collider.base.ac_flags & AC_ON), (AT_ON, AC_ON));
    assert!(b.skel.is(ANIM_MAIN));
    // sColChkInfoInit (1, 9, 28, -20, 30); sCylinderInit 9 by 28, 20 down, hurting 8 with a shock.
    let c = &b.actor.col_chk_info;
    assert_eq!((c.health, c.cyl_radius, c.cyl_height, c.cyl_y_shift, c.mass), (1, 9, 28, -20, 30));
    assert_eq!((b.collider.dim.radius, b.collider.dim.height, b.collider.dim.y_shift), (9, 28, -20));
    assert_eq!((b.collider.info.at_dmg_info.damage, b.collider.info.at_dmg_info.hit_special_effect), (8, HIT_SPECIAL_EFFECT_ELECTRIC));
}

#[test]
fn thrown_out_of_a_bari_it_flies_up_and_out_then_drifts() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let p = HOMES[0];
    let h = w.actor_spawn(ACTOR_EN_BILI, p, [0, 0x5555, 0], EN_BILI_TYPE_SPAWNED_BY_BARI).expect("the Biri");
    set_link(&mut w, h, 1000.0, 0.0, 0);
    // EnBili_SetupSpawnedByBari: 25 frames, 6 up (gravity -0.3), 3 out, not hurting.
    let b = bl(&w, h);
    assert_eq!((b.action, b.timer, b.actor.velocity.y, b.actor.gravity, b.actor.speed_xz), (Action::SpawnedByBari, 25, 6.0, -0.3, 3.0));
    assert_eq!(b.collider.base.at_flags & AT_ON, 0);
    // Actor_MoveXZGravity: 6 - 0.3.
    update(&mut w, h);
    assert_eq!((bl(&w, h).timer, bl(&w, h).actor.velocity.y), (24, 5.7));
    let n = until_not(&mut w, h, Action::SpawnedByBari, 30);
    assert_eq!(n, 24);
    let b = bl(&w, h);
    assert_eq!((b.action, b.actor.speed_xz, b.actor.gravity, b.collider.base.at_flags & AT_ON), (Action::Idle, 0.7, 0.0, AT_ON));
}

#[test]
fn drifting_it_bobs_toward_40_over_link_and_comes_at_him_within_160() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = drifting(&mut w);
    // Its first frame: its floorHeight (0 from the zeroed actor until its first bg check, after
    // the action) is the target's; from then on the floor's.
    update(&mut w, h);
    let home_y = bl(&w, h).actor.home_pos.y;
    let floor = bl(&w, h).actor.floor_height;
    // Link 300 below it (lower than the floor): the floor's height + 40, 1 a frame.
    set_link(&mut w, h, 1000.0, -300.0, 0);
    update(&mut w, h);
    let b = bl(&w, h);
    assert_eq!((b.action, b.timer), (Action::Idle, 30));
    assert!(floor + 40.0 < home_y - 1.0, "{floor} {home_y}");
    assert_eq!(b.actor.home_pos.y, home_y - 1.0);
    // EnBili_UpdateMovement: sinf(timer × 0.19634955) × 3 over home (no vertical speed after).
    assert_eq!(b.actor.world_pos.y, ((30.0f32 * 0.19634955).sin() * 3.0) + b.actor.home_pos.y);
    // Within 160 and 45 up or down: EnBili_SetupApproachPlayer (1.2).
    set_link(&mut w, h, 150.0, 40.0, 0x2000);
    update(&mut w, h);
    assert_eq!((bl(&w, h).action, bl(&w, h).actor.speed_xz), (Action::ApproachPlayer, 1.2));
    // EnBili_ApproachPlayer: turning to him by halves, 0x71C at most.
    let yaw = bl(&w, h).actor.world_rot.y;
    update(&mut w, h);
    let want = {
        let mut y = yaw;
        eng_math::approach_s(&mut y, 0x2000, 2, 0x71C);
        y
    };
    assert_eq!(bl(&w, h).actor.world_rot.y, want);
    // Over 200 away: drifting again.
    set_link(&mut w, h, 201.0, 40.0, 0x2000);
    update(&mut w, h);
    assert_eq!(bl(&w, h).action, Action::Idle);
}

#[test]
fn touching_link_it_shocks_then_rises_away_and_flees() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = drifting(&mut w);
    let bolts = effects(&w, EFFECT_SS_LIGHTNING);
    bl_mut(&mut w, h).collider.base.at_flags |= AT_HIT;
    update(&mut w, h);
    // EnBili_SetupElectrify (its AT hit forgotten): gBiriElectrifyAnim, 10, still, -1 up; the
    // first frame: bolts where (10 + 2i) % 4 is 0 (i 1 and 3), the spark, 9, velocity.y 1.
    let b = bl(&w, h);
    assert_eq!((b.action, b.timer, b.actor.speed_xz, b.actor.velocity.y), (Action::Electrify, 9, 0.0, 1.0));
    assert_eq!(b.collider.base.at_flags & AT_HIT, 0);
    assert!(b.skel.is(ANIM_ELECTRIFY));
    assert_eq!(b.actor.sfx, NA_SE_EN_BIRI_SPARK - SFX_FLAG);
    assert_eq!(effects(&w, EFFECT_SS_LIGHTNING), bolts + 2);
    // Its core's combiner inverted on odd frames (EnBili_Draw).
    use oot_game::actor_ctx::ActorImpl;
    assert_eq!(b.render_state().switches[2], 1);
    // 9: none; 8: i 0 and 2.
    update(&mut w, h);
    assert_eq!(effects(&w, EFFECT_SS_LIGHTNING), bolts + 2);
    assert_eq!(bl(&w, h).render_state().switches[2], 0);
    update(&mut w, h);
    assert_eq!(effects(&w, EFFECT_SS_LIGHTNING), bolts + 4);
    // Shaking: velocity.y -1, 1 by turns.
    assert_eq!(bl(&w, h).actor.velocity.y, 1.0);
    // The count out and the animation at its start: EnBili_SetupAscendAway (not hurting).
    until_not(&mut w, h, Action::Electrify, 100);
    let b = bl(&w, h);
    assert_eq!((b.action, b.timer, b.collider.base.at_flags & AT_ON), (Action::AscendAway, 0, 0));
    assert!(b.skel.is(ANIM_ASCEND_AWAY));
    // EnBili_AscendAway: NA_SE_EN_BIRI_JUMP at frame 9, then toward 100 over Link; done,
    // EnBili_SetupFleePlayer (96, 0.9, hurting).
    set_link(&mut w, h, 100.0, 0.0, 0);
    let y0 = bl(&w, h).actor.world_pos.y;
    until_not(&mut w, h, Action::AscendAway, 100);
    let b = bl(&w, h);
    assert_eq!(sfx(&w, h, NA_SE_EN_BIRI_JUMP), 1);
    assert!(b.actor.world_pos.y > y0);
    assert_eq!((b.action, b.timer, b.actor.speed_xz, b.collider.base.at_flags & AT_ON), (Action::FleePlayer, 96, 0.9, AT_ON));
    assert_eq!(b.actor.home_pos.y, b.actor.world_pos.y);
    // EnBili_FleePlayer: away from Link, 0x38E a frame; 96 frames, then drifting.
    let n = until_not(&mut w, h, Action::FleePlayer, 200);
    assert_eq!(n, 96);
    assert_eq!(bl(&w, h).action, Action::Idle);
}

#[test]
fn a_sword_kills_it_with_a_shock_then_it_bursts_into_bubbles() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = drifting(&mut w);
    hit(&mut w, h, 1, EN_BILI_DMG_REACT_SWORD_SLASH, DMG_SLASH_KOKIRI);
    w.actors.freeze_flash_timer = 0;
    update(&mut w, h);
    // EnBili_CheckCollide: health 1 - 1: NA_SE_EN_BIRI_DEAD, the finishing blow, lock-on off;
    // the sword: red 10 frames, EN_BILI_TYPE_DIE_AFTER_ELECTRIFY, the shock.
    let b = bl(&w, h);
    assert_eq!((b.action, b.actor.col_chk_info.health, b.actor.params), (Action::Electrify, 0, EN_BILI_TYPE_DIE_AFTER_ELECTRIFY));
    assert_eq!(b.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(b.actor.color_filter_params & COLORFILTER_COLORFLAG_RED, COLORFILTER_COLORFLAG_RED);
    assert_eq!(sfx(&w, h, NA_SE_EN_BIRI_DEAD), 1);
    assert_eq!(w.actors.freeze_flash_timer, 5);
    // No more hits (health 0): EnBili_CheckCollide passes it over.
    hit(&mut w, h, 1, EN_BILI_DMG_REACT_SWORD_SLASH, DMG_SLASH_KOKIRI);
    update(&mut w, h);
    assert_eq!((bl(&w, h).action, bl(&w, h).timer), (Action::Electrify, 8));
    assert_eq!(sfx(&w, h, NA_SE_EN_BIRI_DEAD), 1);
    // The shock done: EnBili_SetupDie (18), not rising away.
    until_not(&mut w, h, Action::Electrify, 100);
    let b = bl(&w, h);
    assert_eq!((b.action, b.timer, b.actor.speed_xz), (Action::Die, 18, 0.0));
    // EnBili_Die: gone from view (the drop), then two bubbles a frame for 17 frames,
    // NA_SE_EN_BIRI_BUBLE at 14; at 0 gone.
    let bubbles = effects(&w, EFFECT_SS_DT_BUBBLE);
    let bubles = sfx_any(&w, NA_SE_EN_BIRI_BUBLE);
    for k in 1..=17 {
        update(&mut w, h);
        let b = bl(&w, h);
        assert!(!b.drawn && !gone(&w, h), "frame {k}");
        assert_eq!(b.timer, 18 - k);
        assert_eq!(effects(&w, EFFECT_SS_DT_BUBBLE), bubbles + 2 * k as usize);
        assert_eq!(sfx_any(&w, NA_SE_EN_BIRI_BUBLE), bubles + (k >= 4) as usize, "frame {k}");
    }
    update(&mut w, h);
    assert!(gone(&w, h));
}

#[test]
fn a_deku_nut_stuns_it_and_a_sword_then_damages_it() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = drifting(&mut w);
    hit(&mut w, h, 0, EN_BILI_DMG_REACT_STUN, DMG_DEKU_NUT);
    update(&mut w, h);
    // EnBili_SetupStunned: 80 (79 after its first frame), no backlash, falling, still, blue 80,
    // NA_SE_EN_GOMA_JR_FREEZE, not hurting; health kept (no damage).
    let b = bl(&w, h);
    assert_eq!((b.action, b.timer, b.actor.gravity, b.actor.speed_xz, b.actor.col_chk_info.health), (Action::Stunned, 79, -1.0, 0.0, 1));
    assert_eq!(b.collider.info.ac_dmg_info.hit_backlash, HIT_BACKLASH_NONE);
    assert_eq!(b.collider.base.at_flags & AT_ON, 0);
    assert_eq!((b.actor.color_filter_params & COLORFILTER_COLORFLAG_RED, b.actor.color_filter_timer), (0, 80));
    assert_eq!(sfx(&w, h, NA_SE_EN_GOMA_JR_FREEZE), 1);
    // Squashed by the count (EnBili_GetHoodLimbScale, _GetCoreLimbScale).
    let f = ((79.0f32) * (std::f32::consts::PI / 10.0)).sin() * 0.08;
    assert_eq!(bl(&w, h).limb_scales()[0], Vec3::new(1.0 - f, 1.0 + f, 1.0 - f));
    // A second nut doesn't start it again.
    hit(&mut w, h, 0, EN_BILI_DMG_REACT_STUN, DMG_DEKU_NUT);
    update(&mut w, h);
    assert_eq!(bl(&w, h).timer, 78);
    // A sword while it's stunned: EnBili_SetupDamaged (20 then 19; neither hurting nor hittable),
    // the last health gone: dying after.
    hit(&mut w, h, 1, EN_BILI_DMG_REACT_SWORD_SLASH, DMG_SLASH_KOKIRI);
    update(&mut w, h);
    let b = bl(&w, h);
    assert_eq!((b.action, b.timer, b.actor.col_chk_info.health), (Action::Damaged, 19, 0));
    assert_eq!((b.collider.base.at_flags & AT_ON, b.collider.base.ac_flags & AC_ON), (0, 0));
    assert_ne!(b.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    assert_eq!(until_not(&mut w, h, Action::Damaged, 100), 19);
    assert_eq!(bl(&w, h).action, Action::Die);
}

#[test]
fn stunned_80_frames_it_drifts_again_with_its_backlash() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = drifting(&mut w);
    hit(&mut w, h, 0, EN_BILI_DMG_REACT_STUN, DMG_DEKU_NUT);
    update(&mut w, h);
    let lands = sfx(&w, h, oot_game::audio::sfx::NA_SE_EN_DODO_M_GND);
    assert_eq!(until_not(&mut w, h, Action::Stunned, 100), 79);
    let b = bl(&w, h);
    assert_eq!((b.action, b.collider.info.ac_dmg_info.hit_backlash, b.actor.gravity), (Action::Idle, HIT_BACKLASH_ELECTRIC, 0.0));
    // It fell 150 at gravity -1 and landed once (NA_SE_EN_DODO_M_GND on BGCHECKFLAG_GROUND_TOUCH).
    assert_eq!(sfx(&w, h, oot_game::audio::sfx::NA_SE_EN_DODO_M_GND), lands + 1);
}

#[test]
fn fire_burns_it_to_death_in_two_frames() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = drifting(&mut w);
    hit(&mut w, h, 4, EN_BILI_DMG_REACT_FIRE, DMG_ARROW_FIRE);
    update(&mut w, h);
    // EnBili_SetupDamaged, then timer 2 (1 after its frame); an arrow: updating everywhere.
    let b = bl(&w, h);
    assert_eq!((b.action, b.timer, b.actor.col_chk_info.health), (Action::Damaged, 1, 0));
    assert_eq!(b.actor.color_filter_timer, 20);
    update(&mut w, h);
    assert_eq!(bl(&w, h).action, Action::Die);
}

#[test]
fn an_arrow_stuck_in_it_holds_its_death() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = drifting(&mut w);
    hit(&mut w, h, 2, EN_BILI_DMG_REACT_NONE, DMG_ARROW_NORMAL);
    bl_mut(&mut w, h).actor.flags |= ACTOR_FLAG_ATTACHED_TO_ARROW;
    for _ in 0..30 {
        update(&mut w, h);
    }
    // EnBili_Damaged: red (20) while the arrow is in it, the count held.
    let b = bl(&w, h);
    assert_eq!((b.action, b.timer, b.actor.color_filter_timer), (Action::Damaged, 20, 20));
    // Dying with it in: EnBili_Die waits, drawn.
    bl_mut(&mut w, h).setup_die();
    update(&mut w, h);
    assert_eq!((bl(&w, h).timer, bl(&w, h).drawn), (18, true));
    bl_mut(&mut w, h).actor.flags &= !ACTOR_FLAG_ATTACHED_TO_ARROW;
    update(&mut w, h);
    assert_eq!((bl(&w, h).timer, bl(&w, h).drawn), (17, false));
}

#[test]
fn ice_freezes_it_until_it_lands_then_it_dies() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = drifting(&mut w);
    let ice = effects(&w, EFFECT_SS_EN_ICE);
    hit(&mut w, h, 4, EN_BILI_DMG_REACT_ICE, DMG_ARROW_ICE);
    update(&mut w, h);
    // EnBili_SetupFrozen: eight chunks of ice, falling, still, neither hurting nor hittable, 300.
    let b = bl(&w, h);
    assert_eq!((b.action, b.timer, b.actor.gravity, b.actor.speed_xz), (Action::Frozen, 299, -1.0, 0.0));
    assert_eq!((b.collider.base.at_flags & AT_ON, b.collider.base.ac_flags & AC_ON), (0, 0));
    assert_eq!(effects(&w, EFFECT_SS_EN_ICE), ice + 8);
    assert_eq!(b.actor.color_filter_timer, 10);
    // Falling 150 at gravity -1 (BGCHECKFLAG_GROUND from the frame after it lands): dying, the
    // tint off.
    let n = until_not(&mut w, h, Action::Frozen, 300);
    assert!(n < 40, "{n}");
    let b = bl(&w, h);
    assert_eq!((b.action, b.actor.color_filter_timer), (Action::Die, 0));
}

#[test]
fn a_slingshot_seed_knocks_it_back_from_where_the_seed_was() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = drifting(&mut w);
    let p = bl(&w, h).actor.world_pos;
    // The seed (the attacker) was 100 along -x from it last frame, level.
    w.player_mut().actor.prev_pos = p - Vec3::X * 100.0;
    hit(&mut w, h, 0, EN_BILI_DMG_REACT_SLINGSHOT, DMG_SLINGSHOT);
    update(&mut w, h);
    // EnBili_SetupSlingshotKnockback: away from it (yaw toward it + 0x8000: +x, 0x4000), level,
    // 5; slowing 0.3 a frame (4.7 after its first), moving along x and y (Actor_MoveXYZ).
    let b = bl(&w, h);
    assert_eq!((b.action, b.actor.world_rot.y, b.actor.world_rot.x, b.actor.col_chk_info.health), (Action::SlingshotKnockback, 0x4000, 0, 1));
    assert!((b.actor.speed_xz - 4.7).abs() < 1e-6);
    assert!(b.actor.world_pos.x > p.x);
    // Stopped (17 frames in all), turned round and drifting.
    until_not(&mut w, h, Action::SlingshotKnockback, 30);
    let b = bl(&w, h);
    assert_eq!((b.action, b.actor.world_rot.y), (Action::Idle, -0x4000));
}

#[test]
fn its_tentacles_picture_and_limb_scales_follow_the_animation() {
    let Some(a) = assets() else { return };
    let mut w = room_1(&a);
    let h = drifting(&mut w);
    let b = bl_mut(&mut w, h);
    // EnBili_UpdateTentaclesTexIndex: drifting frame >> 1; shocking (|3 - frame| + 5) % 8;
    // rising away by its four spans.
    for (action, frame, want) in [
        (Action::Idle, 10.0, 5),
        (Action::Idle, 15.5, 7),
        (Action::Electrify, 0.0, 0),
        (Action::Electrify, 3.0, 5),
        (Action::Electrify, 5.0, 7),
        (Action::AscendAway, 6.0, 3),
        (Action::AscendAway, 9.0, 3),
        (Action::AscendAway, 12.0, 2),
        (Action::AscendAway, 18.0, 0),
        (Action::AscendAway, 19.0, 7),
        (Action::AscendAway, 36.0, 2),
        (Action::AscendAway, 38.0, 1),
    ] {
        b.action = action;
        b.skel.cur_frame = frame;
        b.update_tentacles_tex_index();
        assert_eq!(b.tentacles_tex_index, want, "{action:?} {frame}");
    }
    // EnBili_GetHoodLimbScale and the rest at a few frames.
    use std::f32::consts::PI;
    assert_eq!(hood_limb_scale(Action::Idle, 0.0, 0), Vec3::new(1.0, (0.0f32.cos() * 0.13) + 0.87, 1.0));
    assert_eq!(hood_limb_scale(Action::Electrify, 3.0, 0), Vec3::new(1.0, 1.0 - (((PI * 0.16667) * 3.0).sin() * 0.26), 1.0));
    assert_eq!(core_limb_scale(Action::Idle, 4.0, 0), Vec3::new(1.0, 1.1 - (((PI / 8.0) * 4.0).cos() * 0.1), 1.0));
    let f = ((12.0f32 - 8.0) * (PI / 10.0)).cos();
    assert_eq!(tentacles_limb_scale(Action::AscendAway, 12.0), Vec3::new((0.325 * f) + 0.925, 0.95 - (0.55 * f), (0.325 * f) + 0.925));
    assert_eq!(tentacles_limb_scale(Action::Idle, 12.0), Vec3::ONE);
    // The bakes: one per picture and combiner.
    assert_eq!(bakes().len(), 16);
    assert_eq!(bake_name(7, true), "En_Bili/tentacles7_invert");
}

#[test]
fn its_draw_is_the_bake_of_its_picture_and_combiner() {
    let Some(a) = assets() else { return };
    // Each bake whole: the tentacles' picture and the combiner bound (nothing unresolved).
    for b in bakes() {
        let d: eng_gfx::DrawList = a.pack.assets.get(&oot_game::pack::keys::bake(&b.name)).expect(&b.name);
        assert!(d.triangle_count() > 0 && d.stats.unresolved_addresses.is_empty(), "{}", b.name);
    }
    let mut w = room_1(&a);
    let h = drifting(&mut w);
    bl_mut(&mut w, h).collider.base.at_flags |= AT_HIT;
    update(&mut w, h);
    // Shocking on an odd count: the inverted combiner, translucent, the skeleton's pose.
    use oot_game::actor_ctx::ActorImpl;
    let b = bl(&w, h);
    let view = oot_game::play::ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
    let mut out = oot_game::play::DrawOut::default();
    b.draw(&b.render_state(), &w, &view, &mut out);
    assert_eq!((out.opa.len(), out.xlu.len()), (0, 1));
    assert_eq!(out.xlu[0].mesh, eng_gfx::MeshKey::named(oot_game::pack::keys::bake(&bake_name(b.tentacles_tex_index as usize, true))));
    assert_eq!(out.xlu[0].bones.len(), 4);
}
