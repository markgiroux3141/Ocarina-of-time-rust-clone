//! King Dodongo (GAME-09 milestone 4) against the C: `Boss_Dodongo` (`z_boss_dodongo.c`) in his
//! room (`ENTR_DODONGOS_CAVERN_BOSS_0`, room 1), his fire breath `En_Bdfire` (`z_en_bdfire.c`),
//! the lava's `Effect_Ss_G_Magma` and `G_Magma2`, his room texture, and `Door_Warp1`'s branch for
//! his room. No fight is scripted: his states are set directly where a test is about one of them,
//! and the hits a sword or a seed would make are injected into his spheres.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::boss_dodongo::{self as kd, Action, BossDodongo};
use oot_actors::door_warp1::{Action as WarpAction, DoorWarp1};
use oot_actors::en_bdfire::{self as bdfire, EnBdfire};
use oot_actors::item_b_heart::ItemBHeart;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_HOSTILE};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::*;
use oot_game::camera::{CAM_ID_MAIN, CAM_STAT_WAIT};
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

/// `SCENE_DODONGOS_CAVERN_BOSS` (`scene_table.h`).
const SCENE: usize = 0x12;
/// Over the hole the cracked floor (`Bg_Breakwall` 0x4001) leaves.
const OVER_THE_HOLE: Vec3 = Vec3::new(-890.0, -700.0, -2790.0);

/// His room with `preset` (and `edit` on the save), the sound log on, three frames run (the room's
/// objects in).
fn boss_room(a: &Arc<GameAssets>, preset: &str, edit: impl Fn(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DODONGOS_CAVERN_BOSS_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    edit(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 3);
    w
}

/// The room with its floor bombed (switch flag 1), Link placed over the hole.
fn floor_open(a: &Arc<GameAssets>, began: bool) -> PlayState {
    let mut w = boss_room(a, "cavern-boss", |s| {
        s.scene_flags[SCENE].swch |= 1 << 1;
        if began {
            s.set_event_chk_inf(kd::EVENTCHKINF_BEGAN_KING_DODONGO_BATTLE);
        }
    });
    w.place_player(OVER_THE_HOLE, 0x4000);
    w
}

fn kd_h(w: &PlayState) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<BossDodongo>(h).is_some_and(|b| !b.actor.killed)).expect("Boss_Dodongo")
}

fn kd(w: &PlayState) -> &BossDodongo {
    w.actors.downcast::<BossDodongo>(kd_h(w)).unwrap()
}

fn kd_mut(w: &mut PlayState) -> &mut BossDodongo {
    let h = kd_h(w);
    w.actors.downcast_mut::<BossDodongo>(h).unwrap()
}

fn sfx_any(w: &PlayState, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(_, s, _)| s == id)
}

/// Idles until `cond`, at most `n` frames; the frames it took.
fn until(w: &mut PlayState, n: usize, cond: impl Fn(&PlayState) -> bool) -> usize {
    for i in 0..n {
        if cond(w) {
            return i;
        }
        idle(w, 1);
    }
    panic!("not within {n} frames");
}

/// The intro run to its end (a second try: it starts beside him), the fight on.
fn fighting(a: &Arc<GameAssets>) -> PlayState {
    let mut w = floor_open(a, true);
    until(&mut w, 600, |w| kd(w).action == Action::Walk);
    w
}

/// An AC hit with `dmg_flags` on his sphere `elem`, as the collision check leaves it.
fn hit(w: &mut PlayState, elem: usize, dmg_flags: u32) {
    let player = w.player.unwrap();
    let e = &mut kd_mut(w).collider.elements[elem].info;
    e.ac_elem_flags |= cc::ACELEM_HIT;
    e.ac_hit_elem = Some(cc::HitElem {
        elem: cc::ElemRef { col: cc::ColliderRef { actor: player, id: 0 }, elem: 0 },
        at_dmg_info: cc::ColliderElementDamageInfoAT { dmg_flags, hit_special_effect: cc::HIT_SPECIAL_EFFECT_NONE, damage: 1 },
        ac_dmg_info: Default::default(),
        elem_material: 0,
    });
}

