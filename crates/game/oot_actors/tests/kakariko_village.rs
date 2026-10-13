//! GAME-07 milestone 5 against the C: Kakariko's set pieces and the man on the roof. The
//! windmill's sails (`z_bg_spot01_fusya.c`), the well's crossbeam (`z_bg_spot01_idohashira.c`)
//! and water (`z_bg_spot01_idomizu.c`), the posters and the shooting gallery
//! (`z_bg_spot01_objects2.c`), and `En_Ani` (`z_en_ani.c`).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_collision::dyna::BG_ACTOR_MAX;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_spot01_fusya::{self, BgSpot01Fusya};
use oot_actors::bg_spot01_idohashira::{self, BgSpot01Idohashira};
use oot_actors::bg_spot01_idomizu::{self, BgSpot01Idomizu};
use oot_actors::bg_spot01_objects2::{self, BgSpot01Objects2};
use oot_actors::en_ani::{self, EnAni};
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_A3` (`save.h`): Kakariko's intro seen.
const EVENTCHKINF_A3: u16 = 0xA3;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Link (adult or child) entered at `ENTR_KAKARIKO_VILLAGE_0` at `hour`:00, Kakariko's intro
/// seen, `edit` applied to the save last; two frames on (the room's actors and objects in).
fn enter(a: &Arc<GameAssets>, adult: bool, hour: i32, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_KAKARIKO_VILLAGE_0").expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(hour, 0) as u16);
    if !adult {
        save.apply_preset("deku-tree-dead").unwrap();
    }
    save.set_event_chk_inf(EVENTCHKINF_A3);
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 2);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn all<T: 'static + ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn one<T: 'static + ActorImpl>(w: &PlayState) -> ActorHandle {
    let v = all::<T>(w);
    assert_eq!(v.len(), 1, "one {}", std::any::type_name::<T>());
    v[0]
}

fn get<T: 'static + ActorImpl>(w: &PlayState, h: ActorHandle) -> &T {
    w.actors.downcast::<T>(h).unwrap()
}

fn base(w: &PlayState, h: ActorHandle) -> &Actor {
    w.actors.actor(h).unwrap()
}

/// One frame with `pad`, then one released.
fn press(w: &mut PlayState, pad: PadState) {
    w.tick_with(scripted_input(PadState::default(), pad));
    w.tick_with(scripted_input(pad, PadState::default()));
}

fn a_button() -> PadState {
    PadState { button: BTN_A, ..Default::default() }
}

/// Runs `h`'s update once outside the frame (`play.cur_actor` unset).
fn update_once(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).unwrap();
    a.update(w);
    w.actors.put_back(h, a);
}

#[test]
fn the_windmills_sails_turn_100_a_frame_then_up_to_1800() {
    let Some(a) = assets() else { return };
    // BgSpot01Fusya_Init outside a cutscene layer clears EVENTCHKINF_65: set before, it's gone.
    let mut w = enter(&a, false, 10, |s| s.set_event_chk_inf(bg_spot01_fusya::EVENTCHKINF_65));
    assert!(!w.save.get_event_chk_inf(bg_spot01_fusya::EVENTCHKINF_65));
    let h = one::<BgSpot01Fusya>(&w);
    // sInitChain's scale 0.1; unk_154 = unk_158 = 100, unk_15C 0.5.
    assert_eq!(base(&w, h).scale, Vec3::splat(0.1));
    let turn = |w: &mut PlayState| {
        let before = base(w, h).shape_rot.z;
        idle(w, 1);
        base(w, h).shape_rot.z.wrapping_sub(before)
    };
    // func_808AAA50: shape.rot.z += unk_154 every frame (100 while unk_158 is 100).
    for _ in 0..5 {
        assert_eq!(turn(&mut w), 100);
    }
    // EVENTCHKINF_65: unk_158 1800; Math_ApproachF(unk_154, 1800, 0.5, 100) adds
    // min((1800 - unk_154) / 2, 100): 100, 200, .., 1700 (17 frames), then 1750, 1775, 1787.5.
    w.save.set_event_chk_inf(bg_spot01_fusya::EVENTCHKINF_65);
    for i in 1..=17 {
        assert_eq!(turn(&mut w), 100 * i, "frame {i}");
    }
    assert_eq!(turn(&mut w), 1750);
    assert_eq!(turn(&mut w), 1775);
    let f = get::<BgSpot01Fusya>(&w, h);
    assert_eq!((f.unk_154, f.unk_158), (1787.5, 1800.0));
}

#[test]
fn the_well_water_stands_at_52_and_drains_2_a_frame() {
    let Some(a) = assets() else { return };
    // A child, not drained: waterHeight 52, world.pos.y 52, waterBoxes[0] at 52.
    let mut w = enter(&a, false, 10, |_| {});
    let h = one::<BgSpot01Idomizu>(&w);
    // It updates only in view (no ACTOR_FLAG_UPDATE_CULLING_DISABLED); the well isn't from the
    // entrance.
    w.actors.actor_mut(h).unwrap().flags |= oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    assert_eq!(get::<BgSpot01Idomizu>(&w, h).water_height, 52.0);
    assert_eq!(base(&w, h).world_pos.y, 52.0);
    assert_eq!(w.col.water_box_surface(bg_spot01_idomizu::WATER_BOX), Some(52));
    // EVENTCHKINF_DRAINED_WELL: waterHeight -550; each frame the box takes the height first, then
    // Math_ApproachF(y, -550, 1, 2) lowers it 2.
    w.save.set_event_chk_inf(bg_spot01_idomizu::EVENTCHKINF_DRAINED_WELL);
    idle(&mut w, 1);
    assert_eq!(base(&w, h).world_pos.y, 50.0);
    assert_eq!(w.col.water_box_surface(bg_spot01_idomizu::WATER_BOX), Some(52));
    idle(&mut w, 10);
    assert_eq!(base(&w, h).world_pos.y, 30.0);
    assert_eq!(w.col.water_box_surface(bg_spot01_idomizu::WATER_BOX), Some(32));
    // Drained already: -550 from the start.
    let w = enter(&a, false, 10, |s| s.set_event_chk_inf(bg_spot01_idomizu::EVENTCHKINF_DRAINED_WELL));
    let h = one::<BgSpot01Idomizu>(&w);
    assert_eq!(base(&w, h).world_pos.y, -550.0);
    // An adult (LINK_AGE_IN_YEARS == YEARS_ADULT): -550, drained or not.
    let mut w = enter(&a, true, 10, |_| {});
    let h = one::<BgSpot01Idomizu>(&w);
    assert_eq!(base(&w, h).world_pos.y, -550.0);
    w.actors.actor_mut(h).unwrap().flags |= oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    idle(&mut w, 1);
    assert_eq!(w.col.water_box_surface(bg_spot01_idomizu::WATER_BOX), Some(-550));
}

#[test]
fn the_wells_crossbeam_stands_until_the_nocturne_for_an_adult() {
    let Some(a) = assets() else { return };
    // Outside a cutscene layer: action 0, its collision registered (gKakarikoWellArchCol).
    let w = enter(&a, false, 10, |s| s.set_event_chk_inf(bg_spot01_idohashira::EVENTCHKINF_54));
    let h = one::<BgSpot01Idohashira>(&w);
    let b = get::<BgSpot01Idohashira>(&w, h);
    assert_eq!(b.action, bg_spot01_idohashira::ACTION_STILL, "a child keeps it with EVENTCHKINF_54");
    assert_ne!(b.bg, BG_ACTOR_MAX);
    let w = enter(&a, true, 10, |_| {});
    assert_eq!(all::<BgSpot01Idohashira>(&w).len(), 1, "an adult before the Nocturne");
    // EVENTCHKINF_54 && LINK_IS_ADULT: Actor_Kill.
    let w = enter(&a, true, 10, |s| s.set_event_chk_inf(bg_spot01_idohashira::EVENTCHKINF_54));
    assert!(all::<BgSpot01Idohashira>(&w).is_empty());
}

#[test]
fn the_crossbeam_in_the_cutscene_layers() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, false, 10, |_| {});
    let placed = base(&w, one::<BgSpot01Idohashira>(&w)).home_pos;
    let spawn = |w: &mut PlayState, layer: usize| {
        w.save.scene_layer = layer;
        w.actor_spawn(bg_spot01_idohashira::ACTOR_BG_SPOT01_IDOHASHIRA, placed, [0, -10923, 0], -1).unwrap()
    };
    // Layer 4: action 1 (the cues), shape.yOffset -(kREG(10) + 1100).
    let h = spawn(&mut w, 4);
    let b = get::<BgSpot01Idohashira>(&w, h);
    assert_eq!((b.action, b.actor.shape_y_offset), (bg_spot01_idohashira::ACTION_CUES, -1100.0));
    // Layer 6: action 0. Layer 5 (any other cutscene layer): Actor_Kill.
    let h = spawn(&mut w, 6);
    assert_eq!(get::<BgSpot01Idohashira>(&w, h).action, bg_spot01_idohashira::ACTION_STILL);
    let h = spawn(&mut w, 5);
    assert!(base(&w, h).killed);
}

#[test]
fn the_crossbeam_breaks_off_and_falls_along_its_cue() {
    use oot_game::cutscene::CsCmdActorCue;
    let Some(a) = assets() else { return };
    let mut w = enter(&a, false, 10, |_| {});
    w.save.scene_layer = 4;
    let home = Vec3::new(799.0, 190.0, 503.0);
    let h = w.actor_spawn(bg_spot01_idohashira::ACTOR_BG_SPOT01_IDOHASHIRA, home, [0, -10923, 0], -1).unwrap();
    // A script running (CS_STATE_RUN) with cue 2 on channel 2 from frame 100 to 120, ending at
    // (900, 0, 600).
    w.cs_ctx.state = 2;
    let cue = |id: u16| CsCmdActorCue { action: id, start_frame: 100, end_frame: 120, rot: [0; 3], start_pos: glam::IVec3::ZERO, end_pos: glam::IVec3::new(900, 0, 600), normal: glam::IVec3::ZERO };
    w.cs_ctx.npc_actions[2] = Some(cue(2));
    w.cs_ctx.frames = 100;
    // func_808AB444: cue 2, func_808AB3F8: action 2, unk_170 1 (the break's dust at the next draw).
    update_once(&mut w, h);
    let b = get::<BgSpot01Idohashira>(&w, h);
    assert_eq!((b.action, b.cue_id, b.unk_170), (bg_spot01_idohashira::ACTION_FALL, 2, 1));
    // func_808AB530 at frame 110 (t 10 of T 20): the spin (+0x3E8 y, +0x7D0 z); x and z half way
    // (Environment_LerpWeight 0.5); y: func_808AB1DC(190, 110 + 0, 120, 100, 110) =
    // ((110 - 190 - 300) / 400) * 100 + 300 = 205, plus 190.
    w.cs_ctx.frames = 110;
    update_once(&mut w, h);
    let b = get::<BgSpot01Idohashira>(&w, h);
    assert_eq!((b.actor.shape_rot.y, b.actor.shape_rot.z), ((-10923i16).wrapping_add(0x3E8), 0x7D0));
    assert_eq!(b.actor.world_pos, Vec3::new(849.5, 395.0, 551.5));
    // At frame 120: the cue's end, 110 up (drawn 110 down): func_808AB414, action 3.
    w.cs_ctx.frames = 120;
    update_once(&mut w, h);
    let b = get::<BgSpot01Idohashira>(&w, h);
    assert_eq!(b.actor.world_pos, Vec3::new(900.0, 110.0, 600.0));
    assert_eq!(b.action, bg_spot01_idohashira::ACTION_LANDED);
    // Cue 3: Actor_Kill.
    w.cs_ctx.npc_actions[2] = Some(cue(3));
    update_once(&mut w, h);
    assert!(base(&w, h).killed);
}

#[test]
fn the_unfinished_gallery_has_its_collision_and_a_carpenter_by_day() {
    let Some(a) = assets() else { return };
    // Layer 0 (a child by day): Bg_Spot01_Objects2 0x0203 at (324, 160, 1000): type 3
    // (object_spot01_matoyab), path 2. Once the object is in: object_spot01_matoyab_col and, by
    // day, En_Daiku_Kakariko as its child at path 2's first point (497, 300, 831), its rotation
    // (0, 0, 0), params (2 << 8) + 1.
    let w = enter(&a, false, 10, |_| {});
    let h = one::<BgSpot01Objects2>(&w);
    let o = get::<BgSpot01Objects2>(&w, h);
    assert_eq!((o.ty(), o.path()), (bg_spot01_objects2::TYPE_SHOOTING_GALLERY_UNFINISHED, 2));
    assert_eq!(o.action, bg_spot01_objects2::Action::Idle);
    assert!(o.drawn);
    assert_ne!(o.bg, BG_ACTOR_MAX, "its collision");
    let c = o.actor.child.expect("the carpenter");
    let d = base(&w, c);
    assert_eq!(d.id, bg_spot01_objects2::ACTOR_EN_DAIKU_KAKARIKO);
    assert_eq!(d.home_pos, Vec3::new(497.0, 300.0, 831.0));
    assert_eq!((d.home_rot.x, d.home_rot.y, d.home_rot.z), (0, 0, 0));
    assert_eq!(d.params, 0x0201);
    assert_eq!(d.parent, Some(h));
    // Layer 1 (by night): 0x0003, path 0: the collision, no carpenter.
    let w = enter(&a, false, 22, |_| {});
    let h = one::<BgSpot01Objects2>(&w);
    let o = get::<BgSpot01Objects2>(&w, h);
    assert_eq!(o.ty(), bg_spot01_objects2::TYPE_SHOOTING_GALLERY_UNFINISHED);
    assert_ne!(o.bg, BG_ACTOR_MAX);
    assert!(o.drawn);
    assert_eq!(o.actor.child, None, "no carpenter by night");
}

#[test]
fn an_adults_posters_and_gallery() {
    let Some(a) = assets() else { return };
    // Layer 2 (an adult by day): the posters 0x0000, 0x0001, 0x0002 (no collision) and the
    // gallery 0xFF04 (gKakarikoShootingGalleryCol), all of object_spot01_matoya.
    let w = enter(&a, true, 10, |_| {});
    let mut v: Vec<_> = all::<BgSpot01Objects2>(&w).into_iter().map(|h| get::<BgSpot01Objects2>(&w, h)).map(|o| (o.ty(), o.bg != BG_ACTOR_MAX, o.drawn, o.actor.child.is_some())).collect();
    v.sort();
    assert_eq!(v, vec![(0, false, true, false), (1, false, true, false), (2, false, true, false), (4, true, true, false)]);
}

/// The roof man, `always_update`d.
fn ani(w: &mut PlayState) -> ActorHandle {
    let h = one::<EnAni>(w);
    w.actors.actor_mut(h).unwrap().flags |= oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    h
}

/// Link `dist` from the roof man at `yaw_diff` from his facing, at height `y`, facing him; a
/// frame on.
fn near_ani(w: &mut PlayState, h: ActorHandle, yaw_diff: i16, dist: f32, y: f32) {
    let a = base(w, h);
    let yaw = a.shape_rot.y.wrapping_add(yaw_diff);
    let p = a.world_pos + Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw)) * dist;
    w.place_player(Vec3::new(p.x, y, p.z), yaw.wrapping_add(-0x8000));
    idle(w, 1);
}

#[test]
fn the_roof_mans_texts_by_day_and_night() {
    let Some(a) = assets() else { return };
    // EnAni_Init for a child: func_809B064C. Link below him (no y limit for this text), 200 off,
    // 0x1000 off his facing (in [-0x3E7, 0x36B0)): EnAni_SetText(0x5050) by day.
    let mut w = enter(&a, false, 10, |_| {});
    let h = ani(&mut w);
    assert_eq!(get::<EnAni>(&w, h).action, en_ani::Action::ChildIdle);
    let y = base(&w, h).world_pos.y - 200.0;
    near_ani(&mut w, h, 0x1000, 200.0, y);
    assert_eq!(base(&w, h).text_id, en_ani::TEXT_CHILD_DAY);
    assert_eq!(get::<EnAni>(&w, h).unk_2a8 & 1, 1, "tracking from now on");
    // !IS_DAY: 0x5051.
    let mut w = enter(&a, false, 22, |_| {});
    let h = ani(&mut w);
    let y = base(&w, h).world_pos.y - 200.0;
    near_ani(&mut w, h, 0x1000, 200.0, y);
    assert_eq!(base(&w, h).text_id, en_ani::TEXT_CHILD_NIGHT);
    // Out of 350 or past 0x36B0: nothing offered.
    let mut w = enter(&a, false, 10, |_| {});
    let h = ani(&mut w);
    near_ani(&mut w, h, 0x1000, 400.0, y);
    assert_eq!(base(&w, h).text_id, 0);
    near_ani(&mut w, h, 0x3800, 200.0, y);
    assert_eq!(base(&w, h).text_id, 0);
}

#[test]
fn an_adult_hears_the_roof_man_by_the_eruption_and_the_heart_piece() {
    let Some(a) = assets() else { return };
    // func_809B07F8: 0x5052 until EVENTCHKINF_2F; then 0x5053, or 0x5054 with ITEMGETINF_15.
    let text = |edit: fn(&mut SaveContext)| {
        let mut w = enter(&a, true, 10, edit);
        let h = ani(&mut w);
        assert_eq!(get::<EnAni>(&w, h).action, en_ani::Action::AdultIdle);
        let y = base(&w, h).world_pos.y - 200.0;
        near_ani(&mut w, h, 0x1000, 200.0, y);
        base(&w, h).text_id
    };
    assert_eq!(text(|_| {}), en_ani::TEXT_ADULT);
    assert_eq!(text(|s| s.set_event_chk_inf(en_ani::EVENTCHKINF_2F)), en_ani::TEXT_ADULT_2F);
    assert_eq!(
        text(|s| {
            s.set_event_chk_inf(en_ani::EVENTCHKINF_2F);
            s.set_item_get_inf(en_ani::ITEMGETINF_15);
        }),
        en_ani::TEXT_ADULT_GIVEN
    );
}

#[test]
fn the_roof_man_gives_a_piece_of_heart_on_the_roof() {
    use oot_game::message::{TEXT_STATE_AWAITING_NEXT, TEXT_STATE_DONE, TEXT_STATE_NONE};
    let Some(a) = assets() else { return };
    // func_809B064C: Link on the roof by him, yawDiff in [-0x36AF, 0) (-0x2000), within 150 (80),
    // not 80 below him (the roof there is about 450, he is at 483): EnAni_SetText(0x5055), offered
    // within 100.
    let mut w = enter(&a, false, 10, |_| {});
    let h = ani(&mut w);
    let ap = base(&w, h).world_pos;
    let yaw = base(&w, h).shape_rot.y.wrapping_add(-0x2000);
    let spot = ap + Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw)) * 80.0;
    let (floor, _) = w.col.entity_raycast_down(Vec3::new(spot.x, ap.y + 60.0, spot.z));
    assert!(floor > ap.y - 80.0, "the roof at {floor}");
    near_ani(&mut w, h, -0x2000, 80.0, floor);
    assert!(base(&w, h).y_dist_to_player > -80.0);
    assert_eq!(base(&w, h).text_id, en_ani::TEXT_HEART_PIECE);
    assert_eq!(w.player().target_actor, Some(h));
    // A: Actor_TalkOfferAccepted with 0x5055: func_809B05F0, which offers GI_HEART_PIECE every
    // frame; on the box's closing frame func_809B0558, until Player takes it (Actor_HasParent):
    // ITEMGETINF_15, func_809B04F0 (the item's box), then func_809B064C again.
    press(&mut w, a_button());
    let mut prev = PadState::default();
    let mut n = 0;
    let mut saw_text = false;
    while !w.save.get_item_get_inf(en_ani::ITEMGETINF_15) {
        if get::<EnAni>(&w, h).action == en_ani::Action::HeartPieceText {
            saw_text = true;
        }
        let waiting = matches!(w.message_state(), TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE);
        let pad = if waiting && prev.button == 0 { a_button() } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        n += 1;
        assert!(n < 1000, "never given: {:?}, text state {}", get::<EnAni>(&w, h).action, w.message_state());
    }
    assert!(saw_text, "0x5055 read first");
    assert_eq!(get::<EnAni>(&w, h).action, en_ani::Action::ChildTalk);
    assert_eq!(base(&w, h).parent, None);
    // The item's box read through: back to func_809B064C, and the same place gets 0x5056.
    n = 0;
    while w.message_state() != TEXT_STATE_NONE || get::<EnAni>(&w, h).action != en_ani::Action::ChildIdle {
        let waiting = matches!(w.message_state(), TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE);
        let pad = if waiting && prev.button == 0 { a_button() } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        n += 1;
        assert!(n < 1000, "the item's box never closed");
    }
    idle(&mut w, 30);
    near_ani(&mut w, h, -0x2000, 80.0, floor);
    assert_eq!(base(&w, h).text_id, en_ani::TEXT_HEART_PIECE_GIVEN);
}
