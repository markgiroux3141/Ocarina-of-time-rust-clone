//! `Bg_Mjin` (`ovl_Bg_Mjin/z_bg_mjin.c`): a warp pad. `params` 0: the ocarina's pad
//! (`object_mjin_oka`); 1 to 6: a medallion's (`object_mjin`, with the medallion's platform
//! texture from `sObjectIds[params - 1]` on segment 8: the Temple of Time's is the Light
//! Medallion's).
//!
//! It waits for its object (`func_808A0850`), then is a DynaPoly actor of its collision
//! (`gWarpPadCol`, `gOcarinaWarpPadCol`) and drawn (`BgMjin_Draw`), from baked meshes
//! (docs/adr/0012-actor-bakes.md).
//!
//! Params: `params` whole (0: the ocarina's pad, `object_mjin_oka`; 1 to 6: a medallion's pad,
//! its texture `sObjectIds[params - 1]`: 1 the Light Medallion's, 2 Shadow, 3 Fire, 4 Water, 5
//! Spirit, 6 Forest).
//! Flags: none.

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, MeshKey};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_MJIN` (`actor_table.h`: 0x006E).
pub const ACTOR_BG_MJIN: i16 = 0x006E;

/// `Bg_Mjin_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_MJIN, name: "Bg_Mjin", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep", schema: &SCHEMA };

/// `thisx->params`: which pad (`sObjectIds[params - 1]` for a medallion's).
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[
        (0, "the ocarina's pad"),
        (1, "the Light Medallion's pad"),
        (2, "the Shadow Medallion's pad"),
        (3, "the Fire Medallion's pad"),
        (4, "the Water Medallion's pad"),
        (5, "the Spirit Medallion's pad"),
        (6, "the Forest Medallion's pad"),
    ]),
    "which warp pad",
);

/// Bg_Mjin's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false };

/// `OBJECT_MJIN`, `OBJECT_MJIN_OKA` (`object_table.h`: 0x0061, 0x0068).
const OBJECT_MJIN: i16 = 0x0061;
const OBJECT_MJIN_OKA: i16 = 0x0068;
/// `sObjectIds`: the medallions' objects (`OBJECT_MJIN_FLASH` 0x0062 to `OBJECT_MJIN_WIND`
/// 0x0067), their files and platform textures.
const OBJECT_IDS: [(i16, &str, &str); 6] = [
    (0x0062, "object_mjin_flash", "gLightMedallionPlatformTex"),
    (0x0063, "object_mjin_dark", "gShadowMedallionPlatformTex"),
    (0x0064, "object_mjin_flame", "gFireMedallionPlatformTex"),
    (0x0065, "object_mjin_ice", "gWaterMedallionPlatformTex"),
    (0x0066, "object_mjin_soul", "gSpiritMedallionPlatformTex"),
    (0x0067, "object_mjin_wind", "gForestMedallionPlatformTex"),
];

/// The ocarina's pad (`gOcarinaWarpPadDL`).
pub const OKA_BAKE: &str = "Bg_Mjin/oka";
/// A medallion's pad (`gWarpPadBaseDL`, the platform texture on segment 8), by `params - 1`.
pub fn bake_name(medallion: usize) -> String {
    format!("Bg_Mjin/pad{medallion}")
}
/// A medallion's pad drawn without its medallion's object: `D_06000000` read through
/// `object_mjin` (`gSegments[6]` left as the actor's object).
pub const NO_MEDALLION_BAKE: &str = "Bg_Mjin/pad_none";

/// The meshes `BgMjin_Draw` draws.
pub fn bakes() -> Vec<MeshBake> {
    let pad = |name: String, seg8: BakeSegment| MeshBake {
        name,
        object: "object_mjin".into(),
        segments: vec![(0x08, seg8)],
        prelude: Vec::new(),
        body: BakeBody::DLists(vec![("object_mjin".into(), "gWarpPadBaseDL".into())]),
    };
    let mut v = vec![MeshBake {
        name: OKA_BAKE.into(),
        object: "object_mjin_oka".into(),
        segments: Vec::new(),
        prelude: Vec::new(),
        body: BakeBody::DLists(vec![("object_mjin_oka".into(), "gOcarinaWarpPadDL".into())]),
    }];
    for (i, (_, file, tex)) in OBJECT_IDS.iter().enumerate() {
        v.push(pad(bake_name(i), BakeSegment::Texture { file: (*file).into(), symbol: (*tex).into() }));
    }
    v.push(pad(NO_MEDALLION_BAKE.into(), BakeSegment::File("object_mjin".into())));
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_808A0850`: waiting for the object.
    Func808A0850,
    DoNothing,
}

pub struct BgMjin {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    /// `requiredObjectSlot`.
    pub required_object_slot: Option<usize>,
    pub action: Action,
    /// `actor.draw` set (once the object is loaded).
    pub drawn: bool,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgMjin {
    /// `BgMjin_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgMjin { actor, bg: BG_ACTOR_MAX, required_object_slot: None, action: Action::Func808A0850, drawn: false };
        // sInitChain.
        this.actor.scale = Vec3::ONE;
        this.actor.culling_volume_distance = 4000.0;
        this.actor.culling_volume_scale = 400.0;
        this.actor.culling_volume_downward = 400.0;
        let object = if TYPE.get(this.actor.params) != 0 { OBJECT_MJIN } else { OBJECT_MJIN_OKA };
        this.required_object_slot = play.object_ctx.get_index(object);
        if this.required_object_slot.is_none() {
            this.actor.kill();
        }
        Box::new(this)
    }

    fn load_collision(play: &PlayState, params: i16) -> Option<Arc<CollisionHeader>> {
        let (object, symbol) = if params != 0 { ("object_mjin", "gWarpPadCol") } else { ("object_mjin_oka", "gOcarinaWarpPadCol") };
        crate::obj_kibako2::load_collision(play, object, symbol)
    }

    /// `func_808A0850`: once the object is loaded, the collision and the draw.
    fn func_808a0850(&mut self, play: &mut PlayState) {
        let Some(slot) = self.required_object_slot else { return };
        if play.object_ctx.is_loaded(slot) {
            self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            // Actor_SetObjectDependency.
            self.actor.obj_bank_index = Some(slot);
            // DynaPolyActor_Init(&this->dyna, 0).
            if let Some(h) = Self::load_collision(play, TYPE.get(self.actor.params)) {
                self.bg = play.col.dyna.set_bg_actor(h, source(&self.actor), 0);
            }
            self.action = Action::DoNothing;
            self.drawn = true;
        }
    }
}

impl ActorImpl for BgMjin {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgMjin_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Func808A0850 => self.func_808a0850(play),
            Action::DoNothing => {}
        }
    }
    /// `BgMjin_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `BgMjin_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if !self.drawn {
            return;
        }
        let bake = if TYPE.get(self.actor.params) != 0 {
            // gSegments[6] = the medallion's object when it's loaded; segment 8 its D_06000000.
            let medallion = (TYPE.get(self.actor.params) - 1) as usize;
            match OBJECT_IDS.get(medallion) {
                Some(&(id, _, _)) if play.object_ctx.get_index(id).is_some() => bake_name(medallion),
                _ => NO_MEDALLION_BAKE.into(),
            }
        } else {
            OKA_BAKE.into()
        };
        out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(&bake)), oot_game::play::actor_draw_matrix(rs)));
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
