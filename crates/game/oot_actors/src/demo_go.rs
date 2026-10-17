//! `Demo_Go` (`ovl_Demo_Go/z_demo_go.c`): the three Gorons of the Goron's Ruby cutscene
//! (`gDMTGoronRubyCs`, Death Mountain Trail's layer 5), who drop in after Darunia has given Link
//! the Ruby and chase him off down the trail.
//!
//! Each follows its cue on its own channel (`DemoGo_GetCueChannel`: params 0 on 3, 1 on 4, any
//! other on 5). Hidden (`drawConfig` 0) until its cue 2 (`func_8097CE20`): then drawn, placed
//! along the cue from its start to its end over its frames (`func_8097CB0C`, turned to its
//! rotation), the first two with `NA_SE_EV_OBJECT_FALL` (`func_8097C8A8`). At the cue's end frame
//! it lands (`func_8097CA78`): 11 puffs of dust (`func_80033480`) and `NA_SE_EN_DODO_M_GND`. On cue
//! 3 it plays `gGoronAnim_004930` once, blinking (`func_8097C930`); then the walk
//! (`gGoronAnim_0029A8`, looped, an 8-frame morph): 8 frames turning to its cue's yaw at 0x96 a
//! frame (`func_8097CCE0`) on the spot, then at 1.2 a frame (`func_8097CC08`), the footsteps
//! (`NA_SE_EN_MORIBLIN_WALK` on frames 12 and 25). Every state from the fall on keeps it on the
//! floor (`Actor_UpdateBgCheckInfo`, floor and walls).
//!
//! Drawn (`func_8097D29C`) as `gGoronSkel` (`object_oF1d_map`) with its eye on segment 8
//! (`gGoronCsEyeOpenTex`, `...HalfTex`, `...ClosedTex`) and `gGoronCsMouthSmileTex` on 9: the same
//! skeleton and textures as `En_Go2`, so `En_Go2`'s bakes (its eyes 1 to 3, its smile); the circle
//! shadow (`ActorShadow_DrawCircle`, scale 30) once it has a floor.
//!
//! The whole overlay is ported. Placed only on Death Mountain Trail: layer 5 (the Ruby's cutscene)
//! and layer 6 (`gDMTUnusedCs`, which has no cues: they stay hidden), params 0, 1 and 2.
//!
//! Params: `params` whole, which Goron (0: cue channel 3 and the fall's sound; 1: channel 4 and
//! the sound; any other, 2 placed: channel 5, no sound).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_gfx::{DrawCmd, MeshKey};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EN_DODO_M_GND, SfxPos};
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;

/// `ACTOR_DEMO_GO` (`actor_table.h`: 0x00CA).
pub const ACTOR_DEMO_GO: i16 = 0x00CA;
/// `OBJECT_OF1D_MAP` (`object_table.h`: 0x00C9).
pub const OBJECT: &str = "object_oF1d_map";

/// `Demo_Go_Profile`: `ACTORCAT_NPC`, `FLAGS` (`ACTOR_FLAG_UPDATE_CULLING_DISABLED`).
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_DEMO_GO, name: "Demo_Go", category: ACTORCAT_NPC, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `this->actor.params` (`DemoGo_GetCueChannel`'s switch, `func_8097C8A8`'s check): which Goron.
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[
        (0, "the first Goron: cue channel 3, its fall heard"),
        (1, "the second: cue channel 4, its fall heard"),
        (2, "the third: cue channel 5 (any other value too), its fall silent"),
    ]),
    "which Goron of the Ruby's cutscene (its cue channel)",
);

/// Demo_Go's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false };

/// `NA_SE_EV_OBJECT_FALL` (`environmentbank_table.h`: 0x28A0).
pub const NA_SE_EV_OBJECT_FALL: u16 = 0x28A0;
/// `NA_SE_EN_MORIBLIN_WALK` (`enemybank_table.h`: 0x38B8).
pub const NA_SE_EN_MORIBLIN_WALK: u16 = 0x38B8;

/// `sEyeTextures`: `gGoronCsEyeOpenTex`, `gGoronCsEyeHalfTex`, `gGoronCsEyeClosedTex`, which are
/// `En_Go2`'s eye textures 1 to 3 (its 0 is `gGoronCsEyeClosed2Tex`).
pub const EYE_TEXTURES: [&str; 3] = ["gGoronCsEyeOpenTex", "gGoronCsEyeHalfTex", "gGoronCsEyeClosedTex"];
/// `gGoronCsMouthSmileTex`: `En_Go2`'s mouth 1.
const EN_GO2_MOUTH_SMILE: usize = 1;

