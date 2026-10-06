//! The Skulltula (`En_St`, `z_en_st.c`) against the C, in the Deku Tree (MQ): room 5's at
//! (-1347, -806, 1079) facing +x (rot y 16384), 74 above its floor, and room 2's at (-1257, 748,
//! 1249), both params 0 (the normal size; 1 is the big one). Neither is in room 0: each test
//! changes room first (`Room_RequestNewRoom`, a frame, `Room_FinishRoomChange`).
//!
//! Expected values are worked out from the C in the comments. Where a run can't reach a state
//! reliably (which way it faces when a slash lands, a Deku Nut, which Link can't throw yet),
//! the test sets it directly and says so.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_B, PadState};
use eng_math::{cos_s, sin_s, smooth_step_to_f, smooth_step_to_s};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_item00::EnItem00;
use oot_actors::en_st::{self, Action, EnSt};
use oot_game::actor::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::*;
use oot_game::collision_check as cc;
use oot_game::effect::{EFFECT_SS_BLAST, EFFECT_SS_DEAD_DB, EFFECT_SS_DUST};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::sys_matrix::{MtxF, binang_to_rad};

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn tick(w: &mut PlayState, p: PadState, prev: &mut PadState) {
    w.tick_with(scripted_input(*prev, p));
    *prev = p;
}

const HOME5: Vec3 = Vec3::new(-1347.0, -806.0, 1079.0);
const HOME2: Vec3 = Vec3::new(-1257.0, 748.0, 1249.0);

/// Inside the Deku Tree (`deku-tree-inside`: the Kokiri Sword and the Deku Shield), the sound log
/// on, then in `room`: the room's Skulltula. (It spawns with the room and updates in the frame
/// that loads it.)
fn deku_tree_room(a: &Arc<GameAssets>, room: i8) -> (PlayState, ActorHandle) {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 60);
    assert!(w.room_request(room));
    idle(&mut w, 1);
    w.room_change_done();
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnSt>(h).is_some()).expect("the room's En_St");
    (w, h)
}

fn st(w: &PlayState, h: ActorHandle) -> &EnSt {
    w.actors.downcast::<EnSt>(h).expect("En_St")
}

fn st_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnSt {
    w.actors.downcast_mut::<EnSt>(h).expect("En_St")
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

fn sfx_frames(w: &PlayState, id: u16, after: u32) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(f, s, _)| f > after && s == id).map(|&(f, _, _)| f).collect()
}

/// Link `dx` along +x from room 5's Skulltula's home, on the floor there, facing it (-x).
fn place_link(w: &mut PlayState, dx: f32) {
    let (y, _) = w.col.entity_raycast_down(Vec3::new(HOME5.x + dx, HOME5.y, HOME5.z));
    w.place_player(Vec3::new(HOME5.x + dx, y, HOME5.z), -0x4000);
}

/// Room 5's Skulltula brought down by Link 100 in front of it, waiting on the ground.
fn on_the_ground(a: &Arc<GameAssets>) -> (PlayState, ActorHandle) {
    let (mut w, h) = deku_tree_room(a, 5);
    place_link(&mut w, 100.0);
    let mut n = 0;
    while st(&w, h).action != Action::WaitOnGround {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 60, "it never landed: {:?}", st(&w, h).action);
    }
    (w, h)
}

/// The live soft sprites of type `ty`.
fn effects(w: &PlayState, ty: u8) -> Vec<&oot_game::effect::EffectSs> {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == ty).collect()
}

