//! GAME-09 milestone 3 against the C: the fire walls (`z_bg_hidan_curtain.c`). Dodongo's Cavern
//! (Master Quest): room 8's three (params 0x2FFC: type 2, the small one; chest flag 0x3F, switch
//! flag 0x3C, which the floor crystal `Obj_Switch` 0x3C03 at (1709, 16, -3789) sets) stand on the
//! 60-high step at x 1640 to 1800 in the wall between the gravestones' floor and the Armos'; room
//! 12's two (0x2FFA at x 2926, flag 0x3A: the floor switch 0x3A00 under the boulder `Obj_Bombiwa`
//! 0x8004; 0x2FF9 at x 3112, flag 0x39: the crystal 0x3903) stand on the y-531 walk's stepping
//! platforms.
//!
//! Child Link enters at room 7 (`ENTR_DODONGOS_CAVERN_1`) on a `cavern-bombs` save and goes into
//! room 8 or room 12 (`Room_RequestNewRoom`). Expected values are worked out from the C in the
//! comments. The numbers they use (`sHCParams[1]`, the small one's): 46 across, 88 high, scale
//! 0.055, sinking and rising 88 at 3 a frame (30 frames: 29 × 3 and 1); type 2's timer 400.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_hidan_curtain::*;
use oot_actors::obj_switch::ObjSwitch;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorHandle};
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::collision_check::MASS_IMMOVABLE;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Room 8's three (`ootx scene-info --scene ddan --layer 0`).
const ROOM_8: [Vec3; 3] = [Vec3::new(1655.0, 16.0, -3690.0), Vec3::new(1712.0, 16.0, -3692.0), Vec3::new(1770.0, 16.0, -3693.0)];
/// Room 12's: 0x2FFA and 0x2FF9.
const ROOM_12_3A: Vec3 = Vec3::new(2926.0, 531.0, -919.0);
const ROOM_12_39: Vec3 = Vec3::new(3112.0, 531.0, -921.0);
/// Room 12's crystal 0x3903.
const CRYSTAL_39: Vec3 = Vec3::new(2713.0, 411.0, -795.0);

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in `room` (`cavern-bombs`, the sound log on), entered at room 7 and the room
/// changed; Link at `at`, two frames on.
fn cavern_room(a: &Arc<GameAssets>, room: i8, at: Vec3, yaw: i16) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_1").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("cavern-bombs").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert!(w.room_request(room));
    idle(&mut w, 1);
    w.room_change_done();
    idle(&mut w, 3);
    w.place_player(at, yaw);
    idle(&mut w, 2);
    w
}

fn curtains(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.category(ACTORCAT_PROP).iter().copied().filter(|&h| w.actors.downcast::<BgHidanCurtain>(h).is_some_and(|c| !c.actor.killed)).collect()
}

fn at_home(w: &PlayState, home: Vec3) -> ActorHandle {
    *curtains(w).iter().find(|&&h| c(w, h).actor.home_pos == home).expect("the fire wall")
}

fn c(w: &PlayState, h: ActorHandle) -> &BgHidanCurtain {
    w.actors.downcast::<BgHidanCurtain>(h).expect("Bg_Hidan_Curtain")
}

/// The fire's AT and OC registered this frame (`alpha` over 50).
fn burning(w: &PlayState, h: ActorHandle) -> bool {
    c(w, h).alpha > 50
}

