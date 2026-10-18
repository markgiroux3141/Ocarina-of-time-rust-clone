//! `Eff_Dust` (`ovl_Eff_Dust/z_eff_dust.c`): 64 sparkles (`gEffSparklesDL`, gameplay_keep), each
//! flying in from a random point to the centre and fading, a new one from each spent slot (up to
//! three or two a frame). Five types by params (`EffDustType`):
//!
//! - **0** (`EffDust_UpdateFunc_8099DB28`, drawn by `EffDust_DrawFunc_8099E4F4`): from 800 behind
//!   (a 45-degree cone back along x), by 0.05 a frame, three a frame; grey, a tenth the size,
//!   at its own position, facing the camera, the x part all the way and y and z to a fifth.
//! - **1** (`EffDust_UpdateFunc_8099DD74`): from a dome 400 round (up to 45 degrees up), by 0.03,
//!   two a frame; half the size, y all the way.
//! - **2, 3, 4** (`EffDust_UpdateFunc_8099DFC0`, `EffDust_DrawFunc_8099E784`): the spin attack's
//!   charge (`En_M_Thunder`'s child, `swordType + 2`: 2 the Master Sword's, 3 the Kokiri Sword's,
//!   4 the Biggoron's): following its parent, by 0.1 a frame, three a frame, from a ring round
//!   the blade (Player's `mf_9E0`, 320 along it) 700 to 5200 (2), 700 to 3200 (3) or 1700 to
//!   10200 (4) out; growing to 15, 10 or 20 times its size as it comes in and fading in; blue,
//!   red from the charge's 0.85 (`unk_858`). Once the parent or the charge is gone, ten frames
//!   (by 0.2) and gone. Type 4 is kept over room changes (`room` -1).
//!
//! Spawned by `En_M_Thunder` (types 2 to 4), `Demo_6K` and `En_Jj` (not ported).
//!
//! **@bug (game)**: any other params frees the actor's memory without killing it
//! (`SYSTEM_ARENA_FREE`, in the wrong arena): here it's killed (logged), a placeholder for that.
//! The draws set up the opaque list (`Gfx_SetupDL_25Opa`) and draw into the translucent one,
//! whose state is the last translucent draw's: the bake starts from `Gfx_SetupDL_25Opa` (the
//! list sets its own render mode), and segment 8's empty list (`sEmptyDL`) isn't called by it.
//!
//! Params: `params` whole (`EffDustType`: 0 to 4). Not placed: only spawned.
//! Flags: none.

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

use crate::player::{Player, STATE1_12};

/// `ACTOR_EFF_DUST` (`actor_table.h`: 0x0101).
pub const ACTOR_EFF_DUST: i16 = 0x0101;

/// `Eff_Dust_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EFF_DUST, name: "Eff_Dust", category: ACTORCAT_NPC, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED, object: "gameplay_keep", schema: &SCHEMA };

/// `EffDustType` (`z_eff_dust.h`).
pub const EFF_DUST_TYPE_0: i16 = 0;
pub const EFF_DUST_TYPE_1: i16 = 1;
pub const EFF_DUST_TYPE_2: i16 = 2;
pub const EFF_DUST_TYPE_3: i16 = 3;
pub const EFF_DUST_TYPE_4: i16 = 4;

/// `this->actor.params`: the type.
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[(0, "EFF_DUST_TYPE_0"), (1, "EFF_DUST_TYPE_1"), (2, "EFF_DUST_TYPE_2"), (3, "EFF_DUST_TYPE_3"), (4, "EFF_DUST_TYPE_4")]),
    "the sparkles' kind: 0 from behind, 1 from a dome round, 2 to 4 the spin attack's charge (Master, Kokiri, Biggoron sword)",
);

