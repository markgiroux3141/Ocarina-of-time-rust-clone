//! The Deku Tree's Deku Scrubs (GAME-05 milestone 3b) against the C: the Mad Scrub
//! (`En_Dekunuts`, `z_en_dekunuts.c`; room 4's at (-74, -880, 1046), params 0xFF00), the hint
//! scrubs' order puzzle (`En_Hintnuts`, `z_en_hintnuts.c`; room 9's three, params 0x0001, 0x0002,
//! 0x9C03) and the Business Scrub (`En_Shopnuts`, `z_en_shopnuts.c`, room 3's at (-562, -820,
//! 332), params 4) with its salesman (`En_Dns`, `z_en_dns.c`). Each from a debug start in its
//! room (`Room_RequestNewRoom`, then Link placed), as the MQ scene places them.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_B, BTN_R, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_dekunuts::{Action as NutsAction, DEKUNUTS_FLOWER, EnDekunuts};
use oot_actors::en_dns::{Action as DnsAction, EnDns};
use oot_actors::en_hintnuts::{Action as HintAction, EnHintnuts};
use oot_actors::en_item00::EnItem00;
use oot_actors::en_nutsball::EnNutsball;
use oot_actors::en_shopnuts::{Action as ShopAction, EnShopnuts};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_HOSTILE, ACTOR_FLAG_TALK};
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_ENEMY, ACTORCAT_PROP, ActorHandle, ActorImpl};
use oot_game::audio::sfx::*;
use oot_game::collision_check as cc;
use oot_game::effect::{EFFECT_SS_DEAD_DB, EFFECT_SS_HAHEN};
use oot_game::message::{TEXT_STATE_AWAITING_NEXT, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_EVENT};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const NONE: PadState = PadState { button: 0, stick_x: 0, stick_y: 0 };
const A: PadState = PadState { button: BTN_A, stick_x: 0, stick_y: 0 };
const B: PadState = PadState { button: BTN_B, stick_x: 0, stick_y: 0 };
const R: PadState = PadState { button: BTN_R, stick_x: 0, stick_y: 0 };

struct Run {
    w: PlayState,
    prev: PadState,
}

impl Run {
    /// Inside the Deku Tree (`deku-tree-inside`: the Kokiri Sword and the Deku Shield worn), the
    /// sound log on, changed to `room` and Link at `pos` facing `yaw`, the room's actors in.
    fn enter(room: i8, pos: Vec3, yaw: i16, f: impl FnOnce(&mut SaveContext)) -> Option<Run> {
        let a = assets()?;
        let e = a.scenes.entrance_index("ENTR_DEKU_TREE_0").expect("entrance");
        let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
        save.apply_preset("deku-tree-inside").unwrap();
        f(&mut save);
        let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
        let w = PlayState::play_init_with(a.clone(), data()?, rules()?, save, audio).expect("Play_Init");
        let mut r = Run { w, prev: NONE };
        r.frames(NONE, 20);
        // Room_RequestNewRoom, a frame for it to load, Room_FinishRoomChange.
        assert!(r.w.room_request(room));
        r.frames(NONE, 1);
        r.w.room_change_done();
        r.w.place_player(pos, yaw);
        Some(r)
    }

    fn frames(&mut self, pad: PadState, n: usize) {
        for _ in 0..n {
            self.w.tick_with(scripted_input(self.prev, pad));
            self.prev = pad;
        }
    }

    fn press(&mut self, pad: PadState) {
        self.frames(pad, 1);
        self.frames(NONE, 1);
    }

    /// Runs frames with `pad` until `done`, at most `n`; the frames run.
    fn until(&mut self, n: usize, pad: PadState, done: impl Fn(&Run) -> bool) -> usize {
        for i in 0..n {
            if done(self) {
                return i;
            }
            self.frames(pad, 1);
        }
        assert!(done(self), "not done in {n} frames");
        n
    }

    /// Through the message box until `done` (at most `n` frames): A on each box waiting for it
    /// (its text done, the next one or the event), else no input.
    fn advance(&mut self, n: usize, done: impl Fn(&Run) -> bool) {
        for _ in 0..n {
            if done(self) {
                return;
            }
            if matches!(self.w.message_state(), TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_DONE | TEXT_STATE_AWAITING_NEXT | TEXT_STATE_EVENT) {
                self.press(A);
            } else {
                self.frames(NONE, 1);
            }
        }
        assert!(done(self), "not done in {n} frames: text {:#x}, state {}", self.w.msg_ctx.text_id, self.w.message_state());
    }

    fn sfx_on_last_frame(&self, id: u16) -> bool {
        let f = self.w.audio.frames;
        self.w.audio.log.as_ref().unwrap().sfx.iter().any(|&(fr, s, _)| fr == f && s == id)
    }

