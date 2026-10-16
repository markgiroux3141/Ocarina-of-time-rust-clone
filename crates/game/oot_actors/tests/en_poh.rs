//! `En_Poh` (`z_en_poh.c`) against the C: Dodongo's Cavern's room 4 Poes (Master Quest, three
//! normal ones) on `cavern-open` (a child with the Kokiri Sword, three hearts, Talon's milk
//! bottle full); the composers (`EN_POH_SHARP`, `EN_POH_FLAT`) and the grave's rupee Poes
//! (`EN_POH_RUPEE`) spawned there as `En_Wonder_Talk` and `Bg_Haka` spawn them, their object
//! loaded for them (`Object_SpawnPersistent`).
//!
//! The expectations are worked out from the C in the comments. A Poe updates only in view once
//! its object is in (`ACTOR_FLAG_UPDATE_CULLING_DISABLED` cleared), so Link is placed with the
//! camera behind him looking its way.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_B, BTN_Z, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_item00::EnItem00;
use oot_actors::en_poh::*;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_MISC, ActorHandle};
use oot_game::collision_check::{AC_HIT, AC_ON, OC1_ON, OC1_TYPE_PLAYER};
use oot_game::effect::{EFFECT_SS_DEAD_DB, EFFECT_SS_HAHEN};
use oot_game::item::{ITEM_BOTTLE_EMPTY, ITEM_BOTTLE_MILK_FULL, ITEM_BOTTLE_POE, SLOT_BOTTLE_1, item_give};
use oot_game::lights::{LIGHT_POINT_GLOW, LIGHT_POINT_NOGLOW};
use oot_game::message::{TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_EVENT, TEXT_STATE_NONE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `ITEM_SONG_SUN` (`item.h`: 0x63; `ITEM_SONG_LULLABY` 0x60 .. `ITEM_SONG_STORMS` 0x65).
const ITEM_SONG_SUN: u8 = 0x63;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// The room 4 Poes' homes (`ddan_room_4`'s placements): by the north door, the middle's east
/// end, the south.
const HOME_NORTH: Vec3 = Vec3::new(1729.0, 120.0, -858.0);
const HOME_EAST: Vec3 = Vec3::new(2079.0, 120.0, -1210.0);
const HOME_SOUTH: Vec3 = Vec3::new(1706.0, 120.0, -1599.0);
/// Room 4's floor.
const FLOOR: f32 = 80.0;

/// `cavern-open` (edited by `edit`), Link entering the cavern, put in room 4 at `pos` facing
/// `yaw` and the room loaded as walking into it would (its objects, then its actors' inits).
fn room4_with(a: &Arc<GameAssets>, pos: Vec3, yaw: i16, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("cavern-open").unwrap();
    edit(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    w.place_player(pos, yaw);
    assert!(w.room_request(4));
    idle(&mut w, 1);
    w.room_change_done();
    idle(&mut w, 3);
    w
}

fn room4(a: &Arc<GameAssets>, pos: Vec3, yaw: i16) -> PlayState {
    room4_with(a, pos, yaw, |_| {})
}

fn pohs(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnPoh>(h).is_some_and(|p| !p.actor.killed)).collect()
}

fn poh_at(w: &PlayState, home: Vec3) -> ActorHandle {
    pohs(w).into_iter().find(|&h| poh(w, h).actor.home_pos.x == home.x && poh(w, h).actor.home_pos.z == home.z).expect("the Poe at its home")
}

fn poh(w: &PlayState, h: ActorHandle) -> &EnPoh {
    w.actors.downcast::<EnPoh>(h).expect("En_Poh")
}

fn poh_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnPoh {
    w.actors.downcast_mut::<EnPoh>(h).expect("En_Poh")
}

/// Frames until `f` holds (at most `max`).
fn until(w: &mut PlayState, max: usize, what: &str, f: impl Fn(&PlayState) -> bool) -> usize {
    for n in 0..max {
        if f(w) {
            return n;
        }
        idle(w, 1);
    }
    panic!("{what}: not after {max} frames");
}

fn sfx_since(w: &PlayState, from: usize, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx[from..].iter().any(|&(_, s, _)| s == id)
}

fn sfx_len(w: &PlayState) -> usize {
    w.audio.log.as_ref().unwrap().sfx.len()
}

fn effects_of(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == ty).count()
}

/// Link `dist` from `p`, the side `side_yaw` points from it, facing it (or facing away).
fn link_by(p: Vec3, side_yaw: i16, dist: f32, facing: bool) -> (Vec3, i16) {
    let (s, c) = (eng_math::sin_s(side_yaw), eng_math::cos_s(side_yaw));
    let pos = Vec3::new(p.x + s * dist, FLOOR, p.z + c * dist);
    let to_it = side_yaw.wrapping_add(i16::MIN);
    (pos, if facing { to_it } else { side_yaw })
}

/// The Poe at `home`, appeared (Link 250 west of the east one, facing it): to its idle.
fn appeared(a: &Arc<GameAssets>) -> (PlayState, ActorHandle) {
    let (pos, yaw) = link_by(HOME_EAST, -0x4000, 250.0, true);
    let mut w = room4(a, pos, yaw);
    let h = poh_at(&w, HOME_EAST);
    until(&mut w, 60, "appeared", |w| poh(w, h).action == Action::Idle);
    (w, h)
}

