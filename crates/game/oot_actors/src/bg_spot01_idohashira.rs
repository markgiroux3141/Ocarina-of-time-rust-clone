//! `Bg_Spot01_Idohashira` (`ovl_Bg_Spot01_Idohashira/z_bg_spot01_idohashira.c`): the wooden
//! crossbeam over Kakariko's well, a DynaPoly actor (`gKakarikoWellArchCol`, drawn with
//! `gKakarikoWellArchDL`, in `object_spot01_objects`).
//!
//! Outside a cutscene layer it stands still (`action` 0), and is gone for an adult once the
//! Nocturne of Shadow's cutscene has ended (`EVENTCHKINF_54`, set by
//! `CS_DEST_KAKARIKO_VILLAGE_NOCTURNE_PART_2`). In scene layer 4 (that cutscene: Kakariko burning)
//! it follows the cues on channel 2 (`func_808AB444`): cue 1 waits, cue 2 breaks it off (dust at
//! both its ends and their sparks, drawn once, `func_808AAF34`), spinning (0x3E8 a frame about y,
//! 0x7D0 about z) and flying from its home to the cue's end in a fall (`func_808AB29C`,
//! `func_808AB1DC`: the lerp `Environment_LerpWeight` across x and z, y a parabola with 30 a frame
//! of rise taken off), and when it lands its dust, sparks and `NA_SE_EV_WOODBOX_BREAK`
//! (`func_808AAE6C`); cue 3 kills it. There its shape is lowered 110 (`shape.yOffset` -1100, so its
//! placement is 110 up). In layer 6 it stands still; in every other cutscene layer it's killed.
//!
//! The debug registers (`kREG(6)` to `kREG(21)`) it adds are 0 (they start zeroed).
//!
//! The whole overlay is ported.
//!
//! Params: none read (Kakariko places it with 0xFFFF, unused).
//! Flags: reads EVENTCHKINF_54 (gone for an adult).

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::audio::sfx::SfxPos;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::effect::EffectInit;
use oot_game::effect::spark::EffectSparkInit;
use oot_game::env::lerp_weight;
use oot_game::event_chk_inf;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_BG_SPOT01_IDOHASHIRA` (`actor_table.h`: 0x0103).
pub const ACTOR_BG_SPOT01_IDOHASHIRA: i16 = 0x0103;
/// `OBJECT_SPOT01_OBJECTS` (`object_table.h`: 0x00F9).
pub const OBJECT: &str = "object_spot01_objects";

/// `Bg_Spot01_Idohashira_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT01_IDOHASHIRA, name: "Bg_Spot01_Idohashira", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// The placed params, read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed as 0xFFFF; read by nothing");

/// Bg_Spot01_Idohashira's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[FlagUse::new(event_chk_inf!(EVENTCHKINF_54)).reads().note("gone for an adult once set")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `EVENTCHKINF_54` (`save.h`: 0x54): the Nocturne of Shadow's cutscene seen.
pub const EVENTCHKINF_54: u16 = 0x54;
/// `NA_SE_EV_BOX_BREAK`, `NA_SE_EV_WOODBOX_BREAK` (`environmentbank_table.h`: 0x2839, 0x28AA).
pub const NA_SE_EV_BOX_BREAK: u16 = 0x2839;
pub const NA_SE_EV_WOODBOX_BREAK: u16 = 0x28AA;
/// The cue channel it reads (`BgSpot01Idohashira_GetCue(play, 2)`).
const CUE_CHANNEL: usize = 2;
/// `kREG(10) + 1100.0f`: the cutscene layer's lowered shape.
const DROP: f32 = 1100.0;

/// `action`: the index into `sActionFuncs`.
pub const ACTION_STILL: i32 = 0;
pub const ACTION_CUES: i32 = 1;
pub const ACTION_FALL: i32 = 2;
pub const ACTION_LANDED: i32 = 3;

