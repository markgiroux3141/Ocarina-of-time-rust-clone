//! GAME-07 milestone 6 against the C: the ReDeads of the royal tomb's room 1 (`z_en_rd.c`;
//! `hakaana_ouke_scene`, three at params 0x7F00) and their hooks into Player (`func_80852F38`'s
//! grab, `Player_Action_8084F308`'s struggle, `Player_SetAutoLockOnActor`, `play->damagePlayer`).
//!
//! Child Link enters the tomb with `kakariko-letter` (the Kokiri Sword and the Deku Shield, three
//! hearts) or `ocarina-songs` (the Sun's Song), room 1 is loaded and he's placed in it. The
//! ReDeads' homes and facings (`ootx scene-info --scene hakaana_ouke_scene --layer 0`): A at
//! (130, 0, 177) facing -x (-0x4000), B at (215, 0, 446) facing +z (0), C at (-196, 0, 428) facing
//! +x (0x4000). Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_B, BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_CUP, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_rd::*;
use oot_actors::player::{self as pl, Player};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_REACT_TO_LENS, COLORFILTER_COLORFLAG_GRAY, COLORFILTER_COLORFLAG_RED};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorHandle};
use oot_game::clock::{SUNSSONG_INACTIVE, SUNSSONG_START};
use glam::Mat4;
use oot_game::actor_ctx::ActorImpl;
use oot_game::collision_check::AC_HIT;
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// The three homes, in the order the tests name them A, B, C.
const HOME_A: Vec3 = Vec3::new(130.0, 0.0, 177.0);
const HOME_B: Vec3 = Vec3::new(215.0, 0.0, 446.0);
const HOME_C: Vec3 = Vec3::new(-196.0, 0.0, 428.0);

/// Away from all three (over 300 from each home), on room 1's floor (x -300 to 300, z -200 to
/// 700 at 0; the stairs up to the door from z 750).
const AWAY: Vec3 = Vec3::new(0.0, 0.0, 700.0);

