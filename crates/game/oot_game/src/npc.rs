//! The helpers `z_actor.c` gives NPCs: offering to talk (`Actor_OfferTalkExchange` and its short forms),
//! answering a talk request (`Actor_TalkOfferAccepted`), the per-frame talk handler
//! (`Npc_UpdateTalking`), the head and torso turning towards a point (`Npc_TrackPoint`,
//! `Npc_TrackPointWithLimits`, `Npc_UpdateAutoTurn` with the `sNpcTrackingPresets` presets), and the idle limb sway
//! (`Actor_UpdateFidgetTables`), and the fade by distance (`Actor_UpdateAlphaByDistance`).
//!
//! Player takes an offer on A (`Player_ActionHandler_Talk`, in `oot_actors::player`): the actor gets
//! `ACTOR_FLAG_TALK` and the message box opens with its `textId` (`crate::message`).

use glam::Vec3;

use crate::actor::{ACTOR_FLAG_TALK, Actor};
use crate::play::PlayState;
use crate::target::{pitch_to, yaw_to};

/// `EXCH_ITEM_NONE`.
pub const EXCH_ITEM_NONE: u8 = 0;

/// `Actor_TalkOfferAccepted`: did Player accept this actor's talk offer (`ACTOR_FLAG_TALK`)?
pub fn process_talk_request(actor: &mut Actor) -> bool {
    if actor.flags & ACTOR_FLAG_TALK != 0 {
        actor.flags &= !ACTOR_FLAG_TALK;
        return true;
    }
    false
}

/// `Actor_OfferTalkExchange`: the updating actor (`play.cur_actor`) offers to talk if it's the nearest
/// offer within `xz_range` and `y_range` this frame (or Player targets it), recording itself in
/// Player's `targetActor`.
pub fn offer_talk_range(play: &mut PlayState, actor: &Actor, xz_range: f32, y_range: f32, exchange_item: u8) -> bool {
    let (Some(ph), Some(me)) = (play.player, play.cur_actor) else { return false };
    let Some(p) = play.actors.get_mut(ph) else { return false };
    let player_flag8 = p.base().flags & ACTOR_FLAG_TALK != 0;
    // Player_InCsMode: never (no cutscenes).
    let Some(pi) = p.as_player_mut() else { return false };
    let (_, target_dist) = pi.talk_target();
    if player_flag8 || (!actor.is_targeted && (y_range < actor.y_dist_to_player.abs() || target_dist < actor.xz_dist_to_player || xz_range < actor.xz_dist_to_player)) {
        return false;
    }
    pi.set_talk_target(me, actor.xz_dist_to_player, exchange_item);
    true
}

/// `Actor_OfferTalkExchangeEquiCylinder`.
pub fn offer_talk_exchange(play: &mut PlayState, actor: &Actor, range: f32, exchange_item: u8) -> bool {
    offer_talk_range(play, actor, range, range, exchange_item)
}

/// `Actor_OfferTalk`.
pub fn offer_talk(play: &mut PlayState, actor: &Actor, range: f32) -> bool {
    offer_talk_exchange(play, actor, range, EXCH_ITEM_NONE)
}

/// `Actor_OfferTalkNearColChkInfoCylinder`: within 50 plus the actor's `cylRadius`.
pub fn offer_talk_default(play: &mut PlayState, actor: &Actor) -> bool {
    let r = 50.0 + actor.col_chk_info.cyl_radius as f32;
    offer_talk(play, actor, r)
}

