//! GAME-10 milestone 5 against the C: child Ruto (`z_en_ru1.c`), every type, her states driven
//! directly (no routes).
//!
//! Child Link enters Jabu-Jabu at `ENTR_JABU_JABU_0` on a `jabu-jabu` save (the belly's intro
//! seen), the room changed to the one tested; Zora's Fountain and Zora's Domain likewise. Most
//! tests run Ruto's update alone (`Actor_UpdateAll`'s call); the cutscenes run the whole world, the
//! scripts' cues driving her. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_bdan_objects::{self as objects, BgBdanObjects};
use oot_actors::bg_bdan_switch::{self as switch, BgBdanSwitch};
use oot_actors::door_warp1::{self as warp, DoorWarp1};
use oot_actors::en_ru1::{self as ru1, EnRu1};
use oot_game::actor::{ACTOR_FLAG_TALK, BGCHECKFLAG_GROUND};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::SfxPos;
use oot_game::camera::{CAM_SET_DUNGEON0, CAM_SET_NORMAL0};
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// A frame with A pressed on even `f` (released on odd): a script's or a box's texts read.
fn tick_a(w: &mut PlayState, f: usize) {
    let a = with(PadState::default(), BTN_A);
    let (prev, cur) = if f % 2 == 0 { (PadState::default(), a) } else { (a, PadState::default()) };
    w.tick_with(scripted_input(prev, cur));
}

/// Child Link at `entrance` on save `preset` with the infTable flags `inf` set, the sound log on.
fn play_at(a: &Arc<GameAssets>, entrance: &str, preset: &str, inf: &[u16], event: &[u16]) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    for &f in inf {
        save.set_inf_table(f);
    }
    for &f in event {
        save.set_event_chk_inf(f);
    }
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init")
}

/// Jabu-Jabu (`jabu-jabu`), in room `room`, its actors in.
fn jabu(a: &Arc<GameAssets>, room: i8, inf: &[u16]) -> PlayState {
    let mut w = play_at(a, "ENTR_JABU_JABU_0", "jabu-jabu", inf, &[]);
    idle(&mut w, 2);
    if room != 0 {
        assert!(w.room_request(room));
        idle(&mut w, 1);
        w.room_change_done();
    }
    assert_eq!(w.room_ctx.cur.num, room);
    idle(&mut w, 20);
    w
}

/// The live Rutos.
fn rutos(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnRu1>(h).is_some_and(|r| !r.actor.killed)).collect()
}

fn ruto(w: &PlayState, params: i16) -> ActorHandle {
    *rutos(w).iter().find(|&&h| w.actors.actor(h).unwrap().params == params).unwrap_or_else(|| panic!("Ruto {params:#06x}"))
}

fn ru(w: &mut PlayState, h: ActorHandle) -> &mut EnRu1 {
    w.actors.downcast_mut::<EnRu1>(h).expect("En_Ru1")
}

fn alive(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<EnRu1>(h).is_some_and(|r| !r.actor.killed)
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// The times a sound was asked for at `pos`.
fn sfx(w: &PlayState, pos: SfxPos, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == pos).count()
}

/// `object_ru1` loaded, where the room doesn't load it.
fn load_ru1(a: &Arc<GameAssets>, w: &mut PlayState) {
    load_object(a, w, "object_ru1");
}

fn load_object(a: &Arc<GameAssets>, w: &mut PlayState, name: &str) {
    let id = a.scenes.object_id(name).expect(name);
    if w.object_ctx.get_index(id).is_none() {
        w.object_ctx.spawn(id);
    }
}

/// Ruto as the basement's (`0x0203`, met) spawned in the room, then her two spawn states run:
/// talkative (`ENRU1_ACTION_SPEAK_JABU_IDLE`).
fn spawn_basement_ruto(w: &mut PlayState, pos: Vec3) -> ActorHandle {
    let h = w.actor_spawn(ru1::ACTOR_EN_RU1, pos, [0; 3], 0x0203).expect("spawned");
    update(w, h);
    update(w, h);
    assert_eq!(ru(w, h).action, ru1::ENRU1_ACTION_SPEAK_JABU_IDLE);
    h
}

