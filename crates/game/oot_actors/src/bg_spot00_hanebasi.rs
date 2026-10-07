//! `Bg_Spot00_Hanebasi` (`ovl_Bg_Spot00_Hanebasi/z_bg_spot00_hanebasi.c`): Hyrule Castle's
//! drawbridge in Hyrule Field, its two chains, and the torches either side of the gate.
//!
//! The placed actor (params -1, `DT_DRAWBRIDGE`) spawns the first chain as its child, which
//! spawns the second as its own (`DT_CHAIN_1`, `DT_CHAIN_2`); all three are DynaPoly actors
//! (`gHyruleFieldCastleDrawbridgeCol`, `gHyruleFieldCastleDrawbridgeChainsCol` in
//! `object_spot00_objects`). The bridge is raised (`shape.rot.x` -0x4000) in the opening's
//! cutscene layers (4, 5), by night for a child, and with the three spiritual stones before
//! `EVENTCHKINF_ZELDA_FLED_CASTLE`; it lowers (and raises) by 80 a frame, the chains following at 0.4 of that
//! once it's past -0x27D8. Each chain holds a point light at its torch, flickering by
//! `Rand_ZeroOne`, sized by the torches' flame (`sTorchFlameScale`, which the bridge's draw
//! sets).
//!
//! The draw puts the chains where the bridge's matrix takes their ends (`(±158, 10, 400)`), so
//! it changes them: that part runs at `Play_Draw`'s time (`draw_update`). The torches' flames
//! are `gEffFire1DL` turned to face the camera with a scroll on segment 8 (a bake).
//!
//! Not modelled: the glow sprites of the point lights (`Lights_GlowCheck`, `Lights_DrawGlow`)
//! and the time speed in scene layer 5 (`gTimeSpeed`: time doesn't pass in this port).

use std::sync::Arc;

use anyhow::Result;
use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_TRANSFORM_POS};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{binang_to_rad, cos_s, scaled_step_to_s, sin_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::{NA_SE_EV_BRIDGE_CLOSE, NA_SE_EV_BRIDGE_CLOSE_STOP, NA_SE_EV_BRIDGE_OPEN, NA_SE_EV_BRIDGE_OPEN_STOP, SFX_FLAG};
use oot_game::env::STORM_REQUEST_START;
use oot_game::item::{QUEST_GORON_RUBY, QUEST_KOKIRI_EMERALD, QUEST_ZORA_SAPPHIRE};
use oot_game::lights::{LightInfo, LightNode};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::save::EVENTCHKINF_ZELDA_FLED_CASTLE;
use oot_game::scene_table::gfx_two_tex_scroll;
use oot_game::transition::TRANS_TRIGGER_START;

use crate::bg_treemouth::actor_is_facing_and_near_player;

/// `ACTOR_BG_SPOT00_HANEBASI` (`actor_table.h`: 0x004A).
pub const ACTOR_BG_SPOT00_HANEBASI: i16 = 0x004A;

pub const OBJECT: &str = "object_spot00_objects";
const DRAWBRIDGE_COL: &str = "gHyruleFieldCastleDrawbridgeCol";
const CHAINS_COL: &str = "gHyruleFieldCastleDrawbridgeChainsCol";
const DRAWBRIDGE_DL: &str = "gHyruleFieldCastleDrawbridgeDL";
const CHAINS_DL: &str = "gHyruleFieldCastleDrawbridgeChainsDL";

/// `Bg_Spot00_Hanebasi_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_SPOT00_HANEBASI, name: "Bg_Spot00_Hanebasi", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT };

/// `DrawbridgeType`.
pub const DT_DRAWBRIDGE: i16 = -1;
pub const DT_CHAIN_1: i16 = 0;
pub const DT_CHAIN_2: i16 = 1;

/// `SCENE_HYRULE_FIELD` (`scene_table.h`: Hyrule Field).
const SCENE_HYRULE_FIELD: u16 = 0x51;
/// `EVENTCHKINF_82` (`save.h`).
const EVENTCHKINF_82: u16 = 0x82;
/// `ENTR_HYRULE_FIELD_0` (`entrance_table.h`: 0x00CD).
const ENTR_HYRULE_FIELD_0: u16 = 0x00CD;
/// `TRANS_TYPE_FADE_BLACK_FAST` (`transition.h`).
const TRANS_TYPE_FADE_BLACK_FAST: u8 = 42;

/// The torches' flame: `Gfx_SetupDL_25Xlu`, the colours, `gEffFire1DL` under the scroll.
const TORCH_BAKE: &str = "Bg_Spot00_Hanebasi/torch";
const SEG_SCROLL: u8 = 0x08;
const SEG_COLORS: u8 = 0x0C;

/// `BgSpot00Hanebasi_DrawTorches`' scroll for flame `i` at `gameplayFrames`:
/// `Gfx_TwoTexScroll(.., 0, 0, 0, 32, 64, 1, 0, ((gameplayFrames + i) * -20) & 0x1FF, 32, 128)`.
fn torch_scroll(gameplay_frames: u32, i: u32) -> Vec<(u32, u32)> {
    let y2 = (gameplay_frames.wrapping_add(i) as i32).wrapping_mul(-20) as u32 & 0x1FF;
    gfx_two_tex_scroll(0, 0, 0, 32, 64, 1, 0, y2, 32, 128)
}

/// The torches' flame bake (`gDPSetPrimColor(128, 128, 255, 255, 0, 255)`,
/// `gDPSetEnvColor(255, 0, 0, 0)`, `gEffFire1DL`).
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: TORCH_BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_COLORS, BakeSegment::Commands(vec![(0xFA00_8080, 0xFFFF_00FF), (0xFB00_0000, 0xFF00_0000), (0xDF00_0000, 0)])), (SEG_SCROLL, BakeSegment::Dynamic(torch_scroll(0, 0)))],
        prelude: vec![SEG_COLORS],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffFire1DL".into())]),
    }]
}

