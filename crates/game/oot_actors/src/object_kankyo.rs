//! `Object_Kankyo` (`ovl_Object_Kankyo/z_object_kankyo.c`): the environment's effects. Params 0
//! is Kokiri Forest's fairy dust: up to 64 motes drifting in front of the camera, blinking,
//! wrapping round its view; the first 32 gather and circle Link when he stands still long
//! enough, as trails behind a leader. In the opening's Kokiri Forest layer (7) it also plays
//! Navi's flight: the wing hum by the camera's speed, her calls, and her crash into the fence
//! (cutscene frames 473, 583, 763, 771).
//!
//! Params 2 is the lightning of the royal tomb's opening (`ObjectKankyo_Lightning`): when the
//! cutscene cues channel 0 with id 2 it strikes, 7 frames of `sEffLightningTextures` (from the
//! first) at the cue's start position, tilted 20 degrees about x and z, scaled 2, 5, 2 and facing
//! the camera (`gEffLightningDL` after `Gfx_SetupDL_61Xlu`, prim `(255, 255, 255, 128)`, env
//! `(0, 255, 255, 128)`: `ObjectKankyo_DrawLightning`); then it waits for a cue of id 1 to strike
//! again.
//!
//! Params 4 is the spark over the Sun's Song grave in the same cutscene
//! (`ObjectKankyo_SunGraveSparkInit`, `_WaitForSunGraveSparkObject`, `_SunGraveSpark`): once
//! `object_spot02_objects` is loaded, size 7; while channel 1's cue is id 2,
//! `NA_SE_EN_BIRI_SPARK` each frame, its alpha 20 more (to 255) and its size smoothed towards 1.8
//! (`Math_SmoothStepToF(.., 0.5, 0.28, 0.01)`). Its draw (`ObjectKankyo_DrawSunGraveSpark`, which
//! also moves its brightness up and down by a seventh a frame and steps through `D_80BA5900`'s 8
//! textures) puts `object_spot02_objects_DL_009620` facing the camera on the cue's line, lerped
//! across the cue's frames, its colours from the brightness. It moves to the ITEMACTION category
//! (already its profile's).
//!
//! Not ported here: 3 (the snow: `ObjectKankyo_Snow` does nothing, and `ObjectKankyo_DrawSnow`
//! isn't drawn; it takes the one place params 0 takes too), 5 (Ganon's castle's beams:
//! `ObjectKankyo_InitBeams`, `_WaitForBeamObject`, `_Beams`, `_DrawBeams`; logged at init, it
//! does nothing).
//!
//! Params: `params` the whole params (0 the fairy dust, 2 the lightning, 3 the snow, 4 the Sun's
//! Song grave's spark, 5 Ganon's castle's beams: not ported; any other does nothing).
//! Flags: params 5 (not ported) reads EVENTCHKINF_BB to EVENTCHKINF_BF and EVENTCHKINF_AD, the
//! trials' beams (in the schema, marked indirect while it isn't ported).

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, smooth_step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_EV_NAVY_CRASH, NA_SE_EV_NAVY_FLY, NA_SE_VO_NA_HELLO_2, NA_SE_VO_NA_HELLO_3, NA_SE_VO_RT_THROW, SFX_FLAG, SfxPos};
use oot_game::env::PRECIP_SNOW_MAX;
use oot_game::gbi::setup_dl;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_OBJECT_KANKYO` (`actor_table.h`: 0x0097).
pub const ACTOR_OBJECT_KANKYO: i16 = 0x0097;

/// `Object_Kankyo_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_OBJECT_KANKYO, name: "Object_Kankyo", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA, object: "gameplay_keep", schema: &SCHEMA };

/// `this->actor.params` (`ObjectKankyo_Init`'s and `ObjectKankyo_Draw`'s switches): the effect.
pub const PARAMS: Field = Field::whole(
    "params",
    Kind::Named(&[(0, "the fairy dust"), (2, "the lightning"), (3, "the snow"), (4, "the Sun's Song grave's spark"), (5, "Ganon's castle's beams")]),
    "which effect",
);

