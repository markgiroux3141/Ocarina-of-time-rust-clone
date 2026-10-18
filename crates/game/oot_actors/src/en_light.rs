//! `En_Light` (`ovl_En_Light/z_en_light.c`): a flame on its own (a torch's fire set in a wall, a
//! candle's), with its point light. `params & 0xF` picks the colours and size (`D_80A9E840`); a
//! negative `params` is the small candle flame (`gUnusedCandleDL` of `gameplay_dangeon_keep`, a
//! light 100 across, no crackle), else the fire (`gEffFire1DL`, 300 across, `NA_SE_EV_TORCH`).
//! Bit 10 lets a switch flag (bits 4 to 9) light it or put it out, growing or shrinking 0.05 a
//! frame (bit 11 reverses the flag's sense). The flame turns to face the active camera; with a
//! parent it follows it 17 above. The light's glow isn't drawn (BACKLOG #31).
//!
//! The whole overlay is ported.
//!
//! Params: `flameParams` bits 0 to 3 (`D_80A9E840`'s entry: the colours and size), bit 0 also
//! turning it half a turn; `switchFlag` bits 4 to 9 (with bit 10); bit 10 (the switch flag lights
//! it or puts it out); bit 11 (with bit 10: 1 lit while the flag is set, 0 while it's clear); bit 15
//! (`params < 0`: the candle, else the fire). `world.rot.x`: a rotation (the flame tilted).
//! Flags: reads the switch flag `switchFlag` (bit 10).

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{binang_to_rad, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::audio::sfx::{NA_SE_EV_TORCH, SFX_FLAG};
use oot_game::lights::{LightInfo, LightNode};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

/// `ACTOR_EN_LIGHT` (`actor_table.h`: 0x0008).
pub const ACTOR_EN_LIGHT: i16 = 0x0008;

/// `En_Light_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_LIGHT, name: "En_Light", category: ACTORCAT_ITEMACTION, flags: 0, object: "gameplay_keep", schema: &SCHEMA };

/// `PARAMS_GET_S(this->actor.params, 0, 4)`: the flame's entry in `D_80A9E840`.
pub const FLAME: Field = Field::params("flameParams", 0, 4, Kind::Range(0, 15), "the flame's colours and size (D_80A9E840's entry)");
/// `PARAMS_GET_S(this->actor.params, 0, 1)`: the draw's half turn.
pub const HALF_TURN: Field =
    Field::params("PARAMS_GET_S(params, 0, 1)", 0, 1, Kind::Named(&[(0, "as it is"), (1, "turned half a turn")]), "the flame drawn turned half a turn (odd entries)").part_of("flameParams");
/// `PARAMS_GET_S(this->actor.params, 4, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 4, 6, Kind::SwitchFlag, "lights it or puts it out (with bit 10)");
/// `PARAMS_GET_NOSHIFT(this->actor.params, 10, 1)`: `EnLight_UpdateSwitch`.
pub const BY_SWITCH: Field =
    Field::params("PARAMS_GET_NOSHIFT(params, 10, 1)", 10, 1, Kind::Named(&[(0, "always lit"), (1, "lit or out by the switch flag")]), "whether the switch flag lights it");
/// `PARAMS_GET_NOSHIFT(this->actor.params, 11, 1)`: the flag's sense.
pub const LIT_WITH_FLAG: Field = Field::params(
    "PARAMS_GET_NOSHIFT(params, 11, 1)",
    11,
    1,
    Kind::Named(&[(0, "lit while the flag is clear"), (1, "lit while the flag is set")]),
    "the switch flag's sense",
)
.when("PARAMS_GET_NOSHIFT(params, 10, 1)", &[1]);
/// `this->actor.params < 0`: the candle.
pub const CANDLE: Field = Field::params("params < 0", 15, 1, Kind::Named(&[(0, "the fire"), (1, "the candle")]), "the small candle flame (light 100, silent) or the fire");
/// `world.rot.x`: a tilt, drawn.
pub const ROT_X: Field = Field::rot("world.rot.x", Source::RotX, 0, 16, Kind::Rotation, "its tilt");

/// En_Light's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[FLAME, HALF_TURN, SWITCH_FLAG, BY_SWITCH, LIT_WITH_FLAG, CANDLE, ROT_X],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().note("bit 10: lights it or puts it out")],
    todo: false,
    objects: &[],
};

/// `GAMEMODE_END_CREDITS` (`save.h`: 3).
const GAMEMODE_END_CREDITS: u8 = 3;

/// `FlameParams`: `primColor`, `envColor`, `scale`.
#[derive(Debug, Clone, Copy)]
pub struct FlameParams {
    pub prim: [u8; 4],
    pub env: [u8; 3],
    pub scale: u8,
}

const fn fp(prim: [u8; 4], env: [u8; 3], scale: u8) -> FlameParams {
    FlameParams { prim, env, scale }
}

