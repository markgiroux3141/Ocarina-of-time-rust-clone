//! The Deku Tree's webs (`Bg_Ydan_Sp`, `z_bg_ydan_sp.c`) against the C, in the Master Quest
//! Deku Tree (`ootx scene-info --scene ydan`):
//! - room 0's floor web, params 0x0FC5 at (0, 0, 0) (destroyed flag 5): the bottom of the pit
//!   under room 0's top floor (y 800), over the drop to room 3 (`En_Holl` transition 9 at
//!   (0, -320, 0));
//! - room 0's wall webs, 0x19CA at (-491, 800, -1) facing +x (burn flag 0x27, destroyed 0x0A: over
//!   room 10's door) and 0x1FD6 at (-388, 400, 389) facing 0x6000 (destroyed 0x16: over room 1's
//!   door);
//! - room 3's floor web, 0x0FC6 at (-635, -820, 0) (destroyed 6).
//!
//! Expected values are worked out from the C in the comments. The tests set what Player holds
//! (`heldItemAction`, `unk_860`, the stick's tip) for a frame: the webs (`ACTORCAT_BG`) update
//! before Player and read it as set. (Link's own stick, lit at a torch, burns room 1's web in
//! `tests/stick_run.rs`.) A fire hit, which nothing Link has gives, is set on the web's collider
//! as the collision check leaves it.

mod common;

use std::f64::consts::PI;
use std::sync::Arc;

use common::*;
use eng_collision::dyna::DYNA_INTERACT_PLAYER_ON_TOP;
use eng_input::pad::PadState;
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_ydan_sp::{self, Action, BgYdanSp, NA_SE_EV_WEB_BROKEN, NA_SE_EV_WEB_VIBRATION, Statics, WEB_FLOOR, WEB_WALL};
use oot_actors::player::PLAYER_IA_DEKU_STICK;
use oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::NA_SE_SY_CORRECT_CHIME;
use oot_game::collision_check as cc;
use oot_game::effect::{EFFECT_SS_DEAD_DB, EFFECT_SS_DUST};
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

/// Inside the Deku Tree (`deku-tree-inside`), the sound log on, 60 frames in, then in `room`
/// (`Room_RequestNewRoom`, a frame, `Room_FinishRoomChange`; room 0 is the entrance's).
fn deku_tree_room(a: &Arc<GameAssets>, room: i8) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 60);
    if room != 0 {
        change_room(&mut w, room);
    }
    w
}

fn change_room(w: &mut PlayState, room: i8) {
    assert!(w.room_request(room));
    idle(w, 1);
    w.room_change_done();
}

/// The live web whose destroyed flag is `flag`.
fn find(w: &PlayState, flag: u8) -> Option<ActorHandle> {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<BgYdanSp>(h).is_some_and(|s| s.is_destroyed_switch_flag == flag && !s.actor.killed))
}

fn web(w: &PlayState, h: ActorHandle) -> &BgYdanSp {
    w.actors.downcast::<BgYdanSp>(h).expect("Bg_Ydan_Sp")
}

fn web_mut(w: &mut PlayState, h: ActorHandle) -> &mut BgYdanSp {
    w.actors.downcast_mut::<BgYdanSp>(h).expect("Bg_Ydan_Sp")
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

/// `sinf((f32)timer * (M_PI / n))`, the product in double precision.
fn timer_sin(timer: i16, n: f64) -> f32 {
    ((timer as f32 as f64 * (PI / n)) as f32).sin()
}

/// `gDTWebFloorCol` as the floor webs have written it.
fn floor_web_col(w: &PlayState) -> Arc<eng_collision::collision::CollisionHeader> {
    w.overlay_statics.get(&bg_ydan_sp::ACTOR_BG_YDAN_SP).and_then(|s| s.downcast_ref::<Statics>()).and_then(|s| s.floor_web_col.clone()).expect("the floor webs' header")
}

/// The live soft sprites of type `ty`.
fn effects(w: &PlayState, ty: u8) -> Vec<&oot_game::effect::EffectSs> {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == ty).collect()
}

