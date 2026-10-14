//! `En_Ex_Item` (`z_en_ex_item.c`): the minigames' prizes, each type spawned directly (only the
//! bullet bag is reached in the game so far, from `En_Dnt_Nomal`: tests/en_dnt_nomal.rs) in the
//! Lost Woods' room 1, which loads `object_gi_dekupouch`; the other get-item objects are loaded
//! into the next banks first (`Object_LoadPersistent`). Expected values are worked out from the C
//! in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_ex_item::*;
use oot_actors::playthrough::deku_tree_room_start;
use oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::item::{ITEM_SLINGSHOT, UPG_BULLET_BAG, item_give};
use oot_game::message::TEXT_STATE_NONE;
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// The Lost Woods' room 1 as a child with the slingshot (`UPG_BULLET_BAG` 1), the sound log on;
/// Link on the stumps' floor facing +x; `f` changes the save first.
fn woods(a: &Arc<GameAssets>, f: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LOST_WOODS_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    after_sarias_greeting(&mut save);
    item_give(&mut save, None, ITEM_SLINGSHOT);
    f(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 20);
    deku_tree_room_start(&mut w, 1, Vec3::new(1100.0, -180.0, -30.0), 0x4000);
    idle(&mut w, 20);
    w
}

/// Where the items are spawned: 100 in front of Link, 50 up.
const AT: Vec3 = Vec3::new(1200.0, -130.0, -30.0);

/// `object` loaded unless the room has it (in place of `object_owl`, which room 1 loads and none
/// of its actors use), then an `En_Ex_Item` of `ty`.
fn spawn(w: &mut PlayState, object: Option<i16>, ty: i16) -> ActorHandle {
    if let Some(o) = object
        && w.object_ctx.get_index(o).is_none()
    {
        // object_owl's bank, or the one a get-item object took before.
        let owl = w.assets.as_ref().unwrap().scenes.objects.iter().position(|n| n == "object_owl").unwrap() as i16;
        let bank = (0..w.object_ctx.num).find(|&b| w.object_ctx.status[b].id == owl || TYPES.iter().any(|t| t.1 != OBJECT_GI_DEKUPOUCH && t.1 == w.object_ctx.status[b].id)).expect("a bank to use");
        w.object_ctx.status[bank].id = o;
    }
    w.actor_spawn(ACTOR_EN_EX_ITEM, AT, [0, 0, 0], ty).expect("spawned")
}

fn get(w: &PlayState, h: ActorHandle) -> &EnExItem {
    w.actors.downcast::<EnExItem>(h).expect("En_Ex_Item")
}

fn get_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnExItem {
    w.actors.downcast_mut::<EnExItem>(h).expect("En_Ex_Item")
}