/// Child Link entered at the royal tomb (`ENTR_ROYAL_FAMILYS_TOMB_0`) at 22:00 with `preset`, the
/// audio side running and logging; room 1 loaded and Link placed at `link` facing `yaw`; once the
/// three ReDeads are in (their object loads with the room), returns them as [A, B, C].
fn tomb(a: &Arc<GameAssets>, preset: &str, link: Vec3, yaw: i16) -> (PlayState, [ActorHandle; 3]) {
    let e = a.scenes.entrance_index("ENTR_ROYAL_FAMILYS_TOMB_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(22, 0) as u16);
    save.apply_preset(preset).unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    idle(&mut w, 20);
    w.room_request(1);
    idle(&mut w, 1);
    w.room_change_done();
    w.place_player(link, yaw);
    for _ in 0..20 {
        let all = redeads(&w);
        if all.len() == 3 {
            let near = |home: Vec3| *all.iter().find(|&&h| rd(&w, h).actor.home_pos == home).expect("a ReDead at its home");
            let hs = [near(HOME_A), near(HOME_B), near(HOME_C)];
            return (w, hs);
        }
        idle(&mut w, 1);
    }
    panic!("no ReDeads in room 1");
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// `b` pressed for one frame, then let go.
fn tap(w: &mut PlayState, b: u16) {
    let pad = PadState { button: b, ..Default::default() };
    w.tick_with(scripted_input(PadState::default(), pad));
    w.tick_with(scripted_input(pad, PadState::default()));
}

fn redeads(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnRd>(h).is_some_and(|r| !r.actor.killed)).collect()
}

fn rd(w: &PlayState, h: ActorHandle) -> &EnRd {
    w.actors.downcast::<EnRd>(h).expect("En_Rd")
}

fn rd_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnRd {
    w.actors.downcast_mut::<EnRd>(h).expect("En_Rd")
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// `EnRd_Draw`'s commands for `h` now.
fn draw(w: &PlayState, h: ActorHandle) -> DrawOut {
    let r = rd(w, h);
    let mut out = DrawOut::default();
    r.draw(&r.render_state(), w, &ViewInfo::new(Vec3::ZERO, Mat4::IDENTITY), &mut out);
    out
}

fn bake_key(gibdo: bool, xlu: bool) -> eng_gfx::MeshKey {
    eng_gfx::MeshKey::named(oot_game::pack::keys::bake(&bake_name(gibdo, xlu)))
}

fn anim_name(r: &EnRd) -> &str {
    r.skel.animation.as_ref().map(|a| a.name.as_str()).unwrap_or("")
}

#[test]
fn redeads_and_gibdos_initialise_by_params() {
    let Some(a) = assets() else { return };
    let (mut w, hs) = tomb(&a, "kakariko-letter", AWAY, 0);
    // The tomb's 0x7F00: rdFlags 0x7F (PARAMS_GET_S(params, 8, 8)), bit 7 clear so params & 0xFF:
    // type 0 (REDEAD_TYPE_DOES_NOT_MOURN_IF_WALKING); gRedeadSkel (params >= -1), NAVI_ENEMY_REDEAD;
    // 8 health, MASS_HEAVY; sInitChain: lockOnArrowOffset 2000, scale 0.01, gravity -3.5; alpha and
    // unk_31D 255; EnRd_SetupIdle (params >= -2).
    for (h, home, yaw) in [(hs[0], HOME_A, -0x4000), (hs[1], HOME_B, 0), (hs[2], HOME_C, 0x4000)] {
        let r = rd(&w, h);
        assert_eq!((r.actor.params, r.rd_flags, r.gibdo, r.actor.navi_enemy_id), (REDEAD_TYPE_DOES_NOT_MOURN_IF_WALKING, 0x7F, false, NAVI_ENEMY_REDEAD));
        assert_eq!((r.actor.col_chk_info.health, r.actor.col_chk_info.mass, r.actor.target_arrow_offset, r.actor.scale.x, r.actor.gravity), (8, 0xFE, 2000.0, 0.01, -3.5));
        assert_eq!((r.alpha, r.unk_31d, r.action, r.func), (255, 255, REDEAD_ACTION_IDLE, Func::Idle));
        assert_eq!((r.actor.home_pos, r.actor.shape_rot.y), (home, yaw));
        assert_eq!(anim_name(r), "gGibdoRedeadIdleAnim");
        // sCylinderInit: 20 by 70.
        assert_eq!((r.collider.dim.radius, r.collider.dim.height), (20, 70));
        assert_eq!(r.actor.flags & ACTOR_FLAG_REACT_TO_LENS, 0);
        // Idle, Link 400 away: its timer (5 to 14 at the setup, then 10 to 19) never out of range.
        assert!((1..20).contains(&r.timer), "{}", r.timer);
    }
    // Spawned: 0xFD (bit 7: params |= 0xFF00, -3) a Gibdo rising (flat, pitch -0x4000, no
    // gravity, held at the idle's frame 0); 0xFE (-2) a Gibdo idling; 0xFF (-1) a ReDead; 0x02 one
    // crying (gGibdoRedeadSobbingAnim); 0x03 the invisible one (ACTOR_FLAG_REACT_TO_LENS); 0x8001
    // rdFlags 0x80 (no freeze), type 1.
    let at = |i: f32| AWAY + Vec3::new(-200.0 + i * 60.0, 0.0, -150.0);
    let spawned: Vec<(u16, ActorHandle)> =
        [0x00FDu16, 0x00FE, 0x00FF, 0x0002, 0x0003, 0x8001].iter().enumerate().map(|(i, &p)| (p, w.actor_spawn(ACTOR_EN_RD, at(i as f32), [0; 3], p as i16).unwrap())).collect();
    idle(&mut w, 2);
    let get = |w: &PlayState, p: u16| rd(w, spawned.iter().find(|s| s.0 == p).unwrap().1).actor.clone();
    let rising = rd(&w, spawned[0].1);
    assert_eq!((rising.actor.params, rising.gibdo, rising.actor.navi_enemy_id, rising.func, rising.action), (REDEAD_TYPE_GIBDO_RISING_OUT_OF_COFFIN, true, NAVI_ENEMY_GIBDO, Func::RiseFromCoffin, REDEAD_ACTION_RISE_FROM_COFFIN));
    assert_eq!((rising.actor.shape_rot.x, rising.actor.gravity, rising.skel.play_speed, rising.skel.cur_frame), (-0x4000, 0.0, 0.0, 0.0));
    let gibdo = rd(&w, spawned[1].1);
    assert_eq!((gibdo.actor.params, gibdo.gibdo, gibdo.func), (REDEAD_TYPE_GIBDO, true, Func::Idle));
    let never = rd(&w, spawned[2].1);
    assert_eq!((never.actor.params, never.gibdo, never.actor.navi_enemy_id), (REDEAD_TYPE_DOES_NOT_MOURN, false, NAVI_ENEMY_REDEAD));
    let crying = rd(&w, spawned[3].1);
    assert_eq!((crying.actor.params, crying.func), (REDEAD_TYPE_CRYING, Func::Idle));
    assert!(anim_name(crying) == "gGibdoRedeadSobbingAnim" || anim_name(crying) == "gGibdoRedeadWipingTearsAnim", "{}", anim_name(crying));
    assert_ne!(get(&w, 0x0003).flags & ACTOR_FLAG_REACT_TO_LENS, 0);
    let quiet = rd(&w, spawned[5].1);
    assert_eq!((quiet.actor.params, quiet.rd_flags), (REDEAD_TYPE_REGULAR, 0x80));
    // The rising Gibdo: up to 50 over its home (Math_SmoothStepToF 0.3, 2, 0.3), forward 6
    // frames, then standing (0x7D0 a frame) and down to its home, idle with gravity back.
    let rising_h = spawned[0].1;
    let home_y = rd(&w, rising_h).actor.home_pos.y;
    let mut top = home_y;
    for _ in 0..200 {
        idle(&mut w, 1);
        let r = rd(&w, rising_h);
        top = top.max(r.actor.world_pos.y);
        if r.func == Func::Idle {
            break;
        }
    }
    let r = rd(&w, rising_h);
    assert_eq!((r.func, r.actor.shape_rot.x, r.actor.gravity), (Func::Idle, 0, -3.5));
    assert_eq!(top, home_y + 50.0);
    // The four bakes are in the pack.
    for gibdo in [false, true] {
        for xlu in [false, true] {
            let k = oot_game::pack::keys::bake(&bake_name(gibdo, xlu));
            let _: eng_gfx::DrawList = a.pack.assets.get(&k).unwrap_or_else(|e| panic!("{k}: {e:#}"));
        }
    }
}

#[test]
fn link_standing_quietly_near_one_doesnt_wake_it_a_swing_does() {
    let Some(a) = assets() else { return };
    // 100 in front of B (it faces +z), facing it (yaw 0x8000).
    let (mut w, [_, b, _]) = tomb(&a, "kakariko-letter", HOME_B + Vec3::new(0.0, 0.0, 100.0), -0x8000);
    // EnRd_Idle reacts within 150 only to func_8002DDE4 (PLAYER_STATE2_3: a footstep faster than
    // 4, a swing, the sword drawn): standing there, nothing for 100 frames.
    idle(&mut w, 100);
    assert_eq!((rd(&w, b).func, rd(&w, b).action), (Func::Idle, REDEAD_ACTION_IDLE));
    // B: the swing (Player_UpdateCommon's func_808328EC, NA_SE_IT_SWORD_SWING): on its frame,
    // EnRd_SetupAttemptPlayerFreeze (the look back held), and EnRd_TurnTowardsPlayer.
    let pad = PadState { button: BTN_B, ..Default::default() };
    let mut woke = None;
    for i in 0..10 {
        w.tick_with(scripted_input(if i == 0 { PadState::default() } else { pad }, if i == 0 { pad } else { PadState::default() }));
        if rd(&w, b).func != Func::Idle {
            woke = Some(i);
            break;
        }
    }
    let woke = woke.expect("woken by the swing");
    println!("woke {woke} frames after B");
    assert_ne!(player(&w).state2 & pl::STATE2_3, 0, "Link's noise this frame");
    assert_eq!((rd(&w, b).func, rd(&w, b).action, anim_name(rd(&w, b))), (Func::AttemptPlayerFreeze, REDEAD_ACTION_ATTEMPT_PLAYER_FREEZE, "gGibdoRedeadLookBackAnim"));
    // Link right in front (yaw within 0x2008): the scream next frame: Player's freezeTimer 60,
    // Player_SetAutoLockOnActor (focusActor and autoLockOnActor the ReDead,
    // PLAYER_STATE1_FRIENDLY_ACTOR_FOCUS), NA_SE_EN_REDEAD_AIM, then walking at him (0.4).
    idle(&mut w, 1);
    let f = w.audio.frames;
    let p = player(&w);
    assert_eq!((p.actor.freeze_timer, p.focus_actor, p.auto_lock_on_actor), (60, Some(b), Some(b)));
    assert_ne!(p.state1 & pl::STATE1_16, 0);
    assert_eq!((rd(&w, b).func, rd(&w, b).actor.speed_xz), (Func::WalkToPlayer, 0.4));
    assert_eq!(sfx_frames(&w, NA_SE_EN_REDEAD_AIM), vec![f]);
}

#[test]
fn the_scream_freezes_link_every_60_frames_then_the_grab_holds_him_until_he_struggles_free() {
    let Some(a) = assets() else { return };
    // 140 in front of B (the swing out of reach, even after 40 frames' closing in).
    let (mut w, [_, b, _]) = tomb(&a, "kakariko-letter", HOME_B + Vec3::new(0.0, 0.0, 140.0), -0x8000);
    // Ten hearts, so the bites don't take the last of them (that frees Link: Player_InflictDamage).
    w.save.health_capacity = 0xA0;
    w.save.health = 0xA0;
    idle(&mut w, 5);
    tap(&mut w, BTN_B);
    for _ in 0..10 {
        if rd(&w, b).func == Func::WalkToPlayer {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(rd(&w, b).func, Func::WalkToPlayer);
    let start = w.audio.frames;
    // EnRd_AttemptPlayerFreeze's scream: freezeTimer 60.
    assert_eq!(player(&w).actor.freeze_timer, 60);
    assert_eq!(sfx_frames(&w, NA_SE_EN_REDEAD_AIM), vec![start]);
    // EnRd_WalkToPlayer: Link within 150 and 0x1554 of its facing, playerStunWaitTimer 0: the
    // scream again at once (freezeTimer 40 over the 60's remains, the wait 60), then each time
    // the wait runs out (counted down before the action: 60 frames apart), while it closes in at
    // 0.4 a frame. Player is frozen 40 of each 60.
    let mut screams = Vec::new();
    let health0 = w.save.health;
    let mut grabbed = None;
    for _ in 0..400 {
        idle(&mut w, 1);
        // Frozen from frame 1 (40: Player skips 39 updates), the lock-on held; Player's first
        // update after: autoLockOnActor still set (Player_UpdateZTargeting: zTargetActiveTimer 5,
        // the focus kept), then cleared at its end; the next update lets the lock-on go
        // (Attention_ShouldReleaseLockOn: ~124 off, past ATTENTION_RANGE_0's leash, 99 by its scale).
        let n = w.audio.frames - start;
        let p = player(&w);
        if (1..41).contains(&n) {
            assert_eq!((p.actor.freeze_timer as u32, p.focus_actor), (41 - n, Some(b)));
        } else if n == 41 {
            assert_eq!((p.focus_actor, p.auto_lock_on_actor, p.z_target_active_timer), (Some(b), None, 5));
        } else if n == 42 {
            assert_eq!(p.focus_actor, None);
        }
        if sfx_frames(&w, NA_SE_EN_REDEAD_AIM).contains(&w.audio.frames) {
            screams.push((w.audio.frames - start, player(&w).actor.freeze_timer, rd(&w, b).player_stun_wait_timer));
        }
        if rd(&w, b).func == Func::Grab {
            grabbed = Some(w.audio.frames - start);
            break;
        }
    }
    println!("screams {screams:?}, grabbed at {grabbed:?}");
    assert!(screams.len() >= 2);
    assert_eq!(screams[0].0, 1);
    for s in &screams {
        assert_eq!((s.1, s.2), (40, 60), "{screams:?}");
    }
    for p in screams.windows(2) {
        assert_eq!(p[1].0 - p[0].0, 60, "{screams:?}");
    }
    // The grab: within 45 and facing Link (0x38E3), grabWaitTimer 0: freezeTimer 0 and
    // play->grabPlayer: Link in Player_Action_8084F308 with PLAYER_STATE2_7, the ReDead
    // untargetable, gGibdoRedeadGrabStartAnim, grabDamageTimer 200.
    let g0 = w.audio.frames;
    assert!(grabbed.is_some());
    let r = rd(&w, b);
    assert!(r.actor.world_pos.distance(player(&w).actor.world_pos) <= 45.0 + 0.5);
    assert_eq!((r.action, r.grab_state, r.grab_damage_timer, anim_name(r)), (REDEAD_ACTION_GRAB, REDEAD_GRAB_START, 200, "gGibdoRedeadGrabStartAnim"));
    assert_eq!(r.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    let p = player(&w);
    assert_eq!((p.action, p.actor.freeze_timer), (pl::Action::Grabbed, 0));
    assert_ne!(p.state2 & pl::STATE2_7, 0);
    assert_eq!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_VO_LI_HELD + p.age.climb.unk_92).len(), 1);
    assert_eq!(w.save.health, health0);
    // The grab's start played out (SkelAnime_Once is true on the frame after it reached its last
    // frame: last + 1 frames in): REDEAD_GRAB_INITIAL_DAMAGE: 8 off (half a heart), the bite
    // looping, grabDamageTimer 20, and falling through to REDEAD_GRAB_ATTACK's count (19 at once):
    // the next 19 frames on, then every 20; Link's NA_SE_VO_LI_DAMAGE_S (his age's voice) with each
    // after the first (the ReDead's Player_PlaySfx); the ReDead sinking to -1500 for a child (150 a
    // frame).
    let mut bites = Vec::new();
    let mut last = w.save.health;
    for _ in 0..70 {
        idle(&mut w, 1);
        if w.save.health != last {
            bites.push((w.audio.frames - g0, last - w.save.health));
            last = w.save.health;
        }
    }
    println!("bites {bites:?}");
    let start_last = w.assets.as_ref().unwrap().animation(OBJECT, "gGibdoRedeadGrabStartAnim").unwrap().last_frame() as u32;
    assert_eq!(bites[0], (start_last + 1, 8));
    assert_eq!((bites[1].0 - bites[0].0, bites[1].1), (19, 8));
    for p in bites[1..].windows(2) {
        assert_eq!((p[1].0 - p[0].0, p[1].1), (20, 8), "{bites:?}");
    }
    let r = rd(&w, b);
    assert_eq!((r.grab_state, r.action, r.actor.shape_y_offset, anim_name(r)), (REDEAD_GRAB_ATTACK, REDEAD_ACTION_GRAB, -1500.0, "gGibdoRedeadGrabAttackAnim"));
    assert_eq!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_VO_LI_DAMAGE_S + player(&w).age.climb.unk_92).len(), bites.len() - 1);
    // Mashing A (func_80832594(0, 100): 5 a press, past 100): the 21st press frees Link
    // (func_80839F90, PLAYER_STATE2_7 off); the ReDead lets go (gGibdoRedeadGrabEndAnim at half
    // speed, REDEAD_ACTION_WALK_TO_PLAYER_OR_RELEASE_GRAB, still EnRd_Grab).
    let mut presses = 0;
    while player(&w).state2 & pl::STATE2_7 != 0 {
        assert!(presses < 30, "never free");
        tap(&mut w, BTN_A);
        presses += 1;
    }
    println!("free after {presses} presses");
    assert_eq!(presses, 21);
    assert_ne!(player(&w).action, pl::Action::Grabbed);
    let r = rd(&w, b);
    assert_eq!((r.func, r.action, r.grab_state, anim_name(r)), (Func::Grab, REDEAD_ACTION_WALK_TO_PLAYER_OR_RELEASE_GRAB, REDEAD_GRAB_RELEASE, "gGibdoRedeadGrabEndAnim"));
    // The release's end: REDEAD_GRAB_END: targetable again, 10 frames before a scream and 15
    // before a grab, walking at Link.
    for _ in 0..100 {
        if rd(&w, b).func != Func::Grab {
            break;
        }
        idle(&mut w, 1);
    }
    let r = rd(&w, b);
    assert_eq!((r.func, r.player_stun_wait_timer, r.grab_wait_timer), (Func::WalkToPlayer, 10, 0xF));
    assert_ne!(r.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
}

#[test]
fn sword_hits_hurt_it_and_its_death_brings_the_others_to_mourn() {
    let Some(a) = assets() else { return };
    // Behind B (it faces +z), 35 back, facing it: the swing hits before its turn (500 a frame for
    // the head and the upper body each) brings Link within 0x2008.
    let (mut w, [ra, b, rc]) = tomb(&a, "kakariko-letter", HOME_B - Vec3::new(0.0, 0.0, 35.0), 0);
    idle(&mut w, 5);
    // EnRd_Draw at alpha 255: the opaque skeleton (ACTOR_SETUP_OPA_DL, env alpha 255), and the
    // circle shadow's stand-in.
    let out = draw(&w, b);
    assert_eq!((out.opa.len(), out.xlu.len()), (1, 1));
    assert_eq!(out.opa[0].mesh, bake_key(false, false));
    let mut hit = None;
    let pad = PadState { button: BTN_B, ..Default::default() };
    for i in 0..20 {
        w.tick_with(scripted_input(if i == 0 { PadState::default() } else { pad }, if i == 0 { pad } else { PadState::default() }));
        if rd(&w, b).actor.col_chk_info.health < 8 {
            hit = Some(i);
            break;
        }
    }
    assert!(hit.is_some(), "the Kokiri Sword's hit");
    // The Kokiri Sword: 1 (REDEAD_DMG_REACT_DAMAGE): red 8 frames, EnRd_SetupDamaged
    // (gGibdoRedeadDamageAnim, NA_SE_EN_REDEAD_DAMAGE).
    let r = rd(&w, b);
    assert_eq!((r.actor.col_chk_info.health, r.func, r.action, r.damage_reaction), (7, Func::Damaged, REDEAD_ACTION_DAMAGED, REDEAD_DMG_REACT_DAMAGE));
    assert_eq!(r.actor.color_filter_params & 0xC000, COLORFILTER_COLORFLAG_RED);
    assert_eq!(sfx_frames(&w, NA_SE_EN_REDEAD_DAMAGE), vec![w.audio.frames]);
    // At 1 health, the next swing that lands kills it: EnRd_UpdateMourningTarget (A and C, type 0,
    // mourn: their parent B), EnRd_SetupDead (gGibdoRedeadDeathAnim, timer 300, untargetable,
    // NA_SE_EN_REDEAD_DEAD), its drop from table 9.
    idle(&mut w, 30);
    rd_mut(&mut w, b).actor.col_chk_info.health = 1;
    let mut killed = false;
    for _ in 0..6 {
        tap(&mut w, BTN_B);
        for _ in 0..12 {
            idle(&mut w, 1);
            if rd(&w, b).actor.col_chk_info.health == 0 {
                killed = true;
                break;
            }
        }
        if killed {
            break;
        }
    }
    assert!(killed, "the killing blow ({:?})", rd(&w, b).func);
    let r = rd(&w, b);
    assert_eq!((r.func, r.action, r.timer, anim_name(r)), (Func::Dead, REDEAD_ACTION_DEAD, 300, "gGibdoRedeadDeathAnim"));
    assert_eq!(r.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(sfx_frames(&w, NA_SE_EN_REDEAD_DEAD).len(), 1);
    assert_eq!((w.actors.actor(ra).unwrap().parent, w.actors.actor(rc).unwrap().parent), (Some(b), Some(b)));
    // EnRd_Dead: a prop from its next update; A and C, idle with a parent and not mourning:
    // EnRd_SetupWalkToParent (isMourning), at 0.4 to within 45 of it, then idle.
    // (The sword's hit stops every actor for a frame: Player's func_80832630, freezeFlashTimer 1.)
    for _ in 0..3 {
        if [ra, rc].iter().all(|&h| rd(&w, h).func == Func::WalkToParent) {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(rd(&w, b).actor.category, ACTORCAT_PROP);
    assert!(w.actors.category(ACTORCAT_PROP).contains(&b));
    for h in [ra, rc] {
        assert_eq!((rd(&w, h).func, rd(&w, h).is_mourning), (Func::WalkToParent, true));
    }
    // Lying 300 frames once the death's played, then its switch flag (rdFlags & 0x7F, 0x7F: the
    // shift's wrap lands on temp flag 0x3F), fading 5 a frame and sinking 0.000075: at alpha 180
    // A and C let go (parent NULL: from idle, mourning, EnRd_SetupAttemptPlayerFreeze; walking,
    // EnRd_SetupWalkToPlayer); at 0 it's killed.
    let mut let_go = None;
    let mut gone = None;
    for n in 0..800 {
        idle(&mut w, 1);
        if let_go.is_none() && w.actors.actor(ra).unwrap().parent.is_none() {
            let_go = Some((n, rd(&w, b).alpha));
            assert!(w.flags.get_switch(0x3F) && w.flags.get_switch(0x7F));
            // Fading: the translucent skeleton (gActorSetupXluDL) at its alpha, scale.y down
            // 0.000075 a step (16 steps from 255 to 175).
            let out = draw(&w, b);
            assert_eq!((out.opa.len(), out.xlu.len()), (0, 2));
            assert_eq!(out.xlu[0].mesh, bake_key(false, true));
            assert_eq!(out.xlu[0].params.segments.as_ref().unwrap().env[0x0E], Some([0, 0, 0, 175]));
            assert!((rd(&w, b).actor.scale.y - (0.01 - 16.0 * 0.000075)).abs() < 1e-6);
        }
        if redeads(&w).len() == 2 {
            gone = Some(n);
            break;
        }
    }
    println!("let go {let_go:?}, gone {gone:?}");
    assert_eq!(let_go.unwrap().1, 175);
    assert!(gone.is_some());
    assert!(w.actors.actor(rc).unwrap().parent.is_none());
    // Neither had reached the corpse (282 and 411 off, at 0.4 a frame): EnRd_WalkToParent with
    // no parent: EnRd_SetupWalkToPlayer, isMourning kept (type 0 doesn't clear it walking); Link
    // over 150 from their homes: EnRd_SetupWalkToHome.
    for h in [ra, rc] {
        let r = rd(&w, h);
        assert_eq!((r.func, r.is_mourning), (Func::WalkToHome, true));
    }
}

/// The live soft sprites of type `ty`.
fn effects(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == ty).count()
}

#[test]
fn dins_fire_burns_it_with_ten_flames() {
    let Some(a) = assets() else { return };
    let (mut w, [_, b, _]) = tomb(&a, "kakariko-letter", AWAY, 0);
    // A Din's Fire hit as CollisionCheck leaves it (AC_HIT, damage 4, REDEAD_DMG_REACT_FIRE_MAGIC):
    // red 80 frames, fireTimer 40, 4 off, EnRd_SetupDamaged.
    {
        let r = rd_mut(&mut w, b);
        r.collider.base.ac_flags |= AC_HIT;
        r.actor.col_chk_info.damage = 4;
        r.actor.col_chk_info.damage_reaction = REDEAD_DMG_REACT_FIRE_MAGIC;
    }
    idle(&mut w, 1);
    let r = rd(&w, b);
    assert_eq!((r.actor.col_chk_info.health, r.func, r.actor.color_filter_params & 0xC000), (4, Func::Damaged, COLORFILTER_COLORFLAG_RED));
    // EnRd_Draw (each frame, at full alpha): the colour kept on (colorFilterTimer++), fireTimer
    // down 1, and at each multiple of 4 a flame (EffectSsEnFire_SpawnVec3s, scale 0x4B) on
    // firePos[fireTimer >> 2], recorded by EnRd_PostLimbDraw: 10 in all, from body part 9 (left
    // foot) to 0 (head), each with NA_SE_EV_FLAME_IGNITION.
    let fire = oot_game::effect::EFFECT_SS_EN_FIRE;
    let first = w.audio.frames;
    let mut most = 0;
    for _ in 0..45 {
        idle(&mut w, 1);
        most = most.max(effects(&w, fire));
    }
    let r = rd(&w, b);
    assert_eq!(r.fire_timer, 0);
    let ignitions: Vec<u32> = sfx_frames(&w, oot_game::audio::sfx::NA_SE_EV_FLAME_IGNITION).into_iter().filter(|&f| f > first).collect();
    println!("flames at {:?}, at most {most} alive; head at {:?}, root {:?}", ignitions.iter().map(|f| f - first).collect::<Vec<_>>(), r.fire_pos[0], r.fire_pos[1]);
    assert_eq!(ignitions.len(), 10);
    assert!(ignitions.windows(2).all(|p| p[1] - p[0] == 4));
    assert!(most >= 1);
    // The parts are about the body: the head above the root.
    assert!(r.fire_pos[0][1] > r.fire_pos[1][1]);
}

#[test]
fn a_mourner_idle_with_no_parent_screams_from_anywhere() {
    let Some(a) = assets() else { return };
    let (mut w, [ra, _, _]) = tomb(&a, "kakariko-letter", AWAY, 0);
    // A mourner whose dead one faded before it got there keeps isMourning (EnRd_WalkToParent
    // with no parent walks at Link; type 0 clears it only from idle); back home and idle with no
    // parent: EnRd_SetupAttemptPlayerFreeze at once, with no noise and Link 538 off; turned to
    // him (EnRd_TurnTowardsPlayer), the scream: Link frozen 60 and locked on, across the room.
    rd_mut(&mut w, ra).is_mourning = true;
    idle(&mut w, 1);
    let r = rd(&w, ra);
    assert_eq!((r.func, r.is_mourning), (Func::AttemptPlayerFreeze, false));
    assert!(r.actor.xz_dist_to_player > 500.0);
    let mut n = 0;
    while rd(&w, ra).func == Func::AttemptPlayerFreeze {
        assert!(n < 60, "never turned");
        idle(&mut w, 1);
        n += 1;
    }
    println!("A screamed after {n} frames' turn");
    assert_eq!(rd(&w, ra).func, Func::WalkToPlayer);
    assert_eq!((player(&w).actor.freeze_timer, player(&w).focus_actor), (60, Some(ra)));
}

#[test]
fn it_walks_home_with_link_150_from_its_home_and_comes_back_within() {
    let Some(a) = assets() else { return };
    let (mut w, [_, b, _]) = tomb(&a, "kakariko-letter", HOME_B + Vec3::new(0.0, 0.0, 120.0), -0x8000);
    idle(&mut w, 5);
    tap(&mut w, BTN_B);
    for _ in 0..10 {
        if rd(&w, b).func == Func::WalkToPlayer {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(rd(&w, b).func, Func::WalkToPlayer);
    idle(&mut w, 60);
    let moved = rd(&w, b).actor.world_pos;
    assert!(moved.distance(HOME_B) > 5.0, "walked at Link: {moved:?}");
    // Link 150 or more from its home (Actor_WorldDistXYZToPoint): EnRd_SetupWalkToHome (the walk
    // at half speed, its speed kept), turning home 0x1C2 a frame; within 5, stopped and turned to
    // its home's facing, then EnRd_SetupIdle.
    w.place_player(Vec3::new(-100.0, 0.0, 650.0), -0x8000);
    idle(&mut w, 1);
    assert_eq!((rd(&w, b).func, rd(&w, b).action, rd(&w, b).skel.play_speed), (Func::WalkToHome, REDEAD_ACTION_WALK_TO_HOME, 0.5));
    let mut home_at = None;
    for n in 0..1000 {
        idle(&mut w, 1);
        if rd(&w, b).func == Func::Idle {
            home_at = Some(n);
            break;
        }
    }
    let r = rd(&w, b);
    println!("home after {home_at:?} at {:?}", r.actor.world_pos);
    assert!(home_at.is_some());
    assert!(r.actor.world_pos.distance(HOME_B) < 5.0);
    assert_eq!((r.actor.shape_rot.y, r.actor.speed_xz), (0, 0.0));
    // Walking home, Link back within 150 of its home: at him again. (From idle it needs a noise.)
    rd_mut(&mut w, b).func = Func::WalkToHome;
    w.place_player(HOME_B + Vec3::new(0.0, 0.0, 140.0), -0x8000);
    idle(&mut w, 1);
    assert_eq!(rd(&w, b).func, Func::WalkToPlayer);
}

/// A pad button of an `OCARINA_BTN_*`.
fn pad_button(b: u8) -> u16 {
    [BTN_A, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT, BTN_CUP][b as usize]
}

#[test]
fn the_suns_song_stuns_every_redead_in_the_room_for_600_frames() {
    let Some(a) = assets() else { return };
    // Link away from all three (over 400 from each home), the Sun's Song played on the Fairy
    // Ocarina.
    let (mut w, hs) = tomb(&a, "ocarina-songs", AWAY, -0x8000);
    idle(&mut w, 5);
    let pad = PadState { button: BTN_CLEFT, ..Default::default() };
    w.tick_with(scripted_input(PadState::default(), pad));
    w.tick_with(scripted_input(pad, PadState::default()));
    for _ in 0..40 {
        if w.msg_ctx.msg_mode == oot_game::message::MSGMODE_OCARINA_PLAYING {
            break;
        }
        idle(&mut w, 1);
    }
    let b = w.audio.ocarina.song_buttons(oot_game::audio::ocarina::OCARINA_SONG_SUNS);
    let notes: Vec<u16> = b.buttons_index[..b.num_buttons as usize].iter().map(|&i| pad_button(i)).collect();
    for n in notes {
        let p = PadState { button: n, ..Default::default() };
        w.tick_with(scripted_input(PadState::default(), p));
        w.tick_with(scripted_input(p, p));
        w.tick_with(scripted_input(p, p));
        w.tick_with(scripted_input(p, PadState::default()));
        idle(&mut w, 1);
    }
    // OceffSpot_End: SUNSSONG_START (the tomb's restriction 3 then keeps it SUNSSONG_SPECIAL in
    // Interface_Update). EnRd_UpdateDamage, on each ReDead's next update (standing, not hurt,
    // dying or stunned), wherever it is: EnRd_SetupStunned: stunnedBySunsSong, 600 frames,
    // NA_SE_EN_LIGHT_ARROW_HIT, grey (COLORFILTER_INTENSITY_FLAG | 200, 255 frames) kept at 255
    // while 255 or more remain; the first stunned doesn't clear the state, so all three are.
    let mut stunned_at = None;
    for n in 0..800 {
        idle(&mut w, 1);
        if hs.iter().any(|&h| rd(&w, h).stunned_by_suns_song) {
            stunned_at = Some(n);
            break;
        }
    }
    assert!(stunned_at.is_some(), "the song's stun (state {})", w.save.suns_song_state);
    let s = w.audio.frames;
    assert_ne!(w.save.suns_song_state, SUNSSONG_INACTIVE);
    for &h in &hs {
        let r = rd(&w, h);
        assert_eq!((r.func, r.action, r.stunned_by_suns_song, r.suns_song_stun_timer), (Func::Stunned, REDEAD_ACTION_STUNNED, true, 599));
        assert_eq!((r.actor.color_filter_params & 0xC000, r.actor.color_filter_timer), (COLORFILTER_COLORFLAG_GRAY, 255));
    }
    assert_eq!(sfx_frames(&w, NA_SE_EN_LIGHT_ARROW_HIT), vec![s; 3]);
    // 600 frames in all (the stun's first frame counted): the grey runs out (255 frames after
    // its last reset at 255 left) on the frame the timer reaches 0: the song's state
    // SUNSSONG_INACTIVE, and each EnRd_SetupDamaged (no health lost).
    idle(&mut w, 598);
    assert!(hs.iter().all(|&h| rd(&w, h).func == Func::Stunned));
    assert_ne!(w.save.suns_song_state, SUNSSONG_INACTIVE);
    idle(&mut w, 1);
    assert_eq!(w.save.suns_song_state, SUNSSONG_INACTIVE);
    for &h in &hs {
        let r = rd(&w, h);
        assert_eq!((r.func, r.stunned_by_suns_song, r.actor.col_chk_info.health), (Func::Damaged, false, 8));
    }
}

#[test]
fn a_redead_not_stunned_by_the_song_clears_its_state() {
    let Some(a) = assets() else { return };
    let (mut w, hs) = tomb(&a, "kakariko-letter", AWAY, 0);
    // EnRd_Update: one hurt (REDEAD_ACTION_DAMAGED) isn't stunned and clears sunsSongState at
    // once, so those updating after it in the enemy list (newest first) aren't stunned either.
    let order: Vec<ActorHandle> = w.actors.category(oot_game::actor_ctx::ACTORCAT_ENEMY).iter().copied().filter(|h| hs.contains(h)).collect();
    rd_mut(&mut w, order[1]).action = REDEAD_ACTION_DAMAGED;
    w.save.suns_song_state = SUNSSONG_START;
    idle(&mut w, 1);
    assert_eq!(w.save.suns_song_state, SUNSSONG_INACTIVE);
    assert!(rd(&w, order[0]).stunned_by_suns_song);
    assert!(!rd(&w, order[1]).stunned_by_suns_song);
    assert!(!rd(&w, order[2]).stunned_by_suns_song);
    // EnRd_Destroy clears it too: the stunned one (it keeps the state) killed, the others able to
    // be stunned: its destroy, first in the list, clears it before their updates.
    rd_mut(&mut w, order[1]).action = REDEAD_ACTION_IDLE;
    w.save.suns_song_state = SUNSSONG_START;
    rd_mut(&mut w, order[0]).actor.kill();
    idle(&mut w, 1);
    assert_eq!(w.save.suns_song_state, SUNSSONG_INACTIVE);
    assert!(!rd(&w, order[1]).stunned_by_suns_song && !rd(&w, order[2]).stunned_by_suns_song);
}
