//! The fragments (`Effect_Ss_Kakera`, `z_eff_ss_kakera.c`) and the props that throw them, against
//! the C: a cut bush's leaves (`En_Kusa`), a broken rock's pieces and dust (`En_Ishi`), the
//! training boulder's break (`En_Goroiwa`), and a fragment's flight frame by frame (the drag, the
//! forces, gravity, the bounces and the floor, its life).
//!
//! Kokiri Forest's village (`ENTR_KOKIRI_FOREST_3`, room 0) has the bushes (`En_Kusa` 0x0200: type
//! 0, drop table 2) and small rocks (`En_Ishi` 0x0200); its room 2 the boulder (`En_Goroiwa`
//! 0x0C02, loop mode 0: it never breaks there, so its break is called directly). Link can't
//! break a rock (a hammer, a bomb, a throw): the tests inject the hammer's hit. The flights run
//! on the Deku Tree's room 0 ground floor (flat, y 0), `EffectSs_UpdateAll` called alone so that
//! only the fragment draws `Rand`. Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_B, PadState};
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_goroiwa::EnGoroiwa;
use oot_actors::en_ishi::{self, EnIshi};
use oot_actors::en_kusa::{self, EnKusa};
use oot_game::actor::{BGCHECKFLAG_GROUND, BGCHECKFLAG_WALL};
use oot_game::actor_ctx::ActorHandle;
use oot_game::collision_check as cc;
use oot_game::effect::kakera::{self as k, KAKERA_COLOR_NONE, KAKERA_COLOR_WHITE, KAKERA_OBJECT_DEFAULT, OBJECT_GAMEPLAY_FIELD_KEEP};
use oot_game::effect::{EFFECT_SS_DUST, EFFECT_SS_KAKERA, EffectSs, SsDraw, dust};
use oot_game::play::{PlayState, Rand, scripted_input};
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

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

/// Kokiri Forest's village on a new file with the Kokiri Sword and the Deku Shield, the sound log
/// on, 4 frames in (room 0's props initialised).
fn village(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_3").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    oot_game::save::kokiri_sword_and_deku_shield(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 4);
    w
}

/// Inside the Deku Tree, room 0, 60 frames in, Link standing on the ground floor at
/// (100, 0, -100).
fn deku_tree(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("deku-tree-inside").unwrap();
    let mut w =
        PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true)).expect("Play_Init");
    w.flags.set_switch(0x1F);
    idle(&mut w, 60);
    w.place_player(Vec3::new(100.0, 0.0, -100.0), 0);
    idle(&mut w, 2);
    w
}

fn kakera_of<'a>(w: &'a PlayState, symbol: &str) -> Vec<(usize, &'a EffectSs)> {
    w.effect_ss.table.iter().enumerate().filter(|(_, e)| e.life > -1 && e.ty == EFFECT_SS_KAKERA && e.gfx.is_some_and(|g| g.1 == symbol)).collect()
}

/// `EnKusa_SpawnFragments` the C's way from `r`, for a bush at `p` with scale `s`: for each of
/// `sUnitDirections`, the stalk (20 out, 10 up; ±4 across, up to 10 up; `sFragmentScales[(s32)(Rand
/// × 111.1) & 7]`, then `EffectSsKakera_Init`'s pitch and yaw), then the tip (40 out; ±3; `% 7`).
fn expected_leaves(r: &mut Rand, p: Vec3, s: Vec3) -> Vec<(Vec3, Vec3, i16, i16, i16, &'static str)> {
    let dirs = [Vec3::new(0.0, 0.7071, 0.7071), Vec3::new(0.7071, 0.7071, 0.0), Vec3::new(0.0, 0.7071, -0.7071), Vec3::new(-0.7071, 0.7071, 0.0)];
    let mut v = Vec::new();
    for d in dirs {
        for (dist, spread, tip) in [(20.0, 8.0, false), (40.0, 6.0, true)] {
            let pos = Vec3::new(p.x + (d.x * s.x * dist), p.y + (d.y * s.y * dist) + 10.0, p.z + (d.z * s.z * dist));
            let vx = (r.zero_one() - 0.5) * spread;
            let vy = r.zero_one() * 10.0;
            let vz = (r.zero_one() - 0.5) * spread;
            let n = (r.zero_one() * 111.1) as i32;
            let scale = en_kusa::FRAGMENT_SCALES[if tip { n % 7 } else { n & 7 } as usize];
            let pitch = (r.zero_one() * 32767.0) as i16;
            let yaw = (r.zero_one() * 32767.0) as i16;
            v.push((pos, Vec3::new(vx, vy, vz), scale, pitch, yaw, if tip { "gCuttableShrubTipDL" } else { "gCuttableShrubStalkDL" }));
        }
    }
    v
}

