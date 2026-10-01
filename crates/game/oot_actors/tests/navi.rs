//! `En_Elf` (`z_en_elf.c`): Navi and the Kokiri children's fairies, against the C:
//! - `Player_Init` spawns Navi (`Player_SpawnFairy`, `FAIRY_NAVI`) 50 above Link
//!   (`D_80854778`), in no room, at scale 0.008 (`ICHAIN_VEC3F_DIV1000(scale, 8)`), with
//!   `fairyFlags` 4 and `unk_2C7` 0x14; her update is `func_80A053F0`;
//! - with nothing to point at and `PLAYER_STATE2_20` clear (a new scene's Player), mode 0 turns
//!   at once into 7 (`func_80A0461C`: `NA_SE_EV_NAVY_VANISH`): into Link's hat, which she
//!   reaches within the 30 frames of `unk_2AE` and stays in (8), undrawn (`EnElf_Draw`) at
//!   scale 0;
//! - `naviTimer` counts outside cutscenes; from 600 to 3000 her update sets Player's
//!   `naviTextId` from `ElfMessage_GetCUpText`: in Kokiri Forest (`elf_message_field`),
//!   `ELF_MSG_FLAG(CHECK, 0x40, false, EVENTCHKINF_05)` gives 0x140 on a new save;
//! - C-Up then talks to her (`func_8083B644`, `func_80853148`): her text, `naviTimer` 3001
//!   (`func_80A053F0`: the text is her C-Up text), her talk (`func_80A052F4`) until it closes;
//! - a Kokiri's fairy (`FAIRY_KOKIRI`) bobs 1500 × 0.008 + 40 above its parent
//!   (`func_80A0353C`), in a colour of `sColorFlags` with alpha 0.

mod common;

use std::sync::Arc;

