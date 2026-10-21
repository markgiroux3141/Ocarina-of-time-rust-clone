//! GAME-10 milestone 4 against the C: the Like Like (`z_en_rr.c`). Master Quest's Jabu-Jabu's
//! Belly places one in room 14 at (1246, -1293, -2470) (`ootx scene-info --scene bdan`), where
//! `ENTR_JABU_JABU_1` (spawn 1) brings Link, so `object_rr` is loaded.
//!
//! Child Link enters on a `fairy-magic` save (the Kokiri Sword, the Deku Shield, the Kokiri
//! Tunic). Each test spawns a fresh Like Like beside the placed one and runs its update alone
//! (`Actor_UpdateAll`'s call), Link's distance and yaw set as `Actor_UpdateAll` would and its
//! colliders' flags set as the collision check would, so no fight is scripted and nothing waits on
//! `Rand`. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::{Mat4, Vec3};
use oot_actors::PlayExt;
use oot_actors::en_item00::{EnItem00, ITEM00_SHIELD_DEKU, ITEM00_TUNIC_ZORA};
use oot_actors::en_rr::*;
use oot_actors::player::STATE2_7;
use oot_game::actor::*;
use oot_game::actor_ctx::{ActorHandle, ActorImpl, PLAYER_STATE1_28, PLAYER_STATE1_29};
use oot_game::audio::sfx::NA_SE_EN_GOMA_JR_FREEZE;
use oot_game::collision_check::*;
use oot_game::effect::{EFFECT_SS_DEAD_DB, EFFECT_SS_EN_FIRE};
use oot_game::env::clock_time;
use oot_game::item::{EQUIP_TYPE_SHIELD, EQUIP_TYPE_TUNIC, EQUIP_VALUE_SHIELD_DEKU, EQUIP_VALUE_SHIELD_HYLIAN, EQUIP_VALUE_SHIELD_MIRROR, EQUIP_VALUE_TUNIC_KOKIRI, owned_equip_flag};
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::shadow::ShadowDraw;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Room 14's Like Like (layer 0).
const ROOM_14: Vec3 = Vec3::new(1246.0, -1293.0, -2470.0);
/// The Zora Tunic's equip value (`EQUIP_VALUE_TUNIC_ZORA`, `save.h`: 3).
const EQUIP_VALUE_TUNIC_ZORA: u16 = 3;

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in Jabu-Jabu's room 14 (`fairy-magic`, the sound log on), his own again; and a
/// fresh Like Like 200 east of the placed one.
fn belly(a: &Arc<GameAssets>) -> (PlayState, ActorHandle) {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_1").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert_eq!(w.room_ctx.cur.num, 14);
    for _ in 0..200 {
        if w.player().state1 & (PLAYER_STATE1_28 | PLAYER_STATE1_29) == 0 {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(w.player().state1 & (PLAYER_STATE1_28 | PLAYER_STATE1_29), 0);
    let placed: Vec<ActorHandle> = rrs(&w);
    assert_eq!(placed.len(), 1);
    assert_eq!(rr(&w, placed[0]).actor.home_pos, ROOM_14);
    let h = w.actor_spawn(ACTOR_EN_RR, ROOM_14 + Vec3::new(200.0, 0.0, 0.0), [0, 0, 0], -1).expect("a Like Like");
    (w, h)
}

fn rrs(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnRr>(h).is_some_and(|o| !o.actor.killed)).collect()
}

fn rr(w: &PlayState, h: ActorHandle) -> &EnRr {
    w.actors.downcast::<EnRr>(h).expect("En_Rr")
}

fn rr_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnRr {
    w.actors.downcast_mut::<EnRr>(h).expect("En_Rr")
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<EnRr>(h).is_none_or(|o| o.actor.killed)
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// Its draw's changes (`EnRr_Draw`'s `mouthPos`, `effectPos`, the effects), as the drawn actor.
fn draw_update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.draw_update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// `Actor_UpdateAll`'s distances: Link `dist` across at `yaw`.
fn set_link(w: &mut PlayState, h: ActorHandle, dist: f32, yaw: i16) {
    let o = rr_mut(w, h);
    o.actor.xz_dist_to_player = dist;
    o.actor.yaw_towards_player = yaw;
}

/// The times a sound was asked for at `h`.
fn sfx(w: &PlayState, h: ActorHandle, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == oot_game::audio::sfx::SfxPos::Actor(h)).count()
}

/// The live soft sprites of a type.
fn effects(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.ty == ty && e.life >= 0).count()
}

