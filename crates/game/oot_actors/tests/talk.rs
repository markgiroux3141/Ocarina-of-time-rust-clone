//! Talking and the message box, in Kokiri Forest after `Play_Init`: Player's side
//! (`func_8083B644`, `func_80853148`, `func_8083A2F8`, `func_8084B530` in `z_player.c`), the
//! actors' (`func_8002F2CC`, `Actor_ProcessTalkRequest`, `Actor_TextboxIsClosing`) and the box's
//! (`z_message_PAL.c`). Expected values come from the C and the scene's actor list:
//! - the sign at (49, -80, 967), rotation 0x8000, params 0x031F: `EnKanban_Init` gives it
//!   text `params | 0x300`; it offers within 68 when Link is in front of it (`EnKanban_Message`).

mod common;

use std::sync::Arc;

use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_kanban::EnKanban;
use oot_actors::player::Action;
use oot_game::actor::ACTOR_FLAG_8;
use oot_game::actor_ctx::ActorHandle;
use oot_game::message::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn enter(entrance: &str) -> Option<PlayState> {
    let a = assets()?;
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    Some(oot_actors::play_entrance(a, common::data()?, common::rules()?, save).expect("Play_Init"))
}

fn frames(w: &mut PlayState, prev: &mut PadState, pad: PadState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(*prev, pad));
        *prev = pad;
    }
}

const NONE: PadState = PadState { button: 0, stick_x: 0, stick_y: 0 };
const A: PadState = PadState { button: BTN_A, stick_x: 0, stick_y: 0 };

/// Kokiri Forest settled, Link placed at `pos` facing `yaw`, one frame for the actors to see
/// him.
fn at(pos: Vec3, yaw: i16) -> Option<(PlayState, PadState)> {
    let mut w = enter("ENTR_SPOT04_0")?;
    let mut prev = PadState::default();
    // Play_Init's fade-in, and the HUD's fade back after it: Camera_Init's D_8011D3F0 set alpha
    // type 2 for the first three frames, which the transition's 0xF200 then kept.
    frames(&mut w, &mut prev, NONE, 60);
    w.place_player(pos, yaw);
    frames(&mut w, &mut prev, NONE, 2);
    Some((w, prev))
}

fn sign_at(w: &PlayState, pos: Vec3) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnKanban>(h).is_some_and(|k| k.actor.home_pos.distance(pos) < 1.0)).expect("the sign")
}

