//! The helpers `z_actor.c` gives NPCs: offering to talk (`func_8002F1C4` and its short forms),
//! answering a talk request (`Actor_ProcessTalkRequest`), the per-frame talk handler
//! (`func_800343CC`), the head and torso turning towards a point (`func_80034A14`,
//! `func_800344BC`, `func_80034810` with the `D_80116130` presets), and the idle limb sway
//! (`func_80034F54`), and the fade by distance (`func_80034DD4`).
//!
//! Player takes an offer on A (`func_8083B644`, in `oot_actors::player`): the actor gets
//! `ACTOR_FLAG_8` and the message box opens with its `textId` (`crate::message`).

use glam::Vec3;

use crate::actor::{ACTOR_FLAG_8, Actor};
use crate::play::PlayState;
use crate::target::{pitch_to, yaw_to};

/// `EXCH_ITEM_NONE`.
pub const EXCH_ITEM_NONE: u8 = 0;

/// `Actor_ProcessTalkRequest`: did Player accept this actor's talk offer (`ACTOR_FLAG_8`)?
pub fn process_talk_request(actor: &mut Actor) -> bool {
    if actor.flags & ACTOR_FLAG_8 != 0 {
        actor.flags &= !ACTOR_FLAG_8;
        return true;
    }
    false
}

/// `func_8002F1C4`: the updating actor (`play.cur_actor`) offers to talk if it's the nearest
/// offer within `xz_range` and `y_range` this frame (or Player targets it), recording itself in
/// Player's `targetActor`.
pub fn offer_talk_range(play: &mut PlayState, actor: &Actor, xz_range: f32, y_range: f32, exchange_item: u8) -> bool {
    let (Some(ph), Some(me)) = (play.player, play.cur_actor) else { return false };
    let Some(p) = play.actors.get_mut(ph) else { return false };
    let player_flag8 = p.base().flags & ACTOR_FLAG_8 != 0;
    // Player_InCsMode: never (no cutscenes).
    let Some(pi) = p.as_player_mut() else { return false };
    let (_, target_dist) = pi.talk_target();
    if player_flag8 || (!actor.is_targeted && (y_range < actor.y_dist_to_player.abs() || target_dist < actor.xz_dist_to_player || xz_range < actor.xz_dist_to_player)) {
        return false;
    }
    pi.set_talk_target(me, actor.xz_dist_to_player, exchange_item);
    true
}

/// `func_8002F298`.
pub fn offer_talk_exchange(play: &mut PlayState, actor: &Actor, range: f32, exchange_item: u8) -> bool {
    offer_talk_range(play, actor, range, range, exchange_item)
}

/// `func_8002F2CC`.
pub fn offer_talk(play: &mut PlayState, actor: &Actor, range: f32) -> bool {
    offer_talk_exchange(play, actor, range, EXCH_ITEM_NONE)
}

/// `func_8002F2F4`: within 50 plus the actor's `cylRadius`.
pub fn offer_talk_default(play: &mut PlayState, actor: &Actor) -> bool {
    let r = 50.0 + actor.col_chk_info.cyl_radius as f32;
    offer_talk(play, actor, r)
}

/// `Actor_TextboxIsClosing`: the message box is on its closing frame (`TEXT_STATE_CLOSING`).
pub fn textbox_is_closing(play: &PlayState) -> bool {
    play.message_state() == crate::message::TEXT_STATE_CLOSING
}

/// `func_800343CC`: an NPC's talk handling for one frame. `talk_state` is the NPC's
/// `unk_1E8.unk_00` (0: not talking). When a request was accepted it becomes 1; while talking,
/// `update` advances it (`unkFunc2`); otherwise, on screen and in range, the NPC offers to talk
/// and picks its text (`text`, `unkFunc1`). True when a talk starts.
pub fn talk_update(play: &mut PlayState, actor: &mut Actor, talk_state: &mut i16, interact_range: f32, text: impl FnOnce(&PlayState, &Actor) -> u16, update: impl FnOnce(&mut PlayState, &mut Actor) -> i16) -> bool {
    if process_talk_request(actor) {
        *talk_state = 1;
        return true;
    }
    if *talk_state != 0 {
        *talk_state = update(play, actor);
        return false;
    }
    let (x, y) = crate::target::actor_screen_pos(play.view_proj, actor);
    if !(0..=320).contains(&x) || !(0..=240).contains(&y) {
        return false;
    }
    if !offer_talk(play, actor, interact_range) {
        return false;
    }
    actor.text_id = text(play, actor);
    false
}

/// `struct_80034A14_arg1`: an NPC's talk state and head / torso tracking.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NpcTrack {
    /// `unk_00`: the talk state (`func_800343CC`).
    pub talk_state: i16,
    /// `unk_02`: this frame's tracking mode (1 none, 2 head, 3 head and torso partly, 4 full).
    pub mode: i16,
    /// `unk_04`: frames until the idle mode changes; `unk_06`: the idle mode's step.
    pub timer: i16,
    pub idle_step: i16,
    /// `unk_08`: the head's (x pitch, y yaw, z) turn; `unk_0E`: the torso's.
    pub head: [i16; 3],
    pub torso: [i16; 3],
    /// `unk_14`: the height the tracking looks from, above the actor's position.
    pub eye_height: f32,
    /// `unk_18`: the point to look at (Player, or the camera in a cutscene).
    pub target: Vec3,
}

