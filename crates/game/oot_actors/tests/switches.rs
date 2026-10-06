//! The switches (`Obj_Switch`, `z_obj_switch.c`) against the C, in the Deku Tree (MQ): room 0's
//! top-floor switch (params 0x2700: floor, once, flag 0x27) at (-311, 800, -311), room 5's held
//! switch (0x3E20: floor, held, flag 0x3E) at (-527, -880, 1114), and room 3's eye switch (0x1502:
//! eye, once, flag 0x15) at (-76, -727, 551) facing -z (rot y -32768). The Deku Tree places no
//! toggle floor switch and no crystal: the tests spawn them in room 0.
//!
//! Expected values are worked out from the C in the comments. The eye takes only seeds and arrows,
//! which Link can't shoot yet: its hit is injected as `CollisionCheck_SetATvsAC` leaves it, as the
//! other tests inject Deku Nuts.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::obj_switch::{self, Action, CrystalTex, ObjSwitch, objswitch_params};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_SWITCH, ActorHandle, ActorImpl};
use oot_game::audio::sfx::*;
use oot_game::collision_check as cc;
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

fn tick(w: &mut PlayState, p: PadState, prev: &mut PadState) {
    w.tick_with(scripted_input(*prev, p));
    *prev = p;
}

const HOME0: Vec3 = Vec3::new(-311.0, 800.0, -311.0);
const HOME5: Vec3 = Vec3::new(-527.0, -880.0, 1114.0);
const HOME3_EYE: Vec3 = Vec3::new(-76.0, -727.0, 551.0);

/// Inside the Deku Tree (`deku-tree-inside`), the sound log on, 60 frames in, then in `room`
/// (changed to unless it's room 0, the entrance's).
fn deku_tree(a: &Arc<GameAssets>, room: i8) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 60);
    if room != 0 {
        assert!(w.room_request(room));
        idle(&mut w, 1);
        w.room_change_done();
    }
    w
}

/// The switch with `params`.
fn switch_with(w: &PlayState, params: i16) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<ObjSwitch>(h).is_some_and(|s| s.actor.params == params)).unwrap_or_else(|| panic!("no Obj_Switch {params:#06x}"))
}

fn sw(w: &PlayState, h: ActorHandle) -> &ObjSwitch {
    w.actors.downcast::<ObjSwitch>(h).expect("Obj_Switch")
}

fn sw_mut(w: &mut PlayState, h: ActorHandle) -> &mut ObjSwitch {
    w.actors.downcast_mut::<ObjSwitch>(h).expect("Obj_Switch")
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

fn sfx_frames(w: &PlayState, id: u16, after: u32) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(f, s, _)| f > after && s == id).map(|&(f, _, _)| f).collect()
}

/// The floor straight down from 100 above `p`.
fn floor_at(w: &PlayState, p: Vec3) -> f32 {
    w.col.entity_raycast_down(Vec3::new(p.x, p.y + 100.0, p.z)).0
}

/// Link put at `pos` facing `yaw` between frames, as if he'd walked there, with no attention
/// camera running (`place_player` resets the cameras).
fn move_link(w: &mut PlayState, pos: Vec3, yaw: i16) {
    assert_eq!(w.active_cam_id, oot_game::camera::CAM_ID_MAIN, "a one-point camera is running");
    w.place_player(pos, yaw);
}

/// The active camera is the attention cutscene (`OnePointCutscene_Attention`, 5010) on switch
/// `h`, with `sfx` for `Camera_Demo5` to play (`OnePointCutscene_AttentionSetSfx`'s `data1`).
fn attention_on(w: &PlayState, h: ActorHandle, sfx: u16) -> bool {
    w.camera(w.active_cam_id).is_some_and(|c| c.cs_id == 5010 && c.target == Some(h) && c.data1 == sfx)
}

/// Whether this frame's wait is over in `ObjSwitch_FloorPress` and the others: the camera looks
/// at the switch category (`func_8005B198`, read before the frame: the cameras update after the
/// actors) or the cooldown is out, or there's none.
fn cooldown_over(w: &PlayState, h: ActorHandle) -> bool {
    let s = sw(w, h);
    // The update counts cooldownTimer down first.
    let t = if s.cooldown_timer > 0 { s.cooldown_timer - 1 } else { 0 };
    !s.cooldown_on || w.func_8005b198() == ACTORCAT_SWITCH as i32 || t <= 0
}

