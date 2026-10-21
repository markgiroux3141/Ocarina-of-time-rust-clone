//! GAME-08 milestone 1 against the C: the Wolfos (`z_en_wf.c`) by the Sacred Forest Meadow's
//! entrance (`spot05`, room 0, layers 0 and 1: params 0x1F00 at (-345, 0, 1658), yaw 2549), whose
//! death sets switch flag 0x1F (the gate's, `Bg_Spot05_Soko` 0x1F01).
//!
//! Child Link enters the meadow (`ENTR_SACRED_FOREST_MEADOW_0`: spawn 0 at (-201, 0, 2183), 544
//! from the Wolfos) with `kakariko-letter` (the Kokiri Sword and the Deku Shield). Expected values
//! are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_B, BTN_R, BTN_Z, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_wf::*;
use oot_actors::player::Player;
use oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED;
use oot_game::collision_check::MASS_HEAVY;
use oot_game::actor_ctx::ActorHandle;
use oot_game::collision_check::AC_HIT;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// The meadow's Wolfos's home (`ootx scene-info --scene spot05 --layer 0`).
const HOME: Vec3 = Vec3::new(-345.0, 0.0, 1658.0);

/// Child Link entered at the meadow (`ENTR_SACRED_FOREST_MEADOW_0`) at 10:00 with
/// `kakariko-letter`, the sound log on, 20 frames run.
fn meadow(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_SACRED_FOREST_MEADOW_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("kakariko-letter").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 20);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn wolfoses(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnWf>(h).is_some_and(|r| !r.actor.killed)).collect()
}

/// The meadow's Wolfos.
fn the_wolfos(w: &PlayState) -> ActorHandle {
    *wolfoses(w).iter().find(|&&h| wf(w, h).actor.home_pos == HOME).expect("the meadow's Wolfos")
}

fn wf(w: &PlayState, h: ActorHandle) -> &EnWf {
    w.actors.downcast::<EnWf>(h).expect("En_Wf")
}

fn wf_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnWf {
    w.actors.downcast_mut::<EnWf>(h).expect("En_Wf")
}