#[test]
fn the_holes_room_meeting_starts_in_range_sets_met_and_she_drops_down_a_hole() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 2, &[]);
    // Room 2's 0x0202: not met: EnRu1_InitInJabuJabuHolesRoom, frowning, hidden.
    let h = ruto(&w, 0x0202);
    let r = ru(&mut w, h);
    assert_eq!((r.action, r.draw_config, r.mouth), (ru1::ENRU1_ACTION_MEETING_RANGE_CHECK, ru1::ENRU1_DRAW_NOTHING, ru1::ENRU1_MOUTH_FROWNING));
    let home = r.actor.world_pos;
    assert_eq!(home, Vec3::new(216.0, -340.0, -3072.0));
    // Link 266 along z from her: out of range (playerPosZ - thisPosZ <= 265).
    w.place_player(Vec3::new(12.0, -340.0, home.z + 266.0), -0x8000);
    idle(&mut w, 1);
    w.place_player(Vec3::new(12.0, -340.0, home.z + 266.0), -0x8000);
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).action, ru1::ENRU1_ACTION_MEETING_RANGE_CHECK);
    // 265: gRutoFirstMeetingCs, Link stopped (Player's speedXZ 0).
    w.place_player(Vec3::new(12.0, -340.0, home.z + 265.0), -0x8000);
    w.player_mut().linear_velocity = 3.0;
    assert_ne!(w.actors.actor(w.player.unwrap()).unwrap().bg_check_flags & BGCHECKFLAG_GROUND, 0);
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).action, ru1::ENRU1_ACTION_MEETING_INIT_POSITION);
    assert_eq!(w.cs_ctx.segment.as_ref().map(|s| s.name.as_str()), Some("gRutoFirstMeetingCs"));
    assert_eq!(w.save.cutscene_trigger, 1);
    assert_eq!(w.player().linear_velocity, 0.0);
    // The script: cue 2 (frames 0 to 178) at (127, -340, -3041) facing 0xEC16; 3 she turns and
    // walks off; off the floor she falls; 4 her arms up; 5 met; the script over, gone.
    let mut seen = vec![ru1::ENRU1_ACTION_MEETING_INIT_POSITION];
    let mut first_facing = None;
    for f in 0..1000 {
        // The script's texts (0x404B, 0x401E) wait for A.
        tick_a(&mut w, f);
        if !alive(&w, h) {
            break;
        }
        let r = ru(&mut w, h);
        if *seen.last().unwrap() != r.action {
            seen.push(r.action);
            if r.action == ru1::ENRU1_ACTION_MEETING_FACING_LINK {
                first_facing = Some((f, r.actor.world_pos, r.actor.shape_rot.y, r.draw_config));
            }
            if r.action == ru1::ENRU1_ACTION_MEETING_WALKING_AWAY_ACCEL {
                // EnRu1_StartWalkingAway: turned round (+0x8000).
                assert_eq!(r.actor.world_rot.y, 0xEC16u16 as i16 ^ i16::MIN);
            }
            if r.action == ru1::ENRU1_ACTION_MEETING_FALLING_DOWN_HOLE {
                assert!(r.actor.world_pos.y > -360.0, "falls from the floor: {}", r.actor.world_pos);
            }
        }
    }
    let (_, pos, rot, draw) = first_facing.expect("facing Link");
    assert_eq!((pos, rot, draw), (Vec3::new(127.0, -340.0, -3041.0), 0xEC16u16 as i16, ru1::ENRU1_DRAW_OPA));
    assert_eq!(
        seen,
        [
            ru1::ENRU1_ACTION_MEETING_INIT_POSITION,
            ru1::ENRU1_ACTION_MEETING_FACING_LINK,
            ru1::ENRU1_ACTION_MEETING_TURNING_AROUND,
            ru1::ENRU1_ACTION_MEETING_WALKING_AWAY_ACCEL,
            ru1::ENRU1_ACTION_MEETING_WALKING_AWAY_CONSTANT,
            ru1::ENRU1_ACTION_MEETING_FALLING_DOWN_HOLE,
            ru1::ENRU1_ACTION_MEETING_END,
        ]
    );
    assert!(w.save.get_inf_table(ru1::INFTABLE_RUTO_MET_FIRST_TIME));
    // Cue 4: her cry (NA_SE_VO_RT_FALL) and the fall's sound, once.
    assert_eq!(sfx(&w, SfxPos::Actor(h), ru1::NA_SE_VO_RT_FALL), 1);
    assert_eq!(sfx(&w, SfxPos::Actor(h), ru1::NA_SE_EV_FALL_DOWN_DIRT), 1);
    assert!(!alive(&w, h), "gone once the script ends");
    assert_eq!(w.cs_ctx.state, CS_STATE_IDLE);
}

#[test]
fn the_basement_ruto_says_her_lines_by_the_flags_and_sits_to_be_carried() {
    let Some(a) = assets() else { return };
    // Not met: the basement's Ruto (room 3's 0x0203) is killed at her init.
    let w = jabu(&a, 3, &[]);
    assert!(rutos(&w).is_empty());
    let mut w = jabu(&a, 3, &[ru1::INFTABLE_RUTO_MET_FIRST_TIME]);
    let h = ruto(&w, 0x0203);
    // EnRu1_InitInJabuJabuBasement: following rooms; two frames on, talkative and drawn.
    let r = ru(&mut w, h);
    assert_eq!((r.actor.room, r.room_num1, r.room_num2, r.room_num3), (-1, 3, 3, 3));
    assert_eq!((r.action, r.draw_config), (ru1::ENRU1_ACTION_SPEAK_JABU_IDLE, ru1::ENRU1_DRAW_OPA));
    // EnRu1_TalkOfferAccepted: 0x404C; explained, 0x404D; let Link carry her, 0x404E.
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).actor.text_id, 0x404C);
    w.save.set_inf_table(ru1::INFTABLE_RUTO_EXPLAINED_WHY_IN_JABU);
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).actor.text_id, 0x404D);
    w.save.set_inf_table(ru1::INFTABLE_RUTO_LET_LINK_CARRY);
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).actor.text_id, 0x404E);
    // Talked to: turned to Link while talking.
    ru(&mut w, h).actor.flags |= ACTOR_FLAG_TALK;
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).action, ru1::ENRU1_ACTION_SPEAK_JABU_TALKING);
    // EnRu1_BecomingCarrier's sit (gRutoChildSitAnim, once): when it ends she's seated
    // (gRutoChildSittingAnim), INFTABLE_RUTO_LET_LINK_CARRY, carriable, her sitting OC off.
    let sit = a.animation("object_ru1", "gRutoChildSitAnim").unwrap();
    let r = ru(&mut w, h);
    r.action = ru1::ENRU1_ACTION_SPEAK_JABU_SIT_DOWN;
    let end = sit.last_frame();
    r.skel.change(sit, 1.0, 0.0, end, oot_game::skelanime_std::ANIMMODE_ONCE, -8.0);
    let r = ru(&mut w, h);
    r.is_sitting_oc_active = true;
    let land = sfx(&w, SfxPos::Actor(h), ru1::NA_SE_EV_LAND_DIRT);
    for _ in 0..(end as usize + 1) {
        update(&mut w, h);
    }
    let r = ru(&mut w, h);
    assert_eq!(r.action, ru1::ENRU1_ACTION_SITTING_IDLE);
    assert!(r.skel.is("gRutoChildSittingAnim"));
    assert!(!r.is_sitting_oc_active);
    assert!(w.save.get_inf_table(ru1::INFTABLE_RUTO_LET_LINK_CARRY));
    // EnRu1_PlaySittingSfx: frame 11.
    assert_eq!(sfx(&w, SfxPos::Actor(h), ru1::NA_SE_EV_LAND_DIRT), land + 1);
}

