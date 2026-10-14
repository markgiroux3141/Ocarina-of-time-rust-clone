//! GAME-07 milestone 6 against the C: the royal tomb's opening in the graveyard
//! (`z_bg_spot02_objects.c`, `z_object_kankyo.c`'s params 2 and 4, `z_eff_ss_bomb2.c`), and the
//! Shadow Temple's gate.
//!
//! The graveyard (`ootx scene-info --scene spot02_scene`): room 1 holds, in layers 0 to 3, the
//! gravestone (`Bg_Spot02_Objects` 2 at (762, 180, 80)), Dampé's grave's stone (1 at (-578, 120,
//! -336)), the lightning (3 at (685, 180, 85)), the beam (4 at (685, 180, 137)), the spark
//! (`Object_Kankyo` 4 at (685, 180, 33)) and `En_Okarina_Tag` 0x1BFF (type 6, Zelda's Lullaby) at
//! (602, 181, 81); room 0 the gate (`Bg_Spot02_Objects` 0x1F00 at (2676, 100, 80)).
//!
//! `gGraveyardTombOpeningChildCs` (`ootx cutscene`, 1637 frames) cues the gravestone on channel
//! 3 (id 2 from 392), the lightning on 0 (id 2 at 349-380 and 410-560), the spark on 1 (id 2 at
//! 302-340 and 345-398) and the beam on 2 (id 2 from 351 to 400: `rot.z` 0, then 910 at 371, 3640
//! from 372). `Play_Update` runs `Actor_UpdateAll` before `Cutscene_UpdateScripted`: an actor's
//! update sees the cues (and `curFrame`) as the last frame left them, its draw this frame's. The
//! tests take a snapshot of the cues before each frame for what the updates see.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_CUP, PadState};
use eng_math::smooth_step_to_f;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_spot02_objects::{self as spot02, Action, BgSpot02Objects, DrawFn};
use oot_actors::object_kankyo::{self, ObjectKankyo};
use oot_game::actor::{ACTOR_FLAG_IGNORE_POINT_LIGHTS, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_ITEMACTION, ActorHandle, ActorImpl};
use oot_game::audio::ocarina::*;
use oot_game::audio::sfx::{NA_SE_EV_STONEDOOR_STOP, SFX_FLAG};
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::effect::bomb2;
use oot_game::effect::{EFFECT_SS_BOMB2, EFFECT_SS_HAHEN, SsDraw, lerp_inv};
use oot_game::message::{MSGMODE_OCARINA_PLAYING, OCARINA_ACTION_CHECK_LULLABY};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Link (a child, or an adult with `adult`) entered at `entrance` at `hour`:00 with
/// `ocarina-songs`, `edit` applied to the save last, the sound log on and the audio side running;
/// five frames on (the rooms' objects loaded, the actors' inits run).
fn enter(a: &Arc<GameAssets>, entrance: &str, adult: bool, hour: i32, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset("ocarina-songs").unwrap();
    // The graveyard's first visit seen (sEntranceCutsceneTable: ENTR_GRAVEYARD_0's
    // gGraveyardIntroCs plays until EVENTCHKINF_B6).
    save.set_event_chk_inf(EVENTCHKINF_B6);
    edit(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    idle(&mut w, 5);
    assert_eq!(w.scene_id, spot02::SCENE_GRAVEYARD, "the graveyard");
    assert_eq!(w.cs_ctx.state, CS_STATE_IDLE, "no cutscene on entering");
    w
}

/// `EVENTCHKINF_B6` (`save.h`: 0xB6): the graveyard's intro seen.
const EVENTCHKINF_B6: u16 = 0xB6;

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// One frame; the sounds it asked for (`Audio_PlaySfxGeneral`'s requests).
fn tick(w: &mut PlayState) -> Vec<u16> {
    let n = w.audio.log.as_ref().unwrap().sfx.len();
    w.tick_with(scripted_input(PadState::default(), PadState::default()));
    w.audio.log.as_ref().unwrap().sfx[n..].iter().map(|&(_, id, _)| id).collect()
}

fn press(w: &mut PlayState, b: u16, frames: usize) {
    let pad = PadState { button: b, ..Default::default() };
    for i in 0..frames {
        w.tick_with(scripted_input(if i == 0 { PadState::default() } else { pad }, pad));
    }
    w.tick_with(scripted_input(pad, PadState::default()));
}

/// `song`'s notes (the pack's `gOcarinaSongButtons`), each held 3 frames and let go 2.
fn play_song(w: &mut PlayState, song: u8) {
    let b = w.audio.ocarina.song_buttons(song);
    let notes: Vec<u16> = b.buttons_index[..b.num_buttons as usize].iter().map(|&i| [BTN_A, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT, BTN_CUP][i as usize]).collect();
    for n in notes {
        press(w, n, 3);
        idle(w, 1);
    }
}

fn until(w: &mut PlayState, max: usize, what: &str, f: impl Fn(&PlayState) -> bool) {
    for _ in 0..max {
        if f(w) {
            return;
        }
        idle(w, 1);
    }
    panic!("{what}: not after {max} frames (cs state {}, frame {})", w.cs_ctx.state, w.cs_ctx.frames);
}

fn base(w: &PlayState, h: ActorHandle) -> &Actor {
    w.actors.actor(h).unwrap()
}

fn alive(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.actor(h).is_some_and(|a| !a.killed)
}

/// The live `Bg_Spot02_Objects` of type `ty`.
fn spot02_of(w: &PlayState, ty: i16) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<BgSpot02Objects>(h).is_some_and(|a| !a.actor.killed && a.actor.params == ty)).collect()
}