#[test]
fn room_4s_poes_wait_unseen_until_link_is_near_then_fade_in() {
    let Some(a) = assets() else { return };
    // Link 300 west of the east Poe: over 280 from each of them.
    let (pos, yaw) = link_by(HOME_EAST, -0x4000, 300.0, true);
    let mut w = room4(&a, pos, yaw);
    let all = pohs(&w);
    assert_eq!(all.len(), 3, "room 4 places three");
    for &h in &all {
        let p = poh(&w, h);
        // EnPoh_Init: params 0 (EN_POH_NORMAL), sPoeInfo[0], 4 health, NAVI_ENEMY_POE.
        // EnPoh_Update: object_poh in, EnPoh_UpdateLiving, EnPoh_DrawRegular, no more update
        // culling off; EnPoh_SetupInitialAction: alpha 0, not targetable, gPoeAppearAnim held.
        assert_eq!((p.actor.params, p.info_idx, p.actor.col_chk_info.health, p.actor.navi_enemy_id), (EN_POH_NORMAL, EN_POH_INFO_NORMAL, 4, NAVI_ENEMY_POE));
        assert_eq!((p.update_fn, p.draw_fn, p.action), (Update::Living, Draw::Regular, Action::Func80ADEF38));
        assert_eq!(p.light_color[3], 0);
        assert_eq!(p.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED), 0);
        assert_eq!((p.skel.play_speed, p.skel.cur_frame, p.skel.anim_length), (0.0, 0.0, 30.0), "the 30-frame appear animation, held");
        assert!((700..1000).contains(&p.visibility_timer), "Rand_S16Offset(700, 300)");
    }
    idle(&mut w, 20);
    assert!(all.iter().all(|&h| poh(&w, h).action == Action::Func80ADEF38 && poh(&w, h).skel.play_speed == 0.0), "Link over 280 away: they wait");

    // Link 250 from the east one (the others are 366 and 408 away).
    let (pos, yaw) = link_by(HOME_EAST, -0x4000, 250.0, true);
    w.place_player(pos, yaw);
    let h = poh_at(&w, HOME_EAST);
    let s = sfx_len(&w);
    idle(&mut w, 1);
    // func_80ADEF38: playSpeed 0 and Link within 280: NA_SE_EN_PO_APPEAR, playSpeed 1.
    assert_eq!(poh(&w, h).skel.play_speed, 1.0);
    assert!(sfx_since(&w, s, NA_SE_EN_PO_APPEAR));
    // Then a frame of the animation an update; from frame 11 the alpha is
    // ((curFrame - 10) * 0.05) * 255, cast to u8; at frame 29 (the last) the next update ends
    // it: alpha 255, Rand_S16Offset(700, 300) visible, targetable, EnPoh_SetupIdle.
    for k in 1..=29 {
        idle(&mut w, 1);
        let p = poh(&w, h);
        assert_eq!(p.skel.cur_frame, k as f32);
        let want = if k > 10 { (((k as f32 - 10.0) * 0.05) * 255.0) as i32 as u8 } else { 0 };
        assert_eq!(p.light_color[3], want, "frame {k}");
        assert_eq!(p.action, Action::Func80ADEF38);
    }
    idle(&mut w, 1);
    let p = poh(&w, h);
    assert_eq!((p.action, p.light_color[3]), (Action::Idle, 255));
    assert!((700..1000).contains(&p.visibility_timer));
    assert_ne!(p.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    for o in [poh_at(&w, HOME_NORTH), poh_at(&w, HOME_SOUTH)] {
        assert_eq!(poh(&w, o).action, Action::Func80ADEF38, "the others still wait");
    }
}

#[test]
fn a_poe_fades_out_when_its_time_is_up_and_back_in_later() {
    let Some(a) = assets() else { return };
    let (mut w, h) = appeared(&a);
    // EnPoh_UpdateVisibility: visibilityTimer down each frame; at 0, visible, in a moving
    // action: EnPoh_SetupDisappear (unk_194 32, NA_SE_EN_PO_DISAPPEAR).
    poh_mut(&mut w, h).visibility_timer = 1;
    let s = sfx_len(&w);
    idle(&mut w, 1);
    assert_eq!(poh(&w, h).action, Action::Disappear);
    assert!(sfx_since(&w, s, NA_SE_EN_PO_DISAPPEAR) && sfx_since(&w, s, NA_SE_EN_PO_LAUGH));
    // EnPoh_Disappear (run the same frame): unk_194 down, alpha unk_194 * 7.96875; at 0,
    // Rand_S16Offset(100, 50) invisible, idle.
    assert_eq!(poh(&w, h).light_color[3], (31.0 * 7.96875) as u8);
    for k in 1..=31u8 {
        idle(&mut w, 1);
        let p = poh(&w, h);
        assert_eq!(p.light_color[3], ((31 - k) as f32 * 7.96875) as u8, "frame {k}");
    }
    let p = poh(&w, h);
    assert_eq!((p.action == Action::Disappear, p.light_color[3]), (false, 0));
    assert!((100..150).contains(&(p.visibility_timer + 0)), "Rand_S16Offset(100, 50): {}", p.visibility_timer);
    assert_eq!(p.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_ATTENTION_ENABLED, "still targetable by its flag");
    // Invisible it's never put in AC (CollisionCheck_SetAC needs alpha 255).
    // The timer out: EnPoh_SetupAppear; 32 frames up at 7.96875 a frame; 700 to 1000 frames.
    poh_mut(&mut w, h).visibility_timer = 1;
    idle(&mut w, 1);
    // (The frame's action may have been an idle one that set up the appear.)
    let p = poh(&w, h);
    assert!(matches!(p.action, Action::Appear), "{:?}", p.action);
    let start = p.unk_194;
    for k in (start + 1)..=32u8 {
        idle(&mut w, 1);
        let p = poh(&w, h);
        if k < 32 {
            assert_eq!(p.light_color[3], (k as f32 * 7.96875) as u8);
        }
    }
    let p = poh(&w, h);
    assert_eq!((p.light_color[3], p.unk_194), (255, 0));
    assert!((700..1000).contains(&p.visibility_timer));
}

