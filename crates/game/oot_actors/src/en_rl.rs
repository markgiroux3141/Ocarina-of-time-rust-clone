//! `En_Rl` (`ovl_En_Rl/z_en_rl.c`): Rauru, in two of the Chamber of the Sages' scripts.
//!
//! **Type 0, the Light Medallion** (`gChamberOfSagesLightMedallionCs`, the Chamber's layer 4; also
//! placed on layer 5, Rauru's last message): hidden until his first update, which shows him and
//! turns Link to face him (`func_80AE7668`: Player's `world.rot.y` and `shape.rot.y` Rauru's yaw +
//! 0x8000). He idles (`object_rl_Anim_000A3C`) until cue 3 on channel 0, plays
//! `object_rl_Anim_00040C` once, then loops `object_rl_Anim_000830`. In that last state, on the
//! Chamber's layer 4 with cue 2 on channel 6 (`func_80AE7590`), he spawns `Demo_Effect`
//! `DEMO_EFFECT_MEDAL_LIGHT` (0xE) 80 above Link and gives the Light Medallion
//! (`Item_Give(ITEM_MEDALLION_LIGHT)`), once.
//!
//! **Type 2, the sages' Rauru** (Inside Ganon's Castle's layers 0 to 3, tilted 0x2000; the
//! Chamber's layer 6): hidden (action 4) until cue 4 on channel 0, then faded in over
//! `kREG(5) + 10` frames, drawn translucent (`drawConfig` 2), then solid (action 7). A cue other
//! than 4 fades him back out; the first time, his light ball (`Demo_6K` type 5, 22 above him, his
//! child) is spawned. Cue 3 plays `object_rl_Anim_00040C` then loops `object_rl_Anim_000830`. With
//! `DEBUG_FEATURES` (`func_80AE73D8`), the end of a script resets him to hidden.
//!
//! Every action from the first on keeps him on the floor (`Actor_UpdateBgCheckInfo`, floor and
//! walls) and blinks (every 60 to 120 frames). Drawn by `drawConfig`: nothing,
//! `func_80AE7FDC` (`object_rl_Skel_007B38` opaque, the eye on segments 8 and 9, env alpha 255) or
//! `func_80AE7D94` (translucent, `gActorSetupXluDL` on 0xC, env alpha `alpha`). His circle shadow
//! (50) is the shape's, its alpha written with his fade.
//!
//! The whole overlay is ported. `Demo_6K` type 5 is a placeholder (Phase 19's).
//!
//! Params: `params` whole, the type (2: the sages' Rauru, Ganon's sealing and the Chamber after
//! Ganon; any other, 0 placed: the Chamber's Light Medallion). `world.rot.x`: a tilt, drawn as
//! placed (Inside Ganon's Castle's, 0x2000).
//! Flags: none (type 0 reads `gSaveContext.sceneLayer` 4 and gives the Light Medallion,
//! `Item_Give`).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_gfx::{DrawCmd, DrawImage, DrawParams, MeshKey, SegmentValues, SourceImage};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind, Source};
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::gbi::Dl;
use oot_game::item::ITEM_MEDALLION_LIGHT;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;

use crate::demo_effect::{ACTOR_DEMO_EFFECT, DEMO_EFFECT_MEDAL_LIGHT};

/// `ACTOR_EN_RL` (`actor_table.h`: 0x00A6).
pub const ACTOR_EN_RL: i16 = 0x00A6;
/// `OBJECT_RL` (`object_table.h`: 0x00A7).
pub const OBJECT: &str = "object_rl";
/// `ACTOR_DEMO_6K` (`actor_table.h`: 0x00F5).
const ACTOR_DEMO_6K: i16 = 0x00F5;
/// `SCENE_CHAMBER_OF_THE_SAGES` (`scene_table.h`: 0x44).
const SCENE_CHAMBER_OF_THE_SAGES: u16 = 0x44;

/// `En_Rl_Profile`: `ACTORCAT_NPC`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_RL, name: "En_Rl", category: ACTORCAT_NPC, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `this->actor.params` (`EnRl_Init`'s `params == 2`; `func_80AE73D8`'s too).
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[
        (0, "the Chamber of the Sages: the Light Medallion (any value but 2)"),
        (2, "the sages' Rauru: Ganon's sealing, the Chamber after Ganon (his light ball, Demo_6K 5)"),
    ]),
    "which of Rauru's scripts",
);
/// `world.rot.x`: Inside Ganon's Castle's Rauru is placed tilted (0x2000), drawn so.
pub const TILT: Field = Field::rot("world.rot.x", Source::RotX, 0, 16, Kind::Rotation, "his tilt, drawn as placed");

