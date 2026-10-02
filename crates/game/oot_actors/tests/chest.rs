//! The Kokiri Sword's chest on a new save (GAME-03 milestone 1's exit): `En_Box` offering its
//! item, Player's get-item flow (`Player_ActionHandler_2`, `func_8083A434`, `Player_Action_8084E6D4`,
//! `func_8084DFF4` in `z_player.c`), the item's text, and the sword put on B.
//!
//! The chest is room 2's `En_Box` at (-232, 178, 2245), params 0x04E0:
//! `ENBOX_TYPE_BIG_DEFAULT` (`params >> 12 & 0xF`), `GI_SWORD_KOKIRI` 0x27 (`params >> 5 &
//! 0x7F`), treasure flag 0 (`params & 0x1F`). Room 2 is behind a crawlspace; the test loads it
//! and places Link in front of the chest (the way there is the `playthrough` test's
//! `a_new_save_to_the_kokiri_sword`).
//!
//! Also a piece of heart, which `En_Item00` offers with `Actor_OfferGetItemNearby` (`Actor_OfferGetItem` with
//! 50 and 10) when Link touches it (`z_en_item00.c`: `EnItem00_Update`).

mod common;

use std::sync::Arc;

use eng_input::pad::{BTN_A, BTN_B, PadState};
use glam::{Mat4, Vec3};
use oot_actors::PlayExt;
use oot_actors::en_box::EnBox;
use oot_actors::player::{Action, UpperAction};
use oot_game::actor_ctx::ActorHandle;
use oot_game::camera::{CAM_SET_SLOW_CHEST_CS, CAM_SET_TURN_AROUND};
use oot_game::interface::{DO_ACTION_NEXT, DO_ACTION_OPEN};
use oot_game::item::{EQUIP_TYPE_SWORD, GI_SWORD_KOKIRI, ITEM_SWORD_KOKIRI};
use oot_game::message::{MSGMODE_NONE, TEXT_STATE_CLOSING};
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::{ITEM_NONE, SaveContext};

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const NONE: PadState = PadState { button: 0, stick_x: 0, stick_y: 0 };
const A: PadState = PadState { button: BTN_A, stick_x: 0, stick_y: 0 };
const B: PadState = PadState { button: BTN_B, stick_x: 0, stick_y: 0 };

fn frames(w: &mut PlayState, prev: &mut PadState, pad: PadState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(*prev, pad));
        *prev = pad;
    }
}

const CHEST_POS: Vec3 = Vec3::new(-232.0, 178.0, 2245.0);

fn chest(w: &PlayState) -> Option<ActorHandle> {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnBox>(h).is_some_and(|b| b.actor.home_pos.distance(CHEST_POS) < 1.0))
}

/// Kokiri Forest on a new save, room 2 loaded with its chest initialised.
fn room2(a: Arc<GameAssets>) -> Option<(PlayState, PadState)> {
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_0").expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    let sound = a.pack.audio_data().expect("the pack's audio data");
    let mut w = oot_actors::play_entrance(a, common::data()?, common::rules()?, save).expect("Play_Init");
    // The audio library offline beside it, for the fanfares.
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&sound, false)));
    let mut prev = NONE;
    frames(&mut w, &mut prev, NONE, 20);
    // What the crawlspace's room change would do: room 2 in, room 0 kept as the previous room.
    w.room_request(2);
    frames(&mut w, &mut prev, NONE, 1);
    w.room_change_done();
    for _ in 0..20 {
        if chest(&w).is_some() {
            break;
        }
        frames(&mut w, &mut prev, NONE, 1);
    }
    Some((w, prev))
}

/// The meshes the last frame draws (opaque and translucent).
fn drawn(w: &PlayState) -> Vec<String> {
    let f = w.current_frame();
    let mut out = DrawOut::default();
    w.draw(&f, &ViewInfo::new(f.view.eye, Mat4::IDENTITY), &mut out);
    out.opa.iter().chain(out.xlu.iter()).map(|c| c.mesh.name.clone()).collect()
}

