//! Barinade (GAME-10 milestone 6) against the C: `Boss_Va` (`z_boss_va.c`) in his room
//! (`ENTR_JABU_JABU_BOSS_0`: `SCENE_JABU_JABU_BOSS` 0x13, room 1, the body placed at (10, 0, -220),
//! params -1) on a `jabu-barinade` save. No fight is scripted: states, timers and the statics
//! (`Statics`) are set directly where a test is about one of them; the hits go through the
//! collision check from a boomerang's quad (`En_Boom`) laid across the part (as `tests/en_ba.rs`),
//! or are injected (the damage the collision check leaves). Nothing waits on `Rand`. Expected values
//! are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::boss_va::*;
use oot_actors::boss_va_effects as fx;
use oot_actors::en_boom::{ACTOR_EN_BOOM, EnBoom};
use oot_game::actor::*;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::*;
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

/// `SCENE_JABU_JABU_BOSS` (`scene_table.h`: 0x13).
const SCENE: usize = 0x13;
/// The body's placement (room 1's actor list).
const BODY_POS: Vec3 = Vec3::new(10.0, 0.0, -220.0);

/// His room with `edit` on the save, the sound log on, three frames run (the room's actors in).
fn boss_room(a: &Arc<GameAssets>, edit: impl Fn(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_BOSS_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("jabu-barinade").unwrap();
    edit(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 3);
    w
}

/// Every live `Boss_Va` with these params.
fn parts(w: &PlayState, params: i16) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<BossVa>(h).is_some_and(|b| !b.actor.killed && b.actor.params == params)).collect()
}

/// The live Bari of `params` the fight spawned (not the intro's, still dying off: `BariIntro`).
fn fighting_bari(w: &PlayState, params: i16) -> Vec<ActorHandle> {
    parts(w, params).into_iter().filter(|&h| va(w, h).action != Action::BariIntro).collect()
}

fn part(w: &PlayState, params: i16) -> ActorHandle {
    *parts(w, params).first().unwrap_or_else(|| panic!("Boss_Va {params}"))
}

fn va(w: &PlayState, h: ActorHandle) -> &BossVa {
    w.actors.downcast::<BossVa>(h).expect("Boss_Va")
}

fn va_mut(w: &mut PlayState, h: ActorHandle) -> &mut BossVa {
    w.actors.downcast_mut::<BossVa>(h).expect("Boss_Va")
}

fn sts(w: &PlayState) -> &Statics {
    statics(w).expect("the overlay's statics")
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// Its draw's changes (`BossVa_Draw`'s points, spheres and quads; the body's fog).
fn draw_update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.draw_update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// Live actors of `id`.
fn of_id(w: &PlayState, id: i16) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|a| a.id == id && !a.killed)).collect()
}

fn seq_queued(w: &PlayState, seq: u16) -> bool {
    w.audio.log.as_ref().unwrap().seq_cmds.iter().any(|&(_, c)| c == oot_game::audio::bgm::start_seq(0, 0, seq))
}

/// A boomerang (`En_Boom`) whose quad, hurting with `dmg_flags`, crosses `center` (a 60-wide
/// square), through `CollisionCheck_AT` against the part's cylinder (`id` 0) or sphere (1)
/// registered as AC. Whether the part took the hit (`AC_HIT`), and the boomerang (the hit's `ac`).
fn hit_with(w: &mut PlayState, h: ActorHandle, id: u8, center: Vec3, dmg_flags: u32) -> (bool, ActorHandle) {
    w.col_chk.clear();
    {
        let mut a = w.actors.take(h).unwrap();
        let b = a.as_any_mut().downcast_mut::<BossVa>().unwrap();
        if id == 0 {
            b.body_collider.update(&b.actor);
            w.col_chk.set_ac(Some(h), &b.actor, 0, &mut b.body_collider);
        } else {
            w.col_chk.set_ac(Some(h), &b.actor, 1, &mut b.collider_jnt_sph);
        }
        w.actors.put_back(h, a);
    }
    let boom = w.actor_spawn(ACTOR_EN_BOOM, center, [0, 0, 0], 0).expect("En_Boom");
    {
        let mut a = w.actors.take(boom).unwrap();
        let b = a.as_any_mut().downcast_mut::<EnBoom>().unwrap();
        let c = center;
        b.collider.set_vertices(c + Vec3::new(-30.0, -30.0, 0.0), c + Vec3::new(30.0, -30.0, 0.0), c + Vec3::new(-30.0, 30.0, 0.0), c + Vec3::new(30.0, 30.0, 0.0));
        b.collider.info.at_dmg_info.dmg_flags = dmg_flags;
        w.col_chk.set_at(Some(boom), &b.actor, 0, &mut b.collider);
        w.actors.put_back(boom, a);
    }
    let _ = w.col_chk.check(&mut w.actors);
    w.col_chk.clear();
    let b = va(w, h);
    let flags = if id == 0 { b.body_collider.base.ac_flags } else { b.collider_jnt_sph.base.ac_flags };
    (flags & AC_HIT != 0, boom)
}