/// The meshes its draw submits, opaque then translucent.
fn drawn(w: &PlayState, h: ActorHandle) -> Vec<String> {
    let it = get(w, h);
    let rs = it.render_state();
    let mut out = DrawOut::default();
    it.draw(&rs, w, &ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
    out.opa.iter().chain(out.xlu.iter()).map(|c| c.mesh.name.clone()).collect()
}

/// Each type: (type, its object, the draw id `EnExItem_Draw` draws, its `giDrawId`, its scale,
/// its action, its `shape.yOffset`), from `EnExItem_Init` and `EnExItem_WaitForObject`.
const TYPES: &[(i16, i16, i16, i16, f32, Action, f32)] = &[
    (EXITEM_BOMB_BAG_BOWLING, OBJECT_GI_BOMBPOUCH, GID_BOMB_BAG_30, GID_BOMB_BAG_30, 0.5, Action::BowlPrize, 0.0),
    // The heart piece's giDrawId stays 0: EnExItem_DrawHeartPiece draws GID_HEART_PIECE.
    (EXITEM_HEART_PIECE_BOWLING, OBJECT_GI_HEARTS, GID_HEART_PIECE, 0, 0.5, Action::BowlPrize, 0.0),
    (EXITEM_BOMBCHUS_BOWLING, OBJECT_GI_BOMB_2, GID_BOMBCHU, GID_BOMBCHU, 0.5, Action::BowlPrize, 0.0),
    (EXITEM_BOMBS_BOWLING, OBJECT_GI_BOMB_1, GID_BOMB, GID_BOMB, 0.5, Action::BowlPrize, 0.0),
    (EXITEM_PURPLE_RUPEE_BOWLING, OBJECT_GI_RUPY, GID_RUPEE_PURPLE, GID_RUPEE_PURPLE, 0.5, Action::BowlPrize, 0.0),
    (EXITEM_BOMB_BAG_COUNTER, OBJECT_GI_BOMBPOUCH, GID_BOMB_BAG_30, GID_BOMB_BAG_30, 0.5, Action::SetupBowlCounter, -18.0),
    (EXITEM_HEART_PIECE_COUNTER, OBJECT_GI_HEARTS, GID_HEART_PIECE, 0, 0.5, Action::SetupBowlCounter, -10.0),
    // The bombchus on the counter get no yOffset.
    (EXITEM_BOMBCHUS_COUNTER, OBJECT_GI_BOMB_2, GID_BOMBCHU, GID_BOMBCHU, 0.5, Action::SetupBowlCounter, 0.0),
    (EXITEM_BOMBS_COUNTER, OBJECT_GI_BOMB_1, GID_BOMB, GID_BOMB, 0.5, Action::SetupBowlCounter, 10.0),
    (EXITEM_PURPLE_RUPEE_COUNTER, OBJECT_GI_RUPY, GID_RUPEE_PURPLE, GID_RUPEE_PURPLE, 0.5, Action::SetupBowlCounter, 40.0),
    (EXITEM_GREEN_RUPEE_CHEST, OBJECT_GI_RUPY, GID_RUPEE_GREEN, GID_RUPEE_GREEN, 0.5, Action::ExitChest, 0.0),
    (EXITEM_BLUE_RUPEE_CHEST, OBJECT_GI_RUPY, GID_RUPEE_BLUE, GID_RUPEE_BLUE, 0.5, Action::ExitChest, 0.0),
    (EXITEM_RED_RUPEE_CHEST, OBJECT_GI_RUPY, GID_RUPEE_RED, GID_RUPEE_RED, 0.5, Action::ExitChest, 0.0),
    // EXITEM_13 has no case for its giDrawId: 0, GID_BOTTLE_EMPTY.
    (EXITEM_13, OBJECT_GI_RUPY, 0, 0, 0.5, Action::ExitChest, 0.0),
    (EXITEM_14, OBJECT_GI_RUPY, GID_RUPEE_PURPLE, GID_RUPEE_PURPLE, 0.5, Action::ExitChest, 0.0),
    (EXITEM_MAGIC_FIRE, OBJECT_GI_GODDESS, GID_DINS_FIRE, 0, 0.35, Action::FairyMagic, 0.0),
    (EXITEM_MAGIC_WIND, OBJECT_GI_GODDESS, GID_FARORES_WIND, 0, 0.35, Action::FairyMagic, 0.0),
    (EXITEM_MAGIC_DARK, OBJECT_GI_GODDESS, GID_NAYRUS_LOVE, 0, 0.35, Action::FairyMagic, 0.0),
    (EXITEM_BULLET_BAG, OBJECT_GI_DEKUPOUCH, GID_BULLET_BAG, GID_BULLET_BAG, 0.1, Action::TargetPrizeApproach, 0.0),
];

#[test]
fn each_type_waits_for_its_object_and_draws_its_get_item_model() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    for &(ty, object, draw_id, gi_draw_id, scale, action, y_offset) in TYPES {
        let h = spawn(&mut w, Some(object), ty);
        // EnExItem_Init: untargetable, its object found (not drawn yet), waiting for it.
        let it = get(&w, h);
        assert_eq!((it.ty, it.get_item_object_id, it.drawn, it.action_func), (ty, object, false, Action::WaitForObject), "type {ty}");
        assert_eq!(it.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
        assert!(drawn(&w, h).is_empty());
        // Its object in: shown and set up.
        idle(&mut w, 1);
        let it = get(&w, h);
        assert!(it.drawn, "type {ty}");
        assert_eq!((it.gi_draw_id, it.scale, it.action_func, it.actor.shape_y_offset), (gi_draw_id, scale, action, y_offset), "type {ty}");
        assert_eq!(it.draw_id(), Some(draw_id));
        // EnExItem_Draw: Actor_SetScale(this->scale) after Actor_Draw scaled the matrix: this
        // frame's draw at Actor_Init's 0.01, the next at its own.
        assert_eq!((it.render_state().scale, it.actor.scale), (Vec3::splat(0.01), Vec3::splat(scale)), "type {ty}");
        let meshes = drawn(&w, h);
        assert!(!meshes.is_empty() && meshes.iter().all(|m| m.starts_with(&format!("bake/GetItem/{draw_id:02X}/"))), "type {ty}: {meshes:?}");
        idle(&mut w, 1);
        if w.actors.exists(h) {
            assert_eq!(get(&w, h).render_state().scale, Vec3::splat(scale), "type {ty}");
        }
        w.actors.actor_mut(h).unwrap().kill();
        idle(&mut w, 1);
    }
}

#[test]
fn the_bowling_heart_piece_chimes_when_it_shows() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    let h = spawn(&mut w, Some(OBJECT_GI_HEARTS), EXITEM_HEART_PIECE_BOWLING);
    idle(&mut w, 1);
    assert_eq!(get(&w, h).action_func, Action::BowlPrize);
    assert_eq!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_SY_PIECE_OF_HEART), vec![w.audio.frames]);
}