#[test]
fn the_sign_by_links_house_shows_its_text_and_closes_on_a() {
    let sign_pos = Vec3::new(49.0, -80.0, 967.0);
    // In front of the sign (it faces -z), 40 away, facing it.
    let Some((mut w, mut prev)) = at(Vec3::new(49.0, -80.0, 927.0), 0) else { return };
    let sign = sign_at(&w, sign_pos);
    assert_eq!(w.actors.actor(sign).unwrap().text_id, 0x031F, "params | 0x300");
    // func_8002F2CC last frame: the sign is Player's targetActor, so func_8083B644 sets
    // PLAYER_STATE2_1, and func_808473D4 (which runs before the interrupts, so a frame later)
    // shows "Check" (the sign isn't an NPC).
    assert_eq!(w.player().target_actor, Some(sign));
    frames(&mut w, &mut prev, NONE, 1);
    assert_eq!(w.interface_ctx.unk_1f0, oot_game::interface::DO_ACTION_CHECK);
    assert_eq!(w.message_state(), TEXT_STATE_NONE);

    // A: func_80853148 → func_8083A2F8 at once (not an NPC): the talk action, Player's text,
    // the sign's talk request (answered in its update this frame), Message_StartTextbox.
    frames(&mut w, &mut prev, A, 1);
    let p = w.player();
    assert_eq!(p.action, Action::Talk);
    assert_eq!((p.actor.flags & ACTOR_FLAG_8, p.actor.text_id, p.unk_664), (ACTOR_FLAG_8, 0x031F, Some(sign)));
    assert!(w.actors.downcast::<EnKanban>(sign).unwrap().msg_flag);
    let m = &w.msg_ctx;
    assert_eq!((m.text_id, m.talk_actor, m.msg_mode), (0x031F, Some(sign), MSGMODE_TEXT_START));
    // typePos 0x10: TEXTBOX_TYPE_WOODEN, TEXTBOX_POS_VARIABLE; Message_OpenText's colours.
    assert_eq!((m.text_box_type, m.text_box_pos), (TEXTBOX_TYPE_WOODEN, 0));
    assert_eq!((m.textbox_color, m.textbox_color_alpha_target, m.textbox_color_alpha_current), ([70, 50, 30], 230, 0));

    // MSGMODE_TEXT_START waits until D_8014B2F4 reaches 4 in a SCENE_CAM_TYPE_DEFAULT scene,
    // then places the box and starts growing it.
    frames(&mut w, &mut prev, NONE, 2);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_TEXT_START);
    frames(&mut w, &mut prev, NONE, 1);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_TEXT_BOX_GROWING);
    // A variable box goes to the bottom (142) when the midpoint of Player's and the sign's
    // screen heights is above XREG(94) = 160; the arrow 59 below its top.
    let r = &w.msg_ctx.regs;
    assert_eq!((r.textbox_x_target, r.textbox_y_target, r.textbox_end_ypos), (34, 142, 201));
    // The talk camera (CAM_MODE_TALK, on the sign) while Player has ACTOR_FLAG_8.
    assert_eq!(w.game_camera.mode, oot_game::camera::CAM_MODE_TALK);
    assert_eq!(w.game_camera.target, Some(sign));

    // Message_GrowTextbox: eight frames, then the full box, faded in.
    frames(&mut w, &mut prev, NONE, 7);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_TEXT_BOX_GROWING);
    frames(&mut w, &mut prev, NONE, 1);
    let m = &w.msg_ctx;
    assert_eq!(m.msg_mode, MSGMODE_TEXT_STARTING);
    assert_eq!((m.regs.textbox_x, m.regs.textbox_y, m.regs.textbox_width, m.regs.textbox_height, m.regs.textbox_texwidth), (34, 142, 256, 64, 512));
    assert_eq!(m.textbox_color_alpha_current, 230);
    let boxes = m.sprites.iter().filter(|s| s.name == box_sprite(TEXTBOX_TYPE_WOODEN)).count();
    assert_eq!(boxes, 1);

    // MSGMODE_TEXT_STARTING: "Next" on A. Then Message_Decode.
    frames(&mut w, &mut prev, NONE, 1);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_TEXT_NEXT_MSG);
    assert_eq!(w.interface_ctx.unk_1f0, oot_game::interface::DO_ACTION_NEXT);
    frames(&mut w, &mut prev, NONE, 1);
    let m = &w.msg_ctx;
    assert_eq!(m.msg_mode, MSGMODE_TEXT_DISPLAYING);
    // QUICKTEXT_ENABLE at the first drawn position jumps textDrawPos past everything up to
    // QUICKTEXT_DISABLE (15), and the frame ends at 16: nothing but the box is drawn yet.
    assert_eq!(m.text_draw_pos, 16);
    assert_eq!(m.sprites.len(), 1);
    // One line: the text starts 26 below the box's top.
    assert_eq!(m.regs.text_init_ypos, 142 + 26);

    // Next frame: all of it. SHIFT 0x1E, then the name ("LINK") and "'s House", each glyph a
    // shadow one pixel down and right, then the glyph, 12 pixels (R_TEXT_CHAR_SCALE 75).
    frames(&mut w, &mut prev, NONE, 1);
    let m = &w.msg_ctx;
    let glyphs: Vec<_> = m.sprites.iter().filter(|s| s.name.starts_with("message/char")).collect();
    assert_eq!(glyphs.len(), 2 * 11);
    let l = oot_game::sprite::rect_transform(95.0, 168.0, 107.0, 180.0);
    assert_eq!((glyphs[1].name.as_str(), glyphs[1].transform), ("message/char4C", l));
    assert_eq!(glyphs[0].transform, oot_game::sprite::rect_transform(96.0, 169.0, 108.0, 181.0));
    assert_eq!(glyphs[0].prim, Some([0, 0, 0, 255]));
    assert_eq!(glyphs[1].prim, Some([255, 255, 255, 255]));
    // sFontWidths at 75%: 'L' 8 → 6, 'I' 4 → 3, 'N' 11 → 8, 'K' 10 → 7.
    let x = |i: usize| glyphs[2 * i + 1].transform.w_axis.x + 160.0;
    assert_eq!([x(0), x(1), x(2), x(3)], [95.0, 101.0, 104.0, 112.0]);

    // MESSAGE_END: done, the square icon, "Return" on A; the icon from the next frame.
    frames(&mut w, &mut prev, NONE, 1);
    assert_eq!((w.msg_ctx.msg_mode, w.msg_ctx.font.icon), (MSGMODE_TEXT_DONE, TEXTBOX_ICON_SQUARE));
    assert_eq!(w.message_state(), TEXT_STATE_DONE);
    assert_eq!(w.interface_ctx.unk_1f0, oot_game::interface::DO_ACTION_RETURN);
    frames(&mut w, &mut prev, NONE, 1);
    let icon = w.msg_ctx.sprites.iter().find(|s| s.name == icon_sprite(TEXTBOX_ICON_SQUARE)).expect("the icon");
    assert_eq!(icon.transform, oot_game::sprite::rect_transform(158.0, 201.0, 170.0, 213.0));
    assert_eq!(w.player().action, Action::Talk);

    // A closes it: MSGMODE_TEXT_CLOSING for two frames (the box goes at once), and
    // TEXT_STATE_CLOSING on the second, when Player stands and the sign waits 20 frames.
    frames(&mut w, &mut prev, A, 1);
    assert_eq!(w.msg_ctx.msg_mode, MSGMODE_TEXT_CLOSING);
    assert!(w.msg_ctx.sprites.is_empty());
    frames(&mut w, &mut prev, NONE, 1);
    assert_eq!(w.message_state(), TEXT_STATE_CLOSING);
    assert_eq!(w.player().action, Action::Talk);
    frames(&mut w, &mut prev, NONE, 1);
    let p = w.player();
    assert_eq!((p.action, p.actor.flags & ACTOR_FLAG_8, p.unk_88E), (Action::StandingStill, 0, 10));
    let k = w.actors.downcast::<EnKanban>(sign).unwrap();
    assert_eq!((k.msg_flag, k.msg_timer), (false, 20));
    assert_eq!((w.message_state(), w.msg_ctx.msg_mode), (TEXT_STATE_NONE, MSGMODE_NONE));

    // unk_88E masks A for 10 frames, and the sign doesn't offer for 20: A does nothing.
    frames(&mut w, &mut prev, A, 1);
    assert_eq!(w.player().action, Action::StandingStill);
    frames(&mut w, &mut prev, NONE, 21);
    assert_eq!(w.player().target_actor, Some(sign), "the sign offers again");
    frames(&mut w, &mut prev, A, 1);
    assert_eq!(w.player().action, Action::Talk);
}