/// Frames of `ObjSwitch_FloorPress` until the switch is down, each checked against the C: the
/// cooldown counted down; nothing until the wait is over, then down by 99 / 2000 a frame; at 33
/// / 2000 or under, `ObjSwitch_FloorDownInit` (33 / 2000, releaseTimer 6) and
/// NA_SE_EV_FOOT_SWITCH. Returns the frame it got down, and how many it took.
fn until_down(w: &mut PlayState, h: ActorHandle) -> (u32, u32) {
    let mut n = 0;
    loop {
        let s = sw(w, h);
        assert_eq!(s.action, Action::FloorPress);
        let (y0, t0) = (s.actor.scale.y, s.cooldown_timer);
        let moves = cooldown_over(w, h);
        idle(w, 1);
        n += 1;
        let s = sw(w, h);
        assert_eq!(s.cooldown_timer, (t0 - 1).max(0), "frame {n}");
        let mut y = y0;
        if moves {
            y -= obj_switch::FLOOR_SCALE_STEP;
        }
        let f = w.audio.frames;
        if moves && y <= obj_switch::FLOOR_DOWN_SCALE_Y {
            assert_eq!((s.action, s.actor.scale.y, s.release_timer), (Action::FloorDown, obj_switch::FLOOR_DOWN_SCALE_Y, 6), "frame {n}");
            assert!(sfx_on(w, f, obj_switch::NA_SE_EV_FOOT_SWITCH));
            return (f, n);
        }
        assert_eq!(s.actor.scale.y, y, "frame {n}");
        assert!(!sfx_on(w, f, obj_switch::NA_SE_EV_FOOT_SWITCH));
        assert!(n < 120, "never down");
    }
}

/// Frames of `ObjSwitch_FloorRelease` until the switch is up: up by 99 / 2000 a frame (a toggle
/// once its wait is over); at 33 / 200 or over, `ObjSwitch_FloorUpInit` and NA_SE_EV_FOOT_SWITCH.
fn until_up(w: &mut PlayState, h: ActorHandle) -> (u32, u32) {
    let mut n = 0;
    loop {
        let s = sw(w, h);
        assert_eq!(s.action, Action::FloorRelease);
        let sub = (s.actor.params >> 4) & 7;
        let y0 = s.actor.scale.y;
        let moves = (sub != obj_switch::OBJSWITCH_SUBTYPE_TOGGLE && sub != obj_switch::OBJSWITCH_SUBTYPE_HOLD_INVERTED) || cooldown_over(w, h);
        idle(w, 1);
        n += 1;
        let s = sw(w, h);
        let mut y = y0;
        if moves {
            y += obj_switch::FLOOR_SCALE_STEP;
        }
        let f = w.audio.frames;
        if moves && y >= obj_switch::FLOOR_UP_SCALE_Y {
            assert_eq!((s.action, s.actor.scale.y), (Action::FloorUp, obj_switch::FLOOR_UP_SCALE_Y), "frame {n}");
            assert!(sfx_on(w, f, obj_switch::NA_SE_EV_FOOT_SWITCH));
            return (f, n);
        }
        assert_eq!(s.actor.scale.y, y, "frame {n}");
        assert!(n < 120, "never up");
    }
}