fn player(w: &PlayState) -> &Player {
    w.actors.downcast::<Player>(w.player.unwrap()).unwrap()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

fn anim_name(r: &EnWf) -> &str {
    r.skel.animation.as_ref().map(|a| a.name.as_str()).unwrap_or("")
}

/// Link `d` from the Wolfos's home towards the meadow's entrance (+z, on the path's floor),
/// facing it.
fn link_at(w: &mut PlayState, d: f32) {
    w.place_player(HOME + Vec3::new(0.0, 0.0, d), -0x8000);
}

/// Link 239 away: the Wolfos rises (5 frames), howls and stands; returns it idle.
fn risen(w: &mut PlayState) -> ActorHandle {
    let h = the_wolfos(w);
    link_at(w, 239.0);
    for _ in 0..60 {
        idle(w, 1);
        if wf(w, h).func != Func::WaitAppear {
            return h;
        }
    }
    panic!("it never rose");
}

#[test]
fn the_wolfos_initialises_buried_by_its_params() {
    let Some(a) = assets() else { return };
    let w = meadow(&a);
    let h = the_wolfos(&w);
    let r = wf(&w, h);
    // 0x1F00: switchFlag PARAMS_GET_U(params, 8, 8) = 0x1F, the type PARAMS_GET_U(params, 0, 8)
    // = 0 written over the params: gWolfosGraySkel, scale 0.0075, NAVI_ENEMY_WOLFOS.
    assert_eq!((r.actor.params, r.switch_flag), (0, 0x1F));
    assert_eq!((r.actor.scale.x, r.actor.scale.z, r.actor.navi_enemy_id), (0.0075, 0.0075, 0x4C));
    // Health 8, MASS_HEAVY, colChkInfo's cylinder 50 by 100; sInitChain: lockOnArrowOffset 2000.
    let c = &r.actor.col_chk_info;
    assert_eq!((c.health, c.mass, c.cyl_radius, c.cyl_height, r.actor.target_arrow_offset), (8, MASS_HEAVY, 50, 100, 2000.0));
    // EnWf_SetupWaitAppear: gWolfosHowlAnim held (0.5, to 7), 5 under its home, flat (scale.y 0),
    // no gravity, not targetable, timer 20; unk_2F4 10. Link is 544 away: it's still buried.
    assert_eq!((r.func, r.action, r.timer), (Func::WaitAppear, ENWF_ACTION_WAIT_APPEAR, 20));
    assert_eq!(anim_name(r), "gWolfosHowlAnim");
    assert_eq!((r.skel.play_speed, r.skel.end_frame), (0.5, 7.0));
    assert_eq!((r.actor.world_pos.y, r.actor.scale.y, r.actor.gravity), (HOME.y - 5.0, 0.0, 0.0));
    assert_eq!(r.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(r.unk_2f4, 10.0);
    // The colliders: the claws' spheres 15 (4 damage), the head's 25 (800 along it), the
    // torso's 30; the body cylinder 20 by 50, the tail's 15 by 20 shifted 15 down.
    let s = &r.attack_and_block_collider;
    let radii: Vec<i16> = s.elements.iter().map(|e| e.dim.model_sphere.radius).collect();
    assert_eq!(radii, [15, 15, 25, 30]);
    let limbs: Vec<u8> = s.elements.iter().map(|e| e.dim.limb).collect();
    assert_eq!(limbs, [15, 21, 17, 12]);
    assert_eq!((s.elements[0].info.at_dmg_info.damage, s.elements[1].info.at_dmg_info.damage), (4, 4));
    assert_eq!(s.elements[2].dim.model_sphere.center, [800, 0, 0]);
    assert_eq!((r.body_collider.dim.radius, r.body_collider.dim.height, r.body_collider.dim.y_shift), (20, 50, 0));
    assert_eq!((r.tail_collider.dim.radius, r.tail_collider.dim.height, r.tail_collider.dim.y_shift), (15, 20, -15));
}

#[test]
fn it_rises_when_link_is_within_240_across() {
    let Some(a) = assets() else { return };
    let mut w = meadow(&a);
    let h = the_wolfos(&w);
    // EnWf_WaitAppear: xzDistToPlayer < 240 only. At 241 (and 100 up: the height isn't read) it
    // stays buried.
    w.place_player(HOME + Vec3::new(0.0, 0.0, 241.0), -0x8000);
    idle(&mut w, 30);
    let r = wf(&w, h);
    assert_eq!((r.func, r.timer, r.actor.scale.y), (Func::WaitAppear, 20, 0.0));
    assert!(sfx_frames(&w, NA_SE_EN_WOLFOS_APPEAR).is_empty());
    // At 239: that frame timer 5 and targetable.
    link_at(&mut w, 239.0);
    idle(&mut w, 1);
    let r = wf(&w, h);
    assert!((r.actor.xz_dist_to_player - 239.0).abs() < 1.0, "{}", r.actor.xz_dist_to_player);
    assert_eq!((r.func, r.timer), (Func::WaitAppear, 5));
    assert_ne!(r.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    // Five frames rising: scale.y up a fifth of scale.x each (to 0.0075), 0.5 up each, the
    // shadow to 70 by 14; at the fifth (timer 0) NA_SE_EN_WOLFOS_APPEAR.
    for i in 1..=5 {
        idle(&mut w, 1);
        let r = wf(&w, h);
        assert_eq!(r.timer, 5 - i);
        assert!((r.actor.scale.y - 0.0015 * i as f32).abs() < 1e-6, "{i}: {}", r.actor.scale.y);
        assert!((r.actor.shape_shadow_scale - 14.0 * i as f32).abs() < 1e-3);
    }
    assert_eq!(sfx_frames(&w, NA_SE_EN_WOLFOS_APPEAR), [w.audio.frames]);
    // Then the howl to frame 7 at half speed; its end: scale.y = scale.x, gravity -2,
    // EnWf_SetupIdle (gWolfosIdleAnim looped, timer 2 to 11, speed 0).
    let mut n = 0;
    while wf(&w, h).func == Func::WaitAppear {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 30, "the howl never ended");
    }
    let r = wf(&w, h);
    assert_eq!((r.func, r.action, r.actor.scale.y, r.actor.gravity), (Func::Idle, ENWF_ACTION_IDLE, 0.0075, -2.0));
    // 7 frames at 0.5 a frame.
    assert!((13..=16).contains(&n), "{n}");
}

#[test]
fn its_switch_flag_set_kills_it_at_init() {
    let Some(a) = assets() else { return };
    let mut w = meadow(&a);
    // EnWf_Init: switchFlag != 0xFF and Flags_GetSwitch: Actor_Kill. 0xFF never.
    w.flags.set_switch(0x1F);
    let pos = HOME + Vec3::new(100.0, 0.0, 0.0);
    let set = w.actor_spawn(ACTOR_EN_WF, pos, [0, 0, 0], 0x1F00).expect("spawned");
    let unset = w.actor_spawn(ACTOR_EN_WF, pos, [0, 0, 0], 0x1E00).expect("spawned");
    let none = w.actor_spawn(ACTOR_EN_WF, pos, [0, 0, 0], 0xFF00u16 as i16).expect("spawned");
    assert!(wf(&w, set).actor.killed);
    assert!(!wf(&w, unset).actor.killed);
    assert!(!wf(&w, none).actor.killed);
    assert_eq!((wf(&w, unset).switch_flag, wf(&w, none).switch_flag), (0x1E, 0xFF));
    idle(&mut w, 2);
    assert!(w.actors.get(set).is_none());
    assert!(w.actors.get(unset).is_some() && w.actors.get(none).is_some());
}

#[test]
fn the_white_wolfos_is_bigger_and_harder_and_plays_the_mini_boss_music() {
    let Some(a) = assets() else { return };
    let mut w = meadow(&a);
    // Type 1 with switch flag 0x1E: gWolfosWhiteSkel, scale 0.01, its claws 8, NAVI_ENEMY_WHITE_WOLFOS.
    let pos = HOME + Vec3::new(0.0, 0.0, 200.0);
    let h = w.actor_spawn(ACTOR_EN_WF, pos, [0, 0, 0], 0x1E01).expect("spawned");
    let r = wf(&w, h);
    assert_eq!((r.actor.params, r.switch_flag, r.actor.scale.x, r.actor.navi_enemy_id), (1, 0x1E, 0.01, 0x57));
    let s = &r.attack_and_block_collider;
    assert_eq!((s.elements[0].info.at_dmg_info.damage, s.elements[1].info.at_dmg_info.damage), (8, 8));
    // Link within 240 of it (and over 240 from the gray one): it rises and starts
    // NA_BGM_MINI_BOSS (func_800F5ACC: SEQ_PLAYER_BGM_MAIN's start).
    w.place_player(pos + Vec3::new(0.0, 0.0, 100.0), -0x8000);
    idle(&mut w, 2);
    assert_eq!(wf(&w, h).timer, 4);
    let cmds = &w.audio.log.as_ref().unwrap().seq_cmds;
    assert!(cmds.iter().any(|&(_, c)| c == oot_game::audio::bgm::start_seq(0, 0, NA_BGM_MINI_BOSS)));
}

/// `sDamageTable` as the C lists it: (damage, reaction) per `DMG_*` bit.
const DAMAGE_TABLE: [(u8, u8); 32] = [
    (0, 0x1), // Deku nut: stunned
    (2, 0x0), // Deku stick
    (1, 0x0), // Slingshot
    (2, 0x0), // Explosive
    (0, 0x1), // Boomerang: stunned
    (2, 0x0), // Normal arrow
    (2, 0x0), // Hammer swing
    (0, 0x1), // Hookshot: stunned
    (1, 0x0), // Kokiri sword
    (2, 0x0), // Master sword
    (4, 0x0), // Giant's Knife
    (4, 0xE), // Fire arrow: burns
    (2, 0x0), // Ice arrow
    (2, 0x0), // Light arrow
    (2, 0x0), // Unk arrow 1
    (2, 0x0), // Unk arrow 2
    (2, 0x0), // Unk arrow 3
    (4, 0xE), // Fire magic: burns
    (0, 0x6), // Ice magic
    (3, 0xD), // Light magic
    (0, 0x0), // Shield
    (0, 0x0), // Mirror Ray
    (1, 0x0), // Kokiri spin
    (4, 0x0), // Giant spin
    (2, 0x0), // Master spin
    (2, 0x0), // Kokiri jump
    (8, 0x0), // Giant jump
    (4, 0x0), // Master jump
    (0, 0x0), // Unknown 1
    (0, 0x0), // Unblockable
    (4, 0x0), // Hammer jump
    (0, 0x0), // Unknown 2
];

#[test]
fn the_damage_table_is_the_cs() {
    for (i, &(damage, reaction)) in DAMAGE_TABLE.iter().enumerate() {
        // DMG_ENTRY(damage, effect): (effect << 4) | damage.
        assert_eq!(S_DAMAGE_TABLE.table[i], (reaction << 4) | damage, "entry {i}");
    }
}

/// A hit as `CollisionCheck_AC` leaves it for the next update: the collider's `AC_HIT`, the
/// table's damage and reaction in `colChkInfo`.
fn hit(w: &mut PlayState, h: ActorHandle, body: bool, tail: bool, damage: u8) {
    let r = wf_mut(w, h);
    if body {
        r.body_collider.base.ac_flags |= AC_HIT;
    }
    if tail {
        r.tail_collider.base.ac_flags |= AC_HIT;
    }
    r.actor.col_chk_info.damage = damage;
    r.actor.col_chk_info.damage_reaction = 0;
}

#[test]
fn a_hit_on_its_tail_alone_or_from_behind_does_four_times_the_damage() {
    let Some(a) = assets() else { return };
    let mut w = meadow(&a);
    let h = risen(&mut w);
    // EnWf_CheckCollide: the Kokiri Sword's 1 on the body, Link in front: 1 (8 to 7), red 8
    // frames, EnWf_SetupDamaged (NA_SE_EN_WOLFOS_DAMAGE, knocked back 4).
    wf_mut(&mut w, h).actor.shape_rot.y = 0; // facing Link (+z: yaw 0)
    hit(&mut w, h, true, false, 1);
    idle(&mut w, 1);
    let r = wf(&w, h);
    assert_eq!((r.actor.col_chk_info.health, r.func, r.action), (7, Func::Damaged, ENWF_ACTION_DAMAGED));
    assert_eq!(sfx_frames(&w, NA_SE_EN_WOLFOS_DAMAGE), [w.audio.frames]);
    // The body and the tail both: still 1 (`!(body || !tail)` is false), 7 to 6.
    idle(&mut w, 20);
    wf_mut(&mut w, h).actor.shape_rot.y = 0;
    hit(&mut w, h, true, true, 1);
    idle(&mut w, 1);
    assert_eq!(wf(&w, h).actor.col_chk_info.health, 6);
    // The tail alone: 4 (6 to 2).
    idle(&mut w, 20);
    wf_mut(&mut w, h).actor.shape_rot.y = 0;
    hit(&mut w, h, false, true, 1);
    idle(&mut w, 1);
    assert_eq!(wf(&w, h).actor.col_chk_info.health, 2);
}

#[test]
fn a_hit_from_behind_does_four_times_the_damage() {
    let Some(a) = assets() else { return };
    let mut w = meadow(&a);
    let h = risen(&mut w);
    // Link at +z (yaw 0 from it), the Wolfos facing away (-0x8000: Link straight behind, 0x8000
    // off its facing, over 0x4A38): the body's 1 is 4 (8 to 4).
    wf_mut(&mut w, h).actor.shape_rot.y = -0x8000;
    hit(&mut w, h, true, false, 1);
    idle(&mut w, 1);
    assert_eq!(wf(&w, h).actor.col_chk_info.health, 4);
}

#[test]
fn its_death_sets_switch_flag_0x1f() {
    let Some(a) = assets() else { return };
    let mut w = meadow(&a);
    let h = risen(&mut w);
    assert!(!w.flags.get_switch(0x1F));
    // A hit to 0: EnWf_SetupDie (the howl, not targetable, NA_SE_EN_WOLFOS_DEAD) and
    // Enemy_StartFinishingBlow (freezeFlashTimer 5).
    wf_mut(&mut w, h).actor.shape_rot.y = 0;
    wf_mut(&mut w, h).actor.col_chk_info.health = 1;
    hit(&mut w, h, true, false, 1);
    idle(&mut w, 1);
    let r = wf(&w, h);
    assert_eq!((r.actor.col_chk_info.health, r.func, r.action), (0, Func::Die, ENWF_ACTION_DIE));
    assert_eq!(anim_name(r), "gWolfosHowlAnim");
    assert_eq!(r.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(sfx_frames(&w, NA_SE_EN_WOLFOS_DEAD), [w.audio.frames]);
    assert!(w.actors.freeze_flash_timer > 0);
    // EnWf_Die: through the howl, blue flames (EffectSsDeadDb) rising; the flag only at its end,
    // then gone.
    let mut n = 0;
    while w.actors.get(h).is_some_and(|a| !a.base().killed) {
        assert!(!w.flags.get_switch(0x1F));
        idle(&mut w, 1);
        n += 1;
        assert!(n < 200, "it never died");
    }
    assert!(w.flags.get_switch(0x1F));
}

/// What the fight's script did.
#[derive(Debug, Default)]
struct Fight {
    frames: u32,
    death_blow: Option<u32>,
    b_presses: u32,
    z_presses: u32,
    shield_frames: u32,
    walk_frames: u32,
    hits_taken: u32,
    hits_given: u32,
}

/// Child Link (the Kokiri Sword, the Deku Shield, four hearts) fights the meadow's Wolfos as a
/// player would, reading only what's on screen: he walks at it from the entrance; once it's up
/// he turns to it and Z-targets it, keeps within 75 of it, holds the shield (R) while it runs,
/// circles, blocks or slashes, and slashes (B, a press every other frame) when it can't block:
/// recoiling off his shield (`EnWf_AttackBouncedOff`), spinning after its slash (frame 20 on),
/// hurt or stunned.
fn fight(w: &mut PlayState, h: ActorHandle, max_frames: u32) -> Fight {
    let mut out = Fight::default();
    let mut prev = PadState::default();
    for f in 0..max_frames {
        let Some(r) = w.actors.downcast::<EnWf>(h) else { break };
        if r.actor.killed {
            break;
        }
        let pos = r.actor.world_pos;
        let dist = r.actor.xz_dist_to_player;
        let open = match r.func {
            Func::AttackBouncedOff | Func::Damaged | Func::Stunned => true,
            Func::Attack => r.skel.cur_frame >= 20.0,
            _ => false,
        };
        let locked = player(w).focus_actor == Some(h);
        let mut pad = PadState::default();
        if r.func == Func::WaitAppear {
            pad = oot_actors::script::stick_towards(w, pos, 80.0);
            out.walk_frames += 1;
        } else if !locked {
            // Turn to it (and walk at it), then Z (a new press) once it's in front.
            let p = player(w);
            let off = eng_math::vec3f_yaw(p.actor.world_pos, pos).wrapping_sub(p.actor.shape_rot.y);
            if (off as i32).abs() < 0x2000 && prev.button & BTN_Z == 0 {
                pad.button |= BTN_Z;
                out.z_presses += 1;
            } else {
                pad = oot_actors::script::stick_towards(w, pos, if dist > 75.0 { 60.0 } else { 20.0 });
                out.walk_frames += 1;
            }
        } else {
            pad.button |= BTN_Z;
            if dist > 75.0 {
                let s = oot_actors::script::stick_towards(w, pos, 60.0);
                (pad.stick_x, pad.stick_y) = (s.stick_x, s.stick_y);
                out.walk_frames += 1;
            }
            if open && dist < 90.0 {
                if prev.button & BTN_B == 0 {
                    pad.button |= BTN_B;
                    out.b_presses += 1;
                }
            } else {
                pad.button |= BTN_R;
                out.shield_frames += 1;
            }
        }
        let (link_hp, wf_hp) = (w.save.health, r.actor.col_chk_info.health);
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        out.frames = f + 1;
        let r = wf(w, h);
        if w.save.health < link_hp {
            out.hits_taken += 1;
        }
        if r.actor.col_chk_info.health < wf_hp {
            out.hits_given += 1;
        }
        if std::env::var("WF_DEBUG").is_ok() && (w.save.health != link_hp || r.actor.col_chk_info.health != wf_hp) {
            eprintln!("{f}: Link {link_hp:#x} to {:#x}, the Wolfos {wf_hp} to {} ({:?}, {:.0} away)", w.save.health, r.actor.col_chk_info.health, r.func, r.actor.xz_dist_to_player);
        }
        if out.death_blow.is_none() && r.func == Func::Die {
            out.death_blow = Some(out.frames);
        }
    }
    out
}

#[test]
fn a_child_with_the_kokiri_sword_kills_the_meadows_wolfos() {
    let Some(a) = assets() else { return };
    let mut w = meadow(&a);
    let h = the_wolfos(&w);
    // Four hearts: the castle's presets on have Queen Gohma's heart container (GAME-10 milestone 1).
    assert_eq!(w.save.health, 0x40, "four hearts");
    let f = fight(&mut w, h, 6000);
    println!("{f:?}, Link's health left {:#x}", w.save.health);
    let blow = f.death_blow.expect("the Wolfos never died");
    assert!(w.save.health > 0, "Link died");
    // Once dying: the howl's end (EnWf_Die), its switch flag 0x1F and gone.
    assert!(w.actors.get(h).is_none() || wf(&w, h).actor.killed, "dead");
    assert!(f.frames > blow);
    assert!(w.flags.get_switch(0x1F));
}
