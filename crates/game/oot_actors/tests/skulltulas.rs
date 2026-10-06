//! The Deku Tree's Skulltulas (GAME-05 milestone 3b) against the C: `En_Sw` (`z_en_sw.c`), the
//! Skullwalltula and the Gold Skulltula, and `En_Si` (`z_en_si.c`), the token, in the Deku
//! Tree's first room (room 0): the Skullwalltula on the vines at (95, 133, -308) (params 0, yaw
//! -2913) and the Gold Skulltula at (278, 360, 332) (params 0x8102), as the MQ scene places them.
//!
//! The Gold Skulltula is shut in room 0's large crate (`Obj_Kibako2`): its test breaks the crate
//! with an injected explosion first, as a bomb would.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_B, PadState};
use eng_math::{atan2_s, cos_s, sin_s, vec3f_yaw};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_si::{Action as TokenAction, EnSi};
use oot_actors::en_sw::{self, Action, EnSw};
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_NPC, ActorHandle};
use oot_game::audio::sfx::*;
use oot_game::effect::{EFFECT_SS_DEAD_DB, EFFECT_SS_DUST};
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

fn tick(w: &mut PlayState, p: PadState, prev: &mut PadState) {
    w.tick_with(scripted_input(*prev, p));
    *prev = p;
}

/// Play_Init inside the Deku Tree (`deku-tree-inside`: the Kokiri Sword and the Deku Shield), the
/// sound log on; no frame run.
fn deku_tree_init(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init")
}

/// `deku_tree_init`, the entrance's walk in over. Navi's hints by the vines and the Gold
/// Skulltula's ledge (room 0's three `Elf_Msg` 0x1F02) as if heard: their flag 0x1F set, so they
/// go (`ElfMsg_KillCheck`) instead of stopping Link for Navi's forced text.
fn deku_tree(a: &Arc<GameAssets>) -> PlayState {
    let mut w = deku_tree_init(a);
    w.flags.set_switch(0x1F);
    idle(&mut w, 60);
    w
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

/// The effects of type `ty` spawned this frame: `EffectSs_UpdateAll` takes a frame off a new
/// one's `life` after the actors.
fn new_effects(w: &PlayState, ty: u8, life: i16) -> usize {
    w.effect_ss.table.iter().filter(|e| e.ty == ty && e.life == life - 1).count()
}

/// The Skullwalltula's placement (`En_Sw` params 0, rot (0x4000, -2913, 0)).
const SW_PLACED: Vec3 = Vec3::new(95.0, 133.0, -308.0);
const SW_YAW: i16 = -2913;
/// The Gold Skulltula's (`En_Sw` params 0x8102, rot (0, -24576, 0)).
const GOLD_PLACED: Vec3 = Vec3::new(278.0, 360.0, 332.0);

/// The `En_Sw` whose home is nearest `pos`.
fn sw_near(w: &PlayState, pos: Vec3) -> Option<ActorHandle> {
    w.actors
        .all()
        .into_iter()
        .filter(|&h| w.actors.downcast::<EnSw>(h).is_some_and(|s| s.actor.home_pos.distance(pos) < 30.0))
        .min_by_key(|&h| w.actors.actor(h).unwrap().home_pos.distance(pos) as i32)
}

fn sw(w: &PlayState, h: ActorHandle) -> &EnSw {
    w.actors.downcast::<EnSw>(h).expect("En_Sw")
}

fn sw_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnSw {
    w.actors.downcast_mut::<EnSw>(h).expect("En_Sw")
}

/// `func_80B0DE34`: the roll on its wall pointing its head at `p` (`Math_Vec3f_Yaw` less
/// `wallYaw`; `Math_Vec3f_Pitch` less 0x4000, negated for a yaw at or over 0).
fn roll_towards(s: &EnSw, p: Vec3) -> i16 {
    let a = s.actor.world_pos;
    let yaw = vec3f_yaw(a, p).wrapping_sub(s.actor.wall_yaw);
    let (dx, dz) = (p.x - a.x, p.z - a.z);
    let pitch = atan2_s((dx * dx + dz * dz).sqrt(), a.y - p.y).wrapping_sub(0x4000);
    (pitch as i32 * if yaw >= 0 { -1 } else { 1 }) as i16
}

/// `PLAYER_STATE1_21`: climbing.
const PLAYER_STATE1_21: u32 = 1 << 21;

#[test]
fn a_skullwalltula_finds_its_wall_and_turns_on_it() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree_init(&a);
    // The frame its object is in and EnSw_Init runs (Actor_UpdateAll then skips its update).
    let mut n = 0;
    let h = loop {
        idle(&mut w, 1);
        if let Some(h) = sw_near(&w, SW_PLACED) {
            break h;
        }
        n += 1;
        assert!(n < 60, "never spawned");
    };
    let s = sw(&w, h);
    // Params 0: the Skullwalltula. Actor_ChangeCategory to ACTORCAT_ENEMY (its profile's is
    // ACTORCAT_NPC), NAVI_ENEMY_SKULLWALLTULA, targetable and hostile.
    assert_eq!(s.actor.params, 0);
    assert_eq!(s.actor.category, ACTORCAT_ENEMY);
    assert!(w.actors.category(ACTORCAT_ENEMY).contains(&h) && !w.actors.category(ACTORCAT_NPC).contains(&h));
    assert_eq!(s.actor.navi_enemy_id, 0x1F);
    let f = oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED | oot_game::actor::ACTOR_FLAG_HOSTILE;
    assert_eq!(s.actor.flags & f, f);
    // Scale 0.02; DamageTable_Get(0xE); D_80B0F074 { 1, 2, 25, 25, MASS_IMMOVABLE }; the bite 8.
    assert_eq!(s.actor.scale, Vec3::splat(0.02));
    assert_eq!(s.actor.col_chk_info.damage_table.unwrap().table, en_sw::S_DAMAGE_TABLE_PRESET_14.table);
    let c = &s.actor.col_chk_info;
    assert_eq!((c.health, c.cyl_radius, c.cyl_height, c.cyl_y_shift, c.mass), (1, 2, 25, 25, 0xFF));
    assert_eq!(s.collider.elements[0].info.at_dmg_info.damage, 8);
    // world.rot.x and .z zeroed (the placement's x is 0x4000), shape.rot the same.
    assert_eq!((s.actor.world_rot.x, s.actor.world_rot.y, s.actor.world_rot.z), (0, SW_YAW, 0));
    assert_eq!(s.actor.shape_rot, s.actor.world_rot);
    // unk_484 60 behind it (the draw has since put limb 1's there); func_80B0DFFC: the line
    // test to it meets the vines (wallYaw from the poly's normal) and it goes 6 out from where
    // (unk_434 1 out); home there.
    let behind = SW_PLACED + Vec3::new(sin_s(SW_YAW) * -60.0, 0.0, cos_s(SW_YAW) * -60.0);
    let (hit, wall) = w.col.entity_line_test(SW_PLACED, behind, true, false, false, true).expect("its wall");
    let want = hit + Vec3::new(6.0 * sin_s(SW_YAW), 0.0, 6.0 * cos_s(SW_YAW));
    assert_eq!((s.actor.world_pos, s.actor.home_pos), (want, want));
    assert_eq!(s.unk_434, hit + Vec3::new(sin_s(SW_YAW), 0.0, cos_s(SW_YAW)));
    let nrm = w.col.poly(wall).normal;
    assert_eq!(s.actor.wall_yaw, oot_game::sys_matrix::rad_to_binang(oot_game::camera::f_atan2f(nrm[0] as f32, nrm[2] as f32)));
    use oot_game::surface::SurfaceType;
    assert_ne!(w.col.wall_flags(wall) & oot_game::surface::WALL_FLAG_3, 0, "the vines");
    // unk_38E = Rand_S16Offset(0xF, 0x1E); func_80B0E5E0.
    assert!((15..45).contains(&s.unk_38e));
    assert_eq!(s.action, Action::Idle);
    // func_80B0E5E0 frame by frame (Rand picks the turns: their invariants). func_80B0E430(6,
    // 0x3E8, 1): while unk_388 counts down the roll holds; else the roll steps 0x3E8 towards
    // unk_444 (Math_SmoothStepToS with step and min step 0x3E8), world.rot following; there,
    // the next turn: 0x2EE0 + 20000 × rand either way, and unk_388 = Rand_S16Offset(10, 30).
    let (mut turns, mut steps) = (0, 0);
    for k in 0..200 {
        let before = (s_roll(&w, h), sw(&w, h).unk_444, sw(&w, h).unk_388);
        idle(&mut w, 1);
        let s = sw(&w, h);
        let roll = s.actor.shape_rot.z;
        assert_eq!(s.actor.world_rot, s.actor.shape_rot, "frame {k}");
        assert_eq!(s.action, Action::Idle);
        if before.2 != 0 {
            // DECR(unk_388) != 0: no turn this frame (a pick at its end needs it at 0 first).
            if before.2 > 1 {
                assert_eq!((roll, s.unk_388), (before.0, before.2 - 1), "frame {k}");
            }
            continue;
        }
        if roll != before.0 {
            steps += 1;
            let d = (roll.wrapping_sub(before.0) as i32).abs();
            assert!(d == 0x3E8 || roll == before.1, "frame {k}: {} to {roll} towards {}", before.0, before.1);
        }
        if s.unk_444 != before.1 {
            // A new turn picked with the roll there.
            turns += 1;
            assert_eq!(roll, before.1, "frame {k}");
            let d = s.unk_444.wrapping_sub(roll) as i32;
            assert!((0x2EE0..0x2EE0 + 20000).contains(&d.abs()), "frame {k}: turn {d}");
            assert!((10..40).contains(&s.unk_388), "frame {k}: wait {}", s.unk_388);
        }
    }
    assert!(turns >= 2 && steps >= 20, "{turns} turns, {steps} steps");
}

