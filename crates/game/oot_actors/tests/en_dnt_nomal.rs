//! `En_Dnt_Nomal` (`z_en_dnt_nomal.c`): the Lost Woods' target scrub (room 1's placement,
//! params 0xFFFF at (1368, 80, -173)) and the mask contest's stage scrubs.
//!
//! The target scrub's ledge is 80 to 106 high; the target is on the cliff below its edge at
//! x 1340, facing -x, over the stumps' floor (-180). The seeds are real `En_Arrow` seeds, shot as
//! Player's shot leaves them (`Actor_Spawn` at their pitch and yaw, `unk_A73` 4, as
//! tests/arrows.rs does), or from Player's own slingshot (the full game). Expected values are
//! worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_CRIGHT, PadState};
use eng_math::{atan2_s, vec3f_yaw};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_arrow::{ACTOR_EN_ARROW, ARROW_SEED};
use oot_actors::en_dnt_nomal::*;
use oot_actors::en_ex_item::{EXITEM_BULLET_BAG, EnExItem, GI_BULLET_BAG_40};
use oot_actors::en_ex_ruppy::EnExRuppy;
use oot_actors::playthrough::deku_tree_room_start;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_PROP, ActorHandle, ActorImpl};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::{AC_HIT, AC_ON, AC_TYPE_PLAYER, ColliderRef, MASS_IMMOVABLE, OC1_ON};
use oot_game::effect::{EFFECT_SS_EXTRA, EFFECT_SS_HAHEN, EffectSs};
use oot_game::item::{ITEM_SLINGSHOT, UPG_BULLET_BAG, item_give};
use oot_game::message::TEXT_STATE_NONE;
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

fn press(button: u16) -> PadState {
    PadState { button, ..Default::default() }
}

/// Where Link stands to shoot: on the stumps' floor west of the target, facing it (+x).
const LINK_SPOT: (Vec3, i16) = (Vec3::new(1100.0, -180.0, -30.0), 0x4000);
/// The placement's home.
const HOME: Vec3 = Vec3::new(1368.0, 80.0, -173.0);

/// The Lost Woods' room 1 (from `ENTR_LOST_WOODS_0`, room 0) as a child with the slingshot (the
/// 30 bag: `UPG_BULLET_BAG` 1), the sound log on, `f` changing the save first; Link at
/// `LINK_SPOT`, settled.
fn woods(a: &Arc<GameAssets>, f: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_LOST_WOODS_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    after_sarias_greeting(&mut save);
    item_give(&mut save, None, ITEM_SLINGSHOT);
    save.equip_item_on_c(2, ITEM_SLINGSHOT);
    f(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 20);
    deku_tree_room_start(&mut w, 1, LINK_SPOT.0, LINK_SPOT.1);
    idle(&mut w, 20);
    w
}

fn scrubs(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnDntNomal>(h).is_some()).collect()
}

fn scrub(w: &PlayState) -> ActorHandle {
    let v = scrubs(w);
    assert_eq!(v.len(), 1, "one En_Dnt_Nomal");
    v[0]
}

fn get(w: &PlayState, h: ActorHandle) -> &EnDntNomal {
    w.actors.downcast::<EnDntNomal>(h).expect("En_Dnt_Nomal")
}

fn get_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnDntNomal {
    w.actors.downcast_mut::<EnDntNomal>(h).expect("En_Dnt_Nomal")
}

fn effects(w: &PlayState, ty: u8) -> Vec<&EffectSs> {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == ty).collect()
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// `Math_Vec3f_Pitch(a, b)`: positive looking down.
fn vec3f_pitch(a: Vec3, b: Vec3) -> i16 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    atan2_s((dx * dx + dz * dz).sqrt(), a.y - b.y)
}

/// A seed shot level at `at` from 190 west of the target (`En_Arrow` `ARROW_SEED` at yaw 0x4000,
/// Player's `unk_A73` 4: `EnArrow_Shoot` lets it fly; no gravity for its first 7 frames), flown
/// until it's gone. Its quad (`func_809B4800`: ±4 up and down, 15 ahead) crosses the target
/// within 4 of `at`.
fn shoot(w: &mut PlayState, at: Vec3) {
    let start = Vec3::new(1150.0, at.y, at.z);
    w.player_mut().unk_A73 = 4;
    let h = w.actor_spawn(ACTOR_EN_ARROW, start, [vec3f_pitch(start, at), vec3f_yaw(start, at), 0], ARROW_SEED).expect("En_Arrow");
    for _ in 0..30 {
        idle(w, 1);
        if !w.actors.exists(h) {
            return;
        }
    }
    panic!("the seed never landed");
}

