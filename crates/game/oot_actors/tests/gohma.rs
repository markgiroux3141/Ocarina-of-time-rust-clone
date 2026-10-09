//! Queen Gohma (GAME-05 milestone 6a) against the C: `Boss_Goma` (`z_boss_goma.c`) in her room
//! (`ENTR_DEKU_TREE_BOSS_0`, room 1), `Item_B_Heart` (`z_item_b_heart.c`), and Player's jump slash
//! (`Player_ActionHandler_10`, `func_8083BA90`, `Player_Action_80844AF4`) pulled forward for her.
//!
//! Expected values are worked out from the C in the comments. Her states are set directly where
//! the test is about one of them, and the hits a seed or a sword would make are injected into
//! her eye's sphere (`elements[0]`, the only one `BossGoma_UpdateHit` reads).

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_B, BTN_Z, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::boss_goma::{self as bg, Action, BossGoma};
use oot_actors::en_goma::EnGoma;
use oot_actors::item_b_heart::ItemBHeart;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_HOSTILE};
use oot_game::actor_ctx::ActorHandle;
use oot_game::audio::sfx::*;
use oot_game::camera::{CAM_ID_MAIN, CAM_STAT_ACTIVE, CAM_STAT_WAIT};
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

/// Queen Gohma's room with `preset`, the sound log on, three frames run (the room's objects in:
/// they load a frame after the room's list asks for them, and her init waits for hers).
fn boss_room(a: &Arc<GameAssets>, preset: &str) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEKU_TREE_BOSS_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset(preset).unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 3);
    w
}

fn goma_h(w: &PlayState) -> ActorHandle {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<BossGoma>(h).is_some_and(|g| !g.actor.killed)).expect("Boss_Goma")
}

fn goma(w: &PlayState) -> &BossGoma {
    w.actors.downcast::<BossGoma>(goma_h(w)).unwrap()
}

fn goma_mut(w: &mut PlayState) -> &mut BossGoma {
    let h = goma_h(w);
    w.actors.downcast_mut::<BossGoma>(h).unwrap()
}

fn sfx_on(w: &PlayState, frame: u32, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
}

fn sfx_any(w: &PlayState, id: u16) -> bool {
    w.audio.log.as_ref().unwrap().sfx.iter().any(|&(_, s, _)| s == id)
}

/// An AC hit on her eye's sphere with `dmg_flags`, as the collision check leaves it
/// (`ACELEM_HIT`, `acHitElem`), for her next update.
fn eye_hit(w: &mut PlayState, dmg_flags: u32) {
    let player = w.player.unwrap();
    let e = &mut goma_mut(w).collider.elements[0].info;
    e.ac_elem_flags |= cc::ACELEM_HIT;
    e.ac_hit_elem = Some(cc::HitElem {
        elem: cc::ElemRef { col: cc::ColliderRef { actor: player, id: 0 }, elem: 0 },
        at_dmg_info: cc::ColliderElementDamageInfoAT { dmg_flags, hit_special_effect: cc::HIT_SPECIAL_EFFECT_NONE, damage: 1 },
        ac_dmg_info: Default::default(),
        elem_material: 0,
    });
}

/// Her room's floor below the ceiling's centre (`roomCenter`).
const CENTER: Vec3 = Vec3::new(-150.0, -640.0, -350.0);
/// Link far from her (550 across), on the room's floor by its east bushes.
const LINK_FAR: Vec3 = Vec3::new(400.0, -640.0, -300.0);

/// Her on the floor at the room's centre, the fight on (`disableGameplayLogic` off), in
/// `action`, with Link far from her; her frame count where `BossGoma_UpdateEye`'s random blink
/// (every 16th frame) doesn't come next.
fn on_the_floor(w: &mut PlayState, action: Action) {
    w.place_player(LINK_FAR, -0x6000);
    let g = goma_mut(w);
    g.action = action;
    g.action_state = 0;
    g.disable_gameplay_logic = false;
    g.patience_timer = 200;
    g.frames_until_next_action = 100;
    g.frame_count = 1;
    g.actor.world_pos = CENTER;
    g.actor.shape_rot.x = 0;
    g.actor.gravity = -2.0;
}

