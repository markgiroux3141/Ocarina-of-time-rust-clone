//! GAME-10 milestone 1's soft sprites against the C, where they need the play state:
//! `Actor_UpdateBgCheckInfo`'s three ripples (`z_actor.c`), `Effect_Ss_G_Ripple`,
//! `Effect_Ss_G_Splash`'s second splash, `Effect_Ss_Bubble`'s rise to a ripple,
//! `Effect_Ss_Dt_Bubble`'s wobble and `Effect_Ss_Lightning`'s branching (`EffectSs_Insert`); and an
//! update's spawn taking its own slot when the table's full. The pure parts (inits, steps,
//! matrices) are the modules' unit tests. These need the asset pack (for `GameData`).
//!
//! The random numbers are replayed from a copy of the generator, the C's calls in its order.

use std::sync::{Arc, OnceLock};

use eng_collision::bgcheck::CollisionContext;
use eng_collision::collision::CollisionBuilder;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_WATER, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_6};
use oot_game::actor_ctx::{ACTORCAT_MISC, ActorImpl};
use oot_game::data::GameData;
use oot_game::effect::{EFFECT_SS_BUBBLE, EFFECT_SS_G_RIPPLE, EFFECT_SS_G_SPLASH, EFFECT_SS_LIGHTNING, EffectSs, bubble, dt_bubble, g_ripple, g_splash, lightning};
use oot_game::pack::{GamePack, keys};
use oot_game::play::{PlayState, Rand, scripted_input};
use oot_game::player_lib::PlayerRules;

struct Ctx {
    data: Arc<GameData>,
    rules: Arc<PlayerRules>,
}

fn ctx() -> Option<&'static Ctx> {
    static C: OnceLock<Option<Ctx>> = OnceLock::new();
    C.get_or_init(|| {
        let pack = match GamePack::open_default() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("skipping: {e:#}");
                return None;
            }
        };
        Some(Ctx { data: Arc::new(pack.game_data().unwrap()), rules: Arc::new(pack.player_rules().unwrap()) })
    })
    .as_ref()
}

/// A floor at 0 and a water box over it, its surface at 50 (x and z from -200 to 200, all rooms).
fn play() -> Option<PlayState> {
    let c = ctx()?;
    let mut b = CollisionBuilder::new();
    let s = b.surface(0, 0);
    b.quad(Vec3::new(-500.0, 0.0, 500.0), Vec3::new(500.0, 0.0, 500.0), Vec3::new(500.0, 0.0, -500.0), Vec3::new(-500.0, 0.0, -500.0), s);
    b.water_box(-200, -200, 400, 400, 50, 0x3F);
    Some(PlayState::new(c.data.clone(), c.rules.clone(), CollisionContext::new(b.finish()), (Vec3::ZERO, 0), true))
}

fn frame(p: &mut PlayState) {
    p.tick_with(scripted_input(Default::default(), Default::default()));
}

fn live(p: &PlayState, ty: u8) -> Vec<(usize, EffectSs)> {
    p.effect_ss.table.iter().enumerate().filter(|(_, e)| e.life > -1 && e.ty == ty).map(|(i, e)| (i, e.clone())).collect()
}

/// An actor that drops to `y` and checks the floor and water there each update.
struct Diver {
    actor: Actor,
    y: f32,
    flags: u32,
}