#[test]
fn the_talk_camera_swings_in_over_ten_frames_then_holds() {
    // sSetNormal0ModeTalkData: CAM_FUNCDATA_KEEP3(-30, 70, 200, 40, 10, 0, 5, 70, 45, 50, 10,
    // 0x3500): initTimer 10, fov 45, flags 0x3500 (32-row bars).
    let Some((mut w, mut prev)) = at(Vec3::new(49.0, -80.0, 927.0), 0) else { return };
    frames(&mut w, &mut prev, NONE, 1);
    let (eye0, at0) = (w.game_camera.eye, w.game_camera.at);
    // The A frame: Player asks for TALK; Camera_KeepOn3's first call asks for the update after
    // Play_Draw (view.unk_124), which sets the swing up. Neither moves the camera.
    frames(&mut w, &mut prev, A, 1);
    assert_eq!(w.game_camera.mode, oot_game::camera::CAM_MODE_TALK);
    assert_eq!(w.game_camera.view_unk_124, 0, "done by the end of the frame");
    assert_eq!((w.game_camera.eye, w.game_camera.at), (eye0, at0));
    // Ten frames: `at` goes 1/animTimer of the way to its target each frame, so the steps
    // shrink in a fixed ratio along one line; the eye follows at its own rate.
    let mut ats = vec![w.game_camera.at];
    let mut eyes = vec![w.game_camera.eye];
    for _ in 0..10 {
        frames(&mut w, &mut prev, NONE, 1);
        ats.push(w.game_camera.at);
        eyes.push(w.game_camera.eye);
    }
    let target = ats[10];
    for k in 0..10 {
        // at_{k+1} = at_k + (target - at_k) / (10 - k).
        let want = ats[k] + (target - ats[k]) / (10 - k) as f32;
        assert!(ats[k + 1].distance(want) < 0.01, "step {k}: {:?} vs {want:?}", ats[k + 1]);
    }
    assert!(eyes[0].distance(eyes[10]) > 1.0, "the eye swung");
    assert_eq!(w.game_camera.fov, 45.0);
    // Then it holds while the text is up; the bars reached 32.
    frames(&mut w, &mut prev, NONE, 3);
    assert_eq!((w.game_camera.eye, w.game_camera.at), (eyes[10], target));
    assert_eq!(w.letterbox.rows(), 32);
}