#[test]
fn queen_gohma_waits_upside_down_on_the_ceiling() {
    let Some(a) = assets() else { return };
    let w = boss_room(&a, "deku-tree-gohma");
    let g = goma(&w);
    // BossGoma_Init: health 10, MASS_IMMOVABLE, upside down (shape.rot.x -0x8000) at y -300,
    // gravity 0, the encounter (actionState 0, gameplay off), the lights at setting 4 at once.
    assert_eq!((g.actor.col_chk_info.health, g.actor.col_chk_info.mass), (10, cc::MASS_IMMOVABLE));
    assert_eq!((g.actor.shape_rot.x, g.actor.world_pos.y, g.actor.gravity), (i16::MIN, -300.0, 0.0));
    assert_eq!((g.action, g.action_state, g.disable_gameplay_logic), (Action::Encounter, 0, true));
    assert_eq!((w.env_ctx.light_setting_override, w.env_ctx.light_blend_rate_override), (4, 255));
    // sInitChain: ATTENTION_RANGE_2, NAVI_ENEMY_GOHMA; shape.yOffset 4000; the iris at scale 1.
    assert_eq!((g.actor.target_mode, g.actor.navi_enemy_id, g.actor.shape_y_offset), (2, 1, 4000.0));
    assert_eq!((g.eye_iris_scale_x, g.eye_iris_scale_y), (1.0, 1.0));
    assert!(g.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE) == ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE);
    // Her 13 spheres: the eye (limb 5, 20 at (0, 0, 1200)) first.
    assert_eq!(g.collider.elements.len(), 13);
    assert_eq!((g.collider.elements[0].dim.limb, g.collider.elements[0].dim.model_sphere.radius), (5, 20));
}

#[test]
fn in_a_cleared_room_she_leaves_the_blue_warp_and_the_heart_container() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, "deku-tree-gohma");
    let room = w.room_ctx.cur.num;
    w.flags.set_clear(room);
    let h = w.actor_spawn(oot_actors::en_goma::ACTOR_BOSS_GOMA, Vec3::new(-160.0, -280.0, -371.0), [0; 3], -1).expect("spawned");
    // BossGoma_Init with the room cleared: killed, her child the warp (WARP_DUNGEON_CHILD) at (0,
    // -640, 0), and the heart at (141, -640, -84).
    assert!(w.actors.actor(h).is_none_or(|g| g.killed));
    let warp = w.actors.all().into_iter().find(|&x| w.actors.actor(x).is_some_and(|a| a.id == bg::ACTOR_DOOR_WARP1)).expect("the warp");
    let wa = w.actors.actor(warp).unwrap();
    assert_eq!((wa.world_pos, wa.params), (Vec3::new(0.0, -640.0, 0.0), bg::WARP_DUNGEON_CHILD));
    let heart = w.actors.all().into_iter().find_map(|x| w.actors.downcast::<ItemBHeart>(x)).expect("the heart");
    assert_eq!(heart.actor.world_pos, Vec3::new(141.0, -640.0, -84.0));
}

