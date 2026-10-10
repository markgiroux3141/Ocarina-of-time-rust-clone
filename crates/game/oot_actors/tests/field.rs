//! GAME-06 milestone 4 against the C: Hyrule Field's actors. The Peahat and its larvae
//! (`z_en_peehat.c`), the Stalchild (`z_en_skb.c`) and its spawner (`z_en_encount1.c`), the parts
//! a body breaks into (`z_en_part.c`, `z_actor.c`'s `BodyBreak`), the enemy music
//! (`Attention_FindActorInCategory`'s `bgmEnemy`, `Player_UpdateCamAndSeqModes`), the roll's bonk
//! into a tree (`Player_Action_Roll`, `z_en_wood02.c`), a grotto's hole (`z_door_ana.c`); then the
//! exit's runs (`Route::Field`, `Route::FieldNight`).
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_peehat::*;
use oot_actors::en_skb::{self, EnSkb};
use oot_actors::playthrough::{Playthrough, Route, Step};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, COLORFILTER_COLORFLAG_RED};
use oot_game::actor_ctx::ActorHandle;
use oot_game::collision_check::*;
use oot_game::env::clock_time;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

/// `SCENE_MARKET_ENTRANCE_DAY`, `SCENE_KAKARIKO_VILLAGE` (`scene_table.h`).
const SCENE_MARKET_ENTRANCE_DAY: u16 = 0x1B;
const SCENE_KAKARIKO_VILLAGE: u16 = 0x52;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Child Link in Hyrule Field (`deku-tree-dead`) at `h:m`, entered at the drawbridge and placed on
/// the grass north of Lon Lon Ranch (no water box over it) facing east before the first frame; two
/// frames on (room 0's actors and objects in).
fn field(a: &Arc<GameAssets>, h: i32, m: i32) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_HYRULE_FIELD_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(h, m) as u16);
    save.apply_preset("deku-tree-dead").unwrap();
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.place_player(OPEN_FIELD, 0x4000);
    idle(&mut w, 2);
    w
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn peahat(w: &PlayState, h: ActorHandle) -> &EnPeehat {
    w.actors.downcast::<EnPeehat>(h).expect("the Peahat")
}

fn peahat_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnPeehat {
    w.actors.downcast_mut::<EnPeehat>(h).expect("the Peahat")
}

fn all<T: 'static + oot_game::actor_ctx::ActorImpl>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some()).collect()
}

/// Where `field` places Link.
const OPEN_FIELD: Vec3 = Vec3::new(1500.0, 0.0, 2000.0);

#[test]
fn peahats_initialise_as_their_c_does() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, 0);
    let at = OPEN_FIELD + Vec3::new(0.0, 0.0, 3000.0);
    let grounded = w.actor_spawn(ACTOR_EN_PEEHAT, at, [0; 3], PEAHAT_TYPE_GROUNDED).unwrap();
    let flying = w.actor_spawn(ACTOR_EN_PEEHAT, at + Vec3::new(300.0, 0.0, 0.0), [0; 3], PEAHAT_TYPE_FLYING).unwrap();
    let larva = w.actor_spawn(ACTOR_EN_PEEHAT, at + Vec3::new(600.0, 100.0, 0.0), [0; 3], PEAHAT_TYPE_LARVA).unwrap();
    // EnPeehat_Init: scale 36 / 1000, health 6, ActorShape_Init's yOffset 100, lockOnArrowOffset
    // 700; the cylinder 50 by 160 from 70 down.
    let g = peahat(&w, grounded);
    assert_eq!((g.actor.scale.x, g.actor.col_chk_info.health, g.actor.shape_y_offset, g.actor.target_arrow_offset), (36.0 * 0.001, 6, 100.0, 700.0));
    assert_eq!((g.collider_cylinder.dim.radius, g.collider_cylinder.dim.height, g.collider_cylinder.dim.y_shift), (50, 160, -70));
    // Grounded: 740 to rise, 1200 from home; EnPeehat_Ground_SetStateGround: state 3, 600 frames
    // seeking, the rising animation held at frame 3.
    assert_eq!((g.xz_dist_to_rise, g.xz_dist_max, g.state, g.seek_player_timer, g.skel.cur_frame, g.skel.play_speed), (740.0, 1200.0, PEAHAT_STATE_3, 600, 3.0, 0.0));
    assert_eq!(g.actor.navi_enemy_id, 0x48);
    // Flying: 2800 and 1400, state 4, 400 frames, not targetable.
    let f = peahat(&w, flying);
    assert_eq!((f.xz_dist_to_rise, f.xz_dist_max, f.state, f.seek_player_timer), (2800.0, 1400.0, PEAHAT_STATE_4, 400));
    assert_eq!(f.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    // A larva: scale (0.006, 0.003, 0.006), its cylinder 25 by 15 from 5 down hit only by arrows and
    // seeds, its quad's AT and AC (not hard), seeking Link (state 14).
    let l = peahat(&w, larva);
    assert_eq!(l.actor.scale, Vec3::new(0.006, 0.003, 0.006));
    assert_eq!((l.collider_cylinder.dim.radius, l.collider_cylinder.dim.height, l.collider_cylinder.dim.y_shift), (25, 15, -5));
    assert_eq!(l.collider_cylinder.info.ac_dmg_info.dmg_flags, DMG_ARROW | DMG_SLINGSHOT);
    assert_eq!((l.collider_quad.base.at_flags, l.collider_quad.base.ac_flags), (AT_ON | AT_TYPE_ENEMY, AC_ON | AC_TYPE_PLAYER));
    assert_eq!((l.state, l.actor.navi_enemy_id), (PEAHAT_STATE_SEEK_PLAYER, 0x49));
}

