//! GAME-02's exit test (ARCHITECTURE-PLAN.md §5, Phase 3): a headless scripted playthrough of
//! Kokiri Forest, from Link's bed into the Deku Tree, with checks at each step
//! (`oot_actors::playthrough`). It runs on a save with the `deku-tree-open` preset
//! (`EVENTCHKINF_0C` and `EVENTCHKINF_05`): the talk that opens the mouth is a cutscene.
//!
//! Expected values come from the scene data and the C:
//! - `ENTR_LINK_HOME_0` is Link's house's spawn 0, (1, 0, 95), params 0x0D00 (standing); the
//!   house is `SCENE_CAM_TYPE_FIXED_TOGGLE_VIEWPOINT`, so `Play_Init` starts on
//!   `VIEWPOINT_PIVOT`, whose bg camera 1 is `CAM_SET_PREREND_PIVOT`;
//! - the house's exit 2 leads to `ENTR_SPOT04_3`, Kokiri Forest's spawn 3 at (-31, 100, 1073);
//! - the sign by the house (params 0x031F) has text `params | 0x300`;
//! - `BgTreemouth_Update` puts the open mouth (`unk_168` 1) at (4029 - 160, 136 - 399,
//!   -1255 + 92);
//! - Kokiri Forest's exit 2 is `ENTR_YDAN_0` (0x0000), the Deku Tree's scene (`SCENE_YDAN`, 0).

mod common;

use std::sync::Arc;

use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_treemouth::{self, BgTreemouth};
use oot_actors::en_item00::ITEM00_RUPEE_GREEN;
use oot_actors::en_ko::EnKo;
use oot_actors::player::Action;
use oot_actors::playthrough::{Playthrough, Step};
use oot_game::actor_ctx::ActorHandle;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::{EVENTCHKINF_0C, EVENTCHKINF_05, EVENTCHKINF_07, SaveContext};
use oot_game::spawn::Placeholder;
use oot_game::surface::SurfaceType;

const SCENE_YDAN: u16 = 0x00;
const SCENE_LINK_HOME: u16 = 0x34;
const SCENE_SPOT04: u16 = 0x55;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Play entering by `entrance` on a new save with `preset`, child Link, 10:00.
fn enter(a: &Arc<GameAssets>, entrance: &str, preset: Option<&str>) -> Option<PlayState> {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    if let Some(p) = preset {
        save.apply_preset(p).expect("the preset");
    }
    Some(oot_actors::play_entrance(a.clone(), common::data()?, common::rules()?, save).expect("Play_Init"))
}

fn placeholder_of(w: &PlayState, id: i16) -> Option<ActorHandle> {
    w.actors.all().into_iter().find(|&h| w.actors.get(h).is_some_and(|a| a.base().id == id && a.as_any().is::<Placeholder>()))
}

fn treemouth(w: &PlayState) -> Option<&BgTreemouth> {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<BgTreemouth>(h))
}

#[test]
fn the_presets_set_the_flags_the_c_reads() {
    let mut s = SaveContext::default();
    s.apply_preset("deku-tree-open").unwrap();
    // z64save.h: EVENTCHKINF_05 0x05, EVENTCHKINF_0C 0x0C: eventChkInf[0] bits 5 and 12.
    assert_eq!(s.event_chk_inf[0], (1 << 0x05) | (1 << 0x0C));
    assert!(!s.get_event_chk_inf(EVENTCHKINF_07));
    let mut s = SaveContext::default();
    s.apply_preset("deku-tree-dead").unwrap();
    // Door_Warp1's EVENTCHKINF_07 and _09 too, and Item_Give(ITEM_KOKIRI_EMERALD).
    assert_eq!(s.event_chk_inf[0], (1 << 0x05) | (1 << 0x07) | (1 << 0x09) | (1 << 0x0C));
    assert!(s.check_quest_item(oot_game::save::QUEST_KOKIRI_EMERALD));
    assert!(SaveContext::default().apply_preset("nothing").is_err());
}