#[test]
fn a_poe_locked_on_for_20_frames_disappears() {
    let Some(a) = assets() else { return };
    let (mut w, h) = appeared(&a);
    poh_mut(&mut w, h).visibility_timer = 1000;
    // Z held with the Poe in front: Link's lock-on (isLockedOn); unk_194 counts it to 20.
    let z = PadState { button: BTN_Z, ..Default::default() };
    let mut prev = PadState::default();
    let mut frames = 0;
    for n in 0..60 {
        w.tick_with(scripted_input(prev, z));
        prev = z;
        if poh(&w, h).action == Action::Disappear {
            frames = n;
            break;
        }
    }
    // Locked on from the first frame: unk_194 1 to 20, and on the 20th EnPoh_SetupDisappear.
    assert_eq!(frames, 19, "the lock-on's 20th frame");
}

#[test]
fn its_light_is_at_its_lantern_in_the_flames_colour() {
    let Some(a) = assets() else { return };
    let (pos, yaw) = link_by(HOME_EAST, -0x4000, 300.0, true);
    let w = room4(&a, pos, yaw);
    let h = poh_at(&w, HOME_EAST);
    let p = poh(&w, h);
    let l = w.light_ctx.info(p.light_node.expect("LightContext_InsertLight")).unwrap();
    // While its appear animation is held at frame 0 (func_80AE089C): envColor 55 grey, alpha 0;
    // EnPoh_PostLimbDraw: Lights_PointGlowSetInfo at the lantern's sphere, radius alpha * 200/255.
    let c = p.collider_jnt_sph.elements[0].dim.world_sphere.center;
    assert_eq!((l.ty, [l.x, l.y, l.z], l.color, l.radius), (LIGHT_POINT_GLOW, c, [55, 55, 55], 0));
    let (w, h) = appeared(&a);
    let p = poh(&w, h);
    let l = w.light_ctx.info(p.light_node.unwrap()).unwrap();
    let c = p.collider_jnt_sph.elements[0].dim.world_sphere.center;
    // Flickering: (225 + rand * 30, 155 + rand * 100, 95 + rand * 160), alpha 200: radius
    // 200 * 200/255 = 156.
    assert_eq!((l.ty, [l.x, l.y, l.z], l.radius), (LIGHT_POINT_GLOW, c, 156));
    assert_eq!(l.color, [p.env_color[0], p.env_color[1], p.env_color[2]]);
    assert!(l.color[0] >= 225 && l.color[1] >= 155 && l.color[2] >= 95);
    // The sphere is the lantern's: 1400 up limb 18, near the Poe.
    let d = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32).distance(p.actor.world_pos);
    assert!(d < 60.0, "{d}");
}

#[test]
fn its_lantern_hits_link_for_half_a_heart_and_it_backs_off() {
    let Some(a) = assets() else { return };
    let (mut w, h) = appeared(&a);
    poh_mut(&mut w, h).visibility_timer = 1000;
    let pp = poh(&w, h).actor.world_pos;
    // Link 100 west of it, his back to it (Player_IsFacingActor false): within 140, it attacks.
    let (pos, yaw) = link_by(pp, -0x4000, 100.0, false);
    w.place_player(pos, yaw);
    let health = w.save.health;
    let s = sfx_len(&w);
    until(&mut w, 200, "the attack", |w| poh(w, h).action == Action::Attack);
    // EnPoh_SetupAttack: 12 swings, still, NA_SE_EN_PO_LAUGH.
    let p = poh(&w, h);
    assert!(p.unk_198 <= 12 && p.unk_198 >= 11);
    assert!(sfx_since(&w, s, NA_SE_EN_PO_LAUGH));
    // The lantern's sphere hits only from unk_198 9 (below 10), at 5 a frame.
    let mut hit = false;
    for _ in 0..400 {
        let before = poh(&w, h).unk_198;
        idle(&mut w, 1);
        let p = poh(&w, h);
        if p.action == Action::Func80ADF574 {
            // func_80ADE4C8 (AT_HIT): backing off at 5, slowing 0.5 a frame in func_80ADF574.
            assert!(before < 10, "the lantern hits only below 10 ({before})");
            assert_eq!(p.actor.speed_xz, -4.5);
            hit = true;
            break;
        }
        if p.action != Action::Attack {
            panic!("it stopped attacking without hitting: {:?}", p.action);
        }
    }
    assert!(hit, "the lantern hit Link");
    // sJntSphElementsInit: 8 damage; Link's health -8 (half a heart).
    assert_eq!(w.save.health, health - 8);
    // func_80ADF574: spinning 0x1000 a frame until gPoeFloatAnim's end, then idle (unk_198 23).
    until(&mut w, 100, "idle again", |w| poh(w, h).action != Action::Func80ADF574);
    let p = poh(&w, h);
    assert!(p.action == Action::Idle || p.action == Action::Func80ADEC9C, "{:?}", p.action);
}

