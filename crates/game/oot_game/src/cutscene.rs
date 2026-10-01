//! The cutscene system (`z_demo.c`): `play->csCtx` (`CutsceneContext`, `z64.h`), the scripts
//! it plays, and the triggers that start them.
//!
//! ## The scripts
//!
//! A script is an array of `CutsceneData` words (`z64cutscene_commands.h`): the number of
//! commands and the last frame (`CS_BEGIN_CUTSCENE`), then the commands, each a type word and
//! its entries, up to `CS_END`. The pack holds each script as the ROM's bytes, big-endian as
//! the N64 reads them (`CutsceneScript`, docs/adr/0022-cutscenes.md), and this module reads
//! them the way `Cutscene_ProcessCommands` does: a byte offset stands for each of the C's
//! pointers into the script (`linkAction`, `npcActions`, the camera's points).
//!
//! The scripts come from two places, both read by the importer:
//! - scene files: the 73 the decomp's XMLs name (`gDekuTreeIntroCs` in `ydan_scene`, played on
//!   entering the Deku Tree the first time);
//! - actor overlays' data: 27 arrays in `*_cutscene_data*.c` (`Bg_Treemouth`'s four, the Deku
//!   Tree's talk).

use std::sync::Arc;

use eng_input::pad::{BTN_A, BTN_B, BTN_DLEFT, BTN_DRIGHT, BTN_DUP, BTN_START};
use glam::{IVec3, Vec3};

use crate::camera::{CAM_ID_MAIN, CAM_SET_CS_0, CAM_SET_FREE0, CAM_STAT_ACTIVE, CAM_STAT_WAIT};
use crate::message::{TEXT_STATE_8, TEXT_STATE_9, TEXT_STATE_CHOICE, TEXT_STATE_CLOSING, TEXT_STATE_EVENT, TEXT_STATE_NONE, TEXT_STATE_SONG_DEMO_DONE};
use crate::play::PlayState;
use crate::save::GAMEMODE_NORMAL;
use crate::transition::*;

/// `CutsceneState` (`z64cutscene.h`).
pub const CS_STATE_IDLE: u8 = 0;
pub const CS_STATE_SKIPPABLE_INIT: u8 = 1;
pub const CS_STATE_SKIPPABLE_EXEC: u8 = 2;
pub const CS_STATE_UNSKIPPABLE_INIT: u8 = 3;
pub const CS_STATE_UNSKIPPABLE_EXEC: u8 = 4;

/// `CutsceneCmd` (`z64cutscene.h`).
pub const CS_CMD_CAM_EYE: i32 = 0x0001;
pub const CS_CMD_CAM_AT: i32 = 0x0002;
pub const CS_CMD_MISC: i32 = 0x0003;
pub const CS_CMD_SET_LIGHTING: i32 = 0x0004;
pub const CS_CMD_CAM_EYE_REL_TO_PLAYER: i32 = 0x0005;
pub const CS_CMD_CAM_AT_REL_TO_PLAYER: i32 = 0x0006;
pub const CS_CMD_07: i32 = 0x0007;
pub const CS_CMD_08: i32 = 0x0008;
pub const CS_CMD_09: i32 = 0x0009;
pub const CS_CMD_SET_PLAYER_ACTION: i32 = 0x000A;
pub const CS_CMD_TEXTBOX: i32 = 0x0013;
pub const CS_CMD_SCENE_TRANS_FX: i32 = 0x002D;
pub const CS_CMD_PLAYBGM: i32 = 0x0056;
pub const CS_CMD_STOPBGM: i32 = 0x0057;
pub const CS_CMD_FADEBGM: i32 = 0x007C;
pub const CS_CMD_SETTIME: i32 = 0x008C;
pub const CS_CMD_TERMINATOR: i32 = 0x03E8;
/// `CS_END()`'s first word.
pub const CS_CMD_END: i32 = -1;

/// `CS_CMD_STOP` (`z64cutscene.h`): a camera point list's last point.
pub const CS_CMD_STOP: i8 = -1;

/// The command types `Cutscene_ProcessCommands` gives each actor cue slot, `npcActions[0]` to
/// `[9]` (`CS_CMD_SET_ACTOR_ACTION_1` .. `_10` and the numbers listed with them).
pub const ACTOR_ACTION_SLOTS: [&[i32]; 10] = [
    &[0x0F, 17, 18, 23, 34, 39, 46, 76, 85, 93, 105, 107, 110, 119, 123, 138, 139, 144],
    &[0x0E, 16, 24, 35, 40, 48, 64, 68, 70, 78, 80, 94, 116, 118, 120, 125, 131, 141],
    &[0x19, 36, 41, 50, 67, 69, 72, 74, 81, 106, 117, 121, 126, 132],
    &[0x1D, 37, 42, 51, 53, 63, 65, 66, 75, 82, 108, 127, 133],
    &[0x1E, 38, 43, 47, 54, 79, 83, 128, 135],
    &[0x2C, 55, 77, 84, 90, 129, 136],
    &[0x1F, 52, 57, 58, 88, 115, 130, 137],
    &[0x31, 60, 89, 111, 114, 134, 142],
    &[0x3E],
    &[0x8F],
];

/// The cue slot a command type fills (`npcActions[i]`), if it's an actor cue list.
pub fn actor_action_slot(cmd_type: i32) -> Option<usize> {
    ACTOR_ACTION_SLOTS.iter().position(|s| s.contains(&cmd_type))
}

/// A cutscene script, as the pack holds it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CutsceneScript {
    /// The file it's in (a scene's, `ydan_scene`, or an overlay's, `ovl_Bg_Treemouth`) and its
    /// symbol (`gDekuTreeIntroCs`, `D_808BCE20`).
    pub file: String,
    pub name: String,
    /// The `CutsceneData` words as the ROM has them (big-endian), from `CS_BEGIN_CUTSCENE`
    /// through `CS_END`.
    pub data: Vec<u8>,
}

/// A row of `sEntranceCutsceneTable` (`z_demo.c`, `EntranceCutscene` in `z64cutscene.h`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EntranceCutscene {
    /// `entrance`: the `gEntranceTable` index, and its name.
    pub entrance: u16,
    pub entrance_name: String,
    /// `ageRestriction`: 0 adult only, 1 child only, 2 either.
    pub age_restriction: u8,
    /// `flag`: the `EVENTCHKINF_*` it sets (and waits for to be unset).
    pub flag: u8,
    /// `segAddr`: the script's pack key (`keys::cutscene`), and its symbol.
    pub script: String,
    pub script_name: String,
}

/// What the pack holds about cutscenes besides the scripts (`keys::CUTSCENES`).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CutsceneTables {
    /// `sEntranceCutsceneTable`, in order.
    pub entrance_cutscenes: Vec<EntranceCutscene>,
    /// Every script's pack key, by symbol (the symbols are unique across files).
    pub scripts: Vec<(String, String)>,
}

impl CutsceneTables {
    /// The pack key of the script named `name`.
    pub fn key(&self, name: &str) -> Option<&str> {
        self.scripts.iter().find(|(n, _)| n == name).map(|(_, k)| k.as_str())
    }
}

/// A big-endian reader over a script (the C's `MemCpy` and struct reads).
pub fn be_u16(d: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([d[o], d[o + 1]])
}
pub fn be_i16(d: &[u8], o: usize) -> i16 {
    be_u16(d, o) as i16
}
pub fn be_i32(d: &[u8], o: usize) -> i32 {
    i32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}
pub fn be_f32(d: &[u8], o: usize) -> f32 {
    f32::from_bits(be_i32(d, o) as u32)
}

/// One command of a script, as `Cutscene_ProcessCommands` steps over it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptCommand {
    /// The type word's offset, and the type.
    pub offset: usize,
    pub cmd_type: i32,
    /// Where its entries start and how many there are (for a camera list, its points; for the
    /// terminator and the transition, the one entry), and each entry's size.
    pub entries_offset: usize,
    pub entries: usize,
    pub entry_size: usize,
    /// The offset after the command.
    pub end: usize,
}

/// The size of a camera point list from its header (the `CsCmdBase` at `offset`): the header,
/// then 0x10-byte points up to the one flagged `CS_CMD_STOP`
/// (`Cutscene_Command_CameraEyePoints`' size).
fn camera_list_size(d: &[u8], offset: usize) -> Result<usize, String> {
    let mut size = 8;
    loop {
        let p = offset + size;
        if p + 0x10 > d.len() {
            return Err(format!("camera point list at {offset:#x} runs past the script"));
        }
        size += 0x10;
        if d[p] as i8 == CS_CMD_STOP {
            return Ok(size);
        }
    }
}

/// The commands of a script in order, stepping as `Cutscene_ProcessCommands` does: at most
/// `totalEntries` of them, stopping at `CS_END`'s -1. Also returns the offset just past the
/// script (`CS_END`'s second word) when the walk reached it.
pub fn walk(d: &[u8]) -> Result<(Vec<ScriptCommand>, Option<usize>), String> {
    let word = |o: usize| -> Result<i32, String> { if o + 4 <= d.len() { Ok(be_i32(d, o)) } else { Err(format!("the script ends inside the word at {o:#x}")) } };
    let total = word(0)?;
    let mut at = 8;
    let mut out = Vec::new();
    let end;
    // The walk goes on to CS_END even past totalEntries, so the importer finds the script's
    // end; `ScriptCommand`s past totalEntries are left out, as the C never reads them.
    let mut i = 0;
    loop {
        let cmd_type = word(at)?;
        if cmd_type == CS_CMD_END {
            end = Some(at + 8);
            break;
        }
        let start = at;
        at += 4;
        let (entries_offset, entries, entry_size, next) = match cmd_type {
            CS_CMD_CAM_EYE | CS_CMD_CAM_EYE_REL_TO_PLAYER | CS_CMD_CAM_AT | CS_CMD_CAM_AT_REL_TO_PLAYER => {
                let size = camera_list_size(d, at)?;
                (at + 8, (size - 8) / 0x10, 0x10, at + size)
            }
            // Cutscene_Command_07 and _08: a header and one point.
            CS_CMD_07 | CS_CMD_08 => (at + 8, 1, 0x10, at + 8 + 0x10),
            // A word (always 1), then the one 8-byte entry.
            CS_CMD_TERMINATOR | CS_CMD_SCENE_TRANS_FX => (at + 4, 1, 8, at + 12),
            _ => {
                let n = word(at)?;
                if n < 0 {
                    return Err(format!("command {cmd_type:#x} at {start:#x} has {n} entries"));
                }
                let size = match cmd_type {
                    CS_CMD_09 | CS_CMD_SETTIME | CS_CMD_TEXTBOX => 0xC,
                    _ => 0x30,
                };
                (at + 4, n as usize, size, at + 4 + n as usize * size)
            }
        };
        if next > d.len() {
            return Err(format!("command {cmd_type:#x} at {start:#x} runs past the script"));
        }
        if i < total {
            out.push(ScriptCommand { offset: start, cmd_type, entries_offset, entries, entry_size, end: next });
        }
        i += 1;
        at = next;
        if i > 4096 {
            return Err("no CS_END in 4096 commands".into());
        }
    }
    Ok((out, end))
}