#[test]
fn her_intro_zooms_in_on_link_drops_the_slab_and_waits_to_be_seen() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, "deku-tree-gohma");
    // Link on her trigger: the room's entrance (150, 350) within 60.
    w.place_player(Vec3::new(150.0, -640.0, 350.0), -0x705C);
    idle(&mut w, 1);
    // State 0: Link held (PLAYER_CSACTION_8), state 1.
    assert_eq!(goma(&w).action_state, 1);
    assert_eq!(w.player().cs_mode, 8);
    idle(&mut w, 1);
    // State 1 falls through to 2: her sub camera active, the main one waiting, her at the
    // ceiling's centre (-150, -320, -350), Link at (150, 300) facing -0x705C, still.
    let g = goma(&w);
    assert_eq!((g.action_state, g.frame_count, g.timer), (2, 0, 80));
    let sub = g.sub_cam_id;
    assert!(sub > 0);
    assert_eq!(w.active_cam_id, sub);
    assert_eq!(w.camera(CAM_ID_MAIN).unwrap().status, CAM_STAT_WAIT);
    assert_eq!(w.camera(sub).unwrap().status, CAM_STAT_ACTIVE);
    assert_eq!(g.actor.world_pos, Vec3::new(-150.0, -320.0, -350.0));
    let p = w.player();
    assert_eq!((p.actor.world_pos.x, p.actor.world_pos.z, p.actor.shape_rot.y), (150.0, 300.0, -0x705C));
    // frameCount 176: the slab, her child (Door_Shutter, SHUTTER_GOHMA_BLOCK), at (164.72, -480,
    // 397.68), and the lights to setting 3 blending as they will.
    idle(&mut w, 175);
    let slabs = |w: &PlayState| w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| a.id == oot_actors::door_shutter::ACTOR_DOOR_SHUTTER && a.params == 0x180)).count();
    assert_eq!(slabs(&w), 0);
    idle(&mut w, 1);
    assert_eq!(goma(&w).frame_count, 176);
    assert_eq!(slabs(&w), 1);
    assert_eq!((w.env_ctx.light_setting_override, w.env_ctx.light_blend_rate_override), (3, oot_game::env::LIGHT_BLENDRATE_OVERRIDE_NONE));
    // Its landing (the slab's first frames) shakes her sub camera, not the main one
    // (DoorShutter_GohmaBlockFall: ((BossGoma*)parent)->subCamId).
    let mut quake_cam = None;
    for _ in 0..14 {
        idle(&mut w, 1);
        if let Some(r) = w.quake.requests.iter().find(|r| r.duration > 0) {
            quake_cam = Some(r.cam_id);
        }
    }
    assert_eq!(quake_cam, Some(sub));
    // 190: Link turns to her (PLAYER_CSACTION_2).
    assert_eq!(goma(&w).frame_count, 190);
    assert_eq!(w.player().cs_mode, 2);
    // 228: back to the main camera (taking her view), the cutscene over (PLAYER_CSACTION_7),
    // state 3: she waits to be seen.
    idle(&mut w, 38);
    let g = goma(&w);
    assert_eq!((g.frame_count, g.action_state, g.sub_cam_id), (228, 3, bg::SUB_CAM_ID_DONE));
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
}

#[test]
fn a_second_try_skips_to_her_eye_and_shows_no_title_card() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, "deku-tree-gohma-again");
    w.place_player(Vec3::new(150.0, -640.0, 350.0), -0x705C);
    idle(&mut w, 1);
    // EVENTCHKINF_BEGAN_GOHMA_BATTLE: BossGoma_SetupEncounterState4 at once (state 4, her sub
    // camera from (90, eye + 20, 170), the music stopped), and the slab.
    let g = goma(&w);
    assert_eq!((g.action_state, g.frames_until_next_action), (4, 50));
    assert_eq!((g.sub_cam_eye.x, g.sub_cam_eye.z), (90.0, 170.0));
    assert!(w.actors.all().into_iter().any(|h| w.actors.actor(h).is_some_and(|a| a.id == oot_actors::door_shutter::ACTOR_DOOR_SHUTTER && a.params == 0x180)));
    // Until the fight: no title card (TitleCard_InitBossName only before the flag).
    let mut card = false;
    for _ in 0..400 {
        idle(&mut w, 1);
        card |= w.title_ctx.duration_timer != 0 || w.title_ctx.alpha != 0;
        if goma(&w).action == Action::FloorMain {
            break;
        }
    }
    assert_eq!(goma(&w).action, Action::FloorMain);
    assert!(!card);
    // The fight: patient for 200 frames (less this frame's), the boss music started.
    assert!(goma(&w).patience_timer >= 199);
    let cmds = &w.audio.log.as_ref().unwrap().seq_cmds;
    assert!(cmds.iter().any(|&(_, c)| c == oot_game::audio::bgm::start_seq(0, 0, oot_game::audio::NA_BGM_BOSS)));
}