fn s_roll(w: &PlayState, h: ActorHandle) -> i16 {
    sw(w, h).actor.shape_rot.z
}

#[test]
fn a_skullwalltula_dashes_at_link_climbing_its_vines() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let h = sw_near(&w, SW_PLACED).expect("the Skullwalltula");
    let home = sw(&w, h).actor.home_pos;
    // Link at the vines' foot below it, facing them; stick up: he grabs them and climbs
    // (PLAYER_STATE1_21) until 128 below it (within func_80B0DEA8's 130).
    let wy = sw(&w, h).actor.wall_yaw;
    let n = Vec3::new(sin_s(wy), 0.0, cos_s(wy));
    w.place_player(Vec3::new(home.x, 0.0, home.z) + n * 30.0, wy.wrapping_add(i16::MIN));
    let up = PadState { stick_y: 60, ..Default::default() };
    let mut prev = PadState::default();
    let mut n = 0;
    while !(w.player().state1 & PLAYER_STATE1_21 != 0 && w.player().actor.world_pos.distance(home) < 128.0) {
        tick(&mut w, up, &mut prev);
        n += 1;
        assert!(n < 60, "Link never climbed");
    }
    // Its roll comes from Rand's turns: driven to face Link (func_80B0DE34 of him), so the angle
    // test holds (|roll - func_80B0DE34(Link)| < 0x1FC2), its laugh not waiting on a turn.
    let link = w.player().actor.world_pos;
    let s = sw_mut(&mut w, h);
    let r = roll_towards(s, link);
    s.actor.shape_rot.z = r;
    s.actor.world_rot.z = r;
    s.unk_444 = r;
    s.unk_442 = 0;
    // Link moving on the vines (PLAYER_STATE2_12 off): func_80B0DEA8 holds;
    // NA_SE_EN_STALWALL_LAUGH, unk_442 20, func_80B0E728.
    tick(&mut w, up, &mut prev);
    let s = sw(&w, h);
    assert_eq!((s.action, s.unk_442), (Action::Dash, 20));
    assert!(sfx_on(&w, w.audio.frames, en_sw::NA_SE_EN_STALWALL_LAUGH));
    // func_80B0E5E0 also picked a turn this frame (unk_388 = Rand_S16Offset(10, 30)): cleared, so
    // func_80B0E430 turns the roll to Link at once (driven).
    sw_mut(&mut w, h).unk_388 = 0;
    // Its 20 frames of func_80B0E728's turning: unk_448 Link 30 up (where he is in the actors'
    // update: the climb's movement comes after, in AnimTaskQueue_Update), unk_444 the roll to
    // it, the roll stepping 0xFA0 towards it; Dash's purple fog in the draw.
    for k in 1..20 {
        let roll0 = s_roll(&w, h);
        let link = w.player().actor.world_pos;
        tick(&mut w, up, &mut prev);
        let s = sw(&w, h);
        assert_eq!((s.action, s.unk_442), (Action::Dash, 20 - k), "frame {k}");
        let after = w.player().actor.world_pos;
        let seen = s.unk_448 - Vec3::new(0.0, 30.0, 0.0);
        assert!(seen.y >= link.y.min(after.y) - 0.01 && seen.y <= link.y.max(after.y) + 0.01, "frame {k}: {seen:?} vs Link {link:?} to {after:?}");
        assert!(seen.distance(link).min(seen.distance(after)) < 3.0, "frame {k}: {seen:?} vs Link {link:?} to {after:?}");
        assert_eq!(s.unk_444, roll_towards(s, s.unk_448), "frame {k}");
        let d = (s.actor.shape_rot.z.wrapping_sub(roll0) as i32).abs();
        assert!(d <= 0xFA0, "frame {k}: {roll0} to {}", s.actor.shape_rot.z);
    }
    assert!(dash_fog(&w), "the dash's purple fog");
    // unk_442 out: func_80B0DFFC (one corner's line test a frame, play->state.frames % 4), which
    // also puts it back on its wall (6 out from where the line to unk_484 meets it). Clear,
    // func_80B0E314(unk_448, 8): speed eased 0 -> 8 (Math_SmoothStepToF 0.3, 100, 0.1), along the
    // line to unk_448 by it; blocked, home after Rand_S16Offset(20, 10).
    let mut speed = 0.0f32;
    let mut dashed = 0;
    let mut dash_sfx = 0;
    loop {
        let s = sw(&w, h);
        let (target, yaw) = (s.unk_448, s.actor.world_rot.y);
        let pos0 = match w.col.entity_line_test(s.actor.world_pos, s.unk_484, true, false, false, true) {
            Some((hit, _)) => hit + Vec3::new(6.0 * sin_s(yaw), 0.0, 6.0 * cos_s(yaw)),
            None => s.actor.world_pos,
        };
        tick(&mut w, up, &mut prev);
        let s = sw(&w, h);
        match s.action {
            Action::Dash => {
                eng_math::smooth_step_to_f(&mut speed, 8.0, 0.3, 100.0, 0.1);
                assert_eq!(s.actor.speed_xz, speed);
                let step = (target - pos0) / (target - pos0).length() * speed;
                assert!((s.actor.world_pos - (pos0 + step)).length() < 1e-3, "{:?} vs {:?}", s.actor.world_pos, pos0 + step);
                dashed += 1;
                dash_sfx += sfx_on(&w, w.audio.frames, en_sw::NA_SE_EN_STALWALL_DASH) as usize;
            }
            Action::Brake => {
                // Within 13 of unk_448, or Link holding still on the vines (func_8002DDF4):
                // func_80B0E90C.
                eng_math::smooth_step_to_f(&mut speed, 8.0, 0.3, 100.0, 0.1);
                assert_eq!(s.actor.speed_xz, speed);
                assert!(s.actor.world_pos.distance(target) <= 13.0 || w.player().state2 & (1 << 12) != 0);
                dashed += 1;
                break;
            }
            Action::ReturnHome => {
                // Blocked: home after Rand_S16Offset(20, 10), its roll towards home.
                assert!((20..30).contains(&s.unk_442), "{}", s.unk_442);
                assert_eq!((s.unk_448, s.unk_444), (home, roll_towards(s, home)));
                break;
            }
            other => panic!("{other:?}"),
        }
        assert!(dashed < 40);
    }
    eprintln!("dashed {dashed} frames, {dash_sfx} dash sounds");
    // NA_SE_EN_STALWALL_DASH every 4 frames of the dash (unk_440).
    assert!(dashed < 2 || dash_sfx >= 1);
    // No fog out of the dash (func_80B0EEA4 puts the scene's back).
    assert!(!dash_fog(&w));
    // func_80B0E90C: braking (speed eased to 0, still towards unk_448); stopped, func_80B0E9BC
    // with the roll towards home (func_80B0DE34(home)); turned, back there at 2 (eased), within
    // 4: turning again.
    let mut n = 0;
    while sw(&w, h).action != Action::Idle {
        let was = sw(&w, h).action;
        tick(&mut w, PadState::default(), &mut prev);
        let s = sw(&w, h);
        if was == Action::Brake && s.action == Action::ReturnHome {
            assert_eq!(s.actor.speed_xz, 0.0);
            assert_eq!((s.unk_448, s.unk_444), (home, roll_towards(s, home)));
        }
        n += 1;
        assert!(n < 400, "never home: {:?} at {:?}", s.action, s.actor.world_pos);
    }
    assert!(sw(&w, h).actor.world_pos.distance(home) <= 4.0);
}

