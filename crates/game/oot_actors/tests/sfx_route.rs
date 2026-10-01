//! GAME-04's exit test (Phase 5): a scripted run's sound effect requests against the calls in
//! the C, frame by frame, with the audio offline beside the game and every request logged with
//! where it is (`GameAudio::log`). The runs are `oot_actors::playthrough`'s.
//!
//! On the Mido and shop run (`Route::MidoShop`):
//! - **The Kokiri Sword's chest** (a big chest, `ENBOX_TYPE_BIG`, opened on a major item: its
//!   `unk_1F4` 1). On the frame Player's A sets it, `EnBox_WaitOpen` starts the long opening
//!   (`SkelAnime` at 1.5, a frame step of `R_UPDATE_RATE / 3`: 1.5 frames a game frame), spawns
//!   the light (`Demo_Tre_Lgt`) and plays the chest fanfare (`Audio_PlayFanfare(NA_BGM_OPEN_TRE_BOX
//!   | 0x900)`, started by the same frame's `func_800F5CF8`: `start_seq(SEQ_PLAYER_FANFARE, 1,
//!   ...)`). `EnBox_Open` plays `NA_SE_EV_TBOX_UNLOCK` on the animation's frame 30 and
//!   `NA_SE_EV_TBOX_OPEN` on 90, at the chest: 20 and 60 frames on. The light waits for the
//!   chest's frame 10 (reading it before the chest's own update: 8 frames on, the chest at 10.5),
//!   starts its curve there plus a step (`SkelCurve_Update`: 1.5 a frame, 12), and flashes
//!   (`NA_SE_EV_TRE_BOX_FLASH`, at the light) once past 30: 14 frames later, 22 on. The item's
//!   text starts the item fanfare (`func_8084DFF4`: `NA_BGM_ITEM_GET | 0x900`).
//! - **Mido's four chests**, kicked open (`unk_1F4` -1): the lid's sounds as far as their
//!   animation goes, and for their rupees `NA_SE_SY_GET_BOXITEM` (`func_8084DFF4`) in place of a
//!   fanfare.
//! - **Rupees:** `EnItem00_Update` plays `NA_SE_SY_GET_RUPY` (no position) on the frame Link
//!   takes one; `Interface_Update` plays `NA_SE_SY_RUPY_COUNT` on each frame it counts the
//!   accumulator into the wallet (in, and out at the shop).
//! - **The wonder items' drops** (`EnWonderItem_DropCollectible`: `NA_SE_SY_GET_ITEM`, no
//!   position) on the frame each drops and goes.
//! - **Navi** (`func_80A0461C`): `NA_SE_EV_NAVY_VANISH` at her, on each frame she goes from
//!   following (0) into Link's hat (7), unless she's silenced (`unk_2C7`).
//!
//! Kokiri Forest has no `En_Door` (its houses have doorways), so **the doors** are the door
//! test's (`tests/door.rs`): a Kakariko house's scene exit and the souko's room door.
//! `EnDoor_Open` (`SkelAnime` at 1.5 for a child, 1.5 frames a game frame) plays
//! `NA_SE_OC_DOOR_OPEN` on frame 25 (`sDoorAnimOpenFrames`) and `NA_SE_EV_DOOR_CLOSE` on 60 or 70
//! (`sDoorAnimCloseFrames`, by `openAnim`), at the door, while the door is there; the scene exit
//! leaves `gSaveContext.entranceSound`, which the next scene's `Player_Init` plays at Player.
//!
//! On the Deku Tree run (`Route::DekuTree`), **the bushes**: `EnKusa_Main` on the sword's hit
//! plays `NA_SE_EV_PLANT_BROKEN` through a fixed-position sound source
//! (`SfxSource_PlaySfxAtFixedWorldPos`, 20 frames) at the bush (the bush's collider is
//! `COLTYPE_NONE`: `CollisionCheck_HitEffects` plays nothing for it).

mod common;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use eng_input::pad::PadState;
use oot_actors::PlayExt;
use oot_actors::demo_tre_lgt::DemoTreLgt;
use oot_actors::en_box::{self, EnBox};
use oot_actors::en_door::{self, EnDoor};
use oot_actors::en_elf::EnElf;
use oot_actors::en_item00::{self, EnItem00};
use oot_actors::en_kusa::{self, EnKusa};
use oot_actors::en_wonder_item::{EnWonderItem, WONDERITEM_INTERACT_SWITCH};
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::bgm::start_seq;
use oot_game::audio::offline::OfflineAudio;
use oot_game::audio::sfx::*;
use oot_game::audio::{GameAudio, NA_BGM_ITEM_GET, NA_BGM_OPEN_TRE_BOX, SEQ_PLAYER_FANFARE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;

fn assets() -> Option<(Arc<GameAssets>, eng_audio::AudioData)> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    let data = pack.audio_data().expect("the pack's audio data");
    Some((oot_actors::game_assets(pack).expect("the pack's tables"), data))
}