    fn sfx_count(&self, id: u16) -> usize {
        self.w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).count()
    }

    /// The actors of type `T`, by their home's distance from `pos`.
    fn find<T: ActorImpl>(&self, pos: Vec3, pick: impl Fn(&T) -> bool) -> Option<ActorHandle> {
        self.w.actors.all().into_iter().filter(|&h| self.w.actors.downcast::<T>(h).is_some_and(&pick)).min_by(|&a, &b| {
            let d = |h| self.w.actors.actor(h).map(|x| x.home_pos.distance(pos)).unwrap_or(f32::MAX);
            d(a).total_cmp(&d(b))
        })
    }

    #[track_caller]
    fn get<T: ActorImpl>(&self, h: ActorHandle) -> &T {
        self.w.actors.downcast::<T>(h).expect("the actor")
    }
    #[track_caller]
    fn get_mut<T: ActorImpl>(&mut self, h: ActorHandle) -> &mut T {
        self.w.actors.downcast_mut::<T>(h).expect("the actor")
    }

    /// The live effects of type `ty`.
    fn effects(&self, ty: u8) -> usize {
        self.w.effect_ss.table.iter().filter(|e| e.life >= 0 && e.ty == ty).count()
    }
}

const MAD_SCRUB_HOME: Vec3 = Vec3::new(-74.0, -880.0, 1046.0);
/// 250 in front of it (it faces -z, rot.y 0x8000), facing it.
const MAD_SCRUB_START: (Vec3, i16) = (Vec3::new(-74.0, -880.0, 796.0), 0);

fn mad_scrub(r: &Run) -> ActorHandle {
    r.find::<EnDekunuts>(MAD_SCRUB_HOME, |n| n.actor.params != DEKUNUTS_FLOWER).expect("En_Dekunuts")
}

/// The Mad Scrub up to its first spit, then its next round and its burrow.
#[test]
fn a_mad_scrub_pops_up_spits_its_nut_and_burrows() {
    let Some(mut r) = Run::enter(4, Vec3::new(-74.0, -880.0, 560.0), 0, |_| {}) else { return };
    r.frames(NONE, 3);
    let h = mad_scrub(&r);
    let n = r.get::<EnDekunuts>(h);
    // EnDekunuts_Init: params 0xFF00: shotsPerRound 0xFF -> 1, params 0; sColChkInfoInit
    // { 1, 18, 32, MASS_IMMOVABLE }, sDamageTable; EnDekunuts_SetupWait: the collider 5 high and
    // off, at home, the timer Rand_S16Offset(100, 50).
    assert_eq!((n.shots_per_round, n.actor.params), (1, 0));
    assert_eq!((n.actor.col_chk_info.health, n.actor.col_chk_info.mass), (1, cc::MASS_IMMOVABLE));
    assert_eq!(n.actor.col_chk_info.damage_table.unwrap().table, oot_actors::en_dekunuts::S_DAMAGE_TABLE.table);
    assert_eq!((n.action, n.collider.dim.height, n.collider.base.ac_flags & cc::AC_ON), (NutsAction::Wait, 5, 0));
    assert_eq!(n.skel.play_speed, 0.0);
    assert!((90..150).contains(&n.anim_flag_and_timer), "{}", n.anim_flag_and_timer);
    assert_eq!(n.actor.world_pos, MAD_SCRUB_HOME);
    // Its flower: the child (params DEKUNUTS_FLOWER), untargetable, still an enemy.
    let flower = n.actor.child.expect("its flower");
    let f = r.get::<EnDekunuts>(flower);
    assert_eq!((f.actor.params, f.actor.category, f.actor.parent), (DEKUNUTS_FLOWER, ACTORCAT_ENEMY, Some(h)));
    assert_eq!(f.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE), 0);
    // Link 486 off (over 480): it waits, its timer counting down one a frame.
    let t0 = r.get::<EnDekunuts>(h).anim_flag_and_timer;
    r.frames(NONE, 5);
    assert_eq!(r.get::<EnDekunuts>(h).anim_flag_and_timer, t0 - 5);
    assert_eq!(r.get::<EnDekunuts>(h).skel.play_speed, 0.0);
    // 250 off (between 160 and 480, level): up at speed 1 at once.
    r.w.place_player(MAD_SCRUB_START.0, MAD_SCRUB_START.1);
    r.frames(NONE, 1);
    assert_eq!(r.get::<EnDekunuts>(h).skel.play_speed, 1.0);
    // Up: NA_SE_EN_NUTS_UP on frame 8, AC_ON on frame 9; the collider (clamp(frame, 9, 12) - 9)
    // × 9 + 5 high, before the frame's update.
    let mut heights = Vec::new();
    let mut up_sfx = None;
    let mut ac_on = None;
    for i in 0..30 {
        let before = r.get::<EnDekunuts>(h).skel.cur_frame;
        r.frames(NONE, 1);
        let n = r.get::<EnDekunuts>(h);
        if n.action != NutsAction::Wait {
            break;
        }
        heights.push((before, n.collider.dim.height));
        if r.sfx_on_last_frame(NA_SE_EN_NUTS_UP) {
            up_sfx = Some(before);
        }
        if ac_on.is_none() && n.collider.base.ac_flags & cc::AC_ON != 0 {
            ac_on = Some(before);
        }
        assert!(i < 29);
    }
    assert_eq!((up_sfx, ac_on), (Some(8.0), Some(9.0)));
    for (f, ht) in heights {
        assert_eq!(ht, (((f.clamp(9.0, 12.0) - 9.0) * 9.0) + 5.0) as i16, "frame {f}");
    }
    // The animation done (timer not out): EnDekunuts_SetupStand, one loop, turning to Link by
    // Math_ApproachS(2, 0xE38).
    let n = r.get::<EnDekunuts>(h);
    assert_eq!((n.action, n.anim_flag_and_timer), (NutsAction::Stand, 1));
    let nuts_before: Vec<ActorHandle> = r.w.actors.all().into_iter().filter(|&x| r.w.actors.downcast::<EnNutsball>(x).is_some()).collect();
    r.until(80, NONE, |r| r.get::<EnDekunuts>(h).action == NutsAction::ThrowNut);
    assert_eq!(r.get::<EnDekunuts>(h).anim_flag_and_timer, 1);
    // EnDekunuts_ThrowNut: frame 6, the nut 23 ahead along shape.rot.y and 12 up
    // (ACTOR_EN_NUTSBALL, its rotation, params 0), NA_SE_EN_NUTS_THROW.
    let mut nut = None;
    for _ in 0..20 {
        r.frames(NONE, 1);
        let new: Vec<ActorHandle> = r.w.actors.all().into_iter().filter(|&x| r.w.actors.downcast::<EnNutsball>(x).is_some() && !nuts_before.contains(&x)).collect();
        if let Some(&x) = new.first() {
            // (SkelAnime_Update first, then Animation_OnFrame(6).)
            assert_eq!(r.get::<EnDekunuts>(h).skel.cur_frame, 6.0);
            assert!(r.sfx_on_last_frame(NA_SE_EN_NUTS_THROW));
            nut = Some(x);
            break;
        }
    }
    let nut = nut.expect("the nut");
    let n = r.get::<EnDekunuts>(h);
    let rot = n.actor.shape_rot;
    let want = n.actor.world_pos + Vec3::new(eng_math::sin_s(rot.y) * 23.0, 12.0, eng_math::cos_s(rot.y) * 23.0);
    let b = r.get::<EnNutsball>(nut);
    assert_eq!((b.actor.home_pos, b.actor.params, b.actor.world_rot.y), (want, 0, rot.y));
    // (The nut is taken away: this test isn't about Link's shield.)
    r.get_mut::<EnNutsball>(nut).actor.kill();
    // The spit done: EnDekunuts_SetupStand after a spit: two loops and the flag (2 | 0x1000),
    // not turning; then (Link between 120 and 480) the next round.
    r.until(40, NONE, |r| r.get::<EnDekunuts>(h).action == NutsAction::Stand);
    assert_eq!(r.get::<EnDekunuts>(h).anim_flag_and_timer, 0x1002);
    // Link within 120: at the round's end (0x1000) it burrows (NA_SE_EN_NUTS_DOWN), then waits
    // at home.
    r.w.place_player(MAD_SCRUB_HOME + Vec3::new(0.0, 0.0, -100.0), 0);
    r.until(120, NONE, |r| r.get::<EnDekunuts>(h).action == NutsAction::Burrow);
    assert!(r.sfx_on_last_frame(NA_SE_EN_NUTS_DOWN));
    r.until(40, NONE, |r| r.get::<EnDekunuts>(h).action == NutsAction::Wait);
    let n = r.get::<EnDekunuts>(h);
    assert_eq!((n.collider.dim.height, n.collider.base.ac_flags & cc::AC_ON, n.actor.world_pos), (5, 0, MAD_SCRUB_HOME));
}

