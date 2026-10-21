//! GAME-10 milestone 2 against the C: the Zoras (`z_en_zo.c`).
//!
//! The placements (`ootx scene-info --scene spot07_scene`), the child layers 0 and 1, room 1:
//! types 0 to 5 (params 0xFFC0 to 0xFFC5) at (-786, 14, -486), (841, 24, 39), (241, -75, -384),
//! (-298, -200, -217), (-589, -120, -621) and (-936, -319, -352). Child Link enters at
//! `ENTR_ZORAS_DOMAIN_0` (spawn 0, room 1) with `fairy-magic`, the Domain's intro seen
//! (`EVENTCHKINF_A4`). The tests that need `Rand` replay it on a copy of the generator, or read
//! what it gave (a timer) and count from there. Expected values are worked out from the C in the
//! comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_zo::*;
use oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED;
use oot_game::actor_ctx::ActorHandle;
use oot_game::item::QUEST_ZORA_SAPPHIRE;
use oot_game::message::*;
use oot_game::play::{PlayState, Rand, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::shadow::ShadowDraw;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `EVENTCHKINF_A4` (`save.h`): Zora's Domain's intro seen.
const EVENTCHKINF_A4: u16 = 0xA4;

/// Link entered at the Domain (`ENTR_ZORAS_DOMAIN_0`, room 1) at 10:00 with `fairy-magic`, its
/// intro seen, `setup`, the sound log on; two frames on.
fn domain(a: &Arc<GameAssets>, adult: bool, setup: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_ZORAS_DOMAIN_0").expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    save.set_event_chk_inf(EVENTCHKINF_A4);
    setup(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert_eq!(w.room_ctx.cur.num, 1);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn zoras(w: &PlayState) -> Vec<ActorHandle> {
    let mut v: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnZo>(h).is_some_and(|z| !z.actor.killed)).collect();
    v.sort_by_key(|&h| w.actors.actor(h).unwrap().params as u16);
    v
}

fn zo(w: &PlayState, h: ActorHandle) -> &EnZo {
    w.actors.downcast::<EnZo>(h).expect("a Zora")
}

fn zo_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnZo {
    w.actors.downcast_mut::<EnZo>(h).expect("a Zora")
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// Link `d` from `p` (along +x), on whatever is below.
fn put_link_near(w: &mut PlayState, p: Vec3, d: Vec3, yaw: i16) {
    w.place_player(p + d, yaw);
}

#[test]
fn the_six_stand_or_wait_under_water_by_their_depth() {
    let Some(a) = assets() else { return };
    let w = domain(&a, false, |_| {});
    let hs = zoras(&w);
    assert_eq!(hs.len(), 6);
    for &h in &hs {
        let z = zo(&w, h);
        // EnZo_Init: ATTENTION_RANGE_6, scale 0.01, dialogRadius 26 + 30.
        assert_eq!((z.actor.target_mode, z.actor.scale, z.dialog_radius), (6, Vec3::splat(0.01), 56.0));
        // Less than 54 under water: standing (a circle shadow of 24, gZoraIdleAnim, talking, seen);
        // else submerged (unseen, untargetable).
        if z.action == Action::Standing {
            assert!(z.can_speak && z.alpha == 255.0);
            assert_eq!((z.actor.shape_shadow_draw, z.actor.shape_shadow_scale), (ShadowDraw::Circle, 24.0));
            assert!(z.skel.is("gZoraIdleAnim"));
        } else {
            assert_eq!(z.action, Action::Submerged);
            assert!(!z.can_speak && z.alpha == 0.0);
            assert_eq!(z.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
            assert_eq!(z.actor.shape_shadow_draw, ShadowDraw::None);
            assert!(z.skel.is("gZoraSurfaceAnim"));
        }
    }
    // The pool's surface at y 0: types 0 and 1 stand on its banks; 2 to 5 wait 75, 200, 120 and
    // 319 under it.
    let actions: Vec<Action> = hs.iter().map(|&h| zo(&w, h).action).collect();
    use Action::*;
    assert_eq!(actions, vec![Standing, Standing, Submerged, Submerged, Submerged, Submerged]);
    assert_eq!(hs.iter().skip(2).map(|&h| zo(&w, h).actor.y_dist_to_water).collect::<Vec<_>>(), vec![75.0, 200.0, 120.0, 319.0]);
}

#[test]
fn their_texts_by_type_and_story() {
    let Some(a) = assets() else { return };
    let mut w = domain(&a, false, |_| {});
    let base = zo(&w, zoras(&w)[0]).actor.clone();
    let text = |w: &PlayState, ty: i16| {
        let mut actor = base.clone();
        actor.params = (0xFFC0u16 as i16) | ty;
        get_text_id(w, &actor)
    };
    // Before any Zora's talk: 0x4006 for 0 to 5 and 8; 6 and 7 their own.
    let first: Vec<u16> = (0..=8).map(|t| text(&w, t)).collect();
    assert_eq!(first, vec![0x4006, 0x4006, 0x4006, 0x4006, 0x4006, 0x4006, 0x4020, 0x4021, 0x4006]);
    // EVENTCHKINF_30: their second texts.
    w.save.set_event_chk_inf(EVENTCHKINF_30);
    let second: Vec<u16> = (0..=8).map(|t| text(&w, t)).collect();
    assert_eq!(second, vec![0x4007, 0x4008, 0x400A, 0x400C, 0x400F, 0x4011, 0x4020, 0x4021, 0x402A]);
    // INFTABLE_124: type 1's 0x4009; Ruto's letter found: type 2's 0x402F, then 0x400B; King
    // Zora moved: type 4's 0x4010.
    w.save.set_inf_table(INFTABLE_124);
    w.save.set_event_chk_inf(EVENTCHKINF_31);
    w.save.set_event_chk_inf(EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA);
    assert_eq!((text(&w, 1), text(&w, 2), text(&w, 4)), (0x4009, 0x402F, 0x4010));
    w.save.set_inf_table(INFTABLE_129);
    assert_eq!(text(&w, 2), 0x400B);
    // The Sapphire: 0x402D (0, 2, 4), 0x402E (1, 3, 5).
    w.save.inventory.quest_items |= 1 << QUEST_ZORA_SAPPHIRE;
    let third: Vec<u16> = (0..=5).map(|t| text(&w, t)).collect();
    assert_eq!(third, vec![0x402D, 0x402E, 0x402D, 0x402E, 0x402D, 0x402E]);
    // An unknown type: 0x4006. The Goron Mask (5): 0x7189 for all.
    assert_eq!(text(&w, 0x3F), 0x4006);
    w.player_mut().current_mask = 5;
    assert_eq!(text(&w, 0), 0x7189);
}

#[test]
fn a_talk_taps_his_foot_and_sets_eventchkinf_30() {
    let Some(a) = assets() else { return };
    let mut w = domain(&a, false, |_| {});
    let h = zoras(&w).into_iter().find(|&h| zo(&w, h).action == Action::Standing).expect("a standing Zora");
    let z = zo(&w, h);
    let (pos, yaw) = (z.actor.world_pos, z.actor.shape_rot.y);
    // Link 40 in front of him, facing him.
    let fwd = Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw));
    put_link_near(&mut w, pos, fwd * 40.0, yaw.wrapping_add(i16::MIN));
    idle(&mut w, 10);
    assert_eq!(w.player().target_actor, Some(h));
    assert_eq!(zo(&w, h).actor.text_id, 0x4006);
    // A: Npc_UpdateTalking's talk; EnZo_Standing: talking, the whole body on Link
    // (NPC_TRACKING_FULL_BODY), 0x4006: gZoraHandsOnHipsTappingFootAnim (EnZo_SetAnimation).
    w.tick_with(scripted_input(PadState::default(), PadState { button: BTN_A, ..Default::default() }));
    idle(&mut w, 3);
    let z = zo(&w, h);
    assert_eq!(z.interact_info.talk_state, NPC_TALK_STATE_TALKING);
    assert_eq!(z.tracking_mode, NPC_TRACKING_FULL_BODY);
    assert!(z.skel.is(TAPPING_FOOT_ANIM));
    // Read through: closing sets EVENTCHKINF_30; idle, back to gZoraIdleAnim (ENZO_ANIM_0).
    let mut pressed = false;
    for _ in 0..400 {
        if zo(&w, h).interact_info.talk_state == NPC_TALK_STATE_IDLE && w.message_state() == TEXT_STATE_NONE {
            break;
        }
        let st = w.message_state();
        let a = !pressed && matches!(st, TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_EVENT | TEXT_STATE_AWAITING_NEXT);
        w.tick_with(scripted_input(PadState::default(), PadState { button: if a { BTN_A } else { 0 }, ..Default::default() }));
        pressed = a;
    }
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_30));
    idle(&mut w, 1);
    assert!(zo(&w, h).skel.is("gZoraIdleAnim"));
}

