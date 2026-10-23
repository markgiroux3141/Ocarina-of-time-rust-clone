//! GAME-07 milestone 3 against the C: the ocarina. Player's (`Player_ActionHandler_13`'s ocarina,
//! `Player_Action_8084E3C4`), the message box's modes (`Message_StartOcarinaImpl`,
//! `Message_DrawMain`), the audio (`AudioOcarina_*`, its staffs read through the box), the songs'
//! effects (`Oceff_*`, `En_Okarina_Effect`), the spots (`Shot_Sun`, `En_Okarina_Tag`), a cutscene's
//! teach and playback texts (`CS_TEXT_OCARINA_ACTION`), and the exit's run (`Route::Ocarina`).
//!
//! The songs are played on the pad, their notes read from the pack's `gOcarinaSongButtons` (each
//! note held 3 frames, let go 2). Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_B, BTN_CDOWN, BTN_CLEFT, BTN_CRIGHT, BTN_CUP, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::player::{Action as PA, Player};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::ocarina::*;
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Child Link at `entrance` at `hour`:00 with `ocarina-songs` (less the quest songs in `unlearn`),
/// the audio side running (a scene's audio spec waits for it; the ocarina counts its tasks).
fn at(a: &Arc<GameAssets>, entrance: &str, hour: i32, unlearn: &[u32]) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(hour, 0) as u16);
    save.apply_preset("ocarina-songs").unwrap();
    for &q in unlearn {
        save.inventory.quest_items &= !(1 << q);
    }
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    idle(&mut w, 2);
    w
}

/// Hyrule Field (`ENTR_HYRULE_FIELD_0`), Link put down at `pos` and settled.
fn field(a: &Arc<GameAssets>, hour: i32, pos: Vec3) -> PlayState {
    let mut w = at(a, "ENTR_HYRULE_FIELD_0", hour, &[]);
    w.place_player(pos, 0);
    idle(&mut w, 30);
    w
}

/// Open field 300 north of `Shot_Sun`'s hollow (`OCARINA_START`, out of its 150).
const FIELD_AT: Vec3 = Vec3::new(5143.0, -30.0, 4700.0);

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// `b` pressed and held for `frames` frames, then let go (a frame with it the previous state).
fn press(w: &mut PlayState, b: u16, frames: usize) {
    let pad = PadState { button: b, ..Default::default() };
    for i in 0..frames {
        w.tick_with(scripted_input(if i == 0 { PadState::default() } else { pad }, pad));
    }
    w.tick_with(scripted_input(pad, PadState::default()));
}

fn player(w: &PlayState) -> &Player {
    w.player.and_then(|h| w.actors.downcast::<Player>(h)).expect("Player")
}

/// The pad button of an `OCARINA_BTN_*` (`sOcarinaButtonsToPad`'s order: A, C-Down, C-Right,
/// C-Left, C-Up).
fn pad_button(b: u8) -> u16 {
    [BTN_A, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT, BTN_CUP][b as usize]
}

/// `song`'s buttons from the pack's `gOcarinaSongButtons`.
fn song(w: &PlayState, song: u8) -> Vec<u16> {
    let b = w.audio.ocarina.song_buttons(song);
    b.buttons_index[..b.num_buttons as usize].iter().map(|&i| pad_button(i)).collect()
}

/// Plays `buttons`, each held 3 frames and let go 2.
fn play_notes(w: &mut PlayState, buttons: &[u16]) {
    for &b in buttons {
        press(w, b, 3);
        idle(w, 1);
    }
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

/// The ocarina out with C-Left (its item there) and the staff up (`MSGMODE_OCARINA_PLAYING`).
fn ocarina_out(w: &mut PlayState) {
    press(w, BTN_CLEFT, 1);
    until(w, 40, "the staff", |w| w.msg_ctx.msg_mode == MSGMODE_OCARINA_PLAYING);
}

/// The msgModes seen frame by frame, each with the frame it began, across `notes` and `until`s.
struct Watch {
    seen: Vec<(usize, u8)>,
    n: usize,
}

impl Watch {
    fn new(w: &PlayState) -> Self {
        Watch { seen: vec![(0, w.msg_ctx.msg_mode)], n: 0 }
    }

    fn tick(&mut self, w: &mut PlayState, prev: PadState, cur: PadState) {
        w.tick_with(scripted_input(prev, cur));
        self.n += 1;
        if self.seen.last().unwrap().1 != w.msg_ctx.msg_mode {
            self.seen.push((self.n, w.msg_ctx.msg_mode));
        }
    }

    /// `notes` played (as `play_notes`).
    fn notes(&mut self, w: &mut PlayState, notes: &[u16]) {
        for &b in notes {
            let pad = PadState { button: b, ..Default::default() };
            for (prev, cur) in [(PadState::default(), pad), (pad, pad), (pad, pad), (pad, PadState::default()), (PadState::default(), PadState::default())] {
                self.tick(w, prev, cur);
            }
        }
    }

    /// Frames until `f` holds (at most `max`).
    fn until(&mut self, w: &mut PlayState, max: usize, f: impl Fn(&PlayState) -> bool) -> &mut Self {
        for _ in 0..max {
            if f(w) {
                return self;
            }
            self.tick(w, PadState::default(), PadState::default());
        }
        panic!("not after {max} frames: {:x?}", self.seen);
    }

    /// The frame `mode` began.
    fn began(&self, mode: u8) -> usize {
        self.seen.iter().find(|s| s.1 == mode).unwrap_or_else(|| panic!("msgMode {mode:#x} never seen: {:x?}", self.seen)).0
    }

    /// Frames from `a`'s start to `b`'s.
    fn between(&self, a: u8, b: u8) -> usize {
        self.began(b) - self.began(a)
    }
}

/// Through a cutscene's texts until `f` holds (at most `max` frames): A pressed on a frame of its
/// own whenever a text waits (done, an event, or a choice: its first).
fn answer_until(w: &mut PlayState, max: usize, what: &str, f: impl Fn(&PlayState) -> bool) {
    let mut pressed = false;
    for _ in 0..max {
        if f(w) {
            return;
        }
        let st = w.message_state();
        let a = !pressed && matches!(st, TEXT_STATE_CHOICE | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_EVENT | TEXT_STATE_AWAITING_NEXT);
        let pad = PadState { button: if a { BTN_A } else { 0 }, ..Default::default() };
        w.tick_with(scripted_input(PadState::default(), pad));
        pressed = a;
    }
    panic!("{what}: not after {max} frames (msgMode {:#x}, text {:#x}, cs frame {})", w.msg_ctx.msg_mode, w.msg_ctx.text_id, w.cs_ctx.frames);
}

fn find<T: oot_game::actor_ctx::ActorImpl + 'static>(w: &PlayState) -> Option<(ActorHandle, &T)> {
    w.actors.all().into_iter().find_map(|h| w.actors.downcast::<T>(h).map(|t| (h, t)))
}

