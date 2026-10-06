//! `Bg_Ydan_Maruta` (`z_bg_ydan_maruta.c`) against the C, in the Deku Tree (MQ):
//! - room 5's spiked log `0x00FF` at (-835, -860, 1050) facing +x (rot y 0x4000), over the pool
//!   the floating block (`Bg_Ydan_Hasi` 0xFF00, at (-835, -905, 1050), sliding ±165 along x)
//!   carries Link across;
//! - room 2's ladder `0x0121` at (-1066, 560, 1066), rot y -0x2000, on switch flag 0x21.
//!
//! The ladder takes only a seed (`DMG_SLINGSHOT`), which Link can't shoot yet (milestone 5): its
//! hit is injected as `CollisionCheck_SetATvsAC` leaves it, as tests/switches.rs does the eye's.
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_B, PadState};
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::bg_ydan_hasi::{self, BgYdanHasi};
use oot_actors::bg_ydan_maruta::{self, Action, BgYdanMaruta, shake_offset};
use oot_actors::player::Action as PlayerAction;
use oot_actors::playthrough::{DEKU_TREE_ROOM_STARTS, deku_tree_room_start};
use oot_game::actor::{ACTOR_AUDIO_FLAG_SFX_CENTERED_1, ACTOR_AUDIO_FLAG_SFX_CENTERED_2, ACTOR_FLAG_SFX_ACTOR_POS_2, ACTOR_FLAG_SFX_TIMER};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::*;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check as cc;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Inside the Deku Tree (`deku-tree-inside`), the sound log on, 60 frames in, then at `room`'s
/// debug start (`DEKU_TREE_ROOM_STARTS`), the room's enemies gone (the props are what's tested)
/// and the attention cameras its clear brings waited out.
fn deku_tree(a: &Arc<GameAssets>, room: i8) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 60);
    let &(r, pos, yaw, _) = DEKU_TREE_ROOM_STARTS.iter().find(|s| s.0 == room).expect("a debug start");
    deku_tree_room_start(&mut w, r, pos, yaw);
    for h in w.actors.category(oot_game::actor_ctx::ACTORCAT_ENEMY).to_vec() {
        if let Some(a) = w.actors.actor_mut(h) {
            a.kill();
        }
    }
    idle(&mut w, 1);
    for _ in 0..600 {
        if w.active_cam_id == CAM_ID_MAIN && w.sub_cameras.iter().flatten().next().is_none() {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
    w
}

/// The `Bg_Ydan_Maruta` of kind `kind` (its params after the init).
fn maruta_of(w: &PlayState, kind: i16) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<BgYdanMaruta>(h).is_some_and(|p| p.actor.params == kind)).unwrap_or_else(|| panic!("no Bg_Ydan_Maruta of kind {kind}"))
}

fn maruta(w: &PlayState, h: ActorHandle) -> &BgYdanMaruta {
    w.actors.downcast::<BgYdanMaruta>(h).expect("Bg_Ydan_Maruta")
}

fn maruta_mut(w: &mut PlayState, h: ActorHandle) -> &mut BgYdanMaruta {
    w.actors.downcast_mut::<BgYdanMaruta>(h).expect("Bg_Ydan_Maruta")
}

