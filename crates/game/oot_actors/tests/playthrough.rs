//! The headless scripted playthroughs of Kokiri Forest (`oot_actors::playthrough`), with checks
//! at each step:
//! - GAME-02's exit test (ARCHITECTURE-PLAN.md §5, Phase 3): from Link's bed into the Deku Tree,
//!   on a save with the `deku-tree-open` preset (`EVENTCHKINF_0C` and `EVENTCHKINF_05`): the
//!   talk that opens the mouth is a cutscene;
//! - GAME-03 milestone 2's exit test: from Link's bed on a new save, through the crawlspace and
//!   past the boulder, to the Kokiri Sword's chest, opened;
//! - GAME-03 milestone 3's: from Link's bed on a new save to the sword, 40 rupees, the Deku
//!   Shield bought in the Kokiri shop, both worn, and past Mido once he has stepped aside.
//!
//! Expected values come from the scene data and the C:
//! - `ENTR_LINK_HOME_0` is Link's house's spawn 0, (1, 0, 95), params 0x0D00 (standing); the
//!   house is `SCENE_CAM_TYPE_FIXED_TOGGLE_VIEWPOINT`, so `Play_Init` starts on
//!   `VIEWPOINT_PIVOT`, whose bg camera 1 is `CAM_SET_PREREND_PIVOT`;
//! - the house's exit 2 leads to `ENTR_SPOT04_3`, Kokiri Forest's spawn 3 at (-31, 100, 1073);
//! - the sign by the house (params 0x031F) has text `params | 0x300`;
//! - `BgTreemouth_Update` puts the open mouth (`unk_168` 1) at (4029 - 160, 136 - 399,
//!   -1255 + 92);
//! - Kokiri Forest's exit 2 is `ENTR_YDAN_0` (0x0000), the Deku Tree's scene (`SCENE_YDAN`, 0);
//! - the crawlspace's mouth is the wall at z 1059 (x -801..-769, `WALL_FLAG_4`) whose triangles'
//!   middle is x -785, facing -z (`wallYaw` 0x8000); its floor names bg camera 9
//!   (`CAM_SET_CRAWLSPACE`); `En_Holl` 0 joins rooms 0 and 2 across it; room 2's floor beyond it
//!   names bg camera 14 (`CAM_SET_DUNGEON0`);
//! - room 2's `En_Goroiwa` (params 0x0C02) rolls path 2; its two `En_Wonder_Item`s (0x123F) each
//!   drop a green rupee that collects itself; its `En_Box` (0x04E0) holds `GI_SWORD_KOKIRI`,
//!   treasure flag 0.

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
    // z64save.h: EVENTCHKINF_04 0x04 (Mido stepped aside), EVENTCHKINF_05 0x05, EVENTCHKINF_0C
    // 0x0C: eventChkInf[0] bits 4, 5 and 12.
    assert_eq!(s.event_chk_inf[0], (1 << 0x04) | (1 << 0x05) | (1 << 0x0C));
    assert!(!s.get_event_chk_inf(EVENTCHKINF_07));
    let mut s = SaveContext::default();
    s.apply_preset("deku-tree-dead").unwrap();
    // Door_Warp1's EVENTCHKINF_07 and _09 too, and Item_Give(ITEM_KOKIRI_EMERALD).
    assert_eq!(s.event_chk_inf[0], (1 << 0x04) | (1 << 0x05) | (1 << 0x07) | (1 << 0x09) | (1 << 0x0C));
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
fn kokiri_forest_to_the_deku_tree() {
    let Some(a) = assets() else { return };
    let Some(mut w) = enter(&a, Playthrough::ENTRANCE, Some(Playthrough::PRESET)) else {
        return;
    };
    let entr = |name: &str| a.scenes.entrance_index(name).unwrap();
    let mut run = Playthrough::new();
    let mut prev = PadState::default();
    let mut talked_to = Vec::new();
    // How near Link came to Mido (En_Md) and to Saria's placeholder.
    let (mut near_mido, mut near_saria) = (f32::MAX, f32::MAX);
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
            if let Some(h) = placeholder_of(&w, sa) {
                let p = w.actors.actor(h).unwrap().world_pos;
                near_saria = near_saria.min(Vec3::new(link.x - p.x, 0.0, link.z - p.z).length());
            }
            if let Some(m) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<oot_actors::en_md::EnMd>(h)) {
                // EVENTCHKINF_04 (the preset): EnMd_SetMovedPos, standing at path 1's last point
                // (func_80AAB874).
                assert_eq!((m.action, m.actor.world_pos), (oot_actors::en_md::Action::Moved, Vec3::new(1412.0, 0.0, 211.0)));
                let p = m.actor.world_pos;
                near_mido = near_mido.min(Vec3::new(link.x - p.x, 0.0, link.z - p.z).length());
            }
        }
    }
    assert_eq!(run.failure, None, "the run stopped at {}", run.at());
    assert_eq!(seen, [Step::House, Step::OutDoor, Step::Ladder, Step::Sign, Step::Kokiri, Step::Bush, Step::Tree, Step::Mouth, Step::DekuTree]);
    // Link passed Mido where he stands aside (his collider, 36, and Link's kept them apart), and
    // Saria's placeholder, which has no collider.
    assert!(near_mido < 80.0, "Mido: {near_mido}");
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
            // the drop was green rupees: Item_Give(ITEM_RUPEE_GREEN) adds 1 for each of the
            // drop's quantity (Item_DropCollectibleRandom's sDropQuantities: 1 to 3), all
            // collected.
            assert!(!run.bushes_cut.is_empty());
            for home in &run.bushes_cut {
                assert!(!w.actors.all().into_iter().any(|h| w.actors.downcast::<oot_actors::en_kusa::EnKusa>(h).is_some_and(|k| k.actor.home_pos == *home)), "the bush at {home} is gone");
            }
            assert_eq!(run.drop, Some(ITEM00_RUPEE_GREEN));
            assert!((1..=3).contains(&(w.save.rupees + w.save.rupee_accumulator)), "rupees {}", w.save.rupees + w.save.rupee_accumulator);
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
        _ => panic!("{step:?} isn't on the Deku Tree's route"),
    }
}

