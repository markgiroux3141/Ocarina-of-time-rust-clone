//! GAME-07 milestone 5 against the C: Anju, the cucco lady (`z_en_niw_lady.c`), and the cuccos
//! a struck cucco calls (`z_en_attack_niw.c`, spawned by `z_en_niw.c`'s
//! `EnNiw_SpawnAttackCucco`).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CLEFT, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_attack_niw::{self, EnAttackNiw};
use oot_actors::en_niw::{self, EnNiw};
use oot_actors::en_niw_lady::{self as lady, Action, EnNiwLady, MISSING_CUCCO_TEXT_IDS};
use oot_actors::player::Player;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_TALK, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::{NA_SE_SY_ERROR, NA_SE_SY_TRE_BOX_APPEAR};
use oot_game::collision_check::AC_HIT;
use oot_game::message::{MSGMODE_TEXT_AWAIT_INPUT, TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_EVENT, TEXT_STATE_NONE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `EVENTCHKINF_A3` (`save.h`): Kakariko's intro seen.
const EVENTCHKINF_A3: u16 = 0xA3;
/// Anju by day in Kakariko (`spot01`, layer 0) and in Impa's house (`labo`, layers 0 to 3).
const KAKARIKO_ANJU: (f32, f32) = (486.0, 1423.0);
const IMPAS_HOUSE_ANJU: (f32, f32) = (-99.0, 117.0);

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Link (`adult` or not) entered at `entrance` at `hour`:00 with `preset`, `edit` applied to the
/// save last; two frames on (the room's actors and objects in).
fn enter_with(a: &Arc<GameAssets>, entrance: &str, adult: bool, hour: i32, preset: &str, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset(preset).unwrap();
    save.adult = adult;
    edit(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 2);
    w
}

/// Kakariko by day from its entrance as a child, its intro seen.
fn kakariko(a: &Arc<GameAssets>, edit: impl FnOnce(&mut SaveContext)) -> PlayState {
    enter_with(a, "ENTR_KAKARIKO_VILLAGE_0", false, 10, "deku-tree-dead", |s| {
        s.set_event_chk_inf(EVENTCHKINF_A3);
        edit(s);
    })
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn all<T: 'static + ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

/// The `T` placed at (`x`, `z`).
fn at<T: 'static + ActorImpl>(w: &PlayState, x: f32, z: f32) -> ActorHandle {
    let v: Vec<_> = all::<T>(w).into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| (a.home_pos.x - x).abs() < 1.0 && (a.home_pos.z - z).abs() < 1.0)).collect();
    assert_eq!(v.len(), 1, "one {} at ({x}, {z})", std::any::type_name::<T>());
    v[0]
}

fn base(w: &PlayState, h: ActorHandle) -> &Actor {
    w.actors.actor(h).unwrap()
}

fn anju(w: &PlayState, h: ActorHandle) -> &EnNiwLady {
    w.actors.downcast::<EnNiwLady>(h).unwrap()
}