#[test]
fn a_seed_or_a_nut_into_her_open_eye_stuns_her_while_shes_patient() {
    let Some(a) = assets() else { return };
    for (dmg, frames) in [(cc::DMG_SLINGSHOT, 90), (cc::DMG_DEKU_NUT, 40)] {
        let mut w = boss_room(&a, "deku-tree-gohma");
        on_the_floor(&mut w, Action::FloorMain);
        eye_hit(&mut w, dmg);
        idle(&mut w, 1);
        let f = w.audio.frames;
        // BossGoma_UpdateHit: NA_SE_EN_GOMA_DAM2, 10 frames invincible, stunned for 90 (a seed)
        // or 40 (a nut), sfxFaintTimer 100, timer 4.
        let g = goma(&w);
        assert_eq!((g.action, g.frames_until_next_action, g.sfx_faint_timer, g.timer, g.invincibility_frames), (Action::FloorStunned, frames, 100, 4, 10));
        assert!(sfx_on(&w, f, NA_SE_EN_GOMA_DAM2));
    }
    // Her eye closed (eyeClosedTimer): nothing.
    let mut w = boss_room(&a, "deku-tree-gohma");
    on_the_floor(&mut w, Action::FloorMain);
    goma_mut(&mut w).eye_closed_timer = 5;
    eye_hit(&mut w, cc::DMG_SLINGSHOT);
    idle(&mut w, 1);
    assert_eq!(goma(&w).action, Action::FloorMain);
    // Out of patience: a seed does nothing either.
    let mut w = boss_room(&a, "deku-tree-gohma");
    on_the_floor(&mut w, Action::FloorMain);
    goma_mut(&mut w).patience_timer = 0;
    eye_hit(&mut w, cc::DMG_SLINGSHOT);
    idle(&mut w, 1);
    assert_ne!(goma(&w).action, Action::FloorStunned);
}

#[test]
fn shooting_closes_her_eye_unless_its_red() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, "deku-tree-gohma");
    on_the_floor(&mut w, Action::FloorMain);
    let ph = w.player.unwrap();
    w.actors.downcast_mut::<oot_actors::player::Player>(ph).unwrap().unk_A73 = 4;
    idle(&mut w, 1);
    // Player's update counts unk_A73 down to 3 first; her BossGoma_UpdateEye (EYESTATE_IRIS_
    // FOLLOW_BONUS_IFRAMES) clears it and closes her eye for 12 frames, 11 left after this one.
    assert_eq!(w.player().unk_A73, 0);
    assert_eq!(goma(&w).eye_closed_timer, 11);
    // Rearing up (EYESTATE_IRIS_FOLLOW_NO_IFRAMES): the shot doesn't close it.
    let mut w = boss_room(&a, "deku-tree-gohma");
    on_the_floor(&mut w, Action::FloorAttackPosture);
    w.actors.downcast_mut::<oot_actors::player::Player>(ph).unwrap().unk_A73 = 4;
    idle(&mut w, 1);
    assert_eq!(w.player().unk_A73, 3);
    assert_eq!(goma(&w).eye_closed_timer, 0);
}

#[test]
fn stunned_she_takes_a_swords_damage_and_dies_at_none() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, "deku-tree-gohma");
    on_the_floor(&mut w, Action::FloorStunned);
    goma_mut(&mut w).frames_until_next_action = 90;
    let sibukis = |w: &PlayState| w.effect_ss.table.iter().filter(|e| e.life >= 0 && e.ty == oot_game::effect::EFFECT_SS_SIBUKI).count();
    let before = sibukis(&w);
    eye_hit(&mut w, cc::DMG_SLASH_KOKIRI);
    idle(&mut w, 1);
    let f = w.audio.frames;
    // The Kokiri Sword's slash: 1 off, NA_SE_EN_GOMA_DAM1, damaged, a burst of bubbles at her
    // focus (EffectSsSibuki_SpawnBurst), 10 frames invincible.
    let g = goma(&w);
    assert_eq!((g.actor.col_chk_info.health, g.action, g.invincibility_frames), (9, Action::FloorDamaged, 10));
    assert!(sfx_on(&w, f, NA_SE_EN_GOMA_DAM1));
    assert!(sibukis(&w) > before);
    // Invincible: the next hit is lost.
    eye_hit(&mut w, cc::DMG_JUMP_KOKIRI);
    idle(&mut w, 1);
    assert_eq!(goma(&w).actor.col_chk_info.health, 9);
    // The damage animation over: stunned again, her patience gone.
    for _ in 0..200 {
        if goma(&w).action == Action::FloorStunned {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!((goma(&w).action, goma(&w).patience_timer), (Action::FloorStunned, 0));
    // A jump slash: 2 off.
    eye_hit(&mut w, cc::DMG_JUMP_KOKIRI);
    idle(&mut w, 1);
    assert_eq!(goma(&w).actor.col_chk_info.health, 7);
    // The last of her health: defeated (BossGoma_SetupDefeated: 1200 frames, untargetable, the
    // music stopped, NA_SE_EN_GOMA_DEAD), the finishing blow's freeze.
    let g = goma_mut(&mut w);
    g.action = Action::FloorStunned;
    g.invincibility_frames = 0;
    g.actor.col_chk_info.health = 1;
    eye_hit(&mut w, cc::DMG_SLASH_KOKIRI);
    idle(&mut w, 1);
    let f = w.audio.frames;
    let g = goma(&w);
    assert_eq!((g.action, g.frames_until_next_action, g.disable_gameplay_logic), (Action::Defeated, 1200, true));
    assert_eq!(g.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE), 0);
    assert!(sfx_on(&w, f, NA_SE_EN_GOMA_DEAD));
    assert!(w.actors.freeze_flash_timer > 0);
    let cmds = &w.audio.log.as_ref().unwrap().seq_cmds;
    assert!(cmds.iter().any(|&(fr, c)| fr == f && c == oot_game::audio::bgm::seq_cmd1(0, 1)));
}

