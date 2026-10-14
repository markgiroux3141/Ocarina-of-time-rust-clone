//! GAME-08 milestone 2 against the C: the Lost Woods' Skull Kids (`z_en_skj.c`) and their needles
//! (`z_en_skjneedle.c`). The placements (`ootx scene-info --scene spot10`): child layers 0 and 1,
//! room 9: 0x03FF (type 0, the Skull Kid on the stump) at (-1132, 80, -128) yaw 8920, 0x17FF (type
//! 5, the small stump) at (-951, 20, 52); room 1: 0x1BFF (type 6, the memory game's stump) at
//! (1123, -140, 340) yaw 7646, 0x07FF (type 1) at (1265, -94, 420), 0x0BFF (type 2) at
//! (1183, -72, 476); the adult layers 2 and 3: 0xFFFF (type 63) in room 1 (two) and room 9.
//!
//! Child Link enters the Lost Woods (`ENTR_LOST_WOODS_0`, room 0) with `ocarina-songs` (the Fairy
//! Ocarina on C-Left, Saria's Song) and is put in room 9 or room 1 as the debug starts do
//! (`playthrough::deku_tree_room_start`). Expected values are worked out from the C in the
//! comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_CUP, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_item00::{EnItem00, ITEM00_RUPEE_ORANGE};
use oot_actors::en_skj::*;
use oot_actors::en_skjneedle::EnSkjneedle;
use oot_actors::player::{Player, STATE2_23, STATE2_25, STATE3_5};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_NPC, ACTORCAT_PROP, ActorHandle};
use oot_game::audio::ocarina::{OCARINA_PITCH_A4, OCARINA_PITCH_B4, OCARINA_PITCH_D4, OCARINA_PITCH_D5, OCARINA_PITCH_F4, OCARINA_PITCH_NONE, OCARINA_SONG_MEMORY_GAME, OCARINA_SONG_SARIAS};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_OCARINA_ERROR, NA_SE_SY_TRE_BOX_APPEAR};
use oot_game::collision_check::AC_HIT;
use oot_game::item::{ITEM_POACHERS_SAW, ITEM_POCKET_EGG, QUEST_HEART_PIECE_COUNT};
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `ITEM_MASK_SKULL` (`item.h`: 0x25).
const ITEM_MASK_SKULL: u8 = 0x25;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Room 9's placements.
const KID_POS: Vec3 = Vec3::new(-1132.0, 80.0, -128.0);
const KID_YAW: i16 = 8920;
const SMALL_STUMP_POS: Vec3 = Vec3::new(-951.0, 20.0, 52.0);
/// Room 1's.
const STUMP_POS: Vec3 = Vec3::new(1123.0, -140.0, 340.0);
const LEFT_POS: Vec3 = Vec3::new(1265.0, -94.0, 420.0);
const RIGHT_POS: Vec3 = Vec3::new(1183.0, -72.0, 476.0);

