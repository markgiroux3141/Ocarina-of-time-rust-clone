//! GAME-09 milestone 3 against the C: the clear blocks and their timer (`z_bg_gnd_darkmeiro.c`).
//! Dodongo's Cavern (Master Quest) room 8's two clear blocks (`DARKMEIRO_CLEAR_BLOCK`), both on
//! edge (`shape.rot.x` 0x4000), plug the way from the Armos' floor (y 16) to the west platform (y
//! 76, `En_Holl` 4 at (593, 76, -3814)): 0xFF01 at (1193, 66, -3841), always there (flag 0x3F), its
//! top at 116; 0x1901 at (1273, 16, -3841), there with switch flag 0x19 (the wall crystal
//! `Obj_Switch` 0x1903 at (1106, 148, -3966)), its top at 66: the step up to the other. The timer
//! (`DARKMEIRO_BLOCK_TIMER`) isn't placed in Dodongo's Cavern; its test spawns one.
//!
//! Child Link enters at room 7 (`ENTR_DODONGOS_CAVERN_1`) on a `cavern-bombs` save and goes into
//! room 8 (`Room_RequestNewRoom`). Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_gnd_darkmeiro::*;
use oot_actors::en_am::EnAm;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorHandle};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const STATIC_BLOCK: Vec3 = Vec3::new(1193.0, 66.0, -3841.0);
const SWITCH_BLOCK: Vec3 = Vec3::new(1273.0, 16.0, -3841.0);

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in room 8 (`cavern-bombs`, the sound log on), entered at room 7 and the room
/// changed; on the gravestones' floor, two frames on.
fn room8(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_1").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("cavern-bombs").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert!(w.room_request(8));
    idle(&mut w, 1);
    w.room_change_done();
    idle(&mut w, 3);
    w.place_player(Vec3::new(1500.0, 16.0, -3600.0), 0);
    idle(&mut w, 2);
    w
}

fn blocks(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.category(ACTORCAT_PROP).iter().copied().filter(|&h| w.actors.downcast::<BgGndDarkmeiro>(h).is_some_and(|c| !c.actor.killed)).collect()
}

fn at_home(w: &PlayState, home: Vec3) -> ActorHandle {
    *blocks(w).iter().find(|&&h| d(w, h).actor.home_pos == home).expect("the block")
}

fn d(w: &PlayState, h: ActorHandle) -> &BgGndDarkmeiro {
    w.actors.downcast::<BgGndDarkmeiro>(h).expect("Bg_Gnd_Darkmeiro")
}

/// The floor under (x, z) from y 300, and whose (`Some(bg)` a bg actor's).
fn floor(w: &PlayState, x: f32, z: f32) -> (f32, Option<u16>) {
    let (y, p) = w.col.entity_raycast_down(Vec3::new(x, 300.0, z));
    (y, p.map(|p| p.bg).filter(|&bg| bg != eng_collision::bgcheck::BGCHECK_SCENE))
}

/// Whether the draw would draw it (`render_state`'s switch: 2 the block).
fn drawn(w: &PlayState, h: ActorHandle) -> bool {
    use oot_game::actor_ctx::ActorImpl;
    d(w, h).render_state().switches == [2]
}

#[test]
fn room_8s_clear_blocks() {
    let Some(a) = assets() else { return };
    let w = room8(&a);
    assert_eq!(blocks(&w).len(), 2);
    let s = at_home(&w, STATIC_BLOCK);
    let x = d(&w, s);
    // 0xFF01: DARKMEIRO_CLEAR_BLOCK, gClearBlockCol; flag (bits 8 to 13) 0x3F: the static block
    // (BgGndDarkmeiro_UpdateStaticBlock, _DrawStaticBlock), its collision on. Scale 0.1; on edge:
    // 100 high from 16 to 116, 40 across in x (1173 to 1213), 100 in z.
    assert_eq!((x.update_func, x.draw_func, x.actor.scale.x), (UpdateFunc::UpdateStaticBlock, Some(DrawFunc::StaticBlock), 0.1));
    let (top, bg) = floor(&w, STATIC_BLOCK.x, STATIC_BLOCK.z);
    assert_eq!((top.round(), bg), (116.0, Some(x.bg)));
    assert!(drawn(&w, s));
    let b = at_home(&w, SWITCH_BLOCK);
    let x = d(&w, b);
    // 0x1901: the switch block, flag 0x19 clear: its collision off (DynaPoly_DisableCollision),
    // timer1 0, not drawn.
    assert_eq!((x.update_func, x.draw_func, x.timer1, x.action_flags), (UpdateFunc::UpdateSwitchBlock, Some(DrawFunc::SwitchBlock), 0, 0));
    assert_eq!(floor(&w, SWITCH_BLOCK.x, SWITCH_BLOCK.z), (16.0, None));
    assert!(!drawn(&w, b));
}