/// Its nut bounced back off the Deku Shield pops the Mad Scrub out; it runs, gasps, and a slash
/// kills it.
#[test]
fn its_nut_bounced_back_pops_a_mad_scrub_out_and_a_slash_kills_it() {
    let Some(mut r) = Run::enter(4, MAD_SCRUB_START.0, MAD_SCRUB_START.1, |_| {}) else { return };
    r.frames(NONE, 3);
    let h = mad_scrub(&r);
    // R: the guard. The nut hits the shield, turns back as Link's attack, and hits the scrub:
    // EnDekunuts_ColliderCheck in the ground (immovable): EnDekunuts_SetupBeginRun: 37 high,
    // mass 50, the collider off, NA_SE_EN_NUTS_DAMAGE.
    let health = r.w.save.health;
    r.until(300, R, |r| r.get::<EnDekunuts>(h).action == NutsAction::BeginRun);
    assert!(r.sfx_on_last_frame(NA_SE_EN_NUTS_DAMAGE));
    let n = r.get::<EnDekunuts>(h);
    assert_eq!((n.collider.dim.height, n.actor.col_chk_info.mass, n.collider.base.ac_flags & cc::AC_ON), (37, 50, 0));
    assert_eq!(r.w.save.health, health, "blocked");
    // EnDekunuts_BeginRun: the unburrow animation; then EnDekunuts_SetupRun: away from Link
    // (yawTowardsPlayer + 0x8000), three runs, two loops, the collider on.
    r.until(40, NONE, |r| r.get::<EnDekunuts>(h).action == NutsAction::Run);
    let n = r.get::<EnDekunuts>(h);
    assert_eq!((n.run_away_count, n.anim_flag_and_timer, n.collider.base.ac_flags & cc::AC_ON != 0), (3, 2, true));
    // EnDekunuts_Run: Math_StepToF(speed, 7.5, 1); facing back the way it runs.
    let mut speeds = Vec::new();
    for _ in 0..9 {
        r.frames(NONE, 1);
        let n = r.get::<EnDekunuts>(h);
        if n.action != NutsAction::Run {
            break;
        }
        speeds.push(n.actor.speed_xz);
        assert_eq!(n.actor.shape_rot.y, n.actor.world_rot.y.wrapping_add(i16::MIN));
    }
    assert_eq!(&speeds[..8], &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 7.5]);
    // Its two loops done: EnDekunuts_SetupGasp: stopped, three loops, one run fewer.
    r.until(120, NONE, |r| r.get::<EnDekunuts>(h).action == NutsAction::Gasp);
    let n = r.get::<EnDekunuts>(h);
    assert_eq!((n.actor.speed_xz, n.anim_flag_and_timer, n.run_away_count), (0.0, 3, 2));
    // Caught while it gasps: Link 30 off facing it, B. The Kokiri Sword (sDamageTable: 1) with
    // mass 50: EnDekunuts_SetupBeDamaged: knocked away from Link at 10, red for the damage
    // animation's last frame, NA_SE_EN_NUTS_DAMAGE and _CUTBODY; 1 health: the finishing blow.
    let p = n.actor.world_pos;
    let yaw_to = eng_math::vec3f_yaw(p + Vec3::new(0.0, 0.0, -30.0), p);
    r.w.place_player(p + Vec3::new(0.0, 0.0, -30.0), yaw_to);
    r.until(20, B, |r| r.get::<EnDekunuts>(h).action == NutsAction::BeDamaged);
    assert!(r.sfx_on_last_frame(NA_SE_EN_NUTS_CUTBODY));
    let n = r.get::<EnDekunuts>(h);
    let last = n.skel.animation.as_ref().unwrap().last_frame();
    assert_eq!((n.actor.col_chk_info.health, n.actor.speed_xz), (0, 9.0), "one step of Math_StepToF(speed, 0, 1) after 10");
    assert_eq!(n.actor.color_filter_params & 0xFF, last as u16 & 0xFF);
    assert_ne!(n.actor.color_filter_params & oot_game::actor::COLORFILTER_COLORFLAG_RED, 0);
    // EnDekunuts_BeDamaged to the animation's end, EnDekunuts_SetupDie (NA_SE_EN_NUTS_DEAD),
    // then EnDekunuts_Die's end: the white puff (EffectSsDeadDb), 15 fragments
    // (EffectSsHahen_SpawnBurst), its flower a prop, gone.
    let flower = n.actor.child.unwrap();
    r.until(60, NONE, |r| r.get::<EnDekunuts>(h).action == NutsAction::Die);
    assert!(r.sfx_on_last_frame(NA_SE_EN_NUTS_DEAD));
    // (Room 4's Gohma eggs shed fragments of their own: only those at the scrub count.)
    let near = |r: &Run, p: Vec3| r.w.effect_ss.table.iter().filter(|e| e.life >= 0 && e.ty == EFFECT_SS_HAHEN && e.pos.distance(p) < 20.0).count();
    let p = r.w.actors.actor(h).unwrap().world_pos;
    let hahen_before = near(&r, p);
    r.until(60, NONE, |r| r.w.actors.downcast::<EnDekunuts>(h).is_none_or(|n| n.actor.killed));
    assert_eq!(r.effects(EFFECT_SS_DEAD_DB), 1);
    assert_eq!(near(&r, p), hahen_before + 15);
    assert_eq!(r.get::<EnDekunuts>(flower).actor.category, ACTORCAT_PROP);
    assert!(r.w.actors.category(ACTORCAT_PROP).contains(&flower));
}

