//! GAME-07 milestone 1 against the C: Castle Town's actors. Child Malon (`z_en_ma1.c`), the
//! haggling townsfolk (`z_en_mu.c`), the dancing couple (`z_en_tg.c`), the cucco girl and her cucco
//! (`z_en_niw_girl.c`, `z_en_niw.c`), the gossip stones (`z_en_gs.c`), the butterflies and their
//! groups (`z_en_butte.c`, `z_obj_mure.c`), the Temple of Time's altar text, windows and warp pad
//! (`z_en_wonder_talk.c`, `z_bg_toki_hikari.c`, `z_bg_mjin.c`), the market's balcony camera
//! (`Camera_Fixed1`); then the exit's run (`Route::Market`).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_mjin::BgMjin;
use oot_actors::bg_toki_hikari::{self, BgTokiHikari};
use oot_actors::en_butte::{self, EnButte};
use oot_actors::en_gs::{self, EnGs};
use oot_actors::en_ma1::{self, EnMa1};
use oot_actors::en_mu::EnMu;
use oot_actors::en_niw::{self, EnNiw};
use oot_actors::en_niw_girl::{self, EnNiwGirl};
use oot_actors::en_tg::EnTg;
use oot_actors::en_wonder_talk::EnWonderTalk;
use oot_actors::obj_mure::ObjMure;
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::collision_check::AC_HIT;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::surface::SurfaceType;

/// `SCENE_HYRULE_CASTLE` (`scene_table.h`).
const SCENE_HYRULE_CASTLE: u16 = 0x5F;
/// `CAM_SET_MARKET_BALCONY` (`camera.h`: 0x14).
const CAM_SET_MARKET_BALCONY: i16 = 0x14;
/// `EVENTCHKINF_A5`: the castle's intro seen (`sEntranceCutsceneTable`).
const EVENTCHKINF_A5: u16 = 0xA5;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Child Link entered at `entrance` by day (`deku-tree-dead`), `edit` applied to the save first;
/// two frames on (the room's actors and objects in).
fn enter(a: &Arc<GameAssets>, entrance: &str, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-dead").unwrap();
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 2);
    w
}