/// The newest `EffectSsDeadDb` flames: those with the most life left.
fn newest_flames(w: &PlayState) -> Vec<&oot_game::effect::EffectSs> {
    let f = effects(w, EFFECT_SS_DEAD_DB);
    let max = f.iter().map(|e| e.life).max().unwrap_or(-1);
    f.into_iter().filter(|e| e.life == max).collect()
}

/// Sets what Link holds: `heldItemAction`, `unk_860` and the stick's tip.
fn hold(w: &mut PlayState, item: i32, unk_860: i16, tip: Vec3) {
    let p = w.player_mut();
    p.held_item_ap = item;
    p.item_ap = item;
    p.unk_860 = unk_860;
    p.melee_weapon_info[0].tip = tip;
}

#[test]
fn init_room_0s_webs() {
    let Some(a) = assets() else { return };
    let w = deku_tree_room(&a, 0);
    // 0x0FC5: destroyed flag 0x05, burn flag 0x3F, the floor type; scale 0.1 (sInitChain).
    let f = web(&w, find(&w, 0x05).expect("room 0's floor web"));
    assert_eq!((f.burn_switch_flag, f.actor.params, f.action, f.actor.scale), (0x3F, WEB_FLOOR, Action::FloorWebIdle, Vec3::splat(0.1)));
    // Its tris: sTrisElementsInit[0] at its position, then the other half of the square.
    let t = &f.collider_tris.elements;
    assert_eq!(t[0].dim.vtx, [Vec3::new(75.0, -8.0, 75.0), Vec3::new(-75.0, -8.0, 75.0), Vec3::new(-75.0, -8.0, -75.0)]);
    assert_eq!(t[1].dim.vtx, [Vec3::new(75.0, -8.0, 75.0), Vec3::new(-75.0, -8.0, -75.0), Vec3::new(75.0, -8.0, -75.0)]);
    assert_eq!(t[0].info.ac_dmg_info.dmg_flags, cc::DMG_ARROW_FIRE | cc::DMG_MAGIC_FIRE);
    assert_eq!((f.collider_tris.base.ac_flags & !cc::AC_HIT, f.collider_tris.base.oc_flags2), (cc::AC_ON | cc::AC_TYPE_PLAYER, cc::OC2_TYPE_2));
    // gDTWebFloorCol: its rim (the eight vertices the web writes) at 0, its middle 100 under.
    let col = floor_web_col(&w);
    for i in 0..col.vertices.len() {
        let want = if bg_ydan_sp::FLOOR_WEB_COL_VERTICES.contains(&i) { 0 } else { -100 };
        assert_eq!(col.vertices[i][1], want, "vertex {i}");
    }
    // Standing still, the floor is its middle, 10 under it (scale 0.1); the top floor at
    // (0, 800, 30) is the pit over it.
    let floor = |y: f32| w.col.entity_raycast_down(Vec3::new(0.0, y, 30.0));
    assert_eq!((floor(100.0).0, floor(900.0).0, floor(100.0).1.map(|p| p.bg)), (-10.0, -10.0, Some(f.bg)));

    // 0x19CA: destroyed 0x0A, burn 0x27, the wall type, its focus 30 up.
    let wl = web(&w, find(&w, 0x0A).expect("the web over room 10's door"));
    assert_eq!((wl.burn_switch_flag, wl.actor.params, wl.action), (0x27, WEB_WALL, Action::WallWebIdle));
    assert_eq!(wl.actor.focus_pos, Vec3::new(-491.0, 830.0, -1.0));
    // The tris on its face: rot (0, 0x4000, 0), so x' = pos.x + cos·vx, z' = pos.z - sin·vx.
    let (s, c) = (sin_s(0x4000), cos_s(0x4000));
    let p = Vec3::new(-491.0, 800.0, -1.0);
    let at = |vx: f32, vy: f32| Vec3::new(p.x + (c * vx) - (s * vy * -0.0), p.y + vy * 1.0, p.z - (s * vx) + (vy * c * -0.0));
    let t = &wl.collider_tris.elements;
    assert_eq!(t[0].dim.vtx, [at(140.0, 288.8), at(-140.0, 288.0), at(-140.0, 0.0)]);
    assert_eq!(t[1].dim.vtx, [at(140.0, 288.8), at(-140.0, 0.0), at(140.0, 0.0)]);
    // 0x1FD6: destroyed 0x16, burn 0x3F.
    let wl = web(&w, find(&w, 0x16).expect("the web over room 1's door"));
    assert_eq!((wl.burn_switch_flag, wl.actor.params, wl.actor.shape_rot.y), (0x3F, WEB_WALL, 0x6000));
}

