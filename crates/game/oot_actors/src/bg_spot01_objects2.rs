//! `Bg_Spot01_Objects2` (`ovl_Bg_Spot01_Objects2/z_bg_spot01_objects2.c`): Kakariko's set pieces,
//! by type: the posters of the potion shop (0), the shooting gallery (1) and the bazaar (2), the
//! unfinished shooting gallery (3, `object_spot01_matoyab`) and the finished one (4); the others
//! from `object_spot01_matoya`.
//!
//! Its profile's object is `gameplay_keep`: the init finds the type's object in the scene's
//! banks (`Object_GetSlot`; killed if it isn't there) and waits for it to load
//! (`func_808AC2BC`). Then the actor takes that object as its own and becomes a DynaPoly actor
//! (`DYNA_TRANSFORM_POS`): the finished gallery with `gKakarikoShootingGalleryCol`, the unfinished
//! one with `object_spot01_matoyab_col`, the posters with no collision. By day the unfinished
//! gallery spawns a carpenter as its child (`En_Daiku_Kakariko`, params `(path << 8) + 1`, at the
//! first point of path `path`, with its own rotation). From then on it draws its type's list
//! (`func_808AC4A4`, `Gfx_DrawDListOpa`).
//!
//! The whole overlay is ported. Its destroy deletes no bg actor (`BgSpot01Objects2_Destroy` is
//! empty), so a killed gallery would leave its collision behind; it never kills itself once it
//! has one, and the scene's end clears the DynaPoly context.
//!
//! Params: `type` bits 0 to 2 (0 to 2: a poster, 3: the unfinished gallery, 4: the finished
//! gallery; 5 to 7 have no object, `objectId` left 0, and are killed); `path` bits 8 to 15 (type
//! 3: the carpenter's path).
//! Flags: none.

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_TRANSFORM_POS};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_SPOT01_OBJECTS2` (`actor_table.h`: 0x019D).
pub const ACTOR_BG_SPOT01_OBJECTS2: i16 = 0x019D;
/// `ACTOR_EN_DAIKU_KAKARIKO` (`actor_table.h`: 0x01BC).
pub const ACTOR_EN_DAIKU_KAKARIKO: i16 = 0x01BC;
/// `OBJECT_GAMEPLAY_KEEP` (`object_table.h`: 0x0001), the profile's.
pub const OBJECT: &str = "gameplay_keep";
/// `OBJECT_SPOT01_MATOYA`, `OBJECT_SPOT01_MATOYAB` (`object_table.h`: 0x0180, 0x0181).
pub const OBJECT_SPOT01_MATOYA: i16 = 0x0180;
pub const OBJECT_SPOT01_MATOYAB: i16 = 0x0181;

/// `Bg_Spot01_Objects2_Profile`: no draw until the object is in.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT01_OBJECTS2, name: "Bg_Spot01_Objects2", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT };

/// The types (`PARAMS_GET_U(params, 0, 3)`).
pub const TYPE_POTION_SHOP_POSTER: i16 = 0;
pub const TYPE_SHOOTING_GALLERY_POSTER: i16 = 1;
pub const TYPE_BAZAAR_POSTER: i16 = 2;
pub const TYPE_SHOOTING_GALLERY_UNFINISHED: i16 = 3;
pub const TYPE_SHOOTING_GALLERY: i16 = 4;

/// `D_808AC510`: the type's display list, and the object file it's in.
const D_808AC510: [(&str, &str); 5] = [
    ("object_spot01_matoya", "gKakarikoPotionShopSignDL"),
    ("object_spot01_matoya", "gKakarikoShootingGallerySignDL"),
    ("object_spot01_matoya", "gKakarikoBazaarSignDL"),
    ("object_spot01_matoyab", "gKakarikoConstructionSiteDL"),
    ("object_spot01_matoya", "gKakarikoShootingGalleryDL"),
];

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_808AC2BC`: waiting for the object.
    WaitForObject,
    /// `func_808AC474`: nothing.
    Idle,
}

pub struct BgSpot01Objects2 {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    /// `objectId`.
    pub object_id: i16,
    /// `requiredObjectSlot`.
    pub required_object_slot: Option<usize>,
    /// `dyna.actor.draw` set (`func_808AC4A4`).
    pub drawn: bool,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

fn load_collision(play: &PlayState, file: &str, symbol: &str) -> anyhow::Result<Arc<CollisionHeader>> {
    let assets = play.assets.as_ref().ok_or_else(|| anyhow::anyhow!("no asset pack"))?;
    Ok(Arc::new(assets.pack.collision(&keys::collision(file, symbol))?))
}

impl BgSpot01Objects2 {
    /// `PARAMS_GET_U(params, 0, 3)`.
    pub fn ty(&self) -> i16 {
        self.actor.params & 7
    }