#[test]
fn the_mouth_is_closed_on_a_new_save_and_open_with_eventchkinf_05() {
    let Some(a) = assets() else { return };
    // Kokiri Forest's spawn 1, in front of the tree (the way out of the Deku Tree).
    let at_tree = |preset: Option<&str>, frames: usize| {
        let mut w = enter(&a, "ENTR_SPOT04_1", preset)?;
        for _ in 0..frames {
            w.tick_with(scripted_input(PadState::default(), PadState::default()));
        }
        Some(w)
    };
    let Some(w) = at_tree(None, 10) else { return };
    let m = treemouth(&w).expect("Bg_Treemouth in room 1");
    // BgTreemouth_Init on scene layer 0, child: func_808BC8B8, targetMode 5, text 0x905,
    // DynaPoly with gDekuTreeMouthCol.
    assert_eq!((m.actor.target_mode, m.actor.text_id, m.actor.scale), (5, 0x905, Vec3::ONE));
    assert!(w.col.dyna.is_bg_actor(m.bg));
    // Link spawns within 1658 of it, facing within 0x4E20: EVENTCHKINF_0C and the first
    // cutscene (not ported), then func_808BC9EC waits for it. Closed: unk_168 0.
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_0C));
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_05));
    assert_eq!((m.action, m.unk_168), (bg_treemouth::Action::WaitCsStart, 0.0));
    assert_eq!(m.actor.world_pos, Vec3::new(4029.0, 136.0, -1255.0));
    // BgTreemouth_Draw: alpha 500 → env alpha 50.
    assert_eq!(BgTreemouth::draw_alpha(&w), 500);

    // With EVENTCHKINF_05: held open, at unk_168 1.
    let Some(w) = at_tree(Some("deku-tree-open"), 10) else {
        return;
    };
    let m = treemouth(&w).unwrap();
    assert_eq!((m.action, m.unk_168), (bg_treemouth::Action::Wait, 1.0));
    assert_eq!(m.actor.world_pos, Vec3::new(4029.0 - 160.0, 136.0 - 399.0, -1255.0 + 92.0));
    // Its collision moved with it: the bg actor's transform is the actor's.
    assert_eq!(w.col.dyna.actors[m.bg as usize].cur.pos, m.actor.world_pos);
    assert_eq!(BgTreemouth::draw_alpha(&w), 500);

    // The tree dead (EVENTCHKINF_07): 2150, as Scene_DrawConfigSpot04 has it for the tree.
    let Some(w) = at_tree(Some("deku-tree-dead"), 10) else {
        return;
    };
    assert_eq!(BgTreemouth::draw_alpha(&w), 2150);
    assert!(w.scene.as_ref().unwrap().draw.event_chk_inf_07);
}

#[test]
fn func_808bc9ec_sets_eventchkinf_05_when_the_cutscene_starts_on_yes() {
    use oot_game::cutscene::{CS_STATE_SKIPPABLE_EXEC, CS_STATE_UNSKIPPABLE_INIT, CsCmdActorAction};
    let Some(a) = assets() else { return };
    let Some(mut w) = enter(&a, "ENTR_SPOT04_1", None) else {
        return;
    };
    let tick = |w: &mut PlayState| w.tick_with(scripted_input(PadState::default(), PadState::default()));
    for _ in 0..10 {
        tick(&mut w);
    }
    assert_eq!(treemouth(&w).unwrap().action, bg_treemouth::Action::WaitCsStart);
    // Nothing runs cutscenes: it waits.
    for _ in 0..10 {
        tick(&mut w);
    }
    assert_eq!(treemouth(&w).unwrap().action, bg_treemouth::Action::WaitCsStart);
    // By hand, as z_demo.c would: the cutscene starting, with the first choice answered.
    w.cs_ctx.state = CS_STATE_UNSKIPPABLE_INIT;
    w.msg_ctx.choice_index = 0;
    tick(&mut w);
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_05));
    assert_eq!((w.cs_ctx.state, w.cs_ctx.frames, w.cs_ctx.unk_18), (CS_STATE_SKIPPABLE_EXEC, 0, 0xFFFF));
    assert_eq!(treemouth(&w).unwrap().action, bg_treemouth::Action::WaitCsCue);
    // The script's cue 3 for the mouth: func_808BC6F8 opens it by 0.01 a frame.
    w.cs_ctx.npc_actions[0] = Some(CsCmdActorAction { action: 3, ..Default::default() });
    tick(&mut w);
    assert_eq!(treemouth(&w).unwrap().action, bg_treemouth::Action::Open);
    for _ in 0..10 {
        tick(&mut w);
    }
    let m = treemouth(&w).unwrap();
    assert!((m.unk_168 - 0.10).abs() < 1e-5, "{}", m.unk_168);
}

#[test]
fn kokiri_forest_to_the_deku_tree() {
    let Some(a) = assets() else { return };
    let Some(mut w) = enter(&a, Playthrough::ENTRANCE, Some(Playthrough::PRESET)) else {
        return;
    };
    let entr = |name: &str| a.scenes.entrance_index(name).unwrap();
    let mut run = Playthrough::new();
    let mut prev = PadState::default();
    let mut talked_to = Vec::new();
    // How near Link came to Mido's and Saria's placeholders.
    let (mut near_mido, mut near_saria) = (f32::MAX, f32::MAX);
    let md = a.actors.id("ACTOR_EN_MD").unwrap();
    let sa = a.actors.id("ACTOR_EN_SA").unwrap();
    let mut seen = Vec::new();
    loop {
        // A step is done on the state before the next frame's input.
        let pad = run.next(&w);
        if let Some(step) = run.take_done() {
            seen.push(step);
            check(step, &w, &run, &talked_to, &entr);
        }
        let Some(pad) = pad else { break };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        if let Some(t) = w.msg_ctx.talk_actor
            && talked_to.last() != Some(&t)
        {
            talked_to.push(t);
        }
        let link = w.player().actor.world_pos;
        if w.scene_id == SCENE_SPOT04 {
            for (id, near) in [(md, &mut near_mido), (sa, &mut near_saria)] {
                if let Some(h) = placeholder_of(&w, id) {
                    let p = w.actors.actor(h).unwrap().world_pos;
                    *near = near.min(Vec3::new(link.x - p.x, 0.0, link.z - p.z).length());
                }
            }
        }
    }
    assert_eq!(run.failure, None, "the run stopped at {}", run.at());
    assert_eq!(seen, [Step::House, Step::OutDoor, Step::Ladder, Step::Sign, Step::Kokiri, Step::Bush, Step::Tree, Step::Mouth, Step::DekuTree]);
    // Mido and Saria are placeholders with no colliders: Link walked right past them.
    assert!(near_mido < 60.0, "Mido: {near_mido}");
    assert!(near_saria < 60.0, "Saria: {near_saria}");
}