fn market(a: &Arc<GameAssets>) -> PlayState {
    enter(a, "ENTR_MARKET_DAY_0", |_| {})
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

#[test]
fn malon_is_in_the_market_until_she_has_been_at_the_castle() {
    let Some(a) = assets() else { return };
    // EnMa1_ShouldSpawn: the market (by day or night) while neither
    // EVENTCHKINF_TALON_RETURNED_FROM_CASTLE nor INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE is set.
    let w = market(&a);
    let h = one::<EnMa1>(&w);
    let m = w.actors.downcast::<EnMa1>(h).unwrap();
    // EnMa1_Init: scale 0.01, ATTENTION_RANGE_6, singing (MALON_ANIM_SING_NOMORPH) in EnMa1_Idle.
    assert_eq!(m.actor.scale, Vec3::splat(0.01));
    assert_eq!(m.actor.target_mode, 6);
    assert!(m.skel.is("gMalonChildSingAnim"));
    assert_eq!(m.action, en_ma1::Action::Idle);

    // The castle the first time: the flag set, and she isn't there.
    let w = enter(&a, "ENTR_HYRULE_CASTLE_0", |_| {});
    assert!(all::<EnMa1>(&w).is_empty());
    assert!(w.save.get_inf_table(en_ma1::INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE));
    // The castle from then on: there; the market: gone.
    let w = enter(&a, "ENTR_HYRULE_CASTLE_0", |s| s.set_inf_table(en_ma1::INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE));
    assert_eq!(all::<EnMa1>(&w).len(), 1);
    let w = enter(&a, "ENTR_MARKET_DAY_0", |s| s.set_inf_table(en_ma1::INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE));
    assert!(all::<EnMa1>(&w).is_empty());
    // Talon back from the castle: not in the market.
    let w = enter(&a, "ENTR_MARKET_DAY_0", |s| s.set_event_chk_inf(en_ma1::EVENTCHKINF_TALON_RETURNED_FROM_CASTLE));
    assert!(all::<EnMa1>(&w).is_empty());
}

#[test]
fn malons_texts_follow_the_story() {
    let Some(a) = assets() else { return };
    let mut w = market(&a);
    // EnMa1_GetTextId, from the first meeting on.
    assert_eq!(EnMa1::get_text_id(&w), 0x2041);
    w.save.set_inf_table(en_ma1::INFTABLE_TALKED_TO_MALON_FIRST_TIME);
    assert_eq!(EnMa1::get_text_id(&w), 0x2042);
    w.save.set_inf_table(en_ma1::INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE);
    assert_eq!(EnMa1::get_text_id(&w), 0x2043);
    w.save.set_event_chk_inf(en_ma1::EVENTCHKINF_RECEIVED_WEIRD_EGG);
    assert_eq!(EnMa1::get_text_id(&w), 0x2044);
    w.save.set_event_chk_inf(en_ma1::EVENTCHKINF_TALON_RETURNED_FROM_CASTLE);
    assert_eq!(EnMa1::get_text_id(&w), 0x2047);
    w.save.set_event_chk_inf(en_ma1::EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH);
    assert_eq!(EnMa1::get_text_id(&w), 0x2048);
    w.save.set_inf_table(en_ma1::INFTABLE_TOLD_EPONA_IS_SCARED);
    assert_eq!(EnMa1::get_text_id(&w), 0x2049);
    // QUEST_SONG_EPONA (0x0D) first of all.
    w.save.inventory.quest_items |= 1 << 0x0D;
    assert_eq!(EnMa1::get_text_id(&w), 0x204A);
}

#[test]
fn malon_stops_singing_to_talk() {
    let Some(a) = assets() else { return };
    let mut w = market(&a);
    let h = one::<EnMa1>(&w);
    // A talk accepted (ACTOR_FLAG_TALK): Npc_UpdateTalking's talkState 1 after EnMa1_Idle ran,
    // still singing, so EnMa1_UpdateSinging turns the voice off (Audio_ToggleMalonSinging(true));
    // the next frame EnMa1_Idle changes to the idle animation (MALON_ANIM_IDLE).
    w.actors.downcast_mut::<EnMa1>(h).unwrap().actor.flags |= oot_game::actor::ACTOR_FLAG_TALK;
    idle(&mut w, 1);
    let m = w.actors.downcast::<EnMa1>(h).unwrap();
    assert_eq!(m.interact_info.talk_state, 1);
    assert!(m.skel.is("gMalonChildSingAnim"));
    assert!(m.singing_disabled);
    idle(&mut w, 1);
    assert!(w.actors.downcast::<EnMa1>(h).unwrap().skel.is("gMalonChildIdleAnim"));
    // Back to idle: singing again (MALON_ANIM_SING), the voice back on.
    w.actors.downcast_mut::<EnMa1>(h).unwrap().interact_info.talk_state = 0;
    idle(&mut w, 1);
    let m = w.actors.downcast::<EnMa1>(h).unwrap();
    assert!(m.skel.is("gMalonChildSingAnim"));
    assert!(!m.singing_disabled);
}

#[test]
fn the_haggling_townsfolk_say_all_five_texts_before_any_again() {
    let Some(a) = assets() else { return };
    let mut w = market(&a);
    let h = all::<EnMu>(&w)[0];
    // EnMu_Interact: the next unheard of 0x7042, 0x7043, 0x703F, 0x7041, 0x703E, its bit in
    // eventInf[2] (EVENTINF_HAGGLING_TOWNSFOLK_MESG_0..4); all five heard, the bits cleared, and
    // the one just heard skipped.
    let mut texts = Vec::new();
    for _ in 0..6 {
        let mut mu = w.actors.take(h).unwrap();
        mu.as_any_mut().downcast_mut::<EnMu>().unwrap().interact(&mut w);
        texts.push(mu.as_any().downcast_ref::<EnMu>().unwrap().default_text_id);
        w.actors.put_back(h, mu);
    }
    let mut first5 = texts[..5].to_vec();
    first5.sort();
    // (Both townsfolk's inits ran EnMu_Interact first: the first of these is the third text.)
    assert_eq!(first5.iter().filter(|t| [0x703E, 0x703F, 0x7041, 0x7042, 0x7043].contains(t)).count(), 5);
    assert_ne!(texts[5], texts[4]);
    assert_eq!(w.save.event_inf[2] & 0x1F != 0, true);
}

#[test]
fn the_dancing_couple_spins_and_alternates_its_two_texts() {
    let Some(a) = assets() else { return };
    let mut w = market(&a);
    let h = one::<EnTg>(&w);
    // EnTg_SpinIfNotTalking: 0x800 a frame.
    let y0 = w.actors.downcast::<EnTg>(h).unwrap().actor.shape_rot.y;
    idle(&mut w, 3);
    assert_eq!(w.actors.downcast::<EnTg>(h).unwrap().actor.shape_rot.y, y0.wrapping_add(0x1800));
    // EnTg_GetTextId: odd nextDialogue 0x7025, even 0x7026 (Kakariko: 0x5089, 0x508A).
    assert_eq!(EnTg::get_text_id(&w, 1), 0x7025);
    assert_eq!(EnTg::get_text_id(&w, 2), 0x7026);
}

#[test]
fn the_cucco_girl_spawns_her_cucco_ahead_and_lends_it_her_path() {
    let Some(a) = assets() else { return };
    let mut w = market(&a);
    let h = one::<EnNiwGirl>(&w);
    let girl = w.actors.downcast::<EnNiwGirl>(h).unwrap();
    // EnNiwGirl_Init: path (params >> 8) & 0xFF = 2 (params 0x0200); her En_Niw (type 0xA) 50
    // ahead of her (rot.y -0x4000: towards -x), immovable.
    assert_eq!(girl.path, 2);
    let niw_h = girl.chased_en_niw.expect("her cucco");
    let home = girl.actor.home_pos;
    let niw = w.actors.downcast::<EnNiw>(niw_h).unwrap();
    assert_eq!(niw.actor.params, 0xA);
    assert!((niw.actor.home_pos - (home + Vec3::new(-50.0, 0.0, 0.0))).length() < 0.01, "{}", niw.actor.home_pos);
    assert_eq!(niw.actor.col_chk_info.mass, oot_game::collision_check::MASS_IMMOVABLE);
    // The chase (func_80AB9210): within 70 of it, the cucco gets path + 1 and its count and runs
    // it (func_80AB6A38, speed 4).
    let mut ran = false;
    for _ in 0..1200 {
        idle(&mut w, 1);
        let niw = w.actors.downcast::<EnNiw>(niw_h).unwrap();
        if niw.action == en_niw::Action::Func80AB6A38 {
            assert_eq!(niw.path, 3);
            assert!(niw.unk_2ec > 0);
            ran = true;
            break;
        }
    }
    assert!(ran, "the cucco never ran the girl's path");
    assert!(matches!(w.actors.downcast::<EnNiwGirl>(h).unwrap().action, en_niw_girl::Action::Chase));
}

#[test]
fn a_struck_cucco_cries_flees_and_counts_the_strike() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_MARKET_DAY_0", |_| {});
    let at = w.player().actor.world_pos + Vec3::new(0.0, 0.0, -200.0);
    let Ok(h) = w.actor_spawn(en_niw::ACTOR_EN_NIW, at, [0; 3], 0) else {
        // object_niw isn't in the market's rooms but for the girl's: it is (she's there).
        panic!("En_Niw spawns in the market");
    };
    idle(&mut w, 1);
    let strikes = w.actors.downcast::<EnNiw>(h).unwrap().unk_2a4;
    // EnNiw_Init: (s16)Rand_ZeroFloat(3.99) + 5.
    assert!((5..=8).contains(&strikes), "{strikes}");
    // func_80AB747C: struck (AC_HIT), 100 frames of flight, one strike counted, feathers.
    w.actors.downcast_mut::<EnNiw>(h).unwrap().collider.base.ac_flags |= AC_HIT;
    idle(&mut w, 1);
    let n = w.actors.downcast::<EnNiw>(h).unwrap();
    assert_eq!(n.unk_2a4, strikes - 1);
    assert_eq!(n.action, en_niw::Action::Func80AB7290);
    // unk_2A6 1: the next update's 20 feathers (EnNiw_SpawnFeather), as func_80AB7290 hands on.
    assert_eq!(n.unk_2a6, 1);
    idle(&mut w, 1);
    let n = w.actors.downcast::<EnNiw>(h).unwrap();
    assert_eq!(n.effects.iter().filter(|e| e.ty == 1).count(), 20);
    assert_eq!(n.action, en_niw::Action::Func80AB7328);
    assert_eq!(n.actor.speed_xz, 4.0);
}