/// Link's model variant as drawn (`player/<age>/<left hand>+<right hand>+<sheath>+<waist>`).
fn link_mesh(w: &PlayState) -> String {
    drawn(w).into_iter().find(|n| n.starts_with("player/")).expect("Link")
}

/// Runs until `done`, at most `max` frames, pressing A every 10 frames if `press_a`.
fn until(w: &mut PlayState, prev: &mut PadState, max: usize, press_a: bool, done: impl Fn(&PlayState) -> bool) {
    for i in 0..max {
        if done(w) {
            return;
        }
        let pad = if press_a && i % 10 == 0 { A } else { NONE };
        frames(w, prev, pad, 1);
    }
    panic!("not done after {max} frames: {:?}", w.player().action);
}

#[test]
fn the_kokiri_sword_chest_on_a_new_save() {
    let Some(a) = assets() else { return };
    let Some((mut w, mut prev)) = room2(a.clone()) else { return };
    let c = chest(&w).expect("the chest");
    let sword = a.items.get_item(GI_SWORD_KOKIRI).expect("sGetItemTable's GI_SWORD_KOKIRI").clone();
    // GET_ITEM(ITEM_SWORD_KOKIRI, OBJECT_GI_SWORD_1, GID_SWORD_KOKIRI, 0xA4, 0x80, CHEST_ANIM_LONG).
    assert_eq!((sword.item_id, sword.text_id), (ITEM_SWORD_KOKIRI, 0xA4));
    assert!(sword.gi > 0, "CHEST_ANIM_LONG: gi is +(GID_SWORD_KOKIRI + 1)");
    // Sram_InitNewSave: nothing owned but the Kokiri tunic and boots, B empty.
    assert_eq!(w.save.inventory.equipment, 0x1100);
    assert_eq!(w.save.equips.button_items[0], ITEM_NONE);

    // In front of it (it faces +z; Link at -z of it, facing +z). The chest's collision pushes
    // Link out to his wall radius.
    w.place_player(CHEST_POS - Vec3::Z * 30.0, 0);
    frames(&mut w, &mut prev, NONE, 2);
    w.audio.log = Some(Default::default());
    // EnBox_WaitOpen: Actor_OfferGetItemNearby(&this->dyna.actor, play, -(params >> 5 & 0x7F)) while
    // Link is in front (Actor_WorldToActorCoords: within 20 of its axis, -50 < z < 0 in its space) and
    // facing it.
    // No sword on B, no shield: Player_OverrideLimbDrawGameplayDefault's child sheath entry
    // past the shields (buttonItems[0] != ITEM_SWORD_KOKIRI) is empty.
    assert_eq!(link_mesh(&w), "player/child/gLinkChildLeftHandNearDL+gLinkChildRightHandNearDL+-+gLinkChildWaistNearDL");
    let p = w.player();
    assert_eq!(p.interact_range_actor, Some(c), "the chest offers its item");
    assert_eq!(p.get_item_id, -GI_SWORD_KOKIRI);
    assert_eq!(w.interface_ctx.unk_1f0, DO_ACTION_OPEN, "A says Open");

    // A: Player_ActionHandler_2's chest branch. Link steps to 29.4343 in front of it, facing it; the
    // item is obtainable and gi > 0, so the slow open (gPlayerAnim_clink_demo_Tbox_open,
    // ageProperties->unk_98), chest->unk_1F4 = 1 and CAM_SET_SLOW_CHEST_CS.
    frames(&mut w, &mut prev, A, 1);
    frames(&mut w, &mut prev, NONE, 1);
    let p = w.player();
    assert_eq!(w.data.anim_name(p.skel.animation), "clink_demo_Tbox_open");
    assert!((p.actor.world_pos.z - (CHEST_POS.z - 29.4343)).abs() < 0.5, "at the chest: {:?}", p.actor.world_pos);
    assert!((p.actor.world_pos.x - CHEST_POS.x).abs() < 0.1);
    assert_eq!(p.actor.shape_rot.y, 0);
    assert_eq!(w.actors.downcast::<EnBox>(c).unwrap().unk_1f4, 1, "the chest opens slowly");
    assert_eq!(w.game_camera.setting, CAM_SET_SLOW_CHEST_CS);

    // Player_Action_8084E6D4: at the animation's end, gPlayerAnim_link_demo_get_itemA with av2.actionVar2 = 2
    // and Player_SetTurnAroundCamera(play, 9) (CAM_SET_TURN_AROUND).
    until(&mut w, &mut prev, 200, false, |w| w.data.anim_name(w.player().skel.animation) == "link_demo_get_itemA");
    assert_eq!(w.player().action_var2, 2);
    assert_eq!(w.game_camera.setting, CAM_SET_TURN_AROUND);
    assert_eq!(w.player().unk_862, 0, "nothing held up yet");

    // Frame 21: func_808332F4, unk_862 = ABS(giEntry->gi): Link holds the sword up
    // (Player_DrawGetItem draws GetItem_Draw's GID_SWORD_KOKIRI).
    until(&mut w, &mut prev, 40, false, |w| w.player().unk_862 != 0);
    assert_eq!(w.player().unk_862, sword.gi.abs() as i16);
    let gid = sword.gi.abs() - 1;
    let names = drawn(&w);
    assert!(names.iter().any(|n| n.starts_with(&format!("bake/GetItem/{gid:02X}/"))), "the sword held up: {names:?}");

    // func_8084DFF4 (av2.actionVar2 1 once get_itemA ends): the text and Item_Give. Item_Give
    // (ITEM_SWORD_KOKIRI) only sets the owned bit (OWNED_EQUIP_FLAG(EQUIP_TYPE_SWORD,
    // EQUIP_VALUE_SWORD_KOKIRI - 1)): the sword isn't equipped.
    until(&mut w, &mut prev, 60, false, |w| w.msg_ctx.text_id == 0xA4);
    assert_eq!(w.player().action_var1, 1);
    assert_eq!(w.save.inventory.equipment, 0x1101, "the sword owned");
    assert_eq!(w.save.cur_equip_value(EQUIP_TYPE_SWORD), 0, "not equipped");
    assert_eq!(w.save.equips.button_items[0], ITEM_NONE, "B still empty");
    until(&mut w, &mut prev, 30, false, |w| w.interface_ctx.unk_1f0 == DO_ACTION_NEXT);
    // MESSAGE_ITEM_ICON 0x3B: the box shows the sword's icon.
    frames(&mut w, &mut prev, NONE, 2);
    let mut out = DrawOut::default();
    w.msg_ctx.draw(&mut out);
    let names: Vec<&str> = out.overlay_2d.iter().map(|c| c.mesh.name.as_str()).collect();
    assert!(names.iter().any(|n| n.ends_with("message/item3B")), "the sword's icon in the box: {names:?}");

    // The fanfares (Audio_PlayFanfare, then Audio_UpdateFanfare a frame later: with the fanfare
    // player off, the bgm players fade out under it, the setup commands bring them back after,
    // and the fanfare starts with a fade of 1): the chest's as it opens (En_Box,
    // NA_BGM_OPEN_TRE_BOX | 0x900), then the item's (func_8084DFF4, NA_BGM_ITEM_GET | 0x900).
    let starts: Vec<u32> = w.audio.log.as_ref().unwrap().seq_cmds.iter().map(|(_, c)| *c).filter(|c| c >> 24 == 0x01).collect();
    assert_eq!(starts, [0x0101_092B, 0x0101_0922]);

    // Through both boxes (A at the box break), to TEXT_STATE_CLOSING, then func_8084DFAC:
    // Link stands, GI_NONE.
    until(&mut w, &mut prev, 600, true, |w| w.message_state() == TEXT_STATE_CLOSING);
    until(&mut w, &mut prev, 60, false, |w| w.msg_ctx.msg_mode == MSGMODE_NONE && w.player().action == Action::StandingStill);
    assert_eq!(w.player().get_item_id, 0);
    // Flags_SetTreasure(play, params & 0x1F), kept for the scene (Play_SaveSceneFlags on the
    // next reinit).
    assert!(w.flags.get_treasure(0), "the chest's treasure flag");

    // B does nothing: B_BTN_ITEM is ITEM_NONE.
    frames(&mut w, &mut prev, B, 1);
    frames(&mut w, &mut prev, NONE, 5);
    assert_eq!(w.player().action, Action::StandingStill);

    // The pause menu's equipping (its stand-in): the sword on B, then Player_SetEquipmentData.
    assert!(w.equip_owned_unworn());
    assert_eq!(w.save.cur_equip_value(EQUIP_TYPE_SWORD), 1);
    assert_eq!(w.save.equips.button_items[0], ITEM_SWORD_KOKIRI, "B gets the sword");
    assert_eq!(w.player().current_sword_item_id, ITEM_SWORD_KOKIRI);
    assert!(!w.equip_owned_unworn(), "nothing else to equip");
    // Player_OverrideLimbDrawGameplayDefault's sheath: the sword in it now B holds it.
    frames(&mut w, &mut prev, NONE, 1);
    assert_eq!(link_mesh(&w), "player/child/gLinkChildLeftHandNearDL+gLinkChildRightHandNearDL+gLinkChildSwordAndSheathNearDL+gLinkChildWaistNearDL");

    // B now draws it (Player_UseItem: the upper body's change) and slashes on the swap frame.
    frames(&mut w, &mut prev, B, 1);
    assert_eq!(w.player().upper, UpperAction::Change, "B draws the sword");
    until(&mut w, &mut prev, 30, false, |w| w.player().action == Action::Attack);
    assert_eq!(w.player().held_item_ap, w.data.items.ap("SWORD_KOKIRI"));
}

