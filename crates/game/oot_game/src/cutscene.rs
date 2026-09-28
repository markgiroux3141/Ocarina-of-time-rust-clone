//! `play->csCtx` (`CutsceneContext`, `z64.h`): the fields actors read while a cutscene runs.
//! Nothing runs cutscenes yet (the cutscene system, `z_demo.c`, is GAME-03 milestone 4), so the
//! state stays `CS_STATE_IDLE` and no actor cues are set. The actors that wait for a cutscene
//! are ported against it as written, and a test can drive it by hand.

use glam::IVec3;

/// `CutsceneState` (`z64cutscene.h`).
pub const CS_STATE_IDLE: u8 = 0;
pub const CS_STATE_SKIPPABLE_INIT: u8 = 1;
pub const CS_STATE_SKIPPABLE_EXEC: u8 = 2;
pub const CS_STATE_UNSKIPPABLE_INIT: u8 = 3;
pub const CS_STATE_UNSKIPPABLE_EXEC: u8 = 4;

/// `CsCmdActorAction` (`z64cutscene.h`): an actor's cue.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CsCmdActorAction {
    /// `action` ("dousa").
    pub action: u16,
    pub start_frame: u16,
    pub end_frame: u16,
    pub rot: [i16; 3],
    pub start_pos: IVec3,
    pub end_pos: IVec3,
    pub normal: IVec3,
}

/// `CutsceneContext`, without the script pointer and the cameras.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CutsceneContext {
    /// `state` (`CS_STATE_*`).
    pub state: u8,
    /// `frames`: the script's frame counter.
    pub frames: u16,
    /// `unk_18`, `unk_1A`, `unk_1B`.
    pub unk_18: u16,
    pub unk_1a: u8,
    pub unk_1b: u8,
    /// `linkAction`, `npcActions[10]` ("npcdemopnt").
    pub link_action: Option<CsCmdActorAction>,
    pub npc_actions: [Option<CsCmdActorAction>; 10],
}
