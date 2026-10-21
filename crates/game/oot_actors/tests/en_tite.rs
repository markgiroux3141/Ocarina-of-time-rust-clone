//! GAME-09 milestone 2 against the C: the Tektites (`z_en_tite.c`). The trail's four red ones
//! (`spot16` room 0, layer 0: params -1), the blue ones on Zora's River (`spot03` room 0: -2), and
//! `En_Encount1`'s red ones (`z_en_encount1.c`: placed only in the test scene `sasatest`, layer 3).
//!
//! Child Link enters Death Mountain Trail (`ENTR_DEATH_MOUNTAIN_TRAIL_0`) with `ranch-epona`
//! (the Kokiri Sword) and `EVENTCHKINF_A1` (`gDMTIntroCs` seen). Expected values are worked out
//! from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_B, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_encount1::EnEncount1;
use oot_actors::en_item00::EnItem00;
use oot_actors::en_part::{EnPart, tite_body_bake_name};
use oot_actors::en_tite::*;
use oot_game::actor::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::{NA_SE_EN_DODO_M_GND, NA_SE_EN_GOMA_JR_FREEZE, NA_SE_PL_BODY_HIT, SfxPos};
use oot_game::collision_check::{AC_HIT, AT_HIT, MASS_HEAVY};
use oot_game::effect::{EFFECT_SS_DEAD_SOUND, EFFECT_SS_EN_ICE, EFFECT_SS_G_RIPPLE, EFFECT_SS_KIRAKIRA, g_ripple};
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// The trail's four (`ootx scene-info --scene spot16 --layer 0`: `En_Tite` params 0xFFFF).
const HOMES: [Vec3; 4] = [Vec3::new(-2012.0, 258.0, 1953.0), Vec3::new(-1996.0, 414.0, 683.0), Vec3::new(-1918.0, 357.0, 1085.0), Vec3::new(-1776.0, 790.0, -19.0)];
/// Zora's River's two (`spot03` room 0, layers 0 and 1: params 0xFFFE).
const RIVER: [Vec3; 2] = [Vec3::new(-122.0, 180.0, -1252.0), Vec3::new(330.0, 180.0, -1108.0)];

/// `EVENTCHKINF_A1` (`save.h`: 0xA1), `gDMTIntroCs`'s flag (`sEntranceCutsceneTable`).
const EVENTCHKINF_A1: u16 = 0xA1;

/// Enters `entrance` with `save` set up by `f`, the sound log on, two frames run.
fn enter(a: &Arc<GameAssets>, entrance: &str, adult: bool, time: (u8, u8), f: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(time.0 as _, time.1 as _) as u16);
    f(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    w
}

/// Child Link on the trail at 10:00 with `ranch-epona`, its intro seen.
fn trail(a: &Arc<GameAssets>) -> PlayState {
    enter(a, "ENTR_DEATH_MOUNTAIN_TRAIL_0", false, (10, 0), |s| {
        s.apply_preset("ranch-epona").unwrap();
        s.set_event_chk_inf(EVENTCHKINF_A1);
    })
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn all<T: 'static + oot_game::actor_ctx::ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some_and(|a| !a.base().killed)).collect()
}

fn tite(w: &PlayState, h: ActorHandle) -> &EnTite {
    w.actors.downcast::<EnTite>(h).expect("En_Tite")
}

fn tite_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnTite {
    w.actors.downcast_mut::<EnTite>(h).expect("En_Tite")
}

/// The Tektite whose home is `home`.
fn at_home(w: &PlayState, home: Vec3) -> ActorHandle {
    *all::<EnTite>(w).iter().find(|&&h| tite(w, h).actor.home_pos == home).expect("the Tektite")
}

fn anim_name(t: &EnTite) -> &str {
    t.skel.animation.as_ref().map(|a| a.name.as_str()).unwrap_or("")
}

/// The frames a sound was asked for at `pos`.
fn sfx_frames(w: &PlayState, id: u16, pos: Option<SfxPos>) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && pos.is_none_or(|q| q == p)).map(|&(f, _, _)| f).collect()
}

fn effects_of(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == ty).count()
}

/// Link `d` across from `t`'s position towards the trail's entrance (spawn 0, (-1344, 118, 2221)),
/// 30 up to land, facing it; two frames to land.
fn link_near(w: &mut PlayState, h: ActorHandle, d: f32) {
    let pos = tite(w, h).actor.world_pos;
    let to_spawn = (Vec3::new(-1344.0, pos.y, 2221.0) - pos).normalize();
    let at = pos + to_spawn * d + Vec3::Y * 30.0;
    let yaw = eng_math::vec3f_yaw(at, pos);
    w.place_player(at, yaw);
    idle(w, 2);
}

/// A hit as `CollisionCheck_AC` leaves it for the next update: `AC_HIT`, the table's damage and
/// reaction in `colChkInfo`.
fn hit(w: &mut PlayState, h: ActorHandle, damage: u8, reaction: u8) {
    let t = tite_mut(w, h);
    t.collider.base.ac_flags |= AC_HIT;
    t.actor.col_chk_info.damage = damage;
    t.actor.col_chk_info.damage_reaction = reaction;
}

