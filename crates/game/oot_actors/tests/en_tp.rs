//! GAME-10 milestone 8 against the C: the Electric Tailpasaran (`z_en_tp.c`). Master Quest's
//! Jabu-Jabu (`bdan`) room 12 places three heads (params 0xFFFF, `ENTP_TYPE_HEAD`) at (-551,
//! -340, -5752), (-766, -340, -5754) and (-662, -340, -5555); each spawns its seven segments.
//!
//! Child Link enters at `ENTR_JABU_JABU_0` on a `jabu-fish` save and the room changes to room 12
//! (`Room_RequestNewRoom`, as the game's `--room`). The tests run one part's update alone
//! (`Actor_UpdateAll`'s call), Link's distance and the hits set as `Actor_UpdateAll` and the
//! collision check would, so no fight is scripted and nothing waits on `Rand`. Expected values
//! are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_actors::en_tp::*;
use oot_game::actor::*;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, SfxPos};
use oot_game::collision_check::*;
use oot_game::effect::{EFFECT_SS_DEAD_DB, EFFECT_SS_KIRAKIRA};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::shadow::ShadowDraw;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Room 12's three heads (`ootx scene-info --scene bdan --layer 0`).
const HOMES: [Vec3; 3] = [Vec3::new(-551.0, -340.0, -5752.0), Vec3::new(-766.0, -340.0, -5754.0), Vec3::new(-662.0, -340.0, -5555.0)];

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Child Link in Jabu-Jabu (`jabu-fish`), the sound log on, in room 12.
fn room_12(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_JABU_JABU_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("jabu-fish").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 2);
    assert!(w.room_request(12));
    idle(&mut w, 1);
    w.room_change_done();
    assert_eq!(w.room_ctx.cur.num, 12);
    // Room 12's objects come in over a few frames (its actors wait as `Uninit` till then).
    for _ in 0..30 {
        if w.object_ctx.get_index(OBJECT_TP).is_some_and(|i| w.object_ctx.is_loaded(i)) && heads(&w).len() == 3 {
            break;
        }
        idle(&mut w, 1);
    }
    w
}

fn tps(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnTp>(h).is_some_and(|t| !t.actor.killed)).collect()
}

fn heads(w: &PlayState) -> Vec<ActorHandle> {
    tps(w).into_iter().filter(|&h| tp(w, h).actor.params == ENTP_TYPE_HEAD).collect()
}

fn tp(w: &PlayState, h: ActorHandle) -> &EnTp {
    w.actors.downcast::<EnTp>(h).expect("En_Tp")
}

fn tp_mut(w: &mut PlayState, h: ActorHandle) -> &mut EnTp {
    w.actors.downcast_mut::<EnTp>(h).expect("En_Tp")
}

fn gone(w: &PlayState, h: ActorHandle) -> bool {
    w.actors.downcast::<EnTp>(h).is_none_or(|t| t.actor.killed)
}

/// The head at `home`.
fn head_at(w: &PlayState, home: Vec3) -> ActorHandle {
    *heads(w).iter().find(|&&h| tp(w, h).actor.home_pos.x == home.x && tp(w, h).actor.home_pos.z == home.z).expect("the head")
}

/// The head's segments, first to last (its `child` chain).
fn segments(w: &PlayState, head: ActorHandle) -> Vec<ActorHandle> {
    let mut v = Vec::new();
    let mut c = tp(w, head).actor.child;
    while let Some(h) = c {
        v.push(h);
        c = w.actors.actor(h).and_then(|a| a.child);
    }
    v
}

/// The actor's update alone (`Actor_UpdateAll`'s call: `prevPos` first), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    let b = a.base_mut();
    b.prev_pos = b.world_pos;
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// Its draw's write (`EnTp_Draw`'s spheres).
fn draw_update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    a.draw_update(w);
    w.actors.put_back(h, a);
}

/// `Actor_UpdateAll`'s distance and yaw: Link `dist` across at `yaw`.
fn set_link(w: &mut PlayState, h: ActorHandle, dist: f32, yaw: i16) {
    let t = tp_mut(w, h);
    t.actor.xz_dist_to_player = dist;
    t.actor.yaw_towards_player = yaw;
}

/// The live soft sprites of a type.
fn effects(w: &PlayState, ty: u8) -> usize {
    w.effect_ss.table.iter().filter(|e| e.ty == ty && e.life >= 0).count()
}