/// Eff_Dust's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false, objects: &[] };

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EffDust_UpdateFunc_8099DB28` (type 0).
    UpdateFunc8099DB28,
    /// `EffDust_UpdateFunc_8099DD74` (type 1).
    UpdateFunc8099DD74,
    /// `EffDust_UpdateFunc_8099DFC0` (types 2 to 4).
    UpdateFunc8099DFC0,
}

/// `drawFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawFunc {
    /// `EffDust_DrawFunc_8099E4F4` (types 0 and 1).
    DrawFunc8099E4F4,
    /// `EffDust_DrawFunc_8099E784` (types 2 to 4).
    DrawFunc8099E784,
}

pub struct EffDust {
    pub actor: Actor,
    /// `distanceTraveled`: each sparkle's way in, 0 to 1 (1: spent).
    pub distance_traveled: [f32; 64],
    /// `initialPositions`: where each started.
    pub initial_positions: [Vec3; 64],
    pub index: u8,
    /// `life`: the frames left once the parent or the charge is gone (types 2 to 4).
    pub life: u8,
    pub dx: f32,
    pub dy: f32,
    pub dz: f32,
    pub scaling_factor: f32,
    pub action: Action,
    pub draw_func: DrawFunc,
    /// Player's `mf_9E0` and `unk_858` at this frame's draw (Player draws first).
    pub player_mf: Mat4,
    pub player_unk_858: f32,
}

impl EffDust {
    /// `EffDust_InitPosAndDistance`: every sparkle spent at the centre.
    fn init_pos_and_distance(&mut self) {
        for i in 0..64 {
            self.initial_positions[i] = Vec3::ZERO;
            self.distance_traveled[i] = 1.0;
        }
        self.index = 0;
    }

    /// `EffDust_Init`.
    pub fn init(actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EffDust {
            actor,
            distance_traveled: [1.0; 64],
            initial_positions: [Vec3::ZERO; 64],
            index: 0,
            life: 0,
            dx: 0.0,
            dy: 0.0,
            dz: 0.0,
            scaling_factor: 0.0,
            action: Action::UpdateFunc8099DB28,
            draw_func: DrawFunc::DrawFunc8099E4F4,
            player_mf: Mat4::IDENTITY,
            player_unk_858: 0.0,
        };
        this.init_pos_and_distance();
        match TYPE.get(this.actor.params) {
            EFF_DUST_TYPE_0 => {
                this.action = Action::UpdateFunc8099DB28;
                this.draw_func = DrawFunc::DrawFunc8099E4F4;
                this.dy = 0.8;
                this.dz = 0.8;
                this.dx = 1.0;
                this.scaling_factor = 0.1;
            }
            EFF_DUST_TYPE_1 => {
                this.action = Action::UpdateFunc8099DD74;
                this.draw_func = DrawFunc::DrawFunc8099E4F4;
                this.dx = 0.8;
                this.dz = 0.8;
                this.dy = 1.0;
                this.scaling_factor = 0.5;
            }
            EFF_DUST_TYPE_2 => {
                this.action = Action::UpdateFunc8099DFC0;
                this.draw_func = DrawFunc::DrawFunc8099E784;
                this.dx = 0.5;
                this.scaling_factor = 15.0;
            }
            EFF_DUST_TYPE_3 => {
                this.action = Action::UpdateFunc8099DFC0;
                this.draw_func = DrawFunc::DrawFunc8099E784;
                this.dx = 0.5;
                this.scaling_factor = 10.0;
            }
            EFF_DUST_TYPE_4 => {
                this.action = Action::UpdateFunc8099DFC0;
                this.draw_func = DrawFunc::DrawFunc8099E784;
                this.actor.room = -1;
                this.dx = 0.5;
                this.scaling_factor = 20.0;
            }
            t => {
                // @bug (game): SYSTEM_ARENA_FREE(this) (not Actor_Kill, and the wrong arena).
                log::warn!("Eff_Dust: type {t} frees itself (SYSTEM_ARENA_FREE): killed instead");
                this.actor.kill();
            }
        }
        this.life = 10;
        Box::new(this)
    }

    /// `distanceTraveled[i] += step` for each sparkle on its way.
    fn advance(&mut self, step: f32) {
        for d in self.distance_traveled.iter_mut() {
            if *d < 1.0 {
                *d += step;
            }
        }
    }

