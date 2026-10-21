//! GAME-10 milestone 2 against the C: King Zora (`z_en_kz.c`).
//!
//! The placement (`ootx scene-info --scene spot07_scene`): the child layers 0 and 1, room 0:
//! `En_Kz` 0x0100 (path 1) at (628, 996, -1780), yaw 0 (facing +z, down into the Domain); path 1
//! runs from (628, 996, -1783) to (531, 996, -1783). Child Link enters at `ENTR_ZORAS_DOMAIN_1`
//! (spawn 1, room 0, behind the throne: the way from the fountain) with `fairy-magic`, the Domain's
//! intro seen (`EVENTCHKINF_A4`), and is put on the floor in front of the throne. Expected values
//! are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CLEFT, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_kz::*;
use oot_actors::player::Player;
use oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::NA_SE_SY_CORRECT_CHIME;
use oot_game::camera::{CAM_ID_MAIN, CAM_ID_SUB_FIRST};
use oot_game::item::{ITEM_BOTTLE_EMPTY, ITEM_BOTTLE_RUTOS_LETTER, QUEST_ZORA_SAPPHIRE, SLOT_BOTTLE_1};
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `EVENTCHKINF_A4` (`save.h`): Zora's Domain's intro (`gZorasDomainIntroCs`) seen.
const EVENTCHKINF_A4: u16 = 0xA4;
/// His placement and path 1's ends.
const KZ_HOME: Vec3 = Vec3::new(628.0, 996.0, -1780.0);
const PATH_LAST: Vec3 = Vec3::new(531.0, 996.0, -1783.0);

