//! `Demo_Tre_Lgt` (`ovl_Demo_Tre_Lgt/z_demo_tre_lgt.c`): the light that rises out of a big
//! chest as it opens on a major item. `En_Box` spawns it as its child; on the chest's frame 10
//! it starts its curve animation from the chest's frame, fades its two parts' alphas
//! (`unk_170`, `unk_174`), flashes (`NA_SE_EV_TRE_BOX_FLASH`) past its frame 30, and goes at
//! the animation's end.
//!
//! Its logic is ported; its draw isn't (a curve skeleton, `SkelCurve_Draw`, with
//! `gTreasureChestCurveSkel`): the frame count the draw would read is kept (`SkelCurve_Update`'s
//! `curFrame`), the curves' joints aren't evaluated.

use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile, cur_sfx_pos};
use oot_game::audio::sfx::{NA_SE_EV_TRE_BOX_FLASH, SfxF32, SfxS8};
use oot_game::play::PlayState;
use oot_game::play_scene::R_UPDATE_RATE;

use crate::en_box::EnBox;

/// `ACTOR_DEMO_TRE_LGT` (`actor_table.h`: 0x00AA).
pub const ACTOR_DEMO_TRE_LGT: i16 = 0x00AA;

/// `Demo_Tre_Lgt_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_DEMO_TRE_LGT, name: "Demo_Tre_Lgt", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "object_box" };

/// `DemoTreLgtInfo` (`sDemoTreLgtInfo`, by `linkAge`): `startFrame`, `endFrame`, `unk_08`,
/// `unk_0C`.
const INFO: [[f32; 4]; 2] = [[1.0, 136.0, 190.0, 40.0], [1.0, 136.0, 220.0, 50.0]];

/// `DemoTreLgtAction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `DEMO_TRE_LGT_ACTION_WAIT`: `func_8099375C`.
    Wait,
    /// `DEMO_TRE_LGT_ACTION_ANIMATE`: `func_80993848`.
    Animate,
}

/// `SkelCurve`'s frame counter (`curFrame`, `endFrame`, `playSpeed`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkelCurveFrames {
    pub cur_frame: f32,
    pub end_frame: f32,
    pub play_speed: f32,
}

impl SkelCurveFrames {
    /// `SkelCurve_Update`'s frame step: true once past the end (held there).
    fn update(&mut self) -> bool {
        self.cur_frame += self.play_speed * R_UPDATE_RATE as f32 * 0.5;
        if (self.play_speed >= 0.0 && self.cur_frame > self.end_frame) || (self.play_speed < 0.0 && self.cur_frame < self.end_frame) {
            self.cur_frame = self.end_frame;
            return true;
        }
        false
    }
}

pub struct DemoTreLgt {
    pub actor: Actor,
    pub skel_curve: SkelCurveFrames,
    pub action: Action,
    /// `unk_170`, `unk_174`: the two parts' alphas.
    pub unk_170: u32,
    pub unk_174: u32,
    pub status: u8,
}

impl DemoTreLgt {
    /// `DemoTreLgt_Init` (`SkelCurve_Init` starts at frame 0).
    pub fn init(actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        Box::new(DemoTreLgt { actor, skel_curve: SkelCurveFrames::default(), action: Action::Wait, unk_170: 255, unk_174: 255, status: 0 })
    }

    fn info(play: &PlayState) -> [f32; 4] {
        INFO[if play.save.adult { 0 } else { 1 }]
    }

    /// `func_8099375C`: on the chest's frame 10 (its parent's), `func_809937B4`.
    fn func_8099375c(&mut self, play: &mut PlayState) {
        let chest = self.actor.parent.and_then(|h| play.actors.downcast::<EnBox>(h)).and_then(|c| c.skel.as_ref()).map(|s| (s.on_frame(10.0), s.cur_frame));
        if let Some((true, cur_frame)) = chest {
            self.func_809937b4(play, cur_frame);
        }
    }

    /// `func_809937B4`: the age's animation from the chest's frame to `endFrame + unk_08`, and
    /// its first step.
    fn func_809937b4(&mut self, play: &mut PlayState, current_frame: f32) {
        self.action = Action::Animate;
        let i = Self::info(play);
        // SkelCurve_SetAnim(skelCurve, sAnimations[linkAge], 1.0f, endFrame + unk_08, currentFrame, 1.0f).
        self.skel_curve = SkelCurveFrames { cur_frame: current_frame, end_frame: i[1] + i[2], play_speed: 1.0 };
        self.skel_curve.update();
    }

    /// `func_80993848`: the alphas by the frame, the flash once past frame 30, and the end.
    fn func_80993848(&mut self, play: &mut PlayState) {
        let current_frame = self.skel_curve.cur_frame;
        let i = Self::info(play);
        if current_frame < i[1] {
            self.unk_170 = 255;
        } else if current_frame <= i[1] + i[2] {
            self.unk_170 = ((((i[1] - current_frame) / i[2]) * 255.0) + 255.0) as u32;
        } else {
            self.unk_170 = 0;
        }
        if current_frame < i[3] {
            self.unk_174 = 255;
        } else if current_frame < i[3] + 10.0 {
            self.unk_174 = ((((i[3] - current_frame) / 10.0) * 255.0) + 255.0) as u32;
        } else {
            self.unk_174 = 0;
        }
        if current_frame > 30.0 && self.status & 1 == 0 {
            self.status |= 1;
            let pos = cur_sfx_pos(play);
            play.audio.play_sfx_general(NA_SE_EV_TRE_BOX_FLASH, pos, 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
        }
        if self.skel_curve.update() {
            self.actor.kill();
        }
    }
}

impl ActorImpl for DemoTreLgt {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `DemoTreLgt_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Wait => self.func_8099375c(play),
            Action::Animate => self.func_80993848(play),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