#[test]
fn he_waits_in_the_west_until_link_falls_through_the_floor() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, "cavern-boss", |_| {});
    // BossDodongo_Init: 12 health, scale 0.01, shape.yOffset 9200, not targetable until his walk;
    // at his placement (-1386, -1504, -3339) in the intro's state 0.
    let b = kd(&w);
    assert_eq!((b.action, b.cs_state, b.health), (Action::IntroCutscene, 0, 12));
    assert_eq!(b.actor.id, kd::ACTOR_EN_DODONGO, "@bug (game): his profile's id");
    assert_eq!(b.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!((b.unk_224, b.unk_228, b.color_filter_max), (2.0, 9200.0, 1000.0));
    // Link on the uncracked floor (y -744): nothing starts.
    idle(&mut w, 30);
    assert_eq!(kd(&w).cs_state, 0);
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
}

#[test]
fn his_intro_follows_link_down_then_him_walking_in_and_roaring() {
    let Some(a) = assets() else { return };
    let mut w = floor_open(&a, false);
    // State 0 -> 1 the frame Link is below -1223.76; state 1 falls through into 2: the manual
    // cutscene, his sub camera active (the main one waiting), Link put at y -1023.76, him at the
    // west wall (-1390, -3374) going to corner 1.
    until(&mut w, 100, |w| kd(w).cs_state == 2);
    let b = kd(&w);
    assert_ne!(b.sub_cam_id, 0);
    assert_eq!(w.active_cam_id, b.sub_cam_id);
    assert_eq!(w.camera(CAM_ID_MAIN).unwrap().status, CAM_STAT_WAIT);
    // (Its timers count down before the switch: these are this frame's.)
    assert_eq!((b.unk_196, b.unk_198, b.unk_1a0), (0x3C, 160, 1));
    assert_eq!(w.player().cs_mode, 1);
    assert_eq!((w.player().actor.world_pos.x, w.player().actor.world_pos.z), (-890.0, -2804.0));
    // unk_198 110: PLAYER_CSACTION_9 (the landing turned to look); 5: 12 (a step back).
    until(&mut w, 100, |w| kd(w).unk_198 == 110);
    assert_eq!(w.player().cs_mode, 9);
    until(&mut w, 200, |w| kd(w).unk_198 == 5);
    assert_eq!(w.player().cs_mode, 12);
    // unk_198 0: state 3, the camera at him (his steps shake it) and its up tilted by unk_20C
    // (Play_SetCameraAtEyeUp: (unk_20C, 1, unk_20C)).
    until(&mut w, 30, |w| kd(w).cs_state == 3);
    idle(&mut w, 20);
    let (sub, tilt) = (kd(&w).sub_cam_id, kd(&w).unk_20c);
    assert_ne!(tilt, 0.0);
    let cam = w.camera(sub).unwrap();
    // (BossDodongo_Update steps unk_20C 0.001 towards 0 after the intro set the camera's up.)
    let set = tilt + 0.001;
    assert!((cam.up - Vec3::new(set, 1.0, set)).length() < 1e-6, "{:?}", cam.up);
    assert_ne!(cam.shaken_view().up, Vec3::Y, "the view rolls");
    // Within 200 of Link across: state 4, his roar at unk_198 100, the title card and the fire
    // boss's music at 90 (the first time), the camera back at 0 with the battle begun.
    until(&mut w, 400, |w| kd(w).cs_state == 4);
    until(&mut w, 100, |w| kd(w).unk_198 == 0x5A - 1);
    assert!(sfx_any(&w, NA_SE_EN_DODO_K_OTAKEBI));
    assert_eq!(w.title_ctx.texture.as_deref(), Some(oot_game::title_card::boss_sprite_name("object_kingdodongo", 0x17410, 0).as_str()));
    until(&mut w, 200, |w| kd(w).action == Action::Walk);
    let b = kd(&w);
    assert_eq!((b.sub_cam_id, b.unk_1bc, b.unk_1da), (0, 0, 50));
    assert_ne!(b.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert!(w.save.get_event_chk_inf(kd::EVENTCHKINF_BEGAN_KING_DODONGO_BATTLE));
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
    assert_eq!(w.player().cs_mode, 7);
    // The view's up is the world's again.
    idle(&mut w, 2);
    assert_eq!(w.active_camera().shaken_view().up, Vec3::Y);
}

#[test]
fn a_second_try_starts_him_beside_link_without_a_title_card() {
    let Some(a) = assets() else { return };
    let mut w = floor_open(&a, true);
    until(&mut w, 100, |w| kd(w).cs_state == 2);
    // EVENTCHKINF_BEGAN_KING_DODONGO_BATTLE: at unk_198 100 he's put at (-1114, -2804) facing east
    // (0x3FFF), state 4 with 150 frames.
    until(&mut w, 100, |w| kd(w).cs_state == 4);
    let b = kd(&w);
    assert_eq!((b.actor.world_pos.x, b.actor.world_pos.z, b.actor.world_rot.y, b.unk_198), (-1114.0, -2804.0, 0x3FFF, 150));
    until(&mut w, 200, |w| kd(w).action == Action::Walk);
    assert_eq!(w.title_ctx.texture, None);
}

#[test]
fn inhaling_he_eats_an_explosive_by_his_mouth_and_it_blows_inside_him() {
    let Some(a) = assets() else { return };
    let mut w = fighting(&a);
    {
        let b = kd_mut(&mut w);
        b.action = Action::Inhale;
        b.unk_1da = 100;
        b.unk_1ac = 0;
    }
    // BossDodongo_Inhale: only from his 21st frame inhaling (unk_1AC > 20).
    until(&mut w, 30, |w| kd(w).unk_1ac == 20);
    // A bomb (ACTORCAT_EXPLOSIVE, updated before him) within 40 of his mouth on each axis.
    let mouth = kd(&w).mouth_pos;
    let bomb = w.actor_spawn(oot_actors::en_bom::ACTOR_EN_BOM, mouth + Vec3::new(20.0, -10.0, 30.0), [0; 3], 0).unwrap();
    idle(&mut w, 1);
    assert!(w.actors.actor(bomb).is_none_or(|b| b.killed), "eaten");
    // BossDodongo_SetupExplode: 35 frames, the hit flash (unk_1C0 2).
    let b = kd(&w);
    assert_eq!((b.action, b.unk_1da, b.unk_1c0), (Action::Explode, 35, 2));
    assert!(sfx_any(&w, NA_SE_EN_DODO_K_DRINK));
    until(&mut w, 40, |w| kd(w).action == Action::LayDown);
    // The blast inside him: 2 off his health.
    assert_eq!(kd(&w).health, 10);
    assert!(sfx_any(&w, NA_SE_IT_BOMB_EXPLOSION) && sfx_any(&w, NA_SE_EN_DODO_K_DAMAGE));
}

#[test]
fn the_last_bomb_never_kills_him() {
    let Some(a) = assets() else { return };
    let mut w = fighting(&a);
    {
        let b = kd_mut(&mut w);
        b.health = 2;
        b.action = Action::Explode;
        b.unk_1da = 1;
    }
    idle(&mut w, 1);
    // `this->health -= 2; if (health <= 0) health = 1`.
    assert_eq!((kd(&w).action, kd(&w).health), (Action::LayDown, 1));
}

#[test]
fn down_his_head_takes_a_sword_and_inhaling_a_seed_stops_him() {
    let Some(a) = assets() else { return };
    let mut w = fighting(&a);
    // Walking: a slash on his head takes nothing.
    hit(&mut w, 0, cc::DMG_SLASH_KOKIRI);
    idle(&mut w, 1);
    assert_eq!(kd(&w).health, 12);
    // Down (BossDodongo_Vulnerable): a Kokiri Sword slash takes 1 (CollisionCheck_GetSwordDamage),
    // the hit flash 5, BossDodongo_SetupDamaged (100 frames); a jump slash 2.
    kd_mut(&mut w).action = Action::Vulnerable;
    kd_mut(&mut w).unk_1da = 100;
    hit(&mut w, 0, cc::DMG_SLASH_KOKIRI);
    idle(&mut w, 1);
    let b = kd(&w);
    assert_eq!((b.health, b.action, b.unk_1c0, b.unk_1da), (11, Action::Damaged, 5, 100));
    kd_mut(&mut w).action = Action::Vulnerable;
    kd_mut(&mut w).unk_1c0 = 0;
    hit(&mut w, 0, cc::DMG_JUMP_KOKIRI);
    idle(&mut w, 1);
    assert_eq!(kd(&w).health, 9);
    // Inhaling: a seed on any sphere (his tail's here) stops it: walking, 50 frames, the flash.
    kd_mut(&mut w).action = Action::Inhale;
    kd_mut(&mut w).unk_1da = 100;
    kd_mut(&mut w).unk_1c0 = 0;
    hit(&mut w, 18, cc::DMG_SLINGSHOT);
    idle(&mut w, 1);
    let b = kd(&w);
    assert_eq!((b.action, b.unk_1da, b.unk_1c0), (Action::Walk, 0x32, 2));
}

#[test]
fn his_death_cools_the_lava_and_leaves_the_heart_and_the_warp() {
    let Some(a) = assets() else { return };
    let mut w = fighting(&a);
    kd_mut(&mut w).health = 0;
    idle(&mut w, 1);
    // BossDodongo_UpdateDamage: no health, the death (not targetable, not hostile, the music
    // stopped); its state 0 takes the camera from the main one's view.
    let b = kd(&w);
    assert_eq!(b.action, Action::DeathCutscene);
    assert_eq!(b.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE), 0);
    assert!(sfx_any(&w, NA_SE_EN_DODO_K_DEAD));
    idle(&mut w, 1);
    assert_eq!(w.active_cam_id, kd(&w).sub_cam_id);
    // The lava's cover (Bg_Breakwall 0x6000) at his second death animation.
    until(&mut w, 400, |w| kd(w).cs_state == 6);
    assert!(w.actors.all().into_iter().any(|h| w.actors.actor(h).is_some_and(|x| x.params == 0x6000)));
    // The roll round the corners, then (unk_1DA 1000 down) into the lava.
    until(&mut w, 600, |w| kd(w).cs_state == 8);
    // 710: the lava starts cooling (unk_1C6) and Link is put at a side's middle.
    until(&mut w, 400, |w| kd(w).unk_1c6 != 0);
    // 820: the boss-clear music and the heart container at him.
    until(&mut w, 200, |w| w.actors.all().into_iter().any(|h| w.actors.downcast::<ItemBHeart>(h).is_some()));
    // 600: the camera back, the warp (his child, WARP_DUNGEON_CHILD at the arena's centre), the
    // room's clear flag.
    until(&mut w, 300, |w| kd(w).cs_state == 100);
    let me = kd_h(&w);
    let warp = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<DoorWarp1>(h)).expect("the blue warp");
    assert_eq!((warp.actor.params, warp.actor.world_pos, warp.actor.parent), (0, kd::ARENA_CENTER, Some(me)));
    assert!(w.flags.get_clear(1));
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
    // By then his decay has run its 256 steps (2 a frame from 854) into his object's RAM, and the
    // lava's texture has cooled to rock (20 texels a frame, 37 apart: every one of the 2048 in 103
    // frames), its wobble down from 2 by 0.01 a frame.
    let b = kd(&w);
    assert_eq!(b.unk_1cc, 256);
    assert!(w.object_ctx.written(b.actor.obj_bank_index.unwrap(), kd::DECAY_REGION).is_some());
    assert_eq!(b.lava_tex[..2048], b.rock_tex[..2048]);
    assert!(b.unk_224 < 1.0);
    // The room's floor texture is the cooled lava now (Link is below -1000: the wobble copies it).
    let floor = w.room_ram.region(kd::FLOOR_ROOM, kd::FLOOR_TEX_OFFSET).expect("written");
    assert_eq!(floor.len(), 4096);
}