#[test]
fn lifted_she_is_carried_and_thrown_she_bounces_off_the_floor() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 3, &[ru1::INFTABLE_RUTO_MET_FIRST_TIME]);
    let h = ruto(&w, 0x0203);
    let me = SfxPos::Actor(h);
    let link = SfxPos::Actor(w.player.unwrap());
    ru(&mut w, h).action = ru1::ENRU1_ACTION_SITTING_IDLE;
    // Sitting, Link 100 behind her: she's offered to be lifted (Actor_OfferCarry, GI_NONE).
    let pos = ru(&mut w, h).actor.world_pos;
    w.place_player(pos + Vec3::new(0.0, 0.0, 20.0), -0x8000);
    idle(&mut w, 1);
    ru(&mut w, h).action = ru1::ENRU1_ACTION_SITTING_IDLE;
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).action, ru1::ENRU1_ACTION_SITTING_IDLE);
    assert_eq!(w.player().interact_range_actor, Some(h));
    // Lifted (Player's func on frame 4: her parent set): carried, Link's pull and her cry.
    ru(&mut w, h).actor.parent = w.player;
    update(&mut w, h);
    assert_eq!((ru(&mut w, h).action, ru(&mut w, h).room_num3), (ru1::ENRU1_ACTION_SITTING_CARRIED, 3));
    assert_eq!((sfx(&w, link, ru1::NA_SE_PL_PULL_UP_RUTO), sfx(&w, me, ru1::NA_SE_VO_RT_LIFT)), (1, 1));
    // Let go 60 over the floor, nothing along: released (gravity -1.3, at most 6.8 down), no throw
    // cry with no speed.
    let floor = pos.y;
    let r = ru(&mut w, h);
    r.actor.parent = None;
    r.actor.world_pos = pos + Vec3::Y * 60.0;
    r.actor.prev_pos = r.actor.world_pos;
    r.actor.speed_xz = 0.0;
    r.actor.velocity = Vec3::ZERO;
    update(&mut w, h);
    let r = ru(&mut w, h);
    assert_eq!((r.action, r.actor.gravity, r.actor.min_velocity_y), (ru1::ENRU1_ACTION_SITTING_RELEASED, -1.3, -6.8));
    assert_eq!(sfx(&w, me, oot_game::audio::sfx::NA_SE_VO_RT_THROW), 0);
    // Falling: EnRu1_UpdateBouncing on the scene's floor: velocity.y x -0.6 (then the gravity),
    // NA_SE_EV_LAND_DIRT; the bounce under 0.78 (1.3 x 0.6) stops her: sitting again.
    let mut bounces = 0;
    for i in 0..120 {
        let vy = ru(&mut w, h).actor.velocity.y;
        let land = sfx(&w, me, ru1::NA_SE_EV_LAND_DIRT);
        update(&mut w, h);
        let r = ru(&mut w, h);
        if r.action == ru1::ENRU1_ACTION_SITTING_IDLE {
            break;
        }
        if sfx(&w, me, ru1::NA_SE_EV_LAND_DIRT) > land {
            bounces += 1;
            let r = ru(&mut w, h);
            if r.actor.gravity != 0.0 {
                assert!((r.actor.velocity.y - (vy * -0.6 - 1.3)).abs() < 1e-4, "update {i}: {vy} -> {}", r.actor.velocity.y);
            }
        }
        assert!(i < 119, "never sat");
    }
    let r = ru(&mut w, h);
    assert!(bounces >= 2, "{bounces} bounces");
    assert_eq!((r.action, r.actor.gravity, r.actor.speed_xz), (ru1::ENRU1_ACTION_SITTING_IDLE, 0.0, 0.0));
    assert!((r.actor.world_pos.y - floor).abs() < 1.0, "{} on {floor}", r.actor.world_pos.y);
    // Picked up and thrown (speed 10): her cry, and her speed kept for the throw.
    let link_h = w.player;
    ru(&mut w, h).actor.parent = link_h;
    update(&mut w, h);
    let r = ru(&mut w, h);
    r.actor.parent = None;
    r.actor.speed_xz = 10.0;
    r.actor.velocity.y = 12.0;
    update(&mut w, h);
    let r = ru(&mut w, h);
    assert_eq!((r.action, r.actor.speed_xz), (ru1::ENRU1_ACTION_SITTING_RELEASED, 10.0));
    assert_eq!(sfx(&w, me, oot_game::audio::sfx::NA_SE_VO_RT_THROW), 1);
}

