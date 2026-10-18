//! `En_Bdfire` (`ovl_En_Bdfire/z_en_bdfire.c`): King Dodongo's fire breath, his child
//! (`BossDodongo_SpawnFire`). Two kinds by params:
//!
//! - **The flame at his mouth** (params below 0): wide (scale x 2.8), at his `firePos` every frame,
//!   with a white point light where it spawned that turns orange and red (its colour by
//!   `unk_190`/`unk_194`, radius 300); while he inhales (`unk_1E2`) it grows its alpha over 90
//!   frames and then shrinks away; once he stops inhaling it fades out at once.
//! - **A fire ball** (params 0 and up, the order he breathes them in): rolling along the floor at
//!   30 a frame, smaller and fainter the later it came (`unk_188`, `unk_18C`), turning a quarter
//!   (his way round, `unk_1A2`) at the arena's walls (x -1390 and -390, z -2804 and -3804); for its
//!   time (25 less 0.8 per param) it sets Link burning within 130 × its scale / 4.2 (his
//!   `bodyFlameTimers`, a knockdown back along its way: 20 at speed 8), then fades out. The first
//!   ball carries the breath's sound.
//!
//! Its draw: a billboard 11 up through eight flame textures in turn (`unk_156`), prim
//! (255, 255, 100, `unk_18C`), env (200, 0, 0).
//!
//! **@bug (game)**, kept: its profile's id is 0 (`ACTOR_PLAYER`), so every fire's actor `id` is 0.
//!
//! The balls' circle shadow is the shape's (`ActorShape_Init`'s, scale 0, drawn by `Actor_Draw`
//! with a floor: they never look for one, so it's never drawn).
//!
//! Params: below 0 the flame at his mouth; 0 and up a fire ball's order in the breath (not placed:
//! spawned by `Boss_Dodongo`).
//! Flags: none.

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::smooth_step_to_f;
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, actor_set_player_knockback_large, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::*;
use oot_game::gbi::{G_CD_DISABLE, cc_ab, cc_c, cc_d, setup_dl};
use oot_game::lights::{LightInfo, LightNode};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

use crate::boss_dodongo::BossDodongo;

/// `ACTOR_EN_BDFIRE` (`actor_table.h`).
pub const ACTOR_EN_BDFIRE: i16 = 0x0030;
pub const OBJECT: &str = "object_kingdodongo";