/// `CsCmdActorAction` (`z64cutscene.h`): an actor's cue, or Player's (`linkAction`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CsCmdActorAction {
    /// `action` ("dousa").
    pub action: u16,
    pub start_frame: u16,
    pub end_frame: u16,
    /// `rot` (`urot` as unsigned).
    pub rot: [i16; 3],
    pub start_pos: IVec3,
    pub end_pos: IVec3,
    /// `normal`: the script's `CMD_F` floats, read as the `Vec3i` the struct declares.
    pub normal: IVec3,
}

impl CsCmdActorAction {
    /// The 0x30-byte entry at `o`.
    pub fn read(d: &[u8], o: usize) -> CsCmdActorAction {
        let v3 = |o: usize| IVec3::new(be_i32(d, o), be_i32(d, o + 4), be_i32(d, o + 8));
        CsCmdActorAction {
            action: be_u16(d, o),
            start_frame: be_u16(d, o + 2),
            end_frame: be_u16(d, o + 4),
            rot: [be_i16(d, o + 6), be_i16(d, o + 8), be_i16(d, o + 10)],
            start_pos: v3(o + 0xC),
            end_pos: v3(o + 0x18),
            normal: v3(o + 0x24),
        }
    }
}

/// `CutsceneCameraPoint` (`z64cutscene.h`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CutsceneCameraPoint {
    pub continue_flag: i8,
    pub camera_roll: i8,
    pub next_point_frame: u16,
    /// `viewAngle`, in degrees.
    pub view_angle: f32,
    pub pos: [i16; 3],
}

impl CutsceneCameraPoint {
    pub fn read(d: &[u8], o: usize) -> CutsceneCameraPoint {
        CutsceneCameraPoint {
            continue_flag: d[o] as i8,
            camera_roll: d[o + 1] as i8,
            next_point_frame: be_u16(d, o + 2),
            view_angle: be_f32(d, o + 4),
            pos: [be_i16(d, o + 8), be_i16(d, o + 10), be_i16(d, o + 12)],
        }
    }

    /// The points from `o` up to and including the one flagged `CS_CMD_STOP`, as the camera's
    /// spline reads them through the C's pointer.
    pub fn read_list(d: &[u8], o: usize) -> Vec<CutsceneCameraPoint> {
        let mut out = Vec::new();
        let mut p = o;
        while p + 0x10 <= d.len() {
            let pt = CutsceneCameraPoint::read(d, p);
            out.push(pt);
            p += 0x10;
            if pt.continue_flag == CS_CMD_STOP {
                break;
            }
        }
        out
    }
}

/// A pointer into a script (the C's `u8*` or `CutsceneCameraPoint*` into `csCtx.segment`): the
/// script, which stays alive while pointed at, and a byte offset.
#[derive(Debug, Clone, PartialEq)]
pub struct CsPtr {
    pub script: Arc<CutsceneScript>,
    pub offset: usize,
}

impl CsPtr {
    /// The camera points from here to the one flagged `CS_CMD_STOP`.
    pub fn points(&self) -> Vec<CutsceneCameraPoint> {
        CutsceneCameraPoint::read_list(&self.script.data, self.offset)
    }
    pub fn point(&self) -> CutsceneCameraPoint {
        CutsceneCameraPoint::read(&self.script.data, self.offset)
    }
}

/// `CutsceneContext` (`z64.h`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CutsceneContext {
    /// `segment`: the script (`Cutscene_SetSegment`, or an actor's `play->csCtx.segment = ...`).
    pub segment: Option<Arc<CutsceneScript>>,
    /// `state` (`CS_STATE_*`).
    pub state: u8,
    /// `unk_0C`: 0 to 1 as a script starts (`func_8006472C`), back to 0 as it ends.
    pub unk_0c: f32,
    /// `frames`: the script's frame counter.
    pub frames: u16,
    pub unk_12: u16,
    /// `subCamId`: the cutscene's camera.
    pub sub_cam_id: i16,
    /// `unk_18`: the start frame of the eye list last applied; `unk_1A`, `unk_1B`: an at list, an
    /// eye list seen.
    pub unk_18: u16,
    pub unk_1a: u8,
    pub unk_1b: u8,
    /// `subCamLookAtPoints`, `subCamEyePoints`.
    pub sub_cam_look_at_points: Option<CsPtr>,
    pub sub_cam_eye_points: Option<CsPtr>,
    /// `linkAction`, `npcActions[10]` ("npcdemopnt"): the cues in effect (copies of the
    /// script's entries).
    pub link_action: Option<CsCmdActorAction>,
    pub npc_actions: [Option<CsCmdActorAction>; 10],
}

/// `z_demo.c`'s file-scope variables. They live in the code segment, so they carry over from one
/// play state to the next (`PlayState::reinit`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DemoStatics {
    /// `D_8011E1C0`, `D_8011E1C4`: the text (and the ocarina action) a script's text command
    /// last started.
    pub d_8011e1c0: u16,
    pub d_8011e1c4: u16,
    /// `D_8015FCC0`, `D_8015FCC2`, `D_8015FCC4`: the start frames of the at list, the `CS_CMD_07`
    /// list and the `CS_CMD_08` list last applied.
    pub d_8015fcc0: u16,
    pub d_8015fcc2: u16,
    pub d_8015fcc4: u16,
    /// `sReturnToCamId`: the camera active when the script began.
    pub return_to_cam_id: i16,
    /// `D_8015FCC8` (later decomps' `gUseCutsceneCam`): whether scripts drive a camera.
    /// `Environment_Init` sets it (`z_kankyo.c:419`); the debug D-Left / D-Up replays clear and
    /// set it.
    pub d_8015fcc8: u8,
    /// `sQuakeIndex`, `sTitleCsState`, `D_8015FCCC`, `D_8015FCE4`.
    pub quake_index: i16,
    pub title_cs_state: u8,
    pub d_8015fccc: u16,
    pub d_8015fce4: u8,
}

