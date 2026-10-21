//! GAME-10 milestone 4 against the C: the Stinger on the floor (`z_en_eiyer.c`). Jabu-Jabu's Belly
//! (Master Quest) room 14, the boomerang's room, places two wanderers (params 0xA) at (1080,
//! -1283, -2261) and (923, -1293, -2355); a leader (params 0) and its clones are spawned here.
//!
//! Entered by `ENTR_JABU_JABU_0` on a `jabu-fish` save, room 14 loaded with `Room_RequestNewRoom`,
//! Link placed out of view (the Stingers aren't updated by the frames). Each test runs one
//! Stinger's update alone (`Actor_UpdateAll`'s call), Link's distances set as `Actor_UpdateAll`
//! would, its hits put on its collider as the collision check leaves them, so no fight is
//! scripted and nothing waits on `Rand`. Expected values are worked out from the C in the
//! comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_eiyer::*;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorHandle, ActorImpl};
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, SfxPos};
use oot_game::collision_check::*;
use oot_game::effect::EFFECT_SS_G_SPLASH;
use oot_game::env::clock_time;
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::shadow::ShadowDraw;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Room 14's two (`ootx scene-info --scene bdan`, layers 0 to 3).
const EAST: Vec3 = Vec3::new(1080.0, -1283.0, -2261.0);
const WEST: Vec3 = Vec3::new(923.0, -1293.0, -2355.0);
/// `ACTOR_EN_ZF`, `ACTOR_EN_RR` (`actor_table.h`: 0x0025, 0x00DD): room 14's other enemies.
const ACTOR_EN_ZF: i16 = 0x0025;
const ACTOR_EN_RR: i16 = 0x00DD;
/// The boomerang's room.
const ROOM: i8 = 14;

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in room 14 (`jabu-fish`), out of view of the Stingers, three frames on, the audio
/// logged.
fn room14(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("jabu-fish").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    idle(&mut w, 2);
    assert!(w.room_request(ROOM));
    idle(&mut w, 1);
    w.room_change_done();
    w.place_player(Vec3::ZERO, 0);
    idle(&mut w, 3);
    assert_eq!(w.room_ctx.cur.num, ROOM);
    w
}

fn eiyers(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnEiyer>(h).is_some_and(|o| !o.actor.killed)).collect()
}

fn at_home(w: &PlayState, home: Vec3) -> ActorHandle {
    *eiyers(w).iter().find(|&&h| ei(w, h).actor.home_pos == home).expect("the Stinger")
}

fn ei(w: &PlayState, h: ActorHandle) -> &EnEiyer {
    w.actors.downcast::<EnEiyer>(h).expect("En_Eiyer")
}

fn ei_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnEiyer {
    w.actors.downcast_mut::<EnEiyer>(h).expect("En_Eiyer")
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<EnEiyer>(h).is_none_or(|o| o.actor.killed)
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// A call on it out of the arena, as the updating actor (its setups that play sounds or spawn).
fn act(w: &mut PlayState, h: ActorHandle, f: impl FnOnce(&mut EnEiyer, &mut PlayState)) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    f(a.as_any_mut().downcast_mut::<EnEiyer>().expect("En_Eiyer"), w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// `Actor_UpdateAll`'s distances: Link `dist` across at `yaw`, `y` above it.
fn set_link(w: &mut PlayState, h: ActorHandle, dist: f32, y: f32, yaw: i16) {
    let o = ei_mut(w, h);
    o.actor.xz_dist_to_player = dist;
    o.actor.y_dist_to_player = y;
    o.actor.yaw_towards_player = yaw;
}

/// A hit as the collision check leaves it: `AC_HIT`, the damage table's damage and reaction.
fn hit(w: &mut PlayState, h: ActorHandle, damage: u8, reaction: u8) {
    let o = ei_mut(w, h);
    o.collider.base.ac_flags |= AC_HIT;
    o.actor.col_chk_info.damage = damage;
    o.actor.col_chk_info.damage_reaction = reaction;
}

fn effects(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.ty == ty && e.life >= 0).count()
}

fn sfx(w: &PlayState, h: ActorHandle, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == SfxPos::Actor(h)).count()
}

