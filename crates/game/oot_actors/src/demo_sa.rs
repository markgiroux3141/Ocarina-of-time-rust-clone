//! `Demo_Sa` (`ovl_Demo_Sa/z_demo_sa.c`): Saria in cutscenes. Its params pick one of five uses:
//! the Chamber of Sages' Forest Medallion (0 and 1), the sages' magic (2), an unused one (3), the
//! credits (4), and the Lost Woods' bridge (5), the only one this phase reaches.
//!
//! On the bridge (`DemoSa_InitBridge`, `gLostWoodsFairyOcarinaCs`) she follows her cue on channel
//! 1: hidden (4), standing sad with her eyes on Link, faded in or at once (12), clutching the
//! ocarina, eyes shut (13), holding it out to Link (14). Her fairy (`En_Elf` `FAIRY_KOKIRI`) is
//! spawned as her child.
//!
//! Ported (GAME-06 milestone 2, as decided): the bridge's actions and draws and the helpers they
//! share (`DemoSa_Blink`, `_SetEyes`, `_SetMouth`, `_UpdateBgCheckInfo`, `_UpdateSkelAnime`,
//! `_GetCue`, `_CheckForCue`, `_CheckForNoCue`, `_SetStartPosRotFromCue`, `_AnimationChange`,
//! `_OverrideLimbDraw`, `_DrawOpa`, `_DrawXlu`). The other four uses are logged and draw nothing.
//! The draws are bakes, one per face, pass and hand the bridge shows (ADR 0012).

use std::sync::Arc;

use eng_gfx::{DrawCmd, MeshKey, SegmentValues};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::gbi::Dl;
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};

/// `ACTOR_DEMO_SA` (`actor_table.h`: 0x00C9).
pub const ACTOR_DEMO_SA: i16 = 0x00C9;
pub const OBJECT: &str = "object_sa";
/// `ACTOR_EN_ELF` (0x0018), `FAIRY_KOKIRI` (`z_en_elf.h`: 3).
const ACTOR_EN_ELF: i16 = 0x0018;
const FAIRY_KOKIRI: i16 = 3;

/// `Demo_Sa_Profile`: `ACTORCAT_NPC`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_DEMO_SA, name: "Demo_Sa", category: ACTORCAT_NPC, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT };

/// `SariaEyes`.
pub const SARIA_EYE_OPEN: i16 = 0;
pub const SARIA_EYE_HALF: i16 = 1;
pub const SARIA_EYE_CLOSED: i16 = 2;
pub const SARIA_EYE_SURPRISED: i16 = 3;
pub const SARIA_EYE_SAD: i16 = 4;
/// `SariaMouth`.
pub const SARIA_MOUTH_CLOSED2: i16 = 0;
pub const SARIA_MOUTH_SURPRISED: i16 = 1;
pub const SARIA_MOUTH_CLOSED: i16 = 2;
pub const SARIA_MOUTH_SMILING_OPEN: i16 = 3;
pub const SARIA_MOUTH_FROWNING: i16 = 4;

/// `sEyeTextures`, `sMouthTextures`.
const EYE_TEXTURES: [&str; 5] = ["gSariaEyeOpenTex", "gSariaEyeHalfTex", "gSariaEyeClosedTex", "gSariaEyeSuprisedTex", "gSariaEyeSadTex"];
const MOUTH_TEXTURES: [&str; 5] = ["gSariaMouthClosed2Tex", "gSariaMouthSuprisedTex", "gSariaMouthClosedTex", "gSariaMouthSmilingOpenTex", "gSariaMouthFrowningTex"];

/// `DemoSaAction` (the bridge's; the others are logged).
pub const DEMOSA_ACTION_BRIDGE_INVISIBLE: i32 = 16;
pub const DEMOSA_ACTION_BRIDGE_FADE_IN: i32 = 17;
pub const DEMOSA_ACTION_BRIDGE_LOOKING_SAD: i32 = 18;
pub const DEMOSA_ACTION_BRIDGE_CLUTCH_OCARINA: i32 = 19;
pub const DEMOSA_ACTION_BRIDGE_GIVE_OCARINA: i32 = 20;

/// `DemoSaDrawConfig`.
pub const DEMOSA_DRAW_NOTHING: i32 = 0;
pub const DEMOSA_DRAW_OPA: i32 = 1;
pub const DEMOSA_DRAW_XLU: i32 = 2;

