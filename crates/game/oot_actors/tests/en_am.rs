//! GAME-09 milestone 3 against the C: the Armos (`z_en_am.c`). Dodongo's Cavern's room 8 (Master
//! Quest, `ddan_room_8`) places seven enemies (params 0xFFFF), in a column at x 1290 to 1345 on
//! the room's south floor (y 16), facing east (0x4000); the statue (params 0) is placed in Ganon's
//! Castle (an adult's), so its test spawns one in room 8.
//!
//! Child Link enters the cavern at room 7 (`ENTR_DODONGOS_CAVERN_1`) on a `cavern-bombs` save
//! (the Kokiri Sword, the slingshot, 20 bombs) and goes into room 8 (`Room_RequestNewRoom`).
//! Expected values are worked out from the C in the comments. The numbers they use:
//! - `Actor_UpdatePos` moves by `velocity × R_UPDATE_RATE / 2` (1.5); gravity -4: a hop (`vy` 12
//!   at the hop animation's frame 8) rises 12, 6, 0 and falls 6, 12: five frames in the air, its
//!   top 18 up; 6 on is 9 a frame;
//! - `gArmosHopAnim` played at 4 a frame: frames 0, 4, 8 (the hop), 12, then held at 11 in the
//!   air (11 + 4 = 15), the landing at 15 (19 after), 0 on the next frame;
//! - the waking: `textureBlend` up 20 a frame from 0; 240 or more wakes it (`attackTimer` 200).

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_B, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_am::*;
use oot_actors::en_bom::{ACTOR_EN_BOM, EnBom};
use oot_actors::player::{Action as PA, STATE2_0};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_ENEMY, ActorHandle};
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, NA_SE_EV_ROCK_SLIDE, SFX_FLAG, SfxPos};
use oot_game::collision_check::{AC_HIT, MASS_HEAVY};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Room 8's seven (`ootx scene-info --scene ddan --layer 0`), north to south.
const HOMES: [Vec3; 7] = [
    Vec3::new(1339.0, 16.0, -3744.0),
    Vec3::new(1309.0, 16.0, -3768.0),
    Vec3::new(1345.0, 16.0, -3800.0),
    Vec3::new(1317.0, 16.0, -3837.0),
    Vec3::new(1344.0, 16.0, -3871.0),
    Vec3::new(1309.0, 16.0, -3900.0),
    Vec3::new(1290.0, 16.0, -3942.0),
];

/// The gravestones' floor north of the room's wall, out of the Armos' way.
const NORTH_FLOOR: Vec3 = Vec3::new(1500.0, 16.0, -3600.0);

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn frame(w: &mut PlayState, prev: PadState, cur: PadState) -> PadState {
    w.tick_with(scripted_input(prev, cur));
    cur
}

/// One frame with B pressed (not held the frame before).
fn slash(w: &mut PlayState) {
    w.tick_with(scripted_input(PadState::default(), with(PadState::default(), BTN_B)));
}

/// Child Link in room 8 (`cavern-bombs`, the sound log on), entered at room 7 and the room
/// changed; Link on the gravestones' floor, two frames on.
fn room8(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_1").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("cavern-bombs").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert!(w.room_request(8));
    idle(&mut w, 1);
    w.room_change_done();
    idle(&mut w, 3);
    w.place_player(NORTH_FLOOR, 0);
    idle(&mut w, 2);
    w
}

fn all_armos(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnAm>(h).is_some_and(|a| !a.actor.killed)).collect()
}

/// The Armos whose home is `home`.
fn at_home(w: &PlayState, home: Vec3) -> ActorHandle {
    *all_armos(w).iter().find(|&&h| am(w, h).actor.home_pos == home).expect("the Armos")
}

fn am(w: &PlayState, h: ActorHandle) -> &EnAm {
    w.actors.downcast::<EnAm>(h).expect("En_Am")
}

fn am_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnAm {
    w.actors.downcast_mut::<EnAm>(h).expect("En_Am")
}

fn anim_name(a: &EnAm) -> &str {
    a.skel.animation.as_ref().map(|a| a.name.as_str()).unwrap_or("")
}

/// The frames a sound was asked for at `pos`.
fn sfx_frames(w: &PlayState, id: u16, pos: Option<SfxPos>) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && pos.is_none_or(|q| q == p)).map(|&(f, _, _)| f).collect()
}