/// Link entered at the Lost Woods (`ENTR_LOST_WOODS_0`) at 10:00 with `ocarina-songs` and
/// `setup`, the sound log and the audio side on (the ocarina counts its tasks).
fn woods(a: &Arc<GameAssets>, adult: bool, setup: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LOST_WOODS_0").expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("ocarina-songs").unwrap();
    setup(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    idle(&mut w, 2);
    w
}

/// Room `room` loaded and Link put at `pos` facing `yaw`, `n` frames run.
fn in_room(w: &mut PlayState, room: i8, pos: Vec3, yaw: i16, n: usize) {
    oot_actors::playthrough::deku_tree_room_start(w, room, pos, yaw);
    idle(w, n);
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// `b` pressed and held for `frames` frames, then let go.
fn press(w: &mut PlayState, b: u16, frames: usize) {
    let pad = PadState { button: b, ..Default::default() };
    for i in 0..frames {
        w.tick_with(scripted_input(if i == 0 { PadState::default() } else { pad }, pad));
    }
    w.tick_with(scripted_input(pad, PadState::default()));
}

/// Plays `buttons`, each held 3 frames and let go 2 (as `tests/ocarina.rs`).
fn play_notes(w: &mut PlayState, buttons: &[u16]) {
    for &b in buttons {
        press(w, b, 3);
        idle(w, 1);
    }
}

/// The pad button of an `OCARINA_BTN_*` (`sOcarinaButtonsToPad`: A, C-Down, C-Right, C-Left,
/// C-Up).
fn pad_button(b: u8) -> u16 {
    [BTN_A, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT, BTN_CUP][b as usize]
}

/// `song`'s buttons from the pack's `gOcarinaSongButtons`.
fn song(w: &PlayState, song: u8) -> Vec<u16> {
    let b = w.audio.ocarina.song_buttons(song);
    b.buttons_index[..b.num_buttons as usize].iter().map(|&i| pad_button(i)).collect()
}

/// The memory game's notes so far (`sOcarinaSongNotes[OCARINA_SONG_MEMORY_GAME]` up to
/// `OCARINA_PITCH_NONE`) as the pad's buttons: the Fairy Ocarina's pitches (`sOcarinaButtonsToPitch`
/// without a bend: A D4, C-Down F4, C-Right A4, C-Left B4, C-Up D5).
fn memory_game_notes(w: &PlayState) -> Vec<u16> {
    let mut v = Vec::new();
    for i in 0..20 {
        let n = w.audio.ocarina.song_note(OCARINA_SONG_MEMORY_GAME, i);
        let b = match n.pitch {
            OCARINA_PITCH_NONE => break,
            OCARINA_PITCH_D4 => BTN_A,
            OCARINA_PITCH_F4 => BTN_CDOWN,
            OCARINA_PITCH_A4 => BTN_CRIGHT,
            OCARINA_PITCH_B4 => BTN_CLEFT,
            OCARINA_PITCH_D5 => BTN_CUP,
            p => panic!("the memory game's note {i}: pitch {p:#x}"),
        };
        v.push(b);
    }
    v
}

/// Frames until `f` holds (at most `max`); panics with `what` if it never does.
fn until(w: &mut PlayState, max: usize, what: &str, f: impl Fn(&PlayState) -> bool) -> usize {
    for n in 0..max {
        if f(w) {
            return n;
        }
        idle(w, 1);
    }
    panic!("{what}: not after {max} frames (msgMode {:#x}, ocarinaMode {}, text {:#x})", w.msg_ctx.msg_mode, w.msg_ctx.ocarina_mode, w.msg_ctx.text_id);
}

/// Through texts until `f` holds (at most `max` frames): A on a frame of its own whenever a text
/// waits (done, an event, a choice: its first, unless `choice` says), each text id recorded.
fn answer_until(w: &mut PlayState, max: usize, what: &str, texts: &mut Vec<u16>, choice: u8, f: impl Fn(&PlayState) -> bool) {
    let mut pressed = false;
    for _ in 0..max {
        if f(w) {
            return;
        }
        let st = w.message_state();
        if st == TEXT_STATE_CHOICE && choice != 0 && !pressed {
            w.msg_ctx.choice_index = choice;
        }
        let a = !pressed && matches!(st, TEXT_STATE_CHOICE | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_EVENT | TEXT_STATE_AWAITING_NEXT);
        let mut record = |w: &PlayState| {
            if w.message_state() != TEXT_STATE_NONE && w.msg_ctx.text_id != 0 && texts.last() != Some(&w.msg_ctx.text_id) {
                texts.push(w.msg_ctx.text_id);
            }
        };
        record(w);
        let pad = PadState { button: if a { BTN_A } else { 0 }, ..Default::default() };
        w.tick_with(scripted_input(PadState::default(), pad));
        pressed = a;
        record(w);
    }
    panic!("{what}: not after {max} frames (msgMode {:#x}, text {:#x}, texts {texts:x?})", w.msg_ctx.msg_mode, w.msg_ctx.text_id);
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

fn skj(w: &PlayState, h: ActorHandle) -> &EnSkj {
    w.actors.downcast::<EnSkj>(h).expect("En_Skj")
}

fn skj_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnSkj {
    w.actors.downcast_mut::<EnSkj>(h).expect("En_Skj")
}

/// The En_Skj placed at `home` (live or killed, until deleted).
fn at(w: &PlayState, home: Vec3) -> Option<ActorHandle> {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnSkj>(h).is_some_and(|e| e.actor.home_pos == home))
}

fn statics(w: &PlayState) -> Option<&Statics> {
    w.overlay_statics.get(&ACTOR_EN_SKJ).and_then(|s| s.downcast_ref::<Statics>())
}

fn needles(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnSkjneedle>(h).is_some_and(|n| !n.actor.killed)).collect()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// The pack has `En_Skj`'s bakes and the lists its draws name (`gSkullKidSkullMaskDL`,
/// `gSkullKidNeedleDL`).
#[test]
fn the_pack_has_the_skull_kids_meshes() {
    let Some(a) = assets() else { return };
    for b in [bake_name(false), bake_name(true)] {
        assert!(a.pack.bake(&b).is_ok(), "{b}");
    }
    for dl in ["gSkullKidSkullMaskDL", "gSkullKidNeedleDL"] {
        assert!(a.pack.mesh(OBJECT, dl).is_ok(), "{dl}");
    }
}

#[test]
fn each_placement_initialises_by_its_type() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, false, |_| {});
    // Link on the small stump (room 9).
    in_room(&mut w, 9, SMALL_STUMP_POS, 0, 2);
    let kid = at(&w, KID_POS).expect("type 0");
    let small = at(&w, SMALL_STUMP_POS).expect("type 5");
    let k = skj(&w, kid);
    // EnSkj_Init's default: params = type (0); types 0..2: the flags' ATTENTION_ENABLED |
    // FRIENDLY instead of HOSTILE, ACTORCAT_NPC; type 0 < 7 keeps UPDATE_DURING_OCARINA.
    assert_eq!(k.actor.params, 0);
    assert_eq!(k.actor.category, ACTORCAT_NPC);
    let mask = ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA;
    assert_eq!(k.actor.flags & mask, ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA);
    // EnSkj_SetNaviId: type 0 without ITEMGETINF_16 or _39: NAVI_ENEMY_SKULL_KID.
    assert_eq!(k.actor.navi_enemy_id, NAVI_ENEMY_SKULL_KID);
    // sInitChain: ATTENTION_RANGE_2, lockOnArrowOffset 30; health 10, scale 0.01, gravity -1,
    // alpha 255, needlesToShoot and hitsUntilDodge 3; D_80B01678: 8 by 48.
    assert_eq!((k.actor.target_mode, k.actor.target_arrow_offset), (2, 30.0));
    assert_eq!((k.actor.col_chk_info.health, k.actor.scale.x, k.actor.gravity, k.alpha), (10, 0.01, -1.0, 255));
    assert_eq!((k.needles_to_shoot, k.hits_until_dodge), (3, 3));
    assert_eq!((k.collider.dim.radius, k.collider.dim.height), (8, 48));
    // EnSkj_CalculateCenter: 120 along his yaw (8920).
    let r = std::f64::consts::PI * (KID_YAW as f64 / 32768.0);
    let c = KID_POS + Vec3::new((120.0 * r.sin()) as f32, -KID_POS.y, (120.0 * r.cos()) as f32);
    assert!((k.center - c).length() < 0.01, "{:?} vs {c:?}", k.center);
    // EnSkj_SetupResetFight: watching 0xAAA ahead, 200 around (Link on the small stump is 255
    // away: he stays, and waits for him: EnSkj_SetupWaitInRange's 0x10BC).
    assert_eq!((k.unk_2c8, k.unk_2ec), (0xAAA, 200.0));
    assert_eq!((k.func, k.text_id, k.anim_index), (Func::WaitInRange, 0x10BC, SKJ_ANIM_WAIT));
    // Type 5: no draw, EnSkj_SariasSongShortStumpUpdate, ACTORCAT_PROP, neither targetable nor
    // hostile, its params as placed; it's sSmallStumpSkullKid.
    let s = skj(&w, small);
    assert_eq!((s.has_draw, s.update_fn, s.actor.category, s.func), (false, UpdateFn::SariasSongShortStump, ACTORCAT_PROP, Func::None));
    assert_eq!(s.actor.flags & mask, ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA);
    assert_eq!(s.actor.params as u16, 0x17FF);
    let st = statics(&w).expect("the statics");
    assert_eq!((st.s_small_stump_skull_kid.unk_0, st.s_small_stump_skull_kid.skullkid), (1, Some(small)));

    // Room 1, Link 300 from the stump (not on it).
    in_room(&mut w, 1, STUMP_POS + Vec3::new(0.0, 0.0, -300.0), 0, 0);
    let stump = at(&w, STUMP_POS).expect("type 6");
    let (left, right) = (at(&w, LEFT_POS).expect("type 1"), at(&w, RIGHT_POS).expect("type 2"));
    let s = skj(&w, stump);
    // Type 6: no draw, EnSkj_OcarinaMinigameShortStumpUpdate, a prop, its focus (1230, -90, 450),
    // EnSkj_SetupWaitForOcarina; it's sSmallStumpSkullKid now.
    assert_eq!((s.has_draw, s.update_fn, s.actor.category, s.func), (false, UpdateFn::OcarinaMinigameShortStump, ACTORCAT_PROP, Func::SetupWaitForOcarina));
    assert_eq!(s.actor.flags & mask, ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA);
    assert_eq!(s.actor.focus_pos, Vec3::new(1230.0, -90.0, 450.0));
    assert_eq!(s.actor.params as u16, 0x1BFF);
    // Types 1 and 2: ATTENTION_RANGE_7, alpha 0, minigameState 0, posCopy; not targetable until
    // Link's on the stump (EnSkj_OcarinaGameSetupWaitForPlayer); NAVI_ENEMY_SKULL_KID.
    for (h, ty, pos) in [(left, 1, LEFT_POS), (right, 2, RIGHT_POS)] {
        let k = skj(&w, h);
        assert_eq!((k.actor.params, k.actor.category, k.actor.target_mode, k.alpha, k.minigame_state), (ty, ACTORCAT_NPC, 7, 0, 0));
        assert_eq!(k.pos_copy, pos);
        assert_eq!(k.actor.flags & mask, ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA);
        assert_eq!((k.func, k.action, k.unk_2d3), (Func::OcarinaGameWaitForPlayer, SKJ_ACTION_OCARINA_GAME_WAIT_FOR_PLAYER, 0));
        assert_eq!(k.actor.navi_enemy_id, NAVI_ENEMY_SKULL_KID);
    }
    idle(&mut w, 1);
    let st = statics(&w).expect("the statics");
    assert_eq!(st.s_small_stump_skull_kid.skullkid, Some(stump));
    assert_eq!(st.s_ocarina_minigame_skull_kids.map(|k| (k.unk_0, k.skullkid)), [(1, Some(left)), (1, Some(right))]);
    // Room 9's went with their room: the statics are room 1's.
    assert!(at(&w, KID_POS).is_none());
}