/// Until Anju (`h`) has her objects (`func_80AB9F24`), then three frames: her first action, the
/// next, and the animation it chose (`unk_26E`, read at the next update's start).
fn settle(w: &mut PlayState, h: ActorHandle) {
    let mut n = 0;
    while anju(w, h).action == Action::Func80AB9F24 {
        idle(w, 1);
        n += 1;
        assert!(n < 30, "her objects never came in");
    }
    idle(w, 3);
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// Kakariko's seven cuccos by their index (`unk_2AA`, `sKakarikoPosList`).
fn cuccos(w: &PlayState) -> [ActorHandle; 7] {
    let mut v: Vec<_> = all::<EnNiw>(w).into_iter().map(|h| (w.actors.downcast::<EnNiw>(h).unwrap().unk_2aa, h)).collect();
    v.sort_by_key(|&(i, _)| i);
    assert_eq!(v.iter().map(|&(i, _)| i).collect::<Vec<_>>(), [0, 1, 2, 3, 4, 5, 6], "Kakariko's seven cuccos");
    v.iter().map(|&(_, h)| h).collect::<Vec<_>>().try_into().unwrap()
}

/// Puts the cucco `h` at (`x`, `z`) in the pen (90 up, falling), its home there too, so it stays
/// about it (`func_80AB6570` hops about `unk_2B8`).
fn put(w: &mut PlayState, h: ActorHandle, x: f32, z: f32) {
    let c = w.actors.downcast_mut::<EnNiw>(h).unwrap();
    let p = Vec3::new(x, 90.0, z);
    c.actor.world_pos = p;
    c.actor.home_pos = p;
    c.unk_2ac = p;
    c.unk_2b8 = p;
}

/// The pen's `i`th spot (all well inside |x - 330| < 90, |z - 1610| < 190).
fn pen_spot(i: usize) -> (f32, f32) {
    (300.0 + 20.0 * (i % 3) as f32, 1560.0 + 40.0 * (i / 3) as f32)
}

/// `infTable[INFTABLE_INDEX_199_19A_19B_19C_19D_19E_19F]`'s seven cucco bits.
fn pen_bits(w: &PlayState) -> Vec<bool> {
    (0..7).map(|i| w.save.get_inf_table(lady::INFTABLE_199 + i)).collect()
}

#[test]
fn anju_by_day_in_kakariko_by_night_in_impas_house() {
    let Some(a) = assets() else { return };
    // EnNiwLady_Init in Kakariko: unk_278 0, cullingVolumeDistance 600, func_80AB9F24 until
    // object_ane and object_os_anime are in; then gCuccoLadySkel, gravity -3, scale 0.01,
    // ATTENTION_RANGE_6, and (a child without ITEMGETINF_0C) gObjOsAnim_A630, func_80ABA21C:
    // 0x5036, unk_262 TEXT_STATE_DONE, func_80ABA244.
    let mut w = kakariko(&a, |_| {});
    let h = at::<EnNiwLady>(&w, KAKARIKO_ANJU.0, KAKARIKO_ANJU.1);
    settle(&mut w, h);
    let n = anju(&w, h);
    assert_eq!((n.unk_278, n.unk_27e, n.action), (0, 1, Action::Func80ABA244));
    assert_eq!(n.actor.culling_volume_distance, 600.0);
    assert_eq!((n.actor.gravity, n.actor.scale, n.actor.target_mode), (-3.0, Vec3::splat(0.01), 6));
    assert_eq!(n.actor.text_id, 0x5036);
    assert_eq!(n.unk_262, TEXT_STATE_DONE);
    // No text open: unk_26E 101 each frame, EnNiwLady_ChoseAnimation(100): gObjOsAnim_A630,
    // unk_276 0 (her head eased to her front).
    assert!(n.skel.is("gObjOsAnim_A630"));
    assert_eq!((n.unk_270, n.unk_276), (100, 0));

    // By day in Impa's house (SCENE_IMPAS_HOUSE: unk_278 1): Actor_Kill.
    let w = enter_with(&a, "ENTR_IMPAS_HOUSE_0", false, 10, "deku-tree-dead", |_| {});
    assert!(all::<EnNiwLady>(&w).is_empty(), "not in Impa's house by day");
    // By night: gObjOsAnim_07D0, func_80ABAD38 (unk_262 TEXT_STATE_DONE), func_80ABAD7C: 0x503D,
    // unk_26E 8 with no text open: EnNiwLady_ChoseAnimation(7), gObjOsAnim_0718.
    let mut w = enter_with(&a, "ENTR_IMPAS_HOUSE_0", false, 20, "deku-tree-dead", |_| {});
    let h = at::<EnNiwLady>(&w, IMPAS_HOUSE_ANJU.0, IMPAS_HOUSE_ANJU.1);
    settle(&mut w, h);
    let n = anju(&w, h);
    assert_eq!((n.unk_278, n.action), (1, Action::Func80ABAD7C));
    assert_eq!(n.actor.text_id, 0x503D);
    assert!(n.skel.is("gObjOsAnim_0718"));
    // Talked to: unk_274 1, unk_26E unk_27A (0) + 9: gObjOsAnim_9F94 next frame; func_80ABAD38.
    w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_TALK;
    idle(&mut w, 1);
    let n = anju(&w, h);
    assert_eq!((n.unk_274, n.action), (1, Action::Func80ABAD38));
}

#[test]
fn the_pen_counts_its_cuccos_and_keeps_their_bits() {
    let Some(a) = assets() else { return };
    let mut w = kakariko(&a, |_| {});
    let h = at::<EnNiwLady>(&w, KAKARIKO_ANJU.0, KAKARIKO_ANJU.1);
    let c = cuccos(&w);
    settle(&mut w, h);
    // None in the pen: 0x5036, no bits.
    assert_eq!(anju(&w, h).cuccos_in_pen, 0);
    assert_eq!(pen_bits(&w), [false; 7]);
    // Cucco 2 (by the pen, (796, 80, 1639)) put in: func_80ABA244 counts it (Anju, an NPC,
    // updates before the props) and sets INFTABLE_19B (D_80ABB3B4[2]); 0x5070.
    let (x, z) = pen_spot(0);
    put(&mut w, c[2], x, z);
    idle(&mut w, 1);
    assert_eq!(anju(&w, h).cuccos_in_pen, 1);
    assert_eq!(pen_bits(&w), [false, false, true, false, false, false, false]);
    assert_eq!(anju(&w, h).actor.text_id, 0x5070);
    // Cucco 5 too: INFTABLE_19E; 0x5072.
    let (x, z) = pen_spot(1);
    put(&mut w, c[5], x, z);
    idle(&mut w, 1);
    assert_eq!(anju(&w, h).cuccos_in_pen, 2);
    assert_eq!(pen_bits(&w), [false, false, true, false, false, true, false]);
    assert_eq!(anju(&w, h).actor.text_id, 0x5072);
    // Cucco 2 out again (|x - 330| = 90: not < 90): its bit cleared.
    put(&mut w, c[2], 420.0, 1610.0);
    idle(&mut w, 1);
    assert_eq!(anju(&w, h).cuccos_in_pen, 1);
    assert_eq!(pen_bits(&w), [false, false, false, false, false, true, false]);
    // The far edge in z: |z - 1610| < 190.
    put(&mut w, c[2], 330.0, 1610.0 + 189.0);
    idle(&mut w, 1);
    assert_eq!(anju(&w, h).cuccos_in_pen, 2);
    assert!(pen_bits(&w)[2]);
}

#[test]
fn her_text_follows_the_count() {
    let Some(a) = assets() else { return };
    let mut w = kakariko(&a, |_| {});
    let h = at::<EnNiwLady>(&w, KAKARIKO_ANJU.0, KAKARIKO_ANJU.1);
    let c = cuccos(&w);
    settle(&mut w, h);
    // sMissingCuccoTextIds[count]: 0x5036, 0x5070, 0x5072, 0x5037, 0x5038, 0x5039, 0x503A; with
    // seven (unk_26C 0): phi_s1 7, 0x503B.
    let expect = [0x5036, 0x5070, 0x5072, 0x5037, 0x5038, 0x5039, 0x503A, 0x503B];
    assert_eq!(anju(&w, h).actor.text_id, expect[0]);
    for (i, &cucco) in c.iter().enumerate() {
        let (x, z) = pen_spot(i);
        put(&mut w, cucco, x, z);
        idle(&mut w, 1);
        assert_eq!(anju(&w, h).cuccos_in_pen, i as i16 + 1);
        assert_eq!(anju(&w, h).actor.text_id, expect[i + 1], "{} in the pen", i + 1);
    }
    assert_eq!(pen_bits(&w), [true; 7]);
}

#[test]
fn more_brought_she_chimes_fewer_she_errs() {
    let Some(a) = assets() else { return };
    let mut w = kakariko(&a, |_| {});
    w.audio.log = Some(Default::default());
    let h = at::<EnNiwLady>(&w, KAKARIKO_ANJU.0, KAKARIKO_ANJU.1);
    let c = cuccos(&w);
    settle(&mut w, h);
    for (i, &cucco) in c.iter().take(3).enumerate() {
        let (x, z) = pen_spot(i);
        put(&mut w, cucco, x, z);
    }
    // Talked to with 3 (unk_26A 0): unk_26E phi_s1 + 1 = 4; 3 != 0 and not fewer: phi_s1 + 1 < 9,
    // NA_SE_SY_TRE_BOX_APPEAR; unk_26A 3.
    w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_TALK;
    idle(&mut w, 1);
    let f = w.audio.frames;
    assert_eq!(sfx_frames(&w, NA_SE_SY_TRE_BOX_APPEAR), [f]);
    assert_eq!(anju(&w, h).unk_26a, 3);
    // One out, talked to again: 2 < 3, NA_SE_SY_ERROR; unk_26A stays 3.
    put(&mut w, c[0], 600.0, 1610.0);
    idle(&mut w, 1);
    w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_TALK;
    idle(&mut w, 1);
    assert_eq!(sfx_frames(&w, NA_SE_SY_ERROR), [w.audio.frames]);
    assert_eq!(anju(&w, h).unk_26a, 3);
    assert_eq!(anju(&w, h).action, Action::Func80ABA244);

    // After the reward (unk_26C 1) with fewer than seven: phi_s1 9, 0x503C; talked to (NTSC 1.1
    // and later): NA_SE_SY_ERROR, unk_26C 2, unk_262 TEXT_STATE_EVENT, func_80ABA654. The bits
    // aren't kept once unk_26C is set.
    w.save.set_item_get_inf(lady::ITEMGETINF_0C);
    w.actors.downcast_mut::<EnNiwLady>(h).unwrap().unk_26c = 1;
    put(&mut w, c[0], 300.0, 1560.0);
    put(&mut w, c[6], 600.0, 1610.0);
    w.save.inf_table[25] = 0;
    idle(&mut w, 1);
    assert_eq!(anju(&w, h).actor.text_id, MISSING_CUCCO_TEXT_IDS[9]);
    assert_eq!(anju(&w, h).actor.text_id, 0x503C);
    assert_eq!(pen_bits(&w), [false; 7]);
    w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_TALK;
    idle(&mut w, 1);
    let n = anju(&w, h);
    assert_eq!((n.unk_26c, n.unk_262, n.action), (2, TEXT_STATE_EVENT, Action::Func80ABA654));
    assert_eq!(sfx_frames(&w, NA_SE_SY_ERROR).last(), Some(&w.audio.frames));
}

/// Presses A whenever the message box waits (or her event text does), from the talk's start
/// until `done` or 3000 frames; the texts shown.
fn read_on(w: &mut PlayState, done: impl Fn(&PlayState) -> bool) -> Vec<u16> {
    let mut prev = PadState::default();
    let mut texts = Vec::new();
    for _ in 0..3000 {
        let st = w.message_state();
        if st != TEXT_STATE_NONE && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        if done(w) {
            return texts;
        }
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_EVENT) || w.msg_ctx.msg_mode == MSGMODE_TEXT_AWAIT_INPUT;
        let pad = if waits && prev.button == 0 { PadState { button: BTN_A, ..Default::default() } } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
    }
    panic!("never done: {texts:x?}");
}