/// `Actor_SetTextWithPrefix`: `base_text_id` with the scene's message prefix (0x1000 for the
/// forest's scenes, the Deku Tree and its boss room among them; 0x2000 to 0x7000 for the
/// field, the mountain, Zora's, Kakariko, the desert and the market; else none).
pub fn actor_set_text_with_prefix(scene_id: u16, actor: &mut Actor, base_text_id: i16) {
    // The SCENE_* ids (`tables/scene_table.h`), in the C's cases' order; 112 is a number in the C.
    let prefix: i16 = match scene_id {
        // SCENE_DEKU_TREE, _DEKU_TREE_BOSS, _FOREST_TEMPLE_BOSS, _KNOW_IT_ALL_BROS_HOUSE,
        // _TWINS_HOUSE, _MIDOS_HOUSE, _SARIAS_HOUSE, _KOKIRI_SHOP, _LINKS_HOUSE, _KOKIRI_FOREST,
        // _SACRED_FOREST_MEADOW, _LOST_WOODS, 112.
        0x00 | 0x11 | 0x14 | 0x26 | 0x27 | 0x28 | 0x29 | 0x2D | 0x34 | 0x55 | 0x56 | 0x5B | 112 => 0x1000,
        // SCENE_STABLE, _HYRULE_FIELD, _LON_LON_RANCH.
        0x36 | 0x51 | 0x63 => 0x2000,
        // SCENE_FIRE_TEMPLE, _DODONGOS_CAVERN_BOSS, _FIRE_TEMPLE_BOSS, _DEATH_MOUNTAIN_TRAIL,
        // _DEATH_MOUNTAIN_CRATER, _GORON_CITY.
        0x04 | 0x12 | 0x15 | 0x60 | 0x61 | 0x62 => 0x3000,
        // SCENE_JABU_JABU, _JABU_JABU_BOSS, _ZORAS_RIVER, _ZORAS_DOMAIN, _ZORAS_FOUNTAIN.
        0x02 | 0x13 | 0x54 | 0x58 | 0x59 => 0x4000,
        // SCENE_SHADOW_TEMPLE, _SHADOW_TEMPLE_BOSS, _KAKARIKO_CENTER_GUEST_HOUSE,
        // _BACK_ALLEY_HOUSE, _DOG_LADY_HOUSE, _GRAVEKEEPERS_HUT, _REDEAD_GRAVE,
        // _WINDMILL_AND_DAMPES_GRAVE, _KAKARIKO_VILLAGE, _GRAVEYARD.
        0x07 | 0x18 | 0x2A | 0x2B | 0x35 | 0x3A | 0x3F | 0x48 | 0x52 | 0x53 => 0x5000,
        // SCENE_SPIRIT_TEMPLE, _SPIRIT_TEMPLE_BOSS, _IMPAS_HOUSE, _CARPENTERS_TENT, _LAKE_HYLIA,
        // _GERUDO_VALLEY, _DESERT_COLOSSUS.
        0x06 | 0x17 | 0x37 | 0x39 | 0x57 | 0x5A | 0x5C => 0x6000,
        // SCENE_MARKET_ENTRANCE_DAY, _BACK_ALLEY_DAY, _BACK_ALLEY_NIGHT, _MARKET_DAY,
        // _MARKET_NIGHT, _MARKET_RUINS, _HYRULE_CASTLE.
        0x1B | 0x1E | 0x1F | 0x20 | 0x21 | 0x22 | 0x5F => 0x7000,
        _ => 0x0000,
    };
    actor.text_id = (prefix | base_text_id) as u16;
}

/// `Actor_TextboxIsClosing`: the message box is on its closing frame (`TEXT_STATE_CLOSING`).
pub fn textbox_is_closing(play: &PlayState) -> bool {
    play.message_state() == crate::message::TEXT_STATE_CLOSING
}

/// `Npc_UpdateTalking`: an NPC's talk handling for one frame. `talk_state` is the NPC's
/// `interactInfo.talkState` (0: not talking). When a request was accepted it becomes 1; while talking,
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

/// `NpcInteractInfo`: an NPC's talk state and head / torso tracking.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NpcTrack {
    /// `talkState`: the talk state (`Npc_UpdateTalking`).
    pub talk_state: i16,
    /// `trackingMode`: this frame's tracking mode (1 none, 2 head, 3 head and torso partly, 4 full).
    pub mode: i16,
    /// `autoTurnTimer`: frames until the idle mode changes; `autoTurnState`: the idle mode's step.
    pub timer: i16,
    pub idle_step: i16,
    /// `headRot`: the head's (x pitch, y yaw, z) turn; `torsoRot`: the torso's.
    pub head: [i16; 3],
    pub torso: [i16; 3],
    /// `yOffset`: the height the tracking looks from, above the actor's position.
    pub eye_height: f32,
    /// `trackPos`: the point to look at (Player, or the camera in a cutscene).
    pub target: Vec3,
}

/// `NpcTrackingRotLimits`: a tracking preset. `Npc_TrackPoint` passes it to `Npc_TrackPointWithLimits` as
/// (`maxHeadYaw` head yaw, `maxHeadPitch` head pitch max, `minHeadPitch` head pitch min, `maxTorsoYaw` torso yaw,
/// `maxTorsoPitch` torso pitch max, `minTorsoPitch` torso pitch min, `rotateYaw` turn the body).
#[derive(Debug, Clone, Copy)]
#[allow(non_snake_case)]
struct Limits {
    max_head_yaw: i16,
    min_head_pitch: i16,
    max_head_pitch: i16,
    max_torso_yaw: i16,
    min_torso_pitch: i16,
    max_torso_pitch: i16,
    rotate_yaw: u8,
}

