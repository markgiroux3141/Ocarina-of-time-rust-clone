//! `Bg_Spot07_Taki` (`ovl_Bg_Spot07_Taki/z_bg_spot07_taki.c`): Zora's Domain's waterfalls, and for
//! an adult their ice (`object_spot07_object`). Params 0 is the big fall into the pool (room 1, at
//! the origin), 1 the small one by King Zora's throne (room 0).
//!
//! For a child it's only drawn: translucent (`Gfx_SetupDL_25Xlu`), env white at alpha 128, its
//! water scrolled by `play->gameplayFrames` on segments 8, 9 and 0xA (`Gfx_TwoTexScroll`) through
//! `object_spot07_object_DL_000460` (0) or `_DL_000BE0` (1). For an adult the falls are frozen:
//! a DynaPoly actor (`DYNA_TRANSFORM_POS`) with `object_spot07_object_Col_002590` (0) or `_0038FC`
//! (1), drawn opaque (`_DL_001CF0` or `_DL_003210`) and then translucent (`_DL_001F68` or
//! `_DL_0032D8`) under segment 8's scroll. It does nothing else (`BgSpot07Taki_DoNothing`).
//!
//! The whole overlay is ported, both ages (chosen by age at its init and draw, as the C does).
//! `@bug (game)`: the big fall's 64x64 texture on segment 9 is scrolled mod 128 (`& 0x7F`) where
//! 256 would wrap it whole, so its scroll jumps each time it wraps (fixed only in the PAL N64
//! builds; this is gc-eu-mq-dbg).
//!
//! Params: `type` (the whole params: 0 the pool's fall, 1 the throne's; anything else draws and
//! collides as 1).
//! Flags: none.

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_TRANSFORM_POS};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

/// `ACTOR_BG_SPOT07_TAKI` (`actor_table.h`: 0x0120).
pub const ACTOR_BG_SPOT07_TAKI: i16 = 0x0120;
/// `OBJECT_SPOT07_OBJECT` (`object_table.h`: 0x0113).
pub const OBJECT: &str = "object_spot07_object";

/// `Bg_Spot07_Taki_Profile`: `ACTORCAT_BG`, `FLAGS` (`ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_DRAW_CULLING_DISABLED`).
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_BG_SPOT07_TAKI,
    name: "Bg_Spot07_Taki",
    category: ACTORCAT_BG,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `dyna.actor.params`, whole: which fall.
pub const TYPE: Field = Field::whole("type", Kind::Named(&[(0, "the pool's big fall"), (1, "the small fall by King Zora's throne")]), "which waterfall (and for an adult which ice); not 0 draws as 1");

/// Bg_Spot07_Taki's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false, objects: &[], texts: &[] };

/// The adult's collision and opaque ice, by type (0, else 1).
const COLLISIONS: [&str; 2] = ["object_spot07_object_Col_002590", "object_spot07_object_Col_0038FC"];
const ADULT_OPA_DLS: [&str; 2] = ["object_spot07_object_DL_001CF0", "object_spot07_object_DL_003210"];
const ADULT_XLU_DLS: [&str; 2] = ["object_spot07_object_DL_001F68", "object_spot07_object_DL_0032D8"];
/// The child's falls, by type.
const CHILD_DLS: [&str; 2] = ["object_spot07_object_DL_000460", "object_spot07_object_DL_000BE0"];

const SEG_8: u8 = 0x08;
const SEG_9: u8 = 0x09;
const SEG_A: u8 = 0x0A;
/// The child's `gDPSetEnvColor(255, 255, 255, 128)` before the fall's list, as a prelude.
const SEG_ENV: u8 = 0x0E;
const ENV_128: (u32, u32) = (0xFB00_0000, 0xFFFF_FF80);

/// The bakes: the child's falls, the adult's translucent ice.
pub fn child_bake(ty: usize) -> String {
    format!("Bg_Spot07_Taki/child{ty}")
}
pub fn adult_xlu_bake(ty: usize) -> String {
    format!("Bg_Spot07_Taki/adult{ty}_xlu")
}

/// Segment 8's scroll (both ages): `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, (frames * -1) & 0x7F,
/// (frames * 1) & 0x7F, 32, 32, 1, (frames * 1) & 0x7F, (frames * 1) & 0x7F, 32, 32)`.
pub fn scroll_8(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, frames.wrapping_neg() & 0x7F, frames & 0x7F, 32, 32, 1, frames & 0x7F, frames & 0x7F, 32, 32)
}