/// Link 40 in front of Anju (`h`), facing her, until she offers to talk; then A.
fn talk_to(w: &mut PlayState, h: ActorHandle) {
    let p = base(w, h).world_pos;
    for _ in 0..30 {
        w.place_player(Vec3::new(p.x, p.y, p.z - 40.0), 0);
        idle(w, 1);
        if player(w).target_actor == Some(h) {
            let a = PadState { button: BTN_A, ..Default::default() };
            w.tick_with(scripted_input(PadState::default(), a));
            w.tick_with(scripted_input(a, PadState::default()));
            return;
        }
    }
    panic!("Anju never offered to talk");
}

#[test]
fn seven_penned_she_gives_the_empty_bottle() {
    let Some(a) = assets() else { return };
    let mut w = kakariko(&a, |_| {});
    w.audio.log = Some(Default::default());
    let h = at::<EnNiwLady>(&w, KAKARIKO_ANJU.0, KAKARIKO_ANJU.1);
    let c = cuccos(&w);
    settle(&mut w, h);
    for (i, &cucco) in c.iter().enumerate() {
        let (x, z) = pen_spot(i);
        put(&mut w, cucco, x, z);
    }
    idle(&mut w, 1);
    assert_eq!(pen_bits(&w), [true; 7]);
    assert_eq!(anju(&w, h).actor.text_id, 0x503B);
    // Talked to with seven: phi_s1 7: NA_SE_SY_TRE_BOX_APPEAR, unk_26C 1, unk_262
    // TEXT_STATE_EVENT, unk_26A 7, the seven bits cleared, func_80ABA654.
    talk_to(&mut w, h);
    let n = anju(&w, h);
    assert_eq!((n.unk_26c, n.unk_26a, n.unk_262, n.action), (1, 7, TEXT_STATE_EVENT, Action::Func80ABA654));
    assert_eq!(pen_bits(&w), [false; 7]);
    assert_eq!(sfx_frames(&w, NA_SE_SY_TRE_BOX_APPEAR).len(), 1);
    // 0x503B read (its event): Message_CloseTextbox, GI_BOTTLE_EMPTY offered (func_80ABAC00 until
    // Player has it); the item's text read: func_80ABAC84 sets ITEMGETINF_0C, back to
    // func_80ABA244.
    let texts = read_on(&mut w, |w| w.save.get_item_get_inf(lady::ITEMGETINF_0C));
    assert_eq!(texts.first(), Some(&0x503B), "{texts:x?}");
    assert!(oot_game::item::inventory_has_empty_bottle(&w.save), "the Empty Bottle");
    let n = anju(&w, h);
    assert_eq!((n.action, n.unk_262), (Action::Func80ABA244, TEXT_STATE_DONE));
    // Seven still in, the reward given (unk_26C 1): phi_s1 8, 0x503D; no bits kept now.
    idle(&mut w, 2);
    assert_eq!(anju(&w, h).actor.text_id, 0x503D);
    assert_eq!(pen_bits(&w), [false; 7]);
}