/// What the chase's test compares frame to frame.
struct Snap {
    pos: Vec3,
    unk_264: i16,
    unk_258: i16,
    yaw: i16,
    action: Action,
}

fn snap(x: &EnAm) -> Snap {
    Snap { pos: x.actor.world_pos, unk_264: x.unk_264, unk_258: x.unk_258, yaw: x.actor.world_rot.y, action: x.action }
}

/// Runs until `done` or `max` frames; the frames run.
fn run_until(w: &mut PlayState, max: usize, mut done: impl FnMut(&PlayState) -> bool) -> usize {
    for n in 0..max {
        if done(w) {
            return n;
        }
        idle(w, 1);
    }
    panic!("not done in {max} frames");
}

/// Wakes `h` as Link's touch does (`unk_258` 1: `EnAm_Sleep` goes on waking) and runs until it
/// lunges (13 frames).
fn wake(w: &mut PlayState, h: ActorHandle) {
    am_mut(w, h).unk_258 = 1;
    let n = run_until(w, 20, |w| am(w, h).action == Action::Lunge);
    assert_eq!(n, 13, "12 frames turning, the 13th up");
}

/// A hit as `CollisionCheck_AC` leaves it on the body for the next update: `AC_HIT`, the table's
/// damage and reaction in `colChkInfo`.
fn hit(w: &mut PlayState, h: ActorHandle, damage: u8, reaction: u8) {
    let a = am_mut(w, h);
    a.hurt_collider.base.ac_flags |= AC_HIT;
    a.actor.col_chk_info.damage = damage;
    a.actor.col_chk_info.damage_reaction = reaction;
}