/// Runs until `done` or `max` frames; the frames run.
fn run_until(w: &mut PlayState, max: usize, mut done: impl FnMut(&PlayState) -> bool) -> usize {
    for n in 0..max {
        if done(w) {
            return n;
        }
        idle(w, 1);
    }
    panic!("not reached in {max} frames");
}

/// `DMG_ENTRY(damage, effect)` of `sDamageTable`, in its order.
const DAMAGE_TABLE: [(u8, u8); 32] = [
    (0, 0x1), // Deku nut
    (2, 0x0), // Deku stick
    (1, 0x0), // Slingshot
    (2, 0x0), // Explosive
    (0, 0x1), // Boomerang
    (2, 0x0), // Normal arrow
    (2, 0x0), // Hammer swing
    (0, 0x1), // Hookshot
    (1, 0x0), // Kokiri sword
    (2, 0x0), // Master sword
    (4, 0x0), // Giant's Knife
    (2, 0x0), // Fire arrow
    (4, 0xF), // Ice arrow
    (2, 0x0), // Light arrow
    (2, 0x0), // Unk arrow 1
    (2, 0x0), // Unk arrow 2
    (2, 0x0), // Unk arrow 3
    (0, 0xE), // Fire magic
    (3, 0xF), // Ice magic
    (0, 0xE), // Light magic
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

#[test]
fn the_trails_four_red_tektites_spawn_with_2_health() {
    let Some(a) = assets() else { return };
    let w = trail(&a);
    assert_eq!(w.cs_ctx.state, oot_game::cutscene::CS_STATE_IDLE, "EVENTCHKINF_A1: no intro");
    let ts = all::<EnTite>(&w);
    assert_eq!(ts.len(), 4);
    for home in HOMES {
        let t = tite(&w, at_home(&w, home));
        // params -1 (TEKTITE_RED): health 2, NAVI_ENEMY_RED_TEKTITE (0x45); sInitChain:
        // lockOnArrowOffset 2000, minVelocityY -40, gravity -1; ATTENTION_RANGE_3, scale 0.01,
        // MASS_HEAVY; ActorShape_Init(-200, ActorShadow_DrawCircle, 70).
        assert_eq!((t.actor.params, t.actor.col_chk_info.health, t.actor.navi_enemy_id), (TEKTITE_RED, 2, 0x45));
        assert_eq!((t.actor.target_arrow_offset, t.actor.min_velocity_y, t.actor.target_mode), (2000.0, -40.0, 3));
        assert_eq!((t.actor.scale, t.actor.col_chk_info.mass, t.actor.shape_y_offset, t.actor.shape_shadow_scale), (Vec3::splat(0.01), MASS_HEAVY, -200.0, 70.0));
        // UPDBGCHECKINFO_FLAG_0 | _2 | _3 | _4 (no _6: the red one's ripples are the engine's).
        assert_eq!(t.unk_2dc, 0x1D);
        assert_eq!((t.flip_state, t.body_break.val), (TEKTITE_INITIAL, 0));
        // EnTite_SetupIdle: object_tite_Anim_0012E4 looped, vIdleTimer Rand_S16Offset(15, 30)
        // (15 to 44), counted down from its first update.
        assert_eq!((t.func, t.action, anim_name(t)), (Func::Idle, TEKTITE_IDLE, "object_tite_Anim_0012E4"));
        assert!((13..=44).contains(&t.action_var1), "{}", t.action_var1);
        // sJntSphElementsInit: one sphere on limb 0 (the actor's matrix) 1500 up, 20 across
        // (scale 100), 8 damage.
        let e = &t.collider.elements[0];
        assert_eq!((t.collider.elements.len(), e.dim.limb, e.dim.model_sphere.center, e.dim.model_sphere.radius), (1, 0, [0, 1500, 0], 20));
        assert_eq!((e.info.at_dmg_info.damage, e.info.at_dmg_info.dmg_flags, e.info.ac_dmg_info.dmg_flags), (8, 0xFFCF_FFFF, 0xFFCF_FFFF));
        // Drawn (in its culling volume or not) its sphere is on it: 15 up less 2 (the yOffset).
        if t.actor.is_drawn {
            let c = e.dim.world_sphere.center;
            assert!((c[1] as f32 - (t.actor.world_pos.y + 13.0)).abs() <= 1.0, "{c:?} {}", t.actor.world_pos);
        }
    }
}

#[test]
fn near_link_it_turns_to_him_and_hops_at_him() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = at_home(&w, HOMES[0]);
    idle(&mut w, 20);
    link_near(&mut w, h, 250.0);
    let t = tite(&w, h);
    assert!(t.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0, "on the ground");
    assert!((t.actor.xz_dist_to_player - 250.0).abs() < 20.0 && t.actor.y_dist_to_player.abs() <= 80.0, "{} {}", t.actor.xz_dist_to_player, t.actor.y_dist_to_player);
    // Facing away from Link, its idle timer out: EnTite_Idle (within 300 across and 80 up) sets up
    // the turn (object_tite_Anim_000A14 looped).
    let away = tite(&w, h).actor.yaw_towards_player.wrapping_add(i16::MIN);
    let t = tite_mut(&mut w, h);
    t.actor.world_rot.y = away;
    t.actor.shape_rot.y = away;
    t.action_var1 = 0;
    idle(&mut w, 1);
    let t = tite(&w, h);
    assert_eq!((t.func, t.action, anim_name(t)), (Func::TurnTowardPlayer, TEKTITE_TURN_TOWARD_PLAYER, "object_tite_Anim_000A14"));
    // EnTite_TurnTowardPlayer: angleToPlayer (s16) = its yaw to Link less world.rot.y;
    // turnVelocity (s16) = angle / 42 ± 10; world.rot.y += turnVelocity × 2, playSpeed
    // turnVelocity × 0.01.
    let link = w.player().actor.world_pos;
    let rot0 = t.actor.world_rot.y;
    let angle = eng_math::vec3f_yaw(t.actor.world_pos, link).wrapping_sub(rot0);
    let turn = if angle > 0 { (angle as f32 / 42.0 + 10.0) as i16 } else { (angle as f32 / 42.0 - 10.0) as i16 };
    idle(&mut w, 1);
    let t = tite(&w, h);
    assert_eq!(t.actor.world_rot.y, (rot0 as i32 + turn as i32 * 2) as i16);
    assert_eq!((t.actor.shape_rot.y, t.skel.play_speed), (t.actor.world_rot.y, turn as f32 * 0.01));
    // Facing him within 3640, 250 away (over 180): EnTite_SetupMoveTowardPlayer: the hop
    // (object_tite_Anim_000C70), 10 up (9 after the frame's gravity), 4 on, NA_SE_EN_STAL_JUMP,
    // one to three hops more (vQueuedJumps).
    run_until(&mut w, 100, |w| tite(w, h).func != Func::TurnTowardPlayer);
    let t = tite(&w, h);
    assert_eq!((t.func, t.action, anim_name(t)), (Func::MoveTowardPlayer, TEKTITE_MOVE_TOWARD_PLAYER, "object_tite_Anim_000C70"));
    assert!((t.actor.yaw_towards_player.wrapping_sub(t.actor.shape_rot.y) as i32).abs() < 3640 + 2 * 100);
    assert_eq!((t.actor.velocity.y, t.actor.speed_xz), (9.0, 4.0));
    assert!((1..=3).contains(&t.action_var2), "{}", t.action_var2);
    assert_eq!(sfx_frames(&w, NA_SE_EN_STAL_JUMP, Some(SfxPos::Actor(h))), [w.audio.frames]);
    // Hopping at him (the landings' NA_SE_EN_DODO_M_GND) it closes in; within 180 it turns and
    // attacks (EnTite_SetupAttack: object_tite_Anim_00083C).
    let d0 = t.actor.xz_dist_to_player;
    run_until(&mut w, 300, |w| tite(w, h).func == Func::Attack);
    let t = tite(&w, h);
    assert!(t.actor.xz_dist_to_player <= 180.0 && t.actor.xz_dist_to_player < d0, "{} from {d0}", t.actor.xz_dist_to_player);
    assert!(!sfx_frames(&w, NA_SE_EN_DODO_M_GND, Some(SfxPos::Actor(h))).is_empty());
}