/// The score effects (`Effect_Ss_Extra`): their `rScoreIdx` (regs 2) and `rScale` (regs 3).
fn scores(w: &PlayState) -> Vec<(i16, i16, Vec3)> {
    effects(w, EFFECT_SS_EXTRA).iter().map(|e| (e.regs[2], e.regs[3], e.pos)).collect()
}

#[test]
fn the_placement_waits_for_its_object_and_sets_its_flower_on_the_floor() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    // The placement: params 0xFFFF, read as an s16 (-1), below ENDNTNOMAL_TARGET: the target.
    let h = scrub(&w);
    let s = get(&w, h);
    assert_eq!((s.actor.params, s.ty, s.object_id), (-1, ENDNTNOMAL_TARGET, OBJECT_HINTNUTS));
    assert_eq!(s.required_object_slot, w.object_ctx.get_index(OBJECT_HINTNUTS));
    assert!(s.required_object_slot.is_some());
    // EnDntNomal_Init: untargetable, immovable, facing yawTowardsPlayer (0 before any update).
    assert_eq!((s.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, s.actor.col_chk_info.mass), (0, MASS_IMMOVABLE));
    assert_eq!((s.actor.world_rot.y, s.actor.shape_rot.y), (0, 0));
    // Waited for, then: gravity -2, scale 0.01, drawn as the target scrub; on the floor its flower
    // where it stands; EnDntNomal_SetupTargetWait: gHintNutsBurrowAnim held (speed 0) at frame 8,
    // endFrame its last frame.
    let floor = w.col.entity_raycast_down(HOME + Vec3::Y * 10.0).0;
    assert_eq!((s.actor.gravity, s.actor.scale, s.draw), (-2.0, Vec3::splat(0.01), Some(Draw::Target)));
    assert_eq!(s.flower_pos, Vec3::new(HOME.x, floor, HOME.z));
    let burrow = a.animation("object_hintnuts", "gHintNutsBurrowAnim").unwrap();
    assert_eq!((s.action_func, s.skel.cur_frame, s.skel.play_speed, s.end_frame), (Action::TargetWait, 8.0, 0.0, burrow.last_frame() as i16));
    // EnDntNomal_TargetWait's quad, set and checked for hits each frame (CollisionCheck_SetAC).
    assert_eq!(
        s.target_collider_quad.dim.quad,
        [Vec3::new(1340.0, 26.0, -6.0), Vec3::new(1340.0, 26.0, -54.0), Vec3::new(1340.0, 74.0, -6.0), Vec3::new(1340.0, 74.0, -54.0)]
    );
    assert_eq!(s.target_collider_quad.base.ac_flags & (AC_ON | AC_TYPE_PLAYER), AC_ON | AC_TYPE_PLAYER);
    assert_eq!(s.target_collider_quad.info.ac_dmg_info.dmg_flags, 0x0001_F824);
    assert!(w.col_chk.col_ac.contains(&ColliderRef { actor: h, id: 0 }));

    // The same, spawned again step by step with its object still loading: EnDntNomal_WaitForObject
    // waits (nothing drawn), then its first update with the object in sets it up and falls.
    let bank = w.object_ctx.get_index(OBJECT_HINTNUTS).unwrap();
    w.object_ctx.status[bank].id = -OBJECT_HINTNUTS;
    w.object_ctx.status[bank].dma_started = false;
    let h2 = w.actor_spawn(ACTOR_EN_DNT_NOMAL, HOME, [0, 0, 0], -1).expect("spawned");
    assert_eq!(get(&w, h2).action_func, Action::WaitForObject);
    // Object_UpdateEntries: the DMA started this frame, done the next.
    idle(&mut w, 1);
    let s = get(&w, h2);
    assert_eq!((s.action_func, s.draw, s.actor.gravity), (Action::WaitForObject, None, 0.0));
    let mut n = 0;
    while get(&w, h2).action_func == Action::WaitForObject {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 5);
    }
    // EnDntNomal_WaitForObject ran: then Actor_MoveXZGravity at -2 a frame (1.5 times that), and
    // the floor check puts it back on the floor: BGCHECKFLAG_GROUND.
    let s = get(&w, h2);
    assert_eq!((s.action_func, s.draw, s.actor.gravity), (Action::SetFlower, Some(Draw::Target), -2.0));
    assert!(s.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0);
    idle(&mut w, 1);
    assert_eq!((get(&w, h2).action_func, get(&w, h2).flower_pos), (Action::SetupTargetWait, Vec3::new(HOME.x, floor, HOME.z)));
    idle(&mut w, 1);
    assert_eq!((get(&w, h2).action_func, get(&w, h2).skel.cur_frame), (Action::TargetWait, 8.0));
}

