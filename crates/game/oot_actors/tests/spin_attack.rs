//! The spin attack against the C (`z_player.c`, `z_en_m_thunder.c`, `z_eff_dust.c`): Player's
//! charge (B held as a slash's `unk_844` counts down to 1, `Player_ActionHandler_8`), the charge
//! actions (`Player_Action_80844E68`, `_80845000`, `_80845308`), the release (`func_80844BE4`),
//! the quick spin (`Player_ActionHandler_7` with the stick spun), and their effect `En_M_Thunder`
//! with and without magic, and its sparkles `Eff_Dust`.
//!
//! Child Link with the Kokiri Sword in Kokiri Forest's village (`sword-and-40-rupees`: no magic,
//! a new save's), with the meter given where a test says so; adult Link with the Master Sword
//! in Hyrule Field on the debug save (magic acquired, the meter grown and filled first).
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_B, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::eff_dust::{self, ACTOR_EFF_DUST, EffDust};
use oot_actors::en_m_thunder::{self, EnMThunder, M_THUNDER_ATTACK_STRONG, M_THUNDER_ATTACK_WEAK, NA_SE_IT_ROLLING_CUT, NA_SE_IT_ROLLING_CUT_LV1, NA_SE_IT_ROLLING_CUT_LV2};
use oot_actors::player::{Action, STATE1_12, STATE2_17};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ACTORCAT_NPC, ActorHandle};
use oot_game::audio::sfx::NA_SE_IT_SWORD_SWING_HARD;
use oot_game::collision_check as cc;
use oot_game::magic::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::{MAGIC_NORMAL_METER, SaveContext};

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn b(p: PadState) -> PadState {
    with(p, BTN_B)
}

/// The pad, frame by frame, its last state kept (a press is a button down now and not before).
struct Pad {
    prev: PadState,
}

impl Pad {
    fn new() -> Pad {
        Pad { prev: PadState::default() }
    }
    fn step(&mut self, w: &mut PlayState, cur: PadState) {
        w.tick_with(scripted_input(self.prev, cur));
        self.prev = cur;
    }
    fn steps(&mut self, w: &mut PlayState, cur: PadState, n: usize) {
        for _ in 0..n {
            self.step(w, cur);
        }
    }
}

fn boot(a: &Arc<GameAssets>, save: SaveContext) -> PlayState {
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init")
}

/// Child Link in Kokiri Forest's village with the Kokiri Sword, settled; with `magic` the meter
/// as a file with it has it (`isMagicAcquired`, level 1, 0x30 of 0x30, idle).
fn child(a: &Arc<GameAssets>, magic: bool) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_3").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("sword-and-40-rupees").unwrap();
    let mut w = boot(a, save);
    if magic {
        let s = &mut w.save;
        s.is_magic_acquired = true;
        s.magic_level = 1;
        s.magic_capacity = MAGIC_NORMAL_METER as i16;
        s.magic = MAGIC_NORMAL_METER;
        s.magic_state = MAGIC_STATE_IDLE;
    }
    Pad::new().steps(&mut w, stick(0, 0), 30);
    assert_eq!(w.player().action, Action::StandingStill);
    w
}

/// Adult Link in Hyrule Field on the debug save (its own entrance; the Master Sword on B, magic
/// acquired: the meter grows and fills over the first frames), the meter idle. (Not in a
/// house: B is disabled indoors.)
fn adult(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_HYRULE_FIELD_0").expect("entrance");
    let save = SaveContext::debug(e, true, oot_game::env::clock_time(10, 0) as u16);
    let mut w = boot(a, save);
    Pad::new().steps(&mut w, stick(0, 0), 40);
    assert_eq!((w.save.magic_state, w.save.magic, w.save.magic_capacity), (MAGIC_STATE_IDLE, 0x30, 0x30));
    assert_eq!(w.player().action, Action::StandingStill);
    w
}