#[test]
fn a_new_save_to_the_kokiri_sword() {
    use oot_actors::en_goroiwa::EnGoroiwa;
    use oot_actors::playthrough::Route;
    use oot_game::camera::CAM_SET_CRAWLSPACE;
    use oot_game::interface::DO_ACTION_ENTER;
    let Some(a) = assets() else { return };
    let route = Route::SwordChest;
    let Some(mut w) = enter(&a, route.entrance(), route.preset()) else {
        return;
    };
    let entr = |name: &str| a.scenes.entrance_index(name).unwrap();
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut seen = Vec::new();
    // What the frames between the steps showed: "Enter" on A before the crawl, the crawlspace's
    // camera while crawling, and the nearest the boulder came to Link.
    let (mut saw_enter, mut saw_crawl_camera, mut nearest_boulder) = (false, false, f32::MAX);
    loop {
        let pad = run.next(&w);
        if let Some(step) = run.take_done() {
            seen.push(step);
            check_sword(step, &w, &run, saw_enter, saw_crawl_camera, &entr);
        }
        let Some(pad) = pad else { break };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        let p = w.player();
        saw_enter |= w.interface_ctx.unk_1f0 == DO_ACTION_ENTER;
        saw_crawl_camera |= p.action == Action::Crawl && w.game_camera.setting == CAM_SET_CRAWLSPACE;
        if let Some(b) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnGoroiwa>(h)) {
            nearest_boulder = nearest_boulder.min(b.actor.world_pos.distance(p.actor.world_pos));
        }
    }
    assert_eq!(run.failure, None, "the run stopped at {}", run.at());
    assert_eq!(seen, [Step::House, Step::OutDoor, Step::Ladder, Step::Crawlspace, Step::TrainingArea, Step::Boulder, Step::Chest]);
    // The boulder's sphere (58) never reached Link.
    assert!(nearest_boulder > 58.0 + 12.0, "the boulder came within {nearest_boulder}");
}