#[test]
fn a_cut_bush_scatters_eight_leaves_before_its_drop() {
    let Some(a) = assets() else { return };
    let mut w = village(&a);
    // A bush, cut (AC_HIT) and updated alone (EnKusa_Main), on an empty effect table: the
    // leaves' Rand calls come first, from the state before its update.
    let bush = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnKusa>(h).is_some_and(|k| k.actor.home_pos.x == 385.0)).expect("the bush");
    let (p, s, params) = {
        let b = w.actors.downcast::<EnKusa>(bush).unwrap();
        (b.actor.world_pos, b.actor.scale, b.actor.params)
    };
    assert_eq!((params, s), (0x0200, Vec3::splat(0.4)));
    w.effect_ss = Default::default();
    let mut r = w.rand;
    let want = expected_leaves(&mut r, p, s);
    let mut act = w.actors.take(bush).unwrap();
    act.as_any_mut().downcast_mut::<EnKusa>().unwrap().collider.base.ac_flags |= cc::AC_HIT;
    w.cur_actor = Some(bush);
    act.update(&mut w);
    w.cur_actor = None;
    w.actors.put_back(bush, act);
    for (i, &(pos, velocity, scale, pitch, yaw, dl)) in want.iter().enumerate() {
        let e = &w.effect_ss.table[i];
        // EffectSsKakera_Spawn(pos, velocity, pos, -100, 64, 40, 3, 0, scale, 0, 0, 80,
        // KAKERA_COLOR_NONE, OBJECT_GAMEPLAY_KEEP, dl): a keep, so no object check.
        assert_eq!((e.ty, e.life, e.pos, e.velocity, e.vec), (EFFECT_SS_KAKERA, 80, pos, velocity, pos), "leaf {i}");
        assert_eq!(e.gfx, Some(("gameplay_keep", dl)), "leaf {i}");
        let rg = &e.regs;
        assert_eq!((rg[k::R_GRAVITY], rg[k::R_REG4], rg[k::R_REG5], rg[k::R_REG6], rg[k::R_REG0]), (-100, 64, 40, 3, 0), "leaf {i}");
        assert_eq!((rg[k::R_SCALE], rg[k::R_PITCH], rg[k::R_YAW], rg[k::R_REG8], rg[k::R_REG9]), (scale, pitch, yaw, 0, 0), "leaf {i}");
        assert_eq!((rg[k::R_OBJ_ID], rg[k::R_COLOR_IDX]), (KAKERA_OBJECT_DEFAULT, KAKERA_COLOR_NONE));
    }
    assert_eq!(w.effect_ss.table[8].ty == EFFECT_SS_KAKERA && w.effect_ss.table[8].life > -1, false, "eight leaves");
    // Then EnKusa_DropCollectible (Item_DropCollectibleRandom, table 2) draws on from there; the
    // bush (type 0) is killed.
    assert_ne!(w.rand, r);
    assert!(w.actors.actor(bush).unwrap().killed);
}

#[test]
fn the_sword_cuts_a_bush_into_leaves() {
    let Some(a) = assets() else { return };
    let mut w = village(&a);
    // The bush at (385, 0, 643): Link 35 in front of it, facing it; B draws the sword, B again
    // slashes.
    let bush = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnKusa>(h).is_some_and(|k| k.actor.home_pos.x == 385.0)).expect("the bush");
    let bush_pos = w.actors.actor(bush).unwrap().world_pos;
    w.place_player(bush_pos + Vec3::new(0.0, 0.0, -35.0), 0);
    let mut prev = PadState::default();
    let b = with(PadState::default(), BTN_B);
    let mut cut = None;
    for f in 0..60 {
        let pad = if f % 12 < 2 { b } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        if w.actors.actor(bush).is_none_or(|a| a.killed) {
            cut = Some(w.audio.frames);
            break;
        }
    }
    let f = cut.expect("the bush cut");
    // That frame: four stalks and four tips (80 frames, less EffectSs_UpdateAll's), and
    // NA_SE_EV_PLANT_BROKEN.
    let stalks = kakera_of(&w, "gCuttableShrubStalkDL");
    let tips = kakera_of(&w, "gCuttableShrubTipDL");
    assert_eq!((stalks.len(), tips.len()), (4, 4));
    assert!(stalks.iter().chain(tips.iter()).all(|(_, e)| e.life == 79 && e.regs[k::R_GRAVITY] == -100 && en_kusa::FRAGMENT_SCALES.contains(&e.regs[k::R_SCALE])));
    // A tip's scale index is % 7: never the last.
    assert!(tips.iter().all(|(_, e)| e.regs[k::R_SCALE] != en_kusa::FRAGMENT_SCALES[7]));
    assert!(sfx_on(&w, f, oot_game::audio::sfx::NA_SE_EV_PLANT_BROKEN));
    // The sword's hit stops the next frame (Player's freezeFlashTimer 1: no actor or effect
    // updates), then they fall (rReg4 bit 4 clear, no bounces) for the rest of their 80 frames
    // (600 under Link's floor would end them sooner).
    idle(&mut w, 1);
    assert!(kakera_of(&w, "gCuttableShrubStalkDL").iter().all(|(_, e)| e.life == 79));
    idle(&mut w, 79);
    assert!(kakera_of(&w, "gCuttableShrubStalkDL").iter().all(|(_, e)| e.life == 0));
    idle(&mut w, 1);
    assert!(kakera_of(&w, "gCuttableShrubStalkDL").is_empty() && kakera_of(&w, "gCuttableShrubTipDL").is_empty());
}