/// The route's start, the audio's game side logging from its first frame, the audio side
/// offline.
fn start(a: &Arc<GameAssets>, data: &eng_audio::AudioData, route: Route) -> Option<PlayState> {
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let audio = GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), common::data()?, common::rules()?, route.save(e), audio).expect("Play_Init");
    w.audio_side = Some(Box::new(OfflineAudio::new(data, false)));
    Some(w)
}

/// What the checks read of a frame, after it ran.
#[derive(Default)]
struct Obs {
    frame: u32,
    scene_changes: u32,
    /// Each door: action, the animation's frame and play speed, `openAnim`.
    doors: Vec<(ActorHandle, en_door::Action, f32, u8)>,
    /// Each chest: action, `unk_1F4`, the animation's frame and length, its get-item id.
    chests: Vec<(ActorHandle, en_box::Action, i16, f32, f32, i16)>,
    /// Each `Demo_Tre_Lgt`: its parent.
    lights: Vec<(ActorHandle, Option<ActorHandle>)>,
    /// Navi: `unk_2A8`, `unk_2C7`.
    navi: Option<(ActorHandle, i16, u8)>,
    /// Each `En_Item00`: params, action, killed.
    items: Vec<(ActorHandle, i16, en_item00::Action, bool)>,
    /// Each `En_Wonder_Item`: killed, its mode, whether all its tags were touched.
    wonder: Vec<(ActorHandle, bool, i16, bool)>,
    /// Each bush: action, killed, position.
    bushes: Vec<(ActorHandle, en_kusa::Action, bool, glam::Vec3)>,
    rupees: i16,
}

fn observe(w: &PlayState) -> Obs {
    let mut o = Obs { frame: w.audio.frames, scene_changes: w.scene_changes, rupees: w.save.rupees, ..Default::default() };
    let navi = w.player.and_then(|_| w.player().navi_actor);
    for h in w.actors.all() {
        if let Some(d) = w.actors.downcast::<EnDoor>(h) {
            o.doors.push((h, d.action, d.skel.cur_frame, d.open_anim));
        } else if let Some(c) = w.actors.downcast::<EnBox>(h) {
            let (cur, len) = c.skel.as_ref().map(|s| (s.cur_frame, s.anim_length)).unwrap_or((0.0, 0.0));
            o.chests.push((h, c.action, c.unk_1f4, cur, len, c.get_item_id()));
        } else if let Some(l) = w.actors.downcast::<DemoTreLgt>(h) {
            o.lights.push((h, l.actor.parent));
        } else if let Some(e) = w.actors.downcast::<EnElf>(h)
            && Some(h) == navi
        {
            o.navi = Some((h, e.unk_2a8, e.unk_2c7));
        } else if let Some(i) = w.actors.downcast::<EnItem00>(h) {
            o.items.push((h, i.actor.params, i.action, i.actor.killed));
        } else if let Some(wi) = w.actors.downcast::<EnWonderItem>(h) {
            o.wonder.push((h, wi.actor.killed, wi.wonder_mode, wi.tag_count == wi.num_tag_points));
        } else if let Some(k) = w.actors.downcast::<EnKusa>(h) {
            o.bushes.push((h, k.action, k.actor.killed, k.actor.world_pos));
        }
    }
    o
}

/// Runs `route` to its end, observing every frame (index 0: before the first).
fn run(w: &mut PlayState, route: Route) -> (Vec<Obs>, Vec<(Step, u32)>) {
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut obs = vec![observe(w)];
    let mut steps = Vec::new();
    loop {
        let pad = run.next(w);
        if let Some(step) = run.take_done() {
            steps.push((step, w.audio.frames));
        }
        let Some(pad) = pad else { break };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        obs.push(observe(w));
    }
    assert_eq!(run.failure, None, "the run stopped at {}", run.at());
    (obs, steps)
}