#[test]
fn room_8s_seven_armos_sleep_as_statues() {
    let Some(a) = assets() else { return };
    let w = room8(&a);
    let hs = all_armos(&w);
    assert_eq!(hs.len(), 7);
    for home in HOMES {
        let h = at_home(&w, home);
        let x = am(&w, h);
        // params 0xFFFF: not ARMOS_STATUE, the enemy. EnAm_Init: health 1, sDamageTable, heavy;
        // EnAm_SetupSleep: the ricochet's last frame still, AM_BEHAVIOR_DO_NOTHING; unk_258 0
        // after it. sInitChain: NAVI_ENEMY_ARMOS, gravity -4, lockOnArrowOffset 5300; scale 0.01.
        assert_eq!((x.actor.params, x.action, x.behavior, x.texture_blend, x.unk_258), (-1, Action::Sleep, AM_BEHAVIOR_DO_NOTHING, 0, 0));
        assert_eq!((x.actor.col_chk_info.health, x.actor.col_chk_info.mass, x.actor.category), (1, MASS_HEAVY, ACTORCAT_ENEMY));
        assert_eq!((x.actor.navi_enemy_id, x.actor.gravity, x.actor.target_arrow_offset, x.actor.scale.x), (NAVI_ENEMY_ARMOS, -4.0, 5300.0, 0.01));
        assert_eq!((anim_name(x), x.skel.play_speed, x.skel.cur_frame), ("gArmosRicochetAnim", 0.0, x.skel.end_frame));
        // DynaPolyActor_Init: no collision of its own (bgId -1: EnAm_Sleep's unk_264 0).
        assert_eq!(x.bg, eng_collision::dyna::BG_ACTOR_MAX);
        // EnAm_Sleep with nothing touching it: the textures at the statue's (0), not targetable;
        // its push (dyna.unk_150, nothing gives the enemy one) 0, world.rot.y dyna.unk_158 (the
        // zeroed actor's 0); facing its placement's 0x4000.
        assert_eq!(x.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
        assert_eq!((x.actor.world_rot.y, x.actor.shape_rot.y, x.actor.speed_xz), (0, 0x4000, 0.0));
        // Actor_SetFocus(scale.x × 4500): 45 up.
        assert_eq!(x.actor.focus_pos.y, x.actor.world_pos.y + 45.0);
        // sHurtCylinderInit and sBlockCylinderInit: 15 across, 70 high.
        assert_eq!((x.hurt_collider.dim.radius, x.hurt_collider.dim.height, x.block_collider.dim.radius, x.block_collider.dim.height), (15, 70, 15, 70));
    }
}

#[test]
fn link_bumping_into_an_armos_wakes_it() {
    let Some(a) = assets() else { return };
    let mut w = room8(&a);
    let h = at_home(&w, HOMES[3]);
    let others: Vec<ActorHandle> = all_armos(&w).into_iter().filter(|&o| o != h).collect();
    // Link 22 east of it, facing it: his cylinder overlaps its hurt cylinder (15 across; OC1_ON,
    // OC1_TYPE_ALL). The frame he's placed, CollisionCheck_OC sets OC1_HIT with him as `oc`; the
    // next frame's EnAm_Sleep reads it: its cry (NA_SE_EN_AMOS_WAVE, _VOICE), a red flash (8),
    // textureBlend 20 and unk_258 1.
    let pos = am(&w, h).actor.world_pos;
    w.place_player(pos + Vec3::new(22.0, 0.0, 0.0), -0x4000);
    idle(&mut w, 1);
    assert_eq!(am(&w, h).texture_blend, 0);
    idle(&mut w, 1);
    let w0 = w.audio.frames;
    let x = am(&w, h);
    assert_eq!((x.texture_blend, x.unk_258, x.action), (20, 1, Action::Sleep));
    assert_eq!(x.actor.color_filter_params & 0xC000, COLORFILTER_COLORFLAG_RED);
    assert_eq!(sfx_frames(&w, NA_SE_EN_AMOS_WAVE, Some(SfxPos::Actor(h))), [w0]);
    assert_eq!(sfx_frames(&w, NA_SE_EN_AMOS_VOICE, Some(SfxPos::Actor(h))), [w0]);
    // Waking: 20 more a frame, shaking up to 5 either way along its facing's cosine and sine
    // about where it stood (x: cos 0x4000 is 0, z: sin is 1).
    let origin = am(&w, h).shake_origin;
    for k in 1..12u32 {
        idle(&mut w, 1);
        let x = am(&w, h);
        assert_eq!((x.texture_blend as u32, x.action), (20 * (k + 1), Action::Sleep), "frame {k}");
        assert_eq!(x.actor.world_pos.x, origin.x);
        assert!((x.actor.world_pos.z - origin.z).abs() <= 5.0, "{} about {}", x.actor.world_pos.z, origin.z);
    }
    // 240: awake. attackTimer 200, the enemy's textures (255), targetable, EnAm_SetupLunge: the
    // hop animation at 4, three hops (unk_258), AM_BEHAVIOR_AGGRO, facing its shape's way.
    idle(&mut w, 1);
    let x = am(&w, h);
    assert_eq!((x.action, x.behavior, x.texture_blend, x.attack_timer, x.unk_258), (Action::Lunge, AM_BEHAVIOR_AGGRO, 255, 200, 3));
    assert_eq!((anim_name(x), x.skel.play_speed, x.actor.world_rot.y), ("gArmosHopAnim", 4.0, x.actor.shape_rot.y));
    assert_ne!(x.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    // No other Armos woke.
    for o in others {
        assert_eq!((am(&w, o).action, am(&w, o).texture_blend), (Action::Sleep, 0));
    }
    // Asleep again only far from home or after attackTimer: no sound of a second waking.
    assert_eq!(sfx_frames(&w, NA_SE_EN_AMOS_WAVE, Some(SfxPos::Actor(h))).len(), 1);
}

#[test]
fn an_awake_armos_hops_at_link_three_times_and_rests() {
    let Some(a) = assets() else { return };
    let mut w = room8(&a);
    let h = at_home(&w, HOMES[3]);
    // Link 200 east of it on the south floor, still.
    let link = HOMES[3] + Vec3::new(200.0, 0.0, 0.0);
    w.place_player(link, -0x4000);
    idle(&mut w, 2);
    wake(&mut w, h);
    let mut takeoffs = Vec::new();
    let mut landings = Vec::new();
    let mut prev = snap(am(&w, h));
    for f in 0..60 {
        idle(&mut w, 1);
        let x = am(&w, h);
        let now = snap(x);
        // The hop's frame (curFrame 8): 12 up, 6 on (EnAm_CanMove 80 ahead: the south floor),
        // the sword's quad on; moved this frame: 12 up and 9 on.
        if now.unk_264 == 1 && prev.unk_264 == 0 {
            takeoffs.push(f);
            assert_eq!((x.actor.velocity.y, x.actor.speed_xz), (8.0, 6.0), "the hop's frame {f}");
            assert_eq!(now.pos.y - prev.pos.y, 12.0);
            assert!(((now.pos - prev.pos).with_y(0.0).length() - 9.0).abs() < 0.01, "{:?} to {:?}", prev.pos, now.pos);
        }
        // The landing (curFrame over 11 on the ground, BGCHECKFLAG_GROUND_TOUCH): a hop less, still,
        // the quad off, NA_SE_EN_AMOS_WALK, turned to Link by up to 0x1770.
        if now.unk_258 < prev.unk_258 && x.action == Action::Lunge || (x.action == Action::Cooldown && prev.action == Action::Lunge) {
            landings.push(f);
            assert_eq!((x.actor.speed_xz, x.unk_264, x.actor.world_pos.y), (0.0, 0, 16.0));
            assert!(sfx_frames(&w, NA_SE_EN_AMOS_WALK, Some(SfxPos::Actor(h))).contains(&w.audio.frames));
            let turned = (now.yaw.wrapping_sub(prev.yaw) as i32).abs();
            assert!(turned <= 0x1770, "turned {turned:#x}");
        }
        if x.action == Action::Cooldown {
            break;
        }
        prev = now;
    }
    // Three hops, each five frames in the air, landing on the sixth frame after its take-off,
    // the next take-off four frames on (the landing at 15, then 19, 0, 4, 8).
    assert_eq!(takeoffs.len(), 3, "takeoffs {takeoffs:?} landings {landings:?}");
    assert_eq!(landings.len(), 3, "takeoffs {takeoffs:?} landings {landings:?}");
    for i in 0..3 {
        assert_eq!(landings[i] - takeoffs[i], 5, "takeoffs {takeoffs:?} landings {landings:?}");
    }
    assert_eq!(takeoffs[1] - landings[0], 4);
    // After the third: EnAm_SetupCooldown, 40 frames' rest (the hop animation from 0, still).
    let x = am(&w, h);
    assert_eq!((x.action, x.cooldown_timer, x.unk_258, x.skel.cur_frame), (Action::Cooldown, 40, 3, 0.0));
    for t in (0..40).rev() {
        idle(&mut w, 1);
        assert_eq!((am(&w, h).cooldown_timer, am(&w, h).skel.cur_frame), (t, 0.0));
    }
    // Then hops round to Link (up to 0x1F40 at the hop's frame), and once it lands facing him
    // within 3500, lunges again with NA_SE_EN_AMOS_VOICE.
    let n = run_until(&mut w, 60, |w| am(w, h).action == Action::Lunge);
    assert!(n > 4);
    assert!(sfx_frames(&w, NA_SE_EN_AMOS_VOICE, Some(SfxPos::Actor(h))).contains(&w.audio.frames));
    let x = am(&w, h);
    assert!((x.actor.yaw_towards_player.wrapping_sub(x.actor.world_rot.y) as i32).abs() < 3500 + 0x1F40);
}

#[test]
fn its_metal_turns_the_kokiri_sword_away() {
    let Some(a) = assets() else { return };
    let mut w = room8(&a);
    let h = at_home(&w, HOMES[3]);
    // Asleep: the slash bounces off its block cylinder (AC_HARD, the Kokiri slash 0x100 in its
    // 0x00400106): EnAm_UpdateDamage clears the block's AC_HIT and AC_BOUNCED and the body's
    // AC_HIT; behavior DO_NOTHING (3) is under AM_BEHAVIOR_5: no ricochet, and EnAm_Sleep finds no
    // hit: it sleeps on.
    let pos = am(&w, h).actor.world_pos;
    w.place_player(pos + Vec3::new(32.0, 0.0, 0.0), -0x4000);
    idle(&mut w, 2);
    slash(&mut w);
    // The swing bounced (AT_BOUNCED): Link's rebound (func_80842D20).
    run_until(&mut w, 12, |w| w.player().action == PA::Rebound);
    idle(&mut w, 2);
    for o in all_armos(&w) {
        assert_eq!((am(&w, o).action, am(&w, o).texture_blend), (Action::Sleep, 0), "{:?} asleep", am(&w, o).actor.home_pos);
    }
    let x = am(&w, h);
    assert_eq!((x.action, x.texture_blend, x.actor.col_chk_info.health), (Action::Sleep, 0, 1));
    // Awake (resting after its hops), the same slash: EnAm_SetupRicochet: the ricochet's frames 0
    // to 8, facing Link, knocked back 6 (the floor behind it), AM_BEHAVIOR_RICOCHET, cooldownTimer
    // 5; its health untouched.
    w.place_player(NORTH_FLOOR, 0);
    idle(&mut w, 2);
    wake(&mut w, h);
    run_until(&mut w, 80, |w| am(w, h).action == Action::Cooldown);
    let pos = am(&w, h).actor.world_pos;
    let yaw = am(&w, h).actor.shape_rot.y;
    // In front of it, 32 off, facing it.
    let front = pos + Vec3::new(eng_math::sin_s(yaw), 0.0, eng_math::cos_s(yaw)) * 32.0;
    w.place_player(front, yaw.wrapping_add(i16::MIN));
    idle(&mut w, 1);
    slash(&mut w);
    let n = run_until(&mut w, 12, |w| am(w, h).action == Action::Ricochet);
    assert!(n <= 10);
    let x = am(&w, h);
    assert_eq!((x.behavior, x.cooldown_timer, x.unk_264, x.unk_258), (AM_BEHAVIOR_RICOCHET, 5, 0, 0));
    assert_eq!((anim_name(x), x.skel.end_frame, x.actor.col_chk_info.health), ("gArmosRicochetAnim", 8.0, 1));
    assert_eq!(x.actor.world_rot.y, x.actor.yaw_towards_player);
    // Knocked back: -6, slowing by 0.5 a frame; the animation played, it lunges again.
    assert!(x.actor.speed_xz < 0.0);
    let n = run_until(&mut w, 20, |w| am(w, h).action == Action::Lunge);
    assert_eq!(n, 9, "frames 0 to 8 at 1 a frame, then SkelAnime_Update's end");
    assert_eq!(am(&w, h).actor.speed_xz, 0.0);
}

#[test]
fn a_deku_nut_stuns_it_and_then_the_sword_kills_it() {
    let Some(a) = assets() else { return };
    let mut w = room8(&a);
    let h = at_home(&w, HOMES[3]);
    wake(&mut w, h);
    run_until(&mut w, 80, |w| am(w, h).action == Action::Cooldown);
    // A Deku Nut (DMG_ENTRY(0, AM_DMG_REACT_NUT)): EnAm_SetupStunned: the damaged animation set
    // but never updated, facing Link, blue 100 frames (COLORFILTER_COLORFLAG_BLUE), behavior
    // STUNNED, NA_SE_EN_GOMA_JR_FREEZE; no damage, its health kept.
    hit(&mut w, h, 0, AM_DMG_REACT_NUT);
    idle(&mut w, 1);
    let x = am(&w, h);
    assert_eq!((x.action, x.behavior, x.damage_reaction, x.actor.col_chk_info.health), (Action::Stunned, AM_BEHAVIOR_STUNNED, AM_DMG_REACT_NUT, 1));
    assert_eq!((anim_name(x), x.actor.color_filter_params & 0xC000), ("gArmosDamagedAnim", COLORFILTER_COLORFLAG_BLUE));
    assert!(x.actor.color_filter_timer >= 98, "{}", x.actor.color_filter_timer);
    assert!(sfx_frames(&w, NA_SE_EN_GOMA_JR_FREEZE, Some(SfxPos::Actor(h))).contains(&w.audio.frames));
    // A second nut while stunned: no restart (behavior STUNNED), nor sparks (not AM_DMG_REACT_STUN).
    hit(&mut w, h, 0, AM_DMG_REACT_NUT);
    idle(&mut w, 1);
    assert_eq!((am(&w, h).action, am(&w, h).actor.col_chk_info.health), (Action::Stunned, 1));
    // Stunned, its block cylinder isn't registered (colorFilterTimer on): the Kokiri Sword's slash
    // reaches its body: AM_DMG_REACT_NONE, but behavior STUNNED: health 0,
    // EnAm_SetupRecoilFromDamage (NA_SE_EN_AMOS_DAMAGE, the finishing blow's freeze 5, its colour
    // cleared).
    let pos = am(&w, h).actor.world_pos;
    let to_link = am(&w, h).actor.yaw_towards_player;
    let front = pos + Vec3::new(eng_math::sin_s(to_link), 0.0, eng_math::cos_s(to_link)) * 32.0;
    w.place_player(front, to_link.wrapping_add(i16::MIN));
    idle(&mut w, 1);
    slash(&mut w);
    run_until(&mut w, 12, |w| am(w, h).action == Action::RecoilFromDamage);
    let x = am(&w, h);
    assert_eq!((x.behavior, x.actor.col_chk_info.health, x.damage_reaction), (AM_BEHAVIOR_DAMAGED, 0, AM_DMG_REACT_NONE));
    assert_eq!((anim_name(x), x.skel.start_frame, x.skel.end_frame), ("gArmosDamagedAnim", 4.0, x.skel.animation.as_ref().unwrap().last_frame() - 6.0));
    assert!(sfx_frames(&w, NA_SE_EN_AMOS_DAMAGE, Some(SfxPos::Actor(h))).contains(&w.audio.frames));
    assert_eq!(w.actors.freeze_flash_timer, 5);
    death_and_blast(&mut w, h);
}

/// From `EnAm_RecoilFromDamage`: the animation out, the death's hops (`deathTimer` 64, down to
/// 63 the same frame): still while it's 52 or more, then spinning faster by 800 up to 8000 a
/// frame, red for 4 frames each time it's a multiple of 4; at 0 its own bomb (`En_Bom`, `timer`
/// 0, `shape.rot.z` 2), `NA_SE_EN_AMOS_DEAD`, and it's gone.
fn death_and_blast(w: &mut PlayState, h: ActorHandle) {
    run_until(w, 40, |w| am(w, h).death_timer != 0);
    let x = am(w, h);
    assert_eq!((x.action, x.death_timer, x.panic_spin_rot), (Action::Lunge, 63, 0));
    let still = am(w, h).actor.world_pos;
    let mut spin = Vec::new();
    for t in (1..63).rev() {
        let before = am(w, h).actor.shape_rot.y;
        idle(w, 1);
        let x = am(w, h);
        assert_eq!(x.death_timer, t);
        if t >= 51 {
            // Lunge saw 52 or more: nothing moved.
            assert_eq!(x.actor.world_pos.with_y(0.0), still.with_y(0.0), "still at {t}");
        } else {
            spin.push(x.panic_spin_rot);
            assert_eq!(x.actor.shape_rot.y, before.wrapping_add(x.panic_spin_rot));
        }
        if t % 4 == 0 {
            assert_eq!(x.actor.color_filter_params & 0xC000, COLORFILTER_COLORFLAG_RED);
        }
    }
    assert_eq!(&spin[..3], &[800, 1600, 2400]);
    assert_eq!(*spin.last().unwrap(), 8000);
    let pos = am(w, h).actor.world_pos;
    idle(w, 1);
    assert!(w.actors.get(h).is_none_or(|a| a.base().killed), "the Armos is gone");
    assert!(sfx_frames(w, NA_SE_EN_AMOS_DEAD, None).contains(&w.audio.frames));
    let bombs: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&b| w.actors.downcast::<EnBom>(b).is_some()).collect();
    assert_eq!(bombs.len(), 1);
    let b = w.actors.downcast::<EnBom>(bombs[0]).unwrap();
    // EnBom_Init: shape.rot.z's low byte (2) kept as the growth; timer 0 from the Armos: it blows
    // in its first update (the next frame).
    assert_eq!((b.actor.id, b.actor.shape_rot.z, b.timer), (ACTOR_EN_BOM, 2, 0));
    assert!(b.actor.world_pos.distance(pos) < 0.01);
}

#[test]
fn a_bomb_kills_an_awake_armos_and_its_own_blast_hurts_link_and_wakes_and_kills_others() {
    let Some(a) = assets() else { return };
    let mut w = room8(&a);
    // Two awake: the middle one (3) and its southern neighbour (4), 43 apart.
    let h = at_home(&w, HOMES[3]);
    let o = at_home(&w, HOMES[4]);
    let s = at_home(&w, HOMES[2]);
    wake(&mut w, h);
    am_mut(&mut w, o).unk_258 = 1;
    run_until(&mut w, 80, |w| am(w, h).action == Action::Cooldown && am(w, o).action != Action::Sleep);
    // A bomb at its feet, lit to blow in two frames (drawn once first: EnBom_Draw puts its
    // explosion's sphere on it): its explosion (DMG_EXPLOSIVE: AM_DMG_REACT_KILL) kills it:
    // EnAm_SetupRecoilFromDamage.
    let pos = am(&w, h).actor.world_pos;
    let b = w.actor_spawn(ACTOR_EN_BOM, pos + Vec3::new(0.0, 0.0, 10.0), [0, 0, 0], 0).expect("bomb");
    w.actors.downcast_mut::<EnBom>(b).unwrap().timer = 2;
    run_until(&mut w, 15, |w| w.actors.downcast::<EnAm>(h).is_some_and(|x| x.action == Action::RecoilFromDamage));
    assert_eq!(am(&w, h).actor.col_chk_info.health, 0);
    // Link 50 off it (out of the first blast's reach by now), the other Armos where they are.
    run_until(&mut w, 40, |w| am(w, h).death_timer != 0);
    let pos = am(&w, h).actor.world_pos;
    let health = w.save.health;
    w.place_player(pos + Vec3::new(50.0, 0.0, 0.0), -0x4000);
    // Its own blast at 0 (ten frames growing 10 a frame: shape.rot.z 2 + 8, to 100): Link within it
    // hurt (8), the awake neighbour killed (AM_DMG_REACT_KILL), a sleeping one woken by the hit
    // (EnAm_Sleep: AC_HIT; EnAm_UpdateDamage ignores it under AM_BEHAVIOR_5).
    run_until(&mut w, 70, |w| w.actors.get(h).is_none_or(|a| a.base().killed));
    let mut o_killed = false;
    let mut s_woke = false;
    for _ in 0..15 {
        idle(&mut w, 1);
        o_killed |= w.actors.downcast::<EnAm>(o).is_some_and(|x| x.actor.col_chk_info.health == 0);
        s_woke |= w.actors.downcast::<EnAm>(s).is_some_and(|x| x.texture_blend > 0);
    }
    assert!(w.save.health < health, "Link hurt: {} from {health}", w.save.health);
    assert!(o_killed, "the awake neighbour killed");
    assert!(s_woke, "the sleeping one woken");
}

/// `ACTOR_EN_AM` params 0 (`ARMOS_STATUE`) spawned on room 8's gravestones' floor.
fn spawn_statue(w: &mut PlayState, pos: Vec3) -> ActorHandle {
    let h = w.actor_spawn(ACTOR_EN_AM, pos, [0, 0, 0], ARMOS_STATUE).expect("the statue");
    // DynaPoly_UpdateContext builds its collision for the next frame.
    idle(w, 2);
    h
}

#[test]
fn the_statue_slides_as_link_pushes_it() {
    let Some(a) = assets() else { return };
    let mut w = room8(&a);
    let at = Vec3::new(1500.0, 16.0, -3560.0);
    let h = spawn_statue(&mut w, at);
    let x = am(&w, h);
    // EnAm_Init for ARMOS_STATUE: gArmosCol (DynaPoly_SetBgActor), ACTORCAT_BG, health 1, the
    // block cylinder OC1_NO_PUSH | OC1_TYPE_PLAYER; EnAm_SetupStatue: not targetable,
    // AM_BEHAVIOR_DO_NOTHING.
    assert_eq!((x.action, x.behavior, x.actor.category, x.actor.col_chk_info.health), (Action::Statue, AM_BEHAVIOR_DO_NOTHING, ACTORCAT_BG, 1));
    assert_eq!(x.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_ne!(x.bg, eng_collision::dyna::BG_ACTOR_MAX);
    let bg = x.bg;
    // It doesn't move by itself.
    idle(&mut w, 10);
    assert_eq!(am(&w, h).actor.world_pos, at);
    // Link walks into it from the west (facing +x) until "Grab" (gArmosCol's walls are
    // WALL_FLAG_6), holds A, and pushes.
    w.place_player(at - Vec3::new(60.0, 0.0, 0.0), 0x4000);
    idle(&mut w, 2);
    let mut prev = PadState::default();
    for _ in 0..60 {
        if w.player().state2 & STATE2_0 != 0 {
            break;
        }
        prev = frame(&mut w, prev, PadState { button: 0, stick_x: 0, stick_y: 40 });
    }
    assert_ne!(w.player().state2 & STATE2_0, 0, "no Grab at the statue (Link at {:?})", w.player().actor.world_pos);
    for _ in 0..20 {
        if w.player().linear_velocity == 0.0 {
            break;
        }
        prev = frame(&mut w, prev, PadState::default());
    }
    let mut n = 0;
    while w.player().action != PA::PushWait {
        prev = frame(&mut w, prev, PadState { button: BTN_A, ..Default::default() });
        n += 1;
        assert!(n < 10, "never held on");
    }
    let fwd = PadState { button: BTN_A, stick_x: 0, stick_y: 60 };
    // Once Player pushes (func_8084B840: 2 on dyna.unk_150 each frame, unk_158 his yaw 0x4000),
    // EnAm_Statue (ACTORCAT_BG: before Player, reading last frame's push) starts a step: unk_258
    // 0x8000, then down 0x800 a frame, speed sin(unk_258) × 2 / 2 along 0x4000, moved 1.5 times
    // that (Actor_UpdatePos); NA_SE_EV_ROCK_SLIDE each moving frame; at unk_258 0 it stops,
    // clearing Player's STATE2_0/_4/_6/_8 and his speed. A step: 1.5 × Σ sin(k × π/16), k 1..15
    // (cot(π/32) = 10.153): 15.23.
    let start = am(&w, h).actor.world_pos.x;
    let mut started = None;
    for f in 0..120 {
        prev = frame(&mut w, prev, fwd);
        if started.is_none() && am(&w, h).unk_258 != 0 {
            started = Some(f);
            assert_eq!(am(&w, h).unk_258, 0x8000u16 as i16);
            assert_eq!(w.col.dyna.unk_158(bg), 0x4000);
        }
        if started.is_some() && am(&w, h).unk_258 == 0 {
            break;
        }
    }
    assert!(started.is_some(), "the push never started (Player {:?})", w.player().action);
    let moved = am(&w, h).actor.world_pos.x - start;
    let want = 1.5 * (1..16).map(|k| eng_math::sin_s((k * 0x800) as i16)).sum::<f32>();
    assert!((moved - want).abs() < 0.05, "moved {moved}, want {want}");
    assert_eq!(am(&w, h).actor.world_pos.z, at.z);
    assert!(sfx_frames(&w, NA_SE_EV_ROCK_SLIDE - SFX_FLAG, Some(SfxPos::Actor(h))).len() >= 15);
}

#[test]
fn its_sword_hurts_link_in_the_air_of_a_lunge() {
    let Some(a) = assets() else { return };
    let mut w = room8(&a);
    let h = at_home(&w, HOMES[3]);
    // Link 45 east of it, facing it, still: its hops come at him.
    w.place_player(HOMES[3] + Vec3::new(45.0, 0.0, 0.0), -0x4000);
    idle(&mut w, 1);
    wake(&mut w, h);
    let health = w.save.health;
    // In the air of a lunge (unk_264 1, behavior AGGRO), the quad on limb 1 (EnAm_PostLimbDraw)
    // is registered (CollisionCheck_SetAT); hitting Link it hurts him (8) and, the next update
    // while it's still on, plays NA_SE_PL_BODY_HIT on him.
    let me = w.player.unwrap();
    // (The damage lands in Player's next update: the frame before it is the hit's.)
    let mut before = (am(&w, h).action, am(&w, h).unk_264);
    run_until(&mut w, 40, |w| {
        if w.save.health < health {
            return true;
        }
        before = (am(w, h).action, am(w, h).unk_264);
        false
    });
    assert_eq!(before, (Action::Lunge, 1));
    // NA_SE_PL_BODY_HIT only if the quad is still on (still in the air): the frame it hit can
    // be its last in the air, as the room's other actors move the Rand sequence its hops read.
    if am(&w, h).unk_264 == 1 {
        run_until(&mut w, 6, |w| sfx_frames(w, oot_game::audio::sfx::NA_SE_PL_BODY_HIT, Some(SfxPos::Actor(me))).len() == 1);
    } else {
        idle(&mut w, 3);
        assert!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_PL_BODY_HIT, Some(SfxPos::Actor(me))).is_empty());
    }
}