#[test]
fn a_gossip_stone_wobbles_at_the_sword_and_says_so() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_TEMPLE_OF_TIME_EXTERIOR_DAY_0", |_| {});
    let stones = all::<EnGs>(&w);
    assert_eq!(stones.len(), 4);
    let h = stones[0];
    // EnGs_Init: scale 0.1, ATTENTION_RANGE_6, waiting for a song (func_80A4F734).
    let g = w.actors.downcast::<EnGs>(h).unwrap();
    assert_eq!(g.actor.scale, Vec3::splat(0.1));
    assert_eq!(g.action, en_gs::Action::Func80A4F734);
    // The sword (damage reaction 15, sDamageTable's 0xF): the wobble (func_80A4E910), turned
    // (unk_19E & 1), the talk stopped (func_80A4F77C: unk_19C 3).
    {
        let g = w.actors.downcast_mut::<EnGs>(h).unwrap();
        g.collider.base.ac_flags |= AC_HIT;
        g.actor.col_chk_info.damage_reaction = 15;
    }
    idle(&mut w, 1);
    let g = w.actors.downcast::<EnGs>(h).unwrap();
    assert_eq!(g.action, en_gs::Action::Func80A4E910);
    assert_eq!(g.unk_19e & 1, 1);
    // Its first step: unk_1E8 0.5 to 0 (func_80A4E754's double smoothing); once still, 0x71B1.
    let mut said = false;
    for _ in 0..300 {
        idle(&mut w, 1);
        if w.actors.downcast::<EnGs>(h).unwrap().action == en_gs::Action::Func80A4F734 {
            said = w.msg_ctx.text_id == 0x71B1;
            break;
        }
    }
    assert!(said, "the stone's text 0x71B1");
}