/// `EnIshi_SpawnFragmentsSmall` the C's way from `r`, for a rock at `p` whose velocity bounced
/// is `bounced`: three `Rand` places (±4 across, 5 to 10 up), the velocity ±5.5 across and up to
/// 6 up, the tumble 65 or 33, then the init's pitch and yaw.
fn expected_chips(r: &mut Rand, p: Vec3, bounced: Vec3) -> Vec<(Vec3, Vec3, i16, i16, i16)> {
    let mut v = Vec::new();
    for _ in 0..6 {
        let pos = Vec3::new(p.x + (r.zero_one() - 0.5) * 8.0, p.y + (r.zero_one() * 5.0) + 5.0, p.z + (r.zero_one() - 0.5) * 8.0);
        let mut vel = bounced;
        vel.x += (r.zero_one() - 0.5) * 11.0;
        vel.y += r.zero_one() * 6.0;
        vel.z += (r.zero_one() - 0.5) * 11.0;
        let tumble = if r.zero_one() < 0.5 { 65 } else { 33 };
        let pitch = (r.zero_one() * 32767.0) as i16;
        let yaw = (r.zero_one() * 32767.0) as i16;
        v.push((pos, vel, tumble, pitch, yaw));
    }
    v
}

fn a_rock(w: &PlayState) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnIshi>(h).is_some_and(|i| !i.actor.killed)).expect("a rock")
}

#[test]
fn a_small_rocks_pieces_and_dust_follow_the_c() {
    let Some(a) = assets() else { return };
    let mut w = village(&a);
    let h = a_rock(&w);
    // As if it landed from a throw (EnIshi_Fly): moving (1, -3, 2), on the ground: the pieces
    // take its velocity bounced (× 0.8 across, × -0.8 up), the dust its position moved by twice
    // it (+x, -y, +z).
    let mut act = w.actors.take(h).unwrap();
    let rock = act.as_any_mut().downcast_mut::<EnIshi>().unwrap();
    rock.actor.velocity = Vec3::new(1.0, -3.0, 2.0);
    rock.actor.bg_check_flags = BGCHECKFLAG_GROUND;
    let p = rock.actor.world_pos;
    w.effect_ss = Default::default();
    let mut r = w.rand;
    let want = expected_chips(&mut r, p, Vec3::new(1.0 * 0.8, -3.0 * -0.8, 2.0 * 0.8));
    // EnIshi_SpawnDustSmall: func_80033480(pos + (2, 6, 4), 60, 3, 0x50, 0x3C, 1): four puffs.
    let at = Vec3::new(p.x + 2.0 * 1.0, p.y - 2.0 * -3.0, p.z + 2.0 * 2.0);
    let mut puffs = Vec::new();
    for _ in 0..4 {
        let pos = Vec3::new(at.x + (r.zero_one() - 0.5) * 60.0, at.y + (r.zero_one() - 0.5) * 60.0, at.z + (r.zero_one() - 0.5) * 60.0);
        let s = (((80.0 * r.zero_one()) * 0.2) as i16).wrapping_add(80);
        r.zero_one();
        puffs.push((pos, s));
    }
    let rock = act.as_any_mut().downcast_mut::<EnIshi>().unwrap();
    rock.spawn_fragments(&mut w, 0);
    rock.spawn_dust(&mut w, 0);
    w.actors.put_back(h, act);
    assert_eq!(w.rand, r, "the Rand calls, in the C's order");
    for (i, &(pos, vel, tumble, pitch, yaw)) in want.iter().enumerate() {
        let e = &w.effect_ss.table[i];
        // EffectSsKakera_Spawn(pos, velocity, pos, -420, tumble, 30, 5, 0, scales[i], 3, 10, 40,
        // KAKERA_COLOR_NONE, OBJECT_GAMEPLAY_FIELD_KEEP, gFieldKakeraDL).
        assert_eq!((e.ty, e.life, e.pos, e.velocity, e.vec), (EFFECT_SS_KAKERA, 40, pos, vel, pos), "chip {i}");
        assert_eq!(e.gfx, Some(("gameplay_field_keep", "gFieldKakeraDL")));
        let rg = &e.regs;
        assert_eq!((rg[k::R_GRAVITY], rg[k::R_REG4], rg[k::R_REG5], rg[k::R_REG6], rg[k::R_REG0]), (-420, tumble, 30, 5, 0), "chip {i}");
        assert_eq!((rg[k::R_SCALE], rg[k::R_REG8], rg[k::R_REG9], rg[k::R_PITCH], rg[k::R_YAW]), (en_ishi::FRAGMENT_SCALES_SMALL[i], 3, 10, pitch, yaw), "chip {i}");
        assert_eq!((rg[k::R_OBJ_ID], rg[k::R_COLOR_IDX]), (KAKERA_OBJECT_DEFAULT, KAKERA_COLOR_NONE));
    }
    for (j, &(pos, s)) in puffs.iter().enumerate() {
        let e = &w.effect_ss.table[6 + j];
        assert_eq!((e.ty, e.pos, e.regs[dust::R_SCALE], e.regs[dust::R_DRAW_FLAGS]), (EFFECT_SS_DUST, pos, s, 5), "puff {j}");
    }
}