#[test]
fn a_grounded_peahat_rises_by_day_within_740_and_stays_down_at_night() {
    let Some(a) = assets() else { return };
    // By day, within 740: EnPeehat_Ground_SetStateRise (state 8).
    let mut w = field(&a, 10, 0);
    let near = w.actor_spawn(ACTOR_EN_PEEHAT, OPEN_FIELD + Vec3::new(700.0, 0.0, 0.0), [0; 3], PEAHAT_TYPE_GROUNDED).unwrap();
    let far = w.actor_spawn(ACTOR_EN_PEEHAT, OPEN_FIELD + Vec3::new(0.0, 0.0, 760.0), [0; 3], PEAHAT_TYPE_GROUNDED).unwrap();
    idle(&mut w, 2);
    assert_eq!(peahat(&w, near).state, PEAHAT_STATE_8);
    assert_eq!(peahat(&w, far).state, PEAHAT_STATE_3);
    assert_ne!(peahat(&w, far).actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0, "targetable by day");
    // Up: Ground_StateRise into Ground_SetStateHover (state 15), then seeking Link (14).
    for _ in 0..120 {
        idle(&mut w, 1);
        if peahat(&w, near).state == PEAHAT_STATE_SEEK_PLAYER {
            break;
        }
    }
    assert_eq!(peahat(&w, near).state, PEAHAT_STATE_SEEK_PLAYER);
    // At night, Link 300 away: it stays down, untargetable, its yOffset from 100 to -1000 by 50 a
    // frame (Math_SmoothStepToF(.., -1000, 1, 50, 0)).
    let mut w = field(&a, 22, 0);
    let h = w.actor_spawn(ACTOR_EN_PEEHAT, OPEN_FIELD + Vec3::new(300.0, 0.0, 0.0), [0; 3], PEAHAT_TYPE_GROUNDED).unwrap();
    idle(&mut w, 1);
    assert_eq!((peahat(&w, h).state, peahat(&w, h).actor.shape_y_offset), (PEAHAT_STATE_3, 50.0));
    assert_eq!(peahat(&w, h).actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    idle(&mut w, 30);
    assert_eq!((peahat(&w, h).state, peahat(&w, h).actor.shape_y_offset), (PEAHAT_STATE_3, -1000.0));
}

#[test]
fn a_hit_on_its_root_at_night_lets_out_its_larvae() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, 22, 0);
    let h = w.actor_spawn(ACTOR_EN_PEEHAT, OPEN_FIELD + Vec3::new(300.0, 0.0, 0.0), [0; 3], PEAHAT_TYPE_GROUNDED).unwrap();
    idle(&mut w, 1);
    // EnPeehat_HitWhenGrounded, on a frame not a multiple of 16: MAX_LARVA - unk_2FA larvae (3),
    // each its child, 6 up; then 8 frames' wobble.
    while (w.gameplay_frames + 1) & 0xF == 0 {
        idle(&mut w, 1);
    }
    peahat_mut(&mut w, h).collider_cylinder.base.ac_flags |= AC_HIT;
    idle(&mut w, 1);
    let larvae: Vec<ActorHandle> = all::<EnPeehat>(&w).into_iter().filter(|&l| peahat(&w, l).actor.params == PEAHAT_TYPE_LARVA).collect();
    assert_eq!(larvae.len(), 3);
    assert_eq!((peahat(&w, h).unk_2fa, peahat(&w, h).unk_2d4), (3, 8));
    for &l in &larvae {
        assert_eq!(peahat(&w, l).actor.parent, Some(h));
    }
    // EnPeehat_Destroy: a larva gone counts itself off its parent.
    peahat_mut(&mut w, larvae[0]).actor.kill();
    idle(&mut w, 1);
    assert_eq!(peahat(&w, h).unk_2fa, 2);
}