#[test]
fn a_piece_of_heart() {
    use oot_actors::en_item00::{ACTOR_EN_ITEM00, ITEM00_HEART_PIECE};
    use oot_game::item::QUEST_HEART_PIECE_COUNT;
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    // One piece already (questItems' top four bits).
    save.inventory.quest_items |= 1 << QUEST_HEART_PIECE_COUNT;
    let Some((data, rules)) = common::data().zip(common::rules()) else { return };
    let mut w = oot_actors::play_entrance(a.clone(), data, rules, save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    let mut prev = NONE;
    frames(&mut w, &mut prev, NONE, 20);
    let pos = w.player().actor.world_pos;
    let h = w.actor_spawn(ACTOR_EN_ITEM00, pos, [0; 3], ITEM00_HEART_PIECE).expect("En_Item00");
    // GET_ITEM(ITEM_HEART_PIECE_2, OBJECT_GI_HEARTS, GID_HEART_PIECE, 0xC2, 0x80,
    // CHEST_ANIM_LONG).
    let gi = a.items.get_item(oot_game::item::GI_HEART_PIECE).expect("GI_HEART_PIECE").clone();
    assert_eq!(gi.text_id, 0xC2);
    // Touching it: Actor_OfferGetItemNearby, then Player's get-item interrupt, held up (unk_862).
    until(&mut w, &mut prev, 60, false, |w| w.player().unk_862 != 0);
    assert_eq!(w.player().unk_862, gi.gi.abs() as i16);
    // func_8084DFF4: Message_StartTextbox before Item_Give, so Message_StartTextbox's 0xC2 +
    // the pieces counts the one owned: 0xC3. Then Item_Give adds the second.
    until(&mut w, &mut prev, 120, false, |w| w.msg_ctx.text_id != 0);
    assert_eq!(w.msg_ctx.text_id, 0xC3);
    assert_eq!(w.save.inventory.quest_items >> QUEST_HEART_PIECE_COUNT, 2);
    // The piece is gone (Actor_HasParent: Player took it), and Link stands after the text.
    until(&mut w, &mut prev, 600, true, |w| w.msg_ctx.msg_mode == MSGMODE_NONE && w.player().action == Action::StandingStill);
    assert!(w.actors.get(h).is_none_or(|a| a.base().killed), "the piece is taken");
    assert_eq!(w.save.health_capacity, 0x30, "two pieces make no container");
    // The second of four: NA_BGM_SMALL_ITEM_GET (the fourth would be NA_BGM_HEART_GET).
    let starts: Vec<u32> = w.audio.log.as_ref().unwrap().seq_cmds.iter().map(|(_, c)| *c).filter(|c| c >> 24 == 0x01).collect();
    assert_eq!(starts, [0x0101_0039]);
}