#[test]
fn a_large_rocks_pieces_follow_the_c() {
    let Some(a) = assets() else { return };
    let mut w = village(&a);
    let h = a_rock(&w);
    // sFragmentSpawnFuncs[ROCK_LARGE] on a rock against a wall (EnIshi_Fly's hit), moving
    // (2, 1, -1): bounced (× -0.9 across, × 0.8 up).
    let mut act = w.actors.take(h).unwrap();
    let rock = act.as_any_mut().downcast_mut::<EnIshi>().unwrap();
    rock.actor.velocity = Vec3::new(2.0, 1.0, -1.0);
    rock.actor.bg_check_flags = BGCHECKFLAG_WALL;
    let p = rock.actor.world_pos;
    w.effect_ss = Default::default();
    let mut r = w.rand;
    let bounced = Vec3::new(2.0 * -0.9, 1.0 * 0.8, -1.0 * -0.9);
    let mut want = Vec::new();
    let mut angle: i16 = 0x1000;
    for i in 0..9 {
        angle = angle.wrapping_add(0x4E20);
        let rnd = r.zero_one() * 10.0;
        let pos = Vec3::new(p.x + (sin_s(angle) * rnd), p.y + (r.zero_one() * 40.0) + 5.0, p.z + (cos_s(angle) * rnd));
        let mut vel = bounced;
        let rnd = r.zero_one() * 10.0;
        vel.x += rnd * sin_s(angle);
        // (Rand_ZeroOne() * 4.0f) + ((Rand_ZeroOne() * i) * 0.7f): left to right (IDO).
        let r1 = r.zero_one() * 4.0;
        let r2 = (r.zero_one() * i as f32) * 0.7;
        vel.y += r1 + r2;
        vel.z += rnd * cos_s(angle);
        // The first heavy (41, -450), the next three (37, -380), the rest (69, -320).
        let (tumble, gravity) = if i == 0 {
            (41, -450)
        } else if i < 4 {
            (37, -380)
        } else {
            (69, -320)
        };
        let pitch = (r.zero_one() * 32767.0) as i16;
        let yaw = (r.zero_one() * 32767.0) as i16;
        want.push((pos, vel, tumble, gravity, pitch, yaw));
    }
    rock.spawn_fragments(&mut w, 1);
    w.actors.put_back(h, act);
    assert_eq!(w.rand, r, "the Rand calls, in the C's order");
    for (i, &(pos, vel, tumble, gravity, pitch, yaw)) in want.iter().enumerate() {
        let e = &w.effect_ss.table[i];
        // (pos, velocity, &world.pos, gravity, tumble, 30, 5, 0, scales[i], 5, 2, 70,
        // KAKERA_COLOR_WHITE, OBJECT_GAMEPLAY_FIELD_KEEP, gSilverRockFragmentsDL).
        assert_eq!((e.life, e.pos, e.velocity, e.vec), (70, pos, vel, p), "piece {i}");
        assert_eq!(e.gfx, Some(("gameplay_field_keep", "gSilverRockFragmentsDL")));
        let rg = &e.regs;
        assert_eq!((rg[k::R_GRAVITY], rg[k::R_REG4], rg[k::R_REG5], rg[k::R_REG6], rg[k::R_SCALE]), (gravity, tumble, 30, 5, en_ishi::FRAGMENT_SCALES_LARGE[i]), "piece {i}");
        assert_eq!((rg[k::R_REG8], rg[k::R_REG9], rg[k::R_PITCH], rg[k::R_YAW], rg[k::R_COLOR_IDX]), (5, 2, pitch, yaw, KAKERA_COLOR_WHITE), "piece {i}");
    }
    // EffectSsKakera_Draw: opaque (rReg4 bit 7 clear), the white of colors[0] as the primitive
    // colour, at its position turned by yaw then pitch (hundredths of a radian), scale / 256.
    w.effect_draw_all();
    let e = &w.effect_ss.table[0];
    let name = oot_game::pack::keys::bake("Effect_Ss_Kakera/gSilverRockFragmentsDL");
    let m = glam::Mat4::from_translation(e.pos)
        * glam::Mat4::from_rotation_y(e.regs[k::R_YAW] as f32 * 0.01)
        * glam::Mat4::from_rotation_x(e.regs[k::R_PITCH] as f32 * 0.01)
        * glam::Mat4::from_scale(Vec3::splat(145.0 / 256.0));
    let c = w.effect_draws.opa.iter().find(|c| c.mesh.name == name && c.transform == m).expect("the first piece's draw");
    assert_eq!(c.params.segments.as_ref().unwrap().prim[0x0E], Some([255, 255, 255, 255]));
    // Every fragment list the ported callers spawn is baked (pack format 21), with triangles.
    for (file, symbol) in k::KAKERA_DLISTS {
        let bake: eng_gfx::DrawList = a.pack.assets.get(&oot_game::pack::keys::bake(&k::bake_name(symbol))).unwrap_or_else(|e| panic!("{file} {symbol}: {e:#}"));
        assert!(bake.triangle_count() > 0, "{symbol}");
    }
}