/// The Poe put back over the floor at its home (it floats over the pits as it flees) and Link 45
/// west of it, facing it, and B: a slash (`Kokiri sword` 1 in `sDamageTable`).
fn slash(w: &mut PlayState, h: ActorHandle) {
    let p = poh_mut(w, h);
    p.actor.world_pos.x = HOME_EAST.x;
    p.actor.world_pos.z = HOME_EAST.z;
    let pp = p.actor.world_pos;
    let (pos, yaw) = link_by(pp, -0x4000, 45.0, true);
    w.place_player(pos, yaw);
    idle(w, 1);
    let b = PadState { button: BTN_B, ..Default::default() };
    w.tick_with(scripted_input(PadState::default(), b));
}

#[test]
fn four_kokiri_sword_slashes_burn_it_to_a_lantern_and_a_soul() {
    let Some(a) = assets() else { return };
    let (mut w, h) = appeared(&a);
    let items_before = w.actors.all().into_iter().filter(|&x| w.actors.downcast::<EnItem00>(x).is_some()).count();
    for hit in 1..=4u8 {
        poh_mut(&mut w, h).visibility_timer = 1000;
        // Hittable again: AC on, visible (func_80ADE368 after the damage animation).
        until(&mut w, 300, "hittable", |w| {
            let p = poh(w, h);
            p.collider_cylinder.base.ac_flags & AC_ON != 0 && p.light_color[3] == 255 && p.action != Action::Func80ADEECC
        });
        let s = sfx_len(&w);
        let mut tries = 0;
        while poh(&w, h).actor.col_chk_info.health != 4 - hit {
            tries += 1;
            assert!(tries < 6, "hit {hit} never landed");
            slash(&mut w, h);
            for _ in 0..15 {
                if poh(&w, h).actor.col_chk_info.health == 4 - hit {
                    break;
                }
                idle(&mut w, 1);
            }
            if poh(&w, h).actor.col_chk_info.health != 4 - hit {
                idle(&mut w, 20);
            }
        }
        let p = poh(&w, h);
        // func_80AE032C: 1 off; NA_SE_EN_PO_DAMAGE (NA_SE_EN_PO_DEAD the last).
        assert!(sfx_since(&w, s, if hit < 4 { NA_SE_EN_PO_DAMAGE } else { NA_SE_EN_PO_DEAD }), "hit {hit}");
        if hit < 4 {
            // func_80ADE28C: knocked back at 5 (slowing 0.5), unhittable until the damage
            // animation ends, then func_80ADE368: fleeing at 5, hittable (unk_198 200).
            assert!(matches!(p.action, Action::Func80ADEECC | Action::Func80ADF894 | Action::Idle | Action::Func80ADEC9C | Action::Func80ADEAC4 | Action::Func80ADF5E0 | Action::Attack), "{:?}", p.action);
        }
    }
    // The 4th: health 0, func_80ADE48C after the damage animation: burning (not targetable, no
    // Navi), 3 EffectSsDeadDb a frame for unk_198 1 to 7.
    let mut flames = 0;
    while poh(&w, h).action != Action::Func80ADF15C {
        let before = effects_of(&w, EFFECT_SS_DEAD_DB);
        idle(&mut w, 1);
        flames += effects_of(&w, EFFECT_SS_DEAD_DB).saturating_sub(before);
    }
    let p = poh(&w, h);
    assert_eq!((p.actor.col_chk_info.health, p.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, p.actor.navi_enemy_id), (0, 0, NAVI_ENEMY_NONE));
    let mut seen = 0;
    for _ in 0..40 {
        let before = effects_of(&w, EFFECT_SS_DEAD_DB);
        idle(&mut w, 1);
        let now = effects_of(&w, EFFECT_SS_DEAD_DB);
        if now > before {
            flames += now - before;
        }
        seen += 1;
        if poh(&w, h).action != Action::Func80ADF15C {
            break;
        }
    }
    assert_eq!(flames, 21, "3 x 7 flames");
    // At unk_198 28: EnPoh_SetupDeath: the lantern, a misc actor, 0.01, 1500 up, upside down.
    let p = poh(&w, h);
    assert_eq!(p.action, Action::Death, "after {seen} frames");
    assert_eq!((p.update_fn, p.draw_fn, p.actor.category), (Update::Dead, Draw::Soul, ACTORCAT_MISC));
    assert_eq!((p.actor.scale.x, p.actor.shape_y_offset, p.actor.shape_rot.x, p.actor.gravity), (0.01, 1500.0, i16::MIN, -1.0));
    assert!(w.actors.category(ACTORCAT_ENEMY).iter().all(|&x| x != h));
    // EnPoh_Death: falling to the floor, it breaks: EffectSsHahen_SpawnBurst (15), the soul.
    until(&mut w, 60, "broken", |w| poh(w, h).action != Action::Death);
    assert_eq!(effects_of(&w, EFFECT_SS_HAHEN), 15);
    let p = poh(&w, h);
    assert_eq!(p.action, Action::Func80ADFE28);
    // func_80ADE6D4: the soul's colour (0, 0, 200) to start, unlit.
    assert_eq!(&p.light_color[1..3], &[0, 200]);
    // func_80ADFE28: 20 a frame up to 255 (13 frames), then EnPoh_Talk: 0x5005.
    until(&mut w, 20, "the soul", |w| poh(w, h).action == Action::Func80ADFE80);
    let p = poh(&w, h);
    assert_eq!((p.light_color[3], p.actor.text_id, p.unk_198), (255, TEXT_POE_SOUL, 200));
    assert_eq!(&p.light_color[..3], &[100, 0, 150], "sPoeInfo[0].lightColor");
    assert_eq!((p.collider_cylinder.dim.radius, p.collider_cylinder.dim.height, p.collider_cylinder.base.oc_flags1), (13, 30, OC1_ON | OC1_TYPE_PLAYER));
    let l = w.light_ctx.info(p.light_node.unwrap()).unwrap();
    assert_eq!((l.ty, l.color, l.radius), (LIGHT_POINT_NOGLOW, [100, 0, 150], 200));
    // It drops nothing.
    let items = w.actors.all().into_iter().filter(|&x| w.actors.downcast::<EnItem00>(x).is_some()).count();
    assert_eq!(items, items_before);
}