/// A second try (`EVENTCHKINF_BEGAN_BARINADE_BATTLE`) run to the fight (`BossVa_BodyPhase1`).
fn battle(a: &Arc<GameAssets>) -> PlayState {
    let mut w = boss_room(a, |s| s.set_event_chk_inf(EVENTCHKINF_BEGAN_BARINADE_BATTLE));
    for _ in 0..600 {
        if va(&w, part(&w, BOSSVA_BODY)).action == Action::BodyPhase1 {
            return w;
        }
        idle(&mut w, 1);
    }
    panic!("no fight within 600 frames");
}

#[test]
fn the_body_spawns_the_door_supports_and_zappers() {
    let Some(a) = assets() else { return };
    let w = boss_room(&a, |_| {});
    let body = part(&w, BOSSVA_BODY);
    // BossVa_Init: scale 0.1, his damage table, sPhase2Timer 0xFFFF; the door (his child) at
    // (0, 80, 400); the supports and zappers (his children); no Bari before INTRO_SPAWN_BARI.
    let b = va(&w, body);
    assert_eq!((b.actor.world_pos, b.actor.scale.x, b.action), (BODY_POS, 0.1, Action::BodyIntro));
    assert!(b.actor.col_chk_info.damage_table.is_some());
    assert_eq!(b.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    assert_eq!(sts(&w).phase2_timer, 0xFFFF);
    let door = part(&w, BOSSVA_DOOR);
    assert_eq!(va(&w, door).actor.world_pos, DOOR_POS);
    assert_eq!(va(&w, door).actor.parent, Some(body));
    for p in BOSSVA_SUPPORT_1..=BOSSVA_ZAPPER_3 {
        let h = part(&w, p);
        assert_eq!(va(&w, h).actor.parent, Some(body), "part {p} is his child");
    }
    assert_eq!(va(&w, part(&w, BOSSVA_SUPPORT_2)).action, Action::SupportIntro);
    assert_eq!(va(&w, part(&w, BOSSVA_ZAPPER_1)).action, Action::ZapperIntro);
    assert!((BOSSVA_BARI_UPPER_1..=BOSSVA_BARI_LOWER_5).all(|p| parts(&w, p).is_empty()));
    // A zapper attached (BossVa_AttachToBody): sInitPosOffsets[3] from him, his shape unturned.
    let z = va(&w, part(&w, BOSSVA_ZAPPER_1));
    assert_eq!(z.actor.world_pos, BODY_POS + S_INIT_POS_OFFSETS[3]);
    // The intro's start: INTRO_START's fill colour (no fill: fillScreen stays off), Link held
    // (PLAYER_CSACTION_8) then turned (2); the door closing 8 a frame from 5 while sCsState <=
    // INTRO_DOOR_SHUT.
    let s = sts(&w);
    assert!(s.cs_state > INTRO_START && s.cs_state <= INTRO_DOOR_SHUT, "{}", s.cs_state);
    assert_eq!((s.door_state - 5) % 8, 0);
    assert!(s.door_state > 5);
    assert_eq!(oot_actors::boss_va::screen_fill_color(&w), [0xDC, 0xDC, 0xBE, 0xD2]);
    assert_eq!(w.transition.screen_fill, None);
}

#[test]
fn a_second_try_starts_at_the_bari_call_with_them_spawned() {
    let Some(a) = assets() else { return };
    let w = boss_room(&a, |s| s.set_event_chk_inf(EVENTCHKINF_BEGAN_BARINADE_BATTLE));
    // EVENTCHKINF_BEGAN_BARINADE_BATTLE: INTRO_CALL_BARI, the door shut (100), Link held
    // (PLAYER_CSACTION_1), the sub camera on, the ten Bari at once (BossVa_SetupBariIntro: at
    // their cutscene places, sInitPosOffsets[params + 10] from their homes).
    let s = sts(&w);
    assert_eq!((s.cs_state, s.door_state), (INTRO_CALL_BARI, 100));
    assert_ne!(s.sub_cam_id, 0);
    assert_eq!(w.active_cam_id, s.sub_cam_id);
    assert_eq!(w.player().cs_mode, 1);
    for p in BOSSVA_BARI_UPPER_1..=BOSSVA_BARI_LOWER_5 {
        assert_eq!(va(&w, part(&w, p)).action, Action::BariIntro);
    }
    let upper1 = va(&w, part(&w, BOSSVA_BARI_UPPER_1));
    assert_eq!(upper1.actor.home_pos, BODY_POS + S_INIT_POS_OFFSETS[6]);
}

#[test]
fn the_intro_runs_to_the_title_card_and_the_battle() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, |_| {});
    let body = part(&w, BOSSVA_BODY);
    // INTRO_SPAWN_BARI: the ten Bari, his children.
    let mut n = 0;
    while sts(&w).cs_state < INTRO_LOOK_BARI {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 200);
    }
    assert_eq!(va(&w, part(&w, BOSSVA_BARI_LOWER_5)).actor.parent, Some(body));
    // On to INTRO_TITLE: the light setting 1, the boss music, and his name (the first time:
    // gBarinadeTitleCardTex, object_bv 0x1230, 128 by 40).
    while sts(&w).cs_state < INTRO_TITLE {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 1500, "stuck at {}", sts(&w).cs_state);
    }
    assert_eq!(w.env_ctx.light_setting_override, 1);
    assert!(seq_queued(&w, oot_game::audio::NA_BGM_BOSS));
    assert_eq!(w.title_ctx.texture.as_deref(), Some(oot_game::title_card::boss_sprite_name("object_bv", 0x1230, 0).as_str()));
    // The Bari called to him all attached (sBodyBari: 1 on, then 2 and 3 in INTRO_ATTACH_BARI).
    assert!(sts(&w).body_bari.iter().all(|&b| b != 0), "{:?}", sts(&w).body_bari);
    // INTRO_FINISH's end: the camera back, Link let go (PLAYER_CSACTION_7), the battle begun;
    // BOSSVA_BATTLE the next frame: BossVa_SetupBodyPhase1 (sBodyState 0x80 for 25 frames).
    while va(&w, body).action != Action::BodyPhase1 {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 1700);
    }
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_BEGAN_BARINADE_BATTLE));
    assert_eq!((sts(&w).sub_cam_id, w.active_cam_id), (0, CAM_ID_MAIN));
    // (PLAYER_CSACTION_7 the frame before: Player, updated first, has ended his cutscene.)
    assert_eq!(w.player().cs_mode, 0);
    assert_eq!(sts(&w).body_state, 0x80);
    idle(&mut w, 1);
    assert_eq!(va(&w, part(&w, BOSSVA_SUPPORT_1)).action, Action::SupportAttached);
    assert_eq!(va(&w, part(&w, BOSSVA_ZAPPER_2)).action, Action::ZapperAttack);
}