/// `En_Bdfire_Profile`: its id is 0 (@bug (game): the actor's `id` is `ACTOR_PLAYER`'s).
pub const PROFILE: ActorProfile =
    ActorProfile { id: 0, name: "En_Bdfire", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `params`: below 0 the flame at King Dodongo's mouth, 0 and up a fire ball's order in his breath.
pub const PARAMS: Field = Field::whole("params", Kind::Range(-1, 0x7FFF), "below 0: the flame at his mouth; 0 and up: a fire ball, its order in the breath (smaller, fainter and shorter-lived the later)");

/// En_Bdfire's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[PARAMS], flags: &[], todo: false };

/// `D_809BCB10`: the flame's eight textures (`object_kingdodongo`, IA8 64 by 64).
const TEXTURES: [&str; 8] = [
    "object_kingdodongo_Tex_0264E0",
    "object_kingdodongo_Tex_0274E0",
    "object_kingdodongo_Tex_0284E0",
    "object_kingdodongo_Tex_0294E0",
    "object_kingdodongo_Tex_02A4E0",
    "object_kingdodongo_Tex_02B4E0",
    "object_kingdodongo_Tex_02C4E0",
    "object_kingdodongo_Tex_02D4E0",
];

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_809BC2A4`: the flame at his mouth.
    Flame,
    /// `func_809BC598`: a fire ball.
    Ball,
}

pub struct EnBdfire {
    pub actor: Actor,
    pub action: Action,
    /// `unk_154`: the timer (the flame's 90, a ball's life).
    pub unk_154: i16,
    /// `unk_156`: the texture counter (from a random 0 to 7 for a ball).
    pub unk_156: i16,
    /// `unk_158`: King Dodongo's way round (`unk_1A2`), for the turns at the walls.
    pub unk_158: i16,
    /// `unk_188`: a ball's scale to grow to.
    pub unk_188: f32,
    /// `unk_18C`: the alpha.
    pub unk_18c: f32,
    /// `unk_190`, `unk_194`: the flame's light's red and green.
    pub unk_190: f32,
    pub unk_194: f32,
    pub light_node: Option<LightNode>,
    pub light_info_no_glow: LightInfo,
}

impl EnBdfire {
    /// `EnBdfire_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.scale = Vec3::splat(0.6);
        let p = actor.world_pos;
        let mut this = EnBdfire {
            actor,
            action: Action::Ball,
            unk_154: 0,
            unk_156: 0,
            unk_158: 0,
            unk_188: 0.0,
            unk_18c: 0.0,
            unk_190: 0.0,
            unk_194: 0.0,
            light_node: None,
            light_info_no_glow: LightInfo::point_no_glow(p.x as i16, p.y as i16, p.z as i16, [255, 255, 255], 300),
        };
        let params = PARAMS.get(this.actor.params);
        if params < 0 {
            this.action = Action::Flame;
            this.actor.scale.x = 2.8;
            this.unk_154 = 90;
            this.light_info_no_glow = LightInfo::point_no_glow(p.x as i16, p.y as i16, p.z as i16, [255, 255, 255], 300);
            this.light_node = play.light_ctx.insert_light(this.light_info_no_glow);
        } else {
            this.action = Action::Ball;
            this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 0.0);
            this.actor.speed_xz = 30.0;
            this.unk_154 = (25 - (params as f32 * 0.8) as i32).max(0) as i16;
            this.unk_188 = 4.2000003 - (params as f32 * 0.25 * 0.6);
            if this.unk_188 < 0.90000004 {
                this.unk_188 = 0.90000004;
            }
            this.unk_18c = 255.0 - (params as f32 * 10.0);
            if this.unk_18c < 20.0 {
                this.unk_18c = 20.0;
            }
            this.unk_156 = (play.rand.zero_one() * 8.0) as i16;
        }
        Box::new(this)
    }

    /// The parent, King Dodongo, if he's there.
    fn king_dodongo<'a>(&self, play: &'a PlayState) -> Option<&'a BossDodongo> {
        self.actor.parent.and_then(|h| play.actors.downcast::<BossDodongo>(h))
    }

    /// `Actor_SetScale(&this->actor, this->actor.scale.x)`.
    fn set_scale(&mut self) {
        let s = self.actor.scale.x;
        self.actor.scale = Vec3::splat(s);
    }

    /// `func_809BC2A4`: at his `firePos`; once he stops inhaling, the flame narrows and fades (killed
    /// at 0); while he inhales, from 70 frames left its alpha to 128 and its light to orange, then at
    /// its end it narrows and fades out with its light.
    fn flame(&mut self, play: &mut PlayState) {
        let Some((fire_pos, inhaling)) = self.king_dodongo(play).map(|k| (k.fire_pos, k.unk_1e2)) else { return };
        self.actor.world_pos = fire_pos;
        if inhaling == 0 {
            smooth_step_to_f(&mut self.actor.scale.x, 0.0, 1.0, 0.6, 0.0);
            if smooth_step_to_f(&mut self.unk_18c, 0.0, 1.0, 20.0, 0.0) == 0.0 {
                self.actor.kill();
            }
        } else {
            if self.unk_154 < 70 {
                smooth_step_to_f(&mut self.unk_18c, 128.0, 0.1, 1.5, 0.0);
                smooth_step_to_f(&mut self.unk_190, 255.0, 1.0, 3.8249998, 0.0);
                smooth_step_to_f(&mut self.unk_194, 100.0, 1.0, 1.5, 0.0);
            }
            let temp = if self.unk_154 == 0 {
                0
            } else {
                self.unk_154 -= 1;
                self.unk_154
            };
            if temp == 0 {
                smooth_step_to_f(&mut self.actor.scale.x, 0.0, 1.0, 0.3, 0.0);
                smooth_step_to_f(&mut self.unk_190, 0.0, 1.0, 25.5, 0.0);
                smooth_step_to_f(&mut self.unk_194, 0.0, 1.0, 10.0, 0.0);
                if smooth_step_to_f(&mut self.unk_18c, 0.0, 1.0, 10.0, 0.0) == 0.0 {
                    self.actor.kill();
                }
            }
            self.set_scale();
            // Lights_PointSetColorAndRadius(&lightInfoNoGlow, unk_190, unk_194, 0, 300).
            self.light_info_no_glow.color = [self.unk_190 as i32 as u8, self.unk_194 as i32 as u8, 0];
            self.light_info_no_glow.radius = 300;
            play.light_ctx.set_info(self.light_node, self.light_info_no_glow);
        }
    }

    /// `func_809BC598`: a fire ball.
    fn ball(&mut self, play: &mut PlayState) {
        if let Some(k) = self.king_dodongo(play) {
            self.unk_158 = k.unk_1a2;
        }
        let mut quarter_turn = false;
        if PARAMS.get(self.actor.params) == 0 {
            // SFX_PLAY_AT_POS(&this->actor.projectedPos, NA_SE_EN_DODO_K_FIRE - SFX_FLAG).
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_K_FIRE - SFX_FLAG);
        }
        smooth_step_to_f(&mut self.actor.scale.x, self.unk_188, 0.3, 0.5, 0.0);
        self.set_scale();
        let (p, v) = (&mut self.actor.world_pos, self.actor.velocity);
        if p.x < -1390.0 && v.x < -10.0 {
            p.x = -1390.0;
            quarter_turn = true;
        }
        if p.x > -390.0 && v.x > 10.0 {
            p.x = -390.0;
            quarter_turn = true;
        }
        if p.z > -2804.0 && v.z > 10.0 {
            p.z = -2804.0;
            quarter_turn = true;
        }
        if p.z < -3804.0 && v.z < -10.0 {
            p.z = -3804.0;
            quarter_turn = true;
        }
        if quarter_turn {
            let r = &mut self.actor.world_rot.y;
            *r = if self.unk_158 == 0 { r.wrapping_add(0x4000) } else { r.wrapping_sub(0x4000) };
        }
        // DECR(this->unk_154) == 0.
        let decr = if self.unk_154 == 0 {
            0
        } else {
            self.unk_154 -= 1;
            self.unk_154
        };
        if decr == 0 {
            smooth_step_to_f(&mut self.unk_18c, 0.0, 1.0, 10.0, 0.0);
            if self.unk_18c < 10.0 {
                self.actor.kill();
            }
            return;
        }
        let burning = play.player.and_then(|h| play.actors.downcast::<crate::player::Player>(h)).map(|p| p.body_is_burning).unwrap_or(true);
        if !burning {
            let dist_to_burn = (self.actor.scale.x * 130.0) / 4.2000003;
            if self.actor.xyz_dist_to_player_sq < dist_to_burn * dist_to_burn {
                let mut timers = [0u8; 18];
                for t in timers.iter_mut() {
                    *t = play.rand.s16_offset(0, 200) as u8;
                }
                if let Some(p) = play.player.and_then(|h| play.actors.downcast_mut::<crate::player::Player>(h)) {
                    for (d, t) in p.body_flame_timers.iter_mut().zip(timers) {
                        *d = t;
                    }
                    p.body_is_burning = true;
                }
                actor_set_player_knockback_large(play, 20.0, self.actor.world_rot.y, 0.0, 8);
                log::debug!("POWER");
            }
        }
    }
}