/// The east Poe killed at once (a 4-damage hit put in its cylinder, as the collision check
/// leaves one), to its soul.
fn soul(a: &Arc<GameAssets>, edit: impl FnOnce(&mut SaveContext)) -> (PlayState, ActorHandle) {
    let (pos, yaw) = link_by(HOME_EAST, -0x4000, 250.0, true);
    let mut w = room4_with(a, pos, yaw, edit);
    let h = poh_at(&w, HOME_EAST);
    until(&mut w, 60, "appeared", |w| poh(w, h).action == Action::Idle);
    let player = w.player;
    let p = poh_mut(&mut w, h);
    p.collider_cylinder.base.ac_flags |= AC_HIT;
    p.collider_cylinder.base.ac = player;
    p.actor.col_chk_info.damage = 4;
    until(&mut w, 200, "the soul", |w| poh(w, h).action == Action::Func80ADFE80);
    (w, h)
}

/// Through texts until `f` holds: A whenever a text waits; `choice` at the choice.
fn answer_until(w: &mut PlayState, max: usize, texts: &mut Vec<u16>, choice: u8, f: impl Fn(&PlayState) -> bool) {
    let mut pressed = false;
    for _ in 0..max {
        if f(w) {
            return;
        }
        let st = w.message_state();
        if st == TEXT_STATE_CHOICE && !pressed {
            w.msg_ctx.choice_index = choice;
        }
        let a = !pressed && matches!(st, TEXT_STATE_CHOICE | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_EVENT | TEXT_STATE_AWAITING_NEXT);
        let mut record = |w: &PlayState| {
            if w.message_state() != TEXT_STATE_NONE && w.msg_ctx.text_id != 0 && texts.last() != Some(&w.msg_ctx.text_id) {
                texts.push(w.msg_ctx.text_id);
            }
        };
        record(w);
        let pad = PadState { button: if a { BTN_A } else { 0 }, ..Default::default() };
        w.tick_with(scripted_input(PadState::default(), pad));
        pressed = a;
        record(w);
    }
    panic!("not after {max} frames (texts {texts:x?}, state {}, msgMode {:#x})", w.message_state(), w.msg_ctx.msg_mode);
}

/// Link walked into the soul: its `OC1_HIT` takes the talk at once.
fn touch(w: &mut PlayState, h: ActorHandle) {
    let p = poh(w, h).actor.world_pos;
    w.place_player(Vec3::new(p.x - 5.0, FLOOR, p.z), 0x4000);
}

#[test]
fn the_soul_with_no_empty_bottle_laughs_and_goes() {
    let Some(a) = assets() else { return };
    // cavern-open's only bottle: Talon's milk, full.
    let (mut w, h) = soul(&a, |_| {});
    assert_eq!(w.save.inventory.items[SLOT_BOTTLE_1], ITEM_BOTTLE_MILK_FULL);
    touch(&mut w, h);
    until(&mut w, 10, "talking", |w| poh(w, h).action == Action::TalkRegular);
    let mut texts = Vec::new();
    let s = sfx_len(&w);
    answer_until(&mut w, 600, &mut texts, 0, |w| poh(w, h).actor.killed || pohs(w).iter().all(|&x| x != h));
    // EnPoh_TalkRegular: yes without an empty bottle: 0x5006, NA_SE_EN_PO_LAUGH; it fades as
    // the text shows; the text closing (func_80ADE950(0)), it's gone at alpha 0.
    assert_eq!(texts.first(), Some(&TEXT_POE_SOUL));
    assert!(texts.contains(&TEXT_POE_NO_BOTTLE), "{texts:x?}");
    assert!(sfx_since(&w, s, NA_SE_EN_PO_LAUGH));
    assert_eq!(w.save.inventory.items[SLOT_BOTTLE_1], ITEM_BOTTLE_MILK_FULL);
}

#[test]
fn the_soul_goes_in_an_empty_bottle() {
    let Some(a) = assets() else { return };
    let (mut w, h) = soul(&a, |s| {
        item_give(s, None, ITEM_BOTTLE_EMPTY);
    });
    let slot = (SLOT_BOTTLE_1..SLOT_BOTTLE_1 + 4).find(|&i| w.save.inventory.items[i] == ITEM_BOTTLE_EMPTY).expect("an empty bottle");
    touch(&mut w, h);
    until(&mut w, 10, "talking", |w| poh(w, h).action == Action::TalkRegular);
    let mut texts = Vec::new();
    let s = sfx_len(&w);
    answer_until(&mut w, 600, &mut texts, 0, |w| pohs(w).iter().all(|&x| x != h));
    // Yes with an empty bottle: 0x5008, Item_Give(ITEM_BOTTLE_POE), NA_SE_EN_PO_BIG_GET.
    assert!(texts.contains(&TEXT_POE_BOTTLED), "{texts:x?}");
    assert!(sfx_since(&w, s, NA_SE_EN_PO_BIG_GET));
    assert_eq!(w.save.inventory.items[slot], ITEM_BOTTLE_POE);
}