/// Link wearing `shield` and `tunic` (owned), as `Player_SetEquipmentData` leaves him.
fn wear(w: &mut PlayState, shield: u16, tunic: u16) {
    let s = &mut w.save;
    s.inventory_change_equipment(EQUIP_TYPE_SHIELD, shield);
    s.inventory_change_equipment(EQUIP_TYPE_TUNIC, tunic);
    if shield != 0 {
        s.inventory.equipment |= owned_equip_flag(EQUIP_TYPE_SHIELD, shield - 1);
    }
    s.inventory.equipment |= owned_equip_flag(EQUIP_TYPE_TUNIC, tunic - 1);
    let (data, save) = (w.data.clone(), w.save.clone());
    w.player_mut().set_equipment_data(&data, &save);
}

/// Link caught: its body's cylinder touching him (`OC1_HIT`) on a frame it can catch; the mouth
/// at `mouth`.
fn catch(w: &mut PlayState, h: ActorHandle, mouth: Vec3) {
    {
        let o = rr_mut(w, h);
        o.collider1.base.oc_flags1 |= OC1_HIT;
        o.oc_timer = 0;
        o.mouth_pos = mouth;
    }
    update(w, h);
    assert_eq!(rr(w, h).action, Action::GrabPlayer);
}

#[test]
fn it_initialises_as_the_c_does() {
    let Some(a) = assets() else { return };
    let (w, h) = belly(&a);
    let o = rr(&w, h);
    // sInitChain: NAVI_ENEMY_LIKE_LIKE, ATTENTION_RANGE_2, lockOnArrowOffset 30; health 4, mass
    // immovable, scale 0.014 by 0.013, gravity -0.4, approaching; no shadow.
    assert_eq!((o.actor.navi_enemy_id, o.actor.target_mode, o.actor.target_arrow_offset), (NAVI_ENEMY_LIKE_LIKE, 2, 30.0));
    assert_eq!((o.actor.col_chk_info.health, o.actor.col_chk_info.mass), (4, MASS_IMMOVABLE));
    assert_eq!((o.actor.scale, o.actor.gravity), (Vec3::new(0.014, 0.013, 0.014), -0.4));
    assert_eq!((o.action, o.reach_state, o.is_dead, o.has_player), (Action::Approach, REACH_NONE, false, false));
    assert_eq!(o.actor.shape_shadow_draw, ShadowDraw::None);
    let want = ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | (1 << 10);
    assert_eq!(o.actor.flags & want, want);
    // The cylinders: the body 30 by 55; the mouth 20 by 20, 10 down, hard.
    assert_eq!((o.collider1.dim.radius, o.collider1.dim.height, o.collider1.dim.y_shift), (30, 55, 0));
    assert_eq!((o.collider2.dim.radius, o.collider2.dim.height, o.collider2.dim.y_shift), (20, 20, -10));
    assert_ne!(o.collider2.base.ac_flags & AC_HARD, 0);
    // EnRr_InitBodySegments: every ring at full size, the pulse (0.15) and the wobble (2048) to
    // come, at phase 0.
    for s in &o.body_segs {
        assert_eq!((s.scale, s.scale_target, s.scale_mod, s.height), (Vec3::ONE, Vec3::ONE, Vec3::ZERO, 0.0));
    }
    assert_eq!((o.pulse_size, o.pulse_size_target, o.wobble_size, o.wobble_size_target), (0.0, 0.15, 0.0, 2048.0));
    assert_eq!((o.seg_phase_vel_target, o.seg_wobble_x_target, o.seg_wobble_z_target), (2500.0, 3.0, 1.0));
}