#[test]
fn its_lunge_hurts_link_and_it_recoils_off_him() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = at_home(&w, HOMES[0]);
    idle(&mut w, 20);
    // Link 60 in front of it, it facing him, its idle timer out: it turns (already facing) and
    // attacks (within 180 across, 80 up).
    link_near(&mut w, h, 60.0);
    let yaw = tite(&w, h).actor.yaw_towards_player;
    let t = tite_mut(&mut w, h);
    t.actor.world_rot.y = yaw;
    t.actor.shape_rot.y = yaw;
    t.action_var1 = 0;
    let health = w.save.health;
    run_until(&mut w, 5, |w| tite(w, h).func == Func::Attack);
    // EnTite_SetupAttack: object_tite_Anim_00083C once, vAttackState TEKTITE_BEGIN_LUNGE, speed
    // and velocity 0, one to three lunges more.
    let t = tite(&w, h);
    assert_eq!((t.action, t.action_var1, anim_name(t), t.actor.speed_xz), (TEKTITE_ATTACK, TEKTITE_BEGIN_LUNGE, "object_tite_Anim_00083C", 0.0));
    assert!((1..=3).contains(&t.action_var2));
    // The crouch's end: TEKTITE_MID_LUNGE (D_80B1B634[1], object_tite_Anim_0004F8), 8 up (7 after
    // the frame's gravity), 4 on, NA_SE_EN_STAL_JUMP; its feet sparkle leaving the ground
    // (func_800355B8: EffectSsKiraKira).
    run_until(&mut w, 20, |w| tite(w, h).action_var1 != TEKTITE_BEGIN_LUNGE);
    let t = tite(&w, h);
    assert_eq!((t.action_var1, anim_name(t)), (TEKTITE_MID_LUNGE, "object_tite_Anim_0004F8"));
    assert_eq!((t.actor.velocity.y, t.actor.speed_xz), (7.0, 4.0));
    assert_eq!(sfx_frames(&w, NA_SE_EN_STAL_JUMP, Some(SfxPos::Actor(h))), [w.audio.frames]);
    assert!(effects_of(&w, EFFECT_SS_KIRAKIRA) >= 4);
    // Its sphere hits Link: half a heart (8); then it's knocked back off him at -6 (only
    // EnTite_SetupAction(EnTite_Recoil): `action` stays TEKTITE_ATTACK), NA_SE_PL_BODY_HIT on him.
    run_until(&mut w, 30, |w| tite(w, h).func == Func::Recoil);
    let t = tite(&w, h);
    assert_eq!((t.action, t.collider.base.at_flags & AT_HIT), (TEKTITE_ATTACK, 0));
    assert!(t.actor.speed_xz < 0.0, "{}", t.actor.speed_xz);
    let link = w.player.unwrap();
    assert_eq!(sfx_frames(&w, NA_SE_PL_BODY_HIT, Some(SfxPos::Actor(link))), [w.audio.frames]);
    idle(&mut w, 2);
    assert!(w.save.health < health, "{:#x} to {:#x}", health, w.save.health);
    // EnTite_Recoil: slowing by 0.5 a frame; stopped on the ground, it attacks again (Link
    // within 180, it facing him within 6000) or hops at him.
    run_until(&mut w, 40, |w| tite(w, h).func != Func::Recoil);
    assert!(matches!(tite(&w, h).func, Func::Attack | Func::MoveTowardPlayer | Func::Idle), "{:?}", tite(&w, h).func);
}