#[test]
fn the_ocarina_comes_out_in_free_play() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, FIELD_AT);
    ocarina_out(&mut w);
    // Player_ActionHandler_13: Player_Action_8084E3C4 with PLAYER_STATE2_USING_OCARINA (27); its
    // first frame, without PLAYER_STATE2_24 (nothing listening), Message_StartOcarina(FREE_PLAY).
    let p = player(&w);
    assert_eq!(p.action, PA::Ocarina);
    assert_ne!(p.state2 & oot_actors::player::STATE2_27, 0);
    // Message_StartOcarinaImpl: FREE_PLAY opens 0x86E ("Play using [A] and [C]; [B] to Stop"),
    // MSGMODE_OCARINA_STARTING then (Message_DrawMain) ocarinaMode 01 and MSGMODE_OCARINA_PLAYING;
    // the instrument's the Fairy Ocarina's (OCARINA_INSTRUMENT_DEFAULT, 1).
    assert_eq!(w.msg_ctx.ocarina_action, OCARINA_ACTION_FREE_PLAY);
    assert_eq!(w.msg_ctx.text_id, 0x86E);
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_01);
    assert_eq!(w.audio.ocarina.instrument_id as u8, OCARINA_INSTRUMENT_DEFAULT);
    // Notes go onto the staff: MSGMODE_OCARINA_PLAYING takes the playing staff's buttonIndex as
    // its pos moves on (sOcarinaButtonIndexBuf, lastOcarinaButtonIndex for that frame: Message_Update
    // sets it OCARINA_BTN_INVALID the next).
    press(&mut w, BTN_A, 3);
    idle(&mut w, 1);
    let pad = PadState { button: BTN_CUP, ..Default::default() };
    let mut prev = PadState::default();
    for _ in 0..4 {
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        if w.audio.ocarina.playing_staff.pos == 2 {
            break;
        }
    }
    // (Audio_Update runs after the frame's draw: the box takes it the next.)
    let staff = w.audio.ocarina.playing_staff;
    assert_eq!((staff.pos, staff.button_index, w.msg_ctx.last_ocarina_button_index), (2, OCARINA_BTN_C_UP, OCARINA_BTN_INVALID));
    w.tick_with(scripted_input(pad, pad));
    assert_eq!(w.msg_ctx.last_ocarina_button_index, OCARINA_BTN_C_UP);
    w.tick_with(scripted_input(pad, PadState::default()));
    // (Let go: the staff's buttonIndex is sCurOcarinaButtonIndex & 0x3F, 63.)
    let staff = w.audio.ocarina.playing_staff;
    assert_eq!((staff.pos, staff.button_index, w.msg_ctx.last_ocarina_button_index), (2, 0xFF & 0x3F, OCARINA_BTN_INVALID));
    // B: `CHECK_BTN_ALL(input->press.button, BTN_B)` in MSGMODE_OCARINA_PLAYING: the instrument
    // off, ocarinaMode 04, the box closed; Player_Action_8084E3C4 then puts the ocarina away.
    press(&mut w, BTN_B, 1);
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    assert_eq!(w.audio.ocarina.instrument_id as u8, OCARINA_INSTRUMENT_OFF);
    until(&mut w, 60, "the ocarina away", |w| player(w).action != PA::Ocarina && w.message_state() == TEXT_STATE_NONE);
}

/// The ocarina's channel (`SFX_CHANNEL_OCARINA` of `SEQ_PLAYER_SFX`) as the game last read it:
/// its IO port 1, `SEQ_IO_VAL_NONE` (-1) once its sound has ended.
fn ocarina_channel_io1(w: &PlayState) -> i8 {
    use oot_game::audio::{SEQ_PLAYER_SFX, SFX_CHANNEL_OCARINA};
    w.audio.view.players[SEQ_PLAYER_SFX as usize].channels[SFX_CHANNEL_OCARINA as usize].sound_script_io[1]
}

#[test]
fn a_released_note_stops() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, FIELD_AT);
    ocarina_out(&mut w);
    // AudioOcarina_PlayControllerInput: a button let go stops the note (Audio_StopSfxById), which
    // reaches the channel (AUDIOCMD_CHANNEL_SET_IO port 0) only while its sound's entry is kept:
    // Audio_PlayActiveSfx forgets an entry whose channel's IO port 1 reads SEQ_IO_VAL_NONE, so the
    // game must read the channel after the audio side has run the note's start (a view from
    // before it lost every other note's stop, which then played on to its end). Forty notes,
    // held 3 frames, the gaps after them 0 to 6 frames: each over 8 frames after its release.
    let buttons = [BTN_A, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT, BTN_CUP];
    for k in 0..40usize {
        press(&mut w, buttons[k % 5], 3);
        idle(&mut w, 8);
        assert_eq!(ocarina_channel_io1(&w), -1, "note {k} still sounding");
        idle(&mut w, k % 7);
    }
}