#[test]
fn a_bullseye_scores_100_the_ring_30_and_resets_the_count() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    let h = scrub(&w);
    let pos = get(&w, h).actor.world_pos;
    // A seed at the centre: within 8 of it. EffectSsExtra_Spawn(scale 4, EXTRA_SCORE_100) at 20 to
    // the scrub's -x and 20 up, rising at 5 (the effect's own update moves it the same frame:
    // EffectSs_UpdateAll runs after the actors); Audio_StopSfxById and Sfx_PlaySfxCentered of
    // NA_SE_SY_TRE_BOX_APPEAR; a child without ITEMGETINF_1D: hitCounter 1.
    shoot(&mut w, TARGET_POS);
    let s = scores(&w);
    assert_eq!(s.len(), 1, "{s:?}");
    assert_eq!((s[0].0, s[0].1), (oot_game::effect::extra::EXTRA_SCORE_100, 4));
    assert_eq!((s[0].2.x, s[0].2.z), (pos.x - 20.0, pos.z));
    assert!(s[0].2.y >= pos.y + 20.0, "{}", s[0].2);
    assert_eq!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR).len(), 1);
    let sc = get(&w, h);
    assert_eq!((sc.hit_counter, sc.action_func), (1, Action::TargetWait));
    assert_eq!(sc.target_collider_quad.base.ac_flags & AC_HIT, 0);
    // A second: 2.
    idle(&mut w, 60);
    shoot(&mut w, TARGET_POS);
    assert_eq!(get(&w, h).hit_counter, 2);
    // At the ring, 15 above the centre (within 24, not 8): EXTRA_SCORE_30 20 above the scrub, and
    // the count starts over; no chime.
    idle(&mut w, 60);
    let chimes = sfx_frames(&w, oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR).len();
    shoot(&mut w, TARGET_POS + Vec3::new(0.0, 15.0, 0.0));
    let s = scores(&w);
    assert_eq!(s.len(), 1, "{s:?}");
    assert_eq!((s[0].0, s[0].1, s[0].2.x, s[0].2.z), (oot_game::effect::extra::EXTRA_SCORE_30, 4, pos.x, pos.z));
    assert_eq!(get(&w, h).hit_counter, 0);
    assert_eq!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR).len(), chimes);
    // In the quad's corner, 20 off both ways (28 from the centre): a hit, but nothing scores.
    idle(&mut w, 60);
    get_mut(&mut w, h).hit_counter = 2;
    shoot(&mut w, TARGET_POS + Vec3::new(0.0, 20.0, 20.0));
    assert!(scores(&w).is_empty());
    assert_eq!(get(&w, h).hit_counter, 2);
}

#[test]
fn with_itemgetinf_1d_or_as_an_adult_bullseyes_dont_count() {
    let Some(a) = assets() else { return };
    // ITEMGETINF_1D set: the score and the chime, but no count.
    let mut w = woods(&a, |s| s.set_item_get_inf(ITEMGETINF_1D));
    let h = scrub(&w);
    for _ in 0..3 {
        shoot(&mut w, TARGET_POS);
        assert_eq!(scores(&w).len(), 1);
        idle(&mut w, 60);
    }
    let s = get(&w, h);
    assert_eq!((s.hit_counter, s.action_func), (0, Action::TargetWait));
    assert_eq!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR).len(), 3);
    // LINK_IS_ADULT (the save's age; the adult layers don't place it, so the child's woods with
    // the age read as adult): the same.
    let mut w = woods(&a, |_| {});
    let h = scrub(&w);
    w.save.adult = true;
    for _ in 0..3 {
        shoot(&mut w, TARGET_POS);
        idle(&mut w, 60);
    }
    assert_eq!((get(&w, h).hit_counter, get(&w, h).action_func), (0, Action::TargetWait));
}