/// The actor's sound this frame by `Actor_PlaySfx_Flagged` (at the actor, none of the other
/// flags).
fn flagged(a: &oot_game::actor::Actor) -> u16 {
    assert_eq!(a.flags & (ACTOR_FLAG_SFX_ACTOR_POS_2 | ACTOR_AUDIO_FLAG_SFX_CENTERED_1 | ACTOR_AUDIO_FLAG_SFX_CENTERED_2 | ACTOR_FLAG_SFX_TIMER), 0);
    a.sfx
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// `BgYdanMaruta_Init`'s triangles for an element `v` at `pos` turned by `yaw`: each vertex
/// `(v.x cosS + pos.x, v.y + pos.y, pos.z - v.x sinS)`; the second triangle is vertices 0 and 2
/// and a fourth, `(v[2].x .., v[0].y ..)`.
fn tris(v: [Vec3; 3], pos: Vec3, yaw: i16) -> [[Vec3; 3]; 2] {
    let (s, c) = (sin_s(yaw), cos_s(yaw));
    let at = |x: f32, y: f32| Vec3::new((x * c) + pos.x, y + pos.y, pos.z - (x * s));
    let p = [at(v[0].x, v[0].y), at(v[1].x, v[1].y), at(v[2].x, v[2].y)];
    [p, [p[0], p[2], at(v[2].x, v[0].y)]]
}

fn tri_vertices(m: &BgYdanMaruta) -> [[Vec3; 3]; 2] {
    [m.collider.elements[0].dim.vtx, m.collider.elements[1].dim.vtx]
}

const LOG_HOME: Vec3 = Vec3::new(-835.0, -860.0, 1050.0);
const LADDER_HOME: Vec3 = Vec3::new(-1066.0, 560.0, 1066.0);

/// Room 5's log: kind 0 (`0x00FF >> 8`), flag 0xFF, scale 0.1, no bg actor. Its triangles from
/// `sTrisElementsInit[0]` ((220, -10), (220, 10), (-220, 10)) turned by 0x4000 (sinS 1, cosS 0):
/// an upright 440 along z by 20, across x = -835. Each frame `shape.rot.x` up 0x360, in place,
/// `NA_SE_EV_TOGE_STICK_ROLLING - SFX_FLAG` flagged, its AT registered (`AT_ON`, no hit: Link's
/// on the bank).
#[test]
fn room_5s_log_spins_in_place() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 5);
    let h = maruta_of(&w, bg_ydan_maruta::MARUTA_LOG);
    let m = maruta(&w, h);
    assert_eq!((m.switch_flag, m.bg, m.actor.scale), (0xFF, eng_collision::dyna::BG_ACTOR_MAX, Vec3::splat(0.1)));
    assert_eq!((m.actor.home_pos, m.actor.shape_rot.y), (LOG_HOME, 0x4000));
    let want = tris(bg_ydan_maruta::tris_elements_init()[0].vtx, LOG_HOME, 0x4000);
    assert_eq!(want[0], [Vec3::new(-835.0, -870.0, 830.0), Vec3::new(-835.0, -850.0, 830.0), Vec3::new(-835.0, -850.0, 1270.0)]);
    assert_eq!(want[1], [Vec3::new(-835.0, -870.0, 830.0), Vec3::new(-835.0, -850.0, 1270.0), Vec3::new(-835.0, -870.0, 1270.0)]);
    assert_eq!(tri_vertices(m), want);
    let mut rot_x = m.actor.shape_rot.x;
    for k in 1..=200 {
        idle(&mut w, 1);
        rot_x = rot_x.wrapping_add(0x360);
        let m = maruta(&w, h);
        assert_eq!((m.action, m.actor.shape_rot.x, m.actor.world_pos), (Action::Spin, rot_x, LOG_HOME), "frame {k}");
        assert_eq!(flagged(&m.actor), bg_ydan_maruta::NA_SE_EV_TOGE_STICK_ROLLING - SFX_FLAG, "frame {k}");
        assert_eq!(m.collider.base.at_flags, cc::AT_ON | cc::AT_TYPE_ENEMY, "frame {k}");
    }
    assert!(!sfx_frames(&w, bg_ydan_maruta::NA_SE_EV_TOGE_STICK_ROLLING - SFX_FLAG).is_empty());
}