#[test]
fn a_submerged_zora_surfaces_treads_water_and_dives() {
    let Some(a) = assets() else { return };
    let mut w = domain(&a, false, |_| {});
    let h = zoras(&w).into_iter().find(|&h| zo(&w, h).action == Action::Submerged).expect("a submerged Zora");
    w.actors.actor_mut(h).unwrap().flags |= oot_game::actor::ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    let home = zo(&w, h).actor.home_pos;
    let surface = home.y + zo(&w, h).actor.y_dist_to_water;
    // Link on the surface above him, 100 across (EnZo_PlayerInProximity: within 240 and 80).
    put_link_near(&mut w, Vec3::new(home.x, surface, home.z), Vec3::new(100.0, 0.0, 0.0), 0x4000);
    let mut n = 0;
    while zo(&w, h).action == Action::Submerged {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 20, "he never rose");
    }
    // EnZo_Submerged: up at 4.
    assert_eq!((zo(&w, h).action, zo(&w, h).actor.velocity.y), (Action::Surface, 4.0));
    let mut n = 0;
    while zo(&w, h).action == Action::Surface {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 300, "he never surfaced");
    }
    // EnZo_Surface at 54 deep: NA_SE_EV_OUT_OF_WATER, 20 splashes (all 15 slots), treading
    // (gZoraSurfaceAnim held at its end), targetable, fully seen.
    let z = zo(&w, h);
    assert_eq!(z.action, Action::TreadWater);
    assert!(z.actor.y_dist_to_water < 54.0);
    assert_eq!((z.alpha, z.actor.velocity.y), (255.0, 0.0));
    assert_ne!(z.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert!(!sfx_frames(&w, NA_SE_EV_OUT_OF_WATER).is_empty());
    // The splashes went into the effects (their update the same frame may have turned some into
    // ripples): no slot free and no bubble left.
    assert!(z.effects.iter().all(|e| e.type_ == ENZO_EFFECT_SPLASH || e.type_ == ENZO_EFFECT_RIPPLE), "{:?}", z.effects.iter().map(|e| e.type_).collect::<Vec<_>>());
    // Once the animation's end is played: talking and the whole body on Link.
    let mut n = 0;
    while !zo(&w, h).can_speak {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 100);
    }
    assert_eq!(zo(&w, h).tracking_mode, NPC_TRACKING_FULL_BODY);
    // Link far away (back at the entrance, spawn 0): the timer Rand_S16Offset(40, 40) gave last
    // frame counts down (DECR), the dive on its 0.
    w.place_player(Vec3::new(-1112.0, 210.0, -160.0), 0x4000);
    let timer = zo(&w, h).time_to_dive;
    assert!((40..80).contains(&timer), "{timer}");
    idle(&mut w, 1);
    let left = zo(&w, h).time_to_dive;
    let mut n = 0;
    while zo(&w, h).action == Action::TreadWater {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 100);
    }
    assert_eq!(n, left as usize, "the dive {left} frames on");
    // EnZo_Dive: gZoraSurfaceAnim backwards (speed -1) from its end to 8; untalkative.
    let z = zo(&w, h);
    assert_eq!((z.skel.play_speed, z.skel.end_frame, z.can_speak, z.tracking_mode), (-1.0, 8.0, false, NPC_TRACKING_NONE));
    // At frame 8: NA_SE_EV_DIVE_WATER, down at 4; faded out: home, submerged.
    let mut n = 0;
    while zo(&w, h).action == Action::Dive {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 400, "he never faded");
    }
    assert!(!sfx_frames(&w, NA_SE_EV_DIVE_WATER).is_empty());
    let z = zo(&w, h);
    assert_eq!((z.action, z.actor.world_pos, z.alpha), (Action::Submerged, home, 0.0));
    assert_eq!(z.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
}