/// The bridge's animations.
const ANIM_WAIT_ON_BRIDGE: &str = "gSariaWaitOnBridgeAnim";
const ANIM_HOLD_OCARINA: &str = "gSariaHoldOcarinaAnim";
const ANIM_GIVE_LINK_OCARINA: &str = "gSariaGiveLinkOcarinaAnim";
const ANIM_HOLD_OUT_OCARINA: &str = "gSariaHoldOutOcarinaAnim";

/// `kREG(17)`: 0 (the debug registers start zeroed).
const KREG_17: f32 = 0.0;

pub struct DemoSa {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Option<Arc<eng_anim::skeleton::Skeleton>>,
    pub eye_index: i16,
    pub blink_timer: i16,
    pub mouth_index: i16,
    pub action: i32,
    pub draw_config: i32,
    pub fade_timer: f32,
    pub alpha: i32,
    pub is_light_ball: bool,
    pub cue_id: i32,
    pub is_holding_ocarina: bool,
    /// `actor.shape.shadowAlpha`.
    pub shadow_alpha: u8,
}

/// The faces, passes and hands the bridge shows, each a bake: (eye, mouth, translucent, holding
/// the ocarina).
const BAKED: [(i16, i16, bool, bool); 3] = [(SARIA_EYE_SAD, SARIA_MOUTH_CLOSED, false, false), (SARIA_EYE_SAD, SARIA_MOUTH_CLOSED, true, false), (SARIA_EYE_CLOSED, SARIA_MOUTH_CLOSED, false, true)];

/// The segments `DemoSa_DrawOpa` / `_DrawXlu` bind: the eyes (8 and 9), the mouth (0xA), the
/// setup list on 0xC (`ACTOR_SETUP_OPA_DL`, empty, or `gActorSetupXluDL`), the env alpha (0xE,
/// run before the skeleton).
const SEG_EYES: u8 = 0x08;
const SEG_EYES_2: u8 = 0x09;
const SEG_MOUTH: u8 = 0x0A;
const SEG_SETUP: u8 = 0x0C;
const SEG_ALPHA: u8 = 0x0E;

pub fn bake_name(eye: i16, mouth: i16, xlu: bool, ocarina: bool) -> String {
    format!("Demo_Sa/{}_eye{eye}_mouth{mouth}{}", if xlu { "xlu" } else { "opa" }, if ocarina { "_ocarina" } else { "" })
}

/// `gActorSetupXluDL` (`z_actor.c`): `gsDPSetRenderMode(G_RM_FOG_SHADE_A, G_RM_AA_ZB_XLU_SURF2 |
/// Z_UPD)`, `gsDPSetAlphaCompare(G_AC_THRESHOLD)`.
fn actor_setup_xlu_dl() -> Vec<(u32, u32)> {
    let mut d = Dl::default();
    d.0.push((0xE200_001C, 0xC810_49F8));
    d.othermode_l(0, 2, 1);
    d.end();
    d.0
}

/// Its bakes.
pub fn bakes() -> Vec<MeshBake> {
    BAKED
        .iter()
        .map(|&(eye, mouth, xlu, ocarina)| {
            let tex = |s: &str| BakeSegment::Texture { file: OBJECT.into(), symbol: s.into() };
            MeshBake {
                name: bake_name(eye, mouth, xlu, ocarina),
                object: OBJECT.into(),
                segments: vec![
                    (SEG_EYES, tex(EYE_TEXTURES[eye as usize])),
                    (SEG_EYES_2, tex(EYE_TEXTURES[eye as usize])),
                    (SEG_MOUTH, tex(MOUTH_TEXTURES[mouth as usize])),
                    (SEG_SETUP, BakeSegment::Commands(if xlu { actor_setup_xlu_dl() } else { vec![(0xDF00_0000, 0)] })),
                    (SEG_ALPHA, BakeSegment::DynamicColor { env: true, prim: false }),
                ],
                prelude: vec![SEG_ALPHA],
                body: BakeBody::Skeleton {
                    file: OBJECT.into(),
                    symbol: "gSariaSkel".into(),
                    // DemoSa_OverrideLimbDraw: limb 15 (the right hand) and the ocarina.
                    limbs: if ocarina { vec![LimbOverride { limb: 14, file: OBJECT.into(), symbol: "gSariaRightHandAndOcarinaDL".into() }] } else { Vec::new() },
                },
            }
        })
        .collect()
}

/// The render state's switches and values.
mod rs {
    pub const DRAW_CONFIG: usize = 0;
    pub const EYE: usize = 1;
    pub const MOUTH: usize = 2;
    pub const OCARINA: usize = 3;
    pub const ALPHA: usize = 0;
    pub const SHADOW_ALPHA: usize = 1;
}