/// The checks when a step of the Kokiri Sword run is done.
fn check_sword(step: Step, w: &PlayState, run: &Playthrough, saw_enter: bool, saw_crawl_camera: bool, entr: &dyn Fn(&str) -> u16) {
    use oot_actors::player::STATE2_18;
    use oot_game::camera::CAM_SET_DUNGEON0;
    let p = w.player();
    match step {
        Step::House => {
            assert_eq!((w.scene_id, w.save.entrance_index, w.room_ctx.cur.num), (SCENE_LINK_HOME, entr("ENTR_LINK_HOME_0"), 0));
            // Sram_InitNewSave: three hearts, no rupees, the Kokiri tunic and boots only, no flags.
            assert_eq!((w.save.health, w.save.rupees, w.save.inventory.equipment), (0x30, 0, 0x1100));
            assert!(!w.save.get_event_chk_inf(EVENTCHKINF_05));
        }
        Step::OutDoor => {
            assert_eq!((w.scene_id, w.save.entrance_index), (SCENE_SPOT04, entr("ENTR_SPOT04_3")));
        }
        Step::Ladder => {
            assert!((p.actor.world_pos.y + 80.0).abs() < 1.0, "on the ground: {}", p.actor.world_pos);
        }
        Step::Crawlspace => {
            // func_8083F0C8: A said "Enter" (PLAYER_STATE2_16) at the mouth; Link lined up with
            // its triangles' middle (x -785), facing wallYaw + 0x8000 (+z), crawling
            // (PLAYER_STATE2_18, func_8084C760 after tunnel_start).
            assert!(saw_enter, "DO_ACTION_ENTER before the crawl");
            assert_eq!(p.action, Action::Crawl);
            assert!(p.state2 & STATE2_18 != 0);
            assert!((p.actor.world_pos.x + 785.0).abs() < 0.01, "lined up with the mouth: {}", p.actor.world_pos);
            assert_eq!(p.actor.shape_rot.y, 0);
            assert_eq!(w.data.anim_name(p.skel.animation), "link_child_tunnel_start");
            assert_eq!(w.room_ctx.cur.num, 0);
        }
        Step::TrainingArea => {
            // Camera_Subj4 on the tunnel's floor (bg camera 9, CAM_SET_CRAWLSPACE) kept Link on
            // the crawlspace's line while he crawled; En_Holl 0 loaded room 2; func_8083F570 at
            // the far wall (z 1359), then func_8084C81C's tunnel_end and standing
            // (func_8083C0E8), the crawl over. Room 2's floor names bg camera 14.
            assert!(saw_crawl_camera, "the crawlspace's camera while crawling");
            assert_eq!(w.room_ctx.cur.num, 2);
            assert_eq!(p.action, Action::StandingStill);
            assert_eq!(p.state2 & STATE2_18, 0);
            assert!(p.actor.world_pos.z > 1379.0 && (p.actor.world_pos.x + 785.0).abs() < 5.0, "out of the tunnel: {}", p.actor.world_pos);
            assert_eq!((w.game_camera.setting, w.game_camera.bg_cam_index), (CAM_SET_DUNGEON0, 14));
        }
        Step::Boulder => {
            // Past the corridors unhurt, with both wonder items' green rupees
            // (EnWonderItem_ProximityDrop: ITEM00_RUPEE_GREEN | 0x8000, collected at once).
            assert_eq!(w.save.health, 0x30, "never hit");
            assert!(p.actor.world_pos.z > 1930.0, "north of the corridors: {}", p.actor.world_pos);
            assert_eq!(w.save.rupees + w.save.rupee_accumulator, 2);
            assert_eq!(run.boulder_waits.len(), 2);
        }
        Step::Chest => {
            // func_8084DFF4: text 0xA4 and Item_Give(ITEM_SWORD_KOKIRI), which only owns it
            // (equipment bit 0): B stays empty until the pause menu (its stand-in). The chest's
            // treasure flag 0.
            assert_eq!(run.texts, [0xA4]);
            assert_eq!(w.save.inventory.equipment, 0x1101);
            assert_eq!(w.save.equips.button_items[0], oot_game::save::ITEM_NONE);
            assert!(w.flags.get_treasure(0));
            assert_eq!(p.action, Action::StandingStill);
        }
        _ => panic!("{step:?} isn't on the Kokiri Sword's route"),
    }
}