/// What `three_bullseyes_win_the_bullet_bag` saw, frame by frame.
#[derive(Default, Debug)]
struct Seen {
    unburrow: Option<u32>,
    walk: Option<u32>,
    face: Option<u32>,
    talk: Option<u32>,
    talk_done: Option<u32>,
    bag: Option<(u32, ActorHandle)>,
    back_home: Option<u32>,
    hidden: Option<u32>,
    offered: Option<u32>,
    taken: Option<u32>,
    flag: Option<u32>,
    bag_gone: Option<u32>,
}

#[test]
fn three_bullseyes_win_the_bullet_bag() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    let h = scrub(&w);
    let flower = get(&w, h).flower_pos;
    shoot(&mut w, TARGET_POS);
    idle(&mut w, 60);
    shoot(&mut w, TARGET_POS);
    idle(&mut w, 60);
    assert_eq!(get(&w, h).hit_counter, 2);
    // The third: the frame it's seen, OnePointCutscene_Init(4140, -99, the scrub, CAM_ID_MAIN) (a
    // sub camera active with its csId), Player_SetCsActionWithHaltedActors(the scrub,
    // PLAYER_CSACTION_1), timer4 50, EnDntNomal_SetupTargetUnburrow.
    let start = Vec3::new(1150.0, TARGET_POS.y, TARGET_POS.z);
    w.player_mut().unk_A73 = 4;
    w.actor_spawn(ACTOR_EN_ARROW, start, [0, 0x4000, 0], ARROW_SEED).expect("En_Arrow");
    let mut hit = false;
    for _ in 0..10 {
        idle(&mut w, 1);
        if get(&w, h).action_func != Action::TargetWait {
            hit = true;
            break;
        }
    }
    assert!(hit, "the third bullseye");
    let hit_frame = w.audio.frames;
    let s = get(&w, h);
    assert_eq!((s.action_func, s.hit_counter, s.timer4), (Action::SetupTargetUnburrow, 3, 50));
    assert_ne!(w.active_cam_id, CAM_ID_MAIN);
    assert_eq!(w.camera(w.active_cam_id).unwrap().cs_id, 4140);
    assert_eq!((w.player().cs_mode, w.player().cs_actor), (1, Some(h)));
    // Then frame by frame, A pressed through the texts (every other frame).
    let mut seen = Seen::default();
    let mut prev = PadState::default();
    let mut bag_spawn_mouth = Vec3::ZERO;
    let mut bag_home = Vec3::ZERO;
    for _ in 0..1500 {
        let pad = if w.message_state() != TEXT_STATE_NONE && prev.button & BTN_A == 0 { press(BTN_A) } else { PadState::default() };
        let mouth_before = get(&w, h).mouth_pos;
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        let f = w.audio.frames;
        let s = get(&w, h);
        let first = |o: &mut Option<u32>| {
            if o.is_none() {
                *o = Some(f);
            }
        };
        match s.action_func {
            Action::TargetUnburrow => first(&mut seen.unburrow),
            Action::TargetWalk => first(&mut seen.walk),
            Action::TargetFacePlayer => first(&mut seen.face),
            Action::TargetTalk => first(&mut seen.talk),
            Action::SetupTargetGivePrize | Action::TargetGivePrize => first(&mut seen.talk_done),
            Action::TargetWait if seen.talk_done.is_some() => first(&mut seen.back_home),
            _ => {}
        }
        if seen.unburrow == Some(f) {
            // EnDntNomal_SetupTargetUnburrow: gHintNutsUnburrowAnim, the burst 50 up,
            // NA_SE_EN_NUTS_UP.
            assert!(effects(&w, EFFECT_SS_HAHEN).iter().any(|e| (e.pos - (s.actor.world_pos + Vec3::Y * 50.0)).length() < 20.0));
            assert!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_EN_NUTS_UP).contains(&f));
            assert!(s.skel.is("gHintNutsUnburrowAnim"));
        }
        if seen.walk == Some(f) {
            // EnDntNomal_SetupTargetWalk: running at 1, mass 0.
            assert_eq!((s.actor.speed_xz, s.actor.col_chk_info.mass), (1.0, 0));
        }
        if seen.face == Some(f) {
            // Past z -30: stopped, by the target. It heads for (1343, 0), turning slowly (scale
            // 0x32) from facing +z: it ends between there and its flower's x.
            assert!(s.actor.world_pos.z > -30.0 && s.actor.speed_xz == 0.0, "{}", s.actor.world_pos);
            assert!((1343.0..flower.x).contains(&s.actor.world_pos.x), "{}", s.actor.world_pos);
        }
        if seen.talk == Some(f) {
            // EnDntNomal_SetupTargetTalk: text 0x10AF, once turned to Link (within 30, before
            // this frame's turn); Player still held.
            assert_eq!((s.actor.text_id, w.msg_ctx.text_id), (0x10AF, 0x10AF));
            assert_eq!(w.player().cs_mode, 1);
        }
        if seen.talk_done == Some(f) {
            // The text's event and A: closed, the camera told it's finished (csId 0), Player
            // PLAYER_CSACTION_8.
            assert_eq!(w.player().cs_mode, 8);
        }
        if seen.bag.is_none()
            && let Some(c) = s.actor.child
            && let Some(bag) = w.actors.downcast::<EnExItem>(c)
        {
            // EnDntNomal_TargetGivePrize from frame 8: En_Ex_Item EXITEM_BULLET_BAG as its child,
            // at its mouth (the last draw's limb 5) 10 to -x; NA_SE_EN_NUTS_THROW.
            seen.bag = Some((f, c));
            bag_spawn_mouth = mouth_before;
            bag_home = bag.actor.home_pos;
            assert_eq!((bag.actor.params, bag.actor.parent), (EXITEM_BULLET_BAG, Some(h)));
            assert!(s.spawned_item);
            assert!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_EN_NUTS_THROW).contains(&f));
        }
        if let Some((_, b)) = seen.bag {
            match w.actors.downcast::<EnExItem>(b) {
                Some(bag) => {
                    if seen.hidden.is_none() && !bag.drawn && bag.action_func != oot_actors::en_ex_item::Action::WaitForObject {
                        seen.hidden = Some(f);
                        // EnExItem_TargetPrizeApproach's end: hidden, Player PLAYER_CSACTION_7, its
                        // parent let go, offered.
                        assert_eq!((w.player().cs_mode, bag.actor.parent, bag.action_func), (7, None, oot_actors::en_ex_item::Action::TargetPrizeGive));
                    }
                    if seen.offered.is_none() && w.player().get_item_id == GI_BULLET_BAG_40 {
                        seen.offered = Some(f);
                    }
                    if seen.taken.is_none() && seen.offered.is_some() && bag.actor.parent.is_some() {
                        seen.taken = Some(f);
                    }
                }
                None => {
                    if seen.bag_gone.is_none() {
                        seen.bag_gone = Some(f);
                    }
                }
            }
        }
        if seen.flag.is_none() && w.save.get_item_get_inf(ITEMGETINF_1D) {
            seen.flag = Some(f);
        }
        if seen.back_home.is_some() && seen.bag_gone.is_some() && seen.flag.is_some() && w.message_state() == TEXT_STATE_NONE {
            break;
        }
    }
    // 50 frames after the hit, up.
    assert_eq!(seen.unburrow, Some(hit_frame + 50));
    let (Some(walk), Some(face), Some(talk), Some(talk_done), Some((bag_frame, _)), Some(home)) = (seen.walk, seen.face, seen.talk, seen.talk_done, seen.bag, seen.back_home) else {
        panic!("{seen:?}")
    };
    assert!(seen.unburrow.unwrap() < walk && walk < face && face < talk && talk < talk_done && talk_done < bag_frame && bag_frame < home, "{seen:?}");
    assert_eq!(bag_home, Vec3::new(bag_spawn_mouth.x - 10.0, bag_spawn_mouth.y, bag_spawn_mouth.z));
    // The bag: offered once its 80 frames are out (set by EnExItem_WaitForObject on its first
    // update, the frame after its spawn, then counted down from the next), GI_BULLET_BAG_40 with
    // the 30 bag; taken; its text done: ITEMGETINF_1D, gone. Item_Give(ITEM_BULLET_BAG_40): the
    // 40 bag, full.
    let (Some(hidden), Some(offered), Some(taken), Some(flag), Some(gone)) = (seen.hidden, seen.offered, seen.taken, seen.flag, seen.bag_gone) else { panic!("{seen:?}") };
    assert_eq!(hidden, bag_frame + 81, "{seen:?}");
    // That first offer (Actor_OfferGetItem) finds Player still in PLAYER_STATE1_29 (his cutscene
    // mode, until his update handles PLAYER_CSACTION_7): refused; EnExItem_TargetPrizeGive's
    // offer the next frame is taken at once (a positive get-item id: Player's parent link the
    // frame after).
    assert_eq!((offered, taken), (hidden + 1, hidden + 2), "{seen:?}");
    assert!(taken < flag && flag < gone, "{seen:?}");
    assert_eq!((w.save.cur_upg_value(UPG_BULLET_BAG), w.save.ammo(ITEM_SLINGSHOT)), (2, 40));
    // The scrub back in its flower: EnDntNomal_TargetReturn put it at z -173, burrowed, waiting
    // (this ROM isn't N64 PAL: no EnDntNomal_DoNothing).
    let s = get(&w, h);
    assert_eq!((s.action_func, s.actor.world_pos.z), (Action::TargetWait, -173.0));
    assert!((s.actor.world_pos.x - flower.x).abs() < 4.0, "{}", s.actor.world_pos);
    // A bullseye now scores but counts no more.
    idle(&mut w, 30);
    let n = get(&w, h).hit_counter;
    shoot(&mut w, TARGET_POS);
    assert_eq!(scores(&w).len(), 1);
    assert_eq!(get(&w, h).hit_counter, n);
}