#[test]
fn the_audio_hand_over_reads_the_view_before_each_frame() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, FIELD_AT);
    w.audio_side = None;
    // advance_with: the view back before a frame, its ops after it, frame by frame (two frames
    // due at once still alternate).
    let seen = std::cell::RefCell::new(Vec::new());
    let n = w.advance_with(2.5 / 20.0, |a| seen.borrow_mut().push(('v', a.frames)), |a| seen.borrow_mut().push(('o', a.frames)));
    assert_eq!(n, 2);
    let s = seen.into_inner();
    assert_eq!(s.iter().map(|x| x.0).collect::<String>(), "vovo");
    assert!(s[0].1 < s[1].1 && s[1].1 == s[2].1 && s[2].1 < s[3].1, "{s:?}");
}

#[test]
fn sarias_song_in_free_play_calls_saria() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, FIELD_AT);
    ocarina_out(&mut w);
    let notes = song(&w, OCARINA_SONG_SARIAS);
    assert_eq!(notes, [BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT, BTN_CDOWN, BTN_CRIGHT, BTN_CLEFT]);
    // The staff's state OCARINA_SONG_SARIAS (6) with its quest bit: lastPlayedSong and unk_E3F2 6,
    // MSGMODE_OCARINA_CORRECT_PLAYBACK, then (FREE_PLAY) 0x86F and MSGMODE_SONG_PLAYED.
    let mut m = Watch::new(&w);
    m.notes(&mut w, &notes);
    m.until(&mut w, 400, |w| w.msg_ctx.msg_mode == MSGMODE_DISPLAY_SONG_PLAYED);
    assert_eq!((w.msg_ctx.last_played_song, w.msg_ctx.unk_e3f2), (OCARINA_SONG_SARIAS as u16, OCARINA_SONG_SARIAS as u16));
    // MSGMODE_SONG_PLAYED's stateTimer 10, then MSGMODE_SETUP_DISPLAY_SONG_PLAYED.
    assert_eq!(m.between(MSGMODE_SONG_PLAYED, MSGMODE_SETUP_DISPLAY_SONG_PLAYED), 10);
    // MSGMODE_SETUP_DISPLAY_SONG_PLAYED: sOcarinaSongFanfares[6] (NA_BGM_OCA_SARIA, 0x44), and the
    // FREE_PLAY action done (OCARINA_ACTION_FREE_PLAY_DONE).
    assert_eq!(w.audio.fanfare_seq_id, 0x44);
    assert_eq!(w.msg_ctx.ocarina_action, OCARINA_ACTION_FREE_PLAY_DONE);
    // The song's effect (MSGMODE_SONG_PLAYED's end): sOcarinaEffectActorIds[6 - 6], Oceff_Wipe3
    // with params 0, at Link (it follows the camera's eye after); OceffWipe3_Update counts to
    // 100, then kills it (deleted at the next Actor_UpdateAll).
    let (h, wipe) = find::<oot_actors::oceff_wipe3::OceffWipe3>(&w).expect("Oceff_Wipe3");
    assert_eq!(w.actors.actor(h).unwrap().params, 0);
    let first = m.n - wipe.counter as usize;
    m.until(&mut w, 200, |w| find::<oot_actors::oceff_wipe3::OceffWipe3>(w).is_none());
    assert_eq!(m.n - first, 100 + 2);
    // MSGMODE_DISPLAY_SONG_PLAYED: the song played back; "You played Saria's Song" (0x893 + 6).
    m.until(&mut w, 400, |w| w.msg_ctx.text_id == 0xE0);
    assert_eq!(m.between(MSGMODE_DISPLAY_SONG_PLAYED_TEXT, MSGMODE_SONG_PLAYED_ACT_BEGIN), 20);
    // The order Message_DrawMain goes through.
    let order = [
        MSGMODE_SONG_PLAYED,
        MSGMODE_SETUP_DISPLAY_SONG_PLAYED,
        MSGMODE_DISPLAY_SONG_PLAYED,
        MSGMODE_DISPLAY_SONG_PLAYED_TEXT_BEGIN,
        MSGMODE_DISPLAY_SONG_PLAYED_TEXT,
        MSGMODE_SONG_PLAYED_ACT_BEGIN,
        MSGMODE_SONG_PLAYED_ACT,
    ];
    assert!(order.windows(2).all(|p| m.began(p[0]) < m.began(p[1])), "{:x?}", m.seen);
    // The box closed (MSGMODE_SONG_PLAYED_ACT: Saria's isn't a warp song), then (TEXT_CLOSING,
    // Saria's Song) `play->msgCtx.ocarinaMode == OCARINA_MODE_01` with lastPlayedSong SARIAS:
    // Navi's text -0xE0 (Player_SetNaviTextId's negative id: she talks at once).
    assert_eq!(w.msg_ctx.text_id, 0xE0);
}

#[test]
fn the_suns_song_by_night_runs_the_clock_to_dawn() {
    use oot_game::clock::{SUNSSONG_INACTIVE, SUNSSONG_SPEED_TIME};
    let Some(a) = assets() else { return };
    let mut w = field(&a, 19, FIELD_AT);
    let prev_speed = w.env_statics.time_speed;
    ocarina_out(&mut w);
    let notes = song(&w, OCARINA_SONG_SUNS);
    play_notes(&mut w, &notes);
    // "You played the Sun's Song" (0x893 + 9); sOcarinaEffectActorIds[9 - 6]: Oceff_Spot.
    until(&mut w, 200, "0x89C", |w| w.msg_ctx.text_id == 0x89C);
    assert_eq!(w.audio.fanfare_seq_id, 0x47); // NA_BGM_OCA_SUNS
    assert!(find::<oot_actors::oceff_spot::OceffSpot>(&w).is_some());
    // OceffSpot_End: gTimeSpeed isn't 400, so sunsSongState SUNSSONG_START; Interface_Update
    // (the field's time moves; 19:00 isn't within 06:30..18:01, so D_80125B60 false): ocarinaMode
    // 04, SUNSSONG_SPEED_TIME and gTimeSpeed 400.
    until(&mut w, 400, "SUNSSONG_SPEED_TIME", |w| w.save.suns_song_state == SUNSSONG_SPEED_TIME);
    // (Killed: deleted at the next Actor_UpdateAll.)
    idle(&mut w, 1);
    assert!(find::<oot_actors::oceff_spot::OceffSpot>(&w).is_none());
    assert_eq!(w.env_statics.time_speed, 400);
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    // At 400 a frame to 06:30 (CLOCK_TIME(6, 30)): SUNSSONG_INACTIVE, the speed it was.
    let dawn = oot_game::env::clock_time(6, 30) as u16;
    until(&mut w, 400, "dawn", |w| w.save.suns_song_state == SUNSSONG_INACTIVE);
    assert!(w.save.day_time >= dawn && w.save.day_time < dawn + 400 * 2, "{:#x}", w.save.day_time);
    assert_eq!(w.env_statics.time_speed, prev_speed);
}