impl ActorImpl for Diver {
    fn name(&self) -> &'static str {
        "Diver"
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    fn update(&mut self, play: &mut PlayState) {
        self.actor.world_pos.y = self.y;
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, self.flags);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn diver(p: &mut PlayState, x: f32, flags: u32) {
    let mut actor = Actor::new(Vec3::new(x, 100.0, 30.0), 0);
    actor.category = ACTORCAT_MISC;
    actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
    p.spawn(Box::new(Diver { actor, y: 40.0, flags }));
}

#[test]
fn entering_the_water_spawns_three_ripples_unless_flag_6() {
    let Some(mut p) = play() else { return };
    diver(&mut p, 20.0, UPDBGCHECKINFO_FLAG_2);
    diver(&mut p, -20.0, UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_6);
    frame(&mut p);
    // z_actor.c: EffectSsGRipple_Spawn(play, &ripplePos, 100, 500, 0 / 4 / 8) at (x, ySurface, z),
    // for the first diver only. EffectSs_UpdateAll ran after the actors: the lives (20, 24, 28)
    // down one, the first ripple grown (DECR(0) is 0: 100 + 30, alpha 255 - 15), the others
    // counting down (3, 7).
    let r = live(&p, EFFECT_SS_G_RIPPLE);
    assert_eq!(r.len(), 3, "{r:?}");
    for (_, e) in &r {
        assert_eq!(e.pos, Vec3::new(20.0, 50.0, 30.0));
        // The water box (index 0) within 3 of the surface.
        assert_eq!((e.regs[g_ripple::R_WATER_BOX_NUM], e.regs[g_ripple::R_RADIUS_MAX]), (0, 500));
    }
    let got: Vec<_> = r.iter().map(|(_, e)| (e.life, e.regs[g_ripple::R_LIFESPAN], e.regs[g_ripple::R_RADIUS], e.regs[g_ripple::R_PRIM_COLOR_A])).collect();
    assert_eq!(got, [(19, 0, 130, 240), (23, 3, 100, 255), (27, 7, 100, 255)]);
    // EffectSsGRipple_Draw: only the first (the others wait, rLifespan not 0).
    let ripple_mesh = eng_gfx::MeshKey::named(keys::bake(g_ripple::BAKE));
    assert_eq!(p.effect_draws.xlu.iter().filter(|d| d.mesh == ripple_mesh).count(), 1);
    // In the water now (BGCHECKFLAG_WATER): no more.
    frame(&mut p);
    assert_eq!(live(&p, EFFECT_SS_G_RIPPLE).len(), 3);
    for h in p.actors.all() {
        assert_ne!(p.actors.actor(h).unwrap().bg_check_flags & BGCHECKFLAG_WATER, 0);
    }
}

#[test]
fn a_bubble_wobbles_up_and_ends_in_a_ripple() {
    let Some(mut p) = play() else { return };
    p.rand = Rand { state: 99 };
    let mut r = p.rand;
    // En_Door's bubble without its spread: at (10, 20 + 0, 30), scale 0.15.
    p.with_ss(|s| s.bubble_spawn(Vec3::new(10.0, 20.0, 30.0), 0.0, 0.0, 0.0, 0.15));
    // The init's seven calls: the texture (two), x, y, z (no spread), the scale.
    let tex = bubble::random_texture(&mut r);
    for _ in 0..3 {
        r.zero_one();
    }
    let scale = ((((r.zero_one() * 0.5) + 1.0) * 0.15) * 100.0) as i16;
    let (i, e) = live(&p, EFFECT_SS_BUBBLE).remove(0);
    assert_eq!((e.pos, e.life, e.gfx, e.regs[bubble::R_SCALE]), (Vec3::new(10.0, 20.0, 30.0), 1, Some(("gameplay_keep", tex)), scale));
    // EffectSs_UpdateAll: life 1 to 0, then the update under the surface puts it back to 1:
    // x and z within a quarter of where it started, next frame's rise (Rand - 0.3) × 0.2.
    p.effect_ss_update_all();
    let x = ((r.zero_one() * 0.5) - 0.25) + 10.0;
    let ay = (r.zero_one() - 0.3) * 0.2;
    let z = ((r.zero_one() * 0.5) - 0.25) + 30.0;
    let e = &p.effect_ss.table[i];
    assert_eq!((e.life, e.pos, e.accel.y), (1, Vec3::new(x, 20.0, z), ay));
    assert_eq!(p.rand, r);
    // The next frame it rises by that.
    p.effect_ss_update_all();
    let e = &p.effect_ss.table[i];
    assert_eq!((e.life, e.velocity.y, e.pos.y), (1, ay, 20.0 + ay));
    // Above the surface (moved there): a ripple at the surface (0 to 80, at once), the bubble gone.
    p.effect_ss.table[i].pos.y = 60.0;
    p.effect_ss.table[i].velocity = Vec3::ZERO;
    p.effect_ss.table[i].accel = Vec3::ZERO;
    let (bx, bz) = (p.effect_ss.table[i].pos.x, p.effect_ss.table[i].pos.z);
    p.effect_ss_update_all();
    assert!(live(&p, EFFECT_SS_BUBBLE).is_empty());
    let ripples = live(&p, EFFECT_SS_G_RIPPLE);
    assert_eq!(ripples.len(), 1);
    let (_, rp) = &ripples[0];
    // Spawned in a later slot than the bubble's: updated the same frame (life 20 to 19, grown 0
    // + 80 × 0.2 = 16).
    assert_eq!((rp.pos, rp.life, rp.regs[g_ripple::R_RADIUS], rp.regs[g_ripple::R_RADIUS_MAX]), (Vec3::new(bx, 50.0, bz), 19, 16, 80));
    // Out of every water box: gone at once, no ripple.
    p.with_ss(|s| s.bubble_spawn(Vec3::new(300.0, 20.0, 0.0), 0.0, 0.0, 0.0, 0.15));
    p.effect_ss_update_all();
    assert!(live(&p, EFFECT_SS_BUBBLE).is_empty());
    assert_eq!(live(&p, EFFECT_SS_G_RIPPLE).len(), 1);
}

#[test]
fn a_type_1_splash_throws_up_a_type_2() {
    let Some(mut p) = play() else { return };
    p.with_ss(|s| s.g_splash_spawn(Vec3::new(0.0, 50.0, 0.0), None, None, 1, 400));
    // Lives 8 to 7, 6, then 5: the second splash, 400 × 20 × 0.002 = 16 above, scale 200.
    for _ in 0..2 {
        p.effect_ss_update_all();
    }
    assert_eq!(live(&p, EFFECT_SS_G_SPLASH).len(), 1);
    p.effect_ss_update_all();
    let s = live(&p, EFFECT_SS_G_SPLASH);
    assert_eq!(s.len(), 2);
    let (first, second) = (&s[0].1, &s[1].1);
    assert_eq!((first.life, first.regs[g_splash::RG_TEX_IDX]), (5, 300));
    // The new one, in a later slot, updated the same frame (life 7, texture 100).
    assert_eq!((second.pos, second.life, second.regs[g_splash::RG_SCALE], second.regs[g_splash::RG_TEX_IDX]), (Vec3::new(0.0, 66.0, 0.0), 7, 200, 100));
    assert_eq!((second.regs[g_splash::R_TYPE], second.regs[g_splash::RG_PRIM_COLOR_A]), (2, 200));
    // A type 2 throws nothing.
    for _ in 0..8 {
        p.effect_ss_update_all();
    }
    assert!(live(&p, EFFECT_SS_G_SPLASH).is_empty());
}

#[test]
fn a_full_table_hands_the_updating_splash_slot_to_its_spawn() {
    let Some(mut p) = play() else { return };
    // Every slot taken at priority 128 (a still effect with no update), the splash in slot 0
    // about to reach life 5, the search starting at 0.
    for e in p.effect_ss.table.iter_mut() {
        *e = EffectSs { life: 100, priority: 128, ty: EFFECT_SS_BUBBLE, ..Default::default() };
    }
    p.with_ss(|s| s.g_splash_spawn(Vec3::new(0.0, 50.0, 0.0), None, None, 1, 400));
    p.effect_ss.search_start_index = 0;
    // The splash went where the search started (0: no slot free, the first of equal priority).
    let mut s = p.effect_ss.table[0].clone();
    assert_eq!(s.update, Some(oot_game::effect::SsUpdate::GSplash));
    s.ty = EFFECT_SS_G_SPLASH;
    s.life = 6;
    p.effect_ss.table[0] = s;
    p.effect_ss_update_all();
    // EffectSs_FindSlot gave the splash's own slot: the type 2 is there, and the update's
    // `rgTexIdx += rgTexIdxStep` then stepped it (the C's `this` is the slot).
    let e = &p.effect_ss.table[0];
    assert_eq!((e.ty, e.life, e.regs[g_splash::R_TYPE], e.regs[g_splash::RG_SCALE], e.regs[g_splash::RG_TEX_IDX]), (EFFECT_SS_G_SPLASH, 8, 2, 200, 100));
}

#[test]
fn a_dt_bubble_wobbles_with_rand_xz() {
    let Some(mut p) = play() else { return };
    p.with_ss(|s| s.dt_bubble_spawn_color_profile(Vec3::new(5.0, 20.0, 5.0), Vec3::new(0.0, 1.5, 0.0), Vec3::ZERO, 100, 25, 0, 1));
    let mut r = p.rand;
    p.effect_ss_update_all();
    // Moved by its velocity, then x and z by up to 1 either way.
    let x = 5.0 + ((r.zero_one() * 2.0) - 1.0);
    let z = 5.0 + ((r.zero_one() * 2.0) - 1.0);
    let (_, e) = &live(&p, oot_game::effect::EFFECT_SS_DT_BUBBLE)[0];
    assert_eq!((e.pos, e.life), (Vec3::new(x, 21.5, z), 24));
    // 255 × 24 / 25 = 244.
    assert_eq!(dt_bubble::colors(e).0, [255, 255, 100, 244]);
}

#[test]
fn a_lightning_bolt_branches_into_the_next_slots() {
    let Some(mut p) = play() else { return };
    // A seed whose third Rand_ZeroOne is under 0.1: the mirrored second bolt too.
    let seed = (1u32..).find(|&s| {
        let mut r = Rand { state: s };
        r.zero_one();
        r.zero_one();
        r.zero_one() < 0.1
    });
    p.rand = Rand { state: seed.unwrap() };
    let mut r = p.rand;
    let start = Vec3::new(0.0, 100.0, 0.0);
    // One bolt left, 4 frames, yaw 0x1000, scale 100.
    p.with_ss(|s| s.lightning_spawn(start, [255, 255, 255, 255], [0, 0, 255, 255], 100, 0x1000, 4, 1));
    p.effect_ss_update_all();
    // Its first update (life 3, + 1 = rLifespan): the yaw 0x1000 ± ((s16)(Rand × 3640) + 0xE38).
    let sign = if r.zero_one() < 0.5 { -1 } else { 1 };
    let yaw = (0x1000 + sign * (((r.zero_one() * 3640.0) as i16) as i32 + 0xE38)) as i16;
    // Its end: 100 × 0.01 × 80 along the yaw less a quarter turn, across by the camera's yaw.
    let scale = (100.0f32 * 0.01) * 80.0;
    let y = 100.0 + eng_math::sin_s(0x1000 - 0x4000) * scale;
    let across = eng_math::cos_s(0x1000 - 0x4000) * scale;
    let cam = p.input_dir_yaw();
    let pos = Vec3::new(0.0 - eng_math::cos_s(cam) * across, y, 0.0 + eng_math::sin_s(cam) * across);
    assert!(r.zero_one() < 0.1);
    let mirrored = (0x1000 * 2 - yaw as i32) as i16;
    assert_eq!(p.rand, r);
    let bolts = live(&p, EFFECT_SS_LIGHTNING);
    assert_eq!(bolts.iter().map(|(i, _)| *i).collect::<Vec<_>>(), [0, 1, 2]);
    // The inserted bolts: copies, no bolts left, life whole then updated this frame (4 to 3; with
    // no bolts they don't branch).
    for ((_, b), want_yaw) in bolts[1..].iter().zip([yaw, mirrored]) {
        assert_eq!((b.pos, b.life, b.regs[lightning::R_NUM_BOLTS], b.regs[lightning::R_YAW], b.regs[lightning::R_LIFESPAN]), (pos, 3, 0, want_yaw, 4));
    }
    // The first's later updates don't branch again (life + 1 isn't rLifespan).
    p.effect_ss_update_all();
    assert_eq!(live(&p, EFFECT_SS_LIGHTNING).len(), 3);
    // Drawn: the first bolt's third texture (rLifespan 4 - life 2 = index 2).
    assert_eq!(lightning::texture(&p.effect_ss.table[0]), 3);
}