#[test]
fn a_hammer_hit_breaks_a_small_rock() {
    let Some(a) = assets() else { return };
    let mut w = village(&a);
    let h = a_rock(&w);
    let p = w.actors.actor(h).unwrap().world_pos;
    w.place_player(p + Vec3::new(0.0, 0.0, -60.0), 0);
    idle(&mut w, 1);
    // The hammer's hit (DMG_HAMMER_SWING: in DMG_HAMMER | DMG_EXPLOSIVE) as the collision check
    // leaves it.
    let player = w.player.unwrap();
    {
        let rock = w.actors.downcast_mut::<EnIshi>(h).unwrap();
        rock.collider.base.ac_flags |= cc::AC_HIT;
        rock.collider.info.ac_hit_elem = Some(cc::HitElem {
            elem: cc::ElemRef { col: cc::ColliderRef { actor: player, id: 0 }, elem: 0 },
            at_dmg_info: cc::ColliderElementDamageInfoAT { dmg_flags: cc::DMG_HAMMER_SWING, hit_special_effect: cc::HIT_SPECIAL_EFFECT_NONE, damage: 2 },
            ac_dmg_info: Default::default(),
            elem_material: 0,
        });
    }
    idle(&mut w, 1);
    // EnIshi_Wait: the drop, NA_SE_EV_ROCK_BROKEN, six pieces (40 frames), four puffs (10), gone.
    assert!(w.actors.actor(h).is_none_or(|a| a.killed));
    assert!(sfx_on(&w, w.audio.frames, oot_game::audio::sfx::NA_SE_EV_ROCK_BROKEN));
    let chips = kakera_of(&w, "gFieldKakeraDL");
    assert_eq!(chips.len(), 6);
    assert!(chips.iter().all(|(_, e)| e.life == 39 && e.regs[k::R_GRAVITY] == -420));
    let mut scales: Vec<i16> = chips.iter().map(|(_, e)| e.regs[k::R_SCALE]).collect();
    scales.sort();
    assert_eq!(scales, vec![5, 7, 9, 11, 13, 16]);
    assert_eq!(w.effect_ss.table.iter().filter(|e| e.ty == EFFECT_SS_DUST && e.life == 9).count(), 4);
}