#[test]
fn it_creeps_at_link_then_reaches_for_him_in_stages() {
    let Some(a) = assets() else { return };
    let (mut w, h) = belly(&a);
    // Link 300 across: off at 2 (slowed 0.1 the same frame), NA_SE_EN_LIKE_WALK; no reach.
    set_link(&mut w, h, 300.0, 0);
    update(&mut w, h);
    assert_eq!((rr(&w, h).action, rr(&w, h).actor.speed_xz), (Action::Approach, 1.9));
    assert_eq!(sfx(&w, h, NA_SE_EN_LIKE_WALK), 1);
    // Within 160 with its count out: EnRr_SetupReach (20 frames), NA_SE_EN_LIKE_UNARI.
    set_link(&mut w, h, 150.0, 0);
    update(&mut w, h);
    let o = rr(&w, h);
    assert_eq!((o.action, o.reach_state, o.action_timer), (Action::Reach, REACH_EXTEND, 20));
    assert_eq!(sfx(&w, h, NA_SE_EN_LIKE_UNARI), 1);
    let heights: Vec<f32> = o.body_segs.iter().map(|s| s.height_target).collect();
    assert_eq!(heights, [0.0, 500.0, 750.0, 1000.0, 1000.0]);
    // EnRr_Reach: 20 frames up, then the mouth opens (1.5, 5 frames), gapes (2000, 2 frames),
    // closes (0.8, 20 frames) and it settles (EnRr_SetupNeutral: 60 frames to the next reach).
    let mut stages = vec![];
    for k in 1..=60 {
        let before = rr(&w, h).reach_state;
        update(&mut w, h);
        let o = rr(&w, h);
        if o.reach_state != before || o.action != Action::Reach {
            stages.push((k, o.reach_state));
        }
        if o.action != Action::Reach {
            break;
        }
        if o.reach_state == REACH_STOP {
            // Up by the stop: each ring at its height, narrowed to 0.8.
            for (s, want) in o.body_segs.iter().zip([0.0, 500.0, 750.0, 1000.0, 1000.0]) {
                assert_eq!((s.height, s.scale.x, s.scale.z), (want, 0.8, 0.8));
            }
        }
    }
    assert_eq!(stages, [(20, REACH_STOP), (21, REACH_OPEN), (26, REACH_GAPE), (28, REACH_CLOSE), (48, REACH_NONE)]);
    let o = rr(&w, h);
    assert_eq!((o.action, o.action_timer), (Action::Approach, 60));
    assert_eq!(o.body_segs[RR_MOUTH].height_target, 0.0);
}

#[test]
fn touching_link_it_swallows_him_toward_its_mouth() {
    let Some(a) = assets() else { return };
    let (mut w, h) = belly(&a);
    let start = w.player().actor.world_pos;
    let mouth = start + Vec3::new(100.0, 50.0, -10.0);
    // A touch while it's flashing, or within ocTimer, catches nothing.
    {
        let o = rr_mut(&mut w, h);
        o.collider1.base.oc_flags1 |= OC1_HIT;
        o.oc_timer = 5;
    }
    update(&mut w, h);
    assert_eq!(rr(&w, h).action, Action::Approach);
    // EnRr_CollisionCheck: play->grabPlayer, Link's parent the Like Like; EnRr_SetupGrabPlayer:
    // 100 frames, lock-on off, no catch for 8, NA_SE_EN_LIKE_DRINK; EnRr_GrabPlayer the same
    // frame: Link 30 a frame toward the mouth on each axis, the swallow 5 down.
    catch(&mut w, h, mouth);
    let o = rr(&w, h);
    assert_eq!((o.grab_timer, o.oc_timer, o.has_player, o.swallow_offset), (100, 8, true, -5.0));
    assert_eq!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!((o.seg_phase_vel_target, o.wobble_size_target, o.pulse_size_target), (5000.0, 512.0, 0.15));
    assert_eq!(sfx(&w, h, NA_SE_EN_LIKE_DRINK), 1);
    let p = w.player();
    assert_ne!(p.state2 & STATE2_7, 0);
    assert_eq!(p.actor.parent, Some(h));
    assert_eq!(p.actor.world_pos, start + Vec3::new(30.0, 30.0, -10.0));
    // The next frame: x 30 more, y to the mouth less the swallow so far (5: 15 left), z there;
    // the swallow 10 down.
    update(&mut w, h);
    assert_eq!(rr(&w, h).grab_timer, 99);
    assert_eq!(w.player().actor.world_pos, Vec3::new(start.x + 60.0, mouth.y - 5.0, mouth.z));
    assert_eq!(rr(&w, h).swallow_offset, -10.0);
    // Swallowed to -55 (5 a frame, there by the 11th frame), Link pulled down after it (each
    // frame to the swallow as the frame before left it).
    for _ in 0..12 {
        update(&mut w, h);
    }
    assert_eq!(rr(&w, h).swallow_offset, -55.0);
    assert_eq!(w.player().actor.world_pos, Vec3::new(mouth.x, mouth.y - 55.0, mouth.z));
    // NA_SE_EN_LIKE_EAT every 8th frame of its count.
    assert!(sfx(&w, h, NA_SE_EN_LIKE_EAT) >= 1);
}