#[test]
fn the_song_of_storms_brings_rain() {
    use oot_game::env::{LIGHTNING_LAST, LIGHTNING_ON, PRECIP_SOS_MAX, STORM_REQUEST_NONE, STORM_REQUEST_START};
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, FIELD_AT);
    ocarina_out(&mut w);
    let notes = song(&w, OCARINA_SONG_STORMS);
    play_notes(&mut w, &notes);
    // 0x893 + 11; sOcarinaEffectActorIds[11 - 6]: Oceff_Storm (params 0), which spawns
    // En_Okarina_Effect (params 1) 30 below it.
    until(&mut w, 200, "0x89E", |w| w.msg_ctx.text_id == 0x89E);
    assert_eq!(w.audio.fanfare_seq_id, 0x49); // NA_BGM_OCA_STORM
    assert!(find::<oot_actors::oceff_storm::OceffStorm>(&w).is_some());
    let (_, e) = find::<oot_actors::en_okarina_effect::EnOkarinaEffect>(&w).expect("En_Okarina_Effect");
    // EnOkarinaEffect_TriggerStorm: timer 400, precipitation[PRECIP_SOS_MAX] 20, a storm asked
    // for, the lightning on.
    assert_eq!(e.action, oot_actors::en_okarina_effect::Action::ManageStorm);
    assert!(e.timer <= 400 && e.timer > 300, "{}", e.timer);
    assert_eq!(w.env_ctx.precipitation[PRECIP_SOS_MAX], 20);
    assert_eq!(w.env_ctx.storm_request, STORM_REQUEST_START);
    assert_eq!(w.env_ctx.lightning_state, LIGHTNING_ON);
    // Environment_UpdateStorm: STORM_STATE_OFF with STORM_REQUEST_START: the sky's change to the
    // storm's (skyboxConfig 0 to 1, lights 0 to 2 over 100 frames), STORM_STATE_ON.
    assert_eq!(w.env_ctx.storm_state, 1);
    // EnOkarinaEffect_ManageStorm counts it down (a frame at a time with no text up, or while
    // it's 250 or more); at 0 the rain's maximum goes, the storm's stopped (WEATHER_MODE_CLEAR:
    // STORM_REQUEST_STOP), the lightning's last, and the actor's killed; Environment_UpdateStorm the
    // same frame: the sky back, STORM_REQUEST_NONE and STORM_STATE_OFF.
    let n = until(&mut w, 600, "the storm's end", |w| w.env_ctx.lightning_state == LIGHTNING_LAST);
    assert!(n >= 250, "{n}");
    assert_eq!(w.env_ctx.precipitation[PRECIP_SOS_MAX], 0);
    assert_eq!((w.env_ctx.storm_request, w.env_ctx.storm_state), (STORM_REQUEST_NONE, 0));
    idle(&mut w, 1);
    assert!(find::<oot_actors::en_okarina_effect::EnOkarinaEffect>(&w).is_none());
}