/// Link from room 5's debug start onto the floating block at the west end of its slide
/// (`gameplayFrames & 0xFF` 192: -835 - 165), facing +x. The block carries him east into the
/// log's triangles. The frame after his cylinder first reaches them (the AT is checked at the
/// start of the next frame, `CollisionCheck_AT` before `Actor_UpdateAll`), Player takes the
/// element's damage 4 (`CollisionCheck_ApplyDamage`: no damage table, defense 0) and staggers
/// (`func_80837C0C` with no hit response, standing still: `Player_Action_8084370C`,
/// `invincibilityTimer` 20); then the log, a PROP after Player, sees `AT_HIT` and asks for the
/// knockdown (`Actor_SetPlayerKnockbackLargeNoDamage(7, 0x4000, 6)`: type 2, damage 0). The next
/// frame Player takes it (`func_808382DC`: a large knockback whatever the timer): knocked down
/// (`Player_Action_8084377C`) along 0x4000 (`yaw`, `world.rot.y`), facing it still (the turn
/// `0x4000 - shape.rot.y` is 0), no further damage. While he's invincible his AC isn't set, so
/// the log doesn't hit him again.
#[test]
fn the_log_knocks_link_down() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 5);
    assert_eq!(w.room_ctx.cur.num, 5);
    let log = maruta_of(&w, bg_ydan_maruta::MARUTA_LOG);
    let block = w.actors.all().into_iter().find(|&h| w.actors.downcast::<BgYdanHasi>(h).is_some_and(|p| p.actor.params == bg_ydan_hasi::HASI_WATER_BLOCK)).unwrap();
    while w.gameplay_frames & 0xFF != 192 {
        idle(&mut w, 1);
    }
    let b = w.actors.downcast::<BgYdanHasi>(block).unwrap().actor.world_pos;
    assert!((b.x - -1000.0).abs() < 1e-3, "{b}");
    w.place_player(Vec3::new(b.x, b.y as i16 as f32, b.z), 0x4000);
    let health = w.save.health;
    let radius = w.player().cylinder.dim.radius as f32;
    // The cylinder's centre is Link's position truncated to an s16 (Collider_UpdateCylinder).
    let cyl_x = |w: &PlayState| w.player().cylinder.dim.pos[0] as f32;
    let (mut before, mut prev) = (cyl_x(&w), cyl_x(&w));
    let mut n = 0;
    while w.save.health == health {
        (before, prev) = (prev, cyl_x(&w));
        idle(&mut w, 1);
        n += 1;
        assert!(n < 100, "the log never hit");
        if w.save.health == health {
            assert_eq!(w.player().action, PlayerAction::StandingStill, "frame {n}");
        }
    }
    // Carried there: his cylinder (radius 12) reached the triangles' plane, x -835, in the frame
    // before (its AT and his AC set then), and not the frame before that.
    assert_eq!(radius, 12.0);
    assert!(prev + radius >= -835.0 && before + radius < -835.0, "{before} {prev}");
    let p = w.player();
    assert_eq!(w.save.health, health - 4);
    assert_eq!((p.action, p.invincibility_timer), (PlayerAction::Damaged, 20));
    assert_eq!((p.knockback_type, p.knockback_rot, p.knockback_speed, p.knockback_y_velocity, p.knockback_damage), (2, 0x4000, 7.0, 6.0, 0));
    // The log's AT was set again this frame (AT_HIT cleared by CollisionCheck_SetAT).
    assert_eq!(maruta(&w, log).collider.base.at_flags & cc::AT_HIT, 0);
    idle(&mut w, 1);
    let p = w.player();
    assert_eq!(p.action, PlayerAction::KnockedDown);
    assert_eq!((p.knockback_type, p.current_yaw, p.actor.world_rot.y, p.actor.shape_rot.y), (0, 0x4000, 0x4000, 0x4000));
    assert_eq!(w.save.health, health - 4);
    // Invincible: hit no more.
    for k in 0..15 {
        idle(&mut w, 1);
        assert_eq!(maruta(&w, log).collider.base.at_flags & cc::AT_HIT, 0, "frame {k}");
        assert_eq!(w.save.health, health - 4, "frame {k}");
    }
}

/// Room 2's ladder: kind 1, flag 0x21; its bg actor (`gDTFallingLadderCol`, no carrying flags)
/// where it's placed; home 280 down (280); waiting for a seed (`func_808BF078`), its AC set each
/// frame. Its triangles from `sTrisElementsInit[1]` ((16, 0), (16, 135), (-16, 135)) turned by
/// -0x2000 at 560. With 0x21 set, one spawned there is down already (`world.pos.y = home.pos.y`,
/// 280, its triangles there) and does nothing.
#[test]
fn room_2s_ladder_waits_raised_or_is_down_with_its_flag() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 2);
    let h = maruta_of(&w, bg_ydan_maruta::MARUTA_LADDER);
    let m = maruta(&w, h);
    assert_eq!((m.switch_flag, m.action, m.actor.world_pos, m.actor.home_pos), (0x21, Action::WaitForHit, LADDER_HOME, Vec3::new(-1066.0, 280.0, 1066.0)));
    assert!(w.col.dyna.is_bg_actor(m.bg));
    let bga = &w.col.dyna.actors[m.bg as usize];
    assert_eq!((bga.move_flags, bga.source.pos), (0, LADDER_HOME));
    assert_eq!(tri_vertices(m), tris(bg_ydan_maruta::tris_elements_init()[1].vtx, LADDER_HOME, -0x2000));
    idle(&mut w, 5);
    let m = maruta(&w, h);
    assert_eq!((m.action, m.actor.world_pos), (Action::WaitForHit, LADDER_HOME));
    assert_eq!(m.collider.base.ac_flags, cc::AC_ON | cc::AC_TYPE_PLAYER);

    w.flags.set_switch(0x21);
    let n = w.actor_spawn(bg_ydan_maruta::ACTOR_BG_YDAN_MARUTA, LADDER_HOME, [0, -0x2000, 0], 0x0121).expect("spawn");
    let down = Vec3::new(-1066.0, 280.0, 1066.0);
    let m = maruta(&w, n);
    assert_eq!((m.action, m.actor.world_pos, m.actor.home_pos), (Action::DoNothing, down, down));
    assert_eq!(tri_vertices(m), tris(bg_ydan_maruta::tris_elements_init()[1].vtx, down, -0x2000));
    idle(&mut w, 3);
    assert_eq!((maruta(&w, n).action, maruta(&w, n).actor.world_pos), (Action::DoNothing, down));
}