#[test]
fn his_ripples_bubbles_and_splashes() {
    let Some(a) = assets() else { return };
    let mut w = domain(&a, false, |_| {});
    let h = zoras(&w).into_iter().find(|&h| zo(&w, h).action == Action::Submerged).expect("a submerged Zora");
    let (at, surface) = {
        let z = zo(&w, h);
        (z.actor.world_pos, z.actor.world_pos.y + z.actor.y_dist_to_water)
    };
    let z = zo_mut(&mut w, h);
    z.effects = [EnZoEffect::default(); EN_ZO_EFFECT_COUNT];
    // A ripple: Math_ApproachF(scale, target, 0.2, 0.8), alpha down 20 a frame, gone at 0: 200
    // in ten frames.
    z.spawn_ripple(at, 0.2, 1.0, 200);
    z.update_effects_ripples();
    let e = z.effects[0];
    assert_eq!((e.type_, e.alpha), (ENZO_EFFECT_RIPPLE, 180));
    assert!((e.scale - (0.2 + 0.8 * 0.2)).abs() < 1e-6);
    for _ in 0..9 {
        z.update_effects_ripples();
    }
    assert_eq!(z.effects[0].type_, ENZO_EFFECT_NONE);

    // A bubble above the surface isn't spawned (no Rand drawn); 2.5 below it is, its scale
    // 0.12 + (Rand - 0.5) * 0.02 (replayed on a copy of the generator).
    let mut rand = Rand { state: 7 };
    let before = rand;
    z.spawn_bubble(&mut rand, Vec3::new(at.x, surface + 1.0, at.z));
    assert_eq!((z.effects[0].type_, rand), (ENZO_EFFECT_NONE, before));
    let start = Vec3::new(at.x, surface - 2.5, at.z);
    let mut replay = rand;
    z.spawn_bubble(&mut rand, start);
    let scale = ((replay.zero_one() - 0.5) * 0.02) + 0.12;
    assert_eq!((z.effects[0].type_, z.effects[0].scale, z.effects[0].vel), (ENZO_EFFECT_BUBBLE, scale, Vec3::Y));
    // Rising 1 a frame, x and z within a quarter of its start (two Rand a frame); at the surface
    // (frame 3: 0.5 above) a ripple there (0.06 growing to 0.12, alpha 200) in the slot it freed.
    for frame in 1..=3 {
        let (dx, dz) = ((replay.zero_one() * 0.5) - 0.25, (replay.zero_one() * 0.5) - 0.25);
        z.update_effects_bubbles(&mut rand);
        let e = z.effects[0];
        if frame < 3 {
            assert_eq!((e.type_, e.pos), (ENZO_EFFECT_BUBBLE, Vec3::new(start.x + dx, start.y + frame as f32, start.z + dz)));
        } else {
            assert_eq!((e.type_, e.pos.y, e.scale, e.target_scale, e.alpha), (ENZO_EFFECT_RIPPLE, surface, 0.06, 0.12, 200));
        }
        assert_eq!(rand, replay);
    }

    // A splash thrown up at 3 from the surface (and 0.5 across): gravity 1 a frame; through the
    // surface again, a ripple set on it. Its alpha 100 + Rand * 100.
    z.effects = [EnZoEffect::default(); EN_ZO_EFFECT_COUNT];
    let mut replay = rand;
    z.spawn_splash(&mut rand, Vec3::new(at.x, surface, at.z), Vec3::new(0.5, 3.0, 0.0), 0.08);
    let e = z.effects[0];
    assert_eq!((e.type_, e.alpha, e.vec), (ENZO_EFFECT_SPLASH, ((replay.zero_one() * 100.0) + 100.0) as u8, Vec3::new(0.0, -1.0, 0.0)));
    let mut ys = Vec::new();
    while z.effects[0].type_ == ENZO_EFFECT_SPLASH {
        z.update_effects_splashes();
        ys.push(z.effects[0].pos.y - surface);
        assert!(ys.len() < 20);
    }
    // Up 3, 2, 1, 0, -1, -2, -3 (at the surface: not below it), -4: through, on the surface.
    assert_eq!(ys, vec![3.0, 5.0, 6.0, 6.0, 5.0, 3.0, 0.0, 0.0]);
    assert_eq!((z.effects[0].type_, z.effects[0].pos.x), (ENZO_EFFECT_RIPPLE, at.x + 0.5 * 8.0));

    // EnZo_SpawnSplashes: 20 splashes, each into the first slot that isn't one (@bug (game):
    // whatever it held, the ripple here); the last five find none (their Rand drawn all the same).
    z.effects = [EnZoEffect::default(); EN_ZO_EFFECT_COUNT];
    z.spawn_ripple(at, 0.2, 1.0, 200);
    let mut replay = rand;
    z.spawn_splashes(&mut rand);
    assert!(z.effects.iter().all(|e| e.type_ == ENZO_EFFECT_SPLASH));
    for _ in 0..(20 * 3 + 15) {
        replay.zero_one();
    }
    assert_eq!(rand, replay);
}