#[test]
fn in_deep_water_she_bobs_sinks_and_in_another_room_fades_and_is_gone() {
    let Some(a) = assets() else { return };
    // Room 1's water (waterBoxes[1], its surface at -1248, the floor 305 below at its middle).
    let mut w = jabu(&a, 1, &[ru1::INFTABLE_RUTO_MET_FIRST_TIME]);
    load_ru1(&a, &mut w);
    let surface = -1248.0;
    let h = spawn_basement_ruto(&mut w, Vec3::new(-160.0, surface + 20.0, -1703.0));
    assert_eq!(ru(&mut w, h).room_num1, 1);
    // Dropped in: released, she falls into the water: entering it.
    let r = ru(&mut w, h);
    r.action = ru1::ENRU1_ACTION_SITTING_RELEASED;
    r.actor.min_velocity_y = -6.8;
    r.actor.gravity = -1.3;
    for _ in 0..30 {
        update(&mut w, h);
        if ru(&mut w, h).action != ru1::ENRU1_ACTION_SITTING_RELEASED {
            break;
        }
    }
    assert_eq!((ru(&mut w, h).action, ru(&mut w, h).water_state), (ru1::ENRU1_ACTION_SITTING_ENTERING_WATER, ru1::ENRU1_WATER_OUTSIDE));
    // EnRu1_UpdateWaterState: her fall cut to a fifth each frame (and her gravity) until it's
    // under 0.1: then still, immersed: bobbing half her depth less 10, 0.95 a frame.
    for _ in 0..10 {
        update(&mut w, h);
        if ru(&mut w, h).water_state != ru1::ENRU1_WATER_OUTSIDE {
            break;
        }
    }
    let r = ru(&mut w, h);
    assert_eq!(r.water_state, ru1::ENRU1_WATER_IMMERSED);
    let depth = r.actor.y_dist_to_water;
    assert!((r.bob_depth - (depth - 10.0) * 0.5).abs() < 1e-3);
    let (start, mut bob) = (r.sinking_start_pos_y, r.bob_depth);
    assert_eq!(start, r.actor.world_pos.y + r.bob_depth);
    let mut phase: i16 = 0;
    let mut n = 0;
    while bob > 1.0 {
        update(&mut w, h);
        let r = ru(&mut w, h);
        assert_eq!(r.actor.world_pos.y, eng_math::cos_s(phase) * -bob + start);
        phase = phase.wrapping_add(0x3E8);
        bob *= 0.95;
        n += 1;
    }
    // Bobbed out: still (bobbing), then sinking: gravity -0.1 to 0.7 a frame.
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).water_state, ru1::ENRU1_WATER_BOBBING, "after {n} bobs");
    update(&mut w, h);
    let r = ru(&mut w, h);
    assert_eq!((r.action, r.actor.gravity, r.actor.min_velocity_y), (ru1::ENRU1_ACTION_SITTING_SINKING, -0.1, -0.7));
    let y = r.actor.world_pos.y;
    for _ in 0..200 {
        update(&mut w, h);
        if ru(&mut w, h).actor.y_dist_to_water > 55.0 {
            break;
        }
    }
    let r = ru(&mut w, h);
    assert!(r.actor.world_pos.y < y - 5.0);
    assert_eq!((r.action, r.actor.velocity.y), (ru1::ENRU1_ACTION_SITTING_SINKING, -0.7));
    // Placed in another room (room 3) and over 50 deep: fading, translucent, 255 down by 255/20 a
    // frame (EnRu1_FadeWhileSinking), gone on the 20th.
    let r = ru(&mut w, h);
    assert!(r.actor.y_dist_to_water > 50.0);
    r.room_num1 = 3;
    update(&mut w, h);
    let r = ru(&mut w, h);
    assert_eq!((r.action, r.draw_config, r.alpha), (ru1::ENRU1_ACTION_SITTING_DISAPPEARING, ru1::ENRU1_DRAW_XLU, 255));
    for t in 1..20 {
        update(&mut w, h);
        let r = ru(&mut w, h);
        let alpha = ((20.0 - t as f32) * 255.0 / 20.0) as u32;
        assert_eq!((r.alpha, r.actor.shape_shadow_alpha), (alpha as i32, alpha as u8), "frame {t}");
    }
    update(&mut w, h);
    assert!(!alive(&w, h));
}