#[test]
fn two_kokiri_sword_hits_kill_a_red_one() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = at_home(&w, HOMES[0]);
    idle(&mut w, 20);
    link_near(&mut w, h, 250.0);
    // The Kokiri Sword's 1 (DMG_ENTRY(1, 0x0)): EnTite_CheckDamage from idle on: red 8 frames
    // (Actor_SetColorFilter(COLORFILTER_COLORFLAG_RED, 255, OPA, 8)), 2 to 1, NA_SE_EN_TEKU_DAMAGE,
    // EnTite_SetupRecoil (TEKTITE_RECOIL, -6, facing away from Link, gravity -1); its Recoil the
    // same frame (-6 to -5.5).
    hit(&mut w, h, 1, 0);
    idle(&mut w, 1);
    let t = tite(&w, h);
    assert_eq!((t.actor.col_chk_info.health, t.func, t.action), (1, Func::Recoil, TEKTITE_RECOIL));
    assert_eq!((t.actor.color_filter_params, t.actor.color_filter_timer), (0x4000 | (0xF8 << 5) | 8, 8));
    assert_eq!((t.actor.speed_xz, t.actor.world_rot.y), (-5.5, t.actor.yaw_towards_player));
    assert_eq!(sfx_frames(&w, NA_SE_EN_TEKU_DAMAGE, Some(SfxPos::Actor(h))), [w.audio.frames]);
    // Recoiling (TEKTITE_RECOIL, under TEKTITE_IDLE) it isn't hit: the hit's lost.
    hit(&mut w, h, 1, 0);
    idle(&mut w, 1);
    assert_eq!((tite(&w, h).actor.col_chk_info.health, tite(&w, h).func), (1, Func::Recoil));
    run_until(&mut w, 40, |w| tite(w, h).func != Func::Recoil);
    assert_eq!(tite(&w, h).actor.color_filter_timer, 0);
    // The second: 1 to 0, EnTite_SetupDeathCry (colour off, stopped), and EnTite_DeathCry the
    // same frame: its cry (EffectSsDeadSound_SpawnStationary(NA_SE_EN_TEKU_DEAD, 40)),
    // TEKTITE_FALL_APART, BodyBreak_Alloc(24).
    let dead_sounds = effects_of(&w, EFFECT_SS_DEAD_SOUND);
    hit(&mut w, h, 1, 0);
    idle(&mut w, 1);
    let t = tite(&w, h);
    assert_eq!((t.actor.col_chk_info.health, t.func, t.action, t.actor.speed_xz), (0, Func::FallApart, TEKTITE_FALL_APART, 0.0));
    assert_eq!(effects_of(&w, EFFECT_SS_DEAD_SOUND), dead_sounds + 1);
    assert!(w.effect_ss.table.iter().any(|e| e.life > -1 && e.ty == EFFECT_SS_DEAD_SOUND && e.life <= 40));
    // Drawn that frame: BodyBreak_SetInfo takes each of its 24 limbs with a list (ready at the
    // 24th); EnTite_FallApart's BodyBreak_SpawnParts the next frame: an En_Part of kind
    // params + 0xB = 10 for each, its child, at its scale; the drop (table 4), and it's gone.
    assert!(t.actor.is_drawn, "in view");
    let skel = a.skeleton("object_tite", "object_tite_Skel_003A20").unwrap();
    let with_lists = skel.limbs.iter().filter(|l| l.dlists.first().is_some_and(|&d| d != 0)).count();
    assert_eq!(t.body_break.val, -1, "ready");
    assert_eq!(t.body_break.count as usize, with_lists);
    let death_pos = t.actor.world_pos;
    let items: Vec<ActorHandle> = all::<EnItem00>(&w);
    idle(&mut w, 1);
    assert!(!w.actors.exists(h) || w.actors.get(h).unwrap().base().killed);
    let parts: Vec<ActorHandle> = all::<EnPart>(&w).into_iter().filter(|&p| w.actors.actor(p).unwrap().parent == Some(h)).collect();
    assert_eq!(parts.len(), with_lists);
    for &p in &parts {
        let part = w.actors.downcast::<EnPart>(p).unwrap();
        assert_eq!((part.actor.params, part.actor.scale), (10, Vec3::splat(0.01)));
        let (file, sym) = part.display_list.expect("a list");
        assert!(file == "object_tite" && DLISTS.contains(&sym), "{sym}");
    }
    // EnPart_Draw: the body (object_tite_DL_002FF0) with the red's textures on segments 8 to 10.
    let body = parts.iter().copied().find(|&p| w.actors.downcast::<EnPart>(p).unwrap().display_list.unwrap().1 == "object_tite_DL_002FF0").expect("the body");
    let part = w.actors.downcast::<EnPart>(body).unwrap();
    let mut out = DrawOut::default();
    oot_game::actor_ctx::ActorImpl::draw(part, &oot_game::actor_ctx::ActorImpl::render_state(part), &w, &ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
    assert_eq!(out.opa[0].mesh.name, oot_game::pack::keys::bake(&tite_body_bake_name(false)));
    // Item_DropCollectibleRandom(COLLECTIBLE_DROP_TABLE_4): a random one of the table's 16 (half of
    // them none); what came is at its place.
    let new_items: Vec<ActorHandle> = all::<EnItem00>(&w).into_iter().filter(|i| !items.contains(i)).collect();
    for &i in &new_items {
        let it = w.actors.actor(i).unwrap();
        assert!((it.home_pos.x - death_pos.x).abs() < 1.0 && (it.home_pos.z - death_pos.z).abs() < 1.0, "{} vs {death_pos}", it.home_pos);
    }
    println!("the drop: {:?}", new_items.iter().map(|&i| w.actors.actor(i).unwrap().params).collect::<Vec<_>>());
    // The parts fly up and puff out (func_80ACE13C: 5 to 21 frames).
    run_until(&mut w, 40, |w| parts.iter().all(|&p| !w.actors.exists(p)));
}