/// A script's commands, one line each, for `ootx cutscene` and the tests' messages.
pub fn describe(d: &[u8]) -> Result<Vec<String>, String> {
    let (cmds, _) = walk(d)?;
    let mut out = vec![format!("CS_BEGIN_CUTSCENE({}, {})", be_i32(d, 0), be_i32(d, 4))];
    for c in cmds {
        let e = |i: usize| c.entries_offset + i * c.entry_size;
        match c.cmd_type {
            CS_CMD_CAM_EYE | CS_CMD_CAM_EYE_REL_TO_PLAYER | CS_CMD_CAM_AT | CS_CMD_CAM_AT_REL_TO_PLAYER | CS_CMD_07 | CS_CMD_08 => {
                let what = match c.cmd_type {
                    CS_CMD_CAM_EYE => "CAM_EYE",
                    CS_CMD_CAM_EYE_REL_TO_PLAYER => "CAM_EYE_REL_TO_PLAYER",
                    CS_CMD_CAM_AT => "CAM_AT",
                    CS_CMD_CAM_AT_REL_TO_PLAYER => "CAM_AT_REL_TO_PLAYER",
                    CS_CMD_07 => "CMD_07",
                    _ => "CMD_08",
                };
                out.push(format!("  {what} frames {}..{}, {} points", be_u16(d, c.offset + 6), be_u16(d, c.offset + 8), c.entries));
                for i in 0..c.entries {
                    let p = CutsceneCameraPoint::read(d, e(i));
                    out.push(format!("    {:>2} roll {:>3} frame {:>4} fov {:.2} pos {:?}", p.continue_flag, p.camera_roll, p.next_point_frame, p.view_angle, p.pos));
                }
            }
            CS_CMD_TERMINATOR | CS_CMD_SCENE_TRANS_FX => {
                let what = if c.cmd_type == CS_CMD_TERMINATOR { "TERMINATOR" } else { "SCENE_TRANS_FX" };
                out.push(format!("  {what}({}, {}, {})", be_u16(d, e(0)), be_u16(d, e(0) + 2), be_u16(d, e(0) + 4)));
            }
            CS_CMD_TEXTBOX => {
                out.push(format!("  TEXT_LIST({})", c.entries));
                for i in 0..c.entries {
                    let o = e(i);
                    out.push(format!(
                        "    text {:#06x} frames {}..{} type {:#x} branches {:#06x} {:#06x}",
                        be_u16(d, o),
                        be_u16(d, o + 2),
                        be_u16(d, o + 4),
                        be_u16(d, o + 6),
                        be_u16(d, o + 8),
                        be_u16(d, o + 10)
                    ));
                }
            }
            t => {
                let what = match t {
                    CS_CMD_MISC => "MISC".to_string(),
                    CS_CMD_SET_LIGHTING => "LIGHTING".to_string(),
                    CS_CMD_09 => "CMD_09".to_string(),
                    CS_CMD_SET_PLAYER_ACTION => "PLAYER_ACTION".to_string(),
                    CS_CMD_PLAYBGM => "PLAY_BGM".to_string(),
                    CS_CMD_STOPBGM => "STOP_BGM".to_string(),
                    CS_CMD_FADEBGM => "FADE_BGM".to_string(),
                    CS_CMD_SETTIME => "TIME".to_string(),
                    t => match actor_action_slot(t) {
                        Some(s) => format!("NPC_ACTION {t} (npcActions[{s}])"),
                        None => format!("UNUSED {t:#x}"),
                    },
                };
                out.push(format!("  {what} ({} entries)", c.entries));
                for i in 0..c.entries {
                    let o = e(i);
                    if c.entry_size == 0x30 {
                        let a = CsCmdActorAction::read(d, o);
                        out.push(format!("    {:#06x} frames {}..{} rot {:?} {:?} -> {:?}", a.action, a.start_frame, a.end_frame, a.rot, a.start_pos.to_array(), a.end_pos.to_array()));
                    } else {
                        out.push(format!("    {:#06x} frames {}..{} {:#010x}", be_u16(d, o), be_u16(d, o + 2), be_u16(d, o + 4), be_i32(d, o + 6.min(c.entry_size - 4))));
                    }
                }
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// z_demo.c
// ---------------------------------------------------------------------------------------------

/// `EVENTCHKINF_*` (`z64save.h`) the commands and the triggers set or read.
const EVENTCHKINF_18: u16 = 0x18;
const EVENTCHKINF_45: u16 = 0x45;
const EVENTCHKINF_48: u16 = 0x48;
const EVENTCHKINF_49: u16 = 0x49;
const EVENTCHKINF_4A: u16 = 0x4A;
const EVENTCHKINF_4F: u16 = 0x4F;
const EVENTCHKINF_54: u16 = 0x54;
const EVENTCHKINF_65: u16 = 0x65;
const EVENTCHKINF_67: u16 = 0x67;
const EVENTCHKINF_69: u16 = 0x69;
const EVENTCHKINF_AA: u16 = 0xAA;
const EVENTCHKINF_AC: u16 = 0xAC;
const EVENTCHKINF_AD: u16 = 0xAD;
const EVENTCHKINF_BB: u16 = 0xBB;
const EVENTCHKINF_BC: u16 = 0xBC;
const EVENTCHKINF_BD: u16 = 0xBD;
const EVENTCHKINF_BE: u16 = 0xBE;
const EVENTCHKINF_BF: u16 = 0xBF;
const EVENTCHKINF_C1: u16 = 0xC1;
const EVENTCHKINF_C4: u16 = 0xC4;
const EVENTCHKINF_C7: u16 = 0xC7;
const EVENTCHKINF_C8: u16 = 0xC8;

/// `z64item.h`.
const ITEM_SONG_REQUIEM: u8 = 0x5D;
const ITEM_SONG_NOCTURNE: u8 = 0x5E;
const ITEM_MEDALLION_FIRE: u8 = 0x67;
const QUEST_MEDALLION_SPIRIT: u32 = 0x03;
const QUEST_MEDALLION_SHADOW: u32 = 0x04;
const QUEST_GORON_RUBY: u32 = 0x13;
const QUEST_ZORA_SAPPHIRE: u32 = 0x14;

/// `TRANS_TYPE_CIRCLE(TCA_NORMAL, TCC_WHITE, TCS_SLOW)` (`z64.h`: `(1 << 5) | ((color & 3) << 3)
/// | ((appearance & 3) << 1) | (speed & 1)`, with `TCC_WHITE` 1 and `TCS_SLOW` 1).
const TRANS_TYPE_CIRCLE_NORMAL_WHITE_SLOW: u8 = 0x29;

/// The terminator destinations that only start a transition (`Cutscene_Command_Terminator`'s
/// arms that set `nextEntranceIndex`, maybe `cutsceneIndex`, `transitionType` and maybe
/// `nextTransitionType`, and nothing else), transcribed from the C in its order.
#[rustfmt::skip]
const TERMINATOR_DESTINATIONS: &[(u16, &str, Option<u16>, u8, Option<u8>)] = &[
    (1, "ENTR_HIRAL_DEMO_0", Some(0xFFF1), TRANS_TYPE_FADE_BLACK, None),
    (2, "ENTR_HIRAL_DEMO_0", Some(0xFFF0), TRANS_TYPE_FILL_WHITE, None),
    (3, "ENTR_SPOT09_0", Some(0xFFF1), TRANS_TYPE_FILL_WHITE, None),
    (4, "ENTR_SPOT16_0", Some(0xFFF0), TRANS_TYPE_FILL_WHITE, None),
    (5, "ENTR_SPOT04_0", Some(0xFFF0), TRANS_TYPE_FILL_WHITE, None),
    (6, "ENTR_HIRAL_DEMO_0", Some(0xFFF2), TRANS_TYPE_FILL_WHITE, None),
    (7, "ENTR_SPOT04_0", Some(0xFFF2), TRANS_TYPE_INSTANT, None),
    (9, "ENTR_SPOT09_0", Some(0xFFF0), TRANS_TYPE_FILL_BROWN, None),
    (10, "ENTR_LINK_HOME_0", Some(0xFFF0), TRANS_TYPE_FADE_BLACK, None),
    (11, "ENTR_SPOT04_0", Some(0xFFF3), TRANS_TYPE_FADE_WHITE, None),
    (12, "ENTR_SPOT16_5", None, TRANS_TYPE_FADE_BLACK, None),
    (13, "ENTR_SPOT08_0", None, TRANS_TYPE_FADE_BLACK, Some(TRANS_TYPE_FADE_BLACK)),
    (14, "ENTR_SPOT04_11", None, TRANS_TYPE_FADE_BLACK, None),
    (15, "ENTR_TOKINOMA_0", Some(0xFFF4), TRANS_TYPE_FADE_WHITE, None),
    (16, "ENTR_TOKINOMA_0", Some(0xFFF5), TRANS_TYPE_FADE_WHITE, None),
    (17, "ENTR_TOKINOMA_0", Some(0xFFF6), TRANS_TYPE_FADE_WHITE, None),
    (19, "ENTR_SPOT16_0", Some(0x8000), TRANS_TYPE_FADE_BLACK_FAST, None),
    (21, "ENTR_SPOT06_0", Some(0xFFF0), TRANS_TYPE_FADE_WHITE, None),
    (23, "ENTR_HIRAL_DEMO_0", Some(0xFFF8), TRANS_TYPE_FADE_WHITE, None),
    (24, "ENTR_BDAN_0", None, TRANS_TYPE_FADE_BLACK, None),
    (26, "ENTR_TOKINOMA_0", Some(0xFFF4), TRANS_TYPE_FADE_WHITE, None),
    (27, "ENTR_TOKINOMA_0", Some(0xFFF5), TRANS_TYPE_FADE_WHITE, None),
    (28, "ENTR_TOKINOMA_0", Some(0xFFF6), TRANS_TYPE_FADE_WHITE, None),
    (33, "ENTR_SPOT00_0", None, TRANS_TYPE_FADE_WHITE, None),
    (34, "ENTR_HIRAL_DEMO_0", Some(0xFFF3), TRANS_TYPE_FADE_WHITE, None),
    (35, "ENTR_SPOT00_0", Some(0xFFF0), TRANS_TYPE_FADE_BLACK_FAST, None),
    (38, "ENTR_HIRAL_DEMO_0", Some(0xFFF4), TRANS_TYPE_FADE_BLACK_FAST, None),
    (39, "ENTR_TOKINOMA_0", Some(0xFFF9), TRANS_TYPE_FADE_BLACK_FAST, None),
    (41, "ENTR_SPOT06_5", None, TRANS_TYPE_FADE_BLACK, None),
    (42, "ENTR_SPOT01_0", Some(0xFFF2), TRANS_TYPE_FADE_BLACK_FAST, None),
    (43, "ENTR_HAKASITARELAY_2", None, TRANS_TYPE_FADE_BLACK_FAST, None),
    (44, "ENTR_TOKINOMA_3", None, TRANS_TYPE_FADE_WHITE_INSTANT, None),
    (48, "ENTR_SPOT11_4", None, TRANS_TYPE_SANDSTORM_END, Some(TRANS_TYPE_SANDSTORM_END)),
    (49, "ENTR_TOKINOMA_5", None, TRANS_TYPE_FADE_BLACK_FAST, None),
    (50, "ENTR_SPOT01_13", None, TRANS_TYPE_FADE_WHITE_INSTANT, None),
    (51, "ENTR_SPOT00_0", Some(0xFFF8), TRANS_TYPE_CIRCLE_NORMAL_WHITE_SLOW, None),
    (52, "ENTR_TOKINOMA_0", Some(0xFFF7), TRANS_TYPE_INSTANT, None),
    (53, "ENTR_SPOT00_16", None, TRANS_TYPE_FADE_WHITE, None),
    (55, "ENTR_SPOT12_0", Some(0xFFF1), TRANS_TYPE_FADE_BLACK, None),
    (56, "ENTR_SPOT01_0", Some(0xFFF4), TRANS_TYPE_FADE_BLACK, None),
    (57, "ENTR_SPOT16_0", Some(0xFFF3), TRANS_TYPE_FADE_BLACK, None),
    (58, "ENTR_SPOT18_0", Some(0xFFF1), TRANS_TYPE_FADE_BLACK, None),
    (59, "ENTR_SPOT06_0", Some(0xFFF1), TRANS_TYPE_FADE_BLACK, None),
    (60, "ENTR_SPOT08_0", Some(0xFFF2), TRANS_TYPE_FADE_BLACK, None),
    (61, "ENTR_SPOT07_0", Some(0xFFF0), TRANS_TYPE_FADE_BLACK, None),
    (63, "ENTR_SPOT04_0", Some(0xFFF7), TRANS_TYPE_FADE_BLACK, None),
    (64, "ENTR_SPOT00_0", Some(0xFFF5), TRANS_TYPE_FADE_BLACK, None),
    (66, "ENTR_SPOT01_14", None, TRANS_TYPE_FADE_BLACK, None),
    (67, "ENTR_SPOT00_9", None, TRANS_TYPE_FADE_BLACK, None),
    (68, "ENTR_HIRAL_DEMO_0", Some(0xFFF5), TRANS_TYPE_FADE_BLACK, None),
    (69, "ENTR_SPOT04_12", None, TRANS_TYPE_FADE_BLACK, None),
    (70, "ENTR_SPOT16_0", Some(0xFFF4), TRANS_TYPE_FADE_BLACK, Some(TRANS_TYPE_FADE_BLACK)),
    (72, "ENTR_NAKANIWA_0", Some(0xFFF0), TRANS_TYPE_FADE_BLACK, Some(TRANS_TYPE_FADE_BLACK)),
    (74, "ENTR_SPOT20_0", Some(0xFFF3), TRANS_TYPE_FADE_WHITE, Some(TRANS_TYPE_FADE_WHITE)),
    (78, "ENTR_SPOT20_0", Some(0xFFF7), TRANS_TYPE_FADE_BLACK, None),
    // Cases 79 to 92 fall through to 93.
    (79, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (80, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (81, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (82, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (83, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (84, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (85, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (86, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (87, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (88, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (89, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (90, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (91, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (92, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (93, "ENTR_SPOT20_0", None, TRANS_TYPE_FADE_BLACK, None),
    (94, "ENTR_SPOT20_1", None, TRANS_TYPE_FADE_WHITE, None),
    (98, "ENTR_SPOT17_5", None, TRANS_TYPE_FADE_WHITE, Some(TRANS_TYPE_FADE_WHITE)),
    (99, "ENTR_SPOT05_3", None, TRANS_TYPE_FADE_BLACK, Some(TRANS_TYPE_FADE_BLACK)),
    (100, "ENTR_SPOT04_0", Some(0xFFF8), TRANS_TYPE_FADE_WHITE, Some(TRANS_TYPE_FADE_WHITE)),
    (101, "ENTR_SPOT11_6", None, TRANS_TYPE_SANDSTORM_END, None),
    (102, "ENTR_TOKINOMA_6", None, TRANS_TYPE_FADE_BLACK, None),
    (103, "ENTR_SPOT00_0", Some(0xFFF3), TRANS_TYPE_FADE_BLACK, None),
    (105, "ENTR_SPOT02_0", Some(0xFFF1), TRANS_TYPE_FADE_BLACK, None),
    (106, "ENTR_HAKAANA_OUKE_1", None, TRANS_TYPE_FADE_BLACK, None),
    (107, "ENTR_GANONTIKA_2", None, TRANS_TYPE_FADE_BLACK, None),
    (108, "ENTR_GANONTIKA_3", None, TRANS_TYPE_FADE_BLACK, None),
    (109, "ENTR_GANONTIKA_4", None, TRANS_TYPE_FADE_BLACK, None),
    (110, "ENTR_GANONTIKA_5", None, TRANS_TYPE_FADE_BLACK, None),
    (111, "ENTR_GANONTIKA_6", None, TRANS_TYPE_FADE_BLACK, None),
    (112, "ENTR_GANONTIKA_7", None, TRANS_TYPE_FADE_BLACK, None),
    (114, "ENTR_SPOT00_3", None, TRANS_TYPE_FADE_BLACK, None),
    (115, "ENTR_SPOT00_17", None, TRANS_TYPE_FADE_BLACK, Some(TRANS_TYPE_FADE_BLACK)),
];

impl PlayState {
    /// `IS_CUTSCENE_LAYER` (`macros.h`: `gSaveContext.sceneLayer > 3`).
    pub fn is_cutscene_layer(&self) -> bool {
        self.save.scene_layer > 3
    }

    /// The script named `name` from the pack (an actor's `play->csCtx.segment = D_...`).
    pub fn cutscene_script(&self, name: &str) -> Option<Arc<CutsceneScript>> {
        match self.assets.as_ref().map(|a| a.cutscene(name)) {
            Some(Ok(s)) => Some(s),
            Some(Err(e)) => {
                log::error!("{e:#}");
                None
            }
            None => None,
        }
    }

    /// The `gEntranceTable` index of the entrance `name`.
    fn entrance_by_name(&self, name: &str) -> Option<u16> {
        self.assets.as_ref()?.scenes.entrances.iter().position(|e| e.name == name).map(|i| i as u16)
    }

    /// `func_8002DF38` (`z_actor.c`): Player's `csMode`, the actor it's about (`unk_448`), and
    /// `doorBgCamIndex` 0.
    pub fn func_8002df38(&mut self, actor: Option<crate::actor_ctx::ActorHandle>, cs_mode: u8) -> bool {
        self.set_player_cs_mode(actor, cs_mode, 0)
    }

    /// `func_8002DF54` (`z_actor.c`): `func_8002DF38`, then `doorBgCamIndex` 1.
    pub fn func_8002df54(&mut self, actor: Option<crate::actor_ctx::ActorHandle>, cs_mode: u8) -> bool {
        self.set_player_cs_mode(actor, cs_mode, 1)
    }

    fn set_player_cs_mode(&mut self, actor: Option<crate::actor_ctx::ActorHandle>, cs_mode: u8, door_bg_cam_index: i16) -> bool {
        match self.player.and_then(|h| self.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
            Some(p) => {
                p.set_cs_mode(cs_mode, actor, door_bg_cam_index);
                true
            }
            None => false,
        }
    }

    /// `Flags_SetEnv`, `Flags_UnsetEnv`, `Flags_GetEnv` (`code_8006C3A0.c`): `play->envFlags`.
    pub fn flags_set_env(&mut self, flag: i16) {
        self.env_flags[(flag / 16) as usize] |= 1 << (flag % 16);
    }
    pub fn flags_unset_env(&mut self, flag: i16) {
        self.env_flags[(flag / 16) as usize] &= (1u16 << (flag % 16)) ^ 0xFFFF;
    }
    pub fn flags_get_env(&self, flag: i16) -> bool {
        self.env_flags[(flag / 16) as usize] & (1 << (flag % 16)) != 0
    }

    /// `func_8006450C` (later decomps' `Cutscene_InitContext`).
    pub fn func_8006450c(&mut self) {
        self.cs_ctx.state = CS_STATE_IDLE;
        self.cs_ctx.unk_0c = 0.0;
    }

    /// `func_80064520`.
    pub fn func_80064520(&mut self) {
        self.cs_ctx.state = CS_STATE_SKIPPABLE_INIT;
        self.cs_ctx.link_action = None;
    }

    /// `func_80064534`: end the script (the debug camera's toggle, `Camera_Update`).
    pub fn func_80064534(&mut self) {
        if self.cs_ctx.state != CS_STATE_UNSKIPPABLE_EXEC {
            self.cs_ctx.state = CS_STATE_UNSKIPPABLE_INIT;
        }
    }

    /// `func_80064558` (later decomps' `Cutscene_UpdateManual`): outside a script
    /// (`cutsceneIndex < 0xFFF0`), `sCsStateHandlers1`: an actor's own cutscene only fades the
    /// letterbox and the interface in and out.
    pub fn func_80064558(&mut self) {
        if self.save.cutscene_index < 0xFFF0 {
            // sCsStateHandlers1: func_80064720, func_80064760, func_80064720, func_80068D84,
            // func_80064720.
            match self.cs_ctx.state {
                CS_STATE_SKIPPABLE_INIT => self.func_80064760(),
                CS_STATE_UNSKIPPABLE_INIT => self.func_80068d84(),
                _ => {}
            }
        }
    }

    /// `func_800645A0` (later decomps' `Cutscene_UpdateScripted`): a trigger starts the script
    /// in `csCtx.segment` (`cutsceneIndex` 0xFFFD), then `sCsStateHandlers2` runs it.
    pub fn func_800645a0(&mut self) {
        let press = self.input.press;
        // The debug ROM's replays in a cutscene layer: D-Left without the script's camera, D-Up
        // with it.
        if press.held(BTN_DLEFT) && self.cs_ctx.state == CS_STATE_IDLE && self.is_cutscene_layer() {
            self.demo.d_8015fcc8 = 0;
            self.save.cutscene_index = 0xFFFD;
            self.save.cutscene_trigger = 1;
        }
        // (gDbgCamEnabled: the debug camera isn't ported.)
        if press.held(BTN_DUP) && self.cs_ctx.state == CS_STATE_IDLE && self.is_cutscene_layer() {
            self.demo.d_8015fcc8 = 1;
            self.save.cutscene_index = 0xFFFD;
            self.save.cutscene_trigger = 1;
        }
        if self.save.cutscene_trigger != 0 && self.transition.trigger == TRANS_TRIGGER_START {
            self.save.cutscene_trigger = 0;
        }
        if self.save.cutscene_trigger != 0 && self.cs_ctx.state == CS_STATE_IDLE {
            log::debug!("cutscene start request (demo start request announcement)");
            self.save.cutscene_index = 0xFFFD;
            self.save.cutscene_trigger = 1;
        }
        if self.save.cutscene_index >= 0xFFF0 {
            self.func_80068ecc();
            // sCsStateHandlers2: func_80064720, func_800647C0, func_80068C3C, func_80068DC0,
            // func_80068C3C.
            match self.cs_ctx.state {
                CS_STATE_SKIPPABLE_INIT => self.func_800647c0(),
                CS_STATE_SKIPPABLE_EXEC | CS_STATE_UNSKIPPABLE_EXEC => self.func_80068c3c(),
                CS_STATE_UNSKIPPABLE_INIT => self.func_80068dc0(),
                _ => {}
            }
        }
    }

    /// `func_8006472C`: `unk_0C` towards `target` by 0.1; true once there.
    fn func_8006472c(&mut self, target: f32) -> bool {
        eng_math::step_to_f(&mut self.cs_ctx.unk_0c, target, 0.1)
    }

    /// `func_80064760`.
    fn func_80064760(&mut self) {
        crate::interface::change_alpha(&mut self.save, 1);
        self.letterbox.set_size_target(32);
        if self.func_8006472c(1.0) {
            self.audio.set_cutscene_flag(1);
            self.cs_ctx.state += 1;
        }
    }

    /// `func_800647C0`: the script runs as it starts.
    fn func_800647c0(&mut self) {
        self.func_80068c3c();
        crate::interface::change_alpha(&mut self.save, 1);
        self.letterbox.set_size_target(32);
        if self.func_8006472c(1.0) {
            self.audio.set_cutscene_flag(1);
            self.cs_ctx.state += 1;
        }
    }

    /// `func_80064824`: command 3, the misc actions. Those that need what isn't ported (the
    /// weather, the lights and fog, the skybox, quakes, title cards, the sandstorm, the
    /// Sun's Song, sound) are logged on their first frame.
    fn func_80064824(&mut self, d: &[u8], o: usize) {
        let (base, start, end) = (be_u16(d, o), be_u16(d, o + 2), be_u16(d, o + 4));
        let frames = self.cs_ctx.frames;
        if frames < start || (frames >= end && end != start) {
            return;
        }
        let temp = crate::env::lerp_weight(end.wrapping_sub(1), start, frames);
        let first = frames == start;
        let not_ported = |what: &str| {
            if first {
                log::warn!("cutscene misc {base} ({what}) at frame {frames}: not ported");
            }
        };
        match base {
            1 => not_ported("rain"),
            2 => not_ported("lightning"),
            3 => {
                if first {
                    self.flags_set_env(0);
                    if self.save.entrance_index == self.entrance_by_name("ENTR_TOKINOMA_0").unwrap_or(u16::MAX) {
                        self.flags_set_env(2);
                    }
                }
            }
            6 => not_ported("envCtx.adjFogFar"),
            7 => not_ported("the skybox and light config change"),
            8 => {
                if let Some(s) = self.scene.as_mut()
                    && s.draw.room_unk_74[0] < 0x80
                {
                    s.draw.room_unk_74[0] += 4;
                }
            }
            9 => not_ported("snow"),
            10 => self.flags_set_env(1),
            11 => {
                if let Some(s) = self.scene.as_mut() {
                    if s.draw.room_unk_74[0] < 0x672 {
                        s.draw.room_unk_74[0] += 0x14;
                    }
                    // func_80078884(NA_SE_EV_DEKU_DEATH) at 0x30F: no audio.
                    if frames == 0x2CD {
                        s.draw.room_unk_74[0] = 0;
                    }
                }
            }
            12 => {
                if first && self.cs_ctx.state != CS_STATE_UNSKIPPABLE_EXEC {
                    self.cs_ctx.state = CS_STATE_UNSKIPPABLE_INIT;
                }
            }
            13 => {
                if let Some(s) = self.scene.as_mut()
                    && s.draw.room_unk_74[1] < 0xFF
                {
                    s.draw.room_unk_74[1] += 5;
                }
            }
            14 => {
                if first {
                    self.set_viewpoint(crate::play::VIEWPOINT_LOCKED);
                }
            }
            15 => not_ported("TitleCard_InitPlaceName"),
            16 => not_ported("Quake_Add"),
            17 => not_ported("Quake_RemoveFromIdx"),
            18 => not_ported("the storm's end"),
            19 => self.save.set_event_chk_inf(EVENTCHKINF_65),
            20 => self.save.set_event_chk_inf(EVENTCHKINF_67),
            21 => self.save.set_event_chk_inf(EVENTCHKINF_69),
            22 | 23 => {
                let _ = temp;
                not_ported("D_801614B0, the screen tint")
            }
            24 => not_ported("roomCtx.curRoom.segment = NULL"),
            25 => {
                self.save.day_time = self.save.day_time.wrapping_add(30);
                if self.save.day_time >= crate::env::clock_time(19, 0) as u16 {
                    self.save.day_time = crate::env::clock_time(19, 0) as u16 - 1;
                }
            }
            26 => not_ported("envCtx.lightSettingOverride by the time"),
            27 => not_ported("envCtx.adjAmbientColor"),
            28 | 29 => not_ported("play->unk_11DE9 (actors frozen)"),
            30 => self.flags_set_env(3),
            31 => self.flags_set_env(4),
            32 => not_ported("the sandstorm"),
            33 => not_ported("the Sun's Song"),
            34 => not_ported("time running backwards (gTimeSpeed)"),
            35 => not_ported("the scarecrow's song after the credits"),
            _ => {}
        }
    }

    /// Command 4, `Cutscene_Command_SetLighting`: `envCtx.lightSettingOverride` isn't ported.
    fn cutscene_command_set_lighting(&mut self, d: &[u8], o: usize) {
        if self.cs_ctx.frames == be_u16(d, o + 2) {
            log::warn!("cutscene lighting {} at frame {}: envCtx.lightSettingOverride not ported", d[o + 1], self.cs_ctx.frames);
        }
    }

    /// Command 0x8C, `func_80065134`: the time of day (`skyboxTime` isn't kept apart here).
    fn func_80065134(&mut self, d: &[u8], o: usize) {
        if self.cs_ctx.frames == be_u16(d, o + 2) {
            let (hour, minute) = (d[o + 6], d[o + 7]);
            let temp1 = ((hour as f32 * 60.0) / (360.0 / 0x4000 as f32)) as i32 as i16;
            let temp2 = ((minute as i32 + 1) as f32 / (360.0 / 0x4000 as f32)) as i32 as i16;
            self.save.day_time = temp1.wrapping_add(temp2) as u16;
        }
    }

    /// Command 0x3E8, `Cutscene_Command_Terminator`: on its frame (or A, B or Start after frame
    /// 20 outside normal play, or Start after frame 20 anywhere, off the Deku Tree's debug
    /// file), the script ends and its destination's transition starts.
    fn cutscene_command_terminator(&mut self, d: &[u8], o: usize) {
        let (base, start) = (be_u16(d, o), be_u16(d, o + 2));
        let frames = self.cs_ctx.frames;
        let press = self.input.press;
        let mut temp = false;
        let spot00 = self.scene_id == 0x51;
        if self.save.game_mode != GAMEMODE_NORMAL
            && self.save.game_mode != 3
            && !spot00
            && frames > 20
            && (press.held(BTN_A) || press.held(BTN_B) || press.held(BTN_START))
            && self.save.file_num != 0xFEDC
            && self.transition.trigger == TRANS_TRIGGER_OFF
        {
            // NA_SE_SY_PIECE_OF_HEART: no audio.
            temp = true;
        }
        if !(frames == start || temp || (frames > 20 && press.held(BTN_START) && self.save.file_num != 0xFEDC)) {
            return;
        }
        self.cs_ctx.state = CS_STATE_UNSKIPPABLE_EXEC;
        self.audio.set_cutscene_flag(0);
        self.save.cutscene_transition_control = 1;
        log::debug!("cutscene terminator: destination {base}");
        if self.save.game_mode != GAMEMODE_NORMAL && frames != start {
            self.save.unk_13e7 = 1;
        }
        self.save.cutscene_index = 0;
        let go = |play: &mut PlayState, entrance: &str, cs_index: Option<u16>, ty: u8, next_ty: Option<u8>| {
            match play.entrance_by_name(entrance) {
                Some(e) => play.transition.next_entrance_index = e,
                None => log::error!("cutscene terminator {base}: no entrance {entrance}"),
            }
            if let Some(c) = cs_index {
                play.save.cutscene_index = c;
            }
            play.transition.trigger = TRANS_TRIGGER_START;
            play.transition.ty = ty;
            if let Some(n) = next_ty {
                play.save.next_transition_type = n;
            }
        };
        if let Some(&(_, e, c, t, n)) = TERMINATOR_DESTINATIONS.iter().find(|r| r.0 == base) {
            go(self, e, c, t, n);
            return;
        }
        let age_not_ported = |age: &str| log::warn!("cutscene terminator {base}: play->linkAgeOnLoad = {age} not ported");
        match base {
            8 => {
                // gSaveContext.fw.set = 0 (Farore's Wind isn't ported).
                self.save.respawn[crate::save::RESPAWN_MODE_TOP].data = 0;
                if !self.save.get_event_chk_inf(EVENTCHKINF_45) {
                    self.save.set_event_chk_inf(EVENTCHKINF_45);
                    go(self, "ENTR_HIRAL_DEMO_0", Some(0xFFF3), TRANS_TYPE_INSTANT, None);
                } else {
                    if !self.is_cutscene_layer() {
                        age_not_ported(if !self.save.adult { "LINK_AGE_ADULT" } else { "LINK_AGE_CHILD" });
                    }
                    go(self, "ENTR_TOKINOMA_2", None, TRANS_TYPE_FADE_WHITE, Some(TRANS_TYPE_FADE_WHITE));
                }
            }
            18 => {
                self.save.set_event_chk_inf(EVENTCHKINF_4F);
                go(self, "ENTR_TOKINOMA_4", None, TRANS_TYPE_FADE_BLACK, Some(TRANS_TYPE_FADE_BLACK));
            }
            22 => {
                crate::item::item_give(&mut self.save, Some(&mut self.audio), ITEM_SONG_REQUIEM);
                go(self, "ENTR_SPOT11_0", Some(0xFFF0), TRANS_TYPE_FADE_WHITE, None);
            }
            25 => {
                age_not_ported("LINK_AGE_ADULT");
                go(self, "ENTR_KENJYANOMA_0", Some(0xFFF0), TRANS_TYPE_FADE_WHITE, None);
            }
            29 | 30 | 31 => {
                // gSaveContext.chamberCutsceneNum = 0, 1, 2: the Chamber of Sages isn't ported.
                go(self, "ENTR_KENJYANOMA_0", None, TRANS_TYPE_FADE_WHITE, None);
                if base == 30 {
                    crate::item::item_give(&mut self.save, Some(&mut self.audio), ITEM_MEDALLION_FIRE);
                }
            }
            32 => {
                age_not_ported("LINK_AGE_CHILD");
                go(self, "ENTR_SPOT00_0", Some(0xFFF2), TRANS_TYPE_INSTANT, None);
            }
            40 => {
                age_not_ported("LINK_AGE_ADULT");
                go(self, "ENTR_TOKINOMA_0", Some(0xFFFA), TRANS_TYPE_FADE_BLACK_FAST, None);
            }
            46 => {
                self.save.set_event_chk_inf(EVENTCHKINF_4F);
                go(self, "ENTR_TOKINOMA_4", None, TRANS_TYPE_FADE_BLACK_FAST, None);
            }
            47 => {
                crate::item::item_give(&mut self.save, Some(&mut self.audio), ITEM_SONG_NOCTURNE);
                self.save.set_event_chk_inf(EVENTCHKINF_54);
                go(self, "ENTR_SPOT01_0", Some(0xFFF1), TRANS_TYPE_FADE_BLACK_FAST, None);
            }
            54 | 117 => {
                // GAMEMODE_END_CREDITS, Audio_SetSfxBanksMute(0x6F).
                self.save.game_mode = 3;
                if base == 54 {
                    age_not_ported("LINK_AGE_CHILD");
                    go(self, "ENTR_SPOT09_0", Some(0xFFF2), TRANS_TYPE_FADE_BLACK, None);
                } else {
                    age_not_ported("LINK_AGE_ADULT");
                    go(self, "ENTR_SPOT00_0", Some(0xFFF7), TRANS_TYPE_FADE_WHITE, None);
                }
            }
            62 => {
                age_not_ported("LINK_AGE_ADULT");
                go(self, "ENTR_SPOT04_0", Some(0xFFF6), TRANS_TYPE_FADE_BLACK, None);
            }
            65 | 73 | 75 | 76 | 77 => {
                let (age, c) = match base {
                    65 => ("LINK_AGE_CHILD", 0xFFF2),
                    73 => ("LINK_AGE_CHILD", 0xFFF2),
                    75 => ("LINK_AGE_CHILD", 0xFFF4),
                    76 => ("LINK_AGE_ADULT", 0xFFF5),
                    _ => ("LINK_AGE_CHILD", 0xFFF6),
                };
                age_not_ported(age);
                go(self, "ENTR_SPOT20_0", Some(c), TRANS_TYPE_FADE_BLACK, None);
            }
            71 => {
                use crate::item::{EQUIP_TYPE_BOOTS, EQUIP_TYPE_TUNIC, EQUIP_VALUE_BOOTS_KOKIRI, EQUIP_VALUE_TUNIC_KOKIRI};
                self.save.equips.equipment |= EQUIP_VALUE_TUNIC_KOKIRI << (EQUIP_TYPE_TUNIC * 4);
                self.player_set_equipment_data();
                self.save.equips.equipment |= EQUIP_VALUE_BOOTS_KOKIRI << (EQUIP_TYPE_BOOTS * 4);
                self.player_set_equipment_data();
                age_not_ported("LINK_AGE_CHILD");
                go(self, "ENTR_TOKINOMA_0", Some(0xFFF1), TRANS_TYPE_FADE_BLACK, None);
            }
            95 => {
                if self.save.get_event_chk_inf(EVENTCHKINF_48) && self.save.get_event_chk_inf(EVENTCHKINF_49) && self.save.get_event_chk_inf(EVENTCHKINF_4A) {
                    go(self, "ENTR_TOKINOMA_0", Some(0xFFF3), TRANS_TYPE_FADE_BLACK, None);
                } else {
                    match self.save.scene_layer {
                        8 => go(self, "ENTR_SPOT05_0", None, TRANS_TYPE_FADE_BLACK, None),
                        9 => go(self, "ENTR_SPOT17_0", None, TRANS_TYPE_FADE_BLACK, None),
                        10 => go(self, "ENTR_SPOT06_0", Some(0xFFF0), TRANS_TYPE_FADE_WHITE, None),
                        _ => {}
                    }
                }
            }
            96 => {
                if self.save.check_quest_item(QUEST_MEDALLION_SHADOW) {
                    go(self, "ENTR_KENJYANOMA_0", Some(0xFFF1), TRANS_TYPE_FADE_WHITE_FAST, None);
                } else {
                    self.save.set_event_chk_inf(EVENTCHKINF_C8);
                    go(self, "ENTR_SPOT11_8", None, TRANS_TYPE_FADE_WHITE, Some(TRANS_TYPE_FADE_WHITE));
                }
            }
            97 => {
                if self.save.check_quest_item(QUEST_MEDALLION_SPIRIT) {
                    go(self, "ENTR_KENJYANOMA_0", Some(0xFFF1), TRANS_TYPE_FADE_WHITE_FAST, None);
                } else {
                    go(self, "ENTR_SPOT02_8", None, TRANS_TYPE_FADE_WHITE, Some(TRANS_TYPE_FADE_WHITE));
                }
            }
            104 => match self.demo.title_cs_state {
                0 => {
                    go(self, "ENTR_JYASINBOSS_0", Some(0xFFF2), TRANS_TYPE_FADE_BLACK, None);
                    self.demo.title_cs_state += 1;
                }
                1 => {
                    go(self, "ENTR_SPOT17_0", Some(0xFFF1), TRANS_TYPE_FADE_BLACK, None);
                    self.demo.title_cs_state += 1;
                }
                2 => {
                    go(self, "ENTR_HIRAL_DEMO_0", Some(0xFFF6), TRANS_TYPE_FADE_BLACK, None);
                    self.demo.title_cs_state = 0;
                }
                _ => {}
            },
            113 => {
                let all = [EVENTCHKINF_BB, EVENTCHKINF_BC, EVENTCHKINF_BD, EVENTCHKINF_BE, EVENTCHKINF_BF, EVENTCHKINF_AD].iter().all(|&f| self.save.get_event_chk_inf(f));
                if all {
                    self.cs_ctx.segment = self.cutscene_script("gTowerBarrierCs");
                    self.cs_ctx.frames = 0;
                    self.save.cutscene_trigger = 1;
                }
                self.save.cutscene_index = 0xFFFF;
                self.cs_ctx.state = CS_STATE_UNSKIPPABLE_INIT;
            }
            116 => {
                if self.save.get_event_chk_inf(EVENTCHKINF_C8) {
                    go(self, "ENTR_SPOT02_8", None, TRANS_TYPE_FADE_WHITE, None);
                } else {
                    go(self, "ENTR_SPOT11_8", None, TRANS_TYPE_FADE_WHITE, None);
                }
                self.save.next_transition_type = TRANS_TYPE_FADE_WHITE;
            }
            118 => {
                match self.entrance_by_name("ENTR_GANON_DEMO_0") {
                    Some(e) => self.save.respawn[crate::save::RESPAWN_MODE_DOWN].entrance_index = e,
                    None => log::error!("cutscene terminator 118: no entrance ENTR_GANON_DEMO_0"),
                }
                self.trigger_void_out();
                self.save.respawn_flag = -2;
                self.save.next_transition_type = TRANS_TYPE_FADE_BLACK;
            }
            119 => {
                self.save.day_time = crate::env::clock_time(12, 0) as u16;
                go(self, "ENTR_NAKANIWA_1", None, TRANS_TYPE_FADE_WHITE, None);
            }
            _ => {}
        }
    }

    /// `Player_SetEquipmentData(play, player)`.
    fn player_set_equipment_data(&mut self) {
        let (data, save) = (self.data.clone(), self.save.clone());
        if let Some(p) = self.player.and_then(|h| self.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
            p.set_equipment_data(&data, &save);
        }
    }

    /// Command 0x2D, `Cutscene_Command_TransitionFX`: a fill over the screen
    /// (`envCtx.fillScreen`, `screenFillColor`), fading in or out over the command's frames.
    fn cutscene_command_transition_fx(&mut self, d: &[u8], o: usize) {
        let (base, start, end) = (be_u16(d, o), be_u16(d, o + 2), be_u16(d, o + 4));
        let frames = self.cs_ctx.frames;
        if !(frames >= start && frames <= end) {
            return;
        }
        let temp = crate::env::lerp_weight(end, start, frames);
        let fill = |rgb: [u8; 3], a: f32| Some([rgb[0], rgb[1], rgb[2], a as u8]);
        let (rise, fall) = (255.0 * temp, (1.0 - temp) * 255.0);
        let prev = self.transition.screen_fill;
        // (The white-out sounds aren't played.)
        self.transition.screen_fill = match base {
            1 => fill([160, 160, 160], rise),
            5 => fill([160, 160, 160], fall),
            2 => fill([0, 0, 255], rise),
            6 => fill([0, 0, 255], fall),
            3 => fill([255, 0, 0], fall),
            7 => fill([255, 0, 0], rise),
            4 => fill([0, 255, 0], fall),
            8 => fill([0, 255, 0], rise),
            10 => fill([0, 0, 0], fall),
            11 => fill([0, 0, 0], rise),
            13 => fill([0, 0, 0], 255.0 - ((1.0 - temp) * 155.0)),
            // envCtx.fillScreen = true, with the colour as it was.
            _ => Some(prev.unwrap_or([0, 0, 0, 0])),
        };
        match base {
            9 => self.save.cutscene_transition_control = 1,
            12 => self.save.cutscene_transition_control = (255.0 - (155.0 * temp)) as u8,
            _ => {}
        }
    }

    /// Commands 1 and 5, `Cutscene_Command_CameraEyePoints`: once an at list has been seen
    /// (`unk_1A`), the list whose frames these are puts the cutscene's camera on its splines.
    fn cutscene_command_camera_eye_points(&mut self, script: &Arc<CutsceneScript>, o: usize, relative_to_link: i16) {
        let d = &script.data;
        let (start, end) = (be_u16(d, o + 2), be_u16(d, o + 4));
        let cs = &mut self.cs_ctx;
        if start < cs.frames && cs.frames < end && (cs.unk_18 < start || cs.unk_18 >= 0xF000) {
            cs.unk_1b = 1;
            cs.sub_cam_eye_points = Some(CsPtr { script: script.clone(), offset: o + 8 });
            if cs.unk_1a != 0 {
                cs.unk_18 = start;
                if self.demo.d_8015fcc8 != 0 {
                    self.apply_cutscene_camera(relative_to_link);
                }
            }
        }
    }

    /// Commands 2 and 6, `Cutscene_Command_CameraLookAtPoints`.
    fn cutscene_command_camera_look_at_points(&mut self, script: &Arc<CutsceneScript>, o: usize, relative_to_link: i16) {
        let d = &script.data;
        let (start, end) = (be_u16(d, o + 2), be_u16(d, o + 4));
        let cs = &mut self.cs_ctx;
        if start < cs.frames && cs.frames < end && (self.demo.d_8015fcc0 < start || self.demo.d_8015fcc0 >= 0xF000) {
            cs.unk_1a = 1;
            cs.sub_cam_look_at_points = Some(CsPtr { script: script.clone(), offset: o + 8 });
            if cs.unk_1b != 0 {
                self.demo.d_8015fcc0 = start;
                if self.demo.d_8015fcc8 != 0 {
                    self.apply_cutscene_camera(relative_to_link);
                }
            }
        }
    }

    /// The camera part both point commands share: `CAM_SET_CS_0` on the sub camera, which takes
    /// over from the camera that was active (`CAM_STAT_WAIT`), starting its splines again
    /// (`Camera_ResetAnim`, `Camera_SetCSParams`).
    fn apply_cutscene_camera(&mut self, relative_to_link: i16) {
        let sub = self.cs_ctx.sub_cam_id;
        self.camera_change_setting(sub, CAM_SET_CS_0);
        self.change_camera_status(self.demo.return_to_cam_id, CAM_STAT_WAIT);
        self.change_camera_status(sub, CAM_STAT_ACTIVE);
        let at = self.cs_ctx.sub_cam_look_at_points.as_ref().map(|p| p.points()).unwrap_or_default();
        let eye = self.cs_ctx.sub_cam_eye_points.as_ref().map(|p| p.points()).unwrap_or_default();
        let Some(pv) = self.player_view() else { return };
        if let Some(c) = self.camera_mut(sub) {
            c.reset_anim();
            c.set_cs_params(at, eye, &pv, relative_to_link);
        }
    }

    /// Commands 7 and 8, `Cutscene_Command_07` and `_08`: one eye point, or one at point; with
    /// both seen, the sub camera (`CAM_SET_FREE0`, no player) is put there at once.
    fn cutscene_command_07_08(&mut self, script: &Arc<CutsceneScript>, o: usize, eye: bool) {
        let d = &script.data;
        let (start, end) = (be_u16(d, o + 2), be_u16(d, o + 4));
        let frames = self.cs_ctx.frames;
        let applied = if eye { self.demo.d_8015fcc2 } else { self.demo.d_8015fcc4 };
        if !(start < frames && frames < end && (applied < start || applied >= 0xF000)) {
            return;
        }
        let ptr = Some(CsPtr { script: script.clone(), offset: o + 8 });
        let other_ready = if eye {
            self.cs_ctx.unk_1b = 1;
            self.cs_ctx.sub_cam_eye_points = ptr;
            self.cs_ctx.unk_1a != 0
        } else {
            self.cs_ctx.unk_1a = 1;
            self.cs_ctx.sub_cam_look_at_points = ptr;
            self.cs_ctx.unk_1b != 0
        };
        if !other_ready {
            return;
        }
        if eye {
            self.demo.d_8015fcc2 = start;
        } else {
            self.demo.d_8015fcc4 = start;
        }
        if self.demo.d_8015fcc8 == 0 {
            return;
        }
        let sub = self.cs_ctx.sub_cam_id;
        let (Some(at), Some(eye_pt)) = (self.cs_ctx.sub_cam_look_at_points.as_ref().map(|p| p.point()), self.cs_ctx.sub_cam_eye_points.as_ref().map(|p| p.point())) else { return };
        if let Some(c) = self.camera_mut(sub) {
            c.has_player = false;
        }
        self.change_camera_status(CAM_ID_MAIN, CAM_STAT_WAIT);
        self.change_camera_status(sub, CAM_STAT_ACTIVE);
        self.camera_change_setting(sub, CAM_SET_FREE0);
        if eye
            && let Some(c) = self.camera_mut(sub)
        {
            c.set_roll_deg(at.camera_roll as f32 * 1.40625);
        }
        let v = |p: [i16; 3]| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
        self.camera_set_at_eye(sub, v(at.pos), v(eye_pt.pos));
        self.camera_set_fov(sub, eye_pt.view_angle);
    }

    /// Command 0x13, `Cutscene_Command_Textbox`: a text opens on the first frame after its
    /// start, and at its end frame the script waits while the box is up, answering a choice
    /// with the branch's text, or waits for the event's end (ocarina actions: `func_8010BD58`,
    /// not ported).
    fn cutscene_command_textbox(&mut self, d: &[u8], o: usize) {
        let (base, start, end, ty, text_id1, text_id2) = (be_u16(d, o), be_u16(d, o + 2), be_u16(d, o + 4), be_u16(d, o + 6), be_u16(d, o + 8), be_u16(d, o + 10));
        let frames = self.cs_ctx.frames;
        if !(start < frames && frames <= end) {
            return;
        }
        if ty != 2 {
            if self.demo.d_8011e1c0 != base {
                self.demo.d_8011e1c0 = base;
                if (ty == 3 && self.save.check_quest_item(QUEST_ZORA_SAPPHIRE)) || (ty == 4 && self.save.check_quest_item(QUEST_GORON_RUBY)) {
                    self.start_textbox(text_id1, None);
                } else {
                    self.start_textbox(base, None);
                }
                return;
            }
        } else if self.demo.d_8011e1c4 != base {
            self.demo.d_8011e1c4 = base;
            log::warn!("cutscene text: ocarina action {base:#x} (func_8010BD58) not ported");
            return;
        }
        if frames >= end {
            let original = frames;
            let st = self.message_state();
            if st != TEXT_STATE_CLOSING && st != TEXT_STATE_NONE && st != TEXT_STATE_SONG_DEMO_DONE && st != TEXT_STATE_8 {
                self.cs_ctx.frames = self.cs_ctx.frames.wrapping_sub(1);
                if st == TEXT_STATE_CHOICE && self.message_should_advance() {
                    let branch = if self.msg_ctx.choice_index == 0 { text_id1 } else { text_id2 };
                    if branch != 0xFFFF {
                        self.continue_textbox(branch);
                    } else {
                        self.cs_ctx.frames = self.cs_ctx.frames.wrapping_add(1);
                    }
                }
                if st == TEXT_STATE_9 {
                    if text_id1 != 0xFFFF {
                        self.continue_textbox(text_id1);
                    } else {
                        self.cs_ctx.frames = self.cs_ctx.frames.wrapping_add(1);
                    }
                }
                if st == TEXT_STATE_EVENT && self.message_should_advance() {
                    log::warn!("cutscene text {base:#x}: the event's func_8010BD58 not ported");
                }
            }
            if self.cs_ctx.frames == original {
                crate::interface::change_alpha(&mut self.save, 1);
                self.demo.d_8011e1c0 = 0;
                self.demo.d_8011e1c4 = 0;
            }
        }
    }

    /// `Cutscene_ProcessCommands`: one frame of the script, command by command. Past the
    /// script's last frame, or on D-Right (the debug skip), the script ends
    /// (`CS_STATE_UNSKIPPABLE_INIT`).
    pub fn cutscene_process_commands(&mut self, script: Arc<CutsceneScript>) {
        let d = &script.data[..];
        let total = be_i32(d, 0);
        let end_frame = be_i32(d, 4);
        if end_frame < self.cs_ctx.frames as i32 && self.cs_ctx.state != CS_STATE_UNSKIPPABLE_EXEC {
            self.cs_ctx.state = CS_STATE_UNSKIPPABLE_INIT;
            return;
        }
        if self.input.press.held(BTN_DRIGHT) {
            self.cs_ctx.state = CS_STATE_UNSKIPPABLE_INIT;
            return;
        }
        let mut p = 8;
        for _ in 0..total {
            let cmd_type = be_i32(d, p);
            p += 4;
            if cmd_type == CS_CMD_END {
                return;
            }
            let entries = |p: usize| be_i32(d, p).max(0) as usize;
            match cmd_type {
                CS_CMD_MISC | CS_CMD_SET_LIGHTING | CS_CMD_PLAYBGM | CS_CMD_STOPBGM | CS_CMD_FADEBGM => {
                    let n = entries(p);
                    p += 4;
                    for _ in 0..n {
                        match cmd_type {
                            CS_CMD_MISC => self.func_80064824(d, p),
                            CS_CMD_SET_LIGHTING => self.cutscene_command_set_lighting(d, p),
                            // Cutscene_Command_PlayBGM, _StopBGM, _FadeBGM: the cutscenes' audio waits for their
                            // polish pass (BACKLOG #10).
                            _ => {}
                        }
                        p += 0x30;
                    }
                }
                // Cutscene_Command_09 (Rumble_Request: no rumble), func_80065134.
                CS_CMD_09 | CS_CMD_SETTIME => {
                    let n = entries(p);
                    p += 4;
                    for _ in 0..n {
                        if cmd_type == CS_CMD_SETTIME {
                            self.func_80065134(d, p);
                        }
                        p += 0xC;
                    }
                }
                CS_CMD_SET_PLAYER_ACTION => {
                    let n = entries(p);
                    p += 4;
                    for _ in 0..n {
                        let (start, end) = (be_u16(d, p + 2), be_u16(d, p + 4));
                        if start < self.cs_ctx.frames && self.cs_ctx.frames <= end {
                            self.cs_ctx.link_action = Some(CsCmdActorAction::read(d, p));
                        }
                        p += 0x30;
                    }
                }
                CS_CMD_CAM_EYE | CS_CMD_CAM_EYE_REL_TO_PLAYER => {
                    self.cutscene_command_camera_eye_points(&script, p, (cmd_type == CS_CMD_CAM_EYE_REL_TO_PLAYER) as i16);
                    p += camera_list_size(d, p).unwrap_or(8);
                }
                CS_CMD_CAM_AT | CS_CMD_CAM_AT_REL_TO_PLAYER => {
                    self.cutscene_command_camera_look_at_points(&script, p, (cmd_type == CS_CMD_CAM_AT_REL_TO_PLAYER) as i16);
                    p += camera_list_size(d, p).unwrap_or(8);
                }
                CS_CMD_07 | CS_CMD_08 => {
                    self.cutscene_command_07_08(&script, p, cmd_type == CS_CMD_07);
                    p += 8 + 0x10;
                }
                CS_CMD_TERMINATOR => {
                    p += 4;
                    self.cutscene_command_terminator(d, p);
                    p += 8;
                }
                CS_CMD_TEXTBOX => {
                    let n = entries(p);
                    p += 4;
                    for _ in 0..n {
                        if be_u16(d, p) != 0xFFFF {
                            self.cutscene_command_textbox(d, p);
                        }
                        p += 0xC;
                    }
                }
                CS_CMD_SCENE_TRANS_FX => {
                    p += 4;
                    self.cutscene_command_transition_fx(d, p);
                    p += 8;
                }
                t => {
                    let n = entries(p);
                    p += 4;
                    if let Some(slot) = actor_action_slot(t) {
                        for _ in 0..n {
                            let (start, end) = (be_u16(d, p + 2), be_u16(d, p + 4));
                            if start < self.cs_ctx.frames && self.cs_ctx.frames <= end {
                                self.cs_ctx.npc_actions[slot] = Some(CsCmdActorAction::read(d, p));
                            }
                            p += 0x30;
                        }
                    } else {
                        p += n * 0x30;
                    }
                }
            }
        }
    }

    /// `func_80068C3C` (later decomps' `CutsceneHandler_RunScript`): the next frame of the
    /// script. (`Cutscene_DrawDebugInfo` with `BREG(0)`, and `dREG(95)`'s test script, are the
    /// debug ROM's.)
    fn func_80068c3c(&mut self) {
        if self.save.cutscene_index >= 0xFFF0 {
            self.cs_ctx.frames = self.cs_ctx.frames.wrapping_add(1);
            if let Some(s) = self.cs_ctx.segment.clone() {
                self.cutscene_process_commands(s);
            }
        }
    }

    /// `func_80068D84`: an actor's cutscene ends as `unk_0C` falls to 0.
    fn func_80068d84(&mut self) {
        if self.func_8006472c(0.0) {
            self.audio.set_cutscene_flag(0);
            self.cs_ctx.state = CS_STATE_IDLE;
        }
    }

    /// `func_80068DC0` (later decomps' `CutsceneHandler_StopScript`): once `unk_0C` is back to 0,
    /// the cues go, the camera that was active before comes back and the cutscene's is cleared.
    fn func_80068dc0(&mut self) {
        if !self.func_8006472c(0.0) {
            return;
        }
        self.cs_ctx.link_action = None;
        self.cs_ctx.npc_actions = [None; 10];
        log::debug!("cutscene over (right here, huh)");
        self.save.cutscene_index = 0;
        self.save.game_mode = GAMEMODE_NORMAL;
        if self.demo.d_8015fcc8 != 0 {
            let (ret, sub) = (self.demo.return_to_cam_id, self.cs_ctx.sub_cam_id);
            // The Hyrule Field Epona jumps' entrances keep the cutscene's view.
            if matches!(self.save.entrance_index, 0x028A | 0x028E | 0x0292 | 0x0476) {
                self.copy_camera(ret, sub);
            }
            self.change_camera_status(ret, CAM_STAT_ACTIVE);
            self.clear_camera(sub);
            if let Some(c) = self.camera_mut(ret) {
                c.func_8005b1a4();
            }
        }
        self.audio.set_cutscene_flag(0);
        self.cs_ctx.state = CS_STATE_IDLE;
    }

    /// `func_80068ECC` (later decomps' `Cutscene_SetupScripted`): a script starts, with the
    /// cutscene camera if `D_8015FCC8`; an entrance's (`cutsceneTrigger` 2) or an actor's (1)
    /// fades the letterbox in, anything else puts it up at once.
    fn func_80068ecc(&mut self) {
        if self.save.cutscene_trigger != 0 && self.cs_ctx.state == CS_STATE_IDLE && !self.player_in_cs_mode() {
            self.save.cutscene_index = 0xFFFD;
        }
        if self.save.cutscene_index >= 0xFFF0 && self.cs_ctx.state == CS_STATE_IDLE {
            self.flags_unset_env(0);
            self.demo.d_8011e1c0 = 0;
            self.demo.d_8011e1c4 = 0;
            self.cs_ctx.unk_12 = 0;
            self.cs_ctx.link_action = None;
            self.cs_ctx.npc_actions = [None; 10];
            self.cs_ctx.state += 1;
            if self.cs_ctx.state == CS_STATE_SKIPPABLE_INIT {
                self.audio.set_cutscene_flag(1);
                self.cs_ctx.frames = 0xFFFF;
                self.cs_ctx.unk_18 = 0xFFFF;
                self.demo.d_8015fcc0 = 0xFFFF;
                self.demo.d_8015fcc2 = 0xFFFF;
                self.demo.d_8015fcc4 = 0xFFFF;
                self.cs_ctx.unk_1a = 0;
                self.cs_ctx.unk_1b = 0;
                self.demo.return_to_cam_id = self.active_cam_id;
                if self.demo.d_8015fcc8 != 0 {
                    self.cs_ctx.sub_cam_id = self.create_sub_camera();
                }
                if self.save.cutscene_trigger == 0 {
                    crate::interface::change_alpha(&mut self.save, 1);
                    self.letterbox.set_size_target(32);
                    self.letterbox.set_size(32);
                    self.cs_ctx.state += 1;
                }
                self.func_80068c3c();
            }
            self.save.cutscene_trigger = 0;
        }
    }

    /// `func_80069048`.
    pub fn func_80069048(&mut self) {
        self.demo.d_8015fccc = 0;
        self.demo.d_8015fce4 = 0;
    }

    /// `func_8006907C`.
    pub fn func_8006907c(&mut self) {
        if self.demo.d_8015fccc != 0 {
            self.demo.d_8015fccc = 0;
        }
    }

    /// `Cutscene_HandleEntranceTriggers` (`Play_Init`, after the scene loads): the first time in
    /// by an entrance of `sEntranceCutsceneTable`, for the right age and not on a respawn, its
    /// flag is set and its script starts (`cutsceneTrigger` 2). The Epona jumps' flag
    /// (`EVENTCHKINF_18`) doesn't stop them replaying.
    pub fn cutscene_handle_entrance_triggers(&mut self) {
        let Some(assets) = self.assets.clone() else { return };
        for e in &assets.cutscenes.entrance_cutscenes {
            let required_age = if e.age_restriction == 2 { self.save.adult as u8 ^ 1 } else { e.age_restriction };
            // linkAge: LINK_AGE_ADULT 0, LINK_AGE_CHILD 1.
            let link_age = if self.save.adult { 0 } else { 1 };
            if self.save.entrance_index == e.entrance
                && (!self.save.get_event_chk_inf(e.flag as u16) || e.flag as u16 == EVENTCHKINF_18)
                && self.save.cutscene_index < 0xFFF0
                && link_age == required_age
                && self.save.respawn_flag <= 0
            {
                self.save.set_event_chk_inf(e.flag as u16);
                self.cs_ctx.segment = match assets.cutscene(&e.script_name) {
                    Ok(s) => Some(s),
                    Err(err) => {
                        log::error!("{err:#}");
                        None
                    }
                };
                self.save.cutscene_trigger = 2;
                self.save.show_title_card = false;
                break;
            }
        }
    }

    /// `Cutscene_HandleConditionalTriggers` for this play state's save.
    pub fn cutscene_handle_conditional_triggers(&mut self) {
        if let Some(assets) = self.assets.clone() {
            handle_conditional_triggers(&assets, &mut self.save);
        }
    }

    /// `Cutscene_SetSegment`.
    pub fn cutscene_set_segment(&mut self, script: Option<Arc<CutsceneScript>>) {
        self.cs_ctx.segment = script;
    }

    /// `Play_InCsMode`: a script running, or Player in a cutscene's hold.
    pub fn play_in_cs_mode(&self) -> bool {
        self.cs_ctx.state != CS_STATE_IDLE || self.player_in_cs_mode()
    }
}

/// `Cutscene_HandleConditionalTriggers` (`Play_Init`, before the scene layer is picked): the
/// entrances that play a cutscene layer the first time (the desert's first visit, the Kakariko
/// return with the three medallions, the Fairy Ocarina, the Temple of Time's light arrows,
/// Ganon's tower's collapse).
pub fn handle_conditional_triggers(assets: &crate::play_scene::GameAssets, save: &mut crate::save::SaveContext) {
    if save.game_mode != GAMEMODE_NORMAL || save.respawn_flag > 0 || save.cutscene_index >= 0xFFF0 {
        return;
    }
    let entr = |n: &str| assets.scenes.entrances.iter().position(|e| e.name == n).map(|i| i as u16);
    let is = |save: &crate::save::SaveContext, n: &str| entr(n) == Some(save.entrance_index);
    let scene_of = |save: &crate::save::SaveContext| assets.scenes.entrances.get(save.entrance_index as usize).map(|e| e.scene);
    let scene_id = |n: &str| assets.scenes.scenes.iter().position(|s| s.enum_name == n).map(|i| i as u16);
    if is(save, "ENTR_SPOT11_1") && !save.get_event_chk_inf(EVENTCHKINF_AC) {
        save.set_event_chk_inf(EVENTCHKINF_AC);
        save.entrance_index = entr("ENTR_SPOT11_0").unwrap_or(save.entrance_index);
        save.cutscene_index = 0xFFF0;
    } else if is(save, "ENTR_SPOT01_0") && save.adult && save.get_event_chk_inf(EVENTCHKINF_48) && save.get_event_chk_inf(EVENTCHKINF_49) && save.get_event_chk_inf(EVENTCHKINF_4A) && !save.get_event_chk_inf(EVENTCHKINF_AA) {
        save.set_event_chk_inf(EVENTCHKINF_AA);
        save.cutscene_index = 0xFFF0;
    } else if is(save, "ENTR_SPOT10_9") && !save.get_event_chk_inf(EVENTCHKINF_C1) {
        save.set_event_chk_inf(EVENTCHKINF_C1);
        crate::item::item_give(save, None, crate::item::ITEM_OCARINA_FAIRY);
        save.entrance_index = entr("ENTR_SPOT10_0").unwrap_or(save.entrance_index);
        save.cutscene_index = 0xFFF0;
    } else if save.check_quest_item(QUEST_MEDALLION_SPIRIT) && save.check_quest_item(QUEST_MEDALLION_SHADOW) && save.adult && !save.get_event_chk_inf(EVENTCHKINF_C4) && scene_of(save).is_some() && scene_of(save) == scene_id("SCENE_TOKINOMA") {
        save.set_event_chk_inf(EVENTCHKINF_C4);
        save.entrance_index = entr("ENTR_TOKINOMA_0").unwrap_or(save.entrance_index);
        save.cutscene_index = 0xFFF8;
    } else if !save.get_event_chk_inf(EVENTCHKINF_C7) && scene_of(save).is_some() && scene_of(save) == scene_id("SCENE_GANON_DEMO") {
        save.set_event_chk_inf(EVENTCHKINF_C7);
        save.entrance_index = entr("ENTR_GANON_DEMO_0").unwrap_or(save.entrance_index);
        save.cutscene_index = 0xFFF0;
    }
}