#[test]
fn init_the_floor_and_eye_switches() {
    let Some(a) = assets() else { return };
    let w = deku_tree(&a, 0);
    let h = switch_with(&w, 0x2700);
    let s = sw(&w, h);
    // ObjSwitch_Init (floor): its bg actor (gFloorSwitchCol, DYNA_TRANSFORM_POS); scale 0.1
    // (ICHAIN_VEC3F_DIV1000(scale, 100)), then ObjSwitch_FloorUpInit's 33 / 200 high; 1 above home;
    // the focus 10 up (sFocusHeights); MASS_IMMOVABLE; switch category, culling disabled.
    assert_eq!((s.actor.home_pos, s.actor.shape_rot.y), (HOME0, -8192));
    assert!(s.bg < eng_collision::dyna::BG_ACTOR_MAX);
    assert_eq!(s.dyna_bg_id(), Some(s.bg));
    assert_eq!(w.col.dyna.actors[s.bg as usize].move_flags, eng_collision::dyna::DYNA_TRANSFORM_POS);
    assert_eq!(s.actor.scale, Vec3::new(0.1, 0.165, 0.1));
    assert_eq!(s.actor.world_pos, HOME0 + Vec3::Y);
    assert_eq!(s.actor.focus_pos, HOME0 + Vec3::Y * 11.0);
    assert_eq!((s.action, s.actor.col_chk_info.mass, s.actor.category), (Action::FloorUp, cc::MASS_IMMOVABLE, ACTORCAT_SWITCH));
    assert_ne!(s.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    // DynaPoly takes its scale: the switch's top is 33 / 200 of the collision's height above it.
    let top = floor_at(&w, HOME0);
    assert!(top > HOME0.y + 1.0, "{top}");

    // Room 3's eye: no bg actor; its focus at its position (sFocusHeights 0); open (eyeTexIndex
    // 0); its two triangles (sEyeTrisElementsInit) turned by home.rot.y (ObjSwitch_RotateY: x' =
    // z sin + x cos, z' = z cos - x sin) about its position, taking 0x0001F824 (the slingshot,
    // the arrows), the first ELEM_MATERIAL_UNK4.
    let w = deku_tree(&a, 3);
    let h = switch_with(&w, 0x1502);
    let s = sw(&w, h);
    assert_eq!((s.actor.home_pos, s.actor.shape_rot.y), (HOME3_EYE, -32768));
    assert_eq!((s.bg, s.dyna_bg_id()), (eng_collision::dyna::BG_ACTOR_MAX, None));
    assert_eq!((s.actor.scale, s.actor.focus_pos), (Vec3::splat(0.1), HOME3_EYE));
    assert_eq!((s.action, s.eye_tex_index), (Action::EyeOpen, 0));
    let (sn, cs) = (sin_s(-32768), cos_s(-32768));
    let rot = |v: Vec3| Vec3::new(v.z * sn + v.x * cs, v.y, v.z * cs - v.x * sn) + HOME3_EYE;
    let model = [[Vec3::new(0.0, 23.0, 8.5), Vec3::new(-23.0, 0.0, 8.5), Vec3::new(0.0, -23.0, 8.5)], [Vec3::new(0.0, 23.0, 8.5), Vec3::new(0.0, -23.0, 8.5), Vec3::new(23.0, 0.0, 8.5)]];
    assert_eq!(s.tris.elements.len(), 2);
    for (e, m) in s.tris.elements.iter().zip(model) {
        assert_eq!(e.dim.vtx, m.map(rot));
        assert_eq!(e.info.ac_dmg_info.dmg_flags, 0x0001_F824);
        assert_eq!(e.info.ac_elem_flags & !cc::ACELEM_HIT, cc::ACELEM_ON);
    }
    assert_eq!((s.tris.elements[0].info.elem_material, s.tris.elements[1].info.elem_material), (cc::ELEM_MATERIAL_UNK4, cc::ELEM_MATERIAL_UNK0));
    // sEyeTrisInit: AC on, by Player's attacks.
    assert_eq!(s.tris.base.ac_flags & !cc::AC_HIT, cc::AC_ON | cc::AC_TYPE_PLAYER);
    // Room 3's two floor switches start up too.
    for p in [0x0200, 0x1400] {
        assert_eq!(sw(&w, switch_with(&w, p)).action, Action::FloorUp);
    }
}

#[test]
fn link_walks_onto_room_0s_switch_and_it_stays_down() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 0);
    let h = switch_with(&w, 0x2700);
    let top_up = floor_at(&w, HOME0);
    // Link 80 off along the diagonal towards the room's middle, on the top floor, facing the
    // switch (yaw -0x6000: towards -x, -z).
    let start = Vec3::new(HOME0.x + 56.0, 0.0, HOME0.z + 56.0);
    let y = floor_at(&w, Vec3::new(start.x, HOME0.y, start.z));
    assert!((y - HOME0.y).abs() < 1.0, "the top floor: {y}");
    w.place_player(Vec3::new(start.x, y, start.z), -0x6000);
    idle(&mut w, 3);
    assert_eq!(sw(&w, h).action, Action::FloorUp);
    assert!(!w.flags.get_switch(0x27));
    // Walking on: ObjSwitch_FloorUp (once) waits for DynaPolyActor_IsPlayerOnTop, which Player's
    // last update set (DynaPoly_SetPlayerOnTop: his floor is the switch's).
    let mut prev = PadState::default();
    let mut n = 0;
    while sw(&w, h).action == Action::FloorUp {
        tick(&mut w, stick(0, 60), &mut prev);
        n += 1;
        assert!(n < 60, "never stepped on it");
    }
    let pressed = w.audio.frames;
    // ObjSwitch_FloorPressInit (cooldownTimer 100) and ObjSwitch_SetOn: flag 0x27 set, the
    // attention camera on the switch with NA_SE_SY_CORRECT_CHIME (once), cooldownOn.
    let s = sw(&w, h);
    assert_eq!((s.action, s.cooldown_timer, s.cooldown_on), (Action::FloorPress, 100, true));
    assert_eq!(s.actor.scale.y, obj_switch::FLOOR_UP_SCALE_Y);
    assert!(w.flags.get_switch(0x27));
    assert!(attention_on(&w, h, NA_SE_SY_CORRECT_CHIME));
    // ObjSwitch_FloorPress: still while the attention camera hasn't looked at it, then down
    // (`until_down`'s checks). It's the camera that lets it go (Camera_Demo5 sets D_8011D3AC to the
    // switch category), well before the 100 frames.
    let (down, n) = until_down(&mut w, h);
    assert!(n < 90, "{n}");
    assert_eq!(w.func_8005b198(), ACTORCAT_SWITCH as i32);
    // DynaPoly_UpdateContext builds the collision from the actor's scale: down, it's lower.
    let s = sw(&w, h);
    assert_eq!(w.col.dyna.actors[s.bg as usize].cur.scale, Vec3::new(0.1, obj_switch::FLOOR_DOWN_SCALE_Y, 0.1));
    let top_down = floor_at(&w, HOME0);
    assert!(top_down < top_up - 20.0 && top_down > HOME0.y, "{top_up} {top_down}");
    // Camera_Demo5 plays the camera's sound (data1, the chime); flag 0x27 also burns the web
    // over room 10's door (Bg_Ydan_Sp 0x19CA's burn flag: BgYdanSp_BurnWeb's
    // Sfx_PlaySfxCentered(NA_SE_SY_CORRECT_CHIME)), on the same frame.
    idle(&mut w, 60);
    let chimes = sfx_frames(&w, NA_SE_SY_CORRECT_CHIME, pressed - 1);
    assert_eq!(chimes.len(), 2, "{chimes:?}");
    assert_eq!(chimes[0], chimes[1], "{chimes:?}");
    // The flag also lights room 0's three golden torches (Obj_Syokudai 0x03E7), each with its
    // own attention camera after the switch's: wait for the main camera.
    let mut n = 0;
    while w.active_cam_id != oot_game::camera::CAM_ID_MAIN {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 600, "the attention cameras never ended");
    }
    // Once: ObjSwitch_FloorDown waits only for the flag to go; it stays down with Link gone.
    let away = Vec3::new(start.x, y, start.z);
    move_link(&mut w, away, -0x6000);
    for k in 0..60 {
        idle(&mut w, 1);
        let s = sw(&w, h);
        assert_eq!((s.action, s.actor.scale.y), (Action::FloorDown, obj_switch::FLOOR_DOWN_SCALE_Y), "frame {k}");
        assert!(w.flags.get_switch(0x27));
    }
    assert_eq!(sfx_frames(&w, obj_switch::NA_SE_EV_FOOT_SWITCH, down), Vec::<u32>::new());
    // The flag cleared (as a scene change would): ObjSwitch_FloorReleaseInit, then up at once
    // (not a toggle: no wait).
    w.flags.unset_switch(0x27);
    idle(&mut w, 1);
    assert_eq!(sw(&w, h).action, Action::FloorRelease);
    until_up(&mut w, h);
}