/// Link dropped 100 onto room 0's floor web: the frame after his landing (Player sets
/// `DYNA_INTERACT_PLAYER_ON_TOP` and his `fallDistance`; the BG category updates before him), the
/// web swings by `2 √fallDistance` from a 14-frame timer: `timer` 14 then 13 at once,
/// `world.y = home.y + sin(13π/7) unk_16C`, `unk_16C` less 0.8, and at 13 with it over 3,
/// `NA_SE_EV_WEB_VIBRATION`. Its rim's vertices go to `(home.y - world.y) × 10`.
#[test]
fn a_landing_swings_the_floor_web() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0);
    let h = find(&w, 0x05).unwrap();
    let bg = web(&w, h).bg;
    w.place_player(Vec3::new(0.0, 90.0, 30.0), 0);
    let mut fall = None;
    for _ in 0..40 {
        if w.col.dyna.interact_flag(bg, DYNA_INTERACT_PLAYER_ON_TOP) {
            fall = Some(w.player().fall_distance);
            break;
        }
        assert_eq!((web(&w, h).unk_16c, web(&w, h).actor.world_pos.y), (0.0, 0.0), "still before he lands");
        idle(&mut w, 1);
    }
    let fall = fall.expect("Link never landed on the web");
    // From 90 to its middle at -10.
    assert!((99..=101).contains(&fall), "{fall}");
    idle(&mut w, 1);
    let unk = (fall as f32).sqrt() * 2.0;
    let s = web(&w, h);
    let y = timer_sin(13, 7.0) * unk;
    assert_eq!((s.timer, s.actor.world_pos.y, s.unk_16c), (13, y, unk - 0.8));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EV_WEB_VIBRATION));
    let col = floor_web_col(&w);
    let new_y = ((0.0 - y) * 10.0) as i32 as i16;
    assert!(new_y > 0);
    assert!(bg_ydan_sp::FLOOR_WEB_COL_VERTICES.iter().all(|&i| col.vertices[i][1] == new_y));
    assert_eq!(col.vertices[2][1], -100);
    // The DynaPoly expands it the next frame (DynaPoly_UpdateContext follows the BG category):
    // the rim stays at home, the middle 10 under the web (truncated to s16).
    let b = &w.col.dyna.actors[bg as usize];
    let vs = b.vtx_start;
    assert_eq!(w.col.dyna.verts_s[vs + 14][1], (y + new_y as f32 * 0.1) as i16);
    assert_eq!(w.col.dyna.verts_s[vs + 2][1], (y - 10.0) as i16);
    // It goes round its 14 frames: the next swing's start (timer 13 again), the vibration
    // again while it's over 3.
    let f0 = w.audio.frames;
    let mut seen = vec![];
    for _ in 0..14 {
        idle(&mut w, 1);
        seen.push(web(&w, h).timer);
    }
    assert_eq!(seen, vec![12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 14, 13]);
    assert!(web(&w, h).unk_16c > 3.0);
    assert!(sfx_on(&w, f0 + 14, NA_SE_EV_WEB_VIBRATION));
}