#[test]
fn the_adults_skull_kids_go_without_the_poachers_saw() {
    let Some(a) = assets() else { return };
    // Adult layer 2, room 1: two 0xFFFF (type 63). The Pocket Egg in the adult trade slot
    // (INV_CONTENT(ITEM_TRADE_ADULT) < ITEM_POACHERS_SAW): both killed at init.
    let mut w = woods(&a, true, |s| s.set_inv_content(ITEM_POCKET_EGG, ITEM_POCKET_EGG));
    in_room(&mut w, 1, STUMP_POS + Vec3::new(0.0, 0.0, -300.0), 0, 0);
    for pos in [LEFT_POS, RIGHT_POS] {
        let h = at(&w, pos);
        assert!(h.is_none_or(|h| skj(&w, h).actor.killed), "{pos:?}");
    }
    // The Poacher's Saw: they stay, adults' (type 63 >= 7: no UPDATE_DURING_OCARINA), enemies,
    // NAVI_ENEMY_SKULL_KID_ADULT, on their flutes.
    let mut w = woods(&a, true, |s| s.set_inv_content(ITEM_POCKET_EGG, ITEM_POACHERS_SAW));
    in_room(&mut w, 1, STUMP_POS + Vec3::new(0.0, 0.0, -300.0), 0, 0);
    for pos in [LEFT_POS, RIGHT_POS] {
        let k = skj(&w, at(&w, pos).expect("type 63"));
        assert!(!k.actor.killed);
        assert_eq!((k.actor.params, k.actor.category, k.actor.navi_enemy_id), (63, ACTORCAT_ENEMY, NAVI_ENEMY_SKULL_KID_ADULT));
        assert_eq!(k.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_DURING_OCARINA), ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE);
        assert_eq!(k.alpha, 255);
    }
    // An empty slot (ITEM_NONE, 0xFF) isn't below the saw (a u8 compare): they stay too.
    let mut w = woods(&a, true, |s| s.set_inv_content(ITEM_POCKET_EGG, oot_game::item::ITEM_NONE));
    in_room(&mut w, 1, STUMP_POS + Vec3::new(0.0, 0.0, -300.0), 0, 0);
    assert!(!skj(&w, at(&w, LEFT_POS).expect("type 63")).actor.killed);
}

#[test]
fn the_skull_kid_flees_within_200_before_sarias_song() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, false, |_| {});
    in_room(&mut w, 9, SMALL_STUMP_POS, 0, 2);
    let kid = at(&w, KID_POS).expect("type 0");
    // 255 away on the small stump: he stays. Link 190 from him (towards the small stump):
    // EnSkj_SariasSongKidIdle's !ITEMGETINF_16 && xzDistToPlayer < 200: EnSkj_Backflip.
    in_room(&mut w, 9, SMALL_STUMP_POS, 0, 0);
    skj_mut(&mut w, kid).func = Func::SariasSongKidIdle;
    let d = (SMALL_STUMP_POS - KID_POS) * Vec3::new(1.0, 0.0, 1.0);
    let pos = KID_POS + d.normalize() * 190.0;
    w.place_player(Vec3::new(pos.x, 40.0, pos.z), 0);
    idle(&mut w, 1);
    let k = skj(&w, kid);
    assert!(k.actor.xz_dist_to_player < 200.0, "{}", k.actor.xz_dist_to_player);
    // EnSkj_Backflip: 8 up (less the frame's gravity, 1), back at 8; the backflip; FADE (unk_2D3 0).
    assert_eq!((k.func, k.backflip_flag, k.anim_index, k.unk_2d3), (Func::Fade, 1, SKJ_ANIM_BACKFLIP, 0));
    assert_eq!((k.actor.velocity.y, k.actor.speed_xz), (7.0, -8.0));
    // EnSkj_Fade: alpha 20 less a frame (and the shadow's); below 0 it's 0; the next update kills
    // him (backflipFlag with alpha 0), with no drop (not SKJ_ACTION_SPAWN_DEATH_EFFECT).
    let mut alphas = Vec::new();
    for _ in 0..14 {
        idle(&mut w, 1);
        let k = skj(&w, kid);
        alphas.push((k.alpha, k.shadow_alpha));
        if k.actor.killed {
            break;
        }
    }
    let expect: Vec<(u32, u8)> = (1..=12).map(|n| (255 - 20 * n, (255 - 20 * n) as u8)).chain([(0, 0), (0, 0)]).collect();
    assert_eq!(alphas, expect);
    assert!(skj(&w, kid).actor.killed);
    idle(&mut w, 2);
    assert!(at(&w, KID_POS).is_none());
    assert!(w.actors.all().iter().all(|&h| w.actors.downcast::<EnItem00>(h).is_none()), "no drop");
}

/// Link on the small stump with the ocarina out (C-Left), the staff up.
fn ocarina_out(w: &mut PlayState) {
    press(w, BTN_CLEFT, 1);
    until(w, 60, "the staff", |w| w.msg_ctx.msg_mode == MSGMODE_OCARINA_PLAYING);
}