#[test]
fn init_its_colliders_scale_and_ceiling() {
    let Some(a) = assets() else { return };
    let (w, h) = deku_tree_room(&a, 5);
    let s = st(&w, h);
    assert_eq!((s.actor.home_pos, s.actor.params), (HOME5, 0));
    // EnSt_SetColliderScale (params 0: scaleAmount 1): scale 0.04, floorHeightOffset 32, the
    // colliders as initialised.
    assert_eq!((s.actor.scale.x, s.collider_scale, s.floor_height_offset), (0.04, 1.0, 32.0));
    for (i, c) in s.collider_cylinders.iter().enumerate() {
        let (r, ht, ys) = if i < 3 { (32, 50, -24) } else { (20, 60, -30) };
        assert_eq!((c.dim.radius, c.dim.height, c.dim.y_shift), (r, ht, ys), "cylinder {i}");
    }
    // EnSt_InitColliders: the body (Din's Fire, arrows, hookshot, hammer, boomerang, explosives,
    // Deku Nuts); the back (DMG_DEFAULT less those and light and ice magic, plus the Deku Stick:
    // EnSt_CheckBodyStickHit with unk_860 0); the front (DMG_DEFAULT less the body's), metal,
    // hookable, no AT info, ELEM_MATERIAL_UNK2.
    let c = &s.collider_cylinders;
    let body = cc::DMG_MAGIC_FIRE | cc::DMG_ARROW | cc::DMG_HOOKSHOT | cc::DMG_HAMMER_SWING | cc::DMG_BOOMERANG | cc::DMG_EXPLOSIVE | cc::DMG_DEKU_NUT;
    assert_eq!(c[0].info.ac_dmg_info.dmg_flags, body & !cc::DMG_DEKU_STICK);
    assert_eq!(c[1].info.ac_dmg_info.dmg_flags, (cc::DMG_DEFAULT & !body & !(cc::DMG_MAGIC_LIGHT | cc::DMG_MAGIC_ICE)) | cc::DMG_DEKU_STICK);
    assert_eq!(c[2].info.ac_dmg_info.dmg_flags, (cc::DMG_DEFAULT & !body) | cc::DMG_DEKU_STICK);
    assert_eq!(c[2].base.col_type, cc::COL_MATERIAL_METAL);
    assert_eq!(c[2].info.ac_elem_flags & !cc::ACELEM_HIT, cc::ACELEM_ON | cc::ACELEM_HOOKABLE | cc::ACELEM_NO_AT_INFO);
    assert_eq!(c[2].info.elem_material, cc::ELEM_MATERIAL_UNK2);
    for i in 3..6 {
        assert_eq!((c[i].base.ac_flags, c[i].base.oc_flags1 & !cc::OC1_HIT), (cc::AC_NONE, cc::OC1_ON | cc::OC1_TYPE_ALL));
    }
    // The sphere: limb 1, {0, -240, 0}, 28, AT 0xFFCFFFFF for 4.
    let e = &s.collider_jnt_sph.elements[0];
    assert_eq!((e.dim.limb, e.dim.model_sphere.center, e.dim.model_sphere.radius, e.info.at_dmg_info.damage), (1, [0, -240, 0], 28, 4));
    // DamageTable_Get(2), sColChkInit { 2, 0, 0, 0, MASS_IMMOVABLE }.
    assert_eq!(s.actor.col_chk_info.damage_table.unwrap().table, en_st::S_DAMAGE_TABLE_PRESET_2.table);
    assert_eq!((s.actor.col_chk_info.health, s.actor.col_chk_info.mass), (2, cc::MASS_IMMOVABLE));
    // The flags: CAN_ATTACH_TO_ARROW (1 << 14), SFX_FOR_PLAYER_BODY_HIT (cleared by Actor_UpdateAll
    // before each update: only the init sets it), not REACT_TO_LENS (params 2's);
    // NAVI_ENEMY_SKULLTULA (4).
    assert_ne!(s.actor.flags & (1 << 14), 0);
    assert_eq!(s.actor.flags & ACTOR_FLAG_REACT_TO_LENS, 0);
    assert_eq!(s.actor.navi_enemy_id, 4);
    // EnSt_CheckCeilingPos: the ceiling straight up (-560 here), 100 below its position kept
    // (unusedPos).
    assert_eq!(s.ceiling_pos, Vec3::new(HOME5.x, -560.0, HOME5.z));
    assert_eq!(s.unused_pos, HOME5 - Vec3::Y * 100.0);
    // The trail: Effect_Add finds no EffectBlure slot (TOTAL_EFFECT_COUNT, 3 + 25 + 3).
    assert_eq!(s.blure_idx, 31);
    // initialYaw, no gravity.
    assert_eq!((s.initial_yaw, s.actor.gravity), (16384, 0.0));
    // EnSt_StartOnCeilingOrGround (74 above its floor: not close to it): rotAwayTimer 60 and
    // EnSt_WaitOnCeiling; then EnSt_UpdateYaw, not on the ground, resets it to 30 and counts it
    // to 29.
    assert_eq!(s.actor.floor_height, -880.0);
    assert_eq!((s.action, s.rot_away_timer, s.rot_towards_timer), (Action::WaitOnCeiling, 29, 0));
    // Room 2's: its ceiling 40 up, its floor 468 below (more than EnSt_IsCloseToPlayer's 400:
    // Link on that floor never brings it down).
    let (w, h) = deku_tree_room(&a, 2);
    let s = st(&w, h);
    assert_eq!((s.actor.home_pos, s.actor.params), (HOME2, 0));
    assert_eq!(s.ceiling_pos, Vec3::new(HOME2.x, 788.0, HOME2.z));
    assert_eq!(s.actor.floor_height, 280.0);
}