impl DemoSa {
    fn anim(&self, play: &PlayState, name: &str) -> Option<Anim> {
        let a = play.assets.as_ref()?.animation(OBJECT, name);
        a.map_err(|e| log::error!("Demo_Sa: {e:#}")).ok()
    }

    /// `DemoSa_Init`: `ActorShape_Init(0, ActorShadow_DrawCircle, 30)`, then the use's init.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = DemoSa {
            actor,
            skel: SkelAnimeStd::init_flex(0, None),
            skeleton: None,
            eye_index: 0,
            blink_timer: 0,
            mouth_index: 0,
            action: 0,
            draw_config: DEMOSA_DRAW_NOTHING,
            fade_timer: 0.0,
            alpha: 0,
            is_light_ball: false,
            cue_id: 0,
            is_holding_ocarina: false,
            shadow_alpha: 255,
        };
        this.actor.shape_y_offset = 0.0;
        match this.actor.params {
            5 => this.init_bridge(play),
            p => log::warn!("Demo_Sa: params {p} (the Chamber of Sages, the sages' magic, the credits, the unused one) isn't ported: drawn as nothing"),
        }
        Box::new(this)
    }

    /// `DemoSa_InitBridge`: `gSariaWaitOnBridgeAnim`, her fairy, hidden, sad.
    fn init_bridge(&mut self, play: &mut PlayState) {
        let skeleton = play.assets.as_ref().map(|a| a.skeleton(OBJECT, "gSariaSkel"));
        match skeleton {
            Some(Ok(s)) => {
                let wait = self.anim(play, ANIM_WAIT_ON_BRIDGE);
                self.skel = SkelAnimeStd::init_flex(s.limbs.len(), wait);
                self.skeleton = Some(s);
            }
            Some(Err(e)) => log::error!("Demo_Sa: {e:#}"),
            None => {}
        }
        let pos = self.actor.world_pos;
        if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_ELF, pos, [0; 3], FAIRY_KOKIRI) {
            log::debug!("Demo_Sa's fairy: {e:?}");
        }
        self.action = DEMOSA_ACTION_BRIDGE_INVISIBLE;
        self.draw_config = DEMOSA_DRAW_NOTHING;
        self.shadow_alpha = 0;
        self.set_eyes(SARIA_EYE_SAD);
        self.set_mouth(SARIA_MOUTH_CLOSED);
    }

    /// `DemoSa_Blink`: shared, not called on the bridge.
    pub fn blink(&mut self, play: &mut PlayState) {
        // DECR(*blinkTimer) == 0: 0 when it was 0, else the decremented value.
        let decr = if self.blink_timer == 0 {
            0
        } else {
            self.blink_timer -= 1;
            self.blink_timer
        };
        if decr == 0 {
            // Rand_S16Offset(0x3C, 0x3C).
            self.blink_timer = (play.rand.zero_one() * 60.0) as i16 + 0x3C;
        }
        self.eye_index = self.blink_timer;
        if self.eye_index >= SARIA_EYE_SURPRISED {
            self.eye_index = SARIA_EYE_OPEN;
        }
    }

    fn set_eyes(&mut self, eye: i16) {
        self.eye_index = eye;
    }

    fn set_mouth(&mut self, mouth: i16) {
        self.mouth_index = mouth;
    }

    /// `DemoSa_UpdateBgCheckInfo`.
    fn update_bg_check_info(&mut self, play: &PlayState) {
        self.actor.update_bg_check_info(&play.col, 75.0, 30.0, 30.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
    }

    /// `DemoSa_UpdateSkelAnime`.
    fn update_skel_anime(&mut self) -> bool {
        self.skel.update()
    }

    /// `DemoSa_GetCue`.
    fn get_cue(play: &PlayState, channel: usize) -> Option<CsCmdActorCue> {
        if play.cs_ctx.state != CS_STATE_IDLE { play.cs_ctx.npc_actions.get(channel).copied().flatten() } else { None }
    }

    /// `DemoSa_CheckForCue`.
    pub fn check_for_cue(play: &PlayState, cue_id: u16, channel: usize) -> bool {
        Self::get_cue(play, channel).is_some_and(|c| c.action == cue_id)
    }

    /// `DemoSa_CheckForNoCue`.
    pub fn check_for_no_cue(play: &PlayState, cue_id: u16, channel: usize) -> bool {
        Self::get_cue(play, channel).is_some_and(|c| c.action != cue_id)
    }

    /// `DemoSa_SetStartPosRotFromCue`.
    fn set_start_pos_rot_from_cue(&mut self, play: &PlayState, channel: usize) {
        if let Some(cue) = Self::get_cue(play, channel) {
            self.actor.world_pos = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
            self.actor.world_rot.y = cue.rot[1];
            self.actor.shape_rot.y = cue.rot[1];
        }
    }

    /// `DemoSa_AnimationChange`: from the start (or, reversed, from the end) at speed 1.
    fn animation_change(&mut self, play: &PlayState, name: &str, mode: u8, morph_frames: f32, play_reversed: bool) {
        let Some(a) = self.anim(play, name) else { return };
        let frame_count = a.last_frame();
        let (start, end, speed) = if !play_reversed { (0.0, frame_count, 1.0) } else { (frame_count, 0.0, -1.0) };
        self.skel.change(a, speed, start, end, mode, morph_frames);
    }

    /// `DemoSa_CsBridge_Fade`: in over `kREG(17) + 10` frames.
    fn cs_bridge_fade(&mut self) {
        self.fade_timer += 1.0;
        let fade_duration = KREG_17 + 10.0;
        let a = if fade_duration <= self.fade_timer { 255 } else { ((self.fade_timer / fade_duration) * 255.0) as i32 };
        self.alpha = a;
        self.shadow_alpha = a as u8;
    }

    /// `DemoSa_CsBridge_StillHidden`.
    fn cs_bridge_still_hidden(&mut self) {
        self.action = DEMOSA_ACTION_BRIDGE_INVISIBLE;
        self.draw_config = DEMOSA_DRAW_NOTHING;
        self.shadow_alpha = 0;
    }

    /// `DemoSa_CsBridge_LookAtLink`: from hidden (the last cue 4), placed by the cue and faded in;
    /// else at once, `gSariaWaitOnBridgeAnim` looping.
    fn cs_bridge_look_at_link(&mut self, play: &PlayState) {
        if self.cue_id == 4 {
            self.set_start_pos_rot_from_cue(play, 1);
            self.action = DEMOSA_ACTION_BRIDGE_FADE_IN;
            self.draw_config = DEMOSA_DRAW_XLU;
            self.is_holding_ocarina = false;
            self.shadow_alpha = 0;
        } else {
            self.animation_change(play, ANIM_WAIT_ON_BRIDGE, ANIMMODE_LOOP, 0.0, false);
            self.action = DEMOSA_ACTION_BRIDGE_LOOKING_SAD;
            self.draw_config = DEMOSA_DRAW_OPA;
            self.is_holding_ocarina = false;
            self.shadow_alpha = 255;
        }
        self.set_eyes(SARIA_EYE_SAD);
    }

    /// `DemoSa_CsBridge_CheckFadeFinished`.
    fn cs_bridge_check_fade_finished(&mut self) {
        if self.fade_timer >= KREG_17 + 10.0 {
            self.action = DEMOSA_ACTION_BRIDGE_LOOKING_SAD;
            self.draw_config = DEMOSA_DRAW_OPA;
            self.is_holding_ocarina = false;
            self.shadow_alpha = 255;
        }
    }

    /// `DemoSa_CsBridge_ClutchOcarina`.
    fn cs_bridge_clutch_ocarina(&mut self, play: &PlayState) {
        self.animation_change(play, ANIM_HOLD_OCARINA, ANIMMODE_LOOP, 0.0, false);
        self.action = DEMOSA_ACTION_BRIDGE_CLUTCH_OCARINA;
        self.draw_config = DEMOSA_DRAW_OPA;
        self.is_holding_ocarina = true;
        self.shadow_alpha = 255;
        self.set_eyes(SARIA_EYE_CLOSED);
    }

    /// `DemoSa_CsBridge_GiveOcarina`.
    fn cs_bridge_give_ocarina(&mut self, play: &PlayState) {
        self.animation_change(play, ANIM_GIVE_LINK_OCARINA, ANIMMODE_ONCE, -8.0, false);
        self.action = DEMOSA_ACTION_BRIDGE_GIVE_OCARINA;
        self.draw_config = DEMOSA_DRAW_OPA;
        self.is_holding_ocarina = true;
        self.shadow_alpha = 255;
    }

    /// `DemoSa_CsBridge_HoldOutOcarina`.
    fn cs_bridge_hold_out_ocarina(&mut self, play: &PlayState, anim_finished: bool) {
        if anim_finished {
            self.animation_change(play, ANIM_HOLD_OUT_OCARINA, ANIMMODE_LOOP, 0.0, false);
        }
    }

    /// `DemoSa_CsBridge_CheckNextAction`: a new cue on channel 1.
    fn cs_bridge_check_next_action(&mut self, play: &PlayState) {
        let Some(cue) = Self::get_cue(play, 1) else { return };
        let next = cue.action as i32;
        if next != self.cue_id {
            match next {
                4 => self.cs_bridge_still_hidden(),
                12 => self.cs_bridge_look_at_link(play),
                13 => self.cs_bridge_clutch_ocarina(play),
                14 => self.cs_bridge_give_ocarina(play),
                _ => log::debug!("Demo_Sa_inPresent_Check_DemoMode: There is no such action!!!!!!!!"),
            }
            self.cue_id = next;
        }
    }
}