/// En_Rl's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE, TILT], flags: &[], todo: false, objects: &[], texts: &[] };

/// `D_80AE81A0`: the eyes.
pub const EYE_TEXTURES: [&str; 3] = ["object_rl_Tex_003620", "object_rl_Tex_003960", "object_rl_Tex_003B60"];
/// The animations.
pub const ANIM_IDLE: &str = "object_rl_Anim_000A3C";
pub const ANIM_RAISE: &str = "object_rl_Anim_00040C";
pub const ANIM_RAISED: &str = "object_rl_Anim_000830";

/// The draws: `func_80AE7FDC` (opaque) and `func_80AE7D94` (translucent).
pub const BAKE_OPA: &str = "En_Rl/opa";
pub const BAKE_XLU: &str = "En_Rl/xlu";
/// The eye's segment (8; 9 gets the same texture, the skeleton's lists only call 8), where a draw
/// puts the blink's texture in the bake's place.
const SEG_EYE: u32 = 0x0800_0000;
/// Segment 0xC: `ACTOR_SETUP_OPA_DL` (an end) or `gActorSetupXluDL`.
const SEG_SETUP: u8 = 0x0C;
/// `gDPSetEnvColor(0, 0, 0, alpha)` before the skeleton, as a dynamic colour.
const SEG_ALPHA: u8 = 0x0E;

/// `kREG(5)`, `kREG(18)`: 0 (the debug registers start zeroed).
const KREG_5: f32 = 0.0;
const KREG_18: f32 = 0.0;

/// `drawConfig`.
pub const DRAW_NOTHING: i32 = 0;
pub const DRAW_OPA: i32 = 1;
pub const DRAW_XLU: i32 = 2;

// shared: gActorSetupXluDL (z_actor.c), as demo_sa.rs and demo_du.rs build it.
/// `gActorSetupXluDL`: `gsDPSetRenderMode(G_RM_FOG_SHADE_A, G_RM_AA_ZB_XLU_SURF2 | Z_UPD)`,
/// `gsDPSetAlphaCompare(G_AC_THRESHOLD)`, the end.
fn actor_setup_xlu_dl() -> Vec<(u32, u32)> {
    let mut d = Dl::default();
    d.0.push((0xE200_001C, 0xC810_49F8));
    d.othermode_l(0, 2, 1);
    d.end();
    d.0
}

/// Its meshes: the skeleton opaque and translucent. Rauru's lists call segment 8 only (not 0xC):
/// the translucent bake runs `gActorSetupXluDL` first, standing in for `Gfx_SetupDL_25Xlu`'s
/// render mode, then the env alpha.
pub fn bakes() -> Vec<MeshBake> {
    let tex = |s: &str| BakeSegment::Texture { file: OBJECT.into(), symbol: s.into() };
    [(BAKE_OPA, false), (BAKE_XLU, true)]
        .into_iter()
        .map(|(name, xlu)| MeshBake {
            name: name.into(),
            object: OBJECT.into(),
            segments: vec![
                (0x08, tex(EYE_TEXTURES[0])),
                (0x09, tex(EYE_TEXTURES[0])),
                (SEG_SETUP, BakeSegment::Commands(if xlu { actor_setup_xlu_dl() } else { vec![(0xDF00_0000, 0)] })),
                (SEG_ALPHA, BakeSegment::DynamicColor { env: true, prim: false }),
            ],
            prelude: if xlu { vec![SEG_SETUP, SEG_ALPHA] } else { vec![SEG_ALPHA] },
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "object_rl_Skel_007B38".into(), limbs: Vec::new() },
        })
        .collect()
}

pub struct EnRl {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub eye_texture_index: i16,
    pub timer: i16,
    /// `action`: the index into `sActionFuncs`.
    pub action: i32,
    pub draw_config: i32,
    pub unk_19c: f32,
    pub alpha: i32,
    pub light_ball_spawned: bool,
    pub light_medallion_given: bool,
    /// `D_80AE81AC` (`func_80AE73D8`'s static, one for the overlay; one type-2 Rauru at a time).
    pub cs_seen: bool,
    idle: Anim,
    raise: Anim,
    raised: Anim,
    eyes: Vec<DrawImage>,
    logged: bool,
}