#[test]
fn on_the_ceiling_it_bobs_by_the_frame_counter() {
    let Some(a) = assets() else { return };
    let (mut w, h) = deku_tree_room(&a, 5);
    // Link 300 off (and on a floor below its own): EnSt_IsCloseToPlayer is false.
    place_link(&mut w, 300.0);
    for _ in 0..40 {
        let (frames, p0, v0) = (w.state_frames, st(&w, h).actor.world_pos, st(&w, h).actor.velocity.y);
        idle(&mut w, 1);
        let s = st(&w, h);
        assert_eq!(s.action, Action::WaitOnCeiling);
        // Actor_UpdatePos first (the last frame's speed, R_UPDATE_RATE * 0.5 = 1.5 a unit), then
        // EnSt_Bob: Math_SmoothStepToF(&velocity.y, (frames & 8) ? -0.5 : 0.5, 0.4, 1000, 0).
        assert_eq!(s.actor.world_pos.y, p0.y + v0 * 1.5, "frame {frames}");
        let mut v = v0;
        smooth_step_to_f(&mut v, if frames & 8 != 0 { -0.5 } else { 0.5 }, 0.4, 1000.0, 0.0);
        assert_eq!(s.actor.velocity.y, v, "frame {frames}");
        // Facing its home yaw; the teeth black.
        assert_eq!((s.actor.shape_rot.y, s.rot_away_timer, s.teeth_r), (16384, 29, 0));
    }
}