#[test]
fn as_an_adult_the_pocket_egg_and_the_trade() {
    let Some(a) = assets() else { return };
    // Adult Kakariko by day (layer 2): Anju at (496, 80, 1418). func_80AB9F24: gObjOsAnim_07D0,
    // func_80ABA778: no ITEMGETINF_2C: unk_27A 0 (not talked to), unk_273 1, unk_262
    // TEXT_STATE_CHOICE, 0x503E; func_80ABA878.
    let mut w = enter_with(&a, "ENTR_KAKARIKO_VILLAGE_0", true, 10, "deku-tree-dead", |s| s.set_event_chk_inf(EVENTCHKINF_A3));
    let h = at::<EnNiwLady>(&w, 496.0, 1418.0);
    settle(&mut w, h);
    let n = anju(&w, h);
    assert_eq!((n.action, n.unk_27a, n.unk_273, n.unk_262), (Action::Func80ABA878, 0, 1, TEXT_STATE_CHOICE));
    assert_eq!(n.actor.text_id, 0x503E);
    // Talked to (nothing shown): unk_274 1, unk_26E 21, func_80ABA9B8 (the choice); next frame
    // EnNiwLady_ChoseAnimation(20): gObjOsAnim_0718.
    w.actors.actor_mut(h).unwrap().flags |= ACTOR_FLAG_TALK;
    idle(&mut w, 2);
    let n = anju(&w, h);
    assert_eq!((n.unk_274, n.action), (1, Action::Func80ABA9B8));
    assert!(n.skel.is("gObjOsAnim_0718"));

    // With the Pocket Egg got (and hatched: the Pocket Cucco on C-Left) and Talon woken in
    // Kakariko: func_80ABA778: unk_27A 9, 0x5042. Link 40 in front of her, facing her: she offers a
    // talk for EXCH_ITEM_POCKET_CUCCO within 50 (Actor_OfferTalkExchangeEquiCylinder); C-Left
    // shows it (Player_ActionHandler_13). Shown: NA_SE_SY_TRE_BOX_APPEAR, Player's text
    // sTradeItemTextIds[5] (0x5043), unk_26E 30, unk_262 TEXT_STATE_CHOICE, func_80ABAB08.
    let mut w = enter_with(&a, "ENTR_KAKARIKO_VILLAGE_0", true, 10, "deku-tree-dead", |s| {
        s.set_event_chk_inf(EVENTCHKINF_A3);
        s.set_item_get_inf(lady::ITEMGETINF_2C);
        s.set_event_chk_inf(lady::EVENTCHKINF_TALON_WOKEN_IN_KAKARIKO);
        s.inventory.items[oot_game::item::SLOT_TRADE_ADULT] = oot_game::item::ITEM_POCKET_CUCCO;
        s.equip_item_on_c_left(oot_game::item::ITEM_POCKET_CUCCO);
    });
    w.audio.log = Some(Default::default());
    let h = at::<EnNiwLady>(&w, 496.0, 1418.0);
    settle(&mut w, h);
    let n = anju(&w, h);
    assert_eq!((n.unk_27a, n.unk_262), (9, TEXT_STATE_DONE));
    assert_eq!(n.actor.text_id, 0x5042);
    let p = base(&w, h).world_pos;
    let mut n = 0;
    while player(&w).target_actor != Some(h) {
        w.place_player(Vec3::new(p.x, p.y, p.z - 40.0), 0);
        idle(&mut w, 1);
        n += 1;
        assert!(n < 30, "Anju never offered to talk");
    }
    let c_left = PadState { button: BTN_CLEFT, ..Default::default() };
    w.tick_with(scripted_input(PadState::default(), c_left));
    w.tick_with(scripted_input(c_left, PadState::default()));
    n = 0;
    while anju(&w, h).action == Action::Func80ABA878 {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 60, "she never took the talk");
    }
    assert_eq!(player(&w).exchange_item_id, lady::EXCH_ITEM_POCKET_CUCCO);
    let n = anju(&w, h);
    assert_eq!((n.unk_262, n.action), (TEXT_STATE_CHOICE, Action::Func80ABAB08));
    assert_eq!(player(&w).actor.text_id, 0x5043);
    assert_eq!(sfx_frames(&w, NA_SE_SY_TRE_BOX_APPEAR), [w.audio.frames]);
    // unk_26E 30 was read at this update's start: EnNiwLady_ChoseAnimation(29), gObjOsAnim_9F94.
    idle(&mut w, 1);
    assert!(anju(&w, h).skel.is("gObjOsAnim_9F94"));
    assert_eq!(anju(&w, h).unk_270, 29);
}