#[test]
fn without_its_object_it_is_gone_at_init() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    // The woods load no object_gi_bombpouch: Object_GetSlot is negative, Actor_Kill.
    assert!(w.object_ctx.get_index(OBJECT_GI_BOMBPOUCH).is_none());
    let h = spawn(&mut w, None, EXITEM_BOMB_BAG_BOWLING);
    assert!(get(&w, h).actor.killed);
    idle(&mut w, 1);
    assert!(!w.actors.exists(h));
}

/// `EnExItem_BowlPrize`'s step towards the camera, from `view`'s eye and at as the actor reads
/// them (`play->view`): 5 along the view to a point 5 in front of the eye, its `at` `extra`
/// further in z.
fn bowl_step(pos: Vec3, eye: Vec3, at: Vec3, extra: f32) -> Vec3 {
    let t1 = at.x - eye.x;
    let t2 = at.y - eye.y;
    let t3 = at.z + extra - eye.z;
    let t4 = (t1 * t1 + t2 * t2 + t3 * t3).sqrt();
    let (t5, t6, t7) = ((t1 / t4) * 5.0, (t2 / t4) * 5.0, (t3 / t4) * 5.0);
    let d = Vec3::new(eye.x + t5 - pos.x, eye.y + t6 - pos.y, eye.z + t7 - pos.z);
    Vec3::new(pos.x + (d.x / t4) * 5.0, pos.y + (d.y / t4) * 5.0, pos.z + (d.z / t4) * 5.0)
}

#[test]
fn the_bowling_prize_rises_and_turns_then_goes() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    let h = spawn(&mut w, Some(OBJECT_GI_BOMBPOUCH), EXITEM_BOMB_BAG_BOWLING);
    // Frame 0: EnExItem_WaitForObject: timer 65, prizeRotateTimer 35.
    idle(&mut w, 1);
    assert_eq!((get(&w, h).timer, get(&w, h).prize_rotate_timer), (65, 35));
    // Frame k: the timers down first (65 - k, 35 - k), then: the turn by 0x1000 until, with
    // prizeRotateTimer out, it faces 0x9000 (k 41: 41 × 0x1000 is 0x9000 to 16 bits), then
    // Math_SmoothStepToS to 0 (scale 5, step 0x1000); the move while prizeRotateTimer runs (k 1
    // to 34, 250 past the view's at in z); at k 65, timer out: gone (no parent to tell).
    let mut rot: i16 = 0;
    let mut stopped = false;
    for k in 1..=65 {
        let (eye, at) = (w.view.eye, w.view.at);
        let before = get(&w, h).actor.world_pos;
        idle(&mut w, 1);
        if k == 65 {
            assert!(get(&w, h).actor.killed);
            break;
        }
        let it = get(&w, h);
        assert_eq!((it.timer, it.prize_rotate_timer), (65 - k, (35 - k).max(0)), "frame {k}");
        if !stopped {
            rot = rot.wrapping_add(0x1000);
            if k >= 35 && rot as u16 == 0x9000 {
                stopped = true;
                assert_eq!(k, 41);
            }
        } else {
            eng_math::smooth_step_to_s(&mut rot, 0, 5, 0x1000, 0);
        }
        assert_eq!((it.actor.shape_rot.y, it.stop_rotate != 0), (rot, stopped), "frame {k}");
        let want = if k <= 34 { bowl_step(before, eye, at, 250.0) } else { before };
        assert!((it.actor.world_pos - want).length() < 1e-3, "frame {k}: {} want {want}", it.actor.world_pos);
    }
}