/// The checks when `step` is done.
fn check(step: Step, w: &PlayState, run: &Playthrough, talked_to: &[ActorHandle], entr: &dyn Fn(&str) -> u16) {
    let p = w.player();
    match step {
        Step::House => {
            assert_eq!((w.scene_id, w.save.entrance_index, w.room_ctx.cur.num), (SCENE_LINK_HOME, entr("ENTR_LINK_HOME_0"), 0));
            assert!(p.actor.world_pos.distance(Vec3::new(1.0, 0.0, 95.0)) < 1.0, "at spawn 0: {}", p.actor.world_pos);
            assert_eq!((w.viewpoint, w.game_camera.setting), (oot_game::play::VIEWPOINT_PIVOT, oot_game::camera::CAM_SET_PREREND_PIVOT));
            // The preset's flags, and a new save's hearts and rupees.
            assert!(w.save.get_event_chk_inf(EVENTCHKINF_0C) && w.save.get_event_chk_inf(EVENTCHKINF_05));
            assert_eq!((w.save.health, w.save.rupees), (0x30, 0));
        }
        Step::OutDoor => {
            assert_eq!((w.scene_id, w.save.entrance_index, w.room_ctx.cur.num), (SCENE_SPOT04, entr("ENTR_SPOT04_3"), 0));
            assert!(p.actor.world_pos.distance(Vec3::new(-31.0, 100.0, 1073.0)) < 1.0, "at spawn 3: {}", p.actor.world_pos);
        }
        Step::Ladder => {
            assert!((p.actor.world_pos.y + 80.0).abs() < 1.0, "on the ground: {}", p.actor.world_pos);
            assert!((p.actor.world_pos.x + 29.0).abs() < 8.0, "at the ladder's foot: {}", p.actor.world_pos);
        }
        Step::Sign => {
            assert_eq!(run.texts, [0x031F], "the sign's text");
            assert_eq!(p.action, Action::StandingStill);
            let sign = talked_to.last().copied().unwrap();
            assert_eq!(w.actors.actor(sign).unwrap().home_pos.x, 49.0);
        }
        Step::Kokiri => {
            let child = talked_to.last().copied().unwrap();
            let k = w.actors.downcast::<EnKo>(child).expect("an En_Ko");
            assert_eq!(k.actor.params & 0xFF, 4);
            // Child 4's texts (func_80A97610) after the sign's.
            assert!(run.texts.len() > 1 && run.texts[1..].iter().all(|t| t & 0xFF00 == 0x1000), "{:x?}", run.texts);
        }
        Step::Bush => {
            // The bushes Link cut are gone (EnKusa_Main kills a cut ENKUSA_TYPE_0 bush), and
            // the drop was a green rupee: Item_Give(ITEM_RUPEE_GREEN) adds 1.
            assert!(!run.bushes_cut.is_empty());
            for home in &run.bushes_cut {
                assert!(!w.actors.all().into_iter().any(|h| w.actors.downcast::<oot_actors::en_kusa::EnKusa>(h).is_some_and(|k| k.actor.home_pos == *home)), "the bush at {home} is gone");
            }
            assert_eq!(run.drop, Some(ITEM00_RUPEE_GREEN));
            assert_eq!(w.save.rupees + w.save.rupee_accumulator, 1);
        }
        Step::Tree => {
            assert_eq!(w.room_ctx.cur.num, 1, "the En_Holl took Link into room 1");
            let m = treemouth(w).expect("Bg_Treemouth");
            assert_eq!((m.action, m.unk_168), (bg_treemouth::Action::Wait, 1.0));
            // Standing on the open jaw: its DynaPoly floor.
            assert!(p.grounded());
            assert_eq!(p.actor.floor_bg_id, m.bg, "on the mouth's collision");
        }
        Step::Mouth => {
            // The floor's exit 2: ENTR_YDAN_0.
            let floor = p.actor.floor_poly.expect("a floor");
            assert_eq!(w.col.exit_index(floor), 2);
            assert_eq!(w.transition.next_entrance_index, entr("ENTR_YDAN_0"));
        }
        Step::DekuTree => {
            assert_eq!((w.scene_id, w.save.entrance_index), (SCENE_YDAN, entr("ENTR_YDAN_0")));
            assert_eq!(p.action, Action::StandingStill);
        }
    }
}