/// The times a sound was asked for at `h`.
fn sfx(w: &PlayState, h: ActorHandle, id: u16) -> usize {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, p)| s == id && p == SfxPos::Actor(h)).count()
}

/// A hit this frame: `damage` and `reaction` (`CollisionCheck_ApplyDamage`'s), from `flags`.
fn hit(w: &mut PlayState, h: ActorHandle, damage: u8, reaction: u8, flags: u32) {
    let link = w.player.expect("Link");
    let t = tp_mut(w, h);
    t.collider.base.ac_flags |= AC_HIT;
    t.collider.base.ac = Some(link);
    t.collider.elements[0].info.ac_hit_elem = Some(HitElem {
        elem: ElemRef { col: ColliderRef { actor: link, id: 0 }, elem: 0 },
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: flags, hit_special_effect: 0, damage },
        ac_dmg_info: Default::default(),
        elem_material: 0,
    });
    t.actor.col_chk_info.damage = damage;
    t.actor.col_chk_info.damage_reaction = reaction;
}

#[test]
fn room_12s_three_heads_each_with_seven_segments_as_init_sets_them() {
    let Some(a) = assets() else { return };
    let w = room_12(&a);
    assert_eq!(heads(&w).len(), 3);
    assert_eq!(tps(&w).len(), 24);
    for home in HOMES {
        let h = head_at(&w, home);
        let t = tp(&w, h);
        // EnTp_Init's head: NAVI_ENEMY_TAILPASARAN, hard, its sphere 8, lockable, hostile, always
        // updating, 1.5 big; lockOnArrowOffset 10, ATTENTION_RANGE_3, health 1, the circle
        // shadow 0.14.
        assert_eq!((t.actor.navi_enemy_id, t.actor.scale, t.actor.target_arrow_offset, t.actor.target_mode), (NAVI_ENEMY_TAILPASARAN, Vec3::splat(1.5), 10.0, ATTENTION_RANGE_3));
        let f = ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        assert_eq!(t.actor.flags & f, f);
        assert_eq!((t.actor.col_chk_info.health, t.body_elem_alpha), (1, 255));
        assert_eq!((t.actor.shape_shadow_draw, t.actor.shape_shadow_scale), (ShadowDraw::Circle, 0.14));
        assert_ne!(t.collider.base.ac_flags & AC_HARD, 0);
        let e = &t.collider.elements[0];
        assert_eq!((e.dim.world_sphere.radius, e.dim.model_sphere.radius, e.dim.scale), (8, 8, 1.0));
        assert_eq!((e.info.at_dmg_info.damage, e.info.at_dmg_info.hit_special_effect, e.info.ac_dmg_info.hit_backlash), (8, HIT_SPECIAL_EFFECT_ELECTRIC, HIT_BACKLASH_ELECTRIC));
        // EnTp_Head_SetupWaitPlayerAndRise: pointing down, 60, still.
        assert_eq!((t.action, t.func, t.mode_var, t.actor.shape_rot.x, t.timer, t.timer2), (ENTP_ACTION_HEAD_WAIT_PLAYER_AND_RISE, Func::HeadWaitPlayerAndRise, 0, -0x4000, 60, 0));
        assert_eq!(t.actor.parent, None);
        // Seven segments, each the last one's child and the next one's parent.
        let segs = segments(&w, h);
        assert_eq!(segs.len(), 7);
        for (i, &s) in segs.iter().enumerate() {
            let c = tp(&w, s);
            let i = i as i32;
            assert_eq!((c.actor.params, c.action, c.func), (ENTP_TYPE_BODYELEM, ENTP_ACTION_BODYELEM, Func::BodyElem));
            assert_eq!(c.actor.parent, Some(if i == 0 { h } else { segs[i as usize - 1] }));
            assert_eq!((c.spark_timer, c.timer, c.timer2), ((i + 1) as i16, (i * -5) as i16, (i * -5) as i16));
            assert_eq!((c.actor.scale, c.body_elem_dist, c.actor.world_pos), (Vec3::splat(0.3), 6.0 - (i as f32 * 0.75), home));
            assert_eq!(c.collider.elements[0].dim.model_sphere.radius, 4);
            // The third lockable and hostile (modeVar 1).
            assert_eq!((c.mode_var, c.actor.flags & f == f), ((i == 2) as i32, i == 2));
        }
    }
}

