//! `Bg_Spot16_Doughnut` (`ovl_Bg_Spot16_Doughnut/z_bg_spot16_doughnut.c`): the ring of cloud
//! round Death Mountain's summit, turning; fiery red for an adult until `EVENTCHKINF_2F` (Darunia's
//! cutscene fades it, on cue 2 of channel 2). Smaller where the mountain is in the background
//! (Kakariko, the Temple of Time's outside). Params 1 to 4 are the expanding rings (the
//! creation's), which grow, fade and go.
//!
//! Ported whole (GAME-06 milestone 1b). The draws are bakes (ADR 0006), the env alpha and the
//! fiery scroll dynamic. The culling volume isn't used, as for every actor.

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

/// `ACTOR_BG_SPOT16_DOUGHNUT` (`actor_table.h`: 0x00E5).
pub const ACTOR_BG_SPOT16_DOUGHNUT: i16 = 0x00E5;
pub const OBJECT: &str = "object_efc_doughnut";

/// `Bg_Spot16_Doughnut_Profile`: `ACTORCAT_PROP`, `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT16_DOUGHNUT, name: "Bg_Spot16_Doughnut", category: ACTORCAT_PROP, flags: 0, object: OBJECT };

/// `sScales`: the expanding rings' starting scales (× 1e-4).
const S_SCALES: [i16; 5] = [0, 0, 70, 210, 300];

/// `scene_table.h`.
const SCENE_TEMPLE_OF_TIME_EXTERIOR_DAY: u16 = 0x23;
const SCENE_TEMPLE_OF_TIME_EXTERIOR_NIGHT: u16 = 0x24;
const SCENE_TEMPLE_OF_TIME_EXTERIOR_RUINS: u16 = 0x25;
const SCENE_KAKARIKO_VILLAGE: u16 = 0x52;

/// `EVENTCHKINF_2F` (`save.h`): the mountain's fire put out.
const EVENTCHKINF_2F: u16 = 0x2F;

pub struct BgSpot16Doughnut {
    pub actor: Actor,
    pub fire_flag: u16,
    pub env_color_alpha: u8,
    /// `actor.update` / `actor.draw`: `BgSpot16Doughnut_UpdateExpanding` and `_DrawExpanding`.
    pub expanding: bool,
}

impl BgSpot16Doughnut {
    /// `BgSpot16Doughnut_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: the culling volume only.
        actor.scale = Vec3::splat(0.1);
        let mut this = BgSpot16Doughnut { actor, fire_flag: 0, env_color_alpha: 255, expanding: false };
        let params = this.actor.params;
        if (1..=4).contains(&params) {
            this.actor.scale = Vec3::splat(S_SCALES[params as usize] as f32 * 1.0e-4);
            this.expanding = true;
        } else {
            let s = match play.scene_id {
                SCENE_KAKARIKO_VILLAGE => 0.04,
                SCENE_TEMPLE_OF_TIME_EXTERIOR_DAY | SCENE_TEMPLE_OF_TIME_EXTERIOR_NIGHT | SCENE_TEMPLE_OF_TIME_EXTERIOR_RUINS => 0.018,
                _ => 0.1,
            };
            this.actor.scale = Vec3::splat(s);
            log::debug!("{}", this.actor.scale.x);
            if !play.save.adult || play.save.get_event_chk_inf(EVENTCHKINF_2F) {
                this.fire_flag &= !1;
            } else {
                this.fire_flag |= 1;
            }
            log::debug!("(spot16 Donut Cloud)(arg_data {params:#06x})");
        }
        Box::new(this)
    }

    /// `BgSpot16Doughnut_Update`: the white ring turns, fading in; the fiery one fades out on
    /// channel 2's cue 2, then turns white.
    fn update_normal(&mut self, play: &PlayState) {
        if self.fire_flag & 1 == 0 {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_sub(0x20);
            if self.env_color_alpha < 255 {
                self.env_color_alpha = self.env_color_alpha.wrapping_add(5);
            } else {
                self.env_color_alpha = 255;
            }
        } else if play.cs_ctx.state != CS_STATE_IDLE && play.cs_ctx.npc_actions.get(2).copied().flatten().is_some_and(|c| c.action == 2) {
            if self.env_color_alpha >= 6 {
                self.env_color_alpha -= 5;
            } else {
                self.env_color_alpha = 0;
                self.fire_flag &= !1;
            }
        }
    }

    /// `BgSpot16Doughnut_UpdateExpanding`: fading out, turning, growing 0.002 a frame.
    fn update_expanding(&mut self) {
        if self.env_color_alpha >= 6 {
            self.env_color_alpha -= 5;
        } else {
            self.actor.kill();
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_sub(0x20);
        // Actor_SetScale(scale.x + 0.0019999998f).
        self.actor.scale = Vec3::splat(self.actor.scale.x + 0.0019999998);
    }
}

const BAKE_FIERY: &str = "Bg_Spot16_Doughnut/fiery";
const BAKE_NORMAL: &str = "Bg_Spot16_Doughnut/normal";

/// The segments: the fiery scroll (8, as the list calls it), the draw's colours (0x0A).
const SEG_SCROLL: u8 = 0x08;
const SEG_COLOR: u8 = 0x0A;

/// `Gfx_TwoTexScroll(G_TX_RENDERTILE, scroll * -1, 0, 16, 32, 1, scroll, scroll * -2, 16, 32)`.
fn fiery_scroll(scroll: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, scroll.wrapping_neg(), 0, 16, 32, 1, scroll, scroll.wrapping_mul(2).wrapping_neg(), 16, 32)
}

/// Its bakes: the fiery ring (its env colour, `(255, 0, 0, alpha)`), the white one (env
/// `(255, 255, 255, alpha)`, prim white).
pub fn bakes() -> Vec<MeshBake> {
    let env = |c: [u8; 4]| (0xFB00_0000u32, u32::from_be_bytes(c));
    let end = (0xDF00_0000u32, 0u32);
    vec![
        MeshBake {
            name: BAKE_FIERY.into(),
            object: OBJECT.into(),
            segments: vec![(SEG_SCROLL, BakeSegment::Dynamic(fiery_scroll(0))), (SEG_COLOR, BakeSegment::Dynamic(vec![env([255, 0, 0, 255]), end]))],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![(OBJECT.into(), "gDeathMountainCloudCircleFieryDL".into())]),
        },
        MeshBake {
            name: BAKE_NORMAL.into(),
            object: OBJECT.into(),
            segments: vec![(SEG_COLOR, BakeSegment::Dynamic(vec![env([255, 255, 255, 255]), (0xFA00_0000, 0xFFFF_FFFF), end]))],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![(OBJECT.into(), "gDeathMountainCloudCircleNormalDL".into())]),
        },
    ]
}

impl ActorImpl for BgSpot16Doughnut {
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
        if self.expanding {
            self.update_expanding();
        } else {
            self.update_normal(play);
        }
    }

    /// `BgSpot16Doughnut_Draw` (the fiery ring scrolled by `gameplayFrames`, or the white one) and
    /// `_DrawExpanding` (the white one).
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let m = actor_draw_matrix(rs);
        let mut sv = SegmentValues::default();
        let a = self.env_color_alpha;
        let name = if !self.expanding && self.fire_flag & 1 != 0 {
            sv.read(SEG_SCROLL, &fiery_scroll(play.gameplay_frames & 0xFFFF));
            sv.env[SEG_COLOR as usize] = Some([255, 0, 0, a]);
            BAKE_FIERY
        } else {
            sv.env[SEG_COLOR as usize] = Some([255, 255, 255, a]);
            BAKE_NORMAL
        };
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(name)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