use eng_input::pad::{BTN_A, BTN_CUP, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_elf::{self, EnElf, FAIRY_KOKIRI, FAIRY_NAVI, PLAYER_STATE2_20};
use oot_actors::player::Action;
use oot_game::actor_ctx::{ActorImpl, PLAYER_BODYPART_HAT};
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn enter(a: &Arc<GameAssets>, entrance: &str) -> Option<PlayState> {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    Some(oot_actors::play_entrance(a.clone(), common::data()?, common::rules()?, save).expect("Play_Init"))
}

fn navi(w: &PlayState) -> &EnElf {
    let h = w.player().navi_actor.expect("naviActor");
    w.actors.downcast::<EnElf>(h).expect("Navi is an En_Elf")
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

#[test]
fn player_init_spawns_navi_and_she_goes_into_links_hat() {
    let Some(a) = assets() else { return };
    // Kokiri Forest's spawn 3, Link's porch: nothing targetable in range.
    let Some(mut w) = enter(&a, "ENTR_SPOT04_3") else { return };
    let p = w.player().actor.world_pos;
    let n = navi(&w);
    assert_eq!(n.actor.params, FAIRY_NAVI);
    assert_eq!((n.actor.room, n.actor.scale, n.fairy_flags, n.unk_2c7, n.update_fn, n.action), (-1, Vec3::splat(0.008), 4, 0x14, en_elf::Update::Navi, en_elf::Action::Navi));
    // Player_SpawnFairy: world.pos + D_80854778 (0, 50, 0) turned by the shape's yaw.
    assert_eq!(n.actor.world_pos, p + Vec3::new(0.0, 50.0, 0.0));
    // Mode 0 (func_80A01C38(0)): unk_2C0 100.
    assert_eq!((n.unk_2a8, n.unk_2c0), (0, 100));
    // PLAYER_STATE2_20 clear: into the hat on the first frame, there within unk_2AE's 30.
    let mut into_hat = None;
    for f in 1..60 {
        // Navi's update reads bodyPartsPos from the last frame's draw.
        let hat = w.player().body_parts_pos[PLAYER_BODYPART_HAT];
        idle(&mut w, 1);
        let n = navi(&w);
        assert!(w.target_ctx.arrow_pointed.is_none());
        if n.unk_2a8 == 7 && into_hat.is_none() {
            into_hat = Some(f);
            assert_eq!(w.player().state2 & PLAYER_STATE2_20, 0, "PLAYER_STATE2_20 off as she vanishes");
        }
        if n.unk_2a8 == 8 {
            let started = into_hat.expect("mode 7 before 8");
            assert_eq!(started, 1, "into the hat on the first frame");
            assert!(f - started <= 31, "in the hat {} frames later", f - started);
            // func_80A03CF8's case 8: at the hat, scale 0, not drawn.
            assert_eq!(n.actor.world_pos, hat);
            assert_eq!(n.actor.scale.x, 0.0);
            assert_eq!(n.render_state().switches[3], 0, "EnElf_Draw skips mode 8");
            return;
        }
    }
    panic!("Navi never went into the hat");
}

#[test]
fn c_up_talks_to_navi_once_she_has_her_text() {
    let Some(a) = assets() else { return };
    let Some(mut w) = enter(&a, "ENTR_SPOT04_3") else { return };
    // EnElf_Init: naviTimer below 3000 starts again at 0; func_80A053F0 counts it outside
    // cutscenes.
    assert_eq!(w.save.navi_timer, 0);
    // Before 600: no text. (It doesn't count through the entrance's walk, Play_InCsMode.)
    idle(&mut w, 100);
    assert!(w.save.navi_timer > 50 && w.save.navi_timer < 100, "naviTimer {}", w.save.navi_timer);
    assert_eq!(w.player().navi_text_id, 0);
    // At 600, her C-Up text: EVENTCHKINF_05 unset, 0x40 | 0x100.
    while w.save.navi_timer < 600 {
        idle(&mut w, 1);
    }
    idle(&mut w, 1);
    assert_eq!(w.player().navi_text_id, 0x140);
    // Player: STATE2_21 (func_8083B644 found her), then on the next update Navi's call on the
    // HUD (func_808473D4's Interface_SetNaviCall 0x1D, early in Player_UpdateCommon).
    idle(&mut w, 1);
    assert_ne!(w.player().state2 & oot_actors::player::STATE2_21, 0);
    assert!(!w.interface_ctx.navi_calling);
    idle(&mut w, 1);
    assert!(w.interface_ctx.navi_calling);
    // C-Up: the talk with her.
    w.tick_with(scripted_input(PadState::default(), PadState { button: BTN_CUP, ..Default::default() }));
    let mut prev = PadState { button: BTN_CUP, ..Default::default() };
    let mut opened = false;
    for _ in 0..40 {
        if w.message_state() != TEXT_STATE_NONE {
            opened = true;
            break;
        }
        w.tick_with(scripted_input(prev, PadState::default()));
        prev = PadState::default();
    }
    assert!(opened, "her text opens");
    assert_eq!(w.msg_ctx.text_id, 0x140);
    let n = navi(&w);
    assert_eq!(n.update_fn, en_elf::Update::Talk, "func_80A052F4");
    assert_eq!(n.fairy_flags & 0x80, 0x80, "her C-Up text: fairyFlags 0x80");
    assert_eq!(w.save.navi_timer, 3001);
    assert_eq!(w.player().action, Action::Talk);
    // func_80835EA4(play, 0xB): the camera turns round.
    assert_eq!(w.game_camera.setting, oot_game::camera::CAM_SET_TURN_AROUND);
    // A through it to its end.
    for _ in 0..600 {
        if w.message_state() == TEXT_STATE_NONE && w.player().action != Action::Talk {
            break;
        }
        let st = w.message_state();
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT) || w.msg_ctx.msg_mode == MSGMODE_TEXT_AWAIT_INPUT;
        let pad = if waits && prev.button == 0 { PadState { button: BTN_A, ..Default::default() } } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
    }
    let n = navi(&w);
    assert_eq!((n.update_fn, n.unk_2a8, n.fairy_flags & 0x20), (en_elf::Update::Navi, 0, 0));
    // naviTimer 3001 and on: no more text until it runs out at 25800.
    idle(&mut w, 20);
    assert_eq!(w.player().navi_text_id, 0);
    assert!(w.save.navi_timer > 3001);
}

#[test]
fn the_kokiri_fairies_bob_above_their_children() {
    let Some(a) = assets() else { return };
    let Some(mut w) = enter(&a, "ENTR_SPOT04_3") else { return };
    idle(&mut w, 80);
    let fairies: Vec<&EnElf> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnElf>(h)).filter(|e| e.actor.params == FAIRY_KOKIRI).collect();
    assert_eq!(fairies.len(), 9, "the 8 children's and Mido's");
    for f in fairies {
        let parent = f.actor.parent.and_then(|h| w.actors.actor(h)).expect("its child");
        // func_80A0353C: towards the parent's position raised 1500 × scale.y + 40, plus the
        // circle's drift (func_80A02A20: 20 around, 5 up and down).
        let target = parent.world_pos + Vec3::new(0.0, 1500.0 * f.actor.scale.y + 40.0, 0.0);
        assert!(f.actor.world_pos.distance(target) < 30.0, "{} from its parent's point", f.actor.world_pos.distance(target));
        // EnElf_Init: sColorFlags[1..12]: the outer colour's channels 0, 200..255 or 0..255,
        // alpha 0; the inner white.
        assert_eq!((f.inner_color, f.outer_color[3]), ([255.0; 4], 0.0));
        assert!(f.outer_color[..3].iter().all(|&c| (0.0..=255.0).contains(&c)) && f.outer_color[..3].iter().any(|&c| c >= 200.0));
        assert_eq!(f.fairy_flags & 4, 0, "not Navi's bake");
    }
}

