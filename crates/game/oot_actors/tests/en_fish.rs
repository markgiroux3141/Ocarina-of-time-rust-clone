//! GAME-10 milestone 1 against the C: the fish (`z_en_fish.c`). The generic grotto (room 0 of
//! `kakusiana`, `ENTR_GROTTOS_0`: Link at (35, 0, 85) on dry ground at y 0) places the tiny pond's
//! fish (params 1) at (95, -60, -314), in a pond whose water is at y -56 over a floor at -75; the
//! Tektite grotto (room 10, `ENTR_GROTTOS_11`) two `Obj_Mure` groups of five (0x5322).
//!
//! Most tests run the fish's update alone (`Actor_UpdateAll`'s call, `update`), Link's distance
//! and yaw set as `Actor_UpdateAll` would, so every `Rand` call is the fish's: the expected values
//! replay them on a copy of the generator. The catch runs whole frames. Expected values are worked
//! out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use eng_math::{cos_s, sin_s, step_to_angle_s, vec3f_yaw};
use glam::{IVec3, Vec3};
use oot_actors::PlayExt;
use oot_actors::en_fish::*;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, BGCHECKFLAG_GROUND, BGCHECKFLAG_WATER, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::SfxPos;
use oot_game::collision_check::{OC1_ON, OC1_TYPE_ALL, OC2_TYPE_1};
use oot_game::cutscene::{CS_STATE_RUN, CsCmdActorCue};
use oot_game::play::{PlayState, Rand, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::shadow::ShadowDraw;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link entered at `entrance` (`deku-tree-dead`, the sound log on), two frames on.
fn enter(a: &Arc<GameAssets>, entrance: &str) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-dead").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    w
}

/// The generic grotto (room 0), Link at its spawn.
fn grotto(a: &Arc<GameAssets>) -> PlayState {
    let w = enter(a, "ENTR_GROTTOS_0");
    assert_eq!(w.room_ctx.cur.num, 0);
    w
}

/// Where Link stands in the generic grotto (its spawn 0): dry ground, the floor at y 0.
const LINK: Vec3 = Vec3::new(35.0, 0.0, 85.0);
/// The tiny pond's fish's place, and the pond's water and floor heights there.
const POND: Vec3 = Vec3::new(95.0, -60.0, -314.0);
const POND_WATER_Y: f32 = -56.0;

fn spawn(w: &mut PlayState, params: i16, pos: Vec3, rot: [i16; 3]) -> ActorHandle {
    w.actor_spawn(ACTOR_EN_FISH, pos, rot, params).expect("En_Fish spawned")
}

fn fish(w: &PlayState, h: ActorHandle) -> &EnFish {
    w.actors.downcast::<EnFish>(h).expect("an En_Fish")
}

fn fish_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnFish {
    w.actors.downcast_mut::<EnFish>(h).expect("an En_Fish")
}

fn all_fish(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnFish>(h).is_some_and(|f| !f.actor.killed)).collect()
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<EnFish>(h).is_none_or(|f| f.actor.killed)
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// `Actor_UpdateAll`'s distances to Link for a fish with Link `dist` away along `yaw`.
fn set_link_at(w: &mut PlayState, h: ActorHandle, dist: f32, yaw: i16) {
    let f = fish_mut(w, h);
    f.actor.xz_dist_to_player = dist;
    f.actor.y_dist_to_player = 0.0;
    f.actor.yaw_towards_player = yaw;
}

/// The `NA_SE_EV_FISH_LEAP`s asked for at `h`.
fn leaps(w: &PlayState, h: ActorHandle) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == NA_SE_EV_FISH_LEAP && p == SfxPos::Actor(h)).count()
}

/// `Math_SmoothStepToF(&v, target, fraction, step, 0)` from `v`.
fn eased(v: f32, target: f32, fraction: f32, step: f32) -> f32 {
    let mut v = v;
    eng_math::smooth_step_to_f(&mut v, target, fraction, step, 0.0);
    v
}

/// `Rand_S16Offset` on a copy.
fn s16_offset(r: &mut Rand, base: i16, range: i16) -> i16 {
    r.s16_offset(base, range)
}