#[test]
fn a_hit_on_the_ceiling_knocks_her_down_stunned_for_150_frames() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, "deku-tree-gohma");
    w.place_player(LINK_FAR, -0x6000);
    let g = goma_mut(&mut w);
    g.action = Action::CeilingIdle;
    g.disable_gameplay_logic = false;
    g.frames_until_next_action = 50;
    g.frame_count = 1;
    g.actor.world_pos = Vec3::new(-150.0, -320.0, -350.0);
    eye_hit(&mut w, cc::DMG_SLINGSHOT);
    idle(&mut w, 1);
    let f = w.audio.frames;
    // BossGoma_SetupFallStruckDown: gravity -2, NA_SE_EN_GOMA_DAM2.
    let g = goma(&w);
    assert_eq!((g.action, g.actor.gravity), (Action::FallStruckDown, -2.0));
    assert!(sfx_on(&w, f, NA_SE_EN_GOMA_DAM2));
    // Down (NA_SE_EN_GOMA_DAM1 on landing), the crash's animation, then stunned 150 frames with
    // sfxFaintTimer 92 and no patience.
    let mut landed = false;
    for _ in 0..300 {
        idle(&mut w, 1);
        landed |= goma(&w).action == Action::FloorLandStruckDown;
        if goma(&w).action == Action::FloorStunned {
            break;
        }
    }
    assert!(landed);
    assert!(sfx_any(&w, NA_SE_EN_GOMA_DAM1));
    let g = goma(&w);
    assert_eq!((g.action, g.frames_until_next_action, g.sfx_faint_timer, g.patience_timer), (Action::FloorStunned, 150, 92, 0));
    assert_eq!(g.actor.world_pos.y, -640.0);
}

#[test]
fn she_lays_three_eggs_and_jumps_down_once_their_larvae_are_dead() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, "deku-tree-gohma");
    w.place_player(LINK_FAR, -0x6000);
    let g = goma_mut(&mut w);
    g.action = Action::CeilingPrepareSpawnGohmas;
    g.disable_gameplay_logic = false;
    g.frames_until_next_action = 2;
    g.actor.world_pos = Vec3::new(-150.0, -320.0, -350.0);
    let gh = goma_h(&w);
    let mut tails = Vec::new();
    for _ in 0..400 {
        idle(&mut w, 1);
        let g = goma(&w);
        // Set to 10, then counted down once by BossGoma_UpdateTailLimbsScale in the same update.
        if g.tail_limbs_scale_timers.iter().any(|&t| t == 9) {
            tails.push(g.spawn_gohmas_action_timer);
        }
        if g.action == Action::CeilingIdle {
            break;
        }
    }
    // BossGoma_CeilingSpawnGohmas: her tail swells from the body out (24, 32, 40, 48), an egg as
    // the last limb's timer comes to 2, three rounds (back to 23 while one's still to come).
    assert_eq!(&tails[..4], &[24, 32, 40, 48]);
    assert_eq!(goma(&w).children_gohma_state, [1, 1, 1]);
    let eggs: Vec<&EnGoma> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnGoma>(h)).filter(|e| e.actor.params < 3).collect();
    assert_eq!(eggs.len(), 3);
    for e in &eggs {
        // Her children (Actor_SpawnAsChild), a third of a turn apart.
        assert_eq!(e.actor.parent, Some(gh));
        assert_eq!(e.actor.home_rot.y, (e.actor.params as i32 * (0x10000 / 3)) as i16);
    }
    // Each egg broken by a hit writes -1 into her childrenGohmaState (EnGoma_UpdateHit).
    let egg_handles: Vec<ActorHandle> = w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnGoma>(h).is_some_and(|e| e.actor.params < 3)).collect();
    let player = w.player.unwrap();
    for h in egg_handles {
        let e = w.actors.downcast_mut::<EnGoma>(h).unwrap();
        e.invincibility_timer = 0;
        e.collider_cylinder2.base.ac_flags |= cc::AC_HIT;
        e.collider_cylinder2.info.ac_hit_elem = Some(cc::HitElem {
            elem: cc::ElemRef { col: cc::ColliderRef { actor: player, id: 0 }, elem: 0 },
            at_dmg_info: cc::ColliderElementDamageInfoAT { dmg_flags: cc::DMG_SLASH_KOKIRI, hit_special_effect: cc::HIT_SPECIAL_EFFECT_NONE, damage: 1 },
            ac_dmg_info: Default::default(),
            elem_material: 0,
        });
    }
    idle(&mut w, 1);
    assert_eq!(goma(&w).children_gohma_state, [-1, -1, -1]);
    // All dead: from her idle, the jump down (BossGoma_SetupFallJump).
    for _ in 0..80 {
        if goma(&w).action == Action::FallJump {
            break;
        }
        idle(&mut w, 1);
    }
    assert_eq!(goma(&w).action, Action::FallJump);
}