#[test]
fn shot_sun_gives_a_fairy_for_the_song_of_storms() {
    use oot_actors::shot_sun::{Action as SA, SPAWNER_OCARINA_PLAYING, SPAWNER_OUT_OF_RANGE, ShotSun};
    let Some(a) = assets() else { return };
    // Hyrule Field's fairy spot (params 0x41: the Song of Storms) at (5143, -55, 5012); Link 80
    // north of it, within its 150.
    let mut w = field(&a, 10, Vec3::new(5143.0, -60.0, 4932.0));
    let (h, s) = find::<ShotSun>(&w).expect("Shot_Sun");
    assert_eq!(w.actors.actor(h).unwrap().params & 0xFF, 0x41);
    assert_eq!(s.fairy_spawner_state, SPAWNER_OUT_OF_RANGE);
    // ShotSun_UpdateFairySpawner: in range without PLAYER_STATE2_24, Link's told something
    // listens (PLAYER_STATE2_23). The ocarina out (PLAYER_STATE2_24): SPAWNER_OCARINA_START, the
    // spot's own Message_StartOcarina(FREE_PLAY), SPAWNER_OCARINA_PLAYING.
    ocarina_out(&mut w);
    assert_eq!(find::<ShotSun>(&w).unwrap().1.fairy_spawner_state, SPAWNER_OCARINA_PLAYING);
    // Another song (Epona's): at OCARINA_MODE_04 it isn't the spot's, SPAWNER_OUT_OF_RANGE.
    let notes = song(&w, OCARINA_SONG_EPONAS);
    play_notes(&mut w, &notes);
    until(&mut w, 300, "the song's end", |w| w.msg_ctx.ocarina_mode == OCARINA_MODE_04);
    idle(&mut w, 1);
    let (_, s) = find::<ShotSun>(&w).expect("Shot_Sun");
    assert_eq!((s.action, s.fairy_spawner_state), (SA::UpdateFairySpawner, SPAWNER_OUT_OF_RANGE));
    until(&mut w, 60, "the ocarina away", |w| player(w).action != PA::Ocarina);
    // The Song of Storms: ShotSun_TriggerFairy (OnePointCutscene_Attention), then
    // ShotSun_SpawnFairy with timer 50 and Demo_Kankyo's sparkles (DEMOKANKYO_SPARKLES) at home.
    ocarina_out(&mut w);
    let notes = song(&w, OCARINA_SONG_STORMS);
    play_notes(&mut w, &notes);
    until(&mut w, 300, "ShotSun_SpawnFairy", |w| find::<ShotSun>(w).is_some_and(|(_, s)| s.action == SA::SpawnFairy));
    assert_eq!(find::<ShotSun>(&w).unwrap().1.timer, 50);
    let home = w.actors.actor(h).unwrap().home_pos;
    let sparkles = w
        .actors
        .all()
        .into_iter()
        .filter_map(|h| w.actors.actor(h))
        .any(|a| a.id == oot_actors::demo_kankyo::ACTOR_DEMO_KANKYO && a.params == oot_actors::demo_kankyo::DEMOKANKYO_SPARKLES && a.world_pos == home);
    assert!(sparkles);
    // 50 frames counted down, the next En_Elf (FAIRY_HEAL_BIG) at home, and the spot's killed
    // (deleted at the next Actor_UpdateAll).
    idle(&mut w, 50);
    assert_eq!(find::<ShotSun>(&w).unwrap().1.timer, 0);
    let elf = |w: &PlayState| w.actors.all().into_iter().filter_map(|h| w.actors.actor(h)).any(|a| a.id == oot_actors::en_elf::ACTOR_EN_ELF && a.params == oot_actors::en_elf::FAIRY_HEAL_BIG);
    assert!(!elf(&w));
    idle(&mut w, 1);
    assert!(elf(&w));
    idle(&mut w, 1);
    assert!(find::<ShotSun>(&w).is_none());
    let fairy = w.actors.all().into_iter().filter_map(|h| w.actors.actor(h)).find(|a| a.id == oot_actors::en_elf::ACTOR_EN_ELF && a.params == oot_actors::en_elf::FAIRY_HEAL_BIG).expect("the fairy");
    assert_eq!(fairy.home_pos, home);
}

#[test]
fn a_tag_checks_its_song() {
    use oot_actors::en_okarina_tag::{Action as TA, EnOkarinaTag};
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, FIELD_AT);
    // En_Okarina_Tag type 7 (params >> 10), song 0 (Saria's: (params >> 6) & 0xF), switch flag
    // 0x10: 0x1C10. Link 30 from it (func_80ABEF2C: within 50 + interactRange, 40 in y, it's
    // player->unk_6A8).
    let pos = player(&w).actor.world_pos;
    let tag = w.actor_spawn(oot_actors::en_okarina_tag::ACTOR_EN_OKARINA_TAG, pos + Vec3::new(0.0, 0.0, 30.0), [0; 3], 0x1C10).expect("En_Okarina_Tag");
    idle(&mut w, 2);
    assert_eq!(w.actors.downcast::<EnOkarinaTag>(tag).unwrap().ocarina_song, 0);
    assert_eq!(player(&w).unk_6a8, Some(tag));
    // The ocarina out to it: PLAYER_STATE2_25 (Player_ActionHandler_13 with unk_6A8), and the
    // tag's Message_StartOcarina(0 + OCARINA_ACTION_CHECK_SARIA), NA_SE_SY_TRE_BOX_APPEAR.
    ocarina_out(&mut w);
    assert_ne!(player(&w).state2 & oot_actors::player::STATE2_25, 0);
    assert_eq!(w.msg_ctx.ocarina_action, OCARINA_ACTION_CHECK_SARIA);
    assert_eq!(w.actors.downcast::<EnOkarinaTag>(tag).unwrap().action, TA::Func80abf0cc);
    // Notes no owned song begins with: the staff's state 0xFF once its notes are out (on-stage:
    // every owned song checked), MSGMODE_OCARINA_FAIL (stateTimer 10), the notes dropped, and
    // MSGMODE_OCARINA_STARTING again (ocarinaMode stays 01: the tag waits).
    let mut m = Watch::new(&w);
    m.notes(&mut w, &[BTN_A; 8]);
    m.until(&mut w, 200, |w| w.msg_ctx.msg_mode == MSGMODE_OCARINA_PLAYING);
    assert_eq!(m.between(MSGMODE_OCARINA_FAIL, MSGMODE_OCARINA_NOTES_DROP), 10);
    assert!(m.began(MSGMODE_OCARINA_NOTES_DROP) < m.began(MSGMODE_OCARINA_STARTING), "{:x?}", m.seen);
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_01);
    assert!(!w.flags.get_switch(0x10));
    // Saria's Song: MSGMODE_SONG_PLAYED_ACT with CHECK_SARIA (>= CHECK_MINUET): lastPlayedSong 6 ==
    // CHECK_SARIA - CHECK_MINUET, OCARINA_MODE_03; func_80ABF0CC sets the switch flag, ocarinaMode
    // 04, NA_SE_SY_CORRECT_CHIME, and goes back to func_80ABEF2C.
    let notes = song(&w, OCARINA_SONG_SARIAS);
    let mut m = Watch::new(&w);
    m.notes(&mut w, &notes);
    m.until(&mut w, 300, |w| w.flags.get_switch(0x10));
    // (Not FREE_PLAY: MSGMODE_OCARINA_CORRECT_PLAYBACK's stateTimer 20, then 0x86F and
    // MSGMODE_SONG_PLAYED with stateTimer 1.)
    assert_eq!(m.between(MSGMODE_OCARINA_CORRECT_PLAYBACK, MSGMODE_SONG_PLAYED), 20);
    assert_eq!(m.between(MSGMODE_SONG_PLAYED, MSGMODE_SETUP_DISPLAY_SONG_PLAYED), 1);
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    assert_eq!(w.actors.downcast::<EnOkarinaTag>(tag).unwrap().action, TA::Func80abef2c);
    // Navi all the same: TEXT_CLOSING's call is sLastPlayedSong SARIAS with any action but
    // OCARINA_ACTION_CHECK_NOWARP_DONE.
    until(&mut w, 60, "Navi's 0xE0", |w| w.msg_ctx.text_id == 0xE0);
}