/// The first-person aim at `target` (as the playthrough's `Task::Shoot`): C-Right held, the stick
/// towards it; and whether the seed in hand points within 64 of it.
fn aim_slingshot(w: &PlayState, target: Vec3) -> (PadState, bool) {
    let p = w.player();
    let held = press(BTN_CRIGHT);
    let Some(seed) = p.held_actor.and_then(|h| w.actors.actor(h)) else { return (held, false) };
    if p.unk_836 <= 0 || p.unk_6AD != 2 {
        return (held, false);
    }
    let d = target - seed.world_pos;
    let want_yaw = vec3f_yaw(seed.world_pos, target);
    let want_pitch = atan2_s((d.x * d.x + d.z * d.z).sqrt(), -d.y);
    let (ey, ep) = (want_yaw.wrapping_sub(seed.world_rot.y) as i32, want_pitch.wrapping_sub(seed.world_rot.x) as i32);
    let stick = |err: i32| -> i8 {
        let s = (1..=60).rev().find(|&s: &i32| ((1.0 - eng_math::cos_s((s * 200) as i16)) * 1500.0) as i32 <= err.abs() * 3 / 4).unwrap_or(0);
        if s == 0 { 0 } else { ((s + 7) * err.signum()) as i8 }
    };
    (PadState { button: BTN_CRIGHT, stick_x: -stick(ey), stick_y: stick(ep) }, ey.abs() < 64 && ep.abs() < 64)
}