    /// `EffDust_UpdateFunc_8099DB28`: three new sparkles a frame at most, from 800 behind in a
    /// cone (`fi` within ±0x1000, `theta` within ±0x800).
    fn update_func_8099db28(&mut self, play: &mut PlayState) {
        self.advance(0.05);
        for _ in 0..3 {
            let i = (self.index & 0x3F) as usize;
            if self.distance_traveled[i] >= 1.0 {
                // Spherical coordinate system.
                let fi = play.rand.centered_float(8192.0) as i32 as i16;
                let theta = play.rand.centered_float(4096.0) as i32 as i16;
                self.initial_positions[i] = Vec3::new(-800.0 * cos_s(fi) * cos_s(theta), -800.0 * sin_s(theta), -800.0 * sin_s(fi) * cos_s(theta));
                self.distance_traveled[i] = 0.0;
                self.index = self.index.wrapping_add(1);
            }
        }
    }

    /// `EffDust_UpdateFunc_8099DD74`: two a frame at most, from a dome 400 round.
    fn update_func_8099dd74(&mut self, play: &mut PlayState) {
        self.advance(0.03);
        for _ in 0..2 {
            let i = (self.index & 0x3F) as usize;
            if self.distance_traveled[i] >= 1.0 {
                // Spherical coordinate system.
                let fi = play.rand.centered_float(65536.0) as i32 as i16;
                let theta = play.rand.zero_float(8192.0) as i32 as i16;
                self.initial_positions[i] = Vec3::new(400.0 * cos_s(fi) * cos_s(theta), 400.0 * sin_s(theta), 400.0 * sin_s(fi) * cos_s(theta));
                self.distance_traveled[i] = 0.0;
                self.index = self.index.wrapping_add(1);
            }
        }
    }

    /// `EffDust_UpdateFunc_8099DFC0`: with the parent there and Player charging, at the parent,
    /// three a frame at most, from a ring round the blade (by type); else the sparkles hurry in
    /// (by 0.2) and after `life` frames it's gone.
    fn update_func_8099dfc0(&mut self, play: &mut PlayState) {
        let charging = play.player.and_then(|h| play.actors.downcast::<Player>(h)).is_some_and(|p| p.state1 & STATE1_12 != 0);
        let parent_pos = self.actor.parent.and_then(|h| play.actors.actor(h)).filter(|a| !a.killed).map(|a| a.world_pos);
        let Some(parent_pos) = parent_pos.filter(|_| charging) else {
            if self.life != 0 {
                self.life -= 1;
            } else {
                self.actor.kill();
            }
            self.advance(0.2);
            return;
        };
        self.advance(0.1);
        self.actor.world_pos = parent_pos;
        for _ in 0..3 {
            let i = (self.index & 0x3F) as usize;
            if self.distance_traveled[i] >= 1.0 {
                let theta = play.rand.centered_float(65536.0) as i32 as i16;
                // (x out along the blade; inside the ring's radius at the start, the ring.)
                let ring = |rand: &mut oot_game::play::Rand, len: f32, base: f32, cut: f32, r: f32| {
                    let x = rand.zero_one() * len + base;
                    if x > cut {
                        let y = (r * rand.zero_one()) * sin_s(theta);
                        let z = (r * rand.zero_one()) * cos_s(theta);
                        Vec3::new(x, y, z)
                    } else {
                        Vec3::new(x, r * sin_s(theta), r * cos_s(theta))
                    }
                };
                match TYPE.get(self.actor.params) {
                    EFF_DUST_TYPE_2 => self.initial_positions[i] = ring(&mut play.rand, 4500.0, 700.0, 3000.0, 3000.0),
                    EFF_DUST_TYPE_3 => self.initial_positions[i] = ring(&mut play.rand, 2500.0, 700.0, 2000.0, 2000.0),
                    EFF_DUST_TYPE_4 => self.initial_positions[i] = ring(&mut play.rand, 8500.0, 1700.0, 5000.0, 4000.0),
                    _ => {}
                }
                self.distance_traveled[i] = 0.0;
                self.index = self.index.wrapping_add(1);
            }
        }
    }
}