/// `DECR`.
fn decr(x: &mut i16) -> i16 {
    if *x == 0 {
        0
    } else {
        *x -= 1;
        *x
    }
}

impl EnRl {
    /// `EnRl_Init`: the circle shadow (50), then type 2's init (`func_80AE7878`) or the other's
    /// (`func_80AE7544`).
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut actor = actor;
        let Some(assets) = play.assets.clone() else {
            actor.kill();
            return oot_game::spawn::Placeholder::init(actor, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.skeleton(OBJECT, "object_rl_Skel_007B38")?;
            let anim = |s: &str| assets.animation(OBJECT, s);
            let anims = (anim(ANIM_IDLE)?, anim(ANIM_RAISE)?, anim(ANIM_RAISED)?);
            let mut eyes = Vec::new();
            for s in EYE_TEXTURES {
                let t = assets.pack.texture(OBJECT, s)?;
                eyes.push(DrawImage { width: t.width, height: t.height, rgba: t.rgba.clone().into() });
            }
            Ok((skeleton, anims, eyes))
        })();
        let (skeleton, (idle, raise, raised), eyes) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Rl: {e:#}");
                actor.kill();
                return oot_game::spawn::Placeholder::init(actor, play);
            }
        };
        let mut this = EnRl {
            actor,
            // SkelAnime_InitFlex(.., object_rl_Skel_007B38, object_rl_Anim_000A3C, NULL, NULL, 0).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(idle.clone())),
            skeleton,
            eye_texture_index: 0,
            timer: 0,
            action: 0,
            draw_config: DRAW_NOTHING,
            unk_19c: 0.0,
            alpha: 0,
            light_ball_spawned: false,
            light_medallion_given: false,
            cs_seen: false,
            idle,
            raise,
            raised,
            eyes,
            logged: false,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 50.0);
        if TYPE.get(this.actor.params) == 2 {
            // func_80AE7878 (its SkelAnime_InitFlex is the one above).
            this.action = 4;
            this.actor.shape_shadow_alpha = 0;
        }
        // func_80AE7544: SkelAnime_InitFlex only.
        Box::new(this)
    }

    /// `func_80AE72D0`: the blink, every 60 to 120 frames.
    fn blink(&mut self, play: &mut PlayState) {
        if decr(&mut self.timer) == 0 {
            self.timer = play.rand.s16_offset(60, 60);
        }
        self.eye_texture_index = self.timer;
        if self.eye_texture_index > 2 {
            self.eye_texture_index = 0;
        }
    }

    /// `func_80AE7358` (`DEBUG_FEATURES`): back to hidden, idling.
    fn func_80ae7358(&mut self) {
        let a = self.idle.clone();
        let end = a.last_frame();
        self.skel.change(a, 1.0, 0.0, end, ANIMMODE_LOOP, 0.0);
        self.action = 4;
        self.draw_config = DRAW_NOTHING;
        self.alpha = 0;
        self.light_ball_spawned = false;
        self.actor.shape_shadow_alpha = 0;
        self.unk_19c = 0.0;
    }

    /// `func_80AE73D8` (`DEBUG_FEATURES`): a script ended, type 2 back to hidden.
    fn func_80ae73d8(&mut self, play: &PlayState) {
        if play.cs_ctx.state == CS_STATE_IDLE {
            if self.cs_seen {
                if TYPE.get(self.actor.params) == 2 {
                    self.func_80ae7358();
                }
                self.cs_seen = false;
            }
        } else if !self.cs_seen {
            self.cs_seen = true;
        }
    }

    /// `func_80AE744C`.
    fn update_bg_check_info(&mut self, play: &PlayState) {
        self.actor.update_bg_check_info(&play.col, 75.0, 30.0, 30.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
    }

    /// The cue on `channel` while a script runs.
    fn cue(play: &PlayState, channel: usize) -> Option<CsCmdActorCue> {
        if play.cs_ctx.state == CS_STATE_IDLE {
            return None;
        }
        play.cs_ctx.npc_actions[channel]
    }

    /// `func_80AE74B4`: the cue on `channel` is `cue_id`.
    fn check_for_cue(play: &PlayState, cue_id: u16, channel: usize) -> bool {
        Self::cue(play, channel).is_some_and(|c| c.action == cue_id)
    }

    /// `func_80AE74FC`: a cue on `channel`, not `cue_id`.
    fn check_for_no_cue(play: &PlayState, cue_id: u16, channel: usize) -> bool {
        Self::cue(play, channel).is_some_and(|c| c.action != cue_id)
    }

    fn player_pos(play: &PlayState) -> Vec3 {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO)
    }

    /// `func_80AE7590`: on the Chamber's layer 4, cue 2 on channel 6: the Light Medallion over
    /// Link and given, once.
    fn func_80ae7590(&mut self, play: &mut PlayState) {
        let given = play.save.scene_layer == 4
            && play.scene_id == SCENE_CHAMBER_OF_THE_SAGES
            && play.cs_ctx.state != CS_STATE_IDLE
            && play.cs_ctx.npc_actions[6].is_some_and(|c| c.action == 2)
            && !self.light_medallion_given;
        if given {
            let p = Self::player_pos(play);
            let pos = Vec3::new(p.x, p.y + 80.0, p.z);
            if let Err(e) = play.actor_spawn(ACTOR_DEMO_EFFECT, pos, [0; 3], DEMO_EFFECT_MEDAL_LIGHT as i16) {
                log::debug!("En_Rl's medallion: {e:?}");
            }
            oot_game::item::item_give(&mut play.save, Some(&mut play.audio), ITEM_MEDALLION_LIGHT);
            self.light_medallion_given = true;
        }
    }

    /// `func_80AE7668`: shown; Link turned to face him.
    fn func_80ae7668(&mut self, play: &mut PlayState) {
        self.draw_config = DRAW_OPA;
        self.action = 1;
        let yaw = self.actor.world_rot.y.wrapping_add(-0x8000i32 as i16);
        if let Some(p) = play.player.and_then(|h| play.actors.actor_mut(h)) {
            p.shape_rot.y = yaw;
            p.world_rot.y = yaw;
        }
    }

    /// `func_80AE7698`: cue 3 on channel 0: `object_rl_Anim_00040C` once.
    fn func_80ae7698(&mut self, play: &PlayState) {
        if Self::check_for_cue(play, 3, 0) {
            let a = self.raise.clone();
            let end = a.last_frame();
            self.skel.change(a, 1.0, 0.0, end, ANIMMODE_ONCE, 0.0);
            self.action = 2;
        }
    }

    /// `func_80AE772C`: `object_rl_Anim_000830` looped once that's done.
    fn func_80ae772c(&mut self, anim_finished: bool) {
        if anim_finished {
            let a = self.raised.clone();
            let end = a.last_frame();
            self.skel.change(a, 1.0, 0.0, end, ANIMMODE_LOOP, 0.0);
            self.action = 3;
        }
    }

    /// `func_80AE78D4`: his light ball (`Demo_6K` 5), `kREG(18) + 22` above him, his child.
    fn func_80ae78d4(&mut self, play: &mut PlayState) {
        let p = self.actor.world_pos;
        let pos = Vec3::new(p.x, KREG_18 + 22.0 + p.y, p.z);
        if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_DEMO_6K, pos, [0; 3], 5) {
            log::debug!("En_Rl's light ball: {e:?}");
        }
    }

    /// `func_80AE7954`: cue 4 on channel 0: fading in.
    fn func_80ae7954(&mut self, play: &PlayState) {
        if Self::check_for_cue(play, 4, 0) {
            self.action = 5;
            self.draw_config = DRAW_XLU;
            self.alpha = 0;
            self.actor.shape_shadow_alpha = 0;
            self.unk_19c = 0.0;
        }
    }

    /// `func_80AE79A4`: the fade, in while cue 4 holds (solid at its end), else out (hidden at
    /// its end).
    fn func_80ae79a4(&mut self, play: &PlayState) {
        let fade = KREG_5 + 10.0;
        if Self::check_for_cue(play, 4, 0) {
            self.unk_19c += 1.0;
            if self.unk_19c >= fade {
                self.action = 7;
                self.draw_config = DRAW_OPA;
                self.unk_19c = fade;
                self.alpha = 255;
                self.actor.shape_shadow_alpha = 255;
                return;
            }
        } else {
            self.unk_19c -= 1.0;
            if self.unk_19c <= 0.0 {
                self.action = 4;
                self.draw_config = DRAW_NOTHING;
                self.unk_19c = 0.0;
                self.alpha = 0;
                self.actor.shape_shadow_alpha = 0;
                return;
            }
        }
        let alpha = ((self.unk_19c / fade) * 255.0) as i32;
        self.alpha = alpha;
        self.actor.shape_shadow_alpha = alpha as u8;
    }

    /// `func_80AE7AF8`: cue 3: `object_rl_Anim_00040C` (an 8-frame morph); another cue but 4:
    /// fading out from solid, the light ball spawned the first time.
    fn func_80ae7af8(&mut self, play: &mut PlayState) {
        if Self::check_for_cue(play, 3, 0) {
            let a = self.raise.clone();
            let frame = a.last_frame();
            self.skel.change(a, 1.0, 0.0, frame, ANIMMODE_ONCE, -8.0);
            self.action = 6;
        } else if Self::check_for_no_cue(play, 4, 0) {
            self.action = 5;
            self.draw_config = DRAW_XLU;
            self.unk_19c = KREG_5 + 10.0;
            self.alpha = 255;
            if !self.light_ball_spawned {
                self.func_80ae78d4(play);
                self.light_ball_spawned = true;
            }
            self.actor.shape_shadow_alpha = 255;
        }
    }

    /// `func_80AE7BF8`: `object_rl_Anim_000830` looped once the raise is done.
    fn func_80ae7bf8(&mut self, anim_finished: bool) {
        if anim_finished {
            let a = self.raised.clone();
            let end = a.last_frame();
            self.skel.change(a, 1.0, 0.0, end, ANIMMODE_LOOP, 0.0);
            self.action = 7;
        }
    }
}