fn one_spot02(w: &PlayState, ty: i16) -> ActorHandle {
    let v = spot02_of(w, ty);
    assert_eq!(v.len(), 1, "one Bg_Spot02_Objects of type {ty}");
    v[0]
}

fn obj(w: &PlayState, h: ActorHandle) -> &BgSpot02Objects {
    w.actors.downcast::<BgSpot02Objects>(h).unwrap()
}

fn kankyo_of(w: &PlayState, params: i16) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<ObjectKankyo>(h).is_some_and(|a| !a.actor.killed && a.actor.params == params)).collect()
}

fn kankyo(w: &PlayState, h: ActorHandle) -> &ObjectKankyo {
    w.actors.downcast::<ObjectKankyo>(h).unwrap()
}

/// The cue ids on channels 0 to 3 while the cutscene runs (`None` without a cue or a cutscene).
fn cues(w: &PlayState) -> [Option<u16>; 4] {
    let on = w.cs_ctx.state != CS_STATE_IDLE;
    std::array::from_fn(|i| if on { w.cs_ctx.npc_actions[i].map(|c| c.action) } else { None })
}

#[test]
fn inits_by_type_age_and_the_tomb_flag() {
    let Some(a) = assets() else { return };
    println!("the pack loaded: running");
    // A child by day (layer 0), the tomb shut: room 1's four.
    let w = enter(&a, "ENTR_GRAVEYARD_0", false, 10, |_| {});
    // Type 2 in the graveyard: func_808AC908, drawn (BgSpot02Objects_Draw), a BG actor at
    // scale 0.1 with its collision.
    let stone = one_spot02(&w, spot02::TYPE_TOMB_STONE);
    let o = obj(&w, stone);
    assert_eq!(o.action, Action::TombStoneWait);
    assert_eq!(o.draw_fn, DrawFn::Default);
    assert_eq!(o.actor.category, ACTORCAT_BG);
    assert_eq!(o.actor.scale, Vec3::splat(0.1));
    assert!(o.dyna_bg_id().is_some(), "the gravestone's collision");
    assert_eq!(o.actor.world_pos, Vec3::new(762.0, 180.0, 80.0));
    // Type 1 for a child: func_808AC8FC, ACTOR_FLAG_IGNORE_POINT_LIGHTS.
    let dampe = one_spot02(&w, spot02::TYPE_DAMPE_STONE);
    assert_eq!(obj(&w, dampe).action, Action::Idle);
    assert_ne!(base(&w, dampe).flags & ACTOR_FLAG_IGNORE_POINT_LIGHTS, 0);
    assert!(obj(&w, dampe).dyna_bg_id().is_some());
    // Type 3: unk_16A 0, ACTORCAT_ITEMACTION, func_808ACC34 and func_808ACCB8; no collision.
    let lightning = one_spot02(&w, spot02::TYPE_LIGHTNING);
    let o = obj(&w, lightning);
    assert_eq!((o.action, o.draw_fn, o.unk_16a, o.actor.category), (Action::Lightning, DrawFn::Lightning, 0, ACTORCAT_ITEMACTION));
    assert!(o.dyna_bg_id().is_none());
    assert!(w.actors.category(ACTORCAT_ITEMACTION).contains(&lightning));
    // Type 4: timer -12, unk_170 0xFFFF, ACTORCAT_ITEMACTION.
    let beam = one_spot02(&w, spot02::TYPE_BEAM);
    let o = obj(&w, beam);
    assert_eq!((o.action, o.draw_fn, o.timer, o.unk_170, o.actor.category), (Action::Beam, DrawFn::Beam, -12, 0xFFFF, ACTORCAT_ITEMACTION));
    // Room 0 (the gate) isn't loaded from room 1.
    assert!(spot02_of(&w, spot02::TYPE_SHADOW_GATE).is_empty());
    // Object_Kankyo 4: object_spot02_objects loaded, ObjectKankyo_SunGraveSpark with size 7 and
    // alpha 0 (ObjectKankyo_WaitForSunGraveSparkObject).
    let k = kankyo_of(&w, 4);
    assert_eq!(k.len(), 1);
    let s = kankyo(&w, k[0]);
    assert_eq!(s.action, object_kankyo::Action::SunGraveSpark);
    assert!(s.required_object_loaded);
    assert_eq!((s.effects[0].size, s.effects[0].alpha), (7.0, 0));
    // No cutscene: nothing drawn by the cue draws.
    assert!(obj(&w, lightning).cue_draw.is_none() && obj(&w, beam).cue_draw.is_none() && s.cue_draw.is_none());

    // Type 2 outside the graveyard: func_808AC8FC (the play state's scene set to Kakariko's for
    // its init).
    let mut w2 = w;
    w2.scene_id = 0x52;
    let h = w2.actor_spawn(spot02::ACTOR_BG_SPOT02_OBJECTS, Vec3::new(0.0, 0.0, 0.0), [0, 0, 0], 2).unwrap();
    w2.scene_id = spot02::SCENE_GRAVEYARD;
    assert_eq!(obj(&w2, h).action, Action::Idle);

    // The tomb open (EVENTCHKINF_1D): the gravestone and the lightning are gone from the start
    // (killed at init); Dampé's stone and the beam stay.
    let w = enter(&a, "ENTR_GRAVEYARD_0", false, 10, |s| s.set_event_chk_inf(spot02::EVENTCHKINF_1D));
    assert!(spot02_of(&w, spot02::TYPE_TOMB_STONE).is_empty());
    assert!(spot02_of(&w, spot02::TYPE_LIGHTNING).is_empty());
    assert_eq!(spot02_of(&w, spot02::TYPE_DAMPE_STONE).len(), 1);
    assert_eq!(spot02_of(&w, spot02::TYPE_BEAM).len(), 1);

    // An adult (layer 2), the tomb shut: Dampé's stone is gone (LINK_IS_ADULT && params == 1);
    // the gravestone stays.
    let w = enter(&a, "ENTR_GRAVEYARD_0", true, 10, |_| {});
    assert!(spot02_of(&w, spot02::TYPE_DAMPE_STONE).is_empty());
    assert_eq!(obj(&w, one_spot02(&w, spot02::TYPE_TOMB_STONE)).action, Action::TombStoneWait);
    // An adult with the tomb open: neither.
    let w = enter(&a, "ENTR_GRAVEYARD_0", true, 10, |s| s.set_event_chk_inf(spot02::EVENTCHKINF_1D));
    assert!(spot02_of(&w, spot02::TYPE_DAMPE_STONE).is_empty());
    assert!(spot02_of(&w, spot02::TYPE_TOMB_STONE).is_empty());
}

