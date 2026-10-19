//! `Bg_Spot02_Objects` (`ovl_Bg_Spot02_Objects/z_bg_spot02_objects.c`): the graveyard's actors,
//! in `object_spot02_objects`, five kinds by params:
//!
//! - **0: the Shadow Temple's gate** (`object_spot02_objects_DL_012A50`, its collision
//!   `_Col_012BA4`): risen 255 from the start once its switch flag is set; else it waits for the
//!   flag (`func_808ACAFC`), then with the attention camera on it (`OnePointCutscene_Attention`,
//!   its focus 60 up) it rises 1 a frame (`func_808ACB58`: `NA_SE_EV_WALL_MOVE_SP` each frame,
//!   `NA_SE_EV_STONEDOOR_STOP` at the top).
//! - **1: the stone over Dampé's grave** (`_DL_0127C0`, `_Col_0128D8`; point lights ignored):
//!   gone for an adult.
//! - **2: the royal tomb's gravestone** (`_DL_0130B0`, `_Col_0133EC`): in the graveyard it waits
//!   for the tomb's opening (`gGraveyardTombOpeningChildCs`/`AdultCs`, started by `En_Okarina_Tag`
//!   type 6 on Zelda's Lullaby) to cue channel 3 with id 2 (`func_808AC908`): then
//!   `NA_SE_EV_GRAVE_EXPLOSION`, `EVENTCHKINF_1D`, the blast 50 in front of it and 30 up
//!   (`EffectSsBomb2_SpawnLayered`, scale 70 growing 30), and 25 frames (`func_808ACA08`): with 20
//!   left it's no longer drawn and 40 fragments fly (`EffectSsHahen_SpawnBurst`,
//!   `object_spot02_objects_DL_012D30`), at 0 it's gone. At the cutscene's frame 402 Link cries
//!   out (`NA_SE_VO_LI_DEMO_DAMAGE_KID`, or `_DAMAGE` for an adult). In the graveyard it's gone
//!   from the start once `EVENTCHKINF_1D` is set; elsewhere it stands still.
//! - **3: the lightning** (`func_808ACC34`, its draw `func_808ACCB8`): each frame the cutscene
//!   cues channel 0 with id 2 it counts one more of `D_808AD850`'s 12 textures, gone at the
//!   twelfth; drawn (`_DL_0126F0`, translucent) at the cue's start position and rotation while
//!   the cue is id 2, its prim colour white and env going from blue to yellow over the first 5,
//!   then both to green and dark over the last 7. (The update counts before the draw, so the
//!   first texture is never shown.) `NA_SE_EV_LIGHTNING` at the cutscene's frames 245 and 351.
//!   Gone from the start once `EVENTCHKINF_1D` is set.
//! - **4: the light beam** (`func_808AD3D4`, its draw `func_808AD450`): while channel 2's cue is id
//!   2 its timer counts from -12 to 32 (`NA_SE_IT_EXPLOSION_ICE` at 2), then it's gone; drawn
//!   (`_DL_0013F0`, translucent, prim `(255, 255, 170, 128)`, env `(150, 120, 0, 128)`, its two
//!   textures scrolling by the timer) at the cue's start position, turned by its x and y, 0.9 wide
//!   and long by the cue's `rot.z` read as degrees, lerped from the last such value across the
//!   cue's frames (`unk_170`, `unk_172`; the draw keeps them).
//!
//! Types 3 and 4 move to the ITEMACTION category. Types 0 to 2 are DynaPoly actors
//! (`DynaPolyActor_Init(0)`, scale 0.1). The whole overlay is ported.
//!
//! Params: `type` bits 0 to 7 (0 the Shadow Temple's gate, 1 Dampé's grave's stone, 2 the royal
//! tomb's gravestone, 3 the lightning, 4 the light beam), which the init writes over the params;
//! `unk_16B` bits 8 to 15 (type 0's switch flag; the graveyard places the gate with 0x1F).
//! `rot.x` and `rot.z`: a rotation (the desert colossus's gravestones, type 2, are placed tilted).
//! Flags: type 0 reads its switch flag `unk_16B`; type 2 reads and sets EVENTCHKINF_1D (the royal
//! tomb open); type 3 reads EVENTCHKINF_1D.

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_IGNORE_POINT_LIGHTS, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_ITEMACTION, ActorImpl, ActorProfile, audio_play_actor_sfx2, player_play_sfx};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::audio::sfx::{NA_SE_EV_STONEDOOR_STOP, SFX_FLAG};
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::env::lerp_weight;
use oot_game::event_chk_inf;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::scene_table::gfx_two_tex_scroll;