#[test]
fn flag_0x19_brings_the_switch_block_which_goes_64_frames_after_it_clears() {
    let Some(a) = assets() else { return };
    let mut w = room8(&a);
    let h = at_home(&w, SWITCH_BLOCK);
    let bg = d(&w, h).bg;
    w.flags.set_switch(0x19);
    // BgGndDarkmeiro_UpdateSwitchBlock: timer1 64 while the flag's set; _ToggleBlock: the
    // collision on (actionFlags 2), there from the next DynaPoly_UpdateContext: its top at 66.
    idle(&mut w, 1);
    assert_eq!((d(&w, h).timer1, d(&w, h).action_flags), (64, 2));
    assert!(drawn(&w, h));
    idle(&mut w, 1);
    let (top, on) = floor(&w, SWITCH_BLOCK.x, SWITCH_BLOCK.z);
    assert_eq!((top.round(), on), (66.0, Some(bg)));
    // BgGndDarkmeiro_DrawSwitchBlock's timer2 with timer1 64 (not over 64): cos(64 × 0x1000, an
    // s16: 0) × 64 + 127, at most 127. (The list's own prim colour hides it: @bug (game).)
    assert_eq!(d(&w, h).timer2, 127);
    // The flag cleared: 64 frames down, the block there and drawn, its alpha flickering
    // (cos(timer1 × 0x1000) × 64 + 127 to 16, then timer1 × 8); at 0 the collision off.
    w.flags.unset_switch(0x19);
    for t in (1..64).rev() {
        idle(&mut w, 1);
        let x = d(&w, h);
        assert_eq!((x.timer1, x.action_flags), (t, 2));
        let want = if t > 16 { ((eng_math::cos_s((t as i32 * 0x1000) as i16) * 64.0 + 127.0) as i16).min(127) } else { t * 8 };
        assert_eq!(x.timer2, want, "timer1 {t}");
    }
    idle(&mut w, 1);
    assert_eq!((d(&w, h).timer1, d(&w, h).action_flags), (0, 0));
    assert!(!drawn(&w, h));
    idle(&mut w, 1);
    assert_eq!(floor(&w, SWITCH_BLOCK.x, SWITCH_BLOCK.z), (16.0, None));
}

#[test]
fn the_block_timer_holds_its_flag_while_a_timer_runs() {
    let Some(a) = assets() else { return };
    let mut w = room8(&a);
    // A timer of flag 0x10 (DARKMEIRO_BLOCK_TIMER: params 0x1002), spawned in room 8 (its object's
    // there): flags 0x11 and 0x12 clear, it clears 0x10 at init.
    w.flags.set_switch(0x10);
    let h = w.actor_spawn(ACTOR_BG_GND_DARKMEIRO, Vec3::new(1500.0, 16.0, -3560.0), [0, 0, 0], 0x1002).expect("the timer");
    assert!(!w.flags.get_switch(0x10));
    assert_eq!((d(&w, h).update_func, d(&w, h).draw_func, d(&w, h).timer1, d(&w, h).timer2), (UpdateFunc::UpdateBlockTimer, None, 0, 0));
    // Flag 0x11 (a switch's): the first timer starts, 304, NA_SE_EV_RED_EYE (centred); flag 0x10
    // set while it's 64 or more, the timer's tick (Actor_PlaySfx_FlaggedTimer) as it runs.
    w.flags.set_switch(0x11);
    idle(&mut w, 1);
    assert_eq!((d(&w, h).timer1, d(&w, h).action_flags), (304, 4));
    assert!(w.flags.get_switch(0x10));
    let red_eye = w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(f, s, p)| f == w.audio.frames && s == NA_SE_EV_RED_EYE && p == oot_game::audio::sfx::SfxPos::Default).count();
    assert_eq!(red_eye, 1);
    assert_ne!(d(&w, h).actor.flags & oot_game::actor::ACTOR_FLAG_SFX_TIMER, 0);
    for t in (64..304).rev() {
        idle(&mut w, 1);
        assert_eq!(d(&w, h).timer1, t);
        assert!(w.flags.get_switch(0x10), "timer1 {t}");
    }
    // Under 64: flag 0x10 cleared (the blocks of 0x10 start their 64 frames).
    idle(&mut w, 1);
    assert_eq!(d(&w, h).timer1, 63);
    assert!(!w.flags.get_switch(0x10));
    // At 0, the next update clears flag 0x11 and the timer stops.
    for _ in 0..63 {
        idle(&mut w, 1);
    }
    assert_eq!((d(&w, h).timer1, w.flags.get_switch(0x11)), (0, true));
    idle(&mut w, 1);
    assert_eq!((d(&w, h).action_flags, w.flags.get_switch(0x11)), (0, false));
    // Spawned with 0x12 set: its second timer at 64 from the init, and 0x10 set there.
    w.flags.set_switch(0x12);
    let h2 = w.actor_spawn(ACTOR_BG_GND_DARKMEIRO, Vec3::new(1500.0, 16.0, -3560.0), [0, 0, 0], 0x1002).expect("the timer");
    assert_eq!((d(&w, h2).timer2, d(&w, h2).action_flags), (64, 8));
    assert!(w.flags.get_switch(0x10));
}