#[test]
fn the_soul_told_no_laughs() {
    let Some(a) = assets() else { return };
    let (mut w, h) = soul(&a, |s| {
        item_give(s, None, ITEM_BOTTLE_EMPTY);
    });
    touch(&mut w, h);
    until(&mut w, 10, "talking", |w| poh(w, h).action == Action::TalkRegular);
    let mut texts = Vec::new();
    answer_until(&mut w, 600, &mut texts, 1, |w| pohs(w).iter().all(|&x| x != h));
    // The second answer: 0x5007; the bottle stays empty.
    assert!(texts.contains(&TEXT_POE_NO), "{texts:x?}");
    assert!((SLOT_BOTTLE_1..SLOT_BOTTLE_1 + 4).any(|i| w.save.inventory.items[i] == ITEM_BOTTLE_EMPTY));
}

#[test]
fn the_soul_left_alone_laughs_and_fades_after_200_frames() {
    let Some(a) = assets() else { return };
    let (mut w, h) = soul(&a, |s| {
        item_give(s, None, ITEM_BOTTLE_EMPTY);
    });
    // Link 250 away: no touch. func_80ADFE80 counts 200 down from EnPoh_Talk's frame.
    let left = poh(&w, h).unk_198;
    assert_eq!(left, 200);
    let s = sfx_len(&w);
    idle(&mut w, 199);
    assert_eq!(poh(&w, h).action, Action::Func80ADFE80);
    idle(&mut w, 1);
    // unk_198 0: func_80ADE950(1): NA_SE_EN_PO_LAUGH, func_80AE009C: 13 a frame down.
    assert_eq!(poh(&w, h).action, Action::Func80AE009C);
    assert!(sfx_since(&w, s, NA_SE_EN_PO_LAUGH));
    // From the next frame 255 - 13n: 8 after 19 frames; the 20th 0 (CLAMP), and Actor_Kill.
    idle(&mut w, 19);
    assert_eq!(poh(&w, h).light_color[3], 8);
    idle(&mut w, 1);
    assert!(pohs(&w).iter().all(|&x| x != h), "gone");
    assert!((SLOT_BOTTLE_1..SLOT_BOTTLE_1 + 4).any(|i| w.save.inventory.items[i] == ITEM_BOTTLE_EMPTY), "nothing bottled");
}

/// `object_po_composer` loaded (`Object_SpawnPersistent`) and the composer `params` spawned
/// 250 east of Link (out of his 200: it idles), as `En_Wonder_Talk` spawns them.
fn spawn_composer(w: &mut PlayState, params: i16) -> Option<ActorHandle> {
    if w.object_ctx.get_index(OBJECT_PO_COMPOSER).is_none() {
        w.object_ctx.spawn(OBJECT_PO_COMPOSER);
        idle(w, 3);
    }
    let lp = w.player().actor.world_pos;
    let h = w.actor_spawn(ACTOR_EN_POH, Vec3::new(lp.x + 250.0, lp.y, lp.z), [0; 3], params).ok()?;
    (!poh(w, h).actor.killed).then_some(h)
}

#[test]
fn the_composers_differ_by_their_tables() {
    let Some(a) = assets() else { return };
    let (pos, yaw) = link_by(HOME_EAST, -0x4000, 300.0, true);
    let mut w = room4(&a, pos, yaw);
    // Without object_po_composer: Object_GetSlot < 0, killed at init.
    let lp = w.player().actor.world_pos;
    let h = w.actor_spawn(ACTOR_EN_POH, lp + Vec3::X * 150.0, [0; 3], EN_POH_SHARP).unwrap();
    assert!(poh(&w, h).actor.killed);
    // That spawn set Sharp's switch 0x29 all the same (the kill is the init's last check).
    assert!(w.flags.get_switch(0x29));
    w.flags.unset_switch(0x29);

    let sharp = spawn_composer(&mut w, EN_POH_SHARP).expect("Sharp");
    // EnPoh_Init: switch 0x29 set; sPoeInfo[1]; NAVI_ENEMY_POE_COMPOSER.
    assert!(w.flags.get_switch(0x29));
    let flat = spawn_composer(&mut w, EN_POH_FLAT).expect("Flat");
    assert!(w.flags.get_switch(0x28));
    // A second Sharp: 0x29 set, killed.
    assert!(spawn_composer(&mut w, EN_POH_SHARP).is_none());
    idle(&mut w, 1);
    for (h, ty) in [(sharp, EN_POH_SHARP), (flat, EN_POH_FLAT)] {
        let p = poh(&w, h);
        assert_eq!((p.actor.params, p.info_idx, p.actor.navi_enemy_id, p.actor.col_chk_info.health), (ty, EN_POH_INFO_COMPOSER, NAVI_ENEMY_POE_COMPOSER, 4));
        // EnPoh_Update's composer setup: turned -0x4000, cylinder 20 by 55 up 15, the lantern's
        // sphere on limb 9 at y -1400; EnPoh_SetupInitialAction: 20 up, appearing.
        assert_eq!((p.draw_fn, p.actor.shape_rot.y, p.actor.world_rot.y), (Draw::Composer, -0x4000, -0x4000));
        let c = p.collider_cylinder.dim;
        assert_eq!((c.radius, c.height, c.y_shift), (20, 55, 15));
        let e = p.collider_jnt_sph.elements[0].dim;
        assert_eq!((e.limb, e.model_sphere.center), (9, [0, -1400, 0]));
        assert!(matches!(p.action, Action::ComposerAppear | Action::Idle));
    }
    // sPoeInfo[1]: prim (255, 255, 170), light (0, 150, 0), limbs 9 and 1, scroll -12.
    let i = &S_POE_INFO[1];
    assert_eq!((i.prim_color, i.light_color, i.unk_6, i.unk_7, i.unk_8), ([255, 255, 170], [0, 150, 0], 9, 1, -12));
    let i = &S_POE_INFO[0];
    assert_eq!((i.prim_color, i.light_color, i.unk_6, i.unk_7, i.unk_8), ([255, 170, 255], [100, 0, 150], 18, 5, -8));
    // EnPoh_ComposerAppear: alpha curFrame * 25.5 (to 255), then idle.
    until(&mut w, 60, "Sharp appeared", |w| poh(w, sharp).update_fn == Update::Living && poh(w, sharp).action != Action::ComposerAppear);
    assert_eq!(poh(&w, sharp).light_color[3], 255);
}