impl ActorImpl for EnRl {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnRl_Update`: `sActionFuncs[action]`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            // func_80AE7798.
            0 => self.func_80ae7668(play),
            // func_80AE77B8.
            1 => {
                self.update_bg_check_info(play);
                self.skel.update();
                self.blink(play);
                self.func_80ae7698(play);
            }
            // func_80AE77F8.
            2 => {
                self.update_bg_check_info(play);
                let done = self.skel.update();
                self.blink(play);
                self.func_80ae772c(done);
            }
            // func_80AE7838.
            3 => {
                self.update_bg_check_info(play);
                self.skel.update();
                self.blink(play);
                self.func_80ae7590(play);
            }
            // func_80AE7C64.
            4 => {
                self.func_80ae7954(play);
                self.func_80ae73d8(play);
            }
            // func_80AE7C94.
            5 => {
                self.update_bg_check_info(play);
                self.skel.update();
                self.blink(play);
                self.func_80ae79a4(play);
                self.func_80ae73d8(play);
            }
            // func_80AE7CE8.
            6 => {
                self.update_bg_check_info(play);
                let done = self.skel.update();
                self.blink(play);
                self.func_80ae7bf8(done);
                self.func_80ae73d8(play);
            }
            // func_80AE7D40.
            7 => {
                self.update_bg_check_info(play);
                self.skel.update();
                self.blink(play);
                self.func_80ae7af8(play);
                self.func_80ae73d8(play);
            }
            _ => {
                if !self.logged {
                    log::debug!("En_Rl: The main mode is wrong!!!!!!!!!!!!!!!!!!!!!!!!!");
                    self.logged = true;
                }
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.draw_config.max(0) as u32, self.eye_texture_index.clamp(0, 2) as u32, self.alpha.clamp(0, 255) as u32];
        rs
    }

    /// `EnRl_Draw`: by `drawConfig`, nothing (`func_80AE7FD0`), `func_80AE7FDC` or `func_80AE7D94`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.len() < 3 {
            return;
        }
        let Some(joints) = &rs.joints else { return };
        let (bake, xlu, alpha) = match rs.switches[0] as i32 {
            DRAW_OPA => (BAKE_OPA, false, 255u8),
            DRAW_XLU => (BAKE_XLU, true, rs.switches[2] as u8),
            DRAW_NOTHING => return,
            _ => {
                log::debug!("En_Rl: The drawing mode is wrong!!!!!!!!!!!!!!!!!!!!!!!!!");
                return;
            }
        };
        let eye = self.eyes[(rs.switches[1] as usize).min(EYE_TEXTURES.len() - 1)].clone();
        let mut sv = SegmentValues::default();
        sv.env[SEG_ALPHA as usize] = Some([0, 0, 0, alpha]);
        let cmd = DrawCmd {
            mesh: MeshKey::named(keys::bake(bake)),
            transform: actor_draw_matrix(rs),
            bones: self.skeleton.pose(joints),
            params: DrawParams { segments: Some(sv), texture_images: vec![SourceImage { source: SEG_EYE, image: eye }], ..Default::default() },
        };
        if xlu {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
    }

    /// `EnRl_Destroy`: `SkelAnime_Free`.
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