#[test]
fn the_markets_butterflies_come_alive_near_the_camera() {
    let Some(a) = assets() else { return };
    let mut w = market(&a);
    // North of the fountain: the pivot camera at the market's centre looks north, at the group
    // (77, 49, -204), which updates only inside its culling volume (no
    // ACTOR_FLAG_UPDATE_CULLING_DISABLED until it's active).
    w.place_player(Vec3::new(0.0, 0.0, -450.0), -0x8000);
    idle(&mut w, 5);
    // Obj_Mure 0x3324: 3 (chNum) butterflies (type 4), spawn kind 1 (ObjMure_SpawnActors1): the
    // first params 1 (the fairy one), the others -1 (0: EnButte_Init).
    let h = one::<ObjMure>(&w);
    let m = w.actors.downcast::<ObjMure>(h).unwrap();
    assert_eq!((m.ch_num, m.ptn, m.sv_num, m.ty), (3, 3, 1, 4));
    assert_eq!(m.action, oot_actors::obj_mure::Action::ActiveState);
    let butterflies = all::<EnButte>(&w);
    assert_eq!(butterflies.len(), 3);
    let fairy: Vec<_> = butterflies.iter().filter(|&&b| w.actors.downcast::<EnButte>(b).unwrap().actor.params == 1).collect();
    assert_eq!(fairy.len(), 1);
    let b = w.actors.downcast::<EnButte>(*fairy[0]).unwrap();
    // EnButte_Init: scale 0.01, cullingVolumeScale 200 for the fairy one (20 otherwise).
    assert_eq!(b.actor.scale, Vec3::splat(0.01));
    assert_eq!(b.actor.culling_volume_scale, 200.0);
    let other = butterflies.iter().find(|&&x| x != *fairy[0]).unwrap();
    assert_eq!(w.actors.downcast::<EnButte>(*other).unwrap().actor.culling_volume_scale, 20.0);
}