#[test]
fn on_a_blue_switch_she_presses_it_and_with_flag_0x38_sets_placed_on_switch() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 3, &[ru1::INFTABLE_RUTO_MET_FIRST_TIME]);
    let h = ruto(&w, 0x0203);
    // Room 3's own blue switch, 0x3C00: dropped on it she holds it down (its flag 0x3C), but
    // its flag isn't 0x38: INFTABLE_RUTO_PLACED_ON_SWITCH stays clear.
    let blue = *w.actors.all().iter().find(|&&s| w.actors.downcast::<BgBdanSwitch>(s).is_some_and(|s| s.actor.params == 0x3C00)).expect("room 3's blue switch");
    let at = w.actors.actor(blue).unwrap().world_pos;
    let drop = |w: &mut PlayState, h: ActorHandle, at: Vec3| {
        let r = ru(w, h);
        r.action = ru1::ENRU1_ACTION_SITTING_RELEASED;
        r.actor.world_pos = at + Vec3::Y * 15.0;
        r.actor.prev_pos = r.actor.world_pos;
        r.actor.velocity = Vec3::ZERO;
        r.actor.gravity = -1.3;
        r.actor.min_velocity_y = -6.8;
        for _ in 0..30 {
            update(w, h);
            if ru(w, h).action != ru1::ENRU1_ACTION_SITTING_RELEASED {
                break;
            }
        }
    };
    w.save.set_inf_table(ru1::INFTABLE_RUTO_PLACED_ON_SWITCH);
    drop(&mut w, h, at);
    // On a bg actor's floor: stopped dead and sitting.
    assert_eq!(ru(&mut w, h).action, ru1::ENRU1_ACTION_SITTING_IDLE);
    let bg = ru(&mut w, h).actor.floor_bg_id;
    assert_eq!(oot_game::actor_ctx::dyna_poly_get_actor(&w.actors, &w.col, bg), Some(blue));
    assert!(!w.save.get_inf_table(ru1::INFTABLE_RUTO_PLACED_ON_SWITCH));
    // Sitting there (ACTOR_FLAG_CAN_PRESS_SWITCHES) the switch sinks and sets its flag.
    for _ in 0..10 {
        idle(&mut w, 1);
    }
    assert!(w.flags.get_switch(0x3C));
    assert_ne!(w.actors.downcast::<BgBdanSwitch>(blue).unwrap().action, switch::Action::Up);
    // A blue switch with flag 0x38 (the original's): INFTABLE_RUTO_PLACED_ON_SWITCH set.
    let at = Vec3::new(222.0, -1113.0, -3270.0);
    let s = w.actor_spawn(switch::ACTOR_BG_BDAN_SWITCH, at, [0; 3], 0x3800).expect("spawned");
    idle(&mut w, 2);
    let at = w.actors.actor(s).unwrap().world_pos;
    drop(&mut w, h, at);
    let bg = ru(&mut w, h).actor.floor_bg_id;
    assert_eq!(oot_game::actor_ctx::dyna_poly_get_actor(&w.actors, &w.col, bg), Some(s));
    assert!(w.save.get_inf_table(ru1::INFTABLE_RUTO_PLACED_ON_SWITCH));
    for _ in 0..10 {
        idle(&mut w, 1);
    }
    assert!(w.flags.get_switch(0x38));
}