/// Injects a seed's hit on the ladder (`CollisionCheck_SetATvsAC`'s marks: `AC_HIT`, the
/// colliding actor, a slingshot seed's element).
fn shoot(w: &mut PlayState, h: ActorHandle, by: ActorHandle) {
    let c = &mut maruta_mut(w, h).collider;
    c.base.ac_flags |= cc::AC_HIT;
    c.base.ac = Some(by);
    c.elements[0].info.ac_elem_flags |= cc::ACELEM_HIT;
    c.elements[0].info.ac_hit_elem = Some(cc::HitElem {
        elem: cc::ElemRef { col: cc::ColliderRef { actor: by, id: 0 }, elem: 0 },
        at_dmg_info: cc::ColliderElementDamageInfoAT { dmg_flags: cc::DMG_SLINGSHOT, hit_special_effect: cc::HIT_SPECIAL_EFFECT_NONE, damage: 1 },
        ac_dmg_info: cc::ColliderElementDamageInfoAC { dmg_flags: 0, hit_backlash: 0, defense: 0, hit_pos: [0; 3] },
        elem_material: cc::ELEM_MATERIAL_UNK0,
    });
}

/// A seed's hit: that frame (`func_808BF078`) flag 0x21, `NA_SE_SY_CORRECT_CHIME`, one-point
/// cutscene 3010 on it for 50 frames, `unk_16A` 20, not moved. Then 20 frames of shaking
/// (`func_808BF108`): the count down, the ladder `(unk_16A % 4) - 2` (-2 made 0, else doubled)
/// across its face (`x = cosS(-0x2000) t + home.x`, `z = sinS(-0x2000) t + home.z`):
/// 2, 0, -2, 0, ... for counts 19 down to 0, `NA_SE_EV_TRAP_OBJ_SLIDE` flagged; at 0 the fall.
/// Then `func_808BF1EC`: the speed up 1 a frame (1, 2, 3 ...), the ladder that much nearer home:
/// 1 + 2 + ... + 23 is 276 of the 280, so the 24th frame lands it, with
/// `NA_SE_EV_LADDER_DOUND`; the collision with it each frame; its triangles stay up.
#[test]
fn a_seed_drops_room_2s_ladder() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a, 2);
    let h = maruta_of(&w, bg_ydan_maruta::MARUTA_LADDER);
    let tris_before = tri_vertices(maruta(&w, h));
    let player = w.player.unwrap();
    assert!(!w.flags.get_switch(0x21));
    shoot(&mut w, h, player);
    idle(&mut w, 1);
    let hit = w.audio.frames;
    let m = maruta(&w, h);
    assert_eq!((m.action, m.unk_16a, m.actor.world_pos), (Action::Shake, 20, LADDER_HOME));
    assert!(w.flags.get_switch(0x21));
    assert_eq!(sfx_frames(&w, NA_SE_SY_CORRECT_CHIME), vec![hit]);
    let cam = w.sub_cameras.iter().flatten().find(|c| c.cs_id == 3010).expect("one-point cutscene 3010");
    assert_eq!(cam.target, Some(h));

    let home = LADDER_HOME;
    let mut offsets = Vec::new();
    for k in 1..=20i16 {
        idle(&mut w, 1);
        let count = 20 - k;
        let t = shake_offset(count);
        offsets.push(t);
        let m = maruta(&w, h);
        assert_eq!(m.unk_16a, count, "shake {k}");
        assert_eq!(m.action, if count == 0 { Action::Fall } else { Action::Shake }, "shake {k}");
        let want = Vec3::new((cos_s(-0x2000) * t as f32) + home.x, home.y, (sin_s(-0x2000) * t as f32) + home.z);
        assert_eq!(m.actor.world_pos, want, "shake {k}");
        assert_eq!(flagged(&m.actor), bg_ydan_maruta::NA_SE_EV_TRAP_OBJ_SLIDE - SFX_FLAG, "shake {k}");
        assert_eq!(w.col.dyna.actors[m.bg as usize].source.pos, want, "shake {k}");
    }
    assert_eq!(offsets, [2, 0, -2, 0, 2, 0, -2, 0, 2, 0, -2, 0, 2, 0, -2, 0, 2, 0, -2, 0]);
    let mut y = home.y;
    for k in 1..=24 {
        idle(&mut w, 1);
        let m = maruta(&w, h);
        y = (y - k as f32).max(280.0);
        assert_eq!(m.actor.velocity.y, k as f32, "fall {k}");
        assert_eq!(m.actor.world_pos, Vec3::new(home.x, y, home.z), "fall {k}");
        assert_eq!(m.action, if k == 24 { Action::DoNothing } else { Action::Fall }, "fall {k}");
        assert_eq!(w.col.dyna.actors[m.bg as usize].source.pos.y, y, "fall {k}");
        if k == 23 {
            assert_eq!(y, 560.0 - 276.0);
        }
    }
    assert_eq!(sfx_frames(&w, bg_ydan_maruta::NA_SE_EV_LADDER_DOUND), vec![w.audio.frames]);
    idle(&mut w, 5);
    let m = maruta(&w, h);
    assert_eq!((m.action, m.actor.world_pos.y), (Action::DoNothing, 280.0));
    assert_eq!(tri_vertices(m), tris_before);
    assert_eq!(sfx_frames(&w, bg_ydan_maruta::NA_SE_EV_LADDER_DOUND).len(), 1);
}

