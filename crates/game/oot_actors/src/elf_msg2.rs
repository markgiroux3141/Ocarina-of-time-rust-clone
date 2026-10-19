//! `Elf_Msg2` (`ovl_Elf_Msg2/z_elf_msg2.c`): a tag Navi can be asked about. Targetable, and
//! talked to with C-Up (`ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_WITH_C_UP`), it says
//! its text; when the text closes it's gone, setting its switch flag (or, with `rot.z` 1, it
//! waits to be asked again).
//!
//! Params: the low byte the text (0x100 plus it); bits 8 to 13 a switch flag (0x3F: none), set
//! when it's done and killing it once set. Its rotation is more data: `rot.x` 1 to 7 the attention
//! range (`attentionRangeType` `rot.x - 1`); `rot.y` 1 to 0x40: gone once switch flag
//! `rot.y - 1` is set; -1: gone once its room is cleared; 0x41 to 0x80: targetable only once
//! switch flag `rot.y - 0x41` is set (over 0x80, never: `@bug (game)`, kept); `rot.z` 1: asked
//! again and again.
//!
//! The Master Quest Deku Tree's: room 4's two 0x3F06 (text 0x106, `rot` (2, 26, 0)) 60 above its
//! timed torches, gone once they're lit (flag 0x19); room 10's 0x3F06 above its timed torch (gone
//! with 0x13); room 2's 0x3F04.
//!
//! The whole overlay is ported. Its draw (`ElfMsg2_Draw`, debug builds only) shows the cube only
//! with `R_NAVI_MSG_REGION_ALPHA` set: that register is 0 in the game, so it draws nothing.
//!
//! Params: `text` bits 0 to 7 (`PARAMS_GET_U(params, 0, 8)`: the text, 0x100 plus it);
//! `switchFlag` bits 8 to 13 (set when its text closes; set, it's gone; 0x3F: none).
//! `world.rot.x` (`rotX`): 1 to 7 the attention range (`attentionRangeType` `rotX - 1`), else
//! the default. `world.rot.y` (`rotY`): 1 to 0x40 gone once switch flag
//! `rotY - 1` is set; -1 gone once the room is cleared; 0x41 to 0x80 targetable once switch flag
//! `rotY - 0x41` is set (over 0x80 never). `world.rot.z` (`rotZ`): 1 asked again and again, else
//! gone once asked.
//! Flags: reads switch flag `switchFlag` (set: gone) and sets it when its text closes or the
//! kill check kills it; reads switch flag `rotY - 1` (`rotY` 1 to 0x40), switch flag
//! `rotY - 0x41` (`rotY` 0x41 to 0x80) and the room's clear flag (`rotY` -1).

use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_TALK_WITH_C_UP, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source, TextLine};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

use crate::elf_msg::{R_NAVI_MSG_REGION_ALPHA, kill_check};

/// `ACTOR_ELF_MSG2` (`actor_table.h`).
pub const ACTOR_ELF_MSG2: i16 = 0x0173;

/// `Elf_Msg2_Profile` (`OBJECT_GAMEPLAY_KEEP`).
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_ELF_MSG2, name: "Elf_Msg2", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep", schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 8)`: the text (`ElfMsg2_GetMessageId`).
pub const TEXT: Field = Field::params("text", 0, 8, Kind::Range(0, 0xFF), "the text, 0x100 plus it");
/// `PARAMS_GET_U(params, 8, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 8, 6, Kind::SwitchFlag, "set when its text closes; set, it's gone (0x3F: none)");
/// `world.rot.x`: the attention range.
pub const ROT_X: Field = Field::rot("rotX", Source::RotX, 0, 16, Kind::Range(-0x8000, 0x7FFF), "1 to 7: the attention range (attentionRangeType rotX - 1); else the default");
/// `world.rot.y`: when it's there.
pub const ROT_Y: Field = Field::rot(
    "rotY",
    Source::RotY,
    0,
    16,
    Kind::Range(-0x8000, 0x7FFF),
    "1 to 0x40: gone once switch flag rotY - 1 is set; -1: gone once the room is cleared; 0x41 to 0x80: targetable once switch flag rotY - 0x41 is set (over 0x80: never)",
);
/// `world.rot.z`: asked again.
pub const ROT_Z: Field = Field::rot("rotZ", Source::RotZ, 0, 16, Kind::Range(-0x8000, 0x7FFF), "1: asked again and again; else gone once asked");