/// `D_80A9E840`.
pub const FLAME_PARAMS: [FlameParams; 16] = [
    fp([255, 200, 0, 255], [255, 0, 0], 75),
    fp([255, 200, 0, 255], [255, 0, 0], 75),
    fp([0, 170, 255, 255], [0, 0, 255], 75),
    fp([170, 255, 0, 255], [0, 150, 0], 75),
    fp([255, 200, 0, 255], [255, 0, 0], 40),
    fp([255, 200, 0, 255], [255, 0, 0], 75),
    fp([170, 255, 0, 255], [0, 150, 0], 75),
    fp([0, 170, 255, 255], [0, 0, 255], 75),
    fp([255, 0, 170, 255], [200, 0, 0], 75),
    fp([255, 255, 170, 255], [255, 50, 0], 75),
    fp([255, 255, 170, 255], [255, 255, 0], 75),
    fp([255, 255, 170, 255], [100, 255, 0], 75),
    fp([255, 170, 255, 255], [255, 0, 100], 75),
    fp([255, 170, 255, 255], [100, 0, 255], 75),
    fp([170, 255, 255, 255], [0, 0, 255], 75),
    fp([170, 255, 255, 255], [0, 150, 255], 75),
];

/// The fire (`Gfx_SetupDL_25Xlu`, its colours and `gEffFire1DL` under segment 8's scroll) and the
/// candle (`gUnusedCandleDL`).
pub const FIRE_BAKE: &str = "En_Light/fire";
pub const CANDLE_BAKE: &str = "En_Light/candle";
const SEG_SCROLL: u8 = 0x08;
const SEG_COLORS: u8 = 0x0C;
const SEG_KEEP: u8 = 0x05;

/// The fire's colours: `gDPSetPrimColor(0x80, 0x80, prim)`, `gDPSetEnvColor(env, 0)`.
fn fire_colors(f: &FlameParams) -> Vec<(u32, u32)> {
    let [r, g, b, a] = f.prim;
    let [er, eg, eb] = f.env;
    vec![(0xFA00_8080, u32::from_be_bytes([r, g, b, a])), (0xFB00_0000, u32::from_be_bytes([er, eg, eb, 0])), (0xDF00_0000, 0)]
}

/// The candle's: `gDPSetPrimColor(0xC0, 0xC0, 255, 200, 0, 0)`, `gDPSetEnvColor(255, 0, 0, 0)`.
const CANDLE_COLORS: [(u32, u32); 3] = [(0xFA00_C0C0, 0xFFC8_0000), (0xFB00_0000, 0xFF00_0000), (0xDF00_0000, 0)];

/// The fire's scroll: `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, 0, 0, 32, 64, 1, 0, (timer * -20) & 511, 32, 128)`.
fn fire_scroll(timer: u8) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, 0, 32, 64, 1, 0, (timer as i32 * -20) as u32 & 511, 32, 128)
}

/// The candle's: `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, 0, 0, 16, 32, 1, (timer * 2) & 63, (timer * -6) & 127, 16, 32)`.
fn candle_scroll(timer: u8) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, 0, 16, 32, 1, (timer as u32 * 2) & 63, (timer as i32 * -6) as u32 & 127, 16, 32)
}

pub fn bakes() -> Vec<MeshBake> {
    vec![
        MeshBake {
            name: FIRE_BAKE.into(),
            object: "gameplay_keep".into(),
            segments: vec![(SEG_COLORS, BakeSegment::Dynamic(fire_colors(&FLAME_PARAMS[0]))), (SEG_SCROLL, BakeSegment::Dynamic(fire_scroll(0)))],
            prelude: vec![SEG_COLORS],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffFire1DL".into())]),
        },
        MeshBake {
            name: CANDLE_BAKE.into(),
            object: "gameplay_keep".into(),
            segments: vec![
                (SEG_KEEP, BakeSegment::File("gameplay_dangeon_keep".into())),
                (SEG_COLORS, BakeSegment::Commands(CANDLE_COLORS.to_vec())),
                (SEG_SCROLL, BakeSegment::Dynamic(candle_scroll(0))),
            ],
            prelude: vec![SEG_COLORS],
            body: BakeBody::DLists(vec![("gameplay_dangeon_keep".into(), "gUnusedCandleDL".into())]),
        },
    ]
}

pub struct EnLight {
    pub actor: Actor,
    pub timer: u8,
    pub light_node: Option<LightNode>,
    pub light_info: LightInfo,
    /// `actor.update`: `EnLight_UpdateSwitch` (params bit 10).
    pub update_switch: bool,
}