pub const BAKE: &str = "Eff_Dust/sparkle";
/// The colours (`gDPSetPrimColor`, `gDPSetEnvColor` before each sparkle).
const SEG_COLOR: u8 = 0x0A;

/// `gEffSparklesDL` after the draw's colours.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: "gameplay_keep".into(),
        segments: vec![(SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true })],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffSparklesDL".into())]),
    }]
}

fn cmd(m: Mat4, prim: [u8; 4], env: [u8; 4]) -> DrawCmd {
    let mut sv = SegmentValues::default();
    sv.prim[SEG_COLOR as usize] = Some(prim);
    sv.env[SEG_COLOR as usize] = Some(env);
    DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } }
}

impl ActorImpl for EffDust {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EffDust_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::UpdateFunc8099DB28 => self.update_func_8099db28(play),
            Action::UpdateFunc8099DD74 => self.update_func_8099dd74(play),
            Action::UpdateFunc8099DFC0 => self.update_func_8099dfc0(play),
        }
    }

    /// `EffDust_Destroy`: nothing.
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// Player's `mf_9E0` and charge as this frame drew him.
    fn draw_update(&mut self, play: &mut PlayState) {
        if let Some(p) = play.player.and_then(|h| play.actors.downcast::<Player>(h)) {
            self.player_mf = p.mf_9e0;
            self.player_unk_858 = p.unk_858;
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.values = self.distance_traveled.to_vec();
        rs
    }

    /// `EffDust_Draw`: `drawFunc`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        if rs.values.len() != 64 {
            return;
        }
        match self.draw_func {
            // EffDust_DrawFunc_8099E4F4: grey, each at its place facing the camera:
            // Matrix_Translate(NEW), Matrix_Scale, Matrix_Mult(&billboardMtxF, APPLY).
            DrawFunc::DrawFunc8099E4F4 => {
                for (d, p) in rs.values.iter().zip(self.initial_positions.iter()) {
                    if *d < 1.0 {
                        let aux = 1.0 - d * d;
                        let t = Vec3::new(
                            rs.pos.x + p.x * ((self.dx * aux) + (1.0 - self.dx)),
                            rs.pos.y + p.y * ((self.dy * aux) + (1.0 - self.dy)),
                            rs.pos.z + p.z * ((self.dz * aux) + (1.0 - self.dz)),
                        );
                        let m = Mat4::from_translation(t) * Mat4::from_scale(Vec3::splat(self.scaling_factor)) * view.billboard;
                        out.xlu.push(cmd(m, [128, 128, 128, 255], [128, 128, 128, 0]));
                    }
                }
            }
            // EffDust_DrawFunc_8099E784: along the blade (mf_9E0), 320 out and 20 back, growing
            // and fading in as each comes in, facing the camera (Matrix_ReplaceRotation); env red
            // from the charge's 0.85, else blue.
            DrawFunc::DrawFunc8099E784 => {
                let env = if self.player_unk_858 >= 0.85 { [255, 0, 0, 0] } else { [0, 0, 255, 0] };
                for (d, p) in rs.values.iter().zip(self.initial_positions.iter()) {
                    if *d < 1.0 {
                        let alpha = (*d * 255.0) as i32 as u8;
                        let aux = 1.0 - d * d;
                        let t = Vec3::new(p.x * ((self.dx * aux) + (1.0 - self.dx)), p.y * (1.0 - d) + 320.0, p.z * (1.0 - d) + -20.0);
                        let s = d * self.scaling_factor;
                        let m = self.player_mf * Mat4::from_translation(t) * Mat4::from_scale(Vec3::splat(s));
                        let m = oot_game::draw::replace_rotation(m, view.billboard);
                        out.xlu.push(cmd(m, [255, 255, 255, alpha], env));
                    }
                }
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