#[test]
fn waiting_it_rises_toward_link_within_200_then_hovers_and_far_it_sinks_back() {
    let Some(a) = assets() else { return };
    let mut w = room_12(&a);
    let h = head_at(&w, HOMES[0]);
    let home = HOMES[0];
    set_link(&mut w, h, 100.0, 0x2000);
    update(&mut w, h);
    // Within 200: 59, its head up 0x1F4, turning to Link 0x5DC, up toward 30 over its place
    // (1 a frame at most), at its place across (bodyElemDist 0).
    let t = tp(&w, h);
    assert_eq!((t.timer, t.timer2, t.actor.shape_rot.x, t.actor.world_rot.y), (59, -1, -0x4000 + 0x1F4, 0x5DC));
    assert_eq!((t.actor.world_pos.x, t.actor.world_pos.y, t.actor.world_pos.z), (home.x, home.y + 1.0, home.z));
    assert_eq!(t.actor.shape_rot.y, t.actor.world_rot.y);
    assert_eq!(sfx(&w, h, NA_SE_EN_TAIL_FLY - oot_game::audio::sfx::SFX_FLAG), 1);
    // Its spin: 0x800 a frame (NA_SE_EN_TAIL_CRY each turn).
    assert_eq!(t.actor.shape_rot.z, 0x800);
    // Link far: its head down again, back to its place, wobbling 6 across.
    set_link(&mut w, h, 300.0, 0x2000);
    update(&mut w, h);
    let t = tp(&w, h);
    assert_eq!((t.timer, t.actor.shape_rot.x), (59, -0x4000));
    let (ppos, a2) = (w.actors.actor(w.player.unwrap()).unwrap().world_pos, (-2i32 * 0x7D0) as i16);
    let sp32 = eng_math::vec3f_yaw(home, ppos);
    assert_eq!(t.actor.world_pos.x, (sin_s(a2) * (sin_s(sp32) * 6.0)) + home.x);
    for _ in 0..10 {
        update(&mut w, h);
    }
    // Back at its place: 60 again.
    let t = tp(&w, h);
    assert_eq!((t.actor.world_pos.y, t.timer), (home.y, 60));
    // Touching Link within 200: at once (its AT's hit on him), EnTp_Head_SetupHover: flying
    // (modeVar 1), its head level, 40 to 54 frames.
    set_link(&mut w, h, 100.0, 0x2000);
    let link = w.player;
    let t = tp_mut(&mut w, h);
    t.collider.base.at_flags |= AT_HIT;
    t.collider.base.at = link;
    update(&mut w, h);
    let t = tp(&w, h);
    assert_eq!((t.action, t.func, t.mode_var, t.actor.shape_rot.x), (ENTP_ACTION_HEAD_8, Func::HeadHover, 1, 0));
    assert!((40..55).contains(&t.timer), "{}", t.timer);
}

#[test]
fn the_60_frames_out_it_hovers_then_attacks_then_dives() {
    let Some(a) = assets() else { return };
    let mut w = room_12(&a);
    let h = head_at(&w, HOMES[1]);
    set_link(&mut w, h, 100.0, 0);
    for _ in 0..60 {
        update(&mut w, h);
    }
    assert_eq!(tp(&w, h).func, Func::HeadWaitPlayerAndRise);
    update(&mut w, h);
    assert_eq!((tp(&w, h).func, tp(&w, h).mode_var), (Func::HeadHover, 1));
    // EnTp_Head_Hover: speeding to 2.5 (Math_SmoothStepToF a tenth, 0.2 at most), its red fading.
    let t = tp_mut(&mut w, h);
    t.red_shine_amount = 30;
    t.timer = 2;
    update(&mut w, h);
    let t = tp(&w, h);
    assert_eq!((t.timer, t.red_shine_amount), (1, 15));
    assert!((t.actor.speed_xz - 0.2).abs() < 1e-6, "{}", t.actor.speed_xz);
    assert_eq!(t.actor.shape_rot.y, t.actor.world_rot.y);
    // The count out: EnTp_Head_SetupAttack (200, 199 after its frame), its red up 15 a frame.
    update(&mut w, h);
    assert_eq!((tp(&w, h).func, tp(&w, h).action, tp(&w, h).timer), (Func::HeadAttack, ENTP_ACTION_HEAD_ATTACK, 200));
    update(&mut w, h);
    assert_eq!((tp(&w, h).timer, tp(&w, h).red_shine_amount), (199, 15));
    // Touching Link: the count to 1, so out at once: EnTp_Head_SetupSinkIntoGroundAndReappear.
    let link = w.player;
    let t = tp_mut(&mut w, h);
    t.collider.base.at_flags |= AT_HIT;
    t.collider.base.at = link;
    update(&mut w, h);
    let t = tp(&w, h);
    assert_eq!((t.func, t.action, t.timer), (Func::HeadSinkIntoGroundAndReappear, ENTP_ACTION_HEAD_SINK_INTO_GROUND_AND_REAPPEAR, 0));
}