#[test]
fn a_new_save_to_mido_and_the_shop() {
    use oot_actors::en_md::{self, EnMd};
    use oot_actors::playthrough::Route;
    let Some(a) = assets() else { return };
    let route = Route::MidoShop;
    let Some(mut w) = enter(&a, route.entrance(), route.preset()) else {
        return;
    };
    let entr = |name: &str| a.scenes.entrance_index(name).unwrap();
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut seen = Vec::new();
    // Mido's positions while he blocks (his home is room 0's placement), and whether Link was
    // hidden while he browsed.
    let mut mido_block = Vec::new();
    let mut hidden_while_browsing = false;
    loop {
        let pad = run.next(&w);
        if let Some(step) = run.take_done() {
            seen.push(step);
            check_mido_shop(step, &w, &run, hidden_while_browsing, &entr);
        }
        let Some(pad) = pad else { break };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        // (On its init frame Mido hasn't updated yet: still at home.)
        if let Some(m) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnMd>(h))
            && m.action == en_md::Action::Blocking
            && m.unk_1e0.talk_state == 0
            && m.actor.world_pos != m.actor.home_pos
        {
            mido_block.push((m.actor.home_pos, m.actor.world_pos));
        }
        if let Some(o) = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<oot_actors::en_ossan::EnOssan>(h))
            && o.state_flag == oot_actors::en_ossan::OSSAN_STATE_BROWSE_RIGHT_SHELF
        {
            hidden_while_browsing |= w.player().state2 & oot_actors::player::STATE2_29 != 0;
        }
    }
    assert_eq!(run.failure, None, "the run stopped at {}", run.at());
    assert_eq!(
        seen,
        [
            Step::House,
            Step::OutDoor,
            Step::Ladder,
            Step::Crawlspace,
            Step::TrainingArea,
            Step::Boulder,
            Step::Chest,
            Step::SwordOn,
            Step::Plateau,
            Step::Switch,
            Step::MidoHouse,
            Step::MidoChests,
            Step::Shop,
            Step::Shield,
            Step::Equipped,
            Step::ShopOut,
            Step::Mido,
            Step::MidoAside,
            Step::PastMido,
        ]
    );
    // func_80AAB948: while blocking, 60 from his home (1522, 0, 105) towards Link.
    assert!(!mido_block.is_empty());
    for (home, pos) in &mido_block {
        assert_eq!(*home, Vec3::new(1522.0, 0.0, 105.0));
        assert!((Vec3::new(pos.x - home.x, 0.0, pos.z - home.z).length() - 60.0).abs() < 0.01, "{pos}");
    }
}

