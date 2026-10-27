//! GAME-13 milestone 1 against the C: the hookshot (`Arms_Hook`, Player's hookshot,
//! `Camera_Special5`). Goron City's room 3 (`ootx scene-info --scene spot18`) has the stone post
//! `Obj_Hsblock` 0xFFC0 at (1161, 480, -148), its floor west of it at its height.
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_CLEFT, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::arms_hook::{Action, ArmsHook};
use oot_actors::player::Action as PA;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const POST: Vec3 = Vec3::new(1161.0, 480.0, -148.0);

/// An adult at Goron City's bottom (room 3) with the hookshot on C-Left, set 200 west of the post
/// facing it (the ledge at its height runs 100 to 200 west of it), settled.
fn by_the_post(a: &Arc<GameAssets>) -> PlayState {
    use oot_game::item::{ITEM_HOOKSHOT, slot};
    let e = a.scenes.entrance_index("ENTR_GORON_CITY_2").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("epona-won").unwrap();
    let s = slot(ITEM_HOOKSHOT);
    save.inventory.items[s] = ITEM_HOOKSHOT;
    save.equips.button_items[1] = ITEM_HOOKSHOT;
    save.equips.c_button_slots[0] = s as u8;
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.place_player(POST + Vec3::new(-200.0, 0.0, 0.0), 0x4000);
    for _ in 0..20 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w
}

fn hook(w: &PlayState) -> Option<&ArmsHook> {
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<ArmsHook>(h)).find(|h| !h.actor.killed)
}

#[test]
fn the_hookshot_pulls_link_to_the_post() {
    let Some(a) = assets() else { return };
    let mut w = by_the_post(&a);
    let start = w.player().actor.world_pos;
    // C-Left: the hookshot in hand (Player_UseItem's change, then Player_InitHookshotIA): the hook
    // spawned as Player's child and held, waiting.
    let c = PadState { button: BTN_CLEFT, ..Default::default() };
    let mut prev = PadState::default();
    w.tick_with(scripted_input(prev, c));
    prev = c;
    let none = PadState::default();
    for _ in 0..30 {
        w.tick_with(scripted_input(prev, none));
        prev = none;
    }
    let h = hook(&w).expect("Arms_Hook spawned");
    assert_eq!(h.action, Action::Wait);
    assert_eq!(w.player().held_actor, w.actors.all().into_iter().find(|&x| w.actors.downcast::<ArmsHook>(x).is_some_and(|k| !k.actor.killed)));
    // C-Left held, then let go: raised in first person (func_80834D2C, func_80834EB8), the shot
    // (func_808351D4, func_808350A4): Player lets go of it (its parent cleared), ArmsHook_Wait
    // sends it out at 20 for 13 frames.
    let (shot_at, flight, landed) = shoot(&mut w, &mut prev, -40);
    assert!(shot_at.is_some(), "never shot");
    // ArmsHook_Shoot: the post's upper band takes the hookshot (SurfaceType_CanHookshot), on the
    // post's bg actor: attached (ArmsHook_AttachToActor) and pulling (ArmsHook_PullPlayer: Player's
    // parent). Player_UpdateUpperBody: Player_Action_HookshotFly, PLAYER_STATE3_FLYING_WITH_HOOKSHOT,
    // CAM_MODE_HOOKSHOT_FLY (Camera_Special5) on the hook.
    let (first, last, attached_to_post, cam) = flight.expect("Link never flew");
    assert!(attached_to_post, "the hook wasn't on the post");
    assert!(cam, "no CAM_MODE_HOOKSHOT_FLY");
    eprintln!("flew {first}..{last}");
    // The hook back on the hookshot (func_80834FBC): he lands, by the post, and holds it again.
    let (_, at) = landed.expect("landed");
    assert!(at.x > start.x + 100.0 && (at.x - POST.x).abs() < 60.0, "Link at {at}, from {start}");
    assert_eq!(hook(&w).map(|k| k.action), Some(Action::Wait));
    assert!(w.player().held_actor.is_some());
}

/// Aims (C-Left held, the stick up for frames 8 to 13 by `stick_y`), shoots at frame 25, and
/// watches 200 frames: the shot's frame; the flight's first and last frames, whether the hook was
/// on the post and the camera in `CAM_MODE_HOOKSHOT_FLY`; where Link stood after it.
#[allow(clippy::type_complexity)]
fn shoot(w: &mut PlayState, prev: &mut PadState, stick_y: i8) -> (Option<usize>, Option<(usize, usize, bool, bool)>, Option<(usize, Vec3)>) {
    use oot_game::camera::CAM_MODE_HOOKSHOT_FLY;
    let c = PadState { button: BTN_CLEFT, ..Default::default() };
    let none = PadState::default();
    let (mut shot_at, mut flight, mut landed) = (None, None::<(usize, usize, bool, bool)>, None);
    for f in 0..200 {
        let pad = if f < 25 { PadState { stick_y: if (8..14).contains(&f) { stick_y } else { 0 }, ..c } } else { none };
        w.tick_with(scripted_input(*prev, pad));
        *prev = pad;
        let on_post = hook(w).and_then(|k| k.attached_actor).and_then(|h| w.actors.downcast::<oot_actors::obj_hsblock::ObjHsblock>(h)).is_some();
        if shot_at.is_none() && hook(w).is_some_and(|k| k.action == Action::Shoot) {
            shot_at = Some(f);
        }
        let p = w.player();
        if p.action == PA::HookshotFly {
            let cam = w.game_camera.mode == CAM_MODE_HOOKSHOT_FLY;
            flight = Some(match flight {
                None => (f, f, on_post, cam),
                Some((a, _, o, c)) => (a, f, o || on_post, c || cam),
            });
        } else if flight.is_some() && landed.is_none() && p.grounded() {
            landed = Some((f, p.actor.world_pos));
        }
    }
    (shot_at, flight, landed)
}

#[test]
fn the_posts_lower_band_throws_the_hook_back() {
    let Some(a) = assets() else { return };
    let mut w = by_the_post(&a);
    let start = w.player().actor.world_pos;
    let c = PadState { button: BTN_CLEFT, ..Default::default() };
    let mut prev = PadState::default();
    w.tick_with(scripted_input(prev, c));
    prev = c;
    for _ in 0..30 {
        w.tick_with(scripted_input(prev, PadState::default()));
        prev = PadState::default();
    }
    // Level, into the post's lower band (its polys there don't take the hookshot): the spark and
    // NA_SE_IT_HOOKSHOT_REFLECT, then back at 200 a frame and held again; Link stays.
    let (shot_at, flight, _) = shoot(&mut w, &mut prev, 0);
    assert!(shot_at.is_some());
    assert!(flight.is_none(), "Link flew");
    assert_eq!(hook(&w).map(|k| k.action), Some(Action::Wait));
    assert!(w.player().actor.world_pos.distance(start) < 5.0);
}