#[test]
fn carried_to_the_sapphire_she_sees_it_and_on_big_octos_platform_takes_it() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 6, &[ru1::INFTABLE_RUTO_MET_FIRST_TIME]);
    // Room 6's own Ruto (0x0004) wants the sapphire taken: gone.
    assert!(rutos(&w).is_empty());
    let h = spawn_basement_ruto(&mut w, Vec3::new(-1100.0, -1025.0, -3343.0));
    let me = SfxPos::Actor(h);
    // Carried in (her parent Link) where Demo_Effect's DEMO_EFFECT_JEWEL_ZORA is:
    // gRutoFoundSapphireCs, NA_SE_VO_RT_DISCOVER.
    let link_h = w.player;
    let r = ru(&mut w, h);
    r.action = ru1::ENRU1_ACTION_SITTING_CARRIED;
    r.actor.parent = link_h;
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).action, ru1::ENRU1_ACTION_SITTING_SEES_SAPPHIRE);
    assert_eq!(w.cs_ctx.segment.as_ref().map(|s| s.name.as_str()), Some("gRutoFoundSapphireCs"));
    assert_eq!(sfx(&w, me, ru1::NA_SE_VO_RT_DISCOVER), 1);
    // The script over: carried again, INFTABLE_RUTO_BROUGHT_TO_SAPPHIRE_ROOM.
    for _ in 0..400 {
        idle(&mut w, 1);
        if ru(&mut w, h).action != ru1::ENRU1_ACTION_SITTING_SEES_SAPPHIRE {
            break;
        }
    }
    assert_eq!(ru(&mut w, h).action, ru1::ENRU1_ACTION_SITTING_CARRIED);
    assert!(w.save.get_inf_table(ru1::INFTABLE_RUTO_BROUGHT_TO_SAPPHIRE_ROOM));
    // Sitting on Big Octo's platform (Bg_Bdan_Objects type 0): gRutoObtainingSapphireCs, hidden,
    // still, her shadow off.
    let p = *w.actors.all().iter().find(|&&p| w.actors.downcast::<BgBdanObjects>(p).is_some_and(|o| o.actor.params == objects::JABU_OBJECTS_TYPE_BIG_OCTO_PLATFORM)).expect("the platform");
    let bg = w.actors.downcast::<BgBdanObjects>(p).unwrap().bg;
    let centre = w.actors.actor(p).unwrap().world_pos;
    let (y, poly) = w.col.entity_raycast_down(centre + Vec3::Y * 300.0);
    assert_eq!(poly.map(|p| p.bg), Some(bg), "the platform's top under its middle");
    let top = Vec3::new(centre.x, y, centre.z);
    let r = ru(&mut w, h);
    r.actor.parent = None;
    r.action = ru1::ENRU1_ACTION_SITTING_IDLE;
    r.room_num2 = 6;
    r.actor.world_pos = top + Vec3::new(0.0, 5.0, 0.0);
    r.actor.prev_pos = r.actor.world_pos;
    let mut on = false;
    for _ in 0..3 {
        update(&mut w, h);
        if ru(&mut w, h).action == ru1::ENRU1_ACTION_THROWN_ONTO_SAPPHIRE_PLATFORM {
            on = true;
            break;
        }
    }
    assert!(on, "not on the platform: {:?}", ru(&mut w, h).action);
    let r = ru(&mut w, h);
    assert_eq!((r.draw_config, r.actor.shape_shadow_alpha, r.big_octo_platform), (ru1::ENRU1_DRAW_NOTHING, 0, Some(p)));
    assert_eq!(w.cs_ctx.segment.as_ref().map(|s| s.name.as_str()), Some("gRutoObtainingSapphireCs"));
    // The script: past cue 1 (frame 40) at (-1352, -969, -3341) facing 0x4000, walking to the
    // sapphire; it held up and put down; the script over: the sapphire hers, her switch flag (2),
    // the platform's CAM_SET_NORMAL0, ready to go home.
    let mut seen = vec![ru1::ENRU1_ACTION_THROWN_ONTO_SAPPHIRE_PLATFORM];
    for f in 0..1400 {
        // The script's texts (0x4050 to 0x4052) wait for A.
        tick_a(&mut w, f);
        let r = ru(&mut w, h);
        if *seen.last().unwrap() != r.action {
            seen.push(r.action);
            if r.action == ru1::ENRU1_ACTION_RETRIEVING_SAPPHIRE {
                // At the cue, then her first root motion (gRutoChildWalkToAndHoldUpSapphireAnim's
                // root starts 71 ahead of its end: she appears where she landed and walks back).
                assert_eq!((r.actor.world_pos.y, r.actor.shape_rot.y, r.draw_config), (-969.0, 0x4000, ru1::ENRU1_DRAW_OPA));
            }
            if r.action == ru1::ENRU1_ACTION_HOLDING_SAPPHIRE {
                // Walked to the sapphire: back at the cue's place.
                assert!((r.actor.world_pos - Vec3::new(-1352.0, -969.0, -3341.0)).length() < 1.0, "{}", r.actor.world_pos);
            }
        }
        if r.action == ru1::ENRU1_ACTION_READY_TO_GO_HOME {
            break;
        }
    }
    assert_eq!(
        seen,
        [
            ru1::ENRU1_ACTION_THROWN_ONTO_SAPPHIRE_PLATFORM,
            ru1::ENRU1_ACTION_RETRIEVING_SAPPHIRE,
            ru1::ENRU1_ACTION_HOLDING_SAPPHIRE,
            ru1::ENRU1_ACTION_DONE_HOLDING_SAPPHIRE,
            ru1::ENRU1_ACTION_MISSION_ACCOMPLISHED,
            ru1::ENRU1_ACTION_READY_TO_GO_HOME,
        ]
    );
    assert!(w.save.get_inf_table(ru1::INFTABLE_RUTO_HAS_SAPPHIRE));
    assert!(w.flags.get_switch(2));
    assert_eq!(ru(&mut w, h).actor.room, 6);
    // (The platform may have read CAM_SET_NORMAL0 already: Link within 250 makes it NORMAL1.)
    let cam = w.actors.downcast::<BgBdanObjects>(p).unwrap().camera_setting;
    assert!(cam == CAM_SET_NORMAL0 || cam == objects::CAM_SET_NORMAL1, "{cam}");
    // The platform rising (CAM_SET_NORMAL1): she looks up and shudders.
    w.actors.downcast_mut::<BgBdanObjects>(p).unwrap().camera_setting = objects::CAM_SET_NORMAL1;
    let throws = sfx(&w, me, oot_game::audio::sfx::NA_SE_VO_RT_THROW);
    update(&mut w, h);
    let r = ru(&mut w, h);
    assert_eq!((r.eyes, r.mouth), (ru1::ENRU1_EYES_UP, ru1::ENRU1_MOUTH_OPEN));
    assert!(r.skel.is("gRutoChildShutterAnim"));
    assert_eq!(sfx(&w, me, oot_game::audio::sfx::NA_SE_VO_RT_THROW), throws + 1);
    // Up (CAM_SET_DUNGEON0): 0x4048, Big Octo takes her (hidden); the box closed, the platform's
    // CAM_SET_DUNGEON1 (Big Octo comes) and she's gone.
    w.actors.downcast_mut::<BgBdanObjects>(p).unwrap().camera_setting = CAM_SET_DUNGEON0;
    update(&mut w, h);
    let r = ru(&mut w, h);
    assert_eq!((r.action, r.draw_config, r.actor.text_id), (ru1::ENRU1_ACTION_BEING_ABDUCTED, ru1::ENRU1_DRAW_NOTHING, 0x4048));
    for f in 0..400 {
        tick_a(&mut w, f);
        if !alive(&w, h) {
            break;
        }
    }
    assert!(!alive(&w, h), "taken");
    assert_eq!(w.actors.downcast::<BgBdanObjects>(p).map(|o| o.camera_setting), Some(objects::CAM_SET_DUNGEON1));
}

