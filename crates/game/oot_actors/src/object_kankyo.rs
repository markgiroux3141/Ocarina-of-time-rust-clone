//! `Object_Kankyo` (`ovl_Object_Kankyo/z_object_kankyo.c`): the environment's effects. Params 0
//! is Kokiri Forest's fairy dust: up to 64 motes drifting in front of the camera, blinking,
//! wrapping round its view; the first 32 gather and circle Link when he stands still long
//! enough, as trails behind a leader. In the opening's Kokiri Forest layer (7) it also plays
//! Navi's flight: the wing hum by the camera's speed, her calls, and her crash into the fence
//! (cutscene frames 473, 583, 763, 771).
//!
//! The other kinds (2 the lightning, 3 the snow, 4 the Sun's Song grave's spark, 5 Ganon's
//! castle's beams) aren't ported: they do nothing.

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, smooth_step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::audio::sfx::{NA_SE_EV_NAVY_CRASH, NA_SE_EV_NAVY_FLY, NA_SE_VO_NA_HELLO_2, NA_SE_VO_NA_HELLO_3, NA_SE_VO_RT_THROW, SFX_FLAG, SfxPos};
use oot_game::env::PRECIP_SNOW_MAX;
use oot_game::gbi::setup_dl;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_OBJECT_KANKYO` (`actor_table.h`: 0x0097).
pub const ACTOR_OBJECT_KANKYO: i16 = 0x0097;

/// `Object_Kankyo_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_OBJECT_KANKYO, name: "Object_Kankyo", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA, object: "gameplay_keep" };

/// `SCENE_KOKIRI_FOREST` (Kokiri Forest), `ENTR_KOKIRI_FOREST_0`.
const SCENE_KOKIRI_FOREST: u16 = 0x55;
const ENTR_KOKIRI_FOREST_0: u16 = 0x00EE;

const BAKE: &str = "Object_Kankyo/dust_mote";
const SEG_SETUP: u8 = 0x0D;
const SEG_COLOR: u8 = 0x0E;

/// `ObjectKankyo_DrawFairies`' mote: `Gfx_SetupDL(SETUPDL_20)`, `gSun1Tex` on segment 8,
/// `gKokiriDustMoteMaterialDL`, then the mote's colours and `gKokiriDustMoteModelDL`.
pub fn bakes() -> Vec<MeshBake> {
    let keep = "gameplay_keep";
    let mut setup = setup_dl::setup_dl_20();
    setup.end();
    vec![MeshBake {
        name: BAKE.into(),
        object: keep.into(),
        segments: vec![
            (SEG_SETUP, BakeSegment::Commands(setup.0)),
            (0x08, BakeSegment::Texture { file: keep.into(), symbol: "gSun1Tex".into() }),
            (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
        ],
        prelude: vec![SEG_SETUP, SEG_COLOR],
        body: BakeBody::DLists(vec![(keep.into(), "gKokiriDustMoteMaterialDL".into()), (keep.into(), "gKokiriDustMoteModelDL".into())]),
    }]
}

/// The overlay's statics: `sIsSpawned`, `sTrailingFairies`.
#[derive(Debug, Default)]
struct Statics {
    is_spawned: bool,
    trailing_fairies: i16,
}

/// `ObjectKankyoEffect`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Effect {
    pub state: u8,
    pub pos: Vec3,
    pub prev_pos: Vec3,
    pub base: Vec3,
    pub dir_phase: Vec3,
    pub speed: f32,
    pub target_speed: f32,
    pub alpha_timer: u16,
    pub angle: u16,
    pub alpha: u8,
    pub size: f32,
    pub angle_vel: u16,
    pub flight_radius: u16,
    pub amplitude: f32,
    pub timer: u16,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `ObjectKankyo_Fairies`.
    Fairies,
    /// The kinds not ported.
    None,
}

pub struct ObjectKankyo {
    pub actor: Actor,
    pub action: Action,
    pub effects: [Effect; 64],
    /// `prevEyePos`.
    pub prev_eye_pos: Vec3,
    /// What the last draw drew: each mote's place, scale and alpha.
    drawn: Vec<(Vec3, f32, u8)>,
}