#[test]
fn sarias_song_on_the_small_stump_gives_a_piece_of_heart() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, false, |_| {});
    in_room(&mut w, 9, SMALL_STUMP_POS + Vec3::new(0.0, 10.0, 0.0), 0x4000, 4);
    let kid = at(&w, KID_POS).expect("type 0");
    let small = at(&w, SMALL_STUMP_POS).expect("type 5");
    // EnSkj_RangeCheck holds: EnSkj_SetupWaitInRange, and each frame PLAYER_STATE2_23.
    assert_eq!(skj(&w, kid).func, Func::WaitInRange);
    assert_ne!(player(&w).state2 & STATE2_23, 0);
    // Not talked to before the song (no offer without ITEMGETINF_16).
    assert_ne!(player(&w).target_actor, Some(kid));
    // The ocarina out: Player_Action_8084E3C4 sees PLAYER_STATE2_23 and sets _24; EnSkj_WaitInRange:
    // PLAYER_STATE2_25, unk_6A8 the small stump, Link at its position, turned towards its facing,
    // Message_StartOcarinaSunsSongDisabled(OCARINA_ACTION_CHECK_SARIA); EnSkj_WaitForSong.
    ocarina_out(&mut w);
    let k = skj(&w, kid);
    assert_eq!(k.func, Func::WaitForSong);
    assert_eq!((w.msg_ctx.ocarina_action, w.msg_ctx.disable_suns_song), (OCARINA_ACTION_CHECK_SARIA, 1));
    let p = player(&w);
    assert_ne!(p.state2 & STATE2_25, 0);
    assert_eq!(p.unk_6a8, Some(small));
    assert_eq!((p.actor.world_pos.x, p.actor.world_pos.z), (SMALL_STUMP_POS.x, SMALL_STUMP_POS.z));
    // Saria's Song: MSGMODE_OCARINA_CORRECT_PLAYBACK: he plays along (unk_2D6 1, the flute), then
    // OCARINA_MODE_03 without ITEMGETINF_16: the chime, ocarinaMode 04, his text 0x10BB offered
    // (EnSkj_SetupAfterSong).
    let notes = song(&w, OCARINA_SONG_SARIAS);
    play_notes(&mut w, &notes);
    let mut played_along = false;
    for _ in 0..300 {
        let k = skj(&w, kid);
        played_along |= k.unk_2d6 == 1 && k.anim_index == SKJ_ANIM_PLAY_FLUTE;
        if k.func != Func::WaitForSong {
            break;
        }
        idle(&mut w, 1);
    }
    assert!(played_along, "he plays along");
    let k = skj(&w, kid);
    assert_eq!((k.func, k.text_id, k.actor.text_id), (Func::AfterSong, 0x10BB, 0x10BB));
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    assert!(!sfx_frames(&w, NA_SE_SY_CORRECT_CHIME).is_empty());
    let pieces = w.save.inventory.quest_items >> QUEST_HEART_PIECE_COUNT;
    // Player's ocarina done (OCARINA_MODE_04) with his offer (targetActor == unk_6A8): the talk.
    // 0x10BB read: the piece of heart (GI_HEART_PIECE) offered within the stump's reach, taken;
    // its text read: ITEMGETINF_16, NAVI_ENEMY_SKULL_KID_FRIENDLY, waiting again.
    let mut texts = Vec::new();
    answer_until(&mut w, 1500, "ITEMGETINF_16", &mut texts, 0, |w| w.save.get_item_get_inf(ITEMGETINF_16));
    // ("You played Saria's Song", 0x893 + 6, his 0x10BB, the piece of heart's 0xC2.)
    assert_eq!(texts, [0x899, 0x10BB, 0xC2]);
    assert_eq!(w.save.inventory.quest_items >> QUEST_HEART_PIECE_COUNT, pieces + 1);
    let k = skj(&w, kid);
    assert_eq!((k.func, k.text_id), (Func::WaitInRange, 0x10BC));
    assert_eq!(k.actor.navi_enemy_id, NAVI_ENEMY_SKULL_KID_FRIENDLY);
    // Then (ITEMGETINF_16, no mask) he offers 0x10BC from the stump.
    answer_until(&mut w, 300, "the box closed", &mut texts, 0, |w| w.message_state() == TEXT_STATE_NONE);
    idle(&mut w, 3);
    assert_eq!(player(&w).target_actor, Some(kid));
    assert_eq!(w.actors.actor(kid).unwrap().text_id, 0x10BC);
    // Saria's Song again: OCARINA_MODE_03 with ITEMGETINF_16 is ocarinaMode 05, then (>= 05)
    // sunsSongState SUNSSONG_INACTIVE and 0x10BD.
    until(&mut w, 60, "Link free", |w| player(w).state2 & STATE2_25 == 0);
    ocarina_out(&mut w);
    assert_eq!(skj(&w, kid).func, Func::WaitForSong);
    play_notes(&mut w, &notes);
    until(&mut w, 300, "0x10BD", |w| skj(w, kid).func != Func::WaitForSong);
    assert_eq!((skj(&w, kid).func, skj(&w, kid).text_id), (Func::AfterSong, 0x10BD));
    assert_eq!(w.save.suns_song_state, oot_game::clock::SUNSSONG_INACTIVE);
}

