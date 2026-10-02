//! `En_Tana` (`ovl_En_Tana/z_en_tana.c`): a shop's shelves. `params` is the kind: 0 wooden
//! (the Kokiri shop's), 1 and 2 stone, with `object_shop_dungen`'s two stone textures on
//! segment 8. They do nothing but stand there: `En_Ossan` finds them (`Actor_Find`) and puts
//! its items on them (`En_GirlA`).
//!
//! `EnTana_Init`'s print of `sShelfTypes[params]` reads past its two names for kind 2
//! (`@bug (game)`, a print only: not ported).

use eng_gfx::{DrawCmd, MeshKey};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_EN_TANA` (`actor_table.h`).
pub const ACTOR_EN_TANA: i16 = 0x00C2;

/// `En_Tana_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_TANA, name: "En_Tana", category: ACTORCAT_PROP, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: "object_shop_dungen" };

/// `sShelfDLists`.
const SHELF_DLISTS: [&str; 3] = ["gShopDungenWoodenShelvesDL", "gShopDungenStoneShelvesDL", "gShopDungenStoneShelvesDL"];
/// `sStoneTextures` (kind 0 has none), on segment 8.
const STONE_TEXTURES: [Option<&str>; 3] = [None, Some("gShopDungenStone1Tex"), Some("gShopDungenStone2Tex")];
const SEG_STONE: u8 = 0x08;

fn stone_bake(kind: usize) -> String {
    format!("En_Tana/stone{kind}")
}

/// `EnTana_DrawStoneShelves`' meshes: the stone shelves with each stone texture. (The wooden
/// ones are their display list, `Gfx_SetupDL_25Opa` and all, as the pack has it.)
pub fn bakes() -> Vec<MeshBake> {
    (1..3)
        .map(|kind| MeshBake {
            name: stone_bake(kind),
            object: "object_shop_dungen".into(),
            segments: vec![(SEG_STONE, BakeSegment::Texture { file: "object_shop_dungen".into(), symbol: STONE_TEXTURES[kind].unwrap().into() })],
            prelude: Vec::new(),
            body: BakeBody::DLists(vec![("object_shop_dungen".into(), SHELF_DLISTS[kind].into())]),
        })
        .collect()
}

pub struct EnTana {
    pub actor: Actor,
}

impl EnTana {
    /// `EnTana_Init`.
    pub fn init(mut actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.scale = glam::Vec3::splat(1.0);
        actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        Box::new(EnTana { actor })
    }

    fn kind(&self) -> usize {
        (self.actor.params as usize).min(2)
    }
}

impl ActorImpl for EnTana {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnTana_Update`: nothing.
    fn update(&mut self, _play: &mut PlayState) {}
    /// `EnTana_DrawWoodenShelves` / `EnTana_DrawStoneShelves` (`sDrawFuncs[params]`).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let kind = self.kind();
        if kind == 0 {
            crate::gfx_draw_dlist_opa(out, "object_shop_dungen", SHELF_DLISTS[0], rs);
        } else {
            out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(&stone_bake(kind))), actor_draw_matrix(rs)));
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