#[test]
fn link_near_and_below_brings_it_down_to_land_wait_and_turn() {
    let Some(a) = assets() else { return };
    let (mut w, h) = deku_tree_room(&a, 5);
    place_link(&mut w, 300.0);
    idle(&mut w, 2);
    // Lifted 150 (its ceiling is at -560) so the drop is long enough to hear its rhythm; Link's
    // 225 below it then, within EnSt_IsCloseToPlayer's 400.
    let s = st_mut(&mut w, h);
    s.actor.world_pos.y = HOME5.y + 150.0;
    s.actor.prev_pos = s.actor.world_pos;
    s.actor.velocity.y = 0.0;
    // Link 100 in front, below it: EnSt_SetDropAnimAndVel (object_st_Anim_0055A8, animFrames its
    // length, sfxTimer 0, velocity -10), EnSt_MoveToGround.
    place_link(&mut w, 100.0);
    idle(&mut w, 1);
    let s = st(&w, h);
    assert_eq!((s.action, s.actor.velocity.y, s.sfx_timer), (Action::MoveToGround, -10.0, 0));
    assert!(s.skel.is("object_st_Anim_0055A8"));
    assert_eq!(s.anim_frames, s.skel.anim_length as i16);
    // EnSt_MoveToGround: down 15 a frame; NA_SE_EN_STALTU_DOWN when DECR(sfxTimer) is 0 (from 0:
    // at once, then every 3 frames); two frames' fall from 32 above the floor, the landing.
    let mut timer = 0i16;
    let mut downs = Vec::new();
    loop {
        let y0 = st(&w, h).actor.world_pos.y;
        idle(&mut w, 1);
        let s = st(&w, h);
        let y1 = y0 - 15.0;
        if (y1 + (-10.0 * 2.0)) - s.actor.floor_height <= 32.0 {
            // EnSt_SpawnBlastEffect, EnSt_SetLandAnimation: at the floor + 32, sfxTimer 0.
            assert_eq!((s.action, s.actor.world_pos.y, s.sfx_timer), (Action::LandOnGround, s.actor.floor_height + 32.0, 0));
            break;
        }
        assert_eq!(s.actor.world_pos.y, y1);
        // DECR: 0 stays 0, 1 goes to 0: the sound, and 3 again.
        let fired = timer <= 1;
        timer = if fired { 3 } else { timer - 1 };
        if fired {
            downs.push(w.audio.frames);
        }
        assert_eq!(s.sfx_timer, timer);
        assert_eq!(sfx_on(&w, w.audio.frames, NA_SE_EN_STALTU_DOWN), fired);
    }
    assert!(downs.len() >= 3, "{downs:?}");
    assert!(downs.windows(2).all(|d| d[1] - d[0] == 3), "{downs:?}");
    // The shockwave (EffectSsBlast_SpawnWhiteShockwaveSetScale(pos at the floor, 100, 220, 8)):
    // 5 above the floor; alpha step 255 / 8 = 31; EffectSs_UpdateAll ran once: life 7, the inner
    // alpha 224, the scale 320, its step 185 (220 - 35).
    let (fx, fy) = (st(&w, h).actor.world_pos, st(&w, h).actor.floor_height);
    let blasts = effects(&w, EFFECT_SS_BLAST);
    assert_eq!(blasts.len(), 1);
    let b = blasts[0];
    assert_eq!(b.pos, Vec3::new(fx.x, fy + 5.0, fx.z));
    assert_eq!((b.life, b.regs[3], b.regs[7], b.regs[8], b.regs[9], b.regs[10], b.regs[11]), (7, 224, 0, 31, 320, 185, 35));
    // EnSt_LandOnGround: sfxTimer up by one a frame, NA_SE_EN_STALTU_DOWN_SET at 14; down onto
    // the floor and back up (Math_SmoothStepToF(&velocity.y, 2, 0.3, 1, 0)) until above the
    // floor + 32: EnSt_WaitOnGround (sfxTimer 0).
    let mut n = 0;
    while st(&w, h).action == Action::LandOnGround {
        let v0 = st(&w, h).actor.velocity.y;
        idle(&mut w, 1);
        n += 1;
        let s = st(&w, h);
        assert_eq!(sfx_on(&w, w.audio.frames, NA_SE_EN_STALTU_DOWN_SET), n == 14, "frame {n}");
        if s.action == Action::LandOnGround {
            assert_eq!(s.sfx_timer, n);
            // (On the floor, Actor_UpdateBgCheckInfo zeroes the speed first.)
            if s.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
                let mut v = v0;
                smooth_step_to_f(&mut v, 2.0, 0.3, 1.0, 0.0);
                assert_eq!(s.actor.velocity.y, v, "frame {n}");
            }
        }
        assert!(n < 60);
    }
    assert!(n > 14);
    let s = st(&w, h);
    assert!(s.actor.world_pos.y > s.actor.floor_height + 32.0);
    // EnSt_WaitOnGround from the next frame: DECR(sfxTimer) from 0 is 0: NA_SE_EN_STALTU_LAUGH
    // at once, then every 64 frames. EnSt_UpdateYaw on the ground (already this frame): rotAwayTimer
    // (reset to 30 and counted to 29 by each landing frame, 28 now) counts down while it faces
    // Link, then at 0 NA_SE_EN_STALTU_ROLL and rotTowardsTimer 30 while it faces away, and back.
    // The teeth step by 255 / (s16)(0.6 * 8) = 63 towards red while (frames & 0x10), black
    // otherwise.
    assert_eq!((s.sfx_timer, s.rot_away_timer, s.rot_towards_timer), (0, 28, 0));
    for k in 1..=130u32 {
        let (frames, s0) = (w.state_frames, st(&w, h));
        let (teeth0, away0, towards0) = (s0.teeth_r, s0.rot_away_timer, s0.rot_towards_timer);
        idle(&mut w, 1);
        let s = st(&w, h);
        assert_eq!(s.action, Action::WaitOnGround);
        let mut t = teeth0 as i16;
        smooth_step_to_s(&mut t, if frames & 0x10 != 0 { 255 } else { 0 }, 1, 63, 63);
        assert_eq!(s.teeth_r, t as u8, "frame {k}");
        assert_eq!(sfx_on(&w, w.audio.frames, NA_SE_EN_STALTU_LAUGH), k % 64 == 1, "frame {k}");
        let rolled = sfx_on(&w, w.audio.frames, NA_SE_EN_STALTU_ROLL);
        if away0 != 0 {
            assert_eq!(s.rot_away_timer, away0 - 1);
            assert_eq!(rolled, away0 == 1);
            if away0 == 1 {
                assert_eq!(s.rot_towards_timer, 30);
            }
            if away0 == 2 {
                // The facing's last frame: towards Link (yawTowardsPlayer, 0x4000 here), give or
                // take the shake (0x800).
                assert!((s.actor.shape_rot.y.wrapping_sub(s.actor.yaw_towards_player) as i32).abs() <= 0x800);
            }
        } else {
            assert_eq!(s.rot_towards_timer, towards0 - 1);
            assert_eq!(rolled, towards0 == 1);
            if towards0 == 1 {
                assert_eq!(s.rot_away_timer, 30);
            }
            if towards0 == 2 {
                // Facing away (yawTowardsPlayer ^ 0x8000).
                assert!((s.actor.shape_rot.y.wrapping_sub(s.actor.yaw_towards_player ^ i16::MIN) as i32).abs() <= 0x800);
            }
        }
    }
}