/// Whether the draw has `func_80B0EDB8`'s fog on the Skullwalltula (184, 0, 228, from 0 to
/// 11500 / 30 × 10).
fn dash_fog(w: &PlayState) -> bool {
    let f = w.current_frame();
    let mut out = oot_game::play::DrawOut::default();
    w.draw(&f, &oot_game::play::ViewInfo::new(f.view.eye, glam::Mat4::IDENTITY), &mut out);
    let want = oot_game::gbi::gfx_set_fog(184, 0, 228, 255, 0, 3833);
    out.opa.iter().any(|c| c.mesh.name.contains("object_st_Skel_005298") && c.params.fog == Some(want))
}

/// The `En_Item00`s in the play.
fn items(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&i| w.actors.downcast::<oot_actors::en_item00::EnItem00>(i).is_some()).collect()
}

/// B every 12 frames until `done` (at most `max` frames); the frame it was.
fn slash_until(w: &mut PlayState, prev: &mut PadState, max: usize, mut done: impl FnMut(&PlayState) -> bool) -> Option<u32> {
    let b = with(PadState::default(), BTN_B);
    for i in 0..max {
        tick(w, if i % 12 < 2 { b } else { PadState::default() }, prev);
        if done(w) {
            return Some(w.audio.frames);
        }
    }
    None
}