#[test]
fn diving_it_plunges_then_comes_back_up_at_its_place_with_its_segments() {
    let Some(a) = assets() else { return };
    let mut w = room_12(&a);
    let h = head_at(&w, HOMES[2]);
    let home = HOMES[2];
    let segs = segments(&w, h);
    let t = tp_mut(&mut w, h);
    t.head_setup_sink_into_ground_and_reappear();
    t.actor.shape_rot.x = 0;
    t.red_shine_amount = 30;
    t.actor.world_pos.y = home.y + 50.0;
    update(&mut w, h);
    // 1. Pitching 0x400 a frame, its red down 15, 2 × cos ahead, -2 × sin up.
    let t = tp(&w, h);
    assert_eq!((t.actor.shape_rot.x, t.red_shine_amount, t.timer2), (-0x400, 15, -1));
    assert_eq!(t.actor.speed_xz, 2.0 * cos_s(-0x400));
    assert_eq!(t.actor.velocity.y, sin_s(-0x400) * -2.0);
    // 2. More than 60 under its place: the head and the segments put back (they 80 under), 80
    // (the first time through, the count -1 then 80: the C's own comment).
    tp_mut(&mut w, h).actor.world_pos.y = home.y - 61.0;
    tp_mut(&mut w, segs[3]).timer = 7;
    update(&mut w, h);
    let t = tp(&w, h);
    assert_eq!((t.timer, t.actor.shape_rot.x, t.actor.speed_xz, t.actor.velocity.y), (80, -0x4000, 0.0, 0.0));
    // 3. Coming up 1 a frame (its place - the count; Actor_MoveXZGravity still).
    assert_eq!((t.actor.world_pos.x, t.actor.world_pos.y, t.actor.world_pos.z), (home.x, home.y - 80.0, home.z));
    for &s in &segs {
        assert_eq!(tp(&w, s).actor.world_pos, home - Vec3::Y * 80.0);
    }
    update(&mut w, h);
    assert_eq!((tp(&w, h).timer, tp(&w, h).actor.world_pos.y), (79, home.y - 79.0));
    for _ in 0..78 {
        update(&mut w, h);
    }
    assert_eq!(tp(&w, h).timer, 1);
    // 4. Up: waiting again, each segment's wobble count from its own.
    update(&mut w, h);
    let t = tp(&w, h);
    assert_eq!((t.func, t.mode_var, t.timer, t.actor.shape_rot.x), (Func::HeadWaitPlayerAndRise, 0, 60, -0x4000));
    assert_eq!(tp(&w, segs[3]).timer2, 7);
}