/// Segment 9's scroll, by type: the big fall's 64x64 texture (`@bug (game)`: mod 128), the small
/// fall's.
pub fn scroll_9(ty: usize, frames: u32) -> Vec<(u32, u32)> {
    let f = frames;
    if ty == 0 {
        gfx_two_tex_scroll(0, f.wrapping_neg() & 0x7F, f.wrapping_mul(3).wrapping_neg() & 0xFF, 64, 64, 1, f & 0x7F, f.wrapping_mul(3).wrapping_neg() & 0xFF, 64, 64)
    } else {
        gfx_two_tex_scroll(0, 0, f.wrapping_neg() & 0x7F, 32, 32, 1, 0, f.wrapping_neg() & 0x7F, 32, 32)
    }
}

/// Segment 0xA's scroll (both types): `(frames * 3) & 0x1FF` down a 32x128 texture.
pub fn scroll_a(frames: u32) -> Vec<(u32, u32)> {
    let y = frames.wrapping_mul(3) & 0x1FF;
    gfx_two_tex_scroll(0, 0, y, 32, 128, 1, 0, y, 32, 128)
}

/// The meshes `BgSpot07Taki_Draw` draws beyond the plain lists: each child's fall (its scrolls,
/// the env colour) and each adult's translucent ice (segment 8's scroll).
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for ty in 0..2 {
        v.push(MeshBake {
            name: child_bake(ty),
            object: OBJECT.into(),
            segments: vec![
                (SEG_8, BakeSegment::Dynamic(scroll_8(0))),
                (SEG_9, BakeSegment::Dynamic(scroll_9(ty, 0))),
                (SEG_A, BakeSegment::Dynamic(scroll_a(0))),
                (SEG_ENV, BakeSegment::Commands(vec![ENV_128])),
            ],
            prelude: vec![SEG_ENV],
            body: BakeBody::DLists(vec![(OBJECT.into(), CHILD_DLS[ty].into())]),
        });
        v.push(MeshBake {
            name: adult_xlu_bake(ty),
            object: OBJECT.into(),
            segments: vec![(SEG_8, BakeSegment::Dynamic(scroll_8(0)))],
            prelude: Vec::new(),
            body: BakeBody::DLists(vec![(OBJECT.into(), ADULT_XLU_DLS[ty].into())]),
        });
    }
    v
}

pub struct BgSpot07Taki {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    /// `LINK_IS_ADULT` at its init (the draw reads the save's, the same in a scene).
    adult: bool,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgSpot07Taki {
    /// `params == 0` picks the first of each pair.
    fn index(&self) -> usize {
        if TYPE.get(self.actor.params) == 0 { 0 } else { 1 }
    }

    /// `BgSpot07Taki_Init`: `DynaPolyActor_Init(DYNA_TRANSFORM_POS)`, scale 0.1 (`sInitChain`); for
    /// an adult the ice's collision.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let adult = play.save.adult;
        let mut this = BgSpot07Taki { actor, bg: BG_ACTOR_MAX, adult };
        this.actor.scale = Vec3::splat(0.1);
        if adult {
            let col = COLLISIONS[this.index()];
            if let Some(h) = crate::obj_kibako2::load_collision(play, OBJECT, col) {
                this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), DYNA_TRANSFORM_POS);
            }
        }
        Box::new(this)
    }
}

impl ActorImpl for BgSpot07Taki {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgSpot07Taki_Update`: `BgSpot07Taki_DoNothing`.
    fn update(&mut self, _play: &mut PlayState) {}

    /// `BgSpot07Taki_Destroy`: `DynaPoly_DeleteBgActor` (the child's none).
    fn destroy(&mut self, play: &mut PlayState) {
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.delete_bg_actor(self.bg);
        }
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }

    /// `BgSpot07Taki_Draw`: an adult's ice opaque, then the translucent fall or ice under segment
    /// 8's scroll (a child's with 9 and 0xA too).
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let frames = play.gameplay_frames;
        let m = actor_draw_matrix(rs);
        let i = self.index();
        let mut sv = SegmentValues::default();
        sv.read(SEG_8, &scroll_8(frames));
        let key = if self.adult {
            out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, ADULT_OPA_DLS[i])), m));
            adult_xlu_bake(i)
        } else {
            sv.read(SEG_9, &scroll_9(i, frames));
            sv.read(SEG_A, &scroll_a(frames));
            child_bake(i)
        };
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(&key)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_big_fall_s_scroll_wraps_at_128() {
        // @bug (game): (frames * -1) & 0x7F on a 64x64 texture (256 units across): frame 128's
        // first tile is back at frame 0's across (its uls, bits 12 to 23 of gDPSetTileSize).
        let uls = |dl: &[(u32, u32)]| (dl[1].0 >> 12) & 0xFFF;
        assert_eq!(uls(&scroll_9(0, 128)), uls(&scroll_9(0, 0)));
        assert_eq!(uls(&scroll_9(0, 1)), 127);
        assert_eq!(bakes().len(), 4);
    }
}
