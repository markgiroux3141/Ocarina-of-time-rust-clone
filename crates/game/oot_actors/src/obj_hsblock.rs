//! `Obj_Hsblock` (`ovl_Obj_Hsblock/z_obj_hsblock.c`): a stone hookshot target, a DynaPoly actor of
//! `object_d_hsblock` (`DynaPolyActor_Init(0)`): the post (`gHookshotPostCol`, `gHookshotPostDL`,
//! types 0 and 1) or the flat target (`gHookshotTargetCol`, `gHookshotTargetDL`, type 2). Its
//! focus is 85 up for the posts, at its base for the target.
//!
//! Type 1 waits sunk 105 below its home until its switch flag is set (`func_80B93DF4`), then
//! rises (`func_80B93E5C`: its speed easing up to 16, the height easing home) and stays; with the
//! flag already set it starts home. Types 0 and 2 just stand. With the ice bit it spawns an ice
//! cover (`Obj_Ice_Poly`, params 1) as its child.
//!
//! Drawn with the env colour (165, 125, 55) in the Fire Temple, else white (the debug ROM reads
//! `mREG(13)` to `mREG(15)`, which its init sets to 255 and nothing else writes).
//!
//! The whole overlay is ported. `Obj_Ice_Poly` isn't (a placeholder, logged; no placement here has
//! the ice bit). Goron City's is room 3's 0xFFC0 at (1161, 480, -148), a post (the hookshot's, an
//! adult's: Phase 15).
//!
//! Params: `type` bits 0 and 1 (0 a post, 1 a post rising on its switch flag, 2 a target; 3 would
//! read past the C's tables: never placed); `ice` bit 5 (1: `Obj_Ice_Poly` spawned on it);
//! `switchFlag` bits 8 to 13 (`PARAMS_GET_U(params, 8, 6)`: type 1's; set, it's up); bits 6 and 7
//! and 14 and 15 unused (set in the data's 0xFFC0); bits 2 to 4 unused. `shape.rot.x` and
//! `shape.rot.z`: a tilt, as a rotation (Jabu-Jabu's targets on walls).
//! Flags: type 1 reads switch flag `switchFlag` (set: it rises, or starts up).

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_OBJ_HSBLOCK` (`actor_table.h`: 0x012D).
pub const ACTOR_OBJ_HSBLOCK: i16 = 0x012D;
/// `ACTOR_OBJ_ICE_POLY` (`actor_table.h`: 0x011E): not ported.
pub const ACTOR_OBJ_ICE_POLY: i16 = 0x011E;
pub const OBJECT: &str = "object_d_hsblock";
/// `OBJECT_D_HSBLOCK` (`object_table.h`: 0x011C).
pub const OBJECT_D_HSBLOCK: i16 = 0x011C;
/// `SCENE_FIRE_TEMPLE` (`scene_table.h`: 0x04).
pub const SCENE_FIRE_TEMPLE: u16 = 0x04;

/// `Obj_Hsblock_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_HSBLOCK, name: "Obj_Hsblock", category: ACTORCAT_BG, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 2)`: the type.
pub const TYPE: Field = Field::params("type", 0, 2, Kind::Named(&[(0, "a post"), (1, "a post rising on its switch flag"), (2, "a flat target")]), "the block");
/// Bits 2 to 4: read by nothing.
pub const BITS_2_TO_4: Field = Field::params("bits 2 to 4", 2, 3, Kind::Unused, "read by nothing");
/// `PARAMS_GET_U(params, 5, 1)`: the ice cover.
pub const ICE: Field = Field::params("ice", 5, 1, Kind::Named(&[(0, "no ice"), (1, "an ice cover (Obj_Ice_Poly, params 1) on it")]), "whether it's iced over");
/// Bits 6 and 7: set in the data's 0xFFC0.
pub const BITS_6_AND_7: Field = Field::params("bits 6 and 7", 6, 2, Kind::Unused, "set in the data (0xFFC0); read by nothing");
/// `PARAMS_GET_U(params, 8, 6)`: type 1's switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 8, 6, Kind::SwitchFlag, "type 1's: set, it rises (or starts up); 0x3F in the other types' data");
/// Bits 14 and 15: set in the data's 0xFFC0.
pub const BITS_14_AND_15: Field = Field::params("bits 14 and 15", 14, 2, Kind::Unused, "set in the data (0xFFC0); read by nothing");

/// `shape.rot.x`, `shape.rot.z`: a tilt (Jabu-Jabu's targets on walls), as a rotation.
pub const ROT_X: Field = Field::rot("rot.x", Source::RotX, 0, 16, Kind::Rotation, "a tilt (a target on a wall)");
pub const ROT_Z: Field = Field::rot("rot.z", Source::RotZ, 0, 16, Kind::Rotation, "a tilt (a target on a wall)");

/// Obj_Hsblock's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, BITS_2_TO_4, ICE, BITS_6_AND_7, SWITCH_FLAG, BITS_14_AND_15, ROT_X, ROT_Z],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().note("type 1: set, it rises")],
    todo: false,
};

/// `D_80B940C0`: the focus's height, by type.
pub const FOCUS_HEIGHTS: [f32; 3] = [85.0, 85.0, 0.0];
/// `sCollisionHeaders`, by type.
const COLLISIONS: [&str; 3] = ["gHookshotPostCol", "gHookshotPostCol", "gHookshotTargetCol"];
/// `sDLists`, by type.
const DLISTS: [&str; 3] = ["gHookshotPostDL", "gHookshotPostDL", "gHookshotTargetDL"];
/// `sFireTempleColor`.
pub const FIRE_TEMPLE_COLOR: [u8; 3] = [165, 125, 55];
/// `mREG(13)`, `mREG(14)`, `mREG(15)`: 255 each, as `ObjHsblock_Init` sets them.
pub const DEFAULT_COLOR: [u8; 3] = [255, 255, 255];

/// The segment the bakes take the draw's `gDPSetEnvColor` from.
const SEG_ENV: u8 = 0x0B;

fn bake_name(dl: &str) -> String {
    format!("Obj_Hsblock/{dl}")
}

/// `ObjHsblock_Draw`'s lists after `Gfx_SetupDL_25Opa` and a dynamic `gDPSetEnvColor`.
pub fn bakes() -> Vec<MeshBake> {
    ["gHookshotPostDL", "gHookshotTargetDL"]
        .iter()
        .map(|dl| MeshBake {
            name: bake_name(dl),
            object: OBJECT.into(),
            segments: vec![(SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false })],
            prelude: vec![SEG_ENV],
            body: BakeBody::DLists(vec![(OBJECT.into(), (*dl).into())]),
        })
        .collect()
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `NULL` (`func_80B93D90`): standing.
    None,
    /// `func_80B93DF4`: sunk, waiting for the switch flag.
    WaitForSwitch,
    /// `func_80B93E5C`: rising.
    Rise,
}

pub struct ObjHsblock {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl ObjHsblock {
    /// `PARAMS_GET_U(params, 0, 2)`.
    fn ty(&self) -> usize {
        TYPE.get(self.actor.params) as usize
    }

    /// `PARAMS_GET_U(params, 8, 6)`.
    fn switch_flag(&self) -> i32 {
        SWITCH_FLAG.get(self.actor.params) as i32
    }

    /// `ObjHsblock_Init`: the type's collision (`func_80B93B68`, transform flags 0), `sInitChain`
    /// (scale 0.1; culling volume 2000, 400, 2000), the ice (`func_80B93BF0`); types 0 and 2
    /// stand, type 1 stands with its flag set and else sinks to wait for it.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = ObjHsblock { actor, bg: BG_ACTOR_MAX, action: Action::None };
        let ty = this.ty();
        // func_80B93B68: DynaPolyActor_Init(transformFlags 0), the collision.
        match COLLISIONS.get(ty) {
            Some(col) => match crate::obj_kibako2::load_collision(play, OBJECT, col) {
                Some(h) => this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0),
                None => log::warn!("Warning : move BG registration failed (../z_obj_hsblock.c 163)(name {})", this.actor.id),
            },
            // sCollisionHeaders[3]: past the C's table.
            None => log::error!("Obj_Hsblock: type {ty} reads past sCollisionHeaders"),
        }
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100); cullingVolumeDistance 2000,
        // cullingVolumeScale 400, cullingVolumeDownward 2000.
        this.actor.scale = Vec3::splat(0.1);
        this.actor.culling_volume_distance = 2000.0;
        this.actor.culling_volume_scale = 400.0;
        this.actor.culling_volume_downward = 2000.0;
        this.func_80b93bf0(play);
        match ty {
            0 | 2 => this.func_80b93d90(),
            1 => {
                if play.flags.get_switch(this.switch_flag()) {
                    this.func_80b93d90();
                } else {
                    this.func_80b93db0();
                }
            }
            _ => {}
        }
        // mREG(13) = mREG(14) = mREG(15) = 255 (DEBUG_FEATURES): DEFAULT_COLOR.
        play.col.dyna.set_source(this.bg, source(&this.actor));
        Box::new(this)
    }

    /// `func_80B93BF0`: with the ice bit, `Obj_Ice_Poly` (params 1) spawned as its child at it.
    fn func_80b93bf0(&mut self, play: &mut PlayState) {
        if ICE.get(self.actor.params) != 0 {
            let r = self.actor.world_rot;
            let pos = self.actor.world_pos;
            log::debug!("Obj_Hsblock: Obj_Ice_Poly isn't ported (a placeholder)");
            let _ = play.actor_spawn_as_child(&mut self.actor, ACTOR_OBJ_ICE_POLY, pos, [r.x, r.y, r.z], 1);
        }
    }

    /// `func_80B93D90`: standing (no action).
    fn func_80b93d90(&mut self) {
        self.action = Action::None;
    }

    /// `func_80B93DB0`: updated off screen, 105 below its home, waiting for its flag.
    fn func_80b93db0(&mut self) {
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.actor.world_pos.y = self.actor.home_pos.y - 105.0;
        self.action = Action::WaitForSwitch;
    }

    /// `func_80B93DF4`: the flag set, it rises (`func_80B93E38`).
    fn func_80b93df4(&mut self, play: &PlayState) {
        if play.flags.get_switch(self.switch_flag()) {
            self.action = Action::Rise;
        }
    }

    /// `func_80B93E5C`: `velocity.y` eases to 16 (a tenth, at most 0.8 a frame); the height eases
    /// home (three tenths, at most `velocity.y`, at least 0.3); home (within 0.001), it stands
    /// and is culled again.
    fn func_80b93e5c(&mut self) {
        eng_math::smooth_step_to_f(&mut self.actor.velocity.y, 16.0, 0.1, 0.8, 0.0);
        let vy = self.actor.velocity.y;
        if eng_math::smooth_step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 0.3, vy, 0.3).abs() < 0.001 {
            self.actor.world_pos.y = self.actor.home_pos.y;
            self.func_80b93d90();
            self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        }
    }
}

impl ActorImpl for ObjHsblock {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjHsblock_Update`: the action, then the focus at the type's height.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::None => {}
            Action::WaitForSwitch => self.func_80b93df4(play),
            Action::Rise => self.func_80b93e5c(),
        }
        let h = FOCUS_HEIGHTS.get(self.ty()).copied().unwrap_or(0.0);
        self.actor.set_focus(h);
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `ObjHsblock_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// `ObjHsblock_Draw`: `Gfx_SetupDL_25Opa`, the model matrix, the env colour (the Fire
    /// Temple's, else white), the type's list.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(dl) = DLISTS.get(self.ty()) else { return };
        let [r, g, b] = if play.scene_id == SCENE_FIRE_TEMPLE { FIRE_TEMPLE_COLOR } else { DEFAULT_COLOR };
        let mut sv = SegmentValues::default();
        sv.env[SEG_ENV as usize] = Some([r, g, b, 255]);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(dl))), transform: actor_draw_matrix(rs), bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