#[test]
fn with_link_gone_it_climbs_back_to_its_ceiling() {
    let Some(a) = assets() else { return };
    let (mut w, h) = on_the_ground(&a);
    // Link 300 off: EnSt_SetReturnToCeilingAnimation (NA_SE_EN_STALTU_UP, ENST_ANIM_2: the
    // anim at 4 times speed, once), EnSt_ReturnToCeiling.
    place_link(&mut w, 300.0);
    idle(&mut w, 1);
    let s = st(&w, h);
    assert_eq!((s.action, s.skel.play_speed), (Action::ReturnToCeiling, 4.0));
    assert!(s.skel.is("object_st_Anim_000304"));
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EN_STALTU_UP));
    // EnSt_ReturnToCeiling: up at 4 × curFrame / (animLength - 1); within two frames' rise of its
    // home height, ENST_ANIM_3 and EnSt_WaitOnCeiling.
    let mut n = 0;
    loop {
        idle(&mut w, 1);
        n += 1;
        let s = st(&w, h);
        if s.action != Action::ReturnToCeiling {
            assert_eq!(s.action, Action::WaitOnCeiling);
            assert_eq!(s.skel.play_speed, 1.0);
            break;
        }
        // (At the animation's end, pct 1: up at 4, and the animation again from its start with
        // NA_SE_EN_STALTU_UP.)
        let pct = if sfx_on(&w, w.audio.frames, NA_SE_EN_STALTU_UP) { 1.0 } else { s.skel.cur_frame / (s.skel.anim_length - 1.0) };
        assert_eq!(s.actor.velocity.y, 4.0 * pct, "frame {n}");
        assert!(n < 200);
    }
    // EnSt_IsCloseToInitialPos: pos.y + velocity.y * 2 >= home.y.
    let s = st(&w, h);
    assert!(s.actor.world_pos.y + s.actor.velocity.y * 2.0 >= HOME5.y);
}

/// B for two frames (drawing the Kokiri Sword, then the slash), then nothing, until `done` or 30
/// frames.
fn slash(w: &mut PlayState, h: ActorHandle, done: impl Fn(&EnSt) -> bool) -> u32 {
    let mut prev = PadState::default();
    let b_press = with(PadState::default(), BTN_B);
    for i in 0..30 {
        tick(w, if i < 2 { b_press } else { PadState::default() }, &mut prev);
        if done(st(w, h)) {
            return w.audio.frames;
        }
    }
    panic!("the slash never landed: {:?}", st(w, h).action);
}

#[test]
fn a_slash_on_its_front_makes_it_sway_without_damage() {
    let Some(a) = assets() else { return };
    let (mut w, h) = on_the_ground(&a);
    // Link 50 in front. Held facing him (rotAwayTimer set high: the timers would turn it away
    // after 30 frames), so the front's cylinder (2) is its legs' AC (EnSt_SetLegsCylinderAC).
    place_link(&mut w, 50.0);
    st_mut(&mut w, h).rot_away_timer = 1000;
    idle(&mut w, 8);
    let f = slash(&mut w, h, |s| s.sway_timer != 0);
    // EnSt_CheckHitFrontside: invulnerableTimer 8 (7 after EnSt_UpdateCylinders), playSwayFlag 0,
    // swayTimer 60; no damage; then (not stunned, swaying) EnSt_Sway at once: swayAngle 0xA28,
    // swayTimer 59, the angle sin(0xA28) × 59 × 7/15 degrees, at 200 below its ceiling across
    // its facing, leaning by twice the angle.
    let s = st(&w, h);
    assert_eq!((s.sway_timer, s.sway_angle, s.invulnerable_timer, s.actor.col_chk_info.health), (59, 0xA28, 7, 2));
    assert!(!sfx_on(&w, f, NA_SE_EN_STALTU_DAMAGE));
    let rot_angle = (sin_s(0xA28) * (59.0 * (7.0f32 / 15.0f32) * (65536.0f32 / 360.0f32))) as i32 as i16;
    assert_eq!(s.abs_prev_sway_angle, rot_angle.abs());
    assert_eq!(s.actor.shape_rot.z, -(rot_angle * 2));
    let c = s.ceiling_pos;
    let mut m = MtxF::set_translate(c.x, c.y, c.z);
    m.rotate_y(binang_to_rad(s.actor.world_rot.y));
    let p = m.mult_vec3f(Vec3::new(sin_s(rot_angle) * -200.0, cos_s(rot_angle) * -200.0, 0.0));
    assert_eq!((s.actor.world_pos.x, s.actor.world_pos.z), (p.x, p.z));
    // 59 more frames of it (no Actor_UpdatePos, no action), NA_SE_EN_STALTU_WAVE as each swing
    // turns back; then swayAngle 0.
    for k in 1..=59 {
        idle(&mut w, 1);
        assert_eq!(st(&w, h).sway_timer, 59 - k, "frame {k}");
    }
    let s = st(&w, h);
    assert_eq!((s.sway_angle, s.actor.col_chk_info.health), (0, 2));
    assert!(!sfx_frames(&w, NA_SE_EN_STALTU_WAVE, f - 1).is_empty());
}