impl ObjectKankyo {
    /// `ObjectKankyo_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.room = -1;
        let mut action = Action::None;
        match actor.params {
            0 => {
                let st = play.overlay_static::<Statics>(ACTOR_OBJECT_KANKYO);
                if !st.is_spawned {
                    action = Action::Fairies;
                    st.is_spawned = true;
                } else {
                    actor.kill();
                }
            }
            3 => {
                // ObjectKankyo_Snow does nothing; it takes the one place too.
                let st = play.overlay_static::<Statics>(ACTOR_OBJECT_KANKYO);
                if !st.is_spawned {
                    st.is_spawned = true;
                } else {
                    actor.kill();
                }
            }
            p => log::debug!("Object_Kankyo params {p}: not ported"),
        }
        Box::new(ObjectKankyo { actor, action, effects: [Effect::default(); 64], prev_eye_pos: Vec3::ZERO, drawn: Vec::new() })
    }

    /// `ObjectKankyo_Fairies`.
    fn fairies(&mut self, play: &mut PlayState) {
        if play.scene_id == SCENE_KOKIRI_FOREST && play.save.scene_layer == 7 {
            // Navi's wing hum, higher the faster the camera flies; her calls and the crash.
            let mut dist = self.prev_eye_pos.distance(play.view.eye);
            self.prev_eye_pos = play.view.eye;
            dist /= 30.0;
            if dist > 1.0 {
                dist = 1.0;
            }
            play.audio.func_800f436c(SfxPos::Default, NA_SE_EV_NAVY_FLY - SFX_FLAG, (0.4 * dist) + 0.6);
            match play.cs_ctx.frames {
                473 => play.audio.play_sfx_centered2(NA_SE_VO_NA_HELLO_3),
                583 => play.audio.func_800f4524(SfxPos::Default, NA_SE_VO_NA_HELLO_2, 32),
                763 => play.audio.play_sfx_centered(NA_SE_EV_NAVY_CRASH - SFX_FLAG),
                771 => play.audio.play_sfx_centered(NA_SE_VO_RT_THROW),
                _ => {}
            }
        }
        let env = &mut play.env_ctx;
        let snow_max = env.precipitation[PRECIP_SNOW_MAX];
        if snow_max < 64 && (play.save.entrance_index != ENTR_KOKIRI_FOREST_0 || play.save.scene_layer != 4 || snow_max != 0) {
            env.precipitation[PRECIP_SNOW_MAX] += 16;
        }
        let Some(player) = play.player.and_then(|h| play.actors.actor(h)).map(|a| (a.world_pos, a.velocity)) else { return };
        let (ppos, pvel) = player;
        let (eye, at) = (play.view.eye, play.view.at);
        for i in 0..(play.env_ctx.precipitation[PRECIP_SNOW_MAX] as usize).min(64) {
            // Spawn in front of the camera.
            let d = at - eye;
            let dist = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
            let fwd = Vec3::new(d.x / dist, d.y / dist, d.z / dist);
            match self.effects[i].state {
                0 => {
                    let rand = &mut play.rand;
                    let e = &mut self.effects[i];
                    e.base = Vec3::new(eye.x + fwd.x * 80.0, eye.y + fwd.y * 80.0, eye.z + fwd.z * 80.0);
                    e.pos.x = (rand.zero_one() - 0.5) * 160.0;
                    e.pos.y = 30.0;
                    e.pos.z = (rand.zero_one() - 0.5) * 160.0;
                    e.target_speed = rand.zero_one() * 1.6 + 0.5;
                    e.alpha = 0;
                    e.alpha_timer = (rand.zero_one() * 65535.0) as u16;
                    e.size = 0.1;
                    e.dir_phase.x = rand.zero_one() * 360.0;
                    e.dir_phase.y = rand.zero_one() * 360.0;
                    e.dir_phase.z = rand.zero_one() * 360.0;
                    e.state += 1;
                    e.timer = 0;
                }
                1 | 2 => self.update_mote(play, i, eye, fwd, ppos, pvel),
                3 => self.effects[i].state = 0,
                _ => {}
            }
        }
    }

    /// A mote's states 1 (drifting, blinking) and 2 (a trail circling Link).
    fn update_mote(&mut self, play: &mut PlayState, i: usize, eye: Vec3, fwd: Vec3, ppos: Vec3, pvel: Vec3) {
        let base = Vec3::new(eye.x + fwd.x * 80.0, eye.y + fwd.y * 80.0, eye.z + fwd.z * 80.0);
        let (base_x, base_y, base_z) = (base.x, base.y, base.z);
        let prev_leader = if i > 0 { Some((self.effects[i - 1].prev_pos, self.effects[i - 1].base)) } else { None };
        let mut trailing = play.overlay_static::<Statics>(ACTOR_OBJECT_KANKYO).trailing_fairies;
        let rand = &mut play.rand;
        let e = &mut self.effects[i];
        e.alpha_timer = e.alpha_timer.wrapping_add(1);
        e.prev_pos = e.pos;
        // The y velocity is -4 when Player stands on the ground.
        let mut player_moved = true;
        if pvel.x + pvel.y + pvel.z == -4.0 {
            player_moved = false;
            e.timer = e.timer.wrapping_add(1);
        } else {
            e.timer = 0;
        }
        if e.state == 1 {
            // The first 32 gather once Link has stood still a while.
            if i < 32 && !player_moved && e.timer > 256 {
                e.timer = 0;
                e.angle_vel = if rand.zero_one() < 0.5 { ((rand.zero_one() * 200.0) as i16 + 200) as u16 } else { (-((rand.zero_one() * 200.0) as i16 + 200)) as u16 };
                e.flight_radius = ((rand.zero_one() * 50.0) as i16 + 15) as u16;
                e.amplitude = (rand.zero_one() * 10.0 + 10.0) * 0.01;
                let random = rand.zero_one();
                trailing = if random < 0.2 {
                    1
                } else if random < 0.4 {
                    // (random < 0.2 again for 3: unreachable.)
                    7
                } else {
                    15
                };
                if (i as i16 & trailing) == 0 {
                    e.pos.y = 0.0;
                }
                e.state = 2;
                e.target_speed = 0.0;
            }
            smooth_step_to_f(&mut e.size, 0.1, 0.10, 0.001, 0.00001);
            smooth_step_to_f(&mut e.speed, e.target_speed, 0.5, 0.2, 0.02);
            e.pos.x += e.dir_phase.x.sin() * e.speed;
            e.pos.y += e.dir_phase.y.sin() * e.speed;
            e.pos.z += e.dir_phase.z.sin() * e.speed;
            match (i >> 1) & 3 {
                0 => {
                    e.dir_phase.x += 0.008;
                    e.dir_phase.y += 0.05 * rand.zero_one();
                    e.dir_phase.z += 0.015;
                }
                1 => {
                    e.dir_phase.x += 0.01 * rand.zero_one();
                    e.dir_phase.y += 0.05 * rand.zero_one();
                    e.dir_phase.z += 0.005 * rand.zero_one();
                }
                2 => {
                    e.dir_phase.x += 0.01 * rand.zero_one();
                    e.dir_phase.y += 0.4 * rand.zero_one();
                    e.dir_phase.z += 0.004 * rand.zero_one();
                }
                _ => {
                    // `0.01 * Rand_ZeroOne()` in double precision.
                    e.dir_phase.x = (e.dir_phase.x as f64 + 0.01 * rand.zero_one() as f64) as f32;
                    e.dir_phase.y += 0.08 * rand.zero_one();
                    e.dir_phase.z += 0.05 * rand.zero_one();
                }
            }
        } else if e.state == 2 {
            // Scatter when Link moves, or after a long while.
            if player_moved || e.timer > 1280 {
                e.timer = 0;
                e.state = 1;
                e.speed = 1.5;
                e.target_speed = rand.zero_one() * 1.6 + 0.5;
            }
            if (i as i16 & trailing) == 0 {
                // The leader: circling Link, bobbing.
                smooth_step_to_f(&mut e.size, 0.25, 0.1, 0.001, 0.00001);
                smooth_step_to_f(&mut e.base.x, ppos.x, 0.5, 1.0, 0.2);
                smooth_step_to_f(&mut e.base.y, ppos.y + 50.0, 0.5, 1.0, 0.2);
                smooth_step_to_f(&mut e.base.z, ppos.z, 0.5, 1.0, 0.2);
                let a = (e.angle as i16).wrapping_sub(0x8000u16 as i16);
                smooth_step_to_f(&mut e.pos.x, sin_s(a) * e.flight_radius as f32, 0.5, 2.0, 0.2);
                smooth_step_to_f(&mut e.pos.z, cos_s(a) * e.flight_radius as f32, 0.5, 2.0, 0.2);
                e.angle = e.angle.wrapping_add(e.angle_vel);
                e.pos.y += e.dir_phase.y.sin();
                e.dir_phase.x += 0.2 * rand.zero_one();
                e.dir_phase.y += e.amplitude;
                e.dir_phase.z += 0.1 * rand.zero_one();
                let a = (e.angle as i16).wrapping_sub(0x8000u16 as i16);
                e.pos.x = sin_s(a) * e.flight_radius as f32;
                e.pos.z = cos_s(a) * e.flight_radius as f32;
            } else if let Some((lp, lb)) = prev_leader {
                // A trailing mote: where the one before was, relative to its own base.
                smooth_step_to_f(&mut e.size, 0.1, 0.10, 0.001, 0.00001);
                smooth_step_to_f(&mut e.speed, 1.5, 0.5, 0.1, 0.0002);
                e.pos = Vec3::new(lp.x + (lb.x - e.base.x), lp.y + (lb.y - e.base.y), lp.z + (lb.z - e.base.z));
            }
        }
        if e.state != 2 {
            // Off the view's box: wrap round to the other side.
            let max = 130.0;
            let p = e.base + e.pos;
            if p.x - base_x > max || p.x - base_x < -max || p.y - base_y > max || p.y - base_y < -max || p.z - base_z > max || p.z - base_z < -max {
                if e.base.x + e.pos.x - base_x > max {
                    e.base.x = base_x - max;
                    e.pos.x = 0.0;
                }
                if e.base.x + e.pos.x - base_x < -max {
                    e.base.x = base_x + max;
                    e.pos.x = 0.0;
                }
                if e.base.y + e.pos.y - base_y > 50.0 {
                    e.base.y = base_y - 50.0;
                    e.pos.y = 0.0;
                }
                if e.base.y + e.pos.y - base_y < -50.0 {
                    e.base.y = base_y + 50.0;
                    e.pos.y = 0.0;
                }
                if e.base.z + e.pos.z - base_z > max {
                    e.base.z = base_z - max;
                    e.pos.z = 0.0;
                }
                if e.base.z + e.pos.z - base_z < -max {
                    e.base.z = base_z + max;
                    e.pos.z = 0.0;
                }
            }
        }
        play.overlay_static::<Statics>(ACTOR_OBJECT_KANKYO).trailing_fairies = trailing;
    }
}