/// The log's requests by frame (the C's silent id 0 aside).
fn requests(w: &PlayState) -> BTreeMap<u32, Vec<(u16, SfxPos)>> {
    let mut m: BTreeMap<u32, Vec<(u16, SfxPos)>> = BTreeMap::new();
    for &(f, id, pos) in &w.audio.log.as_ref().unwrap().sfx {
        if id != 0 {
            m.entry(f).or_default().push((id, pos));
        }
    }
    m
}

fn of(r: &BTreeMap<u32, Vec<(u16, SfxPos)>>, frame: u32, id: u16) -> Vec<SfxPos> {
    r.get(&frame).map(|v| v.iter().filter(|(i, _)| *i == id).map(|(_, p)| *p).collect()).unwrap_or_default()
}

/// `Animation_OnFrameImpl` for a `SkelAnime` stepped 1.5 a game frame from 0 (the C's
/// arithmetic): how many updates until it reaches `frame`.
fn updates_to(frame: f32, step: f32) -> u32 {
    let mut cur = 0.0f32;
    let mut k = 0;
    loop {
        k += 1;
        let prev = cur;
        cur += step;
        if prev < frame && frame <= cur {
            return k;
        }
    }
}

#[test]
fn the_mido_and_shop_run_sounds_as_the_c() {
    let Some((a, data)) = assets() else { return };
    let route = Route::MidoShop;
    let Some(mut w) = start(&a, &data, route) else { return };
    let (obs, steps) = run(&mut w, route);
    let r = requests(&w);
    let by_frame: HashMap<u32, usize> = obs.iter().enumerate().map(|(i, o)| (o.frame, i)).collect();
    let chest_step = steps.iter().find(|(s, _)| *s == Step::Chest).expect("the chest step").1;

    // The sword's chest: the frame it starts opening (unk_1F4 1, EnBox_Open).
    let (f0, chest, len) = obs.iter().find_map(|o| o.chests.iter().find(|c| c.1 == en_box::Action::Open && c.2 > 0).map(|c| (o.frame, c.0, c.4))).expect("the sword's chest opens");
    assert!(f0 < chest_step);
    // EnBox_Open's sounds, at the chest.
    let unlock = f0 + updates_to(30.0, 1.5);
    assert_eq!(unlock, f0 + 20);
    assert_eq!(of(&r, unlock, NA_SE_EV_TBOX_UNLOCK), [SfxPos::Actor(chest)]);
    if len >= 90.0 {
        assert_eq!(of(&r, f0 + updates_to(90.0, 1.5), NA_SE_EV_TBOX_OPEN), [SfxPos::Actor(chest)]);
    }
    let lids: Vec<u32> = r.iter().filter(|(_, v)| v.iter().any(|(id, p)| (*id == NA_SE_EV_TBOX_UNLOCK || *id == NA_SE_EV_TBOX_OPEN) && *p == SfxPos::Actor(chest))).map(|(f, _)| *f).collect();
    assert_eq!(lids, if len >= 90.0 { vec![f0 + 20, f0 + 60] } else { vec![f0 + 20] }, "the lid's sounds");
    // The light: spawned with the chest's opening, its flash 22 frames on, at it.
    let light = obs[by_frame[&(f0 + 1)]].lights.iter().find(|l| l.1 == Some(chest)).expect("Demo_Tre_Lgt, the chest's child").0;
    let flashes: Vec<(u32, SfxPos)> = r.iter().flat_map(|(f, v)| v.iter().filter(|(id, _)| *id == NA_SE_EV_TRE_BOX_FLASH).map(move |(_, p)| (*f, *p))).collect();
    assert_eq!(flashes, [(f0 + 22, SfxPos::Actor(light))]);
    // The fanfares: the chest's on its frame, the item's after.
    let seq_cmds = &w.audio.log.as_ref().unwrap().seq_cmds;
    let chest_fanfare = start_seq(SEQ_PLAYER_FANFARE, 1, NA_BGM_OPEN_TRE_BOX | 0x900);
    let item_fanfare = start_seq(SEQ_PLAYER_FANFARE, 1, NA_BGM_ITEM_GET | 0x900);
    assert_eq!(seq_cmds.iter().filter(|(_, c)| *c == chest_fanfare).map(|(f, _)| *f).collect::<Vec<_>>(), [f0]);
    let item_fanfares: Vec<u32> = seq_cmds.iter().filter(|(f, c)| *c == item_fanfare && *f > f0 && *f <= chest_step).map(|(f, _)| *f).collect();
    assert_eq!(item_fanfares.len(), 1, "the sword's fanfare");

    // Every chest on the way (the sword's, then Mido's four, kicked open: unk_1F4 -1): the lid's
    // sounds 20 and 60 frames into EnBox_Open, as far as its animation goes (an update that
    // ends it plays nothing); a rupee or a heart from a chest (func_8084DFF4) plays
    // NA_SE_SY_GET_BOXITEM in place of a fanfare.
    let mut openings = Vec::new();
    for (i, o) in obs.iter().enumerate().skip(1) {
        let p = &obs[i - 1];
        for &(h, action, unk_1f4, _, len, gi) in &o.chests {
            if action == en_box::Action::Open && p.scene_changes == o.scene_changes && p.chests.iter().any(|c| c.0 == h && c.1 == en_box::Action::WaitOpen) {
                openings.push((o.frame, h, unk_1f4, len, gi));
            }
        }
    }
    assert_eq!(openings.len(), 5, "the sword's chest and Mido's four: {openings:?}");
    for &(f, h, _, len, _) in &openings {
        let mut want = Vec::new();
        for (at, frame, id) in [(20, 30.0, NA_SE_EV_TBOX_UNLOCK), (60, 90.0, NA_SE_EV_TBOX_OPEN)] {
            if frame < len {
                want.push((f + at, id));
            }
        }
        let got: Vec<(u32, u16)> =
            r.iter().flat_map(|(fr, v)| v.iter().filter(|(id, p)| (*id == NA_SE_EV_TBOX_UNLOCK || *id == NA_SE_EV_TBOX_OPEN) && *p == SfxPos::Actor(h)).map(move |(id, _)| (*fr, *id))).collect();
        assert_eq!(got, want, "the chest opened on frame {f} (animation {len})");
    }
    let small_items = openings.iter().filter(|o| {
        let gi = o.4.abs();
        (oot_game::item::GI_RUPEE_GREEN..=oot_game::item::GI_RUPEE_RED).contains(&gi)
            || (oot_game::item::GI_RUPEE_PURPLE..=oot_game::item::GI_RUPEE_GOLD).contains(&gi)
            || gi == oot_game::item::GI_RECOVERY_HEART
    });
    let boxitems: Vec<SfxPos> = r.values().flatten().filter(|(id, _)| *id == NA_SE_SY_GET_BOXITEM).map(|(_, p)| *p).collect();
    assert_eq!(boxitems, vec![SfxPos::Default; small_items.count()]);
    assert!(!boxitems.is_empty());

    // No door on the way (Kokiri Forest's houses, Mido's and the shop have doorways, no
    // En_Door): no door's sound. (`doors_sound_as_the_c` has them.)
    assert!(obs.iter().all(|o| o.doors.is_empty()));
    assert!(r.values().flatten().all(|(id, _)| *id != NA_SE_OC_DOOR_OPEN && *id != NA_SE_EV_DOOR_CLOSE));

    // Rupees and the wonder items' drops.
    let mut got_rupees = 0;
    for (i, o) in obs.iter().enumerate().skip(1) {
        let p = &obs[i - 1];
        let same_scene = p.scene_changes == o.scene_changes;
        let collected = |rupee: bool| {
            o.items
                .iter()
                .filter(|&&(h, params, action, _)| {
                    let is_rupee = params <= en_item00::ITEM00_RUPEE_RED || params == en_item00::ITEM00_RUPEE_ORANGE;
                    is_rupee == rupee && action == en_item00::Action::Collected && same_scene && p.items.iter().any(|q| q.0 == h && q.2 != en_item00::Action::Collected)
                })
                .count()
        };
        // A switch's drop (EnWonderItem_InteractSwitch: not taken at once) is spawned before the
        // items' category updates, so a rupee popping out within Link's reach is taken by its
        // first update, in the same frame: GET_RUPY too. (The other kinds' drops are taken at
        // once, by EnItem00_Init: no sound.)
        let dropped = |pred: &dyn Fn(i16, bool) -> bool| {
            o.wonder.iter().filter(|&&(h, killed, mode, all_tagged)| killed && pred(mode, all_tagged) && same_scene && p.wonder.iter().any(|q| q.0 == h && !q.1)).count()
        };
        let switch_drops = dropped(&|m, _| m == WONDERITEM_INTERACT_SWITCH);
        let new_taken = o
            .items
            .iter()
            .filter(|&&(h, params, action, _)| {
                (params <= en_item00::ITEM00_RUPEE_RED || params == en_item00::ITEM00_RUPEE_ORANGE) && action == en_item00::Action::Collected && same_scene && !p.items.iter().any(|q| q.0 == h)
            })
            .count();
        let want_rupy = collected(true) + new_taken.min(switch_drops);
        got_rupees += want_rupy;
        assert_eq!(of(&r, o.frame, NA_SE_SY_GET_RUPY), vec![SfxPos::Default; want_rupy], "frame {}", o.frame);
        // The kinds that drop (EnWonderItem_DropCollectible's callers): the proximity drop, the
        // switch, the roll drop, and the multitags once every tag is touched (out of order, or
        // out of time, the ordered one just goes); the tag points and the proximity switch go
        // without.
        let drops = dropped(&|m, all_tagged| matches!(m, 2 | 3 | 9) || (matches!(m, 0 | 5) && all_tagged));
        assert_eq!(of(&r, o.frame, NA_SE_SY_GET_ITEM), vec![SfxPos::Default; drops + collected(false)], "frame {}", o.frame);
        let counted = o.rupees != p.rupees;
        assert_eq!(of(&r, o.frame, NA_SE_SY_RUPY_COUNT), if counted { vec![SfxPos::Default] } else { vec![] }, "frame {}: {} rupees to {}", o.frame, p.rupees, o.rupees);
    }
    assert!(got_rupees >= 5, "rupees taken: {got_rupees}");

    // Navi.
    let mut vanished = 0;
    for (i, o) in obs.iter().enumerate().skip(1) {
        let p = &obs[i - 1];
        let want = match (p.navi, o.navi) {
            (Some((h0, before, _)), Some((h, after, silenced))) if h0 == h && before == 0 && after == 7 && silenced == 0 => vec![SfxPos::Actor(h)],
            _ => vec![],
        };
        vanished += want.len();
        assert_eq!(of(&r, o.frame, NA_SE_EV_NAVY_VANISH), want, "frame {}", o.frame);
    }
    assert!(vanished >= 1, "Navi goes into Link's hat on the way");

    let total: usize = r.values().map(|v| v.len()).sum();
    eprintln!("{} frames, {total} requests: chest opening at {f0}, {got_rupees} rupees taken, Navi into the hat {vanished} times", obs.len());
}