#[test]
fn the_skull_mask_trade() {
    let Some(a) = assets() else { return };
    // Masks aren't ported: the Skull Mask forced on (Player's currentMask PLAYER_MASK_SKULL), after
    // Saria's Song.
    // The Skull Mask (ITEM_MASK_SKULL, item.h: 0x25) in the child trade slot and on C-Down
    // (Player_ProcessItemButtons takes off a mask no C button has).
    let mut w = woods(&a, false, |s| {
        s.set_item_get_inf(ITEMGETINF_16);
        s.set_inv_content(ITEM_MASK_SKULL, ITEM_MASK_SKULL);
        s.equip_item_on_c(1, ITEM_MASK_SKULL);
    });
    in_room(&mut w, 9, SMALL_STUMP_POS + Vec3::new(0.0, 10.0, 0.0), 0x4000, 4);
    let kid = at(&w, KID_POS).expect("type 0");
    w.player_mut().current_mask = PLAYER_MASK_SKULL;
    idle(&mut w, 2);
    // EnSkj_WaitInRange: the Skull Mask worn: 0x101B offered.
    assert_eq!(skj(&w, kid).text_id, 0x101B);
    assert_eq!(player(&w).target_actor, Some(kid));
    // A: D_80B01EA0 with the mask and not ITEMGETINF_39: NA_SE_SY_TRE_BOX_APPEAR, the trade
    // (EnSkj_StartMaskTrade: Link held, PLAYER_CSACTION_1).
    w.tick_with(scripted_input(PadState::default(), PadState { button: BTN_A, ..Default::default() }));
    idle(&mut w, 2);
    assert_eq!(skj(&w, kid).func, Func::StartMaskTrade);
    assert!(!sfx_frames(&w, NA_SE_SY_TRE_BOX_APPEAR).is_empty());
    assert_eq!(player(&w).cs_mode, 1);
    // 0x101B read: he jumps down (EnSkj_JumpFromStump), lands, walks to within 120 of Link, and
    // asks (0x101C); yes: 0x101E, then 10 rupees, ITEMGETINF_39 (NAVI_ENEMY_SKULL_KID_MASK), the
    // mask off, 0x101F; read, Link let go (PLAYER_CSACTION_7) and he backflips away.
    let rupees = w.save.rupees + w.save.rupee_accumulator;
    let mut texts = Vec::new();
    answer_until(&mut w, 2000, "ITEMGETINF_39", &mut texts, 0, |w| w.save.get_item_get_inf(ITEMGETINF_39));
    assert_eq!(skj(&w, kid).actor.navi_enemy_id, NAVI_ENEMY_SKULL_KID_MASK);
    assert_eq!(player(&w).current_mask, PLAYER_MASK_NONE);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, rupees + 10);
    answer_until(&mut w, 600, "the backflip", &mut texts, 0, |w| at(w, KID_POS).is_none_or(|h| skj(w, h).func == Func::Fade));
    assert_eq!(texts, [0x101B, 0x101C, 0x101E, 0x101F]);
    assert_eq!(player(&w).cs_mode, 7);
}

/// Link on the memory game's stump, the Skull Kids there.
fn on_the_stump(a: &Arc<GameAssets>, setup: impl FnOnce(&mut SaveContext)) -> (PlayState, ActorHandle, ActorHandle, ActorHandle) {
    let mut w = woods(a, false, setup);
    in_room(&mut w, 1, STUMP_POS + Vec3::new(0.0, 10.0, 0.0), 0, 4);
    let stump = at(&w, STUMP_POS).expect("type 6");
    let (left, right) = (at(&w, LEFT_POS).expect("type 1"), at(&w, RIGHT_POS).expect("type 2"));
    (w, stump, left, right)
}

/// The ocarina out on the stump, 0x10BE read: the memory game's first notes.
fn start_the_game(w: &mut PlayState, stump: ActorHandle) {
    press(w, BTN_CLEFT, 1);
    until(w, 60, "0x10BE", |w| w.msg_ctx.text_id == 0x10BE);
    let mut texts = Vec::new();
    answer_until(w, 300, "the memory game", &mut texts, 0, |w| skj(w, stump).func == Func::WaitForPlayback);
}

/// One turn: the Skull Kids' notes, then Link's turn (`MSGMODE_MEMORY_GAME_PLAYER_PLAYING`);
/// the msgModes seen on the way.
fn to_links_turn(w: &mut PlayState) -> Vec<u8> {
    let mut seen = vec![w.msg_ctx.msg_mode];
    for _ in 0..1200 {
        if w.msg_ctx.msg_mode == MSGMODE_MEMORY_GAME_PLAYER_PLAYING {
            return seen;
        }
        idle(w, 1);
        if seen.last() != Some(&w.msg_ctx.msg_mode) {
            seen.push(w.msg_ctx.msg_mode);
        }
    }
    panic!("Link's turn never came: {seen:x?}");
}