#[test]
fn a_kokiri_child_talks_and_remembers_it() {
    use oot_actors::en_ko::EnKo;
    // Child 1 (params 0xFF01) at (45, 0, -272), facing +z; Link 40 in front, facing it.
    let Some((mut w, mut prev)) = at(Vec3::new(45.0, 0.0, -232.0), -0x8000) else { return };
    let child = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnKo>(h).is_some_and(|k| k.actor.params & 0xFF == 1)).expect("child 1");
    frames(&mut w, &mut prev, NONE, 1);
    assert_eq!(w.player().target_actor, Some(child));
    // func_80A97610: 0x1005 until INFTABLE_1E; "Speak" (an NPC).
    assert_eq!(w.actors.actor(child).unwrap().text_id, 0x1005);
    assert_eq!(w.interface_ctx.unk_1f0, oot_game::interface::DO_ACTION_SPEAK);

    // A: an NPC is talked to after the item is put away (func_80836898, then func_8083A2F8 the
    // next frame); its talk request is answered at once (func_800343CC: unk_1E8.unk_00 = 1).
    frames(&mut w, &mut prev, A, 1);
    assert_eq!(w.player().action, Action::ItemPutAway);
    assert_eq!(w.actors.downcast::<EnKo>(child).unwrap().unk_1e8.talk_state, 1);
    frames(&mut w, &mut prev, NONE, 1);
    assert_eq!(w.player().action, Action::Talk);
    assert_eq!((w.msg_ctx.text_id, w.msg_ctx.talk_actor), (0x1005, Some(child)));

    // Read it through: A whenever a box waits.
    let mut n = 0;
    while w.message_state() != TEXT_STATE_NONE || w.player().action == Action::Talk {
        let waiting = matches!(w.message_state(), TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE);
        let pad = if waiting && prev.button == 0 { A } else { NONE };
        frames(&mut w, &mut prev, pad, 1);
        n += 1;
        assert!(n < 1000, "the talk ends");
    }
    // func_80A97738 on TEXT_STATE_CLOSING: SET_INFTABLE(INFTABLE_1E), and the talk state 0.
    assert!(w.save.get_inf_table(0x1E));
    assert_eq!(w.actors.downcast::<EnKo>(child).unwrap().unk_1e8.talk_state, 0);
    frames(&mut w, &mut prev, NONE, 12);
    assert_eq!(w.actors.actor(child).unwrap().text_id, 0x1006, "the second text");
}