/// Room 8's west way out: the Armos beaten (killed here), flag 0x19's block a step (66) up to
/// the static one (116), and down onto the west platform (76) where `En_Holl` 4 is.
#[test]
fn the_switch_block_is_the_step_to_the_west_platform() {
    let Some(a) = assets() else { return };
    let mut w = room8(&a);
    for h in w.actors.all() {
        if w.actors.downcast::<EnAm>(h).is_some() {
            w.actors.actor_mut(h).unwrap().kill();
        }
    }
    // The stick forward at a run (a stick under 55 doesn't climb up from a hang:
    // Player_ProcessControlStick's spin angle -1), until Link is past x 1000 or `frames` run.
    let walk_west = |w: &mut PlayState, frames: usize| -> f32 {
        let mut prev = PadState::default();
        let mut top = 0.0f32;
        for _ in 0..frames {
            let cur = PadState { button: 0, stick_x: 0, stick_y: 80 };
            w.tick_with(scripted_input(prev, cur));
            prev = cur;
            top = top.max(w.player().actor.world_pos.y);
            if w.player().actor.world_pos.x < 1000.0 {
                break;
            }
        }
        top
    };
    // Without the block: Link (facing west, -x) walks into the static block's east face (100
    // high: no climbing it from the floor) and stays on the floor east of it.
    w.place_player(Vec3::new(1340.0, 16.0, -3841.0), -0x4000);
    idle(&mut w, 2);
    walk_west(&mut w, 90);
    let p = w.player().actor.world_pos;
    assert!(p.x > 1213.0 && p.y < 20.0, "Link at {p:?}");
    // With flag 0x19: a hop up (ClimbLedge) to hang from the switch block's edge (50 up) and climb
    // it, the same onto the static block (116), then off its west side down to the platform.
    w.flags.set_switch(0x19);
    w.place_player(Vec3::new(1340.0, 16.0, -3841.0), -0x4000);
    idle(&mut w, 3);
    let top = walk_west(&mut w, 300);
    let p = w.player().actor.world_pos;
    assert!(top >= 115.0, "on the static block: top {top}");
    assert!(p.x < 1000.0 && (p.y - 76.0).abs() < 1.0, "on the west platform: {p:?}");
}

/// Flag 0x19's crystal (`Obj_Switch` 0x1903, on the west platform's side at (1106, 148, -3966)) is
/// out of a child's reach from the Armos' floor; the Bomb Flowers on the south wall lead up to it
/// ((1276, 87, -3961), (1240, 155), (1199, 216), then (1159, 196, -3921) and (1133, 158, -3921) by
/// the crystal), each set off by the last one's blast. A blast low by the wall sets off the first:
/// the southmost Armos' own (shape.rot.z 2: its sphere grows to 100), killed with Link by the wall,
/// does. The chain's last blast hits the crystal: 0x19, and the switch block comes.
#[test]
fn an_armos_blast_by_the_south_wall_sets_off_the_bomb_flowers_up_to_crystal_0x19() {
    let Some(a) = assets() else { return };
    let mut w = room8(&a);
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnAm>(h).is_some_and(|x| x.actor.home_pos == Vec3::new(1290.0, 16.0, -3942.0))).expect("the southmost Armos");
    // Link by the south wall east of it; the Armos woken (as his touch does), then killed (a
    // killing hit as CollisionCheck_AC leaves it: an explosive's AM_DMG_REACT_KILL).
    w.place_player(Vec3::new(1300.0, 16.0, -3955.0), 0x4000);
    idle(&mut w, 1);
    w.actors.downcast_mut::<EnAm>(h).unwrap().unk_258 = 1;
    idle(&mut w, 14);
    {
        let x = w.actors.downcast_mut::<EnAm>(h).unwrap();
        x.hurt_collider.base.ac_flags |= oot_game::collision_check::AC_HIT;
        x.actor.col_chk_info.damage = 2;
        x.actor.col_chk_info.damage_reaction = oot_actors::en_am::AM_DMG_REACT_KILL;
    }
    let block = at_home(&w, SWITCH_BLOCK);
    let mut flowers_blown = 0;
    let mut n = 0;
    while !w.flags.get_switch(0x19) {
        idle(&mut w, 1);
        flowers_blown += w.actors.all().into_iter().filter(|&b| w.actors.downcast::<oot_actors::en_bombf::EnBombf>(b).is_some_and(|f| f.actor.params == 1 && f.timer == 9)).count();
        n += 1;
        assert!(n < 300, "0x19 never set");
    }
    assert!(w.actors.get(h).is_none_or(|a| a.base().killed), "the Armos blew up");
    assert_eq!(flowers_blown, 5, "the five Bomb Flowers, one after another");
    idle(&mut w, 1);
    assert_eq!(d(&w, block).timer1, 64);
}