#[test]
fn a_butterfly_initialises_as_its_c_does() {
    let Some(a) = assets() else { return };
    let mut w = market(&a);
    let at = w.player().actor.world_pos + Vec3::new(0.0, 40.0, -100.0);
    let h = w.actor_spawn(en_butte::ACTOR_EN_BUTTE, at, [0; 3], -1).expect("En_Butte");
    let b = w.actors.downcast::<EnButte>(h).unwrap();
    // params -1 becomes 0; sInitChain's culling volume (700, 20, 600); shape.rot.x -= 0x2320;
    // EnButte_SetupFlyAround from flightParamsIdx 0: 1 or 2 (Rand_ZeroOne() < 0.6), and a timer
    // of Rand_S16Offset(5, 35).
    assert_eq!(b.actor.params, 0);
    assert_eq!((b.actor.culling_volume_distance, b.actor.culling_volume_scale, b.actor.culling_volume_downward), (700.0, 20.0, 600.0));
    assert_eq!(b.actor.shape_rot.x, -0x2320);
    assert!(b.flight_params_idx == 1 || b.flight_params_idx == 2);
    assert!((5..40).contains(&b.timer), "{}", b.timer);
    assert_eq!(b.action, en_butte::Action::FlyAround);
}

#[test]
fn the_temple_of_times_windows_altar_and_warp_pad() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_TEMPLE_OF_TIME_0", |_| {});
    // Bg_Mjin params 1 (the Light Medallion's pad): object_mjin loaded, then its collision.
    let h = one::<BgMjin>(&w);
    assert!(w.actors.downcast::<BgMjin>(h).unwrap().drawn);
    assert!(w.actors.get(h).unwrap().dyna_bg_id().is_some());
    // En_Wonder_Talk 0x0FFF: kind 1 (the slate), no switch flag: 0x7040 for a child, range 80.
    let h = one::<EnWonderTalk>(&w);
    let t = w.actors.downcast::<EnWonderTalk>(h).unwrap();
    assert_eq!((t.unk_150, t.switch_flag), (1, -1));
    assert_eq!(t.actor.text_id, 0x7040);
    assert_eq!(t.unk_15c, 80.0);
    // Bg_Toki_Hikari params 1: waits for the room's drawParams[1] (the door opening), then grows
    // 0.05 a frame to 1, then fades 0.025 a frame and is gone at 0.2.
    let lights = all::<BgTokiHikari>(&w);
    assert_eq!(lights.len(), 2);
    let beam = *lights.iter().find(|&&l| w.actors.downcast::<BgTokiHikari>(l).unwrap().actor.params == 1).unwrap();
    assert_eq!(w.actors.downcast::<BgTokiHikari>(beam).unwrap().action, bg_toki_hikari::Action::Func808BA204);
    w.scene.as_mut().unwrap().draw.room_draw_params[1] = 0xFF;
    idle(&mut w, 1);
    assert_eq!(w.actors.downcast::<BgTokiHikari>(beam).unwrap().action, bg_toki_hikari::Action::Func808BA22C);
    let mut frames = 0;
    while w.actors.downcast::<BgTokiHikari>(beam).is_some_and(|b| b.action == bg_toki_hikari::Action::Func808BA22C) {
        idle(&mut w, 1);
        frames += 1;
        assert!(frames < 100);
    }
    // 0.05 summed in f32 reaches 1.0 after 20 or 21 steps (rounding), then one frame sets it.
    assert!((20..=22).contains(&frames), "{frames}");
    assert_eq!(w.actors.downcast::<BgTokiHikari>(beam).unwrap().unk_14c, 1.0);
}