/// `gGoronAnim_004930` (played once on cue 3), `gGoronAnim_0029A8` (the walk).
pub const ANIM_004930: &str = "gGoronAnim_004930";
pub const ANIM_0029A8: &str = "gGoronAnim_0029A8";

/// The debug registers `kREG(11)` to `kREG(16)` (0: they start zeroed).
const KREG_11: f32 = 0.0;
const KREG_12: i32 = 0;
const KREG_13: i32 = 0;
const KREG_14: i32 = 0;
const KREG_15: f32 = 0.0;
const KREG_16: i32 = 0;

/// The bake `func_8097D29C` draws with eye `eye` (`sEyeTextures`): `En_Go2`'s.
pub fn bake_name(eye: usize) -> String {
    crate::en_go2::bake_name(eye.min(EYE_TEXTURES.len() - 1) + 1, EN_GO2_MOUTH_SMILE)
}

pub struct DemoGo {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    /// `unk_190`: the eye (`sEyeTextures`).
    pub unk_190: i16,
    /// `unk_192`: the blink's timer.
    pub unk_192: i16,
    /// `action`: the index into `D_8097D44C`.
    pub action: i32,
    /// `drawConfig`: the index into `D_8097D468` (0 nothing, 1 the skeleton).
    pub draw_config: i32,
    /// `unk_19C`: the frames walked on the spot (to 8).
    pub unk_19c: f32,
    anim_004930: Anim,
    anim_0029a8: Anim,
    not_ported_logged: bool,
}

/// `DECR`: 0 when it was 0, else the decremented value.
fn decr(x: &mut i16) -> i16 {
    if *x == 0 {
        0
    } else {
        *x -= 1;
        *x
    }
}