#[test]
fn the_counters_items_turn_until_the_shopkeeper_takes_them() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    let h = spawn(&mut w, Some(OBJECT_GI_BOMB_1), EXITEM_BOMBS_COUNTER);
    // EnExItem_SetupBowlCounter: facing 0x4268; then EnExItem_BowlCounter turns 0x800 a frame.
    idle(&mut w, 2);
    let it = get(&w, h);
    assert_eq!((it.action_func, it.actor.world_rot.y, it.actor.shape_rot.y), (Action::BowlCounter, 0x4268, 0x4268));
    idle(&mut w, 3);
    assert_eq!(get(&w, h).actor.shape_rot.y, 0x4268 + 3 * 0x800);
    assert!(!get(&w, h).actor.killed);
    // killItem (En_Bom_Bowl_Man's): gone.
    get_mut(&mut w, h).kill_item = 1;
    idle(&mut w, 1);
    assert!(get(&w, h).actor.killed);
}

/// `EnExItem_ExitChest` from its first frame (`timer` 7, `velocity.y` 10 already set): the
/// timers down, spinning 0x1000 a frame, rising 10 × 1.5 a frame while `timer` runs (6 frames;
/// at its 1 `chestKillTimer` 20), then still; gone when that's out, 20 frames on.
fn exits_the_chest(w: &mut PlayState, h: ActorHandle) {
    let start = get(w, h).actor.world_pos;
    let rot = get(w, h).actor.shape_rot.y;
    for k in 1..=26i32 {
        idle(w, 1);
        let it = get(w, h);
        if k == 26 {
            assert!(it.actor.killed);
            break;
        }
        assert!(!it.actor.killed, "frame {k}");
        assert_eq!(it.actor.shape_rot.y, rot.wrapping_add((k * 0x1000) as i16), "frame {k}");
        assert_eq!(it.actor.world_pos.y, start.y + 15.0 * k.min(6) as f32, "frame {k}");
        assert_eq!(it.chest_kill_timer, if k < 6 { 0 } else { (26 - k) as i16 }.min(20), "frame {k}");
    }
}

#[test]
fn the_chests_rupee_and_key_rise_out_and_go() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    // A blue rupee: set up out of the chest when its object's in.
    let h = spawn(&mut w, Some(OBJECT_GI_RUPY), EXITEM_BLUE_RUPEE_CHEST);
    idle(&mut w, 1);
    let it = get(&w, h);
    assert_eq!((it.timer, it.actor.velocity.y, it.action_func), (7, 10.0, Action::ExitChest));
    exits_the_chest(&mut w, h);
    // The key: no object; out of the chest from its init (scale 0.05), drawn at once as
    // gItemDropDL with gDropKeySmallTex (Gfx_SetupDL_41Opa), facing the camera.
    let h = spawn(&mut w, None, EXITEM_SMALL_KEY_CHEST);
    let it = get(&w, h);
    assert_eq!((it.get_item_object_id, it.drawn, it.scale, it.timer, it.actor.velocity.y, it.action_func), (-1, true, 0.05, 7, 10.0, Action::ExitChest));
    assert_eq!(it.draw_id(), None);
    assert_eq!(drawn(&w, h), vec![format!("bake/{KEY_BAKE}")]);
    exits_the_chest(&mut w, h);
}