#[test]
fn spat_out_link_loses_his_deku_shield_but_not_the_kokiri_tunic() {
    let Some(a) = assets() else { return };
    let (mut w, h) = belly(&a);
    wear(&mut w, EQUIP_VALUE_SHIELD_DEKU, EQUIP_VALUE_TUNIC_KOKIRI);
    assert_eq!(w.player().current_shield, 1);
    let mouth = w.player().actor.world_pos;
    catch(&mut w, h, mouth);
    // The count out: EnRr_SetupReleasePlayer.
    rr_mut(&mut w, h).grab_timer = 1;
    rr_mut(&mut w, h).actor.shape_rot.y = 0x1234;
    update(&mut w, h);
    let o = rr(&w, h);
    // The Deku Shield eaten (Inventory_DeleteEquipment: worn and owned no more,
    // Player_SetEquipmentData, the pause cursor on its left arrow); the Kokiri Tunic kept. It
    // retreats; it says so (0x305F).
    assert_eq!((o.eaten_shield, o.eaten_tunic, o.retreat, o.has_player), (1, 0, true, false));
    assert_eq!(w.save.cur_equip_value(EQUIP_TYPE_SHIELD), 0);
    assert!(!w.save.check_owned_equip(EQUIP_TYPE_SHIELD, EQUIP_VALUE_SHIELD_DEKU - 1));
    assert_eq!(w.save.cur_equip_value(EQUIP_TYPE_TUNIC), EQUIP_VALUE_TUNIC_KOKIRI);
    assert_eq!(w.player().current_shield, 0);
    assert_eq!(w.pause_ctx.cursor_special_pos, oot_game::kaleido::PAUSE_CURSOR_PAGE_LEFT);
    assert_eq!(w.msg_ctx.text_id, 0x305F);
    // Lock-on on, 110 frames before another catch; not flashing: approaching,
    // NA_SE_EN_LIKE_THROW.
    assert_eq!((o.action, o.oc_timer), (Action::Approach, 110));
    assert_ne!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(sfx(&w, h, NA_SE_EN_LIKE_THROW), 1);
    // Link thrown out: parent cleared; Actor_SetPlayerKnockbackLarge(4, its facing, 12, 8).
    let p = w.player();
    assert_eq!(p.actor.parent, None);
    assert_eq!((p.knockback_type, p.knockback_speed, p.knockback_rot, p.knockback_y_velocity, p.knockback_damage), (2, 4.0, 0x1234, 12.0, 8));
    // Having eaten, its next settle is a retreat (EnRr_SetupNeutral): 100 frames away from Link.
    rr_mut(&mut w, h).action = Action::Reach;
    rr_mut(&mut w, h).reach_state = REACH_CLOSE;
    rr_mut(&mut w, h).action_timer = 1;
    update(&mut w, h);
    assert_eq!((rr(&w, h).action, rr(&w, h).action_timer), (Action::Retreat, 100));
}