/// Out of the ground, a Deku Nut's flash (reaction 1, no damage) stuns it for five loops; then it
/// runs. A hit by fire (reaction 2) puts a ring of fire round it.
#[test]
fn a_deku_nut_stuns_a_mad_scrub_out_of_the_ground_and_fire_rings_it() {
    let Some(mut r) = Run::enter(4, MAD_SCRUB_START.0, MAD_SCRUB_START.1, |_| {}) else { return };
    r.frames(NONE, 3);
    let h = mad_scrub(&r);
    r.until(300, R, |r| r.get::<EnDekunuts>(h).action == NutsAction::Run);
    // The collision check's result for a Deku Nut (sDamageTable: DMG_ENTRY(0, 0x1)), driven.
    let hit = |r: &mut Run, reaction: u8, damage: u8| {
        let n = r.get_mut::<EnDekunuts>(h);
        n.collider.base.ac_flags |= cc::AC_HIT;
        n.actor.col_chk_info.damage_reaction = reaction;
        n.actor.col_chk_info.damage = damage;
        r.frames(NONE, 1);
    };
    hit(&mut r, 1, 0);
    // EnDekunuts_SetupBeStunned: five loops of the damage animation, stopped, blue for the last
    // frame × 5, NA_SE_EN_GOMA_JR_FREEZE.
    let n = r.get::<EnDekunuts>(h);
    let last = n.skel.animation.as_ref().unwrap().last_frame() as i16;
    assert_eq!((n.action, n.anim_flag_and_timer, n.actor.speed_xz), (NutsAction::BeStunned, 5, 0.0));
    assert!(r.sfx_on_last_frame(NA_SE_EN_GOMA_JR_FREEZE));
    assert_eq!(n.actor.color_filter_params & 0xFF, (last * 5) as u16 & 0xFF);
    assert_eq!(n.actor.color_filter_params & oot_game::actor::COLORFILTER_COLORFLAG_RED, 0);
    // A second stun while stunned does nothing.
    hit(&mut r, 1, 0);
    assert_eq!(r.get::<EnDekunuts>(h).anim_flag_and_timer, 5);
    // Each loop NA_SE_EN_NUTS_FAINT, four of them; the fifth, it runs.
    let faint0 = r.sfx_count(NA_SE_EN_NUTS_FAINT);
    r.until(200, NONE, |r| r.get::<EnDekunuts>(h).action == NutsAction::Run);
    assert_eq!(r.sfx_count(NA_SE_EN_NUTS_FAINT) - faint0, 4);
    // Fire (reaction 2, 4 damage: a fire arrow or Din's Fire): EffectSsFCircle_Spawn(40, 50) at
    // it, then damaged.
    hit(&mut r, 2, 4);
    assert_eq!(r.get::<EnDekunuts>(h).action, NutsAction::BeDamaged);
    let ring = r.w.effect_ss.table.iter().find(|e| e.life >= 0 && e.ty == oot_game::effect::EFFECT_SS_FCIRCLE).expect("the ring of fire");
    assert_eq!((ring.regs[8], ring.regs[9], ring.actor), (40, 50, Some(h)));
}