#[test]
fn the_spot_by_the_window_in_links_house_is_checked() {
    use oot_actors::en_wonder_talk2::{self, EnWonderTalk2};
    // En_Wonder_Talk2 params 0x8ABF at (78, 38, 116), rotation -29127: talk mode 2 (check
    // only), text 0x200 | 0x2A, no switch flag, rot.z 0 (range 0: offers within 50, from 40).
    let Some(mut w) = enter("ENTR_LINK_HOME_0") else { return };
    let mut prev = PadState::default();
    frames(&mut w, &mut prev, NONE, 40);
    let spot = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnWonderTalk2>(h).is_some()).expect("the spot");
    let s = w.actors.downcast::<EnWonderTalk2>(spot).unwrap();
    assert_eq!((s.talk_mode, s.base_msg_id, s.switch_flag, s.trigger_range, s.action), (2, 0x2A, -1, 0.0, en_wonder_talk2::Action::Offer));
    assert_eq!(s.actor.text_id, 0x22A);
    // 30 in front of it, facing it.
    let yaw: i16 = -29127;
    let dir = Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw));
    let pos = Vec3::new(78.0, 0.0, 116.0) + dir * 30.0;
    w.place_player(pos, yaw.wrapping_add(-0x8000i32 as i16));
    frames(&mut w, &mut prev, NONE, 3);
    assert_eq!(w.player().target_actor, Some(spot));
    assert_eq!(w.interface_ctx.unk_1f0, oot_game::interface::DO_ACTION_CHECK, "not an NPC");
    frames(&mut w, &mut prev, A, 1);
    assert_eq!(w.player().action, Action::Talk);
    assert_eq!(w.msg_ctx.text_id, 0x22A);
    assert_eq!(w.game_camera.mode, oot_game::camera::CAM_MODE_TALK);
    // The spot takes the request and sets its text again (func_80B3A10C), then offers again.
    frames(&mut w, &mut prev, NONE, 2);
    assert_eq!(w.actors.downcast::<EnWonderTalk2>(spot).unwrap().action, en_wonder_talk2::Action::Offer);
}


#[test]
fn talking_fades_the_hud_to_the_a_button_and_the_hearts() {
    let Some((mut w, mut prev)) = at(Vec3::new(49.0, -80.0, 927.0), 0) else { return };
    // Play_Init's fade-in (alpha type 50 after the transition's 2) is over: everything shown.
    let c = &w.interface_ctx;
    assert_eq!((c.a_alpha, c.b_alpha, c.c_left_alpha, c.health_alpha, w.save.unk_13e8), (255, 255, 255, 255, 0));
    frames(&mut w, &mut prev, NONE, 1);
    frames(&mut w, &mut prev, A, 1);
    // The talk camera (KEEP3's interface flags 0x3500): Camera_UpdateInterface's
    // Interface_ChangeAlpha(5), after this frame's Interface_Update.
    assert_eq!((w.game_camera.interface_flags as u16, w.save.unk_13e8, w.save.unk_13ec), (0x3500, 5, 1));
    // func_80082850 case 5, maxAlpha 255 - (unk_13EC << 5) and alpha 255 - maxAlpha:
    // func_8008277C clamps B, A and the C buttons to maxAlpha, then A (no longer 255) takes
    // alpha, so it dips and comes back while B and C fade out; the hearts stay.
    let mut seen = Vec::new();
    for _ in 0..9 {
        frames(&mut w, &mut prev, NONE, 1);
        let c = &w.interface_ctx;
        seen.push((c.a_alpha, c.b_alpha, c.c_left_alpha, c.health_alpha));
    }
    let expected: Vec<(i16, i16, i16, i16)> = (1..=8).map(|k: i16| ((32 * k).min(255), (255 - 32 * k).max(0), (255 - 32 * k).max(0), 255)).chain([(255, 0, 0, 255)]).collect();
    assert_eq!(seen, expected);
    assert_eq!(w.save.unk_13e8, 0, "done once alpha reaches 0");
}