#[test]
fn the_deku_tree_run_cuts_a_bush_as_the_c() {
    let Some((a, data)) = assets() else { return };
    let route = Route::DekuTree;
    let Some(mut w) = start(&a, &data, route) else { return };
    let mut run_obs = Vec::new();
    // The sound sources as each frame left them.
    let mut sources = Vec::new();
    {
        let mut run = Playthrough::for_route(route);
        let mut prev = PadState::default();
        run_obs.push(observe(&w));
        sources.push(w.sfx_sources);
        loop {
            let pad = run.next(&w);
            run.take_done();
            let Some(pad) = pad else { break };
            w.tick_with(scripted_input(prev, pad));
            prev = pad;
            run_obs.push(observe(&w));
            sources.push(w.sfx_sources);
        }
        assert_eq!(run.failure, None, "the run stopped at {}", run.at());
    }
    let r = requests(&w);
    let mut cuts = 0;
    for (i, o) in run_obs.iter().enumerate().skip(1) {
        let p = &run_obs[i - 1];
        if p.scene_changes != o.scene_changes {
            continue;
        }
        let cut: Vec<glam::Vec3> = o
            .bushes
            .iter()
            .filter(|&&(h, action, killed, _)| p.bushes.iter().any(|q| q.0 == h && q.1 == en_kusa::Action::Main && !q.2) && (killed || action != en_kusa::Action::Main))
            .map(|b| b.3)
            .collect();
        let got = of(&r, o.frame, NA_SE_EV_PLANT_BROKEN);
        assert_eq!(got.len(), cut.len(), "frame {}", o.frame);
        for (pos, s) in cut.iter().zip(&got) {
            // At a sound source at the bush, for 20 frames (SfxSource_UpdateAll has counted
            // this frame's off).
            let SfxPos::Source(k) = *s else { panic!("frame {}: {s:?}", o.frame) };
            let src = sources[i][k as usize];
            assert_eq!((src.world_pos, src.countdown), (*pos, 19), "frame {}", o.frame);
        }
        cuts += cut.len();
    }
    assert!(cuts >= 1, "the run cuts a bush");
    eprintln!("{cuts} bushes cut");
}