#[test]
fn room_8s_three_fire_walls_burn() {
    let Some(a) = assets() else { return };
    let mut w = cavern_room(&a, 8, Vec3::new(1500.0, 16.0, -3600.0), 0);
    assert_eq!(curtains(&w).len(), 3);
    for home in ROOM_8 {
        let h = at_home(&w, home);
        let x = c(&w, h);
        // BgHidanCurtain_Init: type 0x2FFC >> 12 = 2, the small size (sHCParams[1]: scale 0.055,
        // 46 across, 88 high), treasureFlag (0x2FFC >> 6) & 0x3F = 0x3F, the params masked to the
        // switch flag 0x3C; CollisionCheck_SetInfo(sCcInfoInit: 1, 80, 100, immovable); focus
        // 20 up; waiting for its flag.
        assert_eq!((x.ty, x.size, x.treasure_flag, x.actor.params, x.action), (2, 1, 0x3F, 0x3C, Action::WaitForSwitchOn));
        assert_eq!((x.actor.scale.x, x.collider.dim.radius, x.collider.dim.height), (0.055, 46, 88));
        let cc = &x.actor.col_chk_info;
        assert_eq!((cc.health, cc.cyl_radius, cc.cyl_height, cc.mass), (1, 80, 100, MASS_IMMOVABLE));
        assert_eq!(x.actor.focus_pos.y, home.y + 20.0);
        // Risen whole: alpha 255, the cylinder at its full height where it was placed, its
        // flames' sound (Actor_PlaySfx_Flagged: NA_SE_EV_FIRE_PILLAR_S - SFX_FLAG).
        assert_eq!((x.alpha, x.actor.world_pos), (255, home));
        assert_eq!(x.collider.dim.pos, [home.x as i16, home.y as i16, home.z as i16]);
        assert_eq!(x.actor.sfx, NA_SE_EV_FIRE_PILLAR_S - SFX_FLAG);
        assert!(x.tex_scroll >= 0);
        // The texture scrolls a step a frame.
        let t = x.tex_scroll;
        idle(&mut w, 1);
        assert_eq!(c(&w, h).tex_scroll, t + 1);
    }
}

#[test]
fn a_fire_wall_burns_link_and_knocks_him_down() {
    let Some(a) = assets() else { return };
    let mut w = cavern_room(&a, 8, Vec3::new(1500.0, 16.0, -3600.0), 0);
    let h = at_home(&w, ROOM_8[2]);
    let health = w.save.health;
    // Link on the step in the east wall's flames (its cylinder 46 across round (1770, -3693),
    // from y 16 up 88), out of its neighbour's (58 off). The frame's AT check hits him (4, HIT_SPECIAL_EFFECT_FIRE): Player burns
    // (func_80838280); the next update the wall sees AT_HIT and knocks him down
    // (Actor_SetPlayerKnockbackLargeNoDamage: 5 along its yaw to him, 1 up, no damage of its own).
    w.place_player(Vec3::new(1790.0, 76.0, -3700.0), 0);
    let mut knocked = None;
    for _ in 0..4 {
        idle(&mut w, 1);
        let p = w.player();
        if p.knockback_type == 2 {
            knocked = Some((p.knockback_speed, p.knockback_rot, p.knockback_y_velocity, p.knockback_damage));
            assert_eq!(p.knockback_rot, c(&w, h).actor.yaw_towards_player);
            break;
        }
    }
    let yaw = knocked.map(|k| k.1).unwrap_or(0);
    assert_eq!(knocked, Some((5.0, yaw, 1.0, 0)));
    idle(&mut w, 2);
    assert!(w.player().body_is_burning, "Link burns");
    assert!(w.save.health < health, "hurt: {} from {health}", w.save.health);
}

