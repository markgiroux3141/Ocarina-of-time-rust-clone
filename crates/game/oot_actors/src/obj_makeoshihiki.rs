//! `Obj_Makeoshihiki` (`ovl_Obj_Makeoshihiki/z_obj_makeoshihiki.c`): a hard-coded push-block
//! puzzle. It spawns its `Obj_Oshihiki` as its child at one of three places its flags pick, and
//! its draw (which draws nothing) sets and clears the flags by where the block rests: draw-time
//! state, run here in `draw_update` (ADR 0037).
//!
//! Params: bits 0..5 the first flag (bit 6 set: none), bits 8..13 the second (bit 14 set: none).
//! `home.rot.z & 1` picks the puzzle (`sBlocks`): 0 a large block (unused in Master Quest's Deku
//! Tree), 1 room 3's small block on the upper floor (-605, -820, -290), down in the pit at
//! (-365, -905, -290) once flag 0x10 is set.
//!
//! The Master Quest Deku Tree's is room 3's, params 0xFF10 at `home.rot.z` 1: the first flag 0x10,
//! the second none (but the draw still writes it: flag 0x3F is cleared while the block rests, as
//! the C does). The whole overlay is ported.
//!
//! Params: `switchFlag1` bits 0 to 5 (`PARAMS_GET_U(params, 0, 6)`: set, the block starts at
//! place 1); `noSwitchFlag1` bit 6 (1: no first flag to read); `switchFlag2` bits 8 to 13
//! (`PARAMS_GET_U(params, 8, 6)`: set, place 2); `noSwitchFlag2` bit 14 (1: no second flag to
//! read); bit 15 unused (the data's 0xFF10). `home.rot.z & 1` (`block`): the puzzle (`sBlocks`:
//! 0 the large block, 1 the Deku Tree's room 3 small block).
//! Flags: reads switch flags `switchFlag1` and `switchFlag2` (unless their bit 6 or 14 is set) at
//! init and in the draw; the draw sets and clears both as the block's resting place says, even
//! a flag its bit marks as none.

use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::audio::sfx::NA_SE_SY_TRE_BOX_APPEAR;
use oot_game::play::PlayState;

use crate::obj_oshihiki::{ACTOR_OBJ_OSHIHIKI, ObjOshihiki, PUSHBLOCK_LARGE_START_ON, PUSHBLOCK_SMALL_START_ON};

/// `ACTOR_OBJ_MAKEOSHIHIKI` (`actor_table.h`: 0x017D).
pub const ACTOR_OBJ_MAKEOSHIHIKI: i16 = 0x017D;
pub const OBJECT: &str = "gameplay_dangeon_keep";

/// `Obj_Makeoshihiki_Profile`: `ACTOR_FLAG_DRAW_CULLING_DISABLED`; its update is `Actor_Noop`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_MAKEOSHIHIKI, name: "Obj_Makeoshihiki", category: ACTORCAT_PROP, flags: ACTOR_FLAG_DRAW_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 6)`: the first flag.
pub const SWITCH_FLAG_1: Field = Field::params("switchFlag1", 0, 6, Kind::SwitchFlag, "set: the block starts at place 1; set or cleared by where it rests");
/// `PARAMS_GET_U(params, 6, 1)`: no first flag.
pub const NO_SWITCH_FLAG_1: Field = Field::params("noSwitchFlag1", 6, 1, Kind::Named(&[(0, "switchFlag1 read"), (1, "switchFlag1 not read")]), "whether the first flag is read");
/// `PARAMS_GET_U(params, 8, 6)`: the second flag.
pub const SWITCH_FLAG_2: Field = Field::params("switchFlag2", 8, 6, Kind::SwitchFlag, "set: the block starts at place 2; set or cleared by where it rests");
/// `PARAMS_GET_U(params, 14, 1)`: no second flag.
pub const NO_SWITCH_FLAG_2: Field = Field::params("noSwitchFlag2", 14, 1, Kind::Named(&[(0, "switchFlag2 read"), (1, "switchFlag2 not read")]), "whether the second flag is read");
/// Bit 15: set in the data's 0xFF10.
pub const BIT_15: Field = Field::params("bit 15", 15, 1, Kind::Unused, "set in the data (0xFF10); read by nothing");
/// `home.rot.z & 1`: the puzzle, an index into `sBlocks`.
pub const BLOCK: Field = Field::rot("block", Source::RotZ, 0, 1, Kind::Named(&[(0, "the large block"), (1, "the Deku Tree's room 3 small block")]), "which puzzle (sBlocks)");

/// Obj_Makeoshihiki's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG_1, NO_SWITCH_FLAG_1, SWITCH_FLAG_2, NO_SWITCH_FLAG_2, BIT_15, BLOCK],
    flags: &[
        FlagUse::new(Flag::Switch("switchFlag1")).reads().sets().clears().note("read unless noSwitchFlag1; set and cleared by where the block rests"),
        FlagUse::new(Flag::Switch("switchFlag2")).reads().sets().clears().note("read unless noSwitchFlag2; set and cleared by where the block rests"),
    ],
    todo: false,
    objects: &[],
};

/// `BlockConfig`: the block's three places, what each does (`unk_24`: bit 0 the chime when the
/// flags change there, bit 1 the block can't be moved any more), its colour, type and yaw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockConfig {
    pub pos_vecs: [Vec3; 3],
    pub unk_24: [u8; 3],
    pub color: u8,
    pub block_type: i16,
    pub rot_y: i16,
}