#[test]
fn her_death_breaks_her_apart_erases_her_textures_and_leaves_the_heart_and_warp() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, "deku-tree-gohma");
    on_the_floor(&mut w, Action::FloorStunned);
    let gh = goma_h(&w);
    let bank = goma(&w).actor.obj_bank_index.expect("her object's bank");
    goma_mut(&mut w).actor.col_chk_info.health = 1;
    eye_hit(&mut w, cc::DMG_SLASH_KOKIRI);
    idle(&mut w, 1);
    assert_eq!(goma(&w).action, Action::Defeated);
    // Her six textures as the file has them (the pack's), one after another.
    let pack = &a.pack;
    let original: Vec<u8> = bg::DECAY_TEXTURES.iter().flat_map(|(s, _, _)| pack.texture(bg::OBJECT, s).unwrap().texels).collect();
    let (mut pieces, mut heart_at, mut warp_at, mut first_pass_done, mut gone_at) = (0, None, None, None, None);
    // Her updates are counted (k), not the frames: the finishing blow freezes every actor for a
    // few (Enemy_StartFinishingBlow's freezeFlashTimer).
    let f0 = goma(&w).frame_count;
    let mut last_k = 0;
    for _ in 1..=720 {
        idle(&mut w, 1);
        let alive = w.actors.downcast::<BossGoma>(gh).is_some_and(|g| !g.actor.killed);
        if !alive {
            gone_at.get_or_insert(last_k + 1);
            break;
        }
        let g = w.actors.downcast::<BossGoma>(gh).unwrap();
        let k = g.frame_count.wrapping_sub(f0);
        if k == last_k {
            continue;
        }
        last_k = k;
        // k updates after the killing blow: framesUntilNextAction 1200 - k.
        assert_eq!(g.frames_until_next_action, 1200 - k);
        if k == 199 {
            // 1001: every limb with a lifetime breaks off as an En_Goma piece (params 100 +
            // lifetime), her child, drawing its limb's list.
            let ps: Vec<&EnGoma> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnGoma>(h)).filter(|e| e.actor.params >= 100).collect();
            pieces = ps.len();
            for p in &ps {
                assert_eq!(p.actor.parent, Some(gh));
                let lifetime = (p.actor.params - 100) as u8;
                assert!(bg::S_DEAD_LIMB_LIFETIME.contains(&lifetime) && lifetime != 0);
                assert!(p.boss_limb_dl.is_some_and(|(f, s)| f == bg::OBJECT && s.starts_with("gGohma")));
                assert_eq!(p.actor.obj_bank_index, Some(bank));
            }
        }
        if k == 184 {
            // The first pass from 1079 (k 121), 4 steps a frame: 64 frames to 256.
            assert_eq!(g.decaying_progress, 256);
        }
        if k == 190 {
            // From then on step 256 is erased each time (the progress stays there).
            first_pass_done = w.object_ctx.written(bank, bg::DECAY_REGION).map(|v| v.to_vec());
        }
        if heart_at.is_none() && w.actors.all().into_iter().any(|h| w.actors.downcast::<ItemBHeart>(h).is_some()) {
            heart_at = Some(k);
        }
        if warp_at.is_none() && w.actors.all().into_iter().any(|h| w.actors.actor(h).is_some_and(|a| a.id == bg::ACTOR_DOOR_WARP1)) {
            warp_at = Some(k);
            assert!(w.flags.get_clear(w.room_ctx.cur.num));
        }
    }
    // 20 limbs have a lifetime (sDeadLimbLifetime).
    assert_eq!(pieces, bg::S_DEAD_LIMB_LIFETIME.iter().filter(|&&l| l != 0).count());
    assert_eq!(pieces, 20);
    // The first pass: each 16x16 texture's pixel i erased where sClearPixelTableFirstPass[i] is
    // set, and (@bug (game)) at progress 256, which reads the second table's first byte (1),
    // pixel 256: the next texture's first.
    let after = first_pass_done.expect("the region written");
    let px = |v: &[u8], tex: usize, i: usize| {
        let o = (bg::DECAY_TEXTURES[tex].1 - bg::DECAY_REGION) as usize + i * 2;
        u16::from_be_bytes([v[o], v[o + 1]])
    };
    for i in 0..256 {
        let erased = bg::S_CLEAR_PIXEL_TABLE_FIRST_PASS[i] != 0;
        assert_eq!(px(&after, 0, i) == 0, erased || px(&original, 0, i) == 0, "body pixel {i}");
    }
    // The body's 256th pixel is the underside's first (FirstPass[0] is 0: only the bug clears it).
    assert_eq!(bg::S_CLEAR_PIXEL_TABLE_FIRST_PASS[0], 0);
    assert_ne!(px(&original, 1, 0), 0);
    assert_eq!(px(&after, 1, 0), 0);
    // The timer's 270 frames from her first defeated update: the heart at her (k 271); 70 more:
    // the warp (her child) and the room cleared (k 341); 30 more, then she shrinks away.
    assert_eq!((heart_at, warp_at), (Some(271), Some(341)));
    let gone = gone_at.expect("gone");
    assert!((372..=400).contains(&gone), "{gone}");
    assert_eq!(w.active_cam_id, CAM_ID_MAIN);
    let cmds = &w.audio.log.as_ref().unwrap().seq_cmds;
    assert!(cmds.iter().any(|&(_, c)| c == oot_game::audio::bgm::start_seq(0, 0, oot_game::audio::NA_BGM_BOSS_CLEAR)));
}