#[test]
fn a_support_is_cut_by_the_boomerang_but_not_a_sword() {
    let Some(a) = assets() else { return };
    let mut w = battle(&a);
    let sup = part(&w, BOSSVA_SUPPORT_2);
    draw_update(&mut w, sup);
    let c = va(&w, sup).collider_jnt_sph.elements[0].dim.world_sphere.center;
    let c = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
    // sJntSphElementsInitSupport: hit by 0x10 (DMG_BOOMERANG) alone.
    let (hit, boom) = hit_with(&mut w, sup, 1, c, DMG_SLASH_KOKIRI);
    assert!(!hit, "a sword doesn't hit it");
    w.delete_actor(boom);
    let (hit, _) = hit_with(&mut w, sup, 1, c, DMG_BOOMERANG);
    assert!(hit);
    let tip = va(&w, sup).arm_tip;
    update(&mut w, sup);
    // BossVa_SetupSupportCut: sBodyState and sFightPhase on, its stump (params 1 + 16) at its tip
    // 20 up.
    let s = sts(&w);
    assert_eq!((s.fight_phase, s.body_state & 0x7F), (1, 1));
    assert_eq!(va(&w, sup).action, Action::SupportCut);
    let stump = part(&w, BOSSVA_SUPPORT_2 + BOSSVA_STUMP_1);
    assert_eq!(va(&w, stump).actor.world_pos, tip + Vec3::new(0.0, 20.0, 0.0));
    assert_eq!(va(&w, stump).action, Action::Stump);
    // Its next update: the cut skeleton, the body 60 lower and tilted (support 2: x -0x258,
    // z -0x4E2).
    let body = part(&w, BOSSVA_BODY);
    let (y0, rot0) = (va(&w, body).actor.shape_y_offset, va(&w, body).actor.world_rot);
    update(&mut w, sup);
    assert!(va(&w, sup).cut);
    let b = va(&w, body);
    assert_eq!(b.actor.shape_y_offset, y0 - 60.0);
    assert_eq!((b.actor.world_rot.x, b.actor.world_rot.z), (rot0.x.wrapping_sub(0x258), rot0.z.wrapping_sub(0x4E2)));
}