/// `ACTOR_BG_SPOT02_OBJECTS` (`actor_table.h`: 0x009C).
pub const ACTOR_BG_SPOT02_OBJECTS: i16 = 0x009C;
pub const OBJECT: &str = "object_spot02_objects";
/// `OBJECT_SPOT02_OBJECTS` (`object_table.h`: 0x00A1).
pub const OBJECT_SPOT02_OBJECTS: i16 = 0x00A1;

/// `Bg_Spot02_Objects_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT02_OBJECTS, name: "Bg_Spot02_Objects", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(thisx->params, 0, 8)`: the type, written over the params at init.
pub const TYPE: Field = Field::params(
    "type",
    0,
    8,
    Kind::Named(&[
        (0, "the Shadow Temple's gate"),
        (1, "Dampé's grave's stone (gone for an adult)"),
        (2, "the royal tomb's gravestone"),
        (3, "the lightning that blows the gravestone up"),
        (4, "the light beam"),
    ]),
    "which set piece",
);
/// `(u16)PARAMS_GET_NOMASK(thisx->params, 8)`, kept in the `u8` `unk_16B`.
pub const UNK_16B: Field = Field::params("unk_16B", 8, 8, Kind::SwitchFlag, "the gate's switch flag: set, it's open").when("type", &[0]);

/// `shape.rot.x`: placed tilted (the desert's gravestones), drawn and collided so.
pub const ROT_X: Field = Field::rot("rot.x", Source::RotX, 0, 16, Kind::Rotation, "its tilt about x");
/// `shape.rot.z`: likewise.
pub const ROT_Z: Field = Field::rot("rot.z", Source::RotZ, 0, 16, Kind::Rotation, "its tilt about z");

