//! Player movement, ported from `ovl_player_actor/z_player.c` (the decomp commit in use still
//! calls most of these `func_808xxxxx`; each port names its original).
//!
//! Scope: standing, turning in place, walking/running with the walk/run blend, rolling on A
//! (including bonking into walls), running off ledges (auto-jump or fall), air control,
//! landing (including fall-damage stagger and landing rolls), and the head/torso look and
//! lean rotations. Everything that needs items, targeting, water, cutscenes, other actors,
//! ledge hanging or climbing is left out. The interrupt checks for those are stubbed to
//! "not taken", and the list is in the spike 03 findings.
//!
//! Naming: fields keep the decomp's `unk_XXX` names where the decomp has no name yet, so the
//! port can be diffed against the C. File-scope statics (`D_808535D4` ...) live in
//! `PlayerStatics`.

#![allow(non_snake_case)] // fields keep the decomp's unk_XXX names

use eng_collision::bgcheck::{BGCHECK_Y_MIN, CollisionContext, PolyId, udist_plane_to_pos};
use eng_input::pad::{BTN_A, BTN_Z, Input, stick_to_mag_angle};
use eng_math::*;
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTOR_PLAYER, ACTORCAT_NPC, ACTORCAT_PLAYER, ActorContext, ActorHandle, ActorImpl, PLAYER_BODYPART_WAIST, PlayerIface};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::cutscene::CsCmdActorAction;
use oot_game::play_scene::{PlayIo, SCENE_GANON_FINAL, SCENE_HAKADAN, SCENE_SPOT04};
use oot_game::save::{RESPAWN_MODE_DOWN, RESPAWN_MODE_RETURN};
use oot_game::scene::EntranceInfo;
use oot_game::transition::{TRANS_TRIGGER_OFF, TRANS_TRIGGER_START, TRANS_TYPE_FADE_BLACK, TRANS_TYPE_FADE_BLACK_FAST, TRANS_TYPE_FADE_WHITE};
use std::cell::RefCell;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::data::{AgeProperties, AnimId, GameData, Regs};
use oot_game::player_lib::Blinker;
use oot_game::skelanime::{ANIMMODE_LOOP, ANIMMODE_ONCE, SkelAnime};
use oot_game::surface::{SurfaceType, WALL_FLAG_0, WALL_FLAG_1, WALL_FLAG_2, WALL_FLAG_3, WALL_FLAG_4, WALL_FLAG_5, WALL_FLAG_6};
use oot_game::target::{ACTOR_FLAG_27, TargetView};
use oot_game::collision_check::{self as cc, ColliderCylinder, ColliderCylinderInit, ColliderInfoInit, ColliderInit, ColliderMut, ColliderQuad, ColliderQuadInit, ColliderTouch, ColliderBumpInit};
use eng_collision::math3d::Cylinder16;

// stateFlags1
pub const STATE1_0: u32 = 1 << 0; // going through an exit
pub const STATE1_2: u32 = 1 << 2;
pub const STATE1_3: u32 = 1 << 3;
pub const STATE1_8: u32 = 1 << 8; // item change pending
pub const STATE1_10: u32 = 1 << 10; // getting an item
pub const STATE1_24: u32 = 1 << 24;
pub const STATE1_4: u32 = 1 << 4; // locked on (func_8008E9C4)
pub const STATE1_7: u32 = 1 << 7; // dead
pub const STATE1_15: u32 = 1 << 15; // Z pressed this lock-on
pub const STATE1_23: u32 = 1 << 23; // riding
pub const STATE1_25: u32 = 1 << 25; // boomerang in flight
pub const STATE1_6: u32 = 1 << 6;
pub const STATE1_11: u32 = 1 << 11; // holding an actor
pub const STATE1_12: u32 = 1 << 12;
pub const STATE1_13: u32 = 1 << 13;
pub const STATE1_14: u32 = 1 << 14;
pub const STATE1_16: u32 = 1 << 16;
pub const STATE1_17: u32 = 1 << 17;
pub const STATE1_18: u32 = 1 << 18; // jumping
pub const STATE1_19: u32 = 1 << 19; // freefall
pub const STATE1_20: u32 = 1 << 20;
pub const STATE1_21: u32 = 1 << 21;
pub const STATE1_22: u32 = 1 << 22; // shielding
pub const STATE1_26: u32 = 1 << 26;
pub const STATE1_27: u32 = 1 << 27; // swimming
pub const STATE1_28: u32 = 1 << 28;
pub const STATE1_29: u32 = 1 << 29;
pub const STATE1_30: u32 = 1 << 30;
pub const STATE1_31: u32 = 1 << 31;
// stateFlags2
pub const STATE2_0: u32 = 1 << 0;
pub const STATE2_10: u32 = 1 << 10; // under the water surface
pub const STATE2_11: u32 = 1 << 11; // diving from the surface
pub const STATE2_1: u32 = 1 << 1;
pub const STATE2_13: u32 = 1 << 13; // keep the target (Switch Z-targeting)
pub const STATE2_21: u32 = 1 << 21;
pub const STATE2_2: u32 = 1 << 2;
pub const STATE2_3: u32 = 1 << 3;
pub const STATE2_5: u32 = 1 << 5; // facing follows currentYaw
pub const STATE2_6: u32 = 1 << 6;
pub const STATE2_8: u32 = 1 << 8;
pub const STATE2_9: u32 = 1 << 9;
pub const STATE2_12: u32 = 1 << 12;
pub const STATE2_14: u32 = 1 << 14;
pub const STATE2_16: u32 = 1 << 16;
pub const STATE2_18: u32 = 1 << 18;
pub const STATE2_19: u32 = 1 << 19; // side hop / backflip
pub const STATE2_22: u32 = 1 << 22;
pub const STATE2_26: u32 = 1 << 26;
pub const STATE2_27: u32 = 1 << 27;
pub const STATE2_28: u32 = 1 << 28; // idle fidget
pub const STATE2_17: u32 = 1 << 17; // spin attack
pub const STATE2_30: u32 = 1 << 30; // stab lunge
pub const STATE2_7: u32 = 1 << 7;
pub const STATE2_31: u32 = 1 << 31;
/// `PLAYER_STATE2_29`: not drawn (`Player_Draw`), while Link browses a shop's shelves
/// (`En_Ossan`).
pub const STATE2_29: u32 = 1 << 29;
// stateFlags3
pub const STATE3_1: u32 = 1 << 1;
pub const STATE3_3: u32 = 1 << 3;
pub const STATE3_4: u32 = 1 << 4;
pub const STATE3_7: u32 = 1 << 7;

// PlayerDoorType.
pub const PLAYER_DOORTYPE_AJAR: i8 = -1;
pub const PLAYER_DOORTYPE_NONE: i8 = 0;
pub const PLAYER_DOORTYPE_HANDLE: i8 = 1;
pub const PLAYER_DOORTYPE_SLIDING: i8 = 2;
pub const PLAYER_DOORTYPE_FAKE: i8 = 3;

/// What Player does to the play state, the camera and other actors during its update, applied
/// in order right after it (Player is out of the actor context while it updates; the C calls
/// these in place).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayRequest {
    /// `func_80835E44`: `Camera_ChangeSetting` on the main camera, unless the scene's camera
    /// is fixed (`Play_CamIsNotFixed`), where `CAM_SET_SCENE_TRANSITION` only changes the
    /// interface alpha.
    CamSetting(i16),
    /// `Camera_ChangeSetting(Play_GetCamera(play, CAM_ID_MAIN), setting)`.
    ChangeSetting(i16),
    /// `Camera_ChangeDoorCam(mainCam, door, bgCamIndex, 0, timer1, timer2, timer3)`.
    DoorCam { door: ActorHandle, bg_cam_index: i16, timers: [i16; 3] },
    /// `func_8005B1A4(mainCam)`.
    CamDone,
    /// `func_8009728C`: load a room.
    RoomLoad(i8),
    /// `func_80097534`: the previous room goes.
    RoomChangeDone,
    /// The door's `openAnim` and `playerIsOpening = true`.
    OpenDoor { door: ActorHandle, open_anim: u8 },
    /// `doorActor->room = play->roomCtx.curRoom.num`, and its double's.
    DoorRoom { door: ActorHandle },
    /// `actor->flags |= ACTOR_FLAG_8`: Player accepted the actor's talk offer (the actor's
    /// `Actor_ProcessTalkRequest` sees it in its update this frame).
    TalkRequest(ActorHandle),
    /// `actor->textId = text_id` (an ajar door's 0xD0).
    SetTextId { actor: ActorHandle, text_id: u16 },
    /// `Message_StartTextbox(play, textId, actor)`.
    StartTextbox { text_id: u16, actor: Option<ActorHandle> },
    /// `Interface_SetDoAction(play, action)`.
    DoAction(u16),
    /// `interactedActor->parent = &this->actor` (`func_8083E5A8`): Player took the actor's
    /// item (its `Actor_HasParent` is true from now).
    SetParent(ActorHandle),
    /// `chest->unk_1F4 = v`: Player opens the chest, with the long (1) or short (-1) animation.
    ChestOpen { chest: ActorHandle, unk_1f4: i16 },
    /// `Camera_SetCameraData(Play_GetCamera(play, CAM_ID_MAIN), 4, NULL, NULL, data2, 0, 0)`.
    SetCameraData { data2: i16 },
    /// `Item_DropCollectible(play, &pos, params)` (`func_8083E4C4`).
    DropCollectible { pos: Vec3, params: i16 },
    /// `Item_Give(play, item)`, in its place among the requests (after `Message_StartTextbox`,
    /// whose heart piece text counts the pieces before this one).
    ItemGive(u8),
    /// `Interface_SetNaviCall(play, naviCallState)`.
    NaviCall(u16),
    /// `func_8084DFF4`'s sound for item `get_item_id`, after its `Item_Give` (a heart piece's
    /// fanfare counts the pieces with this one): `Audio_PlayFanfare`, or a sound effect for a
    /// rupee or a heart.
    GetItemFanfare(i16),
    /// `func_800F6964(arg)`: every player fades out (the secret hole's exit).
    FadeOutAllSeq(u16),
    /// A sound, in its order among Player's calls.
    Sfx(PlayerSfx),
    /// `OnePointCutscene_Init(play, cs_id, timer, actor, parent)`, the actor Player himself
    /// (`player`) or NULL.
    OnePointCutscene { cs_id: i16, timer: i16, player: bool, parent: i16 },
}

/// Player's sounds: what its `func_8002F7DC`-style calls ask the audio for, at Player
/// (`&this->actor.projectedPos`) unless said.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerSfx {
    /// `func_80078884`: no position.
    NoPos(u16),
    /// `func_8002F7DC`: `Audio_PlaySfxGeneral` at Player, the default scales.
    Actor(u16),
    /// `func_800F4010`: a footstep at speed `f32`.
    Footstep(u16, f32),
    /// `func_800F4138`, `func_800F4190`.
    F4138(u16, f32),
    F4190(u16),
    /// `Audio_SetCodeReverb`: the floor's echo.
    CodeReverb(i8),
    /// `Audio_StopSfxById`.
    StopById(u16),
    /// `Audio_SetBaseFilter`: the underwater filter (and the bubbling).
    BaseFilter(u8),
}

/// `SurfaceSfxType` (`z64bgcheck.h`): the ones Player names.
const SURFACE_SFX_TYPE_1: u16 = 1;
const SURFACE_SFX_TYPE_4: u16 = 4;
const SURFACE_SFX_TYPE_5: u16 = 5;
/// `PLAYER_BOOTS_IRON` (`z64player.h`).
const PLAYER_BOOTS_IRON: u8 = 1;
use oot_game::audio::sfx::*;

// floor properties (FLOOR_PROPERTY_*)
const FLOOR_PROPERTY_5: u32 = 5;
const FLOOR_PROPERTY_12: u32 = 12;
const FLOOR_TYPE_4: u32 = 4;
const FLOOR_TYPE_10: u32 = 10;
const FLOOR_TYPE_11: u32 = 11;
const FLOOR_TYPE_12: u32 = 12;
const FLOOR_EFFECT_2: u32 = 2;
/// `ENTR_RETURN_YOUSEI_IZUMI_YOKO`, `ENTR_RETURN_GROTTO` (`z64scene.h`).
const ENTR_RETURN_YOUSEI_IZUMI_YOKO: u16 = 0x7FF9;
const ENTR_RETURN_GROTTO: u16 = 0x7FFF;
const FLOOR_PROPERTY_6: u32 = 6;
const FLOOR_PROPERTY_7: u32 = 7;
const FLOOR_PROPERTY_8: u32 = 8;
const FLOOR_PROPERTY_9: u32 = 9;
const FLOOR_TYPE_6: u32 = 6;
const FLOOR_TYPE_7: u32 = 7;
const FLOOR_TYPE_9: u32 = 9;
const FLOOR_TYPE_2: u32 = 2;
const FLOOR_TYPE_3: u32 = 3;
/// `SCENE_BMORI1` (`scene_table.h`): the Forest Temple.
const SCENE_BMORI1: u16 = 0x03;

/// `PLAYER_ANIMGROUP_*` indices used here (group 0 = wait, 1 = walk, 2 = run, ...).
pub mod group {
    pub const WAIT: usize = 0;
    pub const WALK: usize = 1;
    pub const RUN: usize = 2;
    pub const DAMAGE_RUN: usize = 3;
    pub const HEAVY_RUN: usize = 4;
    pub const LANDING: usize = 14;
    pub const SHORT_LANDING: usize = 15;
    pub const ROLL: usize = 16;
    pub const ROLL_BONK: usize = 17;
    pub const END_WALK_A: usize = 18;
    pub const END_WALK_B: usize = 19;
    pub const TURN: usize = 26;
    pub const TARGET_EXIT_R: usize = 27;
    pub const TARGET_EXIT_L: usize = 28;
    pub const PARALLEL_BACKWALK: usize = 31;
    pub const TARGET_WAIT_L: usize = 5;
    pub const TARGET_WAIT_R: usize = 6;
    pub const TARGET_ENTER: usize = 7;
    pub const SHORT_LANDING_: usize = 15;
    pub const PARALLEL_SIDEWALK: usize = 23;
    pub const SIDESTEP_R: usize = 24;
    pub const SIDESTEP_L: usize = 25;
    /// Climb up from a hang (`unk_84F` > 0).
    pub const HANG_CLIMB_UP: usize = 38;
    /// Mid-air ledge grab.
    pub const HANG_GRAB: usize = 39;
    /// Hanging loop after a mid-air grab.
    pub const HANG_WAIT: usize = 40;
    /// Climb up after a mid-air grab (`unk_84F` < 0).
    pub const HANG_GRAB_CLIMB_UP: usize = 41;
}

/// The action function in `this->func_674`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80840BC8`: standing (idle, landing, end of roll).
    StandingStill,
    /// `func_80842180`: walking and running.
    Run,
    /// `func_80841BA8`: turning in place.
    Turn,
    /// `func_80844708`: rolling.
    Roll,
    /// `func_8084411C`: in the air (jumping or falling).
    Midair,
    /// `func_8084BBE4`: hanging from a ledge.
    Hang,
    /// `func_8084BDFC`: climbing up from a hang.
    ClimbUp,
    /// `func_80845668`: stepping or jumping up onto a ledge from the ground.
    ClimbLedge,
    /// `func_80840450`: standing locked on to a target.
    TargetIdle,
    /// `func_808407CC`: standing in parallel mode (Z with nothing to target).
    ParallelIdle,
    /// `func_8084227C`: running forward while targeting.
    TargetRun,
    /// `func_8084193C`: sidestepping around a target.
    Sidestep,
    /// `func_808423EC`: stepping back from a locked target.
    TargetBackwalk,
    /// `func_8084251C`: braking after the step back.
    TargetBackBrake,
    /// `func_80840DE4`: walking sideways in parallel mode.
    ParallelWalk,
    /// `func_808414F8`: walking/running backwards in parallel mode.
    ParallelBackwalk,
    /// `func_8084170C`: braking out of a parallel back run.
    ParallelBackBrake,
    /// `func_808417FC`: end of that brake.
    ParallelBackBrakeEnd,
    /// `func_808502D0`: a melee attack (sword slash).
    Attack,
    /// `func_8084D610`: treading water.
    Swim,
    /// `func_8084D84C`: swimming forward.
    SwimMove,
    /// `func_8084DAB4`: swimming while targeting.
    SwimTarget,
    /// `func_8084DC48`: diving.
    Dive,
    /// `func_8084E1EC`: surfacing after a dive.
    Surface,
    /// `func_80845CA4`: walking through an exit, or in from an entrance.
    ExitWalk,
    /// `func_8084F88C`: falling into a void.
    VoidFall,
    /// `func_808458D0`: putting the held item away, then the pending action (`func_A74`).
    ItemPutAway,
    /// `func_8084BF1C`: on a ladder or a climbable wall.
    Climb,
    /// `func_8084C5F8`: stepping off a ladder at its top or bottom.
    ClimbEnd,
    /// `func_80845EF8`: opening a door and walking through it.
    DoorOpen,
    /// `func_8084B530`: talking, until the message box closes.
    Talk,
    /// `func_8084E6D4`: getting an item: a chest's opening, then the item held up over Link's
    /// head with its text.
    GetItem,
    /// `func_8084C760`: in a crawlspace, crawling.
    Crawl,
    /// `func_8084C81C`: climbing out of a crawlspace.
    CrawlExit,
    /// `func_8084370C`: staggering from a hit.
    Damaged,
    /// `func_8084377C`: knocked down, in the air until the landing.
    KnockedDown,
    /// `func_80843954`: lying on the ground after a knockdown.
    Down,
    /// `func_80843A38`: getting up.
    GetUp,
    /// `func_80852E14`: in a cutscene mode (`csMode`).
    Cutscene,
}

/// `func_A74`: what `func_808458D0` runs once the item is away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A74 {
    /// `func_8083A3B0`: start climbing.
    ClimbStart,
    /// `func_8083A2F8`: start talking.
    Talk,
    /// `func_8083A434`: the get-item action.
    GetItem,
    /// `func_8083A40C`: the crawl.
    Crawl,
}

/// Player's upper-body action (`this->func_82C`), run from `func_80836670`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpperAction {
    /// `func_8083485C`: nothing in hand (only the shield check).
    Default,
    /// `func_808349DC`: a sword in hand (shield check, pending item change).
    Sword,
    /// `func_80834A2C`: an item change playing on `skelAnime2`.
    Change,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::StandingStill => "func_80840BC8",
            Action::Run => "func_80842180",
            Action::Turn => "func_80841BA8",
            Action::Roll => "func_80844708",
            Action::Midair => "func_8084411C",
            Action::Hang => "func_8084BBE4",
            Action::ClimbUp => "func_8084BDFC",
            Action::ClimbLedge => "func_80845668",
            Action::TargetIdle => "func_80840450",
            Action::ParallelIdle => "func_808407CC",
            Action::TargetRun => "func_8084227C",
            Action::Sidestep => "func_8084193C",
            Action::TargetBackwalk => "func_808423EC",
            Action::TargetBackBrake => "func_8084251C",
            Action::ParallelWalk => "func_80840DE4",
            Action::ParallelBackwalk => "func_808414F8",
            Action::ParallelBackBrake => "func_8084170C",
            Action::ParallelBackBrakeEnd => "func_808417FC",
            Action::Attack => "func_808502D0",
            Action::Swim => "func_8084D610",
            Action::SwimMove => "func_8084D84C",
            Action::SwimTarget => "func_8084DAB4",
            Action::Dive => "func_8084DC48",
            Action::Surface => "func_8084E1EC",
            Action::ExitWalk => "func_80845CA4",
            Action::VoidFall => "func_8084F88C",
            Action::ItemPutAway => "func_808458D0",
            Action::Climb => "func_8084BF1C",
            Action::ClimbEnd => "func_8084C5F8",
            Action::DoorOpen => "func_80845EF8",
            Action::Talk => "func_8084B530",
            Action::GetItem => "func_8084E6D4",
            Action::Crawl => "func_8084C760",
            Action::CrawlExit => "func_8084C81C",
            Action::Damaged => "func_8084370C",
            Action::KnockedDown => "func_8084377C",
            Action::Down => "func_80843954",
            Action::GetUp => "func_80843A38",
            Action::Cutscene => "func_80852E14",
        }
    }
}

/// File-scope statics of `z_player.c` that Player's update shares between functions.
#[derive(Debug, Clone, Default)]
pub struct PlayerStatics {
    /// `D_808535D4`: stick magnitude 0..60 (`func_80077D10`).
    pub stick_mag: f32,
    /// `D_808535D8`: stick angle relative to up.
    pub stick_angle: i16,
    /// `D_808535DC`: stick direction in world space (camera input yaw + stick angle).
    pub stick_world_yaw: i16,
    /// `D_808535E0`: upper-body item action result (always 0 without items).
    pub item_action: i32,
    /// `D_808535E4`: floor type under Player.
    pub floor_type: u32,
    /// `D_808535E8`: animation/speed scale, 0.5 underwater and 1 otherwise.
    pub speed_scale: f32,
    /// `D_808535F0`: wall flags of the touched wall.
    pub wall_flags: u32,
    /// `D_80853600`: height above the floor.
    pub floor_dist: f32,
    /// `D_80853604`: floor property.
    pub floor_property: u32,
    /// `D_80853608`: |facing - wall normal|.
    pub wall_facing_diff: i32,
    /// `D_8085360C`: |movement yaw - wall normal|.
    pub wall_move_diff: i32,
    /// `D_80853610`: floor slope along the facing direction.
    pub facing_slope: i16,
    /// `D_80853614` / `D_80853618`: B pressed / held with the weapon already in hand this frame.
    pub d_80853614: bool,
    pub d_80853618: bool,
    /// `D_80858AA0`: the animation's `moveFlags` when a cutscene mode starts.
    pub d_80858aa0: i32,
}

/// Per-frame environment the update needs.
pub struct Env<'a> {
    pub data: &'a GameData,
    pub col: &'a CollisionContext,
    /// `Camera_GetInputDirYaw(GET_ACTIVE_CAM(play))`.
    pub cam_input_yaw: i16,
    /// `play->gameplayFrames` (face alternation).
    pub gameplay_frames: u32,
    /// The other actors (Player is taken out of the actor context while it updates), and last
    /// frame's target context.
    pub actors: &'a ActorContext,
    pub target: TargetView,
    /// What Player writes to the play state and the save (exits, voids, respawn points).
    pub io: &'a RefCell<PlayIo>,
    /// `play->setupExitList`, and `gEntranceTable`.
    pub exits: &'a [u16],
    pub entrances: &'a [EntranceInfo],
    /// `transiActorCtx.list`, and `roomCtx.prevRoom.num`.
    pub transi_actors: &'a [oot_game::scene::TransitionActorEntry],
    pub prev_room: i8,
    /// `Play_GetCamera(play, CAM_ID_MAIN)->unk_14C`, as the last camera update left it.
    pub cam_unk_14c: i16,
    /// `Message_GetState(&play->msgCtx)`, as the last `Message_Update` left it.
    pub msg_state: u8,
    /// `play->roomCtx.curRoom.behaviorType1`.
    pub room_behavior_type1: u8,
    /// Player's own handle (`&this->actor`; Player is out of `actors` while it updates).
    pub me: Option<ActorHandle>,
    /// `Camera_GetCamDirYaw(GET_ACTIVE_CAM(play))`, as the last camera update left it.
    pub cam_dir_yaw: i16,
    /// `sGetItemTable` (`table/items`; empty without the pack).
    pub items: &'a oot_game::item::ItemTables,
    /// The game's audio tables (`D_80119E10` for the floor's footsteps).
    pub audio: &'a oot_game::audio::AudioGameTables,
    /// `play->csCtx`'s `state`, `frames` and `linkAction`, and `play->sceneId`.
    pub cs_state: u8,
    pub cs_frames: u16,
    pub cs_link_action: Option<CsCmdActorAction>,
    pub scene_id: u16,
}

impl Env<'_> {
    /// An actor by handle (what `unk_664` holds).
    pub fn target(&self, h: ActorHandle) -> Option<&Actor> {
        self.actors.actor(h)
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub actor: Actor,
    pub skel: SkelAnime,
    pub action: Action,
    pub regs: Regs,
    pub age: AgeProperties,
    pub adult: bool,
    /// `modelAnimType` (0: nothing in hand).
    pub model_anim_type: usize,
    pub state1: u32,
    pub state2: u32,
    pub state3: u32,
    pub linear_velocity: f32,
    pub current_yaw: i16,
    pub target_yaw: i16,
    pub unk_84F: i8,
    /// `unk_840`: the frames spent under water (to 300).
    pub unk_840: i16,
    /// Multipurpose timer.
    pub unk_850: i16,
    /// Rolling index into the stick history below.
    pub unk_846: u8,
    /// Stick direction history (8 sectors), -1 when the stick is below 55.
    pub unk_847: [i8; 4],
    /// Stick direction relative to facing (0 forward, 1 left, 2 back, 3 right), -1 below 55.
    pub unk_84B: [i8; 4],
    pub unk_A7C: f32,
    pub unk_A80: i16,
    /// Walk start blend (0 → 1).
    pub unk_864: f32,
    /// Walk/run cycle phase in walk-animation frames (0..29).
    pub unk_868: f32,
    pub unk_870: f32,
    pub unk_874: f32,
    /// Current run speed limit (`R_RUN_SPEED_LIMIT`, reduced when running into walls).
    pub unk_880: f32,
    /// Floor slope along `currentYaw` and across it.
    pub unk_898: i16,
    pub unk_89A: i16,
    /// Walk climb/descend blend.
    pub unk_89C: i16,
    pub unk_6C2: i16,
    pub unk_6C4: f32,
    /// Which of the look/lean rotations were driven this frame (`func_80847298` decays the rest).
    pub unk_6AE: u16,
    pub unk_6AC: i8,
    pub unk_6AD: u8,
    /// Head: z (unk_6B6), y (unk_6B8), x (unk_6BA). Upper body: x (unk_6BC), y (unk_6BE), z (unk_6C0).
    pub unk_6B0: i16,
    pub unk_6B6: i16,
    pub unk_6B8: i16,
    pub unk_6BA: i16,
    pub unk_6BC: i16,
    pub unk_6BE: i16,
    pub unk_6C0: i16,
    pub unk_87C: i16,
    pub unk_87E: i16,
    pub unk_890: u8,
    pub unk_A7A: u32,
    pub unk_A79: u8,
    pub unk_A7B: u32,
    pub fall_start_height: i16,
    pub fall_distance: i16,
    pub wall_height: f32,
    pub wall_distance: f32,
    pub unk_88C: u8,
    pub unk_88D: u8,
    pub hover_boots_timer: u8,
    pub pushed_speed: f32,
    pub pushed_yaw: i16,
    pub melee_weapon_state: i8,
    /// `unk_664`: the targeted actor.
    pub unk_664: Option<ActorHandle>,
    /// What `Player_UpdateCamAndSeqModes` asked of the camera this update (applied by
    /// `ActorImpl::update`, which has the play state).
    pub cam_request: Option<(i16, Option<ActorHandle>)>,
    /// `unk_66C`: Z-target timer.
    pub unk_66C: i16,
    /// `bodyPartsPos[PLAYER_BODYPART_HEAD]` from the last draw (also `actor.focus.pos`).
    pub head_pos: Vec3,
    /// `skelAnime2`: the upper-body animation (item changes), merged into `skel` by `func_80836670`.
    pub skel2: SkelAnime,
    /// `func_82C`.
    pub upper: UpperAction,
    /// `heldItemActionParam`, `itemActionParam` (PLAYER_AP_*), `heldItemId` (the item being
    /// changed to, as its action param).
    pub held_item_ap: i32,
    pub item_ap: i32,
    pub held_item: Option<i32>,
    /// `modelGroup`, `nextModelGroup` (PLAYER_MODELGROUP_*).
    pub model_group: usize,
    pub next_model_group: usize,
    /// `currentShield` (`PLAYER_SHIELD_*`: 0 none, 1 Deku, 2 Hylian, 3 Mirror), `currentTunic`,
    /// `currentBoots`, `currentSwordItemId` (`B_BTN_ITEM`): from the save's equipment
    /// (`Player_SetEquipmentData`).
    pub current_shield: u8,
    pub current_tunic: u8,
    pub current_boots: u8,
    /// `unk_89E`: the floor's footstep sound (`SurfaceType_GetSfxId`, an offset from
    /// `NA_SE_PL_WALK_GROUND`), and `unk_A82`, the last frame's.
    pub unk_89E: u16,
    pub unk_A82: u16,
    pub current_sword_item_id: u8,
    /// `getItemId`: what `interactRangeActor` offers this frame (`func_8002F434`): positive to
    /// take at once, negative a chest's (opened on A), `GI_NONE` something to pick up. While
    /// getting an item, what's being got.
    pub get_item_id: i16,
    /// `interactRangeActor`, `getItemDirection`: the actor offering, and how squarely (reset to
    /// 0x6000 each update).
    pub interact_range_actor: Option<ActorHandle>,
    pub get_item_direction: i16,
    /// `unk_862`: the draw id plus one of the item held up (0: none; `Player_DrawGetItem`).
    pub unk_862: i16,
    /// `leftHandPos`: the left hand limb's origin in the last draw.
    pub left_hand_pos: Vec3,
    /// `play->gameplayFrames` at the last update (the held-up item's spin when drawn).
    pub gameplay_frames: u32,
    pub unk_15A: usize,
    pub unk_830: f32,
    pub unk_844: i8,
    pub unk_845: i8,
    /// `meleeWeaponAnimation` (PLAYER_MWA_*).
    pub melee_weapon_animation: usize,
    /// `cylinder`: the body (OC against everything, AC from enemies). Collider id 0.
    pub cylinder: ColliderCylinder,
    /// `meleeWeaponQuads`: the sword's two AT quads, set from the draw. Collider ids 1 and 2.
    pub melee_weapon_quads: [ColliderQuad; 2],
    /// `shieldQuad`: collider id 3 (shielding isn't ported, so it's never registered).
    pub shield_quad: ColliderQuad,
    /// `meleeWeaponInfo`: the sword's tip and base last frame, for the blur and the two quads.
    pub melee_weapon_info: [WeaponInfo; 3],
    /// `bodyPartsPos` (`PLAYER_BODYPART_*`), from the last draw.
    pub body_parts_pos: [Vec3; BODYPART_MAX],
    /// `targetActor`, `targetActorDistance`, `exchangeItemId`: who offered to talk this frame
    /// (`func_8002F1C4`). Accepting it (the talk interrupt) comes with the message box.
    pub target_actor: Option<ActorHandle>,
    pub target_actor_distance: f32,
    pub exchange_item_id: u8,
    pub s: PlayerStatics,
    pub input: Input,
    /// `unk_3A8` blink timer; `actor.shape.face`.
    pub blinker: Blinker,
    pub face: usize,
    rng: u32,
    /// Diagnostics: things the port saw but does not implement (ledge grab, climbing...).
    pub notes: Vec<String>,
    /// The last frame's foot IK per leg (left, right), when it ran.
    pub legs: Option<[oot_game::footik::LegResult; 2]>,
    pub frame: u32,
    /// `unk_450`: where an exit or entrance walk heads; `unk_45C`: a door's second target.
    pub unk_450: Vec3,
    pub unk_45C: Vec3,
    pub door_timer: i16,
    /// `doorType` (`PLAYER_DOORTYPE_*`), `doorDirection` (1 in front, -1 behind) and
    /// `doorActor`: the door that offered to open this frame (`EnDoor_Idle`), reset at the end
    /// of every update.
    pub door_type: i8,
    pub door_direction: i8,
    pub door_actor: Option<ActorHandle>,
    /// What the update asks of the play state (`PlayRequest`).
    pub play_requests: Vec<PlayRequest>,
    /// `unk_A84`: the height the void check measures falls from.
    pub unk_A84: i16,
    /// `csMode`, `prevCsMode`: the cutscene mode (`func_8002DF54`) and the one whose start ran.
    pub cs_mode: u8,
    pub prev_cs_mode: u8,
    /// `unk_446`: the `linkAction` cue being followed; `unk_448`: the actor a mode is about;
    /// `doorBgCamIndex`: 1 when `func_8002DF54` set the mode (Link is then held).
    pub unk_446: u16,
    pub unk_448: Option<ActorHandle>,
    pub door_bg_cam_index: i16,
    /// `func_A74`.
    pub func_a74: Option<A74>,
    /// Start mode 0 (`func_80846648`): `update` is a no-op and `draw` is NULL.
    pub inert: bool,
    /// `naviTextId`: what Navi says when C-Up talks to her (her update sets it; negative: she
    /// speaks at once), cleared at the end of every update.
    pub navi_text_id: i16,
    /// `naviActor`: Navi (`Player_SpawnFairy`).
    pub navi_actor: Option<ActorHandle>,
    /// `actor.shape.shadowDraw` is `ActorShadow_DrawFeet` (`Player_Init`), not `NULL` (Link
    /// asleep in the opening).
    pub shadow_feet: bool,
    /// `unk_88E`: frames after a talk that A, B and C-Up are ignored.
    pub unk_88E: u8,
    /// `unk_837`: frames before "Put Away" shows on the A button.
    pub unk_837: u8,
    /// `invincibilityTimer`: no damage while non-zero; positive counts down after a hit (and
    /// is visible), negative counts up (the roll's, when Link's cylinder is also AT).
    pub invincibility_timer: i8,
    /// `unk_88F`: `func_80837AE0`'s reset of the hit flash's phase (the flash isn't drawn).
    pub unk_88F: u8,
    /// `unk_8A0` .. `unk_8A8`: the knockback an actor asked for this frame (`func_8002F698`):
    /// the extra damage, the kind (1 a push, 2 a knockdown, 3 a shock), the yaw, the speed and
    /// the upward speed. `unk_8A1` is cleared at the end of every update.
    pub unk_8A0: u8,
    pub unk_8A1: u8,
    pub unk_8A2: i16,
    pub unk_8A4: f32,
    pub unk_8A8: f32,
    /// `unk_A86` (the Ganon fight's delayed damage; never set here), `unk_A87` (frames without
    /// damage).
    pub unk_A86: i8,
    pub unk_A87: u8,
}

/// `D_80854730`: the root translation Link's animations are authored around.
const BASE_TRANSL: [i16; 3] = [-57, 3377, 0];
/// `D_8085456C`: head-look floor probe offset.
const HEAD_PROBE: Vec3 = Vec3::new(0.0, 100.0, 40.0);

/// `D_808547C4`: each `linkAction` cue's cutscene mode (negative: without moving Link to the
/// cue's start). A cue past the table reads as 0 (the C reads past it).
#[rustfmt::skip]
const D_808547C4: [i8; 78] = [
    0,  3,  3,  5,   4,   8,   9,   13, 14, 15, 16, 17, 18, -22, 23, 24, 25,  26, 27,  28,  29, 31, 32, 33, 34, -35,
    30, 36, 38, -39, -40, -41, 42,  43, 45, 46, 0,  0,  0,  67,  48, 47, -50, 51, -52, -53, 54, 55, 56, 57, 58, 59,
    60, 61, 62, 63,  64,  -65, -66, 68, 11, 69, 70, 71, 8,  8,   72, 73, 78,  79, 80,  89,  90, 91, 92, 77, 19, 94,
];

pub fn d_808547c4(action: u16) -> i8 {
    D_808547C4.get(action as usize).copied().unwrap_or(0)
}

fn abs16(v: i16) -> i32 {
    (v as i32).abs()
}

impl Player {
    /// `Player_Init` / `Player_InitCommon` for a plain spawn standing at `pos`.
    pub fn new(data: &GameData, adult: bool, pos: Vec3, yaw: i16) -> Player {
        let age = data.ages[if adult { 0 } else { 1 }];
        let regs = data.regs[if adult { 0 } else { 1 }].clone();
        let model_anim_type = 0;
        let skel = SkelAnime::new_link(data, data.player_anim(group::WAIT, model_anim_type), BASE_TRANSL);
        let mut actor = Actor::new(pos, yaw);
        PROFILE.apply(&mut actor);
        actor.gravity = regs.reg(68) as f32 / 100.0;
        let mut p = Player {
            actor,
            skel,
            action: Action::StandingStill,
            unk_880: regs.run_speed_limit(),
            regs,
            age,
            adult,
            model_anim_type,
            state1: 0,
            state2: 0,
            state3: 0,
            linear_velocity: 0.0,
            current_yaw: yaw,
            target_yaw: 0,
            unk_84F: 0,
            unk_840: 0,
            unk_850: 0,
            unk_846: 0,
            unk_847: [-1; 4],
            unk_84B: [-1; 4],
            unk_A7C: 0.0,
            unk_A80: 0,
            unk_864: 0.0,
            unk_868: 0.0,
            unk_870: 0.0,
            unk_874: 0.0,
            unk_898: 0,
            unk_89A: 0,
            unk_89C: 0,
            unk_6C2: 0,
            unk_6C4: 0.0,
            unk_6AE: 0,
            unk_6AC: 0,
            unk_6AD: 0,
            unk_6B0: 0,
            unk_6B6: 0,
            unk_6B8: 0,
            unk_6BA: 0,
            unk_6BC: 0,
            unk_6BE: 0,
            unk_6C0: 0,
            unk_87C: 0,
            unk_87E: 0,
            unk_890: 0,
            unk_A7A: 0,
            unk_A79: 0,
            unk_A7B: 0,
            fall_start_height: pos.y as i16,
            fall_distance: 0,
            wall_height: 0.0,
            wall_distance: 0.0,
            unk_88C: 0,
            unk_88D: 0,
            hover_boots_timer: 0,
            pushed_speed: 0.0,
            pushed_yaw: 0,
            melee_weapon_state: 0,
            unk_664: None,
            cam_request: None,
            legs: None,
            unk_66C: 0,
            head_pos: pos + Vec3::Y * 50.0,
            skel2: SkelAnime::new_link(data, data.player_anim(group::WAIT, 0), BASE_TRANSL),
            upper: UpperAction::Default,
            held_item_ap: 0,
            item_ap: 0,
            held_item: None,
            model_group: data.items.model_group("DEFAULT"),
            next_model_group: data.items.model_group("DEFAULT"),
            // A plain start is the map select's file's (SaveContext::debug): the Kokiri Sword and
            // the Deku Shield for the child, the Master Sword and the Hylian Shield for the adult,
            // the Kokiri tunic and boots. Player_Init takes the save's (set_equipment_data).
            current_shield: if adult { 2 } else { 1 },
            current_tunic: 0,
            current_boots: 0,
            unk_89E: 0,
            unk_A82: 0,
            current_sword_item_id: if adult { oot_game::item::ITEM_SWORD_MASTER } else { oot_game::item::ITEM_SWORD_KOKIRI },
            get_item_id: 0,
            interact_range_actor: None,
            get_item_direction: 0x6000,
            unk_862: 0,
            left_hand_pos: pos,
            gameplay_frames: 0,
            unk_15A: 0,
            unk_830: 0.0,
            unk_844: 0,
            unk_845: 0,
            melee_weapon_animation: 0,
            cylinder: ColliderCylinder::new(&D_80854624),
            melee_weapon_quads: [ColliderQuad::new(&D_80854650), ColliderQuad::new(&D_80854650)],
            shield_quad: ColliderQuad::new(&D_808546A0),
            melee_weapon_info: [WeaponInfo::default(); 3],
            body_parts_pos: [pos; BODYPART_MAX],
            target_actor: None,
            target_actor_distance: f32::MAX,
            exchange_item_id: 0,
            s: PlayerStatics { speed_scale: 1.0, ..Default::default() },
            input: Input::default(),
            blinker: Blinker::default(),
            face: 0,
            rng: 0x1234_5678,
            notes: Vec::new(),
            frame: 0,
            unk_450: Vec3::ZERO,
            unk_45C: Vec3::ZERO,
            door_timer: 0,
            door_type: PLAYER_DOORTYPE_NONE,
            door_direction: 0,
            door_actor: None,
            play_requests: Vec::new(),
            unk_A84: pos.y as i16,
            cs_mode: 0,
            prev_cs_mode: 0,
            unk_446: 0,
            unk_448: None,
            door_bg_cam_index: 0,
            func_a74: None,
            inert: false,
            navi_text_id: 0,
            navi_actor: None,
            shadow_feet: true,
            unk_88E: 0,
            unk_837: 0,
            invincibility_timer: 0,
            unk_88F: 0,
            unk_8A0: 0,
            unk_8A1: 0,
            unk_8A2: 0,
            unk_8A4: 0.0,
            unk_8A8: 0.0,
            unk_A86: 0,
            unk_A87: 0,
        };
        // A plain start (the tests' and the sandbox's): standing still (`func_80853080`).
        p.func_80853080(data);
        p
    }

    /// `Player_Init` for a spawn from the scene's spawn list: the respawn point, then the
    /// start mode in `params` bits 8..11 (`D_80854738`).
    pub fn init(base: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let adult = play.save.adult;
        let data = play.data.clone();
        let mut p = Player::new(&data, adult, base.world_pos, base.shape_rot.y);
        p.set_equipment_data(&data, &play.save);
        let flags = p.actor.flags;
        p.actor = Actor { flags, gravity: p.actor.gravity, ..base };
        // thisx->room = -1: Player belongs to no room.
        p.actor.room = -1;
        p.current_yaw = p.actor.world_rot.y;
        let respawn_flag = play.save.respawn_flag;
        if respawn_flag != 0 {
            if respawn_flag == -3 {
                p.actor.params = play.save.respawn[RESPAWN_MODE_RETURN].player_params;
            } else {
                let mode = if respawn_flag < 0 { RESPAWN_MODE_DOWN } else { respawn_flag as usize - 1 };
                if respawn_flag > 0 {
                    let r = play.save.respawn[mode];
                    p.actor.world_pos = r.pos;
                    p.actor.home_pos = r.pos;
                    p.actor.prev_pos = r.pos;
                    p.fall_start_height = r.pos.y as i16;
                    p.actor.shape_rot.y = r.yaw;
                    p.current_yaw = r.yaw;
                    p.actor.params = r.player_params;
                }
                play.flags.temp_swch = play.save.respawn[mode].temp_swch_flags & 0xFF_FFFF;
                play.flags.temp_collect = play.save.respawn[mode].temp_collect_flags;
            }
        }
        // func_80845C68(play, respawnFlag == 2): the void-out respawn point is here.
        if respawn_flag != 2 {
            let (pos, yaw) = (p.actor.world_pos, p.actor.shape_rot.y);
            let mut io = play.take_io();
            io.setup_respawn_point(RESPAWN_MODE_DOWN, 0xDFF, pos, yaw);
            io.save.respawn[RESPAWN_MODE_DOWN].data = 0;
            io.save.respawn[RESPAWN_MODE_DOWN].player_params = (p.actor.params & 0xFF) | 0xD00;
            play.put_io(io);
        } else {
            play.save.respawn[RESPAWN_MODE_DOWN].data = 0;
        }
        play.save.respawn[RESPAWN_MODE_DOWN].data = 1;
        p.unk_A84 = p.actor.world_pos.y as i16;
        let mut init_mode = ((p.actor.params as u16 & 0xF00) >> 8) as u8;
        // The warp songs' and blue warps' starts are mode 13 in a cutscene layer.
        if (init_mode == 5 || init_mode == 6) && play.save.cutscene_index >= 0xFFF0 {
            init_mode = 13;
        }
        p.start_mode(play, init_mode);
        // GAMEMODE_NORMAL or GAMEMODE_END_CREDITS (3).
        if init_mode != 0 && (play.save.game_mode == oot_game::save::GAMEMODE_NORMAL || play.save.game_mode == 3) {
            // Player_SpawnFairy(play, this, &world.pos, &D_80854778, FAIRY_NAVI): Navi.
            let pos = p.func_808395DC(p.actor.world_pos, Vec3::new(0.0, 50.0, 0.0));
            match play.actor_spawn(crate::en_elf::ACTOR_EN_ELF, pos, [0; 3], crate::en_elf::FAIRY_NAVI) {
                Ok(h) => p.navi_actor = Some(h),
                Err(e) => log::debug!("Navi: {e:?}"),
            }
        }
        Box::new(p)
    }

    /// `Player_SetEquipmentData`: the shield, tunic and boots worn, and B's item, from the save;
    /// then the model group again for the item in hand (`Player_SetModelGroup`, which takes the
    /// shield into account). (`Player_SetBootData`: the Kokiri boots' registers are Player's
    /// already.)
    pub fn set_equipment_data(&mut self, data: &GameData, save: &oot_game::save::SaveContext) {
        use oot_game::item::{EQUIP_TYPE_BOOTS, EQUIP_TYPE_SHIELD, EQUIP_TYPE_TUNIC};
        // csMode 0x56 (a cutscene's equipment) never comes up.
        self.current_shield = save.cur_equip_value(EQUIP_TYPE_SHIELD) as u8;
        self.current_tunic = (save.cur_equip_value(EQUIP_TYPE_TUNIC) as u8).wrapping_sub(1);
        self.current_boots = (save.cur_equip_value(EQUIP_TYPE_BOOTS) as u8).wrapping_sub(1);
        self.current_sword_item_id = save.b_btn_item();
        let g = data.items.action_model_group.get(self.held_item_ap as usize).copied().unwrap_or(self.model_group);
        self.player_set_model_group(data, g);
    }

    /// `D_80854738[initMode]`.
    fn start_mode(&mut self, play: &mut PlayState, mode: u8) {
        let data = play.data.clone();
        match mode {
            // func_80846648: no update, no draw.
            0 => self.inert = true,
            // func_8083CA20.
            13 => {
                if self.func_8083C910(&data, &play.col, 180.0) {
                    self.unk_850 = -20;
                }
            }
            // func_8083CA54.
            8..=12 | 14 => {
                self.linear_velocity = 2.0;
                play.save.entrance_speed = 2.0;
                if self.func_8083C910(&data, &play.col, 120.0) {
                    self.unk_850 = -15;
                }
            }
            // func_8083CA9C.
            15 => {
                if play.save.entrance_speed < 0.1 {
                    play.save.entrance_speed = 0.1;
                }
                self.linear_velocity = play.save.entrance_speed;
                if self.func_8083C910(&data, &play.col, 800.0) {
                    self.unk_850 = (-80.0 / self.linear_velocity) as i16;
                    if self.unk_850 < -20 {
                        self.unk_850 = -20;
                    }
                }
            }
            // 1..=7: the Master Sword pedestal, warp songs, blue warps, the jump down into
            // Kokiri Forest's opening and so on: not ported. Standing still.
            m => self.note(format!("start mode {m} (D_80854738) not ported: standing")),
        }
    }

    /// `func_8083C910`: the entrance walk `dist` ahead, or swimming if the spawn is in deep
    /// water. True for the walk.
    fn func_8083C910(&mut self, data: &GameData, col: &CollisionContext, dist: f32) -> bool {
        if let Some(y) = col.water_surface(self.actor.world_pos.x, self.actor.world_pos.z, col.water_room)
            && y - self.actor.world_pos.y >= self.age.unk_24
        {
            // func_8084D7C4 (swimming in from the entrance) isn't ported: tread water.
            self.note("entrance into deep water: func_8084D7C4 not ported, treading water");
            self.func_80838F18(data);
            self.state1 |= STATE1_27 | STATE1_29;
            return false;
        }
        let yaw = self.actor.shape_rot.y;
        self.func_80838E70(data, dist, yaw);
        self.state1 |= STATE1_29;
        true
    }

    /// `func_80838E70`: walk `dist` towards `yaw` (`func_80845CA4`).
    fn func_80838E70(&mut self, data: &GameData, dist: f32, yaw: i16) {
        self.setup_action(data, Action::ExitWalk, 0);
        self.func_80832440();
        self.unk_84F = 1;
        self.unk_850 = 1;
        self.unk_450.x = sin_s(yaw) * dist + self.actor.world_pos.x;
        self.unk_450.z = cos_s(yaw) * dist + self.actor.world_pos.z;
        // func_80832264(play, this, func_80833338(this)): LinkAnimation_PlayOnce.
        let a = self.anim(data, group::WAIT);
        self.skel.play_once(data, a);
    }

    fn note(&mut self, s: impl Into<String>) {
        let s = s.into();
        if self.notes.last() != Some(&s) {
            log::debug!("player: {s}");
            self.notes.push(s);
        }
    }

    /// `Rand_ZeroOne` (`code_800FD970.c`).
    fn rand_zero_one(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f32::from_bits((self.rng >> 9) | 0x3F80_0000) - 1.0
    }

    fn anim(&self, data: &GameData, g: usize) -> AnimId {
        data.player_anim(g, self.model_anim_type)
    }

    pub fn grounded(&self) -> bool {
        self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0
    }

    // ================================================================================
    // Per-frame update

    /// `Player_Update` → `Player_UpdateCommon`, movement path. Call once per game frame
    /// with the frame's input; then `finish_frame` (the AnimationContext update).
    pub fn update(&mut self, env: &Env, input: Input) {
        self.frame += 1;
        self.gameplay_frames = env.gameplay_frames;
        self.input = input;
        let data = env.data;
        self.actor.prev_pos = self.actor.home_pos;

        // An offering actor that went (update == NULL) offers nothing.
        if self.interact_range_actor.is_some_and(|h| Some(h) != env.me && env.target(h).is_none_or(|a| a.killed)) {
            self.interact_range_actor = None;
        }
        // unk_A73/A87 timers, invincibility: not modelled.
        if self.unk_88E != 0 {
            self.unk_88E -= 1;
        }
        if self.unk_A87 != 0 {
            self.unk_A87 -= 1;
        }
        if self.invincibility_timer < 0 {
            self.invincibility_timer += 1;
        } else if self.invincibility_timer > 0 {
            self.invincibility_timer -= 1;
        }
        if self.unk_890 != 0 {
            self.unk_890 -= 1;
        }
        self.func_808473D4(env);
        self.func_80836BEC(env);

        scaled_step_to_s(&mut self.unk_6C2, 0, 400);
        // func_80032CB4(this->unk_3A8, 20, 80, 6) and the face alternation.
        let blink = self.blinker.step();
        self.face = blink + if env.gameplay_frames & 32 != 0 { 0 } else { 3 };

        if self.skel.move_flags & 0x80 == 0 {
            // Not on ice and no hover boots: speed and direction come straight from Player.
            self.actor.speed_xz = self.linear_velocity;
            self.actor.world_rot.y = self.current_yaw;
            self.actor.update_velocity();
            if self.pushed_speed != 0.0 {
                self.actor.velocity.x += self.pushed_speed * sin_s(self.pushed_yaw);
                self.actor.velocity.z += self.pushed_speed * cos_s(self.pushed_yaw);
            }
            self.actor.update_pos();
            self.func_80847BA0(env);
        }
        if self.pushed_speed != 0.0 {
            step_to_f(&mut self.pushed_speed, 0.0, 1.0);
        }

        if !self.in_blocking_cs_mode(env) && self.state2 & STATE2_18 == 0 {
            self.func_8083D53C(env);
            if env.io.borrow().save.health == 0 {
                // Dying (func_80836448, func_80837B9C) isn't ported: Link carries on.
                self.note("out of health: dying (func_80836448) not ported");
            }
            let trigger_start = env.io.borrow().transition.trigger == TRANS_TRIGGER_START;
            if self.actor.parent.is_none() && (trigger_start || self.unk_A87 != 0 || !self.func_808382DC(env)) {
                self.func_8083AA10(env);
            } else {
                self.fall_start_height = self.actor.world_pos.y as i16;
            }
            // (func_80848EF8: the rumble.)
        }

        // A script running: its cue's mode (6) or held still (0x31), unless riding, grabbed or
        // under water.
        if env.cs_state != oot_game::cutscene::CS_STATE_IDLE && self.cs_mode != 6 && self.state1 & STATE1_23 == 0 && self.state2 & STATE2_7 == 0 {
            if env.cs_link_action.is_some_and(|l| d_808547c4(l.action) != 0) {
                self.func_8002DF54(6);
                self.func_80832210();
            } else if self.cs_mode == 0 && self.state2 & STATE2_10 == 0 && env.cs_state != oot_game::cutscene::CS_STATE_UNSKIPPABLE_INIT {
                self.func_8002DF54(0x31);
                self.func_80832210();
            }
        }
        if self.cs_mode != 0 {
            if self.cs_mode != 7 || self.state1 & (STATE1_13 | STATE1_14 | STATE1_21 | STATE1_26) == 0 {
                self.unk_6AD = 3;
            } else if self.action != Action::Cutscene {
                self.func_80852944(env);
            }
        } else {
            self.prev_cs_mode = 0;
        }

        self.func_8083D6EC();

        if self.unk_664.is_none() && self.navi_text_id == 0 {
            self.state2 &= !(STATE2_1 | STATE2_21);
        }

        self.state1 &= !(STATE1_12 | STATE1_22 | (1 << 1) | (1 << 9));
        self.state2 &= !(STATE2_0 | STATE2_2 | STATE2_3 | STATE2_5 | STATE2_6 | STATE2_8 | (1 << 9) | STATE2_12 | STATE2_14 | STATE2_16 | STATE2_22 | STATE2_26);
        self.state3 &= !STATE3_4;

        self.func_80847298();
        self.func_8083315C(env);

        // D_808535E8: 0.5 while swimming.
        self.s.speed_scale = if self.state1 & STATE1_27 != 0 { 0.5 } else { 1.0 };

        self.s.d_80853614 = false;
        self.s.d_80853618 = false;
        if self.state3 & (1 << 2) == 0 {
            self.run_action(env);
        }

        self.cam_request = self.update_cam_and_seq_modes();
        if self.skel.move_flags & 8 != 0 {
            let s = if self.skel.move_flags & 4 != 0 { 1.0 } else { self.age.translation_scale };
            self.skel.request_move_actor(s);
        }
        self.func_808368EC(env);
        let _ = data;
        self.actor.home_pos = self.actor.world_pos;
    }

    /// `Player_UpdateCamAndSeqModes`'s camera half: the mode Player asks the main camera for
    /// (`Camera_ChangeMode`), with the actor it passes to `Camera_SetParam(camera, 8, ...)`,
    /// or `None` in first person (`PLAYER_STATE1_20`); `seq_mode` is its sequence half.
    /// Hookshot, `func_8084377C`, and the bow, slingshot and boomerang aren't ported, so
    /// their modes never come up.
    pub fn update_cam_and_seq_modes(&self) -> Option<(i16, Option<ActorHandle>)> {
        use oot_game::camera::*;
        if self.cs_mode != 0 {
            return Some((CAM_MODE_NORMAL, None));
        }
        if self.state1 & STATE1_20 != 0 {
            return None;
        }
        let mut target = None;
        let mode = if self.state2 & STATE2_8 != 0 {
            CAM_MODE_PUSHPULL
        } else if let Some(t) = self.unk_664 {
            target = Some(t);
            if self.actor.flags & ACTOR_FLAG_8 == ACTOR_FLAG_8 {
                CAM_MODE_TALK
            } else if self.state1 & STATE1_16 != 0 {
                if self.state1 & STATE1_25 != 0 { CAM_MODE_FOLLOWBOOMERANG } else { CAM_MODE_FOLLOWTARGET }
            } else {
                CAM_MODE_BATTLE
            }
        } else if self.state1 & STATE1_12 != 0 {
            CAM_MODE_CHARGE
        } else if self.state1 & STATE1_25 != 0 {
            CAM_MODE_FOLLOWBOOMERANG
        } else if self.state1 & (STATE1_13 | STATE1_14) != 0 {
            // func_80833B2C.
            if self.state1 & (STATE1_16 | STATE1_17 | STATE1_30) != 0 { CAM_MODE_HANGZ } else { CAM_MODE_HANG }
        } else if self.state1 & (STATE1_17 | STATE1_30) != 0 {
            // func_8002DD78 / func_808334B4 (bow, slingshot, boomerang in hand): not ported.
            if self.state1 & STATE1_21 != 0 { CAM_MODE_CLIMBZ } else { CAM_MODE_TARGET }
        } else if self.state1 & (STATE1_18 | STATE1_21) != 0 {
            if self.action == Action::ClimbLedge || self.state1 & STATE1_21 != 0 { CAM_MODE_CLIMB } else { CAM_MODE_JUMP }
        } else if self.state1 & STATE1_19 != 0 {
            CAM_MODE_FREEFALL
        } else if self.melee_weapon_state != 0
            // PLAYER_MWA_FORWARD_SLASH_1H (0) .. PLAYER_MWA_SPIN_ATTACK_1H (24).
            && self.melee_weapon_animation < 24
        {
            CAM_MODE_STILL
        } else {
            CAM_MODE_NORMAL
        };
        Some((mode, target))
    }

    /// `Player_UpdateCamAndSeqModes`' sequence half: `SEQ_MODE_STILL` standing still in the
    /// normal camera mode or in first person, else `SEQ_MODE_DEFAULT` (`SEQ_MODE_ENEMY` with an
    /// enemy near, `targetCtx.bgmEnemy`, which nothing sets yet). Player isn't riding
    /// (`PLAYER_STATE1_23`; Epona isn't ported).
    pub fn seq_mode(&self) -> u8 {
        use oot_game::audio::{SEQ_MODE_DEFAULT, SEQ_MODE_STILL};
        if self.cs_mode != 0 {
            return SEQ_MODE_DEFAULT;
        }
        match self.update_cam_and_seq_modes() {
            None => SEQ_MODE_STILL,
            Some((oot_game::camera::CAM_MODE_NORMAL, _)) if self.linear_velocity == 0.0 && self.state1 & STATE1_23 == 0 => SEQ_MODE_STILL,
            Some(_) => SEQ_MODE_DEFAULT,
        }
    }

    /// `AnimationContext_Update`, run after all actors (only Player here) have updated.
    pub fn finish_frame(&mut self) {
        self.skel.run_queue(&mut self.actor);
    }

    /// The draw-time part of Player that changes its joint table: `func_8008F87C` (foot IK) for
    /// both legs, as `Player_OverrideLimbDrawGameplayCommon` runs it for `PLAYER_LIMB_L_THIGH`
    /// and `PLAYER_LIMB_R_THIGH`. Call after `finish_frame` (Player_Draw follows every update).
    pub fn apply_foot_ik(&mut self, data: &GameData, col: &CollisionContext) -> [oot_game::footik::LegResult; 2] {
        // Player_OverrideLimbDrawGameplayCommon, PLAYER_LIMB_ROOT: child root scaling, minus unk_6C4.
        let j0 = self.skel.joint[0];
        let mut root = glam::Vec3::new(j0[0] as f32, j0[1] as f32, j0[2] as f32);
        if !self.adult {
            let mf = self.skel.move_flags;
            if mf & 4 == 0 || mf & 1 != 0 {
                root.x *= 0.64;
                root.z *= 0.64;
            }
            if mf & 4 == 0 || mf & 2 != 0 {
                root.y *= 0.64;
            }
        }
        root.y -= self.unk_6C4;
        let r = self.actor.shape_rot;
        let actor = oot_game::footik::actor_matrix(self.actor.world_pos, self.actor.shape_y_offset, [r.x, r.y, r.z]);
        let rig = &data.rigs[if self.adult { 0 } else { 1 }];
        let legs = [("L_THIGH", "L_SHIN", "L_FOOT"), ("R_THIGH", "R_SHIN", "R_FOOT")];
        let legs = legs.map(|(t, s, f)| {
            oot_game::footik::solve_leg(&data.foot_ik, rig, col, self.adult, actor, root, &mut self.skel.joint, self.unk_6C4, data.limb(t), data.limb(s), data.limb(f))
        });
        // Player_PostLimbDraw's bodyPartsPos[PLAYER_BODYPART_HEAD]: the head limb's origin (the
        // look rotations of the upper body are left out here).
        let head = oot_game::footik::limb_matrix(rig, actor, root, &self.skel.joint, data.limb("HEAD"));
        self.head_pos = head.transform_point3(Vec3::ZERO);
        // actor.focus.pos (Player_PostLimbDrawGameplay's head), which the others read (Navi's
        // point, Actor_SetNaviToActor).
        self.actor.focus_pos = self.head_pos;
        legs
    }

    fn run_action(&mut self, env: &Env) {
        match self.action {
            Action::StandingStill => self.func_80840BC8(env),
            Action::Cutscene => self.func_80852E14(env),
            Action::Run => self.func_80842180(env),
            Action::Turn => self.func_80841BA8(env),
            Action::Roll => self.func_80844708(env),
            Action::Midair => self.func_8084411C(env),
            Action::Hang => self.func_8084BBE4(env),
            Action::ClimbUp => self.func_8084BDFC(env),
            Action::ClimbLedge => self.func_80845668(env),
            Action::TargetIdle => self.func_80840450(env),
            Action::ParallelIdle => self.func_808407CC(env),
            Action::TargetRun => self.func_8084227C(env),
            Action::Sidestep => self.func_8084193C(env),
            Action::TargetBackwalk => self.func_808423EC(env),
            Action::TargetBackBrake => self.func_8084251C(env),
            Action::ParallelWalk => self.func_80840DE4(env),
            Action::ParallelBackwalk => self.func_808414F8(env),
            Action::ParallelBackBrake => self.func_8084170C(env),
            Action::ParallelBackBrakeEnd => self.func_808417FC(env),
            Action::Attack => self.func_808502D0(env),
            Action::Swim => self.func_8084D610(env),
            Action::SwimMove => self.func_8084D84C(env),
            Action::SwimTarget => self.func_8084DAB4(env),
            Action::Dive => self.func_8084DC48(env),
            Action::Surface => self.func_8084E1EC(env),
            Action::ExitWalk => self.func_80845CA4(env),
            Action::VoidFall => self.func_8084F88C(env),
            Action::ItemPutAway => self.func_808458D0(env),
            Action::Climb => self.func_8084BF1C(env),
            Action::ClimbEnd => self.func_8084C5F8(env),
            Action::DoorOpen => self.func_80845EF8(env),
            Action::Talk => self.func_8084B530(env),
            Action::GetItem => self.func_8084E6D4(env),
            Action::Crawl => self.func_8084C760(env),
            Action::CrawlExit => self.func_8084C81C(env),
            Action::Damaged => self.func_8084370C(env),
            Action::KnockedDown => self.func_8084377C(env),
            Action::Down => self.func_80843954(env),
            Action::GetUp => self.func_80843A38(env),
        }
    }

    /// `func_8083D6EC`: gravity and terminal velocity (water/sand parts not modelled).
    fn func_8083D6EC(&mut self) {
        self.actor.min_velocity_y = -20.0;
        self.actor.gravity = self.regs.reg(68) as f32 / 100.0;
    }

    /// `func_8083315C`: stick processing, once per frame.
    fn func_8083315C(&mut self, env: &Env) {
        self.unk_A7C = self.s.stick_mag;
        self.unk_A80 = self.s.stick_angle;
        let (mag, ang) = stick_to_mag_angle(&self.input);
        self.s.stick_mag = mag;
        self.s.stick_angle = ang;
        self.s.stick_world_yaw = env.cam_input_yaw.wrapping_add(ang);
        self.unk_846 = (self.unk_846 + 1) % 4;
        let (v1, v0) = if self.s.stick_mag < 55.0 {
            (-1i8, -1i8)
        } else {
            (
                ((self.s.stick_angle.wrapping_add(0x2000) as u16) >> 9) as i8,
                (((self.s.stick_world_yaw.wrapping_sub(self.actor.shape_rot.y)).wrapping_add(0x2000) as u16) >> 14) as i8,
            )
        };
        self.unk_847[self.unk_846 as usize] = v1;
        self.unk_84B[self.unk_846 as usize] = v0;
    }

    fn stick_dir(&self) -> i8 {
        self.unk_84B[self.unk_846 as usize]
    }

    // ================================================================================
    // Collision (func_80847BA0)

    /// `func_80839768`: line from Player's position (raised by `off.y`) to a point `off`
    /// ahead in facing space; walls only, one-sided.
    fn func_80839768(&self, env: &Env, off: Vec3) -> Option<(Vec3, PolyId)> {
        let a = Vec3::new(self.actor.world_pos.x, self.actor.world_pos.y + off.y, self.actor.world_pos.z);
        let b = self.func_808395DC(self.actor.world_pos, off);
        env.col.entity_line_test(a, b, true, false, false, true)
    }

    /// `func_808395DC`: `base` + `off` rotated by the facing yaw.
    fn func_808395DC(&self, base: Vec3, off: Vec3) -> Vec3 {
        let (c, s) = (cos_s(self.actor.shape_rot.y), sin_s(self.actor.shape_rot.y));
        Vec3::new(base.x + (off.x * c + off.z * s), base.y + off.y, base.z + (off.z * c - off.x * s))
    }

    /// `func_80847BA0`: Player's bg check and everything derived from the floor and wall.
    fn func_80847BA0(&mut self, env: &Env) {
        let col = env.col;
        let mut spc7 = 0u8;
        self.s.floor_property = self.unk_A7A;
        let (radius, wall_h, ceil_h) = if self.state2 & STATE2_18 != 0 {
            (10.0, 15.0, 30.0)
        } else {
            (self.age.wall_radius, 26.0, self.age.ceiling_check_height)
        };
        let flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4 | UPDBGCHECKINFO_FLAG_5;
        self.actor.update_bg_check_info(col, wall_h, radius, ceil_h, flags);
        // func_800432A0: a rotating platform turns Player's currentYaw too.
        self.current_yaw = self.current_yaw.wrapping_add(self.actor.carried_yaw);
        if self.actor.bg_check_flags & BGCHECKFLAG_CEILING != 0 {
            self.actor.velocity.y = 0.0;
        }
        self.s.floor_dist = self.actor.world_pos.y - self.actor.floor_height;
        if let Some(fp) = self.actor.floor_poly {
            self.unk_A7A = col.floor_property(fp);
            self.unk_A82 = self.unk_89E;
            if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 {
                self.unk_89E = if self.actor.y_dist_to_water < 20.0 { SURFACE_SFX_TYPE_4 } else { SURFACE_SFX_TYPE_5 };
            } else if self.state2 & STATE2_9 != 0 {
                self.unk_89E = SURFACE_SFX_TYPE_1;
            } else {
                // unk_89E is a sfxType, but SurfaceType_GetSfxId returns a sfxId (the decomp's
                // note): NA_SE_PL_WALK_* - SFX_FLAG, the offset from NA_SE_PL_WALK_GROUND.
                self.unk_89E = env.audio.surface_sfx_id(col.sfx_type(fp));
            }
            if self.actor.category == ACTORCAT_PLAYER {
                self.sfx(PlayerSfx::CodeReverb(col.echo(fp) as i8));
                // (Environment_ChangeLightSetting, DynaPoly_SetPlayerAbove: not ported here.)
            }
        }
        let floor = self.actor.floor_poly;
        self.func_80839034(env, floor);

        self.actor.bg_check_flags &= !BGCHECKFLAG_PLAYER_WALL_INTERACT;
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            let probe = Vec3::new(0.0, 18.0, self.age.wall_radius + 10.0);
            if self.state2 & STATE2_18 == 0 {
                if let Some((_, poly)) = self.func_80839768(env, probe) {
                    self.actor.bg_check_flags |= BGCHECKFLAG_PLAYER_WALL_INTERACT;
                    if self.actor.wall_poly != Some(poly) {
                        self.actor.wall_poly = Some(poly);
                        let n = col.poly(poly).normal;
                        self.actor.wall_yaw = atan2_s(n[2] as f32, n[0] as f32);
                    }
                }
            }
            let wall_back = self.actor.wall_yaw.wrapping_add(-0x8000i32 as i16);
            let sp9a = self.actor.shape_rot.y.wrapping_sub(wall_back);
            self.s.wall_flags = self.actor.wall_poly.map(|w| col.wall_flags(w)).unwrap_or(0);
            self.s.wall_facing_diff = abs16(sp9a);
            let sp9a = self.current_yaw.wrapping_sub(wall_back);
            self.s.wall_move_diff = abs16(sp9a);
            let k = self.s.wall_move_diff as f32 * 0.00008;
            let rsl = self.regs.run_speed_limit();
            if !self.grounded() || k >= 1.0 {
                self.unk_880 = rsl;
            } else {
                self.unk_880 = (rsl * k).max(0.1);
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0 && self.s.wall_facing_diff < 0x3000 {
                if let Some(wp) = self.actor.wall_poly {
                    spc7 = self.wall_height_class(env, wp, probe);
                }
            }
        } else {
            self.unk_880 = self.regs.run_speed_limit();
            self.unk_88D = 0;
            self.wall_height = 0.0;
        }
        if spc7 == self.unk_88C {
            if self.linear_velocity != 0.0 && self.unk_88D < 100 {
                self.unk_88D += 1;
            }
        } else {
            self.unk_88C = spc7;
            self.unk_88D = 0;
        }

        if self.grounded() {
            self.s.floor_type = self.actor.floor_poly.map(|p| col.floor_type(p)).unwrap_or(0);
            if !self.func_80847A78() {
                if let Some(fp) = self.actor.floor_poly {
                    let n = col.poly_normal(fp);
                    let inv_ny = 1.0 / n.y;
                    let (s, c) = (sin_s(self.current_yaw), cos_s(self.current_yaw));
                    self.unk_898 = atan2_s(1.0, (-(n.x * s) - (n.z * c)) * inv_ny);
                    self.unk_89A = atan2_s(1.0, (-(n.x * c) - (n.z * s)) * inv_ny);
                    let (s, c) = (sin_s(self.actor.shape_rot.y), cos_s(self.actor.shape_rot.y));
                    self.s.facing_slope = atan2_s(1.0, (-(n.x * s) - (n.z * c)) * inv_ny);
                    // func_8083E318: slippery slopes (FLOOR_EFFECT_1) are not in the course.
                    if col.floor_effect(fp) == 1 {
                        self.note("slippery slope (func_8083E318) not modelled");
                    }
                }
            }
        } else {
            self.func_80847A78();
        }
        if self.unk_A7B == self.s.floor_type {
            self.unk_A79 = self.unk_A79.saturating_add(1);
        } else {
            self.unk_A7B = self.s.floor_type;
            self.unk_A79 = 0;
        }
    }

    /// The wall-top probe in `func_80847BA0`: sets `wallHeight` and returns the climb class
    /// (`spC7`: 1 step, 2..4 increasingly tall ledges).
    fn wall_height_class(&mut self, env: &Env, wp: PolyId, probe: Vec3) -> u8 {
        let col = env.col;
        let wall = col.poly(wp);
        if abs16(wall.normal[1]) >= 600 {
            return 0;
        }
        let n = col.poly_normal(wp);
        self.wall_distance = udist_plane_to_pos(n, wall.dist as f32, self.actor.world_pos);
        let k = self.wall_distance + 10.0;
        let top_probe = Vec3::new(self.actor.world_pos.x - k * n.x, self.actor.world_pos.y + self.age.wall_probe_height, self.actor.world_pos.z - k * n.z);
        let (top, ground) = col.entity_raycast_down(top_probe);
        self.wall_height = top - self.actor.world_pos.y;
        let ceiling = col
            .check_ceiling(eng_collision::bgcheck::IGNORE_ENTITY, self.actor.world_pos, (top - self.actor.world_pos.y) + 20.0)
            .is_some();
        if self.wall_height < 18.0 || ceiling {
            self.wall_height = 399.96002;
            return 0;
        }
        let probe2 = Vec3::new(probe.x, (top + 5.0) - self.actor.world_pos.y, probe.z);
        if let Some((_, p2)) = self.func_80839768(env, probe2) {
            let n2 = col.poly(p2).normal;
            let d = self.actor.wall_yaw.wrapping_sub(atan2_s(n2[2] as f32, n2[0] as f32));
            if abs16(d) < 0x4000 && col.wall_flags(p2) & WALL_FLAG_1 == 0 {
                self.wall_height = 399.96002;
                return 0;
            }
        }
        if col.wall_flags(wp) & WALL_FLAG_0 != 0 {
            return 0;
        }
        if self.age.unk_1C <= self.wall_height {
            let steep = ground.map(|g| abs16(col.poly(g).normal[1]) > 28000).unwrap_or(false);
            if steep {
                if self.age.unk_14 <= self.wall_height {
                    4
                } else if self.age.unk_18 <= self.wall_height {
                    3
                } else {
                    2
                }
            } else {
                0
            }
        } else {
            1
        }
    }

    /// `func_80847A78`: hover boots bookkeeping; returns true when airborne.
    fn func_80847A78(&mut self) -> bool {
        // Kokiri boots: the timer never counts.
        self.hover_boots_timer = 0;
        if self.grounded() {
            self.hover_boots_timer = 19;
            return false;
        }
        self.s.floor_type = 0;
        self.unk_898 = 0;
        self.unk_89A = 0;
        self.s.facing_slope = 0;
        true
    }

    /// `func_8083AA10`: leaving the ground. Starts an auto-jump off a ledge when running
    /// forward fast, otherwise a fall (or a ledge grab, which is not modelled).
    fn func_8083AA10(&mut self, env: &Env) {
        let data = env.data;
        self.fall_distance = self.fall_start_height.wrapping_sub(self.actor.world_pos.y as i32 as i16);
        if self.state1 & (STATE1_27 | STATE1_29) == 0 && !self.grounded() {
            if self.func_80838FB8(env) {
                return;
            }
            if self.s.floor_property == FLOOR_PROPERTY_8 {
                self.actor.world_pos.x = self.actor.prev_pos.x;
                self.actor.world_pos.z = self.actor.prev_pos.z;
                return;
            }
            if self.state3 & STATE3_1 == 0 && self.skel.move_flags & 0x80 == 0 && self.action != Action::Midair {
                if self.s.floor_property == FLOOR_PROPERTY_7 || self.melee_weapon_state != 0 {
                    self.actor.world_pos = self.actor.prev_pos;
                    self.func_80832210();
                    return;
                }
                if self.hover_boots_timer != 0 {
                    self.actor.velocity.y = 1.0;
                    self.s.floor_property = FLOOR_PROPERTY_9;
                    return;
                }
                let sp5c = self.current_yaw.wrapping_sub(self.actor.shape_rot.y);
                self.setup_action(data, Action::Midair, 1);
                self.func_80832440();
                if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_LEAVE != 0
                    && self.state1 & STATE1_27 == 0
                    && self.s.floor_property != FLOOR_PROPERTY_6
                    && self.s.floor_property != FLOOR_PROPERTY_9
                    && self.s.floor_dist > 20.0
                    && self.melee_weapon_state == 0
                    && abs16(sp5c) < 0x2000
                    && self.linear_velocity > 3.0
                {
                    // FLOOR_PROPERTY_11 jump into water: no water.
                    self.func_8083A4A8(data);
                    return;
                }
                if self.s.floor_property == FLOOR_PROPERTY_9 || self.s.floor_dist <= self.age.ledge_grab_min_drop || !self.func_8083A6AC(env) {
                    let a = data.anim("link_normal_landing_wait");
                    self.skel.play_loop(data, a);
                }
            }
        } else {
            self.fall_start_height = self.actor.world_pos.y as i32 as i16;
        }
    }

    /// `func_8083A6AC`: grab the ledge Player just walked off: a line back towards the previous
    /// position finds the cliff face, and Player hangs from its top (`func_8083A5C4`).
    fn func_8083A6AC(&mut self, env: &Env) -> bool {
        let data = env.data;
        if self.actor.y_dist_to_water < -80.0 && abs16(self.unk_898) < 2730 && abs16(self.unk_89A) < 2730 {
            let d = Vec3::new(self.actor.prev_pos.x - self.actor.world_pos.x, 0.0, self.actor.prev_pos.z - self.actor.world_pos.z);
            let len = (d.x * d.x + d.z * d.z).sqrt();
            let k = if len != 0.0 { 5.0 / len } else { 0.0 };
            let b = Vec3::new(self.actor.prev_pos.x + d.x * k, self.actor.world_pos.y, self.actor.prev_pos.z + d.z * k);
            if let Some((_, poly)) = env.col.entity_line_test(self.actor.world_pos, b, true, false, false, true)
                && abs16(env.col.poly(poly).normal[1]) < 600
            {
                let n = env.col.poly_normal(poly);
                let sp54 = udist_plane_to_pos(n, env.col.poly(poly).dist as f32, self.actor.world_pos);
                let climbable = self.s.floor_property == FLOOR_PROPERTY_6 || env.col.wall_flags(poly) & WALL_FLAG_3 != 0;
                let anim = if climbable { data.anim("link_normal_Fclimb_startB") } else { data.anim("link_normal_fall") };
                self.func_8083A5C4(data, env, poly, sp54, anim);
                if climbable {
                    // Down onto a climbable wall below the edge.
                    self.func_80836898(data, env, A74::ClimbStart);
                    self.current_yaw = self.current_yaw.wrapping_add(i16::MIN);
                    self.actor.shape_rot.y = self.current_yaw;
                    self.state1 |= STATE1_21;
                    self.func_80832F54(0x9F);
                    self.unk_850 = -1;
                    self.unk_84F = 1;
                } else {
                    self.state1 |= STATE1_13;
                    self.state1 &= !STATE1_17;
                }
                self.func_8002F7DC(NA_SE_PL_SLIPDOWN);
                self.func_80832698(NA_SE_VO_LI_HANG);
                return true;
            }
        }
        false
    }

    /// `func_8083A5C4`: start hanging from the top edge of `wall`, `dist` away from its plane.
    fn func_8083A5C4(&mut self, data: &GameData, env: &Env, wall: PolyId, dist: f32, anim: AnimId) {
        let n = env.col.poly_normal(wall);
        self.setup_action(data, Action::Hang, 0);
        // func_80832564: func_80832440, and drop a held actor (none).
        self.func_80832440();
        self.skel.play_once(data, anim);
        self.actor.world_pos.x -= (dist + 1.0) * n.x;
        self.actor.world_pos.z -= (dist + 1.0) * n.z;
        self.current_yaw = atan2_s(n.z, n.x);
        self.actor.shape_rot.y = self.current_yaw;
        self.func_80832224();
        self.func_80832CFC();
    }

    /// `func_80832224`.
    fn func_80832224(&mut self) {
        self.func_80832210();
        self.unk_6AD = 0;
    }

    /// `func_80832E48`: apply the root motion accumulated so far to the actor (`flags` 1: x/z,
    /// 2: y, 4: y unscaled), then fold the root yaw into the facing.
    fn func_80832E48(&mut self, flags: u16) {
        self.skel.move_flags = flags;
        self.skel.prev_transl = self.skel.base_transl;
        let mut pos = self.skel.update_translation(self.actor.shape_rot.y);
        if flags & 1 != 0 {
            if !self.adult {
                pos.x *= 0.64;
                pos.z *= 0.64;
            }
            self.actor.world_pos.x += pos.x * self.actor.scale.x;
            self.actor.world_pos.z += pos.z * self.actor.scale.z;
        }
        if flags & 2 != 0 {
            if flags & 4 == 0 {
                pos.y *= self.age.translation_scale;
            }
            self.actor.world_pos.y += pos.y * self.actor.scale.y;
        }
        self.func_808322FC();
    }

    /// `func_80837B60`: bake the whole root offset (the hanging body) into the position.
    fn func_80837B60(&mut self) {
        self.skel.prev_transl = self.skel.joint[0];
        self.func_80832E48(3);
    }

    /// `func_80837B9C`: let go and fall.
    fn func_80837B9C(&mut self, data: &GameData) {
        self.setup_action(data, Action::Midair, 0);
        let a = data.anim("link_normal_landing_wait");
        self.skel.play_loop(data, a);
        self.unk_850 = 1;
        if self.unk_6AD != 3 {
            self.unk_6AD = 0;
        }
    }

    /// `func_8084BBE4`: hanging from a ledge. Any stick direction climbs up; A lets go.
    fn func_8084BBE4(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_6;
        let fall = data.anim("link_normal_fall");
        if self.skel.update(data) {
            let anim = if self.unk_84F > 0 { data.anim("link_normal_fall_wait") } else { self.anim(data, group::HANG_WAIT) };
            self.skel.play_loop(data, anim);
        } else if self.unk_84F == 0 {
            let f = if self.skel.animation == fall { 11.0 } else { 1.0 };
            if self.skel.on_frame(f) {
                self.func_80832770(NA_SE_PL_WALK_GROUND);
                self.unk_84F = if self.skel.animation == fall { 1 } else { -1 };
            }
        }
        scaled_step_to_s(&mut self.actor.shape_rot.y, self.current_yaw, 0x800);
        if self.unk_84F != 0 {
            if self.unk_847[self.unk_846 as usize] >= 0 {
                let anim = if self.unk_84F > 0 { self.anim(data, group::HANG_CLIMB_UP) } else { self.anim(data, group::HANG_GRAB_CLIMB_UP) };
                self.func_8083A9B8(data, anim);
                return;
            }
            // `actor.shape.feetFloorFlag` (feet touching a floor, set at draw time) also lets go;
            // not modelled.
            if self.input.cur.held(BTN_A) {
                self.func_80837B60();
                self.linear_velocity = if self.unk_84F < 0 { -0.8 } else { 0.8 };
                self.func_80837B9C(data);
                self.state1 &= !(STATE1_13 | STATE1_14);
            }
        }
    }

    /// `func_8083A9B8`: start climbing up from the hang.
    fn func_8083A9B8(&mut self, data: &GameData, anim: AnimId) {
        self.setup_action(data, Action::ClimbUp, 0);
        self.skel.play_once_set_speed(data, anim, 1.3);
    }

    /// `func_8084BDFC`: climbing up; at the end the animation's x/z root motion is applied
    /// and Player stands on the ledge.
    fn func_8084BDFC(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_6;
        if self.skel.update(data) {
            self.func_80832E48(1);
            self.func_8083C0E8(data);
            return;
        }
        if self.skel.on_frame(self.skel.end_frame - 6.0) {
            self.func_808328A0();
        } else if self.skel.on_frame(self.skel.end_frame - 34.0) {
            self.state1 &= !(STATE1_13 | STATE1_14);
            self.func_8002F7DC(NA_SE_PL_CLIMB_CLIFF);
            self.func_80832698(NA_SE_VO_LI_CLIMB_END);
        }
    }

    /// `func_80838A14` (interrupt 12): climb onto the wall in front when it's ledge height
    /// (`unk_88C` ≥ 2) after pushing into it for 6 frames or on A; hop up a low step
    /// (`unk_88C` = 1) after 3 frames.
    fn func_80838A14(&mut self, env: &Env) -> bool {
        let data = env.data;
        let swimming = self.state1 & STATE1_27 != 0;
        if self.state1 & STATE1_11 == 0 && self.unk_88C >= 2 && (!swimming || self.age.unk_14 > self.wall_height) {
            if swimming {
                // func_808332B8: swimming with Kokiri boots.
                if self.actor.y_dist_to_water < 50.0 {
                    if self.unk_88C < 2 || self.wall_height > self.age.unk_10 {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if !self.grounded() || (self.age.unk_14 <= self.wall_height && swimming) {
                return false;
            }
            // The WALL_FLAG_6 prompt case applies to dynamic collision only (wallBgId != BGCHECK_SCENE).
            if self.unk_88D >= 6 || self.input.press.held(BTN_A) {
                self.setup_action(data, Action::ClimbLedge, 0);
                self.state1 |= STATE1_18;
                let mut sp34 = self.wall_height;
                let anim;
                if self.age.unk_14 <= sp34 {
                    anim = data.anim("link_normal_250jump_start");
                    self.linear_velocity = 1.0;
                } else {
                    let n = self.actor.wall_poly.map(|w| env.col.poly_normal(w)).unwrap_or(Vec3::ZERO);
                    let sp24 = self.wall_distance + 0.5;
                    self.state1 |= STATE1_14;
                    let k = self.age.translation_scale;
                    if swimming {
                        anim = data.anim("link_swimer_swim_15step_up");
                        sp34 -= 60.0 * k;
                        self.state1 &= !STATE1_27;
                    } else if self.age.unk_18 <= sp34 {
                        anim = data.anim("link_normal_150step_up");
                        sp34 -= 59.0 * k;
                    } else {
                        anim = data.anim("link_normal_100step_up");
                        sp34 -= 41.0 * k;
                    }
                    self.actor.shape_y_offset -= sp34 * 100.0;
                    self.actor.world_pos.x -= sp24 * n.x;
                    self.actor.world_pos.y += self.wall_height;
                    self.actor.world_pos.z -= sp24 * n.z;
                    self.func_80832224();
                }
                self.actor.bg_check_flags |= BGCHECKFLAG_GROUND;
                self.skel.play_once_set_speed(data, anim, 1.3);
                self.skel.disable_queue();
                self.current_yaw = self.actor.wall_yaw.wrapping_add(i16::MIN);
                self.actor.shape_rot.y = self.current_yaw;
                return true;
            }
        } else if self.grounded() && self.unk_88C == 1 && self.unk_88D >= 3 {
            // func_808389E8: hop up a low step.
            let vy = self.wall_height * 0.08 + 5.5;
            let a = data.anim("link_normal_jump");
            self.func_80838940(data, Some(a), vy, NA_SE_VO_LI_SWORD_N);
            self.linear_velocity = 2.5;
            return true;
        }
        false
    }

    // ================================================================================
    // Climbing (ladders, vines and climbable walls)

    /// `func_80836898`: the action `f` once the held item is put away (`func_808458D0`).
    fn func_80836898(&mut self, data: &GameData, env: &Env, f: A74) -> bool {
        let _ = env;
        self.func_a74 = Some(f);
        self.setup_action(data, Action::ItemPutAway, 0);
        self.state2 |= STATE2_6;
        self.func_80832528(data)
    }

    /// `func_80832528`: put a held item away (`func_80835F44(ITEM_NONE)`).
    fn func_80832528(&mut self, data: &GameData) -> bool {
        if self.held_item_ap >= data.items.ap("FISHING_POLE") {
            self.func_80835F44(data, 0);
            true
        } else {
            false
        }
    }

    /// `func_808458D0`: the item change plays; when it's done (or there was none), the
    /// pending action.
    fn func_808458D0(&mut self, env: &Env) {
        self.state2 |= STATE2_5 | STATE2_6;
        self.skel.update(env.data);
        // (Holding an actor with no item to get: Player doesn't hold actors yet.)
        if !self.func_80836670(env) {
            match self.func_a74 {
                Some(A74::ClimbStart) => self.func_8083A3B0(env.data),
                Some(A74::Talk) => self.func_8083A2F8(env.data),
                Some(A74::GetItem) => self.func_8083A434(env.data),
                Some(A74::Crawl) => self.func_8083A40C(env.data),
                None => {}
            }
        }
    }

    /// `func_8083A3B0`: climbing, keeping `unk_850` and `unk_84F`.
    fn func_8083A3B0(&mut self, data: &GameData) {
        let (sp1c, sp18) = (self.unk_850, self.unk_84F);
        self.func_80835DAC(data, Action::Climb, 0);
        self.actor.velocity.y = 0.0;
        self.unk_850 = sp1c;
        self.unk_84F = sp18;
    }

    /// `func_8083F7BC` (interrupt 5): at a wall Link faces: climbing it (`func_8083EC18`),
    /// entering a crawlspace (`func_8083F0C8`), or pushing it. Pushing and pulling
    /// (`func_8083F72C`, `Bg_Heavy_Block`) aren't ported: A at a pushable wall only shows
    /// "Grab" (`PLAYER_STATE2_0`).
    fn func_8083F7BC(&mut self, env: &Env) -> bool {
        if self.state1 & STATE1_11 == 0 && self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0 && self.s.wall_facing_diff < 0x3000 {
            let flags = self.s.wall_flags;
            if (self.linear_velocity > 0.0 && self.func_8083EC18(env, flags)) || self.func_8083F0C8(env, flags) {
                return true;
            }
            if !self.func_808332B8() && (self.linear_velocity == 0.0 || self.state2 & STATE2_2 == 0) && flags & WALL_FLAG_6 != 0 && self.grounded() && self.wall_height >= 39.0 {
                self.state2 |= STATE2_0;
                if self.input.cur.held(BTN_A) {
                    self.note("pushing a wall (func_8083F72C) not ported");
                }
            }
        }
        false
    }

    /// `func_8083F0C8`: a child at a crawlspace's wall (`WALL_FLAG_4`, `_5`), within 8 of the
    /// line through the middle of its triangle: A says "Enter" (`PLAYER_STATE2_16`). On A,
    /// into the crawlspace (`PLAYER_STATE2_18`): Link lined up with the triangle's middle at
    /// his distance from the wall, facing it, `gPlayerAnim_link_child_tunnel_start` moving him
    /// in (`moveFlags` 0x9D), and the crawl once the item is away (`func_8083A40C`).
    fn func_8083F0C8(&mut self, env: &Env, arg2: u32) -> bool {
        if self.adult || self.state1 & STATE1_27 != 0 || arg2 & (WALL_FLAG_4 | WALL_FLAG_5) == 0 {
            return false;
        }
        let Some(wall) = self.actor.wall_poly else { return false };
        let col = env.col;
        let v = col.poly_vertices(wall);
        let (mut sp4c, mut phi_f2, mut sp44, mut phi_f12) = (v[0].x, v[0].x, v[0].z, v[0].z);
        for p in &v[1..] {
            if sp4c > p.x {
                sp4c = p.x;
            } else if phi_f2 < p.x {
                phi_f2 = p.x;
            }
            if sp44 > p.z {
                sp44 = p.z;
            } else if phi_f12 < p.z {
                phi_f12 = p.z;
            }
        }
        let sp4c = (sp4c + phi_f2) * 0.5;
        let sp44 = (sp44 + phi_f12) * 0.5;
        let n = col.poly_normal(wall);
        let d = ((self.actor.world_pos.x - sp4c) * n.z) - ((self.actor.world_pos.z - sp44) * n.x);
        if d.abs() < 8.0 {
            self.state2 |= STATE2_16;
            if self.input.press.held(BTN_A) {
                let data = env.data;
                let sp30 = self.wall_distance;
                self.func_80836898(data, env, A74::Crawl);
                self.state2 |= STATE2_18;
                self.current_yaw = self.actor.wall_yaw.wrapping_add(i16::MIN);
                self.actor.shape_rot.y = self.current_yaw;
                self.actor.world_pos.x = sp4c + sp30 * n.x;
                self.actor.world_pos.z = sp44 + sp30 * n.z;
                self.func_80832224();
                self.actor.prev_pos = self.actor.world_pos;
                self.skel.play_once(data, data.anim("link_child_tunnel_start"));
                self.func_80832F54(0x9D);
                return true;
            }
        }
        false
    }

    /// `func_8083A40C`: the crawl (`func_8084C760`).
    fn func_8083A40C(&mut self, data: &GameData) {
        self.func_80835DAC(data, Action::Crawl, 0);
    }

    /// `func_8084C760`: in a crawlspace. `tunnel_start` plays out first, its root motion moving
    /// Link in; then the stick's forward tilt is the speed (`rel.stick_y` × 0.03, backwards
    /// too) along `currentYaw`, and `Camera_Subj4` keeps Link on the crawlspace's line. At
    /// either end's wall, the way out (`func_8083F570`). The crawl's sounds: `D_808548B4`
    /// (`func_80832924`).
    fn func_8084C760(&mut self, env: &Env) {
        self.state2 |= STATE2_6;
        if self.skel.update(env.data) {
            if self.state1 & STATE1_0 == 0 {
                if self.skel.move_flags != 0 {
                    self.skel.move_flags = 0;
                    return;
                }
                if !self.func_8083F570(env) {
                    self.linear_velocity = self.input.rel.stick_y as f32 * 0.03;
                }
            }
            return;
        }
        self.func_80832924(env.audio.player_anim_sfx("D_808548B4"));
    }

    /// `func_8083F570`: crawling into a crawlspace's wall (`WALL_FLAG_4`, `_5`) head first
    /// (backwards, feet first): out, with `tunnel_end` facing away from the wall, or
    /// `tunnel_start` played backwards, each with its one-point cutscene (9601, 9602: the
    /// camera's spline up and out of the crawlspace).
    fn func_8083F570(&mut self, env: &Env) -> bool {
        let data = env.data;
        if self.linear_velocity != 0.0 && self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && self.s.wall_flags & (WALL_FLAG_4 | WALL_FLAG_5) != 0 {
            let mut temp = self.actor.shape_rot.y.wrapping_sub(self.actor.wall_yaw);
            if self.linear_velocity < 0.0 {
                temp = temp.wrapping_add(i16::MIN);
            }
            if abs16(temp) > 0x4000 {
                self.setup_action(data, Action::CrawlExit, 0);
                if self.linear_velocity > 0.0 {
                    self.actor.shape_rot.y = self.actor.wall_yaw.wrapping_add(i16::MIN);
                    self.skel.play_once(data, data.anim("link_child_tunnel_end"));
                    self.func_80832F54(0x9D);
                    self.play_requests.push(PlayRequest::OnePointCutscene { cs_id: 9601, timer: 999, player: false, parent: CAM_ID_MAIN });
                } else {
                    self.actor.shape_rot.y = self.actor.wall_yaw;
                    self.play_backwards(data, data.anim("link_child_tunnel_start"));
                    self.func_80832F54(0x9D);
                    self.play_requests.push(PlayRequest::OnePointCutscene { cs_id: 9602, timer: 999, player: false, parent: CAM_ID_MAIN });
                }
                self.current_yaw = self.actor.shape_rot.y;
                self.func_80832210();
                return true;
            }
        }
        false
    }

    /// `func_8084C81C`: out of the crawlspace; standing once the animation ends
    /// (`func_8083C0E8`). Its sounds: `D_808548D8`.
    fn func_8084C81C(&mut self, env: &Env) {
        self.state2 |= STATE2_6;
        if self.skel.update(env.data) {
            self.func_8083C0E8(env.data);
            self.state2 &= !STATE2_18;
            return;
        }
        self.func_80832924(env.audio.player_anim_sfx("D_808548D8"));
    }

    // ================================================================================
    // Damage (func_808382DC) and the knockdown

    /// `Player_InBlockingCsMode` (magic isn't ported).
    fn in_blocking_cs_mode(&self, env: &Env) -> bool {
        self.state1 & (STATE1_7 | STATE1_29) != 0
            || self.cs_mode != 0
            || env.io.borrow().transition.trigger == TRANS_TRIGGER_START
            || self.state1 & STATE1_0 != 0
            || self.state3 & STATE3_7 != 0
    }

    /// `func_80837AE0`: `timer` frames of invincibility, unless the roll's is running.
    fn func_80837AE0(&mut self, timer: i8) {
        if self.invincibility_timer >= 0 {
            self.invincibility_timer = timer;
            self.unk_88F = 0;
        }
    }

    /// `func_80837AFC`: at most `timer`.
    fn func_80837AFC(&mut self, timer: i8) {
        if self.invincibility_timer > timer {
            self.invincibility_timer = timer;
        }
        self.unk_88F = 0;
    }

    /// `func_80837B18`: `Health_ChangeBy(damage)` unless invincible; false once Link is out of
    /// health.
    fn func_80837B18(&mut self, env: &Env, damage: i32) -> bool {
        if self.invincibility_timer != 0 || self.actor.category != ACTORCAT_PLAYER {
            return true;
        }
        // Health_ChangeBy's recovery sound (for a gain), through Player's requests.
        if damage > 0 {
            self.sfx(PlayerSfx::NoPos(NA_SE_SY_HP_RECOVER));
        }
        oot_game::item::health_change_by(&mut env.io.borrow_mut().save, None, damage as i16)
    }

    /// `Player_InflictDamage`: true when it took the last of Link's health.
    fn player_inflict_damage(&mut self, env: &Env, damage: i32) -> bool {
        if !self.in_blocking_cs_mode(env) && !self.func_80837B18(env, damage) {
            self.state2 &= !STATE2_7;
            return true;
        }
        false
    }

    /// `func_808382BC`: at least 20 frames of invincibility.
    fn func_808382BC(&mut self) {
        if self.invincibility_timer >= 0 && self.invincibility_timer < 20 {
            self.invincibility_timer = 20;
        }
    }

    /// `func_808382DC`: what hurts Link this frame; true when something did (then there's no
    /// ledge check, `func_8083AA10`):
    /// - being crushed, a void floor (`FLOOR_TYPE_9`) or `PLAYER_STATE2_31`: the respawn or
    ///   the void-out;
    /// - the knockback an actor asked for (`unk_8A1`, `func_8002F698`), even while invincible
    ///   for kind 2 and up;
    /// - a hit on the body cylinder (`AC_HIT`);
    /// - a hurting wall or floor (`func_80042108`, the hot floors `FLOOR_TYPE_2`, `_3`).
    ///
    /// `unk_A86`'s damage is never set, and the shield's bounce needs shielding, which isn't
    /// ported (the shield's quad is never registered, so `AC_BOUNCED` is never set). The
    /// burning (`func_8083821C`), the shock timer, the rumble and the sounds aren't ported.
    fn func_808382DC(&mut self, env: &Env) -> bool {
        use oot_game::actor::BGCHECKFLAG_CRUSHED;
        if self.unk_A86 != 0 {
            if !self.in_blocking_cs_mode(env) {
                self.player_inflict_damage(env, -16);
                self.unk_A86 = 0;
            }
            return true;
        }
        // Player_GetHeight (not riding).
        let height = if self.adult { 68.0 } else { 44.0 };
        let sp68 = (height - 8.0) < self.unk_6C4 * self.actor.scale.y;
        if sp68 || self.actor.bg_check_flags & BGCHECKFLAG_CRUSHED != 0 || self.s.floor_type == FLOOR_TYPE_9 || self.state2 & STATE2_31 != 0 {
            self.func_80832698(NA_SE_VO_LI_DAMAGE_S);
            let mut io = env.io.borrow_mut();
            if sp68 {
                let (pos, yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
                io.trigger_respawn(pos, yaw);
                io.set_transition_for_next_entrance(env.entrances);
            } else {
                // The Forest Temple's checkerboard ceiling room and the Shadow Temple's falling
                // spikes respawn Link at a set place.
                let special = match (io.scene_id, io.room) {
                    (SCENE_BMORI1, 15) => Some(Vec3::new(1992.0, 403.0, -3432.0)),
                    (SCENE_HAKADAN, 10) => Some(Vec3::new(1200.0, -1343.0, 3850.0)),
                    _ => None,
                };
                if let Some(pos) = special {
                    let (p, yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
                    io.setup_respawn_point(RESPAWN_MODE_DOWN, 0xDFF, p, yaw);
                    io.save.respawn[RESPAWN_MODE_DOWN].pos = pos;
                    io.save.respawn[RESPAWN_MODE_DOWN].yaw = 0;
                }
                io.trigger_void_out();
            }
            drop(io);
            self.func_80832698(NA_SE_VO_LI_TAKEN_AWAY);
            // (play->unk_11DE9 = true: not modelled.)
            self.sfx(PlayerSfx::NoPos(NA_SE_OC_ABYSS));
            return true;
        }
        if self.unk_8A1 != 0 && (self.unk_8A1 >= 2 || self.invincibility_timer == 0) {
            const SP5C: [i32; 3] = [2, 1, 1];
            // func_80838280: acHitEffect 1 sets Link burning (func_8083821C, not ported).
            if self.unk_8A1 == 3 {
                // this->shockTimer = 40 (func_80848B44, not ported).
                self.note("shock (shockTimer) not ported");
            }
            self.actor.col_chk_info.damage = self.actor.col_chk_info.damage.wrapping_add(self.unk_8A0);
            let (kind, speed, vy, yaw) = (SP5C[self.unk_8A1 as usize - 1], self.unk_8A4, self.unk_8A8, self.unk_8A2);
            self.func_80837C0C(env, kind, speed, vy, yaw, 20);
            return true;
        }
        // sp64: the shield's AC_BOUNCED (never set), or the roll's block. @bug (game): that one
        // tests the attacking collider's u8 atFlags against 0x20000000, so it never holds.
        if self.unk_A87 != 0
            || self.invincibility_timer > 0
            || self.state1 & STATE1_26 != 0
            || self.cs_mode != 0
            || self.melee_weapon_quads[0].base.at_flags & cc::AT_HIT != 0
            || self.melee_weapon_quads[1].base.at_flags & cc::AT_HIT != 0
        {
            return false;
        }
        if self.cylinder.base.ac_flags & cc::AC_HIT != 0 {
            if self.cylinder.base.ac.and_then(|h| env.target(h)).is_some_and(|a| a.flags & ACTOR_FLAG_24 != 0) {
                self.func_8002F7DC(NA_SE_PL_BODY_HIT);
            }
            let ac_pos = self.cylinder.base.ac.and_then(|h| env.target(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
            let effect = self.actor.col_chk_info.ac_hit_effect;
            let sp4c = if self.state1 & STATE1_27 != 0 {
                0
            } else {
                match effect {
                    2 => 3,
                    3 => 4,
                    4 => 1,
                    // func_80838280.
                    _ => 0,
                }
            };
            // Actor_WorldYawTowardActor(ac, &this->actor).
            let yaw = vec3f_yaw(ac_pos, self.actor.world_pos);
            self.func_80837C0C(env, sp4c, 4.0, 5.0, yaw, 20);
            return true;
        }
        if self.invincibility_timer != 0 {
            return false;
        }
        // D_808544F4: in the Goron tunic, the frames on a hot floor (FLOOR_TYPE_2, _3) before it
        // hurts; without it, it hurts at once.
        const D_808544F4: [u8; 2] = [120, 60];
        let sp48 = self.s.floor_type.wrapping_sub(FLOOR_TYPE_2);
        let sp48 = (sp48 <= FLOOR_TYPE_3 - FLOOR_TYPE_2).then_some(sp48 as usize);
        let col = env.col;
        let wall_hurts = self.actor.wall_poly.is_some_and(|w| col.flag27(w));
        let floor_hurts = self.actor.floor_poly.is_some_and(|f| col.flag27(f));
        // PLAYER_TUNIC_GORON is 1.
        if wall_hurts
            || sp48.is_some_and(|i| floor_hurts && self.unk_A79 >= D_808544F4[i])
            || sp48.is_some_and(|i| self.current_tunic != 1 || self.unk_A79 >= D_808544F4[i])
        {
            self.unk_A79 = 0;
            self.actor.col_chk_info.damage = 4;
            let yaw = self.actor.shape_rot.y;
            self.func_80837C0C(env, 0, 4.0, 5.0, yaw, 20);
            return true;
        }
        false
    }

    /// `D_808544B0`: the staggers, by (strong hit) × 4 + (from behind) × 2 + (locked on).
    const D_808544B0: [&'static str; 8] = [
        "link_normal_front_shit",
        "link_normal_front_shitR",
        "link_normal_back_shit",
        "link_normal_back_shitR",
        "link_normal_front_hit",
        "link_anchor_front_hitR",
        "link_normal_back_hit",
        "link_anchor_back_hitR",
    ];

    /// `func_80837C0C`: hurt by `colChkInfo.damage`, `arg6` frames of invincibility, and the
    /// reaction: knocked down (`func_8084377C`) for kinds 1 and 2, in the air, hanging or
    /// climbing, at `arg3` and `arg4` from `arg5` (kind 2 with its own); otherwise a stagger
    /// (`func_8084370C`), or only a flinch when running fast (`unk_890`). Not ported: kind 3
    /// (frozen, `func_8084FB10`), kind 4 (the electric shock, `func_8084FBF4`) and the hit
    /// while swimming (`func_8084E30C`), which only take the damage here; the rumble; the
    /// sounds.
    fn func_80837C0C(&mut self, env: &Env, arg2: i32, arg3: f32, arg4: f32, mut arg5: i16, arg6: i8) {
        let data = env.data;
        let mut sp2c = None;
        if self.state1 & STATE1_13 != 0 {
            self.func_80837B60();
        }
        self.unk_890 = 0;
        self.func_8002F7DC(NA_SE_PL_DAMAGE);
        let damage = self.actor.col_chk_info.damage as i32;
        if !self.func_80837B18(env, -damage) {
            self.state2 &= !STATE2_7;
            if !self.grounded() && self.state1 & STATE1_27 == 0 {
                self.func_80837B9C(data);
            }
            return;
        }
        self.func_80837AE0(arg6);
        if arg2 == 3 || arg2 == 4 {
            self.note(format!("func_80837C0C kind {arg2} (frozen, shocked) not ported"));
            return;
        }
        arg5 = arg5.wrapping_sub(self.actor.shape_rot.y);
        if self.state1 & STATE1_27 != 0 {
            self.note("hit while swimming (func_8084E30C) not ported");
            return;
        } else if arg2 == 1 || arg2 == 2 || !self.grounded() || self.state1 & (STATE1_13 | STATE1_14 | STATE1_21) != 0 {
            self.setup_action(data, Action::KnockedDown, 0);
            self.state3 |= STATE3_1;
            self.func_80832224();
            if arg2 == 2 {
                self.unk_850 = 4;
                self.actor.speed_xz = 3.0;
                self.linear_velocity = 3.0;
                self.actor.velocity.y = 6.0;
                // func_80832C2C.
                let a = self.anim(data, group::DAMAGE_RUN);
                self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_ONCE, 0.0);
                self.func_80832698(NA_SE_VO_LI_DAMAGE_S);
            } else {
                self.actor.speed_xz = arg3;
                self.linear_velocity = arg3;
                self.actor.velocity.y = arg4;
                sp2c = Some(if abs16(arg5) > 0x4000 { data.anim("link_normal_front_downA") } else { data.anim("link_normal_back_downA") });
                // (NA_SE_VO_BL_DOWN for a dead non-Player: Link is the Player.)
                self.func_80832698(NA_SE_VO_LI_FALL_L);
            }
            self.hover_boots_timer = 0;
            self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND;
        } else {
            if self.linear_velocity > 4.0 && self.state1 & STATE1_4 == 0 {
                self.unk_890 = 20;
                self.func_80832698(NA_SE_VO_LI_DAMAGE_S);
                return;
            }
            self.setup_action(data, Action::Damaged, 0);
            self.func_80833C3C();
            let mut i = 0;
            if self.actor.col_chk_info.damage >= 5 {
                self.linear_velocity = 23.0;
                i += 4;
            }
            if abs16(arg5) <= 0x4000 {
                i += 2;
            }
            if self.state1 & STATE1_4 != 0 {
                i += 1;
            }
            sp2c = Some(data.anim(Self::D_808544B0[i]));
            self.func_80832698(NA_SE_VO_LI_DAMAGE_S);
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(arg5);
        self.current_yaw = self.actor.shape_rot.y;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        if abs16(arg5) > 0x4000 {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(i16::MIN);
        }
        // func_80832564: func_80832440, and func_808323B4 (Link holds nothing here).
        self.func_80832440();
        self.state1 |= STATE1_26;
        if let Some(a) = sp2c {
            // func_808322D0.
            self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
        }
    }

    /// `func_8084370C`: staggering; standing again at the end (`func_80839F90`).
    fn func_8084370C(&mut self, env: &Env) {
        self.func_8083721C();
        let sp1c = self.func_808374A0(env, 16.0);
        if sp1c != 0 && (self.skel.update(env.data) || sp1c > 0) {
            self.func_80839F90(env.data);
        }
    }

    /// `func_8084377C`: knocked down. While another knockback comes in the air (`unk_8A1`),
    /// it takes its yaw and speed. On the ground at the animation's end: after kind 2's
    /// four frames, standing (`func_80853080`); else, once nothing hits any more, lying down
    /// (`func_80843954`) with `front_downB` or `back_downB`; the bounce's sound on touching down.
    fn func_8084377C(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5 | STATE2_6;
        self.func_808382BC();
        if self.state1 & STATE1_29 == 0 && self.unk_850 == 0 && self.unk_8A1 != 0 {
            let temp = self.actor.shape_rot.y.wrapping_sub(self.unk_8A2);
            self.actor.shape_rot.y = self.unk_8A2;
            self.current_yaw = self.unk_8A2;
            self.linear_velocity = self.unk_8A4;
            if abs16(temp) > 0x4000 {
                self.actor.shape_rot.y = self.unk_8A2.wrapping_add(i16::MIN);
            }
            if self.actor.velocity.y < 0.0 {
                self.actor.gravity = 0.0;
                self.actor.velocity.y = 0.0;
            }
        }
        if self.skel.update(data) && self.grounded() {
            if self.unk_850 != 0 {
                self.unk_850 -= 1;
                if self.unk_850 == 0 {
                    self.func_80853080(data);
                }
            } else if self.state1 & STATE1_29 != 0 || (self.cylinder.base.ac_flags & cc::AC_HIT == 0 && self.unk_8A1 == 0) {
                if self.state1 & STATE1_29 != 0 {
                    self.unk_850 += 1;
                } else {
                    self.setup_action(data, Action::Down, 0);
                    self.state1 |= STATE1_26;
                }
                let a = if self.current_yaw != self.actor.shape_rot.y { data.anim("link_normal_front_downB") } else { data.anim("link_normal_back_downB") };
                self.skel.play_once(data, a);
                self.func_80832698(NA_SE_VO_LI_FREEZE);
            }
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            self.func_80832770(NA_SE_PL_BOUND);
        }
    }

    /// `func_80843954`: lying down until the slide stops, then getting up (`func_80843A38`)
    /// with `front_down_wake` or `back_down_wake`.
    fn func_80843954(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5 | STATE2_6;
        self.func_808382BC();
        self.func_8083721C();
        if self.skel.update(data) && self.linear_velocity == 0.0 {
            if self.state1 & STATE1_29 != 0 {
                self.unk_850 += 1;
            } else {
                self.setup_action(data, Action::GetUp, 0);
                self.state1 |= STATE1_26;
            }
            let a = if self.current_yaw != self.actor.shape_rot.y { data.anim("link_normal_front_down_wake") } else { data.anim("link_normal_back_down_wake") };
            self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
            self.current_yaw = self.actor.shape_rot.y;
        }
    }

    /// `func_80843A38`: getting up; standing at the end (`func_80839F90`), with
    /// `D_808545DC`'s sounds.
    fn func_80843A38(&mut self, env: &Env) {
        self.state2 |= STATE2_5;
        self.func_808382BC();
        if self.state1 & STATE1_29 != 0 {
            self.skel.update(env.data);
        } else {
            let sp24 = self.func_808374A0(env, 16.0);
            if sp24 != 0 && (self.skel.update(env.data) || sp24 > 0) {
                self.func_80839F90(env.data);
            }
        }
        self.func_80832924(env.audio.player_anim_sfx("D_808545DC"));
    }

    /// `func_8083EC18`: onto the wall in front if it's 79 tall and climbable: vines and
    /// climbable walls (`WALL_FLAG_3`) from where Link is, a ladder (`WALL_FLAG_1`) lined up to
    /// its rungs, or a ladder's top (`WALL_FLAG_2`) turning round to climb down.
    fn func_8083EC18(&mut self, env: &Env, arg2: u32) -> bool {
        let data = env.data;
        let col = env.col;
        if self.wall_height < 79.0 {
            return false;
        }
        // Kokiri boots.
        if !(self.state1 & STATE1_27 == 0 || self.actor.y_dist_to_water < self.age.unk_2C) {
            return false;
        }
        let Some(wall) = self.actor.wall_poly else { return false };
        let sp8c: i8 = if arg2 & WALL_FLAG_3 != 0 { 2 } else { 0 };
        if !(sp8c != 0 || arg2 & WALL_FLAG_1 != 0 || col.wall_flags(wall) & WALL_FLAG_2 != 0) {
            return false;
        }
        let (mut phi_f20, mut phi_f12) = (0.0f32, 0.0f32);
        let (sp80, sp7c);
        if sp8c != 0 {
            sp80 = self.actor.world_pos.x;
            sp7c = self.actor.world_pos.z;
        } else {
            // The wall triangle's horizontal middle, and its lowest point.
            let v = col.poly_vertices(wall);
            let (mut x0, mut x1, mut z0, mut z1) = (v[0].x, v[0].x, v[0].z, v[0].z);
            phi_f20 = v[0].y;
            for p in &v[1..] {
                if x0 > p.x {
                    x0 = p.x;
                } else if x1 < p.x {
                    x1 = p.x;
                }
                if z0 > p.z {
                    z0 = p.z;
                } else if z1 < p.z {
                    z1 = p.z;
                }
                if phi_f20 > p.y {
                    phi_f20 = p.y;
                }
            }
            sp80 = (x0 + x1) * 0.5;
            sp7c = (z0 + z1) * 0.5;
            let n = col.poly_normal(wall);
            phi_f12 = ((self.actor.world_pos.x - sp80) * n.z) - ((self.actor.world_pos.z - sp7c) * n.x);
            let sp48 = self.actor.world_pos.y - phi_f20;
            // The height to the next rung, 15 apart, in double precision as written.
            const RUNG: f64 = 15.000000223517418;
            phi_f20 = ((((sp48 as f64 / RUNG) + 0.5) as i32 as f32) as f64 * RUNG - sp48 as f64) as f32;
            phi_f12 = phi_f12.abs();
        }
        if phi_f12 >= 8.0 {
            return false;
        }
        let n = col.poly_normal(wall);
        let mut sp34 = self.wall_distance;
        self.func_80836898(data, env, A74::ClimbStart);
        self.state1 |= STATE1_21;
        self.state1 &= !STATE1_27;
        let anim;
        if sp8c != 0 || arg2 & WALL_FLAG_1 != 0 {
            self.unk_84F = sp8c;
            if sp8c != 0 {
                anim = if self.grounded() { data.anim("link_normal_Fclimb_startA") } else { data.anim("link_normal_Fclimb_hold2upL") };
                sp34 = (self.age.wall_radius - 1.0) - sp34;
            } else {
                anim = self.age.climb.unk_A4;
                sp34 -= 1.0;
            }
            self.unk_850 = -2;
            self.actor.world_pos.y += phi_f20;
            self.current_yaw = self.actor.wall_yaw.wrapping_add(i16::MIN);
            self.actor.shape_rot.y = self.current_yaw;
        } else {
            anim = self.age.climb.unk_A8;
            self.unk_850 = -4;
            self.current_yaw = self.actor.wall_yaw;
            self.actor.shape_rot.y = self.current_yaw;
        }
        self.actor.world_pos.x = (sp34 * n.x) + sp80;
        self.actor.world_pos.z = (sp34 * n.z) + sp7c;
        self.func_80832224();
        self.actor.prev_pos = self.actor.world_pos;
        self.skel.play_once(data, anim);
        self.func_80832F54(0x9F);
        true
    }

    /// `func_8083F360`: keep to the wall: a line from `arg4` to `arg3` ahead at height `arg1`;
    /// on the wall, face it and stand `arg2` off it. True while there's a wall.
    fn func_8083F360(&mut self, env: &Env, arg1: f32, arg2: f32, arg3: f32, arg4: f32) -> bool {
        let (c, s) = (cos_s(self.actor.shape_rot.y), sin_s(self.actor.shape_rot.y));
        let y = self.actor.world_pos.y + arg1;
        let a = Vec3::new(self.actor.world_pos.x + arg4 * s, y, self.actor.world_pos.z + arg4 * c);
        let b = Vec3::new(self.actor.world_pos.x + arg3 * s, y, self.actor.world_pos.z + arg3 * c);
        let hit = env.col.entity_line_test(a, b, true, false, false, true);
        self.actor.wall_poly = hit.map(|h| h.1);
        if let Some((p, poly)) = hit {
            self.actor.bg_check_flags |= BGCHECKFLAG_PLAYER_WALL_INTERACT;
            self.s.wall_flags = env.col.wall_flags(poly);
            let n = env.col.poly_normal(poly);
            let t = atan2_s(-n.z, -n.x);
            scaled_step_to_s(&mut self.actor.shape_rot.y, t, 800);
            self.current_yaw = self.actor.shape_rot.y;
            self.actor.world_pos.x = p.x - sin_s(self.actor.shape_rot.y) * arg2;
            self.actor.world_pos.z = p.z - cos_s(self.actor.shape_rot.y) * arg2;
            return true;
        }
        self.actor.bg_check_flags &= !BGCHECKFLAG_PLAYER_WALL_INTERACT;
        false
    }

    /// `func_8083FBC0`: let go on A, or when the wall isn't climbable any more.
    fn func_8083FBC0(&mut self, env: &Env) -> bool {
        let f = self.s.wall_flags;
        let flag2 = self.actor.wall_poly.is_some_and(|w| env.col.wall_flags(w) & WALL_FLAG_2 != 0);
        if !self.input.press.held(BTN_A) && self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0 && (f & WALL_FLAG_3 != 0 || f & WALL_FLAG_1 != 0 || flag2) {
            return false;
        }
        self.func_8083FB7C(env.data);
        self.func_80832698(NA_SE_VO_LI_AUTO_JUMP);
        true
    }

    /// `func_8083FB7C`: off the wall, falling.
    fn func_8083FB7C(&mut self, data: &GameData) {
        self.state1 &= !(STATE1_21 | STATE1_27);
        self.func_80837B9C(data);
        self.linear_velocity = -0.4;
    }

    /// `func_8083973C`: the floor below a point `off` ahead of Link (`func_808395DC`,
    /// `BgCheck_EntityRaycastDown3`).
    fn func_8083973C(&self, env: &Env, off: Vec3) -> f32 {
        let p = self.func_808395DC(self.actor.world_pos, off);
        env.col.entity_raycast_down(p).0
    }

    /// `func_8083F070`: step off the ladder (`func_8084C5F8`).
    fn func_8083F070(&mut self, data: &GameData, anim: AnimId) {
        self.func_80835DAC(data, Action::ClimbEnd, 0);
        self.skel.play_once_set_speed(data, anim, 4.0 / 3.0);
    }

    /// `LinkAnimation_Change(anim, -1, lastFrame, 0, ANIMMODE_ONCE, 0)`: an animation played
    /// backwards (climbing down).
    fn play_backwards(&mut self, data: &GameData, anim: AnimId) {
        let last = data.anims[anim].last_frame();
        self.skel.change(data, anim, -1.0, last, 0.0, ANIMMODE_ONCE, 0.0);
    }

    /// `func_8084BF1C`: climbing. Up and down one rung per animation (sideways on vines), at a
    /// speed from the stick; off the top onto the ledge (vines) or the ladder's top step, off
    /// the bottom near the floor.
    fn func_8084BF1C(&mut self, env: &Env) {
        let data = env.data;
        let mut sp84 = self.input.rel.stick_y as i32;
        let mut sp80 = self.input.rel.stick_x as i32;
        self.fall_start_height = self.actor.world_pos.y as i32 as i16;
        self.state2 |= STATE2_6;
        let mut phi_f0 = if self.unk_84F != 0 && sp84.abs() < sp80.abs() {
            sp84 = 0;
            sp80.abs() as f32 * 0.0325
        } else {
            sp80 = 0;
            sp84.abs() as f32 * 0.05
        };
        phi_f0 = phi_f0.clamp(1.0, 3.35);
        let phi_f2 = if self.skel.play_speed >= 0.0 { 1.0 } else { -1.0 };
        self.skel.play_speed = phi_f2 * phi_f0;
        if self.unk_850 >= 0 {
            // (A DynaPoly wall carries Link along: no climbable dyna walls are ported.)
            self.actor.update_bg_check_info(env.col, 26.0, 6.0, self.age.ceiling_check_height, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2);
            let r = self.age.unk_3C;
            self.func_8083F360(env, 26.0, r, 50.0, -20.0);
        }
        if (self.unk_850 < 0 || !self.func_8083FBC0(env)) && self.skel.update(data) {
            if self.unk_850 < 0 {
                self.unk_850 = (self.unk_850 as i32).abs() as i16 & 1;
                return;
            }
            let c = self.age.climb;
            if sp84 != 0 {
                let mut sp68 = (self.unk_84F as i32 + self.unk_850 as i32) as usize;
                if sp84 > 0 {
                    // D_8085488C = { 0, unk_40, 26 }: the ledge above.
                    let temp_f0 = self.func_8083973C(env, Vec3::new(0.0, self.age.unk_40, 26.0));
                    if self.actor.world_pos.y < temp_f0 {
                        if self.unk_84F != 0 {
                            self.actor.world_pos.y = temp_f0;
                            self.state1 &= !STATE1_21;
                            if let Some(w) = self.actor.wall_poly {
                                let r = self.age.unk_3C;
                                self.func_8083A5C4(data, env, w, r, data.anim("link_normal_jump_climb_up_free"));
                            }
                            self.current_yaw = self.current_yaw.wrapping_add(i16::MIN);
                            self.actor.shape_rot.y = self.current_yaw;
                            self.func_8083A9B8(data, data.anim("link_normal_jump_climb_up_free"));
                            self.state1 |= STATE1_14;
                        } else {
                            let a = c.unk_CC[self.unk_850 as usize & 1];
                            self.func_8083F070(data, a);
                        }
                    } else {
                        self.skel.prev_transl = c.unk_4A[sp68.min(3)];
                        self.skel.play_once(data, c.unk_AC[sp68.min(3)]);
                    }
                } else if (self.actor.world_pos.y - self.actor.floor_height) < 15.0 {
                    if self.unk_84F != 0 {
                        self.func_8083FB7C(data);
                    } else {
                        if self.unk_850 != 0 {
                            self.skel.prev_transl = c.unk_44;
                        }
                        let a = c.unk_C4[self.unk_850 as usize & 1];
                        self.func_8083F070(data, a);
                        self.unk_850 = 1;
                    }
                } else {
                    sp68 ^= 1;
                    self.skel.prev_transl = c.unk_62[sp68.min(3)];
                    let a1 = c.unk_AC[sp68.min(3)];
                    self.play_backwards(data, a1);
                }
                self.unk_850 ^= 1;
            } else if self.unk_84F != 0 && sp80 != 0 {
                let a2 = c.unk_BC[self.unk_850 as usize & 1];
                if sp80 > 0 {
                    self.skel.prev_transl = c.unk_7A[self.unk_850 as usize & 1];
                    self.skel.play_once(data, a2);
                } else {
                    self.skel.prev_transl = c.unk_86[self.unk_850 as usize & 1];
                    self.play_backwards(data, a2);
                }
            } else {
                self.state2 |= STATE2_12;
            }
            return;
        }
        if self.unk_850 < 0 {
            let f = |p: &Self, frames: &[f32]| frames.iter().any(|&fr| p.skel.on_frame(fr));
            if (self.unk_850 == -2 && f(self, &[14.0, 29.0])) || (self.unk_850 == -4 && f(self, &[22.0, 35.0, 49.0, 55.0])) {
                self.func_8084BEE4();
            }
            return;
        }
        if self.skel.on_frame(if self.skel.play_speed > 0.0 { 20.0 } else { 0.0 }) {
            self.func_8084BEE4();
        }
    }

    /// `func_8084BEE4`: a hand or foot on the ladder, or on the wall.
    fn func_8084BEE4(&mut self) {
        self.func_8002F7DC(if self.unk_84F != 0 { NA_SE_PL_WALK_WALL } else { NA_SE_PL_WALK_LADDER });
    }

    /// `func_8084C5F8`: the step off a ladder, standing at its end.
    fn func_8084C5F8(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_6;
        let temp = self.func_808374A0(env, 4.0);
        if temp == 0 {
            self.state1 &= !STATE1_21;
            return;
        }
        if temp > 0 || self.skel.update(data) {
            self.func_8083C0E8(data);
            self.state1 &= !STATE1_21;
            return;
        }
        // D_80854898, D_808548A0: the steps' frames.
        let mut sp38 = [10.0, 20.0];
        if self.unk_850 != 0 {
            self.func_80832924(env.audio.player_anim_sfx("D_808548A8"));
            sp38 = [40.0, 50.0];
        }
        if self.skel.on_frame(sp38[0]) || self.skel.on_frame(sp38[1]) {
            let p = self.actor.world_pos + Vec3::new(0.0, 20.0, 0.0);
            let (h, poly) = env.col.entity_raycast_down(p);
            if h != 0.0 {
                // SurfaceType_GetSfxType: the floor's type, not its sound's id.
                self.unk_89E = poly.map(|g| env.col.sfx_type(g) as u16).unwrap_or(0);
                self.func_808328A0();
            }
        }
    }

    /// `func_80845668`: stepping up onto a ledge (100/150 step-up animations, the model
    /// offset `shape.yOffset` easing back to 0), or the tall-ledge jump that ends in a mid-air grab.
    fn func_80845668(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        let sp3c = self.skel.update(data);
        if self.skel.animation == data.anim("link_normal_250jump_start") {
            self.linear_velocity = 1.0;
            if self.skel.on_frame(8.0) {
                let mut t = self.wall_height.min(self.age.wall_probe_height);
                t *= if self.state1 & STATE1_27 != 0 { 0.085 } else { 0.072 };
                if !self.adult {
                    t += 1.0;
                }
                self.func_80838940(data, None, t, NA_SE_VO_LI_AUTO_JUMP);
                self.unk_850 = -1;
            }
        } else {
            let temp2 = self.func_808374A0(env, 4.0);
            if temp2 == 0 {
                self.state1 &= !(STATE1_14 | STATE1_18);
                return;
            }
            if sp3c || temp2 > 0 {
                self.func_8083C0E8(data);
                self.state1 &= !(STATE1_14 | STATE1_18);
                return;
            }
            let a = self.skel.animation;
            let temp3 = if a == data.anim("link_swimer_swim_15step_up") {
                if self.skel.on_frame(30.0) {
                    self.func_8083D0A8(env, 10.0);
                }
                50.0
            } else if a == data.anim("link_normal_150step_up") {
                30.0
            } else if a == data.anim("link_normal_100step_up") {
                16.0
            } else {
                0.0
            };
            if self.skel.on_frame(temp3) {
                self.func_808328A0();
                self.func_80832698(NA_SE_VO_LI_CLIMB_END);
            }
            if a == data.anim("link_normal_100step_up") || self.skel.cur_frame > 5.0 {
                if self.unk_850 == 0 {
                    self.func_80832854();
                    self.unk_850 = 1;
                }
                step_to_f(&mut self.actor.shape_y_offset, 0.0, 150.0);
            }
        }
    }

    /// `func_8083A4A8`: the automatic jump off a ledge.
    fn func_8083A4A8(&mut self, data: &GameData) {
        let yaw_diff = self.current_yaw.wrapping_sub(self.actor.shape_rot.y);
        let anim = if abs16(yaw_diff) < 0x1000 && self.linear_velocity > 4.0 {
            data.anim("link_normal_run_jump")
        } else {
            data.anim("link_normal_jump")
        };
        let r = &self.regs;
        let vy = if self.linear_velocity > r.ireg(66) as f32 / 100.0 {
            r.ireg(67) as f32 / 100.0
        } else {
            (r.ireg(68) as f32 / 100.0) + ((r.ireg(69) as f32 * self.linear_velocity) / 1000.0)
        };
        self.func_80838940(data, Some(anim), vy, NA_SE_VO_LI_AUTO_JUMP);
        self.unk_850 = 1;
    }

    /// `func_80838940`: start a jump with vertical speed `vy`: the jump's sound and Link's
    /// voice `sfx_id`.
    fn func_80838940(&mut self, data: &GameData, anim: Option<AnimId>, vy: f32, sfx_id: u16) {
        self.setup_action(data, Action::Midair, 1);
        if let Some(a) = anim {
            self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
        }
        self.actor.velocity.y = vy * self.s.speed_scale;
        self.hover_boots_timer = 0;
        self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND;
        self.func_80832854();
        self.func_80832698(sfx_id);
        self.state1 |= STATE1_18;
    }

    // ================================================================================
    // Action setup and shared helpers

    /// `func_80835C58`: switch action function.
    fn setup_action(&mut self, _data: &GameData, action: Action, flags: i32) -> bool {
        if self.action == action {
            return false;
        }
        self.action = action;
        if flags & 1 == 0 && self.state1 & STATE1_11 == 0 {
            // func_80834644: nothing held.
            self.state1 &= !STATE1_22;
        }
        self.func_80832DBC();
        self.state1 &= !(STATE1_2 | STATE1_6 | STATE1_26 | STATE1_28 | STATE1_29 | STATE1_31);
        self.state2 &= !(STATE2_19 | STATE2_27 | STATE2_28);
        self.state3 &= !(STATE3_1 | STATE3_3 | STATE3_7);
        self.unk_84F = 0;
        self.unk_850 = 0;
        self.unk_6AC = 0;
        true
    }

    /// `func_80832210`.
    fn func_80832210(&mut self) {
        self.actor.speed_xz = 0.0;
        self.linear_velocity = 0.0;
    }

    /// `func_808322FC`: fold the root limb's yaw into the facing.
    fn func_808322FC(&mut self) {
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.skel.joint[1][1]);
        self.skel.joint[1][1] = 0;
    }

    /// `func_80832CFC`.
    fn func_80832CFC(&mut self) {
        self.skel.prev_transl = self.skel.base_transl;
        self.skel.prev_rot = self.actor.shape_rot.y;
    }

    /// `func_80832DBC`: end animation-driven movement.
    fn func_80832DBC(&mut self) {
        if self.skel.move_flags != 0 {
            self.func_808322FC();
            self.skel.joint[0][0] = self.skel.base_transl[0];
            self.skel.joint[0][2] = self.skel.base_transl[2];
            if self.skel.move_flags & 8 != 0 {
                if self.skel.move_flags & 2 != 0 {
                    self.skel.joint[0][1] = self.skel.prev_transl[1];
                }
            } else {
                self.skel.joint[0][1] = self.skel.base_transl[1];
            }
            self.func_80832CFC();
            self.skel.move_flags = 0;
        }
    }

    /// `func_80832440`: drop interaction state when an action is interrupted.
    fn func_80832440(&mut self) {
        self.melee_weapon_state = 0;
        self.unk_6AD = 0;
        self.state1 &= !(STATE1_13 | STATE1_14 | STATE1_20 | STATE1_21);
        self.state2 &= !((1 << 4) | (1 << 7) | STATE2_18);
        self.actor.shape_rot.x = 0;
    }

    /// `func_80833C3C`.
    fn func_80833C3C(&mut self) {
        self.unk_870 = 0.0;
        self.unk_874 = 0.0;
    }

    /// `func_8083721C`: decelerate by `REG(43) / 100`.
    fn func_8083721C(&mut self) -> bool {
        step_to_f(&mut self.linear_velocity, 0.0, self.regs.reg(43) as f32 / 100.0)
    }

    /// `func_80836FAC`: target speed and yaw from the stick. `arg4` selects the curve used
    /// when starting from rest (0.018) versus the linear one (0).
    fn func_80836FAC(&self, arg4: f32) -> (bool, f32, i16) {
        if self.unk_6AD != 0 || self.state1 & 1 != 0 {
            return (false, 0.0, self.actor.shape_rot.y);
        }
        let mut speed = self.s.stick_mag;
        let yaw = self.s.stick_angle;
        if arg4 != 0.0 {
            speed -= 20.0;
            if speed < 0.0 {
                speed = 0.0;
            } else {
                let t = 1.0 - cos_s(f2s(speed * 450.0));
                speed = (t * t) * 30.0 + 7.0;
            }
        } else {
            speed *= 0.8;
        }
        if self.s.stick_mag != 0.0 {
            let slope = sin_s(self.unk_898).clamp(0.0, 0.6);
            let mut limit = self.unk_880;
            if self.unk_6C4 != 0.0 {
                limit = (limit - self.unk_6C4 * 0.008).max(2.0);
            }
            speed = speed * 0.14 - 8.0 * slope * slope;
            speed = speed.clamp(0.0, limit);
            return (true, speed, yaw);
        }
        (false, speed, yaw)
    }

    /// `func_80837268`: target speed and world yaw. Returns false (and a yaw that keeps the
    /// current facing) when the stick is idle.
    fn func_80837268(&self, env: &Env, arg3: f32) -> (bool, f32, i16) {
        let (moving, speed, yaw) = self.func_80836FAC(arg3);
        if !moving {
            let mut yaw = self.actor.shape_rot.y;
            if let Some(t) = self.unk_664 {
                if env.target.reticle_locked && self.state2 & STATE2_6 == 0 {
                    yaw = oot_game::target::yaw_to(self.actor.world_pos, env.target(t).expect("the locked-on actor").focus_pos);
                }
            } else if self.func_80833B2C() {
                yaw = self.target_yaw;
            }
            (false, speed, yaw)
        } else {
            (true, speed, yaw.wrapping_add(env.cam_input_yaw))
        }
    }

    /// `func_80837348`: the per-action list of interrupts (`D_80854448`). Ported: 6
    /// (`func_8083C1DC`, roll / put the sword away on A), 7 (`func_80850224`, B attacks), 10
    /// (`func_8083BDBC`, hops while targeting) and 12 (`func_80838A14`, climbing onto ledges);
    /// the others need other items, doors or NPCs and report "not taken".
    fn func_80837348(&mut self, env: &Env, list: &[i8], arg3: bool) -> bool {
        if self.state1 & (1 | (1 << 7) | STATE1_29) != 0 {
            return false;
        }
        if arg3 {
            // D_808535E0 = func_80836670(): the upper-body item action.
            self.s.item_action = self.func_80836670(env) as i32;
        }
        // No interrupts while an item change is pending or playing.
        if self.state1 & STATE1_8 != 0 || self.upper == UpperAction::Change {
            return false;
        }
        let mut i = 0;
        loop {
            let e = list[i];
            let idx = e.unsigned_abs() as usize;
            if idx == 0 && self.func_8083B998(env) {
                return true;
            }
            if idx == 1 && self.func_80839800(env) {
                return true;
            }
            if idx == 2 && self.func_8083E5A8(env) {
                return true;
            }
            if idx == 4 && self.func_8083B644(env) {
                return true;
            }
            if idx == 5 && self.func_8083F7BC(env) {
                return true;
            }
            if idx == 6 && self.func_8083C1DC(env) {
                return true;
            }
            if idx == 12 && self.func_80838A14(env) {
                return true;
            }
            if idx == 10 && self.func_8083BDBC(env) {
                return true;
            }
            if idx == 7 && self.func_80850224(env) {
                return true;
            }
            if idx == 13 && self.func_8083B040(env) {
                return true;
            }
            if e < 0 {
                break;
            }
            i += 1;
        }
        false
    }

    // Interrupt lists (`D_808543E0` ...) as indices into D_80854448.
    const D_80854418: &'static [i8] = &[0, 11, 1, 2, 3, 5, 4, 9, 8, 7, -6];
    const D_80854424: &'static [i8] = &[0, 11, 1, 2, 3, 12, 5, 4, 9, 8, 7, -6];
    const D_80854414: &'static [i8] = &[-7];
    const D_808543E0: &'static [i8] = &[13, 2, 4, 9, 10, 11, 8, -7];
    const D_808543E8: &'static [i8] = &[13, 1, 2, 5, 3, 4, 9, 10, 11, 7, 8, -6];
    const D_808543F4: &'static [i8] = &[13, 1, 2, 3, 4, 9, 10, 11, 8, 7, -6];
    const D_80854400: &'static [i8] = &[13, 2, 4, 9, 10, 11, 8, -7];
    const D_80854408: &'static [i8] = &[13, 2, 4, 9, 10, 11, 12, 8, -7];
    const D_80854430: &'static [i8] = &[13, 1, 2, 3, 12, 5, 4, 9, 10, 11, 8, 7, -6];
    const D_80854440: &'static [i8] = &[10, 8, -7];
    const D_80854444: &'static [i8] = &[0, 12, 5, -4];

    /// `func_8083C1DC`: A pressed → roll if the stick points forward.
    fn func_8083C1DC(&mut self, env: &Env) -> bool {
        if !self.func_80833B54_env(env) && self.s.item_action == 0 && self.state1 & (1 << 23) == 0 && self.input.press.held(BTN_A) {
            if self.func_8083BC7C(env) {
                return true;
            }
            // unk_837 (a charge timer) is 0 here.
            if self.held_item_ap >= env.data.items.ap("SWORD_MASTER") {
                self.func_80835F44(env.data, 0);
            } else {
                self.state2 ^= 1 << 20;
            }
        }
        false
    }

    /// `func_8083BC7C`.
    fn func_8083BC7C(&mut self, env: &Env) -> bool {
        if self.stick_dir() == 0 && self.s.floor_type != FLOOR_TYPE_7 {
            self.func_8083BC04(env.data);
            return true;
        }
        false
    }

    /// `func_8083BC04`: start a roll.
    fn func_8083BC04(&mut self, data: &GameData) {
        self.setup_action(data, Action::Roll, 0);
        let a = self.anim(data, group::ROLL);
        self.skel.play_once_set_speed(data, a, 1.25 * self.s.speed_scale);
    }

    /// `func_80833B54`: locked on to a hostile target (sets `PLAYER_STATE1_4`).
    fn func_80833B54_env(&mut self, env: &Env) -> bool {
        if let Some(t) = self.unk_664
            && env.target(t).is_some_and(|a| a.is_hostile())
        {
            self.state1 |= STATE1_4;
            return true;
        }
        self.func_80833B54()
    }

    /// The "not locked on" half of `func_80833B54` (clears `PLAYER_STATE1_4`).
    fn func_80833B54(&mut self) -> bool {
        if self.state1 & STATE1_4 != 0 {
            self.state1 &= !STATE1_4;
            if self.linear_velocity == 0.0 {
                self.current_yaw = self.actor.shape_rot.y;
            }
        }
        false
    }

    /// `func_8083DF68`: accelerate towards `speed` (`REG(19)`, 1.5) and turn (`REG(27)`).
    fn func_8083DF68(&mut self, speed: f32, yaw: i16) {
        asym_step_to_f(&mut self.linear_velocity, speed, self.regs.reg(19) as f32 / 100.0, 1.5);
        scaled_step_to_s(&mut self.current_yaw, yaw, self.regs.reg(27));
    }

    /// `func_8083DFE0`: air control.
    fn func_8083DFE0(&mut self, speed: f32, yaw: i16) {
        let yaw_diff = self.current_yaw.wrapping_sub(yaw);
        if self.melee_weapon_state == 0 {
            let rsl = self.regs.run_speed_limit();
            self.linear_velocity = self.linear_velocity.clamp(-rsl, rsl);
        }
        if abs16(yaw_diff) > 0x6000 {
            if step_to_f(&mut self.linear_velocity, 0.0, 1.0) {
                self.current_yaw = yaw;
            }
        } else {
            asym_step_to_f(&mut self.linear_velocity, speed, 0.05, 0.1);
            scaled_step_to_s(&mut self.current_yaw, yaw, 200);
        }
    }

    /// `func_8083C484`: sharp reversal while running → brake first.
    fn func_8083C484(&mut self, speed: &mut f32, yaw: &mut i16) -> bool {
        let d = self.current_yaw.wrapping_sub(*yaw);
        if abs16(d) > 0x6000 {
            if self.func_8083721C() {
                *speed = 0.0;
                *yaw = self.current_yaw;
            } else {
                return true;
            }
        }
        false
    }

    // ================================================================================
    // Transitions

    /// `func_80853080`: stand still, playing the wait animation.
    /// Stands Player still where it is, out of any action (a test or sandbox placing it):
    /// `func_80832440`'s interruption, then `func_80853080`.
    pub fn stand_still(&mut self, data: &GameData) {
        self.func_80832440();
        self.func_80832210();
        self.state1 &= !(STATE1_29 | STATE1_0);
        self.func_80853080(data);
    }

    fn func_80853080(&mut self, data: &GameData) {
        self.setup_action(data, Action::StandingStill, 1);
        let a = self.anim(data, group::WAIT);
        self.func_80832B0C(data, a);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_80832B0C`: play once, blending in over 6 frames.
    fn func_80832B0C(&mut self, data: &GameData, a: AnimId) {
        let last = data.anims[a].last_frame();
        self.skel.change(data, a, 1.0, 0.0, last, ANIMMODE_ONCE, -6.0);
    }

    /// `func_80839FFC`: switch to the standing action (locked-on, parallel or plain) without
    /// touching the animation.
    fn func_80839FFC(&mut self, data: &GameData) {
        let action = if self.state1 & STATE1_4 != 0 {
            Action::TargetIdle
        } else if self.func_80833B2C() {
            Action::ParallelIdle
        } else {
            Action::StandingStill
        };
        self.setup_action(data, action, 1);
    }

    /// `func_8083A060`.
    fn func_8083A060(&mut self, data: &GameData) {
        self.func_80839FFC(data);
        if self.state1 & STATE1_4 != 0 {
            self.unk_850 = 1;
        }
    }

    /// `func_8083A098`: stand, playing `anim` once (landing animations).
    fn func_8083A098(&mut self, data: &GameData, anim: AnimId) {
        self.func_8083A060(data);
        self.skel.play_once_set_speed(data, anim, self.s.speed_scale);
    }

    /// `func_8083C858`: start walking/running (the targeting run while targeting).
    fn func_8083C858(&mut self, data: &GameData) {
        let action = if self.func_80833BCC() { Action::TargetRun } else { Action::Run };
        self.setup_action(data, action, 1);
        let a = self.anim(data, group::RUN);
        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -6.0);
        self.unk_89C = 0;
        self.unk_864 = 0.0;
        self.unk_868 = 0.0;
    }

    /// `func_8083C8DC`: face `yaw` and start running.
    fn func_8083C8DC(&mut self, data: &GameData, yaw: i16) {
        self.actor.shape_rot.y = yaw;
        self.current_yaw = yaw;
        self.func_8083C858(data);
    }

    /// `func_8083CD54`: turn in place towards `yaw`.
    fn func_8083CD54(&mut self, data: &GameData, yaw: i16) {
        self.current_yaw = yaw;
        self.setup_action(data, Action::Turn, 1);
        self.unk_87E = (1200.0 * self.s.speed_scale) as i16;
        let a = self.anim(data, group::TURN);
        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -6.0);
    }

    /// `func_8083C0E8`: finish a turn.
    fn func_8083C0E8(&mut self, data: &GameData) {
        self.setup_action(data, Action::StandingStill, 1);
        let a = self.anim(data, group::WAIT);
        self.skel.play_once(data, a);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_8083BF50`: the stop animation, picked by the walk phase so the feet match.
    fn func_8083BF50(&mut self, data: &GameData) {
        let mut sp30 = self.unk_868 - 3.0;
        if sp30 < 0.0 {
            sp30 += 29.0;
        }
        let anim;
        if sp30 < 14.0 {
            anim = self.anim(data, group::END_WALK_A);
            sp30 = 11.0 - sp30;
            if sp30 < 0.0 {
                sp30 = 1.375 * -sp30;
            }
            sp30 /= 11.0;
        } else {
            anim = self.anim(data, group::END_WALK_B);
            sp30 = 26.0 - sp30;
            if sp30 < 0.0 {
                sp30 = 2.0 * -sp30;
            }
            sp30 /= 12.0;
        }
        let last = data.anims[anim].last_frame();
        self.skel.change(data, anim, 1.0, 0.0, last, ANIMMODE_ONCE, 4.0 * sp30);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_8083C0B8`: stop running.
    fn func_8083C0B8(&mut self, data: &GameData) {
        self.func_80839FFC(data);
        self.func_8083BF50(data);
    }

    /// `func_808409CC`: the next idle animation: the plain wait every other time, otherwise
    /// a random fidget from `D_80853D7C`.
    fn func_808409CC(&mut self, data: &GameData) {
        self.unk_6AC = (self.unk_6AC + 1) & 1;
        let anim = if self.unk_6AC != 0 {
            self.state2 &= !STATE2_28;
            self.anim(data, group::WAIT)
        } else {
            self.state2 |= STATE2_28;
            // Room behaviour type 2 is 0 (normal); health not critical.
            let mut sp38 = 0usize;
            let sp34 = (self.rand_zero_one() * 5.0) as i32;
            // Right hand is open (nothing held), so only variants 1 and 2 qualify.
            if sp34 < 4 && sp34 != 0 && sp34 != 3 {
                sp38 = sp34 as usize + 9;
            }
            let pair = data.idle_variants[sp38];
            if self.model_anim_type != 1 { pair[1] } else { pair[0] }
        };
        let last = data.anims[anim].last_frame();
        self.skel.change(data, anim, (2.0 / 3.0) * self.s.speed_scale, 0.0, last, ANIMMODE_ONCE, -6.0);
    }

    // ================================================================================
    // Look and lean (head/upper body rotations applied at draw time)

    /// `func_808369C8`.
    fn func_808369C8(v: &mut i16, arg1: i16, arg2: i16, arg3: i16, arg4: i16, arg5: i16) -> i16 {
        let t1 = arg4.wrapping_sub(*v);
        let t2 = t1.clamp(arg5.wrapping_neg().min(arg5), arg5.max(arg5.wrapping_neg()));
        *v = v.wrapping_add(t1.wrapping_sub(t2));
        scaled_step_to_s(v, arg1, arg2);
        let t3 = *v;
        if *v < -arg3 {
            *v = -arg3;
        } else if *v > arg3 {
            *v = arg3;
        }
        t3.wrapping_sub(*v)
    }

    /// `func_80836AB8(this, arg1)`: spread the focus rotation over head and torso (`arg1`:
    /// aiming, the whole upper body follows the focus).
    fn func_80836AB8_arg(&mut self, arg1: bool) -> i16 {
        if arg1 {
            self.unk_6BC = self.actor.focus_rot.x;
            self.unk_6AE |= 0x41;
            return self.actor.focus_rot.y;
        }
        self.func_80836AB8()
    }

    /// `func_80836AB8(this, 0)`.
    fn func_80836AB8(&mut self) -> i16 {
        let mut var = self.actor.shape_rot.y;
        let fx = self.actor.focus_rot.x;
        let inner = Self::func_808369C8(&mut self.unk_6B6, fx, 600, 10000, fx, 0);
        let b6 = self.unk_6B6;
        Self::func_808369C8(&mut self.unk_6BC, inner, 200, 4000, b6, 10000);
        let mut sp36 = self.actor.focus_rot.y.wrapping_sub(var);
        let be = self.unk_6BE;
        Self::func_808369C8(&mut sp36, 0, 200, 24000, be, 8000);
        var = self.actor.focus_rot.y.wrapping_sub(sp36);
        let be = self.unk_6BE;
        Self::func_808369C8(&mut self.unk_6B8, sp36.wrapping_sub(be), 200, 8000, sp36, 8000);
        let b8 = self.unk_6B8;
        Self::func_808369C8(&mut self.unk_6BE, sp36, 200, 8000, b8, 8000);
        self.unk_6AE |= 0xD9;
        var
    }

    /// `func_8083DC54`: look at the target, or at the ground ahead.
    fn func_8083DC54(&mut self, env: &Env) {
        if self.unk_664.is_some() {
            self.func_8083DB98(env, false);
            return;
        }
        let mut sp46 = 0i16;
        let p = self.func_808395DC(self.actor.world_pos, HEAD_PROBE);
        let (floor, _) = env.col.entity_raycast_down(p);
        if floor > BGCHECK_Y_MIN {
            let t = atan2_s(40.0, self.actor.world_pos.y - floor);
            sp46 = t.clamp(-4000, 4000);
        }
        self.actor.focus_rot.y = self.actor.shape_rot.y;
        smooth_step_to_s(&mut self.actor.focus_rot.x, sp46, 14, 4000, 30);
        self.func_80836AB8();
    }

    /// `func_8083DDC8`: lean into turns when running fast.
    fn func_8083DDC8(&mut self, env: &Env) {
        if self.linear_velocity > 5.0 {
            let t1 = f2s(self.linear_velocity * 200.0).clamp(-4000, 4000);
            let t2 = f2s(self.current_yaw.wrapping_sub(self.actor.shape_rot.y) as f32 * self.linear_velocity * 0.1);
            let t2 = t2.wrapping_neg().clamp(-4000, 4000);
            scaled_step_to_s(&mut self.unk_6BC, t1, 900);
            self.unk_6B6 = f2s(-(self.unk_6BC as f32) * 0.5);
            scaled_step_to_s(&mut self.unk_6BA, t2, 300);
            scaled_step_to_s(&mut self.unk_6C0, t2, 200);
            self.unk_6AE |= 0x168;
        } else {
            self.func_8083DC54(env);
        }
    }

    /// `func_808471F4`.
    fn func_808471F4(v: &mut i16) {
        let step = f2s((abs16(*v) as f32 * 100.0) / 1000.0).clamp(400, 4000);
        scaled_step_to_s(v, 0, step);
    }

    /// `func_80847298`: relax whichever look/lean rotations were not driven last frame.
    fn func_80847298(&mut self) {
        let f = self.unk_6AE;
        if f & 2 == 0 {
            let mut d = self.actor.focus_rot.y.wrapping_sub(self.actor.shape_rot.y);
            Self::func_808471F4(&mut d);
            self.actor.focus_rot.y = self.actor.shape_rot.y.wrapping_add(d);
        }
        if f & 1 == 0 {
            Self::func_808471F4(&mut self.actor.focus_rot.x);
        }
        if f & 8 == 0 {
            Self::func_808471F4(&mut self.unk_6B6);
        }
        if f & 0x40 == 0 {
            Self::func_808471F4(&mut self.unk_6BC);
        }
        if f & 4 == 0 {
            Self::func_808471F4(&mut self.actor.focus_rot.z);
        }
        if f & 0x10 == 0 {
            Self::func_808471F4(&mut self.unk_6B8);
        }
        if f & 0x20 == 0 {
            Self::func_808471F4(&mut self.unk_6BA);
        }
        if f & 0x80 == 0 {
            if self.unk_6B0 != 0 {
                Self::func_808471F4(&mut self.unk_6B0);
            } else {
                Self::func_808471F4(&mut self.unk_6BE);
            }
        }
        if f & 0x100 == 0 {
            Self::func_808471F4(&mut self.unk_6C0);
        }
        self.unk_6AE = 0;
    }

    /// `func_808368EC`: the facing. Standing, it turns towards a locked target (once the reticle
    /// has locked) or to `targetYaw` in parallel mode; running/rolling it follows `currentYaw`.
    fn func_808368EC(&mut self, env: &Env) {
        let prev = self.actor.shape_rot.y;
        if self.state2 & (STATE2_5 | STATE2_6) == 0 {
            if let Some(t) = self.unk_664.and_then(|t| env.target(t))
                && env.target.reticle_locked
            {
                let y = oot_game::target::yaw_to(self.actor.world_pos, t.focus_pos);
                scaled_step_to_s(&mut self.actor.shape_rot.y, y, 4000);
            } else if self.state1 & STATE1_17 != 0 {
                let y = self.target_yaw;
                scaled_step_to_s(&mut self.actor.shape_rot.y, y, 4000);
            }
        } else if self.state2 & STATE2_6 == 0 {
            scaled_step_to_s(&mut self.actor.shape_rot.y, self.current_yaw, 2000);
        }
        self.unk_87C = self.actor.shape_rot.y.wrapping_sub(prev);
    }

    // ================================================================================
    // Walk / run animation

    /// A sound request (`PlayRequest::Sfx`).
    fn sfx(&mut self, s: PlayerSfx) {
        self.play_requests.push(PlayRequest::Sfx(s));
    }

    /// `func_8002F7DC(&this->actor, sfxId)`.
    fn func_8002F7DC(&mut self, sfx_id: u16) {
        self.sfx(PlayerSfx::Actor(sfx_id));
    }

    /// `func_80832698`: Link's voice (the age's voice bank, `ageProperties->unk_92`).
    fn func_80832698(&mut self, sfx_id: u16) {
        if self.actor.category == ACTORCAT_PLAYER {
            self.func_8002F7DC(sfx_id.wrapping_add(self.age.climb.unk_92));
        } else {
            self.sfx(PlayerSfx::F4190(sfx_id));
        }
    }

    /// `func_8083275C`: a sound for the floor (`unk_89E`).
    fn func_8083275C(&self, sfx_id: u16) -> u16 {
        sfx_id.wrapping_add(self.unk_89E)
    }

    /// `func_80832770`.
    fn func_80832770(&mut self, sfx_id: u16) {
        let id = self.func_8083275C(sfx_id);
        self.func_8002F7DC(id);
    }

    /// `func_808327A4`: a sound for the floor and the age (`ageProperties->unk_94`).
    fn func_808327A4(&self, sfx_id: u16) -> u16 {
        sfx_id.wrapping_add(self.unk_89E).wrapping_add(self.age.climb.unk_94)
    }

    /// `func_808327C4`.
    fn func_808327C4(&mut self, sfx_id: u16) {
        let id = self.func_808327A4(sfx_id);
        self.func_8002F7DC(id);
    }

    /// `func_808327F8`: a footstep at speed `arg1`.
    fn func_808327F8(&mut self, arg1: f32) {
        let sfx_id = if self.current_boots == PLAYER_BOOTS_IRON { NA_SE_PL_WALK_HEAVYBOOTS } else { self.func_808327A4(NA_SE_PL_WALK_GROUND) };
        self.sfx(PlayerSfx::Footstep(sfx_id, arg1));
    }

    /// `func_80832854`: the jump.
    fn func_80832854(&mut self) {
        let sfx_id = if self.current_boots == PLAYER_BOOTS_IRON { NA_SE_PL_JUMP_HEAVYBOOTS } else { self.func_808327A4(NA_SE_PL_JUMP) };
        self.func_8002F7DC(sfx_id);
    }

    /// `func_808328A0`: the landing.
    fn func_808328A0(&mut self) {
        let sfx_id = if self.current_boots == PLAYER_BOOTS_IRON { NA_SE_PL_LAND_HEAVYBOOTS } else { self.func_808327A4(NA_SE_PL_LAND) };
        self.func_8002F7DC(sfx_id);
    }

    /// `func_80832924`: the sounds of a `struct_80832924` table on this frame of the animation:
    /// each row's frame is `field`'s low 11 bits, its kind the next four (0x800 at Player,
    /// 0x1000 for the floor, 0x1800 for the floor and the age, 0x2000 the voice, 0x2800 the
    /// landing, 0x3000 a running step, 0x3800 the jump, 0x4000 a step, 0x4800 the ladder); the
    /// last row's `field` is negative.
    fn func_80832924(&mut self, entries: &[(u16, i16)]) {
        use oot_game::audio::sfx::NA_SE_PL_WALK_LADDER;
        for &(sfx_id, field) in entries {
            let data = (field as i32).abs();
            let flags = data & 0x7800;
            if self.skel.on_frame((data & 0x7FF) as f32) {
                match flags {
                    0x800 => self.func_8002F7DC(sfx_id),
                    0x1000 => self.func_80832770(sfx_id),
                    0x1800 => self.func_808327C4(sfx_id),
                    0x2000 => self.func_80832698(sfx_id),
                    0x2800 => self.func_808328A0(),
                    0x3000 => self.func_808327F8(6.0),
                    0x3800 => self.func_80832854(),
                    0x4000 => self.func_808327F8(0.0),
                    0x4800 => self.sfx(PlayerSfx::Footstep(self.age.climb.unk_94.wrapping_add(NA_SE_PL_WALK_LADDER), 0.0)),
                    _ => {}
                }
            }
            if field < 0 {
                break;
            }
        }
    }

    /// `func_808328EC`: a sound that marks the frame (`PLAYER_STATE2_3`).
    fn func_808328EC(&mut self, sfx_id: u16) {
        self.func_8002F7DC(sfx_id);
        self.state2 |= STATE2_3;
    }

    /// `func_8084021C`: did the phase cross `arg2`/`arg3` this step (footstep timing)?
    fn func_8084021C(arg0: f32, arg1: f32, arg2: f32, mut arg3: f32) -> bool {
        if arg3 == 0.0 && arg1 > 0.0 {
            arg3 = arg2;
        }
        let t = (arg0 + arg1) - arg3;
        t * arg1 >= 0.0 && (t - arg1) * arg1 < 0.0
    }

    /// `func_8084029C`: advance the walk phase `unk_868` (29-frame cycle).
    fn func_8084029C(&mut self, arg1: f32) {
        let arg1 = (arg1 * UPDATE_SCALE).clamp(-7.25, 7.25);
        // (The hover boots' NA_SE_PL_HOBBERBOOTS_LV: they aren't ported.)
        if Self::func_8084021C(self.unk_868, arg1, 29.0, 10.0) || Self::func_8084021C(self.unk_868, arg1, 29.0, 24.0) {
            let v = self.linear_velocity;
            self.func_808327F8(v);
            if self.linear_velocity > 4.0 {
                self.state2 |= STATE2_3;
            }
        }
        self.unk_868 += arg1;
        if self.unk_868 < 0.0 {
            self.unk_868 += 29.0;
        } else if self.unk_868 >= 29.0 {
            self.unk_868 -= 29.0;
        }
    }

    /// `func_80833438`: which run animation (damage, iron boots, normal).
    fn func_80833438(&self, data: &GameData) -> AnimId {
        if self.unk_890 != 0 { self.anim(data, group::DAMAGE_RUN) } else { self.anim(data, group::RUN) }
    }

    /// `func_80841CC4`: walk animation, blended towards climbing up/down on slopes.
    fn func_80841CC4(&mut self, data: &GameData, to_morph: bool) {
        let target = if abs16(self.s.facing_slope) < 3640 { 0 } else { self.s.facing_slope.clamp(-10922, 10922) };
        scaled_step_to_s(&mut self.unk_89C, target, 400);
        let walk = self.anim(data, group::WALK);
        if self.model_anim_type == 3 || (self.unk_89C == 0 && self.unk_6C4 <= 0.0) {
            if to_morph {
                self.skel.load_to_morph(data, walk, self.unk_868);
            } else {
                self.skel.load_to_joint(data, walk, self.unk_868);
            }
            return;
        }
        let mut rate = if self.unk_89C != 0 { self.unk_89C as f32 / 10922.0 } else { self.unk_6C4 * 0.0006 };
        rate *= self.linear_velocity.abs() * 0.5;
        if rate > 1.0 {
            rate = 1.0;
        }
        let anim = if rate < 0.0 {
            rate = -rate;
            data.anim("link_normal_climb_down")
        } else {
            data.anim("link_normal_climb_up")
        };
        if to_morph {
            self.skel.blend_to_morph(data, walk, self.unk_868, anim, self.unk_868, rate);
        } else {
            self.skel.blend_to_joint(data, walk, self.unk_868, anim, self.unk_868, rate);
        }
    }

    /// `func_80841EE4`: the walk→run blend by speed (`REG(35..38)`, `REG(48)`).
    fn func_80841EE4(&mut self, data: &GameData) {
        let r = |n| self.regs.reg(n) as f32 / 1000.0;
        let temp1;
        if self.unk_864 < 1.0 {
            self.func_8084029C(r(35));
            let walk = self.anim(data, group::WALK);
            self.skel.load_to_joint(data, walk, self.unk_868);
            self.unk_864 = (self.unk_864 + UPDATE_SCALE).min(1.0);
            temp1 = self.unk_864;
        } else {
            let temp2 = self.linear_velocity - (self.regs.reg(48) as f32 / 100.0);
            if temp2 < 0.0 {
                temp1 = 1.0;
                self.func_8084029C(r(35) + r(36) * self.linear_velocity);
                self.func_80841CC4(data, false);
            } else {
                let mut t = r(37) * temp2;
                if t < 1.0 {
                    self.func_8084029C(r(35) + r(36) * self.linear_velocity);
                } else {
                    t = 1.0;
                    self.func_8084029C(1.2 + r(38) * temp2);
                }
                temp1 = t;
                self.func_80841CC4(data, true);
                let run = self.func_80833438(data);
                self.skel.load_to_joint(data, run, self.unk_868 * (20.0 / 29.0));
            }
        }
        if temp1 < 1.0 {
            self.skel.interp_joint_morph(1.0 - temp1);
        }
    }

    // ================================================================================
    // Actions

    /// `func_80840BC8`: standing still. Plays idle animations, turns in place towards the
    /// stick, and starts running once the stick gives a speed.
    fn func_80840BC8(&mut self, env: &Env) {
        let data = env.data;
        let done = self.skel.update(data);
        if done {
            if self.unk_850 != 0 {
                // DECR(this->unk_850)
                if self.unk_850 != 0 {
                    self.unk_850 -= 1;
                    if self.unk_850 == 0 {
                        self.skel.end_frame = self.skel.anim_length - 1.0;
                    }
                }
                // Fall-damage stagger: the root bobs by ±0x28.
                self.skel.joint[0][1] = self.skel.joint[0][1].wrapping_add(((self.unk_850 & 1) * 0x50) - 0x28);
            } else {
                self.func_80832DBC();
                self.func_808409CC(data);
            }
        }
        self.func_8083721C();
        if self.unk_850 == 0 && !self.func_80837348(env, Self::D_80854418, true) {
            if self.func_80833B54_env(env) {
                self.func_8083CEAC(data);
                return;
            }
            if self.func_80833B2C() {
                self.func_80839F30(data);
                return;
            }
            let (_, speed, yaw) = self.func_80837268(env, 0.018);
            if speed != 0.0 {
                self.func_8083C8DC(data, yaw);
                return;
            }
            let d = yaw.wrapping_sub(self.actor.shape_rot.y);
            if abs16(d) > 800 {
                self.func_8083CD54(data, yaw);
                return;
            }
            scaled_step_to_s(&mut self.actor.shape_rot.y, yaw, 1200);
            self.current_yaw = self.actor.shape_rot.y;
            if self.anim(data, group::WAIT) == self.skel.animation {
                self.func_8083DC54(env);
            }
        }
    }

    /// `func_80841BA8`: turning in place.
    fn func_80841BA8(&mut self, env: &Env) {
        let data = env.data;
        self.skel.update(data);
        let (_, speed, yaw) = self.func_80837268(env, 0.018);
        if !self.func_80837348(env, Self::D_80854414, true) {
            if speed != 0.0 {
                self.actor.shape_rot.y = yaw;
                self.func_8083C858(data);
            } else if scaled_step_to_s(&mut self.actor.shape_rot.y, yaw, self.unk_87E) {
                self.func_8083C0E8(data);
            }
            self.current_yaw = self.actor.shape_rot.y;
        }
    }

    /// `func_80842180`: walking and running.
    fn func_80842180(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        self.func_80841EE4(data);
        if !self.func_80837348(env, Self::D_80854424, true) {
            if self.func_80833C04(env) {
                self.func_8083C858(data);
                return;
            }
            let (_, mut speed, mut yaw) = self.func_80837268(env, 0.018);
            if !self.func_8083C484(&mut speed, &mut yaw) {
                self.func_8083DF68(speed, yaw);
                self.func_8083DDC8(env);
                if self.linear_velocity == 0.0 && speed == 0.0 {
                    self.func_8083C0B8(data);
                }
            }
        }
    }

    /// `func_80844708`: rolling.
    fn func_80844708(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        let done = self.skel.update(data);
        if self.skel.on_frame(8.0) {
            self.func_80837AFC(-10);
        }
        // func_80842964 (first person, items, grabbing): not taken.
        if self.unk_850 != 0 {
            step_to_f(&mut self.linear_velocity, 0.0, 2.0);
            let temp = self.func_808374A0(env, 5.0);
            if temp != 0 && (temp > 0 || done) {
                self.func_8083A060(data);
            }
        } else {
            if self.linear_velocity >= 7.0
                && self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0
                && self.s.wall_move_diff < 0x2000
            {
                // Bonk.
                let a = self.anim(data, group::ROLL_BONK);
                self.skel.play_once(data, a);
                self.linear_velocity = -self.linear_velocity;
                // (func_808429B4's quake and the rumble aren't ported.)
                self.func_8002F7DC(NA_SE_PL_BODY_HIT);
                self.func_80832698(NA_SE_VO_LI_CLIMB_END);
                self.unk_850 = 1;
                self.note("roll bonk");
                return;
            }
            // Past frame 15 func_80850224 could chain into a sword attack: no sword.
            if self.skel.cur_frame >= 20.0 {
                self.func_8083A060(data);
                return;
            }
            let (_, mut speed, _) = self.func_80837268(env, 0.018);
            speed *= 1.5;
            if speed < 3.0 || self.stick_dir() != 0 {
                speed = 3.0;
            }
            let yaw = self.actor.shape_rot.y;
            self.func_8083DF68(speed, yaw);
            // func_8084269C: on a ground or sand floor the roll raises dust (its effect,
            // func_800286CC, isn't ported) and its sound.
            if self.unk_89E == 0 || self.unk_89E == SURFACE_SFX_TYPE_1 {
                self.actor.func_8002f8f0(NA_SE_PL_ROLL_DUST - SFX_FLAG);
            }
            self.func_80832924(env.audio.player_anim_sfx("D_8085460C"));
        }
    }

    /// `func_808374A0`: near the end of an animation, allow interrupts or restarting to run.
    /// Returns 1 to move on, 0 if an interrupt took over, -1 to keep playing.
    fn func_808374A0(&mut self, env: &Env, arg3: f32) -> i32 {
        if (self.skel.end_frame - arg3) <= self.skel.cur_frame {
            if self.func_80837348(env, Self::D_80854418, true) {
                return 0;
            }
            if self.func_80837268(env, 0.018).0 {
                return 1;
            }
        }
        -1
    }

    /// `func_80843E64`: landing. Returns 1/2 for a damaging fall (≥ 400 / 800: `D_80854600`'s
    /// half heart and heart, then 40 frames of invincibility, the body's and the voice's
    /// sounds), -1 when that was the last of Link's health, 0 otherwise with the landing's sound.
    /// (The quake and the rumble aren't ported.)
    fn func_80843E64(&mut self, env: &Env) -> i32 {
        let sp34 = if self.s.floor_type == FLOOR_TYPE_6 || self.s.floor_type == FLOOR_TYPE_9 { 0 } else { self.fall_distance as i32 };
        step_to_f(&mut self.linear_velocity, 0.0, 1.0);
        self.state1 &= !(STATE1_18 | STATE1_19);
        if sp34 >= 400 {
            let idx = if self.fall_distance < 800 { 0 } else { 1 };
            // D_80854600[impactIndex].damage.
            let damage = if idx == 0 { -8 } else { -16 };
            if self.player_inflict_damage(env, damage) {
                return -1;
            }
            self.func_80837AE0(40);
            self.func_8002F7DC(NA_SE_PL_BODY_HIT);
            // D_80854600[impactIndex].sfxId (both rows).
            self.func_80832698(NA_SE_VO_LI_LAND_DAMAGE_S);
            return idx + 1;
        }
        if sp34 > 200 && self.s.floor_type == FLOOR_TYPE_6 {
            // (The rumble, sp34 doubled, isn't ported.)
            self.func_80832698(NA_SE_VO_LI_CLIMB_END);
        }
        self.func_808328A0();
        0
    }

    /// `func_80843E14`: a fall's voice (Ruto, held, would cry too: `En_Ru1` isn't ported, nor is
    /// holding).
    fn func_80843E14(&mut self, sfx_id: u16) {
        self.func_80832698(sfx_id);
    }

    /// `func_8084411C`: in the air.
    fn func_8084411C(&mut self, env: &Env) {
        let data = env.data;
        // respawn[RESPAWN_MODE_TOP].data > 40 (void-out float): not modelled.
        if self.state1 & STATE1_4 != 0 {
            self.actor.gravity = -1.2;
        }
        let (_, speed, yaw) = self.func_80837268(env, 0.0);
        if !self.grounded() {
            self.skel.update(data);
            if self.state2 & STATE2_19 == 0 {
                self.func_8083DFE0(speed, yaw);
            }
            self.func_80836670(env);
            // func_8083BBA0 (the jump slash in the air) is not ported.
            if self.actor.velocity.y < 0.0 {
                if self.unk_850 >= 0 {
                    if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 || self.unk_850 == 0 || self.fall_distance > 0 {
                        if self.s.floor_dist > 800.0 || self.state1 & STATE1_2 != 0 {
                            self.func_80843E14(NA_SE_VO_LI_FALL_S);
                            self.state1 &= !STATE1_2;
                        }
                        let a = data.anim("link_normal_landing");
                        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_ONCE, 8.0);
                        self.unk_850 = -1;
                    }
                } else {
                    if self.unk_850 == -1 && self.fall_distance > 120 && self.s.floor_dist > 280.0 {
                        self.unk_850 = -2;
                        self.func_80843E14(NA_SE_VO_LI_FALL_L);
                    }
                    if self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0
                        && self.state2 & STATE2_19 == 0
                        && self.state1 & (STATE1_11 | STATE1_27) == 0
                        && self.linear_velocity > 0.0
                    {
                        if self.wall_height >= 150.0 && self.stick_dir() == 0 {
                            // func_8083EC18: grab a climbable wall from the air — not ported.
                            self.note("mid-air wall climb (func_8083EC18) not ported");
                        } else if self.unk_88C >= 2
                            && self.wall_height < 150.0
                            && ((self.actor.world_pos.y - self.actor.floor_height) + self.wall_height) > 70.0 * self.age.translation_scale
                            && let Some(wp) = self.actor.wall_poly
                        {
                            self.skel.disable_queue();
                            self.func_80832698(if self.state1 & STATE1_2 != 0 { NA_SE_VO_LI_HOOKSHOT_HANG } else { NA_SE_VO_LI_HANG });
                            self.actor.world_pos.y += self.wall_height;
                            let a = self.anim(data, group::HANG_GRAB);
                            let dist = self.wall_distance;
                            self.func_8083A5C4(data, env, wp, dist, a);
                            self.current_yaw = self.current_yaw.wrapping_add(i16::MIN);
                            self.actor.shape_rot.y = self.current_yaw;
                            self.state1 |= STATE1_13;
                            return;
                        }
                    }
                }
            }
        } else {
            let mut anim = self.anim(data, group::LANDING);
            if self.state2 & STATE2_19 != 0 {
                let row = data.side_hop_anims[self.unk_84F.clamp(0, 3) as usize];
                anim = if self.state1 & STATE1_4 != 0 { row[2] } else { row[1] };
            } else if self.skel.animation == data.anim("link_normal_run_jump") {
                anim = data.anim("link_normal_run_jump_end");
            } else if self.state1 & STATE1_4 != 0 {
                anim = data.anim("link_anchor_landingR");
                self.func_80833C3C();
            } else if self.fall_distance <= 80 {
                anim = self.anim(data, group::SHORT_LANDING);
            } else if self.fall_distance < 800 && self.stick_dir() == 0 && self.state1 & STATE1_11 == 0 {
                self.func_8083BC04(data);
                return;
            }
            let sp3c = self.func_80843E64(env);
            if sp3c > 0 {
                let a = self.anim(data, group::LANDING);
                self.func_8083A098(data, a);
                self.skel.end_frame = 8.0;
                self.unk_850 = if sp3c == 1 { 10 } else { 20 };
            } else if sp3c == 0 {
                self.func_8083A098(data, anim);
            }
        }
    }

    // ================================================================================
    // Z-targeting (func_80836BEC) and the targeting actions

    /// `func_80833B2C`: parallel mode or a non-hostile target.
    fn func_80833B2C(&self) -> bool {
        self.state1 & (STATE1_16 | STATE1_17 | STATE1_30) != 0
    }

    /// `func_80833BCC`: locked on or in parallel mode.
    fn func_80833BCC(&self) -> bool {
        self.state1 & STATE1_4 != 0 || self.func_80833B2C()
    }

    /// `func_80833C04`.
    fn func_80833C04(&mut self, env: &Env) -> bool {
        self.func_80833B54_env(env) || self.func_80833B2C()
    }

    /// `func_8008EDF0`: drop the target.
    fn func_8008EDF0(&mut self) {
        self.unk_664 = None;
        self.state2 &= !STATE2_13;
    }

    /// `func_8008EE08`: leave targeting (or, in the air, remember to after landing).
    fn func_8008EE08(&mut self) {
        if self.grounded()
            || self.state1 & ((1 << 21) | STATE1_23 | STATE1_27) != 0
            || (self.state1 & (STATE1_18 | STATE1_19) == 0 && (self.actor.world_pos.y - self.actor.floor_height) < 100.0)
        {
            self.state1 &= !(STATE1_15 | STATE1_16 | STATE1_17 | STATE1_18 | STATE1_19 | STATE1_30);
        } else if self.state1 & (STATE1_18 | STATE1_19 | (1 << 21)) == 0 {
            self.state1 |= STATE1_19;
        }
        self.func_8008EDF0();
    }

    /// `func_808355DC`: enter parallel mode, squaring up to a wall in front.
    fn func_808355DC(&mut self) {
        self.state1 |= STATE1_17;
        if self.skel.move_flags & 0x80 == 0 && self.actor.bg_check_flags & BGCHECKFLAG_PLAYER_WALL_INTERACT != 0 && self.s.wall_facing_diff < 0x2000 {
            self.current_yaw = self.actor.wall_yaw.wrapping_add(i16::MIN);
            self.actor.shape_rot.y = self.current_yaw;
        }
        self.target_yaw = self.actor.shape_rot.y;
    }

    /// `func_80836BEC`: the Z button. With the default "Switch" setting (`zTargetSetting` 0) a
    /// press locks on to `arrowPointedActor` (or the next candidate if it's already locked) and
    /// keeps it (`PLAYER_STATE2_13`) until pressed again; with nothing to target, parallel mode.
    fn func_80836BEC(&mut self, env: &Env) {
        let z = self.input.cur.held(BTN_Z);
        if !z {
            self.state1 &= !STATE1_30;
        }
        if self.state1 & (STATE1_7 | STATE1_29) != 0 || self.state3 & STATE3_7 != 0 {
            self.unk_66C = 0;
        } else if z || self.state2 & STATE2_13 != 0 {
            if self.unk_66C <= 5 {
                self.unk_66C = 5;
            } else {
                self.unk_66C -= 1;
            }
        } else if self.state1 & STATE1_17 != 0 {
            self.unk_66C = 0;
        } else if self.unk_66C != 0 {
            self.unk_66C -= 1;
        }
        let sp1c = self.unk_66C >= 6;
        // func_8083224C: talking (ACTOR_FLAG_8) keeps the target as it is.
        let cond = self.actor.flags & ACTOR_FLAG_8 == ACTOR_FLAG_8;
        if cond || self.unk_66C != 0 || self.state1 & (STATE1_12 | STATE1_25) != 0 {
            if !cond {
                if self.state1 & STATE1_25 == 0 && self.input.press.held(BTN_Z) {
                    let hold = false;
                    self.state1 |= STATE1_15;
                    match env.target.arrow_pointed.filter(|&t| env.target(t).is_some_and(|a| a.flags & ACTOR_FLAG_27 == 0)) {
                        Some(t) => {
                            let mut to = Some(t);
                            if to == self.unk_664 {
                                to = env.target.unk_94;
                            }
                            if to != self.unk_664 {
                                if !hold {
                                    self.state2 |= STATE2_13;
                                }
                                self.unk_664 = to;
                                self.unk_66C = 15;
                                self.state2 &= !(STATE2_1 | STATE2_21);
                            } else if !hold {
                                self.func_8008EDF0();
                            }
                            self.state1 &= !STATE1_30;
                        }
                        None => {
                            if self.state1 & (STATE1_17 | STATE1_30) == 0 {
                                self.func_808355DC();
                            }
                        }
                    }
                }
                if let Some(t) = self.unk_664 {
                    if env.target(t).is_none_or(|a| oot_game::target::lost(&env.data.target_ranges, a, true, self.actor.shape_rot.y, sp1c)) {
                        self.func_8008EDF0();
                        self.state1 |= STATE1_30;
                    }
                }
            }
            if let Some(t) = self.unk_664 {
                self.state1 &= !(STATE1_16 | STATE1_17);
                if self.state1 & STATE1_11 != 0 || !env.target(t).is_some_and(|a| a.is_hostile()) {
                    self.state1 |= STATE1_16;
                }
            } else if self.state1 & STATE1_17 != 0 {
                self.state2 &= !STATE2_13;
            } else {
                self.func_8008EE08();
            }
        } else {
            self.func_8008EE08();
        }
    }

    /// `func_8083DB98`: turn the head (and torso) towards the target's focus.
    fn func_8083DB98(&mut self, env: &Env, arg1: bool) -> i16 {
        let Some(t) = self.unk_664.and_then(|t| env.target(t)) else { return self.actor.shape_rot.y };
        let from = Vec3::new(self.actor.world_pos.x, self.head_pos.y + 3.0, self.actor.world_pos.z);
        let pitch = oot_game::target::pitch_to(from, t.focus_pos);
        let yaw = oot_game::target::yaw_to(from, t.focus_pos);
        smooth_step_to_s(&mut self.actor.focus_rot.y, yaw, 4, 10000, 0);
        smooth_step_to_s(&mut self.actor.focus_rot.x, pitch, 4, 10000, 0);
        self.unk_6AE |= 2;
        self.func_80836AB8_arg(arg1)
    }

    /// `func_808334E4` / `func_80833528`: the locked-on wait animations (right / left foot forward).
    fn func_808334E4(&self, data: &GameData) -> AnimId {
        self.anim(data, group::TARGET_WAIT_R)
    }
    fn func_80833528(&self, data: &GameData) -> AnimId {
        self.anim(data, group::TARGET_WAIT_L)
    }

    /// `func_808401B0`: blend the two target waits by `unk_870`.
    fn func_808401B0(&mut self, data: &GameData) {
        let (a, b) = (self.func_808334E4(data), self.func_80833528(data));
        self.skel.blend_to_joint(data, a, self.unk_868, b, self.unk_868, self.unk_870);
    }

    /// `func_80840138`: which foot leads (`unk_874`), eased into `unk_870`.
    fn func_80840138(&mut self, speed: f32, yaw: i16) {
        let d = yaw.wrapping_sub(self.actor.shape_rot.y);
        if speed > 0.0 {
            self.unk_874 = if d < 0 { 0.0 } else { 1.0 };
        }
        step_to_f(&mut self.unk_870, self.unk_874, 0.3);
    }

    /// `func_8083FC68`: locked on, classify the stick: 1 run forward, -1 step back, 0 sidestep/stay.
    fn func_8083FC68(&mut self, env: &Env, speed: f32, yaw: i16) -> i32 {
        let sp1c = yaw.wrapping_sub(self.actor.shape_rot.y) as f32;
        if self.unk_664.is_some() {
            self.func_8083DB98(env, false);
        }
        let t = sp1c.abs() / 32768.0;
        if speed > (t * t) * 50.0 + 6.0 {
            1
        } else if speed > (1.0 - t) * 10.0 + 6.8 {
            -1
        } else {
            0
        }
    }

    /// `func_8083FD78`: the same for parallel mode (relative to `targetYaw`).
    fn func_8083FD78(&mut self, env: &Env, speed: &mut f32, yaw: &mut i16) -> i32 {
        let sp2e = yaw.wrapping_sub(self.target_yaw);
        let sp2c = (sp2e as i32).unsigned_abs() as u16;
        // Bow / boomerang aiming: not held.
        if self.unk_664.is_some() {
            return self.func_8083FC68(env, *speed, *yaw);
        }
        self.func_8083DC54(env);
        if *speed != 0.0 && sp2c < 6000 {
            return 1;
        } else if *speed > sin_s((0x4000u16.wrapping_sub(sp2c >> 1)) as i16) * 200.0 {
            return -1;
        }
        0
    }

    /// `func_8083CEAC`: lock on (standing).
    fn func_8083CEAC(&mut self, data: &GameData) {
        self.setup_action(data, Action::TargetIdle, 1);
        let a = self.anim(data, group::TARGET_ENTER);
        self.func_80832B0C(data, a);
        self.unk_850 = 1;
    }

    /// `func_80839F30`: parallel mode (standing).
    fn func_80839F30(&mut self, data: &GameData) {
        self.setup_action(data, Action::ParallelIdle, 1);
        let a = self.anim(data, group::WAIT);
        self.func_80832B0C(data, a);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_80839E88`: back to the locked-on wait, keeping the leading foot.
    fn func_80839E88(&mut self, data: &GameData) {
        self.setup_action(data, Action::TargetIdle, 1);
        let anim = if self.unk_870 < 0.5 {
            self.unk_870 = 0.0;
            self.func_808334E4(data)
        } else {
            self.unk_870 = 1.0;
            self.func_80833528(data)
        };
        self.unk_874 = self.unk_870;
        self.skel.play_loop(data, anim);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_80839F90`.
    fn func_80839F90(&mut self, data: &GameData) {
        if self.state1 & STATE1_4 != 0 {
            self.func_80839E88(data);
        } else if self.func_80833B2C() {
            self.func_80839F30(data);
        } else {
            self.func_80853080(data);
        }
    }

    /// `func_8083CF10`: target lost while standing.
    fn func_8083CF10(&mut self, data: &GameData) {
        if self.linear_velocity != 0.0 {
            self.func_8083C858(data);
        } else {
            self.func_8083CE0C(data);
        }
    }

    /// `func_8083CE0C`: end the lock-on stance.
    fn func_8083CE0C(&mut self, data: &GameData) {
        self.setup_action(data, Action::StandingStill, 1);
        let a = if self.unk_870 < 0.5 { self.anim(data, group::TARGET_EXIT_L) } else { self.anim(data, group::TARGET_EXIT_R) };
        self.skel.play_once(data, a);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_8083CBF0`: step back from the target.
    fn func_8083CBF0(&mut self, data: &GameData, yaw: i16) {
        self.setup_action(data, Action::TargetBackwalk, 1);
        let a = data.anim("link_anchor_back_walk");
        let last = data.anims[a].last_frame();
        self.skel.change(data, a, 2.2, 0.0, last, ANIMMODE_ONCE, -6.0);
        self.linear_velocity = 8.0;
        self.current_yaw = yaw;
    }

    /// `func_8083CC9C`: start sidestepping.
    fn func_8083CC9C(&mut self, data: &GameData) {
        self.setup_action(data, Action::Sidestep, 1);
        let a = self.anim(data, group::SIDESTEP_L);
        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -6.0);
        self.unk_868 = 0.0;
    }

    /// `func_8083CB2C`: start walking backwards in parallel mode.
    fn func_8083CB2C(&mut self, data: &GameData, yaw: i16) {
        self.setup_action(data, Action::ParallelBackwalk, 1);
        self.skel.copy_joint_to_morph();
        self.unk_864 = 0.0;
        self.unk_868 = 0.0;
        self.current_yaw = yaw;
    }

    /// `func_8083CB94`: start walking sideways in parallel mode.
    fn func_8083CB94(&mut self, data: &GameData) {
        self.setup_action(data, Action::ParallelWalk, 1);
        let a = self.anim(data, group::WALK);
        self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -6.0);
    }

    /// `func_80835DAC`: switch action keeping the animation-driven movement flags.
    fn func_80835DAC(&mut self, data: &GameData, action: Action, flags: i32) {
        let t = self.skel.move_flags;
        self.skel.move_flags = 0;
        self.setup_action(data, action, flags);
        self.skel.move_flags = t;
    }

    /// `func_80840450`: locked on, standing (and turning to face the target).
    fn func_80840450(&mut self, env: &Env) {
        let data = env.data;
        // PLAYER_STATE3_3 (sword swing recovery): no melee weapon in hand yet.
        self.state3 &= !STATE3_3;
        if self.unk_850 != 0 {
            if self.skel.update(data) {
                self.func_80832DBC();
                let a = self.func_808334E4(data);
                self.skel.play_loop(data, a);
                self.unk_850 = 0;
                self.state3 &= !STATE3_3;
            }
            self.func_80833C3C();
        } else {
            self.func_808401B0(data);
        }
        self.func_8083721C();
        if !self.func_80837348(env, Self::D_808543E0, true) {
            // func_80834B5C (shield up) is never the upper-body action here.
            if !self.func_80833B54_env(env) && !self.func_80833B2C() {
                self.func_8083CF10(data);
                return;
            }
            let (_, sp44, sp42) = self.func_80837268(env, 0.0);
            let t1 = self.func_8083FC68(env, sp44, sp42);
            if t1 > 0 {
                self.func_8083C8DC(data, sp42);
                return;
            }
            if t1 < 0 {
                self.func_8083CBF0(data, sp42);
                return;
            }
            if sp44 > 4.0 {
                self.func_8083CC9C(data);
                return;
            }
            self.func_8084029C(self.linear_velocity * 0.3 + 1.0);
            self.func_80840138(sp44, sp42);
            let t2 = self.unk_868 as u32;
            if t2 < 6 || t2.wrapping_sub(0xE) < 6 {
                step_to_f(&mut self.linear_velocity, 0.0, 1.5);
                return;
            }
            let t3 = sp42.wrapping_sub(self.current_yaw);
            let t4 = abs16(t3);
            if t4 > 0x4000 {
                if step_to_f(&mut self.linear_velocity, 0.0, 1.5) {
                    self.current_yaw = sp42;
                }
                return;
            }
            asym_step_to_f(&mut self.linear_velocity, sp44 * 0.3, 2.0, 1.5);
            if self.state3 & STATE3_3 == 0 {
                scaled_step_to_s(&mut self.current_yaw, sp42, (t4 as f32 * 0.1) as i16);
            }
        }
    }

    /// `func_808407CC`: parallel mode, standing.
    fn func_808407CC(&mut self, env: &Env) {
        let data = env.data;
        if self.skel.update(data) {
            self.func_80832DBC();
            let a = self.anim(data, group::WAIT);
            self.skel.play_once(data, a);
        }
        self.func_8083721C();
        if !self.func_80837348(env, Self::D_808543E8, true) {
            if self.func_80833B54_env(env) {
                self.func_8083CEAC(data);
                return;
            }
            if !self.func_80833B2C() {
                self.func_80835DAC(data, Action::StandingStill, 1);
                self.current_yaw = self.actor.shape_rot.y;
                return;
            }
            let (_, mut sp3c, mut sp3a) = self.func_80837268(env, 0.0);
            let t1 = self.func_8083FD78(env, &mut sp3c, &mut sp3a);
            if t1 > 0 {
                self.func_8083C8DC(data, sp3a);
                return;
            }
            if t1 < 0 {
                self.func_8083CB2C(data, sp3a);
                return;
            }
            if sp3c > 4.9 {
                self.func_8083CC9C(data);
                self.func_80833C3C();
                return;
            }
            if sp3c != 0.0 {
                self.func_8083CB94(data);
                return;
            }
            let t2 = sp3a.wrapping_sub(self.actor.shape_rot.y);
            if abs16(t2) > 800 {
                self.func_8083CD54(data, sp3a);
            }
        }
    }

    /// `func_8084227C`: running forward while targeting.
    fn func_8084227C(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        self.func_80841EE4(data);
        if !self.func_80837348(env, Self::D_80854430, true) {
            if !self.func_80833C04(env) {
                self.func_8083C858(data);
                return;
            }
            let (_, mut sp2c, mut sp2a) = self.func_80837268(env, 0.0);
            if !self.func_8083C484(&mut sp2c, &mut sp2a) {
                let stop = if self.func_80833B2C() {
                    sp2c != 0.0 && self.func_8083FD78(env, &mut sp2c, &mut sp2a) <= 0
                } else {
                    self.func_8083FC68(env, sp2c, sp2a) <= 0
                };
                if stop {
                    self.func_80839F90(data);
                    return;
                }
                self.func_8083DF68(sp2c, sp2a);
                self.func_8083DDC8(env);
                if self.linear_velocity == 0.0 && sp2c == 0.0 {
                    self.func_80839F90(data);
                }
            }
        }
    }

    /// `func_80841860`: sidestep animation, the two sidestep cycles blended by `unk_870`.
    fn func_80841860(&mut self, data: &GameData) {
        let a = self.anim(data, group::SIDESTEP_R);
        let b = self.anim(data, group::SIDESTEP_L);
        self.skel.animation = a;
        let r = |n| self.regs.reg(n) as f32 / 1000.0;
        self.func_8084029C(r(30) + r(32) * self.linear_velocity);
        let frame = self.unk_868 * (16.0 / 29.0);
        self.skel.blend_to_joint(data, b, frame, a, frame, self.unk_870);
    }

    /// `func_8084193C`: sidestepping.
    fn func_8084193C(&mut self, env: &Env) {
        let data = env.data;
        self.func_80841860(data);
        if !self.func_80837348(env, Self::D_80854408, true) {
            if !self.func_80833C04(env) {
                self.func_8083C858(data);
                return;
            }
            let (_, mut sp3c, mut sp3a) = self.func_80837268(env, 0.0);
            let t1 = if self.func_80833B2C() { self.func_8083FD78(env, &mut sp3c, &mut sp3a) } else { self.func_8083FC68(env, sp3c, sp3a) };
            if t1 > 0 {
                self.func_8083C858(data);
                return;
            }
            if t1 < 0 {
                if self.func_80833B2C() {
                    self.func_8083CB2C(data, sp3a);
                } else {
                    self.func_8083CBF0(data, sp3a);
                }
                return;
            }
            if self.linear_velocity < 3.6 && sp3c < 4.0 {
                if self.state1 & STATE1_4 == 0 && self.func_80833B2C() {
                    self.func_8083CB94(data);
                } else {
                    self.func_80839F90(data);
                }
                return;
            }
            self.func_80840138(sp3c, sp3a);
            let t2 = sp3a.wrapping_sub(self.current_yaw);
            let t3 = abs16(t2);
            if t3 > 0x4000 {
                if step_to_f(&mut self.linear_velocity, 0.0, 3.0) {
                    self.current_yaw = sp3a;
                }
                return;
            }
            asym_step_to_f(&mut self.linear_velocity, sp3c * 0.9, 2.0, 3.0);
            scaled_step_to_s(&mut self.current_yaw, sp3a, (t3 as f32 * 0.1) as i16);
        }
    }

    /// `func_808423EC`: the step back from a locked target.
    fn func_808423EC(&mut self, env: &Env) {
        let data = env.data;
        let sp34 = self.skel.update(data);
        if !self.func_80837348(env, Self::D_80854408, true) {
            if !self.func_80833C04(env) {
                self.func_8083C858(data);
                return;
            }
            let (_, sp30, sp2e) = self.func_80837268(env, 0.0);
            if self.skel.morph_weight == 0.0 && self.skel.cur_frame > 5.0 {
                self.func_8083721C();
                if self.skel.cur_frame > 10.0 && self.func_8083FC68(env, sp30, sp2e) < 0 {
                    self.func_8083CBF0(data, sp2e);
                    return;
                }
                if sp34 {
                    // func_8083CD00.
                    self.setup_action(data, Action::TargetBackBrake, 1);
                    let a = data.anim("link_anchor_back_brake");
                    self.skel.play_once_set_speed(data, a, 2.0);
                }
            }
        }
    }

    /// `func_8084251C`: braking after the step back.
    fn func_8084251C(&mut self, env: &Env) {
        let data = env.data;
        let sp34 = self.skel.update(data);
        self.func_8083721C();
        if !self.func_80837348(env, Self::D_80854440, true) {
            let (_, sp30, sp2e) = self.func_80837268(env, 0.0);
            if self.linear_velocity == 0.0 {
                self.current_yaw = self.actor.shape_rot.y;
                if self.func_8083FC68(env, sp30, sp2e) > 0 {
                    self.func_8083C858(data);
                    return;
                }
                if sp30 != 0.0 || sp34 {
                    self.func_80839F90(data);
                }
            }
        }
    }

    /// `func_80840DE4`: walking sideways in parallel mode. The side-walk cycle plays at a speed set
    /// by `linearVelocity × MREG(95) / 100`, signed by which way Player moves.
    fn func_80840DE4(&mut self, env: &Env) {
        let data = env.data;
        self.skel.mode = ANIMMODE_LOOP;
        self.skel.animation = self.anim(data, group::PARALLEL_SIDEWALK);
        let (frames, coeff) = (29.0, self.regs.mreg(95) as f32 / 100.0);
        self.skel.anim_length = frames;
        self.skel.end_frame = frames - 1.0;
        let dir = if self.current_yaw.wrapping_sub(self.actor.shape_rot.y) >= 0 { 1.0 } else { -1.0 };
        self.skel.play_speed = dir * (self.linear_velocity * coeff);
        self.skel.update(data);
        if self.skel.on_frame(0.0) || self.skel.on_frame(frames * 0.5) {
            let v = self.linear_velocity;
            self.func_808327F8(v);
        }
        if !self.func_80837348(env, Self::D_808543F4, true) {
            if self.func_80833B54_env(env) {
                self.func_8083CEAC(data);
                return;
            }
            if !self.func_80833B2C() {
                self.func_80853080(data);
                return;
            }
            let (_, mut sp44, mut sp42) = self.func_80837268(env, 0.0);
            let t1 = self.func_8083FD78(env, &mut sp44, &mut sp42);
            if t1 > 0 {
                self.func_8083C8DC(data, sp42);
                return;
            }
            if t1 < 0 {
                self.func_8083CB2C(data, sp42);
                return;
            }
            if sp44 > 4.9 {
                self.func_8083CC9C(data);
                self.func_80833C3C();
                return;
            }
            if sp44 == 0.0 && self.linear_velocity == 0.0 {
                self.func_80839F30(data);
                return;
            }
            let t2 = sp42.wrapping_sub(self.current_yaw);
            let t3 = abs16(t2);
            if t3 > 0x4000 {
                if step_to_f(&mut self.linear_velocity, 0.0, 1.5) {
                    self.current_yaw = sp42;
                }
                return;
            }
            asym_step_to_f(&mut self.linear_velocity, sp44 * 0.4, 1.5, 1.5);
            scaled_step_to_s(&mut self.current_yaw, sp42, (t3 as f32 * 0.1) as i16);
        }
    }

    /// `func_80841138`: parallel back walk/run animation (like `func_80841EE4` with the back cycles).
    fn func_80841138(&mut self, data: &GameData) {
        let r = |n| self.regs.reg(n) as f32 / 1000.0;
        let back = self.anim(data, group::PARALLEL_BACKWALK);
        let temp1;
        if self.unk_864 < 1.0 {
            self.func_8084029C(r(35));
            self.skel.load_to_joint(data, back, self.unk_868);
            self.unk_864 = (self.unk_864 + UPDATE_SCALE).min(1.0);
            temp1 = self.unk_864;
        } else {
            let temp2 = self.linear_velocity - (self.regs.reg(48) as f32 / 100.0);
            if temp2 < 0.0 {
                temp1 = 1.0;
                self.func_8084029C(r(35) + r(36) * self.linear_velocity);
                self.skel.load_to_joint(data, back, self.unk_868);
            } else {
                let mut t = r(37) * temp2;
                if t < 1.0 {
                    self.func_8084029C(r(35) + r(36) * self.linear_velocity);
                } else {
                    t = 1.0;
                    self.func_8084029C(1.2 + r(38) * temp2);
                }
                temp1 = t;
                self.skel.load_to_morph(data, back, self.unk_868);
                let run = data.anim("link_normal_back_run");
                self.skel.load_to_joint(data, run, self.unk_868 * (16.0 / 29.0));
            }
        }
        if temp1 < 1.0 {
            self.skel.interp_joint_morph(1.0 - temp1);
        }
    }

    /// `func_808414F8`: walking/running backwards in parallel mode.
    fn func_808414F8(&mut self, env: &Env) {
        let data = env.data;
        self.func_80841138(data);
        if !self.func_80837348(env, Self::D_80854400, true) {
            if !self.func_80833C04(env) {
                let y = self.current_yaw;
                self.func_8083C8DC(data, y);
                return;
            }
            let (_, mut sp34, mut sp32) = self.func_80837268(env, 0.0);
            let sp2c = self.func_8083FD78(env, &mut sp34, &mut sp32);
            if sp2c >= 0 {
                // func_80841458.
                let handled = if self.linear_velocity > 6.0 {
                    // func_8084140C.
                    self.setup_action(data, Action::ParallelBackBrake, 1);
                    let a = data.anim("link_normal_back_brake");
                    self.func_80832B0C(data, a);
                    true
                } else if sp34 != 0.0 {
                    if self.func_8083721C() {
                        // func_80841458 also resets *arg2 to currentYaw; it isn't read again.
                        sp34 = 0.0;
                        false
                    } else {
                        true
                    }
                } else {
                    false
                };
                if !handled {
                    if sp2c != 0 {
                        self.func_8083C858(data);
                    } else if sp34 > 4.9 {
                        self.func_8083CC9C(data);
                    } else {
                        self.func_8083CB94(data);
                    }
                }
            } else {
                let sp2a = sp32.wrapping_sub(self.current_yaw);
                asym_step_to_f(&mut self.linear_velocity, sp34 * 1.5, 1.5, 2.0);
                scaled_step_to_s(&mut self.current_yaw, sp32, (sp2a as f32 * 0.1) as i16);
                if sp34 == 0.0 && self.linear_velocity == 0.0 {
                    self.func_80839F30(data);
                }
            }
        }
    }

    /// `func_8084170C`: braking out of a parallel back run.
    fn func_8084170C(&mut self, env: &Env) {
        let data = env.data;
        let sp34 = self.skel.update(data);
        self.func_8083721C();
        if !self.func_80837348(env, Self::D_80854400, true) {
            let (_, mut sp30, mut sp2e) = self.func_80837268(env, 0.0);
            if self.linear_velocity == 0.0 {
                self.current_yaw = self.actor.shape_rot.y;
                if self.func_8083FD78(env, &mut sp30, &mut sp2e) > 0 {
                    self.func_8083C858(data);
                } else if sp30 != 0.0 || sp34 {
                    // func_808416C0.
                    self.setup_action(data, Action::ParallelBackBrakeEnd, 1);
                    let a = data.anim("link_normal_back_brake_end");
                    self.skel.play_once(data, a);
                }
            }
        }
    }

    /// `func_808417FC`: end of the parallel back brake.
    fn func_808417FC(&mut self, env: &Env) {
        let data = env.data;
        let sp1c = self.skel.update(data);
        if !self.func_80837348(env, Self::D_80854400, true) && sp1c {
            self.func_80839F30(data);
        }
    }

    /// `func_8083BDBC` (interrupt 10): A while targeting: a side hop or backflip away from the
    /// stick's forward, or a roll forward (a jump slash with a sword: not ported).
    fn func_8083BDBC(&mut self, env: &Env) -> bool {
        let floor_effect = self.actor.floor_poly.map(|p| env.col.floor_effect(p)).unwrap_or(0);
        // Room behaviour type 1 is 0 here (not ROOM_BEHAVIOR_TYPE1_2).
        if self.input.press.held(BTN_A) && self.s.floor_type != FLOOR_TYPE_7 && floor_effect != 1 {
            let sp2c = self.stick_dir();
            if sp2c <= 0 {
                if self.func_80833BCC() {
                    self.func_8083BC04(env.data);
                    return true;
                }
            } else {
                self.func_8083BCD0(env.data, sp2c);
                return true;
            }
        }
        false
    }

    /// `func_8083BCD0`: hop in direction `arg2` (1 left, 2 back = backflip, 3 right).
    fn func_8083BCD0(&mut self, data: &GameData, arg2: i8) {
        let side = arg2 & 1 != 0;
        let a = data.side_hop_anims[arg2 as usize][0];
        self.func_80838940(data, Some(a), if !side { 5.8 } else { 3.5 }, NA_SE_VO_LI_SWORD_N);
        self.unk_850 = 1;
        self.unk_84F = arg2;
        self.current_yaw = self.actor.shape_rot.y.wrapping_add(((arg2 as i32) << 14) as i16);
        self.linear_velocity = if !side { 6.0 } else { 8.5 };
        self.state2 |= STATE2_19;
        self.func_8002F7DC(if ((arg2 as i32) << 0xE) == 0x8000 { NA_SE_PL_ROLL } else { NA_SE_PL_SKIP });
    }

    // ================================================================================
    // Items and the sword (func_80836670, func_80835F44, func_808502D0)

    /// `func_80833350`: -1 if playing the wait, 1 + index if an idle fidget (`D_80853D7C`), else 0.
    fn func_80833350(&self, data: &GameData) -> i32 {
        if self.anim(data, group::WAIT) == self.skel.animation {
            return -1;
        }
        for (i, a) in data.idle_variants.iter().flat_map(|p| p.iter()).enumerate() {
            if *a == self.skel.animation {
                return i as i32 + 1;
            }
        }
        0
    }

    /// `func_80836670`: run the upper-body action; while it's active, its animation replaces the
    /// upper body (`D_80853410`), or the whole body when standing on the wait/fidget.
    fn func_80836670(&mut self, env: &Env) -> bool {
        let data = env.data;
        // Hookshot flight: not held.
        if self.func_808365C8() {
            self.func_80834298(env);
        }
        if !self.run_upper(env) {
            return false;
        }
        // unk_830 (blend back out of skelAnime2) is only ever set to 0 in this decomp.
        if self.func_80833350(data) == 0 || self.linear_velocity != 0.0 {
            let mask = data.items.upper_body;
            let src = self.skel2.joint;
            self.skel.request_copy_external(&src, Some(mask));
        } else {
            let src = self.skel2.joint;
            self.skel.request_copy_external(&src, None);
        }
        true
    }

    /// `func_808365C8`: the item system may run.
    fn func_808365C8(&self) -> bool {
        self.upper != UpperAction::Change || self.held_item == Some(self.held_item_ap)
    }

    fn run_upper(&mut self, env: &Env) -> bool {
        match self.upper {
            // func_8083485C / func_808349DC: func_80834758 (shield up on R) is not ported.
            UpperAction::Default => false,
            UpperAction::Sword => {
                // func_8083499C.
                if self.state1 & STATE1_8 != 0 {
                    self.func_808340DC(env.data);
                    true
                } else {
                    false
                }
            }
            UpperAction::Change => self.func_80834A2C(env),
        }
    }

    /// `D_80853EDC[actionParam]`: the upper-body action for a held item.
    fn upper_for(&self, data: &GameData, ap: i32) -> UpperAction {
        let it = &data.items;
        if (it.ap("SWORD_MASTER")..=it.ap("SWORD_BGS")).contains(&ap) { UpperAction::Sword } else { UpperAction::Default }
    }

    /// `func_80833638`.
    fn func_80833638(&mut self, f: UpperAction) {
        self.upper = f;
        self.unk_830 = 0.0;
        self.func_808326F0();
    }

    /// `func_808326F0`: the idle fidgets' voices stop (`D_8085361C`, the age's bank).
    fn func_808326F0(&mut self) {
        for id in [NA_SE_VO_LI_SWEAT, NA_SE_VO_LI_SNEEZE, NA_SE_VO_LI_RELAX, NA_SE_VO_LI_FALL_L] {
            self.sfx(PlayerSfx::StopById(id.wrapping_add(self.age.climb.unk_92)));
        }
    }

    /// `func_80834298`: B / C buttons, then a pending change.
    fn func_80834298(&mut self, env: &Env) {
        if self.state1 & STATE1_8 == 0 && (self.held_item_ap == self.item_ap || self.state1 & STATE1_22 != 0) {
            self.func_80833DF8(env);
        }
        if self.state1 & STATE1_8 != 0 {
            self.func_808340DC(env.data);
        }
    }

    /// `func_80833DF8`: the item buttons. B's item is the save's (`B_BTN_ITEM`, through
    /// `func_80833CDC`): nothing happens on a B with no item. The C buttons' items (in the save,
    /// on the HUD) aren't used here: their actions (the slingshot, bombs, the ocarina, ...)
    /// aren't ported. A held item no button has any more is put away.
    fn func_80833DF8(&mut self, env: &Env) {
        use oot_game::item::ITEM_NONE_FE;
        // (currentMask: masks aren't ported.)
        if self.state1 & (STATE1_11 | STATE1_29) != 0 {
            return;
        }
        // (func_8008F128: the hookshot's and boomerang's flight: not held.)
        let items = &env.data.items;
        let (b_item, c_items) = {
            let io = env.io.borrow();
            (io.save.b_btn_item(), [io.save.c_btn_item(0), io.save.c_btn_item(1), io.save.c_btn_item(2)])
        };
        // func_80833C50: the button's item is the one in hand.
        let holds = |item: u8| item < ITEM_NONE_FE && items.item_to_action_param(item) == self.item_ap;
        if self.item_ap >= items.ap("FISHING_POLE") && !holds(b_item) && !c_items.iter().any(|&c| holds(c)) {
            self.func_80835F44(env.data, 0);
            return;
        }
        // D_80854388: B, C-left, C-down, C-right; only B's item is used (func_80833CDC(0)).
        if self.input.press.held(eng_input::pad::BTN_B) {
            if b_item < ITEM_NONE_FE {
                let ap = items.item_to_action_param(b_item);
                self.func_80835F44(env.data, ap);
            }
        } else if self.input.cur.held(eng_input::pad::BTN_B) && b_item < ITEM_NONE_FE && items.item_to_action_param(b_item) == self.held_item_ap {
            self.s.d_80853618 = true;
        }
    }

    /// `Player_ActionToMeleeWeapon`.
    fn melee_weapon(ap: i32) -> i32 {
        let m = ap - 2;
        if m > 0 && m < 6 { m } else { 0 }
    }

    /// `func_80835F44`: use / change to the item `ap` (as its action param; 0 = put away). For
    /// the sword this starts the change animation, or, with the sword already in hand, flags the
    /// press (`D_80853614`) for an attack.
    fn func_80835F44(&mut self, data: &GameData, ap: i32) {
        let ok = (self.held_item_ap == self.item_ap && (self.state1 & STATE1_22 == 0 || Self::melee_weapon(ap) != 0 || ap == 0))
            || (self.item_ap < 0 && (Self::melee_weapon(ap) != 0 || ap == 0));
        if !ok || !(ap == 0 || self.state1 & STATE1_27 == 0) {
            return;
        }
        // Sticks, nuts, lens, magic, masks, ocarina and bottles aren't on the buttons here.
        if ap != self.held_item_ap {
            let it = &data.items;
            self.next_model_group = it.action_model_group[ap as usize];
            let next_type = it.model_group_anim_type[self.next_model_group];
            let cur_type = it.model_group_anim_type[self.model_group];
            if self.held_item_ap >= 0 && self.held_item != Some(ap) && it.change_matrix[cur_type][next_type] != 0 {
                self.held_item = Some(ap);
                self.state1 |= STATE1_8;
            } else {
                self.func_80833664(data, ap);
            }
        } else {
            self.s.d_80853614 = true;
            self.s.d_80853618 = true;
        }
    }

    /// `func_80833664`: swap the item now, keeping the animation group.
    fn func_80833664(&mut self, data: &GameData, ap: i32) {
        let cur = self.skel.animation;
        let groups = data.anim_group_names.len();
        let found = (0..groups).find(|&g| data.player_anim(g, self.model_anim_type) == cur);
        self.state1 &= !(STATE1_3 | STATE1_24);
        self.func_8083399C(data, ap);
        if let Some(g) = found {
            self.skel.animation = data.player_anim(g, self.model_anim_type);
        }
    }

    /// `func_8083399C`: the item is in hand.
    fn func_8083399C(&mut self, data: &GameData, ap: i32) {
        self.held_item_ap = ap;
        self.item_ap = ap;
        self.model_group = self.next_model_group;
        self.state1 &= !(STATE1_3 | STATE1_24);
        // D_80853FE8[ap] is func_80833770 (empty) for no item and the swords.
        self.player_set_model_group(data, self.model_group);
    }

    /// `Player_SetModelGroup`: the model group decides the animation type.
    fn player_set_model_group(&mut self, data: &GameData, mg: usize) {
        self.model_group = mg;
        let it = &data.items;
        let mut t = if mg == it.model_group("CHILD_HYLIAN_SHIELD") { 0 } else { it.model_group_anim_type[mg] };
        if t < 3 && self.current_shield == 0 {
            t = 0;
        }
        self.model_anim_type = t;
    }

    /// `func_808340DC`: start the change animation on `skelAnime2`.
    fn func_808340DC(&mut self, data: &GameData) {
        let sp37 = self.held_item.unwrap_or(0);
        self.func_80833638(UpperAction::Change);
        let it = &data.items;
        let next_type = it.model_group_anim_type[self.next_model_group];
        let sp38 = it.change_matrix[it.model_group_anim_type[self.model_group]][next_type];
        // Bottle / boomerang use D_808540F4[13]: not on the buttons.
        self.unk_15A = sp38.unsigned_abs() as usize;
        let mut anim = it.change_anims[self.unk_15A].0;
        if anim == data.anim("link_normal_fighter2free") && self.current_shield == 0 {
            anim = data.anim("link_normal_free2fighter_free");
        }
        let last = data.anims[anim].last_frame();
        let (mut speed, start, end) = if sp38 >= 0 { (1.2, 0.0, last) } else { (-1.2, last, 0.0) };
        if sp37 != 0 {
            speed *= 2.0;
        }
        self.skel2.change(data, anim, speed, start, end, ANIMMODE_ONCE, 0.0);
        self.state1 &= !STATE1_8;
    }

    /// `func_80834A2C`: the change playing. On its swap frame the item goes into the hand; once
    /// it's in hand a B press (or any press for a non-two-handed item) ends the change.
    fn func_80834A2C(&mut self, env: &Env) -> bool {
        let data = env.data;
        let done = self.skel2.update(data);
        let in_hand = self.held_item == Some(self.held_item_ap) && {
            self.s.d_80853614 = self.s.d_80853614 || self.model_anim_type != 3;
            self.s.d_80853614
        };
        if done || in_hand {
            let f = self.upper_for(data, self.held_item_ap);
            self.func_80833638(f);
            self.unk_6AC = 0;
            self.s.d_80853618 = self.s.d_80853614;
            return self.run_upper(env);
        }
        if self.func_80833350(data) != 0 {
            self.func_808348EC(env);
            let a = self.anim(data, group::WAIT);
            self.skel.play_once(data, a);
            self.unk_6AC = 0;
        } else {
            self.func_808348EC(env);
        }
        true
    }

    /// `func_808348EC`: swap the item on the change animation's swap frame.
    fn func_808348EC(&mut self, env: &Env) {
        let mut t = env.data.items.change_anims[self.unk_15A].1;
        if self.skel2.play_speed < 0.0 {
            t -= 1.0;
        }
        if self.skel2.on_frame(t) {
            // func_80834594: the sword sounds, then func_80835F44(heldItemId).
            let ap = self.held_item.unwrap_or(0);
            self.func_80835F44(env.data, ap);
        }
        self.func_80833B54_env(env);
    }

    /// `func_80850224` (interrupt 7): B with a melee weapon in hand attacks.
    fn func_80850224(&mut self, env: &Env) -> bool {
        // func_8083C6B8 (bottles, the fishing rod): not held.
        if self.func_8083BB20() {
            let sp24 = self.func_80837818(env);
            self.func_80837948(env.data, sp24);
            if sp24 >= env.data.items.mwa("SPIN_ATTACK_1H") {
                self.state2 |= STATE2_17;
                // func_80837530 (spin attack effects): not ported.
            }
            return true;
        }
        false
    }

    /// `func_8083BB20`.
    fn func_8083BB20(&self) -> bool {
        self.state1 & STATE1_22 == 0 && Self::melee_weapon(self.held_item_ap) != 0 && self.s.d_80853614
    }

    /// `func_808375D8`: the stick spun a quarter-turn each of the last 3 frames (a quick spin).
    fn func_808375D8(&self) -> bool {
        let mut sp = [0i32; 4];
        for (i, v) in self.unk_847.iter().enumerate() {
            if *v < 0 {
                return false;
            }
            sp[i] = (*v as i32 * 2) as i8 as i32;
        }
        let t1 = (sp[0] - sp[1]) as i8 as i32;
        if t1.abs() < 10 {
            return false;
        }
        for i in 1..3 {
            let t2 = (sp[i] - sp[i + 1]) as i8 as i32;
            if t2.abs() < 10 || t2 * t1 < 0 {
                return false;
            }
        }
        true
    }

    /// `func_80837818`: which attack, by stick direction (`D_80854480`).
    fn func_80837818(&mut self, env: &Env) -> usize {
        let it = &env.data.items;
        let sp1c = self.stick_dir();
        let sp18 = if self.func_808375D8() {
            it.mwa("SPIN_ATTACK_1H")
        } else if sp1c < 0 {
            if self.func_80833BCC() { it.mwa("FORWARD_SLASH_1H") } else { it.mwa("RIGHT_SLASH_1H") }
        } else {
            let mut a = it.attack_by_dir[sp1c as usize];
            if a == it.mwa("STAB_1H") {
                self.state2 |= STATE2_30;
                if !self.func_80833BCC() {
                    a = it.mwa("FORWARD_SLASH_1H");
                }
            }
            a
        };
        // Deku sticks and the two-handed Biggoron's Sword: not held.
        sp18
    }

    /// `func_80837948`: start attack `arg2`; the third of the same attack in a row is its combo.
    fn func_80837948(&mut self, data: &GameData, mut arg2: usize) {
        let it = &data.items;
        let (flip0, jump1) = (it.mwa("FLIPSLASH_START"), it.mwa("JUMPSLASH_FINISH"));
        self.setup_action(data, Action::Attack, 0);
        self.unk_844 = 8;
        if !(arg2 >= it.mwa("FLIPSLASH_FINISH") && arg2 <= jump1) {
            self.func_80832318();
        }
        if arg2 != self.melee_weapon_animation || self.unk_845 >= 3 {
            self.unk_845 = 0;
        }
        self.unk_845 += 1;
        if self.unk_845 >= 3 {
            arg2 += 2;
        }
        self.melee_weapon_animation = arg2;
        let a = it.attacks[arg2].anim;
        self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
        if arg2 != flip0 && arg2 != it.mwa("JUMPSLASH_START") {
            self.func_80832F54(0x209);
        }
        self.current_yaw = self.actor.shape_rot.y;
        // Player_HoldsBrokenKnife: the Biggoron's Sword isn't held.
        let row = (Self::melee_weapon(self.held_item_ap) - 1).clamp(0, 4) as usize;
        let jump = arg2 >= flip0 && arg2 <= jump1;
        let dmg_flags = D_80854488[row][usize::from(jump)];
        self.func_80837918(0, dmg_flags);
        self.func_80837918(1, dmg_flags);
    }

    /// `func_80837918`: the sword quad's damage type for this attack.
    fn func_80837918(&mut self, quad: usize, dmg_flags: u32) {
        let q = &mut self.melee_weapon_quads[quad];
        q.info.toucher.dmg_flags = dmg_flags;
        q.info.toucher_flags = if dmg_flags == cc::DMG_DEKU_STICK { cc::TOUCH_ON | cc::TOUCH_NEAREST | cc::TOUCH_SFX_WOOD } else { cc::TOUCH_ON | cc::TOUCH_NEAREST };
    }

    /// `func_80832318`: weapon inactive.
    fn func_80832318(&mut self) {
        self.state2 &= !STATE2_17;
        self.melee_weapon_state = 0;
        for w in &mut self.melee_weapon_info {
            w.active = false;
        }
    }

    /// `func_80832F54`: animation-driven movement with `flags` (`ANIM_FLAG_*`, 0x200 scales
    /// the start translation by `ageProperties->unk_08`).
    fn func_80832F54(&mut self, flags: u16) {
        if flags & 0x200 != 0 {
            // func_80832D20.
            self.func_80832CFC();
            let k = self.age.translation_scale;
            self.skel.prev_transl = self.skel.prev_transl.map(|v| (v as f32 * k) as i16);
        } else if flags & 0x100 != 0 || self.skel.move_flags != 0 {
            self.func_80832CFC();
        } else {
            self.skel.prev_transl = self.skel.joint[0];
            self.skel.prev_rot = self.actor.shape_rot.y;
        }
        self.skel.move_flags = flags;
        self.func_80832210();
        self.skel.disable_queue();
    }

    /// `func_8084285C`: the weapon is active from `arg1` to `arg3` (`func_80833A20`).
    fn func_8084285C(&mut self, env: &Env, arg1: f32, arg2: f32, arg3: f32) -> bool {
        let f = self.skel.cur_frame;
        if arg1 <= f && f <= arg3 {
            self.func_80833A20(env, if arg2 <= f { 1 } else { -1 });
            return true;
        }
        self.func_80832318();
        false
    }

    /// `func_80833A20`: the weapon's state; as a swing starts, its sound (the swing, the hard
    /// swing from the fourth chained, the hammer, none for a spin) and Link's voice (not for
    /// the flip and jump slashes).
    fn func_80833A20(&mut self, env: &Env, new_melee_weapon_state: i8) {
        if self.melee_weapon_state == 0 {
            let items = &env.data.items;
            let ap = self.held_item_ap;
            let sword_health = env.io.borrow().save.sword_health;
            let mut item_sfx = if ap == items.ap("SWORD_BGS") && sword_health > 0 { NA_SE_IT_HAMMER_SWING } else { NA_SE_IT_SWORD_SWING };
            let mut voice_sfx = NA_SE_VO_LI_SWORD_N;
            // PLAYER_MWA_SPIN_ATTACK_1H (24), PLAYER_MWA_FLIPSLASH_START (16) to
            // PLAYER_MWA_JUMPSLASH_FINISH (19) (z64player.h).
            if ap == items.ap("HAMMER") {
                item_sfx = NA_SE_IT_HAMMER_SWING;
            } else if self.melee_weapon_animation >= 24 {
                item_sfx = 0;
                voice_sfx = NA_SE_VO_LI_SWORD_L;
            } else if self.unk_845 >= 3 {
                item_sfx = NA_SE_IT_SWORD_SWING_HARD;
                voice_sfx = NA_SE_VO_LI_SWORD_L;
            }
            if item_sfx != 0 {
                self.func_808328EC(item_sfx);
            }
            if !(16..=19).contains(&self.melee_weapon_animation) {
                self.func_80832698(voice_sfx);
            }
        }
        self.melee_weapon_state = new_melee_weapon_state;
    }

    /// `func_8083C50C`: B released during the attack.
    fn func_8083C50C(&mut self) {
        if self.unk_844 > 0 && !self.input.cur.held(eng_input::pad::BTN_B) {
            self.unk_844 = -self.unk_844;
        }
    }

    /// `func_808502D0`: attacking. At the end another B press chains (`func_80850224`);
    /// otherwise the end animation plays into the (targeting) stance.
    fn func_808502D0(&mut self, env: &Env) {
        let data = env.data;
        let a = data.items.attacks[self.melee_weapon_animation];
        self.state2 |= STATE2_5;
        // func_80842DF4 (weapon hits and recoil off walls): no actor/weapon collision.
        self.func_8084285C(env, 0.0, a.active_start, a.active_end);
        if self.state2 & STATE2_30 != 0 && self.skel.on_frame(0.0) {
            self.linear_velocity = 15.0;
            self.state2 &= !STATE2_30;
        }
        // linearVelocity > 12: dust (func_8084269C).
        step_to_f(&mut self.linear_velocity, 0.0, 5.0);
        self.func_8083C50C();
        if self.skel.update(data) && !self.func_80850224(env) {
            let mf = self.skel.move_flags;
            let end = if self.state1 & STATE1_4 != 0 { a.end_locked } else { a.end };
            self.func_80832318();
            self.skel.move_flags = 0;
            self.func_8083A098(data, end);
            self.skel.move_flags = mf;
            self.state3 |= STATE3_3;
        }
    }

    // ================================================================================
    // Water (func_8083D53C) and swimming

    /// `func_8083D53C`: the underwater filter, then entering and leaving water.
    fn func_8083D53C(&mut self, env: &Env) {
        if self.actor.y_dist_to_water < self.age.unk_2C {
            self.sfx(PlayerSfx::BaseFilter(0));
            self.unk_840 = 0;
        } else {
            self.sfx(PlayerSfx::BaseFilter(0x20));
            if self.unk_840 < 300 {
                self.unk_840 += 1;
            }
        }
        let a = self.action;
        if a == Action::ClimbLedge || a == Action::ClimbUp {
            return;
        }
        let swim_actions = matches!(a, Action::Swim | Action::SwimMove | Action::SwimTarget | Action::Dive | Action::Surface);
        if self.age.unk_2C < self.actor.y_dist_to_water {
            // Kokiri boots; func_8084E30C / func_8084E368 / func_8084D7C4 are not ported.
            if self.state1 & STATE1_27 == 0 || !swim_actions {
                self.func_8083D36C(env);
            }
        } else if self.state1 & STATE1_27 != 0 && self.actor.y_dist_to_water < self.age.unk_24 {
            if self.skel.move_flags == 0 {
                let y = self.actor.shape_rot.y;
                self.func_8083CD54(env.data, y);
            }
            let vy = self.actor.velocity.y;
            self.func_8083D0A8(env, vy);
        }
    }

    /// `func_8083CFA8`: at more than 2 of vertical speed `arg2`, a splash at the waist where it
    /// meets a water surface less than 100 above Link (`WaterBox_GetSurface1`): true if there
    /// is one. (The splash, `EffectSsGSplash`, of kind 0 up to 10 of speed and 1 above, at
    /// `splash_scale`, isn't drawn: the effects aren't ported.)
    fn func_8083CFA8(&mut self, env: &Env, arg2: f32, _splash_scale: i32) -> bool {
        let sp3c = arg2.abs();
        if sp3c > 2.0 {
            let waist = self.body_parts_pos[PLAYER_BODYPART_WAIST];
            if let Some(sp34) = env.col.water_surface(waist.x, waist.z, env.col.water_room)
                && (sp34 - self.actor.world_pos.y) < 100.0
            {
                return true;
            }
        }
        false
    }

    /// `func_8083D36C`: start swimming.
    fn func_8083D36C(&mut self, env: &Env) {
        let data = env.data;
        // func_80832564.
        self.func_80832440();
        if self.state2 & STATE2_10 != 0 {
            self.state2 &= !STATE2_10;
            self.func_8083D12C(env, false);
            self.unk_84F = 1;
        } else {
            // (From func_80844A44, the hookshot flight, it would dive: not ported.)
            self.setup_action(data, Action::Swim, 1);
            let a = if self.grounded() { data.anim("link_swimer_wait2swim_wait") } else { data.anim("link_swimer_land2swim_wait") };
            self.func_80832B0C(data, a);
        }
        if self.state1 & STATE1_27 == 0 || self.actor.y_dist_to_water < self.age.unk_2C {
            let vy = self.actor.velocity.y;
            if self.func_8083CFA8(env, vy, 500) {
                self.func_8002F7DC(NA_SE_EV_DIVE_INTO_WATER);
                if self.fall_distance > 800 {
                    self.func_80832698(NA_SE_VO_LI_CLIMB_END);
                }
            }
        }
        self.state1 |= STATE1_27;
        self.state2 |= STATE2_10;
        self.state1 &= !(STATE1_18 | STATE1_19);
        // Player_SetBootData: Kokiri boots keep their data in water.
    }

    /// `func_8083D0A8`: out of the water, with a splash's sound if there's one.
    fn func_8083D0A8(&mut self, env: &Env, vy: f32) {
        self.state1 |= STATE1_18;
        self.state1 &= !STATE1_27;
        self.func_80832340();
        if self.func_8083CFA8(env, vy, 500) {
            self.func_8002F7DC(NA_SE_EV_JUMP_OUT_WATER);
        }
    }

    /// `func_80832340`.
    fn func_80832340(&mut self) {
        self.state2 &= !(STATE2_10 | STATE2_11);
    }

    /// `func_8083D12C`: dive on A from the surface (`input`), or surface when rising close to it.
    fn func_8083D12C(&mut self, env: &Env, input: bool) -> bool {
        let data = env.data;
        if self.state1 & STATE1_10 == 0 && self.state2 & STATE2_10 == 0 && (!input || (self.input.press.held(BTN_A) && abs16(self.unk_6C2) < 12000)) {
            self.setup_action(data, Action::Dive, 0);
            let a = data.anim("link_swimer_swim_deep_start");
            self.skel.play_once(data, a);
            self.unk_6C2 = 0;
            self.state2 |= STATE2_10;
            self.actor.velocity.y = 0.0;
            if input {
                self.state2 |= STATE2_11;
                self.func_8002F7DC(NA_SE_PL_DIVE_BUBBLE);
            }
            return true;
        }
        if (self.state1 & STATE1_10 != 0 || self.state2 & STATE2_10 != 0) && self.actor.velocity.y > 0.0 && self.actor.y_dist_to_water < self.age.unk_30 {
            self.state2 &= !STATE2_10;
            if input {
                self.setup_action(data, Action::Surface, 1);
                self.unk_850 = 2;
            }
            self.func_80832340();
            let a = data.anim("link_swimer_swim_deep_end");
            self.func_80832B0C(data, a);
            let vy = self.actor.velocity.y;
            if self.func_8083CFA8(env, vy, 500) {
                self.func_8002F7DC(NA_SE_PL_FACE_UP);
            }
            return true;
        }
        false
    }

    /// `func_8084B000`: buoyancy. Floats up to `unk_28` below the surface; sinks slowly when
    /// deeper than 100 (and flags `PLAYER_STATE2_10`).
    fn func_8084B000(&mut self) {
        let mut phi_f14 = -5.0f32;
        let mut phi_f16 = self.age.unk_28;
        if self.actor.velocity.y < 0.0 {
            phi_f16 += 1.0;
        }
        let phi_f18;
        if self.actor.y_dist_to_water < phi_f16 {
            phi_f16 = if self.actor.velocity.y <= 0.0 { 0.0 } else { self.actor.velocity.y * 0.5 };
            phi_f18 = -0.1 - phi_f16;
        } else {
            phi_f14 = 2.0;
            phi_f16 = if self.actor.velocity.y >= 0.0 { 0.0 } else { self.actor.velocity.y * -0.3 };
            phi_f18 = phi_f16 + 0.1;
            if self.actor.y_dist_to_water > 100.0 {
                self.state2 |= STATE2_10;
            }
        }
        self.actor.velocity.y += phi_f18;
        if (self.actor.velocity.y - phi_f14) * phi_f18 > 0.0 {
            self.actor.velocity.y = phi_f14;
        }
        self.actor.gravity = 0.0;
    }

    /// `func_8084AEEC`: swim acceleration, only in frames 10..20 of the stroke; turn at 1600.
    fn func_8084AEEC(&mut self, v: f32, mut arg2: f32, arg3: i16) -> f32 {
        let mut v = v;
        let mut t = self.skel.cur_frame - 10.0;
        let limit = self.regs.run_speed_limit() * 0.8;
        if v > limit {
            v = limit;
        }
        if 0.0 < t && t < 10.0 {
            t *= 6.0;
        } else {
            t = 0.0;
            arg2 = 0.0;
        }
        let dec = v.abs() * 0.02 + 0.05;
        asym_step_to_f(&mut v, arg2 * 0.8, t, dec);
        scaled_step_to_s(&mut self.current_yaw, arg3, 1600);
        v
    }

    /// `func_8084D530`: `func_8084AEEC`, and the strokes' sound (`D_808549D0`).
    fn func_8084D530(&mut self, env: &Env, v: f32, arg2: f32, arg3: i16) -> f32 {
        let v = self.func_8084AEEC(v, arg2, arg3);
        self.func_80832924(env.audio.player_anim_sfx("D_808549D0"));
        v
    }

    /// `func_8084B158`: stroke animation speed from the swim speed (doubled on A/B).
    fn func_8084B158(&mut self, data: &GameData, input: bool, arg3: f32) {
        let k = if input && (self.input.press.held(BTN_A) || self.input.press.held(eng_input::pad::BTN_B)) { 1.0 } else { 0.5 };
        self.skel.play_speed = (k * arg3).max(1.0);
        self.skel.update(data);
    }

    /// `func_80832C6C`: loop `anim`, blending over 16 frames.
    fn func_80832C6C(&mut self, data: &GameData, anim: AnimId) {
        self.skel.change(data, anim, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -16.0);
    }

    /// `func_8084D574` / `func_8084D5CC` / `func_80838F18`: swim forward, swim targeting, tread.
    fn func_8084D574(&mut self, data: &GameData, yaw: i16) {
        self.setup_action(data, Action::SwimMove, 0);
        self.current_yaw = yaw;
        self.actor.shape_rot.y = yaw;
        let a = data.anim("link_swimer_swim");
        self.func_80832C6C(data, a);
    }
    fn func_8084D5CC(&mut self, data: &GameData) {
        self.setup_action(data, Action::SwimTarget, 0);
        let a = data.anim("link_swimer_swim");
        self.func_80832C6C(data, a);
    }
    fn func_80838F18(&mut self, data: &GameData) {
        self.setup_action(data, Action::Swim, 0);
        let a = data.anim("link_swimer_swim_wait");
        self.func_80832C6C(data, a);
    }

    /// `func_8084D610`: treading water.
    fn func_8084D610(&mut self, env: &Env) {
        let data = env.data;
        // func_80832CB0.
        if self.skel.update(data) {
            let a = data.anim("link_swimer_swim_wait");
            self.skel.play_loop(data, a);
        }
        self.func_8084B000();
        if !self.func_80837348(env, Self::D_80854444, true) && !self.func_8083D12C(env, true) {
            if self.unk_6AD != 1 {
                self.unk_6AD = 0;
            }
            let (_, sp34, sp32) = self.func_80837268(env, 0.0);
            if sp34 != 0.0 {
                let t = self.actor.shape_rot.y.wrapping_sub(sp32);
                if abs16(t) > 0x6000 && !step_to_f(&mut self.linear_velocity, 0.0, 1.0) {
                    return;
                }
                if self.func_80833C04(env) {
                    self.func_8084D5CC(data);
                } else {
                    self.func_8084D574(data, sp32);
                }
            }
            let v = self.linear_velocity;
            self.linear_velocity = self.func_8084AEEC(v, sp34, sp32);
        }
    }

    /// `func_8084D84C`: swimming forward.
    fn func_8084D84C(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        let v = self.linear_velocity;
        self.func_8084B158(data, true, v);
        self.func_8084B000();
        if !self.func_80837348(env, Self::D_80854444, true) && !self.func_8083D12C(env, true) {
            let (_, sp34, sp32) = self.func_80837268(env, 0.0);
            let t = self.actor.shape_rot.y.wrapping_sub(sp32);
            if sp34 == 0.0 || abs16(t) > 0x6000 {
                self.func_80838F18(data);
            } else if self.func_80833C04(env) {
                self.func_8084D5CC(data);
            }
            let v = self.linear_velocity;
            self.linear_velocity = self.func_8084D530(env, v, sp34, sp32);
        }
    }

    /// `func_8084DAB4`: swimming while targeting (forward, back or sideways strokes).
    fn func_8084DAB4(&mut self, env: &Env) {
        let data = env.data;
        let v = self.linear_velocity;
        self.func_8084B158(data, true, v);
        self.func_8084B000();
        if !self.func_80837348(env, Self::D_80854444, true) && !self.func_8083D12C(env, true) {
            let (_, mut sp2c, mut sp2a) = self.func_80837268(env, 0.0);
            if sp2c == 0.0 {
                self.func_80838F18(data);
            } else if !self.func_80833C04(env) {
                self.func_8084D574(data, sp2a);
            } else {
                self.func_8084D980(env, &mut sp2c, &mut sp2a);
            }
            let v = self.linear_velocity;
            self.linear_velocity = self.func_8084D530(env, v, sp2c, sp2a);
        }
    }

    /// `func_8084D980`: the stroke for the direction while targeting.
    fn func_8084D980(&mut self, env: &Env, speed: &mut f32, yaw: &mut i16) -> bool {
        let data = env.data;
        let t1 = self.current_yaw.wrapping_sub(*yaw);
        let anim = if abs16(t1) > 0x6000 {
            if step_to_f(&mut self.linear_velocity, 0.0, 1.0) {
                self.current_yaw = *yaw;
            } else {
                *speed = 0.0;
                *yaw = self.current_yaw;
            }
            data.anim("link_swimer_swim_wait")
        } else {
            let t2 = self.func_8083FD78(env, speed, yaw);
            if t2 > 0 {
                data.anim("link_swimer_swim")
            } else if t2 < 0 {
                data.anim("link_swimer_back_swim")
            } else if self.actor.shape_rot.y.wrapping_sub(*yaw) > 0 {
                data.anim("link_swimer_Rside_swim")
            } else {
                data.anim("link_swimer_Lside_swim")
            }
        };
        if anim != self.skel.animation {
            self.func_80832C6C(data, anim);
            return true;
        }
        false
    }

    /// `func_8083D330`.
    fn func_8083D330(&mut self, data: &GameData) {
        let a = data.anim("link_swimer_swim");
        self.skel.play_loop(data, a);
        self.unk_6C2 = 16000;
        self.unk_850 = 1;
    }

    /// `func_8084DBC4`: move under water (horizontal and vertical swim speeds).
    fn func_8084DBC4(&mut self, env: &Env, arg2: f32) {
        let (_, sp2c, sp2a) = self.func_80837268(env, 0.0);
        let v = self.linear_velocity;
        self.linear_velocity = self.func_8084AEEC(v, sp2c * 0.5, sp2a);
        let vy = self.actor.velocity.y;
        let cy = self.current_yaw;
        self.actor.velocity.y = self.func_8084AEEC(vy, arg2, cy);
    }

    /// `func_8084DC48`: diving: the dive start, holding A to go deeper (to D_80854784[no scale]
    /// = 120 below the surface), then pitching back up and rising.
    fn func_8084DC48(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        self.actor.gravity = 0.0;
        self.func_80836670(env);
        // func_8083B040 (C-button items): none.
        if self.unk_84F == 0 {
            if self.unk_850 == 0 {
                if self.skel.update(data) || (self.skel.cur_frame >= 22.0 && !self.input.cur.held(BTN_A)) {
                    self.func_8083D330(data);
                } else if self.skel.on_frame(20.0) {
                    self.actor.velocity.y = -2.0;
                }
                self.func_8083721C();
                return;
            }
            let vy = self.actor.velocity.y;
            self.func_8084B158(data, true, vy);
            self.unk_6C2 = 16000;
            // func_8083E5A8 (picking up an item) is never taken; no scale upgrade.
            if self.input.cur.held(BTN_A) && !self.grounded() && self.actor.y_dist_to_water < 120.0 {
                self.func_8084DBC4(env, -2.0);
            } else {
                self.unk_84F += 1;
                let a = data.anim("link_swimer_swim_wait");
                self.func_80832C6C(data, a);
            }
        } else if self.unk_84F == 1 {
            self.skel.update(data);
            self.func_8084B000();
            if self.unk_6C2 < 10000 {
                self.unk_84F += 1;
                self.unk_850 = self.actor.y_dist_to_water as i16;
                let a = data.anim("link_swimer_swim");
                self.func_80832C6C(data, a);
            }
        } else if !self.func_8083D12C(env, true) {
            let sp2c = (self.unk_850 as f32 * 0.018 + 4.0).min(8.0);
            let vy = self.actor.velocity.y.abs();
            self.func_8084B158(data, true, vy);
            scaled_step_to_s(&mut self.unk_6C2, -10000, 800);
            self.func_8084DBC4(env, sp2c);
        }
    }

    /// `func_8084E1EC`: surfacing.
    fn func_8084E1EC(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        if self.skel.update(data) {
            // Not holding an item from the bottom (PLAYER_STATE1_10).
            self.func_80838F18(data);
            self.func_80832340();
        } else if self.skel.on_frame(5.0) {
            // (PLAYER_STATE1_10's frame 10, the item from the bottom: not ported.)
            self.func_80832698(NA_SE_VO_LI_BREATH_DRINK);
        }
        self.func_8084B000();
        let y = self.actor.shape_rot.y;
        let v = self.linear_velocity;
        self.linear_velocity = self.func_8084AEEC(v, 0.0, y);
    }

    // ================================================================================
    // Exits, entrances and voids

    /// `func_80839034`: the floor's exit (or a void floor) starts the scene transition and the
    /// walk out; otherwise the fall-into-the-void check.
    fn func_80839034(&mut self, env: &Env, poly: Option<PolyId>) -> bool {
        let col = env.col;
        let mut io = env.io.borrow_mut();
        let mut exit_index = 0;
        let exiting = self.state1 & STATE1_7 == 0
            && io.transition.trigger == TRANS_TRIGGER_OFF
            && self.cs_mode == 0
            && self.state1 & STATE1_0 == 0
            && ({
                exit_index = poly.map(|p| col.exit_index(p)).unwrap_or(0);
                exit_index != 0
            } || (func_8083816C(self.s.floor_type) && self.unk_A7A == FLOOR_PROPERTY_12));
        if exiting {
            let sp34 = self.unk_A84 as i32 - self.actor.world_pos.y as i32;
            if self.state1 & (STATE1_23 | STATE1_27 | STATE1_29) == 0 && !self.grounded() && sp34 < 100 && self.s.floor_dist > 100.0 {
                return false;
            }
            if exit_index == 0 {
                io.trigger_void_out();
                io.set_transition_for_next_entrance(env.entrances);
            } else {
                let Some(&next) = env.exits.get(exit_index as usize - 1) else {
                    drop(io);
                    self.note(format!("exit {exit_index} is past the exit list ({})", env.exits.len()));
                    return false;
                };
                io.transition.next_entrance_index = next;
                if next == ENTR_RETURN_GROTTO {
                    io.save.respawn_flag = 2;
                    io.transition.next_entrance_index = io.save.respawn[RESPAWN_MODE_RETURN].entrance_index;
                    io.transition.ty = TRANS_TYPE_FADE_WHITE;
                    io.save.next_transition_type = TRANS_TYPE_FADE_WHITE;
                } else if next >= ENTR_RETURN_YOUSEI_IZUMI_YOKO {
                    // sReturnEntranceGroupData (the fountains' and shops' way back): not ported.
                    drop(io);
                    self.note(format!("exit to ENTR_RETURN group {next:#x} not ported"));
                    return false;
                } else {
                    if poly.is_some_and(|p| col.floor_effect(p) == FLOOR_EFFECT_2) {
                        io.save.respawn[RESPAWN_MODE_DOWN].entrance_index = next;
                        io.trigger_void_out();
                        io.save.respawn_flag = -2;
                    }
                    io.save.retain_weather_mode = true;
                    io.set_transition_for_next_entrance(env.entrances);
                }
                io.transition.trigger = TRANS_TRIGGER_START;
            }
            let temp = poly.map(|p| col.floor_type(p)).unwrap_or(0);
            if self.state1 & (STATE1_23 | STATE1_29) == 0
                && self.state2 & STATE2_18 == 0
                && !self.func_808332B8()
                && temp != FLOOR_TYPE_10
                && (sp34 < 100 || self.grounded())
            {
                if temp != FLOOR_TYPE_11 {
                    let mut v = self.linear_velocity;
                    if v < 0.0 {
                        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(-0x8000i32 as i16);
                        v = -v;
                    }
                    let limit = self.regs.run_speed_limit();
                    io.save.entrance_speed = if v > limit { limit } else { v };
                    // (Conveyor floors would give sConveyorYaw: not modelled.)
                    let yaw = self.actor.world_rot.y;
                    drop(io);
                    self.func_80838E70(env.data, 400.0, yaw);
                } else {
                    // FLOOR_TYPE_11 (a secret hole): its sound, and the music fades.
                    self.sfx(PlayerSfx::NoPos(NA_SE_OC_SECRET_HOLE_OUT));
                    self.play_requests.push(PlayRequest::FadeOutAllSeq(5));
                    io.save.seq_id = oot_game::audio::NA_BGM_DISABLED as u8;
                    io.save.nature_ambience_id = oot_game::audio::NATURE_ID_DISABLED;
                }
            } else if !self.grounded() {
                self.func_80832210();
            }
            self.state1 |= STATE1_0 | STATE1_29;
            self.play_requests.push(PlayRequest::CamSetting(oot_game::camera::CAM_SET_SCENE_TRANSITION));
            return true;
        }
        if io.transition.trigger == TRANS_TRIGGER_OFF {
            let fall = self.fall_distance;
            let scene = io.scene_id;
            if self.actor.world_pos.y < -4000.0
                || ((self.unk_A7A == FLOOR_PROPERTY_5 || self.unk_A7A == FLOOR_PROPERTY_12)
                    && (self.s.floor_dist < 100.0 || fall > 400 || (scene != SCENE_HAKADAN && fall > 200)))
                || (scene == SCENE_GANON_FINAL && fall > 320)
            {
                if self.grounded() {
                    if self.unk_A7A == FLOOR_PROPERTY_5 {
                        let (pos, yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
                        io.trigger_respawn(pos, yaw);
                    } else {
                        io.trigger_void_out();
                    }
                    io.transition.ty = TRANS_TYPE_FADE_BLACK_FAST;
                    self.sfx(PlayerSfx::NoPos(NA_SE_OC_ABYSS));
                } else {
                    drop(io);
                    self.func_80838F5C(env.data);
                    self.unk_850 = 9999;
                    self.unk_84F = if self.unk_A7A == FLOOR_PROPERTY_5 { -1 } else { 1 };
                    self.unk_A84 = self.actor.world_pos.y as i16;
                    return false;
                }
            }
            self.unk_A84 = self.actor.world_pos.y as i16;
        }
        false
    }

    /// `func_808332B8`: swimming without iron boots.
    fn func_808332B8(&self) -> bool {
        self.state1 & STATE1_27 != 0
    }

    /// `func_80838F5C`: start falling into the void (`func_8084F88C`).
    fn func_80838F5C(&mut self, data: &GameData) {
        self.setup_action(data, Action::VoidFall, 0);
        self.state1 |= STATE1_29 | STATE1_31;
        self.play_requests.push(PlayRequest::ChangeSetting(oot_game::camera::CAM_SET_FREE0));
    }

    /// `func_80838FB8`: falling off an edge while `func_80838F5C` is pending.
    fn func_80838FB8(&mut self, env: &Env) -> bool {
        if env.io.borrow().transition.trigger == TRANS_TRIGGER_OFF && self.state1 & STATE1_31 != 0 {
            self.func_80838F5C(env.data);
            // func_80832284: LinkAnimation_PlayLoop.
            let a = env.data.anim("link_normal_landing_wait");
            self.skel.play_loop(env.data, a);
            self.func_80832698(NA_SE_VO_LI_FALL_S);
            self.sfx(PlayerSfx::NoPos(NA_SE_OC_SECRET_WARP_IN));
            return true;
        }
        false
    }

    /// `func_8084F88C`: after 9 frames of falling, the void out (or respawn) starts.
    fn func_8084F88C(&mut self, env: &Env) {
        self.skel.update(env.data);
        let n = self.unk_850;
        self.unk_850 = self.unk_850.wrapping_add(1);
        let mut io = env.io.borrow_mut();
        if n > 8 && io.transition.trigger == TRANS_TRIGGER_OFF {
            if self.unk_84F != 0 {
                if self.unk_84F < 0 {
                    let (pos, yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
                    io.trigger_respawn(pos, yaw);
                } else {
                    io.trigger_void_out();
                }
                io.transition.ty = TRANS_TYPE_FADE_BLACK_FAST;
                self.sfx(PlayerSfx::NoPos(NA_SE_OC_ABYSS));
            } else {
                io.transition.ty = TRANS_TYPE_FADE_BLACK;
                io.save.next_transition_type = TRANS_TYPE_FADE_BLACK;
            }
            io.transition.trigger = TRANS_TRIGGER_START;
        }
    }

    /// `func_80845CA4`: the walk through an exit (towards `unk_450` at the entrance speed) or
    /// in from an entrance (`unk_850` < 0 counts it down), then standing or running on.
    fn func_80845CA4(&mut self, env: &Env) {
        let data = env.data;
        if !self.func_8083B040(env) {
            if self.unk_850 == 0 {
                self.skel.update(data);
                self.door_timer -= 1;
                if self.door_timer <= 0 {
                    self.door_timer = 0;
                    self.linear_velocity = 0.1;
                    self.unk_850 = 1;
                }
            } else if self.unk_84F == 0 {
                let mut sp3c = 5.0 * self.s.speed_scale;
                if self.func_80845BA0(env, &mut sp3c, -1) < 30 {
                    self.unk_84F = 1;
                    self.state1 |= STATE1_29;
                    self.unk_450.x = self.unk_45C.x;
                    self.unk_450.z = self.unk_45C.z;
                }
            } else {
                let mut sp34 = 5.0;
                let mut sp30 = 20;
                if self.state1 & STATE1_0 != 0 {
                    sp34 = env.io.borrow().save.entrance_speed;
                } else if self.unk_850 < 0 {
                    self.unk_850 += 1;
                    sp34 = env.io.borrow().save.entrance_speed;
                    sp30 = -1;
                }
                let temp = self.func_80845BA0(env, &mut sp34, sp30);
                if self.unk_850 == 0 || (temp == 0 && self.linear_velocity == 0.0 && env.cam_unk_14c & 0x10 != 0) {
                    self.play_requests.push(PlayRequest::CamDone);
                    // func_80845C68(play, respawn[DOWN].data):
                    let mut io = env.io.borrow_mut();
                    if io.save.respawn[RESPAWN_MODE_DOWN].data == 0 {
                        let (pos, yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
                        io.setup_respawn_point(RESPAWN_MODE_DOWN, 0xDFF, pos, yaw);
                    }
                    io.save.respawn[RESPAWN_MODE_DOWN].data = 0;
                    drop(io);
                    if !self.func_8083B644(env) {
                        self.func_8083CF5C(data);
                    }
                }
            }
        }
        if self.state1 & STATE1_11 != 0 {
            self.func_80836670(env);
        }
    }

    /// `func_80839800`: A pressed with a door offering to open (`doorType`, set by `EnDoor_Idle`
    /// last frame): Link lines up at the door, 22 in front, and plays the opening animation
    /// for his side and age (`PLAYER_ANIMGROUP_9` .. `_12`), moved by it (`func_80832F54`,
    /// 0x28F). A scene-exit door starts the exit under its far side (`func_80839034`, entrance
    /// speed 2); any other gets the door camera and loads the room behind it.
    ///
    /// An ajar door (`PLAYER_DOORTYPE_AJAR`) shows text 0xD0 instead.
    ///
    /// Not ported: sliding doors (`Door_Shutter`), `Door_Killer`, holding Ruto
    /// (`ACTOR_EN_RU1`), and `func_8084F9A0` (a cutscene's door walk).
    fn func_80839800(&mut self, env: &Env) -> bool {
        if self.door_type == PLAYER_DOORTYPE_NONE || self.state1 & STATE1_11 != 0 {
            return false;
        }
        if !self.input.press.held(BTN_A) {
            return false;
        }
        let Some(dh) = self.door_actor else { return false };
        let Some(door) = env.actors.actor(dh) else { return false };
        if self.door_type <= PLAYER_DOORTYPE_AJAR {
            // doorActor->textId = 0xD0, then talking to it (and 0: no interrupt taken).
            self.play_requests.push(PlayRequest::SetTextId { actor: dh, text_id: 0xD0 });
            self.func_80853148(env, dh, 0xD0);
            return false;
        }
        let mut door_direction = self.door_direction as i32;
        let (sp78, sp74) = (cos_s(door.shape_rot.y), sin_s(door.shape_rot.y));
        if self.door_type == PLAYER_DOORTYPE_SLIDING {
            self.note("sliding doors (Door_Shutter) aren't ported");
            return false;
        }
        use crate::en_door::{DOOR_OPEN_ANIM_ADULT_L, DOOR_OPEN_ANIM_ADULT_R, DOOR_OPEN_ANIM_CHILD_L, DOOR_OPEN_ANIM_CHILD_R, DOOR_SCENEEXIT};
        let open_anim = match (door_direction < 0, self.adult) {
            (true, true) => DOOR_OPEN_ANIM_ADULT_L,
            (true, false) => DOOR_OPEN_ANIM_CHILD_L,
            (false, true) => DOOR_OPEN_ANIM_ADULT_R,
            (false, false) => DOOR_OPEN_ANIM_CHILD_R,
        };
        let group = match open_anim {
            DOOR_OPEN_ANIM_ADULT_L => 9,
            DOOR_OPEN_ANIM_CHILD_L => 10,
            DOOR_OPEN_ANIM_ADULT_R => 11,
            _ => 12,
        };
        let data = env.data;
        let anim = self.anim(data, group);
        let (door_pos, door_yaw, door_parent, door_params, door_category) = (door.world_pos, door.shape_rot.y, door.parent, door.params, door.category);
        self.setup_action(data, Action::DoorOpen, 0);
        self.func_80832528(data);
        self.actor.shape_rot.y = if door_direction < 0 { door_yaw } else { door_yaw.wrapping_add(-0x8000i32 as i16) };
        self.current_yaw = self.actor.shape_rot.y;
        let sp6c = door_direction as f32 * 22.0;
        self.actor.world_pos.x = door_pos.x + sp6c * sp74;
        self.actor.world_pos.z = door_pos.z + sp6c * sp78;
        // func_8083328C: LinkAnimation_PlayOnceSetSpeed at D_808535E8.
        self.skel.play_once_set_speed(data, anim, self.s.speed_scale);
        if self.door_timer != 0 {
            self.skel.end_frame = 0.0;
        }
        self.func_80832224();
        self.func_80832F54(0x28F);
        // The second half of a double door (spawned as a child).
        if door_parent.is_some() {
            door_direction = -door_direction;
        }
        self.play_requests.push(PlayRequest::OpenDoor { door: dh, open_anim });
        // Not a Door_Killer: an EnDoor.
        self.state1 |= STATE1_29;
        // (Actor_DisableLens: there's no lens.)
        let side = if door_direction > 0 { 0 } else { 1 };
        let entry = env.transi_actors.get((door_params as u16 >> oot_game::scene::TRANSITION_ACTOR_PARAMS_INDEX_SHIFT) as usize).copied();
        if (door_params as u16 >> 7) & 7 == DOOR_SCENEEXIT {
            let check = Vec3::new(door_pos.x - sp6c * sp74, door_pos.y + 10.0, door_pos.z - sp6c * sp78);
            // BgCheck_EntityRaycastDown1. @bug (game): the poly's bgId is taken as BGCHECK_SCENE.
            let (_, ground_poly) = env.col.entity_raycast_down(check);
            if self.func_80839034(env, ground_poly) {
                env.io.borrow_mut().save.entrance_speed = 2.0;
                env.io.borrow_mut().save.entrance_sound = NA_SE_OC_DOOR_OPEN;
            }
        } else if let Some(t) = entry {
            // 38, 26 and 10 frames times D_808535EC (1).
            self.play_requests.push(PlayRequest::DoorCam { door: dh, bg_cam_index: t.sides[side].1 as i16, timers: [38, 26, 10] });
        }
        if door_category == oot_game::actor_ctx::ACTORCAT_DOOR
            && let Some(t) = entry
        {
            let front_room = t.sides[side].0;
            if front_room >= 0 && front_room != env.io.borrow().room {
                self.play_requests.push(PlayRequest::RoomLoad(front_room));
            }
        }
        self.play_requests.push(PlayRequest::DoorRoom { door: dh });
        true
    }

    /// `func_80845EF8`: the door animation, then standing; the old room goes, the door camera
    /// is told Player is through, and the void-out point moves here.
    fn func_80845EF8(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        let done = self.skel.update(data);
        self.func_80836670(env);
        if done {
            if self.unk_850 == 0 {
                // DECR(doorTimer) == 0.
                if self.door_timer != 0 {
                    self.door_timer -= 1;
                }
                if self.door_timer == 0 {
                    self.unk_850 = 1;
                    self.skel.end_frame = self.skel.anim_length - 1.0;
                }
            } else {
                self.func_8083C0E8(data);
                if env.prev_room >= 0 {
                    self.play_requests.push(PlayRequest::RoomChangeDone);
                }
                self.play_requests.push(PlayRequest::CamDone);
                let (pos, yaw) = (self.actor.world_pos, self.actor.shape_rot.y);
                env.io.borrow_mut().setup_respawn_point(RESPAWN_MODE_DOWN, 0xDFF, pos, yaw);
            }
            return;
        }
        if self.state1 & STATE1_29 == 0 && self.skel.on_frame(15.0) {
            // play->func_11D54 (func_80853080): only a Door_Killer's opening gets here.
            self.func_80853080(data);
        }
    }

    /// `func_80845BA0`: walk towards `unk_450` at `speed`, stopping within `arg3`. Returns the
    /// distance left (0 while the animation holds Player in place).
    fn func_80845BA0(&mut self, env: &Env, speed: &mut f32, arg3: i32) -> i32 {
        let dx = self.unk_450.x - self.actor.world_pos.x;
        let dz = self.unk_450.z - self.actor.world_pos.z;
        let sp2c = (dx * dx + dz * dz).sqrt() as i32;
        let mut yaw = atan2_s(dz, dx);
        if sp2c < arg3 {
            *speed = 0.0;
            yaw = self.actor.shape_rot.y;
        }
        if self.func_80845964(env, *speed, yaw) {
            return 0;
        }
        sp2c
    }

    /// `func_80845964(play, this, NULL, speed, yaw, 2)`: the walk without a cutscene's
    /// positions. Standing still, it only plays the animation (true when it ends).
    fn func_80845964(&mut self, env: &Env, speed: f32, yaw: i16) -> bool {
        let data = env.data;
        if self.linear_velocity == 0.0 {
            return self.skel.update(data);
        }
        self.state2 |= STATE2_5;
        self.func_80841EE4(data);
        self.func_8083DF68(speed, yaw);
        if speed == 0.0 && self.linear_velocity == 0.0 {
            self.func_8083BF50(data);
        }
        false
    }

    /// `func_8083CF5C`: after an entrance, run on or stand.
    fn func_8083CF5C(&mut self, data: &GameData) {
        if self.linear_velocity != 0.0 {
            self.func_8083C858(data);
        } else {
            self.func_80839F90(data);
        }
    }

    /// `func_8083ADD4`: with a cutscene mode pending (`unk_6AD` 3), Player's action becomes the
    /// cutscene's (`func_80852E14`), held (`PLAYER_STATE1_29`) when `func_8002DF54` set it.
    fn func_8083ADD4(&mut self, data: &GameData) -> bool {
        if self.unk_6AD == 3 {
            self.setup_action(data, Action::Cutscene, 0);
            if self.door_bg_cam_index != 0 {
                self.state1 |= STATE1_29;
            }
            self.func_80832318();
            return true;
        }
        false
    }

    /// `func_8083B040` (interrupt 13, and the actions that check it first): with `unk_6AD` set
    /// and Link on the ground, swimming or riding, a cutscene mode takes over
    /// (`func_8083ADD4`). The items and spells (`unk_6AD` 2 and 4) aren't ported.
    fn func_8083B040(&mut self, env: &Env) -> bool {
        if self.unk_6AD != 0 && (self.func_808332B8() || self.grounded() || self.state1 & STATE1_23 != 0) {
            if !self.func_8083ADD4(env.data) {
                self.note(format!("func_8083B040 with unk_6AD {}: items and first person aren't ported", self.unk_6AD));
                return false;
            }
            self.func_80832224();
            return true;
        }
        false
    }

    /// `func_8083B998` (interrupt 0): a cutscene mode (or an item) waiting (`unk_6AD`) takes
    /// over through `func_8083B040`; a target Navi would talk about sets `PLAYER_STATE2_21`. C-Up
    /// into first person (`func_8083B8F4`) isn't ported.
    fn func_8083B998(&mut self, env: &Env) -> bool {
        if self.unk_6AD != 0 {
            self.func_8083B040(env);
            return true;
        }
        // (naviEnemyId is NAVI_ENEMY_NONE for every ported actor.)
        if self.unk_664.and_then(|h| env.target(h)).is_some_and(|t| t.flags & (ACTOR_FLAG_0 | ACTOR_FLAG_18) == (ACTOR_FLAG_0 | ACTOR_FLAG_18)) {
            self.state2 |= STATE2_21;
        } else if self.navi_text_id == 0 && self.state1 & STATE1_4 == 0 && self.input.press.held(eng_input::pad::BTN_CUP) {
            self.note("C-Up into first person (func_8083B8F4) not ported");
        }
        false
    }

    /// `func_8002DF54` on Player itself (`z_actor.c`): `csMode`, no actor, `doorBgCamIndex` 1.
    fn func_8002DF54(&mut self, cs_mode: u8) {
        self.cs_mode = cs_mode;
        self.unk_448 = None;
        self.door_bg_cam_index = 1;
    }

    /// `func_80852E14`: the cutscene action. A new `csMode` runs its start (`D_80854B18`), and
    /// every frame its update (`D_80854E50`).
    fn func_80852E14(&mut self, env: &Env) {
        if self.cs_mode != self.prev_cs_mode {
            self.s.d_80858aa0 = self.skel.move_flags as i32;
            self.func_80832DBC();
            self.prev_cs_mode = self.cs_mode;
            log::debug!("DEMO MODE={}", self.cs_mode);
            self.func_80852C0C(self.cs_mode);
            let m = self.cs_mode;
            self.func_80852B4C(env, None, m, true);
        }
        let m = self.cs_mode;
        self.func_80852B4C(env, None, m, false);
    }

    /// `func_80852B4C` with `D_80854B18[mode]` (`start`) or `D_80854E50[mode]` (the pack's
    /// `GameData::cs_mode_starts`, `cs_mode_updates`): a type's handler (`D_80854AA4`) with the
    /// entry's animation or sound table, or the entry's function. The functions that need what
    /// isn't ported (swimming, the ocarina, the items in hand, the effects) are logged.
    fn func_80852B4C(&mut self, env: &Env, cue: Option<CsCmdActorAction>, mode: u8, start: bool) {
        let tab = if start { &env.data.cs_mode_starts } else { &env.data.cs_mode_updates };
        match tab.get(mode as usize).cloned() {
            Some(e) if e.ty > 0 => self.cs_mode_type(env, &e),
            Some(e) if e.ty < 0 => self.cs_mode_func(env, cue, &e.name),
            Some(_) => {}
            None => self.note(format!("cutscene mode {mode} past the tables")),
        }
        if self.s.d_80858aa0 & 4 != 0 && self.skel.move_flags & 4 == 0 {
            self.skel.morph[0][1] = (self.skel.morph[0][1] as f32 / self.age.translation_scale) as i16;
            self.s.d_80858aa0 = 0;
        }
    }

    /// `D_80854AA4[type](play, this, ptr)`: the cutscene modes' typed handlers.
    fn cs_mode_type(&mut self, env: &Env, e: &oot_game::data::CsModeEntry) {
        let data = env.data;
        let a = match e.anim {
            Some(a) => a,
            // Types 1, 11 and 18 take no animation.
            None if matches!(e.ty, 1 | 11 | 18) => 0,
            None => {
                self.note(format!("cutscene mode type {} without an animation", e.ty));
                return;
            }
        };
        match e.ty {
            // func_80851008.
            1 => self.func_80832210(),
            // func_80851030: func_80850ED8.
            2 => self.func_80850ED8(data, a),
            // func_80851094: func_80850F1C.
            3 => self.func_80850F1C(data, a),
            // func_808510B4: func_80850F9C.
            4 => self.func_80850F9C(data, a),
            // func_808510D4: func_8083308C (func_80833064(play, this, anim, 0x1C)).
            5 => self.func_80833064(data, a, 0x1C),
            // func_808510F4: func_8083303C(play, this, anim, 0x9C).
            6 => self.func_8083303C(data, a, 0x9C),
            // func_80851114: func_8083313C (func_80833114(play, this, anim, 0x1C)).
            7 => self.func_80833114(data, a, 0x1C),
            // func_80851134: func_808330EC(play, this, anim, 0x9C).
            8 => self.func_808330EC(data, a, 0x9C),
            // func_80851154: func_80832264 (LinkAnimation_PlayOnce).
            9 => self.skel.play_once(data, a),
            // func_80851174: func_80832284 (LinkAnimation_PlayLoop).
            10 => self.skel.play_loop(data, a),
            // func_808511D4: LinkAnimation_Update.
            11 => {
                self.skel.update(data);
            }
            // func_808511FC: at the animation's end, func_80850F9C, unk_850 1.
            12 => {
                if self.skel.update(data) {
                    self.func_80850F9C(data, a);
                    self.unk_850 = 1;
                }
            }
            // func_80851294: at the end, func_8083313C, unk_850 1.
            13 => self.func_80851294(data, a),
            // func_80851050: func_80832DB0, func_80832C2C, func_80832210.
            14 => {
                self.func_80832DB0();
                self.skel.change(data, a, 1.0, 0.0, 0.0, ANIMMODE_ONCE, 0.0);
                self.func_80832210();
            }
            // func_80851194: func_808322D0 (LinkAnimation_PlayOnceSetSpeed 2/3).
            15 => self.skel.play_once_set_speed(data, a, 2.0 / 3.0),
            // func_808511B4: func_808322A4 (LinkAnimation_PlayLoopSetSpeed 2/3).
            16 => self.skel.play_loop_set_speed(data, a, 2.0 / 3.0),
            // func_80851248: at the end, func_80832DBC and func_808322A4.
            17 => {
                if self.skel.update(data) {
                    self.func_80832DBC();
                    self.skel.play_loop_set_speed(data, a, 2.0 / 3.0);
                }
            }
            // func_808512E0: LinkAnimation_Update and the sound table.
            18 => {
                self.skel.update(data);
                let t = env.audio.player_anim_sfx(&e.name).to_vec();
                self.func_80832924(&t);
            }
            t => self.note(format!("cutscene mode type {t}")),
        }
    }

    /// `func_80850F1C`: the animation once at 2/3, from its last frame's morph, standing.
    fn func_80850F1C(&mut self, data: &GameData, anim: AnimId) {
        self.func_80832DB0();
        let last = data.anims[anim].last_frame();
        self.skel.change(data, anim, 2.0 / 3.0, 0.0, last, ANIMMODE_ONCE, -8.0);
        self.func_80832210();
    }

    /// `func_80850F9C`: the animation looped at 2/3, standing.
    fn func_80850F9C(&mut self, data: &GameData, anim: AnimId) {
        self.func_80832DB0();
        self.skel.change(data, anim, 2.0 / 3.0, 0.0, 0.0, ANIMMODE_LOOP, -8.0);
        self.func_80832210();
    }

    /// `func_80833064`: `func_80832FFC` at 2/3: the animation once with `moveFlags`.
    fn func_80833064(&mut self, data: &GameData, anim: AnimId, flags: u16) {
        self.skel.play_once_set_speed(data, anim, 2.0 / 3.0);
        self.func_80832F54(flags);
    }

    /// `func_80833114`: `func_808330AC` at 2/3: the animation looped with `moveFlags`.
    fn func_80833114(&mut self, data: &GameData, anim: AnimId, flags: u16) {
        self.skel.play_loop_set_speed(data, anim, 2.0 / 3.0);
        self.func_80832F54(flags);
    }

    /// `func_80851294`: at the animation's end, `func_8083313C`, `unk_850` 1.
    fn func_80851294(&mut self, data: &GameData, anim: AnimId) {
        if self.skel.update(data) {
            self.func_80833114(data, anim, 0x1C);
            self.unk_850 = 1;
        }
    }

    /// `func_80851F14`: at the animation's end the next one looped at 2/3; before it, its
    /// sounds.
    fn func_80851F14(&mut self, env: &Env, anim: AnimId, table: &str) {
        let data = env.data;
        if self.skel.update(data) {
            self.skel.play_loop_set_speed(data, anim, 2.0 / 3.0);
            self.unk_850 = 1;
        } else if self.unk_850 == 0 {
            let t = env.audio.player_anim_sfx(table).to_vec();
            self.func_80832924(&t);
        }
    }

    /// `func_80852414`: `func_80851294`, and until its end, the sounds.
    fn func_80852414(&mut self, env: &Env, anim: AnimId, table: &str) {
        self.func_80851294(env.data, anim);
        if self.unk_850 == 0 {
            let t = env.audio.player_anim_sfx(table).to_vec();
            self.func_80832924(&t);
        }
    }

    /// `func_808520BC`: Link along the cue, from its start to its end by the script's frame.
    fn func_808520BC(&mut self, env: &Env, cue: &CsCmdActorAction) {
        let start = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
        let dist = Vec3::new(cue.end_pos.x as f32, cue.end_pos.y as f32, cue.end_pos.z as f32) - start;
        let sp4 = (env.cs_frames as i32 - cue.start_frame as i32) as f32 / (cue.end_frame as i32 - cue.start_frame as i32) as f32;
        self.actor.world_pos = dist * sp4 + start;
    }

    /// The cutscene modes' functions (`struct_80854B18`'s `func`), by name.
    fn cs_mode_func(&mut self, env: &Env, cue: Option<CsCmdActorAction>, name: &str) {
        let data = env.data;
        match name {
            "func_808515A4" => self.func_808515A4(env),
            "func_808514C0" => self.func_808514C0(env),
            "func_8085157C" | "func_80852234" => {
                self.skel.update(data);
            }
            "func_80851998" => self.func_80851998(env, cue),
            "func_808519C0" => self.func_808519C0(env, cue),
            "func_80852C50" => self.func_80852C50(env),
            "func_80852944" => self.func_80852944(env),
            "func_80851688" => self.func_80851688(env),
            "func_80851750" => {
                self.skel.update(data);
                let t = env.audio.player_anim_sfx("D_80855188").to_vec();
                self.func_80832924(&t);
            }
            "func_80851788" => {
                // Walking to unk_450 (func_80851828 after it).
                self.state1 &= !STATE1_25;
                // Math_Vec3f_Yaw.
                let yaw = vec3f_yaw(self.actor.world_pos, self.unk_450);
                self.current_yaw = yaw;
                self.actor.shape_rot.y = yaw;
                self.actor.world_rot.y = yaw;
                if self.linear_velocity <= 0.0 {
                    self.linear_velocity = 0.1;
                } else if self.linear_velocity > 2.5 {
                    self.linear_velocity = 2.5;
                }
            }
            "func_80851828" => {
                let mut sp1c = 2.5;
                self.func_80845BA0(env, &mut sp1c, 10);
                // SCENE_BDAN_BOSS (Jabu-Jabu's boss room): wait on the message box.
                if env.scene_id == 0x12 {
                    let none = env.msg_state == oot_game::message::TEXT_STATE_NONE;
                    if (self.unk_850 == 0 && none) || (self.unk_850 != 0 && !none) {
                        return;
                    }
                }
                self.unk_850 += 1;
                if self.unk_850 > 20 {
                    self.cs_mode = 0xB;
                }
            }
            "func_808518DC" => self.func_8083CEAC(data),
            "func_8085190C" => {
                self.func_80851314(env);
                if self.unk_850 != 0 {
                    if self.skel.update(data) {
                        let a = self.func_808334E4(data);
                        self.skel.play_loop(data, a);
                        self.unk_850 = 0;
                    }
                    self.func_80833C3C();
                } else {
                    self.func_808401B0(data);
                }
            }
            "func_808519EC" => {
                // D_80855198, facing -z, the age's unk_9C at 2/3, moveFlags 0x28F.
                self.actor.world_pos = Vec3::new(-1.0, 70.0, 20.0);
                self.actor.shape_rot.y = i16::MIN;
                let a = self.age.climb.unk_9C;
                self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
                self.func_80832F54(0x28F);
            }
            "func_80851B90" => {
                let a = data.anim("link_demo_warp");
                self.skel.change(data, a, -(2.0 / 3.0), 12.0, 12.0, ANIMMODE_ONCE, 0.0);
            }
            "func_80851BE8" => {
                self.skel.update(data);
                self.unk_850 += 1;
                if self.unk_850 >= 180 {
                    if self.unk_850 == 180 {
                        let a = data.anim("link_okarina_warp_goal");
                        let last = data.anims[a].last_frame();
                        self.skel.change(data, a, 2.0 / 3.0, 10.0, last, ANIMMODE_ONCE, -8.0);
                    }
                    let t = env.audio.player_anim_sfx("D_808551B4").to_vec();
                    self.func_80832924(&t);
                }
            }
            "func_80851CA4" => {
                if self.skel.update(data) && self.unk_850 == 0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                    self.skel.play_once(data, data.anim("link_normal_back_downB"));
                    self.unk_850 = 1;
                }
                if self.unk_850 != 0 {
                    self.func_8083721C();
                }
            }
            "func_80851DEC" | "func_80851E28" => {
                // Math_StepToS(&this->actor.shape.face, 0 or 2, 1).
                self.skel.update(data);
                let target = if name == "func_80851DEC" { 0 } else { 2 };
                if self.face < target {
                    self.face += 1;
                } else if self.face > target {
                    self.face -= 1;
                }
            }
            "func_80851E64" => self.func_80833064(data, data.anim("link_swimer_swim_get"), 0x98),
            "func_80851E90" => self.func_80851E90(data),
            "func_80851ECC" => self.func_80851ECC(data),
            "func_80851F84" => self.func_80851F84(data),
            "func_80851FB0" => self.func_80851FB0(env),
            "func_80852048" => {
                self.skel.update(data);
                let t = env.audio.player_anim_sfx("D_808551C8").to_vec();
                self.func_80832924(&t);
            }
            "func_80852080" => {
                self.func_80833064(data, data.anim("clink_demo_futtobi"), 0x9D);
                self.func_80832698(NA_SE_VO_LI_FALL_L);
            }
            "func_80852174" => {
                if let Some(c) = cue {
                    self.func_808520BC(env, &c);
                }
                self.skel.update(data);
                let t = env.audio.player_anim_sfx("D_808551D8").to_vec();
                self.func_80832924(&t);
            }
            "func_808521B8" => {
                if let Some(c) = cue {
                    self.func_808520BC(env, &c);
                }
                self.skel.update(data);
            }
            "func_808521F4" => {
                let a = self.anim(data, 44);
                self.func_80832B0C(data, a);
                self.func_80832210();
            }
            "func_8085225C" => self.func_80832F54(0x98),
            // func_80852280: actor.draw = Player_Draw (Player is always drawn here).
            "func_80852280" | "func_80852544" | "func_80852554" => {}
            "func_80852328" => self.func_80851F14(env, data.anim("link_demo_furimuki2_wait"), "D_808551E0"),
            "func_80852358" => self.func_80851F14(env, data.anim("link_demo_nozokikomi_wait"), "D_808551E8"),
            "func_80852388" => {
                if self.skel.update(data) {
                    self.skel.play_loop_set_speed(data, data.anim("demo_link_twait"), 2.0 / 3.0);
                    self.unk_850 = 1;
                }
                // (rightHandType: the hand's model, PLAYER_MODELTYPE_LH_OPEN at 900 or RH_FF.)
            }
            "func_80852450" => self.func_80852414(env, data.anim("clink_demo_koutai_wait"), "D_808551F0"),
            "func_80852480" => self.func_80852414(env, data.anim("link_demo_kakeyori_wait"), "D_808551F8"),
            "func_80852564" => {
                self.state3 |= STATE3_1;
                self.linear_velocity = 2.0;
                self.actor.velocity.y = -1.0;
                self.skel.play_once(data, data.anim("link_normal_back_downA"));
                self.func_80832698(NA_SE_VO_LI_FALL_L);
            }
            "func_808525C0" => match self.unk_850 {
                // D_808551FC: the knockdown's three stages.
                0 => self.func_8084377C(env),
                1 => self.func_80843954(env),
                2 => self.func_80843A38(env),
                _ => {}
            },
            n => self.note(format!("cutscene mode function {n} not ported (swimming, the ocarina, the items in hand or the effects)")),
        }
    }

    /// `func_80850ED8` (type 2): the animation once (`func_80832B0C`), standing.
    fn func_80850ED8(&mut self, data: &GameData, anim: AnimId) {
        self.func_80832DB0();
        self.func_80832B0C(data, anim);
        self.func_80832210();
    }

    /// `func_8083303C`: `func_80832FFC` at speed 1: the animation once with `moveFlags`.
    fn func_8083303C(&mut self, data: &GameData, anim: AnimId, flags: u16) {
        self.skel.play_once_set_speed(data, anim, 1.0);
        self.func_80832F54(flags);
    }

    /// `func_808330EC`: `func_808330AC` at speed 1: the animation looped with `moveFlags`.
    fn func_808330EC(&mut self, data: &GameData, anim: AnimId, flags: u16) {
        self.skel.play_loop_set_speed(data, anim, 1.0);
        self.func_80832F54(flags);
    }

    /// `func_80851F84` (mode 38's start): asleep in bed, no shadow, `clink_op3_wait1` looped
    /// (`func_80851134`: `func_808330EC(play, this, anim, 0x9C)`).
    fn func_80851F84(&mut self, data: &GameData) {
        self.shadow_feet = false;
        self.func_808330EC(data, data.anim("clink_op3_wait1"), 0x9C);
    }

    /// `func_80851E90` (mode 39's start): tossing (`clink_op3_negaeri`), groaning.
    fn func_80851E90(&mut self, data: &GameData) {
        self.func_8083303C(data, data.anim("clink_op3_negaeri"), 0x9C);
        self.func_80832698(NA_SE_VO_LI_GROAN);
    }

    /// `func_80851ECC` (mode 39): after the toss, `clink_op3_wait2` looped.
    fn func_80851ECC(&mut self, data: &GameData) {
        if self.skel.update(data) {
            self.func_808330EC(data, data.anim("clink_op3_wait2"), 0x9C);
        }
    }

    /// `func_80851FB0` (mode 40): sitting up, with `D_808551BC`'s sounds (the sigh, the slips
    /// off the bed); then `clink_op3_wait3` looped. The shadow comes back on frame 240
    /// (`ActorShadow_DrawFeet`).
    fn func_80851FB0(&mut self, env: &Env) {
        let data = env.data;
        if self.skel.update(data) {
            self.func_808330EC(data, data.anim("clink_op3_wait3"), 0x9C);
            self.unk_850 = 1;
        } else if self.unk_850 == 0 {
            let t = env.audio.player_anim_sfx("D_808551BC").to_vec();
            self.func_80832924(&t);
            if self.skel.on_frame(240.0) {
                self.shadow_feet = true;
            }
        }
    }

    /// `func_80852C0C`: modes other than 1, 8, 0x31 and 7 drop what Link holds
    /// (`func_808323B4`: nothing is ever held here).
    fn func_80852C0C(&mut self, _cs_mode: u8) {}

    /// `func_80852C50` (mode 6): Link follows the script's cues (`linkAction`), each mapped to
    /// a mode by `D_808547C4`. A cue's start puts him at its start (`func_808529D0`) or, for the
    /// walks (3, 4), only if he's far from it (`func_80852A54`). When the script ends
    /// (`CS_STATE_UNSKIPPABLE_INIT`), mode 7.
    fn func_80852C50(&mut self, env: &Env) {
        let link = env.cs_link_action;
        if env.cs_state == oot_game::cutscene::CS_STATE_UNSKIPPABLE_INIT {
            self.func_8002DF54(7);
            self.unk_446 = 0;
            self.func_80832210();
            return;
        }
        let Some(link) = link else {
            self.actor.flags &= !ACTOR_FLAG_6;
            return;
        };
        if self.unk_446 != link.action {
            let sp24 = d_808547c4(link.action);
            if sp24 >= 0 {
                if sp24 == 3 || sp24 == 4 {
                    self.func_80852A54(env, &link);
                } else {
                    self.func_808529D0(env, &link);
                }
            }
            self.s.d_80858aa0 = self.skel.move_flags as i32;
            self.func_80832DBC();
            log::debug!("TOOL MODE={sp24}");
            let m = sp24.unsigned_abs();
            self.func_80852C0C(m);
            self.func_80852B4C(env, Some(link), m, true);
            self.unk_850 = 0;
            self.unk_84F = 0;
            self.unk_446 = link.action;
        }
        let m = d_808547c4(self.unk_446).unsigned_abs();
        self.func_80852B4C(env, Some(link), m, false);
    }

    /// `func_808529D0`: Link at the cue's start (a child in Kokiri Forest 1 lower), facing its
    /// rotation.
    fn func_808529D0(&mut self, env: &Env, cue: &CsCmdActorAction) {
        self.actor.world_pos.x = cue.start_pos.x as f32;
        self.actor.world_pos.y = cue.start_pos.y as f32;
        if env.scene_id == SCENE_SPOT04 && !self.adult {
            self.actor.world_pos.y -= 1.0;
        }
        self.actor.world_pos.z = cue.start_pos.z as f32;
        self.actor.shape_rot.y = cue.rot[1];
        self.current_yaw = cue.rot[1];
        // Put there at once: the renderer doesn't blend Link across the jump.
        self.actor.teleported = true;
    }

    /// `func_80852A54`: a walk cue moves Link to its start only if he stands more than 50 from
    /// it or facing more than a quarter turn away.
    fn func_80852A54(&mut self, env: &Env, cue: &CsCmdActorAction) {
        let dx = cue.start_pos.x as f32 - self.actor.world_pos.x as i32 as f32;
        let dy = cue.start_pos.y as f32 - self.actor.world_pos.y as i32 as f32;
        let dz = cue.start_pos.z as f32 - self.actor.world_pos.z as i32 as f32;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        let yaw_diff = cue.rot[1].wrapping_sub(self.actor.shape_rot.y);
        if self.linear_velocity == 0.0 && (dist > 50.0 || abs16(yaw_diff) > 0x4000) {
            self.func_808529D0(env, cue);
        }
        self.skel.move_flags = 0;
        self.func_80832DB0();
    }

    /// `func_80832DB0`.
    fn func_80832DB0(&mut self) {
        self.skel.joint[1][1] = 0;
    }

    /// `func_808515A4` (the start of modes 1, 8 and 0x31): Link stands in the cutscene's wait
    /// (`PLAYER_ANIMGROUP_44`), looping. (Swimming, `func_80851368`, isn't ported.)
    fn func_808515A4(&mut self, env: &Env) {
        let data = env.data;
        if self.func_808332B8() {
            self.note("cutscene hold while swimming (func_80851368) not ported");
            return;
        }
        let anim = self.anim(data, 44);
        if self.unk_446 == 6 || self.unk_446 == 0x2E {
            // func_80832264: LinkAnimation_PlayOnce.
            self.skel.play_once(data, anim);
        } else {
            self.func_80832DB0();
            let last = data.anims[anim].last_frame();
            self.skel.change(data, anim, 2.0 / 3.0, 0.0, last, ANIMMODE_LOOP, -4.0);
        }
        self.func_80832210();
    }

    /// `func_80851314`: Link faces the actor the mode is about (`unk_448`), if it's still there.
    fn func_80851314(&mut self, env: &Env) {
        if self.unk_448.is_some_and(|h| env.target(h).is_none_or(|a| a.killed)) {
            self.unk_448 = None;
        }
        self.unk_664 = self.unk_448;
        if self.unk_664.is_some() {
            self.actor.shape_rot.y = self.func_8083DB98(env, false);
        }
    }

    /// `func_808514C0` (mode 1, talking to an actor with no text): facing it, the animation,
    /// and a picked-up item offer (`textId` 0xFFFF).
    fn func_808514C0(&mut self, env: &Env) {
        self.func_80851314(env);
        if self.func_808332B8() {
            self.note("cutscene mode 1 while swimming (func_808513BC) not ported");
            return;
        }
        self.skel.update(env.data);
        // func_8008F128 (the hookshot or boomerang out) and PLAYER_STATE1_11: nothing held.
        if self.state1 & STATE1_11 != 0 {
            self.func_80836670(env);
            return;
        }
        if self.interact_range_actor.and_then(|h| env.target(h)).is_some_and(|a| a.text_id == 0xFFFF) {
            self.func_8083E5A8(env);
        }
    }

    /// `func_80851688` (modes 8 and 0x31): held still, the wait playing; mode 0x31 ends by
    /// itself once no script runs (mode 7). (`func_8084B3CC`, the shooting gallery, never
    /// applies.)
    fn func_80851688(&mut self, env: &Env) {
        if self.cs_mode == 0x31 && env.cs_state == oot_game::cutscene::CS_STATE_IDLE {
            self.func_8002DF54(7);
            return;
        }
        if self.func_808332B8() {
            self.note("cutscene hold while swimming (func_808513BC) not ported");
            return;
        }
        self.skel.update(env.data);
        if self.state1 & STATE1_11 != 0 {
            self.func_80836670(env);
        }
    }

    /// `func_80851998` (mode 3): `func_80845964(play, this, arg2, 0.0f, 0, 0)`.
    fn func_80851998(&mut self, env: &Env, cue: Option<CsCmdActorAction>) {
        self.func_80845964_cs(env, cue, 0.0, 0, 0);
    }

    /// `func_808519C0` (mode 4): `func_80845964(play, this, arg2, 0.0f, 0, 1)`.
    fn func_808519C0(&mut self, env: &Env, cue: Option<CsCmdActorAction>) {
        self.func_80845964_cs(env, cue, 0.0, 0, 1);
    }

    /// `func_80845964` with a cue (`arg5` 0 or 1): the walk towards the cue's end, at the speed
    /// that gets there by its end frame (`R_UPDATE_RATE` × 0.5 a unit); with `arg5` 1 it stops
    /// short, leaving four times the cue's own pace for the slow-down, and stands once still.
    fn func_80845964_cs(&mut self, env: &Env, cue: Option<CsCmdActorAction>, mut speed: f32, mut yaw: i16, arg5: i32) -> bool {
        let data = env.data;
        if arg5 != 0 && self.linear_velocity == 0.0 {
            return self.skel.update(data);
        }
        if arg5 != 2
            && let Some(c) = cue
        {
            let sp34 = 3.0 * 0.5;
            let self_dist_x = c.end_pos.x as f32 - self.actor.world_pos.x;
            let self_dist_z = c.end_pos.z as f32 - self.actor.world_pos.z;
            let sp28 = (self_dist_x * self_dist_x + self_dist_z * self_dist_z).sqrt() / sp34;
            let sp24 = (c.end_frame as i32 - env.cs_frames as i32) + 1;
            yaw = atan2_s(self_dist_z, self_dist_x);
            if arg5 == 1 {
                let dist_x = (c.end_pos.x - c.start_pos.x) as f32;
                let dist_z = (c.end_pos.z - c.start_pos.z) as f32;
                let temp = ((((dist_x * dist_x + dist_z * dist_z).sqrt() / sp34) / (c.end_frame as i32 - c.start_frame as i32) as f32) / 1.5 * 4.0) as i32;
                if temp >= sp24 {
                    yaw = self.actor.shape_rot.y;
                    speed = 0.0;
                } else {
                    speed = sp28 / ((sp24 - temp) + 1) as f32;
                }
            } else {
                speed = sp28 / sp24 as f32;
            }
        }
        self.state2 |= STATE2_5;
        self.func_80841EE4(data);
        self.func_8083DF68(speed, yaw);
        if speed == 0.0 && self.linear_velocity == 0.0 {
            self.func_8083BF50(data);
        }
        false
    }

    /// `func_80852944` (mode 7): the cutscene is over: standing (or treading water), then the
    /// talk and pick-up interrupts, and `csMode` 0.
    fn func_80852944(&mut self, env: &Env) {
        let data = env.data;
        if self.func_808332B8() {
            self.func_80838F18(data);
            self.func_80832340();
        } else {
            self.func_8083C148(data);
            if !self.func_8083B644(env) {
                self.func_8083E5A8(env);
            }
        }
        self.cs_mode = 0;
        self.unk_6AD = 0;
    }

    /// `func_8083C148`.
    fn func_8083C148(&mut self, data: &GameData) {
        if self.state3 & STATE3_7 == 0 {
            self.func_8083B010();
            if self.state1 & STATE1_27 != 0 {
                self.func_80838F18(data);
            } else {
                self.func_80839F90(data);
            }
            if self.unk_6AD < 4 {
                self.unk_6AD = 0;
            }
        }
        self.state1 &= !(STATE1_13 | STATE1_14 | STATE1_20);
    }

    /// `func_8083B010`: the head and upper body straight, the focus facing Link's way.
    fn func_8083B010(&mut self) {
        self.actor.focus_rot = Rot { x: 0, y: self.actor.shape_rot.y, z: 0 };
        self.unk_6B6 = 0;
        self.unk_6B8 = 0;
        self.unk_6BA = 0;
        self.unk_6BC = 0;
        self.unk_6BE = 0;
        self.unk_6C0 = 0;
    }

    /// `func_8083B644` (interrupt 4): A talks to the actor that offered this frame
    /// (`targetActor`, from `func_8002F1C4`), or C-Up talks to Navi: about the target, or her
    /// own text (`naviTextId`, at once when negative). On the ground (or swimming at the
    /// surface), when nothing else is targeted.
    ///
    /// No ported actor has a `naviEnemyId`, so a target Navi speaks about is an
    /// `ACTOR_FLAG_0 | ACTOR_FLAG_18` one. Holding actors isn't ported (`PLAYER_STATE1_11` is
    /// never set).
    fn func_8083B644(&mut self, env: &Env) -> bool {
        let mut sp34 = self.target_actor;
        let mut sp30 = self.unk_664;
        let mut sp2c: Option<ActorHandle> = None;
        let navi_actor: Option<ActorHandle> = self.navi_actor;
        let checkable = |h: Option<ActorHandle>| h.and_then(|h| env.actors.actor(h)).is_some_and(|a| a.flags & (ACTOR_FLAG_0 | ACTOR_FLAG_18) == (ACTOR_FLAG_0 | ACTOR_FLAG_18));
        let sp24 = checkable(sp30);
        let mut sp28 = false;
        if sp24 || self.navi_text_id != 0 {
            sp28 = self.navi_text_id < 0 && ((self.navi_text_id as i32).abs() & 0xFF00) != 0x200;
            if sp28 || !sp24 {
                sp2c = navi_actor;
                if sp28 {
                    sp30 = None;
                    sp34 = None;
                }
            } else {
                sp2c = sp30;
            }
        }
        if sp34.is_none() && sp2c.is_none() {
            return false;
        }
        if !(sp30.is_none() || sp30 == sp34 || sp30 == sp2c) {
            return false;
        }
        if self.state1 & STATE1_11 != 0 {
            return false;
        }
        if !(self.grounded() || self.state1 & STATE1_23 != 0 || (self.func_808332B8() && self.state2 & STATE2_10 == 0)) {
            return false;
        }
        if let Some(t) = sp34 {
            self.state2 |= STATE2_1;
            let flag16 = env.actors.actor(t).is_some_and(|a| a.flags & ACTOR_FLAG_16 != 0);
            if self.input.press.held(BTN_A) || flag16 {
                sp2c = None;
            } else if sp2c.is_none() {
                return false;
            }
        }
        let mut text_override = None;
        if let Some(n) = sp2c {
            if !sp28 {
                self.state2 |= STATE2_21;
            }
            if !self.input.press.held(eng_input::pad::BTN_CUP) && !sp28 {
                return false;
            }
            sp34 = Some(n);
            self.target_actor = None;
            if sp28 || !sp24 {
                let id = if self.navi_text_id >= 0 { self.navi_text_id } else { -self.navi_text_id };
                text_override = Some(id as u16);
            }
            // (sp2c->naviEnemyId + 0x600: no actor has one.)
        }
        let Some(t) = sp34 else { return false };
        let Some(text_id) = text_override.or_else(|| env.actors.actor(t).map(|a| a.text_id)) else { return false };
        if let Some(id) = text_override {
            self.play_requests.push(PlayRequest::SetTextId { actor: t, text_id: id });
        }
        // this->currentMask = D_80858AA4 (masks aren't ported).
        self.func_80853148(env, t, text_id);
        true
    }

    /// `func_80853148`: start talking with `actor` (whose `textId` is `text_id`). The actor
    /// gets the talk request (`ACTOR_FLAG_8`) and Player its text; an NPC is faced after
    /// putting the item away, anything else at once. Link steps back first when closer than
    /// 40.
    fn func_80853148(&mut self, env: &Env, h: ActorHandle, text_id: u16) {
        let data = env.data;
        let Some(actor) = env.actors.actor(h) else { return };
        let (category, xz_dist, flags) = (actor.category, actor.xz_dist_to_player, actor.flags);
        let is_navi = self.navi_actor == Some(h);
        if self.target_actor.is_some() || is_navi || flags & (ACTOR_FLAG_0 | ACTOR_FLAG_18) == (ACTOR_FLAG_0 | ACTOR_FLAG_18) {
            self.play_requests.push(PlayRequest::TalkRequest(h));
        }
        self.target_actor = Some(h);
        self.exchange_item_id = 0;
        if text_id == 0xFFFF {
            // func_8002DF54(play, actor, 1): a cutscene's talk, not ported.
            self.note("a talk with text 0xFFFF (a cutscene) isn't ported");
            self.play_requests.push(PlayRequest::TalkRequest(h));
            self.func_80832528(data);
            return;
        }
        if self.actor.flags & ACTOR_FLAG_8 != 0 {
            self.actor.text_id = 0;
        } else {
            self.actor.flags |= ACTOR_FLAG_8;
            self.actor.text_id = text_id;
        }
        // (Riding isn't ported.)
        let backspace = data.anim("link_normal_backspace");
        if self.func_808332B8() {
            self.func_80836898(data, env, A74::Talk);
            self.func_80832C6C(data, data.anim("link_swimer_swim_wait"));
        } else if category != ACTORCAT_NPC {
            // (Or the fishing pole in hand, which isn't ported.)
            self.func_8083A2F8(data);
            if self.state1 & STATE1_4 == 0 {
                if !is_navi && xz_dist < 40.0 {
                    self.skel.play_once_set_speed(data, backspace, 2.0 / 3.0);
                } else {
                    let a = self.anim(data, group::WAIT);
                    self.skel.play_loop(data, a);
                }
            }
        } else {
            self.func_80836898(data, env, A74::Talk);
            let a = if xz_dist < 40.0 { backspace } else { data.anim("link_normal_talk_free") };
            self.skel.play_once_set_speed(data, a, 2.0 / 3.0);
        }
        if self.skel.animation == backspace {
            self.func_80832F54(0x19);
        }
        self.func_80832224();
        self.state1 |= STATE1_6 | STATE1_29;
        // Navi's own text (not a 0x2xx one): her talk request, and the camera turns round to
        // her (func_80835EA4(play, 0xB)).
        if is_navi && self.target_actor == Some(h) && (text_id & 0xFF00) != 0x200 {
            self.play_requests.push(PlayRequest::TalkRequest(h));
            self.func_80835EA4(0xB);
        }
    }

    /// `func_8083A2F8`: the talking action, and the message box for Player's `textId`.
    fn func_8083A2F8(&mut self, data: &GameData) {
        self.func_80835DAC(data, Action::Talk, 0);
        self.state1 |= STATE1_6 | STATE1_29;
        if self.actor.text_id != 0 {
            self.play_requests.push(PlayRequest::StartTextbox { text_id: self.actor.text_id, actor: self.target_actor });
            self.unk_664 = self.target_actor;
        }
    }

    /// `func_8084B530`: talking, facing the target, until the box closes; then standing (or
    /// treading water), with A, B and C-Up ignored for 10 frames (`unk_88E`).
    fn func_8084B530(&mut self, env: &Env) {
        let data = env.data;
        self.state2 |= STATE2_5;
        self.func_80836670(env);
        let target = self.target_actor.and_then(|h| env.actors.actor(h));
        if env.msg_state == oot_game::message::TEXT_STATE_CLOSING {
            self.actor.flags &= !ACTOR_FLAG_8;
            if !target.is_some_and(|t| t.flags & (ACTOR_FLAG_0 | ACTOR_FLAG_2) == (ACTOR_FLAG_0 | ACTOR_FLAG_2)) {
                self.state2 &= !STATE2_13;
            }
            self.play_requests.push(PlayRequest::CamDone);
            // func_8084B4D4 (the ocarina after a talk) and func_8084B3CC (the shooting gallery)
            // don't apply; func_8083ADD4 (a cutscene mode waiting) does; func_8083E5A8 (an item
            // to pick up) doesn't apply.
            if self.func_8083ADD4(data) {
                self.unk_88E = 10;
                return;
            }
            if self.func_808332B8() {
                self.func_80838F18(data);
            } else {
                self.func_80853080(data);
            }
            self.unk_88E = 10;
            return;
        }
        if self.func_808332B8() {
            self.func_8084D610(env);
        } else if self.state1 & STATE1_4 == 0 && self.skel.update(data) {
            if self.skel.move_flags != 0 {
                self.func_80832DBC();
                if target.is_some_and(|t| t.category == ACTORCAT_NPC) {
                    self.skel.play_once_set_speed(data, data.anim("link_normal_talk_free"), 2.0 / 3.0);
                } else {
                    let a = self.anim(data, group::WAIT);
                    self.skel.play_loop(data, a);
                }
            } else {
                self.skel.play_loop_set_speed(data, data.anim("link_normal_talk_free_wait"), 2.0 / 3.0);
            }
        }
        if self.unk_664.is_some() {
            let y = self.func_8083DB98(env, false);
            self.actor.shape_rot.y = y;
            self.current_yaw = y;
        }
    }

    // ================================================================================
    // Getting items (func_8083E5A8 and the get-item action func_8084E6D4)

    /// `func_8083E5A8` (interrupt 2): take what `interactRangeActor` offers.
    /// - A positive get-item id (a collectible, an NPC's gift): if it would give something new
    ///   (`Item_CheckObtainability`), Link holds it up (`func_8083A434` after the item is put
    ///   away, `link_demo_get_itemB`, the turn-around camera); else it's given at once
    ///   (`func_8083E4C4`).
    /// - A negative one, on A: a chest. Link stands 29.4343 in front of it facing its way. A new
    ///   major item gets the long opening (`ageProperties->unk_98`, the chest's long animation
    ///   and `CAM_SET_SLOW_CHEST_CS`); anything else the kick open (`link_normal_box_kick`).
    ///   Items the table marks (0x40: a blue rupee if not obtainable, 0x20: if had) become a blue
    ///   rupee.
    /// - `GI_NONE` on A: picking up (bushes, rocks, the Master Sword): not ported.
    ///
    /// Title cards (`TitleCard_Clear`) aren't ported: there's always none to clear. `iREG(67)`'s
    /// debug item is 0.
    fn func_8083E5A8(&mut self, env: &Env) -> bool {
        use oot_game::item::*;
        let Some(ih) = self.interact_range_actor else { return false };
        if self.get_item_id > GI_NONE {
            if self.get_item_id < GI_MAX {
                let Some(gi) = env.items.get_item(self.get_item_id).copied() else { return false };
                if Some(ih) != env.me {
                    self.play_requests.push(PlayRequest::SetParent(ih));
                }
                /// `SCENE_BOWLING`.
                const SCENE_BOWLING: u16 = 0x4B;
                let (obtainable, scene) = {
                    let io = env.io.borrow();
                    (item_check_obtainability(&io.save, gi.item_id), io.scene_id)
                };
                if obtainable == ITEM_NONE || scene == SCENE_BOWLING {
                    // func_808323B4 (no held actor or explosive), func_8083AE40 (the item's
                    // object: every get-item model is baked).
                    // PLAYER_STATE2_10 is underwater (Kokiri boots, not iron).
                    if self.state2 & STATE2_10 == 0 {
                        self.func_80836898(env.data, env, A74::GetItem);
                        self.skel.play_once_set_speed(env.data, env.data.anim("link_demo_get_itemB"), 2.0 / 3.0);
                        self.func_80835EA4(9);
                    }
                    self.state1 |= STATE1_10 | STATE1_11 | STATE1_29;
                    self.func_80832224();
                    return true;
                }
                self.func_8083E4C4(gi);
                self.get_item_id = GI_NONE;
            }
        } else if self.input.press.held(BTN_A) && self.state1 & STATE1_11 == 0 && self.state2 & STATE2_10 == 0 {
            if self.get_item_id != GI_NONE {
                let Some(mut gi) = env.items.get_item(-self.get_item_id).copied() else { return false };
                let Some(chest) = env.target(ih) else { return false };
                let (chest_pos, chest_yaw) = (chest.world_pos, chest.shape_rot.y);
                let obtainable = |id: u8| item_check_obtainability(&env.io.borrow().save, id);
                if gi.item_id != ITEM_NONE && ((obtainable(gi.item_id) == ITEM_NONE && gi.field & 0x40 != 0) || (obtainable(gi.item_id) != ITEM_NONE && gi.field & 0x20 != 0)) {
                    self.get_item_id = -GI_RUPEE_BLUE;
                    gi = env.items.get_item(GI_RUPEE_BLUE).copied().unwrap_or(gi);
                }
                self.func_80836898(env.data, env, A74::GetItem);
                self.state1 |= STATE1_10 | STATE1_11 | STATE1_29;
                self.actor.world_pos.x = chest_pos.x - sin_s(chest_yaw) * 29.4343;
                self.actor.world_pos.z = chest_pos.z - cos_s(chest_yaw) * 29.4343;
                self.current_yaw = chest_yaw;
                self.actor.shape_rot.y = chest_yaw;
                self.func_80832224();
                if gi.item_id != ITEM_NONE && gi.gi >= 0 && obtainable(gi.item_id) == ITEM_NONE {
                    let a = self.age.climb.unk_98;
                    self.skel.play_once_set_speed(env.data, a, 2.0 / 3.0);
                    self.func_80832F54(0x28F);
                    self.play_requests.push(PlayRequest::ChestOpen { chest: ih, unk_1f4: 1 });
                    self.play_requests.push(PlayRequest::ChangeSetting(oot_game::camera::CAM_SET_SLOW_CHEST_CS));
                } else {
                    self.skel.play_once(env.data, env.data.anim("link_normal_box_kick"));
                    self.play_requests.push(PlayRequest::ChestOpen { chest: ih, unk_1f4: -1 });
                }
                return true;
            }
            // Picking up (lifting, Bg_Toki_Swd): not ported.
            self.note("picking up (func_8083E5A8 with GI_NONE) isn't ported");
        }
        false
    }

    /// `func_8083E4C4`: an item already had is given without holding it up: a drop over Link's
    /// head (unless the table says 0x80), and `Item_Give` unless the drop gives it itself.
    fn func_8083E4C4(&mut self, gi: oot_game::item::GetItemEntry) {
        use crate::en_item00::*;
        let drop_type = (gi.field & 0x1F) as i16;
        if gi.field & 0x80 == 0 {
            self.play_requests.push(PlayRequest::DropCollectible { pos: self.actor.world_pos, params: drop_type | i16::MIN });
            if !matches!(drop_type, ITEM00_BOMBS_A | ITEM00_ARROWS_SMALL | ITEM00_ARROWS_MEDIUM | ITEM00_ARROWS_LARGE | ITEM00_RUPEE_GREEN | ITEM00_RUPEE_BLUE | ITEM00_RUPEE_RED | ITEM00_RUPEE_PURPLE | ITEM00_RUPEE_ORANGE) {
                self.play_requests.push(PlayRequest::ItemGive(gi.item_id));
            }
        } else {
            self.play_requests.push(PlayRequest::ItemGive(gi.item_id));
        }
        let id = if self.get_item_id < 0 { oot_game::audio::sfx::NA_SE_SY_GET_BOXITEM } else { oot_game::audio::sfx::NA_SE_SY_GET_ITEM };
        self.sfx(PlayerSfx::NoPos(id));
    }

    /// `func_80835EA4`: the turn-around camera (`func_80835E44(CAM_SET_TURN_AROUND)`), told what
    /// kind of item it's for (`Camera_SetCameraData(4, arg1)`: 9 for a held-up item, 8 when
    /// surfacing with one).
    fn func_80835EA4(&mut self, arg1: i16) {
        self.play_requests.push(PlayRequest::CamSetting(oot_game::camera::CAM_SET_TURN_AROUND));
        self.play_requests.push(PlayRequest::SetCameraData { data2: arg1 });
    }

    /// `func_8083A434`: the get-item action, once the item in hand is away. A heart container
    /// holds 20 frames longer; a chest's item (negative) first waits for the opening animation.
    fn func_8083A434(&mut self, data: &GameData) {
        self.func_80835DAC(data, Action::GetItem, 0);
        self.state1 |= STATE1_10 | STATE1_29;
        if self.get_item_id == oot_game::item::GI_HEART_CONTAINER_2 {
            self.unk_850 = 20;
        } else if self.get_item_id >= 0 {
            self.unk_850 = 1;
        } else {
            self.get_item_id = -self.get_item_id;
        }
    }

    /// `func_808332F4`: the item appears over Link's head (`unk_862`, its draw id plus one).
    fn func_808332F4(&mut self, env: &Env) {
        if let Some(gi) = env.items.get_item(self.get_item_id) {
            self.unk_862 = (gi.gi as i16).abs();
        }
    }

    /// `func_8084DF6C`: the item is put away; the camera is told the get-item is over.
    fn func_8084DF6C(&mut self) {
        self.unk_862 = 0;
        self.state1 &= !(STATE1_10 | STATE1_11);
        self.get_item_id = oot_game::item::GI_NONE;
        self.play_requests.push(PlayRequest::CamDone);
    }

    /// `func_8084DFAC`: and Link stands again.
    fn func_8084DFAC(&mut self, data: &GameData) {
        self.func_8084DF6C();
        self.func_808322FC();
        self.func_8083C0E8(data);
        self.current_yaw = self.actor.shape_rot.y;
    }

    /// `func_8084DFF4`: the first time, the item's text (`Message_StartTextbox` with Player as
    /// the talker), `Item_Give` and the item's fanfare; then waiting for the text to close.
    /// True once it has.
    fn func_8084DFF4(&mut self, env: &Env) -> bool {
        use oot_game::item::*;
        if self.get_item_id == GI_NONE {
            return true;
        }
        if self.unk_84F == 0 {
            let Some(gi) = env.items.get_item(self.get_item_id).copied() else { return true };
            self.unk_84F = 1;
            self.play_requests.push(PlayRequest::StartTextbox { text_id: gi.text_id as u16, actor: env.me });
            self.play_requests.push(PlayRequest::ItemGive(gi.item_id));
            self.play_requests.push(PlayRequest::GetItemFanfare(self.get_item_id));
        } else if env.msg_state == oot_game::message::TEXT_STATE_CLOSING {
            if self.get_item_id == GI_GAUNTLETS_SILVER {
                // The Silver Gauntlets' exit to the Desert Colossus (ENTR_SPOT11_0 with the
                // sandstorm and cutscene 0xFFF1): not ported.
                self.note("the Silver Gauntlets' exit to the Desert Colossus isn't ported");
            }
            self.get_item_id = GI_NONE;
        }
        false
    }

    /// `func_8084E6D4`: getting an item.
    /// - From a chest (`unk_850` 0): the opening animation plays (with the child's sounds);
    ///   at its end Link turns to hold the item up (`link_demo_get_itemA`, or `_itemB` after
    ///   the kick) with the turn-around camera.
    /// - Holding up (`get_itemB` turns Link to face the camera): on frame 21 the item appears
    ///   over his head; each time the animation ends the text runs (`func_8084DFF4`), and when
    ///   it's closed and the hearts have counted in, Link stands (`func_8084DFAC`), or talks to
    ///   whoever gave the item.
    ///
    /// An ice trap (`GI_ICE_TRAP`: `En_Clear_Tag`, the freeze damage) isn't ported.
    fn func_8084E6D4(&mut self, env: &Env) {
        let data = env.data;
        if self.skel.update(data) {
            if self.unk_850 != 0 {
                if self.unk_850 >= 2 {
                    self.unk_850 -= 1;
                }
                if self.func_8084DFF4(env) && self.unk_850 == 1 {
                    // (PLAYER_STATE3_5 never comes up.)
                    let cond = self.target_actor.is_some() && (self.exchange_item_id as i8) < 0;
                    let health_accumulator = env.io.borrow().save.health_accumulator;
                    if cond || health_accumulator == 0 {
                        if cond {
                            self.func_8084DF6C();
                            self.exchange_item_id = 0;
                            // func_8084B4D4 (an ocarina after the talk) is 0.
                            if let Some(t) = self.target_actor
                                && let Some(text) = env.actors.actor(t).map(|a| a.text_id)
                            {
                                self.func_80853148(env, t, text);
                            }
                        } else {
                            self.func_8084DFAC(data);
                        }
                    }
                }
            } else {
                self.func_80832DBC();
                if self.get_item_id == oot_game::item::GI_ICE_TRAP {
                    self.note("an ice trap (GI_ICE_TRAP) isn't ported");
                }
                let a = if self.skel.animation == data.anim("link_normal_box_kick") { "link_demo_get_itemB" } else { "link_demo_get_itemA" };
                self.skel.play_once_set_speed(data, data.anim(a), 2.0 / 3.0);
                self.unk_850 = 2;
                self.func_80835EA4(9);
            }
        } else {
            if self.unk_850 == 0 {
                if !self.adult {
                    self.func_80832924(env.audio.player_anim_sfx("D_808549E0"));
                }
                return;
            }
            if self.skel.animation == data.anim("link_demo_get_itemB") {
                scaled_step_to_s(&mut self.actor.shape_rot.y, env.cam_dir_yaw.wrapping_add(i16::MIN), 4000);
            }
            if self.skel.on_frame(21.0) {
                self.func_808332F4(env);
            }
        }
    }

    /// `func_808473D4`: what the A button would do (`Interface_SetDoAction`), when no message
    /// is open (the crawlspace's "Enter" when `func_8083F0C8` lines Link up with one). Riding,
    /// the fishing pole, the ocarina and held actors aren't ported, so those actions never
    /// show. The ocarina's action (`func_8084E3C4`)
    /// never runs, so its exception doesn't apply.
    fn func_808473D4(&mut self, env: &Env) {
        use oot_game::interface::*;
        if env.msg_state != oot_game::message::TEXT_STATE_NONE {
            return;
        }
        let sp20 = self.unk_84B[self.unk_846 as usize];
        let sp1c = self.func_808332B8();
        let mut do_action = DO_ACTION_NONE;
        if !self.in_cs_mode() {
            if self.state1 & STATE1_20 != 0 {
                do_action = DO_ACTION_RETURN;
            } else if self.state2 & STATE2_18 == 0 {
                let target_category = self.target_actor.and_then(|h| env.actors.actor(h)).map(|a| a.category);
                if self.door_type != PLAYER_DOORTYPE_NONE && self.state1 & STATE1_11 == 0 {
                    do_action = DO_ACTION_OPEN;
                } else if self.state1 & STATE1_11 == 0
                    && self.interact_range_actor.is_some()
                    && ((!sp1c && self.get_item_id == oot_game::item::GI_NONE) || (self.get_item_id < 0 && self.state1 & STATE1_27 == 0))
                {
                    // A chest (a negative get-item id), or something to pick up (Bg_Toki_Swd's
                    // DO_ACTION_DROP for the adult isn't ported).
                    do_action = if self.get_item_id < 0 { DO_ACTION_OPEN } else { DO_ACTION_GRAB };
                } else if !sp1c && self.state2 & STATE2_0 != 0 {
                    do_action = DO_ACTION_GRAB;
                } else if self.state2 & STATE2_2 != 0 {
                    do_action = DO_ACTION_CLIMB;
                } else if self.state2 & STATE2_1 != 0 && target_category.is_some() {
                    do_action = if target_category == Some(ACTORCAT_NPC) { DO_ACTION_SPEAK } else { DO_ACTION_CHECK };
                } else if self.state1 & (STATE1_13 | STATE1_21) != 0 {
                    do_action = DO_ACTION_DOWN;
                } else if self.state2 & STATE2_16 != 0 {
                    do_action = DO_ACTION_ENTER;
                } else if self.state2 & STATE2_11 != 0 {
                    // D_80854784[CUR_UPG_VALUE(UPG_SCALE)] (no scale: 120) less the depth, per 40.
                    let sp24 = ((120.0 - self.actor.y_dist_to_water) / 40.0) as i32;
                    do_action = DO_ACTION_1 + sp24.clamp(0, 7) as u16;
                } else if sp1c && self.state2 & STATE2_10 == 0 {
                    do_action = DO_ACTION_DIVE;
                } else if !sp1c && (self.state1 & STATE1_22 == 0 || self.func_80833BCC() || !(!self.adult && self.current_shield == 2)) {
                    let behavior_2 = env.room_behavior_type1 == 2;
                    if self.state1 & STATE1_14 == 0
                        && sp20 <= 0
                        && (self.state1 & STATE1_4 != 0 || (self.s.floor_type != FLOOR_TYPE_7 && (self.func_80833B2C() || (!behavior_2 && self.state1 & STATE1_22 == 0 && sp20 == 0))))
                    {
                        do_action = DO_ACTION_ATTACK;
                    } else if !behavior_2 && self.func_80833BCC() && sp20 > 0 {
                        do_action = DO_ACTION_JUMP;
                    } else if self.held_item_ap >= env.data.items.ap("SWORD_MASTER") || (self.state2 & (1 << 20) != 0 && env.target.arrow_pointed.is_none()) {
                        do_action = DO_ACTION_PUTAWAY;
                    }
                }
            }
        }
        if do_action != DO_ACTION_PUTAWAY {
            self.unk_837 = 20;
        } else if self.unk_837 != 0 {
            do_action = DO_ACTION_NONE;
            self.unk_837 -= 1;
        }
        self.play_requests.push(PlayRequest::DoAction(do_action));
        // Interface_SetNaviCall: Navi calls while she has something to say.
        if self.state2 & STATE2_21 != 0 {
            if self.unk_664.is_some() {
                self.play_requests.push(PlayRequest::NaviCall(0x1E));
            } else {
                self.play_requests.push(PlayRequest::NaviCall(0x1D));
            }
            self.play_requests.push(PlayRequest::NaviCall(0x1E));
        } else {
            self.play_requests.push(PlayRequest::NaviCall(0x1F));
        }
    }

    // ================================================================================
    // Colliders (the end of Player_UpdateCommon, and Player_PostLimbDrawGameplay)

    /// The end of `Player_UpdateCommon`: the body cylinder from the last draw's body parts,
    /// registered for OC and AC; the mass; then the per-frame resets of the AC and the sword's
    /// AT.
    fn update_colliders(&mut self, play: &mut PlayState) {
        use cc::ColliderShape;
        self.door_type = PLAYER_DOORTYPE_NONE;
        self.unk_8A1 = 0;
        // The talk offers of this frame end here (they're made again next frame), unless one
        // was accepted (ACTOR_FLAG_8).
        if self.actor.flags & ACTOR_FLAG_8 == ACTOR_FLAG_8 {
            self.target_actor_distance = 0.0;
        } else {
            self.target_actor = None;
            self.target_actor_distance = f32::MAX;
            self.exchange_item_id = 0;
        }
        // The get-item offers too (made again next frame), unless Player holds what offered.
        if self.state1 & STATE1_11 == 0 {
            self.interact_range_actor = None;
            self.get_item_direction = 0x6000;
        }
        // (rideActor: no riding.) Navi's text is set again by her next update.
        self.navi_text_id = 0;
        let mut temp_f0 = self.actor.world_pos.y - self.actor.prev_pos.y;
        let bp = &self.body_parts_pos;
        let mut phi_f12 = (bp[BODYPART_L_FOOT].y + bp[BODYPART_R_FOOT].y) * 0.5 + temp_f0;
        temp_f0 += bp[BODYPART_HEAD].y + 10.0;
        self.cylinder.dim.height = (temp_f0 - phi_f12) as i16;
        if self.cylinder.dim.height < 0 {
            phi_f12 = temp_f0;
            self.cylinder.dim.height = -self.cylinder.dim.height;
        }
        self.cylinder.dim.y_shift = (phi_f12 - self.actor.world_pos.y) as i16;
        if self.state1 & STATE1_22 != 0 {
            self.cylinder.dim.height = (self.cylinder.dim.height as f32 * 0.8) as i16;
        }
        self.cylinder.update(&self.actor);
        let invincibility_timer = self.invincibility_timer;
        if self.state2 & STATE2_14 == 0 {
            if self.state1 & (STATE1_7 | STATE1_13 | STATE1_14 | STATE1_23) == 0 {
                play.collision_check_set_oc(&self.actor, COLLIDER_CYLINDER, &mut self.cylinder);
            }
            if self.state1 & (STATE1_7 | STATE1_26) == 0 && invincibility_timer <= 0 {
                play.collision_check_set_ac(&self.actor, COLLIDER_CYLINDER, &mut self.cylinder);
                if invincibility_timer < 0 {
                    play.collision_check_set_at(&self.actor, COLLIDER_CYLINDER, &mut self.cylinder);
                }
            }
        }
        self.actor.col_chk_info.mass = if self.state1 & (STATE1_7 | STATE1_28 | STATE1_29) != 0 { cc::MASS_IMMOVABLE } else { 50 };
        self.state3 &= !(1 << 2);
        self.cylinder.reset_ac();
        self.melee_weapon_quads[0].reset_at();
        self.melee_weapon_quads[1].reset_at();
        self.shield_quad.reset_ac();
        self.shield_quad.reset_at();
    }

    /// `bodyPartsPos`: each body part's limb origin, as `Player_PostLimbDrawGameplay` records it
    /// from the matrix the limb drew with (after the foot IK and the look rotations). Returns
    /// every limb's world matrix.
    fn update_body_parts(&mut self, data: &GameData) -> Vec<glam::Mat4> {
        let rig = &data.rigs[if self.adult { 0 } else { 1 }];
        let bones = pose(rig, &self.draw_joints(), &self.look_rotations(), data.limb("HEAD"), data.limb("UPPER"));
        let r = self.actor.shape_rot;
        let root = oot_game::footik::actor_matrix(self.actor.world_pos, self.actor.shape_y_offset, [r.x, r.y, r.z]);
        let world: Vec<glam::Mat4> = bones.iter().map(|b| root * *b).collect();
        for (i, name) in BODYPART_LIMBS.iter().enumerate() {
            self.body_parts_pos[i] = world[data.limb(name)].transform_point3(Vec3::ZERO);
        }
        world
    }

    /// `Player_PostLimbDrawGameplay`, `PLAYER_LIMB_R_HAND`: `sGetItemRefPos`, where the held-up
    /// item floats: the left hand while an exchange item is shown outside the get-item action,
    /// else between the hands.
    fn get_item_ref_pos(&self) -> Vec3 {
        if self.state1 & STATE1_10 == 0 && self.unk_862 != 0 && self.exchange_item_id != 0 {
            self.left_hand_pos
        } else {
            (self.body_parts_pos[BODYPART_R_HAND] + self.left_hand_pos) * 0.5
        }
    }

    /// `Player_PostLimbDrawGameplay` for `PLAYER_LIMB_L_HAND`, the sword part: with the weapon
    /// active, its tip and base from the hand's matrix (`func_80090A28`), then the quads
    /// (`func_800906D4`).
    fn post_limb_draw_l_hand(&mut self, play: &mut PlayState, hand: glam::Mat4) {
        if self.actor.scale.y < 0.0 || self.melee_weapon_state == 0 {
            return;
        }
        // Player_HoldsBrokenKnife: never. D_80126080.x = sMeleeWeaponLengths[...].
        let len = MELEE_WEAPON_LENGTHS[Self::melee_weapon(self.held_item_ap) as usize];
        let d_80126080 = Vec3::new(len, 400.0, 0.0);
        // func_80090A28: the far edge is longer, and longer still in a combo's third attack.
        let mut x = len;
        if self.unk_845 >= 3 {
            // As written: drawing advances unk_845 while the combo attack is active.
            self.unk_845 += 1;
            x *= 1.0 + ((9 - self.unk_845) as f32 * 0.1);
        }
        x += 1200.0;
        let d_8012608c = Vec3::new(x, -400.0, 1000.0);
        let d_80126098 = Vec3::new(x, 1400.0, -1000.0);
        let tips = [hand.transform_point3(d_80126080), hand.transform_point3(d_8012608c), hand.transform_point3(d_80126098)];
        // func_800906D4.
        let bases = D_801260A4.map(|v| hand.transform_point3(v));
        // The first edge is the sword trail's (EffectBlure isn't ported).
        func_80090480(play, &self.actor, None, &mut self.melee_weapon_info[0], tips[0], bases[0]);
        let spin = play.data.items.mwa("SPIN_ATTACK_1H");
        if self.melee_weapon_state > 0 && (self.melee_weapon_animation < spin || self.state2 & STATE2_17 != 0) {
            let [q0, q1] = &mut self.melee_weapon_quads;
            func_80090480(play, &self.actor, Some((COLLIDER_SWORD_0, q0)), &mut self.melee_weapon_info[1], tips[1], bases[1]);
            func_80090480(play, &self.actor, Some((COLLIDER_SWORD_1, q1)), &mut self.melee_weapon_info[2], tips[2], bases[2]);
        }
    }

    // ================================================================================
    // Draw-time state

    /// The joint table the skeleton is drawn with, including
    /// `Player_OverrideLimbDrawGameplayCommon`'s child root scaling and the `unk_6C4` sink.
    pub fn draw_joints(&self) -> eng_anim::anim::JointTable {
        let mut rot: Vec<[i16; 3]> = self.skel.joint.to_vec();
        let mut root = [rot[0][0] as f32, rot[0][1] as f32, rot[0][2] as f32];
        if !self.adult {
            let mf = self.skel.move_flags;
            if mf & 4 == 0 || mf & 1 != 0 {
                root[0] *= 0.64;
                root[2] *= 0.64;
            }
            if mf & 4 == 0 || mf & 2 != 0 {
                root[1] *= 0.64;
            }
        }
        root[1] -= self.unk_6C4;
        rot[0] = [root[0] as i16, root[1] as i16, root[2] as i16];
        eng_anim::anim::JointTable { rot, face: self.skel.face }
    }

    /// Head (x, y, z additions) and upper-body (y, x, z pre-rotations) look/lean angles.
    pub fn look_rotations(&self) -> LookRotations {
        LookRotations {
            head: [self.unk_6BA, self.unk_6B8.wrapping_neg(), self.unk_6B6],
            upper_y: self.unk_6BE,
            upper_x: self.unk_6BC,
            upper_z: self.unk_6C0,
            root_pitch: self.unk_6C2,
        }
    }

    pub fn wall_flag_3(&self) -> bool {
        self.s.wall_flags & WALL_FLAG_3 != 0
    }
}

/// Rotations `Player_OverrideLimbDrawGameplayCommon` applies to the head and upper body.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LookRotations {
    /// Added to the head limb's (x, y, z) rotation.
    pub head: [i16; 3],
    /// Applied before the upper body limb's own transform: RotY, then RotX, then RotZ.
    pub upper_y: i16,
    pub upper_x: i16,
    pub upper_z: i16,
    /// `unk_6C2`: the root limb's pitch while diving (`Player_OverrideLimbDrawGameplayCommon`).
    pub root_pitch: i16,
}

// ================================================================================
// The actor system

/// Player's collider ids (`ActorImpl::collider_mut`).
pub const COLLIDER_CYLINDER: u8 = 0;
pub const COLLIDER_SWORD_0: u8 = 1;
pub const COLLIDER_SWORD_1: u8 = 2;
pub const COLLIDER_SHIELD: u8 = 3;

/// `PLAYER_BODYPART_MAX`, and the limbs of `PLAYER_BODYPART_*` in order: every limb after the
/// root that has a display list (`Player_OverrideLimbDrawGameplayCommon`).
pub const BODYPART_MAX: usize = 18;
const BODYPART_LIMBS: [&str; BODYPART_MAX] =
    ["WAIST", "R_THIGH", "R_SHIN", "R_FOOT", "L_THIGH", "L_SHIN", "L_FOOT", "HEAD", "HAT", "COLLAR", "L_SHOULDER", "L_FOREARM", "L_HAND", "R_SHOULDER", "R_FOREARM", "R_HAND", "SHEATH", "TORSO"];
pub const BODYPART_R_FOOT: usize = 3;
pub const BODYPART_L_FOOT: usize = 6;
pub const BODYPART_HEAD: usize = 7;
pub const BODYPART_L_HAND: usize = 12;
pub const BODYPART_R_HAND: usize = 15;

/// `WeaponInfo`: a weapon edge's tip and base as last drawn.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WeaponInfo {
    pub active: bool,
    pub tip: Vec3,
    pub base: Vec3,
}

/// `sMeleeWeaponLengths` (`z_player_lib.c`), by `Player_ActionToMeleeWeapon`.
const MELEE_WEAPON_LENGTHS: [f32; 6] = [0.0, 4000.0, 3000.0, 5500.0, 0.0, 2500.0];

/// `D_801260A4`: the sword's base points in the left hand's space.
const D_801260A4: [Vec3; 3] = [Vec3::new(0.0, 400.0, 0.0), Vec3::new(0.0, 1400.0, -1000.0), Vec3::new(0.0, -400.0, 1000.0)];

/// `D_80854488`: per melee weapon (`Player_GetMeleeWeaponHeld() - 1`), the damage type of a
/// slash and of a jump attack.
const D_80854488: [[u32; 2]; 5] = [
    [cc::DMG_SLASH_MASTER, cc::DMG_JUMP_MASTER],
    [cc::DMG_SLASH_KOKIRI, cc::DMG_JUMP_KOKIRI],
    [cc::DMG_SLASH_GIANT, cc::DMG_JUMP_GIANT],
    [cc::DMG_DEKU_STICK, cc::DMG_JUMP_MASTER],
    [cc::DMG_HAMMER_SWING, cc::DMG_HAMMER_JUMP],
];

const NO_TOUCH: ColliderTouch = ColliderTouch { dmg_flags: 0, effect: 0, damage: 0 };

/// `D_80854624`: the body.
const D_80854624: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: cc::COLTYPE_HIT5, at_flags: cc::AT_NONE, ac_flags: cc::AC_ON | cc::AC_TYPE_ENEMY, oc_flags1: cc::OC1_ON | cc::OC1_TYPE_ALL, oc_flags2: cc::OC2_TYPE_PLAYER, shape: cc::COLSHAPE_CYLINDER },
    info: ColliderInfoInit {
        elem_type: cc::ELEMTYPE_UNK1,
        toucher: NO_TOUCH,
        bumper: ColliderBumpInit { dmg_flags: 0xFFCF_FFFF, effect: 0, defense: 0 },
        toucher_flags: cc::TOUCH_NONE,
        bumper_flags: cc::BUMP_ON,
        oc_elem_flags: cc::OCELEM_ON,
    },
    dim: Cylinder16 { radius: 12, height: 60, y_shift: 0, pos: [0; 3] },
};

/// `D_80854650`: a sword quad.
const D_80854650: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: cc::COLTYPE_NONE, at_flags: cc::AT_ON | cc::AT_TYPE_PLAYER, ac_flags: cc::AC_NONE, oc_flags1: cc::OC1_NONE, oc_flags2: cc::OC2_TYPE_PLAYER, shape: cc::COLSHAPE_QUAD },
    info: ColliderInfoInit {
        elem_type: cc::ELEMTYPE_UNK2,
        toucher: ColliderTouch { dmg_flags: 0x0000_0100, effect: 0, damage: 1 },
        bumper: ColliderBumpInit { dmg_flags: 0xFFCF_FFFF, effect: 0, defense: 0 },
        toucher_flags: cc::TOUCH_ON | cc::TOUCH_SFX_NORMAL,
        bumper_flags: cc::BUMP_NONE,
        oc_elem_flags: cc::OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `D_808546A0`: the shield.
const D_808546A0: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: cc::COLTYPE_METAL, at_flags: cc::AT_ON | cc::AT_TYPE_PLAYER, ac_flags: cc::AC_ON | cc::AC_HARD | cc::AC_TYPE_ENEMY, oc_flags1: cc::OC1_NONE, oc_flags2: cc::OC2_TYPE_PLAYER, shape: cc::COLSHAPE_QUAD },
    info: ColliderInfoInit {
        elem_type: cc::ELEMTYPE_UNK2,
        toucher: ColliderTouch { dmg_flags: 0x0010_0000, effect: 0, damage: 0 },
        bumper: ColliderBumpInit { dmg_flags: 0xDFCF_FFFF, effect: 0, defense: 0 },
        toucher_flags: cc::TOUCH_ON | cc::TOUCH_SFX_NORMAL,
        bumper_flags: cc::BUMP_ON,
        oc_elem_flags: cc::OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `func_80090480`: moves a weapon edge to its new tip and base. The first draw only records
/// it; after that, if it moved, the quad spans the old and the new edge and attacks
/// (`CollisionCheck_SetAT`). Returns whether the edge is new or moved.
fn func_80090480(play: &mut PlayState, owner: &Actor, collider: Option<(u8, &mut ColliderQuad)>, info: &mut WeaponInfo, new_tip: Vec3, new_base: Vec3) -> bool {
    use cc::ColliderShape;
    if !info.active {
        if let Some((_, c)) = collider {
            c.reset_at();
        }
        info.tip = new_tip;
        info.base = new_base;
        info.active = true;
        true
    } else if info.tip == new_tip && info.base == new_base {
        if let Some((_, c)) = collider {
            c.reset_at();
        }
        false
    } else {
        if let Some((id, c)) = collider {
            c.set_vertices(new_base, new_tip, info.base, info.tip);
            play.collision_check_set_at(owner, id, c);
        }
        info.base = new_base;
        info.tip = new_tip;
        info.active = true;
        true
    }
}

/// `Player_InitVars` (`z_player_call.c`).
pub const PROFILE: oot_game::actor_ctx::ActorProfile = oot_game::actor_ctx::ActorProfile {
    id: ACTOR_PLAYER,
    name: "Player",
    category: ACTORCAT_PLAYER,
    flags: ACTOR_FLAG_0 | ACTOR_FLAG_2 | ACTOR_FLAG_4 | ACTOR_FLAG_5 | ACTOR_FLAG_25 | ACTOR_FLAG_26,
    object: "gameplay_keep",
};

/// Indices into Player's `RenderState` extras.
mod rs {
    /// `angles`: the head's x/y/z additions, the upper body's y/x/z, the root pitch.
    pub const HEAD: usize = 0;
    pub const UPPER_Y: usize = 3;
    pub const UPPER_X: usize = 4;
    pub const UPPER_Z: usize = 5;
    pub const ROOT_PITCH: usize = 6;
    /// `angles`: the held-up item's spin (`gameplayFrames * 1000`).
    pub const GET_ITEM_SPIN: usize = 7;
    /// `values`: `speedXZ`, `shape.yOffset`, `sGetItemRefPos` (x, y, z).
    pub const SPEED_XZ: usize = 0;
    pub const Y_OFFSET: usize = 1;
    pub const GET_ITEM_POS: usize = 2;
    /// `switches`: the blink face, `modelGroup`, `currentShield`, `unk_862`, and whether
    /// `exchangeItemId` is set.
    pub const FACE: usize = 0;
    pub const MODEL_GROUP: usize = 1;
    pub const SHIELD: usize = 2;
    pub const UNK_862: usize = 3;
    pub const EXCHANGE: usize = 4;
    /// `switches`: `PLAYER_STATE2_18` (in a crawlspace).
    pub const CRAWLING: usize = 5;
    /// `switches`: `PLAYER_STATE2_29` (not drawn).
    pub const HIDDEN: usize = 6;
    /// `switches`: `shape.shadowDraw` set.
    pub const SHADOW: usize = 7;
}

impl LookRotations {
    fn from_angles(a: &[i16]) -> LookRotations {
        LookRotations {
            head: [a[rs::HEAD], a[rs::HEAD + 1], a[rs::HEAD + 2]],
            upper_y: a[rs::UPPER_Y],
            upper_x: a[rs::UPPER_X],
            upper_z: a[rs::UPPER_Z],
            root_pitch: a[rs::ROOT_PITCH],
        }
    }
}

/// Poses Link's skeleton like `SkelAnime_DrawFlexLod` with
/// `Player_OverrideLimbDrawGameplayCommon`: the head limb's rotation gets the look offsets,
/// the upper body is pre-rotated (Y, then X, then Z) before its own transform, and while
/// diving the root pitches about a point 200 above it. Returns every limb's matrix in model
/// space.
pub fn pose(rig: &oot_game::footik::Rig, joints: &eng_anim::anim::JointTable, look: &LookRotations, head: usize, upper: usize) -> Vec<glam::Mat4> {
    use glam::Mat4;
    let r = |a: i16| eng_math::binang_to_rad(a);
    let n = rig.parents.len();
    let mut out = vec![Mat4::IDENTITY; n];
    let mut done = vec![false; n];
    // Parents before children (any such order gives the same matrices).
    while done.iter().any(|d| !d) {
        for l in 0..n {
            if done[l] || rig.parents[l].is_some_and(|p| !done[p as usize]) {
                continue;
            }
            let parent = rig.parents[l].map(|p| out[p as usize]).unwrap_or(Mat4::IDENTITY);
            let pos = if l == 0 {
                let t = joints.rot[0];
                Vec3::new(t[0] as f32, t[1] as f32, t[2] as f32)
            } else {
                Vec3::new(rig.joint_pos[l][0] as f32, rig.joint_pos[l][1] as f32, rig.joint_pos[l][2] as f32)
            };
            let mut rot = joints.rot.get(l + 1).copied().unwrap_or([0; 3]);
            let mut pre = Mat4::IDENTITY;
            if l == head {
                rot = [rot[0].wrapping_add(look.head[0]), rot[1].wrapping_add(look.head[1]), rot[2].wrapping_add(look.head[2])];
            } else if l == upper {
                pre = Mat4::from_rotation_y(r(look.upper_y)) * Mat4::from_rotation_x(r(look.upper_x)) * Mat4::from_rotation_z(r(look.upper_z));
            }
            out[l] = if l == 0 && look.root_pitch != 0 {
                // T(pos.x, (cos(unk_6C2) - 1) * 200 + pos.y, pos.z) * RotX(unk_6C2).
                let p = look.root_pitch;
                let t = Vec3::new(pos.x, (eng_math::cos_s(p) - 1.0) * 200.0 + pos.y, pos.z);
                parent * Mat4::from_translation(t) * Mat4::from_rotation_x(r(p)) * eng_anim::skeleton::local_transform(Vec3::ZERO, rot)
            } else {
                parent * pre * eng_anim::skeleton::local_transform(pos, rot)
            };
            done[l] = true;
        }
    }
    out
}

impl ActorImpl for Player {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `Player_Update` (`func_8084663C`, which does nothing, for start mode 0).
    fn update(&mut self, play: &mut PlayState) {
        if self.inert {
            return;
        }
        let io = RefCell::new(play.take_io());
        let assets = play.assets.clone();
        let audio_tables = play.audio.tables.clone();
        let env = Env {
            data: &play.data,
            col: &play.col,
            cam_input_yaw: play.input_dir_yaw(),
            gameplay_frames: play.gameplay_frames,
            actors: &play.actors,
            target: play.target_ctx.view(),
            io: &io,
            exits: play.exit_list(),
            entrances: assets.as_ref().map(|a| a.scenes.entrances.as_slice()).unwrap_or(&[]),
            transi_actors: &play.transi_actors,
            prev_room: play.room_ctx.prev.num,
            cam_unk_14c: play.game_camera.unk_14c,
            msg_state: play.message_state(),
            room_behavior_type1: play.room_ctx.cur.behavior_type1,
            me: play.player,
            cam_dir_yaw: play.cam_dir_yaw(),
            items: assets.as_ref().map(|a| &a.items).unwrap_or(&NO_ITEMS),
            audio: &audio_tables,
            cs_state: play.cs_ctx.state,
            cs_frames: play.cs_ctx.frames,
            cs_link_action: play.cs_ctx.link_action,
            scene_id: play.scene_id,
        };
        // Player_Update: no input while talking or in a cutscene's hold (PLAYER_STATE1_5,
        // _29), and no A, B or C-Up for unk_88E frames after a talk.
        let mut input = play.input;
        if self.state1 & ((1 << 5) | STATE1_29) != 0 {
            input = Input::default();
        } else if self.unk_88E != 0 {
            let mask = !(BTN_A | eng_input::pad::BTN_B | eng_input::pad::BTN_CUP);
            input.cur.button &= mask;
            input.press.button &= mask;
        }
        Player::update(self, &env, input);
        play.put_io(io.into_inner());
        for r in std::mem::take(&mut self.play_requests) {
            apply_play_request(play, r);
        }
        // Player_UpdateCamAndSeqModes' requests, in its order: Camera_SetParam, then
        // Camera_ChangeMode.
        if let Some((mode, target)) = self.cam_request.take() {
            if let Some(t) = target {
                play.game_camera.set_target(t);
            }
            play.game_camera.change_mode(&play.data.camera, mode);
            play.camera_sfx();
        }
        // Then its sequence mode (targetCtx.bgmEnemy is never set: no enemies yet), outside
        // the fishing pond.
        if self.actor.category == ACTORCAT_PLAYER && play.scene_id != oot_game::play_scene::SCENE_TURIBORI {
            let m = self.seq_mode();
            play.audio.set_sequence_mode(m);
        }
        self.update_colliders(play);
    }

    fn animation_update(&mut self) {
        self.finish_frame();
    }

    /// What `Player_Draw` changes: the foot IK (`func_8008F87C`) writes into the joint table,
    /// then the limbs record the body parts, and the left hand places the sword's colliders.
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.inert {
            return;
        }
        if play.debug.foot_ik {
            self.legs = Some(self.apply_foot_ik(&play.data, &play.col));
        }
        let data = play.data.clone();
        let world = self.update_body_parts(&data);
        // Player_PostLimbDrawGameplay, PLAYER_LIMB_L_HAND: leftHandPos.
        self.left_hand_pos = world[data.limb("L_HAND")].transform_point3(Vec3::ZERO);
        self.post_limb_draw_l_hand(play, world[data.limb("L_HAND")]);
    }


    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COLLIDER_CYLINDER => Some(ColliderMut::Cylinder(&mut self.cylinder)),
            COLLIDER_SWORD_0 => Some(ColliderMut::Quad(&mut self.melee_weapon_quads[0])),
            COLLIDER_SWORD_1 => Some(ColliderMut::Quad(&mut self.melee_weapon_quads[1])),
            COLLIDER_SHIELD => Some(ColliderMut::Quad(&mut self.shield_quad)),
            _ => None,
        }
    }

    fn render_state(&self) -> RenderState {
        let look = self.look_rotations();
        let mut angles = vec![0i16; 8];
        angles[rs::HEAD..rs::HEAD + 3].copy_from_slice(&look.head);
        angles[rs::UPPER_Y] = look.upper_y;
        angles[rs::UPPER_X] = look.upper_x;
        angles[rs::UPPER_Z] = look.upper_z;
        angles[rs::ROOT_PITCH] = look.root_pitch;
        // Matrix_RotateZYX(0, play->gameplayFrames * 1000, 0) (Player_DrawGetItemImpl).
        angles[rs::GET_ITEM_SPIN] = (self.gameplay_frames as i32).wrapping_mul(1000) as i16;
        let mut values = vec![0.0f32; 5];
        values[rs::SPEED_XZ] = self.actor.speed_xz;
        values[rs::Y_OFFSET] = self.actor.shape_y_offset;
        let r = self.get_item_ref_pos();
        values[rs::GET_ITEM_POS..rs::GET_ITEM_POS + 3].copy_from_slice(&[r.x, r.y, r.z]);
        let mut switches = vec![0u32; 8];
        switches[rs::FACE] = self.face as u32;
        switches[rs::MODEL_GROUP] = self.model_group as u32;
        switches[rs::SHIELD] = self.current_shield as u32;
        switches[rs::UNK_862] = self.unk_862 as u16 as u32;
        switches[rs::EXCHANGE] = (self.exchange_item_id != 0) as u32;
        switches[rs::CRAWLING] = (self.state2 & STATE2_18 != 0) as u32;
        switches[rs::HIDDEN] = (self.state2 & STATE2_29 != 0) as u32;
        switches[rs::SHADOW] = self.shadow_feet as u32;
        RenderState {
            pos: self.actor.world_pos,
            rot: [0, self.actor.shape_rot.y, 0],
            scale: self.actor.scale,
            y_offset: self.actor.shape_y_offset,
            joints: Some(self.draw_joints()),
            angles,
            values,
            switches,
            teleported: self.actor.teleported,
        }
    }

    /// `Player_Draw`: Link (`SkelAnime_DrawFlexLod` with the model group's hands and sheath for
    /// the shield worn and, for the child, whether the Kokiri Sword is on B; the face on
    /// segments 8 and 9, running fists), the item held up (`Player_DrawGetItem`), and the circle
    /// shadow.
    fn draw(&self, st: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        use eng_gfx::{DrawCmd, MeshKey};
        use glam::Mat4;
        // Player_Draw draws nothing under PLAYER_STATE2_29.
        if self.inert || st.switches.get(rs::HIDDEN).copied().unwrap_or(0) != 0 {
            return;
        }
        let age = oot_game::player_lib::Age::from_adult(self.adult);
        let Some(joints) = &st.joints else { return };
        let look = LookRotations::from_angles(&st.angles);
        let bones = pose(&play.data.rigs[age as usize], joints, &look, play.data.limb("HEAD"), play.data.limb("UPPER"));
        let root = oot_game::play::actor_matrix(st.pos + Vec3::Y * (st.values[rs::Y_OFFSET] * 0.01), st.rot[1], 0.01);
        // Open hands close into fists above speed 2 (Player_OverrideLimbDrawGameplayDefault).
        let fists = st.values[rs::SPEED_XZ] > 2.0;
        let rules = &play.rules;
        let (eye, mouth) = rules.face_indices(joints.face, st.switches[rs::FACE] as usize);
        let group_name = play.data.items.model_group_names.get(st.switches[rs::MODEL_GROUP] as usize).cloned().unwrap_or_default();
        let default = rules.model_group("DEFAULT").unwrap_or(0);
        let loadout = oot_game::player_lib::Loadout {
            age,
            model_group: rules.model_group(&group_name).unwrap_or(default),
            shield: st.switches[rs::SHIELD] as usize,
            tunic: 0,
            // Player_OverrideLimbDrawGameplayDefault reads the save's B item as it draws.
            child_has_kokiri_sword: play.save.equips.button_items[0] == oot_game::item::ITEM_SWORD_KOKIRI,
            moving_fast: fists,
        };
        let mesh = MeshKey { name: loadout.variant_key(rules), segment_textures: vec![(8, eye as u16), (9, mouth as u16)] };
        // In a crawlspace, with Link behind the near plane (actor.projectedPos.z < 0, from
        // viewProjectionMtxF), Player_OverrideLimbDrawGameplay_80090440 draws no limb: the
        // crawl's camera is inside him.
        let projected_z = (play.view_proj * st.pos.extend(1.0)).z;
        if !(st.switches[rs::CRAWLING] != 0 && projected_z < 0.0) {
            out.opa.push(DrawCmd { mesh, transform: root, bones, params: Default::default() });
        }
        // Player_DrawGetItem (unk_862 > 0): GetItem_Draw at sGetItemRefPos, 3.3 in front and 14
        // up (6 for an exchange item; IREG(90) is 0), spinning, at 0.2.
        let unk_862 = st.switches[rs::UNK_862] as u16 as i16;
        if unk_862 > 0
            && let Some(a) = &play.assets
        {
            let yaw = st.rot[1];
            let height = if st.switches[rs::EXCHANGE] != 0 { 6.0 } else { 14.0 };
            let v = &st.values;
            let r = Vec3::new(v[rs::GET_ITEM_POS], v[rs::GET_ITEM_POS + 1], v[rs::GET_ITEM_POS + 2]);
            let t = Vec3::new(r.x + 3.3 * sin_s(yaw), r.y + height, r.z + 3.3 * cos_s(yaw));
            let m = Mat4::from_translation(t) * Mat4::from_rotation_y(eng_math::binang_to_rad(st.angles[rs::GET_ITEM_SPIN])) * Mat4::from_scale(Vec3::splat(0.2));
            oot_game::draw::get_item_draw(&a.items, unk_862.abs() - 1, m, play.gameplay_frames, view, out);
        }
        // ActorShadow_DrawFeet's stand-in: a soft disc on the floor below, shrinking with height
        // (none while shape.shadowDraw is NULL).
        if st.switches.get(rs::SHADOW).copied().unwrap_or(1) == 0 {
            return;
        }
        let (floor, _) = play.col.entity_raycast_down(st.pos + Vec3::Y * 20.0);
        let drop = (st.pos.y - floor).max(0.0);
        let size = if self.adult { 22.0 } else { 16.0 } * (1.0 - (drop / 400.0).min(0.7));
        let shadow = Mat4::from_translation(Vec3::new(st.pos.x, floor + 0.3, st.pos.z)) * Mat4::from_scale(Vec3::new(size, 1.0, size));
        out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), shadow));
    }

    fn as_player(&self) -> Option<&dyn PlayerIface> {
        Some(self)
    }
    fn as_player_mut(&mut self) -> Option<&mut dyn PlayerIface> {
        Some(self)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl PlayerIface for Player {
    fn adult(&self) -> bool {
        self.adult
    }
    fn set_cs_mode(&mut self, cs_mode: u8, actor: Option<ActorHandle>, door_bg_cam_index: i16) {
        self.cs_mode = cs_mode;
        self.unk_448 = actor;
        self.door_bg_cam_index = door_bg_cam_index;
    }
    fn cs_mode(&self) -> u8 {
        self.cs_mode
    }
    fn state_flags1(&self) -> u32 {
        self.state1
    }
    fn state_flags2(&self) -> u32 {
        self.state2
    }
    fn target(&self) -> Option<ActorHandle> {
        self.unk_664
    }
    fn target_timer(&self) -> i16 {
        self.unk_66C
    }
    fn stick_dir(&self) -> i8 {
        self.unk_84B[self.unk_846 as usize]
    }
    fn focus(&self) -> Vec3 {
        self.head_pos
    }
    fn speed_xz(&self) -> f32 {
        self.actor.speed_xz
    }
    fn talk_target(&self) -> (Option<ActorHandle>, f32) {
        (self.target_actor, self.target_actor_distance)
    }
    fn set_talk_target(&mut self, actor: ActorHandle, distance: f32, exchange_item: u8) {
        self.target_actor = Some(actor);
        self.target_actor_distance = distance;
        self.exchange_item_id = exchange_item;
    }
    /// `Player_InBlockingCsMode` without `transitionTrigger` (magic isn't ported), or
    /// `unk_6AD == 4`.
    fn in_cs_mode(&self) -> bool {
        self.state1 & (STATE1_7 | STATE1_29) != 0 || self.cs_mode != 0 || self.state1 & STATE1_0 != 0 || self.state3 & STATE3_7 != 0 || self.unk_6AD == 4
    }
    /// Player holds no actors here (lifting isn't ported).
    fn holds_actor(&self) -> bool {
        false
    }
    fn get_item_direction(&self) -> i16 {
        self.get_item_direction
    }
    fn set_get_item(&mut self, actor: ActorHandle, get_item_id: i16, direction: i16) {
        self.get_item_id = get_item_id;
        self.interact_range_actor = Some(actor);
        self.get_item_direction = direction;
    }
    fn set_knockback(&mut self, damage: u8, kind: u8, yaw: i16, speed: f32, vy: f32) {
        self.unk_8A0 = damage;
        self.unk_8A1 = kind;
        self.unk_8A2 = yaw;
        self.unk_8A4 = speed;
        self.unk_8A8 = vy;
    }
    fn invincibility_timer(&self) -> i8 {
        self.invincibility_timer
    }
    fn change_state_flags2(&mut self, set: u32, clear: u32) {
        self.state2 = (self.state2 | set) & !clear;
    }
    fn unk_89e(&self) -> u16 {
        self.unk_89E
    }
    fn current_boots(&self) -> u8 {
        self.current_boots
    }
    fn change_state_flags1(&mut self, set: u32, clear: u32) {
        self.state1 = (self.state1 | set) & !clear;
    }
    fn set_equipment_data(&mut self, data: &GameData, save: &oot_game::save::SaveContext) {
        Player::set_equipment_data(self, data, save);
    }
    fn body_part(&self, i: usize) -> Vec3 {
        self.body_parts_pos.get(i).copied().unwrap_or(self.actor.world_pos)
    }
    fn navi_text_id(&self) -> i16 {
        self.navi_text_id
    }
    fn set_navi_text_id(&mut self, id: i16) {
        self.navi_text_id = id;
    }
    fn navi_actor(&self) -> Option<ActorHandle> {
        self.navi_actor
    }
}

/// Applies a `PlayRequest` (see there), after Player's update.
fn apply_play_request(play: &mut PlayState, r: PlayRequest) {
    let d = play.data.clone();
    match r {
        PlayRequest::CamSetting(s) => {
            // !Play_CamIsNotFixed: Interface_ChangeAlpha(2) for SCENE_TRANSITION (no interface).
            if play.cam_is_not_fixed() {
                play.game_camera.change_setting(&d.camera, s);
            }
        }
        PlayRequest::ChangeSetting(s) => {
            play.game_camera.change_setting(&d.camera, s);
        }
        PlayRequest::DoorCam { door, bg_cam_index, timers } => {
            play.game_camera.change_door_cam(&d.camera, &play.col, Some(door), bg_cam_index, timers[0], timers[1], timers[2]);
        }
        PlayRequest::CamDone => play.game_camera.func_8005b1a4(),
        PlayRequest::RoomLoad(n) => {
            play.room_request(n);
        }
        PlayRequest::RoomChangeDone => play.room_change_done(),
        PlayRequest::OpenDoor { door, open_anim } => {
            if let Some(dr) = play.actors.downcast_mut::<crate::en_door::EnDoor>(door) {
                dr.open_anim = open_anim;
                dr.player_is_opening = true;
            }
        }
        PlayRequest::DoorRoom { door } => {
            let room = play.room_ctx.cur.num;
            let attached = play.actors.actor_mut(door).and_then(|a| {
                a.room = room;
                a.child.or(a.parent)
            });
            if let Some(a) = attached.and_then(|h| play.actors.actor_mut(h)) {
                a.room = room;
            }
        }
        PlayRequest::TalkRequest(h) => {
            if let Some(a) = play.actors.actor_mut(h) {
                a.flags |= ACTOR_FLAG_8;
            }
        }
        PlayRequest::SetTextId { actor, text_id } => {
            if let Some(a) = play.actors.actor_mut(actor) {
                a.text_id = text_id;
            }
        }
        PlayRequest::StartTextbox { text_id, actor } => play.start_textbox(text_id, actor),
        PlayRequest::DoAction(action) => play.interface_ctx.set_do_action(action),
        PlayRequest::SetParent(h) => {
            let me = play.player;
            if let Some(a) = play.actors.actor_mut(h) {
                a.parent = me;
            }
        }
        PlayRequest::ChestOpen { chest, unk_1f4 } => {
            if let Some(c) = play.actors.downcast_mut::<crate::en_box::EnBox>(chest) {
                c.unk_1f4 = unk_1f4;
            }
        }
        PlayRequest::SetCameraData { data2 } => play.game_camera.set_camera_data(4, data2, 0),
        PlayRequest::OnePointCutscene { cs_id, timer, player, parent } => {
            let actor = if player { play.player.and_then(|h| play.cam_actor(h)) } else { None };
            play.onepoint_cutscene_init(cs_id, timer, actor, parent);
        }
        PlayRequest::DropCollectible { pos, params } => {
            crate::en_item00::item_drop_collectible(play, pos, params);
        }
        PlayRequest::ItemGive(item) => {
            oot_game::item::item_give(&mut play.save, Some(&mut play.audio), item);
        }
        PlayRequest::NaviCall(state) => {
            let cs_idle = play.cs_ctx.state == oot_game::cutscene::CS_STATE_IDLE;
            play.interface_ctx.set_navi_call(state, cs_idle, &mut play.audio);
        }
        PlayRequest::GetItemFanfare(gi) => {
            use oot_game::audio::*;
            use oot_game::item::*;
            if (GI_RUPEE_GREEN..=GI_RUPEE_RED).contains(&gi) || (GI_RUPEE_PURPLE..=GI_RUPEE_GOLD).contains(&gi) || (GI_RUPEE_GREEN_LOSE..=GI_RUPEE_PURPLE_LOSE).contains(&gi) || gi == GI_RECOVERY_HEART {
                play.audio.func_80078884(oot_game::audio::sfx::NA_SE_SY_GET_BOXITEM);
            } else {
                let pieces = play.save.inventory.quest_items & 0xF000_0000;
                let temp1 = if gi == GI_HEART_CONTAINER_2 || gi == GI_HEART_CONTAINER || (gi == GI_HEART_PIECE && pieces == 4 << QUEST_HEART_PIECE_COUNT) {
                    NA_BGM_HEART_GET | 0x900
                } else if gi == GI_HEART_PIECE {
                    NA_BGM_SMALL_ITEM_GET
                } else {
                    NA_BGM_ITEM_GET | 0x900
                };
                play.audio.play_fanfare(temp1);
            }
        }
        PlayRequest::FadeOutAllSeq(n) => play.audio.func_800f6964(n),
        PlayRequest::Sfx(s) => {
            use oot_game::audio::sfx::SfxPos;
            let Some(me) = play.player else { return };
            let pos = SfxPos::Actor(me);
            let a = &mut play.audio;
            match s {
                PlayerSfx::NoPos(id) => a.func_80078884(id),
                PlayerSfx::Actor(id) => a.func_80078914(pos, id),
                PlayerSfx::Footstep(id, v) => a.func_800f4010(pos, id, v),
                PlayerSfx::F4138(id, v) => a.func_800f4138(pos, id, v),
                PlayerSfx::F4190(id) => a.func_800f4190(pos, id),
                PlayerSfx::CodeReverb(r) => a.set_code_reverb(r),
                PlayerSfx::StopById(id) => a.stop_sfx_by_id(id),
                PlayerSfx::BaseFilter(f) => a.set_base_filter(f),
            }
        }
    }
}

/// `func_8083816C`: floor types that count as a void floor with `FLOOR_PROPERTY_12`.
fn func_8083816C(floor_type: u32) -> bool {
    floor_type == FLOOR_TYPE_4 || floor_type == FLOOR_TYPE_7 || floor_type == FLOOR_TYPE_12
}

/// No item tables (play without the pack).
static NO_ITEMS: oot_game::item::ItemTables = oot_game::item::ItemTables { get_items: Vec::new(), draw_items: Vec::new() };