#[test]
fn room_5s_held_switch_is_down_while_link_stands_on_it() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 5);
    let h = switch_with(&w, 0x3E20);
    assert_eq!(sw(&w, h).action, Action::FloorUp);
    // Link stood on its middle: Actor_UpdateBgCheckInfo's floor is the switch's
    // (func_80043334: Player can press switches, DYNA_INTERACT_ACTOR_SWITCH_PRESSED), read by
    // ObjSwitch_FloorUp (held) the next frame: ObjSwitch_SetOn, flag 0x3E, the attention camera
    // with NA_SE_SY_TRE_BOX_APPEAR (not once or synced).
    let top = floor_at(&w, HOME5);
    w.place_player(Vec3::new(HOME5.x, top, HOME5.z), 0);
    let mut n = 0;
    while sw(&w, h).action == Action::FloorUp {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 5, "never pressed");
    }
    let pressed = w.audio.frames;
    let s = sw(&w, h);
    assert_eq!((s.action, s.cooldown_on), (Action::FloorPress, true));
    assert!(w.flags.get_switch(0x3E));
    assert!(attention_on(&w, h, NA_SE_SY_TRE_BOX_APPEAR));
    let (_, n) = until_down(&mut w, h);
    assert!(n < 90, "{n}");
    // Held: ObjSwitch_FloorDown resets releaseTimer to 6 every frame Link presses it (or Player's
    // in a cutscene), after the update counted it down.
    for k in 0..60 {
        idle(&mut w, 1);
        let s = sw(&w, h);
        assert_eq!((s.action, s.release_timer), (Action::FloorDown, 6), "frame {k}");
        assert!(w.flags.get_switch(0x3E));
    }
    assert!(!w.player_in_cs_mode());
    assert_eq!(sfx_frames(&w, NA_SE_SY_TRE_BOX_APPEAR, pressed - 1).len(), 1);
    // Link steps off (to the floor 100 along +x). The next frame the switch still reads his
    // last update's press (6); then releaseTimer counts down from 6, and at 0
    // ObjSwitch_FloorReleaseInit and ObjSwitch_SetOff: the flag cleared, no camera (held).
    let off = Vec3::new(HOME5.x + 100.0, 0.0, HOME5.z);
    let off = Vec3::new(off.x, floor_at(&w, Vec3::new(off.x, HOME5.y, off.z)), off.z);
    assert!(off.y < HOME5.y, "{off}");
    move_link(&mut w, off, 0);
    idle(&mut w, 1);
    assert_eq!((sw(&w, h).action, sw(&w, h).release_timer), (Action::FloorDown, 6));
    for t in (1..6).rev() {
        idle(&mut w, 1);
        let s = sw(&w, h);
        assert_eq!((s.action, s.release_timer), (Action::FloorDown, t));
        assert!(w.flags.get_switch(0x3E));
    }
    idle(&mut w, 1);
    let s = sw(&w, h);
    assert_eq!((s.action, s.release_timer, s.cooldown_on), (Action::FloorRelease, 0, false));
    assert!(!w.flags.get_switch(0x3E));
    assert_eq!(w.active_cam_id, oot_game::camera::CAM_ID_MAIN);
    // ObjSwitch_FloorRelease (held: no wait): up by 99 / 2000 a frame.
    until_up(&mut w, h);
}