pub struct BgSpot01Idohashira {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    /// `action`: 0 `func_808AB504` (nothing), 1 `func_808AB510` (the cues), 2 `func_808AB530`
    /// (falling), 3 `func_808AB570` (the cues).
    pub action: i32,
    /// `drawConfig`: 0, `func_808AB700`.
    pub draw_config: i32,
    /// `cueId`: the last cue taken.
    pub cue_id: u32,
    /// `unk_170`: the break's dust due at the next draw.
    pub unk_170: i32,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

fn load_collision(play: &PlayState) -> anyhow::Result<Arc<CollisionHeader>> {
    let assets = play.assets.as_ref().ok_or_else(|| anyhow::anyhow!("no asset pack"))?;
    Ok(Arc::new(assets.pack.collision(&keys::collision(OBJECT, "gKakarikoWellArchCol"))?))
}

/// `func_808AAD3C`: a black spark burst at `vec`, `arg2` divisions each way, 32 frames.
fn func_808aad3c(play: &mut PlayState, vec: Vec3, arg2: u32) {
    let effect = EffectSparkInit {
        position: [vec.x as i16, vec.y as i16, vec.z as i16],
        speed: 8.0,
        gravity: -1.0,
        u_div: arg2,
        v_div: arg2,
        color_start: [[0, 0, 0, 255]; 4],
        color_end: [[0, 0, 0, 0]; 4],
        timer: 0,
        duration: 32,
    };
    play.effect_add(EffectInit::Spark(effect));
}

impl BgSpot01Idohashira {
    /// `BgSpot01Idohashira_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: scale 0.1.
        actor.scale = Vec3::splat(0.1);
        let mut this = BgSpot01Idohashira { actor, bg: BG_ACTOR_MAX, action: ACTION_STILL, draw_config: 0, cue_id: 0, unk_170: 0 };
        // DynaPolyActor_Init(&this->dyna, 0), gKakarikoWellArchCol.
        match load_collision(play) {
            Ok(h) => this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0),
            Err(e) => log::error!("Bg_Spot01_Idohashira: {e:#}"),
        }
        if !play.is_cutscene_layer() {
            if play.save.get_event_chk_inf(EVENTCHKINF_54) && play.save.adult {
                this.actor.kill();
            } else {
                this.action = ACTION_STILL;
            }
        } else if play.save.scene_layer == 4 {
            this.action = ACTION_CUES;
            this.actor.shape_y_offset = -DROP;
            play.col.dyna.set_source(this.bg, source(&this.actor));
        } else if play.save.scene_layer == 6 {
            this.action = ACTION_STILL;
        } else {
            this.actor.kill();
        }
        Box::new(this)
    }

    /// `BgSpot01Idohashira_PlayBreakSfx1`.
    fn play_break_sfx1(play: &mut PlayState) {
        if let Some(h) = play.cur_actor {
            play.audio.play_sfx_at_pos(SfxPos::Actor(h), NA_SE_EV_BOX_BREAK);
        }
    }

    /// `BgSpot01Idohashira_PlayBreakSfx2`.
    fn play_break_sfx2(&self, play: &mut PlayState) {
        play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 60, NA_SE_EV_WOODBOX_BREAK);
    }

    /// `func_808AAE6C`: landed: the dust (350 across, 6 puffs), the sparks, the sound.
    fn func_808aae6c(&self, play: &mut PlayState) {
        // sp30.y += kREG(15): 0.
        let sp30 = self.actor.world_pos;
        func_80033480(play, sp30, 350.0, 5, 0x7D0, 0x320, 0);
        func_808aad3c(play, sp30, 5);
        self.play_break_sfx2(play);
    }

    /// `func_808AAF34` (from the draw, under its matrix): the break's dust and sparks at both ends
    /// (±1300, 200, 0), once.
    fn func_808aaf34(&mut self, play: &mut PlayState) {
        if self.unk_170 != 0 {
            let m = actor_draw_matrix(&RenderState::of(&self.actor));
            for x in [1300.0, -1300.0] {
                let dest = m.transform_point3(Vec3::new(x, 200.0, 0.0));
                func_80033480(play, dest, 80.0, 10, 1000, 0, 0);
                func_808aad3c(play, dest, 3);
            }
            self.unk_170 = 0;
            Self::play_break_sfx1(play);
        }
    }

    /// `BgSpot01Idohashira_NotInCsMode`.
    fn not_in_cs_mode(play: &PlayState) -> bool {
        play.cs_ctx.state == CS_STATE_IDLE
    }

    /// `BgSpot01Idohashira_GetCue`.
    fn get_cue(play: &PlayState, cue_channel: usize) -> Option<CsCmdActorCue> {
        if !Self::not_in_cs_mode(play) { play.cs_ctx.npc_actions[cue_channel] } else { None }
    }

    /// `func_808AB18C`: the spin.
    fn func_808ab18c(&mut self) {
        // shape.rot.x += kREG(6): 0.
        let r = &mut self.actor.shape_rot;
        r.y = r.y.wrapping_add(0x3E8);
        r.z = r.z.wrapping_add(0x7D0);
    }

    /// `func_808AB1DC`: the fall's height above `arg0` at frame `arg4` of `arg3` to `arg2`, ending
    /// at `arg1`: `((arg1 - arg0 - 30 t) / T^2) t^2 + 30 t`.
    pub fn func_808ab1dc(arg0: f32, arg1: f32, arg2: u16, arg3: u16, arg4: u16) -> f32 {
        let diff23 = (arg2 as i32 - arg3 as i32) as f32;
        let diff43 = (arg4 as i32 - arg3 as i32) as f32;
        if diff23 != 0.0 {
            // kREG(9) + 30.
            let reg_float = 30.0;
            let temp_f12 = reg_float * diff43;
            return ((((arg1 - arg0) - temp_f12) / (diff23 * diff23)) * diff43) * diff43 + temp_f12;
        }
        log::debug!("Bg_Spot01_Idohashira_Get_FreeFall Cannot divide by!!!!!!!!!!!!!!");
        0.0
    }

    /// `func_808AB29C`: along the cue from home; true at its end.
    fn func_808ab29c(&mut self, play: &PlayState) -> bool {
        let Some(cue) = Self::get_cue(play, CUE_CHANNEL) else { return false };
        let cur_frame = play.cs_ctx.frames;
        let temp_f0 = lerp_weight(cue.end_frame, cue.start_frame, cur_frame);
        let init_pos = self.actor.home_pos;
        let end_x = cue.end_pos.x as f32;
        let temp_y = (DROP / 10.0) + cue.end_pos.y as f32;
        let end_z = cue.end_pos.z as f32;
        let p = &mut self.actor.world_pos;
        p.x = ((end_x - init_pos.x) * temp_f0) + init_pos.x;
        p.y = Self::func_808ab1dc(init_pos.y, temp_y, cue.end_frame, cue.start_frame, cur_frame) + init_pos.y;
        p.z = ((end_z - init_pos.z) * temp_f0) + init_pos.z;
        temp_f0 >= 1.0
    }

    /// `func_808AB3E8`.
    fn func_808ab3e8(&mut self) {
        self.action = ACTION_CUES;
        self.draw_config = 0;
    }

    /// `func_808AB3F8`: broken off.
    fn func_808ab3f8(&mut self) {
        self.action = ACTION_FALL;
        self.draw_config = 0;
        self.unk_170 = 1;
    }

    /// `func_808AB414`: landed.
    fn func_808ab414(&mut self, play: &mut PlayState) {
        self.func_808aae6c(play);
        self.action = ACTION_LANDED;
        self.draw_config = 0;
    }

    /// `func_808AB444`: a new cue on channel 2.
    fn func_808ab444(&mut self, play: &PlayState) {
        let Some(cue) = Self::get_cue(play, CUE_CHANNEL) else { return };
        let next_cue_id = cue.action as u32;
        if next_cue_id != self.cue_id {
            match next_cue_id {
                1 => self.func_808ab3e8(),
                2 => self.func_808ab3f8(),
                3 => self.actor.kill(),
                _ => log::debug!("Bg_Spot01_Idohashira_Check_DemoMode: There is no such action!!!!!!!! ({next_cue_id})"),
            }
            self.cue_id = next_cue_id;
        }
    }

    /// `func_808AB530`.
    fn func_808ab530(&mut self, play: &mut PlayState) {
        self.func_808ab18c();
        if self.func_808ab29c(play) {
            self.func_808ab414(play);
        }
    }
}

impl ActorImpl for BgSpot01Idohashira {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgSpot01Idohashira_Update`: `sActionFuncs[action]`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            // func_808AB504.
            ACTION_STILL => {}
            // func_808AB510, func_808AB570.
            ACTION_CUES | ACTION_LANDED => self.func_808ab444(play),
            ACTION_FALL => self.func_808ab530(play),
            _ => {
                log::debug!("Bg_Spot01_Idohashira: The main mode is wrong!!!!!!!!!!!!!!!!!!!!!!!!!");
                return;
            }
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }
    /// `BgSpot01Idohashira_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }
    /// `func_808AB700`'s `func_808AAF34`, which changes the actor.
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.draw_config == 0 {
            self.func_808aaf34(play);
        }
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `BgSpot01Idohashira_Draw`: `sDrawFuncs[drawConfig]`, `func_808AB700`: `Gfx_SetupDL_25Opa`,
    /// `gKakarikoWellArchDL`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if self.draw_config != 0 {
            return;
        }
        crate::gfx_draw_dlist_opa(out, OBJECT, "gKakarikoWellArchDL", rs);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