/// Link walking on room 0's floor web: with his speed not 0 and the web still (`unk_16C` under
/// 0.1), its timer starts at 14 and `unk_16C` is at least 2, so it swings by 2: 13 at once,
/// `sin(13π/7) × 2`, then 1.2; at 13 under 3, `Audio_StopSfxById` (no vibration).
#[test]
fn walking_bounces_the_floor_web() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0);
    let h = find(&w, 0x05).unwrap();
    w.place_player(Vec3::new(0.0, -10.0, -40.0), 0);
    idle(&mut w, 10);
    assert_eq!(web(&w, h).unk_16c, 0.0, "standing still on it");
    let fwd = PadState { button: 0, stick_x: 0, stick_y: 60 };
    let mut prev = PadState::default();
    let mut n = 0;
    loop {
        let speed = w.player().actor.speed_xz;
        w.tick_with(scripted_input(prev, fwd));
        prev = fwd;
        n += 1;
        assert!(n < 20, "it never bounced");
        if speed != 0.0 {
            break;
        }
    }
    let s = web(&w, h);
    assert_eq!((s.timer, s.actor.world_pos.y, s.unk_16c), (13, timer_sin(13, 7.0) * 2.0, 2.0 - 0.8));
    assert!(!sfx_on(&w, w.audio.frames, NA_SE_EV_WEB_VIBRATION));
    // While he walks, it keeps swinging by at least 2 (less the 0.8 once updated).
    for _ in 0..10 {
        let speed = w.player().actor.speed_xz;
        w.tick_with(scripted_input(prev, fwd));
        if speed != 0.0 && w.player().actor.floor_bg_id == web(&w, h).bg {
            assert!(web(&w, h).unk_16c >= 1.2, "{}", web(&w, h).unk_16c);
        }
    }
}