#[test]
fn a_sword_slash_kills_a_skullwalltula_which_falls_bounces_and_drops_from_table_3() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let h = sw_near(&w, SW_PLACED).expect("the Skullwalltula");
    // Driven: moved down its vines to 30 up (where func_80B0DFFC would put it: 6 out from the
    // wall), in the Kokiri Sword's reach, its bite resting (unk_390). Link 40 out, facing it.
    let wy = sw(&w, h).actor.wall_yaw;
    let low = Vec3::new(SW_PLACED.x, 30.0, SW_PLACED.z);
    let (hit, _) = w.col.entity_line_test(low, low + Vec3::new(sin_s(SW_YAW) * -60.0, 0.0, cos_s(SW_YAW) * -60.0), true, false, false, true).expect("the vines");
    let pos = hit + Vec3::new(6.0 * sin_s(SW_YAW), 0.0, 6.0 * cos_s(SW_YAW));
    let n = Vec3::new(sin_s(wy), 0.0, cos_s(wy));
    w.place_player(Vec3::new(pos.x, 0.0, pos.z) + n * 40.0, wy.wrapping_add(i16::MIN));
    {
        let s = sw_mut(&mut w, h);
        s.actor.world_pos = pos;
        s.actor.home_pos = pos;
        s.actor.prev_pos = pos;
        s.unk_390 = 30;
    }
    let mut prev = PadState::default();
    tick(&mut w, PadState::default(), &mut prev);
    let before = items(&w);
    // B: the Kokiri Sword's slash, DMG_ENTRY(1, 0) in DamageTable_Get(0xE): its 1 health gone.
    let f = slash_until(&mut w, &mut prev, 60, |w| sw(w, h).action == Action::Fall).expect("the slash killed it");
    // func_80B0C9F0: unk_392 16 (AC off), red for 16 frames (Actor_SetColorFilter(RED, 200,
    // OPA, 16)); Actor_ApplyDamage to 0: Enemy_StartFinishingBlow (the freeze, and
    // NA_SE_EN_LAST_DAMAGE where it is), unk_38A 2 bounces, gravity -1, not targetable,
    // NA_SE_EN_STALWALL_DEAD; func_80B0DB00 the same frame; then func_80B0CBE8's
    // DECR(unk_392): 15.
    let s = sw(&w, h);
    assert_eq!(s.actor.col_chk_info.health, 0);
    assert_eq!((s.unk_392, s.unk_38a, s.actor.gravity), (15, 2, -1.0));
    assert_eq!(s.actor.color_filter_timer, 16);
    assert_eq!(s.actor.color_filter_params, oot_game::actor::COLORFILTER_COLORFLAG_RED | oot_game::actor::COLORFILTER_BUFFLAG_OPA | ((200 & 0xF8) << 5) | 16);
    assert_eq!(s.actor.flags & oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(w.actors.freeze_flash_timer, 5);
    assert!(sfx_on(&w, f, en_sw::NA_SE_EN_STALWALL_DEAD));
    assert!(sfx_on(&w, f, NA_SE_EN_LAST_DAMAGE));
    // func_80B0DB00: Actor_MoveXZGravity (velocity.y down by 1 a frame to -20), tumbling 0x1000
    // a frame on x and z; on the ground (falling): bounces at 2 × 8 × 0.5 = 8, then 1 × 4 = 4,
    // each with NA_SE_EN_DODO_M_GND and Actor_SpawnFloorDustRing's 13 clouds (12 + 1,
    // func_8002865C: 10 frames); the third landing, func_80B0DC7C with unk_394 10.
    let mut landings = Vec::new();
    let mut n = 0;
    let mut last = (s.actor.velocity.y, s.actor.shape_rot.x);
    while sw(&w, h).action == Action::Fall {
        idle(&mut w, 1);
        let s = sw(&w, h);
        n += 1;
        assert!(n < 200, "never landed");
        if (s.actor.velocity.y, s.actor.shape_rot.x) == last {
            continue; // the finishing blow's freeze
        }
        assert_eq!(s.actor.shape_rot.x, last.1.wrapping_add(0x1000));
        if sfx_on(&w, w.audio.frames, NA_SE_EN_DODO_M_GND) {
            assert_eq!(new_effects(&w, EFFECT_SS_DUST, 10), 13, "the dust ring");
            landings.push(if s.action == Action::Fall { s.actor.velocity.y } else { -1.0 });
        } else if s.action == Action::Fall {
            assert_eq!(s.actor.velocity.y, (last.0 - 1.0).max(-20.0));
        }
        last = (s.actor.velocity.y, s.actor.shape_rot.x);
    }
    assert_eq!(landings, vec![8.0, 4.0, -1.0]);
    assert_eq!((sw(&w, h).action, sw(&w, h).unk_394), (Action::Dissolve, 10));
    // func_80B0DC7C: DECR(unk_394) from 10: a puff a frame while it's not 0 (9:
    // EffectSsDeadDb, scale 42, 9 frames, white to red), tumbling; at 0, its drop
    // (Item_DropCollectibleRandom, table 3) and Actor_Kill.
    let mut puffs = 0;
    let mut at = Vec3::ZERO;
    while w.actors.downcast::<EnSw>(h).is_some_and(|s| !s.actor.killed) {
        at = sw(&w, h).actor.world_pos;
        idle(&mut w, 1);
        puffs += new_effects(&w, EFFECT_SS_DEAD_DB, 9);
        assert!(puffs <= 9);
    }
    assert_eq!(puffs, 9);
    // Table 3 (sItemDropIds[3 * 16 + Rand_ZeroOne() * 16]): what it dropped, where it was.
    let ids = &a.item_drops.ids[3 * 16..3 * 16 + 16];
    let new_items: Vec<ActorHandle> = items(&w).into_iter().filter(|i| !before.contains(i)).collect();
    for &i in &new_items {
        let it = w.actors.actor(i).unwrap();
        assert!(it.home_pos.distance(at) < 1.0, "{:?} vs {at:?}", it.home_pos);
        assert!(ids.contains(&(it.params as u8)), "{} in {ids:?}", it.params);
    }
    eprintln!("table 3: {ids:?}; dropped {:?}", new_items.iter().map(|&i| w.actors.actor(i).unwrap().params).collect::<Vec<_>>());
}