/// A ladder spawned on room 2's floor at Link's height (flag 0x2A, `0x012A`), Link slashing it
/// with the Kokiri Sword from in front: its triangles take only `DMG_SLINGSHOT` (4), the sword
/// is `DMG_SLASH_KOKIRI`, so no hit (`CollisionCheck_NoSharedFlags`) and it waits on. With its
/// element made to take the sword too, the same slash does hit it (the geometry reaches): it's
/// the flags that keep the sword out.
#[test]
fn a_sword_slash_doesnt_drop_the_ladder() {
    let Some(a) = assets() else { return };
    for takes_sword in [false, true] {
        let mut w = deku_tree(&a, 2);
        let at = Vec3::new(-1150.0, 280.0, 1150.0);
        assert_eq!(w.col.entity_raycast_down(at + Vec3::Y * 50.0).0, 280.0);
        assert!(!w.flags.get_switch(0x2A));
        let h = w.actor_spawn(bg_ydan_maruta::ACTOR_BG_YDAN_MARUTA, at, [0, 0, 0], 0x012A).expect("spawn");
        if takes_sword {
            for e in &mut maruta_mut(&mut w, h).collider.elements {
                e.info.ac_dmg_info.dmg_flags |= cc::DMG_SLASH_KOKIRI;
            }
        }
        // In front of it (its face is across z), facing it (-z: yaw 0x8000).
        w.place_player(Vec3::new(at.x, 280.0, at.z + 35.0), -0x8000);
        idle(&mut w, 2);
        let mut f = vec![with(stick(0, 0), BTN_B)];
        f.extend(repeat(stick(0, 0), 30));
        let (mut hit_at, mut swung) = (None, false);
        let mut prev = PadState::default();
        for (k, &p) in f.iter().enumerate() {
            w.tick_with(scripted_input(prev, p));
            prev = p;
            swung |= w.player().melee_weapon_state != 0;
            if hit_at.is_none() && maruta(&w, h).action != Action::WaitForHit {
                hit_at = Some(k);
            }
        }
        // The slash's AT was on (`meleeWeaponState`).
        assert!(swung);
        if takes_sword {
            assert!(hit_at.is_some(), "the slash reaches the triangles");
            assert!(w.flags.get_switch(0x2A));
        } else {
            assert_eq!(hit_at, None);
            assert_eq!((maruta(&w, h).action, maruta(&w, h).actor.world_pos), (Action::WaitForHit, at));
            assert!(!w.flags.get_switch(0x2A));
        }
    }
}