/// Injects a hit on the eye's first triangle by `by` (`CollisionCheck_SetATvsAC`'s marks: AC_HIT,
/// the colliding actor, a slingshot seed's element).
fn shoot_eye(w: &mut PlayState, h: ActorHandle, by: ActorHandle) {
    let s = sw_mut(w, h);
    let c = &mut s.tris;
    c.base.ac_flags |= cc::AC_HIT;
    c.base.ac = Some(by);
    c.elements[0].info.ac_elem_flags |= cc::ACELEM_HIT;
    c.elements[0].info.ac_hit_elem = Some(cc::HitElem {
        elem: cc::ElemRef { col: cc::ColliderRef { actor: by, id: 0 }, elem: 0 },
        at_dmg_info: cc::ColliderElementDamageInfoAT { dmg_flags: cc::DMG_SLINGSHOT, hit_special_effect: cc::HIT_SPECIAL_EFFECT_NONE, damage: 1 },
        ac_dmg_info: cc::ColliderElementDamageInfoAC { dmg_flags: 0, hit_backlash: 0, defense: 0, hit_pos: [0; 3] },
        elem_material: cc::ELEM_MATERIAL_UNK0,
    });
}

#[test]
fn a_seed_from_the_front_closes_room_3s_eye() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 3);
    let h = switch_with(&w, 0x1502);
    let player = w.player.unwrap();
    // Link below the eye, in front of it (it faces -z), facing it: yaw 0.
    let p = Vec3::new(HOME3_EYE.x, 0.0, HOME3_EYE.z - 150.0);
    let y = floor_at(&w, Vec3::new(p.x, HOME3_EYE.y, p.z));
    w.place_player(Vec3::new(p.x, y, p.z), 0);
    idle(&mut w, 2);
    // From behind (a seed flying -z: world.rot.y -0x8000, the eye's own): ObjSwitch_EyeIsHit's
    // yawDiff 0, not over 0x5000. Nothing.
    w.player_mut().actor.world_rot.y = -32768;
    shoot_eye(&mut w, h, player);
    idle(&mut w, 1);
    let s = sw(&w, h);
    assert_eq!((s.action, s.eye_tex_index), (Action::EyeOpen, 0));
    assert!(!w.flags.get_switch(0x15));
    // prevColFlags keeps that frame's AC_HIT, so a hit the very next frame isn't a new one, even
    // from the front (yawDiff 0 - -0x8000: -0x8000, ABS 0x8000).
    assert_ne!(s.prev_col_flags & cc::AC_HIT, 0);
    w.player_mut().actor.world_rot.y = 0;
    shoot_eye(&mut w, h, player);
    idle(&mut w, 1);
    assert_eq!(sw(&w, h).action, Action::EyeOpen);
    assert!(!w.flags.get_switch(0x15));
    idle(&mut w, 1);
    // Then a new hit from the front: ObjSwitch_EyeClosingInit (cooldownTimer 100) and
    // ObjSwitch_SetOn: flag 0x15, the attention camera (NA_SE_SY_CORRECT_CHIME), cooldownOn.
    w.player_mut().actor.world_rot.y = 0;
    shoot_eye(&mut w, h, player);
    idle(&mut w, 1);
    let shot = w.audio.frames;
    let s = sw(&w, h);
    assert_eq!((s.action, s.cooldown_timer, s.cooldown_on, s.eye_tex_index), (Action::EyeClosing, 100, true, 0));
    assert!(w.flags.get_switch(0x15));
    assert!(attention_on(&w, h, NA_SE_SY_CORRECT_CHIME));
    // ObjSwitch_EyeClosing: a texture a frame once the camera looks at it (gold: opening,
    // closing), then at 3 ObjSwitch_EyeClosedInit and NA_SE_EV_FOOT_SWITCH.
    let mut n = 0;
    loop {
        let (i0, go) = (sw(&w, h).eye_tex_index, cooldown_over(&w, h));
        idle(&mut w, 1);
        n += 1;
        let s = sw(&w, h);
        let i = if go { i0 + 1 } else { i0 };
        if i >= 3 {
            assert_eq!((s.action, s.eye_tex_index), (Action::EyeClosed, 3));
            assert!(sfx_on(&w, w.audio.frames, obj_switch::NA_SE_EV_FOOT_SWITCH));
            break;
        }
        assert_eq!((s.action, s.eye_tex_index), (Action::EyeClosing, i), "frame {n}");
        assert!(n < 100);
    }
    assert!(n < 90, "{n}");
    // Closed for good (once): further hits do nothing, the flag stays.
    idle(&mut w, 40);
    shoot_eye(&mut w, h, player);
    idle(&mut w, 2);
    assert_eq!((sw(&w, h).action, sw(&w, h).eye_tex_index), (Action::EyeClosed, 3));
    assert!(w.flags.get_switch(0x15));
    assert_eq!(sfx_frames(&w, NA_SE_SY_CORRECT_CHIME, shot - 1).len(), 1);
}

