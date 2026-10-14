//! GAME-07 milestone 6 against the C: Dampé the gravekeeper (`z_en_tk.c`) and his dig spots
//! (`z_en_it.c`) in the graveyard (`spot02_scene` layer 1, the child's night).
//!
//! Expected values are worked out from the C in the comments. The graveyard is entered at
//! `ENTR_GRAVEYARD_0` with its intro seen (`EVENTCHKINF_B6`: `sEntranceCutsceneTable`'s
//! `gGraveyardIntroCs`).

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_it::{self, EnIt};
use oot_actors::en_item00::{EnItem00, ITEM00_HEART_PIECE, ITEM00_RUPEE_GREEN, ITEM00_RUPEE_PURPLE};
use oot_actors::en_tk::{self, Action, EnTk};
use oot_actors::player::Player;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_ERROR, NA_SE_SY_TRE_BOX_APPEAR};
use oot_game::env::clock_time;
use oot_game::message::*;
use oot_game::play::{PlayState, Rand, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_B6` (`save.h`: 0xB6): the graveyard's intro seen.
const EVENTCHKINF_B6: u16 = 0xB6;
/// Dampé's place (`spot02_scene` layer 1: `En_Tk` 0 at (-476, 61, 434), yaw 0x8000).
const DAMPE_HOME: Vec3 = Vec3::new(-476.0, 61.0, 434.0);
/// The eight `En_It` dig spots of layer 1.
const SPOTS: [[f32; 3]; 8] = [[-473.0, 60.0, 202.0], [-473.0, 60.0, 502.0], [-409.0, 120.0, -234.0], [-335.0, 60.0, 681.0], [-177.0, 120.0, -231.0], [43.0, 120.0, -108.0], [48.0, 120.0, 222.0], [48.0, 120.0, 535.0]];

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Link (a child, or an adult) entered at `ENTR_GRAVEYARD_0` at `hour`:`min` with the intro
/// seen and `rupees`, `edit` applied to the save last; the sound requests logged; two frames on.
fn enter(a: &Arc<GameAssets>, adult: bool, hour: i32, min: i32, rupees: i16, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_GRAVEYARD_0").expect("entrance");
    let mut save = SaveContext::new(e, adult, clock_time(hour, min) as u16);
    save.set_event_chk_inf(EVENTCHKINF_B6);
    save.rupees = rupees;
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    idle(&mut w, 2);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// `b` pressed for one frame, then let go.
fn press(w: &mut PlayState, b: u16) {
    let pad = PadState { button: b, ..Default::default() };
    w.tick_with(scripted_input(PadState::default(), pad));
    w.tick_with(scripted_input(pad, PadState::default()));
}

fn all<T: 'static + ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn dampe(w: &PlayState) -> ActorHandle {
    let v = all::<EnTk>(w);
    assert_eq!(v.len(), 1, "one Dampé");
    v[0]
}

fn tk(w: &PlayState) -> &EnTk {
    w.actors.downcast::<EnTk>(dampe(w)).unwrap()
}

fn tk_mut(w: &mut PlayState) -> &mut EnTk {
    let h = dampe(w);
    w.actors.downcast_mut::<EnTk>(h).unwrap()
}

fn base(w: &PlayState, h: ActorHandle) -> &Actor {
    w.actors.actor(h).unwrap()
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

/// The spot at `p`.
fn spot(w: &PlayState, p: [f32; 3]) -> ActorHandle {
    let v: Vec<_> = all::<EnIt>(w).into_iter().filter(|&h| base(w, h).home_pos == Vec3::from(p)).collect();
    assert_eq!(v.len(), 1, "one spot at {p:?}");
    v[0]
}

/// How many times `id` was asked for since the log had `from` entries.
fn sfx_since(w: &PlayState, from: usize, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx[from..].iter().filter(|&&(_, s, _)| s == id).count()
}

fn sfx_len(w: &PlayState) -> usize {
    w.audio.log.as_ref().unwrap().sfx.len()
}

/// Link `dist` in front of Dampé (along his facing), facing him, on the ground.
fn link_in_front(w: &mut PlayState, dist: f32) {
    let t = tk(w);
    let (pos, yaw) = (t.actor.world_pos, t.actor.shape_rot.y);
    let mut p = pos + Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw)) * dist;
    let (floor, _) = w.col.entity_raycast_down(p + Vec3::Y * 50.0);
    p.y = floor;
    w.place_player(p, yaw.wrapping_add(0x8000u16 as i16));
}

/// Dampé moved to `pos` facing `yaw`, resting (`EnTk_Rest` with 100 frames to go).
fn move_dampe(w: &mut PlayState, pos: Vec3, yaw: i16) {
    let t = tk_mut(w);
    t.actor.world_pos = pos;
    t.actor.prev_pos = pos;
    t.actor.shape_rot.y = yaw;
    t.actor.world_rot.y = yaw;
    t.actor.speed_xz = 0.0;
    t.action = Action::Rest;
    t.action_countdown = 100;
    t.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
}

/// Frames until Dampé offers to talk (Player's talk target): Link within 100 and in front of
/// his face (`EnTk_CheckFacingPlayer`: his yaw less `h_21E`, the rest's head and jaw yaw of
/// about -6000, and `headRot`, which `EnTk_Rest` eases to 1270 and more), within 60
/// (`Npc_UpdateTalking`). Settles in a few frames as `headRot` grows.
fn wait_for_offer(w: &mut PlayState) -> usize {
    let h = dampe(w);
    for n in 0..60 {
        if player(w).target_actor == Some(h) && tk(w).check_facing_player() {
            return n;
        }
        idle(w, 1);
    }
    let t = tk(w);
    panic!("no offer: {:?} facing {} h_21E {} headRot {} dist² {}", t.action, t.check_facing_player(), t.h_21e, t.head_rot, t.actor.xyz_dist_to_player_sq);
}

/// Ticks until `f` holds (at most `max`): A pressed on a frame of its own at each box that
/// waits, and at a choice the stick down until `choice` is picked. The ticks taken.
fn talk_until(w: &mut PlayState, choice: u8, max: usize, what: &str, f: impl Fn(&PlayState) -> bool) -> usize {
    let mut prev = PadState::default();
    for n in 0..max {
        if f(w) {
            return n;
        }
        let st = w.message_state();
        let mut pad = PadState::default();
        if st == TEXT_STATE_CHOICE && w.msg_ctx.choice_index != choice {
            if prev.stick_y == 0 {
                pad.stick_y = -60;
            }
        } else if matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_EVENT) && prev.button == 0 {
            pad.button = BTN_A;
        }
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
    }
    panic!("{what}: not after {max} frames (msgMode {:#x}, text {:#x}, state {}, Dampé {:?} talk {})", w.msg_ctx.msg_mode, w.msg_ctx.text_id, w.message_state(), tk(w).action, tk(w).interact_info.talk_state);
}

/// Talks to Dampé from 50 in front of him (A on his offer) and answers his question with
/// `choice`: until the talk's over or he's been paid (`NPC_TALK_STATE_ACTION`). The texts
/// seen.
fn talk_to_dampe(w: &mut PlayState, choice: u8) -> Vec<u16> {
    link_in_front(w, 50.0);
    wait_for_offer(w);
    press(w, BTN_A);
    let mut texts = Vec::new();
    let mut prev = PadState::default();
    for _ in 0..600 {
        let t = tk(w).interact_info.talk_state;
        let st = w.message_state();
        if st != TEXT_STATE_NONE && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        if t == en_tk::NPC_TALK_STATE_ACTION || (t == en_tk::NPC_TALK_STATE_IDLE && texts.len() > 1 && st == TEXT_STATE_NONE) {
            return texts;
        }
        let mut pad = PadState::default();
        if st == TEXT_STATE_CHOICE && w.msg_ctx.choice_index != choice {
            if prev.stick_y == 0 {
                pad.stick_y = -60;
            }
        } else if matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_EVENT) && prev.button == 0 {
            pad.button = BTN_A;
        }
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
    }
    panic!("the talk didn't end: {texts:x?} (state {}, talk {})", w.message_state(), tk(w).interact_info.talk_state);
}

#[test]
fn dampe_is_there_for_a_child_on_the_graveyards_evenings_only() {
    let Some(a) = assets() else { return };
    // EnTk_Init: killed when dayTime <= CLOCK_TIME(18, 0) (0xC000) or >= CLOCK_TIME(21, 0)
    // (0xE000), for an adult, or out of SCENE_GRAVEYARD.
    assert_eq!((clock_time(18, 0), clock_time(21, 0)), (0xC000, 0xE000));
    let w = enter(&a, false, 19, 0, 0, |_| {});
    println!("graveyard layer {}, {} actors", w.save.scene_layer, w.actors.total());
    let h = dampe(&w);
    let t = tk(&w);
    // Placed at (-476, 61, 434), params 0; scale 0.01, ATTENTION_RANGE_6, gravity -0.1,
    // currentReward -1, no spot, EnTk_Rest with gDampeRestAnim.
    assert_eq!(t.actor.home_pos, DAMPE_HOME);
    assert_eq!(t.actor.params, 0);
    assert_eq!(t.actor.scale, Vec3::splat(0.01));
    assert_eq!((t.actor.target_mode, t.actor.gravity, t.current_reward), (6, -0.1, -1));
    assert_eq!(t.actor.col_chk_info.mass, 0xFF);
    assert_eq!((t.collider.dim.radius, t.collider.dim.height), (30, 52));
    // The eight spots: EnIt_Init sets their params to 0x0D05.
    let spots = all::<EnIt>(&w);
    assert_eq!(spots.len(), 8);
    for p in SPOTS {
        let s = spot(&w, p);
        assert_eq!(base(&w, s).params, en_it::EN_IT_PARAMS);
        assert_eq!(base(&w, s).category, oot_game::actor_ctx::ACTORCAT_PROP);
    }

    // By the time, spawned directly into this play state (object_tk is in): 18:00 exactly
    // gone; 18:01 and 20:59 there; 21:00 gone.
    let mut w = w;
    let pos = base(&w, h).world_pos;
    for (hour, min, there) in [(18, 0, false), (18, 1, true), (20, 59, true), (21, 0, false), (17, 59, false), (23, 0, false)] {
        w.save.day_time = clock_time(hour, min) as u16;
        let s = w.actor_spawn(en_tk::ACTOR_EN_TK, pos, [0, 0, 0], 0).expect("En_Tk");
        assert_eq!(!base(&w, s).killed, there, "{hour}:{min:02}");
        w.delete_actor(s);
    }
    // An adult at 19:00: gone.
    w.save.day_time = clock_time(19, 0) as u16;
    w.save.adult = true;
    let s = w.actor_spawn(en_tk::ACTOR_EN_TK, pos, [0, 0, 0], 0).expect("En_Tk");
    assert!(base(&w, s).killed);
    w.delete_actor(s);
    w.save.adult = false;
    // Out of the graveyard (SCENE_KAKARIKO_VILLAGE, 0x52): gone.
    w.scene_id = 0x52;
    let s = w.actor_spawn(en_tk::ACTOR_EN_TK, pos, [0, 0, 0], 0).expect("En_Tk");
    assert!(base(&w, s).killed);
    w.delete_actor(s);
    w.scene_id = en_tk::SCENE_GRAVEYARD;

    // Entered at those times: the same.
    for (hour, min, there) in [(18, 1, true), (20, 59, true), (21, 0, false), (18, 0, false)] {
        let w = enter(&a, false, hour, min, 0, |_| {});
        assert_eq!(all::<EnTk>(&w).len(), there as usize, "entered at {hour}:{min:02}");
    }
    let w = enter(&a, true, 19, 0, 0, |_| {});
    assert!(all::<EnTk>(&w).is_empty(), "the adult's evening");
}

#[test]
fn dampe_walks_his_round_stops_before_link_on_a_spot_and_digs_there() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, false, 19, 0, 0, |_| {});
    let h = dampe(&w);
    w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    // Link far off (the entrance at (-1411, 0, 330)). EnTk_Rest: actionCountdown starts at 0, so
    // DECR gives 0 on his first update: EnTk_WalkAnim (gDampeWalkAnim, actionCountdown
    // Rand_S16Offset(240, 240)) and EnTk_Walk.
    idle(&mut w, 1);
    let t = tk(&w);
    assert_eq!(t.action, Action::Walk);
    assert!(t.skel.is("gDampeWalkAnim"));
    assert!((200..480).contains(&t.action_countdown), "{}", t.action_countdown);
    // EnTk_Orient: towards path 0's first point (-476, 61, 300), south of him: his yaw already
    // about 0x8000. EnTk_Step: no speed outside frames 10 to 21 and 36 to 47 of the walk.
    let mut moved = 0.0;
    let mut prev = base(&w, h).world_pos;
    for _ in 0..200 {
        idle(&mut w, 1);
        let t = tk(&w);
        if t.action != Action::Walk {
            break;
        }
        let f = t.skel.cur_frame;
        let stepping = (10.0..22.0).contains(&f) || (36.0..48.0).contains(&f);
        if !stepping {
            assert_eq!(t.actor.speed_xz, 0.0, "frame {f}");
        }
        assert!(t.actor.speed_xz <= 2.0);
        let p = t.actor.world_pos;
        moved += prev.z - p.z;
        prev = p;
    }
    println!("walked {moved} south");
    assert!(moved > 20.0, "he walks south along path 0: {moved}");

    // Link on his path 80 south of the spot at (-473, 60, 202), facing him: he walks on until
    // Link is within 100 in front of him (EnTk_CheckFacingPlayer), so about 20 before the spot,
    // and rests there (EnTk_RestAnim) as long as Link stays.
    let mut p = Vec3::new(-474.0, 0.0, 202.0 - 80.0);
    p.y = w.col.entity_raycast_down(p + Vec3::Y * 200.0).0;
    w.place_player(p, 0);
    let mut n = 0;
    loop {
        idle(&mut w, 1);
        n += 1;
        let t = tk(&w);
        if t.action == Action::Rest && t.actor.xyz_dist_to_player_sq < 100.0 * 100.0 {
            break;
        }
        // (A rest of his own on the way: 60 to 119 frames.)
        assert!(n < 3000, "he never reached Link: at {:?}", t.actor.world_pos);
    }
    let t = tk(&w);
    let s = base(&w, spot(&w, SPOTS[0])).world_pos;
    let d = Vec3::new(t.actor.world_pos.x - s.x, 0.0, t.actor.world_pos.z - s.z).length();
    println!("stopped after {n} frames at {:?}, {d} from the spot", t.actor.world_pos);
    assert!(d < 40.0, "stopped on the spot: {d}");
    // He stays while Link stands there (actionCountdown kept at 0 by EnTk_Rest's facing branch).
    idle(&mut w, 150);
    assert_eq!(tk(&w).action, Action::Rest);
    assert_eq!(tk(&w).action_countdown, 0);

    // Link steps up to 50 in front, talks and pays: he digs where he stands, on the spot.
    w.save.rupees = 20;
    let texts = talk_to_dampe(&mut w, 0);
    assert_eq!(texts, vec![0x5018, 0x5019]);
    idle(&mut w, 1);
    let t = tk(&w);
    assert_eq!((t.action, t.valid_dig_here, t.current_spot), (Action::Dig, 1, Some(spot(&w, SPOTS[0]))));
    let new = dig_to_frame_42(&mut w);
    // A prize of this round (EnTk_ChooseReward by the play state's Rand_ZeroOne), dropped by
    // its rewardParams.
    let t = tk(&w);
    assert!((0..4).contains(&t.current_reward), "{}", t.current_reward);
    assert_eq!(t.reward_count.iter().map(|&c| c as i32).sum::<i32>(), 1);
    assert_eq!(new.len(), 1);
    let item = base(&w, new[0]).params & 0xFF;
    let want = if t.current_reward == 3 { ITEM00_HEART_PIECE } else { en_tk::REWARD_PARAMS[t.current_reward as usize] };
    println!("dug up reward {} (item {item:#x})", t.current_reward);
    assert_eq!(item, want);
}

#[test]
fn dampes_talk_by_his_flags_and_link_s_rupees() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, false, 19, 0, 0, |_| {});
    move_dampe(&mut w, DAMPE_HOME, 0x8000u16 as i16);
    // EnTk_GetTextId without INFTABLE_D9: 0x5018 ("Hey kid! ...").
    link_in_front(&mut w, 50.0);
    let n = wait_for_offer(&mut w);
    println!("offered after {n} frames");
    assert_eq!(tk(&w).actor.text_id, 0x5018);
    // "Yes" (choice 0) with no rupees: 0x0085 ("You don't have enough Rupees!"), INFTABLE_D9
    // set; at its event, Message_CloseTextbox and NPC_TALK_STATE_IDLE.
    // (0x5018's box goes on to 0x5019, the message's own next, whose choice is answered with
    // the actor's textId still 0x5018.)
    let texts = talk_to_dampe(&mut w, 0);
    assert_eq!(texts, vec![0x5018, 0x5019, 0x0085]);
    assert!(w.save.get_inf_table(en_tk::INFTABLE_D9));
    assert_eq!(w.save.rupees, 0);
    assert_eq!(tk(&w).action, Action::Rest);
    // INFTABLE_D8 only for 0x5028, not his.
    assert!(!w.save.get_inf_table(en_tk::INFTABLE_D8));
    idle(&mut w, 10);

    // With INFTABLE_D9: 0x5019 ("Do you want me to dig here?"); "No" (choice 1): 0x0084.
    let texts = talk_to_dampe(&mut w, 1);
    assert_eq!(texts, vec![0x5019, 0x0084]);
    assert_eq!(tk(&w).action, Action::Rest);
    idle(&mut w, 10);

    // 9 rupees: still not enough (rupees < 10).
    w.save.rupees = 9;
    let texts = talk_to_dampe(&mut w, 0);
    assert_eq!(texts, vec![0x5019, 0x0085]);
    assert_eq!(w.save.rupees, 9);
    idle(&mut w, 10);

    // 10 rupees: MSGMODE_PAUSED, Rupees_ChangeBy(-10) (the accumulator, counted in by the
    // interface), NPC_TALK_STATE_ACTION; the next frame EnTk_DigAnim and EnTk_Dig.
    w.save.rupees = 10;
    let texts = talk_to_dampe(&mut w, 0);
    assert_eq!(texts, vec![0x5019]);
    assert_eq!(tk(&w).interact_info.talk_state, en_tk::NPC_TALK_STATE_ACTION);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_PAUSED);
    // (The interface took the first one that frame: 9 and -9 to go.)
    assert_eq!((w.save.rupees, w.save.rupee_accumulator), (9, -9));
    idle(&mut w, 1);
    let t = tk(&w);
    assert_eq!((t.action, t.interact_info.talk_state), (Action::Dig, en_tk::NPC_TALK_STATE_IDLE));
    assert!(t.skel.is("gDampeDigAnim"));
    idle(&mut w, 30);
    assert_eq!((w.save.rupees, w.save.rupee_accumulator), (0, 0));
}

/// Dampé moved onto `at` (facing north, +z, Link 50 north of him) and paid: the frames to the
/// dig's start (`EnTk_Dig` at its frame 0).
fn paid_dig(w: &mut PlayState, at: Vec3) {
    move_dampe(w, at, 0);
    w.save.rupees = 99;
    let texts = talk_to_dampe(w, 0);
    assert!(matches!(texts.last(), Some(0x5018 | 0x5019)), "{texts:x?}");
    idle(w, 1);
    let t = tk(w);
    assert_eq!(t.action, Action::Dig);
    assert_eq!(t.skel.cur_frame, 0.0);
}

/// The dig on from its frame 0: frames 1 to 31 nothing; frame 32 `NA_SE_EV_DIG_UP` and the prize
/// (if any); frame 42 (`rewardTimer` 10) its sound. The new `En_Item00`s.
fn dig_to_frame_42(w: &mut PlayState) -> Vec<ActorHandle> {
    let items_before = all::<EnItem00>(w);
    idle(w, 31);
    assert_eq!(tk(w).skel.cur_frame, 31.0);
    assert_eq!(all::<EnItem00>(w), items_before, "nothing before frame 32");
    let from = sfx_len(w);
    idle(w, 1);
    assert_eq!(tk(w).skel.cur_frame, en_tk::DIG_REWARD_FRAME);
    assert_eq!(sfx_since(w, from, en_tk::NA_SE_EV_DIG_UP), 1, "NA_SE_EV_DIG_UP at frame 32");
    assert_eq!(tk(w).reward_timer, 1);
    let new: Vec<_> = all::<EnItem00>(w).into_iter().filter(|h| !items_before.contains(h)).collect();
    // EnTk_DigEff: a puff of dirt a frame from frame 32 (each lives 12 frames), at the blade
    // (v3f_304, from his last draw) within 6 across and 4 up or down.
    let t = tk(w);
    let puffs: Vec<_> = t.eff.iter().filter(|e| e.active != 0).collect();
    assert_eq!(puffs.len(), 1);
    assert_eq!((puffs[0].time_left, puffs[0].time_total), (11, 12));
    assert!((puffs[0].pos - t.v3f_304).length() < 10.0);
    assert!((t.v3f_304 - t.actor.world_pos).length() < 80.0, "the blade at {:?}, Dampé at {:?}", t.v3f_304, t.actor.world_pos);
    idle(w, 9);
    assert_eq!(tk(w).skel.cur_frame, 41.0);
    new
}

#[test]
fn a_dig_on_a_spot_digs_up_the_piece_of_heart_first() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, false, 19, 0, 0, |_| {});
    let s = spot(&w, SPOTS[1]);
    let at = base(&w, s).world_pos + Vec3::new(5.0, 1.0, 0.0);
    paid_dig(&mut w, at);
    // EnTk_DigAnim: EnTk_CheckNextSpot finds (-473, 60, 502) within 40 across and not 10 above
    // his floor: validDigHere, currentSpot.
    let t = tk(&w);
    assert_eq!((t.valid_dig_here, t.current_spot), (1, Some(s)));
    // The round's greens, blues and reds used up: EnTk_ChooseReward falls back to the purple
    // (3) whatever its luck; ITEMGETINF_1C not set: the piece of heart (4), ITEMGETINF_1C set.
    tk_mut(&mut w).reward_count = [8, 4, 2, 0];
    assert!(!w.save.get_item_get_inf(en_tk::ITEMGETINF_1C));
    let new = dig_to_frame_42(&mut w);
    assert_eq!(tk(&w).current_reward, 4);
    assert_eq!(tk(&w).reward_count, [8, 4, 2, 1]);
    assert!(w.save.get_item_get_inf(en_tk::ITEMGETINF_1C));
    // Item_DropCollectible(ITEM00_HEART_PIECE) at (0, 0, -40) turned by
    // Matrix_RotateY(shape.rot.y = 0): 40 south (-z) of him.
    assert_eq!(new.len(), 1);
    let item = base(&w, new[0]);
    assert_eq!(item.params & 0xFF, ITEM00_HEART_PIECE);
    let me = tk(&w).actor.world_pos;
    assert_eq!(tk(&w).reward_pos(), me + Vec3::new(0.0, 0.0, -40.0));
    assert!((item.home_pos - (me + Vec3::new(0.0, 0.0, -40.0))).length() < 0.5, "{:?} from {me:?}", item.home_pos);
    // Frame 42 (rewardTimer 10): NA_SE_SY_CORRECT_CHIME, centred.
    let from = sfx_len(&w);
    idle(&mut w, 1);
    assert_eq!(sfx_since(&w, from, NA_SE_SY_CORRECT_CHIME), 1);
    assert_eq!(sfx_since(&w, from, NA_SE_SY_TRE_BOX_APPEAR), 0);
    // To the dig's last frame (ANIMMODE_LOOP's endFrame, animLength - 1): Message_CloseTextbox,
    // EnTk_RestAnim, currentReward -1, validDigHere 0, EnTk_Rest.
    let end = tk(&w).skel.end_frame;
    println!("the dig's last frame {end}");
    idle(&mut w, (end - 42.0) as usize - 1);
    assert_eq!(tk(&w).action, Action::Dig);
    idle(&mut w, 1);
    let t = tk(&w);
    assert_eq!((t.action, t.current_reward, t.valid_dig_here), (Action::Rest, -1, 0));
    assert!(t.skel.is("gDampeRestAnim"));
    assert_ne!(w.msg_ctx.msg_mode, MSGMODE_PAUSED);
    idle(&mut w, 5);
    assert_eq!(w.message_state(), TEXT_STATE_NONE);

    // Again on the same spot without walking off it: EnTk_CheckNextSpot skips the current spot,
    // so nothing (EnTk_CheckCurrentSpot only forgets it while he walks).
    idle(&mut w, 10);
    let at2 = tk(&w).actor.world_pos;
    paid_dig(&mut w, at2);
    assert_eq!(tk(&w).valid_dig_here, 0);
}

#[test]
fn the_grand_prize_after_the_piece_of_heart_is_a_purple_rupee() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, false, 19, 0, 0, |s| s.set_item_get_inf(en_tk::ITEMGETINF_1C));
    let at = base(&w, spot(&w, SPOTS[0])).world_pos;
    paid_dig(&mut w, at);
    assert_eq!(tk(&w).valid_dig_here, 1);
    tk_mut(&mut w).reward_count = [8, 4, 2, 0];
    let new = dig_to_frame_42(&mut w);
    // ITEMGETINF_1C set: the purple rupee (3) stays.
    assert_eq!(tk(&w).current_reward, 3);
    assert_eq!(new.len(), 1);
    assert_eq!(base(&w, new[0]).params & 0xFF, ITEM00_RUPEE_PURPLE);
    let from = sfx_len(&w);
    idle(&mut w, 1);
    // A rupee: NA_SE_SY_TRE_BOX_APPEAR at Dampé.
    assert_eq!(sfx_since(&w, from, NA_SE_SY_TRE_BOX_APPEAR), 1);
    assert_eq!(sfx_since(&w, from, NA_SE_SY_CORRECT_CHIME), 0);
}

#[test]
fn a_dig_off_the_spots_finds_nothing() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, false, 19, 0, 0, |_| {});
    // At his home (-476, 61, 434): the nearest spot (-473, 60, 502) is 68 away.
    paid_dig(&mut w, DAMPE_HOME);
    assert_eq!((tk(&w).valid_dig_here, tk(&w).current_spot), (0, None));
    let new = dig_to_frame_42(&mut w);
    // No prize; currentReward stays -1.
    assert!(new.is_empty());
    assert_eq!(tk(&w).current_reward, -1);
    assert_eq!(tk(&w).reward_count, [0; 4]);
    // Frame 42: NA_SE_SY_ERROR at him.
    let from = sfx_len(&w);
    idle(&mut w, 1);
    assert_eq!(sfx_since(&w, from, NA_SE_SY_ERROR), 1);
    // The dig's end: "Nope, nothing here!" (0x501A), EnTk_Rest.
    let end = tk(&w).skel.end_frame;
    idle(&mut w, (end - 42.0) as usize);
    assert_eq!(tk(&w).action, Action::Rest);
    talk_until(&mut w, 0, 10, "0x501A", |w| w.msg_ctx.text_id == 0x501A && w.message_state() != TEXT_STATE_NONE);
    talk_until(&mut w, 0, 400, "0x501A closed", |w| w.message_state() == TEXT_STATE_NONE && w.msg_ctx.msg_mode == 0);
}

#[test]
fn the_prizes_come_in_rounds_of_eight_four_two_and_one() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, false, 19, 0, 0, |_| {});
    let t = tk_mut(&mut w);
    // EnTk_ChooseReward's luck (Rand_ZeroOne): under 0.4 a green, 0.7 a blue, 0.9 a red, else a
    // purple. A seed whose first draw is 0.9 or more, from empty counts: the purple.
    let seed = (1..10000u32).find(|&s| Rand { state: s }.zero_one() >= 0.9).unwrap();
    let mut r = Rand { state: seed };
    assert_eq!(t.choose_reward(&mut r), 3);
    assert_eq!(t.reward_count, [0, 0, 0, 1]);
    // Again: the purple's used up (1 a round): the cheapest left, a green.
    let mut r = Rand { state: seed };
    assert_eq!(t.choose_reward(&mut r), 0);
    assert_eq!(t.reward_count, [1, 0, 0, 1]);
    // A draw under 0.4 with the greens used up: a blue.
    let low = (1..10000u32).find(|&s| Rand { state: s }.zero_one() < 0.4).unwrap();
    t.reward_count = [8, 0, 0, 0];
    assert_eq!(t.choose_reward(&mut Rand { state: low }), 1);
    // A whole round from empty, whatever the luck: eight greens, four blues, two reds, one
    // purple; the sixteenth starts the next round (counts [1, 0, 0, 0], a green).
    t.reward_count = [0; 4];
    let mut r = Rand { state: 7 };
    let mut got = [0; 4];
    for _ in 0..15 {
        got[t.choose_reward(&mut r) as usize] += 1;
    }
    assert_eq!(got, [8, 4, 2, 1]);
    assert_eq!(t.reward_count, [8, 4, 2, 1]);
    assert_eq!(t.choose_reward(&mut r), 0);
    assert_eq!(t.reward_count, [1, 0, 0, 0]);
    // The dig's drop table (rewardParams).
    assert_eq!(en_tk::REWARD_PARAMS[0], ITEM00_RUPEE_GREEN);
}