#[test]
fn three_cuts_bring_phase_2_and_the_upper_bari() {
    let Some(a) = assets() else { return };
    let mut w = battle(&a);
    let body = part(&w, BOSSVA_BODY);
    // PHASE_2 (3): at his animation's end (SkelAnime_Update true: it's on its last frame)
    // BossVa_SetupBodyPhase2: the phase on, the five upper Bari, targetable.
    oot_actors::boss_va::st(&mut w).fight_phase = PHASE_2;
    update(&mut w, body);
    assert_eq!(va(&w, body).action, Action::BodyPhase2);
    assert_eq!(sts(&w).fight_phase, PHASE_2 + 1);
    assert_ne!(va(&w, body).actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    for p in BOSSVA_BARI_UPPER_1..=BOSSVA_BARI_UPPER_5 {
        let b = fighting_bari(&w, p);
        assert_eq!(b.len(), 1);
        assert_eq!((va(&w, b[0]).action, va(&w, b[0]).actor.parent), (Action::BariPhase2Attack, Some(body)), "Bari {p}");
    }
    assert!(fighting_bari(&w, BOSSVA_BARI_LOWER_1).is_empty());
}

#[test]
fn stunned_by_the_boomerang_the_body_takes_the_sword() {
    let Some(a) = assets() else { return };
    let mut w = battle(&a);
    let body = part(&w, BOSSVA_BODY);
    oot_actors::boss_va::st(&mut w).fight_phase = PHASE_2;
    update(&mut w, body);
    // BossVa_BodyPhase2: not stunned, his cylinder takes DMG_BOOMERANG alone (:1161).
    update(&mut w, body);
    let c = va(&w, body).actor.world_pos + Vec3::new(0.0, 60.0, 0.0);
    let (hit, boom) = hit_with(&mut w, body, 0, c, DMG_SLASH_KOKIRI);
    assert!(!hit);
    w.delete_actor(boom);
    let (hit, _) = hit_with(&mut w, body, 0, c, DMG_BOOMERANG);
    assert!(hit);
    update(&mut w, body);
    // The boomerang: blue for 160 frames, the sword, the boomerang and a stick hit him now (:1171).
    let b = va(&w, body);
    assert_eq!((b.actor.color_filter_timer, b.actor.color_filter_params & 0x4000), (160, 0));
    assert_eq!(b.body_collider.info.ac_dmg_info.dmg_flags, DMG_SWORD | DMG_BOOMERANG | DMG_DEKU_STICK);
    let (hit, _) = hit_with(&mut w, body, 0, c, DMG_SLASH_KOKIRI);
    assert!(hit);
    // (A sword's hit is Link's: the hit's actor his, not the quad's En_Boom.)
    let player = w.player;
    va_mut(&mut w, body).body_collider.base.ac = player;
    update(&mut w, body);
    // A hit not the boomerang's: sKillBari, red 12 frames.
    let b = va(&w, body);
    assert_eq!(sts(&w).kill_bari, 1);
    assert_ne!(b.actor.color_filter_params & 0x4000, 0);
    // The first upper Bari to update takes it: BossVa_KillBari, the phase on, its death.
    let bari = fighting_bari(&w, BOSSVA_BARI_UPPER_1)[0];
    let phase = sts(&w).fight_phase;
    update(&mut w, bari);
    assert_eq!((sts(&w).kill_bari, sts(&w).fight_phase), (0, phase + 1));
    assert_eq!(va(&w, bari).action, Action::BariDeath);
}

#[test]
fn his_last_hit_and_his_death_bring_the_clear_the_music_and_ruto() {
    let Some(a) = assets() else { return };
    let mut w = battle(&a);
    let body = part(&w, BOSSVA_BODY);
    // Phase 4's last life (sFightPhase 17, sPhase4HP 1), stunned (timer >= 0): a hit of 2 not a
    // stun (damageReaction 0) takes him to PHASE_DEATH: BossVa_SetupBodyDeath.
    {
        let s = oot_actors::boss_va::st(&mut w);
        s.fight_phase = PHASE_DEATH - 1;
        s.phase4_hp = 1;
    }
    {
        let b = va_mut(&mut w, body);
        b.action = Action::BodyPhase4;
        b.timer = 100;
        b.invincibility_timer = 0;
        b.body_collider.base.ac_flags |= AC_HIT;
        b.actor.col_chk_info.damage = 2;
        b.actor.col_chk_info.damage_reaction = 0;
    }
    assert!(!w.flags.get_clear(1));
    update(&mut w, body);
    assert_eq!(va(&w, body).action, Action::BodyDeath);
    assert_eq!(sts(&w).cs_state, DEATH_START);
    assert!(w.flags.get_clear(1), "Flags_SetClear at once");
    assert_eq!(va(&w, body).actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE), 0);
    // DEATH_START: his sub camera, the screen filled white (alpha 0), on to DEATH_BODY_TUMORS.
    update(&mut w, body);
    assert_ne!(sts(&w).sub_cam_id, 0);
    assert_eq!(sts(&w).cs_state, DEATH_BODY_TUMORS);
    assert_eq!(w.transition.screen_fill, Some([255, 255, 255, 0]));
    // DEATH_CORE_BURST at 13: the boss-clear music.
    oot_actors::boss_va::st(&mut w).cs_state = DEATH_CORE_BURST;
    va_mut(&mut w, body).timer = 13;
    update(&mut w, body);
    assert!(seq_queued(&w, oot_game::audio::NA_BGM_BOSS_CLEAR));
    // DEATH_MUSIC's end with Link at sWarpPos[1]: the heart container at him, Ruto (En_Ru1 type 0)
    // at sWarpPos[0] (the C's loop: 1 nearer than 0 picks 0; not the nearest), the camera back.
    w.place_player(S_WARP_POS[1], 0);
    oot_actors::boss_va::st(&mut w).cs_state = DEATH_MUSIC;
    va_mut(&mut w, body).timer = 1;
    assert_eq!((warp_index(S_WARP_POS[1]), warp_index(S_WARP_POS[0])), (0, 2));
    update(&mut w, body);
    assert_eq!(sts(&w).cs_state, DEATH_FINISH);
    assert_eq!((sts(&w).sub_cam_id, w.active_cam_id), (0, CAM_ID_MAIN));
    assert_eq!(w.transition.screen_fill, None, "DEATH_FINISH: fillScreen off");
    let hearts = of_id(&w, oot_actors::boss_goma::ACTOR_ITEM_B_HEART);
    assert_eq!(hearts.len(), 1);
    assert_eq!(w.actors.actor(hearts[0]).unwrap().home_pos, va(&w, body).actor.world_pos);
    let ruto = of_id(&w, oot_actors::en_ru1::ACTOR_EN_RU1);
    assert_eq!(ruto.len(), 1);
    let r = w.actors.actor(ruto[0]).unwrap();
    assert_eq!((r.home_pos, r.params), (S_WARP_POS[0], 0));
}