#[test]
fn a_kokiri_sword_slash_takes_one_off() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = at_home(&w, HOMES[0]);
    idle(&mut w, 20);
    // Link 40 in front of it, facing it; it idles on (vIdleTimer 1000). B: the Kokiri Sword's
    // slash (DMG_SLASH_KOKIRI: DMG_ENTRY(1, 0x0)) on its sphere: 2 to 1, red, recoiling,
    // NA_SE_EN_TEKU_DAMAGE.
    link_near(&mut w, h, 40.0);
    tite_mut(&mut w, h).action_var1 = 1000;
    w.tick_with(scripted_input(PadState::default(), with(PadState::default(), BTN_B)));
    run_until(&mut w, 20, |w| tite(w, h).actor.col_chk_info.health < 2);
    let t = tite(&w, h);
    assert_eq!((t.actor.col_chk_info.health, t.func, t.action, t.damage_reaction), (1, Func::Recoil, TEKTITE_RECOIL, 0));
    assert_eq!(t.actor.color_filter_params & 0x4000, 0x4000);
    assert_eq!(sfx_frames(&w, NA_SE_EN_TEKU_DAMAGE, Some(SfxPos::Actor(h))), [w.audio.frames]);
}

#[test]
fn its_death_drops_from_the_hit_when_the_hit_sets_a_drop_flag() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = at_home(&w, HOMES[0]);
    idle(&mut w, 20);
    link_near(&mut w, h, 250.0);
    tite_mut(&mut w, h).actor.col_chk_info.health = 1;
    hit(&mut w, h, 1, 0);
    idle(&mut w, 1);
    assert_eq!(tite(&w, h).func, Func::FallApart);
    // EnTite_FallApart's drop is Item_DropCollectibleRandom(play, &this->actor, ...): a dropFlag
    // (0x02, as Actor_SetDropFlag sets for an arrow's hit) takes table 1's 7th (a heart: the
    // fixed entry the table's roll is replaced by), whatever the roll; Link not at full health
    // (func_8001F404 makes a heart a green rupee at full health).
    tite_mut(&mut w, h).actor.drop_flag = 0x02;
    w.save.health = 0x20;
    let items = all::<EnItem00>(&w);
    idle(&mut w, 1);
    let new_items: Vec<i16> = all::<EnItem00>(&w).into_iter().filter(|i| !items.contains(i)).map(|i| w.actors.actor(i).unwrap().params).collect();
    assert_eq!(new_items.iter().map(|p| p & 0xFF).collect::<Vec<_>>(), [oot_actors::en_item00::ITEM00_RECOVERY_HEART]);
}