#[test]
fn a_slash_on_its_back_hurts_it_and_it_spins() {
    let Some(a) = assets() else { return };
    let (mut w, h) = on_the_ground(&a);
    // Link 50 in front, the Skulltula held facing away (rotTowardsTimer high): its back's
    // cylinder (1) is the legs' AC.
    place_link(&mut w, 50.0);
    let s = st_mut(&mut w, h);
    (s.rot_away_timer, s.rot_towards_timer) = (0, 1000);
    idle(&mut w, 12);
    let rot0 = st(&w, h).actor.shape_rot.y;
    let f = slash(&mut w, h, |s| s.actor.col_chk_info.health != 2);
    // EnSt_CheckHitBackside: the Kokiri Sword's DMG_ENTRY(1, 0): ENST_ANIM_3, takeDamageSpinTimer
    // its length, red for as long (Actor_SetColorFilter(RED, 200, OPA)), gaveDamageSpinTimer 1,
    // health 1: NA_SE_EN_STALTU_DAMAGE. Then the frame goes on: EnSt_WaitOnGround counts the spin
    // down once, and Link isn't "close" while it spins (EnSt_IsCloseToPlayer): back up
    // (NA_SE_EN_STALTU_UP); EnSt_UpdateYaw spins it by 0x2000; EnSt_UpdateCylinders counts the
    // spin down again (two a frame on the ground), gaveDamageSpinTimer to 0 and invulnerableTimer
    // to 7.
    let s = st(&w, h);
    let len = s.skel.anim_length as i16;
    assert_eq!(s.actor.col_chk_info.health, 1);
    assert!(sfx_on(&w, f, NA_SE_EN_STALTU_DAMAGE));
    assert!(sfx_on(&w, f, NA_SE_EN_STALTU_UP));
    assert_eq!(s.action, Action::ReturnToCeiling);
    assert_eq!((s.take_damage_spin_timer, s.gave_damage_spin_timer, s.invulnerable_timer), (len - 2, 0, 7));
    assert_eq!(s.actor.color_filter_params, COLORFILTER_COLORFLAG_RED | COLORFILTER_BUFFLAG_OPA | ((200 & 0xF8) << 5) | len as u16);
    assert_eq!(s.actor.color_filter_timer, len as u8);
    assert_ne!(s.actor.shape_rot.y, rot0);
    let r = s.actor.shape_rot.y;
    // Spinning on (0x2000 a frame) while the timer runs (after the slash's hit stop, which skips
    // its update).
    let mut n = 0;
    while st(&w, h).actor.shape_rot.y == r {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 3);
    }
    assert_eq!(st(&w, h).actor.shape_rot.y, r.wrapping_add(0x2000));
}