#[test]
fn the_meadows_script_teaches_sarias_song_and_hears_it_back() {
    use oot_game::item::QUEST_SONG_SARIA;
    let Some(a) = assets() else { return };
    // The Sacred Forest Meadow (ENTR_SACRED_FOREST_MEADOW_2, Saria's ledge), Saria's Song not
    // learnt; En_Sa's `play->csCtx.script = gMeadowSariasSongCs; gSaveContext.cutsceneTrigger = 1`.
    let mut w = at(&a, "ENTR_SACRED_FOREST_MEADOW_2", 10, &[QUEST_SONG_SARIA]);
    idle(&mut w, 30);
    w.cs_ctx.segment = Some(w.cutscene_script("gMeadowSariasSongCs").expect("gMeadowSariasSongCs"));
    w.save.cutscene_trigger = 1;
    // The texts up to the lesson: 0x10A8's choice (its 0: "yes").
    // CS_TEXT_OCARINA_ACTION 0x0008 (OCARINA_ACTION_TEACH_SARIA): MSGMODE_SONG_DEMONSTRATION_STARTING,
    // 20 frames of MSGMODE_SONG_DEMONSTRATION_SELECT_INSTRUMENT (TEACH_SARIA isn't below
    // TEACH_SARIA, nor Epona's, the lullaby's or the storms': OCARINA_INSTRUMENT_DEFAULT), then
    // the song played back twice (AudioOcarina_SetPlaybackSong(8 - 2 + 1, 2)).
    answer_until(&mut w, 1200, "TEACH_SARIA", |w| w.msg_ctx.ocarina_action == OCARINA_ACTION_TEACH_SARIA);
    let mut m = Watch::new(&w);
    m.until(&mut w, 600, |w| w.msg_ctx.msg_mode == MSGMODE_SONG_DEMONSTRATION);
    assert_eq!(m.between(MSGMODE_SONG_DEMONSTRATION_SELECT_INSTRUMENT, MSGMODE_SONG_DEMONSTRATION), 20);
    assert_eq!(w.audio.ocarina.instrument_id as u8, OCARINA_INSTRUMENT_DEFAULT);
    // The playback staff's notes are Saria's Song's, twice: twelve on the box's staff in turn.
    let n = until(&mut w, 1200, "the demonstration's end", |w| w.msg_ctx.msg_mode == MSGMODE_SONG_DEMONSTRATION_DONE);
    assert!(n > 100, "{n}");
    assert_eq!(w.message_state(), TEXT_STATE_SONG_DEMO_DONE);
    // CS_TEXT_OCARINA_ACTION 0x0015 (OCARINA_ACTION_PLAYBACK_SARIA): MSGMODE_SONG_PLAYBACK_STARTING,
    // then MSGMODE_SONG_PLAYBACK (AudioOcarina_Start((1 << 6) + 0x8000)).
    answer_until(&mut w, 600, "PLAYBACK_SARIA", |w| w.msg_ctx.ocarina_action == OCARINA_ACTION_PLAYBACK_SARIA && w.msg_ctx.msg_mode == MSGMODE_SONG_PLAYBACK);
    // A wrong note: the staff's 0xFF, MSGMODE_SONG_PLAYBACK_FAIL (stateTimer 10), 0x88B (the red X),
    // MSGMODE_SONG_PLAYBACK_NOTES_DROP, then MSGMODE_OCARINA_AWAIT_INPUT.
    let mut m = Watch::new(&w);
    m.notes(&mut w, &[BTN_CUP]);
    m.until(&mut w, 200, |w| w.msg_ctx.msg_mode == MSGMODE_OCARINA_AWAIT_INPUT);
    assert_eq!(m.between(MSGMODE_SONG_PLAYBACK_FAIL, MSGMODE_SONG_PLAYBACK_NOTES_DROP), 10);
    assert_eq!(w.msg_ctx.text_id, 0x88B);
    assert!(!w.save.check_quest_item(QUEST_SONG_SARIA));
    // MSGMODE_OCARINA_AWAIT_INPUT is TEXT_STATE_9: the script's text command (still at its end
    // frame) continues the box with its altTextId1 (0x088B); its event, with A, is the script's
    // Message_StartOcarina(PLAYBACK_SARIA) again.
    assert_eq!(w.message_state(), TEXT_STATE_9);
    answer_until(&mut w, 200, "MSGMODE_SONG_PLAYBACK", |w| w.msg_ctx.msg_mode == MSGMODE_SONG_PLAYBACK);
    // The song right: MSGMODE_SONG_PLAYBACK_SUCCESS, Item_Give(ITEM_SONG_MINUET + 8): the quest
    // bit; 20 frames on (Saria's >= OCARINA_SONG_SARIAS) 0x86F and MSGMODE_SONG_PLAYED, the song
    // played back, "You played Saria's Song" (0x899), and MSGMODE_SONG_PLAYED_ACT with
    // PLAYBACK_SARIA: lastPlayedSong 6 == PLAYBACK_SARIA - PLAYBACK_MINUET, OCARINA_MODE_03.
    let notes = song(&w, OCARINA_SONG_SARIAS);
    let mut m = Watch::new(&w);
    m.notes(&mut w, &notes);
    m.until(&mut w, 600, |w| w.msg_ctx.ocarina_mode == OCARINA_MODE_03);
    assert!(w.save.check_quest_item(QUEST_SONG_SARIA));
    assert_eq!(m.between(MSGMODE_SONG_PLAYBACK_SUCCESS, MSGMODE_SONG_PLAYED), 20);
    assert!(m.began(MSGMODE_DISPLAY_SONG_PLAYED_TEXT) < m.began(MSGMODE_SONG_PLAYED_ACT), "{:x?}", m.seen);
    assert_eq!(w.msg_ctx.last_played_song, OCARINA_SONG_SARIAS as u16);
    // The script goes on to its end.
    answer_until(&mut w, 3000, "the script's end", |w| w.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE);
}