/// Room 0's floor 150 in front of the entrance's spawn (-4, 0, 603), facing -z.
fn room_0_floor(w: &PlayState) -> Vec3 {
    let p = Vec3::new(-4.0, 0.0, 453.0);
    Vec3::new(p.x, floor_at(w, p), p.z)
}

#[test]
fn a_toggle_floor_switch_flips_each_time_link_steps_on() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 0);
    let at = room_0_floor(&w);
    let params = objswitch_params(obj_switch::OBJSWITCH_TYPE_FLOOR, obj_switch::OBJSWITCH_SUBTYPE_TOGGLE, 0x30);
    let h = w.actor_spawn(obj_switch::ACTOR_OBJ_SWITCH, at, [0; 3], params).expect("spawn");
    idle(&mut w, 2);
    assert_eq!(sw(&w, h).action, Action::FloorUp);
    let top = floor_at(&w, at);
    assert!(top > at.y + 1.0);
    let off = Vec3::new(at.x, at.y, at.z + 100.0);
    w.place_player(off, -32768);
    idle(&mut w, 3);
    // On: DYNA_INTERACT_PLAYER_ON_TOP and not last update's (prevColFlags): ObjSwitch_SetOn,
    // flag 0x30, the attention camera with NA_SE_SY_TRE_BOX_APPEAR (a toggle).
    move_link(&mut w, Vec3::new(at.x, top, at.z), -32768);
    let mut n = 0;
    while sw(&w, h).action == Action::FloorUp {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 5, "never pressed");
    }
    let first = w.audio.frames;
    assert!(w.flags.get_switch(0x30));
    assert!(sw(&w, h).cooldown_on);
    assert!(attention_on(&w, h, NA_SE_SY_TRE_BOX_APPEAR));
    let (_, n) = until_down(&mut w, h);
    assert!(n < 90, "{n}");
    // Standing on: Player on top every frame, never newly: it stays down.
    for k in 0..60 {
        idle(&mut w, 1);
        assert_eq!(sw(&w, h).action, Action::FloorDown, "frame {k}");
    }
    assert!(w.flags.get_switch(0x30));
    assert_eq!(sfx_frames(&w, NA_SE_SY_TRE_BOX_APPEAR, first - 1).len(), 1);
    // Off, and it stays down.
    move_link(&mut w, off, -32768);
    idle(&mut w, 5);
    assert_eq!(sw(&w, h).action, Action::FloorDown);
    assert!(w.flags.get_switch(0x30));
    // On again (the top is down now): ObjSwitch_FloorReleaseInit and ObjSwitch_SetOff: the flag
    // cleared with the attention camera (a toggle's), cooldownOn.
    let top_down = floor_at(&w, at);
    move_link(&mut w, Vec3::new(at.x, top_down, at.z), -32768);
    let mut n = 0;
    while sw(&w, h).action == Action::FloorDown {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 5, "never toggled");
    }
    let s = sw(&w, h);
    assert_eq!((s.action, s.cooldown_timer, s.cooldown_on), (Action::FloorRelease, 100, true));
    assert!(!w.flags.get_switch(0x30));
    assert!(attention_on(&w, h, NA_SE_SY_TRE_BOX_APPEAR));
    // ObjSwitch_FloorRelease (a toggle: waits for the camera), up with NA_SE_EV_FOOT_SWITCH.
    until_up(&mut w, h);
    // Still on it: not a new step, so it stays up.
    for k in 0..30 {
        idle(&mut w, 1);
        assert_eq!(sw(&w, h).action, Action::FloorUp, "frame {k}");
    }
    assert!(!w.flags.get_switch(0x30));
}