/// Object_Kankyo's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(Flag::EventChkInf("EVENTCHKINF_BB", 0xBB)).reads().note("params 5: the forest trial's beam gone").indirect("params 5 isn't ported"),
        FlagUse::new(Flag::EventChkInf("EVENTCHKINF_BC", 0xBC)).reads().note("params 5: the water trial's beam gone").indirect("params 5 isn't ported"),
        FlagUse::new(Flag::EventChkInf("EVENTCHKINF_BD", 0xBD)).reads().note("params 5: the shadow trial's beam gone").indirect("params 5 isn't ported"),
        FlagUse::new(Flag::EventChkInf("EVENTCHKINF_BE", 0xBE)).reads().note("params 5: the fire trial's beam gone").indirect("params 5 isn't ported"),
        FlagUse::new(Flag::EventChkInf("EVENTCHKINF_BF", 0xBF)).reads().note("params 5: the light trial's beam gone").indirect("params 5 isn't ported"),
        FlagUse::new(Flag::EventChkInf("EVENTCHKINF_AD", 0xAD)).reads().note("params 5: the spirit trial's beam gone").indirect("params 5 isn't ported"),
    ],
    todo: false,
};

/// `SCENE_KOKIRI_FOREST` (Kokiri Forest), `ENTR_KOKIRI_FOREST_0`.
const SCENE_KOKIRI_FOREST: u16 = 0x55;
const ENTR_KOKIRI_FOREST_0: u16 = 0x00EE;

const BAKE: &str = "Object_Kankyo/dust_mote";
const SEG_SETUP: u8 = 0x0D;
const SEG_COLOR: u8 = 0x0E;

/// `ObjectKankyo_DrawFairies`' mote: `Gfx_SetupDL(SETUPDL_20)`, `gSun1Tex` on segment 8,
/// `gKokiriDustMoteMaterialDL`, then the mote's colours and `gKokiriDustMoteModelDL`; the
/// lightning's, one per texture; the spark's, one per texture.
pub fn bakes() -> Vec<MeshBake> {
    let keep = "gameplay_keep";
    let mut setup = setup_dl::setup_dl_20();
    setup.end();
    let mut v = vec![MeshBake {
        name: BAKE.into(),
        object: keep.into(),
        segments: vec![
            (SEG_SETUP, BakeSegment::Commands(setup.0)),
            (0x08, BakeSegment::Texture { file: keep.into(), symbol: "gSun1Tex".into() }),
            (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
        ],
        prelude: vec![SEG_SETUP, SEG_COLOR],
        body: BakeBody::DLists(vec![(keep.into(), "gKokiriDustMoteMaterialDL".into()), (keep.into(), "gKokiriDustMoteModelDL".into())]),
    }];
    // ObjectKankyo_DrawLightning: its colours (gDPSetPrimColor(.., 255, 255, 255, 128),
    // gDPSetEnvColor(.., 0, 255, 255, 128)), Gfx_SetupDL_61Xlu, the texture on segment 8,
    // gEffLightningDL.
    let mut setup61 = setup_dl::setup_dl_61();
    setup61.end();
    // G_SETPRIMCOLOR (0xFA), G_SETENVCOLOR (0xFB), G_ENDDL (0xDF).
    let colors = vec![(0xFA00_0000u32, u32::from_be_bytes([255, 255, 255, 128])), (0xFB00_0000u32, u32::from_be_bytes([0, 255, 255, 128])), (0xDF00_0000u32, 0u32)];
    for (i, tex) in S_EFF_LIGHTNING_TEXTURES.iter().enumerate() {
        v.push(MeshBake {
            name: lightning_bake(i),
            object: keep.into(),
            segments: vec![
                (SEG_COLOR, BakeSegment::Commands(colors.clone())),
                (SEG_SETUP, BakeSegment::Commands(setup61.0.clone())),
                (0x08, BakeSegment::Texture { file: keep.into(), symbol: (*tex).into() }),
            ],
            prelude: vec![SEG_COLOR, SEG_SETUP],
            body: BakeBody::DLists(vec![(keep.into(), "gEffLightningDL".into())]),
        });
    }
    // ObjectKankyo_DrawSunGraveSpark: Gfx_SetupDL_25Xlu (the bake's start), the colours,
    // D_80BA5900's texture on segment 8, object_spot02_objects_DL_009620.
    for (i, tex) in D_80BA5900.iter().enumerate() {
        v.push(MeshBake {
            name: spark_bake(i),
            object: SPOT02_OBJECTS.into(),
            segments: vec![(SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true }), (0x08, BakeSegment::Texture { file: SPOT02_OBJECTS.into(), symbol: (*tex).into() })],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![(SPOT02_OBJECTS.into(), "object_spot02_objects_DL_009620".into())]),
        });
    }
    v
}