#[test]
fn flag_0x3c_puts_room_8s_walls_out_for_400_frames_then_they_light_and_clear_it() {
    let Some(a) = assets() else { return };
    // Link on the gravestones' floor out of the room's Lizalfos's reach (En_Zf 0xFFFF drops within
    // 160 of (1558, 16, -3540)): with it fighting him there, the walls' count lost a frame.
    let mut w = cavern_room(&a, 8, Vec3::new(1760.0, 16.0, -3460.0), 0);
    let hs: Vec<ActorHandle> = ROOM_8.iter().map(|&p| at_home(&w, p)).collect();
    w.flags.set_switch(0x3C);
    // BgHidanCurtain_WaitForSwitchOn: the flag set: type 2 straight to TurnOff.
    idle(&mut w, 1);
    for &h in &hs {
        assert_eq!(c(&w, h).action, Action::TurnOff);
    }
    // Sinking 3 a frame: alpha 255 × (88 − 3k) / 88; the cylinder (88 × that) registered while the
    // alpha's over 50 (k up to 23); the 30th frame down (88), the timer 400.
    for k in 1..=30 {
        idle(&mut w, 1);
        for (&h, home) in hs.iter().zip(ROOM_8) {
            let x = c(&w, h);
            let sunk = (3.0 * k as f32).min(88.0);
            assert_eq!(x.actor.world_pos.y, home.y - sunk, "frame {k}");
            let progress = (88.0 - sunk) / 88.0;
            assert_eq!(x.alpha, (255.0 * progress) as u8);
            assert_eq!(burning(&w, h), k <= 23, "frame {k}");
            if k < 30 {
                assert_eq!(x.action, Action::TurnOff);
            }
        }
    }
    for &h in &hs {
        assert_eq!((c(&w, h).action, c(&w, h).timer), (Action::WaitForTimer, 400));
    }
    // 400 frames out (DECR then the check: the 400th turns it on), the flag kept.
    for t in (1..400).rev() {
        idle(&mut w, 1);
        assert_eq!((c(&w, hs[0]).timer, c(&w, hs[0]).action), (t, Action::WaitForTimer));
    }
    assert!(w.flags.get_switch(0x3C));
    idle(&mut w, 1);
    assert_eq!(c(&w, hs[0]).action, Action::TurnOn);
    // Rising 3 a frame for 30 frames; there, BgHidanCurtain_TurnOn clears the flag.
    for k in 1..30 {
        idle(&mut w, 1);
        assert_eq!(c(&w, hs[0]).actor.world_pos.y, ROOM_8[0].y - 88.0 + 3.0 * k as f32);
        assert!(w.flags.get_switch(0x3C));
    }
    idle(&mut w, 1);
    for (&h, home) in hs.iter().zip(ROOM_8) {
        assert_eq!((c(&w, h).actor.world_pos, c(&w, h).action, c(&w, h).alpha), (home, Action::WaitForSwitchOn, 255));
    }
    assert!(!w.flags.get_switch(0x3C), "Flags_UnsetSwitch(0x3C)");
}

#[test]
fn room_12s_walls_each_go_out_by_their_own_switch() {
    let Some(a) = assets() else { return };
    let mut w = cavern_room(&a, 12, Vec3::new(2400.0, 531.0, -920.0), 0x4000);
    let w3a = at_home(&w, ROOM_12_3A);
    let w39 = at_home(&w, ROOM_12_39);
    assert_eq!((c(&w, w3a).ty, c(&w, w3a).actor.params, c(&w, w39).ty, c(&w, w39).actor.params), (2, 0x3A, 2, 0x39));
    // A bomb by the crystal 0x3903 (it takes explosions: 0xEFC1FFFE): flag 0x39 set, the wall at
    // x 3112 sinks; the one at x 2926 (0x3A, the floor switch's) burns on.
    let b = w.actor_spawn(oot_actors::en_bom::ACTOR_EN_BOM, CRYSTAL_39 + Vec3::new(20.0, 0.0, 0.0), [0, 0, 0], 0).expect("bomb");
    w.actors.downcast_mut::<oot_actors::en_bom::EnBom>(b).unwrap().timer = 2;
    let crystal = w.actors.all().into_iter().find(|&h| w.actors.downcast::<ObjSwitch>(h).is_some_and(|s| s.actor.home_pos == CRYSTAL_39)).expect("the crystal");
    let mut n = 0;
    while !w.flags.get_switch(0x39) {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 150, "the crystal never set 0x39");
    }
    // The crystal (ACTORCAT_SWITCH) sets it in its update, before the walls' (ACTORCAT_PROP):
    // the wall at x 3112 starts down this same frame; down on the 30th after, its timer 400.
    assert!(w.actors.downcast::<ObjSwitch>(crystal).is_some());
    assert_eq!(c(&w, w39).action, Action::TurnOff);
    idle(&mut w, 30);
    assert_eq!((c(&w, w39).action, c(&w, w39).timer), (Action::WaitForTimer, 400));
    assert!(!burning(&w, w39));
    assert_eq!((c(&w, w3a).action, c(&w, w3a).alpha), (Action::WaitForSwitchOn, 255));
    // Flag 0x3A (the floor switch's) puts out the other.
    w.flags.set_switch(0x3A);
    idle(&mut w, 31);
    assert_eq!((c(&w, w3a).action, c(&w, w3a).timer), (Action::WaitForTimer, 400));
}