/// What the test expects of `ObjectKankyo_Lightning`'s state machine (replayed from the C).
#[derive(Default)]
struct KankyoLightning {
    state: u8,
    timer: u16,
}

#[test]
fn the_tomb_opening_cutscene() {
    let Some(a) = assets() else { return };
    println!("the pack loaded: running");
    let mut w = enter(&a, "ENTR_GRAVEYARD_0", false, 10, |_| {});
    let stone = one_spot02(&w, spot02::TYPE_TOMB_STONE);
    let lightning = one_spot02(&w, spot02::TYPE_LIGHTNING);
    let beam = one_spot02(&w, spot02::TYPE_BEAM);
    let spark = kankyo_of(&w, 4)[0];
    // Object_Kankyo 2 isn't placed in layers 0 to 3 (layer 5's): one spawned for the cutscene's
    // channel 0.
    let k2 = w.actor_spawn(object_kankyo::ACTOR_OBJECT_KANKYO, Vec3::new(762.0, 214.0, 80.0), [0, 0, 0], 2).unwrap();
    assert_eq!(kankyo(&w, k2).action, object_kankyo::Action::Lightning);
    // As En_Okarina_Tag's func_80ABF4C8 starts it (without its EVENTCHKINF_1D: the gravestone
    // sets it itself at its cue).
    w.cs_ctx.segment = w.cutscene_script("gGraveyardTombOpeningChildCs");
    assert!(w.cs_ctx.segment.is_some(), "the script");
    w.save.cutscene_trigger = 1;

    let mut lightning_frames = 0u8; // the frames func_808ACC34 saw channel 0 at id 2
    let mut lightning_killed_at = None;
    let mut lightning_sfx_frames = Vec::new();
    let mut beam_timer = -12i16;
    let mut beam_sfx_at = None;
    let mut beam_killed_at = None;
    let mut beam_drawn_long = false;
    let mut explosion_at = None;
    let mut cry_heard = false;
    let mut spark_alpha = 0u8;
    let mut spark_size = 7.0f32;
    let mut spark_amp = 0.0f32;
    let mut spark_amp_state = 0u8;
    let mut spark_tex = 0u16;
    let mut spark_frames = 0;
    let mut k2_state = KankyoLightning::default();
    let mut k2_strikes = 0;
    let mut stone_states = Vec::new();
    for _ in 0..700 {
        // What this frame's updates see: the cues and curFrame as the last frame left them.
        let seen = cues(&w);
        let seen_frame = w.cs_ctx.frames;
        let on = w.cs_ctx.state != CS_STATE_IDLE;
        let sfx = tick(&mut w);
        let now = cues(&w);

        // The lightning (func_808ACC34): one more frame per frame at id 2, gone at the 12th; its
        // sound at curFrame 245 and 351 (Sfx_PlaySfxCentered2).
        if lightning_killed_at.is_none() {
            if seen[spot02::CUE_LIGHTNING] == Some(2) {
                lightning_frames += 1;
            }
            if lightning_frames >= 12 {
                assert!(!alive(&w, lightning), "the lightning gone at its 12th frame");
                lightning_killed_at = Some(seen_frame);
            } else {
                let o = obj(&w, lightning);
                assert_eq!(o.unk_16a, lightning_frames);
                // func_808ACCB8 draws while this frame's cue is id 2: texture unk_16A, its colours.
                if now[spot02::CUE_LIGHTNING] == Some(2) {
                    let d = o.cue_draw.as_ref().expect("the lightning drawn");
                    assert_eq!(d.bake, format!("Bg_Spot02_Objects/lightning/{lightning_frames}"));
                    assert_eq!((d.prim, d.env), spot02::lightning_colors(lightning_frames));
                    // At the cue's start, (762, 214, 80).
                    assert_eq!(d.mtx.w_axis.truncate(), Vec3::new(762.0, 214.0, 80.0));
                } else {
                    assert!(o.cue_draw.is_none());
                }
            }
        }
        if on && (seen_frame == 245 || seen_frame == 351) && sfx.contains(&spot02::NA_SE_EV_LIGHTNING) {
            lightning_sfx_frames.push(seen_frame);
        }

        // The beam (func_808AD3D4): the timer from -12 at id 2, the sound at 2, gone at 32.
        if beam_killed_at.is_none() && seen[spot02::CUE_BEAM] == Some(2) {
            if beam_timer == 2 {
                assert!(sfx.contains(&spot02::NA_SE_IT_EXPLOSION_ICE), "the beam's sound at timer 2");
                beam_sfx_at = Some(seen_frame);
            }
            if beam_timer < 32 {
                beam_timer += 1;
                assert_eq!(obj(&w, beam).timer, beam_timer);
            } else {
                assert!(!alive(&w, beam), "the beam gone at 32");
                beam_killed_at = Some(seen_frame);
            }
        }
        if beam_killed_at.is_none() && alive(&w, beam) {
            let o = obj(&w, beam);
            // func_808AD450: from frame 372 rot.z is 3640 (19 degrees, CAM_BINANG_TO_DEG
            // truncated), the one before 910 (4): the length lerps from 4 to 19 over 372-400.
            if now[spot02::CUE_BEAM] == Some(2) && w.cs_ctx.frames > 373 && w.cs_ctx.frames < 400 {
                assert_eq!((o.unk_170, o.unk_172), (19, 4));
                let lerp = oot_game::env::lerp_weight(400, 372, w.cs_ctx.frames);
                let len = (((19 - 4) as f32 * lerp) + 4.0) * 0.1;
                let d = o.cue_draw.as_ref().expect("the beam drawn");
                assert!((d.mtx.z_axis.truncate().length() - len).abs() < 1e-3, "the beam's length {len}");
                assert!((d.mtx.x_axis.truncate().length() - 0.9).abs() < 1e-4);
                assert_eq!(d.timer, o.timer);
                beam_drawn_long = true;
            }
        }

        // The gravestone (func_808AC908, func_808ACA08).
        if alive(&w, stone) || explosion_at.is_some() {
            if explosion_at.is_none() && seen[spot02::CUE_TOMB_STONE] == Some(2) {
                explosion_at = Some(seen_frame);
                let o = obj(&w, stone);
                assert_eq!((o.action, o.timer), (Action::TombStoneExplode, 25));
                assert!(w.save.get_event_chk_inf(spot02::EVENTCHKINF_1D), "EVENTCHKINF_1D");
                assert!(sfx.contains(&spot02::NA_SE_EV_GRAVE_EXPLOSION), "NA_SE_EV_GRAVE_EXPLOSION");
                // EffectSsBomb2_SpawnLayered 50 in front (shape.rot.y -0x4000: -x) and 30 up,
                // scale 70 step 30; its first EffectSs_UpdateAll this frame: life 23, rTexIndex
                // (23 - 23) / 3 = 0, rScale 100, rDepth 4 (the step is 30).
                let e = w.effect_ss.table.iter().find(|e| e.life > -1 && e.ty == EFFECT_SS_BOMB2).expect("the blast");
                assert_eq!(e.draw, Some(SsDraw::Bomb2Layered));
                assert!((e.pos - Vec3::new(712.0, 210.0, 80.0)).length() < 1e-3, "{:?}", e.pos);
                assert_eq!(e.life, 23);
                assert_eq!((e.regs[bomb2::R_TEX_INDEX], e.regs[bomb2::R_SCALE], e.regs[bomb2::R_DEPTH]), (0, 100, 4));
            } else if let Some(at) = explosion_at {
                stone_states.push((seen_frame - at, alive(&w, stone), w.actors.downcast::<BgSpot02Objects>(stone).map(|o| (o.timer, o.draw_fn))));
            } else {
                assert!(!w.save.get_event_chk_inf(spot02::EVENTCHKINF_1D), "EVENTCHKINF_1D before the cue");
                assert_eq!(obj(&w, stone).action, Action::TombStoneWait);
            }
        }
        // Link's cry at curFrame 402 (NA_SE_VO_LI_DEMO_DAMAGE_KID for a child).
        if on && seen_frame == 402 && sfx.contains(&spot02::NA_SE_VO_LI_DEMO_DAMAGE_KID) {
            cry_heard = true;
        }

        // Object_Kankyo 4 (ObjectKankyo_SunGraveSpark): at id 2, NA_SE_EN_BIRI_SPARK
        // (flagged), alpha 20 more to 255, size smoothed to 1.8.
        if seen[object_kankyo::CUE_SPARK] == Some(2) {
            spark_frames += 1;
            spark_alpha = if spark_alpha as i16 + 20 > 255 { 255 } else { spark_alpha + 20 };
            smooth_step_to_f(&mut spark_size, 1.8, 0.5, 0.28, 0.01);
            assert_eq!(base(&w, spark).sfx, object_kankyo::NA_SE_EN_BIRI_SPARK - SFX_FLAG);
        }
        let s = kankyo(&w, spark);
        assert_eq!((s.effects[0].alpha, s.effects[0].size), (spark_alpha, spark_size));
        // ObjectKankyo_DrawSunGraveSpark, at this frame's id 2: the brightness up or down a
        // seventh, the texture on (past 7 to 0).
        if now[object_kankyo::CUE_SPARK] == Some(2) {
            if spark_amp_state == 0 {
                spark_amp += 1.0 / 7.0;
                if spark_amp >= 1.0 {
                    spark_amp = 1.0;
                    spark_amp_state += 1;
                }
            } else {
                spark_amp -= 1.0 / 7.0;
                if spark_amp <= 0.1 {
                    spark_amp = 0.0;
                    spark_amp_state = 0;
                }
            }
            spark_tex += 1;
            if spark_tex > 7 {
                spark_tex = 0;
            }
            let d = s.cue_draw.as_ref().expect("the spark drawn");
            assert_eq!(d.bake, format!("Object_Kankyo/sun_grave_spark/{spark_tex}"));
            let p = (105.0 * spark_amp) as i32 as u8 + 150;
            assert_eq!(d.prim, [p, 255, p, spark_alpha]);
            assert_eq!(d.env, [(155.0 * spark_amp) as i32 as u8 + 100, (255.0 * spark_amp) as i32 as u8, 255 - (255.0 * spark_amp) as i32 as u8, spark_alpha]);
            assert!((d.mtx.x_axis.truncate().length() - spark_size).abs() < 1e-4);
        } else {
            assert!(s.cue_draw.is_none());
        }
        assert_eq!((s.effects[0].amplitude, s.effects[0].state, s.effects[0].timer), (spark_amp, spark_amp_state, spark_tex));

        // Object_Kankyo 2 (ObjectKankyo_Lightning): replayed.
        if let Some(id) = seen[object_kankyo::CUE_LIGHTNING] {
            match k2_state.state {
                0 => {
                    k2_state.timer = 0;
                    if id == 2 {
                        k2_state.state = 1;
                        k2_strikes += 1;
                    }
                }
                1 => {
                    k2_state.timer += 1;
                    if k2_state.timer >= 7 {
                        k2_state.state = 2;
                    }
                }
                2 => {
                    if id == 1 {
                        k2_state.state = 0;
                    }
                }
                _ => {}
            }
        }
        let k = kankyo(&w, k2);
        assert_eq!((k.effects[0].state, k.effects[0].timer), (k2_state.state, k2_state.timer));
        match (&k.cue_draw, k2_state.state) {
            (Some(d), 1) => {
                assert_eq!(d.bake, format!("Object_Kankyo/lightning/{}", k2_state.timer));
                assert_eq!(d.mtx.w_axis.truncate(), Vec3::new(762.0, 214.0, 80.0));
            }
            (None, s) => assert_ne!(s, 1, "the lightning drawn in state 1"),
            (Some(_), s) => panic!("the lightning drawn in state {s}"),
        }
        if beam_killed_at.is_some() && explosion_at.is_some_and(|at| seen_frame > at + 30) {
            break;
        }
    }

    println!("lightning gone at {lightning_killed_at:?}, its sounds {lightning_sfx_frames:?}; the beam's sound {beam_sfx_at:?}, gone {beam_killed_at:?}; the explosion {explosion_at:?}; spark frames {spark_frames}, strikes {k2_strikes}");
    // The lightning: its 12 frames from 349 (the first frame at id 2 seen at curFrame 350, the
    // update a frame behind the cue), its sounds at 245 and 351.
    assert_eq!(lightning_killed_at, Some(361), "the lightning's 12th frame");
    assert_eq!(lightning_sfx_frames, vec![245, 351]);
    // The beam: id 2 seen from curFrame 352; its timer -12 reaches 2 on the 15th frame (366), 32
    // after 44 (the 45th frame, 396, kills it).
    assert_eq!(beam_sfx_at, Some(366));
    assert_eq!(beam_killed_at, Some(396));
    assert!(beam_drawn_long, "the beam drawn with its length lerped");
    // The gravestone: id 2 from 392, seen at curFrame 393.
    assert_eq!(explosion_at, Some(393));
    // 25 frames: 24 .. 21 drawn, at 20 its draw gone (DrawFn::None) and the fragments; killed at 0.
    for &(n, live, state) in &stone_states {
        let timer = 25 - n as i16;
        if timer > 20 {
            assert_eq!(state, Some((timer, DrawFn::Default)), "frame {n} after the explosion");
        } else if timer > 0 {
            assert!(live);
            assert_eq!(state, Some((timer, DrawFn::None)), "frame {n} after the explosion");
        } else {
            assert!(!live, "the gravestone gone 25 frames after");
        }
    }
    assert!(stone_states.iter().any(|&(n, ..)| n == 25));
    assert!(cry_heard, "Link's cry at curFrame 402");
    // The spark at id 2: 302-340 and 345-398, each seen a frame later (38 + 53).
    assert_eq!(spark_frames, 38 + 53);
    assert_eq!(spark_alpha, 255);
    // Object_Kankyo 2 strikes at 349 and 410.
    assert_eq!(k2_strikes, 2);
}