/// `sEffLightningTextures`.
const S_EFF_LIGHTNING_TEXTURES: [&str; 8] = ["gEffLightning1Tex", "gEffLightning2Tex", "gEffLightning3Tex", "gEffLightning4Tex", "gEffLightning5Tex", "gEffLightning6Tex", "gEffLightning7Tex", "gEffLightning8Tex"];

/// `D_80BA5900`: the spark's textures (in `object_spot02_objects`).
const D_80BA5900: [&str; 8] = ["gEffSunGraveSpark1Tex", "gEffSunGraveSpark2Tex", "gEffSunGraveSpark3Tex", "gEffSunGraveSpark4Tex", "gEffSunGraveSpark5Tex", "gEffSunGraveSpark6Tex", "gEffSunGraveSpark7Tex", "gEffSunGraveSpark8Tex"];

/// `OBJECT_SPOT02_OBJECTS` (`object_table.h`: 0x00A1).
pub const OBJECT_SPOT02_OBJECTS: i16 = 0x00A1;
const SPOT02_OBJECTS: &str = "object_spot02_objects";
/// `NA_SE_EN_BIRI_SPARK` (`enemybank_table.h`: 0x389A).
pub const NA_SE_EN_BIRI_SPARK: u16 = 0x389A;

/// The cue channels: the lightning's, the spark's.
pub const CUE_LIGHTNING: usize = 0;
pub const CUE_SPARK: usize = 1;

fn lightning_bake(i: usize) -> String {
    format!("Object_Kankyo/lightning/{i}")
}

fn spark_bake(i: usize) -> String {
    format!("Object_Kankyo/sun_grave_spark/{i}")
}

/// What the lightning's or the spark's draw draws this frame: the bake, its matrix (the
/// billboard to be multiplied in), its colours (the spark's).
#[derive(Debug, Clone, PartialEq)]
pub struct CueDraw {
    pub bake: String,
    pub mtx: Mat4,
    pub prim: [u8; 4],
    pub env: [u8; 4],
}

/// `play->csCtx.actorCues[channel]` while a cutscene runs.
fn cue(play: &PlayState, channel: usize) -> Option<oot_game::cutscene::CsCmdActorCue> {
    if play.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE { play.cs_ctx.npc_actions[channel] } else { None }
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
    /// `ObjectKankyo_Lightning`.
    Lightning,
    /// `ObjectKankyo_SunGraveSparkInit`.
    SunGraveSparkInit,
    /// `ObjectKankyo_WaitForSunGraveSparkObject`.
    WaitForSunGraveSparkObject,
    /// `ObjectKankyo_SunGraveSpark`.
    SunGraveSpark,
    /// `ObjectKankyo_Snow` (nothing), and the kinds not ported.
    None,
}

pub struct ObjectKankyo {
    pub actor: Actor,
    pub action: Action,
    pub effects: [Effect; 64],
    /// `prevEyePos`.
    pub prev_eye_pos: Vec3,
    /// `requiredObjectSlot`, `requiredObjectLoaded` (params 4: `object_spot02_objects`).
    pub required_object_slot: Option<usize>,
    pub required_object_loaded: bool,
    /// What the last draw drew: each mote's place, scale and alpha.
    drawn: Vec<(Vec3, f32, u8)>,
    /// The lightning's or the spark's draw this frame (`draw_update`).
    pub cue_draw: Option<CueDraw>,
}