#[test]
fn a_gold_skulltula_takes_two_slashes_and_leaves_its_token() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    let h = sw_near(&w, GOLD_PLACED).expect("the Gold Skulltula");
    // It's shut in room 0's large crate (Obj_Kibako2 0xFFFF at (279, 360, 333)), which only a
    // hammer or an explosion breaks: an explosion in reach first (func_80033684: an
    // ACTORCAT_EXPLOSIVE actor with params 1 within rot.z × 10 + 80). The crate breaks that
    // frame (its collision off) and is gone the next (ObjKibako2_Kill: bit 15 set, no En_Sw of
    // its own).
    let crate_ = break_room_0s_crate(&mut w);
    assert!(w.actors.actor(crate_).is_none_or(|c| c.killed));
    // EnSw_Init: params 0x8102 has 0x8000: type ((0x8102 - 0x8000) >> 13 & 7) + 1 = 1, so 0x2102;
    // then its index one less, (0x2102 >> 8 & 0x1F) - 1 = 0: 0x2002 (GET_GS_FLAGS(0) & 2).
    let s = sw(&w, h);
    assert_eq!(s.actor.params, 0x2002);
    // Type 1: the bite doubled (16), NAVI_ENEMY_GOLD_SKULLTULA, health doubled (2), not
    // targetable; still ACTORCAT_NPC; func_80B0D590.
    assert_eq!(s.collider.elements[0].info.at_dmg_info.damage, 16);
    assert_eq!((s.actor.navi_enemy_id, s.actor.col_chk_info.health), (0x20, 2));
    assert_eq!(s.actor.flags & oot_game::actor::ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert!(w.actors.category(ACTORCAT_NPC).contains(&h));
    assert_eq!(s.action, Action::GoldIdle);
    // func_80B0C0CC from init: the floor 18 below (its normal up, unk_364), on it.
    assert_eq!(s.unk_364, Vec3::Y);
    let (floor, _) = w.col.entity_line_test(GOLD_PLACED + Vec3::Y * 18.0, GOLD_PLACED - Vec3::Y * 18.0, true, true, true, false).expect("its floor");
    assert_eq!(s.actor.home_pos, floor);
    // EnSw_Draw: its gold limbs (EnSw_OverrideLimbDraw's ten lists, baked: format 19), pitched
    // up 80 degrees (Matrix_RotateX(DEG_TO_RAD(-80))) and 200 out while alive.
    let gold_bake = oot_game::pack::keys::bake("En_Sw/gold");
    let _: eng_gfx::DrawList = a.pack.assets.get(&gold_bake).expect("the gold bake");
    let f = w.current_frame();
    let mut out = oot_game::play::DrawOut::default();
    w.draw(&f, &oot_game::play::ViewInfo::new(f.view.eye, glam::Mat4::IDENTITY), &mut out);
    let rs = &f.actors.iter().find(|(x, _)| *x == h).unwrap().1;
    let want =
        oot_game::play::actor_draw_matrix(rs) * glam::Mat4::from_rotation_x((-80.0f64 * (std::f64::consts::PI / 180.0f32 as f64)) as f32) * glam::Mat4::from_translation(Vec3::new(0.0, 0.0, 200.0));
    assert!(out.opa.iter().any(|c| c.mesh.name == gold_bake && c.transform == want));
    // Link 40 off, facing it.
    let link = s.actor.world_pos + Vec3::new(0.0, 0.0, 40.0);
    w.place_player(link, i16::MIN);
    let mut prev = PadState::default();
    // The first slash: 2 - 1 = 1 left: NA_SE_EN_STALTU_DAMAGE; unk_392 16 then 15 (its AC off
    // for 16 frames), red; still turning.
    let f = slash_until(&mut w, &mut prev, 60, |w| sw(w, h).actor.col_chk_info.health == 1).expect("the first slash hit");
    let s = sw(&w, h);
    assert!(sfx_on(&w, f, en_sw::NA_SE_EN_STALTU_DAMAGE));
    assert_eq!((s.action, s.unk_392, s.actor.color_filter_timer), (Action::GoldIdle, 15, 16));
    // The second: 0: Enemy_StartFinishingBlow, NA_SE_EN_STALWALL_DEAD; func_80B0D878 with the
    // animation at 8, unk_420 ±0.1 × 4 by the frame's parity, unk_394 10, unk_392 16 - 1.
    let f = slash_until(&mut w, &mut prev, 80, |w| sw(w, h).action == Action::GoldDie).expect("the second slash killed it");
    let s = sw(&w, h);
    assert_eq!(s.actor.col_chk_info.health, 0);
    assert!(sfx_on(&w, f, en_sw::NA_SE_EN_STALWALL_DEAD));
    assert_eq!(s.unk_420.abs(), 0.4);
    assert_eq!((s.unk_394, s.unk_392, s.skel.play_speed), (10, 15, 8.0));
    let spin = s.unk_420;
    // func_80B0D878 a frame at a time (the freeze's frames skipped): spinning by unk_420 about
    // its floor's normal (func_80B0CCF4: the yaw by 0.4 rad, 0x104C), unk_392 down to 0
    // (func_80B0CBE8), then 9 puffs (DECR(unk_394) from 10), then NA_SE_SY_KINSTA_MARK_APPEAR and
    // its token: En_Si 10 up its normal, its params, no parent; it's gone.
    let mut puffs = 0;
    let mut n = 0;
    let mut last = sw(&w, h).actor.world_rot.y;
    let mut at = Vec3::ZERO;
    while w.actors.downcast::<EnSw>(h).is_some_and(|s| !s.actor.killed) {
        at = sw(&w, h).actor.world_pos;
        let r392 = sw(&w, h).unk_392;
        idle(&mut w, 1);
        n += 1;
        assert!(n < 80);
        puffs += new_effects(&w, EFFECT_SS_DEAD_DB, 9);
        let Some(s) = w.actors.downcast::<EnSw>(h) else { break };
        if s.actor.world_rot.y == last {
            continue; // frozen
        }
        let d = s.actor.world_rot.y.wrapping_sub(last) as i32;
        assert!((d - (spin / 0.4 * 0x104C as f32) as i32).abs() <= 2, "turned {d:#x}");
        last = s.actor.world_rot.y;
        if !s.actor.killed {
            assert_eq!(s.unk_392, (r392 - 1).max(0));
        }
    }
    assert_eq!(puffs, 9);
    assert!(sfx_on(&w, w.audio.frames, en_sw::NA_SE_SY_KINSTA_MARK_APPEAR));
    let t = w.actors.all().into_iter().find(|&t| w.actors.downcast::<EnSi>(t).is_some()).expect("its token");
    let tok = w.actors.downcast::<EnSi>(t).unwrap();
    assert_eq!(tok.actor.params, 0x2002);
    assert_eq!(tok.actor.home_pos, at + Vec3::new(0.0, 10.0, 0.0));
    assert_eq!(tok.actor.parent, None);
    assert_eq!(tok.action, TokenAction::Spin);
}

