//! `Oceff_Spot` (`ovl_Oceff_Spot/z_oceff_spot.c`): the Sun's Song's effect (`MSGMODE_SONG_PLAYED`
//! spawns it at Link). A cylinder of light round Link grows over 20 frames (`unk_174` to 1 by
//! 0.05: the width eased by `(1 - cos(unk_174 * pi)) / 2`), stays 60, shrinks over 20; two point
//! lights shine with it (above Link and in front of him), and the scene darkens
//! (`Environment_AdjustLights`). When it's gone the Sun's Song acts (`SUNSSONG_START`, which
//! `Interface_Update` carries out), unless time already runs at the song's 400, the song is
//! disabled here (`Message_StartOcarinaSunsSongDisabled`) or Epona's rented for Ingo's race: then
//! the ocarina is only put away (`OCARINA_MODE_04`).
//!
//! The whole overlay is ported, its destroy's `Magic_Reset` too; Nayru's Love's restore never
//! applies (the spells aren't ported: `nayrusLoveTimer` stays 0).
//!
//! Params: none (`Message_DrawMain` spawns it with 0).
//! Flags: sets `gSaveContext.sunsSongState` (`SUNSSONG_START`) as it goes; reads the Ingo race's
//! state (`GET_EVENTINF_INGO_RACE_STATE`, `eventInf[0]` bits 0 to 3: no Sun's Song while Epona's
//! rented).

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Flag, FlagUse};
use oot_game::lights::{LightInfo, LightNode};
use oot_game::message::{OCARINA_ACTION_CHECK_NOWARP_DONE, OCARINA_MODE_04, OCARINA_MODE_08};
use oot_game::pack::{MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

use crate::oceff_wipe::{SEG_SCROLL, destroy_magic_reset, wipe_bake};

/// `ACTOR_OCEFF_SPOT` (`actor_table.h`: 0x017E).
pub const ACTOR_OCEFF_SPOT: i16 = 0x017E;

/// `Oceff_Spot_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_OCEFF_SPOT, name: "Oceff_Spot", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA, object: "gameplay_keep", schema: &SCHEMA };

/// Oceff_Spot's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[],
    flags: &[
        FlagUse::new(Flag::Other("gSaveContext.sunsSongState")).sets().note("SUNSSONG_START as it ends: the Sun's Song acts"),
        FlagUse::new(Flag::Other("GET_EVENTINF_INGO_RACE_STATE()")).reads().note("Epona rented: no Sun's Song"),
    ],
    todo: false,
};

pub const BAKE: &str = "Oceff_Spot/cylinder";

/// `SUNSSONG_START` (`save.h`), `INGO_RACE_STATE_HORSE_RENTAL_PERIOD`.
const SUNSSONG_START: i16 = 1;
const INGO_RACE_STATE_HORSE_RENTAL_PERIOD: u16 = 1;

/// `OceffSpot_Draw`: `Gfx_SetupDL_25Xlu`, the actor's matrix, `sCylinderMaterialDL`, the scroll,
/// `sCylinderModelDL`.
pub fn bakes() -> Vec<MeshBake> {
    vec![wipe_bake(BAKE, "ovl_Oceff_Spot", Vec::new(), &["sCylinderMaterialDL"], &["sCylinderModelDL"], scroll(0))]
}

/// `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, scroll * 2, scroll * -2, 32, 32, 1, 0, scroll * -8, 32, 32)`.
fn scroll(scroll: u32) -> Vec<(u32, u32)> {
    let m = |k: i32| scroll.wrapping_mul(k as u32);
    gfx_two_tex_scroll(0, m(2), m(-2), 32, 32, 1, 0, m(-8), 32, 32)
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    GrowCylinder,
    Wait,
    End,
}

pub struct OceffSpot {
    pub actor: Actor,
    pub light_node1: Option<LightNode>,
    pub light_info1: LightInfo,
    pub light_node2: Option<LightNode>,
    pub light_info2: LightInfo,
    pub unk_174: f32,
    pub timer: u16,
    pub action: Action,
}