#[test]
fn a_killing_slash_bounces_it_three_times_then_it_burns_and_drops() {
    let Some(a) = assets() else { return };
    let (mut w, h) = on_the_ground(&a);
    place_link(&mut w, 50.0);
    // One Kokiri slash from dead (a first one took it from 2 to 1: see the test above), held
    // facing away.
    let s = st_mut(&mut w, h);
    (s.rot_away_timer, s.rot_towards_timer) = (0, 1000);
    s.actor.col_chk_info.health = 1;
    idle(&mut w, 12);
    let items_before: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&i| w.actors.downcast::<EnItem00>(i).is_some()).collect();
    let f = slash(&mut w, h, |s| s.actor.col_chk_info.health == 0);
    // EnSt_CheckHitBackside at 0 health: Enemy_StartFinishingBlow (freezeFlashTimer 5), not
    // targetable, groundBounces 3, deathTimer 20, gravity -1, NA_SE_EN_STALWALL_DEAD,
    // EnSt_BounceAround (a sword: not an arrow); the update ends there.
    let s = st(&w, h);
    assert_eq!((s.action, s.ground_bounces, s.death_timer, s.actor.gravity), (Action::BounceAround, 3, 20, -1.0));
    assert_eq!(s.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(w.actors.freeze_flash_timer, 5);
    assert!(sfx_on(&w, f, NA_SE_EN_STALWALL_DEAD));
    // EnSt_BounceAround: falling by 1, tumbling; on the ground, NA_SE_EN_DODO_M_GND, eleven puffs
    // of dust (EnSt_SpawnDust(10): from 10 down to 0), and up at 6 / (4 - groundBounces): 6, 3,
    // then 2 at the last, which is set to 0: EnSt_FinishBouncing (yOffset 400, speed 1, gravity
    // -2).
    let mut bounces = Vec::new();
    let mut n = 0;
    while st(&w, h).action == Action::BounceAround {
        let gb = st(&w, h).ground_bounces;
        idle(&mut w, 1);
        n += 1;
        assert!(n < 200, "still bouncing");
        let s = st(&w, h);
        if s.ground_bounces != gb {
            assert!(sfx_on(&w, w.audio.frames, NA_SE_EN_DODO_M_GND));
            // EffectSs_UpdateAll ran once on them: life 9.
            assert_eq!(effects(&w, EFFECT_SS_DUST).iter().filter(|e| e.life == 9).count(), 11);
            bounces.push((gb, s.actor.velocity.y));
        } else if s.action == Action::BounceAround {
            assert_eq!(s.actor.color_filter_timer, 20);
        }
    }
    assert_eq!(bounces, vec![(3, 6.0), (2, 3.0), (1, 0.0)]);
    let s = st(&w, h);
    assert_eq!((s.action, s.actor.shape_y_offset, s.actor.speed_xz, s.actor.gravity), (Action::FinishBouncing, 400.0, 1.0, -2.0));
    // EnSt_FinishBouncing: DECR(deathTimer) from 20 (the 20th frame: EnSt_Die, finishDeathTimer 8,
    // stopped).
    for k in 1..=20 {
        idle(&mut w, 1);
        let s = st(&w, h);
        if k < 20 {
            assert_eq!((s.action, s.death_timer), (Action::FinishBouncing, 20 - k), "frame {k}");
        } else {
            assert_eq!((s.action, s.finish_death_timer, s.actor.velocity), (Action::Die, 8, Vec3::ZERO));
        }
    }
    // EnSt_Die: DECR(finishDeathTimer) from 8: seven flames (EffectSsDeadDb, 9 frames, white to
    // red), one a frame; at 0, Item_DropCollectibleRandom(table 14) and Actor_Kill.
    for k in 1..=7 {
        let before = effects(&w, EFFECT_SS_DEAD_DB).len();
        idle(&mut w, 1);
        let s = st(&w, h);
        assert_eq!(s.finish_death_timer, 8 - k as i16);
        let now = effects(&w, EFFECT_SS_DEAD_DB);
        assert_eq!(now.len(), before + 1, "frame {k}");
        assert!(now.iter().any(|e| e.life == 8 && e.regs[0] == 100));
    }
    let last = st(&w, h).actor.world_pos;
    idle(&mut w, 1);
    let gone = w.actors.downcast::<EnSt>(h).is_none() || w.actors.actor(h).is_some_and(|a| a.killed);
    assert!(gone);
    // Table 14 (sItemDropIds[14 * 16 + Rand_ZeroOne() * 16]): at most one, where it died.
    let ids = &a.item_drops.ids[14 * 16..14 * 16 + 16];
    let new_items: Vec<&EnItem00> = w.actors.all().into_iter().filter(|i| !items_before.contains(i)).filter_map(|i| w.actors.downcast::<EnItem00>(i)).collect();
    assert!(new_items.len() <= 1);
    for it in &new_items {
        assert!(it.actor.home_pos.distance(last) < 1.0);
        assert!(ids.contains(&(it.actor.params as u8)));
    }
}