/// A composer to his soul (a 4-damage hit), and Link talking to it: the texts. `cavern-open`
/// has the Sun's Song (`graveyard-sun`'s), taken out first (`edit` may give it back).
fn composer_talk(edit: impl FnOnce(&mut PlayState), ty: i16, choice: u8) -> Option<(PlayState, Vec<u16>)> {
    let a = assets()?;
    let (pos, yaw) = link_by(HOME_EAST, -0x4000, 300.0, true);
    let mut w = room4(&a, pos, yaw);
    // QUEST_SONG_SUN (item.h: 0x0F).
    assert!(w.save.check_quest_item(0x0F), "cavern-open has the Sun's Song");
    w.save.inventory.quest_items &= !(1 << 0x0F);
    edit(&mut w);
    let h = spawn_composer(&mut w, ty).expect("the composer");
    until(&mut w, 80, "appeared", |w| poh(w, h).update_fn == Update::Living && poh(w, h).action != Action::ComposerAppear);
    let player = w.player;
    let p = poh_mut(&mut w, h);
    p.collider_cylinder.base.ac_flags |= AC_HIT;
    p.collider_cylinder.base.ac = player;
    p.actor.col_chk_info.damage = 4;
    until(&mut w, 200, "the soul", |w| poh(w, h).action == Action::Func80ADFE80);
    let p = poh(&w, h);
    // func_80ADE6D4: a composer's soul (0, 200, 0) to start; its light colour (0, 150, 0); not
    // upside down as it fell.
    assert_eq!(&p.light_color[..3], &[0, 150, 0]);
    touch(&mut w, h);
    until(&mut w, 10, "talking", |w| poh(w, h).action == Action::TalkComposer);
    let mut texts = Vec::new();
    if choice == 0 {
        // The first answer: 0x5010 or 0x5014, and the ask again (the talk goes on); then the
        // second answer ends it.
        answer_until(&mut w, 3000, &mut texts, 0, |w| [TEXT_COMPOSER_YES_FIRST, TEXT_COMPOSER_YES_AGAIN].contains(&w.msg_ctx.text_id) || pohs(w).iter().all(|&x| x != h));
    }
    answer_until(&mut w, 3000, &mut texts, 1, |w| pohs(w).iter().all(|&x| x != h));
    Some((w, texts))
}

#[test]
fn sharp_first_asks_and_the_second_answer_sets_his_switch() {
    // Neither 0xA nor 0xB: 0x500F; the second answer: Flags_SetSwitch(0xB), he goes.
    let Some((w, texts)) = composer_talk(|_| {}, EN_POH_SHARP, 1) else { return };
    assert_eq!(texts.first(), Some(&TEXT_COMPOSER_FIRST), "{texts:x?}");
    assert!(w.flags.get_switch(0xB) && !w.flags.get_switch(0xA));
    // The first answer: 0x5010 (neither switch set), and the ask goes on; the second then sets
    // Flat's 0xA.
    let Some((w, texts)) = composer_talk(|_| {}, EN_POH_FLAT, 0) else { return };
    assert!(texts.contains(&TEXT_COMPOSER_YES_FIRST), "{texts:x?}");
    assert!(w.flags.get_switch(0xA) && !w.flags.get_switch(0xB));
}

#[test]
fn the_composers_texts_follow_the_switches_and_the_suns_song() {
    // Sharp answered (0xB): Flat says 0x5012, Sharp 0x5013; the first answer then 0x5014.
    let Some((_, texts)) = composer_talk(|w| w.flags.set_switch(0xB), EN_POH_FLAT, 0) else { return };
    assert_eq!(texts.first(), Some(&TEXT_COMPOSER_OTHER_ANSWERED), "{texts:x?}");
    assert!(texts.contains(&TEXT_COMPOSER_YES_AGAIN), "{texts:x?}");
    let Some((_, texts)) = composer_talk(|w| w.flags.set_switch(0xB), EN_POH_SHARP, 1) else { return };
    assert_eq!(texts.first(), Some(&TEXT_COMPOSER_OWN_ANSWERED), "{texts:x?}");
    // The Sun's Song learnt: 0x5000; it closing sets the switch 9: no composer inits again.
    let Some((mut w, texts)) = composer_talk(|w| { item_give(&mut w.save, None, ITEM_SONG_SUN); }, EN_POH_FLAT, 0) else { return };
    assert!(w.save.check_quest_item(0x0F));
    assert_eq!(texts.first(), Some(&TEXT_COMPOSER_SUNS_SONG), "{texts:x?}");
    assert!(w.flags.get_switch(9));
    w.flags.unset_switch(0x29);
    assert!(spawn_composer(&mut w, EN_POH_SHARP).is_none(), "switch 9: killed at init");
}