/// The overlay's statics: `sTorchFlameScale`.
#[derive(Debug, Default)]
struct Statics {
    torch_flame_scale: f32,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `BgSpot00Hanebasi_DrawbridgeWait`.
    DrawbridgeWait,
    /// `BgSpot00Hanebasi_DrawbridgeRiseAndFall`.
    DrawbridgeRiseAndFall,
    /// `BgSpot00Hanebasi_SetTorchLightInfo` (the chains).
    SetTorchLightInfo,
    /// `BgSpot00Hanebasi_DoNothing`.
    DoNothing,
}

pub struct BgSpot00Hanebasi {
    /// `dyna.actor`.
    pub actor: Actor,
    /// `dyna.bgId`.
    pub bg: u16,
    pub action: Action,
    /// `destAngle`.
    pub dest_angle: i16,
    /// `lightNode` (the chains').
    pub light_node: Option<LightNode>,
}

/// `IS_CUTSCENE_LAYER` (`save.h`: `sceneLayer >= SCENE_LAYER_CUTSCENE_FIRST`, 4).
fn is_cutscene_layer(play: &PlayState) -> bool {
    play.save.scene_layer >= 4
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: 0.0 }
}

fn load_collision(play: &PlayState, symbol: &str) -> Result<Arc<CollisionHeader>> {
    let assets = play.assets.as_ref().ok_or_else(|| anyhow::anyhow!("no asset pack"))?;
    Ok(Arc::new(assets.pack.collision(&keys::collision(OBJECT, symbol))?))
}

/// The three spiritual stones and not yet `EVENTCHKINF_ZELDA_FLED_CASTLE` (Zelda's escape not seen).
fn stones_before_escape(play: &PlayState) -> bool {
    let s = &play.save;
    s.check_quest_item(QUEST_KOKIRI_EMERALD) && s.check_quest_item(QUEST_GORON_RUBY) && s.check_quest_item(QUEST_ZORA_SAPPHIRE) && !s.get_event_chk_inf(EVENTCHKINF_ZELDA_FLED_CASTLE)
}