#[test]
fn a_cleared_room_has_ruto_and_the_heart_and_no_body() {
    let Some(a) = assets() else { return };
    let w = boss_room(&a, |s| s.scene_flags[SCENE].clear |= 1 << 1);
    // BossVa_Init with the room cleared: En_Ru1 params 0 at him, Item_B_Heart 160 east, the door
    // shut (100), the body gone.
    assert!(parts(&w, BOSSVA_BODY).is_empty());
    let ruto = of_id(&w, oot_actors::en_ru1::ACTOR_EN_RU1);
    assert_eq!(ruto.len(), 1);
    let r = w.actors.actor(ruto[0]).unwrap();
    assert_eq!((r.home_pos, r.params), (BODY_POS, 0));
    let hearts = of_id(&w, oot_actors::boss_goma::ACTOR_ITEM_B_HEART);
    assert_eq!(w.actors.actor(hearts[0]).unwrap().home_pos, BODY_POS + Vec3::new(160.0, 0.0, 0.0));
    assert_eq!(sts(&w).door_state, 100);
    assert_eq!(parts(&w, BOSSVA_DOOR).len(), 1);
    // With EVENTCHKINF_37 (Ruto's warp taken): Door_Warp1 params 0 in her place.
    let w = boss_room(&a, |s| {
        s.scene_flags[SCENE].clear |= 1 << 1;
        s.set_event_chk_inf(EVENTCHKINF_37);
    });
    assert!(of_id(&w, oot_actors::en_ru1::ACTOR_EN_RU1).is_empty());
    let warps = of_id(&w, oot_actors::boss_goma::ACTOR_DOOR_WARP1);
    assert_eq!(warps.len(), 1);
    let d = w.actors.actor(warps[0]).unwrap();
    assert_eq!((d.home_pos, d.params), (BODY_POS, 0));
}

