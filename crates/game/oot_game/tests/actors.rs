//! The actor system (`oot_game::actor_ctx`, `PlayState`'s `Actor_UpdateAll`) against the rules
//! of `z_actor.c`: generational handles, category lists newest first (`Actor_AddToCategory`),
//! the category order of `Actor_UpdateAll`, an actor taken out of its slot while it updates,
//! spawns waiting for the next frame, `Actor_Kill` then deletion, `freezeTimer` (`DECR`), and
//! render blending. The `PlayState` tests need the asset pack (for `GameData`); the arena tests
//! don't.

use std::sync::{Arc, Mutex, OnceLock};

use eng_collision::bgcheck::CollisionContext;
use eng_collision::collision::CollisionBuilder;
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_ENEMY, ACTORCAT_NPC, ACTORCAT_SWITCH, ActorContext, ActorHandle, ActorImpl};
use oot_game::data::GameData;
use oot_game::pack::GamePack;
use oot_game::play::{PlayState, RenderFrame, RenderState, scripted_input};
use oot_game::player_lib::PlayerRules;

type Log = Arc<Mutex<Vec<String>>>;

/// What a test actor does in its update.
#[derive(Clone)]
enum Does {
    Nothing,
    /// Spawns an actor of this category the first time it updates.
    Spawn(usize),
    /// Kills this actor the first time it updates.
    Kill(ActorHandle),
    /// Looks itself up (finds nothing: it's taken out) and moves this other actor.
    Push(ActorHandle),
}

struct Probe {
    actor: Actor,
    tag: String,
    log: Log,
    does: Does,
    me: Option<ActorHandle>,
}

impl Probe {
    fn boxed(tag: &str, cat: usize, log: &Log, does: Does) -> Box<Probe> {
        let mut actor = Actor::new(Vec3::ZERO, 0);
        actor.category = cat;
        Box::new(Probe { actor, tag: tag.into(), log: log.clone(), does, me: None })
    }
}