fn sfx_any(w: &PlayState, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
}

/// Its draw: the meshes, opaque and translucent.
fn draw(w: &PlayState, h: ActorHandle) -> (Vec<String>, Vec<String>) {
    let o = ei(w, h);
    let rs = o.render_state();
    let mut out = DrawOut::default();
    o.draw(&rs, w, &ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
    (out.opa.iter().map(|c| c.mesh.name.clone()).collect(), out.xlu.iter().map(|c| c.mesh.name.clone()).collect())
}

/// Updates until the action changes; the updates run.
fn until_not(w: &mut PlayState, h: ActorHandle, action: Action, max: usize) -> usize {
    for n in 1..=max {
        update(w, h);
        if gone(w, h) || ei(w, h).action != action {
            return n;
        }
    }
    panic!("still {action:?} after {max}");
}

/// The east one up and wandering under the floor.
fn wandering(w: &mut PlayState) -> ActorHandle {
    let h = at_home(w, EAST);
    until_not(w, h, Action::AppearFromGround, 60);
    assert_eq!(ei(w, h).action, Action::WanderUnderground);
    h
}

/// The east one out and gliding (`EnEiyer_SetupAmbush`, its animation run out).
fn gliding(w: &mut PlayState) -> ActorHandle {
    let h = wandering(w);
    let o = ei_mut(w, h);
    o.collider.base.oc_flags2 |= OC2_HIT_PLAYER;
    update(w, h);
    until_not(w, h, Action::Ambush, 100);
    assert_eq!(ei(w, h).action, Action::Glide);
    h
}

#[test]
fn room_14_places_two_wanderers_under_its_floor() {
    let Some(a) = assets() else { return };
    let w = room14(&a);
    assert_eq!(eiyers(&w).len(), 2);
    for home in [EAST, WEST] {
        let h = at_home(&w, home);
        let o = ei(&w, h);
        assert_eq!((o.actor.params, o.actor.parent, o.actor.child), (TYPE_WANDERER, None, None));
        // sInitChain: NAVI_ENEMY_STINGER, scale 5 / 1000, lockOnArrowOffset 2500;
        // sColChkInfoInit 2, 45, 15, 100; sColliderCylinderInit 27 by 17, 10 down.
        assert_eq!((o.actor.navi_enemy_id, o.actor.scale, o.actor.target_arrow_offset), (NAVI_ENEMY_STINGER, Vec3::splat(0.005), 2500.0));
        let c = &o.actor.col_chk_info;
        assert_eq!((c.health, c.cyl_radius, c.cyl_height, c.mass), (2, 45, 15, 100));
        assert_eq!((o.collider.dim.radius, o.collider.dim.height, o.collider.dim.y_shift), (27, 17, -10));
        // EnEiyer_SetupAppearFromGround: 40 under its home, a random yaw (shape and world),
        // only the boomerang, bombs and nuts hit it (the fin alone drawn), AT and AC off, no
        // lock-on, ActorShape_Init's circle shadow scaled to 0 and no offset.
        assert_eq!(o.action, Action::AppearFromGround);
        assert_eq!(o.actor.world_pos, Vec3::new(home.x, home.y - 40.0, home.z));
        assert_eq!(o.actor.world_rot.y, o.actor.shape_rot.y);
        assert_eq!(o.collider.info.ac_dmg_info.dmg_flags, 0x19);
        assert!(o.fin_only());
        assert_eq!((o.collider.base.at_flags & AT_ON, o.collider.base.ac_flags & AC_ON), (0, 0));
        assert_eq!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
        assert_ne!(o.actor.flags & ACTOR_FLAG_HOSTILE, 0);
        assert_eq!((o.actor.shape_shadow_draw, o.actor.shape_shadow_scale, o.actor.shape_y_offset), (ShadowDraw::Circle, 0.0, 0.0));
        assert_eq!(draw(&w, h), (vec!["bake/En_Eiyer/fin".to_string()], vec![]));
    }
    // The room's enemies: the two Stingers, the Lizalfos and the Like Like (the Gold Skulltula,
    // En_Sw 0x8301, is an NPC).
    let mut ids: Vec<i16> = w.actors.category(ACTORCAT_ENEMY).iter().map(|&h| w.actors.actor(h).unwrap().id).collect();
    ids.sort();
    assert_eq!(ids, vec![ACTOR_EN_ZF, ACTOR_EN_EIYER, ACTOR_EN_EIYER, ACTOR_EN_RR]);
}

#[test]
fn it_comes_up_to_its_home_then_wanders_backing_about_it() {
    let Some(a) = assets() else { return };
    let mut w = room14(&a);
    let h = at_home(&w, EAST);
    // EnEiyer_AppearFromGround: 0.75 a frame; 40 / 0.75: home on the 54th.
    for n in 1..=53 {
        update(&mut w, h);
        assert_eq!(ei(&w, h).actor.world_pos.y, EAST.y - 40.0 + 0.75 * n as f32, "frame {n}");
        assert_eq!(ei(&w, h).action, Action::AppearFromGround);
    }
    update(&mut w, h);
    // EnEiyer_SetupUnderground: a wanderer backs at 0.5; AC and lock-on on, not always updating.
    let o = ei(&w, h);
    assert_eq!((o.action, o.actor.world_pos.y, o.actor.speed_xz), (Action::WanderUnderground, EAST.y, -0.5));
    assert_ne!(o.collider.base.ac_flags & AC_ON, 0);
    assert_ne!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(o.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    // The focus 12.5 ahead (lock-on on), at its height.
    let r = o.actor.shape_rot.y;
    let p = o.actor.world_pos;
    assert_eq!(o.actor.focus_pos, Vec3::new(p.x + eng_math::sin_s(r) * 12.5, p.y, p.z + eng_math::cos_s(r) * 12.5));
    // Over 100 from its home (150 east): the yaw from it plus 0x8000 (east, 0x4000), turned to at
    // 0xB6 (scaled); shape as world.
    let o = ei_mut(&mut w, h);
    o.actor.world_pos = Vec3::new(EAST.x + 150.0, EAST.y, EAST.z);
    o.actor.world_rot.y = 0;
    update(&mut w, h);
    let o = ei(&w, h);
    let mut expect = 0i16;
    eng_math::scaled_step_to_s(&mut expect, 0x4000, 0xB6);
    assert_eq!((o.target_yaw, o.actor.world_rot.y, o.actor.shape_rot.y), (0x4000, expect, expect));
}

#[test]
fn touching_link_it_jumps_out_and_glides() {
    let Some(a) = assets() else { return };
    let mut w = room14(&a);
    let h = wandering(&mut w);
    let splashes = effects(&w, EFFECT_SS_G_SPLASH);
    let o = ei_mut(&mut w, h);
    o.collider.base.oc_flags2 |= OC2_HIT_PLAYER;
    let at = o.actor.world_pos;
    update(&mut w, h);
    // EnEiyer_SetupAmbush: still, gStingerBackflipAnim, every hit (DMG_DEFAULT), AC off, its
    // shadow 65 and offset 600, NA_SE_EN_OCTAROCK_JUMP, a splash (700).
    let o = ei(&w, h);
    assert_eq!(o.action, Action::Ambush);
    assert!(o.skel.is(ANIM_BACKFLIP));
    assert_eq!(o.collider.base.oc_flags2 & OC2_HIT_PLAYER, 0);
    assert_eq!(o.collider.info.ac_dmg_info.dmg_flags, DMG_DEFAULT);
    assert!(!o.fin_only());
    assert_eq!(o.collider.base.ac_flags & AC_ON, 0);
    assert_ne!(o.actor.flags & ACTOR_FLAG_IGNORE_QUAKE, 0);
    assert_eq!((o.actor.shape_shadow_scale, o.actor.shape_y_offset), (65.0, 600.0));
    assert_eq!(sfx(&w, h, oot_actors::en_okuta::NA_SE_EN_OCTAROCK_JUMP), 1);
    assert_eq!(effects(&w, EFFECT_SS_G_SPLASH), splashes + 1);
    assert_eq!(o.base_pos, at);
    assert_eq!(draw(&w, h), (vec!["bake/En_Eiyer/opa".to_string()], vec![]));
    // EnEiyer_Ambush, frames under 12: the backflip, (1 - cos) × 40 up and sin × 40 back.
    let yaw = o.actor.shape_rot.y;
    loop {
        update(&mut w, h);
        let o = ei(&w, h);
        if o.action != Action::Ambush {
            break;
        }
        let f = o.skel.cur_frame;
        if f < 12.0 {
            let k = ((0.996f32 * std::f32::consts::PI) / 12.0) * f;
            assert_eq!(o.actor.world_pos.y, ((1.0 - k.cos()) * 40.0) + EAST.y, "frame {f}");
            assert_eq!(o.actor.world_pos.x, eng_math::sin_s(yaw) * (k.sin() * -40.0) + at.x, "frame {f}");
        } else {
            assert_eq!(o.actor.speed_xz, 0.8);
        }
    }
    // Done: AC on, gliding (60 frames before it can attack, gStingerHitAnim).
    let o = ei(&w, h);
    assert_eq!((o.action, o.timer), (Action::Glide, 60));
    assert!(o.skel.is(ANIM_HIT));
    assert_ne!(o.collider.base.ac_flags & AC_ON, 0);
}

#[test]
fn over_link_it_somersaults_and_dives_then_burrows() {
    let Some(a) = assets() else { return };
    let mut w = room14(&a);
    let h = gliding(&mut w);
    // Link on the floor under its home.
    let p = w.player.unwrap();
    w.actors.actor_mut(p).unwrap().world_pos = Vec3::new(EAST.x, EAST.y - 10.0, EAST.z);
    // EnEiyer_Glide: the 60 frames not out, or Link not under it within 120: gliding on.
    set_link(&mut w, h, 100.0, -50.0, 0);
    ei_mut(&mut w, h).timer = 5;
    update(&mut w, h);
    assert_eq!(ei(&w, h).action, Action::Glide);
    ei_mut(&mut w, h).timer = 0;
    set_link(&mut w, h, 130.0, -50.0, 0);
    update(&mut w, h);
    assert_eq!(ei(&w, h).action, Action::Glide);
    set_link(&mut w, h, 100.0, 10.0, 0);
    update(&mut w, h);
    assert_eq!(ei(&w, h).action, Action::Glide);
    set_link(&mut w, h, 100.0, -50.0, 0);
    update(&mut w, h);
    assert_eq!(ei(&w, h).action, Action::StartAttack);
    // EnEiyer_StartAttack: a somersault back, 0x1000 a frame (s16: -0x8000 less 0x1000 is
    // 0x7000), world.rot.x minus shape.rot.x.
    let mut x = ei(&w, h).actor.shape_rot.x;
    assert_eq!(x, 0);
    for _ in 0..9 {
        update(&mut w, h);
        x = x.wrapping_sub(0x1000);
        let o = ei(&w, h);
        assert_eq!((o.actor.shape_rot.x, o.actor.world_rot.x), (x, x.wrapping_neg()));
    }
    assert_eq!(x, 0x7000);
    // Then down at Link's waist (0x1000 steps); aimed, the dive: AT on.
    until_not(&mut w, h, Action::StartAttack, 20);
    let o = ei(&w, h);
    assert_eq!(o.action, Action::DiveAttack);
    assert_ne!(o.collider.base.at_flags & AT_ON, 0);
    // EnEiyer_DiveAttack: 1.1 times faster a frame; a hit on Link turns AT off.
    let s = o.actor.speed_xz;
    let o = ei_mut(&mut w, h);
    o.collider.base.at_flags |= AT_HIT;
    o.actor.bg_check_flags = 0;
    update(&mut w, h);
    let o = ei(&w, h);
    assert_eq!(o.actor.speed_xz, s * 1.1);
    assert_eq!(o.collider.base.at_flags & (AT_ON | AT_HIT), 0);
    // The ground: it lands (gStingerDiveAnim, AT off, always updating, the check to come).
    ei_mut(&mut w, h).actor.bg_check_flags = BGCHECKFLAG_GROUND;
    let o = ei_mut(&mut w, h);
    o.collider.base.at_flags |= AT_ON;
    update(&mut w, h);
    let o = ei(&w, h);
    assert_eq!((o.action, o.timer), (Action::Land, -1));
    assert!(o.skel.is(ANIM_DIVE));
    assert_eq!(o.collider.base.at_flags & AT_ON, 0);
    assert_ne!(o.actor.flags & ACTOR_FLAG_UPDATE_CULLING_DISABLED, 0);
    // EnEiyer_Land: on the ground, NA_SE_EN_OCTAROCK_SINK where it is and a splash; ten more
    // frames, then back under (40 under its home, unpitched).
    let (sinks, splashes) = (sfx_any(&w, oot_actors::en_okuta::NA_SE_EN_OCTAROCK_SINK), effects(&w, EFFECT_SS_G_SPLASH));
    ei_mut(&mut w, h).actor.bg_check_flags = BGCHECKFLAG_GROUND;
    update(&mut w, h);
    assert_eq!(ei(&w, h).timer, 10);
    assert_eq!(sfx_any(&w, oot_actors::en_okuta::NA_SE_EN_OCTAROCK_SINK), sinks + 1);
    assert_eq!(effects(&w, EFFECT_SS_G_SPLASH), splashes + 1);
    for _ in 0..9 {
        update(&mut w, h);
        assert_eq!(ei(&w, h).action, Action::Land);
    }
    update(&mut w, h);
    let o = ei(&w, h);
    assert_eq!(o.action, Action::AppearFromGround);
    assert_eq!((o.actor.world_pos.y, o.actor.shape_rot.x, o.actor.world_rot.x), (EAST.y - 40.0, 0, 0));
    assert!(o.fin_only());
}

#[test]
fn out_of_the_ground_nuts_stun_it_and_swords_hurt_then_kill_it() {
    let Some(a) = assets() else { return };
    let mut w = room14(&a);
    let h = gliding(&mut w);
    // A Deku nut (damage 0, reaction 1): stunned (blue 80 frames, falling, 8 taller,
    // NA_SE_EN_GOMA_JR_FREEZE), no damage.
    hit(&mut w, h, 0, 1);
    update(&mut w, h);
    let o = ei(&w, h);
    assert_eq!((o.action, o.timer, o.actor.speed_xz, o.actor.gravity), (Action::Stunned, 79, 0.0, -1.0));
    assert_eq!((o.collider.dim.height, o.actor.col_chk_info.health), (25, 2));
    assert!(o.skel.is(ANIM_POP_OUT));
    assert_eq!(sfx(&w, h, NA_SE_EN_GOMA_JR_FREEZE), 1);
    assert_eq!(o.actor.color_filter_params & COLORFILTER_COLORFLAG_RED, 0);
    // Stunned again, nothing new.
    hit(&mut w, h, 0, 1);
    update(&mut w, h);
    assert_eq!((ei(&w, h).timer, sfx(&w, h, NA_SE_EN_GOMA_JR_FREEZE)), (78, 1));
    // The 80 frames out: gliding, its height back.
    ei_mut(&mut w, h).timer = 1;
    update(&mut w, h);
    let o = ei(&w, h);
    assert_eq!((o.action, o.actor.gravity, o.collider.dim.height), (Action::Glide, 0.0, 17));
    // The Kokiri sword (1): hurt (NA_SE_EN_EIER_DAMAGE, red 40 frames, AC off, backing off at 5).
    hit(&mut w, h, 1, 0);
    update(&mut w, h);
    let o = ei(&w, h);
    assert_eq!((o.action, o.timer, o.actor.speed_xz, o.actor.col_chk_info.health), (Action::Hurt, 39, 5.0, 1));
    assert_eq!(o.collider.base.ac_flags & AC_ON, 0);
    assert_ne!(o.actor.color_filter_params & COLORFILTER_COLORFLAG_RED, 0);
    assert_eq!(sfx(&w, h, NA_SE_EN_EIER_DAMAGE), 1);
    // Hurt out: AC on, gliding.
    ei_mut(&mut w, h).timer = 1;
    update(&mut w, h);
    let o = ei(&w, h);
    assert_eq!((o.action, o.actor.shape_rot.z), (Action::Glide, 0));
    assert_ne!(o.collider.base.ac_flags & AC_ON, 0);
    // The Master sword (2): no health left, it dies (the finishing blow, NA_SE_EN_EIER_DEAD, no
    // lock-on, at 6 forward, 20 frames, AT and AC off).
    hit(&mut w, h, 2, 0);
    update(&mut w, h);
    let o = ei(&w, h);
    assert_eq!((o.action, o.timer, o.actor.speed_xz, o.actor.col_chk_info.health), (Action::Die, 19, 6.0, 0));
    assert_eq!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!((o.collider.base.at_flags & AT_ON, o.collider.base.ac_flags & AC_ON), (0, 0));
    assert_eq!(sfx(&w, h, NA_SE_EN_EIER_DEAD), 1);
}

#[test]
fn under_the_ground_the_boomerang_brings_it_out_and_a_bomb_kills_it() {
    let Some(a) = assets() else { return };
    let mut w = room14(&a);
    let h = wandering(&mut w);
    // The boomerang (damage 0, reaction 1), under the ground: it jumps out.
    hit(&mut w, h, 0, 1);
    update(&mut w, h);
    assert_eq!((ei(&w, h).action, ei(&w, h).actor.col_chk_info.health), (Action::Ambush, 2));
    // The west one, wandering: a bomb (2) kills it outright, backwards (its -0.5 less 6), its
    // animation as it was.
    let h = at_home(&w, WEST);
    until_not(&mut w, h, Action::AppearFromGround, 60);
    hit(&mut w, h, 2, 0);
    update(&mut w, h);
    let o = ei(&w, h);
    assert_eq!((o.action, o.actor.speed_xz, o.actor.col_chk_info.health), (Action::Die, -6.5, 0));
    assert!(o.skel.is(ANIM_IDLE));
    assert_eq!(o.collider.info.ac_dmg_info.dmg_flags, DMG_DEFAULT);
    assert_eq!(sfx(&w, h, NA_SE_EN_EIER_DEAD), 1);
}

#[test]
fn dead_they_fade_and_the_last_one_gone_clears_room_14() {
    let Some(a) = assets() else { return };
    let mut w = room14(&a);
    // The Lizalfos and the Like Like gone first.
    for h in w.actors.category(ACTORCAT_ENEMY).to_vec() {
        let a = w.actors.actor_mut(h).unwrap();
        if a.id != ACTOR_EN_EIYER {
            a.kill();
        }
    }
    idle(&mut w, 1);
    assert_eq!(w.actors.category(ACTORCAT_ENEMY).len(), 2);
    let room_bit = 1u32 << ROOM;
    assert_eq!(w.flags.temp_clear & room_bit, 0);
    for (n, home) in [EAST, WEST].into_iter().enumerate() {
        let h = at_home(&w, home);
        until_not(&mut w, h, Action::AppearFromGround, 60);
        ei_mut(&mut w, h).setup_die();
        // EnEiyer_Die: 20 frames (or a ceiling), then dead.
        let n_die = until_not(&mut w, h, Action::Die, 20);
        assert!(n_die <= 20);
        let o = ei(&w, h);
        assert_eq!((o.action, o.actor.speed_xz), (Action::Dead, 0.0));
        assert_ne!(o.actor.color_filter_params & 0x2000, 0);
        // EnEiyer_Dead: translucent at its shadow's alpha, 5 less and 2 lower a frame.
        let (alpha, y) = (o.actor.shape_shadow_alpha, o.actor.world_pos.y);
        update(&mut w, h);
        let o = ei(&w, h);
        assert_eq!((o.actor.shape_shadow_alpha, o.actor.world_pos.y), (alpha - 5, y - 2.0));
        assert_eq!(draw(&w, h), (vec![], vec!["bake/En_Eiyer/xlu".to_string()]));
        // 255 / 5: gone on the 51st (a drop from table 5), the enemies' list left at the frame's end.
        let frames = until_not(&mut w, h, Action::Dead, 60);
        assert_eq!(frames, (alpha as usize - 5) / 5);
        assert!(gone(&w, h));
        idle(&mut w, 1);
        // Flags_SetTempClear with the room's last enemy deleted.
        assert_eq!(w.flags.temp_clear & room_bit != 0, n == 1, "after Stinger {n}");
    }
}

#[test]
fn a_leader_spawns_three_clones_that_circle_with_it() {
    let Some(a) = assets() else { return };
    let mut w = room14(&a);
    let enemies = w.actors.category(ACTORCAT_ENEMY).len();
    let home = Vec3::new(1000.0, -1293.0, -2450.0);
    let leader = w.actor_spawn(ACTOR_EN_EIYER, home, [0, 0x1000, 0], TYPE_LEADER).expect("the leader");
    // EnEiyer_Init: each of params under 3 spawns the next as its child, at the leader's home,
    // turned a quarter on; the leader takes all three as its own.
    let clones = clones(&w, leader);
    assert_eq!(clones.len(), 3);
    assert_eq!(w.actors.category(ACTORCAT_ENEMY).len(), enemies + 4);
    for (i, &c) in clones.iter().enumerate() {
        let o = ei(&w, c);
        assert_eq!((o.actor.params, o.actor.parent, o.actor.home_pos), (i as i16 + 1, Some(leader), home));
        // EnEiyer_SetupInactive: no lock-on; world yaw as the shape's (the spawn's 0x1000 +
        // 0x4000 per clone).
        assert_eq!(o.action, Action::Inactive);
        assert_eq!(o.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
        assert_eq!(o.actor.world_rot.y, (0x1000 + 0x4000 * (i as i32 + 1)) as i16);
    }
    // The leader comes up 80 out round its home at a random yaw, faced along the circle.
    let o = ei(&w, leader);
    assert_eq!(o.action, Action::AppearFromGround);
    let yaw = o.actor.world_rot.y;
    let ring = |yaw: i16| Vec3::new(eng_math::sin_s(yaw) * 80.0 + home.x, home.y - 40.0, eng_math::cos_s(yaw) * 80.0 + home.z);
    assert_eq!((o.actor.world_pos, o.actor.shape_rot.y), (ring(yaw), yaw.wrapping_add(0x4000)));
    // A clone's update with its leader coming up: it comes up too, a quarter turn on per number.
    for (i, &c) in clones.iter().enumerate() {
        update(&mut w, c);
        let o = ei(&w, c);
        let y = yaw.wrapping_add((0x4000 * (i as i32 + 1)) as i16);
        assert_eq!((o.action, o.actor.world_rot.y), (Action::AppearFromGround, y));
        assert_eq!(o.actor.world_pos, ring(y));
        assert!(o.fin_only());
    }
    // All up: circling, 0x60 a frame.
    for h in std::iter::once(leader).chain(clones.iter().copied()) {
        until_not(&mut w, h, Action::AppearFromGround, 60);
        assert_eq!(ei(&w, h).action, Action::CircleUnderground);
    }
    let y0 = ei(&w, leader).actor.world_rot.y;
    update(&mut w, leader);
    let o = ei(&w, leader);
    assert_eq!(o.actor.world_rot.y, y0.wrapping_sub(0x60));
    assert_eq!(o.actor.shape_rot.y, y0.wrapping_sub(0x60).wrapping_add(0x4000));
    // The clones never update their colliders (only the leader's and a wanderer's).
    update(&mut w, clones[0]);
    assert_eq!(ei(&w, clones[0]).collider.dim.pos, [0; 3]);
    assert_ne!(ei(&w, leader).collider.dim.pos, [0; 3]);
    // The leader jumps out: the clones wait (sinking 0.5 a frame to 50 under the home).
    act(&mut w, leader, |o, w| o.setup_ambush(w));
    for &c in &clones {
        update(&mut w, c);
        assert_eq!(ei(&w, c).action, Action::Inactive);
        let y = ei(&w, c).actor.world_pos.y;
        update(&mut w, c);
        assert_eq!(ei(&w, c).actor.world_pos.y, y - 0.5);
    }
    // Back under: they come up with it.
    act(&mut w, leader, |o, w| o.setup_appear_from_ground(w));
    update(&mut w, clones[1]);
    assert_eq!(ei(&w, clones[1]).action, Action::AppearFromGround);
    // Dead: a waiting clone is gone.
    ei_mut(&mut w, leader).setup_dead();
    ei_mut(&mut w, clones[0]).setup_inactive();
    update(&mut w, clones[0]);
    assert!(gone(&w, clones[0]));
}