/// Injects a hit on the crystal's sphere.
fn hit_crystal(w: &mut PlayState, h: ActorHandle, by: ActorHandle) {
    let c = &mut sw_mut(w, h).jnt_sph;
    c.base.ac_flags |= cc::AC_HIT;
    c.base.ac = Some(by);
    c.elements[0].info.ac_elem_flags |= cc::ACELEM_HIT;
}

/// One frame of a toggle crystal, its scroll checked: `ObjSwitch_UpdateTwoTexScrollXY` (x1 - 1,
/// y1 + 1, x2 + 1, y2 - 1, each & 0x7F) every frame off or on, and on the frame a turn ends.
fn crystal_frame(w: &mut PlayState, h: ActorHandle) {
    let s = sw(w, h);
    let (a0, sc) = (s.action, [s.x1_tex_scroll, s.y1_tex_scroll, s.x2_tex_scroll, s.y2_tex_scroll]);
    idle(w, 1);
    let s = sw(w, h);
    let scrolls = match a0 {
        Action::CrystalOff | Action::CrystalOn => true,
        Action::CrystalTurnOn | Action::CrystalTurnOff => s.action != a0,
        _ => false,
    };
    let want = if scrolls { [sc[0].wrapping_sub(1) & 0x7F, sc[1].wrapping_add(1) & 0x7F, sc[2].wrapping_add(1) & 0x7F, sc[3].wrapping_sub(1) & 0x7F] } else { sc };
    assert_eq!([s.x1_tex_scroll, s.y1_tex_scroll, s.x2_tex_scroll, s.y2_tex_scroll], want, "{a0:?}");
}