#[test]
fn links_own_slingshot_hits_the_bullseye_from_below() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    let h = scrub(&w);
    // From the stumps' floor the seed climbs at about 40 degrees: its quad's edge runs 15 ahead of
    // it (across the ground) at its own height, so it crosses the target about 12 lower than the
    // seed would: aimed 12 above the centre, a bullseye.
    let aim = TARGET_POS + Vec3::new(0.0, 12.0, 0.0);
    let mut prev = PadState::default();
    let mut aimed = 0;
    for _ in 0..240 {
        let (pad, on) = aim_slingshot(&w, aim);
        aimed = if on { aimed + 1 } else { 0 };
        let pad = if aimed > 3 { PadState::default() } else { pad };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        if aimed > 3 {
            break;
        }
    }
    assert!(aimed > 3, "never aimed");
    for _ in 0..20 {
        idle(&mut w, 1);
        if !scores(&w).is_empty() {
            break;
        }
    }
    let s = scores(&w);
    assert_eq!(s.len(), 1, "{s:?}");
    assert_eq!(s[0].0, oot_game::effect::extra::EXTRA_SCORE_100);
    assert_eq!(get(&w, h).hit_counter, 1);
    assert_eq!(w.save.ammo(ITEM_SLINGSHOT), 29);
}

/// A stage scrub (`ENDNTNOMAL_STAGE`, type `ty`) spawned in the woods' room 1, which loads
/// `object_dnk`, on the stumps' floor 150 east of Link, settled in `EnDntNomal_StageWait`.
fn stage_scrub(w: &mut PlayState, ty: i16) -> ActorHandle {
    let pos = Vec3::new(1250.0, -180.0, -30.0);
    let h = w.actor_spawn(ACTOR_EN_DNT_NOMAL, pos, [0, 0, 0], ty).expect("spawned");
    idle(w, 10);
    h
}