impl ActorImpl for ObjectKankyo {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjectKankyo_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.action == Action::Fairies {
            self.fairies(play);
        }
    }

    /// `ObjectKankyo_DrawFairies`' changes: each mote's scale by its alpha (before this frame's
    /// change), then the alpha fading (the first 32 in while circling, out otherwise; the rest
    /// blinking by `alphaTimer`). Skipped with the main camera's `stateFlags & 0x100`.
    fn draw_update(&mut self, play: &mut PlayState) {
        self.drawn.clear();
        if self.action != Action::Fairies || play.game_camera.state_flags & 0x100 != 0 {
            return;
        }
        let n = (play.env_ctx.precipitation[PRECIP_SNOW_MAX] as usize).min(64);
        for (i, e) in self.effects.iter_mut().enumerate().take(n) {
            let alpha_scale = (e.alpha as f32 / 50.0).min(1.0);
            if i < 32 {
                if e.state != 2 {
                    if e.alpha > 0 {
                        e.alpha -= 1;
                    }
                } else if e.alpha < 100 {
                    e.alpha += 1;
                }
            } else if e.state != 2 {
                if (e.alpha_timer & 0x1F) < 16 {
                    if e.alpha < 235 {
                        e.alpha += 20;
                    }
                } else if e.alpha > 20 {
                    e.alpha -= 20;
                }
            } else if (e.alpha_timer & 0xF) < 8 {
                // Unreachable: the last 32 never circle.
                if e.alpha < 255 {
                    e.alpha = e.alpha.wrapping_add(100);
                }
            } else if e.alpha > 10 {
                e.alpha -= 10;
            }
            self.drawn.push((e.base + e.pos, e.size * alpha_scale, e.alpha));
        }
    }

    /// `ObjectKankyo_DrawFairies`: each mote on the billboard, turning 20 degrees a frame, gold
    /// (even) or blue (odd).
    fn draw(&self, _rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let spin = (play.gameplay_frames as f32 * 20.0).to_radians();
        for (i, &(p, s, alpha)) in self.drawn.iter().enumerate() {
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(s)) * view.billboard * Mat4::from_rotation_z(spin);
            let (prim, env) = if i & 1 == 0 { ([255, 255, 155, alpha], [250, 180, 0, alpha]) } else { ([255, 255, 255, alpha], [0, 100, 255, alpha]) };
            let mut sv = SegmentValues::default();
            sv.prim[SEG_COLOR as usize] = Some(prim);
            sv.env[SEG_COLOR as usize] = Some(env);
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