#[test]
fn below_the_ledge_the_floor_texture_wobbles_from_his_lava() {
    let Some(a) = assets() else { return };
    let mut w = fighting(&a);
    idle(&mut w, 1);
    // Link below -1000: func_808C1554 every update into room 1's gDodongosCavernBossLavaFloorTex.
    let b = kd(&w);
    let mut expected = vec![0u16; 2048];
    kd::func_808c1554(&mut expected, &b.lava_tex, b.unk_19e as i32, b.unk_224);
    let floor = w.room_ram.region(kd::FLOOR_ROOM, kd::FLOOR_TEX_OFFSET).expect("written");
    let got: Vec<u16> = floor.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
    assert_eq!(got, expected);
    // The room's draws replace that texture by its source (segment 3, 0x21D8).
    let images = w.room_ram.images(kd::FLOOR_ROOM);
    assert_eq!(images.len(), 1);
    assert_eq!((images[0].source, images[0].image.width, images[0].image.height), (0x0300_21D8, 32, 64));
}

#[test]
fn the_wobble_shifts_rows_then_columns_by_sines() {
    let src: Vec<u16> = (0..2048).map(|i| i as u16).collect();
    // No amount: a copy.
    let mut dst = vec![0u16; 2048];
    kd::func_808c1554(&mut dst, &src, 0, 0.0);
    assert_eq!(dst, src);
    // Amount 2 at frame 0: row r shifted right by (s16)(sin(r * pi / 16) * 2) (row 8: 2, row 24:
    // -2; row 1: (s16)0.39 = 0), wrapping in 32; column c shifted down by the same of c, 32 a row.
    kd::func_808c1554(&mut dst, &src, 0, 2.0);
    // Column 0 isn't moved (sin 0); row 8's texel 0 went to column 2.
    assert_eq!(dst[8 * 32 + 2], src[8 * 32]);
    // Column 8 moves 2 rows down: row 0's texel at column 8 came from row 0's column 8 - 0.
    assert_eq!(dst[2 * 32 + 8], src[8]);
}