impl ObjectKankyo {
    /// `ObjectKankyo_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.room = -1;
        let mut action = Action::None;
        match PARAMS.get(actor.params) {
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
            2 => action = Action::Lightning,
            4 => {
                // effects[0].alpha = 0, .amplitude = 0 (as Effect::default leaves them);
                // Actor_ChangeCategory(.., ACTORCAT_ITEMACTION): the spawn files it there.
                actor.category = ACTORCAT_ITEMACTION;
                action = Action::SunGraveSparkInit;
            }
            5 => log::warn!("Object_Kankyo params 5 (Ganon's castle's beams): not ported"),
            p => log::debug!("Object_Kankyo params {p}: nothing (the C's default)"),
        }
        Box::new(ObjectKankyo { actor, action, effects: [Effect::default(); 64], prev_eye_pos: Vec3::ZERO, required_object_slot: None, required_object_loaded: false, drawn: Vec::new(), cue_draw: None })
    }

    /// `ObjectKankyo_Lightning`: with channel 0's cue, state 0 waits for its id 2 (the timer at
    /// 0), 1 counts 7 frames, 2 waits for its id 1 to start over.
    fn lightning(&mut self, play: &PlayState) {
        let Some(c) = cue(play, CUE_LIGHTNING) else { return };
        let e = &mut self.effects[0];
        match e.state {
            0 => {
                e.timer = 0;
                if c.action == 2 {
                    e.state += 1;
                }
            }
            1 => {
                e.timer += 1;
                if e.timer >= 7 {
                    e.state += 1;
                }
            }
            2 => {
                if c.action == 1 {
                    e.state = 0;
                }
            }
            _ => {}
        }
    }

    /// `ObjectKankyo_SunGraveSparkInit`: `object_spot02_objects`' slot (the C asserts it's
    /// there).
    fn sun_grave_spark_init(&mut self, play: &PlayState) {
        match play.object_ctx.get_index(OBJECT_SPOT02_OBJECTS) {
            Some(slot) => self.required_object_slot = Some(slot),
            None => log::error!("Object_Kankyo: object_spot02_objects isn't loaded (the C's ASSERT)"),
        }
        self.action = Action::WaitForSunGraveSparkObject;
    }

    /// `ObjectKankyo_WaitForSunGraveSparkObject`: once it's loaded, the spark at alpha 0, size 7.
    fn wait_for_sun_grave_spark_object(&mut self, play: &PlayState) {
        if self.required_object_slot.is_some_and(|s| play.object_ctx.is_loaded(s)) {
            self.required_object_loaded = true;
            self.effects[0].alpha = 0;
            // this->actor.objectSlot = this->requiredObjectSlot: the bakes draw with the object.
            self.effects[0].size = 7.0;
            self.action = Action::SunGraveSpark;
        }
    }

    /// `ObjectKankyo_SunGraveSpark`: while channel 1's cue is id 2, its sound, its alpha 20 more
    /// (to 255), its size smoothed towards 1.8.
    fn sun_grave_spark(&mut self, play: &mut PlayState) {
        if cue(play, CUE_SPARK).is_some_and(|c| c.action == 2) {
            self.actor.play_sfx_flagged(NA_SE_EN_BIRI_SPARK - SFX_FLAG);
            let e = &mut self.effects[0];
            if e.alpha as i16 + 20 > 255 {
                e.alpha = 255;
            } else {
                e.alpha += 20;
            }
            smooth_step_to_f(&mut e.size, 1.8, 0.5, 0.28, 0.01);
        }
    }

    /// `ObjectKankyo_DrawLightning`: in state 1, `sEffLightningTextures[timer]` at channel 0's
    /// cue's start, turned 20 degrees about x then z, scaled 2, 5, 2 (then the billboard, segment
    /// 1's matrix).
    fn draw_lightning(&self, play: &PlayState) -> Option<CueDraw> {
        let e = &self.effects[0];
        if e.state != 1 {
            return None;
        }
        // The C reads actorCues[0] without a check.
        let c = play.cs_ctx.npc_actions[CUE_LIGHTNING]?;
        let p = c.start_pos;
        let mtx = Mat4::from_translation(Vec3::new(p.x as f32, p.y as f32, p.z as f32)) * Mat4::from_rotation_x(20f32.to_radians()) * Mat4::from_rotation_z(20f32.to_radians()) * Mat4::from_scale(Vec3::new(2.0, 5.0, 2.0));
        Some(CueDraw { bake: lightning_bake((e.timer as usize).min(S_EFF_LIGHTNING_TEXTURES.len() - 1)), mtx, prim: [255, 255, 255, 128], env: [0, 255, 255, 128] })
    }

    /// `ObjectKankyo_DrawSunGraveSpark`: while channel 1's cue is id 2 (and the object's loaded),
    /// the brightness (`amplitude`) up a seventh a frame to 1 (state 0), then down to 0 below 0.1
    /// (state 1); the texture on to the next (`timer`, past 7 back to 0); the spark at the cue's
    /// line lerped across its frames, scaled `size`, facing the camera; prim `(105a + 150, 255,
    /// 105a + 150, alpha)`, env `(155a + 100, 255a, 255 - 255a, alpha)`.
    fn draw_sun_grave_spark(&mut self, play: &PlayState) -> Option<CueDraw> {
        let c = cue(play, CUE_SPARK)?;
        if c.action != 2 || !self.required_object_loaded {
            return None;
        }
        let e = &mut self.effects[0];
        if e.state == 0 {
            e.amplitude += 1.0 / 7.0;
            if e.amplitude >= 1.0 {
                e.amplitude = 1.0;
                e.state += 1;
            }
        } else {
            e.amplitude -= 1.0 / 7.0;
            if e.amplitude <= 0.1 {
                e.amplitude = 0.0;
                e.state = 0;
            }
        }
        e.timer += 1;
        if e.timer > 7 {
            e.timer = 0;
        }
        let start = Vec3::new(c.start_pos.x as f32, c.start_pos.y as f32, c.start_pos.z as f32);
        let end = Vec3::new(c.end_pos.x as f32, c.end_pos.y as f32, c.end_pos.z as f32);
        let weight = oot_game::env::lerp_weight(c.end_frame, c.start_frame, play.cs_ctx.frames);
        // LERP(start, end, weight).
        let pos = Vec3::new(start.x + (end.x - start.x) * weight, start.y + (end.y - start.y) * weight, start.z + (end.z - start.z) * weight);
        let mtx = Mat4::from_translation(pos) * Mat4::from_scale(Vec3::splat(e.size));
        let a = e.amplitude;
        let prim = [((105.0 * a) as i32 as u8).wrapping_add(150), 255, ((105.0 * a) as i32 as u8).wrapping_add(150), e.alpha];
        let env = [((155.0 * a) as i32 as u8).wrapping_add(100), (255.0 * a) as i32 as u8, 255u8.wrapping_sub((255.0 * a) as i32 as u8), e.alpha];
        Some(CueDraw { bake: spark_bake(e.timer as usize & 7), mtx, prim, env })
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
        match self.action {
            Action::Fairies => self.fairies(play),
            Action::Lightning => self.lightning(play),
            Action::SunGraveSparkInit => self.sun_grave_spark_init(play),
            Action::WaitForSunGraveSparkObject => self.wait_for_sun_grave_spark_object(play),
            Action::SunGraveSpark => self.sun_grave_spark(play),
            Action::None => {}
        }
    }

    /// `ObjectKankyo_Draw` by params: the lightning's (2) and the spark's (4) draws, which
    /// change the spark; `ObjectKankyo_DrawFairies`' changes (0): each mote's scale by its alpha
    /// (before this frame's change), then the alpha fading (the first 32 in while circling, out
    /// otherwise; the rest blinking by `alphaTimer`). Skipped with the main camera's `stateFlags &
    /// 0x100`.
    fn draw_update(&mut self, play: &mut PlayState) {
        self.drawn.clear();
        self.cue_draw = match PARAMS.get(self.actor.params) {
            2 => self.draw_lightning(play),
            4 => self.draw_sun_grave_spark(play),
            _ => None,
        };
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
        // ObjectKankyo_DrawLightning, ObjectKankyo_DrawSunGraveSpark: their matrices times the
        // billboard (the lightning's gSPMatrix(&D_01000000, G_MTX_MUL), the spark's
        // Matrix_Mult(&play->billboardMtxF)).
        if let Some(d) = &self.cue_draw {
            let mut sv = SegmentValues::default();
            sv.prim[SEG_COLOR as usize] = Some(d.prim);
            sv.env[SEG_COLOR as usize] = Some(d.env);
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(&d.bake)), transform: d.mtx * view.billboard, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