#[test]
fn a_peahat_reacts_to_hits_by_its_damage_table() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, 0);
    let h = w.actor_spawn(ACTOR_EN_PEEHAT, OPEN_FIELD + Vec3::new(0.0, 0.0, 2000.0), [0; 3], PEAHAT_TYPE_GROUNDED).unwrap();
    idle(&mut w, 1);
    let hit = |w: &mut PlayState, reaction: u8, damage: u8| {
        let p = peahat_mut(w, h);
        p.actor.col_chk_info.damage_reaction = reaction;
        p.actor.col_chk_info.damage = damage;
        p.collider_jnt_sph.base.ac_flags |= AC_HIT;
    };
    // A Deku Nut (PEAHAT_DMG_REACT_NUT): nothing.
    hit(&mut w, PEAHAT_DMG_REACT_NUT, 0);
    idle(&mut w, 1);
    assert_eq!((peahat(&w, h).state, peahat(&w, h).actor.col_chk_info.health), (PEAHAT_STATE_3, 6));
    // The boomerang: EnPeehat_SetStateBoomerangStunned, blue at 200 for 80 frames
    // (Actor_SetColorFilter: 0x0000 | 0x0000 | (200 & 0xF8) << 5 | 80).
    hit(&mut w, PEAHAT_DMG_REACT_BOOMERANG, 0);
    idle(&mut w, 1);
    let p = peahat(&w, h);
    assert_eq!((p.state, p.actor.color_filter_params, p.actor.color_filter_timer), (PEAHAT_STATE_STUNNED, (200 & 0xF8) << 5 | 80, 80));
    // The sword (PEAHAT_DMG_REACT_ATTACK, 1): a point off, red at 255 for 8.
    hit(&mut w, PEAHAT_DMG_REACT_ATTACK, 1);
    idle(&mut w, 1);
    let p = peahat(&w, h);
    assert_eq!((p.actor.col_chk_info.health, p.actor.color_filter_params), (5, COLORFILTER_COLORFLAG_RED | (255 & 0xF8) << 5 | 8));
    // The hookshot: no health at once, EnPeehat_Adult_SetStateDie (state 0); in the same update
    // EnPeehat_Adult_StateDie's first pass (health 0: the recoil, 6 up, unk_2D4 14); 14 updates
    // more to EnPeehat_SetStateExplode (state 1); its first update spawns the bomb (En_Bom: a
    // placeholder), and it's killed when animTimer reaches 0, five updates in.
    hit(&mut w, PEAHAT_DMG_REACT_HOOKSHOT, 2);
    idle(&mut w, 1);
    let p = peahat(&w, h);
    assert_eq!((p.state, p.actor.col_chk_info.health, p.unk_2d4, p.actor.velocity.y), (PEAHAT_STATE_DYING, 0, 14, 6.0));
    idle(&mut w, 13);
    assert_eq!(peahat(&w, h).state, PEAHAT_STATE_DYING);
    idle(&mut w, 1);
    assert_eq!((peahat(&w, h).state, peahat(&w, h).anim_timer), (PEAHAT_STATE_EXPLODE, 5));
    let bombs = |w: &PlayState| w.actors.all().into_iter().filter(|&b| w.actors.actor(b).is_some_and(|a| a.id == 0x0010)).count();
    assert_eq!(bombs(&w), 0);
    idle(&mut w, 1);
    assert_eq!(bombs(&w), 1, "En_Bom (rot.z 0x602) at its place");
    idle(&mut w, 3);
    assert!(w.actors.exists(h));
    idle(&mut w, 2);
    assert!(!w.actors.exists(h), "Actor_Kill when animTimer reaches 0");
}