/// B: the sword out (and its slash), then standing again.
fn draw_sword(w: &mut PlayState) {
    let mut pad = Pad::new();
    pad.step(w, b(stick(0, 0)));
    pad.steps(w, stick(0, 0), 40);
    assert_eq!(w.player().action, Action::StandingStill);
}

fn thunder(w: &PlayState) -> Option<(ActorHandle, &EnMThunder)> {
    let h = en_m_thunder::find(w)?;
    Some((h, w.actors.downcast::<EnMThunder>(h)?))
}

fn dust(w: &PlayState) -> Option<(ActorHandle, &EffDust)> {
    let h = w.actors.find(ACTOR_EFF_DUST, ACTORCAT_NPC)?;
    Some((h, w.actors.downcast::<EffDust>(h)?))
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// Holds B from standing with the sword out until the charge starts: the slash (`func_80837948`:
/// `unk_844` 8), counted down each frame by `Player_UpdateCommon` while B stays held; on the
/// frame it's 1 the end animation's interrupt list (`Player_ActionHandler_8`) starts the charge.
/// Returns the frames held after the slash's, the pad (B down).
fn start_charge(w: &mut PlayState) -> (usize, Pad) {
    let mut pad = Pad::new();
    for n in 0..20 {
        pad.step(w, b(stick(0, 0)));
        if n == 0 {
            assert_eq!((w.player().action, w.player().unk_844), (Action::Attack, 8));
        }
        if w.player().action == Action::ChargeSpin {
            return (n, pad);
        }
    }
    panic!("no charge");
}

/// Holds B until Player's charge (`unk_858`) is at least `at`; returns the frames.
fn charge_to(w: &mut PlayState, pad: &mut Pad, at: f32) -> usize {
    for n in 1..200 {
        pad.step(w, b(stick(0, 0)));
        if w.player().unk_858 >= at {
            return n;
        }
    }
    panic!("no charge to {at}");
}

#[test]
fn b_held_after_a_slash_charges_and_spawns_the_effect() {
    let Some(a) = assets() else { return };
    let mut w = child(&a, false);
    let d = w.data.clone();
    draw_sword(&mut w);
    let (n, mut pad) = start_charge(&mut w);
    // unk_844 from 8 on the slash's frame to 1 seven frames later (B held: no func_8083C50C).
    assert_eq!(n, 7);
    let p = w.player();
    assert_eq!(p.unk_844, 1);
    // func_808377DC: Player_Action_80844E68; func_80837704: after PLAYER_MWA_RIGHT_SLASH_1H (stick
    // neutral, not targeting) D_80854358[0], from frame 8; func_80837530(play, this, 0x200): the
    // charge 0, PLAYER_STATE1_CHARGING_SPIN_ATTACK.
    assert_eq!(p.melee_weapon_animation, d.items.mwa("RIGHT_SLASH_1H"));
    assert_eq!(d.anim_name(p.skel.animation), "link_fighter_power_kiru_startL");
    assert_eq!(p.skel.cur_frame, 8.0);
    assert_eq!(p.unk_858, 0.0);
    assert!(p.state1 & STATE1_12 != 0);
    // En_M_Thunder: Player_GetMeleeWeaponHeld (PLAYER_MELEEWEAPON_SWORD_KOKIRI, 2) | 0x200; its
    // init without PLAYER_STATE2_17 charges (swordType 1, scale 0.1, room -1, the cylinder 0 wide,
    // 40 tall, 20 down, following for 8 frames), facing away from Link. It doesn't update the
    // frame it spawns (its flags are 0 and it hasn't been drawn in the culling volume yet).
    let (_, t) = thunder(&w).expect("En_M_Thunder");
    assert_eq!(t.actor.params, 0x202);
    assert_eq!(t.actor.category, ACTORCAT_ITEMACTION);
    assert_eq!((t.action, t.sword_type, t.actor.room, t.actor.scale), (en_m_thunder::Action::ChargingSpinAttack, 1, -1, Vec3::splat(0.1)));
    assert_eq!((t.collider.dim.radius, t.collider.dim.height, t.collider.dim.y_shift, t.follow_player_timer), (0, 40, -20, 8));
    assert!(!t.is_using_magic);
    assert_eq!(t.actor.shape_rot.y, p.actor.shape_rot.y.wrapping_add(i16::MIN));
    // No charge until the start animation is over (av2.actionVar2 0), then 0.02 a frame
    // (func_80844E3C) on the charge loop (D_80854360[0]).
    let mut charges = Vec::new();
    for _ in 0..12 {
        pad.step(&mut w, b(stick(0, 0)));
        charges.push(w.player().unk_858);
    }
    let first = charges.iter().position(|&c| c > 0.0).expect("charging");
    assert!(first >= 1, "{charges:?}");
    for (k, c) in charges[first..].iter().enumerate() {
        assert!((c - 0.02 * (k + 1) as f32).abs() < 1e-5, "{charges:?}");
    }
    assert_eq!(d.anim_name(w.player().skel.animation), "link_fighter_power_kiru_wait");
    assert_eq!(w.player().action, Action::ChargeSpin);
}

#[test]
fn without_magic_the_charge_is_empty_and_the_spin_is_plain() {
    let Some(a) = assets() else { return };
    let mut w = child(&a, false);
    let d = w.data.clone();
    draw_sword(&mut w);
    let (_, mut pad) = start_charge(&mut w);
    // EnMThunder_ChargingSpinAttack at 0.1: Magic_RequestChange(2, MAGIC_CONSUME_WAIT_PREVIEW)
    // fails (no magic): EnMThunder_EmptySpinAttack, nothing drawn, no darkening.
    charge_to(&mut w, &mut pad, 0.1);
    let (h, t) = thunder(&w).expect("En_M_Thunder");
    assert_eq!((t.action, t.charge_alpha, t.dimming_intensity, t.spin_attack_timer), (en_m_thunder::Action::EmptySpinAttack, 0, 0.0, 0.0));
    assert!(!t.is_using_magic);
    assert_eq!(w.save.magic_state, MAGIC_STATE_IDLE);
    // No sparkles without the charge's magic.
    charge_to(&mut w, &mut pad, 0.5);
    assert!(dust(&w).is_none());
    // Let go below 0.85: func_80844BE4's PLAYER_MWA_SPIN_ATTACK_1H (D_80854380[0], Player_CanSpinAttack
    // false), PLAYER_STATE2_17; the empty effect sees it and goes with the plain spin's sounds.
    pad.step(&mut w, stick(0, 0));
    let p = w.player();
    assert_eq!((p.action, p.melee_weapon_animation), (Action::Attack, d.items.mwa("SPIN_ATTACK_1H")));
    assert_eq!(d.anim_name(p.skel.animation), d.anim_name(d.items.attacks[d.items.mwa("SPIN_ATTACK_1H")].anim));
    assert!(p.state2 & STATE2_17 != 0);
    // Player_SetInvulnerability(this, -8).
    assert_eq!(p.invincibility_timer, -8);
    let now = w.audio.frames;
    assert_eq!(sfx_frames(&w, NA_SE_IT_ROLLING_CUT), vec![now]);
    assert_eq!(sfx_frames(&w, NA_SE_IT_SWORD_SWING_HARD), vec![now]);
    assert!(w.actors.actor(h).is_some_and(|a| a.killed));
    pad.step(&mut w, stick(0, 0));
    assert!(!w.actors.exists(h));
    // Player's own sword quads do the spin's hits (PLAYER_STATE2_17 still on).
    assert!(w.player().state2 & STATE2_17 != 0);
}

#[test]
fn the_charge_given_up_below_0_1() {
    let Some(a) = assets() else { return };
    let mut w = child(&a, false);
    let d = w.data.clone();
    draw_sword(&mut w);
    let (_, mut pad) = start_charge(&mut w);
    // B let go during the start animation (av2.actionVar2 0): nothing; on the charge loop's
    // first frame (av2 -1, the charge 0.02): func_80844D68, standing, D_80854368[0].
    let mut n = 0;
    while w.player().unk_858 == 0.0 {
        pad.step(&mut w, stick(0, 0));
        n += 1;
        assert!(n < 20);
    }
    let p = w.player();
    assert!((p.unk_858 - 0.02).abs() < 1e-6);
    assert_eq!(p.action, Action::StandingStill);
    assert_eq!(d.anim_name(p.skel.animation), "link_fighter_power_kiru_wait_end");
    // The effect, charging, sees PLAYER_STATE1_CHARGING_SPIN_ATTACK off next frame and goes.
    let (h, _) = thunder(&w).expect("En_M_Thunder");
    pad.step(&mut w, stick(0, 0));
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
    assert!(w.player().state1 & STATE1_12 == 0);
}

#[test]
fn with_magic_the_charge_shows_spends_and_spins() {
    let Some(a) = assets() else { return };
    let mut w = child(&a, true);
    draw_sword(&mut w);
    let (_, mut pad) = start_charge(&mut w);
    // At 0.1: Magic_RequestChange(2, MAGIC_CONSUME_WAIT_PREVIEW): the target 0x30 - 2, the meter
    // flashing with the preview (MAGIC_STATE_METER_FLASH_2); isUsingMagic.
    charge_to(&mut w, &mut pad, 0.1);
    let (h, t) = thunder(&w).expect("En_M_Thunder");
    assert_eq!(t.action, en_m_thunder::Action::ChargingSpinAttack);
    assert!(t.is_using_magic);
    assert_eq!((w.save.magic_state, w.save.magic_target, w.save.magic), (MAGIC_STATE_METER_FLASH_2, 0x2E, 0x30));
    // Above 0.1 to 0.15: chargeAlpha (charge - 0.1) * 255 * 20, the timer (charge - 0.1) * 10.
    pad.step(&mut w, b(stick(0, 0)));
    let c = w.player().unk_858;
    let t = w.actors.downcast::<EnMThunder>(h).unwrap();
    assert!(c > 0.1 && c <= 0.15, "{c}");
    assert_eq!(t.charge_alpha, ((c - 0.1) * 255.0 * 20.0) as i32 as u8);
    assert!((t.spin_attack_timer - (c - 0.1) * 10.0).abs() < 1e-6);
    // Above 0.15: drawn whole, its sparkles (Eff_Dust, its child, swordType + 2 = 3), the scene
    // darkening by half the way to (charge - 0.15) * 1.5 a frame.
    charge_to(&mut w, &mut pad, 0.16);
    let t = w.actors.downcast::<EnMThunder>(h).unwrap();
    assert_eq!(t.charge_alpha, 255);
    let (dh, dst) = dust(&w).expect("Eff_Dust");
    assert_eq!((dst.actor.params, dst.actor.parent, t.actor.child), (3, Some(h), Some(dh)));
    assert_eq!((dst.dx, dst.scaling_factor, dst.life), (0.5, 10.0, 10));
    let before = t.dimming_intensity;
    pad.step(&mut w, b(stick(0, 0)));
    let c = w.player().unk_858;
    let t = w.actors.downcast::<EnMThunder>(h).unwrap();
    assert!((t.dimming_intensity - (before + ((c - 0.15) * 1.5 - before) * 0.5)).abs() < 1e-6);
    assert!((t.spin_charge_percent - c).abs() < 1e-6);
    // Only one child: the sparkles follow it (their position the effect's).
    assert_eq!(w.actors.category(ACTORCAT_NPC).iter().filter(|&&x| w.actors.actor(x).is_some_and(|a| a.id == ACTOR_EFF_DUST)).count(), 1);
    charge_to(&mut w, &mut pad, 0.5);
    // (They update among the NPCs, before the item actions: where it was the frame before.)
    let was = w.actors.downcast::<EnMThunder>(h).unwrap().actor.world_pos;
    pad.step(&mut w, b(stick(0, 0)));
    assert_eq!(dust(&w).unwrap().1.actor.world_pos, was);
    // Let go at 0.5: the spin (PLAYER_STATE2_17); the effect turns it off, the meter consumes
    // (MAGIC_STATE_CONSUME_SETUP, and Magic_Update the same frame: MAGIC_STATE_CONSUME), weak:
    // DMG_SPIN_KOKIRI, scale to 2, the timer 1, NA_SE_IT_ROLLING_CUT_LV1 (sSfxIds[1]); the
    // sparkles let go of (parent NULL).
    pad.step(&mut w, stick(0, 0));
    let now = w.audio.frames;
    let t = w.actors.downcast::<EnMThunder>(h).unwrap();
    assert_eq!(t.action, en_m_thunder::Action::SpinAttacking);
    assert_eq!((t.attack_strength, t.target_scale, t.spin_attack_timer, t.follow_player_timer), (M_THUNDER_ATTACK_WEAK, 2, 1.0, 8));
    assert_eq!(t.collider.info.at_dmg_info.dmg_flags, cc::DMG_SPIN_KOKIRI);
    assert!(w.player().state2 & STATE2_17 == 0);
    assert_eq!(w.save.magic_state, MAGIC_STATE_CONSUME);
    assert_eq!(sfx_frames(&w, NA_SE_IT_ROLLING_CUT_LV1), vec![now]);
    assert!(sfx_frames(&w, NA_SE_IT_ROLLING_CUT).is_empty());
    assert_eq!(w.actors.actor(dh).unwrap().parent, None);
    // The spin: Math_SmoothStepToF(scale, 2, 0.6, 0.8, 0): 0.1 + 0.8 (the step's cap) = 0.9, then
    // + 0.66 = 1.56, + 0.264 = 1.824; the radius scale * 25; the timer down by 1/16.
    let mut scales = Vec::new();
    for _ in 0..3 {
        pad.step(&mut w, stick(0, 0));
        let t = w.actors.downcast::<EnMThunder>(h).unwrap();
        scales.push((t.actor.scale.x, t.collider.dim.radius, t.spin_attack_timer));
    }
    let want = [(0.9, 22, 15.0 / 16.0), (1.56, 39, 14.0 / 16.0), (1.824, 45, 13.0 / 16.0)];
    for (got, want) in scales.iter().zip(want) {
        assert!((got.0 - want.0).abs() < 1e-5 && got.1 == want.1 && (got.2 - want.2).abs() < 1e-6, "{scales:?}");
    }
    // The meter: 0x30 down by 2 to the target, flashing (MAGIC_STATE_METER_FLASH_1).
    assert_eq!((w.save.magic, w.save.magic_state), (0x2E, MAGIC_STATE_METER_FLASH_1));
    // The timer at 0 on its 16th spinning frame: killed; its destroy (next frame) Magic_Reset:
    // idle again after the meter's update.
    pad.steps(&mut w, stick(0, 0), 12);
    assert!(w.actors.actor(h).is_some_and(|a| !a.killed));
    pad.step(&mut w, stick(0, 0));
    assert!(w.actors.actor(h).is_some_and(|a| a.killed));
    pad.step(&mut w, stick(0, 0));
    assert!(!w.actors.exists(h));
    assert_eq!((w.save.magic, w.save.magic_state), (0x2E, MAGIC_STATE_IDLE));
}

#[test]
fn the_sparkles_go_ten_frames_after_their_parent() {
    let Some(a) = assets() else { return };
    let mut w = child(&a, true);
    draw_sword(&mut w);
    let (_, mut pad) = start_charge(&mut w);
    charge_to(&mut w, &mut pad, 0.3);
    let (dh, _) = dust(&w).expect("Eff_Dust");
    // Let go: its parent NULL (the effect's release); EffDust_UpdateFunc_8099DFC0 counts its life
    // (10) down, then is killed on the 11th frame without one.
    pad.step(&mut w, stick(0, 0));
    for k in 0..11 {
        pad.step(&mut w, stick(0, 0));
        let d = w.actors.downcast::<EffDust>(dh).expect("Eff_Dust");
        assert_eq!((d.life, d.actor.killed), (if k < 10 { 9 - k as u8 } else { 0 }, k == 10), "frame {k}");
    }
    pad.step(&mut w, stick(0, 0));
    assert!(!w.actors.exists(dh));
}

#[test]
fn the_big_spin_at_0_85_with_the_master_sword() {
    let Some(a) = assets() else { return };
    let mut w = adult(&a);
    let d = w.data.clone();
    draw_sword(&mut w);
    let (_, mut pad) = start_charge(&mut w);
    // Player_GetMeleeWeaponHeld: PLAYER_MELEEWEAPON_SWORD_MASTER (1) | 0x200; swordType 0.
    let (h, t) = thunder(&w).expect("En_M_Thunder");
    assert_eq!((t.actor.params, t.sword_type), (0x201, 0));
    charge_to(&mut w, &mut pad, 0.16);
    // The Master Sword's sparkles: type 2.
    assert_eq!(dust(&w).expect("Eff_Dust").1.actor.params, 2);
    charge_to(&mut w, &mut pad, 0.85);
    pad.step(&mut w, stick(0, 0));
    // func_80844BE4 at 0.85: D_80854384[0], PLAYER_MWA_BIG_SPIN_1H; the effect strong:
    // DMG_JUMP_MASTER, scale to 8, NA_SE_IT_ROLLING_CUT_LV2 (sSfxIds[0]).
    let p = w.player();
    assert_eq!((p.action, p.melee_weapon_animation), (Action::Attack, d.items.mwa("BIG_SPIN_1H")));
    let now = w.audio.frames;
    let t = w.actors.downcast::<EnMThunder>(h).unwrap();
    assert_eq!((t.action, t.attack_strength, t.target_scale), (en_m_thunder::Action::SpinAttacking, M_THUNDER_ATTACK_STRONG, 8));
    assert_eq!(t.collider.info.at_dmg_info.dmg_flags, cc::DMG_JUMP_MASTER);
    assert_eq!(sfx_frames(&w, NA_SE_IT_ROLLING_CUT_LV2), vec![now]);
    assert_eq!(w.save.magic_state, MAGIC_STATE_CONSUME);
}

#[test]
fn walking_while_charging() {
    let Some(a) = assets() else { return };
    let mut w = child(&a, false);
    draw_sword(&mut w);
    let (_, mut pad) = start_charge(&mut w);
    charge_to(&mut w, &mut pad, 0.12);
    // From 0.1 (av2.actionVar2 1) the stick walks: func_80840058 with the stick straight ahead of
    // the camera (within 0x2000 of its input yaw) is 1: Player_Action_80845000, at a fifth of
    // the stick's speed (Math_AsymStepToF by 1 up).
    pad.step(&mut w, b(stick(0, 80)));
    assert_eq!(w.player().action, Action::ChargeSpinWalk);
    assert!(w.player().state1 & STATE1_12 != 0);
    let c0 = w.player().unk_858;
    pad.steps(&mut w, b(stick(0, 80)), 6);
    let p = w.player();
    assert_eq!(p.action, Action::ChargeSpinWalk);
    assert!(p.linear_velocity > 0.0 && p.linear_velocity <= 6.0 * 0.2 + 1e-4, "{}", p.linear_velocity);
    // Still charging, 0.02 a frame.
    assert!((p.unk_858 - (c0 + 6.0 * 0.02)).abs() < 1e-4);
    // Sideways (90 degrees off the camera's input yaw): -1, Player_Action_80845308.
    pad.step(&mut w, b(stick(80, 0)));
    assert_eq!(w.player().action, Action::ChargeSpinSideWalk);
    // Stick let go: braking to 0 (no target speed), then charging standing (func_80844DC8).
    for _ in 0..30 {
        pad.step(&mut w, b(stick(0, 0)));
        if w.player().action == Action::ChargeSpin {
            break;
        }
    }
    let p = w.player();
    assert_eq!((p.action, p.linear_velocity, p.action_var2), (Action::ChargeSpin, 0.0, 1));
    assert!(p.state1 & STATE1_12 != 0);
    assert_eq!(w.data.anim_name(p.skel.animation), "link_fighter_power_kiru_wait");
}

/// Turns the stick an eighth a frame at full tilt, and presses B on the frame whose
/// control-stick index is 3 (`Player_CanSpinAttack` compares the ring's slots in their order:
/// then the newest is in slot 3, its three before it in 2, 1, 0, all turning the same way).
fn quick_spin(w: &mut PlayState) {
    const DIRS: [(i8, i8); 8] = [(0, 80), (56, 56), (80, 0), (56, -56), (0, -80), (-56, -56), (-80, 0), (-56, 56)];
    let mut pad = Pad::new();
    let mut k = 0;
    loop {
        let (x, y) = DIRS[k % 8];
        let next = (w.player().control_stick_data_index + 1) % 4;
        if k >= 4 && next == 3 {
            pad.step(w, b(stick(x, y)));
            return;
        }
        pad.step(w, stick(x, y));
        k += 1;
        assert!(k < 20);
    }
}

#[test]
fn the_quick_spin_without_magic() {
    let Some(a) = assets() else { return };
    let mut w = child(&a, false);
    let d = w.data.clone();
    draw_sword(&mut w);
    quick_spin(&mut w);
    // Player_ActionHandler_7: func_80837818's PLAYER_MWA_SPIN_ATTACK_1H (Player_CanSpinAttack),
    // PLAYER_STATE2_17, func_80837530(play, this, 0): the charge 0.5, En_M_Thunder with
    // PLAYER_MELEEWEAPON_SWORD_KOKIRI | 0. Its init sees PLAYER_STATE2_17 without magic acquired:
    // the plain spin's sounds, killed at once (and deleted in its category's turn the same
    // frame: Actor_UpdateAll reaches it after Player). Player's sword quads hit
    // (PLAYER_STATE2_17 still on).
    let p = w.player();
    assert_eq!((p.action, p.melee_weapon_animation), (Action::Attack, d.items.mwa("SPIN_ATTACK_1H")));
    assert_eq!(p.unk_858, 0.5);
    assert!(p.state2 & STATE2_17 != 0);
    assert!(p.state1 & STATE1_12 != 0);
    assert!(thunder(&w).is_none());
    let now = w.audio.frames;
    assert_eq!(sfx_frames(&w, NA_SE_IT_ROLLING_CUT), vec![now]);
    assert_eq!(sfx_frames(&w, NA_SE_IT_SWORD_SWING_HARD), vec![now]);
    assert!(sfx_frames(&w, NA_SE_IT_ROLLING_CUT_LV1).is_empty());
}

#[test]
fn the_quick_spin_with_magic() {
    let Some(a) = assets() else { return };
    let mut w = child(&a, true);
    draw_sword(&mut w);
    quick_spin(&mut w);
    // The init with magic acquired, the meter idle and no magic asked for (params' high byte 0):
    // it turns Player's PLAYER_STATE2_17 off and spins weak at once (isUsingMagic: its destroy's
    // Magic_Reset), NA_SE_IT_ROLLING_CUT_LV1; nothing consumed.
    let (h, t) = thunder(&w).expect("En_M_Thunder");
    assert_eq!(t.actor.params, 2);
    assert_eq!((t.action, t.attack_strength, t.target_scale, t.spin_attack_timer), (en_m_thunder::Action::SpinAttacking, M_THUNDER_ATTACK_WEAK, 2, 1.0));
    assert!(t.is_using_magic && !t.actor.killed);
    assert_eq!(t.collider.info.at_dmg_info.dmg_flags, cc::DMG_SPIN_KOKIRI);
    assert!(w.player().state2 & STATE2_17 == 0);
    let now = w.audio.frames;
    assert_eq!(sfx_frames(&w, NA_SE_IT_ROLLING_CUT_LV1), vec![now]);
    assert!(sfx_frames(&w, NA_SE_IT_ROLLING_CUT).is_empty());
    assert_eq!((w.save.magic, w.save.magic_state), (0x30, MAGIC_STATE_IDLE));
    // 16 frames of spinning, then gone; the meter as it was.
    Pad::new().steps(&mut w, stick(0, 0), 18);
    assert!(!w.actors.exists(h));
    assert_eq!((w.save.magic, w.save.magic_state), (0x30, MAGIC_STATE_IDLE));
}

fn get(w: &PlayState, h: ActorHandle) -> &EffDust {
    w.actors.downcast::<EffDust>(h).unwrap()
}

#[test]
fn eff_dust_types() {
    let Some(a) = assets() else { return };
    let mut w = child(&a, false);
    let pos = w.player().actor.world_pos + Vec3::new(0.0, 50.0, 100.0);
    let spawn = |w: &mut PlayState, params: i16| w.actor_spawn(ACTOR_EFF_DUST, pos, [0; 3], params).expect("Eff_Dust");
    // EffDust_Init: each type's spread (dx, dy, dz) and size; every sparkle spent (1.0).
    let h0 = spawn(&mut w, eff_dust::EFF_DUST_TYPE_0);
    let h1 = spawn(&mut w, eff_dust::EFF_DUST_TYPE_1);
    let h4 = spawn(&mut w, eff_dust::EFF_DUST_TYPE_4);
    let h5 = spawn(&mut w, 5);
    let e0 = get(&w, h0);
    assert_eq!((e0.dx, e0.dy, e0.dz, e0.scaling_factor, e0.life, e0.index), (1.0, 0.8, 0.8, 0.1, 10, 0));
    assert!(e0.distance_traveled.iter().all(|&d| d == 1.0));
    let e1 = get(&w, h1);
    assert_eq!((e1.dx, e1.dy, e1.dz, e1.scaling_factor), (0.8, 1.0, 0.8, 0.5));
    let e4 = get(&w, h4);
    assert_eq!((e4.dx, e4.scaling_factor, e4.actor.room), (0.5, 20.0, -1));
    // Any other type: SYSTEM_ARENA_FREE (@bug (game)): killed here.
    assert!(get(&w, h5).actor.killed);
    // A frame: type 0 starts three sparkles (index 3, those at 0), type 1 two; type 4 with no
    // parent counts its life down.
    Pad::new().step(&mut w, stick(0, 0));
    let e0 = get(&w, h0);
    assert_eq!(e0.index, 3);
    assert_eq!(e0.distance_traveled.iter().filter(|&&d| d == 0.0).count(), 3);
    let e1 = get(&w, h1);
    assert_eq!(e1.index, 2);
    assert_eq!(get(&w, h4).life, 9);
    // The next: type 0's on their way by 0.05 (three more), type 1's by 0.03 (two more).
    Pad::new().step(&mut w, stick(0, 0));
    let e0 = get(&w, h0);
    assert_eq!((e0.index, e0.distance_traveled[0]), (6, 0.05));
    let e1 = get(&w, h1);
    assert_eq!((e1.index, e1.distance_traveled[0]), (4, 0.03));
    assert!(!w.actors.exists(h5));
}