#[test]
fn a_deku_nut_stuns_it_for_120_frames() {
    let Some(a) = assets() else { return };
    let (mut w, h) = on_the_ground(&a);
    let player = w.player.unwrap();
    // A Deku Nut (Link can't throw one yet): its hit on the body's cylinder, as
    // CollisionCheck_SetATvsAC leaves it; this frame's CollisionCheck_Damage then gives
    // DamageTable_Get(2)'s DMG_ENTRY(0, 1).
    let s = st_mut(&mut w, h);
    let c = &mut s.collider_cylinders[0];
    c.base.ac_flags |= cc::AC_HIT;
    c.info.ac_elem_flags |= cc::ACELEM_HIT;
    c.info.ac_hit_elem = Some(cc::HitElem {
        elem: cc::ElemRef { col: cc::ColliderRef { actor: player, id: 0 }, elem: 0 },
        at_dmg_info: cc::ColliderElementDamageInfoAT { dmg_flags: cc::DMG_DEKU_NUT, hit_special_effect: cc::HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: cc::ColliderElementDamageInfoAC { dmg_flags: 0, hit_backlash: 0, defense: 0, hit_pos: [0; 3] },
        elem_material: cc::ELEM_MATERIAL_UNK0,
    });
    let rot0 = s.actor.shape_rot.y;
    idle(&mut w, 1);
    // EnSt_CheckHitBackside (damageReaction 1): NA_SE_EN_GOMA_JR_FREEZE, stunTimer 120, blue
    // for 120 (Actor_SetColorFilter(BLUE, 200, OPA)), invulnerableTimer 8 (7); no damage. The
    // frame goes on stunned: no animation, no move, EnSt_DecrStunTimer (119).
    let s = st(&w, h);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EN_GOMA_JR_FREEZE));
    assert_eq!((s.stun_timer, s.actor.col_chk_info.health, s.invulnerable_timer), (119, 2, 7));
    assert_eq!(s.actor.color_filter_params, COLORFILTER_COLORFLAG_BLUE | COLORFILTER_BUFFLAG_OPA | ((200 & 0xF8) << 5) | 120);
    assert_eq!(s.actor.shape_rot.y, rot0);
    // Counting down 1 a frame, still (frame and position); under 30, EnSt_UpdateYaw shakes it:
    // +0x800 on odd counts, -0x800 on even.
    let (frame0, pos0) = (s.skel.cur_frame, s.actor.world_pos);
    for k in 1..=119 {
        let r0 = st(&w, h).actor.shape_rot.y;
        idle(&mut w, 1);
        let s = st(&w, h);
        let t = 119 - k;
        assert_eq!(s.stun_timer, t as i16, "frame {k}");
        if t == 0 {
            break;
        }
        assert_eq!((s.skel.cur_frame, s.actor.world_pos), (frame0, pos0));
        let want = if t >= 30 {
            r0
        } else if t % 2 != 0 {
            r0.wrapping_add(0x800)
        } else {
            r0.wrapping_sub(0x800)
        };
        assert_eq!(s.actor.shape_rot.y, want, "frame {k}");
    }
}

#[test]
fn touching_link_hurts_him_and_knocks_him_down() {
    let Some(a) = assets() else { return };
    let (mut w, h) = on_the_ground(&a);
    let health = w.save.health;
    // Link 10 in front: in its middle OC cylinder (20 across).
    place_link(&mut w, 10.0);
    let mut n = 0;
    while st(&w, h).gave_damage_spin_timer == 0 {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 5, "it never touched Link");
    }
    // EnSt_CheckHitPlayer (an OC cylinder's OC2_HIT_PLAYER): NA_SE_EN_STALTU_ROLL (not swaying),
    // gaveDamageSpinTimer 30 (29 after EnSt_UpdateCylinders the same frame),
    // play->damagePlayer(play, -8): half a heart, NA_SE_PL_BODY_HIT at Link,
    // Actor_SetPlayerKnockbackLargeNoDamage(4, yawTowardsPlayer, 6): a knockdown (type 2) for
    // Player's next update.
    let f = w.audio.frames;
    let s = st(&w, h);
    assert_eq!(s.gave_damage_spin_timer, 29);
    assert!(sfx_on(&w, f, NA_SE_EN_STALTU_ROLL));
    assert!(sfx_on(&w, f, NA_SE_PL_BODY_HIT));
    assert_eq!(w.save.health, health - 8);
    let yaw = s.actor.yaw_towards_player;
    let p = w.player();
    assert_eq!((p.knockback_type, p.knockback_damage, p.knockback_rot, p.knockback_speed, p.knockback_y_velocity), (2, 0, yaw, 4.0, 6.0));
    // The spin: 0x2000 a frame while gaveDamageSpinTimer runs (no OC meanwhile:
    // EnSt_SetCylinderOC waits for it).
    let r = s.actor.shape_rot.y;
    idle(&mut w, 1);
    let s = st(&w, h);
    assert_eq!((s.gave_damage_spin_timer, s.actor.shape_rot.y), (28, r.wrapping_add(0x2000)));
}