#[test]
fn the_boulders_break_follows_the_c() {
    let Some(a) = assets() else { return };
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_0").expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    idle(&mut w, 20);
    w.room_request(2);
    idle(&mut w, 1);
    w.room_change_done();
    w.place_player(Vec3::new(-785.0, 120.0, 1420.0), 0);
    let mut n = 0;
    let h = loop {
        if let Some(h) = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnGoroiwa>(h).is_some()) {
            break h;
        }
        idle(&mut w, 1);
        n += 1;
        assert!(n < 10, "no boulder");
    };
    // EnGoroiwa_SpawnFragments (at a breaking path's end; Kokiri Forest's never breaks): bit 10
    // set, so yOffsets[1] 59.5. For each of 16 a turn of 0x4E20 apart: angle2 from Rand, x from
    // Rand × 50 × sin1 × sin(angle2), y (Rand - 0.5) × 100 × sin2 + 59.5, z from Rand × 50 × cos1
    // × sin(angle2), the velocity a fifth across and Rand × 15 + 2 up, the scale Rand × 7 + 1;
    // objId 1 (gameplay_keep): unchecked. Then func_80033480 twice: six puffs 80 across (70 big)
    // and six 90 across (110 big).
    let p = w.actors.actor(h).unwrap().world_pos;
    w.effect_ss = Default::default();
    let mut r = w.rand;
    let mut want = Vec::new();
    let mut angle1: i16 = 0;
    for _ in 0..16 {
        let (sin1, cos1) = (sin_s(angle1), cos_s(angle1));
        let angle2 = (r.zero_one() * 65535.0) as i32 as i16;
        let x = r.zero_one() * 50.0 * sin1 * sin_s(angle2);
        let sin2 = sin_s(angle2);
        let y = (r.zero_one() - 0.5) * 100.0 * sin2 + 59.5;
        let z = r.zero_one() * 50.0 * cos1 * sin_s(angle2);
        let vel = Vec3::new(x * 0.2, r.zero_one() * 15.0 + 2.0, z * 0.2);
        let pos = Vec3::new(x + p.x, y + p.y, z + p.z);
        let scale = (r.zero_one() * 7.0 + 1.0) as i16;
        let pitch = (r.zero_one() * 32767.0) as i16;
        let yaw = (r.zero_one() * 32767.0) as i16;
        want.push((pos, vel, scale, pitch, yaw));
        angle1 = angle1.wrapping_add(0x4E20);
    }
    let centre = Vec3::new(p.x, p.y + 59.5, p.z);
    let mut puffs = Vec::new();
    for (range, base) in [(80.0, 70i16), (90.0, 110)] {
        for _ in 0..6 {
            let pos = Vec3::new(centre.x + (r.zero_one() - 0.5) * range, centre.y + (r.zero_one() - 0.5) * range, centre.z + (r.zero_one() - 0.5) * range);
            let s = (((base as f32 * r.zero_one()) * 0.2) as i16).wrapping_add(base);
            r.zero_one();
            puffs.push((pos, s));
        }
    }
    let mut act = w.actors.take(h).unwrap();
    act.as_any_mut().downcast_mut::<EnGoroiwa>().unwrap().spawn_fragments(&mut w);
    w.actors.put_back(h, act);
    assert_eq!(w.rand, r, "the Rand calls, in the C's order");
    for (i, &(pos, vel, scale, pitch, yaw)) in want.iter().enumerate() {
        let e = &w.effect_ss.table[i];
        // (pos, velocity, pos, -340, 33, 28, 2, 0, scale, 1, 0, 70, KAKERA_COLOR_NONE, 1,
        // gBoulderFragmentsDL).
        assert_eq!((e.ty, e.life, e.pos, e.velocity, e.vec), (EFFECT_SS_KAKERA, 70, pos, vel, pos), "fragment {i}");
        assert_eq!(e.gfx, Some(("gameplay_keep", "gBoulderFragmentsDL")));
        let rg = &e.regs;
        assert_eq!((rg[k::R_GRAVITY], rg[k::R_REG4], rg[k::R_REG5], rg[k::R_REG6], rg[k::R_SCALE]), (-340, 33, 28, 2, scale), "fragment {i}");
        assert_eq!((rg[k::R_REG8], rg[k::R_REG9], rg[k::R_PITCH], rg[k::R_YAW], rg[k::R_OBJ_ID]), (1, 0, pitch, yaw, KAKERA_OBJECT_DEFAULT), "fragment {i}");
    }
    for (j, &(pos, s)) in puffs.iter().enumerate() {
        let e = &w.effect_ss.table[16 + j];
        assert_eq!((e.ty, e.pos, e.regs[dust::R_SCALE]), (EFFECT_SS_DUST, pos, s), "puff {j}");
    }
}

/// A fragment as the test follows it: the fields `EffectSs_Update` and the overlay touch.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Model {
    pos: Vec3,
    velocity: Vec3,
    accel: Vec3,
    vec: Vec3,
    life: i16,
    regs: [i16; 13],
}

/// `func_809A9818`.
fn uniform(r: &mut Rand, a: f32, b: f32) -> f32 {
    let t = r.zero_one() * b;
    ((t * 2.0) - b) + a
}