#[test]
fn it_eats_a_zora_tunic_and_a_hylian_shield_but_never_the_mirror_shield() {
    let Some(a) = assets() else { return };
    let (mut w, h) = belly(&a);
    // The Hylian Shield and the Zora Tunic: both eaten (the Kokiri Tunic put on), 0x3061.
    wear(&mut w, EQUIP_VALUE_SHIELD_HYLIAN, EQUIP_VALUE_TUNIC_ZORA);
    let mouth = w.player().actor.world_pos;
    catch(&mut w, h, mouth);
    // Link struggled free (Player_Action_8084F308 clears PLAYER_STATE2_7): let go.
    w.player_mut().state2 &= !STATE2_7;
    update(&mut w, h);
    let o = rr(&w, h);
    assert_eq!((o.eaten_shield, o.eaten_tunic, o.action), (2, 3, Action::Approach));
    assert_eq!(w.save.cur_equip_value(EQUIP_TYPE_SHIELD), 0);
    assert_eq!(w.save.cur_equip_value(EQUIP_TYPE_TUNIC), EQUIP_VALUE_TUNIC_KOKIRI);
    assert!(!w.save.check_owned_equip(EQUIP_TYPE_TUNIC, EQUIP_VALUE_TUNIC_ZORA - 1));
    // Player_SetEquipmentData: currentTunic the Kokiri's (0).
    assert_eq!(w.player().current_tunic, 0);
    assert_eq!(w.msg_ctx.text_id, 0x3061);
    // EnRr_GetMessage: only these four count.
    assert_eq!(EnRr::get_message(1, 0), RR_MESSAGE_SHIELD);
    assert_eq!(EnRr::get_message(0, 2), RR_MESSAGE_TUNIC);
    assert_eq!(EnRr::get_message(3, 1), 0);
    // The Mirror Shield isn't eaten: nothing taken, no retreat.
    let (mut w, h) = belly(&a);
    wear(&mut w, EQUIP_VALUE_SHIELD_MIRROR, EQUIP_VALUE_TUNIC_KOKIRI);
    let mouth = w.player().actor.world_pos;
    catch(&mut w, h, mouth);
    w.player_mut().state2 &= !STATE2_7;
    update(&mut w, h);
    let o = rr(&w, h);
    assert_eq!((o.eaten_shield, o.eaten_tunic, o.retreat), (0, 0, false));
    assert_eq!(w.save.cur_equip_value(EQUIP_TYPE_SHIELD), EQUIP_VALUE_SHIELD_MIRROR);
}

#[test]
fn a_hit_while_it_holds_link_lets_him_go_and_hurts_it() {
    let Some(a) = assets() else { return };
    let (mut w, h) = belly(&a);
    let mouth = w.player().actor.world_pos;
    catch(&mut w, h, mouth);
    // The Kokiri Sword (2 with its jump: RR_DMG_REACT_NORMAL): health 4 - 2, 40 frames red and
    // invincible, Link let go; still flashing: EnRr_SetupDamage (NA_SE_EN_LIKE_DAMAGE), its rings
    // swaying (colorFilterTimer 40 & 8: 5000).
    {
        let o = rr_mut(&mut w, h);
        o.collider1.base.ac_flags |= AC_HIT;
        o.actor.col_chk_info.damage = 2;
        o.actor.col_chk_info.damage_reaction = RR_DMG_REACT_NORMAL;
    }
    update(&mut w, h);
    let o = rr(&w, h);
    assert_eq!((o.action, o.actor.col_chk_info.health, o.invincibility_timer, o.has_player), (Action::Damage, 2, 40, false));
    assert_eq!(o.actor.color_filter_params & (COLORFILTER_COLORFLAG_RED | COLORFILTER_BUFFLAG_XLU), COLORFILTER_COLORFLAG_RED | COLORFILTER_BUFFLAG_XLU);
    assert_eq!(o.actor.color_filter_timer, 40);
    assert_eq!(o.body_segs[1].rot_target.z, 5000.0);
    assert_eq!(sfx(&w, h, NA_SE_EN_LIKE_DAMAGE), 1);
    assert_eq!(w.player().actor.parent, None);
    assert_eq!(w.player().knockback_type, 2);
    // The colour worn off: approaching again.
    rr_mut(&mut w, h).actor.color_filter_timer = 0;
    update(&mut w, h);
    assert_eq!(rr(&w, h).action, Action::Approach);
}