/// Link dropped from room 0's top floor (y 800, a fall of 810 to the web's middle, over 750)
/// 30 from its centre (under 80): the web breaks (`unk_16C` 200, room -1, updated out of view,
/// 40 frames, `NA_SE_EV_WEB_BROKEN`), sinks on `sin(timer π/20) × 200`, and at timer 32
/// (190.2 down, over 190) its collision goes, it's broken for 40 frames, the chime, flag 5, six
/// puffs of dust 60 under it in a ring of 60. Link falls through, past `En_Holl` at -320, into
/// room 3; the broken web isn't room 0's any more, so it stays for its 40 frames.
#[test]
fn a_long_fall_breaks_the_floor_web_and_link_drops_into_room_3() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0);
    let h = find(&w, 0x05).unwrap();
    let bg = web(&w, h).bg;
    w.place_player(Vec3::new(0.0, 800.0, 30.0), 0);
    let mut fall = None;
    for _ in 0..80 {
        if w.col.dyna.interact_flag(bg, DYNA_INTERACT_PLAYER_ON_TOP) {
            fall = Some(w.player().fall_distance);
            break;
        }
        idle(&mut w, 1);
    }
    let fall = fall.expect("Link never landed on the web");
    assert!(fall > 750, "{fall}");
    assert!(web(&w, h).actor.xz_dist_to_player < 80.0);
    idle(&mut w, 1);
    let s = web(&w, h);
    assert_eq!((s.action, s.unk_16c, s.actor.room, s.timer), (Action::FloorWebBreaking, 200.0, -1, 40));
    assert_ne!(s.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EV_WEB_BROKEN));
    // It hasn't moved yet: the breaking starts next frame.
    assert_eq!(s.actor.world_pos.y, 0.0);
    for k in 1..=7i16 {
        idle(&mut w, 1);
        let s = web(&w, h);
        let y = timer_sin(40 - k, 20.0) * 200.0;
        assert_eq!((s.action, s.timer, s.actor.world_pos.y), (Action::FloorWebBreaking, 40 - k, y), "frame {k}");
        assert!(-y <= 190.0);
        // The rim's vertices keep it at home; Link rides the rest down (DYNA_TRANSFORM_POS).
        let new_y = (-y * 10.0) as i32 as i16;
        assert!(bg_ydan_sp::FLOOR_WEB_COL_VERTICES.iter().all(|&i| floor_web_col(&w).vertices[i][1] == new_y));
        if k == 7 {
            // Timer 33: 178.2 down. The DynaPoly's rim at 0, its middle 188 down.
            let vs = w.col.dyna.actors[bg as usize].vtx_start;
            assert_eq!((w.col.dyna.verts_s[vs][1], w.col.dyna.verts_s[vs + 2][1]), ((y + new_y as f32 * 0.1) as i16, (y - 10.0) as i16));
            assert!((w.player().actor.world_pos.y - (y - 10.0)).abs() < 2.0, "Link rides it down: {}", w.player().actor.world_pos.y);
        }
    }
    assert!(!w.flags.get_switch(0x05));
    let dust = effects(&w, EFFECT_SS_DUST).len();
    idle(&mut w, 1);
    let s = web(&w, h);
    let y = timer_sin(32, 20.0) * 200.0;
    assert!(-y > 190.0);
    assert_eq!((s.action, s.timer, s.actor.world_pos.y), (Action::FloorWebBroken, 40, y));
    assert!(w.col.dyna.actors[bg as usize].collision_disabled);
    assert!(w.flags.get_switch(0x05));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_SY_CORRECT_CHIME));
    assert_eq!(effects(&w, EFFECT_SS_DUST).len(), dust + 6);
    // The breaking frame's write stays in the shared header: (s16)(190.2 × 10).
    let new_y = (-y * 10.0) as i32 as i16;
    assert_eq!(new_y, 1902);
    assert!(bg_ydan_sp::FLOOR_WEB_COL_VERTICES.iter().all(|&i| floor_web_col(&w).vertices[i][1] == new_y));

    // Link falls through it and into room 3.
    let mut n = 0;
    while w.room_ctx.cur.num != 3 {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 120, "never reached room 3: Link at {}", w.player().actor.world_pos);
        // Broken, the web stops writing its collision.
        if web(&w, h).action == Action::FloorWebBroken {
            assert!(bg_ydan_sp::FLOOR_WEB_COL_VERTICES.iter().all(|&i| floor_web_col(&w).vertices[i][1] == 1902));
        }
    }
    assert!(w.player().actor.world_pos.y < -320.0);
    // The broken web isn't in a room: it's still there, until its 40 frames are out.
    let mut alive = 0;
    while w.actors.exists(h) {
        alive += 1;
        idle(&mut w, 1);
        assert!(alive < 60);
    }
    // Room 3's own floor web, once loaded, writes its own rim (0) over the broken one's.
    idle(&mut w, 5);
    assert!(find(&w, 0x06).is_some(), "room 3's floor web");
    assert!(bg_ydan_sp::FLOOR_WEB_COL_VERTICES.iter().all(|&i| floor_web_col(&w).vertices[i][1] == 0));
}

/// Room 0's web over room 10's door, on its burn flag 0x27 (the top floor's switch): its home
/// 80 up its middle, the chime, its flag 0x0A, then 30 frames: six flames every third frame
/// (timer 27, 24, .. 3), from its home and in its plane, and gone on the 30th.
#[test]
fn the_switch_flag_burns_the_wall_web() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0);
    let h = find(&w, 0x0A).unwrap();
    idle(&mut w, 1);
    assert_eq!(web(&w, h).action, Action::WallWebIdle);
    w.flags.set_switch(0x27);
    idle(&mut w, 1);
    let s = web(&w, h);
    assert_eq!((s.action, s.timer, s.actor.home_pos), (Action::BurnWallWeb, 30, Vec3::new(-491.0, 880.0, -1.0)));
    assert!(w.flags.get_switch(0x0A));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_SY_CORRECT_CHIME));
    let mut rounds = 0;
    for k in 1..=29i16 {
        let before = effects(&w, EFFECT_SS_DEAD_DB).len();
        idle(&mut w, 1);
        let s = web(&w, h);
        assert_eq!(s.timer, 30 - k);
        if (30 - k) % 3 == 0 {
            rounds += 1;
            let f = newest_flames(&w);
            assert_eq!(f.len(), 6, "timer {}", 30 - k);
            for e in f {
                // Spawned at its home, then moved once by its velocity (accel 0).
                assert!((e.pos - e.velocity).distance(Vec3::new(-491.0, 880.0, -1.0)) < 1e-3);
                // In its plane: (velocity) · (sin yaw, 0, cos yaw) = 0.
                assert!((e.velocity.x * sin_s(0x4000) + e.velocity.z * cos_s(0x4000)).abs() < 1e-3);
                assert!(e.velocity.length() > 0.0);
                assert_eq!(e.regs[0], 80 + 6, "scale 80, grown once by 6");
            }
        } else {
            assert!(effects(&w, EFFECT_SS_DEAD_DB).len() <= before);
        }
    }
    assert_eq!(rounds, 9);
    assert!(!web(&w, h).actor.killed);
    idle(&mut w, 1);
    assert!(web(&w, h).actor.killed, "killed on the 30th frame");
    // Deleted when Actor_UpdateAll next reaches it.
    idle(&mut w, 1);
    assert!(!w.actors.exists(h));
}