/// Strikes the cucco `h` once (`AC_HIT`, as Player's sword would) with one strike left: the call.
fn strike_last(w: &mut PlayState, h: ActorHandle) {
    let c = w.actors.downcast_mut::<EnNiw>(h).unwrap();
    c.unk_2a4 = 1;
    c.collider.base.ac_flags |= AC_HIT;
}

/// The `En_Attack_Niw` whose parent is `h` (killed ones too, until they're deleted).
fn flock(w: &PlayState, h: ActorHandle) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&f| w.actors.downcast::<EnAttackNiw>(f).is_some() && base(w, f).parent == Some(h)).collect()
}

#[test]
fn a_cucco_struck_enough_calls_the_flock() {
    let Some(a) = assets() else { return };
    let mut w = kakariko(&a, |_| {});
    let h = at::<EnNiw>(&w, -247.0, 854.0);
    let p = base(&w, h).world_pos;
    w.place_player(Vec3::new(p.x, p.y, p.z - 100.0), 0);
    idle(&mut w, 1);
    // func_80AB747C: the last strike (unk_2A4 0); the next update, 100 from Link (> 10):
    // D_80AB85E0 1, unk_2A8 1, func_80AB70A0 (the camera 2290), then 100 frames (func_80AB70F8)
    // and 60 (func_80AB714C), then func_80AB7204: EnNiw_SpawnAttackCucco every 10 frames.
    strike_last(&mut w, h);
    idle(&mut w, 2);
    let c = w.actors.downcast::<EnNiw>(h).unwrap();
    assert_eq!((c.unk_2a4, c.unk_2a8), (0, 1));
    let mut n = 0;
    while flock(&w, h).is_empty() {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 200, "no attack cucco");
    }
    assert!((158..=164).contains(&n), "the flock comes 160 frames on: {n}");
    // Actor_SpawnAsChild(ACTOR_EN_ATTACK_NIW, params 0): its parent the cucco, counted (unk_296).
    let f = flock(&w, h)[0];
    let fa = w.actors.downcast::<EnAttackNiw>(f).unwrap();
    assert_eq!(fa.actor.id, en_attack_niw::ACTOR_EN_ATTACK_NIW);
    assert_eq!(fa.actor.id, en_niw::ACTOR_EN_ATTACK_NIW);
    assert_eq!(base(&w, h).child, Some(f));
    assert_eq!(w.actors.downcast::<EnNiw>(h).unwrap().unk_296, 1);
    // EnAttackNiw_Init: ATTENTION_RANGE_1, gravity 0 (after the chain's -2), scale 0.01, not
    // attention-enabled, func_809B5670.
    assert_eq!((fa.actor.target_mode, fa.actor.gravity, fa.actor.scale), (1, 0.0, Vec3::splat(0.01)));
    assert!(fa.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED == 0);
    assert!(fa.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED != 0);
    // The flock grows to 7, one each 10 frames; unk_296 always the live flock (each counted off
    // as it goes, EnAttackNiw_Destroy).
    let mut most = 0;
    let mut dived = false;
    let mut knocked = false;
    for _ in 0..900 {
        idle(&mut w, 1);
        let live = flock(&w, h).len();
        let c = w.actors.downcast::<EnNiw>(h).unwrap();
        assert_eq!(c.unk_296 as usize, live);
        most = most.max(live);
        dived |= flock(&w, h).iter().any(|&f| w.actors.downcast::<EnAttackNiw>(f).unwrap().action == en_attack_niw::Action::Func809B59B0);
        // Within 20 of Link, the knockdown: the flock's parent's timer9 70.
        knocked |= c.timer9 > 0;
    }
    assert_eq!(most, 7);
    assert!(dived, "one came down and dived at Link (func_809B59B0)");
    assert!(knocked, "Link was knocked down (timer9)");
}