impl BgSpot00Hanebasi {
    /// `BgSpot00Hanebasi_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: the cull zone (not ported) and ICHAIN_VEC3F_DIV1000(scale, 1000).
        actor.scale = Vec3::ONE;
        let params = actor.params;
        // DynaPolyActor_Init(DYNA_TRANSFORM_POS), the collision by type, DynaPoly_SetBgActor.
        let symbol = if params == DT_DRAWBRIDGE { DRAWBRIDGE_COL } else { CHAINS_COL };
        let mut bg = BG_ACTOR_MAX;
        let mut this = BgSpot00Hanebasi { actor, bg, action: Action::DoNothing, dest_angle: 0, light_node: None };
        match load_collision(play, symbol) {
            Ok(h) => bg = play.col.dyna.set_bg_actor(h, source(&this.actor), DYNA_TRANSFORM_POS),
            Err(e) => log::error!("Bg_Spot00_Hanebasi: {e:#}"),
        }
        this.bg = bg;
        let a = &mut this.actor;
        if params == DT_DRAWBRIDGE {
            if play.save.adult && !is_cutscene_layer(play) {
                a.kill();
                return Box::new(this);
            }
            let layer = play.save.scene_layer;
            a.shape_rot.x = if layer != 6 && (layer == 4 || layer == 5 || (!play.save.adult && !play.save.is_day())) { -0x4000 } else { 0 };
            if layer != 6 && stones_before_escape(play) {
                a.shape_rot.x = -0x4000;
            }
            let (rx, ry) = (a.shape_rot.x, a.shape_rot.y);
            let cy = 10.0 * cos_s(rx) - sin_s(rx) * 400.0;
            let cz = 10.0 * sin_s(rx) - cos_s(rx) * 400.0;
            let chain = Vec3::new(158.0 * cos_s(ry) + sin_s(ry) * cz, cy, -158.0 * sin_s(ry) + cos_s(ry) * cz);
            let pos = a.world_pos + chain;
            let rot = [if rx == 0 { 0 } else { 0xF020u16 as i16 }, ry, 0];
            if play.actor_spawn_as_child(a, ACTOR_BG_SPOT00_HANEBASI, pos, rot, DT_CHAIN_1).is_err() {
                a.kill();
            }
            this.action = Action::DrawbridgeWait;
            this.dest_angle = 40;
        } else if params == DT_CHAIN_1 {
            let ry = a.shape_rot.y;
            let pos = Vec3::new(a.world_pos.x - cos_s(ry) * 316.0, a.world_pos.y, a.world_pos.z + sin_s(ry) * 316.0);
            let rot = [a.shape_rot.x, ry, 0];
            if play.actor_spawn_as_child(a, ACTOR_BG_SPOT00_HANEBASI, pos, rot, DT_CHAIN_2).is_err() {
                a.kill();
                // Actor_Kill(this->dyna.actor.parent): the parent's init is still running, so
                // it isn't in the actor context; its own spawn check kills it.
            }
            this.action = Action::SetTorchLightInfo;
        } else {
            this.action = Action::SetTorchLightInfo;
        }
        if params >= DT_CHAIN_1 {
            // LightContext_InsertLight, Lights_PointGlowSetInfo(±260, 168, 690, (255, 255, 0), 0).
            this.light_node = play.light_ctx.insert_light(LightInfo::point_glow(Self::torch_x(params), 168, 690, [255, 255, 0], 0));
        }
        Box::new(this)
    }

    /// The chain's torch: x 260 (`DT_CHAIN_1`) or -260.
    fn torch_x(params: i16) -> i16 {
        if params == DT_CHAIN_1 { 260 } else { -260 }
    }

    fn child_dest_angle(play: &PlayState, child: Option<ActorHandle>) -> Option<i16> {
        child.and_then(|h| play.actors.downcast::<BgSpot00Hanebasi>(h)).map(|c| c.dest_angle)
    }

    fn set_child_dest_angle(play: &mut PlayState, child: Option<ActorHandle>, angle: i16) {
        if let Some(c) = child.and_then(|h| play.actors.downcast_mut::<BgSpot00Hanebasi>(h)) {
            c.dest_angle = angle;
        }
    }

    /// `BgSpot00Hanebasi_DrawbridgeWait`.
    fn drawbridge_wait(&mut self, play: &mut PlayState) {
        let child = self.actor.child;
        let cs_layer = is_cutscene_layer(play);
        if cs_layer || !stones_before_escape(play) {
            if self.actor.shape_rot.x != 0 && (play.flags_get_env(0) || (!cs_layer && play.save.is_day())) {
                self.action = Action::DrawbridgeRiseAndFall;
                self.dest_angle = 0;
                Self::set_child_dest_angle(play, child, 0);
                return;
            }
            if self.actor.shape_rot.x == 0 && !cs_layer && !play.save.adult && !play.save.is_day() {
                self.action = Action::DrawbridgeRiseAndFall;
                self.dest_angle = -0x4000;
                Self::set_child_dest_angle(play, child, -0xFE0);
            }
        }
    }

    /// `BgSpot00Hanebasi_DrawbridgeRiseAndFall`: the bridge by 80 a frame; past -0x27D8 the
    /// chains by 80 * 0.4 towards the child's `destAngle`; the bridge's sound while it moves and
    /// its stop.
    fn drawbridge_rise_and_fall(&mut self, play: &mut PlayState) {
        let angle: i16 = 80;
        if scaled_step_to_s(&mut self.actor.shape_rot.x, self.dest_angle, angle) {
            self.action = Action::DrawbridgeWait;
        }
        if self.actor.shape_rot.x >= -0x27D8 {
            let child = self.actor.child;
            let step = (angle as f32 * 0.4) as i16;
            if let Some(dest) = Self::child_dest_angle(play, child) {
                let mut grandchild = None;
                if let Some(c) = child.and_then(|h| play.actors.downcast_mut::<BgSpot00Hanebasi>(h)) {
                    scaled_step_to_s(&mut c.actor.shape_rot.x, dest, step);
                    play.col.dyna.set_source(c.bg, source(&c.actor));
                    grandchild = c.actor.child;
                }
                if let Some(g) = grandchild.and_then(|h| play.actors.downcast_mut::<BgSpot00Hanebasi>(h)) {
                    scaled_step_to_s(&mut g.actor.shape_rot.x, dest, step);
                    play.col.dyna.set_source(g.bg, source(&g.actor));
                }
            }
        }
        let stopped = self.action == Action::DrawbridgeWait;
        if self.dest_angle < 0 {
            if stopped {
                audio_play_actor_sfx2(play, NA_SE_EV_BRIDGE_CLOSE_STOP);
            } else {
                self.actor.play_sfx_flagged(NA_SE_EV_BRIDGE_CLOSE - SFX_FLAG);
            }
        } else if stopped {
            audio_play_actor_sfx2(play, NA_SE_EV_BRIDGE_OPEN_STOP);
        } else {
            self.actor.play_sfx_flagged(NA_SE_EV_BRIDGE_OPEN - SFX_FLAG);
        }
    }

    /// `BgSpot00Hanebasi_SetTorchLightInfo`: the light flickers (red and green 128 to 254),
    /// higher and wider with the flame.
    fn set_torch_light_info(&mut self, play: &mut PlayState) {
        let light_color = ((play.rand.zero_one() * 127.0) as u8).wrapping_add(128);
        let scale = play.overlay_static::<Statics>(ACTOR_BG_SPOT00_HANEBASI).torch_flame_scale;
        let y = ((5000.0 * scale) + 128.0) as i16;
        let radius = (scale * 37500.0) as i16;
        play.light_ctx.set_info(self.light_node, LightInfo::point_glow(Self::torch_x(self.actor.params), y, 690, [light_color, light_color, 0], radius));
    }

    /// The child-only part of the drawbridge's update in Hyrule Field: with the three stones
    /// before `EVENTCHKINF_ZELDA_FLED_CASTLE`, Link on the bridge's way (x within 450, z 1080 to 1700, not in a
    /// cutscene) starts Zelda's escape (`ENTR_HYRULE_FIELD_0` with cutscene 0xFFF1); facing it within
    /// 3000, the storm.
    fn drawbridge_field(&mut self, play: &mut PlayState) {
        if play.scene_id != SCENE_HYRULE_FIELD || !(stones_before_escape(play) && !play.save.adult) {
            return;
        }
        let Some(pos) = play.player.and_then(|h| play.actors.actor(h)).map(|p| p.world_pos) else { return };
        if pos.x > -450.0 && pos.x < 450.0 && pos.z > 1080.0 && pos.z < 1700.0 && !play.play_in_cs_mode() {
            play.save.set_event_chk_inf(EVENTCHKINF_ZELDA_FLED_CASTLE);
            play.save.set_event_chk_inf(EVENTCHKINF_82);
            self.action = Action::DoNothing;
            play.player_set_cs_action_with_halted_actors(None, 8);
            play.transition.next_entrance_index = ENTR_HYRULE_FIELD_0;
            play.save.next_cutscene_index = 0xFFF1;
            play.transition.trigger = TRANS_TRIGGER_START;
            play.transition.ty = TRANS_TYPE_FADE_BLACK_FAST;
        } else if actor_is_facing_and_near_player(&self.actor, 3000.0, 0x7530) {
            play.env_ctx.storm_request = STORM_REQUEST_START;
        }
    }

    /// The torches' flame scale `BgSpot00Hanebasi_DrawTorches` sets: 0.008 in a cutscene layer,
    /// else by how far the bridge is raised; 0 when the torches aren't drawn.
    fn torch_flame_scale(play: &PlayState, rot_x: i16) -> Option<f32> {
        if play.save.scene_layer == 12 {
            return None;
        }
        let cs_layer = is_cutscene_layer(play);
        if cs_layer || (!play.save.adult && rot_x < -0x2000) { Some(if cs_layer { 0.008 } else { ((rot_x as i32 * -1) - 0x2000) as f32 * (1.0 / 1024000.0) }) } else { None }
    }
}