/// One `EffectSs_UpdateAll` turn of a fragment, from the C: the life down (gone below 0), the
/// velocity and position stepped, then `EffectSsKakera_Update` (its tumble, `func_809A9C10`'s drag
/// with z taking y's terms, `func_809AA0EC`'s forces and gravity, `func_809AA230`'s floor). The
/// floor under it is flat at `floor` (`BgCheck_SphVsFirstPoly`: the sphere's truncated centre
/// within its radius of it); Player's floor is `player_floor`. Returns false once it's gone.
fn model_step(m: &mut Model, r: &mut Rand, floor: f32, player_floor: f32) -> bool {
    m.life -= 1;
    if m.life < 0 {
        return false;
    }
    m.velocity += m.accel;
    m.pos += m.velocity;
    let g = &mut m.regs;
    let (dp, dy) = match ((g[k::R_REG4] >> 5) & 3) << 5 {
        0x20 => (0xB, 3),
        0x40 => (0x41, 0xB),
        0x60 => (0x9B, 0x1F),
        _ => (0, 0),
    };
    g[k::R_PITCH] = g[k::R_PITCH].wrapping_add(dp);
    g[k::R_YAW] = g[k::R_YAW].wrapping_add(dy);
    // func_809A9C10.
    let (lin, quad, jit) = (g[k::R_REG5] as f32 / 1024.0, g[k::R_REG6] as f32 / 1024.0, (g[k::R_REG9] as f32 / 1024.0) * 4.0);
    let tx = m.velocity.x - uniform(r, 0.0, jit);
    let ty = m.velocity.y - uniform(r, 0.0, jit);
    let tz = m.velocity.z - uniform(r, 0.0, jit);
    m.velocity.x -= if tx > 0.0 { (tx * lin) + (tx * tx * quad) } else { (tx * lin) - (tx * tx * quad) };
    let (f0, f2) = (ty * lin, ty * ty * quad);
    m.velocity.y -= if ty > 0.0 { f0 + f2 } else { f0 - f2 };
    // @bug (game): z less y's terms.
    m.velocity.z -= if tz > 0.0 { f0 + f2 } else { f0 - f2 };
    // func_809AA0EC.
    m.accel = Vec3::ZERO;
    let d = m.pos - m.vec;
    let dist = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
    if dist > 1000.0 {
        m.life = 0;
    } else {
        let r0 = m.regs[k::R_REG0];
        if r0 != 0 {
            let inv = if dist > 1.0 { 1.0 / dist } else { 1.0 };
            // The swirl: D_809AA558 {0.05, 1}.
            let s = (r0 & 3) as usize;
            if s != 0 {
                let kf = [0.05f32, 1.0, 4.0][s - 1];
                m.accel.x += (kf * d.z) * inv;
                m.accel.z -= (kf * d.x) * inv;
            }
            // The push: D_809AA560 {4, 0.1, 0.3, 0.9, -0.1, -0.3, -0.9}.
            let u = ((r0 >> 2) & 7) as usize;
            if u != 0 {
                m.accel.y += [4.0f32, 0.1, 0.3, 0.9, -0.1, -0.3, -0.9, 0.1][u];
            }
            // The pull: D_809AA57C {0.1, 1, 6}.
            let p = ((r0 >> 5) & 3) as usize;
            if p != 0 {
                let kf = [0.1f32, 1.0, 6.0][p - 1];
                m.accel.x -= (d.x * kf) * inv;
                m.accel.z -= (d.z * kf) * inv;
            }
            // The fade: D_809AA588[(r0 >> 7) & 0xF] on D_809AA530.
            let fi = ((r0 >> 7) & 0xF) as usize;
            let table = [1.0f32, 100.0, 40.0, 5.0, 100.0, 40.0, 5.0, 100.0, 40.0, 5.0];
            let mut t = match fi {
                0 => 1.0,
                1..=3 => {
                    if table[fi] < dist {
                        table[fi] / dist
                    } else {
                        1.0
                    }
                }
                _ => {
                    let sq = dist * dist;
                    if table[fi] < sq { table[fi] / sq } else { 1.0 }
                }
            };
            t = uniform(r, t, (m.regs[k::R_REG9] as f32 * t) / 1024.0);
            m.accel *= t;
            m.accel += Vec3::splat(t * 0.01);
        }
        m.accel.y += m.regs[k::R_GRAVITY] as f32 / 256.0;
    }
    // func_809AA230.
    let g = &mut m.regs;
    let size = ((g[k::R_REG4] >> 2) & 3) as usize;
    if g[k::R_REG8] == 0 {
        if (g[k::R_REG4] >> 4) & 1 != 0 {
            if m.pos.y <= player_floor - size as f32 {
                g[k::R_REG9] = 0;
                g[k::R_REG0] = 0;
                g[k::R_REG4] &= !0x60;
                m.accel = Vec3::ZERO;
                m.velocity = Vec3::ZERO;
                g[k::R_REG5] = 0;
                g[k::R_GRAVITY] = 0;
            }
        } else if m.pos.y <= (player_floor - size as f32) - 600.0 {
            m.life = 0;
        }
    } else {
        let radius = [10.0f32, 20.0, 40.0][size];
        match g[k::R_REG4] & 3 {
            0 => g[k::R_REG8] = 0,
            1 => {
                if m.velocity.y < 0.0 && ((m.pos.y as i16) as f32 - floor).abs() <= radius {
                    m.velocity.x *= uniform(r, 0.9, 0.2);
                    m.velocity.y *= -0.8;
                    m.velocity.z *= uniform(r, 0.9, 0.2);
                    if g[k::R_REG8] > 0 {
                        g[k::R_REG8] -= 1;
                    }
                }
            }
            _ => {}
        }
    }
    true
}

fn model_of(e: &EffectSs) -> Model {
    Model { pos: e.pos, velocity: e.velocity, accel: e.accel, vec: e.vec, life: e.life, regs: e.regs }
}

/// Follows slot 0 frame by frame (`EffectSs_UpdateAll` alone) against the model, until it's
/// gone; the frames it lived, the bounces it made (`rReg8` counted down) and its last state.
fn fly(w: &mut PlayState, floor: f32) -> (u32, u32, Model) {
    let player_floor = w.player().actor.floor_height;
    let mut m = model_of(&w.effect_ss.table[0]);
    let mut r = w.rand;
    let (mut frames, mut bounces) = (0, 0);
    let mut last = m;
    loop {
        let alive = model_step(&mut m, &mut r, floor, player_floor);
        let before = w.effect_ss.table[0].regs[k::R_REG8];
        w.effect_ss_update_all();
        assert_eq!(w.rand, r, "frame {frames}: the Rand calls");
        let e = &w.effect_ss.table[0];
        if !alive {
            assert_eq!(e.life, -1, "gone on frame {frames}");
            return (frames, bounces, last);
        }
        assert_eq!(model_of(e), m, "frame {frames}");
        last = m;
        if e.regs[k::R_REG8] < before {
            bounces += 1;
        }
        frames += 1;
        assert!(frames < 300);
    }
}