#[test]
fn nuts_stun_it_blue_and_ice_freezes_it() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = at_home(&w, HOMES[0]);
    idle(&mut w, 20);
    link_near(&mut w, h, 250.0);
    // A nut (DMG_ENTRY(0, 0x1)): blue (COLORFILTER_COLORFLAG_BLUE, 120, OPA, 80), no damage,
    // EnTite_SetupStunned: TEKTITE_STUNNED, the idle animation held (speed 0), knocked back 6
    // (-5.5 after its Stunned that frame), NA_SE_EN_GOMA_JR_FREEZE.
    hit(&mut w, h, 0, 1);
    idle(&mut w, 1);
    let t = tite(&w, h);
    assert_eq!((t.func, t.action, t.actor.col_chk_info.health, t.skel.play_speed), (Func::Stunned, TEKTITE_STUNNED, 2, 0.0));
    assert_eq!((t.actor.color_filter_params, t.actor.color_filter_timer, t.actor.speed_xz), (((120 & 0xF8) << 5) | 80, 80, -5.5));
    assert_eq!(sfx_frames(&w, NA_SE_EN_GOMA_JR_FREEZE, Some(SfxPos::Actor(h))), [w.audio.frames]);
    assert_eq!(t.spawn_ice_timer, 0);
    // Another nut while stunned: nothing new (the timer runs on).
    idle(&mut w, 10);
    hit(&mut w, h, 0, 1);
    idle(&mut w, 1);
    assert_eq!(tite(&w, h).actor.color_filter_timer, 80 - 11);
    // Stunned till its colour's gone (80 frames), then on (Link 250 away: it hops at him).
    run_until(&mut w, 80, |w| tite(w, h).func != Func::Stunned);
    assert_eq!(tite(&w, h).actor.color_filter_timer, 0);
    assert_eq!(tite(&w, h).func, Func::MoveTowardPlayer);
    // Ice magic (DMG_ENTRY(3, 0xF)) on a fresh one: stunned with 3 damage (2 to 0), spawnIceTimer
    // 48; its draw holds the colour on and every 4 frames puts an ice chunk on it
    // (EffectSsEnIce_SpawnFlyingVec3f); dead once the colour's gone and it's down.
    let h = at_home(&w, HOMES[2]);
    link_near(&mut w, h, 250.0);
    hit(&mut w, h, 3, 0xF);
    idle(&mut w, 1);
    let t = tite(&w, h);
    assert_eq!((t.func, t.actor.col_chk_info.health, t.damage_reaction), (Func::Stunned, 0, 0xF));
    // (The draw that frame: 47, its colour's 80 up one, to 81.)
    assert_eq!((t.spawn_ice_timer, t.actor.color_filter_timer), (47, 81));
    idle(&mut w, 8);
    assert!(effects_of(&w, EFFECT_SS_EN_ICE) >= 2);
    run_until(&mut w, 250, |w| tite(w, h).func != Func::Stunned);
    assert_eq!(tite(&w, h).func, Func::DeathCry);
}

#[test]
fn fire_magic_does_nothing_and_holds_it_still_that_frame() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = at_home(&w, HOMES[0]);
    idle(&mut w, 20);
    link_near(&mut w, h, 250.0);
    // Din's Fire (DMG_ENTRY(0, 0xE)): the hit cleared, nothing else; EnTite_Update skips its
    // action and movement that frame (colChkInfo.damageReaction 0xE).
    let t = tite_mut(&mut w, h);
    t.action_var1 = 5;
    hit(&mut w, h, 0, 0xE);
    idle(&mut w, 1);
    let t = tite(&w, h);
    assert_eq!((t.func, t.action_var1, t.actor.col_chk_info.health, t.actor.color_filter_timer), (Func::Idle, 5, 2, 0));
    assert_eq!(t.collider.base.ac_flags & AC_HIT, 0);
}