#[test]
fn its_mouth_clanks_the_boomerang_stuns_and_fire_burns() {
    let Some(a) = assets() else { return };
    let (mut w, h) = belly(&a);
    // The mouth (AC_HARD): sparks and NA_SE_IT_SHIELD_REFLECT_SW, nothing else.
    {
        let o = rr_mut(&mut w, h);
        o.collider2.base.ac_flags |= AC_HIT;
        o.actor.col_chk_info.damage = 2;
        o.actor.col_chk_info.damage_reaction = RR_DMG_REACT_NORMAL;
    }
    let reflects = w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == oot_game::audio::sfx::NA_SE_IT_SHIELD_REFLECT_SW).count();
    update(&mut w, h);
    assert_eq!((rr(&w, h).action, rr(&w, h).actor.col_chk_info.health), (Action::Approach, 4));
    assert_eq!(w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == oot_game::audio::sfx::NA_SE_IT_SHIELD_REFLECT_SW).count(), reflects + 1);
    // The boomerang (RR_DMG_REACT_STUN, no damage): blue 80, NA_SE_EN_GOMA_JR_FREEZE, stunned (the
    // scroll stopped) until the colour wears off.
    {
        let o = rr_mut(&mut w, h);
        o.collider1.base.ac_flags |= AC_HIT;
        o.actor.col_chk_info.damage = 0;
        o.actor.col_chk_info.damage_reaction = RR_DMG_REACT_STUN;
    }
    update(&mut w, h);
    let o = rr(&w, h);
    assert_eq!((o.action, o.stop_scroll, o.actor.color_filter_timer), (Action::Stunned, true, 80));
    assert_eq!(o.actor.color_filter_params & (COLORFILTER_COLORFLAG_RED | COLORFILTER_COLORFLAG_GRAY), COLORFILTER_COLORFLAG_BLUE);
    assert_eq!(sfx(&w, h, NA_SE_EN_GOMA_JR_FREEZE), 1);
    let scroll = o.scroll_timer;
    update(&mut w, h);
    assert_eq!(rr(&w, h).scroll_timer, scroll);
    rr_mut(&mut w, h).actor.color_filter_timer = 0;
    update(&mut w, h);
    assert_eq!((rr(&w, h).action, rr(&w, h).stop_scroll), (Action::Approach, false));
    // A fire arrow (4, RR_DMG_REACT_FIRE): red 80, 20 frames of flames (the draw: one every other
    // frame, holding the colour on), stunned.
    {
        let o = rr_mut(&mut w, h);
        o.collider1.base.ac_flags |= AC_HIT;
        o.actor.col_chk_info.damage = 4;
        o.actor.col_chk_info.damage_reaction = RR_DMG_REACT_FIRE;
    }
    update(&mut w, h);
    let o = rr(&w, h);
    assert_eq!((o.action, o.effect_timer, o.actor.col_chk_info.health, o.actor.color_filter_timer), (Action::Stunned, 20, 0, 80));
    let flames = effects(&w, EFFECT_SS_EN_FIRE);
    // effectTimer 20: the draw's 19 is odd, no flame; 18 a flame; the colour held.
    draw_update(&mut w, h);
    assert_eq!(effects(&w, EFFECT_SS_EN_FIRE), flames);
    assert_eq!(rr(&w, h).actor.color_filter_timer, 81);
    rr_mut(&mut w, h).effect_timer = 19;
    draw_update(&mut w, h);
    assert_eq!(effects(&w, EFFECT_SS_EN_FIRE), flames + 1);
    // Burnt to no health: dying once the colour wears off.
    rr_mut(&mut w, h).actor.color_filter_timer = 0;
    update(&mut w, h);
    assert_eq!((rr(&w, h).action, rr(&w, h).is_dead), (Action::Death, true));
}