#[test]
fn each_type_initialises_as_its_c_does() {
    let Some(a) = assets() else { return };
    let mut w = grotto(&a);
    // The grotto's own: params 1 (EN_FISH_TYPE_TINY_POND) at its home, swimming.
    let placed: Vec<_> = all_fish(&w);
    assert_eq!(placed.len(), 1);
    let f = fish(&w, placed[0]);
    assert_eq!((f.actor.params, f.actor.home_pos, f.action), (EN_FISH_TYPE_TINY_POND, POND, Action::TinyPondSwim));

    for params in [EN_FISH_TYPE_NORMAL, EN_FISH_TYPE_DROPPED, EN_FISH_TYPE_TINY_POND, 5] {
        // EnFish_Init's Rand calls: the two phases ((s16)(s32)(Rand_ZeroOne() * 65535.5f)), then
        // the normal's and the tiny pond's Rand_S16Offset(5, 35).
        let mut r = w.rand;
        let phase1 = (r.zero_one() * 65535.5) as i32 as i16;
        let phase2 = (r.zero_one() * 65535.5) as i32 as i16;
        let timer = if params == EN_FISH_TYPE_DROPPED { 300 } else { s16_offset(&mut r, 5, 35) };
        let h = spawn(&mut w, params, LINK + Vec3::new(0.0, 40.0, -100.0), [0x4000, 0, 0]);
        assert_eq!(w.rand, r, "params {params}: the init's Rand calls");
        let f = fish(&w, h);
        // sInitChain: 0.01 big; culling 900, 40, 700. Mass 50. gFishSkel: 6 limbs, 7 joints.
        assert_eq!(f.actor.scale, Vec3::splat(0.01));
        assert_eq!((f.actor.culling_volume_distance, f.actor.culling_volume_scale, f.actor.culling_volume_downward), (900.0, 40.0, 700.0));
        assert_eq!(f.actor.col_chk_info.mass, 50);
        assert_eq!(f.skel.joint_table.len(), 7);
        assert_eq!((f.phase1, f.phase2, f.timer, f.reappear_timer), (phase1, phase2, timer, 0));
        // sJntSphInit: one sphere of 5 at the root (scale 100), pushing everything as type 1.
        assert_eq!((f.collider.base.oc_flags1, f.collider.base.oc_flags2), (OC1_ON | OC1_TYPE_ALL, OC2_TYPE_1));
        assert_eq!(f.collider.elements.len(), 1);
        assert_eq!((f.collider.elements[0].dim.limb, f.collider.elements[0].dim.model_sphere.radius), (0, 5));
        assert!(f.drawn);
        if params == EN_FISH_TYPE_DROPPED {
            // Always updating, ActorShape_Init(.., ActorShadow_DrawCircle, 8), falling:
            // gravity -1 to -10, gFishInAirAnim, walls and floor checked, 300 frames.
            assert_ne!(f.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
            assert_eq!((f.actor.shape_shadow_draw, f.actor.shape_shadow_scale), (ShadowDraw::Circle, 8.0));
            assert_eq!((f.action, f.actor.gravity, f.actor.min_velocity_y), (Action::DroppedFall, -1.0, -10.0));
            assert_eq!(f.upd_bg_check_info_flags, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
            assert!(f.skel.is(ANIM_IN_AIR));
        } else {
            // No ActorShape_Init: no shadow; no gravity, no bg check, gFishInWaterAnim.
            assert_eq!(f.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
            assert_eq!(f.actor.shape_shadow_draw, ShadowDraw::None);
            assert_eq!((f.actor.gravity, f.actor.min_velocity_y, f.upd_bg_check_info_flags), (0.0, 0.0, 0));
            assert!(f.skel.is(ANIM_IN_WATER));
            // Any value but 0 and 1 is the normal fish.
            let expected = if params == EN_FISH_TYPE_TINY_POND { Action::TinyPondSwim } else { Action::NormalStay };
            assert_eq!(f.action, expected);
        }
    }
}

#[test]
fn obj_mures_fish_come_alive_in_the_tektite_grotto() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_GROTTOS_11");
    assert_eq!(w.room_ctx.cur.num, 10);
    idle(&mut w, 10);
    // Two Obj_Mure 0x5322: type 2 (fish), svNum 1 (all respawned), chNum 5: five En_Fish each,
    // EN_FISH_TYPE_NORMAL, at the group's place (ObjMure_GetSpawnPos), in the group's room.
    let fish_now = all_fish(&w);
    assert_eq!(fish_now.len(), 10);
    for home in [Vec3::new(3006.0, -137.0, 787.0), Vec3::new(3010.0, -207.0, 831.0)] {
        let group: Vec<_> = fish_now.iter().filter(|&&h| fish(&w, h).actor.home_pos == home).collect();
        assert_eq!(group.len(), 5);
        for &&h in &group {
            let f = fish(&w, h);
            assert_eq!((f.actor.params, f.actor.room), (EN_FISH_TYPE_NORMAL, 10));
            assert!(matches!(f.action, Action::NormalStay | Action::NormalSwim), "{:?}", f.action);
        }
    }
}

#[test]
fn the_normal_fish_stays_then_swims_home_or_after_another() {
    let Some(a) = assets() else { return };
    let mut w = grotto(&a);
    let home = LINK + Vec3::new(0.0, 20.0, -300.0);
    let h = spawn(&mut w, EN_FISH_TYPE_NORMAL, home, [0; 3]);
    {
        let f = fish_mut(&mut w, h);
        f.timer = 2;
        f.actor.speed_xz = 1.0;
        (f.phase1, f.phase2, f.actor.shape_y_offset) = (0, 0, 0.0);
    }
    set_link_at(&mut w, h, 300.0, 0);
    update(&mut w, h);
    let f = fish(&w, h);
    // EnFish_UpdateImpl: the timer 1, the phases 0x111 and 0x500; EnFish_Normal_Stay: the bob
    // (sin(0x111) × 10 + sin(0x500) × 5), the speed 1 eased by 0.05 (0.95), the animation at
    // min(0.95 × 1.4 + 0.8, 2) = 2; still staying.
    assert_eq!((f.timer, f.phase1, f.phase2, f.action), (1, 0x111, 0x500, Action::NormalStay));
    assert_eq!(f.actor.shape_y_offset, sin_s(0x111) * 10.0 + sin_s(0x500) * 5.0);
    assert_eq!((f.actor.speed_xz, f.skel.play_speed), (eased(1.0, 0.0, 0.05, 0.3), 2.0));
    // Its time up: EnFish_Normal_SetupSwim, Rand_S16Offset(15, 45) frames.
    let mut r = w.rand;
    let t = s16_offset(&mut r, 15, 45);
    update(&mut w, h);
    assert_eq!((fish(&w, h).action, fish(&w, h).timer), (Action::NormalSwim, t));

    // Swimming 100 east of home: turned home (yaw -0x4000) 0xBB8 a frame, at 0 + 1.8 × 0.08.
    {
        let f = fish_mut(&mut w, h);
        f.actor.world_pos = home + Vec3::new(100.0, 0.0, 0.0);
        f.actor.world_rot.y = 0;
        f.actor.speed_xz = 0.0;
        f.timer = 20;
    }
    assert_eq!(vec3f_yaw(home + Vec3::new(100.0, 0.0, 0.0), home), -0x4000);
    update(&mut w, h);
    let f = fish(&w, h);
    assert_eq!((f.actor.world_rot.y, f.actor.shape_rot.y, f.timer), (-0xBB8, -0xBB8, 19));
    assert_eq!(f.actor.speed_xz, 1.8 * 0.08);
    assert_eq!(f.skel.play_speed, 1.8 * 0.08 * 1.5 + 0.8);

    // Near home, after another fish (its `child`): towards it, 0xBB8 a frame.
    let other = spawn(&mut w, EN_FISH_TYPE_NORMAL, home + Vec3::new(-30.0, 0.0, 40.0), [0; 3]);
    let other_pos = fish(&w, other).actor.world_pos;
    {
        let f = fish_mut(&mut w, h);
        f.actor.world_pos = home + Vec3::new(10.0, 0.0, 0.0);
        f.actor.world_rot.y = 0;
        f.actor.child = Some(other);
    }
    let mut expected = 0i16;
    step_to_angle_s(&mut expected, vec3f_yaw(home + Vec3::new(10.0, 0.0, 0.0), other_pos), 0xBB8);
    update(&mut w, h);
    assert_eq!(fish(&w, h).actor.world_rot.y, expected);
    // The fish it follows gone: forgotten (EnFish_UpdateImpl).
    w.actors.actor_mut(other).unwrap().kill();
    update(&mut w, h);
    assert_eq!(fish(&w, h).actor.child, None);
}

#[test]
fn the_normal_fish_flees_link_and_follows_him_when_its_group_says() {
    let Some(a) = assets() else { return };
    let mut w = grotto(&a);
    let home = LINK + Vec3::new(0.0, 20.0, -300.0);
    let h = spawn(&mut w, EN_FISH_TYPE_NORMAL, home, [0; 3]);
    fish_mut(&mut w, h).timer = 20;
    // Link within 60 (40 east): EnFish_Normal_SetupFleePlayer, Rand_S16Offset(10, 40) frames.
    set_link_at(&mut w, h, 40.0, 0x4000);
    let mut r = w.rand;
    let t = s16_offset(&mut r, 10, 40);
    update(&mut w, h);
    assert_eq!((fish(&w, h).action, fish(&w, h).timer), (Action::NormalFleePlayer, t));
    // Fleeing: at 4.2 (0 + 4.2 × 0.08), away from him (0xC000), swerving 0x2000 by
    // state.frames' bits 4 and 5, 0x7D0 a frame.
    for frames in [0x00u32, 0x10, 0x20, 0x30] {
        w.state_frames = frames;
        {
            let f = fish_mut(&mut w, h);
            f.actor.world_pos = home;
            f.actor.world_rot.y = 0x4000;
            f.actor.speed_xz = 0.0;
            f.timer = 20;
        }
        set_link_at(&mut w, h, 40.0, 0x4000);
        let target = match frames {
            0x30 => -0x4000 + 0x2000,
            0x20 => -0x4000 - 0x2000,
            _ => -0x4000,
        };
        let mut expected = 0x4000i16;
        step_to_angle_s(&mut expected, target, 0x7D0);
        update(&mut w, h);
        let f = fish(&w, h);
        assert_eq!((f.actor.world_rot.y, f.actor.speed_xz, f.action), (expected, 4.2 * 0.08, Action::NormalFleePlayer), "frames {frames:#x}");
    }
    // Link gone: staying again (Rand_S16Offset(5, 35)).
    set_link_at(&mut w, h, 100.0, 0x4000);
    let mut r = w.rand;
    let t = s16_offset(&mut r, 5, 35);
    update(&mut w, h);
    assert_eq!((fish(&w, h).action, fish(&w, h).timer), (Action::NormalStay, t));

    // Obj_Mure makes it its own child (ObjMure_SetChildToFollowPlayer): following Link,
    // Rand_S16Offset(10, 40) frames.
    fish_mut(&mut w, h).actor.child = Some(h);
    set_link_at(&mut w, h, 100.0, 0x4000);
    let mut r = w.rand;
    let t = s16_offset(&mut r, 10, 40);
    update(&mut w, h);
    assert_eq!((fish(&w, h).action, fish(&w, h).timer), (Action::NormalFollowPlayer, t));
    // Towards 20 beside Link, 0x9000 off his yaw with state.frames' bit 6, else 0x7000.
    let link = w.player().actor.world_pos;
    for (frames, off) in [(0x40u32, 0x9000u16 as i16), (0x00, 0x7000)] {
        w.state_frames = frames;
        {
            let f = fish_mut(&mut w, h);
            f.actor.world_pos = home;
            f.actor.world_rot.y = 0;
            f.timer = 20;
        }
        let yaw_to_link = vec3f_yaw(home, link);
        set_link_at(&mut w, h, 300.0, yaw_to_link);
        let angle = yaw_to_link.wrapping_add(off);
        let beside = Vec3::new(sin_s(angle) * 20.0 + link.x, link.y, cos_s(angle) * 20.0 + link.z);
        let mut expected = 0i16;
        step_to_angle_s(&mut expected, vec3f_yaw(home, beside), 0xBB8);
        update(&mut w, h);
        assert_eq!(fish(&w, h).actor.world_rot.y, expected, "frames {frames:#x}");
    }
}

#[test]
fn the_fish_offers_itself_to_the_bottle_and_goes_when_taken() {
    let Some(a) = assets() else { return };
    let mut w = grotto(&a);
    w.place_player(LINK, 0);
    idle(&mut w, 1);
    // 20 in front of Link, 5 up: within 32, his place moved 16 its way 4 from it (within 20),
    // within 80 across and 20 up: Actor_OfferGetItem(GI_MAX) makes it his interactRangeActor.
    let h = spawn(&mut w, EN_FISH_TYPE_NORMAL, LINK + Vec3::new(0.0, 5.0, 20.0), [0; 3]);
    // (Its first frame it isn't updated: not yet found inside its culling volume, which the draw
    // does.)
    idle(&mut w, 2);
    assert_eq!(w.player().interact_range_actor, Some(h));
    assert_eq!(w.player().get_item_id, oot_game::item::GI_MAX);
    // Player's bottle swing sets its parent: the next update clears it and it disappears
    // (EnFish_Disappear: 400 frames away, 0.001 big, no draw).
    let player = w.player;
    fish_mut(&mut w, h).actor.parent = player;
    idle(&mut w, 1);
    let f = fish(&w, h);
    assert_eq!((f.actor.parent, f.reappear_timer, f.actor.scale, f.drawn), (None, 400, Vec3::splat(0.001), false));
    // EnFish_UpdateWaitReappear: drawn again with 20 left, then 0.001 a frame back to 0.01 (at
    // most), its full size at 0; the timers held meanwhile.
    idle(&mut w, 379);
    assert_eq!((fish(&w, h).reappear_timer, fish(&w, h).drawn), (21, false));
    idle(&mut w, 1);
    assert_eq!((fish(&w, h).reappear_timer, fish(&w, h).drawn, fish(&w, h).actor.scale.x), (20, true, 0.001));
    idle(&mut w, 1);
    assert_eq!((fish(&w, h).reappear_timer, fish(&w, h).actor.scale.x), (19, 0.002));
    idle(&mut w, 19);
    assert_eq!((fish(&w, h).reappear_timer, fish(&w, h).actor.scale.x), (0, 0.01));
    assert!(!gone(&w, h));

    // The dropped fish taken: gone.
    let d = spawn(&mut w, EN_FISH_TYPE_DROPPED, LINK + Vec3::new(0.0, 5.0, 20.0), [0x4000, 0, 0]);
    fish_mut(&mut w, d).actor.parent = player;
    idle(&mut w, 1);
    assert!(gone(&w, d));

    // The tiny pond's taken: away, and the next frame gone for good.
    let p = all_fish(&w).into_iter().find(|&x| fish(&w, x).actor.params == EN_FISH_TYPE_TINY_POND).expect("the pond's fish");
    // (Its own updates: out of Link's view it isn't updated.)
    fish_mut(&mut w, p).actor.parent = player;
    set_link_at(&mut w, p, 300.0, 0);
    update(&mut w, p);
    assert_eq!(fish(&w, p).reappear_timer, 400);
    update(&mut w, p);
    assert!(gone(&w, p));
}

#[test]
fn the_tiny_ponds_fish_clamps_its_animation_the_wrong_way_round() {
    let Some(a) = assets() else { return };
    let mut w = grotto(&a);
    let h = all_fish(&w)[0];
    // At home, still (Link far, 10 frames left: sStaySpeedParams, 0 at 0.04 by at most 0.09).
    {
        let f = fish_mut(&mut w, h);
        f.actor.world_pos = POND;
        f.actor.speed_xz = 0.0;
        f.timer = 10;
    }
    set_link_at(&mut w, h, 300.0, 0);
    update(&mut w, h);
    // @bug (game): CLAMP(0 × 1.2 + 0.2, 1.5f, 0.5): under 1.5, so 1.5.
    assert_eq!((fish(&w, h).actor.speed_xz, fish(&w, h).skel.play_speed), (0.0, 1.5));
    // Fast (2 eased by 0.04 to 1.92): 1.92 × 1.2 + 0.2 over 1.5, so 0.5.
    fish_mut(&mut w, h).actor.speed_xz = 2.0;
    update(&mut w, h);
    assert_eq!((fish(&w, h).actor.speed_xz, fish(&w, h).skel.play_speed), (eased(2.0, 0.0, 0.04, 0.09), 0.5));
    // Link within 60 and 11 frames left (under 12): sMoveSpeedParams, to 0.5 at 0.1.
    {
        let f = fish_mut(&mut w, h);
        f.actor.speed_xz = 0.0;
        f.timer = 12;
    }
    set_link_at(&mut w, h, 40.0, 0);
    update(&mut w, h);
    assert_eq!(fish(&w, h).actor.speed_xz, eased(0.0, 0.5, 0.1, 0.15));
    // Its time up: another Rand_S16Offset(5, 80).
    fish_mut(&mut w, h).timer = 1;
    let mut r = w.rand;
    let t = s16_offset(&mut r, 5, 80);
    update(&mut w, h);
    assert_eq!(fish(&w, h).timer, t);
}

#[test]
fn the_dropped_fish_falls_then_flaps_on_the_ground() {
    let Some(a) = assets() else { return };
    let mut w = grotto(&a);
    // As Player_Action_8084EFC0 drops it: turned 0x4000 on x.
    let start = LINK + Vec3::new(0.0, 40.0, -60.0);
    let h = spawn(&mut w, EN_FISH_TYPE_DROPPED, start, [0x4000, 0, 0]);
    set_link_at(&mut w, h, 300.0, 0);
    // EnFish_Dropped_Fall: gravity 1 a frame, x held at 0x4000 (Math_StepToAngleS lands there),
    // z towards -0x4000 0x64 a frame; the timer from 300.
    for n in 1..=5 {
        update(&mut w, h);
        let f = fish(&w, h);
        assert_eq!(f.actor.velocity.y, -(n as f32), "frame {n}");
        // Actor_UpdatePos: 1.5 times the velocity a frame (R_UPDATE_RATE × 0.5).
        assert_eq!(f.actor.world_pos.y, start.y - 1.5 * (n * (n + 1) / 2) as f32, "frame {n}");
        assert_eq!((f.actor.world_rot.x, f.actor.world_rot.z, f.actor.shape_rot.z), (0x4000, -0x64 * n as i16, -0x64 * n as i16));
        assert_eq!((f.timer, f.action), (300 - n as i16, Action::DroppedFall));
    }
    // Down: the frame after the floor is found, flapping, 400 frames. Its setup's Rand calls: a
    // hop one time in ten 2.5 to 5.5, one in ten 0.2 to 1.4 (both NA_SE_EV_FISH_LEAP), else none
    // (and the sound one time in five).
    let mut flaps = 0;
    let mut hops = 0;
    for _ in 0..200 {
        let f = fish(&w, h);
        let (on_ground, in_water, action, timer) = (f.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0, f.actor.bg_check_flags & BGCHECKFLAG_WATER != 0, f.action, f.timer);
        assert!(!in_water);
        let mut r = w.rand;
        if action == Action::DroppedFlapOnGround {
            // EnFish_Dropped_FlapOnGround: its drift's and its turn's Rand_ZeroOne.
            r.zero_one();
            r.zero_one();
        }
        let sets_up = on_ground && (action == Action::DroppedFall || timer - 1 > 60);
        let (mut vy, mut leap) = (None, false);
        if sets_up {
            let rng = r.zero_one();
            if rng < 0.1 {
                vy = Some(r.zero_one() * 3.0 + 2.5);
                leap = true;
            } else if rng < 0.2 {
                vy = Some(r.zero_one() * 1.2 + 0.2);
                leap = true;
            } else {
                vy = Some(0.0);
                leap = r.zero_one() < 0.2;
            }
        }
        let before = leaps(&w, h);
        update(&mut w, h);
        assert_eq!(w.rand, r);
        let f = fish(&w, h);
        if sets_up {
            flaps += 1;
            if vy.unwrap() > 0.0 {
                hops += 1;
            }
            assert_eq!(f.action, Action::DroppedFlapOnGround);
            assert_eq!((f.actor.shape_y_offset, f.actor.gravity), (300.0, -1.0));
            // The setup's velocity, then Actor_MoveXZGravity's gravity.
            if vy.unwrap() > 1.0 {
                assert_eq!(f.actor.velocity.y, vy.unwrap() - 1.0);
            }
            if action == Action::DroppedFall {
                assert_eq!(f.timer, 400);
            }
            assert_eq!(leaps(&w, h) - before, leap as usize);
        }
    }
    assert!(flaps > 100 && hops > 0, "{flaps} flaps, {hops} hops");
    // The twist: state.frames 0x44 (bits 2 and 6): (s16)(0x1800 × 0.3) = 1843, negated by bit 2;
    // x towards it 0xFA0 a frame, z towards 0x4000 0x3E8.
    w.state_frames = 0x44;
    let (mut x, mut z) = (fish(&w, h).actor.world_rot.x, fish(&w, h).actor.world_rot.z);
    step_to_angle_s(&mut x, -1843, 0xFA0);
    step_to_angle_s(&mut z, 0x4000, 0x3E8);
    update(&mut w, h);
    assert_eq!((fish(&w, h).actor.world_rot.x, fish(&w, h).actor.world_rot.z), (x, z));
    assert_eq!(fish(&w, h).actor.shape_rot, fish(&w, h).actor.world_rot);
    // Its last 60 frames blinking with state.frames' bit 2, then gone.
    fish_mut(&mut w, h).timer = 61;
    w.state_frames = 4;
    update(&mut w, h);
    assert!(fish(&w, h).drawn);
    w.state_frames = 8;
    update(&mut w, h);
    assert!(!fish(&w, h).drawn);
    // Not drawn: no shadow (Actor_Draw isn't called).
    assert!(oot_game::actor_ctx::ActorImpl::render_state(fish(&w, h)).shadow.is_none());
    fish_mut(&mut w, h).timer = 1;
    update(&mut w, h);
    assert!(gone(&w, h));
}

#[test]
fn the_dropped_fish_swims_off_in_water() {
    let Some(a) = assets() else { return };
    let mut w = grotto(&a);
    let start = Vec3::new(POND.x, POND_WATER_Y + 20.0, POND.z);
    let h = spawn(&mut w, EN_FISH_TYPE_DROPPED, start, [0x4000, 0, 0]);
    set_link_at(&mut w, h, 300.0, 0);
    let mut n = 0;
    while fish(&w, h).action == Action::DroppedFall {
        update(&mut w, h);
        n += 1;
        assert!(n < 20);
    }
    // Under the water (22.5 down after five frames: -58.5), found in it; the next frame
    // EnFish_Dropped_SetupSwim: home there, no gravity, always updating, 200 frames.
    assert_eq!(n, 6);
    let f = fish(&w, h);
    assert_eq!(f.action, Action::DroppedSwim);
    assert_eq!((f.timer, f.actor.gravity, f.actor.min_velocity_y, f.actor.shape_y_offset), (200, 0.0, 0.0, 0.0));
    assert_eq!(f.actor.home_pos, f.actor.world_pos);
    assert!(f.skel.is(ANIM_IN_WATER));
    let home = f.actor.home_pos;
    // EnFish_Dropped_Swim: sinking 2 a frame towards 10 under home (4 on the ground); its last
    // 100 frames shrinking (0.982 a frame); at 0 gone.
    update(&mut w, h);
    assert_eq!((fish(&w, h).timer, fish(&w, h).actor.world_pos.y), (199, home.y - 2.0));
    for _ in 0..100 {
        update(&mut w, h);
    }
    let f = fish(&w, h);
    assert_eq!((f.timer, f.actor.scale.x), (99, 0.01 * 0.982));
    for _ in 0..98 {
        update(&mut w, h);
    }
    assert_eq!(fish(&w, h).timer, 1);
    update(&mut w, h);
    assert!(gone(&w, h));
}

#[test]
fn a_dropped_fish_follows_the_jabu_cutscenes_cue() {
    let Some(a) = assets() else { return };
    let mut w = grotto(&a);
    let h = spawn(&mut w, EN_FISH_TYPE_DROPPED, LINK + Vec3::new(0.0, 40.0, -60.0), [0x4000, 0, 0]);
    let normal = spawn(&mut w, EN_FISH_TYPE_NORMAL, LINK + Vec3::new(0.0, 40.0, -200.0), [0; 3]);
    let (start, end) = (IVec3::new(0, 100, 0), IVec3::new(200, 300, -400));
    let cue = |action: u16| Some(CsCmdActorCue { action, start_frame: 0, end_frame: 20, rot: [0; 3], start_pos: start, end_pos: end, normal: IVec3::ZERO });
    w.cs_ctx.state = CS_STATE_RUN;
    w.cs_ctx.frames = 10;
    w.cs_ctx.npc_actions[1] = cue(1);
    // A normal fish doesn't join.
    update(&mut w, normal);
    assert_eq!(statics(&mut w).jabu_cutscene_fish, None);
    // EnFish_JabuCutscene_Enter: the cutscene's fish, on its side (0, -0x6410, 0x4000), 600 up,
    // always updating, the hop 10. Cue 1 (EnFish_JabuCutscene_CueFlapOnGround): the hop up 0
    // (still 10, over 1: its speed down 0.4); at the cue's half (frame 10 of 0 to 20) plus it.
    let (p1, p2) = (fish(&w, h).phase1, fish(&w, h).phase2);
    update(&mut w, h);
    assert_eq!(statics(&mut w).jabu_cutscene_fish, Some(h));
    let f = fish(&w, h);
    assert_eq!((f.actor.shape_rot.x, f.actor.shape_rot.y, f.actor.shape_rot.z, f.actor.shape_y_offset), (0, -0x6410, 0x4000, 600.0));
    assert_ne!(f.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    assert!(f.skel.is(ANIM_IN_AIR));
    assert_eq!(f.actor.world_pos, Vec3::new(100.0, 200.0 + 10.0, -200.0));
    let (q1, q2) = (p1.wrapping_add(0x111), p2.wrapping_add(0x500));
    assert_eq!((f.phase1, f.phase2), (q1, q2));
    assert_eq!(f.skel.play_speed, (sin_s(q1) + sin_s(q2)) * 0.5 + 2.0);
    assert_eq!(statics(&mut w).jabu_cutscene_offset_y_speed, -0.4);
    // The hop falling: 9.6, 8.8, 7.6, 6.0, 4.0, 1.6; then down (at most 1: 1), one time in ten a
    // new one (2 to 5, NA_SE_EV_FISH_LEAP).
    let mut y = 10.0f32;
    let mut speed = -0.4f32;
    for _ in 0..6 {
        y += speed;
        speed -= 0.4;
        update(&mut w, h);
        assert!((fish(&w, h).actor.world_pos.y - (200.0 + y)).abs() < 1e-4);
    }
    let offset = |w: &mut PlayState| {
        let s = statics(w);
        (s.jabu_cutscene_offset_y, s.jabu_cutscene_offset_y_speed)
    };
    for _ in 0..40 {
        let (y, speed) = offset(&mut w);
        let mut r = w.rand;
        let before = leaps(&w, h);
        update(&mut w, h);
        let (y_after, speed_after) = offset(&mut w);
        if y + speed <= 1.0 {
            assert_eq!(y_after, 1.0);
            let leap = r.zero_one() < 0.1;
            let new_speed = if leap { r.zero_one() * 3.0 + 2.0 } else { 0.0 };
            assert_eq!(speed_after, new_speed);
            assert_eq!(leaps(&w, h) - before, leap as usize);
        } else {
            assert_eq!((y_after, speed_after), (y + speed, speed - 0.4));
        }
        assert_eq!(w.rand, r);
    }
    // A second dropped fish isn't the cutscene's: it falls.
    let other = spawn(&mut w, EN_FISH_TYPE_DROPPED, LINK + Vec3::new(30.0, 40.0, -60.0), [0x4000, 0, 0]);
    update(&mut w, other);
    assert_eq!(fish(&w, other).action, Action::DroppedFall);
    // Cue 2 (EnFish_JabuCutscene_CueFly): turning (x -0x1F4, z +0x64), the hop eased 1 a frame to 0.
    w.cs_ctx.npc_actions[1] = cue(2);
    statics(&mut w).jabu_cutscene_offset_y = 1.5;
    update(&mut w, h);
    let f = fish(&w, h);
    assert_eq!((f.actor.shape_rot.x, f.actor.shape_rot.z), (-0x1F4, 0x4064));
    assert_eq!(statics(&mut w).jabu_cutscene_offset_y, 0.5);
    update(&mut w, h);
    assert_eq!(statics(&mut w).jabu_cutscene_offset_y, 0.0);
    // The floor under it (BgCheck_EntityRaycastDown4).
    let pos = fish(&w, h).actor.world_pos;
    assert_eq!(fish(&w, h).actor.floor_height, w.col.entity_raycast_down(pos).0);
    // Cue 3: gone, the statics cleared.
    w.cs_ctx.npc_actions[1] = cue(3);
    update(&mut w, h);
    assert!(gone(&w, h));
    let s = statics(&mut w);
    assert_eq!((s.jabu_cutscene_fish, s.jabu_cutscene_offset_y, s.jabu_cutscene_offset_y_speed), (None, 0.0, 0.0));

    // The cue gone first: the cutscene fish is gone too.
    let h2 = spawn(&mut w, EN_FISH_TYPE_DROPPED, LINK + Vec3::new(0.0, 40.0, -60.0), [0x4000, 0, 0]);
    w.cs_ctx.npc_actions[1] = cue(1);
    update(&mut w, h2);
    assert_eq!(statics(&mut w).jabu_cutscene_fish, Some(h2));
    w.cs_ctx.npc_actions[1] = None;
    update(&mut w, h2);
    assert!(gone(&w, h2));
    assert_eq!(statics(&mut w).jabu_cutscene_fish, None);
}