/// A bomb's explosion as `func_80033684` sees one (`ACTORCAT_EXPLOSIVE`, params 1, `shape.rot.z`
/// the blast's size); it does nothing itself (`En_Bom` isn't ported).
struct Blast {
    actor: oot_game::actor::Actor,
}

impl oot_game::actor_ctx::ActorImpl for Blast {
    fn name(&self) -> &'static str {
        "Injected explosion"
    }
    fn base(&self) -> &oot_game::actor::Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut oot_game::actor::Actor {
        &mut self.actor
    }
    fn update(&mut self, _play: &mut PlayState) {}
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Room 0's large crate broken by an explosion 20 from it (rot.z 0: a reach of 80); the
/// explosion gone after. Returns the crate's handle.
fn break_room_0s_crate(w: &mut PlayState) -> ActorHandle {
    use oot_actors::obj_kibako2::{Action as CrateAction, ObjKibako2};
    let at = Vec3::new(279.0, 360.0, 333.0);
    let c = w.actors.all().into_iter().find(|&c| w.actors.downcast::<ObjKibako2>(c).is_some_and(|k| k.actor.home_pos == at)).expect("room 0's crate");
    let mut b = oot_game::actor::Actor::new(at + Vec3::new(20.0, 0.0, 0.0), 0);
    b.id = 0x0010; // ACTOR_EN_BOM
    b.category = oot_game::actor_ctx::ACTORCAT_EXPLOSIVE;
    b.params = 1;
    let blast = w.spawn(Box::new(Blast { actor: b })).expect("spawn");
    idle(w, 1);
    assert_eq!(w.actors.downcast::<ObjKibako2>(c).unwrap().action, CrateAction::Kill);
    w.actors.actor_mut(blast).unwrap().kill();
    idle(w, 1);
    c
}

fn token(w: &PlayState, h: ActorHandle) -> &EnSi {
    w.actors.downcast::<EnSi>(h).expect("En_Si")
}

/// The token's text read to its end (A every 4 frames once it's out), Link frozen throughout;
/// its decoded characters, the boxes joined.
fn read_token_text(w: &mut PlayState, t: ActorHandle, prev: &mut PadState) -> String {
    let a_btn = with(PadState::default(), BTN_A);
    let mut boxes: Vec<String> = Vec::new();
    let mut n = 0;
    while w.actors.downcast::<EnSi>(t).is_some_and(|s| !s.actor.killed) {
        assert_eq!(w.player().actor.freeze_timer, 10, "frame {n}");
        if w.msg_ctx.decoded_text_len > 0 {
            let t = decoded_text(w);
            if boxes.last().is_none_or(|b| !t.starts_with(b.as_str()) && !b.starts_with(t.as_str())) {
                boxes.push(t);
            } else if boxes.last().is_some_and(|b| t.len() > b.len()) {
                *boxes.last_mut().unwrap() = t;
            }
        }
        tick(w, if n % 4 == 0 && n > 20 { a_btn } else { PadState::default() }, prev);
        n += 1;
        assert!(n < 300, "the text never closed: {boxes:?}");
    }
    boxes.join(" ")
}

/// The message box's decoded text: its printable characters, a new line as a space, without
/// the colours (`MESSAGE_COLOR` and its argument).
fn decoded_text(w: &PlayState) -> String {
    let mut out = String::new();
    let mut it = w.msg_ctx.msg_buf_decoded.iter().take(w.msg_ctx.decoded_text_len as usize + 1);
    while let Some(&c) = it.next() {
        match c {
            oot_game::message::MESSAGE_COLOR => {
                it.next();
            }
            oot_game::message::MESSAGE_NEWLINE => out.push(' '),
            b' '..=b'~' => out.push(c as char),
            _ => {}
        }
    }
    out
}