#[test]
fn the_particles_spawn_follow_their_part_and_land() {
    let Some(a) = assets() else { return };
    let mut w = boss_room(&a, |_| {});
    let mut e = vec![fx::BossVaEffect::default(); fx::BOSS_VA_EFFECT_COUNT];
    let parent = fx::ParentState { world_pos: BODY_POS, scale_y: 0.1, ..Default::default() };
    let sp = fx::Spawner { handle: None, state: parent };
    // BossVa_SpawnSpark, SPARK_BARI: a large spark in the first free slot, hidden (0, -1000, 0),
    // 111 to 120 frames, its offset kept, opaque.
    fx::spawn_spark(&mut w, &mut e, &sp, Vec3::new(5.0, 6.0, 7.0), 100, fx::SPARK_BARI);
    assert_eq!((e[0].ty, e[0].pos, e[0].prim_color[3]), (fx::VA_LARGE_SPARK, Vec3::new(0.0, -1000.0, 0.0), 255));
    assert!((111..=120).contains(&e[0].timer));
    let t = e[0].timer;
    // An update: its timer down one, held at its part (offset + the part's place).
    fx::update_effects(&mut w, &mut e, BOSSVA_BATTLE, &|_, _| None);
    assert_eq!((e[0].timer, e[0].pos), (t - 1, BODY_POS + Vec3::new(5.0, 6.0, 7.0)));
    // Gore after the shell's burst (GORE_PERMANENT) 60 over the floor: it falls till the floor
    // under it (BgCheck_EntityRaycastDown1, 4 under where its velocity takes it) stops it:
    // GORE_FLOOR, 1 over the floor, flat (rot.x -0x4000), still, 30 frames.
    let over = Vec3::new(10.0, 60.0, -100.0);
    fx::spawn_gore(&mut w, &mut e, over, 0, 100, DEATH_CORE_TUMORS);
    let g = 1;
    assert_eq!((e[g].ty, e[g].mode), (fx::VA_GORE, fx::GORE_PERMANENT));
    let mut n = 0;
    while e[g].mode == fx::GORE_PERMANENT {
        fx::update_effects(&mut w, &mut e, DEATH_CORE_TUMORS, &|_, _| None);
        n += 1;
        assert!(n < 100, "no floor");
    }
    let (floor_y, poly) = w.col.entity_raycast_down(Vec3::new(e[g].pos.x, e[g].pos.y + 10.0, e[g].pos.z));
    assert!(poly.is_some());
    assert_eq!((e[g].mode, e[g].pos.y, e[g].rot[0], e[g].velocity, e[g].timer), (fx::GORE_FLOOR, floor_y + 1.0, -0x4000, Vec3::ZERO, 30));
    // Before the burst it's gone in 20 frames (GORE_FADING) wherever it is.
    fx::spawn_gore(&mut w, &mut e, over, 0, 100, DEATH_SHELL_BURST);
    let f = 2;
    assert_eq!(e[f].mode, fx::GORE_FADING);
    for _ in 0..20 {
        fx::update_effects(&mut w, &mut e, DEATH_SHELL_BURST, &|_, _| None);
    }
    assert_eq!(e[f].ty, fx::VA_NONE);
}