#[test]
fn a_stage_scrub_is_driven_by_its_signals() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    // EnDntNomal_Init: type 3 (En_Dnt_Demo's third), the body cylinder, object_dnk; its object in:
    // drawn as a stage scrub, its flower set, EnDntNomal_SetupStageWait (timer3 0):
    // gDntStageHideAnim held at frame 8, not solid (no OC).
    let h = stage_scrub(&mut w, 3);
    let s = get(&w, h);
    assert_eq!((s.ty, s.object_id, s.draw, s.action_func), (3, OBJECT_DNK, Some(Draw::Stage), Action::StageWait));
    assert_eq!((s.body_collider_cylinder.dim.radius, s.body_collider_cylinder.dim.height), (16, 46));
    assert_eq!(s.body_collider_cylinder.base.oc_flags1, OC1_ON | oot_game::collision_check::OC1_TYPE_ALL);
    assert!(s.skel.is("gDntStageHideAnim") && s.skel.cur_frame == 8.0 && !s.is_solid);
    assert!(!w.col_chk.col_oc.contains(&ColliderRef { actor: h, id: 0 }));
    // Its draw: the eye texture by eyeState, the skeleton and the flower in sLeafColors[3 - 1].
    let rs = get(&w, h).render_state();
    let mut out = oot_game::play::DrawOut::default();
    get(&w, h).draw(&rs, &w, &oot_game::play::ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
    assert_eq!(out.opa.len(), 2);
    assert_eq!(out.opa[0].mesh.name, format!("bake/{}", stage_bake(get(&w, h).eye_state as usize)));
    assert_eq!(out.opa[1].mesh.name, format!("bake/{STAGE_FLOWER_BAKE}"));
    assert_eq!(out.opa[0].params.segments.as_ref().unwrap().env[0x0B], Some([210, 255, 0, 255]));
    // DNT_SIGNAL_LOOK with no prize: EnDntNomal_SetupStageUp (timer3 0): up at once (a burst,
    // NA_SE_EN_NUTS_UP), solid, rotDirection -1; the signal cleared.
    get_mut(&mut w, h).stage_signal = DNT_SIGNAL_LOOK;
    idle(&mut w, 1);
    let f = w.audio.frames;
    let s = get(&w, h);
    assert_eq!((s.action_func, s.stage_signal, s.is_solid, s.rot_direction), (Action::StageUp, DNT_SIGNAL_NONE, true, -1));
    assert!(s.skel.is("gDntStageUpAnim"));
    assert!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_EN_NUTS_UP).contains(&f));
    // Solid: its cylinder pushes (CollisionCheck_SetOC).
    assert!(w.col_chk.col_oc.contains(&ColliderRef { actor: h, id: 0 }));
    // Link within 70: EnDntNomal_SetupStageHide, then (timer3 0) down: NA_SE_EN_NUTS_DOWN, not
    // solid; with DNT_ACTION_LOW_RUPEES, at the hide's end a rupee (En_Ex_Ruppy 3, 20 up: colorIdx
    // 0, or 1 one time in four), NA_SE_SY_TRE_BOX_APPEAR, and back to waiting with no action.
    get_mut(&mut w, h).action = DNT_ACTION_LOW_RUPEES;
    let pos = get(&w, h).actor.world_pos;
    w.place_player(pos + Vec3::new(-50.0, 0.0, 0.0), 0x4000);
    idle(&mut w, 2);
    assert_eq!(get(&w, h).action_func, Action::StageHide);
    assert!(!get(&w, h).is_solid);
    let before: Vec<ActorHandle> = w.actors.all();
    let chimes = sfx_frames(&w, oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR).len();
    let mut n = 0;
    while get(&w, h).action_func == Action::StageHide {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 60);
    }
    let s = get(&w, h);
    assert_eq!((s.action_func, s.action), (Action::SetupStageWait, DNT_ACTION_NONE));
    let rupee = w.actors.all().into_iter().find(|x| !before.contains(x) && w.actors.downcast::<EnExRuppy>(*x).is_some()).expect("a rupee");
    let r = w.actors.downcast::<EnExRuppy>(rupee).unwrap();
    assert_eq!((r.actor.params, r.actor.home_pos), (3, s.actor.world_pos + Vec3::Y * 20.0));
    assert!(r.color_idx == 0 || r.color_idx == 1, "{}", r.color_idx);
    assert_eq!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR).len(), chimes + 1);
    // DNT_SIGNAL_CELEBRATE: no action, mass 0, timer3 Rand_ZeroFloat(3) plus half its type (1),
    // EnDntNomal_SetupStageUnburrow when that's out, then celebrating.
    w.place_player(LINK_SPOT.0, LINK_SPOT.1);
    idle(&mut w, 10);
    get_mut(&mut w, h).stage_signal = DNT_SIGNAL_CELEBRATE;
    idle(&mut w, 1);
    let s = get(&w, h);
    assert!(s.action_func == Action::SetupStageUnburrow || s.action_func == Action::StageUnburrow, "{:?}", s.action_func);
    assert_eq!((s.action, s.actor.col_chk_info.mass), (DNT_ACTION_NONE, 0));
    let mut n = 0;
    while get(&w, h).action_func != Action::StageCelebrate {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 80);
    }
    // EnDntNomal_SetupStageCelebrate: walking at 3, solid.
    let s = get(&w, h);
    assert_eq!((s.actor.speed_xz, s.is_solid), (3.0, true));
    assert!(s.skel.is("gDntStageWalkAnim"));
    // DNT_SIGNAL_RETURN: EnDntNomal_StageSetupReturn, walking home at 4 to its flower; there,
    // hiding, then waiting.
    get_mut(&mut w, h).stage_signal = DNT_SIGNAL_RETURN;
    idle(&mut w, 2);
    assert_eq!((get(&w, h).action_func, get(&w, h).actor.speed_xz), (Action::StageReturn, 4.0));
    let mut n = 0;
    while get(&w, h).action_func != Action::StageWait {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 300);
    }
    let s = get(&w, h);
    assert_eq!((s.actor.world_pos.x, s.actor.world_pos.z), (s.flower_pos.x, s.flower_pos.z));
}