impl DemoGo {
    /// `DemoGo_Init`: the circle shadow (30), `gGoronSkel` with `gGoronAnim_004930` set once,
    /// action 0.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, "gGoronSkel")?, assets.animation(OBJECT, ANIM_004930)?, assets.animation(OBJECT, ANIM_0029A8)?)) })();
        let (skeleton, anim_004930, anim_0029a8) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("Demo_Go: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = DemoGo {
            actor,
            // SkelAnime_InitFlex(.., &gGoronSkel, NULL, NULL, NULL, 0).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            unk_190: 0,
            unk_192: 0,
            action: 0,
            draw_config: 0,
            unk_19c: 0.0,
            anim_004930,
            anim_0029a8,
            not_ported_logged: false,
        };
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 30).
        this.actor.shape_y_offset = 0.0;
        let a = this.anim_004930.clone();
        let end = a.last_frame();
        this.skel.change(a, 1.0, 0.0, end, ANIMMODE_ONCE, 0.0);
        this.action = 0;
        Box::new(this)
    }

    /// `DemoGo_GetCueChannel`: 3, 4, else 5.
    pub fn get_cue_channel(&self) -> usize {
        match TYPE.get(self.actor.params) {
            0 => 3,
            1 => 4,
            _ => 5,
        }
    }

    /// The cue on its channel while a script runs.
    fn cue(&self, play: &PlayState) -> Option<CsCmdActorCue> {
        if play.cs_ctx.state == CS_STATE_IDLE {
            return None;
        }
        play.cs_ctx.npc_actions[self.get_cue_channel()]
    }

    /// `func_8097C8A8`: the first two Gorons' fall, `NA_SE_EV_OBJECT_FALL` from a fixed point for 20
    /// frames. @bug (game): the position is projected through the view (`SkinMatrix_Vec3fMtxFMultXYZW`)
    /// and then handed on as a world position, which `SfxSource_PlaySfxAtFixedWorldPos` projects
    /// again: the sound plays from wherever that doubly projected point lands, not from the Goron.
    fn func_8097c8a8(&self, play: &mut PlayState) {
        let t = TYPE.get(self.actor.params);
        if t == 0 || t == 1 {
            let sp20 = (play.view_proj * self.actor.world_pos.extend(1.0)).truncate();
            play.sfx_source_play_sfx_at_fixed_world_pos(sp20, 20, NA_SE_EV_OBJECT_FALL);
        }
    }

    /// `func_8097C930`: the blink, every 60 to 120 frames.
    fn func_8097c930(&mut self, play: &mut PlayState) {
        if decr(&mut self.unk_192) == 0 {
            self.unk_192 = play.rand.s16_offset(60, 60);
        }
        self.unk_190 = self.unk_192;
        if self.unk_190 >= 3 {
            self.unk_190 = 0;
        }
    }

    fn sfx_pos(play: &PlayState) -> SfxPos {
        play.cur_actor.map(SfxPos::Actor).unwrap_or(SfxPos::Default)
    }

    /// `func_8097C9B8`: the landing's thud.
    fn func_8097c9b8(&self, play: &mut PlayState) {
        let pos = Self::sfx_pos(play);
        play.audio.play_sfx_at_pos(pos, NA_SE_EN_DODO_M_GND);
    }

    /// `func_8097C9DC`: a footstep on frames 12 and 25 of the walk.
    fn func_8097c9dc(&self, play: &mut PlayState) {
        if self.skel.on_frame(12.0) || self.skel.on_frame(25.0) {
            let pos = Self::sfx_pos(play);
            play.audio.play_sfx_at_pos(pos, NA_SE_EN_MORIBLIN_WALK);
        }
    }

    /// `func_8097CA30`: `Actor_UpdateBgCheckInfo(75, 30, 30, floor and walls)`.
    fn func_8097ca30(&mut self, play: &PlayState) {
        self.actor.update_bg_check_info(&play.col, 75.0, 30.0, 30.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
    }

    /// `func_8097CA78`: the landing: 11 puffs of dust 100 across (`func_80033480`), the thud.
    fn func_8097ca78(&self, play: &mut PlayState) {
        let vec = self.actor.world_pos;
        func_80033480(play, vec, KREG_11 + 100.0, KREG_12 + 0xA, (KREG_13 + 0x12C) as i16, KREG_14 as i16, 0);
        self.func_8097c9b8(play);
    }

    /// `func_8097CB0C`: from the cue's start to its end by `Environment_LerpWeight` of the frame,
    /// turned to its yaw.
    fn func_8097cb0c(&mut self, play: &PlayState) {
        let Some(cue) = self.cue(play) else { return };
        let lerp = oot_game::env::lerp_weight(cue.end_frame, cue.start_frame, play.cs_ctx.frames);
        let start = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
        let end = Vec3::new(cue.end_pos.x as f32, cue.end_pos.y as f32, cue.end_pos.z as f32);
        self.actor.world_pos.x = (end.x - start.x) * lerp + start.x;
        self.actor.world_pos.y = (end.y - start.y) * lerp + start.y;
        self.actor.world_pos.z = (end.z - start.z) * lerp + start.z;
        self.actor.world_rot.y = cue.rot[1];
        self.actor.shape_rot.y = cue.rot[1];
    }

    /// `func_8097CC08`: the walk at `1.2 / 8` per frame walked while under 8, else 1.2
    /// (`Actor_MoveXZGravity`). Only action 6 calls it, with `unk_19C` already 8: always 1.2.
    fn func_8097cc08(&mut self) {
        let something = self.unk_19c;
        if something < 8.0 {
            self.actor.speed_xz = (((KREG_15 * 0.01) + 1.2) / 8.0) * something;
        } else {
            self.actor.speed_xz = (KREG_15 * 0.01) + 1.2;
        }
        self.actor.move_forward();
    }

    /// `func_8097CCC0`: `Actor_MoveXZGravity` (its speed still 0: on the spot).
    fn func_8097ccc0(&mut self) {
        self.actor.move_forward();
    }

    /// `func_8097CCE0`: turned to the cue's yaw, at most 0x96 a frame. @bug (game): the difference
    /// is taken in 32 bits without wrapping (`(s16)cue->rot.y - world.rot.y`), so a turn across
    /// 0x8000 goes the long way round (none of the Ruby's cues asks for one).
    fn func_8097cce0(&mut self, play: &PlayState) {
        let Some(cue) = self.cue(play) else { return };
        let this_rot_y = self.actor.world_rot.y as i32;
        let rot_y_delta = cue.rot[1] as i32 - this_rot_y;
        let new_rot_y = if rot_y_delta > -(KREG_16 + 0x96) && rot_y_delta < KREG_16 + 0x96 {
            cue.rot[1] as i32
        } else if rot_y_delta > 0 {
            (this_rot_y + KREG_16) + 0x96
        } else {
            (this_rot_y - KREG_16) - 0x96
        };
        self.actor.shape_rot.y = new_rot_y as i16;
        self.actor.world_rot.y = new_rot_y as i16;
    }

    /// `DemoGo_UpdateSkelAnime`.
    fn update_skel_anime(&mut self) -> bool {
        self.skel.update()
    }

    /// `func_8097CDB0`: its cue is `cue_id`.
    fn func_8097cdb0(&self, play: &PlayState, cue_id: u16) -> bool {
        self.cue(play).is_some_and(|c| c.action == cue_id)
    }

    /// `func_8097CE10`.
    fn func_8097ce10(&mut self) {
        self.action = 1;
    }

    /// `func_8097CE20`: cue 2: shown, falling, its sound.
    fn func_8097ce20(&mut self, play: &mut PlayState) {
        if self.func_8097cdb0(play, 2) {
            self.action = 2;
            self.draw_config = 1;
            self.func_8097cb0c(play);
            self.func_8097c8a8(play);
        }
    }

    /// `func_8097CE78`: the cue's end frame reached: landed.
    fn func_8097ce78(&mut self, play: &mut PlayState) {
        let Some(cue) = self.cue(play) else { return };
        if play.cs_ctx.frames >= cue.end_frame {
            self.func_8097ca78(play);
            self.action = 3;
        }
    }

    /// `func_8097CEEC`: cue 3.
    fn func_8097ceec(&mut self, play: &PlayState) {
        if self.func_8097cdb0(play, 3) {
            self.action = 4;
        }
    }

    /// `func_8097CF20`: `gGoronAnim_004930` over: the walk, looped, an 8-frame morph.
    fn func_8097cf20(&mut self, arg2: bool) {
        if arg2 {
            let a = self.anim_0029a8.clone();
            let end = a.last_frame();
            self.skel.change(a, 1.0, 0.0, end, ANIMMODE_LOOP, -8.0);
            self.action = 5;
            self.unk_19c = 0.0;
        }
    }

    /// `func_8097CF9C`: 8 frames on the spot.
    fn func_8097cf9c(&mut self) {
        self.unk_19c += 1.0;
        if self.unk_19c >= 8.0 {
            self.action = 6;
        }
    }

    /// `D_8097D44C[action]`.
    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            // func_8097CFDC.
            0 => self.func_8097ce10(),
            // func_8097CFFC.
            1 => self.func_8097ce20(play),
            // func_8097D01C.
            2 => {
                self.func_8097cb0c(play);
                self.func_8097ca30(play);
                self.func_8097ce78(play);
            }
            // func_8097D058.
            3 => {
                self.func_8097ca30(play);
                self.func_8097ceec(play);
            }
            // func_8097D088.
            4 => {
                self.func_8097ca30(play);
                let anim_finished = self.update_skel_anime();
                self.func_8097c930(play);
                self.func_8097cf20(anim_finished);
            }
            // func_8097D0D0.
            5 => {
                self.func_8097cce0(play);
                self.func_8097ccc0();
                self.func_8097ca30(play);
                self.update_skel_anime();
                self.func_8097c930(play);
                self.func_8097c9dc(play);
                self.func_8097cf9c();
            }
            // func_8097D130.
            _ => {
                self.func_8097cce0(play);
                self.func_8097cc08();
                self.func_8097ca30(play);
                self.update_skel_anime();
                self.func_8097c930(play);
                self.func_8097c9dc(play);
            }
        }
    }
}