#[test]
fn a_graves_rupee_poe_drops_a_blue_rupee_and_only_two_live() {
    let Some(a) = assets() else { return };
    let (pos, yaw) = link_by(HOME_EAST, -0x4000, 300.0, true);
    let mut w = room4(&a, pos, yaw);
    let lp = w.player().actor.world_pos;
    let items = |w: &PlayState| w.actors.all().into_iter().filter(|&x| w.actors.downcast::<EnItem00>(x).is_some_and(|i| !i.actor.killed)).collect::<Vec<_>>();
    let before = items(&w).len();
    let mut spawned = Vec::new();
    for i in 0..3 {
        let h = w.actor_spawn(ACTOR_EN_POH, lp + Vec3::new(100.0 + 30.0 * i as f32, 0.0, 0.0), [0; 3], EN_POH_RUPEE).unwrap();
        spawned.push(h);
    }
    // D_80AE1A50 1, 2: Item_DropCollectible(0x4000 | ITEM00_RUPEE_BLUE), speed 0; 3: killed.
    let now = items(&w);
    assert_eq!(now.len(), before + 2);
    for &i in &now[..2] {
        let it = w.actors.downcast::<EnItem00>(i).unwrap();
        assert_eq!((it.actor.params & 0xFF, it.actor.speed_xz), (oot_actors::en_item00::ITEM00_RUPEE_BLUE, 0.0));
    }
    assert!(!poh(&w, spawned[0]).actor.killed && !poh(&w, spawned[1]).actor.killed);
    assert!(poh(&w, spawned[2]).actor.killed);
    idle(&mut w, 2);
    // The third's destroy counts it back down: 2.
    assert_eq!(w.overlay_static::<Statics>(ACTOR_EN_POH).d_80ae1a50, 2);
    // A rupee Poe is a normal Poe otherwise: object_poh, NAVI_ENEMY_POE.
    assert_eq!((poh(&w, spawned[0]).info_idx, poh(&w, spawned[0]).actor.navi_enemy_id), (EN_POH_INFO_NORMAL, NAVI_ENEMY_POE));
    // params 4 and up: EN_POH_NORMAL.
    let h = w.actor_spawn(ACTOR_EN_POH, lp + Vec3::X * 200.0, [0; 3], 7).unwrap();
    assert_eq!(poh(&w, h).actor.params, EN_POH_NORMAL);
}

/// Question 3: Link at the bomb flower chains (the north chain x 1735 to 1860, z -1000 to -1160;
/// the south one x 2017 to 2040, z -1925 to -2045), each down a pit's side: on the floor at the
/// pit's edge by each (the north one's x 1740, the south one's x 2040; floor 80).
#[test]
fn at_the_flower_chains() {
    let Some(a) = assets() else { return };
    // The south chain: every Poe's home over 280 away (the south one's 500): none appears.
    let at = Vec3::new(2040.0, FLOOR, -1985.0);
    let mut w = room4(&a, at, vec3_yaw(at, HOME_SOUTH));
    let floor = w.col.entity_raycast_down(at + Vec3::Y * 50.0).0;
    eprintln!("south chain floor {floor}");
    idle(&mut w, 600);
    for &h in &pohs(&w) {
        assert_eq!(poh(&w, h).action, Action::Func80ADEF38, "at the south chain no Poe appears");
    }
    // The north chain: the north Poe's home is 222 away (within 280): it appears, then drifts
    // about its home (back past 400 from it) and comes at Link once within 200.
    let at = Vec3::new(1740.0, FLOOR, -1080.0);
    let mut w = room4(&a, at, vec3_yaw(at, HOME_NORTH));
    let h = poh_at(&w, HOME_NORTH);
    let health = w.save.health;
    until(&mut w, 60, "the north Poe appears", |w| poh(w, h).action != Action::Func80ADEF38);
    let mut chased = None;
    let mut attacked = None;
    for n in 0..2400 {
        idle(&mut w, 1);
        if pohs(&w).iter().all(|&x| x != h) {
            break;
        }
        // Link stays put (place_player each 100 frames: a hit knocks him).
        if n % 100 == 0 {
            w.place_player(at, vec3_yaw(at, HOME_NORTH));
        }
        let p = poh(&w, h);
        if chased.is_none() && p.action == Action::Func80ADEC9C {
            chased = Some(n);
        }
        if attacked.is_none() && p.action == Action::Attack {
            attacked = Some(n);
        }
        if w.save.health < health {
            break;
        }
    }
    // Where its drift (Rand) takes it is luck; this run's, for the record.
    eprintln!("north chain: chased at {chased:?}, attacked at {attacked:?}, health {} -> {}", health, w.save.health);
    // Once within 200 of him (EnPoh_Idle with unk_198 below 19, or func_80ADEAC4), it comes at
    // him (func_80ADE1BC): his place at the chain doesn't keep it off. Put there:
    let mut w = room4(&a, at, vec3_yaw(at, HOME_NORTH));
    let h = poh_at(&w, HOME_NORTH);
    until(&mut w, 60, "appeared", |w| poh(w, h).action == Action::Idle);
    let to = (HOME_NORTH - at).normalize() * 150.0;
    let p = poh_mut(&mut w, h);
    p.actor.world_pos.x = at.x + to.x;
    p.actor.world_pos.z = at.z + to.z;
    p.unk_198 = 0;
    idle(&mut w, 1);
    assert!(matches!(poh(&w, h).action, Action::Func80ADEC9C | Action::Func80ADEAC4 | Action::Func80ADF5E0), "{:?}", poh(&w, h).action);
    until(&mut w, 40, "at Link", |w| poh(w, h).action == Action::Func80ADEC9C);
}

fn vec3_yaw(from: Vec3, to: Vec3) -> i16 {
    eng_math::vec3f_yaw(from, to)
}