#[test]
fn the_gravestones_fragments_and_blast() {
    let Some(a) = assets() else { return };
    println!("the pack loaded: running");
    let mut w = enter(&a, "ENTR_GRAVEYARD_0", false, 10, |_| {});
    let stone = one_spot02(&w, spot02::TYPE_TOMB_STONE);
    w.cs_ctx.segment = w.cutscene_script("gGraveyardTombOpeningChildCs");
    w.save.cutscene_trigger = 1;
    until(&mut w, 500, "the explosion", |w| w.actors.downcast::<BgSpot02Objects>(stone).is_some_and(|o| o.action == Action::TombStoneExplode));
    // The blast's colours (EffectSsBomb2_Update), one frame at a time: life 23 leaves them;
    // life 22 (divisor 9): prim B 255 + (150 - 255) / 9 = 244 (-11.67 truncated), env R
    // 0 + 150 / 9 = 16, env B 200 - 200 / 9 = 178.
    let blast = w.effect_ss.table.iter().position(|e| e.life > -1 && e.ty == EFFECT_SS_BOMB2).expect("the blast");
    let r = w.effect_ss.table[blast].regs;
    assert_eq!([r[bomb2::R_PRIM_COLOR_B], r[bomb2::R_ENV_COLOR_R], r[bomb2::R_ENV_COLOR_B]], [255, 0, 200]);
    let mut regs = r;
    for life in (0..23i16).rev() {
        idle(&mut w, 1);
        let e = &w.effect_ss.table[blast];
        assert_eq!((e.ty, e.life), (EFFECT_SS_BOMB2, life));
        // rTexIndex (23 - life) / 3; rScale 30 more; rDepth 4 more.
        assert_eq!(e.regs[bomb2::R_TEX_INDEX], (23 - life) / 3);
        assert_eq!(e.regs[bomb2::R_SCALE], 100 + 30 * (23 - life));
        assert_eq!(e.regs[bomb2::R_DEPTH], 4 * (24 - life));
        if life == 22 {
            assert_eq!([e.regs[bomb2::R_PRIM_COLOR_B], e.regs[bomb2::R_ENV_COLOR_R], e.regs[bomb2::R_ENV_COLOR_B]], [244, 16, 178]);
        }
        // The colours to yellow and red over 22 to 14, to grey over 13 to 0.
        let (targets, divisor) = if life > 13 { ([255, 255, 150, 255, 150, 0, 0], life as i32 - 13) } else { ([50, 50, 50, 150, 10, 10, 10], life as i32 + 1) };
        for (k, t) in (bomb2::R_PRIM_COLOR_R..=bomb2::R_ENV_COLOR_B).zip(targets) {
            regs[k] = lerp_inv(regs[k], t, divisor);
        }
        assert_eq!(e.regs[bomb2::R_PRIM_COLOR_R..=bomb2::R_ENV_COLOR_B], regs[bomb2::R_PRIM_COLOR_R..=bomb2::R_ENV_COLOR_B]);
        // Life 0's colours are the targets exactly (EffectSs_LerpInv by 1).
        if life == 0 {
            assert_eq!(e.regs[bomb2::R_PRIM_COLOR_R..=bomb2::R_ENV_COLOR_B], [50, 50, 50, 150, 10, 10, 10]);
        }
        // The fragments: at the stone's timer 20 (5 frames after), 40 of
        // object_spot02_objects_DL_012D30 (as many as find a slot).
        if life == 18 {
            let n = w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == EFFECT_SS_HAHEN && e.gfx == Some((spot02::OBJECT, spot02::FRAGMENT_DL))).count();
            assert!(n >= 30, "{n} fragments");
        }
    }
    // The layered draw: three quads (the blast and its two layers), the last turned and 0.925 ×
    // 0.775 the size of the first.
    let m = bomb2::layered_matrices(Vec3::ZERO, glam::Mat4::IDENTITY, 1.0, 10.0, 5);
    assert!((m[2].x_axis.truncate().length() - 0.925 * 0.775).abs() < 1e-5);
    assert!((m[1].w_axis.z - 10.0).abs() < 1e-5);
    idle(&mut w, 1);
    assert_eq!(w.effect_ss.table[blast].life, -1, "the blast's 24 frames over");
}