#[test]
fn stalchildren_initialise_as_their_c_does_and_sink_by_day() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, 22, 0);
    let small = w.actor_spawn(en_skb::ACTOR_EN_SKB, OPEN_FIELD + Vec3::new(400.0, 0.0, 0.0), [0; 3], 0).unwrap();
    let big = w.actor_spawn(en_skb::ACTOR_EN_SKB, OPEN_FIELD + Vec3::new(0.0, 0.0, 400.0), [0; 3], 5).unwrap();
    // EnSkb_Init: scale (params × 0.1 + 1) × 0.01, its spheres 10 + params and 20 + 2 params,
    // health 2, yOffset -8000, rising untargetable (EnSkb_SetupRiseFromGround).
    for (h, scale, r0, r1) in [(small, 0.01, 10, 20), (big, 0.015, 15, 30)] {
        let s = w.actors.downcast::<EnSkb>(h).unwrap();
        assert_eq!((s.actor.scale.x, s.collider.elements[0].dim.model_sphere.radius, s.collider.elements[1].dim.model_sphere.radius), (scale, r0, r1));
        assert_eq!((s.actor.col_chk_info.health, s.actor.shape_y_offset, s.action), (2, -8000.0, en_skb::Action::RiseFromGround));
        assert_eq!(s.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    }
    // By day one rises, decides (EnSkb_DecideNextAction: IS_DAY) to sink, and goes at the
    // uncurling's start.
    let mut w = field(&a, 10, 0);
    let h = w.actor_spawn(en_skb::ACTOR_EN_SKB, OPEN_FIELD + Vec3::new(400.0, 0.0, 0.0), [0; 3], 0).unwrap();
    let mut sank = false;
    for _ in 0..300 {
        idle(&mut w, 1);
        match w.actors.downcast::<EnSkb>(h) {
            Some(s) => sank |= s.action == en_skb::Action::Despawn,
            None => break,
        }
    }
    assert!(sank && !w.actors.exists(h));
}

/// Until the Stalchild has risen (`actionState` attacking or past).
fn risen(w: &mut PlayState, h: ActorHandle) {
    for _ in 0..200 {
        idle(w, 1);
        if w.actors.downcast::<EnSkb>(h).is_some_and(|s| s.action_state >= en_skb::SKB_BEHAVIOR_ATTACKING) {
            return;
        }
    }
    panic!("the Stalchild never rose");
}

/// The `En_Part`s there are, by list.
fn parts(w: &PlayState) -> Vec<(&'static str, &'static str)> {
    all::<oot_actors::en_part::EnPart>(w).into_iter().filter_map(|h| w.actors.downcast::<oot_actors::en_part::EnPart>(h).unwrap().display_list).collect()
}

#[test]
fn a_horizontal_slash_knocks_a_stalchilds_head_off() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, 22, 0);
    let h = w.actor_spawn(en_skb::ACTOR_EN_SKB, OPEN_FIELD + Vec3::new(300.0, 0.0, 0.0), [0; 3], 0).unwrap();
    risen(&mut w, h);
    // The Kokiri Sword's slash (DMG_ENTRY(1, 0xE)) with PLAYER_MWA_RIGHT_SLASH_1H: a point off,
    // BodyBreak_Alloc(2), breakFlags 1, EnSkb_SetupTakeDamage.
    w.player_mut().melee_weapon_animation = 4;
    let s = w.actors.downcast_mut::<EnSkb>(h).unwrap();
    s.actor.col_chk_info.damage_reaction = 0xE;
    s.actor.col_chk_info.damage = 1;
    s.collider.base.ac_flags |= AC_HIT;
    idle(&mut w, 1);
    let s = w.actors.downcast::<EnSkb>(h).unwrap();
    assert_eq!((s.actor.col_chk_info.health, s.break_flags, s.action), (1, 1, en_skb::Action::TakeDamage));
    // The draw takes the head's and the jaw's matrices (BodyBreak_SetInfo, limbs 11 and 12);
    // EnSkb_TakeDamage spawns them (BodyBreak_SpawnParts, En_Part type 1), then it's headless.
    for _ in 0..10 {
        idle(&mut w, 1);
        if w.actors.downcast::<EnSkb>(h).unwrap().break_flags & 2 != 0 {
            break;
        }
    }
    assert_eq!(w.actors.downcast::<EnSkb>(h).unwrap().break_flags, 3);
    let mut p: Vec<&str> = parts(&w).into_iter().map(|d| d.1).collect();
    p.sort();
    assert_eq!(p, vec!["gStalchildHeadDL", "gStalchildJawDL"]);
}