#[test]
fn the_hammers_floor_hit_flips_it_on_its_back() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = at_home(&w, HOMES[0]);
    idle(&mut w, 20);
    link_near(&mut w, h, 250.0);
    tite_mut(&mut w, h).action_var1 = 100;
    // actorCtx.unk_02 (the hammer's floor hit: func_80842A28 sets 4 in Player's update, and
    // Actor_UpdateAll counts it down first each frame; nothing sets it in the port): a grounded
    // Tektite within 400 of Link, from idle on: EnTite_SetupFlipOnBack: TEKTITE_FLIPPED, 500 frames on its
    // back, 11 up (10 after gravity), the turn at 1.5, NA_SE_EN_TEKU_REVERSE. Its FlipOnBack the
    // same frame still finds the ground (last frame's flags): 500 to 499.
    w.actors.unk_02 = 4;
    idle(&mut w, 1);
    let t = tite(&w, h);
    assert_eq!((t.func, t.flip_state, t.action_var1, t.actor.velocity.y), (Func::FlipOnBack, TEKTITE_FLIPPED, 499, 10.0));
    assert_eq!((anim_name(t), t.skel.play_speed), ("object_tite_Anim_000A14", 1.5));
    assert_eq!(sfx_frames(&w, NA_SE_EN_TEKU_REVERSE, Some(SfxPos::Actor(h))), [w.audio.frames]);
    // (Off the ground the next three frames: the hit's over for it.)
    // In the air its yOffset rises 400 a frame to its back (to 3000); landed, upside down
    // (func_800359B8, then 0x7FFF on the roll), dust and NA_SE_EN_DODO_M_GND.
    run_until(&mut w, 40, |w| tite(w, h).actor.bg_check_flags & BGCHECKFLAG_GROUND != 0);
    idle(&mut w, 2);
    let t = tite(&w, h);
    assert_eq!(t.actor.shape_y_offset, 3000.0);
    assert!((t.actor.shape_rot.z as i32).abs() > 0x6000, "{:#x}", t.actor.shape_rot.z);
    assert!(t.action_var1 < 500);
    // A sword hit on its back: 2 to 1, NA_SE_EN_TEKU_DAMAGE, EnTite_SetupFlipUpright
    // (TEKTITE_UNFLIPPED, 13 up, NA_SE_EN_TEKU_REVERSE); landed, idle with its yOffset 0.
    hit(&mut w, h, 1, 0);
    idle(&mut w, 1);
    let t = tite(&w, h);
    assert_eq!((t.func, t.flip_state, t.actor.col_chk_info.health, t.actor.velocity.y), (Func::FlipUpright, TEKTITE_UNFLIPPED, 1, 12.0));
    run_until(&mut w, 60, |w| tite(w, h).func != Func::FlipUpright);
    let t = tite(&w, h);
    assert_eq!((t.actor.shape_y_offset, t.flip_state), (0.0, TEKTITE_UNFLIPPED));
    assert_ne!(t.func, Func::FlipUpright);
}

#[test]
fn a_blue_one_floats_on_zoras_river() {
    let Some(a) = assets() else { return };
    let mut w = enter(&a, "ENTR_ZORAS_RIVER_0", false, (10, 0), |s| s.apply_preset("ranch-epona").unwrap());
    let ts = all::<EnTite>(&w);
    assert_eq!(ts.len(), 2);
    idle(&mut w, 60);
    for home in RIVER {
        let h = at_home(&w, home);
        let t = tite(&w, h);
        // params -2 (TEKTITE_BLUE): health 4, NAVI_ENEMY_BLUE_TEKTITE (0x46), UPDBGCHECKINFO_FLAG_6.
        assert_eq!((t.actor.params, t.actor.col_chk_info.health, t.actor.navi_enemy_id, t.unk_2dc), (TEKTITE_BLUE, 4, 0x46, 0x5D));
        // EnTite_Idle on water: gravity 0, velocity.y and depthInWater eased to 0 (2 a frame):
        // on the surface.
        assert!(t.actor.bg_check_flags & BGCHECKFLAG_WATER != 0, "in water at {}", t.actor.world_pos);
        assert_eq!(t.actor.gravity, 0.0);
        assert!(t.actor.y_dist_to_water.abs() < 0.01 && t.actor.velocity.y.abs() < 0.01, "{} {}", t.actor.y_dist_to_water, t.actor.velocity.y);
        let y = t.actor.world_pos.y;
        idle(&mut w, 30);
        let t = tite(&w, h);
        assert!((t.actor.world_pos.y - y).abs() < 0.01, "it stays on the surface");
        // Its feet on the water (EnTite_Update): a foot at or under the surface is put on it with a
        // ripple (EffectSsGRipple_Spawn(.., 0, 220, 0)), each foot every 8 frames (its own phase).
        // The feet are where the last draw put them (EnTite_PostLimbDraw): held 5 under the
        // surface here, round it, for 8 frames.
        let pos = t.actor.world_pos;
        let offsets = [Vec3::new(13.0, -5.0, -11.0), Vec3::new(-13.0, -5.0, -11.0), Vec3::new(-13.0, -5.0, 11.0), Vec3::new(13.0, -5.0, 11.0)];
        for _ in 0..8 {
            let t = tite_mut(&mut w, h);
            t.back_right_foot_pos = pos + offsets[0];
            t.back_left_foot_pos = pos + offsets[1];
            t.front_left_foot_pos = pos + offsets[2];
            t.front_right_foot_pos = pos + offsets[3];
            idle(&mut w, 1);
        }
        for o in offsets {
            let foot = Vec3::new(pos.x + o.x, y, pos.z + o.z);
            let n = w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == EFFECT_SS_G_RIPPLE && e.regs[g_ripple::R_RADIUS_MAX] == 220 && e.pos.distance(foot) < 0.01).count();
            assert!(n >= 1, "a ripple on the surface under the foot at {foot}");
        }
    }
    // Link 150 in front of one: it turns to him on the water and lunges off the surface (its
    // depthInWater added back, NA_SE_EN_TEKU_JUMP_WATER), 8 up and 4 on, landing back on it
    // (falling at -7, not under -8: its speed and fall stopped there, NA_SE_EN_TEKU_LAND_WATER2;
    // WATER_TOUCH cleared).
    let h = at_home(&w, RIVER[0]);
    let surface = tite(&w, h).actor.world_pos.y;
    let pos = tite(&w, h).actor.world_pos;
    w.place_player(pos + Vec3::new(0.0, 0.0, 150.0), -0x8000);
    idle(&mut w, 2);
    tite_mut(&mut w, h).action_var1 = 0;
    run_until(&mut w, 60, |w| !sfx_frames(w, NA_SE_EN_TEKU_JUMP_WATER, Some(SfxPos::Actor(h))).is_empty());
    run_until(&mut w, 60, |w| tite(w, h).actor.world_pos.y > surface + 10.0);
    run_until(&mut w, 120, |w| {
        let t = tite(w, h);
        t.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 && t.actor.velocity.y == 0.0
    });
    assert!(!sfx_frames(&w, NA_SE_EN_TEKU_LAND_WATER2, Some(SfxPos::Actor(h))).is_empty());
    let t = tite(&w, h);
    assert!((t.actor.world_pos.y - surface).abs() < 10.0 && t.actor.speed_xz == 0.0 && t.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH == 0);
}