#[test]
fn the_great_fairys_magic_turns() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    let h = spawn(&mut w, Some(OBJECT_GI_GODDESS), EXITEM_MAGIC_WIND);
    idle(&mut w, 1);
    let r = get(&w, h).actor.shape_rot.y;
    idle(&mut w, 4);
    assert_eq!(get(&w, h).actor.shape_rot.y, r.wrapping_add(4 * 0x800));
}

/// The bullet bag spawned on its own, flown, offered, taken and its text read with A (every other
/// frame while a text is up): the get-item id Player was given, and the frames it was hidden at
/// and gone at (counted from its spawn).
fn bullet_bag(w: &mut PlayState) -> (i16, u32, u32) {
    let h = spawn(w, None, EXITEM_BULLET_BAG);
    let spawned = w.audio.frames;
    let mut prev = PadState::default();
    let (mut given, mut hidden, mut gone) = (0, None, None);
    let mut scale = 0.1f32;
    for _ in 0..600 {
        let pad = if w.message_state() != TEXT_STATE_NONE && prev.button & BTN_A == 0 { PadState { button: BTN_A, ..Default::default() } } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        let f = w.audio.frames - spawned;
        if w.player().get_item_id > 0 && given == 0 {
            given = w.player().get_item_id;
        }
        match w.actors.downcast::<EnExItem>(h) {
            Some(it) => {
                if it.action_func == Action::TargetPrizeApproach && f >= 2 {
                    // Math_ApproachF(&scale, 0.8, 0.1, 0.02).
                    eng_math::approach_f(&mut scale, 0.8, 0.1, 0.02);
                    assert_eq!(it.scale, scale, "frame {f}");
                }
                if hidden.is_none() && it.action_func == Action::TargetPrizeGive {
                    hidden = Some(f);
                    assert!(!it.drawn && it.actor.parent.is_none());
                    assert_eq!(w.player().cs_mode, 7);
                }
            }
            None => {
                gone = Some(f);
                break;
            }
        }
    }
    (given, hidden.expect("offered"), gone.expect("gone"))
}

#[test]
fn the_bullet_bag_flies_at_the_camera_and_is_given() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    // Spawned (frame 0), set up on its first update (frame 1: timer 80, prizeRotateTimer 35, scale
    // 0.1), its timer out at frame 81: hidden, Player PLAYER_CSACTION_7, its parent let go,
    // Actor_OfferGetItem(GI_BULLET_BAG_40: CUR_UPG_VALUE(UPG_BULLET_BAG) is 1) within 2000 and
    // 1000; taken; its text done: ITEMGETINF_1D, gone. Item_Give(ITEM_BULLET_BAG_40): the 40 bag,
    // full.
    let (given, hidden, gone) = bullet_bag(&mut w);
    assert_eq!((given, hidden), (GI_BULLET_BAG_40, 81));
    assert!(gone > hidden);
    assert!(w.save.get_item_get_inf(ITEMGETINF_1D));
    assert_eq!((w.save.cur_upg_value(UPG_BULLET_BAG), w.save.ammo(ITEM_SLINGSHOT)), (2, 40));
}

#[test]
fn with_the_40_bag_it_gives_the_50_bag() {
    let Some(a) = assets() else { return };
    // CUR_UPG_VALUE(UPG_BULLET_BAG) 2: EnExItem_TargetPrizeApproach's not 1, GI_BULLET_BAG_50;
    // EnExItem_TargetPrizeGive's 2, GI_BULLET_BAG_50 too.
    let mut w = woods(&a, |s| s.inventory_change_upgrade(UPG_BULLET_BAG, 2));
    let (given, _, _) = bullet_bag(&mut w);
    assert_eq!(given, GI_BULLET_BAG_50);
    assert_eq!((w.save.cur_upg_value(UPG_BULLET_BAG), w.save.ammo(ITEM_SLINGSHOT)), (3, 50));
}