/// `sBlocks`.
pub const BLOCKS: [BlockConfig; 2] = [
    BlockConfig {
        pos_vecs: [Vec3::new(660.0, 460.0, 660.0), Vec3::new(660.0, 457.0, 540.0), Vec3::new(780.0, 454.0, 540.0)],
        unk_24: [0x00, 0x00, 0x03],
        color: 0xFF,
        block_type: PUSHBLOCK_LARGE_START_ON,
        rot_y: 0x0000,
    },
    BlockConfig {
        pos_vecs: [Vec3::new(-605.0, -820.0, -290.0), Vec3::new(-365.0, -905.0, -290.0), Vec3::new(-365.0, -905.0, -290.0)],
        unk_24: [0x00, 0x03, 0x00],
        color: 0xFF,
        block_type: PUSHBLOCK_SMALL_START_ON,
        rot_y: 0x0000,
    },
];

/// `sFlags`: the two flags at each place (1 set, 0 cleared: `sFlagSwitchFuncs`'
/// `Flags_UnsetSwitch`, `Flags_SetSwitch`).
pub const FLAGS: [[u32; 2]; 3] = [[0, 0], [1, 0], [0, 1]];

pub struct ObjMakeoshihiki {
    pub actor: Actor,
}

impl ObjMakeoshihiki {
    /// `sBlocks[home.rot.z & 1]`.
    pub fn block(&self) -> &'static BlockConfig {
        &BLOCKS[BLOCK.get(self.actor.home_rot.z) as usize]
    }

    /// `PARAMS_GET_U(params, 0, 6)`, `PARAMS_GET_U(params, 8, 6)`.
    pub fn flags(&self) -> (i32, i32) {
        (SWITCH_FLAG_1.get(self.actor.params) as i32, SWITCH_FLAG_2.get(self.actor.params) as i32)
    }

    /// `ObjMakeoshihiki_Init`: the block at its first place whose flag is set (the first flag:
    /// place 1, the second: place 2), else place 0, spawned as its child (`Obj_Oshihiki`, params
    /// the colour, type and no flag: `| 0xFF00`); a place with `unk_24` bit 1 makes it immovable.
    /// No block, and it's gone. `rot.z` zeroed.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let block = &BLOCKS[BLOCK.get(actor.home_rot.z) as usize];
        let type_idx = if NO_SWITCH_FLAG_1.get(actor.params) == 0 && play.flags.get_switch(SWITCH_FLAG_1.get(actor.params) as i32) {
            1
        } else if NO_SWITCH_FLAG_2.get(actor.params) == 0 && play.flags.get_switch(SWITCH_FLAG_2.get(actor.params) as i32) {
            2
        } else {
            0
        };
        let spawn_pos = block.pos_vecs[type_idx];
        let params = (((block.color as u16) << 6) & 0xC0) | (block.block_type as u16 & 0xF) | 0xFF00;
        let child = match play.actor_spawn_as_child(&mut actor, ACTOR_OBJ_OSHIHIKI, spawn_pos, [0, block.rot_y, 0], params as i16) {
            Ok(h) => h,
            Err(_) => {
                log::error!("Error : Push/pull block failed to spawn (../z_obj_makeoshihiki.c 194)");
                actor.kill();
                return Box::new(ObjMakeoshihiki { actor });
            }
        };
        if block.unk_24[type_idx] & 2 != 0
            && let Some(b) = play.actors.downcast_mut::<ObjOshihiki>(child)
        {
            b.cant_move = true;
        }
        actor.world_rot.z = 0;
        actor.shape_rot.z = 0;
        // schema: the whole params printed.
        log::debug!("(../z_obj_makeoshihiki.c)(arg_data {:04x}F)(angleZ {})", actor.params, actor.home_rot.z);
        Box::new(ObjMakeoshihiki { actor })
    }
}

impl ActorImpl for ObjMakeoshihiki {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `Actor_Noop`.
    fn update(&mut self, _play: &mut PlayState) {}

    /// `ObjMakeoshihiki_Draw` (it draws nothing; this is its state): at the first of the three
    /// places the block rests on exactly (within 0.001 squared), the flags set and cleared as
    /// `sFlags` says for it (the second flag too, even without one in the params), with the chime
    /// (`NA_SE_SY_TRE_BOX_APPEAR`) where `unk_24` bit 0 says, when either flag it uses changes;
    /// a place with bit 1 makes the block immovable.
    fn draw_update(&mut self, play: &mut PlayState) {
        let Some(child) = self.actor.child else { return };
        let Some(child_pos) = play.actors.actor(child).map(|a| a.world_pos) else {
            // The C reads its child pointer as it is (a block gone with it dangling).
            log::warn!("Obj_Makeoshihiki: its block is gone");
            return;
        };
        let block = self.block();
        let (flag1, flag2) = self.flags();
        let (no_flag1, no_flag2) = (NO_SWITCH_FLAG_1.get(self.actor.params) != 0, NO_SWITCH_FLAG_2.get(self.actor.params) != 0);
        for i in 0..3 {
            // Math3D_Vec3fDistSq.
            if child_pos.distance_squared(block.pos_vecs[i]) < 0.001 {
                if block.unk_24[i] & 1 != 0 {
                    let sfx_cond1 = if no_flag1 { false } else { (FLAGS[i][0] ^ play.flags.get_switch(flag1) as u32) != 0 };
                    let sfx_cond2 = if no_flag2 { false } else { (FLAGS[i][1] ^ play.flags.get_switch(flag2) as u32) != 0 };
                    if sfx_cond1 || sfx_cond2 {
                        play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
                    }
                }
                for (which, flag) in [(FLAGS[i][0], flag1), (FLAGS[i][1], flag2)] {
                    if which == 1 {
                        play.flags.set_switch(flag);
                    } else {
                        play.flags.unset_switch(flag);
                    }
                }
                if block.unk_24[i] & 2 != 0
                    && let Some(b) = play.actors.downcast_mut::<ObjOshihiki>(child)
                {
                    b.cant_move = true;
                }
                break;
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