/// Link entered at the Domain's top (`ENTR_ZORAS_DOMAIN_1`, room 0) at 10:00 with `fairy-magic`,
/// its intro seen, `setup`, the sound log on; two frames on.
fn domain_top(a: &Arc<GameAssets>, adult: bool, setup: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_ZORAS_DOMAIN_1").expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    save.set_event_chk_inf(EVENTCHKINF_A4);
    setup(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert_eq!(w.room_ctx.cur.num, 0);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn press(w: &mut PlayState, b: u16) {
    w.tick_with(scripted_input(PadState::default(), PadState { button: b, ..Default::default() }));
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

fn king(w: &PlayState) -> Option<ActorHandle> {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnKz>(h).is_some_and(|k| !k.actor.killed))
}

fn kz(w: &PlayState) -> &EnKz {
    w.actors.downcast::<EnKz>(king(w).expect("King Zora")).unwrap()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// Through texts until `f` holds (at most `max` frames): A on a frame of its own whenever a text
/// waits, each text id recorded, `each` after every frame.
fn answer_until(w: &mut PlayState, max: usize, what: &str, texts: &mut Vec<u16>, f: impl Fn(&PlayState) -> bool, mut each: impl FnMut(&PlayState)) -> usize {
    let mut pressed = false;
    for n in 0..max {
        if f(w) {
            return n;
        }
        let st = w.message_state();
        let a = !pressed && matches!(st, TEXT_STATE_CHOICE | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_EVENT | TEXT_STATE_AWAITING_NEXT);
        let pad = PadState { button: if a { BTN_A } else { 0 }, ..Default::default() };
        w.tick_with(scripted_input(PadState::default(), pad));
        pressed = a;
        if w.message_state() != TEXT_STATE_NONE && w.msg_ctx.text_id != 0 && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        each(w);
    }
    panic!("{what}: not after {max} frames (msgMode {:#x}, text {:#x}, texts {texts:x?})", w.msg_ctx.msg_mode, w.msg_ctx.text_id);
}

/// Link on the floor in front of the throne, `dz` south of the king's home, facing him (-z).
fn before_the_throne(w: &mut PlayState, dz: f32) {
    let x = KZ_HOME.x;
    let z = KZ_HOME.z + dz;
    let (floor, _) = w.col.entity_raycast_down(Vec3::new(x, KZ_HOME.y - 20.0, z));
    assert!(floor > 800.0 && floor < KZ_HOME.y, "the floor below the throne at {floor}");
    w.place_player(Vec3::new(x, floor, z), i16::MIN);
    idle(w, 10);
}

#[test]
fn king_zora_s_init_by_the_story_and_age() {
    let Some(a) = assets() else { return };
    // EnKz_Init: scale 0.01, ATTENTION_RANGE_3, gKzIdleAnim, no shadow; a child's waits for the
    // letter (EnKz_PreMweepWait) where he's placed.
    let w = domain_top(&a, false, |_| {});
    let k = kz(&w);
    assert_eq!(k.action, Action::PreMweepWait);
    assert_eq!((k.actor.scale, k.actor.target_mode, k.waypoint), (Vec3::splat(0.01), 3, 0));
    assert!(k.skel.is("gKzIdleAnim"));
    assert_eq!(k.actor.shape_shadow_draw, oot_game::shadow::ShadowDraw::None);
    assert_eq!(k.actor.world_pos, KZ_HOME);
    // EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA: EnKz_SetMovedPos, path 1's last point.
    let w = domain_top(&a, false, |s| s.set_event_chk_inf(EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA));
    let k = kz(&w);
    assert_eq!((k.action, k.actor.world_pos, k.actor.home_pos), (Action::PreMweepWait, PATH_LAST, KZ_HOME));
}

#[test]
fn an_adult_s_king_zora_is_a_placeholder() {
    let Some(a) = assets() else { return };
    // LINK_IS_ADULT: his ice (Bg_Ice_Shelter 0x04FF, Phase 16) isn't ported: no En_Kz of his own.
    let w = domain_top(&a, true, |_| {});
    assert!(king(&w).is_none());
}

#[test]
fn his_texts_follow_the_letter_and_the_sapphire() {
    let Some(a) = assets() else { return };
    let mut w = domain_top(&a, false, |_| {});
    // EnKz_GetTextIdChild: 0x401A, Player asked for the letter (exchangeItemId).
    w.player_mut().exchange_item_id = 0;
    assert_eq!(get_text_id(&mut w), 0x401A);
    assert_eq!(player(&w).exchange_item_id, EXCH_ITEM_BOTTLE_RUTOS_LETTER);
    // Moved aside: 0x401C; with the Sapphire: 0x402B (before the letter's flag).
    w.save.set_event_chk_inf(EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA);
    assert_eq!(get_text_id(&mut w), 0x401C);
    w.save.inventory.quest_items |= 1 << QUEST_ZORA_SAPPHIRE;
    assert_eq!(get_text_id(&mut w), 0x402B);
    // The Zora Mask (PLAYER_MASK_ZORA 6): MaskReaction_GetTextId first, 0x718F.
    w.player_mut().current_mask = 6;
    assert_eq!(get_text_id(&mut w), 0x718F);
}

#[test]
fn he_talks_only_from_the_floor_in_front_of_him() {
    let Some(a) = assets() else { return };
    let mut w = domain_top(&a, false, |_| {});
    let h = king(&w).unwrap();
    // EnKz_UpdateTalking: under 265 from him he's untargetable.
    before_the_throne(&mut w, 200.0);
    assert_eq!(w.actors.actor(h).unwrap().flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_ne!(player(&w).target_actor, Some(h));
    // 300 in front: targetable, offering to talk within 340 of his home (0x401A).
    before_the_throne(&mut w, 300.0);
    let k = kz(&w);
    assert_ne!(k.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(k.actor.text_id, 0x401A);
    assert_eq!(player(&w).target_actor, Some(h));
    // Talked to without the letter: 0x401A stays (func_80A9CB18), Player's text too; read, idle
    // (TEXT_STATE_DONE: none of his).
    press(&mut w, BTN_A);
    let mut texts = Vec::new();
    answer_until(&mut w, 300, "0x401A read", &mut texts, |w| w.message_state() == TEXT_STATE_NONE && kz(w).talk_state == NPC_TALK_STATE_IDLE, |_| {});
    assert_eq!(texts.first(), Some(&0x401A));
    assert_eq!(kz(&w).action, Action::PreMweepWait);
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA));
}

#[test]
fn ruto_s_letter_shown_moves_him_aside_and_empties_the_bottle() {
    let Some(a) = assets() else { return };
    // Ruto's letter in the first bottle, on C-Left.
    let mut w = domain_top(&a, false, |s| {
        s.inventory.items[SLOT_BOTTLE_1] = ITEM_BOTTLE_RUTOS_LETTER;
        s.equip_item_on_c_left(ITEM_BOTTLE_RUTOS_LETTER);
    });
    let h = king(&w).unwrap();
    before_the_throne(&mut w, 300.0);
    assert_eq!(player(&w).target_actor, Some(h));
    assert_eq!(player(&w).exchange_item_id, EXCH_ITEM_BOTTLE_RUTOS_LETTER);
    // C-Left: Player_ActionHandler_13's exchange (EXCH_ITEM_BOTTLE_RUTOS_LETTER, the one asked
    // for): the talk accepted; func_80A9CB18: 0x401B, Player's text.
    press(&mut w, BTN_CLEFT);
    let mut n = 0;
    while kz(&w).actor.text_id != 0x401B {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 60, "the letter never reached him");
    }
    assert_eq!(player(&w).actor.text_id, 0x401B);
    assert_eq!(kz(&w).talk_state, NPC_TALK_STATE_TALKING);
    // 0x401B's boxes, then the message's own next, 0x401C (his textId still 0x401B); read
    // (TEXT_STATE_DONE, advanced): his action; EnKz_PreMweepWait: gKzMweepAnim, EnKz_SetupMweep:
    // his sub camera active, the main one waiting, Link held (PLAYER_CSACTION_8), speed 0.1. The
    // chime as 0x401B faded in.
    let mut texts = Vec::new();
    answer_until(&mut w, 3000, "the mweep", &mut texts, |w| kz(w).action == Action::Mweep, |_| {});
    assert_eq!(texts, vec![0x401B, 0x401C]);
    assert!(!sfx_frames(&w, NA_SE_SY_CORRECT_CHIME).is_empty());
    let k = kz(&w);
    assert!(k.skel.is("gKzMweepAnim"));
    assert_eq!(k.actor.speed_xz, 0.1);
    assert!(k.sub_cam_id >= CAM_ID_SUB_FIRST);
    assert_eq!((k.return_to_cam_id, w.active_cam_id), (CAM_ID_MAIN, k.sub_cam_id));
    let sub = k.sub_cam_id;
    idle(&mut w, 2);
    assert_eq!(player(&w).cs_mode, PLAYER_CSACTION_8);
    // EnKz_Mweep: waypoint 0 (3 from his home: within sqrt 10) at once, then 0.1 a frame west to
    // (531, -1783): 97 across, the camera on him (at his position 60 up, from his
    // home 100 down and 260 south). Then: standing, the bottle empty, on the last point, the flag.
    let mut frames = 0;
    let mut at_eye_ok = true;
    while kz(&w).action == Action::Mweep {
        idle(&mut w, 1);
        frames += 1;
        assert!(frames < 1200, "the mweep never ended at {:?}", kz(&w).actor.world_pos);
        let k = kz(&w);
        if k.action == Action::Mweep
            && let Some(c) = w.camera(sub)
        {
            let at = k.actor.world_pos + Vec3::new(0.0, 60.0, 0.0);
            at_eye_ok &= c.at.distance(at) < 1.0 && c.eye.distance(KZ_HOME + Vec3::new(0.0, -100.0, 260.0)) < 1.0;
        }
    }
    assert!(at_eye_ok, "the mweep's camera");
    // Actor_UpdatePos moves velocity times R_UPDATE_RATE / 2 (1.5) a frame: 0.15; 94 to within
    // sqrt 10 of the end, about 625 frames and the turn's.
    assert!((600..660).contains(&frames), "{frames} frames");
    assert!(!sfx_frames(&w, NA_SE_VO_KZ_MOVE).is_empty());
    let k = kz(&w);
    assert_eq!((k.action, k.actor.world_pos, k.actor.speed_xz, k.waypoint), (Action::StopMweep, PATH_LAST, 0.0, 0));
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA));
    assert_eq!(w.save.inventory.items[SLOT_BOTTLE_1], ITEM_BOTTLE_EMPTY);
    assert_eq!(w.save.equips.button_items[1], ITEM_BOTTLE_EMPTY);
    // EnKz_StopMweep: the main camera back, his cleared, Link let go (PLAYER_CSACTION_7); waiting.
    idle(&mut w, 2);
    assert_eq!(kz(&w).action, Action::Wait);
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
    assert!(w.camera(sub).is_none());
    assert_ne!(player(&w).cs_mode, PLAYER_CSACTION_8);
    // His text now: 0x401C.
    assert_eq!(get_text_id(&mut w), 0x401C);
}