#[test]
fn the_route_plays_its_songs() {
    use oot_actors::playthrough::{Playthrough, Route};
    let Some(a) = assets() else { return };
    let route = Route::Ocarina;
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), route.save(e)).expect("Play_Init");
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert_eq!(run.failure, None);
    let names: Vec<&str> = run.steps.iter().map(|s| s.0.name()).collect();
    assert_eq!(names, ["ocarina_out", "sarias_song", "saria_talk", "suns_song", "dawn", "song_of_storms", "storm_fairy"]);
    // The texts: the staff (0x86E) and "Ocarina" (0x86F) for each song, "You played ..." (0x893
    // + song), Navi's call (0xE0), her yes/no's yes to Saria (0xE2, 0x161), and "talk again?" no.
    assert_eq!(run.texts, [0x86E, 0x86F, 0x899, 0xE0, 0xE2, 0x161, 0xE3, 0x86E, 0x86F, 0x89C, 0x86E, 0x86F, 0x89E]);
}

// GAME-10 milestone 9: the box's scarecrow and frog modes, started as the actors start them (the
// ocarina out to them: Bonooru's func_80A915B8, En_Kakasi's func_80A8F660, En_Fr's
// EnFr_SetupFrogSong).

/// `notes` played (as `play_notes`), the box's `lastOcarinaButtonIndex` taken on each frame it's
/// set (`Message_Update` puts it back to `OCARINA_BTN_INVALID` the next).
fn notes_taken(w: &mut PlayState, notes: &[u16]) -> Vec<u8> {
    let mut taken = Vec::new();
    for &b in notes {
        let pad = PadState { button: b, ..Default::default() };
        for (prev, cur) in [(PadState::default(), pad), (pad, pad), (pad, pad), (pad, PadState::default()), (PadState::default(), PadState::default())] {
            w.tick_with(scripted_input(prev, cur));
            if w.msg_ctx.last_ocarina_button_index != OCARINA_BTN_INVALID {
                taken.push(w.msg_ctx.last_ocarina_button_index);
            }
        }
    }
    taken
}

/// `ocarina_out`, then the free play's box replaced by `action`'s (`Message_StartOcarina`) and
/// its text run to its ocarina (`Message_HandleOcarina`) until `mode`. The free play's listening
/// turned off (`AudioOcarina_Start(0xFFFF)`): the ocarina out to a scarecrow starts no free play
/// (Player's `PLAYER_STATE2_24`), so nothing listens for songs while it records.
fn ocarina_action(w: &mut PlayState, action: u16, mode: u8) {
    ocarina_out(w);
    w.audio.audio_ocarina_start(0xFFFF);
    w.message_start_ocarina(action);
    until(w, 60, "the action's mode", |w| w.msg_ctx.msg_mode == mode);
}

// OCARINA_ACTION_SCARECROW_SPAWN_RECORDING (0x2C, >= CHECK_SARIA: 0x86E) reaches its ocarina:
// MSGMODE_SCARECROW_SPAWN_RECORDING_START (AudioOcarina_SetRecordingState(SPAWN), the default
// instrument), then _ONGOING. Eight notes, none a song's, not one note eight times: the
// recording staff's state off, scarecrowSpawnSongSet, the spawn song's notes MemCopy'd into the
// save, NA_SE_SY_TRE_BOX_APPEAR, MSGMODE_SCARECROW_SPAWN_RECORDING_DONE for 20 frames (the
// notes flashing), then the box closed and OCARINA_MODE_03 (Bonooru's func_80A91620 waits on it).
// Then OCARINA_ACTION_SCARECROW_SPAWN_PLAYBACK plays it back: OCARINA_MODE_0F at its end.
#[test]
fn the_box_records_the_spawn_song() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, FIELD_AT);
    ocarina_action(&mut w, OCARINA_ACTION_SCARECROW_SPAWN_RECORDING, MSGMODE_SCARECROW_SPAWN_RECORDING_ONGOING);
    assert_eq!(w.audio.ocarina.recording_state, OCARINA_RECORD_SCARECROW_SPAWN);
    let notes = [BTN_A, BTN_CDOWN, BTN_A, BTN_CUP, BTN_A, BTN_CDOWN, BTN_A, BTN_CUP];
    let mut m = Watch::new(&w);
    assert_eq!(notes_taken(&mut w, &notes[..7]), [0, 1, 0, 4, 0, 1, 0]);
    m.notes(&mut w, &notes[7..]);
    m.until(&mut w, 40, |w| w.msg_ctx.ocarina_mode == OCARINA_MODE_03);
    assert_eq!(m.between(MSGMODE_SCARECROW_SPAWN_RECORDING_DONE, MSGMODE_TEXT_CLOSING), 20);
    assert_eq!(w.save.scarecrow_spawn_song_set, 1);
    assert_eq!(w.save.scarecrow_spawn_song, w.audio.ocarina.scarecrow_spawn_song_bytes());
    let sb = w.audio.ocarina.song_buttons(OCARINA_SONG_SCARECROW_SPAWN);
    assert_eq!(sb.buttons_index, [0, 1, 0, 4, 0, 1, 0, 4]);
    assert_ne!(w.audio.ocarina.song_note(OCARINA_SONG_SCARECROW_SPAWN, 1).volume, 0xFF);
    // Played back: the box's playback mode takes the playback staff's notes into its buffer,
    // OCARINA_MODE_0F once the staff's state drops to 0.
    until(&mut w, 60, "the box closed", |w| w.message_state() == TEXT_STATE_NONE);
    w.message_start_ocarina(OCARINA_ACTION_SCARECROW_SPAWN_PLAYBACK);
    until(&mut w, 60, "the playback", |w| w.msg_ctx.msg_mode == MSGMODE_SCARECROW_SPAWN_PLAYBACK);
    assert_eq!(heard_back(&mut w), [0, 1, 0, 4, 0, 1, 0, 4]);
}