#[test]
fn the_memory_game_starts_on_the_stump_with_the_ocarina() {
    let Some(a) = assets() else { return };
    let (mut w, stump, left, right) = on_the_stump(&a, |_| {});
    // EnSkj_SetupWaitForOcarina: Link in range: both Skull Kids' playerInRange; they're targetable
    // (EnSkj_OcarinaGameWaitForPlayer) and fade in, 20 a frame (EnSkj_Appear).
    for h in [left, right] {
        let k = skj(&w, h);
        assert_eq!((k.player_in_range, k.func), (1, Func::OcarinaGameIdle));
        assert_ne!(k.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
        assert!(k.alpha > 0 && k.alpha < 255 && k.alpha % 20 == 0, "{}", k.alpha);
    }
    assert_eq!(skj(&w, stump).func, Func::WaitForOcarina);
    // The ocarina out: PLAYER_STATE2_25, func_800F5BF0 (the nature ambience), 0x10BE
    // (EnSkj_StartOcarinaMinigame); its closing: Message_StartOcarina(OCARINA_ACTION_MEMORY_GAME),
    // the left Skull Kid's notes (minigameState SKULL_KID_OCARINA_PLAY_NOTES), songFailTimer 160.
    start_the_game(&mut w, stump);
    assert_eq!(w.msg_ctx.ocarina_action, OCARINA_ACTION_MEMORY_GAME);
    assert_eq!(skj(&w, left).minigame_state, SKULL_KID_OCARINA_PLAY_NOTES);
    assert_eq!(skj(&w, stump).multiuse_timer, 160);
    assert_eq!(player(&w).unk_6a8, Some(stump));
    // The left Skull Kid plays its notes (Message_DrawMain's MSGMODE_MEMORY_GAME_START: round 0,
    // three notes), waits; EnSkj_WaitForPlayback: once NA_SE_SY_METRONOME is done, the right one
    // plays (Message_UpdateOcarinaMemoryGame), waits; then Link's turn, songFailTimer 160.
    let seen = to_links_turn(&mut w);
    let order = [
        MSGMODE_MEMORY_GAME_LEFT_SKULLKID_PLAYING,
        MSGMODE_MEMORY_GAME_LEFT_SKULLKID_WAIT,
        MSGMODE_MEMORY_GAME_RIGHT_SKULLKID_PLAYING,
        MSGMODE_MEMORY_GAME_RIGHT_SKULLKID_WAIT,
        MSGMODE_MEMORY_GAME_PLAYER_PLAYING,
    ];
    let pos: Vec<usize> = order.iter().map(|m| seen.iter().position(|s| s == m).unwrap_or_else(|| panic!("{m:#x} not in {seen:x?}"))).collect();
    assert!(pos.windows(2).all(|p| p[0] < p[1]), "{seen:x?}");
    assert_eq!((skj(&w, left).minigame_state, skj(&w, right).minigame_state), (SKULL_KID_OCARINA_WAIT, SKULL_KID_OCARINA_WAIT));
    assert_eq!(memory_game_notes(&w).len(), 3);
    assert!(skj(&w, stump).multiuse_timer <= 160);
    // The Skull Kids play their flutes by minigameState (EnSkj_PlayOcarinaGame) and stop with it.
    assert_eq!(skj(&w, left).func, Func::OcarinaGameIdle);
}

#[test]
fn a_wrong_note_fails_the_memory_game_and_no_ends_it() {
    let Some(a) = assets() else { return };
    let (mut w, stump, left, right) = on_the_stump(&a, |_| {});
    start_the_game(&mut w, stump);
    to_links_turn(&mut w);
    // A note other than the first: the staff's 0xFF, NA_SE_SY_OCARINA_ERROR, OCARINA_MODE_03;
    // EnSkj_WaitForPlayback: the box closed, ocarinaMode 04, 0x102D offered (26).
    let first = memory_game_notes(&w)[0];
    let wrong = [BTN_A, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT, BTN_CUP].into_iter().find(|&b| b != first).unwrap();
    let pad = PadState { button: wrong, ..Default::default() };
    let mut funcs = vec![skj(&w, stump).func];
    for i in 0..60 {
        w.tick_with(scripted_input(if (1..4).contains(&i) { pad } else { PadState::default() }, if i < 3 { pad } else { PadState::default() }));
        if funcs.last() != Some(&skj(&w, stump).func) {
            funcs.push(skj(&w, stump).func);
        }
    }
    // (The next frame Player, its ocarina done with the stump's offer, talks: EnSkj_WaitForNextRound.)
    assert_eq!(funcs, [Func::WaitForPlayback, Func::FailedMiniGame, Func::WaitForNextRound]);
    assert_eq!(skj(&w, stump).text_id, 0x102D);
    assert!(!sfx_frames(&w, NA_SE_SY_OCARINA_ERROR).is_empty());
    // Player talks (the ocarina done, its offer): 0x102D; read: 0x102E's choice; "no":
    // EnSkj_CleanupOcarinaGame: the Skull Kids' unk_0 2, the music back (func_800F5C2C), the
    // stump killed; they leave (EnSkj_IsLeavingGame: the backflip, out of the statics, fading).
    let mut texts = Vec::new();
    answer_until(&mut w, 600, "the stump gone", &mut texts, 1, |w| at(w, STUMP_POS).is_none_or(|h| skj(w, h).actor.killed));
    assert_eq!(texts, [0x102D, 0x102E]);
    for h in [left, right] {
        until(&mut w, 30, "the Skull Kids leave", |w| w.actors.downcast::<EnSkj>(h).is_none_or(|k| k.func == Func::Fade));
    }
    let st = statics(&w).expect("the statics");
    assert_eq!(st.s_ocarina_minigame_skull_kids.map(|k| (k.unk_0, k.skullkid)), [(0, None), (0, None)]);
    assert_eq!(w.save.ocarina_game_round_num, 0);
}

#[test]
fn the_memory_game_times_out_after_160_frames() {
    let Some(a) = assets() else { return };
    let (mut w, stump, _, _) = on_the_stump(&a, |_| {});
    start_the_game(&mut w, stump);
    to_links_turn(&mut w);
    // MSGMODE_MEMORY_GAME_PLAYER_PLAYING: songFailTimer down from 160; at 0 the error sound and
    // 0x102D (EnSkj_FailedMiniGame).
    // (Link's turn began in the update that set songFailTimer 160: 160 updates count it down,
    // the 161st fails the game.)
    let n = until(&mut w, 200, "the timeout", |w| skj(w, stump).func != Func::WaitForPlayback);
    assert_eq!(n, 161);
    assert_eq!(skj(&w, stump).text_id, 0x102D);
    assert!(!sfx_frames(&w, NA_SE_SY_OCARINA_ERROR).is_empty());
}

/// Link's turns until the game's won: each turn the notes so far, then the next round's.
fn win_the_game(w: &mut PlayState, stump: ActorHandle) -> usize {
    let mut turns = 0;
    loop {
        to_links_turn(w);
        let notes = memory_game_notes(w);
        play_notes(w, &notes);
        turns += 1;
        // MSGMODE_MEMORY_GAME_ROUND_SUCCESS (30 frames), then the next note
        // (MSGMODE_MEMORY_GAME_START_NEXT_ROUND, which the stump turns into the left Skull Kid's
        // playing) or, with the round's notes all there, OCARINA_MODE_0F.
        until(w, 400, "the turn's end", |w| skj(w, stump).func != Func::WaitForPlayback || w.msg_ctx.msg_mode == MSGMODE_MEMORY_GAME_START_NEXT_ROUND || w.msg_ctx.msg_mode == MSGMODE_MEMORY_GAME_LEFT_SKULLKID_PLAYING);
        if skj(w, stump).func != Func::WaitForPlayback {
            return turns;
        }
        assert!(turns < 10, "the game never ends");
    }
}

#[test]
fn a_won_round_gives_its_reward_and_counts() {
    let Some(a) = assets() else { return };
    let (mut w, stump, _, _) = on_the_stump(&a, |_| {});
    start_the_game(&mut w, stump);
    // Round 0 (sOcaMemoryGameNumNotes[0], 5): three notes, then four, then five; the last right:
    // OCARINA_MODE_0F: the chime, 0x10BF (EnSkj_WonOcarinaMiniGame).
    let turns = win_the_game(&mut w, stump);
    assert_eq!(turns, 3);
    assert_eq!((skj(&w, stump).func, skj(&w, stump).text_id), (Func::WonOcarinaMiniGame, 0x10BF));
    assert_eq!(memory_game_notes(&w).len(), 5);
    // 0x10BF read: sOcarinaGameRewards[0] (GI_RUPEE_GREEN: one rupee), its text read: round 1,
    // and 0x102E again (EnSkj_OfferNextRound).
    let rupees = w.save.rupees + w.save.rupee_accumulator;
    let mut texts = Vec::new();
    answer_until(&mut w, 900, "0x102E", &mut texts, 0, |w| skj(w, stump).func == Func::WaitForOfferResponse);
    assert_eq!(w.save.rupees + w.save.rupee_accumulator, rupees + 1);
    assert_eq!(w.save.ocarina_game_round_num, 1);
    assert!(!w.save.get_item_get_inf(ITEMGETINF_17));
    assert!(texts.contains(&0x10BF), "{texts:x?}");
    // "Yes": PLAYER_STATE3_5 (the ocarina out as the box closes), EnSkj_SetupWaitForOcarina.
    answer_until(&mut w, 60, "again", &mut texts, 0, |w| skj(w, stump).func != Func::WaitForOfferResponse);
    assert_eq!(skj(&w, stump).func, Func::SetupWaitForOcarina);
    assert_ne!(player(&w).state3 & STATE3_5, 0);
}

#[test]
fn the_third_round_gives_a_piece_of_heart_and_ends_the_game() {
    let Some(a) = assets() else { return };
    // Two rounds won: sOcarinaGameRewards[2] (GI_HEART_PIECE), round 2's eight notes.
    let (mut w, stump, _, _) = on_the_stump(&a, |s| s.ocarina_game_round_num = 2);
    start_the_game(&mut w, stump);
    let turns = win_the_game(&mut w, stump);
    assert_eq!(turns, 6);
    assert_eq!(memory_game_notes(&w).len(), 8);
    let pieces = w.save.inventory.quest_items >> QUEST_HEART_PIECE_COUNT;
    // EnSkj_FinishOcarinaGameRound with round 2: round 3, ITEMGETINF_17, EnSkj_CleanupOcarinaGame.
    let mut texts = Vec::new();
    answer_until(&mut w, 900, "the stump gone", &mut texts, 0, |w| at(w, STUMP_POS).is_none_or(|h| skj(w, h).actor.killed));
    assert_eq!(w.save.inventory.quest_items >> QUEST_HEART_PIECE_COUNT, pieces + 1);
    assert_eq!(w.save.ocarina_game_round_num, 3);
    assert!(w.save.get_item_get_inf(ITEMGETINF_17));
}

/// The adult's Skull Kid (0xFFFF) spawned in room 9 at the stump's Skull Kid's place, facing Link
/// 150 in front of him; an empty adult trade slot keeps him.
fn adult_skull_kid(w: &mut PlayState) -> ActorHandle {
    in_room(w, 9, SMALL_STUMP_POS, 0, 1);
    let kid = at(w, KID_POS).expect("type 0");
    skj_mut(w, kid).actor.kill();
    idle(w, 2);
    let d = (SMALL_STUMP_POS - KID_POS) * Vec3::new(1.0, 0.0, 1.0);
    let yaw = oot_game::target::yaw_to(KID_POS, SMALL_STUMP_POS);
    let h = w.actor_spawn(ACTOR_EN_SKJ, KID_POS, [0, yaw, 0], -1).expect("En_Skj 0xFFFF");
    let pos = KID_POS + d.normalize() * 150.0;
    w.place_player(Vec3::new(pos.x, 40.0, pos.z), yaw.wrapping_add(-0x8000i32 as i16));
    h
}

#[test]
fn the_adults_skull_kid_fights_with_needles_and_dies_dropping_an_orange_rupee() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, false, |_| {});
    let h = adult_skull_kid(&mut w);
    // EnSkj_SariasSongKidIdle (params 63): Link in front (within 0xAAA: func_80AFEDF8):
    // func_80AFF334 (looking about, battleExitTimer 400); EnSkj_ResetFight: Link in front (0x2000):
    // battleExitTimer 600, EnSkj_SetupStand (needleShootTimer 60).
    until(&mut w, 10, "the fight", |w| skj(w, h).func == Func::Fight);
    let k = skj(&w, h);
    assert_eq!((k.anim_index, k.unk_2c8, k.unk_2ec), (SKJ_ANIM_FIGHTING_STANCE, 0x2000, 600.0));
    assert!(k.multiuse_timer > 590, "{}", k.multiuse_timer);
    // EnSkj_Fight: round his centre (120 from it) facing Link; at needleShootTimer 0 the volley:
    // at the shot's last frame, three En_Skjneedle 4 frames apart (killTimer 100, speed 24).
    let mut spawned: Vec<(usize, ActorHandle)> = Vec::new();
    for n in 0..200 {
        idle(&mut w, 1);
        for nh in needles(&w) {
            if !spawned.iter().any(|s| s.1 == nh) {
                let nd = w.actors.downcast::<EnSkjneedle>(nh).unwrap();
                assert_eq!((nd.kill_timer, nd.actor.speed_xz), (100, 24.0));
                spawned.push((n, nh));
            }
        }
        if spawned.len() == 3 {
            break;
        }
    }
    assert_eq!(spawned.len(), 3, "three needles");
    assert_eq!((spawned[1].0 - spawned[0].0, spawned[2].0 - spawned[1].0), (4, 4));
    until(&mut w, 40, "EnSkj_NeedleRecover", |w| skj(w, h).func == Func::NeedleRecover);
    // A sword hit (as CollisionCheck_AC leaves it: AC_HIT, the Kokiri sword's damage 1 and reaction
    // 0xF) in the fight: Actor_ApplyDamage (9), red, the hit (func_80AFF1F0), dodge timer 60.
    {
        let k = skj_mut(&mut w, h);
        k.collider.base.ac_flags |= AC_HIT;
        k.actor.col_chk_info.damage = 1;
        k.actor.col_chk_info.damage_reaction = 0xF;
    }
    idle(&mut w, 1);
    let k = skj(&w, h);
    assert_eq!((k.func, k.actor.col_chk_info.health, k.hits_until_dodge, k.dodge_reset_timer), (Func::PickNextFightAction, 9, 2, 60));
    assert_ne!(k.actor.color_filter_timer, 0);
    // Then a hit for the rest: health 0, EnSkj_SetupDie; the death animation, then fading 4 a frame
    // in blue flames; at 0 an orange rupee (ITEM00_RUPEE_ORANGE) where he was.
    until(&mut w, 60, "the stance", |w| skj(w, h).func == Func::Fight || skj(w, h).func == Func::WaitToShootNeedle);
    until(&mut w, 20, "the colour gone", |w| skj(w, h).actor.color_filter_timer == 0);
    {
        let k = skj_mut(&mut w, h);
        k.collider.base.ac_flags |= AC_HIT;
        k.actor.col_chk_info.damage = 9;
        k.actor.col_chk_info.damage_reaction = 0xF;
    }
    idle(&mut w, 1);
    assert_eq!((skj(&w, h).func, skj(&w, h).actor.col_chk_info.health), (Func::WaitForDeathAnim, 0));
    until(&mut w, 120, "EnSkj_SpawnDeathEffect", |w| skj(w, h).func == Func::SpawnDeathEffect);
    let n = until(&mut w, 120, "gone", |w| skj(w, h).actor.killed);
    // 255 / 4: 64 frames to alpha 0, the next update drops and kills.
    assert!((64..=66).contains(&n), "{n}");
    let pos = skj(&w, h).actor.world_pos;
    idle(&mut w, 1);
    let rupee = w.actors.all().into_iter().filter_map(|r| w.actors.downcast::<EnItem00>(r)).find(|r| r.actor.params & 0xFF == ITEM00_RUPEE_ORANGE).expect("the orange rupee");
    assert!((rupee.actor.home_pos - pos).length() < 1.0, "{:?} vs {pos:?}", rupee.actor.home_pos);
}