/// Indices into the render state's switches.
mod rs {
    pub const DRAW_CONFIG: usize = 0;
    pub const EYE: usize = 1;
    pub const SHADOW: usize = 2;
    pub const SWITCHES: usize = 3;
}

impl ActorImpl for DemoGo {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `DemoGo_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if !(0..7).contains(&self.action) {
            if !self.not_ported_logged {
                log::debug!("Demo_Go: The main mode is wrong!!!!!!!!!!!!!!!!!!!!!!!!!");
                self.not_ported_logged = true;
            }
            return;
        }
        self.run_action(play);
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        // Actor_Draw's shadow (shape.shadowAlpha 255), whatever the draw config.
        let shadow = oot_game::draw::actor_shadow_circle(&self.actor, 30.0);
        r.switches = vec![self.draw_config.max(0) as u32, self.unk_190.clamp(0, 2) as u32, shadow.is_some() as u32];
        r.values = shadow.unwrap_or(Mat4::IDENTITY).to_cols_array().to_vec();
        r
    }

    /// `DemoGo_Draw`: by `drawConfig`, nothing (`func_8097D290`) or `func_8097D29C`: the skeleton
    /// opaque, its eye on segment 8, the smile on 9.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.len() < rs::SWITCHES {
            return;
        }
        if rs.switches[rs::SHADOW] != 0 && rs.values.len() == 16 {
            let m = Mat4::from_cols_array(&rs.values[..16].try_into().unwrap());
            out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), m));
        }
        let Some(joints) = &rs.joints else { return };
        if rs.switches[rs::DRAW_CONFIG] != 1 {
            return;
        }
        let bones = self.skeleton.pose(joints);
        let key = MeshKey::named(keys::bake(&bake_name(rs.switches[rs::EYE] as usize)));
        out.opa.push(DrawCmd { mesh: key, transform: actor_draw_matrix(rs), bones, params: Default::default() });
    }

    /// `DemoGo_Destroy`: `SkelAnime_Free`.
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