#[test]
fn the_spark_and_death_lights_are_written_and_wear_off() {
    let Some(a) = assets() else { return };
    let mut w = battle(&a);
    let body = part(&w, BOSSVA_BODY);
    // BossVa_SetSparkEnv.
    set_spark_env(&mut w);
    assert_eq!((w.env_ctx.adj_ambient_color, w.env_ctx.adj_light1_color, w.env_ctx.adj_fog_color), ([10; 3], [115, 65, 100], [120, 120, 70]));
    // The body's update takes them down: ambient 1, light and fog 10 a frame.
    update(&mut w, body);
    assert_eq!((w.env_ctx.adj_ambient_color, w.env_ctx.adj_light1_color, w.env_ctx.adj_fog_color), ([9; 3], [105, 55, 90], [110, 110, 60]));
    // BossVa_SetDeathEnv with the screen filled: its colour the fill's; the fog near and the far
    // plane (adjZFar: the port's adj_fog_far) pulled in, the body's draw takes them back 350 a
    // frame.
    set_fill_screen(&mut w, true);
    set_death_env(&mut w);
    assert_eq!(w.transition.screen_fill, Some([220, 220, 150, 100]));
    assert_eq!((w.env_ctx.adj_fog_near, w.env_ctx.adj_fog_far), (-1000, -900));
    assert_eq!((w.env_ctx.adj_ambient_color, w.env_ctx.adj_light1_color), ([200; 3], [215, 165, 200]));
    draw_update(&mut w, body);
    assert_eq!((w.env_ctx.adj_fog_near, w.env_ctx.adj_fog_far), (-650, -550));
    for _ in 0..3 {
        draw_update(&mut w, body);
    }
    assert_eq!((w.env_ctx.adj_fog_near, w.env_ctx.adj_fog_far), (0, 0));
    set_fill_screen(&mut w, false);
    assert_eq!(w.env_ctx.screen_fill_color, [220, 220, 150, 100]);
}

#[test]
fn his_bakes_are_in_the_pack_and_every_part_draws() {
    let Some(a) = assets() else { return };
    for b in oot_actors::boss_va::bakes() {
        let d: eng_gfx::DrawList = a.pack.assets.get(&oot_game::pack::keys::bake(&b.name)).expect(&b.name);
        assert!(d.triangle_count() > 0 && d.stats.unresolved_addresses.is_empty(), "{}", b.name);
    }
    // His title card: two blocks (128 by 40 IA8 is over 0x1000 bytes).
    for block in [0, 0x1000] {
        let name = oot_game::title_card::boss_sprite_name("object_bv", 0x1230, block);
        let d: eng_gfx::DrawList = a.pack.assets.get(&oot_game::pack::keys::bake(&name)).expect(&name);
        assert!(d.triangle_count() > 0, "{name}");
    }
    // In the fight: the body's three passes and his tube (limb 25), the door's eight pieces, a
    // support's, a zapper's skeleton.
    let w = battle(&a);
    let view = oot_game::play::ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
    let key = |n: &str| eng_gfx::MeshKey::named(oot_game::pack::keys::bake(n));
    let draw = |h: ActorHandle| {
        let mut out = oot_game::play::DrawOut::default();
        let b = va(&w, h);
        b.draw(&b.render_state(), &w, &view, &mut out);
        out
    };
    let out = draw(part(&w, BOSSVA_BODY));
    for n in [BAKE_BODY_A, BAKE_BODY_B] {
        assert!(out.opa.iter().any(|c| c.mesh == key(n)), "{n}");
    }
    for n in [BAKE_BODY_XLU, BAKE_BODY_TUBE] {
        assert!(out.xlu.iter().any(|c| c.mesh == key(n)), "{n}");
    }
    let out = draw(part(&w, BOSSVA_DOOR));
    assert_eq!(out.opa.len(), 8);
    assert!(draw(part(&w, BOSSVA_SUPPORT_3)).opa.iter().any(|c| c.mesh == key(BAKE_SUPPORT)));
    assert!(draw(part(&w, BOSSVA_ZAPPER_3)).opa.iter().any(|c| c.mesh == key(BAKE_ZAPPER)));
}