#[test]
fn zeldas_lullaby_at_the_tomb_opens_it() {
    let Some(a) = assets() else { return };
    println!("the pack loaded: running");
    let mut w = enter(&a, "ENTR_GRAVEYARD_0", false, 10, |_| {});
    let stone = one_spot02(&w, spot02::TYPE_TOMB_STONE);
    // Link at the tag (602, 181, 81), within its 50: the ocarina out starts
    // OCARINA_ACTION_CHECK_LULLABY (func_80ABF28C, type 6).
    let from = Vec3::new(590.0, 230.0, 81.0);
    let (y, _) = w.col.entity_raycast_down(from);
    w.place_player(Vec3::new(from.x, y, from.z), 0x4000);
    idle(&mut w, 2);
    press(&mut w, BTN_CLEFT, 1);
    until(&mut w, 40, "CHECK_LULLABY", |w| w.msg_ctx.ocarina_action == OCARINA_ACTION_CHECK_LULLABY && w.msg_ctx.msg_mode == MSGMODE_OCARINA_PLAYING);
    play_song(&mut w, OCARINA_SONG_LULLABY);
    // func_80ABF4C8: gGraveyardTombOpeningChildCs, EVENTCHKINF_1D (the tag sets it first).
    until(&mut w, 400, "the cutscene", |w| w.cs_ctx.state != CS_STATE_IDLE);
    assert!(w.save.get_event_chk_inf(spot02::EVENTCHKINF_1D));
    // The gravestone blows up at its cue and is gone 25 frames later.
    until(&mut w, 600, "the explosion", |w| w.actors.downcast::<BgSpot02Objects>(stone).is_some_and(|o| o.action == Action::TombStoneExplode));
    idle(&mut w, 25);
    assert!(!alive(&w, stone));
}