#[test]
fn a_toggle_crystal_switch_turns_on_and_off_with_hits() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 0);
    let player = w.player.unwrap();
    let at = room_0_floor(&w);
    let params = objswitch_params(obj_switch::OBJSWITCH_TYPE_CRYSTAL, obj_switch::OBJSWITCH_SUBTYPE_TOGGLE, 0x31);
    let h = w.actor_spawn(obj_switch::ACTOR_OBJ_SWITCH, at, [0, 0x1234, 0], params).expect("spawn");
    // ObjSwitch_Init (crystal): no bg actor; the focus 30 up; its sphere placed once through the
    // actor's matrix (scale 0.1): {0, 300, 0} is 30 up, radius 20 × 100 / 100; metal, AC by
    // 0xEFC1FFFE, OC (OC1_TYPE_ALL, OC2_TYPE_2); off (ObjSwitch_CrystalOffInit: black, red).
    let s = sw(&w, h);
    assert_eq!((s.bg, s.actor.focus_pos), (eng_collision::dyna::BG_ACTOR_MAX, at + Vec3::Y * 30.0));
    let e = &s.jnt_sph.elements[0];
    assert_eq!((e.dim.world_sphere.center, e.dim.world_sphere.radius), ([at.x as i16, (at.y + 30.0) as i16, at.z as i16], 20));
    assert_eq!((e.info.ac_dmg_info.dmg_flags, s.jnt_sph.base.col_type), (0xEFC1_FFFE, cc::COL_MATERIAL_METAL));
    assert_eq!((s.jnt_sph.base.oc_flags1 & !cc::OC1_HIT, s.jnt_sph.base.oc_flags2 & !cc::OC2_HIT_PLAYER), (cc::OC1_ON | cc::OC1_TYPE_ALL, cc::OC2_TYPE_2));
    assert_eq!((s.action, s.crystal_color, s.crystal_subtype1_texture), (Action::CrystalOff, [0, 0, 0], CrystalTex::Red));
    assert_eq!(s.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    for _ in 0..3 {
        crystal_frame(&mut w, h);
    }
    // A hit: ObjSwitch_SetOn (flag 0x31, the camera, cooldownOn), disableAcTimer 10 (counted to
    // 9 by the update when Player isn't in a cutscene), ObjSwitch_CrystalTurnOnInit.
    hit_crystal(&mut w, h, player);
    let cs = w.player_in_cs_mode();
    crystal_frame(&mut w, h);
    let s = sw(&w, h);
    assert_eq!((s.action, s.cooldown_on, s.disable_ac_timer), (Action::CrystalTurnOn, true, if cs { 10 } else { 9 }));
    assert!(w.flags.get_switch(0x31));
    assert!(attention_on(&w, h, NA_SE_SY_TRE_BOX_APPEAR));
    // ObjSwitch_CrystalTurnOn: once the camera looks at it, ObjSwitch_CrystalOnInit (white,
    // blue), the scroll, NA_SE_EV_DIAMOND_SWITCH.
    let mut n = 0;
    while sw(&w, h).action == Action::CrystalTurnOn {
        crystal_frame(&mut w, h);
        n += 1;
        assert!(n < 90, "{n}");
    }
    let s = sw(&w, h);
    assert_eq!((s.action, s.crystal_color, s.crystal_subtype1_texture), (Action::CrystalOn, [255, 255, 255], CrystalTex::Blue));
    assert!(sfx_on(&w, w.audio.frames, obj_switch::NA_SE_EV_DIAMOND_SWITCH));
    // disableAcTimer counts down only out of Player's cutscenes; till it's out, no AC.
    let mut n = 0;
    while sw(&w, h).disable_ac_timer > 0 {
        let (t0, cs) = (sw(&w, h).disable_ac_timer, w.player_in_cs_mode());
        crystal_frame(&mut w, h);
        assert_eq!(sw(&w, h).disable_ac_timer, if cs { t0 } else { t0 - 1 });
        n += 1;
        assert!(n < 200);
    }
    // A second hit: ObjSwitch_CrystalTurnOffInit and ObjSwitch_SetOff (the flag cleared, the
    // camera: a toggle's); then off (black, red) with the sound.
    hit_crystal(&mut w, h, player);
    crystal_frame(&mut w, h);
    let s = sw(&w, h);
    assert_eq!((s.action, s.cooldown_on), (Action::CrystalTurnOff, true));
    assert!(!w.flags.get_switch(0x31));
    let mut n = 0;
    while sw(&w, h).action == Action::CrystalTurnOff {
        crystal_frame(&mut w, h);
        n += 1;
        assert!(n < 101, "{n}");
    }
    let s = sw(&w, h);
    assert_eq!((s.action, s.crystal_color, s.crystal_subtype1_texture), (Action::CrystalOff, [0, 0, 0], CrystalTex::Red));
    assert!(sfx_on(&w, w.audio.frames, obj_switch::NA_SE_EV_DIAMOND_SWITCH));
}

#[test]
fn the_targetable_crystal_and_the_frozen_eye() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 0);
    let at = room_0_floor(&w);
    // Type 4: ACTOR_FLAG_ATTENTION_ENABLED, ATTENTION_RANGE_4.
    let params = objswitch_params(obj_switch::OBJSWITCH_TYPE_CRYSTAL_TARGETABLE, obj_switch::OBJSWITCH_SUBTYPE_ONCE, 0x32);
    let h = w.actor_spawn(obj_switch::ACTOR_OBJ_SWITCH, at, [0; 3], params).expect("spawn");
    let s = sw(&w, h);
    assert_ne!(s.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!((s.actor.target_mode, s.action), (4, Action::CrystalOff));
    // A frozen eye (OBJSWITCH_FROZEN_FLAG): ObjSwitch_SpawnIce, Obj_Ice_Poly as its child with
    // the flag in its params (a placeholder here); ObjSwitch_EyeFrozenInit.
    let params = objswitch_params(obj_switch::OBJSWITCH_TYPE_EYE, obj_switch::OBJSWITCH_SUBTYPE_ONCE, 0x33) | obj_switch::OBJSWITCH_FROZEN_FLAG;
    let h = w.actor_spawn(obj_switch::ACTOR_OBJ_SWITCH, at + Vec3::Y * 50.0, [0; 3], params).expect("spawn");
    let s = sw(&w, h);
    assert_eq!(s.action, Action::EyeInit);
    let ice = s.actor.child.expect("the ice");
    let ice = w.actors.actor(ice).unwrap();
    assert_eq!((ice.id, ice.params, ice.parent), (obj_switch::ACTOR_OBJ_ICE_POLY, 0x3300, Some(h)));
    // ObjSwitch_EyeInit: open (the flag isn't set).
    idle(&mut w, 1);
    assert_eq!(sw(&w, h).action, Action::EyeOpen);
}