impl ActorImpl for BgSpot00Hanebasi {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgSpot00Hanebasi_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::DrawbridgeWait => self.drawbridge_wait(play),
            Action::DrawbridgeRiseAndFall => self.drawbridge_rise_and_fall(play),
            Action::SetTorchLightInfo => self.set_torch_light_info(play),
            Action::DoNothing => {}
        }
        if self.actor.params == DT_DRAWBRIDGE {
            self.drawbridge_field(play);
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `BgSpot00Hanebasi_Draw`'s changes: the chains to the bridge's ends (`(±158, 10, 400)`
    /// through its matrix), and `sTorchFlameScale`.
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.actor.params != DT_DRAWBRIDGE {
            return;
        }
        let m = actor_draw_matrix(&RenderState::of(&self.actor));
        let left = m.transform_point3(Vec3::new(158.0, 10.0, 400.0));
        let right = m.transform_point3(Vec3::new(-158.0, 10.0, 400.0));
        let mut grandchild = None;
        if let Some(c) = self.actor.child.and_then(|h| play.actors.actor_mut(h)) {
            c.world_pos = left;
            grandchild = c.child;
        }
        if let Some(g) = grandchild.and_then(|h| play.actors.actor_mut(h)) {
            g.world_pos = right;
        }
        let scale = Self::torch_flame_scale(play, self.actor.shape_rot.x).unwrap_or(0.0);
        play.overlay_static::<Statics>(ACTOR_BG_SPOT00_HANEBASI).torch_flame_scale = scale;
    }

    /// `BgSpot00Hanebasi_Draw`: the bridge or a chain, and the bridge's torches.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let dl = if self.actor.params == DT_DRAWBRIDGE { DRAWBRIDGE_DL } else { CHAINS_DL };
        out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, dl)), actor_draw_matrix(rs)));
        if self.actor.params != DT_DRAWBRIDGE {
            return;
        }
        // BgSpot00Hanebasi_DrawTorches: two flames at (±260, 128, 690), turned to face the
        // active camera, scaled by sTorchFlameScale.
        let Some(scale) = Self::torch_flame_scale(play, rs.rot[0]) else { return };
        let angle = binang_to_rad(play.cam_dir_yaw().wrapping_add(i16::MIN));
        for i in 0..2u32 {
            let x = if i == 0 { 260.0 } else { -260.0 };
            let t = Mat4::from_translation(Vec3::new(x, 128.0, 690.0)) * Mat4::from_rotation_y(angle) * Mat4::from_scale(Vec3::splat(scale));
            let mut sv = SegmentValues::default();
            sv.read(SEG_SCROLL, &torch_scroll(play.gameplay_frames, i));
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(TORCH_BAKE)), transform: t, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
    }

    /// `BgSpot00Hanebasi_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
        if self.actor.params >= DT_CHAIN_1 {
            play.light_ctx.remove_light(self.light_node);
        }
    }

    /// `DynaPoly_GetActor` finds it by its bg id; `Actor_UpdateAll` clears its interact flags after
    /// its update.
    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != eng_collision::dyna::BG_ACTOR_MAX).then_some(self.bg)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