#[test]
fn stalchildren_rise_round_link_at_night_and_break_up_dying() {
    use oot_actors::en_encount1::EnEncount1;
    let Some(a) = assets() else { return };
    // By day the spawner spawns nothing (IS_DAY: killCount 0).
    let mut w = field(&a, 10, 0);
    idle(&mut w, 30);
    assert!(all::<EnSkb>(&w).is_empty());
    // At night, Link on the grass (floorSfxOffset not dirt, on the scene's floor, on the ground,
    // out of water): two at once (maxCurSpawns 2, params 0x10A4), the first 200 ± 20 ahead of him
    // (± 20 across), the second 100 ± 20 at his facing negated; then fieldSpawnTimer 100.
    let mut w = field(&a, 22, 0);
    let mut skbs = all::<EnSkb>(&w);
    // (The one ahead first.)
    skbs.sort_by(|&p, &q| w.actors.actor(q).unwrap().world_pos.x.total_cmp(&w.actors.actor(p).unwrap().world_pos.x));
    assert_eq!(skbs.len(), 2);
    let link = w.player().actor.world_pos;
    let enc = all::<EnEncount1>(&w)[0];
    let e = w.actors.downcast::<EnEncount1>(enc).unwrap();
    assert_eq!((e.cur_num_spawn, e.max_cur_spawns), (2, 2));
    let first = w.actors.actor(skbs[0]).unwrap().world_pos;
    // Facing +x (yaw 0x4000): ahead is +x; the second's angle -0x4000 is -x.
    assert!((first.x - link.x) > 180.0 - 20.0 - 1.0 && (first.x - link.x) < 220.0 + 20.0 + 1.0 && (first.z - link.z).abs() <= 20.0, "{first} from {link}");
    let second = w.actors.actor(skbs[1]).unwrap().world_pos;
    assert!((link.x - second.x) > 80.0 - 20.0 - 1.0 && (link.x - second.x) < 120.0 + 20.0 + 1.0 && (second.z - link.z).abs() <= 20.0, "{second} from {link}");
    for &s in &skbs {
        assert_eq!(w.actors.actor(s).unwrap().parent, Some(enc));
    }
    // One killed: EnSkb_SetupDeath (BodyBreak_Alloc(18)), its limbs with lists broken off as
    // En_Parts once the draw has set them, then its drop and gone; EnSkb_Destroy counts it off the
    // spawner. BodyBreak_SetInfo is ready at its 18th limb (`count` counts the limbs seen), so the
    // spine (limb 19) is never taken: 16 pieces (limbs 2 and 4 to 18; 1 and 3 have no list).
    let h = skbs[1];
    risen(&mut w, h);
    let s = w.actors.downcast_mut::<EnSkb>(h).unwrap();
    s.actor.col_chk_info.health = 1;
    s.actor.col_chk_info.damage_reaction = 0xF;
    s.actor.col_chk_info.damage = 1;
    s.collider.base.ac_flags |= AC_HIT;
    idle(&mut w, 1);
    assert_eq!(w.actors.downcast::<EnSkb>(h).unwrap().action, en_skb::Action::Death);
    for _ in 0..10 {
        idle(&mut w, 1);
        if !w.actors.exists(h) {
            break;
        }
    }
    assert!(!w.actors.exists(h));
    assert_eq!(parts(&w).len(), 16);
    assert_eq!(w.actors.downcast::<EnEncount1>(enc).unwrap().cur_num_spawn, 1);
}