#[test]
fn link_takes_a_gold_skulltula_token_and_its_skulltula_isnt_spawned_again() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    // A token as the Gold Skulltula at (278, 360, 332) leaves it (func_80B0D878: 10 up its
    // floor's normal, its params 0x2002), on the ground floor away from it; Link 100 off.
    let floor = Vec3::new(100.0, 0.0, -200.0);
    assert!(w.col.entity_line_test(floor + Vec3::Y * 18.0, floor - Vec3::Y * 18.0, true, true, true, false).is_some_and(|(p, _)| p == floor));
    w.place_player(floor + Vec3::new(0.0, 0.0, 100.0), i16::MIN);
    let pos = floor + Vec3::new(0.0, 10.0, 0.0);
    let t = w.actor_spawn(oot_actors::en_si::ACTOR_EN_SI, pos, [0, 0, 0], 0x2002).expect("En_Si");
    // EnSi_Init: scale 0.025, shape.yOffset 42, func_80AFB768.
    let tk = token(&w, t);
    assert_eq!((tk.actor.scale.x, tk.actor.shape_y_offset, tk.action), (0.025, 42.0, TokenAction::Spin));
    // func_80AFB768: Math_SmoothStepToF(&scale.x, 0.25, 0.4, 1, 0) (0.025 + 0.225 × 0.4 = 0.115,
    // ...), turning 0x400 a frame; Actor_SetFocus 16 up.
    let mut scale = 0.025f32;
    for k in 0..5 {
        let rot0 = token(&w, t).actor.shape_rot.y;
        idle(&mut w, 1);
        eng_math::smooth_step_to_f(&mut scale, 0.25, 0.4, 1.0, 0.0);
        let tk = token(&w, t);
        assert_eq!(tk.actor.scale, Vec3::splat(scale), "frame {k}");
        assert_eq!(tk.actor.shape_rot.y, rot0.wrapping_add(0x400));
        assert_eq!(tk.actor.focus_pos, tk.actor.world_pos + Vec3::new(0.0, 16.0, 0.0));
    }
    // Link onto it: his OC cylinder meets its (OC2_HIT_PLAYER, from the last frame's check):
    // Item_Give(ITEM_SKULL_TOKEN) (gsTokens 0 -> 1, the QUEST_SKULL_TOKEN bit), Link frozen 10
    // frames, text 0xB4, NA_BGM_SMALL_ITEM_GET; func_80AFB950.
    assert_eq!(w.save.inventory.gs_tokens, 0);
    w.place_player(floor, i16::MIN);
    let mut n = 0;
    while token(&w, t).action == TokenAction::Spin {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 5, "never touched");
    }
    assert_eq!(w.save.inventory.gs_tokens, 1);
    assert_ne!(w.save.inventory.quest_items & (1 << oot_game::item::QUEST_SKULL_TOKEN), 0);
    assert_eq!(w.player().actor.freeze_timer, 10);
    assert_eq!(w.msg_ctx.text_id, oot_actors::en_si::TEXT_ID_TOKEN);
    assert_eq!(w.audio.fanfare_seq_id, oot_game::audio::NA_BGM_SMALL_ITEM_GET);
    // Not drawn once taken (EnSi_Draw).
    assert_eq!(w.current_frame().actors.iter().find(|(h, _)| *h == t).map(|(_, rs)| rs.switches[0]), Some(1));
    // While the text shows, Link stays frozen (func_80AFB950); text 0xB4 (no count in it).
    let mut prev = PadState::default();
    let text = read_token_text(&mut w, t, &mut prev);
    assert!(text.starts_with("You destroyed a") && text.contains("Gold Skulltula"), "{text:?}");
    assert!(!a.messages.raw(0xB4).unwrap().contains(&oot_game::message::MESSAGE_TOKENS));
    // At TEXT_STATE_CLOSING: SET_GS_FLAGS(0, 2), and it's gone.
    assert_eq!(w.save.get_gs_flags(0), 2);
    // A second token (flag 4), after the House of Skulltula's EVENTCHKINF_96: Message_OpenText
    // would give 0xB5 if msgCtx->textId were still 0xB4, but the closed box reset it to 0
    // (@bug (game); 0xB5's English text is 0xB4's anyway). (Link, frozen his last 10 frames,
    // registered no OC: touched after.)
    w.save.set_event_chk_inf(0x96);
    let t2 = w.actor_spawn(oot_actors::en_si::ACTOR_EN_SI, pos, [0, 0, 0], 0x2004).expect("En_Si");
    let mut n = 0;
    while token(&w, t2).action == TokenAction::Spin {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 15, "never touched");
    }
    assert_eq!((w.save.inventory.gs_tokens, w.msg_ctx.text_id), (2, 0xB4));
    read_token_text(&mut w, t2, &mut prev);
    assert_eq!(w.save.get_gs_flags(0), 2 | 4);
    // The token count in a text: MESSAGE_TOKENS (gsTokens, no leading zeros), in the House of
    // Skulltula's 0x24 ("Since you've destroyed N Spiders of the Curse...").
    assert!(a.messages.raw(0x24).unwrap().contains(&oot_game::message::MESSAGE_TOKENS));
    w.start_textbox(0x24, None);
    let mut text = String::new();
    for _ in 0..40 {
        tick(&mut w, PadState::default(), &mut prev);
        if w.msg_ctx.decoded_text_len > 0 {
            text = decoded_text(&w);
        }
    }
    assert!(text.starts_with("Since you've destroyed 2 Spiders"), "{text:?}");
    // EnSw_Init for the same Gold Skulltula (params 0x8102): GET_GS_FLAGS(0) & 2, Actor_Kill.
    let again = w.actor_spawn(en_sw::ACTOR_EN_SW, GOLD_PLACED, [0, -24576, 0], 0x8102u16 as i16).expect("En_Sw");
    assert!(w.actors.actor(again).unwrap().killed);
    idle(&mut w, 1);
    assert!(w.actors.actor(again).is_none());
    // Another flag (0x8108: GET_GS_FLAGS(0) & 8) is spawned.
    let other = w.actor_spawn(en_sw::ACTOR_EN_SW, GOLD_PLACED, [0, -24576, 0], 0x8108u16 as i16).expect("En_Sw");
    assert!(!w.actors.actor(other).unwrap().killed);
}