/// GAME-03 milestone 4's exit test: the Mido and shop run, then on into the meadow, where the
/// Deku Tree's first talk (`D_808BCE20`) starts by itself; yes (`D_808BD520`) opens his mouth
/// (`EVENTCHKINF_05`), and Link walks in to `ENTR_YDAN_0`, whose intro (`gDekuTreeIntroCs`)
/// plays the first time. No preset.
#[test]
fn a_new_save_into_the_deku_tree() {
    use oot_actors::playthrough::Route;
    use oot_game::cutscene::CS_STATE_IDLE;
    let Some(a) = assets() else { return };
    let route = Route::NewSaveDekuTree;
    assert_eq!(route.preset(), None);
    let Some(mut w) = enter(&a, route.entrance(), route.preset()) else {
        return;
    };
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut seen = Vec::new();
    // The scripts played, in order, and the cameras they were seen from.
    let mut scripts: Vec<String> = Vec::new();
    let mut sub_camera = false;
    loop {
        let pad = run.next(&w);
        if let Some(step) = run.take_done() {
            seen.push(step);
            match step {
                Step::TreeTalk => {
                    assert!(w.save.get_event_chk_inf(EVENTCHKINF_0C) && w.save.get_event_chk_inf(EVENTCHKINF_05));
                    assert_eq!((w.cs_ctx.state, w.active_cam_id, w.player().cs_mode), (CS_STATE_IDLE, 0, 0));
                    assert_eq!(treemouth(&w).unwrap().action, bg_treemouth::Action::Open);
                }
                Step::Tree => {
                    // The jaw all the way open by then, held by EVENTCHKINF_05.
                    let m = treemouth(&w).unwrap();
                    assert_eq!(m.unk_168, 1.0);
                    assert_eq!(m.actor.world_pos, Vec3::new(4029.0 - 160.0, 136.0 - 399.0, -1255.0 + 92.0));
                }
                Step::Mouth => assert_eq!(w.transition.next_entrance_index, a.scenes.entrance_index("ENTR_YDAN_0").unwrap()),
                Step::DekuTree => {
                    assert_eq!(w.scene_id, SCENE_YDAN);
                    // Cutscene_HandleEntranceTriggers set EVENTCHKINF_A8; the intro is over.
                    assert!(w.save.get_event_chk_inf(0xA8));
                    assert_eq!((w.cs_ctx.state, w.player().action), (CS_STATE_IDLE, Action::StandingStill));
                }
                _ => {}
            }
        }
        let Some(pad) = pad else { break };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        if let Some(s) = &w.cs_ctx.segment
            && w.cs_ctx.state != CS_STATE_IDLE
            && scripts.last() != Some(&s.name)
        {
            scripts.push(s.name.clone());
        }
        sub_camera |= w.cs_ctx.state != CS_STATE_IDLE && w.active_cam_id != 0;
    }
    assert_eq!(run.failure, None, "the run stopped at {}", run.at());
    let tail = [Step::MidoAside, Step::PastMido, Step::TreeTalk, Step::Tree, Step::Mouth, Step::DekuTree];
    assert_eq!(&seen[seen.len() - tail.len()..], tail);
    assert_eq!(scripts, ["D_808BCE20", "D_808BD520", "gDekuTreeIntroCs"]);
    assert!(sub_camera);
    for t in [0x107D, 0x1015, 0x1016, 0x1017] {
        assert!(run.texts.contains(&t), "text {t:#x}: {:x?}", run.texts);
    }
}