/// Room 8's way south (the question of how a child passes): the gravestones' floor (y 16) ends at
/// the room's dividing wall (z -3680 to -3720, 196 high) but for the 60-high step at x 1640 to
/// 1800 that the three walls stand on; the crystal 0x3C03 that puts them out is beyond them, on
/// the south floor, out of the slingshot's sight (the step's in the way). A bomb thrown running
/// from the north floor clears the step and lands by the crystal: its blast hits it (the crystal
/// takes explosives), 0x3C is set, the walls sink; once they're low (alpha 50 or under: no
/// cylinder) Link climbs the step and drops to the south floor unburnt.
#[test]
fn a_bomb_thrown_over_room_8s_walls_hits_their_crystal_and_link_crosses() {
    use eng_input::pad::BTN_CRIGHT;
    let Some(a) = assets() else { return };
    let mut w = cavern_room(&a, 8, Vec3::new(1709.0, 16.0, -3560.0), i16::MIN);
    let walls: Vec<ActorHandle> = ROOM_8.iter().map(|&p| at_home(&w, p)).collect();
    // C-Right: a bomb out of the bag, held over his head.
    w.tick_with(scripted_input(PadState::default(), with(PadState::default(), BTN_CRIGHT)));
    let mut bomb = None;
    for _ in 0..40 {
        idle(&mut w, 1);
        bomb = w.actors.all().into_iter().find(|&h| w.actors.downcast::<oot_actors::en_bom::EnBom>(h).is_some());
        if bomb.is_some() && w.player().held_actor.is_some() {
            break;
        }
    }
    let bomb = bomb.expect("a bomb in hand");
    idle(&mut w, 6);
    // Running south at the step (8 frames), C-Right again: thrown (Link faster than 1.1).
    let run = PadState { button: 0, stick_x: 0, stick_y: 80 };
    let mut prev = PadState::default();
    for _ in 0..8 {
        w.tick_with(scripted_input(prev, run));
        prev = run;
    }
    let cur = with(run, BTN_CRIGHT);
    w.tick_with(scripted_input(prev, cur));
    // Over the step (its top 76 from z -3680 to -3720) and down beyond it.
    let mut over = f32::MAX;
    let mut landed = None;
    for _ in 0..40 {
        idle(&mut w, 1);
        let Some(b) = w.actors.downcast::<oot_actors::en_bom::EnBom>(bomb) else { break };
        let p = b.actor.world_pos;
        if (-3720.0..=-3680.0).contains(&p.z) {
            over = over.min(p.y);
        }
        if p.z < -3740.0 && p.y < 20.0 && landed.is_none() {
            landed = Some(p);
        }
    }
    assert!(over > 76.0, "the bomb over the step: {over}");
    let landed = landed.expect("the bomb on the south floor");
    assert!(landed.distance(Vec3::new(1709.0, 16.0, -3789.0)) < 80.0, "by the crystal: {landed:?}");
    let n = {
        let mut n = 0;
        while !w.flags.get_switch(0x3C) {
            idle(&mut w, 1);
            n += 1;
            assert!(n < 100, "the crystal never set 0x3C");
        }
        n
    };
    assert!(n > 0);
    // The walls sink; 24 frames on they're out (alpha 50 or under).
    idle(&mut w, 26);
    for &h in &walls {
        assert!(!burning(&w, h));
    }
    // Link runs south: up the 60 step (a hang and a climb), off its south side.
    w.place_player(Vec3::new(1709.0, 16.0, -3640.0), i16::MIN);
    idle(&mut w, 1);
    let health = w.save.health;
    let mut prev = PadState::default();
    let mut top = 0.0f32;
    for _ in 0..150 {
        w.tick_with(scripted_input(prev, run));
        prev = run;
        top = top.max(w.player().actor.world_pos.y);
        if w.player().actor.world_pos.z < -3760.0 && w.player().actor.world_pos.y < 20.0 {
            break;
        }
    }
    let p = w.player().actor.world_pos;
    assert!(top >= 75.0, "on the step: top {top}");
    assert!(p.z < -3760.0 && p.y < 20.0, "on the south floor: {p:?}");
    assert!(!w.player().body_is_burning && w.save.health == health, "unburnt");
}