#[test]
fn beaten_his_init_leaves_the_warp_the_heart_the_cover_and_rock() {
    let Some(a) = assets() else { return };
    let w = boss_room(&a, "cavern-boss-beaten", |_| {});
    // Flags_GetClear: killed at once; the warp (his child), the lava's cover and the heart.
    assert!(!w.actors.all().into_iter().any(|h| w.actors.downcast::<BossDodongo>(h).is_some_and(|b| !b.actor.killed)));
    let warp = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<DoorWarp1>(h)).expect("warp");
    assert_eq!((warp.actor.params, warp.actor.world_pos), (0, kd::ARENA_CENTER));
    let heart = w.actors.all().into_iter().find_map(|h| w.actors.downcast::<ItemBHeart>(h)).expect("heart");
    assert_eq!(heart.actor.home_pos, Vec3::new(-690.0, -1523.76, -3304.0));
    assert!(w.actors.all().into_iter().any(|h| w.actors.actor(h).is_some_and(|x| x.params == 0x6000)));
    // The floor's texture is sLavaFloorRockTex's 2048 texels.
    let rock = a.pack.texture("ovl_Boss_Dodongo", "sLavaFloorRockTex").unwrap();
    assert_eq!(w.room_ram.region(kd::FLOOR_ROOM, kd::FLOOR_TEX_OFFSET), Some(&rock.texels[..4096]));
}