#[test]
fn a_rock_chip_flies_bounces_three_times_and_lies_on_the_floor() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    // The floor at (40, 0, -200) is flat (y 0) round it.
    let at = Vec3::new(40.0, 0.0, -200.0);
    let (floor, poly) = w.col.entity_raycast_down(at + Vec3::Y * 50.0);
    assert_eq!(floor, 0.0);
    assert_eq!(w.col.poly(poly.unwrap()).normal[1], 32767);
    assert_eq!(w.player().actor.floor_height, 0.0);
    w.effect_ss = Default::default();
    // A small rock's chip (EnIshi_SpawnFragmentsSmall's parameters): 8 up, flying (3, 4, -2):
    // gravity -420/256, the bounce test (rReg4 65: mode 1, size 0: a radius of 10; tumbling
    // 0x41/0xB), drag 30 and 5 with a jitter of 10, three bounces, 40 frames.
    let pos = at + Vec3::Y * 8.0;
    w.with_ss(|s| {
        s.kakera_spawn(pos, Vec3::new(3.0, 4.0, -2.0), pos, -420, 65, 30, 5, 0, 16, 3, 10, 40, KAKERA_COLOR_NONE, OBJECT_GAMEPLAY_FIELD_KEEP, Some(("gameplay_field_keep", "gFieldKakeraDL")))
    });
    let (frames, bounces, last) = fly(&mut w, floor);
    // Its 40 frames: updated with its life at 39 down to 0 (40 turns), gone on the 41st. Its
    // three bounces used, it falls through the floor (rReg4 bit 4 clear) but not the 600 past
    // Link's floor that would end it sooner.
    assert_eq!(frames, 40);
    assert_eq!((bounces, last.regs[k::R_REG8]), (3, 0));
    assert!(last.pos.y <= 0.0 && last.pos.y > -600.0);
}

#[test]
fn a_fragment_under_forces_stops_on_links_floor() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    w.effect_ss = Default::default();
    // No ported caller sets rReg0; one with every force: a swirl (1: 0.05), a push down (5:
    // -0.1), a pull (1: 0.1), faded by distance squared (4: 100 / dist²), a jitter of 16;
    // rReg4 0x30: stop on Link's floor (bit 4), slow tumble; no bounces. vec 50 off.
    let pos = Vec3::new(40.0, 60.0, -200.0);
    let r0 = 1 | (5 << 2) | (1 << 5) | (4 << 7);
    w.with_ss(|s| {
        s.kakera_spawn(
            pos,
            Vec3::new(2.0, 3.0, 1.0),
            pos + Vec3::new(50.0, 0.0, 0.0),
            -256,
            0x30,
            20,
            2,
            r0,
            30,
            0,
            16,
            120,
            KAKERA_COLOR_NONE,
            OBJECT_GAMEPLAY_FIELD_KEEP,
            Some(("gameplay_field_keep", "gFieldKakeraDL")),
        )
    });
    let (frames, _, last) = fly(&mut w, 0.0);
    // Its 120 frames (stopped, it isn't ended).
    assert_eq!(frames, 120);
    // Stopped on Link's floor (0, less size 0): rReg0, rReg9, rReg5 and gravity zeroed, no
    // tumble, still.
    let g = last.regs;
    assert_eq!((g[k::R_REG0], g[k::R_REG9], g[k::R_REG5], g[k::R_GRAVITY], g[k::R_REG4]), (0, 0, 0, 0, 0x10));
    assert_eq!((last.velocity, last.accel), (Vec3::ZERO, Vec3::ZERO));
    assert!(last.pos.y <= 0.0);
}

#[test]
fn a_fragment_whose_object_isnt_loaded_goes_at_once() {
    let Some(a) = assets() else { return };
    let mut w = deku_tree(&a);
    w.effect_ss = Default::default();
    // OBJECT_D_LIFT isn't in room 0's banks: func_809A9BA8 gives slot -1 and life 0 (and no
    // draw, but the init sets the draw after it); it's gone at the next EffectSs_UpdateAll,
    // before any draw.
    assert!(w.object_ctx.get_index(oot_actors::obj_lift::OBJECT_D_LIFT).is_none());
    let pos = Vec3::new(40.0, 60.0, -200.0);
    w.with_ss(|s| s.kakera_spawn(pos, Vec3::ZERO, pos, -256, 64, 15, 15, 0, 7, 0, 32, 50, KAKERA_COLOR_NONE, oot_actors::obj_lift::OBJECT_D_LIFT, Some(("object_d_lift", "gCollapsingPlatformDL"))));
    let e = &w.effect_ss.table[0];
    assert_eq!((e.life, e.regs[k::R_OBJECT_SLOT], e.draw), (0, -1, Some(SsDraw::Kakera)));
    w.effect_ss_update_all();
    assert_eq!(w.effect_ss.table[0].life, -1);
}