const HINT_HOMES: [Vec3; 3] = [Vec3::new(-369.0, -1880.0, -904.0), Vec3::new(-947.0, -1880.0, -757.0), Vec3::new(-660.0, -1880.0, -951.0)];

/// Room 9's three hint scrubs, by their place in the puzzle (params 1, 2, 3).
fn hint_scrubs(r: &Run) -> [ActorHandle; 3] {
    HINT_HOMES.map(|p| r.find::<EnHintnuts>(p, |n| n.actor.params != oot_actors::en_hintnuts::HINTNUTS_FLOWER).expect("En_Hintnuts"))
}

fn puzzle_counter(r: &mut Run) -> i16 {
    r.w.overlay_static::<oot_actors::en_hintnuts::Statics>(oot_actors::en_hintnuts::ACTOR_EN_HINTNUTS).puzzle_counter
}

/// A nut (`ACTOR_EN_NUTSBALL`, as the collision check would report it) hitting scrub `h`, driven.
fn nut_hits(r: &mut Run, h: ActorHandle) {
    let nut = r.w.actor_spawn(oot_actors::en_nutsball::ACTOR_EN_NUTSBALL, Vec3::new(-660.0, -1800.0, -700.0), [0; 3], 1).expect("a nut");
    let n = r.get_mut::<EnHintnuts>(h);
    n.collider.base.ac_flags |= cc::AC_HIT;
    n.collider.base.ac = Some(nut);
    r.frames(NONE, 1);
    if let Some(a) = r.w.actors.actor_mut(nut) {
        a.kill();
    }
}