#[test]
fn the_fountains_ruto_follows_the_sapphire_cutscenes_cues() {
    let Some(a) = assets() else { return };
    let mut w = play_at(&a, "ENTR_ZORAS_FOUNTAIN_0", "jabu-fish", &[], &[]);
    idle(&mut w, 5);
    load_ru1(&a, &mut w);
    let h = w.actor_spawn(ru1::ACTOR_EN_RU1, Vec3::new(-1571.0, 146.0, 1703.0), [0; 3], 0x0001).expect("spawned");
    // EnRu1_InitOutsideJabuJabu: gazing, smiling, drawn.
    let r = ru(&mut w, h);
    assert_eq!((r.action, r.draw_config, r.eyes, r.mouth), (ru1::ENRU1_ACTION_FOUNTAIN_GAZING_AT_LINK, ru1::ENRU1_DRAW_OPA, ru1::ENRU1_EYES_GAZING, ru1::ENRU1_MOUTH_SMILING));
    w.cs_ctx.segment = w.cutscene_script("gZorasFountainSapphireCs");
    w.save.cutscene_trigger = 1;
    // Channel 3: cue 2 (0 to 325) at (-1571, 146, 1703) facing 0x6000; past it the dive (hidden);
    // past 3 (362) resurfacing at (-1426, 18, 1772) facing -19115; treading; cue 6 (485) on her
    // back; past it (507) swimming away; two before cue 7's end (618) turning back; cue 8 (686) and
    // that done, gone.
    let mut seen: Vec<(i32, u16)> = vec![(ru1::ENRU1_ACTION_FOUNTAIN_GAZING_AT_LINK, 0)];
    let dive = sfx(&w, SfxPos::Actor(h), oot_game::audio::sfx::NA_SE_EV_DIVE_INTO_WATER);
    for f in 0..3000 {
        // The script's texts (0x4025's choice, 0x4026, 0x4027) wait for A.
        tick_a(&mut w, f);
        if !alive(&w, h) {
            break;
        }
        let frame = w.cs_ctx.frames;
        let r = ru(&mut w, h);
        if r.action == ru1::ENRU1_ACTION_FOUNTAIN_GAZING_AT_LINK && frame > 2 && frame < 325 {
            assert_eq!((r.actor.world_pos, r.actor.shape_rot.y), (Vec3::new(-1571.0, 146.0, 1703.0), 0x6000));
        }
        if seen.last().unwrap().0 != r.action {
            seen.push((r.action, frame));
            match r.action {
                ru1::ENRU1_ACTION_FOUNTAIN_DIVING => assert_eq!(r.draw_config, ru1::ENRU1_DRAW_NOTHING),
                ru1::ENRU1_ACTION_FOUNTAIN_RESURFACING => {
                    assert_eq!(r.draw_config, ru1::ENRU1_DRAW_OPA);
                    assert_eq!(r.actor.shape_rot.y, -19115);
                }
                _ => {}
            }
        }
    }
    let actions: Vec<i32> = seen.iter().map(|s| s.0).collect();
    assert_eq!(
        actions,
        [
            ru1::ENRU1_ACTION_FOUNTAIN_GAZING_AT_LINK,
            ru1::ENRU1_ACTION_FOUNTAIN_DIVING,
            ru1::ENRU1_ACTION_FOUNTAIN_RESURFACING,
            ru1::ENRU1_ACTION_FOUNTAIN_TREADING_WATER,
            ru1::ENRU1_ACTION_FOUNTAIN_STARTING_SWIM_BACK,
            ru1::ENRU1_ACTION_FOUNTAIN_SWIMMING_BACK,
            ru1::ENRU1_ACTION_FOUNTAIN_FINISHING_SWIM_BACK,
        ],
        "{seen:?}"
    );
    let at = |a: i32| seen.iter().find(|s| s.0 == a).unwrap().1;
    // (Seen by the script's frame counter after the frame's tick: the cue's start frame, plus the
    // frames the engine's order of the script and the actors puts between them.)
    let near = |f: u16, cue: u16| (cue..=cue + 2).contains(&f);
    assert!(near(at(ru1::ENRU1_ACTION_FOUNTAIN_DIVING), 325), "{seen:?}");
    assert!(near(at(ru1::ENRU1_ACTION_FOUNTAIN_RESURFACING), 362), "{seen:?}");
    assert!(near(at(ru1::ENRU1_ACTION_FOUNTAIN_STARTING_SWIM_BACK), 485), "{seen:?}");
    assert!(near(at(ru1::ENRU1_ACTION_FOUNTAIN_SWIMMING_BACK), 507), "{seen:?}");
    assert!(near(at(ru1::ENRU1_ACTION_FOUNTAIN_FINISHING_SWIM_BACK), 618), "{seen:?}");
    assert!(!alive(&w, h), "gone after cue 8");
    assert_eq!(sfx(&w, SfxPos::Actor(h), oot_game::audio::sfx::NA_SE_EV_DIVE_INTO_WATER), dive + 1);
}

#[test]
fn beside_king_zora_she_talks_or_answers_the_mask() {
    let Some(a) = assets() else { return };
    // Before her warp (EVENTCHKINF_37) Zora's Domain's 0x0005 is gone.
    let mut w = play_at(&a, "ENTR_ZORAS_DOMAIN_1", "zora-letter", &[], &[]);
    idle(&mut w, 5);
    assert!(rutos(&w).is_empty());
    let mut w = play_at(&a, "ENTR_ZORAS_DOMAIN_1", "zora-letter", &[], &[ru1::EVENTCHKINF_37]);
    idle(&mut w, 5);
    let h = ruto(&w, 0x0005);
    let r = ru(&mut w, h);
    assert_eq!((r.action, r.draw_config), (ru1::ENRU1_ACTION_THRONE_IDLE, ru1::ENRU1_DRAW_OPA));
    assert_eq!(r.actor.flags & oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).actor.text_id, 0x402C);
    // MaskReaction_GetTextId(MASK_REACTION_SET_RUTO): the Zora Mask (6) 0x7193, the Bunny Hood
    // (4) 0x7192.
    w.player_mut().current_mask = 6;
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).actor.text_id, 0x7193);
    w.player_mut().current_mask = 4;
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).actor.text_id, 0x7192);
    // Talked to: turned to him while talking.
    ru(&mut w, h).actor.flags |= ACTOR_FLAG_TALK;
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).action, ru1::ENRU1_ACTION_THRONE_TALK);
}