#[test]
fn the_screen_fill_follows_unk_244_though_nothing_sets_it() {
    let Some(a) = assets() else { return };
    let mut w = fighting(&a);
    assert_eq!(w.mreg_fill[0], 0);
    kd_mut(&mut w).unk_244 = 100.0;
    idle(&mut w, 1);
    // MREG(64) 1, (255, 80, 0), alpha (u8)unk_244; then unk_244 steps down 2 a frame.
    assert_eq!(w.mreg_fill, [1, 255, 80, 0, 100]);
    assert_eq!(w.draw_fills().0, Some([255, 80, 0, 100]));
    idle(&mut w, 1);
    assert_eq!(w.mreg_fill[4], 98);
}

#[test]
fn the_decay_table_reads_past_its_end() {
    assert_eq!(kd::D_808C7000.len(), 254);
    assert_eq!(kd::D_808C7000.iter().filter(|&&v| v != 0).count(), 137);
    // The overlay's next bytes: two of padding, then sJntSphElementsInit[0]'s ELEM_MATERIAL_UNK3.
    assert_eq!((kd::decay_table(254), kd::decay_table(255), kd::decay_table(256)), (0, 0, 3));
}

#[test]
fn his_fire_balls_roll_turn_at_the_walls_and_set_link_burning() {
    let Some(a) = assets() else { return };
    let mut w = fighting(&a);
    let me = kd_h(&w);
    // EnBdfire_Init for params 3: speed 30, life 25 - (s32)2.4 = 23, scale target 4.2 - 0.45,
    // alpha 255 - 30.
    let pos = Vec3::new(-1300.0, -1504.0, -3300.0);
    let h = w.actor_spawn(bdfire::ACTOR_EN_BDFIRE, pos, [0, -0x4000, 0], 3).unwrap();
    idle(&mut w, 1);
    w.actors.actor_mut(h).unwrap().parent = Some(me);
    let f = w.actors.downcast::<EnBdfire>(h).unwrap();
    assert_eq!(f.actor.id, 0, "@bug (game): its profile's id");
    assert!((f.unk_188 - 3.7500002).abs() < 1e-5 && f.unk_18c == 225.0);
    // Rolling west (yaw -0x4000) at 30 (45 a frame: Actor_UpdatePos's 1.5): past x -1390 it's
    // put back on the wall and turned a quarter (King Dodongo's way round, unk_1A2 0: +0x4000).
    until(&mut w, 10, |w| w.actors.downcast::<EnBdfire>(h).is_some_and(|f| f.actor.world_rot.y == 0));
    // Link within 130 × its scale / 4.2 and not burning: his body parts' flame timers, a knockdown.
    let fp = w.actors.actor(h).unwrap().world_pos;
    w.place_player(fp + Vec3::new(0.0, 0.0, 30.0), 0);
    idle(&mut w, 1);
    let p = w.actors.downcast::<oot_actors::player::Player>(w.player.unwrap()).unwrap();
    assert!(p.body_is_burning);
}