#[test]
fn a_stage_scrub_attacks_as_an_enemy() {
    let Some(a) = assets() else { return };
    let mut w = woods(&a, |_| {});
    let h = stage_scrub(&mut w, 2);
    // DNT_ACTION_ATTACK with DNT_SIGNAL_LOOK: up; the animation done, EnDntNomal_SetupStageAttack:
    // the spit, immovable, an enemy (Actor_ChangeCategory); from frame 8 a nut (En_Nutsball 4,
    // object_dnk's).
    get_mut(&mut w, h).action = DNT_ACTION_ATTACK;
    get_mut(&mut w, h).stage_signal = DNT_SIGNAL_LOOK;
    let mut n = 0;
    while get(&w, h).action_func != Action::StageAttack {
        idle(&mut w, 1);
        n += 1;
        assert!(n < 60);
    }
    let s = get(&w, h);
    assert_eq!((s.actor.category, s.actor.col_chk_info.mass), (ACTORCAT_ENEMY, MASS_IMMOVABLE));
    assert!(w.actors.category(ACTORCAT_ENEMY).contains(&h) && !w.actors.category(ACTORCAT_PROP).contains(&h));
    let before: Vec<ActorHandle> = w.actors.all();
    let mut nut = None;
    for _ in 0..30 {
        idle(&mut w, 1);
        nut = w.actors.all().into_iter().find(|x| !before.contains(x) && w.actors.actor(*x).is_some_and(|a| a.id == oot_actors::en_nutsball::ACTOR_EN_NUTSBALL));
        if nut.is_some() {
            break;
        }
    }
    let f = w.audio.frames;
    let nut = w.actors.actor(nut.expect("a nut")).unwrap();
    assert_eq!(nut.params, oot_actors::en_nutsball::EN_NUTSBALL_TYPE_DNK);
    assert!(get(&w, h).spawned_item);
    assert!(sfx_frames(&w, oot_game::audio::sfx::NA_SE_EN_NUTS_THROW).contains(&f));
}

#[test]
fn without_object_dnk_a_stage_scrub_is_gone_at_init() {
    let Some(a) = assets() else { return };
    // Kokiri Forest loads no object_dnk: Object_GetSlot is negative, Actor_Kill.
    let e = a.scenes.entrance_index("ENTR_KOKIRI_FOREST_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    after_sarias_greeting(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap();
    idle(&mut w, 3);
    assert!(w.object_ctx.get_index(OBJECT_DNK).is_none());
    let pos = w.player().actor.world_pos;
    let h = w.actor_spawn(ACTOR_EN_DNT_NOMAL, pos, [0, 0, 0], 1).expect("spawned");
    let s = get(&w, h);
    assert!(s.actor.killed && s.required_object_slot.is_none());
    idle(&mut w, 1);
    assert!(!w.actors.exists(h));
}