#[test]
fn the_boss_rooms_ruto_rises_through_her_blue_warp_and_scolds_link() {
    let Some(a) = assets() else { return };
    let mut w = jabu(&a, 2, &[ru1::INFTABLE_RUTO_MET_FIRST_TIME]);
    // Barinade's room loads object_warp1 (bdan_boss_room_0 and _1).
    load_object(&a, &mut w, "object_warp1");
    let link = w.actors.actor(w.player.unwrap()).unwrap().world_pos;
    let pos = link + Vec3::new(100.0, 0.0, 0.0);
    let h = w.actor_spawn(ru1::ACTOR_EN_RU1, pos, [0; 3], 0x0000).expect("spawned");
    // EnRu1_InitInBossRoom: 10000 under, blushing, mouth open.
    let r = ru(&mut w, h);
    assert_eq!((r.action, r.actor.shape_y_offset, r.eyes, r.mouth), (ru1::ENRU1_ACTION_PRE_SPAWN_BOSS_ROOM, -10000.0, ru1::ENRU1_EYES_BLUSH, ru1::ENRU1_MOUTH_OPEN));
    update(&mut w, h);
    assert_eq!(ru(&mut w, h).action, ru1::ENRU1_ACTION_SPAWN_BOSS_ROOM);
    // Turned to Link, drawn, her warp spawned as her child 12 towards him (WARP_BLUE_RUTO).
    let yaw = ru(&mut w, h).actor.yaw_towards_player;
    update(&mut w, h);
    let r = ru(&mut w, h);
    assert_eq!((r.action, r.draw_config, r.actor.shape_rot.y), (ru1::ENRU1_ACTION_RISE_THROUGH_BLUE_WARP, ru1::ENRU1_DRAW_OPA, yaw));
    let wh = r.blue_warp.expect("her warp");
    assert_eq!(r.actor.child, Some(wh));
    let wa = w.actors.actor(wh).unwrap();
    assert_eq!((wa.id, wa.params, wa.parent), (oot_actors::boss_goma::ACTOR_DOOR_WARP1, warp::WARP_BLUE_RUTO, Some(h)));
    let expect = Vec3::new(pos.x + 12.0 * eng_math::sin_s(yaw), pos.y, pos.z + 12.0 * eng_math::cos_s(yaw));
    assert!((wa.world_pos - expect).length() < 1e-3, "{} vs {expect}", wa.world_pos);
    // Up 250/3 a frame: 10000 / 83.33 = 120 more updates, then waiting, the warp ready.
    for i in 0..122 {
        update(&mut w, h);
        // (250/3 summed in floats is a hair short of 10000 after 120: the 121st.)
        assert_eq!(ru(&mut w, h).action == ru1::ENRU1_ACTION_WAIT_INSIDE_BLUE_WARP, i >= 120, "update {i}: {}", ru(&mut w, h).actor.shape_y_offset);
    }
    assert_eq!(ru(&mut w, h).actor.shape_y_offset, 0.0);
    assert_eq!(w.actors.downcast::<DoorWarp1>(wh).unwrap().ruto_warp_state, warp::WARP_BLUE_RUTO_STATE_READY);
    // The warp: Link entered. His mark 30 in front of her (unk_450).
    w.actors.downcast_mut::<DoorWarp1>(wh).unwrap().ruto_warp_state = warp::WARP_BLUE_RUTO_STATE_ENTERED;
    update(&mut w, h);
    let r = ru(&mut w, h);
    assert_eq!(r.action, ru1::ENRU1_ACTION_LINK_WALKS_TO_POINT_IN_BLUE_WARP);
    let (rp, rr) = (r.actor.world_pos, r.actor.shape_rot.y);
    let mark = w.player().unk_450;
    assert_eq!((mark.x, mark.z), (30.0 * eng_math::sin_s(rr) + rp.x, 30.0 * eng_math::cos_s(rr) + rp.z));
    // Link still: from the third frame he's on his mark and turned to her; facing her, she
    // crosses her arms and legs (WARP_BLUE_RUTO_STATE_3).
    w.actors.actor_mut(w.player.unwrap()).unwrap().speed_xz = 0.0;
    for _ in 0..100 {
        update(&mut w, h);
        if ru(&mut w, h).action != ru1::ENRU1_ACTION_LINK_WALKS_TO_POINT_IN_BLUE_WARP {
            break;
        }
    }
    assert_eq!(ru(&mut w, h).action, ru1::ENRU1_ACTION_WHAT_TOOK_YOU_SO_LONG);
    assert_eq!(w.actors.actor(w.player.unwrap()).unwrap().world_pos, w.player().unk_450);
    assert_eq!(w.actors.downcast::<DoorWarp1>(wh).unwrap().ruto_warp_state, warp::WARP_BLUE_RUTO_STATE_3);
    // The warp warping: she rises with Link, her distance to him kept.
    w.actors.downcast_mut::<DoorWarp1>(wh).unwrap().ruto_warp_state = warp::WARP_BLUE_RUTO_STATE_WARPING;
    update(&mut w, h);
    let r = ru(&mut w, h);
    assert_eq!(r.action, ru1::ENRU1_ACTION_WARPING_OUT);
    assert_eq!(r.xz_dist_to_player_in_blue_warp, r.actor.xz_dist_to_player);
}