#[test]
fn the_heart_container_grows_bobs_and_is_taken_once() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, "deku-tree-gohma");
    let home = Vec3::new(141.0, -640.0, -84.0);
    let h = w.actor_spawn(bg::ACTOR_ITEM_B_HEART, home, [0; 3], 0).expect("spawned");
    let b = w.actors.downcast::<ItemBHeart>(h).unwrap();
    // sInitChain: scale 0.
    assert_eq!(b.actor.scale, Vec3::ZERO);
    // (Spawned between frames, it updates from the next Actor_UpdateAll on.)
    while w.actors.downcast::<ItemBHeart>(h).unwrap().unk_164 == 0 {
        idle(&mut w, 1);
    }
    // func_80B85264: the scale to 0.4 by a tenth, at most 0.01 a frame; the bob's speed 0 this
    // frame (then up by 0.1 to 2); 0x400 of spin.
    let b = w.actors.downcast::<ItemBHeart>(h).unwrap();
    assert_eq!((b.actor.scale.x, b.actor.scale.y, b.unk_164, b.unk_158), (0.01, 0.01, 1, 0.1));
    assert_eq!(b.actor.world_pos.y, home.y);
    assert_eq!(b.actor.shape_rot.y, 0x400);
    // Taken (Actor_HasParent): collectible flag 0x1F, gone; never spawned again.
    let player = w.player;
    w.actors.downcast_mut::<ItemBHeart>(h).unwrap().actor.parent = player;
    idle(&mut w, 1);
    assert!(w.actors.downcast::<ItemBHeart>(h).is_none_or(|b| b.actor.killed));
    assert!(w.flags.get_collectible(0x1F));
    let h2 = w.actor_spawn(bg::ACTOR_ITEM_B_HEART, home, [0; 3], 0).expect("spawned");
    assert!(w.actors.downcast::<ItemBHeart>(h2).is_none_or(|b| b.actor.killed));
}