#[test]
fn a_stalchilds_death_cry_plays_once() {
    use oot_game::audio::sfx::NA_SE_EN_STALKID_DEAD;
    let Some(a) = assets() else { return };
    // The same field at 22:00 with the audio logged (and its side running: the field changes the
    // audio spec).
    let e = a.scenes.entrance_index("ENTR_HYRULE_FIELD_0").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(22, 0) as u16);
    save.apply_preset("deku-tree-dead").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    w.audio_side = Some(Box::new(oot_game::audio::offline::OfflineAudio::new(&a.pack.audio_data().unwrap(), false)));
    w.place_player(OPEN_FIELD, 0x4000);
    idle(&mut w, 2);
    let h = w.actor_spawn(en_skb::ACTOR_EN_SKB, OPEN_FIELD + Vec3::new(300.0, 0.0, 0.0), [0; 3], 0).unwrap();
    risen(&mut w, h);
    let s = w.actors.downcast_mut::<EnSkb>(h).unwrap();
    s.actor.col_chk_info.health = 1;
    s.actor.col_chk_info.damage_reaction = 0xF;
    s.actor.col_chk_info.damage = 1;
    s.collider.base.ac_flags |= AC_HIT;
    let first = w.gameplay_frames;
    idle(&mut w, 60);
    // EnSkb_SetupDeath: EffectSsDeadSound_SpawnStationary(.., repeatMode 1, life 40).
    // DEADSOUND_REPEAT_MODE_OFF (1) plays it and counts down to 0, which plays nothing; ON is 2.
    let log = w.audio.log.as_ref().unwrap();
    let cries = log.sfx.iter().filter(|(f, id, _)| *f >= first && *id == NA_SE_EN_STALKID_DEAD).count();
    assert_eq!(cries, 1);
}

#[test]
fn a_hostile_enemy_within_500_plays_the_enemy_music() {
    use oot_game::audio::SEQ_MODE_ENEMY;
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, 0);
    let h = w.actor_spawn(ACTOR_EN_PEEHAT, OPEN_FIELD + Vec3::new(700.0, 0.0, 0.0), [0; 3], PEAHAT_TYPE_GROUNDED).unwrap();
    idle(&mut w, 2);
    assert!(w.target_ctx.bgm_enemy.is_none());
    assert_ne!(w.audio.seq_mode_input, SEQ_MODE_ENEMY);
    // Attention_FindActorInCategory: a targetable, hostile enemy whose xyzDistToPlayerSq is under
    // 500²; Player's next update sets SEQ_MODE_ENEMY (Audio_SetBgmEnemyVolume by the distance).
    let mut near_at = None;
    for f in 0..300 {
        idle(&mut w, 1);
        if near_at.is_none() && peahat(&w, h).actor.xyz_dist_to_player_sq < 500.0 * 500.0 {
            near_at = Some(f);
        }
        if near_at.is_some_and(|n| f >= n + 2) {
            break;
        }
    }
    assert!(near_at.is_some(), "the Peahat came within 500");
    assert_eq!(w.target_ctx.bgm_enemy, Some(h));
    assert_eq!(w.audio.seq_mode_input, SEQ_MODE_ENEMY);
}

#[test]
fn a_roll_into_a_tree_bonks_it() {
    use oot_actors::en_wood02::EnWood02;
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, 0);
    // A placed tree (type below 11, not a spawner's child): Link 150 west of it, running at it
    // and rolling (A) at full speed.
    // (Away from the signs and the owls, which A would talk to.)
    let talkers: Vec<Vec3> = w
        .actors
        .all()
        .into_iter()
        .filter(|&h| w.actors.downcast::<oot_actors::en_a_obj::EnAObj>(h).is_some() || w.actors.downcast::<oot_actors::en_owl::EnOwl>(h).is_some())
        .map(|h| w.actors.actor(h).unwrap().world_pos)
        .collect();
    let tree = all::<EnWood02>(&w)
        .into_iter()
        .find(|&h| {
            let t = w.actors.downcast::<EnWood02>(h).unwrap();
            t.actor.params & 0xFF < 11 && t.actor.parent.is_none() && talkers.iter().all(|p| p.distance(t.actor.world_pos) > 1000.0)
        })
        .expect("a tree");
    let at = w.actors.actor(tree).unwrap().world_pos;
    w.place_player(at - Vec3::new(150.0, 0.0, 0.0), 0x4000);
    let leaves = |w: &PlayState| all::<EnWood02>(w).into_iter().filter(|&h| w.actors.actor(h).unwrap().params >= 23).count();
    assert_eq!(leaves(&w), 0);
    let mut prev = PadState::default();
    let mut bonked = false;
    for f in 0..90 {
        let mut p = oot_actors::script::stick_towards(&w, at, 80.0);
        if f == 12 {
            p.button = eng_input::pad::BTN_A;
        }
        w.tick_with(scripted_input(prev, p));
        prev = p;
        // EnWood02_Update: home.rot.y set, the drop (its table), the swing and four leaves;
        // unk_14C to -21.
        if w.actors.downcast::<EnWood02>(tree).unwrap().unk_14c < -1 {
            bonked = true;
            break;
        }
    }
    assert!(bonked, "the tree took the bonk (player {:?})", w.player().action);
    assert_eq!(leaves(&w), 4);
}