#[test]
fn en_encount1_spawns_red_tektites_two_at_a_time() {
    let Some(a) = assets() else { return };
    // sasatest's layer 3 (an adult at night: ENTR_SASATEST_0 + 3) places En_Encount1 0x0AB2 at (4,
    // 0, -245): spawnType PARAMS_GET_U(params, 11, 5) = 1 (SPAWNER_TEKTITE), maxTotalSpawns
    // PARAMS_GET_U(params, 0, 6) = 0x32 (50); maxCurSpawns set to 2; range 120 (rot.z 0).
    let mut w = enter(&a, "ENTR_SASATEST_0", true, (22, 0), |_| {});
    assert_eq!(w.save.scene_layer, 3);
    let enc = all::<EnEncount1>(&w)[0];
    let e = w.actors.downcast::<EnEncount1>(enc).unwrap();
    assert_eq!((e.spawn_type, e.max_cur_spawns, e.max_total_spawns, e.spawn_range), (1, 2, 50, 120.0));
    // Link at spawn 0 (11, 0, 11): 256 away, out of range: none.
    idle(&mut w, 20);
    assert!(all::<EnTite>(&w).is_empty());
    // Within 120 (and 100 up or down): every 10 frames a red one (TEKTITE_RED) within 25 of it on
    // the floor, its child, till two are out.
    let home = Vec3::new(4.0, 0.0, -245.0);
    w.place_player(home + Vec3::new(0.0, 0.0, 100.0), -0x8000);
    run_until(&mut w, 30, |w| all::<EnTite>(w).len() == 2);
    let ts = all::<EnTite>(&w);
    for &h in &ts {
        let t = tite(&w, h);
        assert_eq!((t.actor.params, t.actor.parent, t.actor.col_chk_info.health), (TEKTITE_RED, Some(enc), 2));
        assert!((t.actor.home_pos.x - home.x).abs() <= 25.0 && (t.actor.home_pos.z - home.z).abs() <= 25.0, "{}", t.actor.home_pos);
    }
    idle(&mut w, 30);
    assert_eq!(all::<EnTite>(&w).len(), 2);
    assert_eq!((w.actors.downcast::<EnEncount1>(enc).unwrap().cur_num_spawn, w.actors.downcast::<EnEncount1>(enc).unwrap().total_num_spawn), (2, 2));
    // One killed: EnTite_Destroy counts it off (curNumSpawn 1); another comes.
    let h = ts[0];
    tite_mut(&mut w, h).actor.col_chk_info.health = 1;
    hit(&mut w, h, 1, 0);
    run_until(&mut w, 10, |w| !w.actors.exists(h));
    assert_eq!(w.actors.downcast::<EnEncount1>(enc).unwrap().cur_num_spawn, 1);
    run_until(&mut w, 30, |w| all::<EnTite>(w).len() == 2);
    assert_eq!(w.actors.downcast::<EnEncount1>(enc).unwrap().total_num_spawn, 3);
}
