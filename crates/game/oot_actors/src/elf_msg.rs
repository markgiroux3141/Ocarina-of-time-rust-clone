//! `Elf_Msg` (`ovl_Elf_Msg/z_elf_msg.c`): a place where Navi calls. While Link stands in its
//! cylinder (or cuboid), it sets Player's `naviTextId` each frame and Navi's `elfMsg`; Navi's
//! talk accepted ends it (`ACTOR_FLAG_TALK` from `func_80A053F0`): it sets its switch flag and is
//! gone.
//!
//! Params: the low byte the text (0x100 plus it); bits 8 to 13 a switch flag (0x3F: none), set
//! when it's done and killing it once set; bit 14 a cuboid rather than a cylinder; bit 15 a C-Up
//! hint (a positive `naviTextId`) rather than a call Navi makes at once (a negative one). Its
//! rotation is more data:
//! - `rot.x`: the radius (or the cuboid's half width), `rot.x * 4` (0: 40); `rot.z` the height,
//!   `rot.z * 4` (0: 40), up from it;
//! - `rot.y` 1 to 0x40: gone once switch flag `rot.y - 1` is set (setting its own flag); -1:
//!   gone once its room is cleared; 0x42 to 0x80: active only once switch flag `rot.y - 0x41`
//!   is set.
//!
//! The Master Quest Deku Tree's: room 0's 0x1F02 calls (text 0x102) on the ground floor at (118,
//! 0, -325), 80 across and 40 up, and twice on the middle floor; 0x0103 at the doors to rooms 1
//! and 10 (flag 1, text 0x103); 0x8001, a C-Up hint at the bottom, gone with flag 5; room 2's
//! three; room 3's 0x3208.
//!
//! The whole overlay is ported. Its draw (`ElfMsg_Draw`, debug builds only) shows the shape only
//! with `R_NAVI_MSG_REGION_ALPHA` set: that register is 0 in the game, so it draws nothing.

use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

use crate::en_elf::EnElf;
use crate::player::Player;

/// `ACTOR_ELF_MSG` (`actor_table.h`).
pub const ACTOR_ELF_MSG: i16 = 0x011B;

/// `Elf_Msg_Profile` (`OBJECT_GAMEPLAY_KEEP`).
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_ELF_MSG, name: "Elf_Msg", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep" };

/// `R_NAVI_MSG_REGION_ALPHA` (`regs.h`: `nREG(87)`), a debug register: 0 in the game.
pub const R_NAVI_MSG_REGION_ALPHA: i16 = 0;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    CallNaviCuboid,
    CallNaviCylinder,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::CallNaviCuboid => "ElfMsg_CallNaviCuboid",
            Action::CallNaviCylinder => "ElfMsg_CallNaviCylinder",
        }
    }
}

/// `PARAMS_GET_U(params, 8, 6)`: the switch flag (0x3F: none).
fn params_switch_flag(params: i16) -> i32 {
    (params as i32 >> 8) & 0x3F
}

/// `ElfMsg_WithinXZDistance`.
fn within_xz_distance(pos1: Vec3, pos2: Vec3, distance: f32) -> bool {
    (pos2.x - pos1.x) * (pos2.x - pos1.x) + (pos2.z - pos1.z) * (pos2.z - pos1.z) < distance * distance
}

/// `ElfMsg_KillCheck` (`ElfMsg2_KillCheck` is the same): gone (setting its own switch flag, if
/// any) once switch flag `rot.y - 1` is set (`rot.y` 1 to 0x40) or, with `rot.y` -1, its room
/// cleared; else gone once its own flag is set. True when it's killed.
pub(crate) fn kill_check(actor: &mut Actor, play: &mut PlayState) -> bool {
    let rot_y = actor.world_rot.y;
    let flag = params_switch_flag(actor.params);
    if rot_y > 0 && rot_y < 0x41 && play.flags.get_switch(rot_y as i32 - 1) {
        log::debug!("{}: mutual destruction", if actor.id == ACTOR_ELF_MSG { "Elf_Msg" } else { "Elf_Msg2" });
        if flag != 0x3F {
            play.flags.set_switch(flag);
        }
        actor.kill();
        true
    } else if rot_y == -1 && play.flags.get_clear(actor.room) {
        log::debug!("{}: mutual destruction", if actor.id == ACTOR_ELF_MSG { "Elf_Msg" } else { "Elf_Msg2" });
        if flag != 0x3F {
            play.flags.set_switch(flag);
        }
        actor.kill();
        true
    } else if flag == 0x3F {
        false
    } else if play.flags.get_switch(flag) {
        actor.kill();
        true
    } else {
        false
    }
}

pub struct ElfMsg {
    pub actor: Actor,
    pub action: Action,
}