impl ActorImpl for DemoSa {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `DemoSa_Update`: the action (the bridge's; the others aren't ported).
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            // DemoSa_Action_BridgeInvisible.
            DEMOSA_ACTION_BRIDGE_INVISIBLE => self.cs_bridge_check_next_action(play),
            // DemoSa_Action_BridgeFadeIn.
            DEMOSA_ACTION_BRIDGE_FADE_IN => {
                self.update_bg_check_info(play);
                self.update_skel_anime();
                self.cs_bridge_fade();
                self.cs_bridge_check_fade_finished();
            }
            // DemoSa_Action_BridgeLookingSad, DemoSa_Action_BridgeClutchOcarina.
            DEMOSA_ACTION_BRIDGE_LOOKING_SAD | DEMOSA_ACTION_BRIDGE_CLUTCH_OCARINA => {
                self.update_bg_check_info(play);
                self.update_skel_anime();
                self.cs_bridge_check_next_action(play);
            }
            // DemoSa_Action_BridgeGiveOcarina.
            DEMOSA_ACTION_BRIDGE_GIVE_OCARINA => {
                self.update_bg_check_info(play);
                let done = self.update_skel_anime();
                self.cs_bridge_hold_out_ocarina(play, done);
                self.cs_bridge_check_next_action(play);
            }
            _ => {}
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.draw_config as u32, self.eye_index as u32, self.mouth_index as u32, self.is_holding_ocarina as u32];
        rs.values = vec![self.alpha as f32, self.shadow_alpha as f32];
        rs
    }

    /// `DemoSa_Draw`: by `drawConfig`, nothing, `DemoSa_DrawOpa` (its env alpha 255) or
    /// `DemoSa_DrawXlu` (its alpha), the face's textures and the hand by `isHoldingOcarina`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(skeleton), Some(joints)) = (&self.skeleton, &rs.joints) else { return };
        if rs.switches.len() < 4 || rs.values.len() < 2 {
            return;
        }
        let draw_config = rs.switches[rs::DRAW_CONFIG] as i32;
        if draw_config == DEMOSA_DRAW_NOTHING {
            return;
        }
        let xlu = draw_config == DEMOSA_DRAW_XLU;
        let (eye, mouth, ocarina) = (rs.switches[rs::EYE] as i16, rs.switches[rs::MOUTH] as i16, rs.switches[rs::OCARINA] != 0);
        // The XLU draw has no limb override (SkelAnime_DrawFlex with NULLs).
        let ocarina = ocarina && !xlu;
        if !BAKED.contains(&(eye, mouth, xlu, ocarina)) {
            log::warn!("Demo_Sa: no bake for eye {eye}, mouth {mouth}, {}, ocarina {ocarina}", if xlu { "xlu" } else { "opa" });
            return;
        }
        let alpha = if xlu { rs.values[rs::ALPHA].clamp(0.0, 255.0) as u8 } else { 255 };
        let bones = skeleton.pose(joints);
        let mut sv = SegmentValues::default();
        sv.env[SEG_ALPHA as usize] = Some([0, 0, 0, alpha]);
        let cmd = DrawCmd {
            mesh: MeshKey::named(keys::bake(&bake_name(eye, mouth, xlu, ocarina))),
            transform: oot_game::play::actor_draw_matrix(rs),
            bones,
            params: eng_gfx::DrawParams { segments: Some(sv), ..Default::default() },
        };
        if xlu {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
        // ActorShadow_DrawCircle (shadowScale 30, shadowAlpha): the circle shadow's stand-in, at
        // full strength once it shows.
        if rs.values[rs::SHADOW_ALPHA] > 0.0 {
            let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
            let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(30.0, 1.0, 30.0));
            out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), shadow));
        }
    }

    /// `DemoSa_Destroy`: `SkelAnime_Free`.
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