#[test]
fn the_market_balcony_camera_turns_to_its_bg_cameras_rotation() {
    let Some(a) = assets() else { return };
    let mut w = market(&a);
    // A floor under bg camera 1 (CAM_SET_MARKET_BALCONY: eye (0, 64, 0), rot (-1456, -12743)):
    // looked for over the market.
    let mut spot = None;
    'find: for xi in -30..=30 {
        for zi in -30..=30 {
            let p = Vec3::new(xi as f32 * 25.0, 600.0, zi as f32 * 25.0);
            let (y, poly) = w.col.entity_raycast_down(p);
            if let Some(poly) = poly
                && w.col.bg_cam_index(poly) == 1
            {
                spot = Some(Vec3::new(p.x, y, p.z));
                break 'find;
            }
        }
    }
    let spot = spot.expect("a floor under the balcony's camera");
    eprintln!("the balcony's camera at {spot}");
    w.place_player(spot, 0);
    idle(&mut w, 60);
    let cam = w.active_camera();
    assert_eq!(cam.setting, CAM_SET_MARKET_BALCONY);
    // Camera_Fixed1: the eye eases to the bg camera's position (Camera_LERPCeilVec3f 0.1, 0.1,
    // 0.2); `at` along its yaw rot.y (Camera_LERPCeilS by lerpStep, to within 5).
    assert!((cam.eye - Vec3::new(0.0, 64.0, 0.0)).length() < 1.0, "{}", cam.eye);
    let yaw = oot_game::camera::diff_to_sph_geo(cam.eye, cam.at).yaw;
    assert!((yaw.wrapping_sub(-12743) as i32).abs() < 64, "yaw {yaw:#x}");
}

/// The exit's run.
fn run_route(route: Route) -> (Playthrough, PlayState) {
    let a = assets().unwrap();
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let save = route.save(e);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}", run.steps);
    (run, w)
}

#[test]
fn exit_castle_town_through_the_market_to_the_castle() {
    if assets().is_none() {
        return;
    }
    let (run, w) = run_route(Route::Market);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::Market, Step::Townsman, Step::Malon, Step::CastleRoad]);
    assert_eq!(w.scene_id, SCENE_HYRULE_CASTLE);
    // Malon's 0x2041 closed: INFTABLE_TALKED_TO_MALON_FIRST_TIME and
    // EVENTCHKINF_TALKED_TO_MALON_FIRST_TIME (EnMa1_UpdateTalkState); the castle's intro seen.
    assert!(w.save.get_inf_table(en_ma1::INFTABLE_TALKED_TO_MALON_FIRST_TIME));
    assert!(w.save.get_event_chk_inf(en_ma1::EVENTCHKINF_TALKED_TO_MALON_FIRST_TIME));
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_A5));
    // At the castle the first time: INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE, and no Malon.
    assert!(w.save.get_inf_table(en_ma1::INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE));
}

#[test]
fn player_takes_the_light_setting_of_the_floor_he_stands_on() {
    let Some(a) = assets() else { return };
    // Environment_ChangeLightSetting from Player's floor (z_player.c: SurfaceType_GetLightSetting
    // where floorBgId is BGCHECK_SCENE). The shrine's floors ask for 31, past LIGHT_SETTING_MAX:
    // setting 0 stays (its fog, near 970, far 3000, is the haze there).
    let mut w = enter(&a, "ENTR_TEMPLE_OF_TIME_EXTERIOR_DAY_0", |_| {});
    idle(&mut w, 10);
    assert_eq!(w.env_ctx.light_setting, 0);
    assert_eq!(w.scene.as_ref().unwrap().lights.fog_near, 970);
    // The Temple of Time: a floor asking for setting 3 (looked for over the room).
    let mut w = enter(&a, "ENTR_TEMPLE_OF_TIME_0", |s| s.set_event_chk_inf(0xA7));
    let mut spot = None;
    'find: for xi in -80..=80 {
        for zi in -80..=80 {
            let p = Vec3::new(xi as f32 * 25.0, 3000.0, zi as f32 * 25.0);
            let (y, poly) = w.col.entity_raycast_down(p);
            if let Some(poly) = poly
                && w.col.light_setting(poly) == 3
            {
                spot = Some(Vec3::new(p.x, y, p.z));
                break 'find;
            }
        }
    }
    let spot = spot.expect("a floor asking for light setting 3");
    w.place_player(spot, 0);
    idle(&mut w, 2);
    // The setting changed, the old one kept to blend from.
    assert_eq!(w.env_ctx.light_setting, 3);
    assert_eq!(w.env_ctx.prev_light_setting, 0);
}
