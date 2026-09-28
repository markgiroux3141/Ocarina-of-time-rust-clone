//! The sandbox's dummy Z-target: not a game actor, a stand-in enemy to lock on to. It's
//! targetable and hostile (`ACTOR_FLAG_0 | ACTOR_FLAG_2`), `targetMode` 3 (a 350 range), with
//! its focus 40 above its feet, and it doesn't move. It draws as a box.

use eng_gfx::{DrawCmd, MeshKey};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_0, ACTOR_FLAG_2, Actor};
use oot_game::actor_ctx::{ACTOR_SANDBOX_DUMMY_TARGET, ACTORCAT_ENEMY, ActorImpl, ActorProfile};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_SANDBOX_DUMMY_TARGET,
    name: "Sandbox_Dummy_Target",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_0 | ACTOR_FLAG_2,
    object: "gameplay_keep",
};

#[derive(Debug, Clone)]
pub struct DummyTarget {
    pub actor: Actor,
}

impl DummyTarget {
    /// A dummy enemy standing at `pos`.
    pub fn new(pos: Vec3) -> DummyTarget {
        let mut actor = Actor::new(pos, 0);
        PROFILE.apply(&mut actor);
        actor.focus_pos = pos + Vec3::Y * 40.0;
        actor.target_mode = 3;
        DummyTarget { actor }
    }
}

impl ActorImpl for DummyTarget {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    fn update(&mut self, _play: &mut PlayState) {}
    fn draw(&self, st: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        out.opa.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::TARGET_BOX), Mat4::from_translation(st.pos)));
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