#[test]
fn his_flame_follows_his_mouth_and_dies_once_he_stops_inhaling() {
    let Some(a) = assets() else { return };
    let mut w = fighting(&a);
    let me = kd_h(&w);
    kd_mut(&mut w).action = Action::Inhale;
    kd_mut(&mut w).unk_1da = 100;
    let h = w.actor_spawn(bdfire::ACTOR_EN_BDFIRE, Vec3::new(-900.0, -1500.0, -2900.0), [0; 3], -1).unwrap();
    idle(&mut w, 1);
    w.actors.actor_mut(h).unwrap().parent = Some(me);
    idle(&mut w, 2);
    // func_809BC2A4: at his firePos (updated before him: last frame's), x scale 2.8.
    let fire_pos = kd(&w).fire_pos;
    let f = w.actors.downcast::<EnBdfire>(h).unwrap();
    assert_eq!(f.action, bdfire::Action::Flame);
    assert!(f.actor.world_pos.distance(fire_pos) < 30.0);
    // He stops: it narrows and fades (alpha 0 at once: killed).
    kd_mut(&mut w).action = Action::Walk;
    idle(&mut w, 3);
    assert!(w.actors.actor(h).is_none_or(|f| f.killed));
}

#[test]
fn the_lava_bubbles_and_splashes() {
    let Some(a) = assets() else { return };
    let mut w = fighting(&a);
    w.effect_ss = Default::default();
    let pos = Vec3::new(-890.0, -1523.76, -3304.0);
    w.with_ss(|s| s.g_magma_spawn(pos));
    w.with_ss(|s| s.g_magma2_spawn(pos, [255, 255, 0, 255], [255, 0, 0, 255], 10, 0, 100));
    let live: Vec<_> = w.effect_ss.table.iter().filter(|e| e.life >= 0).cloned().collect();
    assert_eq!(live.len(), 2);
    // EffectSsGMagma_Init: 16 frames, scale 200 to 299, the texture index up 50 a frame.
    let m = live.iter().find(|e| e.ty == oot_game::effect::EFFECT_SS_G_MAGMA).unwrap();
    assert_eq!((m.life, m.regs[2]), (16, 50));
    assert!((200..300).contains(&m.regs[1]));
    // EffectSsGMagma2_Init (with OBJECT_KINGDODONGO loaded): 100 frames.
    let m2 = live.iter().find(|e| e.ty == oot_game::effect::EFFECT_SS_G_MAGMA2).unwrap();
    assert_eq!(m2.life, 100);
    // At rate 10 a texture a frame, prim green and env red down 26 each (draw mode 0); the tenth
    // ends it.
    let mut e = m2.clone();
    oot_game::effect::g_magma2::update(&mut e);
    assert_eq!((e.regs[6], e.regs[1], e.regs[3]), (1, 255 - 26, 255 - 26));
    for _ in 0..9 {
        oot_game::effect::g_magma2::update(&mut e);
    }
    assert_eq!(e.life, 0);
}