/// `struct_80116130_0`: a tracking preset. `func_80034A14` passes it to `func_800344BC` as
/// (`unk_00` head yaw, `unk_04` head pitch max, `unk_02` head pitch min, `unk_06` torso yaw,
/// `unk_0A` torso pitch max, `unk_08` torso pitch min, `unk_0C` turn the body).
#[derive(Debug, Clone, Copy)]
#[allow(non_snake_case)]
struct Limits {
    unk_00: i16,
    unk_02: i16,
    unk_04: i16,
    unk_06: i16,
    unk_08: i16,
    unk_0A: i16,
    unk_0C: u8,
}

/// `D_80116130`: the presets (`sub_00`, `unk_10` the distance, `unk_14` the yaw range).
const D_80116130: [(Limits, f32, i16); 13] = {
    const fn l(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u8) -> Limits {
        Limits { unk_00: a as i16, unk_02: b as i16, unk_04: c as i16, unk_06: d as i16, unk_08: e as i16, unk_0A: f as i16, unk_0C: g }
    }
    [
        (l(0x2AA8, 0xF1C8, 0x18E2, 0x1554, 0x0000, 0x0000, 1), 170.0, 0x3FFC),
        (l(0x2AA8, 0xEAAC, 0x1554, 0x1554, 0xF8E4, 0x0E38, 1), 170.0, 0x3FFC),
        (l(0x31C4, 0xE390, 0x0E38, 0x0E38, 0xF1C8, 0x071C, 1), 170.0, 0x3FFC),
        (l(0x1554, 0xF1C8, 0x0000, 0x071C, 0xF8E4, 0x0000, 1), 170.0, 0x3FFC),
        (l(0x2AA8, 0xF8E4, 0x071C, 0x0E38, 0xD558, 0x2AA8, 1), 170.0, 0x3FFC),
        (l(0x0000, 0xE390, 0x2AA8, 0x3FFC, 0xF1C8, 0x0E38, 1), 170.0, 0x3FFC),
        (l(0x2AA8, 0xF1C8, 0x0E38, 0x0E38, 0x0000, 0x0000, 1), 0.0, 0x0000),
        (l(0x2AA8, 0xF1C8, 0x0000, 0x0E38, 0x0000, 0x1C70, 1), 0.0, 0x0000),
        (l(0x2AA8, 0xF1C8, 0xF1C8, 0x0000, 0x0000, 0x0000, 1), 0.0, 0x0000),
        (l(0x071C, 0xF1C8, 0x0E38, 0x1C70, 0x0000, 0x0000, 1), 0.0, 0x0000),
        (l(0x0E38, 0xF1C8, 0x0000, 0x1C70, 0x0000, 0x0E38, 1), 0.0, 0x0000),
        (l(0x2AA8, 0xE390, 0x1C70, 0x0E38, 0xF1C8, 0x0E38, 1), 0.0, 0x0000),
        (l(0x18E2, 0xF1C8, 0x0E38, 0x0E38, 0x0000, 0x0000, 1), 0.0, 0x0000),
    ]
};