#[test]
fn an_open_grotto_hole_takes_link_down() {
    use oot_actors::door_ana::DoorAna;
    let Some(a) = assets() else { return };
    let mut w = field(&a, 10, 0);
    // Door_Ana 0x0003 near Lake Hylia: open (bits 8 and 9 clear); Link walking onto it.
    let hole = all::<DoorAna>(&w).into_iter().find(|&h| w.actors.actor(h).unwrap().params == 0x0003).expect("the hole");
    let at = w.actors.actor(hole).unwrap().world_pos;
    w.place_player(at + Vec3::new(60.0, 0.0, 0.0), -0x4000);
    let mut prev = PadState::default();
    for _ in 0..300 {
        let p = if w.player().state1 & oot_actors::player::STATE1_29 == 0 { oot_actors::script::stick_towards(&w, at, 40.0) } else { PadState::default() };
        w.tick_with(scripted_input(prev, p));
        prev = p;
        if w.scene_id == oot_game::play_scene::SCENE_GROTTOS {
            break;
        }
    }
    assert_eq!(w.scene_id, oot_game::play_scene::SCENE_GROTTOS);
    // The return point: RESPAWN_MODE_RETURN at the hole, Link launched up on return
    // (PLAYER_START_MODE_GROTTO, params 0x04FF).
    let r = &w.save.respawn[oot_game::save::RESPAWN_MODE_RETURN];
    assert_eq!(r.player_params, 0x04FF);
    // (Play_SetupRespawnPoint: Link's place, its y the hole's.)
    assert!(r.pos.y == at.y && Vec3::new(r.pos.x - at.x, 0.0, r.pos.z - at.z).length() <= 15.0, "{} at {}", r.pos, at);
}

fn run_route(route: Route) -> (Playthrough, PlayState) {
    let a = assets().unwrap();
    let e = a.scenes.entrance_index(route.entrance()).expect("entrance");
    let save = route.save(e);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    route.debug_start(&mut w);
    let mut run = Playthrough::for_route(route);
    let mut prev = PadState::default();
    while let Some(p) = run.next(&w) {
        w.tick_with(scripted_input(prev, p));
        prev = p;
    }
    assert!(run.failure.is_none(), "{:?}", run.failure);
    eprintln!("steps {:?}", run.steps);
    (run, w)
}

#[test]
fn exit_hyrule_field_by_day_past_a_peahat_to_castle_town() {
    if assets().is_none() {
        return;
    }
    let (run, w) = run_route(Route::Field);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::PeahatKilled, Step::CastleTown]);
    assert_eq!(w.scene_id, SCENE_MARKET_ENTRANCE_DAY);
    assert!(w.save.health > 0);
}

#[test]
fn exit_hyrule_field_at_night_past_stalchildren_to_kakariko() {
    if assets().is_none() {
        return;
    }
    let (run, w) = run_route(Route::FieldNight);
    let steps: Vec<Step> = run.steps.iter().map(|s| s.0).collect();
    assert_eq!(steps, vec![Step::StalchildrenKilled, Step::OwlTalk, Step::OwlFlown, Step::StalchildrenGone, Step::Kakariko]);
    assert_eq!(w.scene_id, SCENE_KAKARIKO_VILLAGE);
    assert!(w.save.health > 0);
}