impl EnLight {
    /// `EnLight_Init`: the point light 40 above (1 for a candle), white-yellow, its radius -1 until
    /// the first update; the scale from the flame's entry; a random start to the scroll.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let y_offset: i16 = if CANDLE.get(actor.params) != 0 { 1 } else { 40 };
        let p = actor.world_pos;
        let (x, y, z) = (p.x as i16, y_offset.wrapping_add(p.y as i16), p.z as i16);
        let light_info = if play.save.game_mode == GAMEMODE_END_CREDITS { LightInfo::point_no_glow(x, y, z, [255, 255, 180], -1) } else { LightInfo::point_glow(x, y, z, [255, 255, 180], -1) };
        let light_node = play.light_ctx.insert_light(light_info);
        let mut this = EnLight { actor, timer: 0, light_node, light_info, update_switch: false };
        this.actor.scale = Vec3::splat(FLAME_PARAMS[FLAME.get(this.actor.params) as usize].scale as f32 * 0.0001);
        this.timer = (play.rand.zero_one() * 255.0) as i32 as u8;
        if BY_SWITCH.get(this.actor.params) != 0 {
            this.update_switch = true;
        }
        Box::new(this)
    }

    /// `EnLight_UpdatePosRot`: turned to face the active camera; on its parent 17 above it.
    fn update_pos_rot(&mut self, play: &PlayState) {
        self.actor.shape_rot.y = play.cam_dir_yaw().wrapping_add(i16::MIN);
        if let Some(p) = self.actor.parent.and_then(|h| play.actors.actor(h)) {
            self.actor.world_pos = p.world_pos;
            self.actor.world_pos.y += 17.0;
        }
        self.timer = self.timer.wrapping_add(1);
    }

    /// `Lights_PointSetColorAndRadius` with the flame's colour at a random brightness.
    fn set_light(&mut self, play: &mut PlayState, f: &FlameParams, radius: i16) {
        let intensity = play.rand.zero_one() * 0.5 + 0.5;
        self.light_info.color = [(f.prim[0] as f32 * intensity) as u8, (f.prim[1] as f32 * intensity) as u8, (f.prim[2] as f32 * intensity) as u8];
        self.light_info.radius = radius;
        play.light_ctx.set_info(self.light_node, self.light_info);
    }

    /// `EnLight_Update`.
    fn update_normal(&mut self, play: &mut PlayState) {
        let f = FLAME_PARAMS[FLAME.get(self.actor.params) as usize];
        let radius = if CANDLE.get(self.actor.params) != 0 { 100 } else { 300 };
        self.set_light(play, &f, radius);
        self.update_pos_rot(play);
        if CANDLE.get(self.actor.params) == 0 {
            audio_play_actor_sfx2(play, NA_SE_EV_TORCH - SFX_FLAG);
        }
    }

    /// `EnLight_UpdateSwitch`: lit with the flag (or without it, bit 11 clear) it grows to its full
    /// size, else shrinks to nothing (gone at a tenth).
    fn update_by_switch(&mut self, play: &mut PlayState) {
        let f = FLAME_PARAMS[FLAME.get(self.actor.params) as usize];
        // (f32)scale * 0.0001: a double.
        let full = f.scale as f32 as f64 * 0.0001;
        let mut scale = (self.actor.scale.x as f64 / full) as f32;
        let flag = play.flags.get_switch(SWITCH_FLAG.get(self.actor.params) as i32);
        let lit = if LIT_WITH_FLAG.get(self.actor.params) != 0 { flag } else { !flag };
        if lit {
            step_to_f(&mut scale, 1.0, 0.05);
        } else {
            if scale < 0.1 {
                self.actor.scale = Vec3::ZERO;
                return;
            }
            step_to_f(&mut scale, 0.0, 0.05);
        }
        self.actor.scale = Vec3::splat((full * scale as f64) as f32);
        let intensity = play.rand.zero_one() * 0.5 + 0.5;
        self.light_info.color = [(f.prim[0] as f32 * intensity) as u8, (f.prim[1] as f32 * intensity) as u8, (f.prim[2] as f32 * intensity) as u8];
        self.light_info.radius = (300.0 * scale) as i16;
        play.light_ctx.set_info(self.light_node, self.light_info);
        self.update_pos_rot(play);
        if CANDLE.get(self.actor.params) == 0 {
            audio_play_actor_sfx2(play, NA_SE_EV_TORCH - SFX_FLAG);
        }
    }
}

impl ActorImpl for EnLight {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    fn update(&mut self, play: &mut PlayState) {
        if self.update_switch { self.update_by_switch(play) } else { self.update_normal(play) }
    }
    /// `EnLight_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.light_ctx.remove_light(self.light_node.take());
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.timer as u32];
        rs
    }
    /// `EnLight_Draw`: turned by `(camDirYaw - shape.rot.y) + 0x8000` (and half a turn more for
    /// odd `params`).
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(&timer) = rs.switches.first() else { return };
        let timer = timer as u8;
        let f = FLAME_PARAMS[FLAME.get(self.actor.params) as usize];
        let mut sv = SegmentValues::default();
        let bake = if CANDLE.get(self.actor.params) == 0 {
            sv.read(SEG_SCROLL, &fire_scroll(timer));
            sv.read(SEG_COLORS, &fire_colors(&f));
            FIRE_BAKE
        } else {
            sv.read(SEG_SCROLL, &candle_scroll(timer));
            CANDLE_BAKE
        };
        let angle = binang_to_rad(play.cam_dir_yaw().wrapping_sub(rs.rot[1]).wrapping_add(i16::MIN));
        let mut t = actor_draw_matrix(rs) * Mat4::from_rotation_y(angle);
        if HALF_TURN.get(self.actor.params) != 0 {
            t *= Mat4::from_rotation_y(std::f32::consts::PI);
        }
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: t, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