/// Play entering by `entrance` (child Link, 10:00) with the audio offline and logged, settled
/// (30 frames), Link placed at `pos` facing `yaw`, one frame for the door to see him.
fn at_door(a: &Arc<GameAssets>, data: &eng_audio::AudioData, entrance: &str, pos: glam::Vec3, yaw: i16) -> Option<(PlayState, PadState)> {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let save = oot_game::save::SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    let audio = GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), common::data()?, common::rules()?, save, audio).expect("Play_Init");
    w.audio_side = Some(Box::new(OfflineAudio::new(data, false)));
    for _ in 0..30 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    w.place_player(pos, yaw);
    w.tick_with(scripted_input(PadState::default(), PadState::default()));
    Some((w, PadState::default()))
}

/// A on the door, then idle until `frames` have passed or the scene changes: the frame the door
/// started opening (`EnDoor_Open`), its handle and `openAnim`.
fn open_door(w: &mut PlayState, mut prev: PadState, frames: usize) -> (u32, ActorHandle, u8) {
    let a_press = PadState { button: eng_input::pad::BTN_A, ..Default::default() };
    let mut opened = None;
    let changes = w.scene_changes;
    for i in 0..frames {
        let pad = if i == 0 { a_press } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        if w.scene_changes != changes {
            break;
        }
        if opened.is_none()
            && let Some((h, open_anim)) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnDoor>(h).filter(|d| d.action == en_door::Action::Open).map(|d| (h, d.open_anim)))
        {
            opened = Some((w.audio.frames, h, open_anim));
        }
    }
    opened.expect("the door opens")
}