#[test]
fn locked_on_with_the_sword_out_a_jumps_into_a_jump_slash() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, "deku-tree-gohma");
    on_the_floor(&mut w, Action::FloorStunned);
    goma_mut(&mut w).frames_until_next_action = 200;
    w.place_player(CENTER + Vec3::new(0.0, 0.0, 130.0), -0x8000);
    idle(&mut w, 10);
    let mut prev = PadState::default();
    let mut tick = |w: &mut PlayState, p: PadState| {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    };
    // Z (locked on), B (the sword out: a slash), then A with the stick let go.
    tick(&mut w, PadState { button: BTN_Z, ..Default::default() });
    for _ in 0..30 {
        tick(&mut w, PadState::default());
    }
    assert!(w.player().focus_actor.is_some());
    tick(&mut w, PadState { button: BTN_B, ..Default::default() });
    for _ in 0..40 {
        tick(&mut w, PadState::default());
    }
    assert_eq!(w.player().held_item_id, oot_game::item::ITEM_SWORD_KOKIRI);
    tick(&mut w, PadState { button: BTN_A, ..Default::default() });
    // Player_ActionHandler_10 -> func_8083BA90(PLAYER_MWA_JUMPSLASH_START, 5, 5): off the ground at
    // 5 across and 5 up (set after this frame's move), NA_SE_VO_LI_SWORD_L.
    let p = w.player();
    assert_eq!(p.action, oot_actors::player::Action::JumpSlash);
    assert_eq!((p.linear_velocity, p.actor.velocity.y), (5.0, 5.0));
    // (Player_PlayVoiceSfx: his voice plus the age's offset, unk_92.)
    assert!(sfx_on(&w, w.audio.frames, NA_SE_VO_LI_SWORD_L + p.age.climb.unk_92));
    // Landing: the finish (the start + 2), on the ground.
    let start = data().unwrap().items.mwa("JUMPSLASH_START");
    let mut finished = false;
    for _ in 0..40 {
        tick(&mut w, PadState::default());
        if w.player().melee_weapon_animation == start + 2 {
            finished = true;
            break;
        }
    }
    assert!(finished);
}

#[test]
fn her_bakes_are_in_the_pack_with_their_textures_sources() {
    let Ok(pack) = oot_game::pack::GamePack::open_default() else { return };
    for no_cull in [false, true] {
        for base in [bg::BAKE_SKEL, bg::BAKE_EYE, bg::BAKE_IRIS] {
            let name = bg::bake_name(base, no_cull);
            let d: eng_gfx::DrawList = pack.assets.get(&oot_game::pack::keys::bake(&name)).expect(&name);
            assert!(d.triangle_count() > 0 && d.stats.unresolved_addresses.is_empty(), "{name}");
        }
    }
    // The skeleton's textures name where they were loaded from: her six decaying ones among
    // them, at 0x06 + their offsets (what DrawParams::texture_images replaces).
    let d: eng_gfx::DrawList = pack.assets.get(&oot_game::pack::keys::bake(bg::BAKE_SKEL)).unwrap();
    let sources: Vec<u32> = d.textures.iter().filter_map(|t| t.source_addr).collect();
    for (s, off, _) in bg::DECAY_TEXTURES {
        if s == "gGohmaIrisTex" || s == "gGohmaEyeTex" {
            continue;
        }
        assert!(sources.contains(&(0x0600_0000 | off)), "{s} in {sources:x?}");
    }
    let eye: eng_gfx::DrawList = pack.assets.get(&oot_game::pack::keys::bake(bg::BAKE_EYE)).unwrap();
    assert!(eye.textures.iter().any(|t| t.source_addr == Some(0x0600_0000 | 0x191A8)));
    // The pieces' bakes, one per limb that breaks off.
    for (limb, s) in bg::LIMB_DLISTS {
        if bg::S_DEAD_LIMB_LIFETIME[limb] != 0 {
            let d: eng_gfx::DrawList = pack.assets.get(&oot_game::pack::keys::bake(&oot_actors::en_goma::boss_limb_bake_name(s))).expect(s);
            assert!(d.triangle_count() > 0, "{s}");
        }
    }
}