impl OceffSpot {
    /// `OceffSpot_Init`: no size yet (`ICHAIN_VEC3F_DIV1000(scale, 0)`), culled past 1500, the two
    /// lights dark; 2.4 high under a fixed camera, else 0.3.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let p = actor.world_pos;
        let info = LightInfo::point_no_glow(p.x as i16, p.y as i16, p.z as i16, [0, 0, 0], 0);
        let light_node1 = play.light_ctx.insert_light(info);
        let light_node2 = play.light_ctx.insert_light(info);
        let mut this = OceffSpot { actor, light_node1, light_info1: info, light_node2, light_info2: info, unk_174: 0.0, timer: 0, action: Action::GrowCylinder };
        this.actor.scale = Vec3::ZERO;
        this.actor.culling_volume_distance = 1500.0;
        this.actor.scale.y = if play.scene_cam_type != oot_game::scene::SCENE_CAM_TYPE_DEFAULT { 2.4 } else { 0.3 };
        this.unk_174 = 0.0;
        Box::new(this)
    }

    /// `OceffSpot_End`: shrinking; at nothing, killed, and the Sun's Song starts (or the ocarina
    /// ends).
    fn end(&mut self, play: &mut PlayState) {
        if self.unk_174 > 0.0 {
            self.unk_174 -= 0.05;
            return;
        }
        self.actor.kill();
        // schema: GET_EVENTINF_INGO_RACE_STATE(), a flag of the schema's
        let renting = play.save.event_inf[0] & 0xF == INGO_RACE_STATE_HORSE_RENTAL_PERIOD;
        if play.env_statics.time_speed != 400 && play.msg_ctx.disable_suns_song == 0 && !renting {
            if play.msg_ctx.ocarina_action != OCARINA_ACTION_CHECK_NOWARP_DONE || play.msg_ctx.ocarina_mode != OCARINA_MODE_08 {
                play.save.suns_song_state = SUNSSONG_START;
                log::debug!("z_oceff_spot  Sun's Song Flag");
            }
        } else {
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            log::debug!("z_oceff_spot  Ocarina finished");
        }
    }

    /// `OceffSpot_Wait`: 60 frames.
    fn wait(&mut self) {
        if self.timer > 0 {
            self.timer -= 1;
        } else {
            self.action = Action::End;
        }
    }

    /// `OceffSpot_GrowCylinder`.
    fn grow_cylinder(&mut self) {
        if self.unk_174 < 1.0 {
            self.unk_174 += 0.05;
        } else {
            self.action = Action::Wait;
            self.timer = 60;
        }
    }
}

impl ActorImpl for OceffSpot {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `OceffSpot_Update`.
    fn update(&mut self, play: &mut PlayState) {
        let player = play.player.and_then(|h| play.actors.actor(h)).map(|p| (p.world_pos, p.shape_rot.y));
        // (1 - cosf(unk_174 * M_PI)) * 0.5f, the product a double.
        let mut temp = (1.0 - ((self.unk_174 as f64 * std::f64::consts::PI) as f32).cos()) * 0.5;
        match self.action {
            Action::GrowCylinder => self.grow_cylinder(),
            Action::Wait => self.wait(),
            Action::End => self.end(play),
        }
        self.actor.scale.z = 0.42 * temp;
        self.actor.scale.x = 0.42 * temp;
        let Some((ppos, pyaw)) = player else { return };
        self.actor.world_pos = ppos;
        self.actor.world_pos.y += 5.0;
        temp = (2.0 - self.unk_174) * self.unk_174;
        play.environment_adjust_lights(temp * 0.5, 880.0, 0.2, 0.9);
        let p = self.actor.world_pos;
        let color = [(255.0 * temp) as i32 as u8, (255.0 * temp) as i32 as u8, (200.0 * temp) as i32 as u8];
        let radius = (100.0 * temp) as i16;
        // (s16)world.pos.y + 55.0f, and the like: the s16 then the float sum, back to s16.
        self.light_info1 = LightInfo::point_no_glow(p.x as i16, (p.y as i16 as f32 + 55.0) as i16, p.z as i16, color, radius);
        play.light_ctx.set_info(self.light_node1, self.light_info1);
        self.light_info2 = LightInfo::point_no_glow((p.x as i16 as f32 + sin_s(pyaw) * 20.0) as i16, (p.y as i16 as f32 + 20.0) as i16, (p.z as i16 as f32 + cos_s(pyaw) * 20.0) as i16, color, radius);
        play.light_ctx.set_info(self.light_node2, self.light_info2);
    }
    /// `OceffSpot_Destroy`: the lights out.
    fn destroy(&mut self, play: &mut PlayState) {
        play.light_ctx.remove_light(self.light_node1.take());
        play.light_ctx.remove_light(self.light_node2.take());
        destroy_magic_reset(play, PROFILE.name);
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// `OceffSpot_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let mut sv = SegmentValues::default();
        sv.read(SEG_SCROLL, &scroll(play.state_frames & 0xFFFF));
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: actor_draw_matrix(rs), bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