/// Room 0's web over room 1's door (facing 0x6000) and a burning Deku Stick: its tip within 100
/// across its face, behind it (local z under 1) and under 200 up burns it from the tip, with
/// one-point cutscene 3020 for 40 frames. In front of it, too far across, too high, a stick
/// that isn't burning or no stick: nothing.
#[test]
fn a_burning_stick_burns_the_wall_web() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 0);
    let h = find(&w, 0x16).unwrap();
    let p = web(&w, h).actor.world_pos;
    let (s, c) = (sin_s(0x6000), cos_s(0x6000));
    // Actor_WorldToActorCoords inverted: dx = x c + z s, dz = z c - x s.
    let tip = |x: f32, y: f32, z: f32| Vec3::new(p.x + x * c + z * s, p.y + y, p.z + z * c - x * s);
    let held_before = w.player().held_item_ap;
    for (why, item, unk_860, t) in [
        ("in front", PLAYER_IA_DEKU_STICK, 100, tip(0.0, 100.0, 30.0)),
        ("too far across", PLAYER_IA_DEKU_STICK, 100, tip(110.0, 100.0, -10.0)),
        ("too high", PLAYER_IA_DEKU_STICK, 100, tip(0.0, 210.0, -10.0)),
        ("not burning", PLAYER_IA_DEKU_STICK, 0, tip(0.0, 100.0, -10.0)),
        ("no stick", held_before, 100, tip(0.0, 100.0, -10.0)),
    ] {
        hold(&mut w, item, unk_860, t);
        idle(&mut w, 1);
        hold(&mut w, held_before, 0, Vec3::ZERO);
        assert_eq!(web(&w, h).action, Action::WallWebIdle, "{why}");
        assert!(!w.flags.get_switch(0x16), "{why}");
    }
    assert!(w.sub_cameras.iter().flatten().all(|c| c.cs_id != 3020));
    let t = tip(-60.0, 150.0, 0.0);
    hold(&mut w, PLAYER_IA_DEKU_STICK, 100, t);
    idle(&mut w, 1);
    hold(&mut w, held_before, 0, Vec3::ZERO);
    let s = web(&w, h);
    assert_eq!((s.action, s.timer, s.actor.home_pos), (Action::BurnWallWeb, 30, t));
    assert!(w.flags.get_switch(0x16));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_SY_CORRECT_CHIME));
    let cam = w.sub_cameras.iter().flatten().find(|c| c.cs_id == 3020).expect("one-point cutscene 3020");
    assert_eq!(cam.target, Some(h));
}