impl ElfMsg {
    /// `ElfMsg_Init`: unless already done (`ElfMsg_KillCheck`), its size from `rot.x` and `rot.z`
    /// (`* 0.04`, 0 for 0.4), the cuboid or the cylinder by params bit 14, the shape unrotated.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut m = ElfMsg { actor, action: Action::CallNaviCylinder };
        log::debug!("Elf_Msg: conditions for disappearing {}", params_switch_flag(m.actor.params));
        if m.actor.shape_rot.y >= 0x41 {
            log::debug!("Elf_Msg: conditions for appearing {}", m.actor.shape_rot.y - 0x41);
        }
        if !kill_check(&mut m.actor, play) {
            let a = &mut m.actor;
            // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 1000), cullingVolumeDistance 1000 (the cull
            // zone isn't ported).
            a.scale = Vec3::ONE;
            if a.world_rot.x == 0 {
                a.scale.z = 0.4;
                a.scale.x = 0.4;
            } else {
                a.scale.x = a.world_rot.x as f32 * 0.04;
                a.scale.z = a.scale.x;
            }
            if a.world_rot.z == 0 {
                a.scale.y = 0.4;
            } else {
                a.scale.y = a.world_rot.z as f32 * 0.04;
            }
            m.action = if a.params & 0x4000 != 0 { Action::CallNaviCuboid } else { Action::CallNaviCylinder };
            a.shape_rot.x = 0;
            a.shape_rot.y = 0;
            a.shape_rot.z = 0;
        }
        Box::new(m)
    }

    /// `ElfMsg_GetMessageId`: 0x100 plus the low byte; negative (Navi calls at once) without
    /// params bit 15.
    pub fn get_message_id(&self) -> i16 {
        let id = (self.actor.params as i32 & 0xFF) + 0x100;
        if self.actor.params as u16 & 0x8000 != 0 { id as i16 } else { -id as i16 }
    }

    /// `player->naviTextId = ElfMsg_GetMessageId(this); navi->elfMsg = this`.
    fn call_navi(&self, play: &mut PlayState) {
        let id = self.get_message_id();
        let Some(p) = play.player.and_then(|h| play.actors.downcast_mut::<Player>(h)) else { return };
        p.navi_text_id = id;
        let navi = p.navi_actor;
        if let Some(n) = navi.and_then(|h| play.actors.downcast_mut::<EnElf>(h)) {
            n.elf_msg = play.cur_actor;
        }
    }

    fn player_pos(play: &PlayState) -> Option<Vec3> {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos)
    }

    /// `ElfMsg_CallNaviCuboid`: Link within `100 * scale` across in x and z, and up to
    /// `100 * scale.y` above it.
    fn call_navi_cuboid(&mut self, play: &mut PlayState) {
        let Some(pp) = Self::player_pos(play) else { return };
        let (p, s) = (self.actor.world_pos, self.actor.scale);
        if (pp.x - p.x).abs() < 100.0 * s.x && p.y <= pp.y && (pp.y - p.y) < 100.0 * s.y && (pp.z - p.z).abs() < 100.0 * s.z {
            self.call_navi(play);
        }
    }

    /// `ElfMsg_CallNaviCylinder`: Link within `100 * scale.x` across, and up to `100 * scale.y`
    /// above it.
    fn call_navi_cylinder(&mut self, play: &mut PlayState) {
        let Some(pp) = Self::player_pos(play) else { return };
        let (p, s) = (self.actor.world_pos, self.actor.scale);
        if within_xz_distance(pp, p, s.x * 100.0) && p.y <= pp.y && (pp.y - p.y) < 100.0 * s.y {
            self.call_navi(play);
        }
    }
}

impl ActorImpl for ElfMsg {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ElfMsg_Update`: unless done, Navi's talk accepted (`Actor_TalkOfferAccepted`) sets its
    /// switch flag and kills it; else, active (`rot.y` up to 0x41 or over 0x80, or its appearing
    /// flag set), its call.
    fn update(&mut self, play: &mut PlayState) {
        if kill_check(&mut self.actor, play) {
            return;
        }
        if oot_game::npc::process_talk_request(&mut self.actor) {
            let flag = params_switch_flag(self.actor.params);
            if flag != 0x3F {
                play.flags.set_switch(flag);
            }
            self.actor.kill();
            return;
        }
        // @bug (game): `<= 0x41` keeps rot.y 0x41 (switch flag 0) always active.
        let rot_y = self.actor.world_rot.y;
        if rot_y <= 0x41 || rot_y > 0x80 || play.flags.get_switch(rot_y as i32 - 0x41) {
            match self.action {
                Action::CallNaviCuboid => self.call_navi_cuboid(play),
                Action::CallNaviCylinder => self.call_navi_cylinder(play),
            }
        }
    }

    /// `ElfMsg_Draw`: nothing while `R_NAVI_MSG_REGION_ALPHA` is 0, which it always is.
    fn draw(&self, _rs: &RenderState, _play: &PlayState, _view: &ViewInfo, _out: &mut DrawOut) {
        if R_NAVI_MSG_REGION_ALPHA == 0 {
            return;
        }
        // The region in its prim colour (255, 100, 100 for a C-Up hint, else white):
        // sMaterialDL, then sCubeDL or sCylinderDL, the overlay's own data, not baked.
        log::debug!("Elf_Msg: the debug region isn't baked");
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