fn bake_name(i: usize) -> String {
    format!("En_Bdfire/{i}")
}

/// The colours' segment (`gDPSetPrimColor`, `gDPSetEnvColor`), and the setup's.
const SEG_COLOR: u8 = 0x0E;
const SEG_SETUP: u8 = 0x0D;
/// The texture's segment (`gSPSegment(0x08, D_809BCB10[texIndex])`).
const SEG_TEX: u8 = 0x08;

/// One bake per texture: `Gfx_SetupDL_60NoCDXlu`, then `Gfx_SetupDL_20NoCD`, the combiner
/// `PRIMITIVE, ENVIRONMENT, TEXEL0, ENVIRONMENT` in colour and alpha, the colours dynamic, the
/// texture on segment 8, `DL_01D950`.
pub fn bakes() -> Vec<MeshBake> {
    let mut setup = setup_dl::setup_dl_60();
    setup.color_dither(G_CD_DISABLE);
    setup.0.extend(setup_dl::setup_dl_20().0);
    setup.color_dither(G_CD_DISABLE);
    let c = [cc_ab::PRIMITIVE, cc_ab::ENVIRONMENT, cc_c::TEXEL0, cc_d::ENVIRONMENT, cc_ab::PRIMITIVE, cc_ab::ENVIRONMENT, cc_c::TEXEL0, cc_d::ENVIRONMENT];
    setup.combine_lerp(c, c);
    setup.pipe_sync();
    setup.end();
    TEXTURES
        .iter()
        .enumerate()
        .map(|(i, tex)| MeshBake {
            name: bake_name(i),
            object: OBJECT.into(),
            segments: vec![
                (SEG_TEX, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() }),
                (SEG_SETUP, BakeSegment::Commands(setup.0.clone())),
                (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
            ],
            prelude: vec![SEG_SETUP, SEG_COLOR],
            body: BakeBody::DLists(vec![(OBJECT.into(), "object_kingdodongo_DL_01D950".into())]),
        })
        .collect()
}

impl ActorImpl for EnBdfire {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnBdfire_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.unk_156 = self.unk_156.wrapping_add(1);
        match self.action {
            Action::Flame => self.flame(play),
            Action::Ball => self.ball(play),
        }
        self.actor.move_forward();
    }

    /// `EnBdfire_Destroy`: the flame's light removed.
    fn destroy(&mut self, play: &mut PlayState) {
        if PARAMS.get(self.actor.params) < 0 {
            play.light_ctx.remove_light(self.light_node.take());
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![(self.unk_156 & 7) as u32];
        rs.values = vec![self.unk_18c];
        rs
    }

    /// `EnBdfire_DrawFire`: the actor's matrix with the camera's rotation (`Matrix_ReplaceRotation`,
    /// its scale kept), 11 up; texture `unk_156 & 7`; prim alpha `(s8)unk_18C`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let (Some(&tex), Some(&alpha)) = (rs.switches.first(), rs.values.first()) else { return };
        let m = oot_game::draw::replace_rotation(oot_game::play::actor_draw_matrix(rs), view.billboard) * Mat4::from_translation(Vec3::new(0.0, 11.0, 0.0));
        let mut sv = SegmentValues::default();
        sv.prim[SEG_COLOR as usize] = Some([255, 255, 100, alpha as i32 as i8 as u8]);
        sv.env[SEG_COLOR as usize] = Some([200, 0, 0, 0]);
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(tex as usize & 7))), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