/// Bg_Spot02_Objects's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, UNK_16B, ROT_X, ROT_Z],
    flags: &[
        FlagUse::new(Flag::Switch("unk_16B")).reads().note("type 0: the gate open"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_1D)).reads().sets().note("type 2: set as the gravestone blows up, gone once set; type 3: gone once set"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `EVENTCHKINF_1D` (`save.h`: 0x1D): the royal tomb opened.
pub const EVENTCHKINF_1D: u16 = 0x1D;
/// `SCENE_GRAVEYARD` (`scene_table.h`: 0x53).
pub const SCENE_GRAVEYARD: u16 = 0x53;

/// `NA_SE_EV_GRAVE_EXPLOSION` (`environmentbank_table.h`: 0x28C2).
pub const NA_SE_EV_GRAVE_EXPLOSION: u16 = 0x28C2;
/// `NA_SE_EV_LIGHTNING` (`environmentbank_table.h`: 0x282E).
pub const NA_SE_EV_LIGHTNING: u16 = 0x282E;
/// `NA_SE_EV_WALL_MOVE_SP` (`playerbank_table.h`: 0x08CB).
pub const NA_SE_EV_WALL_MOVE_SP: u16 = 0x08CB;
/// `NA_SE_IT_EXPLOSION_ICE` (`itembank_table.h`: 0x183B).
pub const NA_SE_IT_EXPLOSION_ICE: u16 = 0x183B;
/// `NA_SE_VO_LI_DEMO_DAMAGE`, `NA_SE_VO_LI_DEMO_DAMAGE_KID` (`voicebank_table.h`: 0x681E,
/// 0x683E).
pub const NA_SE_VO_LI_DEMO_DAMAGE: u16 = 0x681E;
pub const NA_SE_VO_LI_DEMO_DAMAGE_KID: u16 = 0x683E;

/// The types (`params & 0xFF`).
pub const TYPE_SHADOW_GATE: i16 = 0;
pub const TYPE_DAMPE_STONE: i16 = 1;
pub const TYPE_TOMB_STONE: i16 = 2;
pub const TYPE_LIGHTNING: i16 = 3;
pub const TYPE_BEAM: i16 = 4;

/// The cue channels: the gravestone's, the lightning's, the beam's.
pub const CUE_TOMB_STONE: usize = 3;
pub const CUE_LIGHTNING: usize = 0;
pub const CUE_BEAM: usize = 2;

/// `BgSpot02Objects_Draw`'s `dLists` (types 0 to 2) and the collisions.
const DLISTS: [&str; 3] = ["object_spot02_objects_DL_012A50", "object_spot02_objects_DL_0127C0", "object_spot02_objects_DL_0130B0"];
const COLLISIONS: [&str; 3] = ["object_spot02_objects_Col_012BA4", "object_spot02_objects_Col_0128D8", "object_spot02_objects_Col_0133EC"];
/// The gravestone's fragments (`EffectSsHahen_SpawnBurst`'s `dList`).
pub const FRAGMENT_DL: &str = "object_spot02_objects_DL_012D30";

/// `D_808AD850`: the lightning's textures, on segment 8.
const D_808AD850: [&str; 12] = [
    "object_spot02_objects_Tex_0096B0",
    "object_spot02_objects_Tex_00A2B0",
    "object_spot02_objects_Tex_00AEB0",
    "object_spot02_objects_Tex_00BAB0",
    "object_spot02_objects_Tex_00C6B0",
    "object_spot02_objects_Tex_00D2B0",
    "object_spot02_objects_Tex_00DEB0",
    "object_spot02_objects_Tex_00EAB0",
    "object_spot02_objects_Tex_00F6B0",
    "object_spot02_objects_Tex_0102B0",
    "object_spot02_objects_Tex_010EB0",
    "object_spot02_objects_Tex_011AB0",
];

const SEG_TEX: u8 = 0x08;
const SEG_COLOR: u8 = 0x0E;
const BAKE_BEAM: &str = "Bg_Spot02_Objects/beam";

fn lightning_bake(i: usize) -> String {
    format!("Bg_Spot02_Objects/lightning/{i}")
}

/// `func_808AD450`'s scroll at `timer`: `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, 2 * timer, -3 *
/// timer, 32, 64, 1, 4 * timer, -6 * timer, 32, 64)` (the C passes them as `u32`).
pub fn beam_scroll(timer: i16) -> Vec<(u32, u32)> {
    let t = timer as i32;
    gfx_two_tex_scroll(0, (2 * t) as u32, (-3 * t) as u32, 32, 64, 1, (4 * t) as u32, (-6 * t) as u32, 32, 64)
}

/// The draws' bakes (types 0 to 2 draw the object's lists as they are, `Gfx_DrawDListOpa`):
/// - the lightning (`func_808ACCB8`), one per texture of `D_808AD850` on segment 8: after
///   `Gfx_SetupDL_25Xlu` (the bake's start), the colours dynamic, `object_spot02_objects_DL_0126F0`;
/// - the beam (`func_808AD450`): its colours (`gDPSetPrimColor(.., 255, 255, 170, 128)`,
///   `gDPSetEnvColor(.., 150, 120, 0, 128)`), its scroll on segment 8, `_DL_0013F0`.
pub fn bakes() -> Vec<MeshBake> {
    let mut v: Vec<MeshBake> = D_808AD850
        .iter()
        .enumerate()
        .map(|(i, tex)| MeshBake {
            name: lightning_bake(i),
            object: OBJECT.into(),
            segments: vec![(SEG_TEX, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() }), (SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true })],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![(OBJECT.into(), "object_spot02_objects_DL_0126F0".into())]),
        })
        .collect();
    // G_SETPRIMCOLOR (0xFA), G_SETENVCOLOR (0xFB), G_ENDDL (0xDF).
    let colors = vec![(0xFA00_0000u32, u32::from_be_bytes([255, 255, 170, 128])), (0xFB00_0000u32, u32::from_be_bytes([150, 120, 0, 128])), (0xDF00_0000u32, 0u32)];
    v.push(MeshBake {
        name: BAKE_BEAM.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_TEX, BakeSegment::Dynamic(beam_scroll(0))), (SEG_COLOR, BakeSegment::Commands(colors))],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(vec![(OBJECT.into(), "object_spot02_objects_DL_0013F0".into())]),
    });
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_808AC8FC`: nothing.
    Idle,
    /// `func_808AC908`: the gravestone waiting for its cue.
    TombStoneWait,
    /// `func_808ACA08`: the gravestone blown up.
    TombStoneExplode,
    /// `func_808ACAFC`: the gate waiting for its flag.
    GateWait,
    /// `func_808ACB58`: the gate rising.
    GateRise,
    /// `func_808ACC34`: the lightning.
    Lightning,
    /// `func_808AD3D4`: the beam.
    Beam,
}

/// `actor.draw`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawFn {
    /// `BgSpot02Objects_Draw`.
    Default,
    /// `func_808ACCB8`.
    Lightning,
    /// `func_808AD450`.
    Beam,
    /// `NULL` (the gravestone once it bursts).
    None,
}

/// What a lightning or beam draw draws this frame: the bake, its matrix, its colours (the
/// lightning's) or its scroll (the beam's timer).
#[derive(Debug, Clone, PartialEq)]
pub struct CueDraw {
    pub bake: String,
    pub mtx: Mat4,
    pub prim: [u8; 4],
    pub env: [u8; 4],
    pub timer: i16,
}

pub struct BgSpot02Objects {
    /// `dyna.actor`, `dyna.bgId`. `actor.params` is the type (`params & 0xFF`, as the init leaves
    /// it).
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    pub draw_fn: DrawFn,
    pub timer: i16,
    /// `unk_16A`: the lightning's frame.
    pub unk_16a: u8,
    /// `unk_16B`: the switch flag (params bits 8 to 15).
    pub unk_16b: u8,
    /// `unk_170`, `unk_172`: the beam's length, now and before (`0xFFFF` unset).
    pub unk_170: u16,
    pub unk_172: u16,
    /// The lightning's or the beam's draw this frame (`draw_update`).
    pub cue_draw: Option<CueDraw>,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

fn load_collision(play: &PlayState, symbol: &str) -> anyhow::Result<Arc<CollisionHeader>> {
    let assets = play.assets.as_ref().ok_or_else(|| anyhow::anyhow!("no asset pack"))?;
    Ok(Arc::new(assets.pack.collision(&keys::collision(OBJECT, symbol))?))
}

/// `play->csCtx.actorCues[channel]` while a cutscene runs (`csCtx.state != CS_STATE_IDLE`).
fn cue(play: &PlayState, channel: usize) -> Option<CsCmdActorCue> {
    if play.cs_ctx.state != CS_STATE_IDLE { play.cs_ctx.npc_actions[channel] } else { None }
}

/// `BINANG_TO_RAD` of a cue's rotation (`Vec3us`).
fn binang_to_rad(r: i16) -> f32 {
    (r as u16) as f32 * (std::f32::consts::PI / 32768.0)
}

/// `CAM_BINANG_TO_DEG` of a cue's `rot.z` (`Vec3us`), as the `u16` the beam keeps.
pub fn cam_binang_to_deg(r: i16) -> u16 {
    ((r as u16) as f32 * (360.0001525 / 65535.0)) as i32 as u16
}

/// `func_808ACCB8`'s colours at frame `unk_16A`: prim and env (both alpha 255).
pub fn lightning_colors(unk_16a: u8) -> ([u8; 4], [u8; 4]) {
    if unk_16a < 5 {
        let rate = unk_16a as f32 / 5.0;
        let red_env = (100.0 + 155.0 * rate) as i32 as u8;
        let blue_env = (255.0 - 155.0 * rate) as i32 as u8;
        ([255, 255, 255, 255], [red_env, 255, blue_env, 255])
    } else {
        let rate = (unk_16a as i32 - 5) as f32 / 7.0;
        let red_prim = (255.0 - (255.0 * rate)) as i32 as u8;
        let green_prim = (255.0 - (55.0 * rate)) as i32 as u8;
        let blue_prim = (255.0 - (255.0 * rate)) as i32 as u8;
        let red_env = (255.0 - (105.0 * rate)) as i32 as u8;
        let green_env = (255.0 - (255.0 * rate)) as i32 as u8;
        let blue_env = (100.0 + (100.0 * rate)) as i32 as u8;
        ([red_prim, green_prim, blue_prim, 255], [red_env, green_env, blue_env, 255])
    }
}

impl BgSpot02Objects {
    /// `BgSpot02Objects_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // DynaPolyActor_Init(&this->dyna, 0).
        let unk_16b = UNK_16B.get(actor.params) as u8;
        actor.params = TYPE.get(actor.params);
        let mut this = BgSpot02Objects { actor, bg: BG_ACTOR_MAX, action: Action::Idle, draw_fn: DrawFn::Default, timer: 0, unk_16a: 0, unk_16b, unk_170: 0, unk_172: 0, cue_draw: None };
        // schema: the type, written over the params above
        let ty = this.actor.params;
        match ty {
            TYPE_SHADOW_GATE | TYPE_DAMPE_STONE | TYPE_TOMB_STONE => {
                // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100).
                this.actor.scale = Vec3::splat(0.1);
                if ty == TYPE_SHADOW_GATE {
                    if play.flags.get_switch(this.unk_16b as i32) {
                        this.action = Action::Idle;
                        this.actor.world_pos.y += 255.0;
                    } else {
                        this.action = Action::GateWait;
                    }
                } else if ty == TYPE_DAMPE_STONE {
                    this.action = Action::Idle;
                    this.actor.flags |= ACTOR_FLAG_IGNORE_POINT_LIGHTS;
                } else if play.scene_id == SCENE_GRAVEYARD {
                    this.action = Action::TombStoneWait;
                } else {
                    this.action = Action::Idle;
                }
                match load_collision(play, COLLISIONS[ty as usize]) {
                    Ok(h) => this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0),
                    Err(e) => log::error!("Bg_Spot02_Objects: {e:#}"),
                }
                if (play.save.get_event_chk_inf(EVENTCHKINF_1D) && play.scene_id == SCENE_GRAVEYARD && ty == TYPE_TOMB_STONE) || (play.save.adult && ty == TYPE_DAMPE_STONE) {
                    this.actor.kill();
                }
            }
            TYPE_LIGHTNING => {
                this.unk_16a = 0;
                // Actor_ChangeCategory(.., ACTORCAT_ITEMACTION): the spawn files it there.
                this.actor.category = ACTORCAT_ITEMACTION;
                this.action = Action::Lightning;
                this.draw_fn = DrawFn::Lightning;
                if play.save.get_event_chk_inf(EVENTCHKINF_1D) {
                    this.actor.kill();
                }
            }
            TYPE_BEAM => {
                this.timer = -12;
                this.unk_170 = 0xFFFF;
                this.actor.category = ACTORCAT_ITEMACTION;
                this.action = Action::Beam;
                this.draw_fn = DrawFn::Beam;
            }
            t => {
                // The C leaves actionFunc NULL (its update would crash).
                log::warn!("Bg_Spot02_Objects: type {t} has no action");
            }
        }
        Box::new(this)
    }

    /// `func_808AC908`: the gravestone's cue: the explosion.
    fn func_808ac908(&mut self, play: &mut PlayState) {
        let Some(c) = cue(play, CUE_TOMB_STONE) else { return };
        if c.action == 2 {
            audio_play_actor_sfx2(play, NA_SE_EV_GRAVE_EXPLOSION);
            play.save.set_event_chk_inf(EVENTCHKINF_1D);
            self.timer = 25;
            let a = &self.actor;
            let pos = Vec3::new((sin_s(a.shape_rot.y) * 50.0) + a.world_pos.x, a.world_pos.y + 30.0, (cos_s(a.shape_rot.y) * 50.0) + a.world_pos.z);
            play.with_ss(|s| s.bomb2_spawn_layered(pos, Vec3::ZERO, Vec3::ZERO, 70, 30));
            self.action = Action::TombStoneExplode;
        }
    }

    /// `func_808ACA08`: 25 frames: at 20 left the stone's draw goes and its fragments fly; at 0
    /// it's gone. Link's cry at the cutscene's frame 402.
    fn func_808aca08(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 20 {
            self.draw_fn = DrawFn::None;
            let pos = self.actor.world_pos;
            play.with_ss(|s| s.hahen_spawn_burst(pos, 30.0, 0, 25, 5, 40, OBJECT_SPOT02_OBJECTS, 20, Some((OBJECT, FRAGMENT_DL))));
        } else if self.timer == 0 {
            self.actor.kill();
        }
        if play.cs_ctx.frames == 402
            && let Some(p) = play.player
        {
            player_play_sfx(play, p, if !play.save.adult { NA_SE_VO_LI_DEMO_DAMAGE_KID } else { NA_SE_VO_LI_DEMO_DAMAGE });
        }
    }

    /// `func_808ACAFC`: the gate waiting for its switch flag; then the attention camera on it.
    fn func_808acafc(&mut self, play: &mut PlayState) {
        if play.flags.get_switch(self.unk_16b as i32) {
            self.actor.set_focus(60.0);
            // OnePointCutscene_Attention(play, &this->dyna.actor).
            if let Some(h) = play.cur_actor {
                let me = play.cam_actor_of(h, &self.actor);
                play.onepoint_attention(me);
            }
            self.action = Action::GateRise;
        }
    }

    /// `func_808ACB58`: the gate rising 1 a frame to 255 above home.
    fn func_808acb58(&mut self, play: &mut PlayState) {
        let target = self.actor.home_pos.y + 255.0;
        if step_to_f(&mut self.actor.world_pos.y, target, 1.0) {
            audio_play_actor_sfx2(play, NA_SE_EV_STONEDOOR_STOP);
            self.action = Action::Idle;
        } else {
            self.actor.play_sfx_flagged(NA_SE_EV_WALL_MOVE_SP - SFX_FLAG);
        }
    }

    /// `func_808ACC34`: the lightning's frames while channel 0's cue is id 2 (gone at 12); its
    /// sounds at the cutscene's frames 245 and 351.
    fn func_808acc34(&mut self, play: &mut PlayState) {
        if cue(play, CUE_LIGHTNING).is_some_and(|c| c.action == 2) {
            self.unk_16a = self.unk_16a.wrapping_add(1);
            if self.unk_16a >= 12 {
                self.actor.kill();
            }
        }
        if play.cs_ctx.frames == 245 || play.cs_ctx.frames == 351 {
            play.audio.play_sfx_centered2(NA_SE_EV_LIGHTNING);
        }
    }

    /// `func_808AD3D4`: the beam's timer while channel 2's cue is id 2: the sound at 2, gone at 32.
    fn func_808ad3d4(&mut self, play: &mut PlayState) {
        if cue(play, CUE_BEAM).is_some_and(|c| c.action == 2) {
            if self.timer == 2 {
                audio_play_actor_sfx2(play, NA_SE_IT_EXPLOSION_ICE);
            }
            if self.timer < 32 {
                self.timer += 1;
            } else {
                self.actor.kill();
            }
        }
    }

    /// `func_808ACCB8`: while channel 0's cue is id 2, the lightning's texture `unk_16A` at the
    /// cue's start, turned by its x, y and z (`Matrix_Scale(1, 1, 1)`).
    fn func_808accb8(&self, play: &PlayState) -> Option<CueDraw> {
        let c = cue(play, CUE_LIGHTNING).filter(|c| c.action == 2)?;
        let (prim, env) = lightning_colors(self.unk_16a);
        let p = c.start_pos;
        let mtx = Mat4::from_translation(Vec3::new(p.x as f32, p.y as f32, p.z as f32)) * Mat4::from_rotation_x(binang_to_rad(c.rot[0])) * Mat4::from_rotation_y(binang_to_rad(c.rot[1])) * Mat4::from_rotation_z(binang_to_rad(c.rot[2]));
        Some(CueDraw { bake: lightning_bake((self.unk_16a as usize).min(D_808AD850.len() - 1)), mtx, prim, env, timer: 0 })
    }

    /// `func_808AD450`: with channel 2's cue, its `rot.z` in degrees taken as the beam's length
    /// (`unk_170`, the last one kept in `unk_172`; both at the first), lerped across the cue's
    /// frames; while the cue is id 2, the beam at its start, turned by its x and y, scaled 0.9,
    /// 0.9 and a tenth of the length.
    fn func_808ad450(&mut self, play: &PlayState) -> Option<CueDraw> {
        let c = cue(play, CUE_BEAM)?;
        let temp_v1 = cam_binang_to_deg(c.rot[2]);
        if self.unk_170 != temp_v1 {
            if self.unk_170 == 0xFFFF {
                self.unk_170 = temp_v1;
                self.unk_172 = temp_v1;
            } else {
                self.unk_172 = self.unk_170;
                self.unk_170 = temp_v1;
            }
        }
        let lerp = lerp_weight(c.end_frame, c.start_frame, play.cs_ctx.frames);
        if c.action != 2 {
            return None;
        }
        let p = c.start_pos;
        let len = (((self.unk_170 as i32 - self.unk_172 as i32) as f32 * lerp) + self.unk_172 as f32) * 0.1;
        let mtx = Mat4::from_translation(Vec3::new(p.x as f32, p.y as f32, p.z as f32)) * Mat4::from_rotation_x(binang_to_rad(c.rot[0])) * Mat4::from_rotation_y(binang_to_rad(c.rot[1])) * Mat4::from_scale(Vec3::new(0.9, 0.9, len));
        Some(CueDraw { bake: BAKE_BEAM.into(), mtx, prim: [255, 255, 170, 128], env: [150, 120, 0, 128], timer: self.timer })
    }
}