    /// `PARAMS_GET_U((s32)params, 8, 8)`: the carpenter's path.
    pub fn path(&self) -> i16 {
        (self.actor.params >> 8) & 0xFF
    }

    /// `BgSpot01Objects2_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgSpot01Objects2 { actor, bg: BG_ACTOR_MAX, action: Action::WaitForObject, object_id: 0, required_object_slot: None, drawn: false };
        match this.ty() {
            0..=2 => this.object_id = OBJECT_SPOT01_MATOYA,
            3 => this.object_id = OBJECT_SPOT01_MATOYAB,
            4 => this.object_id = OBJECT_SPOT01_MATOYA,
            // objectId stays 0 (the zeroed instance): no bank holds it.
            _ => {}
        }
        if this.object_id >= 0 {
            this.required_object_slot = play.object_ctx.get_index(this.object_id);
            if this.required_object_slot.is_none() {
                log::debug!("Bg_Spot01_Objects2: ----------------------------- There was no bank setting.");
                this.actor.kill();
                return Box::new(this);
            }
        } else {
            this.actor.kill();
        }
        this.action = Action::WaitForObject;
        // sInitChain: cullingVolumeDistance 12800, cullingVolumeScale 2000,
        // cullingVolumeDownward 1500, scale 0.1.
        this.actor.culling_volume_distance = 12800.0;
        this.actor.culling_volume_scale = 2000.0;
        this.actor.culling_volume_downward = 1500.0;
        this.actor.scale = Vec3::splat(0.1);
        Box::new(this)
    }

    /// `func_808AC22C`: point `waypoint` of path `path`.
    fn func_808ac22c(play: &PlayState, path: i16, waypoint: usize) -> Option<Vec3> {
        play.setup_path_list().get(path as usize).map(|p| p.point(waypoint))
    }

    /// `func_808AC2BC`: once the object is loaded, the collision by type, by day the carpenter,
    /// and the draw.
    fn func_808ac2bc(&mut self, play: &mut PlayState) {
        let Some(slot) = self.required_object_slot else { return };
        if !play.object_ctx.is_loaded(slot) {
            return;
        }
        log::debug!("Bg_Spot01_Objects2: ----- Successful bank switching!!");
        self.actor.obj_bank_index = Some(slot);
        // DynaPolyActor_Init(&this->dyna, DYNA_TRANSFORM_POS).
        match self.ty() {
            TYPE_SHOOTING_GALLERY => match load_collision(play, "object_spot01_matoya", "gKakarikoShootingGalleryCol") {
                Ok(h) => self.bg = play.col.dyna.set_bg_actor(h, source(&self.actor), DYNA_TRANSFORM_POS),
                Err(e) => log::error!("Bg_Spot01_Objects2: {e:#}"),
            },
            TYPE_SHOOTING_GALLERY_UNFINISHED => {
                match load_collision(play, "object_spot01_matoyab", "object_spot01_matoyab_col") {
                    Ok(h) => self.bg = play.col.dyna.set_bg_actor(h, source(&self.actor), DYNA_TRANSFORM_POS),
                    Err(e) => log::error!("Bg_Spot01_Objects2: {e:#}"),
                }
                if play.save.is_day() {
                    let path = self.path();
                    match Self::func_808ac22c(play, path, 0) {
                        Some(position) => {
                            let r = self.actor.world_rot;
                            let params = ((path as i32) << 8) + 1;
                            if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_DAIKU_KAKARIKO, position, [r.x, r.y, r.z], params as i16) {
                                log::debug!("Bg_Spot01_Objects2: En_Daiku_Kakariko not spawned: {e:?}");
                            }
                        }
                        // The C reads past the scene's path list.
                        None => log::warn!("Bg_Spot01_Objects2: the scene has no path {path}"),
                    }
                }
            }
            // The posters: no collision.
            _ => {}
        }
        self.drawn = true;
        self.action = Action::Idle;
    }
}

impl ActorImpl for BgSpot01Objects2 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgSpot01Objects2_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::WaitForObject => self.func_808ac2bc(play),
            // func_808AC474.
            Action::Idle => {}
        }
    }
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.drawn as u32];
        rs
    }
    /// `func_808AC4A4`: `Gfx_DrawDListOpa(play, D_808AC510[type])`; nothing while `draw` is
    /// `NULL`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.first() != Some(&1) {
            return;
        }
        // Types 5 to 7 would read past D_808AC510; they never get here.
        if let Some((file, dl)) = D_808AC510.get(self.ty() as usize) {
            crate::gfx_draw_dlist_opa(out, file, dl, rs);
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