#[test]
fn doors_sound_as_the_c() {
    let Some((a, data)) = assets() else { return };
    // A Kakariko house's scene exit (DOOR_SCENEEXIT, behind it: DOOR_OPEN_ANIM_CHILD_L): the
    // door opens and the exit to ENTR_SPOT01_6 runs; the door's sound on its frame 25, then
    // the new scene's Player_Init plays the entrance's at Player.
    let Some((mut w, prev)) = at_door(&a, &data, "ENTR_KAKARIKO_0", glam::Vec3::new(100.0, 0.0, 180.0), 0) else { return };
    let changes = w.scene_changes;
    let (f0, door, open_anim) = open_door(&mut w, prev, 200);
    assert_eq!(open_anim, en_door::DOOR_OPEN_ANIM_CHILD_L);
    let r = requests(&w);
    let open_at = f0 + updates_to(25.0, 1.5);
    assert_eq!(open_at, f0 + 17);
    assert_eq!(of(&r, open_at, NA_SE_OC_DOOR_OPEN), [SfxPos::Actor(door)]);
    assert_ne!(w.scene_changes, changes, "the exit ran");
    let player = w.player.unwrap();
    let entered: Vec<(u32, SfxPos)> = r.iter().filter(|(f, _)| **f > open_at).flat_map(|(f, v)| v.iter().filter(|(id, _)| *id == NA_SE_OC_DOOR_OPEN).map(move |(_, p)| (*f, *p))).collect();
    assert_eq!(entered.len(), 1, "{entered:?}");
    assert_eq!(entered[0].1, SfxPos::Actor(player), "Player_Init's entranceSound, at the new scene's Player");
    assert_eq!(w.save.entrance_sound, 0);

    // The souko's room door (DOOR_ROOMLOAD, in front: DOOR_OPEN_ANIM_CHILD_R): the door's open
    // and close sounds on its frames 25 and 70, nothing on entering.
    let Some((mut w, prev)) = at_door(&a, &data, "ENTR_SOUKO_2", glam::Vec3::new(1190.0, 140.0, 150.0), 0x4000) else { return };
    let (f0, door, open_anim) = open_door(&mut w, prev, 120);
    assert_eq!(open_anim, en_door::DOOR_OPEN_ANIM_CHILD_R);
    let r = requests(&w);
    let close_at = f0 + updates_to(70.0, 1.5);
    assert_eq!(close_at, f0 + 47);
    let sounds: Vec<(u32, u16, SfxPos)> = r.iter().flat_map(|(f, v)| v.iter().filter(|(id, _)| *id == NA_SE_OC_DOOR_OPEN || *id == NA_SE_EV_DOOR_CLOSE).map(move |(id, p)| (*f, *id, *p))).collect();
    assert_eq!(sounds, [(f0 + 17, NA_SE_OC_DOOR_OPEN, SfxPos::Actor(door)), (close_at, NA_SE_EV_DOOR_CLOSE, SfxPos::Actor(door))]);
}
