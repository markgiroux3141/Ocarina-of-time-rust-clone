//! Actors ported from the decomp's overlays (`src/overlays/actors`), one module per overlay:
//! Player (`ovl_player_actor`, `z_player.c`), `En_Holl`, `Bg_Ydan_Hasi`, `Bg_Treemouth` and
//! Kokiri Forest's props and NPCs, plus the sandbox's dummy Z-target. Each implements
//! `oot_game::actor_ctx::ActorImpl` and lives in `PlayState`'s actor context.
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

pub mod bg_treemouth;
pub mod bg_ydan_hasi;
pub mod demo_tre_lgt;
pub mod dummy_target;
pub mod en_box;
pub mod en_door;
pub mod en_elf;
pub mod en_girla;
pub mod en_goroiwa;
pub mod en_holl;
pub mod en_item00;
pub mod en_ishi;
pub mod en_kanban;
pub mod en_ko;
pub mod en_kusa;
pub mod en_md;
pub mod en_ossan;
pub mod en_river_sound;
pub mod en_tana;
pub mod en_wonder_item;
pub mod en_wonder_talk2;
pub mod obj_hana;
pub mod player;
pub mod playthrough;
pub mod script;

use bg_ydan_hasi::BgYdanHasi;
use dummy_target::DummyTarget;
use player::Player;

/// The profiles of the actors this crate ports.
pub const PROFILES: &[ActorProfile] =
    &[player::PROFILE, en_holl::PROFILE, bg_ydan_hasi::PROFILE, dummy_target::PROFILE, obj_hana::PROFILE, en_ishi::PROFILE, en_kusa::PROFILE, en_kanban::PROFILE, en_ko::PROFILE, en_door::PROFILE, en_wonder_talk2::PROFILE, en_item00::PROFILE, bg_treemouth::PROFILE, en_box::PROFILE, en_wonder_item::PROFILE, en_goroiwa::PROFILE, en_md::PROFILE, en_ossan::PROFILE, en_girla::PROFILE, en_tana::PROFILE, en_elf::PROFILE, en_river_sound::PROFILE, demo_tre_lgt::PROFILE];

/// The constructors `Actor_Spawn` uses for ids this crate ports. (`Bg_Ydan_Hasi`'s init isn't:
/// only the floating block the sandbox builds directly.)
pub fn overlays() -> Overlays {
    let mut o = Overlays::default();
    o.register(oot_game::actor_ctx::ACTOR_PLAYER, Player::init);
    o.register(en_holl::ACTOR_EN_HOLL, en_holl::EnHoll::init);
    o.register(obj_hana::ACTOR_OBJ_HANA, obj_hana::ObjHana::init);
    o.register(en_ishi::ACTOR_EN_ISHI, en_ishi::EnIshi::init);
    o.register(en_kusa::ACTOR_EN_KUSA, en_kusa::EnKusa::init);
    o.register(en_kanban::ACTOR_EN_KANBAN, en_kanban::EnKanban::init);
    o.register(en_ko::ACTOR_EN_KO, en_ko::EnKo::init);
    o.register(en_door::ACTOR_EN_DOOR, en_door::EnDoor::init);
    o.register(en_wonder_talk2::ACTOR_EN_WONDER_TALK2, en_wonder_talk2::EnWonderTalk2::init);
    o.register(en_item00::ACTOR_EN_ITEM00, en_item00::EnItem00::init);
    o.register(bg_treemouth::ACTOR_BG_TREEMOUTH, bg_treemouth::BgTreemouth::init);
    o.register(en_box::ACTOR_EN_BOX, en_box::EnBox::init);
    o.register(en_wonder_item::ACTOR_EN_WONDER_ITEM, en_wonder_item::EnWonderItem::init);
    o.register(en_goroiwa::ACTOR_EN_GOROIWA, en_goroiwa::EnGoroiwa::init);
    o.register(en_md::ACTOR_EN_MD, en_md::EnMd::init);
    o.register(en_ossan::ACTOR_EN_OSSAN, en_ossan::EnOssan::init);
    o.register(en_girla::ACTOR_EN_GIRLA, en_girla::EnGirlA::init);
    o.register(en_tana::ACTOR_EN_TANA, en_tana::EnTana::init);
    o.register(en_elf::ACTOR_EN_ELF, en_elf::EnElf::init);
    o.register(en_river_sound::ACTOR_EN_RIVER_SOUND, en_river_sound::EnRiverSound::init);
    o.register(demo_tre_lgt::ACTOR_DEMO_TRE_LGT, demo_tre_lgt::DemoTreLgt::init);
    o
}

/// The meshes the ported actors need baked (docs/adr/0012-actor-bakes.md), for the importer.
pub fn bakes() -> Vec<oot_game::pack::MeshBake> {
    let mut v = en_kanban::bakes();
    v.extend(en_ko::bakes());
    v.extend(en_door::bakes());
    v.extend(en_item00::bakes());
    v.extend(bg_treemouth::bakes());
    v.extend(en_box::bakes());
    v.extend(en_md::bakes());
    v.extend(en_ossan::bakes());
    v.extend(en_tana::bakes());
    v.extend(en_elf::bakes());
    // z_actor.c's target reticle.
    v.extend(oot_game::target::bakes());
    // The message box's sprites (docs/adr/0017-interface-sprites.md).
    v.extend(oot_game::message::bakes().iter().map(|b| b.mesh_bake()));
    // The HUD's.
    v.extend(oot_game::interface::bakes().iter().map(|b| b.mesh_bake()));
    v
}

/// `Gfx_DrawDListOpa`: `file`'s display list `symbol` (baked with `Gfx_SetupDL_25Opa` in the
/// pack) at `Actor_Draw`'s model matrix.
pub(crate) fn gfx_draw_dlist_opa(out: &mut oot_game::play::DrawOut, file: &str, symbol: &str, rs: &oot_game::play::RenderState) {
    let m = oot_game::play::actor_draw_matrix(rs);
    out.opa.push(eng_gfx::DrawCmd::new(eng_gfx::MeshKey::named(oot_game::pack::keys::mesh(file, symbol)), m));
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
    /// Moves Player to `pos` facing `yaw`, standing, with the cameras behind it (a test or
    /// sandbox start somewhere other than the entrance's spawn).
    fn place_player(&mut self, pos: Vec3, yaw: i16);
    /// The pause menu's equipping, as a stand-in (the pause menu isn't ported):
    /// [`oot_game::save::SaveContext::equip_owned_unworn`], then `Player_SetEquipmentData` as the menu's
    /// closing runs it. Returns whether anything was equipped.
    fn equip_owned_unworn(&mut self) -> bool;
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
    fn place_player(&mut self, pos: Vec3, yaw: i16) {
        let data = self.data.clone();
        let p = self.player_mut();
        let a = &mut p.actor;
        a.world_pos = pos;
        a.home_pos = pos;
        a.prev_pos = pos;
        a.world_rot.y = yaw;
        a.shape_rot.y = yaw;
        a.teleported = true;
        p.current_yaw = yaw;
        p.fall_start_height = pos.y as i16;
        p.stand_still(&data);
        self.reset_cameras();
        self.reset_blending();
    }
    fn equip_owned_unworn(&mut self) -> bool {
        self.pause_menu_equip()
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