/// Elf_Msg2's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TEXT, SWITCH_FLAG, ROT_X, ROT_Y, ROT_Z],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag")).sets().note("when its text closes (not with rotZ 1)"),
        FlagUse::new(Flag::Switch("switchFlag"))
            .reads()
            .sets()
            .note("set: gone; set when rotY's flag or the room's clear kills it")
            .indirect("ElfMsg2_KillCheck: elf_msg::kill_check, the same"),
        FlagUse::new(Flag::Switch("rotY - 1")).reads().note("rotY 1 to 0x40: set, it's gone").indirect("ElfMsg2_KillCheck: elf_msg::kill_check, the same"),
        FlagUse::new(Flag::Switch("rotY - 0x41")).reads().note("rotY 0x41 to 0x80: targetable once set"),
        FlagUse::new(Flag::Clear).reads().note("rotY -1: the room cleared, it's gone").indirect("ElfMsg2_KillCheck: elf_msg::kill_check, the same"),
    ],
    todo: false,
    objects: &[],
    texts: &[TextLine::field(0x0100, "text").note("what Navi says, asked with C-Up")],
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitUntilActivated,
    WaitForTextRead,
    WaitForTextClose,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::WaitUntilActivated => "ElfMsg2_WaitUntilActivated",
            Action::WaitForTextRead => "ElfMsg2_WaitForTextRead",
            Action::WaitForTextClose => "ElfMsg2_WaitForTextClose",
        }
    }
}

pub struct ElfMsg2 {
    pub actor: Actor,
    pub action: Action,
}

impl ElfMsg2 {
    /// `ElfMsg2_Init`: unless already done (`ElfMsg2_KillCheck`), its attention range from
    /// `rot.x`; waiting for its flag (`rot.y` 0x41 and up), or targetable with C-Up with its text;
    /// the shape unrotated.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut m = ElfMsg2 { actor, action: Action::WaitForTextRead };
        // schema: the whole params printed.
        log::debug!(" Elf_Msg2_Actor_ct {:04x}", m.actor.params as u16);
        if !kill_check(&mut m.actor, play) {
            let a = &mut m.actor;
            let rot_x = ROT_X.get(a.world_rot.x);
            if rot_x > 0 && rot_x < 8 {
                a.target_mode = (rot_x - 1) as u8;
            }
            // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 200), cullingVolumeDistance 1000.
            a.scale = Vec3::splat(0.2);
            a.culling_volume_distance = 1000.0;
            if ROT_Y.get(a.world_rot.y) >= 0x41 {
                m.action = Action::WaitUntilActivated;
            } else {
                m.action = Action::WaitForTextRead;
                m.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_WITH_C_UP;
                m.actor.text_id = m.get_message_id();
            }
            let a = &mut m.actor;
            a.shape_rot.x = 0;
            a.shape_rot.y = 0;
            a.shape_rot.z = 0;
        }
        Box::new(m)
    }

    /// `ElfMsg2_GetMessageId`: 0x100 plus the low byte.
    pub fn get_message_id(&self) -> u16 {
        TEXT.get(self.actor.params) as u16 + 0x100
    }

    /// `ElfMsg2_WaitForTextClose`: as the text closes (`Actor_TextboxIsClosing`), gone (its switch
    /// flag set), or with `rot.z` 1 waiting to be asked again.
    fn wait_for_text_close(&mut self, play: &mut PlayState) {
        if oot_game::npc::textbox_is_closing(play) {
            if ROT_Z.get(self.actor.world_rot.z) != 1 {
                self.actor.kill();
                let switch_flag = SWITCH_FLAG.get(self.actor.params) as i32;
                // schema: 0x3F, no switch flag
                if switch_flag != 0x3F {
                    play.flags.set_switch(switch_flag);
                }
            } else {
                self.action = Action::WaitForTextRead;
            }
        }
    }

    /// `ElfMsg2_WaitForTextRead`: asked (`Actor_TalkOfferAccepted`), it waits for the text to close.
    fn wait_for_text_read(&mut self) {
        if oot_game::npc::process_talk_request(&mut self.actor) {
            self.action = Action::WaitForTextClose;
        }
    }

    /// `ElfMsg2_WaitUntilActivated`: with `rot.y` 0x41 to 0x80, switch flag `rot.y - 0x41` set
    /// makes it targetable with C-Up, with its text. (Over 0x80 it waits for good.)
    fn wait_until_activated(&mut self, play: &mut PlayState) {
        let rot_y = ROT_Y.get(self.actor.world_rot.y);
        if (0x41..=0x80).contains(&rot_y) && play.flags.get_switch(rot_y as i32 - 0x41) {
            self.action = Action::WaitForTextRead;
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_WITH_C_UP;
            self.actor.text_id = self.get_message_id();
        }
    }
}

impl ActorImpl for ElfMsg2 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ElfMsg2_Update`: unless done (`ElfMsg2_KillCheck`), its action.
    fn update(&mut self, play: &mut PlayState) {
        if kill_check(&mut self.actor, play) {
            return;
        }
        match self.action {
            Action::WaitUntilActivated => self.wait_until_activated(play),
            Action::WaitForTextRead => self.wait_for_text_read(),
            Action::WaitForTextClose => self.wait_for_text_close(play),
        }
    }

    /// `ElfMsg2_Draw`: nothing while `R_NAVI_MSG_REGION_ALPHA` is 0, which it always is.
    fn draw(&self, _rs: &RenderState, _play: &PlayState, _view: &ViewInfo, _out: &mut DrawOut) {
        if R_NAVI_MSG_REGION_ALPHA == 0 {
            return;
        }
        // The cube in prim colour (100, 100, 255): sMaterialDL, then sCubeDL, the overlay's own
        // data, not baked.
        log::debug!("Elf_Msg2: the debug region isn't baked");
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
