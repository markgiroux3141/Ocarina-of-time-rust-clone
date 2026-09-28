//! Actors ported from the decomp's overlays (`src/overlays/actors`), one module per overlay:
//! Player (`ovl_player_actor`, `z_player.c`), `En_Holl` and `Bg_Ydan_Hasi`, plus the
//! sandbox's dummy Z-target. Each implements `oot_game::actor_ctx::ActorImpl` and lives in
//! `PlayState`'s actor context.
//!
//! - `overlays()` gives `Actor_Spawn` the constructors of the actors spawned by id (Player and
//!   `En_Holl`; every other id spawns a placeholder), and `play_entrance` enters a scene from
//!   the pack with them (`Play_Init`).
//! - `new_play` sets up a `PlayState` on any collision with Player standing in it (the tests'
//!   and the sandbox's).
//! - `PlayExt` gives typed access to the actors the tests and the apps look at.

use std::sync::Arc;

use eng_collision::bgcheck::CollisionContext;
use eng_collision::collision::CollisionHeader;
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTOR_BG_YDAN_HASI, ACTOR_SANDBOX_DUMMY_TARGET, ACTORCAT_BG, ACTORCAT_ENEMY, ActorHandle, ActorProfile};
use oot_game::data::GameData;
use oot_game::pack::GamePack;
use oot_game::play::PlayState;
use oot_game::play_scene::GameAssets;
use oot_game::player_lib::PlayerRules;
use oot_game::save::SaveContext;
use oot_game::spawn::Overlays;

pub mod bg_ydan_hasi;
pub mod dummy_target;
pub mod en_holl;
pub mod player;
pub mod script;

use bg_ydan_hasi::BgYdanHasi;
use dummy_target::DummyTarget;
use player::Player;

/// The profiles of the actors this crate ports.
pub const PROFILES: &[ActorProfile] = &[player::PROFILE, en_holl::PROFILE, bg_ydan_hasi::PROFILE, dummy_target::PROFILE];

/// The constructors `Actor_Spawn` uses for ids this crate ports. (`Bg_Ydan_Hasi`'s init isn't:
/// only the floating block the sandbox builds directly.)
pub fn overlays() -> Overlays {
    let mut o = Overlays::default();
    o.register(oot_game::actor_ctx::ACTOR_PLAYER, Player::init);
    o.register(en_holl::ACTOR_EN_HOLL, en_holl::EnHoll::init);
    o
}

/// The game's assets for play from `pack`, with this crate's actors.
pub fn game_assets(pack: GamePack) -> anyhow::Result<Arc<GameAssets>> {
    Ok(Arc::new(GameAssets::load(pack, overlays())?))
}

/// `Play_Init` for `save.entrance_index` (see `oot_game::play_scene`).
pub fn play_entrance(assets: Arc<GameAssets>, data: Arc<GameData>, rules: Arc<PlayerRules>, save: SaveContext) -> anyhow::Result<PlayState> {
    PlayState::play_init(assets, data, rules, save)
}

/// A play state on `col` with Player (adult or child) standing at `pos`, facing `yaw`, and
/// both cameras behind it.
pub fn new_play(data: Arc<GameData>, rules: Arc<PlayerRules>, col: CollisionContext, adult: bool, pos: Vec3, yaw: i16) -> PlayState {
    let mut play = PlayState::new(data.clone(), rules, col, (pos, yaw), adult);
    play.spawn(Box::new(Player::new(&data, adult, pos, yaw)));
    play.respawn_player = Some(respawn_player);
    play.reset_cameras();
    play.reset_blending();
    play
}

/// Rebuilds Player at the play state's spawn, keeping its age (the sandbox's void-out).
fn respawn_player(play: &mut PlayState) {
    let (pos, yaw) = play.spawn;
    let data = play.data.clone();
    if let Some(p) = play.player.and_then(|h| play.actors.downcast_mut::<Player>(h)) {
        let adult = p.adult;
        *p = Player::new(&data, adult, pos, yaw);
    }
}

/// Typed access to the ported actors in a play state.
pub trait PlayExt {
    /// `GET_PLAYER(play)`.
    fn player(&self) -> &Player;
    fn player_mut(&mut self) -> &mut Player;
    /// The dummy targets, oldest first.
    fn targets(&self) -> Vec<ActorHandle>;
    fn target(&self, i: usize) -> &Actor;
    fn target_mut(&mut self, i: usize) -> &mut Actor;
    /// The `Bg_Ydan_Hasi` platforms, oldest first.
    fn platforms(&self) -> Vec<ActorHandle>;
    fn platform(&self, i: usize) -> &BgYdanHasi;
    /// Adds a dummy Z-target standing at `pos`.
    fn spawn_target(&mut self, pos: Vec3) -> ActorHandle;
    /// Adds a `Bg_Ydan_Hasi` floating block with `header` (`gDTSlidingPlatformCol`) at `home`,
    /// floating on `water_surface`, with its collision built for the next frame.
    fn spawn_platform(&mut self, header: Arc<CollisionHeader>, home: Vec3, yaw: i16, water_surface: f32) -> ActorHandle;
}

impl PlayExt for PlayState {
    fn player(&self) -> &Player {
        self.player.and_then(|h| self.actors.downcast::<Player>(h)).expect("Player")
    }
    fn player_mut(&mut self) -> &mut Player {
        let h = self.player.expect("Player");
        self.actors.downcast_mut::<Player>(h).expect("Player")
    }
    fn targets(&self) -> Vec<ActorHandle> {
        let mut v: Vec<ActorHandle> = self.actors.category(ACTORCAT_ENEMY).iter().copied().filter(|&h| self.actors.actor(h).is_some_and(|a| a.id == ACTOR_SANDBOX_DUMMY_TARGET)).collect();
        v.reverse();
        v
    }
    fn target(&self, i: usize) -> &Actor {
        self.actors.actor(self.targets()[i]).unwrap()
    }
    fn target_mut(&mut self, i: usize) -> &mut Actor {
        let h = self.targets()[i];
        self.actors.actor_mut(h).unwrap()
    }
    fn platforms(&self) -> Vec<ActorHandle> {
        let mut v: Vec<ActorHandle> = self.actors.category(ACTORCAT_BG).iter().copied().filter(|&h| self.actors.actor(h).is_some_and(|a| a.id == ACTOR_BG_YDAN_HASI)).collect();
        v.reverse();
        v
    }
    fn platform(&self, i: usize) -> &BgYdanHasi {
        self.actors.downcast::<BgYdanHasi>(self.platforms()[i]).unwrap()
    }
    fn spawn_target(&mut self, pos: Vec3) -> ActorHandle {
        let h = self.spawn(Box::new(DummyTarget::new(pos))).expect("spawn");
        self.reset_blending();
        h
    }
    fn spawn_platform(&mut self, header: Arc<CollisionHeader>, home: Vec3, yaw: i16, water_surface: f32) -> ActorHandle {
        let p = BgYdanHasi::new(&mut self.col.dyna, header, home, yaw, water_surface);
        let h = self.spawn(Box::new(p)).expect("spawn");
        self.col.dyna.update_context();
        self.reset_blending();
        h
    }
}