/// `sNpcTrackingPresets`: the presets (`sub_00`, `autoTurnDistanceRange` the distance, `maxYawForPlayerTracking` the yaw range).
const S_NPC_TRACKING_PRESETS: [(Limits, f32, i16); 13] = {
    const fn l(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u8) -> Limits {
        Limits { max_head_yaw: a as i16, min_head_pitch: b as i16, max_head_pitch: c as i16, max_torso_yaw: d as i16, min_torso_pitch: e as i16, max_torso_pitch: f as i16, rotate_yaw: g }
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

/// `Npc_TrackPointWithLimits`: the head turns towards the target within its limits, the torso takes what
/// the head can't cover, and the whole body turns too if `arg8`.
#[allow(clippy::too_many_arguments)]
fn track_point_with_limits(actor: &mut Actor, t: &mut NpcTrack, arg2: i16, arg3: i16, arg4: i16, arg5: i16, arg6: i16, arg7: i16, arg8: u8) {
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

/// `Npc_UpdateAutoTurn`: which tracking mode this frame. `forced` (`arg4`) wins; talking tracks
/// fully; beyond `dist` nothing; within `yaw_range` of facing the head only; otherwise the
/// idle glances (`Rand_S16Offset` timers).
fn update_auto_turn(play: &mut PlayState, actor: &Actor, t: &mut NpcTrack, dist: f32, yaw_range: i16, forced: i16) -> i16 {
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
    // DECR(maxHeadPitch).
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

/// `Npc_TrackPoint`: the head and torso tracking for preset `preset`, with `forced` as the mode
/// if non-zero.
pub fn track_point(play: &mut PlayState, actor: &mut Actor, t: &mut NpcTrack, preset: usize, forced: i16) {
    let (mut sp38, dist, yaw_range) = S_NPC_TRACKING_PRESETS[preset];
    t.mode = update_auto_turn(play, actor, t, dist, yaw_range, forced);
    // The C's switch falls through: 1 clears the head's limits, 1 and 3 the torso's, 1, 2 and 3
    // the body turn.
    if t.mode == 1 {
        sp38.max_head_yaw = 0;
        sp38.max_head_pitch = 0;
        sp38.min_head_pitch = 0;
    }
    if t.mode == 1 || t.mode == 3 {
        sp38.max_torso_yaw = 0;
        sp38.max_torso_pitch = 0;
        sp38.min_torso_pitch = 0;
    }
    if matches!(t.mode, 1..=3) {
        sp38.rotate_yaw = 0;
    }
    track_point_with_limits(actor, t, sp38.max_head_yaw, sp38.max_head_pitch, sp38.min_head_pitch, sp38.max_torso_yaw, sp38.max_torso_pitch, sp38.min_torso_pitch, sp38.rotate_yaw);
}

/// `Npc_GetTrackingPresetMaxPlayerYaw`: preset `preset`'s yaw range (`sNpcTrackingPresets[preset].maxYawForPlayerTracking`).
pub fn get_tracking_preset_max_player_yaw(preset: usize) -> i16 {
    S_NPC_TRACKING_PRESETS[preset].2
}

/// `Actor_UpdateAlphaByDistance`: an NPC fading in within `dist` of Link and out beyond it (targetable only
/// while near: `ACTOR_FLAG_ATTENTION_ENABLED`); returns the new alpha. (In a cutscene the distance would be a
/// quarter of the camera's: no cutscenes.)
pub fn actor_update_alpha_by_distance(actor: &mut Actor, player_pos: Vec3, alpha: i16, dist: f32) -> i16 {
    use crate::actor::ACTOR_FLAG_ATTENTION_ENABLED;
    let mut alpha = alpha;
    let var = actor.world_pos.distance(player_pos);
    if dist < var {
        actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        eng_math::smooth_step_to_s(&mut alpha, 0, 6, 0x14, 1);
    } else {
        actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        eng_math::smooth_step_to_s(&mut alpha, 0xFF, 6, 0x14, 1);
    }
    alpha
}

/// `Actor_UpdateFidgetTables`: the idle sway angles of `n` limbs from `gameplayFrames`.
pub fn actor_update_fidget_tables(play: &PlayState, a: &mut [i16], b: &mut [i16], n: usize) {
    let frames = play.gameplay_frames;
    for i in 0..n {
        a[i] = ((0x814 + 50 * i as u32).wrapping_mul(frames)) as i16;
        b[i] = ((0x940 + 50 * i as u32).wrapping_mul(frames)) as i16;
    }
}