impl ActorImpl for BgSpot02Objects {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgSpot02Objects_Update`: the action; the new position for `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Idle => {}
            Action::TombStoneWait => self.func_808ac908(play),
            Action::TombStoneExplode => self.func_808aca08(play),
            Action::GateWait => self.func_808acafc(play),
            Action::GateRise => self.func_808acb58(play),
            Action::Lightning => self.func_808acc34(play),
            Action::Beam => self.func_808ad3d4(play),
        }
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(self.bg, source(&self.actor));
        }
    }

    /// `BgSpot02Objects_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// The lightning's and the beam's draws, from the cues (the beam's keeps its lengths).
    fn draw_update(&mut self, play: &mut PlayState) {
        self.cue_draw = match self.draw_fn {
            DrawFn::Lightning => self.func_808accb8(play),
            DrawFn::Beam => self.func_808ad450(play),
            DrawFn::Default | DrawFn::None => None,
        };
    }

    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }

    /// `BgSpot02Objects_Draw` (`Gfx_DrawDListOpa` of the type's list), `func_808ACCB8`,
    /// `func_808AD450`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        match self.draw_fn {
            DrawFn::Default => {
                // schema: the type, written over the params at init
                if let Some(dl) = DLISTS.get(self.actor.params as usize) {
                    crate::gfx_draw_dlist_opa(out, OBJECT, dl, rs);
                }
            }
            DrawFn::Lightning => {
                if let Some(d) = &self.cue_draw {
                    let mut sv = SegmentValues::default();
                    sv.prim[SEG_COLOR as usize] = Some(d.prim);
                    sv.env[SEG_COLOR as usize] = Some(d.env);
                    out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(&d.bake)), transform: d.mtx, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
                }
            }
            DrawFn::Beam => {
                if let Some(d) = &self.cue_draw {
                    let mut sv = SegmentValues::default();
                    sv.read(SEG_TEX, &beam_scroll(d.timer));
                    out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(&d.bake)), transform: d.mtx, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
                }
            }
            DrawFn::None => {}
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