impl ActorImpl for Probe {
    fn name(&self) -> &'static str {
        "Probe"
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    fn update(&mut self, play: &mut PlayState) {
        self.log.lock().unwrap().push(format!("{}@{}", self.tag, play.gameplay_frames));
        match std::mem::replace(&mut self.does, Does::Nothing) {
            Does::Nothing => {}
            Does::Spawn(cat) => {
                play.spawn(Probe::boxed(&format!("{}-child", self.tag), cat, &self.log, Does::Nothing));
            }
            Does::Kill(h) => {
                play.actors.actor_mut(h).unwrap().kill();
            }
            Does::Push(h) => {
                assert!(self.me.is_some_and(|me| play.actors.get(me).is_none()), "an updating actor isn't in its slot");
                assert!(self.me.is_some_and(|me| play.actors.exists(me)), "but its handle stays valid");
                play.actors.actor_mut(h).unwrap().world_pos.x += 10.0;
            }
        }
    }
    fn destroy(&mut self, _play: &mut PlayState) {
        self.log.lock().unwrap().push(format!("{} destroyed", self.tag));
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[test]
fn handles_are_generational() {
    let log: Log = Default::default();
    let mut ctx = ActorContext::default();
    let a = ctx.insert(Probe::boxed("a", ACTORCAT_NPC, &log, Does::Nothing)).unwrap();
    assert!(ctx.exists(a));
    ctx.remove(a);
    assert!(!ctx.exists(a) && ctx.get(a).is_none());
    // The slot is reused, under a new generation.
    let b = ctx.insert(Probe::boxed("b", ACTORCAT_NPC, &log, Does::Nothing)).unwrap();
    assert!(ctx.get(a).is_none(), "the old handle doesn't reach the new actor");
    assert_eq!(ctx.downcast::<Probe>(b).unwrap().tag, "b");
    assert_eq!(ctx.total(), 1);
}

#[test]
fn categories_are_newest_first_in_category_order() {
    let log: Log = Default::default();
    let mut ctx = ActorContext::default();
    let n1 = ctx.insert(Probe::boxed("n1", ACTORCAT_NPC, &log, Does::Nothing)).unwrap();
    let s1 = ctx.insert(Probe::boxed("s1", ACTORCAT_SWITCH, &log, Does::Nothing)).unwrap();
    let n2 = ctx.insert(Probe::boxed("n2", ACTORCAT_NPC, &log, Does::Nothing)).unwrap();
    // Actor_AddToCategory: at the head.
    assert_eq!(ctx.category(ACTORCAT_NPC), &[n2, n1]);
    assert_eq!(ctx.all(), vec![s1, n2, n1]);
    // Taking one out doesn't change the lists; putting it back restores access.
    let a = ctx.take(n1).unwrap();
    assert!(ctx.get(n1).is_none() && ctx.exists(n1));
    ctx.put_back(n1, a);
    assert!(ctx.get(n1).is_some());
}

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

fn play() -> Option<PlayState> {
    let c = ctx()?;
    let mut b = CollisionBuilder::new();
    let s = b.surface(0, 0);
    b.quad(Vec3::new(-100.0, 0.0, 100.0), Vec3::new(100.0, 0.0, 100.0), Vec3::new(100.0, 0.0, -100.0), Vec3::new(-100.0, 0.0, -100.0), s);
    Some(PlayState::new(c.data.clone(), c.rules.clone(), CollisionContext::new(b.finish()), (Vec3::ZERO, 0), true))
}

fn frame(p: &mut PlayState) {
    p.tick_with(scripted_input(Default::default(), Default::default()));
}

#[test]
fn update_all_runs_categories_in_order_and_spawns_wait_a_frame() {
    let Some(mut p) = play() else { return };
    let log: Log = Default::default();
    p.spawn(Probe::boxed("enemy", ACTORCAT_ENEMY, &log, Does::Nothing));
    p.spawn(Probe::boxed("npc", ACTORCAT_NPC, &log, Does::Spawn(ACTORCAT_NPC)));
    p.spawn(Probe::boxed("bg", ACTORCAT_BG, &log, Does::Nothing));
    frame(&mut p);
    // BG (1) before NPC (4) before ENEMY (5); the NPC spawned this frame waits.
    assert_eq!(*log.lock().unwrap(), vec!["bg@1", "npc@1", "enemy@1"]);
    log.lock().unwrap().clear();
    frame(&mut p);
    // Now the new actor updates, at the head of the NPC list.
    assert_eq!(*log.lock().unwrap(), vec!["bg@2", "npc-child@2", "npc@2", "enemy@2"]);
}

#[test]
fn a_killed_actor_is_deleted_when_update_all_reaches_it() {
    let Some(mut p) = play() else { return };
    let log: Log = Default::default();
    let victim = p.spawn(Probe::boxed("victim", ACTORCAT_ENEMY, &log, Does::Nothing)).unwrap();
    p.spawn(Probe::boxed("killer", ACTORCAT_SWITCH, &log, Does::Kill(victim)));
    frame(&mut p);
    // The switch category comes first: Actor_Kill, then the enemy's turn deletes it (update is
    // NULL) instead of updating it.
    assert_eq!(*log.lock().unwrap(), vec!["killer@1", "victim destroyed"]);
    assert!(!p.actors.exists(victim));
    assert!(p.actors.category(ACTORCAT_ENEMY).is_empty());
}

#[test]
fn an_updating_actor_is_taken_out_and_reaches_the_others() {
    let Some(mut p) = play() else { return };
    let log: Log = Default::default();
    let other = p.spawn(Probe::boxed("other", ACTORCAT_NPC, &log, Does::Nothing)).unwrap();
    let me = p.spawn(Probe::boxed("pusher", ACTORCAT_SWITCH, &log, Does::Push(other))).unwrap();
    p.actors.downcast_mut::<Probe>(me).unwrap().me = Some(me);
    frame(&mut p);
    assert_eq!(p.actors.actor(other).unwrap().world_pos.x, 10.0);
    assert!(p.actors.get(me).is_some(), "put back after its update");
}

#[test]
fn freeze_timer_skips_updates_like_decr() {
    let Some(mut p) = play() else { return };
    let log: Log = Default::default();
    let h = p.spawn(Probe::boxed("frozen", ACTORCAT_NPC, &log, Does::Nothing)).unwrap();
    // DECR(freezeTimer) == 0: with 3, frames 1 and 2 are skipped (3 → 2 → 1), frame 3 updates (→ 0).
    p.actors.actor_mut(h).unwrap().freeze_timer = 3;
    for _ in 0..4 {
        frame(&mut p);
    }
    assert_eq!(*log.lock().unwrap(), vec!["frozen@3", "frozen@4"]);
}

#[test]
fn render_states_blend_short_way_and_teleports_dont() {
    let a = RenderState { pos: Vec3::ZERO, rot: [0, 0x7F00, 0], angles: vec![-0x7F00], values: vec![0.0], switches: vec![1], ..Default::default() };
    let b = RenderState { pos: Vec3::new(10.0, 0.0, 0.0), rot: [0, -0x7F00, 0], angles: vec![0x7F00], values: vec![4.0], switches: vec![2], ..Default::default() };
    let m = a.lerp(&b, 0.25);
    assert_eq!(m.pos.x, 2.5);
    // 0x7F00 → -0x7F00 is +0x200 the short way.
    assert_eq!(m.rot[1], 0x7F80);
    assert_eq!(m.angles[0], -0x7F80);
    assert_eq!(m.values[0], 1.0);
    assert_eq!(m.switches, vec![1], "switches flip halfway");
    assert_eq!(a.lerp(&b, 0.5).switches, vec![2]);
    let t = RenderState { teleported: true, ..b.clone() };
    assert_eq!(a.lerp(&t, 0.25).pos, b.pos, "no blending into a teleport");
    // An actor that wasn't in the previous frame isn't blended either.
    let Some(mut p) = play() else { return };
    let log: Log = Default::default();
    p.spawn(Probe::boxed("x", ACTORCAT_NPC, &log, Does::Nothing));
    let before: RenderFrame = p.current_frame();
    let h = p.spawn(Probe::boxed("new", ACTORCAT_NPC, &log, Does::Nothing)).unwrap();
    frame(&mut p);
    let after = p.current_frame();
    assert!(before.actor(h).is_none() && after.lerp(&after, 0.5).actor(h).is_some());
    assert_eq!(before.lerp(&after, 0.5).actor(h).unwrap().pos, after.actor(h).unwrap().pos);
}