/// `EnHintnuts_HitByScrubProjectile2`'s count and `EnHintnuts_Freeze`: out of order, the third
/// wrong one plays `NA_SE_SY_ERROR` (-3 -> -4) and the three sink and come back; in order, the
/// third runs to talk, and its talk ends the puzzle.
#[test]
fn the_hint_scrubs_order_puzzle() {
    let Some(mut r) = Run::enter(9, Vec3::new(-660.0, -1880.0, -620.0), i16::MIN, |_| {}) else { return };
    r.frames(NONE, 3);
    let [s1, s2, s3] = hint_scrubs(&r);
    // EnHintnuts_Init: the text with the Deku Tree's prefix (Actor_SetTextWithPrefix: 0x1000),
    // params to the low byte, sPuzzleCounter 0.
    let texts: Vec<(u16, i16)> = [s1, s2, s3].iter().map(|&h| (r.get::<EnHintnuts>(h).text_id_copy, r.get::<EnHintnuts>(h).actor.params)).collect();
    assert_eq!(texts, vec![(0x1000, 1), (0x1000, 2), (0x109C, 3)]);
    assert_eq!(puzzle_counter(&mut r), 0);
    // A sword's hit (not a nut): EnHintnuts_SetupBurrow, no count.
    {
        let pl = r.w.player;
        let n = r.get_mut::<EnHintnuts>(s1);
        n.collider.base.ac_flags |= cc::AC_HIT;
        n.collider.base.ac = pl;
    }
    r.frames(NONE, 1);
    assert_eq!((r.get::<EnHintnuts>(s1).action, puzzle_counter(&mut r)), (HintAction::Burrow, 0));
    r.until(40, NONE, |r| r.get::<EnHintnuts>(s1).action == HintAction::Wait);
    // Out of order: 2 (0 -> -1), 1 (-1 -> -2), 3 (-2 -> -3): each pops out (37 high,
    // NA_SE_EN_NUTS_DAMAGE) and freezes (EnHintnuts_SetupFreeze: blue, untargetable,
    // NA_SE_EN_NUTS_FAINT).
    for (h, want) in [(s2, -1), (s1, -2)] {
        nut_hits(&mut r, h);
        assert_eq!((r.get::<EnHintnuts>(h).action, r.get::<EnHintnuts>(h).collider.dim.height, puzzle_counter(&mut r)), (HintAction::BeginFreeze, 37, want));
        r.until(40, NONE, |r| r.get::<EnHintnuts>(h).action == HintAction::Freeze);
        assert_eq!(r.get::<EnHintnuts>(h).actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
    }
    let errors = r.sfx_count(NA_SE_SY_ERROR);
    nut_hits(&mut r, s3);
    assert_eq!((r.get::<EnHintnuts>(s3).action, r.get::<EnHintnuts>(s3).actor.category, puzzle_counter(&mut r)), (HintAction::BeginFreeze, ACTORCAT_ENEMY, -3));
    r.until(40, NONE, |r| r.get::<EnHintnuts>(s3).action == HintAction::Freeze);
    assert_eq!((puzzle_counter(&mut r), r.sfx_count(NA_SE_SY_ERROR)), (-4, errors + 1));
    // All three sink to home - 35 at 7 a frame and wait again, targetable, their health back.
    r.until(30, NONE, |r| [s1, s2, s3].iter().all(|&h| r.get::<EnHintnuts>(h).action == HintAction::Wait));
    for h in [s1, s2, s3] {
        let n = r.get::<EnHintnuts>(h);
        assert_ne!(n.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED, 0);
        assert_eq!((n.actor.col_chk_info.health, n.actor.world_pos), (1, n.actor.home_pos));
    }
    // In order: 1 (-4 -> 0 -> 1), 2 (2): frozen; 3 with the count at 2:
    // EnHintnuts_HitByScrubProjectile1 makes it friendly (ACTORCAT_BG), so it runs.
    for (h, want) in [(s1, 1), (s2, 2)] {
        nut_hits(&mut r, h);
        assert_eq!(puzzle_counter(&mut r), want);
        r.until(40, NONE, |r| r.get::<EnHintnuts>(h).action == HintAction::Freeze);
    }
    nut_hits(&mut r, s3);
    let c = puzzle_counter(&mut r);
    let n = r.get::<EnHintnuts>(s3);
    assert_eq!((n.action, n.actor.category, c), (HintAction::BeginRun, ACTORCAT_BG, 2));
    assert_eq!(n.actor.flags & (ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_HOSTILE), ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY);
    assert!(r.w.actors.category(ACTORCAT_BG).contains(&s3));
    r.until(40, NONE, |r| r.get::<EnHintnuts>(s3).action == HintAction::Run);
    assert_eq!(r.get::<EnHintnuts>(s3).anim_flag_and_timer, 5);
    // Caught: Link touching it (OC1_HIT) makes the offer auto-accepted; Player takes it and the
    // message box opens with 0x109C; EnHintnuts_Talk; the text's event: EnHintnuts_SetupLeave
    // (a recovery heart, ACTOR_EN_ITEM00 3), running off.
    let hearts0 = r.w.actors.all().into_iter().filter(|&x| r.w.actors.downcast::<EnItem00>(x).is_some_and(|i| i.actor.params == 3)).count();
    let p = r.w.actors.actor(s3).unwrap().world_pos;
    r.w.place_player(p + Vec3::new(0.0, 0.0, 25.0), i16::MIN);
    r.until(30, NONE, |r| r.get::<EnHintnuts>(s3).action == HintAction::Talk || r.w.actors.actor(s3).is_some_and(|a| a.flags & ACTOR_FLAG_TALK != 0));
    r.until(30, NONE, |r| r.get::<EnHintnuts>(s3).action == HintAction::Talk);
    assert_eq!(r.w.msg_ctx.text_id, 0x109C);
    r.advance(600, |r| r.get::<EnHintnuts>(s3).action == HintAction::Leave);
    assert_eq!(r.get::<EnHintnuts>(s3).action, HintAction::Leave);
    assert_eq!(r.w.actors.all().into_iter().filter(|&x| r.w.actors.downcast::<EnItem00>(x).is_some_and(|i| i.actor.params == 3)).count(), hearts0 + 1);
    // Its 100 frames (or off screen): the room cleared, sPuzzleCounter 3; the frozen two sink and
    // are gone; the flowers props.
    r.until(120, NONE, |r| r.w.actors.downcast::<EnHintnuts>(s3).is_none_or(|n| n.actor.killed));
    assert!(r.w.flags.get_clear(9));
    assert_eq!(puzzle_counter(&mut r), 3);
    r.until(30, NONE, |r| [s1, s2].iter().all(|&h| r.w.actors.downcast::<EnHintnuts>(h).is_none_or(|n| n.actor.killed)));
    let flowers: Vec<ActorHandle> =
        r.w.actors.all().into_iter().filter(|&x| r.w.actors.downcast::<EnHintnuts>(x).is_some_and(|n| n.actor.params == oot_actors::en_hintnuts::HINTNUTS_FLOWER)).collect();
    assert_eq!(flowers.len(), 3);
    for f in flowers {
        assert_eq!(r.w.actors.actor(f).unwrap().category, ACTORCAT_PROP);
    }
}

const BUSINESS_SCRUB_HOME: Vec3 = Vec3::new(-562.0, -820.0, 332.0);
/// 220 from it towards -x and -z, facing it (on the floor: room 3's floor round it has holes,
/// down to room 9, to its -z and further to its -x; and it only comes up with Link 160 to 480
/// away).
const BUSINESS_SCRUB_START: (Vec3, i16) = (Vec3::new(-718.0, -820.0, 177.0), 8202);

/// The Business Scrub hit pops out as the salesman (`En_Dns` 4, the Deku Shield for 50); with the
/// shield owned he can't sell it (0x10A6) and burrows away; one selling a piece of heart already
/// bought isn't there.
#[test]
fn a_caught_business_scrub_becomes_the_salesman() {
    let Some(mut r) = Run::enter(3, BUSINESS_SCRUB_START.0, BUSINESS_SCRUB_START.1, |_| {}) else { return };
    r.frames(NONE, 3);
    let h = r.find::<EnShopnuts>(BUSINESS_SCRUB_HOME, |_| true).expect("En_Shopnuts");
    let n = r.get::<EnShopnuts>(h);
    assert_eq!((n.actor.params, n.action, n.collider.dim.height), (4, ShopAction::Idle, 5));
    // Hit (its own nut back, or a sword): EnShopnuts_SetupSpawnSalesman (NA_SE_EN_NUTS_DAMAGE,
    // the collider off), turning to Link as it spins up; then ACTOR_EN_DNS with its params in its
    // place, and it's gone.
    {
        let n = r.get_mut::<EnShopnuts>(h);
        n.collider.base.ac_flags |= cc::AC_HIT;
    }
    r.frames(NONE, 1);
    assert_eq!(r.get::<EnShopnuts>(h).action, ShopAction::SpawnSalesman);
    assert!(r.sfx_on_last_frame(NA_SE_EN_NUTS_DAMAGE));
    r.until(60, NONE, |r| r.w.actors.downcast::<EnShopnuts>(h).is_none_or(|n| n.actor.killed));
    r.frames(NONE, 1);
    let d = r.find::<EnDns>(BUSINESS_SCRUB_HOME, |_| true).expect("En_Dns");
    let dns = r.get::<EnDns>(d);
    // EnDns_Init: type 4, the Deku Shield (sItemDekuShield: 50, GI_SHIELD_DEKU), text 0x10CB
    // (sStartingTextIds[4]), immovable, gravity -1; a BG actor, friendly.
    assert_eq!((dns.actor.params, dns.actor.text_id, dns.dns_item_entry.item_price, dns.dns_item_entry.get_item_id), (4, 0x10CB, 50, oot_game::item::GI_SHIELD_DEKU));
    assert_eq!((dns.actor.category, dns.actor.col_chk_info.mass, dns.actor.gravity), (ACTORCAT_BG, cc::MASS_IMMOVABLE, -1.0));
    assert_eq!((dns.actor.world_pos.x, dns.actor.world_pos.z), (BUSINESS_SCRUB_HOME.x, BUSINESS_SCRUB_HOME.z));
    // EnDns_SetupIdle: the transition to its last frame, then EnDns_Idle.
    r.until(60, NONE, |r| r.get::<EnDns>(d).action == DnsAction::Idle);
    // Within 130, the talk; A takes it: 0x10CB, its choice; "Yes" (choice 0): the Deku Shield is
    // owned (EnDns_CanBuyDekuShield: DNS_CANBUY_RESULT_CAPACITY_FULL): 0x10A6, no sale.
    let p = r.w.actors.actor(d).unwrap().world_pos;
    r.w.place_player(p + Vec3::new(-40.0, 0.0, 0.0), 0x4000);
    r.frames(NONE, 2);
    r.press(A);
    r.until(30, NONE, |r| r.get::<EnDns>(d).action == DnsAction::Talk);
    assert_eq!(r.w.msg_ctx.text_id, 0x10CB);
    r.advance(600, |r| r.w.message_state() == TEXT_STATE_CHOICE);
    assert_eq!((r.w.msg_ctx.text_id, r.w.msg_ctx.choice_index), (0x10A3, 0));
    let rupees = r.w.save.rupees;
    r.press(A);
    assert_eq!((r.get::<EnDns>(d).action, r.w.msg_ctx.text_id), (DnsAction::SetupNoSaleBurrow, 0x10A6));
    // The text done and A: burrowing (the collider off, untargetable); at the animation's last
    // frame NA_SE_EN_AKINDONUTS_HIDE, down through the floor spinning by 0x2000, dust every 4
    // frames (func_80028990: 20 puffs); 400 down, gone, no hearts (no sale), nothing paid.
    r.advance(300, |r| r.get::<EnDns>(d).action == DnsAction::Burrow);
    let dns = r.get::<EnDns>(d);
    assert_eq!((dns.action, dns.is_collider_enabled, dns.actor.flags & ACTOR_FLAG_ATTENTION_ENABLED), (DnsAction::Burrow, false, 0));
    r.until(120, NONE, |r| r.get::<EnDns>(d).action == DnsAction::PostBurrow);
    assert!(r.sfx_on_last_frame(NA_SE_EN_AKINDONUTS_HIDE));
    let y0 = r.get::<EnDns>(d).y_init_pos;
    let rot0 = r.get::<EnDns>(d).actor.shape_rot.y;
    r.frames(NONE, 1);
    assert_eq!(r.get::<EnDns>(d).actor.shape_rot.y, rot0.wrapping_add(0x2000));
    let hearts0 = r.w.actors.all().into_iter().filter(|&x| r.w.actors.downcast::<EnItem00>(x).is_some()).count();
    r.until(120, NONE, |r| r.w.actors.downcast::<EnDns>(d).is_none_or(|n| n.actor.killed));
    assert!(y0 - r.w.actors.actor(d).map(|a| a.world_pos.y).unwrap_or(y0 - 401.0) > 400.0);
    r.frames(NONE, 1);
    assert_eq!(r.w.actors.all().into_iter().filter(|&x| r.w.actors.downcast::<EnItem00>(x).is_some()).count(), hearts0);
    assert_eq!(r.w.save.rupees, rupees);

    // EnShopnuts_Init: one selling the piece of heart (type 2) with ITEMGETINF_DEKU_HEART_PIECE
    // set is gone at once.
    r.w.save.set_item_get_inf(oot_actors::en_dns::ITEMGETINF_DEKU_HEART_PIECE);
    let gone = r.w.actor_spawn(oot_actors::en_shopnuts::ACTOR_EN_SHOPNUTS, BUSINESS_SCRUB_HOME, [0; 3], 2).expect("spawned");
    r.frames(NONE, 2);
    assert!(r.w.actors.downcast::<EnShopnuts>(gone).is_none_or(|n| n.actor.killed));
}

/// `EnDns_CanBuy*`: with 50 rupees and no Deku Shield, "Yes" sells it: 0x10A7, the item offered
/// and taken (`Actor_HasParent`), paid (`Rupees_ChangeBy(-50)`) when its text is done; he burrows
/// and leaves three recovery hearts.
#[test]
fn the_salesman_sells_the_deku_shield() {
    let Some(mut r) = Run::enter(3, BUSINESS_SCRUB_START.0, BUSINESS_SCRUB_START.1, |s| {
        s.rupees = 60;
        s.inventory.equipment &= !oot_game::item::owned_equip_flag(oot_game::item::EQUIP_TYPE_SHIELD, oot_game::item::EQUIP_INV_SHIELD_DEKU);
        s.inventory_change_equipment(oot_game::item::EQUIP_TYPE_SHIELD, oot_game::item::EQUIP_VALUE_SHIELD_NONE);
    }) else {
        return;
    };
    r.frames(NONE, 3);
    assert!(!r.w.save.check_owned_equip(oot_game::item::EQUIP_TYPE_SHIELD, oot_game::item::EQUIP_INV_SHIELD_DEKU));
    let h = r.find::<EnShopnuts>(BUSINESS_SCRUB_HOME, |_| true).expect("En_Shopnuts");
    r.get_mut::<EnShopnuts>(h).collider.base.ac_flags |= cc::AC_HIT;
    r.until(80, NONE, |r| r.find::<EnDns>(BUSINESS_SCRUB_HOME, |_| true).is_some());
    let d = r.find::<EnDns>(BUSINESS_SCRUB_HOME, |_| true).unwrap();
    r.until(60, NONE, |r| r.get::<EnDns>(d).action == DnsAction::Idle);
    let p = r.w.actors.actor(d).unwrap().world_pos;
    r.w.place_player(p + Vec3::new(-40.0, 0.0, 0.0), 0x4000);
    r.frames(NONE, 2);
    r.press(A);
    r.advance(600, |r| r.w.message_state() == TEXT_STATE_CHOICE);
    r.press(A);
    assert_eq!((r.get::<EnDns>(d).action, r.w.msg_ctx.text_id), (DnsAction::SetupSale, 0x10A7));
    // 0x10A7's event and A: the box closed and the shield offered; Link takes it.
    r.advance(600, |r| r.get::<EnDns>(d).action != DnsAction::SetupSale);
    assert_eq!(r.get::<EnDns>(d).action, DnsAction::Sale);
    r.until(200, NONE, |r| r.get::<EnDns>(d).action != DnsAction::Sale);
    assert_eq!(r.get::<EnDns>(d).action, DnsAction::SetupBurrow);
    // Link holds it up (PLAYER_STATE1_10): paid once its text is done and A.
    r.advance(600, |r| r.get::<EnDns>(d).action != DnsAction::SetupBurrow);
    assert_eq!(r.get::<EnDns>(d).action, DnsAction::Burrow);
    assert!(r.w.save.check_owned_equip(oot_game::item::EQUIP_TYPE_SHIELD, oot_game::item::EQUIP_INV_SHIELD_DEKU));
    assert_eq!(r.w.save.rupees + r.w.save.rupee_accumulator, 10);
    // Down and gone: three recovery hearts (Item_DropCollectible ITEM00_RECOVERY_HEART) where
    // he went down.
    let hearts0 = r.w.actors.all().into_iter().filter(|&x| r.w.actors.downcast::<EnItem00>(x).is_some()).count();
    r.until(200, NONE, |r| r.w.actors.downcast::<EnDns>(d).is_none_or(|n| n.actor.killed));
    r.frames(NONE, 1);
    let hearts = r.w.actors.all().into_iter().filter(|&x| r.w.actors.downcast::<EnItem00>(x).is_some()).count();
    assert_eq!(hearts, hearts0 + 3);
}