#[test]
fn the_domain_s_bakes_and_lists_are_in_the_pack() {
    use oot_game::pack::keys;
    let Ok(pack) = oot_game::pack::GamePack::open_default() else { return };
    // The bakes: King Zora's eyes, the Zoras' eyes in both passes and their three effects, the
    // falls (the child's two, the adult's translucent ice).
    let mut bakes: Vec<String> = (0..3).map(bake_name).collect();
    bakes.extend((0..3).flat_map(|e| [false, true].map(|x| oot_actors::en_zo::bake_name(e, x))));
    bakes.extend([oot_actors::en_zo::BAKE_RIPPLE, oot_actors::en_zo::BAKE_BUBBLE, oot_actors::en_zo::BAKE_SPLASH].map(String::from));
    bakes.extend((0..2).flat_map(|t| [oot_actors::bg_spot07_taki::child_bake(t), oot_actors::bg_spot07_taki::adult_xlu_bake(t)]));
    for name in &bakes {
        let d: eng_gfx::DrawList = pack.assets.get(&keys::bake(name)).expect(name);
        assert!(d.triangle_count() > 0 && d.stats.unresolved_addresses.is_empty(), "{name}: {:x?}", d.stats.unresolved_addresses);
    }
    // The plain lists: the hive, the adult's opaque ice.
    for (file, dl) in [("gameplay_field_keep", "gFieldBeehiveDL"), ("object_spot07_object", "object_spot07_object_DL_001CF0"), ("object_spot07_object", "object_spot07_object_DL_003210")] {
        let d: eng_gfx::DrawList = pack.assets.get(&keys::mesh(file, dl)).expect(dl);
        assert!(d.triangle_count() > 0, "{dl}");
    }
}

#[test]
fn he_draws_his_eye_s_bake_and_his_focus_is_his_head() {
    let Some(a) = assets() else { return };
    let mut w = domain_top(&a, false, |_| {});
    let h = king(&w).unwrap();
    {
        let k = w.actors.downcast_mut::<EnKz>(h).unwrap();
        k.eye_idx = 2;
    }
    let mut a2 = w.actors.take(h).unwrap();
    a2.draw_update(&mut w);
    let k = a2.as_any().downcast_ref::<EnKz>().unwrap();
    let view = oot_game::play::ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
    let mut out = oot_game::play::DrawOut::default();
    k.draw(&k.render_state(), &w, &view, &mut out);
    assert_eq!(out.opa.len(), 1);
    assert_eq!(out.opa[0].mesh, eng_gfx::MeshKey::named(oot_game::pack::keys::bake(&bake_name(2))));
    // EnKz_PostLimbDraw: 2600 along his head (limb 11), above his throne.
    assert!(k.actor.focus_pos.y > KZ_HOME.y + 40.0, "{:?}", k.actor.focus_pos);
    w.actors.put_back(h, a2);
}