#[test]
fn its_segments_sway_under_it_waiting_and_trail_it_flying() {
    let Some(a) = assets() else { return };
    let mut w = room_12(&a);
    let h = head_at(&w, HOMES[0]);
    let home = HOMES[0];
    let segs = segments(&w, h);
    // The head waiting (modeVar 0): 4 under the parent (1 a frame at most), swaying across the
    // head's facing with the head's count and its own, bodyElemDist wide.
    let t = tp_mut(&mut w, h);
    t.actor.world_pos.y = home.y + 20.0;
    t.timer2 = 3;
    t.actor.shape_rot.y = 0x1000;
    let s1 = segs[1];
    update(&mut w, s1);
    let c = tp(&w, s1);
    assert_eq!(c.head_instance, Some(h));
    let parent_y = tp(&w, segs[0]).actor.world_pos.y;
    assert_eq!(c.actor.world_pos.y, (home.y + 1.0).min(parent_y - 4.0).max(home.y - 1.0));
    let a2 = ((3 + -5) * 0x7D0) as i16;
    let sp36 = 0x5000i16;
    assert_eq!(c.actor.world_pos.x, (sin_s(a2) * (sin_s(sp36) * 5.25)) + home.x);
    assert_eq!(c.actor.world_pos.z, (sin_s(a2) * (cos_s(sp36) * 5.25)) + home.z);
    // The head flying (modeVar 1): where its parent was last frame, its red and speed 0; under
    // the head's place, not lockable (the third).
    tp_mut(&mut w, h).mode_var = 1;
    let p = Vec3::new(10.0, home.y - 30.0, 20.0);
    w.actors.actor_mut(segs[1]).unwrap().prev_pos = p;
    let t = tp_mut(&mut w, segs[2]);
    t.red_shine_amount = 60;
    t.actor.world_pos.y = home.y - 30.0;
    update(&mut w, segs[2]);
    let c = tp(&w, segs[2]);
    assert_eq!((c.actor.world_pos, c.red_shine_amount, c.actor.speed_xz), (p, 0, 0.0));
    assert_eq!(c.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    // Above the head's place, the third is lockable again (modeVar 1).
    tp_mut(&mut w, segs[2]).actor.world_pos.y = home.y + 10.0;
    w.actors.actor_mut(segs[1]).unwrap().prev_pos = home + Vec3::Y * 10.0;
    update(&mut w, segs[2]);
    assert_ne!(tp(&w, segs[2]).actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
}

#[test]
fn a_sword_on_a_segment_kills_the_whole_tailpasaran_segment_by_segment() {
    let Some(a) = assets() else { return };
    let mut w = room_12(&a);
    let h = head_at(&w, HOMES[0]);
    let segs = segments(&w, h);
    let s = segs[2];
    hit(&mut w, s, 1, ENTP_DMG_REACT_MELEE_BACKLASH, DMG_SLASH_KOKIRI);
    tp_mut(&mut w, s).red_shine_amount = 45;
    update(&mut w, s);
    // EnTp_CheckCollide: 1 - 1, no lock-on; its head's EnTp_SetupDying (13, NA_SE_EN_TAIL_DEAD,
    // every segment dying with no health), the reaction, ENTP_TYPE_HEAD_DYING.
    let c = tp(&w, s);
    assert_eq!((c.actor.col_chk_info.health, c.actor.params, c.unk_158, c.action), (0, ENTP_TYPE_BODYELEM_DYING, ENTP_DMG_REACT_MELEE_BACKLASH, ENTP_ACTION_DYING));
    assert_eq!(c.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    let t = tp(&w, h);
    assert_eq!((t.func, t.action, t.timer, t.actor.params, t.unk_158), (Func::Dying, ENTP_ACTION_DYING, 13, ENTP_TYPE_HEAD_DYING, ENTP_DMG_REACT_MELEE_BACKLASH));
    assert_eq!(sfx(&w, h, NA_SE_EN_TAIL_DEAD), 1);
    for &x in &segs {
        assert_eq!((tp(&w, x).actor.params, tp(&w, x).actor.col_chk_info.health), (ENTP_TYPE_BODYELEM_DYING, 0));
    }
    // The head: 12 frames on, two puffs (EffectSsDeadDb) and a drop, its first segment let go,
    // gone (modeVar 2).
    let puffs = effects(&w, EFFECT_SS_DEAD_DB);
    for _ in 0..12 {
        update(&mut w, h);
    }
    assert!(!gone(&w, h));
    update(&mut w, h);
    assert!(gone(&w, h));
    assert_eq!(tp(&w, h).mode_var, 2);
    assert_eq!(effects(&w, EFFECT_SS_DEAD_DB), puffs + 2);
    assert_eq!(w.actors.actor(segs[0]).unwrap().parent, None);
    // Each segment: let go, EnTp_SetupDying (2), then a remnant (half its size, its red) and
    // gone, letting the next go.
    for (i, &x) in segs.iter().enumerate() {
        let before = tps(&w).len();
        update(&mut w, x);
        assert_eq!((tp(&w, x).func, tp(&w, x).timer), (Func::Dying, 2), "segment {i}");
        update(&mut w, x);
        assert!(!gone(&w, x));
        update(&mut w, x);
        assert!(gone(&w, x), "segment {i}");
        assert_eq!(tps(&w).len(), before, "segment {i}: one gone, one remnant");
        if let Some(&n) = segs.get(i + 1) {
            assert_eq!(w.actors.actor(n).unwrap().parent, None);
        }
    }
    let rem: Vec<ActorHandle> = tps(&w).into_iter().filter(|&r| tp(&w, r).actor.params == ENTP_TYPE_BODYELEMREMNANT).collect();
    assert_eq!(rem.len(), 7);
    let red: Vec<i16> = rem.iter().map(|&r| tp(&w, r).red_shine_amount).collect();
    assert!(red.contains(&45));
    // A remnant: 0.15 big, no lock-on, drifting, fading 20 a frame; under 20, gone (12 frames).
    let r = rem[0];
    let t = tp(&w, r);
    assert_eq!((t.actor.scale, t.action, t.func), (Vec3::splat(0.15), ENTP_ACTION_BODYELEMREMNANT, Func::BodyElemRemnant));
    assert_eq!(t.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    let (p0, v) = (t.actor.world_pos, t.actor.velocity);
    update(&mut w, r);
    // Actor_UpdatePos: its velocity at the update rate (R_UPDATE_RATE × 0.5).
    assert_eq!((tp(&w, r).body_elem_alpha, tp(&w, r).actor.world_pos), (235, p0 + v * eng_math::UPDATE_SCALE));
    for _ in 0..10 {
        update(&mut w, r);
    }
    assert_eq!(tp(&w, r).body_elem_alpha, 35);
    update(&mut w, r);
    assert!(gone(&w, r));
    assert_eq!(tp(&w, r).body_elem_alpha, 0);
}

#[test]
fn a_deku_nut_stuns_the_whole_of_it() {
    let Some(a) = assets() else { return };
    let mut w = room_12(&a);
    let h = head_at(&w, HOMES[1]);
    let segs = segments(&w, h);
    // On the head: no damage (its health kept), 80 frames' freeze for it and every segment,
    // blue in the OPA list, NA_SE_EN_GOMA_JR_FREEZE once.
    hit(&mut w, h, 0, ENTP_DMG_REACT_STUN, DMG_DEKU_NUT);
    update(&mut w, h);
    let t = tp(&w, h);
    assert_eq!((t.actor.col_chk_info.health, t.actor.freeze_timer, t.unk_158), (1, 80, ENTP_DMG_REACT_STUN));
    assert_eq!((t.actor.color_filter_params & COLORFILTER_BUFFLAG_XLU, t.actor.color_filter_timer), (0, 80));
    assert_eq!(sfx(&w, h, NA_SE_EN_GOMA_JR_FREEZE), 1);
    for &s in &segs {
        let c = tp(&w, s);
        assert_eq!((c.actor.freeze_timer, c.actor.color_filter_params & COLORFILTER_BUFFLAG_XLU, c.actor.col_chk_info.health), (80, 0, 1));
    }
    // On the fourth segment: XLU blue for all, the freeze's sound at it once and once for each
    // part above it (three segments and the head).
    let s = segs[3];
    hit(&mut w, s, 0, ENTP_DMG_REACT_STUN, DMG_DEKU_NUT);
    // (One of the others hit too this frame: its hit forgotten.)
    tp_mut(&mut w, segs[6]).collider.base.ac_flags |= AC_HIT;
    update(&mut w, s);
    assert_eq!(sfx(&w, s, NA_SE_EN_GOMA_JR_FREEZE), 5);
    assert_eq!(tp(&w, segs[6]).collider.base.ac_flags & AC_HIT, 0);
    for &x in segs.iter().chain([h].iter()) {
        assert_ne!(tp(&w, x).actor.color_filter_params & COLORFILTER_BUFFLAG_XLU, 0);
    }
    assert_eq!(tp(&w, s).actor.col_chk_info.health, 1);
}

#[test]
fn hits_with_no_effect_and_the_heads_hard_shell() {
    let Some(a) = assets() else { return };
    let mut w = room_12(&a);
    let h = head_at(&w, HOMES[2]);
    let segs = segments(&w, h);
    // A slingshot seed on a segment: nothing (its reaction 0).
    hit(&mut w, segs[0], 0, ENTP_DMG_REACT_NO_EFFECT, DMG_SLINGSHOT);
    update(&mut w, segs[0]);
    assert_eq!((tp(&w, segs[0]).actor.col_chk_info.health, tp(&w, segs[0]).unk_158), (1, 0));
    // A sword on the head: no damage (hard), but its melee reaction keeps its AT on.
    hit(&mut w, h, 1, ENTP_DMG_REACT_MELEE_BACKLASH, DMG_SLASH_KOKIRI);
    update(&mut w, h);
    let t = tp(&w, h);
    assert_eq!((t.actor.col_chk_info.health, t.unk_158, t.func), (1, ENTP_DMG_REACT_MELEE_BACKLASH, Func::HeadWaitPlayerAndRise));
    // A hit while dying (action under ENTP_ACTION_BODYELEM): passed over.
    let t = tp_mut(&mut w, segs[1]);
    t.action = ENTP_ACTION_DYING;
    hit(&mut w, segs[1], 1, ENTP_DMG_REACT_MELEE_BACKLASH, DMG_SLASH_KOKIRI);
    tp_mut(&mut w, segs[1]).func = Func::Dying;
    tp_mut(&mut w, segs[1]).timer = 50;
    update(&mut w, segs[1]);
    assert_eq!((tp(&w, segs[1]).actor.col_chk_info.health, tp(&w, segs[1]).collider.base.ac_flags & AC_HIT), (1, AC_HIT));
}

#[test]
fn every_part_sparks_every_eighth_frame() {
    let Some(a) = assets() else { return };
    let mut w = room_12(&a);
    let h = head_at(&w, HOMES[0]);
    let s = segments(&w, h)[0];
    // sparkTimer 1 (the first segment's): 0 after its first frame: a spark; then every 8th.
    let k0 = effects(&w, EFFECT_SS_KIRAKIRA);
    set_link(&mut w, s, 1000.0, 0);
    update(&mut w, s);
    assert_eq!((tp(&w, s).spark_timer, effects(&w, EFFECT_SS_KIRAKIRA)), (0, k0 + 1));
    for _ in 0..7 {
        update(&mut w, s);
    }
    assert_eq!(effects(&w, EFFECT_SS_KIRAKIRA), k0 + 1);
    update(&mut w, s);
    assert_eq!(effects(&w, EFFECT_SS_KIRAKIRA), k0 + 2);
}

#[test]
fn its_draw_and_its_spheres() {
    let Some(a) = assets() else { return };
    for b in bakes() {
        let d: eng_gfx::DrawList = a.pack.assets.get(&oot_game::pack::keys::bake(&b.name)).expect(&b.name);
        assert!(d.triangle_count() > 0 && d.stats.unresolved_addresses.is_empty(), "{}", b.name);
    }
    let mut w = room_12(&a);
    let h = head_at(&w, HOMES[0]);
    let s = segments(&w, h)[0];
    // Collider_UpdateSpheres under the draw's matrix: the head's 8 ahead (× 1.5), radius 8; a
    // segment's at its place, 4.
    let t = tp_mut(&mut w, h);
    t.actor.shape_rot = Rot { x: 0, y: 0, z: 0 };
    t.actor.world_pos = Vec3::new(10.0, 20.0, 30.0);
    draw_update(&mut w, h);
    let e = tp(&w, h).collider.elements[0].dim.world_sphere;
    assert_eq!((e.center, e.radius), ([10, 20, 42], 8));
    tp_mut(&mut w, s).actor.world_pos = Vec3::new(-5.5, 7.9, 3.2);
    draw_update(&mut w, s);
    let e = tp(&w, s).collider.elements[0].dim.world_sphere;
    assert_eq!((e.center, e.radius), ([-5, 7, 3], 4));
    // The head opaque; a segment translucent, its colour (red, 0, 255, alpha), facing the camera.
    let view = oot_game::play::ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
    let mut out = oot_game::play::DrawOut::default();
    let t = tp(&w, h);
    t.draw(&t.render_state(), &w, &view, &mut out);
    assert_eq!((out.opa.len(), out.xlu.len()), (1, 0));
    assert_eq!(out.opa[0].mesh, eng_gfx::MeshKey::named(oot_game::pack::keys::bake(BAKE_HEAD)));
    let t = tp_mut(&mut w, s);
    t.red_shine_amount = 90;
    t.body_elem_alpha = 200;
    let t = tp(&w, s);
    let mut out = oot_game::play::DrawOut::default();
    t.draw(&t.render_state(), &w, &view, &mut out);
    assert_eq!((out.opa.len(), out.xlu.len()), (0, 1));
    assert_eq!(out.xlu[0].mesh, eng_gfx::MeshKey::named(oot_game::pack::keys::bake(BAKE_BODY_ELEM)));
    let sv = out.xlu[0].params.segments.as_ref().unwrap();
    assert_eq!(sv.prim[0x0E], Some([90, 0, 255, 200]));
}