#[test]
fn a_second_time_his_warp_goes_to_the_trail_s_spawn_5() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, "goron-ruby", |_| {});
    // DoorWarp1_ChildWarpOut with EVENTCHKINF_25: ENTR_DEATH_MOUNTAIN_TRAIL_5, no cutscene.
    let warp = w.actors.all().into_iter().find(|&h| w.actors.downcast::<DoorWarp1>(h).is_some()).expect("warp");
    until(&mut w, 200, |w| w.actors.downcast::<DoorWarp1>(warp).is_some_and(|d| d.action == WarpAction::ChildWarpIdle));
    w.place_player(kd::ARENA_CENTER + Vec3::new(0.0, 20.0, 0.0), 0);
    until(&mut w, 600, |w| w.transition.trigger == oot_game::transition::TRANS_TRIGGER_START);
    assert_eq!(Some(w.transition.next_entrance_index), w.entrance_by_name("ENTR_DEATH_MOUNTAIN_TRAIL_5"));
    assert_eq!(w.save.next_cutscene_index, 0xFFEF);
}

#[test]
fn his_bakes_are_in_the_pack_with_his_textures_sources() {
    let Ok(pack) = oot_game::pack::GamePack::open_default() else { return };
    let mut names = vec![kd::BAKE_SKEL.to_string(), kd::BAKE_PARTICLE.to_string()];
    names.extend((0..8).map(|i| format!("En_Bdfire/{i}")));
    names.extend((1..=8).map(|i| format!("EffectSs/g_magma/{i}")));
    names.extend((0..2).flat_map(|m| (0..10).map(move |t| format!("EffectSs/g_magma2/{m}/{t}"))));
    for name in &names {
        let d: eng_gfx::DrawList = pack.assets.get(&oot_game::pack::keys::bake(name)).expect(name);
        assert!(d.triangle_count() > 0 && d.stats.unresolved_addresses.is_empty(), "{name}");
    }
    // His skeleton's textures name where they were loaded from: the decaying ones among them.
    let d: eng_gfx::DrawList = pack.assets.get(&oot_game::pack::keys::bake(kd::BAKE_SKEL)).unwrap();
    let sources: Vec<u32> = d.textures.iter().filter_map(|t| t.source_addr).collect();
    let found = kd::DECAY_TEXTURES.iter().filter(|(_, off, _, _)| sources.contains(&(0x0600_0000 | off))).count();
    assert!(found >= 8, "{found} of the decay's textures in {sources:x?}");
    // His title card: two blocks (128 × 40 IA8 is over 0x1000 bytes).
    for block in [0, 0x1000] {
        let name = oot_game::title_card::boss_sprite_name("object_kingdodongo", 0x17410, block);
        let d: eng_gfx::DrawList = pack.assets.get(&oot_game::pack::keys::bake(&name)).expect(&name);
        assert!(d.triangle_count() > 0, "{name}");
    }
}

#[test]
fn room_1_draws_the_floor_texture_from_its_segment_3_address() {
    let Some(a) = assets() else { return };
    let w = boss_room(&a, "cavern-boss", |_| {});
    let s = w.scene.as_ref().unwrap();
    let room = s.room(1).unwrap();
    let sources: Vec<u32> = room.entries.iter().flat_map(|e| [&e.opa, &e.xlu]).flatten().flat_map(|d| d.textures.iter().filter_map(|t| t.source_addr)).collect();
    assert!(sources.contains(&0x0300_21D8), "{sources:x?}");
}