#[test]
fn navi_follows_her_cue_in_the_deku_trees_talk() {
    use oot_game::cutscene::CS_STATE_IDLE;
    let Some(a) = assets() else { return };
    // In front of the tree: his first talk (D_808BCE20) starts by itself.
    let Some(mut w) = enter(&a, "ENTR_SPOT04_1") else { return };
    let mut checked = 0;
    for _ in 0..200 {
        // Navi updates before the frame's script commands (func_800645A0 after Actor_UpdateAll):
        // she follows the cue the last frame left.
        let (state, cue, frames) = (w.cs_ctx.state, w.cs_ctx.npc_actions[8], w.cs_ctx.frames);
        idle(&mut w, 1);
        if state == CS_STATE_IDLE || w.cs_ctx.state == CS_STATE_IDLE {
            continue;
        }
        let Some(cue) = cue else { continue };
        let n = navi(&w);
        // func_80A0461C: cue 4 → mode 9, 3 → 6, 1 → 10, else 0.
        let mode = match cue.action {
            4 => 9,
            3 => 6,
            1 => 10,
            _ => 0,
        };
        assert_eq!(n.unk_2a8, mode, "cue {}", cue.action);
        if mode == 10 {
            // func_80A02EC0: x and z on the cue's point (EnElf_GetCutsceneNextPos) plus the drift.
            let (s, e) = (cue.start_pos.as_vec3(), cue.end_pos.as_vec3());
            let t = oot_game::env::lerp_weight(cue.end_frame, cue.start_frame, frames);
            let at = Vec3::new((e.x - s.x) * t + s.x, 0.0, (e.z - s.z) * t + s.z);
            assert!((n.actor.world_pos.x - (at.x + n.unk_28c.x)).abs() < 1e-3 && (n.actor.world_pos.z - (at.z + n.unk_28c.z)).abs() < 1e-3);
        }
        checked += 1;
    }
    assert!(checked > 50, "{checked} frames with her cue");
}

#[test]
fn the_fairies_bakes_are_in_the_pack() {
    let Ok(pack) = oot_game::pack::GamePack::open_default() else { return };
    for b in [en_elf::BAKE_NAVI, en_elf::BAKE_FAIRY] {
        let d: eng_gfx::DrawList = pack.assets.get(&oot_game::pack::keys::bake(b)).expect(b);
        assert!(d.triangle_count() >= 10, "{b}: the glow and four wings");
        assert!(d.stats.unresolved_addresses.is_empty(), "{b}");
    }
}