#[test]
fn a_hit_from_behind_kills_him_at_once() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, false, |_| {});
    let h = adult_skull_kid(&mut w);
    idle(&mut w, 1);
    // On his flute (SKJ_ACTION_SARIA_SONG_IDLE): a sword hit with Link behind him (over 0x6000
    // off his facing): EnSkj_SetupDie straight away, the health untouched.
    let k = skj_mut(&mut w, h);
    k.func = Func::SariasSongKidIdle;
    k.action = SKJ_ACTION_SARIA_SONG_IDLE;
    k.actor.world_rot.y = k.actor.yaw_towards_player.wrapping_add(-0x8000i32 as i16);
    k.actor.shape_rot.y = k.actor.world_rot.y;
    k.unk_2c8 = 0;
    k.unk_2ec = 0.0;
    k.collider.base.ac_flags |= AC_HIT;
    k.actor.col_chk_info.damage = 1;
    k.actor.col_chk_info.damage_reaction = 0xF;
    idle(&mut w, 1);
    assert_eq!((skj(&w, h).func, skj(&w, h).actor.col_chk_info.health), (Func::WaitForDeathAnim, 10));
}

#[test]
fn the_stump_skull_kid_hit_backflips_away() {
    let Some(a) = assets() else { return };
    // After Saria's Song he doesn't flee: Link 150 in front of him.
    let mut w = woods(&a, false, |s| s.set_item_get_inf(ITEMGETINF_16));
    in_room(&mut w, 9, SMALL_STUMP_POS, 0, 1);
    let kid = at(&w, KID_POS).expect("type 0");
    let d = (SMALL_STUMP_POS - KID_POS) * Vec3::new(1.0, 0.0, 1.0);
    let pos = KID_POS + d.normalize() * 150.0;
    w.place_player(Vec3::new(pos.x, 40.0, pos.z), 0);
    idle(&mut w, 2);
    assert_eq!(skj(&w, kid).func, Func::SariasSongKidIdle);
    // A sword hit from in front: the damage and the hit; after it the stance, and with
    // battleExitTimer 0 and Link in front, EnSkj_Fight's backflip away (backflipFlag: gone).
    {
        let k = skj_mut(&mut w, kid);
        k.collider.base.ac_flags |= AC_HIT;
        k.actor.col_chk_info.damage = 1;
        k.actor.col_chk_info.damage_reaction = 0xF;
    }
    idle(&mut w, 1);
    assert_eq!(skj(&w, kid).func, Func::PickNextFightAction);
    until(&mut w, 60, "the backflip", |w| skj(w, kid).func == Func::Fade);
    assert_eq!(skj(&w, kid).backflip_flag, 1);
    assert!(needles(&w).is_empty());
    until(&mut w, 30, "gone", |w| skj(w, kid).actor.killed);
}