/// The checks when a step of the Mido and shop run is done.
fn check_mido_shop(step: Step, w: &PlayState, run: &Playthrough, hidden_while_browsing: bool, entr: &dyn Fn(&str) -> u16) {
    use oot_actors::en_md::{self, EnMd};
    use oot_game::item::*;
    let p = w.player();
    let rupees = w.save.rupees + w.save.rupee_accumulator;
    let mido = || w.actors.all().into_iter().find_map(|h| w.actors.downcast::<EnMd>(h));
    match step {
        Step::House => assert_eq!((w.save.rupees, w.save.inventory.equipment), (0, 0x1100)),
        Step::OutDoor | Step::Ladder | Step::Crawlspace | Step::TrainingArea => {}
        Step::Boulder => {
            // Room 2's two proximity drops (green rupees), and its two blue ones (En_Item00
            // 0x0F01 and 0x0E01, collectible flags 0x0F and 0x0E): 2 + 5 + 5.
            assert_eq!(rupees, 12);
            assert!(w.flags.get_collectible(0x0F) && w.flags.get_collectible(0x0E));
        }
        Step::Chest => assert_eq!(w.save.inventory.equipment, 0x1101),
        Step::SwordOn => {
            // The stand-in equips it as the pause menu does: Inventory_ChangeEquipment, and B the
            // sword.
            assert_eq!(w.save.cur_equip_value(EQUIP_TYPE_SWORD), EQUIP_VALUE_SWORD_KOKIRI);
            assert_eq!(w.save.equips.button_items[0], ITEM_SWORD_KOKIRI);
        }
        Step::Plateau => {
            assert_eq!((w.scene_id, w.room_ctx.cur.num), (SCENE_SPOT04, 0));
            assert_eq!(p.action, Action::StandingStill);
        }
        Step::Switch => {
            // EnWonderItem_InteractSwitch: switch 0x13 set, and its blue rupee (not collected by
            // itself) picked up.
            assert!(w.flags.get_switch(0x13));
            assert_eq!(rupees, 17);
        }
        Step::MidoHouse => {
            assert_eq!(w.save.entrance_index, entr("ENTR_KOKIRI_HOME4_0"));
            // The two greens at the foot of the ramp.
            assert_eq!(rupees, 19);
        }
        Step::MidoChests => {
            // Two blue rupees, a green one and a recovery heart (En_Box 0x59A0, 0x59A1, 0x5982,
            // 0x5903), their treasure flags 0 to 3 saved with his house's scene flags.
            assert_eq!(w.save.entrance_index, entr("ENTR_SPOT04_9"));
            assert_eq!(rupees, 30);
            const SCENE_KOKIRI_HOME4: u16 = 0x28;
            assert_eq!(w.save.scene_flags(SCENE_KOKIRI_HOME4).chest & 0xF, 0xF);
        }
        Step::Shop => {
            // Two more greens and the free multitag's blue rupee.
            assert_eq!(w.save.entrance_index, entr("ENTR_KOKIRI_SHOP_0"));
            assert_eq!(rupees, 37);
            assert!(run.texts.contains(&0x218), "the forced text by the shop, read");
        }
        Step::Shield => {
            // The shop's own blue rupee (42), then the Deku Shield's 40, charged after its text
            // (EnGirlA_BuyEvent_ShieldDiscount): 2 left. Owned, not worn (Item_Give).
            assert_eq!(rupees, 2);
            assert!(w.save.check_owned_equip(EQUIP_TYPE_SHIELD, EQUIP_INV_SHIELD_DEKU));
            assert_eq!(w.save.cur_equip_value(EQUIP_TYPE_SHIELD), EQUIP_VALUE_SHIELD_NONE);
            let texts: Vec<u16> = run.texts.iter().copied().skip_while(|&t| t != 0x9E).collect();
            assert_eq!(texts, [0x9E, 0x83, 0x9F, 0x89, 0x4C, 0x6B]);
            // EnOssan_EndInteraction: YREG(31) 0, the fixed view, Link shown.
            assert_eq!((w.msg_ctx.yreg_31, w.viewpoint), (0, oot_game::play::VIEWPOINT_LOCKED));
            assert!(hidden_while_browsing, "PLAYER_STATE2_29 while browsing");
            assert_eq!(p.state2 & oot_actors::player::STATE2_29, 0);
        }
        Step::Equipped => {
            assert_eq!(w.save.cur_equip_value(EQUIP_TYPE_SHIELD), EQUIP_VALUE_SHIELD_DEKU);
            assert_eq!(p.current_shield, 1, "PLAYER_SHIELD_DEKU (Player_SetEquipmentData)");
        }
        Step::ShopOut => assert_eq!(w.save.entrance_index, entr("ENTR_SPOT04_4")),
        Step::Mido => {
            // With both worn, 0x1033 (EnMd_GetTextKokiriForest), then its next texts; as it
            // closes, func_80AAAF04's 2: EVENTCHKINF_04, and on his way along path 1.
            let texts: Vec<u16> = run.texts.iter().copied().skip_while(|&t| t != 0x1033).collect();
            assert_eq!(texts, [0x1033, 0x10D2, 0x10D3, 0x1034]);
            assert!(w.save.get_event_chk_inf(oot_game::save::EVENTCHKINF_04));
            let m = mido().expect("Mido");
            assert_eq!((m.action, m.actor.speed_xz, m.waypoint), (en_md::Action::Walking, 1.5, 1));
        }
        Step::MidoAside => {
            // func_80AABD0C: path 1's last point reached (within 10), standing (func_80AAB8F8).
            let m = mido().expect("Mido");
            assert_eq!(m.action, en_md::Action::Arrived);
            assert!(Vec3::new(m.actor.world_pos.x - 1412.0, 0.0, m.actor.world_pos.z - 211.0).length() < 10.0, "{}", m.actor.world_pos);
            assert_eq!(m.actor.speed_xz, 0.0);
        }
        Step::PastMido => {
            assert!(p.actor.world_pos.x > 1600.0, "past him: {}", p.actor.world_pos);
            assert_eq!(w.save.health, 0x30, "never hurt");
        }
        _ => panic!("{step:?} isn't on the Mido and shop route"),
    }
}