/// The C `CLAMP(x, min, max)`: `x < min ? min : (x > max ? max : x)`, in `int`.
fn clamp_c(x: i32, min: i32, max: i32) -> i32 {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

/// `ABS(x) >= 0x8000 ? 0 : ABS(x)` for an `s16` (only -0x8000 gives 0).
fn abs_or_zero(v: i16) -> i32 {
    let a = (v as i32).abs();
    if a >= 0x8000 { 0 } else { a }
}

/// `func_800344BC`: the head turns towards the target within its limits, the torso takes what
/// the head can't cover, and the whole body turns too if `arg8`.
#[allow(clippy::too_many_arguments)]
fn func_800344bc(actor: &mut Actor, t: &mut NpcTrack, arg2: i16, arg3: i16, arg4: i16, arg5: i16, arg6: i16, arg7: i16, arg8: u8) {
    use eng_math::smooth_step_to_s;
    let sp30 = Vec3::new(actor.world_pos.x, actor.world_pos.y + t.eye_height, actor.world_pos.z);
    let sp46 = pitch_to(sp30, t.target);
    let sp44 = yaw_to(sp30, t.target);
    let mut sp40 = yaw_to(actor.world_pos, t.target).wrapping_sub(actor.shape_rot.y);

    let temp1 = clamp_c(sp40 as i32, -(arg2 as i32), arg2 as i32) as i16;
    smooth_step_to_s(&mut t.head[1], temp1, 6, 2000, 1);
    let temp1 = abs_or_zero(sp40);
    t.head[1] = clamp_c(t.head[1] as i32, -temp1, temp1) as i16;
    sp40 = sp40.wrapping_sub(t.head[1]);

    let temp1 = clamp_c(sp40 as i32, -(arg5 as i32), arg5 as i32) as i16;
    smooth_step_to_s(&mut t.torso[1], temp1, 6, 2000, 1);
    let temp1 = abs_or_zero(sp40);
    t.torso[1] = clamp_c(t.torso[1] as i32, -temp1, temp1) as i16;

    if arg8 != 0 {
        smooth_step_to_s(&mut actor.shape_rot.y, sp44, 6, 2000, 1);
    }

    let temp1 = clamp_c(sp46 as i32, arg4 as i32, arg3 as u16 as i16 as i32) as i16;
    smooth_step_to_s(&mut t.head[0], temp1, 6, 2000, 1);
    let temp2 = sp46.wrapping_sub(t.head[0]);
    let temp1 = clamp_c(temp2 as i32, arg7 as i32, arg6 as i32) as i16;
    smooth_step_to_s(&mut t.torso[0], temp1, 6, 2000, 1);
}

/// `func_80034810`: which tracking mode this frame. `forced` (`arg4`) wins; talking tracks
/// fully; beyond `dist` nothing; within `yaw_range` of facing the head only; otherwise the
/// idle glances (`Rand_S16Offset` timers).
fn func_80034810(play: &mut PlayState, actor: &Actor, t: &mut NpcTrack, dist: f32, yaw_range: i16, forced: i16) -> i16 {
    if forced != 0 {
        return forced;
    }
    if t.talk_state != 0 {
        return 4;
    }
    if dist < actor.world_pos.distance(t.target) {
        t.timer = 0;
        t.idle_step = 0;
        return 1;
    }
    let var = yaw_to(actor.world_pos, t.target);
    let abs_var = ((var as f32 - actor.shape_rot.y as f32) as i32 as i16 as i32).abs() as i16;
    if yaw_range >= abs_var {
        t.timer = 0;
        t.idle_step = 0;
        return 2;
    }
    // DECR(unk_04).
    if t.timer != 0 {
        t.timer -= 1;
        if t.timer != 0 {
            return t.mode;
        }
    }
    match t.idle_step {
        0 | 2 => {
            t.timer = play.rand.s16_offset(30, 30);
            t.idle_step += 1;
            1
        }
        1 => {
            t.timer = play.rand.s16_offset(10, 10);
            t.idle_step += 1;
            3
        }
        _ => 4,
    }
}

/// `func_80034A14`: the head and torso tracking for preset `preset`, with `forced` as the mode
/// if non-zero.
pub fn func_80034a14(play: &mut PlayState, actor: &mut Actor, t: &mut NpcTrack, preset: usize, forced: i16) {
    let (mut sp38, dist, yaw_range) = D_80116130[preset];
    t.mode = func_80034810(play, actor, t, dist, yaw_range, forced);
    // The C's switch falls through: 1 clears the head's limits, 1 and 3 the torso's, 1, 2 and 3
    // the body turn.
    if t.mode == 1 {
        sp38.unk_00 = 0;
        sp38.unk_04 = 0;
        sp38.unk_02 = 0;
    }
    if t.mode == 1 || t.mode == 3 {
        sp38.unk_06 = 0;
        sp38.unk_0A = 0;
        sp38.unk_08 = 0;
    }
    if matches!(t.mode, 1..=3) {
        sp38.unk_0C = 0;
    }
    func_800344bc(actor, t, sp38.unk_00, sp38.unk_04, sp38.unk_02, sp38.unk_06, sp38.unk_0A, sp38.unk_08, sp38.unk_0C);
}

/// `func_800347E8`: preset `preset`'s yaw range (`D_80116130[preset].unk_14`).
pub fn func_800347e8(preset: usize) -> i16 {
    D_80116130[preset].2
}

/// `func_80034DD4`: an NPC fading in within `dist` of Link and out beyond it (targetable only
/// while near: `ACTOR_FLAG_0`); returns the new alpha. (In a cutscene the distance would be a
/// quarter of the camera's: no cutscenes.)
pub fn func_80034dd4(actor: &mut Actor, player_pos: Vec3, alpha: i16, dist: f32) -> i16 {
    use crate::actor::ACTOR_FLAG_0;
    let mut alpha = alpha;
    let var = actor.world_pos.distance(player_pos);
    if dist < var {
        actor.flags &= !ACTOR_FLAG_0;
        eng_math::smooth_step_to_s(&mut alpha, 0, 6, 0x14, 1);
    } else {
        actor.flags |= ACTOR_FLAG_0;
        eng_math::smooth_step_to_s(&mut alpha, 0xFF, 6, 0x14, 1);
    }
    alpha
}

/// `func_80034F54`: the idle sway angles of `n` limbs from `gameplayFrames`.
pub fn func_80034f54(play: &PlayState, a: &mut [i16], b: &mut [i16], n: usize) {
    let frames = play.gameplay_frames;
    for i in 0..n {
        a[i] = ((0x814 + 50 * i as u32).wrapping_mul(frames)) as i16;
        b[i] = ((0x940 + 50 * i as u32).wrapping_mul(frames)) as i16;
    }
}