/// Room 3's floor web hit by fire (its tris take only fire): the chime, flag 6, then 30 frames
/// of flames from its middle (home and position the same: each point 120 out is 1 away, so
/// none turns round and each flies at 7 across), and it's gone with its collision.
#[test]
fn fire_burns_room_3s_floor_web() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 3);
    let h = find(&w, 0x06).expect("room 3's floor web");
    let (home, bg) = (web(&w, h).actor.home_pos, web(&w, h).bg);
    assert_eq!(home, Vec3::new(-635.0, -820.0, 0.0));
    {
        let s = web_mut(&mut w, h);
        s.collider_tris.base.ac_flags |= cc::AC_HIT;
        s.collider_tris.elements[0].info.ac_hit_elem = None;
    }
    idle(&mut w, 1);
    let s = web(&w, h);
    assert_eq!((s.action, s.timer), (Action::BurnFloorWeb, 30));
    assert!(w.flags.get_switch(0x06));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_SY_CORRECT_CHIME));
    idle(&mut w, 2);
    assert!(effects(&w, EFFECT_SS_DEAD_DB).is_empty(), "none at 29 and 28");
    idle(&mut w, 1);
    let f = newest_flames(&w);
    assert_eq!(f.len(), 6);
    for e in f {
        assert!((e.pos - e.velocity).distance(home) < 1e-3);
        assert_eq!(e.velocity.y, 0.0);
        assert!((e.velocity.length() - 7.0).abs() < 1e-3, "{}", e.velocity.length());
        assert_eq!(e.regs[0], 60 + 6, "scale 60, grown once by 6");
    }
    idle(&mut w, 26);
    assert!(!web(&w, h).actor.killed);
    idle(&mut w, 1);
    assert!(web(&w, h).actor.killed, "killed on the 30th frame");
    assert!(w.col.dyna.is_bg_actor(bg));
    // Deleted when Actor_UpdateAll next reaches it, its bg actor with it (BgYdanSp_Destroy).
    idle(&mut w, 1);
    assert!(!w.actors.exists(h));
    assert!(!w.col.dyna.is_bg_actor(bg));
}

/// `Player_IsBurningStickInRange(play, web 50 down, 70, 50)`: a burning stick's tip 0 to 50 over
/// the point 50 under room 3's floor web, within 70 across, burns it from the tip's x and z. Too
/// far across or above the web: nothing.
#[test]
fn a_burning_stick_under_the_floor_web_burns_it() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 3);
    let h = find(&w, 0x06).unwrap();
    let p = web(&w, h).actor.world_pos;
    let held_before = w.player().held_item_ap;
    for (why, t) in [("above it", p + Vec3::new(0.0, 5.0, 0.0)), ("too far across", p + Vec3::new(50.0, -20.0, 50.0)), ("too far down", p + Vec3::new(0.0, -55.0, 0.0))] {
        hold(&mut w, PLAYER_IA_DEKU_STICK, 100, t);
        idle(&mut w, 1);
        hold(&mut w, held_before, 0, Vec3::ZERO);
        assert_eq!(web(&w, h).action, Action::FloorWebIdle, "{why}");
    }
    let t = p + Vec3::new(30.0, -20.0, -40.0);
    hold(&mut w, PLAYER_IA_DEKU_STICK, 100, t);
    idle(&mut w, 1);
    hold(&mut w, held_before, 0, Vec3::ZERO);
    let s = web(&w, h);
    assert_eq!((s.action, s.timer), (Action::BurnFloorWeb, 30));
    assert_eq!(s.actor.home_pos, Vec3::new(t.x, p.y, t.z));
    assert!(w.flags.get_switch(0x06));
}

/// A destroyed web isn't spawned again: back in room 0 with flags 5 and 0x0A set, its floor web
/// and the web over room 10's door kill themselves in their init; the web over room 1's door is
/// there.
#[test]
fn a_destroyed_web_stays_gone() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_room(&a, 3);
    assert!(find(&w, 0x05).is_none() && find(&w, 0x16).is_none(), "room 0's webs went with it");
    w.flags.set_switch(0x05);
    w.flags.set_switch(0x0A);
    change_room(&mut w, 0);
    idle(&mut w, 1);
    assert!(find(&w, 0x05).is_none());
    assert!(find(&w, 0x0A).is_none());
    assert!(find(&w, 0x16).is_some());
    // And the only floor web's collision left is none: room 0's was deleted.
    let webs: Vec<_> = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<BgYdanSp>(h).is_some()).collect();
    assert_eq!(webs.len(), 1);
}