#[test]
fn the_shadow_temples_gate_rises_on_its_flag() {
    let Some(a) = assets() else { return };
    println!("the pack loaded: running");
    // ENTR_GRAVEYARD_1: room 0, the gate (0x1F00: type 0, switch flag 0x1F) at (2676, 100, 80).
    let mut w = enter(&a, "ENTR_GRAVEYARD_1", false, 10, |_| {});
    let gate = one_spot02(&w, spot02::TYPE_SHADOW_GATE);
    let o = obj(&w, gate);
    assert_eq!((o.unk_16b, o.action, o.actor.world_pos.y), (0x1F, Action::GateWait, 100.0));
    idle(&mut w, 3);
    assert_eq!(obj(&w, gate).action, Action::GateWait);
    // The flag: func_808ACAFC: focus 60 up, OnePointCutscene_Attention, func_808ACB58.
    w.flags.set_switch(0x1F);
    tick(&mut w);
    let o = obj(&w, gate);
    assert_eq!((o.action, o.actor.world_pos.y), (Action::GateRise, 100.0));
    assert_eq!(o.actor.focus_pos.y, 160.0);
    let attention = (0..oot_game::camera::NUM_CAMS as i16).any(|i| w.camera(i).is_some_and(|c| c.cs_id == 5010 && c.target == Some(gate)));
    assert!(attention, "the attention camera on the gate");
    // 1 a frame (Math_StepToF): 254 frames of NA_SE_EV_WALL_MOVE_SP (flagged), the 255th reaches
    // 355 with NA_SE_EV_STONEDOOR_STOP.
    for n in 1..=255 {
        let sfx = tick(&mut w);
        let o = obj(&w, gate);
        assert_eq!(o.actor.world_pos.y, 100.0 + n as f32);
        if n < 255 {
            assert_eq!(o.action, Action::GateRise);
            assert_eq!(o.actor.sfx, spot02::NA_SE_EV_WALL_MOVE_SP - SFX_FLAG);
        } else {
            assert_eq!(o.action, Action::Idle);
            assert!(sfx.contains(&NA_SE_EV_STONEDOOR_STOP), "NA_SE_EV_STONEDOOR_STOP");
        }
    }
    idle(&mut w, 2);
    assert_eq!(base(&w, gate).world_pos.y, 355.0);
    // With the flag set at init: risen 255 at once (func_808AC8FC).
    let h = w.actor_spawn(spot02::ACTOR_BG_SPOT02_OBJECTS, Vec3::new(2676.0, 100.0, 80.0), [0, -16384, 0], 0x1F00).unwrap();
    let o = obj(&w, h);
    assert_eq!((o.action, o.actor.world_pos.y), (Action::Idle, 355.0));
}