#[test]
fn the_statics_go_with_the_last_skull_kid() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, false, |_| {});
    in_room(&mut w, 9, SMALL_STUMP_POS, 0, 2);
    assert!(statics(&w).is_some_and(|s| s.s_small_stump_skull_kid.skullkid.is_some()));
    // Back to room 0: room 9's En_Skj go; the overlay unloads (Actor_FreeOverlay at numLoaded 0)
    // and its statics with it.
    in_room(&mut w, 0, Vec3::new(-3.0, 0.0, 295.0), 0, 3);
    assert!(at(&w, KID_POS).is_none());
    assert!(statics(&w).is_none());
}

#[test]
fn a_needle_flies_and_goes() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, false, |_| {});
    // Link on the small stump facing the Skull Kid: a needle 100 in front of him is in view (it
    // has no ACTOR_FLAG_UPDATE_CULLING_DISABLED: out of view it doesn't update).
    let yaw = oot_game::target::yaw_to(SMALL_STUMP_POS, KID_POS);
    in_room(&mut w, 9, SMALL_STUMP_POS, yaw, 2);
    let dir = Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw));
    let at_pos = SMALL_STUMP_POS + dir * 100.0 + Vec3::Y * 27.0;
    // En_Skjneedle alone: killTimer 0 at init, so its first update kills it. (Spawned between
    // frames, it isn't in view until the next draw sets ACTOR_FLAG_INSIDE_CULLING_VOLUME: its
    // first update is the second frame's.)
    let h = w.actor_spawn(oot_actors::en_skjneedle::ACTOR_EN_SKJNEEDLE, at_pos, [0, yaw, 0], 0).expect("En_Skjneedle");
    let n = w.actors.downcast::<EnSkjneedle>(h).unwrap();
    assert_eq!((n.actor.target_mode, n.actor.target_arrow_offset, n.actor.scale.x), (2, 30.0, 0.01));
    assert_eq!(n.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!((n.collider.dim.radius, n.collider.dim.height, n.collider.dim.y_shift, n.collider.info.at_dmg_info.damage), (10, 4, -2, 8));
    idle(&mut w, 1);
    assert!(!w.actors.downcast::<EnSkjneedle>(h).unwrap().actor.killed);
    idle(&mut w, 1);
    assert!(w.actors.downcast::<EnSkjneedle>(h).is_none_or(|n| n.actor.killed));
    // With killTimer 100 and speed 24 (as EnSkj_ShootNeedle sets them), across Link's view: 24 a
    // frame along its yaw (Actor_MoveXZGravity, no gravity: Actor_UpdatePos's R_UPDATE_RATE * 0.5,
    // 1.5, a frame).
    let side = yaw.wrapping_add(0x4000);
    let h = w.actor_spawn(oot_actors::en_skjneedle::ACTOR_EN_SKJNEEDLE, at_pos, [0, side, 0], 0).expect("En_Skjneedle");
    {
        let n = w.actors.downcast_mut::<EnSkjneedle>(h).unwrap();
        n.kill_timer = 100;
        n.actor.speed_xz = 24.0;
    }
    idle(&mut w, 2);
    let n = w.actors.downcast::<EnSkjneedle>(h).unwrap();
    let moved = n.actor.world_pos - at_pos;
    let expect = Vec3::new(eng_math::sin_s(side), 0.0, eng_math::cos_s(side)) * 24.0 * 1.5;
    assert!((moved - expect).length() < 0.01, "{moved:?} vs {expect:?}");
    assert_eq!((n.kill_timer, n.unused_timer1), (99, 1));
    // Held there (speed 0): killTimer at 0 on the 100th update, killed.
    w.actors.downcast_mut::<EnSkjneedle>(h).unwrap().actor.speed_xz = 0.0;
    let n = until(&mut w, 120, "the needle gone", |w| w.actors.downcast::<EnSkjneedle>(h).is_none_or(|n| n.actor.killed));
    assert_eq!(n, 99);
}