#[test]
fn spawned_gold_skulltulas_jump_out_or_wait_for_the_night() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    // On the ground floor by the vines, Link 60 off.
    let ground = Vec3::new(40.0, 0.0, -200.0);
    assert!(w.col.entity_line_test(ground + Vec3::Y * 18.0, ground - Vec3::Y * 18.0, true, true, true, false).is_some_and(|(p, _)| p == ground));
    w.place_player(ground + Vec3::new(60.0, 0.0, 0.0), 0x4000);
    idle(&mut w, 1);
    // Type 3 (params 3 << 13, index 1 - 1 = 0, flag 0x10): NA_SE_SY_CORRECT_CHIME; jumping
    // (unk_360 1) at 8 up and 4 ahead, falling by 1, from scale 0; health 2; unk_38C 0x28,
    // func_80B0D364.
    let params = (3 << 13) | (1 << 8) | 0x10;
    // Facing -z (yaw 0x8000): it jumps towards the vines.
    let h = w.actor_spawn(en_sw::ACTOR_EN_SW, ground, [0, i16::MIN, 0], params).expect("En_Sw");
    assert!(w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == w.audio.frames && s == NA_SE_SY_CORRECT_CHIME));
    let s = sw(&w, h);
    assert_eq!(s.actor.params, (3 << 13) | 0x10);
    assert_eq!((s.unk_360, s.actor.velocity.y, s.actor.speed_xz, s.actor.gravity, s.actor.scale.x), (1, 8.0, 4.0, -1.0, 0.0));
    assert_eq!((s.actor.col_chk_info.health, s.unk_38c, s.unk_394, s.action), (2, 0x28, 1, Action::SetupEmerge));
    // func_80B0D364 (type 3): 10 frames' wait, func_80B0D3AC.
    idle(&mut w, 1);
    assert_eq!((sw(&w, h).unk_38c, sw(&w, h).action), (10, Action::Emerge));
    // func_80B0D3AC: the wait down from 10, a burst of 6 dust clouds (func_80B0CF44(5):
    // func_8002836C, 12 frames) when it has bit 2 (at 7, 6, 5 and 4); out of it,
    // NA_SE_EN_STALGOLD_UP_CRY and NA_SE_EN_DODO_M_UP (SfxSource_PlaySfxAtFixedWorldPos), and
    // it moves.
    for k in (0..10).rev() {
        let had = sw(&w, h).unk_38c;
        idle(&mut w, 1);
        assert_eq!(sw(&w, h).unk_38c, k);
        assert_eq!(new_effects(&w, EFFECT_SS_DUST, 12), if had & 4 != 0 { 6 } else { 0 }, "at {had}");
    }
    let f = w.audio.frames;
    assert!(sfx_on(&w, f, en_sw::NA_SE_EN_STALGOLD_UP_CRY) && sfx_on(&w, f, en_sw::NA_SE_EN_DODO_M_UP));
    // Flying: the scale to 0.02 (Math_ApproachF 0.2, 0.01), up along unk_364 by velocity.y and
    // ahead along unk_37C by 4, velocity.y down by 1; falling (velocity.y < 0) unk_360 0, and
    // func_80B0C0CC finds its floor: NA_SE_EN_DODO_M_GND, 9 clouds (func_80B0D14C(8)), scale 0.02,
    // stopped, func_80B0D590.
    let mut n = 0;
    while sw(&w, h).action == Action::Emerge {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 40, "never landed");
    }
    let s = sw(&w, h);
    assert_eq!(s.action, Action::GoldIdle);
    assert!(sfx_on(&w, w.audio.frames, NA_SE_EN_DODO_M_GND));
    assert_eq!(new_effects(&w, EFFECT_SS_DUST, 10), 9);
    assert_eq!((s.actor.scale.x, s.actor.velocity.y, s.actor.speed_xz, s.actor.gravity, s.unk_360), (0.02, 0.0, 0.0, 0.0, 0));
    eprintln!("type 3 landed after {n} frames at {:?}", s.actor.world_pos);
    // Type 2 (params 2 << 13): out only at night (IS_DAY: the scale held at 0, its sphere off
    // under 0.014); at night it grows (Math_ApproachF to 0.02 by 0.2, at most 0.01).
    let h2 = w.actor_spawn(en_sw::ACTOR_EN_SW, ground + Vec3::new(-40.0, 0.0, 0.0), [0, 0, 0], (2 << 13) | (1 << 8) | 0x20).expect("En_Sw");
    idle(&mut w, 3);
    let s = sw(&w, h2);
    assert_eq!((s.action, s.actor.scale.x), (Action::GoldIdle, 0.0));
    assert_eq!((s.collider.elements[0].info.ac_elem_flags, s.collider.elements[0].info.at_elem_flags), (oot_game::collision_check::ACELEM_NONE, oot_game::collision_check::ATELEM_NONE));
    w.save.night_flag = true;
    let mut scale = 0.0f32;
    for _ in 0..6 {
        idle(&mut w, 1);
        eng_math::approach_f(&mut scale, 0.02, 0.2, 0.01);
        assert_eq!(sw(&w, h2).actor.scale.x, scale);
    }
    idle(&mut w, 1);
    assert_eq!(sw(&w, h2).collider.elements[0].info.ac_elem_flags, oot_game::collision_check::ACELEM_ON);
}