/// Phase 4's exit: the file select's new file (`SaveContext::file_select_new`) from its first
/// frame, the game's way, into the Deku Tree: the opening's four scripts in the order their
/// terminators chain them, then the new save's run from where the wake-up leaves Link, with C-Up
/// to Navi on the way.
#[test]
fn a_new_file_into_the_deku_tree() {
    use oot_actors::playthrough::Route;
    use oot_game::cutscene::CS_STATE_IDLE;
    let Some(a) = assets() else { return };
    let route = Route::NewFileDekuTree;
    assert!(route.new_file() && route.preset().is_none());
    let e = a.scenes.entrance_index(route.entrance()).unwrap();
    let Some(mut w) = oot_actors::play_entrance(a.clone(), common::data().unwrap(), common::rules().unwrap(), route.save(e)).ok() else {
        return;
    };
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    let mut seen = Vec::new();
    let mut scripts: Vec<String> = Vec::new();
    loop {
        let pad = run.next(&w);
        if let Some(step) = run.take_done() {
            seen.push(step);
            match step {
                Step::Nightmare => assert_eq!((w.scene_id, w.save.scene_layer), (0x51, 4)),
                Step::NaviSent => assert_eq!((w.scene_id, w.save.scene_layer), (SCENE_SPOT04, 7)),
                Step::WakeUp => {
                    // The wake-up's end: in his house's layer 4, at cue 5's point, free.
                    assert_eq!((w.scene_id, w.save.scene_layer, w.save.cutscene_index), (SCENE_LINK_HOME, 4, 0));
                    assert_eq!(w.player().actor.world_pos, Vec3::new(0.0, 0.0, 60.0));
                    assert_eq!(run.texts, [0x109D, 0x109E, 0x109F, 0x1099, 0x109A, 0x1095, 0x1096, 0x1000, 0x1098]);
                }
                Step::Navi => {
                    // Her C-Up text in Kokiri Forest: 0x140, and naviTimer 3001 on.
                    assert_eq!(run.texts.last(), Some(&0x140));
                    assert!(w.save.navi_timer >= 3001);
                }
                Step::DekuTree => {
                    assert_eq!(w.scene_id, SCENE_YDAN);
                    assert_eq!((w.cs_ctx.state, w.player().action), (CS_STATE_IDLE, Action::StandingStill));
                }
                _ => {}
            }
        }
        let Some(pad) = pad else { break };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        if let Some(s) = &w.cs_ctx.segment
            && w.cs_ctx.state != CS_STATE_IDLE
            && scripts.last() != Some(&format!("{}/{}", s.file, s.name))
        {
            scripts.push(format!("{}/{}", s.file, s.name));
        }
    }
    assert_eq!(run.failure, None, "the run stopped at {}", run.at());
    assert_eq!(&seen[..6], [Step::Nightmare, Step::NaviSent, Step::WakeUp, Step::House, Step::OutDoor, Step::Ladder]);
    let navi = seen.iter().position(|&s| s == Step::Navi).expect("the talk to Navi");
    assert_eq!(seen[navi + 1], Step::Crawlspace);
    assert_eq!(seen.last(), Some(&Step::DekuTree));
    // The opening's scripts (by their offsets: no XML names them), then the Deku Tree's.
    assert_eq!(scripts, ["link_home_scene/0x15D0", "spot00_scene/0x12400", "spot04_scene/0xA6D0", "link_home_scene/0x1040", "ovl_Bg_Treemouth/D_808BCE20", "ovl_Bg_Treemouth/D_808BD520", "ydan_scene/gDekuTreeIntroCs"]);
}