/// The playback staff's notes until OCARINA_MODE_0F.
fn heard_back(w: &mut PlayState) -> Vec<u8> {
    let mut heard = Vec::new();
    for _ in 0..600 {
        if w.msg_ctx.ocarina_mode == OCARINA_MODE_0F {
            return heard;
        }
        idle(w, 1);
        let s = w.audio.ocarina.playback_staff;
        if s.pos as usize > heard.len() {
            heard.push(s.button_index);
        }
    }
    panic!("no OCARINA_MODE_0F: {heard:?}");
}

// Seven notes that hold Saria's Song (from the second) and an eighth: refused
// (OCARINA_RECORD_REJECTED): SetRecordingState(OFF), NA_SE_SY_OCARINA_ERROR, the box closed,
// MSGMODE_SCARECROW_SPAWN_RECORDING_FAILED: 0x40AD ("Bonooru doesn't remember") and
// OCARINA_MODE_04; nothing saved.
#[test]
fn the_box_refuses_a_song_for_the_spawn_song() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, FIELD_AT);
    ocarina_action(&mut w, OCARINA_ACTION_SCARECROW_SPAWN_RECORDING, MSGMODE_SCARECROW_SPAWN_RECORDING_ONGOING);
    let mut notes = vec![BTN_A];
    notes.extend(song(&w, OCARINA_SONG_SARIAS));
    notes.push(BTN_A);
    play_notes(&mut w, &notes);
    until(&mut w, 30, "0x40AD", |w| w.msg_ctx.text_id == 0x40AD);
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    assert_eq!(w.save.scarecrow_spawn_song_set, 0);
    assert_eq!(w.audio.ocarina.recording_state, OCARINA_RECORD_OFF);
    assert_eq!(w.audio.ocarina.song_note(OCARINA_SONG_SCARECROW_SPAWN, 1).volume, 0xFF);
}

// OCARINA_ACTION_SCARECROW_LONG_RECORDING (0x2A): MSGMODE_SCARECROW_LONG_RECORDING_START
// (SetRecordingState(LONG), the buffer emptied) then _ONGOING, each note into the buffer (eight
// at most, scrolled) and lastOcarinaButtonIndex. B: scarecrowLongSongSet (notes were played),
// NA_SE_SY_OCARINA_ERROR, SetRecordingState(OFF), OCARINA_MODE_04, the box closed, the long song
// MemCopy'd into the save. Then OCARINA_ACTION_SCARECROW_LONG_PLAYBACK (Message_StartOcarinaImpl
// itself: stateTimer 3, the playback song OCARINA_SONG_SCARECROW_LONG): OCARINA_MODE_0F at its end.
#[test]
fn the_box_records_the_long_song() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, FIELD_AT);
    ocarina_action(&mut w, OCARINA_ACTION_SCARECROW_LONG_RECORDING, MSGMODE_SCARECROW_LONG_RECORDING_ONGOING);
    assert_eq!(w.audio.ocarina.recording_state, OCARINA_RECORD_SCARECROW_LONG);
    assert_eq!(notes_taken(&mut w, &[BTN_CUP, BTN_CLEFT, BTN_CRIGHT]), [OCARINA_BTN_C_UP, OCARINA_BTN_C_LEFT, OCARINA_BTN_C_RIGHT]);
    press(&mut w, BTN_B, 1);
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_04);
    assert_eq!(w.save.scarecrow_long_song_set, 1);
    assert_eq!(w.audio.ocarina.recording_state, OCARINA_RECORD_OFF);
    assert_eq!(w.save.scarecrow_long_song, w.audio.ocarina.scarecrow_long_song_bytes());
    let pitches: Vec<u8> = w.audio.ocarina.scarecrows_long_song_notes[..7].iter().map(|n| n.pitch).collect();
    let r = OCARINA_PITCH_NONE;
    assert_eq!(pitches, [r, 0xE, r, 0xB, r, 0x9, r]);
    until(&mut w, 60, "the box closed", |w| w.message_state() == TEXT_STATE_NONE && player(w).action != PA::Ocarina);
    // Played back from Message_StartOcarinaImpl (no ocarina out: Kakasi's func_80A8F8D0).
    w.message_start_ocarina(OCARINA_ACTION_SCARECROW_LONG_PLAYBACK);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_SCARECROW_LONG_PLAYBACK);
    assert_eq!(heard_back(&mut w), [OCARINA_BTN_C_UP, OCARINA_BTN_C_LEFT, OCARINA_BTN_C_RIGHT]);
}

// OCARINA_ACTION_FROGS: Message_StartOcarinaImpl goes straight to MSGMODE_FROGS_START (the
// default instrument, the playing staff, OCARINA_MODE_01, AudioOcarina_Start(flags + 0xC000)),
// then MSGMODE_FROGS_PLAYING; a note played: lastOcarinaButtonIndex, MSGMODE_FROGS_WAITING (En_Fr
// answers and puts MSGMODE_FROGS_START back).
#[test]
fn the_frogs_mode_takes_a_note_at_a_time() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, FIELD_AT);
    ocarina_out(&mut w);
    w.message_start_ocarina(OCARINA_ACTION_FROGS);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_FROGS_START);
    idle(&mut w, 1);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_FROGS_PLAYING);
    assert_eq!(w.msg_ctx.ocarina_mode, OCARINA_MODE_01);
    assert_eq!(notes_taken(&mut w, &[BTN_CDOWN]), [OCARINA_BTN_C_DOWN]);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_FROGS_WAITING);
    w.msg_ctx.msg_mode = MSGMODE_FROGS_START;
    idle(&mut w, 1);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_FROGS_PLAYING);
    assert_eq!(notes_taken(&mut w, &[BTN_A]), [OCARINA_BTN_A]);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_FROGS_WAITING);
}