#[test]
fn dead_it_drops_what_it_ate_and_its_own() {
    let Some(a) = assets() else { return };
    let (mut w, h) = belly(&a);
    // It ate a Deku Shield and a Zora Tunic; a light arrow (15, RR_DMG_REACT_LIGHT_ARROW) kills it:
    // dropType 4 (a purple rupee); EnRr_SetupDeath: NA_SE_EN_LIKE_DEAD, lock-on off, frames from 0.
    {
        let o = rr_mut(&mut w, h);
        o.eaten_shield = 1;
        o.eaten_tunic = 3;
        o.collider1.base.ac_flags |= AC_HIT;
        o.actor.col_chk_info.damage = 15;
        o.actor.col_chk_info.damage_reaction = RR_DMG_REACT_LIGHT_ARROW;
    }
    update(&mut w, h);
    let o = rr(&w, h);
    assert_eq!((o.action, o.is_dead, o.drop_type, o.frame_count), (Action::Death, true, RR_DROP_RUPEE_PURPLE, 0));
    assert_eq!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(sfx(&w, h, NA_SE_EN_LIKE_DEAD), 1);
    // EnRr_Death: 40 frames of the rings sinking and swelling (the base 16 × frame × 0.003 + 1),
    // then shrinking; a puff of smoke at 88; at 95 the drops, gone.
    let smoke = effects(&w, EFFECT_SS_DEAD_DB);
    let items_before: Vec<ActorHandle> = w.actors.all();
    for k in 1..=94i16 {
        update(&mut w, h);
        let o = rr(&w, h);
        assert_eq!(o.frame_count, k);
        if k < 40 {
            assert_eq!(o.body_segs[0].scale_target.x, (16.0 * k as f32 * 0.003) + 1.0, "frame {k}");
        }
        if k == 88 {
            assert_eq!(effects(&w, EFFECT_SS_DEAD_DB), smoke + 1);
        }
    }
    assert!(!gone(&w, h));
    assert!(rr(&w, h).actor.scale.x < 0.014);
    update(&mut w, h);
    assert!(gone(&w, h));
    // Item_DropCollectible: the Deku Shield, the Zora Tunic (killed at once: no object_gi_clothes
    // in the room), the purple rupee (ITEM00_RUPEE_PURPLE 0x14).
    let new: Vec<i16> = w.actors.all().into_iter().filter(|h| !items_before.contains(h)).filter_map(|h| w.actors.downcast::<EnItem00>(h)).map(|e| e.actor.params & 0xFF).collect();
    assert!(new.contains(&ITEM00_SHIELD_DEKU), "{new:x?}");
    assert!(new.contains(&ITEM00_TUNIC_ZORA), "{new:x?}");
    assert!(new.contains(&0x14), "{new:x?}");
}

#[test]
fn its_draw_stacks_the_rings_and_puts_the_mouth_on_top() {
    let Some(a) = assets() else { return };
    let (mut w, h) = belly(&a);
    // At rest (heights 0, no turns, full size), facing 0: each ring 1000 above the last in model
    // units, × 0.013: the mouth 52 up.
    draw_update(&mut w, h);
    let o = rr(&w, h);
    let p = o.actor.world_pos;
    for i in 1..5 {
        assert!(o.effect_pos[i].distance(p + Vec3::new(0.0, 13.0 * i as f32, 0.0)) < 1e-3, "{i}: {:?}", o.effect_pos[i]);
    }
    assert_eq!(o.effect_pos[0], p);
    assert_eq!(o.mouth_pos, o.effect_pos[4]);
    // Its next update puts the mouth's cylinder there.
    update(&mut w, h);
    let m = rr(&w, h).mouth_pos;
    assert_eq!(rr(&w, h).collider2.dim.pos, [m.x as i32 as i16, m.y as i32 as i16, m.z as i32 as i16]);
    // The draw: one XLU command of the bake, the base under the transform, four rings.
    let o = rr(&w, h);
    let mut out = DrawOut::default();
    o.draw(&o.render_state(), &w, &ViewInfo::new(Vec3::ZERO, Mat4::IDENTITY), &mut out);
    assert!(out.opa.is_empty());
    assert_eq!(out.xlu.len(), 1);
    assert_eq!(out.xlu[0].mesh, eng_gfx::MeshKey::named(oot_game::pack::keys::bake(BAKE)));
    assert_eq!(out.xlu[0].bones.len(), 4);
    // The bake: the base's vertices on no bone (the transform), the rings' on bones 0 to 3, every
    // matrix load resolved.
    let dl = a.pack.bake(BAKE).expect("the bake is in the pack");
    let bones: std::collections::BTreeSet<u16> = dl.batches.iter().flat_map(|b| b.vertices.iter().map(|v| v.bone)).collect();
    assert_eq!(bones.into_iter().collect::<Vec<_>>(), [0, 1, 2, 3, eng_gfx::NO_BONE]);
    assert!(dl.stats.unresolved_addresses.is_empty(), "{:?}", dl.stats.unresolved_addresses);
}
